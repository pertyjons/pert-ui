//! Independent acceptance checks (verifier-added, round 3 task 6).
//!
//! Driven only through the library's public facilities: `HeadlessApp`,
//! `testing::Driver` input (pointer, keys, chords, typed text), widget
//! semantics, the `Runtime::inspect` snapshot and the CPU renderer's capture
//! service. The application's own model functions are never called; the
//! `Notes` handle is used only for its mounted root `WidgetId`, as a starting
//! point for addressed lookups. Disk contents are read with `std::fs`.

use notes_editor::Notes;
use pert_ui::prelude::*;
use pert_ui::testing::*;
use pert_ui::{PointerButton, PointerEvent, PointerKind, SemanticRole};
use std::path::{Path, PathBuf};
use std::time::Duration;

type R = Result<(), Box<dyn std::error::Error>>;

const MAIN: [&str; 4] = ["new-dialog", "close-dialog", "menu", "main"];

/// The verifier's own notes folder. Files are created out of alphabetical
/// order, with non-`.txt` files and a directory whose name ends in `.txt`.
fn own_folder(name: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("acc-{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    let _ = std::fs::remove_dir_all(dir.with_extension("moved"));
    std::fs::create_dir_all(&dir).unwrap();
    for (file, text) in [
        ("zulu.txt", "last one"),
        ("todo.md", "markdown, not a note"),
        ("mike.txt", "middle\nof the\nlist"),
        ("data.json", "{}"),
        ("alpha.txt", "first note"),
        ("kilo.txt", ""),
        ("notes.txt.bak", "backup, not a note"),
    ] {
        std::fs::write(dir.join(file), text).unwrap();
        std::thread::sleep(Duration::from_millis(5));
    }
    std::fs::create_dir(dir.join("folder.txt")).unwrap();
    dir
}

fn p<'a>(tail: &[&'a str]) -> Vec<&'a str> {
    MAIN.iter().copied().chain(tail.iter().copied()).collect()
}

fn files(dir: &Path) -> Vec<String> {
    let mut v: Vec<String> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .collect();
    v.sort();
    v
}

fn read(dir: &Path, name: &str) -> String {
    std::fs::read_to_string(dir.join(name)).unwrap()
}

fn list_id(d: &Driver<'_>, n: &Notes) -> Result<WidgetId, TestError> {
    d.find(&n.root, &p(&["split", "notes"]))
}

/// Listed names through the list's addressed rows and their semantics.
fn listed(d: &Driver<'_>, n: &Notes) -> Result<Vec<String>, TestError> {
    let list = list_id(d, n)?;
    d.address_children(&list)?
        .iter()
        .map(|row| Ok(d.semantics(&row.id)?.name.unwrap_or_default()))
        .collect()
}

/// Tab titles and the selected title, from the TabList's semantic items.
fn tabs(d: &Driver<'_>, n: &Notes) -> Result<(Vec<String>, Option<String>), TestError> {
    let id = d.find(&n.root, &p(&["split", "tabs"]))?;
    let s = d.semantics(&id)?;
    assert_eq!(s.role, SemanticRole::TabList);
    let titles = s.items.iter().map(|i| i.name.clone().unwrap_or_default()).collect();
    let sel = s.items.iter().find(|i| i.selected).and_then(|i| i.name.clone());
    Ok((titles, sel))
}

fn editor(d: &Driver<'_>, n: &Notes, name: &str) -> Result<WidgetId, TestError> {
    let tabs = d.find(&n.root, &p(&["split", "tabs"]))?;
    d.find(&tabs, &[name, "editor"])
}

fn editor_text(d: &Driver<'_>, n: &Notes, name: &str) -> Result<String, Box<dyn std::error::Error>> {
    let id = editor(d, n, name)?;
    let s = d.semantics(&id)?;
    assert_eq!(s.role, SemanticRole::MultilineTextInput, "a library multi-line editor");
    match s.value {
        Some(SemanticValue::Text(t)) => Ok(t),
        other => Err(format!("editor value {other:?}").into()),
    }
}

