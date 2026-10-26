//! `InlineText` as a list-row label (`InlineText::in_row`): a single press
//! reaches the row, a double click edits, what breaks a click series,
//! reordering by the label, leaving an edit, Tab and redescription.
use pert_ui::*;
use std::{cell::RefCell, rc::Rc, time::Duration};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const KEYS: [&str; 3] = ["a", "b", "c"];

/// Rows whose only content is a label bound to the row's own signal, so a
/// rebuilt row keeps its label's state.
struct Labels {
    names: Vec<Signal<String>>,
    in_row: bool,
}
impl ListItems for Labels {
    fn len(&self) -> usize {
        self.names.len()
    }
    fn key(&self, index: usize) -> Option<String> {
        KEYS.get(index).map(|key| (*key).to_owned())
    }
    fn build(&self, index: usize) -> Option<Element> {
        let label = InlineText::new(self.names.get(index)?).in_row(self.in_row);
        Some(Element::new("title", label))
    }
}

struct Stage {
    ui: Runtime,
    names: Vec<Signal<String>>,
    rows: Signal<ListSource>,
    selection: ListSelection,
    list: WidgetId,
    after: WidgetId,
    drops: Rc<RefCell<Vec<Vec<String>>>>,
}

/// A list of three labelled rows above a button. A reorderable list records
/// each drop and refuses it, so the rows stay where they are.
fn stage(
    in_row: bool,
    mode: SelectionMode,
    reorderable: bool,
) -> Result<Stage, Box<dyn std::error::Error>> {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(300.0, 200.0, 1.0)?)?;
    let names: Vec<_> = KEYS
        .iter()
        .map(|key| ui.signal(format!("Title {key}")))
        .collect();
    let rows = ui.signal(ListSource::new(Labels {
        names: names.clone(),
        in_row,
    }));
    let selection = ListSelection::new(&ui, mode);
    let drops: Rc<RefCell<Vec<Vec<String>>>> = Rc::default();
    let log = drops.clone();
    let mut list = VirtualList::new(rows.read_only(), 32.0)?.selection(selection.clone());
    if reorderable {
        list = list.reorderable(move |_, drop| {
            log.borrow_mut().push(drop.items().to_vec());
            Ok(false)
        });
    }
    let root = ui.mount_tree(Element::new(
        "root",
        Column::new([
            Element::new("list", list).with_grow(1),
            Element::new("after", Button::new("After", |_| Ok(()))),
        ]),
    ))?;
    ui.flush(Duration::ZERO)?;
    let list = ui.find(&root, &["list"])?.ok_or("the list")?;
    let after = ui.find(&root, &["after"])?.ok_or("the button")?;
    Ok(Stage {
        ui,
        names,
        rows,
        selection,
        list,
        after,
        drops,
    })
}

impl Stage {
    fn label(&self, key: &str) -> WidgetId {
        self.ui
            .find(&self.list, &[key])
            .expect("the list is mounted")
            .expect("the row is mounted")
    }
    /// A point inside the label of row `key`, `dx` right of its leading
    /// edge.
    fn at(&self, key: &str, dx: f32) -> Point {
        let [x, y, _, h] = self
            .ui
            .bounds(&self.label(key))
            .expect("mounted")
            .expect("laid out")
            .values();
        Point::new(x + 10.0 + dx, y + h / 2.0).expect("finite")
    }
    fn pointer(&mut self, at: Point, kind: PointerKind) {
        self.ui
            .pointer_input(PointerEvent::new(at, kind))
            .expect("pointer input");
        self.ui.flush(self.ui.now()).expect("flush");
    }
    /// A press and its release on row `key`'s label, at the current time.
    fn click(&mut self, key: &str) {
        let at = self.at(key, 0.0);
        self.pointer(at, PointerKind::Press(PointerButton::Primary));
        self.pointer(at, PointerKind::Release(PointerButton::Primary));
    }
    fn wait(&mut self, by: Duration) {
        self.ui.flush(self.ui.now() + by).expect("flush");
    }
    fn modifier(&mut self, code: KeyCode, action: KeyAction) {
        self.ui.key_input(code, action).expect("key input");
    }
    fn property(&self, key: &str, name: &str) -> Option<InspectValue> {
        self.ui
            .inspect()
            .components
            .into_iter()
            .find(|c| c.key == key)
            .and_then(|c| c.details.properties.get(name).cloned())
    }
    fn editing(&self, key: &str) -> bool {
        self.property(key, "editing") == Some(InspectValue::Bool(true))
    }
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
}

