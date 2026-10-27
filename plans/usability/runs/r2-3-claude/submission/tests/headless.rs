//! Headless acceptance checks, driven through `App::headless` and `testing::Driver`.

use pert_ui::prelude::*;
use pert_ui::testing::*;
use pert_ui::{InspectValue, SemanticRole};
use point_picker::app::{self, Handles};

type Error = Box<dyn std::error::Error>;

fn start() -> Result<(HeadlessApp, Handles, WidgetId), Error> {
    let slot = app::Slot::default();
    let app = app::application(slot.clone()).headless()?;
    let handles = slot.borrow().clone().ok_or("application not built")?;
    let picker = app.runtime().find(&handles.root, app::PICKER)?.ok_or("no picker")?;
    Ok((app, handles, picker))
}

fn point(ui: &Runtime, picker: &WidgetId) -> Result<Point, Error> {
    let bounds = ui.bounds(picker)?.ok_or("picker not laid out")?;
    Ok(Point::new(bounds.x() + bounds.width() / 2.0, bounds.y() + bounds.height() / 2.0)?)
}

fn pointer(ui: &mut Runtime, x: f32, y: f32, kind: PointerKind) -> Result<(), Error> {
    ui.pointer_input(PointerEvent::new(Point::new(x, y)?, kind))?;
    Ok(())
}

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-9
}

#[test]
fn drag_outside_clamps() -> Result<(), Error> {
    let (mut app, h, picker) = start()?;
    let mut driver = Driver::new(app.runtime_mut());
    let bounds = driver.runtime().bounds(&picker)?.ok_or("no bounds")?;
    let middle = point(driver.runtime(), &picker)?;
    let ui = driver.runtime_mut();
    pointer(ui, middle.x(), middle.y(), PointerKind::Press(PointerButton::Primary))?;
    // Far above and to the left of the area: x clamps to 0, y to 1.
    pointer(ui, bounds.x() - 80.0, bounds.y() - 80.0, PointerKind::Move)?;
    driver.settle()?;
    assert!(close(h.x.get()?, 0.0) && close(h.y.get()?, 1.0), "{:?}", (h.x.get()?, h.y.get()?));
    // Far below and to the right; release there.
    let (far_x, far_y) = (bounds.x() + bounds.width() + 300.0, bounds.y() + bounds.height() + 300.0);
    let ui = driver.runtime_mut();
    pointer(ui, far_x, far_y, PointerKind::Move)?;
    pointer(ui, far_x, far_y, PointerKind::Release(PointerButton::Primary))?;
    driver.settle()?;
    assert!(close(h.x.get()?, 1.0) && close(h.y.get()?, 0.0));
    // The value through inspection, without knowing the picker's type.
    let semantics = driver.semantics(&picker)?;
    assert_eq!(semantics.value, Some(SemanticValue::Text("1.00, 0.00".into())));
    assert_eq!(h.edits.get()?, 1, "one committed edit");
    Ok(())
}

#[test]
fn escape_cancels_drag() -> Result<(), Error> {
    let (mut app, h, picker) = start()?;
    h.x.set(0.25)?;
    h.y.set(0.75)?;
    let mut driver = Driver::new(app.runtime_mut());
    driver.settle()?;
    let bounds = driver.runtime().bounds(&picker)?.ok_or("no bounds")?;
    let middle = point(driver.runtime(), &picker)?;
    let ui = driver.runtime_mut();
    pointer(ui, middle.x(), middle.y(), PointerKind::Press(PointerButton::Primary))?;
    pointer(ui, bounds.x() + bounds.width() + 50.0, middle.y(), PointerKind::Move)?;
    driver.settle()?;
    assert!(close(h.x.get()?, 1.0), "the drag moved the point");
    driver.key(KeyCode::Escape)?;
    assert!(close(h.x.get()?, 0.25) && close(h.y.get()?, 0.75), "{:?}", (h.x.get()?, h.y.get()?));
    // The release after the cancel changes nothing.
    pointer(driver.runtime_mut(), middle.x(), middle.y(), PointerKind::Release(PointerButton::Primary))?;
    driver.settle()?;
    assert!(close(h.x.get()?, 0.25) && close(h.y.get()?, 0.75));
    assert_eq!(h.edits.get()?, 0, "a cancelled drag is not reported");
    Ok(())
}

#[test]
fn arrow_keys_step() -> Result<(), Error> {
    let (mut app, h, picker) = start()?;
    let mut driver = Driver::new(app.runtime_mut());
    driver.runtime_mut().focus(&picker)?;
    driver.key(KeyCode::ArrowRight)?;
    driver.key(KeyCode::ArrowUp)?;
    assert!(close(h.x.get()?, 0.51) && close(h.y.get()?, 0.51), "{:?}", (h.x.get()?, h.y.get()?));
    driver.chord(KeyChord::named(KeyCode::ArrowLeft).shifted(), KeyCode::ArrowLeft)?;
    driver.chord(KeyChord::named(KeyCode::ArrowDown).shifted(), KeyCode::ArrowDown)?;
    assert!(close(h.x.get()?, 0.41) && close(h.y.get()?, 0.41), "{:?}", (h.x.get()?, h.y.get()?));
    assert_eq!(h.edits.get()?, 4);
    Ok(())
}

