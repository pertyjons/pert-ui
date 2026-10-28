//! Author acceptance checks (independent verifier, round 3, task 6).
//!
//! Added in the acceptance copy only. They start the submitted application
//! through its existing public entry `notes::build_app_reporting` on a notes
//! folder of the verifier's own, drive it only through the library's input
//! (`Driver::click`, `type_text`, `key`, `chord`, raw `pointer_input` for menu
//! rows) and read results only through inspection (semantics, visibility,
//! bounds) and the files on disk. No application model function is called.

use pert_ui::prelude::*;
use pert_ui::testing::*;
use pert_ui::{
    ComponentInspection, Detail, InspectValue, InspectionQuery, KeyChord, PointerButton,
    PointerEvent, PointerKind, Selector, SemanticRole, Semantics,
};
use std::collections::BTreeSet;
use std::time::Duration;
use std::{cell::RefCell, path::PathBuf, rc::Rc};

type Res<T = ()> = Result<T, Box<dyn std::error::Error>>;

struct App {
    app: HeadlessApp,
    root: WidgetId,
    dir: PathBuf,
}

/// The verifier's own folder: notes created out of alphabetical order, plus
/// files that are not `.txt` notes.
fn own_folder(test: &str) -> Res<PathBuf> {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("verifier-{test}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir)?;
    // Creation order deliberately differs from name order.
    for (name, text) in [
        ("zebra.txt", "stripes and more stripes"),
        ("notes.md", "# not a note"),
        ("mango.txt", "ripe\nsweet"),
        ("kiwi.txt", ""),
        ("todo.txt.bak", "backup"),
        ("apple.txt", "red green"),
        ("image.png", "\u{89}PNG"),
    ] {
        std::fs::write(dir.join(name), text)?;
        std::thread::sleep(Duration::from_millis(5));
    }
    Ok(dir)
}

fn start(test: &str) -> Res<App> {
    let dir = own_folder(test)?;
    let slot = Rc::new(RefCell::new(None));
    let app = notes::build_app_reporting(dir.clone(), slot.clone())?.headless()?;
    let root = slot.borrow().clone().ok_or("nothing mounted")?;
    let mut a = App { app, root, dir };
    a.driver().settle()?;
    Ok(a)
}

fn files_on_disk(dir: &PathBuf) -> Res<BTreeSet<String>> {
    Ok(std::fs::read_dir(dir)?
        .map(|e| e.map(|e| e.file_name().to_string_lossy().into_owned()))
        .collect::<Result<_, _>>()?)
}

impl App {
    fn driver(&mut self) -> Driver<'_> {
        Driver::new(self.app.runtime_mut())
    }
    fn rt(&self) -> &Runtime {
        self.app.runtime()
    }
    fn find(&self, key: &str) -> Res<WidgetId> {
        Ok(self.rt().find_descendant(&self.root, key)?.ok_or(format!("no {key}"))?)
    }
    fn inspected(&self, id: &WidgetId) -> Res<ComponentInspection> {
        let iid = self.rt().inspection_id(id)?;
        let r = self.rt().query(
            &InspectionQuery::components(Selector::id(iid)).detail(Detail::Semantics),
        );
        Ok(r.components.into_iter().next().ok_or("not inspected")?)
    }
    fn semantics(&self, id: &WidgetId) -> Res<Semantics> {
        Ok(self.inspected(id)?.details.semantics.ok_or("no semantics")?)
    }
    fn snapshot(&self) -> Vec<ComponentInspection> {
        self.rt().inspect().components
    }

    /// Sidebar names in display order (rows sorted by their laid-out top).
    fn listed(&self) -> Res<Vec<String>> {
        let sidebar = self.find("sidebar")?;
        let mut rows = Vec::new();
        for row in self.rt().children(&sidebar)? {
            let top = self.rt().bounds(&row)?.ok_or("row not laid out")?.y();
            rows.push((top, self.semantics(&row)?.name.ok_or("unnamed row")?));
        }
        rows.sort_by(|a, b| a.0.total_cmp(&b.0));
        Ok(rows.into_iter().map(|r| r.1).collect())
    }

    /// Tab titles from the TabList's inspection, with the selected index.
    fn tabs(&self) -> Res<(Vec<String>, Option<usize>)> {
        let s = self.semantics(&self.find("tabs")?)?;
        assert_eq!(s.role, SemanticRole::TabList);
        let titles = s.items.iter().map(|i| i.name.clone().unwrap_or_default()).collect();
        Ok((titles, s.items.iter().position(|i| i.selected)))
    }

    fn status(&self) -> Res<String> {
        let status = self.find("status")?;
        let label = self.rt().children(&status)?.into_iter().next().ok_or("no label")?;
        Ok(self.semantics(&label)?.name.unwrap_or_default())
    }

    fn editor(&self, name: &str) -> Res<WidgetId> {
        let strip = self.find("tabs")?;
        Ok(self.rt().find(&strip, &[name, "editor"])?.ok_or(format!("no editor {name}"))?)
    }
    fn editor_text(&self, name: &str) -> Res<String> {
        match self.semantics(&self.editor(name)?)?.value {
            Some(SemanticValue::Text(t)) => Ok(t),
            other => Err(format!("editor value {other:?}").into()),
        }
    }

    fn pause(&mut self) -> Res {
        self.driver().advance(ClickCounter::DEFAULT_INTERVAL + Duration::from_millis(1))?;
        Ok(())
    }
    fn click(&mut self, id: &WidgetId) -> Res {
        self.driver().click(id)?;
        self.pause()?;
        self.driver().settle()?;
        Ok(())
    }
    fn click_row(&mut self, name: &str) -> Res {
        let sidebar = self.find("sidebar")?;
        let row = self.rt().find(&sidebar, &[name])?.ok_or(format!("no row {name}"))?;
        self.click(&row)
    }
    fn chord(&mut self, c: char, code: KeyCode) -> Res {
        self.driver().chord(KeyChord::primary(c), code)?;
        self.driver().settle()?;
        Ok(())
    }

    /// Open dialogs in the snapshot: Dialog/AlertDialog components that are
    /// visible and report `open = true`.
    fn open_dialogs(&self) -> usize {
        self.snapshot()
            .iter()
            .filter(|c| c.visible)
            .filter(|c| {
                c.details.semantics.as_ref().is_some_and(|s| {
                    matches!(s.role, SemanticRole::Dialog | SemanticRole::AlertDialog)
                })
            })
            .filter(|c| c.details.properties.get("open") == Some(&InspectValue::Bool(true)))
            .count()
    }
    fn form_visible(&self, key: &str) -> Res<bool> {
        Ok(self.inspected(&self.find(key)?)?.visible)
    }
    /// Visible Alert/Notifications components and their names.
    fn notifications(&self) -> Vec<(SemanticRole, Option<String>, Option<bool>)> {
        self.snapshot()
            .iter()
            .filter(|c| c.visible)
            .filter_map(|c| c.details.semantics.clone())
            .filter(|s| matches!(s.role, SemanticRole::Alert | SemanticRole::Notifications))
            .filter(|s| s.expanded != Some(false))
            .map(|s| (s.role, s.name, s.expanded))
            .collect()
    }
    fn visible_texts(&self) -> Vec<String> {
        let mut out = Vec::new();
        for c in self.snapshot().iter().filter(|c| c.visible) {
            if let Some(s) = &c.details.semantics {
                out.extend(s.name.clone());
                out.extend(s.description.clone());
                if let Some(SemanticValue::Text(t)) = &s.value {
                    out.push(t.clone());
                }
            }
        }
        out
    }

    /// Recursively finds the first descendant of `from` that is a visible
    /// Menu with rows.
    fn open_menu_list(&self, from: &WidgetId) -> Res<Option<WidgetId>> {
        for child in self.rt().children(from)? {
            let c = self.inspected(&child)?;
            if c.visible
                && let Some(s) = &c.details.semantics
                && s.role == SemanticRole::Menu
                && !s.items.is_empty()
            {
                return Ok(Some(child));
            }
            if let Some(found) = self.open_menu_list(&child)? {
                return Ok(Some(found));
            }
        }
        Ok(None)
    }

    /// Chooses File → `row` with the pointer: a simulated click on the File
    /// title in the menu bar, pointer moves over the open menu until the
    /// inspected highlight is `row`, then a primary press and release there.
    fn menu_click(&mut self, row: &str) -> Res {
        let title = self.find("title/file")?;
        self.click(&title)?;
        let list = self.open_menu_list(&title)?.ok_or("File menu did not open on click")?;
        let b = self.rt().bounds(&list)?.ok_or("menu not laid out")?;
        let x = b.x() + b.width() / 2.0;
        let mut y = b.y() + 1.0;
        let mut hit = None;
        while y < b.y() + b.height() {
            let at = Point::new(x, y)?;
            self.app.runtime_mut().pointer_input(PointerEvent::new(at, PointerKind::Move))?;
            self.driver().settle()?;
            let s = self.semantics(&list)?;
            if s.items.iter().any(|i| i.selected && i.name.as_deref() == Some(row)) {
                hit = Some(at);
                // Aim for the row's middle: keep moving while it stays highlighted.
                let start = y;
                let mut end = y;
                loop {
                    let next = end + 1.0;
                    if next >= b.y() + b.height() {
                        break;
                    }
                    self.app.runtime_mut().pointer_input(PointerEvent::new(
                        Point::new(x, next)?,
                        PointerKind::Move,
                    ))?;
                    self.driver().settle()?;
                    let s = self.semantics(&list)?;
                    if s.items.iter().any(|i| i.selected && i.name.as_deref() == Some(row)) {
                        end = next;
                    } else {
                        break;
                    }
                }
                let mid = Point::new(x, (start + end) / 2.0)?;
                self.app.runtime_mut().pointer_input(PointerEvent::new(mid, PointerKind::Move))?;
                hit = Some(mid).or(hit);
                break;
            }
            y += 2.0;
        }
        let at = hit.ok_or(format!("no menu row {row} under the pointer"))?;
        let rt = self.app.runtime_mut();
        rt.pointer_input(PointerEvent::new(at, PointerKind::Press(PointerButton::Primary)))?;
        rt.pointer_input(PointerEvent::new(at, PointerKind::Release(PointerButton::Primary)))?;
        self.driver().settle()?;
        self.pause()?;
        self.driver().settle()?;
        assert!(self.open_menu_list(&title)?.is_none(), "menu stayed open after choosing {row}");
        Ok(())
    }
}

