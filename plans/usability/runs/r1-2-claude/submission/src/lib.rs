//! A small to-do list on pert-ui.
//!
//! The application's state is one `Signal<Vec<Task>>`; every widget shows what
//! that vector holds and every edit goes through it. `build` mounts the whole
//! UI on a `Runtime`, so the native `App` and the headless tests share it.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use pert_ui::prelude::*;
use pert_ui::{
    CommandKey, CommandScope, DropPosition, FilterMatcher, KeyChord, ListDrop, Modifiers,
};

#[derive(Clone, Debug, PartialEq)]
pub struct Task {
    pub id: u64,
    pub title: String,
    pub done: bool,
}

/// The list key of a task.
pub fn task_key(id: u64) -> String {
    format!("t{id}")
}

fn id_of(key: &str) -> Option<u64> {
    key.strip_prefix('t')?.parse().ok()
}

/// "N of M done".
pub fn status_text(tasks: &[Task]) -> String {
    let done = tasks.iter().filter(|t| t.done).count();
    format!("{done} of {} done", tasks.len())
}

fn normalize_title(text: String) -> Result<String, InputRejection> {
    let text = text.trim().to_owned();
    if text.is_empty() {
        Err(InputRejection::new("A task needs a title"))
    } else {
        Ok(text)
    }
}

/// The rows of the list: a snapshot of the vector, plus the vector's signal so
/// that row controls can write their edits back into it.
struct TaskItems {
    tasks: Rc<Vec<Task>>,
    model: Signal<Vec<Task>>,
}

impl ListItems for TaskItems {
    fn len(&self) -> usize {
        self.tasks.len()
    }

    fn key(&self, index: usize) -> Option<String> {
        self.tasks.get(index).map(|t| task_key(t.id))
    }

    fn build(&self, index: usize) -> Option<Element> {
        let task = self.tasks.get(index)?;
        let id = task.id;
        // Row controls bind signals; these are views of the vector's values,
        // rebuilt whenever the vector changes, and their callbacks write the
        // vector.
        let done = self.model.sibling(task.done);
        let title = self.model.sibling(task.title.clone());
        let (on_done, on_title) = (self.model.clone(), self.model.clone());
        let checkbox = Checkbox::new("", done).on_change(move |_, done| {
            on_done.update(|tasks| {
                if let Some(task) = tasks.iter_mut().find(|t| t.id == id) {
                    task.done = done;
                }
            })
        });
        let editor = InlineText::new(title)
            .normalize(normalize_title)
            .on_save(move |_, text| {
                let text = text.to_owned();
                on_title.update(|tasks| {
                    if let Some(task) = tasks.iter_mut().find(|t| t.id == id) {
                        task.title = text;
                    }
                })
            });
        Some(
            Row::new([checkbox.key("done"), editor.key("title")])
                .align(Align::Center)
                .key("row"),
        )
    }

    fn index_of(&self, key: &str) -> Option<usize> {
        let id = id_of(key)?;
        self.tasks.iter().position(|t| t.id == id)
    }

    fn text(&self, index: usize) -> Option<String> {
        self.tasks.get(index).map(|t| t.title.clone())
    }
}

/// Handles to the mounted application, for the native host and for tests.
#[derive(Clone)]
pub struct TodoApp {
    pub tasks: Signal<Vec<Task>>,
    pub new_title: Signal<String>,
    pub query: Signal<String>,
    pub selection: ListSelection,
    pub status: ReadSignal<String>,
    pub root: WidgetId,
    pub new_title_field: WidgetId,
    pub list: WidgetId,
}

/// The edits the UI performs, each going through the vector.
#[derive(Clone)]
struct Actions {
    tasks: Signal<Vec<Task>>,
    new_title: Signal<String>,
    query: Signal<String>,
    selection: ListSelection,
    next_id: Rc<Cell<u64>>,
}

impl Actions {
    /// Keys of the tasks the filter shows, in order.
    fn visible_keys(&self) -> Result<Vec<String>, RuntimeError> {
        let query = self.query.get()?;
        let matcher = FilterMatcher::substring();
        let all = query.trim().is_empty();
        Ok(self
            .tasks
            .get()?
            .iter()
            .filter(|t| all || matcher.matches(&query, &t.title))
            .map(|t| task_key(t.id))
            .collect())
    }

