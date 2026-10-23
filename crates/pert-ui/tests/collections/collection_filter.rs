//! Filter fields over lists, tables and trees, through the public facade.
use crate::support;
use pert_ui::*;
use std::{cell::RefCell, collections::BTreeSet, rc::Rc};
use support::{find, press, settle, widget};

type TestResult = Result<(), Box<dyn std::error::Error>>;
const ROW: f32 = 20.0;

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

struct ListFixture {
    ui: Runtime,
    root: WidgetId,
    filter: ListFilter,
    query: Signal<String>,
    selection: ListSelection,
}
fn list_fixture(
    mode: SelectionMode,
    offered: Permission,
) -> Result<ListFixture, Box<dyn std::error::Error>> {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(300.0, 400.0, 1.0)?)?;
    let source = ui.signal(ListSource::new(Fruit));
    let selection = ListSelection::new(&ui, mode);
    let filter = ListFilter::new(&ui, source.read_only())?.selection(selection.clone());
    let list = VirtualList::new(filter.items(), ROW)?.selection(selection.clone());
    let query = ui.signal(String::new());
    let root = ui.mount_tree(Element::new(
        "fruit",
        CollectionFilter::with_query(query.clone(), filter.clone(), Element::new("list", list))
            .name("Filter fruit")
            .offered(offered),
    ))?;
    settle(&mut ui);
    Ok(ListFixture {
        ui,
        root,
        filter,
        query,
        selection,
    })
}
fn shown(ui: &Runtime, key: &str) -> bool {
    ui.inspect()
        .components
        .into_iter()
        .any(|component| component.key == key && component.visible)
}
fn keys(source: &ListSource) -> Vec<String> {
    (0..source.len())
        .filter_map(|index| source.key(index))
        .collect()
}

#[test]
fn repeated_offered_permissions_cannot_lift_a_fixed_denial() -> TestResult {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(300.0, 400.0, 1.0)?)?;
    let allowed = ui.signal(true);
    let source = ui.signal(ListSource::new(Fruit));
    let filter = ListFilter::new(&ui, source.read_only())?;
    let query = ui.signal("Banana".to_owned());
    let list = VirtualList::new(filter.items(), ROW)?;
    ui.mount_tree(Element::new(
        "fruit",
        CollectionFilter::with_query(query, filter.clone(), Element::new("list", list))
            .offered(false)
            .offered(allowed.read_only()),
    ))?;
    settle(&mut ui);
    assert!(!shown(&ui, "filter-field"));
    assert_eq!(filter.items().get()?.len(), FRUIT.len());
    Ok(())
}
fn property(ui: &Runtime, key: &str, name: &str) -> InspectValue {
    ui.inspect()
        .components
        .into_iter()
        .find(|component| component.key == key)
        .and_then(|component| component.details.properties.get(name).cloned())
        .unwrap()
}

#[test]
fn a_list_filter_hides_rows_and_keeps_the_ones_it_shows() -> TestResult {
    let mut f = list_fixture(SelectionMode::Single, Permission::allowed())?;
    let banana = widget(&f.ui, &f.root, "banana");
    f.query.set("an".to_owned())?;
    settle(&mut f.ui);
    assert_eq!(keys(&f.filter.items().get()?), ["banana"]);
    assert_eq!(
        f.filter.counts().get()?,
        FilterCounts { shown: 1, total: 6 }
    );
    // The row that stays keeps its component, and hidden rows are unmounted.
    assert_eq!(widget(&f.ui, &f.root, "banana"), banana);
    assert!(find(&f.ui, &f.root, "apple").is_none());
    assert_eq!(
        property(&f.ui, "fruit", "query"),
        InspectValue::Text("an".into())
    );
    assert_eq!(property(&f.ui, "fruit", "shown"), InspectValue::Number(1.0));
    assert_eq!(property(&f.ui, "fruit", "total"), InspectValue::Number(6.0));
    // Clearing brings every row back under the same keys.
    f.query.set("  ".to_owned())?;
    settle(&mut f.ui);
    assert_eq!(keys(&f.filter.items().get()?).len(), 6);
    assert_eq!(widget(&f.ui, &f.root, "banana"), banana);
    Ok(())
}

