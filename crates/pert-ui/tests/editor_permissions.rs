//! Editor availability and editability through the public facade: bound
//! `enabled` and `editable` permissions, observed at update, with the
//! revocation policy of the Permission and geometry design note.
use pert_ui::*;
mod support;
use std::time::Duration;
use support::{frame, key, settle};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn runtime() -> Result<Runtime, Box<dyn std::error::Error>> {
    let ui = Runtime::new();
    ui.set_viewport(Viewport::new(400.0, 200.0, 1.0)?)?;
    Ok(ui)
}

fn component(ui: &Runtime, key: &str) -> ComponentInspection {
    ui.inspect()
        .components
        .into_iter()
        .find(|c| c.key == key)
        .unwrap_or_else(|| panic!("no component {key}"))
}

fn shown_text(ui: &Runtime, key: &str) -> String {
    match &component(ui, key).details.properties["text"] {
        InspectValue::Text(text) => text.clone(),
        other => panic!("unexpected text {other:?}"),
    }
}

/// Mounts `field` beside a button and returns the root and the field.
fn mount(
    ui: &mut Runtime,
    field: Element,
) -> Result<(WidgetId, WidgetId), Box<dyn std::error::Error>> {
    let root = ui.mount_tree(Element::new(
        "root",
        Column::new([
            field,
            Element::new("after", Button::new("After", |_| Ok(()))),
        ]),
    ))?;
    frame(ui);
    let field = ui.children(&root)?[0].clone();
    Ok((root, field))
}

#[test]
fn revoking_a_text_field_mid_composition_keeps_its_draft_unsaved() -> TestResult {
    let mut ui = runtime()?;
    let value = ui.signal("old".to_owned());
    let saves = ui.signal(0_u32);
    let enabled = ui.signal(true);
    let saved = saves.clone();
    let (_, field) = mount(
        &mut ui,
        Element::new(
            "field",
            TextField::new(value.clone())
                .normalize(|text| Ok(text.trim().to_owned()))
                .on_save(move |_, _| saved.set(saved.get()? + 1).map(|_| ()))
                .enabled(enabled.read_only()),
        ),
    )?;
    ui.focus(&field)?;
    key(&mut ui, KeyCode::End);
    ui.text_input(" draft")?;
    ui.text_preedit(&TextPreedit::new("ime", None)?)?;
    frame(&mut ui);
    assert!(ui.text_input_state().is_some(), "composing");

    enabled.set(false)?;
    frame(&mut ui);
    assert_ne!(ui.focused(), Some(field.clone()), "focus released");
    assert!(ui.text_input_state().is_none(), "composition cancelled");
    assert_eq!(value.get()?, "old", "nothing saved, also not on focus loss");
    assert_eq!(saves.get()?, 0);
    assert_eq!(shown_text(&ui, "field"), "old draft", "the draft stays");
    // Input is refused while disabled.
    ui.focus(&field).ok();
    ui.text_input("x").ok();
    frame(&mut ui);
    assert_eq!(shown_text(&ui, "field"), "old draft");

    // Enabling again replays nothing; the draft saves on its next save.
    enabled.set(true)?;
    frame(&mut ui);
    assert_eq!(saves.get()?, 0);
    ui.focus(&field)?;
    key(&mut ui, KeyCode::Enter);
    assert_eq!(value.get()?, "old draft");
    assert_eq!(saves.get()?, 1);
    Ok(())
}

#[test]
fn revoking_editability_keeps_focus_and_selection_and_refuses_typing() -> TestResult {
    let mut ui = runtime()?;
    let value = ui.signal("old".to_owned());
    let editable = ui.signal(true);
    let (_, field) = mount(
        &mut ui,
        Element::new(
            "field",
            TextField::new(value.clone())
                .min_length(Some(1))
                .editable(editable.read_only()),
        ),
    )?;
    ui.focus(&field)?;
    key(&mut ui, KeyCode::End);
    ui.text_input(" draft")?;
    frame(&mut ui);
    editable.set(false)?;
    frame(&mut ui);
    assert_eq!(ui.focused(), Some(field.clone()), "focus stays");
    assert_eq!(
        component(&ui, "field").details.properties["read_only"],
        InspectValue::Bool(true)
    );
    ui.text_input("x")?;
    frame(&mut ui);
    assert_eq!(shown_text(&ui, "field"), "old draft", "typing refused");
    // Selection still works while read-only.
    ui.key_input(KeyCode::ShiftLeft, KeyAction::Press)?;
    key(&mut ui, KeyCode::Home);
    ui.key_input(KeyCode::ShiftLeft, KeyAction::Release)?;
    frame(&mut ui);
    let selection = ui.text_input_state();
    assert!(selection.is_none(), "no IME while read-only");
    // Focus loss saves nothing while editing is denied.
    ui.clear_focus();
    frame(&mut ui);
    assert_eq!(value.get()?, "old");
    assert_eq!(shown_text(&ui, "field"), "old draft");
    Ok(())
}

#[test]
fn a_bound_revocation_discards_a_pending_search_even_when_due() -> TestResult {
    for due in [false, true] {
        let mut ui = runtime()?;
        let query = ui.signal(String::new());
        let enabled = ui.signal(true);
        let (_, field) = mount(
            &mut ui,
            Element::new(
                "search",
                SearchField::new(query.clone())
                    .debounce(Duration::from_millis(200))
                    .enabled(enabled.read_only()),
            ),
        )?;
        ui.focus(&field)?;
        ui.text_input("pending")?;
        frame(&mut ui);
        enabled.set(false)?;
        // A due deadline is delivered before the update that observes the
        // revocation; it must not write either.
        let at = if due {
            Duration::from_secs(1)
        } else {
            ui.now()
        };
        ui.flush(at)?;
        ui.flush(Duration::from_secs(2))?;
        assert_eq!(query.get()?, "", "due={due}: nothing written");
        assert_eq!(
            shown_text(&ui, "search"),
            "",
            "due={due}: bound query shown"
        );
        enabled.set(true)?;
        settle(&mut ui);
        assert_eq!(query.get()?, "", "due={due}: nothing replayed");
    }
    Ok(())
}