#[test]
fn a_single_press_on_a_label_selects_its_row_and_leaves_focus_on_the_list() -> TestResult {
    let mut stage = stage(true, SelectionMode::Single, false)?;
    stage.click("b");
    assert_eq!(stage.active().as_deref(), Some("b"));
    assert!(!stage.editing("b"));
    assert_eq!(stage.ui.focused(), Some(stage.list.clone()));
    assert_eq!(
        stage.property("b", "in_row"),
        Some(InspectValue::Bool(true))
    );
    Ok(())
}

#[test]
fn a_double_click_on_an_unselected_row_selects_it_and_edits() -> TestResult {
    let mut stage = stage(true, SelectionMode::Single, false)?;
    stage.click("c");
    stage.click("c");
    assert_eq!(stage.active().as_deref(), Some("c"));
    assert!(stage.editing("c"));
    stage.ui.text_input("Renamed")?;
    stage.ui.key_input(KeyCode::Enter, KeyAction::Press)?;
    assert_eq!(stage.names[2].get()?, "Renamed");
    Ok(())
}

#[test]
fn presses_farther_apart_than_the_interval_do_not_edit() -> TestResult {
    let mut stage = stage(true, SelectionMode::Single, false)?;
    stage.click("a");
    stage.wait(ClickCounter::DEFAULT_INTERVAL + Duration::from_millis(1));
    stage.click("a");
    assert!(!stage.editing("a"));
    Ok(())
}

#[test]
fn a_press_on_another_label_breaks_the_series() -> TestResult {
    let mut stage = stage(true, SelectionMode::Single, false)?;
    stage.click("a");
    stage.click("b");
    stage.click("a");
    assert!(!stage.editing("a"), "A, B, A are three single clicks");
    assert!(!stage.editing("b"));
    assert_eq!(stage.active().as_deref(), Some("a"));
    Ok(())
}

#[test]
fn a_drag_onto_another_row_breaks_the_series() -> TestResult {
    let mut stage = stage(true, SelectionMode::Single, true)?;
    let start = stage.at("a", 0.0);
    stage.pointer(start, PointerKind::Press(PointerButton::Primary));
    stage.pointer(stage.at("a", 8.0), PointerKind::Move);
    assert!(stage.ui.drag_active());
    let target = stage.at("c", 0.0);
    stage.pointer(target, PointerKind::Move);
    stage.pointer(target, PointerKind::Release(PointerButton::Primary));
    assert_eq!(*stage.drops.borrow(), [["a".to_owned()]]);
    stage.click("a");
    assert!(!stage.editing("a"));
    Ok(())
}

#[test]
fn a_drag_that_returns_to_its_start_breaks_the_series() -> TestResult {
    let mut stage = stage(true, SelectionMode::Single, true)?;
    let start = stage.at("a", 0.0);
    stage.pointer(start, PointerKind::Press(PointerButton::Primary));
    stage.pointer(stage.at("a", 12.0), PointerKind::Move);
    assert!(stage.ui.drag_active(), "the drag stays on the label");
    stage.pointer(start, PointerKind::Move);
    stage.pointer(start, PointerKind::Release(PointerButton::Primary));
    assert!(!stage.ui.drag_active());
    stage.click("a");
    assert!(!stage.editing("a"));
    Ok(())
}

#[test]
fn control_presses_toggle_the_selection_and_never_edit() -> TestResult {
    let mut stage = stage(true, SelectionMode::Multiple, false)?;
    stage.modifier(KeyCode::ControlLeft, KeyAction::Press);
    stage.click("b");
    assert_eq!(stage.selected(), ["b"]);
    stage.click("b");
    assert!(stage.selected().is_empty());
    stage.modifier(KeyCode::ControlLeft, KeyAction::Release);
    assert!(!stage.editing("b"));
    Ok(())
}

#[test]
fn a_modified_press_between_two_plain_ones_resets_the_series() -> TestResult {
    let mut stage = stage(true, SelectionMode::Multiple, false)?;
    stage.click("b");
    stage.modifier(KeyCode::ShiftLeft, KeyAction::Press);
    stage.click("b");
    stage.modifier(KeyCode::ShiftLeft, KeyAction::Release);
    stage.click("b");
    assert!(!stage.editing("b"));
    Ok(())
}

