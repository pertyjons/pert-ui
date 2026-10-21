use pert_ui::{prelude::*, *};
use serde::{Deserialize, Serialize};
use std::{cell::RefCell, path::PathBuf, rc::Rc};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct Settings {
    prefix: String,
    format: String,
    quality: i64,
    max_width: i64,
    open_folder: bool,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            prefix: "photo".into(),
            format: "PNG".into(),
            quality: 85,
            max_width: 2048,
            open_folder: true,
        }
    }
}
fn valid_prefix(s: &str) -> bool {
    (1..=32).contains(&s.len())
        && s.bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'-' || c == b'_')
}
fn load(path: &PathBuf) -> (Settings, String) {
    let read = || -> Result<Settings, String> {
        let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
        let s: Settings = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
        if !valid_prefix(&s.prefix)
            || !["PNG", "JPEG", "WebP"].contains(&s.format.as_str())
            || !(1..=100).contains(&s.quality)
            || !(16..=16384).contains(&s.max_width)
        {
            return Err("Settings file contains invalid values".into());
        }
        Ok(s)
    };
    if !path.exists() {
        return (Settings::default(), "Ready".into());
    }
    match read() {
        Ok(s) => (s, "Loaded settings".into()),
        Err(e) => (
            Settings::default(),
            format!("Load failed: {e}. Using defaults."),
        ),
    }
}

