//! Rearranging a `Tree` by drag and by keyboard, through the facade.
//!
//! The tree reuses the list's drag, so these pin what the tree adds: positions
//! among siblings, dropping into a node that takes children, the level chosen
//! by the pointer after a last child, the forbidden drop into a dragged node's
//! own subtree, the insertion line's indent and the outline's keyboard moves.
use pert_ui::*;
use std::{cell::RefCell, collections::BTreeMap, rc::Rc, time::Duration};

const ROW: f32 = 24.0;
// The default IconSize, which is one level of indent.
const STEP: f32 = 16.0;

/// Children per parent key; `""` holds the top level. A key ending in `/` is a
/// folder, which takes dropped children.
type Model = BTreeMap<String, Vec<String>>;

#[derive(Clone)]
struct Outline(Model);
impl TreeItems for Outline {
    fn roots(&self) -> Vec<String> {
        self.children("")
    }
    fn children(&self, key: &str) -> Vec<String> {
        self.0.get(key).cloned().unwrap_or_default()
    }
    fn can_expand(&self, key: &str) -> bool {
        key.ends_with('/')
    }
    fn can_drop_into(&self, key: &str) -> bool {
        key.ends_with('/')
    }
    fn text(&self, key: &str) -> Option<String> {
        Some(key.to_owned())
    }
    fn build(&self, key: &str) -> Option<Element> {
        Some(Element::new(key, Label::new(key)))
    }
}

/// Moves the dropped keys as one edit, the way an application would.
fn apply(model: &mut Model, drop: &ListDrop<'_>) {
    let items = drop.items().to_vec();
    let named = match drop.position() {
        DropPosition::Before(key) | DropPosition::After(key) | DropPosition::Into(key) => {
            Some(key.clone())
        }
        _ => None,
    };
    // A position beside a dragged node leaves everything where it is.
    if named.as_ref().is_some_and(|key| items.contains(key)) {
        return;
    }
    for children in model.values_mut() {
        children.retain(|key| !items.contains(key));
    }
    let parent_of = |model: &Model, key: &str| {
        model
            .iter()
            .find(|(_, children)| children.iter().any(|child| child == key))
            .map(|(parent, _)| parent.clone())
            .unwrap_or_default()
    };
    match drop.position() {
        DropPosition::Before(key) | DropPosition::After(key) => {
            let parent = parent_of(model, key);
            let siblings = model.entry(parent).or_default();
            let at = siblings.iter().position(|sibling| sibling == key).unwrap()
                + usize::from(matches!(drop.position(), DropPosition::After(_)));
            siblings.splice(at..at, items);
        }
        DropPosition::Into(key) => model.entry(key.clone()).or_default().extend(items),
        _ => model.entry(String::new()).or_default().extend(items),
    }
}

struct Stage {
    ui: Runtime,
    tree: WidgetId,
    state: TreeController,
    model: Rc<RefCell<Model>>,
    edits: Rc<RefCell<Vec<String>>>,
}

/// `music/` (expanded, holding `a` and `b`), the empty folder `docs/` and the
/// leaf `readme`, as rows 0 to 4.
fn stage(reorderable: bool) -> Stage {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(300.0, 240.0, 1.0).unwrap())
        .unwrap();
    let model: Model = [
        ("", vec!["music/", "docs/", "readme"]),
        ("music/", vec!["a", "b"]),
        ("docs/", vec![]),
    ]
    .into_iter()
    .map(|(parent, children)| {
        (
            parent.to_owned(),
            children.into_iter().map(str::to_owned).collect(),
        )
    })
    .collect();
    let model = Rc::new(RefCell::new(model));
    let selection = ListSelection::new(&ui, SelectionMode::Single);
    let state = TreeController::new(
        &ui,
        TreeSource::new(Outline(model.borrow().clone())),
        selection,
    )
    .unwrap();
    state.set_expanded("music/", true).unwrap();
    let edits = Rc::new(RefCell::new(Vec::new()));
    let mut tree = Tree::new(&state, ROW).unwrap();
    if reorderable {
        let (model, edits, state) = (model.clone(), edits.clone(), state.clone());
        tree = tree.reorderable(move |_, drop| {
            edits
                .borrow_mut()
                .push(format!("{:?} {:?}", drop.items(), drop.position()));
            apply(&mut model.borrow_mut(), drop);
            state.set_source(TreeSource::new(Outline(model.borrow().clone())))?;
            Ok(true)
        });
    }
    let tree = ui.mount_tree(Element::new("tree", tree)).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    Stage {
        ui,
        tree,
        state,
        model,
        edits,
    }
}

