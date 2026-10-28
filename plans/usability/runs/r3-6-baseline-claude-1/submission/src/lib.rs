//! A small editor for plain-text notes kept as `.txt` files in one folder.
//!
//! [`build_app`] describes the whole application; `main` runs it in a native
//! window and the tests start it with `App::headless`.

use pert_ui::prelude::*;
use pert_ui::{
    Align, CommandScope, Dialog, DialogController, Dynamic, KeyChord, Length, ListItems,
    ListSelection, ListSource, MenuBar, MenuItem, SelectionMode, SplitAxis, SplitPane, SplitSizing,
    Banner, Tab, Tabs, TextArea, VirtualList,
};
use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;

/// Window size of the application, in logical pixels.
pub const WINDOW: (f32, f32) = (900.0, 600.0);

/// One note open in a tab.
struct OpenNote {
    name: String,
    /// The editor's text.
    text: Signal<String>,
    /// The text last read from or written to the file.
    saved: Signal<String>,
    /// `<name>: L lines, W words`, following `text`.
    status: ReadSignal<String>,
}

/// The application's state, shared by its callbacks.
struct State {
    dir: PathBuf,
    /// The `.txt` names in the folder, sorted.
    files: Signal<Vec<String>>,
    /// Open notes in tab order.
    open: RefCell<Vec<OpenNote>>,
    /// The key (file name) of the selected tab.
    active: Signal<Option<String>>,
    /// The mounted tab strip.
    strip: RefCell<Option<WidgetId>>,
    /// The message of the notification banner, and whether it is shown.
    notice: Signal<String>,
    notice_shown: Signal<bool>,
    new_dialog: DialogController,
    new_name: Signal<String>,
    new_error: Signal<String>,
    close_dialog: DialogController,
    /// The note the close dialog asks about.
    closing: Signal<Option<String>>,
}

