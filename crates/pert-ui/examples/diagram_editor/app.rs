//! The editor window: commands, menus, tool windows, the inspector, files,
//! export and close admission, around the canvas.
//!
//! Construction order: the application state (`Editor`, `Files`) first, then
//! commands registered against it, then the tree that presents both. The
//! command ids live apart from the state, so nothing waits for a value that
//! is created later.
use crate::canvas::{Canvas, Shared};
use crate::editor::{Editor, Shown, Tool};
use crate::model::{Diagram, MAX_LABEL, MAX_SIDE, MIN_SIDE, NodeId, NodeShape};
use pert_ui::prelude::*;
use pert_ui::{
    Capacity, CommandId, CommandReceipt, CommandScope, CommandStatus, DialogOutcome, DialogStatus,
    DirectoryEntry, DirectoryResponse, HeldInputId, KeyChord, KeyCode, KeyTransition, ListItems,
    ListSource, Mailbox, MetricRole, Modifiers, ViewScale, ViewTransform, Workspace,
};
use std::{path::PathBuf, rc::Rc};

/// The document a new window starts with.
pub fn sample() -> Diagram {
    let mut doc = Diagram::default();
    let start = doc.add_node(NodeShape::Box, 40.0, 40.0, "Start");
    let decide = doc.add_node(NodeShape::Ellipse, 260.0, 60.0, "Decide");
    let finish = doc.add_node(NodeShape::Box, 260.0, 200.0, "Finish");
    // A new document has every identity left.
    if let (Some(start), Some(decide), Some(finish)) = (start, decide, finish) {
        doc.connect(start, decide);
        doc.connect(decide, finish);
    }
    doc
}

/// Files, dialogs and notifications: the application's own I/O.
pub struct Files {
    editor: Rc<Editor>,
    pub dir: PathBuf,
    pub browser: FileBrowserController,
    pub open_dialog: DialogController,
    pub save_dialog: DialogController,
    pub discard_dialog: DialogController,
    pub save_name: Signal<String>,
    pub toasts: ToastQueue,
    /// Delivers directory listings made on worker threads.
    _listings: Mailbox<DirectoryResponse>,
    /// Advances with every open, so only the latest one is applied.
    opening: std::cell::Cell<u64>,
    /// Whether a save is running: saves run one at a time, so a file never
    /// receives an older snapshot after a newer one.
    saving: std::cell::Cell<bool>,
}

impl Files {
    fn status(&self, text: impl Into<String>) -> Result<(), RuntimeError> {
        self.editor.status.set(text.into())
    }

    /// Saves on a background thread, one save at a time. The document is
    /// captured now, so edits made meanwhile leave it dirty; a save that
    /// finishes after another document was opened or created records
    /// nothing for that one.
    pub fn save_to(self: &Rc<Self>, ui: &mut Runtime, path: PathBuf) -> Result<(), RuntimeError> {
        if self.saving.replace(true) {
            return self.status("A save is still running; try again when it has finished");
        }
        let doc = self.editor.diagram()?;
        let incarnation = self.editor.incarnation()?;
        let text = doc.encode();
        let (files, target) = (self.clone(), path.clone());
        self.status(format!("Saving {}", path.display()))?;
        let started = ui.spawn(
            move || std::fs::write(&target, text),
            move |written| {
                files.saving.set(false);
                match written {
                    Ok(()) if files.editor.incarnation()? == incarnation => {
                        files.editor.mark_saved(doc, path.clone())?;
                        files.status(format!("Saved {}", path.display()))
                    }
                    Ok(()) => files.status(format!("Saved {}", path.display())),
                    Err(error) => {
                        // The status line keeps the message after the toast.
                        let _receipt = files
                            .toasts
                            .push(Toast::new(format!("Could not save: {error}")))?;
                        files.status(format!("Save failed: {error}"))
                    }
                }
            },
        );
        if started.is_err() {
            self.saving.set(false);
        }
        started.map(|_| ())
    }

