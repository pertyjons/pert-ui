use pert_ui::prelude::*;

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
        let (store, id) = (self.store.clone(), task.id);
        let check = Checkbox::controlled(task.done, "Done", move |_, done| {
            edit(&store, id, |task| task.done = done)
        });
        let store = self.store.clone();
        let title = InlineText::controlled(task.title.clone(), move |_, title| {
            edit(&store, id, |task| task.title = title.to_owned())
        })
        .in_row(true);
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
fn add(
    store: &Signal<Vec<Task>>,
    draft: &Signal<String>,
    next: &Signal<u64>,
) -> Result<(), RuntimeError> {
    let title = draft.get()?.trim().to_owned();
    if title.is_empty() {
        return Ok(());
    }
    let id = next.get()?;
    store.update(|tasks| {
        tasks.push(Task {
            id,
            title,
            done: false,
        })
    })?;
    next.set(id + 1)?;
    draft.set(String::new())
}
fn remove(
    store: &Signal<Vec<Task>>,
    selection: &ListSelection,
    shown: &ReadSignal<bool>,
) -> Result<(), RuntimeError> {
    if !shown.get()? {
        return Ok(());
    }
    if let Some(key) = selection.active().get()? {
        store.update(|tasks| tasks.retain(|task| task.id.to_string() != key))?;
    }
    Ok(())
}
fn move_selected(
    store: &Signal<Vec<Task>>,
    selection: &ListSelection,
    shown: &ReadSignal<bool>,
    down: bool,
) -> Result<(), RuntimeError> {
    if !shown.get()? {
        return Ok(());
    }
    let Some(key) = selection.active().get()? else {
        return Ok(());
    };
    store.update(|tasks| {
        if let Some(index) = tasks.iter().position(|task| task.id.to_string() == key) {
            let other = if down {
                index.checked_add(1)
            } else {
                index.checked_sub(1)
            };
            if let Some(other) = other.filter(|&other| other < tasks.len()) {
                tasks.swap(index, other);
            }
        }
    })
}
fn reorder(store: &Signal<Vec<Task>>, drop: &ListDrop<'_>) -> Result<bool, RuntimeError> {
    let (moved, mut rest): (Vec<Task>, Vec<Task>) = store
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
    let Some(index) = index else {
        return Ok(false);
    };
    rest.splice(index..index, moved);
    store.set(rest)?;
    Ok(true)
}

struct TaskList {
    tasks: Signal<Vec<Task>>,
    root: WidgetId,
    selection: ListSelection,
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
    let shown = ReadSignal::combine((filter.items(), selection.active()), |(rows, active)| {
        active
            .as_deref()
            .is_some_and(|key| rows.index_of(key).is_some())
    })?;
    let status = tasks.map(|all| {
        format!(
            "{} of {} done",
            all.iter().filter(|t| t.done).count(),
            all.len()
        )
    })?;
    let order = tasks.clone();
    let list = VirtualList::new(filter.items(), 36.0)?
        .selection(selection.clone())
        .on_activate(|cx, key| {
            let (list, key) = (cx.id(), key.to_owned());
            cx.defer(move |ui| {
                if let Some(title) = ui.find(&list, &[&key, "title"])? {
                    ui.text_command(&title, TextCommand::BeginEdit)?;
                }
                Ok(())
            })?;
            Ok(())
        })
        .reorderable(move |_, drop| reorder(&order, drop));
    let (a, d, n) = (tasks.clone(), draft.clone(), next.clone());
    let field = TextField::new(draft.clone())
        .placeholder("New task — Ctrl+N")
        .on_submit(move |_, _| add(&a, &d, &n));
    let (a, d, n) = (tasks.clone(), draft, next);
    let add_button = Button::new("Add", move |_| add(&a, &d, &n));
    let (a, s, v) = (tasks.clone(), selection.clone(), shown.clone());
    let delete = Button::new("Remove", move |_| remove(&a, &s, &v)).enabled(&shown);
    let (a, s, v) = (tasks.clone(), selection.clone(), shown.clone());
    let up = Button::new("Move up", move |_| move_selected(&a, &s, &v, false)).enabled(&shown);
    let (a, s, v) = (tasks.clone(), selection.clone(), shown.clone());
    let down = Button::new("Move down", move |_| move_selected(&a, &s, &v, true)).enabled(&shown);
    let root = ui.mount_tree(Column::new([
        Label::new("Tasks").key("heading"),
        Row::new([field.key("new").with_grow(1), add_button.key("add")]).key("entry"),
        CollectionFilter::new(ui, filter, list.key("list"))
            .hidden_active(HiddenActive::Clear).placeholder("Search tasks")
            .key("tasks").with_grow(1),
        Row::new([delete.key("remove"), up.key("up"), down.key("down")]).key("actions"),
        Label::new(status).key("status"),
        Label::new("Select with arrows · Enter or double-click to rename · Escape cancels · Alt+↑/↓ moves · Delete removes").key("help"),
    ]).key("app"))?;
    let new_field = ui
        .find(&root, &["entry", "new"])?
        .expect("mounted new-task field");
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
    let (a, s, v) = (tasks.clone(), selection.clone(), shown);
    ui.register_command(
        "Remove selected task",
        CommandScope::Application,
        vec![KeyChord::named(KeyCode::Delete)],
        move |_, _| remove(&a, &s, &v),
    )?;
    Ok(TaskList {
        tasks,
        root,
        selection,
    })
}
fn app() -> App<TaskList> {
    App::new("Task list", |ui| Ok(build(ui)?))
}
fn main() -> Result<(), pert_ui::AppError> {
    app().run()
}

