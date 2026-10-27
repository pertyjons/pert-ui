//! Headless checks of the settings editor, driven through pert-ui's own
//! testing facilities: `App::headless`, `Driver` input and semantics.

use pert_ui::prelude::*;
use pert_ui::testing::*;
use pert_ui::{InspectValue, Semantics};
use settings_editor::{settings_app, CLOSE_REFUSED_MESSAGE};
use std::{path::PathBuf, time::Duration};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("settings-editor-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

const SAVED: &str = r#"{
  "prefix": "holiday",
  "format": "WebP",
  "jpeg_quality": 70,
  "max_width": 4096,
  "open_folder": true
}"#;

fn start(path: PathBuf) -> Result<(HeadlessApp, WidgetId), Box<dyn std::error::Error>> {
    let (app, built) = settings_app(path)?;
    let app = app.headless()?;
    let root = built.borrow().as_ref().ok_or("not built")?.root.clone();
    Ok((app, root))
}

fn id(driver: &Driver, root: &WidgetId, key: &str) -> Result<WidgetId, Box<dyn std::error::Error>> {
    Ok(driver.runtime().find_descendant(root, key)?.ok_or(format!("no {key}"))?)
}

fn sem(driver: &Driver, root: &WidgetId, key: &str) -> Result<Semantics, Box<dyn std::error::Error>> {
    Ok(driver.semantics(&id(driver, root, key)?)?)
}

fn name(driver: &Driver, root: &WidgetId, key: &str) -> Result<String, Box<dyn std::error::Error>> {
    Ok(sem(driver, root, key)?.name.unwrap_or_default())
}

#[test]
fn shows_loaded_values() -> TestResult {
    let dir = scratch("loaded");
    let path = dir.join("settings.json");
    std::fs::write(&path, SAVED)?;
    let (mut app, root) = start(path)?;
    let driver = Driver::new(app.runtime_mut());
    assert_eq!(name(&driver, &root, "output")?, "Output");
    assert_eq!(name(&driver, &root, "quality")?, "Quality");
    assert_eq!(name(&driver, &root, "behavior")?, "Behavior");
    assert_eq!(sem(&driver, &root, "prefix")?.value, Some(SemanticValue::Text("holiday".into())));
    assert_eq!(name(&driver, &root, "prefix")?, "File name prefix");
    let format = sem(&driver, &root, "format")?;
    assert_eq!(format.name.as_deref(), Some("Format"));
    assert!(matches!(format.value, Some(SemanticValue::Choice { label: Some(ref l), count: 3, .. }) if l == "WebP"));
    let names: Vec<_> = format.items.iter().filter_map(|i| i.name.clone()).collect();
    assert_eq!(names, ["PNG", "JPEG", "WebP"]);
    assert_eq!(name(&driver, &root, "jpeg-quality")?, "JPEG quality");
    assert_eq!(number(&driver, &root, "jpeg-quality")?, 70.0);
    assert_eq!(name(&driver, &root, "max-width")?, "Maximum width");
    assert_eq!(number(&driver, &root, "max-width")?, 4096.0);
    // At rest the width shows its display text followed by its unit.
    let width = id(&driver, &root, "max-width")?;
    let iid = driver.runtime().inspection_id(&width)?;
    let found = driver.runtime().query(&InspectionQuery::components(Selector::id(iid)).detail(Detail::Full));
    let props = &found.components[0].details.properties;
    assert_eq!(props.get("unit"), Some(&InspectValue::Text("px".into())));
    assert_eq!(props.get("display_value"), Some(&InspectValue::Text("4096".into())));
    assert_eq!(props.get("shows_display"), Some(&InspectValue::Bool(true)));
    let toggle = sem(&driver, &root, "open-folder")?;
    assert_eq!(toggle.name.as_deref(), Some("Open folder after export"));
    assert_eq!(toggle.value, Some(SemanticValue::Bool(true)));
    assert_eq!(name(&driver, &root, "apply")?, "Apply");
    assert_eq!(name(&driver, &root, "revert")?, "Revert");
    assert_eq!(name(&driver, &root, "dirty-marker")?, "No unsaved changes");
    app.shutdown()?;
    Ok(())
}

