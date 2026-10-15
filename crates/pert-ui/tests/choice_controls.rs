//! Boolean and discrete control contracts through the public facade.
use pert_ui::*;
use std::time::Duration;
fn send(ui: &mut Runtime, y: f32, kind: PointerKind) {
    ui.pointer_input(PointerEvent::new(Point::new(10.0, y).unwrap(), kind))
        .unwrap();
}
fn setup() -> Runtime {
    let ui = Runtime::new();
    ui.set_viewport(Viewport::new(400.0, 300.0, 1.0).unwrap())
        .unwrap();
    ui
}
#[test]
fn shared_boolean_updates_cancelled_clicks_and_keyboard_repeats() {
    let mut ui = setup();
    let value = ui.signal(false);
    let root = ui
        .mount_tree(Element::new(
            "root",
            Column::new([
                Element::new("check", Checkbox::new("Enabled", value.clone())),
                Element::new("toggle", Toggle::new("Enabled", value.clone())),
                Element::new(
                    "disabled",
                    Checkbox::new("Disabled", value.clone()).disabled(),
                ),
            ]),
        ))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let ids = ui.children(&root).unwrap();
    ui.focus(&ids[0]).unwrap();
    ui.key_input(KeyCode::Space, KeyAction::Press).unwrap();
    ui.key_input(KeyCode::Space, KeyAction::Repeat).unwrap();
    assert!(!value.get().unwrap());
    ui.key_input(KeyCode::Space, KeyAction::Release).unwrap();
    assert!(value.get().unwrap());
    ui.flush(Duration::ZERO).unwrap();
    ui.paint().unwrap();
    assert_eq!(ui.flush(Duration::ZERO).unwrap().components_updated, 0);
    assert!(ui.next_deadline().is_none());
    send(&mut ui, 10.0, PointerKind::Press(PointerButton::Primary));
    ui.clear_focus();
    send(&mut ui, 10.0, PointerKind::Release(PointerButton::Primary));
    assert!(value.get().unwrap());
    ui.focus(&ids[1]).unwrap();
    ui.focus_next(false).unwrap();
    assert_eq!(ui.focused(), Some(ids[0].clone()));
    ui.key_input(KeyCode::Space, KeyAction::Press).unwrap();
    ui.set_visible(&root, false).unwrap();
    ui.set_visible(&root, true).unwrap();
    ui.focus(&ids[0]).unwrap();
    ui.key_input(KeyCode::Space, KeyAction::Release).unwrap();
    assert!(value.get().unwrap());
}
#[test]
fn radio_navigation_skips_disabled_and_reorder_never_retargets_press() {
    let mut ui = setup();
    let value = ui.signal("a");
    let choices = ui.signal(vec![
        Choice::new("a", "A"),
        Choice::new("b", "B").disabled(),
        Choice::new("c", "C"),
    ]);
    let id = ui
        .mount_tree(Element::new(
            "radio",
            RadioGroup::bound(value.clone(), choices.read_only()),
        ))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    ui.focus(&id).unwrap();
    ui.key_input(KeyCode::ArrowDown, KeyAction::Press).unwrap();
    assert_eq!(value.get().unwrap(), "c");
    ui.key_input(KeyCode::ArrowDown, KeyAction::Release)
        .unwrap();
    ui.key_input(KeyCode::ArrowDown, KeyAction::Press).unwrap();
    assert_eq!(value.get().unwrap(), "a");
    send(&mut ui, 10.0, PointerKind::Press(PointerButton::Primary));
    choices
        .set(vec![Choice::new("c", "C"), Choice::new("a", "A")])
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    send(&mut ui, 10.0, PointerKind::Release(PointerButton::Primary));
    assert_eq!(value.get().unwrap(), "a");
    value.set("missing").unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(value.get().unwrap(), "missing");
    choices.set(vec![]).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    ui.paint().unwrap();
}
#[test]
fn duplicate_values_are_errors_and_typed_values_need_no_float_encoding() {
    #[derive(Clone, Copy, Eq, PartialEq)]
    enum Mode {
        One,
        Two,
    }
    let mut ui = setup();
    let value = ui.signal(Mode::One);
    assert!(matches!(
        RadioGroup::new(
            value.clone(),
            [
                Choice::new(Mode::One, "One"),
                Choice::new(Mode::One, "Other")
            ]
        ),
        Err(ChoiceError::DuplicateValue)
    ));
    let source = ui.signal(vec![
        Choice::new(Mode::One, "One"),
        Choice::new(Mode::Two, "Two"),
    ]);
    ui.mount_tree(Element::new(
        "radio",
        RadioGroup::bound(value, source.read_only()),
    ))
    .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    source
        .set(vec![
            Choice::new(Mode::One, "One"),
            Choice::new(Mode::One, "Duplicate"),
        ])
        .unwrap();
    assert_eq!(
        ui.flush(Duration::ZERO),
        Err(RuntimeError::Choice(ChoiceError::DuplicateValue))
    );
    source.set(vec![]).unwrap();
    ui.flush(Duration::ZERO).unwrap();
}

