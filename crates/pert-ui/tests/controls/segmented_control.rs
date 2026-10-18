//! Joined, mutually exclusive choices through the public facade.
use pert_ui::*;
use std::{cell::Cell, rc::Rc, time::Duration};
fn runtime() -> Runtime {
    let ui = Runtime::new();
    ui.set_viewport(Viewport::new(300.0, 40.0, 1.0).unwrap())
        .unwrap();
    ui
}
fn pointer(ui: &mut Runtime, x: f32, kind: PointerKind) -> Result<InputOutcome, RuntimeError> {
    ui.pointer_input(PointerEvent::new(Point::new(x, 15.0).unwrap(), kind))
}
fn click(ui: &mut Runtime, x: f32) {
    pointer(ui, x, PointerKind::Press(PointerButton::Primary)).unwrap();
    pointer(ui, x, PointerKind::Release(PointerButton::Primary)).unwrap();
}
#[test]
fn joined_choices_select_one_value_and_follow_external_state_without_callbacks() {
    let mut ui = runtime();
    let value = ui.signal("a");
    let calls = Rc::new(Cell::new(0));
    let observed = calls.clone();
    let id = ui
        .mount_tree(Element::new(
            "group",
            SegmentedControl::new(
                value.clone(),
                [
                    Choice::new("a", "Alpha"),
                    Choice::new("b", "Beta"),
                    Choice::new("c", "Gamma").disabled(),
                ],
            )
            .unwrap()
            .on_change(move |_, _| {
                observed.set(observed.get() + 1);
                Ok(())
            }),
        ))
        .unwrap();
    click(&mut ui, 150.0);
    assert_eq!(value.get().unwrap(), "b");
    assert_eq!(calls.get(), 1);
    click(&mut ui, 150.0);
    click(&mut ui, 250.0);
    assert_eq!(calls.get(), 1);
    ui.focus(&id).unwrap();
    ui.key_input(KeyCode::ArrowRight, KeyAction::Press).unwrap();
    ui.key_input(KeyCode::ArrowRight, KeyAction::Release)
        .unwrap();
    assert_eq!(value.get().unwrap(), "a");
    assert_eq!(calls.get(), 2);
    value.set("c").unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let selected = ui
        .inspect()
        .components
        .into_iter()
        .find(|c| c.key == "group")
        .unwrap();
    assert_eq!(
        selected.details.properties.get("selected"),
        Some(&InspectValue::Text("Gamma".into()))
    );
    assert_eq!(calls.get(), 2);
    value.set("absent").unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert!(
        !ui.inspect()
            .components
            .into_iter()
            .find(|c| c.key == "group")
            .unwrap()
            .details
            .properties
            .contains_key("selected")
    );
    ui.paint().unwrap();
    assert_eq!(ui.flush(Duration::ZERO).unwrap().components_updated, 0);
    assert!(ui.next_deadline().is_none());
}
#[test]
fn replacement_and_context_loss_cancel_segment_activation() {
    let mut ui = runtime();
    let value = ui.signal("a");
    let choices = ui.signal(vec![Choice::new("a", "Alpha"), Choice::new("b", "Beta")]);
    let id = ui
        .mount_tree(Element::new(
            "group",
            SegmentedControl::bound(value.clone(), choices.read_only()),
        ))
        .unwrap();
    pointer(&mut ui, 200.0, PointerKind::Press(PointerButton::Primary)).unwrap();
    choices
        .set(vec![Choice::new("b", "Beta"), Choice::new("a", "Alpha")])
        .unwrap();
    pointer(&mut ui, 200.0, PointerKind::Release(PointerButton::Primary)).unwrap();
    assert_eq!(value.get().unwrap(), "a");
    pointer(&mut ui, 50.0, PointerKind::Press(PointerButton::Primary)).unwrap();
    ui.clear_focus();
    pointer(&mut ui, 50.0, PointerKind::Release(PointerButton::Primary)).unwrap();
    assert_eq!(value.get().unwrap(), "a");
    pointer(&mut ui, 50.0, PointerKind::Press(PointerButton::Primary)).unwrap();
    pointer(&mut ui, 200.0, PointerKind::Release(PointerButton::Primary)).unwrap();
    assert_eq!(value.get().unwrap(), "a");
    ui.focus(&id).unwrap();
    ui.key_input(KeyCode::Home, KeyAction::Press).unwrap();
    assert_eq!(value.get().unwrap(), "b");
    ui.key_input(KeyCode::Space, KeyAction::Press).unwrap();
    ui.set_visible(&id, false).unwrap();
    ui.set_visible(&id, true).unwrap();
    ui.focus(&id).unwrap();
    ui.key_input(KeyCode::Space, KeyAction::Release).unwrap();
    assert_eq!(value.get().unwrap(), "b");
    choices
        .set(vec![Choice::new("a", "A"), Choice::new("a", "Duplicate")])
        .unwrap();
    assert_eq!(
        ui.flush(Duration::ZERO),
        Err(RuntimeError::Choice(ChoiceError::DuplicateValue))
    );
    choices.set(vec![]).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert!(ui.focused().is_none());
    ui.paint().unwrap();
}
#[test]
fn callback_failure_preserves_the_written_choice_without_replay() {
    let mut ui = runtime();
    let value = ui.signal(0);
    let calls = Rc::new(Cell::new(0));
    let observed = calls.clone();
    ui.mount_tree(Element::new(
        "group",
        SegmentedControl::new(
            value.clone(),
            [Choice::new(0, "Zero"), Choice::new(1, "One")],
        )
        .unwrap()
        .on_change(move |_, _| {
            observed.set(observed.get() + 1);
            Err(RuntimeError::Exhausted)
        }),
    ))
    .unwrap();
    pointer(&mut ui, 200.0, PointerKind::Press(PointerButton::Primary)).unwrap();
    assert_eq!(
        pointer(&mut ui, 200.0, PointerKind::Release(PointerButton::Primary)),
        Err(RuntimeError::Exhausted)
    );
    assert_eq!(value.get().unwrap(), 1);
    ui.cancel_input().unwrap();
    ui.flush(Duration::ZERO).unwrap();
    click(&mut ui, 200.0);
    assert_eq!(calls.get(), 1);
}

