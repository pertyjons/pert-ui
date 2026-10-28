//! A form's Revert (contracts.md, "Draft policy", rule 3, and
//! `FormFields::discard_drafts`): writing the value an editor already shows
//! keeps its draft, and the form discards the draft of every field it shows.
use crate::support;
use pert_ui::testing::Driver;
use pert_ui::*;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
use support::{key, press, widget};

/// Each `discard_drafts` result a `FormDiscarder` recorded.
type Results = Rc<RefCell<Vec<Result<usize, RuntimeError>>>>;

/// Each `discard_drafts` result, with the summary read right after the call.
type Calls = Rc<RefCell<Vec<(Result<usize, RuntimeError>, FormSummary)>>>;

/// Three flushes, so reports delivered after settling have arrived.
fn settle(ui: &mut Runtime) {
    for _ in 0..3 {
        support::settle(ui);
    }
}

fn inspection(ui: &Runtime, id: &WidgetId) -> ComponentInspection {
    let inspection = ui.inspection_id(id).unwrap();
    ui.inspect()
        .components
        .into_iter()
        .find(|c| c.id == inspection)
        .unwrap()
}

/// Whether the number editor `id` shows a draft it refuses. Its semantics
/// would also be invalid while its `Field` shows an error status.
fn invalid(ui: &Runtime, id: &WidgetId) -> bool {
    match inspection(ui, id).details.properties.get("valid") {
        Some(InspectValue::Bool(valid)) => !valid,
        other => panic!("valid is not a flag: {other:?}"),
    }
}

/// The text the editor `id` shows: its draft, or the bound value at rest.
fn shown(ui: &Runtime, id: &WidgetId) -> String {
    match inspection(ui, id).details.properties.get("text") {
        Some(InspectValue::Text(text)) => text.clone(),
        other => panic!("text is not text: {other:?}"),
    }
}

fn property(ui: &Runtime, id: &WidgetId, name: &str) -> Option<InspectValue> {
    inspection(ui, id).details.properties.get(name).cloned()
}

fn modified(form: &FormFields) -> usize {
    form.summary().get().unwrap().modified()
}

fn status(handle: &FieldHandle) -> FieldStatus {
    handle.status().get().unwrap()
}

/// Focuses `id`, selects its text and types `text` over it.
fn type_over(ui: &mut Runtime, id: &WidgetId, text: &str) {
    ui.focus(id).unwrap();
    settle(ui);
    ui.text_command(id, TextCommand::SelectAll).unwrap();
    ui.text_input(text).unwrap();
    settle(ui);
}

/// An `IntegerInput` with `range(1, 99)` keyed "number", publishing its
/// refusals as the handle's status and its kept draft as the handle's mark.
fn number(value: &Signal<i64>, handle: &FieldHandle) -> Element {
    keyed_number(value, handle, "number")
}

/// [`number`] keyed `key`.
fn keyed_number(value: &Signal<i64>, handle: &FieldHandle, key: &str) -> Element {
    let (status, mark) = (handle.clone(), handle.clone());
    IntegerInput::new(value)
        .range(1, 99)
        .unwrap()
        .on_rejection(move |_, refusal| match refusal {
            Some(refusal) => status.set(FieldStatus::error(refusal.to_string())),
            None => status.clear().map(|_| ()),
        })
        .on_modified(move |_, modified| mark.set_modified(modified).map(|_| ()))
        .key(key)
}

/// A trimming `TextField` keyed `key`, which therefore holds a draft, and
/// reports its kept draft to `mark` when one is given.
fn text(
    value: &Signal<String>,
    enabled: &Signal<bool>,
    mark: Option<&FieldHandle>,
    key: &str,
) -> Element {
    let field = TextField::new(value)
        .normalize(|text| Ok(text.trim().to_owned()))
        .enabled(enabled);
    match mark.cloned() {
        Some(mark) => field.on_modified(move |_, modified| mark.set_modified(modified).map(|_| ())),
        None => field,
    }
    .key(key)
}

