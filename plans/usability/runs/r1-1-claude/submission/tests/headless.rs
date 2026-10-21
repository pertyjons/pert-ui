//! Headless checks: the editor is mounted on a plain `Runtime` and driven
//! through focus, typed text, keys and inspection, as a user would.

use std::path::{Path, PathBuf};
use std::time::Duration;

use pert_ui::{
    KeyAction, KeyCode, PresentationAction, PresentationStatus, Runtime, SemanticValue,
    Semantics, TextCommand, Viewport, WidgetId,
};
use settings_editor::{build, Editor};

type R<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("settings-editor-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

const SAVED: &str = r#"{"prefix":"holiday_2026","format":"JPEG","quality":75,"max_width":2048,"open_folder":true}"#;

struct Harness {
    ui: Runtime,
    editor: Editor,
}

impl Harness {
    fn new(path: &Path) -> R<Self> {
        let mut ui = Runtime::new();
        ui.set_viewport(Viewport::new(800.0, 900.0, 1.0)?)?;
        let editor = build(&mut ui, path.to_owned())?;
        ui.flush(Duration::ZERO)?;
        Ok(Harness { ui, editor })
    }

    fn id(&self, key: &str) -> R<WidgetId> {
        Ok(self.ui.find_descendant(&self.editor.root, key)?.ok_or(format!("no {key}"))?)
    }

    fn semantics(&self, key: &str) -> R<Semantics> {
        let found: Vec<_> = self.ui.inspect().components.into_iter().filter(|c| c.key == key).collect();
        assert_eq!(found.len(), 1, "key {key} should be unique");
        Ok(found[0].details.semantics.clone().ok_or(format!("{key} has no semantics"))?)
    }

    fn text(&self, key: &str) -> R<String> {
        match self.semantics(key)?.value {
            Some(SemanticValue::Text(t)) => Ok(t),
            other => Err(format!("{key}: {other:?}").into()),
        }
    }

    fn number(&self, key: &str) -> R<f64> {
        match self.semantics(key)?.value {
            Some(SemanticValue::Number { value, .. }) => Ok(value),
            other => Err(format!("{key}: {other:?}").into()),
        }
    }

    fn label(&self, key: &str) -> R<String> {
        let s = self.semantics(key)?;
        Ok(s.name.unwrap_or_default())
    }

    fn press(&mut self, code: KeyCode) -> R {
        self.ui.key_input(code, KeyAction::Press)?;
        self.ui.key_input(code, KeyAction::Release)?;
        self.ui.flush(Duration::ZERO)?;
        Ok(())
    }

    /// Replaces a field's text as a user would: focus, select all, type, Enter.
    fn type_into(&mut self, key: &str, text: &str) -> R {
        let id = self.id(key)?;
        self.ui.focus(&id)?;
        self.ui.text_command(&id, TextCommand::SelectAll)?;
        self.ui.text_input(text)?;
        self.press(KeyCode::Enter)
    }

    fn click(&mut self, key: &str) -> R {
        let id = self.id(key)?;
        self.ui.focus(&id)?;
        self.press(KeyCode::Enter)
    }

    fn enabled(&self, key: &str) -> R<bool> {
        Ok(!self.semantics(key)?.disabled)
    }

    /// Flushes until background work has been delivered.
    fn settle(&mut self) -> R {
        for _ in 0..200 {
            self.ui.flush(Duration::ZERO)?;
            if !self.label("notice")?.starts_with("Saving") {
                return Ok(());
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        Err("background save never delivered".into())
    }
}

#[test]
fn shows_loaded_values_in_titled_groups() -> R {
    let dir = temp_dir("load");
    let path = dir.join("settings.json");
    std::fs::write(&path, SAVED)?;
    let h = Harness::new(&path)?;

    for group in ["Output", "Quality", "Behavior"] {
        assert!(h.ui.inspect().components.iter().any(|c| c
            .details
            .semantics
            .as_ref()
            .and_then(|s| s.name.as_deref())
            == Some(group)), "group {group} is titled");
    }
    assert_eq!(h.text("prefix")?, "holiday_2026");
    assert_eq!(h.label("prefix")?, "File name prefix");
    let format = h.semantics("format")?;
    assert!(matches!(format.value, Some(SemanticValue::Choice { label: Some(ref l), .. }) if l == "JPEG"), "{format:?}");
    assert_eq!(h.number("quality")?, 75.0);
    assert_eq!(h.number("max-width")?, 2048.0);
    assert_eq!(h.semantics("open-folder")?.value, Some(SemanticValue::Bool(true)));
    assert_eq!(h.label("open-folder")?, "Open folder after export");
    assert_eq!(h.label("apply")?, "Apply");
    assert_eq!(h.label("revert")?, "Revert");
    let mut h = h;
    capture(&mut h, "loaded.png")?;
    Ok(())
}

#[test]
fn refuses_invalid_prefix_and_out_of_range_quality() -> R {
    let dir = temp_dir("invalid");
    let path = dir.join("settings.json");
    std::fs::write(&path, SAVED)?;
    let mut h = Harness::new(&path)?;

    h.type_into("prefix", "bad name!")?;
    let prefix = h.semantics("prefix")?;
    assert!(prefix.invalid, "{prefix:?}");
    // The field's status line (the control's own description reports its draft).
    let message = h.semantics("prefix-field")?.description.unwrap_or_default();
    assert!(message.contains("not allowed"), "explanation shown: {message}");
    // Nothing changed: the applied value is still in effect, nothing is dirty.
    assert_eq!(h.label("dirty-marker")?, "");
    assert!(!h.enabled("apply")?);

    h.type_into("quality", "250")?;
    capture(&mut h, "invalid.png")?;
    let quality = h.semantics("quality")?;
    assert!(quality.invalid, "{quality:?}");
    let message = h.semantics("quality-field")?.description.unwrap_or_default();
    assert!(message.contains("between 1 and 100"), "explanation shown: {message}");
    assert_eq!(h.number("quality")?, 75.0, "old value kept");
    assert!(!h.enabled("apply")?);

    // Escape drops the refused prefix draft: the field shows the value in effect.
    let prefix = h.id("prefix")?;
    h.ui.focus(&prefix)?;
    h.press(KeyCode::Escape)?;
    assert_eq!(h.text("prefix")?, "holiday_2026");

    // Correcting the input clears the explanation.
    h.type_into("quality", "80")?;
    let quality = h.semantics("quality")?;
    assert!(!quality.invalid);
    assert_eq!(h.number("quality")?, 80.0);
    Ok(())
}

#[test]
fn apply_and_revert_follow_dirty_state() -> R {
    let dir = temp_dir("dirty");
    let path = dir.join("settings.json");
    std::fs::write(&path, SAVED)?;
    let mut h = Harness::new(&path)?;

    assert!(!h.enabled("apply")?);
    assert!(!h.enabled("revert")?);
    assert_eq!(h.label("dirty-marker")?, "");

    h.type_into("prefix", "renamed")?;
    h.type_into("max-width", "1024")?;
    let toggle = h.id("open-folder")?;
    h.ui.focus(&toggle)?;
    h.press(KeyCode::Space)?;
    assert!(h.enabled("apply")?);
    assert!(h.enabled("revert")?);
    assert_eq!(h.label("dirty-marker")?, "● Unsaved changes");

    h.click("revert")?;
    assert_eq!(h.text("prefix")?, "holiday_2026");
    assert_eq!(h.number("max-width")?, 2048.0);
    assert_eq!(h.semantics("open-folder")?.value, Some(SemanticValue::Bool(true)));
    assert!(!h.enabled("apply")?);
    assert!(!h.enabled("revert")?);
    assert_eq!(h.label("dirty-marker")?, "");
    Ok(())
}

#[test]
fn successful_save_writes_file_and_clears_dirty() -> R {
    let dir = temp_dir("save");
    let path = dir.join("settings.json");
    let mut h = Harness::new(&path)?; // no file yet: defaults
    assert_eq!(h.text("prefix")?, "export");

    h.type_into("prefix", "trip")?;
    h.type_into("quality", "60")?;
    h.click("apply")?;
    h.settle()?;
    assert!(h.label("notice")?.starts_with("Saved"), "{}", h.label("notice")?);
    assert!(!h.enabled("apply")?);
    assert_eq!(h.label("dirty-marker")?, "");

    let written: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&path)?)?;
    assert_eq!(written["prefix"], "trip");
    assert_eq!(written["quality"], 60);

    // A fresh start loads what was saved.
    let again = Harness::new(&path)?;
    assert_eq!(again.text("prefix")?, "trip");
    assert_eq!(again.number("quality")?, 60.0);
    Ok(())
}

#[test]
fn failed_save_is_reported_and_stays_dirty() -> R {
    let dir = temp_dir("fail");
    let path = dir.join("missing-dir").join("settings.json"); // parent does not exist
    let mut h = Harness::new(&path)?;

    h.type_into("prefix", "trip")?;
    h.click("apply")?;
    h.settle()?;
    let notice = h.label("notice")?;
    assert!(notice.starts_with("Could not save"), "{notice}");
    assert!(h.enabled("apply")?, "still dirty, Apply can be retried");
    assert_eq!(h.label("dirty-marker")?, "● Unsaved changes");
    assert!(!path.exists());
    capture(&mut h, "failed-save.png")?;
    Ok(())
}

/// Plays the native host's part for close requests: the policy is the one
/// `main` installs with `App::on_close`.
fn host_close(h: &mut Harness) -> R<PresentationStatus> {
    let window = h.ui.primary_window();
    let receipt = h.ui.request_close(&window)?;
    for request in h.ui.take_presentation_requests()? {
        if let PresentationAction::Close(id) = request.action().clone() {
            let editor = h.editor.clone();
            let status = match editor.on_close(&mut h.ui, &id)? {
                pert_ui::NativeCloseAction::KeepOpen => PresentationStatus::CloseRejected,
                _ => PresentationStatus::Closed,
            };
            request.complete(status)?;
        }
    }
    h.ui.flush(Duration::ZERO)?;
    Ok(receipt.status())
}

#[test]
fn closing_is_refused_while_dirty() -> R {
    let dir = temp_dir("close");
    let path = dir.join("settings.json");
    std::fs::write(&path, SAVED)?;
    let mut h = Harness::new(&path)?;

    h.type_into("prefix", "unsaved")?;
    assert_eq!(host_close(&mut h)?, PresentationStatus::CloseRejected);
    assert!(h.label("notice")?.contains("apply or revert"), "{}", h.label("notice")?);

    h.click("revert")?;
    assert_eq!(host_close(&mut h)?, PresentationStatus::Closed);
    Ok(())
}

/// Renders the whole window on the CPU into `target/<name>`.
fn capture(h: &mut Harness, name: &str) -> R {
    use pert_ui::{cpu::CpuRenderer, CaptureRequest, CaptureTarget};
    h.ui.set_capture_support(true);
    let window = h.ui.primary_window();
    let pending = h.ui.request_capture(CaptureRequest::new(CaptureTarget::window(&window)));
    h.ui.flush(Duration::ZERO)?;
    CpuRenderer::new().serve_captures(&mut h.ui);
    let capture = pending.wait()?.value?;
    let out = Path::new(env!("CARGO_MANIFEST_DIR")).join("target").join(name);
    std::fs::write(out, capture.image.encode_png())?;
    Ok(())
}
