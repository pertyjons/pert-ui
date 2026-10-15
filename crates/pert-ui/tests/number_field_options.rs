//! Step buttons, formats and units on the ordinary (framed) number fields.
use pert_ui::*;
mod support;
use std::time::Duration;
use support::{frame, key};

fn pointer(ui: &mut Runtime, point: Point, kind: PointerKind) {
    ui.pointer_input(PointerEvent::new(point, kind)).unwrap();
}
fn click(ui: &mut Runtime, point: Point) {
    pointer(ui, point, PointerKind::Press(PointerButton::Primary));
    pointer(ui, point, PointerKind::Release(PointerButton::Primary));
    frame(ui);
}
fn property(ui: &Runtime, name: &str) -> InspectValue {
    ui.inspect()
        .components
        .iter()
        .find(|c| c.key == "field")
        .unwrap()
        .details
        .properties[name]
        .clone()
}
fn metric(ui: &Runtime, role: MetricRole) -> f32 {
    ui.theme().metrics(Density::Standard).unwrap().get(role)
}
fn mount(ui: &mut Runtime, field: impl Component + 'static) -> (WidgetId, WidgetId) {
    ui.set_viewport(Viewport::new(400.0, 200.0, 1.0).unwrap())
        .unwrap();
    let root = ui
        .mount_tree(Element::new(
            "root",
            Column::new([
                Element::new("field", field),
                Element::new("other", Button::new("Other", |_| Ok(()))),
            ]),
        ))
        .unwrap();
    frame(ui);
    let children = ui.children(&root).unwrap();
    (children[0].clone(), children[1].clone())
}
/// The centers of an ordinary field's minus and plus buttons.
fn buttons(ui: &Runtime, id: &WidgetId) -> (Point, Point) {
    let [x, y, w, h] = ui.bounds(id).unwrap().unwrap().values();
    let pad = metric(ui, MetricRole::ControlPaddingX);
    let side = metric(ui, MetricRole::IconSize);
    let gap = metric(ui, MetricRole::IconGap);
    let plus = Point::new(x + w - pad - side / 2.0, y + h / 2.0).unwrap();
    let minus = Point::new(x + w - pad - side - gap - side / 2.0, y + h / 2.0).unwrap();
    (minus, plus)
}

#[test]
fn an_ordinary_field_steps_by_its_buttons_and_held_repeats() {
    let mut ui = Runtime::new();
    let count = ui.signal(10_i64);
    let (id, _) = mount(
        &mut ui,
        IntegerInput::new(count.clone())
            .range(0, 12)
            .unwrap()
            .stepper(true),
    );
    assert_eq!(property(&ui, "stepper"), InspectValue::Bool(true));
    // A focused ordinary field keeps a caret deadline, so the repeat's end is
    // observed through the value rather than through `next_deadline`.
    let (minus, plus) = buttons(&ui, &id);
    click(&mut ui, plus);
    assert_eq!(count.get().unwrap(), 11, "a press saves one step");
    click(&mut ui, minus);
    assert_eq!(count.get().unwrap(), 10);
    pointer(&mut ui, plus, PointerKind::Press(PointerButton::Primary));
    assert_eq!(count.get().unwrap(), 11);
    frame(&mut ui);
    assert!(ui.next_deadline().is_some());
    ui.flush(ui.now() + Duration::from_millis(450)).unwrap();
    assert_eq!(count.get().unwrap(), 12, "a held repeat saves on time");
    ui.flush(ui.now() + Duration::from_millis(500)).unwrap();
    assert_eq!(count.get().unwrap(), 12, "repeats never pass the bound");
    pointer(&mut ui, plus, PointerKind::Release(PointerButton::Primary));
    frame(&mut ui);
    // Escape ends a hold and keeps the steps already saved.
    pointer(&mut ui, minus, PointerKind::Press(PointerButton::Primary));
    ui.flush(ui.now() + Duration::from_millis(450)).unwrap();
    assert_eq!(count.get().unwrap(), 10);
    key(&mut ui, KeyCode::Escape);
    frame(&mut ui);
    ui.flush(ui.now() + Duration::from_millis(500)).unwrap();
    assert_eq!(count.get().unwrap(), 10, "Escape ends the repeat");
    pointer(&mut ui, minus, PointerKind::Release(PointerButton::Primary));
    frame(&mut ui);
    assert_eq!(count.get().unwrap(), 10);
    // Release ends the repeat too.
    pointer(&mut ui, minus, PointerKind::Press(PointerButton::Primary));
    pointer(&mut ui, minus, PointerKind::Release(PointerButton::Primary));
    ui.flush(ui.now() + Duration::from_millis(1000)).unwrap();
    assert_eq!(count.get().unwrap(), 9, "release ends the repeat");
    assert_eq!(property(&ui, "text"), InspectValue::Text("9".into()));
}