/// Test 1's form: an `IntegerInput` bound to 7 and a normalizing `TextField`
/// bound to "Name", with a Write button that writes both applied values and
/// a Revert button that writes them and calls `discard_drafts`.
struct Case {
    form: FormFields,
    number: Signal<i64>,
    text: Signal<String>,
    number_field: FieldHandle,
    text_field: FieldHandle,
    text_enabled: Signal<bool>,
    calls: Calls,
}
impl Case {
    /// Registers the two fields, first in the form.
    fn new(ui: &Runtime) -> Self {
        let form = FormFields::new(ui);
        Self {
            number_field: form.field("number").unwrap(),
            text_field: form.field("text").unwrap(),
            form,
            number: ui.signal(7),
            text: ui.signal("Name".to_owned()),
            text_enabled: ui.signal(true),
            calls: Calls::default(),
        }
    }
    fn fields(&self) -> Vec<Element> {
        vec![
            Field::new(number(&self.number, &self.number_field))
                .form(&self.number_field)
                .key("number-field"),
            Field::new(text(
                &self.text,
                &self.text_enabled,
                Some(&self.text_field),
                "text",
            ))
            .form(&self.text_field)
            .key("text-field"),
        ]
    }
    fn buttons(&self) -> Vec<Element> {
        let (number, text) = (self.number.clone(), self.text.clone());
        let write = Button::new("Write", move |_| {
            number.set(7)?;
            text.set("Name".to_owned())
        });
        let (number, text) = (self.number.clone(), self.text.clone());
        let (form, calls) = (self.form.clone(), self.calls.clone());
        let revert = Button::new("Revert", move |cx| {
            number.set(7)?;
            text.set("Name".to_owned())?;
            let result = form.discard_drafts(cx);
            calls.borrow_mut().push((result, form.summary().get()?));
            Ok(())
        });
        vec![write.key("write"), revert.key("revert")]
    }
    /// Mounts the fields, `extra` and the buttons in one column.
    fn mount(&self, ui: &mut Runtime, extra: Vec<Element>) -> WidgetId {
        ui.set_viewport(Viewport::new(400.0, 600.0, 1.0).unwrap())
            .unwrap();
        let children: Vec<Element> = self
            .fields()
            .into_iter()
            .chain(extra)
            .chain(self.buttons())
            .collect();
        let root = ui.mount_tree(Column::new(children).key("root")).unwrap();
        settle(ui);
        root
    }
    /// The number field refuses 150 and keeps it; the text field keeps the
    /// draft "Other" while disabled.
    fn drafts(&self, ui: &mut Runtime, root: &WidgetId) {
        let number = widget(ui, root, "number");
        type_over(ui, &number, "150");
        press(ui, KeyCode::Enter);
        assert!(invalid(ui, &number));
        assert!(status(&self.number_field).is_error());
        let text = widget(ui, root, "text");
        type_over(ui, &text, "Other");
        self.text_enabled.set(false).unwrap();
        settle(ui);
        assert_eq!(shown(ui, &text), "Other");
        assert_eq!(self.text.get().unwrap(), "Name");
    }
}

fn click(ui: &mut Runtime, root: &WidgetId, key: &str) {
    let button = widget(ui, root, key);
    Driver::new(ui).click(&button).unwrap();
    settle(ui);
}

