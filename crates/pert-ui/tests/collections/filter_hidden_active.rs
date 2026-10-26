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

/// The filtered collections every behavior runs for.
#[derive(Clone, Copy, Debug)]
enum Kind {
    List,
    Table,
    Tree,
}
const KINDS: [Kind; 3] = [Kind::List, Kind::Table, Kind::Tree];
/// The collections whose controllers replace their source.
const REPLACING: [Kind; 2] = [Kind::Table, Kind::Tree];

/// Keys and queries of one collection's rows.
struct Case {
    /// The item the tests make active.
    active: &'static str,
    /// Another key `hide` hides, selected beside `active` in `Multiple` mode.
    also: &'static str,
    /// A query that hides `active` and `also` while rows remain.
    hide: &'static str,
    /// The row still shown nearest to `active` after `hide`.
    nearest: &'static str,
    /// Another query that hides `active` while rows remain.
    narrower: &'static str,
}

impl Kind {
    fn case(self) -> Case {
        match self {
            // "ap" keeps Apple, Apricot and Grape: Grape follows Banana.
            Kind::List => Case {
                active: "banana",
                also: "blueberry",
                hide: "ap",
                nearest: "grape",
                narrower: "apr",
            },
            // "sun" keeps Oak, which follows Fern.
            Kind::Table => Case {
                active: "fern",
                also: "moss",
                hide: "sun",
                nearest: "oak",
                narrower: "su",
            },
            // "rock" shows `rock` with its ancestor `music`; `notes` is
            // hidden, and the nearest row is the last one before it in the
            // rows shown before the change, where `music` was collapsed.
            Kind::Tree => Case {
                active: "notes",
                also: "jazz",
                hide: "rock",
                nearest: "music",
                narrower: "roc",
            },
        }
    }
}

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

/// A table's or a tree's controller, which replaces its source.
enum Controller {
    Table(TableController),
    Tree(TreeController),
}

/// A source replacement and what it does to the rows.
#[derive(Clone, Copy, Debug)]
enum Replacement {
    /// The same rows.
    Same,
    /// Hides the replacement case's active item; a row remains.
    Hide,
    /// Hides it too when applied after `Hide`, and hides `stray`.
    HideMore,
}

/// The keys of a replacement that hides the active item.
struct Replaced {
    /// The query set first, if the hidden item shows only through it.
    query: &'static str,
    /// The item the tests make active.
    active: &'static str,
    /// The row still shown nearest to it after `Replacement::Hide`.
    nearest: &'static str,
    /// Another key `Replacement::HideMore` leaves hidden.
    stray: &'static str,
}

impl Controller {
    fn replace(&self, replacement: Replacement) -> Result<(), RuntimeError> {
        match self {
            Controller::Table(table) => {
                table.set_source(TableSource::new(Plants(match replacement {
                    Replacement::Same => ALL_PLANTS,
                    Replacement::Hide => &["moss", "oak"],
                    Replacement::HideMore => &["oak"],
                })))
            }
            Controller::Tree(tree) => tree.set_source(TreeSource::new(Library {
                bebop: matches!(replacement, Replacement::Same),
            })),
        }
    }
    fn replaced(&self) -> Replaced {
        match self {
            Controller::Table(_) => Replaced {
                query: "",
                active: "fern",
                nearest: "moss",
                stray: "moss",
            },
            // `bebop` is shown only through the filter's temporary expansion
            // of "o", and `jazz` only as its ancestor.
            Controller::Tree(_) => Replaced {
                query: "o",
                active: "bebop",
                nearest: "rock",
                stray: "jazz",
            },
        }
    }
}

/// Describes an element again, as an owner does for each description.
type Build = Box<dyn Fn() -> Result<Element, RuntimeError>>;

