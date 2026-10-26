//! `CollectionFilter::hidden_active`: what an unfocused filter change does
//! with an active item it leaves hidden, for a list, a table and a tree, and
//! the cases where the policy leaves the item alone.
use crate::support;
use pert_ui::*;
use std::time::Duration;
use support::{settle, widget};

type TestResult = Result<(), Box<dyn std::error::Error>>;
const ROW: f32 = 20.0;
const POLICIES: [HiddenActive; 3] = [
    HiddenActive::Keep,
    HiddenActive::Nearest,
    HiddenActive::Clear,
];

const FRUIT: [&str; 6] = ["Apple", "Apricot", "Banana", "Blueberry", "Cherry", "Grape"];
struct Fruit;
impl ListItems for Fruit {
    fn len(&self) -> usize {
        FRUIT.len()
    }
    fn key(&self, index: usize) -> Option<String> {
        FRUIT.get(index).map(|name| name.to_lowercase())
    }
    fn build(&self, index: usize) -> Option<Element> {
        Some(Element::new(self.key(index)?, Label::new(FRUIT[index])))
    }
    fn text(&self, index: usize) -> Option<String> {
        FRUIT.get(index).map(|name| (*name).to_owned())
    }
}

/// A filter over a collection, the collection's selection and its query.
struct Stage {
    ui: Runtime,
    root: WidgetId,
    query: Signal<String>,
    selection: ListSelection,
}
impl Stage {
    fn active(&self) -> Option<String> {
        self.selection.active().get().expect("readable")
    }
    fn selected(&self) -> Vec<String> {
        self.selection
            .selected()
            .get()
            .expect("readable")
            .iter()
            .cloned()
            .collect()
    }
    fn set_active(&mut self, key: &str) {
        self.selection
            .set_active(Some(key.to_owned()))
            .expect("writable");
        settle(&mut self.ui);
    }
    /// Types `query` as the user does: with focus in the field.
    fn type_query(&mut self, query: &str) {
        let field = widget(&self.ui, &self.root, "filter");
        self.ui.focus(&field).expect("focusable");
        self.query.set(query.to_owned()).expect("writable");
        settle(&mut self.ui);
    }
    /// Changes the query while the collection, keyed `collection`, has focus.
    fn query_in_collection(&mut self, query: &str) {
        let collection = widget(&self.ui, &self.root, "collection");
        self.ui.focus(&collection).expect("focusable");
        settle(&mut self.ui);
        self.query.set(query.to_owned()).expect("writable");
        settle(&mut self.ui);
    }
}

fn mount(
    mut ui: Runtime,
    target: impl Into<FilterTarget>,
    collection: Element,
    selection: ListSelection,
    policy: HiddenActive,
) -> Result<Stage, Box<dyn std::error::Error>> {
    ui.set_viewport(Viewport::new(400.0, 400.0, 1.0)?)?;
    let query = ui.signal(String::new());
    let root = ui.mount_tree(Element::new(
        "filtered",
        CollectionFilter::with_query(query.clone(), target, collection).hidden_active(policy),
    ))?;
    ui.flush(Duration::ZERO)?;
    Ok(Stage {
        ui,
        root,
        query,
        selection,
    })
}

fn list(policy: HiddenActive, mode: SelectionMode) -> Result<Stage, Box<dyn std::error::Error>> {
    let ui = Runtime::new();
    let source = ui.signal(ListSource::new(Fruit));
    let selection = ListSelection::new(&ui, mode);
    let filter = ListFilter::new(&ui, source.read_only())?.selection(selection.clone());
    let list = VirtualList::new(filter.items(), ROW)?.selection(selection.clone());
    mount(
        ui,
        filter,
        Element::new("collection", list),
        selection,
        policy,
    )
}

