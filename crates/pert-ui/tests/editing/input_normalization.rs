//! Commit-time normalization of every input field through the public facade.
use crate::support;
use pert_ui::*;
use std::{cell::RefCell, rc::Rc};
use support::{frame, key};

fn command(ui: &mut Runtime, code: KeyCode) {
    let modifier = KeyCode::primary();
    ui.key_input(modifier, KeyAction::Press).unwrap();
    key(ui, code);
    ui.key_input(modifier, KeyAction::Release).unwrap();
}
fn replace(ui: &mut Runtime, text: &str) {
    command(ui, KeyCode::KeyA);
    key(ui, KeyCode::Backspace);
    ui.text_input(text).unwrap();
    frame(ui);
}
fn component(ui: &Runtime) -> ComponentInspection {
    ui.inspect()
        .components
        .into_iter()
        .find(|c| c.key == "field")
        .unwrap()
}
fn property(ui: &Runtime, name: &str) -> Option<InspectValue> {
    component(ui).details.properties.get(name).cloned()
}
fn invalid(ui: &Runtime) -> bool {
    component(ui).details.semantics.unwrap().invalid
}
/// Mounts `field` beside a button that can take focus, and focuses the field.
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
    let children = ui.children(&root).unwrap();
    ui.focus(&children[0]).unwrap();
    frame(ui);
    (children[0].clone(), children[1].clone())
}
fn trim_lower(text: String) -> Result<String, InputRejection> {
    Ok(text.trim().to_lowercase())
}
fn not_empty(text: String) -> Result<String, InputRejection> {
    let text = text.trim().to_owned();
    if text.is_empty() {
        Err(InputRejection::new("Enter a name"))
    } else {
        Ok(text)
    }
}
fn tenths(value: f64) -> Result<f64, InputRejection> {
    Ok((value * 10.0).round() / 10.0)
}

#[test]
fn a_normalizing_text_field_holds_a_draft_until_enter_or_focus_loss() {
    let mut ui = Runtime::new();
    let tag = ui.signal(String::from("old"));
    let submitted = Rc::new(RefCell::new(Vec::new()));
    let log = submitted.clone();
    let (field, other) = mount(
        &mut ui,
        TextField::new(tag.clone())
            .normalize(trim_lower)
            .on_submit(move |_, text| {
                log.borrow_mut().push(text.to_owned());
                Ok(())
            }),
    );
    replace(&mut ui, "  Drums ");
    assert_eq!(tag.get().unwrap(), "old", "edits stay in the draft");
    assert_eq!(property(&ui, "dirty"), Some(InspectValue::Bool(true)));
    key(&mut ui, KeyCode::Enter);
    frame(&mut ui);
    assert_eq!(tag.get().unwrap(), "drums");
    assert_eq!(
        property(&ui, "text"),
        Some(InspectValue::Text("drums".into())),
        "the saved text replaces the draft"
    );
    assert_eq!(*submitted.borrow(), ["drums"]);
    replace(&mut ui, " BASS");
    ui.focus(&other).unwrap();
    frame(&mut ui);
    assert_eq!(tag.get().unwrap(), "bass", "focus transfer saves");
    ui.focus(&field).unwrap();
    replace(&mut ui, "Keys ");
    ui.clear_focus();
    frame(&mut ui);
    assert_eq!(tag.get().unwrap(), "keys", "window focus loss saves");
    assert_eq!(*submitted.borrow(), ["drums"], "only Enter submits");
}

#[test]
fn a_refused_text_draft_is_kept_marked_and_never_written_or_submitted() {
    let mut ui = Runtime::new();
    let name = ui.signal(String::from("Lead"));
    let submitted = Rc::new(RefCell::new(0));
    let count = submitted.clone();
    let (field, other) = mount(
        &mut ui,
        TextField::new(name.clone())
            .normalize(not_empty)
            .on_submit(move |_, _| {
                *count.borrow_mut() += 1;
                Ok(())
            }),
    );
    replace(&mut ui, "   ");
    key(&mut ui, KeyCode::Enter);
    frame(&mut ui);
    assert_eq!(name.get().unwrap(), "Lead");
    assert_eq!(*submitted.borrow(), 0, "a refused save does not submit");
    assert_eq!(
        property(&ui, "validation_error"),
        Some(InspectValue::Text("Enter a name".into()))
    );
    assert!(invalid(&ui));
    assert_eq!(
        property(&ui, "text"),
        Some(InspectValue::Text("   ".into()))
    );
    ui.focus(&other).unwrap();
    frame(&mut ui);
    assert_eq!(name.get().unwrap(), "Lead", "focus loss refuses it too");
    assert!(property(&ui, "validation_error").is_some());
    // Correcting the draft ends the refusal before the next save.
    ui.focus(&field).unwrap();
    ui.text_input("Pad").unwrap();
    frame(&mut ui);
    assert!(property(&ui, "validation_error").is_none());
    assert!(!invalid(&ui));
    key(&mut ui, KeyCode::Enter);
    assert_eq!(name.get().unwrap(), "Pad");
    assert_eq!(*submitted.borrow(), 1);
}

