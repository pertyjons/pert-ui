//! The development overlay is part of the scene, so its contract is asserted on
//! the scene: what it draws, what bounds it, and that it costs nothing idle.
use pert_ui_runtime::*;
use std::{cell::RefCell, rc::Rc, time::Duration};

#[derive(Clone, Default)]
struct Observed(Rc<RefCell<ComponentDetails>>);
impl Observed {
    fn at(bounds: [f32; 4]) -> Self {
        let observed = Self::default();
        observed.0.borrow_mut().box_model.border_box = Some(bounds);
        observed
    }
}
impl Component for Observed {
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
    fn inspect(&self) -> ComponentDetails {
        self.0.borrow().clone()
    }
}
type TestResult = Result<(), Box<dyn std::error::Error>>;

#[cfg(debug_assertions)]
const RED: [f32; 4] = [1.0, 0.0, 0.0, 1.0];

#[cfg(debug_assertions)]
fn rects(scene: &DisplayList) -> Vec<(Rect, Color)> {
    scene
        .commands()
        .iter()
        .map(|command| match command {
            DrawCommand::FillRect { rect, color } => (*rect, *color),
            other => panic!("the overlay draws rectangles only, found {other:?}"),
        })
        .collect()
}

/// Two overlapping peers: one issue, two markers.
fn overlapping(runtime: &mut Runtime) -> Result<(Observed, WidgetId), RuntimeError> {
    runtime.set_viewport(Viewport::new(200.0, 100.0, 2.0).expect("valid viewport"))?;
    runtime.mount("a", Observed::at([10.0, 10.0, 40.0, 20.0]))?;
    let second = Observed::at([30.0, 10.0, 40.0, 20.0]);
    let id = runtime.mount("b", second.clone())?;
    runtime.flush(Duration::ZERO)?;
    Ok((second, id))
}

#[test]
fn off_draws_nothing_and_release_builds_draw_nothing_in_any_mode() -> TestResult {
    let mut runtime = Runtime::new();
    overlapping(&mut runtime)?;
    assert_eq!(runtime.debug_overlay(), DebugOverlay::Off);
    assert!(runtime.paint()?.commands().is_empty());
    for overlay in [DebugOverlay::Issues, DebugOverlay::Boxes] {
        runtime.set_debug_overlay(overlay);
        assert_eq!(runtime.inspect().overlay, overlay);
        let drawn = !runtime.paint()?.commands().is_empty();
        assert_eq!(drawn, cfg!(debug_assertions), "{overlay:?}");
    }
    Ok(())
}

#[cfg(debug_assertions)]
#[test]
fn issues_outline_each_marker_inside_its_bounds_two_physical_pixels_wide() -> TestResult {
    let mut runtime = Runtime::new();
    let (second, id) = overlapping(&mut runtime)?;
    runtime.set_debug_overlay(DebugOverlay::Issues);
    let scene = runtime.paint()?;
    let drawn = rects(&scene);
    assert_eq!(drawn.len(), 8, "two outlines of four edges, and no badge");
    assert!(drawn.iter().all(|(_, color)| color.channels() == RED));
    // Scale 2: two physical pixels are one logical pixel.
    let (top, _) = drawn[0];
    assert_eq!(top.values(), [10.0, 10.0, 40.0, 1.0]);
    let (left, _) = drawn[2];
    assert_eq!(left.values(), [10.0, 11.0, 1.0, 18.0]);

    // A fix removes the outlines on the next paint, which the fix requested.
    second.0.borrow_mut().box_model.border_box = Some([60.0, 10.0, 40.0, 20.0]);
    runtime.invalidate(&id, Invalidation::Layout)?;
    runtime.flush(Duration::ZERO)?;
    assert!(runtime.validation_report().issues.is_empty());
    assert!(runtime.needs_paint());
    assert!(runtime.paint()?.commands().is_empty());
    Ok(())
}

