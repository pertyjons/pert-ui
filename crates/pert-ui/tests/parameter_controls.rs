//! Standard and inline controls bound to boolean, choice and stepped parameters
//! through the public facade: one begin/commit per change, spec-supplied
//! domains, cancellation, shared bindings and unavailable/faulted states.
use std::{cell::RefCell, rc::Rc};

use pert_ui::*;
mod support;
use support::{frame, key};

type TestResult = Result<(), Box<dyn std::error::Error>>;
type Log<T> = Rc<RefCell<Vec<EditOutcome<T>>>>;

/// Mounts `control` above a focusable neighbor and returns the control's id.
fn mount(
    ui: &mut Runtime,
    control: impl Component + 'static,
) -> Result<WidgetId, Box<dyn std::error::Error>> {
    ui.set_viewport(Viewport::new(600.0, 400.0, 1.0)?)?;
    let root = ui.mount_tree(Element::new(
        "root",
        Column::new([
            Element::new("control", control),
            Element::new("next", Button::new("Next", |_| Ok(()))),
        ]),
    ))?;
    frame(ui);
    Ok(ui.children(&root)?[0].clone())
}
fn semantics(ui: &Runtime, key: &str) -> Result<Semantics, Box<dyn std::error::Error>> {
    ui.inspect()
        .components
        .into_iter()
        .find(|c| c.key == key)
        .and_then(|c| c.details.semantics)
        .ok_or_else(|| format!("no semantics for {key}").into())
}
fn outcomes<T: ParameterValue>(
    host: &ParameterHost<T>,
) -> Result<(Log<T>, DeliveryMonitor<T>), RuntimeError> {
    let log: Log<T> = Rc::default();
    let sink = log.clone();
    let monitor = host.on_outcome(move |event| {
        sink.borrow_mut().push(event.outcome.clone());
        Ok(())
    })?;
    Ok((log, monitor))
}
fn id(text: &str) -> Result<ChoiceId, ParameterError> {
    ChoiceId::new(text)
}
fn waveforms() -> Result<ParameterSpec<ChoiceId>, ParameterError> {
    ParameterSpec::choices(
        "Waveform",
        [
            ChoiceOption::new(id("sine")?, "Sine"),
            ChoiceOption::new(id("saw")?, "Saw"),
            ChoiceOption::new(id("square")?, "Square"),
        ],
        id("sine")?,
    )
}

#[test]
fn toggle_activation_is_one_commit_and_follows_the_parameter_name() -> TestResult {
    let mut ui = Runtime::new();
    let (bypass, host) = ui.parameter(
        ParameterId::new(1, 1, "bypass")?,
        ParameterSpec::boolean("Bypass", false)?,
    )?;
    let (log, _monitor) = outcomes(&host)?;
    let toggle = mount(&mut ui, Toggle::for_parameter(bypass.clone()))?;
    assert_eq!(semantics(&ui, "control")?.name.as_deref(), Some("Bypass"));
    ui.focus(&toggle)?;
    key(&mut ui, KeyCode::Space);
    assert!(bypass.snapshot().get()?.base);
    assert!(matches!(
        log.borrow().as_slice(),
        [EditOutcome::Committed { value: true, .. }]
    ));
    // OS repeats never toggle; a new press/release toggles back.
    ui.key_input(KeyCode::Space, KeyAction::Press)?;
    ui.key_input(KeyCode::Space, KeyAction::Repeat)?;
    ui.key_input(KeyCode::Space, KeyAction::Release)?;
    assert!(!bypass.snapshot().get()?.base);
    assert_eq!(log.borrow().len(), 2);

    // A renamed specification relabels the control.
    host.replace_spec(ParameterSpec::boolean("Bypass all", false)?)?;
    frame(&mut ui);
    assert_eq!(
        semantics(&ui, "control")?.name.as_deref(),
        Some("Bypass all")
    );
    Ok(())
}

#[test]
fn a_revision_change_during_activation_cancels_instead_of_retargeting() -> TestResult {
    let mut ui = Runtime::new();
    let (mute, host) = ui.parameter(
        ParameterId::new(1, 1, "mute")?,
        ParameterSpec::boolean("Mute", false)?,
    )?;
    let (log, _monitor) = outcomes(&host)?;
    let checkbox = mount(
        &mut ui,
        Checkbox::for_parameter(mute.clone())
            .on_change(|_, _| panic!("stale activation must not report a write")),
    )?;
    ui.focus(&checkbox)?;
    ui.key_input(KeyCode::Space, KeyAction::Press)?;
    host.write_base(true)?; // an external change arrives mid-press
    frame(&mut ui);
    ui.key_input(KeyCode::Space, KeyAction::Release)?;
    assert!(mute.snapshot().get()?.base);
    assert!(log.borrow().is_empty());
    Ok(())
}