#[test]
fn a_normalized_result_that_is_not_bound_text_is_refused() {
    let mut ui = Runtime::new();
    let value = ui.signal(String::from("a"));
    mount(
        &mut ui,
        TextField::new(value.clone()).normalize(|text| Ok(format!("{text}\n"))),
    );
    replace(&mut ui, "b");
    key(&mut ui, KeyCode::Enter);
    frame(&mut ui);
    assert_eq!(value.get().unwrap(), "a");
    assert!(property(&ui, "validation_error").is_some());
    assert!(invalid(&ui));
}

#[test]
fn an_external_write_wins_over_a_text_draft_and_is_never_normalized() {
    let mut ui = Runtime::new();
    let tag = ui.signal(String::from("old"));
    let (_, other) = mount(&mut ui, TextField::new(tag.clone()).normalize(trim_lower));
    replace(&mut ui, "Draft");
    tag.set("  From App ".into()).unwrap();
    frame(&mut ui);
    assert_eq!(
        property(&ui, "text"),
        Some(InspectValue::Text("  From App ".into()))
    );
    assert_eq!(property(&ui, "dirty"), Some(InspectValue::Bool(false)));
    ui.focus(&other).unwrap();
    frame(&mut ui);
    assert_eq!(
        tag.get().unwrap(),
        "  From App ",
        "focus loss has no user edit to save"
    );
}

#[test]
fn undo_acts_on_the_draft_and_a_save_clears_it() {
    let mut ui = Runtime::new();
    let calls = Rc::new(RefCell::new(Vec::new()));
    let log = calls.clone();
    let tag = ui.signal(String::new());
    mount(
        &mut ui,
        TextField::new(tag.clone()).normalize(move |text| {
            log.borrow_mut().push(text.clone());
            Ok(text.to_uppercase())
        }),
    );
    ui.text_input("a").unwrap();
    ui.text_input("b").unwrap();
    command(&mut ui, KeyCode::KeyZ);
    frame(&mut ui);
    assert_eq!(property(&ui, "text"), Some(InspectValue::Text("a".into())));
    assert!(calls.borrow().is_empty(), "undo never normalizes");
    assert_eq!(tag.get().unwrap(), "");
    key(&mut ui, KeyCode::Enter);
    assert_eq!(tag.get().unwrap(), "A");
    assert_eq!(*calls.borrow(), ["a"]);
    command(&mut ui, KeyCode::KeyZ);
    frame(&mut ui);
    assert_eq!(property(&ui, "text"), Some(InspectValue::Text("A".into())));
    assert_eq!(tag.get().unwrap(), "A", "a save cleared the local history");
}

#[test]
fn escape_and_hiding_discard_a_text_draft() {
    let mut ui = Runtime::new();
    let tag = ui.signal(String::from("kept"));
    let (field, _) = mount(&mut ui, TextField::new(tag.clone()).normalize(trim_lower));
    replace(&mut ui, "Draft");
    key(&mut ui, KeyCode::Escape);
    frame(&mut ui);
    assert_eq!(
        property(&ui, "text"),
        Some(InspectValue::Text("kept".into()))
    );
    assert_eq!(tag.get().unwrap(), "kept");
    replace(&mut ui, "Hidden");
    ui.set_visible(&field, false).unwrap();
    frame(&mut ui);
    assert_eq!(tag.get().unwrap(), "kept");
    ui.set_visible(&field, true).unwrap();
    frame(&mut ui);
    assert_eq!(
        property(&ui, "text"),
        Some(InspectValue::Text("kept".into()))
    );
}

#[test]
fn ime_preedit_is_never_passed_to_the_normalizer() {
    let mut ui = Runtime::new();
    let calls = Rc::new(RefCell::new(Vec::new()));
    let log = calls.clone();
    let tag = ui.signal(String::new());
    let (_, other) = mount(
        &mut ui,
        TextField::new(tag.clone()).normalize(move |text| {
            log.borrow_mut().push(text.clone());
            Ok(text)
        }),
    );
    ui.text_input("ab").unwrap();
    ui.text_preedit(&TextPreedit::new("pending", None).unwrap())
        .unwrap();
    key(&mut ui, KeyCode::Enter);
    frame(&mut ui);
    assert!(
        calls.borrow().is_empty(),
        "Enter during composition belongs to the IME"
    );
    ui.focus(&other).unwrap();
    frame(&mut ui);
    assert_eq!(*calls.borrow(), ["ab"]);
    assert_eq!(tag.get().unwrap(), "ab");
}

