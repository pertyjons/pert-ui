//! A small to-do application built on pert-ui.
//!
//! The application keeps its tasks in one `Signal<Vec<Task>>`; the list, the
//! status line and every action read and write that vector.

use pert_ui::prelude::*;
use std::cell::Cell;
use std::rc::Rc;

#[derive(Clone, Debug, PartialEq)]
pub struct Task {
    pub id: u64,
    pub title: String,
    pub done: bool,
}

/// The records as rows: a copy for reading, and the signal to write back to.
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
                return Ok(());
            }
            edit(&store, id, |task| task.title = title.to_owned())
        })
        .in_row(true);
        Some(
            Row::new([check.key("done"), title.key("title").with_grow(1)]).key("row"),
        )
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

/// Where moved tasks go, by the key of a task that stays.
enum Place {
    Before(String),
    After(String),
    End,
}

/// Moves the tasks named by `keys`, in their order, to `place`.
fn move_keys(store: &Signal<Vec<Task>>, keys: &[String], place: Place) -> Result<bool, RuntimeError> {
    let (moved, mut rest): (Vec<Task>, Vec<Task>) = store
        .get()?
        .into_iter()
        .partition(|task| keys.contains(&task.id.to_string()));
    let at = |key: &str| rest.iter().position(|task| task.id.to_string() == key);
    let index = match place {
        Place::Before(key) => at(&key),
        Place::After(key) => at(&key).map(|index| index + 1),
        Place::End => Some(rest.len()),
    };
    let Some(index) = index else { return Ok(false) };
    rest.splice(index..index, moved);
    store.set(rest)?;
    Ok(true)
}

/// The state the actions read and write.
#[derive(Clone)]
pub struct Model {
    pub tasks: Signal<Vec<Task>>,
    pub new_title: Signal<String>,
    pub selection: ListSelection,
    pub filter: ListFilter,
    pub status: ReadSignal<String>,
    pub selected_shown: ReadSignal<bool>,
    next_id: Rc<Cell<u64>>,
}

/// Everything the native runner and the tests reach: the model and the
/// mounted widgets.
#[derive(Clone)]
pub struct TaskList {
    pub model: Model,
    pub root: WidgetId,
    pub new_field: WidgetId,
    pub list: WidgetId,
}

impl std::ops::Deref for TaskList {
    type Target = Model;
    fn deref(&self) -> &Model {
        &self.model
    }
}

impl Model {
    /// Adds a task with `title`; empty titles are refused (returns false).
    fn add(&self, title: &str) -> Result<bool, RuntimeError> {
        let title = title.trim();
        if title.is_empty() {
            return Ok(false);
        }
        let id = self.next_id.get();
        self.next_id.set(id + 1);
        self.tasks.update(|tasks| {
            tasks.push(Task { id, title: title.to_owned(), done: false })
        })?;
        self.new_title.set(String::new())?;
        Ok(true)
    }

    /// The active task's key, when that task is shown by the filter.
    fn shown_active(&self) -> Result<Option<String>, RuntimeError> {
        if !self.selected_shown.get()? {
            return Ok(None);
        }
        self.selection.active().get()
    }

    fn remove_selected(&self) -> Result<(), RuntimeError> {
        let Some(key) = self.shown_active()? else { return Ok(()) };
        self.tasks.update(|tasks| tasks.retain(|task| task.id.to_string() != key))
    }

    /// Moves the selected task one shown row up (`-1`) or down (`+1`).
    fn move_selected(&self, up: bool) -> Result<(), RuntimeError> {
        let Some(key) = self.shown_active()? else { return Ok(()) };
        let rows = self.filter.items().get()?;
        let Some(index) = rows.index_of(&key) else { return Ok(()) };
        let place = if up {
            match index.checked_sub(1).and_then(|i| rows.key(i)) {
                Some(before) => Place::Before(before),
                None => return Ok(()),
            }
        } else {
            match rows.key(index + 1) {
                Some(after) => Place::After(after),
                None => return Ok(()),
            }
        };
        move_keys(&self.tasks, &[key], place).map(|_| ())
    }
}

