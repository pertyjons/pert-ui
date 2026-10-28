//! How an `Each`'s items are addressed: the default container is internal
//! structure, a container given with `within` keeps its key, the
//! addressing a container mounted with survives a new description, and an
//! item `build` returns as a bare component or an `Element` is keyed by the
//! item.
//!
//! Each test is marked as a new-behavior test, which names the assertion that
//! detects the earlier `items` segment, or as a contract test of a rule this
//! addressing keeps.
use pert_ui::*;
use std::time::Duration;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn runtime() -> Result<Runtime, Box<dyn std::error::Error>> {
    let ui = Runtime::new();
    ui.set_viewport(Viewport::new(400.0, 300.0, 1.0)?)?;
    Ok(ui)
}

/// The tasks the lists below show.
fn tasks(ui: &Runtime) -> Signal<Vec<String>> {
    ui.signal(vec!["write".to_owned(), "ship".to_owned()])
}

/// An item: a label, which `Each` keys by the item.
fn label(task: &str) -> Label {
    Label::new(task)
}

/// An item holding a workspace participant, a scroll area keyed `scroll`.
fn scrolling(task: &str) -> Column {
    Column::new([Element::new(
        "scroll",
        ScrollArea::new(Element::new("body", Label::new(task))),
    )])
}

/// The workspace paths a capture records, sorted.
fn captured_paths(ui: &Runtime) -> Result<Vec<String>, WorkspaceError> {
    let mut paths: Vec<String> = ui
        .capture_workspace()?
        .entries()
        .map(|(path, _)| path.to_string())
        .collect();
    paths.sort();
    Ok(paths)
}

/// The item keyed `key` of the `Each` mounted as `each`, found by walking
/// keys and children, so the lookup does not depend on any addressing.
fn item(ui: &Runtime, each: &WidgetId, key: &str) -> Result<WidgetId, RuntimeError> {
    let container = ui.children(each)?.remove(0);
    for child in ui.children(&container)? {
        if ui.key(&child)? == key {
            return Ok(child);
        }
    }
    Err(RuntimeError::Unmounted)
}

/// The child keyed `key` of `parent`, found without any addressing.
fn child(ui: &Runtime, parent: &WidgetId, key: &str) -> Result<WidgetId, RuntimeError> {
    for child in ui.children(parent)? {
        if ui.key(&child)? == key {
            return Ok(child);
        }
    }
    Err(RuntimeError::Unmounted)
}

/// The inspection record of the one component keyed `key`.
fn inspected(ui: &Runtime, key: &str) -> ComponentInspection {
    let mut found = ui
        .query(&InspectionQuery::components(Selector::key(key)).detail(Detail::Semantics))
        .components;
    assert_eq!(found.len(), 1, "one component is keyed {key:?}");
    found.remove(0)
}

/// New-behavior test. Named assertions: `find(&each, &["ship"])` is the
/// item, and `["items", "ship"]` no longer resolves, because the default
/// container adds no segment.
#[test]
fn an_item_is_found_by_its_key_below_the_each() -> TestResult {
    let mut ui = runtime()?;
    let tasks = tasks(&ui);
    let each =
        ui.mount_tree(Each::new(&tasks, Clone::clone, |_, task| label(task)).key("tasks"))?;
    ui.flush(Duration::ZERO)?;
    let ship = item(&ui, &each, "ship")?;
    assert_eq!(ui.find(&each, &["ship"])?, Some(ship));
    assert_eq!(ui.find(&each, &["items", "ship"])?, None);
    // The container keeps its key for key lookups, without an address.
    let container = ui.find_descendant(&each, "items")?.ok_or("the container")?;
    assert_eq!(ui.children(&container)?.len(), 2);
    assert_eq!(ui.find(&each, &["items"])?, None);
    Ok(())
}

/// New-behavior test. Named assertion: the item's inspection path is
/// `["root", "tasks", "ship"]`, and `Selector::children` of the `Each`'s
/// path returns exactly the items.
#[test]
fn inspection_and_selectors_address_items_below_the_each() -> TestResult {
    let mut ui = runtime()?;
    let tasks = tasks(&ui);
    ui.mount_tree(
        Column::new([Each::new(&tasks, Clone::clone, |_, task| label(task)).key("tasks")])
            .key("root"),
    )?;
    ui.flush(Duration::ZERO)?;
    assert_eq!(
        inspected(&ui, "ship").path,
        Some(vec![
            "root".to_owned(),
            "tasks".to_owned(),
            "ship".to_owned()
        ])
    );
    assert_eq!(inspected(&ui, "items").path, None);
    let children = ui
        .query(&InspectionQuery::components(Selector::children([
            "root", "tasks",
        ])))
        .components;
    let mut keys: Vec<_> = children.iter().map(|c| c.key.as_str()).collect();
    keys.sort_unstable();
    assert_eq!(keys, ["ship", "write"]);
    assert!(
        ui.query(&InspectionQuery::components(Selector::path([
            "root", "tasks", "items"
        ])))
        .components
        .is_empty()
    );
    assert_eq!(
        ui.query(&InspectionQuery::components(Selector::path([
            "root", "tasks", "ship"
        ])))
        .components
        .len(),
        1
    );
    Ok(())
}

