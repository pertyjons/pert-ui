//! The draft policy (contracts.md, "Draft policy"): what save attempts,
//! refusals, application writes, revocation, hiding and text commands do to
//! the drafts of the built-in editors, and how `on_modified` reports them.
use crate::support;
use pert_ui::testing::HostIntegration;
use pert_ui::*;
use std::{cell::RefCell, rc::Rc, time::Duration};
use support::{frame, key_path};

type Reports = Rc<RefCell<Vec<bool>>>;
type Log = Rc<RefCell<Vec<(EditorTrigger, EditorOutcome)>>>;

/// A modified log and the callback that fills it.
fn reports() -> (
    Reports,
    impl FnMut(&mut EventContext, bool) -> Result<(), RuntimeError> + 'static,
) {
    let log = Reports::default();
    let sink = log.clone();
    (log, move |_, modified| {
        sink.borrow_mut().push(modified);
        Ok(())
    })
}

fn settle(ui: &mut Runtime) {
    for _ in 0..3 {
        frame(ui);
    }
}

/// Mounts `field` keyed "field" above a button keyed "other", and returns
/// the root and the field.
fn mount(ui: &mut Runtime, field: Element) -> (WidgetId, WidgetId) {
    ui.set_viewport(Viewport::new(400.0, 200.0, 1.0).unwrap())
        .unwrap();
    let root = ui
        .mount_tree(Element::new(
            "root",
            Column::new([
                field,
                Element::new("other", Button::new("Other", |_| Ok(()))),
            ]),
        ))
        .unwrap();
    settle(ui);
    let field = ui.find_descendant(&root, "field").unwrap().unwrap();
    (root, field)
}

/// A property of the component keyed `key`.
fn property(ui: &Runtime, key: &str, name: &str) -> Option<InspectValue> {
    ui.inspect()
        .components
        .into_iter()
        .find(|c| c.key == key)
        .and_then(|c| c.details.properties.get(name).cloned())
}

/// A text property of the component `id`.
fn text_of(ui: &Runtime, id: &WidgetId, name: &str) -> String {
    let inspection = ui.inspection_id(id).unwrap();
    let component = ui
        .inspect()
        .components
        .into_iter()
        .find(|c| c.id == inspection)
        .unwrap();
    match component.details.properties.get(name) {
        Some(InspectValue::Text(text)) => text.clone(),
        other => panic!("{name} is not text: {other:?}"),
    }
}

/// Focuses `id`, selects its text and types `text` over it.
fn replace(ui: &mut Runtime, id: &WidgetId, text: &str) {
    ui.focus(id).unwrap();
    settle(ui);
    ui.text_command(id, TextCommand::SelectAll).unwrap();
    ui.text_input(text).unwrap();
    settle(ui);
}

fn press(ui: &mut Runtime, code: KeyCode) {
    ui.key_input(code, KeyAction::Press).unwrap();
    ui.key_input(code, KeyAction::Release).unwrap();
    settle(ui);
}

/// The middle of `id`, in window coordinates.
fn center(ui: &Runtime, id: &WidgetId) -> Point {
    let [x, y, w, h] = ui.bounds(id).unwrap().unwrap().values();
    Point::new(x + w / 2.0, y + h / 2.0).unwrap()
}

#[derive(Clone, Default)]
struct Memory(Rc<RefCell<String>>);
impl Clipboard for Memory {
    fn read_text(&mut self) -> Result<String, ClipboardError> {
        Ok(self.0.borrow().clone())
    }
    fn write_text(&mut self, text: &str) -> Result<(), ClipboardError> {
        text.clone_into(&mut self.0.borrow_mut());
        Ok(())
    }
}

/// New-behavior test 1. Named assertion: the revoked field still shows `42`
/// and is `dirty` while its binding holds 7 and focus is gone. Mutation
/// checked: restoring `abandon` and `reset_draft` in the update's revocation
/// branch (bound) and in `adopt` (fixed) makes it show 7.
#[test]
fn a_disabled_number_field_keeps_its_draft() {
    #[derive(Clone, Copy, Debug)]
    enum Kind {
        Integer,
        Decimal,
    }
    for kind in [Kind::Integer, Kind::Decimal] {
        for fixed in [false, true] {
            let mut ui = Runtime::new();
            let enabled = ui.signal(true);
            let integer = ui.signal(7_i64);
            let decimal = ui.signal(7.0_f64);
            let describe = |disabled: bool| -> Vec<Element> {
                let field = match kind {
                    Kind::Integer => {
                        let field = IntegerInput::new(integer.clone()).enabled(&enabled);
                        if disabled { field.disabled() } else { field }.key("field")
                    }
                    Kind::Decimal => {
                        let field = DecimalInput::new(decimal.clone()).enabled(&enabled);
                        if disabled { field.disabled() } else { field }.key("field")
                    }
                };
                vec![field, Button::new("Other", |_| Ok(())).key("other")]
            };
            ui.set_viewport(Viewport::new(400.0, 200.0, 1.0).unwrap())
                .unwrap();
            let root = ui
                .mount_tree(Column::new(describe(false)).key("root"))
                .unwrap();
            settle(&mut ui);
            let field = ui.find_descendant(&root, "field").unwrap().unwrap();
            replace(&mut ui, &field, "42");
            if fixed {
                ui.reconcile_children(&root, describe(true)).unwrap();
            } else {
                enabled.set(false).unwrap();
            }
            settle(&mut ui);
            let context = format!("{kind:?}, fixed {fixed}");
            assert_eq!(text_of(&ui, &field, "text"), "42", "{context}");
            assert_eq!(
                property(&ui, "field", "dirty"),
                Some(InspectValue::Bool(true)),
                "{context}"
            );
            assert_ne!(ui.focused(), Some(field.clone()), "{context}");
            assert_eq!(integer.get().unwrap(), 7, "{context}");
            assert_eq!(decimal.get().unwrap(), 7.0, "{context}");
            // Enabled again, a later save attempt saves the kept draft.
            if fixed {
                ui.reconcile_children(&root, describe(false)).unwrap();
            } else {
                enabled.set(true).unwrap();
            }
            settle(&mut ui);
            ui.focus(&field).unwrap();
            settle(&mut ui);
            ui.clear_focus();
            settle(&mut ui);
            match kind {
                Kind::Integer => assert_eq!(integer.get().unwrap(), 42, "{context}"),
                Kind::Decimal => assert_eq!(decimal.get().unwrap(), 42.0, "{context}"),
            }
        }
    }
}

/// New-behavior test 2. Named assertion: after a pointer move over the field
/// and a bound revocation, the unfocused refused draft `4x` is still shown
/// with its outline. Mutation checked: restoring `cancel()` in
/// `cancel_input`'s unfocused branch makes it show 7.
#[test]
fn an_unfocused_kept_number_draft_survives_revocation_and_hiding_discards_it() {
    let mut ui = Runtime::new();
    let value = ui.signal(7_i64);
    let enabled = ui.signal(true);
    let (_, field) = mount(
        &mut ui,
        IntegerInput::new(value.clone())
            .enabled(&enabled)
            .key("field"),
    );
    replace(&mut ui, &field, "4x");
    // Focus leaves: the save attempt is refused and the draft stays.
    ui.clear_focus();
    settle(&mut ui);
    assert_eq!(text_of(&ui, &field, "text"), "4x");
    // A pointer move over the field clears its cancellation marker, so the
    // revocation reaches it as cancelled input.
    let point = center(&ui, &field);
    ui.pointer_input(PointerEvent::new(point, PointerKind::Move))
        .unwrap();
    settle(&mut ui);
    enabled.set(false).unwrap();
    settle(&mut ui);
    assert_eq!(text_of(&ui, &field, "text"), "4x");
    assert_eq!(
        property(&ui, "field", "valid"),
        Some(InspectValue::Bool(false))
    );
    assert_eq!(value.get().unwrap(), 7);
    // Hiding discards it (rule 7).
    ui.set_visible(&field, false).unwrap();
    settle(&mut ui);
    ui.set_visible(&field, true).unwrap();
    settle(&mut ui);
    assert_eq!(text_of(&ui, &field, "text"), "7");
    assert_eq!(
        property(&ui, "field", "dirty"),
        Some(InspectValue::Bool(false))
    );
}

/// The child editor of an inline row or label, which holds its text mode.
fn child_editor(ui: &Runtime, row: &WidgetId) -> WidgetId {
    key_path(ui, row, &["editor"]).expect("the row's editor")
}