#[cfg(debug_assertions)]
#[test]
fn the_overlay_never_schedules_frames_or_changes_what_inspection_reports() -> TestResult {
    let mut runtime = Runtime::new();
    overlapping(&mut runtime)?;
    runtime.paint()?;
    assert!(!runtime.needs_paint());
    let before = runtime.inspect();

    // Switching is one repaint, not a schedule; repeating the value is nothing.
    runtime.set_debug_overlay(DebugOverlay::Issues);
    assert!(runtime.needs_paint());
    assert_eq!(runtime.next_deadline(), None);
    let first = runtime.paint()?;
    assert!(
        !runtime.needs_paint(),
        "composing the overlay invalidates nothing"
    );
    runtime.set_debug_overlay(DebugOverlay::Issues);
    assert!(!runtime.needs_paint());

    // Stable issues: later flushes validate nothing new and request no frame.
    for second in 1..4 {
        let report = runtime.flush(Duration::from_secs(second))?;
        assert_eq!(report.next_deadline, None);
        assert!(!runtime.needs_paint());
    }
    assert_eq!(runtime.paint()?.commands(), first.commands());

    let mut after = runtime.inspect();
    assert_eq!(after.overlay, DebugOverlay::Issues);
    after.overlay = before.overlay;
    after.at = before.at;
    assert_eq!(after, before);

    runtime.set_debug_overlay(DebugOverlay::Off);
    assert!(runtime.needs_paint());
    assert!(runtime.paint()?.commands().is_empty());
    Ok(())
}

#[cfg(debug_assertions)]
#[test]
fn outlines_stop_at_the_cap_and_the_badge_counts_the_rest() -> TestResult {
    let mut runtime = Runtime::new();
    runtime.set_viewport(Viewport::new(200.0, 100.0, 1.0)?)?;
    runtime.set_debug_overlay(DebugOverlay::Issues);
    // Every component carries its own issue, so each gets a marker.
    let extra = 3;
    for index in 0..Runtime::MAX_OVERLAY_MARKERS + extra {
        let component = Observed::at([0.0, index as f32 * 10.0, 8.0, 8.0]);
        component.0.borrow_mut().box_model.padding = Some([-1.0; 4]);
        runtime.mount(format!("c{index}"), component)?;
    }
    // Never measured: it has an issue and a marker, but no rectangle to outline.
    let unmeasured = Observed::default();
    unmeasured.0.borrow_mut().box_model.padding = Some([-1.0; 4]);
    runtime.mount("unmeasured", unmeasured)?;
    runtime.flush(Duration::ZERO)?;
    let report = runtime.validation_report();
    assert_eq!(
        report.markers.len(),
        Runtime::MAX_OVERLAY_MARKERS + extra + 1
    );
    assert!(report.markers.iter().any(|marker| marker.bounds.is_none()));

    let scene = runtime.paint()?;
    let drawn = rects(&scene);
    let outlines = Runtime::MAX_OVERLAY_MARKERS * 4;
    assert!(drawn.len() > outlines, "a badge follows the outlines");
    // The badge sits in the top-right corner: a red plate, then white cells.
    let (plate, color) = drawn[outlines];
    assert_eq!(color.channels(), RED);
    assert_eq!(plate.x() + plate.width(), 200.0);
    assert_eq!(plate.y(), 0.0);
    // One numeral, "4": the plate is one glyph and its padding wide.
    assert_eq!(plate.width(), 5.0 * 2.0);
    let cells = &drawn[outlines + 1..];
    assert_eq!(cells.len(), 9, "the block numeral 4 lights nine cells");
    assert!(cells.iter().all(|(_, color)| color.channels() == [1.0; 4]));
    Ok(())
}

