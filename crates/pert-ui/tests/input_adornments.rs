//! The second slice of input field states through the public facade: the
//! clear button, a leading icon and suffix text, and the modified state with
//! its indicator, callback and form count.
use pert_ui::*;
mod support;
use std::{cell::RefCell, rc::Rc, time::Duration};
use support::frame;

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
fn click(ui: &mut Runtime, x: f32, y: f32, time: u64) {
    ui.flush(Duration::from_millis(time)).unwrap();
    for kind in [
        PointerKind::Press(PointerButton::Primary),
        PointerKind::Release(PointerButton::Primary),
    ] {
        ui.pointer_input(PointerEvent::new(Point::new(x, y).unwrap(), kind))
            .unwrap();
    }
    frame(ui);
}
/// Mounts `field` beside a button and focuses the field.
fn mount(ui: &mut Runtime, field: TextField) -> (WidgetId, WidgetId, WidgetId) {
    ui.set_viewport(Viewport::new(320.0, 160.0, 1.0).unwrap())
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
    let field = support::widget(ui, &root, "field");
    let other = support::widget(ui, &root, "other");
    ui.focus(&field).unwrap();
    frame(ui);
    (root, field, other)
}
/// The point over the end button of a field without a reveal button.
fn clear_point(ui: &Runtime, field: &WidgetId) -> (f32, f32) {
    let [x, y, w, h] = ui.bounds(field).unwrap().unwrap().values();
    (x + w - 20.0, y + h / 2.0)
}
fn shown(ui: &Runtime) -> InspectValue {
    details(ui, "field").properties["clear_shown"].clone()
}

#[test]
fn the_clear_button_empties_the_text_as_one_undoable_edit() {
    let mut ui = Runtime::new();
    let value = ui.signal("hello".to_owned());
    let (_, field, _) = mount(&mut ui, TextField::new(value.clone()).clearable(true));
    assert_eq!(shown(&ui), InspectValue::Bool(true));
    let (x, y) = clear_point(&ui, &field);
    click(&mut ui, x, y, 1000);
    // Without draft mode the cleared text is written at once, and focus stays.
    assert_eq!(value.get().unwrap(), "");
    assert_eq!(ui.focused(), Some(field.clone()));
    assert_eq!(shown(&ui), InspectValue::Bool(false));
    // A press where the hidden button was edits nothing.
    click(&mut ui, x, y, 2000);
    assert_eq!(value.get().unwrap(), "");
    // One undo restores the whole text.
    chord(&mut ui, KeyCode::primary(), KeyCode::KeyZ);
    assert_eq!(value.get().unwrap(), "hello");
}

#[test]
fn the_clear_button_is_offered_only_to_an_editable_field_with_text() {
    let mut ui = Runtime::new();
    let value = ui.signal("text".to_owned());
    let (root, field, _) = mount(
        &mut ui,
        TextField::new(value.clone()).clearable(true).read_only(),
    );
    assert_eq!(shown(&ui), InspectValue::Bool(false));
    let (x, y) = clear_point(&ui, &field);
    click(&mut ui, x, y, 1000);
    assert_eq!(value.get().unwrap(), "text");
    for (description, expected) in [
        (
            TextField::new(value.clone()).clearable(true).disabled(),
            false,
        ),
        (TextField::new(value.clone()).clearable(true), true),
        (TextField::new(value.clone()), false),
    ] {
        ui.reconcile_children(
            &root,
            vec![
                Element::new("field", description),
                Element::new("other", Button::new("Other", |_| Ok(()))),
            ],
        )
        .unwrap();
        frame(&mut ui);
        let d = details(&ui, "field");
        assert_eq!(
            d.properties
                .get("clear_shown")
                .cloned()
                .unwrap_or(InspectValue::Bool(false)),
            InspectValue::Bool(expected)
        );
    }
}

