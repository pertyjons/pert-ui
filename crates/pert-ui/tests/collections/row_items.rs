//! `RowItem`: collections build rows without keys and key them themselves,
//! with the same public conversion an external collection uses.
use pert_ui::component::*;
use pert_ui::prelude::*;
use pert_ui::testing::*;
use std::rc::Rc;

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// Rows keyed by name: even positions are labels, odd ones buttons, and the
/// row named "group" is an element with a label and two children.
struct Mixed(Vec<&'static str>);
impl ListItems for Mixed {
    fn len(&self) -> usize {
        self.0.len()
    }
    fn key(&self, index: usize) -> Option<String> {
        self.0.get(index).map(|name| (*name).to_owned())
    }
    fn build(&self, index: usize) -> Option<RowItem> {
        let name = *self.0.get(index)?;
        Some(match name {
            "group" => Element::new(
                "ignored",
                Column::new([Label::new("one").key("one"), Label::new("two").key("two")]),
            )
            .with_label("Grouped row")
            .into(),
            _ if index.is_multiple_of(2) => Label::new(name).into(),
            _ => Button::new(name, |_| Ok(())).into(),
        })
    }
}

fn mount(rows: Vec<&'static str>) -> Result<(Runtime, Signal<ListSource>, WidgetId), RuntimeError> {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(300.0, 400.0, 1.0)?)?;
    let source = ui.signal(ListSource::new(Mixed(rows)));
    let list = ui.mount_tree(Element::new(
        "list",
        VirtualList::new(source.read_only(), 40.0)?,
    ))?;
    Driver::new(&mut ui)
        .settle()
        .map_err(|_| RuntimeError::Unmounted)?;
    Ok((ui, source, list))
}

fn row(ui: &Runtime, list: &WidgetId, key: &str) -> WidgetId {
    ui.find_descendant(list, key)
        .unwrap()
        .unwrap_or_else(|| panic!("row {key}"))
}

#[test]
fn heterogeneous_rows_are_keyed_by_the_list_and_keep_element_metadata() -> TestResult {
    let (ui, _, list) = mount(vec!["alpha", "beta", "group"])?;
    let keys: Vec<String> = ui
        .children(&list)?
        .iter()
        .map(|id| ui.key(id).unwrap().to_owned())
        .collect();
    assert_eq!(
        keys,
        ["alpha", "beta", "group"],
        "the list's keys, not the rows'"
    );
    // The element row keeps its label and its children.
    let group = row(&ui, &list, "group");
    let children: Vec<String> = ui
        .children(&group)?
        .iter()
        .map(|id| ui.key(id).unwrap().to_owned())
        .collect();
    assert_eq!(children, ["one", "two"]);
    // Keying replaces only the key: the label stays.
    let keyed = Mixed(vec!["a", "b", "group"])
        .build(2)
        .ok_or("the row")?
        .into_element("group");
    assert_eq!((keyed.key(), keyed.label()), ("group", Some("Grouped row")));
    Ok(())
}

#[test]
fn reordering_keeps_identities_and_a_recreated_row_is_a_new_incarnation() -> TestResult {
    let (mut ui, source, list) = mount(vec!["alpha", "beta", "gamma"])?;
    let alpha = row(&ui, &list, "alpha");
    let gamma = row(&ui, &list, "gamma");
    // A new source with the same keys in another order reconciles in place.
    source.set(ListSource::new(Mixed(vec!["gamma", "beta", "alpha"])))?;
    Driver::new(&mut ui).settle()?;
    assert_eq!(row(&ui, &list, "alpha"), alpha);
    assert_eq!(row(&ui, &list, "gamma"), gamma);
    // Removing a row and adding it back makes the old identity stale.
    source.set(ListSource::new(Mixed(vec!["gamma", "beta"])))?;
    Driver::new(&mut ui).settle()?;
    source.set(ListSource::new(Mixed(vec!["gamma", "beta", "alpha"])))?;
    Driver::new(&mut ui).settle()?;
    let again = row(&ui, &list, "alpha");
    assert_ne!(again, alpha);
    assert_eq!(ui.event(&alpha, |_| ()), Err(RuntimeError::Unmounted));
    Ok(())
}

/// An external collection: a column of the rows `items` builds, keyed with
/// the public conversion the built-in lists use.
struct Stack(Option<Rc<dyn ListItems>>);
impl Component for Stack {
    fn take_children(&mut self) -> Vec<Element> {
        let Some(items) = self.0.take() else {
            return Vec::new();
        };
        (0..items.len())
            .filter_map(|index| Some(items.build(index)?.into_element(items.key(index)?)))
            .collect()
    }
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
}

#[test]
fn an_external_collection_keys_rows_through_a_trait_object() -> TestResult {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(300.0, 400.0, 1.0)?)?;
    let items: Rc<dyn ListItems> = Rc::new(Mixed(vec!["alpha", "beta"]));
    let stack = ui.mount_tree(Element::new("stack", Stack(Some(items))))?;
    Driver::new(&mut ui).settle()?;
    let keys: Vec<String> = ui
        .children(&stack)?
        .iter()
        .map(|id| ui.key(id).unwrap().to_owned())
        .collect();
    assert_eq!(keys, ["alpha", "beta"]);
    Ok(())
}