#[test]
fn a_text_area_saves_on_focus_loss_and_enter_inserts_a_line_break() {
    let mut ui = Runtime::new();
    let notes = ui.signal(String::new());
    let (_, other) = mount(
        &mut ui,
        TextArea::new(notes.clone()).normalize(|text| Ok(text.trim_end().to_owned())),
    );
    ui.text_input("first").unwrap();
    key(&mut ui, KeyCode::Enter);
    ui.text_input("second").unwrap();
    key(&mut ui, KeyCode::Enter);
    frame(&mut ui);
    assert_eq!(
        notes.get().unwrap(),
        "",
        "Enter is a line break, not a save"
    );
    ui.focus(&other).unwrap();
    frame(&mut ui);
    assert_eq!(notes.get().unwrap(), "first\nsecond");
}

#[test]
fn a_text_field_without_a_normalizer_still_writes_every_edit() {
    let mut ui = Runtime::new();
    let tag = ui.signal(String::new());
    mount(&mut ui, TextField::new(tag.clone()));
    ui.text_input("Live").unwrap();
    assert_eq!(tag.get().unwrap(), "Live");
    assert!(property(&ui, "dirty").is_none());
}

#[test]
fn inline_text_normalizes_changed_drafts_and_keeps_refused_ones_open() {
    let mut ui = Runtime::new();
    let calls = Rc::new(RefCell::new(0));
    let count = calls.clone();
    let name = ui.signal(String::from("Lead"));
    let (field, other) = mount(
        &mut ui,
        InlineText::new(name.clone()).normalize(move |text| {
            *count.borrow_mut() += 1;
            not_empty(text)
        }),
    );
    // An unchanged draft saves nothing and is never normalized.
    key(&mut ui, KeyCode::F2);
    frame(&mut ui);
    key(&mut ui, KeyCode::Enter);
    frame(&mut ui);
    assert_eq!(*calls.borrow(), 0);
    assert_eq!(property(&ui, "editing"), Some(InspectValue::Bool(false)));
    ui.focus(&field).unwrap();
    key(&mut ui, KeyCode::F2);
    frame(&mut ui);
    ui.text_input("  ").unwrap();
    key(&mut ui, KeyCode::Enter);
    frame(&mut ui);
    assert_eq!(name.get().unwrap(), "Lead");
    assert_eq!(property(&ui, "editing"), Some(InspectValue::Bool(true)));
    assert_eq!(
        property(&ui, "validation_error"),
        Some(InspectValue::Text("Enter a name".into()))
    );
    assert!(invalid(&ui));
    ui.text_input(" Bass ").unwrap();
    frame(&mut ui);
    assert!(
        property(&ui, "validation_error").is_none(),
        "a changed draft ends the refusal"
    );
    ui.focus(&other).unwrap();
    frame(&mut ui);
    assert_eq!(name.get().unwrap(), "Bass");
    assert_eq!(property(&ui, "editing"), Some(InspectValue::Bool(false)));
}

#[test]
fn numbers_parse_then_normalize_then_check_the_range() {
    let mut ui = Runtime::new();
    let gain = ui.signal(1.0_f64);
    let (field, _) = mount(
        &mut ui,
        DecimalInput::new(gain.clone())
            .range(0.0, 9.99)
            .unwrap()
            .normalize(tenths)
            .unwrap(),
    );
    replace(&mut ui, "3.14159");
    key(&mut ui, KeyCode::Enter);
    assert_eq!(gain.get().unwrap(), 3.1);
    frame(&mut ui);
    assert_eq!(
        property(&ui, "text"),
        Some(InspectValue::Text("3.1".into()))
    );
    // 9.96 rounds to 10.0, beyond the maximum: an error, never clamped.
    ui.focus(&field).unwrap();
    replace(&mut ui, "9.96");
    assert_eq!(property(&ui, "valid"), Some(InspectValue::Bool(false)));
    key(&mut ui, KeyCode::Enter);
    ui.clear_focus();
    frame(&mut ui);
    assert_eq!(gain.get().unwrap(), 3.1);
    assert_eq!(
        property(&ui, "text"),
        Some(InspectValue::Text("9.96".into()))
    );
}

#[test]
fn a_number_rejection_or_non_finite_result_writes_nothing() {
    let mut ui = Runtime::new();
    let value = ui.signal(4_i64);
    mount(
        &mut ui,
        IntegerInput::new(value.clone())
            .normalize(|n| {
                if n % 2 == 0 {
                    Ok(n)
                } else {
                    Err(InputRejection::new("Enter an even number"))
                }
            })
            .unwrap(),
    );
    replace(&mut ui, "7");
    key(&mut ui, KeyCode::Enter);
    frame(&mut ui);
    assert_eq!(value.get().unwrap(), 4);
    assert_eq!(
        property(&ui, "validation_error"),
        Some(InspectValue::Text("Enter an even number".into()))
    );
    assert!(invalid(&ui));

    let mut ui = Runtime::new();
    let value = ui.signal(1.0_f64);
    mount(
        &mut ui,
        DecimalInput::new(value.clone())
            .normalize(|_| Ok(f64::NAN))
            .unwrap(),
    );
    replace(&mut ui, "2");
    key(&mut ui, KeyCode::Enter);
    assert_eq!(value.get().unwrap(), 1.0);
    assert_eq!(property(&ui, "valid"), Some(InspectValue::Bool(false)));
}

