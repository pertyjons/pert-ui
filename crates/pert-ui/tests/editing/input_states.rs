//! Input field states through the public facade: concealed entry,
//! placeholder, required, length limits with a counter, and a pending field
//! status.
use crate::support;
use pert_ui::testing::HostIntegration;
use pert_ui::*;
use std::{cell::RefCell, rc::Rc, time::Duration};
use support::frame;

fn mount(ui: &mut Runtime, field: TextField) -> WidgetId {
    ui.set_viewport(Viewport::new(320.0, 160.0, 1.0).unwrap())
        .unwrap();
    let id = ui.mount_tree(Element::new("field", field)).unwrap();
    ui.focus(&id).unwrap();
    frame(ui);
    id
}
fn details(ui: &Runtime, key: &str) -> ComponentDetails {
    ui.inspect()
        .components
        .into_iter()
        .find(|c| c.key == key)
        .unwrap()
        .details
}
fn key(ui: &mut Runtime, code: KeyCode) {
    support::key(ui, code);
    frame(ui);
}
fn chord(ui: &mut Runtime, modifier: KeyCode, code: KeyCode) {
    ui.key_input(modifier, KeyAction::Press).unwrap();
    support::key(ui, code);
    ui.key_input(modifier, KeyAction::Release).unwrap();
    frame(ui);
}
fn pointer(ui: &mut Runtime, x: f32, y: f32, kind: PointerKind) {
    ui.pointer_input(PointerEvent::new(Point::new(x, y).unwrap(), kind))
        .unwrap();
}
fn click(ui: &mut Runtime, x: f32, y: f32, time: u64) {
    ui.flush(Duration::from_millis(time)).unwrap();
    pointer(ui, x, y, PointerKind::Press(PointerButton::Primary));
    pointer(ui, x, y, PointerKind::Release(PointerButton::Primary));
    frame(ui);
}
#[derive(Clone, Default)]
struct MemoryClipboard(Rc<RefCell<String>>);
impl Clipboard for MemoryClipboard {
    fn read_text(&mut self) -> Result<String, ClipboardError> {
        Ok(self.0.borrow().clone())
    }
    fn write_text(&mut self, text: &str) -> Result<(), ClipboardError> {
        *self.0.borrow_mut() = text.to_owned();
        Ok(())
    }
}

#[test]
fn concealed_text_never_leaves_the_field() {
    let mut ui = Runtime::new();
    let secret = ui.signal(String::new());
    let field = mount(&mut ui, TextField::new(secret.clone()).concealed(true));
    let clipboard = MemoryClipboard::default();
    *clipboard.0.borrow_mut() = "unchanged".into();
    ui.set_clipboard(clipboard.clone()).unwrap();
    ui.set_debug_mode(DebugMode::Inspect);
    // A combining sequence is one grapheme and one dot.
    ui.text_input("pa\u{0301}ss word").unwrap();
    // A key press reaches the field as a key event before its text.
    key(&mut ui, KeyCode::KeyX);
    ui.text_input("x").unwrap();
    frame(&mut ui);
    assert_eq!(secret.get().unwrap(), "pa\u{0301}ss wordx");
    let d = details(&ui, "field");
    assert!(!d.properties.contains_key("text"));
    assert!(!d.properties.contains_key("selection"));
    assert_eq!(d.properties["length"], InspectValue::Number(10.0));
    assert_eq!(d.properties["concealed"], InspectValue::Bool(true));
    let semantics = d.semantics.unwrap();
    assert!(semantics.protected);
    assert_eq!(semantics.value, None);
    // No key typed into it is recorded, at the field or while bubbling.
    let events = ui.inspect().events;
    assert!(
        events
            .iter()
            .all(|e| !matches!(e.kind, EventKind::KeyInput(_))),
        "{events:?}"
    );
    assert!(
        events
            .iter()
            .any(|e| matches!(e.kind, EventKind::TextInput { .. }))
    );
    // Copy and Cut are refused by shortcut and by command; the text stays.
    let primary = KeyCode::primary();
    chord(&mut ui, primary, KeyCode::KeyA);
    chord(&mut ui, primary, KeyCode::KeyC);
    chord(&mut ui, primary, KeyCode::KeyX);
    assert_eq!(clipboard.0.borrow().as_str(), "unchanged");
    assert_eq!(secret.get().unwrap(), "pa\u{0301}ss wordx");
    for command in [
        TextCommand::Copy,
        TextCommand::Cut,
        TextCommand::PasteBefore,
        TextCommand::PasteAfter,
    ] {
        assert_eq!(
            ui.text_command(&field, command).unwrap(),
            TextCommandOutcome::Unavailable,
            "{command:?}"
        );
    }
    // Paste still works, replacing the selection.
    assert_eq!(
        ui.text_command(&field, TextCommand::Paste).unwrap(),
        TextCommandOutcome::Handled
    );
    frame(&mut ui);
    assert_eq!(secret.get().unwrap(), "unchanged");
    // No IME is requested, so no candidate window can show the text.
    assert!(ui.text_input_state().is_none());
    ui.text_preedit(&TextPreedit::new("composed", None).unwrap())
        .unwrap();
    frame(&mut ui);
    assert_eq!(
        details(&ui, "field").properties["length"],
        InspectValue::Number(9.0)
    );
    assert_eq!(ui.focused(), Some(field));
}

