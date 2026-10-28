// Independent acceptance checks added by the verifier (round 3, task 6).
//
// They start the submitted application with `App::headless` on folders of
// their own and drive it only through pert-ui's public facilities: Driver
// input (pointer clicks, typed text, named keys, key chords), raw
// `Runtime::pointer_input` for menu-bar clicks at inspected geometry,
// semantic inspection and the CPU renderer's capture service. The only
// value read from application state is the root `WidgetId` the application
// stored from `mount_tree` (`root_handle`), used as the starting point for
// `Driver::find`; the library offers no other public accessor for it.
use super::*;
use pert_ui::testing::*;
use std::path::Path;
use std::time::Duration;

type R = Result<(), Box<dyn std::error::Error>>;

const FILES: &[&str] = &["new-dialog", "menu", "content", "split", "files"];
const TABS: &[&str] = &["new-dialog", "menu", "content", "split", "documents", "tabs"];
const STATUS: &[&str] = &["new-dialog", "menu", "content", "status-host", "status"];
const NOTIFICATION: &[&str] = &["new-dialog", "menu", "content", "notification"];
const NEW_FORM: &[&str] = &["new-dialog", "dialog", "new-form"];
const CLOSE_FORM: &[&str] = &["dialog", "close-form"];

// ---------------------------------------------------------------- helpers

fn dump(d: &Driver<'_>, tag: &str) {
    let snap = d.runtime().inspect();
    println!("==== dump {tag}");
    for c in &snap.components {
        let s = c.details.semantics.as_ref();
        println!(
            "id={} parent={:?} key={:?} path={:?} vis={} foc={} role={:?} name={:?} value={:?} box={:?} items={:?}",
            c.id,
            c.parent,
            c.key,
            c.path,
            c.visible,
            c.focused,
            s.map(|s| s.role),
            s.and_then(|s| s.name.clone()),
            s.and_then(|s| s.value.clone()),
            c.details.box_model.border_box,
            s.map(|s| s
                .items
                .iter()
                .map(|i| (i.name.clone(), i.selected, i.bounds))
                .collect::<Vec<_>>()),
        );
    }
}

/// A new, empty notes folder of the verifier's own.
fn fresh_folder(name: &str) -> PathBuf {
    let folder = Path::new("/work/acceptance-notes").join(name);
    if folder.exists() {
        std::fs::remove_dir_all(&folder).unwrap();
    }
    std::fs::create_dir_all(&folder).unwrap();
    folder
}

/// Writes files in the given order, pausing so creation times differ.
fn populate(folder: &Path, files: &[(&str, &str)]) {
    for (name, text) in files {
        std::fs::write(folder.join(name), text).unwrap();
        std::thread::sleep(Duration::from_millis(15));
    }
}

fn dir_listing(folder: &Path) -> Vec<String> {
    let mut v: Vec<String> = std::fs::read_dir(folder)
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .collect();
    v.sort();
    v
}

/// Test access: the root handle the application stored from `mount_tree`.
fn root_handle(app: &HeadlessApp<Editor>) -> WidgetId {
    app.state().root.borrow().clone().expect("mounted root")
}

fn control(c: char) -> KeyChord {
    KeyChord::new(
        CommandKey::Character(c),
        Modifiers {
            control: true,
            ..Modifiers::default()
        },
    )
}

fn shortcut(d: &mut Driver<'_>, c: char) {
    let code = match c {
        's' => KeyCode::KeyS,
        'n' => KeyCode::KeyN,
        'w' => KeyCode::KeyW,
        _ => unreachable!(),
    };
    d.chord(control(c), code).unwrap();
    settle(d);
}

fn settle(d: &mut Driver<'_>) {
    for _ in 0..3 {
        d.settle().unwrap();
    }
}

/// Lets a double-click interval pass so the next click is a single click.
fn pause(d: &mut Driver<'_>) {
    d.advance(Duration::from_secs(1)).unwrap();
}

/// Components in the current inspection whose key is `key`.
fn by_key(d: &Driver<'_>, key: &str) -> Vec<pert_ui::ComponentInspection> {
    d.runtime()
        .inspect()
        .components
        .into_iter()
        .filter(|c| c.key == key)
        .collect()
}

/// Inspection record of the component at address `path` below the root.
fn record(d: &Driver<'_>, root: &WidgetId, path: &[&str]) -> pert_ui::ComponentInspection {
    let id = if path.is_empty() {
        root.clone()
    } else {
        d.find(root, path).unwrap()
    };
    let inspection = d.runtime().inspection_id(&id).unwrap();
    d.runtime()
        .inspect()
        .components
        .into_iter()
        .find(|c| c.id == inspection)
        .unwrap()
}

