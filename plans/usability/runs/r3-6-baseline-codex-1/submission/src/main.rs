use pert_ui::prelude::*;
use pert_ui::windowing::*;
use std::{cell::RefCell, fs, path::PathBuf, rc::Rc};

type UiResult<T = ()> = Result<T, RuntimeError>;

#[derive(Clone)]
struct Note {
    name: String,
    text: Signal<String>,
    saved: Signal<String>,
}

#[derive(Clone)]
struct Editor {
    folder: PathBuf,
    notes: Rc<RefCell<Vec<Note>>>,
    names: Signal<Vec<String>>,
    active: Signal<Option<String>>,
    tabs: Rc<RefCell<Option<WidgetId>>>,
    new_dialog: DialogController,
    close_dialog: DialogController,
    draft: Signal<String>,
    error: Signal<String>,
    toast: ToastController,
}

impl Editor {
    fn notify(&self, message: String) -> UiResult {
        self.toast.push(Toast::new(message))?;
        Ok(())
    }
    fn current(&self) -> UiResult<Option<Note>> {
        let active = self.active.get()?;
        Ok(self
            .notes
            .borrow()
            .iter()
            .find(|n| Some(&n.name) == active.as_ref())
            .cloned())
    }
    fn sync_tabs(&self, ui: &mut Runtime) -> UiResult {
        let tabs = self.tabs.borrow().clone().expect("mounted tabs");
        let children = self
            .notes
            .borrow()
            .iter()
            .map(|note| {
                let name = note.name.clone();
                let title = note
                    .text
                    .read_only()
                    .combine(&note.saved.read_only(), move |text, saved| {
                        format!("{name}{}", if text != saved { " *" } else { "" })
                    })?;
                Ok(Tab::new(
                    note.name.clone(),
                    title,
                    TextArea::new(&note.text).key("editor").with_grow(1),
                )
                .into_element())
            })
            .collect::<UiResult<Vec<_>>>()?;
        ui.reconcile_children(&tabs, children)
    }
    fn open(&self, ui: &mut Runtime, name: &str) -> UiResult {
        if !self.notes.borrow().iter().any(|n| n.name == name) {
            match fs::read_to_string(self.folder.join(name)) {
                Ok(raw) => {
                    let text = raw.replace("\r\n", "\n").replace('\r', "\n");
                    if let Err(e) = TextArea::check(&text) {
                        return self.notify(format!("Cannot open {name}: {e}"));
                    }
                    self.notes.borrow_mut().push(Note {
                        name: name.into(),
                        text: ui.signal(text.clone()),
                        saved: ui.signal(text),
                    });
                    self.sync_tabs(ui)?;
                }
                Err(e) => return self.notify(format!("Cannot open {name}: {e}")),
            }
        }
        self.active.set(Some(name.into()))?;
        let tabs = self.tabs.borrow().clone().unwrap();
        ui.reveal_child(&tabs, name)?;
        Ok(())
    }
    fn save(&self) -> UiResult<bool> {
        let Some(note) = self.current()? else {
            return Ok(true);
        };
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
    fn remove_current(&self, ui: &mut Runtime) -> UiResult {
        let active = self.active.get()?;
        let next = {
            let mut notes = self.notes.borrow_mut();
            let index = notes
                .iter()
                .position(|n| Some(&n.name) == active.as_ref())
                .unwrap_or(0);
            notes.retain(|n| Some(&n.name) != active.as_ref());
            notes
                .get(index.min(notes.len().saturating_sub(1)))
                .map(|n| n.name.clone())
        };
        self.active.set(next.clone())?;
        self.sync_tabs(ui)?;
        if let Some(name) = next {
            ui.reveal_child(&self.tabs.borrow().clone().unwrap(), &name)?;
        }
        Ok(())
    }
    fn request_close(&self, ui: &mut Runtime) -> UiResult {
        if let Some(note) = self.current()? {
            if note.text.get()? != note.saved.get()? {
                self.close_dialog.take_outcome()?;
                self.close_dialog.open()?;
            } else {
                self.remove_current(ui)?;
            }
        }
        Ok(())
    }
    fn new_note(&self, ui: &mut Runtime) -> UiResult {
        let input = self.draft.get()?;
        let invalid = if !(1..=64).contains(&input.chars().count()) {
            Some("Use a name of 1 to 64 characters.")
        } else if input.contains(['/', '\\']) || input.starts_with('.') {
            Some("Names cannot start with . or contain / or \\.")
        } else {
            None
        };
        if let Some(reason) = invalid {
            return self.error.set(reason.into());
        }
        let name = if input.ends_with(".txt") {
            input
        } else {
            format!("{input}.txt")
        };
        match fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(self.folder.join(&name))
        {
            Ok(_) => {
                self.names.update(|names| {
                    names.push(name.clone());
                    names.sort();
                })?;
                self.new_dialog.accept()?;
                self.open(ui, &name)
            }
            Err(e) => self
                .error
                .set(if e.kind() == std::io::ErrorKind::AlreadyExists {
                    "A note with that name already exists.".into()
                } else {
                    format!("Cannot create note: {e}")
                }),
        }
    }
}

struct NoteRows {
    names: Vec<String>,
    app: Editor,
}
impl ListItems for NoteRows {
    fn len(&self) -> usize {
        self.names.len()
    }
    fn key(&self, index: usize) -> Option<String> {
        self.names.get(index).cloned()
    }
    fn text(&self, index: usize) -> Option<String> {
        self.names.get(index).cloned()
    }
    fn build(&self, index: usize) -> Option<Element> {
        let name = self.names.get(index)?.clone();
        let app = self.app.clone();
        Some(
            Button::new(name.clone(), move |cx| {
                let (app, name) = (app.clone(), name.clone());
                let _ = cx.defer(move |ui| app.open(ui, &name))?;
                Ok(())
            })
            .key("note"),
        )
    }
}

fn build(ui: &mut Runtime, folder: PathBuf, names: Vec<String>) -> UiResult<WidgetId> {
    let app = Editor {
        folder,
        names: ui.signal(names),
        notes: Rc::default(),
        active: ui.signal(None),
        tabs: Rc::default(),
        new_dialog: DialogController::new(ui),
        close_dialog: DialogController::new(ui),
        draft: ui.signal(String::new()),
        error: ui.signal(String::new()),
        toast: ToastController::new(ui),
    };
    let a = app.clone();
    let new = ui.register_command(
        "New note",
        CommandScope::Application,
        vec![KeyChord::primary('n')],
        move |_, _| {
            a.draft.set(String::new())?;
            a.error.set(String::new())?;
            a.new_dialog.take_outcome()?;
            a.new_dialog.open()
        },
    )?;
    let a = app.clone();
    let save = ui.register_command(
        "Save",
        CommandScope::Application,
        vec![KeyChord::primary('s')],
        move |_, _| a.save().map(|_| ()),
    )?;
    let a = app.clone();
    let close = ui.register_command(
        "Close tab",
        CommandScope::Application,
        vec![KeyChord::primary('w')],
        move |cx, _| {
            let a = a.clone();
            let _ = cx.defer(move |ui| a.request_close(ui))?;
            Ok(())
        },
    )?;
    let a = app.clone();
    let source = app.names.map(move |names| {
        ListSource::new(NoteRows {
            names: names.clone(),
            app: a.clone(),
        })
    })?;
    let a = app.clone();
    let list = VirtualList::new(source, 36.0)?
        .selection(ListSelection::new(ui, SelectionMode::Single))
        .on_activate(move |cx, name| {
            let (a, name) = (a.clone(), name.to_owned());
            let _ = cx.defer(move |ui| a.open(ui, &name))?;
            Ok(())
        });
    let a = app.clone();
    let tabs = Tabs::new([]).on_select(move |_, change| a.active.set(change.selected.clone()));
    let a = app.clone();
    let status = Dynamic::new(&app.active, move |_, active| {
        if let Some(note) = a
            .notes
            .borrow()
            .iter()
            .find(|n| Some(&n.name) == active.as_ref())
        {
            let name = note.name.clone();
            Label::new(
                note.text
                    .map(move |text| {
                        format!(
                            "{name}: {} lines, {} words",
                            text.matches('\n').count() + 1,
                            text.split_whitespace().count()
                        )
                    })
                    .expect("local signal"),
            )
            .key("counts")
        } else {
            Label::new("").key("counts")
        }
    });
    let body = Column::new([
        SplitPane::new(SplitAxis::LeftRight, list.key("notes"), tabs.key("tabs"))
            .sizing(SplitSizing::first(Length::from(220)))
            .key("split")
            .with_grow(1),
        status.key("status"),
    ])
    .key("body");
    let bar = MenuBar::new(
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
    .content(body);
    let a = app.clone();
    let cancel = app.new_dialog.clone();
    let new_form = Column::new([
        Label::new("New note").key("new-title"),
        TextField::new(&app.draft).key("name"),
        Label::new(app.error.read_only()).key("name-error"),
        Row::new([
            Button::new("Create", move |cx| {
                let a = a.clone();
                let _ = cx.defer(move |ui| a.new_note(ui))?;
                Ok(())
            })
            .key("create"),
            Button::new("Cancel", move |_| cancel.cancel().map(|_| ())).key("cancel-new"),
        ])
        .key("new-actions"),
    ])
    .key("new-form");
    let a = app.clone();
    let b = app.clone();
    let cancel = app.close_dialog.clone();
    let close_form = Column::new([
        Label::new("Save changes before closing?").key("close-title"),
        Row::new([
            Button::new("Save", move |cx| {
                let a = a.clone();
                let _ = cx.defer(move |ui| {
                    if a.save()? {
                        a.close_dialog.accept()?;
                        a.remove_current(ui)?;
                    }
                    Ok(())
                })?;
                Ok(())
            })
            .key("save-close"),
            Button::new("Discard", move |cx| {
                let b = b.clone();
                let _ = cx.defer(move |ui| {
                    b.close_dialog.accept()?;
                    b.remove_current(ui)
                })?;
                Ok(())
            })
            .key("discard"),
            Button::new("Cancel", move |_| cancel.cancel().map(|_| ())).key("cancel-close"),
        ])
        .key("close-actions"),
    ])
    .key("close-form");
    let root = ui.mount_tree(
        ToastHost::new(
            app.toast.clone(),
            Dialog::new(
                app.close_dialog.clone(),
                Dialog::new(app.new_dialog.clone(), bar.key("menu"), new_form).key("new-dialog"),
                close_form,
            )
            .require_response()
            .key("close-dialog"),
        )
        .key("app"),
    )?;
    *app.tabs.borrow_mut() = ui.find_descendant(&root, "tabs")?;
    Ok(root)
}

fn list_notes(folder: &std::path::Path) -> std::io::Result<Vec<String>> {
    let mut names = Vec::new();
    for entry in fs::read_dir(folder)? {
        let entry = entry?;
        if entry.file_type()?.is_file() && entry.path().extension().is_some_and(|e| e == "txt") {
            if let Some(name) = entry.file_name().to_str() {
                names.push(name.to_owned());
            }
        }
    }
    names.sort();
    Ok(names)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let folder = std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("notes"));
    fs::create_dir_all(&folder)?;
    let names = list_notes(&folder)?;
    App::new("Notes", move |ui| {
        build(ui, folder.clone(), names.clone())?;
        Ok(())
    })
    .run()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use pert_ui::testing::*;

    struct Harness {
        ui: Runtime,
        root: WidgetId,
        folder: PathBuf,
    }
    impl Harness {
        fn new() -> Self {
            let folder = PathBuf::from(format!("/work/test-notes-{}", std::process::id()));
            let _ = fs::remove_dir_all(&folder);
            fs::create_dir_all(&folder).unwrap();
            fs::write(folder.join("zebra.txt"), "last note").unwrap();
            fs::write(folder.join("alpha.txt"), "first note").unwrap();
            fs::write(folder.join("ignore.md"), "hidden").unwrap();
            let mut ui = Runtime::new();
            ui.set_viewport(Viewport::new(1000.0, 700.0, 1.0).unwrap())
                .unwrap();
            let root = build(&mut ui, folder.clone(), list_notes(&folder).unwrap()).unwrap();
            Driver::new(&mut ui).settle().unwrap();
            Self { ui, root, folder }
        }
        fn id(&self, key: &str) -> WidgetId {
            self.ui
                .find_descendant(&self.root, key)
                .unwrap()
                .unwrap_or_else(|| panic!("missing {key}"))
        }
        fn click(&mut self, key: &str) {
            let id = self.id(key);
            Driver::new(&mut self.ui).click(&id).unwrap();
            Driver::new(&mut self.ui).settle().unwrap();
        }
        fn key(&mut self, key: KeyCode) {
            Driver::new(&mut self.ui).key(key).unwrap();
        }
        fn chord(&mut self, letter: char, key: KeyCode) {
            Driver::new(&mut self.ui)
                .chord(KeyChord::primary(letter), key)
                .unwrap();
            Driver::new(&mut self.ui).settle().unwrap();
        }
        fn text(&mut self, key: &str, text: &str) {
            let id = self.id(key);
            Driver::new(&mut self.ui).replace_text(&id, text).unwrap();
        }
        fn name(&mut self, key: &str) -> String {
            let id = self.id(key);
            Driver::new(&mut self.ui)
                .semantics(&id)
                .unwrap()
                .name
                .unwrap_or_default()
        }
        fn editor(&self, name: &str) -> WidgetId {
            let tabs = self.id("tabs");
            let tab = self.ui.find(&tabs, &[name]).unwrap().unwrap();
            self.ui.find_descendant(&tab, "editor").unwrap().unwrap()
        }
        fn edit(&mut self, name: &str, text: &str) {
            let id = self.editor(name);
            Driver::new(&mut self.ui).replace_text(&id, text).unwrap();
        }
        fn tab_count(&self) -> usize {
            self.ui.children(&self.id("tabs")).unwrap().len()
        }
        fn menu(&mut self, steps: usize) {
            self.key(KeyCode::F10);
            self.key(KeyCode::ArrowDown);
            for _ in 0..steps {
                self.key(KeyCode::ArrowDown);
            }
            self.key(KeyCode::Enter);
            Driver::new(&mut self.ui).settle().unwrap();
        }
        fn visible(&self, key: &str) -> bool {
            self.ui
                .inspect()
                .components
                .iter()
                .any(|c| c.key == key && c.visible)
        }
        fn title(&mut self, name: &str) -> String {
            let tab = self.ui.find(&self.id("tabs"), &[name]).unwrap().unwrap();
            Driver::new(&mut self.ui)
                .semantics(&tab)
                .unwrap()
                .name
                .unwrap_or_default()
        }
    }
    impl Drop for Harness {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.folder);
        }
    }

    #[test]
    fn headless_acceptance() {
        let mut h = Harness::new();
        let list = h.id("notes");
        let rows = h.ui.children(&list).unwrap();
        let row_keys: Vec<_> = rows
            .iter()
            .map(|id| h.ui.key(id).unwrap().to_owned())
            .collect();
        assert_eq!(row_keys, ["alpha.txt", "zebra.txt"]);
        // Pointer opens the first note. Keyboard selects and activates the second.
        let row = h.ui.find(&list, &["alpha.txt"]).unwrap().unwrap();
        Driver::new(&mut h.ui).click(&row).unwrap();
        Driver::new(&mut h.ui).settle().unwrap();
        assert_eq!(h.tab_count(), 1);
        h.ui.focus(&list).unwrap();
        h.key(KeyCode::Home);
        h.key(KeyCode::ArrowDown);
        h.key(KeyCode::Enter);
        Driver::new(&mut h.ui).settle().unwrap();
        assert_eq!(h.tab_count(), 2);
        let row = h.ui.find(&list, &["alpha.txt"]).unwrap().unwrap();
        Driver::new(&mut h.ui).click(&row).unwrap();
        Driver::new(&mut h.ui).settle().unwrap();
        assert_eq!(h.tab_count(), 2);
        assert!(h.name("counts").starts_with("alpha.txt:"));
        h.edit("alpha.txt", "one two\nthree\n");
        assert_eq!(h.title("alpha.txt"), "alpha.txt *");
        assert_eq!(h.name("counts"), "alpha.txt: 3 lines, 3 words");
        h.chord('s', KeyCode::KeyS);
        assert_eq!(
            fs::read_to_string(h.folder.join("alpha.txt")).unwrap(),
            "one two\nthree\n"
        );
        assert_eq!(h.title("alpha.txt"), "alpha.txt");
        h.edit("alpha.txt", "menu saved");
        h.menu(1);
        assert_eq!(
            fs::read_to_string(h.folder.join("alpha.txt")).unwrap(),
            "menu saved"
        );
        assert_eq!(h.title("alpha.txt"), "alpha.txt");

        h.menu(0);
        assert!(h.visible("new-form"));
        for (name, explanation) in [
            ("", "1 to 64"),
            ("bad/name", "cannot"),
            ("bad\\name", "cannot"),
            (".hidden", "cannot"),
            ("alpha", "already exists"),
            ("alpha.txt", "already exists"),
        ] {
            h.text("name", name);
            h.click("create");
            assert!(h.visible("new-form"));
            assert!(h.name("name-error").contains(explanation));
            assert_eq!(fs::read_dir(&h.folder).unwrap().count(), 3);
        }
        h.text("name", "fresh");
        h.click("create");
        assert!(!h.visible("new-form"));
        assert!(h.folder.join("fresh.txt").exists());
        assert_eq!(h.tab_count(), 3);
        assert_eq!(h.name("counts"), "fresh.txt: 1 lines, 0 words");
        let rows: Vec<_> =
            h.ui.children(&list)
                .unwrap()
                .iter()
                .map(|id| h.ui.key(id).unwrap().to_owned())
                .collect();
        assert_eq!(rows, ["alpha.txt", "fresh.txt", "zebra.txt"]);
        // The shortcut opens the same New note form as the menu, and can be cancelled.
        h.chord('n', KeyCode::KeyN);
        assert!(h.visible("new-form"));
        h.text("name", &"x".repeat(65));
        h.click("create");
        assert!(h.name("name-error").contains("1 to 64"));
        h.click("cancel-new");
        assert!(!h.visible("new-form"));
        h.edit("fresh.txt", "keep this");
        h.chord('w', KeyCode::KeyW);
        assert!(h.visible("close-form"));
        h.click("cancel-close");
        assert!(!h.visible("close-form"));
        assert_eq!(h.tab_count(), 3);
        let editor = h.editor("fresh.txt");
        assert_eq!(
            Driver::new(&mut h.ui).semantics(&editor).unwrap().value,
            Some(SemanticValue::Text("keep this".into()))
        );
        h.chord('w', KeyCode::KeyW);
        h.click("discard");
        assert_eq!(h.tab_count(), 2);
        assert_eq!(fs::read_to_string(h.folder.join("fresh.txt")).unwrap(), "");
        h.edit("zebra.txt", "save on close");
        h.chord('w', KeyCode::KeyW);
        h.click("save-close");
        assert_eq!(h.tab_count(), 1);
        assert_eq!(
            fs::read_to_string(h.folder.join("zebra.txt")).unwrap(),
            "save on close"
        );
        h.menu(2);
        assert_eq!(h.tab_count(), 0);
        assert!(!h.visible("close-form"));
        assert_eq!(h.name("counts"), "");

        // A CPU capture with two tabs and a modified note, before inducing failure.
        for name in ["alpha.txt", "zebra.txt"] {
            let row = h.ui.find(&list, &[name]).unwrap().unwrap();
            Driver::new(&mut h.ui).click(&row).unwrap();
            Driver::new(&mut h.ui).settle().unwrap();
        }
        h.edit(
            "zebra.txt",
            "Ideas for tomorrow\n\nWrite a small notes editor.\nTry the CPU renderer.",
        );
        h.ui.set_capture_support(true);
        let pending =
            h.ui.request_capture(CaptureRequest::new(CaptureTarget::window(
                &h.ui.primary_window(),
            )));
        Driver::new(&mut h.ui).settle().unwrap();
        CpuRenderer::new().serve_captures(&mut h.ui);
        let capture = pending.wait().unwrap().value.unwrap();
        fs::write("/work/notes-editor.png", capture.image.encode_png()).unwrap();
        fs::remove_dir_all(&h.folder).unwrap();
        h.chord('s', KeyCode::KeyS);
        assert_eq!(h.title("zebra.txt"), "zebra.txt *");
        let toasts = h.id("toasts");
        assert!(h.visible("toasts"));
        assert_eq!(
            Driver::new(&mut h.ui).semantics(&toasts).unwrap().value,
            Some(SemanticValue::Number {
                value: 1.0,
                minimum: Some(0.0),
                maximum: None
            })
        );
        h.chord('w', KeyCode::KeyW);
        h.click("save-close");
        assert_eq!(h.tab_count(), 2);
        assert!(h.visible("close-form"));
        h.click("cancel-close");
    }
}