/// The group is drawn as joined fills only: selection is the tab palette's
/// selected fill, and nothing draws the accent underline or the baseline that
/// `Tabs` puts beneath a selected title. Unfocused, the accent color appears
/// nowhere, and outline-colored geometry is exactly the internal edges, one per
/// segment after the first. A group border or any other outline geometry would
/// change that count and should be reviewed against this contract.
#[test]
fn segments_carry_no_underline_or_baseline() {
    let mut ui = runtime();
    ui.theme().set_mode(ThemeMode::Dark).unwrap();
    let value = ui.signal("a");
    ui.mount_tree(Element::new(
        "group",
        SegmentedControl::new(
            value,
            [
                Choice::new("a", "Alpha"),
                Choice::new("b", "Beta"),
                Choice::new("c", "Gamma"),
            ],
        )
        .unwrap(),
    ))
    .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let scene = ui.paint().unwrap();
    let palette = ui.theme().palette(ThemeMode::Dark).unwrap();
    let tint = |command: &DrawCommand| match command {
        DrawCommand::FillRect { color, .. } | DrawCommand::DrawMask { color, .. } => Some(*color),
        _ => None,
    };
    let count = |role: ColorRole| {
        scene
            .commands()
            .iter()
            .filter(|command| tint(command) == Some(palette.color(role)))
            .count()
    };
    assert_eq!(
        count(ColorRole::Primary),
        0,
        "an unfocused group draws nothing in the accent color"
    );
    assert_eq!(
        count(ColorRole::Outline),
        2,
        "three segments have two internal edges and no horizontal rule"
    );
}
