use pert_ui::prelude::*;
use pert_ui::windowing::*;
use pert_ui::{CommandKey, Modifiers};
use std::{cell::RefCell, path::PathBuf, rc::Rc};

fn ctrl(c: char) -> KeyChord {
    KeyChord::new(
        CommandKey::Character(c),
        Modifiers {
            control: true,
            ..Modifiers::default()
        },
    )
}

struct Files {
    names: Vec<String>,
    editor: Editor,
}
impl ListItems for Files {
    fn len(&self) -> usize {
        self.names.len()
    }
    fn key(&self, i: usize) -> Option<String> {
        self.names.get(i).cloned()
    }
    fn text(&self, i: usize) -> Option<String> {
        self.names.get(i).cloned()
    }
    fn build(&self, i: usize) -> Option<Element> {
        let name = self.names.get(i)?.clone();
        let editor = self.editor.clone();
        Some(Button::new(name.clone(), move |cx| editor.open(cx, &name)).key("note"))
    }
}

type UiResult<T = ()> = Result<T, RuntimeError>;

#[derive(Clone)]
struct Note {
    name: String,
    text: Signal<String>,
    saved: Signal<String>,
    title: ReadSignal<String>,
    status: ReadSignal<String>,
}
impl PartialEq for Note {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}
#[derive(Clone)]
struct Editor {
    folder: PathBuf,
    files: Signal<Vec<String>>,
    notes: Signal<Vec<Note>>,
    active: Signal<Option<String>>,
    new_dialog: DialogController,
    close_dialog: DialogController,
    name: Signal<String>,
    error: Signal<String>,
    notification: Signal<String>,
    pending_close: Signal<Option<String>>,
    root: Rc<RefCell<Option<WidgetId>>>,
}
impl Editor {
    fn active_note(&self) -> UiResult<Option<Note>> {
        let active = self.active.get()?;
        Ok(self
            .notes
            .get()?
            .into_iter()
            .find(|n| Some(&n.name) == active.as_ref()))
    }
    fn open(&self, cx: &mut EventContext, name: &str) -> UiResult {
        if !self.notes.get()?.iter().any(|n| n.name == name) {
            let text = match std::fs::read_to_string(self.folder.join(name)) {
                Ok(text) => text,
                Err(e) => return self.notification.set(format!("Could not open {name}: {e}")),
            };
            let text_signal = cx.signal(text.clone());
            let saved = cx.signal(text);
            let title_name = name.to_owned();
            let title = ReadSignal::combine((&text_signal, &saved), move |(text, saved)| {
                format!("{}{}", title_name, if text != saved { " *" } else { "" })
            })?;
            let status_name = name.to_owned();
            let status = text_signal.map(move |text| {
                format!(
                    "{}: {} lines, {} words",
                    status_name,
                    text.chars().filter(|c| *c == '\n').count() + 1,
                    text.split_whitespace().count()
                )
            })?;
            self.notes.update(|notes| {
                notes.push(Note {
                    name: name.into(),
                    text: text_signal,
                    saved,
                    title,
                    status,
                })
            })?;
        }
        self.active.set(Some(name.into()))?;
        // The first deferred action allows Dynamic to mount a newly added tab.
        let root = self.root.clone();
        let name = name.to_owned();
        cx.defer(move |ui| {
            ui.defer(move |ui| {
                if let Some(root) = root.borrow().as_ref() {
                    if let Some(tabs) = ui.find_descendant(root, "tabs")? {
                        ui.reveal_child(&tabs, &name)?;
                    }
                }
                Ok(())
            })?;
            Ok(())
        })?;
        Ok(())
    }
    fn save_note(&self, note: &Note) -> UiResult<bool> {
        let text = note.text.get()?;
        match std::fs::write(self.folder.join(&note.name), &text) {
            Ok(()) => {
                note.saved.set(text)?;
                self.notification.set(String::new())?;
                Ok(true)
            }
            Err(e) => {
                self.notification
                    .set(format!("Save failed for {}: {e}", note.name))?;
                Ok(false)
            }
        }
    }
    fn save(&self) -> UiResult {
        if let Some(note) = self.active_note()? {
            self.save_note(&note)?;
        }
        Ok(())
    }
    fn remove(&self, name: &str) -> UiResult {
        self.notes
            .update(|notes| notes.retain(|n| n.name != name))?;
        if self.active.get()?.as_deref() == Some(name) {
            self.active
                .set(self.notes.get()?.last().map(|n| n.name.clone()))?;
        }
        Ok(())
    }
    fn close(&self) -> UiResult {
        if let Some(note) = self.active_note()? {
            if note.text.get()? != note.saved.get()? {
                self.pending_close.set(Some(note.name))?;
                self.close_dialog.take_outcome()?;
                self.close_dialog.open()?;
            } else {
                self.remove(&note.name)?;
            }
        }
        Ok(())
    }
    fn new_note(&self) -> UiResult {
        self.new_dialog.take_outcome()?;
        self.name.set(String::new())?;
        self.error.set(String::new())?;
        self.new_dialog.open()
    }
    fn create(&self, cx: &mut EventContext) -> UiResult {
        let name = self.name.get()?;
        let count = name.chars().count();
        if !(1..=64).contains(&count) || name.contains(['/', '\\']) || name.starts_with('.') {
            return self
                .error
                .set("Use 1–64 characters, no / or \\, and do not start with a dot.".into());
        }
        let name = if name.ends_with(".txt") {
            name
        } else {
            format!("{name}.txt")
        };
        match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(self.folder.join(&name))
        {
            Ok(_) => {}
            Err(e) => {
                return self
                    .error
                    .set(if e.kind() == std::io::ErrorKind::AlreadyExists {
                        "A note with that name already exists.".into()
                    } else {
                        format!("Could not create note: {e}")
                    });
            }
        }
        self.files.update(|files| {
            files.push(name.clone());
            files.sort();
        })?;
        self.new_dialog.accept()?;
        self.open(cx, &name)
    }
}

