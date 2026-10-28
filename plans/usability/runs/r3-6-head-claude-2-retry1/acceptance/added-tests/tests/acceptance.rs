//! Independent acceptance checks for task 6 (notes editor), added by the
//! acceptance verifier in the acceptance copy only. They drive the
//! submitted application through pert-ui's public input facilities
//! (Driver clicks, typing, keys and chords, raw pointer input on the menu
//! bar) and read results from the inspection snapshot, the CPU capture and
//! the files on disk. No application code is called besides `notes::app`
//! and the key-path constants the application publishes.
use notes::Notes;
use pert_ui::prelude::*;
use pert_ui::testing::*;
use std::path::{Path, PathBuf};

type R<T = ()> = Result<T, Box<dyn std::error::Error>>;

/// A fresh folder; `files` are created in the order given.
fn folder(name: &str, files: &[(&str, &str)]) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("acceptance-{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    for (file, text) in files {
        std::fs::write(dir.join(file), text).unwrap();
    }
    dir
}

fn disk(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .collect();
    names.sort();
    names
}

fn base() -> Vec<String> {
    ["app"].iter().chain(Notes::content_path().iter()).map(|s| s.to_string()).collect()
}

fn under(prefix: &[String], rest: &[&str]) -> Vec<String> {
    prefix.iter().cloned().chain(rest.iter().map(|s| s.to_string())).collect()
}

/// The visible inspection record at exactly `path`.
fn component(driver: &Driver<'_>, path: &[String]) -> Option<pert_ui::ComponentInspection> {
    driver
        .runtime()
        .inspect()
        .components
        .into_iter()
        .find(|c| c.visible && c.path.as_deref() == Some(path))
}

fn semantics(driver: &Driver<'_>, path: &[String]) -> R<Semantics> {
    Ok(component(driver, path)
        .ok_or_else(|| format!("no visible component at {path:?}"))?
        .details
        .semantics
        .ok_or("no semantics")?)
}

/// Sidebar rows from inspection: visible children of the list, top to bottom.
fn listed(driver: &Driver<'_>) -> R<Vec<String>> {
    let list = under(&base(), &["split", "notes"]);
    let mut rows: Vec<(f32, String)> = driver
        .runtime()
        .inspect()
        .components
        .into_iter()
        .filter(|c| c.visible)
        .filter_map(|c| {
            let path = c.path.clone()?;
            (path.len() == list.len() + 1 && path.starts_with(&list)).then_some(())?;
            let y = c.details.box_model.border_box?[1];
            Some((y, c.details.semantics?.name?))
        })
        .collect();
    rows.sort_by(|a, b| a.0.total_cmp(&b.0));
    let count = match semantics(driver, &list)?.value {
        Some(SemanticValue::Choice { count, .. }) => count,
        other => return Err(format!("list value {other:?}").into()),
    };
    assert_eq!(count, rows.len(), "list count and visible rows");
    Ok(rows.into_iter().map(|r| r.1).collect())
}

/// (tab count, selected tab title) from the tab strip's inspection.
fn tabs(driver: &Driver<'_>) -> R<(usize, Option<String>)> {
    match semantics(driver, &under(&base(), &["split", "editor", "tabs"]))?.value {
        Some(SemanticValue::Choice { count, label, .. }) => Ok((count, label)),
        other => Err(format!("tab value {other:?}").into()),
    }
}

/// Tab titles, from the strip's semantic items.
fn tab_titles(driver: &Driver<'_>) -> R<Vec<String>> {
    Ok(semantics(driver, &under(&base(), &["split", "editor", "tabs"]))?
        .items
        .into_iter()
        .filter_map(|i| i.name)
        .collect())
}

/// The status line's text from inspection (empty when it shows nothing).
fn status(driver: &Driver<'_>) -> R<String> {
    let prefix = under(&base(), &["split", "editor", "status"]);
    let labels: Vec<String> = driver
        .runtime()
        .inspect()
        .components
        .into_iter()
        .filter(|c| c.visible && c.path.as_ref().is_some_and(|p| p.starts_with(&prefix) && p.len() > prefix.len()))
        .filter_map(|c| c.details.semantics)
        .filter(|s| s.role == SemanticRole::Label)
        .map(|s| s.name.unwrap_or_default())
        .collect();
    assert!(labels.len() <= 1, "{labels:?}");
    Ok(labels.into_iter().next().unwrap_or_default())
}

