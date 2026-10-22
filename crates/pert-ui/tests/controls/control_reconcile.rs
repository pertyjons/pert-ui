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
    let failed = ui.signal(ProgressStatus::Failed);
    let done = ui.signal(ProgressStatus::Succeeded);
    let (root, bar) = mount(
        &mut ui,
        Element::new("bar", ProgressBar::new(failed.read_only())),
    )?;
    redescribe(
        &mut ui,
        &root,
        &bar,
        Element::new("bar", ProgressBar::new(done.read_only())),
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
        Element::new("spinner", Spinner::new(idle.read_only())),
    )?;
    assert_eq!(property(&ui, "spinner", "busy")?, InspectValue::Bool(false));
    redescribe(
        &mut ui,
        &root,
        &spinner,
        Element::new("spinner", Spinner::new(busy.read_only())),
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
