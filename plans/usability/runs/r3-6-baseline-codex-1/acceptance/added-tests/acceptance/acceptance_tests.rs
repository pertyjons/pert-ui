//! Independent acceptance tests for task 6 (notes editor), added by the
//! acceptance verifier in the acceptance copy only.
//!
//! They mount the application's own `build` with the folder listing from its
//! own `list_notes`, exactly as `main` does, then drive it only through
//! pert-ui's public input facilities (`Driver::click`, `Driver::type_text`,
//! `Driver::key`, `Driver::chord`, `Runtime::pointer_input`) and read results
//! only through `Runtime::inspect` / `Driver::semantics` and the file system.
//! No editor model function or application field is called or read.
use super::*;
use pert_ui::inspection::*;
use pert_ui::testing::*;
use std::collections::BTreeSet;

const OUT: &str = "/work/acceptance-out";

struct Acc {
    ui: Runtime,
    root: WidgetId,
    folder: PathBuf,
}

fn log(msg: impl AsRef<str>) {
    println!("[acceptance] {}", msg.as_ref());
}

impl Acc {
    /// Creates `/work/acceptance-out/<test>/notes` with `files` written in the
    /// given order (deliberately not alphabetical), then starts the app the
    /// way `main` does: `list_notes(&folder)` and `build`.
    fn start(test: &str, files: &[(&str, &str)]) -> Self {
        let folder = PathBuf::from(format!("{OUT}/{test}/notes"));
        let _ = fs::remove_dir_all(folder.parent().unwrap());
        fs::create_dir_all(&folder).unwrap();
        for (name, text) in files {
            fs::write(folder.join(name), text).unwrap();
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        log(format!("{test}: folder created in order {:?}", files.iter().map(|f| f.0).collect::<Vec<_>>()));
        let mut ui = Runtime::new();
        ui.set_viewport(Viewport::new(1000.0, 700.0, 1.0).unwrap()).unwrap();
        let names = list_notes(&folder).unwrap();
        let root = build(&mut ui, folder.clone(), names).unwrap();
        Driver::new(&mut ui).settle().unwrap();
        Self { ui, root, folder }
    }

    fn id(&self, key: &str) -> WidgetId {
        self.ui
            .find_descendant(&self.root, key)
            .unwrap()
            .unwrap_or_else(|| panic!("missing {key}"))
    }

    // ---------- inspection reads ----------

    fn snap(&self) -> InspectionSnapshot {
        self.ui.inspect()
    }
    /// The unique inspected component whose address ends with `key`.
    fn comp(&self, key: &str) -> ComponentInspection {
        let snap = self.snap();
        let found: Vec<_> = snap
            .components
            .into_iter()
            .filter(|c| c.path.as_ref().and_then(|p| p.last()).map(String::as_str) == Some(key))
            .collect();
        assert_eq!(found.len(), 1, "address ending {key:?} not unique");
        found.into_iter().next().unwrap()
    }
    fn props_text(c: &ComponentInspection, prop: &str) -> Option<String> {
        match c.details.properties.get(prop) {
            Some(InspectValue::Text(t)) => Some(t.clone()),
            _ => None,
        }
    }
    fn props_bool(c: &ComponentInspection, prop: &str) -> Option<bool> {
        match c.details.properties.get(prop) {
            Some(InspectValue::Bool(b)) => Some(*b),
            _ => None,
        }
    }
    fn props_num(c: &ComponentInspection, prop: &str) -> Option<f64> {
        match c.details.properties.get(prop) {
            Some(InspectValue::Number(n)) => Some(*n),
            _ => None,
        }
    }
    /// Listed note names, in on-screen order, read from inspection: the
    /// list's visible row components sorted by their laid-out y.
    fn listed(&self) -> Vec<String> {
        let snap = self.snap();
        let list = snap
            .components
            .iter()
            .find(|c| c.path.as_ref().and_then(|p| p.last()).map(String::as_str) == Some("notes"))
            .expect("list in inspection");
        assert_eq!(list.details.semantics.as_ref().unwrap().role, SemanticRole::List);
        let mut rows: Vec<_> = snap
            .components
            .iter()
            .filter(|c| c.parent == Some(list.id) && c.visible)
            .map(|c| {
                let y = c.details.box_model.border_box.expect("row laid out")[1];
                let name = c.details.semantics.as_ref().and_then(|s| s.name.clone()).unwrap();
                (y, name)
            })
            .collect();
        rows.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        let rows: Vec<String> = rows.into_iter().map(|r| r.1).collect();
        assert_eq!(Self::props_num(list, "rows"), Some(rows.len() as f64));
        rows
    }
    /// Tab titles in order, the selected tab's key, and the tab count, from
    /// the Tabs component's inspection.
    fn tabs(&self) -> (Vec<String>, Option<String>, usize) {
        let c = self.comp("tabs");
        let s = c.details.semantics.as_ref().unwrap();
        assert_eq!(s.role, SemanticRole::TabList);
        let titles: Vec<String> = s.items.iter().map(|i| i.name.clone().unwrap_or_default()).collect();
        let count = Self::props_num(&c, "tab_count").unwrap() as usize;
        assert_eq!(count, titles.len());
        (titles, Self::props_text(&c, "selected"), count)
    }
    fn selected_title(&self) -> Option<String> {
        let c = self.comp("tabs");
        let s = c.details.semantics.unwrap();
        s.items.iter().find(|i| i.selected).and_then(|i| i.name.clone())
    }
    fn status(&self) -> String {
        let c = self.comp("counts");
        c.details
            .semantics
            .as_ref()
            .and_then(|s| s.name.clone())
            .or_else(|| Self::props_text(&c, "text"))
            .unwrap_or_default()
    }
    /// Names of open dialogs (role Dialog/AlertDialog with `open` true).
    fn open_dialogs(&self) -> Vec<String> {
        self.snap()
            .components
            .iter()
            .filter(|c| {
                matches!(
                    c.details.semantics.as_ref().map(|s| s.role),
                    Some(SemanticRole::Dialog | SemanticRole::AlertDialog)
                ) && Self::props_bool(c, "open") == Some(true)
            })
            .map(|c| c.key.clone())
            .collect()
    }
    fn visible(&self, key: &str) -> bool {
        self.comp(key).visible
    }
    fn label_text(&self, key: &str) -> String {
        let c = self.comp(key);
        Self::props_text(&c, "text")
            .or_else(|| c.details.semantics.as_ref().and_then(|s| s.name.clone()))
            .unwrap_or_default()
    }
    /// Notification count and role, from the toast host's inspection.
    fn notifications(&self) -> f64 {
        let c = self.comp("toasts");
        let s = c.details.semantics.as_ref().unwrap();
        assert_eq!(s.role, SemanticRole::Notifications);
        assert!(c.visible);
        match s.value {
            Some(SemanticValue::Number { value, .. }) => {
                assert_eq!(Self::props_num(&c, "visible_count"), Some(value));
                value
            }
            ref other => panic!("unexpected toast value {other:?}"),
        }
    }
    fn editor(&self, note: &str) -> WidgetId {
        let tab = self.ui.find(&self.id("tabs"), &[note]).unwrap().expect("tab");
        self.ui.find_descendant(&tab, "editor").unwrap().expect("editor")
    }
    fn editor_text(&mut self, note: &str) -> String {
        let id = self.editor(note);
        match Driver::new(&mut self.ui).semantics(&id).unwrap().value {
            Some(SemanticValue::Text(t)) => t,
            other => panic!("editor value {other:?}"),
        }
    }
    fn disk(&self, name: &str) -> String {
        fs::read_to_string(self.folder.join(name)).unwrap()
    }
    fn dir_names(&self) -> BTreeSet<String> {
        fs::read_dir(&self.folder)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect()
    }

    // ---------- input ----------

    fn settle(&mut self) {
        Driver::new(&mut self.ui).settle().unwrap();
    }
    /// Pointer click on a row of the sidebar.
    fn click_row(&mut self, name: &str) {
        let row = self.ui.find(&self.id("notes"), &[name]).unwrap().expect("row");
        // Separate clicks, never a double click.
        Driver::new(&mut self.ui).advance(std::time::Duration::from_secs(1)).unwrap();
        Driver::new(&mut self.ui).click(&row).unwrap();
        self.settle();
    }
    fn click(&mut self, key: &str) {
        let id = self.id(key);
        Driver::new(&mut self.ui).advance(std::time::Duration::from_secs(1)).unwrap();
        Driver::new(&mut self.ui).click(&id).unwrap();
        self.settle();
    }
    fn key(&mut self, code: KeyCode) {
        Driver::new(&mut self.ui).key(code).unwrap();
        self.settle();
    }
    fn chord(&mut self, letter: char, code: KeyCode) {
        Driver::new(&mut self.ui).chord(KeyChord::primary(letter), code).unwrap();
        self.settle();
    }
    fn type_text(&mut self, text: &str) {
        Driver::new(&mut self.ui).type_text(text).unwrap();
        self.settle();
    }
    /// Pointer click into a tab's editor, then End to reach the end of the
    /// (single) line, as a user would before typing.
    fn focus_editor(&mut self, note: &str) {
        let id = self.editor(note);
        Driver::new(&mut self.ui).advance(std::time::Duration::from_secs(1)).unwrap();
        Driver::new(&mut self.ui).click(&id).unwrap();
        self.settle();
        assert_eq!(self.ui.focused(), Some(id), "editor focused by click");
    }
    fn pointer(&mut self, x: f32, y: f32, kind: PointerKind) {
        self.ui
            .pointer_input(PointerEvent::new(Point::new(x, y).unwrap(), kind))
            .unwrap();
        self.settle();
    }
    /// File → `label` by pointer: a click on the File title of the menu bar,
    /// then a click on the menu row. The row is located by moving the pointer
    /// down the open menu until inspection reports `label` as the hovered
    /// (active) row, then pressing and releasing there.
    fn menu_click(&mut self, label: &str) {
        self.click("title/file");
        let items = self
            .snap()
            .components
            .into_iter()
            .find(|c| {
                c.key == "items"
                    && c.visible
                    && c.details.semantics.as_ref().map(|s| s.role) == Some(SemanticRole::Menu)
            })
            .expect("File menu open after clicking its title");
        let [x, y, w, h] = items.details.box_model.border_box.expect("menu laid out");
        let cx = x + w / 2.0;
        let mut at = None;
        let mut yy = y + 1.0;
        while yy < y + h {
            self.pointer(cx, yy, PointerKind::Move);
            let now = self
                .snap()
                .components
                .into_iter()
                .find(|c| c.id == items.id)
                .unwrap();
            if Self::props_text(&now, "active").as_deref() == Some(label) {
                at = Some(yy + 4.0);
                break;
            }
            yy += 2.0;
        }
        let at = at.unwrap_or_else(|| panic!("menu row {label:?} not found under the pointer"));
        self.pointer(cx, at, PointerKind::Move);
        self.pointer(cx, at, PointerKind::Press(PointerButton::Primary));
        self.pointer(cx, at, PointerKind::Release(PointerButton::Primary));
        self.settle();
        let still_open = self
            .snap()
            .components
            .iter()
            .any(|c| c.key == "items" && c.visible && c.id == items.id);
        assert!(!still_open, "menu closes after choosing {label:?}");
        log(format!("menu: clicked File → {label} at y={at}"));
    }
    fn capture(&mut self, file: &str) {
        self.ui.set_capture_support(true);
        let pending = self
            .ui
            .request_capture(CaptureRequest::new(CaptureTarget::window(&self.ui.primary_window())));
        self.settle();
        CpuRenderer::new().serve_captures(&mut self.ui);
        let capture = pending.wait().unwrap().value.unwrap();
        let path = format!("{OUT}/{file}");
        fs::write(&path, capture.image.encode_png()).unwrap();
        log(format!("capture written: {path}"));
    }
}

// ---------------------------------------------------------------- item 1

#[test]
fn item1_listing_open_and_reopen() {
    let mut a = Acc::start(
        "item1",
        &[
            ("zulu.txt", "last"),
            ("readme.md", "not a note"),
            ("mike.txt", "middle note"),
            ("archive.txt.bak", "not a note either"),
            ("alpha.txt", "first note"),
        ],
    );
    let listed = a.listed();
    log(format!("item1: listed {listed:?}"));
    assert_eq!(listed, ["alpha.txt", "mike.txt", "zulu.txt"]);
    assert_eq!(a.tabs().2, 0);
    assert_eq!(a.status(), "");

    // Keyboard only, from a fresh start: Tab focus traversal reaches the
    // list; Down, Enter opens a note.
    let list = a.id("notes");
    let mut tabs_pressed = 0;
    while a.ui.focused() != Some(list.clone()) && tabs_pressed < 8 {
        a.key(KeyCode::Tab);
        tabs_pressed += 1;
    }
    log(format!("item1: list focused after {tabs_pressed} Tab presses: {}", a.ui.focused() == Some(list.clone())));
    assert_eq!(a.ui.focused(), Some(list.clone()), "list reachable by Tab");
    a.key(KeyCode::ArrowDown);
    a.key(KeyCode::Enter);
    let (titles, sel, n) = a.tabs();
    log(format!("item1: keyboard-only open: {titles:?} selected={sel:?}"));
    assert_eq!(n, 1);
    // Close it again (unmodified, Ctrl+W) to start the pointer part empty.
    a.chord('w', KeyCode::KeyW);
    assert_eq!(a.tabs().2, 0);

    // Pointer opens zulu.
    a.click_row("zulu.txt");
    let (titles, sel, n) = a.tabs();
    log(format!("item1: after pointer open zulu: {titles:?} selected={sel:?}"));
    assert_eq!((n, sel.as_deref()), (1, Some("zulu.txt")));
    assert_eq!(a.editor_text("zulu.txt"), "last");

    // Keyboard: focus is in the list after the pointer click? Use the list's
    // own keys: Home then Down then Enter opens the second row (mike).
    let list = a.id("notes");
    if a.ui.focused() != Some(list.clone()) {
        log("item1: list not focused by row click; focusing it with Runtime::focus");
        a.ui.focus(&list).unwrap();
        a.settle();
    }
    a.key(KeyCode::Home);
    a.key(KeyCode::ArrowDown);
    a.key(KeyCode::Enter);
    let (titles, sel, n) = a.tabs();
    log(format!("item1: after keyboard open: {titles:?} selected={sel:?}"));
    assert_eq!((n, sel.as_deref()), (2, Some("mike.txt")));
    assert_eq!(a.editor_text("mike.txt"), "middle note");
    assert!(a.status().starts_with("mike.txt:"));

    // Reopen zulu by pointer: no new tab, its tab becomes active.
    a.click_row("zulu.txt");
    let (titles, sel, n) = a.tabs();
    log(format!("item1: after reopening zulu by pointer: {titles:?} selected={sel:?}"));
    assert_eq!((n, sel.as_deref()), (2, Some("zulu.txt")));
    assert_eq!(a.selected_title().as_deref(), Some("zulu.txt"));
    assert_eq!(a.status(), "zulu.txt: 1 lines, 1 words");

    // Reopen mike by keyboard (Down from current list position, Enter).
    a.ui.focus(&list).unwrap();
    a.settle();
    a.key(KeyCode::Home);
    a.key(KeyCode::ArrowDown);
    a.key(KeyCode::Enter);
    let (titles, sel, n) = a.tabs();
    log(format!("item1: after reopening mike by keyboard: {titles:?} selected={sel:?}"));
    assert_eq!((n, sel.as_deref()), (2, Some("mike.txt")));
    assert_eq!(titles, ["zulu.txt", "mike.txt"]);
}

// ---------------------------------------------------------------- item 2

#[test]
fn item2_typing_marker_status_saves_and_failed_save() {
    let mut a = Acc::start("item2", &[("zeta.txt", "zz"), ("other.md", "x"), ("beta.txt", "one")]);
    assert_eq!(a.listed(), ["beta.txt", "zeta.txt"]);
    a.click_row("beta.txt");
    assert_eq!(a.status(), "beta.txt: 1 lines, 1 words");
    assert_eq!(a.tabs().0, ["beta.txt"]);

    // Simulated typing with line breaks (Enter key presses).
    a.focus_editor("beta.txt");
    a.key(KeyCode::End);
    a.type_text(" two");
    log(format!("item2: after typing ' two': title={:?} status={:?}", a.tabs().0, a.status()));
    assert_eq!(a.tabs().0, ["beta.txt *"]);
    assert_eq!(a.status(), "beta.txt: 1 lines, 2 words");
    a.key(KeyCode::Enter);
    assert_eq!(a.status(), "beta.txt: 2 lines, 2 words");
    a.type_text("three   four");
    a.key(KeyCode::Enter);
    let typed = a.editor_text("beta.txt");
    log(format!("item2: editor text {typed:?}; status={:?}", a.status()));
    assert_eq!(typed, "one two\nthree   four\n");
    assert_eq!(a.status(), "beta.txt: 3 lines, 4 words");
    assert_eq!(a.tabs().0, ["beta.txt *"]);
    assert_eq!(a.disk("beta.txt"), "one", "not written before saving");

    // Ctrl+S through the library's command key path.
    a.chord('s', KeyCode::KeyS);
    log(format!("item2: after Ctrl+S disk={:?} title={:?}", a.disk("beta.txt"), a.tabs().0));
    assert_eq!(a.disk("beta.txt"), "one two\nthree   four\n");
    assert_eq!(a.tabs().0, ["beta.txt"]);
    assert_eq!(a.notifications(), 0.0);

    // More typing, then File → Save by pointer clicks in the menu bar.
    a.focus_editor("beta.txt");
    a.type_text("five");
    assert_eq!(a.tabs().0, ["beta.txt *"]);
    let expected = a.editor_text("beta.txt");
    assert!(expected.contains("five"));
    a.menu_click("Save");
    log(format!("item2: after File→Save disk={:?} title={:?}", a.disk("beta.txt"), a.tabs().0));
    assert_eq!(a.disk("beta.txt"), expected);
    assert_eq!(a.tabs().0, ["beta.txt"]);
    assert_eq!(a.status(), format!("beta.txt: {} lines, 5 words", expected.matches('\n').count() + 1));

    // A second tab: Save writes only the active tab.
    a.click_row("zeta.txt");
    a.focus_editor("zeta.txt");
    a.key(KeyCode::End);
    a.type_text("\nline");
    assert_eq!(a.tabs().0, ["beta.txt", "zeta.txt *"]);
    assert_eq!(a.status(), "zeta.txt: 2 lines, 2 words");
    a.capture("item2-two-tabs-one-modified.png");

    // Failed save: remove the notes folder, then Ctrl+S, then File → Save.
    fs::remove_dir_all(&a.folder).unwrap();
    let before = a.notifications();
    a.chord('s', KeyCode::KeyS);
    let after = a.notifications();
    log(format!("item2: failed Ctrl+S notifications {before} -> {after}; titles={:?}", a.tabs().0));
    assert_eq!(after, before + 1.0);
    assert_eq!(a.tabs().0, ["beta.txt", "zeta.txt *"], "marker kept after failed save");
    assert!(!a.folder.exists());
    a.capture("item2-failed-save-notification.png");
    a.menu_click("Save");
    let after_menu = a.notifications();
    log(format!("item2: failed File→Save notifications {after} -> {after_menu}; titles={:?}", a.tabs().0));
    assert!(after_menu > after);
    assert_eq!(a.tabs().0, ["beta.txt", "zeta.txt *"]);
    assert_eq!(a.editor_text("zeta.txt"), "zz\nline");
}

// ---------------------------------------------------------------- item 3

#[test]
fn item3_new_note_dialog_refusals_and_creation() {
    let mut a = Acc::start("item3", &[("mike.txt", ""), ("notes.md", ""), ("alpha.txt", "a")]);
    let initial = a.dir_names();
    assert_eq!(a.open_dialogs(), Vec::<String>::new());

    // Open via File → New note (pointer).
    a.menu_click("New note");
    assert_eq!(a.open_dialogs(), ["new-dialog"]);
    assert!(a.visible("new-form"));

    let refusals: &[(&str, &str)] = &[
        ("", "1 to 64"),
        ("a/b", "/"),
        ("back\\slash", "\\"),
        (".hidden", "."),
        ("alpha", "already exists"),
        ("alpha.txt", "already exists"),
    ];
    for (name, expect) in refusals {
        let field = a.id("name");
        Driver::new(&mut a.ui).replace_text(&field, name).unwrap();
        a.settle();
        a.click("create");
        let shown = a.label_text("name-error");
        log(format!("item3: name {name:?} -> dialogs={:?} error visible={} text={shown:?}", a.open_dialogs(), a.visible("name-error")));
        assert_eq!(a.open_dialogs(), ["new-dialog"], "dialog stays open for {name:?}");
        assert!(a.visible("name-error"));
        assert!(shown.contains(expect), "explanation for {name:?}: {shown:?}");
        assert_eq!(a.dir_names(), initial, "no file created for {name:?}");
        assert_eq!(a.tabs().2, 0);
    }
    // 65 characters refused; 64 accepted later via the shortcut path.
    let field = a.id("name");
    Driver::new(&mut a.ui).replace_text(&field, &"x".repeat(65)).unwrap();
    a.settle();
    a.click("create");
    assert!(a.label_text("name-error").contains("1 to 64"));
    assert_eq!(a.dir_names(), initial);

    // A valid name typed into the field: creates, lists, opens.
    let field = a.id("name");
    Driver::new(&mut a.ui).advance(std::time::Duration::from_secs(1)).unwrap();
    Driver::new(&mut a.ui).click(&field).unwrap();
    Driver::new(&mut a.ui).replace_text(&field, "").unwrap();
    a.type_text("beta");
    a.click("create");
    log(format!("item3: after 'beta': dialogs={:?} dir={:?} listed={:?} tabs={:?}", a.open_dialogs(), a.dir_names(), a.listed(), a.tabs()));
    assert_eq!(a.open_dialogs(), Vec::<String>::new());
    assert!(a.folder.join("beta.txt").is_file());
    assert_eq!(a.disk("beta.txt"), "");
    assert_eq!(a.listed(), ["alpha.txt", "beta.txt", "mike.txt"]);
    let (titles, sel, _) = a.tabs();
    assert_eq!((titles, sel.as_deref()), (vec!["beta.txt".to_string()], Some("beta.txt")));
    assert_eq!(a.status(), "beta.txt: 1 lines, 0 words");

    // Ctrl+N opens the same dialog (fresh, no stale error); a name with .txt
    // is kept as is.
    a.chord('n', KeyCode::KeyN);
    assert_eq!(a.open_dialogs(), ["new-dialog"]);
    assert_eq!(a.label_text("name-error"), "");
    let field = a.id("name");
    Driver::new(&mut a.ui).replace_text(&field, "beta").unwrap();
    a.click("create");
    assert!(a.label_text("name-error").contains("already exists"));
    let field = a.id("name");
    Driver::new(&mut a.ui).replace_text(&field, "gamma.txt").unwrap();
    a.click("create");
    assert_eq!(a.open_dialogs(), Vec::<String>::new());
    assert!(a.folder.join("gamma.txt").is_file());
    assert!(!a.folder.join("gamma.txt.txt").exists());
    assert_eq!(a.listed(), ["alpha.txt", "beta.txt", "gamma.txt", "mike.txt"]);
    assert_eq!(a.tabs().0, ["beta.txt", "gamma.txt"]);
    assert_eq!(a.tabs().1.as_deref(), Some("gamma.txt"));

    // Ctrl+N then Cancel creates nothing.
    let before = a.dir_names();
    a.chord('n', KeyCode::KeyN);
    let field = a.id("name");
    Driver::new(&mut a.ui).replace_text(&field, "never").unwrap();
    a.click("cancel-new");
    assert_eq!(a.open_dialogs(), Vec::<String>::new());
    assert_eq!(a.dir_names(), before);
    log(format!("item3: final dir {:?}", a.dir_names()));
}

// ---------------------------------------------------------------- item 4

#[test]
fn item4_close_choices_and_unmodified_close() {
    let mut a = Acc::start(
        "item4",
        &[("c.txt", "cee"), ("ignored.log", "x"), ("b.txt", "bee"), ("a.txt", "ay"), ("d.txt", "dee")],
    );
    for n in ["a.txt", "b.txt", "c.txt", "d.txt"] {
        a.click_row(n);
    }
    assert_eq!(a.tabs().2, 4);

    // Unmodified tab: Ctrl+W closes at once, no dialog in the snapshot.
    assert_eq!(a.tabs().1.as_deref(), Some("d.txt"));
    a.chord('w', KeyCode::KeyW);
    log(format!("item4: unmodified Ctrl+W -> tabs={:?} dialogs={:?}", a.tabs(), a.open_dialogs()));
    assert_eq!(a.tabs().0, ["a.txt", "b.txt", "c.txt"]);
    assert_eq!(a.open_dialogs(), Vec::<String>::new());
    assert!(!a.visible("close-form"));
    assert_eq!(a.disk("d.txt"), "dee");

    // Modified c: Cancel keeps tab and text.
    a.click_row("c.txt");
    a.focus_editor("c.txt");
    a.key(KeyCode::End);
    a.type_text(" changed");
    a.key(KeyCode::Enter);
    a.type_text("x");
    assert_eq!(a.tabs().0, ["a.txt", "b.txt", "c.txt *"]);
    a.chord('w', KeyCode::KeyW);
    log(format!("item4: modified Ctrl+W -> dialogs={:?} close-form visible={}", a.open_dialogs(), a.visible("close-form")));
    assert_eq!(a.open_dialogs(), ["close-dialog"]);
    assert!(a.visible("close-form"));
    a.click("cancel-close");
    log(format!("item4: Cancel -> tabs={:?} dialogs={:?}", a.tabs(), a.open_dialogs()));
    assert_eq!(a.open_dialogs(), Vec::<String>::new());
    assert_eq!(a.tabs().0, ["a.txt", "b.txt", "c.txt *"]);
    assert_eq!(a.tabs().1.as_deref(), Some("c.txt"));
    assert_eq!(a.editor_text("c.txt"), "cee changed\nx");
    assert_eq!(a.disk("c.txt"), "cee");

    // Discard via File → Close tab (pointer): closes, file unchanged.
    a.menu_click("Close tab");
    assert_eq!(a.open_dialogs(), ["close-dialog"]);
    a.click("discard");
    log(format!("item4: Discard -> tabs={:?} dialogs={:?} disk={:?}", a.tabs(), a.open_dialogs(), a.disk("c.txt")));
    assert_eq!(a.open_dialogs(), Vec::<String>::new());
    assert_eq!(a.tabs().0, ["a.txt", "b.txt"]);
    assert_eq!(a.disk("c.txt"), "cee");
    // Reopening shows the file's text, not the discarded edit.
    a.click_row("c.txt");
    assert_eq!(a.editor_text("c.txt"), "cee");
    assert_eq!(a.tabs().0, ["a.txt", "b.txt", "c.txt"]);

    // Save in the dialog: writes, then the tab is gone.
    a.click_row("b.txt");
    a.focus_editor("b.txt");
    a.key(KeyCode::End);
    a.key(KeyCode::Enter);
    a.type_text("saved on close");
    assert_eq!(a.tabs().0, ["a.txt", "b.txt *", "c.txt"]);
    a.chord('w', KeyCode::KeyW);
    assert_eq!(a.open_dialogs(), ["close-dialog"]);
    a.click("save-close");
    log(format!("item4: Save -> tabs={:?} dialogs={:?} disk={:?}", a.tabs(), a.open_dialogs(), a.disk("b.txt")));
    assert_eq!(a.open_dialogs(), Vec::<String>::new());
    assert_eq!(a.tabs().0, ["a.txt", "c.txt"]);
    assert_eq!(a.disk("b.txt"), "bee\nsaved on close");

    // Unmodified tab via File → Close tab (pointer): no dialog.
    a.click_row("c.txt");
    a.menu_click("Close tab");
    log(format!("item4: unmodified File→Close tab -> tabs={:?} dialogs={:?}", a.tabs(), a.open_dialogs()));
    assert_eq!(a.open_dialogs(), Vec::<String>::new());
    assert_eq!(a.tabs().0, ["a.txt"]);

    // Save-and-close that fails keeps the tab open and modified.
    a.focus_editor("a.txt");
    a.type_text("!");
    assert_eq!(a.tabs().0, ["a.txt *"]);
    fs::remove_dir_all(&a.folder).unwrap();
    let before = a.notifications();
    a.chord('w', KeyCode::KeyW);
    a.click("save-close");
    log(format!("item4: failing Save-and-close -> tabs={:?} dialogs={:?} notifications {before}->{}", a.tabs(), a.open_dialogs(), a.notifications()));
    assert_eq!(a.tabs().0, ["a.txt *"]);
    assert_eq!(a.notifications(), before + 1.0);
    // Dialog remains to choose again; Discard then closes and status empties.
    assert_eq!(a.open_dialogs(), ["close-dialog"]);
    a.click("discard");
    assert_eq!(a.tabs().2, 0);
    assert_eq!(a.status(), "");
    assert_eq!(a.open_dialogs(), Vec::<String>::new());
}

// ---------------------------------------------------------------- item 5

#[test]
fn item5_cpu_capture_two_tabs_one_modified() {
    let mut a = Acc::start("item5", &[("shopping.txt", "milk eggs"), ("ideas.txt", "Write a notes editor")]);
    a.click_row("ideas.txt");
    a.click_row("shopping.txt");
    a.focus_editor("shopping.txt");
    a.key(KeyCode::End);
    a.key(KeyCode::Enter);
    a.type_text("bread");
    let (titles, sel, _) = a.tabs();
    log(format!("item5: tabs {titles:?} selected={sel:?} status={:?}", a.status()));
    assert_eq!(titles, ["ideas.txt", "shopping.txt *"]);
    a.capture("item5-two-tabs-one-modified.png");
    let png = fs::read(format!("{OUT}/item5-two-tabs-one-modified.png")).unwrap();
    assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n");
}
