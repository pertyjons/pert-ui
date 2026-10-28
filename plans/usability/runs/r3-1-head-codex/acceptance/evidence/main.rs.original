use pert_ui::prelude::*;
use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
enum Format {
    PNG,
    JPEG,
    WebP,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct Settings {
    prefix: String,
    format: Format,
    quality: i64,
    max_width: i64,
    open_folder: bool,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            prefix: "photo".into(),
            format: Format::JPEG,
            quality: 85,
            max_width: 2048,
            open_folder: true,
        }
    }
}
fn valid_prefix(s: &str) -> bool {
    (1..=32).contains(&s.len())
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}
impl Settings {
    fn valid(&self) -> bool {
        valid_prefix(&self.prefix)
            && (1..=100).contains(&self.quality)
            && (16..=16384).contains(&self.max_width)
    }
}

#[derive(Clone)]
struct Values {
    prefix: Signal<String>,
    format: Signal<Format>,
    quality: Signal<i64>,
    width: Signal<i64>,
    open: Signal<bool>,
}
impl Values {
    fn get(&self) -> Result<Settings, RuntimeError> {
        Ok(Settings {
            prefix: self.prefix.get()?,
            format: self.format.get()?,
            quality: self.quality.get()?,
            max_width: self.width.get()?,
            open_folder: self.open.get()?,
        })
    }
    fn set(&self, s: Settings) -> Result<(), RuntimeError> {
        self.prefix.set(s.prefix)?;
        self.format.set(s.format)?;
        self.quality.set(s.quality)?;
        self.width.set(s.max_width)?;
        self.open.set(s.open_folder)
    }
}
struct Editor {
    root: WidgetId,
    values: Values,
    applied: Signal<Settings>,
    dirty: ReadSignal<bool>,
    message: Signal<String>,
}
fn build(ui: &mut Runtime, path: PathBuf) -> Result<Editor, RuntimeError> {
    let (saved, startup) = match fs::read(&path) {
        Ok(bytes) => match serde_json::from_slice::<Settings>(&bytes) {
            Ok(s) if s.valid() => (s, "Settings loaded".into()),
            Ok(_) => (
                Settings::default(),
                "Load failed: settings contain invalid values; showing defaults".into(),
            ),
            Err(e) => (
                Settings::default(),
                format!("Load failed: {e}; showing defaults"),
            ),
        },
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => (Settings::default(), "Ready".into()),
        Err(e) => (
            Settings::default(),
            format!("Load failed: {e}; showing defaults"),
        ),
    };
    let v = Values {
        prefix: ui.signal(saved.prefix.clone()),
        format: ui.signal(saved.format.clone()),
        quality: ui.signal(saved.quality),
        width: ui.signal(saved.max_width),
        open: ui.signal(saved.open_folder),
    };
    let applied = ui.signal(saved);
    let message = ui.signal(startup);
    let busy = ui.signal(false);
    let form = FormFields::new(ui);
    let pf = form.field("prefix")?;
    let qf = form.field("quality")?;
    let wf = form.field("width")?;
    let changed = ReadSignal::combine(
        (
            &v.prefix, &v.format, &v.quality, &v.width, &v.open, &applied,
        ),
        |(p, f, q, w, o, a)| {
            *p != a.prefix
                || *f != a.format
                || *q != a.quality
                || *w != a.max_width
                || *o != a.open_folder
        },
    )?;
    let dirty = ReadSignal::combine((&changed, &form.summary()), |(c, s)| *c || s.modified() > 0)?;
    let enabled = ReadSignal::combine((&dirty, &busy), |(d, b)| *d && !*b)?;
    let marker: ReadSignal<String> = dirty.map(|d| {
        if *d {
            "Unsaved changes".into()
        } else {
            "All changes saved".into()
        }
    })?;
    let (status, mark) = (pf.clone(), pf.clone());
    let prefix = TextField::new(&v.prefix)
        .normalize(|s| {
            if valid_prefix(&s) {
                Ok(s.to_owned())
            } else {
                Err(InputRejection::new("Use 1–32 letters, digits, - or _ only"))
            }
        })
        .on_rejection(move |_, r| match r {
            Some(r) => status.set(FieldStatus::error(r.to_string())),
            None => status.clear().map(|_| ()),
        })
        .on_modified(move |_, m| mark.set_modified(m).map(|_| ()));
    let (status, mark) = (qf.clone(), qf.clone());
    let quality = IntegerInput::new(&v.quality)
        .range(1, 100)?
        .on_rejection(move |_, r| match r {
            Some(r) => status.set(FieldStatus::error(r.to_string())),
            None => status.clear().map(|_| ()),
        })
        .on_modified(move |_, m| mark.set_modified(m).map(|_| ()));
    let (status, mark) = (wf.clone(), wf.clone());
    let width = IntegerInput::new(&v.width)
        .range(16, 16384)?
        .on_rejection(move |_, r| match r {
            Some(r) => status.set(FieldStatus::error(r.to_string())),
            None => status.clear().map(|_| ()),
        })
        .on_modified(move |_, m| mark.set_modified(m).map(|_| ()));
    let (values, record, msg, saving) = (v.clone(), applied.clone(), message.clone(), busy.clone());
    let apply = Button::new("Apply", move |cx| {
        let snapshot = values.get()?;
        let (record, msg, saving) = (record.clone(), msg.clone(), saving.clone());
        let path = path.clone();
        saving.set(true)?;
        msg.set("Saving…".into())?;
        cx.spawn(
            move || {
                let result = serde_json::to_vec_pretty(&snapshot)
                    .map_err(|e| e.to_string())
                    .and_then(|bytes| fs::write(path, bytes).map_err(|e| e.to_string()));
                (snapshot, result)
            },
            move |(snapshot, result)| {
                saving.set(false)?;
                match result {
                    Ok(()) => {
                        record.set(snapshot)?;
                        msg.set("Settings saved".into())
                    }
                    Err(e) => msg.set(format!("Save failed: {e}. Apply to retry or Revert.")),
                }
            },
        )?;
        Ok(())
    })
    .enabled(&enabled);
    let (values, record, msg) = (v.clone(), applied.clone(), message.clone());
    let (rp, rq, rw) = (pf.clone(), qf.clone(), wf.clone());
    let drafts = form.clone();
    let revert = Button::new("Revert", move |cx| {
        values.set(record.get()?)?;
        for handle in [&rp, &rq, &rw] {
            handle.clear()?;
        }
        drafts.discard_drafts(cx)?;
        msg.set("Changes reverted".into())
    })
    .enabled(&enabled);
    let root = ui.mount_tree(
        Column::new([
            Label::new("Photo export settings").key("title"),
            Column::new([
                Label::new("Output").key("title"),
                Field::new(prefix.key("input"))
                    .label("File name prefix")
                    .form(&pf)
                    .key("prefix"),
                Field::new(
                    SegmentedControl::new(
                        &v.format,
                        [
                            Choice::new(Format::PNG, "PNG"),
                            Choice::new(Format::JPEG, "JPEG"),
                            Choice::new(Format::WebP, "WebP"),
                        ],
                    )?
                    .key("input"),
                )
                .label("Format")
                .key("format"),
            ])
            .gap(8)
            .key("output"),
            Column::new([
                Label::new("Quality").key("title"),
                Field::new(quality.key("input"))
                    .label("JPEG quality (1–100)")
                    .form(&qf)
                    .key("quality"),
                Field::new(
                    Row::new([
                        width.key("input").with_grow(1),
                        Label::new("px").key("unit"),
                    ])
                    .key("row"),
                )
                .label("Maximum width")
                .form(&wf)
                .key("width"),
            ])
            .gap(8)
            .key("quality-group"),
            Column::new([
                Label::new("Behavior").key("title"),
                Toggle::new(&v.open, "Open folder after export").key("open"),
            ])
            .gap(8)
            .key("behavior"),
            Label::new(marker).key("dirty"),
            Row::new([apply.key("apply"), revert.key("revert")]).key("buttons"),
            Label::new(message.read_only()).key("message"),
        ])
        .gap(16)
        .key("settings"),
    )?;
    Ok(Editor {
        root,
        values: v,
        applied,
        dirty,
        message,
    })
}

