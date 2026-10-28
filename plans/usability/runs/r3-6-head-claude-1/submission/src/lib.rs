//! A small editor for plain-text notes kept as `.txt` files in one folder.

use pert_ui::prelude::*;
use pert_ui::windowing::{SplitAxis, SplitPane, SplitSizing};
use pert_ui::{DialogController, DialogStatus, ListSelection, SelectionMode, Toast, ToastController};
use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;

/// Width of the notes list when the window opens, in logical pixels.
const SIDEBAR_WIDTH: u16 = 220;

/// One note open in a tab.
struct OpenNote {
    name: String,
    /// The editor's text.
    text: Signal<String>,
    /// The text as last read from or written to the file.
    saved: Signal<String>,
    /// The tab title, with a modified marker.
    title: ReadSignal<String>,
    /// The status line for this note.
    status: ReadSignal<String>,
}

/// Application data that is not a binding.
struct Model {
    dir: PathBuf,
    open: Vec<OpenNote>,
    tabs: Option<WidgetId>,
    /// The note the close dialog asks about.
    pending_close: Option<String>,
}

/// Everything the callbacks and the tests reach.
#[derive(Clone)]
pub struct Notes {
    model: Rc<RefCell<Model>>,
    /// The `.txt` file names in the folder, sorted.
    pub notes: Signal<Vec<String>>,
    /// The key (file name) of the selected tab.
    pub active: Signal<Option<String>>,
    pub toasts: ToastController,
    pub new_dialog: DialogController,
    pub close_dialog: DialogController,
    new_name: Signal<String>,
    new_error: Signal<String>,
    close_message: Signal<String>,
    /// The mounted root.
    pub root: WidgetId,
}

impl Notes {
    /// The folder the notes are kept in.
    pub fn dir(&self) -> PathBuf {
        self.model.borrow().dir.clone()
    }
}

/// The `.txt` files of `dir`, sorted by name.
pub fn list_notes(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .map(|entries| {
            entries
                .filter_map(Result::ok)
                .filter(|entry| entry.file_type().map(|t| t.is_file()).unwrap_or(false))
                .filter_map(|entry| entry.file_name().into_string().ok())
                .filter(|name| name.ends_with(".txt"))
                .collect()
        })
        .unwrap_or_default();
    names.sort();
    names
}

/// `<name>: L lines, W words`.
pub fn status_text(name: &str, text: &str) -> String {
    let lines = text.matches('\n').count() + 1;
    let words = text.split_whitespace().count();
    format!("{name}: {lines} lines, {words} words")
}

/// Checks a new note's name and returns the file name, with `.txt` appended
/// when missing.
pub fn validate_name(name: &str, existing: &[String]) -> Result<String, String> {
    let length = name.chars().count();
    if length == 0 {
        return Err("Enter a name.".into());
    }
    if length > 64 {
        return Err("A name has at most 64 characters.".into());
    }
    if name.contains('/') || name.contains('\\') {
        return Err("A name cannot contain / or \\.".into());
    }
    if name.starts_with('.') {
        return Err("A name cannot start with a dot.".into());
    }
    let file = if name.ends_with(".txt") { name.to_owned() } else { format!("{name}.txt") };
    if existing.contains(&file) {
        return Err(format!("A note named {file} already exists."));
    }
    Ok(file)
}

/// The list rows: one button per note.
struct NoteRows {
    names: Vec<String>,
    shared: Rc<RefCell<Option<Notes>>>,
}

impl ListItems for NoteRows {
    fn len(&self) -> usize {
        self.names.len()
    }
    fn key(&self, index: usize) -> Option<String> {
        self.names.get(index).cloned()
    }
    fn build(&self, index: usize) -> Option<Element> {
        let name = self.names.get(index)?.clone();
        let shared = self.shared.clone();
        let label = name.clone();
        Some(
            Button::new(label, move |cx| {
                let (shared, name) = (shared.clone(), name.clone());
                cx.defer(move |ui| match shared.borrow().clone() {
                    Some(notes) => open_note(ui, &notes, &name),
                    None => Ok(()),
                })?;
                Ok(())
            })
            .frameless()
            .key("open"),
        )
    }
}

/// The application, started on the notes folder `dir`.
pub fn app(dir: PathBuf) -> App<Notes> {
    App::new("Notes", move |ui| Ok(build(ui, dir)?))
}