fn center(b: [f32; 4]) -> (f32, f32) {
    (b[0] + b[2] / 2.0, b[1] + b[3] / 2.0)
}

fn pointer(d: &mut Driver<'_>, at: (f32, f32), kind: PointerKind) {
    let p = Point::new(at.0, at.1).unwrap();
    d.runtime_mut()
        .pointer_input(PointerEvent::new(p, kind))
        .unwrap();
}

/// A primary-button click at window point `at` through the runtime's pointer
/// input, as a host reports it, then a settle.
fn press_at(d: &mut Driver<'_>, at: (f32, f32)) {
    pointer(d, at, PointerKind::Move);
    pointer(d, at, PointerKind::Press(PointerButton::Primary));
    pointer(d, at, PointerKind::Release(PointerButton::Primary));
    settle(d);
}

/// Chooses `row` from the File menu with simulated pointer clicks: a click
/// on the File title at its inspected bounds, then a move onto the row, an
/// inspection check that the pointer highlights exactly that row, and a
/// click there. Menu rows report no item bounds at this library version, so
/// the row rectangle is the open menu panel's inspected bounds divided by
/// its item count (the File menu has no separators).
fn menu_choose(d: &mut Driver<'_>, row: &str) {
    pause(d);
    let title = by_key(d, "title/file")
        .into_iter()
        .find(|c| c.visible)
        .expect("visible File title");
    assert_eq!(
        title.details.semantics.as_ref().unwrap().name.as_deref(),
        Some("File")
    );
    press_at(d, center(title.details.box_model.border_box.unwrap()));
    let panel = by_key(d, "items")
        .into_iter()
        .find(|c| {
            c.visible
                && c.details
                    .semantics
                    .as_ref()
                    .is_some_and(|s| s.items.iter().any(|i| i.name.as_deref() == Some("Save")))
        })
        .expect("the File menu is open");
    let items = panel.details.semantics.clone().unwrap().items;
    let names: Vec<_> = items.iter().map(|i| i.name.clone().unwrap()).collect();
    assert_eq!(names, ["New note", "Save", "Close tab"]);
    let index = names.iter().position(|n| n == row).unwrap();
    let [x, y, w, h] = panel.details.box_model.border_box.unwrap();
    let row_h = h / items.len() as f32;
    let at = (x + w / 2.0, y + row_h * index as f32 + row_h / 2.0);
    pointer(d, at, PointerKind::Move);
    d.settle().unwrap();
    let hovered = by_key(d, "items")
        .into_iter()
        .find(|c| c.id == panel.id)
        .unwrap();
    let selected: Vec<_> = hovered
        .details
        .semantics
        .unwrap()
        .items
        .into_iter()
        .filter(|i| i.selected)
        .filter_map(|i| i.name)
        .collect();
    assert_eq!(selected, [row], "pointer is over the intended menu row");
    pointer(d, at, PointerKind::Press(PointerButton::Primary));
    pointer(d, at, PointerKind::Release(PointerButton::Primary));
    settle(d);
    assert!(
        !by_key(d, "items").iter().any(|c| c.id == panel.id && c.visible),
        "the menu closed after choosing {row}"
    );
}

/// Names listed in the sidebar, in display order, from inspection.
fn listed(d: &Driver<'_>, root: &WidgetId) -> Vec<String> {
    let files = d.find(root, FILES).unwrap();
    let rows = d.address_children(&files).unwrap();
    let mut named: Vec<(f32, String)> = rows
        .iter()
        .map(|r| {
            let s = d.semantics(&r.id).unwrap();
            let b = d.runtime().bounds(&r.id).unwrap().unwrap();
            assert_eq!(s.name.as_deref(), Some(r.segment.as_str()));
            (b.y(), s.name.unwrap())
        })
        .collect();
    named.sort_by(|a, b| a.0.total_cmp(&b.0));
    named.into_iter().map(|(_, n)| n).collect()
}

/// Tab titles and the selected one, from the tab list's semantics.
fn tabs(d: &Driver<'_>, root: &WidgetId) -> (Vec<String>, Option<String>) {
    let s = d.semantics(&d.find(root, TABS).unwrap()).unwrap();
    assert_eq!(s.role, SemanticRole::TabList);
    let titles = s.items.iter().filter_map(|i| i.name.clone()).collect();
    let selected = s.items.into_iter().find(|i| i.selected).and_then(|i| i.name);
    (titles, selected)
}

