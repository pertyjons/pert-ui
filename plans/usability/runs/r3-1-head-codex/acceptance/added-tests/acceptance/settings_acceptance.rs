//! Independent acceptance checks for task 1 (settings editor), added by the
//! acceptance verifier in the acceptance copy only.
//!
//! The application is started with its own `app(path)` constructor through
//! `App::headless`, driven with `pert_ui::testing::Driver` (text, keys,
//! clicks) and observed through the library's semantics and inspection
//! snapshot, the settings file on disk, and `HeadlessApp::admit_close`.
//! The only application value used is `Editor::root`, the mounted root
//! `WidgetId`, to address widgets; no application signal is read or written.

use super::app as start_app;
use pert_ui::prelude::*;
use pert_ui::testing::*;
use std::io::Read;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};
use std::{fs, thread};

type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;

const LOADED: &str = r#"{
  "prefix": "cam-01_X",
  "format": "PNG",
  "quality": 33,
  "max_width": 16384,
  "open_folder": true
}"#;

fn dir(name: &str) -> Result<PathBuf> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("acceptance-output")
        .join(name);
    if dir.exists() {
        fs::remove_dir_all(&dir)?;
    }
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

fn with_file(name: &str, contents: &str) -> Result<PathBuf> {
    let path = dir(name)?.join("settings.json");
    fs::write(&path, contents)?;
    Ok(path)
}

fn root_of(app: &HeadlessApp<super::Editor>) -> WidgetId {
    app.state().root.clone()
}

fn sem(d: &Driver<'_>, r: &WidgetId, path: &[&str]) -> Result<Semantics> {
    Ok(d.semantics(&d.find(r, path)?)?)
}
fn name(d: &Driver<'_>, r: &WidgetId, path: &[&str]) -> Result<String> {
    Ok(sem(d, r, path)?.name.unwrap_or_default())
}
fn text(d: &Driver<'_>, id: &WidgetId) -> Result<String> {
    match d.semantics(id)?.value {
        Some(SemanticValue::Text(t)) => Ok(t),
        other => Err(format!("not text: {other:?}").into()),
    }
}
fn number(d: &Driver<'_>, id: &WidgetId) -> Result<f64> {
    match d.semantics(id)?.value {
        Some(SemanticValue::Number { value, .. }) => Ok(value),
        other => Err(format!("not a number: {other:?}").into()),
    }
}
fn choice(d: &Driver<'_>, id: &WidgetId) -> Result<Option<usize>> {
    match d.semantics(id)?.value {
        Some(SemanticValue::Choice { selected, .. }) => Ok(selected),
        other => Err(format!("not a choice: {other:?}").into()),
    }
}
fn toggle(d: &Driver<'_>, id: &WidgetId) -> Result<bool> {
    match d.semantics(id)?.value {
        Some(SemanticValue::Bool(b)) => Ok(b),
        other => Err(format!("not a bool: {other:?}").into()),
    }
}
/// (apply enabled, revert enabled), read from semantics.
fn buttons(d: &Driver<'_>, r: &WidgetId) -> Result<(bool, bool)> {
    Ok((
        !sem(d, r, &["buttons", "apply"])?.disabled,
        !sem(d, r, &["buttons", "revert"])?.disabled,
    ))
}
fn marker(d: &Driver<'_>, r: &WidgetId) -> Result<String> {
    name(d, r, &["dirty"])
}
fn message(d: &Driver<'_>, r: &WidgetId) -> Result<String> {
    name(d, r, &["message"])
}

/// The inspection record whose addressable path ends with `suffix`.
fn component<'a>(
    snapshot: &'a pert_ui::inspection::InspectionSnapshot,
    suffix: &[&str],
) -> Result<&'a ComponentInspection> {
    snapshot
        .components
        .iter()
        .find(|c| {
            c.path.as_ref().is_some_and(|p| {
                p.len() >= suffix.len()
                    && p[p.len() - suffix.len()..]
                        .iter()
                        .zip(suffix)
                        .all(|(a, b)| a == b)
            })
        })
        .ok_or_else(|| format!("no component at {suffix:?}").into())
}
/// The status line a `Field` draws below its control, from inspection.
fn field_message(ui: &Runtime, suffix: &[&str]) -> Result<Option<String>> {
    let snapshot = ui.inspect();
    let field = component(&snapshot, suffix)?;
    Ok(match field.details.properties.get("message") {
        Some(InspectValue::Text(t)) if !t.is_empty() => Some(t.clone()),
        _ => None,
    })
}
/// Label text read from the inspection snapshot, usable inside `wait_until`.
fn label_in(ui: &Runtime, suffix: &[&str]) -> String {
    let snapshot = ui.inspect();
    component(&snapshot, suffix)
        .ok()
        .and_then(|c| c.details.semantics.as_ref())
        .and_then(|s| s.name.clone())
        .unwrap_or_default()
}

