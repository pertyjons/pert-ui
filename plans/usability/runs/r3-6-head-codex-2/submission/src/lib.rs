use pert_ui::prelude::*;
use pert_ui::{SplitAxis, SplitPane, SplitSizing};
use std::{cell::RefCell, fs, path::PathBuf, rc::Rc};

type Shared = Rc<RefCell<Notes>>;
#[derive(Clone)]
struct Document {
    name: String,
    text: Signal<String>,
    saved: Signal<String>,
    title: ReadSignal<String>,
    status: ReadSignal<String>,
}
struct Notes {
    folder: PathBuf,
    documents: Vec<Document>,
    tabs: Option<WidgetId>,
    active: Option<String>,
    source: Signal<ListSource>,
    notification: Signal<String>,
    new_dialog: DialogController,
    close_dialog: DialogController,
    draft: Signal<String>,
    error: Signal<String>,
    closing: Option<String>,
}
pub struct Window {
    pub root: WidgetId,
}

pub fn ctrl(c: char) -> KeyChord {
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
    state: Option<Shared>,
}
impl ListItems for Files {
    fn len(&self) -> usize {
        self.names.len()
    }
    fn key(&self, i: usize) -> Option<String> {
        self.names.get(i).cloned()
    }
    fn build(&self, i: usize) -> Option<Element> {
        let name = self.names.get(i)?.clone();
        let state = self.state.clone()?;
        Some(
            Row::new([Button::new(name.clone(), move |cx| {
                let (state, name) = (state.clone(), name.clone());
                cx.defer(move |ui| open(ui, &state, &name))?;
                Ok(())
            })
            .key("open")
            .with_grow(1)])
            .key("row"),
        )
    }
}
fn refresh(state: &Shared) -> Result<(), RuntimeError> {
    let s = state.borrow();
    match fs::read_dir(&s.folder) {
        Ok(entries) => {
            let mut names: Vec<_> = entries
                .filter_map(Result::ok)
                .filter(|e| e.file_type().is_ok_and(|t| t.is_file()))
                .filter_map(|e| e.file_name().into_string().ok())
                .filter(|n| n.ends_with(".txt"))
                .collect();
            names.sort();
            s.source.set(ListSource::new(Files {
                names,
                state: Some(state.clone()),
            }))
        }
        Err(e) => s.notification.set(format!("Could not list notes: {e}")),
    }
}
fn reconcile(ui: &mut Runtime, state: &Shared) -> Result<(), RuntimeError> {
    let s = state.borrow();
    let tabs = s.tabs.as_ref().unwrap();
    let children = s
        .documents
        .iter()
        .map(|d| {
            Tab::new(
                &d.name,
                d.title.clone(),
                Column::new([
                    TextArea::new(d.text.clone()).key("editor").with_grow(1),
                    Label::new(d.status.clone()).key("status"),
                ])
                .align(Align::Stretch)
                .key("page"),
            )
            .fixed()
            .into_element()
        })
        .collect::<Vec<_>>();
    ui.reconcile_children(tabs, children)
}
fn open(ui: &mut Runtime, state: &Shared, name: &str) -> Result<(), RuntimeError> {
    let exists = state.borrow().documents.iter().any(|d| d.name == name);
    if !exists {
        let path = state.borrow().folder.join(name);
        let contents = match fs::read_to_string(path) {
            Ok(text) => text,
            Err(e) => {
                return state
                    .borrow()
                    .notification
                    .set(format!("Could not open {name}: {e}"));
            }
        };
        let text = ui.signal(contents.clone());
        let saved = ui.signal(contents);
        let title_name = name.to_owned();
        let title = ReadSignal::combine((&text, &saved), move |(t, s)| {
            format!("{title_name}{}", if t != s { " *" } else { "" })
        })?;
        let status_name = name.to_owned();
        let status = text.map(move |t| {
            format!(
                "{status_name}: {} lines, {} words",
                t.chars().filter(|c| *c == '\n').count() + 1,
                t.split_whitespace().count()
            )
        })?;
        state.borrow_mut().documents.push(Document {
            name: name.into(),
            text,
            saved,
            title,
            status,
        });
        reconcile(ui, state)?;
    }
    state.borrow_mut().active = Some(name.into());
    let tabs = state.borrow().tabs.clone().unwrap();
    ui.reveal_child(&tabs, name)?;
    Ok(())
}
fn save(state: &Shared, name: &str) -> Result<bool, RuntimeError> {
    let s = state.borrow();
    let Some(d) = s.documents.iter().find(|d| d.name == name) else {
        return Ok(false);
    };
    let text = d.text.get()?;
    match fs::write(s.folder.join(name), &text) {
        Ok(()) => {
            d.saved.set(text)?;
            s.notification.set(String::new())?;
            Ok(true)
        }
        Err(e) => {
            s.notification.set(format!("Save failed for {name}: {e}"))?;
            Ok(false)
        }
    }
}
fn remove(ui: &mut Runtime, state: &Shared, name: &str) -> Result<(), RuntimeError> {
    {
        let mut s = state.borrow_mut();
        s.documents.retain(|d| d.name != name);
        s.active = s.documents.last().map(|d| d.name.clone());
    }
    reconcile(ui, state)?;
    let (tabs, active) = {
        let s = state.borrow();
        (s.tabs.clone().unwrap(), s.active.clone())
    };
    if let Some(name) = active {
        ui.reveal_child(&tabs, &name)?;
    }
    Ok(())
}
fn request_close(ui: &mut Runtime, state: &Shared) -> Result<(), RuntimeError> {
    let name = state.borrow().active.clone();
    let Some(name) = name else { return Ok(()) };
    let modified = {
        let s = state.borrow();
        let d = s.documents.iter().find(|d| d.name == name).unwrap();
        d.text.get()? != d.saved.get()?
    };
    if modified {
        let mut s = state.borrow_mut();
        s.closing = Some(name);
        s.close_dialog.take_outcome()?;
        s.close_dialog.open()
    } else {
        remove(ui, state, &name)
    }
}
fn create(ui: &mut Runtime, state: &Shared) -> Result<(), RuntimeError> {
    let name = state.borrow().draft.get()?;
    let count = name.chars().count();
    let invalid = if !(1..=64).contains(&count) {
        Some("Name must contain 1 to 64 characters.")
    } else if name.contains(['/', '\\']) {
        Some("Name must not contain / or \\.")
    } else if name.starts_with('.') {
        Some("Name must not start with a dot.")
    } else {
        None
    };
    if let Some(error) = invalid {
        return state.borrow().error.set(error.into());
    }
    let name = if name.ends_with(".txt") {
        name
    } else {
        format!("{name}.txt")
    };
    let path = state.borrow().folder.join(&name);
    match fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
    {
        Ok(_) => {
            state.borrow().new_dialog.accept()?;
            refresh(state)?;
            open(ui, state, &name)
        }
        Err(e) => state
            .borrow()
            .error
            .set(if e.kind() == std::io::ErrorKind::AlreadyExists {
                "A note with that name already exists.".into()
            } else {
                format!("Could not create note: {e}")
            }),
    }
}
pub fn app(folder: PathBuf) -> App<Window> {
    App::new("Notes", move |ui| {
        let state = Rc::new(RefCell::new(Notes {
            folder,
            documents: Vec::new(),
            tabs: None,
            active: None,
            source: ui.signal(ListSource::new(Files {
                names: vec![],
                state: None,
            })),
            notification: ui.signal(String::new()),
            new_dialog: DialogController::new(ui),
            close_dialog: DialogController::new(ui),
            draft: ui.signal(String::new()),
            error: ui.signal(String::new()),
            closing: None,
        }));
        refresh(&state)?;
        let s = state.clone();
        let new = ui.register_command(
            "New note",
            CommandScope::Application,
            vec![ctrl('n')],
            move |_, _| {
                let s = s.borrow();
                s.draft.set(String::new())?;
                s.error.set(String::new())?;
                s.new_dialog.take_outcome()?;
                s.new_dialog.open()
            },
        )?;
        let s = state.clone();
        let save_cmd = ui.register_command(
            "Save",
            CommandScope::Application,
            vec![ctrl('s')],
            move |_, _| {
                let name = s.borrow().active.clone();
                if let Some(name) = name {
                    save(&s, &name)?;
                }
                Ok(())
            },
        )?;
        let s = state.clone();
        let close = ui.register_command(
            "Close tab",
            CommandScope::Application,
            vec![ctrl('w')],
            move |cx, _| {
                let s = s.clone();
                cx.defer(move |ui| request_close(ui, &s))?;
                Ok(())
            },
        )?;
        let s = state.clone();
        let list = VirtualList::new(state.borrow().source.read_only(), 36.0)?
            .selection(ListSelection::new(ui, SelectionMode::Single))
            .on_activate(move |cx, name| {
                let (s, name) = (s.clone(), name.to_owned());
                cx.defer(move |ui| open(ui, &s, &name))?;
                Ok(())
            });
        let s = state.clone();
        let tabs = Tabs::new([]).on_select(move |_, change| {
            s.borrow_mut().active = change.selected.clone();
            Ok(())
        });
        let body = Column::new([
            SplitPane::new(SplitAxis::LeftRight, list.key("files"), tabs.key("tabs"))
                .sizing(SplitSizing::first(Length::from(220)))
                .key("split")
                .with_grow(1),
            Label::new(state.borrow().notification.read_only()).key("notification"),
        ])
        .align(Align::Stretch)
        .key("body");
        let bar = MenuBar::new(
            ui,
            vec![MenuItem::submenu(
                "file",
                "File",
                vec![
                    MenuItem::command(ui, "new", &new, |_, _| Ok(()))?,
                    MenuItem::command(ui, "save", &save_cmd, |_, _| Ok(()))?,
                    MenuItem::command(ui, "close", &close, |_, _| Ok(()))?,
                ],
            )],
        )?
        .content(body);
        let s = state.clone();
        let new_cancel = state.borrow().new_dialog.clone();
        let new_form = Column::new([
            Label::new("New note").key("heading"),
            TextField::new(state.borrow().draft.clone())
                .placeholder("Note name")
                .key("name"),
            Label::new(state.borrow().error.read_only()).key("error"),
            Row::new([
                Button::new("Create", move |cx| {
                    let s = s.clone();
                    cx.defer(move |ui| create(ui, &s))?;
                    Ok(())
                })
                .key("create"),
                Button::new("Cancel", move |_| new_cancel.cancel().map(|_| ())).key("cancel"),
            ])
            .key("actions"),
        ])
        .key("new-form");
        let s_save = state.clone();
        let s_discard = state.clone();
        let cancel = state.borrow().close_dialog.clone();
        let close_form = Column::new([
            Label::new("Save changes before closing?").key("heading"),
            Row::new([
                Button::new("Save", move |cx| {
                    let s = s_save.clone();
                    cx.defer(move |ui| {
                        let name = s.borrow().closing.clone().unwrap();
                        if save(&s, &name)? {
                            s.borrow().close_dialog.accept()?;
                            remove(ui, &s, &name)?;
                        }
                        Ok(())
                    })?;
                    Ok(())
                })
                .key("save"),
                Button::new("Discard", move |cx| {
                    let s = s_discard.clone();
                    cx.defer(move |ui| {
                        let name = s.borrow().closing.clone().unwrap();
                        s.borrow().close_dialog.accept()?;
                        remove(ui, &s, &name)
                    })?;
                    Ok(())
                })
                .key("discard"),
                Button::new("Cancel", move |_| cancel.cancel().map(|_| ())).key("cancel"),
            ])
            .key("actions"),
        ])
        .key("close-form");
        let root = ui.mount_tree(
            Dialog::new(
                state.borrow().new_dialog.clone(),
                Dialog::new(
                    state.borrow().close_dialog.clone(),
                    bar.key("bar"),
                    close_form,
                )
                .key("close-dialog"),
                new_form,
            )
            .key("window"),
        )?;
        state.borrow_mut().tabs = ui.find_descendant(&root, "tabs")?;
        Ok(Window { root })
    })
}
