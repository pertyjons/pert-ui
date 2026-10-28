//! Independent acceptance checks added by the verifier in the acceptance
//! copy only (not part of the submission). Every check drives the
//! application through the library's public input facilities (`Driver`
//! clicks, chords, keys and typing, and `Runtime::pointer_input` for menu
//! rows, which are not separate components in this library version) and
//! reads results from `Runtime::inspect` semantics and the file system.
//! The application is built through its public `notes::build` entry point;
//! no application model function is called.

use pert_ui::prelude::*;
use pert_ui::testing::*;
use pert_ui::inspection::{ComponentInspection, SemanticRole};
use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;

type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;

/// The verifier's own folder: names created out of alphabetical order,
/// plus non-`.txt` files and a `.txt` lookalike.
fn folder(test: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("verifier-{test}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    for (name, text) in [
        ("zebra.txt", "stripes and\nmore stripes"),
        ("mango.txt", "ripe"),
        ("data.json", "{\"not\": \"a note\"}"),
        ("apple.txt", "red green\n\nyellow"),
        ("banana.txt.bak", "backup"),
        ("cherry.txt", ""),
        ("README", "no extension"),
    ] {
        fs::write(dir.join(name), text).unwrap();
    }
    dir
}

fn on_disk(dir: &PathBuf) -> BTreeSet<String> {
    fs::read_dir(dir)
        .map(|entries| entries.filter_map(|e| e.ok()?.file_name().into_string().ok()).collect())
        .unwrap_or_default()
}

struct App {
    ui: Runtime,
    root: WidgetId,
    dir: PathBuf,
}

impl App {
    fn start(test: &str) -> Result<App> {
        let dir = folder(test);
        let mut ui = Runtime::new();
        ui.set_viewport(Viewport::new(900.0, 600.0, 1.0)?)?;
        let root = notes::build(&mut ui, dir.clone())?;
        Driver::new(&mut ui).settle()?;
        Ok(App { ui, root, dir })
    }