#[cfg(test)]
mod tests {
    use super::*;
    use pert_ui::testing::*;
    type Result = std::result::Result<(), Box<dyn std::error::Error>>;

    fn status(d: &Driver<'_>, root: &WidgetId, expected: &str) -> Result {
        let id = d.find(root, &["status"])?;
        assert_eq!(d.semantics(&id)?.name.as_deref(), Some(expected));
        Ok(())
    }
    fn rows(d: &Driver<'_>, list: &WidgetId) -> std::result::Result<Vec<String>, TestError> {
        Ok(d.address_children(list)?
            .into_iter()
            .map(|row| row.segment)
            .collect())
    }
    fn alt(key: KeyCode) -> KeyChord {
        KeyChord::new(
            CommandKey::Named(key),
            Modifiers {
                alt: true,
                ..Modifiers::default()
            },
        )
    }
    #[test]
    fn full_headless_user_session() -> Result {
        let mut app = app().headless()?;
        let (ui, state) = app.runtime_and_state_mut();
        let mut d = Driver::new(ui);
        d.settle()?;
        let root = &state.root;
        let field = d.find(root, &["entry", "new"])?;
        let add = d.find(root, &["entry", "add"])?;
        let list = d.find(root, &["tasks", "list"])?;
        status(&d, root, "0 of 0 done")?;

        // Whitespace and empty titles are refused through both entry routes.
        d.click(&add)?;
        d.replace_text(&field, "   ")?;
        d.key(KeyCode::Enter)?;
        assert!(state.tasks.get()?.is_empty());
        status(&d, root, "0 of 0 done")?;

        d.replace_text(&field, "  Buy Milk  ")?;
        d.key(KeyCode::Enter)?;
        d.replace_text(&field, "Write plan")?;
        d.click(&add)?;
        d.replace_text(&field, "Ship release")?;
        d.key(KeyCode::Enter)?;
        assert_eq!(rows(&d, &list)?, ["1", "2", "3"]);
        assert_eq!(state.tasks.get()?[0].title, "Buy Milk");
        assert_eq!(
            d.semantics(&field)?.value,
            Some(SemanticValue::Text(String::new()))
        );
        status(&d, root, "0 of 3 done")?;

        let check = d.find(&list, &["1", "done"])?;
        d.click(&check)?;
        assert!(state.tasks.get()?[0].done);
        status(&d, root, "1 of 3 done")?;

        // A single title click selects; arrows navigate one task at a time.
        let title = d.find(&list, &["1", "title"])?;
        d.click(&title)?;
        assert_eq!(state.selection.active().get()?.as_deref(), Some("1"));
        d.key(KeyCode::ArrowDown)?;
        assert_eq!(state.selection.active().get()?.as_deref(), Some("2"));
        d.key(KeyCode::ArrowUp)?;
        assert_eq!(state.selection.active().get()?.as_deref(), Some("1"));
        d.chord(alt(KeyCode::ArrowDown), KeyCode::ArrowDown)?;
        assert_eq!(rows(&d, &list)?, ["2", "1", "3"]);
        assert_eq!(
            state.tasks.get()?.iter().map(|t| t.id).collect::<Vec<_>>(),
            [2, 1, 3]
        );
        status(&d, root, "1 of 3 done")?;
        d.chord(alt(KeyCode::ArrowUp), KeyCode::ArrowUp)?;
        assert_eq!(rows(&d, &list)?, ["1", "2", "3"]);

        // Enter opens an editor inside the selected row; Enter saves to Vec.
        d.key(KeyCode::Enter)?;
        d.type_text("Oat milk")?;
        assert_eq!(state.tasks.get()?[0].title, "Buy Milk");
        d.key(KeyCode::Enter)?;
        assert_eq!(state.tasks.get()?[0].title, "Oat milk");
        assert_eq!(d.runtime().focused(), Some(list.clone()));
        assert_eq!(
            d.semantics(&title)?.value,
            Some(SemanticValue::Text("Oat milk".into()))
        );
        d.key(KeyCode::Enter)?;
        d.type_text("Discard this")?;
        d.key(KeyCode::Escape)?;
        assert_eq!(state.tasks.get()?[0].title, "Oat milk");
        assert_eq!(
            d.semantics(&title)?.value,
            Some(SemanticValue::Text("Oat milk".into()))
        );

        // Buttons use the same vector, retaining selection by task identity.
        let down = d.find(root, &["actions", "down"])?;
        let up = d.find(root, &["actions", "up"])?;
        d.click(&down)?;
        assert_eq!(rows(&d, &list)?, ["2", "1", "3"]);
        d.click(&up)?;
        assert_eq!(rows(&d, &list)?, ["1", "2", "3"]);
        status(&d, root, "1 of 3 done")?;

        // Filtering is case-insensitive and clears an active hidden row.
        let search = d.find(root, &["tasks", "filter-field", "filter"])?;
        let before = state.tasks.get()?;
        d.replace_text(&search, "PLAN")?;
        assert_eq!(rows(&d, &list)?, ["2"]);
        assert_eq!(state.tasks.get()?, before);
        assert_eq!(state.selection.active().get()?, None);
        status(&d, root, "1 of 3 done")?;
        let remove_button = d.find(root, &["actions", "remove"])?;
        d.click(&remove_button)?;
        assert_eq!(state.tasks.get()?, before);
        d.replace_text(&search, "nothing matches")?;
        assert!(rows(&d, &list)?.is_empty());
        assert_eq!(state.tasks.get()?, before);
        d.replace_text(&search, "")?;
        assert_eq!(rows(&d, &list)?, ["1", "2", "3"]);

        // Double-click rename and Escape exercise the pointer edit route too.
        let second = d.find(&list, &["2", "title"])?;
        d.click(&second)?;
        d.click(&second)?;
        d.type_text("Keep draft private")?;
        d.key(KeyCode::Escape)?;
        assert_eq!(state.tasks.get()?[1].title, "Write plan");
        d.key(KeyCode::Delete)?;
        assert_eq!(rows(&d, &list)?, ["1", "3"]);
        status(&d, root, "1 of 2 done")?;

        // Ctrl+N works with list focus and actually routes typing to the field.
        d.chord(
            KeyChord::new(
                CommandKey::Character('n'),
                Modifiers {
                    control: true,
                    ..Modifiers::default()
                },
            ),
            KeyCode::KeyN,
        )?;
        assert_eq!(d.runtime().focused(), Some(field.clone()));
        d.type_text("Fourth")?;
        d.key(KeyCode::Enter)?;
        assert_eq!(rows(&d, &list)?, ["1", "3", "4"]);
        status(&d, root, "1 of 3 done")?;
        let first = d.find(&list, &["1", "title"])?;
        d.click(&first)?;
        d.click(&remove_button)?;
        assert_eq!(rows(&d, &list)?, ["3", "4"]);
        status(&d, root, "0 of 2 done")?;
        Ok(())
    }
}

// Verifier test entry point (acceptance copy only; test builds only).
#[cfg(test)]
mod acceptance_tests;
