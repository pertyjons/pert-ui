use pert_ui::prelude::*;
use serde::{Deserialize, Serialize};
use std::{cell::RefCell, path::PathBuf, rc::Rc};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct Settings {
    prefix: String,
    format: String,
    quality: i64,
    width: i64,
    open_folder: bool,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            prefix: "photo".into(),
            format: "PNG".into(),
            quality: 90,
            width: 2048,
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
    let result = std::fs::read(path)
        .map_err(|e| e.to_string())
        .and_then(|data| {
            let s: Settings = serde_json::from_slice(&data).map_err(|e| e.to_string())?;
            if valid_prefix(&s.prefix)
                && ["PNG", "JPEG", "WebP"].contains(&s.format.as_str())
                && (1..=100).contains(&s.quality)
                && (16..=16384).contains(&s.width)
            {
                Ok(s)
            } else {
                Err("Settings file contains invalid values".into())
            }
        });
    match result {
        Ok(s) => (s, "Settings loaded".into()),
        Err(_) if !path.exists() => (Settings::default(), "Using defaults".into()),
        Err(e) => (Settings::default(), format!("Load failed: {e}")),
    }
}
#[derive(Clone)]
struct Built {
    #[cfg_attr(not(test), allow(dead_code))]
    root: WidgetId,
    dirty: ReadSignal<bool>,
    busy: Signal<bool>,
    status: Signal<String>,
}
fn build(ui: &mut Runtime, path: PathBuf) -> Result<Built, RuntimeError> {
    let (initial, message) = load(&path);
    let prefix = ui.signal(initial.prefix.clone());
    let format = ui.signal(initial.format.clone());
    let quality = ui.signal(initial.quality);
    let width = ui.signal(initial.width);
    let open = ui.signal(initial.open_folder);
    let applied = ui.signal(initial);
    let status = ui.signal(message);
    let busy = ui.signal(false);
    let editable = busy.map(|b| !b)?;
    let current = prefix
        .read_only()
        .combine(&format.read_only(), |p, f| (p.clone(), f.clone()))?
        .combine(&quality.read_only(), |(p, f), q| (p.clone(), f.clone(), *q))?
        .combine(&width.read_only(), |(p, f, q), w| {
            (p.clone(), f.clone(), *q, *w)
        })?
        .combine(&open.read_only(), |(p, f, q, w), o| Settings {
            prefix: p.clone(),
            format: f.clone(),
            quality: *q,
            width: *w,
            open_folder: *o,
        })?;
    let dirty = current.combine(&applied.read_only(), |a, b| a != b)?;
    let can_act = dirty.combine(&busy.read_only(), |d, b| *d && !*b)?;
    let marker = dirty.map(|d| {
        if *d {
            "Unsaved changes".to_owned()
        } else {
            "All changes saved".to_owned()
        }
    })?;
    let prefix_error = ui.signal(FieldStatus::Normal);
    let quality_error = ui.signal(FieldStatus::Normal);
    let width_error = ui.signal(FieldStatus::Normal);
    let pe = prefix_error.clone();
    let qe = quality_error.clone();
    let we = width_error.clone();
    let prefix_input = TextField::new(&prefix)
        .enabled(&editable)
        .normalize(|s| {
            if valid_prefix(&s) {
                Ok(s)
            } else {
                Err(InputRejection::new("Use 1–32 letters, digits, - or _"))
            }
        })
        .on_rejection(move |_, r| {
            pe.set(r.map_or(FieldStatus::Normal, |e| FieldStatus::error(e.message())))
        });
    let quality_input = IntegerInput::new(&quality)
        .range(1, 100)?
        .enabled(&editable)
        .on_rejection(move |_, r| {
            qe.set(r.map_or(FieldStatus::Normal, |_| {
                FieldStatus::error("JPEG quality must be an integer from 1 to 100")
            }))
        });
    let width_input = IntegerInput::new(&width)
        .range(16, 16384)?
        .unit("px")
        .enabled(&editable)
        .on_rejection(move |_, r| {
            we.set(r.map_or(FieldStatus::Normal, |_| {
                FieldStatus::error("Width must be an integer from 16 to 16384 px")
            }))
        });
    let (save_current, save_applied, save_status, save_busy) = (
        current.clone(),
        applied.clone(),
        status.clone(),
        busy.clone(),
    );
    let apply = Button::new("Apply", move |cx| {
        let value = save_current.get()?;
        let snapshot = value.clone();
        let path = path.clone();
        let (applied, status, busy) =
            (save_applied.clone(), save_status.clone(), save_busy.clone());
        busy.set(true)?;
        status.set("Saving…".into())?;
        let _start = cx.spawn(
            move || -> Result<(), String> {
                let data = serde_json::to_vec_pretty(&value).map_err(|e| e.to_string())?;
                std::fs::write(path, data).map_err(|e| e.to_string())
            },
            move |result| {
                busy.set(false)?;
                match result {
                    Ok(()) => {
                        applied.set(snapshot)?;
                        status.set("Settings saved".into())
                    }
                    Err(e) => status.set(format!("Save failed: {e}")),
                }
            },
        )?;
        Ok(())
    })
    .enabled(&can_act);
    let revert_status = status.clone();
    let editors: Rc<RefCell<Vec<WidgetId>>> = Rc::default();
    let discard = editors.clone();
    let (rp, rf, rq, rw, ro) = (
        prefix.clone(),
        format.clone(),
        quality.clone(),
        width.clone(),
        open.clone(),
    );
    let (rpe, rqe, rwe) = (
        prefix_error.clone(),
        quality_error.clone(),
        width_error.clone(),
    );
    let revert = Button::new("Revert", move |cx| {
        let s = applied.get()?;
        rp.set(s.prefix)?;
        rf.set(s.format)?;
        rq.set(s.quality)?;
        rw.set(s.width)?;
        ro.set(s.open_folder)?;
        rpe.set(FieldStatus::Normal)?;
        rqe.set(FieldStatus::Normal)?;
        rwe.set(FieldStatus::Normal)?;
        revert_status.set("Changes reverted".into())?;
        let discard = discard.clone();
        let _discard = cx.defer(move |ui| {
            for field in discard.borrow().iter() {
                ui.text_command(field, pert_ui::TextCommand::DiscardDraft)?;
            }
            Ok(())
        })?;
        Ok(())
    })
    .enabled(&can_act);
    let root = ui.mount_tree(
        ScrollArea::new(
            Column::new([
                Label::new("Photo export settings").key("heading"),
                Column::new([
                    Label::new("Output").key("title"),
                    Field::new(prefix_input.key("prefix"))
                        .label("File name prefix")
                        .status(&prefix_error)
                        .key("prefix-field"),
                    Field::new(
                        SegmentedControl::new(
                            &format,
                            [
                                Choice::new("PNG".to_owned(), "PNG"),
                                Choice::new("JPEG".to_owned(), "JPEG"),
                                Choice::new("WebP".to_owned(), "WebP"),
                            ],
                        )?
                        .enabled(&editable)
                        .key("format"),
                    )
                    .label("Format")
                    .key("format-field"),
                ])
                .key("output"),
                Column::new([
                    Label::new("Quality").key("title"),
                    Field::new(quality_input.key("quality"))
                        .label("JPEG quality")
                        .status(&quality_error)
                        .key("quality-field"),
                    Field::new(width_input.key("width"))
                        .label("Maximum width (px)")
                        .status(&width_error)
                        .key("width-field"),
                ])
                .key("quality-group"),
                Column::new([
                    Label::new("Behavior").key("title"),
                    Toggle::new(&open, "Open folder after export")
                        .enabled(&editable)
                        .key("open-folder"),
                ])
                .key("behavior"),
                Label::new(marker).key("dirty"),
                Row::new([apply.key("apply"), revert.key("revert")]).key("actions"),
                Label::new(status.read_only()).key("status"),
            ])
            .key("settings"),
        )
        .axes(ScrollAxes::Both)
        .key("viewport"),
    )?;
    for key in ["prefix", "quality", "width"] {
        if let Some(id) = ui.find_descendant(&root, key)? {
            editors.borrow_mut().push(id);
        }
    }
    Ok(Built {
        root,
        dirty,
        busy,
        status,
    })
}
fn application(path: PathBuf, slot: Rc<RefCell<Option<Built>>>) -> App {
    let policy = slot.clone();
    App::new("Photo export settings", move |ui| {
        *slot.borrow_mut() = Some(build(ui, path)?);
        Ok(())
    })
    .size(Size::new(720.0, 820.0).expect("valid size"))
    .expect("valid size")
    .on_close(move |_, _| {
        if let Some(b) = policy.borrow().as_ref() {
            if b.dirty.get()? || b.busy.get()? {
                b.status
                    .set("Unsaved changes: Apply or Revert before closing".into())?;
                return Ok(NativeCloseAction::KeepOpen);
            }
        }
        Ok(NativeCloseAction::Close)
    })
}
fn main() -> Result<(), AppError> {
    let path = std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("settings.json"));
    application(path, Rc::default()).run()
}