/// The text area's text in the tab for `name`, from inspection.
fn editor_text(driver: &Driver<'_>, name: &str) -> R<String> {
    match semantics(driver, &under(&base(), &["split", "editor", "tabs", name, "text"]))?.value {
        Some(SemanticValue::Text(text)) => Ok(text),
        other => Err(format!("editor value {other:?}").into()),
    }
}

/// (count, messages) of the Notifications-role stack.
fn notifications(driver: &Driver<'_>) -> R<(f64, Vec<String>)> {
    let found: Vec<Semantics> = driver
        .runtime()
        .inspect()
        .components
        .into_iter()
        .filter_map(|c| c.details.semantics)
        .filter(|s| s.role == SemanticRole::Notifications)
        .collect();
    assert_eq!(found.len(), 1, "one notification stack");
    let s = &found[0];
    let count = match s.value {
        Some(SemanticValue::Number { value, .. }) => value,
        ref other => return Err(format!("stack value {other:?}").into()),
    };
    Ok((count, s.items.iter().filter_map(|i| i.name.clone()).collect()))
}

/// Paths of every visible component that belongs to an open dialog's
/// content (a dialog presents its content under its `dialog` part).
fn open_dialog_content(driver: &Driver<'_>) -> Vec<Vec<String>> {
    driver
        .runtime()
        .inspect()
        .components
        .into_iter()
        .filter(|c| c.visible)
        .filter_map(|c| c.path)
        .filter(|p| p.iter().any(|s| s == "dialog"))
        .collect()
}

fn new_dialog() -> Vec<String> {
    ["app", "new-dialog", "dialog", "new-note"].iter().map(|s| s.to_string()).collect()
}

fn close_dialog() -> Vec<String> {
    ["app", "new-dialog", "close-dialog", "dialog", "close-tab"].iter().map(|s| s.to_string()).collect()
}

fn press_at(driver: &mut Driver<'_>, x: f32, y: f32) -> R {
    driver.advance(ClickCounter::DEFAULT_INTERVAL * 2)?;
    let at = Point::new(x, y)?;
    driver.runtime_mut().pointer_input(PointerEvent::new(at, PointerKind::Move))?;
    driver.runtime_mut().pointer_input(PointerEvent::new(at, PointerKind::Press(PointerButton::Primary)))?;
    driver.runtime_mut().pointer_input(PointerEvent::new(at, PointerKind::Release(PointerButton::Primary)))?;
    driver.settle()?;
    Ok(())
}

