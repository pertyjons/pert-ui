//! Moving table columns by drag and by keyboard, through the facade.
//!
//! A column move is the table's own arrangement, like a dragged width: these
//! pin that only movable columns move and only among their movable
//! neighbours, that cells follow their column, that a move and a sort share a
//! title without stealing each other's gesture, and that the order survives a
//! workspace round trip.
use pert_ui::*;
use std::{cell::RefCell, rc::Rc, time::Duration};

type TestResult = Result<(), Box<dyn std::error::Error>>;
const ROW: f32 = 24.0;
const WIDTH: f32 = 100.0;

/// Rows keyed `r0`, `r1`, …, each cell showing its column key.
struct Cells(usize);
impl TableItems for Cells {
    fn len(&self) -> usize {
        self.0
    }
    fn key(&self, index: usize) -> Option<String> {
        (index < self.0).then(|| format!("r{index}"))
    }
    fn cell(&self, index: usize, column: &str) -> Option<Element> {
        (index < self.0).then(|| Element::new("text", Label::new(column)))
    }
}

struct Fixture {
    ui: Runtime,
    state: TableController,
    table: WidgetId,
    sorts: Rc<RefCell<usize>>,
}

/// `name` and `d` stay put; `a`, `b`, `c` and any `extra` are movable. Every
/// column is 100 wide, `name` is sortable as well as fixed and `a` is sortable
/// as well as movable.
fn fixture(width: f32, extra: &[&str]) -> Result<Fixture, Box<dyn std::error::Error>> {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(width, 200.0, 1.0)?)?;
    let column =
        |key: &str| TableColumn::new(key, key.to_uppercase()).width(Length::new(WIDTH).unwrap());
    let mut columns = vec![
        column("name").sortable(true),
        column("a").movable(true).sortable(true),
        column("b").movable(true),
        column("c").movable(true),
    ];
    columns.extend(extra.iter().map(|key| column(key).movable(true)));
    columns.push(column("d"));
    let state = TableController::new(
        &ui,
        columns,
        TableSource::new(Cells(20)),
        ListSelection::new(&ui, SelectionMode::Single),
    )?;
    let sorts = Rc::new(RefCell::new(0));
    let counted = sorts.clone();
    let table = ui.mount_tree(Element::new(
        "table",
        Table::new(&state, ROW)?.on_sort(move |_, _| {
            *counted.borrow_mut() += 1;
            Ok(())
        }),
    ))?;
    ui.flush(Duration::ZERO)?;
    Ok(Fixture {
        ui,
        state,
        table,
        sorts,
    })
}