#[test]
fn a_held_button_ends_on_focus_loss_and_keeps_its_steps() {
    let mut ui = Runtime::new();
    let count = ui.signal(0_i64);
    let (id, other) = mount(&mut ui, IntegerInput::new(count.clone()).stepper(true));
    let (_, plus) = buttons(&ui, &id);
    pointer(&mut ui, plus, PointerKind::Press(PointerButton::Primary));
    ui.flush(ui.now() + Duration::from_millis(450)).unwrap();
    assert_eq!(count.get().unwrap(), 2);
    ui.focus(&other).unwrap();
    frame(&mut ui);
    ui.flush(ui.now() + Duration::from_millis(500)).unwrap();
    assert_eq!(count.get().unwrap(), 2, "no repeat after focus loss");
    assert!(ui.next_deadline().is_none(), "no timer and no caret remain");
}

#[test]
fn the_step_buttons_leave_the_text_area_and_do_not_type() {
    let mut ui = Runtime::new();
    let value = ui.signal(1.5_f64);
    let (id, _) = mount(
        &mut ui,
        DecimalInput::new(value.clone())
            .step(0.5)
            .unwrap()
            .stepper(true),
    );
    let (minus, _) = buttons(&ui, &id);
    ui.focus(&id).unwrap();
    frame(&mut ui);
    // A typed draft is saved together with the step.
    ui.text_input("0").unwrap();
    frame(&mut ui);
    click(&mut ui, minus);
    assert_eq!(value.get().unwrap(), 1.0, "the draft 1.50 minus one step");
    assert_eq!(property(&ui, "text"), InspectValue::Text("1".into()));
}

#[test]
fn a_read_only_field_shows_but_ignores_its_buttons() {
    let mut ui = Runtime::new();
    let count = ui.signal(5_i64);
    let (id, _) = mount(
        &mut ui,
        IntegerInput::new(count.clone()).stepper(true).read_only(),
    );
    let (_, plus) = buttons(&ui, &id);
    click(&mut ui, plus);
    assert_eq!(count.get().unwrap(), 5);
}

#[test]
fn an_ordinary_field_rests_on_its_display_text_and_edits_the_number() {
    let mut ui = Runtime::new();
    let seconds = ui.signal(0.25_f64);
    let milliseconds = NumberFormat::new(
        "9999.0",
        |v: f64| format!("{:.1}", v * 1000.0),
        |s| {
            s.trim()
                .parse::<f64>()
                .map(|v| v / 1000.0)
                .map_err(|_| NumberInputError::InvalidNumber)
        },
    )
    .edit_text(|v| (v * 1000.0).to_string());
    let (id, other) = mount(
        &mut ui,
        DecimalInput::new(seconds.clone())
            .format(milliseconds)
            .unit("ms"),
    );
    assert_eq!(property(&ui, "shows_display"), InspectValue::Bool(true));
    assert_eq!(
        property(&ui, "display_value"),
        InspectValue::Text("250.0".into())
    );
    assert_eq!(property(&ui, "unit"), InspectValue::Text("ms".into()));
    ui.focus(&id).unwrap();
    frame(&mut ui);
    assert_eq!(property(&ui, "shows_display"), InspectValue::Bool(false));
    assert_eq!(property(&ui, "text"), InspectValue::Text("250".into()));
    let modifier = KeyCode::primary();
    ui.key_input(modifier, KeyAction::Press).unwrap();
    key(&mut ui, KeyCode::KeyA);
    ui.key_input(modifier, KeyAction::Release).unwrap();
    ui.text_input("250.5").unwrap();
    ui.focus(&other).unwrap();
    frame(&mut ui);
    assert_eq!(
        seconds.get().unwrap(),
        0.2505,
        "edit text parses in milliseconds"
    );
    assert_eq!(
        property(&ui, "shows_display"),
        InspectValue::Bool(true),
        "the saved value rests on its display again"
    );
    assert_eq!(
        property(&ui, "display_value"),
        InspectValue::Text("250.5".into())
    );
    // An invalid draft keeps showing what the user typed.
    ui.focus(&id).unwrap();
    ui.text_input("x").unwrap();
    ui.focus(&other).unwrap();
    frame(&mut ui);
    assert_eq!(property(&ui, "shows_display"), InspectValue::Bool(false));
}

#[test]
fn a_field_without_a_format_or_unit_always_shows_its_editing_text() {
    let mut ui = Runtime::new();
    let count = ui.signal(3_i64);
    mount(&mut ui, IntegerInput::new(count));
    assert_eq!(property(&ui, "shows_display"), InspectValue::Bool(false));
}