fn disk(path: &PathBuf) -> Result<serde_json::Value> {
    Ok(serde_json::from_slice(&fs::read(path)?)?)
}

#[test]
fn discovery_prints_addressable_paths() -> Result {
    let path = with_file("discovery", LOADED)?;
    let mut app = start_app(path).headless()?;
    let (ui, _) = app.runtime_and_state_mut();
    Driver::new(ui).settle()?;
    let snapshot = app.runtime().inspect();
    for c in &snapshot.components {
        if let Some(p) = &c.path {
            let s = c.details.semantics.as_ref();
            println!(
                "{:?} role={:?} name={:?} props={:?}",
                p,
                s.map(|s| s.role),
                s.and_then(|s| s.name.clone()),
                c.details.properties.keys().collect::<Vec<_>>()
            );
        }
    }
    app.shutdown()?;
    Ok(())
}

/// Item 1: groups, fields and buttons exist and show the loaded file's values;
/// startup without a file shows defaults and creates nothing.
#[test]
fn item1_structure_and_loaded_values() -> Result {
    let path = with_file("item1", LOADED)?;
    let mut app = start_app(path).headless()?;
    let r = root_of(&app);
    let (ui, _) = app.runtime_and_state_mut();
    let mut d = Driver::new(ui);
    d.settle()?;
    assert_eq!(name(&d, &r, &["output", "title"])?, "Output");
    assert_eq!(name(&d, &r, &["quality-group", "title"])?, "Quality");
    assert_eq!(name(&d, &r, &["behavior", "title"])?, "Behavior");

    let p = d.find(&r, &["output", "prefix", "input"])?;
    let f = d.find(&r, &["output", "format", "input"])?;
    let q = d.find(&r, &["quality-group", "quality", "input"])?;
    let w = d.find(&r, &["quality-group", "width", "row", "input"])?;
    let t = d.find(&r, &["behavior", "open"])?;
    let ps = d.semantics(&p)?;
    println!("prefix semantics: {ps:?}");
    assert_eq!(ps.role, SemanticRole::TextInput);
    assert_eq!(ps.name.as_deref(), Some("File name prefix"));
    assert_eq!(text(&d, &p)?, "cam-01_X");
    let fs_ = d.semantics(&f)?;
    println!("format semantics: {fs_:?}");
    assert_eq!(fs_.name.as_deref(), Some("Format"));
    let items: Vec<_> = fs_.items.iter().map(|i| i.name.clone().unwrap_or_default()).collect();
    println!("format items: {items:?}");
    assert_eq!(choice(&d, &f)?, Some(0), "PNG loaded");
    let qs = d.semantics(&q)?;
    println!("quality semantics: {qs:?}");
    assert!(qs.name.as_deref().unwrap_or("").starts_with("JPEG quality"));
    assert_eq!(number(&d, &q)?, 33.0);
    let ws = d.semantics(&w)?;
    println!("width semantics: {ws:?}");
    // The Field wraps a Row (control + unit), so its label names the Field,
    // not the NumberInput itself (recorded as an accessibility observation).
    assert_eq!(name(&d, &r, &["quality-group", "width"])?, "Maximum width");
    assert_eq!(number(&d, &w)?, 16384.0);
    assert_eq!(name(&d, &r, &["quality-group", "width", "row", "unit"])?, "px");
    let ts = d.semantics(&t)?;
    println!("toggle semantics: {ts:?}");
    assert_eq!(ts.name.as_deref(), Some("Open folder after export"));
    assert!(toggle(&d, &t)?);
    for (key, label) in [("apply", "Apply"), ("revert", "Revert")] {
        let s = sem(&d, &r, &["buttons", key])?;
        assert_eq!(s.role, SemanticRole::Button);
        assert_eq!(s.name.as_deref(), Some(label));
    }
    assert_eq!(buttons(&d, &r)?, (false, false));
    assert_eq!(marker(&d, &r)?, "All changes saved");
    println!("startup message: {:?}", message(&d, &r)?);
    app.shutdown()?;

    // A second file with other values, to rule out matching defaults.
    let path = with_file(
        "item1-b",
        r#"{"prefix":"z","format":"JPEG","quality":1,"max_width":16,"open_folder":false}"#,
    )?;
    let mut app = start_app(path).headless()?;
    let r = root_of(&app);
    let (ui, _) = app.runtime_and_state_mut();
    let mut d = Driver::new(ui);
    d.settle()?;
    assert_eq!(text(&d, &d.find(&r, &["output", "prefix", "input"])?)?, "z");
    assert_eq!(choice(&d, &d.find(&r, &["output", "format", "input"])?)?, Some(1));
    assert_eq!(number(&d, &d.find(&r, &["quality-group", "quality", "input"])?)?, 1.0);
    assert_eq!(number(&d, &d.find(&r, &["quality-group", "width", "row", "input"])?)?, 16.0);
    assert!(!toggle(&d, &d.find(&r, &["behavior", "open"])?)?);
    app.shutdown()?;

    // No file: defaults shown, nothing written at startup.
    let missing = dir("item1-missing")?.join("settings.json");
    let mut app = start_app(missing.clone()).headless()?;
    let r = root_of(&app);
    let (ui, _) = app.runtime_and_state_mut();
    let mut d = Driver::new(ui);
    d.settle()?;
    println!(
        "defaults: prefix={:?} format={:?} quality={:?} width={:?} open={:?} message={:?}",
        text(&d, &d.find(&r, &["output", "prefix", "input"])?)?,
        choice(&d, &d.find(&r, &["output", "format", "input"])?)?,
        number(&d, &d.find(&r, &["quality-group", "quality", "input"])?)?,
        number(&d, &d.find(&r, &["quality-group", "width", "row", "input"])?)?,
        toggle(&d, &d.find(&r, &["behavior", "open"])?)?,
        message(&d, &r)?
    );
    assert!(!missing.exists());
    app.shutdown()?;
    Ok(())
}