/// New-behavior test 1, the R2-2 reproduction. Named assertions: after an
/// equal write both drafts remain with `modified() == 2`; Revert's call
/// returns `Ok(2)` and the summary read in its callback still counts 2
/// (the call changes no mark); after settling neither draft remains, the
/// bindings are unchanged, `modified() == 0` and the refused field's status
/// is `Normal` from its `on_rejection(None)`. With a changed value the write
/// ends the draft first: the call still returns `Ok(2)`, but the status the
/// application published stays `Error` until Escape. Mutations checked:
/// queueing only fields whose status is an error (the text draft remains,
/// `modified() == 1`); clearing each handle's mark in the call (the summary
/// read in the callback counts 0); returning the count without queueing
/// (both drafts remain).
#[test]
fn revert_discards_what_its_equal_write_keeps() {
    let mut ui = Runtime::new();
    let case = Case::new(&ui);
    let root = case.mount(&mut ui, Vec::new());
    case.drafts(&mut ui, &root);
    let (number, text) = (widget(&ui, &root, "number"), widget(&ui, &root, "text"));
    assert_eq!(modified(&case.form), 2);

    // Rule 3: writing the values the fields show keeps both drafts.
    click(&mut ui, &root, "write");
    assert!(invalid(&ui, &number), "the refused draft stays");
    assert_eq!(shown(&ui, &text), "Other", "the kept draft stays");
    assert_eq!(modified(&case.form), 2);

    click(&mut ui, &root, "revert");
    {
        let calls = case.calls.borrow();
        let [(result, summary)] = calls.as_slice() else {
            panic!("one call: {calls:?}");
        };
        assert!(matches!(result, Ok(2)), "{result:?}");
        assert_eq!(summary.modified(), 2, "the call writes no mark");
    }
    assert!(!invalid(&ui, &number));
    assert_eq!(shown(&ui, &number), "7");
    assert_eq!(shown(&ui, &text), "Name");
    assert_eq!(case.number.get().unwrap(), 7);
    assert_eq!(case.text.get().unwrap(), "Name");
    assert_eq!(modified(&case.form), 0);
    assert_eq!(status(&case.number_field), FieldStatus::Normal);

    // A changed value: the write ends the draft before the request arrives.
    let mut ui = Runtime::new();
    let case = Case::new(&ui);
    let root = case.mount(&mut ui, Vec::new());
    let number = widget(&ui, &root, "number");
    type_over(&mut ui, &number, "8");
    press(&mut ui, KeyCode::Enter);
    assert_eq!(case.number.get().unwrap(), 8);
    type_over(&mut ui, &number, "150");
    press(&mut ui, KeyCode::Enter);
    assert!(status(&case.number_field).is_error());
    click(&mut ui, &root, "revert");
    assert!(matches!(case.calls.borrow()[0].0, Ok(2)));
    assert_eq!(case.number.get().unwrap(), 7);
    assert!(!invalid(&ui, &number));
    assert_eq!(shown(&ui, &number), "7");
    assert_eq!(property(&ui, &number, "validation_error"), None);
    assert_eq!(modified(&case.form), 0);
    assert!(
        status(&case.number_field).is_error(),
        "the write ended the draft, so the request found none and no None came"
    );
    ui.focus(&number).unwrap();
    settle(&mut ui);
    press(&mut ui, KeyCode::Escape);
    assert_eq!(status(&case.number_field), FieldStatus::Normal);
}

/// New-behavior test 2. Besides test 1's fields: an unchanged
/// `IntegerInput` (no draft) and a `TextField` holding a draft with no
/// `on_modified`, both counted; a field never shown, a field described with
/// `status_signal` and a field whose control is hidden before the call, all
/// skipped. Revert runs in an input call, and test 1's number field is
/// hidden before the flush. Named assertions: the call returns `Ok(4)`; the
/// flush succeeds; both text drafts, the unreported one included, are
/// discarded; the hidden number field lost its draft to hiding and its
/// request was dropped. Mutations checked: selecting fields by their
/// `modified` mark (`Ok(2)`, the unreported draft remains); counting
/// registered fields (7); queueing hidden controls (5); skipping only empty
/// cells and leaving visibility to delivery (5).
#[test]
fn discard_reaches_every_shown_field_whatever_the_marks_say() {
    let mut ui = Runtime::new();
    let case = Case::new(&ui);
    let unchanged = ui.signal(3_i64);
    let (unreported, unreported_enabled) = (ui.signal("Kept".to_owned()), ui.signal(true));
    let hidden = ui.signal(String::new());
    let fields = (
        case.form.field("unchanged").unwrap(),
        case.form.field("unreported").unwrap(),
        case.form.field("unshown").unwrap(),
        case.form.field("status-only").unwrap(),
        case.form.field("hidden").unwrap(),
    );
    let enabled = ui.signal(true);
    let extra = vec![
        Field::new(IntegerInput::new(&unchanged).key("unchanged"))
            .form(&fields.0)
            .key("unchanged-field"),
        Field::new(text(&unreported, &unreported_enabled, None, "unreported"))
            .form(&fields.1)
            .key("unreported-field"),
        Field::new(TextField::new(ui.signal(String::new())).key("status-only"))
            .status_signal(fields.3.status())
            .key("status-only-field"),
        Field::new(text(&hidden, &enabled, None, "hidden"))
            .form(&fields.4)
            .key("hidden-field"),
    ];
    let root = case.mount(&mut ui, extra);
    case.drafts(&mut ui, &root);
    let unreported_id = widget(&ui, &root, "unreported");
    type_over(&mut ui, &unreported_id, "Typed");
    unreported_enabled.set(false).unwrap();
    settle(&mut ui);
    assert_eq!(shown(&ui, &unreported_id), "Typed");
    let hidden_id = widget(&ui, &root, "hidden");
    ui.set_visible(&hidden_id, false).unwrap();
    settle(&mut ui);
    assert_eq!(modified(&case.form), 2, "the unreported draft has no mark");

    let revert = widget(&ui, &root, "revert");
    ui.focus(&revert).unwrap();
    settle(&mut ui);
    key(&mut ui, KeyCode::Space);
    assert!(
        matches!(case.calls.borrow().as_slice(), [(Ok(4), _)]),
        "{:?}",
        case.calls.borrow()
    );
    let number = widget(&ui, &root, "number");
    ui.set_visible(&number, false).unwrap();
    ui.flush(ui.now())
        .expect("a dropped request and an editor without a draft are no errors");
    settle(&mut ui);
    assert_eq!(shown(&ui, &widget(&ui, &root, "text")), "Name");
    assert_eq!(shown(&ui, &unreported_id), "Kept", "the unreported draft");
    assert_eq!(
        modified(&case.form),
        0,
        "hiding ended the number field's draft"
    );
    ui.set_visible(&number, true).unwrap();
    settle(&mut ui);
    assert!(!invalid(&ui, &number));
    assert_eq!(case.number.get().unwrap(), 7);
}