fn counts(name: &str, text: &str) -> String {
    format!(
        "{name}: {} lines, {} words",
        text.matches('\n').count() + 1,
        text.split_whitespace().count()
    )
}

/// Item 1: own folder, listing and order, two notes opened, reopening
/// switches to the existing tab without adding one.
#[test]
fn item1_listing_and_reopen() -> Res {
    let mut a = start("item1")?;
    assert_eq!(a.listed()?, ["apple.txt", "kiwi.txt", "mango.txt", "zebra.txt"]);
    assert_eq!(a.tabs()?, (vec![], None));
    assert_eq!(a.status()?, "");

    a.click_row("zebra.txt")?;
    a.click_row("mango.txt")?;
    assert_eq!(a.tabs()?, (vec!["zebra.txt".into(), "mango.txt".into()], Some(1)));
    assert_eq!(a.status()?, "mango.txt: 2 lines, 2 words");

    // Reopen zebra by pointer: tab count unchanged, its tab active.
    a.click_row("zebra.txt")?;
    assert_eq!(a.tabs()?, (vec!["zebra.txt".into(), "mango.txt".into()], Some(0)));
    assert_eq!(a.status()?, "zebra.txt: 1 lines, 4 words");

    // Reopen mango by keyboard (arrow keys and Enter in the focused list).
    let sidebar = a.find("sidebar")?;
    a.driver().runtime_mut().focus(&sidebar)?;
    a.driver().key(KeyCode::Home)?;
    a.driver().key(KeyCode::ArrowDown)?;
    a.driver().key(KeyCode::ArrowDown)?;
    a.driver().key(KeyCode::Enter)?;
    a.driver().settle()?;
    assert_eq!(a.tabs()?, (vec!["zebra.txt".into(), "mango.txt".into()], Some(1)));
    a.app.shutdown()?;
    Ok(())
}

