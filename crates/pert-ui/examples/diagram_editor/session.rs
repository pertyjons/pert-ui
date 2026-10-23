//! The deterministic headless session: drives the editor through pointer
//! and key input, the test driver's keys and shortcuts on the native command
//! path, and checks the document, the selection, the inspector and the files
//! it writes.
use crate::app::{self, DiagramApp};
use crate::editor::Tool;
use crate::model::{Diagram, NodeId};
use pert_ui::prelude::*;
use pert_ui::testing::Driver;
use pert_ui::{
    CommandKey, CommandKeyInput, CommandSource, KeyAction, KeyCode, ViewTransform, Viewport,
};
use std::{collections::BTreeSet, path::PathBuf, rc::Rc, time::Duration};

type Outcome = Result<(), Box<dyn std::error::Error>>;

struct Session {
    ui: Runtime,
    app: Rc<DiagramApp>,
    origin: (f32, f32),
}

impl Session {
    /// The test driver over the session's runtime: each action settles with
    /// one flush.
    fn driver(&mut self) -> Driver<'_> {
        Driver::new(&mut self.ui)
    }
    /// One settle, a flush 16 ms later; returns how many components updated.
    fn settle(&mut self) -> Result<usize, Box<dyn std::error::Error>> {
        Ok(self.driver().settle()?.components_updated)
    }
    /// Lets more than a double-click interval pass, so the next press at a
    /// point starts a new click. The runtime's clock moves only with a flush.
    fn pause(&mut self) -> Outcome {
        self.driver().advance(Duration::from_secs(1))?;
        Ok(())
    }
    /// Flushes until `done` holds for the application, for background work.
    fn wait_for(&mut self, what: &str, mut done: impl FnMut(&DiagramApp) -> bool) -> Outcome {
        let app = self.app.clone();
        self.driver()
            .wait_until(Duration::from_secs(5), |_| Ok(done(&app)))
            .map_err(|error| format!("waiting for {what}: {error}").into())
    }
    fn doc(&self) -> Result<Rc<Diagram>, RuntimeError> {
        self.app.editor.diagram()
    }
    fn selected(&self) -> Result<BTreeSet<NodeId>, RuntimeError> {
        Ok(self.app.editor.selected_nodes()?.into_iter().collect())
    }
    fn label(&self, id: NodeId) -> Result<String, Box<dyn std::error::Error>> {
        Ok(self.doc()?.node(id).ok_or("node")?.label.clone())
    }
    /// The window point of a diagram point, through the canvas's view.
    fn at(&self, x: f32, y: f32) -> Result<Point, Box<dyn std::error::Error>> {
        let local = self.app.editor.view.get()?.apply(Point::new(x, y)?)?;
        Ok(Point::new(
            self.origin.0 + local.x(),
            self.origin.1 + local.y(),
        )?)
    }
    fn node_point(&self, id: NodeId) -> Result<Point, Box<dyn std::error::Error>> {
        let (x, y) = self.doc()?.node(id).ok_or("node")?.center();
        self.at(x, y)
    }
    fn pointer(&mut self, point: Point, kind: PointerKind) -> Outcome {
        self.ui.pointer_input(PointerEvent::new(point, kind))?;
        self.settle()?;
        Ok(())
    }
    fn click(&mut self, point: Point) -> Outcome {
        self.pause()?;
        self.pointer(point, PointerKind::Press(PointerButton::Primary))?;
        self.pointer(point, PointerKind::Release(PointerButton::Primary))
    }
    fn drag(&mut self, from: Point, to: Point) -> Outcome {
        self.pause()?;
        self.pointer(from, PointerKind::Press(PointerButton::Primary))?;
        for step in 1..=4 {
            let t = step as f32 / 4.0;
            let point = Point::new(
                from.x() + (to.x() - from.x()) * t,
                from.y() + (to.y() - from.y()) * t,
            )?;
            self.pointer(point, PointerKind::Move)?;
        }
        self.pointer(to, PointerKind::Release(PointerButton::Primary))
    }
    /// One key transition through the native path, which routes commands
    /// and held input, for a key held across other input.
    fn send(&mut self, code: KeyCode, action: KeyAction, base: CommandKey) -> Outcome {
        self.ui
            .command_key_input(CommandKeyInput::new(code, action, base))?;
        Ok(())
    }
    /// A shortcut with the platform's primary modifier held, and Shift too
    /// when `shift`.
    fn shortcut(&mut self, letter: char, code: KeyCode, shift: bool) -> Outcome {
        let chord = KeyChord::primary(letter);
        let chord = if shift { chord.shifted() } else { chord };
        self.driver().chord(chord, code)?;
        Ok(())
    }
    fn find(&self, key: &str) -> Result<WidgetId, Box<dyn std::error::Error>> {
        Ok(self
            .ui
            .find_descendant(&self.app.root, key)?
            .ok_or_else(|| format!("no {key}"))?)
    }
    fn assert_clean(&self, when: &str) -> Outcome {
        let issues = self.ui.validation_report().issues;
        if !issues.is_empty() {
            return Err(format!("validation issues {when}: {issues:?}").into());
        }
        Ok(())
    }
}

