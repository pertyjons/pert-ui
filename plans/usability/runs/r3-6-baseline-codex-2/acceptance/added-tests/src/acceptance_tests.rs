//! Independent acceptance checks (verifier-authored, not part of the submission).
//!
//! Every check starts the application the way `main` does (the folder is
//! listed, then `build` mounts the UI) on a folder of the verifier's own, then
//! drives it only through pert-ui's `Driver` (pointer clicks, typed text, keys
//! and chords), raw `Runtime::pointer_input` for menu rows, and reads results
//! through inspection (`Runtime::inspect`, `Driver::semantics`), CPU captures
//! and the files on disk. No application model function is called after
//! startup.
use super::*;
use pert_ui::testing::*;
use pert_ui::{ComponentInspection, SemanticRole, Semantics};
use std::error::Error;

type TestResult = Result<(), Box<dyn Error>>;

struct Acc {
    ui: Runtime,
    root: WidgetId,
    folder: PathBuf,
}

impl Acc {
    /// Creates `/work/acc-<tag>-<pid>`, writes `files` in the given order and
    /// starts the application on it as `main` does.
    fn start(tag: &str, files: &[(&str, &str)]) -> Result<Self, Box<dyn Error>> {
        let folder = PathBuf::from(format!("/work/acc-{tag}-{}", std::process::id()));
        if folder.exists() {
            fs::remove_dir_all(&folder)?;
        }
        fs::create_dir_all(&folder)?;
        for (name, text) in files {
            if let Some(dir) = name.strip_suffix('/') {
                fs::create_dir(folder.join(dir))?;
            } else {
                fs::write(folder.join(name), text)?;
            }
        }
        let mut ui = Runtime::new();
        ui.set_viewport(Viewport::new(1000.0, 700.0, 1.0)?)?;
        // Startup exactly as main(): list the folder, then mount the UI.
        let names = Editor::list(&folder)?;
        let root = build(&mut ui, folder.clone(), names)?;
        let mut acc = Self { ui, root, folder };
        acc.settle()?;
        Ok(acc)
    }
    fn d(&mut self) -> Driver<'_> {
        Driver::new(&mut self.ui)
    }
    fn settle(&mut self) -> TestResult {
        self.d().settle()?;
        self.d().settle()?;
        Ok(())
    }
    fn find(&self, key: &str) -> Result<WidgetId, Box<dyn Error>> {
        Ok(self
            .ui
            .find_descendant(&self.root, key)?
            .ok_or_else(|| format!("no component keyed {key}"))?)
    }
    fn find_in(&self, parent: &str, key: &str) -> Result<WidgetId, Box<dyn Error>> {
        let parent = self.find(parent)?;
        Ok(self
            .ui
            .find_descendant(&parent, key)?
            .ok_or_else(|| format!("no component keyed {key}"))?)
    }
    fn inspected(&self, id: &WidgetId) -> Result<ComponentInspection, Box<dyn Error>> {
        let iid = self.ui.inspection_id(id)?;
        Ok(self
            .ui
            .inspect()
            .components
            .into_iter()
            .find(|c| c.id == iid)
            .ok_or("component missing from inspection")?)
    }
    fn visible(&self, id: &WidgetId) -> Result<bool, Box<dyn Error>> {
        Ok(self.inspected(id)?.visible)
    }
    fn sem(&mut self, id: &WidgetId) -> Result<Semantics, Box<dyn Error>> {
        Ok(self.d().semantics(id)?)
    }
    fn click(&mut self, id: &WidgetId) -> TestResult {
        self.d().click(id)?;
        self.settle()
    }
    fn key(&mut self, code: KeyCode) -> TestResult {
        self.d().key(code)?;
        self.settle()
    }
    fn chord(&mut self, letter: char, code: KeyCode) -> TestResult {
        self.d().chord(KeyChord::primary(letter), code)?;
        self.settle()
    }
    fn type_text(&mut self, text: &str) -> TestResult {
        self.d().type_text(text)?;
        self.settle()
    }

    /// The sidebar's rows as inspection shows them: names of the visible
    /// components under the "notes" list, in on-screen (top to bottom) order.
    fn listed(&self) -> Result<Vec<String>, Box<dyn Error>> {
        let list = self.ui.inspection_id(&self.find("notes")?)?;
        let mut rows: Vec<(f32, String)> = self
            .ui
            .inspect()
            .components
            .into_iter()
            .filter(|c| c.parent == Some(list) && c.visible)
            .map(|c| {
                let y = c.details.box_model.border_box.map_or(f32::NAN, |b| b[1]);
                let name = c.details.semantics.and_then(|s| s.name).unwrap_or_default();
                (y, name)
            })
            .collect();
        rows.sort_by(|a, b| a.0.total_cmp(&b.0));
        Ok(rows.into_iter().map(|(_, n)| n).collect())
    }
    fn row(&self, name: &str) -> Result<WidgetId, Box<dyn Error>> {
        self.find_in("notes", name)
    }
    /// Tab titles and which one is selected, from the tab list's semantics.
    fn tabs(&mut self) -> Result<Vec<(String, bool)>, Box<dyn Error>> {
        let tabs = self.find("tabs")?;
        Ok(self
            .sem(&tabs)?
            .items
            .into_iter()
            .map(|i| (i.name.unwrap_or_default(), i.selected))
            .collect())
    }
    fn active_tab(&mut self) -> Result<Option<String>, Box<dyn Error>> {
        Ok(self.tabs()?.into_iter().find(|t| t.1).map(|t| t.0))
    }
    /// The status line's text from inspection; "" when it shows nothing.
    fn status(&mut self) -> Result<String, Box<dyn Error>> {
        let counts = self.find("counts")?;
        Ok(self.sem(&counts)?.name.unwrap_or_default())
    }
    fn editor(&self, note: &str) -> Result<WidgetId, Box<dyn Error>> {
        let tab = self.find_in("tabs", note)?;
        Ok(self
            .ui
            .find_descendant(&tab, "editor")?
            .ok_or("no editor in tab")?)
    }
    fn editor_text(&mut self, note: &str) -> Result<String, Box<dyn Error>> {
        let ed = self.editor(note)?;
        match self.sem(&ed)?.value {
            Some(SemanticValue::Text(t)) => Ok(t),
            other => Err(format!("editor value {other:?}").into()),
        }
    }
    /// Clicks into the note's editor and moves the caret to the end of its
    /// (single-line) text with the End key.
    fn focus_editor_end(&mut self, note: &str) -> TestResult {
        let ed = self.editor(note)?;
        self.click(&ed)?;
        assert!(self.inspected(&ed)?.focused, "editor focused after click");
        self.key(KeyCode::End)
    }
    /// Whether any dialog surface (the "dialog" part of a library Dialog) or
    /// either form is visible in the inspection snapshot.
    fn any_dialog_visible(&self) -> bool {
        let snap = self.ui.inspect();
        let dialogs: Vec<u64> = snap
            .components
            .iter()
            .filter(|c| {
                c.details
                    .semantics
                    .as_ref()
                    .is_some_and(|s| matches!(s.role, SemanticRole::Dialog | SemanticRole::AlertDialog))
            })
            .map(|c| c.id)
            .collect();
        snap.components.iter().any(|c| {
            c.visible
                && ((c.key == "dialog" && c.parent.is_some_and(|p| dialogs.contains(&p)))
                    || c.key == "new-form"
                    || c.key == "close-form")
        })
    }
    /// The "dialog" part directly under the library Dialog keyed `outer`.
    fn dialog_part(&self, outer: &str) -> Result<WidgetId, Box<dyn Error>> {
        let outer = self.find(outer)?;
        for child in self.ui.children(&outer)? {
            if self.ui.key(&child)? == "dialog" {
                return Ok(child);
            }
        }
        Err("no dialog part".into())
    }
    fn new_dialog_open(&self) -> Result<bool, Box<dyn Error>> {
        let dialog = self.dialog_part("new-dialog")?;
        let role = self
            .inspected(&self.find("new-dialog")?)?
            .details
            .semantics
            .map(|s| s.role);
        assert_eq!(role, Some(SemanticRole::Dialog), "New note uses a library Dialog");
        Ok(self.visible(&dialog)? && self.visible(&self.find("new-form")?)?)
    }
    fn close_dialog_open(&self) -> Result<bool, Box<dyn Error>> {
        let dialog = self.dialog_part("close-dialog")?;
        let role = self
            .inspected(&self.find("close-dialog")?)?
            .details
            .semantics
            .map(|s| s.role);
        assert_eq!(role, Some(SemanticRole::Dialog), "close confirmation is a library Dialog");
        Ok(self.visible(&dialog)? && self.visible(&self.find("close-form")?)?)
    }
    /// The toast stack's notification count, from its Notifications semantics.
    fn notifications(&self) -> Result<f64, Box<dyn Error>> {
        let stack = self
            .ui
            .inspect()
            .components
            .into_iter()
            .find(|c| c.key == "toasts")
            .ok_or("no toast stack")?;
        let sem = stack.details.semantics.ok_or("toast stack without semantics")?;
        assert_eq!(sem.role, SemanticRole::Notifications);
        match sem.value {
            Some(SemanticValue::Number { value, .. }) => Ok(value),
            other => Err(format!("toast value {other:?}").into()),
        }
    }

    /// Clicks File in the menu bar with the pointer, then clicks the menu row
    /// named `item`: the pointer moves down the open menu until the menu's own
    /// semantics highlight that row, then presses and releases there.
    fn menu(&mut self, item: &str) -> TestResult {
        let title = self.find("title/file")?;
        self.click(&title)?;
        let menu = self
            .ui
            .inspect()
            .components
            .into_iter()
            .find(|c| {
                c.visible
                    && c.key == "items"
                    && c.details.semantics.as_ref().is_some_and(|s| {
                        s.role == SemanticRole::Menu
                            && s.items.iter().any(|i| i.name.as_deref() == Some("New note"))
                    })
            })
            .ok_or("File menu did not open on click")?;
        let [x, y, w, h] = menu.details.box_model.border_box.ok_or("menu not laid out")?;
        let cx = x + w / 2.0;
        let mut at = None;
        let mut py = y + 1.0;
        while py < y + h {
            let p = Point::new(cx, py)?;
            self.ui.pointer_input(PointerEvent::new(p, PointerKind::Move))?;
            self.d().settle()?;
            let sem = self
                .ui
                .inspect()
                .components
                .into_iter()
                .find(|c| c.id == menu.id)
                .and_then(|c| c.details.semantics)
                .ok_or("menu vanished")?;
            if sem
                .items
                .iter()
                .any(|i| i.selected && i.name.as_deref() == Some(item))
            {
                at = Some(p);
                break;
            }
            py += 2.0;
        }
        let p = at.ok_or_else(|| format!("pointer never highlighted menu row {item}"))?;
        self.ui
            .pointer_input(PointerEvent::new(p, PointerKind::Press(PointerButton::Primary)))?;
        self.ui
            .pointer_input(PointerEvent::new(p, PointerKind::Release(PointerButton::Primary)))?;
        self.settle()?;
        let still_open = self
            .ui
            .inspect()
            .components
            .into_iter()
            .any(|c| c.id == menu.id && c.visible);
        assert!(!still_open, "menu closes after choosing {item}");
        Ok(())
    }

    fn disk(&self, name: &str) -> Option<String> {
        fs::read_to_string(self.folder.join(name)).ok()
    }
    fn dir_entries(&self) -> Vec<String> {
        let mut v: Vec<String> = fs::read_dir(&self.folder)
            .map(|r| {
                r.filter_map(|e| e.ok())
                    .map(|e| e.file_name().to_string_lossy().into_owned())
                    .collect()
            })
            .unwrap_or_default();
        v.sort();
        v
    }
    fn capture(&mut self, path: &str) -> Result<FrameImage, Box<dyn Error>> {
        self.ui.set_capture_support(true);
        let pending = self
            .ui
            .request_capture(CaptureRequest::new(CaptureTarget::window(&self.ui.primary_window())));
        self.settle()?;
        let served = CpuRenderer::new().serve_captures(&mut self.ui);
        assert!(served >= 1, "CPU renderer served the capture");
        let capture = pending.wait()?.value?;
        fs::write(path, capture.image.encode_png())?;
        Ok(capture.image)
    }
}