fn label_text(d: &Driver<'_>, root: &WidgetId, path: &[&str]) -> String {
    d.semantics(&d.find(root, path).unwrap())
        .unwrap()
        .name
        .unwrap_or_default()
}

fn status(d: &Driver<'_>, root: &WidgetId) -> String {
    label_text(d, root, STATUS)
}

fn editor_path(name: &str) -> Vec<&str> {
    let mut p = TABS.to_vec();
    p.push(name);
    p.push("editor");
    p
}

fn editor_text(d: &Driver<'_>, root: &WidgetId, name: &str) -> String {
    let s = d.semantics(&d.find(root, &editor_path(name)).unwrap()).unwrap();
    assert_eq!(s.role, SemanticRole::MultilineTextInput);
    match s.value {
        Some(SemanticValue::Text(t)) => t,
        other => panic!("editor value {other:?}"),
    }
}

/// Clicks a note's row in the sidebar with the pointer.
fn open_by_pointer(d: &mut Driver<'_>, root: &WidgetId, name: &str) {
    pause(d);
    let files = d.find(root, FILES).unwrap();
    let row = d.find(&files, &[name]).unwrap();
    d.click(&row).unwrap();
    settle(d);
}

/// Focuses the note's editor with a pointer click and moves the caret to the
/// end of the text with Ctrl+End.
fn focus_editor_end(d: &mut Driver<'_>, root: &WidgetId, name: &str) {
    pause(d);
    let editor = d.find(root, &editor_path(name)).unwrap();
    d.click(&editor).unwrap();
    assert_eq!(d.runtime().focused(), Some(editor.clone()));
    d.chord(
        KeyChord::new(
            CommandKey::Named(KeyCode::End),
            Modifiers {
                control: true,
                ..Modifiers::default()
            },
        ),
        KeyCode::End,
    )
    .unwrap();
}

/// Whether any dialog panel or either application form is presented.
fn any_dialog_visible(d: &Driver<'_>) -> bool {
    d.runtime().inspect().components.iter().any(|c| {
        c.visible && (c.key == "panel" || c.key == "new-form" || c.key == "close-form")
    })
}

fn form_visible(d: &Driver<'_>, root: &WidgetId, path: &[&str]) -> bool {
    record(d, root, path).visible
}

fn capture(d: &mut Driver<'_>, path: &str) {
    d.runtime().set_capture_support(true);
    let pending = d
        .runtime()
        .request_capture(CaptureRequest::new(CaptureTarget::window(
            &d.runtime().primary_window(),
        )));
    settle(d);
    CpuRenderer::new().serve_captures(d.runtime_mut());
    let image = pending.try_reply().unwrap().value.unwrap().image;
    std::fs::create_dir_all("/work/acceptance-captures").unwrap();
    std::fs::write(path, image.encode_png()).unwrap();
    println!("capture written: {path}");
}

// ---------------------------------------------------------------- item 1