/// The status line's text, read from the snapshot: every visible Label under
/// the "status" address.
fn status(d: &Driver<'_>, n: &Notes) -> Result<String, TestError> {
    let id = d.find(&n.root, &p(&["status"]))?;
    let shown = d.address_children(&id)?;
    Ok(match shown.first() {
        Some(label) => d.semantics(&label.id)?.name.unwrap_or_default(),
        None => String::new(),
    })
}

/// Visible snapshot components with `role`, with their names.
fn visible_roles(d: &Driver<'_>, role: SemanticRole) -> Vec<(String, Option<String>, usize)> {
    d.runtime()
        .inspect()
        .components
        .into_iter()
        .filter(|c| c.visible)
        .filter_map(|c| {
            let s = c.details.semantics?;
            (s.role == role).then(|| (c.key, s.name, s.items.len()))
        })
        .collect()
}

/// Open dialogs in the snapshot: Dialog or AlertDialog components whose
/// semantics report them expanded (the library's Dialog reports `expanded`
/// and an `open` property for its shown surface).
fn open_dialogs(d: &Driver<'_>) -> Vec<String> {
    d.runtime()
        .inspect()
        .components
        .into_iter()
        .filter(|c| c.visible)
        .filter_map(|c| {
            let s = c.details.semantics?;
            let open = c.details.properties.get("open") == Some(&pert_ui::InspectValue::Bool(true));
            (matches!(s.role, SemanticRole::Dialog | SemanticRole::AlertDialog) && s.expanded == Some(true))
                .then(|| { assert!(open, "open property agrees"); c.key })
        })
        .collect()
}

/// Notification count and names from the snapshot's Notifications component.
fn notifications(d: &Driver<'_>) -> (usize, Vec<String>) {
    let snap = d.runtime().inspect();
    let mut count = 0;
    let mut names = Vec::new();
    for c in snap.components {
        if let Some(s) = c.details.semantics {
            if s.role == SemanticRole::Notifications {
                count += s.items.len();
                names.extend(s.items.into_iter().filter_map(|i| i.name));
            }
        }
    }
    (count, names)
}

/// Serves a CPU-renderer capture of the window and writes it as PNG under
/// `acceptance-captures/`.
fn capture(d: &mut Driver<'_>, name: &str) -> Result<PathBuf, Box<dyn std::error::Error>> {
    d.runtime().set_capture_support(true);
    let pending = d.runtime().request_capture(CaptureRequest::new(CaptureTarget::window(
        &d.runtime().primary_window(),
    )));
    d.settle()?;
    CpuRenderer::new().serve_captures(d.runtime_mut());
    let image = pending.try_reply()?.value?.image;
    let out = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("acceptance-captures");
    std::fs::create_dir_all(&out)?;
    let file = out.join(name);
    std::fs::write(&file, image.encode_png())?;
    assert!(std::fs::read(&file)?.starts_with(b"\x89PNG"));
    eprintln!("capture {}x{} -> {}", image.width(), image.height(), file.display());
    Ok(file)
}

fn click_note(d: &mut Driver<'_>, n: &Notes, name: &str) -> R {
    let list = list_id(d, n)?;
    let row = d.find(&list, &[name])?;
    d.click(&row)?;
    d.advance(ClickCounter::DEFAULT_INTERVAL + Duration::from_millis(1))?;
    Ok(())
}

fn pointer(d: &mut Driver<'_>, x: f32, y: f32, kind: PointerKind) -> R {
    d.runtime_mut().pointer_input(PointerEvent::new(Point::new(x, y)?, kind))?;
    Ok(())
}

