//! Reordering a `VirtualList` by drag and by keyboard, through the facade.
//!
//! The list is the first built-in consumer of the drag session pinned in
//! `drag_and_drop.rs`; these pin what it adds: keyed payloads and positions, the
//! insertion line, a multiple selection dragged together, moving between two
//! lists as one edit, auto-scroll, and the keyboard equivalent.
use pert_ui::*;
use std::{cell::RefCell, rc::Rc, time::Duration};

const ROW: f32 = 40.0;

/// An application's ordered keys, shown as rows of plain labels.
#[derive(Clone)]
struct Rows(Vec<String>);
impl ListItems for Rows {
    fn len(&self) -> usize {
        self.0.len()
    }
    fn key(&self, index: usize) -> Option<String> {
        self.0.get(index).cloned()
    }
    fn build(&self, index: usize) -> Option<Element> {
        let key = self.key(index)?;
        Some(Element::new(key.clone(), Label::new(key)))
    }
    fn index_of(&self, key: &str) -> Option<usize> {
        self.0.iter().position(|candidate| candidate == key)
    }
}

/// The application's model: ordered keys per list, edited as one operation.
#[derive(Default)]
struct Model {
    lists: Vec<Vec<String>>,
    edits: Vec<String>,
    refuse: bool,
}
impl Model {
    /// Removes the items wherever they are and inserts them at the position.
    fn apply(&mut self, target: usize, drop: &ListDrop<'_>) -> bool {
        if self.refuse {
            return false;
        }
        self.edits.push(format!(
            "{:?} {:?} -> list {target} {:?} internal={}",
            drop.operation(),
            drop.items(),
            drop.position(),
            drop.is_internal()
        ));
        let moved: Vec<String> = drop.items().to_vec();
        // Resolved before removal: the position may name a dragged row.
        let list = &self.lists[target];
        let boundary = match drop.position() {
            ListDropPosition::Before(key) => {
                list.iter().position(|k| k == key).unwrap_or(list.len())
            }
            ListDropPosition::After(key) => list
                .iter()
                .position(|k| k == key)
                .map_or(list.len(), |index| index + 1),
            _ => list.len(),
        };
        let moving = drop.operation() == DropOperation::Move;
        let leaving = list[..boundary]
            .iter()
            .filter(|key| moving && moved.contains(key))
            .count();
        let index = boundary - leaving;
        if moving {
            for list in &mut self.lists {
                list.retain(|key| !moved.contains(key));
            }
        }
        self.lists[target].splice(index..index, moved);
        true
    }
}

struct Stage {
    ui: Runtime,
    lists: Vec<WidgetId>,
    model: Rc<RefCell<Model>>,
    selections: Vec<ListSelection>,
    sources: Vec<Signal<ListSource>>,
}

fn keys(names: &[&str]) -> Vec<String> {
    names.iter().map(|name| (*name).to_owned()).collect()
}

/// Lists side by side, each 100 wide, in a window `height` tall.
fn stage(contents: &[&[&str]], height: f32, mode: SelectionMode) -> Stage {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(100.0 * contents.len() as f32, height, 1.0).unwrap())
        .unwrap();
    let model = Rc::new(RefCell::new(Model {
        lists: contents.iter().map(|names| keys(names)).collect(),
        ..Model::default()
    }));
    let sources: Vec<_> = contents
        .iter()
        .map(|names| ui.signal(ListSource::new(Rows(keys(names)))))
        .collect();
    let mut selections = Vec::new();
    let mut elements = Vec::new();
    for (index, source) in sources.iter().enumerate() {
        let selection = ListSelection::new(&ui, mode);
        selections.push(selection.clone());
        let model = model.clone();
        let sources = sources.clone();
        let list = VirtualList::new(source.read_only(), ROW)
            .unwrap()
            .selection(selection)
            .drag_operations(DropOperations::MOVE | DropOperations::COPY)
            .reorderable(move |_, drop| {
                let performed = model.borrow_mut().apply(index, drop);
                if performed {
                    // One edit, then every list shows the model again.
                    for (list, source) in model.borrow().lists.iter().zip(&sources) {
                        source.set(ListSource::new(Rows(list.clone())))?;
                    }
                }
                Ok(performed)
            });
        // A list takes the width it is given, so each gets a fixed column.
        elements.push(Element::new(
            format!("column-{index}"),
            Container::new(Element::new(format!("list-{index}"), list))
                .padding(Length::new(0.0).unwrap())
                .preferred_size(Size::new(100.0, height).unwrap()),
        ));
    }
    let root = ui
        .mount_tree(Element::new(
            "row",
            Row::new(elements).gap(Length::new(0.0).unwrap()),
        ))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let lists = ui
        .children(&root)
        .unwrap()
        .iter()
        .map(|column| ui.children(column).unwrap()[0].clone())
        .collect();
    Stage {
        ui,
        lists,
        model,
        selections,
        sources,
    }
}

