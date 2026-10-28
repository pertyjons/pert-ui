//! Re-describing a standard control under the same key applies the new
//! description to the retained control: a new binding is edited from then on,
//! a pending gesture on the old one ends with a terminal outcome instead of
//! retargeting, and labels and choice lists follow the description.
use crate::support;
use pert_ui::*;
use std::{cell::RefCell, rc::Rc};
use support::{frame, key};

type Error = Box<dyn std::error::Error>;
type TestResult = Result<(), Error>;
type Log<T> = Rc<RefCell<Vec<EditOutcome<T>>>>;

/// Mounts `control` in a column and returns the column and the control.
fn mount(ui: &mut Runtime, control: Element) -> Result<(WidgetId, WidgetId), Error> {
    ui.set_viewport(Viewport::new(600.0, 400.0, 1.0)?)?;
    let root = ui.mount_tree(Element::new("root", Column::new([control])))?;
    frame(ui);
    let control = ui.children(&root)?[0].clone();
    Ok((root, control))
}
/// Re-describes the column's only child and checks that it kept its identity.
fn redescribe(
    ui: &mut Runtime,
    root: &WidgetId,
    control: &WidgetId,
    next: Element,
) -> Result<(), Error> {
    ui.reconcile_children(root, vec![next])?;
    frame(ui);
    assert_eq!(&ui.children(root)?[0], control, "reconciled, not remounted");
    Ok(())
}
fn details(ui: &Runtime, key: &str) -> Result<ComponentDetails, Box<dyn std::error::Error>> {
    ui.inspect()
        .components
        .into_iter()
        .find(|c| c.key == key)
        .map(|c| c.details)
        .ok_or_else(|| format!("no component {key}").into())
}
fn property(
    ui: &Runtime,
    key: &str,
    name: &str,
) -> Result<InspectValue, Box<dyn std::error::Error>> {
    details(ui, key)?
        .properties
        .get(name)
        .cloned()
        .ok_or_else(|| format!("no property {name} on {key}").into())
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
fn parameter(
    ui: &mut Runtime,
    index: u128,
) -> Result<(Parameter<f32>, ParameterHost<f32>), Box<dyn std::error::Error>> {
    Ok(ui.parameter(
        ParameterId::new(1, index, format!("value-{index}"))?,
        ParameterSpec::new(format!("Value {index}"), 0.0, 100.0, 50.0)?,
    )?)
}

#[test]
fn a_rebound_slider_edits_the_new_parameter() -> TestResult {
    let mut ui = Runtime::new();
    let (p1, _h1) = parameter(&mut ui, 1)?;
    let (p2, _h2) = parameter(&mut ui, 2)?;
    let (root, slider) = mount(&mut ui, Element::new("slider", Slider::new(p1.clone())))?;
    redescribe(
        &mut ui,
        &root,
        &slider,
        Element::new("slider", Slider::new(p2.clone()).show_value(true)),
    )?;
    ui.focus(&slider)?;
    key(&mut ui, KeyCode::ArrowRight);
    assert_eq!(
        p1.snapshot().get()?.base,
        50.0,
        "the old parameter is untouched"
    );
    assert_eq!(p2.snapshot().get()?.base, 51.0);
    Ok(())
}

#[test]
fn rebinding_a_slider_mid_drag_ends_the_old_gesture() -> TestResult {
    let mut ui = Runtime::new();
    let (p1, h1) = parameter(&mut ui, 1)?;
    let (p2, h2) = parameter(&mut ui, 2)?;
    let (log1, _m1) = outcomes(&h1)?;
    let (log2, _m2) = outcomes(&h2)?;
    let (root, slider) = mount(&mut ui, Element::new("slider", Slider::new(p1.clone())))?;
    let [x, y, width, height] = ui.bounds(&slider)?.ok_or("slider bounds")?.values();
    let at = |fraction: f32| Point::new(x + width * fraction, y + height / 2.0);
    ui.pointer_input(PointerEvent::new(
        at(0.8)?,
        PointerKind::Press(PointerButton::Primary),
    ))?;
    assert!(
        p1.snapshot().get()?.preview.is_some(),
        "the drag previews p1"
    );

    redescribe(
        &mut ui,
        &root,
        &slider,
        Element::new("slider", Slider::new(p2.clone())),
    )?;
    assert_eq!(p1.snapshot().get()?.preview, None);
    assert!(
        matches!(log1.borrow().as_slice(), [EditOutcome::Cancelled]),
        "the old gesture has a terminal outcome: {:?}",
        log1.borrow()
    );
    assert_eq!(
        property(&ui, "slider", "editing")?,
        InspectValue::Bool(false)
    );

    // The rest of the old drag neither commits nor retargets.
    ui.pointer_input(PointerEvent::new(at(0.2)?, PointerKind::Move))?;
    ui.pointer_input(PointerEvent::new(
        at(0.2)?,
        PointerKind::Release(PointerButton::Primary),
    ))?;
    frame(&mut ui);
    assert_eq!(p1.snapshot().get()?.base, 50.0);
    assert_eq!(p2.snapshot().get()?.base, 50.0);
    assert_eq!(p2.snapshot().get()?.preview, None);
    assert_eq!(log1.borrow().len(), 1);
    assert!(log2.borrow().is_empty());
    Ok(())
}

#[test]
fn a_rebuilt_checkbox_takes_its_new_signal_label_and_enabled_state() -> TestResult {
    let mut ui = Runtime::new();
    let first = ui.signal(false);
    let second = ui.signal(false);
    let (root, checkbox) = mount(
        &mut ui,
        Element::new("check", Checkbox::new(first.clone(), "First")),
    )?;
    redescribe(
        &mut ui,
        &root,
        &checkbox,
        Element::new("check", Checkbox::new(second.clone(), "Second")),
    )?;
    assert_eq!(
        property(&ui, "check", "text")?,
        InspectValue::Text("Second".into())
    );
    ui.focus(&checkbox)?;
    key(&mut ui, KeyCode::Space);
    assert!(!first.get()?, "the old signal is untouched");
    assert!(second.get()?);

    redescribe(
        &mut ui,
        &root,
        &checkbox,
        Element::new("check", Checkbox::new(second.clone(), "Second").disabled()),
    )?;
    assert_eq!(
        property(&ui, "check", "enabled")?,
        InspectValue::Bool(false)
    );
    Ok(())
}

#[test]
fn a_pending_toggle_activation_does_not_retarget_to_a_new_signal() -> TestResult {
    let mut ui = Runtime::new();
    let first = ui.signal(false);
    let second = ui.signal(false);
    let (root, toggle) = mount(
        &mut ui,
        Element::new("toggle", InlineToggle::new(first.clone(), "Mute")),
    )?;
    ui.focus(&toggle)?;
    ui.key_input(KeyCode::Space, KeyAction::Press)?;
    redescribe(
        &mut ui,
        &root,
        &toggle,
        Element::new("toggle", InlineToggle::new(second.clone(), "Solo")),
    )?;
    ui.key_input(KeyCode::Space, KeyAction::Release)?;
    assert!(
        !first.get()? && !second.get()?,
        "the armed press was cancelled"
    );
    assert_eq!(
        property(&ui, "toggle", "text")?,
        InspectValue::Text("Solo".into())
    );
    key(&mut ui, KeyCode::Space);
    assert!(!first.get()? && second.get()?);
    Ok(())
}

#[test]
fn rebuilt_choice_controls_gain_a_third_choice() -> TestResult {
    let mut ui = Runtime::new();
    let mode = ui.signal("a");
    let two = || [Choice::new("a", "A"), Choice::new("b", "B")];
    let three = || {
        [
            Choice::new("a", "A"),
            Choice::new("b", "B"),
            Choice::new("c", "C"),
        ]
    };
    let (root, radio) = mount(
        &mut ui,
        Element::new("radio", RadioGroup::new(mode.clone(), two())?),
    )?;
    redescribe(
        &mut ui,
        &root,
        &radio,
        Element::new("radio", RadioGroup::new(mode.clone(), three())?),
    )?;
    assert_eq!(
        property(&ui, "radio", "options")?,
        InspectValue::Number(3.0)
    );
    ui.focus(&radio)?;
    key(&mut ui, KeyCode::End);
    assert_eq!(mode.get()?, "c");

    let mut ui = Runtime::new();
    let mode = ui.signal("a");
    let picker = Select::new(&ui, mode.clone(), two())?;
    let (root, select) = mount(&mut ui, Element::new("picker", picker))?;
    let combo_items = |ui: &Runtime| {
        ui.inspect()
            .components
            .into_iter()
            .filter_map(|c| c.details.semantics)
            .find(|s| s.role == SemanticRole::ComboBox)
            .map(|s| s.items.len())
    };
    assert_eq!(combo_items(&ui), Some(2));
    let picker = Select::new(&ui, mode.clone(), three())?;
    redescribe(&mut ui, &root, &select, Element::new("picker", picker))?;
    assert_eq!(combo_items(&ui), Some(3));
    // The list offers the new choice too: open it, go to the end and choose.
    ui.focus_next(FocusDirection::Forward)?;
    key(&mut ui, KeyCode::ArrowDown);
    frame(&mut ui);
    key(&mut ui, KeyCode::End);
    key(&mut ui, KeyCode::Enter);
    frame(&mut ui);
    assert_eq!(mode.get()?, "c");
    Ok(())
}

#[test]
fn a_rebound_progress_bar_follows_the_new_status() -> TestResult {
    let mut ui = Runtime::new();
    let failed = ui.signal(ProgressStatus::Failed(None));
    let done = ui.signal(ProgressStatus::Succeeded);
    let (root, bar) = mount(
        &mut ui,
        Element::new("bar", ProgressBar::bound(failed.read_only())),
    )?;
    redescribe(
        &mut ui,
        &root,
        &bar,
        Element::new("bar", ProgressBar::bound(done.read_only())),
    )?;
    assert_eq!(
        property(&ui, "bar", "status")?,
        InspectValue::Text("succeeded".into())
    );
    Ok(())
}

#[test]
fn a_rebound_spinner_follows_the_new_busy_signal() -> TestResult {
    let mut ui = Runtime::new();
    let idle = ui.signal(false);
    let busy = ui.signal(true);
    let (root, spinner) = mount(
        &mut ui,
        Element::new("spinner", Spinner::bound(idle.read_only())),
    )?;
    assert_eq!(property(&ui, "spinner", "busy")?, InspectValue::Bool(false));
    redescribe(
        &mut ui,
        &root,
        &spinner,
        Element::new("spinner", Spinner::bound(busy.read_only())),
    )?;
    assert_eq!(property(&ui, "spinner", "busy")?, InspectValue::Bool(true));
    // The replaced binding no longer drives it; the new one does.
    idle.set(true)?;
    busy.set(false)?;
    frame(&mut ui);
    assert_eq!(property(&ui, "spinner", "busy")?, InspectValue::Bool(false));
    Ok(())
}

#[test]
fn a_redescribed_readout_adopts_its_reader_and_keeps_text_mode() -> TestResult {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(600.0, 400.0, 1.0)?)?;
    let first = ui.signal("001:01:000".to_owned());
    let second = ui.signal("001:01:000".to_owned());
    let describe = |source: &Signal<String>| {
        InlineReadout::new(source.read_only())
            .label("Pos")
            .unit("bar")
            .width_hint("000:00:000")
    };
    // A row gives the readout its own measured width.
    let root = ui.mount_tree(Element::new(
        "root",
        Row::new([Element::new("readout", describe(&first))]),
    ))?;
    frame(&mut ui);
    let readout = ui.children(&root)?[0].clone();
    let narrow = ui.bounds(&readout)?.ok_or("unplaced")?.width();
    ui.focus(&readout)?;
    key(&mut ui, KeyCode::F2);
    frame(&mut ui);
    assert_eq!(
        property(&ui, "readout", "text_editing")?,
        InspectValue::Bool(true)
    );

    // Same text from a new reader, with a new caption, unit and typography.
    ui.reconcile_children(
        &root,
        vec![Element::new(
            "readout",
            describe(&second)
                .label("Position")
                .unit("beat")
                .style(TextStyle::Heading),
        )],
    )?;
    frame(&mut ui);
    assert_eq!(ui.children(&root)?[0], readout, "reconciled, not remounted");
    assert_eq!(
        property(&ui, "readout", "text_editing")?,
        InspectValue::Bool(true),
        "text mode survives a reader that shows the same text"
    );
    assert_eq!(
        property(&ui, "readout", "unit")?,
        InspectValue::Text("beat".into())
    );
    let name = details(&ui, "readout")?
        .semantics
        .and_then(|s| s.name)
        .ok_or("unnamed readout")?;
    assert_eq!(name, "Position");
    let wide = ui.bounds(&readout)?.ok_or("unplaced")?.width();
    assert!(
        wide > narrow,
        "the heading style widens the slot: {narrow} -> {wide}"
    );

    // The new reader drives the text; the old one no longer does.
    first.set("009:09:000".to_owned())?;
    frame(&mut ui);
    assert_eq!(
        property(&ui, "readout", "text")?,
        InspectValue::Text("001:01:000".into())
    );
    second.set("002:03:000".to_owned())?;
    frame(&mut ui);
    assert_eq!(
        property(&ui, "readout", "text")?,
        InspectValue::Text("002:03:000".into())
    );
    assert_eq!(
        property(&ui, "readout", "text_editing")?,
        InspectValue::Bool(false),
        "changed text leaves text mode"
    );
    Ok(())
}

