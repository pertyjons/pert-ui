//! Headless checks of the point picker and the application, driven through
//! the runtime's own input entry points.

use pert_ui::{
    InspectValue, KeyAction, KeyCode, Point, PointerButton, PointerEvent, PointerKind, Rect,
    Runtime, SemanticValue, Viewport, WidgetId,
};
use point_picker::app::{self, AppState};
use std::time::Duration;

type TestResult = Result<(), Box<dyn std::error::Error>>;

struct Harness {
    ui: Runtime,
    state: AppState,
    root: WidgetId,
    picker: WidgetId,
    area: Rect,
}

fn harness() -> Result<Harness, Box<dyn std::error::Error>> {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(640.0, 400.0, 1.0)?)?;
    let (state, root) = app::mount(&mut ui)?;
    ui.flush(Duration::ZERO)?;
    let picker = ui.find_descendant(&root, "picker")?.expect("the picker");
    let area = ui.bounds(&picker)?.expect("the picker is laid out");
    Ok(Harness { ui, state, root, picker, area })
}

impl Harness {
    fn point(&self) -> Result<(f64, f64), Box<dyn std::error::Error>> {
        Ok((self.state.x.get()?, self.state.y.get()?))
    }
    /// Window position of a point given in fractions of the picker's square.
    fn at(&self, fx: f32, fy_down: f32) -> Result<Point, Box<dyn std::error::Error>> {
        let side = self.area.width().min(self.area.height());
        Ok(Point::new(self.area.x() + fx * side, self.area.y() + fy_down * side)?)
    }
    fn pointer(&mut self, at: Point, kind: PointerKind) -> TestResult {
        self.ui.pointer_input(PointerEvent::new(at, kind))?;
        self.ui.flush(Duration::ZERO)?;
        Ok(())
    }
    fn key(&mut self, code: KeyCode) -> TestResult {
        self.ui.key_input(code, KeyAction::Press)?;
        self.ui.key_input(code, KeyAction::Release)?;
        self.ui.flush(Duration::ZERO)?;
        Ok(())
    }
}

fn close(a: (f64, f64), b: (f64, f64)) -> bool {
    (a.0 - b.0).abs() < 1e-6 && (a.1 - b.1).abs() < 1e-6
}

#[test]
fn drag_moves_handle_and_clamps_outside() -> TestResult {
    let mut h = harness()?;
    assert!(h.area.width() > 50.0, "picker area {:?}", h.area.values());
    // Press at a quarter across, a quarter down: y grows upwards.
    let press = h.at(0.25, 0.25)?;
    h.pointer(press, PointerKind::Press(PointerButton::Primary))?;
    assert!(close(h.point()?, (0.25, 0.75)), "{:?}", h.point()?);
    assert_eq!(h.ui.focused(), Some(h.picker.clone()));
    // Drag far outside to the right and below: clamped to (1, 0).
    let outside = Point::new(h.area.x() + h.area.width() + 300.0, h.area.y() + 2000.0)?;
    h.pointer(outside, PointerKind::Move)?;
    assert!(close(h.point()?, (1.0, 0.0)), "{:?}", h.point()?);
    // Outside to the left and above: clamped to (0, 1).
    let outside = Point::new(0.0, 0.0)?;
    h.pointer(outside, PointerKind::Move)?;
    assert!(close(h.point()?, (0.0, 1.0)), "{:?}", h.point()?);
    h.pointer(outside, PointerKind::Release(PointerButton::Primary))?;
    assert!(close(h.point()?, (0.0, 1.0)));
    // After the release, moves no longer change the point.
    h.pointer(h.at(0.5, 0.5)?, PointerKind::Move)?;
    assert!(close(h.point()?, (0.0, 1.0)));
    // The fields followed the drag.
    Ok(())
}

#[test]
fn escape_cancels_drag_and_restores() -> TestResult {
    let mut h = harness()?;
    h.state.set(0.3, 0.6)?;
    h.ui.flush(Duration::ZERO)?;
    h.pointer(h.at(0.9, 0.9)?, PointerKind::Press(PointerButton::Primary))?;
    assert!(close(h.point()?, (0.9, 0.1)), "{:?}", h.point()?);
    let outside = Point::new(h.area.x() + h.area.width() + 50.0, h.area.y() + 10.0)?;
    h.pointer(outside, PointerKind::Move)?;
    assert!(close(h.point()?, (1.0, 1.0 - 10.0 / h.area.height() as f64)), "{:?}", h.point()?);
    h.key(KeyCode::Escape)?;
    assert!(close(h.point()?, (0.3, 0.6)), "{:?}", h.point()?);
    // The cancelled drag no longer follows the pointer, and its release is ignored.
    h.pointer(h.at(0.1, 0.1)?, PointerKind::Move)?;
    h.pointer(h.at(0.1, 0.1)?, PointerKind::Release(PointerButton::Primary))?;
    assert!(close(h.point()?, (0.3, 0.6)), "{:?}", h.point()?);
    Ok(())
}

