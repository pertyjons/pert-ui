//! Independent acceptance checks added by the verifier (not the tester).
//!
//! They drive the unchanged application through the library's headless
//! facilities: `testing::Driver` input (clicks, typed text, named keys and
//! modifier chords) and inspection (`address_children`, `semantics`,
//! `focused`). The application's `tasks` signal is read to compare the
//! underlying vector and, in `vector_drives_rows`, written directly to show
//! that the rows follow the vector. No application function is called.

use pert_ui::prelude::*;
use pert_ui::testing::*;
use std::time::Duration;
use todo::{Task, TaskList};

type R = Result<(), Box<dyn std::error::Error>>;

fn ids(app: &TaskList) -> Vec<u64> {
    app.tasks.get().unwrap().iter().map(|t| t.id).collect()
}

fn titles(app: &TaskList) -> Vec<String> {
    app.tasks.get().unwrap().iter().map(|t| t.title.clone()).collect()
}

fn status(driver: &Driver<'_>, app: &TaskList) -> Option<String> {
    let label = driver.find(&app.root, &["status"]).unwrap();
    driver.semantics(&label).unwrap().name
}

fn shown(driver: &Driver<'_>, app: &TaskList) -> Vec<String> {
    driver
        .address_children(&app.list)
        .unwrap()
        .into_iter()
        .map(|row| row.segment)
        .collect()
}

fn title_value(driver: &Driver<'_>, app: &TaskList, key: &str) -> Option<SemanticValue> {
    let title = driver.find(&app.list, &[key, "title"]).unwrap();
    driver.semantics(&title).unwrap().value
}

fn checked(driver: &Driver<'_>, app: &TaskList, key: &str) -> Option<SemanticValue> {
    let check = driver.find(&app.list, &[key, "done"]).unwrap();
    driver.semantics(&check).unwrap().value
}

fn pause(driver: &mut Driver<'_>) {
    driver
        .advance(ClickCounter::DEFAULT_INTERVAL + Duration::from_millis(1))
        .unwrap();
}

fn ctrl(letter: char) -> KeyChord {
    KeyChord::new(
        CommandKey::Character(letter),
        Modifiers { control: true, ..Modifiers::default() },
    )
}

fn alt(key: KeyCode) -> KeyChord {
    KeyChord::new(CommandKey::Named(key), Modifiers { alt: true, ..Modifiers::default() })
}

/// The rows follow the vector: removing, adding and changing a record in the
/// vector itself changes the inspected rows and the status line.
#[test]
fn vector_drives_rows() -> R {
    let mut headless = todo::app().headless()?;
    let (ui, app) = headless.runtime_and_state_mut();
    let app = app.clone();
    let mut driver = Driver::new(ui);
    driver.settle()?;
    assert_eq!(shown(&driver, &app), ["1", "2", "3"]);
    assert_eq!(status(&driver, &app).as_deref(), Some("0 of 3 done"));

    // Remove "Write the plan" from the vector directly.
    app.tasks.update(|tasks| tasks.retain(|t| t.id != 2))?;
    driver.settle()?;
    assert_eq!(shown(&driver, &app), ["1", "3"]);
    assert!(driver.find(&app.list, &["2"]).is_err(), "row 2 still present");
    assert_eq!(status(&driver, &app).as_deref(), Some("0 of 2 done"));

    // Push a done record directly; its row, title and checkbox appear.
    app.tasks.update(|tasks| {
        tasks.push(Task { id: 9, title: "From the vector".into(), done: true })
    })?;
    driver.settle()?;
    assert_eq!(shown(&driver, &app), ["1", "3", "9"]);
    assert_eq!(
        title_value(&driver, &app, "9"),
        Some(SemanticValue::Text("From the vector".into()))
    );
    assert_eq!(checked(&driver, &app, "9"), Some(SemanticValue::Bool(true)));
    assert_eq!(status(&driver, &app).as_deref(), Some("1 of 3 done"));

    // Reorder in the vector; the rows follow.
    app.tasks.update(|tasks| tasks.reverse())?;
    driver.settle()?;
    assert_eq!(shown(&driver, &app), ["9", "3", "1"]);
    Ok(())
}