/// A display control's source: a fixed value or a binding holding one.
#[derive(Clone, Copy, Debug)]
enum Source<T> {
    Fixed(T),
    Bound(T),
}

/// Re-describes a display control from `before` to `after` through
/// `check_redescription`, checks the shown value, and, after a change from a
/// binding to a fixed value, that a write to the old binding updates nothing.
fn redescribe_display<T: Clone + PartialEq + 'static>(
    (before, after): (Source<T>, Source<T>),
    describe: fn(&Runtime, Source<T>, Option<&Signal<T>>) -> Element,
    (property_name, shown): (&str, InspectValue),
    other: T,
) -> TestResult {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(600.0, 400.0, 1.0)?)?;
    let old = match &before {
        Source::Bound(value) => Some(ui.signal(value.clone())),
        Source::Fixed(_) => None,
    };
    let new = match &after {
        Source::Bound(value) => Some(ui.signal(value.clone())),
        Source::Fixed(_) => None,
    };
    let first = describe(&ui, before.clone(), old.as_ref());
    let second = describe(&ui, after.clone(), new.as_ref());
    let fresh = describe(&ui, after.clone(), new.as_ref());
    let result = check_redescription(&mut ui, first, second, fresh)?;
    let key = ui.key(&result.component)?.to_owned();
    assert_eq!(property(&ui, &key, property_name)?, shown);
    if let (Some(old), Source::Fixed(_)) = (&old, &after) {
        old.set(other)?;
        let report = ui.flush(ui.now())?;
        assert_eq!(report.components_updated, 0, "the old binding is dropped");
        assert_eq!(property(&ui, &key, property_name)?, shown);
    }
    Ok(())
}