    /// Reads and decodes on a background thread. Only the latest open
    /// applies, and only to the document it was started from unchanged: an
    /// edit, a new document or another open meanwhile wins over it. A
    /// damaged file is reported and leaves the document as it was.
    pub fn open_path(self: &Rc<Self>, ui: &mut Runtime, path: PathBuf) -> Result<(), RuntimeError> {
        let generation = self.opening.get().wrapping_add(1);
        self.opening.set(generation);
        let (incarnation, revision) = (self.editor.incarnation()?, self.editor.doc.revision());
        let (files, source) = (self.clone(), path.clone());
        self.status(format!("Opening {}", path.display()))?;
        ui.spawn(
            move || {
                std::fs::read_to_string(&source)
                    .map_err(|error| error.to_string())
                    .and_then(|text| Diagram::decode(&text).map_err(|error| error.to_string()))
            },
            move |read| {
                let current = files.opening.get() == generation
                    && files.editor.incarnation()? == incarnation
                    && files.editor.doc.revision() == revision;
                match read {
                    Ok(_) if !current => files.status(format!(
                        "Did not open {}: the diagram changed while it was loading",
                        path.display()
                    )),
                    Ok(diagram) => {
                        files.editor.replace(diagram, Some(path.clone()))?;
                        files.status(format!("Opened {}", path.display()))
                    }
                    Err(error) => {
                        let _receipt = files
                            .toasts
                            .push(Toast::new(format!("Could not open: {error}")))?;
                        files.status(format!("Open failed: {error}"))
                    }
                }
            },
        )?;
        Ok(())
    }

    fn layout_path(&self) -> PathBuf {
        self.dir.join("layout.workspace")
    }

    /// Saves the window's arrangement, separately from the document.
    pub fn save_layout(&self, ui: &Runtime) -> Result<(), RuntimeError> {
        let text = match ui.capture_workspace() {
            Ok(workspace) => workspace.encode(),
            Err(error) => return self.status(format!("Layout not captured: {error}")),
        };
        match std::fs::write(self.layout_path(), text) {
            Ok(()) => self.status("Layout saved"),
            Err(error) => self.status(format!("Layout not saved: {error}")),
        }
    }

    /// Restores a saved arrangement, reporting what could not be applied.
    pub fn restore_layout(&self, ui: &mut Runtime) -> Result<(), RuntimeError> {
        let text = match std::fs::read_to_string(self.layout_path()) {
            Ok(text) => text,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(error) => return self.status(format!("Layout not read: {error}")),
        };
        let workspace = match Workspace::decode(&text) {
            Ok(workspace) => workspace,
            Err(error) => return self.status(format!("Layout not restored: {error}")),
        };
        match ui.restore_workspace(&workspace) {
            Ok(report) if report.is_complete() => self.status("Layout restored"),
            Ok(report) => self.status(format!("Layout partly restored: {report:?}")),
            Err(error) => self.status(format!("Layout not restored: {error}")),
        }
    }
}

pub struct Commands {
    pub new: CommandId,
    pub open: CommandId,
    pub save: CommandId,
    pub save_as: CommandId,
    pub export: CommandId,
    pub undo: CommandId,
    pub redo: CommandId,
    pub delete: CommandId,
    pub select_all: CommandId,
    pub duplicate: CommandId,
    pub front: CommandId,
    pub zoom_in: CommandId,
    pub zoom_out: CommandId,
    pub zoom_fit: CommandId,
    pub palette: CommandId,
    pub save_layout: CommandId,
}

pub struct DiagramApp {
    pub editor: Rc<Editor>,
    pub shared: Rc<Shared>,
    pub files: Rc<Files>,
    pub commands: Commands,
    pub root: WidgetId,
    pub canvas: WidgetId,
    _space: HeldInputId,
}

