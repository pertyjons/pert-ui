use notes::{Notes, keys};
use pert_ui::prelude::*;
use pert_ui::testing::*;
use std::path::PathBuf;

fn folder(name: &str, files: &[(&str, &str)]) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    for (file, text) in files {
        std::fs::write(dir.join(file), text).unwrap();
    }
    dir
}

/// The title of the selected tab, from the strip's semantics.
fn selected_tab(driver: &Driver<'_>, tabs: &WidgetId) -> Result<Option<String>, TestError> {
    Ok(match driver.semantics(tabs)?.value {
        Some(SemanticValue::Choice { label, .. }) => label,
        _ => None,
    })
}

#[test]
fn lists_and_opens() -> Result<(), Box<dyn std::error::Error>> {
    let dir = folder("listing", &[("b.txt", "bee"), ("a.txt", "ay"), ("c.txt", ""), ("other.md", "x")]);
    let mut app = notes::app(dir).headless()?;
    let (ui, notes) = app.runtime_and_state_mut();
    let root = notes.root.clone();
    let mut driver = Driver::new(ui);
    driver.settle()?;
    let content = driver.find(&root, &Notes::content_path())?;
    let list = driver.find(&content, &[keys::SPLIT, keys::SIDEBAR])?;
    let rows: Vec<String> = driver.address_children(&list)?.into_iter().map(|r| r.segment).collect();
    assert_eq!(rows, ["a.txt", "b.txt", "c.txt"]);
    let tabs = driver.find(&content, &[keys::SPLIT, keys::EDITOR, keys::TABS])?;
    let tab_keys = |driver: &Driver<'_>| -> Result<Vec<String>, TestError> {
        Ok(driver.address_children(&tabs)?.into_iter().map(|r| r.segment).collect())
    };

    // Pointer: open b.txt.
    let b = driver.find(&list, &["b.txt"])?;
    driver.click(&b)?;
    assert_eq!(tab_keys(&driver)?, ["b.txt"]);
    assert_eq!(selected_tab(&driver, &tabs)?.as_deref(), Some("b.txt"));

    // Keyboard: focus the list, Down to the first row, Enter opens a.txt.
    driver.runtime_mut().focus(&list)?;
    driver.key(KeyCode::ArrowDown)?;
    driver.key(KeyCode::Enter)?;
    assert_eq!(tab_keys(&driver)?, ["b.txt", "a.txt"]);
    assert_eq!(selected_tab(&driver, &tabs)?.as_deref(), Some("a.txt"));

    // Reopening b.txt switches to its tab instead of opening another.
    driver.advance(ClickCounter::DEFAULT_INTERVAL * 2)?;
    driver.click(&b)?;
    assert_eq!(tab_keys(&driver)?, ["b.txt", "a.txt"]);
    assert_eq!(selected_tab(&driver, &tabs)?.as_deref(), Some("b.txt"));
    Ok(())
}

/// Handles a test uses on a started application.
struct Ui {
    root: WidgetId,
    list: WidgetId,
    tabs: WidgetId,
    status: WidgetId,
}

fn handles(driver: &Driver<'_>, root: &WidgetId) -> Result<Ui, TestError> {
    let content = driver.find(root, &Notes::content_path())?;
    Ok(Ui {
        root: root.clone(),
        list: driver.find(&content, &[keys::SPLIT, keys::SIDEBAR])?,
        tabs: driver.find(&content, &[keys::SPLIT, keys::EDITOR, keys::TABS])?,
        status: driver.find(&content, &[keys::SPLIT, keys::EDITOR, keys::STATUS])?,
    })
}

/// The status line's text, empty when it shows nothing.
fn status_line(driver: &Driver<'_>, ui: &Ui) -> Result<String, TestError> {
    let label = driver.address_children(&ui.status)?.remove(0).id;
    Ok(driver.semantics(&label)?.name.unwrap_or_default())
}

