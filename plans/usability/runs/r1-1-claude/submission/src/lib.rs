//! Settings editor for a fictional photo-export tool.
//!
//! `build` mounts the whole editor on a `Runtime`, so the native `App` in
//! `main.rs` and the headless tests share exactly the same tree.

use std::path::{Path, PathBuf};

use pert_ui::prelude::*;
use pert_ui::{
    Field, FieldHandle, FieldStatus, Form, FormStatus, Group, InputRejection, IntegerInput,
    NativeCloseAction, NativeWindowId, SegmentedControl, TextField, Toggle,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Format {
    #[serde(rename = "PNG")]
    Png,
    #[serde(rename = "JPEG")]
    Jpeg,
    #[serde(rename = "WebP")]
    WebP,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Settings {
    pub prefix: String,
    pub format: Format,
    pub quality: i64,
    pub max_width: i64,
    pub open_folder: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            prefix: "export".to_owned(),
            format: Format::Png,
            quality: 90,
            max_width: 4096,
            open_folder: false,
        }
    }
}

pub const QUALITY_RANGE: (i64, i64) = (1, 100);
pub const WIDTH_RANGE: (i64, i64) = (16, 16384);

pub fn check_prefix(text: &str) -> Result<(), String> {
    let count = text.chars().count();
    if !(1..=32).contains(&count) {
        return Err("Prefix must be 1 to 32 characters long".to_owned());
    }
    if let Some(bad) = text
        .chars()
        .find(|c| !(c.is_ascii_alphanumeric() || *c == '-' || *c == '_'))
    {
        let what = if bad.is_whitespace() { "Spaces are".to_owned() } else { format!("'{bad}' is") };
        return Err(format!("{what} not allowed: use letters, digits, '-' and '_' only"));
    }
    Ok(())
}

fn check_range(value: i64, (min, max): (i64, i64), what: &str) -> Result<(), String> {
    if (min..=max).contains(&value) {
        Ok(())
    } else {
        Err(format!("{what} must be between {min} and {max}"))
    }
}

impl Settings {
    pub fn validate(&self) -> Result<(), String> {
        check_prefix(&self.prefix)?;
        check_range(self.quality, QUALITY_RANGE, "Quality")?;
        check_range(self.max_width, WIDTH_RANGE, "Maximum width")?;
        Ok(())
    }
}

/// Reads settings from `path`. A missing file gives the defaults; an unreadable
/// or invalid one gives the defaults and an explanation.
pub fn load(path: &Path) -> (Settings, Option<String>) {
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return (Settings::default(), None),
        Err(e) => return (Settings::default(), Some(format!("Could not read {}: {e}", path.display()))),
    };
    match serde_json::from_str::<Settings>(&text) {
        Ok(s) => match s.validate() {
            Ok(()) => (s, None),
            Err(e) => (Settings::default(), Some(format!("Ignored {}: {e}", path.display()))),
        },
        Err(e) => (Settings::default(), Some(format!("Ignored {}: {e}", path.display()))),
    }
}

fn save(path: &Path, settings: &Settings) -> Result<(), String> {
    let text = serde_json::to_string_pretty(settings).map_err(|e| e.to_string())?;
    std::fs::write(path, text).map_err(|e| format!("Could not save {}: {e}", path.display()))
}

/// The bindings the close policy needs after the tree is built.
#[derive(Clone)]
pub struct Editor {
    pub root: WidgetId,
    pub dirty: ReadSignal<bool>,
    pub notice: Signal<String>,
}

impl Editor {
    /// The application's close policy, installed through `App::on_close` and
    /// used by a headless host for `PresentationAction::Close`.
    pub fn close_action(&self) -> Result<NativeCloseAction, RuntimeError> {
        if self.dirty.get()? {
            self.notice
                .set("Unsaved changes: apply or revert them before closing.".to_owned())?;
            Ok(NativeCloseAction::KeepOpen)
        } else {
            Ok(NativeCloseAction::Close)
        }
    }

    pub fn on_close(
        &self,
        _ui: &mut Runtime,
        _window: &NativeWindowId,
    ) -> Result<NativeCloseAction, RuntimeError> {
        self.close_action()
    }
}

fn report(handle: &FieldHandle, rejection: Option<&InputRejection>) -> Result<(), RuntimeError> {
    handle.set(rejection.map_or(FieldStatus::Normal, |r| FieldStatus::error(r.message())))
}