/// Renders the document by itself, sized to its content, on a second
/// runtime with a light theme and the CPU renderer, and returns PNG bytes.
#[cfg(feature = "cpu")]
pub fn export_png(diagram: &Diagram) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    use pert_ui::cpu::CpuRenderer;
    use pert_ui::{CaptureRequest, CaptureTarget, ThemeMode, Viewport};
    let bounds = diagram.bounds().ok_or("the diagram is empty")?;
    let margin = 16.0;
    let mut ui = Runtime::new();
    ui.theme().set_mode(ThemeMode::Light)?;
    ui.set_viewport(Viewport::new(
        bounds.width() + 2.0 * margin,
        bounds.height() + 2.0 * margin,
        2.0,
    )?)?;
    // The canvas reads signals of the runtime it is mounted in: give it an
    // editor of its own over a copy of the document.
    let editor = Editor::new(&ui, diagram.clone())?;
    editor.view.set(ViewTransform::new(
        ViewScale::ONE,
        Point::new(margin - bounds.x(), margin - bounds.y())?,
    ))?;
    ui.mount_tree(Element::new("export", Canvas::new(editor, Rc::default())))?;
    ui.flush(std::time::Duration::ZERO)?;
    ui.set_capture_support(true);
    let window = ui.primary_window().id();
    let pending = ui.request_capture(CaptureRequest::new(CaptureTarget::Window(window)));
    ui.flush(std::time::Duration::ZERO)?;
    CpuRenderer::new().serve_captures(&mut ui);
    Ok(pending.wait()?.value?.image.encode_png())
}

/// Zooms so that every node is visible in the canvas.
fn zoom_to_fit(editor: &Editor, shared: &Shared) -> Result<(), RuntimeError> {
    let Some(area) = shared.size.get() else {
        return Ok(());
    };
    let Some(content) = editor.diagram()?.bounds() else {
        return editor.view.set(ViewTransform::IDENTITY);
    };
    let margin = 24.0;
    let (width, height) = (area.width() - 2.0 * margin, area.height() - 2.0 * margin);
    let scale = (width / content.width())
        .min(height / content.height())
        .clamp(ViewScale::MIN.get(), 4.0);
    let offset = Point::new(
        margin - content.x() * scale + (width - content.width() * scale) / 2.0,
        margin - content.y() * scale + (height - content.height() * scale) / 2.0,
    )?;
    editor
        .view
        .set(ViewTransform::new(ViewScale::new(scale)?, offset))
}

fn center(shared: &Shared) -> Result<Point, RuntimeError> {
    let size = shared.size.get().unwrap_or(Size::ZERO);
    Ok(Point::new(size.width() / 2.0, size.height() / 2.0)?)
}