/// New-behavior test 8. Named assertion: with editing revoked in the flush
/// that observed it, typing `9` leaves the draft `42`, a composition started
/// before is gone and Enter writes nothing while text mode stays. Mutations
/// checked: without the `set_read_only` call in `lock`, and with `lock`
/// moved back before `read_permissions`, typing changes the draft.
#[test]
fn an_open_inline_text_mode_follows_editability() {
    for parameter in [false, true] {
        let mut ui = Runtime::new();
        let editable = ui.signal(true);
        let value = ui.signal(8_i64);
        let (count, host) = ui
            .parameter(
                ParameterId::new(1, 1, "count").unwrap(),
                ParameterSpec::stepped("Count", 0, 1000, 8).unwrap(),
            )
            .unwrap();
        let row = if parameter {
            InlineInteger::for_parameter(count.clone())
        } else {
            InlineInteger::new(value.clone())
        };
        let (_, field) = mount(&mut ui, row.editable(&editable).key("field"));
        let written = || {
            if parameter {
                count.snapshot().get().unwrap().base
            } else {
                value.get().unwrap()
            }
        };
        ui.focus(&field).unwrap();
        settle(&mut ui);
        press(&mut ui, KeyCode::F2);
        ui.text_input("42").unwrap();
        settle(&mut ui);
        let editor = child_editor(&ui, &field);
        ui.text_preedit(&TextPreedit::new("x", None).unwrap())
            .unwrap();
        settle(&mut ui);
        assert_eq!(
            property(&ui, "editor", "composing"),
            Some(InspectValue::Bool(true))
        );
        editable.set(false).unwrap();
        settle(&mut ui);
        let context = format!("parameter {parameter}");
        assert_eq!(
            property(&ui, "editor", "composing"),
            Some(InspectValue::Bool(false)),
            "{context}"
        );
        ui.text_input("9").unwrap();
        settle(&mut ui);
        assert_eq!(text_of(&ui, &editor, "text"), "42", "{context}");
        press(&mut ui, KeyCode::Enter);
        assert_eq!(written(), 8, "{context}");
        assert_eq!(
            property(&ui, "field", "text_editing"),
            Some(InspectValue::Bool(true)),
            "{context}"
        );
        // Allowed again: the session is editable with its draft and caret.
        editable.set(true).unwrap();
        settle(&mut ui);
        ui.text_input("9").unwrap();
        settle(&mut ui);
        assert_eq!(text_of(&ui, &editor, "text"), "429", "{context}");
        press(&mut ui, KeyCode::Enter);
        assert_eq!(written(), 429, "{context}");
        drop(host);
    }
}

/// A custom owner of a `DraftEditor` that does not answer text commands.
struct Owner {
    draft: DraftEditorController,
    editor: Option<WidgetId>,
    children: Vec<Element>,
}
impl Component for Owner {
    fn take_children(&mut self) -> Vec<Element> {
        std::mem::take(&mut self.children)
    }
    fn focusable(&self) -> bool {
        !self.draft.is_active()
    }
    fn child_visible(&self, _: &str) -> bool {
        self.draft.is_active()
    }
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        self.editor = cx.children().first().map(|child| child.id().clone());
        Ok(())
    }
    fn keyboard_event(
        &mut self,
        cx: &mut EventContext,
        event: KeyEvent,
        _: &KeyboardState,
    ) -> Result<InputOutcome, RuntimeError> {
        if event.code == KeyCode::F2 && event.transition == KeyTransition::Pressed {
            self.draft.begin(cx, "120", None, DraftSelection::End)?;
            if let Some(editor) = &self.editor {
                cx.request_focus_for(editor)?;
            }
            return Ok(InputOutcome::Handled);
        }
        Ok(InputOutcome::Ignored)
    }
}

/// New-behavior test 12. Named assertion: `on_modified` reports `false` for
/// each editor's draft the command discards, the binding unchanged.
/// Mutation checked: answering the command only while the editor is enabled
/// leaves the disabled number field's draft (`Unavailable`).
#[test]
fn discard_draft_discards_without_writing() {
    // A disabled number field.
    let mut ui = Runtime::new();
    let value = ui.signal(7_i64);
    let enabled = ui.signal(true);
    let (modified, report) = reports();
    let (_, field) = mount(
        &mut ui,
        IntegerInput::new(value.clone())
            .enabled(&enabled)
            .on_modified(report)
            .key("field"),
    );
    replace(&mut ui, &field, "42");
    enabled.set(false).unwrap();
    settle(&mut ui);
    let outcome = ui.text_command(&field, TextCommand::DiscardDraft).unwrap();
    settle(&mut ui);
    assert_eq!(*modified.borrow(), [true, false]);
    assert_eq!(outcome, TextCommandOutcome::Handled);
    assert_eq!(value.get().unwrap(), 7);
    assert_eq!(text_of(&ui, &field, "text"), "7");
    assert_eq!(
        ui.text_command(&field, TextCommand::DiscardDraft).unwrap(),
        TextCommandOutcome::Unavailable,
        "no draft left"
    );

    // A read-only text field in draft mode.
    let mut ui = Runtime::new();
    let name = ui.signal("Bass".to_owned());
    let editable = ui.signal(true);
    let (modified, report) = reports();
    let (_, field) = mount(
        &mut ui,
        TextField::new(name.clone())
            .max_length(Some(20))
            .editable(&editable)
            .on_modified(report)
            .key("field"),
    );
    replace(&mut ui, &field, "Drums");
    editable.set(false).unwrap();
    settle(&mut ui);
    assert_eq!(
        ui.text_command(&field, TextCommand::DiscardDraft).unwrap(),
        TextCommandOutcome::Handled
    );
    settle(&mut ui);
    assert_eq!(name.get().unwrap(), "Bass");
    assert_eq!(text_of(&ui, &field, "text"), "Bass");
    assert_eq!(*modified.borrow(), [true, false]);

    // A focused search field with a pending query.
    let mut ui = Runtime::new();
    let query = ui.signal(String::new());
    let (_, field) = mount(
        &mut ui,
        SearchField::new(query.clone())
            .debounce(Duration::from_millis(300))
            .key("field"),
    );
    ui.focus(&field).unwrap();
    ui.text_input("abc").unwrap();
    settle(&mut ui);
    assert_eq!(
        property(&ui, "field", "pending"),
        Some(InspectValue::Bool(true))
    );
    assert_eq!(
        ui.text_command(&field, TextCommand::DiscardDraft).unwrap(),
        TextCommandOutcome::Handled
    );
    settle(&mut ui);
    assert_eq!(
        property(&ui, "field", "pending"),
        Some(InspectValue::Bool(false))
    );
    assert_eq!(text_of(&ui, &field, "text"), "");
    ui.flush(ui.now() + Duration::from_millis(400)).unwrap();
    assert_eq!(query.get().unwrap(), "", "nothing is written later");
    assert_eq!(
        ui.text_command(&field, TextCommand::DiscardDraft).unwrap(),
        TextCommandOutcome::Unavailable
    );

    // An editor without a draft.
    let mut ui = Runtime::new();
    let plain = ui.signal("plain".to_owned());
    let (_, field) = mount(&mut ui, TextField::new(plain).key("field"));
    assert_eq!(
        ui.text_command(&field, TextCommand::DiscardDraft).unwrap(),
        TextCommandOutcome::Unavailable
    );
}

/// New-behavior test 12, targeting. Named assertion: the command aimed at an
/// in-place owner answers `Handled` and ends the session, checked first for
/// a refused session that stayed open without focus (`focused false`).
/// Mutation checked: forwarding the command to the editor instead of
/// answering at the owner answers `NotAnEditor` there.
#[test]
fn discard_draft_is_answered_by_the_owning_control() {
    // InlineText, unfocused with a refused session and then focused.
    for focused in [false, true] {
        let mut ui = Runtime::new();
        let name = ui.signal("Bass".to_owned());
        let (modified, report) = reports();
        let (_, label) = mount(
            &mut ui,
            InlineText::new(name.clone())
                .normalize(|text: String| {
                    if text.trim().is_empty() {
                        Err(InputRejection::new("Enter a name"))
                    } else {
                        Ok(text)
                    }
                })
                .on_modified(report)
                .key("field"),
        );
        ui.text_command(&label, TextCommand::BeginEdit).unwrap();
        settle(&mut ui);
        ui.text_input(" ").unwrap();
        settle(&mut ui);
        if !focused {
            ui.clear_focus();
            settle(&mut ui);
            assert_eq!(
                property(&ui, "field", "editing"),
                Some(InspectValue::Bool(true)),
                "the refused session stays open"
            );
        }
        assert_eq!(
            ui.text_command(&label, TextCommand::DiscardDraft).unwrap(),
            TextCommandOutcome::Handled,
            "focused {focused}"
        );
        settle(&mut ui);
        assert_eq!(
            property(&ui, "field", "editing"),
            Some(InspectValue::Bool(false))
        );
        assert_eq!(name.get().unwrap(), "Bass");
        assert_eq!(*modified.borrow(), [true, false], "focused {focused}");
    }

    // An inline number row, unfocused with a refused draft and then focused.
    for focused in [false, true] {
        let mut ui = Runtime::new();
        let value = ui.signal(8_i64);
        let (modified, report) = reports();
        let (_, row) = mount(
            &mut ui,
            InlineInteger::new(value.clone())
                .on_modified(report)
                .key("field"),
        );
        ui.focus(&row).unwrap();
        settle(&mut ui);
        press(&mut ui, KeyCode::F2);
        ui.text_input("4x").unwrap();
        settle(&mut ui);
        if !focused {
            ui.clear_focus();
            settle(&mut ui);
            assert_eq!(
                property(&ui, "field", "text_editing"),
                Some(InspectValue::Bool(true)),
                "the refused text mode stays open"
            );
        }
        assert_eq!(
            ui.text_command(&row, TextCommand::DiscardDraft).unwrap(),
            TextCommandOutcome::Handled,
            "focused {focused}"
        );
        settle(&mut ui);
        assert_eq!(
            property(&ui, "field", "text_editing"),
            Some(InspectValue::Bool(false))
        );
        assert_eq!(value.get().unwrap(), 8);
        assert_eq!(*modified.borrow(), [true, false], "focused {focused}");
    }

    // Aimed at the focused DraftEditor of an inline row: nothing changes.
    let mut ui = Runtime::new();
    let value = ui.signal(8_i64);
    let (_, row) = mount(&mut ui, InlineInteger::new(value.clone()).key("field"));
    ui.focus(&row).unwrap();
    settle(&mut ui);
    press(&mut ui, KeyCode::F2);
    ui.text_input("42").unwrap();
    settle(&mut ui);
    let editor = child_editor(&ui, &row);
    assert_eq!(
        ui.text_command(&editor, TextCommand::DiscardDraft).unwrap(),
        TextCommandOutcome::Unavailable
    );
    assert_eq!(text_of(&ui, &editor, "text"), "42");
    assert_eq!(
        property(&ui, "field", "text_editing"),
        Some(InspectValue::Bool(true))
    );

    // A custom owner that does not answer: forwarded to its focused editor,
    // which answers `Unavailable`; the session stays.
    let mut ui = Runtime::new();
    let draft = DraftEditorController::new();
    let (_, owner) = mount(
        &mut ui,
        Element::new(
            "field",
            Owner {
                draft: draft.clone(),
                editor: None,
                children: vec![Element::new("editor", DraftEditor::new(draft.clone()))],
            },
        ),
    );
    ui.focus(&owner).unwrap();
    settle(&mut ui);
    press(&mut ui, KeyCode::F2);
    ui.text_input("8").unwrap();
    settle(&mut ui);
    assert_eq!(
        ui.text_command(&owner, TextCommand::DiscardDraft).unwrap(),
        TextCommandOutcome::Unavailable
    );
    assert!(draft.is_active());
    assert_eq!(draft.text(), "1208");
}