impl Drop for Acc {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.folder);
    }
}

fn s(v: &[(&str, bool)]) -> Vec<(String, bool)> {
    v.iter().map(|(n, b)| ((*n).to_owned(), *b)).collect()
}

/// Item 1: listing from the verifier's folder; opening by pointer and by
/// arrow keys + Enter; reopening switches without a second tab.
#[test]
#[ignore]
fn probe_focus_order() -> TestResult {
    let mut a = Acc::start("probe", &[("b.txt", "b"), ("a.txt", "a"), ("c.txt", "c")])?;
    for i in 0..12 {
        let f: Vec<String> = a.ui.inspect().components.iter().filter(|c| c.focused).map(|c| format!("{}#{} parent={:?}", c.key, c.id, c.parent)).collect();
        println!("tab {i}: focused {f:?}");
        a.key(KeyCode::Tab)?;
    }
    let list = a.ui.inspection_id(&a.find("notes")?)?;
    println!("list id {list}");
    Ok(())
}

#[test]
fn item1_listing_open_and_reopen() -> TestResult {
    // Created out of alphabetical order, with non-.txt files and a directory
    // whose name ends in .txt.
    let mut a = Acc::start(
        "item1",
        &[
            ("pear.txt", "pear"),
            ("zucchini.txt", "zucchini"),
            ("readme.md", "not a note"),
            ("apple.txt", "apple"),
            ("photo.png", "png"),
            ("mango.txt", "mango"),
            ("old.txt.bak", "backup"),
            ("archive.txt/", ""),
        ],
    )?;
    let listed = a.listed()?;
    println!("listed: {listed:?}");
    assert_eq!(listed, ["apple.txt", "mango.txt", "pear.txt", "zucchini.txt"]);
    let list_sem = a.sem(&a.find("notes")?)?;
    assert_eq!(list_sem.role, SemanticRole::List);
    assert!(matches!(list_sem.value, Some(SemanticValue::Choice { count: 4, .. })));
    assert!(a.tabs()?.is_empty());
    assert_eq!(a.status()?, "", "status line empty with no tab open");

    // Pointer.
    a.click(&a.row("pear.txt")?)?;
    assert_eq!(a.tabs()?, s(&[("pear.txt", true)]));
    assert_eq!(a.editor_text("pear.txt")?, "pear");

    // Keyboard: Tab until the list itself has focus, then arrows + Enter.
    let list = a.find("notes")?;
    let list_iid = a.ui.inspection_id(&list)?;
    let tab_to_list = |a: &mut Acc| -> Result<usize, Box<dyn Error>> {
        let focused = |a: &Acc| a.ui.inspect().components.iter().any(|c| c.focused && c.id == list_iid);
        let mut n = 0;
        while !focused(a) && n < 30 {
            a.key(KeyCode::Tab)?;
            n += 1;
        }
        assert!(focused(a), "sidebar list reachable with Tab");
        Ok(n)
    };
    println!("Tab presses to reach the sidebar: {}", tab_to_list(&mut a)?);
    // Go to the first row, then down to mango.
    for _ in 0..4 {
        a.key(KeyCode::ArrowUp)?;
    }
    a.key(KeyCode::ArrowDown)?;
    a.key(KeyCode::Enter)?;
    assert_eq!(a.tabs()?, s(&[("pear.txt", false), ("mango.txt", true)]));
    a.key(KeyCode::ArrowDown)?;
    a.key(KeyCode::ArrowDown)?;
    a.key(KeyCode::Enter)?;
    assert_eq!(
        a.tabs()?,
        s(&[("pear.txt", false), ("mango.txt", false), ("zucchini.txt", true)])
    );

    // Reopen by pointer: count unchanged, its tab active.
    a.click(&a.row("pear.txt")?)?;
    assert_eq!(
        a.tabs()?,
        s(&[("pear.txt", true), ("mango.txt", false), ("zucchini.txt", false)])
    );
    assert_eq!(a.status()?, "pear.txt: 1 lines, 1 words");
    // Reopen by keyboard: focus back in the list, select mango, Enter.
    println!("Tab presses to reach the sidebar again: {}", tab_to_list(&mut a)?);
    for _ in 0..4 {
        a.key(KeyCode::ArrowUp)?;
    }
    a.key(KeyCode::ArrowDown)?;
    a.key(KeyCode::Enter)?;
    assert_eq!(
        a.tabs()?,
        s(&[("pear.txt", false), ("mango.txt", true), ("zucchini.txt", false)])
    );
    assert_eq!(a.status()?, "mango.txt: 1 lines, 1 words");
    Ok(())
}

