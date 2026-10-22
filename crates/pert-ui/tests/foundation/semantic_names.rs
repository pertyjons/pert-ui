//! How inspection resolves the name of a component that reports none of its
//! own: its element's label, a wrapping `Field`, and both through a
//! composite's internal structure.
use pert_ui::*;
use std::time::Duration;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn semantics(ui: &Runtime, key: &str) -> (u64, Semantics, Option<u64>) {
    let component = ui
        .inspect()
        .components
        .into_iter()
        .find(|component| component.key == key)
        .unwrap_or_else(|| panic!("no component {key}"));
    (
        component.id,
        component.details.semantics.expect("semantics"),
        component.labelled_by,
    )
}

fn options() -> [Choice<u8>; 2] {
    [Choice::new(0, "Linear"), Choice::new(1, "Smooth")]
}

#[test]
fn an_element_label_names_a_component_without_a_name() -> TestResult {
    let mut ui = Runtime::new();
    let query = ui.signal(String::new());
    ui.mount_tree(Element::new(
        "root",
        Column::new([
            Element::new("search", SearchField::new(query.clone())).with_label("Find"),
            // A name the component reports itself wins over the label.
            Element::new("save", Button::new("Save", |_| Ok(()))).with_label("Other"),
            // A blank label names nothing.
            Element::new("blank", SearchField::new(query)).with_label("  "),
        ]),
    ))?;
    ui.flush(Duration::ZERO)?;
    assert_eq!(semantics(&ui, "search").1.name.as_deref(), Some("Find"));
    assert_eq!(semantics(&ui, "save").1.name.as_deref(), Some("Save"));
    assert_eq!(semantics(&ui, "blank").1.name, None);
    Ok(())
}

#[test]
fn a_composites_label_names_the_part_that_represents_it() -> TestResult {
    let mut ui = Runtime::new();
    let mode = ui.signal(0_u8);
    ui.mount_tree(Element::new(
        "root",
        Column::new([
            Element::new("plain", Select::new(&ui, mode.clone(), options())?),
            Element::new("labelled", Select::new(&ui, mode, options())?).with_label("Mode"),
        ]),
    ))?;
    ui.flush(Duration::ZERO)?;
    // A Select reports nothing itself; its anchor, internal structure keyed
    // `select`, is the combo box.
    let anchors: Vec<_> = ui
        .inspect()
        .components
        .into_iter()
        .filter(|component| component.key == "select")
        .map(|component| component.details.semantics.and_then(|s| s.name))
        .collect();
    assert_eq!(anchors, [None, Some("Mode".to_owned())]);
    Ok(())
}

#[test]
fn a_field_names_a_composites_part_through_its_structure() -> TestResult {
    let mut ui = Runtime::new();
    let mode = ui.signal(0_u8);
    ui.mount_tree(Element::new(
        "root",
        Column::new([Element::new(
            "field",
            Field::new(Element::new("mode", Select::new(&ui, mode, options())?))
                .label("Interpolation"),
        )]),
    ))?;
    ui.flush(Duration::ZERO)?;
    let (field, _, _) = semantics(&ui, "field");
    let (_, anchor, labelled_by) = semantics(&ui, "select");
    assert_eq!(anchor.name.as_deref(), Some("Interpolation"));
    assert_eq!(anchor.role, SemanticRole::ComboBox);
    assert_eq!(labelled_by, Some(field));
    // A query for the part alone resolves the same name.
    let queried =
        ui.query(&InspectionQuery::components(Selector::key("select")).detail(Detail::Semantics));
    let queried = queried.components.first().expect("the anchor");
    assert_eq!(
        queried
            .details
            .semantics
            .as_ref()
            .and_then(|s| s.name.as_deref()),
        Some("Interpolation")
    );
    assert_eq!(queried.labelled_by, Some(field));
    Ok(())
}

#[test]
fn a_named_composite_that_reports_semantics_keeps_its_parts_own() -> TestResult {
    let mut ui = Runtime::new();
    let checked = ui.signal(false);
    // The checkbox reports its own name, so the field's label does not replace
    // it, and a composite with semantics of its own passes nothing further.
    ui.mount_tree(Element::new(
        "root",
        Column::new([Element::new(
            "field",
            Field::new(Element::new(
                "check",
                Checkbox::new(checked, "Process audio"),
            ))
            .label("Enabled"),
        )]),
    ))?;
    ui.flush(Duration::ZERO)?;
    let (_, check, labelled_by) = semantics(&ui, "check");
    assert_eq!(check.name.as_deref(), Some("Process audio"));
    assert_eq!(labelled_by, None);
    Ok(())
}