/// The messages of the visible notifications.
fn toasts(driver: &Driver<'_>, ui: &Ui) -> Result<Vec<String>, TestError> {
    let stack = driver.find(&ui.root, &["toasts"])?;
    Ok(driver.semantics(&stack)?.items.into_iter().filter_map(|item| item.name).collect())
}

/// Chooses the `downs`-th row of the File menu through the menu bar.
fn file_menu(driver: &mut Driver<'_>, downs: usize) -> Result<(), TestError> {
    driver.key(KeyCode::F10)?;
    for _ in 0..downs {
        driver.key(KeyCode::ArrowDown)?;
    }
    driver.key(KeyCode::Enter)
}

#[test]
fn edits_and_saves() -> Result<(), Box<dyn std::error::Error>> {
    let dir = folder("saving", &[("a.txt", "one two"), ("b.txt", "")]);
    let mut app = notes::app(dir.clone()).headless()?;
    let (rt, notes) = app.runtime_and_state_mut();
    let root = notes.root.clone();
    let mut driver = Driver::new(rt);
    driver.settle()?;
    let ui = handles(&driver, &root)?;
    assert_eq!(status_line(&driver, &ui)?, "");

    let a = driver.find(&ui.list, &["a.txt"])?;
    driver.click(&a)?;
    assert_eq!(selected_tab(&driver, &ui.tabs)?.as_deref(), Some("a.txt"));
    assert_eq!(status_line(&driver, &ui)?, "a.txt: 1 lines, 2 words");

    let editor = driver.find(&ui.tabs, &["a.txt", keys::TEXT])?;
    driver.runtime_mut().focus(&editor)?;
    driver.type_text("zero ")?;
    driver.key(KeyCode::Enter)?;
    assert_eq!(selected_tab(&driver, &ui.tabs)?.as_deref(), Some("a.txt *"));
    assert_eq!(status_line(&driver, &ui)?, "a.txt: 2 lines, 3 words");

    // Ctrl+S saves and clears the marker.
    driver.chord(KeyChord::primary('s'), KeyCode::KeyS)?;
    assert_eq!(std::fs::read_to_string(dir.join("a.txt"))?, "zero \none two");
    assert_eq!(selected_tab(&driver, &ui.tabs)?.as_deref(), Some("a.txt"));

    // File › Save does the same.
    driver.runtime_mut().focus(&editor)?;
    driver.type_text("more ")?;
    assert_eq!(selected_tab(&driver, &ui.tabs)?.as_deref(), Some("a.txt *"));
    file_menu(&mut driver, 2)?;
    assert_eq!(std::fs::read_to_string(dir.join("a.txt"))?, "zero \nmore one two");
    assert_eq!(selected_tab(&driver, &ui.tabs)?.as_deref(), Some("a.txt"));
    assert_eq!(status_line(&driver, &ui)?, "a.txt: 2 lines, 4 words");

    // A failed save shows a notification and keeps the marker.
    driver.runtime_mut().focus(&editor)?;
    driver.type_text("x")?;
    std::fs::remove_dir_all(&dir)?;
    assert!(toasts(&driver, &ui)?.is_empty());
    driver.chord(KeyChord::primary('s'), KeyCode::KeyS)?;
    let shown = toasts(&driver, &ui)?;
    assert_eq!(shown.len(), 1, "{shown:?}");
    assert!(shown[0].starts_with("Could not save a.txt"), "{shown:?}");
    assert_eq!(selected_tab(&driver, &ui.tabs)?.as_deref(), Some("a.txt *"));
    Ok(())
}

/// Whether a component is mounted and visible, from inspection.
fn shown(driver: &Driver<'_>, root: &WidgetId, path: &[&str]) -> Result<bool, Box<dyn std::error::Error>> {
    let Some(id) = driver.runtime().find(root, path)? else { return Ok(false) };
    let query = InspectionQuery::components(Selector::id(driver.runtime().inspection_id(&id)?));
    let result = driver.runtime().query(&query);
    Ok(result.components.first().is_some_and(|component| component.visible))
}