fn describe_bar(
    _: &Runtime,
    source: Source<ProgressStatus>,
    signal: Option<&Signal<ProgressStatus>>,
) -> Element {
    let bar = match (source, signal) {
        (Source::Fixed(status), _) => ProgressBar::new(status),
        (Source::Bound(_), Some(signal)) => ProgressBar::bound(signal.read_only()),
        (Source::Bound(_), None) => unreachable!("a bound case has its signal"),
    };
    Element::new("bar", bar)
}

fn describe_spinner(_: &Runtime, source: Source<bool>, signal: Option<&Signal<bool>>) -> Element {
    let spinner = match (source, signal) {
        (Source::Fixed(busy), _) => Spinner::new(busy),
        (Source::Bound(_), Some(signal)) => Spinner::bound(signal.read_only()),
        (Source::Bound(_), None) => unreachable!("a bound case has its signal"),
    };
    Element::new("spinner", spinner)
}

#[test]
fn a_progress_bar_adopts_every_change_of_source() -> TestResult {
    use ProgressStatus::{Failed, Succeeded};
    let quarter = ProgressStatus::of(1, 4);
    let status = |name: &str| ("status", InspectValue::Text(name.into()));
    for (change, shown) in [
        (
            (Source::Fixed(quarter), Source::Fixed(Succeeded)),
            "succeeded",
        ),
        (
            (Source::Fixed(quarter), Source::Bound(Succeeded)),
            "succeeded",
        ),
        (
            (Source::Bound(quarter), Source::Fixed(Succeeded)),
            "succeeded",
        ),
        // The shown value stays equal, but the binding is still dropped.
        (
            (Source::Bound(Failed(None)), Source::Fixed(Failed(None))),
            "failed",
        ),
    ] {
        redescribe_display(change, describe_bar, status(shown), ProgressStatus::Idle)
            .map_err(|error| format!("{change:?}: {error}"))?;
    }
    Ok(())
}