fn build(ui: &mut Runtime, folder: PathBuf) -> UiResult<Editor> {
    let mut files: Vec<String> = match std::fs::read_dir(&folder) {
        Ok(entries) => entries
            .filter_map(Result::ok)
            .filter(|e| e.file_type().is_ok_and(|t| t.is_file()))
            .filter_map(|e| e.file_name().into_string().ok())
            .filter(|n| n.ends_with(".txt"))
            .collect(),
        Err(_) => Vec::new(),
    };
    files.sort();
    let ed = Editor {
        folder,
        files: ui.signal(files),
        notes: ui.signal(Vec::new()),
        active: ui.signal(None),
        new_dialog: DialogController::new(ui),
        close_dialog: DialogController::new(ui),
        name: ui.signal(String::new()),
        error: ui.signal(String::new()),
        notification: ui.signal(String::new()),
        pending_close: ui.signal(None),
        root: Rc::new(RefCell::new(None)),
    };
    let e = ed.clone();
    let new = ui.register_command(
        "New note",
        CommandScope::Application,
        vec![ctrl('n')],
        move |_, _| e.new_note(),
    )?;
    let e = ed.clone();
    let save = ui.register_command(
        "Save",
        CommandScope::Application,
        vec![ctrl('s')],
        move |_, _| e.save(),
    )?;
    let e = ed.clone();
    let close = ui.register_command(
        "Close tab",
        CommandScope::Application,
        vec![ctrl('w')],
        move |_, _| e.close(),
    )?;

    let e = ed.clone();
    let source = ed.files.map(move |names| {
        ListSource::new(Files {
            names: names.clone(),
            editor: e.clone(),
        })
    })?;
    let e = ed.clone();
    let sidebar = VirtualList::new(source, 34.0)?
        .selection(ListSelection::new(ui, SelectionMode::Single))
        .on_activate(move |cx, name| e.open(cx, name))
        .key("files");
    let active = ed.active.clone();
    let tabs = Dynamic::new(ed.notes.read_only(), move |_, notes| {
        let active = active.clone();
        Tabs::new(notes.iter().map(|n| {
            Tab::new(
                &n.name,
                n.title.clone(),
                TextArea::new(n.text.clone()).key("editor"),
            )
        }))
        .on_select(move |_, change| active.set(change.selected.clone()))
        .key("tabs")
    })
    .key("documents");
    let notes = ed.notes.clone();
    let status = Dynamic::new(
        ReadSignal::combine((&ed.active, &ed.notes), |(a, n)| (a.clone(), n.clone()))?,
        move |_, (active, _)| {
            let note = notes
                .get()
                .unwrap_or_default()
                .into_iter()
                .find(|n| Some(&n.name) == active.as_ref());
            match note {
                Some(n) => Label::new(n.status).key("status"),
                None => Label::new("").key("status"),
            }
        },
    )
    .key("status-host");
    let content = Column::new([
        SplitPane::new(SplitAxis::LeftRight, sidebar, tabs)
            .sizing(SplitSizing::first(Length::from(220)))
            .key("split")
            .with_grow(1),
        Label::new(ed.notification.read_only()).key("notification"),
        status,
    ])
    .key("content");
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
    .content(content)
    .key("menu");
    let e = ed.clone();
    let cancel = ed.new_dialog.clone();
    let new_form = Column::new([
        Label::new("New note").key("title"),
        TextField::new(ed.name.clone()).key("name"),
        Label::new(ed.error.read_only()).key("error"),
        Row::new([
            Button::new("Create", move |cx| e.create(cx)).key("create"),
            Button::new("Cancel", move |_| cancel.cancel().map(|_| ())).key("cancel"),
        ])
        .key("actions"),
    ])
    .key("new-form");
    let e = ed.clone();
    let discard = ed.clone();
    let cancel = ed.close_dialog.clone();
    let close_form = Column::new([
        Label::new("Save changes before closing?").key("title"),
        Row::new([
            Button::new("Save", move |_| {
                if let Some(name) = e.pending_close.get()? {
                    if let Some(note) = e.notes.get()?.into_iter().find(|n| n.name == name) {
                        if e.save_note(&note)? {
                            e.remove(&name)?;
                            e.close_dialog.accept()?;
                        }
                    }
                }
                Ok(())
            })
            .key("save"),
            Button::new("Discard", move |_| {
                if let Some(name) = discard.pending_close.get()? {
                    discard.remove(&name)?;
                }
                discard.close_dialog.accept().map(|_| ())
            })
            .key("discard"),
            Button::new("Cancel", move |_| cancel.cancel().map(|_| ())).key("cancel"),
        ])
        .key("actions"),
    ])
    .key("close-form");
    let root = ui.mount_tree(
        Dialog::new(
            ed.close_dialog.clone(),
            Dialog::new(ed.new_dialog.clone(), bar, new_form).key("new-dialog"),
            close_form,
        )
        .key("app"),
    )?;
    *ed.root.borrow_mut() = Some(root);
    Ok(ed)
}
fn app(folder: PathBuf) -> App<Editor> {
    App::new("Notes", move |ui| Ok(build(ui, folder)?))
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let folder = std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("notes"));
    std::fs::create_dir_all(&folder)?;
    app(folder).run()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use pert_ui::testing::*;
    use std::error::Error;
    type TestResult = Result<(), Box<dyn Error>>;

    fn find(d: &Driver<'_>, root: &WidgetId, key: &str) -> WidgetId {
        d.runtime()
            .find_descendant(root, key)
            .unwrap()
            .unwrap_or_else(|| panic!("missing {key}"))
    }
    fn click(d: &mut Driver<'_>, root: &WidgetId, key: &str) {
        let id = find(d, root, key);
        d.click(&id).unwrap();
    }
    fn settle(d: &mut Driver<'_>) {
        for _ in 0..3 {
            d.settle().unwrap();
        }
    }
    fn editor(d: &Driver<'_>, root: &WidgetId, name: &str) -> WidgetId {
        let tabs = find(d, root, "tabs");
        let tab = d.find(&tabs, &[name]).unwrap();
        find(d, &tab, "editor")
    }
    fn label(d: &Driver<'_>, root: &WidgetId, key: &str) -> String {
        d.semantics(&find(d, root, key))
            .unwrap()
            .name
            .unwrap_or_default()
    }
    fn titles(d: &Driver<'_>, root: &WidgetId) -> Vec<String> {
        d.semantics(&find(d, root, "tabs"))
            .unwrap()
            .items
            .iter()
            .filter_map(|i| i.name.clone())
            .collect()
    }
    fn selected(d: &Driver<'_>, root: &WidgetId) -> Option<String> {
        d.semantics(&find(d, root, "tabs"))
            .unwrap()
            .items
            .into_iter()
            .find(|item| item.selected)
            .and_then(|item| item.name)
    }
    fn menu(d: &mut Driver<'_>, row: usize) {
        d.key(KeyCode::F10).unwrap();
        d.key(KeyCode::ArrowDown).unwrap();
        for _ in 0..row {
            d.key(KeyCode::ArrowDown).unwrap();
        }
        d.key(KeyCode::Enter).unwrap();
        settle(d);
    }
    fn open(d: &mut Driver<'_>, root: &WidgetId, name: &str) {
        let files = find(d, root, "files");
        let row = d.find(&files, &[name]).unwrap();
        d.click(&row).unwrap();
        settle(d);
    }
    fn modified(d: &Driver<'_>, root: &WidgetId, name: &str, expected: bool) {
        assert!(
            titles(d, root).contains(&format!("{name}{}", if expected { " *" } else { "" })),
            "titles: {:?}",
            titles(d, root)
        );
    }
    fn dialog_action(d: &mut Driver<'_>, root: &WidgetId, dialog: &str, action: &str) {
        let form = find(d, root, dialog);
        click(d, &form, action);
        settle(d);
    }
    #[test]
    fn acceptance_through_ui() -> TestResult {
        let folder = PathBuf::from("/work/test-notes");
        if folder.exists() {
            std::fs::remove_dir_all(&folder)?;
        }
        std::fs::create_dir_all(&folder)?;
        std::fs::write(folder.join("Beta.txt"), "Second note")?;
        std::fs::write(folder.join("Alpha.txt"), "First note")?;
        std::fs::write(folder.join("ignore.md"), "not a note")?;
        let mut app = app(folder.clone()).headless()?;
        let root = app.state().root.borrow().clone().unwrap();
        let mut d = Driver::new(app.runtime_mut());
        settle(&mut d);
        let files = find(&d, &root, "files");
        assert_eq!(
            d.address_children(&files)?
                .iter()
                .map(|r| r.segment.clone())
                .collect::<Vec<_>>(),
            ["Alpha.txt", "Beta.txt"]
        );
        assert_eq!(label(&d, &root, "status"), "");
        open(&mut d, &root, "Alpha.txt");
        assert_eq!(titles(&d, &root), ["Alpha.txt"]);
        // Drag the actual split divider using host pointer events.
        let documents = find(&d, &root, "documents");
        let left = d.runtime().bounds(&files)?.unwrap();
        let right = d.runtime().bounds(&documents)?.unwrap();
        let x = (left.x() + left.width() + right.x()) / 2.0;
        let y = left.y() + left.height() / 2.0;
        for (x, kind) in [
            (x, PointerKind::Press(PointerButton::Primary)),
            (x + 48.0, PointerKind::Move),
            (x + 48.0, PointerKind::Release(PointerButton::Primary)),
        ] {
            d.runtime_mut()
                .pointer_input(PointerEvent::new(Point::new(x, y)?, kind))?;
            d.settle()?;
        }
        assert!(d.runtime().bounds(&files)?.unwrap().width() > left.width() + 40.0);
        // Arrow navigation and Enter open the second note through the list.
        d.runtime_mut().focus(&files)?;
        d.key(KeyCode::ArrowDown)?;
        d.key(KeyCode::ArrowDown)?;
        d.key(KeyCode::Enter)?;
        settle(&mut d);
        assert_eq!(titles(&d, &root), ["Alpha.txt", "Beta.txt"]);
        assert_eq!(selected(&d, &root).as_deref(), Some("Beta.txt"));
        assert_eq!(label(&d, &root, "status"), "Beta.txt: 1 lines, 2 words");
        open(&mut d, &root, "Alpha.txt");
        assert_eq!(titles(&d, &root).len(), 2);
        assert_eq!(selected(&d, &root).as_deref(), Some("Alpha.txt"));
        assert_eq!(label(&d, &root, "status"), "Alpha.txt: 1 lines, 2 words");
        let field = editor(&d, &root, "Alpha.txt");
        d.replace_text(&field, "one two\nthree")?;
        modified(&d, &root, "Alpha.txt", true);
        assert_eq!(label(&d, &root, "status"), "Alpha.txt: 2 lines, 3 words");
        d.chord(ctrl('s'), KeyCode::KeyS)?;
        assert_eq!(
            std::fs::read_to_string(folder.join("Alpha.txt"))?,
            "one two\nthree"
        );
        modified(&d, &root, "Alpha.txt", false);
        d.replace_text(&field, "saved through menu")?;
        menu(&mut d, 1);
        assert_eq!(
            std::fs::read_to_string(folder.join("Alpha.txt"))?,
            "saved through menu"
        );
        modified(&d, &root, "Alpha.txt", false);
        d.replace_text(&field, "CPU capture\nAn unfinished thought.")?;
        modified(&d, &root, "Alpha.txt", true);
        // Capture the same application driven by the checks.
        d.runtime().set_capture_support(true);
        let pending = d
            .runtime()
            .request_capture(CaptureRequest::new(CaptureTarget::window(
                &d.runtime().primary_window(),
            )));
        settle(&mut d);
        CpuRenderer::new().serve_captures(d.runtime_mut());
        let capture = pending.try_reply()?.value?;
        std::fs::write("/work/notes-editor.png", capture.image.encode_png())?;

        // New dialog validation: each refusal leaves files and tabs unchanged.
        d.chord(ctrl('n'), KeyCode::KeyN)?;
        for (name, explanation) in [
            ("", "1–64"),
            ("bad/name", "no /"),
            ("Alpha", "already exists"),
            (".secret", "dot"),
            ("bad\\name", "no /"),
            (&"a".repeat(65), "1–64"),
        ] {
            let form = find(&d, &root, "new-form");
            let name_field = find(&d, &form, "name");
            d.replace_text(&name_field, name)?;
            click(&mut d, &form, "create");
            assert!(label(&d, &form, "error").contains(explanation));
            assert_eq!(std::fs::read_dir(&folder)?.count(), 3);
            assert_eq!(titles(&d, &root).len(), 2);
        }
        let form = find(&d, &root, "new-form");
        let name_field = find(&d, &form, "name");
        d.replace_text(&name_field, "Gamma")?;
        click(&mut d, &form, "create");
        settle(&mut d);
        assert_eq!(std::fs::read_to_string(folder.join("Gamma.txt"))?, "");
        assert_eq!(titles(&d, &root), ["Alpha.txt *", "Beta.txt", "Gamma.txt"]);
        assert_eq!(label(&d, &root, "status"), "Gamma.txt: 1 lines, 0 words");
        assert_eq!(selected(&d, &root).as_deref(), Some("Gamma.txt"));
        assert_eq!(
            d.address_children(&files)?
                .iter()
                .map(|r| r.segment.clone())
                .collect::<Vec<_>>(),
            ["Alpha.txt", "Beta.txt", "Gamma.txt"]
        );
        // Unmodified close has no modal; New remains operable afterwards.
        d.chord(ctrl('w'), KeyCode::KeyW)?;
        settle(&mut d);
        assert_eq!(titles(&d, &root).len(), 2);
        open(&mut d, &root, "Alpha.txt");
        d.chord(ctrl('w'), KeyCode::KeyW)?;
        dialog_action(&mut d, &root, "close-form", "cancel");
        modified(&d, &root, "Alpha.txt", true);
        assert_eq!(
            d.semantics(&editor(&d, &root, "Alpha.txt"))?.value,
            Some(SemanticValue::Text(
                "CPU capture\nAn unfinished thought.".into()
            ))
        );
        assert_eq!(
            std::fs::read_to_string(folder.join("Alpha.txt"))?,
            "saved through menu"
        );
        // Menu Close uses the same dialog, and Discard does not write.
        menu(&mut d, 2);
        dialog_action(&mut d, &root, "close-form", "discard");
        assert_eq!(titles(&d, &root), ["Beta.txt"]);
        assert_eq!(
            std::fs::read_to_string(folder.join("Alpha.txt"))?,
            "saved through menu"
        );
        open(&mut d, &root, "Alpha.txt");
        d.replace_text(&editor(&d, &root, "Alpha.txt"), "saved on close")?;
        d.chord(ctrl('w'), KeyCode::KeyW)?;
        dialog_action(&mut d, &root, "close-form", "save");
        assert_eq!(titles(&d, &root), ["Beta.txt"]);
        assert_eq!(
            std::fs::read_to_string(folder.join("Alpha.txt"))?,
            "saved on close"
        );
        // Failure is deterministic even when running as a privileged user.
        open(&mut d, &root, "Alpha.txt");
        d.replace_text(&editor(&d, &root, "Alpha.txt"), "must survive failed save")?;
        let moved = PathBuf::from("/work/test-notes-moved");
        if moved.exists() {
            std::fs::remove_dir_all(&moved)?;
        }
        std::fs::rename(&folder, &moved)?;
        d.chord(ctrl('s'), KeyCode::KeyS)?;
        assert!(label(&d, &root, "notification").contains("Save failed"));
        modified(&d, &root, "Alpha.txt", true);
        d.chord(ctrl('w'), KeyCode::KeyW)?;
        dialog_action(&mut d, &root, "close-form", "save");
        modified(&d, &root, "Alpha.txt", true);
        assert!(label(&d, &root, "notification").contains("Save failed"));
        dialog_action(&mut d, &root, "close-form", "cancel");
        std::fs::rename(&moved, &folder)?;
        d.chord(ctrl('s'), KeyCode::KeyS)?;
        modified(&d, &root, "Alpha.txt", false);
        d.chord(ctrl('w'), KeyCode::KeyW)?;
        settle(&mut d);
        d.chord(ctrl('w'), KeyCode::KeyW)?;
        settle(&mut d);
        assert!(titles(&d, &root).is_empty());
        // New note is also reachable through File, with the same form.
        menu(&mut d, 0);
        let form = find(&d, &root, "new-form");
        let name_field = find(&d, &form, "name");
        d.replace_text(&name_field, "Menu.txt")?;
        click(&mut d, &form, "create");
        settle(&mut d);
        assert_eq!(std::fs::read_to_string(folder.join("Menu.txt"))?, "");
        assert_eq!(selected(&d, &root).as_deref(), Some("Menu.txt"));
        d.chord(ctrl('w'), KeyCode::KeyW)?;
        settle(&mut d);
        assert!(titles(&d, &root).is_empty());
        assert_eq!(label(&d, &root, "status"), "");
        Ok(())
    }
}