/// Builds the application's content on `ui` from the initial tasks.
pub fn build(ui: &mut Runtime, initial: Vec<Task>) -> Result<TaskList, RuntimeError> {
    let next_id = Rc::new(Cell::new(initial.iter().map(|t| t.id).max().unwrap_or(0) + 1));
    let tasks = ui.signal(initial);
    let new_title = ui.signal(String::new());

    let store = tasks.clone();
    let source = tasks.map(move |all| {
        ListSource::new(Rows { tasks: all.clone(), store: store.clone() })
    })?;
    let selection = ListSelection::new(ui, SelectionMode::Single);
    let filter = ListFilter::new(ui, source)?.selection(selection.clone());
    let selected_shown =
        ReadSignal::combine((filter.items(), selection.active()), |(rows, active)| {
            active.as_deref().is_some_and(|key| rows.index_of(key).is_some())
        })?;
    let status = tasks.map(|tasks| {
        let done = tasks.iter().filter(|task| task.done).count();
        format!("{done} of {} done", tasks.len())
    })?;

    let model = Model {
        tasks: tasks.clone(),
        new_title: new_title.clone(),
        selection: selection.clone(),
        filter: filter.clone(),
        status: status.clone(),
        selected_shown: selected_shown.clone(),
        next_id,
    };

    let order = tasks.clone();
    let list = VirtualList::new(filter.items(), 32.0)?
        .selection(selection.clone())
        .on_activate(|cx, key| {
            let (list, key) = (cx.id(), key.to_owned());
            cx.defer(move |ui| {
                let Some(title) = ui.find(&list, &[key.as_str(), "title"])? else {
                    return Ok(());
                };
                ui.text_command(&title, TextCommand::BeginEdit).map(|_| ())
            })?;
            Ok(())
        })
        .reorderable(move |_, drop| {
            let place = match drop.position() {
                ListDropPosition::Before(key) => Place::Before(key.to_owned()),
                ListDropPosition::After(key) => Place::After(key.to_owned()),
                ListDropPosition::End => Place::End,
                _ => return Ok(false),
            };
            move_keys(&order, drop.items(), place)
        });

    let m = model.clone();
    let field = TextField::new(new_title.clone())
        .placeholder("New task")
        .on_submit(move |_, text| m.add(text).map(|_| ()));
    let m = model.clone();
    let add = Button::new("Add", move |_| m.add(&m.new_title.get()?).map(|_| ()));
    let m = model.clone();
    let up = Button::new("Move up", move |_| m.move_selected(true)).enabled(&selected_shown);
    let m = model.clone();
    let down = Button::new("Move down", move |_| m.move_selected(false)).enabled(&selected_shown);
    let m = model.clone();
    let remove = Button::new("Remove", move |_| m.remove_selected()).enabled(&selected_shown);

    let root = ui.mount_tree(
        Column::new([
            Row::new([field.key("new").with_grow(1), add.key("add")]).key("entry"),
            CollectionFilter::new(ui, filter.clone(), Element::new("list", list))
                .placeholder("Search")
                .hidden_active(HiddenActive::Clear)
                .key("tasks")
                .with_grow(1),
            Row::new([up.key("up"), down.key("down"), remove.key("remove")]).key("actions"),
            Label::new(status.clone()).key("status"),
        ])
        .key("app"),
    )?;
    let new_field = find(ui, &root, &["entry", "new"])?;
    let list = find(ui, &root, &["tasks", "list"])?;

    let state = TaskList { model, root, new_field, list };

    // Keyboard shortcuts.
    let focus_new = state.new_field.clone();
    ui.register_command("New task", CommandScope::Application, vec![KeyChord::primary('n')],
        move |ui, _| ui.focus(&focus_new))?;
    let s = state.model.clone();
    ui.register_command("Remove task", CommandScope::Application,
        vec![KeyChord::named(KeyCode::Delete)], move |_, _| s.remove_selected())?;
    let alt = Modifiers { alt: true, ..Modifiers::default() };
    let s = state.model.clone();
    ui.register_command("Move task up", CommandScope::Application,
        vec![KeyChord::new(CommandKey::Named(KeyCode::ArrowUp), alt)],
        move |_, _| s.move_selected(true))?;
    let s = state.model.clone();
    ui.register_command("Move task down", CommandScope::Application,
        vec![KeyChord::new(CommandKey::Named(KeyCode::ArrowDown), alt)],
        move |_, _| s.move_selected(false))?;

    Ok(state)
}

/// A component this module mounted itself; missing, it is a bug in `build`.
fn find(ui: &Runtime, root: &WidgetId, path: &[&str]) -> Result<WidgetId, RuntimeError> {
    let missing = || {
        let message: Box<dyn std::error::Error + Send + Sync> =
            format!("no component at {path:?}").into();
        RuntimeError::Application(ApplicationError::from(message))
    };
    ui.find(root, path)?.ok_or_else(missing)
}

/// The initial tasks of a fresh application.
pub fn sample_tasks() -> Vec<Task> {
    let task = |id, title: &str| Task { id, title: title.to_owned(), done: false };
    vec![task(1, "Buy milk"), task(2, "Write the plan"), task(3, "Ship it")]
}

/// The application, for `run` or `headless`.
pub fn app() -> App<TaskList> {
    App::new("Tasks", |ui| Ok(build(ui, sample_tasks())?))
}
