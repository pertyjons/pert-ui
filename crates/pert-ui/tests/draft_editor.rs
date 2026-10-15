//! `DraftEditor` contracts through the public facade: a consumer-written row
//! owns the session, commits and cancels by bubbled keys, steps through the
//! payload and saves on focus loss.
use pert_ui::*;
mod support;
use std::{cell::RefCell, rc::Rc};
use support::{find, frame, key};

type Error = Box<dyn std::error::Error>;
type TestResult = Result<(), Error>;
type Script =
    Box<dyn FnMut(&mut EventContext, &DraftEditorController<f64>) -> Result<(), RuntimeError>>;

#[derive(Default)]
struct Log {
    keys: Vec<KeyCode>,
    edits: Vec<(String, Option<f64>)>,
    results: Vec<Result<(), RuntimeError>>,
    focus_losses: usize,
}

/// A consumer's inline row: caption area, a draft editor in the value slot.
/// Enter opens and commits, Escape cancels, Up/Down step by 0.25 through the
/// payload, F2 runs a test script with the row's context.
struct Row {
    value: Signal<f64>,
    draft: DraftEditorController<f64>,
    editor: Option<WidgetId>,
    log: Rc<RefCell<Log>>,
    script: Option<Script>,
    children: Vec<Element>,
}

fn draft_value(draft: &DraftEditorController<f64>) -> Option<f64> {
    draft.payload().or_else(|| draft.text().trim().parse().ok())
}

impl Row {
    fn begin(&mut self, cx: &mut EventContext) -> Result<(), RuntimeError> {
        let value = self.value.get()?;
        self.draft
            .begin(cx, &format!("{value:.1}"), Some(value), DraftSelection::All)?;
        if let Some(editor) = &self.editor {
            cx.request_focus_for(editor)?;
        }
        Ok(())
    }
}

impl Component for Row {
    fn take_children(&mut self) -> Vec<Element> {
        std::mem::take(&mut self.children)
    }
    fn focusable(&self) -> bool {
        !self.draft.is_active()
    }
    fn child_visible(&self, _key: &str) -> bool {
        self.draft.is_active()
    }
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        self.editor = cx.children().first().map(|child| child.id().clone());
        Ok(())
    }
    fn measure(
        &self,
        _: &MeasureContext<'_>,
        available: Constraints,
    ) -> Result<Size, RuntimeError> {
        Ok(Size::new(
            available.width().map_or(240.0, |w| w.min(240.0)),
            24.0,
        )?)
    }
    fn arrange(&self, cx: &mut ArrangeContext<'_>, size: Size) -> Result<(), RuntimeError> {
        for index in 0..cx.len() {
            cx.place(index, Rect::new(60.0, 0.0, 120.0, size.height())?)?;
        }
        Ok(())
    }
    // Focus left the row by Tab, a press elsewhere or window focus loss: save,
    // as local values do.
    fn focus_within_changed(
        &mut self,
        _: &mut EventContext,
        within: bool,
    ) -> Result<(), RuntimeError> {
        if within || !self.draft.is_active() {
            return Ok(());
        }
        self.log.borrow_mut().focus_losses += 1;
        if let Some(value) = draft_value(&self.draft) {
            self.value.set(value)?;
        }
        self.draft.end();
        Ok(())
    }
    // A press on the caption runs the script too, for states in which the
    // editor keeps every key, such as a composition.
    fn pointer_input(
        &mut self,
        cx: &mut EventContext,
        event: PointerEvent,
    ) -> Result<InputOutcome, RuntimeError> {
        if event.kind() != PointerKind::Press(PointerButton::Primary) {
            return Ok(InputOutcome::Ignored);
        }
        if let Some(script) = &mut self.script {
            let result = script(cx, &self.draft);
            self.log.borrow_mut().results.push(result);
        }
        Ok(InputOutcome::Handled)
    }
    fn keyboard_event(
        &mut self,
        cx: &mut EventContext,
        event: KeyEvent,
        _: &KeyboardState,
    ) -> Result<InputOutcome, RuntimeError> {
        if event.transition != KeyTransition::Pressed {
            return Ok(InputOutcome::Ignored);
        }
        self.log.borrow_mut().keys.push(event.code);
        let active = self.draft.is_active();
        match event.code {
            KeyCode::F2 => {
                if let Some(script) = &mut self.script {
                    let result = script(cx, &self.draft);
                    self.log.borrow_mut().results.push(result);
                }
            }
            KeyCode::Enter if !active => self.begin(cx)?,
            KeyCode::Enter => {
                if let Some(value) = draft_value(&self.draft) {
                    self.value.set(value)?;
                }
                self.draft.end();
                cx.request_focus();
            }
            KeyCode::Escape if active => {
                self.draft.end();
                cx.request_focus();
            }
            KeyCode::ArrowUp | KeyCode::ArrowDown if active => {
                let step = if event.code == KeyCode::ArrowUp {
                    0.25
                } else {
                    -0.25
                };
                let next = draft_value(&self.draft).unwrap_or(self.value.get()?) + step;
                self.draft.replace(cx, &format!("{next:.1}"), Some(next))?;
            }
            _ => return Ok(InputOutcome::Ignored),
        }
        cx.invalidate(Invalidation::Layout);
        Ok(InputOutcome::Handled)
    }
}