/// New-behavior test 13. Named assertion for each type: the recorded
/// sequence. Mutations checked: removing the type's delivery request leaves
/// the sequence empty; reporting a resting drag on the inline row adds a
/// report for it.
#[test]
fn on_modified_reports_the_added_types_drafts() {
    // IntegerInput, bound to a signal.
    let mut ui = Runtime::new();
    let memory = Memory::default();
    ui.set_clipboard(memory.clone()).unwrap();
    let value = ui.signal(7_i64);
    let enabled = ui.signal(true);
    let (modified, report) = reports();
    let (_, field) = mount(
        &mut ui,
        IntegerInput::new(value.clone())
            .enabled(&enabled)
            .on_modified(report)
            .key("field"),
    );
    replace(&mut ui, &field, "42");
    press(&mut ui, KeyCode::Enter); // saved
    replace(&mut ui, &field, "43");
    press(&mut ui, KeyCode::Escape); // discarded
    replace(&mut ui, &field, "44");
    value.set(9).unwrap(); // an application write wins
    settle(&mut ui);
    replace(&mut ui, &field, "45");
    enabled.set(false).unwrap(); // kept: no report
    settle(&mut ui);
    ui.set_visible(&field, false).unwrap(); // hiding discards
    settle(&mut ui);
    ui.set_visible(&field, true).unwrap();
    enabled.set(true).unwrap();
    settle(&mut ui);
    // An unfocused paste makes a draft too.
    ui.clear_focus();
    settle(&mut ui);
    "5".clone_into(&mut memory.0.borrow_mut());
    ui.text_command(&field, TextCommand::Paste).unwrap();
    settle(&mut ui);
    assert_eq!(value.get().unwrap(), 9);
    assert_eq!(
        *modified.borrow(),
        [true, false, true, false, true, false, true, false, true]
    );

    // IntegerInput bound to a parameter.
    let mut ui = Runtime::new();
    let (count, host) = ui
        .parameter(
            ParameterId::new(1, 1, "count").unwrap(),
            ParameterSpec::stepped("Count", 0, 96, 12)
                .unwrap()
                .with_step(std::num::NonZeroU64::new(4).unwrap())
                .unwrap(),
        )
        .unwrap();
    let hold = ui.signal(false);
    let (modified, report) = reports();
    ui.set_viewport(Viewport::new(400.0, 200.0, 1.0).unwrap())
        .unwrap();
    let root = ui
        .mount_tree(
            Column::new([
                IntegerInput::for_parameter(count.clone())
                    .on_modified(report)
                    .key("field"),
                Holder::new(count.clone(), hold.clone()).key("holder"),
                Button::new("Other", |_| Ok(())).key("other"),
            ])
            .key("root"),
        )
        .unwrap();
    settle(&mut ui);
    let field = ui.find_descendant(&root, "field").unwrap().unwrap();
    replace(&mut ui, &field, "13"); // off the step: refused, kept
    press(&mut ui, KeyCode::Enter);
    assert_eq!(text_of(&ui, &field, "text"), "13");
    assert_eq!(*modified.borrow(), [true]);
    // Another edit holds the parameter: the protocol refuses, the draft is
    // replaced.
    replace(&mut ui, &field, "24");
    hold.set(true).unwrap();
    settle(&mut ui);
    press(&mut ui, KeyCode::Enter);
    assert_eq!(text_of(&ui, &field, "text"), "12");
    assert_eq!(*modified.borrow(), [true, false]);
    hold.set(false).unwrap();
    settle(&mut ui);
    replace(&mut ui, &field, "24");
    press(&mut ui, KeyCode::Enter); // committed
    assert_eq!(count.snapshot().get().unwrap().base, 24);
    assert_eq!(*modified.borrow(), [true, false, true, false]);
    drop(host);

    // InlineText, bound and controlled.
    for controlled in [false, true] {
        let mut ui = Runtime::new();
        let name = ui.signal("Bass".to_owned());
        let enabled = ui.signal(true);
        let modified = Reports::default();
        let describe = |text: &str| {
            let label = if controlled {
                InlineText::controlled(text, |_, _| Ok(()))
            } else {
                InlineText::new(name.clone())
            };
            let sink = modified.clone();
            let label = label
                .normalize(|text: String| {
                    if text.trim().is_empty() {
                        Err(InputRejection::new("Enter a name"))
                    } else {
                        Ok(text)
                    }
                })
                .enabled(&enabled)
                .on_modified(move |_, modified| {
                    sink.borrow_mut().push(modified);
                    Ok(())
                });
            vec![
                label.key("field"),
                Button::new("Other", |_| Ok(())).key("other"),
            ]
        };
        ui.set_viewport(Viewport::new(400.0, 200.0, 1.0).unwrap())
            .unwrap();
        let root = ui
            .mount_tree(Column::new(describe("Bass")).key("root"))
            .unwrap();
        settle(&mut ui);
        let label = ui.find_descendant(&root, "field").unwrap().unwrap();
        let begin = |ui: &mut Runtime| {
            ui.text_command(&label, TextCommand::BeginEdit).unwrap();
            settle(ui);
        };
        let context = format!("controlled {controlled}");
        begin(&mut ui);
        assert!(modified.borrow().is_empty(), "beginning reports nothing");
        ui.text_input("Drums").unwrap();
        settle(&mut ui);
        press(&mut ui, KeyCode::Enter); // saved or proposed
        begin(&mut ui);
        ui.text_input(" ").unwrap();
        settle(&mut ui);
        ui.clear_focus(); // refused on focus loss: stays open, still true
        settle(&mut ui);
        assert_eq!(*modified.borrow(), [true, false, true], "{context}");
        // An application write, or changed controlled text, ends the session.
        if controlled {
            ui.reconcile_children(&root, describe("Keys")).unwrap();
        } else {
            name.set("Keys".to_owned()).unwrap();
        }
        settle(&mut ui);
        assert_eq!(*modified.borrow(), [true, false, true, false], "{context}");
        begin(&mut ui);
        ui.text_input("Lead").unwrap();
        settle(&mut ui);
        enabled.set(false).unwrap(); // disabling ends the session
        settle(&mut ui);
        assert_eq!(
            *modified.borrow(),
            [true, false, true, false, true, false],
            "{context}"
        );
    }

    // InlineDecimal: text mode reports, a drag at rest does not.
    let mut ui = Runtime::new();
    let seconds = ui.signal(0.5_f64);
    let (modified, report) = reports();
    let (_, row) = mount(
        &mut ui,
        InlineDecimal::new(seconds.clone())
            .range(0.0, 10.0)
            .unwrap()
            .step(0.5)
            .unwrap()
            .on_modified(report)
            .key("field"),
    );
    ui.focus(&row).unwrap();
    settle(&mut ui);
    press(&mut ui, KeyCode::F2);
    ui.text_input("2").unwrap();
    settle(&mut ui);
    press(&mut ui, KeyCode::Escape);
    assert_eq!(*modified.borrow(), [true, false]);
    // A value drag at rest previews and saves on release: a gesture.
    let start = center(&ui, &row);
    let [x, _, w, _] = ui.bounds(&row).unwrap().unwrap().values();
    let start = Point::new(x + w - 12.0, start.y()).unwrap();
    ui.pointer_input(PointerEvent::new(
        start,
        PointerKind::Press(PointerButton::Primary),
    ))
    .unwrap();
    for step in 1..=4 {
        let point = Point::new(start.x() + 8.0 * step as f32, start.y()).unwrap();
        ui.pointer_input(PointerEvent::new(point, PointerKind::Move))
            .unwrap();
        settle(&mut ui);
    }
    let end = Point::new(start.x() + 32.0, start.y()).unwrap();
    ui.pointer_input(PointerEvent::new(
        end,
        PointerKind::Release(PointerButton::Primary),
    ))
    .unwrap();
    settle(&mut ui);
    assert_ne!(seconds.get().unwrap(), 0.5, "the drag saved a value");
    assert_eq!(*modified.borrow(), [true, false], "a drag is not a draft");

    // A failing callback: the delivering call returns the error once, a
    // later flush calls nothing, and the session completion still arrives.
    let mut ui = Runtime::new();
    let value = ui.signal(7_i64);
    let calls = Rc::new(RefCell::new(0_u32));
    let counted = calls.clone();
    let (_, field) = mount(
        &mut ui,
        IntegerInput::new(value.clone())
            .on_modified(move |_, modified| {
                *counted.borrow_mut() += 1;
                if modified {
                    Ok(())
                } else {
                    Err(RuntimeError::Exhausted)
                }
            })
            .key("field"),
    );
    replace(&mut ui, &field, "42");
    assert_eq!(
        ui.key_input(KeyCode::Enter, KeyAction::Press).map(|_| ()),
        Err(RuntimeError::Exhausted),
        "the input call that delivered the failing report returns its error"
    );
    ui.key_input(KeyCode::Enter, KeyAction::Release).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(value.get().unwrap(), 42);
    assert_eq!(*calls.borrow(), 2);
    settle(&mut ui);
    assert_eq!(*calls.borrow(), 2, "nothing is replayed");
}

