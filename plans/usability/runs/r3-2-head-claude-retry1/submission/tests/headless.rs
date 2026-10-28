//! Drives the application headlessly through simulated input.

use pert_ui::prelude::*;
use pert_ui::testing::*;
use std::time::Duration;
use todo::TaskList;

type R = Result<(), Box<dyn std::error::Error>>;

fn titles(app: &TaskList) -> Vec<String> {
    app.tasks.get().unwrap().iter().map(|t| t.title.clone()).collect()
}

fn status(driver: &Driver<'_>, app: &TaskList) -> Option<String> {
    let label = driver.find(&app.root, &["status"]).unwrap();
    driver.semantics(&label).unwrap().name
}

/// The keys of the rows the list shows, in order.
fn shown(driver: &Driver<'_>, app: &TaskList) -> Vec<String> {
    driver
        .address_children(&app.list)
        .unwrap()
        .into_iter()
        .map(|row| row.segment)
        .collect()
}

fn alt(key: KeyCode) -> KeyChord {
    KeyChord::new(CommandKey::Named(key), Modifiers { alt: true, ..Modifiers::default() })
}

#[test]
fn add_remove_check_reorder() -> R {
    let mut headless = todo::app().headless()?;
    let (ui, app) = headless.runtime_and_state_mut();
    let app = app.clone();
    let mut driver = Driver::new(ui);
    driver.settle()?;
    assert_eq!(status(&driver, &app).as_deref(), Some("0 of 3 done"));

    // Add with Enter.
    driver.replace_text(&app.new_field, "Call the bank")?;
    driver.key(KeyCode::Enter)?;
    assert_eq!(titles(&app).last().map(String::as_str), Some("Call the bank"));
    assert_eq!(app.new_title.get()?, "");
    assert_eq!(status(&driver, &app).as_deref(), Some("0 of 4 done"));

    // Add with the button.
    driver.replace_text(&app.new_field, "Water plants")?;
    let add = driver.find(&app.root, &["entry", "add"])?;
    driver.click(&add)?;
    assert_eq!(titles(&app).len(), 5);
    assert_eq!(status(&driver, &app).as_deref(), Some("0 of 5 done"));

    // Empty titles are refused, by Enter and by the button.
    driver.replace_text(&app.new_field, "   ")?;
    driver.key(KeyCode::Enter)?;
    driver.advance(ClickCounter::DEFAULT_INTERVAL + Duration::from_millis(1))?;
    driver.click(&add)?;
    assert_eq!(titles(&app).len(), 5);

    // Check a task with a click on its checkbox.
    let check = driver.find(&app.list, &["2", "done"])?;
    driver.click(&check)?;
    assert!(app.tasks.get()?[1].done);
    assert_eq!(status(&driver, &app).as_deref(), Some("1 of 5 done"));

    // Select "Buy milk" with a click on its title, move it down with the button.
    driver.advance(ClickCounter::DEFAULT_INTERVAL + Duration::from_millis(1))?;
    let title = driver.find(&app.list, &["1", "title"])?;
    driver.click(&title)?;
    assert_eq!(app.selection.active().get()?.as_deref(), Some("1"));
    let down = driver.find(&app.root, &["actions", "down"])?;
    driver.click(&down)?;
    assert_eq!(titles(&app)[..2], ["Write the plan", "Buy milk"]);
    assert_eq!(shown(&driver, &app)[..2], ["2", "1"]);
    let up = driver.find(&app.root, &["actions", "up"])?;
    driver.click(&up)?;
    driver.advance(ClickCounter::DEFAULT_INTERVAL + Duration::from_millis(1))?;
    driver.click(&up)?; // already first: nothing happens
    assert_eq!(titles(&app)[..2], ["Buy milk", "Write the plan"]);

    // Remove the selected task with the button.
    let remove = driver.find(&app.root, &["actions", "remove"])?;
    driver.click(&remove)?;
    assert_eq!(titles(&app), ["Write the plan", "Ship it", "Call the bank", "Water plants"]);
    assert_eq!(status(&driver, &app).as_deref(), Some("1 of 4 done"));
    assert_eq!(shown(&driver, &app), ["2", "3", "4", "5"]);
    Ok(())
}