struct Fixture {
    ui: Runtime,
    value: Signal<f64>,
    draft: DraftEditorController<f64>,
    root: WidgetId,
    row: WidgetId,
    log: Rc<RefCell<Log>>,
}

impl Fixture {
    fn new(align: DraftAlign, script: Option<Script>) -> Result<Self, Error> {
        let mut ui = Runtime::new();
        ui.set_viewport(Viewport::new(400.0, 200.0, 1.0)?)?;
        let value = ui.signal(120.0);
        let draft = DraftEditorController::new();
        let log = Rc::new(RefCell::new(Log::default()));
        {
            let log = log.clone();
            draft.on_edit(move |_, text, payload| {
                log.borrow_mut().edits.push((text.to_owned(), payload));
                Ok(())
            });
        }
        let row = Row {
            value: value.clone(),
            draft: draft.clone(),
            editor: None,
            log: log.clone(),
            script,
            children: vec![Element::new(
                "editor",
                DraftEditor::new(draft.clone()).align(align),
            )],
        };
        let root = ui.mount_tree(Element::new(
            "root",
            Column::new([
                Element::new("before", Button::new("Before", |_| Ok(()))),
                Element::new("row", row),
                Element::new("after", Button::new("After", |_| Ok(()))),
            ]),
        ))?;
        frame(&mut ui);
        let row = find(&ui, &root, "row").ok_or("no row")?;
        ui.focus(&row)?;
        frame(&mut ui);
        Ok(Self {
            ui,
            value,
            draft,
            root,
            row,
            log,
        })
    }
    fn press(&mut self, code: KeyCode) {
        key(&mut self.ui, code);
        frame(&mut self.ui);
    }
    fn chord(&mut self, modifier: KeyCode, code: KeyCode) {
        self.ui.key_input(modifier, KeyAction::Press).unwrap();
        key(&mut self.ui, code);
        self.ui.key_input(modifier, KeyAction::Release).unwrap();
        frame(&mut self.ui);
    }
    fn editor(&self) -> WidgetId {
        find(&self.ui, &self.row, "editor").expect("the editor is mounted")
    }
    fn focused(&self, key: &str) -> bool {
        self.ui.focused().is_some() && self.ui.focused() == find(&self.ui, &self.root, key)
    }
    fn editor_details(&self) -> ComponentDetails {
        self.ui
            .inspect()
            .components
            .into_iter()
            .find(|c| c.key == "editor")
            .map(|c| c.details)
            .expect("the editor is mounted")
    }
}

#[test]
fn enter_opens_the_value_selected_and_enter_commits_the_typed_draft() -> TestResult {
    let mut f = Fixture::new(DraftAlign::Start, None)?;
    f.press(KeyCode::Enter);
    assert!(f.draft.is_active());
    assert_eq!(f.draft.text(), "120.0");
    assert_eq!(
        f.ui.focused(),
        Some(f.editor()),
        "focus moved to the editor"
    );
    f.ui.text_input("128")?;
    frame(&mut f.ui);
    assert_eq!(f.draft.text(), "128", "typing replaced the selected value");
    assert_eq!(f.draft.payload(), None, "a user edit clears the payload");
    assert_eq!(f.value.get()?, 120.0, "nothing is written while editing");
    f.press(KeyCode::Enter);
    assert_eq!(f.value.get()?, 128.0);
    assert!(!f.draft.is_active());
    assert_eq!(f.draft.text(), "", "an ended session holds no draft");
    assert_eq!(
        f.ui.focused(),
        Some(f.row.clone()),
        "focus returned to the row"
    );
    Ok(())
}

#[test]
fn escape_cancels_the_session_even_with_the_value_selected() -> TestResult {
    let mut f = Fixture::new(DraftAlign::Start, None)?;
    f.press(KeyCode::Enter);
    f.ui.text_input("9")?;
    f.chord(KeyCode::ShiftLeft, KeyCode::ArrowLeft);
    f.press(KeyCode::Escape);
    assert!(!f.draft.is_active(), "Escape reached the owner");
    assert_eq!(f.value.get()?, 120.0);
    assert_eq!(f.ui.focused(), Some(f.row.clone()));
    Ok(())
}

#[test]
fn a_composition_keeps_escape_and_enter_in_the_editor() -> TestResult {
    let mut f = Fixture::new(DraftAlign::Start, None)?;
    f.press(KeyCode::Enter);
    f.log.borrow_mut().keys.clear();
    f.ui.text_preedit(&TextPreedit::new("x", None)?)?;
    f.press(KeyCode::Escape);
    assert!(f.draft.is_active(), "Escape only cancelled the composition");
    assert!(f.log.borrow().keys.is_empty(), "the owner saw no key");
    assert_eq!(
        f.editor_details().properties["composing"],
        InspectValue::Bool(false)
    );
    f.press(KeyCode::Escape);
    assert!(!f.draft.is_active(), "the next Escape reached the owner");
    Ok(())
}

