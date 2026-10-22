//! Selection notifications through the facade: lists, trees and tables report
//! each change of their `ListSelection` with its origin, after the flush
//! settles, and the application can follow the user's choice.
use pert_ui::*;
use std::{cell::RefCell, rc::Rc, time::Duration};

type TestResult = Result<(), Box<dyn std::error::Error>>;
type Log = Rc<RefCell<Vec<ListSelectionChange>>>;
const ROW: f32 = 24.0;

struct Words(Vec<&'static str>);
impl ListItems for Words {
    fn len(&self) -> usize {
        self.0.len()
    }
    fn key(&self, index: usize) -> Option<String> {
        self.0.get(index).map(|word| (*word).to_owned())
    }
    fn build(&self, index: usize) -> Option<Element> {
        Some(Label::new(*self.0.get(index)?).key("row"))
    }
    fn text(&self, index: usize) -> Option<String> {
        self.key(index)
    }
}

fn logging() -> (
    Log,
    impl FnMut(&mut EventContext, &ListSelectionChange) -> Result<(), RuntimeError>,
) {
    let log: Log = Rc::default();
    let record = log.clone();
    (
        log,
        move |_: &mut EventContext, change: &ListSelectionChange| {
            record.borrow_mut().push(change.clone());
            Ok(())
        },
    )
}

fn origins(log: &Log) -> Vec<(Option<String>, ChangeOrigin)> {
    log.borrow()
        .iter()
        .map(|change| (change.active.clone(), change.origin))
        .collect()
}

fn press(ui: &mut Runtime, code: KeyCode) -> Result<(), RuntimeError> {
    ui.key_input(code, KeyAction::Press)?;
    ui.key_input(code, KeyAction::Release)?;
    Ok(())
}

fn click(ui: &mut Runtime, row: &WidgetId) -> TestResult {
    let bounds = ui.bounds(row)?.ok_or("the row is laid out")?;
    let at = Point::new(bounds.x() + 10.0, bounds.y() + bounds.height() / 2.0)?;
    ui.pointer_input(PointerEvent::new(
        at,
        PointerKind::Press(PointerButton::Primary),
    ))?;
    ui.pointer_input(PointerEvent::new(
        at,
        PointerKind::Release(PointerButton::Primary),
    ))?;
    ui.flush(Duration::ZERO)?;
    Ok(())
}

struct List {
    ui: Runtime,
    list: WidgetId,
    selection: ListSelection,
    log: Log,
}

fn list(mode: SelectionMode) -> Result<List, Box<dyn std::error::Error>> {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(200.0, 200.0, 1.0)?)?;
    let items = ui.signal(ListSource::new(Words(vec!["apple", "pear", "plum"])));
    let selection = ListSelection::new(&ui, mode);
    let (log, record) = logging();
    let list = ui.mount_tree(
        VirtualList::new(&items, ROW)?
            .selection(selection.clone())
            .on_selection_change(record)
            .key("list"),
    )?;
    ui.flush(Duration::ZERO)?;
    Ok(List {
        ui,
        list,
        selection,
        log,
    })
}

#[test]
fn the_initial_selection_is_no_change_and_user_input_reports_user() -> TestResult {
    let mut f = list(SelectionMode::Single)?;
    assert!(f.log.borrow().is_empty());
    f.ui.focus(&f.list)?;
    press(&mut f.ui, KeyCode::ArrowDown)?;
    let pear = f.ui.find_path(&f.list, &["pear"])?.ok_or("mounted")?;
    click(&mut f.ui, &pear)?;
    assert_eq!(
        origins(&f.log),
        [
            (Some("apple".to_owned()), ChangeOrigin::User),
            (Some("pear".to_owned()), ChangeOrigin::User),
        ]
    );
    let last = f.log.borrow().last().cloned().ok_or("reported")?;
    assert_eq!(last.previous_active.as_deref(), Some("apple"));
    assert_eq!(*last.selected, ["pear".to_owned()].into_iter().collect());
    Ok(())
}

#[test]
fn application_writes_report_application_and_coalesce_until_the_next_update() -> TestResult {
    let mut f = list(SelectionMode::Single)?;
    f.selection.set_active(Some("pear".to_owned()))?;
    f.selection.set_active(Some("plum".to_owned()))?;
    f.ui.flush(Duration::ZERO)?;
    assert_eq!(
        origins(&f.log),
        [(Some("plum".to_owned()), ChangeOrigin::Application)]
    );
    assert_eq!(f.log.borrow()[0].previous_active, None);
    // A write back to the reported state is no change.
    f.selection.set_active(Some("apple".to_owned()))?;
    f.selection.set_active(Some("plum".to_owned()))?;
    f.ui.flush(Duration::ZERO)?;
    assert_eq!(f.log.borrow().len(), 1);
    Ok(())
}