/// Opens File by clicking its title, then clicks row `index` of the open menu
/// with the pointer at the row's center. Menu rows report no per-item bounds,
/// so the row's rectangle is derived from the open menu list's bounds divided
/// into its equal-height rows (no separators in this menu).
fn menu_click(d: &mut Driver<'_>, n: &Notes, label: &str) -> R {
    let bar = d.find(&n.root, &["new-dialog", "close-dialog", "menu"])?;
    assert_eq!(d.semantics(&bar)?.role, SemanticRole::MenuBar);
    let file = d.runtime().find_descendant(&bar, "title/file")?.ok_or("File title")?;
    d.click(&file)?;
    assert_eq!(d.semantics(&file)?.expanded, Some(true), "File menu opened by click");
    let items = d.runtime().find_descendant(&file, "items")?.ok_or("File rows")?;
    let names: Vec<String> =
        d.semantics(&items)?.items.into_iter().map(|i| i.name.unwrap_or_default()).collect();
    assert_eq!(names, ["New note", "Save", "Close tab"]);
    let index = names.iter().position(|x| x == label).ok_or("no such row")?;
    let [x, y, w, h] = d.runtime().bounds(&items)?.ok_or("rows laid out")?.values();
    let row = h / names.len() as f32;
    let (cx, cy) = (x + w / 2.0, y + row * index as f32 + row / 2.0);
    pointer(d, cx, cy, PointerKind::Move)?;
    pointer(d, cx, cy, PointerKind::Press(PointerButton::Primary))?;
    pointer(d, cx, cy, PointerKind::Release(PointerButton::Primary))?;
    d.settle()?;
    assert_ne!(d.semantics(&file)?.expanded, Some(true), "menu closed after choosing");
    d.advance(ClickCounter::DEFAULT_INTERVAL + Duration::from_millis(1))?;
    Ok(())
}

#[test]
fn acc1_own_folder_listing_open_reopen() -> R {
    let dir = own_folder("list");
    let mut app = notes_editor::app(dir.clone()).headless()?;
    let (ui, n) = app.runtime_and_state_mut();
    let n = n.clone();
    let mut d = Driver::new(ui);
    d.settle()?;

    let list = list_id(&d, &n)?;
    eprintln!("list role: {:?}", d.semantics(&list)?.role);
    assert_eq!(listed(&d, &n)?, ["alpha.txt", "kilo.txt", "mike.txt", "zulu.txt"]);
    // The same names, in order, as visible Buttons in the snapshot.
    let shown: Vec<String> = d
        .runtime()
        .inspect()
        .components
        .into_iter()
        .filter(|c| c.visible && c.path.as_ref().is_some_and(|p| p.len() > 1 && p[p.len() - 2] == "notes"))
        .filter_map(|c| c.details.semantics.and_then(|s| s.name))
        .collect();
    assert_eq!(shown, ["alpha.txt", "kilo.txt", "mike.txt", "zulu.txt"]);
    assert_eq!(tabs(&d, &n)?.0.len(), 0);
    assert_eq!(status(&d, &n)?, "", "empty status line with no tab");

    // Pointer opens zulu, keyboard (arrows + Enter) opens mike.
    click_note(&mut d, &n, "zulu.txt")?;
    assert_eq!(tabs(&d, &n)?, (vec!["zulu.txt".into()], Some("zulu.txt".into())));
    d.runtime_mut().focus(&list)?;
    d.key(KeyCode::Home)?;
    d.key(KeyCode::ArrowDown)?;
    d.key(KeyCode::ArrowDown)?;
    let before_enter = tabs(&d, &n)?.0.len();
    d.key(KeyCode::Enter)?;
    let (t, sel) = tabs(&d, &n)?;
    assert_eq!(before_enter, 1, "arrows alone open nothing");
    assert_eq!(t, ["zulu.txt", "mike.txt"]);
    assert_eq!(sel.as_deref(), Some("mike.txt"));
    assert_eq!(editor_text(&d, &n, "mike.txt")?, "middle\nof the\nlist");
    assert_eq!(status(&d, &n)?, "mike.txt: 3 lines, 4 words");

    // Reopening zulu from the list: tab count unchanged, its tab active.
    click_note(&mut d, &n, "zulu.txt")?;
    let (t, sel) = tabs(&d, &n)?;
    assert_eq!(t, ["zulu.txt", "mike.txt"]);
    assert_eq!(sel.as_deref(), Some("zulu.txt"));
    assert_eq!(status(&d, &n)?, "zulu.txt: 1 lines, 2 words");
    let views = visible_roles(&d, SemanticRole::MultilineTextInput);
    assert_eq!(views.len(), 1, "only the active tab's editor is visible: {views:?}");
    // Reopening mike with the keyboard too.
    d.runtime_mut().focus(&list)?;
    d.key(KeyCode::Home)?;
    d.key(KeyCode::ArrowDown)?;
    d.key(KeyCode::ArrowDown)?;
    d.key(KeyCode::Enter)?;
    let (t, sel) = tabs(&d, &n)?;
    assert_eq!(t.len(), 2);
    assert_eq!(sel.as_deref(), Some("mike.txt"));
    Ok(())
}