#[cfg(test)]
mod tests {
    use super::*;
    use pert_ui::testing::*;
    use std::time::Duration;
    type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;

    fn fixture(name: &str) -> PathBuf {
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("test-output")
            .join(name);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }
    fn find(d: &Driver<'_>, b: &Built, key: &str) -> WidgetId {
        d.runtime()
            .find_descendant(&b.root, key)
            .unwrap()
            .unwrap_or_else(|| panic!("missing {key}"))
    }
    fn runtime_name(ui: &Runtime, b: &Built, key: &str) -> String {
        let widget = ui.find_descendant(&b.root, key).unwrap().unwrap();
        let id = ui.inspection_id(&widget).unwrap();
        ui.query(&InspectionQuery::components(Selector::id(id)).detail(Detail::Semantics))
            .components[0]
            .details
            .semantics
            .as_ref()
            .unwrap()
            .name
            .clone()
            .unwrap_or_default()
    }
    fn name(d: &Driver<'_>, b: &Built, key: &str) -> String {
        d.semantics(&find(d, b, key))
            .unwrap()
            .name
            .unwrap_or_default()
    }
    fn value(d: &Driver<'_>, b: &Built, key: &str) -> SemanticValue {
        d.semantics(&find(d, b, key)).unwrap().value.unwrap()
    }
    fn buttons(d: &Driver<'_>, b: &Built, disabled: bool) {
        for key in ["apply", "revert"] {
            assert_eq!(
                d.semantics(&find(d, b, key)).unwrap().disabled,
                disabled,
                "{key}"
            );
        }
    }
    fn text(d: &mut Driver<'_>, b: &Built, key: &str, value: &str) -> Result {
        let id = find(d, b, key);
        d.replace_text(&id, value)?;
        d.key(KeyCode::Enter)?;
        Ok(())
    }
    fn properties(d: &Driver<'_>, b: &Built, key: &str) -> String {
        let id = d.runtime().inspection_id(&find(d, b, key)).unwrap();
        let query = d
            .runtime()
            .query(&InspectionQuery::components(Selector::id(id)).detail(Detail::Full));
        format!("{:?}", query.components[0].details.properties)
    }
    #[test]
    fn loaded_fields_validation_dirty_revert_save_and_close() -> Result {
        let path = fixture("success").join("settings.json");
        let original = Settings {
            prefix: "loaded_42".into(),
            format: "JPEG".into(),
            quality: 73,
            width: 4096,
            open_folder: false,
        };
        std::fs::write(&path, serde_json::to_vec(&original)?)?;
        let slot = Rc::default();
        let mut app = application(path.clone(), Rc::clone(&slot)).headless()?;
        let b = slot.borrow().as_ref().unwrap().clone();
        {
            let mut d = Driver::new(app.runtime_mut());
            d.settle()?;
            for (key, title) in [
                ("output", "Output"),
                ("quality-group", "Quality"),
                ("behavior", "Behavior"),
            ] {
                let group = find(&d, &b, key);
                let label = d.find(&group, &["title"])?;
                assert_eq!(d.semantics(&label)?.name.as_deref(), Some(title));
            }
            assert_eq!(
                value(&d, &b, "prefix"),
                SemanticValue::Text("loaded_42".into())
            );
            assert!(matches!(
                value(&d, &b, "quality"),
                SemanticValue::Number { value: 73.0, .. }
            ));
            assert!(matches!(
                value(&d, &b, "width"),
                SemanticValue::Number { value: 4096.0, .. }
            ));
            assert_eq!(value(&d, &b, "open-folder"), SemanticValue::Bool(false));
            assert!(format!("{:?}", value(&d, &b, "format")).contains("JPEG"));
            assert!(properties(&d, &b, "width").contains("px"));
            buttons(&d, &b, true);
            text(&mut d, &b, "prefix", "bad prefix!")?;
            assert!(properties(&d, &b, "prefix-field").contains("Use 1–32"));
            assert!(d.semantics(&find(&d, &b, "prefix"))?.invalid);
            d.key(KeyCode::Escape)?;
            assert_eq!(
                value(&d, &b, "prefix"),
                SemanticValue::Text("loaded_42".into())
            );
            text(&mut d, &b, "quality", "101")?;
            assert!(properties(&d, &b, "quality-field").contains("1 to 100"));
            d.key(KeyCode::Escape)?;
            assert!(matches!(
                value(&d, &b, "quality"),
                SemanticValue::Number { value: 73.0, .. }
            ));
            buttons(&d, &b, true);
            text(&mut d, &b, "prefix", "changed")?;
            buttons(&d, &b, false);
            assert_eq!(name(&d, &b, "dirty"), "Unsaved changes");
            // Revert must discard a refused draft even if its valid binding is unchanged.
            text(&mut d, &b, "quality", "999")?;
            assert!(properties(&d, &b, "quality-field").contains("1 to 100"));
            d.click(&find(&d, &b, "revert"))?;
            d.settle()?;
            buttons(&d, &b, true);
            assert_eq!(
                value(&d, &b, "prefix"),
                SemanticValue::Text("loaded_42".into())
            );
            assert!(matches!(
                value(&d, &b, "quality"),
                SemanticValue::Number { value: 73.0, .. }
            ));
            assert!(!d.semantics(&find(&d, &b, "quality"))?.invalid);
            assert!(!properties(&d, &b, "quality-field").contains("1 to 100"));
            text(&mut d, &b, "quality", "81")?;
        }
        let window = app.main_window();
        assert!(matches!(
            app.admit_close(&window)?,
            NativeCloseAction::KeepOpen
        ));
        {
            let mut d = Driver::new(app.runtime_mut());
            d.settle()?;
            assert!(name(&d, &b, "status").contains("Apply or Revert"));
            d.click(&find(&d, &b, "apply"))?;
            d.wait_until(Duration::from_secs(5), |ui| {
                Ok(runtime_name(ui, &b, "status") == "Settings saved")
            })?;
            buttons(&d, &b, true);
            assert_eq!(name(&d, &b, "dirty"), "All changes saved");
        }
        let saved: Settings = serde_json::from_slice(&std::fs::read(&path)?)?;
        assert_eq!(saved.quality, 81);
        assert_eq!(saved.prefix, original.prefix);
        assert!(matches!(
            app.admit_close(&window)?,
            NativeCloseAction::Close
        ));
        app.shutdown()?;
        Ok(())
    }
    #[test]
    fn failed_save_is_visible_and_remains_dirty() -> Result {
        // A directory is a deterministic unwritable file target, including as root.
        let path = fixture("failure");
        let slot = Rc::default();
        let mut app = application(path, Rc::clone(&slot)).headless()?;
        let b = slot.borrow().as_ref().unwrap().clone();
        {
            let mut d = Driver::new(app.runtime_mut());
            text(&mut d, &b, "prefix", "unsaved")?;
            d.click(&find(&d, &b, "apply"))?;
            d.wait_until(Duration::from_secs(5), |ui| {
                Ok(runtime_name(ui, &b, "status").starts_with("Save failed:"))
            })?;
            buttons(&d, &b, false);
            assert_eq!(name(&d, &b, "dirty"), "Unsaved changes");
            assert_eq!(
                value(&d, &b, "prefix"),
                SemanticValue::Text("unsaved".into())
            );
        }
        let window = app.main_window();
        assert!(matches!(
            app.admit_close(&window)?,
            NativeCloseAction::KeepOpen
        ));
        app.shutdown()?;
        Ok(())
    }
}