    fn d(&mut self) -> Driver<'_> {
        Driver::new(&mut self.ui)
    }

    fn id(&self, key: &str) -> Result<WidgetId> {
        Ok(self.ui.find_descendant(&self.root, key)?.ok_or(format!("no {key}"))?)
    }

    fn visible(&self) -> Vec<ComponentInspection> {
        self.ui.inspect().components.into_iter().filter(|c| c.visible).collect()
    }

    fn by_key(&self, key: &str) -> Vec<ComponentInspection> {
        self.visible().into_iter().filter(|c| c.key == key).collect()
    }

    /// Sidebar rows from the inspection snapshot: the visible children of the
    /// `List` component, ordered by their laid-out vertical position.
    fn listed(&self) -> Result<Vec<String>> {
        let comps = self.visible();
        let list = comps
            .iter()
            .find(|c| c.key == "notes" && c.details.semantics.as_ref().map(|s| s.role) == Some(SemanticRole::List))
            .ok_or("no list in snapshot")?;
        let list_id = self.id("notes")?;
        let mut rows = Vec::new();
        for child in self.ui.children(&list_id)? {
            let y = self.ui.bounds(&child)?.ok_or("row not laid out")?.values()[1];
            rows.push((y, self.ui.key(&child)?.to_owned()));
        }
        rows.sort_by(|a, b| a.0.total_cmp(&b.0));
        // The snapshot lists components in mount order, so its rows are
        // ordered by their inspected border box, then compared with the
        // runtime's laid-out order.
        let mut inspected: Vec<(f32, String)> = comps
            .iter()
            .filter(|c| c.parent == Some(list.id))
            .map(|c| -> Result<(f32, String)> {
                let y = c.details.box_model.border_box.ok_or("row without border box")?[1];
                let name = c.details.semantics.as_ref().and_then(|s| s.name.clone()).ok_or("row without name")?;
                Ok((y, name))
            })
            .collect::<Result<_>>()?;
        inspected.sort_by(|a, b| a.0.total_cmp(&b.0));
        let by_inspection: Vec<String> = inspected.into_iter().map(|r| r.1).collect();
        let by_position: Vec<String> = rows.into_iter().map(|r| r.1).collect();
        assert_eq!(by_inspection, by_position, "inspection names vs laid-out order");
        Ok(by_position)
    }

    /// Tab titles and the selected title from the `TabList` semantics.
    fn tabs(&self) -> Result<(Vec<String>, Option<String>)> {
        let tabs = self.by_key("tabs").into_iter()
            .find(|c| c.details.semantics.as_ref().map(|s| s.role) == Some(SemanticRole::TabList))
            .ok_or("no tab list")?;
        let s = tabs.details.semantics.unwrap();
        let titles = s.items.iter().filter_map(|i| i.name.clone()).collect();
        let selected = s.items.iter().find(|i| i.selected).and_then(|i| i.name.clone());
        Ok((titles, selected))
    }

    /// The status line: the name of the visible label under `status`.
    fn status(&self) -> Result<String> {
        let comps = self.visible();
        let status = comps.iter().find(|c| c.key == "status").ok_or("no status")?;
        let label = comps.iter().find(|c| c.parent == Some(status.id)).ok_or("no status label")?;
        Ok(label.details.semantics.as_ref().and_then(|s| s.name.clone()).unwrap_or_default())
    }

    /// Number of open library dialogs: a `Dialog`-role component whose
    /// presented `dialog` part is visible.
    fn open_dialogs(&self) -> usize {
        let comps = self.visible();
        let dialogs: Vec<u64> = comps.iter()
            .filter(|c| c.details.semantics.as_ref().map(|s| s.role) == Some(SemanticRole::Dialog))
            .map(|c| c.id)
            .collect();
        comps.iter().filter(|c| c.key == "dialog" && c.parent.is_some_and(|p| dialogs.contains(&p))).count()
    }

    fn visible_names(&self) -> Vec<String> {
        self.visible().into_iter().filter_map(|c| c.details.semantics?.name).collect()
    }

    /// Visible notifications from the `Notifications`-role toast host.
    fn notifications(&self) -> Result<f64> {
        let host = self.visible().into_iter()
            .find(|c| c.details.semantics.as_ref().map(|s| s.role) == Some(SemanticRole::Notifications))
            .ok_or("no notifications host")?;
        match host.details.semantics.unwrap().value {
            Some(SemanticValue::Number { value, .. }) => Ok(value),
            other => Err(format!("host value {other:?}").into()),
        }
    }

    fn editor(&self, note: &str) -> Result<WidgetId> {
        let tabs = self.id("tabs")?;
        Ok(self.ui.find(&tabs, &[note, "editor"])?.ok_or(format!("no editor for {note}"))?)
    }

    fn editor_text(&mut self, note: &str) -> Result<String> {
        let editor = self.editor(note)?;
        match self.d().semantics(&editor)?.value {
            Some(SemanticValue::Text(t)) => Ok(t),
            other => Err(format!("editor value {other:?}").into()),
        }
    }

    fn file(&self, note: &str) -> Option<String> {
        fs::read_to_string(self.dir.join(note)).ok()
    }

    fn click_row(&mut self, note: &str) -> Result {
        let list = self.id("notes")?;
        let row = self.d().find(&list, &[note])?;
        self.d().click(&row)?;
        Ok(())
    }

    fn click_in(&mut self, parent: &str, key: &str) -> Result {
        let parent = self.id(parent)?;
        let target = self.ui.find_descendant(&parent, key)?.ok_or(format!("no {key}"))?;
        self.d().click(&target)?;
        Ok(())
    }

    fn chord(&mut self, c: char, code: KeyCode) -> Result {
        self.d().chord(KeyChord::primary(c), code)?;
        Ok(())
    }

    fn pointer(&mut self, x: f32, y: f32, kind: PointerKind) -> Result {
        self.ui.pointer_input(PointerEvent::new(Point::new(x, y)?, kind))?;
        self.d().settle()?;
        Ok(())
    }

    /// File > `row` with the pointer only: click the File title, read the
    /// open menu's rows from its semantics, then move to, press and release
    /// on the row's position inside the menu list.
    fn menu_click(&mut self, row: &str) -> Result {
        let bar = self.id("bar")?;
        let title = self.ui.find_descendant(&bar, "title/file")?.ok_or("no File title")?;
        self.d().click(&title)?;
        let items = self.ui.find_descendant(&title, "items")?.ok_or("menu did not open")?;
        let names: Vec<String> = self.d().semantics(&items)?.items.into_iter()
            .map(|i| i.name.unwrap_or_default()).collect();
        assert_eq!(names, ["New note", "Save", "Close tab"]);
        let index = names.iter().position(|n| n == row).ok_or("no such row")?;
        let [x, y, w, h] = self.ui.bounds(&items)?.ok_or("menu not laid out")?.values();
        let row_h = h / names.len() as f32;
        let (px, py) = (x + w / 2.0, y + row_h * (index as f32 + 0.5));
        self.pointer(px, py, PointerKind::Move)?;
        self.pointer(px, py, PointerKind::Press(PointerButton::Primary))?;
        self.pointer(px, py, PointerKind::Release(PointerButton::Primary))?;
        // The menu closed after the choice.
        assert!(
            !self.visible().iter().any(|c| c.key == "items"
                && c.details.semantics.as_ref().map(|s| s.role) == Some(SemanticRole::Menu)),
            "menu still open after choosing {row}"
        );
        Ok(())
    }

    fn capture(&mut self, file: &str) -> Result {
        self.ui.set_capture_support(true);
        let pending = self.ui.request_capture(CaptureRequest::new(CaptureTarget::window(&self.ui.primary_window())));
        self.d().settle()?;
        CpuRenderer::new().serve_captures(&mut self.ui);
        let capture = pending.wait()?.value?;
        assert_eq!((capture.image.width(), capture.image.height()), (900, 600));
        let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/verifier-captures");
        fs::create_dir_all(dir)?;
        fs::write(format!("{dir}/{file}"), capture.image.encode_png())?;
        Ok(())
    }
}