#[test]
fn a_spinner_adopts_every_change_of_source() -> TestResult {
    let busy = |busy: bool| ("busy", InspectValue::Bool(busy));
    for (change, shown) in [
        ((Source::Fixed(true), Source::Fixed(false)), false),
        ((Source::Fixed(true), Source::Bound(false)), false),
        ((Source::Bound(false), Source::Fixed(true)), true),
        ((Source::Bound(true), Source::Fixed(true)), true),
    ] {
        let other = match change.0 {
            Source::Bound(value) | Source::Fixed(value) => !value,
        };
        redescribe_display(change, describe_spinner, busy(shown), other)
            .map_err(|error| format!("{change:?}: {error}"))?;
    }
    Ok(())
}

#[test]
fn fixed_values_apply_when_the_description_is_adopted() -> TestResult {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(600.0, 400.0, 1.0)?)?;
    let name = ui.signal(String::new());
    let children = |status: ProgressStatus, busy: bool, field: FieldStatus| {
        vec![
            Element::new("bar", ProgressBar::new(status)),
            Element::new("spinner", Spinner::new(busy)),
            Element::new(
                "field",
                Field::new(Element::new("name", TextField::new(name.clone()))).status(field),
            ),
        ]
    };
    let check = |ui: &Runtime, status: &str, busy: bool, field: (&str, &str)| -> TestResult {
        assert_eq!(
            property(ui, "bar", "status")?,
            InspectValue::Text(status.into())
        );
        assert_eq!(property(ui, "spinner", "busy")?, InspectValue::Bool(busy));
        assert_eq!(
            property(ui, "field", "status")?,
            InspectValue::Text(field.0.into())
        );
        assert_eq!(
            property(ui, "field", "message")?,
            InspectValue::Text(field.1.into())
        );
        Ok(())
    };
    let root = ui.mount_tree(Element::new(
        "root",
        Column::new(children(
            ProgressStatus::of(1, 4),
            true,
            FieldStatus::error("Required"),
        )),
    ))?;
    // Before any flush: the constructors applied the values.
    check(&ui, "determinate", true, ("error", "Required"))?;
    frame(&mut ui);
    check(&ui, "determinate", true, ("error", "Required"))?;

    ui.reconcile_children(
        &root,
        children(
            ProgressStatus::Succeeded,
            false,
            FieldStatus::warning("Check"),
        ),
    )?;
    // Before any flush: reconciliation applied them.
    check(&ui, "succeeded", false, ("warning", "Check"))?;
    frame(&mut ui);
    check(&ui, "succeeded", false, ("warning", "Check"))?;
    Ok(())
}

