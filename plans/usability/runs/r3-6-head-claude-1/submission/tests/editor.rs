//! Headless checks of the notes editor, driven through `pert_ui::testing`.

use notes_editor::Notes;
use pert_ui::prelude::*;
use pert_ui::testing::*;
use pert_ui::DialogStatus;
use std::path::{Path, PathBuf};

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// A fresh notes folder holding two notes and a file that is not a note.
fn folder(name: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("beta.txt"), "beta text\nline two").unwrap();
    std::fs::write(dir.join("alpha.txt"), "alpha").unwrap();
    std::fs::write(dir.join("gamma.txt"), "").unwrap();
    std::fs::write(dir.join("readme.md"), "not a note").unwrap();
    dir
}

const MAIN: [&str; 4] = ["new-dialog", "close-dialog", "menu", "main"];

fn path<'a>(tail: &[&'a str]) -> Vec<&'a str> {
    MAIN.iter().copied().chain(tail.iter().copied()).collect()
}

fn list(driver: &Driver<'_>, notes: &Notes) -> Result<WidgetId, TestError> {
    driver.find(&notes.root, &path(&["split", "notes"]))
}

fn tabs(driver: &Driver<'_>, notes: &Notes) -> Result<WidgetId, TestError> {
    driver.find(&notes.root, &path(&["split", "tabs"]))
}

/// The listed note names, in order.
fn listed(driver: &Driver<'_>, notes: &Notes) -> Result<Vec<String>, TestError> {
    let list = list(driver, notes)?;
    driver
        .address_children(&list)?
        .iter()
        .map(|row| Ok(driver.semantics(&row.id)?.name.unwrap_or_default()))
        .collect()
}

/// The tab titles, in order, and the selected one.
fn tab_titles(driver: &Driver<'_>, notes: &Notes) -> Result<(Vec<String>, Option<String>), TestError> {
    let items = driver.semantics(&tabs(driver, notes)?)?.items;
    let titles = items.iter().map(|item| item.name.clone().unwrap_or_default()).collect();
    let selected = items.iter().find(|item| item.selected).and_then(|item| item.name.clone());
    Ok((titles, selected))
}

/// The status line's text, empty when it shows none.
fn status(driver: &Driver<'_>, notes: &Notes) -> Result<String, TestError> {
    let status = driver.find(&notes.root, &path(&["status"]))?;
    let shown = driver.address_children(&status)?;
    Ok(match shown.first() {
        Some(label) => driver.semantics(&label.id)?.name.unwrap_or_default(),
        None => String::new(),
    })
}

/// The messages of the visible notifications.
fn toasts(driver: &Driver<'_>, notes: &Notes) -> Result<Vec<String>, TestError> {
    let stack = driver.find(&notes.root, &["toasts"])?;
    Ok(driver.semantics(&stack)?.items.into_iter().filter_map(|item| item.name).collect())
}

fn click_note(driver: &mut Driver<'_>, notes: &Notes, name: &str) -> Result<(), TestError> {
    let list = list(driver, notes)?;
    let row = driver.find(&list, &[name])?;
    driver.click(&row)?;
    // Two clicks on one point within the double-click interval would count as one gesture.
    driver.advance(ClickCounter::DEFAULT_INTERVAL + std::time::Duration::from_millis(1))?;
    Ok(())
}

fn editor(driver: &Driver<'_>, notes: &Notes, name: &str) -> Result<WidgetId, TestError> {
    driver.find(&tabs(driver, notes)?, &[name, "editor"])
}

fn read(dir: &Path, name: &str) -> String {
    std::fs::read_to_string(dir.join(name)).unwrap()
}

/// Opens File with the pointer and chooses row `index` (0 New note, 1 Save, 2 Close tab).
fn menu(driver: &mut Driver<'_>, notes: &Notes, index: usize) -> Result<(), TestError> {
    let bar = driver.find(&notes.root, &["new-dialog", "close-dialog", "menu"])?;
    // The titles and rows are parts of the bar, not addressed children: walk the structure.
    let file = driver.runtime().find_descendant(&bar, "title/file")?.expect("the File title");
    driver.click(&file)?;
    let items = driver.runtime().find_descendant(&file, "items")?.expect("the File rows");
    let names: Vec<_> = driver.semantics(&items)?.items.into_iter().map(|item| item.name).collect();
    assert_eq!(names, [Some("New note".into()), Some("Save".into()), Some("Close tab".into())]);
    assert_eq!(driver.semantics(&file)?.expanded, Some(true));
    // Menu rows report no bounds, so `click_item` refuses them: the open menu's keys choose the row.
    assert_eq!(driver.semantics(&items)?.value.and_then(|value| match value {
        SemanticValue::Choice { selected, .. } => selected,
        _ => None,
    }), Some(0));
    for _ in 0..index {
        driver.key(KeyCode::ArrowDown)?;
    }
    driver.key(KeyCode::Enter)?;
    driver.settle()?;
    Ok(())
}