fn number(driver: &Driver, root: &WidgetId, key: &str) -> Result<f64, Box<dyn std::error::Error>> {
    match sem(driver, root, key)?.value {
        Some(SemanticValue::Number { value, .. }) => Ok(value),
        other => Err(format!("{key} shows {other:?}").into()),
    }
}

fn text(driver: &Driver, root: &WidgetId, key: &str) -> Result<String, Box<dyn std::error::Error>> {
    match sem(driver, root, key)?.value {
        Some(SemanticValue::Text(t)) => Ok(t),
        other => Err(format!("{key} shows {other:?}").into()),
    }
}

fn enabled(driver: &Driver, root: &WidgetId, key: &str) -> Result<bool, Box<dyn std::error::Error>> {
    Ok(!sem(driver, root, key)?.disabled)
}

/// Types `value` into the field and presses Enter to save it.
fn enter(driver: &mut Driver, root: &WidgetId, key: &str, value: &str) -> TestResult {
    let field = id(driver, root, key)?;
    driver.replace_text(&field, value)?;
    driver.key(KeyCode::Enter)?;
    Ok(())
}

#[test]
fn refuses_invalid_input_and_keeps_old_value() -> TestResult {
    let dir = scratch("invalid");
    let path = dir.join("settings.json");
    std::fs::write(&path, SAVED)?;
    let (mut app, root) = start(path)?;
    let mut driver = Driver::new(app.runtime_mut());

    // An invalid prefix: refused with its explanation; the applied value stays.
    enter(&mut driver, &root, "prefix", "bad name!")?;
    let prefix = sem(&driver, &root, "prefix")?;
    eprintln!("prefix after refusal: {prefix:?}");
    assert!(prefix.invalid);
    assert!(prefix.description.as_deref().unwrap_or("").contains("not allowed"), "{prefix:?}");
    assert_eq!(name(&driver, &root, "dirty-marker")?, "No unsaved changes");
    assert!(!enabled(&driver, &root, "apply")?);
    // Too long, too.
    enter(&mut driver, &root, "prefix", &"a".repeat(33))?;
    assert!(sem(&driver, &root, "prefix")?.description.unwrap_or_default().contains("at most 32"));
    // Escape drops the refused draft: the field shows the value in effect again.
    driver.key(KeyCode::Escape)?;
    assert_eq!(text(&driver, &root, "prefix")?, "holiday");
    assert!(!sem(&driver, &root, "prefix")?.invalid);

    // An out-of-range quality: refused with its explanation; 70 stays.
    enter(&mut driver, &root, "jpeg-quality", "150")?;
    let quality = sem(&driver, &root, "jpeg-quality")?;
    eprintln!("quality after refusal: {quality:?}");
    assert!(quality.invalid);
    assert!(quality.description.is_some_and(|d| d != "1 to 100"));
    assert_eq!(name(&driver, &root, "dirty-marker")?, "No unsaved changes");
    assert!(!enabled(&driver, &root, "apply")?);
    driver.key(KeyCode::Escape)?;
    assert_eq!(number(&driver, &root, "jpeg-quality")?, 70.0);

    // A valid value is accepted and clears the explanation.
    enter(&mut driver, &root, "prefix", "trip_2026")?;
    let prefix = sem(&driver, &root, "prefix")?;
    assert!(!prefix.invalid);
    assert_eq!(prefix.value, Some(SemanticValue::Text("trip_2026".into())));
    assert_eq!(prefix.description.as_deref(), Some("1 to 32 letters, digits, - or _"));
    drop(driver);
    app.shutdown()?;
    Ok(())
}