fn readout_text(
    ui: &Runtime,
) -> Result<(InspectValue, InspectValue, Option<SemanticValue>), Error> {
    Ok((
        property(ui, "readout", "text")?,
        property(ui, "readout", "display_value")?,
        details(ui, "readout")?.semantics.and_then(|s| s.value),
    ))
}
fn shows(text: &str) -> (InspectValue, InspectValue, Option<SemanticValue>) {
    (
        InspectValue::Text(text.into()),
        InspectValue::Text(text.into()),
        Some(SemanticValue::Text(text.into())),
    )
}
fn is_invalid_text(error: &RuntimeError) -> bool {
    matches!(error, RuntimeError::TextEdit(TextEditError::InvalidText))
}

#[test]
fn fixed_readout_text_applies_when_the_description_is_adopted() -> TestResult {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(600.0, 400.0, 1.0)?)?;
    let describe = |text: TextSource| {
        vec![Element::new(
            "readout",
            InlineReadout::new(text).label("Pos"),
        )]
    };
    let root = ui.mount_tree(Element::new(
        "root",
        Row::new(describe("001:01:000".into())),
    ))?;
    assert_eq!(
        readout_text(&ui)?,
        shows("001:01:000"),
        "before the first flush"
    );
    frame(&mut ui);
    let readout = ui.children(&root)?[0].clone();
    ui.reconcile_children(&root, describe("002:01:000".into()))?;
    assert_eq!(readout_text(&ui)?, shows("002:01:000"), "before the flush");
    frame(&mut ui);
    assert_eq!(readout_text(&ui)?, shows("002:01:000"));

    // Text mode stays open for an equal text and ends for a changed one.
    ui.focus(&readout)?;
    assert_eq!(
        ui.text_command(&readout, TextCommand::BeginEdit)?,
        TextCommandOutcome::Handled
    );
    frame(&mut ui);
    let editing = |ui: &Runtime| property(ui, "readout", "text_editing");
    assert_eq!(editing(&ui)?, InspectValue::Bool(true));
    ui.reconcile_children(&root, describe("002:01:000".into()))?;
    frame(&mut ui);
    assert_eq!(editing(&ui)?, InspectValue::Bool(true), "an equal text");
    ui.reconcile_children(&root, describe("003:01:000".into()))?;
    frame(&mut ui);
    assert_eq!(editing(&ui)?, InspectValue::Bool(false), "a changed text");
    assert_eq!(readout_text(&ui)?, shows("003:01:000"));

    // Text that cannot be shown is refused at adoption: the last valid text
    // stays, and every flush reports the refusal until a valid one replaces it.
    ui.reconcile_children(&root, describe("\u{7}".into()))?;
    assert_eq!(readout_text(&ui)?, shows("003:01:000"));
    let error = ui.flush(ui.now()).unwrap_err();
    assert!(is_invalid_text(&error), "{error:?}");
    let error = ui.flush(ui.now()).unwrap_err();
    assert!(is_invalid_text(&error), "{error:?}");
    assert_eq!(readout_text(&ui)?, shows("003:01:000"));
    ui.reconcile_children(&root, describe("004:01:000".into()))?;
    ui.flush(ui.now())?;
    assert_eq!(readout_text(&ui)?, shows("004:01:000"));

    // Rich content shows as its plain text.
    ui.reconcile_children(&root, describe(RichText::from("12").text(" ms").into()))?;
    frame(&mut ui);
    assert_eq!(readout_text(&ui)?, shows("12 ms"));
    assert_eq!(ui.children(&root)?[0], readout, "reconciled, not remounted");
    Ok(())
}