/// Holds an edit of a parameter open from its own delivery, never focused,
/// so the parameter refuses another edit's admission as busy.
struct Holder<T: ParameterValue> {
    parameter: Parameter<T>,
    hold: Signal<bool>,
    holding: bool,
    session: Option<EditSession<T>>,
    notifier: Option<Notifier>,
}
impl<T: ParameterValue> Holder<T> {
    fn new(parameter: Parameter<T>, hold: Signal<bool>) -> Self {
        Self {
            parameter,
            hold,
            holding: false,
            session: None,
            notifier: None,
        }
    }
}
impl<T: ParameterValue> Component for Holder<T> {
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        let notifier = self.notifier.get_or_insert_with(|| cx.notifier()).clone();
        let hold = cx.read(&self.hold)?;
        if hold != self.holding {
            self.holding = hold;
            notifier.request();
        }
        Ok(())
    }
    fn deliver_notifications(&mut self, cx: &mut EventContext) -> Result<(), RuntimeError> {
        if self.holding && self.session.is_none() {
            let revision = self.parameter.snapshot().get()?.revision;
            self.session = Some(self.parameter.begin_edit(cx, revision)?);
        } else if !self.holding
            && let Some(session) = self.session.take()
        {
            session.cancel()?;
        }
        Ok(())
    }
}

/// A host that opens one session editor, focuses it and logs completions.
struct SessionHost {
    build: Box<dyn Fn(&EditorSession) -> Element>,
    session: Option<EditorSession>,
    editor: Option<WidgetId>,
    focused: bool,
    notifier: Option<Notifier>,
    log: Log,
}
impl Component for SessionHost {
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        let notifier = self.notifier.get_or_insert_with(|| cx.notifier()).clone();
        if self.session.is_none() {
            let session = EditorSession::new(notifier);
            session.begin(EditorStart::CaretAtEnd);
            cx.request_children(vec![(self.build)(&session)]);
            self.session = Some(session);
        } else if self.editor.is_none()
            && let Some(child) = cx.children().first()
        {
            self.editor = Some(child.id().clone());
            notifier.request();
        }
        Ok(())
    }
    fn deliver_notifications(&mut self, cx: &mut EventContext) -> Result<(), RuntimeError> {
        if let Some(editor) = &self.editor
            && !self.focused
        {
            self.focused = true;
            cx.request_focus_within(editor)?;
        }
        while let Some(done) = self
            .session
            .as_ref()
            .and_then(EditorSession::take_completion)
        {
            self.log.borrow_mut().push((done.trigger, done.outcome));
        }
        Ok(())
    }
    fn measure(&self, cx: &MeasureContext<'_>, space: Constraints) -> Result<Size, RuntimeError> {
        if cx.is_empty() {
            return Ok(Size::new(0.0, 0.0)?);
        }
        cx.measure(0, space)
    }
    fn arrange(&self, cx: &mut ArrangeContext<'_>, size: Size) -> Result<(), RuntimeError> {
        if !cx.is_empty() {
            cx.place(0, Rect::from_size(size))?;
        }
        Ok(())
    }
}