fn app(path: PathBuf) -> App<Editor> {
    App::new("Photo export settings", move |ui| Ok(build(ui, path)?))
        .size(Size::from([760, 900]))
        .expect("valid initial size")
        .min_size(Size::from([600, 800]))
        .expect("valid minimum size")
        .on_close(|_, editor, _| {
            if editor.dirty.get()? {
                editor
                    .message
                    .set("Unsaved changes: apply or revert before closing.".into())?;
                Ok(NativeCloseAction::KeepOpen)
            } else {
                Ok(NativeCloseAction::Close)
            }
        })
}
fn main() -> Result<(), pert_ui::AppError> {
    let path = std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| "settings.json".into());
    app(path).run()
}

#[cfg(test)]
mod tests {
    use super::*;
    use pert_ui::testing::*;
    use std::time::Duration;
    type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;

    fn fixture(name: &str) -> Result<(PathBuf, Settings)> {
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("test-output")
            .join(name);
        fs::create_dir_all(&dir)?;
        let path = dir.join("settings.json");
        let saved = Settings {
            prefix: "loaded_42".into(),
            format: Format::WebP,
            quality: 72,
            max_width: 4096,
            open_folder: false,
        };
        fs::write(&path, serde_json::to_vec_pretty(&saved)?)?;
        Ok((path, saved))
    }
    fn sem(d: &Driver<'_>, root: &WidgetId, path: &[&str]) -> Result<Semantics> {
        Ok(d.semantics(&d.find(root, path)?)?)
    }
    fn buttons(d: &Driver<'_>, root: &WidgetId, enabled: bool) -> Result {
        for name in ["apply", "revert"] {
            assert_eq!(
                sem(d, root, &["buttons", name])?.disabled,
                !enabled,
                "{name}"
            );
        }
        Ok(())
    }
    fn number(s: Semantics, expected: f64) {
        assert!(
            matches!(s.value, Some(SemanticValue::Number { value, .. }) if value == expected),
            "{s:?}"
        );
    }