/// Item 2: typing with line breaks, marker and counts from inspection, save
/// with Ctrl+S and with a clicked File → Save, a failing save.
#[test]
fn item2_typing_save_and_failed_save() -> TestResult {
    let mut a = Acc::start(
        "item2",
        &[("mango.txt", "ripe mango"), ("apple.txt", "apple"), ("x.md", "")],
    )?;
    a.click(&a.row("mango.txt")?)?;
    assert_eq!(a.tabs()?, s(&[("mango.txt", true)]));
    assert_eq!(a.status()?, "mango.txt: 1 lines, 2 words");

    a.focus_editor_end("mango.txt")?;
    a.type_text(" is sweet")?;
    assert_eq!(a.editor_text("mango.txt")?, "ripe mango is sweet");
    assert_eq!(a.tabs()?, s(&[("mango.txt *", true)]), "marker after typing");
    assert_eq!(a.status()?, "mango.txt: 1 lines, 4 words");
    a.key(KeyCode::Enter)?;
    assert_eq!(a.status()?, "mango.txt: 2 lines, 4 words", "Enter adds a line");
    a.type_text("second line")?;
    a.key(KeyCode::Enter)?;
    a.key(KeyCode::Enter)?;
    a.type_text("  four\tfive  ")?;
    let typed = "ripe mango is sweet\nsecond line\n\n  four\tfive  ";
    assert_eq!(a.editor_text("mango.txt")?, typed);
    assert_eq!(a.status()?, "mango.txt: 4 lines, 8 words");
    assert_eq!(a.disk("mango.txt").as_deref(), Some("ripe mango"), "not saved yet");

    // Ctrl+S through the key path.
    a.chord('s', KeyCode::KeyS)?;
    assert_eq!(a.disk("mango.txt").as_deref(), Some(typed));
    assert_eq!(a.tabs()?, s(&[("mango.txt", true)]), "marker cleared by Ctrl+S");
    assert_eq!(a.status()?, "mango.txt: 4 lines, 8 words");

    // More typing, then File → Save clicked in the menu bar.
    a.focus_editor_end("mango.txt")?;
    a.key(KeyCode::Enter)?;
    a.type_text("from menu")?;
    let typed2 = format!("{typed}\nfrom menu");
    assert_eq!(a.editor_text("mango.txt")?, typed2);
    assert_eq!(a.tabs()?, s(&[("mango.txt *", true)]));
    assert_eq!(a.status()?, "mango.txt: 5 lines, 10 words");
    a.menu("Save")?;
    assert_eq!(a.disk("mango.txt").as_deref(), Some(typed2.as_str()));
    assert_eq!(a.tabs()?, s(&[("mango.txt", true)]), "marker cleared by menu Save");

    // Save writes only the active tab.
    a.click(&a.row("apple.txt")?)?;
    a.focus_editor_end("apple.txt")?;
    a.type_text(" pie")?;
    assert_eq!(a.tabs()?, s(&[("mango.txt", false), ("apple.txt *", true)]));

    // Failing save: the folder is removed.
    let before = a.notifications()?;
    assert_eq!(before, 0.0);
    fs::remove_dir_all(&a.folder)?;
    a.chord('s', KeyCode::KeyS)?;
    let after_shortcut = a.notifications()?;
    println!("notifications before={before} after Ctrl+S failure={after_shortcut}");
    assert_eq!(after_shortcut, before + 1.0, "failure notification in inspection");
    let stack = a
        .ui
        .inspect()
        .components
        .into_iter()
        .find(|c| c.key == "toasts")
        .unwrap();
    assert!(stack.visible, "toast stack visible");
    assert_eq!(a.tabs()?, s(&[("mango.txt", false), ("apple.txt *", true)]), "marker kept");
    assert_eq!(a.editor_text("apple.txt")?, "apple pie");
    assert!(a.disk("apple.txt").is_none());
    // Capture for the displayed failure message.
    let img = a.capture("/work/acceptance-save-failure.png")?;
    assert_eq!((img.width(), img.height()), (1000, 700));

    // The menu item fails the same way.
    a.menu("Save")?;
    assert_eq!(a.notifications()?, after_shortcut + 1.0);
    assert_eq!(a.tabs()?, s(&[("mango.txt", false), ("apple.txt *", true)]));
    Ok(())
}