/// The editors of the policy table, one per row of the design's matrix.
#[derive(Clone, Copy, Debug, PartialEq)]
enum EditorRow {
    TextPlain,
    TextDraft,
    TextSession,
    Search,
    InlineTextBound,
    InlineTextControlled,
    IntegerLocal,
    IntegerSession,
    IntegerParameter,
    InlineIntegerLocal,
    InlineIntegerParameter,
    InlineSlider,
}
impl EditorRow {
    const ALL: [Self; 12] = [
        Self::TextPlain,
        Self::TextDraft,
        Self::TextSession,
        Self::Search,
        Self::InlineTextBound,
        Self::InlineTextControlled,
        Self::IntegerLocal,
        Self::IntegerSession,
        Self::IntegerParameter,
        Self::InlineIntegerLocal,
        Self::InlineIntegerParameter,
        Self::InlineSlider,
    ];
    fn text(self) -> bool {
        matches!(
            self,
            Self::TextPlain
                | Self::TextDraft
                | Self::TextSession
                | Self::Search
                | Self::InlineTextBound
                | Self::InlineTextControlled
        )
    }
    fn in_place(self) -> bool {
        matches!(
            self,
            Self::InlineTextBound
                | Self::InlineTextControlled
                | Self::InlineIntegerLocal
                | Self::InlineIntegerParameter
                | Self::InlineSlider
        )
    }
    fn session(self) -> bool {
        matches!(self, Self::TextSession | Self::IntegerSession)
    }
    fn has_editable(self) -> bool {
        !matches!(
            self,
            Self::InlineTextBound | Self::InlineTextControlled | Self::InlineSlider
        )
    }
    /// The draft each row's test types.
    fn draft(self) -> &'static str {
        match self {
            Self::InlineSlider => "0.75",
            _ if self.text() => "draft",
            _ => "42",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Event {
    Enter,
    FocusLoss,
    ApplicationWrite,
    NotEditable,
    Disabled,
    Hidden,
}

/// One mounted editor of the policy table with a draft typed into it.
struct Subject {
    ui: Runtime,
    field: WidgetId,
    shown: Box<dyn Fn(&Runtime) -> String>,
    written: Box<dyn Fn() -> String>,
    write: Box<dyn Fn(&mut Runtime)>,
    editable: Signal<bool>,
    enabled: Signal<bool>,
    modified: Reports,
    log: Log,
}

fn subject(row: EditorRow) -> Subject {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(400.0, 200.0, 1.0).unwrap())
        .unwrap();
    let editable = ui.signal(true);
    let enabled = ui.signal(true);
    let modified = Reports::default();
    let log = Log::default();
    let report = {
        let sink = modified.clone();
        move |_: &mut EventContext, modified: bool| -> Result<(), RuntimeError> {
            sink.borrow_mut().push(modified);
            Ok(())
        }
    };
    let text = ui.signal("rest".to_owned());
    let integer = ui.signal(7_i64);
    let (count, count_host) = ui
        .parameter(
            ParameterId::new(1, 1, "count").unwrap(),
            ParameterSpec::stepped("Count", 0, 100, 7).unwrap(),
        )
        .unwrap();
    let (gain, gain_host) = ui
        .parameter(
            ParameterId::new(1, 2, "gain").unwrap(),
            ParameterSpec::new("Gain", 0.0, 1.0, 0.25).unwrap(),
        )
        .unwrap();
    let described = Rc::new(RefCell::new("rest".to_owned()));
    let proposals = Rc::new(RefCell::new(Vec::<String>::new()));
    let controlled = {
        let (enabled, proposals) = (enabled.clone(), proposals.clone());
        let report = report.clone();
        move |text: &str| {
            let proposals = proposals.clone();
            InlineText::controlled(text, move |_, text| {
                proposals.borrow_mut().push(text.to_owned());
                Ok(())
            })
            .enabled(&enabled)
            .on_modified(report.clone())
            .key("field")
        }
    };
    let session_host = |build: Box<dyn Fn(&EditorSession) -> Element>| {
        Element::new(
            "host",
            SessionHost {
                build,
                session: None,
                editor: None,
                focused: false,
                notifier: None,
                log: log.clone(),
            },
        )
    };
    let element = match row {
        EditorRow::TextPlain => TextField::new(text.clone())
            .editable(&editable)
            .enabled(&enabled)
            .on_modified(report)
            .key("field"),
        EditorRow::TextDraft => TextField::new(text.clone())
            .max_length(Some(50))
            .editable(&editable)
            .enabled(&enabled)
            .on_modified(report)
            .key("field"),
        EditorRow::TextSession => {
            let (text, editable, enabled) = (text.clone(), editable.clone(), enabled.clone());
            session_host(Box::new(move |session| {
                TextField::new(text.clone())
                    .editable(&editable)
                    .enabled(&enabled)
                    .on_modified(report.clone())
                    .session(session)
                    .key("field")
            }))
        }
        EditorRow::Search => SearchField::new(text.clone())
            .debounce(Duration::from_millis(300))
            .editable(&editable)
            .enabled(&enabled)
            .key("field"),
        EditorRow::InlineTextBound => InlineText::new(text.clone())
            .enabled(&enabled)
            .on_modified(report)
            .key("field"),
        EditorRow::InlineTextControlled => controlled("rest"),
        EditorRow::IntegerLocal => IntegerInput::new(integer.clone())
            .editable(&editable)
            .enabled(&enabled)
            .on_modified(report)
            .key("field"),
        EditorRow::IntegerSession => {
            let (integer, editable, enabled) = (integer.clone(), editable.clone(), enabled.clone());
            session_host(Box::new(move |session| {
                IntegerInput::new(integer.clone())
                    .editable(&editable)
                    .enabled(&enabled)
                    .on_modified(report.clone())
                    .session(session)
                    .key("field")
            }))
        }
        EditorRow::IntegerParameter => IntegerInput::for_parameter(count.clone())
            .editable(&editable)
            .enabled(&enabled)
            .on_modified(report)
            .key("field"),
        EditorRow::InlineIntegerLocal => InlineInteger::new(integer.clone())
            .editable(&editable)
            .enabled(&enabled)
            .on_modified(report)
            .key("field"),
        EditorRow::InlineIntegerParameter => InlineInteger::for_parameter(count.clone())
            .editable(&editable)
            .enabled(&enabled)
            .on_modified(report)
            .key("field"),
        EditorRow::InlineSlider => InlineSlider::new(gain.clone())
            .enabled(&enabled)
            .key("field"),
    };
    let root = ui
        .mount_tree(
            Column::new([element, Button::new("Other", |_| Ok(())).key("other")]).key("root"),
        )
        .unwrap();
    settle(&mut ui);
    let field = ui.find_descendant(&root, "field").unwrap().unwrap();
    // The draft.
    if row.in_place() {
        ui.focus(&field).unwrap();
        settle(&mut ui);
        ui.text_command(&field, TextCommand::BeginEdit).unwrap();
        settle(&mut ui);
        ui.text_input(row.draft()).unwrap();
        settle(&mut ui);
    } else {
        if !row.session() {
            ui.focus(&field).unwrap();
            settle(&mut ui);
        }
        ui.text_command(&field, TextCommand::SelectAll).unwrap();
        ui.text_input(row.draft()).unwrap();
        settle(&mut ui);
    }
    let shown: Box<dyn Fn(&Runtime) -> String> = {
        let field = field.clone();
        if row.in_place() {
            let editing = if matches!(
                row,
                EditorRow::InlineTextBound | EditorRow::InlineTextControlled
            ) {
                "editing"
            } else {
                "text_editing"
            };
            Box::new(move |ui: &Runtime| {
                let inspection = ui.inspection_id(&field).unwrap();
                let open = ui
                    .inspect()
                    .components
                    .into_iter()
                    .find(|c| c.id == inspection)
                    .is_some_and(|c| {
                        c.details.properties.get(editing) == Some(&InspectValue::Bool(true))
                    });
                if open {
                    text_of(ui, &child_editor(ui, &field), "text")
                } else {
                    "-".to_owned()
                }
            })
        } else {
            Box::new(move |ui: &Runtime| text_of(ui, &field, "text"))
        }
    };
    let written: Box<dyn Fn() -> String> = match row {
        _ if row.text() && row != EditorRow::InlineTextControlled => {
            let text = text.clone();
            Box::new(move || text.get().unwrap())
        }
        EditorRow::InlineTextControlled => {
            let (described, proposals) = (described.clone(), proposals.clone());
            Box::new(move || {
                proposals
                    .borrow()
                    .last()
                    .cloned()
                    .unwrap_or_else(|| described.borrow().clone())
            })
        }
        EditorRow::IntegerParameter | EditorRow::InlineIntegerParameter => {
            let count = count.clone();
            Box::new(move || count.snapshot().get().unwrap().base.to_string())
        }
        EditorRow::InlineSlider => {
            let gain = gain.clone();
            Box::new(move || gain.snapshot().get().unwrap().base.to_string())
        }
        _ => {
            let integer = integer.clone();
            Box::new(move || integer.get().unwrap().to_string())
        }
    };
    let write: Box<dyn Fn(&mut Runtime)> = match row {
        _ if row.text() && row != EditorRow::InlineTextControlled => {
            let text = text.clone();
            Box::new(move |_| text.set("other".to_owned()).unwrap())
        }
        EditorRow::InlineTextControlled => {
            let (root, described) = (root.clone(), described.clone());
            Box::new(move |ui: &mut Runtime| {
                "other".clone_into(&mut described.borrow_mut());
                ui.reconcile_children(
                    &root,
                    vec![
                        controlled("other"),
                        Button::new("Other", |_| Ok(())).key("other"),
                    ],
                )
                .unwrap();
            })
        }
        EditorRow::IntegerParameter | EditorRow::InlineIntegerParameter => {
            Box::new(move |_| count_host.write_base(9).unwrap())
        }
        EditorRow::InlineSlider => Box::new(move |_| gain_host.write_base(0.5).unwrap()),
        _ => {
            let integer = integer.clone();
            Box::new(move |_| integer.set(9).unwrap())
        }
    };
    Subject {
        ui,
        field,
        shown,
        written,
        write,
        editable,
        enabled,
        modified,
        log,
    }
}

/// What a cell of the policy table expects: the written value, the shown
/// draft (`-` for an in-place editor at rest), `on_modified`'s last report,
/// and the session log.
type Expect = (
    &'static str,
    &'static str,
    Option<bool>,
    &'static [(EditorTrigger, EditorOutcome)],
);

fn expect(row: EditorRow, event: Event) -> Option<Expect> {
    use EditorOutcome::Saved;
    use EditorRow as R;
    use EditorTrigger::{Enter, FocusLeft};
    use Event as E;
    const NONE: &[(EditorTrigger, EditorOutcome)] = &[];
    Some(match (row, event) {
        // A save attempt by Enter saves an accepted draft.
        (R::TextPlain, E::Enter) => ("draft", "draft", None, NONE),
        (R::TextDraft, E::Enter) => ("draft", "draft", Some(false), NONE),
        (R::TextSession, E::Enter) => ("draft", "draft", Some(false), &[(Enter, Saved)]),
        (R::Search, E::Enter) => ("draft", "draft", None, NONE),
        (R::InlineTextBound | R::InlineTextControlled, E::Enter) => {
            ("draft", "-", Some(false), NONE)
        }
        (R::IntegerLocal | R::IntegerParameter, E::Enter) => ("42", "42", Some(false), NONE),
        (R::IntegerSession, E::Enter) => ("42", "42", Some(false), &[(Enter, Saved)]),
        (R::InlineIntegerLocal | R::InlineIntegerParameter, E::Enter) => {
            ("42", "-", Some(false), NONE)
        }
        (R::InlineSlider, E::Enter) => ("0.75", "-", None, NONE),
        // Focus loss saves a local draft and cancels a parameter draft.
        (R::TextPlain, E::FocusLoss) => ("draft", "draft", None, NONE),
        (R::TextDraft, E::FocusLoss) => ("draft", "draft", Some(false), NONE),
        (R::TextSession, E::FocusLoss) => ("draft", "draft", Some(false), &[(FocusLeft, Saved)]),
        (R::Search, E::FocusLoss) => ("draft", "draft", None, NONE),
        (R::InlineTextBound | R::InlineTextControlled, E::FocusLoss) => {
            ("draft", "-", Some(false), NONE)
        }
        (R::IntegerLocal, E::FocusLoss) => ("42", "42", Some(false), NONE),
        (R::IntegerSession, E::FocusLoss) => ("42", "42", Some(false), &[(FocusLeft, Saved)]),
        (R::IntegerParameter, E::FocusLoss) => ("7", "7", Some(false), NONE),
        (R::InlineIntegerLocal, E::FocusLoss) => ("42", "-", Some(false), NONE),
        (R::InlineIntegerParameter, E::FocusLoss) => ("7", "-", Some(false), NONE),
        (R::InlineSlider, E::FocusLoss) => ("0.25", "-", None, NONE),
        // An application write wins.
        (R::TextPlain | R::Search, E::ApplicationWrite) => ("other", "other", None, NONE),
        (R::TextDraft | R::TextSession, E::ApplicationWrite) => {
            ("other", "other", Some(false), NONE)
        }
        (R::InlineTextBound | R::InlineTextControlled, E::ApplicationWrite) => {
            ("other", "-", Some(false), NONE)
        }
        (R::IntegerLocal | R::IntegerSession | R::IntegerParameter, E::ApplicationWrite) => {
            ("9", "9", Some(false), NONE)
        }
        (R::InlineIntegerLocal | R::InlineIntegerParameter, E::ApplicationWrite) => {
            ("9", "-", Some(false), NONE)
        }
        (R::InlineSlider, E::ApplicationWrite) => ("0.5", "-", None, NONE),
        // Revoked editability keeps the draft unsaved; an in-place session
        // becomes read-only.
        (R::TextPlain, E::NotEditable) => ("draft", "draft", None, NONE),
        (R::TextDraft | R::TextSession, E::NotEditable) => ("rest", "draft", Some(true), NONE),
        (R::Search, E::NotEditable) => ("rest", "draft", None, NONE),
        (R::IntegerLocal | R::IntegerSession | R::IntegerParameter, E::NotEditable) => {
            ("7", "42", Some(true), NONE)
        }
        (R::InlineIntegerLocal | R::InlineIntegerParameter, E::NotEditable) => {
            ("7", "42", Some(true), NONE)
        }
        (R::InlineTextBound | R::InlineTextControlled | R::InlineSlider, E::NotEditable) => {
            return None;
        }
        // Revoked enablement keeps a standalone local draft, with the
        // exceptions: parameters cancel, in-place sessions end and search
        // queries are discarded.
        (R::TextPlain, E::Disabled) => ("draft", "draft", None, NONE),
        (R::TextDraft | R::TextSession, E::Disabled) => ("rest", "draft", Some(true), NONE),
        (R::Search, E::Disabled) => ("rest", "rest", None, NONE),
        (R::InlineTextBound | R::InlineTextControlled, E::Disabled) => {
            ("rest", "-", Some(false), NONE)
        }
        (R::IntegerLocal | R::IntegerSession, E::Disabled) => ("7", "42", Some(true), NONE),
        (R::IntegerParameter, E::Disabled) => ("7", "7", Some(false), NONE),
        (R::InlineIntegerLocal | R::InlineIntegerParameter, E::Disabled) => {
            ("7", "-", Some(false), NONE)
        }
        (R::InlineSlider, E::Disabled) => ("0.25", "-", None, NONE),
        // Hiding discards or cancels; a session hears nothing.
        (R::TextPlain, E::Hidden) => ("draft", "draft", None, NONE),
        (R::TextDraft | R::TextSession, E::Hidden) => ("rest", "rest", Some(false), NONE),
        (R::Search, E::Hidden) => ("rest", "rest", None, NONE),
        (R::InlineTextBound | R::InlineTextControlled, E::Hidden) => {
            ("rest", "-", Some(false), NONE)
        }
        (R::IntegerLocal | R::IntegerSession | R::IntegerParameter, E::Hidden) => {
            ("7", "7", Some(false), NONE)
        }
        (R::InlineIntegerLocal | R::InlineIntegerParameter, E::Hidden) => {
            ("7", "-", Some(false), NONE)
        }
        (R::InlineSlider, E::Hidden) => ("0.25", "-", None, NONE),
    })
}

/// The cells of the policy table this part changed: before it, revoking a
/// number field's enablement discarded its draft. Every other cell held
/// before and after the change.
fn changed(row: EditorRow, event: Event) -> bool {
    matches!(
        (row, event),
        (
            EditorRow::IntegerLocal | EditorRow::IntegerSession,
            Event::Disabled
        )
    )
}

/// Runs one cell of the policy table: a fresh editor of `row` with a draft,
/// then `event`. Returns a description of the difference from the
/// expectation, if any.
fn run_cell(row: EditorRow, event: Event) -> Option<String> {
    let (written, shown, modified, log) = expect(row, event)?;
    assert!(row.has_editable() || event != Event::NotEditable);
    let mut s = subject(row);
    match event {
        Event::Enter => press(&mut s.ui, KeyCode::Enter),
        Event::FocusLoss => {
            s.ui.clear_focus();
            settle(&mut s.ui);
        }
        Event::ApplicationWrite => {
            (s.write)(&mut s.ui);
            settle(&mut s.ui);
        }
        Event::NotEditable => {
            s.editable.set(false).unwrap();
            settle(&mut s.ui);
        }
        Event::Disabled => {
            s.enabled.set(false).unwrap();
            settle(&mut s.ui);
        }
        Event::Hidden => {
            s.ui.set_visible(&s.field, false).unwrap();
            settle(&mut s.ui);
        }
    }
    // A debounce that would write later must not.
    s.ui.flush(s.ui.now() + Duration::from_millis(400)).unwrap();
    let actual = (
        (s.written)(),
        (s.shown)(&s.ui),
        s.modified.borrow().last().copied(),
        s.log.borrow().clone(),
    );
    let expected = (written.to_owned(), shown.to_owned(), modified, log.to_vec());
    (actual != expected).then(|| format!("{row:?} {event:?}: {actual:?}, expected {expected:?}"))
}

const EVENTS: [Event; 6] = [
    Event::Enter,
    Event::FocusLoss,
    Event::ApplicationWrite,
    Event::NotEditable,
    Event::Disabled,
    Event::Hidden,
];

/// The policy table (contract): every editor row against every event, except
/// the cells this part changed, which the next test covers. Mutation checked:
/// making a parameter field save on focus loss fails the parameter rows'
/// focus-loss cells.
#[test]
fn the_policy_table_holds_for_every_built_in_editor() {
    let failures: Vec<String> = EditorRow::ALL
        .into_iter()
        .flat_map(|row| EVENTS.map(|event| (row, event)))
        .filter(|(row, event)| !changed(*row, *event))
        .filter_map(|(row, event)| run_cell(row, event))
        .collect();
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// New-behavior test: the policy table's changed cells, a local number
/// field with and without a session keeping its draft unsaved when its
/// enablement is revoked. Named assertion: no cell differs. Mutation
/// checked: test 1's (the draft discarded on revocation) makes both cells
/// show 7 with `on_modified` false.
#[test]
fn the_policy_tables_changed_cells_keep_a_disabled_number_draft() {
    let failures: Vec<String> = EditorRow::ALL
        .into_iter()
        .flat_map(|row| EVENTS.map(|event| (row, event)))
        .filter(|(row, event)| changed(*row, *event))
        .filter_map(|(row, event)| run_cell(row, event))
        .collect();
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// The center of an ordinary number field's plus button.
fn plus_button(ui: &Runtime, id: &WidgetId) -> Point {
    let [x, y, w, h] = ui.bounds(id).unwrap().unwrap().values();
    let metrics = ui.theme().metrics(Density::Standard).unwrap();
    let (pad, side) = (
        metrics.metric(MetricRole::ControlPaddingX),
        metrics.metric(MetricRole::IconSize),
    );
    Point::new(x + w - pad - side / 2.0, y + h / 2.0).unwrap()
}

/// A parameter case's field, holder, base reader and conflicting write.
type Parts = (Element, Element, Box<dyn Fn() -> String>, Box<dyn Fn()>);

#[derive(Clone, Copy, Debug, PartialEq)]
enum ParameterField {
    Integer,
    Decimal,
    Inline,
}

/// The parameter-bound field of `kind` over a fresh parameter, with its
/// host, an outcome log, the field's `on_rejection` and `on_save` logs and a
/// holder that can keep the parameter busy.
struct ParameterCase {
    ui: Runtime,
    field: WidgetId,
    kind: ParameterField,
    outcomes: Rc<RefCell<Vec<String>>>,
    rejections: Rc<RefCell<Vec<Option<String>>>>,
    saves: Rc<RefCell<Vec<String>>>,
    hold: Signal<bool>,
    base: Box<dyn Fn() -> String>,
    write: Box<dyn Fn()>,
    _keep: Vec<Box<dyn std::any::Any>>,
}
impl ParameterCase {
    fn new(kind: ParameterField, remote: bool) -> Self {
        let mut ui = Runtime::new();
        ui.set_viewport(Viewport::new(400.0, 200.0, 1.0).unwrap())
            .unwrap();
        let outcomes = Rc::new(RefCell::new(Vec::new()));
        let rejections = Rc::new(RefCell::new(Vec::new()));
        let saves = Rc::new(RefCell::new(Vec::new()));
        let hold = ui.signal(false);
        let rejection = {
            let sink = rejections.clone();
            move |_: &mut EventContext, error: Option<&InputRejection>| {
                sink.borrow_mut().push(error.map(ToString::to_string));
                Ok(())
            }
        };
        let mut keep: Vec<Box<dyn std::any::Any>> = Vec::new();
        let (field, holder, base, write): Parts = if kind == ParameterField::Decimal {
            let spec = ParameterSpec::new("Gain", 0.0, 1.0, 0.25).unwrap();
            let id = ParameterId::new(1, 1, "gain").unwrap();
            let (gain, write): (Parameter<f32>, Box<dyn Fn()>) = if remote {
                let (gain, host) = ui.remote_parameter(id, spec).unwrap();
                keep.push(Box::new(host));
                (gain, Box::new(|| {}))
            } else {
                let (gain, host) = ui.parameter(id, spec).unwrap();
                let sink = outcomes.clone();
                let monitor = host
                    .on_outcome(move |event| {
                        sink.borrow_mut().push(format!("{:?}", event.outcome));
                        Ok(())
                    })
                    .unwrap();
                keep.push(Box::new(monitor));
                (gain, Box::new(move || host.write_base(0.75).unwrap()))
            };
            let saved = saves.clone();
            let field = DecimalInput::for_parameter(gain.clone())
                .stepper(true)
                .on_rejection(rejection)
                .on_save(move |_, value| {
                    saved.borrow_mut().push(value.to_string());
                    Ok(())
                })
                .key("field");
            let reader = gain.clone();
            (
                field,
                Holder::new(gain, hold.clone()).key("holder"),
                Box::new(move || reader.snapshot().get().unwrap().base.to_string()),
                write,
            )
        } else {
            let spec = ParameterSpec::stepped("Count", 0, 96, 12)
                .unwrap()
                .with_step(std::num::NonZeroU64::new(4).unwrap())
                .unwrap();
            let id = ParameterId::new(1, 1, "count").unwrap();
            let (count, write): (Parameter<i64>, Box<dyn Fn()>) = if remote {
                let (count, host) = ui.remote_parameter(id, spec).unwrap();
                keep.push(Box::new(host));
                (count, Box::new(|| {}))
            } else {
                let (count, host) = ui.parameter(id, spec).unwrap();
                let sink = outcomes.clone();
                let monitor = host
                    .on_outcome(move |event| {
                        sink.borrow_mut().push(format!("{:?}", event.outcome));
                        Ok(())
                    })
                    .unwrap();
                keep.push(Box::new(monitor));
                (count, Box::new(move || host.write_base(40).unwrap()))
            };
            let saved = saves.clone();
            let save = move |_: &mut EventContext, value: i64| {
                saved.borrow_mut().push(value.to_string());
                Ok(())
            };
            let field = if kind == ParameterField::Inline {
                InlineInteger::for_parameter(count.clone())
                    .on_rejection(rejection)
                    .on_save(save)
                    .key("field")
            } else {
                IntegerInput::for_parameter(count.clone())
                    .stepper(true)
                    .on_rejection(rejection)
                    .on_save(save)
                    .key("field")
            };
            let reader = count.clone();
            (
                field,
                Holder::new(count, hold.clone()).key("holder"),
                Box::new(move || reader.snapshot().get().unwrap().base.to_string()),
                write,
            )
        };
        let root = ui
            .mount_tree(
                Column::new([field, holder, Button::new("Other", |_| Ok(())).key("other")])
                    .key("root"),
            )
            .unwrap();
        settle(&mut ui);
        let field = ui.find_descendant(&root, "field").unwrap().unwrap();
        Self {
            ui,
            field,
            kind,
            outcomes,
            rejections,
            saves,
            hold,
            base,
            write,
            _keep: keep,
        }
    }
    /// Types `text` as the field's draft, in text mode for an inline row.
    fn draft(&mut self, text: &str) {
        self.ui.focus(&self.field).unwrap();
        settle(&mut self.ui);
        if self.kind == ParameterField::Inline {
            press(&mut self.ui, KeyCode::F2);
        } else {
            self.ui
                .text_command(&self.field, TextCommand::SelectAll)
                .unwrap();
        }
        self.ui.text_input(text).unwrap();
        settle(&mut self.ui);
    }
    /// The draft or value the field shows.
    fn shown(&self) -> String {
        if self.text_mode() {
            text_of(&self.ui, &child_editor(&self.ui, &self.field), "text")
        } else {
            text_of(&self.ui, &self.field, "text")
        }
    }
    fn text_mode(&self) -> bool {
        self.kind == ParameterField::Inline
            && property(&self.ui, "field", "text_editing") == Some(InspectValue::Bool(true))
    }
}

/// The parameter number fields' five save outcomes (contract). Mutation
/// checked: keeping the draft on a refused admission fails the replacement
/// assertion.
#[test]
fn parameter_number_fields_distinguish_their_save_outcomes() {
    for kind in [
        ParameterField::Integer,
        ParameterField::Decimal,
        ParameterField::Inline,
    ] {
        let context = format!("{kind:?}");
        let decimal = kind == ParameterField::Decimal;
        let (base, invalid, valid, conflict) = if decimal {
            ("0.25", "x", "0.5", "0.75")
        } else {
            ("12", "13", "24", "40")
        };
        let mut c = ParameterCase::new(kind, false);
        // A validation refusal keeps the draft with the outline and reports
        // it; nothing reaches the host.
        c.draft(invalid);
        press(&mut c.ui, KeyCode::Enter);
        assert_eq!(c.shown(), invalid, "{context}");
        assert!(
            matches!(c.rejections.borrow().last(), Some(Some(_))),
            "{context}"
        );
        assert!(c.saves.borrow().is_empty(), "{context}");
        assert!(c.outcomes.borrow().is_empty(), "{context}");
        press(&mut c.ui, KeyCode::Escape);
        // A refused admission (another edit holds the parameter) replaces
        // the draft with the parameter's value; no host outcome.
        c.hold.set(true).unwrap();
        settle(&mut c.ui);
        c.draft(valid);
        press(&mut c.ui, KeyCode::Enter);
        assert_eq!(c.shown(), base, "{context}: replaced by the base");
        assert!(!c.text_mode(), "{context}: an inline row leaves text mode");
        assert!(c.saves.borrow().is_empty(), "{context}");
        assert!(c.outcomes.borrow().is_empty(), "{context}");
        c.hold.set(false).unwrap();
        settle(&mut c.ui);
        c.outcomes.borrow_mut().clear();
        // A terminal refusal of an admitted edit: a host write during a held
        // step gesture ends it as a conflict, and the field shows the new base.
        c.ui.focus(&c.field).unwrap();
        settle(&mut c.ui);
        if kind == ParameterField::Inline {
            c.ui.key_input(KeyCode::ArrowUp, KeyAction::Press).unwrap();
            settle(&mut c.ui);
            (c.write)();
            settle(&mut c.ui);
            c.ui.key_input(KeyCode::ArrowUp, KeyAction::Release)
                .unwrap();
        } else {
            let plus = plus_button(&c.ui, &c.field);
            c.ui.pointer_input(PointerEvent::new(
                plus,
                PointerKind::Press(PointerButton::Primary),
            ))
            .unwrap();
            settle(&mut c.ui);
            (c.write)();
            settle(&mut c.ui);
            c.ui.pointer_input(PointerEvent::new(
                plus,
                PointerKind::Release(PointerButton::Primary),
            ))
            .unwrap();
        }
        settle(&mut c.ui);
        assert_eq!(*c.outcomes.borrow(), ["Rejected(Conflict)"], "{context}");
        assert_eq!(c.shown(), conflict, "{context}");
        assert_eq!((c.base)(), conflict, "{context}");
        assert!(c.saves.borrow().is_empty(), "{context}");
        c.outcomes.borrow_mut().clear();
        // An unchanged Enter begins no edit.
        c.draft(conflict);
        press(&mut c.ui, KeyCode::Enter);
        assert!(c.outcomes.borrow().is_empty(), "{context}");
        assert!(c.saves.borrow().is_empty(), "{context}");
        // A valid draft commits once and is reported.
        c.draft(valid);
        press(&mut c.ui, KeyCode::Enter);
        assert_eq!(c.outcomes.borrow().len(), 1, "{context}");
        assert_eq!(*c.saves.borrow(), [valid], "{context}");
        assert_eq!((c.base)(), valid, "{context}");

        // A remote authority's submission calls `on_save` and shows the
        // submitted value without changing the base.
        let mut c = ParameterCase::new(kind, true);
        c.draft(valid);
        press(&mut c.ui, KeyCode::Enter);
        assert_eq!(*c.saves.borrow(), [valid], "{context}: remote");
        assert_eq!(c.shown(), valid, "{context}: remote");
        assert_eq!((c.base)(), base, "{context}: remote");
    }
}

/// `InlineSlider`'s own contract (unchanged by the policy). Named assertion:
/// text mode opens with the parameter edit begun. Mutation checked: beginning
/// the edit at confirmation instead leaves no text mode open with an edit.
#[test]
fn an_inline_slider_begins_its_edit_with_text_mode_and_commits_on_enter() {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(400.0, 200.0, 1.0).unwrap())
        .unwrap();
    let (gain, host) = ui
        .parameter(
            ParameterId::new(1, 1, "gain").unwrap(),
            ParameterSpec::new("Gain", 0.0, 1.0, 0.25).unwrap(),
        )
        .unwrap();
    let outcomes = Rc::new(RefCell::new(Vec::new()));
    let sink = outcomes.clone();
    let _monitor = host
        .on_outcome(move |event| {
            sink.borrow_mut().push(format!("{:?}", event.outcome));
            Ok(())
        })
        .unwrap();
    let hold = ui.signal(false);
    let root = ui
        .mount_tree(
            Column::new([
                InlineSlider::new(gain.clone()).key("field"),
                Holder::new(gain.clone(), hold.clone()).key("holder"),
                Button::new("Other", |_| Ok(())).key("other"),
            ])
            .key("root"),
        )
        .unwrap();
    settle(&mut ui);
    let field = ui.find_descendant(&root, "field").unwrap().unwrap();
    let text_mode =
        |ui: &Runtime| property(ui, "field", "text_editing") == Some(InspectValue::Bool(true));
    let begun = || gain.snapshot().get().unwrap().edit_source.is_some();
    // A refused admission opens no text mode and no session.
    hold.set(true).unwrap();
    settle(&mut ui);
    ui.focus(&field).unwrap();
    settle(&mut ui);
    press(&mut ui, KeyCode::F2);
    assert!(!text_mode(&ui));
    hold.set(false).unwrap();
    settle(&mut ui);
    outcomes.borrow_mut().clear();
    // Opening text mode begins the edit.
    ui.focus(&field).unwrap();
    settle(&mut ui);
    press(&mut ui, KeyCode::F2);
    assert!(text_mode(&ui) && begun(), "the edit begins with text mode");
    // Unparsable text stays with the edit open.
    ui.text_input("x").unwrap();
    settle(&mut ui);
    press(&mut ui, KeyCode::Enter);
    assert!(text_mode(&ui));
    assert!(begun());
    assert_eq!(text_of(&ui, &child_editor(&ui, &field), "text"), "x");
    // Escape cancels.
    press(&mut ui, KeyCode::Escape);
    assert!(!text_mode(&ui));
    assert_eq!(*outcomes.borrow(), ["Cancelled"]);
    // Enter with an unchanged value commits once.
    press(&mut ui, KeyCode::F2);
    press(&mut ui, KeyCode::Enter);
    assert_eq!(outcomes.borrow().len(), 2);
    assert!(outcomes.borrow()[1].starts_with("Committed"));
    // A conflicting host write while typing ends text mode.
    press(&mut ui, KeyCode::F2);
    ui.text_input("0.5").unwrap();
    settle(&mut ui);
    host.write_base(0.75).unwrap();
    settle(&mut ui);
    assert!(!text_mode(&ui));
    assert_eq!(gain.snapshot().get().unwrap().base, 0.75);
    // Focus loss cancels.
    press(&mut ui, KeyCode::F2);
    ui.text_input("0.5").unwrap();
    settle(&mut ui);
    ui.clear_focus();
    settle(&mut ui);
    assert!(!text_mode(&ui));
    assert_eq!(
        outcomes.borrow().last().map(String::as_str),
        Some("Cancelled")
    );
    assert_eq!(gain.snapshot().get().unwrap().base, 0.75);
}

/// The inputs that initiate no save attempt (contract): Enter in a
/// `TextArea`, and Enter and Tab in a session field while a composition is
/// active, where Escape ends only the composition.
#[test]
fn some_inputs_initiate_no_save_attempt() {
    let mut ui = Runtime::new();
    let notes = ui.signal(String::new());
    let (_, area) = mount(
        &mut ui,
        TextArea::new(notes.clone())
            .max_length(Some(100))
            .key("field"),
    );
    ui.focus(&area).unwrap();
    settle(&mut ui);
    ui.text_input("a").unwrap();
    press(&mut ui, KeyCode::Enter);
    assert_eq!(notes.get().unwrap(), "", "Enter inserts a line break");
    assert_eq!(text_of(&ui, &area, "text"), "a\n");
    ui.clear_focus();
    settle(&mut ui);
    assert_eq!(notes.get().unwrap(), "a\n", "focus loss saves");

    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(400.0, 200.0, 1.0).unwrap())
        .unwrap();
    let value = ui.signal(7_i64);
    let log = Log::default();
    let bound = value.clone();
    let root = ui
        .mount_tree(
            Column::new([
                Element::new(
                    "host",
                    SessionHost {
                        build: Box::new(move |session| {
                            IntegerInput::new(bound.clone())
                                .session(session)
                                .key("field")
                        }),
                        session: None,
                        editor: None,
                        focused: false,
                        notifier: None,
                        log: log.clone(),
                    },
                ),
                Button::new("Other", |_| Ok(())).key("other"),
            ])
            .key("root"),
        )
        .unwrap();
    settle(&mut ui);
    let field = ui.find_descendant(&root, "field").unwrap().unwrap();
    ui.text_command(&field, TextCommand::SelectAll).unwrap();
    ui.text_input("42").unwrap();
    ui.text_preedit(&TextPreedit::new("9", None).unwrap())
        .unwrap();
    settle(&mut ui);
    press(&mut ui, KeyCode::Enter);
    press(&mut ui, KeyCode::Tab);
    assert_eq!(value.get().unwrap(), 7);
    assert!(log.borrow().is_empty());
    press(&mut ui, KeyCode::Escape);
    assert_eq!(
        property(&ui, "field", "composing"),
        Some(InspectValue::Bool(false))
    );
    assert_eq!(
        text_of(&ui, &field, "text"),
        "42",
        "Escape ended only the composition"
    );
    assert!(log.borrow().is_empty());
}

/// Records the pointer events that reach it.
struct PointerProbe(Rc<RefCell<Vec<PointerKind>>>);
impl Component for PointerProbe {
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
    fn measure(&self, _: &MeasureContext<'_>, _: Constraints) -> Result<Size, RuntimeError> {
        Ok(Size::new(240.0, 40.0)?)
    }
    fn pointer_input(
        &mut self,
        _: &mut EventContext,
        event: PointerEvent,
    ) -> Result<InputOutcome, RuntimeError> {
        self.0.borrow_mut().push(event.kind());
        Ok(InputOutcome::Handled)
    }
}

/// New-behavior test for `DiscardDraft` (review point 1): discarding during a
/// selection drag releases the drag's pointer capture. Named assertion: with
/// the primary button still held, a move over a sibling reaches the sibling.
/// Mutations checked: without the release in the number field's
/// `discard_command`, in `SearchField`'s answer, and in `DraftEditor`'s
/// session end (the inline row), the move goes to the captured editor.
#[test]
fn discard_draft_during_a_selection_drag_releases_the_capture() {
    #[derive(Clone, Copy, Debug, PartialEq)]
    enum Kind {
        Number,
        Inline,
        Search,
    }
    for kind in [Kind::Number, Kind::Inline, Kind::Search] {
        let mut ui = Runtime::new();
        ui.set_viewport(Viewport::new(400.0, 200.0, 1.0).unwrap())
            .unwrap();
        let number = ui.signal(7_i64);
        let query = ui.signal(String::new());
        let seen = Rc::new(RefCell::new(Vec::new()));
        let field = match kind {
            Kind::Number => IntegerInput::new(number.clone()).key("field"),
            Kind::Inline => InlineInteger::new(number.clone()).key("field"),
            Kind::Search => SearchField::new(query.clone())
                .debounce(Duration::from_millis(300))
                .key("field"),
        };
        let root = ui
            .mount_tree(Element::new(
                "root",
                Column::new([field, Element::new("probe", PointerProbe(seen.clone()))]),
            ))
            .unwrap();
        settle(&mut ui);
        let field = ui.find_descendant(&root, "field").unwrap().unwrap();
        let probe = ui.find_descendant(&root, "probe").unwrap().unwrap();
        ui.focus(&field).unwrap();
        settle(&mut ui);
        if kind == Kind::Inline {
            press(&mut ui, KeyCode::F2);
        } else {
            ui.text_command(&field, TextCommand::SelectAll).unwrap();
        }
        ui.text_input(if kind == Kind::Search {
            "abcdef"
        } else {
            "424242"
        })
        .unwrap();
        settle(&mut ui);
        let target = if kind == Kind::Inline {
            child_editor(&ui, &field)
        } else {
            field.clone()
        };
        let point = center(&ui, &target);
        ui.pointer_input(PointerEvent::new(
            point,
            PointerKind::Press(PointerButton::Primary),
        ))
        .unwrap();
        ui.pointer_input(PointerEvent::new(
            Point::new(point.x() - 10.0, point.y()).unwrap(),
            PointerKind::Move,
        ))
        .unwrap();
        assert_eq!(
            ui.text_command(&field, TextCommand::DiscardDraft).unwrap(),
            TextCommandOutcome::Handled,
            "{kind:?}"
        );
        seen.borrow_mut().clear();
        let over = center(&ui, &probe);
        ui.pointer_input(PointerEvent::new(over, PointerKind::Move))
            .unwrap();
        assert!(
            seen.borrow().contains(&PointerKind::Move),
            "{kind:?}: the probe receives the move while the button is held: {:?}",
            seen.borrow()
        );
        ui.pointer_input(PointerEvent::new(
            over,
            PointerKind::Release(PointerButton::Primary),
        ))
        .unwrap();
        settle(&mut ui);
        assert_eq!(number.get().unwrap(), 7, "{kind:?}");
        assert_eq!(query.get().unwrap(), "", "{kind:?}");
    }
}

/// One row whose title is an `InlineText` in row mode.
struct Titles(Signal<String>);
impl ListItems for Titles {
    fn len(&self) -> usize {
        1
    }
    fn key(&self, index: usize) -> Option<String> {
        (index == 0).then(|| "a".to_owned())
    }
    fn build(&self, index: usize) -> Option<Element> {
        (index == 0).then(|| Element::new("title", InlineText::new(&self.0).in_row(true)))
    }
}

/// Sends `DiscardDraft` to a target from its own delivery when `go` turns
/// true.
struct Discarder {
    target: Rc<RefCell<Option<WidgetId>>>,
    go: Signal<bool>,
    sent: bool,
    notifier: Option<Notifier>,
}
impl Component for Discarder {
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        let notifier = self.notifier.get_or_insert_with(|| cx.notifier()).clone();
        if cx.read(&self.go)? && !self.sent {
            notifier.request();
        }
        Ok(())
    }
    fn deliver_notifications(&mut self, cx: &mut EventContext) -> Result<(), RuntimeError> {
        let target = self.target.borrow().clone();
        if let Some(target) = target
            && !self.sent
        {
            self.sent = true;
            cx.request_text_command(&target, TextCommand::DiscardDraft)?;
        }
        Ok(())
    }
}

