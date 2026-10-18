//! Numeric presentation handles preserve retained layout when cloned unchanged.
use pert_ui::*;
use std::time::Duration;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn cloned_formats_preserve_layout_for_each_numeric_presentation() -> TestResult {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(600.0, 400.0, 1.0)?)?;
    let integer = ui.signal(2_i64);
    let decimal = ui.signal(0.5_f64);
    let (parameter, _host) = ui.parameter(
        ParameterId::new(1, 1, "value")?,
        ParameterSpec::new("Value", 0.0, 1.0, 0.5)?,
    )?;
    let integers = NumberFormat::new(
        "999",
        |v: i64| v.to_string(),
        |s| s.parse().map_err(|_| NumberInputError::InvalidNumber),
    );
    let decimals = NumberFormat::new(
        "9.99",
        |v: f64| v.to_string(),
        |s| s.parse().map_err(|_| NumberInputError::InvalidNumber),
    );
    let continuous = NumberFormat::new(
        "9.99",
        |v: f32| v.to_string(),
        |s| s.parse().map_err(|_| NumberInputError::InvalidNumber),
    );
    let describe = || {
        vec![
            Element::new(
                "integer",
                IntegerInput::new(integer.clone()).format(integers.clone()),
            ),
            Element::new(
                "inline-integer",
                InlineInteger::new(integer.clone()).format(integers.clone()),
            ),
            Element::new(
                "decimal",
                DecimalInput::new(decimal.clone()).format(decimals.clone()),
            ),
            Element::new(
                "inline-decimal",
                InlineDecimal::new(decimal.clone()).format(decimals.clone()),
            ),
            Element::new(
                "slider",
                InlineSlider::new(parameter.clone()).format(continuous.clone()),
            ),
        ]
    };
    let root = ui.mount_tree(Element::new("root", Column::new(describe())))?;
    ui.flush(Duration::ZERO)?;
    ui.paint()?;
    ui.set_profiling(true)?;
    ui.reconcile_children(&root, describe())?;
    ui.flush(Duration::ZERO)?;
    ui.paint()?;
    let work: Vec<_> = ui
        .profile()
        .stats
        .into_iter()
        .filter(|stat| {
            matches!(
                stat.operation,
                ProfileOperation::Measure | ProfileOperation::Arrange
            )
        })
        .collect();
    assert!(
        work.is_empty(),
        "unchanged format descriptions caused layout: {work:?}"
    );
    Ok(())
}

#[test]
fn changed_units_and_removing_a_format_update_the_presented_value() -> TestResult {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(400.0, 100.0, 1.0)?)?;
    let value = ui.signal(0.5_f64);
    let format = NumberFormat::new(
        "999 u",
        |v: f64| v.to_string(),
        |s| s.parse().map_err(|_| NumberInputError::InvalidNumber),
    )
    .unit(|_| "u".into());
    let describe = |format| Element::new("value", InlineDecimal::new(value.clone()).format(format));
    let root = ui.mount_tree(Element::new(
        "root",
        Column::new([describe(format.clone())]),
    ))?;
    ui.flush(Duration::ZERO)?;
    ui.set_profiling(true)?;
    ui.reconcile_children(&root, vec![describe(format.unit(|_| "new unit".into()))])?;
    ui.flush(Duration::ZERO)?;
    let shown_unit = |ui: &Runtime| {
        ui.inspect()
            .components
            .into_iter()
            .find(|entry| entry.key == "value")
            .unwrap()
            .details
            .properties["unit"]
            .clone()
    };
    assert_eq!(shown_unit(&ui), InspectValue::Text("new unit".into()));
    assert!(
        ui.profile()
            .stats
            .iter()
            .any(|stat| stat.key == "value" && stat.operation == ProfileOperation::Measure)
    );
    ui.reconcile_children(
        &root,
        vec![Element::new("value", InlineDecimal::new(value))],
    )?;
    ui.flush(Duration::ZERO)?;
    assert_eq!(shown_unit(&ui), InspectValue::Text(String::new()));
    Ok(())
}