#[test]
fn read_only_unavailable_and_faulted_parameters_disable_the_control() -> TestResult {
    let mut ui = Runtime::new();
    let (solo, host) = ui.parameter(
        ParameterId::new(1, 1, "solo")?,
        ParameterSpec::boolean("Solo", false)?.read_only(),
    )?;
    let toggle = mount(&mut ui, InlineToggle::for_parameter(solo.clone()))?;
    // Not focusable while read-only: Tab skips it to the neighbor.
    ui.focus_next(false)?;
    assert_ne!(ui.focused().as_ref(), Some(&toggle));
    key(&mut ui, KeyCode::Space);
    assert!(!solo.snapshot().get()?.base);
    let semantics_now = semantics(&ui, "control")?;
    assert_eq!(semantics_now.name.as_deref(), Some("Solo"));
    assert!(semantics_now.disabled);

    // Editable again, then faulted by a failing sink.
    host.replace_spec(ParameterSpec::boolean("Solo", false)?)?;
    let _monitor = host.on_outcome(|_| Err(SinkError::new(SinkErrorKind::Full, "full")))?;
    frame(&mut ui);
    ui.focus(&toggle)?;
    key(&mut ui, KeyCode::Space);
    assert!(solo.snapshot().get()?.base);
    assert_eq!(solo.delivery_status().get()?, DeliveryStatus::Faulted);
    frame(&mut ui);
    assert!(semantics(&ui, "control")?.disabled);
    key(&mut ui, KeyCode::Space);
    assert!(solo.snapshot().get()?.base);

    drop(host);
    frame(&mut ui);
    assert!(semantics(&ui, "control")?.disabled);
    Ok(())
}

#[test]
fn segmented_choices_come_from_the_spec_and_keep_identity_through_reorder() -> TestResult {
    let mut ui = Runtime::new();
    let (waveform, host) = ui.parameter(ParameterId::new(1, 1, "waveform")?, waveforms()?)?;
    let (log, _monitor) = outcomes(&host)?;
    let group = mount(&mut ui, SegmentedControl::for_parameter(waveform.clone()))?;
    let items = semantics(&ui, "control")?.items;
    assert_eq!(
        items
            .iter()
            .map(|item| item.name.as_deref().unwrap_or_default())
            .collect::<Vec<_>>(),
        ["Sine", "Saw", "Square"]
    );
    ui.focus(&group)?;
    ui.key_input(KeyCode::ArrowRight, KeyAction::Press)?;
    assert_eq!(waveform.snapshot().get()?.base, id("saw")?);
    // Selecting the current option is a no-op: no edit, no event.
    key(&mut ui, KeyCode::Space);
    assert_eq!(log.borrow().len(), 1);

    host.replace_spec(ParameterSpec::choices(
        "Waveform",
        [
            ChoiceOption::new(id("square")?, "Square"),
            ChoiceOption::new(id("saw")?, "Sawtooth"),
            ChoiceOption::new(id("sine")?, "Sine"),
        ],
        id("sine")?,
    )?)?;
    frame(&mut ui);
    let semantics_now = semantics(&ui, "control")?;
    assert_eq!(
        semantics_now.value,
        Some(SemanticValue::Choice {
            selected: Some(1),
            count: 3,
            label: Some("Sawtooth".into())
        })
    );
    ui.key_input(KeyCode::Home, KeyAction::Press)?;
    assert_eq!(waveform.snapshot().get()?.base, id("square")?);
    assert_eq!(log.borrow().len(), 2);
    Ok(())
}

#[test]
fn radio_and_inline_choice_share_one_parameter() -> TestResult {
    let mut ui = Runtime::new();
    let (waveform, host) = ui.parameter(ParameterId::new(1, 1, "waveform")?, waveforms()?)?;
    let (log, _monitor) = outcomes(&host)?;
    ui.set_viewport(Viewport::new(600.0, 400.0, 1.0)?)?;
    let root = ui.mount_tree(Element::new(
        "root",
        Column::new([
            Element::new("radio", RadioGroup::for_parameter(waveform.clone())),
            Element::new("inline", InlineChoice::for_parameter(waveform.clone())),
        ]),
    ))?;
    frame(&mut ui);
    let children = ui.children(&root)?;
    ui.focus(&children[0])?;
    ui.key_input(KeyCode::End, KeyAction::Press)?;
    frame(&mut ui);
    assert_eq!(
        semantics(&ui, "inline")?.value,
        Some(SemanticValue::Choice {
            selected: Some(2),
            count: 3,
            label: Some("Square".into())
        })
    );
    ui.focus(&children[1])?;
    ui.key_input(KeyCode::ArrowLeft, KeyAction::Press)?;
    assert_eq!(waveform.snapshot().get()?.base, id("saw")?);
    assert_eq!(log.borrow().len(), 2);
    assert_eq!(semantics(&ui, "radio")?.name.as_deref(), Some("Waveform"));
    Ok(())
}