// ---- Item 1 ---------------------------------------------------------------

#[test]
fn item1_lists_own_folder_and_reopen_switches() -> Result {
    let mut app = App::start("item1")?;
    assert_eq!(app.listed()?, ["apple.txt", "cherry.txt", "mango.txt", "zebra.txt"]);
    assert_eq!(app.tabs()?, (vec![], None));
    assert_eq!(app.status()?, "");

    // Pointer opens mango.
    app.click_row("mango.txt")?;
    assert_eq!(app.tabs()?, (vec!["mango.txt".into()], Some("mango.txt".into())));
    assert_eq!(app.editor_text("mango.txt")?, "ripe");

    // Keyboard: focus the list, arrow to zebra (selection is on mango), Enter.
    let list = app.id("notes")?;
    app.ui.focus(&list)?;
    app.d().key(KeyCode::ArrowDown)?;
    app.d().key(KeyCode::Enter)?;
    assert_eq!(app.tabs()?, (vec!["mango.txt".into(), "zebra.txt".into()], Some("zebra.txt".into())));
    assert_eq!(app.status()?, "zebra.txt: 2 lines, 4 words");

    // Reopen mango by pointer: still two tabs, mango active.
    app.click_row("mango.txt")?;
    assert_eq!(app.tabs()?, (vec!["mango.txt".into(), "zebra.txt".into()], Some("mango.txt".into())));
    assert_eq!(app.status()?, "mango.txt: 1 lines, 1 words");

    // Reopen zebra by keyboard: still two tabs, zebra active.
    app.ui.focus(&list)?;
    app.d().key(KeyCode::ArrowDown)?;
    app.d().key(KeyCode::Enter)?;
    assert_eq!(app.tabs()?, (vec!["mango.txt".into(), "zebra.txt".into()], Some("zebra.txt".into())));
    Ok(())
}