/// The readout's shown text and unit.
fn text_and_unit(ui: &Runtime) -> Result<(InspectValue, InspectValue), Error> {
    Ok((
        property(ui, "readout", "text")?,
        property(ui, "readout", "unit")?,
    ))
}
fn pair(text: &str, unit: &str) -> (InspectValue, InspectValue) {
    (
        InspectValue::Text(text.into()),
        InspectValue::Text(unit.into()),
    )
}

#[test]
fn a_fixed_readout_refuses_a_unit_that_cannot_be_shown_and_recovers() -> TestResult {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(600.0, 400.0, 1.0)?)?;
    let describe =
        |text: &str, unit: &str| vec![Element::new("readout", InlineReadout::new(text).unit(unit))];
    let root = ui.mount_tree(Element::new("root", Row::new(describe("1.0", "ms"))))?;
    frame(&mut ui);
    assert_eq!(text_and_unit(&ui)?, pair("1.0", "ms"));

    // A unit that cannot be shown refuses the whole description, its valid
    // text included: the last valid text and unit stay.
    ui.reconcile_children(&root, describe("2.0", "\u{7}"))?;
    assert_eq!(text_and_unit(&ui)?, pair("1.0", "ms"), "before the flush");
    let error = ui.flush(ui.now()).unwrap_err();
    assert!(is_invalid_text(&error), "{error:?}");
    assert_eq!(text_and_unit(&ui)?, pair("1.0", "ms"));

    // A valid description with a new text and a new unit ends the refusal.
    ui.reconcile_children(&root, describe("3.0", "s"))?;
    assert_eq!(text_and_unit(&ui)?, pair("3.0", "s"), "before the flush");
    ui.flush(ui.now())?;
    assert_eq!(text_and_unit(&ui)?, pair("3.0", "s"));

    // A new unit alone checks the kept text again, both ways.
    ui.reconcile_children(&root, describe("3.0", "\u{7}"))?;
    assert!(is_invalid_text(&ui.flush(ui.now()).unwrap_err()));
    assert_eq!(text_and_unit(&ui)?, pair("3.0", "s"));
    ui.reconcile_children(&root, describe("3.0", "min"))?;
    ui.flush(ui.now())?;
    assert_eq!(text_and_unit(&ui)?, pair("3.0", "min"));

    // A refused new text with a bad unit recovers when only the unit is
    // corrected, the text described again unchanged.
    ui.reconcile_children(&root, describe("4.0", "\u{7}"))?;
    assert!(is_invalid_text(&ui.flush(ui.now()).unwrap_err()));
    assert_eq!(text_and_unit(&ui)?, pair("3.0", "min"));
    ui.reconcile_children(&root, describe("4.0", "h"))?;
    assert_eq!(text_and_unit(&ui)?, pair("4.0", "h"), "before the flush");
    ui.flush(ui.now())?;
    assert_eq!(text_and_unit(&ui)?, pair("4.0", "h"));
    Ok(())
}