#[test]
fn the_bound_number_is_never_normalized() {
    let mut ui = Runtime::new();
    let gain = ui.signal(1.234_f64);
    let (field, other) = mount(
        &mut ui,
        DecimalInput::new(gain.clone()).normalize(tenths).unwrap(),
    );
    ui.focus(&other).unwrap();
    frame(&mut ui);
    assert_eq!(
        gain.get().unwrap(),
        1.234,
        "an unchanged draft is not saved"
    );
    gain.set(5.678).unwrap();
    frame(&mut ui);
    ui.focus(&field).unwrap();
    key(&mut ui, KeyCode::Enter);
    ui.focus(&other).unwrap();
    assert_eq!(
        gain.get().unwrap(),
        5.678,
        "an application write stays as written"
    );
}

#[test]
fn draft_steps_are_normalized_on_save_and_saving_steps_as_they_save() {
    let mut ui = Runtime::new();
    let value = ui.signal(0.0_f64);
    mount(
        &mut ui,
        DecimalInput::new(value.clone())
            .step(0.25)
            .unwrap()
            .normalize(tenths)
            .unwrap(),
    );
    key(&mut ui, KeyCode::ArrowUp);
    key(&mut ui, KeyCode::ArrowUp);
    frame(&mut ui);
    assert_eq!(
        property(&ui, "text"),
        Some(InspectValue::Text("0.5".into())),
        "draft steps move the draft from its own value"
    );
    key(&mut ui, KeyCode::Enter);
    assert_eq!(value.get().unwrap(), 0.5);

    // Stepping from the draft's own value never sticks on a coarse
    // normalizer: 3 + 1 is 4, not the step from the normalized 2.
    let mut ui = Runtime::new();
    let value = ui.signal(0_i64);
    mount(
        &mut ui,
        IntegerInput::new(value.clone())
            .normalize(|n| Ok(n - n.rem_euclid(2)))
            .unwrap(),
    );
    replace(&mut ui, "3");
    key(&mut ui, KeyCode::ArrowUp);
    frame(&mut ui);
    assert_eq!(property(&ui, "text"), Some(InspectValue::Text("4".into())));
    key(&mut ui, KeyCode::Enter);
    assert_eq!(value.get().unwrap(), 4);

    // An inline row's resting arrow saves at once, normalized.
    let mut ui = Runtime::new();
    let value = ui.signal(0.0_f64);
    mount(
        &mut ui,
        InlineDecimal::new(value.clone())
            .step(0.25)
            .unwrap()
            .normalize(tenths)
            .unwrap(),
    );
    key(&mut ui, KeyCode::ArrowUp);
    assert_eq!(value.get().unwrap(), 0.3);

    // A step smaller than the rounding normalizes back and does nothing.
    let mut ui = Runtime::new();
    let value = ui.signal(1.0_f64);
    mount(
        &mut ui,
        InlineDecimal::new(value.clone())
            .step(0.01)
            .unwrap()
            .normalize(tenths)
            .unwrap(),
    );
    key(&mut ui, KeyCode::ArrowUp);
    frame(&mut ui);
    assert_eq!(value.get().unwrap(), 1.0);
}

#[test]
fn a_refused_step_changes_nothing() {
    let mut ui = Runtime::new();
    let value = ui.signal(3_i64);
    mount(
        &mut ui,
        InlineInteger::new(value.clone())
            .normalize(|n| {
                if n > 3 {
                    Err(InputRejection::new("At most three"))
                } else {
                    Ok(n)
                }
            })
            .unwrap(),
    );
    key(&mut ui, KeyCode::ArrowUp);
    frame(&mut ui);
    assert_eq!(value.get().unwrap(), 3);
    assert_eq!(property(&ui, "valid"), Some(InspectValue::Bool(true)));
    key(&mut ui, KeyCode::ArrowDown);
    assert_eq!(value.get().unwrap(), 2);
}

