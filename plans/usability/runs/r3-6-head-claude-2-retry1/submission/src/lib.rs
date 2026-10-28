//! A small editor for plain-text notes kept as `.txt` files in one folder.

use pert_ui::prelude::*;
use pert_ui::windowing::*;
use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;

/// Key paths a test reaches, from the mounted root.
pub mod keys {
    pub const BAR: &str = "bar";
    pub const SPLIT: &str = "split";
    pub const SIDEBAR: &str = "notes";
    pub const EDITOR: &str = "editor";
    pub const TABS: &str = "tabs";
    pub const STATUS: &str = "status";
    pub const TEXT: &str = "text";
    pub const NAME: &str = "name";
    pub const NAME_ERROR: &str = "name-error";
    pub const CREATE: &str = "create";
    pub const SAVE: &str = "save";
    pub const DISCARD: &str = "discard";
    pub const CANCEL: &str = "cancel";
}

/// The application state the initializer returns, which tests read.
pub struct Notes {
    pub root: WidgetId,
    pub shared: Rc<Shared>,
}

impl Notes {
    /// Path from the root to the content beneath the dialogs and toasts.
    pub fn content_path() -> Vec<&'static str> {
        vec!["new-dialog", "close-dialog", "main", keys::BAR]
    }
}

/// One open note.
pub struct Doc {
    pub name: String,
    pub text: Signal<String>,
    pub saved: Signal<String>,
    title: ReadSignal<String>,
    status: ReadSignal<String>,
}

pub struct Shared {
    pub dir: PathBuf,
    pub files: Signal<Vec<String>>,
    pub docs: RefCell<Vec<Doc>>,
    pub active: Signal<Option<String>>,
    pub toasts: ToastController,
    pub new_dialog: DialogController,
    pub new_name: Signal<String>,
    pub new_error: Signal<String>,
    pub close_dialog: DialogController,
    pub close_target: RefCell<Option<String>>,
    pub close_message: Signal<String>,
    tabs: RefCell<Option<WidgetId>>,
}

/// The `.txt` file names in `dir`, sorted.
pub fn list_notes(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .map(|entries| {
            entries
                .filter_map(Result::ok)
                .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_file()))
                .filter_map(|entry| entry.file_name().into_string().ok())
                .filter(|name| name.ends_with(".txt"))
                .collect()
        })
        .unwrap_or_default();
    names.sort();
    names
}

/// Checks a typed note name and returns the file name it creates.
pub fn note_file_name(typed: &str) -> Result<String, String> {
    let count = typed.chars().count();
    if count == 0 {
        return Err("Enter a name.".into());
    }
    if count > 64 {
        return Err("A name has at most 64 characters.".into());
    }
    if typed.contains('/') || typed.contains('\\') {
        return Err("A name must not contain / or \\.".into());
    }
    if typed.starts_with('.') {
        return Err("A name must not start with a dot.".into());
    }
    Ok(if typed.ends_with(".txt") { typed.to_owned() } else { format!("{typed}.txt") })
}

/// `<name>: L lines, W words`.
pub fn status_text(name: &str, text: &str) -> String {
    let lines = text.matches('\n').count() + 1;
    let words = text.split_whitespace().count();
    format!("{name}: {lines} lines, {words} words")
}

struct FileRows {
    names: Vec<String>,
    shared: Rc<Shared>,
}

impl ListItems for FileRows {
    fn len(&self) -> usize {
        self.names.len()
    }
    fn key(&self, index: usize) -> Option<String> {
        self.names.get(index).cloned()
    }
    fn build(&self, index: usize) -> Option<Element> {
        let name = self.names.get(index)?.clone();
        let shared = self.shared.clone();
        let target = name.clone();
        Some(
            Button::new(name.clone(), move |cx| {
                let (shared, target) = (shared.clone(), target.clone());
                cx.defer(move |ui| open_note(ui, &shared, &target))?;
                Ok(())
            })
            .frameless()
            .key("open"),
        )
    }
    fn index_of(&self, key: &str) -> Option<usize> {
        self.names.iter().position(|name| name == key)
    }
}