/// Item 2: typing with line breaks, marker and counts from inspection,
/// Ctrl+S and clicked File → Save with disk contents, failed save into a
/// removed folder shows a notification in inspection and keeps the marker.
#[test]
fn item2_typing_saving_and_failed_save() -> Res {
    let mut a = start("item2")?;
    a.click_row("kiwi.txt")?;
    a.click_row("apple.txt")?;
    a.click_row("kiwi.txt")?; // kiwi (empty) active
    assert_eq!(a.tabs()?, (vec!["kiwi.txt".into(), "apple.txt".into()], Some(0)));
    assert_eq!(a.status()?, "kiwi.txt: 1 lines, 0 words");

    let editor = a.editor("kiwi.txt")?;
    a.click(&editor)?;
    a.driver().type_text("first line")?;
    assert_eq!(a.tabs()?.0, ["kiwi.txt *", "apple.txt"]);
    assert_eq!(a.status()?, "kiwi.txt: 1 lines, 2 words");
    // A line break typed with the Enter key.
    a.driver().key(KeyCode::Enter)?;
    a.driver().type_text("second  line")?;
    // A line break inside typed text.
    a.driver().type_text("\nthird")?;
    let expected = "first line\nsecond  line\nthird";
    assert_eq!(a.editor_text("kiwi.txt")?, expected);
    assert_eq!(a.status()?, "kiwi.txt: 3 lines, 5 words");
    assert_eq!(a.status()?, counts("kiwi.txt", expected));
    assert_eq!(std::fs::read_to_string(a.dir.join("kiwi.txt"))?, "");

    // Ctrl+S.
    a.chord('s', KeyCode::KeyS)?;
    assert_eq!(std::fs::read_to_string(a.dir.join("kiwi.txt"))?, expected);
    assert_eq!(a.tabs()?.0, ["kiwi.txt", "apple.txt"]);

    // More typing, then a clicked File → Save.
    a.click(&editor)?;
    a.driver().key(KeyCode::Enter)?;
    a.driver().type_text("fourth")?;
    let text = a.editor_text("kiwi.txt")?;
    assert!(text.contains("fourth") && text.matches('\n').count() == 3, "{text:?}");
    assert_eq!(a.tabs()?.0, ["kiwi.txt *", "apple.txt"]);
    assert_eq!(a.status()?, counts("kiwi.txt", &text));
    a.menu_click("Save")?;
    assert_eq!(std::fs::read_to_string(a.dir.join("kiwi.txt"))?, text);
    assert_eq!(a.tabs()?.0, ["kiwi.txt", "apple.txt"]);
    // apple was never written.
    assert_eq!(std::fs::read_to_string(a.dir.join("apple.txt"))?, "red green");

    // Failed save: remove the folder, then Ctrl+S and the clicked menu item.
    assert!(a.notifications().is_empty(), "{:?}", a.notifications());
    a.click(&editor)?;
    a.driver().type_text("lost")?;
    assert_eq!(a.tabs()?.0, ["kiwi.txt *", "apple.txt"]);
    std::fs::remove_dir_all(&a.dir)?;
    a.chord('s', KeyCode::KeyS)?;
    let shown = a.notifications();
    eprintln!("notifications after failed Ctrl+S: {shown:?}");
    assert!(
        shown.iter().any(|(role, name, _)| *role == SemanticRole::Alert
            && name.as_deref().is_some_and(|n| n.contains("Could not save kiwi.txt"))),
        "{shown:?}"
    );
    assert_eq!(a.tabs()?.0, ["kiwi.txt *", "apple.txt"]);
    assert!(!a.dir.exists());

    // Dismiss, then fail again through the clicked menu item.
    let dismiss = a.find("dismiss")?;
    a.click(&dismiss)?;
    assert!(a.notifications().is_empty(), "{:?}", a.notifications());
    a.menu_click("Save")?;
    assert!(!a.notifications().is_empty());
    assert_eq!(a.tabs()?.0, ["kiwi.txt *", "apple.txt"]);
    a.app.shutdown()?;
    Ok(())
}