/// A filter over a collection, the collection's selection and its query.
struct Stage {
    ui: Runtime,
    root: WidgetId,
    query: Signal<String>,
    selection: ListSelection,
    controller: Option<Controller>,
    describe: Box<dyn Fn(HiddenActive) -> Result<Element, RuntimeError>>,
}
impl Stage {
    fn active(&self) -> Option<String> {
        self.selection.active().get().expect("readable")
    }
    fn selected(&self) -> Vec<String> {
        let mut keys: Vec<String> = self
            .selection
            .selected()
            .get()
            .expect("readable")
            .iter()
            .cloned()
            .collect();
        keys.sort();
        keys
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
    /// Changes the query while the collection, keyed `collection`, has focus
    /// where a press puts it: on the collection, or on a table's body.
    fn query_in_collection(&mut self, query: &str) {
        let collection = widget(&self.ui, &self.root, "collection");
        let mut targets = vec![collection.clone()];
        targets.extend(self.ui.children(&collection).expect("mounted"));
        let target = targets
            .into_iter()
            .find(|id| self.ui.can_focus(id).expect("mounted"))
            .expect("a focusable collection");
        self.ui.focus(&target).expect("focusable");
        settle(&mut self.ui);
        self.query.set(query.to_owned()).expect("writable");
        settle(&mut self.ui);
    }
    fn controller(&self) -> &Controller {
        self.controller.as_ref().expect("a replacing collection")
    }
    /// Describes the filter again with `policy`.
    fn redescribe(&mut self, policy: HiddenActive) -> TestResult {
        let filter = (self.describe)(policy)?;
        self.ui.reconcile_children(&self.root, vec![filter])?;
        settle(&mut self.ui);
        Ok(())
    }
    fn inspected_policy(&self) -> Option<InspectValue> {
        self.ui
            .inspect()
            .components
            .into_iter()
            .find(|c| c.key == "filtered")
            .and_then(|c| c.details.properties.get("hidden_active").cloned())
    }
}

fn stage(kind: Kind, policy: HiddenActive) -> Result<Stage, Box<dyn std::error::Error>> {
    stage_with(kind, policy, SelectionMode::Single)
}

fn stage_with(
    kind: Kind,
    policy: HiddenActive,
    mode: SelectionMode,
) -> Result<Stage, Box<dyn std::error::Error>> {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(400.0, 400.0, 1.0)?)?;
    let query = ui.signal(String::new());
    let selection = ListSelection::new(&ui, mode);
    let (target, collection, controller): (FilterTarget, Build, _) = match kind {
        Kind::List => {
            let source = ui.signal(ListSource::new(Fruit));
            let filter = ListFilter::new(&ui, source.read_only())?.selection(selection.clone());
            let items = filter.items();
            let rows = selection.clone();
            let view = move || {
                Ok(Element::new(
                    "collection",
                    VirtualList::new(items.clone(), ROW)?.selection(rows.clone()),
                ))
            };
            (filter.into(), Box::new(view), None)
        }
        Kind::Table => {
            let table = TableController::new(
                &ui,
                selection.clone(),
                [
                    TableColumn::new("name", "Name"),
                    TableColumn::new("light", "Light"),
                ],
                TableSource::new(Plants(ALL_PLANTS)),
            )?;
            let shown = table.clone();
            let view = move || Ok(Element::new("collection", Table::new(&shown, ROW)?));
            (
                table.clone().into(),
                Box::new(view),
                Some(Controller::Table(table)),
            )
        }
        Kind::Tree => {
            let tree = TreeController::new(
                &ui,
                selection.clone(),
                TreeSource::new(Library { bebop: true }),
            )?;
            let shown = tree.clone();
            let view = move || Ok(Element::new("collection", Tree::new(&shown, ROW)?));
            (
                tree.clone().into(),
                Box::new(view),
                Some(Controller::Tree(tree)),
            )
        }
    };
    let filtered = query.clone();
    let describe = Box::new(move |policy| {
        Ok(Element::new(
            "filtered",
            CollectionFilter::with_query(filtered.clone(), target.clone(), collection()?)
                .hidden_active(policy),
        ))
    });
    let root = ui.mount_tree(Element::new("root", Column::new([describe(policy)?])))?;
    ui.flush(Duration::ZERO)?;
    Ok(Stage {
        ui,
        root,
        query,
        selection,
        controller,
        describe,
    })
}

#[test]
fn nearest_moves_an_item_the_field_hides_to_the_nearest_shown_row() -> TestResult {
    for kind in KINDS {
        let case = kind.case();
        let mut stage = stage(kind, HiddenActive::Nearest)?;
        stage.set_active(case.active);
        stage.type_query(case.hide);
        assert_eq!(stage.active().as_deref(), Some(case.nearest), "{kind:?}");
        assert_eq!(
            stage.selected(),
            [case.nearest],
            "{kind:?}: selection follows"
        );
    }
    // At a list's end there is no next row: the previous one.
    let mut stage = stage(Kind::List, HiddenActive::Nearest)?;
    stage.set_active("grape");
    stage.type_query("an");
    assert_eq!(stage.active().as_deref(), Some("banana"));
    Ok(())
}

