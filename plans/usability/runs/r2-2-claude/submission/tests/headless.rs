//! Headless checks: the application is built on a runtime and driven through
//! the library's `Driver` (pointer clicks, typed text, keys and chords); what
//! it shows is read back through inspection semantics.

use pert_ui::prelude::*;
use pert_ui::testing::*;
use std::time::Duration;
use pert_ui::{CommandKey, Modifiers};
use todo::{TodoApp, Task, build, paths, sample_tasks};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn start() -> Result<(Runtime, TodoApp), Box<dyn std::error::Error>> {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(480.0, 560.0, 1.0)?)?;
    let app = build(&mut ui, sample_tasks())?;
    Driver::new(&mut ui).settle()?;
    Ok((ui, app))
}

/// The rows the list shows, in order, as (title, done) read from semantics.
fn shown(driver: &Driver, app: &TodoApp) -> Result<Vec<(String, bool)>, Box<dyn std::error::Error>> {
    let list = driver.find(&app.root, paths::LIST)?;
    let mut rows = Vec::new();
    for row in driver.runtime().children(&list)? {
        let title = driver.semantics(&driver.find(&row, &["title"])?)?;
        let done = driver.semantics(&driver.find(&row, &["done"])?)?;
        let Some(SemanticValue::Text(title)) = title.value else { panic!("title: {title:?}") };
        let Some(SemanticValue::Bool(done)) = done.value else { panic!("done: {done:?}") };
        rows.push((title, done));
    }
    Ok(rows)
}

fn titles(driver: &Driver, app: &TodoApp) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    Ok(shown(driver, app)?.into_iter().map(|(title, _)| title).collect())
}

fn status(driver: &Driver, app: &TodoApp) -> Result<String, Box<dyn std::error::Error>> {
    let label = driver.find(&app.root, paths::STATUS)?;
    Ok(driver.semantics(&label)?.name.unwrap_or_default())
}

fn model_titles(app: &TodoApp) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    Ok(app.tasks.get()?.into_iter().map(|t: Task| t.title).collect())
}

/// A row's title widget, by the title it shows.
fn row_title(driver: &Driver, app: &TodoApp, title: &str) -> Result<WidgetId, Box<dyn std::error::Error>> {
    let id = app.tasks.get()?.iter().find(|t| t.title == title).ok_or("no such task")?.id;
    let list = driver.find(&app.root, paths::LIST)?;
    Ok(driver.find(&list, &[id.to_string().as_str(), "title"])?)
}

/// Whether the title's in-place editor (its "editor" child) has focus.
fn editing(driver: &Driver, title: &WidgetId) -> Result<bool, Box<dyn std::error::Error>> {
    let Ok(editor) = driver.find(title, &["editor"]) else { return Ok(false) };
    Ok(driver.runtime().focused() == Some(editor))
}

/// One click that is not part of a double click.
fn single_click(driver: &mut Driver, id: &WidgetId) -> TestResult {
    driver.advance(ClickCounter::DEFAULT_INTERVAL + Duration::from_millis(1))?;
    driver.click(id)?;
    driver.advance(ClickCounter::DEFAULT_INTERVAL + Duration::from_millis(1))?;
    Ok(())
}

#[test]
fn starts_with_the_vector() -> TestResult {
    let (mut ui, app) = start()?;
    let driver = Driver::new(&mut ui);
    assert_eq!(titles(&driver, &app)?, ["Buy milk", "Write the plan", "Ship it"]);
    assert_eq!(status(&driver, &app)?, "0 of 3 done");
    // Development validation finds nothing wrong with the layout.
    let report = driver.runtime().validation_report();
    assert!(report.enabled);
    assert!(report.issues.is_empty(), "{:#?}", report.issues);
    Ok(())
}