fn register(
    ui: &mut Runtime,
    editor: &Rc<Editor>,
    files: &Rc<Files>,
    shared: &Rc<Shared>,
    palette: &CommandPaletteController,
) -> Result<Commands, RuntimeError> {
    let app = CommandScope::Application;
    let primary = KeyChord::primary;
    let e = editor.clone();
    let new = ui.register_command("New", app.clone(), vec![primary('n')], move |_, _| {
        e.replace(Diagram::default(), None)?;
        e.status.set("New diagram".to_owned())
    })?;
    let f = files.clone();
    let open = ui.register_command("Open…", app.clone(), vec![primary('o')], move |_, _| {
        f.browser.refresh()?;
        f.open_dialog.open()
    })?;
    let (e, f) = (editor.clone(), files.clone());
    let save = ui.register_command(
        "Save",
        app.clone(),
        vec![primary('s')],
        move |ui, _| match e.path.get()? {
            Some(path) => f.save_to(ui, path),
            None => f.save_dialog.open(),
        },
    )?;
    let f = files.clone();
    let save_as = ui.register_command(
        "Save as…",
        app.clone(),
        vec![primary('s').shifted()],
        move |_, _| f.save_dialog.open(),
    )?;
    let (e, f) = (editor.clone(), files.clone());
    let export = ui.register_command(
        "Export PNG",
        app.clone(),
        vec![primary('e')],
        move |_, _| {
            let path = f.dir.join("export.png");
            #[cfg(feature = "cpu")]
            let written = export_png(&*e.diagram()?)
                .map_err(|error| error.to_string())
                .and_then(|png| std::fs::write(&path, png).map_err(|error| error.to_string()));
            #[cfg(not(feature = "cpu"))]
            let written: Result<(), String> = Err("this build has no `cpu` feature".to_owned());
            e.status.set(match written {
                Ok(()) => format!("Exported {}", path.display()),
                Err(error) => format!("Export failed: {error}"),
            })
        },
    )?;
    let e = editor.clone();
    let undo = ui.register_command("Undo", app.clone(), vec![primary('z')], move |_, _| {
        e.undo()
    })?;
    let e = editor.clone();
    let redo = ui.register_command(
        "Redo",
        app.clone(),
        vec![primary('z').shifted(), primary('y')],
        move |_, _| e.redo(),
    )?;
    let e = editor.clone();
    let delete = ui.register_command("Delete", app.clone(), vec![], move |_, _| {
        e.delete_selection()
    })?;
    let e = editor.clone();
    let select_all = ui.register_command(
        "Select all",
        app.clone(),
        vec![primary('a')],
        move |_, _| e.select_all(),
    )?;
    let e = editor.clone();
    let duplicate =
        ui.register_command("Duplicate", app.clone(), vec![primary('d')], move |_, _| {
            e.duplicate()
        })?;
    let e = editor.clone();
    let front = ui.register_command("Bring to front", app.clone(), vec![], move |_, _| {
        e.bring_to_front()
    })?;
    let (e, s) = (editor.clone(), shared.clone());
    let zoom_in =
        ui.register_command("Zoom in", app.clone(), vec![primary('=')], move |_, _| {
            e.zoom_by(1.25, center(&s)?)
        })?;
    let (e, s) = (editor.clone(), shared.clone());
    let zoom_out =
        ui.register_command("Zoom out", app.clone(), vec![primary('-')], move |_, _| {
            e.zoom_by(0.8, center(&s)?)
        })?;
    let (e, s) = (editor.clone(), shared.clone());
    let zoom_fit = ui.register_command(
        "Zoom to fit",
        app.clone(),
        vec![primary('0')],
        move |_, _| zoom_to_fit(&e, &s),
    )?;
    let p = palette.clone();
    let palette = ui.register_command(
        "Command palette",
        app.clone(),
        vec![primary('p').shifted()],
        move |ui, context| p.open(ui, context),
    )?;
    let f = files.clone();
    let save_layout =
        ui.register_command("Save layout", app, vec![], move |ui, _| f.save_layout(ui))?;
    ui.command_enabled_signal(&undo, editor.can_undo.read_only())?;
    ui.command_enabled_signal(&redo, editor.can_redo.read_only())?;
    for command in [&delete, &duplicate, &front] {
        ui.command_enabled_signal(command, editor.has_selection()?)?;
    }
    Ok(Commands {
        new,
        open,
        save,
        save_as,
        export,
        undo,
        redo,
        delete,
        select_all,
        duplicate,
        front,
        zoom_in,
        zoom_out,
        zoom_fit,
        palette,
        save_layout,
    })
}

/// A command whose failure, which a button or menu cannot show itself, goes
/// to the status line.
fn report(
    editor: &Rc<Editor>,
) -> impl FnMut(CommandReceipt, &mut EventContext) -> Result<(), RuntimeError> + 'static {
    let editor = editor.clone();
    move |receipt, _| {
        if let CommandStatus::Finished(Err(error)) = receipt.status() {
            editor.status.set(format!("Command failed: {error}"))?;
        }
        Ok(())
    }
}

/// The outline's rows: one per node, keyed like the canvas's children, so
/// it can share the editor's node selection.
struct Outline(Rc<Diagram>);
impl ListItems for Outline {
    fn len(&self) -> usize {
        self.0.nodes.len()
    }
    fn key(&self, index: usize) -> Option<String> {
        self.0.nodes.get(index).map(|node| node.id.key())
    }
    fn build(&self, index: usize) -> Option<Element> {
        let node = self.0.nodes.get(index)?;
        Some(
            Label::new(node.label.as_str())
                .overflow(TextOverflow::Ellipsis)
                .key("row"),
        )
    }
    fn text(&self, index: usize) -> Option<String> {
        self.0.nodes.get(index).map(|node| node.label.clone())
    }
}