/// Item 2: malformed and out-of-range input is refused through the field,
/// the explanation is shown by the field, and the last valid value stays in
/// effect (Escape shows it again, and Apply writes it to disk).
#[test]
fn item2_refusals_keep_last_valid_value() -> Result {
    let path = with_file("item2", LOADED)?;
    let mut app = start_app(path.clone()).headless()?;
    let r = root_of(&app);
    let (ui, _) = app.runtime_and_state_mut();
    let mut d = Driver::new(ui);
    d.settle()?;
    let p = d.find(&r, &["output", "prefix", "input"])?;
    let q = d.find(&r, &["quality-group", "quality", "input"])?;
    let w = d.find(&r, &["quality-group", "width", "row", "input"])?;

    let long = "a".repeat(33);
    for bad in ["bad prefix!", "dot.name", "ümlaut", long.as_str(), ""] {
        d.replace_text(&p, bad)?;
        d.key(KeyCode::Enter)?;
        let s = d.semantics(&p)?;
        let shown = field_message(d.runtime(), &["output", "prefix"])?;
        println!("prefix {bad:?}: invalid={} description={:?} field message={:?}", s.invalid, s.description, shown);
        assert!(s.invalid, "{bad:?} refused");
        assert!(shown.as_deref().is_some_and(|m| m.contains("1–32")), "{bad:?} explained");
        assert_eq!(s.description, shown);
        d.key(KeyCode::Escape)?;
        assert_eq!(text(&d, &p)?, "cam-01_X", "old prefix still in effect after {bad:?}");
    }
    for bad in ["0", "101", "-5", "abc", "50.5", ""] {
        d.replace_text(&q, bad)?;
        d.key(KeyCode::Enter)?;
        let s = d.semantics(&q)?;
        let shown = field_message(d.runtime(), &["quality-group", "quality"])?;
        println!("quality {bad:?}: invalid={} description={:?} field message={:?}", s.invalid, s.description, shown);
        assert!(s.invalid, "{bad:?} refused");
        assert!(shown.is_some(), "{bad:?} explained");
        d.key(KeyCode::Escape)?;
        assert_eq!(number(&d, &q)?, 33.0, "old quality still in effect after {bad:?}");
    }
    for bad in ["15", "16385"] {
        d.replace_text(&w, bad)?;
        d.key(KeyCode::Enter)?;
        let s = d.semantics(&w)?;
        let shown = field_message(d.runtime(), &["quality-group", "width"])?;
        println!("width {bad:?}: invalid={} description={:?} field message={:?}", s.invalid, s.description, shown);
        assert!(s.invalid, "{bad:?} refused");
        assert!(shown.is_some(), "{bad:?} explained");
        d.key(KeyCode::Escape)?;
        assert_eq!(number(&d, &w)?, 16384.0);
    }

    // Leave refused drafts in place and apply: the file gets the old values.
    d.replace_text(&p, "bad prefix!")?;
    d.key(KeyCode::Enter)?;
    d.replace_text(&q, "101")?;
    d.key(KeyCode::Enter)?;
    assert!(d.semantics(&p)?.invalid && d.semantics(&q)?.invalid);
    let (apply_enabled, _) = buttons(&d, &r)?;
    println!("apply enabled with only refused drafts: {apply_enabled}");
    if apply_enabled {
        d.click(&d.find(&r, &["buttons", "apply"])?)?;
        d.wait_until(Duration::from_secs(5), |ui| {
            Ok(label_in(ui, &["message"]) == "Settings saved")
        })?;
        let saved = disk(&path)?;
        println!("disk after applying with refused drafts: {saved}");
        assert_eq!(saved["prefix"], "cam-01_X");
        assert_eq!(saved["quality"], 33);
        // The refused drafts are still shown for correction.
        assert_eq!(text(&d, &p)?, "bad prefix!");
        assert!(d.semantics(&p)?.invalid);
    }
    app.shutdown()?;
    Ok(())
}

