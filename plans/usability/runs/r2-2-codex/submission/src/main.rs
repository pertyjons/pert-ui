use pert_ui::prelude::*;
use pert_ui::{CommandKey, CommandScope, KeyChord, KeyCode, Modifiers};

#[derive(Clone, Debug, PartialEq)]
struct Task {
    id: u64,
    title: String,
    done: bool,
}

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
    fn text(&self, index: usize) -> Option<String> {
        Some(self.tasks.get(index)?.title.clone())
    }
    fn build(&self, index: usize) -> Option<Element> {
        let task = self.tasks.get(index)?;
        let id = task.id;
        let store = self.store.clone();
        let check = Checkbox::controlled(task.done, "Done", move |_, done| {
            edit(&store, id, |task| task.done = done)
        });
        let store = self.store.clone();
        let title = InlineText::controlled(task.title.clone(), move |_, title| {
            edit(&store, id, |task| task.title = title.to_owned())
        })
        .in_row(true)
        .normalize(|text| {
            let text = text.trim();
            if text.is_empty() {
                Err(InputRejection::new("Enter a title"))
            } else {
                Ok(text.to_owned())
            }
        });
        Some(Row::new([check.key("done"), title.key("title").with_grow(1)]).key("row"))
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

fn reorder(store: &Signal<Vec<Task>>, drop: &ListDrop<'_>) -> Result<bool, RuntimeError> {
    let (moved, mut rest): (Vec<_>, Vec<_>) = store
        .get()?
        .into_iter()
        .partition(|task| drop.items().contains(&task.id.to_string()));
    let at = |key: &str| rest.iter().position(|task| task.id.to_string() == key);
    let index = match drop.position() {
        ListDropPosition::Before(key) => at(key),
        ListDropPosition::After(key) => at(key).map(|i| i + 1),
        ListDropPosition::End => Some(rest.len()),
        _ => None,
    };
    let Some(index) = index else { return Ok(false) };
    rest.splice(index..index, moved);
    store.set(rest)?;
    Ok(true)
}

#[allow(dead_code)]
struct TaskList {
    tasks: Signal<Vec<Task>>,
    root: WidgetId,
    selection: ListSelection,
    filter: ListFilter,
}

#[derive(Clone)]
struct Actions {
    tasks: Signal<Vec<Task>>,
    selection: ListSelection,
    filter: ListFilter,
}
impl Actions {
    fn remove(&self) -> Result<(), RuntimeError> {
        let Some(key) = self.selection.active().get()? else {
            return Ok(());
        };
        let rows = self.filter.items().get()?;
        let Some(index) = rows.index_of(&key) else {
            return Ok(());
        };
        let neighbor = rows
            .key(index + 1)
            .or_else(|| index.checked_sub(1).and_then(|i| rows.key(i)));
        self.tasks
            .update(|tasks| tasks.retain(|task| task.id.to_string() != key))?;
        self.selection.set_active(neighbor)
    }
    fn shift(&self, down: bool) -> Result<(), RuntimeError> {
        let Some(key) = self.selection.active().get()? else {
            return Ok(());
        };
        let rows = self.filter.items().get()?;
        let Some(index) = rows.index_of(&key) else {
            return Ok(());
        };
        let neighbor = if down {
            index.checked_add(1)
        } else {
            index.checked_sub(1)
        };
        let Some(other) = neighbor.and_then(|i| rows.key(i)) else {
            return Ok(());
        };
        self.tasks.update(|tasks| {
            let a = tasks.iter().position(|t| t.id.to_string() == key);
            let b = tasks.iter().position(|t| t.id.to_string() == other);
            if let (Some(a), Some(b)) = (a, b) {
                let task = tasks.remove(a);
                tasks.insert(b, task);
            }
        })
    }
}

fn add(
    tasks: &Signal<Vec<Task>>,
    draft: &Signal<String>,
    next: &Signal<u64>,
) -> Result<(), RuntimeError> {
    let title = draft.get()?.trim().to_owned();
    if title.is_empty() {
        return Ok(());
    }
    let id = next.get()?;
    tasks.update(|tasks| {
        tasks.push(Task {
            id,
            title,
            done: false,
        })
    })?;
    next.set(id + 1)?;
    draft.set(String::new())
}

fn build(ui: &mut Runtime) -> Result<TaskList, RuntimeError> {
    let tasks = ui.signal(Vec::<Task>::new());
    let draft = ui.signal(String::new());
    let next = ui.signal(1_u64);
    let store = tasks.clone();
    let source = tasks.map(move |all| {
        ListSource::new(Rows {
            tasks: all.clone(),
            store: store.clone(),
        })
    })?;
    let selection = ListSelection::new(ui, SelectionMode::Single);
    let filter = ListFilter::new(ui, source)?.selection(selection.clone());
    let shown = filter
        .items()
        .combine(&selection.active(), |rows, active| {
            active
                .as_deref()
                .is_some_and(|key| rows.index_of(key).is_some())
        })?;
    let actions = Actions {
        tasks: tasks.clone(),
        selection: selection.clone(),
        filter: filter.clone(),
    };
    let order = tasks.clone();
    let list = VirtualList::new(filter.items(), 36.0)?
        .selection(selection.clone())
        .on_activate(|cx, key| {
            let (list, key) = (cx.id(), key.to_owned());
            let _receipt = cx.defer(move |ui| {
                if let Some(title) = ui.find(&list, &[&key, "title"])? {
                    ui.text_command(&title, TextCommand::BeginEdit)?;
                }
                Ok(())
            })?;
            Ok(())
        })
        .reorderable(move |_, drop| reorder(&order, drop));
    let (t, d, n) = (tasks.clone(), draft.clone(), next.clone());
    let field = TextField::new(&draft)
        .placeholder("New task (Ctrl+N)")
        .on_submit(move |_, _| add(&t, &d, &n));
    let (t, d, n) = (tasks.clone(), draft, next);
    let add_button = Button::new("Add", move |_| add(&t, &d, &n));
    let remove = actions.clone();
    let up = actions.clone();
    let down = actions.clone();
    let status = tasks.map(|all| {
        format!(
            "{} of {} done",
            all.iter().filter(|t| t.done).count(),
            all.len()
        )
    })?;
    let root = ui.mount_tree(
        Column::new([
            Label::new("Tasks").key("heading"),
            Row::new([field.key("new").with_grow(1), add_button.key("add")]).key("entry"),
            CollectionFilter::new(ui, filter.clone(), list.key("list"))
                .hidden_active(HiddenActive::Clear)
                .key("tasks")
                .with_grow(1),
            Row::new([
                Button::new("Remove (Delete)", move |_| remove.remove())
                    .enabled(&shown)
                    .key("remove"),
                Button::new("Move up (Alt+Up)", move |_| up.shift(false))
                    .enabled(&shown)
                    .key("up"),
                Button::new("Move down (Alt+Down)", move |_| down.shift(true))
                    .enabled(&shown)
                    .key("down"),
            ])
            .key("actions"),
            Label::new(status).key("status"),
            Label::new("Select a task; Enter or double-click its title to rename. Escape cancels.")
                .key("help"),
        ])
        .align(Align::Stretch)
        .key("app"),
    )?;
    let new_field = ui
        .find(&root, &["entry", "new"])?
        .expect("mounted new task field");
    ui.register_command(
        "New task",
        CommandScope::Application,
        vec![KeyChord::new(
            CommandKey::Character('n'),
            Modifiers {
                control: true,
                ..Modifiers::default()
            },
        )],
        move |ui, _| ui.focus(&new_field),
    )?;
    let remove = actions.clone();
    ui.register_command(
        "Remove task",
        CommandScope::Application,
        vec![KeyChord::named(KeyCode::Delete)],
        move |_, _| remove.remove(),
    )?;
    let up = actions.clone();
    ui.register_command(
        "Move task up",
        CommandScope::Application,
        vec![alt_arrow(false)],
        move |_, _| up.shift(false),
    )?;
    ui.register_command(
        "Move task down",
        CommandScope::Application,
        vec![alt_arrow(true)],
        move |_, _| actions.shift(true),
    )?;
    Ok(TaskList {
        tasks,
        root,
        selection,
        filter,
    })
}

fn alt_arrow(down: bool) -> KeyChord {
    KeyChord::new(
        CommandKey::Named(if down {
            KeyCode::ArrowDown
        } else {
            KeyCode::ArrowUp
        }),
        Modifiers {
            alt: true,
            ..Modifiers::default()
        },
    )
}

fn main() -> Result<(), AppError> {
    App::new("Task list", |ui| {
        build(ui)?;
        Ok(())
    })
    .run()
}

#[cfg(test)]
mod tests {
    use super::*;
    use pert_ui::testing::*;
    type TestResult = Result<(), Box<dyn std::error::Error>>;

    struct Harness {
        ui: Runtime,
        app: TaskList,
    }
    impl Harness {
        fn new() -> Result<Self, Box<dyn std::error::Error>> {
            let mut ui = Runtime::new();
            ui.set_viewport(Viewport::new(960.0, 640.0, 1.0)?)?;
            let app = build(&mut ui)?;
            Driver::new(&mut ui).settle()?;
            Ok(Self { ui, app })
        }
        fn find(&self, path: &[&str]) -> WidgetId {
            self.ui
                .find(&self.app.root, path)
                .unwrap()
                .unwrap_or_else(|| panic!("missing {path:?}"))
        }
        fn click(&mut self, path: &[&str]) -> TestResult {
            let widget = self.find(path);
            Driver::new(&mut self.ui).click(&widget)?;
            Ok(())
        }
        fn replace(&mut self, path: &[&str], text: &str) -> TestResult {
            let widget = self.find(path);
            Driver::new(&mut self.ui).replace_text(&widget, text)?;
            Ok(())
        }
        fn key(&mut self, key: KeyCode) -> TestResult {
            Driver::new(&mut self.ui).key(key)?;
            Ok(())
        }
        fn chord(&mut self, chord: KeyChord, key: KeyCode) -> TestResult {
            Driver::new(&mut self.ui).chord(chord, key)?;
            Ok(())
        }
        fn add(&mut self, text: &str, button: bool) -> TestResult {
            self.replace(&["entry", "new"], text)?;
            if button {
                self.click(&["entry", "add"])?;
            } else {
                self.key(KeyCode::Enter)?;
            }
            Ok(())
        }
        fn titles(&self) -> Vec<String> {
            self.app
                .tasks
                .get()
                .unwrap()
                .into_iter()
                .map(|task| task.title)
                .collect()
        }
        fn status(&mut self, expected: &str) -> TestResult {
            let status = self.find(&["status"]);
            assert_eq!(
                Driver::new(&mut self.ui)
                    .semantics(&status)?
                    .name
                    .as_deref(),
                Some(expected)
            );
            Ok(())
        }
        fn displayed(&mut self) -> Result<Vec<String>, Box<dyn std::error::Error>> {
            let list = self.find(&["tasks", "list"]);
            let rows = self.ui.children(&list)?;
            let mut titles = Vec::new();
            for row in rows {
                let title = self.ui.find(&row, &["title"])?.expect("title");
                titles.push(
                    Driver::new(&mut self.ui)
                        .semantics(&title)?
                        .value
                        .map(|value| match value {
                            SemanticValue::Text(text) => text,
                            other => panic!("unexpected {other:?}"),
                        })
                        .unwrap_or_default(),
                );
            }
            Ok(titles)
        }
    }

    #[test]
    fn add_check_remove_and_buttons() -> TestResult {
        let mut h = Harness::new()?;
        h.status("0 of 0 done")?;
        h.add("   ", false)?;
        h.add("", true)?;
        assert!(h.titles().is_empty());
        h.add("  Buy milk  ", false)?;
        h.add("Write plan", true)?;
        h.add("Ship it", false)?;
        assert_eq!(h.titles(), ["Buy milk", "Write plan", "Ship it"]);
        h.status("0 of 3 done")?;
        assert_eq!(h.displayed()?, h.titles());
        h.click(&["tasks", "list", "1", "done"])?;
        assert!(h.app.tasks.get()?[0].done);
        h.status("1 of 3 done")?;
        h.click(&["tasks", "list", "2", "title"])?;
        assert_eq!(h.app.selection.active().get()?.as_deref(), Some("2"));
        h.click(&["actions", "up"])?;
        assert_eq!(h.titles(), ["Write plan", "Buy milk", "Ship it"]);
        assert_eq!(h.displayed()?, h.titles());
        h.status("1 of 3 done")?;
        h.click(&["actions", "down"])?;
        assert_eq!(h.titles(), ["Buy milk", "Write plan", "Ship it"]);
        h.click(&["tasks", "list", "1", "title"])?;
        h.click(&["actions", "remove"])?;
        assert_eq!(h.titles(), ["Write plan", "Ship it"]);
        h.status("0 of 2 done")?;
        Ok(())
    }

    #[test]
    fn rename_enter_escape_and_pointer() -> TestResult {
        let mut h = Harness::new()?;
        h.add("Buy milk", false)?;
        h.add("Ship it", false)?;
        h.click(&["tasks", "list", "1", "title"])?;
        h.key(KeyCode::Enter)?;
        Driver::new(&mut h.ui).type_text("Buy oat milk")?;
        assert_eq!(
            h.titles()[0],
            "Buy milk",
            "draft must not change the vector"
        );
        h.key(KeyCode::Enter)?;
        assert_eq!(h.titles()[0], "Buy oat milk");
        assert_eq!(h.displayed()?, h.titles());
        h.status("0 of 2 done")?;
        h.key(KeyCode::Enter)?;
        Driver::new(&mut h.ui).type_text("Discard me")?;
        h.key(KeyCode::Escape)?;
        assert_eq!(h.titles()[0], "Buy oat milk");
        let list = h.find(&["tasks", "list"]);
        assert_eq!(h.ui.focused(), Some(list));
        h.click(&["tasks", "list", "2", "title"])?;
        h.click(&["tasks", "list", "2", "title"])?;
        Driver::new(&mut h.ui).type_text("Ship today")?;
        h.key(KeyCode::Enter)?;
        assert_eq!(h.titles()[1], "Ship today");
        Ok(())
    }

    #[test]
    fn filtering_selection_arrows_and_shortcuts() -> TestResult {
        let mut h = Harness::new()?;
        let ctrl_n = KeyChord::new(
            CommandKey::Character('n'),
            Modifiers {
                control: true,
                ..Modifiers::default()
            },
        );
        h.chord(ctrl_n, KeyCode::KeyN)?;
        assert_eq!(h.ui.focused(), Some(h.find(&["entry", "new"])));
        Driver::new(&mut h.ui).type_text("Buy milk")?;
        h.key(KeyCode::Enter)?;
        h.add("Write plan", true)?;
        h.add("Milk chocolate", false)?;
        h.click(&["tasks", "list", "1", "title"])?;
        h.key(KeyCode::ArrowDown)?;
        assert_eq!(h.app.selection.active().get()?.as_deref(), Some("2"));
        h.chord(alt_arrow(false), KeyCode::ArrowUp)?;
        assert_eq!(h.titles(), ["Write plan", "Buy milk", "Milk chocolate"]);
        h.chord(alt_arrow(true), KeyCode::ArrowDown)?;
        assert_eq!(h.titles(), ["Buy milk", "Write plan", "Milk chocolate"]);
        h.status("0 of 3 done")?;
        let before = h.app.tasks.get()?;
        h.replace(&["tasks", "filter-field", "filter"], "mILk")?;
        assert_eq!(h.app.filter.counts().get()?.shown, 2);
        assert_eq!(h.app.selection.active().get()?, None);
        assert_eq!(h.app.tasks.get()?, before);
        assert_eq!(h.displayed()?, ["Buy milk", "Milk chocolate"]);
        assert!(h.ui.find(&h.app.root, &["tasks", "list", "2"])?.is_none());
        h.status("0 of 3 done")?;
        // A hidden selection cannot be removed by a toolbar action.
        h.click(&["actions", "remove"])?;
        assert_eq!(h.app.tasks.get()?, before);
        // Pointer-select a visible row, then navigate only the filtered rows.
        h.click(&["tasks", "list", "1", "title"])?;
        h.key(KeyCode::ArrowDown)?;
        assert_eq!(h.app.selection.active().get()?.as_deref(), Some("3"));
        h.key(KeyCode::Delete)?;
        assert_eq!(h.titles(), ["Buy milk", "Write plan"]);
        h.status("0 of 2 done")?;
        assert_eq!(h.app.selection.active().get()?.as_deref(), Some("1"));
        h.replace(&["tasks", "filter-field", "filter"], "no match")?;
        assert_eq!(h.app.filter.counts().get()?.shown, 0);
        assert_eq!(h.app.selection.active().get()?, None);
        h.replace(&["tasks", "filter-field", "filter"], "")?;
        assert_eq!(h.app.filter.counts().get()?.shown, 2);
        h.chord(ctrl_n, KeyCode::KeyN)?;
        assert_eq!(h.ui.focused(), Some(h.find(&["entry", "new"])));
        Ok(())
    }
}