#[test]
fn acc1_divider_drag_resizes() -> R {
    let dir = own_folder("drag");
    let mut app = notes_editor::app(dir).headless()?;
    let (ui, n) = app.runtime_and_state_mut();
    let n = n.clone();
    let mut d = Driver::new(ui);
    d.settle()?;
    let side = d.find(&n.root, &p(&["split", "notes"]))?;
    let width = |d: &Driver<'_>| -> Result<f32, Box<dyn std::error::Error>> {
        Ok(d.runtime().bounds(&side)?.ok_or("laid out")?.values()[2])
    };
    let splitters = visible_roles(&d, SemanticRole::Splitter);
    eprintln!("splitters: {splitters:?}");
    let before = width(&d)?;
    let [x, y, w, h] = d.runtime().bounds(&side)?.ok_or("laid out")?.values();
    let tabs_id = d.find(&n.root, &p(&["split", "tabs"]))?;
    let tx = d.runtime().bounds(&tabs_id)?.ok_or("laid out")?.values()[0];
    // The divider lies between the sidebar's right edge and the tabs' left edge.
    let gx = (x + w + tx) / 2.0;
    let gy = y + h / 2.0;
    eprintln!("sidebar {x},{w}; tabs at {tx}; pressing divider at {gx},{gy}");
    pointer(&mut d, gx, gy, PointerKind::Move)?;
    pointer(&mut d, gx, gy, PointerKind::Press(PointerButton::Primary))?;
    for step in 1..=5 {
        pointer(&mut d, gx + 12.0 * step as f32, gy, PointerKind::Move)?;
    }
    pointer(&mut d, gx + 60.0, gy, PointerKind::Release(PointerButton::Primary))?;
    d.settle()?;
    let after = width(&d)?;
    eprintln!("sidebar width {before} -> {after}");
    assert!((after - before - 60.0).abs() < 1.0, "drag by 60 resizes: {before} -> {after}");
    Ok(())
}

