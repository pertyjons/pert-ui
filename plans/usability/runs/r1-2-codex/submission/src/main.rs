use pert_ui::prelude::*;
use pert_ui::{
    CollectionFilter, CommandKey, CommandScope, DraftEditor, DraftEditorController, DraftSelection,
    KeyChord, ListFilter, ListItems, ListSelection, ListSource, Modifiers, SelectionMode,
    VirtualList,
};

#[derive(Clone, PartialEq, Debug)]
struct Task {
    id: u64,
    title: String,
    done: bool,
}
#[derive(Clone)]
struct Model {
    tasks: Signal<Vec<Task>>,
    selection: ListSelection,
    query: Signal<String>,
    edit: Signal<Option<u64>>,
}
impl Model {
    fn select(&self, id: u64) -> Result<(), RuntimeError> {
        self.selection.set_active(Some(id.to_string()))
    }
    fn visible(&self, task: &Task) -> Result<bool, RuntimeError> {
        Ok(task
            .title
            .to_lowercase()
            .contains(&self.query.get()?.trim().to_lowercase()))
    }
    fn selected_index(&self) -> Result<Option<usize>, RuntimeError> {
        let key = self.selection.active().get()?;
        let tasks = self.tasks.get()?;
        Ok(tasks
            .iter()
            .position(|t| Some(t.id.to_string()) == key && self.visible(t).unwrap_or(false)))
    }
    fn add(&self, title: &str) -> Result<(), RuntimeError> {
        let title = title.trim();
        if title.is_empty() {
            return Ok(());
        }
        let id = self.tasks.get()?.iter().map(|t| t.id).max().unwrap_or(0) + 1;
        self.tasks.update(|v| {
            v.push(Task {
                id,
                title: title.into(),
                done: false,
            })
        })?;
        self.select(id)
    }
    fn remove(&self) -> Result<(), RuntimeError> {
        if let Some(i) = self.selected_index()? {
            self.tasks.update(|v| {
                v.remove(i);
            })?;
            let tasks = self.tasks.get()?;
            let next = tasks
                .iter()
                .skip(i)
                .chain(tasks.iter().take(i).rev())
                .find(|t| self.visible(t).unwrap_or(false));
            self.selection.set_active(next.map(|t| t.id.to_string()))?;
        }
        Ok(())
    }
    fn shift(&self, delta: isize) -> Result<(), RuntimeError> {
        if let Some(i) = self.selected_index()? {
            let j = i as isize + delta;
            if j >= 0 && j < self.tasks.get()?.len() as isize {
                self.tasks.update(|v| v.swap(i, j as usize))?;
            }
        }
        Ok(())
    }
}