impl Stage {
    fn pointer(&mut self, x: f32, y: f32, kind: PointerKind) {
        self.ui
            .pointer_input(PointerEvent::new(Point::new(x, y).unwrap(), kind))
            .unwrap();
        self.ui.flush(self.ui.now()).unwrap();
    }
    /// Presses the row at `from`, clear of its expander, and moves the drag to
    /// `to` without releasing.
    fn hold(&mut self, from: f32, to: (f32, f32)) {
        self.pointer(60.0, from, PointerKind::Press(PointerButton::Primary));
        self.pointer(66.0, from, PointerKind::Move);
        assert!(
            self.ui.drag_active(),
            "crossing the threshold starts a drag"
        );
        self.pointer(to.0, to.1, PointerKind::Move);
    }
    fn drop_at(&mut self, from: f32, to: (f32, f32)) {
        self.hold(from, to);
        self.pointer(to.0, to.1, PointerKind::Release(PointerButton::Primary));
        assert!(!self.ui.drag_active());
    }
    fn children(&self, parent: &str) -> Vec<String> {
        self.model.borrow()[parent].clone()
    }
    fn property(&self, name: &str) -> Option<InspectValue> {
        let key = self.ui.key(&self.tree).unwrap();
        self.ui
            .inspect()
            .components
            .iter()
            .find(|component| component.key == key)
            .and_then(|component| component.details.properties.get(name).cloned())
    }
    fn offers_move(&self) -> bool {
        let key = self.ui.key(&self.tree).unwrap();
        self.ui
            .inspect()
            .components
            .iter()
            .find(|component| component.key == key)
            .and_then(|component| component.details.semantics.clone())
            .is_some_and(|semantics| semantics.actions.contains(&SemanticAction::Move))
    }
    fn alt(&mut self, key: KeyCode) {
        self.ui
            .key_input(KeyCode::AltLeft, KeyAction::Press)
            .unwrap();
        self.ui.key_input(key, KeyAction::Press).unwrap();
        self.ui.key_input(key, KeyAction::Release).unwrap();
        self.ui
            .key_input(KeyCode::AltLeft, KeyAction::Release)
            .unwrap();
        self.ui.flush(self.ui.now()).unwrap();
    }
}

fn row(index: usize, fraction: f32) -> f32 {
    (index as f32 + fraction) * ROW
}