/// Item 3: Apply/Revert disabled when clean, enabled when dirty (from
/// semantics); Revert restores applied values after a valid change and a
/// later refused draft; returning to the applied value makes it clean.
#[test]
fn item3_dirty_state_and_revert() -> Result {
    let path = with_file("item3", LOADED)?;
    let mut app = start_app(path.clone()).headless()?;
    let r = root_of(&app);
    let (ui, _) = app.runtime_and_state_mut();
    let mut d = Driver::new(ui);
    d.settle()?;
    let p = d.find(&r, &["output", "prefix", "input"])?;
    let f = d.find(&r, &["output", "format", "input"])?;
    let q = d.find(&r, &["quality-group", "quality", "input"])?;
    let t = d.find(&r, &["behavior", "open"])?;
    let revert = d.find(&r, &["buttons", "revert"])?;
    let apply = d.find(&r, &["buttons", "apply"])?;
    assert_eq!(buttons(&d, &r)?, (false, false));
    assert_eq!(marker(&d, &r)?, "All changes saved");

    // Valid change makes the form dirty.
    d.replace_text(&q, "60")?;
    d.key(KeyCode::Enter)?;
    assert!(!d.semantics(&q)?.invalid);
    assert_eq!(number(&d, &q)?, 60.0);
    assert_eq!(buttons(&d, &r)?, (true, true));
    assert_eq!(marker(&d, &r)?, "Unsaved changes");
    // Then a refused draft in another field.
    d.replace_text(&p, "bad prefix!")?;
    d.key(KeyCode::Enter)?;
    assert!(d.semantics(&p)?.invalid);
    assert_eq!(buttons(&d, &r)?, (true, true));
    d.click(&revert)?;
    assert_eq!(text(&d, &p)?, "cam-01_X");
    assert!(!d.semantics(&p)?.invalid);
    assert_eq!(field_message(d.runtime(), &["output", "prefix"])?, None);
    assert_eq!(number(&d, &q)?, 33.0);
    assert_eq!(buttons(&d, &r)?, (false, false));
    assert_eq!(marker(&d, &r)?, "All changes saved");
    println!("after revert (valid change + refused draft): message={:?}", message(&d, &r)?);

    // Format and toggle changes, reverted.
    // Driver::click_item refuses SegmentedControl in 0.3.88 (ItemWithoutBounds),
    // so the choice is made with the control's own keyboard input.
    d.runtime_mut().focus(&f)?;
    d.key(KeyCode::End)?;
    assert_eq!(choice(&d, &f)?, Some(2));
    d.click(&t)?;
    assert!(!toggle(&d, &t)?);
    assert_eq!(buttons(&d, &r)?, (true, true));
    d.click(&revert)?;
    assert_eq!(choice(&d, &f)?, Some(0));
    assert!(toggle(&d, &t)?);
    assert_eq!(buttons(&d, &r)?, (false, false));

    // Editing back to the applied value is clean again.
    d.replace_text(&q, "34")?;
    d.key(KeyCode::Enter)?;
    assert_eq!(buttons(&d, &r)?, (true, true));
    d.replace_text(&q, "33")?;
    d.key(KeyCode::Enter)?;
    assert_eq!(buttons(&d, &r)?, (false, false));
    assert_eq!(marker(&d, &r)?, "All changes saved");

    // Revert goes to the newly applied values, not the loaded ones.
    d.replace_text(&q, "70")?;
    d.key(KeyCode::Enter)?;
    d.click(&apply)?;
    d.wait_until(Duration::from_secs(5), |ui| {
        Ok(label_in(ui, &["message"]) == "Settings saved")
    })?;
    assert_eq!(buttons(&d, &r)?, (false, false));
    d.replace_text(&q, "80")?;
    d.key(KeyCode::Enter)?;
    d.replace_text(&p, "also bad!")?;
    d.key(KeyCode::Enter)?;
    d.click(&revert)?;
    assert_eq!(number(&d, &q)?, 70.0);
    assert_eq!(text(&d, &p)?, "cam-01_X");
    assert_eq!(buttons(&d, &r)?, (false, false));
    assert_eq!(disk(&path)?["quality"], 70);
    app.shutdown()?;
    Ok(())
}