#[test]
fn arrows_reach_the_owner_and_undo_restores_the_exact_payload() -> TestResult {
    let mut f = Fixture::new(DraftAlign::Start, None)?;
    f.press(KeyCode::Enter);
    f.press(KeyCode::ArrowUp);
    assert_eq!(f.draft.payload(), Some(120.25), "the exact stepped value");
    assert_eq!(
        f.draft.text(),
        format!("{:.1}", 120.25),
        "a rounded display"
    );
    assert!(
        f.log.borrow().edits.is_empty(),
        "replace does not run on_edit"
    );
    f.ui.text_input("7")?;
    frame(&mut f.ui);
    assert_eq!(f.draft.payload(), None);
    assert_eq!(f.log.borrow().edits.len(), 1, "typing runs on_edit");
    f.chord(KeyCode::primary(), KeyCode::KeyZ);
    assert_eq!(
        f.draft.payload(),
        Some(120.25),
        "undo restores text and payload"
    );
    assert_eq!(f.log.borrow().edits.len(), 2, "undo runs on_edit");
    f.press(KeyCode::Enter);
    assert_eq!(
        f.value.get()?,
        120.25,
        "the payload, not the rounded text, is committed"
    );
    Ok(())
}

#[test]
fn a_new_session_carries_no_history_from_the_last_one() -> TestResult {
    let mut f = Fixture::new(DraftAlign::Start, None)?;
    f.press(KeyCode::Enter);
    f.ui.text_input("1")?;
    f.press(KeyCode::Escape);
    f.press(KeyCode::Enter);
    f.chord(KeyCode::primary(), KeyCode::KeyZ);
    assert_eq!(
        f.draft.text(),
        "120.0",
        "nothing to undo in a fresh session"
    );
    Ok(())
}

#[test]
fn begin_rejects_text_and_selections_the_editor_cannot_hold() -> TestResult {
    let script: Script = Box::new(|cx, draft| {
        let cases: [(&str, DraftSelection); 5] = [
            ("a\nb", DraftSelection::All),
            ("é", DraftSelection::Range(0..1)),
            (
                "ab",
                DraftSelection::Range(std::ops::Range { start: 2, end: 1 }),
            ),
            ("ab", DraftSelection::Range(0..3)),
            (
                &"x".repeat(TextEditError::MAX_TEXT_BYTES + 1),
                DraftSelection::End,
            ),
        ];
        let expected = [
            TextEditError::InvalidText,
            TextEditError::InvalidSelection,
            TextEditError::InvalidSelection,
            TextEditError::InvalidSelection,
            TextEditError::TooLarge,
        ];
        for ((text, selection), error) in cases.into_iter().zip(expected) {
            assert_eq!(draft.begin(cx, text, None, selection), Err(error.into()));
            assert!(!draft.is_active(), "a rejected begin starts nothing");
        }
        assert_eq!(
            draft.replace(cx, "1", None),
            Ok(false),
            "no session to replace"
        );
        draft.begin(cx, "é", None, DraftSelection::Range(0..2))?;
        assert_eq!(
            draft.replace(cx, "a\tb", None),
            Err(TextEditError::InvalidText.into())
        );
        assert_eq!(draft.text(), "é", "a rejected replace changes nothing");
        Ok(())
    });
    let mut f = Fixture::new(DraftAlign::Start, Some(script))?;
    f.press(KeyCode::F2);
    assert_eq!(f.log.borrow().results, vec![Ok(())]);
    Ok(())
}

#[test]
fn a_read_only_session_selects_but_refuses_edits_and_the_ime() -> TestResult {
    let script: Script =
        Box::new(|cx, draft| draft.begin_read_only(cx, "42 ms", DraftSelection::End));
    let mut f = Fixture::new(DraftAlign::Start, Some(script))?;
    f.press(KeyCode::F2);
    let editor = f.editor();
    f.ui.focus(&editor)?;
    frame(&mut f.ui);
    assert!(f.draft.is_read_only());
    f.ui.text_input("x")?;
    f.chord(KeyCode::primary(), KeyCode::KeyA);
    frame(&mut f.ui);
    assert_eq!(f.draft.text(), "42 ms");
    assert!(
        f.log.borrow().edits.is_empty(),
        "no on_edit in a read-only session"
    );
    assert!(
        f.ui.text_input_state().is_none(),
        "the IME gets no input state"
    );
    let semantics = f
        .editor_details()
        .semantics
        .expect("semantics while active");
    assert!(semantics.read_only);
    assert!(!semantics.actions.contains(&SemanticAction::SetValue));
    assert_eq!(
        f.editor_details().properties["selection"],
        InspectValue::Text("0..5".into())
    );
    Ok(())
}

#[test]
fn the_editor_is_hidden_and_schedules_nothing_at_rest() -> TestResult {
    let mut f = Fixture::new(DraftAlign::Start, None)?;
    let visible = |f: &Fixture| {
        f.ui.inspect()
            .components
            .iter()
            .find(|c| c.key == "editor")
            .map(|c| c.visible)
    };
    assert_eq!(visible(&f), Some(false));
    assert_eq!(f.ui.next_deadline(), None);
    f.press(KeyCode::Enter);
    assert_eq!(visible(&f), Some(true));
    assert!(
        f.ui.next_deadline().is_some(),
        "the caret blinks while editing"
    );
    let semantics = f
        .editor_details()
        .semantics
        .expect("semantics while active");
    assert_eq!(semantics.role, SemanticRole::TextInput);
    assert!(semantics.actions.contains(&SemanticAction::SetValue));
    f.press(KeyCode::Escape);
    assert_eq!(visible(&f), Some(false));
    assert_eq!(f.ui.next_deadline(), None);
    Ok(())
}

