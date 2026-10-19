//! Inline toggles, choices, selects and editable labels through the public facade.
use crate::support;
use pert_ui::*;
use std::time::Duration;
use support::{find, frame, key};

fn pointer(ui: &mut Runtime, point: Point, kind: PointerKind) {
    ui.pointer_input(PointerEvent::new(point, kind)).unwrap();
}
fn mount(ui: &mut Runtime, control: impl Component + 'static) -> WidgetId {
    ui.set_viewport(Viewport::new(600.0, 300.0, 1.0).unwrap())
        .unwrap();
    let root = ui
        .mount_tree(Element::new(
            "root",
            Column::new([
                Element::new("field", control),
                Element::new("next", Button::new("Next", |_| Ok(()))),
            ]),
        ))
        .unwrap();
    frame(ui);
    ui.children(&root).unwrap()[0].clone()
}
fn center(ui: &Runtime, id: &WidgetId) -> Point {
    let [x, y, w, h] = ui.bounds(id).unwrap().unwrap().values();
    Point::new(x + w / 2.0, y + h / 2.0).unwrap()
}
fn click(ui: &mut Runtime, point: Point) {
    pointer(ui, point, PointerKind::Press(PointerButton::Primary));
    pointer(ui, point, PointerKind::Release(PointerButton::Primary));
    frame(ui);
}
fn property(ui: &Runtime, key: &str, name: &str) -> InspectValue {
    ui.inspect()
        .components
        .iter()
        .find(|c| c.key == key)
        .unwrap()
        .details
        .properties[name]
        .clone()
}
fn visible(ui: &Runtime, key: &str) -> Vec<bool> {
    ui.inspect()
        .components
        .iter()
        .filter(|c| c.key == key)
        .map(|c| c.visible)
        .collect()
}
fn band(ui: &Runtime) -> f32 {
    ui.theme()
        .metrics(Density::Standard)
        .unwrap()
        .get(MetricRole::ControlHeight)
}

#[test]
fn inline_toggle_is_square_toggles_by_key_and_pointer_and_names_itself() {
    let mut ui = Runtime::new();
    let muted = ui.signal(false);
    let id = mount(
        &mut ui,
        InlineToggle::new(muted.clone(), "Mute").icon(Icon::VolumeOff),
    );
    let [_, _, w, h] = ui.bounds(&id).unwrap().unwrap().values();
    assert_eq!((w, h), (band(&ui), band(&ui)));
    ui.focus(&id).unwrap();
    key(&mut ui, KeyCode::Space);
    assert!(muted.get().unwrap());
    let point = center(&ui, &id);
    click(&mut ui, point);
    assert!(!muted.get().unwrap());
    assert_eq!(
        property(&ui, "field", "text"),
        InspectValue::Text("Mute".into())
    );
    // Keyboard focus presents the name after the shared delay.
    frame(&mut ui);
    assert!(ui.next_deadline().is_some());
    assert_eq!(visible(&ui, "help"), vec![false]);
    ui.flush(ui.now() + Duration::from_millis(600)).unwrap();
    ui.paint().unwrap();
    assert_eq!(visible(&ui, "help"), vec![true]);
    assert_eq!(
        property(&ui, "field", "help_open"),
        InspectValue::Bool(true)
    );
    ui.clear_focus();
    frame(&mut ui);
    assert_eq!(visible(&ui, "help"), vec![false]);
    assert!(ui.next_deadline().is_none());
    assert!(
        ui.validation_report().issues.is_empty(),
        "{:?}",
        ui.validation_report().issues
    );
}

#[test]
fn disabled_inline_toggle_shows_state_without_input() {
    let mut ui = Runtime::new();
    let armed = ui.signal(true);
    let id = mount(
        &mut ui,
        InlineToggle::new(armed.clone(), "Arm")
            .glyph("R")
            .disabled(),
    );
    let point = center(&ui, &id);
    click(&mut ui, point);
    assert!(armed.get().unwrap());
    assert_eq!(property(&ui, "field", "checked"), InspectValue::Bool(true));
    ui.focus_next(false).unwrap();
    assert_ne!(
        ui.inspect()
            .components
            .iter()
            .find(|c| c.focused)
            .map(|c| c.key.clone()),
        Some("field".into())
    );
}

