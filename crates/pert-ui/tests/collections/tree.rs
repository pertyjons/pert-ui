//! The tree as a consumer of the virtualized list and the shared selection.
use crate::support;
use pert_ui::*;
use std::{
    cell::RefCell,
    collections::BTreeSet,
    rc::{Rc, Weak},
    time::Duration,
};
use support::press;

type TestResult = Result<(), Box<dyn std::error::Error>>;
const ROW: f32 = 24.0;

/// `a` has children `a/0..a/2`; `a/1` has `a/1/x`; `b` is a leaf. Records which
/// nodes were asked for their children, to show that collapsed ones are not.
struct Outline {
    asked: Rc<RefCell<Vec<String>>>,
    _alive: Rc<()>,
}
impl TreeItems for Outline {
    fn roots(&self) -> Vec<String> {
        vec!["a".into(), "b".into()]
    }
    fn children(&self, key: &str) -> Vec<String> {
        self.asked.borrow_mut().push(key.to_owned());
        match key {
            "a" => (0..3).map(|index| format!("a/{index}")).collect(),
            "a/1" => vec!["a/1/x".into()],
            _ => Vec::new(),
        }
    }
    fn can_expand(&self, key: &str) -> bool {
        matches!(key, "a" | "a/1")
    }
    fn build(&self, key: &str) -> Option<Element> {
        Some(Element::new(key, Label::new(key)))
    }
}

struct Fixture {
    ui: Runtime,
    state: TreeController,
    selection: ListSelection,
    tree: WidgetId,
    asked: Rc<RefCell<Vec<String>>>,
    alive: Weak<()>,
}
fn fixture() -> Result<Fixture, Box<dyn std::error::Error>> {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(300.0, 200.0, 1.0)?)?;
    let asked = Rc::new(RefCell::new(Vec::new()));
    let alive = Rc::new(());
    let weak = Rc::downgrade(&alive);
    let selection = ListSelection::new(&ui, SelectionMode::Single);
    let state = TreeController::new(
        &ui,
        TreeSource::new(Outline {
            asked: asked.clone(),
            _alive: alive,
        }),
        selection.clone(),
    )?;
    let tree = ui.mount_tree(Element::new("tree", Tree::new(&state, ROW)?))?;
    ui.flush(Duration::ZERO)?;
    Ok(Fixture {
        ui,
        state,
        selection,
        tree,
        asked,
        alive: weak,
    })
}

#[test]
fn expander_has_local_hover_feedback() -> TestResult {
    let mut f = fixture()?;
    let color =
        f.ui.theme()
            .palette(ThemeMode::Dark)?
            .color(ColorRole::SurfaceHover);
    let has_small_hover = |ui: &mut Runtime| {
        ui.paint().unwrap().commands().iter().any(|command| {
            matches!(command,
                DrawCommand::FillRect { rect, color: fill }
                | DrawCommand::DrawMask { rect, color: fill, .. }
                if *fill == color && rect.width() <= 24.0)
        })
    };
    assert!(!has_small_hover(&mut f.ui));
    f.ui.pointer_input(PointerEvent::new(
        Point::new(10.0, 12.0)?,
        PointerKind::Move,
    ))?;
    f.ui.flush(Duration::ZERO)?;
    assert!(has_small_hover(&mut f.ui));
    f.ui.pointer_input(PointerEvent::new(
        Point::new(290.0, 180.0)?,
        PointerKind::Move,
    ))?;
    f.ui.flush(Duration::ZERO)?;
    assert!(!has_small_hover(&mut f.ui));
    Ok(())
}
fn rows(f: &Fixture) -> Vec<String> {
    f.ui.children(&f.tree)
        .unwrap()
        .iter()
        .map(|child| f.ui.key(child).unwrap().to_owned())
        .collect()
}
fn active(f: &Fixture) -> Option<String> {
    f.selection.active().get().unwrap()
}