#[test]
fn select_commits_once_and_refuses_a_choice_overtaken_by_a_revision() -> TestResult {
    let mut ui = Runtime::new();
    let (waveform, host) = ui.parameter(ParameterId::new(1, 1, "waveform")?, waveforms()?)?;
    let (log, _monitor) = outcomes(&host)?;
    let calls = Rc::new(RefCell::new(Vec::new()));
    let sink = calls.clone();
    let select = Select::for_parameter(&ui, waveform.clone()).on_change(move |_, value| {
        sink.borrow_mut().push(value.clone());
        Ok(())
    });
    mount(&mut ui, select)?;
    ui.focus_next(false)?;
    ui.key_input(KeyCode::ArrowDown, KeyAction::Press)?; // opens the list
    frame(&mut ui);
    ui.key_input(KeyCode::ArrowDown, KeyAction::Press)?; // highlight only
    assert_eq!(waveform.snapshot().get()?.base, id("sine")?);
    key(&mut ui, KeyCode::Enter);
    frame(&mut ui);
    assert_eq!(waveform.snapshot().get()?.base, id("saw")?);
    assert_eq!(log.borrow().len(), 1);

    // Open, then an external write lands before the choice.
    ui.focus_next(false)?;
    ui.focus_next(true)?;
    ui.key_input(KeyCode::ArrowDown, KeyAction::Press)?;
    frame(&mut ui);
    host.write_base(id("sine")?)?;
    ui.key_input(KeyCode::ArrowDown, KeyAction::Press)?;
    ui.key_input(KeyCode::ArrowDown, KeyAction::Press)?;
    key(&mut ui, KeyCode::Enter);
    assert_eq!(waveform.snapshot().get()?.base, id("sine")?);
    assert_eq!(log.borrow().len(), 1);
    assert_eq!(*calls.borrow(), [id("saw")?]);
    Ok(())
}

#[test]
fn integer_drafts_validate_exactly_and_cancel_on_focus_loss() -> TestResult {
    let mut ui = Runtime::new();
    let (voices, host) = ui.parameter(
        ParameterId::new(1, 1, "voices")?,
        ParameterSpec::stepped("Voices", 0, 16, 4)?
            .with_step(std::num::NonZeroU64::new(2).ok_or("zero")?)?,
    )?;
    let (log, _monitor) = outcomes(&host)?;
    assert_eq!(
        IntegerInput::for_parameter(voices.clone())
            .range(0, 3)
            .err(),
        Some(NumberInputError::ParameterDomain)
    );
    let field = mount(&mut ui, IntegerInput::for_parameter(voices.clone()))?;
    ui.focus(&field)?;
    ui.key_input(KeyCode::End, KeyAction::Press)?;
    ui.text_input("1")?; // "41": out of range
    key(&mut ui, KeyCode::Enter);
    assert_eq!(voices.snapshot().get()?.base, 4);
    for _ in 0..2 {
        ui.key_input(KeyCode::Backspace, KeyAction::Press)?;
        ui.key_input(KeyCode::Backspace, KeyAction::Release)?;
    }
    ui.text_input("7")?; // off the lattice: rejected, never rounded
    key(&mut ui, KeyCode::Enter);
    frame(&mut ui);
    assert_eq!(voices.snapshot().get()?.base, 4);
    assert!(semantics(&ui, "control")?.invalid);
    ui.key_input(KeyCode::Backspace, KeyAction::Press)?;
    ui.key_input(KeyCode::Backspace, KeyAction::Release)?;
    ui.text_input("8")?;
    key(&mut ui, KeyCode::Enter);
    assert_eq!(voices.snapshot().get()?.base, 8);
    assert!(matches!(
        log.borrow().as_slice(),
        [EditOutcome::Committed { value: 8, .. }]
    ));
    // Enter on an unchanged draft creates no edit.
    key(&mut ui, KeyCode::Enter);
    assert_eq!(log.borrow().len(), 1);

    // Focus loss cancels a parameter draft instead of saving it.
    key(&mut ui, KeyCode::ArrowUp);
    ui.clear_focus();
    frame(&mut ui);
    assert_eq!(voices.snapshot().get()?.base, 8);
    assert_eq!(log.borrow().len(), 1);

    // An external write discards a pending draft.
    ui.focus(&field)?;
    key(&mut ui, KeyCode::ArrowUp);
    host.write_base(2)?;
    frame(&mut ui);
    key(&mut ui, KeyCode::Enter);
    assert_eq!(voices.snapshot().get()?.base, 2);
    assert_eq!(log.borrow().len(), 1);
    Ok(())
}