/// Item 4 (a): Apply writes every edited field to the file and clears dirty.
#[test]
fn item4_successful_save_writes_file() -> Result {
    let path = with_file("item4-ok", LOADED)?;
    let mut app = start_app(path.clone()).headless()?;
    let r = root_of(&app);
    let (ui, _) = app.runtime_and_state_mut();
    let mut d = Driver::new(ui);
    d.settle()?;
    d.replace_text(&d.find(&r, &["output", "prefix", "input"])?, "new_name-2")?;
    d.key(KeyCode::Enter)?;
    let f = d.find(&r, &["output", "format", "input"])?;
    d.runtime_mut().focus(&f)?;
    d.key(KeyCode::ArrowRight)?;
    assert_eq!(choice(&d, &f)?, Some(1));
    d.replace_text(&d.find(&r, &["quality-group", "quality", "input"])?, "100")?;
    d.key(KeyCode::Enter)?;
    d.replace_text(&d.find(&r, &["quality-group", "width", "row", "input"])?, "16")?;
    d.key(KeyCode::Enter)?;
    d.click(&d.find(&r, &["behavior", "open"])?)?;
    assert_eq!(buttons(&d, &r)?, (true, true));
    d.click(&d.find(&r, &["buttons", "apply"])?)?;
    d.wait_until(Duration::from_secs(5), |ui| {
        Ok(label_in(ui, &["message"]) == "Settings saved")
    })?;
    let saved = disk(&path)?;
    println!("disk after apply: {saved}");
    assert_eq!(saved["prefix"], "new_name-2");
    assert_eq!(saved["format"], "JPEG");
    assert_eq!(saved["quality"], 100);
    assert_eq!(saved["max_width"], 16);
    assert_eq!(saved["open_folder"], false);
    assert_eq!(buttons(&d, &r)?, (false, false));
    assert_eq!(marker(&d, &r)?, "All changes saved");
    app.shutdown()?;

    // Restart: the saved file is loaded.
    let mut app = start_app(path).headless()?;
    let r = root_of(&app);
    let (ui, _) = app.runtime_and_state_mut();
    let mut d = Driver::new(ui);
    d.settle()?;
    assert_eq!(text(&d, &d.find(&r, &["output", "prefix", "input"])?)?, "new_name-2");
    assert_eq!(number(&d, &d.find(&r, &["quality-group", "width", "row", "input"])?)?, 16.0);
    app.shutdown()?;
    Ok(())
}