#[test]
fn inline_text_escape_ends_editing_even_when_rejection_clear_fails() {
    use std::{cell::Cell, rc::Rc};
    let mut ui = Runtime::new();
    let value = ui.signal("Original".to_owned());
    let fail = Rc::new(Cell::new(true));
    let fail_callback = fail.clone();
    let field = mount(
        &mut ui,
        InlineText::new(value.clone())
            .normalize(|_| Err(InputRejection::new("Refused")))
            .on_rejection(move |_, rejection| {
                if rejection.is_none() && fail_callback.get() {
                    Err(RuntimeError::Exhausted)
                } else {
                    Ok(())
                }
            }),
    );
    ui.focus(&field).unwrap();
    key(&mut ui, KeyCode::F2);
    ui.text_input("Draft").unwrap();
    key(&mut ui, KeyCode::Enter);
    assert_eq!(property(&ui, "field", "editing"), InspectValue::Bool(true));
    assert_eq!(
        ui.key_input(KeyCode::Escape, KeyAction::Press),
        Err(RuntimeError::Exhausted)
    );
    assert_eq!(property(&ui, "field", "editing"), InspectValue::Bool(false));
    assert_eq!(value.get().unwrap(), "Original");
    fail.set(false);
    ui.key_input(KeyCode::Escape, KeyAction::Release).unwrap();
    key(&mut ui, KeyCode::Escape);
    ui.focus(&field).unwrap();
    key(&mut ui, KeyCode::F2);
    assert_eq!(property(&ui, "field", "editing"), InspectValue::Bool(true));
}

#[test]
fn inline_choice_uses_square_icon_segments_keys_pointer_and_hover_help() {
    let mut ui = Runtime::new();
    let tool = ui.signal("select");
    let id = mount(
        &mut ui,
        InlineChoice::new(
            tool.clone(),
            [
                Choice::icon("select", Icon::Edit, "Select"),
                Choice::icon("erase", Icon::Delete, "Erase"),
                Choice::new("text", "Text"),
            ],
        )
        .unwrap(),
    );
    let [x, y, w, h] = ui.bounds(&id).unwrap().unwrap().values();
    assert_eq!(h, band(&ui));
    assert!(w > 2.0 * band(&ui), "two squares plus a text segment");
    ui.focus(&id).unwrap();
    key(&mut ui, KeyCode::ArrowRight);
    assert_eq!(tool.get().unwrap(), "erase");
    ui.clear_focus();
    // Click inside the first square selects it.
    let first = Point::new(x + band(&ui) / 2.0, y + h / 2.0).unwrap();
    click(&mut ui, first);
    assert_eq!(tool.get().unwrap(), "select");
    // Hovering the second square names it after the shared delay.
    let second = Point::new(x + band(&ui) * 1.5, y + h / 2.0).unwrap();
    pointer(&mut ui, second, PointerKind::Move);
    frame(&mut ui);
    ui.flush(ui.now() + Duration::from_millis(600)).unwrap();
    ui.paint().unwrap();
    assert_eq!(
        property(&ui, "field", "help_text"),
        InspectValue::Text("Erase".into())
    );
    assert_eq!(visible(&ui, "help"), vec![true]);
    pointer(&mut ui, Point::new(5.0, 290.0).unwrap(), PointerKind::Move);
    ui.clear_focus();
    frame(&mut ui);
    assert_eq!(visible(&ui, "help"), vec![false]);
    assert!(ui.next_deadline().is_none());
    assert!(
        ui.validation_report().issues.is_empty(),
        "{:?}",
        ui.validation_report().issues
    );
}