/// The inspector: its structure follows what is selected, its fields stay
/// bound to the editor's view model, and each save is one undoable step.
fn inspector(editor: &Rc<Editor>) -> Element {
    let editor = editor.clone();
    Dynamic::new(
        editor.inspector.shown.read_only(),
        move |_, shown| match *shown {
            Shown::Nothing => Label::new("Nothing selected").key("nothing"),
            Shown::Many(count) => Label::new(format!("{count} items selected")).key("many"),
            // Keyed by the node: another node's fields are new components,
            // so a draft never outlives the node it was typed for.
            Shown::One(id) => node_inspector(&editor, id),
        },
    )
    .key("inspector")
}

fn node_inspector(editor: &Rc<Editor>, id: NodeId) -> Element {
    let inspector = &editor.inspector;
    let e = editor.clone();
    // A normalizer makes a draft that saves on Enter or focus loss. It
    // checks the length the way the file format does, in characters
    // (`max_length` would count graphemes, which can hold several).
    let label = TextField::new(&inspector.label)
        .normalize(|text| {
            if text.chars().count() <= MAX_LABEL {
                Ok(text)
            } else {
                Err(InputRejection::new(format!(
                    "A label has at most {MAX_LABEL} characters"
                )))
            }
        })
        .on_save(move |_, text| {
            let text = text.to_owned();
            e.edit_node(id, "Rename", |node| node.label = text)
        });
    let size = |signal: &Signal<f64>, key: &str, horizontal: bool| -> Element {
        let e = editor.clone();
        let input = DecimalInput::new(signal).normalize(|value| {
            if (f64::from(MIN_SIDE)..=f64::from(MAX_SIDE)).contains(&value) {
                Ok(value.round())
            } else {
                Err(InputRejection::new(format!(
                    "Enter {MIN_SIDE} to {MAX_SIDE}"
                )))
            }
        });
        match input {
            Ok(input) => input
                .on_save(move |_, value| {
                    // The normalizer admitted only values in range.
                    let value = value as f32;
                    e.edit_node(id, "Resize", |node| {
                        if horizontal {
                            node.width = value;
                        } else {
                            node.height = value;
                        }
                    })
                })
                .key(key),
            Err(error) => Label::new(error.to_string()).key(format!("{key}-error")),
        }
    };
    let e = editor.clone();
    let shape = match SegmentedControl::new(
        &inspector.shape,
        [
            Choice::new(NodeShape::Box, "Box"),
            Choice::new(NodeShape::Ellipse, "Ellipse"),
        ],
    ) {
        Ok(control) => control
            .on_change(move |_, shape| {
                let shape = *shape;
                e.edit_node(id, "Change shape", |node| node.shape = shape)
            })
            .key("shape"),
        Err(error) => Label::new(error.to_string()).key("shape-error"),
    };
    let e = editor.clone();
    let fill = ColorPicker::new(&inspector.fill)
        .name("Fill")
        .on_change(move |_, color| e.edit_node(id, "Change fill", |node| node.fill = color))
        .into_popover();
    PropertyGrid::new([
        Property::new("label", "Label", label.key("label-input")),
        Property::new("shape", "Shape", shape),
        Property::new("width", "Width", size(&inspector.width, "width", true)),
        Property::new("height", "Height", size(&inspector.height, "height", false)),
        Property::new("fill", "Fill", fill.key("fill")),
    ])
    // One grid per node: its resized label column is not worth a workspace
    // entry per node.
    .persist(false)
    .key(id.key())
}