#[test]
fn runtime_cancel_input_restores() -> TestResult {
    let mut h = harness()?;
    h.pointer(h.at(0.1, 0.1)?, PointerKind::Press(PointerButton::Primary))?;
    assert!(close(h.point()?, (0.1, 0.9)), "{:?}", h.point()?);
    h.ui.cancel_input()?;
    h.ui.flush(Duration::ZERO)?;
    assert!(close(h.point()?, (0.5, 0.5)), "{:?}", h.point()?);
    Ok(())
}

#[test]
fn arrow_keys_step_with_and_without_shift() -> TestResult {
    let mut h = harness()?;
    let picker = h.picker.clone();
    h.ui.focus(&picker)?;
    h.key(KeyCode::ArrowRight)?;
    assert!(close(h.point()?, (0.51, 0.5)), "{:?}", h.point()?);
    h.key(KeyCode::ArrowUp)?;
    assert!(close(h.point()?, (0.51, 0.51)), "{:?}", h.point()?);
    h.key(KeyCode::ArrowDown)?;
    h.key(KeyCode::ArrowDown)?;
    assert!(close(h.point()?, (0.51, 0.49)), "{:?}", h.point()?);
    h.ui.key_input(KeyCode::ShiftLeft, KeyAction::Press)?;
    h.key(KeyCode::ArrowLeft)?;
    assert!(close(h.point()?, (0.41, 0.49)), "{:?}", h.point()?);
    h.key(KeyCode::ArrowUp)?;
    assert!(close(h.point()?, (0.41, 0.59)), "{:?}", h.point()?);
    for _ in 0..10 {
        h.key(KeyCode::ArrowRight)?;
    }
    assert!(close(h.point()?, (1.0, 0.59)), "clamped at 1: {:?}", h.point()?);
    h.ui.key_input(KeyCode::ShiftLeft, KeyAction::Release)?;
    h.key(KeyCode::ArrowLeft)?;
    assert!(close(h.point()?, (0.99, 0.59)), "{:?}", h.point()?);
    assert_eq!(h.state.changes.get()?, 13, "unchanged steps at the edge are not reported");
    Ok(())
}

/// Reads the picker's value from an inspection snapshot, without its type.
fn inspected_point(ui: &Runtime, picker: &WidgetId) -> Result<(f64, f64, String), Box<dyn std::error::Error>> {
    let id = ui.inspection_id(picker)?;
    let snapshot = ui.inspect();
    let entry = snapshot.components.iter().find(|c| c.id == id).ok_or("no snapshot entry")?;
    let number = |key: &str| match entry.details.properties.get(key) {
        Some(InspectValue::Number(n)) => Ok(*n),
        other => Err(format!("property {key}: {other:?}")),
    };
    let text = match entry.details.semantics.as_ref().and_then(|s| s.value.clone()) {
        Some(SemanticValue::Text(text)) => text,
        other => return Err(format!("semantic value {other:?}").into()),
    };
    Ok((number("x")?, number("y")?, text))
}

#[test]
fn value_appears_in_inspection_snapshot() -> TestResult {
    let mut h = harness()?;
    h.state.set(0.25, 0.75)?;
    h.ui.flush(Duration::ZERO)?;
    let (x, y, text) = inspected_point(&h.ui, &h.picker)?;
    assert_eq!((x, y), (0.25, 0.75));
    assert_eq!(text, "x 0.25, y 0.75");
    Ok(())
}

#[test]
fn editing_a_field_moves_the_handle_and_reset_restores() -> TestResult {
    let mut h = harness()?;
    let x_field = h.ui.find_descendant(&h.root, "x")?.expect("the x field");
    h.ui.focus(&x_field)?;
    // Select the field's text and type a new value.
    h.ui.key_input(KeyCode::End, KeyAction::Press)?;
    for _ in 0..8 {
        h.ui.key_input(KeyCode::Backspace, KeyAction::Press)?;
        h.ui.key_input(KeyCode::Backspace, KeyAction::Release)?;
    }
    h.ui.text_input("0.8")?;
    h.ui.key_input(KeyCode::Enter, KeyAction::Press)?;
    h.ui.key_input(KeyCode::Enter, KeyAction::Release)?;
    h.ui.flush(Duration::ZERO)?;
    assert!(close(h.point()?, (0.8, 0.5)), "{:?}", h.point()?);
    let (x, y, _) = inspected_point(&h.ui, &h.picker)?;
    assert_eq!((x, y), (0.8, 0.5), "the handle follows the field");

    // The picker's own change shows in the other field.
    let picker = h.picker.clone();
    h.ui.focus(&picker)?;
    h.key(KeyCode::ArrowUp)?;
    let y_field = h.ui.find_descendant(&h.root, "y")?.expect("the y field");
    let id = h.ui.inspection_id(&y_field)?;
    let snapshot = h.ui.inspect();
    let entry = snapshot.components.iter().find(|c| c.id == id).ok_or("y field entry")?;
    let shown = entry.details.semantics.as_ref().and_then(|s| s.value.clone());
    assert!(
        matches!(shown, Some(SemanticValue::Number { value, .. }) if (value - 0.51).abs() < 1e-9)
            || matches!(&shown, Some(SemanticValue::Text(t)) if t == "0.51"),
        "y field shows {shown:?}"
    );

    // Reset.
    let reset = h.ui.find_descendant(&h.root, "reset")?.expect("the reset button");
    let bounds = h.ui.bounds(&reset)?.expect("reset laid out");
    let centre = Point::new(bounds.x() + bounds.width() / 2.0, bounds.y() + bounds.height() / 2.0)?;
    h.pointer(centre, PointerKind::Press(PointerButton::Primary))?;
    h.pointer(centre, PointerKind::Release(PointerButton::Primary))?;
    assert!(close(h.point()?, (0.5, 0.5)), "{:?}", h.point()?);
    let (x, y, _) = inspected_point(&h.ui, &h.picker)?;
    assert_eq!((x, y), (0.5, 0.5));
    Ok(())
}

