//! Headless acceptance checks: each drives the application through the
//! library's `HeadlessApp` and `Driver` and reads what it shows through
//! inspection.
use std::path::PathBuf;
use std::time::Duration;

use pert_ui::prelude::*;
use pert_ui::testing::*;
use settings_editor::{Editor, Format, Settings};
use settings_editor::app as editor_app;

type TestResult = Result<(), Box<dyn std::error::Error>>;

const PREFIX: &[&str] = &["content", "output", "fields", "prefix-field", "prefix"];
const FORMAT: &[&str] = &["content", "output", "fields", "format-field", "format"];
const QUALITY: &[&str] = &["content", "quality", "fields", "quality-field", "quality"];
const WIDTH: &[&str] = &["content", "quality", "fields", "max-width-field", "max-width"];
const OPEN_FOLDER: &[&str] = &["content", "behavior", "fields", "open-folder"];
const APPLY: &[&str] = &["content", "buttons", "apply"];
const REVERT: &[&str] = &["content", "buttons", "revert"];
const DIRTY: &[&str] = &["content", "buttons", "dirty"];
const MESSAGE: &[&str] = &["content", "message"];

/// A fresh scratch directory for one test.
fn scratch(name: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn sample() -> Settings {
    Settings {
        prefix: "holiday_2026".to_owned(),
        format: Format::WebP,
        quality: 70,
        max_width: 2048,
        open_folder: true,
    }
}

/// Starts the application headless over a settings file holding `settings`.
fn start(dir: &PathBuf, settings: &Settings) -> Result<(PathBuf, HeadlessApp<Editor>), Box<dyn std::error::Error>> {
    let path = dir.join("settings.json");
    std::fs::write(&path, serde_json::to_string(settings)?)?;
    let mut app = editor_app(path.clone()).headless()?;
    Driver::new(app.runtime_mut()).settle()?;
    Ok((path, app))
}

/// The component at `path` below the application's root.
fn at(driver: &Driver<'_>, editor: &Editor, path: &[&str]) -> Result<WidgetId, TestError> {
    driver.find(&editor.root, path)
}

fn semantics(driver: &Driver<'_>, editor: &Editor, path: &[&str]) -> Result<Semantics, Box<dyn std::error::Error>> {
    Ok(driver.semantics(&at(driver, editor, path)?)?)
}

/// The dropdown's semantics live on its anchor, below its own element.
fn format_semantics(driver: &Driver<'_>, editor: &Editor) -> Result<Semantics, Box<dyn std::error::Error>> {
    let select = at(driver, editor, FORMAT)?;
    let anchor = driver.runtime().find_descendant(&select, "select")?.ok_or("no dropdown anchor")?;
    Ok(driver.semantics(&anchor)?)
}

fn name(driver: &Driver<'_>, editor: &Editor, path: &[&str]) -> Result<String, Box<dyn std::error::Error>> {
    Ok(semantics(driver, editor, path)?.name.unwrap_or_default())
}

fn number(driver: &Driver<'_>, editor: &Editor, path: &[&str]) -> Result<f64, Box<dyn std::error::Error>> {
    match semantics(driver, editor, path)?.value {
        Some(SemanticValue::Number { value, .. }) => Ok(value),
        other => Err(format!("not a number: {other:?}").into()),
    }
}

fn property(driver: &Driver<'_>, id: &WidgetId, key: &str) -> Result<Option<InspectValue>, Box<dyn std::error::Error>> {
    let query = InspectionQuery::components(Selector::id(driver.runtime().inspection_id(id)?));
    let result = driver.runtime().query(&query.detail(Detail::Full));
    let component = result.components.first().ok_or("no such component")?;
    Ok(component.details.properties.get(key).cloned())
}

/// Whether Apply and Revert are enabled, as their semantics report.
fn buttons(driver: &Driver<'_>, editor: &Editor) -> Result<(bool, bool), Box<dyn std::error::Error>> {
    Ok((
        !semantics(driver, editor, APPLY)?.disabled,
        !semantics(driver, editor, REVERT)?.disabled,
    ))
}

/// Acceptance 1: every field, group and button exists and shows the file's values.
#[test]
fn shows_the_values_loaded_from_the_file() -> TestResult {
    let dir = scratch("loads");
    let (_, mut app) = start(&dir, &sample())?;
    let (ui, editor) = app.runtime_and_state_mut();
    let driver = Driver::new(ui);

    for (group, title) in [("output", "Output"), ("quality", "Quality"), ("behavior", "Behavior")] {
        let group = semantics(&driver, editor, &["content", group])?;
        assert_eq!(group.role, SemanticRole::Group);
        assert_eq!(group.name.as_deref(), Some(title));
    }

    let prefix = semantics(&driver, editor, PREFIX)?;
    assert_eq!(prefix.name.as_deref(), Some("File name prefix"));
    assert_eq!(prefix.value, Some(SemanticValue::Text("holiday_2026".into())));

    let format = format_semantics(&driver, editor)?;
    assert_eq!(format.role, SemanticRole::ComboBox);
    assert_eq!(format.name.as_deref(), Some("Format"));
    assert!(matches!(
        format.value,
        Some(SemanticValue::Choice { count: 3, label: Some(ref label), .. }) if label == "WebP"
    ));

    assert_eq!(name(&driver, editor, &["content", "quality", "fields", "quality-field"])?, "JPEG quality");
    assert_eq!(number(&driver, editor, QUALITY)?, 70.0);
    assert_eq!(name(&driver, editor, &["content", "quality", "fields", "max-width-field"])?, "Maximum width");
    assert_eq!(number(&driver, editor, WIDTH)?, 2048.0);
    let width = at(&driver, editor, WIDTH)?;
    assert_eq!(property(&driver, &width, "unit")?, Some(InspectValue::Text("px".into())));

    let toggle = semantics(&driver, editor, OPEN_FOLDER)?;
    assert_eq!(toggle.role, SemanticRole::Switch);
    assert_eq!(toggle.name.as_deref(), Some("Open folder after export"));
    assert_eq!(toggle.value, Some(SemanticValue::Bool(true)));

    assert_eq!(name(&driver, editor, APPLY)?, "Apply");
    assert_eq!(name(&driver, editor, REVERT)?, "Revert");
    assert_eq!(name(&driver, editor, DIRTY)?, "All changes applied");
    assert!(name(&driver, editor, MESSAGE)?.starts_with("Loaded "));
    drop(driver);
    app.shutdown()?;
    Ok(())
}

/// Without a file the defaults are shown and nothing is dirty.
#[test]
fn starts_from_defaults_without_a_file() -> TestResult {
    let dir = scratch("defaults");
    let mut app = editor_app(dir.join("missing.json")).headless()?;
    let (ui, editor) = app.runtime_and_state_mut();
    let mut driver = Driver::new(ui);
    driver.settle()?;
    let defaults = Settings::default();
    assert_eq!(semantics(&driver, editor, PREFIX)?.value, Some(SemanticValue::Text(defaults.prefix)));
    assert_eq!(number(&driver, editor, QUALITY)?, defaults.quality as f64);
    assert_eq!(buttons(&driver, editor)?, (false, false));
    assert!(name(&driver, editor, MESSAGE)?.contains("defaults"));
    drop(driver);
    app.shutdown()?;
    Ok(())
}

/// Acceptance 2: an invalid prefix and an out-of-range quality are refused,
/// the explanation is shown next to the field and the old value stays.
#[test]
fn refuses_invalid_input_and_keeps_the_old_value() -> TestResult {
    let dir = scratch("refuses");
    let (_, mut app) = start(&dir, &sample())?;
    let (ui, editor) = app.runtime_and_state_mut();
    let mut driver = Driver::new(ui);
    let prefix = at(&driver, editor, PREFIX)?;
    let quality = at(&driver, editor, QUALITY)?;

    for (bad, explanation) in [
        ("my photos!", "is not allowed"),
        ("", "Enter a prefix"),
        (&"x".repeat(33)[..], "At most 32 characters"),
    ] {
        driver.replace_text(&prefix, bad)?;
        driver.key(KeyCode::Enter)?;
        let shown = driver.semantics(&prefix)?;
        assert!(shown.invalid, "{bad:?} accepted");
        let description = shown.description.unwrap_or_default();
        assert!(description.contains(explanation), "{bad:?}: {description:?}");
        assert!(editor.prefix_field.status().get()?.is_error());
        assert_eq!(editor.prefix.get()?, "holiday_2026", "the last valid value stays");
    }
    // The field shows the refused draft, and the field wrapper shows the message too.
    let field = semantics(&driver, editor, &["content", "output", "fields", "prefix-field"])?;
    assert!(field.description.unwrap_or_default().contains("At most 32 characters"));

    // A valid prefix ends the refusal.
    driver.replace_text(&prefix, "trip-01")?;
    driver.key(KeyCode::Enter)?;
    assert!(!driver.semantics(&prefix)?.invalid);
    assert_eq!(editor.prefix.get()?, "trip-01");
    assert!(editor.prefix_field.status().get()? == FieldStatus::Normal);

    for bad in ["150", "0", "abc", "7.5"] {
        driver.replace_text(&quality, bad)?;
        driver.key(KeyCode::Enter)?;
        let shown = driver.semantics(&quality)?;
        assert!(shown.invalid, "{bad:?} accepted");
        assert!(shown.description.as_deref().is_some_and(|d| !d.is_empty()), "{bad:?}: no explanation");
        assert!(editor.quality_field.status().get()?.is_error());
        assert_eq!(editor.quality.get()?, 70, "the last valid value stays");
    }
    let width = at(&driver, editor, WIDTH)?;
    driver.replace_text(&width, "20000")?;
    driver.key(KeyCode::Enter)?;
    assert!(driver.semantics(&width)?.invalid);
    assert_eq!(editor.max_width.get()?, 2048);
    drop(driver);
    app.shutdown()?;
    Ok(())
}

/// Acceptance 3: Apply and Revert follow the dirty state and Revert restores
/// the applied values.
#[test]
fn apply_and_revert_follow_the_dirty_state() -> TestResult {
    let dir = scratch("dirty");
    let (_, mut app) = start(&dir, &sample())?;
    let (ui, editor) = app.runtime_and_state_mut();
    let mut driver = Driver::new(ui);
    assert_eq!(buttons(&driver, editor)?, (false, false), "clean");
    assert_eq!(name(&driver, editor, DIRTY)?, "All changes applied");

    // Edit three fields of different kinds.
    let quality = at(&driver, editor, QUALITY)?;
    driver.replace_text(&quality, "55")?;
    driver.key(KeyCode::Enter)?;
    assert_eq!(buttons(&driver, editor)?, (true, true), "dirty");
    assert_eq!(name(&driver, editor, DIRTY)?, "● Unsaved changes");
    let toggle = at(&driver, editor, OPEN_FOLDER)?;
    driver.click(&toggle)?;
    assert_eq!(driver.semantics(&toggle)?.value, Some(SemanticValue::Bool(false)));
    let prefix = at(&driver, editor, PREFIX)?;
    driver.replace_text(&prefix, "renamed")?;
    driver.key(KeyCode::Enter)?;

    // Editing back to the applied value is clean again.
    driver.replace_text(&quality, "70")?;
    driver.key(KeyCode::Enter)?;
    driver.click(&toggle)?;
    driver.replace_text(&prefix, "holiday_2026")?;
    driver.key(KeyCode::Enter)?;
    assert_eq!(buttons(&driver, editor)?, (false, false), "clean after editing back");

    // Revert restores every applied value.
    driver.replace_text(&quality, "55")?;
    driver.key(KeyCode::Enter)?;
    driver.click(&toggle)?;
    driver.replace_text(&prefix, "renamed")?;
    driver.key(KeyCode::Enter)?;
    let revert = at(&driver, editor, REVERT)?;
    driver.click(&revert)?;
    assert_eq!(semantics(&driver, editor, PREFIX)?.value, Some(SemanticValue::Text("holiday_2026".into())));
    assert_eq!(number(&driver, editor, QUALITY)?, 70.0);
    assert_eq!(driver.semantics(&toggle)?.value, Some(SemanticValue::Bool(true)));
    assert_eq!(buttons(&driver, editor)?, (false, false), "clean after Revert");
    assert_eq!(name(&driver, editor, DIRTY)?, "All changes applied");

    // A refused draft alone offers Revert but not Apply; Revert discards it.
    driver.replace_text(&quality, "500")?;
    driver.key(KeyCode::Enter)?;
    assert_eq!(buttons(&driver, editor)?, (false, true), "refused draft");
    assert_eq!(name(&driver, editor, DIRTY)?, "● Unsaved changes");
    driver.click(&revert)?;
    assert!(!driver.semantics(&quality)?.invalid);
    assert_eq!(number(&driver, editor, QUALITY)?, 70.0);
    assert_eq!(buttons(&driver, editor)?, (false, false));
    drop(driver);
    app.shutdown()?;
    Ok(())
}

/// Acceptance 4, success: Apply writes the file on a worker and clears the dirty state.
#[test]
fn apply_saves_the_file() -> TestResult {
    let dir = scratch("save-ok");
    let (path, mut app) = start(&dir, &sample())?;
    let (ui, editor) = app.runtime_and_state_mut();
    let mut driver = Driver::new(ui);
    let prefix = at(&driver, editor, PREFIX)?;
    driver.replace_text(&prefix, "saved-prefix")?;
    driver.key(KeyCode::Enter)?;
    let width = at(&driver, editor, WIDTH)?;
    driver.replace_text(&width, "1024")?;
    driver.key(KeyCode::Enter)?;
    let apply = at(&driver, editor, APPLY)?;
    driver.click(&apply)?;
    driver.wait_until(Duration::from_secs(5), |_| Ok(!editor.saving.get()?))?;

    let written: Settings = serde_json::from_str(&std::fs::read_to_string(&path)?)?;
    assert_eq!(written, Settings { prefix: "saved-prefix".into(), max_width: 1024, ..sample() });
    assert_eq!(editor.applied.get()?, written);
    assert_eq!(buttons(&driver, editor)?, (false, false), "clean after a save");
    assert_eq!(name(&driver, editor, DIRTY)?, "All changes applied");
    assert_eq!(name(&driver, editor, MESSAGE)?, "Settings saved");
    drop(driver);
    app.shutdown()?;

    // A new start shows what was saved.
    let mut app = editor_app(path).headless()?;
    let (ui, editor) = app.runtime_and_state_mut();
    let mut driver = Driver::new(ui);
    driver.settle()?;
    assert_eq!(semantics(&driver, editor, PREFIX)?.value, Some(SemanticValue::Text("saved-prefix".into())));
    assert_eq!(number(&driver, editor, WIDTH)?, 1024.0);
    drop(driver);
    app.shutdown()?;
    Ok(())
}

/// Acceptance 4, failure: an unwritable path is reported and leaves the state dirty.
#[test]
fn a_failed_save_is_reported_and_stays_dirty() -> TestResult {
    let dir = scratch("save-fails");
    // The settings file's directory is a regular file, so writing fails.
    std::fs::write(dir.join("blocker"), "not a directory")?;
    let path = dir.join("blocker").join("settings.json");
    let mut app = editor_app(path.clone()).headless()?;
    let (ui, editor) = app.runtime_and_state_mut();
    let mut driver = Driver::new(ui);
    driver.settle()?;
    let quality = at(&driver, editor, QUALITY)?;
    driver.replace_text(&quality, "42")?;
    driver.key(KeyCode::Enter)?;
    let apply = at(&driver, editor, APPLY)?;
    driver.click(&apply)?;
    driver.wait_until(Duration::from_secs(5), |_| Ok(!editor.saving.get()?))?;

    let message = name(&driver, editor, MESSAGE)?;
    assert!(message.starts_with("Save failed"), "{message}");
    assert!(message.contains("settings.json"), "{message}");
    assert!(!path.exists());
    assert_eq!(editor.applied.get()?, Settings::default());
    assert_eq!(number(&driver, editor, QUALITY)?, 42.0, "the edit is kept");
    assert_eq!(buttons(&driver, editor)?, (true, true), "still dirty");
    assert_eq!(name(&driver, editor, DIRTY)?, "● Unsaved changes");
    drop(driver);
    let main = app.main_window();
    assert!(matches!(app.admit_close(&main)?, NativeCloseAction::KeepOpen));
    app.shutdown()?;
    Ok(())
}

/// Acceptance 5: closing is refused while dirty and asks to apply or revert.
#[test]
fn closing_is_refused_while_dirty() -> TestResult {
    let dir = scratch("close");
    let (_, mut app) = start(&dir, &sample())?;
    let main = app.main_window();
    assert!(matches!(app.admit_close(&main)?, NativeCloseAction::Close), "clean closes");

    // A changed value.
    let (ui, editor) = app.runtime_and_state_mut();
    let mut driver = Driver::new(ui);
    let toggle = at(&driver, editor, OPEN_FOLDER)?;
    driver.click(&toggle)?;
    drop(driver);
    assert!(matches!(app.admit_close(&main)?, NativeCloseAction::KeepOpen));
    let (ui, editor) = app.runtime_and_state_mut();
    let mut driver = Driver::new(ui);
    driver.settle()?;
    let message = name(&driver, editor, MESSAGE)?;
    assert!(message.contains("apply or revert"), "{message}");

    // A typed draft still in the focused field: admission saves it first.
    let revert = at(&driver, editor, REVERT)?;
    driver.click(&revert)?;
    let quality = at(&driver, editor, QUALITY)?;
    driver.replace_text(&quality, "33")?;
    assert_eq!(editor.quality.get()?, 70, "still a draft");
    drop(driver);
    assert!(matches!(app.admit_close(&main)?, NativeCloseAction::KeepOpen));
    assert_eq!(app.state().quality.get()?, 33);

    // A refused draft over the applied value.
    let (ui, editor) = app.runtime_and_state_mut();
    let mut driver = Driver::new(ui);
    driver.click(&revert)?;
    driver.replace_text(&quality, "1000")?;
    driver.key(KeyCode::Enter)?;
    assert_eq!(editor.quality.get()?, 70);
    drop(driver);
    assert!(matches!(app.admit_close(&main)?, NativeCloseAction::KeepOpen));

    // Reverted: closing is allowed.
    let (ui, _) = app.runtime_and_state_mut();
    let mut driver = Driver::new(ui);
    driver.click(&revert)?;
    drop(driver);
    assert!(matches!(app.admit_close(&main)?, NativeCloseAction::Close));
    app.shutdown()?;
    Ok(())
}

/// The format dropdown, driven by keys, takes part in the dirty state.
#[test]
fn choosing_a_format_makes_the_editor_dirty() -> TestResult {
    let dir = scratch("format");
    let (_, mut app) = start(&dir, &sample())?;
    let (ui, editor) = app.runtime_and_state_mut();
    let mut driver = Driver::new(ui);
    let select = at(&driver, editor, FORMAT)?;
    let anchor = driver.runtime().find_descendant(&select, "select")?.ok_or("no dropdown anchor")?;
    driver.runtime_mut().focus(&anchor)?;
    driver.key(KeyCode::Enter)?; // opens the list at WebP
    driver.key(KeyCode::ArrowUp)?; // highlights JPEG
    driver.key(KeyCode::Enter)?; // chooses it
    assert_eq!(editor.format.get()?, Format::Jpeg);
    assert!(matches!(
        format_semantics(&driver, editor)?.value,
        Some(SemanticValue::Choice { label: Some(ref label), .. }) if label == "JPEG"
    ));
    assert_eq!(buttons(&driver, editor)?, (true, true));
    let revert = at(&driver, editor, REVERT)?;
    driver.click(&revert)?;
    assert_eq!(editor.format.get()?, Format::WebP);
    assert_eq!(buttons(&driver, editor)?, (false, false));
    drop(driver);
    app.shutdown()?;
    Ok(())
}