/// Item 3: New note dialog refusals and a valid creation, via clicked menu
/// item and via Ctrl+N.
#[test]
fn item3_new_note_dialog() -> TestResult {
    let mut a = Acc::start("item3", &[("pear.txt", "p"), ("apple.txt", "a"), ("notes.md", "")])?;
    let files0 = a.dir_entries();
    assert!(!a.new_dialog_open()?);

    let open_dialog = |a: &mut Acc, via_menu: bool| -> TestResult {
        if via_menu {
            a.menu("New note")?;
        } else {
            a.chord('n', KeyCode::KeyN)?;
        }
        assert!(a.new_dialog_open()?, "New note dialog opens");
        let err = a.find_in("new-form", "error")?;
        assert_eq!(a.sem(&err)?.name.unwrap_or_default(), "", "no stale explanation");
        Ok(())
    };
    let refuse = |a: &mut Acc, name: Option<&str>, expect: &str| -> TestResult {
        if let Some(name) = name {
            let field = a.find_in("new-form", "name")?;
            a.click(&field)?;
            a.type_text(name)?;
            assert_eq!(
                a.sem(&field)?.value,
                Some(SemanticValue::Text(name.into())),
                "typed name in field"
            );
        }
        a.click(&a.find_in("new-form", "create")?)?;
        assert!(a.new_dialog_open()?, "dialog stays open after refusal of {name:?}");
        let err = a.find_in("new-form", "error")?;
        let shown = a.sem(&err)?.name.unwrap_or_default();
        println!("refused {name:?}: {shown:?}");
        assert!(a.visible(&err)?, "explanation visible");
        assert!(shown.contains(expect), "explanation {shown:?} contains {expect:?}");
        a.click(&a.find_in("new-form", "cancel")?)?;
        assert!(!a.new_dialog_open()?, "Cancel closes the dialog");
        Ok(())
    };

    // Empty, via the clicked menu item.
    open_dialog(&mut a, true)?;
    refuse(&mut a, None, "1–64")?;
    // A name with '/', via Ctrl+N.
    open_dialog(&mut a, false)?;
    refuse(&mut a, Some("sub/dir"), "/")?;
    // Existing note, with and without the extension.
    open_dialog(&mut a, true)?;
    refuse(&mut a, Some("apple"), "already exists")?;
    open_dialog(&mut a, false)?;
    refuse(&mut a, Some("pear.txt"), "already exists")?;
    // Further rules from the feature list.
    open_dialog(&mut a, false)?;
    refuse(&mut a, Some("back\\slash"), "\\")?;
    open_dialog(&mut a, false)?;
    refuse(&mut a, Some(".hidden"), "dot")?;
    open_dialog(&mut a, false)?;
    refuse(&mut a, Some(&"x".repeat(65)), "1–64")?;
    assert_eq!(a.dir_entries(), files0, "no file created by any refusal");
    assert_eq!(a.listed()?, ["apple.txt", "pear.txt"]);
    assert!(a.tabs()?.is_empty());

    // Valid name via the clicked menu item: created, listed, opened.
    open_dialog(&mut a, true)?;
    let field = a.find_in("new-form", "name")?;
    a.click(&field)?;
    a.type_text("kiwi")?;
    a.click(&a.find_in("new-form", "create")?)?;
    assert!(!a.new_dialog_open()?, "dialog closes on success");
    assert_eq!(a.disk("kiwi.txt").as_deref(), Some(""), "empty file created");
    assert_eq!(a.listed()?, ["apple.txt", "kiwi.txt", "pear.txt"]);
    assert_eq!(a.tabs()?, s(&[("kiwi.txt", true)]));
    assert_eq!(a.status()?, "kiwi.txt: 1 lines, 0 words");

    // Valid name already ending in .txt via Ctrl+N; a 64-character name.
    open_dialog(&mut a, false)?;
    let field = a.find_in("new-form", "name")?;
    a.click(&field)?;
    a.type_text("banana.txt")?;
    a.click(&a.find_in("new-form", "create")?)?;
    assert!(!a.new_dialog_open()?);
    assert_eq!(a.disk("banana.txt").as_deref(), Some(""));
    assert!(a.disk("banana.txt.txt").is_none());
    assert_eq!(a.tabs()?, s(&[("kiwi.txt", false), ("banana.txt", true)]));
    let long = "y".repeat(64);
    open_dialog(&mut a, false)?;
    let field = a.find_in("new-form", "name")?;
    a.click(&field)?;
    a.type_text(&long)?;
    a.click(&a.find_in("new-form", "create")?)?;
    assert!(!a.new_dialog_open()?, "64 characters accepted");
    assert_eq!(a.disk(&format!("{long}.txt")).as_deref(), Some(""));
    assert_eq!(
        a.listed()?,
        ["apple.txt", "banana.txt", "kiwi.txt", "pear.txt", &format!("{long}.txt")]
    );
    Ok(())
}