#[cfg(debug_assertions)]
#[test]
fn boxes_outline_border_and_content_boxes_without_markers() -> TestResult {
    let mut runtime = Runtime::new();
    runtime.set_viewport(Viewport::new(200.0, 100.0, 1.0)?)?;
    let padded = Observed::at([10.0, 10.0, 40.0, 20.0]);
    padded.0.borrow_mut().box_model.padding = Some([2.0, 4.0, 2.0, 4.0]);
    runtime.mount("padded", padded)?;
    runtime.mount("plain", Observed::at([60.0, 10.0, 40.0, 20.0]))?;
    runtime.flush(Duration::ZERO)?;
    assert!(runtime.validation_report().issues.is_empty());
    runtime.set_debug_overlay(DebugOverlay::Boxes);
    let drawn = rects(&runtime.paint()?);
    assert_eq!(drawn.len(), 12, "border and content box, then a border box");
    assert_eq!(drawn[0].0.values(), [10.0, 10.0, 40.0, 1.0]);
    assert_eq!(drawn[4].0.values(), [14.0, 12.0, 32.0, 1.0]);
    assert_ne!(drawn[0].1, drawn[4].1);
    assert!(drawn.iter().all(|(_, color)| color.channels() != RED));
    Ok(())
}

#[cfg(debug_assertions)]
#[test]
fn boxes_stop_at_their_cap() -> TestResult {
    let mut runtime = Runtime::new();
    runtime.set_viewport(Viewport::new(400.0, 400.0, 1.0)?)?;
    let extra = 3;
    for index in 0..Runtime::MAX_OVERLAY_BOXES + extra {
        // Small, separate cells in a grid, so every box is a plain border box.
        let (column, row) = ((index % 64) as f32, (index / 64) as f32);
        runtime.mount(
            format!("c{index}"),
            Observed::at([column * 6.0, row * 6.0, 4.0, 4.0]),
        )?;
    }
    runtime.flush(Duration::ZERO)?;
    runtime.set_debug_overlay(DebugOverlay::Boxes);
    let drawn = rects(&runtime.paint()?);
    assert_eq!(
        drawn.len(),
        Runtime::MAX_OVERLAY_BOXES * 4,
        "four edges per outlined box, and nothing for the boxes past the cap"
    );
    Ok(())
}

#[cfg(debug_assertions)]
#[test]
fn containment_checks_report_escaping_children_and_collapsed_parents() -> TestResult {
    use pert_ui_runtime::prelude::*;
    /// Places its only child at a fixed rectangle, whatever its own size.
    struct Pinned {
        child: Vec<Element>,
        at: Rect,
        size: Size,
        clips: bool,
    }
    impl Component for Pinned {
        fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
            Ok(())
        }
        fn take_children(&mut self) -> Vec<Element> {
            std::mem::take(&mut self.child)
        }
        fn clips_children(&self) -> bool {
            self.clips
        }
        fn measure(&self, _: &MeasureContext<'_>, _: Constraints) -> Result<Size, RuntimeError> {
            Ok(self.size)
        }
        fn arrange(&self, cx: &mut ArrangeContext<'_>, _: Size) -> Result<(), RuntimeError> {
            cx.place(0, self.at)
        }
    }
    let codes =
        |size: Size, clips: bool| -> Result<Vec<ValidationCode>, Box<dyn std::error::Error>> {
            let mut runtime = Runtime::new();
            runtime.set_viewport(Viewport::new(200.0, 100.0, 1.0)?)?;
            runtime.mount_tree(Element::new(
                "root",
                Column::new([Element::new(
                    "parent",
                    Pinned {
                        child: vec![Element::new("child", Label::new("content"))],
                        at: Rect::new(30.0, 0.0, 40.0, 20.0)?,
                        size,
                        clips,
                    },
                )]),
            ))?;
            runtime.flush(Duration::ZERO)?;
            Ok(runtime
                .validation_report()
                .issues
                .iter()
                .map(|issue| issue.code)
                .collect())
        };
    assert_eq!(codes(Size::new(80.0, 20.0)?, false)?, []);
    assert_eq!(
        codes(Size::new(50.0, 20.0)?, false)?,
        [ValidationCode::EscapesParent]
    );
    // The same arrangement under a clip is scrolled content, not an error.
    assert_eq!(codes(Size::new(50.0, 20.0)?, true)?, []);
    assert_eq!(
        codes(Size::new(80.0, 0.0)?, false)?,
        [ValidationCode::CollapsedParent]
    );
    Ok(())
}