#[test]
fn a_refused_step_button_keeps_a_typed_draft_unsaved() {
    let mut ui = Runtime::new();
    let value = ui.signal(1_i64);
    let (id, _) = mount(
        &mut ui,
        IntegerInput::new(value.clone())
            .stepper(true)
            .normalize(|n| {
                if n == 3 {
                    Err(InputRejection::new("Not three"))
                } else {
                    Ok(n)
                }
            })
            .unwrap(),
    );
    replace(&mut ui, "2");
    let [x, y, w, h] = ui.bounds(&id).unwrap().unwrap().values();
    let metrics = ui.theme().metrics(Density::Standard).unwrap();
    let pad = metrics.metric(MetricRole::ControlPaddingX);
    let side = metrics.metric(MetricRole::IconSize);
    let plus = Point::new(x + w - pad - side / 2.0, y + h / 2.0).unwrap();
    for kind in [
        PointerKind::Press(PointerButton::Primary),
        PointerKind::Release(PointerButton::Primary),
    ] {
        ui.pointer_input(PointerEvent::new(plus, kind)).unwrap();
    }
    frame(&mut ui);
    assert_eq!(value.get().unwrap(), 1, "the refused step saved nothing");
    assert_eq!(property(&ui, "text"), Some(InspectValue::Text("2".into())));
}

#[test]
fn an_application_write_before_the_next_update_wins_over_a_save() {
    let mut ui = Runtime::new();
    let tag = ui.signal(String::from("old"));
    let (_, other) = mount(&mut ui, TextField::new(tag.clone()).normalize(trim_lower));
    replace(&mut ui, "Draft");
    tag.set("App".into()).unwrap();
    ui.focus(&other).unwrap();
    frame(&mut ui);
    assert_eq!(tag.get().unwrap(), "App");
    assert_eq!(
        property(&ui, "text"),
        Some(InspectValue::Text("App".into()))
    );

    let mut ui = Runtime::new();
    let name = ui.signal(String::from("Lead"));
    let (_, other) = mount(&mut ui, InlineText::new(name.clone()).normalize(not_empty));
    key(&mut ui, KeyCode::F2);
    frame(&mut ui);
    ui.text_input("Draft").unwrap();
    name.set("App".into()).unwrap();
    ui.focus(&other).unwrap();
    frame(&mut ui);
    assert_eq!(name.get().unwrap(), "App");
    assert_eq!(property(&ui, "editing"), Some(InspectValue::Bool(false)));
}

#[test]
fn an_inline_text_refusal_ends_with_any_edit_of_the_draft() {
    let mut ui = Runtime::new();
    let name = ui.signal(String::from("Lead"));
    mount(&mut ui, InlineText::new(name.clone()).normalize(not_empty));
    key(&mut ui, KeyCode::F2);
    frame(&mut ui);
    ui.text_input(" ").unwrap();
    key(&mut ui, KeyCode::Enter);
    frame(&mut ui);
    assert!(property(&ui, "validation_error").is_some());
    ui.text_input("x").unwrap();
    key(&mut ui, KeyCode::Backspace);
    frame(&mut ui);
    assert!(
        property(&ui, "validation_error").is_none(),
        "the same text again is not refused until it is saved again"
    );
    assert_eq!(name.get().unwrap(), "Lead");
}

#[test]
fn a_parameter_bound_number_refuses_a_normalizer() {
    let mut ui = Runtime::new();
    let (count, _host) = ui
        .parameter(
            ParameterId::new(1, 1, "count").unwrap(),
            ParameterSpec::stepped("Count", 0, 10, 5).unwrap(),
        )
        .unwrap();
    assert_eq!(
        IntegerInput::for_parameter(count.clone())
            .normalize(Ok)
            .err(),
        Some(NumberInputError::ParameterDomain)
    );
    assert_eq!(
        InlineInteger::for_parameter(count).normalize(Ok).err(),
        Some(NumberInputError::ParameterDomain)
    );
}

type Reports = Rc<RefCell<Vec<Option<String>>>>;
fn recorder() -> (
    Reports,
    impl FnMut(&mut EventContext, Option<&InputRejection>) -> Result<(), RuntimeError> + 'static,
) {
    let reports: Reports = Rc::new(RefCell::new(Vec::new()));
    let log = reports.clone();
    (reports, move |_, rejection: Option<&InputRejection>| {
        log.borrow_mut()
            .push(rejection.map(|r| r.message().to_owned()));
        Ok(())
    })
}
fn enter(ui: &mut Runtime) {
    key(ui, KeyCode::Enter);
    frame(ui);
}

#[test]
fn a_text_field_reports_each_refused_save_and_the_end_of_a_refusal_once() {
    let mut ui = Runtime::new();
    let (reports, report) = recorder();
    let name = ui.signal(String::new());
    mount(
        &mut ui,
        TextField::new(name.clone())
            .normalize(not_empty)
            .on_rejection(report),
    );
    ui.text_input("  ").unwrap();
    enter(&mut ui);
    enter(&mut ui);
    assert_eq!(
        *reports.borrow(),
        [Some("Enter a name".into()), Some("Enter a name".into())],
        "every refused save is reported"
    );
    key(&mut ui, KeyCode::Escape);
    frame(&mut ui);
    assert_eq!(reports.borrow().last(), Some(&None), "Escape ends it");
    key(&mut ui, KeyCode::Escape);
    frame(&mut ui);
    assert_eq!(reports.borrow().len(), 3, "the end is reported once");
    ui.text_input(" ").unwrap();
    enter(&mut ui);
    ui.text_input("Ada").unwrap();
    enter(&mut ui);
    assert_eq!(
        reports.borrow()[3..],
        [Some("Enter a name".into()), None],
        "a successful save ends it"
    );
    assert_eq!(name.get().unwrap(), "Ada");
    enter(&mut ui);
    assert_eq!(
        reports.borrow().len(),
        5,
        "saves without a refusal report nothing"
    );
}