impl Fixture {
    fn pointer(&mut self, x: f32, y: f32, kind: PointerKind) {
        self.ui
            .pointer_input(PointerEvent::new(Point::new(x, y).unwrap(), kind))
            .unwrap();
        self.ui.flush(self.ui.now()).unwrap();
    }
    /// Presses the title at `from` and moves the drag to `to` without releasing.
    fn hold(&mut self, from: f32, to: f32) {
        self.pointer(from, 10.0, PointerKind::Press(PointerButton::Primary));
        self.pointer(from + 6.0, 10.0, PointerKind::Move);
        assert!(self.ui.drag_active(), "a movable title starts a drag");
        self.pointer(to, 60.0, PointerKind::Move);
    }
    fn drag(&mut self, from: f32, to: f32) {
        self.hold(from, to);
        self.pointer(to, 60.0, PointerKind::Release(PointerButton::Primary));
        assert!(!self.ui.drag_active());
    }
    fn order(&self) -> Vec<String> {
        self.state
            .columns()
            .iter()
            .map(|column| column.key().to_owned())
            .collect()
    }
    /// The first mounted row's cells as `(column key, x)`, in child order.
    fn cells(&self) -> Vec<(String, f32)> {
        let body = self.ui.children(&self.table).unwrap()[0].clone();
        let row = self.ui.children(&body).unwrap()[0].clone();
        self.ui
            .children(&row)
            .unwrap()
            .iter()
            .map(|cell| {
                let x = self.ui.bounds(cell).unwrap().unwrap().x();
                (self.ui.key(cell).unwrap().to_owned(), x)
            })
            .collect()
    }
    fn property(&self, name: &str) -> Option<InspectValue> {
        let key = self.ui.key(&self.table).unwrap();
        self.ui
            .inspect()
            .components
            .iter()
            .find(|component| component.key == key)
            .and_then(|component| component.details.properties.get(name).cloned())
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

fn keys(names: &[&str]) -> Vec<String> {
    names.iter().map(|name| (*name).to_owned()).collect()
}

#[test]
fn dragging_a_title_moves_the_column_and_its_cells_follow() -> TestResult {
    let mut f = fixture(600.0, &[])?;
    // "a" to the boundary after "c", the nearest to x 370.
    f.drag(150.0, 370.0);
    assert_eq!(f.order(), keys(&["name", "b", "c", "a", "d"]));
    assert_eq!(
        f.cells(),
        [
            ("name".to_owned(), 0.0),
            ("b".to_owned(), 100.0),
            ("c".to_owned(), 200.0),
            ("a".to_owned(), 300.0),
            ("d".to_owned(), 400.0),
        ],
        "every mounted row is rebuilt in the new order"
    );
    assert_eq!(
        *f.sorts.borrow(),
        0,
        "a drag on a sortable title does not sort"
    );
    Ok(())
}

#[test]
fn a_column_moves_only_among_its_movable_neighbours() -> TestResult {
    let mut f = fixture(600.0, &[])?;
    // Towards the fixed first column: as far as the start of the run.
    f.drag(250.0, 5.0);
    assert_eq!(f.order(), keys(&["name", "b", "a", "c", "d"]));
    // Past the fixed last column: as far as the end of the run.
    f.drag(150.0, 590.0);
    assert_eq!(f.order(), keys(&["name", "a", "c", "b", "d"]));
    // A column that is not movable starts no drag, and still sorts on a click.
    for x in [50.0, 450.0] {
        f.pointer(x, 10.0, PointerKind::Press(PointerButton::Primary));
        f.pointer(x + 60.0, 10.0, PointerKind::Move);
        assert!(!f.ui.drag_active());
        f.pointer(x + 60.0, 10.0, PointerKind::Release(PointerButton::Primary));
    }
    f.pointer(50.0, 10.0, PointerKind::Press(PointerButton::Primary));
    f.pointer(50.0, 10.0, PointerKind::Release(PointerButton::Primary));
    assert_eq!(*f.sorts.borrow(), 1);
    assert_eq!(f.order(), keys(&["name", "a", "c", "b", "d"]));
    Ok(())
}

#[test]
fn a_click_on_a_movable_sortable_title_still_sorts() -> TestResult {
    let mut f = fixture(600.0, &[])?;
    f.pointer(150.0, 10.0, PointerKind::Press(PointerButton::Primary));
    f.pointer(151.0, 10.0, PointerKind::Move);
    assert!(!f.ui.drag_active(), "below the threshold it is a click");
    f.pointer(151.0, 10.0, PointerKind::Release(PointerButton::Primary));
    assert_eq!(*f.sorts.borrow(), 1);
    assert_eq!(f.order(), keys(&["name", "a", "b", "c", "d"]));
    Ok(())
}

#[test]
fn the_line_marks_the_boundary_and_escape_leaves_the_order() -> TestResult {
    let mut f = fixture(600.0, &[])?;
    f.hold(150.0, 370.0);
    assert_eq!(f.property("drop_boundary"), Some(InspectValue::Number(4.0)));
    assert_eq!(f.ui.cursor_icon(), CursorIcon::Grabbing);
    f.ui.key_input(KeyCode::Escape, KeyAction::Press)?;
    f.ui.flush(f.ui.now())?;
    assert!(!f.ui.drag_active());
    assert_eq!(f.property("drop_boundary"), None);
    f.pointer(370.0, 60.0, PointerKind::Release(PointerButton::Primary));
    assert_eq!(f.order(), keys(&["name", "a", "b", "c", "d"]));
    Ok(())
}

#[test]
fn a_release_outside_the_table_moves_nothing() -> TestResult {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(600.0, 400.0, 1.0)?)?;
    let state = TableController::new(
        &ui,
        [
            TableColumn::new("a", "A")
                .width(Length::new(WIDTH)?)
                .movable(true),
            TableColumn::new("b", "B")
                .width(Length::new(WIDTH)?)
                .movable(true),
        ],
        TableSource::new(Cells(4)),
        ListSelection::new(&ui, SelectionMode::Single),
    )?;
    let table = Container::new(Element::new("table", Table::new(&state, ROW)?))
        .padding(Length::new(0.0)?)
        .preferred_size(Size::new(600.0, 200.0)?);
    ui.mount_tree(Element::new(
        "column",
        Column::new([
            Element::new("top", table),
            Element::new("below", Label::new("below")),
        ])
        .gap(Length::new(0.0)?),
    ))?;
    ui.flush(Duration::ZERO)?;
    let at = |ui: &mut Runtime, x: f32, y: f32, kind| {
        ui.pointer_input(PointerEvent::new(Point::new(x, y).unwrap(), kind))
            .unwrap();
        ui.flush(ui.now()).unwrap();
    };
    at(
        &mut ui,
        50.0,
        10.0,
        PointerKind::Press(PointerButton::Primary),
    );
    at(&mut ui, 56.0, 10.0, PointerKind::Move);
    assert!(ui.drag_active());
    at(&mut ui, 190.0, 300.0, PointerKind::Move);
    at(
        &mut ui,
        190.0,
        300.0,
        PointerKind::Release(PointerButton::Primary),
    );
    assert!(!ui.drag_active());
    let order: Vec<_> = state.columns().iter().map(|c| c.key().to_owned()).collect();
    assert_eq!(order, keys(&["a", "b"]));
    Ok(())
}

#[test]
fn a_drag_resting_near_an_edge_scrolls_overflowing_columns() -> TestResult {
    let mut f = fixture(300.0, &["e", "f"])?;
    f.hold(150.0, 290.0);
    assert_eq!(f.property("scroll_x"), Some(InspectValue::Number(0.0)));
    let deadline = f.ui.next_deadline().expect("an autoscroll tick is due");
    f.ui.flush(deadline)?;
    let Some(InspectValue::Number(scrolled)) = f.property("scroll_x") else {
        panic!("no scroll offset");
    };
    assert!(scrolled > 0.0, "the columns scrolled towards the pointer");
    f.pointer(150.0, 60.0, PointerKind::Move);
    f.pointer(150.0, 60.0, PointerKind::Release(PointerButton::Primary));
    assert_eq!(f.ui.next_deadline(), None, "nothing is left scheduled");
    Ok(())
}

#[test]
fn alt_with_left_or_right_moves_the_active_cells_column() -> TestResult {
    let mut f = fixture(600.0, &[])?;
    // A press on a cell of "a" makes its column active.
    f.pointer(150.0, 40.0, PointerKind::Press(PointerButton::Primary));
    f.pointer(150.0, 40.0, PointerKind::Release(PointerButton::Primary));
    assert_eq!(f.state.active_column().get()?.as_deref(), Some("a"));
    f.alt(KeyCode::ArrowRight);
    assert_eq!(f.order(), keys(&["name", "b", "a", "c", "d"]));
    f.alt(KeyCode::ArrowRight);
    f.alt(KeyCode::ArrowRight);
    assert_eq!(f.order(), keys(&["name", "b", "c", "a", "d"]), "not past d");
    f.alt(KeyCode::ArrowLeft);
    assert_eq!(f.order(), keys(&["name", "b", "a", "c", "d"]));
    assert_eq!(f.state.active_column().get()?.as_deref(), Some("a"));
    // Without Alt the arrows still move the active cell.
    f.ui.key_input(KeyCode::ArrowLeft, KeyAction::Press)?;
    f.ui.key_input(KeyCode::ArrowLeft, KeyAction::Release)?;
    f.ui.flush(f.ui.now())?;
    assert_eq!(f.state.active_column().get()?.as_deref(), Some("b"));
    Ok(())
}

#[test]
fn columns_are_not_movable_unless_asked() -> TestResult {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(400.0, 200.0, 1.0)?)?;
    let state = TableController::new(
        &ui,
        [TableColumn::new("a", "A"), TableColumn::new("b", "B")],
        TableSource::new(Cells(4)),
        ListSelection::new(&ui, SelectionMode::Single),
    )?;
    let table = ui.mount_tree(Element::new("table", Table::new(&state, ROW)?))?;
    ui.flush(Duration::ZERO)?;
    let at = |ui: &mut Runtime, x: f32, kind| {
        ui.pointer_input(PointerEvent::new(Point::new(x, 10.0).unwrap(), kind))
            .unwrap();
        ui.flush(ui.now()).unwrap();
    };
    at(&mut ui, 50.0, PointerKind::Press(PointerButton::Primary));
    at(&mut ui, 350.0, PointerKind::Move);
    assert!(!ui.drag_active());
    at(&mut ui, 350.0, PointerKind::Release(PointerButton::Primary));
    let snapshot = ui.inspect();
    let key = ui.key(&table)?;
    let semantics = snapshot
        .components
        .iter()
        .find(|component| component.key == key)
        .and_then(|component| component.details.semantics.clone())
        .expect("the table describes its headers");
    assert!(
        semantics
            .items
            .iter()
            .all(|item| !item.actions.contains(&SemanticAction::Move))
    );
    Ok(())
}

