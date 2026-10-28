//! A small editor for plain-text notes kept as `.txt` files in one folder.

use pert_ui::prelude::*;
use pert_ui::{
    Dialog, DialogController, ListItems, ListSelection, ListSource, MenuBar, MenuItem,
    SelectionMode, SplitAxis, SplitPane, SplitSizing, Tab, Tabs, Toast, ToastController,
    ToastHost, VirtualList,
};
use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::{fs, io};

/// Lists the `.txt` files of `dir` by name, sorted.
pub fn list_notes(dir: &Path) -> io::Result<Vec<String>> {
    let mut names = Vec::new();
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        if !entry.file_type()?.is_file() {
            continue;
        }
        if let Some(name) = entry.file_name().to_str() {
            if name.ends_with(".txt") {
                names.push(name.to_owned());
            }
        }
    }
    names.sort();
    Ok(names)
}

/// Checks a name typed in the New note dialog and returns the file name.
pub fn note_file_name(name: &str) -> Result<String, &'static str> {
    let count = name.chars().count();
    if count == 0 {
        return Err("A name is required.");
    }
    if count > 64 {
        return Err("A name has at most 64 characters.");
    }
    if name.contains('/') || name.contains('\\') {
        return Err("A name must not contain / or \\.");
    }
    if name.starts_with('.') {
        return Err("A name must not start with a dot.");
    }
    Ok(if name.ends_with(".txt") { name.to_owned() } else { format!("{name}.txt") })
}

/// The status line of a note: `<name>: L lines, W words`.
pub fn status_line(name: &str, text: &str) -> String {
    let lines = text.matches('\n').count() + 1;
    let words = text.split_whitespace().count();
    format!("{name}: {lines} lines, {words} words")
}

/// One open note: its tab edits `text`, `saved` is what the file holds.
struct Doc {
    name: String,
    text: Signal<String>,
    saved: Signal<String>,
    title: ReadSignal<String>,
    status: ReadSignal<String>,
}

struct Notes {
    dir: PathBuf,
    names: Signal<Vec<String>>,
    docs: RefCell<Vec<Rc<Doc>>>,
    tabs: RefCell<Option<WidgetId>>,
    active: Signal<Option<String>>,
    selection: ListSelection,
    toasts: ToastController,
    new_dialog: DialogController,
    new_name: Signal<String>,
    new_error: Signal<String>,
    close_dialog: DialogController,
    close_message: Signal<String>,
    closing: RefCell<Option<String>>,
}

impl Notes {
    fn toast(&self, message: String) -> Result<(), RuntimeError> {
        self.toasts.push(Toast::new(message)).map(|_| ())
    }

    fn doc(&self, name: &str) -> Option<Rc<Doc>> {
        self.docs.borrow().iter().find(|doc| doc.name == name).cloned()
    }

    fn active_doc(&self) -> Result<Option<Rc<Doc>>, RuntimeError> {
        Ok(self.active.get()?.and_then(|name| self.doc(&name)))
    }

    fn tabs(&self) -> Result<WidgetId, RuntimeError> {
        self.tabs.borrow().clone().ok_or(RuntimeError::Unmounted)
    }

    /// Describes the tab strip again from the open notes.
    fn describe_tabs(&self, ui: &mut Runtime) -> Result<(), RuntimeError> {
        let tabs = self.tabs()?;
        let children = self
            .docs
            .borrow()
            .iter()
            .map(|doc| {
                Tab::new(
                    doc.name.clone(),
                    doc.title.clone(),
                    Element::new("editor", TextArea::new(doc.text.clone())),
                )
                .fixed()
                .into_element()
            })
            .collect();
        ui.reconcile_children(&tabs, children)
    }

    /// Shows the note `name` in its tab, opening it when it is not open.
    fn open(&self, ui: &mut Runtime, name: &str) -> Result<(), RuntimeError> {
        if self.doc(name).is_none() {
            let text = match fs::read_to_string(self.dir.join(name)) {
                Ok(text) => text.replace("\r\n", "\n").replace('\r', "\n"),
                Err(error) => return self.toast(format!("Could not open {name}: {error}")),
            };
            if let Err(error) = TextArea::check(&text) {
                return self.toast(format!("Could not open {name}: {error}"));
            }
            let text_signal = ui.signal(text.clone());
            let saved = ui.signal(text);
            let label = name.to_owned();
            let title = text_signal.combine(&saved, move |text, saved| {
                if text == saved { label.clone() } else { format!("{label} *") }
            })?;
            let label = name.to_owned();
            let status = text_signal.map(move |text| status_line(&label, text))?;
            self.docs.borrow_mut().push(Rc::new(Doc {
                name: name.to_owned(),
                text: text_signal,
                saved,
                title,
                status,
            }));
            self.describe_tabs(ui)?;
        }
        let tabs = self.tabs()?;
        ui.reveal_child(&tabs, name)?;
        self.selection.set_active(Some(name.to_owned()))?;
        if let Some(editor) = ui.find(&tabs, &[name, "editor"])? {
            ui.focus(&editor)?;
        }
        Ok(())
    }