fn tab_elements(shared: &Shared) -> Vec<Element> {
    shared
        .docs
        .borrow()
        .iter()
        .map(|doc| {
            Tab::new(
                doc.name.clone(),
                doc.title.clone(),
                Element::new(keys::TEXT, TextArea::new(doc.text.clone())),
            )
            .fixed()
            .into_element()
        })
        .collect()
}

fn refresh_tabs(ui: &mut Runtime, shared: &Shared) -> Result<(), RuntimeError> {
    let Some(tabs) = shared.tabs.borrow().clone() else { return Ok(()) };
    ui.reconcile_children(&tabs, tab_elements(shared))
}

fn notify(shared: &Shared, message: String) -> Result<(), RuntimeError> {
    shared.toasts.push(Toast::new(message)).map(|_| ())
}

/// Opens `name` in a tab of its own, or switches to its tab when it is open.
pub fn open_note(ui: &mut Runtime, shared: &Rc<Shared>, name: &str) -> Result<(), RuntimeError> {
    let is_open = shared.docs.borrow().iter().any(|doc| doc.name == name);
    if !is_open {
        let content = match std::fs::read_to_string(shared.dir.join(name)) {
            Ok(content) => content,
            Err(error) => return notify(shared, format!("Could not open {name}: {error}")),
        };
        let text = ui.signal(content.clone());
        let saved = ui.signal(content);
        let label = name.to_owned();
        let title = ReadSignal::combine((&text, &saved), move |(text, saved)| {
            if text == saved { label.clone() } else { format!("{label} *") }
        })?;
        let label = name.to_owned();
        let status = text.map(move |text| status_text(&label, text))?;
        shared.docs.borrow_mut().push(Doc { name: name.to_owned(), text, saved, title, status });
        refresh_tabs(ui, shared)?;
    }
    if let Some(tabs) = shared.tabs.borrow().clone() {
        ui.reveal_child(&tabs, name)?;
    }
    shared.active.set(Some(name.to_owned()))
}

/// Writes the note's text to its file; returns whether it succeeded.
fn save_doc(shared: &Shared, name: &str) -> Result<bool, RuntimeError> {
    let docs = shared.docs.borrow();
    let Some(doc) = docs.iter().find(|doc| doc.name == name) else { return Ok(false) };
    let text = doc.text.get()?;
    match std::fs::write(shared.dir.join(name), &text) {
        Ok(()) => {
            doc.saved.set(text)?;
            Ok(true)
        }
        Err(error) => {
            notify(shared, format!("Could not save {name}: {error}"))?;
            Ok(false)
        }
    }
}

fn close_doc(ui: &mut Runtime, shared: &Shared, name: &str) -> Result<(), RuntimeError> {
    shared.docs.borrow_mut().retain(|doc| doc.name != name);
    refresh_tabs(ui, shared)?;
    if shared.docs.borrow().is_empty() {
        shared.active.set(None)?;
    }
    Ok(())
}

fn is_modified(shared: &Shared, name: &str) -> Result<bool, RuntimeError> {
    let docs = shared.docs.borrow();
    match docs.iter().find(|doc| doc.name == name) {
        Some(doc) => Ok(doc.text.get()? != doc.saved.get()?),
        None => Ok(false),
    }
}

fn command_new(shared: &Shared) -> Result<(), RuntimeError> {
    shared.new_dialog.take_outcome()?;
    shared.new_name.set(String::new())?;
    shared.new_error.set(String::new())?;
    shared.new_dialog.open()
}

fn command_save(shared: &Shared) -> Result<(), RuntimeError> {
    if let Some(name) = shared.active.get()? {
        save_doc(shared, &name)?;
    }
    Ok(())
}