#[test]
fn a_drag_from_a_selected_rows_label_reorders_and_does_not_edit() -> TestResult {
    let mut stage = stage(true, SelectionMode::Single, true)?;
    stage.click("a");
    assert_eq!(stage.active().as_deref(), Some("a"));
    // Later than a double click: a single press on the selected row.
    stage.wait(ClickCounter::DEFAULT_INTERVAL + Duration::from_millis(1));
    let start = stage.at("a", 0.0);
    stage.pointer(start, PointerKind::Press(PointerButton::Primary));
    stage.pointer(stage.at("a", 8.0), PointerKind::Move);
    assert!(stage.ui.drag_active());
    let target = stage.at("c", 0.0);
    stage.pointer(target, PointerKind::Move);
    stage.pointer(target, PointerKind::Release(PointerButton::Primary));
    assert_eq!(*stage.drops.borrow(), [["a".to_owned()]]);
    assert!(!stage.editing("a"));
    Ok(())
}

#[test]
fn enter_ends_an_edit_with_focus_on_the_list_and_arrows_navigate() -> TestResult {
    let mut stage = stage(true, SelectionMode::Single, false)?;
    stage.click("a");
    stage.click("a");
    assert!(stage.editing("a"));
    stage.ui.text_input("First")?;
    stage.ui.key_input(KeyCode::Enter, KeyAction::Press)?;
    stage.ui.key_input(KeyCode::Enter, KeyAction::Release)?;
    stage.ui.flush(stage.ui.now())?;
    assert_eq!(stage.names[0].get()?, "First");
    assert!(!stage.editing("a"));
    assert_eq!(stage.ui.focused(), Some(stage.list.clone()));
    press(&mut stage.ui, KeyCode::ArrowDown)?;
    assert_eq!(stage.active().as_deref(), Some("b"));
    Ok(())
}

fn press(ui: &mut Runtime, code: KeyCode) -> Result<(), RuntimeError> {
    ui.key_input(code, KeyAction::Press)?;
    ui.key_input(code, KeyAction::Release)?;
    ui.flush(ui.now())?;
    Ok(())
}

#[test]
fn tab_passes_over_the_resting_labels() -> TestResult {
    let mut stage = stage(true, SelectionMode::Single, false)?;
    stage.ui.focus(&stage.list)?;
    stage.ui.focus_next(FocusDirection::Forward)?;
    assert_eq!(stage.ui.focused(), Some(stage.after.clone()));
    Ok(())
}

#[test]
fn turning_row_mode_off_and_on_keeps_an_open_draft() -> TestResult {
    let mut stage = stage(true, SelectionMode::Single, false)?;
    stage.click("a");
    stage.click("a");
    stage.ui.text_input("Draft")?;
    for in_row in [false, true] {
        stage.rows.set(ListSource::new(Labels {
            names: stage.names.clone(),
            in_row,
        }))?;
        stage.ui.flush(stage.ui.now())?;
        assert!(stage.editing("a"));
        assert_eq!(
            stage.property("a", "draft"),
            Some(InspectValue::Text("Draft".into()))
        );
        assert_eq!(
            stage.property("a", "in_row").is_some(),
            in_row,
            "inspection follows the mode"
        );
    }
    stage.ui.key_input(KeyCode::Enter, KeyAction::Press)?;
    assert_eq!(stage.names[0].get()?, "Draft");
    Ok(())
}

#[test]
fn without_row_mode_a_press_focuses_the_label_and_the_row_stays_unselected() -> TestResult {
    // A compatibility check: the default press behavior is unchanged.
    let mut stage = stage(false, SelectionMode::Single, false)?;
    stage.click("b");
    assert_eq!(stage.ui.focused(), Some(stage.label("b")));
    assert_eq!(stage.active(), None);
    assert!(!stage.editing("b"));
    assert_eq!(stage.property("b", "in_row"), None);
    Ok(())
}

#[test]
fn turning_row_mode_on_moves_focus_from_a_resting_label_to_the_list() -> TestResult {
    let mut stage = stage(false, SelectionMode::Single, false)?;
    let label = stage.label("a");
    stage.ui.focus(&label)?;
    stage.rows.set(ListSource::new(Labels {
        names: stage.names.clone(),
        in_row: true,
    }))?;
    stage.ui.flush(stage.ui.now())?;
    assert_eq!(stage.ui.focused(), Some(stage.list.clone()));
    Ok(())
}