#[test]
fn add_check_remove_and_reorder() -> TestResult {
    let (mut ui, app) = start()?;
    let mut driver = Driver::new(&mut ui);
    let field = driver.find(&app.root, paths::NEW_FIELD)?;

    // Enter in the field adds and clears it.
    driver.replace_text(&field, "Call the bank")?;
    driver.key(KeyCode::Enter)?;
    assert_eq!(titles(&driver, &app)?.last().map(String::as_str), Some("Call the bank"));
    assert_eq!(driver.semantics(&field)?.value, Some(SemanticValue::Text(String::new())));
    // The Add button adds too.
    driver.replace_text(&field, "Water plants")?;
    let add = driver.find(&app.root, paths::ADD)?;
    driver.click(&add)?;
    assert_eq!(titles(&driver, &app)?.len(), 5);
    assert_eq!(status(&driver, &app)?, "0 of 5 done");
    // Empty and blank titles are refused, by Enter and by the button.
    driver.replace_text(&field, "   ")?;
    driver.key(KeyCode::Enter)?;
    single_click(&mut driver, &add)?;
    driver.replace_text(&field, "")?;
    driver.key(KeyCode::Enter)?;
    assert_eq!(titles(&driver, &app)?.len(), 5);
    assert_eq!(app.tasks.get()?.len(), 5);

    // Checking a task's box updates the record and the status line.
    let list = driver.find(&app.root, paths::LIST)?;
    let check = driver.find(&list, &["2", "done"])?;
    driver.click(&check)?;
    assert!(app.tasks.get()?[1].done);
    assert_eq!(shown(&driver, &app)?[1], ("Write the plan".to_owned(), true));
    assert_eq!(status(&driver, &app)?, "1 of 5 done");
    single_click(&mut driver, &check)?;
    assert_eq!(status(&driver, &app)?, "0 of 5 done");
    single_click(&mut driver, &check)?;

    // Remove the selected task: a press on its title selects its row.
    let title = row_title(&driver, &app, "Ship it")?;
    single_click(&mut driver, &title)?;
    assert_eq!(app.selection.active().get()?.as_deref(), Some("3"));
    let remove = driver.find(&app.root, paths::REMOVE)?;
    driver.click(&remove)?;
    assert_eq!(titles(&driver, &app)?, ["Buy milk", "Write the plan", "Call the bank", "Water plants"]);
    assert_eq!(model_titles(&app)?, ["Buy milk", "Write the plan", "Call the bank", "Water plants"]);
    assert_eq!(status(&driver, &app)?, "1 of 4 done");

    // Reorder the selected task with the buttons.
    let title = row_title(&driver, &app, "Call the bank")?;
    single_click(&mut driver, &title)?;
    let up = driver.find(&app.root, paths::UP)?;
    driver.click(&up)?;
    assert_eq!(titles(&driver, &app)?, ["Buy milk", "Call the bank", "Write the plan", "Water plants"]);
    single_click(&mut driver, &up)?;
    single_click(&mut driver, &up)?; // already first: nothing moves
    assert_eq!(titles(&driver, &app)?, ["Call the bank", "Buy milk", "Write the plan", "Water plants"]);
    let down = driver.find(&app.root, paths::DOWN)?;
    single_click(&mut driver, &down)?;
    assert_eq!(model_titles(&app)?, ["Buy milk", "Call the bank", "Write the plan", "Water plants"]);
    assert_eq!(status(&driver, &app)?, "1 of 4 done");
    Ok(())
}

#[test]
fn rename_in_place_commits_with_enter_and_cancels_with_escape() -> TestResult {
    let (mut ui, app) = start()?;
    let mut driver = Driver::new(&mut ui);

    // A double click on a title starts editing it; Enter commits.
    let title = row_title(&driver, &app, "Ship it")?;
    driver.click(&title)?;
    driver.click(&title)?;
    assert!(editing(&driver, &title)?);
    driver.type_text("Ship it today")?; // the edit selects the whole title
    driver.key(KeyCode::Enter)?;
    assert_eq!(titles(&driver, &app)?, ["Buy milk", "Write the plan", "Ship it today"]);
    assert_eq!(app.tasks.get()?[2].title, "Ship it today");
    // Focus returns to the list.
    let list = driver.find(&app.root, paths::LIST)?;
    assert_eq!(driver.runtime().focused(), Some(list.clone()));

    // Enter on the list renames the active row; Escape discards the draft.
    driver.key(KeyCode::ArrowUp)?;
    assert_eq!(app.selection.active().get()?.as_deref(), Some("2"));
    driver.key(KeyCode::Enter)?;
    driver.settle()?; // the deferred BeginEdit runs
    let title = row_title(&driver, &app, "Write the plan")?;
    assert!(editing(&driver, &title)?);
    driver.type_text("Throw the plan away")?;
    let editor = driver.find(&title, &["editor"])?;
    assert_eq!(driver.semantics(&editor)?.value, Some(SemanticValue::Text("Throw the plan away".into())));
    driver.key(KeyCode::Escape)?;
    assert_eq!(titles(&driver, &app)?, ["Buy milk", "Write the plan", "Ship it today"]);
    assert_eq!(app.tasks.get()?[1].title, "Write the plan");
    assert_eq!(driver.runtime().focused(), Some(list.clone()));

    // An empty title is refused on Enter: the old one stays.
    driver.key(KeyCode::Enter)?;
    driver.settle()?;
    driver.key(KeyCode::Backspace)?;
    driver.key(KeyCode::Enter)?;
    assert_eq!(app.tasks.get()?[1].title, "Write the plan");
    assert_eq!(titles(&driver, &app)?[1], "Write the plan");
    Ok(())
}