// ---- Item 2 ---------------------------------------------------------------

#[test]
fn item2_typing_save_shortcut_menu_and_failure() -> Result {
    let mut app = App::start("item2")?;
    app.click_row("cherry.txt")?;
    assert_eq!(app.status()?, "cherry.txt: 1 lines, 0 words");
    assert_eq!(app.tabs()?.1.as_deref(), Some("cherry.txt"));

    // Typing with line breaks (Enter key) into the focused TextArea.
    app.d().type_text("one two")?;
    assert_eq!(app.tabs()?.1.as_deref(), Some("cherry.txt *"));
    assert_eq!(app.status()?, "cherry.txt: 1 lines, 2 words");
    app.d().key(KeyCode::Enter)?;
    assert_eq!(app.status()?, "cherry.txt: 2 lines, 2 words");
    app.d().type_text("three")?;
    app.d().key(KeyCode::Enter)?;
    app.d().key(KeyCode::Enter)?;
    app.d().type_text("  four\tfive ")?;
    assert_eq!(app.status()?, "cherry.txt: 4 lines, 5 words");
    assert_eq!(app.editor_text("cherry.txt")?, "one two\nthree\n\n  four\tfive ");
    assert_eq!(app.file("cherry.txt").as_deref(), Some(""));

    // Ctrl+S writes the file and clears the marker.
    app.chord('s', KeyCode::KeyS)?;
    assert_eq!(app.file("cherry.txt").as_deref(), Some("one two\nthree\n\n  four\tfive "));
    assert_eq!(app.tabs()?.1.as_deref(), Some("cherry.txt"));
    assert_eq!(app.notifications()?, 0.0);

    // Deleting back to the saved text clears the marker; typing sets it again.
    app.d().type_text("x")?;
    assert_eq!(app.tabs()?.1.as_deref(), Some("cherry.txt *"));
    app.d().key(KeyCode::Backspace)?;
    assert_eq!(app.tabs()?.1.as_deref(), Some("cherry.txt"));

    // File > Save by pointer on a second tab; the other tab is unaffected.
    app.click_row("apple.txt")?;
    app.d().type_text("pear ")?;
    assert_eq!(app.tabs()?, (vec!["cherry.txt".into(), "apple.txt *".into()], Some("apple.txt *".into())));
    assert_eq!(app.status()?, "apple.txt: 3 lines, 4 words");
    app.menu_click("Save")?;
    assert_eq!(app.file("apple.txt").as_deref(), Some("pear red green\n\nyellow"));
    assert_eq!(app.tabs()?, (vec!["cherry.txt".into(), "apple.txt".into()], Some("apple.txt".into())));
    assert_eq!(app.file("cherry.txt").as_deref(), Some("one two\nthree\n\n  four\tfive "));

    // A failing save: remove the folder, then Ctrl+S and File > Save.
    app.d().type_text("lost ")?;
    let editor = app.editor("apple.txt")?;
    app.ui.focus(&editor)?;
    fs::remove_dir_all(&app.dir)?;
    assert_eq!(app.notifications()?, 0.0);
    app.chord('s', KeyCode::KeyS)?;
    assert_eq!(app.notifications()?, 1.0, "no notification after failed Ctrl+S");
    assert_eq!(app.tabs()?.1.as_deref(), Some("apple.txt *"));
    app.menu_click("Save")?;
    assert_eq!(app.notifications()?, 2.0, "no notification after failed File > Save");
    assert_eq!(app.tabs()?.1.as_deref(), Some("apple.txt *"));
    assert!(!app.dir.exists());

    // Evidence capture of the failure notification (CPU renderer).
    app.capture("verifier-save-failure.png")?;
    Ok(())
}