#[test]
fn a_moved_order_round_trips_through_the_workspace() -> TestResult {
    let mut f = fixture(600.0, &[])?;
    let described = f.ui.capture_workspace()?;
    assert!(
        !described.encode().contains("order"),
        "an unmoved table captures as before"
    );
    f.drag(150.0, 370.0);
    let moved = f.ui.capture_workspace()?;
    assert!(moved.encode().contains("order"));

    let mut fresh = fixture(600.0, &[])?;
    assert!(fresh.ui.restore_workspace(&moved)?.is_complete());
    fresh.ui.flush(Duration::ZERO)?;
    assert_eq!(fresh.order(), keys(&["name", "b", "c", "a", "d"]));
    assert_eq!(fresh.cells()[3], ("a".to_owned(), 300.0));
    assert_eq!(fresh.ui.capture_workspace()?, moved);

    // A document from before the move puts the described order back.
    assert!(fresh.ui.restore_workspace(&described)?.is_complete());
    fresh.ui.flush(Duration::ZERO)?;
    assert_eq!(fresh.order(), keys(&["name", "a", "b", "c", "d"]));
    Ok(())
}

#[test]
fn a_saved_order_keeps_columns_added_since_in_their_place() -> TestResult {
    let mut f = fixture(700.0, &[])?;
    f.drag(150.0, 370.0);
    let moved = f.ui.capture_workspace()?;
    // The application has since added the movable column "e" before "d".
    let mut grown = fixture(700.0, &["e"])?;
    assert!(grown.ui.restore_workspace(&moved)?.is_complete());
    assert_eq!(grown.order(), keys(&["name", "b", "c", "a", "e", "d"]));
    Ok(())
}