#[test]
fn inline_integer_gestures_own_one_session_until_release() -> TestResult {
    let mut ui = Runtime::new();
    let (octave, host) = ui.parameter(
        ParameterId::new(1, 1, "octave")?,
        ParameterSpec::stepped("Octave", -3, 3, 0)?,
    )?;
    let (log, _monitor) = outcomes(&host)?;
    let previews = Rc::new(RefCell::new(Vec::new()));
    let seen = previews.clone();
    host.on_preview(move |event| {
        seen.borrow_mut().push(event.value);
        Ok(())
    })?;
    let row = mount(
        &mut ui,
        InlineInteger::for_parameter(octave.clone()).label("Oct"),
    )?;
    ui.focus(&row)?;
    ui.key_input(KeyCode::ArrowUp, KeyAction::Press)?;
    ui.key_input(KeyCode::ArrowUp, KeyAction::Repeat)?;
    ui.key_input(KeyCode::ArrowUp, KeyAction::Repeat)?;
    // Previews only: the base has not changed yet.
    assert_eq!(octave.snapshot().get()?.base, 0);
    assert_eq!(octave.snapshot().get()?.preview, Some(3));
    ui.key_input(KeyCode::ArrowUp, KeyAction::Release)?;
    assert_eq!(octave.snapshot().get()?.base, 3);
    assert_eq!(previews.borrow().as_slice(), [1, 2, 3]);
    assert!(matches!(
        log.borrow().as_slice(),
        [EditOutcome::Committed { value: 3, .. }]
    ));
    // Escape cancels a held gesture as a whole.
    frame(&mut ui);
    ui.key_input(KeyCode::ArrowDown, KeyAction::Press)?;
    ui.key_input(KeyCode::ArrowDown, KeyAction::Repeat)?;
    ui.key_input(KeyCode::Escape, KeyAction::Press)?;
    ui.key_input(KeyCode::ArrowDown, KeyAction::Release)?;
    assert_eq!(octave.snapshot().get()?.base, 3);
    assert_eq!(log.borrow().len(), 2);
    assert_eq!(log.borrow()[1], EditOutcome::Cancelled);
    // A focused wheel event is one complete edit.
    ui.key_input(KeyCode::Escape, KeyAction::Release)?;
    frame(&mut ui);
    let [x, y, w, h] = ui.bounds(&row)?.ok_or("arranged")?.values();
    ui.pointer_input(PointerEvent::new(
        Point::new(x + w / 2.0, y + h / 2.0)?,
        PointerKind::Scroll(Point::new(0.0, 1.0)?),
    ))?;
    assert_eq!(octave.snapshot().get()?.base, 2);
    assert_eq!(log.borrow().len(), 3);
    Ok(())
}

#[test]
fn cancelled_inline_held_key_does_not_block_the_next_gesture() -> TestResult {
    let mut ui = Runtime::new();
    let (value, host) = ui.parameter(
        ParameterId::new(1, 1, "count")?,
        ParameterSpec::stepped("Count", -10, 10, 0)?,
    )?;
    let (log, _monitor) = outcomes(&host)?;
    let row = mount(&mut ui, InlineInteger::for_parameter(value.clone()))?;
    ui.focus(&row)?;
    ui.command_key_input(CommandKeyInput::new(
        KeyCode::ArrowUp,
        KeyAction::Press,
        CommandKey::Named(KeyCode::ArrowUp),
    ))?;
    assert_eq!(value.snapshot().get()?.preview, Some(1));
    ui.cancel_input()?;
    ui.command_key_input(CommandKeyInput::new(
        KeyCode::ArrowUp,
        KeyAction::Release,
        CommandKey::Named(KeyCode::ArrowUp),
    ))?;
    assert_eq!(value.snapshot().get()?.base, 0);
    assert_eq!(value.snapshot().get()?.preview, None);
    ui.command_key_input(CommandKeyInput::new(
        KeyCode::ArrowDown,
        KeyAction::Press,
        CommandKey::Named(KeyCode::ArrowDown),
    ))?;
    ui.command_key_input(CommandKeyInput::new(
        KeyCode::ArrowDown,
        KeyAction::Release,
        CommandKey::Named(KeyCode::ArrowDown),
    ))?;
    assert_eq!(value.snapshot().get()?.base, -1);
    assert!(matches!(
        log.borrow().as_slice(),
        [
            EditOutcome::Cancelled,
            EditOutcome::Committed { value: -1, .. }
        ]
    ));
    Ok(())
}

#[test]
fn parameter_stepper_escape_releases_the_session_and_draft() -> TestResult {
    let mut ui = Runtime::new();
    let (value, host) = ui.parameter(
        ParameterId::new(1, 1, "count")?,
        ParameterSpec::stepped("Count", 0, 10, 0)?,
    )?;
    let (log, _monitor) = outcomes(&host)?;
    let row = mount(
        &mut ui,
        IntegerInput::for_parameter(value.clone()).stepper(true),
    )?;
    let [x, y, w, h] = ui.bounds(&row)?.ok_or("no bounds")?.values();
    let metrics = ui.theme().metrics(Density::Standard)?;
    let plus = Point::new(
        x + w
            - metrics.get(MetricRole::ControlPaddingX) * 0.5
            - metrics.get(MetricRole::IconSize) * 0.5,
        y + h * 0.5,
    )?;
    ui.pointer_input(PointerEvent::new(
        plus,
        PointerKind::Press(PointerButton::Primary),
    ))?;
    assert_eq!(value.snapshot().get()?.preview, Some(1));
    for action in [KeyAction::Press, KeyAction::Release] {
        ui.command_key_input(CommandKeyInput::new(
            KeyCode::Escape,
            action,
            CommandKey::Named(KeyCode::Escape),
        ))?;
    }
    ui.pointer_input(PointerEvent::new(
        plus,
        PointerKind::Release(PointerButton::Primary),
    ))?;
    assert_eq!(value.snapshot().get()?.base, 0);
    assert_eq!(value.snapshot().get()?.preview, None);
    ui.pointer_input(PointerEvent::new(
        plus,
        PointerKind::Press(PointerButton::Primary),
    ))?;
    ui.pointer_input(PointerEvent::new(
        plus,
        PointerKind::Release(PointerButton::Primary),
    ))?;
    assert_eq!(value.snapshot().get()?.base, 1);
    assert!(matches!(
        log.borrow().as_slice(),
        [
            EditOutcome::Cancelled,
            EditOutcome::Committed { value: 1, .. }
        ]
    ));
    Ok(())
}