#[test]
fn acc2_typing_marker_status_saves_and_failure() -> R {
    let dir = own_folder("save");
    let mut app = notes_editor::app(dir.clone()).headless()?;
    let (ui, n) = app.runtime_and_state_mut();
    let n = n.clone();
    let mut d = Driver::new(ui);
    d.settle()?;
    click_note(&mut d, &n, "alpha.txt")?;
    click_note(&mut d, &n, "kilo.txt")?;
    assert_eq!(tabs(&d, &n)?.0, ["alpha.txt", "kilo.txt"], "no marker before typing");
    assert_eq!(status(&d, &n)?, "kilo.txt: 1 lines, 0 words");

    // Typing with line breaks into the library editor (kilo is empty).
    let kilo = editor(&d, &n, "kilo.txt")?;
    d.runtime_mut().focus(&kilo)?;
    d.type_text("hello  world")?;
    assert_eq!(tabs(&d, &n)?.0, ["alpha.txt", "kilo.txt •"]);
    assert_eq!(status(&d, &n)?, "kilo.txt: 1 lines, 2 words");
    d.key(KeyCode::Enter)?; // a line break through the key path
    d.type_text("second\tline\n\nfourth")?;
    let text = editor_text(&d, &n, "kilo.txt")?;
    assert_eq!(text, "hello  world\nsecond\tline\n\nfourth");
    assert_eq!(status(&d, &n)?, "kilo.txt: 4 lines, 5 words");
    assert_eq!(read(&dir, "kilo.txt"), "", "nothing written before saving");

    // Ctrl+S through the key path.
    d.chord(KeyChord::primary('s'), KeyCode::KeyS)?;
    assert_eq!(read(&dir, "kilo.txt"), text);
    assert_eq!(tabs(&d, &n)?.0, ["alpha.txt", "kilo.txt"], "marker cleared");
    assert_eq!(read(&dir, "alpha.txt"), "first note", "other tab untouched");

    // Modify alpha too, then File > Save by pointer saves only the active tab.
    click_note(&mut d, &n, "alpha.txt")?;
    let alpha = editor(&d, &n, "alpha.txt")?;
    d.runtime_mut().focus(&alpha)?;
    d.type_text(" plus\nmore")?;
    // Focusing a fresh editor leaves the caret at the start of its text.
    assert_eq!(editor_text(&d, &n, "alpha.txt")?, " plus\nmorefirst note");
    assert_eq!(tabs(&d, &n)?, (vec!["alpha.txt •".into(), "kilo.txt".into()], Some("alpha.txt •".into())));
    assert_eq!(status(&d, &n)?, "alpha.txt: 2 lines, 3 words");
    menu_click(&mut d, &n, "Save")?;
    assert_eq!(read(&dir, "alpha.txt"), editor_text(&d, &n, "alpha.txt")?);
    assert_eq!(tabs(&d, &n)?.0, ["alpha.txt", "kilo.txt"]);

    // Make kilo modified, then remove the folder: the save fails.
    click_note(&mut d, &n, "kilo.txt")?;
    d.runtime_mut().focus(&kilo)?;
    d.type_text("X")?;
    let typed = editor_text(&d, &n, "kilo.txt")?;
    let (before, _) = notifications(&d);
    assert_eq!(before, 0);
    std::fs::remove_dir_all(&dir)?;
    d.chord(KeyChord::primary('s'), KeyCode::KeyS)?;
    let (after, names) = notifications(&d);
    eprintln!("notifications after failed Ctrl+S: {after} {names:?}");
    assert_eq!(after, before + 1, "a notification in the Notifications stack");
    assert!(names.iter().any(|m| m.contains("kilo.txt")), "{names:?}");
    assert_eq!(tabs(&d, &n)?.0, ["alpha.txt", "kilo.txt •"], "marker kept");
    assert_eq!(editor_text(&d, &n, "kilo.txt")?, typed, "text kept");
    assert!(!dir.exists());
    capture(&mut d, "acc2-failed-save-notification.png")?;

    // The menu's Save fails the same way.
    menu_click(&mut d, &n, "Save")?;
    let (again, _) = notifications(&d);
    assert_eq!(again, after + 1, "menu Save failure notifies too");
    assert_eq!(tabs(&d, &n)?.0, ["alpha.txt", "kilo.txt •"]);
    Ok(())
}

fn form(d: &Driver<'_>, n: &Notes, tail: &[&str]) -> Result<WidgetId, TestError> {
    let path: Vec<&str> = ["new-dialog", "dialog", "new-form"].iter().copied().chain(tail.iter().copied()).collect();
    d.find(&n.root, &path)
}