#[test]
fn revealed_text_stays_uncopyable_and_undo_spans_a_reveal() {
    let mut ui = Runtime::new();
    let secret = ui.signal("abc".to_owned());
    let field = mount(
        &mut ui,
        TextField::new(secret.clone())
            .concealed(true)
            .revealable(true),
    );
    let clipboard = MemoryClipboard::default();
    *clipboard.0.borrow_mut() = "unchanged".into();
    ui.set_clipboard(clipboard.clone()).unwrap();
    key(&mut ui, KeyCode::End);
    ui.text_input("d").unwrap();
    frame(&mut ui);
    chord(&mut ui, KeyCode::AltLeft, KeyCode::F8);
    assert_eq!(
        details(&ui, "field").properties["revealed"],
        InspectValue::Bool(true)
    );
    // Revealed, the text still never reaches the clipboard.
    let primary = KeyCode::primary();
    chord(&mut ui, primary, KeyCode::KeyA);
    chord(&mut ui, primary, KeyCode::KeyC);
    chord(&mut ui, primary, KeyCode::KeyX);
    assert_eq!(clipboard.0.borrow().as_str(), "unchanged");
    assert_eq!(secret.get().unwrap(), "abcd");
    for command in [TextCommand::Copy, TextCommand::Cut] {
        assert_eq!(
            ui.text_command(&field, command).unwrap(),
            TextCommandOutcome::Unavailable
        );
    }
    // Undo reaches the typing made before the reveal.
    key(&mut ui, KeyCode::End);
    chord(&mut ui, primary, KeyCode::KeyZ);
    assert_eq!(secret.get().unwrap(), "abc");
}

#[test]
fn concealed_text_paints_the_same_dots_and_caret_whatever_the_glyphs() {
    let mut ui = Runtime::new();
    let secret = ui.signal("iiii".to_owned());
    mount(&mut ui, TextField::new(secret.clone()).concealed(true));
    let paint = |ui: &mut Runtime| {
        key(ui, KeyCode::End);
        ui.flush(Duration::ZERO).unwrap();
        ui.paint().unwrap().commands().to_vec()
    };
    let narrow = paint(&mut ui);
    secret.set("WWWW".into()).unwrap();
    frame(&mut ui);
    let wide = paint(&mut ui);
    // Dots and the caret after them depend on the count, not the glyphs.
    assert_eq!(narrow, wide);

    let mut ui = Runtime::new();
    let plain = ui.signal("iiii".to_owned());
    mount(&mut ui, TextField::new(plain.clone()));
    let narrow = paint(&mut ui);
    plain.set("WWWW".into()).unwrap();
    frame(&mut ui);
    assert_ne!(narrow, paint(&mut ui), "plain text paints its glyphs");
}