#[test]
fn a_readout_mounted_with_a_unit_that_cannot_be_shown_recovers() -> TestResult {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(600.0, 400.0, 1.0)?)?;
    let describe =
        |text: &str, unit: &str| vec![Element::new("readout", InlineReadout::new(text).unit(unit))];
    let root = ui.mount_tree(Element::new("root", Row::new(describe("old", "\u{7}"))))?;
    // Nothing was valid yet, so nothing shows.
    assert_eq!(text_and_unit(&ui)?, pair("", ""));
    let error = ui.flush(ui.now()).unwrap_err();
    assert!(is_invalid_text(&error), "{error:?}");
    ui.reconcile_children(&root, describe("new", "ms"))?;
    ui.flush(ui.now())?;
    assert_eq!(text_and_unit(&ui)?, pair("new", "ms"));
    assert_eq!(readout_text(&ui)?, shows("new"));
    Ok(())
}

#[test]
fn a_readout_follows_rich_bindings_and_string_roles() -> TestResult {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(600.0, 400.0, 1.0)?)?;
    let first = ui.signal(RichText::from("a"));
    let second = ui.signal(RichText::from("b"));
    let describe = |text: TextSource| vec![Element::new("readout", InlineReadout::new(text))];
    let root = ui.mount_tree(Element::new("root", Row::new(describe((&first).into()))))?;
    frame(&mut ui);
    assert_eq!(readout_text(&ui)?, shows("a"));
    first.set(RichText::from("one").text(" two"))?;
    frame(&mut ui);
    assert_eq!(readout_text(&ui)?, shows("one two"));

    // Another binding replaces the first, whose writes no longer show.
    ui.reconcile_children(&root, describe((&second).into()))?;
    frame(&mut ui);
    assert_eq!(readout_text(&ui)?, shows("b"));
    first.set(RichText::from("ignored"))?;
    frame(&mut ui);
    assert_eq!(readout_text(&ui)?, shows("b"));
    second.set(RichText::from("three"))?;
    frame(&mut ui);
    assert_eq!(readout_text(&ui)?, shows("three"));

    // A role follows the installed string table.
    let role = StringRole::SearchPlaceholder;
    ui.reconcile_children(&root, describe(role.into()))?;
    frame(&mut ui);
    assert_eq!(readout_text(&ui)?, shows(UiStrings::english().text(role)));
    ui.ui_settings()
        .set_strings(UiStrings::english().with_text(role, "Suchen")?)?;
    frame(&mut ui);
    assert_eq!(readout_text(&ui)?, shows("Suchen"));
    Ok(())
}