#[test]
fn lists_notes_and_opens_tabs() -> TestResult {
    let dir = folder("open");
    let mut app = notes_editor::app(dir).headless()?;
    let (ui, notes) = app.runtime_and_state_mut();
    let notes = notes.clone();
    let mut driver = Driver::new(ui);
    driver.settle()?;

    assert_eq!(listed(&driver, &notes)?, ["alpha.txt", "beta.txt", "gamma.txt"]);
    assert_eq!(tab_titles(&driver, &notes)?.0, Vec::<String>::new());
    assert_eq!(status(&driver, &notes)?, "");

    // Open two notes with the pointer.
    click_note(&mut driver, &notes, "beta.txt")?;
    click_note(&mut driver, &notes, "alpha.txt")?;
    assert_eq!(tab_titles(&driver, &notes)?, (vec!["beta.txt".into(), "alpha.txt".into()], Some("alpha.txt".into())));
    let beta = editor(&driver, &notes, "beta.txt")?;
    assert_eq!(driver.semantics(&beta)?.value, Some(SemanticValue::Text("beta text\nline two".into())));
    assert_eq!(status(&driver, &notes)?, "alpha.txt: 1 lines, 1 words");

    // Opening an open note switches to its tab.
    click_note(&mut driver, &notes, "beta.txt")?;
    assert_eq!(tab_titles(&driver, &notes)?, (vec!["beta.txt".into(), "alpha.txt".into()], Some("beta.txt".into())));
    assert_eq!(status(&driver, &notes)?, "beta.txt: 2 lines, 4 words");

    // The keyboard: arrows move in the list, Enter opens.
    let list = list(&driver, &notes)?;
    driver.runtime_mut().focus(&list)?;
    driver.key(KeyCode::ArrowDown)?; // alpha
    driver.key(KeyCode::ArrowDown)?; // beta
    driver.key(KeyCode::ArrowDown)?; // gamma
    assert_eq!(tab_titles(&driver, &notes)?.0.len(), 2, "arrows only move");
    driver.key(KeyCode::Enter)?;
    assert_eq!(
        tab_titles(&driver, &notes)?,
        (vec!["beta.txt".into(), "alpha.txt".into(), "gamma.txt".into()], Some("gamma.txt".into()))
    );
    driver.key(KeyCode::ArrowUp)?; // beta
    driver.key(KeyCode::Enter)?;
    assert_eq!(tab_titles(&driver, &notes)?.1.as_deref(), Some("beta.txt"));
    assert_eq!(tab_titles(&driver, &notes)?.0.len(), 3);

    // The divider resizes the sidebar (its arrow keys move it as a drag does).
    let split = driver.find(&notes.root, &path(&["split"]))?;
    let width = |driver: &Driver<'_>| -> Result<f32, Box<dyn std::error::Error>> {
        let sidebar = driver.find(&notes.root, &path(&["split", "notes"]))?;
        Ok(driver.runtime().bounds(&sidebar)?.ok_or("laid out")?.width())
    };
    let before = width(&driver)?;
    driver.runtime_mut().focus(&split)?;
    driver.key(KeyCode::ArrowRight)?;
    driver.key(KeyCode::ArrowRight)?;
    assert_eq!(width(&driver)?, before + 16.0);
    Ok(())
}