#[test]
fn tab_and_shift_tab_leave_the_row_and_focus_loss_saves() -> TestResult {
    let mut f = Fixture::new(DraftAlign::Start, None)?;
    f.press(KeyCode::Enter);
    f.ui.text_input("130")?;
    f.chord(KeyCode::ShiftLeft, KeyCode::Tab);
    assert!(f.focused("before"), "Shift+Tab skipped the row itself");
    assert_eq!(
        f.value.get()?,
        130.0,
        "focus_within_changed saved the draft"
    );
    assert!(!f.draft.is_active());

    f.ui.focus(&f.row.clone())?;
    f.press(KeyCode::Enter);
    f.ui.text_input("131")?;
    f.press(KeyCode::Tab);
    assert!(f.focused("after"));
    assert_eq!(f.value.get()?, 131.0);

    f.ui.focus(&f.row.clone())?;
    f.press(KeyCode::Enter);
    f.ui.text_input("132")?;
    f.ui.clear_focus();
    frame(&mut f.ui);
    assert_eq!(f.value.get()?, 132.0, "window focus loss saves too");
    Ok(())
}

#[test]
fn describing_the_editor_again_keeps_the_session_and_a_new_controller_ends_it() -> TestResult {
    let mut f = Fixture::new(DraftAlign::Start, None)?;
    f.press(KeyCode::Enter);
    f.ui.text_input("7")?;
    frame(&mut f.ui);
    let editor = f.editor();
    let row = f.row.clone();
    f.ui.reconcile_children(
        &row,
        vec![Element::new(
            "editor",
            DraftEditor::new(f.draft.clone()).align(DraftAlign::End),
        )],
    )?;
    frame(&mut f.ui);
    assert_eq!(f.editor(), editor, "reconciled, not remounted");
    assert!(f.draft.is_active());
    assert_eq!(f.draft.text(), "7", "the session survived the description");

    let other = DraftEditorController::<f64>::new();
    f.ui.reconcile_children(&row, vec![Element::new("editor", DraftEditor::new(other))])?;
    frame(&mut f.ui);
    assert!(
        !f.draft.is_active(),
        "switching controllers ended the old session"
    );
    Ok(())
}

#[test]
fn a_second_mounted_editor_on_one_controller_fails_its_update() -> TestResult {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(400.0, 200.0, 1.0)?)?;
    let draft = DraftEditorController::<()>::new();
    ui.mount_tree(Element::new(
        "root",
        Column::new([
            Element::new("first", DraftEditor::new(draft.clone())),
            Element::new("second", DraftEditor::new(draft)),
        ]),
    ))?;
    assert_eq!(ui.flush(ui.now()), Err(RuntimeError::DraftEditorBusy));
    Ok(())
}

#[test]
fn a_controller_serves_the_runtime_its_editor_mounted_in() -> TestResult {
    let draft = DraftEditorController::<f64>::new();
    let mut first = Runtime::new();
    first.set_viewport(Viewport::new(400.0, 200.0, 1.0)?)?;
    first.mount_tree(Element::new("editor", DraftEditor::new(draft.clone())))?;
    frame(&mut first);

    let foreign = draft.clone();
    let script: Script = Box::new(move |cx, _| foreign.begin(cx, "1", None, DraftSelection::End));
    let mut f = Fixture::new(DraftAlign::Start, Some(script))?;
    f.press(KeyCode::F2);
    assert_eq!(
        f.log.borrow().results,
        vec![Err(RuntimeError::ForeignRuntime)]
    );
    Ok(())
}

#[test]
fn end_alignment_places_a_short_draft_at_the_trailing_edge() -> TestResult {
    let caret = |align| -> Result<f32, Error> {
        let mut f = Fixture::new(align, None)?;
        f.press(KeyCode::Enter);
        f.press(KeyCode::End);
        let (_, state) = f.ui.text_input_state().ok_or("no input state")?;
        Ok(state.caret.x())
    };
    let start = caret(DraftAlign::Start)?;
    let end = caret(DraftAlign::End)?;
    assert!(end > start + 40.0, "{end} is not flush right of {start}");
    Ok(())
}

fn center(ui: &Runtime, id: &WidgetId) -> Result<Point, Error> {
    let [x, y, w, h] = ui.bounds(id)?.ok_or("unplaced")?.values();
    Ok(Point::new(x + w / 2.0, y + h / 2.0)?)
}

fn pointer(ui: &mut Runtime, point: Point, kind: PointerKind) -> Result<(), RuntimeError> {
    ui.pointer_input(PointerEvent::new(point, kind)).map(|_| ())
}

