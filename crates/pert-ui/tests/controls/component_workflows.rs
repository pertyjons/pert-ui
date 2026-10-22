//! Consumer workflows for the component API review, using only the public facade.
use pert_ui::prelude::*;
use std::time::Duration;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn ordinary_form_keeps_application_state_in_signals() -> TestResult {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(640.0, 400.0, 1.0)?)?;
    let name = ui.signal("Ada".to_owned());
    let subscribed = ui.signal(false);
    let root = ui.mount_tree(Element::new(
        "profile",
        Form::new([
            Element::new(
                "name",
                Field::new(Element::new("editor", TextField::new(name.clone()))).label("Name"),
            ),
            Element::new("subscribe", Checkbox::new(subscribed.clone(), "Subscribe")),
        ]),
    ))?;
    name.set("Grace".to_owned())?;
    subscribed.set(true)?;
    ui.flush(Duration::ZERO)?;
    assert_eq!(name.get()?, "Grace");
    assert!(subscribed.get()?);
    let snapshot = ui.inspect();
    let entry = snapshot
        .components
        .iter()
        .find(|entry| entry.key == "subscribe")
        .unwrap();
    assert_eq!(
        entry.details.semantics.as_ref().unwrap().value,
        Some(pert_ui::SemanticValue::Bool(true))
    );
    let target = ui.children(&root)?[1].clone();
    ui.focus(&target)?;
    ui.key_input(KeyCode::Space, KeyAction::Press)?;
    ui.key_input(KeyCode::Space, KeyAction::Release)?;
    assert!(!subscribed.get()?);
    ui.flush(Duration::ZERO)?;
    ui.paint()?;
    Ok(())
}

#[test]
fn parameter_panel_keeps_the_host_alive_and_shares_one_value() -> TestResult {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(640.0, 400.0, 1.0)?)?;
    let (amount, host) = ui.parameter(
        ParameterId::new(1, 1, "amount")?,
        ParameterSpec::new("Amount", 0.0, 1.0, 0.5)?,
    )?;
    ui.mount_tree(Element::new(
        "panel",
        Row::new([
            Element::new("slider", Slider::new(amount.clone())),
            Element::new("knob", Knob::new(amount.clone())),
        ]),
    ))?;
    host.write_base(0.75)?;
    ui.flush(Duration::ZERO)?;
    assert_eq!(amount.snapshot().get()?.base, 0.75);
    for key in ["slider", "knob"] {
        let snapshot = ui.inspect();
        let entry = snapshot
            .components
            .iter()
            .find(|entry| entry.key == key)
            .unwrap();
        let Some(pert_ui::SemanticValue::Number { value, .. }) =
            entry.details.semantics.as_ref().unwrap().value
        else {
            panic!("numeric presentation missing for {key}");
        };
        assert_eq!(value, 0.75);
    }
    ui.paint()?;
    Ok(())
}

#[test]
fn dynamic_choices_keep_values_independent_of_labels_and_order() -> TestResult {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(640.0, 400.0, 1.0)?)?;
    let value = ui.signal("second");
    let options = ui.signal(vec![
        Choice::new("first", "First"),
        Choice::new("second", "Second"),
    ]);
    ui.mount_tree(Element::new(
        "selection",
        RadioGroup::bound(value.clone(), options.read_only()),
    ))?;
    options.set(vec![
        Choice::new("second", "Renamed"),
        Choice::new("first", "First"),
    ])?;
    ui.flush(Duration::ZERO)?;
    assert_eq!(value.get()?, "second");
    let snapshot = ui.inspect();
    let entry = snapshot
        .components
        .iter()
        .find(|entry| entry.key == "selection")
        .unwrap();
    let Some(pert_ui::SemanticValue::Choice { label, .. }) =
        &entry.details.semantics.as_ref().unwrap().value
    else {
        panic!("selection presentation missing");
    };
    assert_eq!(label.as_deref(), Some("Renamed"));
    ui.paint()?;
    Ok(())
}

struct PaintedBadge;

impl Component for PaintedBadge {
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }

    fn measure(&self, _: &MeasureContext<'_>, _: Constraints) -> Result<Size, RuntimeError> {
        Ok(Size::new(32.0, 32.0)?)
    }

    fn paint(&self, painter: &mut Painter) -> Result<(), DrawError> {
        painter.fill_rect(Rect::new(0.0, 0.0, 32.0, 32.0)?, Color::ACCENT);
        Ok(())
    }
}

#[test]
fn a_stateless_custom_component_still_implements_update() -> TestResult {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(640.0, 400.0, 1.0)?)?;
    ui.mount_tree(Element::new("badge", PaintedBadge))?;
    ui.flush(Duration::ZERO)?;
    assert!(!ui.paint()?.commands().is_empty());
    assert_eq!(ui.flush(Duration::ZERO)?.components_updated, 0);
    Ok(())
}