/// The New note dialog's content: a dialog keeps its content under `"dialog"`.
const NEW: [&str; 3] = ["new-dialog", "dialog", "new-note"];

#[test]
fn new_note_dialog() -> Result<(), Box<dyn std::error::Error>> {
    let dir = folder("new-note", &[("a.txt", "")]);
    let mut app = notes::app(dir.clone()).headless()?;
    let (rt, notes) = app.runtime_and_state_mut();
    let root = notes.root.clone();
    let mut driver = Driver::new(rt);
    driver.settle()?;
    let ui = handles(&driver, &root)?;
    assert!(!shown(&driver, &root, &NEW)?);

    driver.chord(KeyChord::primary('n'), KeyCode::KeyN)?;
    assert!(shown(&driver, &root, &NEW)?);
    let field = driver.find(&root, &[&NEW[..], &["name-field", keys::NAME]].concat())?;
    let create = driver.find(&root, &[&NEW[..], &["actions", keys::CREATE]].concat())?;
    let error = driver.find(&root, &[&NEW[..], &[keys::NAME_ERROR]].concat())?;
    let files = || -> Vec<String> {
        let mut names: Vec<String> = std::fs::read_dir(&dir).unwrap()
            .map(|entry| entry.unwrap().file_name().into_string().unwrap()).collect();
        names.sort();
        names
    };

    for (typed, explanation) in [
        ("", "Enter a name."),
        ("x/y", "A name must not contain / or \\."),
        ("a", "A note named a.txt already exists."),
        ("a.txt", "A note named a.txt already exists."),
        (".hidden", "A name must not start with a dot."),
    ] {
        driver.replace_text(&field, typed)?;
        driver.advance(ClickCounter::DEFAULT_INTERVAL * 2)?;
        driver.click(&create)?;
        assert!(shown(&driver, &root, &NEW)?, "{typed:?} keeps the dialog open");
        assert_eq!(driver.semantics(&error)?.name.as_deref(), Some(explanation), "{typed:?}");
        assert_eq!(files(), ["a.txt"]);
    }

    // Enter in the field submits a valid name.
    driver.replace_text(&field, "fresh")?;
    driver.key(KeyCode::Enter)?;
    assert!(!shown(&driver, &root, &NEW)?);
    assert_eq!(files(), ["a.txt", "fresh.txt"]);
    let rows: Vec<String> = driver.address_children(&ui.list)?.into_iter().map(|r| r.segment).collect();
    assert_eq!(rows, ["a.txt", "fresh.txt"]);
    assert_eq!(selected_tab(&driver, &ui.tabs)?.as_deref(), Some("fresh.txt"));
    assert_eq!(status_line(&driver, &ui)?, "fresh.txt: 1 lines, 0 words");

    // File › New note opens the same dialog, with the earlier text gone.
    file_menu(&mut driver, 1)?;
    assert!(shown(&driver, &root, &NEW)?);
    assert_eq!(driver.semantics(&field)?.value, Some(SemanticValue::Text(String::new())));
    assert_eq!(driver.semantics(&error)?.name, None);
    driver.key(KeyCode::Escape)?;
    assert!(!shown(&driver, &root, &NEW)?);
    Ok(())
}

const CLOSE: [&str; 4] = ["new-dialog", "close-dialog", "dialog", "close-tab"];