#[test]
fn only_expanded_nodes_are_rows_and_only_they_are_asked_for_children() -> TestResult {
    let mut f = fixture()?;
    assert_eq!(rows(&f), ["a", "b"]);
    assert!(
        f.asked.borrow().is_empty(),
        "collapsed nodes produce no children"
    );

    assert!(f.state.set_expanded("a", true)?);
    assert!(!f.state.set_expanded("a", true)?, "unchanged");
    f.ui.flush(Duration::ZERO)?;
    assert_eq!(rows(&f), ["a", "a/0", "a/1", "a/2", "b"]);
    assert_eq!(*f.asked.borrow(), ["a"], "a/1 is still collapsed");

    // A row that keeps its key keeps its component across the change.
    let before = f.ui.children(&f.tree)?;
    f.state.set_expanded("a/1", true)?;
    f.ui.flush(Duration::ZERO)?;
    assert_eq!(rows(&f), ["a", "a/0", "a/1", "a/1/x", "a/2", "b"]);
    let after = f.ui.children(&f.tree)?;
    assert_eq!(before[0], after[0]);
    assert_eq!(before[4], after[5], "`b` moved down and stayed itself");

    // Each level is indented by one IconSize, after the expander slot.
    let step = ThemeMetrics::default().get(MetricRole::IconSize);
    let content = |f: &Fixture, row: usize| {
        let row = &f.ui.children(&f.tree).unwrap()[row];
        f.ui.bounds(&f.ui.children(row).unwrap()[0])
            .unwrap()
            .unwrap()
            .x()
    };
    assert_eq!(content(&f, 0), step);
    assert_eq!(content(&f, 1), 2.0 * step);
    assert_eq!(content(&f, 3), 3.0 * step);
    let root = &f.ui.children(&f.tree)?[0];
    let label = &f.ui.children(root)?[0];
    let row_bounds = f.ui.bounds(root)?.unwrap();
    let label_bounds = f.ui.bounds(label)?.unwrap();
    assert_eq!(
        label_bounds.y() + label_bounds.height() / 2.0,
        row_bounds.y() + row_bounds.height() / 2.0,
        "row content and expander share a vertical center"
    );
    assert_eq!(
        f.state.expanded().get()?,
        BTreeSet::from(["a".into(), "a/1".into()])
    );
    Ok(())
}

#[test]
fn the_cross_arrows_expand_descend_collapse_and_ascend() -> TestResult {
    let mut f = fixture()?;
    f.ui.focus(&f.tree)?;
    press(&mut f.ui, KeyCode::ArrowDown);
    assert_eq!(active(&f).as_deref(), Some("a"));
    press(&mut f.ui, KeyCode::ArrowRight);
    assert!(f.state.is_expanded("a")?, "Right expands a collapsed node");
    assert_eq!(active(&f).as_deref(), Some("a"));
    press(&mut f.ui, KeyCode::ArrowRight);
    assert_eq!(
        active(&f).as_deref(),
        Some("a/0"),
        "then moves to its first child"
    );
    press(&mut f.ui, KeyCode::ArrowRight);
    assert_eq!(
        active(&f).as_deref(),
        Some("a/0"),
        "a leaf has nowhere to go"
    );

    press(&mut f.ui, KeyCode::ArrowLeft);
    assert_eq!(active(&f).as_deref(), Some("a"), "Left moves to the parent");
    press(&mut f.ui, KeyCode::ArrowLeft);
    assert!(!f.state.is_expanded("a")?, "then collapses it");
    assert_eq!(rows(&f), ["a", "b"]);
    // Up and Down are the list's, over the visible nodes.
    press(&mut f.ui, KeyCode::End);
    assert_eq!(active(&f).as_deref(), Some("b"));
    assert_eq!(*f.selection.selected().get()?, BTreeSet::from(["b".into()]));
    Ok(())
}

#[test]
fn collapsing_an_ancestor_moves_a_hidden_active_item_to_it() -> TestResult {
    let mut f = fixture()?;
    f.state.set_expanded("a", true)?;
    f.state.set_expanded("a/1", true)?;
    f.selection.set_active(Some("a/1/x".into()))?;
    f.selection.select(["a/1/x".to_owned()])?;
    f.ui.flush(Duration::ZERO)?;

    f.state.set_expanded("a", false)?;
    f.ui.flush(Duration::ZERO)?;
    assert_eq!(
        active(&f).as_deref(),
        Some("a"),
        "the nearest visible ancestor"
    );
    assert_eq!(*f.selection.selected().get()?, BTreeSet::from(["a".into()]));
    // Expansion is remembered by key, so reopening restores the inner level.
    f.state.set_expanded("a", true)?;
    f.ui.flush(Duration::ZERO)?;
    assert_eq!(rows(&f), ["a", "a/0", "a/1", "a/1/x", "a/2", "b"]);
    Ok(())
}