#[test]
fn a_selected_row_filtered_out_stays_selected_and_unfocused_navigation_stays() -> TestResult {
    let mut f = list_fixture(SelectionMode::Multiple, Permission::allowed())?;
    f.selection
        .select(["apple".to_owned(), "cherry".to_owned()])?;
    f.selection.set_active(Some("apple".to_owned()))?;
    f.query.set("ch".to_owned())?;
    settle(&mut f.ui);
    assert_eq!(keys(&f.filter.items().get()?), ["cherry"]);
    assert!(f.selection.is_selected("apple")? && f.selection.is_selected("cherry")?);
    // Without focus in the collection nothing moves.
    assert_eq!(f.selection.active().get()?.as_deref(), Some("apple"));
    f.query.set(String::new())?;
    settle(&mut f.ui);
    assert!(f.selection.is_selected("apple")?);
    Ok(())
}

#[test]
fn focus_on_a_disappearing_row_moves_to_the_nearest_then_to_the_field() -> TestResult {
    let mut f = list_fixture(SelectionMode::Single, Permission::allowed())?;
    let list = widget(&f.ui, &f.root, "list");
    f.ui.focus(&list)?;
    f.selection.set_active(Some("banana".to_owned()))?;
    settle(&mut f.ui);
    // Banana disappears; the nearest remaining row after it is Blueberry.
    f.query.set("b".to_owned())?;
    settle(&mut f.ui);
    assert_eq!(keys(&f.filter.items().get()?), ["banana", "blueberry"]);
    f.query.set("ue".to_owned())?;
    settle(&mut f.ui);
    assert_eq!(f.selection.active().get()?.as_deref(), Some("blueberry"));
    // In single mode the selection is the active item, so it follows.
    assert!(f.selection.is_selected("blueberry")?);
    assert_eq!(f.ui.focused(), Some(list.clone()));
    // With no row left, focus goes to the field.
    f.query.set("zzz".to_owned())?;
    settle(&mut f.ui);
    assert_eq!(f.ui.focused(), Some(widget(&f.ui, &f.root, "filter")));
    Ok(())
}

#[test]
fn down_in_the_field_moves_focus_into_the_collection() -> TestResult {
    let mut f = list_fixture(SelectionMode::Single, Permission::allowed())?;
    let field = widget(&f.ui, &f.root, "filter");
    f.ui.focus(&field)?;
    settle(&mut f.ui);
    press(&mut f.ui, KeyCode::ArrowDown);
    assert_eq!(f.ui.focused(), Some(widget(&f.ui, &f.root, "list")));
    Ok(())
}

#[test]
fn a_filter_not_offered_hides_its_field_and_shows_every_row() -> TestResult {
    let mut f = list_fixture(SelectionMode::Single, Permission::denied())?;
    f.query.set("an".to_owned())?;
    settle(&mut f.ui);
    assert_eq!(keys(&f.filter.items().get()?).len(), 6);
    assert!(!shown(&f.ui, "filter"));
    assert_eq!(
        property(&f.ui, "fruit", "offered"),
        InspectValue::Bool(false)
    );
    // A bound permission offers it later, and the query then applies.
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(300.0, 400.0, 1.0)?)?;
    let offered = ui.signal(false);
    let source = ui.signal(ListSource::new(Fruit));
    let filter = ListFilter::new(&ui, source.read_only())?;
    let query = ui.signal("an".to_owned());
    ui.mount_tree(Element::new(
        "fruit",
        CollectionFilter::with_query(
            query,
            filter.clone(),
            Element::new("list", VirtualList::new(filter.items(), ROW)?),
        )
        .offered(offered.read_only()),
    ))?;
    settle(&mut ui);
    assert_eq!(filter.items().get()?.len(), 6);
    offered.set(true)?;
    settle(&mut ui);
    assert_eq!(keys(&filter.items().get()?), ["banana"]);
    assert!(shown(&ui, "filter"));
    Ok(())
}