#[test]
fn inline_select_keeps_width_across_values_and_commits_only_from_the_list() {
    let mut ui = Runtime::new();
    let snap = ui.signal("1/4");
    let select = InlineSelect::new(
        &ui,
        snap.clone(),
        [
            Choice::new("1/4", "1/4"),
            Choice::new("1/16", "1/16 triplet"),
            Choice::new("off", "Off"),
        ],
    )
    .unwrap()
    .label("Snap");
    let id = mount(&mut ui, select);
    let anchor = find(&ui, &id, "select").unwrap();
    let before = ui.bounds(&anchor).unwrap().unwrap();
    assert_eq!(before.height(), band(&ui));
    snap.set("1/16").unwrap();
    frame(&mut ui);
    assert_eq!(ui.bounds(&anchor).unwrap().unwrap(), before);
    ui.focus(&anchor).unwrap();
    key(&mut ui, KeyCode::ArrowDown);
    frame(&mut ui);
    assert_eq!(snap.get().unwrap(), "1/16");
    assert_eq!(visible(&ui, "popup"), vec![true]);
    key(&mut ui, KeyCode::ArrowDown);
    key(&mut ui, KeyCode::Enter);
    frame(&mut ui);
    assert_eq!(snap.get().unwrap(), "off");
    assert_eq!(visible(&ui, "popup"), vec![false]);
    assert_eq!(
        property(&ui, "select", "label"),
        InspectValue::Text("Snap".into())
    );
    assert!(
        ui.validation_report().issues.is_empty(),
        "{:?}",
        ui.validation_report().issues
    );
}

#[test]
fn inline_text_edits_as_a_draft_and_yields_to_external_writes() {
    let mut ui = Runtime::new();
    let name = ui.signal(String::from("Lead"));
    let id = mount(
        &mut ui,
        InlineText::new(name.clone()).placeholder("Untitled"),
    );
    let [_, _, w, h] = ui.bounds(&id).unwrap().unwrap().values();
    assert_eq!(h, band(&ui));
    assert!(ui.text_input_state().is_none());
    ui.focus(&id).unwrap();
    frame(&mut ui);
    assert!(ui.next_deadline().is_none(), "resting focus has no caret");
    key(&mut ui, KeyCode::F2);
    frame(&mut ui);
    assert!(ui.text_input_state().is_some());
    assert!(ui.next_deadline().is_some(), "editing blinks the caret");
    ui.text_input("Bass").unwrap();
    assert_eq!(name.get().unwrap(), "Lead");
    assert_eq!(
        property(&ui, "field", "draft"),
        InspectValue::Text("Bass".into())
    );
    key(&mut ui, KeyCode::Escape);
    frame(&mut ui);
    assert_eq!(name.get().unwrap(), "Lead");
    assert_eq!(property(&ui, "field", "editing"), InspectValue::Bool(false));
    key(&mut ui, KeyCode::Enter);
    ui.text_input("Keys").unwrap();
    key(&mut ui, KeyCode::Enter);
    frame(&mut ui);
    assert_eq!(name.get().unwrap(), "Keys");
    assert_eq!(
        ui.bounds(&id).unwrap().unwrap().width(),
        w,
        "width follows the widest of text and placeholder"
    );
    // Focus loss saves a draft.
    key(&mut ui, KeyCode::F2);
    ui.text_input("Pad").unwrap();
    ui.clear_focus();
    frame(&mut ui);
    assert_eq!(name.get().unwrap(), "Pad");
    assert!(ui.next_deadline().is_none());
    // An external write ends editing and wins.
    ui.focus(&id).unwrap();
    key(&mut ui, KeyCode::F2);
    ui.text_input("Draft").unwrap();
    name.set("External".into()).unwrap();
    frame(&mut ui);
    assert_eq!(property(&ui, "field", "editing"), InspectValue::Bool(false));
    key(&mut ui, KeyCode::Enter);
    key(&mut ui, KeyCode::Enter);
    frame(&mut ui);
    assert_eq!(name.get().unwrap(), "External");
    // Double-click begins editing; a slow second click does not.
    ui.clear_focus();
    let p = center(&ui, &id);
    click(&mut ui, p);
    assert_eq!(property(&ui, "field", "editing"), InspectValue::Bool(false));
    click(&mut ui, p);
    assert_eq!(property(&ui, "field", "editing"), InspectValue::Bool(true));
    key(&mut ui, KeyCode::Escape);
    ui.flush(ui.now() + Duration::from_secs(2)).unwrap();
    click(&mut ui, p);
    ui.flush(ui.now() + Duration::from_secs(2)).unwrap();
    click(&mut ui, p);
    assert_eq!(property(&ui, "field", "editing"), InspectValue::Bool(false));
    assert!(
        ui.validation_report().issues.is_empty(),
        "{:?}",
        ui.validation_report().issues
    );
}