#[test]
fn rename_in_place() -> R {
    let mut headless = todo::app().headless()?;
    let (ui, app) = headless.runtime_and_state_mut();
    let app = app.clone();
    let mut driver = Driver::new(ui);
    driver.settle()?;

    // Arrow keys select, Enter starts editing the active row, Enter commits.
    driver.runtime_mut().focus(&app.list)?;
    driver.key(KeyCode::ArrowDown)?;
    driver.key(KeyCode::ArrowDown)?;
    assert_eq!(app.selection.active().get()?.as_deref(), Some("2"));
    driver.key(KeyCode::Enter)?;
    driver.settle()?; // runs the deferred BeginEdit
    let editor = driver.find(&app.list, &["2", "title", "editor"])?;
    assert_eq!(driver.runtime().focused(), Some(editor.clone()));
    driver.type_text("Write the real plan")?;
    driver.key(KeyCode::Enter)?;
    assert_eq!(titles(&app)[1], "Write the real plan");
    assert_eq!(driver.runtime().focused(), Some(app.list.clone()));

    // Escape discards the draft.
    driver.key(KeyCode::Enter)?;
    driver.settle()?;
    driver.type_text("Something else")?;
    driver.key(KeyCode::Escape)?;
    assert_eq!(titles(&app)[1], "Write the real plan");
    let title = driver.find(&app.list, &["2", "title"])?;
    assert_eq!(
        driver.semantics(&title)?.value,
        Some(SemanticValue::Text("Write the real plan".into()))
    );

    // A double click on a title edits it too.
    let ship = driver.find(&app.list, &["3", "title"])?;
    driver.click(&ship)?;
    driver.click(&ship)?;
    driver.type_text("Ship it today")?;
    driver.key(KeyCode::Enter)?;
    assert_eq!(titles(&app)[2], "Ship it today");
    Ok(())
}

#[test]
fn filter_hides_without_changing_tasks() -> R {
    let mut headless = todo::app().headless()?;
    let (ui, app) = headless.runtime_and_state_mut();
    let app = app.clone();
    let mut driver = Driver::new(ui);
    driver.settle()?;
    let before = app.tasks.get()?;

    // Select "Ship it", then filter case-insensitively for "SHIP".
    let ship = driver.find(&app.list, &["3", "title"])?;
    driver.click(&ship)?;
    assert_eq!(app.selection.active().get()?.as_deref(), Some("3"));
    let field = driver.find(&app.root, &["tasks", "filter-field", "filter"])?;
    driver.replace_text(&field, "SHIP")?;
    assert_eq!(shown(&driver, &app), ["3"]);
    assert_eq!(app.tasks.get()?, before);
    // The selected task is still shown, so it stays selected and actionable.
    assert_eq!(app.selection.active().get()?.as_deref(), Some("3"));
    assert!(app.selected_shown.get()?);

    // A query hiding the selected task leaves nothing selected; Delete then
    // removes nothing.
    driver.replace_text(&field, "milk")?;
    assert_eq!(shown(&driver, &app), ["1"]);
    assert_eq!(app.selection.active().get()?, None);
    assert!(!app.selected_shown.get()?);
    driver.runtime_mut().focus(&app.list)?;
    driver.key(KeyCode::ArrowDown)?;
    assert_eq!(app.selection.active().get()?.as_deref(), Some("1"));
    driver.chord(alt(KeyCode::ArrowDown), KeyCode::ArrowDown)?; // only shown row
    assert_eq!(app.tasks.get()?, before);

    // Clearing the query shows everything again, in the original order.
    driver.replace_text(&field, "")?;
    assert_eq!(shown(&driver, &app), ["1", "2", "3"]);
    assert_eq!(app.tasks.get()?, before);
    Ok(())
}