#[test]
fn clearing_a_draft_keeps_it_until_it_is_saved_and_works_when_concealed() {
    let mut ui = Runtime::new();
    let value = ui.signal("draft".to_owned());
    let (_, field, _) = mount(
        &mut ui,
        TextField::new(value.clone())
            .clearable(true)
            .max_length(Some(10)),
    );
    let (x, y) = clear_point(&ui, &field);
    click(&mut ui, x, y, 1000);
    // In draft mode the cleared text is a draft until Enter saves it.
    assert_eq!(value.get().unwrap(), "draft");
    assert_eq!(
        details(&ui, "field").properties["dirty"],
        InspectValue::Bool(true)
    );
    key(&mut ui, KeyCode::Enter);
    assert_eq!(value.get().unwrap(), "");

    let mut ui = Runtime::new();
    let secret = ui.signal("hunter2".to_owned());
    let (_, field, _) = mount(
        &mut ui,
        TextField::new(secret.clone())
            .concealed(true)
            .clearable(true),
    );
    assert_eq!(shown(&ui), InspectValue::Bool(true));
    let (x, y) = clear_point(&ui, &field);
    click(&mut ui, x, y, 1000);
    assert_eq!(secret.get().unwrap(), "");
    assert!(!details(&ui, "field").properties.contains_key("text"));
}

#[test]
fn a_leading_icon_and_a_suffix_narrow_the_text_and_stay_out_of_the_value() {
    let mut ui = Runtime::new();
    let value = ui.signal("x".repeat(200));
    let (root, field, _) = mount(&mut ui, TextField::new(value.clone()));
    key(&mut ui, KeyCode::End);
    let plain = match details(&ui, "field").properties["scroll_x"] {
        InspectValue::Number(n) => n,
        _ => panic!("no scroll"),
    };
    ui.reconcile_children(
        &root,
        vec![
            Element::new(
                "field",
                TextField::new(value.clone())
                    .leading_icon(Icon::Search)
                    .suffix("ms"),
            ),
            Element::new("other", Button::new("Other", |_| Ok(()))),
        ],
    )
    .unwrap();
    frame(&mut ui);
    ui.focus(&field).unwrap();
    key(&mut ui, KeyCode::End);
    let d = details(&ui, "field");
    assert_eq!(d.properties["suffix"], InspectValue::Text("ms".into()));
    assert_eq!(d.properties["leading_icon"], InspectValue::Bool(true));
    assert_eq!(
        d.semantics.unwrap().value,
        Some(SemanticValue::Text("x".repeat(200)))
    );
    // The narrower editing area scrolls further to show the caret at the end.
    let adorned = match d.properties["scroll_x"] {
        InspectValue::Number(n) => n,
        _ => panic!("no scroll"),
    };
    assert!(adorned > plain, "{adorned} > {plain}");
    // A press on the icon places the caret as anywhere else in the field.
    let [x, y, _, h] = ui.bounds(&field).unwrap().unwrap().values();
    click(&mut ui, x + 12.0, y + h / 2.0, 1000);
    assert_eq!(ui.focused(), Some(field.clone()));
    ui.text_input("a").unwrap();
    frame(&mut ui);
    assert_eq!(value.get().unwrap().len(), 201);
    assert!(ui.validation_report().issues.is_empty());
}

#[test]
fn the_modified_state_is_reported_once_per_change_and_counted_by_the_form() {
    let mut ui = Runtime::new();
    let form = FormStatus::new(&ui);
    let handle = form.field("name").unwrap();
    let value = ui.signal(String::new());
    let log = Rc::new(RefCell::new(Vec::new()));
    let (sink, report) = (log.clone(), handle.clone());
    let (_, field, other) = mount(
        &mut ui,
        TextField::new(value.clone())
            .max_length(Some(20))
            .modified_indicator(true)
            .on_modified(move |_, modified| {
                sink.borrow_mut().push(modified);
                report.set_modified(modified).map(|_| ())
            }),
    );
    let modified = |form: &FormStatus| form.summary().get().unwrap().modified();
    ui.text_input("Ad").unwrap();
    ui.text_input("a").unwrap();
    frame(&mut ui);
    // Two edits, one change.
    assert_eq!(*log.borrow(), [true]);
    assert_eq!(modified(&form), 1);
    let d = details(&ui, "field");
    assert_eq!(d.properties["dirty"], InspectValue::Bool(true));
    assert_eq!(d.properties["modified_indicator"], InspectValue::Bool(true));
    assert_eq!(
        d.semantics.unwrap().description.as_deref(),
        Some("Unsaved changes")
    );
    // Escape discards the draft.
    key(&mut ui, KeyCode::Escape);
    assert_eq!(*log.borrow(), [true, false]);
    assert_eq!(modified(&form), 0);
    assert_eq!(details(&ui, "field").semantics.unwrap().description, None);
    // Typing back to the saved text ends the change without a save.
    ui.text_input("b").unwrap();
    key(&mut ui, KeyCode::Backspace);
    assert_eq!(*log.borrow(), [true, false, true, false]);
    // A save on focus loss ends it.
    ui.focus(&field).unwrap();
    ui.text_input("Ada").unwrap();
    frame(&mut ui);
    ui.focus(&other).unwrap();
    frame(&mut ui);
    assert_eq!(value.get().unwrap(), "Ada");
    assert_eq!(*log.borrow(), [true, false, true, false, true, false]);
    // An application write discards a draft in an update; the change is
    // delivered later in the same flush.
    ui.focus(&field).unwrap();
    ui.text_input("!").unwrap();
    frame(&mut ui);
    assert_eq!(modified(&form), 1);
    value.set("Grace".into()).unwrap();
    frame(&mut ui);
    assert_eq!(log.borrow().last(), Some(&false));
    assert_eq!(modified(&form), 0);
    assert_eq!(log.borrow().len(), 8);
}