#[test]
fn hidden_inline_text_discards_its_draft() {
    let mut ui = Runtime::new();
    let name = ui.signal(String::from("Lead"));
    let id = mount(&mut ui, InlineText::new(name.clone()));
    ui.focus(&id).unwrap();
    key(&mut ui, KeyCode::F2);
    ui.text_input("Gone").unwrap();
    ui.set_visible(&id, false).unwrap();
    frame(&mut ui);
    assert_eq!(name.get().unwrap(), "Lead");
    ui.set_visible(&id, true).unwrap();
    frame(&mut ui);
    assert_eq!(property(&ui, "field", "editing"), InspectValue::Bool(false));
    assert!(ui.next_deadline().is_none());
}

#[test]
fn inline_text_refuses_an_unacceptable_binding_without_failing_the_runtime() {
    let mut ui = Runtime::new();
    let name = ui.signal(String::from("Lead"));
    let id = mount(&mut ui, InlineText::new(name.clone()));
    ui.focus(&id).unwrap();
    key(&mut ui, KeyCode::F2);
    ui.text_input("Draft").unwrap();
    // A name read from a file with its line break, written without checking.
    assert!(TextField::check("Bass\n").is_err());
    name.set("Bass\n".into()).unwrap();
    frame(&mut ui);
    frame(&mut ui);
    // The refusal ends the draft unsaved and keeps the previous text.
    assert_eq!(property(&ui, "field", "editing"), InspectValue::Bool(false));
    assert_eq!(
        property(&ui, "field", "text"),
        InspectValue::Text("Lead".into())
    );
    assert!(matches!(
        property(&ui, "field", "binding_error"),
        InspectValue::Text(_)
    ));
    let semantics = ui
        .inspect()
        .components
        .into_iter()
        .find(|c| c.key == "field")
        .and_then(|c| c.details.semantics)
        .unwrap();
    assert!(semantics.read_only, "a refused label is read-only");
    assert!(!semantics.actions.contains(&SemanticAction::SetValue));
    // Editing cannot begin, so the application's value is never overwritten.
    key(&mut ui, KeyCode::F2);
    key(&mut ui, KeyCode::Enter);
    ui.text_input("typed").unwrap();
    ui.clear_focus();
    frame(&mut ui);
    assert_eq!(property(&ui, "field", "editing"), InspectValue::Bool(false));
    assert_eq!(name.get().unwrap(), "Bass\n");
    // Paint shows the refusal with the error outline.
    let error = ui
        .theme()
        .palette(ui.theme().mode().unwrap())
        .unwrap()
        .color(ColorRole::Error);
    ui.flush(ui.now()).unwrap();
    assert!(
        ui.paint()
            .unwrap()
            .commands()
            .iter()
            .any(|command| matches!(
                command,
                DrawCommand::FillRect { color, .. } | DrawCommand::DrawMask { color, .. }
                    if *color == error
            ))
    );
    // The next acceptable value ends the refusal and editing resumes.
    name.set("Bass".into()).unwrap();
    frame(&mut ui);
    assert_eq!(
        property(&ui, "field", "text"),
        InspectValue::Text("Bass".into())
    );
    assert!(
        !ui.inspect()
            .components
            .iter()
            .find(|c| c.key == "field")
            .unwrap()
            .details
            .properties
            .contains_key("binding_error")
    );
    ui.focus(&id).unwrap();
    key(&mut ui, KeyCode::F2);
    ui.text_input("Keys").unwrap();
    key(&mut ui, KeyCode::Enter);
    assert_eq!(name.get().unwrap(), "Keys");
}