/// Clicks the File title in the menu bar, then the row named `row` in the
/// open menu. Menu rows carry no item bounds in this library version, so
/// the pointer moves down the open menu until the menu's highlighted row
/// (its inspection value) is `row`, and presses there.
fn menu(driver: &mut Driver<'_>, row: &str) -> R {
    let find = |driver: &Driver<'_>, key: &str, name: Option<&str>| {
        driver.runtime().inspect().components.into_iter().find(|c| {
            c.visible
                && c.key == key
                && c.details.semantics.as_ref().is_some_and(|s| {
                    s.role == SemanticRole::Menu && (name.is_none() || s.name.as_deref() == name)
                })
        })
    };
    let title = find(driver, "title/file", Some("File")).ok_or("no File title")?;
    let [x, y, w, h] = title.details.box_model.border_box.ok_or("title bounds")?;
    press_at(driver, x + w / 2.0, y + h / 2.0)?;
    let popup = find(driver, "items", None).ok_or("File menu did not open")?;
    let [x, y, w, h] = popup.details.box_model.border_box.ok_or("menu bounds")?;
    let mut probe = y + 1.0;
    while probe < y + h {
        let at = Point::new(x + w / 2.0, probe)?;
        driver.runtime_mut().pointer_input(PointerEvent::new(at, PointerKind::Move))?;
        driver.settle()?;
        let now = find(driver, "items", None).ok_or("menu closed while hovering")?;
        if let Some(SemanticValue::Choice { label: Some(label), .. }) =
            now.details.semantics.and_then(|s| s.value)
        {
            if label == row {
                // Move a few pixels further so the press is inside the row, not on its edge.
                let inside = (probe + 4.0).min(y + h - 1.0);
                let at = Point::new(x + w / 2.0, inside)?;
                driver.runtime_mut().pointer_input(PointerEvent::new(at, PointerKind::Move))?;
                driver.settle()?;
                let still = find(driver, "items", None).and_then(|c| c.details.semantics).and_then(|s| s.value);
                let at = if matches!(still, Some(SemanticValue::Choice { label: Some(ref l), .. }) if l == row) {
                    at
                } else {
                    Point::new(x + w / 2.0, probe)?
                };
                driver.runtime_mut().pointer_input(PointerEvent::new(at, PointerKind::Press(PointerButton::Primary)))?;
                driver.runtime_mut().pointer_input(PointerEvent::new(at, PointerKind::Release(PointerButton::Primary)))?;
                driver.settle()?;
                assert!(find(driver, "items", None).is_none(), "menu closes after choosing {row}");
                return Ok(());
            }
        }
        probe += 2.0;
    }
    Err(format!("no menu row {row:?} under the pointer").into())
}

struct Started {
    app: HeadlessApp<Notes>,
}

fn start(dir: &Path) -> R<Started> {
    Ok(Started { app: notes::app(dir.to_path_buf()).headless()? })
}

fn row_id(driver: &Driver<'_>, root: &WidgetId, name: &str) -> R<WidgetId> {
    let list = driver.find(root, &[&Notes::content_path()[..], &["split", "notes"]].concat())?;
    Ok(driver.find(&list, &[name])?)
}

fn click_row(driver: &mut Driver<'_>, root: &WidgetId, name: &str) -> R {
    let row = row_id(driver, root, name)?;
    driver.advance(ClickCounter::DEFAULT_INTERVAL * 2)?;
    driver.click(&row)?;
    Ok(())
}

fn focus_editor(driver: &mut Driver<'_>, root: &WidgetId, name: &str) -> R {
    let editor = driver.find(root, &[&Notes::content_path()[..], &["split", "editor", "tabs", name, "text"]].concat())?;
    driver.runtime_mut().focus(&editor)?;
    Ok(())
}

/// Item 1: the verifier's own folder (non-.txt files, names created out of
/// alphabetical order) lists only the notes, sorted, through inspection;
/// two notes open (pointer and keyboard) and reopening switches tabs
/// without adding one.
#[test]
fn item1_listing_and_reopen() -> R {
    let dir = folder("listing", &[
        ("mango.txt", "m"),
        ("zeta.md", "not a note"),
        ("kiwi.txt", "k"),
        ("README", "no extension"),
        ("apple.txt", "a"),
        ("notes.txt.bak", "backup"),
        ("banana.txt", "b"),
    ]);
    let mut started = start(&dir)?;
    let (rt, notes) = started.app.runtime_and_state_mut();
    let root = notes.root.clone();
    let mut driver = Driver::new(rt);
    driver.settle()?;
    assert_eq!(listed(&driver)?, ["apple.txt", "banana.txt", "kiwi.txt", "mango.txt"]);
    assert_eq!(tabs(&driver)?, (0, None));

    click_row(&mut driver, &root, "kiwi.txt")?;
    assert_eq!(tabs(&driver)?, (1, Some("kiwi.txt".into())));
    assert_eq!(editor_text(&driver, "kiwi.txt")?, "k");

    // Keyboard: focus the list, Down to the first row, Enter.
    let list = driver.find(&root, &[&Notes::content_path()[..], &["split", "notes"]].concat())?;
    driver.runtime_mut().focus(&list)?;
    driver.key(KeyCode::ArrowDown)?;
    driver.key(KeyCode::Enter)?;
    assert_eq!(tabs(&driver)?, (2, Some("apple.txt".into())));
    println!("tab titles after two opens: {:?}", tab_titles(&driver)?);

    // Reopen kiwi by pointer: same count, kiwi active.
    click_row(&mut driver, &root, "kiwi.txt")?;
    assert_eq!(tabs(&driver)?, (2, Some("kiwi.txt".into())));
    assert_eq!(status(&driver)?, "kiwi.txt: 1 lines, 1 words");

    // Reopen apple by keyboard: same count, apple active.
    driver.runtime_mut().focus(&list)?;
    driver.key(KeyCode::Home)?;
    driver.key(KeyCode::Enter)?;
    assert_eq!(tabs(&driver)?, (2, Some("apple.txt".into())));
    assert_eq!(status(&driver)?, "apple.txt: 1 lines, 1 words");
    Ok(())
}