/// Checking, unchecking and removing a done task update the inspected
/// checkbox state and the status line; removal by button and by Delete.
#[test]
fn status_follows_check_uncheck_remove() -> R {
    let mut headless = todo::app().headless()?;
    let (ui, app) = headless.runtime_and_state_mut();
    let app = app.clone();
    let mut driver = Driver::new(ui);
    driver.settle()?;

    let c1 = driver.find(&app.list, &["1", "done"])?;
    let c3 = driver.find(&app.list, &["3", "done"])?;
    driver.click(&c1)?;
    pause(&mut driver);
    driver.click(&c3)?;
    assert_eq!(checked(&driver, &app, "1"), Some(SemanticValue::Bool(true)));
    assert_eq!(status(&driver, &app).as_deref(), Some("2 of 3 done"));
    pause(&mut driver);
    let c1 = driver.find(&app.list, &["1", "done"])?;
    driver.click(&c1)?;
    assert_eq!(checked(&driver, &app, "1"), Some(SemanticValue::Bool(false)));
    assert!(!app.tasks.get()?[0].done);
    assert_eq!(status(&driver, &app).as_deref(), Some("1 of 3 done"));

    // Remove the done task "Ship it" with the button.
    pause(&mut driver);
    let ship = driver.find(&app.list, &["3", "title"])?;
    driver.click(&ship)?;
    let remove = driver.find(&app.root, &["actions", "remove"])?;
    driver.click(&remove)?;
    assert_eq!(ids(&app), [1, 2]);
    assert_eq!(shown(&driver, &app), ["1", "2"]);
    assert_eq!(status(&driver, &app).as_deref(), Some("0 of 2 done"));

    // Check "Write the plan" and remove it with Delete from the list.
    pause(&mut driver);
    let c2 = driver.find(&app.list, &["2", "done"])?;
    driver.click(&c2)?;
    assert_eq!(status(&driver, &app).as_deref(), Some("1 of 2 done"));
    pause(&mut driver);
    let plan = driver.find(&app.list, &["2", "title"])?;
    driver.click(&plan)?;
    driver.key(KeyCode::Delete)?;
    assert_eq!(ids(&app), [1]);
    assert_eq!(shown(&driver, &app), ["1"]);
    assert_eq!(status(&driver, &app).as_deref(), Some("0 of 1 done"));
    Ok(())
}

/// Rename in place: Enter commits (vector and inspected value), Escape
/// discards (vector and inspected value unchanged, editor closed).
#[test]
fn rename_enter_commits_escape_discards() -> R {
    let mut headless = todo::app().headless()?;
    let (ui, app) = headless.runtime_and_state_mut();
    let app = app.clone();
    let mut driver = Driver::new(ui);
    driver.settle()?;

    // Double click "Buy milk", replace the whole draft, commit with Enter.
    let milk = driver.find(&app.list, &["1", "title"])?;
    driver.click(&milk)?;
    driver.click(&milk)?;
    let editor = driver.find(&app.list, &["1", "title", "editor"])?;
    assert_eq!(driver.runtime().focused(), Some(editor.clone()));
    driver.replace_text(&editor, "Buy oat milk")?;
    assert_eq!(titles(&app)[0], "Buy milk", "draft must not commit before Enter");
    driver.key(KeyCode::Enter)?;
    assert_eq!(titles(&app)[0], "Buy oat milk");
    assert_eq!(title_value(&driver, &app, "1"), Some(SemanticValue::Text("Buy oat milk".into())));
    assert_ne!(driver.runtime().focused(), Some(editor));

    // Enter on the active row edits it; Escape discards the draft.
    let before = app.tasks.get()?;
    driver.key(KeyCode::Enter)?;
    driver.settle()?;
    let editor = driver.find(&app.list, &["1", "title", "editor"])?;
    assert_eq!(driver.runtime().focused(), Some(editor.clone()));
    driver.replace_text(&editor, "Discard me")?;
    driver.key(KeyCode::Escape)?;
    assert_eq!(app.tasks.get()?, before);
    assert_eq!(title_value(&driver, &app, "1"), Some(SemanticValue::Text("Buy oat milk".into())));
    assert_ne!(driver.runtime().focused(), Some(editor));
    assert_eq!(shown(&driver, &app), ["1", "2", "3"]);
    Ok(())
}