#[test]
fn a_modal_taking_focus_conceals_revealed_text() {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(400.0, 300.0, 1.0).unwrap())
        .unwrap();
    let secret = ui.signal("abc".to_owned());
    let dialog = DialogController::new(&ui);
    let opener = dialog.clone();
    let root = ui
        .mount_tree(Element::new(
            "root",
            Dialog::new(
                dialog,
                Element::new(
                    "body",
                    Column::new([Element::new(
                        "field",
                        TextField::new(secret).concealed(true).revealable(true),
                    )]),
                ),
                Element::new("content", Button::new("OK", |_| Ok(()))),
            ),
        ))
        .unwrap();
    let field = support::widget(&ui, &root, "field");
    ui.focus(&field).unwrap();
    frame(&mut ui);
    chord(&mut ui, KeyCode::AltLeft, KeyCode::F8);
    assert_eq!(
        details(&ui, "field").properties["revealed"],
        InspectValue::Bool(true)
    );
    opener.open().unwrap();
    frame(&mut ui);
    assert_ne!(ui.focused(), Some(field));
    assert_eq!(
        details(&ui, "field").properties["revealed"],
        InspectValue::Bool(false)
    );
}

#[test]
fn a_refusal_message_never_exposes_concealed_text() {
    let mut ui = Runtime::new();
    let secret = ui.signal(String::new());
    mount(
        &mut ui,
        TextField::new(secret)
            .concealed(true)
            .normalize(|text| Err(InputRejection::new(format!("rejected {text}")))),
    );
    ui.set_debug_mode(DebugMode::Inspect);
    ui.text_input("hunter2").unwrap();
    frame(&mut ui);
    key(&mut ui, KeyCode::Enter);
    let snapshot = format!("{:?}", ui.inspect());
    assert!(!snapshot.contains("hunter2"), "{snapshot}");
    assert_eq!(
        details(&ui, "field").properties["validation_refused"],
        InspectValue::Bool(true)
    );
}

#[test]
fn enter_on_an_untouched_required_field_checks_its_minimum() {
    let mut ui = Runtime::new();
    let name = ui.signal(String::new());
    let submitted = Rc::new(RefCell::new(0));
    let count = submitted.clone();
    let refusals = Rc::new(RefCell::new(Vec::new()));
    let seen = refusals.clone();
    mount(
        &mut ui,
        TextField::new(name)
            .required(true)
            .min_length(Some(8))
            .on_rejection(move |_, rejection| {
                seen.borrow_mut().push(rejection.is_some());
                Ok(())
            })
            .on_submit(move |_, _| {
                *count.borrow_mut() += 1;
                Ok(())
            }),
    );
    key(&mut ui, KeyCode::Enter);
    assert_eq!(*submitted.borrow(), 0);
    assert_eq!(*refusals.borrow(), [true]);
    assert!(
        details(&ui, "field")
            .properties
            .contains_key("validation_error")
    );
}

#[test]
fn concealed_editing_moves_over_graphemes_and_hides_word_boundaries() {
    let mut ui = Runtime::new();
    let secret = ui.signal("ab cd".to_owned());
    mount(&mut ui, TextField::new(secret.clone()).concealed(true));
    // The caret starts at the end of the dots; one Left steps over one
    // grapheme and Backspace removes the grapheme before the caret.
    key(&mut ui, KeyCode::End);
    key(&mut ui, KeyCode::ArrowLeft);
    key(&mut ui, KeyCode::Backspace);
    assert_eq!(secret.get().unwrap(), "ab d");
    // A word deletion takes everything before the caret, not one word.
    let word = if cfg!(target_os = "macos") {
        KeyCode::AltLeft
    } else {
        KeyCode::ControlLeft
    };
    chord(&mut ui, word, KeyCode::Backspace);
    assert_eq!(secret.get().unwrap(), "d");
    // Typing inserts at the caret in the real text.
    ui.text_input("x\u{1F469}\u{200D}\u{1F4BB}").unwrap();
    frame(&mut ui);
    assert_eq!(secret.get().unwrap(), "x\u{1F469}\u{200D}\u{1F4BB}d");
    assert_eq!(
        details(&ui, "field").properties["length"],
        InspectValue::Number(3.0)
    );
    // Undo restores the text before the typing.
    chord(&mut ui, KeyCode::primary(), KeyCode::KeyZ);
    assert_eq!(secret.get().unwrap(), "d");
}

#[test]
fn a_double_click_selects_all_concealed_text() {
    let mut ui = Runtime::new();
    let secret = ui.signal("two words".to_owned());
    mount(&mut ui, TextField::new(secret.clone()).concealed(true));
    click(&mut ui, 20.0, 15.0, 1000);
    click(&mut ui, 20.0, 15.0, 1100);
    ui.text_input("z").unwrap();
    frame(&mut ui);
    assert_eq!(secret.get().unwrap(), "z");
}