#[test]
fn wide_lattices_step_exactly_and_never_leave_a_session_open() -> TestResult {
    let mut ui = Runtime::new();
    // A step above i64::MAX cannot be expressed as an i64 increment.
    let (value, host) = ui.parameter(
        ParameterId::new(1, 1, "wide")?,
        ParameterSpec::stepped("Wide", i64::MIN, 0, i64::MIN)?
            .with_step(std::num::NonZeroU64::new(1 << 63).ok_or("zero")?)?,
    )?;
    let (log, _monitor) = outcomes(&host)?;
    let row = mount(&mut ui, InlineInteger::for_parameter(value.clone()))?;
    ui.focus(&row)?;
    key(&mut ui, KeyCode::ArrowUp);
    let snapshot = value.snapshot().get()?;
    assert_eq!((snapshot.base, snapshot.edit_source), (0, None));
    assert_eq!(log.borrow().len(), 1);
    Ok(())
}

#[test]
fn continuous_controls_are_disabled_while_delivery_is_faulted() -> TestResult {
    let mut ui = Runtime::new();
    let (gain, host) = ui.parameter(
        ParameterId::new(1, 1, "gain")?,
        ParameterSpec::new("Gain", 0.0, 1.0, 0.0)?,
    )?;
    let _monitor = host.on_outcome(|_| Err(SinkError::new(SinkErrorKind::Closed, "gone")))?;
    let slider = mount(&mut ui, Slider::new(gain.clone()))?;
    ui.focus(&slider)?;
    key(&mut ui, KeyCode::ArrowRight);
    assert_eq!(gain.delivery_status().get()?, DeliveryStatus::Faulted);
    frame(&mut ui);
    assert!(semantics(&ui, "control")?.disabled);
    Ok(())
}

fn remote_pending_control<T: ParameterValue, C: Component + 'static>(
    spec: ParameterSpec<T>,
    make: impl FnOnce(Parameter<T>) -> C,
    actions: &[KeyCode],
    presented: impl Fn(&Semantics, &T),
) -> TestResult {
    let mut ui = Runtime::new();
    ui.set_debug_mode(DebugMode::Inspect);
    let (parameter, host) = ui.remote_parameter(ParameterId::new(1, 1, "remote")?, spec)?;
    let original = parameter.snapshot().get()?.base;
    let control = mount(&mut ui, make(parameter.clone()))?;
    ui.focus(&control)?;
    for &action in actions {
        key(&mut ui, action);
    }
    frame(&mut ui);
    let request = host.request().expect("input submitted a proposal");
    assert_eq!(parameter.snapshot().get()?.base, original);
    assert_eq!(
        parameter.snapshot().get()?.preview,
        Some(request.value().clone())
    );
    let semantics_now = semantics(&ui, "control")?;
    presented(&semantics_now, request.value());
    assert!(
        semantics_now.disabled || semantics_now.read_only,
        "pending control must refuse new edits: {semantics_now:?}"
    );
    assert!(!semantics_now.invalid, "pending is not an input error");
    ui.clear_event_history();
    ui.focus(&control)?;
    for &action in actions {
        key(&mut ui, action);
    }
    assert!(
        !ui.inspect()
            .events
            .iter()
            .any(|event| event.parameter.as_ref() == Some(parameter.id())
                && event.kind == EventKind::EditBegin),
        "pending input never attempts parameter admission"
    );
    assert_eq!(
        host.request(),
        Some(request.clone()),
        "a new gesture cannot replace the pending request"
    );
    assert!(!semantics(&ui, "control")?.invalid);
    host.respond(&request, AuthorityResponse::Rejected)?;
    frame(&mut ui);
    let restored = semantics(&ui, "control")?;
    assert!(!restored.disabled && !restored.read_only);
    presented(&restored, &original);
    ui.focus(&control)?;
    for &action in actions {
        key(&mut ui, action);
    }
    let request = host.request().expect("editing resumed");
    let receipt = host.receipt().expect("retained receipt");
    host.disconnect()?;
    frame(&mut ui);
    assert!(matches!(receipt.status(), AuthorityStatus::Unknown(_)));
    let disconnected = semantics(&ui, "control")?;
    assert!(disconnected.disabled || disconnected.read_only);
    host.resynchronize(original.clone())?;
    frame(&mut ui);
    assert!(matches!(receipt.status(), AuthorityStatus::Unknown(_)));
    assert_eq!(
        host.respond(
            &request,
            AuthorityResponse::Accepted(request.value().clone())
        )?,
        ResponseDisposition::Stale
    );
    let recovered = semantics(&ui, "control")?;
    assert!(!recovered.disabled && !recovered.read_only);
    presented(&recovered, &original);
    Ok(())
}
fn numeric_presented(semantics: &Semantics, expected: f64) {
    let Some(SemanticValue::Number { value, .. }) = semantics.value else {
        panic!("numeric semantics")
    };
    assert!(
        (value - expected).abs() < 1e-6,
        "displayed {value}, expected {expected}"
    );
}
#[test]
fn remote_continuous_controls_display_proposals_and_disable_pending_edits() -> TestResult {
    let spec = || ParameterSpec::new("Remote amount", 0.0, 1.0, 0.25);
    remote_pending_control(spec()?, Slider::new, &[KeyCode::ArrowRight], |s, v| {
        numeric_presented(s, f64::from(*v))
    })?;
    remote_pending_control(spec()?, Knob::new, &[KeyCode::ArrowUp], |s, v| {
        numeric_presented(s, f64::from(*v))
    })?;
    remote_pending_control(
        spec()?,
        InlineSlider::new,
        &[KeyCode::ArrowRight],
        |s, v| numeric_presented(s, f64::from(*v)),
    )?;
    Ok(())
}
#[test]
fn remote_integer_fields_keep_the_submitted_value_and_disable_pending_edits() -> TestResult {
    let spec = || ParameterSpec::stepped("Remote integer", 0, 10, 4);
    remote_pending_control(
        spec()?,
        IntegerInput::for_parameter,
        &[KeyCode::ArrowUp, KeyCode::Enter],
        |s, v| numeric_presented(s, *v as f64),
    )?;
    remote_pending_control(
        spec()?,
        InlineInteger::for_parameter,
        &[KeyCode::ArrowUp],
        |s, v| numeric_presented(s, *v as f64),
    )?;
    Ok(())
}
#[test]
fn remote_boolean_and_choice_controls_disable_pending_edits() -> TestResult {
    remote_pending_control(
        ParameterSpec::boolean("Remote toggle", false)?,
        Toggle::for_parameter,
        &[KeyCode::Space],
        |s, v| assert_eq!(s.value, Some(SemanticValue::Bool(*v))),
    )?;
    remote_pending_control(
        ParameterSpec::boolean("Remote checkbox", false)?,
        Checkbox::for_parameter,
        &[KeyCode::Space],
        |s, v| assert_eq!(s.value, Some(SemanticValue::Bool(*v))),
    )?;
    let choice = |s: &Semantics, v: &ChoiceId| {
        let Some(SemanticValue::Choice { label, .. }) = &s.value else {
            panic!("choice semantics")
        };
        assert_eq!(
            label.as_deref(),
            Some(match v.as_str() {
                "sine" => "Sine",
                "saw" => "Saw",
                "square" => "Square",
                _ => panic!("unknown choice"),
            })
        );
    };
    remote_pending_control(
        waveforms()?,
        RadioGroup::for_parameter,
        &[KeyCode::ArrowRight],
        choice,
    )?;
    remote_pending_control(
        waveforms()?,
        SegmentedControl::for_parameter,
        &[KeyCode::ArrowRight],
        choice,
    )?;
    remote_pending_control(
        waveforms()?,
        InlineChoice::for_parameter,
        &[KeyCode::ArrowRight],
        choice,
    )?;
    Ok(())
}