fn command_close(ui: &mut Runtime, shared: &Shared) -> Result<(), RuntimeError> {
    let Some(name) = shared.active.get()? else { return Ok(()) };
    if !is_modified(shared, &name)? {
        return close_doc(ui, shared, &name);
    }
    shared.close_dialog.take_outcome()?;
    shared.close_message.set(format!("{name} has unsaved changes."))?;
    *shared.close_target.borrow_mut() = Some(name);
    shared.close_dialog.open()
}

fn create_note(ui: &mut Runtime, shared: &Rc<Shared>) -> Result<(), RuntimeError> {
    let file = match note_file_name(&shared.new_name.get()?) {
        Ok(file) => file,
        Err(message) => return shared.new_error.set(message),
    };
    if shared.files.get()?.contains(&file) || shared.dir.join(&file).exists() {
        return shared.new_error.set(format!("A note named {file} already exists."));
    }
    if let Err(error) = std::fs::write(shared.dir.join(&file), "") {
        return shared.new_error.set(format!("Could not create {file}: {error}"));
    }
    shared.new_dialog.accept()?;
    shared.files.update(|files| {
        files.push(file.clone());
        files.sort();
    })?;
    open_note(ui, shared, &file)
}

fn new_note_dialog(shared: &Rc<Shared>, owner: Element) -> Dialog {
    let (submit, create, cancel) = (shared.clone(), shared.clone(), shared.clone());
    let content = Column::new([
        Label::new("New note").style(TextStyle::Heading).key("heading"),
        Field::new(
            TextField::new(shared.new_name.clone())
                .on_submit(move |cx, _| {
                    let shared = submit.clone();
                    cx.defer(move |ui| create_note(ui, &shared))?;
                    Ok(())
                })
                .key(keys::NAME),
        )
        .label("Name")
        .key("name-field"),
        Label::new(shared.new_error.read_only())
            .color_role(ColorRole::Error)
            .key(keys::NAME_ERROR),
        Row::new([
            Button::new("Cancel", move |_| cancel.new_dialog.cancel().map(|_| ())).key(keys::CANCEL),
            Button::new("Create", move |cx| {
                let shared = create.clone();
                cx.defer(move |ui| create_note(ui, &shared))?;
                Ok(())
            })
            .key(keys::CREATE),
        ])
        .key("actions"),
    ]);
    Dialog::new(shared.new_dialog.clone(), owner, content.key("new-note"))
}

fn close_tab_dialog(shared: &Rc<Shared>, owner: Element) -> Dialog {
    let (save, discard, cancel) = (shared.clone(), shared.clone(), shared.clone());
    let content = Column::new([
        Label::new("Close tab").style(TextStyle::Heading).key("heading"),
        Label::new(shared.close_message.read_only()).key("message"),
        Row::new([
            Button::new("Save", move |cx| {
                let shared = save.clone();
                shared.close_dialog.accept()?;
                cx.defer(move |ui| {
                    let Some(name) = shared.close_target.borrow_mut().take() else { return Ok(()) };
                    if save_doc(&shared, &name)? {
                        close_doc(ui, &shared, &name)?;
                    }
                    Ok(())
                })?;
                Ok(())
            })
            .key(keys::SAVE),
            Button::new("Discard", move |cx| {
                let shared = discard.clone();
                shared.close_dialog.accept()?;
                cx.defer(move |ui| {
                    let Some(name) = shared.close_target.borrow_mut().take() else { return Ok(()) };
                    close_doc(ui, &shared, &name)
                })?;
                Ok(())
            })
            .key(keys::DISCARD),
            Button::new("Cancel", move |_| {
                cancel.close_target.borrow_mut().take();
                cancel.close_dialog.cancel().map(|_| ())
            })
            .key(keys::CANCEL),
        ])
        .key("actions"),
    ]);
    Dialog::new(shared.close_dialog.clone(), owner, content.key("close-tab"))
}

