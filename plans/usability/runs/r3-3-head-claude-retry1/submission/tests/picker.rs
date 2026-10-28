//! Headless checks of the point picker and the application around it.

use pert_ui::DrawCommand;
use pert_ui::prelude::*;
use pert_ui::testing::*;
use point_picker::{PickerApp, build};

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// Builds the application on a headless runtime with a window of its size.
fn start() -> Result<(Runtime, PickerApp), Box<dyn std::error::Error>> {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(480.0, 320.0, 1.0)?)?;
    let app = build(&mut ui)?;
    Ok((ui, app))
}

/// The picker's `x` and `y` as inspection reports them, read without knowing
/// the picker's type: a query by inspection id at full detail.
fn inspected_point(ui: &Runtime, id: &WidgetId) -> Result<(f64, f64), Box<dyn std::error::Error>> {
    let query =
        InspectionQuery::components(Selector::id(ui.inspection_id(id)?)).detail(Detail::Full);
    let result = ui.query(&query);
    let component = result.components.first().ok_or("no such component")?;
    let number = |name: &str| match component.details.properties.get(name) {
        Some(InspectValue::Number(value)) => Ok(*value),
        other => Err(format!("property {name}: {other:?}")),
    };
    Ok((number("x")?, number("y")?))
}

/// The picker's bounds as `[x, y, width, height]`.
fn picker_bounds(ui: &Runtime, id: &WidgetId) -> Result<[f32; 4], Box<dyn std::error::Error>> {
    Ok(ui.bounds(id)?.ok_or("the picker is laid out")?.values())
}

fn pointer(x: f32, y: f32, kind: PointerKind) -> Result<PointerEvent, Box<dyn std::error::Error>> {
    Ok(PointerEvent::new(Point::new(x, y)?, kind))
}

const PRESS: PointerKind = PointerKind::Press(PointerButton::Primary);
const RELEASE: PointerKind = PointerKind::Release(PointerButton::Primary);

#[test]
fn drag_moves_the_handle_and_clamps_outside_the_area() -> TestResult {
    let (mut ui, app) = start()?;
    let mut driver = Driver::new(&mut ui);
    driver.settle()?;
    let picker = app.picker.clone();
    let [left, top, width, height] = picker_bounds(driver.runtime(), &picker)?;
    assert_eq!(width, height, "the area is square");
    let ui = driver.runtime_mut();

    // Pressing a quarter in from the left and top moves the handle there.
    ui.pointer_input(pointer(left + width * 0.25, top + height * 0.25, PRESS)?)?;
    driver.settle()?;
    assert_eq!(inspected_point(driver.runtime(), &picker)?, (0.25, 0.75));
    assert_eq!(
        driver.runtime().focused(),
        Some(picker.clone()),
        "a press focuses the picker"
    );

    // The drag continues outside the area, clamped to its edges.
    let ui = driver.runtime_mut();
    ui.pointer_input(pointer(
        left + width + 100.0,
        top + height + 100.0,
        PointerKind::Move,
    )?)?;
    driver.settle()?;
    assert_eq!(inspected_point(driver.runtime(), &picker)?, (1.0, 0.0));
    let ui = driver.runtime_mut();
    ui.pointer_input(pointer(left - 50.0, top + height * 0.5, PointerKind::Move)?)?;
    driver.settle()?;
    assert_eq!(inspected_point(driver.runtime(), &picker)?, (0.0, 0.5));

    // The release ends the drag, outside the area too; later moves do nothing.
    let ui = driver.runtime_mut();
    ui.pointer_input(pointer(left - 50.0, top - 50.0, PointerKind::Move)?)?;
    ui.pointer_input(pointer(left - 50.0, top - 50.0, RELEASE)?)?;
    ui.pointer_input(pointer(left + 10.0, top + 10.0, PointerKind::Move)?)?;
    driver.settle()?;
    assert_eq!(inspected_point(driver.runtime(), &picker)?, (0.0, 1.0));
    assert_eq!((app.x.get()?, app.y.get()?), (0.0, 1.0));
    assert_eq!(app.last_change.get()?, "Picked (0.00, 1.00)");
    Ok(())
}