/// Mounts the editor. Settings are loaded from `path` and saved to it.
pub fn build(ui: &mut Runtime, path: PathBuf) -> Result<Editor, RuntimeError> {
    let (initial, load_problem) = load(&path);

    let applied = ui.signal(initial.clone());
    let prefix = ui.signal(initial.prefix.clone());
    let format = ui.signal(initial.format);
    let quality = ui.signal(initial.quality);
    let max_width = ui.signal(initial.max_width);
    let open_folder = ui.signal(initial.open_folder);
    let saving = ui.signal(false);
    let notice = ui.signal(load_problem.unwrap_or_default());

    // The edited settings, derived from the five field bindings.
    let edited = prefix
        .read_only()
        .combine(&format.read_only(), |p, f| (p.clone(), *f))?
        .combine(&quality.read_only(), |(p, f), q| (p.clone(), *f, *q))?
        .combine(&max_width.read_only(), |(p, f, q), w| (p.clone(), *f, *q, *w))?
        .combine(&open_folder.read_only(), |(p, f, q, w), o| Settings {
            prefix: p.clone(),
            format: *f,
            quality: *q,
            max_width: *w,
            open_folder: *o,
        })?;
    let dirty = edited.combine(&applied.read_only(), |e, a| e != a)?;
    let can_act = dirty.combine(&saving.read_only(), |d, s| *d && !*s)?;
    let marker = dirty.map(|d| if *d { "● Unsaved changes".to_owned() } else { String::new() })?;

    let form = FormStatus::new(ui);
    let prefix_status = form.field("prefix")?;
    let quality_status = form.field("quality")?;
    let width_status = form.field("max-width")?;

    let report_prefix = prefix_status.clone();
    let prefix_field = TextField::new(&prefix)
        .normalize(|text| check_prefix(&text).map(|()| text).map_err(InputRejection::new))
        .on_rejection(move |_, r| report(&report_prefix, r));

    let report_quality = quality_status.clone();
    let quality_field = IntegerInput::new(&quality)
        .range(QUALITY_RANGE.0, QUALITY_RANGE.1)?
        .normalize(|v| {
            check_range(v, QUALITY_RANGE, "Quality")
                .map(|()| v)
                .map_err(InputRejection::new)
        })?
        .on_rejection(move |_, r| report(&report_quality, r));

    let report_width = width_status.clone();
    let width_field = IntegerInput::new(&max_width)
        .range(WIDTH_RANGE.0, WIDTH_RANGE.1)?
        .step(16)?
        .unit("px")
        .normalize(|v| {
            check_range(v, WIDTH_RANGE, "Maximum width")
                .map(|()| v)
                .map_err(InputRejection::new)
        })?
        .on_rejection(move |_, r| report(&report_width, r));

    let format_field = SegmentedControl::new(
        &format,
        [
            Choice::new(Format::Png, "PNG"),
            Choice::new(Format::Jpeg, "JPEG"),
            Choice::new(Format::WebP, "WebP"),
        ],
    )?;

    let output = Group::titled(
        "Output",
        Form::new([
            Field::new(prefix_field.key("prefix"))
                .label("File name prefix")
                .help("1 to 32 letters, digits, '-' or '_'")
                .form(&prefix_status)
                .key("prefix-field"),
            Field::new(format_field.key("format"))
                .label("Format")
                .key("format-field"),
        ])
        .key("fields"),
    );
    let quality_group = Group::titled(
        "Quality",
        Form::new([
            Field::new(quality_field.key("quality"))
                .label("JPEG quality")
                .help("1 to 100")
                .form(&quality_status)
                .key("quality-field"),
            Field::new(width_field.key("max-width"))
                .label("Maximum width")
                .help("16 to 16384 pixels")
                .form(&width_status)
                .key("max-width-field"),
        ])
        .key("fields"),
    );
    let behavior = Group::titled(
        "Behavior",
        Toggle::new("Open folder after export", &open_folder).key("open-folder"),
    );

    // Apply: save in the background; only a successful save becomes applied.
    let apply = {
        let (edited, applied, saving, notice, path) =
            (edited.clone(), applied.clone(), saving.clone(), notice.clone(), path.clone());
        Button::new("Apply", move |cx| {
            let settings = edited.get()?;
            saving.set(true)?;
            notice.set("Saving…".to_owned())?;
            let (applied, saving, notice, path) =
                (applied.clone(), saving.clone(), notice.clone(), path.clone());
            let target = path.clone();
            let written = settings.clone();
            let _receipt = cx.defer(move |ui| {
                ui.spawn(
                    move || save(&target, &written),
                    move |result| {
                        saving.set(false)?;
                        match result {
                            Ok(()) => {
                                applied.set(settings)?;
                                notice.set(format!("Saved to {}", path.display()))
                            }
                            Err(e) => notice.set(e),
                        }
                    },
                )
                .map(|_| ())
            })?;
            Ok(())
        })
        .enabled(can_act.clone())
    };

    let revert = {
        let (applied, prefix, format, quality, max_width, open_folder, notice) = (
            applied.clone(),
            prefix.clone(),
            format.clone(),
            quality.clone(),
            max_width.clone(),
            open_folder.clone(),
            notice.clone(),
        );
        let statuses = [prefix_status.clone(), quality_status.clone(), width_status.clone()];
        Button::new("Revert", move |_| {
            let a = applied.get()?;
            prefix.set(a.prefix)?;
            format.set(a.format)?;
            quality.set(a.quality)?;
            max_width.set(a.max_width)?;
            open_folder.set(a.open_folder)?;
            for s in &statuses {
                s.clear()?;
            }
            notice.set("Reverted to the applied settings".to_owned())
        })
        .enabled(can_act)
    };

    let content = Column::new([
        Label::new("Export settings").style(TextStyle::Heading).key("title"),
        output.key("output-group"),
        quality_group.key("quality-group"),
        behavior.key("behavior-group"),
        Row::new([
            apply.key("apply"),
            revert.key("revert"),
            Label::new(marker).color_role(ColorRole::Warning).key("dirty-marker"),
        ])
        .key("actions"),
        Label::new(notice.read_only()).key("notice"),
    ]);
    let root = ui.mount_tree(
        ScrollArea::new(content.key("content"))
            .axes(ScrollAxes::Both)
            .key("editor"),
    )?;
    Ok(Editor { root, dirty, notice })
}