#[test]
fn a_busy_editor_takes_the_controller_over_once_the_first_unmounts() -> TestResult {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(400.0, 200.0, 1.0)?)?;
    let draft = DraftEditorController::<()>::new();
    let root = ui.mount_tree(Element::new(
        "root",
        Column::new([
            Element::new("first", DraftEditor::new(draft.clone())),
            Element::new("second", DraftEditor::new(draft.clone())),
        ]),
    ))?;
    assert_eq!(ui.flush(ui.now()), Err(RuntimeError::DraftEditorBusy));
    ui.reconcile_children(&root, vec![Element::new("second", DraftEditor::new(draft))])?;
    ui.flush(ui.now())?;
    let busy = ui
        .inspect()
        .components
        .into_iter()
        .find(|c| c.key == "second")
        .map(|c| c.details.properties["busy"].clone());
    assert_eq!(busy, Some(InspectValue::Bool(false)));
    Ok(())
}

#[test]
fn a_renamed_editor_and_a_controller_switched_back_keep_working() -> TestResult {
    let mut f = Fixture::new(DraftAlign::Start, None)?;
    let row = f.row.clone();
    f.ui.reconcile_children(
        &row,
        vec![Element::new("renamed", DraftEditor::new(f.draft.clone()))],
    )?;
    frame(&mut f.ui);
    f.press(KeyCode::Enter);
    assert!(
        f.draft.is_active(),
        "the renamed editor owns the controller"
    );
    assert_eq!(f.ui.focused(), find(&f.ui, &f.row, "renamed"));
    f.press(KeyCode::Escape);

    let other = DraftEditorController::<f64>::new();
    f.ui.reconcile_children(&row, vec![Element::new("renamed", DraftEditor::new(other))])?;
    frame(&mut f.ui);
    f.ui.reconcile_children(
        &row,
        vec![Element::new("renamed", DraftEditor::new(f.draft.clone()))],
    )?;
    frame(&mut f.ui);
    f.press(KeyCode::Enter);
    assert!(
        f.draft.is_active(),
        "switching back claimed the controller again"
    );
    Ok(())
}

#[test]
fn focus_loss_runs_the_callback_once_and_the_owner_hides_the_editor() -> TestResult {
    let mut f = Fixture::new(DraftAlign::Start, None)?;
    f.press(KeyCode::Enter);
    f.press(KeyCode::Enter);
    assert_eq!(
        f.log.borrow().focus_losses,
        0,
        "not after the owner ended the session"
    );

    f.press(KeyCode::Enter);
    f.ui.text_input("140")?;
    f.press(KeyCode::Tab);
    assert_eq!(f.log.borrow().focus_losses, 1);
    let visible =
        f.ui.inspect()
            .components
            .into_iter()
            .find(|c| c.key == "editor")
            .map(|c| c.visible);
    assert_eq!(
        visible,
        Some(false),
        "the owner was re-evaluated and hid it"
    );

    f.ui.focus(&f.row.clone())?;
    f.press(KeyCode::Enter);
    f.ui.text_input("150")?;
    let after = find(&f.ui, &f.root, "after").ok_or("no button")?;
    let point = center(&f.ui, &after)?;
    pointer(&mut f.ui, point, PointerKind::Press(PointerButton::Primary))?;
    pointer(
        &mut f.ui,
        point,
        PointerKind::Release(PointerButton::Primary),
    )?;
    frame(&mut f.ui);
    assert_eq!(f.log.borrow().focus_losses, 2, "a press elsewhere");
    assert_eq!(f.value.get()?, 150.0);
    Ok(())
}

#[test]
fn removing_the_row_cancels_without_the_focus_loss_callback() -> TestResult {
    let mut f = Fixture::new(DraftAlign::Start, None)?;
    f.press(KeyCode::Enter);
    f.ui.text_input("7")?;
    let root = f.root.clone();
    f.ui.reconcile_children(
        &root,
        vec![
            Element::new("before", Button::new("Before", |_| Ok(()))),
            Element::new("after", Button::new("After", |_| Ok(()))),
        ],
    )?;
    frame(&mut f.ui);
    assert_eq!(f.log.borrow().focus_losses, 0);
    assert_eq!(f.value.get()?, 120.0, "removal saves nothing");
    assert!(!f.draft.is_active(), "unmounting ended the session");
    Ok(())
}

#[test]
fn escape_during_a_drag_selection_stays_in_the_editor() -> TestResult {
    let mut f = Fixture::new(DraftAlign::Start, None)?;
    f.press(KeyCode::Enter);
    let point = center(&f.ui, &f.editor())?;
    pointer(&mut f.ui, point, PointerKind::Press(PointerButton::Primary))?;
    pointer(
        &mut f.ui,
        Point::new(point.x() - 20.0, point.y())?,
        PointerKind::Move,
    )?;
    f.log.borrow_mut().keys.clear();
    f.press(KeyCode::Escape);
    assert!(f.draft.is_active());
    assert!(!f.log.borrow().keys.contains(&KeyCode::Escape));
    pointer(
        &mut f.ui,
        point,
        PointerKind::Release(PointerButton::Primary),
    )?;
    Ok(())
}