#[test]
fn successful_clicks_escape_outside_and_unowned_buttons() {
    let mut ui = setup();
    let value = ui.signal(false);
    let id = ui
        .mount_tree(Element::new(
            "checkbox",
            Checkbox::new("Enabled", value.clone()),
        ))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    for b in [
        PointerButton::Secondary,
        PointerButton::Middle,
        PointerButton::Other(7),
    ] {
        assert_eq!(
            ui.pointer_input(PointerEvent::new(
                Point::new(10.0, 10.0).unwrap(),
                PointerKind::Press(b)
            ))
            .unwrap(),
            InputOutcome::Ignored
        );
        assert_eq!(
            ui.pointer_input(PointerEvent::new(
                Point::new(10.0, 10.0).unwrap(),
                PointerKind::Release(b)
            ))
            .unwrap(),
            InputOutcome::Ignored
        );
    }
    send(&mut ui, 10.0, PointerKind::Press(PointerButton::Primary));
    send(&mut ui, 10.0, PointerKind::Release(PointerButton::Primary));
    assert!(value.get().unwrap());
    send(&mut ui, 10.0, PointerKind::Press(PointerButton::Primary));
    ui.key_input(KeyCode::Escape, KeyAction::Press).unwrap();
    send(&mut ui, 10.0, PointerKind::Release(PointerButton::Primary));
    assert!(value.get().unwrap());
    send(&mut ui, 10.0, PointerKind::Press(PointerButton::Primary));
    ui.pointer_input(
        PointerEvent::new(
            Point::new(10.0, 10.0).unwrap(),
            PointerKind::Release(PointerButton::Primary),
        )
        .outside(),
    )
    .unwrap();
    assert!(value.get().unwrap());
    send(&mut ui, 10.0, PointerKind::Press(PointerButton::Primary));
    ui.unmount(&id).unwrap();
    ui.mount_tree(Element::new(
        "checkbox",
        Checkbox::new("Replacement", value.clone()),
    ))
    .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    send(&mut ui, 10.0, PointerKind::Release(PointerButton::Primary));
    assert!(value.get().unwrap());
}
#[test]
fn radio_primary_label_click_selects_and_secondary_propagates() {
    let mut ui = setup();
    let value = ui.signal(1);
    ui.mount_tree(Element::new(
        "radio",
        RadioGroup::new(
            value.clone(),
            [Choice::new(1, "One"), Choice::new(2, "Two")],
        )
        .unwrap(),
    ))
    .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    send(&mut ui, 45.0, PointerKind::Press(PointerButton::Primary));
    send(&mut ui, 45.0, PointerKind::Release(PointerButton::Primary));
    assert_eq!(value.get().unwrap(), 2);
    assert_eq!(
        ui.pointer_input(PointerEvent::new(
            Point::new(10.0, 10.0).unwrap(),
            PointerKind::Press(PointerButton::Secondary)
        ))
        .unwrap(),
        InputOutcome::Ignored
    );
}