#[test]
fn item1_list_from_own_folder_open_and_reopen() -> R {
    let folder = fresh_folder("item1");
    // Created out of alphabetical order, with non-.txt files and a directory.
    populate(
        &folder,
        &[
            ("pear.txt", "pear text"),
            ("todo.md", "not a note"),
            ("apple.txt", "apple one\napple two"),
            ("data.json", "{}"),
            ("mango.txt", "mango"),
            ("README", "x"),
        ],
    );
    std::fs::create_dir(folder.join("archive")).unwrap();
    let mut app = app(folder.clone()).headless()?;
    let root = root_handle(&app);
    let mut d = Driver::new(app.runtime_mut());
    settle(&mut d);

    assert_eq!(listed(&d, &root), ["apple.txt", "mango.txt", "pear.txt"]);
    assert_eq!(tabs(&d, &root).0.len(), 0);
    assert_eq!(status(&d, &root), "", "status empty with no tab");

    // Open by pointer.
    open_by_pointer(&mut d, &root, "apple.txt");
    assert_eq!(tabs(&d, &root), (vec!["apple.txt".into()], Some("apple.txt".into())));
    assert_eq!(editor_text(&d, &root, "apple.txt"), "apple one\napple two");

    // Open by keyboard: arrows on the focused list, then Enter.
    let files = d.find(&root, FILES)?;
    d.runtime_mut().focus(&files)?;
    for _ in 0..5 {
        let s = d.semantics(&files)?;
        if let Some(SemanticValue::Choice { label: Some(l), .. }) = &s.value {
            if l == "pear.txt" {
                break;
            }
        }
        d.key(KeyCode::ArrowDown)?;
    }
    d.key(KeyCode::Enter)?;
    settle(&mut d);
    assert_eq!(
        tabs(&d, &root),
        (
            vec!["apple.txt".into(), "pear.txt".into()],
            Some("pear.txt".into())
        )
    );
    assert_eq!(status(&d, &root), "pear.txt: 1 lines, 2 words");
    assert!(record(&d, &root, &editor_path("pear.txt")).visible);
    assert!(!record(&d, &root, &editor_path("apple.txt")).visible);

    // Reopen apple by pointer: same tab count, its tab becomes active.
    open_by_pointer(&mut d, &root, "apple.txt");
    assert_eq!(
        tabs(&d, &root),
        (
            vec!["apple.txt".into(), "pear.txt".into()],
            Some("apple.txt".into())
        )
    );
    assert_eq!(status(&d, &root), "apple.txt: 2 lines, 4 words");
    assert!(record(&d, &root, &editor_path("apple.txt")).visible);
    assert!(!record(&d, &root, &editor_path("pear.txt")).visible);

    // Reopen pear by keyboard Enter on the list: still two tabs, pear active.
    d.runtime_mut().focus(&files)?;
    d.key(KeyCode::Enter)?;
    settle(&mut d);
    let (titles, selected) = tabs(&d, &root);
    assert_eq!(titles.len(), 2);
    println!("after keyboard reopen: {titles:?} selected {selected:?}");
    assert_eq!(selected.as_deref(), Some("pear.txt"));
    assert_eq!(status(&d, &root), "pear.txt: 1 lines, 2 words");
    Ok(())
}

// ---------------------------------------------------------------- item 2

#[test]
fn item2_typing_marker_status_save_paths_and_failure() -> R {
    let folder = fresh_folder("item2");
    populate(&folder, &[("zeta.txt", "z"), ("beta.txt", "first line")]);
    let mut app = app(folder.clone()).headless()?;
    let root = root_handle(&app);
    let mut d = Driver::new(app.runtime_mut());
    settle(&mut d);
    open_by_pointer(&mut d, &root, "zeta.txt");
    open_by_pointer(&mut d, &root, "beta.txt");
    assert_eq!(tabs(&d, &root).1.as_deref(), Some("beta.txt"));
    assert_eq!(status(&d, &root), "beta.txt: 1 lines, 2 words");
    assert_eq!(tabs(&d, &root).0, ["zeta.txt", "beta.txt"]);

    // Simulated typing, including line breaks by Enter and by typed "\n".
    focus_editor_end(&mut d, &root, "beta.txt");
    d.type_text(" more")?;
    assert_eq!(tabs(&d, &root).0, ["zeta.txt", "beta.txt *"]);
    assert_eq!(status(&d, &root), "beta.txt: 1 lines, 3 words");
    d.key(KeyCode::Enter)?;
    assert_eq!(status(&d, &root), "beta.txt: 2 lines, 3 words");
    d.type_text("second   line\nthird")?;
    let typed = "first line more\nsecond   line\nthird";
    assert_eq!(editor_text(&d, &root, "beta.txt"), typed);
    assert_eq!(status(&d, &root), "beta.txt: 3 lines, 6 words");
    assert_eq!(std::fs::read_to_string(folder.join("beta.txt"))?, "first line");

    // Ctrl+S, with focus still in the editor.
    assert!(d.runtime().inspect().components.iter().any(|c| c.focused && c.key == "editor"));
    shortcut(&mut d, 's');
    assert_eq!(std::fs::read_to_string(folder.join("beta.txt"))?, typed);
    assert_eq!(tabs(&d, &root).0, ["zeta.txt", "beta.txt"]);
    assert_eq!(std::fs::read_to_string(folder.join("zeta.txt"))?, "z");

    // Typing again marks it; File > Save by pointer clicks writes and clears.
    focus_editor_end(&mut d, &root, "beta.txt");
    d.key(KeyCode::Enter)?;
    d.type_text("via menu")?;
    let typed2 = format!("{typed}\nvia menu");
    assert_eq!(tabs(&d, &root).0, ["zeta.txt", "beta.txt *"]);
    assert_eq!(status(&d, &root), "beta.txt: 4 lines, 8 words");
    menu_choose(&mut d, "Save");
    assert_eq!(std::fs::read_to_string(folder.join("beta.txt"))?, typed2);
    assert_eq!(tabs(&d, &root).0, ["zeta.txt", "beta.txt"]);
    assert_eq!(label_text(&d, &root, NOTIFICATION), "");

    // A save into the removed notes folder: notification shown, marker kept.
    focus_editor_end(&mut d, &root, "beta.txt");
    d.type_text(" unsaved")?;
    assert_eq!(tabs(&d, &root).0, ["zeta.txt", "beta.txt *"]);
    std::fs::remove_dir_all(&folder)?;
    shortcut(&mut d, 's');
    let note = record(&d, &root, NOTIFICATION);
    let message = note.details.semantics.as_ref().unwrap().name.clone();
    println!(
        "notification after failed Ctrl+S: {message:?} box {:?}",
        note.details.box_model.border_box
    );
    assert!(note.visible);
    let message = message.expect("a failure message is shown");
    assert!(message.contains("Save failed") && message.contains("beta.txt"), "{message}");
    let b = note.details.box_model.border_box.unwrap();
    assert!(b[2] > 0.0 && b[3] > 0.0 && b[1] + b[3] <= 640.0, "on screen: {b:?}");
    assert_eq!(tabs(&d, &root).0, ["zeta.txt", "beta.txt *"], "marker kept");
    assert_eq!(editor_text(&d, &root, "beta.txt"), format!("{typed2} unsaved"));
    assert!(!folder.exists());
    capture(&mut d, "/work/acceptance-captures/item2-failed-save.png");

    // The menu route fails the same way.
    menu_choose(&mut d, "Save");
    assert!(label_text(&d, &root, NOTIFICATION).contains("Save failed"));
    assert_eq!(tabs(&d, &root).0, ["zeta.txt", "beta.txt *"]);
    Ok(())
}