/// The filter, driven by typing into the search field: case-insensitive,
/// never changes the vector, and the selection stays meaningful: a shown
/// selected task is the one Delete removes; a hidden one is not acted on.
#[test]
fn filter_keeps_vector_and_meaningful_selection() -> R {
    let mut headless = todo::app().headless()?;
    let (ui, app) = headless.runtime_and_state_mut();
    let app = app.clone();
    let mut driver = Driver::new(ui);
    driver.settle()?;
    let before = app.tasks.get()?;

    // Select "Buy milk" with the arrow key, then type a mixed-case query.
    driver.runtime_mut().focus(&app.list)?;
    driver.key(KeyCode::ArrowDown)?;
    assert_eq!(app.selection.active().get()?.as_deref(), Some("1"));
    let field = driver.find(&app.root, &["tasks", "filter-field", "filter"])?;
    driver.runtime_mut().focus(&field)?;
    driver.type_text("PlA")?;
    assert_eq!(shown(&driver, &app), ["2"]);
    assert_eq!(app.tasks.get()?, before);
    // The hidden selection is cleared; Delete and Alt+arrows do nothing.
    assert_eq!(app.selection.active().get()?, None);
    driver.runtime_mut().focus(&app.list)?;
    driver.key(KeyCode::Delete)?;
    driver.chord(alt(KeyCode::ArrowUp), KeyCode::ArrowUp)?;
    assert_eq!(app.tasks.get()?, before);

    // A query matching several rows; select the shown "Ship it" by arrows.
    driver.replace_text(&field, "i")?;
    assert_eq!(shown(&driver, &app), ["1", "2", "3"]);
    driver.replace_text(&field, "IT")?;
    assert_eq!(shown(&driver, &app), ["2", "3"]);
    assert_eq!(app.tasks.get()?, before);
    driver.runtime_mut().focus(&app.list)?;
    driver.key(KeyCode::ArrowDown)?;
    driver.key(KeyCode::ArrowDown)?;
    assert_eq!(app.selection.active().get()?.as_deref(), Some("3"));
    // Delete removes exactly the shown selected task.
    driver.key(KeyCode::Delete)?;
    assert_eq!(ids(&app), [1, 2]);
    assert_eq!(shown(&driver, &app), ["2"]);

    // Clearing the query shows the remaining tasks in vector order.
    driver.replace_text(&field, "")?;
    assert_eq!(shown(&driver, &app), ["1", "2"]);
    Ok(())
}