/// Item 4: Cancel, Discard and Save for a modified tab (shortcut and clicked
/// menu item), a failed Save on close, and an unmodified tab closing without
/// a dialog.
#[test]
fn item4_close_choices() -> TestResult {
    let mut a = Acc::start(
        "item4",
        &[("pear.txt", "pear text"), ("mango.txt", "mango text"), ("apple.txt", "apple text")],
    )?;
    for n in ["apple.txt", "mango.txt", "pear.txt"] {
        a.click(&a.row(n)?)?;
    }
    assert_eq!(
        a.tabs()?,
        s(&[("apple.txt", false), ("mango.txt", false), ("pear.txt", true)])
    );

    // Cancel (Ctrl+W).
    a.focus_editor_end("pear.txt")?;
    a.type_text(" edited")?;
    assert_eq!(a.active_tab()?.as_deref(), Some("pear.txt *"));
    assert!(!a.any_dialog_visible());
    a.chord('w', KeyCode::KeyW)?;
    assert!(a.close_dialog_open()?, "modified tab asks");
    assert!(a.any_dialog_visible(), "positive control for the no-dialog check");
    a.click(&a.find_in("close-form", "cancel")?)?;
    assert!(!a.close_dialog_open()?);
    assert_eq!(
        a.tabs()?,
        s(&[("apple.txt", false), ("mango.txt", false), ("pear.txt *", true)])
    );
    assert_eq!(a.editor_text("pear.txt")?, "pear text edited", "Cancel keeps text");
    assert_eq!(a.disk("pear.txt").as_deref(), Some("pear text"));

    // Discard (clicked File → Close tab).
    a.menu("Close tab")?;
    assert!(a.close_dialog_open()?, "menu Close tab asks for a modified tab");
    a.click(&a.find_in("close-form", "discard")?)?;
    assert!(!a.close_dialog_open()?);
    let titles: Vec<String> = a.tabs()?.into_iter().map(|t| t.0).collect();
    assert_eq!(titles, ["apple.txt", "mango.txt"], "Discard closes the tab");
    assert_eq!(a.disk("pear.txt").as_deref(), Some("pear text"), "Discard writes nothing");

    // Save (Ctrl+W) on mango.
    a.click(&a.row("mango.txt")?)?;
    assert_eq!(a.active_tab()?.as_deref(), Some("mango.txt"));
    a.focus_editor_end("mango.txt")?;
    a.key(KeyCode::Enter)?;
    a.type_text("saved on close")?;
    a.chord('w', KeyCode::KeyW)?;
    assert!(a.close_dialog_open()?);
    a.click(&a.find_in("close-form", "save")?)?;
    assert!(!a.close_dialog_open()?);
    assert_eq!(a.disk("mango.txt").as_deref(), Some("mango text\nsaved on close"));
    let titles: Vec<String> = a.tabs()?.into_iter().map(|t| t.0).collect();
    assert_eq!(titles, ["apple.txt"], "Save closes the tab");

    // Unmodified tab with Ctrl+W: closes at once, no dialog in the snapshot.
    assert_eq!(a.tabs()?, s(&[("apple.txt", true)]));
    a.chord('w', KeyCode::KeyW)?;
    assert!(!a.any_dialog_visible(), "no dialog for an unmodified tab");
    assert!(a.tabs()?.is_empty());
    assert_eq!(a.status()?, "", "status empty with no tab");
    assert_eq!(a.disk("apple.txt").as_deref(), Some("apple text"));

    // Unmodified tab with the clicked menu item.
    a.click(&a.row("apple.txt")?)?;
    a.click(&a.row("pear.txt")?)?;
    a.menu("Close tab")?;
    assert!(!a.any_dialog_visible(), "no dialog via menu either");
    assert_eq!(a.tabs()?, s(&[("apple.txt", true)]));

    // Failed Save on close keeps the tab open and modified.
    a.focus_editor_end("apple.txt")?;
    a.type_text(" lost?")?;
    let before = a.notifications()?;
    fs::remove_dir_all(&a.folder)?;
    a.chord('w', KeyCode::KeyW)?;
    assert!(a.close_dialog_open()?);
    a.click(&a.find_in("close-form", "save")?)?;
    assert_eq!(a.notifications()?, before + 1.0, "failure notified");
    assert_eq!(a.tabs()?, s(&[("apple.txt *", true)]), "failed Save keeps the tab");
    assert_eq!(a.editor_text("apple.txt")?, "apple text lost?");
    println!("dialog open after failed Save: {}", a.close_dialog_open()?);
    if a.close_dialog_open()? {
        a.click(&a.find_in("close-form", "cancel")?)?;
    }
    assert_eq!(a.tabs()?, s(&[("apple.txt *", true)]));
    Ok(())
}