#[test]
fn filter_hides_rows_keeps_selection_meaningful_and_the_vector_intact() -> TestResult {
    let (mut ui, app) = start()?;
    let mut driver = Driver::new(&mut ui);
    let search = driver.find(&app.root, paths::SEARCH)?;
    let remove = driver.find(&app.root, paths::REMOVE)?;
    let all = ["Buy milk", "Write the plan", "Ship it"];

    // Select a task, then search for another, in another case.
    let title = row_title(&driver, &app, "Write the plan")?;
    single_click(&mut driver, &title)?;
    assert!(!driver.semantics(&remove)?.disabled);
    driver.replace_text(&search, "MILK")?;
    assert_eq!(titles(&driver, &app)?, ["Buy milk"]);
    assert_eq!(model_titles(&app)?, all);
    assert_eq!(app.filter.counts().get()?.shown, 1);
    // The hidden task is no longer selected, so nothing can act on it.
    assert_eq!(app.selection.active().get()?, None);
    assert!(driver.semantics(&remove)?.disabled);
    driver.key(KeyCode::Delete)?; // in the search field: edits the query only
    assert_eq!(model_titles(&app)?, all);

    // Escape in the search field clears the query and shows every task.
    driver.runtime_mut().focus(&search)?;
    driver.key(KeyCode::Escape)?;
    assert_eq!(titles(&driver, &app)?, all);

    // A shown selection stays selected, and acts relative to the shown rows.
    let title = row_title(&driver, &app, "Ship it")?;
    single_click(&mut driver, &title)?;
    driver.replace_text(&search, "t")?;
    assert_eq!(titles(&driver, &app)?, ["Write the plan", "Ship it"]);
    assert_eq!(app.selection.active().get()?.as_deref(), Some("3"));
    let up = driver.find(&app.root, paths::UP)?;
    driver.click(&up)?;
    assert_eq!(titles(&driver, &app)?, ["Ship it", "Write the plan"]);
    assert_eq!(model_titles(&app)?, ["Buy milk", "Ship it", "Write the plan"]);

    // Down in the field moves into the list; the arrows walk the shown rows only.
    driver.runtime_mut().focus(&search)?;
    driver.key(KeyCode::ArrowDown)?;
    let list = driver.find(&app.root, paths::LIST)?;
    assert_eq!(driver.runtime().focused(), Some(list.clone()));
    driver.key(KeyCode::ArrowDown)?;
    assert_eq!(app.selection.active().get()?.as_deref(), Some("2"));
    driver.key(KeyCode::ArrowDown)?;
    assert_eq!(app.selection.active().get()?.as_deref(), Some("2"));
    driver.key(KeyCode::ArrowUp)?;
    assert_eq!(app.selection.active().get()?.as_deref(), Some("3"));

    // Removing under a filter removes the selected, shown task only.
    driver.click(&remove)?;
    assert_eq!(titles(&driver, &app)?, ["Write the plan"]);
    assert_eq!(model_titles(&app)?, ["Buy milk", "Write the plan"]);
    assert_eq!(status(&driver, &app)?, "0 of 2 done");
    Ok(())
}