#[test]
fn acc3_new_note_dialog_refusals_and_creation() -> R {
    let dir = own_folder("new");
    let mut app = notes_editor::app(dir.clone()).headless()?;
    let (ui, n) = app.runtime_and_state_mut();
    let n = n.clone();
    let mut d = Driver::new(ui);
    d.settle()?;
    let before = files(&dir);
    assert!(open_dialogs(&d).is_empty());

    // Ctrl+N opens a library Dialog, visible in the snapshot.
    d.chord(KeyChord::primary('n'), KeyCode::KeyN)?;
    let dialogs = open_dialogs(&d);
    eprintln!("dialogs after Ctrl+N: {dialogs:?}");
    assert_eq!(dialogs.len(), 1);
    let name = form(&d, &n, &["name"])?;
    let create = form(&d, &n, &["actions", "create"])?;
    let error = form(&d, &n, &["error"])?;
    let refusal = |d: &Driver<'_>| -> Result<Option<String>, TestError> { Ok(d.semantics(&error)?.name) };

    let long65: String = "n".repeat(65);
    let cases: Vec<(&str, &str)> = vec![
        ("", "Enter a name."),
        ("a/b", "/"),
        ("a\\b", "\\"),
        (".hidden", "dot"),
        (long65.as_str(), "64"),
        ("alpha", "alpha.txt already exists"),
        ("zulu.txt", "zulu.txt already exists"),
    ];
    for (input, expected) in cases {
        d.replace_text(&name, input)?;
        d.click(&create)?;
        let shown = refusal(&d)?.unwrap_or_default();
        eprintln!("{input:?} -> {shown:?}");
        assert!(shown.contains(expected), "{input:?}: {shown:?}");
        // The explanation is a visible Label inside the visible dialog.
        let labels: Vec<_> = visible_roles(&d, SemanticRole::Label)
            .into_iter()
            .filter(|(k, name, _)| k == "error" && name.as_deref() == Some(shown.as_str()))
            .collect();
        assert_eq!(labels.len(), 1, "explanation visible in the snapshot");
        assert_eq!(open_dialogs(&d).len(), 1, "dialog stays open");
        assert_eq!(files(&dir), before, "no file created for {input:?}");
        if input == "alpha" {
            capture(&mut d, "acc3-existing-name-refused.png")?;
        }
    }
    assert_eq!(read(&dir, "alpha.txt"), "first note");
    assert_eq!(read(&dir, "zulu.txt"), "last one");

    // A valid 64-character name: created with .txt appended, listed, opened.
    let long64: String = "b".repeat(64);
    d.replace_text(&name, &long64)?;
    d.click(&create)?;
    assert!(open_dialogs(&d).is_empty(), "dialog closed");
    let file64 = format!("{long64}.txt");
    assert_eq!(read(&dir, &file64), "");
    assert_eq!(listed(&d, &n)?, ["alpha.txt", file64.as_str(), "kilo.txt", "mike.txt", "zulu.txt"]);
    assert_eq!(tabs(&d, &n)?, (vec![file64.clone()], Some(file64.clone())));

    // File > New note by pointer; a name already ending in .txt keeps it.
    menu_click(&mut d, &n, "New note")?;
    assert_eq!(open_dialogs(&d).len(), 1);
    assert_eq!(refusal(&d)?, None, "the explanation is cleared on reopening");
    d.replace_text(&name, "charlie.txt")?;
    d.click(&create)?;
    assert!(open_dialogs(&d).is_empty());
    assert!(dir.join("charlie.txt").is_file());
    assert!(!dir.join("charlie.txt.txt").exists());
    assert_eq!(listed(&d, &n)?, ["alpha.txt", file64.as_str(), "charlie.txt", "kilo.txt", "mike.txt", "zulu.txt"]);
    let (t, sel) = tabs(&d, &n)?;
    assert_eq!(t, [file64.clone(), "charlie.txt".into()]);
    assert_eq!(sel.as_deref(), Some("charlie.txt"));
    assert_eq!(status(&d, &n)?, "charlie.txt: 1 lines, 0 words");
    let mut expected = before.clone();
    expected.push(file64.clone());
    expected.push("charlie.txt".into());
    expected.sort();
    assert_eq!(files(&dir), expected, "exactly the two valid notes were created");
    Ok(())
}