#[test]
fn a_multiple_selection_reports_a_set_change_with_the_same_active_item() -> TestResult {
    let mut f = list(SelectionMode::Multiple)?;
    f.ui.focus(&f.list)?;
    press(&mut f.ui, KeyCode::ArrowDown)?;
    press(&mut f.ui, KeyCode::Space)?;
    let log = f.log.borrow();
    let toggle = log.last().ok_or("reported")?;
    assert_eq!(toggle.origin, ChangeOrigin::User);
    assert_eq!(toggle.previous_active, toggle.active);
    assert_ne!(toggle.previous_selected, toggle.selected);
    Ok(())
}

#[test]
fn a_callback_may_write_and_its_error_does_not_undo_the_change() -> TestResult {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(200.0, 200.0, 1.0)?)?;
    let items = ui.signal(ListSource::new(Words(vec!["apple", "pear"])));
    let selection = ListSelection::new(&ui, SelectionMode::Single);
    let detail = ui.signal(String::new());
    let shown = detail.clone();
    ui.mount_tree(
        VirtualList::new(&items, ROW)?
            .selection(selection.clone())
            .on_selection_change(move |_, change| {
                shown.set(change.active.clone().unwrap_or_default())?;
                Err(RuntimeError::Exhausted)
            })
            .key("list"),
    )?;
    ui.flush(Duration::ZERO)?;
    selection.set_active(Some("pear".to_owned()))?;
    assert_eq!(
        ui.flush(Duration::ZERO).err(),
        Some(RuntimeError::Exhausted)
    );
    assert_eq!(detail.get()?, "pear");
    assert_eq!(selection.active().get()?.as_deref(), Some("pear"));
    Ok(())
}

#[test]
fn collections_sharing_a_selection_report_a_user_choice_as_the_users() -> TestResult {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(400.0, 200.0, 1.0)?)?;
    let items = ui.signal(ListSource::new(Words(vec!["apple", "pear"])));
    let selection = ListSelection::new(&ui, SelectionMode::Single);
    let ((left_log, left), (right_log, right)) = (logging(), logging());
    let sized = |list: VirtualList, key: &str| -> Result<Element, GeometryError> {
        Ok(Container::new(list.key(key))
            .preferred_size(Size::new(200.0, 200.0)?)
            .key(format!("{key}-frame")))
    };
    let row = ui.mount_tree(
        Row::new([
            sized(
                VirtualList::new(&items, ROW)?
                    .selection(selection.clone())
                    .on_selection_change(left),
                "left",
            )?,
            sized(
                VirtualList::new(&items, ROW)?
                    .selection(selection.clone())
                    .on_selection_change(right),
                "right",
            )?,
        ])
        .key("row"),
    )?;
    ui.flush(Duration::ZERO)?;
    let pear = ui
        .find_path(&row, &["left-frame", "left", "pear"])?
        .ok_or("mounted")?;
    click(&mut ui, &pear)?;
    let expected = [(Some("pear".to_owned()), ChangeOrigin::User)];
    assert_eq!(origins(&left_log), expected);
    assert_eq!(origins(&right_log), expected);
    Ok(())
}

#[test]
fn a_hidden_list_reports_a_change_made_meanwhile_when_shown() -> TestResult {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(200.0, 200.0, 1.0)?)?;
    let items = ui.signal(ListSource::new(Words(vec!["apple", "pear"])));
    let selection = ListSelection::new(&ui, SelectionMode::Single);
    let shown = ui.signal(true);
    let (log, record) = logging();
    ui.mount_tree(
        View::new(
            VirtualList::new(&items, ROW)?
                .selection(selection.clone())
                .on_selection_change(record)
                .key("list"),
        )
        .visible_signal(&shown)
        .key("view"),
    )?;
    ui.flush(Duration::ZERO)?;
    shown.set(false)?;
    ui.flush(Duration::ZERO)?;
    selection.set_active(Some("pear".to_owned()))?;
    ui.flush(Duration::ZERO)?;
    assert!(log.borrow().is_empty(), "a hidden list does not update");
    shown.set(true)?;
    ui.flush(Duration::ZERO)?;
    assert_eq!(
        origins(&log),
        [(Some("pear".to_owned()), ChangeOrigin::Application)]
    );
    Ok(())
}