// ---------------------------------------------------------------- item 3

fn type_name(d: &mut Driver<'_>, root: &WidgetId, name: &str) {
    let field = d.find(root, &[NEW_FORM, &["name"]].concat()).unwrap();
    // Clear with select-all/replace, then type the name as keyboard text.
    d.replace_text(&field, "").unwrap();
    assert_eq!(d.runtime().focused(), Some(field.clone()));
    d.type_text(name).unwrap();
    assert_eq!(
        d.semantics(&field).unwrap().value,
        Some(SemanticValue::Text(name.into()))
    );
}

fn click_create(d: &mut Driver<'_>, root: &WidgetId) {
    pause(d);
    let create = d
        .find(root, &[NEW_FORM, &["actions", "create"]].concat())
        .unwrap();
    d.click(&create).unwrap();
    settle(d);
}

#[test]
fn item3_new_note_dialog_refusals_and_creation() -> R {
    let folder = fresh_folder("item3");
    populate(&folder, &[("pear.txt", "p"), ("apple.txt", "a"), ("notes.md", "m")]);
    let before = dir_listing(&folder);
    let mut app = app(folder.clone()).headless()?;
    let root = root_handle(&app);
    let mut d = Driver::new(app.runtime_mut());
    settle(&mut d);
    assert!(!any_dialog_visible(&d));

    // Ctrl+N opens the library dialog with the form.
    shortcut(&mut d, 'n');
    assert!(form_visible(&d, &root, NEW_FORM));
    let host = record(&d, &root, &["new-dialog"]);
    assert_eq!(host.details.semantics.as_ref().unwrap().role, SemanticRole::Dialog);
    assert!(by_key(&d, "panel").iter().any(|c| c.visible));
    let error_path = [NEW_FORM, &["error"]].concat();
    assert_eq!(label_text(&d, &root, &error_path), "");

    let refusals: Vec<(String, &str)> = vec![
        (String::new(), "1–64"),
        ("a/b".into(), "/"),
        ("apple".into(), "already exists"),
        ("apple.txt".into(), "already exists"),
        ("back\\slash".into(), "\\"),
        (".hidden".into(), "dot"),
        ("x".repeat(65), "1–64"),
    ];
    for (name, explanation) in &refusals {
        if name.is_empty() {
            let field = d.find(&root, &[NEW_FORM, &["name"]].concat())?;
            d.replace_text(&field, "")?;
        } else {
            type_name(&mut d, &root, name);
        }
        click_create(&mut d, &root);
        let error = record(&d, &root, &error_path);
        let text = error
            .details
            .semantics
            .as_ref()
            .unwrap()
            .name
            .clone()
            .unwrap_or_default();
        println!("refused {name:?}: {text:?}");
        assert!(error.visible, "explanation visible for {name:?}");
        assert!(text.contains(explanation), "{name:?}: {text}");
        assert!(error.details.box_model.border_box.unwrap()[3] > 0.0);
        assert!(form_visible(&d, &root, NEW_FORM), "dialog stays open for {name:?}");
        assert_eq!(dir_listing(&folder), before, "no file created for {name:?}");
        assert_eq!(listed(&d, &root), ["apple.txt", "pear.txt"]);
        assert_eq!(tabs(&d, &root).0.len(), 0);
    }
    assert!(!folder.join("a").exists());
    capture(&mut d, "/work/acceptance-captures/item3-refusal.png");

    // A valid name creates, lists in order and opens the note.
    type_name(&mut d, &root, "kiwi");
    click_create(&mut d, &root);
    assert!(!form_visible(&d, &root, NEW_FORM));
    assert!(!any_dialog_visible(&d));
    assert_eq!(std::fs::read_to_string(folder.join("kiwi.txt"))?, "");
    assert_eq!(listed(&d, &root), ["apple.txt", "kiwi.txt", "pear.txt"]);
    assert_eq!(tabs(&d, &root), (vec!["kiwi.txt".into()], Some("kiwi.txt".into())));
    assert_eq!(status(&d, &root), "kiwi.txt: 1 lines, 0 words");

    // File > New note by pointer; a name ending in .txt is not doubled; the
    // previous explanation is cleared; Cancel creates nothing.
    menu_choose(&mut d, "New note");
    assert!(form_visible(&d, &root, NEW_FORM));
    assert_eq!(label_text(&d, &root, &error_path), "");
    type_name(&mut d, &root, "fig.txt");
    click_create(&mut d, &root);
    assert!(folder.join("fig.txt").exists() && !folder.join("fig.txt.txt").exists());
    assert_eq!(listed(&d, &root), ["apple.txt", "fig.txt", "kiwi.txt", "pear.txt"]);
    assert_eq!(
        tabs(&d, &root),
        (vec!["kiwi.txt".into(), "fig.txt".into()], Some("fig.txt".into()))
    );

    // Ctrl+N with focus inside the text editor still opens the dialog.
    focus_editor_end(&mut d, &root, "fig.txt");
    assert!(!form_visible(&d, &root, NEW_FORM));
    shortcut(&mut d, 'n');
    assert!(form_visible(&d, &root, NEW_FORM));
    assert_eq!(editor_text(&d, &root, "fig.txt"), "", "no character typed");
    let max = "y".repeat(64);
    type_name(&mut d, &root, &max);
    click_create(&mut d, &root);
    assert!(folder.join(format!("{max}.txt")).exists(), "64 characters accepted");
    assert_eq!(tabs(&d, &root).0.len(), 3);

    shortcut(&mut d, 'n');
    type_name(&mut d, &root, "cancelled");
    pause(&mut d);
    let cancel = d.find(&root, &[NEW_FORM, &["actions", "cancel"]].concat())?;
    d.click(&cancel)?;
    settle(&mut d);
    assert!(!form_visible(&d, &root, NEW_FORM));
    assert!(!folder.join("cancelled.txt").exists());
    Ok(())
}

