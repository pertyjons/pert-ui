//! A small to-do list on pert-ui.
//!
//! The tasks live in an application-owned `Signal<Vec<Task>>`; the list, the
//! status line and every action read from and write to that vector.

use std::cell::Cell;
use std::mem;
use std::rc::Rc;

use pert_ui::prelude::*;
use pert_ui::{
    CollectionFilter, CommandKey, HiddenActive, ListDrop, ListDropPosition, ListFilter, ListItems,
    ListSelection, ListSource, Modifiers, SelectionMode, TextCommand, VirtualList,
};

/// One task. Signal values are compared when written and cloned when read.
#[derive(Clone, Debug, PartialEq)]
pub struct Task {
    pub id: u64,
    pub title: String,
    pub done: bool,
}

/// The records as list rows: a copy for reading, and the signal to write back to.
struct Rows {
    tasks: Vec<Task>,
    store: Signal<Vec<Task>>,
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
        let (store, id) = (self.store.clone(), task.id);
        let check = Checkbox::controlled(task.done, "Done", move |_, done| {
            edit(&store, id, |task| task.done = done)
        });
        let store = self.store.clone();
        let title = InlineText::controlled(task.title.clone(), move |_, title| {
            let title = title.trim();
            if title.is_empty() {
                return Ok(()); // refused: an empty title is not a title
            }
            edit(&store, id, |task| task.title = title.to_owned())
        })
        .in_row(true);
        Some(Row::new([check.key("done"), title.key("title").with_grow(1)]).key("row"))
    }
    fn text(&self, index: usize) -> Option<String> {
        Some(self.tasks.get(index)?.title.clone())
    }
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

/// Applies a drop to the records: the dropped tasks move, in their order, to
/// the position, which names a key of the rows shown.
fn reorder(store: &Signal<Vec<Task>>, drop: &ListDrop<'_>) -> Result<bool, RuntimeError> {
    let position = match drop.position() {
        ListDropPosition::Before(key) => Place::Before(key.to_owned()),
        ListDropPosition::After(key) => Place::After(key.to_owned()),
        ListDropPosition::End => Place::End,
        _ => return Ok(false),
    };
    place(store, drop.items(), position)
}

enum Place {
    Before(String),
    After(String),
    End,
}

fn place(store: &Signal<Vec<Task>>, keys: &[String], at: Place) -> Result<bool, RuntimeError> {
    let (mut moved, mut rest): (Vec<Task>, Vec<Task>) = store
        .get()?
        .into_iter()
        .partition(|task| keys.contains(&task.id.to_string()));
    let find = |key: &str| rest.iter().position(|task| task.id.to_string() == key);
    let index = match at {
        Place::Before(key) => find(&key),
        Place::After(key) => find(&key).map(|index| index + 1),
        Place::End => Some(rest.len()),
    };
    let Some(index) = index else { return Ok(false) };
    rest.splice(index..index, mem::take(&mut moved));
    store.set(rest)?;
    Ok(true)
}

/// Moves the active task one shown row up (`-1`) or down (`1`), relative to
/// the rows the filter shows.
fn move_active(
    store: &Signal<Vec<Task>>,
    shown: &ReadSignal<ListSource>,
    active: &ReadSignal<Option<String>>,
    delta: isize,
) -> Result<(), RuntimeError> {
    let Some(key) = active.get()? else { return Ok(()) };
    let rows = shown.get()?;
    let Some(index) = rows.index_of(&key) else { return Ok(()) };
    let Some(neighbour) = index.checked_add_signed(delta).and_then(|i| rows.key(i)) else {
        return Ok(());
    };
    let at = if delta < 0 { Place::Before(neighbour) } else { Place::After(neighbour) };
    place(store, &[key], at).map(|_| ())
}

/// Adds a task with `title` unless it is blank; returns whether it was added.
fn add(store: &Signal<Vec<Task>>, next_id: &Cell<u64>, title: &str) -> Result<bool, RuntimeError> {
    let title = title.trim();
    if title.is_empty() {
        return Ok(false);
    }
    let id = next_id.get();
    next_id.set(id + 1);
    store.update(|tasks| tasks.push(Task { id, title: title.to_owned(), done: false }))?;
    Ok(true)
}

fn remove_active(
    store: &Signal<Vec<Task>>,
    active: &ReadSignal<Option<String>>,
    shown: &ReadSignal<bool>,
) -> Result<(), RuntimeError> {
    if !shown.get()? {
        return Ok(());
    }
    let Some(key) = active.get()? else { return Ok(()) };
    store.update(|tasks| tasks.retain(|task| task.id.to_string() != key))
}

/// What a test or the application reaches after building the content.
pub struct TodoApp {
    pub tasks: Signal<Vec<Task>>,
    pub root: WidgetId,
    pub selection: ListSelection,
    pub filter: ListFilter,
    pub status: ReadSignal<String>,
}

/// Key paths below [`TodoApp::root`].
pub mod paths {
    pub const NEW_FIELD: &[&str] = &["entry", "title"];
    pub const ADD: &[&str] = &["entry", "add"];
    pub const LIST: &[&str] = &["tasks", "list"];
    pub const SEARCH: &[&str] = &["tasks", "filter-field", "filter"];
    pub const UP: &[&str] = &["actions", "up"];
    pub const DOWN: &[&str] = &["actions", "down"];
    pub const REMOVE: &[&str] = &["actions", "remove"];
    pub const STATUS: &[&str] = &["status"];
}