/// Item 3: New note dialog through Ctrl+N and the clicked menu item; refusals
/// visible in inspection with no file created; valid name creates, lists and
/// opens the note.
#[test]
fn item3_new_note_dialog() -> Res {
    let mut a = start("item3")?;
    let before = files_on_disk(&a.dir)?;
    assert_eq!(a.open_dialogs(), 0);
    a.chord('n', KeyCode::KeyN)?;
    assert_eq!(a.open_dialogs(), 1);
    assert!(a.form_visible("new-form")?);
    let field = a.find("name")?;
    let create = a.find("create")?;
    let error = a.find("error")?;

    let refuse = |a: &mut App, name: &str, needle: &str, submit_with_enter: bool| -> Res {
        a.driver().replace_text(&field, name)?;
        if submit_with_enter {
            a.driver().key(KeyCode::Enter)?;
        } else {
            a.click(&create)?;
        }
        a.driver().settle()?;
        assert_eq!(a.open_dialogs(), 1, "dialog closed after {name:?}");
        let label = a.inspected(&error)?;
        let text = label.details.semantics.and_then(|s| s.name).unwrap_or_default();
        assert!(label.visible, "error label hidden for {name:?}");
        assert!(text.contains(needle), "refusal for {name:?}: {text:?}");
        assert_eq!(files_on_disk(&a.dir)?, before, "a file appeared for {name:?}");
        Ok(())
    };
    refuse(&mut a, "", "Enter a name", false)?;
    refuse(&mut a, "a/b", "/", false)?;
    refuse(&mut a, "a\\b", "\\", true)?;
    refuse(&mut a, ".hidden", "dot", false)?;
    refuse(&mut a, &"x".repeat(65), "64", false)?;
    refuse(&mut a, "apple", "already exists", false)?;
    refuse(&mut a, "apple.txt", "already exists", true)?;
    assert_eq!(std::fs::read_to_string(a.dir.join("apple.txt"))?, "red green");

    // Cancel closes the dialog without creating anything.
    let cancel = a.rt().find(&a.find("new-form")?, &["actions", "cancel"])?.ok_or("no cancel")?;
    a.click(&cancel)?;
    assert_eq!(a.open_dialogs(), 0);
    assert_eq!(files_on_disk(&a.dir)?, before);

    // The clicked File → New note opens the same dialog; a valid name works.
    a.menu_click("New note")?;
    assert_eq!(a.open_dialogs(), 1);
    a.driver().replace_text(&field, "lemon")?;
    a.click(&create)?;
    a.driver().settle()?;
    assert_eq!(a.open_dialogs(), 0);
    let mut after = before.clone();
    after.insert("lemon.txt".into());
    assert_eq!(files_on_disk(&a.dir)?, after);
    assert_eq!(std::fs::read_to_string(a.dir.join("lemon.txt"))?, "");
    assert_eq!(a.listed()?, ["apple.txt", "kiwi.txt", "lemon.txt", "mango.txt", "zebra.txt"]);
    assert_eq!(a.tabs()?, (vec!["lemon.txt".into()], Some(0)));
    assert_eq!(a.status()?, "lemon.txt: 1 lines, 0 words");

    // A name already ending in .txt is not doubled; 64 characters is allowed.
    a.chord('n', KeyCode::KeyN)?;
    a.driver().replace_text(&field, "plum.txt")?;
    a.driver().key(KeyCode::Enter)?;
    a.driver().settle()?;
    assert!(a.dir.join("plum.txt").exists() && !a.dir.join("plum.txt.txt").exists());
    assert_eq!(a.tabs()?.0, ["lemon.txt", "plum.txt"]);
    a.chord('n', KeyCode::KeyN)?;
    a.driver().replace_text(&field, &"y".repeat(64))?;
    a.click(&create)?;
    a.driver().settle()?;
    assert!(a.dir.join(format!("{}.txt", "y".repeat(64))).exists());
    a.app.shutdown()?;
    Ok(())
}