/// Children per parent; "" holds the top level. Every non-leaf is a folder.
#[derive(Clone)]
struct Folders(std::collections::BTreeMap<String, Vec<String>>);
impl TreeItems for Folders {
    fn roots(&self) -> Vec<String> {
        self.children("")
    }
    fn children(&self, key: &str) -> Vec<String> {
        self.0.get(key).cloned().unwrap_or_default()
    }
    fn can_expand(&self, key: &str) -> bool {
        key.ends_with('/')
    }
    fn build(&self, key: &str) -> Option<Element> {
        Some(Element::new(key, Label::new(key)))
    }
}
fn folders(entries: &[(&str, &[&str])]) -> Folders {
    Folders(
        entries
            .iter()
            .map(|(parent, children)| {
                let children = children.iter().map(|key| (*key).to_owned()).collect();
                ((*parent).to_owned(), children)
            })
            .collect(),
    )
}

#[test]
fn collapsing_keeps_a_multiple_selection_and_moves_only_the_active_item() -> TestResult {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(300.0, 200.0, 1.0)?)?;
    let selection = ListSelection::new(&ui, SelectionMode::Multiple);
    let source = folders(&[("", &["a/", "c"]), ("a/", &["x", "y"])]);
    let state = TreeController::new(&ui, TreeSource::new(source), selection.clone())?;
    let tree = ui.mount_tree(Element::new("tree", Tree::new(&state, ROW)?))?;
    state.set_expanded("a/", true)?;
    selection.select(["x".to_owned(), "c".to_owned()])?;
    selection.set_active(Some("x".into()))?;
    ui.flush(Duration::ZERO)?;

    state.set_expanded("a/", false)?;
    ui.flush(Duration::ZERO)?;
    assert_eq!(selection.active().get()?.as_deref(), Some("a/"));
    assert_eq!(
        *selection.selected().get()?,
        BTreeSet::from(["x".into(), "c".into()]),
        "a collapse is not a selection gesture"
    );
    // The collapsed node is also the anchor: Shift+Down ranges from it.
    ui.focus(&tree)?;
    ui.key_input(KeyCode::ShiftLeft, KeyAction::Press)?;
    press(&mut ui, KeyCode::ArrowDown);
    ui.key_input(KeyCode::ShiftLeft, KeyAction::Release)?;
    assert_eq!(
        *selection.selected().get()?,
        BTreeSet::from(["a/".into(), "c".into()])
    );
    Ok(())
}

#[test]
fn an_active_item_moved_into_a_collapsed_node_stays_active_and_selected() -> TestResult {
    for mode in [SelectionMode::Single, SelectionMode::Multiple] {
        let mut ui = Runtime::new();
        ui.set_viewport(Viewport::new(300.0, 200.0, 1.0)?)?;
        let selection = ListSelection::new(&ui, mode);
        let before = folders(&[("", &["a/", "b/"]), ("a/", &["x", "y"]), ("b/", &[])]);
        let state = TreeController::new(&ui, TreeSource::new(before), selection.clone())?;
        ui.mount_tree(Element::new("tree", Tree::new(&state, ROW)?))?;
        state.set_expanded("a/", true)?;
        selection.set_active(Some("x".into()))?;
        selection.select(["x".to_owned()])?;
        ui.flush(Duration::ZERO)?;

        // What `Tree::reorderable` asks the application to do for a drop into
        // the collapsed `b/`: publish the moved hierarchy.
        let after = folders(&[("", &["a/", "b/"]), ("a/", &["y"]), ("b/", &["x"])]);
        state.set_source(TreeSource::new(after))?;
        ui.flush(Duration::ZERO)?;
        assert_eq!(selection.active().get()?.as_deref(), Some("x"), "{mode:?}");
        assert_eq!(
            *selection.selected().get()?,
            BTreeSet::from(["x".into()]),
            "{mode:?}: the old parent is never chosen for the user"
        );
        // It also stays when its old parent is left empty, which shows that
        // parent as a leaf rather than as a collapsed node.
        let only = folders(&[("", &["a/", "b/"]), ("a/", &["x"]), ("b/", &[])]);
        state.set_source(TreeSource::new(only))?;
        ui.flush(Duration::ZERO)?;
        let emptied = folders(&[("", &["a/", "b/"]), ("a/", &[]), ("b/", &["x"])]);
        state.set_source(TreeSource::new(emptied))?;
        ui.flush(Duration::ZERO)?;
        assert_eq!(selection.active().get()?.as_deref(), Some("x"), "{mode:?}");
    }
    Ok(())
}