    /// The selected task's key, when it is shown; a task hidden by the filter
    /// is never the target of an edit.
    fn selected_visible(&self) -> Result<Option<String>, RuntimeError> {
        let Some(active) = self.selection.active().get()? else {
            return Ok(None);
        };
        Ok(self.visible_keys()?.contains(&active).then_some(active))
    }

    fn add(&self) -> Result<(), RuntimeError> {
        let Ok(title) = normalize_title(self.new_title.get()?) else {
            return Ok(()); // empty titles are refused
        };
        let id = self.next_id.get();
        self.next_id.set(id + 1);
        self.tasks.update(|tasks| {
            tasks.push(Task {
                id,
                title,
                done: false,
            })
        })?;
        self.new_title.set(String::new())
    }

    fn remove_selected(&self) -> Result<(), RuntimeError> {
        let Some(key) = self.selected_visible()? else {
            return Ok(());
        };
        let visible = self.visible_keys()?;
        let at = visible.iter().position(|k| *k == key).unwrap_or(0);
        // Select the next shown task, or the previous one at the end.
        let next = visible
            .get(at + 1)
            .or_else(|| at.checked_sub(1).and_then(|i| visible.get(i)))
            .cloned();
        let id = id_of(&key);
        self.tasks
            .update(|tasks| tasks.retain(|t| Some(t.id) != id))?;
        self.selection.set_active(next)
    }

    /// Moves the selected task one place among the shown tasks.
    fn move_selected(&self, up: bool) -> Result<(), RuntimeError> {
        let Some(key) = self.selected_visible()? else {
            return Ok(());
        };
        let visible = self.visible_keys()?;
        let at = visible.iter().position(|k| *k == key).unwrap_or(0);
        let position = if up {
            match at.checked_sub(1) {
                Some(i) => DropPosition::Before(visible[i].clone()),
                None => return Ok(()),
            }
        } else {
            match visible.get(at + 1) {
                Some(next) => DropPosition::After(next.clone()),
                None => return Ok(()),
            }
        };
        self.move_keys(&[key], &position)
    }

    /// Moves the tasks with `keys` to `position`, keeping their order.
    fn move_keys(&self, keys: &[String], position: &DropPosition) -> Result<(), RuntimeError> {
        let ids: Vec<u64> = keys.iter().filter_map(|k| id_of(k)).collect();
        let mut tasks = self.tasks.get()?;
        let moved: Vec<Task> = tasks.iter().filter(|t| ids.contains(&t.id)).cloned().collect();
        if moved.is_empty() {
            return Ok(());
        }
        tasks.retain(|t| !ids.contains(&t.id));
        let index_of = |tasks: &[Task], key: &str| {
            let id = id_of(key)?;
            tasks.iter().position(|t| t.id == id)
        };
        let at = match position {
            DropPosition::Before(key) => index_of(&tasks, key),
            DropPosition::After(key) => index_of(&tasks, key).map(|i| i + 1),
            DropPosition::End => Some(tasks.len()),
            _ => None,
        };
        let Some(at) = at else {
            return Ok(()); // the target is one of the moved tasks: nothing to do
        };
        tasks.splice(at..at, moved);
        self.tasks.set(tasks)
    }
}