#[test]
fn apply_and_revert_follow_dirty_state() -> TestResult {
    let dir = scratch("dirty");
    let path = dir.join("settings.json");
    std::fs::write(&path, SAVED)?;
    let (mut app, root) = start(path)?;
    let mut driver = Driver::new(app.runtime_mut());

    assert!(!enabled(&driver, &root, "apply")?);
    assert!(!enabled(&driver, &root, "revert")?);
    assert_eq!(name(&driver, &root, "dirty-marker")?, "No unsaved changes");

    enter(&mut driver, &root, "jpeg-quality", "90")?;
    let toggle = id(&driver, &root, "open-folder")?;
    driver.click(&toggle)?;
    enter(&mut driver, &root, "prefix", "edited")?;
    assert_eq!(number(&driver, &root, "jpeg-quality")?, 90.0);
    assert_eq!(sem(&driver, &root, "open-folder")?.value, Some(SemanticValue::Bool(false)));
    assert!(enabled(&driver, &root, "apply")?);
    assert!(enabled(&driver, &root, "revert")?);
    assert_eq!(name(&driver, &root, "dirty-marker")?, "● Unsaved changes");

    let revert = id(&driver, &root, "revert")?;
    driver.click(&revert)?;
    assert_eq!(number(&driver, &root, "jpeg-quality")?, 70.0);
    assert_eq!(text(&driver, &root, "prefix")?, "holiday");
    assert_eq!(sem(&driver, &root, "open-folder")?.value, Some(SemanticValue::Bool(true)));
    assert!(!enabled(&driver, &root, "apply")?);
    assert!(!enabled(&driver, &root, "revert")?);
    assert_eq!(name(&driver, &root, "dirty-marker")?, "No unsaved changes");

    // Editing back to the applied value is clean again.
    enter(&mut driver, &root, "jpeg-quality", "71")?;
    assert!(enabled(&driver, &root, "apply")?);
    enter(&mut driver, &root, "jpeg-quality", "70")?;
    assert!(!enabled(&driver, &root, "apply")?);
    drop(driver);
    app.shutdown()?;
    Ok(())
}

fn wait_for_status(driver: &mut Driver, root: &WidgetId, prefix: &str) -> TestResult {
    let status = id(driver, root, "status")?;
    let wanted = prefix.to_owned();
    let result = driver.wait_until(Duration::from_secs(5), move |ui| {
        let id = ui.inspection_id(&status)?;
        let found = ui.query(&InspectionQuery::components(Selector::id(id)).detail(Detail::Semantics));
        Ok(found.components.first()
            .and_then(|c| c.details.semantics.as_ref())
            .and_then(|s| s.name.as_deref())
            .is_some_and(|n| n.starts_with(&wanted)))
    });
    if result.is_err() {
        return Err(format!("status is {:?}", name(driver, root, "status")?).into());
    }
    Ok(())
}

#[test]
fn apply_saves_in_background() -> TestResult {
    let dir = scratch("save");
    let path = dir.join("settings.json");
    // No file yet: the defaults are shown.
    let (mut app, root) = start(path.clone())?;
    let mut driver = Driver::new(app.runtime_mut());
    assert_eq!(text(&driver, &root, "prefix")?, "export");

    enter(&mut driver, &root, "prefix", "summer")?;
    enter(&mut driver, &root, "max-width", "1024")?;
    let apply = id(&driver, &root, "apply")?;
    driver.click(&apply)?;
    wait_for_status(&mut driver, &root, "Saved to")?;
    let saved: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&path)?)?;
    assert_eq!(saved["prefix"], "summer");
    assert_eq!(saved["max_width"], 1024);
    assert_eq!(saved["format"], "JPEG");
    assert!(!enabled(&driver, &root, "apply")?);
    assert!(!enabled(&driver, &root, "revert")?);
    assert_eq!(name(&driver, &root, "dirty-marker")?, "No unsaved changes");
    drop(driver);
    app.shutdown()?;

    // A second start loads what was saved.
    let (mut app, root) = start(path)?;
    let driver = Driver::new(app.runtime_mut());
    assert_eq!(text(&driver, &root, "prefix")?, "summer");
    assert_eq!(number(&driver, &root, "max-width")?, 1024.0);
    drop(driver);
    app.shutdown()?;
    Ok(())
}