// The title owns a transient draft; only Enter writes back to the task vector.
struct Title {
    model: Model,
    id: u64,
    draft: DraftEditorController,
    editor: Option<WidgetId>,
    children: Vec<Element>,
}
impl Title {
    fn begin(&mut self, cx: &mut EventContext) -> Result<(), RuntimeError> {
        self.model.select(self.id)?;
        let title = self
            .model
            .tasks
            .get()?
            .iter()
            .find(|t| t.id == self.id)
            .map(|t| t.title.clone())
            .unwrap_or_default();
        self.draft.begin(cx, &title, None, DraftSelection::All)?;
        if let Some(editor) = &self.editor {
            cx.request_focus_for(editor)?;
        }
        cx.invalidate(Invalidation::Layout);
        Ok(())
    }
}
impl Component for Title {
    fn retain_initial_children(&self) -> bool {
        true
    }
    fn take_children(&mut self) -> Vec<Element> {
        std::mem::take(&mut self.children)
    }
    fn focusable(&self) -> bool {
        !self.draft.is_active()
    }
    fn child_visible(&self, key: &str) -> bool {
        (key == "editor") == self.draft.is_active()
    }
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        self.editor = cx
            .children()
            .iter()
            .find(|c| c.key() == "editor")
            .map(|c| c.id().clone());
        if cx.read(&self.model.edit.read_only())? == Some(self.id) {
            cx.notifier().request();
        }
        Ok(())
    }
    fn deliver_notifications(&mut self, cx: &mut EventContext) -> Result<(), RuntimeError> {
        if self.model.edit.get()? == Some(self.id) {
            self.model.edit.set(None)?;
            self.begin(cx)?;
        }
        Ok(())
    }
    fn pointer_input(
        &mut self,
        cx: &mut EventContext,
        event: PointerEvent,
    ) -> Result<InputOutcome, RuntimeError> {
        if matches!(event.kind(), PointerKind::Press(PointerButton::Primary)) {
            self.model.select(self.id)?;
            cx.request_focus();
            return Ok(InputOutcome::Handled);
        }
        Ok(InputOutcome::Ignored)
    }
    fn keyboard_event(
        &mut self,
        cx: &mut EventContext,
        event: KeyEvent,
        _: &KeyboardState,
    ) -> Result<InputOutcome, RuntimeError> {
        if event.transition != KeyTransition::Pressed {
            return Ok(InputOutcome::Ignored);
        }
        match event.code {
            KeyCode::Enter | KeyCode::F2 if !self.draft.is_active() => {
                self.begin(cx)?;
            }
            KeyCode::Enter => {
                let title = self.draft.text().trim().to_owned();
                if title.is_empty() {
                    return Ok(InputOutcome::Handled);
                }
                self.model.tasks.update(|v| {
                    if let Some(t) = v.iter_mut().find(|t| t.id == self.id) {
                        t.title = title;
                    }
                })?;
                self.draft.end();
                cx.request_focus();
            }
            KeyCode::Escape if self.draft.is_active() => {
                self.draft.end();
                cx.request_focus();
            }
            _ => return Ok(InputOutcome::Ignored),
        }
        cx.invalidate(Invalidation::Layout);
        Ok(InputOutcome::Handled)
    }
    fn focus_within_changed(
        &mut self,
        cx: &mut EventContext,
        within: bool,
    ) -> Result<(), RuntimeError> {
        if !within && self.draft.is_active() {
            self.draft.end();
            cx.invalidate(Invalidation::Layout);
        }
        Ok(())
    }
}
struct Rows {
    tasks: Vec<Task>,
    model: Model,
}
impl ListItems for Rows {
    fn len(&self) -> usize {
        self.tasks.len()
    }
    fn key(&self, i: usize) -> Option<String> {
        self.tasks.get(i).map(|t| t.id.to_string())
    }
    fn text(&self, i: usize) -> Option<String> {
        self.tasks.get(i).map(|t| t.title.clone())
    }
    fn build(&self, i: usize) -> Option<Element> {
        let task = self.tasks.get(i)?;
        let id = task.id;
        let checked = self.model.tasks.sibling(task.done);
        let model = self.model.clone();
        let checkbox = Checkbox::new("", checked).on_change(move |_, done| {
            model.tasks.update(|v| {
                if let Some(t) = v.iter_mut().find(|t| t.id == id) {
                    t.done = done;
                }
            })?;
            model.select(id)
        });
        let draft = DraftEditorController::new();
        let title = self
            .model
            .tasks
            .map(move |v| {
                v.iter()
                    .find(|t| t.id == id)
                    .map(|t| t.title.clone())
                    .unwrap_or_default()
            })
            .ok()?;
        Some(
            Row::new([
                checkbox.key("done"),
                Title {
                    model: self.model.clone(),
                    id,
                    draft: draft.clone(),
                    editor: None,
                    children: vec![
                        Label::new(title).key("label"),
                        DraftEditor::new(draft).key("editor"),
                    ],
                }
                .key("title")
                .with_grow(1),
            ])
            .key("row"),
        )
    }
}
// Selection changes are event-phase writes, including after a search hides a row.
struct SelectionGuard {
    model: Model,
    children: Vec<Element>,
}
impl Component for SelectionGuard {
    fn take_children(&mut self) -> Vec<Element> {
        std::mem::take(&mut self.children)
    }
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        let _ = cx.read(&self.model.tasks.read_only())?;
        let _ = cx.read(&self.model.query.read_only())?;
        let _ = cx.read(&self.model.selection.active())?;
        cx.notifier().request();
        Ok(())
    }
    fn deliver_notifications(&mut self, _: &mut EventContext) -> Result<(), RuntimeError> {
        if let Some(key) = self.model.selection.active().get()? {
            let tasks = self.model.tasks.get()?;
            if !tasks
                .iter()
                .any(|t| t.id.to_string() == key && self.model.visible(t).unwrap_or(false))
            {
                let next = tasks
                    .iter()
                    .find(|t| self.model.visible(t).unwrap_or(false));
                self.model
                    .selection
                    .set_active(next.map(|t| t.id.to_string()))?;
            }
        }
        Ok(())
    }
}
#[cfg_attr(not(test), allow(dead_code))]
struct Handles {
    model: Model,
    root: WidgetId,
    new: WidgetId,
    list: WidgetId,
    filter: ListFilter,
    status: ReadSignal<String>,
}
fn mount(ui: &mut Runtime) -> Result<Handles, RuntimeError> {
    let model = Model {
        tasks: ui.signal(Vec::<Task>::new()),
        selection: ListSelection::new(ui, SelectionMode::Single),
        query: ui.signal(String::new()),
        edit: ui.signal(None),
    };
    let new = ui.signal(String::new());
    let m = model.clone();
    let source = model.tasks.map(move |tasks| {
        ListSource::new(Rows {
            tasks: tasks.clone(),
            model: m.clone(),
        })
    })?;
    let filter = ListFilter::new(ui, source)?.selection(model.selection.clone());
    let m = model.clone();
    let list = VirtualList::new(filter.items(), 36.0)?
        .selection(model.selection.clone())
        .on_activate(move |_, key| m.edit.set(key.parse().ok()));
    let status = model.tasks.map(|v| {
        format!(
            "{} of {} done",
            v.iter().filter(|t| t.done).count(),
            v.len()
        )
    })?;
    let m = model.clone();
    let n = new.clone();
    let field = TextField::new(&new)
        .placeholder("New task (Ctrl+N)")
        .on_submit(move |_, text| {
            m.add(text)?;
            n.set(String::new())
        });
    let m = model.clone();
    let n = new.clone();
    let add = Button::new("Add", move |_| {
        m.add(&n.get()?)?;
        n.set(String::new())
    });
    let m = model.clone();
    let remove = Button::new("Remove", move |_| m.remove());
    let m = model.clone();
    let up = Button::new("Move up", move |_| m.shift(-1));
    let m = model.clone();
    let down = Button::new("Move down", move |_| m.shift(1));
    let content = Column::new([
        Label::new("Tasks").key("heading"),
        Row::new([field.key("new"), add.key("add")]).key("entry"),
        Label::new("Select a title; Enter or F2 renames it. Enter saves, Escape cancels.")
            .key("help"),
        CollectionFilter::with_query(&model.query, filter.clone(), list.key("list"))
            .name("Search tasks")
            .key("filtered")
            .with_grow(1),
        Row::new([remove.key("remove"), up.key("up"), down.key("down")]).key("actions"),
        Label::new(status.clone()).key("status"),
    ])
    .key("content");
    let root = ui.mount_tree(
        SelectionGuard {
            model: model.clone(),
            children: vec![content],
        }
        .key("app"),
    )?;
    let new_id = ui.find_descendant(&root, "new")?.unwrap();
    let list_id = ui.find_descendant(&root, "list")?.unwrap();
    let target = new_id.clone();
    let _ = ui.register_command(
        "New task",
        CommandScope::Application,
        vec![KeyChord::new(
            CommandKey::Character('n'),
            Modifiers {
                control: true,
                ..Modifiers::default()
            },
        )],
        move |ui, _| ui.focus(&target),
    )?;
    let m = model.clone();
    let _ = ui.register_command(
        "Remove selected task",
        CommandScope::Application,
        vec![KeyChord::named(KeyCode::Delete)],
        move |_, _| m.remove(),
    )?;
    for (key, delta) in [(KeyCode::ArrowUp, -1), (KeyCode::ArrowDown, 1)] {
        let m = model.clone();
        let _ = ui.register_command(
            format!("Move {delta}"),
            CommandScope::Application,
            vec![KeyChord::new(
                CommandKey::Named(key),
                Modifiers {
                    alt: true,
                    ..Modifiers::default()
                },
            )],
            move |_, _| m.shift(delta),
        )?;
    }
    Ok(Handles {
        model,
        root,
        new: new_id,
        list: list_id,
        filter,
        status,
    })
}
fn main() -> Result<(), AppError> {
    App::new("Task list", |ui| {
        mount(ui)?;
        Ok(())
    })
    .run()
}