/// Mounts the to-do list on `ui` with `initial` tasks.
pub fn build(ui: &mut Runtime, initial: Vec<Task>) -> Result<TodoApp, RuntimeError> {
    let next_id = Rc::new(Cell::new(initial.iter().map(|t| t.id + 1).max().unwrap_or(1)));
    let tasks = ui.signal(initial);
    let new_title = ui.signal(String::new());
    let query = ui.signal(String::new());
    let selection = ListSelection::new(ui, SelectionMode::Single);
    let actions = Actions {
        tasks: tasks.clone(),
        new_title: new_title.clone(),
        query: query.clone(),
        selection: selection.clone(),
        next_id,
    };

    let model = tasks.clone();
    let source = tasks.map(move |tasks| {
        ListSource::new(TaskItems {
            tasks: Rc::new(tasks.clone()),
            model: model.clone(),
        })
    })?;
    let filter = ListFilter::new(ui, source)?.selection(selection.clone());
    // Whether the selected task is shown: removing and moving act only then,
    // so a task the filter hides is never edited by surprise.
    let has_target = selection.active().combine(&filter.items(), |active, items| {
        active.as_deref().is_some_and(|key| items.index_of(key).is_some())
    })?;
    let reorder = actions.clone();
    // The list's id, known once it is mounted, for starting a rename by key.
    let list_id: Rc<RefCell<Option<WidgetId>>> = Rc::default();
    let activate_list = list_id.clone();
    let list = VirtualList::new(filter.items(), 32.0)?
        .selection(selection.clone())
        .reorderable(move |_, drop: &ListDrop<'_>| {
            reorder.move_keys(drop.items(), drop.position())?;
            Ok(true)
        })
        // Enter on the selected task starts renaming it.
        .on_activate(move |cx, key| {
            let (list, key) = (activate_list.borrow().clone(), key.to_owned());
            let _ = cx.defer(move |ui| {
                let Some(list) = list else { return Ok(()) };
                let Some(row) = ui.find_descendant(&list, &key)? else {
                    return Ok(());
                };
                let Some(title) = ui.find_descendant(&row, "title")? else {
                    return Ok(());
                };
                ui.focus(&title)?;
                // InlineText offers no call that starts editing, so deliver
                // the F2 it documents as the keyboard start.
                ui.key_input(KeyCode::F2, KeyAction::Press)?;
                ui.key_input(KeyCode::F2, KeyAction::Release)?;
                Ok(())
            })?;
            Ok(())
        });
    let search = CollectionFilter::with_query(query.clone(), filter, list.key("list"))
        .name("Search tasks");

    let status = tasks.map(|tasks| status_text(tasks))?;
    let (on_submit, on_add, on_remove, on_up, on_down) = (
        actions.clone(),
        actions.clone(),
        actions.clone(),
        actions.clone(),
        actions.clone(),
    );
    let entry = Row::new([
        TextField::new(&new_title)
            .placeholder("New task (Ctrl+N)")
            .on_submit(move |_, _| on_submit.add())
            .key("new-title")
            .with_grow(1),
        Button::new("Add", move |_| on_add.add()).key("add"),
    ]);
    let buttons = Row::new([
        Button::new("Remove", move |_| on_remove.remove_selected())
            .enabled(has_target.clone())
            .key("remove"),
        Button::new("Move up", move |_| on_up.move_selected(true))
            .enabled(has_target.clone())
            .key("up"),
        Button::new("Move down", move |_| on_down.move_selected(false))
            .enabled(has_target.clone())
            .key("down"),
    ]);
    let content = Column::new([
        entry.key("entry"),
        search.key("tasks").with_grow(1),
        buttons.key("actions"),
        Label::new(status.clone()).key("status"),
    ])
    .align(Align::Stretch);
    let root = ui.mount_tree(content.key("app"))?;

    let new_title_field = ui
        .find(&root, &["entry", "new-title"])?
        .ok_or(RuntimeError::Unmounted)?;
    let list = ui
        .find_descendant(&root, "list")?
        .ok_or(RuntimeError::Unmounted)?;
    *list_id.borrow_mut() = Some(list.clone());

    let field = new_title_field.clone();
    let _ = ui.register_command(
        "New task",
        CommandScope::Application,
        vec![KeyChord::primary('n')],
        move |ui, _| ui.focus(&field),
    )?;
    let remove = actions.clone();
    let remove_command = ui.register_command(
        "Remove task",
        CommandScope::Application,
        vec![KeyChord::named(KeyCode::Delete)],
        move |_, _| remove.remove_selected(),
    )?;
    let alt = Modifiers {
        alt: true,
        ..Modifiers::default()
    };
    let up = actions.clone();
    let up_command = ui.register_command(
        "Move task up",
        CommandScope::Application,
        vec![KeyChord::new(CommandKey::Named(KeyCode::ArrowUp), alt)],
        move |_, _| up.move_selected(true),
    )?;
    let down = actions.clone();
    let down_command = ui.register_command(
        "Move task down",
        CommandScope::Application,
        vec![KeyChord::new(CommandKey::Named(KeyCode::ArrowDown), alt)],
        move |_, _| down.move_selected(false),
    )?;
    for command in [remove_command, up_command, down_command] {
        ui.command_enabled_signal(&command, has_target.clone())?;
    }

    Ok(TodoApp {
        tasks,
        new_title,
        query,
        selection,
        status,
        root,
        new_title_field,
        list,
    })
}