#[test]
fn remote_choice_callbacks_report_submission_once_without_claiming_acceptance() -> TestResult {
    fn check<C: Component + 'static>(
        make: impl FnOnce(Parameter<ChoiceId>, Rc<RefCell<Vec<ChoiceId>>>) -> C,
    ) -> TestResult {
        let mut ui = Runtime::new();
        let (parameter, host) =
            ui.remote_parameter(ParameterId::new(1, 1, "callback")?, waveforms()?)?;
        let calls = Rc::new(RefCell::new(Vec::new()));
        let control = mount(&mut ui, make(parameter.clone(), calls.clone()))?;
        ui.focus(&control)?;
        key(&mut ui, KeyCode::ArrowRight);
        let request = host.request().expect("submitted");
        assert_eq!(calls.borrow().as_slice(), [request.value().clone()]);
        assert_eq!(parameter.snapshot().get()?.base, id("sine")?);
        host.respond(&request, AuthorityResponse::Accepted(id("square")?))?;
        frame(&mut ui);
        assert_eq!(parameter.snapshot().get()?.base, id("square")?);
        assert_eq!(
            calls.borrow().len(),
            1,
            "correction is not another user callback"
        );
        ui.focus(&control)?;
        key(&mut ui, KeyCode::ArrowLeft);
        let request = host.request().expect("second proposal");
        assert_eq!(calls.borrow().len(), 2);
        host.respond(&request, AuthorityResponse::Rejected)?;
        frame(&mut ui);
        assert_eq!(
            calls.borrow().len(),
            2,
            "rejection is not another user callback"
        );
        Ok(())
    }
    check(|parameter, calls| {
        RadioGroup::for_parameter(parameter).on_change(move |_, value| {
            calls.borrow_mut().push(value.clone());
            Ok(())
        })
    })?;
    check(|parameter, calls| {
        SegmentedControl::for_parameter(parameter).on_change(move |_, value| {
            calls.borrow_mut().push(value.clone());
            Ok(())
        })
    })?;
    check(|parameter, calls| {
        InlineChoice::for_parameter(parameter).on_change(move |_, value| {
            calls.borrow_mut().push(value.clone());
            Ok(())
        })
    })?;
    Ok(())
}