#[test]
fn keyboard_shortcuts() -> TestResult {
    let (mut ui, app) = start()?;
    let mut driver = Driver::new(&mut ui);
    let field = driver.find(&app.root, paths::NEW_FIELD)?;
    let list = driver.find(&app.root, paths::LIST)?;
    let search = driver.find(&app.root, paths::SEARCH)?;
    let ctrl = Modifiers { control: true, ..Modifiers::default() };
    let ctrl_n = KeyChord::new(CommandKey::Character('n'), ctrl);

    // Arrow keys select one task at a time on the focused list.
    driver.runtime_mut().focus(&list)?;
    driver.key(KeyCode::ArrowDown)?;
    assert_eq!(app.selection.active().get()?.as_deref(), Some("1"));
    driver.key(KeyCode::ArrowDown)?;
    assert_eq!(app.selection.active().get()?.as_deref(), Some("2"));
    assert_eq!(app.selection.selected().get()?.len(), 1);

    // Alt+Down and Alt+Up move the selected task.
    driver.chord(KeyChord::new(CommandKey::Named(KeyCode::ArrowDown), Modifiers { alt: true, ..Modifiers::default() }), KeyCode::ArrowDown)?;
    assert_eq!(titles(&driver, &app)?, ["Buy milk", "Ship it", "Write the plan"]);
    assert_eq!(app.selection.active().get()?.as_deref(), Some("2"));
    let alt_up = KeyChord::new(CommandKey::Named(KeyCode::ArrowUp), Modifiers { alt: true, ..Modifiers::default() });
    driver.chord(alt_up, KeyCode::ArrowUp)?;
    driver.chord(alt_up, KeyCode::ArrowUp)?;
    assert_eq!(titles(&driver, &app)?, ["Write the plan", "Buy milk", "Ship it"]);
    assert_eq!(model_titles(&app)?, ["Write the plan", "Buy milk", "Ship it"]);

    // Delete removes the selected task.
    driver.key(KeyCode::Delete)?;
    assert_eq!(titles(&driver, &app)?, ["Buy milk", "Ship it"]);
    assert_eq!(status(&driver, &app)?, "0 of 2 done");

    // Ctrl+N focuses the new-task field, from the list and from the search field.
    driver.runtime_mut().focus(&list)?;
    driver.chord(ctrl_n, KeyCode::KeyN)?;
    assert_eq!(driver.runtime().focused(), Some(field.clone()));
    driver.type_text("Pay rent")?;
    driver.key(KeyCode::Enter)?;
    assert_eq!(titles(&driver, &app)?, ["Buy milk", "Ship it", "Pay rent"]);
    driver.runtime_mut().focus(&search)?;
    driver.chord(ctrl_n, KeyCode::KeyN)?;
    assert_eq!(driver.runtime().focused(), Some(field.clone()));

    // Delete inside a text field edits the text, not the list.
    driver.type_text("xy")?;
    driver.key(KeyCode::ArrowLeft)?;
    driver.key(KeyCode::Delete)?;
    assert_eq!(driver.semantics(&field)?.value, Some(SemanticValue::Text("x".into())));
    assert_eq!(app.tasks.get()?.len(), 3);
    Ok(())
}

#[test]
fn the_desktop_app_runs_headless() -> TestResult {
    use std::{cell::RefCell, rc::Rc};
    let built: Rc<RefCell<Option<TodoApp>>> = Rc::default();
    let slot = built.clone();
    let mut headless = todo::app(move |app| *slot.borrow_mut() = Some(app))?.headless()?;
    let app = built.borrow_mut().take().ok_or("not built")?;
    let mut driver = Driver::new(headless.runtime_mut());
    driver.settle()?;
    assert_eq!(status(&driver, &app)?, "0 of 3 done");

    // Alt+Down also works while typing a new task: the field does not use it.
    let title = row_title(&driver, &app, "Buy milk")?;
    single_click(&mut driver, &title)?;
    let field = driver.find(&app.root, paths::NEW_FIELD)?;
    driver.click(&field)?;
    assert_eq!(driver.runtime().focused(), Some(field.clone()));
    let alt = Modifiers { alt: true, ..Modifiers::default() };
    driver.chord(KeyChord::new(CommandKey::Named(KeyCode::ArrowDown), alt), KeyCode::ArrowDown)?;
    assert_eq!(titles(&driver, &app)?, ["Write the plan", "Buy milk", "Ship it"]);

    let list = driver.find(&app.root, paths::LIST)?;
    let check = driver.find(&list, &["3", "done"])?;
    driver.click(&check)?;
    assert_eq!(status(&driver, &app)?, "1 of 3 done");
    drop(driver);
    headless.shutdown()?;
    Ok(())
}

/// Renders the window on the CPU after a few edits, to target/tasks.png.
#[test]
fn capture() -> TestResult {
    let (mut ui, app) = start()?;
    let mut driver = Driver::new(&mut ui);
    let list = driver.find(&app.root, paths::LIST)?;
    driver.click(&driver.find(&list, &["2", "done"])?)?;
    driver.runtime_mut().focus(&list)?;
    driver.key(KeyCode::ArrowDown)?;
    let field = driver.find(&app.root, paths::NEW_FIELD)?;
    driver.replace_text(&field, "Pay rent")?;
    driver.runtime().set_capture_support(true);
    let pending = driver.runtime().request_capture(CaptureRequest::new(
        CaptureTarget::window(&driver.runtime().primary_window()),
    ));
    driver.settle()?;
    CpuRenderer::new().serve_captures(driver.runtime_mut());
    let capture = pending.wait()?.value?;
    std::fs::write(concat!(env!("CARGO_MANIFEST_DIR"), "/target/tasks.png"), capture.image.encode_png())?;
    Ok(())
}