/// Item 5: CPU capture with two notes open, one modified.
#[test]
fn item5_cpu_capture_two_tabs_one_modified() -> TestResult {
    let mut a = Acc::start("item5", &[("plan.txt", "Plan for today"), ("ideas.txt", "Ideas")])?;
    a.click(&a.row("ideas.txt")?)?;
    a.click(&a.row("plan.txt")?)?;
    a.focus_editor_end("plan.txt")?;
    a.key(KeyCode::Enter)?;
    a.type_text("- verify the notes editor")?;
    assert_eq!(a.tabs()?, s(&[("ideas.txt", false), ("plan.txt *", true)]));
    let img = a.capture("/work/acceptance-two-tabs.png")?;
    assert_eq!((img.width(), img.height()), (1000, 700));
    Ok(())
}

/// Feature-list observation (not a numbered item): dragging the split
/// divider with the pointer resizes the sidebar.
#[test]
fn feature_divider_drag_resizes_sidebar() -> TestResult {
    let mut a = Acc::start("divider", &[("a.txt", "a")])?;
    let list = a.find("notes")?;
    let tabs = a.find("tabs")?;
    let [lx, _, lw, _] = a.inspected(&list)?.details.box_model.border_box.ok_or("no box")?;
    let [tx, ty, _, th] = a.inspected(&tabs)?.details.box_model.border_box.ok_or("no box")?;
    println!("sidebar x={lx} w={lw}; tabs x={tx}");
    // The divider lies between the sidebar's right edge and the tabs' left edge.
    let gx = (lx + lw + tx) / 2.0;
    let y = ty + th / 2.0;
    let press = Point::new(gx, y)?;
    a.ui.pointer_input(PointerEvent::new(press, PointerKind::Move))?;
    a.ui.pointer_input(PointerEvent::new(press, PointerKind::Press(PointerButton::Primary)))?;
    for step in 1..=10 {
        let p = Point::new(gx + 12.0 * step as f32, y)?;
        a.ui.pointer_input(PointerEvent::new(p, PointerKind::Move))?;
        a.d().settle()?;
    }
    let end = Point::new(gx + 120.0, y)?;
    a.ui.pointer_input(PointerEvent::new(end, PointerKind::Release(PointerButton::Primary)))?;
    a.settle()?;
    let [_, _, lw2, _] = a.inspected(&list)?.details.box_model.border_box.ok_or("no box")?;
    println!("sidebar width before={lw} after drag={lw2}");
    assert!(lw2 > lw + 60.0, "divider drag widened the sidebar");
    Ok(())
}