#[test]
fn the_reveal_button_toggles_and_resets_on_focus_loss_hiding_and_redescription() {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(320.0, 160.0, 1.0).unwrap())
        .unwrap();
    let secret = ui.signal("abc".to_owned());
    let root = ui
        .mount_tree(Element::new(
            "root",
            Column::new([
                Element::new(
                    "field",
                    TextField::new(secret.clone())
                        .concealed(true)
                        .revealable(true),
                ),
                Element::new("other", Button::new("Other", |_| Ok(()))),
            ]),
        ))
        .unwrap();
    let field = support::widget(&ui, &root, "field");
    let other = support::widget(&ui, &root, "other");
    ui.focus(&field).unwrap();
    frame(&mut ui);
    let revealed = |ui: &Runtime| details(ui, "field").properties["revealed"].clone();
    assert_eq!(revealed(&ui), InspectValue::Bool(false));
    // Alt+F8 toggles in the focused field.
    chord(&mut ui, KeyCode::AltLeft, KeyCode::F8);
    assert_eq!(revealed(&ui), InspectValue::Bool(true));
    // Revealed text is still protected from inspection and copying.
    assert!(!details(&ui, "field").properties.contains_key("text"));
    assert!(ui.text_input_state().is_none());
    // Typing while revealed edits at the caret, which moves over the
    // revealed text as before.
    key(&mut ui, KeyCode::End);
    ui.text_input("d").unwrap();
    frame(&mut ui);
    assert_eq!(secret.get().unwrap(), "abcd");
    // Focus loss conceals again.
    ui.focus(&other).unwrap();
    frame(&mut ui);
    assert_eq!(revealed(&ui), InspectValue::Bool(false));
    // The button at the end of the field toggles by pointer, keeping focus.
    ui.focus(&field).unwrap();
    frame(&mut ui);
    let bounds = ui.bounds(&field).unwrap().unwrap();
    let [x, y, w, h] = bounds.values();
    click(&mut ui, x + w - 20.0, y + h / 2.0, 2000);
    assert_eq!(revealed(&ui), InspectValue::Bool(true));
    assert_eq!(ui.focused(), Some(field.clone()));
    click(&mut ui, x + w - 20.0, y + h / 2.0, 3000);
    assert_eq!(revealed(&ui), InspectValue::Bool(false));
    // Hiding conceals again.
    chord(&mut ui, KeyCode::AltLeft, KeyCode::F8);
    assert_eq!(revealed(&ui), InspectValue::Bool(true));
    ui.set_visible(&field, false).unwrap();
    frame(&mut ui);
    ui.set_visible(&field, true).unwrap();
    frame(&mut ui);
    assert_eq!(revealed(&ui), InspectValue::Bool(false));
    // Withdrawing the button in a new description conceals again.
    ui.focus(&field).unwrap();
    chord(&mut ui, KeyCode::AltLeft, KeyCode::F8);
    assert_eq!(revealed(&ui), InspectValue::Bool(true));
    ui.reconcile_children(
        &root,
        vec![
            Element::new("field", TextField::new(secret.clone()).concealed(true)),
            Element::new("other", Button::new("Other", |_| Ok(()))),
        ],
    )
    .unwrap();
    frame(&mut ui);
    let d = details(&ui, "field");
    assert!(!d.properties.contains_key("revealed"));
    assert!(d.semantics.unwrap().protected);
    // Alt+F8 does nothing without the button.
    chord(&mut ui, KeyCode::AltLeft, KeyCode::F8);
    assert!(!details(&ui, "field").properties.contains_key("revealed"));
}

#[test]
fn placeholder_is_never_the_value() {
    let mut ui = Runtime::new();
    let name = ui.signal(String::new());
    mount(
        &mut ui,
        TextField::new(name.clone()).placeholder("Project name"),
    );
    let d = details(&ui, "field");
    assert_eq!(
        d.properties["placeholder"],
        InspectValue::Text("Project name".into())
    );
    assert_eq!(d.properties["text"], InspectValue::Text(String::new()));
    assert_eq!(
        d.semantics.unwrap().value,
        Some(SemanticValue::Text(String::new()))
    );
    assert_eq!(name.get().unwrap(), "");
    ui.text_input("A").unwrap();
    frame(&mut ui);
    assert_eq!(name.get().unwrap(), "A");
    // A text area takes a placeholder too.
    let mut ui = Runtime::new();
    let notes = ui.signal(String::new());
    ui.mount_tree(Element::new(
        "notes",
        TextArea::new(notes).placeholder("Notes"),
    ))
    .unwrap();
    frame(&mut ui);
    assert_eq!(
        details(&ui, "notes").properties["placeholder"],
        InspectValue::Text("Notes".into())
    );
}