#[test]
fn the_end_of_a_refusal_discarded_by_the_application_is_reported_at_the_next_save() {
    let mut ui = Runtime::new();
    let (reports, report) = recorder();
    let name = ui.signal(String::new());
    let (_, other) = mount(
        &mut ui,
        TextField::new(name.clone())
            .normalize(not_empty)
            .on_rejection(report),
    );
    ui.text_input("  ").unwrap();
    enter(&mut ui);
    name.set("From app".into()).unwrap();
    frame(&mut ui);
    assert_eq!(reports.borrow().len(), 1, "an update runs no callback");
    ui.focus(&other).unwrap();
    frame(&mut ui);
    assert_eq!(*reports.borrow(), [Some("Enter a name".into()), None]);
}

#[test]
fn numbers_report_refusals_on_save_and_not_while_typing() {
    let mut ui = Runtime::new();
    let (reports, report) = recorder();
    let value = ui.signal(4_i64);
    mount(
        &mut ui,
        IntegerInput::new(value.clone())
            .normalize(|n| {
                if n % 2 == 0 {
                    Ok(n)
                } else {
                    Err(InputRejection::new("Enter an even number"))
                }
            })
            .unwrap()
            .on_rejection(report),
    );
    replace(&mut ui, "7");
    assert!(
        reports.borrow().is_empty(),
        "the draft shows it; nobody is told yet"
    );
    enter(&mut ui);
    assert_eq!(*reports.borrow(), [Some("Enter an even number".into())]);
    // The field's own refusal of an unparsable draft is reported too.
    replace(&mut ui, "x");
    enter(&mut ui);
    assert_eq!(
        reports.borrow().last(),
        Some(&Some("Enter a whole number".into()))
    );
    replace(&mut ui, "7");
    enter(&mut ui);
    replace(&mut ui, "8");
    enter(&mut ui);
    assert_eq!(value.get().unwrap(), 8);
    assert_eq!(
        reports.borrow()[2..],
        [Some("Enter an even number".into()), None]
    );
}

#[test]
fn numbers_report_their_own_refusals_with_the_installed_strings() {
    let mut ui = Runtime::new();
    let (reports, report) = recorder();
    let value = ui.signal(5.0_f64);
    mount(
        &mut ui,
        DecimalInput::new(value.clone())
            .range(0.0, 10.0)
            .unwrap()
            .decimal_places(1)
            .unwrap()
            .on_rejection(report),
    );
    enter(&mut ui);
    assert!(
        reports.borrow().is_empty(),
        "an unchanged draft refuses nothing"
    );
    replace(&mut ui, "abc");
    assert!(reports.borrow().is_empty(), "not while typing");
    enter(&mut ui);
    replace(&mut ui, "11");
    enter(&mut ui);
    assert_eq!(value.get().unwrap(), 5.0, "nothing refused is written");
    replace(&mut ui, "7");
    enter(&mut ui);
    assert_eq!(value.get().unwrap(), 7.0);
    assert_eq!(
        *reports.borrow(),
        [
            Some("Enter a number".into()),
            Some("Enter a number from 0.0 to 10.0".into()),
            None
        ]
    );

    // An integer parameter's lattice, and a replaced table's wording.
    let mut ui = Runtime::new();
    ui.ui_settings()
        .set_strings(
            UiStrings::english()
                .with_text(StringRole::NotAWholeNumber, "Skriv ett heltal")
                .unwrap()
                .with_text(StringRole::NumberOffStep, "Steg om {step}")
                .unwrap(),
        )
        .unwrap();
    let (reports, report) = recorder();
    let (count, _host) = ui
        .parameter(
            ParameterId::new(1, 1, "count").unwrap(),
            ParameterSpec::stepped("Count", 0, 100, 10)
                .unwrap()
                .with_step(std::num::NonZeroU64::new(5).unwrap())
                .unwrap(),
        )
        .unwrap();
    mount(
        &mut ui,
        IntegerInput::for_parameter(count.clone()).on_rejection(report),
    );
    replace(&mut ui, "1.5");
    enter(&mut ui);
    replace(&mut ui, "7");
    enter(&mut ui);
    replace(&mut ui, "15");
    enter(&mut ui);
    assert_eq!(count.snapshot().get().unwrap().base, 15);
    assert_eq!(
        *reports.borrow(),
        [
            Some("Skriv ett heltal".into()),
            Some("Steg om 5".into()),
            None
        ]
    );
}