/// Item 4 (b): the save runs off the UI thread. The settings path is a FIFO
/// whose write blocks until a reader opens it; the UI keeps answering, shows
/// the save in progress, and finishes once the test thread reads the bytes.
#[test]
fn item4_save_runs_off_the_ui_thread() -> Result {
    let dir = dir("item4-fifo")?;
    let path = dir.join("settings.json");
    // Start without a file (startup would otherwise block reading the FIFO).
    let mut app = start_app(path.clone()).headless()?;
    let status = std::process::Command::new("mkfifo").arg(&path).status()?;
    assert!(status.success());
    let r = root_of(&app);
    let opened = Arc::new(AtomicBool::new(false));
    let reader = {
        let (path, opened) = (path.clone(), opened.clone());
        thread::spawn(move || -> std::io::Result<String> {
            thread::sleep(Duration::from_millis(1500));
            let mut file = fs::File::open(&path)?;
            opened.store(true, Ordering::SeqCst);
            let mut s = String::new();
            file.read_to_string(&mut s)?;
            Ok(s)
        })
    };
    let (ui, _) = app.runtime_and_state_mut();
    let mut d = Driver::new(ui);
    d.settle()?;
    let p = d.find(&r, &["output", "prefix", "input"])?;
    d.replace_text(&p, "threaded")?;
    d.key(KeyCode::Enter)?;
    let started = Instant::now();
    d.click(&d.find(&r, &["buttons", "apply"])?)?;
    d.advance(Duration::from_millis(50))?;
    let elapsed = started.elapsed();
    let in_progress = message(&d, &r)?;
    let reader_open = opened.load(Ordering::SeqCst);
    println!(
        "click+advance returned after {elapsed:?}; reader opened: {reader_open}; message: {in_progress:?}; buttons: {:?}",
        buttons(&d, &r)?
    );
    assert!(!reader_open, "UI returned before the FIFO had a reader");
    assert!(elapsed < Duration::from_millis(1000));
    assert_eq!(in_progress, "Saving…");
    // Still responsive while the write is blocked.
    assert_eq!(text(&d, &p)?, "threaded");
    let written = reader.join().expect("reader thread")?;
    println!("bytes read from FIFO: {written}");
    d.wait_until(Duration::from_secs(5), |ui| {
        Ok(label_in(ui, &["message"]) == "Settings saved")
    })?;
    let value: serde_json::Value = serde_json::from_str(&written)?;
    assert_eq!(value["prefix"], "threaded");
    assert_eq!(buttons(&d, &r)?, (false, false));
    app.shutdown()?;
    fs::remove_file(&path)?;
    Ok(())
}

/// Item 4 (c) and 5: a save to an unwritable path is reported, leaves the
/// state dirty, and closing is refused through the close request.
#[test]
fn item4_failed_save_to_unwritable_path() -> Result {
    // A path whose parent directory does not exist.
    let base = dir("item4-fail")?;
    let path = base.join("missing-folder").join("settings.json");
    check_failed_save(path.clone(), "missing parent")?;
    assert!(!path.exists());

    // A read-only directory, when the sandbox user is subject to permissions.
    use std::os::unix::fs::PermissionsExt;
    let ro = base.join("read-only");
    fs::create_dir(&ro)?;
    fs::set_permissions(&ro, fs::Permissions::from_mode(0o555))?;
    let probe = ro.join("probe");
    if fs::write(&probe, b"x").is_ok() {
        fs::remove_file(&probe)?;
        println!("read-only directory is writable for this user; that variant is skipped");
    } else {
        check_failed_save(ro.join("settings.json"), "read-only directory")?;
        assert!(!ro.join("settings.json").exists());
    }
    fs::set_permissions(&ro, fs::Permissions::from_mode(0o755))?;
    Ok(())
}