/// Builds the content on `ui` and returns its handles.
pub fn build(ui: &mut Runtime, dir: PathBuf) -> Result<Notes, RuntimeError> {
    let notes = ui.signal(list_notes(&dir));
    let active = ui.signal(None::<String>);
    let toasts = ToastController::new(ui);
    let new_dialog = DialogController::new(ui);
    let close_dialog = DialogController::new(ui);
    let new_name = ui.signal(String::new());
    let new_error = ui.signal(String::new());
    let close_message = ui.signal(String::new());
    let model = Rc::new(RefCell::new(Model { dir, open: Vec::new(), tabs: None, pending_close: None }));
    // Callbacks built before the handles exist reach them through this cell.
    let shared: Rc<RefCell<Option<Notes>>> = Rc::new(RefCell::new(None));

    // Commands: the menu rows and the shortcuts run the same action.
    let with = |shared: &Rc<RefCell<Option<Notes>>>, action: fn(&mut Runtime, &Notes) -> Result<(), RuntimeError>| {
        let shared = shared.clone();
        move |ui: &mut Runtime, _: &pert_ui::CommandContext| -> Result<(), RuntimeError> {
            let Some(notes) = shared.borrow().clone() else { return Ok(()) };
            if notes.dialog_open()? {
                return Ok(());
            }
            action(ui, &notes)
        }
    };
    let new_cmd = ui.register_command("New note", CommandScope::Application,
        vec![KeyChord::primary('n')], with(&shared, new_note))?;
    let save_cmd = ui.register_command("Save", CommandScope::Application,
        vec![KeyChord::primary('s')], with(&shared, |ui, notes| save_active(ui, notes).map(|_| ())))?;
    let close_cmd = ui.register_command("Close tab", CommandScope::Application,
        vec![KeyChord::primary('w')], with(&shared, close_active))?;

    // The sidebar.
    let rows_shared = shared.clone();
    let source = notes.map(move |names| {
        ListSource::new(NoteRows { names: names.clone(), shared: rows_shared.clone() })
    })?;
    let selection = ListSelection::new(ui, SelectionMode::Single);
    let activate_shared = shared.clone();
    let list = VirtualList::new(source, 32.0)
?
        .selection(selection)
        .on_activate(move |cx, key| {
            let (shared, name) = (activate_shared.clone(), key.to_owned());
            cx.defer(move |ui| match shared.borrow().clone() {
                Some(notes) => open_note(ui, &notes, &name),
                None => Ok(()),
            })?;
            Ok(())
        });

    // The tabs: described by the model, see `describe_tabs`.
    let select_active = active.clone();
    let tabs = Tabs::new([]).on_select(move |_, change| select_active.set(change.selected.clone()));

    // The status line follows the active tab and its text.
    let status_model = model.clone();
    let status = Dynamic::new(&active, move |_, active| {
        let model = status_model.borrow();
        match active.as_ref().and_then(|key| model.open.iter().find(|note| &note.name == key)) {
            Some(note) => Label::new(note.status.clone()).key(format!("note-{}", note.name)),
            None => Label::new("").key("none"),
        }
    });

    let main = Column::new([
        SplitPane::new(
            SplitAxis::LeftRight,
            Element::new("notes", list),
            Element::new("tabs", tabs),
        )
        .sizing(SplitSizing::first(Length::from(SIDEBAR_WIDTH)))
        .key("split")
        .with_grow(1),
        status.key("status"),
    ])
    .align(Align::Stretch);

    let menu = MenuBar::new(ui, vec![MenuItem::submenu("file", "File", vec![
        MenuItem::command(ui, "new", &new_cmd, |_, _| Ok(()))?.label("New note"),
        MenuItem::command(ui, "save", &save_cmd, |_, _| Ok(()))?,
        MenuItem::command(ui, "close", &close_cmd, |_, _| Ok(()))?.label("Close tab"),
    ])])
?
    .content(main.key("main"));

    // The close dialog: Save, Discard or Cancel for a modified tab.
    let (save_shared, discard_shared, cancel_dialog) = (shared.clone(), shared.clone(), close_dialog.clone());
    let close_form = Column::new([
        Label::new(close_message.read_only()).key("message"),
        Row::new([
            Button::new("Save", move |cx| {
                let shared = save_shared.clone();
                cx.defer(move |ui| match shared.borrow().clone() {
                    Some(notes) => close_pending(ui, &notes, true),
                    None => Ok(()),
                })?;
                Ok(())
            })
            .key("save"),
            Button::new("Discard", move |cx| {
                let shared = discard_shared.clone();
                cx.defer(move |ui| match shared.borrow().clone() {
                    Some(notes) => close_pending(ui, &notes, false),
                    None => Ok(()),
                })?;
                Ok(())
            })
            .key("discard"),
            Button::new("Cancel", move |_| finish(&cancel_dialog)).key("cancel"),
        ])
        .key("actions"),
    ]);
    let close = Dialog::new(close_dialog.clone(), menu.key("menu"), close_form.key("close-form"));

    // The New note dialog.
    let (create_shared, cancel_new) = (shared.clone(), new_dialog.clone());
    let new_form = Column::new([
        Label::new("Name of the new note").key("prompt"),
        TextField::new(new_name.clone()).key("name"),
        Label::new(new_error.read_only()).color_role(ColorRole::Error).key("error"),
        Row::new([
            Button::new("Create", move |cx| {
                let shared = create_shared.clone();
                cx.defer(move |ui| match shared.borrow().clone() {
                    Some(notes) => create_note(ui, &notes),
                    None => Ok(()),
                })?;
                Ok(())
            })
            .key("create"),
            Button::new("Cancel", move |_| finish(&cancel_new)).key("cancel"),
        ])
        .key("actions"),
    ]);
    let new = Dialog::new(new_dialog.clone(), close.key("close-dialog"), new_form.key("new-form"));

    let root = ui.mount_tree(Element::new("app", ToastHost::new(toasts.clone(), new.key("new-dialog"))))?;
    let tabs_id = ui.find(&root, &["new-dialog", "close-dialog", "menu", "main", "split", "tabs"])?;
    model.borrow_mut().tabs = tabs_id;

    let handles = Notes {
        model,
        notes,
        active,
        toasts,
        new_dialog,
        close_dialog,
        new_name,
        new_error,
        close_message,
        root,
    };
    *shared.borrow_mut() = Some(handles.clone());
    Ok(handles)
}