#[test]
fn a_saved_order_naming_a_removed_column_is_rejected_whole() -> TestResult {
    let mut f = fixture(700.0, &["e"])?;
    f.drag(150.0, 470.0);
    let moved = f.ui.capture_workspace()?;
    let mut shrunk = fixture(700.0, &[])?;
    let report = shrunk.ui.restore_workspace(&moved)?;
    assert!(
        report
            .rejected()
            .any(|(_, reason)| *reason == RestoreRejection::Unresolved("e".into()))
    );
    assert_eq!(shrunk.order(), keys(&["name", "a", "b", "c", "d"]));
    Ok(())
}

#[test]
fn a_keyboard_move_during_an_armed_title_press_keeps_the_pressed_column() -> TestResult {
    let mut f = fixture(600.0, &[])?;
    // Make "a" the active cell's column, then press its title.
    f.pointer(150.0, 40.0, PointerKind::Press(PointerButton::Primary));
    f.pointer(150.0, 40.0, PointerKind::Release(PointerButton::Primary));
    f.pointer(150.0, 10.0, PointerKind::Press(PointerButton::Primary));
    // Before the threshold, the keyboard moves "a" one place: every index
    // after the press now names another column.
    f.alt(KeyCode::ArrowRight);
    assert_eq!(f.order(), keys(&["name", "b", "a", "c", "d"]));
    f.pointer(156.0, 10.0, PointerKind::Move);
    assert!(f.ui.drag_active());
    let preview = f.ui.inspect().drag.and_then(|drag| drag.preview);
    assert!(preview.is_some());
    f.pointer(390.0, 60.0, PointerKind::Move);
    f.pointer(390.0, 60.0, PointerKind::Release(PointerButton::Primary));
    assert_eq!(
        f.order(),
        keys(&["name", "b", "c", "a", "d"]),
        "the pressed column moved, not the one now at its old index"
    );
    Ok(())
}

/// Tries to replace the table's rows from `update`, where writes are refused.
struct Meddler {
    state: TableController,
    refused: Rc<RefCell<bool>>,
}
impl Component for Meddler {
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        let result = self.state.set_source(TableSource::new(Cells(2)));
        *self.refused.borrow_mut() = result.is_err();
        Ok(())
    }
}

#[test]
fn a_refused_source_replacement_is_not_applied_by_a_later_move() -> TestResult {
    let mut f = fixture(600.0, &[])?;
    let refused = Rc::new(RefCell::new(false));
    // The table again, with the meddler as its sibling in place of the
    // fixture's own table.
    f.ui.unmount(&f.table)?;
    let column = f.ui.mount_tree(Element::new(
        "column",
        Column::new([
            Element::new("table", Table::new(&f.state, ROW)?),
            Element::new(
                "meddler",
                Meddler {
                    state: f.state.clone(),
                    refused: refused.clone(),
                },
            ),
        ])
        .gap(Length::new(0.0)?),
    ))?;
    f.ui.flush(Duration::ZERO)?;
    f.table = f.ui.children(&column)?[0].clone();
    let issues = f.ui.validation_report().issues;
    assert!(issues.is_empty(), "{issues:?}");
    assert!(*refused.borrow(), "a write during update is refused");
    f.drag(150.0, 370.0);
    assert_eq!(f.order(), keys(&["name", "b", "c", "a", "d"]));
    let body = f.ui.children(&f.table)?[0].clone();
    assert!(
        f.ui.children(&body)?.len() > 2,
        "the rows are still the original twenty"
    );
    Ok(())
}