#[test]
fn edits_and_saves() -> TestResult {
    let dir = folder("save");
    let mut app = notes_editor::app(dir.clone()).headless()?;
    let (ui, notes) = app.runtime_and_state_mut();
    let notes = notes.clone();
    let mut driver = Driver::new(ui);
    driver.settle()?;
    click_note(&mut driver, &notes, "alpha.txt")?;
    click_note(&mut driver, &notes, "beta.txt")?;
    assert_eq!(status(&driver, &notes)?, "beta.txt: 2 lines, 4 words");

    // Typing marks the tab and the status line follows.
    let beta = editor(&driver, &notes, "beta.txt")?;
    driver.runtime_mut().focus(&beta)?;
    driver.type_text("one two three\n")?;
    assert_eq!(tab_titles(&driver, &notes)?.0, ["alpha.txt", "beta.txt •"]);
    assert_eq!(status(&driver, &notes)?, "beta.txt: 3 lines, 7 words");

    // Ctrl+S saves the active tab and clears its marker.
    driver.chord(KeyChord::primary('s'), KeyCode::KeyS)?;
    assert_eq!(read(&dir, "beta.txt"), "one two three\nbeta text\nline two");
    assert_eq!(tab_titles(&driver, &notes)?.0, ["alpha.txt", "beta.txt"]);
    assert_eq!(read(&dir, "alpha.txt"), "alpha");

    // File › Save does the same.
    driver.runtime_mut().focus(&beta)?;
    driver.type_text("four ")?;
    assert_eq!(tab_titles(&driver, &notes)?.0, ["alpha.txt", "beta.txt •"]);
    menu(&mut driver, &notes, 1)?;
    driver.settle()?;
    assert_eq!(read(&dir, "beta.txt"), "one two three\nfour beta text\nline two");
    assert_eq!(tab_titles(&driver, &notes)?.0, ["alpha.txt", "beta.txt"]);
    assert_eq!(status(&driver, &notes)?, "beta.txt: 3 lines, 8 words");

    // A failed save is reported and keeps the marker.
    driver.runtime_mut().focus(&beta)?;
    driver.type_text("lost ")?;
    std::fs::remove_dir_all(&dir)?;
    driver.chord(KeyChord::primary('s'), KeyCode::KeyS)?;
    let shown = toasts(&driver, &notes)?;
    assert_eq!(shown.len(), 1, "{shown:?}");
    assert!(shown[0].starts_with("Could not save beta.txt"), "{shown:?}");
    assert_eq!(tab_titles(&driver, &notes)?.0, ["alpha.txt", "beta.txt •"]);
    Ok(())
}

fn dialog_part(driver: &Driver<'_>, notes: &Notes, key: &str) -> Result<WidgetId, TestError> {
    driver.find(&notes.root, &["new-dialog", "dialog", "new-form", key])
}

#[test]
fn new_note_dialog() -> TestResult {
    let dir = folder("new");
    let mut app = notes_editor::app(dir.clone()).headless()?;
    let (ui, notes) = app.runtime_and_state_mut();
    let notes = notes.clone();
    let mut driver = Driver::new(ui);
    driver.settle()?;
    let files = || -> Vec<String> {
        let mut names: Vec<String> = std::fs::read_dir(&dir).unwrap()
            .map(|entry| entry.unwrap().file_name().into_string().unwrap()).collect();
        names.sort();
        names
    };
    let before = files();

    // Ctrl+N asks for a name.
    driver.chord(KeyChord::primary('n'), KeyCode::KeyN)?;
    assert_eq!(notes.new_dialog.status().get()?, DialogStatus::Open);
    let name = dialog_part(&driver, &notes, "name")?;
    let create = driver.find(&notes.root, &["new-dialog", "dialog", "new-form", "actions", "create"])?;
    let error = dialog_part(&driver, &notes, "error")?;
    let explanation = |driver: &Driver<'_>| -> Result<Option<String>, TestError> {
        Ok(driver.semantics(&error)?.name)
    };
    assert_eq!(explanation(&driver)?, None);

    // An empty name is refused.
    driver.click(&create)?;
    assert_eq!(explanation(&driver)?.as_deref(), Some("Enter a name."));
    assert_eq!(notes.new_dialog.status().get()?, DialogStatus::Open);

    // A name with a slash is refused.
    driver.replace_text(&name, "a/b")?;
    driver.click(&create)?;
    assert_eq!(explanation(&driver)?.as_deref(), Some("A name cannot contain / or \\."));
    assert_eq!(notes.new_dialog.status().get()?, DialogStatus::Open);

    // An existing note's name is refused, with or without the extension.
    driver.replace_text(&name, "alpha")?;
    driver.click(&create)?;
    assert_eq!(explanation(&driver)?.as_deref(), Some("A note named alpha.txt already exists."));
    driver.replace_text(&name, "beta.txt")?;
    driver.click(&create)?;
    assert_eq!(explanation(&driver)?.as_deref(), Some("A note named beta.txt already exists."));
    assert_eq!(files(), before, "nothing was created");
    assert_eq!(read(&dir, "alpha.txt"), "alpha");

    // A valid name creates, lists and opens the note.
    driver.replace_text(&name, "delta")?;
    driver.click(&create)?;
    assert_ne!(notes.new_dialog.status().get()?, DialogStatus::Open);
    assert_eq!(read(&dir, "delta.txt"), "");
    assert_eq!(listed(&driver, &notes)?, ["alpha.txt", "beta.txt", "delta.txt", "gamma.txt"]);
    assert_eq!(tab_titles(&driver, &notes)?, (vec!["delta.txt".into()], Some("delta.txt".into())));
    assert_eq!(status(&driver, &notes)?, "delta.txt: 1 lines, 0 words");

    // File › New note opens the dialog again, cleared.
    menu(&mut driver, &notes, 0)?;
    assert_eq!(notes.new_dialog.status().get()?, DialogStatus::Open);
    assert_eq!(explanation(&driver)?, None);
    assert_eq!(driver.semantics(&name)?.value, Some(SemanticValue::Text(String::new())));
    let cancel = driver.find(&notes.root, &["new-dialog", "dialog", "new-form", "actions", "cancel"])?;
    driver.click(&cancel)?;
    assert_ne!(notes.new_dialog.status().get()?, DialogStatus::Open);
    Ok(())
}