    /// Writes a note's text to its file; reports a failure in a notification.
    fn save_doc(&self, doc: &Doc) -> Result<bool, RuntimeError> {
        let text = doc.text.get()?;
        match fs::write(self.dir.join(&doc.name), &text) {
            Ok(()) => {
                doc.saved.set(text)?;
                Ok(true)
            }
            Err(error) => {
                self.toast(format!("Could not save {}: {error}", doc.name))?;
                Ok(false)
            }
        }
    }

    fn save(&self) -> Result<(), RuntimeError> {
        if let Some(doc) = self.active_doc()? {
            self.save_doc(&doc)?;
        }
        Ok(())
    }

    fn remove(&self, ui: &mut Runtime, name: &str) -> Result<(), RuntimeError> {
        self.docs.borrow_mut().retain(|doc| doc.name != name);
        self.describe_tabs(ui)?;
        // The strip selects a neighbour and reports it in this flush; its
        // editor takes the focus the closed tab had.
        ui.flush(ui.now())?;
        let tabs = self.tabs()?;
        if let Some(active) = self.active.get()? {
            if let Some(editor) = ui.find(&tabs, &[active.as_str(), "editor"])? {
                ui.focus(&editor)?;
            }
        }
        Ok(())
    }

    /// Closes the active tab, asking first when it is modified.
    fn close(&self, ui: &mut Runtime) -> Result<(), RuntimeError> {
        let Some(doc) = self.active_doc()? else { return Ok(()) };
        if doc.text.get()? == doc.saved.get()? {
            return self.remove(ui, &doc.name);
        }
        *self.closing.borrow_mut() = Some(doc.name.clone());
        self.close_message
            .set(format!("{} has unsaved changes. Save them before closing?", doc.name))?;
        self.close_dialog.take_outcome()?;
        self.close_dialog.open()
    }

    /// Answers the close dialog: `Some(save)` closes the tab, saving first
    /// when `save` is true; `None` keeps it.
    fn finish_close(&self, ui: &mut Runtime, answer: Option<bool>) -> Result<(), RuntimeError> {
        self.close_dialog.accept()?;
        let Some(name) = self.closing.borrow_mut().take() else { return Ok(()) };
        let Some(doc) = self.doc(&name) else { return Ok(()) };
        match answer {
            None => Ok(()),
            Some(true) if !self.save_doc(&doc)? => Ok(()),
            Some(_) => self.remove(ui, &name),
        }
    }

    fn new_note(&self) -> Result<(), RuntimeError> {
        self.new_name.set(String::new())?;
        self.new_error.set(String::new())?;
        self.new_dialog.take_outcome()?;
        self.new_dialog.open()
    }

    /// Creates the note named in the New note dialog, or explains the refusal.
    fn create(&self, ui: &mut Runtime) -> Result<(), RuntimeError> {
        let file = match note_file_name(&self.new_name.get()?) {
            Ok(file) => file,
            Err(reason) => return self.new_error.set(reason.to_owned()),
        };
        let path = self.dir.join(&file);
        if self.names.get()?.contains(&file) || path.exists() {
            return self.new_error.set(format!("A note named {file} already exists."));
        }
        if let Err(error) = fs::OpenOptions::new().write(true).create_new(true).open(&path) {
            return self.new_error.set(format!("Could not create {file}: {error}"));
        }
        self.names.update(|names| {
            names.push(file.clone());
            names.sort();
        })?;
        self.new_dialog.accept()?;
        self.open(ui, &file)
    }
}

/// The sidebar's rows: one button per note.
struct Rows {
    names: Vec<String>,
    notes: Rc<Notes>,
}

impl ListItems for Rows {
    fn len(&self) -> usize {
        self.names.len()
    }
    fn key(&self, index: usize) -> Option<String> {
        self.names.get(index).cloned()
    }
    fn build(&self, index: usize) -> Option<Element> {
        let name = self.names.get(index)?.clone();
        let (notes, key) = (self.notes.clone(), name.clone());
        Some(
            Button::new(name, move |cx| {
                let (notes, key) = (notes.clone(), key.clone());
                cx.defer(move |ui| notes.open(ui, &key)).map(|_| ())
            })
            .frameless()
            .key("open"),
        )
    }
    fn index_of(&self, key: &str) -> Option<usize> {
        self.names.iter().position(|name| name == key)
    }
}