/// Item 4: Cancel, Discard and Save on a modified tab (shortcut and clicked
/// menu item), files checked on disk; an unmodified tab closes with no dialog
/// in the snapshot; a failed Save keeps the tab.
#[test]
fn item4_close_choices() -> Res {
    let mut a = start("item4")?;
    a.click_row("apple.txt")?;
    a.click_row("mango.txt")?;
    a.click_row("zebra.txt")?;
    assert_eq!(a.tabs()?, (vec!["apple.txt".into(), "mango.txt".into(), "zebra.txt".into()], Some(2)));

    // Modify zebra.
    let zebra = a.editor("zebra.txt")?;
    a.click(&zebra)?;
    a.driver().type_text("X")?;
    let modified = a.editor_text("zebra.txt")?;
    assert_ne!(modified, "stripes and more stripes");
    assert_eq!(a.tabs()?.0, ["apple.txt", "mango.txt", "zebra.txt *"]);

    // Cancel (Ctrl+W): tab and text kept, file unchanged.
    a.chord('w', KeyCode::KeyW)?;
    assert_eq!(a.open_dialogs(), 1);
    assert!(a.visible_texts().iter().any(|t| t.contains("zebra.txt has unsaved changes")));
    let form = a.find("close-form")?;
    let cancel = a.rt().find(&form, &["actions", "cancel"])?.ok_or("no cancel")?;
    a.click(&cancel)?;
    assert_eq!(a.open_dialogs(), 0);
    assert_eq!(a.tabs()?, (vec!["apple.txt".into(), "mango.txt".into(), "zebra.txt *".into()], Some(2)));
    assert_eq!(a.editor_text("zebra.txt")?, modified);
    assert_eq!(std::fs::read_to_string(a.dir.join("zebra.txt"))?, "stripes and more stripes");

    // Discard (clicked File → Close tab): tab gone, file unchanged.
    a.menu_click("Close tab")?;
    assert_eq!(a.open_dialogs(), 1);
    let discard = a.rt().find(&form, &["actions", "discard"])?.ok_or("no discard")?;
    a.click(&discard)?;
    a.driver().settle()?;
    assert_eq!(a.open_dialogs(), 0);
    assert_eq!(a.tabs()?.0, ["apple.txt", "mango.txt"]);
    assert_eq!(std::fs::read_to_string(a.dir.join("zebra.txt"))?, "stripes and more stripes");

    // Save (Ctrl+W) on modified mango: written, then the tab closes.
    a.click_row("mango.txt")?;
    let mango = a.editor("mango.txt")?;
    a.click(&mango)?;
    a.driver().key(KeyCode::Enter)?;
    a.driver().type_text("juicy")?;
    let mango_text = a.editor_text("mango.txt")?;
    assert_eq!(a.tabs()?.0, ["apple.txt", "mango.txt *"]);
    a.chord('w', KeyCode::KeyW)?;
    assert_eq!(a.open_dialogs(), 1);
    let save = a.rt().find(&form, &["actions", "save"])?.ok_or("no save")?;
    a.click(&save)?;
    a.driver().settle()?;
    assert_eq!(a.open_dialogs(), 0);
    assert_eq!(a.tabs()?.0, ["apple.txt"]);
    assert_eq!(std::fs::read_to_string(a.dir.join("mango.txt"))?, mango_text);

    // Unmodified apple: Ctrl+W closes at once, no dialog in the snapshot.
    assert_eq!(a.tabs()?, (vec!["apple.txt".into()], Some(0)));
    a.chord('w', KeyCode::KeyW)?;
    assert_eq!(a.open_dialogs(), 0);
    assert!(!a.form_visible("close-form")?);
    assert_eq!(a.tabs()?.0, Vec::<String>::new());
    assert_eq!(a.status()?, "");
    // And through the clicked menu item.
    a.click_row("kiwi.txt")?;
    a.menu_click("Close tab")?;
    assert_eq!(a.open_dialogs(), 0);
    assert_eq!(a.tabs()?.0, Vec::<String>::new());
    assert_eq!(std::fs::read_to_string(a.dir.join("apple.txt"))?, "red green");
    assert_eq!(std::fs::read_to_string(a.dir.join("kiwi.txt"))?, "");

    // A failed Save in the close dialog keeps the tab and its marker.
    a.click_row("apple.txt")?;
    let apple = a.editor("apple.txt")?;
    a.click(&apple)?;
    a.driver().type_text("Z")?;
    std::fs::remove_dir_all(&a.dir)?;
    a.chord('w', KeyCode::KeyW)?;
    let save = a.rt().find(&form, &["actions", "save"])?.ok_or("no save")?;
    a.click(&save)?;
    a.driver().settle()?;
    assert_eq!(a.tabs()?.0, ["apple.txt *"]);
    assert!(!a.notifications().is_empty());
    a.app.shutdown()?;
    Ok(())
}