/// The application on the notes folder `dir`.
pub fn app(dir: PathBuf) -> App<Notes> {
    App::new("Notes", move |ui| {
        let shared = Rc::new(Shared {
            files: ui.signal(list_notes(&dir)),
            dir,
            docs: RefCell::new(Vec::new()),
            active: ui.signal(None),
            toasts: ToastController::new(ui),
            new_dialog: DialogController::new(ui),
            new_name: ui.signal(String::new()),
            new_error: ui.signal(String::new()),
            close_dialog: DialogController::new(ui),
            close_target: RefCell::new(None),
            close_message: ui.signal(String::new()),
            tabs: RefCell::new(None),
        });

        let rows = shared.clone();
        let source = shared.files.map(move |names| {
            ListSource::new(FileRows { names: names.clone(), shared: rows.clone() })
        })?;
        let opener = shared.clone();
        let sidebar = VirtualList::new(source, 32.0)?
            .selection(ListSelection::new(ui, SelectionMode::Single))
            .item_tab_stops(false)
            .on_activate(move |cx, key| {
                let (shared, key) = (opener.clone(), key.to_owned());
                cx.defer(move |ui| open_note(ui, &shared, &key))?;
                Ok(())
            });

        let selected = shared.clone();
        let tabs = Tabs::new([]).on_select(move |_, change| {
            selected.active.set(change.selected.clone())
        });
        let docs = shared.clone();
        let status = Dynamic::new(&shared.active, move |_, active| {
            let docs = docs.docs.borrow();
            match active.as_ref().and_then(|name| docs.iter().find(|doc| &doc.name == name)) {
                Some(doc) => Label::new(doc.status.clone()).key(doc.name.clone()),
                None => Label::new("").key("none"),
            }
        });
        let editor = Column::new([
            tabs.key(keys::TABS).with_grow(1),
            status.key(keys::STATUS),
        ])
        .align(Align::Stretch);

        let split = SplitPane::new(
            SplitAxis::LeftRight,
            sidebar.key(keys::SIDEBAR),
            editor.key(keys::EDITOR),
        )
        .sizing(SplitSizing::first(Length::from(220)));

        let new_cmd = {
            let shared = shared.clone();
            ui.register_command("New note", CommandScope::Application,
                vec![KeyChord::primary('n')], move |_, _| command_new(&shared))?
        };
        let save_cmd = {
            let shared = shared.clone();
            ui.register_command("Save", CommandScope::Application,
                vec![KeyChord::primary('s')], move |_, _| command_save(&shared))?
        };
        let close_cmd = {
            let shared = shared.clone();
            ui.register_command("Close tab", CommandScope::Application,
                vec![KeyChord::primary('w')], move |ui, _| command_close(ui, &shared))?
        };
        let bar = MenuBar::new(ui, vec![MenuItem::submenu("file", "File", vec![
            MenuItem::command(ui, "new", &new_cmd, |_, _| Ok(()))?,
            MenuItem::command(ui, "save", &save_cmd, |_, _| Ok(()))?,
            MenuItem::separator(),
            MenuItem::command(ui, "close", &close_cmd, |_, _| Ok(()))?,
        ])])?
        .content(split.key(keys::SPLIT));

        let main = Column::new([bar.key(keys::BAR).with_grow(1)]).align(Align::Stretch);
        let close = close_tab_dialog(&shared, main.key("main"));
        let new = new_note_dialog(&shared, close.key("close-dialog"));
        let root = ui.mount_tree(Element::new(
            "app",
            ToastHost::new(shared.toasts.clone(), new.key("new-dialog")),
        ))?;
        let tabs_path = [&Notes::content_path()[..], &[keys::SPLIT, keys::EDITOR, keys::TABS]].concat();
        let tabs_id = ui.find(&root, &tabs_path)?;
        *shared.tabs.borrow_mut() = tabs_id;
        Ok(Notes { root, shared })
    })
}