#[test]
fn escape_cancels_a_drag_and_restores_the_old_point() -> TestResult {
    let (mut ui, app) = start()?;
    app.x.set(0.2)?;
    app.y.set(0.3)?;
    let mut driver = Driver::new(&mut ui);
    driver.settle()?;
    let picker = app.picker.clone();
    let [left, top, width, height] = picker_bounds(driver.runtime(), &picker)?;

    let ui = driver.runtime_mut();
    ui.pointer_input(pointer(left + width * 0.9, top + height * 0.1, PRESS)?)?;
    ui.pointer_input(pointer(
        left + width * 2.0,
        top + height * 0.5,
        PointerKind::Move,
    )?)?;
    driver.settle()?;
    assert_eq!(inspected_point(driver.runtime(), &picker)?, (1.0, 0.5));

    driver.key(KeyCode::Escape)?;
    assert_eq!(inspected_point(driver.runtime(), &picker)?, (0.2, 0.3));
    assert_eq!((app.x.get()?, app.y.get()?), (0.2, 0.3));

    // The cancelled drag is over: moving and releasing change nothing.
    let ui = driver.runtime_mut();
    ui.pointer_input(pointer(
        left + width * 0.5,
        top + height * 0.5,
        PointerKind::Move,
    )?)?;
    ui.pointer_input(pointer(left + width * 0.5, top + height * 0.5, RELEASE)?)?;
    driver.settle()?;
    assert_eq!(inspected_point(driver.runtime(), &picker)?, (0.2, 0.3));
    Ok(())
}

#[test]
fn focus_loss_during_a_drag_restores_the_old_point() -> TestResult {
    let (mut ui, app) = start()?;
    let mut driver = Driver::new(&mut ui);
    driver.settle()?;
    let picker = app.picker.clone();
    let [left, top, width, height] = picker_bounds(driver.runtime(), &picker)?;
    let ui = driver.runtime_mut();
    ui.pointer_input(pointer(left + width * 0.1, top + height * 0.1, PRESS)?)?;
    driver.settle()?;
    assert_eq!(inspected_point(driver.runtime(), &picker)?, (0.1, 0.9));
    driver.clear_focus()?;
    assert_eq!(inspected_point(driver.runtime(), &picker)?, (0.5, 0.5));
    Ok(())
}

#[test]
fn arrow_keys_move_the_point_with_and_without_shift() -> TestResult {
    let (mut ui, app) = start()?;
    let mut driver = Driver::new(&mut ui);
    driver.settle()?;
    let picker = app.picker.clone();
    driver.runtime_mut().focus(&picker)?;
    driver.settle()?;

    driver.key(KeyCode::ArrowRight)?;
    assert_eq!(inspected_point(driver.runtime(), &picker)?, (0.51, 0.5));
    driver.key(KeyCode::ArrowUp)?;
    assert_eq!(inspected_point(driver.runtime(), &picker)?, (0.51, 0.51));
    driver.key(KeyCode::ArrowLeft)?;
    driver.key(KeyCode::ArrowDown)?;
    driver.key(KeyCode::ArrowDown)?;
    assert_eq!(inspected_point(driver.runtime(), &picker)?, (0.5, 0.49));

    let shifted = |code| KeyChord::named(code).shifted();
    driver.chord(shifted(KeyCode::ArrowRight), KeyCode::ArrowRight)?;
    assert_eq!(inspected_point(driver.runtime(), &picker)?, (0.6, 0.49));
    driver.chord(shifted(KeyCode::ArrowDown), KeyCode::ArrowDown)?;
    assert_eq!(inspected_point(driver.runtime(), &picker)?, (0.6, 0.39));
    driver.chord(shifted(KeyCode::ArrowLeft), KeyCode::ArrowLeft)?;
    driver.chord(shifted(KeyCode::ArrowUp), KeyCode::ArrowUp)?;
    assert_eq!(inspected_point(driver.runtime(), &picker)?, (0.5, 0.49));

    // Steps stop at the edges.
    for _ in 0..8 {
        driver.chord(shifted(KeyCode::ArrowRight), KeyCode::ArrowRight)?;
    }
    assert_eq!(inspected_point(driver.runtime(), &picker)?, (1.0, 0.49));
    assert_eq!((app.x.get()?, app.y.get()?), (1.0, 0.49));
    Ok(())
}