#[test]
fn inline_text_reports_refusals_and_their_end_at_escape() {
    let mut ui = Runtime::new();
    let (reports, report) = recorder();
    let name = ui.signal(String::from("Lead"));
    mount(
        &mut ui,
        InlineText::new(name.clone())
            .normalize(not_empty)
            .on_rejection(report),
    );
    key(&mut ui, KeyCode::F2);
    frame(&mut ui);
    ui.text_input(" ").unwrap();
    enter(&mut ui);
    assert_eq!(*reports.borrow(), [Some("Enter a name".into())]);
    key(&mut ui, KeyCode::Escape);
    frame(&mut ui);
    assert_eq!(*reports.borrow(), [Some("Enter a name".into()), None]);
    assert_eq!(name.get().unwrap(), "Lead");
}

#[test]
fn a_refused_immediate_step_is_reported() {
    let mut ui = Runtime::new();
    let (reports, report) = recorder();
    let value = ui.signal(2_i64);
    let refuse_three = |n: i64| {
        if n == 3 {
            Err(InputRejection::new("Not three"))
        } else {
            Ok(n)
        }
    };
    let (id, _) = mount(
        &mut ui,
        IntegerInput::new(value.clone())
            .stepper(true)
            .normalize(refuse_three)
            .unwrap()
            .on_rejection(report),
    );
    let [x, y, w, h] = ui.bounds(&id).unwrap().unwrap().values();
    let metrics = ui.theme().metrics(Density::Standard).unwrap();
    let plus = Point::new(
        x + w
            - metrics.metric(MetricRole::ControlPaddingX)
            - metrics.metric(MetricRole::IconSize) / 2.0,
        y + h / 2.0,
    )
    .unwrap();
    for kind in [
        PointerKind::Press(PointerButton::Primary),
        PointerKind::Release(PointerButton::Primary),
    ] {
        ui.pointer_input(PointerEvent::new(plus, kind)).unwrap();
    }
    frame(&mut ui);
    assert_eq!(value.get().unwrap(), 2);
    assert_eq!(*reports.borrow(), [Some("Not three".into())]);

    let mut ui = Runtime::new();
    let (reports, report) = recorder();
    let value = ui.signal(2_i64);
    mount(
        &mut ui,
        InlineInteger::new(value.clone())
            .normalize(refuse_three)
            .unwrap()
            .on_rejection(report),
    );
    key(&mut ui, KeyCode::ArrowUp);
    frame(&mut ui);
    assert_eq!(value.get().unwrap(), 2);
    assert_eq!(*reports.borrow(), [Some("Not three".into())]);
    key(&mut ui, KeyCode::ArrowDown);
    assert_eq!(value.get().unwrap(), 1);
    assert_eq!(reports.borrow().last(), Some(&None), "a saved step ends it");
}

#[test]
fn a_failing_callback_leaves_the_field_consistent_and_retries_the_end() {
    let mut ui = Runtime::new();
    let calls = Rc::new(RefCell::new(Vec::new()));
    let log = calls.clone();
    let fail_next_end = Rc::new(RefCell::new(true));
    let fail = fail_next_end.clone();
    let name = ui.signal(String::new());
    mount(
        &mut ui,
        TextField::new(name.clone())
            .normalize(not_empty)
            .on_rejection(move |_, rejection| {
                log.borrow_mut().push(rejection.is_some());
                if rejection.is_some() || fail.replace(false) {
                    Err(RuntimeError::Closed)
                } else {
                    Ok(())
                }
            }),
    );
    ui.text_input(" ").unwrap();
    assert!(ui.key_input(KeyCode::Enter, KeyAction::Press).is_err());
    ui.key_input(KeyCode::Enter, KeyAction::Release).unwrap();
    frame(&mut ui);
    assert_eq!(name.get().unwrap(), "");
    assert!(
        property(&ui, "validation_error").is_some(),
        "the refusal is shown although its report failed"
    );
    ui.text_input("Ada").unwrap();
    assert!(ui.key_input(KeyCode::Enter, KeyAction::Press).is_err());
    ui.key_input(KeyCode::Enter, KeyAction::Release).unwrap();
    frame(&mut ui);
    assert_eq!(
        name.get().unwrap(),
        "Ada",
        "the save happened before the report"
    );
    enter(&mut ui);
    assert_eq!(
        *calls.borrow(),
        [true, false, false],
        "the failed end is reported again"
    );
    enter(&mut ui);
    assert_eq!(calls.borrow().len(), 3);
}