#[test]
fn interactive_list_row_highlights_only_while_hovered() {
    let mut stage = stage(&[&["one", "two"]], 120.0, SelectionMode::Single);
    let color = stage
        .ui
        .theme()
        .palette(ThemeMode::Dark)
        .unwrap()
        .color(ColorRole::SurfaceHover);
    let count = |ui: &mut Runtime| {
        ui.paint()
            .unwrap()
            .commands()
            .iter()
            .filter(|command| {
                matches!(command,
                    DrawCommand::FillRect { color: fill, .. }
                    | DrawCommand::DrawMask { color: fill, .. }
                    if *fill == color)
            })
            .count()
    };
    let resting = count(&mut stage.ui);
    stage.pointer(10.0, 20.0, PointerKind::Move);
    assert!(count(&mut stage.ui) > resting);
    stage.pointer(10.0, 110.0, PointerKind::Move);
    assert_eq!(count(&mut stage.ui), resting);
}

impl Stage {
    fn pointer(&mut self, x: f32, y: f32, kind: PointerKind) {
        self.ui
            .pointer_input(PointerEvent::new(Point::new(x, y).unwrap(), kind))
            .unwrap();
        self.ui.flush(self.ui.now()).unwrap();
    }
    fn press(&mut self, x: f32, y: f32) {
        self.pointer(x, y, PointerKind::Press(PointerButton::Primary));
    }
    fn release(&mut self, x: f32, y: f32) {
        self.pointer(x, y, PointerKind::Release(PointerButton::Primary));
    }
    fn drag(&mut self, from: (f32, f32), to: (f32, f32)) {
        self.press(from.0, from.1);
        self.pointer(from.0 + 6.0, from.1, PointerKind::Move);
        assert!(
            self.ui.drag_active(),
            "crossing the threshold starts a drag"
        );
        self.pointer(to.0, to.1, PointerKind::Move);
        self.release(to.0, to.1);
        assert!(!self.ui.drag_active());
    }
    fn order(&self, list: usize) -> Vec<String> {
        self.model.borrow().lists[list].clone()
    }
    fn property(&self, list: usize, name: &str) -> Option<InspectValue> {
        let id = &self.lists[list];
        let snapshot = self.ui.inspect();
        let key = self.ui.key(id).unwrap();
        snapshot
            .components
            .iter()
            .find(|component| component.key == key)
            .and_then(|component| component.details.properties.get(name).cloned())
    }
}

#[test]
fn dragging_a_row_reorders_through_one_keyed_edit() {
    let mut stage = stage(&[&["a", "b", "c", "d"]], 300.0, SelectionMode::Single);
    // From the middle of "a" to the lower half of "c".
    stage.drag((20.0, 20.0), (20.0, 2.0 * ROW + 30.0));
    assert_eq!(stage.order(0), ["b", "c", "a", "d"]);
    assert_eq!(
        stage.model.borrow().edits,
        ["Move [\"a\"] -> list 0 Before(\"d\") internal=true"]
    );
    // The list shows the new order, with the moved row's identity rebuilt by key.
    let shown: Vec<_> = stage
        .ui
        .children(&stage.lists[0])
        .unwrap()
        .iter()
        .map(|row| {
            (
                stage.ui.key(row).unwrap().to_owned(),
                stage.ui.bounds(row).unwrap().unwrap().y(),
            )
        })
        .collect();
    assert!(shown.contains(&("a".to_owned(), 2.0 * ROW)));
    assert_eq!(stage.ui.next_deadline(), None);
}