// ---------------------------------------------------------------- item 4

fn close_action(d: &mut Driver<'_>, root: &WidgetId, action: &str) {
    pause(d);
    let button = d
        .find(root, &[CLOSE_FORM, &["actions", action]].concat())
        .unwrap();
    d.click(&button).unwrap();
    settle(d);
}

#[test]
fn item4_close_choices_and_unmodified_close() -> R {
    let folder = fresh_folder("item4");
    populate(&folder, &[("plum.txt", "plum on disk"), ("fig.txt", "fig on disk")]);
    let mut app = app(folder.clone()).headless()?;
    let root = root_handle(&app);
    let mut d = Driver::new(app.runtime_mut());
    settle(&mut d);
    open_by_pointer(&mut d, &root, "fig.txt");
    open_by_pointer(&mut d, &root, "plum.txt");
    focus_editor_end(&mut d, &root, "plum.txt");
    d.type_text(" edited")?;
    assert_eq!(tabs(&d, &root).0, ["fig.txt", "plum.txt *"]);

    // Cancel (Ctrl+W route): tab and text kept, file unchanged.
    shortcut(&mut d, 'w');
    assert!(form_visible(&d, &root, CLOSE_FORM));
    assert_eq!(
        record(&d, &root, &[]).details.semantics.unwrap().role,
        SemanticRole::Dialog
    );
    capture(&mut d, "/work/acceptance-captures/item4-close-dialog.png");
    close_action(&mut d, &root, "cancel");
    assert!(!any_dialog_visible(&d));
    assert_eq!(
        tabs(&d, &root),
        (
            vec!["fig.txt".into(), "plum.txt *".into()],
            Some("plum.txt *".into())
        )
    );
    assert_eq!(editor_text(&d, &root, "plum.txt"), "plum on disk edited");
    assert_eq!(std::fs::read_to_string(folder.join("plum.txt"))?, "plum on disk");

    // Discard (File > Close tab by pointer): tab gone, file unchanged.
    menu_choose(&mut d, "Close tab");
    assert!(form_visible(&d, &root, CLOSE_FORM));
    close_action(&mut d, &root, "discard");
    assert!(!any_dialog_visible(&d));
    assert_eq!(tabs(&d, &root).0, ["fig.txt"]);
    assert_eq!(std::fs::read_to_string(folder.join("plum.txt"))?, "plum on disk");
    println!(
        "after discard: tabs {:?} status {:?}",
        tabs(&d, &root),
        status(&d, &root)
    );

    // Save: written before the tab disappears.
    open_by_pointer(&mut d, &root, "plum.txt");
    assert_eq!(editor_text(&d, &root, "plum.txt"), "plum on disk");
    focus_editor_end(&mut d, &root, "plum.txt");
    d.key(KeyCode::Enter)?;
    d.type_text("saved on close")?;
    assert_eq!(tabs(&d, &root).0, ["fig.txt", "plum.txt *"]);
    shortcut(&mut d, 'w');
    assert!(form_visible(&d, &root, CLOSE_FORM));
    close_action(&mut d, &root, "save");
    assert!(!any_dialog_visible(&d));
    assert_eq!(tabs(&d, &root).0, ["fig.txt"]);
    assert_eq!(
        std::fs::read_to_string(folder.join("plum.txt"))?,
        "plum on disk\nsaved on close"
    );

    // Unmodified tab, by Ctrl+W: closes with no dialog in the snapshot.
    assert_eq!(tabs(&d, &root), (vec!["fig.txt".into()], Some("fig.txt".into())));
    shortcut(&mut d, 'w');
    assert!(!any_dialog_visible(&d), "no dialog for an unmodified tab");
    assert!(!form_visible(&d, &root, CLOSE_FORM));
    assert_eq!(tabs(&d, &root).0.len(), 0);
    assert_eq!(status(&d, &root), "");
    assert_eq!(std::fs::read_to_string(folder.join("fig.txt"))?, "fig on disk");

    // Unmodified tab, by File > Close tab: same.
    open_by_pointer(&mut d, &root, "fig.txt");
    menu_choose(&mut d, "Close tab");
    assert!(!any_dialog_visible(&d), "no dialog for an unmodified tab (menu)");
    assert_eq!(tabs(&d, &root).0.len(), 0);

    // Save choice whose save fails keeps the tab open and modified.
    open_by_pointer(&mut d, &root, "plum.txt");
    focus_editor_end(&mut d, &root, "plum.txt");
    d.type_text("!")?;
    std::fs::remove_dir_all(&folder)?;
    shortcut(&mut d, 'w');
    close_action(&mut d, &root, "save");
    assert_eq!(tabs(&d, &root).0, ["plum.txt *"], "failed save keeps the tab");
    assert!(label_text(&d, &root, NOTIFICATION).contains("Save failed"));
    assert_eq!(editor_text(&d, &root, "plum.txt"), "plum on disk\nsaved on close!");
    println!("after failed close-save: dialog visible {}", any_dialog_visible(&d));
    Ok(())
}