#[test]
fn escape_ends_a_refusal_an_application_write_discarded_and_still_bubbles() {
    let mut ui = Runtime::new();
    let (reports, report) = recorder();
    let name = ui.signal(String::new());
    mount(
        &mut ui,
        TextField::new(name.clone())
            .normalize(not_empty)
            .on_rejection(report),
    );
    ui.text_input(" ").unwrap();
    enter(&mut ui);
    name.set("App".into()).unwrap();
    frame(&mut ui);
    key(&mut ui, KeyCode::Escape);
    frame(&mut ui);
    assert_eq!(*reports.borrow(), [Some("Enter a name".into()), None]);
}

#[test]
fn inline_text_ends_a_refusal_after_an_application_write_on_focus_loss() {
    let mut ui = Runtime::new();
    let (reports, report) = recorder();
    let name = ui.signal(String::from("Lead"));
    let (_, other) = mount(
        &mut ui,
        InlineText::new(name.clone())
            .normalize(not_empty)
            .on_rejection(report),
    );
    key(&mut ui, KeyCode::F2);
    frame(&mut ui);
    ui.text_input(" ").unwrap();
    enter(&mut ui);
    name.set("App".into()).unwrap();
    frame(&mut ui);
    assert_eq!(property(&ui, "editing"), Some(InspectValue::Bool(false)));
    ui.focus(&other).unwrap();
    frame(&mut ui);
    assert_eq!(*reports.borrow(), [Some("Enter a name".into()), None]);
}

#[test]
fn disabled_text_editors_do_not_save_on_immediate_focus_loss() {
    for multiline in [false, true] {
        for normalize in [false, true] {
            let mut ui = Runtime::new();
            ui.set_viewport(Viewport::new(400.0, 200.0, 1.0).unwrap())
                .unwrap();
            let value = ui.signal("old".to_owned());
            let saves = ui.signal(0);
            let root = ui
                .mount_tree(Element::new("root", Column::default()))
                .unwrap();
            let describe = |disabled| {
                let saved = saves.clone();
                let field = if multiline {
                    let mut field = TextArea::new(value.clone())
                        .on_save(move |_, _| saved.set(saved.get()? + 1));
                    field = if normalize {
                        field.normalize(trim_lower)
                    } else {
                        field.min_length(Some(1))
                    };
                    Element::new("field", if disabled { field.disabled() } else { field })
                } else {
                    let mut field = TextField::new(value.clone())
                        .on_save(move |_, _| saved.set(saved.get()? + 1));
                    field = if normalize {
                        field.normalize(trim_lower)
                    } else {
                        field.min_length(Some(1))
                    };
                    Element::new("field", if disabled { field.disabled() } else { field })
                };
                vec![field]
            };
            ui.reconcile_children(&root, describe(false)).unwrap();
            frame(&mut ui);
            let field = ui.children(&root).unwrap()[0].clone();
            ui.focus(&field).unwrap();
            replace(&mut ui, "draft");
            ui.reconcile_children(&root, describe(true)).unwrap();
            ui.clear_focus();
            assert_eq!(value.get().unwrap(), "old");
            assert_eq!(saves.get().unwrap(), 0);
            ui.reconcile_children(&root, describe(false)).unwrap();
            frame(&mut ui);
            ui.focus(&field).unwrap();
            ui.clear_focus();
            assert_eq!(value.get().unwrap(), "draft");
            assert_eq!(saves.get().unwrap(), 1);
        }
    }
}

#[test]
fn the_identity_normalizer_saves_finished_edits_without_changing_them() {
    for multiline in [false, true] {
        let mut ui = Runtime::new();
        let value = ui.signal("Name".to_owned());
        let saves = Rc::new(RefCell::new(Vec::new()));
        let saved = saves.clone();
        let record = move |_: &mut EventContext, text: &str| {
            saved.borrow_mut().push(text.to_owned());
            Ok(())
        };
        let (field, other) = if multiline {
            mount(&mut ui, TextArea::new(&value).normalize(Ok).on_save(record))
        } else {
            mount(
                &mut ui,
                TextField::new(&value).normalize(Ok).on_save(record),
            )
        };
        ui.focus(&field).unwrap();
        replace(&mut ui, " Edited ");
        // A draft: nothing is written while typing.
        assert_eq!(value.get().unwrap(), "Name");
        assert!(saves.borrow().is_empty());
        // Enter saves a single-line field and inserts a line break in a
        // multiline one, which saves on focus loss.
        key(&mut ui, KeyCode::Enter);
        if multiline {
            assert_eq!(value.get().unwrap(), "Name");
            ui.focus(&other).unwrap();
            frame(&mut ui);
            assert_eq!(value.get().unwrap(), " Edited \n", "saved unchanged");
            assert_eq!(*saves.borrow(), [" Edited \n"]);
        } else {
            assert_eq!(value.get().unwrap(), " Edited ", "saved unchanged");
            assert_eq!(*saves.borrow(), [" Edited "]);
            ui.focus(&other).unwrap();
            frame(&mut ui);
            assert_eq!(saves.borrow().len(), 1, "an unchanged draft saves nothing");
        }
    }
}