/// New-behavior test 3. Field A, a `TextField` whose `on_rejection` fails
/// on its first `None`, holds a reported refusal; field B, registered after
/// it, test 1's refused number field. Named assertions: the flush that
/// delivers Revert's requests returns A's error with A's draft discarded and
/// B's still there; the next flush discards B's; A's failed `None` is
/// reported at its next Escape. Mutation checked: queueing in reverse
/// registration order (B's draft is gone after the failing flush).
#[test]
fn a_failing_target_leaves_the_later_requests_queued() {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(400.0, 400.0, 1.0).unwrap())
        .unwrap();
    let form = FormFields::new(&ui);
    let (a, b) = (form.field("a").unwrap(), form.field("b").unwrap());
    let (label, count) = (ui.signal("good".to_owned()), ui.signal(7_i64));
    let rejections: Rc<RefCell<Vec<Option<String>>>> = Rc::default();
    let (log, fail) = (rejections.clone(), Rc::new(Cell::new(true)));
    let field_a = TextField::new(&label)
        .normalize(|text| {
            if text.contains('!') {
                Err(InputRejection::new("No exclamations"))
            } else {
                Ok(text)
            }
        })
        .on_rejection(move |_, refusal| {
            if refusal.is_none() && fail.replace(false) {
                return Err(ApplicationError::new(std::io::Error::other("clear failed")).into());
            }
            log.borrow_mut().push(refusal.map(ToString::to_string));
            Ok(())
        })
        .key("a");
    let (drafts, calls) = (form.clone(), Calls::default());
    let record = calls.clone();
    let revert = Button::new("Revert", move |cx| {
        let result = drafts.discard_drafts(cx);
        record.borrow_mut().push((result, drafts.summary().get()?));
        Ok(())
    });
    let root = ui
        .mount_tree(
            Column::new([
                Field::new(field_a).form(&a).key("a-field"),
                Field::new(number(&count, &b)).form(&b).key("b-field"),
                revert.key("revert"),
            ])
            .key("root"),
        )
        .unwrap();
    settle(&mut ui);
    let (a_id, b_id) = (widget(&ui, &root, "a"), widget(&ui, &root, "number"));
    type_over(&mut ui, &a_id, "bad!");
    press(&mut ui, KeyCode::Enter);
    type_over(&mut ui, &b_id, "150");
    press(&mut ui, KeyCode::Enter);
    assert_eq!(shown(&ui, &a_id), "bad!");
    assert!(invalid(&ui, &b_id));
    assert!(matches!(rejections.borrow().last(), Some(Some(_))));

    let revert = widget(&ui, &root, "revert");
    ui.focus(&revert).unwrap();
    settle(&mut ui);
    key(&mut ui, KeyCode::Space);
    assert!(matches!(calls.borrow().as_slice(), [(Ok(2), _)]));
    let failed = ui.flush(ui.now());
    assert!(
        failed
            .as_ref()
            .is_err_and(|error| error.to_string().contains("clear failed")),
        "{failed:?}"
    );
    assert_eq!(shown(&ui, &a_id), "good", "A's draft is discarded");
    assert!(invalid(&ui, &b_id), "B's request is still queued");
    ui.flush(ui.now()).unwrap();
    assert!(!invalid(&ui, &b_id));
    assert_eq!(shown(&ui, &b_id), "7");
    assert!(matches!(rejections.borrow().last(), Some(Some(_))));
    ui.focus(&a_id).unwrap();
    settle(&mut ui);
    press(&mut ui, KeyCode::Escape);
    assert_eq!(
        rejections.borrow().last(),
        Some(&None),
        "the pending None is reported"
    );
}