#[test]
fn a_replaced_source_is_filtered_by_the_current_query() -> TestResult {
    struct Two;
    impl ListItems for Two {
        fn len(&self) -> usize {
            2
        }
        fn key(&self, index: usize) -> Option<String> {
            ["banana", "mango"].get(index).map(|key| (*key).to_owned())
        }
        fn build(&self, index: usize) -> Option<Element> {
            Some(Element::new(self.key(index)?, Label::new("row")))
        }
        fn text(&self, index: usize) -> Option<String> {
            ["Banana", "Mango"]
                .get(index)
                .map(|text| (*text).to_owned())
        }
    }
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(300.0, 400.0, 1.0)?)?;
    let source = ui.signal(ListSource::new(Fruit));
    let filter = ListFilter::new(&ui, source.read_only())?;
    let query = ui.signal("an".to_owned());
    ui.mount_tree(Element::new(
        "fruit",
        CollectionFilter::with_query(
            query,
            filter.clone(),
            Element::new("list", VirtualList::new(filter.items(), ROW)?),
        ),
    ))?;
    settle(&mut ui);
    assert_eq!(keys(&filter.items().get()?), ["banana"]);
    source.set(ListSource::new(Two))?;
    settle(&mut ui);
    assert_eq!(keys(&filter.items().get()?), ["banana", "mango"]);
    Ok(())
}

#[test]
fn a_replaced_matcher_applies_and_matches_a_fresh_mount() -> TestResult {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(300.0, 400.0, 1.0)?)?;
    let source = ui.signal(ListSource::new(Fruit));
    let filter = ListFilter::new(&ui, source.read_only())?;
    let query = ui.signal("bry".to_owned());
    let describe = |matcher: FilterMatcher| -> Result<Element, Box<dyn std::error::Error>> {
        Ok(Element::new(
            "fruit",
            CollectionFilter::with_query(
                query.clone(),
                filter.clone(),
                Element::new("list", VirtualList::new(filter.items(), ROW)?),
            )
            .matcher(matcher),
        ))
    };
    let root = ui.mount_tree(Element::new(
        "root",
        Column::new([describe(FilterMatcher::substring())?]),
    ))?;
    settle(&mut ui);
    assert!(filter.items().get()?.is_empty());
    ui.reconcile_children(&root, vec![describe(FilterMatcher::fuzzy())?])?;
    settle(&mut ui);
    assert_eq!(keys(&filter.items().get()?), ["blueberry"]);
    Ok(())
}

struct Plants;
impl TableItems for Plants {
    fn len(&self) -> usize {
        3
    }
    fn key(&self, index: usize) -> Option<String> {
        ["fern", "moss", "oak"]
            .get(index)
            .map(|key| (*key).to_owned())
    }
    fn cell(&self, index: usize, column: &str) -> Option<Element> {
        Some(Element::new(column, Label::new(self.text(index, column)?)))
    }
    fn text(&self, index: usize, column: &str) -> Option<String> {
        let row = [("Fern", "Shade"), ("Moss", "Shade"), ("Oak", "Sun")].get(index)?;
        Some(
            match column {
                "name" => row.0,
                _ => row.1,
            }
            .to_owned(),
        )
    }
}

#[test]
fn a_table_filter_matches_any_cell_and_survives_a_new_source() -> TestResult {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(400.0, 300.0, 1.0)?)?;
    let selection = ListSelection::new(&ui, SelectionMode::Single);
    let table = TableController::new(
        &ui,
        selection,
        [
            TableColumn::new("name", "Name"),
            TableColumn::new("light", "Light"),
        ],
        TableSource::new(Plants),
    )?;
    let query = ui.signal(String::new());
    let root = ui.mount_tree(Element::new(
        "plants",
        CollectionFilter::with_query(
            query.clone(),
            table.clone(),
            Element::new("table", Table::new(&table, ROW)?),
        ),
    ))?;
    settle(&mut ui);
    query.set("shade".to_owned())?;
    settle(&mut ui);
    assert_eq!(
        table.filter_counts().get()?,
        FilterCounts { shown: 2, total: 3 }
    );
    assert!(find(&ui, &root, "fern").is_some() && find(&ui, &root, "oak").is_none());
    table.set_source(TableSource::new(Plants))?;
    settle(&mut ui);
    assert_eq!(table.filter_counts().get()?.shown, 2);
    assert!(find(&ui, &root, "oak").is_none());
    Ok(())
}