#[test]
fn an_on_edit_error_propagates_and_the_edit_is_kept() -> TestResult {
    let mut f = Fixture::new(DraftAlign::Start, None)?;
    f.draft.on_edit(|_, _, _| Err(RuntimeError::Exhausted));
    f.press(KeyCode::Enter);
    assert_eq!(f.ui.text_input("5"), Err(RuntimeError::Exhausted));
    assert_eq!(f.draft.text(), "5");
    Ok(())
}

#[test]
fn the_callbacks_may_call_the_controller() -> TestResult {
    let mut f = Fixture::new(DraftAlign::Start, None)?;
    let draft = f.draft.clone();
    f.draft.on_edit(move |_, text, payload| {
        assert!(draft.is_active());
        assert_eq!(draft.text(), text);
        assert_eq!(draft.payload(), payload);
        Ok(())
    });
    f.press(KeyCode::Enter);
    f.ui.text_input("6")?;
    assert_eq!(f.draft.text(), "6");
    Ok(())
}

#[derive(Clone, Default)]
struct MemoryClipboard(Rc<RefCell<String>>);
impl Clipboard for MemoryClipboard {
    fn read_text(&mut self) -> Result<String, ClipboardError> {
        Ok(self.0.borrow().clone())
    }
    fn write_text(&mut self, text: &str) -> Result<(), ClipboardError> {
        text.clone_into(&mut self.0.borrow_mut());
        Ok(())
    }
}

#[test]
fn paste_cut_undo_and_redo_run_on_edit() -> TestResult {
    let mut f = Fixture::new(DraftAlign::Start, None)?;
    let clipboard = MemoryClipboard::default();
    "77".clone_into(&mut clipboard.0.borrow_mut());
    f.ui.set_clipboard(clipboard.clone())?;
    f.press(KeyCode::Enter);
    let primary = KeyCode::primary();
    f.chord(primary, KeyCode::KeyV);
    assert_eq!(f.draft.text(), "77");
    f.chord(primary, KeyCode::KeyA);
    f.chord(primary, KeyCode::KeyX);
    assert_eq!(f.draft.text(), "");
    assert_eq!(clipboard.0.borrow().as_str(), "77");
    f.chord(primary, KeyCode::KeyZ);
    f.ui.key_input(KeyCode::ShiftLeft, KeyAction::Press)?;
    f.chord(primary, KeyCode::KeyZ);
    f.ui.key_input(KeyCode::ShiftLeft, KeyAction::Release)?;
    let texts: Vec<_> = f
        .log
        .borrow()
        .edits
        .iter()
        .map(|(text, _)| text.clone())
        .collect();
    assert_eq!(texts, ["77", "", "77", ""], "paste, cut, undo, redo");
    Ok(())
}

#[test]
fn a_failing_clipboard_does_not_swallow_a_secondary_press_or_the_menu_key() -> TestResult {
    struct Failing;
    impl Clipboard for Failing {
        fn read_text(&mut self) -> Result<String, ClipboardError> {
            Err(ClipboardError::Failed("boom".into()))
        }
        fn write_text(&mut self, _: &str) -> Result<(), ClipboardError> {
            Err(ClipboardError::Failed("boom".into()))
        }
    }
    let mut f = Fixture::new(DraftAlign::Start, None)?;
    f.ui.set_clipboard(Failing)?;
    f.press(KeyCode::Enter);
    let point = center(&f.ui, &f.editor())?;
    pointer(
        &mut f.ui,
        point,
        PointerKind::Press(PointerButton::Secondary),
    )?;
    pointer(
        &mut f.ui,
        point,
        PointerKind::Release(PointerButton::Secondary),
    )?;
    f.ui.key_input(KeyCode::ContextMenu, KeyAction::Press)?;
    f.ui.key_input(KeyCode::ContextMenu, KeyAction::Release)?;
    frame(&mut f.ui);
    assert!(f.draft.is_active());
    Ok(())
}

#[test]
fn a_clipboard_service_may_read_and_end_the_draft_while_pasting() -> TestResult {
    struct Reentrant(DraftEditorController<f64>);
    impl Clipboard for Reentrant {
        fn read_text(&mut self) -> Result<String, ClipboardError> {
            assert!(self.0.is_active());
            assert_eq!(self.0.text(), "120.0", "the draft before the paste");
            self.0.end();
            Ok("9".into())
        }
        fn write_text(&mut self, _: &str) -> Result<(), ClipboardError> {
            Ok(())
        }
    }
    let mut f = Fixture::new(DraftAlign::Start, None)?;
    f.ui.set_clipboard(Reentrant(f.draft.clone()))?;
    f.press(KeyCode::Enter);
    f.chord(KeyCode::primary(), KeyCode::KeyV);
    assert!(!f.draft.is_active(), "the end took effect after the paste");
    Ok(())
}

#[test]
fn replace_works_in_a_read_only_session() -> TestResult {
    let script: Script = Box::new(|cx, draft| {
        draft.begin_read_only(cx, "1 ms", DraftSelection::End)?;
        assert_eq!(draft.replace(cx, "2 ms", None), Ok(true));
        assert_eq!(draft.text(), "2 ms");
        Ok(())
    });
    let mut f = Fixture::new(DraftAlign::Start, Some(script))?;
    f.press(KeyCode::F2);
    assert_eq!(f.log.borrow().results, vec![Ok(())]);
    Ok(())
}