/// Calls `discard_drafts` on each form from its own delivery once `go`
/// turns true, so an editor can keep focus while it runs.
struct FormDiscarder {
    forms: Vec<FormFields>,
    go: Signal<bool>,
    results: Results,
    sent: bool,
    notifier: Option<Notifier>,
}
impl FormDiscarder {
    fn new(forms: Vec<FormFields>, go: &Signal<bool>) -> (Self, Results) {
        let results = Rc::default();
        let discarder = Self {
            forms,
            go: go.clone(),
            results: Rc::clone(&results),
            sent: false,
            notifier: None,
        };
        (discarder, results)
    }
}
impl Component for FormDiscarder {
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        let notifier = self.notifier.get_or_insert_with(|| cx.notifier()).clone();
        if cx.read(&self.go)? && !self.sent {
            notifier.request();
        }
        Ok(())
    }
    fn deliver_notifications(&mut self, cx: &mut EventContext) -> Result<(), RuntimeError> {
        if !self.sent {
            self.sent = true;
            for form in &self.forms {
                let result = form.discard_drafts(cx);
                self.results.borrow_mut().push(result);
            }
        }
        Ok(())
    }
}

/// One row whose title is a row-mode `InlineText` in a `Field` of a form.
struct Titles(Signal<String>, FieldHandle);
impl ListItems for Titles {
    fn len(&self) -> usize {
        1
    }
    fn key(&self, index: usize) -> Option<String> {
        (index == 0).then(|| "a".to_owned())
    }
    fn build(&self, index: usize) -> Option<Element> {
        (index == 0).then(|| {
            Field::new(InlineText::new(&self.0).in_row(true).key("title"))
                .form(&self.1)
                .key("title-field")
        })
    }
}

/// New-behavior test 4. An `IntegerInput` holding a refused draft, then an
/// `InlineText` whose open session holds focus and a changed draft, standalone
/// and as a row-mode label in a `VirtualList` row. Named assertions: both
/// drafts end, the session's is not saved, and focus is where Escape leaves
/// it: on the label standalone, on the list in a row. Mutation checked:
/// requesting focus for the first recorded control, as `focus_first_error`
/// does (focus lands on the number field).
#[test]
fn a_discarded_session_returns_focus_as_escape_does() {
    for in_row in [false, true] {
        let context = format!("in a row: {in_row}");
        let mut ui = Runtime::new();
        ui.set_viewport(Viewport::new(400.0, 400.0, 1.0).unwrap())
            .unwrap();
        let form = FormFields::new(&ui);
        let (count_field, title_field) =
            (form.field("count").unwrap(), form.field("title").unwrap());
        let (count, name, go) = (
            ui.signal(7_i64),
            ui.signal("Mix".to_owned()),
            ui.signal(false),
        );
        let (discarder, results) = FormDiscarder::new(vec![form.clone()], &go);
        let title: Element = if in_row {
            let rows = ui.signal(ListSource::new(Titles(name.clone(), title_field.clone())));
            VirtualList::new(rows.read_only(), 48.0)
                .unwrap()
                .selection(ListSelection::new(&ui, SelectionMode::Single))
                .key("list")
                .with_grow(1)
        } else {
            Field::new(InlineText::new(&name).key("title"))
                .form(&title_field)
                .key("title-field")
        };
        let root = ui
            .mount_tree(
                Column::new([
                    Field::new(number(&count, &count_field))
                        .form(&count_field)
                        .key("count-field"),
                    title,
                    discarder.key("discarder"),
                ])
                .key("root"),
            )
            .unwrap();
        settle(&mut ui);
        let number_id = widget(&ui, &root, "number");
        type_over(&mut ui, &number_id, "150");
        press(&mut ui, KeyCode::Enter);
        assert!(invalid(&ui, &number_id), "{context}");
        let title_id = widget(&ui, &root, "title");
        let rest = if in_row {
            let list = widget(&ui, &root, "list");
            ui.focus(&list).unwrap();
            settle(&mut ui);
            ui.text_command(&title_id, TextCommand::BeginEdit).unwrap();
            list
        } else {
            ui.focus(&title_id).unwrap();
            settle(&mut ui);
            press(&mut ui, KeyCode::F2);
            title_id.clone()
        };
        settle(&mut ui);
        ui.text_input("Master").unwrap();
        settle(&mut ui);
        assert_eq!(
            property(&ui, &title_id, "editing"),
            Some(InspectValue::Bool(true)),
            "{context}"
        );
        assert_ne!(
            ui.focused(),
            Some(rest.clone()),
            "{context}: the session's editor"
        );

        go.set(true).unwrap();
        settle(&mut ui);
        assert!(
            matches!(results.borrow().as_slice(), [Ok(2)]),
            "{context}: {:?}",
            results.borrow()
        );
        assert!(!invalid(&ui, &number_id), "{context}");
        assert_eq!(
            property(&ui, &title_id, "editing"),
            Some(InspectValue::Bool(false)),
            "{context}"
        );
        assert_eq!(name.get().unwrap(), "Mix", "{context}: not saved");
        assert_eq!(count.get().unwrap(), 7, "{context}");
        assert_eq!(ui.focused(), Some(rest), "{context}");
    }
}

