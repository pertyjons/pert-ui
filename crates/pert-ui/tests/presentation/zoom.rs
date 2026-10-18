//! App-wide zoom, native geometry and gesture cancellation through public APIs.
use pert_ui::*;
use std::{cell::Cell, rc::Rc, time::Duration};
fn draw(ui: &mut Runtime) -> DisplayList {
    ui.flush(Duration::ZERO).unwrap();
    ui.paint().unwrap()
}
#[test]
fn zoom_and_native_dpi_compose_and_noop_changes_settle() {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(800.0, 600.0, 1.5).unwrap())
        .unwrap();
    let root = ui
        .mount_tree(Element::new("label", Label::new("Zoom text")))
        .unwrap();
    let original = draw(&mut ui);
    let settings = ui.ui_settings();
    for factor in [0.75, 1.25, 2.0, 1.0] {
        settings.set_zoom(UiZoom::new(factor).unwrap()).unwrap();
        let scene = draw(&mut ui);
        assert_eq!(scene.viewport().scale(), 1.5 * factor);
        assert!(
            (f64::from(scene.viewport().width()) * scene.viewport().scale() - 1200.0).abs() < 0.001
        );
        assert_eq!(ui.key(&root).unwrap(), "label");
        assert!(!settings.set_zoom(UiZoom::new(factor).unwrap()).unwrap());
        assert_eq!(ui.flush(Duration::ZERO).unwrap().components_updated, 0);
        assert!(!ui.needs_paint());
        assert_eq!(ui.next_deadline(), None);
    }
    assert_eq!(
        original.masks()[0].pixels().as_ptr(),
        draw(&mut ui).masks()[0].pixels().as_ptr()
    );
    settings.set_zoom(UiZoom::new(1.25).unwrap()).unwrap();
    ui.set_viewport(Viewport::new(600.0, 400.0, 2.0).unwrap())
        .unwrap();
    assert_eq!(draw(&mut ui).viewport().scale(), 2.5);
    assert_eq!(settings.zoom().unwrap().get(), 1.25);
    ui.theme().set_density(Density::Compact).unwrap();
    ui.theme().set_mode(ThemeMode::Light).unwrap();
    draw(&mut ui);
    assert_eq!(settings.zoom().unwrap().get(), 1.25);
    drop(ui);
    assert_eq!(
        settings.set_zoom(UiZoom::default()),
        Err(RuntimeError::Closed)
    );
}
#[test]
fn zoom_validation_and_steps_do_not_silently_clamp_external_values() {
    for invalid in [f64::NAN, f64::INFINITY, -1.0, 0.0, 0.74, 2.01] {
        assert!(UiZoom::try_from(invalid).is_err());
    }
    let mut zoom = UiZoom::new(0.75).unwrap();
    for expected in [0.9, 1.0, 1.1, 1.25, 1.5, 1.75, 2.0, 2.0] {
        zoom = zoom.increased();
        assert_eq!(zoom.get(), expected);
    }
    assert_eq!(UiZoom::new(0.75).unwrap().decreased().get(), 0.75);
    assert_eq!(UiZoom::new(1.2).unwrap().decreased().get(), 1.1);
}
#[test]
fn zoom_cancels_a_pressed_button_without_activation_and_keeps_focus() {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(400.0, 200.0, 1.0).unwrap())
        .unwrap();
    let clicks = Rc::new(Cell::new(0));
    let result = clicks.clone();
    let button = ui
        .mount_tree(Element::new(
            "button",
            Button::new("Action", move |_| {
                result.set(result.get() + 1);
                Ok(())
            }),
        ))
        .unwrap();
    draw(&mut ui);
    ui.pointer_input(PointerEvent::new(
        Point::new(20.0, 20.0).unwrap(),
        PointerKind::Press(PointerButton::Primary),
    ))
    .unwrap();
    ui.ui_settings()
        .set_zoom(UiZoom::new(2.0).unwrap())
        .unwrap();
    // Dispatch can flush the pending coordinate change itself.
    ui.pointer_input(PointerEvent::new(
        Point::new(10.0, 10.0).unwrap(),
        PointerKind::Release(PointerButton::Primary),
    ))
    .unwrap();
    draw(&mut ui);
    assert_eq!(clicks.get(), 0);
    assert_eq!(ui.focused(), Some(button));
    ui.pointer_input(PointerEvent::new(
        Point::new(10.0, 10.0).unwrap(),
        PointerKind::Press(PointerButton::Primary),
    ))
    .unwrap();
    ui.pointer_input(PointerEvent::new(
        Point::new(10.0, 10.0).unwrap(),
        PointerKind::Release(PointerButton::Primary),
    ))
    .unwrap();
    assert_eq!(clicks.get(), 1);
}
#[test]
fn zoom_cancels_split_preview_and_preserves_preferred_sizing() {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(408.0, 200.0, 1.0).unwrap())
        .unwrap();
    let split = ui
        .mount_tree(Element::new(
            "split",
            SplitPane::new(
                SplitAxis::LeftRight,
                Element::new("a", Label::new("A")),
                Element::new("b", Label::new("B")),
            )
            .sizing(SplitSizing::first(100.into())),
        ))
        .unwrap();
    draw(&mut ui);
    let first = ui.children(&split).unwrap()[0].clone();
    ui.pointer_input(PointerEvent::new(
        Point::new(104.0, 30.0).unwrap(),
        PointerKind::Press(PointerButton::Primary),
    ))
    .unwrap();
    ui.pointer_input(PointerEvent::new(
        Point::new(150.0, 30.0).unwrap(),
        PointerKind::Move,
    ))
    .unwrap();
    draw(&mut ui);
    assert!(ui.bounds(&first).unwrap().unwrap().width() > 100.0);
    ui.ui_settings()
        .set_zoom(UiZoom::new(1.25).unwrap())
        .unwrap();
    draw(&mut ui);
    assert_eq!(ui.bounds(&first).unwrap().unwrap().width(), 100.0);
}