#[test]
fn dropping_a_row_beside_itself_changes_nothing() {
    let mut stage = stage(&[&["a", "b", "c"]], 300.0, SelectionMode::Single);
    stage.drag((20.0, ROW + 20.0), (20.0, ROW + 5.0));
    assert_eq!(stage.order(0), ["a", "b", "c"], "before itself");
    stage.drag((20.0, ROW + 20.0), (20.0, ROW + 35.0));
    assert_eq!(stage.order(0), ["a", "b", "c"], "after itself");
}

#[test]
fn an_armed_press_follows_its_key_when_the_source_is_replaced() {
    let mut stage = stage(&[&["a", "b", "c"]], 300.0, SelectionMode::Single);
    stage.press(20.0, 20.0);
    // The application reorders before the press becomes a drag: "a" is now
    // where "b" was, and another row is under the pressed position.
    stage.model.borrow_mut().lists[0] = keys(&["b", "a", "c"]);
    stage.sources[0]
        .set(ListSource::new(Rows(keys(&["b", "a", "c"]))))
        .unwrap();
    stage.ui.flush(stage.ui.now()).unwrap();
    stage.pointer(26.0, 20.0, PointerKind::Move);
    assert!(stage.ui.drag_active());
    stage.pointer(20.0, 2.0 * ROW + 30.0, PointerKind::Move);
    stage.release(20.0, 2.0 * ROW + 30.0);
    assert_eq!(
        stage.model.borrow().edits,
        ["Move [\"a\"] -> list 0 After(\"c\") internal=true"],
        "the pressed row was dragged, not the row that took its place"
    );
}

#[test]
fn the_upper_half_of_a_row_drops_before_it_and_past_the_end_drops_last() {
    let mut stage = stage(&[&["a", "b", "c"]], 300.0, SelectionMode::Single);
    stage.drag((20.0, 2.0 * ROW + 20.0), (20.0, 5.0));
    assert_eq!(stage.order(0), ["c", "a", "b"]);
    stage.drag((20.0, 20.0), (20.0, 250.0));
    assert_eq!(stage.order(0), ["a", "b", "c"]);
    assert_eq!(
        stage.model.borrow().edits[1],
        "Move [\"c\"] -> list 0 After(\"b\") internal=true",
        "below the last row the position names the last key"
    );
}

#[test]
fn an_insertion_line_marks_the_boundary_while_the_drag_is_over_the_list() {
    let mut stage = stage(&[&["a", "b", "c"]], 300.0, SelectionMode::Single);
    stage.press(20.0, 20.0);
    stage.pointer(26.0, 20.0, PointerKind::Move);
    stage.pointer(20.0, ROW + 30.0, PointerKind::Move);
    assert_eq!(
        stage.property(0, "drop_boundary"),
        Some(InspectValue::Number(2.0)),
        "the lower half of the second row is the boundary before the third"
    );
    let theme = stage.ui.theme();
    let primary = theme
        .palette(theme.mode().unwrap())
        .unwrap()
        .color(ColorRole::Primary);
    let scene = stage.ui.paint().unwrap();
    // The active row's focus ring has the same built-in color; the line is the fill centered on
    // the boundary between the second and third rows.
    let lines = scene
        .commands()
        .iter()
        .filter(|command| {
            matches!(command, DrawCommand::FillRect { rect, color }
                if *color == primary
                    && rect.width() == 100.0
                    && rect.y() + rect.height() / 2.0 == 2.0 * ROW)
        })
        .count();
    assert_eq!(
        lines, 1,
        "an insertion line across the list at the boundary"
    );
    // Leaving the list removes it; so does the end of the session.
    stage
        .ui
        .key_input(KeyCode::Escape, KeyAction::Press)
        .unwrap();
    stage.ui.flush(stage.ui.now()).unwrap();
    assert_eq!(stage.property(0, "drop_boundary"), None);
    assert_eq!(
        stage.order(0),
        ["a", "b", "c"],
        "a cancelled drag edits nothing"
    );
}