/// Plants with a name and a light column; `with` keeps only the named rows.
struct Plants(&'static [&'static str]);
const PLANTS: [(&str, &str, &str); 3] = [
    ("fern", "Fern", "Shade"),
    ("moss", "Moss", "Shade"),
    ("oak", "Oak", "Sun"),
];
impl Plants {
    fn rows(&self) -> Vec<(&'static str, &'static str, &'static str)> {
        PLANTS
            .into_iter()
            .filter(|(key, _, _)| self.0.contains(key))
            .collect()
    }
}
impl TableItems for Plants {
    fn len(&self) -> usize {
        self.rows().len()
    }
    fn key(&self, index: usize) -> Option<String> {
        self.rows().get(index).map(|row| row.0.to_owned())
    }
    fn cell(&self, index: usize, column: &str) -> Option<Element> {
        Some(Element::new(column, Label::new(self.text(index, column)?)))
    }
    fn text(&self, index: usize, column: &str) -> Option<String> {
        let row = *self.rows().get(index)?;
        Some(if column == "name" { row.1 } else { row.2 }.to_owned())
    }
}
const ALL_PLANTS: &[&str] = &["fern", "moss", "oak"];

fn table(policy: HiddenActive) -> Result<(Stage, TableController), Box<dyn std::error::Error>> {
    let ui = Runtime::new();
    let selection = ListSelection::new(&ui, SelectionMode::Single);
    let table = TableController::new(
        &ui,
        selection.clone(),
        [
            TableColumn::new("name", "Name"),
            TableColumn::new("light", "Light"),
        ],
        TableSource::new(Plants(ALL_PLANTS)),
    )?;
    let view = Element::new("collection", Table::new(&table, ROW)?);
    Ok((mount(ui, table.clone(), view, selection, policy)?, table))
}

/// `music` holds `jazz` and `rock`, `jazz` holds `bebop` while `bebop` is
/// true; `notes` is a leaf.
struct Library {
    bebop: bool,
}
impl TreeItems for Library {
    fn roots(&self) -> Vec<String> {
        vec!["music".into(), "notes".into()]
    }
    fn children(&self, key: &str) -> Vec<String> {
        match key {
            "music" => vec!["jazz".into(), "rock".into()],
            "jazz" if self.bebop => vec!["bebop".into()],
            _ => Vec::new(),
        }
    }
    fn can_expand(&self, key: &str) -> bool {
        key == "music" || (key == "jazz" && self.bebop)
    }
    fn known_children(&self, key: &str) -> Option<Vec<String>> {
        Some(self.children(key))
    }
    fn build(&self, key: &str) -> Option<Element> {
        Some(Element::new(key, Label::new(key)))
    }
    fn text(&self, key: &str) -> Option<String> {
        Some(key.to_owned())
    }
}

fn tree(policy: HiddenActive) -> Result<(Stage, TreeController), Box<dyn std::error::Error>> {
    let ui = Runtime::new();
    let selection = ListSelection::new(&ui, SelectionMode::Single);
    let tree = TreeController::new(
        &ui,
        selection.clone(),
        TreeSource::new(Library { bebop: true }),
    )?;
    let view = Element::new("collection", Tree::new(&tree, ROW)?);
    Ok((mount(ui, tree.clone(), view, selection, policy)?, tree))
}

#[test]
fn nearest_moves_a_list_item_the_field_hides_to_the_next_or_previous_row() -> TestResult {
    let mut stage = list(HiddenActive::Nearest, SelectionMode::Single)?;
    stage.set_active("banana");
    // "ap" keeps Apple, Apricot and Grape: Grape follows Banana.
    stage.type_query("ap");
    assert_eq!(stage.active().as_deref(), Some("grape"));
    assert_eq!(stage.selected(), ["grape"], "a single selection follows");
    // At the end there is no next row: the previous one.
    stage.type_query("");
    stage.set_active("grape");
    stage.type_query("an");
    assert_eq!(stage.active().as_deref(), Some("banana"));
    Ok(())
}