/// Builds the to-do list on `ui` with `initial` tasks and registers its shortcuts.
pub fn build(ui: &mut Runtime, initial: Vec<Task>) -> Result<TodoApp, RuntimeError> {
    let next_id = Rc::new(Cell::new(initial.iter().map(|t| t.id + 1).max().unwrap_or(1)));
    let tasks = ui.signal(initial);
    let store = tasks.clone();
    let source = tasks.map(move |all| {
        ListSource::new(Rows { tasks: all.clone(), store: store.clone() })
    })?;
    let selection = ListSelection::new(ui, SelectionMode::Single);
    let filter = ListFilter::new(ui, source)?.selection(selection.clone());
    let active = selection.active();
    let selected_shown = filter.items().combine(&active, |rows, active| {
        active.as_deref().is_some_and(|key| rows.index_of(key).is_some())
    })?;
    let status = tasks.map(|tasks| {
        let done = tasks.iter().filter(|task| task.done).count();
        format!("{done} of {} done", tasks.len())
    })?;

    // New-task entry: Enter in the field or the Add button.
    let new_title = ui.signal(String::new());
    let (store, ids, clear) = (tasks.clone(), next_id.clone(), new_title.clone());
    let field = TextField::new(&new_title)
        .placeholder("New task")
        .on_submit(move |_, text| {
            if add(&store, &ids, text)? {
                clear.set(String::new())?;
            }
            Ok(())
        });
    let (store, ids, typed) = (tasks.clone(), next_id.clone(), new_title.clone());
    let add_button = Button::new("Add", move |_| {
        if add(&store, &ids, &typed.get()?)? {
            typed.set(String::new())?;
        }
        Ok(())
    });

    let order = tasks.clone();
    let list = VirtualList::new(filter.items(), 32.0)?
        .selection(selection.clone())
        // Enter renames the active row: its title is found below the list by key.
        .on_activate(|cx, key| {
            let (list, key) = (cx.id(), key.to_owned());
            let _receipt = cx.defer(move |ui| {
                let Some(title) = ui.find(&list, &[key.as_str(), "title"])? else {
                    return Ok(());
                };
                ui.text_command(&title, TextCommand::BeginEdit).map(|_| ())
            })?;
            Ok(())
        })
        // Drags and Alt+Up/Down on the focused list.
        .reorderable(move |_, drop| reorder(&order, drop));

    let button_move = |delta| {
        let (store, shown, active) = (tasks.clone(), filter.items(), selection.active());
        Button::new(if delta < 0 { "Move up" } else { "Move down" }, move |_| {
            move_active(&store, &shown, &active, delta)
        })
        .enabled(&selected_shown)
    };
    let (store, active, shown) = (tasks.clone(), selection.active(), selected_shown.clone());
    let remove = Button::new("Remove", move |_| remove_active(&store, &active, &shown))
        .enabled(&selected_shown);

    let root = ui.mount_tree(
        Column::new([
            Row::new([field.key("title").with_grow(1), add_button.key("add")]).key("entry"),
            CollectionFilter::new(ui, filter.clone(), Element::new("list", list))
                .placeholder("Search")
                .hidden_active(HiddenActive::Clear)
                .key("tasks")
                .with_grow(1),
            Row::new([
                button_move(-1).key("up"),
                button_move(1).key("down"),
                remove.key("remove"),
            ])
            .key("actions"),
            Label::new(status.clone()).key("status"),
        ])
        .key("app"),
    )?;

    // Shortcuts.
    let new_field = ui.find(&root, paths::NEW_FIELD)?.ok_or(RuntimeError::Unmounted)?;
    ui.register_command("New task", CommandScope::Application, vec![KeyChord::primary('n')],
        move |ui, _| ui.focus(&new_field))?;
    let (store, active, shown) = (tasks.clone(), selection.active(), selected_shown.clone());
    ui.register_command("Remove task", CommandScope::Application,
        vec![KeyChord::named(KeyCode::Delete)],
        move |_, _| remove_active(&store, &active, &shown))?;
    let alt = Modifiers { alt: true, ..Modifiers::default() };
    for (name, key, delta) in [
        ("Move task up", KeyCode::ArrowUp, -1),
        ("Move task down", KeyCode::ArrowDown, 1),
    ] {
        let (store, rows, active) = (tasks.clone(), filter.items(), selection.active());
        ui.register_command(name, CommandScope::Application,
            vec![KeyChord::new(CommandKey::Named(key), alt)],
            move |_, _| move_active(&store, &rows, &active, delta))?;
    }

    Ok(TodoApp { tasks, root, selection, filter, status })
}

/// The desktop application: a window showing [`build`] over [`sample_tasks`].
/// `on_built` receives the handles once the content exists, for a headless test.
pub fn app(on_built: impl FnOnce(TodoApp) + 'static) -> Result<App, RuntimeError> {
    App::new("Tasks", move |ui| {
        on_built(build(ui, sample_tasks())?);
        Ok(())
    })
    .size(Size::from([480, 560]))
}

/// The tasks the application starts with.
pub fn sample_tasks() -> Vec<Task> {
    ["Buy milk", "Write the plan", "Ship it"]
        .into_iter()
        .zip(1..)
        .map(|(title, id)| Task { id, title: title.to_owned(), done: false })
        .collect()
}