#[test]
fn clear_leaves_no_item_active_when_the_field_hides_it() -> TestResult {
    for kind in KINDS {
        let case = kind.case();
        let mut stage = stage(kind, HiddenActive::Clear)?;
        stage.set_active(case.active);
        stage.type_query(case.hide);
        assert_eq!(stage.active(), None, "{kind:?}");
        assert!(stage.selected().is_empty(), "{kind:?}");
        assert_eq!(
            stage.inspected_policy(),
            Some(InspectValue::Text("clear".into())),
            "{kind:?}"
        );
    }
    Ok(())
}

#[test]
fn clear_clears_when_no_row_remains_and_when_the_item_was_already_hidden() -> TestResult {
    for kind in KINDS {
        let case = kind.case();
        let mut stage = stage(kind, HiddenActive::Clear)?;
        stage.set_active(case.active);
        stage.type_query("zzz");
        assert_eq!(stage.active(), None, "{kind:?}: no row remains");

        let mut stage = self::stage(kind, HiddenActive::Clear)?;
        stage.type_query(case.hide);
        // The application makes a hidden row active; the next change clears it.
        stage.set_active(case.active);
        stage.type_query(case.narrower);
        assert_eq!(stage.active(), None, "{kind:?}: hidden before this change");
    }
    Ok(())
}

#[test]
fn a_replacement_that_hides_the_active_item_follows_the_policy() -> TestResult {
    for kind in REPLACING {
        for (policy, nearest) in [(HiddenActive::Nearest, true), (HiddenActive::Clear, false)] {
            let mut stage = stage(kind, policy)?;
            let replaced = stage.controller().replaced();
            stage.type_query(replaced.query);
            stage.set_active(replaced.active);
            stage.controller().replace(Replacement::Hide)?;
            settle(&mut stage.ui);
            let expected = nearest.then_some(replaced.nearest);
            assert_eq!(stage.active().as_deref(), expected, "{kind:?} {policy:?}");
        }
    }
    Ok(())
}

#[test]
fn clear_clears_after_two_replacements_before_one_delivery() -> TestResult {
    for kind in REPLACING {
        let mut stage = stage(kind, HiddenActive::Clear)?;
        let replaced = stage.controller().replaced();
        stage.type_query(replaced.query);
        stage.set_active(replaced.active);
        stage.controller().replace(Replacement::Hide)?;
        stage.controller().replace(Replacement::HideMore)?;
        settle(&mut stage.ui);
        assert_eq!(stage.active(), None, "{kind:?}");
    }
    Ok(())
}

#[test]
fn clear_clears_an_item_the_application_made_active_before_delivery() -> TestResult {
    for kind in REPLACING {
        let mut stage = stage(kind, HiddenActive::Clear)?;
        let replaced = stage.controller().replaced();
        stage.type_query(replaced.query);
        stage.set_active(replaced.active);
        stage.controller().replace(Replacement::HideMore)?;
        stage
            .selection
            .set_active(Some(replaced.stray.to_owned()))?;
        settle(&mut stage.ui);
        assert_eq!(stage.active(), None, "{kind:?}");
    }
    Ok(())
}

#[test]
fn a_query_with_a_replacement_in_one_delivery_supplies_the_transition() -> TestResult {
    for kind in REPLACING {
        let case = kind.case();
        let mut stage = stage(kind, HiddenActive::Nearest)?;
        stage.set_active(case.active);
        let field = widget(&stage.ui, &stage.root, "filter");
        stage.ui.focus(&field)?;
        // The replacement keeps the item; the query over the new rows hides it.
        stage.controller().replace(Replacement::Same)?;
        stage.query.set(case.hide.to_owned())?;
        settle(&mut stage.ui);
        assert_eq!(stage.active().as_deref(), Some(case.nearest), "{kind:?}");
    }
    Ok(())
}