/// `music` holds `jazz` (with `bebop`) and `rock`; `notes` is a leaf.
struct Library {
    asked: Rc<RefCell<Vec<String>>>,
    lazy: bool,
}
impl TreeItems for Library {
    fn roots(&self) -> Vec<String> {
        vec!["music".into(), "notes".into()]
    }
    fn children(&self, key: &str) -> Vec<String> {
        self.asked.borrow_mut().push(key.to_owned());
        match key {
            "music" => vec!["jazz".into(), "rock".into()],
            "jazz" => vec!["bebop".into()],
            _ => Vec::new(),
        }
    }
    fn can_expand(&self, key: &str) -> bool {
        matches!(key, "music" | "jazz")
    }
    fn known_children(&self, key: &str) -> Option<Vec<String>> {
        // A lazy source has listed only `music`.
        if self.lazy && key == "jazz" {
            return None;
        }
        Some(match key {
            "music" => vec!["jazz".into(), "rock".into()],
            "jazz" => vec!["bebop".into()],
            _ => Vec::new(),
        })
    }
    fn build(&self, key: &str) -> Option<Element> {
        Some(Element::new(key, Label::new(key)))
    }
    fn text(&self, key: &str) -> Option<String> {
        Some(key.to_owned())
    }
}
fn shown_rows(ui: &Runtime, _root: &WidgetId) -> Vec<String> {
    // A tree row and its content share the node's key.
    ["music", "jazz", "bebop", "rock", "notes"]
        .into_iter()
        .filter(|key| shown(ui, key))
        .map(str::to_owned)
        .collect()
}

#[test]
fn a_tree_filter_shows_matches_with_ancestors_through_a_temporary_layer() -> TestResult {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(300.0, 400.0, 1.0)?)?;
    let asked = Rc::new(RefCell::new(Vec::new()));
    let tree = TreeController::new(
        &ui,
        ListSelection::new(&ui, SelectionMode::Single),
        TreeSource::new(Library {
            asked: asked.clone(),
            lazy: false,
        }),
    )?;
    let query = ui.signal(String::new());
    let root = ui.mount_tree(Element::new(
        "library",
        CollectionFilter::with_query(
            query.clone(),
            tree.clone(),
            Element::new("tree", Tree::new(&tree, ROW)?),
        ),
    ))?;
    settle(&mut ui);
    assert_eq!(shown_rows(&ui, &root), ["music", "notes"]);
    query.set("bop".to_owned())?;
    settle(&mut ui);
    // The match deep inside collapsed nodes shows with its ancestors open.
    assert_eq!(shown_rows(&ui, &root), ["music", "jazz", "bebop"]);
    assert_eq!(
        tree.filter_counts().get()?,
        FilterCounts { shown: 1, total: 5 }
    );
    assert!(tree.expanded().get()?.is_empty());
    assert!(tree.is_expanded("jazz")?);
    // Collapsing while filtering changes only the layer.
    tree.set_expanded("jazz", false)?;
    settle(&mut ui);
    assert_eq!(shown_rows(&ui, &root), ["music", "jazz"]);
    assert!(tree.expanded().get()?.is_empty());
    // Clearing restores the user's own expansion, here nothing open.
    query.set(String::new())?;
    settle(&mut ui);
    assert_eq!(shown_rows(&ui, &root), ["music", "notes"]);
    tree.set_expanded("music", true)?;
    query.set("rock".to_owned())?;
    settle(&mut ui);
    assert_eq!(shown_rows(&ui, &root), ["music", "rock"]);
    query.set(String::new())?;
    settle(&mut ui);
    assert_eq!(shown_rows(&ui, &root), ["music", "jazz", "rock", "notes"]);
    assert_eq!(tree.expanded().get()?, BTreeSet::from(["music".to_owned()]));
    Ok(())
}