fn close_button(driver: &Driver<'_>, notes: &Notes, key: &str) -> Result<WidgetId, TestError> {
    driver.find(&notes.root, &["new-dialog", "close-dialog", "dialog", "close-form", "actions", key])
}

#[test]
fn closing_tabs() -> TestResult {
    let dir = folder("close");
    let mut app = notes_editor::app(dir.clone()).headless()?;
    let (ui, notes) = app.runtime_and_state_mut();
    let notes = notes.clone();
    let mut driver = Driver::new(ui);
    driver.settle()?;
    click_note(&mut driver, &notes, "alpha.txt")?;
    click_note(&mut driver, &notes, "beta.txt")?;
    let beta = editor(&driver, &notes, "beta.txt")?;
    driver.runtime_mut().focus(&beta)?;
    driver.type_text("changed ")?;
    assert_eq!(tab_titles(&driver, &notes)?.0, ["alpha.txt", "beta.txt •"]);
    let message = driver.find(&notes.root, &["new-dialog", "close-dialog", "dialog", "close-form", "message"])?;

    // Cancel keeps the tab and its text.
    driver.chord(KeyChord::primary('w'), KeyCode::KeyW)?;
    assert_eq!(notes.close_dialog.status().get()?, DialogStatus::Open);
    assert_eq!(
        driver.semantics(&message)?.name.as_deref(),
        Some("beta.txt has unsaved changes. Save them before closing?")
    );
    let cancel = close_button(&driver, &notes, "cancel")?;
    driver.click(&cancel)?;
    assert_ne!(notes.close_dialog.status().get()?, DialogStatus::Open);
    assert_eq!(tab_titles(&driver, &notes)?, (vec!["alpha.txt".into(), "beta.txt •".into()], Some("beta.txt •".into())));
    let beta = editor(&driver, &notes, "beta.txt")?;
    assert_eq!(driver.semantics(&beta)?.value, Some(SemanticValue::Text("changed beta text\nline two".into())));
    assert_eq!(read(&dir, "beta.txt"), "beta text\nline two");

    // Escape cancels too, and the dialog opens again afterwards.
    driver.chord(KeyChord::primary('w'), KeyCode::KeyW)?;
    assert_eq!(notes.close_dialog.status().get()?, DialogStatus::Open);
    driver.key(KeyCode::Escape)?;
    assert_ne!(notes.close_dialog.status().get()?, DialogStatus::Open);
    assert_eq!(tab_titles(&driver, &notes)?.0, ["alpha.txt", "beta.txt •"]);

    // Discard closes without writing; File › Close tab is the same action.
    menu(&mut driver, &notes, 2)?;
    assert_eq!(notes.close_dialog.status().get()?, DialogStatus::Open);
    let discard = close_button(&driver, &notes, "discard")?;
    driver.click(&discard)?;
    assert_ne!(notes.close_dialog.status().get()?, DialogStatus::Open);
    assert_eq!(tab_titles(&driver, &notes)?, (vec!["alpha.txt".into()], Some("alpha.txt".into())));
    assert_eq!(read(&dir, "beta.txt"), "beta text\nline two");
    assert_eq!(status(&driver, &notes)?, "alpha.txt: 1 lines, 1 words");

    // Save writes, then closes.
    click_note(&mut driver, &notes, "beta.txt")?;
    let beta = editor(&driver, &notes, "beta.txt")?;
    assert_eq!(driver.semantics(&beta)?.value, Some(SemanticValue::Text("beta text\nline two".into())));
    driver.runtime_mut().focus(&beta)?;
    driver.type_text("kept ")?;
    driver.chord(KeyChord::primary('w'), KeyCode::KeyW)?;
    let save = close_button(&driver, &notes, "save")?;
    driver.click(&save)?;
    assert_ne!(notes.close_dialog.status().get()?, DialogStatus::Open);
    assert_eq!(tab_titles(&driver, &notes)?.0, ["alpha.txt"]);
    assert_eq!(read(&dir, "beta.txt"), "kept beta text\nline two");

    // A failed save in the dialog keeps the tab open.
    click_note(&mut driver, &notes, "gamma.txt")?;
    let gamma = editor(&driver, &notes, "gamma.txt")?;
    driver.runtime_mut().focus(&gamma)?;
    driver.type_text("unsaved")?;
    let moved = dir.with_extension("moved");
    let _ = std::fs::remove_dir_all(&moved);
    std::fs::rename(&dir, &moved)?;
    driver.chord(KeyChord::primary('w'), KeyCode::KeyW)?;
    let save = close_button(&driver, &notes, "save")?;
    driver.click(&save)?;
    assert_eq!(tab_titles(&driver, &notes)?.0, ["alpha.txt", "gamma.txt •"]);
    assert!(toasts(&driver, &notes)?.iter().any(|toast| toast.starts_with("Could not save gamma.txt")));
    std::fs::rename(&moved, &dir)?;

    // An unmodified tab closes at once, without a dialog.
    // Tab titles report no bounds, so `click_item` cannot press one; reopening switches.
    click_note(&mut driver, &notes, "alpha.txt")?;
    assert_eq!(tab_titles(&driver, &notes)?.1.as_deref(), Some("alpha.txt"));
    driver.chord(KeyChord::primary('w'), KeyCode::KeyW)?;
    assert_eq!(notes.close_dialog.status().get()?, DialogStatus::Idle);
    assert_eq!(tab_titles(&driver, &notes)?, (vec!["gamma.txt •".into()], Some("gamma.txt •".into())));

    // Discarding the last tab empties the status line.
    driver.chord(KeyChord::primary('w'), KeyCode::KeyW)?;
    let discard = close_button(&driver, &notes, "discard")?;
    driver.click(&discard)?;
    assert_eq!(tab_titles(&driver, &notes)?.0, Vec::<String>::new());
    assert_eq!(status(&driver, &notes)?, "");
    assert_eq!(read(&dir, "gamma.txt"), "");
    Ok(())
}