/// Item 5: the verifier's own CPU capture of two open notes, one modified.
#[test]
fn item5_cpu_capture() -> Res {
    let mut a = start("item5")?;
    a.click_row("apple.txt")?;
    a.click_row("mango.txt")?;
    let mango = a.editor("mango.txt")?;
    a.click(&mango)?;
    a.driver().type_text("verifier edit\n")?;
    assert_eq!(a.tabs()?, (vec!["apple.txt".into(), "mango.txt *".into()], Some(1)));
    let mut driver = a.driver();
    driver.runtime().set_capture_support(true);
    let window = driver.runtime().primary_window();
    let pending =
        driver.runtime().request_capture(CaptureRequest::new(CaptureTarget::window(&window)));
    driver.settle()?;
    CpuRenderer::new().serve_captures(driver.runtime_mut());
    let capture = pending.wait()?.value?;
    let out = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("acceptance-capture.png");
    std::fs::write(&out, capture.image.encode_png())?;
    assert_eq!((capture.image.width(), capture.image.height()), (900, 600));
    a.app.shutdown()?;
    Ok(())
}

/// Item 2 corroboration: a CPU capture of the window after a failed save,
/// to show the banner the inspection reports.
#[test]
fn item2_failed_save_capture() -> Res {
    let mut a = start("item2-capture")?;
    a.click_row("apple.txt")?;
    let apple = a.editor("apple.txt")?;
    a.click(&apple)?;
    a.driver().type_text(" blue")?;
    std::fs::remove_dir_all(&a.dir)?;
    a.chord('s', KeyCode::KeyS)?;
    assert_eq!(a.tabs()?.0, ["apple.txt *"]);
    assert!(!a.notifications().is_empty());
    let mut driver = a.driver();
    driver.runtime().set_capture_support(true);
    let window = driver.runtime().primary_window();
    let pending =
        driver.runtime().request_capture(CaptureRequest::new(CaptureTarget::window(&window)));
    driver.settle()?;
    CpuRenderer::new().serve_captures(driver.runtime_mut());
    let capture = pending.wait()?.value?;
    let out = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("acceptance-failed-save.png");
    std::fs::write(&out, capture.image.encode_png())?;
    a.app.shutdown()?;
    Ok(())
}