struct CaptureProbe {
    armed: Rc<Cell<bool>>,
    cancels: Rc<Cell<usize>>,
    commits: Rc<Cell<usize>>,
    focusable: bool,
    fail_leave: Rc<Cell<bool>>,
    change_on_cancel: Option<UiSettings>,
}
impl Component for CaptureProbe {
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
    fn focusable(&self) -> bool {
        self.focusable
    }
    fn measure(&self, _: &MeasureContext<'_>, _: Constraints) -> Result<Size, RuntimeError> {
        Ok(Size::from([80, 40]))
    }
    fn pointer_input(
        &mut self,
        cx: &mut EventContext,
        event: PointerEvent,
    ) -> Result<InputOutcome, RuntimeError> {
        match event.kind() {
            PointerKind::Press(PointerButton::Primary) => {
                self.armed.set(true);
                cx.capture_pointer()?;
            }
            PointerKind::Release(PointerButton::Primary) if self.armed.replace(false) => {
                self.commits.set(self.commits.get() + 1);
            }
            PointerKind::Leave if self.fail_leave.replace(false) => {
                return Err(RuntimeError::Exhausted);
            }
            _ => {}
        }
        Ok(InputOutcome::Ignored)
    }
    fn cancel_input(&mut self, cx: &mut EventContext) {
        self.cancels.set(self.cancels.get() + 1);
        self.armed.set(false);
        cx.release_pointer();
        if let Some(settings) = self.change_on_cancel.take() {
            settings.set_zoom(UiZoom::new(1.5).unwrap()).unwrap();
        }
    }
    fn cursor(&self, point: Point) -> Option<CursorIcon> {
        Some(if point.x() < 30.0 {
            CursorIcon::ResizeHorizontal
        } else {
            CursorIcon::ResizeVertical
        })
    }
}
#[test]
fn nonfocusable_capture_cancels_once_on_last_release_even_outside_and_with_coalesced_settings() {
    for zoom in [false, true] {
        let mut ui = Runtime::new();
        ui.set_viewport(Viewport::new(400.0, 200.0, 1.0).unwrap())
            .unwrap();
        let armed = Rc::new(Cell::new(false));
        let cancels = Rc::new(Cell::new(0));
        let commits = Rc::new(Cell::new(0));
        ui.mount_tree(Element::new(
            "row",
            Row::new([
                Element::new(
                    "probe",
                    CaptureProbe {
                        armed: armed.clone(),
                        cancels: cancels.clone(),
                        commits: commits.clone(),
                        focusable: false,
                        fail_leave: Rc::new(Cell::new(false)),
                        change_on_cancel: None,
                    },
                ),
                Element::new("other", Label::new("Other")),
            ]),
        ))
        .unwrap();
        draw(&mut ui);
        ui.pointer_input(PointerEvent::new(
            Point::new(20.0, 20.0).unwrap(),
            PointerKind::Press(PointerButton::Primary),
        ))
        .unwrap();
        ui.pointer_input(PointerEvent::new(
            Point::new(300.0, 150.0).unwrap(),
            PointerKind::Move,
        ))
        .unwrap();
        assert!(armed.get());
        if zoom {
            ui.ui_settings()
                .set_zoom(UiZoom::new(2.0).unwrap())
                .unwrap();
        }
        ui.theme().set_density(Density::Compact).unwrap();
        ui.pointer_input(PointerEvent::new(
            Point::new(300.0, 150.0).unwrap(),
            PointerKind::Release(PointerButton::Primary),
        ))
        .unwrap();
        draw(&mut ui);
        assert!(!armed.get());
        assert_eq!(cancels.get(), 1);
        assert_eq!(commits.get(), 0);
    }
}
#[test]
fn failed_leave_keeps_scaled_coordinates_and_does_not_repeat_cancellation() {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(400.0, 200.0, 1.0).unwrap())
        .unwrap();
    let cancels = Rc::new(Cell::new(0));
    let fail = Rc::new(Cell::new(false));
    let root = ui
        .mount_tree(Element::new(
            "probe",
            CaptureProbe {
                armed: Rc::new(Cell::new(false)),
                cancels: cancels.clone(),
                commits: Rc::new(Cell::new(0)),
                focusable: true,
                fail_leave: fail.clone(),
                change_on_cancel: None,
            },
        ))
        .unwrap();
    draw(&mut ui);
    ui.pointer_input(PointerEvent::new(
        Point::new(40.0, 20.0).unwrap(),
        PointerKind::Press(PointerButton::Primary),
    ))
    .unwrap();
    fail.set(true);
    ui.ui_settings()
        .set_zoom(UiZoom::new(2.0).unwrap())
        .unwrap();
    assert_eq!(
        ui.pointer_input(PointerEvent::new(
            Point::new(20.0, 10.0).unwrap(),
            PointerKind::Release(PointerButton::Primary)
        )),
        Err(RuntimeError::Exhausted)
    );
    draw(&mut ui);
    assert_eq!(cancels.get(), 1);
    assert_eq!(ui.cursor_icon(), CursorIcon::ResizeHorizontal);
    assert_eq!(ui.focused(), Some(root));
    ui.ui_settings()
        .set_zoom(UiZoom::new(1.25).unwrap())
        .unwrap();
    draw(&mut ui);
    assert_eq!(cancels.get(), 1, "idle focus must not be reset");
}
#[test]
fn cancellation_callback_zoom_writes_coalesce_without_double_rescaling() {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(400.0, 200.0, 1.0).unwrap())
        .unwrap();
    let cancels = Rc::new(Cell::new(0));
    ui.mount_tree(Element::new(
        "probe",
        CaptureProbe {
            armed: Rc::new(Cell::new(false)),
            cancels: cancels.clone(),
            commits: Rc::new(Cell::new(0)),
            focusable: false,
            fail_leave: Rc::new(Cell::new(false)),
            change_on_cancel: Some(ui.ui_settings()),
        },
    ))
    .unwrap();
    draw(&mut ui);
    ui.pointer_input(PointerEvent::new(
        Point::new(40.0, 20.0).unwrap(),
        PointerKind::Press(PointerButton::Primary),
    ))
    .unwrap();
    ui.ui_settings()
        .set_zoom(UiZoom::new(2.0).unwrap())
        .unwrap();
    draw(&mut ui);
    assert_eq!(ui.viewport().scale(), 1.5);
    assert_eq!(cancels.get(), 1);
    assert_eq!(ui.cursor_icon(), CursorIcon::ResizeHorizontal);
    assert_eq!(ui.flush(Duration::ZERO).unwrap().components_updated, 0);
    assert_eq!(ui.cursor_icon(), CursorIcon::ResizeHorizontal);
    assert_eq!(ui.next_deadline(), None);
}
#[test]
fn pending_zoom_cancels_keyboard_activation_before_last_key_release() {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(200.0, 100.0, 1.0).unwrap())
        .unwrap();
    let clicks = Rc::new(Cell::new(0));
    let count = clicks.clone();
    let id = ui
        .mount_tree(Element::new(
            "button",
            Button::new("Press", move |_| {
                count.set(count.get() + 1);
                Ok(())
            }),
        ))
        .unwrap();
    draw(&mut ui);
    ui.focus(&id).unwrap();
    ui.key_input(KeyCode::Space, KeyAction::Press).unwrap();
    ui.ui_settings()
        .set_zoom(UiZoom::new(1.25).unwrap())
        .unwrap();
    ui.key_input(KeyCode::Space, KeyAction::Release).unwrap();
    draw(&mut ui);
    assert_eq!(clicks.get(), 0);
    assert!(!ui.keyboard().is_pressed(KeyCode::Space));
}
struct UnwrappedLine {
    text: String,
    width: Rc<Cell<f32>>,
}
impl Component for UnwrappedLine {
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
    fn measure(&self, cx: &MeasureContext<'_>, _: Constraints) -> Result<Size, RuntimeError> {
        let text = cx.text(&self.text, None, TextStyle::Body)?;
        self.width.set(text.size().width());
        Ok(text.size())
    }
}
#[test]
fn zoom_lowers_only_the_font_raster_scale_so_a_long_line_can_cross_the_raster_side() {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(800.0, 600.0, 1.0).unwrap())
        .unwrap();
    let width = Rc::new(Cell::new(0.0));
    ui.mount_tree(Element::new(
        "line",
        UnwrappedLine {
            text: "M".repeat(500),
            width: width.clone(),
        },
    ))
    .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let side = TextLimit::MAX_RASTER_SIDE as f32;
    let logical = width.get();
    assert!(
        logical < side && logical * UiZoom::MAX.get() as f32 > side,
        "the line fits at 100% and not at the largest zoom: {logical}"
    );
    let settings = ui.ui_settings();
    assert_eq!(settings.set_zoom(UiZoom::MAX), Ok(true));
    assert_eq!(
        ui.flush(Duration::ZERO),
        Err(RuntimeError::Text(TextError::TooLarge(
            TextLimit::RasterSide
        )))
    );
    assert_eq!(settings.set_zoom(UiZoom::default()), Ok(true));
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(width.get(), logical, "the line recovers at 100%");
}
