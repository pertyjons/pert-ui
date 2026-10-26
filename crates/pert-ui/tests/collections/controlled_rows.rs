//! Controlled controls in collection rows (F16): a `VirtualList` and a
//! `Table` describe their mounted rows again on every source replacement and
//! on scrolls that mount another row, and a controlled `Checkbox` or
//! `InlineText` keeps its draft or armed press through those descriptions.
//!
//! These are integration tests: they check the controlled path with the
//! collections' existing redescription and the settle loop, not a transition
//! compared with the `new` behavior. Ending the draft on every description
//! also fails the draft assertions here.
use pert_ui::*;
use std::{cell::RefCell, rc::Rc, time::Duration};

type TestResult = Result<(), Box<dyn std::error::Error>>;
type Log = Rc<RefCell<Vec<String>>>;
const ROW: f32 = 32.0;

#[derive(Clone, PartialEq)]
struct Task {
    id: u64,
    title: String,
    done: bool,
}

fn tasks(count: u64) -> Vec<Task> {
    (1..=count)
        .map(|id| Task {
            id,
            title: format!("Task {id}"),
            done: false,
        })
        .collect()
}

fn edit(
    store: &Signal<Vec<Task>>,
    id: u64,
    change: impl FnOnce(&mut Task),
) -> Result<(), RuntimeError> {
    store.update(|tasks| {
        if let Some(task) = tasks.iter_mut().find(|task| task.id == id) {
            change(task);
        }
    })
}

/// The records as rows of a controlled checkbox and a controlled title,
/// whose callbacks write the records and log each proposal.
struct Rows {
    tasks: Vec<Task>,
    store: Signal<Vec<Task>>,
    log: Log,
}
impl ListItems for Rows {
    fn len(&self) -> usize {
        self.tasks.len()
    }
    fn key(&self, index: usize) -> Option<String> {
        Some(self.tasks.get(index)?.id.to_string())
    }
    fn build(&self, index: usize) -> Option<Element> {
        let task = self.tasks.get(index)?;
        let id = task.id;
        let (store, log) = (self.store.clone(), self.log.clone());
        let check = Checkbox::controlled(task.done, "Done", move |_, done| {
            log.borrow_mut().push(format!("{id} done {done}"));
            edit(&store, id, |task| task.done = done)
        });
        let (store, log) = (self.store.clone(), self.log.clone());
        let title = InlineText::controlled(task.title.clone(), move |_, title| {
            log.borrow_mut().push(format!("{id} title {title}"));
            edit(&store, id, |task| task.title = title.to_owned())
        })
        .in_row(true);
        Some(Row::new([check.key("done"), title.key("title").with_grow(1)]).key("row"))
    }
    fn text(&self, index: usize) -> Option<String> {
        Some(self.tasks.get(index)?.title.clone())
    }
}

/// The properties of one mounted component.
fn property(ui: &Runtime, id: &WidgetId, name: &str) -> Result<Option<InspectValue>, RuntimeError> {
    let id = ui.inspection_id(id)?;
    Ok(ui
        .query(&InspectionQuery::components(Selector::id(id)).detail(Detail::Full))
        .components
        .pop()
        .and_then(|component| component.details.properties.get(name).cloned()))
}

fn text(value: &str) -> Option<InspectValue> {
    Some(InspectValue::Text(value.to_owned()))
}

fn mounted(ui: &Runtime, list: &WidgetId) -> Result<Vec<String>, RuntimeError> {
    ui.children(list)?
        .iter()
        .map(|child| ui.key(child).map(str::to_owned))
        .collect()
}

fn begin(ui: &mut Runtime, title: &WidgetId, typed: &str) -> TestResult {
    assert_eq!(
        ui.text_command(title, TextCommand::BeginEdit)?,
        TextCommandOutcome::Handled
    );
    ui.flush(Duration::ZERO)?;
    ui.text_input(typed)?;
    Ok(())
}