// ---------------------------------------------------------------- item 5

#[test]
fn item5_cpu_capture_two_tabs_one_modified() -> R {
    let folder = fresh_folder("item5");
    populate(
        &folder,
        &[
            ("shopping.txt", "milk\neggs\nbread"),
            ("ideas.txt", "Write a notes editor"),
        ],
    );
    let mut app = app(folder.clone()).headless()?;
    let root = root_handle(&app);
    let mut d = Driver::new(app.runtime_mut());
    settle(&mut d);
    open_by_pointer(&mut d, &root, "ideas.txt");
    open_by_pointer(&mut d, &root, "shopping.txt");
    focus_editor_end(&mut d, &root, "shopping.txt");
    d.key(KeyCode::Enter)?;
    d.type_text("butter")?;
    assert_eq!(tabs(&d, &root).0, ["ideas.txt", "shopping.txt *"]);
    assert_eq!(status(&d, &root), "shopping.txt: 4 lines, 4 words");
    capture(&mut d, "/work/acceptance-captures/item5-two-tabs-one-modified.png");
    Ok(())
}

// ------------------------------------------------- supplementary checks

/// After closing the active tab while others stay open, the tab the user
/// sees selected, the status line and the target of Ctrl+S must agree.
#[test]
fn supplementary_active_tab_consistency_after_close() -> R {
    let folder = fresh_folder("consistency");
    populate(&folder, &[("a.txt", "aaa"), ("b.txt", "bbb"), ("c.txt", "ccc")]);
    let mut app = app(folder.clone()).headless()?;
    let root = root_handle(&app);
    let mut d = Driver::new(app.runtime_mut());
    settle(&mut d);
    open_by_pointer(&mut d, &root, "a.txt");
    open_by_pointer(&mut d, &root, "b.txt");
    open_by_pointer(&mut d, &root, "c.txt");
    open_by_pointer(&mut d, &root, "a.txt");
    assert_eq!(tabs(&d, &root).1.as_deref(), Some("a.txt"));
    shortcut(&mut d, 'w');
    let (titles, selected) = tabs(&d, &root);
    let shown = status(&d, &root);
    println!("after closing a.txt: tabs {titles:?} selected {selected:?} status {shown:?}");
    let visible_editors: Vec<_> = ["b.txt", "c.txt"]
        .into_iter()
        .filter(|n| record(&d, &root, &editor_path(n)).visible)
        .collect();
    println!("visible editors: {visible_editors:?}");
    assert_eq!(titles, ["b.txt", "c.txt"]);
    let selected = selected.expect("a tab is selected");
    assert!(
        shown.starts_with(&format!("{selected}:")),
        "status {shown:?} describes the selected tab {selected:?}"
    );
    // Typing into the visible editor then Ctrl+S saves that note.
    focus_editor_end(&mut d, &root, &selected);
    d.type_text("+")?;
    shortcut(&mut d, 's');
    let on_disk = std::fs::read_to_string(folder.join(&selected))?;
    println!("{selected} on disk after Ctrl+S: {on_disk:?}");
    assert!(on_disk.ends_with('+'), "Ctrl+S saved the visible tab");
    Ok(())
}