#[test]
fn pressing_the_expander_toggles_and_pressing_the_row_selects() -> TestResult {
    let mut f = fixture()?;
    let click = |ui: &mut Runtime, x: f32, y: f32| {
        for kind in [
            PointerKind::Press(PointerButton::Primary),
            PointerKind::Release(PointerButton::Primary),
        ] {
            ui.pointer_input(PointerEvent::new(Point::new(x, y).unwrap(), kind))
                .unwrap();
        }
        ui.flush(Duration::ZERO).unwrap();
    };
    // The expander occupies the first IconSize of a root row.
    click(&mut f.ui, 5.0, ROW / 2.0);
    assert!(f.state.is_expanded("a")?);
    assert_eq!(active(&f), None, "toggling is not selecting");
    click(&mut f.ui, 150.0, ROW / 2.0);
    assert_eq!(active(&f).as_deref(), Some("a"));
    assert!(f.state.is_expanded("a")?, "selecting is not toggling");

    let semantics =
        f.ui.inspect()
            .components
            .into_iter()
            .find(|component| component.key == "tree")
            .and_then(|component| component.details.semantics)
            .unwrap();
    assert_eq!(semantics.role, SemanticRole::Tree);
    assert_eq!(semantics.expanded, Some(true));
    assert!(semantics.actions.contains(&SemanticAction::Collapse));
    Ok(())
}

#[test]
fn dropping_the_state_and_the_tree_releases_the_source() -> TestResult {
    let mut f = fixture()?;
    f.state.set_expanded("a", true)?;
    f.ui.flush(Duration::ZERO)?;
    assert!(f.alive.upgrade().is_some());
    f.ui.unmount(&f.tree)?;
    f.ui.flush(Duration::ZERO)?;
    let Fixture {
        ui, state, alive, ..
    } = f;
    drop(state);
    drop(ui);
    assert!(
        alive.upgrade().is_none(),
        "no cycle keeps the hierarchy alive"
    );
    Ok(())
}

/// One root, `big`, with many leaf children.
struct Big;
impl TreeItems for Big {
    fn roots(&self) -> Vec<String> {
        vec!["big".into()]
    }
    fn children(&self, key: &str) -> Vec<String> {
        match key {
            "big" => (0..10_000).map(|index| format!("big/{index}")).collect(),
            _ => Vec::new(),
        }
    }
    fn can_expand(&self, key: &str) -> bool {
        key == "big"
    }
    fn build(&self, key: &str) -> Option<Element> {
        Some(Element::new(key, Label::new(key)))
    }
}

#[test]
fn the_controller_reveals_a_node_by_key_once_its_ancestors_are_expanded() -> TestResult {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(300.0, 200.0, 1.0)?)?;
    let selection = ListSelection::new(&ui, SelectionMode::Single);
    let state = TreeController::new(&ui, TreeSource::new(Big), selection.clone())?;
    let tree = ui.mount_tree(Element::new("tree", Tree::new(&state, ROW)?))?;
    ui.flush(Duration::ZERO)?;
    let keys = |ui: &Runtime| -> Vec<String> {
        ui.children(&tree)
            .unwrap()
            .iter()
            .map(|child| ui.key(child).unwrap().to_owned())
            .collect()
    };

    // A node inside a collapsed parent is not a row, so nothing happens.
    let _ = state.reveal("big/7777")?;
    ui.flush(Duration::ZERO)?;
    assert_eq!(keys(&ui), ["big"]);

    // Expanding publishes the rows at once, so the next reveal finds the node.
    state.set_expanded("big", true)?;
    let _ = state.reveal("big/7777")?;
    ui.flush(Duration::ZERO)?;
    let row = ui
        .children(&tree)?
        .into_iter()
        .find(|id| ui.key(id).unwrap() == "big/7777")
        .expect("the revealed node is mounted");
    let [_, y, _, h] = ui.bounds(&row)?.unwrap().values();
    assert!(y >= 0.0 && y + h <= 200.0, "revealed at {y}");
    assert!(!keys(&ui).contains(&"big".to_owned()), "scrolled away");
    assert_eq!(
        selection.active().get()?,
        None,
        "the selection is untouched"
    );
    Ok(())
}