#[test]
fn clear_leaves_no_list_item_active_when_the_field_hides_it() -> TestResult {
    let mut stage = list(HiddenActive::Clear, SelectionMode::Single)?;
    stage.set_active("banana");
    stage.type_query("ap");
    assert_eq!(stage.active(), None);
    assert!(stage.selected().is_empty());
    let snapshot = stage.ui.inspect();
    let filter = snapshot
        .components
        .iter()
        .find(|c| c.key == "filtered")
        .ok_or("the filter")?;
    assert_eq!(
        filter.details.properties.get("hidden_active"),
        Some(&InspectValue::Text("clear".into()))
    );
    Ok(())
}

#[test]
fn clear_clears_when_no_row_remains_and_when_the_item_was_already_hidden() -> TestResult {
    let mut stage = list(HiddenActive::Clear, SelectionMode::Single)?;
    stage.set_active("banana");
    stage.type_query("zzz");
    assert_eq!(stage.active(), None, "no row remains");

    let mut stage = list(HiddenActive::Clear, SelectionMode::Single)?;
    stage.type_query("ap");
    // The application makes a hidden row active; the next change clears it.
    stage.set_active("cherry");
    stage.type_query("apr");
    assert_eq!(stage.active(), None, "it was hidden before this change");
    Ok(())
}

#[test]
fn the_policy_follows_the_field_for_a_table_and_a_tree() -> TestResult {
    let (mut stage, _) = table(HiddenActive::Nearest)?;
    stage.set_active("fern");
    stage.type_query("sun");
    assert_eq!(stage.active().as_deref(), Some("oak"));
    let (mut stage, _) = table(HiddenActive::Clear)?;
    stage.set_active("fern");
    stage.type_query("sun");
    assert_eq!(stage.active(), None);

    // A tree shows `rock` with its ancestor `music`; `notes` is hidden, so
    // the nearest row is the last one before it in the rows shown before
    // the change, where `music` was collapsed.
    let (mut stage, _) = tree(HiddenActive::Nearest)?;
    stage.set_active("notes");
    stage.type_query("rock");
    assert_eq!(stage.active().as_deref(), Some("music"));
    let (mut stage, _) = tree(HiddenActive::Clear)?;
    stage.set_active("notes");
    stage.type_query("rock");
    assert_eq!(stage.active(), None);
    Ok(())
}

#[test]
fn a_table_replacement_that_hides_the_active_row_follows_the_policy() -> TestResult {
    let (mut stage, table) = table(HiddenActive::Nearest)?;
    stage.set_active("fern");
    table.set_source(TableSource::new(Plants(&["moss", "oak"])))?;
    settle(&mut stage.ui);
    assert_eq!(stage.active().as_deref(), Some("moss"));

    let (mut stage, table) = self::table(HiddenActive::Clear)?;
    stage.set_active("fern");
    table.set_source(TableSource::new(Plants(&["moss", "oak"])))?;
    settle(&mut stage.ui);
    assert_eq!(stage.active(), None);
    Ok(())
}

#[test]
fn a_tree_replacement_hides_a_node_shown_through_the_temporary_expansion() -> TestResult {
    for (policy, expected) in [
        (HiddenActive::Nearest, Some("rock")),
        (HiddenActive::Clear, None),
    ] {
        let (mut stage, tree) = tree(policy)?;
        // `bebop` is shown only through the filter's temporary expansion.
        stage.type_query("o");
        stage.set_active("bebop");
        tree.set_source(TreeSource::new(Library { bebop: false }))?;
        settle(&mut stage.ui);
        assert_eq!(stage.active().as_deref(), expected, "{policy:?}");
    }
    Ok(())
}

#[test]
fn clear_clears_after_two_replacements_before_one_delivery() -> TestResult {
    let (mut stage, table) = table(HiddenActive::Clear)?;
    stage.set_active("fern");
    table.set_source(TableSource::new(Plants(&["moss", "oak"])))?;
    table.set_source(TableSource::new(Plants(&["oak"])))?;
    settle(&mut stage.ui);
    assert_eq!(stage.active(), None);
    Ok(())
}