/// Item 2: typing with line breaks; marker and status from inspection;
/// Ctrl+S and File > Save (menu clicks) compared on disk; a failed save
/// into a removed folder shows a notification and keeps the marker.
#[test]
fn item2_edit_save_and_failed_save() -> R {
    let dir = folder("saving", &[("plan.txt", ""), ("other.txt", "x"), ("skip.md", "")]);
    let mut started = start(&dir)?;
    let (rt, notes) = started.app.runtime_and_state_mut();
    let root = notes.root.clone();
    let mut driver = Driver::new(rt);
    driver.settle()?;
    assert_eq!(status(&driver)?, "", "no tab open");

    click_row(&mut driver, &root, "plan.txt")?;
    assert_eq!(tabs(&driver)?, (1, Some("plan.txt".into())));
    assert_eq!(status(&driver)?, "plan.txt: 1 lines, 0 words");

    focus_editor(&mut driver, &root, "plan.txt")?;
    driver.type_text("first line")?;
    assert_eq!(tabs(&driver)?.1.as_deref(), Some("plan.txt *"));
    assert_eq!(status(&driver)?, "plan.txt: 1 lines, 2 words");
    driver.key(KeyCode::Enter)?;
    assert_eq!(status(&driver)?, "plan.txt: 2 lines, 2 words");
    driver.type_text("second  line here")?;
    driver.key(KeyCode::Enter)?;
    driver.key(KeyCode::Enter)?;
    driver.type_text("end")?;
    let expected = "first line\nsecond  line here\n\nend";
    assert_eq!(editor_text(&driver, "plan.txt")?, expected);
    assert_eq!(status(&driver)?, "plan.txt: 4 lines, 6 words");
    assert_eq!(std::fs::read_to_string(dir.join("plan.txt"))?, "", "nothing written before saving");

    // Ctrl+S through the key path.
    driver.chord(KeyChord::primary('s'), KeyCode::KeyS)?;
    assert_eq!(std::fs::read_to_string(dir.join("plan.txt"))?, expected);
    assert_eq!(tabs(&driver)?.1.as_deref(), Some("plan.txt"));

    // File > Save by simulated clicks in the menu bar.
    focus_editor(&mut driver, &root, "plan.txt")?;
    driver.key(KeyCode::Enter)?;
    driver.type_text("menu")?;
    assert_eq!(tabs(&driver)?.1.as_deref(), Some("plan.txt *"));
    let edited = editor_text(&driver, "plan.txt")?;
    assert!(edited.contains("menu") && edited.matches('\n').count() == 4, "{edited:?}");
    menu(&mut driver, "Save")?;
    assert_eq!(std::fs::read_to_string(dir.join("plan.txt"))?, edited);
    assert_eq!(tabs(&driver)?.1.as_deref(), Some("plan.txt"));
    // The verifier's own count of the inspected text, per the task's definition.
    let words = edited.split(|c: char| c.is_whitespace()).filter(|w| !w.is_empty()).count();
    assert_eq!(status(&driver)?, format!("plan.txt: 5 lines, {words} words"));
    assert_eq!(notifications(&driver)?.0, 0.0);

    // Remove the folder; Ctrl+S fails: notification appears, marker stays.
    focus_editor(&mut driver, &root, "plan.txt")?;
    driver.type_text("!")?;
    assert_eq!(tabs(&driver)?.1.as_deref(), Some("plan.txt *"));
    std::fs::remove_dir_all(&dir)?;
    driver.chord(KeyChord::primary('s'), KeyCode::KeyS)?;
    let (count, messages) = notifications(&driver)?;
    println!("failed Ctrl+S notifications: {messages:?}");
    assert_eq!(count, 1.0);
    assert!(messages.iter().any(|m| m.contains("plan.txt")), "{messages:?}");
    assert_eq!(tabs(&driver)?.1.as_deref(), Some("plan.txt *"));
    assert!(!dir.exists());
    // Record what the window shows after the failure (CPU renderer).
    driver.runtime().set_capture_support(true);
    let window = driver.runtime().primary_window();
    let pending = driver.runtime().request_capture(CaptureRequest::new(CaptureTarget::window(&window)));
    driver.settle()?;
    CpuRenderer::new().serve_captures(driver.runtime_mut());
    let capture = pending.try_reply()?.value?;
    std::fs::write(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("acceptance-failed-save.png"), capture.image.encode_png())?;

    // File > Save by menu clicks also fails visibly and keeps the marker.
    menu(&mut driver, "Save")?;
    let (count, _) = notifications(&driver)?;
    assert_eq!(count, 2.0);
    assert_eq!(tabs(&driver)?.1.as_deref(), Some("plan.txt *"));
    Ok(())
}