#[test]
fn a_list_row_keeps_its_draft_across_record_changes_and_scrolling() -> TestResult {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(300.0, 100.0, 1.0)?)?;
    let store = ui.signal(tasks(30));
    let log: Log = Rc::default();
    let (records, saves) = (store.clone(), log.clone());
    let source = store.map(move |tasks| {
        ListSource::new(Rows {
            tasks: tasks.clone(),
            store: records.clone(),
            log: saves.clone(),
        })
    })?;
    let list = ui.mount_tree(VirtualList::new(source, ROW)?.key("list"))?;
    ui.flush(Duration::ZERO)?;
    let title = ui.find(&list, &["2", "title"])?.ok_or("a title")?;
    begin(&mut ui, &title, "Typed")?;

    // A change to another record describes every mounted row again.
    edit(&store, 1, |task| task.done = true)?;
    ui.flush(Duration::ZERO)?;
    assert_eq!(
        property(&ui, &title, "editing")?,
        Some(InspectValue::Bool(true))
    );
    assert_eq!(property(&ui, &title, "draft")?, text("Typed"));
    let check = ui.find(&list, &["1", "done"])?.ok_or("a checkbox")?;
    assert_eq!(
        property(&ui, &check, "checked")?,
        Some(InspectValue::Bool(true))
    );

    // So does a scroll that mounts another row.
    let before = mounted(&ui, &list)?;
    ui.pointer_input(PointerEvent::new(
        Point::new(10.0, 10.0)?,
        PointerKind::Scroll(Point::new(0.0, 2.0 * ROW)?),
    ))?;
    ui.flush(Duration::ZERO)?;
    let after = mounted(&ui, &list)?;
    assert!(
        after.iter().any(|key| !before.contains(key)),
        "{before:?} -> {after:?}"
    );
    assert_eq!(ui.find(&list, &["2", "title"])?, Some(title.clone()));
    assert_eq!(property(&ui, &title, "draft")?, text("Typed"));

    // Enter proposes the draft, the callback applies it, and exactly one
    // flush shows the new title.
    ui.key_input(KeyCode::Enter, KeyAction::Press)?;
    ui.key_input(KeyCode::Enter, KeyAction::Release)?;
    assert_eq!(*log.borrow(), ["2 title Typed"]);
    assert_eq!(store.get()?[1].title, "Typed");
    ui.flush(Duration::ZERO)?;
    assert_eq!(property(&ui, &title, "text")?, text("Typed"));
    assert_eq!(
        property(&ui, &title, "editing")?,
        Some(InspectValue::Bool(false))
    );
    Ok(())
}

#[test]
fn a_filter_keeps_the_draft_of_a_row_it_keeps_and_ends_one_it_removes() -> TestResult {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(300.0, 300.0, 1.0)?)?;
    let store = ui.signal(tasks(5));
    let log: Log = Rc::default();
    let (records, saves) = (store.clone(), log.clone());
    let source = store.map(move |tasks| {
        ListSource::new(Rows {
            tasks: tasks.clone(),
            store: records.clone(),
            log: saves.clone(),
        })
    })?;
    let filter = ListFilter::new(&ui, source)?;
    let list = ui.mount_tree(VirtualList::new(filter.items(), ROW)?.key("list"))?;
    ui.flush(Duration::ZERO)?;
    let matcher = FilterMatcher::substring();
    let title = ui.find(&list, &["2", "title"])?.ok_or("a title")?;
    begin(&mut ui, &title, "Typed")?;

    // A query that keeps the edited row keeps its draft.
    filter.filter("Task", &matcher)?;
    ui.flush(Duration::ZERO)?;
    assert_eq!(ui.find(&list, &["2", "title"])?, Some(title.clone()));
    assert_eq!(property(&ui, &title, "draft")?, text("Typed"));

    // One that removes the row ends the draft unsaved, with no callback.
    filter.filter("Task 3", &matcher)?;
    ui.flush(Duration::ZERO)?;
    assert_eq!(ui.find(&list, &["2"])?, None);
    assert!(log.borrow().is_empty(), "{:?}", log.borrow());
    assert_eq!(store.get()?[1].title, "Task 2");
    filter.filter("", &matcher)?;
    ui.flush(Duration::ZERO)?;
    let title = ui.find(&list, &["2", "title"])?.ok_or("a title")?;
    assert_eq!(
        property(&ui, &title, "editing")?,
        Some(InspectValue::Bool(false))
    );
    assert_eq!(property(&ui, &title, "text")?, text("Task 2"));
    Ok(())
}