// ---- Item 3 ---------------------------------------------------------------

#[test]
fn item3_new_note_refusals_and_creation() -> Result {
    let mut app = App::start("item3")?;
    let before = on_disk(&app.dir);
    assert_eq!(app.open_dialogs(), 0);

    // Ctrl+N opens the library dialog.
    app.chord('n', KeyCode::KeyN)?;
    assert_eq!(app.open_dialogs(), 1);
    assert!(!app.by_key("new-form").is_empty());
    let long = "y".repeat(65);
    for (name, reason) in [
        ("", "A name is required."),
        ("a/b", "A name must not contain / or \\."),
        ("mango", "A note named mango.txt already exists."),
        ("zebra.txt", "A note named zebra.txt already exists."),
        ("back\\slash", "A name must not contain / or \\."),
        (".dot", "A name must not start with a dot."),
        (long.as_str(), "A name has at most 64 characters."),
    ] {
        let field = app.ui.find(&app.id("new-form")?, &["name"])?.ok_or("no name field")?;
        app.d().replace_text(&field, name)?;
        app.click_in("new-form", "create")?;
        assert_eq!(app.open_dialogs(), 1, "dialog closed for {name:?}");
        let shown = app.visible_names();
        assert!(shown.iter().any(|t| t == reason), "{name:?}: no {reason:?} in {shown:?}");
        assert_eq!(on_disk(&app.dir), before, "file system changed for {name:?}");
        assert_eq!(app.tabs()?.0, Vec::<String>::new());
    }
    assert_eq!(app.listed()?, ["apple.txt", "cherry.txt", "mango.txt", "zebra.txt"]);

    // A valid name: created empty, listed in order, opened and active.
    let field = app.ui.find(&app.id("new-form")?, &["name"])?.ok_or("no name field")?;
    app.d().replace_text(&field, "kiwi")?;
    app.click_in("new-form", "create")?;
    assert_eq!(app.open_dialogs(), 0);
    assert_eq!(app.file("kiwi.txt").as_deref(), Some(""));
    assert_eq!(app.listed()?, ["apple.txt", "cherry.txt", "kiwi.txt", "mango.txt", "zebra.txt"]);
    assert_eq!(app.tabs()?, (vec!["kiwi.txt".into()], Some("kiwi.txt".into())));
    assert_eq!(app.status()?, "kiwi.txt: 1 lines, 0 words");
    // The new tab's editor has focus: typing goes into it.
    app.d().type_text("fresh")?;
    assert_eq!(app.tabs()?.1.as_deref(), Some("kiwi.txt *"));

    // File > New note by pointer; the just-created name is now a duplicate.
    app.menu_click("New note")?;
    assert_eq!(app.open_dialogs(), 1);
    let field = app.ui.find(&app.id("new-form")?, &["name"])?.ok_or("no name field")?;
    app.d().replace_text(&field, "kiwi.txt")?;
    app.d().key(KeyCode::Enter)?;
    assert_eq!(app.open_dialogs(), 1);
    assert!(app.visible_names().iter().any(|t| t == "A note named kiwi.txt already exists."));
    // A 64-character name is accepted (boundary).
    let max = "q".repeat(64);
    app.d().replace_text(&field, &max)?;
    app.d().key(KeyCode::Enter)?;
    assert_eq!(app.open_dialogs(), 0);
    let file = format!("{max}.txt");
    assert_eq!(app.file(&file).as_deref(), Some(""));
    assert_eq!(app.tabs()?, (vec!["kiwi.txt *".into(), file.clone()], Some(file.clone())));
    Ok(())
}

// ---- Item 4 ---------------------------------------------------------------