#[test]
fn a_multiple_selection_is_dragged_together_and_a_plain_click_narrows_it() {
    let mut stage = stage(
        &[&["a", "b", "c", "d", "e"]],
        300.0,
        SelectionMode::Multiple,
    );
    let selected = |stage: &Stage| {
        let mut keys: Vec<_> = stage.selections[0]
            .selected()
            .get()
            .unwrap()
            .iter()
            .cloned()
            .collect();
        keys.sort();
        keys
    };
    stage.press(20.0, 20.0);
    stage.release(20.0, 20.0);
    stage
        .ui
        .key_input(KeyCode::ShiftLeft, KeyAction::Press)
        .unwrap();
    stage.press(20.0, ROW + 20.0);
    stage.release(20.0, ROW + 20.0);
    stage
        .ui
        .key_input(KeyCode::ShiftLeft, KeyAction::Release)
        .unwrap();
    assert_eq!(selected(&stage), ["a", "b"]);

    // Pressing a selected row keeps the whole selection for the drag.
    stage.drag((20.0, 20.0), (20.0, 3.0 * ROW + 30.0));
    assert_eq!(stage.order(0), ["c", "d", "a", "b", "e"]);
    assert_eq!(
        stage.model.borrow().edits,
        ["Move [\"a\", \"b\"] -> list 0 Before(\"e\") internal=true"]
    );

    // Without a drag, the same press narrows the selection on release.
    assert_eq!(selected(&stage), ["a", "b"]);
    stage.press(20.0, 2.0 * ROW + 20.0);
    assert_eq!(
        selected(&stage),
        ["a", "b"],
        "not yet: it might become a drag"
    );
    stage.release(20.0, 2.0 * ROW + 20.0);
    assert_eq!(selected(&stage), ["a"]);
}

#[test]
fn moving_between_two_lists_is_one_edit_on_the_target_side() {
    let mut stage = stage(&[&["a", "b"], &["x", "y"]], 300.0, SelectionMode::Single);
    stage.drag((20.0, 20.0), (120.0, ROW + 30.0));
    assert_eq!(stage.order(0), ["b"]);
    assert_eq!(stage.order(1), ["x", "y", "a"]);
    assert_eq!(
        stage.model.borrow().edits,
        ["Move [\"a\"] -> list 1 After(\"y\") internal=false"],
        "the source list was not asked to delete anything"
    );
}

#[test]
fn the_copy_modifier_asks_for_a_copy() {
    let mut stage = stage(&[&["a", "b"], &["x"]], 300.0, SelectionMode::Single);
    let copy = if cfg!(target_os = "macos") {
        KeyCode::AltLeft
    } else {
        KeyCode::ControlLeft
    };
    stage.press(20.0, ROW + 20.0);
    stage.pointer(26.0, ROW + 20.0, PointerKind::Move);
    stage.ui.key_input(copy, KeyAction::Press).unwrap();
    stage.pointer(120.0, 5.0, PointerKind::Move);
    assert_eq!(stage.ui.cursor_icon(), CursorIcon::Copy);
    stage.release(120.0, 5.0);
    assert_eq!(stage.order(0), ["a", "b"]);
    assert_eq!(stage.order(1), ["b", "x"]);
}

#[test]
fn a_refused_drop_leaves_the_model_alone() {
    let mut stage = stage(&[&["a", "b", "c"]], 300.0, SelectionMode::Single);
    stage.model.borrow_mut().refuse = true;
    stage.ui.set_debug_mode(DebugMode::Inspect);
    stage.drag((20.0, 20.0), (20.0, 2.0 * ROW + 30.0));
    assert_eq!(stage.order(0), ["a", "b", "c"]);
    let ended = stage
        .ui
        .inspect()
        .events
        .into_iter()
        .find(|event| event.kind == EventKind::DragEnd)
        .unwrap();
    assert!(
        matches!(ended.status, EventStatus::Failed(_)),
        "refused, not cancelled"
    );
}