#[test]
fn a_release_resting_at_the_edge_leaves_nothing_scheduled() -> TestResult {
    let mut f = fixture(300.0, &["e", "f"])?;
    f.hold(150.0, 290.0);
    assert!(f.ui.next_deadline().is_some(), "autoscroll is armed");
    f.pointer(290.0, 60.0, PointerKind::Release(PointerButton::Primary));
    assert!(!f.ui.drag_active());
    let deadline = f.ui.next_deadline();
    if let Some(deadline) = deadline {
        f.ui.flush(deadline)?;
    }
    assert_eq!(f.ui.next_deadline(), None);
    // Escape at the edge ends it the same way.
    f.hold(150.0, 290.0);
    f.ui.key_input(KeyCode::Escape, KeyAction::Press)?;
    f.ui.flush(f.ui.now())?;
    assert!(!f.ui.drag_active());
    if let Some(deadline) = f.ui.next_deadline() {
        f.ui.flush(deadline)?;
    }
    assert_eq!(f.ui.next_deadline(), None);
    Ok(())
}

#[test]
fn a_malformed_saved_order_is_rejected_without_changing_anything() -> TestResult {
    let mut f = fixture(600.0, &[])?;
    f.drag(150.0, 370.0);
    let moved = f.ui.capture_workspace()?.encode();
    let mut fresh = fixture(600.0, &[])?;
    let before = (fresh.order(), fresh.cells());
    // A duplicate position, a negative one and a value of another kind.
    for to in ["0", "-1", "\"x\""] {
        let text = moved.replacen("\"a\"=3", &format!("\"a\"={to}"), 1);
        // A width that differs shows whether anything was applied in part.
        let text = text.replacen("\"b\"=100.0", "\"b\"=150.0", 1);
        assert_ne!(text, moved, "the document holds `\"a\"=3`:\n{moved}");
        let document = Workspace::decode(&text)?;
        let report = fresh.ui.restore_workspace(&document)?;
        assert!(
            report.rejected().any(|(_, reason)| matches!(
                reason,
                RestoreRejection::OutOfRange(_) | RestoreRejection::Value(_)
            )),
            "{to} is rejected"
        );
        fresh.ui.flush(Duration::ZERO)?;
        assert_eq!((fresh.order(), fresh.cells()), before);
    }
    Ok(())
}

#[test]
fn finite_extreme_flex_weights_keep_layout_valid() -> TestResult {
    for weights in [
        vec![f32::MAX],
        vec![f32::MAX, f32::MAX],
        vec![f32::MIN_POSITIVE, f32::MAX],
    ] {
        let mut ui = Runtime::new();
        ui.set_viewport(Viewport::new(300.0, 200.0, 1.0)?)?;
        let columns = weights
            .iter()
            .enumerate()
            .map(|(index, weight)| {
                TableColumn::new(format!("c{index}"), format!("Column {index}")).flex(*weight)
            })
            .collect::<Result<Vec<_>, _>>()?;
        let controller = TableController::new(
            &ui,
            columns,
            TableSource::new(Cells(1)),
            ListSelection::new(&ui, SelectionMode::Single),
        )?;
        ui.mount_tree(Element::new("table", Table::new(&controller, ROW)?))?;
        ui.flush(Duration::ZERO)?;
        ui.paint()?;
        if weights.windows(2).all(|pair| pair[0] == pair[1]) {
            let widths = ui
                .inspect()
                .components
                .into_iter()
                .find(|component| component.key == "table")
                .and_then(|component| component.details.properties.get("column_widths").cloned());
            assert_eq!(
                widths,
                Some(InspectValue::List(vec![
                    InspectValue::Number(
                        300.0 / weights.len() as f64
                    );
                    weights.len()
                ]))
            );
        }
    }
    Ok(())
}

#[test]
fn finite_extreme_fixed_widths_keep_layout_valid() -> TestResult {
    for widths in [vec![f32::MAX, f32::MAX], vec![f32::MAX, 10.0, f32::MAX]] {
        let mut ui = Runtime::new();
        ui.set_viewport(Viewport::new(300.0, 200.0, 1.0)?)?;
        let columns = widths
            .iter()
            .enumerate()
            .map(|(index, width)| {
                TableColumn::new(format!("c{index}"), format!("Column {index}"))
                    .width(Length::new(*width).unwrap())
            })
            .collect::<Vec<_>>();
        let controller = TableController::new(
            &ui,
            columns,
            TableSource::new(Cells(1)),
            ListSelection::new(&ui, SelectionMode::Single),
        )?;
        ui.mount_tree(Element::new("table", Table::new(&controller, ROW)?))?;
        ui.flush(Duration::ZERO)?;
        ui.paint()?;
        // Scrolling to the far end in either direction keeps the offset
        // finite, and a scroll moves it.
        let mut moved = false;
        for delta in [-1.0e30, 1.0e30] {
            ui.pointer_input(PointerEvent::new(
                Point::new(150.0, 100.0)?,
                PointerKind::Scroll(Point::new(delta, 0.0)?),
            ))?;
            ui.flush(Duration::ZERO)?;
            ui.paint()?;
            let scroll = ui
                .inspect()
                .components
                .into_iter()
                .find(|component| component.key == "table")
                .and_then(|component| component.details.properties.get("scroll_x").cloned());
            let Some(InspectValue::Number(scroll)) = scroll else {
                panic!("the table reports its scroll offset");
            };
            assert!(
                scroll.is_finite() && scroll <= f64::from(f32::MAX),
                "{scroll}"
            );
            moved |= scroll > 0.0;
        }
        assert!(moved, "a wheel scrolls the overfull table");
        // Dragging the horizontal bar's thumb to either end does the same.
        let lane = 200.0 - 3.0;
        for (from, to) in [(2.0, 298.0), (298.0, 2.0)] {
            for (x, kind) in [
                (from, PointerKind::Move),
                (from, PointerKind::Press(PointerButton::Primary)),
                (to, PointerKind::Move),
                (to, PointerKind::Release(PointerButton::Primary)),
            ] {
                ui.pointer_input(PointerEvent::new(Point::new(x, lane)?, kind))?;
                ui.flush(Duration::ZERO)?;
                ui.paint()?;
            }
            let scroll = ui
                .inspect()
                .components
                .into_iter()
                .find(|component| component.key == "table")
                .and_then(|component| component.details.properties.get("scroll_x").cloned());
            let Some(InspectValue::Number(scroll)) = scroll else {
                panic!("the table reports its scroll offset");
            };
            assert!(
                scroll.is_finite() && scroll <= f64::from(f32::MAX),
                "{scroll}"
            );
        }
    }
    Ok(())
}