/// Item 3: New note dialog via Ctrl+N and via the menu bar; empty, `/`,
/// `\`, leading dot, too long and existing names are refused with the
/// explanation visible in inspection and no file created; a valid name
/// creates, lists and opens the note.
#[test]
fn item3_new_note_dialog() -> R {
    let dir = folder("new", &[("pear.txt", "p"), ("fig.txt", "f"), ("data.csv", "1,2")]);
    let mut started = start(&dir)?;
    let (rt, notes) = started.app.runtime_and_state_mut();
    let root = notes.root.clone();
    let mut driver = Driver::new(rt);
    driver.settle()?;
    let before = disk(&dir);
    assert!(open_dialog_content(&driver).is_empty());

    driver.chord(KeyChord::primary('n'), KeyCode::KeyN)?;
    let dialog = new_dialog();
    assert!(component(&driver, &dialog).is_some(), "Ctrl+N opens the dialog");
    assert_eq!(component(&driver, &["app".to_string(), "new-dialog".to_string()].to_vec()).and_then(|c| c.details.semantics).map(|s| s.role), Some(SemanticRole::Dialog));
    let field = driver.find(&root, &["new-dialog", "dialog", "new-note", "name-field", "name"])?;
    let create = driver.find(&root, &["new-dialog", "dialog", "new-note", "actions", "create"])?;
    let error_path = under(&dialog, &["name-error"]);

    let long = "x".repeat(65);
    for typed in ["", "a/b", "a\\b", ".hidden", long.as_str(), "pear", "fig.txt"] {
        driver.replace_text(&field, typed)?;
        driver.advance(ClickCounter::DEFAULT_INTERVAL * 2)?;
        driver.click(&create)?;
        assert!(component(&driver, &dialog).is_some(), "{typed:?}: dialog stays open");
        let explanation = semantics(&driver, &error_path)?.name.unwrap_or_default();
        println!("refused {typed:?}: {explanation:?}");
        assert!(!explanation.is_empty(), "{typed:?}: an explanation is visible");
        assert_eq!(disk(&dir), before, "{typed:?}: no file created");
    }
    // A 64-character name is valid; test via Enter in the field.
    let max = "y".repeat(64);
    driver.replace_text(&field, &max)?;
    driver.key(KeyCode::Enter)?;
    assert!(component(&driver, &dialog).is_none(), "valid name closes the dialog");
    let created = format!("{max}.txt");
    assert!(dir.join(&created).exists());
    assert_eq!(std::fs::read_to_string(dir.join(&created))?, "");
    assert_eq!(tabs(&driver)?, (1, Some(created.clone())));

    // File > New note by menu clicks; valid name via the Create button.
    menu(&mut driver, "New note")?;
    assert!(component(&driver, &dialog).is_some(), "menu opens the dialog");
    assert_eq!(semantics(&driver, &error_path).map(|s| s.name).unwrap_or(None), None, "earlier refusal cleared");
    driver.replace_text(&field, "banana")?;
    driver.advance(ClickCounter::DEFAULT_INTERVAL * 2)?;
    driver.click(&create)?;
    assert!(component(&driver, &dialog).is_none());
    assert_eq!(std::fs::read_to_string(dir.join("banana.txt"))?, "");
    let mut expected_disk = before.clone();
    expected_disk.push("banana.txt".into());
    expected_disk.push(created.clone());
    expected_disk.sort();
    assert_eq!(disk(&dir), expected_disk);
    assert_eq!(listed(&driver)?, ["banana.txt", "fig.txt", "pear.txt", created.as_str()]);
    assert_eq!(tabs(&driver)?, (2, Some("banana.txt".into())));
    assert_eq!(status(&driver)?, "banana.txt: 1 lines, 0 words");
    Ok(())
}