#[test]
fn a_lazy_tree_is_filtered_only_over_what_it_listed() -> TestResult {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(300.0, 400.0, 1.0)?)?;
    let asked = Rc::new(RefCell::new(Vec::new()));
    let tree = TreeController::new(
        &ui,
        ListSelection::new(&ui, SelectionMode::Single),
        TreeSource::new(Library {
            asked: asked.clone(),
            lazy: true,
        }),
    )?;
    let query = ui.signal(String::new());
    let root = ui.mount_tree(Element::new(
        "library",
        CollectionFilter::with_query(
            query.clone(),
            tree.clone(),
            Element::new("tree", Tree::new(&tree, ROW)?),
        ),
    ))?;
    settle(&mut ui);
    asked.borrow_mut().clear();
    query.set("bop".to_owned())?;
    settle(&mut ui);
    // `bebop` is under the unlisted `jazz`: not found, and nothing was asked.
    assert!(shown_rows(&ui, &root).is_empty());
    assert_eq!(
        tree.filter_counts().get()?,
        FilterCounts { shown: 0, total: 4 }
    );
    assert!(asked.borrow().is_empty());
    Ok(())
}

fn library_tree(ui: &Runtime, mode: SelectionMode) -> Result<TreeController, RuntimeError> {
    TreeController::new(
        ui,
        ListSelection::new(ui, mode),
        TreeSource::new(Library {
            asked: Rc::new(RefCell::new(Vec::new())),
            lazy: false,
        }),
    )
}

#[test]
fn a_query_change_never_moves_a_hidden_tree_selection() -> TestResult {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(300.0, 400.0, 1.0)?)?;
    let tree = library_tree(&ui, SelectionMode::Single)?;
    let query = ui.signal(String::new());
    let root = ui.mount_tree(Element::new(
        "library",
        CollectionFilter::with_query(
            query.clone(),
            tree.clone(),
            Element::new("tree", Tree::new(&tree, ROW)?),
        ),
    ))?;
    settle(&mut ui);
    query.set("bop".to_owned())?;
    settle(&mut ui);
    tree.selection().set_active(Some("bebop".to_owned()))?;
    tree.selection().select(["bebop".to_owned()])?;
    ui.focus(&widget(&ui, &root, "filter"))?;
    settle(&mut ui);
    // Only `jazz` matches now; `bebop` is hidden but keeps the selection.
    query.set("jazz".to_owned())?;
    settle(&mut ui);
    assert_eq!(shown_rows(&ui, &root), ["music", "jazz"]);
    assert_eq!(tree.selection().active().get()?.as_deref(), Some("bebop"));
    assert!(tree.selection().is_selected("bebop")?);
    Ok(())
}

/// Many rows whose `index_of` is the default linear search, counting every
/// key it reads.
struct Counted {
    rows: usize,
    reads: Rc<std::cell::Cell<usize>>,
}
impl ListItems for Counted {
    fn len(&self) -> usize {
        self.rows
    }
    fn key(&self, index: usize) -> Option<String> {
        self.reads.set(self.reads.get() + 1);
        (index < self.rows).then(|| format!("row-{index}"))
    }
    fn build(&self, index: usize) -> Option<Element> {
        Some(Element::new(self.key(index)?, Label::new("row")))
    }
    fn text(&self, index: usize) -> Option<String> {
        Some(format!("row {index}"))
    }
}

#[test]
fn hiding_the_active_row_finds_its_neighbor_in_linear_time() -> TestResult {
    let ui = Runtime::new();
    let rows = 3000;
    let reads = Rc::new(std::cell::Cell::new(0));
    let source = ui.signal(ListSource::new(Counted {
        rows,
        reads: reads.clone(),
    }));
    let selection = ListSelection::new(&ui, SelectionMode::Single);
    let filter = ListFilter::new(&ui, source.read_only())?.selection(selection.clone());
    selection.set_active(Some("row-0".to_owned()))?;
    reads.set(0);
    let outcome = filter.filter("2999", &FilterMatcher::substring())?;
    assert_eq!(outcome.nearest_active.as_deref(), Some("row-2999"));
    // Each row's key is read a bounded number of times, not once per row.
    assert!(reads.get() < 10 * rows, "{} key reads", reads.get());
    Ok(())
}

struct OnlyMoss;
impl TableItems for OnlyMoss {
    fn len(&self) -> usize {
        1
    }
    fn key(&self, index: usize) -> Option<String> {
        (index == 0).then(|| "moss".to_owned())
    }
    fn cell(&self, index: usize, column: &str) -> Option<Element> {
        Some(Element::new(column, Label::new(self.text(index, column)?)))
    }
    fn text(&self, index: usize, column: &str) -> Option<String> {
        (index == 0).then(|| if column == "name" { "Moss" } else { "Shade" }.to_owned())
    }
}