#[test]
fn item4_close_choices_and_unmodified_close() -> Result {
    let mut app = App::start("item4")?;

    // Unmodified tab: Ctrl+W closes it without any dialog.
    app.click_row("mango.txt")?;
    app.click_row("zebra.txt")?;
    app.chord('w', KeyCode::KeyW)?;
    assert_eq!(app.open_dialogs(), 0);
    assert_eq!(app.tabs()?, (vec!["mango.txt".into()], Some("mango.txt".into())));
    // Unmodified tab: File > Close tab by pointer closes it without any dialog.
    app.menu_click("Close tab")?;
    assert_eq!(app.open_dialogs(), 0);
    assert_eq!(app.tabs()?, (vec![], None));
    assert_eq!(app.status()?, "");

    // Modified tab: Cancel keeps tab, text and file.
    app.click_row("apple.txt")?;
    app.d().type_text("A")?;
    app.d().key(KeyCode::Enter)?;
    assert_eq!(app.editor_text("apple.txt")?, "A\nred green\n\nyellow");
    app.chord('w', KeyCode::KeyW)?;
    assert_eq!(app.open_dialogs(), 1);
    assert!(!app.by_key("close-form").is_empty());
    app.click_in("close-form", "cancel")?;
    assert_eq!(app.open_dialogs(), 0);
    assert_eq!(app.tabs()?, (vec!["apple.txt *".into()], Some("apple.txt *".into())));
    assert_eq!(app.editor_text("apple.txt")?, "A\nred green\n\nyellow");
    assert_eq!(app.file("apple.txt").as_deref(), Some("red green\n\nyellow"));

    // Modified tab, File > Close tab by pointer, Discard: closed, file unchanged.
    app.menu_click("Close tab")?;
    assert_eq!(app.open_dialogs(), 1);
    app.click_in("close-form", "discard")?;
    assert_eq!(app.open_dialogs(), 0);
    assert_eq!(app.tabs()?, (vec![], None));
    assert_eq!(app.file("apple.txt").as_deref(), Some("red green\n\nyellow"));

    // Modified tab with another open: Save writes, then the tab disappears.
    app.click_row("cherry.txt")?;
    app.click_row("mango.txt")?;
    app.d().type_text("very ")?;
    app.chord('w', KeyCode::KeyW)?;
    assert_eq!(app.open_dialogs(), 1);
    // While the dialog is shown, nothing is written and the tab is still there.
    assert_eq!(app.file("mango.txt").as_deref(), Some("ripe"));
    assert_eq!(app.tabs()?.0, ["cherry.txt", "mango.txt *"]);
    app.click_in("close-form", "save")?;
    assert_eq!(app.open_dialogs(), 0);
    assert_eq!(app.file("mango.txt").as_deref(), Some("very ripe"));
    assert_eq!(app.tabs()?, (vec!["cherry.txt".into()], Some("cherry.txt".into())));
    assert_eq!(app.notifications()?, 0.0);

    // Modified tab, Save fails (folder removed): tab stays open and modified.
    app.d().type_text("z")?;
    assert_eq!(app.tabs()?.1.as_deref(), Some("cherry.txt *"));
    fs::remove_dir_all(&app.dir)?;
    app.chord('w', KeyCode::KeyW)?;
    assert_eq!(app.open_dialogs(), 1);
    app.click_in("close-form", "save")?;
    assert_eq!(app.tabs()?, (vec!["cherry.txt *".into()], Some("cherry.txt *".into())));
    assert_eq!(app.editor_text("cherry.txt")?, "z");
    assert_eq!(app.notifications()?, 1.0);
    Ok(())
}

// ---- Item 5 ---------------------------------------------------------------

#[test]
fn item5_cpu_capture_two_tabs_one_modified() -> Result {
    let mut app = App::start("item5")?;
    app.click_row("apple.txt")?;
    app.click_row("zebra.txt")?;
    app.d().type_text("Edited: ")?;
    assert_eq!(app.tabs()?, (vec!["apple.txt".into(), "zebra.txt *".into()], Some("zebra.txt *".into())));
    app.capture("verifier-two-tabs.png")?;
    Ok(())
}