#[test]
fn the_unsaved_description_follows_the_installed_string_table() {
    let mut ui = Runtime::new();
    let value = ui.signal(String::new());
    mount(&mut ui, TextField::new(value).normalize(Ok));
    ui.text_input("draft").unwrap();
    frame(&mut ui);
    let table = UiStrings::english()
        .with(StringRole::TextModified, "Ej sparat")
        .unwrap();
    ui.ui_settings().set_strings(table).unwrap();
    frame(&mut ui);
    assert_eq!(
        details(&ui, "field")
            .semantics
            .unwrap()
            .description
            .as_deref(),
        Some("Ej sparat")
    );
}

#[test]
fn the_second_slice_options_apply_on_redescription() {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(320.0, 160.0, 1.0).unwrap())
        .unwrap();
    let value = ui.signal("abc".to_owned());
    let full = || {
        TextField::new(value.clone())
            .clearable(true)
            .leading_icon(Icon::Search)
            .suffix("ms")
            .max_length(Some(10))
            .modified_indicator(true)
    };
    let result = check_redescription(
        &mut ui,
        Element::new("c", TextField::new(value.clone())),
        Element::new("c", full()),
        Element::new("c", full()),
    );
    if let Err(error) = result {
        panic!("{error}: {error:?}");
    }
    // Withdrawing them again applies too.
    let mut ui = Runtime::new();
    let value = ui.signal("abc".to_owned());
    let result = check_redescription(
        &mut ui,
        Element::new(
            "c",
            TextField::new(value.clone())
                .clearable(true)
                .leading_icon(Icon::Search)
                .suffix("ms"),
        ),
        Element::new("c", TextField::new(value.clone())),
        Element::new("c", TextField::new(value.clone())),
    );
    if let Err(error) = result {
        panic!("{error}: {error:?}");
    }
}

#[test]
fn a_replaced_modified_callback_receives_later_changes() {
    let mut ui = Runtime::new();
    let value = ui.signal(String::new());
    let first = Rc::new(RefCell::new(Vec::new()));
    let second = Rc::new(RefCell::new(Vec::new()));
    let sink = first.clone();
    let (root, field, _) = mount(
        &mut ui,
        TextField::new(value.clone())
            .max_length(Some(5))
            .on_modified(move |_, m| {
                sink.borrow_mut().push(m);
                Ok(())
            }),
    );
    ui.text_input("a").unwrap();
    frame(&mut ui);
    let sink = second.clone();
    ui.reconcile_children(
        &root,
        vec![
            Element::new(
                "field",
                TextField::new(value.clone())
                    .max_length(Some(5))
                    .on_modified(move |_, m| {
                        sink.borrow_mut().push(m);
                        Ok(())
                    }),
            ),
            Element::new("other", Button::new("Other", |_| Ok(()))),
        ],
    )
    .unwrap();
    frame(&mut ui);
    ui.focus(&field).unwrap();
    key(&mut ui, KeyCode::Escape);
    assert_eq!(*first.borrow(), [true]);
    assert_eq!(*second.borrow(), [false]);
}