#[test]
fn read_only_text_mode_claims_typed_keys_like_a_read_only_text_field() {
    fn typed(ui: &mut Runtime) -> KeyDispatch {
        let mut input =
            CommandKeyInput::new(KeyCode::KeyX, KeyAction::Press, CommandKey::Character('x'));
        input.has_text = true;
        let dispatch = ui.command_key_input(input).unwrap();
        ui.command_key_input(CommandKeyInput::new(
            KeyCode::KeyX,
            KeyAction::Release,
            CommandKey::Character('x'),
        ))
        .unwrap();
        dispatch
    }
    let mut ui = Runtime::new();
    let position = ui.signal(String::from("001:01:000"));
    let text = ui.signal(String::from("read only"));
    ui.set_viewport(Viewport::new(600.0, 300.0, 1.0).unwrap())
        .unwrap();
    let root = ui
        .mount_tree(Element::new(
            "root",
            Column::new([
                Element::new("field", InlineReadout::new(position.read_only())),
                Element::new("text", TextField::new(text.clone()).read_only()),
            ]),
        ))
        .unwrap();
    frame(&mut ui);
    let runs = ui.signal(0);
    let written = runs.clone();
    let _command = ui
        .register_command(
            "Plain key",
            CommandScope::Application,
            vec![KeyChord::new(
                CommandKey::Character('x'),
                Modifiers::default(),
            )],
            move |_, _| written.set(written.get()? + 1),
        )
        .unwrap();
    // A resting readout is not an editor: the command runs.
    let readout = find(&ui, &root, "field").unwrap();
    ui.focus(&readout).unwrap();
    frame(&mut ui);
    typed(&mut ui);
    assert_eq!(runs.get().unwrap(), 1);
    // A read-only TextField claims the key, and so does read-only text mode.
    ui.focus(&find(&ui, &root, "text").unwrap()).unwrap();
    frame(&mut ui);
    assert!(typed(&mut ui).deliver_text);
    assert_eq!(runs.get().unwrap(), 1);
    ui.focus(&readout).unwrap();
    key(&mut ui, KeyCode::F2);
    frame(&mut ui);
    assert_eq!(
        property(&ui, "field", "text_editing"),
        InspectValue::Bool(true)
    );
    assert!(typed(&mut ui).deliver_text);
    ui.text_input("x").unwrap();
    assert_eq!(runs.get().unwrap(), 1);
    assert_eq!(position.get().unwrap(), "001:01:000");
}

fn metric(ui: &Runtime, role: MetricRole) -> f32 {
    ui.theme().metrics(Density::Standard).unwrap().get(role)
}

#[test]
fn step_buttons_save_one_step_per_press_and_every_held_repeat() {
    let mut ui = Runtime::new();
    let count = ui.signal(10_i64);
    let id = mount(
        &mut ui,
        InlineInteger::new(count.clone())
            .range(0, 12)
            .unwrap()
            .label("Voices")
            .stepper(true),
    );
    assert_eq!(property(&ui, "field", "stepper"), InspectValue::Bool(true));
    let [x, y, w, h] = ui.bounds(&id).unwrap().unwrap().values();
    let pad = metric(&ui, MetricRole::ControlPaddingX) * 0.5;
    let side = metric(&ui, MetricRole::IconSize);
    let gap = metric(&ui, MetricRole::IconGap);
    let plus = Point::new(x + w - pad - side / 2.0, y + h / 2.0).unwrap();
    let minus = Point::new(x + w - pad - side - gap - side / 2.0, y + h / 2.0).unwrap();
    click(&mut ui, plus);
    assert_eq!(count.get().unwrap(), 11);
    click(&mut ui, minus);
    assert_eq!(count.get().unwrap(), 10);
    // Holding saves one further step per elapsed repeat, within bounds.
    pointer(&mut ui, plus, PointerKind::Press(PointerButton::Primary));
    assert_eq!(count.get().unwrap(), 11);
    frame(&mut ui);
    assert!(ui.next_deadline().is_some());
    ui.flush(ui.now() + Duration::from_millis(450)).unwrap();
    assert_eq!(
        property(&ui, "field", "display_value"),
        InspectValue::Text("12".into())
    );
    assert_eq!(
        count.get().unwrap(),
        12,
        "the repeat saved on time, without waiting for the release"
    );
    ui.flush(ui.now() + Duration::from_millis(500)).unwrap();
    assert_eq!(count.get().unwrap(), 12, "repeats never pass the bound");
    pointer(&mut ui, plus, PointerKind::Release(PointerButton::Primary));
    frame(&mut ui);
    assert_eq!(count.get().unwrap(), 12);
    assert!(ui.next_deadline().is_none(), "release ends the repeat");
    // Escape ends a hold and keeps the steps that were already saved.
    pointer(&mut ui, minus, PointerKind::Press(PointerButton::Primary));
    assert_eq!(count.get().unwrap(), 11);
    ui.flush(ui.now() + Duration::from_millis(600)).unwrap();
    assert_eq!(count.get().unwrap(), 10);
    key(&mut ui, KeyCode::Escape);
    pointer(&mut ui, minus, PointerKind::Release(PointerButton::Primary));
    frame(&mut ui);
    assert_eq!(count.get().unwrap(), 10);
    assert_eq!(
        property(&ui, "field", "display_value"),
        InspectValue::Text("10".into())
    );
    assert!(ui.next_deadline().is_none());
    assert!(
        ui.validation_report().issues.is_empty(),
        "{:?}",
        ui.validation_report().issues
    );
}