/// New-behavior test. Named assertion: an item keyed `items` is what
/// `find(&each, &["items"])` returns, and its participant captures at
/// `tasks/items/scroll`.
#[test]
fn an_item_keyed_items_owns_that_address() -> TestResult {
    let mut ui = runtime()?;
    let list = ui.signal(vec!["items".to_owned(), "other".to_owned()]);
    let each =
        ui.mount_tree(Each::new(&list, Clone::clone, |_, task| scrolling(task)).key("tasks"))?;
    ui.flush(Duration::ZERO)?;
    // The container keeps its key, so a key lookup sees two `items`; an
    // address names only the item.
    assert_eq!(
        ui.find_descendant(&each, "items"),
        Err(RuntimeError::AmbiguousKey("items".into()))
    );
    let container = ui.children(&each)?.remove(0);
    let item = ui.children(&container)?.remove(0);
    assert_eq!(ui.key(&item)?, "items");
    assert_eq!(ui.find(&each, &["items"])?, Some(item));
    assert_eq!(
        captured_paths(&ui)?,
        ["tasks/items/scroll", "tasks/other/scroll"]
    );
    Ok(())
}

/// New-behavior test. Named assertions: after a new description with an
/// explicit container under the same key, `["ship"]` still finds the same
/// item, `["items", "ship"]` does not resolve, and the participant still
/// captures at `tasks/ship/scroll`.
#[test]
fn a_default_container_keeps_its_addressing_when_described_again() -> TestResult {
    let mut ui = runtime()?;
    let tasks = tasks(&ui);
    let explicit = ui.signal(false);
    let root = ui.mount_tree(
        Dynamic::new(&explicit, move |_, explicit| {
            let each = Each::new(&tasks, Clone::clone, |_, task| scrolling(task));
            if *explicit {
                each.within(|items| Column::new(items).key("items"))
                    .key("tasks")
            } else {
                each.key("tasks")
            }
        })
        .key("list"),
    )?;
    ui.flush(Duration::ZERO)?;
    let each = child(&ui, &root, "tasks")?;
    let ship = item(&ui, &each, "ship")?;
    explicit.set(true)?;
    ui.flush(Duration::ZERO)?;
    assert_eq!(item(&ui, &each, "ship")?, ship, "the item is retained");
    assert_eq!(ui.find(&each, &["ship"])?, Some(ship));
    assert_eq!(ui.find(&each, &["items", "ship"])?, None);
    assert_eq!(
        captured_paths(&ui)?,
        ["list/tasks/ship/scroll", "list/tasks/write/scroll"]
    );
    Ok(())
}

/// Contract test: a container given with `within` keeps its addressing when
/// the `Each` is described again with the default container under the same
/// key: the same items, the same addresses, the same workspace paths.
#[test]
fn an_explicit_container_keeps_its_segment_when_described_again() -> TestResult {
    let mut ui = runtime()?;
    let tasks = tasks(&ui);
    let explicit = ui.signal(true);
    let root = ui.mount_tree(
        Dynamic::new(&explicit, move |_, explicit| {
            let each = Each::new(&tasks, Clone::clone, |_, task| scrolling(task));
            if *explicit {
                each.within(|items| Column::new(items).key("items"))
                    .key("tasks")
            } else {
                each.key("tasks")
            }
        })
        .key("list"),
    )?;
    ui.flush(Duration::ZERO)?;
    let each = ui.find(&root, &["tasks"])?.ok_or("the list")?;
    let ship = ui.find(&each, &["items", "ship"])?.ok_or("the item")?;
    explicit.set(false)?;
    ui.flush(Duration::ZERO)?;
    assert_eq!(ui.find(&each, &["items", "ship"])?, Some(ship));
    assert_eq!(ui.find(&each, &["ship"])?, None);
    assert_eq!(
        captured_paths(&ui)?,
        [
            "list/tasks/items/ship/scroll",
            "list/tasks/items/write/scroll"
        ]
    );
    Ok(())
}