#[test]
fn a_new_table_source_moves_the_active_row_and_updates_the_counts() -> TestResult {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(400.0, 300.0, 1.0)?)?;
    let selection = ListSelection::new(&ui, SelectionMode::Single);
    let table = TableController::new(
        &ui,
        selection.clone(),
        [
            TableColumn::new("name", "Name"),
            TableColumn::new("light", "Light"),
        ],
        TableSource::new(Plants),
    )?;
    let query = ui.signal(String::new());
    let root = ui.mount_tree(Element::new(
        "plants",
        CollectionFilter::with_query(
            query.clone(),
            table.clone(),
            Element::new("table", Table::new(&table, ROW)?),
        ),
    ))?;
    settle(&mut ui);
    query.set("shade".to_owned())?;
    settle(&mut ui);
    ui.focus(&widget(&ui, &root, "table"))?;
    selection.set_active(Some("fern".to_owned()))?;
    settle(&mut ui);
    table.set_source(TableSource::new(OnlyMoss))?;
    settle(&mut ui);
    assert_eq!(selection.active().get()?.as_deref(), Some("moss"));
    assert_eq!(property(&ui, "plants", "shown"), InspectValue::Number(1.0));
    assert_eq!(property(&ui, "plants", "total"), InspectValue::Number(1.0));
    Ok(())
}

/// A hundred rows; the first ten say "drop", the others "keep".
struct Hundred;
impl ListItems for Hundred {
    fn len(&self) -> usize {
        100
    }
    fn key(&self, index: usize) -> Option<String> {
        (index < 100).then(|| format!("r{index}"))
    }
    fn build(&self, index: usize) -> Option<Element> {
        Some(Element::new(
            self.key(index)?,
            Label::new(format!("Row {index}")),
        ))
    }
    fn text(&self, index: usize) -> Option<String> {
        Some(if index < 10 { "drop" } else { "keep" }.to_owned())
    }
}

#[test]
fn filtering_keeps_the_top_row_where_it_was_by_key() -> TestResult {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(300.0, 200.0, 1.0)?)?;
    let source = ui.signal(ListSource::new(Hundred));
    let selection = ListSelection::new(&ui, SelectionMode::Single);
    let filter = ListFilter::new(&ui, source.read_only())?.selection(selection.clone());
    let list = VirtualList::new(filter.items(), ROW)?.selection(selection.clone());
    let query = ui.signal(String::new());
    let root = ui.mount_tree(Element::new(
        "rows",
        CollectionFilter::with_query(query.clone(), filter.clone(), Element::new("list", list)),
    ))?;
    settle(&mut ui);
    // Bringing row 60 into view scrolls well past the rows the filter drops.
    selection.set_active(Some("r60".to_owned()))?;
    settle(&mut ui);
    let before = ui.bounds(&widget(&ui, &root, "r60"))?.unwrap();
    query.set("keep".to_owned())?;
    settle(&mut ui);
    let after = ui.bounds(&widget(&ui, &root, "r60"))?.unwrap();
    assert_eq!(before.y(), after.y());
    Ok(())
}

fn described_filter(
    tree: &TreeController,
    query: &Signal<String>,
) -> Result<Element, GeometryError> {
    Ok(Element::new(
        "library",
        CollectionFilter::with_query(
            query.clone(),
            tree.clone(),
            Element::new("tree", Tree::new(tree, ROW)?),
        ),
    ))
}

#[test]
fn describing_the_filter_again_keeps_the_tree_layer() -> TestResult {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(300.0, 400.0, 1.0)?)?;
    let tree = library_tree(&ui, SelectionMode::Single)?;
    let query = ui.signal(String::new());
    let root = ui.mount_tree(Element::new(
        "root",
        Column::new([described_filter(&tree, &query)?]),
    ))?;
    settle(&mut ui);
    query.set("bop".to_owned())?;
    settle(&mut ui);
    tree.set_expanded("jazz", false)?;
    settle(&mut ui);
    assert_eq!(shown_rows(&ui, &root), ["music", "jazz"]);
    // An owner describing its parts again, as for another string table.
    ui.reconcile_children(&root, vec![described_filter(&tree, &query)?])?;
    settle(&mut ui);
    assert_eq!(shown_rows(&ui, &root), ["music", "jazz"]);
    Ok(())
}