/// Mounts a readout, opens its text mode, and returns the root.
fn readout_in_text_mode(ui: &mut Runtime, readout: InlineReadout) -> Result<WidgetId, Error> {
    ui.set_viewport(Viewport::new(600.0, 400.0, 1.0)?)?;
    let root = ui.mount_tree(Element::new(
        "root",
        Row::new([Element::new("readout", readout)]),
    ))?;
    frame(ui);
    let id = ui.children(&root)?[0].clone();
    ui.focus(&id)?;
    ui.text_command(&id, TextCommand::BeginEdit)?;
    frame(ui);
    assert_eq!(
        property(ui, "readout", "text_editing")?,
        InspectValue::Bool(true)
    );
    Ok(root)
}

#[test]
fn a_refused_bound_text_keeps_the_last_valid_presentation() -> TestResult {
    let mut ui = Runtime::new();
    let text = ui.signal(String::from("ok"));
    readout_in_text_mode(&mut ui, InlineReadout::new(text.read_only()))?;
    text.set("bad\u{7}".into())?;
    let error = ui.flush(ui.now()).unwrap_err();
    assert!(is_invalid_text(&error), "{error:?}");
    assert_eq!(readout_text(&ui)?, shows("ok"));
    assert_eq!(
        property(&ui, "readout", "text_editing")?,
        InspectValue::Bool(true),
        "a refused text keeps text mode"
    );
    text.set("fine".into())?;
    ui.flush(ui.now())?;
    assert_eq!(readout_text(&ui)?, shows("fine"));
    assert_eq!(
        property(&ui, "readout", "text_editing")?,
        InspectValue::Bool(false)
    );
    Ok(())
}

#[test]
fn a_refused_formatted_text_keeps_the_last_valid_presentation() -> TestResult {
    let mut ui = Runtime::new();
    let value = ui.signal(1.0_f64);
    // A display that returns a control character for a negative value.
    let format = NumberFormat::new(
        "00",
        |v: f64| {
            if v < 0.0 {
                "\u{7}".to_owned()
            } else {
                format!("{v}")
            }
        },
        |_| Err(NumberInputError::InvalidNumber),
    );
    readout_in_text_mode(&mut ui, InlineReadout::formatted(value.read_only(), format))?;
    assert_eq!(readout_text(&ui)?, shows("1"));
    value.set(-1.0)?;
    let error = ui.flush(ui.now()).unwrap_err();
    assert!(is_invalid_text(&error), "{error:?}");
    assert_eq!(readout_text(&ui)?, shows("1"));
    assert_eq!(
        property(&ui, "readout", "text_editing")?,
        InspectValue::Bool(true)
    );
    value.set(2.0)?;
    ui.flush(ui.now())?;
    assert_eq!(readout_text(&ui)?, shows("2"));
    assert_eq!(
        property(&ui, "readout", "text_editing")?,
        InspectValue::Bool(false)
    );
    Ok(())
}

#[test]
fn every_binding_the_readout_took_before_still_converts() -> TestResult {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(600.0, 400.0, 1.0)?)?;
    let text = ui.signal(String::from("bound"));
    let read = text.read_only();
    ui.mount_tree(Element::new(
        "root",
        Column::new([
            Element::new("read-owned", InlineReadout::new(read.clone())),
            Element::new("read-borrowed", InlineReadout::new(&read)),
            Element::new("signal-owned", InlineReadout::new(text.clone())),
            Element::new("signal-borrowed", InlineReadout::new(&text)),
        ]),
    ))?;
    frame(&mut ui);
    text.set("written".into())?;
    frame(&mut ui);
    for key in [
        "read-owned",
        "read-borrowed",
        "signal-owned",
        "signal-borrowed",
    ] {
        assert_eq!(
            property(&ui, key, "text")?,
            InspectValue::Text("written".into()),
            "{key}"
        );
    }
    Ok(())
}