fn capability_fixture(columns: &[(&str, bool)]) -> Result<Fixture, Box<dyn std::error::Error>> {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(columns.len() as f32 * WIDTH, 200.0, 1.0)?)?;
    let state = TableController::new(
        &ui,
        columns.iter().map(|(key, movable)| {
            TableColumn::new(*key, *key)
                .width(Length::new(WIDTH).unwrap())
                .movable(*movable)
                .resizable(true)
        }),
        TableSource::new(Cells(4)),
        ListSelection::new(&ui, SelectionMode::Single),
    )?;
    let table = ui.mount_tree(Element::new("table", Table::new(&state, ROW)?))?;
    ui.flush(Duration::ZERO)?;
    Ok(Fixture {
        ui,
        state,
        table,
        sorts: Rc::new(RefCell::new(0)),
    })
}

fn saved_column_order(order: &[&str], width_key: &str) -> Result<Workspace, WorkspaceError> {
    let state = ViewState::new("table")?
        .with(
            "order",
            WorkspaceValue::map(
                order
                    .iter()
                    .enumerate()
                    .map(|(index, key)| ((*key).to_owned(), WorkspaceValue::integer(index as i64))),
            )?,
        )?
        .with(
            "widths",
            WorkspaceValue::map([(width_key.to_owned(), WorkspaceValue::number(150.0).unwrap())])?,
        )?;
    Workspace::new(
        WorkspaceSettings::default(),
        [(WorkspacePath::new(["table"])?, state)],
    )
}

#[test]
fn incompatible_saved_column_orders_preserve_widths_cells_and_capture() -> TestResult {
    for (columns, order) in [
        (
            vec![("fixed", false), ("a", true), ("b", true)],
            vec!["a", "fixed", "b"],
        ),
        (
            vec![("a", true), ("fixed", false), ("b", true)],
            vec!["fixed", "a", "b"],
        ),
        (
            vec![("a", true), ("b", true), ("fixed", false)],
            vec!["a", "fixed", "b"],
        ),
        // The fixed column itself stays put, but the others cross its boundary.
        (
            vec![("a", true), ("fixed", false), ("b", true)],
            vec!["b", "fixed", "a"],
        ),
        // A partial order still cannot cross an unlisted fixed separator.
        (
            vec![("a", true), ("b", true), ("fixed", false), ("c", true)],
            vec!["c", "a"],
        ),
    ] {
        let mut f = capability_fixture(&columns)?;
        let before = (f.order(), f.cells(), f.ui.capture_workspace()?);
        let widths: Vec<_> = f
            .state
            .columns()
            .iter()
            .map(TableColumn::fixed_width)
            .collect();
        let report = f.ui.restore_workspace(&saved_column_order(&order, "a")?)?;
        assert!(
            report
                .rejected()
                .any(|(_, reason)| matches!(reason, RestoreRejection::OutOfRange(_)))
        );
        f.ui.flush(f.ui.now())?;
        assert_eq!((f.order(), f.cells(), f.ui.capture_workspace()?), before);
        assert_eq!(
            f.state
                .columns()
                .iter()
                .map(TableColumn::fixed_width)
                .collect::<Vec<_>>(),
            widths
        );
    }
    Ok(())
}