#[test]
fn capture_two_notes_one_modified() -> TestResult {
    let dir = folder("capture");
    let mut app = notes_editor::app(dir).headless()?;
    let (ui, notes) = app.runtime_and_state_mut();
    let notes = notes.clone();
    let mut driver = Driver::new(ui);
    driver.settle()?;
    click_note(&mut driver, &notes, "alpha.txt")?;
    click_note(&mut driver, &notes, "beta.txt")?;
    let beta = editor(&driver, &notes, "beta.txt")?;
    driver.runtime_mut().focus(&beta)?;
    driver.type_text("A modified note\n")?;
    driver.clear_focus()?;
    assert_eq!(tab_titles(&driver, &notes)?.0, ["alpha.txt", "beta.txt •"]);

    driver.runtime().set_capture_support(true);
    let pending = driver.runtime().request_capture(CaptureRequest::new(
        CaptureTarget::window(&driver.runtime().primary_window()),
    ));
    driver.settle()?;
    CpuRenderer::new().serve_captures(driver.runtime_mut());
    let capture = pending.try_reply()?.value?;
    let image: &FrameImage = &capture.image;
    assert_eq!((image.width(), image.height()), (960, 640));
    let file = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("capture.png");
    std::fs::write(&file, image.encode_png())?;
    assert!(std::fs::read(&file)?.starts_with(b"\x89PNG"));
    Ok(())
}
