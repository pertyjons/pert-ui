use pert_ui::prelude::*;
use pert_ui::{
    Length, ListItems, ListSelection, ListSource, SelectionMode, SplitAxis, SplitPane, SplitSizing,
    Toast, ToastController, ToastHost,
};
use std::{
    cell::RefCell,
    fs,
    path::PathBuf,
    rc::{Rc, Weak},
};

type R<T = ()> = Result<T, RuntimeError>;

#[derive(Clone)]
struct Note {
    name: String,
    text: Signal<String>,
    saved: Signal<String>,
    title: ReadSignal<String>,
    status: ReadSignal<String>,
}
#[derive(Clone)]
struct Editor {
    folder: PathBuf,
    weak: Weak<Editor>,
    notes: Rc<RefCell<Vec<Note>>>,
    tabs: Rc<RefCell<Option<WidgetId>>>,
    active: Signal<Option<String>>,
    files: Signal<ListSource>,
    selection: ListSelection,
    new_dialog: DialogController,
    close_dialog: DialogController,
    closing: Rc<RefCell<Option<String>>>,
    draft: Signal<String>,
    error: Signal<String>,
    toast: ToastController,
}
struct Files(Vec<String>, Weak<Editor>);
impl ListItems for Files {
    fn len(&self) -> usize {
        self.0.len()
    }
    fn key(&self, i: usize) -> Option<String> {
        self.0.get(i).cloned()
    }
    fn text(&self, i: usize) -> Option<String> {
        self.key(i)
    }
    fn build(&self, i: usize) -> Option<Element> {
        let name = self.0.get(i)?.clone();
        let weak = self.1.clone();
        Some(
            Button::new(name.clone(), move |cx| {
                if let Some(app) = weak.upgrade() {
                    app.selection.set_active(Some(name.clone()))?;
                    let name = name.clone();
                    let _receipt = cx.defer(move |ui| app.open(ui, &name))?;
                }
                Ok(())
            })
            .key("name"),
        )
    }
}
impl Editor {
    fn list(folder: &PathBuf) -> std::io::Result<Vec<String>> {
        let mut names = Vec::new();
        for item in fs::read_dir(folder)? {
            let item = item?;
            if item.file_type()?.is_file() && item.path().extension().is_some_and(|e| e == "txt") {
                if let Some(name) = item.file_name().to_str() {
                    names.push(name.to_owned());
                }
            }
        }
        names.sort();
        Ok(names)
    }
    fn notify(&self, message: String) -> R {
        self.toast.push(Toast::new(message))?;
        Ok(())
    }
    fn reconcile(&self, ui: &mut Runtime) -> R {
        let children = self
            .notes
            .borrow()
            .iter()
            .map(|note| {
                Tab::new(
                    &note.name,
                    note.title.clone(),
                    TextArea::new(&note.text).key("editor"),
                )
                .fixed()
                .into_element()
            })
            .collect();
        ui.reconcile_children(self.tabs.borrow().as_ref().unwrap(), children)
    }
    fn open(&self, ui: &mut Runtime, name: &str) -> R {
        if !self.notes.borrow().iter().any(|n| n.name == name) {
            let content = match fs::read_to_string(self.folder.join(name)) {
                Ok(s) => s,
                Err(e) => return self.notify(format!("Could not open {name}: {e}")),
            };
            let text = ui.signal(content.clone());
            let saved = ui.signal(content);
            let title_name = name.to_owned();
            let title = text.combine(&saved, move |text, saved| {
                format!("{title_name}{}", if text != saved { " *" } else { "" })
            })?;
            let status_name = name.to_owned();
            let status = text.map(move |text| {
                format!(
                    "{status_name}: {} lines, {} words",
                    text.matches('\n').count() + 1,
                    text.split_whitespace().count()
                )
            })?;
            self.notes.borrow_mut().push(Note {
                name: name.into(),
                text,
                saved,
                title,
                status,
            });
            self.reconcile(ui)?;
        }
        ui.reveal_child(self.tabs.borrow().as_ref().unwrap(), name)?;
        self.active.set(Some(name.to_owned()))
    }
    fn current(&self) -> R<Option<Note>> {
        let active = self.active.get()?;
        Ok(self
            .notes
            .borrow()
            .iter()
            .find(|n| Some(&n.name) == active.as_ref())
            .cloned())
    }
    fn save_note(&self, note: &Note) -> R<bool> {
        let text = note.text.get()?;
        match fs::write(self.folder.join(&note.name), &text) {
            Ok(()) => {
                note.saved.set(text)?;
                Ok(true)
            }
            Err(e) => {
                self.notify(format!("Save failed for {}: {e}", note.name))?;
                Ok(false)
            }
        }
    }
    fn save(&self) -> R {
        if let Some(note) = self.current()? {
            self.save_note(&note)?;
        }
        Ok(())
    }
    fn remove(&self, ui: &mut Runtime, name: &str) -> R {
        self.notes.borrow_mut().retain(|n| n.name != name);
        self.selection.clear()?;
        self.reconcile(ui)?;
        // Tabs reports the replacement selection after settlement.
        if self.notes.borrow().is_empty() {
            self.active.set(None)?;
        }
        Ok(())
    }
    fn close(&self, ui: &mut Runtime) -> R {
        if let Some(note) = self.current()? {
            if note.text.get()? != note.saved.get()? {
                *self.closing.borrow_mut() = Some(note.name);
                self.close_dialog.take_outcome()?;
                self.close_dialog.open()?;
            } else {
                self.remove(ui, &note.name)?;
            }
        }
        Ok(())
    }
    fn finish_close(&self, ui: &mut Runtime, save: bool) -> R {
        let name = self.closing.borrow().clone();
        if let Some(name) = name {
            let note = self.notes.borrow().iter().find(|n| n.name == name).cloned();
            if let Some(note) = note {
                if save && !self.save_note(&note)? {
                    return Ok(());
                }
                self.close_dialog.accept()?;
                self.remove(ui, &name)?;
            }
        }
        Ok(())
    }
    fn show_new(&self) -> R {
        self.new_dialog.take_outcome()?;
        self.draft.set(String::new())?;
        self.error.set(String::new())?;
        self.new_dialog.open()
    }
    fn create(&self, ui: &mut Runtime) -> R {
        let mut name = self.draft.get()?;
        let count = name.chars().count();
        if !(1..=64).contains(&count)
            || name.starts_with('.')
            || name.contains(['/', '\\'])
            || name.contains('\0')
        {
            return self
                .error
                .set("Use 1–64 characters, no / or \\, and do not start with a dot.".into());
        }
        if !name.ends_with(".txt") {
            name.push_str(".txt");
        }
        match fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(self.folder.join(&name))
        {
            Ok(_) => {
                match Self::list(&self.folder) {
                    Ok(names) => self
                        .files
                        .set(ListSource::new(Files(names, self.weak.clone())))?,
                    Err(e) => {
                        self.notify(format!("Could not refresh notes: {e}"))?;
                    }
                }
                self.new_dialog.accept()?;
                self.open(ui, &name)
            }
            Err(e) => self
                .error
                .set(if e.kind() == std::io::ErrorKind::AlreadyExists {
                    format!("A note named {name} already exists.")
                } else {
                    format!("Could not create note: {e}")
                }),
        }
    }
}