#[test]
fn inline_readout_reserves_width_and_allows_read_only_text_mode() {
    let mut ui = Runtime::new();
    let position = ui.signal(String::from("001:01:000"));
    let id = mount(
        &mut ui,
        InlineReadout::new(position.read_only())
            .label("Pos")
            .width_hint("000:00:000"),
    );
    let before = ui.bounds(&id).unwrap().unwrap();
    assert_eq!(before.height(), band(&ui));
    position.set("128:04:240".into()).unwrap();
    frame(&mut ui);
    assert_eq!(ui.bounds(&id).unwrap().unwrap(), before);
    assert_eq!(
        property(&ui, "field", "display_value"),
        InspectValue::Text("128:04:240".into())
    );
    ui.focus(&id).unwrap();
    frame(&mut ui);
    assert!(ui.next_deadline().is_none());
    key(&mut ui, KeyCode::F2);
    frame(&mut ui);
    assert_eq!(
        property(&ui, "field", "text_editing"),
        InspectValue::Bool(true)
    );
    assert!(
        ui.text_input_state().is_none(),
        "read-only text mode has no IME"
    );
    ui.text_input("junk").unwrap();
    assert_eq!(position.get().unwrap(), "128:04:240");
    // Text commands act on the selected value as the chords do: copy works,
    // and nothing can be written.
    #[derive(Clone, Default)]
    struct Memory(std::rc::Rc<std::cell::RefCell<String>>);
    impl Clipboard for Memory {
        fn read_text(&mut self) -> Result<String, ClipboardError> {
            Ok(self.0.borrow().clone())
        }
        fn write_text(&mut self, text: &str) -> Result<(), ClipboardError> {
            *self.0.borrow_mut() = text.to_owned();
            Ok(())
        }
    }
    let clipboard = Memory::default();
    ui.set_clipboard(clipboard.clone()).unwrap();
    assert_eq!(
        ui.text_command(&id, TextCommand::Copy).unwrap(),
        TextCommandOutcome::Handled
    );
    assert_eq!(&*clipboard.0.borrow(), "128:04:240");
    assert_eq!(
        ui.text_command(&id, TextCommand::Paste).unwrap(),
        TextCommandOutcome::Unavailable
    );
    key(&mut ui, KeyCode::Escape);
    frame(&mut ui);
    assert_eq!(
        property(&ui, "field", "text_editing"),
        InspectValue::Bool(false)
    );
    let mut ui = Runtime::new();
    let load = ui.signal(0.42_f64);
    let percent = NumberFormat::new(
        "100",
        |v: f64| format!("{:.0}", v * 100.0),
        |_| Err(NumberInputError::InvalidNumber),
    )
    .unit(|_| "%".into());
    let cpu = mount(
        &mut ui,
        InlineReadout::formatted(load.read_only(), percent).label("CPU"),
    );
    assert_eq!(
        property(&ui, "field", "display_value"),
        InspectValue::Text("42".into())
    );
    assert_eq!(
        property(&ui, "field", "unit"),
        InspectValue::Text("%".into())
    );
    let width = ui.bounds(&cpu).unwrap().unwrap().width();
    load.set(1.0).unwrap();
    frame(&mut ui);
    assert_eq!(ui.bounds(&cpu).unwrap().unwrap().width(), width);
    assert!(ui.next_deadline().is_none());
}