/// Lists the `.txt` files of `dir` by name, sorted.
fn list_notes(dir: &Path) -> std::io::Result<Vec<String>> {
    let mut names = Vec::new();
    for entry in std::fs::read_dir(dir)? {
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

/// `<name>: L lines, W words`.
fn status_line(name: &str, text: &str) -> String {
    let lines = text.matches('\n').count() + 1;
    let words = text.split_whitespace().count();
    format!("{name}: {lines} lines, {words} words")
}

/// Checks a name typed in the New note dialog and returns the file name.
fn note_file_name(name: &str) -> Result<String, String> {
    let length = name.chars().count();
    if length == 0 {
        return Err("Enter a name for the note.".into());
    }
    if length > 64 {
        return Err("A name has at most 64 characters.".into());
    }
    if name.contains('/') || name.contains('\\') {
        return Err("A name must not contain / or \\.".into());
    }
    if name.starts_with('.') {
        return Err("A name must not start with a dot.".into());
    }
    Ok(if name.ends_with(".txt") { name.to_owned() } else { format!("{name}.txt") })
}

/// Reports a failure in the notification banner.
fn notify(state: &State, message: String) -> Result<(), RuntimeError> {
    state.notice.set(message)?;
    state.notice_shown.set(true)
}

impl State {
    fn strip(&self) -> Result<WidgetId, RuntimeError> {
        self.strip.borrow().clone().ok_or(RuntimeError::Unmounted)
    }

    fn note_signals(&self, name: &str) -> Option<(Signal<String>, Signal<String>)> {
        let open = self.open.borrow();
        let note = open.iter().find(|note| note.name == name)?;
        Some((note.text.clone(), note.saved.clone()))
    }

    fn is_modified(&self, name: &str) -> Result<bool, RuntimeError> {
        match self.note_signals(name) {
            Some((text, saved)) => Ok(text.get()? != saved.get()?),
            None => Ok(false),
        }
    }

    fn tab_elements(&self) -> Result<Vec<Element>, RuntimeError> {
        let open = self.open.borrow();
        open.iter()
            .map(|note| {
                let name = note.name.clone();
                let title = note.text.combine(&note.saved.read_only(), move |text, saved| {
                    if text == saved { name.clone() } else { format!("{name} *") }
                })?;
                let editor = Element::new("editor", TextArea::new(note.text.clone()));
                Ok(Tab::new(note.name.clone(), title, editor).fixed().into_element())
            })
            .collect()
    }

    fn describe_tabs(&self, ui: &mut Runtime) -> Result<(), RuntimeError> {
        let strip = self.strip()?;
        let tabs = self.tab_elements()?;
        ui.reconcile_children(&strip, tabs)
    }

    fn refresh_files(&self) -> Result<(), RuntimeError> {
        match list_notes(&self.dir) {
            Ok(names) => self.files.set(names),
            Err(error) => notify(self, format!("Could not list {}: {error}", self.dir.display())),
        }
    }
}

/// Shows the note `name` in its tab, opening it first if needed.
fn open_note(ui: &mut Runtime, state: &State, name: &str) -> Result<(), RuntimeError> {
    let strip = state.strip()?;
    let already = state.open.borrow().iter().any(|note| note.name == name);
    if !already {
        let text = match std::fs::read_to_string(state.dir.join(name)) {
            Ok(text) => text.replace("\r\n", "\n").replace('\r', "\n"),
            Err(error) => return notify(state, format!("Could not open {name}: {error}")),
        };
        if let Err(error) = TextArea::check(&text) {
            return notify(state, format!("Could not open {name}: {error}"));
        }
        let text_signal = ui.signal(text.clone());
        let saved = ui.signal(text);
        let label = name.to_owned();
        let status = text_signal.map(move |text| status_line(&label, text))?;
        state.open.borrow_mut().push(OpenNote {
            name: name.to_owned(),
            text: text_signal,
            saved,
            status,
        });
        state.describe_tabs(ui)?;
    }
    ui.reveal_child(&strip, name)?;
    state.active.set(Some(name.to_owned()))
}

/// Removes the tab of `name` without asking.
fn close_tab(ui: &mut Runtime, state: &State, name: &str) -> Result<(), RuntimeError> {
    state.open.borrow_mut().retain(|note| note.name != name);
    state.describe_tabs(ui)?;
    if state.open.borrow().is_empty() {
        state.active.set(None)?;
    }
    Ok(())
}

/// Writes the note `name` to its file; a failure is reported in the notification banner.
fn save_note(state: &State, name: &str) -> Result<bool, RuntimeError> {
    let Some((text, saved)) = state.note_signals(name) else { return Ok(false) };
    let content = text.get()?;
    match std::fs::write(state.dir.join(name), &content) {
        Ok(()) => {
            saved.set(content)?;
            state.notice_shown.set(false)?;
            Ok(true)
        }
        Err(error) => {
            notify(state, format!("Could not save {name}: {error}"))?;
            Ok(false)
        }
    }
}

/// The New note dialog's Create action.
fn create_note(cx: &mut EventContext, state: &Rc<State>) -> Result<(), RuntimeError> {
    let typed = state.new_name.get()?;
    let name = match note_file_name(&typed) {
        Ok(name) => name,
        Err(message) => return state.new_error.set(message),
    };
    let path = state.dir.join(&name);
    if path.exists() {
        return state.new_error.set(format!("A note named {name} already exists."));
    }
    if let Err(error) = std::fs::OpenOptions::new().write(true).create_new(true).open(&path) {
        return state.new_error.set(format!("Could not create {name}: {error}"));
    }
    state.new_dialog.accept()?;
    let state = state.clone();
    let _receipt = cx.defer(move |ui| {
        state.refresh_files()?;
        open_note(ui, &state, &name)
    })?;
    Ok(())
}

/// The sidebar rows: one frameless button per note.
struct NoteRows {
    names: Vec<String>,
    state: Rc<State>,
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
        let (state, opened) = (self.state.clone(), name.clone());
        let button = Button::new(name.clone(), move |cx| {
            let (state, name) = (state.clone(), opened.clone());
            let _receipt = cx.defer(move |ui| open_note(ui, &state, &name))?;
            Ok(())
        })
        .frameless();
        Some(Element::new(name, button))
    }
    fn index_of(&self, key: &str) -> Option<usize> {
        self.names.iter().position(|name| name == key)
    }
    fn text(&self, index: usize) -> Option<String> {
        self.names.get(index).cloned()
    }
}

/// Describes the notes editor for the folder `dir`.
pub fn build_app(dir: PathBuf) -> Result<App, RuntimeError> {
    build_app_reporting(dir, Rc::default())
}

/// Like [`build_app`], and stores the mounted root in `root` once the
/// application has built its content, for a test to reach it.
pub fn build_app_reporting(
    dir: PathBuf,
    root: Rc<RefCell<Option<WidgetId>>>,
) -> Result<App, RuntimeError> {
    App::new("Notes", move |ui| {
        *root.borrow_mut() = Some(build(ui, dir)?);
        Ok(())
    })
    .size(Size::new(WINDOW.0, WINDOW.1)?)
}

fn build(ui: &mut Runtime, dir: PathBuf) -> Result<WidgetId, RuntimeError> {
    let state = Rc::new(State {
        dir,
        files: ui.signal(Vec::new()),
        open: RefCell::new(Vec::new()),
        active: ui.signal(None),
        strip: RefCell::new(None),
        notice: ui.signal(String::new()),
        notice_shown: ui.signal(false),
        new_dialog: DialogController::new(ui),
        new_name: ui.signal(String::new()),
        new_error: ui.signal(String::new()),
        close_dialog: DialogController::new(ui),
        closing: ui.signal(None),
    });
    state.refresh_files()?;

    // Commands: the menu rows and the shortcuts run the same registration.
    let s = state.clone();
    let new_cmd = ui.register_command(
        "New note",
        CommandScope::Application,
        vec![KeyChord::primary('n')],
        move |_, _| {
            let _ = s.new_dialog.take_outcome()?;
            s.new_name.set(String::new())?;
            s.new_error.set(String::new())?;
            s.new_dialog.open()
        },
    )?;
    let s = state.clone();
    let save_cmd = ui.register_command(
        "Save",
        CommandScope::Application,
        vec![KeyChord::primary('s')],
        move |_, _| {
            if let Some(name) = s.active.get()? {
                save_note(&s, &name)?;
            }
            Ok(())
        },
    )?;
    let s = state.clone();
    let close_cmd = ui.register_command(
        "Close tab",
        CommandScope::Application,
        vec![KeyChord::primary('w')],
        move |ui, _| {
            let Some(name) = s.active.get()? else { return Ok(()) };
            if s.is_modified(&name)? {
                let _ = s.close_dialog.take_outcome()?;
                s.closing.set(Some(name))?;
                s.close_dialog.open()
            } else {
                close_tab(ui, &s, &name)
            }
        },
    )?;

    let menu = MenuBar::new(
        ui,
        vec![MenuItem::submenu(
            "file",
            "File",
            vec![
                MenuItem::command(ui, "new", &new_cmd, |_, _| Ok(()))?,
                MenuItem::command(ui, "save", &save_cmd, |_, _| Ok(()))?,
                MenuItem::command(ui, "close", &close_cmd, |_, _| Ok(()))?,
            ],
        )],
    )?;

    // Sidebar.
    let rows_state = state.clone();
    let source = state.files.map(move |names| {
        ListSource::new(NoteRows { names: names.clone(), state: rows_state.clone() })
    })?;
    let selection = ListSelection::new(ui, SelectionMode::Single);
    let s = state.clone();
    let sidebar = VirtualList::new(source, 28.0)?
        .selection(selection)
        .item_tab_stops(false)
        .on_activate(move |cx, key| {
            let (state, name) = (s.clone(), key.to_owned());
            let _receipt = cx.defer(move |ui| open_note(ui, &state, &name))?;
            Ok(())
        });

    // Tabs.
    let active = state.active.clone();
    let tabs = Tabs::new(Vec::<Tab>::new()).on_select(move |_, change| active.set(change.selected.clone()));

    let split = SplitPane::new(
        SplitAxis::LeftRight,
        Element::new("sidebar", sidebar),
        Element::new("tabs", tabs),
    )
    .sizing(SplitSizing::first(Length::from(220)));

    // Status line for the active tab.
    let s = state.clone();
    let status = Dynamic::new(&state.active, move |_, active| {
        let line = active.as_ref().and_then(|name| {
            s.open.borrow().iter().find(|note| &note.name == name).map(|note| note.status.clone())
        });
        match (active, line) {
            (Some(name), Some(line)) => Element::new(name.clone(), Label::new(line)),
            _ => Element::new("none", Label::new("")),
        }
    });

    let hide = state.notice_shown.clone();
    let banner = Banner::new(
        state.notice.read_only(),
        [Element::new("dismiss", Button::new("Dismiss", move |_| hide.set(false)))],
    )
    .visible_signal(state.notice_shown.read_only());
    let body = Column::new([
        Element::new("notice", banner),
        Element::new("split", split).with_grow(1),
    ])
    .align(Align::Stretch);
    let main = Column::new([
        Element::new("menu", menu.content(Element::new("body", body))).with_grow(1),
        Element::new("status", status),
    ])
    .align(Align::Stretch);

    // New note dialog.
    let submit = state.clone();
    let (cancel_new, create) = (state.new_dialog.clone(), state.clone());
    let new_form = Column::new([
        Element::new("title", Label::new("New note")),
        Element::new(
            "name",
            TextField::new(state.new_name.clone())
                .placeholder("Name")
                .on_submit(move |cx, _| create_note(cx, &submit)),
        ),
        Element::new("error", Label::new(state.new_error.read_only())),
        Element::new(
            "actions",
            Row::new([
                Element::new("cancel", Button::new("Cancel", move |_| cancel_new.cancel().map(|_| ()))),
                Element::new("create", Button::new("Create", move |cx| create_note(cx, &create))),
            ]),
        ),
    ]);

    // Close confirmation dialog.
    let message = state.closing.map(|name| match name {
        Some(name) => format!("{name} has unsaved changes."),
        None => String::new(),
    })?;
    let (on_save, on_discard, on_cancel) = (state.clone(), state.clone(), state.clone());
    let close_form = Column::new([
        Element::new("message", Label::new(message)),
        Element::new(
            "actions",
            Row::new([
                Element::new(
                    "save",
                    Button::new("Save", move |cx| {
                        let state = on_save.clone();
                        state.close_dialog.accept()?;
                        let _receipt = cx.defer(move |ui| {
                            let Some(name) = state.closing.get()? else { return Ok(()) };
                            if save_note(&state, &name)? {
                                close_tab(ui, &state, &name)?;
                            }
                            state.closing.set(None)
                        })?;
                        Ok(())
                    }),
                ),
                Element::new(
                    "discard",
                    Button::new("Discard", move |cx| {
                        let state = on_discard.clone();
                        state.close_dialog.accept()?;
                        let _receipt = cx.defer(move |ui| {
                            let Some(name) = state.closing.get()? else { return Ok(()) };
                            close_tab(ui, &state, &name)?;
                            state.closing.set(None)
                        })?;
                        Ok(())
                    }),
                ),
                Element::new(
                    "cancel",
                    Button::new("Cancel", move |_| {
                        on_cancel.close_dialog.cancel()?;
                        on_cancel.closing.set(None)
                    }),
                ),
            ]),
        ),
    ]);

    let content = Dialog::new(
        state.new_dialog.clone(),
        Element::new(
            "close-dialog",
            Dialog::new(state.close_dialog.clone(), Element::new("main", main), Element::new("close-form", close_form)),
        ),
        Element::new("new-form", new_form),
    );
    let root = ui.mount_tree(Element::new("app", content))?;
    let strip = ui.find_descendant(&root, "tabs")?.ok_or(RuntimeError::Unmounted)?;
    *state.strip.borrow_mut() = Some(strip);
    Ok(root)
}