#[test]
fn keyboard_shortcuts() -> R {
    let mut headless = todo::app().headless()?;
    let (ui, app) = headless.runtime_and_state_mut();
    let app = app.clone();
    let mut driver = Driver::new(ui);
    driver.settle()?;

    // Ctrl+N focuses the new-task field, from the list.
    driver.runtime_mut().focus(&app.list)?;
    driver.chord(KeyChord::primary('n'), KeyCode::KeyN)?;
    assert_eq!(driver.runtime().focused(), Some(app.new_field.clone()));
    driver.type_text("Fourth")?;
    driver.key(KeyCode::Enter)?;
    assert_eq!(titles(&app).len(), 4);

    // Delete in the text field edits text; it does not remove a task.
    driver.runtime_mut().focus(&app.list)?;
    driver.key(KeyCode::ArrowDown)?; // selects "Buy milk"
    driver.chord(KeyChord::primary('n'), KeyCode::KeyN)?;
    driver.type_text("x")?;
    driver.key(KeyCode::ArrowLeft)?;
    driver.key(KeyCode::Delete)?;
    assert_eq!(app.new_title.get()?, "");
    assert_eq!(titles(&app).len(), 4);

    // Alt+Down / Alt+Up with the list focused.
    driver.runtime_mut().focus(&app.list)?;
    driver.chord(alt(KeyCode::ArrowDown), KeyCode::ArrowDown)?;
    assert_eq!(titles(&app), ["Write the plan", "Buy milk", "Ship it", "Fourth"]);
    driver.chord(alt(KeyCode::ArrowDown), KeyCode::ArrowDown)?;
    assert_eq!(titles(&app), ["Write the plan", "Ship it", "Buy milk", "Fourth"]);
    driver.chord(alt(KeyCode::ArrowUp), KeyCode::ArrowUp)?;
    assert_eq!(titles(&app), ["Write the plan", "Buy milk", "Ship it", "Fourth"]);
    assert_eq!(app.selection.active().get()?.as_deref(), Some("1"));

    // Alt+Up from elsewhere (the Add button) still moves the selected task.
    let add = driver.find(&app.root, &["entry", "add"])?;
    driver.runtime_mut().focus(&add)?;
    driver.chord(alt(KeyCode::ArrowUp), KeyCode::ArrowUp)?;
    assert_eq!(titles(&app), ["Buy milk", "Write the plan", "Ship it", "Fourth"]);

    // Delete removes the selected task.
    driver.runtime_mut().focus(&app.list)?;
    driver.key(KeyCode::Delete)?;
    assert_eq!(titles(&app), ["Write the plan", "Ship it", "Fourth"]);
    assert_eq!(status(&driver, &app).as_deref(), Some("0 of 3 done"));
    Ok(())
}

#[test]
fn layout_validates() -> R {
    let mut headless = todo::app().headless()?;
    let mut driver = Driver::new(headless.runtime_mut());
    driver.settle()?;
    let report = driver.runtime().validation_report();
    assert!(report.enabled);
    assert!(report.issues.is_empty(), "{:#?}", report.issues);
    Ok(())
}

#[test]
fn capture_after_edits() -> R {
    let mut headless = todo::app().headless()?;
    let (ui, app) = headless.runtime_and_state_mut();
    let app = app.clone();
    let mut driver = Driver::new(ui);
    driver.settle()?;
    let check = driver.find(&app.list, &["2", "done"])?;
    driver.click(&check)?;
    let title = driver.find(&app.list, &["3", "title"])?;
    driver.advance(ClickCounter::DEFAULT_INTERVAL + Duration::from_millis(1))?;
    driver.click(&title)?;
    driver.runtime().set_capture_support(true);
    let pending = driver.runtime().request_capture(CaptureRequest::new(
        CaptureTarget::window(&driver.runtime().primary_window()),
    ));
    driver.settle()?;
    CpuRenderer::new().serve_captures(driver.runtime_mut());
    let capture = pending.try_reply()?.value?;
    let file = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("tasks.png");
    std::fs::write(&file, capture.image.encode_png())?;
    Ok(())
}