#[test]
fn length_limits_count_graphemes_at_save_and_spare_an_empty_optional_field() {
    let mut ui = Runtime::new();
    let value = ui.signal(String::new());
    let reports = Rc::new(RefCell::new(Vec::new()));
    let seen = reports.clone();
    mount(
        &mut ui,
        TextField::new(value.clone())
            .min_length(Some(3))
            .max_length(Some(4))
            .counter(true)
            .normalize(|text| Ok(text.trim().to_owned()))
            .on_rejection(move |_, rejection| {
                seen.borrow_mut()
                    .push(rejection.map(|r| r.message().to_owned()));
                Ok(())
            }),
    );
    // A limited field holds a draft.
    ui.text_input(" ab ").unwrap();
    frame(&mut ui);
    assert_eq!(value.get().unwrap(), "");
    assert_eq!(
        details(&ui, "field").properties["counter"],
        InspectValue::Text("4/4".into())
    );
    // Normalized to "ab", two graphemes, below the minimum.
    key(&mut ui, KeyCode::Enter);
    assert_eq!(value.get().unwrap(), "");
    assert_eq!(
        reports.borrow().last().cloned().flatten().as_deref(),
        Some("Character count must be at least 3")
    );
    let d = details(&ui, "field");
    assert!(d.semantics.unwrap().invalid);
    // Graphemes, not bytes: "ab" plus one combining sequence is three.
    ui.text_input("e\u{0301}").unwrap();
    key(&mut ui, KeyCode::Enter);
    assert_eq!(value.get().unwrap(), "ab e\u{0301}".trim());
    // Above the maximum is refused too.
    ui.text_input("xyz").unwrap();
    key(&mut ui, KeyCode::Enter);
    assert_eq!(
        reports.borrow().last().cloned().flatten().as_deref(),
        Some("Character count must be at most 4")
    );
    // An empty optional field may be saved empty.
    let len = value.get().unwrap().len() + 3;
    for _ in 0..len {
        key(&mut ui, KeyCode::Backspace);
    }
    key(&mut ui, KeyCode::Enter);
    assert_eq!(value.get().unwrap(), "");
    // A required one may not.
    let mut ui = Runtime::new();
    let value = ui.signal("abc".to_owned());
    mount(
        &mut ui,
        TextField::new(value.clone())
            .required(true)
            .min_length(Some(3)),
    );
    key(&mut ui, KeyCode::End);
    for _ in 0..3 {
        key(&mut ui, KeyCode::Backspace);
    }
    key(&mut ui, KeyCode::Enter);
    assert_eq!(value.get().unwrap(), "abc");
    let semantics = details(&ui, "field").semantics.unwrap();
    assert!(semantics.required && semantics.invalid);
}

#[test]
fn application_writes_are_not_length_checked() {
    let mut ui = Runtime::new();
    let value = ui.signal(String::new());
    mount(&mut ui, TextField::new(value.clone()).max_length(Some(2)));
    value.set("far too long".to_owned()).unwrap();
    frame(&mut ui);
    let d = details(&ui, "field");
    assert_eq!(
        d.properties["text"],
        InspectValue::Text("far too long".into())
    );
    assert!(!d.semantics.unwrap().invalid);
}