fn files(
    ui: &mut Runtime,
    editor: &Rc<Editor>,
    dir: PathBuf,
) -> Result<Rc<Files>, Box<dyn std::error::Error>> {
    // Listings run on worker threads and come back through a mailbox. The
    // browser's request callback must not block or perform I/O itself.
    let browser_slot: Rc<std::cell::RefCell<Option<FileBrowserController>>> = Rc::default();
    let deliver = browser_slot.clone();
    let listings = ui.mailbox(
        Capacity::new(16)?,
        move |response: DirectoryResponse| match deliver.borrow().as_ref() {
            Some(browser) => browser.deliver(response).map(|_| ()),
            None => Ok(()),
        },
    )?;
    let sender = listings.sender();
    let browser = FileBrowserController::new(
        ui,
        [dir.clone()],
        ListSelection::new(ui, SelectionMode::Single),
        move |request| {
            let sender = sender.clone();
            std::thread::spawn(move || {
                let listing = pert_ui::list_directory(request.directory());
                // A full mailbox drains as the window runs: try again, so the
                // folder does not stay loading. A closed one means the window
                // is gone and nobody waits for the listing.
                let mut sent = sender.send(request.respond(listing));
                while let Err(pert_ui::SendError::Full(response)) = sent {
                    std::thread::sleep(std::time::Duration::from_millis(5));
                    sent = sender.send(response);
                }
            });
        },
    )?;
    // The mailbox exists before the browser it delivers to.
    *browser_slot.borrow_mut() = Some(browser.clone());
    Ok(Rc::new(Files {
        editor: editor.clone(),
        dir,
        browser,
        open_dialog: DialogController::new(ui),
        save_dialog: DialogController::new(ui),
        discard_dialog: DialogController::new(ui),
        save_name: ui.signal("diagram.txt".to_owned()),
        toasts: ToastQueue::new(ui),
        _listings: listings,
        opening: std::cell::Cell::new(0),
        saving: std::cell::Cell::new(false),
    }))
}

pub fn build(ui: &mut Runtime, dir: PathBuf) -> Result<Rc<DiagramApp>, Box<dyn std::error::Error>> {
    let editor = Editor::new(ui, sample())?;
    let shared: Rc<Shared> = Rc::default();
    let files = files(ui, &editor, dir)?;
    let palette = CommandPaletteController::new(ui);
    let commands = register(ui, &editor, &files, &shared, &palette)?;

    let opening = files.clone();
    files.browser.on_open(move |cx, entry: &DirectoryEntry| {
        let (files, path) = (opening.clone(), entry.path().to_owned());
        files.open_dialog.accept()?;
        // The dialog closes with this action: work that must outlive it is
        // queued on the window. It reports its own outcome in the status.
        let _receipt = cx.defer_detached(move |ui| files.open_path(ui, path))?;
        Ok(())
    });

    let root = mount(ui, &editor, &files, &shared, &commands, &palette)?;
    let canvas = ui
        .find(
            &root,
            &[
                "dialogs",
                "save",
                "open",
                "palette",
                "toasts",
                "menus",
                "body",
                "tools",
                "canvas-menu",
                "canvas",
            ],
        )?
        .ok_or("the canvas is mounted")?;

    // Space pans while held, but only when the canvas has focus: a held-input
    // registration is offered keys before the focused control, so taking
    // Space everywhere would stop it from pressing buttons.
    let (holding, cancelling) = (shared.clone(), shared.clone());
    let space = ui.register_held_input(
        &canvas,
        vec![KeyCode::Space],
        Modifiers::default(),
        move |_, transition, _| {
            Ok(match transition {
                KeyTransition::Pressed if !holding.focused.get() => InputOutcome::Ignored,
                KeyTransition::Released | KeyTransition::UnmatchedRelease => {
                    holding.space.set(false);
                    InputOutcome::Handled
                }
                _ => {
                    holding.space.set(true);
                    InputOutcome::Handled
                }
            })
        },
        move |_, _| cancelling.space.set(false),
    )?;

    files.restore_layout(ui)?;
    Ok(Rc::new(DiagramApp {
        editor,
        shared,
        files,
        commands,
        root,
        canvas,
        _space: space,
    }))
}

