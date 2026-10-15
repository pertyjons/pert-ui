//! Public drawing extensions and retained cache behavior without a window/GPU.
use pert_ui_runtime::*;
use std::{cell::Cell, rc::Rc, time::Duration};
type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn scene_keeps_parent_body_child_and_foreground_order() -> TestResult {
    struct LayeredParent(Vec<Element>);
    impl Component for LayeredParent {
        fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
            Ok(())
        }
        fn take_children(&mut self) -> Vec<Element> {
            std::mem::take(&mut self.0)
        }
        fn paint(&self, painter: &mut Painter) -> Result<(), DrawError> {
            painter.fill_rect(Rect::new(0.0, 0.0, 10.0, 10.0)?, Color::SURFACE);
            Ok(())
        }
        fn paint_foreground(&self, painter: &mut Painter) -> Result<(), DrawError> {
            painter.fill_rect(Rect::new(0.0, 0.0, 10.0, 10.0)?, Color::ACCENT);
            Ok(())
        }
    }
    struct Child;
    impl Component for Child {
        fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
            Ok(())
        }
        fn paint(&self, painter: &mut Painter) -> Result<(), DrawError> {
            painter.fill_rect(Rect::new(0.0, 0.0, 10.0, 10.0)?, Color::TEXT);
            Ok(())
        }
    }
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(20.0, 20.0, 1.0)?)?;
    ui.mount_tree(Element::new(
        "parent",
        LayeredParent(vec![Element::new("child", Child)]),
    ))?;
    ui.flush(Duration::ZERO)?;
    let scene = ui.paint()?;
    assert_eq!(scene.commands().len(), 3);
    let colors = scene
        .commands()
        .iter()
        .filter_map(|command| match command {
            DrawCommand::FillRect { color, .. } => Some(*color),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(colors, [Color::SURFACE, Color::TEXT, Color::ACCENT]);
    Ok(())
}

struct Rectangle {
    value: ReadSignal<i32>,
    paints: Rc<Cell<usize>>,
    writer: Signal<i32>,
    fail: Rc<Cell<bool>>,
}
impl Component for Rectangle {
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        cx.read_paint(&self.value)?;
        Ok(())
    }
    fn paint(&self, painter: &mut Painter) -> Result<(), DrawError> {
        assert_eq!(self.writer.set(99), Err(RuntimeError::WriteDuringPaint));
        self.paints.set(self.paints.get() + 1);
        if self.fail.get() {
            return Err(DrawError::InvalidColor);
        }
        painter.fill_rect(
            Rect::new(0.0, 0.0, painter.viewport().width(), 10.0)?,
            Color::ACCENT,
        );
        Ok(())
    }
}

#[test]
fn unchanged_components_keep_drawing_across_other_updates_and_exposure() -> TestResult {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(960.0, 640.0, 1.5)?)?;
    let value = ui.signal(0);
    let other = ui.signal(0);
    let counts = [Rc::new(Cell::new(0)), Rc::new(Cell::new(0))];
    let mut ids = Vec::new();
    for (index, signal) in [&value, &other].into_iter().enumerate() {
        ids.push(ui.mount(
            index.to_string(),
            Rectangle {
                value: signal.read_only(),
                paints: counts[index].clone(),
                writer: signal.clone(),
                fail: Rc::new(Cell::new(false)),
            },
        )?);
    }
    assert_eq!(ui.paint(), Err(DrawError::PendingUpdates));
    ui.flush(Duration::ZERO)?;
    assert_eq!(ui.paint()?.len(), 2);
    ui.paint()?;
    assert_eq!([counts[0].get(), counts[1].get()], [1, 1]);
    value.set(1)?;
    ui.flush(Duration::ZERO)?;
    ui.paint()?;
    assert_eq!([counts[0].get(), counts[1].get()], [2, 1]);
    ui.set_visible(&ids[0], false)?;
    assert_eq!(ui.paint()?.len(), 1);
    ui.set_viewport(Viewport::new(800.0, 600.0, 2.0)?)?;
    ui.flush(Duration::ZERO)?;
    ui.paint()?;
    assert_eq!([counts[0].get(), counts[1].get()], [2, 2]);
    ui.set_visible(&ids[0], true)?;
    ui.flush(Duration::ZERO)?;
    ui.paint()?;
    assert_eq!([counts[0].get(), counts[1].get()], [3, 2]);
    assert_eq!(ui.next_deadline(), None);
    Ok(())
}

#[test]
fn failed_paint_is_retryable_and_restores_write_phase() -> TestResult {
    let mut ui = Runtime::new();
    // A real viewport, so the retried drawing lands in the scene: the default
    // zero-sized one clips every component away and composes nothing.
    ui.set_viewport(Viewport::new(200.0, 100.0, 1.0)?)?;
    let value = ui.signal(0);
    let fail = Rc::new(Cell::new(true));
    let paints = Rc::new(Cell::new(0));
    ui.mount(
        "rectangle",
        Rectangle {
            value: value.read_only(),
            paints: paints.clone(),
            writer: value.clone(),
            fail: fail.clone(),
        },
    )?;
    ui.flush(Duration::ZERO)?;
    assert_eq!(ui.paint(), Err(DrawError::InvalidColor));
    value.set(1)?;
    fail.set(false);
    ui.flush(Duration::ZERO)?;
    assert_eq!(ui.paint()?.len(), 1);
    assert_eq!(paints.get(), 2);
    Ok(())
}

#[test]
fn drawing_rejects_invalid_numeric_data() {
    assert!(Rect::new(0.0, 0.0, -1.0, 1.0).is_err());
    assert!(Rect::new(f32::NAN, 0.0, 1.0, 1.0).is_err());
    assert!(Rect::new(-1.0, -1.0, 0.0, 0.0).is_ok());
    assert!(Color::rgba(f32::INFINITY, 0.0, 0.0, 1.0).is_err());
    assert!(Color::rgba(0.0, 0.0, 0.0, 1.1).is_err());
    assert!(Viewport::new(100.0, 100.0, 0.0).is_err());
    assert!(Viewport::new(100.0, 100.0, f64::NAN).is_err());
}

#[test]
fn raster_dimensions_are_rejected_before_byte_count_overflow() {
    for (width, height, bytes) in [
        (u32::MAX, u32::MAX, 0),
        (2_147_529_989, 2_147_437_308, 166_832),
    ] {
        assert_eq!(
            RasterImage::from_srgba8(width, height, vec![0; bytes]),
            Err(DrawError::InvalidImage)
        );
    }
}