/// The sidebar divider moves with a pointer drag.
#[test]
fn supplementary_divider_drag() -> R {
    let folder = fresh_folder("divider");
    populate(&folder, &[("a.txt", "a")]);
    let mut app = app(folder.clone()).headless()?;
    let root = root_handle(&app);
    let mut d = Driver::new(app.runtime_mut());
    settle(&mut d);
    open_by_pointer(&mut d, &root, "a.txt");
    let files = d.find(&root, FILES)?;
    let docs_path: Vec<&str> = FILES[..4].iter().copied().chain(["documents"]).collect();
    let docs = d.find(&root, &docs_path)?;
    let l = d.runtime().bounds(&files)?.unwrap();
    let r = d.runtime().bounds(&docs)?.unwrap();
    let x = (l.x() + l.width() + r.x()) / 2.0;
    let y = l.y() + 100.0;
    pointer(&mut d, (x, y), PointerKind::Move);
    pointer(&mut d, (x, y), PointerKind::Press(PointerButton::Primary));
    pointer(&mut d, (x + 60.0, y), PointerKind::Move);
    pointer(&mut d, (x + 60.0, y), PointerKind::Release(PointerButton::Primary));
    settle(&mut d);
    let after = d.runtime().bounds(&files)?.unwrap();
    println!("sidebar width {} -> {}", l.width(), after.width());
    assert!((after.width() - l.width() - 60.0).abs() < 2.0);
    Ok(())
}

#[test]
#[ignore = "exploration dump; run explicitly"]
fn explore_structure() -> R {
    let folder = fresh_folder("explore");
    populate(&folder, &[("zeta.txt", "z"), ("alpha.txt", "a")]);
    let mut app = app(folder.clone()).headless()?;
    let root = root_handle(&app);
    let mut d = Driver::new(app.runtime_mut());
    settle(&mut d);
    dump(&d, "start");
    open_by_pointer(&mut d, &root, "alpha.txt");
    dump(&d, "opened alpha");
    let title = by_key(&d, "title/file").into_iter().find(|c| c.visible).unwrap();
    press_at(&mut d, center(title.details.box_model.border_box.unwrap()));
    dump(&d, "file menu open");
    Ok(())
}