    #[test]
    fn loaded_fields_validation_revert_and_close() -> Result {
        let (path, saved) = fixture("fields")?;
        let mut app = app(path).headless()?;
        let main = app.main_window();
        let (ui, e) = app.runtime_and_state_mut();
        let mut d = Driver::new(ui);
        let r = e.root.clone();
        for (group, title) in [
            ("output", "Output"),
            ("quality-group", "Quality"),
            ("behavior", "Behavior"),
        ] {
            assert_eq!(sem(&d, &r, &[group, "title"])?.name.as_deref(), Some(title));
        }
        let p = d.find(&r, &["output", "prefix", "input"])?;
        let q = d.find(&r, &["quality-group", "quality", "input"])?;
        let w = d.find(&r, &["quality-group", "width", "row", "input"])?;
        let toggle = d.find(&r, &["behavior", "open"])?;
        let revert = d.find(&r, &["buttons", "revert"])?;
        assert_eq!(
            d.semantics(&p)?.value,
            Some(SemanticValue::Text(saved.prefix.clone()))
        );
        number(d.semantics(&q)?, 72.0);
        number(d.semantics(&w)?, 4096.0);
        assert_eq!(
            sem(&d, &r, &["quality-group", "width", "row", "unit"])?
                .name
                .as_deref(),
            Some("px")
        );
        assert!(matches!(
            sem(&d, &r, &["output", "format", "input"])?.value,
            Some(SemanticValue::Choice {
                selected: Some(2),
                count: 3,
                ..
            })
        ));
        assert_eq!(
            d.semantics(&toggle)?.value,
            Some(SemanticValue::Bool(false))
        );
        buttons(&d, &r, false)?;
        d.replace_text(&p, "bad prefix!")?;
        d.key(KeyCode::Enter)?;
        assert_eq!(e.values.prefix.get()?, saved.prefix);
        let s = d.semantics(&p)?;
        assert!(s.invalid);
        assert!(s.description.unwrap().contains("1–32 letters"));
        buttons(&d, &r, true)?;
        d.replace_text(&q, "101")?;
        d.key(KeyCode::Enter)?;
        assert_eq!(e.values.quality.get()?, 72);
        let s = d.semantics(&q)?;
        assert!(s.invalid);
        assert!(s.description.as_ref().is_some_and(|s| !s.is_empty()));
        println!("Refused quality: {:?}", s.description);
        assert_eq!(
            sem(&d, &r, &["dirty"])?.name.as_deref(),
            Some("Unsaved changes")
        );
        assert!(matches!(
            app.admit_close(&main)?,
            NativeCloseAction::KeepOpen
        ));
        let (ui, e) = app.runtime_and_state_mut();
        let mut d = Driver::new(ui);
        d.settle()?;
        assert!(
            sem(&d, &r, &["message"])?
                .name
                .unwrap()
                .contains("apply or revert")
        );
        d.click(&revert)?;
        assert_eq!(e.values.get()?, saved);
        assert!(!d.semantics(&p)?.invalid);
        assert!(!d.semantics(&q)?.invalid);
        buttons(&d, &r, false)?;
        d.replace_text(&p, "edited")?;
        d.key(KeyCode::Enter)?;
        d.replace_text(&q, "99")?;
        d.key(KeyCode::Enter)?;
        d.click(&toggle)?;
        buttons(&d, &r, true)?;
        d.click(&revert)?;
        assert_eq!(e.values.get()?, saved);
        assert_eq!(
            d.semantics(&p)?.value,
            Some(SemanticValue::Text(saved.prefix))
        );
        number(d.semantics(&q)?, 72.0);
        buttons(&d, &r, false)?;
        assert!(matches!(app.admit_close(&main)?, NativeCloseAction::Close));
        app.shutdown()?;
        Ok(())
    }