/// New-behavior test 5 (P12-D1). Runtime A holds a form whose field holds a
/// refused draft and a form whose registered field is not shown; a
/// `FormDiscarder` in runtime B calls `discard_drafts` on both. Named
/// assertions: both calls return `ForeignRuntime`, and runtime A's flush
/// leaves the draft. Mutation checked: checking only the recorded controls'
/// runtime (the unshown form returns `Ok(0)`).
#[test]
fn a_caller_of_another_runtime_queues_nothing() {
    let mut a = Runtime::new();
    a.set_viewport(Viewport::new(400.0, 200.0, 1.0).unwrap())
        .unwrap();
    let (shown_form, unshown_form) = (FormFields::new(&a), FormFields::new(&a));
    let handle = shown_form.field("count").unwrap();
    let _unshown = unshown_form.field("never").unwrap();
    let count = a.signal(7_i64);
    let root = a
        .mount_tree(
            Field::new(number(&count, &handle))
                .form(&handle)
                .key("field"),
        )
        .unwrap();
    settle(&mut a);
    let number_id = widget(&a, &root, "number");
    type_over(&mut a, &number_id, "150");
    press(&mut a, KeyCode::Enter);
    assert!(invalid(&a, &number_id));

    let mut b = Runtime::new();
    let go = b.signal(false);
    let (discarder, results) = FormDiscarder::new(vec![shown_form, unshown_form], &go);
    b.mount_tree(discarder.key("discarder")).unwrap();
    settle(&mut b);
    go.set(true).unwrap();
    settle(&mut b);
    assert!(
        matches!(
            results.borrow().as_slice(),
            [
                Err(RuntimeError::ForeignRuntime),
                Err(RuntimeError::ForeignRuntime)
            ]
        ),
        "{:?}",
        results.borrow()
    );
    settle(&mut a);
    assert!(invalid(&a, &number_id), "nothing was queued");
    assert_eq!(shown(&a, &number_id), "150");
}