pub fn run(dir: PathBuf) -> Outcome {
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir)?;
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(1200.0, 800.0, 1.0)?)?;
    let app = app::build(&mut ui, dir.clone())?;
    let mut s = Session {
        ui,
        app,
        origin: (0.0, 0.0),
    };
    s.settle()?;
    let [x, y, _, _] =
        s.ui.bounds(&s.app.canvas)?
            .ok_or("canvas laid out")?
            .values();
    s.origin = (x, y);
    s.assert_clean("at start")?;
    assert_eq!((s.doc()?.nodes.len(), s.doc()?.edges.len()), (3, 2));
    assert_eq!(s.settle()?, 0, "nothing updates while nothing happens");
    let (start, decide, finish) = (NodeId(1), NodeId(2), NodeId(3));

    // A click selects a node, in the shared selection the outline shows.
    s.click(s.node_point(start)?)?;
    assert_eq!(s.selected()?, BTreeSet::from([start]));
    assert!(s.app.editor.nodes.is_selected(&start.key())?);

    // A drag moves it as one undoable step; Escape cancels another.
    let before = s.doc()?.node(start).cloned().ok_or("start")?;
    let from = s.node_point(start)?;
    s.drag(from, Point::new(from.x() + 40.0, from.y() + 20.0)?)?;
    let moved = s.doc()?.node(start).cloned().ok_or("start")?;
    assert_eq!((moved.x - before.x, moved.y - before.y), (40.0, 20.0));
    s.pause()?;
    let from = s.node_point(start)?;
    s.pointer(from, PointerKind::Press(PointerButton::Primary))?;
    s.pointer(Point::new(from.x() + 100.0, from.y())?, PointerKind::Move)?;
    s.driver().key(KeyCode::Escape)?;
    s.pointer(
        Point::new(from.x() + 100.0, from.y())?,
        PointerKind::Release(PointerButton::Primary),
    )?;
    assert_eq!(s.doc()?.node(start), Some(&moved));
    s.assert_clean("after dragging")?;

    // Undo and redo through their shortcuts.
    s.shortcut('z', KeyCode::KeyZ, false)?;
    assert_eq!(s.doc()?.node(start), Some(&before));
    assert!(!s.app.editor.dirty.get()?, "back at the opened document");
    s.shortcut('z', KeyCode::KeyZ, true)?;
    assert_eq!(s.doc()?.node(start), Some(&moved));

    // Arrow keys nudge the selection.
    s.ui.focus(&s.app.canvas)?;
    s.driver().key(KeyCode::ArrowRight)?;
    assert_eq!(s.doc()?.node(start).map(|node| node.x), Some(moved.x + 1.0));

    // A rubber band around everything selects all three nodes.
    let bounds = s.doc()?.bounds().ok_or("bounds")?;
    let a = s.at(bounds.x() - 10.0, bounds.y() - 10.0)?;
    let b = s.at(
        bounds.x() + bounds.width() + 10.0,
        bounds.y() + bounds.height() + 10.0,
    )?;
    s.drag(a, b)?;
    assert_eq!(s.selected()?, BTreeSet::from([start, decide, finish]));

    // Connecting adds an edge once; a box is added where clicked.
    s.app.editor.tool.set(Tool::Connect)?;
    s.drag(s.node_point(start)?, s.node_point(finish)?)?;
    s.drag(s.node_point(start)?, s.node_point(finish)?)?;
    assert_eq!(s.doc()?.edges.len(), 3, "no duplicate edge");
    s.app.editor.tool.set(Tool::Box)?;
    s.click(s.at(450.0, 450.0)?)?;
    let added = s.doc()?.nodes.last().cloned().ok_or("added")?;
    assert_eq!((added.x, added.y), (390.0, 420.0));
    assert_eq!(s.selected()?, BTreeSet::from([added.id]));
    s.app.editor.tool.set(Tool::Select)?;

    // Delete removes a node with its edges; undo brings both back.
    s.click(s.node_point(decide)?)?;
    s.ui.focus(&s.app.canvas)?;
    s.driver().key(KeyCode::Delete)?;
    assert!(s.doc()?.node(decide).is_none());
    assert_eq!(s.doc()?.edges.len(), 1);
    s.shortcut('z', KeyCode::KeyZ, false)?;
    assert_eq!(s.doc()?.edges.len(), 3);

    // The inspector: a label draft is one step on Enter, and it survives a
    // change to another node meanwhile.
    s.click(s.node_point(finish)?)?;
    let field = s.find("label-input")?;
    s.ui.focus(&field)?;
    s.driver().key(KeyCode::End)?;
    s.driver().type_text(" line")?;
    let editor = s.app.editor.clone();
    editor.commit("Move", |doc| doc.move_by(&[start], 0.0, 5.0))?;
    s.settle()?;
    assert_eq!(s.label(finish)?, "Finish", "a draft is not committed");
    s.driver().key(KeyCode::Enter)?;
    assert_eq!(
        s.label(finish)?,
        "Finish line",
        "the draft survived the move"
    );

    // A draft belongs to the node it was typed for: duplicating the node,
    // which selects the copy, discards it instead of saving it to the copy.
    s.ui.focus(&s.find("label-input")?)?;
    s.driver().key(KeyCode::End)?;
    s.driver().type_text(" draft")?;
    s.shortcut('d', KeyCode::KeyD, false)?;
    let copy = s.doc()?.nodes.last().map(|node| node.id).ok_or("copy")?;
    assert_ne!(copy, finish);
    assert_eq!(s.selected()?, BTreeSet::from([copy]));
    assert_eq!(s.label(finish)?, "Finish line");
    assert_eq!(s.label(copy)?, "Finish line");
    s.driver().key(KeyCode::Enter)?;
    assert_eq!(s.label(copy)?, "Finish line", "no draft reached the copy");
    s.shortcut('z', KeyCode::KeyZ, false)?;
    assert!(s.doc()?.node(copy).is_none());

    // The inspector counts a label's length as the file format does: 101
    // letters with combining accents are 202 characters, too many to store.
    s.click(s.node_point(finish)?)?;
    s.ui.focus(&s.find("label-input")?)?;
    s.shortcut('a', KeyCode::KeyA, false)?;
    s.driver().type_text(&"e\u{301}".repeat(101))?;
    s.driver().key(KeyCode::Enter)?;
    assert_eq!(
        s.label(finish)?,
        "Finish line",
        "refused, kept for correction"
    );
    s.driver().key(KeyCode::Escape)?;
    s.ui.focus(&s.app.canvas)?;

    // The outline shares the selection: a click on its first row selects
    // that node on the canvas, and the inspector follows.
    let outline = s.find("outline-list")?;
    let row =
        s.ui.find_descendant(&outline, &start.key())?
            .ok_or("outline row")?;
    s.driver().click(&row)?;
    assert_eq!(s.selected()?, BTreeSet::from([start]));
    assert_eq!(
        s.app.editor.inspector.shown.get()?,
        crate::editor::Shown::One(start)
    );
    s.click(s.node_point(finish)?)?;

    // Renaming in place: F2 opens the editor with the label selected.
    s.ui.focus(&s.app.canvas)?;
    s.driver().key(KeyCode::F2)?;
    let rename =
        s.ui.find_descendant(&s.app.canvas, "rename")?
            .ok_or("rename editor")?;
    assert_eq!(s.ui.focused(), Some(rename));
    s.driver().type_text("Done")?;
    s.driver().key(KeyCode::Enter)?;
    assert_eq!(
        s.label(finish)?,
        "Done",
        "typing replaced the selected label"
    );
    assert_eq!(s.ui.focused().as_ref(), Some(&s.app.canvas));
    s.driver().key(KeyCode::F2)?;
    s.driver().type_text("Discarded")?;
    s.driver().key(KeyCode::Escape)?;
    assert_eq!(s.label(finish)?, "Done");
    // A double click on a node opens it too.
    let point = s.node_point(start)?;
    s.click(point)?;
    s.pointer(point, PointerKind::Press(PointerButton::Primary))?;
    s.pointer(point, PointerKind::Release(PointerButton::Primary))?;
    assert_eq!(
        s.ui.focused(),
        s.ui.find_descendant(&s.app.canvas, "rename")?
    );
    s.driver().key(KeyCode::Escape)?;
    // The editor is named for inspection.
    s.ui.focus(&s.app.canvas)?;
    s.driver().key(KeyCode::F2)?;
    let named = s.ui.inspect().components.into_iter().any(|component| {
        component.key == "rename"
            && component
                .details
                .semantics
                .is_some_and(|semantics| semantics.name.as_deref() == Some("Node name"))
    });
    assert!(named, "the rename editor has a name");
    // A label too long for the file format is refused and stays open.
    s.driver()
        .type_text(&"x".repeat(crate::model::MAX_LABEL + 1))?;
    s.driver().key(KeyCode::Enter)?;
    assert_eq!(
        s.ui.focused(),
        s.ui.find_descendant(&s.app.canvas, "rename")?
    );
    assert!(
        s.app
            .editor
            .status
            .get()?
            .starts_with("A label has at most")
    );
    s.driver().key(KeyCode::Escape)?;
    assert_eq!(s.label(start)?, "Start");
    // Hiding the canvas ends a rename without saving it.
    s.driver().key(KeyCode::F2)?;
    s.driver().type_text("Hidden")?;
    s.ui.set_visible(&s.app.canvas, false)?;
    s.settle()?;
    s.ui.set_visible(&s.app.canvas, true)?;
    s.settle()?;
    assert_eq!(s.label(start)?, "Start");
    let rename =
        s.ui.find_descendant(&s.app.canvas, "rename")?
            .ok_or("rename")?;
    assert_ne!(s.ui.focused(), Some(rename.clone()));
    // Replacing the document ends a rename: its draft never reaches the
    // new document's node of the same identity.
    s.ui.focus(&s.app.canvas)?;
    s.driver().key(KeyCode::F2)?;
    s.driver().type_text("Old document")?;
    let current = (*s.doc()?).clone();
    s.app.editor.replace(current, None)?;
    s.settle()?;
    assert_ne!(s.ui.focused(), Some(rename));
    s.driver().key(KeyCode::Enter)?;
    assert_eq!(s.label(start)?, "Start");
    s.assert_clean("after renaming")?;

    // A fill picked in the inspector's picker is one undoable step.
    s.click(s.node_point(finish)?)?;
    let fill = s.doc()?.node(finish).ok_or("finish")?.fill;
    let swatch = s.find("swatch")?;
    s.ui.focus(&swatch)?;
    s.driver().key(KeyCode::Enter)?;
    let hue = s.find("hue")?;
    s.ui.focus(&hue)?;
    s.driver().key(KeyCode::ArrowRight)?;
    let picked = s.doc()?.node(finish).ok_or("finish")?.fill;
    assert_ne!(picked, fill);
    assert_eq!(s.app.editor.inspector.fill.get()?, picked);
    s.driver().key(KeyCode::Escape)?;
    s.shortcut('z', KeyCode::KeyZ, false)?;
    assert_eq!(s.doc()?.node(finish).ok_or("finish")?.fill, fill);

    // Space pans while the canvas has focus, and only then.
    s.ui.focus(&s.app.canvas)?;
    let view = s.app.editor.view.get()?;
    s.send(
        KeyCode::Space,
        KeyAction::Press,
        CommandKey::Named(KeyCode::Space),
    )?;
    let from = s.at(500.0, 500.0)?;
    s.drag(from, Point::new(from.x() + 30.0, from.y())?)?;
    s.send(
        KeyCode::Space,
        KeyAction::Release,
        CommandKey::Named(KeyCode::Space),
    )?;
    let panned = s.app.editor.view.get()?;
    let moved = panned.offset().x() - view.offset().x();
    assert!((moved - 30.0).abs() < 0.01, "{moved}");
    let fit = s.find("fit")?;
    s.ui.focus(&fit)?;
    s.send(
        KeyCode::Space,
        KeyAction::Press,
        CommandKey::Named(KeyCode::Space),
    )?;
    assert!(
        !s.app.shared.space.get(),
        "Space stays with the focused button"
    );
    s.send(
        KeyCode::Space,
        KeyAction::Release,
        CommandKey::Named(KeyCode::Space),
    )?;
    s.settle()?;

    // Space pressed the focused Fit button instead.
    assert_ne!(s.app.editor.view.get()?, panned);
    // A middle-button pan ends only with the middle button's release.
    let panned = s.app.editor.view.get()?;
    let from = s.at(500.0, 500.0)?;
    s.pause()?;
    s.pointer(from, PointerKind::Press(PointerButton::Middle))?;
    s.pointer(Point::new(from.x() + 10.0, from.y())?, PointerKind::Move)?;
    s.pointer(
        Point::new(from.x() + 10.0, from.y())?,
        PointerKind::Release(PointerButton::Primary),
    )?;
    s.pointer(Point::new(from.x() + 20.0, from.y())?, PointerKind::Move)?;
    s.pointer(
        Point::new(from.x() + 20.0, from.y())?,
        PointerKind::Release(PointerButton::Middle),
    )?;
    let moved = s.app.editor.view.get()?.offset().x() - panned.offset().x();
    assert!((moved - 20.0).abs() < 0.01, "{moved}");

    // Ctrl+wheel zooms at the pointer; zoom to fit frames everything.
    let point = s.node_point(start)?;
    let scale = s.app.editor.view.get()?.scale().get();
    // The modifier is held across other input, so it is pressed by hand and
    // reported with `native_modifiers`, as the native host does.
    let primary = KeyCode::primary();
    s.send(primary, KeyAction::Press, CommandKey::Named(primary))?;
    s.ui.native_modifiers(Modifiers::PRIMARY)?;
    s.pointer(point, PointerKind::Scroll(Point::new(0.0, -1.0)?))?;
    s.send(primary, KeyAction::Release, CommandKey::Named(primary))?;
    s.ui.native_modifiers(Modifiers::default())?;
    assert!(s.app.editor.view.get()?.scale().get() > scale);
    let after = s.node_point(start)?;
    assert!((after.x() - point.x()).abs() < 0.5 && (after.y() - point.y()).abs() < 0.5);
    let context = s.ui.command_context(CommandSource::Programmatic);
    s.ui.invoke_command(&s.app.commands.zoom_fit, context)?;
    s.settle()?;
    assert_ne!(s.app.editor.view.get()?, ViewTransform::IDENTITY);
    s.assert_clean("after panning and zooming")?;

    // Save through the dialog's own button, then open the file again.
    let context = s.ui.command_context(CommandSource::Programmatic);
    s.ui.invoke_command(&s.app.commands.save_as, context)?;
    s.settle()?;
    s.app.files.save_name.set("probe.txt".to_owned())?;
    s.settle()?;
    let content = s.find("save-content")?;
    let button =
        s.ui.find_descendant(&content, "save")?
            .ok_or("save button")?;
    s.driver().click(&button)?;
    s.wait_for("the save", |app| !app.editor.dirty.get().unwrap_or(true))?;
    let path = dir.join("probe.txt");
    let saved = s.doc()?;
    assert_eq!(Diagram::decode(&std::fs::read_to_string(&path)?)?, *saved);
    s.app.editor.replace(Diagram::default(), None)?;
    let files = s.app.files.clone();
    files.open_path(&mut s.ui, path.clone())?;
    s.wait_for("the open", |app| {
        app.editor.diagram().is_ok_and(|doc| *doc == *saved)
    })?;
    assert_eq!(s.app.editor.path.get()?, Some(path));
    // A damaged file is reported and changes nothing.
    let damaged = dir.join("damaged.txt");
    std::fs::write(&damaged, "diagram 1\nnode 1 box 0 0 1 1 #ffffff tiny\n")?;
    files.open_path(&mut s.ui, damaged)?;
    s.wait_for("the refusal", |app| {
        app.editor
            .status
            .get()
            .is_ok_and(|status| status.starts_with("Open failed"))
    })?;
    assert_eq!(*s.doc()?, *saved);
    assert!(Diagram::decode("diagram 2\n").is_err());
    assert!(Diagram::decode("diagram 1\nnode 1 box NaN 0 50 50 #ffffff x\n").is_err());
    assert!(Diagram::decode("diagram 1\nedge 1 2 3\n").is_err());
    assert!(Diagram::decode("diagram 1\nnext 18446744073709551615\n").is_err());
    // Labels round-trip whatever backslashes and line breaks they hold.
    let mut tricky = Diagram::default();
    for label in ["a\\nb", "a\nb", "\\", "\\\n\\n", "end\\"] {
        tricky.add_node(crate::model::NodeShape::Box, 0.0, 0.0, label);
    }
    assert_eq!(Diagram::decode(&tricky.encode())?, tricky);

    // An open the user overtook does not replace their work: the file is
    // read, but an edit made meanwhile wins.
    let reopened = dir.join("probe.txt");
    files.open_path(&mut s.ui, reopened)?;
    let edited = s.app.editor.clone();
    edited.commit("Move", |doc| doc.move_by(&[start], 1.0, 0.0))?;
    s.wait_for("the overtaken open", |app| {
        app.editor
            .status
            .get()
            .is_ok_and(|status| status.starts_with("Did not open"))
    })?;
    assert_ne!(*s.doc()?, *saved);
    s.shortcut('z', KeyCode::KeyZ, false)?;

    // Export renders the document by itself.
    #[cfg(feature = "cpu")]
    {
        let png = app::export_png(&*s.doc()?)?;
        assert!(png.starts_with(b"\x89PNG"));
        std::fs::write(dir.join("export.png"), png)?;
    }

    // The layout round-trips, and closing asks only with unsaved changes.
    s.app.files.save_layout(&s.ui)?;
    files.restore_layout(&mut s.ui)?;
    assert_eq!(s.app.editor.status.get()?, "Layout restored");
    assert!(app::close_requested(&s.app, &mut s.ui)?.is_none());
    // A draft open in the inspector counts: closing resolves it first. `App`
    // releases focus before its close policy runs; the session calls the
    // policy itself, so it releases focus as that admission does.
    s.click(s.node_point(start)?)?;
    s.ui.focus(&s.find("label-input")?)?;
    s.driver().key(KeyCode::End)?;
    s.driver().type_text("!")?;
    s.ui.clear_focus();
    let decision = app::close_requested(&s.app, &mut s.ui)?.ok_or("asks")?;
    assert_eq!(s.label(start)?, "Start!");
    s.settle()?;
    assert_eq!(decision.get()?, None);
    s.app.files.discard_dialog.cancel()?;
    s.settle()?;
    assert_eq!(decision.get()?, Some(WindowCloseAction::KeepOpen));
    // So does a rename open on the canvas.
    s.ui.focus(&s.app.canvas)?;
    s.driver().key(KeyCode::F2)?;
    s.driver().type_text("Renamed")?;
    s.ui.clear_focus();
    let decision = app::close_requested(&s.app, &mut s.ui)?.ok_or("asks")?;
    assert_eq!(s.label(start)?, "Renamed");
    s.settle()?;
    s.app.files.discard_dialog.cancel()?;
    s.settle()?;
    assert_eq!(decision.get()?, Some(WindowCloseAction::KeepOpen));

    s.assert_clean("at the end")?;
    assert_eq!(s.settle()?, 0, "nothing updates while nothing happens");
    s.ui.shutdown();
    let cleanup = s.ui.take_shutdown_errors();
    assert!(cleanup.is_empty(), "{cleanup:?}");
    let _ = std::fs::remove_dir_all(&dir);
    println!("diagram editor: headless session passed");
    Ok(())
}