struct Outline;
impl TreeItems for Outline {
    fn roots(&self) -> Vec<String> {
        vec!["a".into(), "b".into()]
    }
    fn children(&self, key: &str) -> Vec<String> {
        if key == "a" {
            vec!["a/1".into()]
        } else {
            Vec::new()
        }
    }
    fn build(&self, key: &str) -> Option<Element> {
        Some(Label::new(key).key("row"))
    }
}

#[test]
fn a_tree_reports_cross_keys_as_user_and_controller_writes_as_application() -> TestResult {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(200.0, 200.0, 1.0)?)?;
    let selection = ListSelection::new(&ui, SelectionMode::Single);
    let state = TreeController::new(&ui, selection.clone(), TreeSource::new(Outline))?;
    let (log, record) = logging();
    let tree = ui.mount_tree(
        Tree::new(&state, ROW)?
            .on_selection_change(record)
            .key("tree"),
    )?;
    ui.flush(Duration::ZERO)?;
    state.set_expanded("a", true)?;
    selection.set_active(Some("a".to_owned()))?;
    ui.flush(Duration::ZERO)?;
    ui.focus(&tree)?;
    // Right on an expanded node moves to its first child.
    press(&mut ui, KeyCode::ArrowRight)?;
    // Collapsing the parent through the controller moves the active node up.
    state.set_expanded("a", false)?;
    ui.flush(Duration::ZERO)?;
    assert_eq!(
        origins(&log),
        [
            (Some("a".to_owned()), ChangeOrigin::Application),
            (Some("a/1".to_owned()), ChangeOrigin::User),
            (Some("a".to_owned()), ChangeOrigin::Application),
        ]
    );
    Ok(())
}

struct Numbers;
impl TableItems for Numbers {
    fn len(&self) -> usize {
        3
    }
    fn key(&self, index: usize) -> Option<String> {
        (index < 3).then(|| format!("n{index}"))
    }
    fn cell(&self, index: usize, _: &str) -> Option<Element> {
        Some(Label::new(index.to_string()).key("text"))
    }
}

#[test]
fn a_table_reports_a_row_click_as_user() -> TestResult {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(300.0, 200.0, 1.0)?)?;
    let selection = ListSelection::new(&ui, SelectionMode::Single);
    let state = TableController::new(
        &ui,
        selection,
        [TableColumn::new("number", "Number")],
        TableSource::new(Numbers),
    )?;
    let (log, record) = logging();
    let table = ui.mount_tree(
        Table::new(&state, ROW)?
            .on_selection_change(record)
            .key("table"),
    )?;
    ui.flush(Duration::ZERO)?;
    let row = ui.find_descendant(&table, "n1")?.ok_or("mounted")?;
    click(&mut ui, &row)?;
    assert_eq!(origins(&log), [(Some("n1".to_owned()), ChangeOrigin::User)]);
    Ok(())
}

#[test]
fn a_filter_moving_the_active_row_off_a_hidden_one_reports_application() -> TestResult {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(300.0, 300.0, 1.0)?)?;
    let source = ui.signal(ListSource::new(Words(vec!["apple", "banana", "blueberry"])));
    let selection = ListSelection::new(&ui, SelectionMode::Single);
    let filter = ListFilter::new(&ui, &source)?.selection(selection.clone());
    let (log, record) = logging();
    let list = VirtualList::new(filter.items(), ROW)?
        .selection(selection.clone())
        .on_selection_change(record);
    let query = ui.signal(String::new());
    let root = ui.mount_tree(
        CollectionFilter::with_query(query.clone(), filter, list.key("list")).key("filter-root"),
    )?;
    ui.flush(Duration::ZERO)?;
    let list = ui.find_descendant(&root, "list")?.ok_or("mounted")?;
    ui.focus(&list)?;
    selection.set_active(Some("banana".to_owned()))?;
    ui.flush(Duration::ZERO)?;
    // With the collection focused, the query changes elsewhere: the move off
    // the hidden row is the application's.
    query.set("blue".to_owned())?;
    ui.flush(Duration::ZERO)?;
    assert_eq!(
        origins(&log).last(),
        Some(&(Some("blueberry".to_owned()), ChangeOrigin::Application))
    );
    Ok(())
}