/// Exploration probe (ignored), restored verbatim from the first verifier
/// draft; its output is evidence/03-probe.log.
fn dump(ui: &Runtime, label: &str) {
    println!("==== {label}");
    for c in ui.inspect().components {
        let sem = c.details.semantics.as_ref();
        println!(
            "{:>4} parent={:?} key={:?} visible={} focused={} role={:?} name={:?} value={:?} items={:?}",
            c.id,
            c.parent,
            c.key,
            c.visible,
            c.focused,
            sem.map(|s| s.role),
            sem.and_then(|s| s.name.clone()),
            sem.and_then(|s| s.value.clone()),
            sem.map(|s| s.items.iter().map(|i| (i.name.clone(), i.selected)).collect::<Vec<_>>()),
        );
    }
}

#[test]
#[ignore]
fn probe_inspection() -> TestResult {
    let folder = PathBuf::from(format!("/work/acc-probe-{}", std::process::id()));
    let _ = fs::remove_dir_all(&folder);
    fs::create_dir_all(&folder)?;
    fs::write(folder.join("pear.txt"), "p")?;
    fs::write(folder.join("apple.txt"), "a")?;
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(1000.0, 700.0, 1.0)?)?;
    let root = build(&mut ui, folder.clone(), Editor::list(&folder)?)?;
    let mut d = Driver::new(&mut ui);
    d.settle()?;
    dump(d.runtime(), "start");
    let title = d.runtime().find_descendant(&root, "title/file")?.expect("title/file");
    d.click(&title)?;
    dump(d.runtime(), "menu open");
    d.chord(KeyChord::primary('n'), KeyCode::KeyN)?;
    dump(d.runtime(), "after ctrl+n");
    let _ = fs::remove_dir_all(&folder);
    Ok(())
}