fn field_value(driver: &Driver, field: &WidgetId) -> Result<f64, Error> {
    match driver.semantics(field)?.value {
        Some(SemanticValue::Number { value, .. }) => Ok(value),
        other => Err(format!("not a number: {other:?}").into()),
    }
}

#[test]
fn fields_move_handle_and_reset_restores() -> Result<(), Error> {
    let (mut app, h, picker) = start()?;
    let mut driver = Driver::new(app.runtime_mut());
    let x_field = driver.find(&h.root, app::X_FIELD)?;
    let y_field = driver.find(&h.root, app::Y_FIELD)?;
    driver.replace_text(&x_field, "0.2")?;
    driver.key(KeyCode::Enter)?;
    driver.replace_text(&y_field, "0.9")?;
    driver.key(KeyCode::Enter)?;
    assert!(close(h.x.get()?, 0.2) && close(h.y.get()?, 0.9));
    // The handle moved: the picker shows the new point.
    assert_eq!(driver.semantics(&picker)?.value, Some(SemanticValue::Text("0.20, 0.90".into())));
    // And the other direction: the picker's keys update the fields.
    driver.runtime_mut().focus(&picker)?;
    driver.key(KeyCode::ArrowRight)?;
    assert_eq!(field_value(&driver, &x_field)?, 0.21);
    let reset = driver.find(&h.root, app::RESET)?;
    driver.click(&reset)?;
    assert!(close(h.x.get()?, 0.5) && close(h.y.get()?, 0.5));
    assert_eq!(driver.semantics(&picker)?.value, Some(SemanticValue::Text("0.50, 0.50".into())));
    assert_eq!(field_value(&driver, &x_field)?, 0.5);
    assert_eq!(field_value(&driver, &y_field)?, 0.5);
    Ok(())
}

#[test]
fn value_in_inspection_snapshot() -> Result<(), Error> {
    let (mut app, h, _) = start()?;
    h.x.set(0.125)?;
    h.y.set(0.875)?;
    Driver::new(app.runtime_mut()).settle()?;
    let snapshot = app.runtime().inspect();
    let picker = snapshot
        .components
        .iter()
        .find(|c| c.key == "picker")
        .ok_or("picker not in snapshot")?;
    let semantics = picker.details.semantics.as_ref().ok_or("no semantics")?;
    assert_eq!(semantics.role, SemanticRole::Slider);
    assert_eq!(semantics.value, Some(SemanticValue::Text("0.12, 0.88".into())));
    assert_eq!(picker.details.properties.get("x"), Some(&InspectValue::Number(0.125)));
    assert_eq!(picker.details.properties.get("y"), Some(&InspectValue::Number(0.875)));
    Ok(())
}

#[test]
fn cpu_captures_in_both_themes() -> Result<(), Error> {
    let (mut app, h, picker) = start()?;
    let dir = std::path::Path::new(env!("CARGO_TARGET_TMPDIR"));
    for (mode, name) in [(pert_ui::ThemeMode::Dark, "dark.png"), (pert_ui::ThemeMode::Light, "light.png")] {
        let png = app::capture_png(&mut app, &h, mode)?;
        assert_eq!(&png[1..4], b"PNG");
        std::fs::write(dir.join(name), png)?;
    }
    assert_eq!(app.runtime().focused(), Some(picker));
    Ok(())
}

#[test]
fn focus_loss_cancels_drag_and_layout_validates() -> Result<(), Error> {
    let (mut app, h, picker) = start()?;
    let mut driver = Driver::new(app.runtime_mut());
    let middle = point(driver.runtime(), &picker)?;
    let bounds = driver.runtime().bounds(&picker)?.ok_or("no bounds")?;
    let ui = driver.runtime_mut();
    pointer(ui, middle.x(), middle.y(), PointerKind::Press(PointerButton::Primary))?;
    pointer(ui, bounds.x(), bounds.y(), PointerKind::Move)?;
    driver.settle()?;
    assert!(!close(h.x.get()?, 0.5));
    let reset = driver.find(&h.root, app::RESET)?;
    driver.runtime_mut().focus(&reset)?;
    driver.settle()?;
    assert!(close(h.x.get()?, 0.5) && close(h.y.get()?, 0.5), "{:?}", (h.x.get()?, h.y.get()?));
    let report = driver.runtime().validation_report();
    assert!(report.enabled && report.issues.is_empty(), "{report:#?}");
    Ok(())
}