#[test]
fn a_failing_clipboard_leaves_an_inline_editors_secondary_press_alone() {
    struct Failing;
    impl Clipboard for Failing {
        fn read_text(&mut self) -> Result<String, ClipboardError> {
            Err(ClipboardError::Failed("boom".into()))
        }
        fn write_text(&mut self, _: &str) -> Result<(), ClipboardError> {
            Err(ClipboardError::Failed("boom".into()))
        }
    }
    let mut ui = Runtime::new();
    let value = ui.signal("Name".to_owned());
    let field = mount(&mut ui, InlineText::new(value));
    ui.set_clipboard(Failing).unwrap();
    ui.focus(&field).unwrap();
    key(&mut ui, KeyCode::Enter);
    frame(&mut ui);
    let point = center(&ui, &field);
    ui.pointer_input(PointerEvent::new(
        point,
        PointerKind::Press(PointerButton::Secondary),
    ))
    .unwrap();
    ui.pointer_input(PointerEvent::new(
        point,
        PointerKind::Release(PointerButton::Secondary),
    ))
    .unwrap();
    ui.key_input(KeyCode::ContextMenu, KeyAction::Press)
        .unwrap();
    ui.key_input(KeyCode::ContextMenu, KeyAction::Release)
        .unwrap();
    frame(&mut ui);
}

#[test]
fn inline_text_edits_on_a_double_click_near_the_first_press_only() {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(300.0, 100.0, 1.0).unwrap())
        .unwrap();
    let name = ui.signal("Track".to_owned());
    let field = ui
        .mount_tree(Element::new("field", InlineText::new(name)))
        .unwrap();
    ui.flush(std::time::Duration::ZERO).unwrap();
    let bounds = ui.bounds(&field).unwrap().unwrap();
    let editing = |ui: &Runtime| {
        ui.inspect()
            .components
            .into_iter()
            .find(|c| c.key == "field")
            .and_then(|c| c.details.properties.get("editing").cloned())
            == Some(InspectValue::Bool(true))
    };
    let press = |ui: &mut Runtime, dx: f32| {
        let at = Point::new(bounds.x() + 20.0 + dx, bounds.y() + bounds.height() / 2.0).unwrap();
        for kind in [
            PointerKind::Press(PointerButton::Primary),
            PointerKind::Release(PointerButton::Primary),
        ] {
            ui.pointer_input(PointerEvent::new(at, kind)).unwrap();
        }
        ui.flush(ui.now()).unwrap();
    };
    press(&mut ui, 0.0);
    press(&mut ui, 5.0);
    assert!(
        !editing(&ui),
        "a second press five pixels away is no double click"
    );
    ui.flush(ui.now() + std::time::Duration::from_secs(1))
        .unwrap();
    press(&mut ui, 0.0);
    press(&mut ui, 2.0);
    assert!(editing(&ui));
}

#[test]
fn inline_text_forgets_a_first_press_when_focus_leaves() {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(300.0, 100.0, 1.0).unwrap())
        .unwrap();
    let name = ui.signal("Track".to_owned());
    let root = ui
        .mount_tree(Element::new(
            "row",
            Row::new([
                Element::new("field", InlineText::new(name)),
                Element::new("other", Button::new("Other", |_| Ok(()))),
            ]),
        ))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let field = find(&ui, &root, "field").unwrap();
    let other = find(&ui, &root, "other").unwrap();
    let bounds = ui.bounds(&field).unwrap().unwrap();
    let at = Point::new(bounds.x() + 20.0, bounds.y() + bounds.height() / 2.0).unwrap();
    let press = |ui: &mut Runtime| {
        pointer(ui, at, PointerKind::Press(PointerButton::Primary));
        pointer(ui, at, PointerKind::Release(PointerButton::Primary));
        ui.flush(ui.now()).unwrap();
    };
    press(&mut ui);
    ui.focus(&other).unwrap();
    ui.flush(ui.now()).unwrap();
    press(&mut ui);
    let editing = ui
        .inspect()
        .components
        .into_iter()
        .find(|c| c.key == "field")
        .and_then(|c| c.details.properties.get("editing").cloned());
    assert_eq!(editing, Some(InspectValue::Bool(false)));
}