#[test]
fn remote_integer_save_callback_reports_proposal_then_follows_canonical_response() -> TestResult {
    let mut ui = Runtime::new();
    let (parameter, host) = ui.remote_parameter(
        ParameterId::new(1, 1, "integer-save")?,
        ParameterSpec::stepped("Save", 0, 10, 4)?,
    )?;
    let calls = Rc::new(RefCell::new(Vec::new()));
    let recorded = calls.clone();
    let field = mount(
        &mut ui,
        IntegerInput::for_parameter(parameter.clone()).on_save(move |_, value| {
            recorded.borrow_mut().push(value);
            Ok(())
        }),
    )?;
    ui.focus(&field)?;
    key(&mut ui, KeyCode::ArrowUp);
    key(&mut ui, KeyCode::Enter);
    let request = host.request().expect("integer save submitted");
    assert_eq!(parameter.snapshot().get()?.base, 4);
    assert_eq!(calls.borrow().as_slice(), [*request.value()]);
    numeric_presented(&semantics(&ui, "control")?, *request.value() as f64);
    host.respond(&request, AuthorityResponse::Accepted(8))?;
    frame(&mut ui);
    assert_eq!(parameter.snapshot().get()?.base, 8);
    numeric_presented(&semantics(&ui, "control")?, 8.0);
    assert_eq!(
        calls.borrow().len(),
        1,
        "authority correction is not a save callback"
    );
    Ok(())
}
#[test]
fn a_failing_choice_callback_cannot_undo_a_retained_remote_submission() -> TestResult {
    let mut ui = Runtime::new();
    let (parameter, host) =
        ui.remote_parameter(ParameterId::new(1, 1, "failing-choice")?, waveforms()?)?;
    let error = RuntimeError::from(ApplicationError::from("settings callback failed"));
    let failure = error.clone();
    let control = mount(
        &mut ui,
        SegmentedControl::for_parameter(parameter.clone())
            .on_change(move |_, _| Err(failure.clone())),
    )?;
    ui.focus(&control)?;
    assert_eq!(
        ui.key_input(KeyCode::ArrowRight, KeyAction::Press).err(),
        Some(error)
    );
    let request = host
        .request()
        .expect("submission survives callback failure");
    assert_eq!(parameter.snapshot().get()?.base, id("sine")?);
    assert_eq!(
        parameter.snapshot().get()?.preview,
        Some(request.value().clone())
    );
    host.respond(&request, AuthorityResponse::Accepted(id("square")?))?;
    assert_eq!(parameter.snapshot().get()?.base, id("square")?);
    Ok(())
}

#[test]
fn inline_integer_pointer_previews_survive_frames_and_release_local_or_remote() -> TestResult {
    for remote in [false, true] {
        let mut ui = Runtime::new();
        ui.set_viewport(Viewport::new(300.0, 200.0, 1.0)?)?;
        let spec = ParameterSpec::stepped("Count", 0, 100, 4)?;
        let id = ParameterId::new(1, 1, "count")?;
        let (parameter, local, authority) = if remote {
            let (parameter, host) = ui.remote_parameter(id, spec)?;
            (parameter, None, Some(host))
        } else {
            let (parameter, host) = ui.parameter(id, spec)?;
            (parameter, Some(host), None)
        };
        let local = local
            .as_ref()
            .or_else(|| authority.as_ref().map(RemoteHost::local_host))
            .ok_or("host")?;
        let (terminal, _monitor) = outcomes(local)?;
        let previews = Rc::new(RefCell::new(Vec::new()));
        let recorded = previews.clone();
        local.on_preview(move |event| {
            recorded.borrow_mut().push(event.value);
            Ok(())
        })?;
        let clicks = Rc::new(std::cell::Cell::new(0));
        let clicked = clicks.clone();
        let root = ui.mount_tree(Element::new(
            "root",
            Column::new([
                Element::new("control", InlineInteger::for_parameter(parameter.clone())),
                Element::new(
                    "next",
                    Button::new("Next", move |_| {
                        clicked.set(clicked.get() + 1);
                        Ok(())
                    }),
                ),
            ]),
        ))?;
        frame(&mut ui);
        let children = ui.children(&root)?;
        let [x, y, w, h] = ui.bounds(&children[0])?.ok_or("field bounds")?.values();
        let point = Point::new(x + w - 12.0, y + h / 2.0)?;
        ui.pointer_input(PointerEvent::new(
            point,
            PointerKind::Press(PointerButton::Primary),
        ))?;
        for offset in [8.0, 16.0] {
            ui.pointer_input(PointerEvent::new(
                Point::new(point.x() + offset, point.y())?,
                PointerKind::Move,
            ))?;
            frame(&mut ui);
        }
        assert_eq!(previews.borrow().as_slice(), [6, 8]);
        assert_eq!(parameter.snapshot().get()?.base, 4);
        ui.pointer_input(PointerEvent::new(
            Point::new(point.x() + 16.0, point.y())?,
            PointerKind::Release(PointerButton::Primary),
        ))?;
        frame(&mut ui);
        if let Some(authority) = authority.as_ref() {
            let request = authority.request().expect("released drag submitted");
            assert_eq!(*request.value(), 8);
            assert_eq!(parameter.snapshot().get()?.base, 4);
            assert!(terminal.borrow().is_empty());
            authority.respond(&request, AuthorityResponse::Accepted(8))?;
        }
        assert!(matches!(
            terminal.borrow().as_slice(),
            [EditOutcome::Committed { value: 8, .. }]
        ));
        assert_eq!(parameter.snapshot().get()?.edit_source, None);
        let [x, y, w, h] = ui.bounds(&children[1])?.ok_or("button bounds")?.values();
        let point = Point::new(x + w / 2.0, y + h / 2.0)?;
        ui.pointer_input(PointerEvent::new(
            point,
            PointerKind::Press(PointerButton::Primary),
        ))?;
        ui.pointer_input(PointerEvent::new(
            point,
            PointerKind::Release(PointerButton::Primary),
        ))?;
        assert_eq!(
            clicks.get(),
            1,
            "pointer capture released to neighboring control"
        );
    }
    Ok(())
}