#[test]
fn a_query_with_a_replacement_in_one_delivery_supplies_the_transition() -> TestResult {
    let (mut stage, table) = table(HiddenActive::Nearest)?;
    stage.set_active("fern");
    let field = widget(&stage.ui, &stage.root, "filter");
    stage.ui.focus(&field)?;
    // The replacement keeps `fern`; the query over the new rows hides it.
    table.set_source(TableSource::new(Plants(ALL_PLANTS)))?;
    stage.query.set("sun".to_owned())?;
    settle(&mut stage.ui);
    assert_eq!(stage.active().as_deref(), Some("oak"));
    Ok(())
}

#[test]
fn a_multiple_selection_keeps_its_hidden_keys_under_either_policy() -> TestResult {
    let mut stage = list(HiddenActive::Nearest, SelectionMode::Multiple)?;
    stage
        .selection
        .select(["banana".to_owned(), "blueberry".to_owned()])?;
    stage.set_active("banana");
    stage.type_query("ap");
    assert_eq!(stage.active().as_deref(), Some("grape"));
    assert_eq!(stage.selected(), ["banana", "blueberry"]);

    let mut stage = list(HiddenActive::Clear, SelectionMode::Multiple)?;
    stage
        .selection
        .select(["banana".to_owned(), "blueberry".to_owned()])?;
    stage.set_active("banana");
    stage.type_query("ap");
    assert_eq!(stage.active(), None);
    assert_eq!(stage.selected(), ["banana", "blueberry"]);
    Ok(())
}

// Compatibility checks: these outcomes are the same with the policy logic
// removed.

#[test]
fn keep_leaves_the_item_active_and_hidden() -> TestResult {
    let mut stage = list(HiddenActive::Keep, SelectionMode::Single)?;
    stage.set_active("banana");
    stage.type_query("ap");
    assert_eq!(stage.active().as_deref(), Some("banana"));
    Ok(())
}

#[test]
fn nearest_keeps_the_item_without_a_transition_of_its_own() -> TestResult {
    // No row remains.
    let mut stage = list(HiddenActive::Nearest, SelectionMode::Single)?;
    stage.set_active("banana");
    stage.type_query("zzz");
    assert_eq!(stage.active().as_deref(), Some("banana"));
    // It was already hidden.
    let mut stage = list(HiddenActive::Nearest, SelectionMode::Single)?;
    stage.type_query("ap");
    stage.set_active("cherry");
    stage.type_query("apr");
    assert_eq!(stage.active().as_deref(), Some("cherry"));
    // The application made another hidden row active before the delivery.
    let (mut stage, table) = table(HiddenActive::Nearest)?;
    stage.set_active("fern");
    table.set_source(TableSource::new(Plants(&["moss"])))?;
    stage.selection.set_active(Some("oak".to_owned()))?;
    settle(&mut stage.ui);
    assert_eq!(stage.active().as_deref(), Some("oak"));
    // Two replacements before one delivery: the second found it hidden.
    let (mut stage, table) = self::table(HiddenActive::Nearest)?;
    stage.set_active("fern");
    table.set_source(TableSource::new(Plants(&["moss", "oak"])))?;
    table.set_source(TableSource::new(Plants(&["oak"])))?;
    settle(&mut stage.ui);
    assert_eq!(stage.active().as_deref(), Some("fern"));
    Ok(())
}

#[test]
fn with_focus_in_the_collection_every_policy_keeps_todays_rule() -> TestResult {
    for policy in POLICIES {
        let mut stage = list(policy, SelectionMode::Single)?;
        stage.set_active("banana");
        stage.query_in_collection("ap");
        assert_eq!(stage.active().as_deref(), Some("grape"), "{policy:?}");
        // No row remains with a query: focus moves to the field and the
        // hidden item stays active.
        stage.set_active("apple");
        stage.query_in_collection("zzz");
        let field = widget(&stage.ui, &stage.root, "filter");
        assert_eq!(stage.ui.focused(), Some(field), "{policy:?}");
        assert_eq!(stage.active().as_deref(), Some("apple"), "{policy:?}");
    }
    Ok(())
}