/// Tries tree writes from its update, where signal writes are refused.
struct WritesInUpdate {
    tree: TreeController,
    results: Rc<RefCell<Vec<Result<(), RuntimeError>>>>,
}
impl Component for WritesInUpdate {
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        let mut results = self.results.borrow_mut();
        if results.is_empty() {
            results.push(self.tree.set_source(TreeSource::new(Other)));
            results.push(self.tree.set_expanded("jazz", true).map(|_| ()));
        }
        Ok(())
    }
}
struct Other;
impl TreeItems for Other {
    fn roots(&self) -> Vec<String> {
        vec!["other".into()]
    }
    fn children(&self, _: &str) -> Vec<String> {
        Vec::new()
    }
    fn build(&self, key: &str) -> Option<Element> {
        Some(Element::new(key, Label::new(key)))
    }
}

#[test]
fn a_refused_tree_write_changes_nothing() -> TestResult {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(300.0, 400.0, 1.0)?)?;
    let tree = library_tree(&ui, SelectionMode::Single)?;
    tree.filter("bop", &FilterMatcher::substring())?;
    tree.set_expanded("jazz", false)?;
    let results = Rc::new(RefCell::new(Vec::new()));
    let root = ui.mount_tree(Element::new(
        "root",
        Column::new([
            Element::new("tree", Tree::new(&tree, ROW)?),
            Element::new(
                "writer",
                WritesInUpdate {
                    tree: tree.clone(),
                    results: results.clone(),
                },
            ),
        ]),
    ))?;
    settle(&mut ui);
    assert_eq!(results.borrow().len(), 2);
    assert!(results.borrow().iter().all(Result::is_err));
    // The refused layer change left `jazz` collapsed in the layer, and the
    // refused source left the old hierarchy, which the next write publishes.
    assert!(!tree.is_expanded("jazz")?);
    tree.set_expanded("jazz", true)?;
    settle(&mut ui);
    assert_eq!(shown_rows(&ui, &root), ["music", "jazz", "bebop"]);
    assert!(find(&ui, &root, "other").is_none());
    Ok(())
}

#[test]
fn the_direct_filter_slot_reserves_filter_field_but_allows_filter_as_collection_key() -> TestResult
{
    for collection_key in ["filter", "filter-field"] {
        let mut ui = Runtime::new();
        ui.set_viewport(Viewport::new(300.0, 400.0, 1.0)?)?;
        let source = ui.signal(ListSource::new(Fruit));
        let filter = ListFilter::new(&ui, source.read_only())?;
        let list = VirtualList::new(filter.items(), ROW)?;
        let result = ui.mount_tree(Element::new(
            "fruit",
            CollectionFilter::new(&ui, filter, Element::new(collection_key, list)),
        ));
        if collection_key == "filter-field" {
            assert!(matches!(result, Err(RuntimeError::DuplicateKey(_))));
        } else {
            let root = result?;
            settle(&mut ui);
            let children = ui.children(&root)?;
            let keys: Vec<_> = children.iter().map(|id| ui.key(id).unwrap()).collect();
            assert_eq!(keys, ["filter-field", "filter"]);
        }
    }
    Ok(())
}

#[test]
fn the_default_name_uses_the_installed_table_before_the_first_update()
-> Result<(), Box<dyn std::error::Error>> {
    let mut ui = Runtime::new();
    ui.ui_settings()
        .set_strings(UiStrings::english().with_text(StringRole::FilterName, "Filtern")?)?;
    let source = ui.signal(ListSource::new(Fruit));
    let filter = ListFilter::new(&ui, source.read_only())?;
    let list = VirtualList::new(filter.items(), ROW)?;
    let query = ui.signal(String::new());
    ui.mount_tree(Element::new(
        "fruit",
        CollectionFilter::with_query(query, filter, Element::new("list", list)),
    ))?;
    let name = ui
        .inspect()
        .components
        .into_iter()
        .find(|c| c.key == "fruit")
        .and_then(|c| c.details.semantics)
        .and_then(|s| s.name);
    assert_eq!(name.as_deref(), Some("Filtern"));
    Ok(())
}