#[test]
fn alt_with_an_arrow_moves_the_active_item_without_a_pointer() {
    let mut stage = stage(&[&["a", "b", "c"]], 300.0, SelectionMode::Single);
    stage.press(20.0, 20.0);
    stage.release(20.0, 20.0);
    let alt = |stage: &mut Stage, key: KeyCode| {
        stage
            .ui
            .key_input(KeyCode::AltLeft, KeyAction::Press)
            .unwrap();
        stage.ui.key_input(key, KeyAction::Press).unwrap();
        stage.ui.key_input(key, KeyAction::Release).unwrap();
        stage
            .ui
            .key_input(KeyCode::AltLeft, KeyAction::Release)
            .unwrap();
        stage.ui.flush(stage.ui.now()).unwrap();
    };
    alt(&mut stage, KeyCode::ArrowDown);
    assert_eq!(stage.order(0), ["b", "a", "c"]);
    alt(&mut stage, KeyCode::ArrowDown);
    assert_eq!(stage.order(0), ["b", "c", "a"]);
    alt(&mut stage, KeyCode::ArrowDown);
    assert_eq!(stage.order(0), ["b", "c", "a"], "already last");
    alt(&mut stage, KeyCode::ArrowUp);
    assert_eq!(stage.order(0), ["b", "a", "c"]);
    assert_eq!(
        stage.selections[0].active().get().unwrap().as_deref(),
        Some("a"),
        "the moved item stays active by key"
    );
}

#[test]
fn a_drag_resting_near_an_edge_scrolls_an_overflowing_list_and_then_stops() {
    let names: Vec<String> = (0..30).map(|index| format!("row-{index}")).collect();
    let names: Vec<&str> = names.iter().map(String::as_str).collect();
    let mut stage = stage(&[&names], 200.0, SelectionMode::Single);
    let scroll = |stage: &Stage| match stage.property(0, "scroll_y") {
        Some(InspectValue::Number(value)) => value,
        other => panic!("scroll_y: {other:?}"),
    };
    stage.press(20.0, 20.0);
    stage.pointer(26.0, 20.0, PointerKind::Move);
    stage.pointer(20.0, 100.0, PointerKind::Move);
    assert_eq!(stage.ui.next_deadline(), None, "nothing scrolls mid-list");
    stage.pointer(20.0, 195.0, PointerKind::Move);
    let first = stage.ui.next_deadline().expect("an auto-scroll timer");
    stage.ui.flush(first).unwrap();
    let after_one = scroll(&stage);
    assert!(after_one > 0.0);
    // The pointer has not moved; the next tick scrolls further and the
    // insertion line follows the rows under it.
    let second = stage
        .ui
        .next_deadline()
        .expect("it repeats while it rests there");
    stage.ui.flush(second).unwrap();
    assert!(scroll(&stage) > after_one);
    stage.release(20.0, 195.0);
    assert_eq!(
        stage.ui.next_deadline(),
        None,
        "the timer ended with the session"
    );
    assert_eq!(
        stage.order(0)[0],
        "row-1",
        "and the row was moved down the list"
    );
}

#[test]
fn a_list_that_is_not_reorderable_neither_starts_nor_accepts_a_drag() {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(100.0, 300.0, 1.0).unwrap())
        .unwrap();
    let source = ui.signal(ListSource::new(Rows(keys(&["a", "b"]))));
    let list = ui
        .mount_tree(Element::new(
            "list",
            VirtualList::new(source.read_only(), ROW)
                .unwrap()
                .selection(ListSelection::new(&ui, SelectionMode::Single)),
        ))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    for (y, kind) in [
        (20.0, PointerKind::Press(PointerButton::Primary)),
        (60.0, PointerKind::Move),
    ] {
        ui.pointer_input(PointerEvent::new(Point::new(20.0, y).unwrap(), kind))
            .unwrap();
    }
    assert!(!ui.drag_active());
    let _ = list;
}

#[test]
fn a_reorderable_list_offers_the_move_action_and_a_plain_one_does_not() {
    let actions = |ui: &Runtime, list: &WidgetId| -> Vec<SemanticAction> {
        let key = ui.key(list).unwrap().to_owned();
        ui.inspect()
            .components
            .iter()
            .find(|component| component.key == key)
            .and_then(|component| component.details.semantics.clone())
            .map(|semantics| semantics.actions)
            .unwrap_or_default()
    };
    let stage = stage(&[&["a", "b"]], 300.0, SelectionMode::Single);
    assert!(actions(&stage.ui, &stage.lists[0]).contains(&SemanticAction::Move));

    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(100.0, 300.0, 1.0).unwrap())
        .unwrap();
    let plain = VirtualList::new(
        ui.signal(ListSource::new(Rows(keys(&["a", "b"]))))
            .read_only(),
        ROW,
    )
    .unwrap()
    .selection(ListSelection::new(&ui, SelectionMode::Single));
    let list = ui.mount_tree(Element::new("plain", plain)).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert!(!actions(&ui, &list).contains(&SemanticAction::Move));
}