fn check_failed_save(path: PathBuf, what: &str) -> Result {
    let mut app = start_app(path).headless()?;
    let main = app.main_window();
    let r = root_of(&app);
    let (ui, _) = app.runtime_and_state_mut();
    let mut d = Driver::new(ui);
    d.settle()?;
    d.replace_text(&d.find(&r, &["quality-group", "quality", "input"])?, "55")?;
    d.key(KeyCode::Enter)?;
    d.click(&d.find(&r, &["buttons", "apply"])?)?;
    d.wait_until(Duration::from_secs(5), |ui| {
        Ok(label_in(ui, &["message"]).starts_with("Save failed"))
    })?;
    println!("{what}: message={:?} marker={:?} buttons={:?}", message(&d, &r)?, marker(&d, &r)?, buttons(&d, &r)?);
    assert_eq!(marker(&d, &r)?, "Unsaved changes");
    assert_eq!(buttons(&d, &r)?, (true, true));
    assert!(matches!(app.admit_close(&main)?, NativeCloseAction::KeepOpen));
    app.shutdown()?;
    Ok(())
}

/// Item 5: close requests through `HeadlessApp::admit_close` are refused while
/// dirty, with a visible request to apply or revert, and admitted once the
/// changes are applied or reverted.
#[test]
fn item5_close_refused_while_dirty() -> Result {
    let path = with_file("item5", LOADED)?;
    let mut app = start_app(path).headless()?;
    let main = app.main_window();
    let r = root_of(&app);
    assert!(matches!(app.admit_close(&main)?, NativeCloseAction::Close), "clean at startup");

    // Valid committed change: refused, then admitted after Apply.
    {
        let (ui, _) = app.runtime_and_state_mut();
        let mut d = Driver::new(ui);
        d.replace_text(&d.find(&r, &["quality-group", "quality", "input"])?, "90")?;
        d.key(KeyCode::Enter)?;
    }
    assert!(matches!(app.admit_close(&main)?, NativeCloseAction::KeepOpen));
    {
        let (ui, _) = app.runtime_and_state_mut();
        let mut d = Driver::new(ui);
        d.settle()?;
        let shown = message(&d, &r)?;
        println!("close refused message: {shown:?}");
        assert!(shown.contains("apply or revert"));
        d.click(&d.find(&r, &["buttons", "apply"])?)?;
        d.wait_until(Duration::from_secs(5), |ui| {
            Ok(label_in(ui, &["message"]) == "Settings saved")
        })?;
    }
    assert!(matches!(app.admit_close(&main)?, NativeCloseAction::Close), "after Apply");

    // Toggle change: refused, admitted after Revert.
    {
        let (ui, _) = app.runtime_and_state_mut();
        let mut d = Driver::new(ui);
        d.click(&d.find(&r, &["behavior", "open"])?)?;
    }
    assert!(matches!(app.admit_close(&main)?, NativeCloseAction::KeepOpen));
    {
        let (ui, _) = app.runtime_and_state_mut();
        let mut d = Driver::new(ui);
        d.settle()?;
        d.click(&d.find(&r, &["buttons", "revert"])?)?;
    }
    assert!(matches!(app.admit_close(&main)?, NativeCloseAction::Close), "after Revert");

    // A focused, uncommitted valid draft is committed by admission and refuses.
    {
        let (ui, _) = app.runtime_and_state_mut();
        let mut d = Driver::new(ui);
        let p = d.find(&r, &["output", "prefix", "input"])?;
        d.replace_text(&p, "typed_only")?;
    }
    assert!(matches!(app.admit_close(&main)?, NativeCloseAction::KeepOpen), "uncommitted draft");
    {
        let (ui, _) = app.runtime_and_state_mut();
        let mut d = Driver::new(ui);
        d.settle()?;
        assert_eq!(buttons(&d, &r)?, (true, true));
        d.click(&d.find(&r, &["buttons", "revert"])?)?;
        assert_eq!(text(&d, &d.find(&r, &["output", "prefix", "input"])?)?, "cam-01_X");
    }
    assert!(matches!(app.admit_close(&main)?, NativeCloseAction::Close));
    app.shutdown()?;
    Ok(())
}