fn build(ui: &mut Runtime, folder: PathBuf, names: Vec<String>) -> R<WidgetId> {
    let app = Rc::new_cyclic(|weak| Editor {
        weak: weak.clone(),
        folder,
        notes: Rc::default(),
        tabs: Rc::default(),
        active: ui.signal(None),
        files: ui.signal(ListSource::new(Files(names, weak.clone()))),
        selection: ListSelection::new(ui, SelectionMode::Single),
        new_dialog: DialogController::new(ui),
        close_dialog: DialogController::new(ui),
        closing: Rc::default(),
        draft: ui.signal(String::new()),
        error: ui.signal(String::new()),
        toast: ToastController::new(ui),
    });
    let a = app.clone();
    let new = ui.register_command(
        "New note",
        CommandScope::Application,
        vec![KeyChord::primary('n')],
        move |_, _| a.show_new(),
    )?;
    let a = app.clone();
    let save = ui.register_command(
        "Save",
        CommandScope::Application,
        vec![KeyChord::primary('s')],
        move |_, _| a.save(),
    )?;
    let a = app.clone();
    let close = ui.register_command(
        "Close tab",
        CommandScope::Application,
        vec![KeyChord::primary('w')],
        move |ui, _| a.close(ui),
    )?;
    let selection = app.selection.clone();
    let a = app.clone();
    let list = VirtualList::new(&app.files, 32.0)?
        .selection(selection)
        .on_activate(move |cx, name| {
            let (a, name) = (a.clone(), name.to_owned());
            let _receipt = cx.defer(move |ui| a.open(ui, &name))?;
            Ok(())
        });
    let a = app.clone();
    let tabs = Tabs::new([]).on_select(move |_, change| a.active.set(change.selected.clone()));
    let a = app.clone();
    let status = Dynamic::new(&app.active, move |_, active| {
        let note = a
            .notes
            .borrow()
            .iter()
            .find(|n| Some(&n.name) == active.as_ref())
            .cloned();
        match note {
            Some(note) => Label::new(note.status).key("counts"),
            None => Label::new("").key("counts"),
        }
    });
    let content = Column::new([
        SplitPane::new(SplitAxis::LeftRight, list.key("notes"), tabs.key("tabs"))
            .sizing(SplitSizing::first(Length::from(220)))
            .key("split")
            .with_grow(1),
        status.key("status"),
    ])
    .align(Align::Stretch)
    .key("content");
    let menu = MenuBar::new(
        ui,
        vec![MenuItem::submenu(
            "file",
            "File",
            vec![
                MenuItem::command(ui, "new", &new, |_, _| Ok(()))?,
                MenuItem::command(ui, "save", &save, |_, _| Ok(()))?,
                MenuItem::command(ui, "close", &close, |_, _| Ok(()))?,
            ],
        )],
    )?
    .content(content);
    let a = app.clone();
    let cancel = app.new_dialog.clone();
    let form = Column::new([
        Label::new("New note").key("heading"),
        TextField::new(&app.draft).key("name"),
        Label::new(app.error.read_only()).key("error"),
        Row::new([
            Button::new("Create", move |cx| {
                let a = a.clone();
                let _receipt = cx.defer(move |ui| a.create(ui))?;
                Ok(())
            })
            .key("create"),
            Button::new("Cancel", move |_| cancel.cancel().map(|_| ())).key("cancel"),
        ])
        .key("actions"),
    ])
    .key("new-form");
    let a = app.clone();
    let b = app.clone();
    let cancel = app.close_dialog.clone();
    let confirm = Column::new([
        Label::new("Save changes before closing?").key("heading"),
        Row::new([
            Button::new("Save", move |cx| {
                let a = a.clone();
                let _receipt = cx.defer(move |ui| a.finish_close(ui, true))?;
                Ok(())
            })
            .key("save"),
            Button::new("Discard", move |cx| {
                let b = b.clone();
                let _receipt = cx.defer(move |ui| b.finish_close(ui, false))?;
                Ok(())
            })
            .key("discard"),
            Button::new("Cancel", move |_| cancel.cancel().map(|_| ())).key("cancel"),
        ])
        .key("actions"),
    ])
    .key("close-form");
    let root = ui.mount_tree(
        ToastHost::new(
            app.toast.clone(),
            Dialog::new(
                app.close_dialog.clone(),
                Dialog::new(app.new_dialog.clone(), menu.key("menu"), form).key("new-dialog"),
                confirm,
            )
            .key("close-dialog"),
        )
        .key("app"),
    )?;
    *app.tabs.borrow_mut() = ui.find_descendant(&root, "tabs")?;
    Ok(root)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let folder = std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("notes"));
    fs::create_dir_all(&folder)?;
    let names = Editor::list(&folder)?;
    App::new("Notes", move |ui| {
        build(ui, folder, names)?;
        Ok(())
    })
    .run()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use pert_ui::Semantics;
    use pert_ui::testing::*;
    use std::error::Error;
    type TestResult = Result<(), Box<dyn Error>>;

    struct Harness {
        ui: Runtime,
        root: WidgetId,
        folder: PathBuf,
    }
    impl Harness {
        fn new() -> Result<Self, Box<dyn Error>> {
            let folder = PathBuf::from(format!("/work/test-notes-{}", std::process::id()));
            if folder.exists() {
                fs::remove_dir_all(&folder)?;
            }
            fs::create_dir_all(&folder)?;
            fs::write(folder.join("Beta.txt"), "Beta original")?;
            fs::write(folder.join("Alpha.txt"), "Hello world")?;
            fs::write(folder.join("ignore.md"), "not a note")?;
            let mut ui = Runtime::new();
            ui.set_viewport(Viewport::new(1000.0, 700.0, 1.0)?)?;
            let root = build(&mut ui, folder.clone(), Editor::list(&folder)?)?;
            let mut h = Self { ui, root, folder };
            h.settle()?;
            Ok(h)
        }
        fn settle(&mut self) -> TestResult {
            let mut d = Driver::new(&mut self.ui);
            d.settle()?;
            d.settle()?;
            Ok(())
        }
        fn find(&self, key: &str) -> WidgetId {
            self.ui
                .find_descendant(&self.root, key)
                .unwrap()
                .unwrap_or_else(|| panic!("missing {key}"))
        }
        fn within(&self, parent: &str, key: &str) -> WidgetId {
            self.ui
                .find_descendant(&self.find(parent), key)
                .unwrap()
                .unwrap()
        }
        fn click(&mut self, id: WidgetId) -> TestResult {
            Driver::new(&mut self.ui).click(&id)?;
            self.settle()
        }
        fn key(&mut self, key: KeyCode) -> TestResult {
            Driver::new(&mut self.ui).key(key)?;
            self.settle()
        }
        fn shortcut(&mut self, letter: char, key: KeyCode) -> TestResult {
            Driver::new(&mut self.ui).chord(KeyChord::primary(letter), key)?;
            self.settle()
        }
        fn replace(&mut self, id: WidgetId, text: &str) -> TestResult {
            Driver::new(&mut self.ui).replace_text(&id, text)?;
            self.settle()
        }
        fn semantics(&mut self, id: WidgetId) -> Semantics {
            Driver::new(&mut self.ui).semantics(&id).unwrap()
        }
        fn tabs(&mut self) -> Vec<(String, bool)> {
            self.semantics(self.find("tabs"))
                .items
                .into_iter()
                .map(|i| (i.name.unwrap(), i.selected))
                .collect()
        }
        fn status(&mut self) -> Option<String> {
            self.semantics(self.find("counts")).name
        }
        fn visible(&self, key: &str) -> bool {
            self.ui
                .inspect()
                .components
                .iter()
                .any(|c| c.key == key && c.visible)
        }
        fn editor(&self, name: &str) -> WidgetId {
            self.ui
                .find_descendant(&self.within("tabs", name), "editor")
                .unwrap()
                .unwrap()
        }
        fn menu(&mut self, index: usize) -> TestResult {
            self.key(KeyCode::F10)?;
            self.key(KeyCode::ArrowDown)?;
            for _ in 0..index {
                self.key(KeyCode::ArrowDown)?;
            }
            self.key(KeyCode::Enter)
        }
        fn capture(&mut self) -> TestResult {
            self.ui.set_capture_support(true);
            let pending = self
                .ui
                .request_capture(CaptureRequest::new(CaptureTarget::window(
                    &self.ui.primary_window(),
                )));
            self.settle()?;
            CpuRenderer::new().serve_captures(&mut self.ui);
            let capture = pending.wait()?.value?;
            fs::write("/work/notes-editor.png", capture.image.encode_png())?;
            assert_eq!((capture.image.width(), capture.image.height()), (1000, 700));
            Ok(())
        }
    }
    impl Drop for Harness {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.folder);
        }
    }

    #[test]
    fn acceptance_through_ui() -> TestResult {
        let mut h = Harness::new()?;
        assert!(h.tabs().is_empty());
        assert_eq!(h.status(), None);
        let list = h.find("notes");
        let rows = h.ui.children(&list)?;
        let shown: Vec<_> = rows
            .iter()
            .map(|id| h.ui.key(id).unwrap().to_owned())
            .collect();
        assert_eq!(shown, ["Alpha.txt", "Beta.txt"]);
        h.click(h.find("Alpha.txt"))?;
        assert_eq!(h.tabs(), [("Alpha.txt".into(), true)]);
        h.ui.focus(&list)?;
        h.key(KeyCode::ArrowDown)?;
        h.key(KeyCode::Enter)?;
        assert_eq!(
            h.tabs(),
            [("Alpha.txt".into(), false), ("Beta.txt".into(), true)]
        );
        // Pointer reopening switches to the existing tab.
        let alpha_row = h.ui.find_descendant(&list, "Alpha.txt")?.unwrap();
        h.click(alpha_row)?;
        assert_eq!(
            h.tabs(),
            [("Alpha.txt".into(), true), ("Beta.txt".into(), false)]
        );
        // An unchanged sidebar selection must still activate on every click.
        h.ui.focus(&h.find("tabs"))?;
        h.key(KeyCode::End)?;
        assert_eq!(h.tabs()[1], ("Beta.txt".into(), true));
        let alpha_row = h.ui.find_descendant(&list, "Alpha.txt")?.unwrap();
        h.click(alpha_row)?;
        assert_eq!(h.tabs()[0], ("Alpha.txt".into(), true));
        let editor = h.editor("Alpha.txt");
        h.replace(editor, "One two\nthree four five\n")?;
        assert_eq!(h.tabs()[0], ("Alpha.txt *".into(), true));
        assert_eq!(h.status().as_deref(), Some("Alpha.txt: 3 lines, 5 words"));
        h.capture()?;
        h.shortcut('s', KeyCode::KeyS)?;
        assert_eq!(
            fs::read_to_string(h.folder.join("Alpha.txt"))?,
            "One two\nthree four five\n"
        );
        assert_eq!(h.tabs()[0].0, "Alpha.txt");
        h.replace(h.editor("Alpha.txt"), "Saved from menu")?;
        h.menu(1)?;
        assert_eq!(
            fs::read_to_string(h.folder.join("Alpha.txt"))?,
            "Saved from menu"
        );
        assert_eq!(h.tabs()[0].0, "Alpha.txt");
        // New note validation stays visible and never creates an invalid file.
        h.menu(0)?;
        assert!(h.visible("new-form"));
        h.key(KeyCode::Escape)?;
        assert!(!h.visible("new-form"));
        h.shortcut('n', KeyCode::KeyN)?;
        assert!(h.visible("new-form"));
        h.click(h.within("new-form", "create"))?;
        assert!(
            h.semantics(h.within("new-form", "error"))
                .name
                .unwrap()
                .contains("1–64")
        );
        h.replace(h.within("new-form", "name"), "bad/name")?;
        h.click(h.within("new-form", "create"))?;
        assert!(
            h.semantics(h.within("new-form", "error"))
                .name
                .unwrap()
                .contains("no /")
        );
        h.replace(h.within("new-form", "name"), "Alpha.txt")?;
        h.click(h.within("new-form", "create"))?;
        assert!(
            h.semantics(h.within("new-form", "error"))
                .name
                .unwrap()
                .contains("already exists")
        );
        assert_eq!(fs::read_dir(&h.folder)?.count(), 3);
        h.replace(h.within("new-form", "name"), "Gamma")?;
        h.click(h.within("new-form", "create"))?;
        assert!(!h.visible("new-form"));
        assert_eq!(fs::read_to_string(h.folder.join("Gamma.txt"))?, "");
        assert_eq!(h.tabs().last().unwrap(), &("Gamma.txt".into(), true));
        let gamma_row = h.ui.find_descendant(&list, "Gamma.txt")?.unwrap();
        assert_eq!(h.semantics(gamma_row).name.as_deref(), Some("Gamma.txt"));
        // Cancel keeps the exact edit; Discard leaves the original empty file.
        h.replace(h.editor("Gamma.txt"), "unsaved draft")?;
        h.shortcut('w', KeyCode::KeyW)?;
        assert!(h.visible("close-form"));
        h.click(h.within("close-form", "cancel"))?;
        assert!(!h.visible("close-form"));
        assert_eq!(
            h.semantics(h.editor("Gamma.txt")).value,
            Some(SemanticValue::Text("unsaved draft".into()))
        );
        assert_eq!(h.tabs().len(), 3);
        h.menu(2)?;
        h.click(h.within("close-form", "discard"))?;
        assert_eq!(h.tabs().len(), 2);
        assert_eq!(fs::read_to_string(h.folder.join("Gamma.txt"))?, "");
        // Save on close writes and removes only the active note.
        h.replace(h.editor("Beta.txt"), "close and save")?;
        h.shortcut('w', KeyCode::KeyW)?;
        h.click(h.within("close-form", "save"))?;
        assert_eq!(
            fs::read_to_string(h.folder.join("Beta.txt"))?,
            "close and save"
        );
        assert_eq!(h.tabs().len(), 1);
        h.shortcut('w', KeyCode::KeyW)?;
        assert!(h.tabs().is_empty());
        assert!(!h.visible("close-form"));
        assert_eq!(h.status(), None);
        // Reopen and make saving fail deterministically even when run as root.
        let row = h.ui.find_descendant(&list, "Alpha.txt")?.unwrap();
        h.click(row)?;
        h.replace(h.editor("Alpha.txt"), "must remain modified")?;
        fs::remove_dir_all(&h.folder)?;
        h.shortcut('s', KeyCode::KeyS)?;
        assert_eq!(h.tabs(), [("Alpha.txt *".into(), true)]);
        let notification =
            h.ui.inspect()
                .components
                .into_iter()
                .find(|c| c.key == "toasts")
                .unwrap();
        assert!(notification.visible);
        assert_eq!(
            notification.details.semantics.unwrap().value,
            Some(SemanticValue::Number {
                value: 1.0,
                minimum: Some(0.0),
                maximum: None
            })
        );
        h.shortcut('w', KeyCode::KeyW)?;
        h.click(h.within("close-form", "save"))?;
        assert!(h.visible("close-form"));
        assert_eq!(h.tabs().len(), 1);
        h.click(h.within("close-form", "cancel"))?;
        assert_eq!(h.tabs(), [("Alpha.txt *".into(), true)]);
        Ok(())
    }
}

// Acceptance verifier entry point (test-only; no application behavior change).
#[cfg(test)]
#[path = "acceptance_tests.rs"]
mod acceptance_tests;