/// Contract test, modality inherited from `request_text_command`: field A
/// outside an open modal dialog and field B inside it each hold a refused
/// draft; Revert inside the dialog queues both. The flush returns
/// `ModalBlocked` with A's draft kept (its request is spent) and B's still
/// queued; the next flush discards B's.
#[test]
fn a_modal_blocked_target_spends_its_request() {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(600.0, 600.0, 1.0).unwrap())
        .unwrap();
    let form = FormFields::new(&ui);
    let (a, b) = (form.field("a").unwrap(), form.field("b").unwrap());
    let (outside, inside) = (ui.signal(7_i64), ui.signal(7_i64));
    let dialog = DialogController::new(&ui);
    let (drafts, calls) = (form.clone(), Calls::default());
    let record = calls.clone();
    let revert = Button::new("Revert", move |cx| {
        let result = drafts.discard_drafts(cx);
        record.borrow_mut().push((result, drafts.summary().get()?));
        Ok(())
    });
    let root = ui
        .mount_tree(
            Dialog::new(
                dialog.clone(),
                Field::new(number(&outside, &a)).form(&a).key("a-field"),
                Column::new([
                    Field::new(keyed_number(&inside, &b, "inside"))
                        .form(&b)
                        .key("b-field"),
                    revert.key("revert"),
                ])
                .key("content"),
            )
            .key("root"),
        )
        .unwrap();
    settle(&mut ui);
    let outside_id = widget(&ui, &root, "number");
    type_over(&mut ui, &outside_id, "150");
    press(&mut ui, KeyCode::Enter);
    dialog.open().unwrap();
    settle(&mut ui);
    let inside_id = widget(&ui, &root, "inside");
    type_over(&mut ui, &inside_id, "150");
    press(&mut ui, KeyCode::Enter);
    assert!(invalid(&ui, &outside_id) && invalid(&ui, &inside_id));

    let revert = widget(&ui, &root, "revert");
    ui.focus(&revert).unwrap();
    settle(&mut ui);
    key(&mut ui, KeyCode::Space);
    assert!(matches!(calls.borrow().as_slice(), [(Ok(2), _)]));
    let blocked = ui.flush(ui.now());
    assert!(
        matches!(blocked, Err(RuntimeError::ModalBlocked)),
        "{blocked:?}"
    );
    assert!(invalid(&ui, &outside_id), "the blocked request is spent");
    assert!(
        invalid(&ui, &inside_id),
        "the later request is still queued"
    );
    ui.flush(ui.now()).unwrap();
    assert!(!invalid(&ui, &inside_id));
    settle(&mut ui);
    assert!(
        invalid(&ui, &outside_id),
        "nothing delivers the spent request"
    );
}

/// Regression test (found by this part's Revert with a changed value): a
/// changed application write that ends a refused draft in an ordinary
/// `IntegerInput` or `DecimalInput` leaves valid text and no
/// `validation_error`, while the reported refusal's `None` stays pending
/// for the next Escape. Mutation checked: without refreshing the error in
/// the write's synchronization, `validation_error` stays.
#[test]
fn a_changed_write_ending_a_refused_number_draft_leaves_no_error() {
    for decimal in [false, true] {
        let context = if decimal { "decimal" } else { "integer" };
        let mut ui = Runtime::new();
        ui.set_viewport(Viewport::new(400.0, 200.0, 1.0).unwrap())
            .unwrap();
        let rejections: Rc<RefCell<Vec<bool>>> = Rc::default();
        let log = rejections.clone();
        let report = move |_: &mut EventContext, refusal: Option<&InputRejection>| {
            log.borrow_mut().push(refusal.is_some());
            Ok(())
        };
        let (count, amount) = (ui.signal(7_i64), ui.signal(7.0_f64));
        let field = if decimal {
            DecimalInput::new(&amount)
                .range(1.0, 99.0)
                .unwrap()
                .on_rejection(report)
                .key("field")
        } else {
            IntegerInput::new(&count)
                .range(1, 99)
                .unwrap()
                .on_rejection(report)
                .key("field")
        };
        let root = ui.mount_tree(Column::new([field]).key("root")).unwrap();
        settle(&mut ui);
        let id = widget(&ui, &root, "field");
        type_over(&mut ui, &id, "150");
        press(&mut ui, KeyCode::Enter);
        assert!(invalid(&ui, &id), "{context}");
        assert!(
            property(&ui, &id, "validation_error").is_some(),
            "{context}"
        );
        assert_eq!(rejections.borrow().last(), Some(&true), "{context}");

        if decimal {
            amount.set(8.0).unwrap();
        } else {
            count.set(8).unwrap();
        }
        settle(&mut ui);
        assert!(!invalid(&ui, &id), "{context}");
        assert_eq!(
            property(&ui, &id, "validation_error"),
            None,
            "{context}: the ended draft's error is gone"
        );
        assert_eq!(shown(&ui, &id), "8", "{context}");
        assert_eq!(rejections.borrow().last(), Some(&true), "{context}");
        press(&mut ui, KeyCode::Escape);
        assert_eq!(
            rejections.borrow().last(),
            Some(&false),
            "{context}: the pending None arrives at Escape"
        );
    }
}