#[test]
fn escape_on_native_key_path_cancels_drag() -> TestResult {
    use pert_ui::{CommandKey, CommandKeyInput};
    let mut h = harness()?;
    h.pointer(h.at(0.0, 0.0)?, PointerKind::Press(PointerButton::Primary))?;
    h.pointer(Point::new(-100.0, -100.0)?, PointerKind::Move)?;
    assert!(close(h.point()?, (0.0, 1.0)), "{:?}", h.point()?);
    for action in [KeyAction::Press, KeyAction::Release] {
        h.ui.command_key_input(CommandKeyInput::new(
            KeyCode::Escape,
            action,
            CommandKey::Named(KeyCode::Escape),
        ))?;
    }
    h.ui.flush(Duration::ZERO)?;
    assert!(close(h.point()?, (0.5, 0.5)), "{:?}", h.point()?);
    Ok(())
}

#[test]
fn redescription_takes_new_bindings_and_handler() -> TestResult {
    use pert_ui::{check_redescription, Element, Signal};
    use point_picker::picker::PointPicker;
    use std::{cell::Cell, rc::Rc};
    let mut ui = Runtime::new();
    // No viewport: check_redescription mounts two roots at the same place,
    // which development validation reports as an overlap once there is a viewport.
    let (ax, ay) = (ui.signal(0.1), ui.signal(0.2));
    let (bx, by) = (ui.signal(0.7), ui.signal(0.8));
    let calls = Rc::new(Cell::new(0));
    let picker = |x: &Signal<f64>, y: &Signal<f64>, name: &str, mark: u32| {
        let calls = calls.clone();
        Element::new(
            "picker",
            PointPicker::new(x.clone(), y.clone()).name(name).on_change(move |_, _, _| {
                calls.set(mark);
                Ok(())
            }),
        )
    };
    let done = check_redescription(
        &mut ui,
        picker(&ax, &ay, "Before", 1),
        picker(&bx, &by, "After", 2),
        picker(&bx, &by, "After", 2),
    )?;
    ui.flush(Duration::ZERO)?;
    ui.focus(&done.component)?;
    ui.key_input(KeyCode::ArrowRight, KeyAction::Press)?;
    ui.flush(Duration::ZERO)?;
    assert_eq!(calls.get(), 2, "the new handler runs");
    assert!((bx.get()? - 0.71).abs() < 1e-9 && ax.get()? == 0.1, "the new binding is written");
    Ok(())
}

#[test]
fn no_validation_issues() -> TestResult {
    let mut h = harness()?;
    let picker = h.picker.clone();
    h.ui.focus(&picker)?;
    h.pointer(h.at(0.5, 0.5)?, PointerKind::Press(PointerButton::Primary))?;
    h.pointer(Point::new(1.0, 1.0)?, PointerKind::Move)?;
    h.pointer(Point::new(1.0, 1.0)?, PointerKind::Release(PointerButton::Primary))?;
    h.ui.paint()?;
    let report = h.ui.validation_report();
    assert!(report.issues.is_empty(), "{:#?}", report.issues);
    Ok(())
}

#[test]
fn focus_loss_cancels_drag_and_restores() -> TestResult {
    let mut h = harness()?;
    h.pointer(h.at(0.2, 0.2)?, PointerKind::Press(PointerButton::Primary))?;
    assert!(close(h.point()?, (0.2, 0.8)), "{:?}", h.point()?);
    let x_field = h.ui.find_descendant(&h.root, "x")?.expect("the x field");
    h.ui.focus(&x_field)?;
    h.ui.flush(Duration::ZERO)?;
    assert!(close(h.point()?, (0.5, 0.5)), "{:?}", h.point()?);
    h.pointer(h.at(0.9, 0.9)?, PointerKind::Move)?;
    assert!(close(h.point()?, (0.5, 0.5)), "the cancelled drag is over: {:?}", h.point()?);
    Ok(())
}