/// Shortcuts with explicit Control/Alt modifiers through the driver's
/// simulated key path, from several focus targets.
#[test]
fn shortcuts_from_other_focus() -> R {
    let mut headless = todo::app().headless()?;
    let (ui, app) = headless.runtime_and_state_mut();
    let app = app.clone();
    let mut driver = Driver::new(ui);
    driver.settle()?;
    let search = driver.find(&app.root, &["tasks", "filter-field", "filter"])?;
    let add = driver.find(&app.root, &["entry", "add"])?;
    let remove = driver.find(&app.root, &["actions", "remove"])?;

    // Ctrl+N from the search field, the Add button and the list.
    for from in [&search, &add, &app.list] {
        driver.runtime_mut().focus(from)?;
        driver.chord(ctrl('n'), KeyCode::KeyN)?;
        assert_eq!(driver.runtime().focused(), Some(app.new_field.clone()));
    }
    assert_eq!(ids(&app), [1, 2, 3]);

    // Select "Buy milk", then Alt+Down / Alt+Up with the Remove button focused.
    driver.runtime_mut().focus(&app.list)?;
    driver.key(KeyCode::ArrowDown)?;
    driver.runtime_mut().focus(&remove)?;
    driver.chord(alt(KeyCode::ArrowDown), KeyCode::ArrowDown)?;
    assert_eq!(ids(&app), [2, 1, 3]);
    assert_eq!(shown(&driver, &app), ["2", "1", "3"]);
    driver.chord(alt(KeyCode::ArrowDown), KeyCode::ArrowDown)?;
    assert_eq!(shown(&driver, &app), ["2", "3", "1"]);
    driver.chord(alt(KeyCode::ArrowDown), KeyCode::ArrowDown)?; // last: no-op
    assert_eq!(ids(&app), [2, 3, 1]);
    driver.chord(alt(KeyCode::ArrowUp), KeyCode::ArrowUp)?;
    driver.chord(alt(KeyCode::ArrowUp), KeyCode::ArrowUp)?;
    driver.chord(alt(KeyCode::ArrowUp), KeyCode::ArrowUp)?; // first: no-op
    assert_eq!(ids(&app), [1, 2, 3]);
    assert_eq!(shown(&driver, &app), ["1", "2", "3"]);
    assert_eq!(app.selection.active().get()?.as_deref(), Some("1"));

    // Delete with the Add button focused removes the selected task.
    driver.runtime_mut().focus(&add)?;
    driver.key(KeyCode::Delete)?;
    assert_eq!(ids(&app), [2, 3]);
    assert_eq!(shown(&driver, &app), ["2", "3"]);
    assert_eq!(status(&driver, &app).as_deref(), Some("0 of 2 done"));
    Ok(())
}

/// Adding with Enter and the button updates inspected rows (not only the
/// vector); empty titles add nothing.
#[test]
fn add_updates_rows() -> R {
    let mut headless = todo::app().headless()?;
    let (ui, app) = headless.runtime_and_state_mut();
    let app = app.clone();
    let mut driver = Driver::new(ui);
    driver.settle()?;
    driver.chord(ctrl('n'), KeyCode::KeyN)?;
    driver.type_text("Fourth")?;
    driver.key(KeyCode::Enter)?;
    assert_eq!(shown(&driver, &app), ["1", "2", "3", "4"]);
    assert_eq!(title_value(&driver, &app, "4"), Some(SemanticValue::Text("Fourth".into())));
    let new_field = driver.semantics(&app.new_field)?;
    assert_eq!(new_field.value, Some(SemanticValue::Text(String::new())));
    driver.key(KeyCode::Enter)?; // empty
    let add = driver.find(&app.root, &["entry", "add"])?;
    driver.click(&add)?; // empty
    assert_eq!(shown(&driver, &app), ["1", "2", "3", "4"]);
    driver.replace_text(&app.new_field, "Fifth")?;
    pause(&mut driver);
    driver.click(&add)?;
    assert_eq!(shown(&driver, &app), ["1", "2", "3", "4", "5"]);
    assert_eq!(status(&driver, &app).as_deref(), Some("0 of 5 done"));

    // A CPU capture of the final state for the record.
    driver.runtime().set_capture_support(true);
    let pending = driver.runtime().request_capture(CaptureRequest::new(
        CaptureTarget::window(&driver.runtime().primary_window()),
    ));
    driver.settle()?;
    CpuRenderer::new().serve_captures(driver.runtime_mut());
    let capture = pending.try_reply()?.value?;
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("acceptance-captures");
    std::fs::create_dir_all(&dir)?;
    std::fs::write(dir.join("after-adds.png"), capture.image.encode_png())?;
    Ok(())
}