struct ConstructTreeInUpdate {
    constructed: Rc<RefCell<Option<TreeController>>>,
}
impl Component for ConstructTreeInUpdate {
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        if self.constructed.borrow().is_none() {
            let controller = TreeController::new(
                cx,
                TreeSource::new(Outline {
                    asked: Rc::new(RefCell::new(Vec::new())),
                    _alive: Rc::new(()),
                }),
                ListSelection::new(cx, SelectionMode::Single),
            )?;
            cx.request_children(vec![Element::new("tree", Tree::new(&controller, ROW)?)]);
            *self.constructed.borrow_mut() = Some(controller);
        }
        Ok(())
    }
}
#[test]
fn a_tree_controller_can_be_created_while_describing_children() -> TestResult {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(300.0, 200.0, 1.0)?)?;
    let constructed = Rc::new(RefCell::new(None));
    let root = ui.mount_tree(Element::new(
        "root",
        ConstructTreeInUpdate {
            constructed: constructed.clone(),
        },
    ))?;
    ui.flush(Duration::ZERO)?;
    assert!(constructed.borrow().is_some());
    assert_eq!(ui.children(&root)?.len(), 1);
    assert!(
        ui.inspect()
            .components
            .iter()
            .any(|component| component.key == "a")
    );
    Ok(())
}

struct RejectTreeMutation {
    state: TreeController,
    trigger: Signal<bool>,
}
impl Component for RejectTreeMutation {
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        if cx.read(&self.trigger.read_only())? {
            assert!(matches!(
                self.state.filter("a", &FilterMatcher::default()),
                Err(RuntimeError::WriteDuringUpdate)
            ));
            assert!(matches!(
                self.state.set_source(TreeSource::new(Outline {
                    asked: Rc::new(RefCell::new(Vec::new())),
                    _alive: Rc::new(())
                })),
                Err(RuntimeError::WriteDuringUpdate)
            ));
            assert_eq!(
                self.state.set_expanded("a", false),
                Err(RuntimeError::WriteDuringUpdate)
            );
        }
        Ok(())
    }
}
#[test]
fn refused_tree_mutations_preserve_the_visible_snapshot_and_navigation() -> TestResult {
    let mut f = fixture()?;
    f.state.set_expanded("a", true)?;
    f.ui.flush(Duration::ZERO)?;
    let before = f.ui.inspect();
    let trigger = f.ui.signal(true);
    let other = f.ui.mount(
        "probe",
        RejectTreeMutation {
            state: f.state.clone(),
            trigger,
        },
    )?;
    f.ui.flush(Duration::ZERO)?;
    assert!(f.state.is_expanded("a")?);
    assert_eq!(f.state.filter_counts().get()?, FilterCounts::default());
    let after = f.ui.inspect();
    for key in ["a", "a/0", "a/1", "a/2", "b"] {
        assert_eq!(
            before
                .components
                .iter()
                .find(|c| c.key == key)
                .map(|c| c.visible),
            after
                .components
                .iter()
                .find(|c| c.key == key)
                .map(|c| c.visible)
        );
    }
    f.ui.unmount(&other)?;
    f.selection.set_active(Some("a".to_owned()))?;
    f.ui.focus(&f.tree)?;
    press(&mut f.ui, KeyCode::ArrowDown);
    assert_eq!(f.selection.active().get()?.as_deref(), Some("a/0"));
    f.state.set_expanded("a", false)?;
    f.ui.flush(Duration::ZERO)?;
    assert!(!f.state.is_expanded("a")?);
    Ok(())
}