#[test]
fn numeric_fields_move_the_handle_and_reset_restores_the_centre() -> TestResult {
    let (mut ui, app) = start()?;
    let mut driver = Driver::new(&mut ui);
    driver.settle()?;
    let picker = app.picker.clone();
    let x_field = driver.find(&app.root, &["layout", "fields", "x-field", "x"])?;
    let y_field = driver.find(&app.root, &["layout", "fields", "y-field", "y"])?;

    driver.replace_text(&x_field, "0.3")?;
    driver.key(KeyCode::Enter)?;
    driver.replace_text(&y_field, "0.8")?;
    driver.key(KeyCode::Enter)?;
    assert_eq!(inspected_point(driver.runtime(), &picker)?, (0.3, 0.8));

    // The other direction: the picker's keys show in the fields.
    driver.runtime_mut().focus(&picker)?;
    driver.key(KeyCode::ArrowRight)?;
    let number =
        |driver: &Driver<'_>, id| -> Result<_, TestError> { Ok(driver.semantics(id)?.value) };
    assert_eq!(
        number(&driver, &x_field)?,
        Some(SemanticValue::Number {
            value: 0.31,
            minimum: Some(0.0),
            maximum: Some(1.0)
        })
    );

    let reset = driver.find(&app.root, &["layout", "fields", "reset"])?;
    driver.click(&reset)?;
    assert_eq!(inspected_point(driver.runtime(), &picker)?, (0.5, 0.5));
    assert_eq!(
        number(&driver, &y_field)?,
        Some(SemanticValue::Number {
            value: 0.5,
            minimum: Some(0.0),
            maximum: Some(1.0)
        })
    );
    Ok(())
}

#[test]
fn the_value_appears_in_an_inspection_snapshot() -> TestResult {
    let (mut ui, app) = start()?;
    app.x.set(0.125)?;
    app.y.set(0.875)?;
    let mut driver = Driver::new(&mut ui);
    driver.settle()?;
    let snapshot = driver.runtime().inspect();
    let picker = snapshot
        .components
        .iter()
        .find(|component| component.key == "picker")
        .ok_or("the picker is in the snapshot")?;
    let semantics = picker
        .details
        .semantics
        .as_ref()
        .ok_or("the picker has semantics")?;
    assert_eq!(semantics.role, SemanticRole::Slider);
    assert_eq!(semantics.name.as_deref(), Some("Point"));
    assert_eq!(
        semantics.value,
        Some(SemanticValue::Text("x 0.125, y 0.875".into()))
    );
    assert_eq!(
        picker.details.properties.get("x"),
        Some(&InspectValue::Number(0.125))
    );
    assert_eq!(
        picker.details.properties.get("y"),
        Some(&InspectValue::Number(0.875))
    );
    Ok(())
}

#[test]
fn the_focus_ring_follows_the_theme() -> TestResult {
    let (mut ui, app) = start()?;
    let ring = Color::from_linear(1.0, 0.0, 0.5, 1.0)?;
    ui.theme()
        .set_color(ui.theme().mode()?, ColorRole::FocusRing, ring)?;
    let mut driver = Driver::new(&mut ui);
    driver.settle()?;
    let ring_sides = |ui: &mut Runtime| -> Result<usize, Box<dyn std::error::Error>> {
        Ok(ui
            .paint()?
            .commands()
            .iter()
            .filter(
                |command| matches!(command, DrawCommand::FillRect { color, .. } if *color == ring),
            )
            .count())
    };
    assert_eq!(
        ring_sides(driver.runtime_mut())?,
        0,
        "no ring while unfocused"
    );
    driver.runtime_mut().focus(&app.picker)?;
    driver.settle()?;
    assert_eq!(
        ring_sides(driver.runtime_mut())?,
        4,
        "four sides in the theme's ring color"
    );
    #[cfg(debug_assertions)]
    assert!(driver.runtime().validation_report().issues.is_empty());
    Ok(())
}

#[test]
fn cpu_captures_in_both_themes() -> TestResult {
    let directory = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("captures");
    std::fs::create_dir_all(&directory)?;
    for (name, mode) in [("dark", ThemeMode::Dark), ("light", ThemeMode::Light)] {
        let png = point_picker::headless::capture_png(mode)?;
        assert!(png.starts_with(b"\x89PNG"));
        std::fs::write(directory.join(format!("{name}.png")), png)?;
    }
    Ok(())
}