fn close_btn(d: &Driver<'_>, n: &Notes, key: &str) -> Result<WidgetId, TestError> {
    d.find(&n.root, &["new-dialog", "close-dialog", "dialog", "close-form", "actions", key])
}

#[test]
fn acc4_close_choices_and_unmodified_close() -> R {
    let dir = own_folder("close");
    let mut app = notes_editor::app(dir.clone()).headless()?;
    let (ui, n) = app.runtime_and_state_mut();
    let n = n.clone();
    let mut d = Driver::new(ui);
    d.settle()?;
    click_note(&mut d, &n, "alpha.txt")?;
    click_note(&mut d, &n, "mike.txt")?;
    click_note(&mut d, &n, "zulu.txt")?;
    let original_zulu = read(&dir, "zulu.txt");

    // Unmodified tab: Ctrl+W closes at once, no dialog in the snapshot.
    d.chord(KeyChord::primary('w'), KeyCode::KeyW)?;
    assert!(open_dialogs(&d).is_empty(), "no dialog");
    assert!(open_dialogs(&d).is_empty(), "no alert");
    assert_eq!(tabs(&d, &n)?.0, ["alpha.txt", "mike.txt"]);
    // Unmodified tab via File > Close tab by pointer.
    click_note(&mut d, &n, "zulu.txt")?;
    menu_click(&mut d, &n, "Close tab")?;
    assert!(open_dialogs(&d).is_empty(), "no dialog from the menu");
    assert_eq!(tabs(&d, &n)?.0, ["alpha.txt", "mike.txt"]);
    assert_eq!(read(&dir, "zulu.txt"), original_zulu);

    // Modify mike.
    click_note(&mut d, &n, "mike.txt")?;
    let mike = editor(&d, &n, "mike.txt")?;
    d.runtime_mut().focus(&mike)?;
    d.type_text("edit\n")?;
    let edited = editor_text(&d, &n, "mike.txt")?;
    assert_eq!(edited, "edit\nmiddle\nof the\nlist");
    assert_eq!(tabs(&d, &n)?.0, ["alpha.txt", "mike.txt •"]);

    // Cancel (via Ctrl+W): tab and text kept, file unchanged.
    d.chord(KeyChord::primary('w'), KeyCode::KeyW)?;
    let dialogs = open_dialogs(&d);
    eprintln!("close dialog: {dialogs:?}");
    assert_eq!(dialogs.len(), 1);
    let cancel = close_btn(&d, &n, "cancel")?;
    d.click(&cancel)?;
    assert!(open_dialogs(&d).is_empty());
    assert_eq!(tabs(&d, &n)?, (vec!["alpha.txt".into(), "mike.txt •".into()], Some("mike.txt •".into())));
    assert_eq!(editor_text(&d, &n, "mike.txt")?, edited);
    assert_eq!(read(&dir, "mike.txt"), "middle\nof the\nlist");

    // Discard (via File > Close tab by pointer): tab gone, file unchanged.
    menu_click(&mut d, &n, "Close tab")?;
    assert_eq!(open_dialogs(&d).len(), 1);
    let discard = close_btn(&d, &n, "discard")?;
    d.click(&discard)?;
    assert!(open_dialogs(&d).is_empty());
    assert_eq!(tabs(&d, &n)?.0, ["alpha.txt"]);
    assert_eq!(read(&dir, "mike.txt"), "middle\nof the\nlist", "discard writes nothing");

    // Reopen shows the file's text again; modify and Save: file written, tab gone.
    click_note(&mut d, &n, "mike.txt")?;
    assert_eq!(editor_text(&d, &n, "mike.txt")?, "middle\nof the\nlist");
    let mike = editor(&d, &n, "mike.txt")?;
    d.runtime_mut().focus(&mike)?;
    d.type_text("saved ")?;
    let to_save = editor_text(&d, &n, "mike.txt")?;
    d.chord(KeyChord::primary('w'), KeyCode::KeyW)?;
    assert_eq!(open_dialogs(&d).len(), 1);
    assert_eq!(read(&dir, "mike.txt"), "middle\nof the\nlist", "not written before choosing");
    let save = close_btn(&d, &n, "save")?;
    d.click(&save)?;
    assert!(open_dialogs(&d).is_empty());
    assert_eq!(read(&dir, "mike.txt"), to_save, "Save wrote the text");
    assert_eq!(tabs(&d, &n)?.0, ["alpha.txt"]);

    // Save with a failing write: notification, tab kept and modified.
    let alpha = editor(&d, &n, "alpha.txt")?;
    d.runtime_mut().focus(&alpha)?;
    d.type_text("unsaved ")?;
    let (before, _) = notifications(&d);
    let moved = dir.with_extension("moved");
    std::fs::rename(&dir, &moved)?;
    d.chord(KeyChord::primary('w'), KeyCode::KeyW)?;
    let save = close_btn(&d, &n, "save")?;
    d.click(&save)?;
    let (after, names) = notifications(&d);
    eprintln!("after failed dialog Save: {after} {names:?}");
    assert_eq!(after, before + 1);
    assert_eq!(tabs(&d, &n)?.0, ["alpha.txt •"], "failed save keeps the tab, modified");
    assert_eq!(editor_text(&d, &n, "alpha.txt")?, "unsaved first note");
    std::fs::rename(&moved, &dir)?;
    assert_eq!(read(&dir, "alpha.txt"), "first note");

    // Discard the last tab: status line empties.
    d.chord(KeyChord::primary('w'), KeyCode::KeyW)?;
    let discard = close_btn(&d, &n, "discard")?;
    d.click(&discard)?;
    assert_eq!(tabs(&d, &n)?.0.len(), 0);
    assert_eq!(status(&d, &n)?, "");
    assert_eq!(read(&dir, "alpha.txt"), "first note");
    Ok(())
}