#[test]
fn a_multiple_selection_keeps_its_hidden_keys_under_either_policy() -> TestResult {
    for kind in KINDS {
        let case = kind.case();
        let mut both = vec![case.active.to_owned(), case.also.to_owned()];
        both.sort();
        for (policy, expected) in [
            (HiddenActive::Nearest, Some(case.nearest)),
            (HiddenActive::Clear, None),
        ] {
            let mut stage = stage_with(kind, policy, SelectionMode::Multiple)?;
            stage.selection.select(both.clone())?;
            stage.set_active(case.active);
            stage.type_query(case.hide);
            assert_eq!(stage.active().as_deref(), expected, "{kind:?} {policy:?}");
            assert_eq!(stage.selected(), both, "{kind:?} {policy:?}");
        }
    }
    Ok(())
}

#[test]
fn a_redescribed_policy_applies_from_the_next_change() -> TestResult {
    for kind in KINDS {
        let case = kind.case();
        let mut stage = stage(kind, HiddenActive::Keep)?;
        stage.set_active(case.active);
        stage.type_query(case.hide);
        assert_eq!(stage.active().as_deref(), Some(case.active), "{kind:?}");
        // Changing the policy acts on nothing by itself.
        stage.redescribe(HiddenActive::Clear)?;
        assert_eq!(
            stage.inspected_policy(),
            Some(InspectValue::Text("clear".into())),
            "{kind:?}"
        );
        assert_eq!(stage.active().as_deref(), Some(case.active), "{kind:?}");
        // The next delivered change applies it.
        stage.type_query(case.narrower);
        assert_eq!(stage.active(), None, "{kind:?}");
    }
    Ok(())
}

// Compatibility checks: these outcomes are the same with the policy logic
// removed.

#[test]
fn keep_leaves_the_item_active_and_hidden() -> TestResult {
    for kind in KINDS {
        let case = kind.case();
        let mut stage = stage(kind, HiddenActive::Keep)?;
        stage.set_active(case.active);
        stage.type_query(case.hide);
        assert_eq!(stage.active().as_deref(), Some(case.active), "{kind:?}");
    }
    Ok(())
}

#[test]
fn nearest_keeps_the_item_without_a_transition_of_its_own() -> TestResult {
    for kind in KINDS {
        let case = kind.case();
        // No row remains.
        let mut stage = stage(kind, HiddenActive::Nearest)?;
        stage.set_active(case.active);
        stage.type_query("zzz");
        assert_eq!(stage.active().as_deref(), Some(case.active), "{kind:?}");
        // It was already hidden.
        let mut stage = self::stage(kind, HiddenActive::Nearest)?;
        stage.type_query(case.hide);
        stage.set_active(case.active);
        stage.type_query(case.narrower);
        assert_eq!(stage.active().as_deref(), Some(case.active), "{kind:?}");
    }
    for kind in REPLACING {
        // The application made another hidden row active before the delivery.
        let mut stage = stage(kind, HiddenActive::Nearest)?;
        let replaced = stage.controller().replaced();
        stage.type_query(replaced.query);
        stage.set_active(replaced.active);
        stage.controller().replace(Replacement::HideMore)?;
        stage
            .selection
            .set_active(Some(replaced.stray.to_owned()))?;
        settle(&mut stage.ui);
        assert_eq!(stage.active().as_deref(), Some(replaced.stray), "{kind:?}");
        // Two replacements before one delivery: the second found it hidden.
        let mut stage = self::stage(kind, HiddenActive::Nearest)?;
        stage.type_query(replaced.query);
        stage.set_active(replaced.active);
        stage.controller().replace(Replacement::Hide)?;
        stage.controller().replace(Replacement::HideMore)?;
        settle(&mut stage.ui);
        assert_eq!(stage.active().as_deref(), Some(replaced.active), "{kind:?}");
    }
    Ok(())
}

#[test]
fn with_focus_in_the_collection_every_policy_keeps_todays_rule() -> TestResult {
    for kind in KINDS {
        let case = kind.case();
        for policy in POLICIES {
            let mut stage = stage(kind, policy)?;
            stage.set_active(case.active);
            stage.query_in_collection(case.hide);
            assert_eq!(
                stage.active().as_deref(),
                Some(case.nearest),
                "{kind:?} {policy:?}"
            );
            // No row remains with a query: focus moves to the field and the
            // hidden item stays active.
            stage.query_in_collection("zzz");
            let field = widget(&stage.ui, &stage.root, "filter");
            assert_eq!(stage.ui.focused(), Some(field), "{kind:?} {policy:?}");
            assert_eq!(
                stage.active().as_deref(),
                Some(case.nearest),
                "{kind:?} {policy:?}"
            );
        }
    }
    Ok(())
}