#[test]
fn new_options_apply_on_redescription() {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(320.0, 160.0, 1.0).unwrap())
        .unwrap();
    let value = ui.signal("abcdef".to_owned());
    let result = check_redescription(
        &mut ui,
        Element::new("c", TextField::new(value.clone())),
        Element::new(
            "c",
            TextField::new(value.clone())
                .concealed(true)
                .revealable(true)
                .placeholder("Secret")
                .required(true)
                .max_length(Some(3))
                .counter(true),
        ),
        Element::new(
            "c",
            TextField::new(value.clone())
                .concealed(true)
                .revealable(true)
                .placeholder("Secret")
                .required(true)
                .max_length(Some(3))
                .counter(true),
        ),
    );
    if let Err(error) = result {
        panic!("{error}: {error:?}");
    }
    let mut ui = Runtime::new();
    let notes = ui.signal("x".to_owned());
    let result = check_redescription(
        &mut ui,
        Element::new("c", TextArea::new(notes.clone())),
        Element::new(
            "c",
            TextArea::new(notes.clone())
                .placeholder("Notes")
                .counter(true),
        ),
        Element::new(
            "c",
            TextArea::new(notes.clone())
                .placeholder("Notes")
                .counter(true),
        ),
    );
    if let Err(error) = result {
        panic!("{error}: {error:?}");
    }
}

#[test]
fn removing_concealment_in_a_new_description_shows_the_text() {
    let mut ui = Runtime::new();
    let value = ui.signal("abc".to_owned());
    let result = redescribe(
        &mut ui,
        Element::new("c", TextField::new(value.clone()).concealed(true)),
        Element::new("c", TextField::new(value.clone())),
    )
    .unwrap();
    let _ = result;
    frame(&mut ui);
    let d = details(&ui, "c");
    assert_eq!(d.properties["text"], InspectValue::Text("abc".into()));
    assert!(!d.semantics.unwrap().protected);
}

#[test]
fn a_pending_status_shows_a_spinner_that_stops_when_hidden() {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(320.0, 200.0, 1.0).unwrap())
        .unwrap();
    let form = FormFields::new(&ui);
    let handle = form.field("name").unwrap();
    let name = ui.signal(String::new());
    let root = ui
        .mount_tree(Element::new(
            "name-field",
            Field::new(Element::new("name", TextField::new(name).required(true)))
                .label("Name")
                .required(true)
                .form(&handle),
        ))
        .unwrap();
    frame(&mut ui);
    assert_eq!(ui.next_deadline(), None);
    handle
        .set(FieldStatus::pending("Checking the name"))
        .unwrap();
    assert_eq!(form.summary().get().unwrap().pending(), 1);
    assert_eq!(form.summary().get().unwrap().errors(), 0);
    frame(&mut ui);
    let field = details(&ui, "name-field");
    assert_eq!(
        field.properties["status"],
        InspectValue::Text("pending".into())
    );
    assert_eq!(
        field.properties["message"],
        InspectValue::Text("Checking the name".into())
    );
    assert_eq!(field.properties["outlined"], InspectValue::Bool(false));
    assert_eq!(field.properties["required"], InspectValue::Bool(true));
    let semantics = field.semantics.unwrap();
    assert!(semantics.required && !semantics.invalid);
    assert!(ui.next_deadline().is_some(), "the spinner steps");
    // Hidden, it schedules nothing.
    ui.set_visible(&root, false).unwrap();
    frame(&mut ui);
    assert_eq!(ui.next_deadline(), None);
    ui.set_visible(&root, true).unwrap();
    frame(&mut ui);
    assert!(ui.next_deadline().is_some());
    // The verdict ends the animation.
    handle.set(FieldStatus::error("Taken")).unwrap();
    frame(&mut ui);
    assert_eq!(ui.next_deadline(), None);
    assert_eq!(
        details(&ui, "name-field").properties["outlined"],
        InspectValue::Bool(true)
    );
}

#[test]
fn a_press_on_the_reveal_button_breaks_a_click_series() {
    for reveal in [false, true] {
        let mut ui = Runtime::new();
        let secret = ui.signal("two words".to_owned());
        let field = mount(
            &mut ui,
            TextField::new(secret.clone())
                .concealed(true)
                .revealable(true),
        );
        let [x, y, w, h] = ui.bounds(&field).unwrap().unwrap().values();
        click(&mut ui, 20.0, 15.0, 1000);
        if reveal {
            click(&mut ui, x + w - 20.0, y + h / 2.0, 1100);
        }
        click(&mut ui, 20.0, 15.0, 1200);
        ui.text_input("z").unwrap();
        frame(&mut ui);
        if reveal {
            // The reveal press ended the series: a single click, a caret.
            assert_ne!(secret.get().unwrap(), "z");
        } else {
            // Without it the two presses select all concealed text.
            assert_eq!(secret.get().unwrap(), "z");
        }
    }
}