#[test]
fn saved_orders_can_permute_independent_runs_and_leave_unlisted_columns_put() -> TestResult {
    let columns = [
        ("a", true),
        ("b", true),
        ("fixed", false),
        ("c", true),
        ("d", true),
    ];
    for order in [vec!["b", "a", "fixed", "d", "c"], vec!["b", "a", "d", "c"]] {
        let mut f = capability_fixture(&columns)?;
        let report = f.ui.restore_workspace(&saved_column_order(&order, "a")?)?;
        assert_eq!(report.rejected().count(), 0);
        f.ui.flush(f.ui.now())?;
        assert_eq!(f.order(), keys(&["b", "a", "fixed", "d", "c"]));
        assert_eq!(
            f.state
                .columns()
                .iter()
                .find(|column| column.key() == "a")
                .unwrap()
                .fixed_width(),
            Some(150.0)
        );
        assert_eq!(
            f.cells()
                .iter()
                .map(|(key, _)| key.clone())
                .collect::<Vec<_>>(),
            f.order()
        );
    }
    Ok(())
}

#[test]
fn restoration_checks_current_capabilities_when_recreating_a_table() -> TestResult {
    let movable = [("a", true), ("b", true), ("c", true)];
    let mut source = capability_fixture(&movable)?;
    let report = source
        .ui
        .restore_workspace(&saved_column_order(&["b", "a", "c"], "a")?)?;
    assert_eq!(report.rejected().count(), 0);
    source.ui.flush(source.ui.now())?;
    let saved = source.ui.capture_workspace()?;
    let mut restricted = capability_fixture(&[("a", true), ("b", false), ("c", true)])?;
    let before = (
        restricted.order(),
        restricted.cells(),
        restricted.ui.capture_workspace()?,
    );
    let report = restricted.ui.restore_workspace(&saved)?;
    assert!(
        report
            .rejected()
            .any(|(_, reason)| matches!(reason, RestoreRejection::OutOfRange(_)))
    );
    restricted.ui.flush(restricted.ui.now())?;
    assert_eq!(
        (
            restricted.order(),
            restricted.cells(),
            restricted.ui.capture_workspace()?
        ),
        before
    );
    let mut compatible = capability_fixture(&movable)?;
    let report = compatible.ui.restore_workspace(&saved)?;
    assert_eq!(report.rejected().count(), 0);
    compatible.ui.flush(compatible.ui.now())?;
    assert_eq!(compatible.order(), keys(&["b", "a", "c"]));
    assert_eq!(compatible.ui.capture_workspace()?, saved);
    Ok(())
}

#[test]
fn order_errors_precede_semantic_width_errors_without_partial_restoration() -> TestResult {
    for (order, expected) in [
        (
            vec!["b", "fixed", "a"],
            RestoreRejection::OutOfRange("a".into()),
        ),
        (vec!["gone"], RestoreRejection::Unresolved("gone".into())),
    ] {
        let mut f = capability_fixture(&[("a", true), ("fixed", false), ("b", true)])?;
        let before = (f.order(), f.cells(), f.ui.capture_workspace()?);
        let state = ViewState::new("table")?
            .with(
                "order",
                WorkspaceValue::map(order.iter().enumerate().map(|(position, key)| {
                    ((*key).to_owned(), WorkspaceValue::integer(position as i64))
                }))?,
            )?
            .with(
                "widths",
                WorkspaceValue::map([("b".to_owned(), WorkspaceValue::number(-1.0).unwrap())])?,
            )?;
        let saved = Workspace::new(
            WorkspaceSettings::default(),
            [(WorkspacePath::new(["table"])?, state)],
        )?;
        let report = f.ui.restore_workspace(&saved)?;
        assert!(report.rejected().any(|(_, reason)| *reason == expected));
        f.ui.flush(f.ui.now())?;
        assert_eq!((f.order(), f.cells(), f.ui.capture_workspace()?), before);
    }
    Ok(())
}

#[test]
fn saved_positions_must_be_the_listed_columns_numbered_from_zero() -> TestResult {
    for positions in [[500, 501], [1, 2], [0, 2]] {
        let mut f = capability_fixture(&[("a", true), ("b", true)])?;
        let state =
            ViewState::new("table")?
                .with("widths", WorkspaceValue::map(std::iter::empty())?)?
                .with(
                    "order",
                    WorkspaceValue::map(["b", "a"].into_iter().zip(positions).map(
                        |(key, position)| (key.to_owned(), WorkspaceValue::integer(position)),
                    ))?,
                )?;
        let document = Workspace::new(
            WorkspaceSettings::default(),
            [(WorkspacePath::new(["table"])?, state)],
        )?;
        let before = f.order();
        let report = f.ui.restore_workspace(&document)?;
        assert!(
            report
                .rejected()
                .any(|(_, reason)| matches!(reason, RestoreRejection::OutOfRange(_))),
            "{positions:?} is rejected"
        );
        f.ui.flush(Duration::ZERO)?;
        assert_eq!(f.order(), before, "{positions:?} changes nothing");
    }
    Ok(())
}