#[test]
fn the_middle_of_a_folder_takes_the_node_as_its_last_child() {
    let mut stage = stage(true);
    stage.drop_at(row(4, 0.5), (60.0, row(3, 0.5)));
    assert_eq!(stage.children("docs/"), ["readme"]);
    assert_eq!(stage.children(""), ["music/", "docs/"]);
    assert_eq!(*stage.edits.borrow(), [r#"["readme"] Into("docs/")"#]);
}

#[test]
fn the_outer_quarters_of_a_folder_drop_beside_it() {
    let mut stage = stage(true);
    stage.drop_at(row(4, 0.5), (60.0, row(3, 0.1)));
    assert_eq!(stage.children(""), ["music/", "readme", "docs/"]);
    let mut stage = self::stage(true);
    stage.drop_at(row(1, 0.5), (60.0, row(3, 0.9)));
    assert_eq!(stage.children(""), ["music/", "docs/", "a", "readme"]);
    assert_eq!(stage.children("docs/"), Vec::<String>::new());
}

#[test]
fn a_leaf_divides_into_halves_and_never_takes_children() {
    let mut stage = stage(true);
    stage.drop_at(row(1, 0.5), (60.0, row(2, 0.6)));
    assert_eq!(stage.children("music/"), ["b", "a"]);
    assert_eq!(*stage.edits.borrow(), [r#"["a"] After("b")"#]);
}

#[test]
fn below_an_expanded_node_a_drop_goes_before_its_first_child() {
    let mut stage = stage(true);
    stage.drop_at(row(4, 0.5), (60.0, row(0, 0.9)));
    assert_eq!(stage.children("music/"), ["readme", "a", "b"]);
    assert_eq!(*stage.edits.borrow(), [r#"["readme"] Before("a")"#]);
}

#[test]
fn after_a_last_child_the_pointer_across_the_row_picks_the_level() {
    // Deep enough to stay inside the folder.
    let mut stage = stage(true);
    stage.drop_at(row(4, 0.5), (STEP * 3.0, row(2, 0.9)));
    assert_eq!(stage.children("music/"), ["a", "b", "readme"]);
    // At the outer level: after the folder itself.
    let mut stage = self::stage(true);
    stage.drop_at(row(4, 0.5), (STEP * 0.5, row(2, 0.9)));
    assert_eq!(stage.children(""), ["music/", "readme", "docs/"]);
    assert_eq!(*stage.edits.borrow(), [r#"["readme"] After("music/")"#]);
}

#[test]
fn the_insertion_line_and_the_outlined_node_are_inspectable() {
    let mut stage = stage(true);
    stage.hold(row(4, 0.5), (60.0, row(3, 0.5)));
    assert_eq!(stage.property("drop_into"), Some(InspectValue::Number(3.0)));
    assert_eq!(stage.property("drop_boundary"), None);
    stage.pointer(60.0, row(2, 0.2), PointerKind::Move);
    assert_eq!(
        stage.property("drop_boundary"),
        Some(InspectValue::Number(2.0))
    );
    assert_eq!(stage.property("drop_into"), None);
    stage.pointer(
        60.0,
        row(2, 0.2),
        PointerKind::Release(PointerButton::Primary),
    );
    assert_eq!(stage.property("drop_boundary"), None);
    assert_eq!(stage.children("music/"), ["a", "readme", "b"]);
}

#[test]
fn a_node_cannot_go_into_its_own_subtree() {
    let mut stage = stage(true);
    // The folder onto its own child, and into itself.
    stage.hold(row(0, 0.5), (60.0, row(1, 0.2)));
    assert_eq!(stage.ui.cursor_icon(), CursorIcon::NoDrop);
    assert_eq!(stage.property("drop_boundary"), None);
    stage.pointer(60.0, row(0, 0.5), PointerKind::Move);
    assert_eq!(stage.ui.cursor_icon(), CursorIcon::NoDrop);
    stage.pointer(
        60.0,
        row(0, 0.5),
        PointerKind::Release(PointerButton::Primary),
    );
    assert!(!stage.ui.drag_active());
    assert!(stage.edits.borrow().is_empty(), "nothing reached the model");
    // Beside another folder it is allowed.
    stage.drop_at(row(0, 0.5), (60.0, row(3, 0.5)));
    assert_eq!(stage.children("docs/"), ["music/"]);
}

#[test]
fn alt_with_an_arrow_moves_the_active_node_through_the_outline() {
    let mut stage = stage(true);
    // Select "b" with a click.
    stage.pointer(
        60.0,
        row(2, 0.5),
        PointerKind::Press(PointerButton::Primary),
    );
    stage.pointer(
        60.0,
        row(2, 0.5),
        PointerKind::Release(PointerButton::Primary),
    );
    stage.alt(KeyCode::ArrowUp);
    assert_eq!(stage.children("music/"), ["b", "a"]);
    stage.alt(KeyCode::ArrowUp);
    assert_eq!(stage.children("music/"), ["b", "a"], "already first");
    stage.alt(KeyCode::ArrowLeft);
    assert_eq!(stage.children(""), ["music/", "b", "docs/", "readme"]);
    stage.alt(KeyCode::ArrowDown);
    assert_eq!(stage.children(""), ["music/", "docs/", "b", "readme"]);
    stage.alt(KeyCode::ArrowRight);
    assert_eq!(stage.children("docs/"), ["b"], "into the folder before it");
    assert_eq!(
        *stage.edits.borrow(),
        [
            r#"["b"] Before("a")"#,
            r#"["b"] After("music/")"#,
            r#"["b"] After("docs/")"#,
            r#"["b"] Into("docs/")"#,
        ]
    );
    assert_eq!(
        stage.state.selection().active().get().unwrap().as_deref(),
        Some("b"),
        "the moved node stays active by key"
    );
}

#[test]
fn alt_right_does_not_move_a_node_into_a_leaf() {
    let mut stage = stage(true);
    // "b" follows the leaf "a", which takes no children.
    stage.pointer(
        60.0,
        row(2, 0.5),
        PointerKind::Press(PointerButton::Primary),
    );
    stage.pointer(
        60.0,
        row(2, 0.5),
        PointerKind::Release(PointerButton::Primary),
    );
    stage.alt(KeyCode::ArrowRight);
    assert!(stage.edits.borrow().is_empty());
}

#[test]
fn a_tree_is_not_reorderable_unless_asked() {
    assert!(stage(true).offers_move());
    let mut stage = stage(false);
    assert!(!stage.offers_move());
    stage.pointer(
        60.0,
        row(4, 0.5),
        PointerKind::Press(PointerButton::Primary),
    );
    stage.pointer(60.0, row(1, 0.5), PointerKind::Move);
    assert!(!stage.ui.drag_active());
    stage.pointer(
        60.0,
        row(1, 0.5),
        PointerKind::Release(PointerButton::Primary),
    );
    stage.alt(KeyCode::ArrowUp);
    assert_eq!(stage.children(""), ["music/", "docs/", "readme"]);
    assert_eq!(
        stage.property("reorderable"),
        Some(InspectValue::Bool(false))
    );
}

/// A publication to run once, from inside a drag delivery.
type Armed = Rc<RefCell<Option<Box<dyn FnOnce() -> Result<(), RuntimeError>>>>>;

/// Wraps the tree and, once armed, republishes the hierarchy from its observer
/// delivery: between the target's `Over` and its `Drop`, with no layout between.
struct Publisher {
    child: Option<Element>,
    armed: Armed,
}
impl Component for Publisher {
    fn take_children(&mut self) -> Vec<Element> {
        self.child.take().into_iter().collect()
    }
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
    fn measure(
        &self,
        cx: &MeasureContext<'_>,
        available: Constraints,
    ) -> Result<Size, RuntimeError> {
        cx.measure(0, available)
    }
    fn arrange(&self, cx: &mut ArrangeContext<'_>, size: Size) -> Result<(), RuntimeError> {
        cx.place(0, Rect::from_size(size))
    }
    fn drag_event(
        &mut self,
        _: &mut EventContext,
        event: &DragEvent<'_>,
    ) -> Result<DropResponse, RuntimeError> {
        if event.role() == DragRole::Observer
            && let Some(publish) = self.armed.borrow_mut().take()
        {
            publish()?;
        }
        Ok(DropResponse::Rejected)
    }
}

#[test]
fn a_hierarchy_published_before_the_drop_keeps_the_row_the_user_saw() {
    let mut stage = stage(true);
    let armed: Armed = Rc::new(RefCell::new(None));
    // Mount the same tree inside the publisher, in place of the stage's own.
    stage.ui.unmount(&stage.tree).unwrap();
    let tree = Tree::new(&stage.state, ROW).unwrap().reorderable({
        let (model, edits, state) = (
            stage.model.clone(),
            stage.edits.clone(),
            stage.state.clone(),
        );
        move |_, drop| {
            edits
                .borrow_mut()
                .push(format!("{:?} {:?}", drop.items(), drop.position()));
            apply(&mut model.borrow_mut(), drop);
            state.set_source(TreeSource::new(Outline(model.borrow().clone())))?;
            Ok(true)
        }
    });
    let root = stage
        .ui
        .mount_tree(Element::new(
            "publisher",
            Publisher {
                child: Some(Element::new("tree", tree)),
                armed: armed.clone(),
            },
        ))
        .unwrap();
    stage.ui.flush(Duration::ZERO).unwrap();
    stage.tree = stage.ui.children(&root).unwrap()[0].clone();
    let issues = stage.ui.validation_report().issues;
    assert!(issues.is_empty(), "{issues:?}");
    // Over the upper half of "b", the third row.
    stage.hold(row(4, 0.5), (60.0, row(2, 0.3)));
    // Before the release lands, the application swaps "a" and "b".
    let (model, state) = (stage.model.clone(), stage.state.clone());
    *armed.borrow_mut() = Some(Box::new(move || {
        model
            .borrow_mut()
            .insert("music/".into(), vec!["b".into(), "a".into()]);
        state.set_source(TreeSource::new(Outline(model.borrow().clone())))
    }));
    stage.pointer(
        60.0,
        row(2, 0.2),
        PointerKind::Release(PointerButton::Primary),
    );
    assert!(
        armed.borrow().is_none(),
        "the publication ran before the drop"
    );
    assert_eq!(
        *stage.edits.borrow(),
        [r#"["readme"] Before("b")"#],
        "the drop names the row under the pointer, not the one now at its index"
    );
}

#[test]
fn hiding_the_tree_mid_drag_clears_its_indication() {
    let mut stage = stage(true);
    stage.hold(row(4, 0.5), (60.0, row(3, 0.5)));
    assert_eq!(stage.property("drop_into"), Some(InspectValue::Number(3.0)));
    let tree = stage.tree.clone();
    stage.ui.set_visible(&tree, false).unwrap();
    stage.ui.flush(stage.ui.now()).unwrap();
    stage.pointer(
        60.0,
        row(3, 0.5),
        PointerKind::Release(PointerButton::Primary),
    );
    stage.ui.set_visible(&tree, true).unwrap();
    stage.ui.flush(stage.ui.now()).unwrap();
    assert!(!stage.ui.drag_active());
    assert_eq!(stage.property("drop_into"), None);
    assert_eq!(stage.property("drop_boundary"), None);
    assert!(stage.edits.borrow().is_empty());
    assert_eq!(stage.ui.next_deadline(), None);
}

#[test]
fn alt_arrows_move_among_the_siblings_a_filter_shows() {
    let mut stage = stage(true);
    // "m" shows "music/" and "readme" and hides "docs/" between them.
    stage
        .state
        .filter("m", &FilterMatcher::substring())
        .unwrap();
    let selection = stage.state.selection();
    selection.select(["music/".to_owned()]).unwrap();
    selection.set_active(Some("music/".to_owned())).unwrap();
    stage.ui.flush(stage.ui.now()).unwrap();
    stage.ui.focus(&stage.tree).unwrap();
    stage.alt(KeyCode::ArrowDown);
    assert_eq!(
        *stage.edits.borrow(),
        [r#"["music/"] After("readme")"#],
        "the move passes the shown sibling, not the hidden one"
    );
    assert_eq!(stage.children(""), ["docs/", "readme", "music/"]);
}