fn mount(
    ui: &mut Runtime,
    editor: &Rc<Editor>,
    files: &Rc<Files>,
    shared: &Rc<Shared>,
    c: &Commands,
    palette: &CommandPaletteController,
) -> Result<WidgetId, Box<dyn std::error::Error>> {
    let item = |key: &str, id: &CommandId| MenuItem::command(ui, key, id, report(editor));
    let tools = |tool: &Signal<Tool>| {
        vec![
            MenuItem::radio("select", "Select", tool, Tool::Select),
            MenuItem::radio("box", "Box", tool, Tool::Box),
            MenuItem::radio("ellipse", "Ellipse", tool, Tool::Ellipse),
            MenuItem::radio("connect", "Connect", tool, Tool::Connect),
        ]
    };
    let menus = vec![
        MenuItem::submenu(
            "file",
            "File",
            vec![
                item("new", &c.new)?,
                item("open", &c.open)?,
                item("save", &c.save)?,
                item("save-as", &c.save_as)?,
                MenuItem::separator(),
                item("export", &c.export)?,
                item("save-layout", &c.save_layout)?,
            ],
        ),
        MenuItem::submenu(
            "edit",
            "Edit",
            vec![
                item("undo", &c.undo)?,
                item("redo", &c.redo)?,
                MenuItem::separator(),
                item("duplicate", &c.duplicate)?,
                item("delete", &c.delete)?,
                item("select-all", &c.select_all)?,
            ],
        ),
        MenuItem::submenu(
            "view",
            "View",
            vec![
                item("zoom-in", &c.zoom_in)?,
                item("zoom-out", &c.zoom_out)?,
                item("zoom-fit", &c.zoom_fit)?,
                MenuItem::separator(),
                item("palette", &c.palette)?,
            ],
        ),
        MenuItem::submenu("tools", "Tools", tools(&editor.tool)),
        MenuItem::submenu("arrange", "Arrange", vec![item("front", &c.front)?]),
    ];

    let toolbar = Row::new([
        Element::new(
            "tool",
            SegmentedControl::new(
                &editor.tool,
                [
                    Choice::new(Tool::Select, "Select"),
                    Choice::new(Tool::Box, "Box"),
                    Choice::new(Tool::Ellipse, "Ellipse"),
                    Choice::new(Tool::Connect, "Connect"),
                ],
            )?,
        ),
        Element::new("undo", Button::command(ui, &c.undo, report(editor))?),
        Element::new("redo", Button::command(ui, &c.redo, report(editor))?),
        Element::new("delete", Button::command(ui, &c.delete, report(editor))?),
        Element::new("fit", Button::command(ui, &c.zoom_fit, report(editor))?),
    ]);

    let canvas = ContextMenu::new(
        ui,
        Element::new("canvas", Canvas::new(editor.clone(), shared.clone())),
        vec![
            MenuItem::command(ui, "duplicate", &c.duplicate, report(editor))?,
            MenuItem::command(ui, "front", &c.front, report(editor))?,
            MenuItem::command(ui, "delete", &c.delete, report(editor))?,
        ],
    )?;

    let rows = editor
        .doc
        .map(|doc| ListSource::new(Outline(doc.clone())))?;
    let syncing = editor.clone();
    let outline = VirtualList::new(&rows, 24.0)?
        .selection(editor.nodes.clone())
        .on_selection_change(move |_, _| {
            // The list wrote the shared selection; the inspector follows.
            syncing.sync()
        });

    let tool_windows = ToolWindowLayout::new(Element::new("canvas-menu", canvas))
        .edge(ToolEdge::Left, EdgeMode::Stripe)
        .edge(ToolEdge::Right, EdgeMode::Stripe)
        .window(
            ToolWindow::new("outline", "Outline", Element::new("outline-list", outline))
                .edge(ToolEdge::Left)
                .expanded(true),
        )
        .window(
            ToolWindow::new("properties", "Properties", inspector(editor))
                .edge(ToolEdge::Right)
                .expanded(true),
        );

    let zoom = editor
        .view
        .map(|view| format!("{:.0} %", view.scale().get() * 100.0))?;
    let title = editor.path.combine(&editor.dirty, |path, dirty| {
        let name = path
            .as_ref()
            .and_then(|path| path.file_name())
            .map_or("Untitled".into(), |name| {
                name.to_string_lossy().into_owned()
            });
        format!("{name}{}", if *dirty { " •" } else { "" })
    })?;
    let status = Row::new([
        Element::new("file", Label::new(title)),
        Element::new(
            "message",
            Label::new(editor.status.read_only()).overflow(TextOverflow::Ellipsis),
        )
        .with_grow(1),
        Element::new("zoom", Label::new(zoom)),
    ]);
    let body = Column::new([
        Element::new("toolbar", toolbar),
        Element::new("tools", tool_windows).with_grow(1),
        Element::new("status", status),
    ])
    .align(Align::Stretch);
    let menus = MenuBar::new(ui, menus)?.content(Element::new("body", body));

    // Open: the file browser in a dialog.
    let (cancel, opening) = (files.clone(), files.clone());
    let open_content = Column::new([
        Element::new("browser", FileBrowser::new(&files.browser, 24.0)?).with_grow(1),
        Element::new(
            "actions",
            Row::new([
                Element::new(
                    "cancel",
                    Button::new("Cancel", move |_| cancel.open_dialog.cancel().map(|_| ())),
                ),
                Element::new(
                    "open",
                    Button::new("Open", move |cx| {
                        let Some(path) = opening.browser.selected_paths()?.into_iter().next()
                        else {
                            return opening.status("Choose a file to open");
                        };
                        opening.open_dialog.accept()?;
                        let files = opening.clone();
                        // Reports its own outcome in the status line.
                        let _receipt = cx.defer_detached(move |ui| files.open_path(ui, path))?;
                        Ok(())
                    }),
                ),
            ]),
        ),
    ]);
    let (cancel, saving) = (files.clone(), files.clone());
    let save_content = Column::new([
        Element::new(
            "name",
            Field::new(TextField::new(&files.save_name).key("input")).label("File name"),
        ),
        Element::new(
            "actions",
            Row::new([
                Element::new(
                    "cancel",
                    Button::new("Cancel", move |_| cancel.save_dialog.cancel().map(|_| ())),
                ),
                Element::new(
                    "save",
                    Button::new("Save", move |cx| {
                        let name = saving.save_name.get()?;
                        let name = name.trim();
                        if name.is_empty() || name.contains(['/', '\\']) {
                            return saving.status("Enter a file name without a folder");
                        }
                        let path = saving.dir.join(name);
                        saving.save_dialog.accept()?;
                        let files = saving.clone();
                        let _receipt = cx.defer_detached(move |ui| files.save_to(ui, path))?;
                        Ok(())
                    }),
                ),
            ]),
        ),
    ]);

    let content = ToastHost::new(files.toasts.clone(), Element::new("menus", menus));
    let content = CommandPalette::new(palette.clone(), Element::new("toasts", content));
    let content = Dialog::new(
        files.open_dialog.clone(),
        Element::new("palette", content),
        Element::new("open-content", open_content),
    );
    let content = Dialog::new(
        files.save_dialog.clone(),
        Element::new("open", content),
        Element::new("save-content", save_content),
    );
    let content = AlertDialog::new(
        files.discard_dialog.clone(),
        Element::new("save", content),
        "Discard changes?",
        "The diagram has unsaved changes.",
        "Discard",
    );
    Ok(ui.mount_tree(
        Container::new(Element::new("dialogs", content))
            .padding(MetricRole::ContentMargin)
            .key("app"),
    )?)
}

/// Close admission. Open drafts are resolved first, as a click elsewhere
/// would resolve them: clearing focus saves the inspector field and the
/// in-place rename being typed (a draft its field refuses stays unsaved and
/// is lost with the window). The layout is saved, and with unsaved changes
/// the user is asked before they are discarded; the returned decision reads
/// `Some(true)` to close.
pub fn close_requested(
    app: &DiagramApp,
    ui: &mut Runtime,
) -> Result<Option<ReadSignal<Option<bool>>>, RuntimeError> {
    ui.clear_focus();
    app.files.save_layout(ui)?;
    if !app.editor.dirty.get()? {
        return Ok(None);
    }
    // An earlier close the user cancelled left its outcome; take it, or the
    // controller refuses to open again.
    let _earlier = app.files.discard_dialog.take_outcome()?;
    app.files.discard_dialog.open()?;
    let decision = app
        .files
        .discard_dialog
        .status()
        .map(|status| match status {
            DialogStatus::Finished(DialogOutcome::Accepted) => Some(true),
            DialogStatus::Finished(_) => Some(false),
            _ => None,
        })?;
    Ok(Some(decision))
}