/// Closes a dialog's session and releases its outcome, so it can open again.
fn finish(dialog: &DialogController) -> Result<(), RuntimeError> {
    dialog.cancel()?;
    dialog.take_outcome()?;
    Ok(())
}

/// Opens a dialog, releasing an outcome left by Escape.
fn open_dialog(dialog: &DialogController) -> Result<(), RuntimeError> {
    dialog.take_outcome()?;
    dialog.open()
}

impl Notes {
    fn dialog_open(&self) -> Result<bool, RuntimeError> {
        Ok(self.new_dialog.status().get()? == DialogStatus::Open
            || self.close_dialog.status().get()? == DialogStatus::Open)
    }

    fn notify(&self, message: String) -> Result<(), RuntimeError> {
        self.toasts.push(Toast::new(message)).map(|_| ())
    }
}

/// Describes the tab strip from the open notes.
fn describe_tabs(ui: &mut Runtime, notes: &Notes) -> Result<(), RuntimeError> {
    let model = notes.model.borrow();
    let Some(tabs) = model.tabs.clone() else { return Ok(()) };
    let elements = model
        .open
        .iter()
        .map(|note| {
            Tab::new(note.name.clone(), note.title.clone(), Element::new("editor", TextArea::new(note.text.clone())))
                .fixed()
                .into_element()
        })
        .collect();
    drop(model);
    ui.reconcile_children(&tabs, elements)
}

/// Opens the note `name` in a tab, or selects its tab when it is open.
pub fn open_note(ui: &mut Runtime, notes: &Notes, name: &str) -> Result<(), RuntimeError> {
    let already = notes.model.borrow().open.iter().any(|note| note.name == name);
    if !already {
        let path = notes.dir().join(name);
        let text = match std::fs::read_to_string(&path) {
            Ok(text) => text.replace("\r\n", "\n").replace('\r', "\n"),
            Err(error) => return notes.notify(format!("Could not open {name}: {error}")),
        };
        if let Err(error) = TextArea::check(&text) {
            return notes.notify(format!("Could not open {name}: {error}"));
        }
        let text_signal = ui.signal(text.clone());
        let saved = ui.signal(text);
        let title_name = name.to_owned();
        let title = ReadSignal::combine((&text_signal, &saved), move |(text, saved)| {
            if text == saved { title_name.clone() } else { format!("{title_name} •") }
        })?;
        let status_name = name.to_owned();
        let status = text_signal.map(move |text| status_text(&status_name, text))?;
        notes.model.borrow_mut().open.push(OpenNote {
            name: name.to_owned(),
            text: text_signal,
            saved,
            title,
            status,
        });
        describe_tabs(ui, notes)?;
    }
    let tabs = notes.model.borrow().tabs.clone();
    if let Some(tabs) = tabs {
        ui.reveal_child(&tabs, name)?;
    }
    notes.active.set(Some(name.to_owned()))
}