#[cfg(test)]
mod tests {
    use super::*;
    use pert_ui::CommandKeyInput;
    type TestResult = Result<(), Box<dyn std::error::Error>>;
    fn flush(ui: &mut Runtime) {
        ui.flush(ui.now()).unwrap();
    }
    fn key(ui: &mut Runtime, code: KeyCode, mods: Modifiers) {
        ui.native_modifiers(mods).unwrap();
        let meaning = match code {
            KeyCode::KeyN => CommandKey::Character('n'),
            KeyCode::KeyA => CommandKey::Character('a'),
            _ => CommandKey::Named(code),
        };
        for action in [KeyAction::Press, KeyAction::Release] {
            ui.command_key_input(CommandKeyInput::new(code, action, meaning))
                .unwrap();
        }
        ui.native_modifiers(Modifiers::default()).unwrap();
        flush(ui);
    }
    fn plain(ui: &mut Runtime, code: KeyCode) {
        key(ui, code, Modifiers::default());
    }
    fn click(ui: &mut Runtime, id: &WidgetId) {
        flush(ui);
        let [x, y, w, h] = ui.bounds(id).unwrap().unwrap().values();
        assert!(w > 0.0 && h > 0.0, "click has empty bounds");
        let p = Point::new(x + w / 2.0, y + h / 2.0).unwrap();
        for kind in [
            PointerKind::Move,
            PointerKind::Press(PointerButton::Primary),
            PointerKind::Release(PointerButton::Primary),
        ] {
            ui.pointer_input(PointerEvent::new(p, kind)).unwrap();
        }
        flush(ui);
    }
    fn find(ui: &Runtime, parent: &WidgetId, name: &str) -> WidgetId {
        ui.find_descendant(parent, name).unwrap().unwrap()
    }
    fn row(ui: &Runtime, h: &Handles, id: u64) -> WidgetId {
        find(ui, &h.list, &id.to_string())
    }
    fn status(ui: &mut Runtime, h: &Handles, expected: &str) {
        flush(ui);
        assert_eq!(h.status.get().unwrap(), expected);
        let snapshot = ui.inspect();
        assert!(
            snapshot.components.iter().any(|c| c.key == "status"
                && c.details.semantics.as_ref().and_then(|s| s.name.as_deref()) == Some(expected)),
            "status label missing from inspection: {snapshot:#?}"
        );
        ui.paint().unwrap();
    }
    fn add(ui: &mut Runtime, h: &Handles, title: &str) {
        key(
            ui,
            KeyCode::KeyN,
            Modifiers {
                control: true,
                ..Modifiers::default()
            },
        );
        assert_eq!(ui.focused(), Some(h.new.clone()));
        ui.text_input(title).unwrap();
        plain(ui, KeyCode::Enter);
    }
    #[test]
    fn task_list_through_simulated_input() -> TestResult {
        let mut ui = Runtime::new();
        ui.set_viewport(Viewport::new(900.0, 640.0, 1.0)?)?;
        let h = mount(&mut ui)?;
        status(&mut ui, &h, "0 of 0 done");
        add(&mut ui, &h, "   ");
        assert!(h.model.tasks.get()?.is_empty());
        add(&mut ui, &h, "Alpha");
        add(&mut ui, &h, "Beta");
        // Add button uses the same text field and vector.
        ui.text_input("Gamma")?;
        let add_button = find(&ui, &h.root, "add");
        click(&mut ui, &add_button);
        status(&mut ui, &h, "0 of 3 done");
        let first = row(&ui, &h, 1);
        let checkbox = find(&ui, &first, "done");
        click(&mut ui, &checkbox);
        assert!(h.model.tasks.get()?[0].done);
        status(&mut ui, &h, "1 of 3 done");
        let second = row(&ui, &h, 2);
        let title = find(&ui, &second, "title");
        click(&mut ui, &title);
        assert_eq!(h.model.selection.active().get()?.as_deref(), Some("2"));
        key(
            &mut ui,
            KeyCode::ArrowUp,
            Modifiers {
                alt: true,
                ..Modifiers::default()
            },
        );
        assert_eq!(
            h.model
                .tasks
                .get()?
                .iter()
                .map(|t| t.id)
                .collect::<Vec<_>>(),
            vec![2, 1, 3]
        );
        key(
            &mut ui,
            KeyCode::ArrowDown,
            Modifiers {
                alt: true,
                ..Modifiers::default()
            },
        );
        assert_eq!(
            h.model
                .tasks
                .get()?
                .iter()
                .map(|t| t.id)
                .collect::<Vec<_>>(),
            vec![1, 2, 3]
        );
        status(&mut ui, &h, "1 of 3 done");
        // A draft replaces the selected title, without changing the vector until Enter.
        plain(&mut ui, KeyCode::Enter);
        ui.text_input("Renamed")?;
        assert_eq!(h.model.tasks.get()?[1].title, "Beta");
        plain(&mut ui, KeyCode::Enter);
        assert_eq!(h.model.tasks.get()?[1].title, "Renamed");
        assert!(ui.inspect().components.iter().any(|c| c.visible
            && c.details.semantics.as_ref().and_then(|s| s.name.as_deref()) == Some("Renamed")));
        plain(&mut ui, KeyCode::Enter);
        ui.text_input("Discard this")?;
        plain(&mut ui, KeyCode::Escape);
        assert_eq!(h.model.tasks.get()?[1].title, "Renamed");
        // Arrow selection is owned by VirtualList.
        ui.focus(&h.list)?;
        plain(&mut ui, KeyCode::ArrowDown);
        assert_eq!(h.model.selection.active().get()?.as_deref(), Some("3"));
        let search = find(&ui, &h.root, "filter-field");
        let search = ui.children(&search)?[0].clone();
        click(&mut ui, &search);
        ui.text_input("aLP")?;
        flush(&mut ui);
        assert_eq!(h.filter.items().get()?.len(), 1);
        assert_eq!(h.filter.items().get()?.key(0).as_deref(), Some("1"));
        assert_eq!(h.model.tasks.get()?.len(), 3);
        status(&mut ui, &h, "1 of 3 done");
        plain(&mut ui, KeyCode::ArrowDown);
        plain(&mut ui, KeyCode::Home);
        assert_eq!(h.model.selection.active().get()?.as_deref(), Some("1"));
        plain(&mut ui, KeyCode::Delete);
        assert_eq!(h.model.tasks.get()?.len(), 2);
        status(&mut ui, &h, "0 of 2 done");
        // Clear search through the editor's own Escape handling.
        click(&mut ui, &search);
        plain(&mut ui, KeyCode::Escape);
        assert_eq!(h.filter.items().get()?.len(), 2);
        ui.focus(&h.list)?;
        plain(&mut ui, KeyCode::Home);
        let down = find(&ui, &h.root, "down");
        click(&mut ui, &down);
        assert_eq!(
            h.model
                .tasks
                .get()?
                .iter()
                .map(|t| t.id)
                .collect::<Vec<_>>(),
            vec![3, 2]
        );
        let remove = find(&ui, &h.root, "remove");
        click(&mut ui, &remove);
        assert_eq!(h.model.tasks.get()?.len(), 1);
        status(&mut ui, &h, "0 of 1 done");
        Ok(())
    }
    #[test]
    fn keyboard_rename_and_filter_selection_edges() -> TestResult {
        let mut ui = Runtime::new();
        ui.set_viewport(Viewport::new(900.0, 640.0, 1.0)?)?;
        let h = mount(&mut ui)?;
        add(&mut ui, &h, "First");
        add(&mut ui, &h, "Second");
        ui.focus(&h.list)?;
        plain(&mut ui, KeyCode::Home);
        plain(&mut ui, KeyCode::ArrowDown);
        plain(&mut ui, KeyCode::ArrowUp);
        assert_eq!(h.model.selection.active().get()?.as_deref(), Some("1"));
        plain(&mut ui, KeyCode::Enter);
        ui.text_input("Updated First")?;
        assert_eq!(h.model.tasks.get()?[0].title, "First");
        plain(&mut ui, KeyCode::Enter);
        assert_eq!(h.model.tasks.get()?[0].title, "Updated First");
        // Empty rename stays a draft; Escape restores the saved title.
        plain(&mut ui, KeyCode::Enter);
        ui.text_input("  ")?;
        plain(&mut ui, KeyCode::Enter);
        assert_eq!(h.model.tasks.get()?[0].title, "Updated First");
        plain(&mut ui, KeyCode::Escape);
        let before = h.model.tasks.get()?;
        let search = find(&ui, &h.root, "filter-field");
        let search = ui.children(&search)?[0].clone();
        click(&mut ui, &search);
        ui.text_input("SECOND")?;
        flush(&mut ui);
        assert_eq!(h.filter.items().get()?.len(), 1);
        assert_eq!(h.model.tasks.get()?, before);
        assert_eq!(h.model.selection.active().get()?.as_deref(), Some("2"));
        key(
            &mut ui,
            KeyCode::KeyA,
            Modifiers {
                control: true,
                ..Modifiers::default()
            },
        );
        ui.text_input("no task matches")?;
        flush(&mut ui);
        assert_eq!(h.filter.items().get()?.len(), 0);
        assert_eq!(h.model.selection.active().get()?, None);
        assert_eq!(h.model.tasks.get()?, before);
        // A command from outside a text editor cannot remove a hidden task.
        let remove = find(&ui, &h.root, "remove");
        click(&mut ui, &remove);
        plain(&mut ui, KeyCode::Delete);
        assert_eq!(h.model.tasks.get()?, before);
        click(&mut ui, &search);
        plain(&mut ui, KeyCode::Escape);
        assert_eq!(h.filter.items().get()?.len(), 2);
        let first = row(&ui, &h, 1);
        let checkbox = find(&ui, &first, "done");
        click(&mut ui, &checkbox);
        status(&mut ui, &h, "1 of 2 done");
        click(&mut ui, &checkbox);
        assert!(!h.model.tasks.get()?[0].done);
        status(&mut ui, &h, "0 of 2 done");
        // Boundary reorders and button reorders preserve selection by key.
        let up = find(&ui, &h.root, "up");
        click(&mut ui, &up);
        assert_eq!(h.model.tasks.get()?[0].id, 1);
        let down = find(&ui, &h.root, "down");
        click(&mut ui, &down);
        click(&mut ui, &up);
        assert_eq!(h.model.tasks.get()?[0].id, 1);
        assert_eq!(h.model.selection.active().get()?.as_deref(), Some("1"));
        Ok(())
    }
}