/// Builds the editor for the notes in `dir` and returns its root.
pub fn build(ui: &mut Runtime, dir: PathBuf) -> Result<WidgetId, RuntimeError> {
    let initial = list_notes(&dir).unwrap_or_default();
    let notes = Rc::new(Notes {
        dir,
        names: ui.signal(initial),
        docs: RefCell::default(),
        tabs: RefCell::default(),
        active: ui.signal(None),
        selection: ListSelection::new(ui, SelectionMode::Single),
        toasts: ToastController::new(ui),
        new_dialog: DialogController::new(ui),
        new_name: ui.signal(String::new()),
        new_error: ui.signal(String::new()),
        close_dialog: DialogController::new(ui),
        close_message: ui.signal(String::new()),
        closing: RefCell::default(),
    });

    let n = notes.clone();
    let new = ui.register_command("New note", CommandScope::Application,
        vec![KeyChord::primary('n')], move |_, _| n.new_note())?;
    let n = notes.clone();
    let save = ui.register_command("Save", CommandScope::Application,
        vec![KeyChord::primary('s')], move |_, _| n.save())?;
    let n = notes.clone();
    let close = ui.register_command("Close tab", CommandScope::Application,
        vec![KeyChord::primary('w')], move |ui, _| n.close(ui))?;

    // Sidebar: the notes, opened by a click or by Enter on the active row.
    let n = notes.clone();
    let source = notes.names.map(move |names| {
        ListSource::new(Rows { names: names.clone(), notes: n.clone() })
    })?;
    let n = notes.clone();
    let list = VirtualList::new(source, 28.0)?
        .selection(notes.selection.clone())
        .item_tab_stops(false)
        .on_activate(move |cx, key| {
            let (notes, key) = (n.clone(), key.to_owned());
            cx.defer(move |ui| notes.open(ui, &key)).map(|_| ())
        });

    // Tabs: described from the open notes by `Notes::describe_tabs`.
    let active = notes.active.clone();
    let tabs = Tabs::new([]).on_select(move |_, change| active.set(change.selected.clone()));

    let n = notes.clone();
    let status = Dynamic::new(&notes.active, move |_, active| {
        match active.as_deref().and_then(|name| n.doc(name)) {
            Some(doc) => Label::new(doc.status.clone()).key(doc.name.clone()),
            None => Label::new("").key("none"),
        }
    });

    let bar = MenuBar::new(ui, vec![MenuItem::submenu("file", "File", vec![
        MenuItem::command(ui, "new", &new, |_, _| Ok(()))?,
        MenuItem::command(ui, "save", &save, |_, _| Ok(()))?,
        MenuItem::command(ui, "close", &close, |_, _| Ok(()))?,
    ])])?
    .content(Element::new("main", Column::new([
        Element::new("split", SplitPane::new(
            SplitAxis::LeftRight,
            Element::new("notes", list),
            Element::new("tabs", tabs),
        ).sizing(SplitSizing::first(Length::from(200)))).with_grow(1),
        Element::new("status", status),
    ]).align(Align::Stretch)));

    // The New note dialog.
    let n = notes.clone();
    let submit = move |cx: &mut EventContext| {
        let notes = n.clone();
        cx.defer(move |ui| notes.create(ui)).map(|_| ())
    };
    let submit_field = submit.clone();
    let cancel_new = notes.new_dialog.clone();
    let new_content = Column::new([
        Element::new("heading", Label::new("New note")),
        Element::new("name", TextField::new(notes.new_name.clone())
            .placeholder("Name")
            .on_submit(move |cx, _| submit_field.clone()(cx))),
        Element::new("error", Label::new(notes.new_error.read_only()).color_role(ColorRole::Error)),
        Element::new("actions", Row::new([
            Element::new("cancel", Button::new("Cancel", move |_| cancel_new.cancel().map(|_| ()))),
            Element::new("create", Button::new("Create", submit)),
        ])),
    ]);

    // The dialog asking what to do with a modified tab being closed.
    let answer = |answer: Option<bool>| {
        let n = notes.clone();
        move |cx: &mut EventContext| {
            let notes = n.clone();
            cx.defer(move |ui| notes.finish_close(ui, answer)).map(|_| ())
        }
    };
    let close_content = Column::new([
        Element::new("message", Label::new(notes.close_message.read_only())),
        Element::new("actions", Row::new([
            Element::new("save", Button::new("Save", answer(Some(true)))),
            Element::new("discard", Button::new("Discard", answer(Some(false)))),
            Element::new("cancel", Button::new("Cancel", answer(None))),
        ])),
    ]);

    let root = ui.mount_tree(Element::new("app", ToastHost::new(
        notes.toasts.clone(),
        Element::new("new-dialog", Dialog::new(
            notes.new_dialog.clone(),
            Element::new("close-dialog", Dialog::new(
                notes.close_dialog.clone(),
                Element::new("bar", bar),
                Element::new("close-form", close_content),
            ).require_response()),
            Element::new("new-form", new_content),
        )),
    )))?;
    let tabs = ui.find_descendant(&root, "tabs")?.ok_or(RuntimeError::Unmounted)?;
    *notes.tabs.borrow_mut() = Some(tabs);
    Ok(root)
}