/// An owner that never invalidates itself: the draft editor's own
/// invalidation must show and hide it.
struct Passive {
    draft: DraftEditorController,
    editor: Option<WidgetId>,
    losses: Rc<RefCell<usize>>,
    children: Vec<Element>,
}

impl Component for Passive {
    fn take_children(&mut self) -> Vec<Element> {
        std::mem::take(&mut self.children)
    }
    fn focusable(&self) -> bool {
        !self.draft.is_active()
    }
    fn child_visible(&self, _key: &str) -> bool {
        self.draft.is_active()
    }
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        self.editor = cx.children().first().map(|child| child.id().clone());
        Ok(())
    }
    fn focus_within_changed(
        &mut self,
        _: &mut EventContext,
        within: bool,
    ) -> Result<(), RuntimeError> {
        if !within {
            *self.losses.borrow_mut() += 1;
        }
        Ok(())
    }
    fn keyboard_event(
        &mut self,
        cx: &mut EventContext,
        event: KeyEvent,
        _: &KeyboardState,
    ) -> Result<InputOutcome, RuntimeError> {
        if event.transition != KeyTransition::Pressed {
            return Ok(InputOutcome::Ignored);
        }
        match event.code {
            KeyCode::Enter if !self.draft.is_active() => {
                self.draft.begin(cx, "draft", None, DraftSelection::All)?;
                if let Some(editor) = &self.editor {
                    cx.request_focus_for(editor)?;
                }
            }
            KeyCode::Escape if self.draft.is_active() => self.draft.end(),
            _ => return Ok(InputOutcome::Ignored),
        }
        Ok(InputOutcome::Handled)
    }
}

#[test]
fn the_editor_shows_and_hides_itself_for_an_owner_that_never_invalidates() -> TestResult {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(400.0, 200.0, 1.0)?)?;
    let draft = DraftEditorController::new();
    let owner = ui.mount_tree(Element::new(
        "owner",
        Passive {
            draft: draft.clone(),
            editor: None,
            losses: Rc::default(),
            children: vec![Element::new("editor", DraftEditor::new(draft.clone()))],
        },
    ))?;
    frame(&mut ui);
    ui.focus(&owner)?;
    let visible = |ui: &Runtime| {
        ui.inspect()
            .components
            .into_iter()
            .find(|c| c.key == "editor")
            .map(|c| c.visible)
    };
    for round in 0..2 {
        key(&mut ui, KeyCode::Enter);
        frame(&mut ui);
        assert_eq!(visible(&ui), Some(true), "round {round}: begin showed it");
        assert_eq!(ui.focused(), find(&ui, &owner, "editor"), "round {round}");
        key(&mut ui, KeyCode::Escape);
        frame(&mut ui);
        assert_eq!(visible(&ui), Some(false), "round {round}: end hid it");
        ui.focus(&owner)?;
    }
    Ok(())
}

/// Shows its children while a signal is true.
struct Gate {
    shown: ReadSignal<bool>,
    show: bool,
    children: Vec<Element>,
}

impl Component for Gate {
    fn take_children(&mut self) -> Vec<Element> {
        std::mem::take(&mut self.children)
    }
    fn child_visible(&self, _key: &str) -> bool {
        self.show
    }
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        // Reading subscribes; the runtime re-reads `child_visible` after an
        // update.
        self.show = cx.read(&self.shown)?;
        Ok(())
    }
}

#[test]
fn hiding_the_row_keeps_the_session_and_runs_no_focus_loss_callback() -> TestResult {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(400.0, 200.0, 1.0)?)?;
    let shown = ui.signal(true);
    let draft = DraftEditorController::new();
    let losses = Rc::new(RefCell::new(0));
    let root = ui.mount_tree(Element::new(
        "gate",
        Gate {
            shown: shown.read_only(),
            show: true,
            children: vec![Element::new(
                "owner",
                Passive {
                    draft: draft.clone(),
                    editor: None,
                    losses: losses.clone(),
                    children: vec![Element::new("editor", DraftEditor::new(draft.clone()))],
                },
            )],
        },
    ))?;
    frame(&mut ui);
    let owner = find(&ui, &root, "owner").ok_or("no owner")?;
    ui.focus(&owner)?;
    key(&mut ui, KeyCode::Enter);
    frame(&mut ui);
    assert!(draft.is_active());
    shown.set(false)?;
    frame(&mut ui);
    let hidden = ui
        .inspect()
        .components
        .into_iter()
        .find(|c| c.key == "editor")
        .map(|c| c.visible);
    assert_eq!(hidden, Some(false), "the gate hid the row");
    assert_eq!(
        *losses.borrow(),
        0,
        "hiding cancels, it does not report focus loss"
    );
    assert!(draft.is_active(), "hiding keeps the session");
    assert_eq!(draft.text(), "draft");
    Ok(())
}

/// Records focus-within notifications around a row.
struct Outer {
    log: Rc<RefCell<Vec<bool>>>,
    children: Vec<Element>,
}