#[test]
fn failed_save_is_reported_and_stays_dirty() -> TestResult {
    let dir = scratch("fail");
    // The settings file's directory is a regular file: writing fails.
    let blocker = dir.join("not-a-directory");
    std::fs::write(&blocker, "")?;
    let path = blocker.join("settings.json");
    let (mut app, root) = start(path)?;
    let mut driver = Driver::new(app.runtime_mut());

    enter(&mut driver, &root, "jpeg-quality", "55")?;
    let apply = id(&driver, &root, "apply")?;
    driver.click(&apply)?;
    wait_for_status(&mut driver, &root, "Save failed")?;
    eprintln!("status: {}", name(&driver, &root, "status")?);
    assert!(enabled(&driver, &root, "apply")?);
    assert!(enabled(&driver, &root, "revert")?);
    assert_eq!(name(&driver, &root, "dirty-marker")?, "● Unsaved changes");
    assert_eq!(number(&driver, &root, "jpeg-quality")?, 55.0);
    drop(driver);
    app.shutdown()?;
    Ok(())
}

#[test]
fn closing_is_refused_while_dirty() -> TestResult {
    let dir = scratch("close");
    let path = dir.join("settings.json");
    std::fs::write(&path, SAVED)?;
    let (mut app, root) = start(path)?;
    let main = app.main_window();

    // Clean: closing is admitted.
    assert!(matches!(app.admit_close(&main)?, NativeCloseAction::Close));

    let mut driver = Driver::new(app.runtime_mut());
    // A focused draft the user typed but did not confirm counts too: admission saves it.
    let field = id(&driver, &root, "prefix")?;
    driver.replace_text(&field, "unsaved")?;
    drop(driver);
    assert!(matches!(app.admit_close(&main)?, NativeCloseAction::KeepOpen));
    let mut driver = Driver::new(app.runtime_mut());
    driver.settle()?;
    assert_eq!(name(&driver, &root, "close-notice")?, CLOSE_REFUSED_MESSAGE);
    assert_eq!(name(&driver, &root, "dirty-marker")?, "● Unsaved changes");

    // After reverting, the notice goes away and closing is admitted.
    let revert = id(&driver, &root, "revert")?;
    driver.click(&revert)?;
    assert_eq!(name(&driver, &root, "close-notice")?, "");
    drop(driver);
    assert!(matches!(app.admit_close(&main)?, NativeCloseAction::Close));
    app.shutdown()?;
    Ok(())
}

/// Renders the window on the CPU and writes it next to the build output, for a
/// person to look at.
fn capture(driver: &mut Driver, file: &str) -> TestResult {
    driver.runtime().set_capture_support(true);
    let pending = driver.runtime().request_capture(CaptureRequest::new(
        CaptureTarget::window(&driver.runtime().primary_window()),
    ));
    driver.settle()?;
    CpuRenderer::new().serve_captures(driver.runtime_mut());
    let capture = pending.wait()?.value?;
    let out = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(file);
    std::fs::write(&out, capture.image.encode_png())?;
    eprintln!("wrote {}", out.display());
    Ok(())
}

#[test]
fn captures_for_review() -> TestResult {
    let dir = scratch("capture");
    let path = dir.join("settings.json");
    std::fs::write(&path, SAVED)?;
    let (mut app, root) = start(path)?;
    let mut driver = Driver::new(app.runtime_mut());
    capture(&mut driver, "clean.png")?;
    enter(&mut driver, &root, "prefix", "bad name!")?;
    enter(&mut driver, &root, "max-width", "8")?;
    enter(&mut driver, &root, "jpeg-quality", "95")?;
    capture(&mut driver, "dirty-with-errors.png")?;
    drop(driver);
    let main = app.main_window();
    assert!(matches!(app.admit_close(&main)?, NativeCloseAction::KeepOpen));
    capture(&mut Driver::new(app.runtime_mut()), "close-refused.png")?;
    app.shutdown()?;
    Ok(())
}