/// Item 4: close an unmodified tab with no dialog in the snapshot (Ctrl+W
/// and File > Close tab), and a modified tab with Cancel, Discard and Save,
/// checking tabs, editor text and files on disk; a failed Save keeps the tab.
#[test]
fn item4_close_choices() -> R {
    let dir = folder("closing", &[("one.txt", "uno"), ("two.txt", "dos"), ("three.txt", "tres"), ("four.txt", "cuatro")]);
    let mut started = start(&dir)?;
    let (rt, notes) = started.app.runtime_and_state_mut();
    let root = notes.root.clone();
    let mut driver = Driver::new(rt);
    driver.settle()?;
    let read = |name: &str| std::fs::read_to_string(dir.join(name)).unwrap();
    let close_button = |driver: &Driver<'_>, key: &str| {
        driver.find(&root, &["new-dialog", "close-dialog", "dialog", "close-tab", "actions", key])
    };
    for name in ["one.txt", "two.txt", "three.txt", "four.txt"] {
        click_row(&mut driver, &root, name)?;
    }
    assert_eq!(tabs(&driver)?, (4, Some("four.txt".into())));

    // Unmodified, Ctrl+W: closes at once, no dialog content in the snapshot.
    driver.chord(KeyChord::primary('w'), KeyCode::KeyW)?;
    assert_eq!(open_dialog_content(&driver), Vec::<Vec<String>>::new());
    assert_eq!(tabs(&driver)?.0, 3);
    assert!(tab_titles(&driver)?.iter().all(|t| !t.starts_with("four")), "{:?}", tab_titles(&driver)?);

    // Unmodified, File > Close tab by menu clicks.
    let active = tabs(&driver)?.1.ok_or("an active tab")?;
    menu(&mut driver, "Close tab")?;
    assert_eq!(open_dialog_content(&driver), Vec::<Vec<String>>::new());
    assert_eq!(tabs(&driver)?.0, 2);
    assert!(tab_titles(&driver)?.iter().all(|t| t != &active));

    // Modified: Cancel keeps tab and text; file unchanged.
    click_row(&mut driver, &root, "two.txt")?;
    focus_editor(&mut driver, &root, "two.txt")?;
    driver.type_text("edited ")?;
    driver.key(KeyCode::Enter)?;
    let text = editor_text(&driver, "two.txt")?;
    assert_ne!(text, "dos");
    assert_eq!(tabs(&driver)?, (2, Some("two.txt *".into())));
    driver.chord(KeyChord::primary('w'), KeyCode::KeyW)?;
    assert!(component(&driver, &close_dialog()).is_some(), "modified tab asks");
    println!("close dialog content: {:?}", open_dialog_content(&driver));
    let cancel = close_button(&driver, "cancel")?;
    driver.advance(ClickCounter::DEFAULT_INTERVAL * 2)?;
    driver.click(&cancel)?;
    assert!(open_dialog_content(&driver).is_empty());
    assert_eq!(tabs(&driver)?, (2, Some("two.txt *".into())));
    assert_eq!(editor_text(&driver, "two.txt")?, text);
    assert_eq!(read("two.txt"), "dos");

    // Modified: Discard closes without writing.
    driver.chord(KeyChord::primary('w'), KeyCode::KeyW)?;
    assert!(component(&driver, &close_dialog()).is_some());
    let discard = close_button(&driver, "discard")?;
    driver.advance(ClickCounter::DEFAULT_INTERVAL * 2)?;
    driver.click(&discard)?;
    assert!(open_dialog_content(&driver).is_empty());
    assert_eq!(tabs(&driver)?.0, 1);
    assert!(tab_titles(&driver)?.iter().all(|t| !t.starts_with("two")));
    assert_eq!(read("two.txt"), "dos");
    // Reopening shows the file's unchanged text.
    click_row(&mut driver, &root, "two.txt")?;
    assert_eq!(editor_text(&driver, "two.txt")?, "dos");
    assert_eq!(tabs(&driver)?, (2, Some("two.txt".into())));

    // Modified: Save (dialog from File > Close tab) writes, then closes.
    focus_editor(&mut driver, &root, "two.txt")?;
    driver.type_text("saved ")?;
    let text = editor_text(&driver, "two.txt")?;
    menu(&mut driver, "Close tab")?;
    assert!(component(&driver, &close_dialog()).is_some());
    let save = close_button(&driver, "save")?;
    driver.advance(ClickCounter::DEFAULT_INTERVAL * 2)?;
    driver.click(&save)?;
    assert!(open_dialog_content(&driver).is_empty());
    assert_eq!(read("two.txt"), text);
    assert_eq!(tabs(&driver)?.0, 1);
    assert!(tab_titles(&driver)?.iter().all(|t| !t.starts_with("two")));

    // Modified with the folder removed: Save fails, tab stays modified.
    let last = tabs(&driver)?.1.ok_or("an active tab")?;
    focus_editor(&mut driver, &root, &last)?;
    driver.type_text("lost ")?;
    std::fs::remove_dir_all(&dir)?;
    driver.chord(KeyChord::primary('w'), KeyCode::KeyW)?;
    let save = close_button(&driver, "save")?;
    driver.advance(ClickCounter::DEFAULT_INTERVAL * 2)?;
    driver.click(&save)?;
    assert_eq!(tabs(&driver)?, (1, Some(format!("{last} *"))));
    assert_eq!(notifications(&driver)?.0, 1.0);
    Ok(())
}