/// `DiscardDraft` on a row-mode `InlineText` whose session holds focus, sent
/// directly and queued: the session ends unsaved and, as when Escape ends
/// it, focus falls back to the nearest focusable ancestor, the list, rather
/// than to the label, which is not focusable at rest in a row.
#[test]
fn discard_draft_in_a_row_leaves_focus_to_the_list() {
    for queued in [false, true] {
        let mut ui = Runtime::new();
        ui.set_viewport(Viewport::new(240.0, 160.0, 1.0).unwrap())
            .unwrap();
        let name = ui.signal("Mix".to_owned());
        let rows = ui.signal(ListSource::new(Titles(name.clone())));
        let go = ui.signal(false);
        let target: Rc<RefCell<Option<WidgetId>>> = Rc::default();
        let root = ui
            .mount_tree(Element::new(
                "root",
                Column::new([
                    Element::new(
                        "list",
                        VirtualList::new(rows.read_only(), 32.0)
                            .unwrap()
                            .selection(ListSelection::new(&ui, SelectionMode::Single)),
                    )
                    .with_grow(1),
                    Element::new(
                        "discarder",
                        Discarder {
                            target: target.clone(),
                            go: go.clone(),
                            sent: false,
                            notifier: None,
                        },
                    ),
                ]),
            ))
            .unwrap();
        settle(&mut ui);
        let list = ui.find_descendant(&root, "list").unwrap().unwrap();
        // The list keys each row by its item, so the label is the row "a".
        let title = ui.find_descendant(&root, "a").unwrap().unwrap();
        // The list is the collection's keyboard owner, as after a press on
        // the row.
        ui.focus(&list).unwrap();
        settle(&mut ui);
        ui.text_command(&title, TextCommand::BeginEdit).unwrap();
        settle(&mut ui);
        ui.text_input("Master").unwrap();
        settle(&mut ui);
        assert_eq!(
            ui.focused(),
            Some(child_editor(&ui, &title)),
            "queued {queued}"
        );
        if queued {
            *target.borrow_mut() = Some(title.clone());
            go.set(true).unwrap();
        } else {
            assert_eq!(
                ui.text_command(&title, TextCommand::DiscardDraft).unwrap(),
                TextCommandOutcome::Handled
            );
        }
        settle(&mut ui);
        assert_eq!(
            property(&ui, "a", "editing"),
            Some(InspectValue::Bool(false)),
            "queued {queued}"
        );
        assert_eq!(name.get().unwrap(), "Mix");
        assert_eq!(ui.focused(), Some(list), "queued {queued}");
    }
}