/// Contract test: a container under another key mounts the items again,
/// with new identities, fresh state and that key in their addresses; another
/// container type under the same key fails the flush and keeps the items.
#[test]
fn a_replacement_key_mounts_the_items_again_and_a_type_change_fails() -> TestResult {
    let mut ui = runtime()?;
    let tasks = tasks(&ui);
    let container = ui.signal(0);
    let root = ui.mount_tree(
        Dynamic::new(&container, move |_, container| {
            let each = Each::new(&tasks, Clone::clone, |_, task| label(task));
            match container {
                0 => each.key("tasks"),
                1 => each
                    .within(|rows| Column::new(rows).key("rows"))
                    .key("tasks"),
                _ => each.within(|rows| Row::new(rows).key("rows")).key("tasks"),
            }
        })
        .key("list"),
    )?;
    ui.flush(Duration::ZERO)?;
    let each = ui.find(&root, &["tasks"])?.ok_or("the list")?;
    let ship = ui.find(&each, &["ship"])?.ok_or("the item")?;
    // Focus is state the item holds; a remounted item does not inherit it.
    ui.focus(&ship)?;
    container.set(1)?;
    ui.flush(Duration::ZERO)?;
    let moved = ui.find(&each, &["rows", "ship"])?.ok_or("mounted again")?;
    assert_ne!(moved, ship);
    assert_eq!(ui.key(&ship), Err(RuntimeError::Unmounted));
    assert_ne!(ui.focused(), Some(moved.clone()));
    assert_eq!(ui.find(&each, &["ship"])?, None);
    container.set(2)?;
    assert!(matches!(
        ui.flush(Duration::ZERO),
        Err(RuntimeError::ChildTypeChanged { .. })
    ));
    assert_eq!(ui.find(&each, &["rows", "ship"])?, Some(moved));
    Ok(())
}

/// Contract test: a container the application gives keeps its key in the
/// address, `items` included.
#[test]
fn a_within_container_keeps_the_segment_the_application_named() -> TestResult {
    let mut ui = runtime()?;
    let tasks = tasks(&ui);
    let named = ui.mount_tree(
        Each::new(&tasks, Clone::clone, |_, task| label(task))
            .within(|items| Column::new(items).key("items"))
            .key("named"),
    )?;
    ui.flush(Duration::ZERO)?;
    assert!(ui.find(&named, &["items", "ship"])?.is_some());
    assert_eq!(ui.find(&named, &["ship"])?, None);
    Ok(())
}

/// New-behavior test. Named assertions: an item the application marks
/// structural takes the `Each`'s label as its name, in inspection, in a
/// semantic selector and in content search, and a `Field` directly above the
/// `Each` names it and is its `labelled_by`.
#[test]
fn a_structural_item_is_named_through_the_default_container() -> TestResult {
    let mut ui = runtime()?;
    let batch = ui.signal(vec!["one".to_owned()]);
    let entries = ui.signal(vec!["one".to_owned()]);
    let root = ui.mount_tree(
        Column::new([
            Each::new(
                &batch,
                |_| "batch-item".to_owned(),
                |cx, _| Element::new("x", TextField::new(cx.signal(String::new()))).structural(),
            )
            .key("batch")
            .with_label("Batch"),
            Element::new(
                "field",
                Field::new(
                    Each::new(
                        &entries,
                        |_| "field-item".to_owned(),
                        |cx, _| {
                            Element::new("x", TextField::new(cx.signal(String::new()))).structural()
                        },
                    )
                    .key("entries"),
                )
                .label("Entry"),
            ),
        ])
        .key("root"),
    )?;
    ui.flush(Duration::ZERO)?;
    let item = inspected(&ui, "batch-item");
    let name = |c: &ComponentInspection| c.details.semantics.as_ref().and_then(|s| s.name.clone());
    assert_eq!(name(&item).as_deref(), Some("Batch"));
    let matched = ui
        .query(&InspectionQuery::components(Selector::semantic(
            SemanticRole::TextInput,
            Some("Batch"),
        )))
        .components;
    assert_eq!(matched.iter().map(|c| c.id).collect::<Vec<_>>(), [item.id]);
    let widget = ui.find_descendant(&root, "batch-item")?.ok_or("the item")?;
    let hits = ui
        .content_search(ContentQuery::new("Batch")?)
        .step(&ui)?
        .hits;
    assert!(
        hits.iter().any(|hit| hit.field == ContentField::Name
            && hit.target == ContentTarget::Widget(widget.clone())),
        "{hits:?}"
    );
    let field = inspected(&ui, "field");
    let item = inspected(&ui, "field-item");
    assert_eq!(name(&item).as_deref(), Some("Entry"));
    assert_eq!(item.labelled_by, Some(field.id));
    Ok(())
}