/// Saves the note `name`; reports a failure in a notification and returns
/// whether it saved.
fn save_note(notes: &Notes, name: &str) -> Result<bool, RuntimeError> {
    let (path, text, saved) = {
        let model = notes.model.borrow();
        let Some(note) = model.open.iter().find(|note| note.name == name) else { return Ok(false) };
        (model.dir.join(name), note.text.clone(), note.saved.clone())
    };
    let content = text.get()?;
    match std::fs::write(&path, &content) {
        Ok(()) => {
            saved.set(content)?;
            Ok(true)
        }
        Err(error) => {
            notes.notify(format!("Could not save {name}: {error}"))?;
            Ok(false)
        }
    }
}

/// File › Save.
fn save_active(_: &mut Runtime, notes: &Notes) -> Result<bool, RuntimeError> {
    match notes.active.get()? {
        Some(name) => save_note(notes, &name),
        None => Ok(false),
    }
}

fn is_modified(notes: &Notes, name: &str) -> Result<bool, RuntimeError> {
    let model = notes.model.borrow();
    match model.open.iter().find(|note| note.name == name) {
        Some(note) => Ok(note.text.get()? != note.saved.get()?),
        None => Ok(false),
    }
}

/// Removes the tab of `name`.
fn remove_tab(ui: &mut Runtime, notes: &Notes, name: &str) -> Result<(), RuntimeError> {
    notes.model.borrow_mut().open.retain(|note| note.name != name);
    describe_tabs(ui, notes)?;
    if notes.model.borrow().open.is_empty() {
        notes.active.set(None)?;
    }
    Ok(())
}

/// File › Close tab.
fn close_active(ui: &mut Runtime, notes: &Notes) -> Result<(), RuntimeError> {
    let Some(name) = notes.active.get()? else { return Ok(()) };
    if is_modified(notes, &name)? {
        notes.close_message.set(format!("{name} has unsaved changes. Save them before closing?"))?;
        notes.model.borrow_mut().pending_close = Some(name);
        open_dialog(&notes.close_dialog)
    } else {
        remove_tab(ui, notes, &name)
    }
}

/// The close dialog's Save (`save`) or Discard.
fn close_pending(ui: &mut Runtime, notes: &Notes, save: bool) -> Result<(), RuntimeError> {
    let name = notes.model.borrow_mut().pending_close.take();
    finish(&notes.close_dialog)?;
    let Some(name) = name else { return Ok(()) };
    if save && !save_note(notes, &name)? {
        return Ok(()); // a failed save keeps the tab open
    }
    remove_tab(ui, notes, &name)
}

/// File › New note.
fn new_note(_: &mut Runtime, notes: &Notes) -> Result<(), RuntimeError> {
    notes.new_name.set(String::new())?;
    notes.new_error.set(String::new())?;
    open_dialog(&notes.new_dialog)
}

/// The New note dialog's Create.
fn create_note(ui: &mut Runtime, notes: &Notes) -> Result<(), RuntimeError> {
    let name = notes.new_name.get()?;
    let file = match validate_name(&name, &notes.notes.get()?) {
        Ok(file) => file,
        Err(message) => return notes.new_error.set(message),
    };
    let path = notes.dir().join(&file);
    let created = std::fs::OpenOptions::new().write(true).create_new(true).open(&path);
    if let Err(error) = created {
        return notes.new_error.set(format!("Could not create {file}: {error}"));
    }
    notes.notes.update(|names| {
        names.push(file.clone());
        names.sort();
    })?;
    finish(&notes.new_dialog)?;
    open_note(ui, notes, &file)
}