#[test]
fn acc5_cpu_capture_two_tabs_one_modified() -> R {
    let dir = own_folder("capture");
    let mut app = notes_editor::app(dir).headless()?;
    let (ui, n) = app.runtime_and_state_mut();
    let n = n.clone();
    let mut d = Driver::new(ui);
    d.settle()?;
    click_note(&mut d, &n, "mike.txt")?;
    click_note(&mut d, &n, "zulu.txt")?;
    let zulu = editor(&d, &n, "zulu.txt")?;
    d.runtime_mut().focus(&zulu)?;
    d.type_text("Verifier edit\n")?;
    d.clear_focus()?;
    assert_eq!(tabs(&d, &n)?.0, ["mike.txt", "zulu.txt •"]);

    d.runtime().set_capture_support(true);
    let pending = d.runtime().request_capture(CaptureRequest::new(CaptureTarget::window(
        &d.runtime().primary_window(),
    )));
    d.settle()?;
    CpuRenderer::new().serve_captures(d.runtime_mut());
    let capture = pending.try_reply()?.value?;
    let out = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("acceptance-captures");
    std::fs::create_dir_all(&out)?;
    let file = out.join("acc5-two-tabs-one-modified.png");
    std::fs::write(&file, capture.image.encode_png())?;
    assert!(std::fs::read(&file)?.starts_with(b"\x89PNG"));
    eprintln!("capture {}x{} -> {}", capture.image.width(), capture.image.height(), file.display());
    Ok(())
}