#[test]
fn revoking_a_parameter_number_input_mid_step_commits_nothing() -> TestResult {
    let mut ui = runtime()?;
    let (voices, host) = ui.parameter(
        ParameterId::new(1, 1, "voices")?,
        ParameterSpec::stepped("Voices", 0, 16, 4)?,
    )?;
    let outcomes = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
    let log = outcomes.clone();
    let _monitor = host.on_outcome(move |event| {
        log.borrow_mut().push(event.outcome);
        Ok(())
    })?;
    let enabled = ui.signal(true);
    let (_, field) = mount(
        &mut ui,
        Element::new(
            "voices",
            InlineInteger::for_parameter(voices.clone()).enabled(enabled.read_only()),
        ),
    )?;
    ui.focus(&field)?;
    // A held step previews in a gesture session.
    ui.key_input(KeyCode::ArrowUp, KeyAction::Press)?;
    frame(&mut ui);
    enabled.set(false)?;
    frame(&mut ui);
    ui.key_input(KeyCode::ArrowUp, KeyAction::Release)?;
    frame(&mut ui);
    assert_eq!(voices.snapshot().get()?.base, 4, "nothing committed");
    assert!(
        !outcomes
            .borrow()
            .iter()
            .any(|outcome| matches!(outcome, EditOutcome::Committed { .. })),
        "{:?}",
        outcomes.borrow()
    );
    assert_ne!(ui.focused(), Some(field));
    Ok(())
}

#[test]
fn a_fixed_denial_is_not_lifted_and_redescription_replaces_the_permission() -> TestResult {
    let mut ui = runtime()?;
    let value = ui.signal("text".to_owned());
    let allowed = ui.signal(true);
    let root = ui.mount_tree(Element::new(
        "root",
        Column::new([Element::new(
            "field",
            TextField::new(value.clone())
                .disabled()
                .enabled(allowed.read_only()),
        )]),
    ))?;
    frame(&mut ui);
    let field = ui.children(&root)?[0].clone();
    ui.text_input("x").ok();
    let _ = ui.focus(&field);
    ui.text_input("x").ok();
    frame(&mut ui);
    assert_eq!(value.get()?, "text", "a disabled field refuses input");
    assert_eq!(
        component(&ui, "field").details.properties["enabled"],
        InspectValue::Bool(false)
    );

    // A new description replaces the old permission, including its denial.
    ui.reconcile_children(
        &root,
        vec![Element::new(
            "field",
            TextField::new(value.clone()).enabled(allowed.read_only()),
        )],
    )?;
    frame(&mut ui);
    assert_eq!(
        component(&ui, "field").details.properties["enabled"],
        InspectValue::Bool(true)
    );
    // A bound denial in the new description does not briefly grant.
    let denied = ui.signal(false);
    ui.reconcile_children(
        &root,
        vec![Element::new(
            "field",
            TextField::new(value.clone()).enabled(denied.read_only()),
        )],
    )?;
    frame(&mut ui);
    assert_eq!(
        component(&ui, "field").details.properties["enabled"],
        InspectValue::Bool(false)
    );
    Ok(())
}

#[test]
fn a_binding_from_another_runtime_fails_the_update() -> TestResult {
    let mut ui = runtime()?;
    let other = Runtime::new();
    let foreign = other.signal(true);
    let value = ui.signal(String::new());
    ui.mount_tree(Element::new(
        "field",
        TextField::new(value).editable(foreign.read_only()),
    ))?;
    assert_eq!(ui.flush(ui.now()), Err(RuntimeError::ForeignRuntime));
    Ok(())
}

#[test]
/// Enter in a field made read-only by a binding submits nothing; granted
/// again, the focused field's caret resumes and Enter saves and submits.
fn a_read_only_field_does_not_submit_its_unsaved_draft() -> TestResult {
    let mut ui = runtime()?;
    let value = ui.signal("old".to_owned());
    let editable = ui.signal(true);
    let submitted = ui.signal(Vec::<String>::new());
    let log = submitted.clone();
    let (_, field) = mount(
        &mut ui,
        Element::new(
            "field",
            TextField::new(value.clone())
                .min_length(Some(1))
                .editable(editable.read_only())
                .on_submit(move |_, text| {
                    let mut texts = log.get()?;
                    texts.push(text.to_owned());
                    log.set(texts).map(|_| ())
                }),
        ),
    )?;
    ui.focus(&field)?;
    key(&mut ui, KeyCode::End);
    ui.text_input(" draft")?;
    frame(&mut ui);
    editable.set(false)?;
    frame(&mut ui);
    key(&mut ui, KeyCode::Enter);
    frame(&mut ui);
    assert!(
        submitted.get()?.is_empty(),
        "nothing submitted while read-only"
    );
    assert_eq!(value.get()?, "old");
    // Granted again, Enter saves and submits the retained draft.
    editable.set(true)?;
    frame(&mut ui);
    key(&mut ui, KeyCode::Enter);
    frame(&mut ui);
    assert_eq!(value.get()?, "old draft");
    assert_eq!(submitted.get()?, ["old draft"]);
    Ok(())
}