impl Component for Outer {
    fn take_children(&mut self) -> Vec<Element> {
        std::mem::take(&mut self.children)
    }
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
    fn focus_within_changed(
        &mut self,
        _: &mut EventContext,
        within: bool,
    ) -> Result<(), RuntimeError> {
        self.log.borrow_mut().push(within);
        Ok(())
    }
}

#[test]
fn committing_and_cancelling_tell_an_outer_container_nothing() -> TestResult {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(400.0, 200.0, 1.0)?)?;
    let value = ui.signal(120.0);
    let draft = DraftEditorController::new();
    let told = Rc::new(RefCell::new(Vec::new()));
    let row = Row {
        value: value.clone(),
        draft: draft.clone(),
        editor: None,
        log: Rc::default(),
        script: None,
        children: vec![Element::new("editor", DraftEditor::new(draft.clone()))],
    };
    let root = ui.mount_tree(Element::new(
        "outer",
        Outer {
            log: told.clone(),
            children: vec![Element::new("row", row)],
        },
    ))?;
    frame(&mut ui);
    let row = find(&ui, &root, "row").ok_or("no row")?;
    ui.focus(&row)?;
    frame(&mut ui);
    assert_eq!(*told.borrow(), [true]);
    for close in [KeyCode::Enter, KeyCode::Escape] {
        key(&mut ui, KeyCode::Enter);
        frame(&mut ui);
        assert!(draft.is_active());
        key(&mut ui, close);
        frame(&mut ui);
        assert!(!draft.is_active());
        assert_eq!(ui.focused(), Some(row.clone()));
    }
    assert_eq!(
        *told.borrow(),
        [true],
        "focus never left the outer container"
    );
    Ok(())
}

/// An owner that keeps its editor visible and ends the session on Escape
/// without taking focus back itself.
struct Visible {
    draft: DraftEditorController,
    editor: Option<WidgetId>,
    children: Vec<Element>,
}

impl Component for Visible {
    fn take_children(&mut self) -> Vec<Element> {
        std::mem::take(&mut self.children)
    }
    fn focusable(&self) -> bool {
        !self.draft.is_active()
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
        if event.transition != KeyTransition::Pressed {
            return Ok(InputOutcome::Ignored);
        }
        match event.code {
            KeyCode::Enter if !self.draft.is_active() => {
                self.draft.begin(cx, "draft", None, DraftSelection::All)?;
                if let Some(editor) = &self.editor {
                    cx.request_focus_for(editor)?;
                }
            }
            KeyCode::Escape if self.draft.is_active() => self.draft.end(),
            _ => return Ok(InputOutcome::Ignored),
        }
        Ok(InputOutcome::Handled)
    }
}

#[test]
fn an_ended_editor_that_stays_visible_hands_focus_back_to_its_owner() -> TestResult {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(400.0, 200.0, 1.0)?)?;
    let draft = DraftEditorController::new();
    let root = ui.mount_tree(Element::new(
        "root",
        Column::new([
            Element::new("first", Button::new("First", |_| Ok(()))),
            Element::new(
                "owner",
                Visible {
                    draft: draft.clone(),
                    editor: None,
                    children: vec![Element::new("editor", DraftEditor::new(draft.clone()))],
                },
            ),
        ]),
    ))?;
    frame(&mut ui);
    let owner = find(&ui, &root, "owner").ok_or("no owner")?;
    ui.focus(&owner)?;
    key(&mut ui, KeyCode::Enter);
    frame(&mut ui);
    assert_eq!(ui.focused(), find(&ui, &owner, "editor"));
    key(&mut ui, KeyCode::Escape);
    frame(&mut ui);
    assert!(!draft.is_active());
    assert_eq!(ui.focused(), Some(owner), "not the window's first control");
    Ok(())
}

#[test]
fn a_failed_replacement_keeps_the_composition() -> TestResult {
    let script: Script = Box::new(|cx, draft| {
        // Accepted text that cannot be laid out: wider than the raster limit.
        draft.replace(cx, &"W".repeat(4000), None).map(|_| ())
    });
    let mut f = Fixture::new(DraftAlign::Start, Some(script))?;
    f.press(KeyCode::Enter);
    f.ui.text_preedit(&TextPreedit::new("x", None)?)?;
    frame(&mut f.ui);
    // The editor keeps every key while composing; the caption, left of the
    // editor's slot, runs the script by pointer.
    let [x, y, _, h] = f.ui.bounds(&f.row)?.ok_or("unplaced")?.values();
    let caption = Point::new(x + 10.0, y + h / 2.0)?;
    pointer(
        &mut f.ui,
        caption,
        PointerKind::Press(PointerButton::Primary),
    )?;
    frame(&mut f.ui);
    assert!(matches!(
        f.log.borrow().results[..],
        [Err(RuntimeError::Text(TextError::TooLarge(
            TextLimit::RasterSide
        )))]
    ));
    assert_eq!(f.draft.text(), "120.0");
    assert_eq!(
        f.editor_details().properties["composing"],
        InspectValue::Bool(true),
        "the draft is unchanged, composition included"
    );
    Ok(())
}
