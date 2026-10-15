//! Bound element labels through the public facade: `Element::with_label`
//! accepts a `TextSource`, the runtime reads a binding for the parent so
//! containers that title their children (Tabs, Accordion) follow it without
//! redescription, the same binding keeps its text across redescription, a
//! leading icon slot is composed with the read text, and a blank read keeps the
//! last titles and fails with `LayoutError::BlankLabel`.
use pert_ui::*;
mod support;
use support::frame;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn runtime() -> Result<Runtime, Box<dyn std::error::Error>> {
    let ui = Runtime::new();
    ui.set_viewport(Viewport::new(480.0, 320.0, 1.0)?)?;
    Ok(ui)
}

fn component(ui: &Runtime, key: &str) -> ComponentInspection {
    ui.inspect()
        .components
        .into_iter()
        .find(|c| c.key == key)
        .unwrap_or_else(|| panic!("no component {key}"))
}

/// The names of the items a container reports, in order.
fn item_names(ui: &Runtime, key: &str) -> Vec<String> {
    component(ui, key)
        .details
        .semantics
        .map(|s| s.items.into_iter().filter_map(|item| item.name).collect())
        .unwrap_or_default()
}

fn body(key: &str) -> Element {
    Element::new(key, Label::new(key.to_owned()))
}

#[test]
fn tab_titles_follow_bindings_and_keep_them_across_redescription() -> TestResult {
    let mut ui = runtime()?;
    let mix = ui.signal("Mix".to_owned());
    let describe = |mix: ReadSignal<String>| {
        Element::new(
            "tabs",
            Tabs::new([
                Tab::new("mix", mix, body("mix-body")).leading_icon(Icon::Settings),
                Tab::new("edit", "Edit", body("edit-body")),
            ]),
        )
    };
    let root = ui.mount_tree(Element::new(
        "root",
        Column::new([describe(mix.read_only())]),
    ))?;
    frame(&mut ui);
    assert_eq!(item_names(&ui, "tabs"), ["Mix", "Edit"]);
    mix.set("Mix *".to_owned())?;
    frame(&mut ui);
    assert_eq!(item_names(&ui, "tabs"), ["Mix *", "Edit"]);

    // The same binding keeps the read title, including in the child's
    // metadata, before and after the next update.
    ui.reconcile_children(&root, vec![describe(mix.read_only())])?;
    frame(&mut ui);
    assert_eq!(item_names(&ui, "tabs"), ["Mix *", "Edit"]);

    // A fixed reference with the same text and icon paints the same strip.
    let mut reference = runtime()?;
    reference.mount_tree(Element::new(
        "root",
        Column::new([Element::new(
            "tabs",
            Tabs::new([
                Tab::new("mix", "Mix *", body("mix-body")).leading_icon(Icon::Settings),
                Tab::new("edit", "Edit", body("edit-body")),
            ]),
        )]),
    ))?;
    frame(&mut reference);
    assert_eq!(ui.paint()?, reference.paint()?);
    Ok(())
}

#[test]
fn a_blank_bound_title_keeps_the_last_titles_and_fails_the_update() -> TestResult {
    let mut ui = runtime()?;
    let title = ui.signal("Mix".to_owned());
    ui.mount_tree(Element::new(
        "root",
        Column::new([Element::new(
            "tabs",
            Tabs::new([
                Tab::new("mix", title.read_only(), body("mix-body")),
                Tab::new("edit", "Edit", body("edit-body")),
            ]),
        )]),
    ))?;
    frame(&mut ui);
    title.set("  ".to_owned())?;
    assert_eq!(
        ui.flush(ui.now()),
        Err(RuntimeError::Layout(LayoutError::BlankLabel))
    );
    assert_eq!(item_names(&ui, "tabs"), ["Mix", "Edit"]);
    title.set("Mixer".to_owned())?;
    frame(&mut ui);
    assert_eq!(item_names(&ui, "tabs"), ["Mixer", "Edit"]);
    Ok(())
}

#[test]
fn accordion_sections_follow_bound_titles() -> TestResult {
    let mut ui = runtime()?;
    let title = ui.signal("Inputs".to_owned());
    ui.mount_tree(Element::new(
        "root",
        Column::new([Element::new(
            "accordion",
            Accordion::new([
                Section::new("in", title.read_only(), body("in-body")),
                Section::new("out", "Outputs", body("out-body")),
            ]),
        )]),
    ))?;
    frame(&mut ui);
    assert_eq!(item_names(&ui, "accordion"), ["Inputs", "Outputs"]);
    title.set("Eingänge".to_owned())?;
    frame(&mut ui);
    assert_eq!(item_names(&ui, "accordion"), ["Eingänge", "Outputs"]);
    Ok(())
}