    #[test]
    fn background_save_success_failure_and_retry() -> Result {
        let (path, _) = fixture("save")?;
        let mut app = app(path.clone()).headless()?;
        let (ui, e) = app.runtime_and_state_mut();
        let mut d = Driver::new(ui);
        let r = e.root.clone();
        let p = d.find(&r, &["output", "prefix", "input"])?;
        let apply = d.find(&r, &["buttons", "apply"])?;
        d.replace_text(&p, "exported")?;
        d.key(KeyCode::Enter)?;
        d.click(&apply)?;
        d.wait_until(Duration::from_secs(5), |_| {
            Ok(e.message.get()? == "Settings saved")
        })?;
        assert_eq!(
            serde_json::from_slice::<Settings>(&fs::read(&path)?)?,
            e.values.get()?
        );
        assert_eq!(e.applied.get()?.prefix, "exported");
        buttons(&d, &r, false)?;
        assert_eq!(
            sem(&d, &r, &["dirty"])?.name.as_deref(),
            Some("All changes saved")
        );
        // A directory at the file path fails deterministically even as root.
        fs::remove_file(&path)?;
        fs::create_dir(&path)?;
        d.replace_text(&p, "retry_me")?;
        d.key(KeyCode::Enter)?;
        d.click(&apply)?;
        d.wait_until(Duration::from_secs(5), |_| {
            Ok(e.message.get()?.starts_with("Save failed:"))
        })?;
        assert!(
            sem(&d, &r, &["message"])?
                .name
                .unwrap()
                .starts_with("Save failed:")
        );
        assert!(e.dirty.get()?);
        assert_eq!(e.applied.get()?.prefix, "exported");
        buttons(&d, &r, true)?;
        fs::remove_dir(&path)?;
        d.click(&apply)?;
        d.wait_until(Duration::from_secs(5), |_| {
            Ok(e.message.get()? == "Settings saved")
        })?;
        assert_eq!(
            serde_json::from_slice::<Settings>(&fs::read(&path)?)?.prefix,
            "retry_me"
        );
        buttons(&d, &r, false)?;
        app.shutdown()?;
        Ok(())
    }
}