#[test]
fn remote_select_disables_pending_anchor_and_an_open_list_without_admission() -> TestResult {
    let mut ui = Runtime::new();
    ui.set_debug_mode(DebugMode::Inspect);
    let (parameter, host) =
        ui.remote_parameter(ParameterId::new(1, 1, "waveform")?, waveforms()?)?;
    let select = Select::for_parameter(&ui, parameter.clone());
    let control = mount(&mut ui, select)?;
    ui.focus_next(false)?;
    key(&mut ui, KeyCode::ArrowDown);
    frame(&mut ui);
    key(&mut ui, KeyCode::ArrowDown);
    key(&mut ui, KeyCode::Enter);
    frame(&mut ui);
    let request = host.request().expect("selection submitted");
    assert_eq!(parameter.snapshot().get()?.base, id("sine")?);
    assert_eq!(request.value(), &id("saw")?);
    assert!(semantics(&ui, "select")?.disabled);
    host.respond(&request, AuthorityResponse::Rejected)?;
    frame(&mut ui);
    assert!(!semantics(&ui, "select")?.disabled);
    let anchor_container = ui.children(&control)?[0].clone();
    let anchor = ui.children(&anchor_container)?[0].clone();
    ui.focus(&anchor)?;
    key(&mut ui, KeyCode::ArrowDown);
    frame(&mut ui);
    assert_eq!(semantics(&ui, "select")?.expanded, Some(true));
    // Authority disconnects while the retained option list is already open.
    host.disconnect()?;
    frame(&mut ui);
    assert!(semantics(&ui, "select")?.disabled);
    assert!(semantics(&ui, "options")?.disabled);
    ui.clear_event_history();
    // Even forced low-level focus must not turn a pending list into admission.
    let popup = ui.children(&control)?[1].clone();
    let options = ui.children(&popup)?[0].clone();
    match ui.focus(&options) {
        Ok(()) => {}
        Err(RuntimeError::Hidden) => assert_eq!(semantics(&ui, "select")?.expanded, Some(false)),
        Err(error) => return Err(error.into()),
    }
    key(&mut ui, KeyCode::ArrowDown);
    key(&mut ui, KeyCode::Enter);
    assert!(!ui.inspect().events.iter().any(|e| e.parameter.as_ref()==Some(parameter.id()) && e.kind==EventKind::EditBegin));
    assert!(host.request().is_none());
    host.resynchronize(id("sine")?)?;
    frame(&mut ui);
    ui.focus(&anchor)?;
    key(&mut ui, KeyCode::ArrowDown);
    frame(&mut ui);
    assert!(!semantics(&ui, "options")?.disabled);
    key(&mut ui, KeyCode::ArrowDown);
    key(&mut ui, KeyCode::Enter);
    assert_eq!(
        host.request().expect("recovered selection").value(),
        &id("saw")?
    );
    Ok(())
}

#[test]
fn parameter_selects_are_named_after_the_specification() -> TestResult {
    let mut ui = Runtime::new();
    let (waveform, host) = ui.parameter(ParameterId::new(1, 1, "waveform")?, waveforms()?)?;
    let control = Select::for_parameter(&ui, waveform.clone());
    mount(&mut ui, control)?;
    assert_eq!(semantics(&ui, "select")?.name.as_deref(), Some("Waveform"));
    let renamed = ParameterSpec::choices(
        "Oscillator shape",
        [
            ChoiceOption::new(id("sine")?, "Sine"),
            ChoiceOption::new(id("saw")?, "Saw"),
        ],
        id("sine")?,
    )?;
    host.replace_spec(renamed)?;
    frame(&mut ui);
    assert_eq!(
        semantics(&ui, "select")?.name.as_deref(),
        Some("Oscillator shape")
    );

    let mut ui = Runtime::new();
    let (waveform, _host) = ui.parameter(ParameterId::new(1, 1, "waveform")?, waveforms()?)?;
    let control = InlineSelect::for_parameter(&ui, waveform.clone());
    mount(&mut ui, control)?;
    assert_eq!(semantics(&ui, "select")?.name.as_deref(), Some("Waveform"));
    let mut ui = Runtime::new();
    let (waveform, _host) = ui.parameter(ParameterId::new(1, 1, "waveform")?, waveforms()?)?;
    let control = InlineSelect::for_parameter(&ui, waveform).label("Wave");
    mount(&mut ui, control)?;
    assert_eq!(semantics(&ui, "select")?.name.as_deref(), Some("Wave"));
    Ok(())
}