#[test]
fn a_bound_element_label_names_a_component_without_a_name() -> TestResult {
    let mut ui = runtime()?;
    let name = ui.signal("Find pages".to_owned());
    let query = ui.signal(String::new());
    let element = Element::new("search", SearchField::new(query)).with_label(name.read_only());
    assert_eq!(element.label(), None);
    assert!(element.label_source().is_some());
    let root = ui.mount_tree(Element::new("root", Column::new([element])))?;
    frame(&mut ui);
    let named = |ui: &Runtime| {
        component(ui, "search")
            .details
            .semantics
            .and_then(|s| s.name)
    };
    assert_eq!(named(&ui).as_deref(), Some("Find pages"));
    name.set("Seiten suchen".to_owned())?;
    frame(&mut ui);
    assert_eq!(named(&ui).as_deref(), Some("Seiten suchen"));

    // Described again with the same binding, the read name stays at once.
    let again = |query: Signal<String>, source: ReadSignal<String>| {
        Element::new("search", SearchField::new(query)).with_label(source)
    };
    let query = ui.signal(String::new());
    ui.reconcile_children(&root, vec![again(query.clone(), name.read_only())])?;
    assert_eq!(named(&ui).as_deref(), Some("Seiten suchen"));
    // Another binding has no name until the parent reads it.
    let other = ui.signal("Other".to_owned());
    ui.reconcile_children(&root, vec![again(query, other.read_only())])?;
    assert_eq!(named(&ui), None);
    frame(&mut ui);
    assert_eq!(named(&ui).as_deref(), Some("Other"));
    Ok(())
}

#[test]
fn fixed_rich_label_metadata_keeps_its_plain_name() {
    let element = Element::new("e", Label::new("x"))
        .with_label(RichText::from("Mix").leading_icon(Icon::Folder));
    assert_eq!(element.label(), Some("Mix"));
    assert_eq!(
        element.rich_label(),
        Some(&RichText::from("Mix").leading_icon(Icon::Folder))
    );
    let plain = Element::new("e", Label::new("x")).with_label("Mix");
    assert_eq!(plain.rich_label(), None);
    assert!(plain.label_source().is_none());
}

#[test]
fn a_property_row_label_follows_its_binding_in_the_grid_and_the_field() -> TestResult {
    let describe = |label: TextSource, value: Signal<String>| {
        Element::new(
            "grid",
            PropertyGrid::new([Property::new(
                "name",
                label,
                Element::new("input", TextField::new(value)),
            )]),
        )
    };
    let mut ui = runtime()?;
    let label = ui.signal("Name".to_owned());
    let value = ui.signal(String::from("Channel 1"));
    ui.mount_tree(Element::new(
        "root",
        Column::new([describe(label.read_only().into(), value)]),
    ))?;
    frame(&mut ui);
    label.set("Bezeichnung".to_owned())?;
    frame(&mut ui);
    let field = component(&ui, "name").details.semantics.unwrap();
    assert_eq!(field.name.as_deref(), Some("Bezeichnung"));

    let mut reference = runtime()?;
    let value = reference.signal(String::from("Channel 1"));
    reference.mount_tree(Element::new(
        "root",
        Column::new([describe("Bezeichnung".into(), value)]),
    ))?;
    frame(&mut reference);
    assert_eq!(ui.paint()?, reference.paint()?);
    Ok(())
}

#[test]
fn a_root_element_reads_its_own_bound_label() -> TestResult {
    let mut ui = runtime()?;
    let name = ui.signal("Find pages".to_owned());
    let query = ui.signal(String::new());
    ui.mount_tree(Element::new("search", SearchField::new(query)).with_label(name.read_only()))?;
    frame(&mut ui);
    let named = |ui: &Runtime| {
        component(ui, "search")
            .details
            .semantics
            .and_then(|s| s.name)
    };
    assert_eq!(named(&ui).as_deref(), Some("Find pages"));
    name.set("Suchen".to_owned())?;
    frame(&mut ui);
    assert_eq!(named(&ui).as_deref(), Some("Suchen"));
    Ok(())
}

/// A container that refuses a child whose label is blank, as the containers
/// that title their children do.
struct Strict;
impl Component for Strict {
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        if cx
            .children()
            .iter()
            .any(|child| child.label().is_some_and(|label| label.trim().is_empty()))
        {
            return Err(LayoutError::BlankLabel.into());
        }
        Ok(())
    }
}

#[test]
fn a_rejected_parent_update_keeps_the_last_child_label() -> TestResult {
    let mut ui = runtime()?;
    let name = ui.signal("Find pages".to_owned());
    let query = ui.signal(String::new());
    ui.mount_tree(
        Element::new("strict", Strict).children([
            Element::new("search", SearchField::new(query)).with_label(name.read_only())
        ]),
    )?;
    frame(&mut ui);
    let named = |ui: &Runtime| {
        component(ui, "search")
            .details
            .semantics
            .and_then(|s| s.name)
    };
    assert_eq!(named(&ui).as_deref(), Some("Find pages"));
    name.set(" ".to_owned())?;
    assert_eq!(
        ui.flush(ui.now()),
        Err(RuntimeError::Layout(LayoutError::BlankLabel))
    );
    assert_eq!(
        named(&ui).as_deref(),
        Some("Find pages"),
        "the rejected read is not committed"
    );
    name.set("Suchen".to_owned())?;
    frame(&mut ui);
    assert_eq!(named(&ui).as_deref(), Some("Suchen"));
    Ok(())
}
