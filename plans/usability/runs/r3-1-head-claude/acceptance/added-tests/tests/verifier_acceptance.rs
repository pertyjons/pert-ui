//! Independent acceptance checks added by the verifier (not part of the
//! submission). They drive the application only through the library's
//! public headless facilities (`App::headless`, `Driver` input,
//! `HeadlessApp::admit_close`) and read results only through semantics,
//! inspection properties, CPU captures and the settings file on disk; they
//! never read or write the application's own signals. The only application
//! item used besides `app()` is `Editor::root`, the mounted root's id, as the
//! anchor for key-path lookup.
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use pert_ui::prelude::*;
use pert_ui::testing::*;
use settings_editor::{Editor, app};

type R<T = ()> = Result<T, Box<dyn std::error::Error>>;

const PREFIX: &[&str] = &["content", "output", "fields", "prefix-field", "prefix"];
const PREFIX_FIELD: &[&str] = &["content", "output", "fields", "prefix-field"];
const FORMAT: &[&str] = &["content", "output", "fields", "format-field", "format"];
const QUALITY: &[&str] = &["content", "quality", "fields", "quality-field", "quality"];
const QUALITY_FIELD: &[&str] = &["content", "quality", "fields", "quality-field"];
const WIDTH: &[&str] = &["content", "quality", "fields", "max-width-field", "max-width"];
const OPEN_FOLDER: &[&str] = &["content", "behavior", "fields", "open-folder"];
const APPLY: &[&str] = &["content", "buttons", "apply"];
const REVERT: &[&str] = &["content", "buttons", "revert"];
const MARKER: &[&str] = &["content", "buttons", "dirty"];
const MESSAGE: &[&str] = &["content", "message"];