/// Item 5: a CPU capture with two notes open, one modified.
#[test]
fn item5_cpu_capture() -> R {
    let dir = folder("capture", &[("alpha.txt", "first note"), ("beta.txt", "second\nnote"), ("gamma.log", "")]);
    let mut started = start(&dir)?;
    let (rt, notes) = started.app.runtime_and_state_mut();
    let root = notes.root.clone();
    let mut driver = Driver::new(rt);
    driver.settle()?;
    click_row(&mut driver, &root, "alpha.txt")?;
    click_row(&mut driver, &root, "beta.txt")?;
    focus_editor(&mut driver, &root, "alpha.txt").ok();
    // Make alpha active and modified, then leave beta active and unmodified.
    click_row(&mut driver, &root, "alpha.txt")?;
    focus_editor(&mut driver, &root, "alpha.txt")?;
    driver.type_text("Edited: ")?;
    click_row(&mut driver, &root, "beta.txt")?;
    driver.clear_focus()?;
    let titles = tab_titles(&driver)?;
    println!("capture tab titles: {titles:?}");
    assert_eq!(tabs(&driver)?, (2, Some("beta.txt".into())));

    driver.runtime().set_capture_support(true);
    let window = driver.runtime().primary_window();
    let pending = driver.runtime().request_capture(CaptureRequest::new(CaptureTarget::window(&window)));
    driver.settle()?;
    CpuRenderer::new().serve_captures(driver.runtime_mut());
    let capture = pending.try_reply()?.value?;
    let out = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("acceptance-capture.png");
    std::fs::write(&out, capture.image.encode_png())?;
    assert!(capture.image.width() > 0 && capture.image.height() > 0);
    Ok(())
}