/// A tree whose rows are built without keys.
struct Files;
impl TreeItems for Files {
    fn roots(&self) -> Vec<String> {
        vec!["docs".into(), "notes".into()]
    }
    fn children(&self, _: &str) -> Vec<String> {
        Vec::new()
    }
    fn build(&self, key: &str) -> Option<RowItem> {
        Some(Label::new(key).into())
    }
}

#[test]
fn a_tree_built_through_a_trait_object_keys_its_rows() -> TestResult {
    let items: Box<dyn TreeItems> = Box::new(Files);
    assert!(items.build("docs").is_some());
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(300.0, 400.0, 1.0)?)?;
    let selection = ListSelection::new(&ui, SelectionMode::Single);
    let tree = TreeController::new(&ui, selection, TreeSource::new(Files))?;
    ui.mount_tree(Element::new("tree", Tree::new(&tree, 28.0)?))?;
    Driver::new(&mut ui).settle()?;
    // The tree keys each row and its content with the node's key.
    let keys: Vec<String> = ui
        .inspect()
        .components
        .into_iter()
        .map(|component| component.key)
        .collect();
    for node in ["docs", "notes"] {
        assert!(keys.iter().any(|key| key == node), "{node} in {keys:?}");
    }
    Ok(())
}

/// A row component that keeps its children, counts its mounts and shows the
/// text its latest description carries.
struct Badge {
    text: String,
    mounts: Rc<std::cell::Cell<usize>>,
    children: Vec<Element>,
}
impl Component for Badge {
    fn take_children(&mut self) -> Vec<Element> {
        std::mem::take(&mut self.children)
    }
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
    fn mounted(&mut self) {
        self.mounts.set(self.mounts.get() + 1);
    }
    fn reconcile(&mut self, next: Self) -> Option<Invalidation> {
        (self.text != next.text).then(|| {
            self.text = next.text;
            Invalidation::Paint
        })
    }
    fn inspect(&self) -> ComponentDetails {
        ComponentDetails {
            semantics: Some(Semantics::new(SemanticRole::Group).name(self.text.as_str())),
            ..ComponentDetails::default()
        }
    }
}

/// One badge row, keyed "badge", converted straight from the component.
struct Badges {
    text: &'static str,
    mounts: Rc<std::cell::Cell<usize>>,
}
impl ListItems for Badges {
    fn len(&self) -> usize {
        1
    }
    fn key(&self, index: usize) -> Option<String> {
        (index == 0).then(|| "badge".to_owned())
    }
    fn build(&self, index: usize) -> Option<RowItem> {
        (index == 0).then(|| {
            Badge {
                text: self.text.to_owned(),
                mounts: self.mounts.clone(),
                children: vec![Label::new("inner").key("inner")],
            }
            .into()
        })
    }
}

#[test]
fn a_component_row_is_reconciled_in_place_through_its_typed_function() -> TestResult {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(300.0, 200.0, 1.0)?)?;
    let mounts = Rc::new(std::cell::Cell::new(0));
    let source = ui.signal(ListSource::new(Badges {
        text: "first",
        mounts: mounts.clone(),
    }));
    let list = ui.mount_tree(Element::new(
        "list",
        VirtualList::new(source.read_only(), 40.0)?,
    ))?;
    Driver::new(&mut ui).settle()?;
    let badge = row(&ui, &list, "badge");
    assert!(
        ui.find_descendant(&badge, "inner")?.is_some(),
        "its initial child"
    );
    let name = |ui: &Runtime| -> Result<Option<String>, RuntimeError> {
        let id = ui.inspection_id(&badge)?;
        Ok(ui
            .inspect()
            .components
            .into_iter()
            .find(|component| component.id == id)
            .and_then(|component| component.details.semantics)
            .and_then(|semantics| semantics.name))
    };
    assert_eq!(name(&ui)?.as_deref(), Some("first"));
    source.set(ListSource::new(Badges {
        text: "second",
        mounts: mounts.clone(),
    }))?;
    Driver::new(&mut ui).settle()?;
    assert_eq!(row(&ui, &list, "badge"), badge, "the same incarnation");
    assert_eq!(
        name(&ui)?.as_deref(),
        Some("second"),
        "reconciled with the new text"
    );
    assert_eq!(mounts.get(), 1, "never mounted again");
    Ok(())
}