#[test]
fn direct_restore_after_shutdown_rejects_without_changing_columns() -> TestResult {
    for order in [["a", "b"], ["b", "a"]] {
        let mut ui = Runtime::new();
        let state = TableController::new(
            &ui,
            [
                TableColumn::new("a", "A").movable(true),
                TableColumn::new("b", "B").movable(true),
            ],
            TableSource::new(Cells(1)),
            ListSelection::new(&ui, SelectionMode::Single),
        )?;
        let mut table = Table::new(&state, ROW)?;
        let before = table.workspace_state()?;
        let columns = state.columns();
        let saved = saved_column_order(&order, "a")?;
        let entry = saved.entries().next().unwrap().1;
        ui.shutdown();
        assert_eq!(
            table.restore_workspace_state(entry),
            Err(RestoreRejection::Runtime(RuntimeError::Closed))
        );
        assert_eq!(state.columns(), columns);
        assert_eq!(table.workspace_state()?, before);
    }
    Ok(())
}

#[test]
fn direct_restore_during_update_preserves_columns_and_capture() -> TestResult {
    struct RestoreDuringUpdate {
        table: Table,
        entry: ViewState,
        checked: Rc<RefCell<bool>>,
    }
    impl Component for RestoreDuringUpdate {
        fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
            let before = self.table.workspace_state().unwrap();
            assert_eq!(
                self.table.restore_workspace_state(&self.entry),
                Err(RestoreRejection::Runtime(RuntimeError::WriteDuringUpdate))
            );
            assert_eq!(self.table.workspace_state().unwrap(), before);
            *self.checked.borrow_mut() = true;
            Ok(())
        }
    }
    for order in [["a", "b"], ["b", "a"]] {
        let mut ui = Runtime::new();
        let state = TableController::new(
            &ui,
            [
                TableColumn::new("a", "A").movable(true),
                TableColumn::new("b", "B").movable(true),
            ],
            TableSource::new(Cells(1)),
            ListSelection::new(&ui, SelectionMode::Single),
        )?;
        let columns = state.columns();
        let counts = state.filter_counts().get()?;
        let saved = saved_column_order(&order, "a")?;
        let entry = saved.entries().next().unwrap().1.clone();
        let checked = Rc::new(RefCell::new(false));
        ui.mount_tree(Element::new(
            "restore",
            RestoreDuringUpdate {
                table: Table::new(&state, ROW)?,
                entry,
                checked: checked.clone(),
            },
        ))?;
        ui.flush(Duration::ZERO)?;
        assert!(*checked.borrow());
        assert_eq!(state.columns(), columns);
        assert_eq!(state.filter_counts().get()?, counts);
    }
    Ok(())
}

#[test]
fn publication_failure_restores_existing_user_sized_columns() -> TestResult {
    struct ShutdownOnLen {
        runtime: std::rc::Weak<RefCell<Runtime>>,
        armed: Rc<std::cell::Cell<bool>>,
    }
    impl TableItems for ShutdownOnLen {
        fn len(&self) -> usize {
            if self.armed.replace(false) {
                self.runtime.upgrade().unwrap().borrow_mut().shutdown();
            }
            1
        }
        fn key(&self, index: usize) -> Option<String> {
            (index == 0).then(|| "row".to_owned())
        }
        fn cell(&self, _: usize, _: &str) -> Option<Element> {
            None
        }
    }

    let ui = Rc::new(RefCell::new(Runtime::new()));
    let armed = Rc::new(std::cell::Cell::new(false));
    let state = {
        let runtime = ui.borrow();
        TableController::new(
            &*runtime,
            [
                TableColumn::new("a", "A").movable(true),
                TableColumn::new("b", "B").movable(true),
            ],
            TableSource::new(ShutdownOnLen {
                runtime: Rc::downgrade(&ui),
                armed: armed.clone(),
            }),
            ListSelection::new(&*runtime, SelectionMode::Single),
        )?
    };
    let mut table = Table::new(&state, ROW)?;
    let original = saved_column_order(&["a", "b"], "a")?;
    table.restore_workspace_state(original.entries().next().unwrap().1)?;
    let before = table.workspace_state()?;
    let columns = state.columns();
    assert_eq!(columns[0].fixed_width(), Some(150.0));
    assert_eq!(columns[1].fixed_width(), None);

    // Unlike preflight rejection, shutdown here happens when the reordered
    // columns are republished. The previous user-sized set must survive too.
    let replacement = saved_column_order(&["b", "a"], "b")?;
    armed.set(true);
    assert_eq!(
        table.restore_workspace_state(replacement.entries().next().unwrap().1),
        Err(RestoreRejection::Runtime(RuntimeError::Closed))
    );
    assert!(
        !armed.get(),
        "restore must reach the row source after preflight"
    );
    assert_eq!(state.columns(), columns);
    assert_eq!(table.workspace_state()?, before);
    Ok(())
}