#[test]
fn closes_tabs() -> Result<(), Box<dyn std::error::Error>> {
    let dir = folder("closing", &[("a.txt", "alpha"), ("b.txt", "beta"), ("c.txt", "gamma")]);
    let mut app = notes::app(dir.clone()).headless()?;
    let (rt, notes) = app.runtime_and_state_mut();
    let root = notes.root.clone();
    let mut driver = Driver::new(rt);
    driver.settle()?;
    let ui = handles(&driver, &root)?;
    let tab_keys = |driver: &Driver<'_>| -> Result<Vec<String>, TestError> {
        Ok(driver.address_children(&ui.tabs)?.into_iter().map(|r| r.segment).collect())
    };
    let read = |name: &str| std::fs::read_to_string(dir.join(name)).unwrap();
    let button = |driver: &Driver<'_>, key: &str| driver.find(&root, &[&CLOSE[..], &["actions", key]].concat());
    for name in ["a.txt", "b.txt", "c.txt"] {
        let row = driver.find(&ui.list, &[name])?;
        driver.advance(ClickCounter::DEFAULT_INTERVAL * 2)?;
        driver.click(&row)?;
    }
    assert_eq!(tab_keys(&driver)?, ["a.txt", "b.txt", "c.txt"]);

    // An unmodified tab closes at once, without a dialog.
    driver.chord(KeyChord::primary('w'), KeyCode::KeyW)?;
    assert!(!shown(&driver, &root, &CLOSE)?);
    assert_eq!(tab_keys(&driver)?, ["a.txt", "b.txt"]);
    assert_eq!(selected_tab(&driver, &ui.tabs)?.as_deref(), Some("b.txt"));

    // Cancel keeps the modified tab and its text.
    let b = driver.find(&ui.tabs, &["b.txt", keys::TEXT])?;
    driver.runtime_mut().focus(&b)?;
    driver.type_text("new ")?;
    driver.chord(KeyChord::primary('w'), KeyCode::KeyW)?;
    assert!(shown(&driver, &root, &CLOSE)?);
    let message = driver.find(&root, &[&CLOSE[..], &["message"]].concat())?;
    assert_eq!(driver.semantics(&message)?.name.as_deref(), Some("b.txt has unsaved changes."));
    let cancel = button(&driver, keys::CANCEL)?;
    driver.click(&cancel)?;
    assert!(!shown(&driver, &root, &CLOSE)?);
    assert_eq!(tab_keys(&driver)?, ["a.txt", "b.txt"]);
    assert_eq!(selected_tab(&driver, &ui.tabs)?.as_deref(), Some("b.txt *"));
    assert_eq!(driver.semantics(&b)?.value, Some(SemanticValue::Text("new beta".into())));
    assert_eq!(read("b.txt"), "beta");

    // Discard closes it without writing.
    driver.chord(KeyChord::primary('w'), KeyCode::KeyW)?;
    assert!(shown(&driver, &root, &CLOSE)?);
    let discard = button(&driver, keys::DISCARD)?;
    driver.click(&discard)?;
    assert!(!shown(&driver, &root, &CLOSE)?);
    assert_eq!(tab_keys(&driver)?, ["a.txt"]);
    assert_eq!(read("b.txt"), "beta");

    // Save, chosen after File › Close tab, writes the file and closes the tab.
    let a = driver.find(&ui.tabs, &["a.txt", keys::TEXT])?;
    driver.runtime_mut().focus(&a)?;
    driver.type_text("big ")?;
    assert_eq!(selected_tab(&driver, &ui.tabs)?.as_deref(), Some("a.txt *"));
    file_menu(&mut driver, 3)?;
    assert!(shown(&driver, &root, &CLOSE)?);
    let save = button(&driver, keys::SAVE)?;
    driver.click(&save)?;
    assert!(!shown(&driver, &root, &CLOSE)?);
    assert!(tab_keys(&driver)?.is_empty());
    assert_eq!(read("a.txt"), "big alpha");
    assert_eq!(status_line(&driver, &ui)?, "");
    assert_eq!(selected_tab(&driver, &ui.tabs)?, None);

    // A failed save from the dialog keeps the tab open and modified.
    let c = driver.find(&ui.list, &["c.txt"])?;
    driver.click(&c)?;
    let c = driver.find(&ui.tabs, &["c.txt", keys::TEXT])?;
    driver.runtime_mut().focus(&c)?;
    driver.type_text("lost ")?;
    std::fs::remove_dir_all(&dir)?;
    driver.chord(KeyChord::primary('w'), KeyCode::KeyW)?;
    let save = button(&driver, keys::SAVE)?;
    driver.click(&save)?;
    assert!(!shown(&driver, &root, &CLOSE)?);
    assert_eq!(tab_keys(&driver)?, ["c.txt"]);
    assert_eq!(selected_tab(&driver, &ui.tabs)?.as_deref(), Some("c.txt *"));
    assert_eq!(toasts(&driver, &ui)?.len(), 1);
    Ok(())
}