/// Contract test: an ordinary item keeps the naming it had, none, under a
/// labelled `Each` and under a `Field` above the `Each`, in inspection, in a
/// semantic selector and in content search.
#[test]
fn an_ordinary_item_is_not_named_by_the_each_or_a_field_above_it() -> TestResult {
    let mut ui = runtime()?;
    let batch = ui.signal(vec!["one".to_owned()]);
    let entries = ui.signal(vec!["one".to_owned()]);
    let root = ui.mount_tree(
        Column::new([
            Each::new(
                &batch,
                |_| "batch-item".to_owned(),
                |cx, _| TextField::new(cx.signal(String::new())).key("x"),
            )
            .key("batch")
            .with_label("Batch"),
            Element::new(
                "field",
                Field::new(
                    Each::new(
                        &entries,
                        |_| "field-item".to_owned(),
                        |cx, _| TextField::new(cx.signal(String::new())).key("x"),
                    )
                    .key("entries"),
                )
                .label("Entry"),
            ),
        ])
        .key("root"),
    )?;
    ui.flush(Duration::ZERO)?;
    for (key, name) in [("batch-item", "Batch"), ("field-item", "Entry")] {
        let item = inspected(&ui, key);
        let semantics = item.details.semantics.as_ref().ok_or("text field")?;
        assert_eq!(semantics.name, None, "{key}");
        assert_eq!(item.labelled_by, None, "{key}");
        let matched = ui
            .query(&InspectionQuery::components(Selector::semantic(
                SemanticRole::TextInput,
                Some(name),
            )))
            .components;
        assert!(matched.is_empty(), "{key}: {matched:?}");
        let widget = ui.find_descendant(&root, key)?.ok_or("the item")?;
        let hits = ui.content_search(ContentQuery::new(name)?).step(&ui)?.hits;
        assert!(
            hits.iter()
                .all(|hit| hit.target != ContentTarget::Widget(widget.clone())),
            "{key}: {hits:?}"
        );
    }
    Ok(())
}

/// What an `Each`'s `build` returns, written with the prelude alone: a bare
/// component, keyed by the item, or an `Element`, whose key the item's
/// replaces.
mod items {
    use pert_ui::prelude::*;
    use std::time::Duration;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    /// New-behavior test 10. Named assertion: after the first flush the one
    /// item, a bare `Label`, resolves under its item key `ship`, and its key
    /// is `ship`. Fails when the component conversion keys with a fixed
    /// string instead of the item's key (`find` returns `None`).
    #[test]
    fn a_bare_component_is_addressed_by_the_item_key() -> TestResult {
        let mut ui = Runtime::new();
        let tasks = ui.signal(vec!["ship".to_owned()]);
        let each = ui.mount_tree(
            Each::new(&tasks, Clone::clone, |_, task| Label::new(task.as_str())).key("tasks"),
        )?;
        ui.flush(Duration::ZERO)?;
        let ship = ui.find(&each, &["ship"])?.ok_or("the item")?;
        assert_eq!(ui.key(&ship)?, "ship");
        Ok(())
    }

    /// Contract test: a closure returning `Element`s of two component types
    /// compiles and addresses each by its item key; the elements' own key
    /// does not resolve.
    #[test]
    fn elements_of_two_component_types_are_addressed_by_item_key() -> TestResult {
        let mut ui = Runtime::new();
        let entries = ui.signal(vec!["Notes".to_owned(), "+".to_owned()]);
        let each = ui.mount_tree(
            Each::new(&entries, Clone::clone, |_, entry| -> Element {
                if entry == "+" {
                    Button::new("Add", |_| Ok(())).key("entry")
                } else {
                    Label::new(entry.as_str()).key("entry")
                }
            })
            .key("entries"),
        )?;
        ui.flush(Duration::ZERO)?;
        assert!(ui.find(&each, &["Notes"])?.is_some());
        assert!(ui.find(&each, &["+"])?.is_some());
        assert_eq!(ui.find(&each, &["entry"])?, None);
        Ok(())
    }

    /// Contract test: a duplicate item key among bare components fails the
    /// flush and keeps the previous items, and a valid list recovers.
    #[test]
    fn a_duplicate_key_among_bare_components_keeps_the_previous_items() -> TestResult {
        let mut ui = Runtime::new();
        let items = ui.signal(vec!["a".to_owned()]);
        let each = ui.mount_tree(
            Each::new(&items, Clone::clone, |_, item| Label::new(item.as_str())).key("each"),
        )?;
        ui.flush(Duration::ZERO)?;
        let a = ui.find(&each, &["a"])?.ok_or("mounted")?;
        items.set(vec!["a".to_owned(), "a".to_owned()])?;
        assert!(ui.flush(Duration::ZERO).is_err());
        assert_eq!(
            ui.find(&each, &["a"])?,
            Some(a.clone()),
            "the previous items stay"
        );
        items.set(vec!["a".to_owned(), "b".to_owned()])?;
        ui.flush(Duration::ZERO)?;
        assert_eq!(ui.find(&each, &["a"])?, Some(a));
        assert!(ui.find(&each, &["b"])?.is_some());
        Ok(())
    }
}