/// Rows built under an empty key, which only the list's own keying makes valid.
struct UnkeyedRows(Vec<String>);
impl ListItems for UnkeyedRows {
    fn len(&self) -> usize {
        self.0.len()
    }
    fn key(&self, index: usize) -> Option<String> {
        self.0.get(index).cloned()
    }
    fn build(&self, index: usize) -> Option<Element> {
        Some(Label::new(self.0.get(index)?.as_str()).key(""))
    }
}

#[test]
fn a_dragged_row_preview_is_keyed_like_the_mounted_row() {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(100.0, 200.0, 1.0).unwrap())
        .unwrap();
    let source = ui.signal(ListSource::new(UnkeyedRows(keys(&["a", "b"]))));
    let list = VirtualList::new(&source, ROW)
        .unwrap()
        .reorderable(|_, _| Ok(false));
    ui.mount_tree(list.key("list")).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let at = |x: f32, y: f32, kind| PointerEvent::new(Point::new(x, y).unwrap(), kind);
    ui.pointer_input(at(20.0, 20.0, PointerKind::Press(PointerButton::Primary)))
        .unwrap();
    ui.pointer_input(at(26.0, 20.0, PointerKind::Move)).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert!(ui.drag_active(), "the preview mounts and the drag starts");
    ui.pointer_input(at(26.0, 20.0, PointerKind::Release(PointerButton::Primary)))
        .unwrap();
}

/// Rows of single-line text fields, one bound value per key.
#[derive(Clone)]
struct Fields(Vec<(String, Signal<String>)>);
impl ListItems for Fields {
    fn len(&self) -> usize {
        self.0.len()
    }
    fn key(&self, index: usize) -> Option<String> {
        self.0.get(index).map(|(key, _)| key.clone())
    }
    fn build(&self, index: usize) -> Option<Element> {
        let (key, value) = self.0.get(index)?;
        Some(Element::new(key.clone(), TextField::new(value.clone())))
    }
    fn index_of(&self, key: &str) -> Option<usize> {
        self.0.iter().position(|(candidate, _)| candidate == key)
    }
}

#[test]
fn alt_with_an_arrow_in_a_row_field_moves_the_row_outside_macos() {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(100.0, 300.0, 1.0).unwrap())
        .unwrap();
    let rows: Vec<_> = ["a", "b", "c"]
        .into_iter()
        .map(|key| (key.to_owned(), ui.signal(key.to_owned())))
        .collect();
    let source = ui.signal(ListSource::new(Fields(rows.clone())));
    let selection = ListSelection::new(&ui, SelectionMode::Single);
    let drops = Rc::new(RefCell::new(Vec::new()));
    let log = drops.clone();
    let list = VirtualList::new(source.read_only(), ROW)
        .unwrap()
        .selection(selection.clone())
        .reorderable(move |_, drop| {
            log.borrow_mut()
                .push(format!("{:?} {:?}", drop.items(), drop.position()));
            Ok(true)
        });
    let root = ui.mount_tree(Element::new("list", list)).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let field = ui.find_descendant(&root, "a").unwrap().unwrap();
    ui.focus(&field).unwrap();
    selection.set_active(Some("a".into())).unwrap();
    ui.flush(ui.now()).unwrap();
    ui.key_input(KeyCode::AltLeft, KeyAction::Press).unwrap();
    ui.key_input(KeyCode::ArrowDown, KeyAction::Press).unwrap();
    ui.key_input(KeyCode::ArrowDown, KeyAction::Release)
        .unwrap();
    ui.key_input(KeyCode::AltLeft, KeyAction::Release).unwrap();
    ui.flush(ui.now()).unwrap();
    // On macOS, Option with Down moves the field's caret instead.
    let expected: &[&str] = if cfg!(target_os = "macos") {
        &[]
    } else {
        &[r#"["a"] After("b")"#]
    };
    assert_eq!(*drops.borrow(), expected, "the list moves the active row");
    assert_eq!(ui.focused(), Some(field));
    assert_eq!(rows[0].1.get().unwrap(), "a", "the field edited nothing");
}