fn scratch(name: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("verifier").join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// Writes a settings file in the submission's documented JSON shape.
fn write_settings(path: &Path, prefix: &str, format: &str, quality: i64, width: i64, open: bool) -> R {
    let json = format!(
        r#"{{"prefix":"{prefix}","format":"{format}","quality":{quality},"max_width":{width},"open_folder":{open}}}"#
    );
    std::fs::write(path, json)?;
    Ok(())
}

fn on_disk(path: &Path) -> R<serde_json::Value> {
    Ok(serde_json::from_str(&std::fs::read_to_string(path)?)?)
}

fn sem(d: &Driver<'_>, root: &WidgetId, path: &[&str]) -> R<Semantics> {
    Ok(d.semantics(&d.find(root, path)?)?)
}

fn name(d: &Driver<'_>, root: &WidgetId, path: &[&str]) -> R<String> {
    Ok(sem(d, root, path)?.name.unwrap_or_default())
}

fn text_value(d: &Driver<'_>, root: &WidgetId, path: &[&str]) -> R<String> {
    match sem(d, root, path)?.value {
        Some(SemanticValue::Text(t)) => Ok(t),
        other => Err(format!("not text: {other:?}").into()),
    }
}

fn number(d: &Driver<'_>, root: &WidgetId, path: &[&str]) -> R<f64> {
    match sem(d, root, path)?.value {
        Some(SemanticValue::Number { value, .. }) => Ok(value),
        other => Err(format!("not a number: {other:?}").into()),
    }
}

fn props(d: &Driver<'_>, id: &WidgetId) -> R<std::collections::BTreeMap<String, InspectValue>> {
    let q = InspectionQuery::components(Selector::id(d.runtime().inspection_id(id)?)).detail(Detail::Full);
    let result = d.runtime().query(&q);
    let c = result.components.first().ok_or("component not in inspection")?;
    Ok(c.details.properties.iter().map(|(k, v)| (k.to_string(), v.clone())).collect())
}

/// (Apply enabled, Revert enabled), each read twice: from semantics and from
/// the inspection snapshot's "enabled" property; the two must agree.
fn buttons(d: &Driver<'_>, root: &WidgetId) -> R<(bool, bool)> {
    let mut out = [false; 2];
    for (i, path) in [APPLY, REVERT].into_iter().enumerate() {
        let id = d.find(root, path)?;
        let by_semantics = !d.semantics(&id)?.disabled;
        let by_inspection = match props(d, &id)?.get("enabled") {
            Some(InspectValue::Bool(b)) => *b,
            other => return Err(format!("{path:?}: no enabled property: {other:?}").into()),
        };
        assert_eq!(by_semantics, by_inspection, "{path:?}: semantics and inspection disagree");
        out[i] = by_semantics;
    }
    Ok((out[0], out[1]))
}

fn format_anchor(d: &Driver<'_>, root: &WidgetId) -> R<WidgetId> {
    let select = d.find(root, FORMAT)?;
    Ok(d.runtime().find_descendant(&select, "select")?.ok_or("no dropdown anchor")?)
}

fn format_label(d: &Driver<'_>, root: &WidgetId) -> R<(String, usize)> {
    match d.semantics(&format_anchor(d, root)?)?.value {
        Some(SemanticValue::Choice { label: Some(l), count, .. }) => Ok((l, count)),
        other => Err(format!("format value {other:?}").into()),
    }
}

/// Settles until the label at `path` satisfies `pred`, for at most 5 s.
fn wait_label(d: &mut Driver<'_>, root: &WidgetId, path: &[&str], pred: impl Fn(&str) -> bool) -> R<String> {
    let start = Instant::now();
    loop {
        d.settle()?;
        let n = name(d, root, path)?;
        if pred(&n) {
            return Ok(n);
        }
        if start.elapsed() > Duration::from_secs(5) {
            return Err(format!("timed out; {path:?} = {n:?}").into());
        }
        std::thread::sleep(Duration::from_millis(2));
    }
}

fn commit(d: &mut Driver<'_>, root: &WidgetId, path: &[&str], text: &str) -> R {
    let id = d.find(root, path)?;
    d.replace_text(&id, text)?;
    d.key(KeyCode::Enter)?;
    Ok(())
}

fn capture(d: &mut Driver<'_>, file: &str) -> R {
    d.runtime().set_capture_support(true);
    let pending = d
        .runtime()
        .request_capture(CaptureRequest::new(CaptureTarget::window(&d.runtime().primary_window())));
    d.settle()?;
    CpuRenderer::new().serve_captures(d.runtime_mut());
    let image = pending.try_reply()?.value?.image;
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("verifier-captures");
    std::fs::create_dir_all(&dir)?;
    std::fs::write(dir.join(file), image.encode_png())?;
    println!("capture {file}");
    Ok(())
}

fn start(path: &Path) -> R<(HeadlessApp<Editor>, WidgetId)> {
    let mut a = app(path.to_path_buf()).headless()?;
    Driver::new(a.runtime_mut()).settle()?;
    let root = a.state().root.clone();
    Ok((a, root))
}

/// Item 1: every group, field and button exists and shows the file's values
/// (values different from the tester's sample and from the defaults).
#[test]
fn v1_every_field_shows_the_loaded_file() -> R {
    let dir = scratch("v1");
    let path = dir.join("settings.json");
    write_settings(&path, "cam-A_7", "Png", 12, 16384, false)?;
    let (mut a, root) = start(&path)?;
    let mut d = Driver::new(a.runtime_mut());

    for (key, title) in [("output", "Output"), ("quality", "Quality"), ("behavior", "Behavior")] {
        let g = sem(&d, &root, &["content", key])?;
        assert_eq!(g.role, SemanticRole::Group, "{key}");
        assert_eq!(g.name.as_deref(), Some(title));
    }
    assert_eq!(name(&d, &root, PREFIX)?, "File name prefix");
    assert_eq!(text_value(&d, &root, PREFIX)?, "cam-A_7");
    let anchor = format_anchor(&d, &root)?;
    let f = d.semantics(&anchor)?;
    assert_eq!(f.role, SemanticRole::ComboBox);
    assert_eq!(f.name.as_deref(), Some("Format"));
    assert_eq!(format_label(&d, &root)?, ("PNG".to_owned(), 3));
    assert_eq!(name(&d, &root, QUALITY_FIELD)?, "JPEG quality");
    assert_eq!(number(&d, &root, QUALITY)?, 12.0);
    assert_eq!(number(&d, &root, WIDTH)?, 16384.0);
    let width = d.find(&root, WIDTH)?;
    assert_eq!(props(&d, &width)?.get("unit"), Some(&InspectValue::Text("px".into())));
    let t = sem(&d, &root, OPEN_FOLDER)?;
    assert_eq!(t.role, SemanticRole::Switch);
    assert_eq!(t.name.as_deref(), Some("Open folder after export"));
    assert_eq!(t.value, Some(SemanticValue::Bool(false)));
    for (path, label) in [(APPLY, "Apply"), (REVERT, "Revert")] {
        let b = sem(&d, &root, path)?;
        assert_eq!(b.role, SemanticRole::Button);
        assert_eq!(b.name.as_deref(), Some(label));
    }
    assert_eq!(buttons(&d, &root)?, (false, false), "loaded file is clean");
    capture(&mut d, "v1-loaded.png")?;

    // The three format choices, in order, through key input on the dropdown.
    d.runtime_mut().focus(&anchor)?;
    let mut seen = vec![format_label(&d, &root)?.0];
    for _ in 0..2 {
        d.key(KeyCode::Enter)?;
        d.key(KeyCode::ArrowDown)?;
        d.key(KeyCode::Enter)?;
        seen.push(format_label(&d, &root)?.0);
    }
    assert_eq!(seen, ["PNG", "JPEG", "WebP"]);
    drop(d);
    a.shutdown()?;
    Ok(())
}

/// Item 2: invalid prefixes and out-of-range quality are refused through the
/// field itself (typing, Enter and focus loss), the explanation is shown next
/// to the field, and the last valid value stays in effect, as proved by what
/// a later Apply writes to disk.
#[test]
fn v2_refused_input_keeps_the_last_valid_value_in_effect() -> R {
    let dir = scratch("v2");
    let path = dir.join("settings.json");
    write_settings(&path, "holiday", "Jpeg", 70, 2048, true)?;
    let (mut a, root) = start(&path)?;
    let mut d = Driver::new(a.runtime_mut());
    let prefix = d.find(&root, PREFIX)?;
    let quality = d.find(&root, QUALITY)?;

    // Typed at the caret, committed by focus loss (Tab).
    d.runtime_mut().focus(&prefix)?;
    d.key(KeyCode::End)?;
    d.type_text(" pics!")?;
    d.key(KeyCode::Tab)?;
    let s = d.semantics(&prefix)?;
    assert!(s.invalid, "space and ! accepted");
    assert_eq!(s.value, Some(SemanticValue::Text("holiday pics!".into())), "the refused draft stays visible");
    assert!(s.description.unwrap_or_default().contains("not allowed"));
    let field = sem(&d, &root, PREFIX_FIELD)?;
    assert!(field.description.unwrap_or_default().contains("not allowed"), "explanation on the field");

    for bad in ["", &"y".repeat(33), "a/b", "ä"] {
        commit(&mut d, &root, PREFIX, bad)?;
        assert!(d.semantics(&prefix)?.invalid, "{bad:?} accepted");
        assert!(!d.semantics(&prefix)?.description.unwrap_or_default().is_empty(), "{bad:?}: no explanation");
    }
    // Boundary: 32 allowed characters are accepted.
    let ok32 = "Ab-_".repeat(8);
    commit(&mut d, &root, PREFIX, &ok32)?;
    assert!(!d.semantics(&prefix)?.invalid, "32 characters refused");
    // And refused again, to leave a refused draft over a valid value.
    commit(&mut d, &root, PREFIX, "no spaces")?;
    assert!(d.semantics(&prefix)?.invalid);

    for bad in ["101", "0", "-5", "1e2", "seventy"] {
        commit(&mut d, &root, QUALITY, bad)?;
        let s = d.semantics(&quality)?;
        assert!(s.invalid, "quality {bad:?} accepted");
        let why = s.description.unwrap_or_default();
        // Out-of-range numbers name the range; malformed text gets its own
        // explanation ("Enter a whole number"), corrected after attempt 1.
        if bad.parse::<i64>().is_ok() {
            assert!(why.contains("1") && why.contains("100"), "quality {bad:?}: {why:?}");
        } else {
            assert!(!why.is_empty(), "quality {bad:?}: no explanation");
        }
        println!("quality {bad:?} refused: {why:?}");
        let f = sem(&d, &root, QUALITY_FIELD)?;
        assert!(f.description.unwrap_or_default().contains(&why), "explanation on the field");
    }
    for ok in ["1", "100"] {
        commit(&mut d, &root, QUALITY, ok)?;
        assert!(!d.semantics(&quality)?.invalid, "quality {ok} refused");
    }
    commit(&mut d, &root, QUALITY, "250")?;
    assert!(d.semantics(&quality)?.invalid);
    commit(&mut d, &root, WIDTH, "15")?;
    assert!(sem(&d, &root, WIDTH)?.invalid, "width 15 accepted");
    capture(&mut d, "v2-refused.png")?;
    commit(&mut d, &root, WIDTH, "16")?;
    assert!(!sem(&d, &root, WIDTH)?.invalid, "width 16 refused");

    // Apply now: the refused drafts must not reach the file; the last valid
    // values (32-character prefix, quality 100) are what is in effect.
    let (apply_on, _) = buttons(&d, &root)?;
    assert!(apply_on);
    let apply = d.find(&root, APPLY)?;
    d.click(&apply)?;
    wait_label(&mut d, &root, MESSAGE, |m| m == "Settings saved")?;
    let v = on_disk(&path)?;
    assert_eq!(v["prefix"], ok32.as_str());
    assert_eq!(v["quality"], 100);
    assert_eq!(v["max_width"], 16);
    drop(d);
    a.shutdown()?;
    Ok(())
}

/// Item 3: disabled when clean and enabled when dirty (read from semantics
/// and inspection), and Revert after a valid change followed by a later
/// refused draft restores every applied value (round 2's Revert case).
#[test]
fn v3_revert_after_valid_change_and_later_refused_draft() -> R {
    let dir = scratch("v3");
    let path = dir.join("settings.json");
    write_settings(&path, "trip", "WebP", 64, 1200, false)?;
    let before = std::fs::read(&path)?;
    let (mut a, root) = start(&path)?;
    let mut d = Driver::new(a.runtime_mut());
    assert_eq!(buttons(&d, &root)?, (false, false), "clean");
    let clean_marker = name(&d, &root, MARKER)?;

    // A disabled Apply does nothing when clicked.
    let apply = d.find(&root, APPLY)?;
    d.click(&apply)?;
    d.settle()?;
    assert_eq!(std::fs::read(&path)?, before);

    // Valid changes of three kinds.
    commit(&mut d, &root, QUALITY, "55")?;
    assert_eq!(buttons(&d, &root)?, (true, true), "dirty after a valid change");
    let dirty_marker = name(&d, &root, MARKER)?;
    assert_ne!(dirty_marker, clean_marker, "visible dirty marker");
    let toggle = d.find(&root, OPEN_FOLDER)?;
    d.click(&toggle)?;
    let anchor = format_anchor(&d, &root)?;
    d.runtime_mut().focus(&anchor)?;
    d.key(KeyCode::Enter)?;
    d.key(KeyCode::ArrowUp)?;
    d.key(KeyCode::Enter)?;
    assert_eq!(format_label(&d, &root)?.0, "JPEG");

    // Later, a refused draft in another field.
    commit(&mut d, &root, PREFIX, "bad name!")?;
    assert!(sem(&d, &root, PREFIX)?.invalid);
    assert_eq!(buttons(&d, &root)?, (true, true), "still dirty with a refused draft");
    capture(&mut d, "v3-dirty-with-refused-draft.png")?;

    let revert = d.find(&root, REVERT)?;
    d.click(&revert)?;
    d.settle()?;
    let p = sem(&d, &root, PREFIX)?;
    assert_eq!(p.value, Some(SemanticValue::Text("trip".into())));
    assert!(!p.invalid, "refusal cleared");
    assert_eq!(number(&d, &root, QUALITY)?, 64.0);
    assert!(!sem(&d, &root, QUALITY)?.invalid);
    assert_eq!(format_label(&d, &root)?.0, "WebP");
    assert_eq!(sem(&d, &root, OPEN_FOLDER)?.value, Some(SemanticValue::Bool(false)));
    assert_eq!(number(&d, &root, WIDTH)?, 1200.0);
    assert_eq!(buttons(&d, &root)?, (false, false), "clean after Revert");
    assert_eq!(name(&d, &root, MARKER)?, clean_marker);
    assert_eq!(std::fs::read(&path)?, before, "Revert writes nothing");

    // Revert restores the *applied* values after an Apply, not the loaded ones.
    commit(&mut d, &root, WIDTH, "640")?;
    let apply = d.find(&root, APPLY)?;
    d.click(&apply)?;
    wait_label(&mut d, &root, MESSAGE, |m| m == "Settings saved")?;
    assert_eq!(buttons(&d, &root)?, (false, false));
    commit(&mut d, &root, WIDTH, "999")?;
    commit(&mut d, &root, QUALITY, "500")?;
    assert_eq!(buttons(&d, &root)?, (true, true));
    d.click(&revert)?;
    d.settle()?;
    assert_eq!(number(&d, &root, WIDTH)?, 640.0);
    assert_eq!(number(&d, &root, QUALITY)?, 64.0);
    assert_eq!(buttons(&d, &root)?, (false, false));
    drop(d);
    a.shutdown()?;
    Ok(())
}

/// Item 4 (background work): the settings file is a FIFO with no reader, so
/// opening it for writing blocks until the test reads it. If Apply saved on
/// the UI thread, the click would block; a watchdog would then unblock it and
/// the test fails. Otherwise the UI keeps settling while the save waits.
#[test]
fn v4_save_runs_off_the_ui_thread() -> R {
    let dir = scratch("v4-fifo");
    let path = dir.join("settings.json");
    let (mut a, root) = start(&path)?; // no file yet: defaults
    let status = std::process::Command::new("mkfifo").arg(&path).status()?;
    assert!(status.success(), "mkfifo failed");

    let clicked = Arc::new(AtomicBool::new(false));
    let blocked = Arc::new(AtomicBool::new(false));
    let watchdog = {
        let (clicked, blocked, path) = (clicked.clone(), blocked.clone(), path.clone());
        std::thread::spawn(move || {
            let start = Instant::now();
            while !clicked.load(Ordering::SeqCst) && start.elapsed() < Duration::from_secs(10) {
                std::thread::sleep(Duration::from_millis(5));
            }
            if !clicked.load(Ordering::SeqCst) {
                blocked.store(true, Ordering::SeqCst);
                let _ = std::fs::read_to_string(&path);
            }
        })
    };

    let mut d = Driver::new(a.runtime_mut());
    commit(&mut d, &root, QUALITY, "42")?;
    let apply = d.find(&root, APPLY)?;
    d.click(&apply)?;
    clicked.store(true, Ordering::SeqCst);
    watchdog.join().unwrap();
    assert!(!blocked.load(Ordering::SeqCst), "the click blocked: the save ran on the UI thread");

    // The UI keeps running while the worker waits on the FIFO.
    for _ in 0..20 {
        d.settle()?;
    }
    assert_eq!(name(&d, &root, MESSAGE)?, "Saving…");
    let (apply_on, _) = buttons(&d, &root)?;
    assert!(!apply_on, "Apply is disabled while saving");

    let reader = {
        let path = path.clone();
        std::thread::spawn(move || std::fs::read_to_string(&path))
    };
    wait_label(&mut d, &root, MESSAGE, |m| m == "Settings saved")?;
    let written: serde_json::Value = serde_json::from_str(&reader.join().unwrap()?)?;
    assert_eq!(written["quality"], 42);
    assert_eq!(buttons(&d, &root)?, (false, false), "clean after the save");
    drop(d);
    a.shutdown()?;
    Ok(())
}

/// Item 4 (results): a successful save writes the file and clears the dirty
/// state; a save into a removed folder and into a read-only folder is
/// reported and leaves the state dirty; a retry after the folder returns
/// succeeds; the next start loads what was saved.
#[test]
fn v4_failed_save_is_reported_and_stays_dirty() -> R {
    let dir = scratch("v4-fail");
    let sub = dir.join("sub");
    std::fs::create_dir_all(&sub)?;
    let path = sub.join("settings.json");
    write_settings(&path, "first", "Png", 90, 3000, false)?;
    let (mut a, root) = start(&path)?;
    let mut d = Driver::new(a.runtime_mut());
    let apply = d.find(&root, APPLY)?;

    std::fs::remove_dir_all(&sub)?;
    commit(&mut d, &root, PREFIX, "after-fail")?;
    d.click(&apply)?;
    let msg = wait_label(&mut d, &root, MESSAGE, |m| m != "Saving…")?;
    assert!(msg.starts_with("Save failed"), "{msg}");
    assert!(!path.exists());
    assert_eq!(buttons(&d, &root)?, (true, true), "still dirty after the failure");
    assert!(name(&d, &root, MARKER)?.contains("Unsaved"));
    assert_eq!(text_value(&d, &root, PREFIX)?, "after-fail", "edit kept");
    capture(&mut d, "v4-save-failed.png")?;
    let main = d.runtime().primary_window();
    drop(d);
    assert!(matches!(a.admit_close(&main)?, NativeCloseAction::KeepOpen));

    // Read-only folder (skipped when the sandbox user can write anyway).
    std::fs::create_dir_all(&sub)?;
    let mut perms = std::fs::metadata(&sub)?.permissions();
    use std::os::unix::fs::PermissionsExt;
    perms.set_mode(0o555);
    std::fs::set_permissions(&sub, perms.clone())?;
    let writable = std::fs::write(sub.join("probe"), "x").is_ok();
    let mut d = Driver::new(a.runtime_mut());
    if writable {
        println!("read-only case skipped: directory still writable");
        let _ = std::fs::remove_file(sub.join("probe"));
    } else {
        d.click(&apply)?;
        let msg = wait_label(&mut d, &root, MESSAGE, |m| m != "Saving…")?;
        assert!(msg.starts_with("Save failed"), "{msg}");
        assert!(msg.to_lowercase().contains("permission"), "{msg}");
        assert_eq!(buttons(&d, &root)?, (true, true), "still dirty after the read-only failure");
        println!("read-only case ran: {msg}");
    }
    perms.set_mode(0o755);
    std::fs::set_permissions(&sub, perms)?;

    // Retry succeeds once the folder is writable.
    d.click(&apply)?;
    wait_label(&mut d, &root, MESSAGE, |m| m == "Settings saved")?;
    assert_eq!(buttons(&d, &root)?, (false, false));
    let v = on_disk(&path)?;
    assert_eq!(v["prefix"], "after-fail");
    assert_eq!(v["format"], "Png");
    assert_eq!(v["quality"], 90);
    assert_eq!(v["max_width"], 3000);
    assert_eq!(v["open_folder"], false);
    drop(d);
    assert!(matches!(a.admit_close(&main)?, NativeCloseAction::Close));
    a.shutdown()?;

    let (mut a, root) = start(&path)?;
    let d = Driver::new(a.runtime_mut());
    assert_eq!(text_value(&d, &root, PREFIX)?, "after-fail");
    assert_eq!(buttons(&d, &root)?, (false, false));
    drop(d);
    a.shutdown()?;
    Ok(())
}

/// Item 5: the library's headless close request is refused while dirty, with
/// a visible request to apply or revert, and admitted once the changes are
/// resolved by Apply, by Revert, or by editing back.
#[test]
fn v5_close_refused_while_dirty_and_admitted_when_resolved() -> R {
    let dir = scratch("v5");
    let path = dir.join("settings.json");
    write_settings(&path, "close", "Jpeg", 50, 500, true)?;
    let (mut a, root) = start(&path)?;
    let main = a.main_window();
    assert!(matches!(a.admit_close(&main)?, NativeCloseAction::Close), "clean closes");

    // Dirty by a valid change; resolved by Apply.
    let mut d = Driver::new(a.runtime_mut());
    commit(&mut d, &root, QUALITY, "51")?;
    drop(d);
    assert!(matches!(a.admit_close(&main)?, NativeCloseAction::KeepOpen));
    let mut d = Driver::new(a.runtime_mut());
    d.settle()?;
    let msg = name(&d, &root, MESSAGE)?;
    assert!(msg.contains("apply or revert"), "{msg}");
    capture(&mut d, "v5-close-refused.png")?;
    assert_eq!(number(&d, &root, QUALITY)?, 51.0, "the window and edit stay");
    let apply = d.find(&root, APPLY)?;
    d.click(&apply)?;
    wait_label(&mut d, &root, MESSAGE, |m| m == "Settings saved")?;
    drop(d);
    assert!(matches!(a.admit_close(&main)?, NativeCloseAction::Close), "admitted after Apply");

    // Dirty by the toggle; resolved by Revert.
    let mut d = Driver::new(a.runtime_mut());
    let toggle = d.find(&root, OPEN_FOLDER)?;
    d.click(&toggle)?;
    drop(d);
    assert!(matches!(a.admit_close(&main)?, NativeCloseAction::KeepOpen));
    let mut d = Driver::new(a.runtime_mut());
    let revert = d.find(&root, REVERT)?;
    d.click(&revert)?;
    drop(d);
    assert!(matches!(a.admit_close(&main)?, NativeCloseAction::Close), "admitted after Revert");

    // Valid change plus a later refused draft; resolved by Revert.
    let mut d = Driver::new(a.runtime_mut());
    commit(&mut d, &root, WIDTH, "700")?;
    commit(&mut d, &root, PREFIX, "x y")?;
    drop(d);
    assert!(matches!(a.admit_close(&main)?, NativeCloseAction::KeepOpen));
    let mut d = Driver::new(a.runtime_mut());
    d.click(&revert)?;
    drop(d);
    assert!(matches!(a.admit_close(&main)?, NativeCloseAction::Close));

    // Editing back to the applied value is not dirty.
    let mut d = Driver::new(a.runtime_mut());
    d.click(&toggle)?;
    d.click(&toggle)?;
    drop(d);
    assert!(matches!(a.admit_close(&main)?, NativeCloseAction::Close), "edited back");
    a.shutdown()?;
    Ok(())
}