/// The inventory case: a controlled checkbox in a table cell.
struct Cells {
    tasks: Vec<Task>,
    store: Signal<Vec<Task>>,
    table: Rc<RefCell<Option<TableController>>>,
    log: Log,
}
impl TableItems for Cells {
    fn len(&self) -> usize {
        self.tasks.len()
    }
    fn key(&self, index: usize) -> Option<String> {
        Some(self.tasks.get(index)?.id.to_string())
    }
    fn cell(&self, index: usize, column: &str) -> Option<Element> {
        let task = self.tasks.get(index)?;
        let id = task.id;
        match column {
            "done" => {
                let (store, table, log) =
                    (self.store.clone(), self.table.clone(), self.log.clone());
                Some(
                    Checkbox::controlled(task.done, "", move |_, done| {
                        log.borrow_mut().push(format!("{id} done {done}"));
                        edit(&store, id, |task| task.done = done)?;
                        publish(&store, &table, &log)
                    })
                    .key("check"),
                )
            }
            "title" => Some(Label::new(task.title.as_str()).key("text")),
            _ => None,
        }
    }
}

/// Publishes the records as the table's rows, as the application does after
/// each change.
fn publish(
    store: &Signal<Vec<Task>>,
    table: &Rc<RefCell<Option<TableController>>>,
    log: &Log,
) -> Result<(), RuntimeError> {
    let rows = Cells {
        tasks: store.get()?,
        store: store.clone(),
        table: table.clone(),
        log: log.clone(),
    };
    match &*table.borrow() {
        Some(table) => table.set_source(TableSource::new(rows)),
        None => Ok(()),
    }
}

#[test]
fn a_table_cell_checkbox_keeps_its_press_and_shows_the_applied_value() -> TestResult {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(300.0, 120.0, 1.0)?)?;
    let store = ui.signal(tasks(30));
    let log: Log = Rc::default();
    let table: Rc<RefCell<Option<TableController>>> = Rc::default();
    let controller = TableController::new(
        &ui,
        ListSelection::new(&ui, SelectionMode::Single),
        [
            TableColumn::new("done", "Done").width(Length::new(60.0)?),
            TableColumn::new("title", "Title"),
        ],
        TableSource::new(Cells {
            tasks: store.get()?,
            store: store.clone(),
            table: table.clone(),
            log: log.clone(),
        }),
    )?;
    *table.borrow_mut() = Some(controller.clone());
    let root = ui.mount_tree(Table::new(&controller, ROW)?.key("table"))?;
    ui.flush(Duration::ZERO)?;
    let check = ui.find_descendant(&root, "2")?.ok_or("a row")?;
    let check = ui.find_descendant(&check, "check")?.ok_or("a checkbox")?;
    let [x, y, _, height] = ui.bounds(&check)?.ok_or("laid out")?.values();
    let at = Point::new(x + 10.0, y + height / 2.0)?;

    // A pointer press survives a change to another record.
    ui.pointer_input(PointerEvent::new(
        at,
        PointerKind::Press(PointerButton::Primary),
    ))?;
    edit(&store, 3, |task| task.done = true)?;
    publish(&store, &table, &log)?;
    ui.flush(Duration::ZERO)?;
    ui.pointer_input(PointerEvent::new(
        at,
        PointerKind::Release(PointerButton::Primary),
    ))?;
    assert_eq!(*log.borrow(), ["2 done true"]);
    assert!(store.get()?[1].done);
    ui.flush(Duration::ZERO)?;
    assert_eq!(
        property(&ui, &check, "checked")?,
        Some(InspectValue::Bool(true))
    );

    // A keyboard press survives a scroll that mounts another row.
    ui.focus(&check)?;
    ui.flush(Duration::ZERO)?;
    ui.key_input(KeyCode::Space, KeyAction::Press)?;
    let unmounted = (1..=30)
        .map(|id: u64| id.to_string())
        .find(|key| matches!(ui.find_descendant(&root, key), Ok(None)))
        .ok_or("an unmounted row")?;
    let body = ui.bounds(&root)?.ok_or("laid out")?.values();
    ui.pointer_input(PointerEvent::new(
        Point::new(body[0] + 100.0, body[1] + 2.0 * ROW)?,
        PointerKind::Scroll(Point::new(0.0, ROW)?),
    ))?;
    ui.flush(Duration::ZERO)?;
    assert!(
        ui.find_descendant(&root, &unmounted)?.is_some(),
        "row {unmounted} mounted"
    );
    assert_eq!(ui.focused(), Some(check.clone()));
    ui.key_input(KeyCode::Space, KeyAction::Release)?;
    assert_eq!(*log.borrow(), ["2 done true", "2 done false"]);
    ui.flush(Duration::ZERO)?;
    assert_eq!(
        property(&ui, &check, "checked")?,
        Some(InspectValue::Bool(false))
    );
    Ok(())
}