/// Writes `notes-capture.png` beside Cargo.toml: two notes open, one modified.
#[test]
fn cpu_capture() -> Result<(), Box<dyn std::error::Error>> {
    let dir = folder("capture", &[
        ("groceries.txt", "milk\neggs\nbread"),
        ("ideas.txt", "A notes editor\nwith tabs"),
        ("todo.txt", "write tests"),
        ("readme.md", "not a note"),
    ]);
    let mut app = notes::app(dir).headless()?;
    let (rt, notes) = app.runtime_and_state_mut();
    let root = notes.root.clone();
    let mut driver = Driver::new(rt);
    driver.settle()?;
    let ui = handles(&driver, &root)?;
    for name in ["groceries.txt", "ideas.txt"] {
        let row = driver.find(&ui.list, &[name])?;
        driver.advance(ClickCounter::DEFAULT_INTERVAL * 2)?;
        driver.click(&row)?;
    }
    let editor = driver.find(&ui.tabs, &["ideas.txt", keys::TEXT])?;
    driver.runtime_mut().focus(&editor)?;
    driver.type_text("Draft: ")?;
    assert_eq!(selected_tab(&driver, &ui.tabs)?.as_deref(), Some("ideas.txt *"));
    driver.clear_focus()?;

    driver.runtime().set_capture_support(true);
    let window = driver.runtime().primary_window();
    let pending = driver.runtime().request_capture(CaptureRequest::new(CaptureTarget::window(&window)));
    driver.settle()?;
    CpuRenderer::new().serve_captures(driver.runtime_mut());
    let capture = pending.try_reply()?.value?;
    let file = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("notes-capture.png");
    std::fs::write(&file, capture.image.encode_png())?;
    assert_eq!((capture.image.width(), capture.image.height()), (960, 640));
    let issues = driver.runtime().validation_report().issues;
    assert!(issues.is_empty(), "{issues:#?}");
    Ok(())
}

#[test]
fn divider_resizes_sidebar() -> Result<(), Box<dyn std::error::Error>> {
    let dir = folder("divider", &[("a.txt", "")]);
    let mut app = notes::app(dir).headless()?;
    let (rt, notes) = app.runtime_and_state_mut();
    let root = notes.root.clone();
    let mut driver = Driver::new(rt);
    driver.settle()?;
    let ui = handles(&driver, &root)?;
    let width = |driver: &Driver<'_>| -> Result<f32, Box<dyn std::error::Error>> {
        Ok(driver.runtime().bounds(&ui.list)?.ok_or("laid out")?.width())
    };
    let before = width(&driver)?;
    let bounds = driver.runtime().bounds(&ui.list)?.ok_or("laid out")?;
    let y = bounds.y() + bounds.height() / 2.0;
    let x = bounds.x() + bounds.width() + 4.0; // the middle of the 8-pixel divider
    for (x, kind) in [
        (x, PointerKind::Press(PointerButton::Primary)),
        (x + 50.0, PointerKind::Move),
        (x + 100.0, PointerKind::Move),
        (x + 100.0, PointerKind::Release(PointerButton::Primary)),
    ] {
        driver.runtime_mut().pointer_input(PointerEvent::new(Point::new(x, y)?, kind))?;
        driver.settle()?;
    }
    assert_eq!(width(&driver)?, before + 100.0);
    Ok(())
}