#[derive(Clone)]
struct CloseGuard {
    dirty: ReadSignal<bool>,
    busy: Signal<bool>,
    message: Signal<String>,
}
impl CloseGuard {
    fn allow(&self) -> Result<bool, RuntimeError> {
        if self.dirty.get()? || self.busy.get()? {
            self.message
                .set("Unsaved changes: apply or revert before closing.".into())?;
            Ok(false)
        } else {
            Ok(true)
        }
    }
}
struct Editor {
    _mailbox: Mailbox<(Settings, Result<(), String>)>,
    guard: CloseGuard,
    #[cfg(test)]
    root: WidgetId,
}
fn mount(ui: &mut Runtime, path: PathBuf) -> Result<Editor, RuntimeError> {
    let (initial, message) = load(&path);
    let current = ui.signal(initial.clone());
    let applied = ui.signal(initial.clone());
    let dirty = current.combine(&applied.read_only(), |a, b| a != b)?;
    let marker = dirty.map(|d| {
        if *d {
            "● Unsaved changes".to_owned()
        } else {
            "All changes saved".to_owned()
        }
    })?;
    let busy = ui.signal(false);
    let editable = busy.map(|b| !b)?;
    let message = ui.signal(message);
    let guard = CloseGuard {
        dirty: dirty.clone(),
        busy: busy.clone(),
        message: message.clone(),
    };
    let prefix = ui.signal(initial.prefix);
    let format = ui.signal(initial.format);
    let quality = ui.signal(initial.quality);
    let width = ui.signal(initial.max_width);
    let folder = ui.signal(initial.open_folder);
    let prefix_error = ui.signal(FieldStatus::Normal);
    let quality_error = ui.signal(FieldStatus::Normal);
    let width_error = ui.signal(FieldStatus::Normal);
    let report = |status: Signal<FieldStatus>| {
        move |_: &mut EventContext, r: Option<&InputRejection>| {
            status.set(r.map_or(FieldStatus::Normal, |r| FieldStatus::error(r.message())))
        }
    };
    let p = current.clone();
    let prefix_control = TextField::new(prefix.clone())
        .normalize(|text| {
            if valid_prefix(&text) {
                Ok(text)
            } else {
                Err(InputRejection::new(
                    "Use 1–32 letters, digits, - or _ only.",
                ))
            }
        })
        .on_rejection(report(prefix_error.clone()))
        .on_save(move |_, v| p.update(|s| s.prefix = v.to_owned()))
        .enabled(editable.clone());
    let q = current.clone();
    // Domain checks run before IntegerInput's built-in validation, allowing a visible message.
    let quality_control = IntegerInput::new(quality.clone())
        .range(1, 100)?
        .normalize(|v| {
            if (1..=100).contains(&v) {
                Ok(v)
            } else {
                Err(InputRejection::new(
                    "JPEG quality must be an integer from 1 to 100.",
                ))
            }
        })?
        .on_rejection(report(quality_error.clone()))
        .on_save(move |_, v| q.update(|s| s.quality = v))
        .enabled(editable.clone());
    let w = current.clone();
    let width_control = IntegerInput::new(width.clone())
        .range(16, 16384)?
        .unit("px")
        .normalize(|v| {
            if (16..=16384).contains(&v) {
                Ok(v)
            } else {
                Err(InputRejection::new("Maximum width must be 16–16384 px."))
            }
        })?
        .on_rejection(report(width_error.clone()))
        .on_save(move |_, v| w.update(|s| s.max_width = v))
        .enabled(editable.clone());
    let f = current.clone();
    let format_control = RadioGroup::new(
        format.clone(),
        [
            Choice::new("PNG".to_owned(), "PNG"),
            Choice::new("JPEG".to_owned(), "JPEG"),
            Choice::new("WebP".to_owned(), "WebP"),
        ],
    )?
    .on_change(move |_, v| f.update(|s| s.format = v.clone()))
    .enabled(editable.clone());
    let b = current.clone();
    let folder_control = Toggle::new("Open folder after export", folder.clone())
        .on_change(move |_, v| b.update(|s| s.open_folder = v))
        .enabled(editable.clone());
    let (saved, pending, status) = (applied.clone(), busy.clone(), message.clone());
    let mailbox = ui.mailbox(
        Capacity::new(1)?,
        move |(settings, result): (Settings, Result<(), String>)| {
            pending.set(false)?;
            match result {
                Ok(()) => {
                    saved.set(settings)?;
                    status.set("Settings saved".into())
                }
                Err(e) => status.set(format!("Save failed: {e}")),
            }
        },
    )?;
    let enabled = dirty.combine(&busy.read_only(), |d, b| *d && !b)?;
    let sender = mailbox.sender();
    let (draft, saving, status) = (current.clone(), busy.clone(), message.clone());
    let apply = Button::new("Apply", move |_| {
        let snapshot = draft.get()?;
        saving.set(true)?;
        status.set("Saving…".into())?;
        let (sender, path) = (sender.clone(), path.clone());
        let failed = std::thread::Builder::new()
            .name("settings-save".into())
            .spawn(move || {
                let result = (|| {
                    let json = serde_json::to_vec_pretty(&snapshot).map_err(|e| e.to_string())?;
                    // Replace atomically in the same directory, retaining the old file on failure.
                    let temp = path.with_extension("json.tmp");
                    std::fs::write(&temp, json).map_err(|e| e.to_string())?;
                    std::fs::rename(&temp, &path).map_err(|e| e.to_string())
                })();
                // One worker at a time, capacity one: Full cannot occur. Closed means UI shutdown.
                let _ = sender.send((snapshot, result));
            });
        if let Err(e) = failed {
            saving.set(false)?;
            status.set(format!("Save failed: {e}"))?;
        }
        Ok(())
    })
    .enabled(enabled.clone());
    let (base, draft, status) = (applied.clone(), current.clone(), message.clone());
    let (pe, qe, we) = (
        prefix_error.clone(),
        quality_error.clone(),
        width_error.clone(),
    );
    let revert = Button::new("Revert", move |_| {
        let s = base.get()?;
        prefix.set(s.prefix.clone())?;
        format.set(s.format.clone())?;
        quality.set(s.quality)?;
        width.set(s.max_width)?;
        folder.set(s.open_folder)?;
        draft.set(s)?;
        pe.set(FieldStatus::Normal)?;
        qe.set(FieldStatus::Normal)?;
        we.set(FieldStatus::Normal)?;
        status.set("Reverted to applied settings".into())
    })
    .enabled(enabled);
    let heading = |text| Label::new(text).style(TextStyle::Heading);
    let content = Column::new([
        heading("Output").key("output-title"),
        Field::new(prefix_control.key("prefix"))
            .label("File name prefix")
            .help("1–32 letters, digits, - or _")
            .status(prefix_error)
            .key("prefix-field"),
        Field::new(format_control.key("format"))
            .label("Format")
            .key("format-field"),
        heading("Quality").key("quality-title"),
        Field::new(quality_control.key("quality"))
            .label("JPEG quality")
            .help("Enter an integer from 1 to 100")
            .status(quality_error)
            .key("quality-field"),
        Field::new(width_control.key("width"))
            .label("Maximum width")
            .help("16–16384 px (integer)")
            .status(width_error)
            .key("width-field"),
        heading("Behavior").key("behavior-title"),
        folder_control.key("open-folder"),
        Label::new(marker).key("dirty-marker"),
        Row::new([apply.key("apply"), revert.key("revert")]).key("actions"),
        Label::new(message.read_only()).key("message"),
    ]);
    let close = guard.clone();
    let window = FloatingWindow::new(
        ui,
        "settings-window",
        "Photo export settings",
        ScrollArea::new(content.key("settings"))
            .axes(ScrollAxes::Both)
            .key("scroll"),
    )
    .initial_size(Size::new(660.0, 850.0)?)?
    .on_close(move |_| {
        Ok(if close.allow()? {
            WindowCloseAction::Close
        } else {
            WindowCloseAction::KeepOpen
        })
    });
    let root = ui.mount_tree(WindowLayer::new([window]).key("windows"))?;
    #[cfg(not(test))]
    let _ = root;
    Ok(Editor {
        _mailbox: mailbox,
        guard,
        #[cfg(test)]
        root,
    })
}
fn main() -> Result<(), AppError> {
    let path = std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("settings.json"));
    let state: Rc<RefCell<Option<Editor>>> = Rc::default();
    let init = state.clone();
    App::new("Photo export settings", move |ui| {
        *init.borrow_mut() = Some(mount(ui, path)?);
        Ok(())
    })
    .size(Size::new(720.0, 920.0)?)?
    .on_close(move |ui, _| {
        ui.clear_focus();
        Ok(
            if state
                .borrow()
                .as_ref()
                .expect("initialized")
                .guard
                .allow()?
            {
                NativeCloseAction::Close
            } else {
                NativeCloseAction::KeepOpen
            },
        )
    })
    .run()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};
    type TestResult = Result<(), Box<dyn std::error::Error>>;
    struct Harness {
        ui: Runtime,
        editor: Editor,
    }
    impl Harness {
        fn new(path: PathBuf) -> Result<Self, RuntimeError> {
            let mut ui = Runtime::new();
            ui.set_viewport(Viewport::new(720.0, 920.0, 1.0)?)?;
            let editor = mount(&mut ui, path)?;
            let mut h = Self { ui, editor };
            h.flush()?;
            Ok(h)
        }
        fn flush(&mut self) -> Result<(), RuntimeError> {
            self.ui.flush(Duration::ZERO)?;
            self.ui.paint().expect("headless paint succeeds");
            Ok(())
        }
        fn id(&self, key: &str) -> WidgetId {
            self.ui
                .find_descendant(&self.editor.root, key)
                .unwrap()
                .unwrap_or_else(|| panic!("missing {key}"))
        }
        fn component(&self, key: &str) -> ComponentInspection {
            self.ui
                .inspect()
                .components
                .into_iter()
                .find(|c| c.key == key)
                .unwrap_or_else(|| panic!("missing {key}"))
        }
        fn semantics(&self, key: &str) -> Semantics {
            self.component(key).details.semantics.unwrap()
        }
        fn text(&self, key: &str) -> String {
            match self.semantics(key).value {
                Some(SemanticValue::Text(s)) => s,
                None if self.semantics(key).name.is_some() => self.semantics(key).name.unwrap(),
                v => panic!("{key}: {v:?}"),
            }
        }
        fn key(&mut self, key: KeyCode) -> Result<(), RuntimeError> {
            self.ui.key_input(key, KeyAction::Press)?;
            self.ui.key_input(key, KeyAction::Release)?;
            self.flush()
        }
        fn click(&mut self, key: &str) -> Result<(), RuntimeError> {
            let id = self.id(key);
            self.ui.focus(&id)?;
            self.key(KeyCode::Enter)
        }
        fn edit(&mut self, key: &str, text: &str) -> Result<(), RuntimeError> {
            let id = self.id(key);
            self.ui.focus(&id)?;
            self.ui.text_command(&id, TextCommand::SelectAll)?;
            self.ui.text_input(text)?;
            self.key(KeyCode::Enter)
        }
        fn dirty(&self, dirty: bool) {
            for key in ["apply", "revert"] {
                assert_eq!(self.semantics(key).disabled, !dirty, "{key}");
            }
            assert_eq!(self.text("dirty-marker").contains("Unsaved"), dirty);
        }
        fn await_save(&mut self) -> Result<(), RuntimeError> {
            let start = Instant::now();
            loop {
                self.flush()?;
                if !self.text("message").starts_with("Saving") {
                    break;
                }
                assert!(
                    start.elapsed() < Duration::from_secs(10),
                    "worker did not finish"
                );
                std::thread::sleep(Duration::from_millis(5));
            }
            Ok(())
        }
    }
    fn test_dir(name: &str) -> PathBuf {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("test-output")
            .join(name);
        std::fs::create_dir_all(&path).unwrap();
        path
    }
    #[test]
    fn loaded_fields_validation_dirty_revert_save_and_close() -> TestResult {
        let path = test_dir("success").join("settings.json");
        let initial = Settings {
            prefix: "loaded_42".into(),
            format: "JPEG".into(),
            quality: 73,
            max_width: 4096,
            open_folder: false,
        };
        std::fs::write(&path, serde_json::to_vec(&initial)?)?;
        let mut h = Harness::new(path.clone())?;
        for key in [
            "output-title",
            "quality-title",
            "behavior-title",
            "prefix",
            "format",
            "quality",
            "width",
            "open-folder",
            "apply",
            "revert",
        ] {
            assert!(h.component(key).visible, "{key}");
        }
        assert_eq!(h.text("prefix"), "loaded_42");
        assert_eq!(
            h.semantics("open-folder").value,
            Some(SemanticValue::Bool(false))
        );
        assert_eq!(h.text("output-title"), "Output");
        assert_eq!(h.text("quality-title"), "Quality");
        assert_eq!(h.text("behavior-title"), "Behavior");
        assert_eq!(
            h.semantics("quality").value,
            Some(SemanticValue::Number {
                value: 73.0,
                minimum: Some(1.0),
                maximum: Some(100.0)
            })
        );
        assert_eq!(
            h.semantics("width").value,
            Some(SemanticValue::Number {
                value: 4096.0,
                minimum: Some(16.0),
                maximum: Some(16384.0)
            })
        );
        assert_eq!(
            h.component("width").details.properties.get("unit"),
            Some(&InspectValue::Text("px".into()))
        );
        assert_eq!(
            h.semantics("format").value,
            Some(SemanticValue::Choice {
                selected: Some(1),
                count: 3,
                label: Some("JPEG".into())
            })
        );
        h.dirty(false);
        h.edit("prefix", "bad prefix!")?;
        assert!(h.semantics("prefix").invalid);
        assert!(
            h.semantics("prefix-field")
                .description
                .unwrap()
                .contains("1–32")
        );
        h.dirty(false);
        h.key(KeyCode::Escape)?;
        assert_eq!(h.text("prefix"), "loaded_42");
        h.edit("quality", "101")?;
        assert!(h.semantics("quality").invalid);
        assert!(
            h.semantics("quality-field")
                .description
                .unwrap()
                .contains("1 to 100")
        );
        assert_eq!(
            h.component("quality").details.properties.get("value"),
            Some(&InspectValue::Text("73".into()))
        );
        h.dirty(false);
        h.key(KeyCode::Escape)?;
        assert_eq!(
            h.component("quality").details.properties.get("value"),
            Some(&InspectValue::Text("73".into()))
        );
        h.edit("prefix", "edited")?;
        h.edit("quality", "88")?;
        h.edit("width", "3000")?;
        h.click("open-folder")?;
        let format = h.id("format");
        h.ui.focus(&format)?;
        h.key(KeyCode::ArrowRight)?;
        h.dirty(true);
        // Use the library's actual decorated-window close button, not the guard directly.
        let close = h
            .ui
            .inspect()
            .components
            .into_iter()
            .find(|c| {
                c.details.semantics.as_ref().and_then(|s| s.name.as_deref()) == Some("Close window")
            })
            .expect("standard window close button");
        h.click(&close.key)?;
        assert!(h.text("message").contains("apply or revert"));
        assert!(h.component("prefix").visible);
        h.dirty(true);
        h.click("revert")?;
        assert_eq!(h.text("prefix"), "loaded_42");
        assert_eq!(
            h.component("quality").details.properties.get("value"),
            Some(&InspectValue::Text("73".into()))
        );
        assert_eq!(
            h.component("width").details.properties.get("value"),
            Some(&InspectValue::Text("4096".into()))
        );
        assert_eq!(
            h.semantics("open-folder").value,
            Some(SemanticValue::Bool(false))
        );
        assert_eq!(
            h.semantics("format").value,
            Some(SemanticValue::Choice {
                selected: Some(1),
                count: 3,
                label: Some("JPEG".into())
            })
        );
        h.dirty(false);
        h.edit("quality", "91")?;
        h.edit("prefix", "saved")?;
        h.click("apply")?;
        h.await_save()?;
        assert_eq!(h.text("message"), "Settings saved");
        h.dirty(false);
        let saved: Settings = serde_json::from_slice(&std::fs::read(&path)?)?;
        assert_eq!(
            saved,
            Settings {
                prefix: "saved".into(),
                quality: 91,
                ..initial
            }
        );
        let loaded = Harness::new(path)?;
        assert_eq!(loaded.text("prefix"), "saved");
        Ok(())
    }
    #[test]
    fn failed_save_stays_dirty() -> TestResult {
        // A regular file used as the parent guarantees failure even when running as root.
        let dir = test_dir("failure");
        let parent = dir.join("not-a-directory");
        std::fs::write(&parent, "file")?;
        let mut h = Harness::new(parent.join("settings.json"))?;
        h.dirty(false);
        h.edit("prefix", "unsaved")?;
        h.dirty(true);
        h.click("apply")?;
        h.await_save()?;
        assert!(h.text("message").starts_with("Save failed:"));
        h.dirty(true);
        h.click("revert")?;
        assert_eq!(h.text("prefix"), "photo");
        h.dirty(false);
        Ok(())
    }
}
