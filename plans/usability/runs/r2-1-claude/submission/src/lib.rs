//! A settings editor for a fictional photo-export tool, built on pert-ui.

use pert_ui::prelude::*;
use serde::{Deserialize, Serialize};
use std::{cell::RefCell, fs, io, path::PathBuf, rc::Rc};

pub const PREFIX_MAX: usize = 32;
pub const QUALITY_RANGE: (i64, i64) = (1, 100);
pub const WIDTH_RANGE: (i64, i64) = (16, 16384);

pub const CLOSE_REFUSED_MESSAGE: &str =
    "There are unsaved changes. Apply or revert them before closing.";

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
    pub jpeg_quality: i64,
    pub max_width: i64,
    pub open_folder: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            prefix: "export".to_owned(),
            format: Format::Jpeg,
            jpeg_quality: 85,
            max_width: 2048,
            open_folder: false,
        }
    }
}

/// Checks a file name prefix, returning the message to show when it is refused.
pub fn check_prefix(prefix: &str) -> Result<(), String> {
    let count = prefix.chars().count();
    if count == 0 {
        return Err("Enter a prefix of at least 1 character.".to_owned());
    }
    if count > PREFIX_MAX {
        return Err(format!("Use at most {PREFIX_MAX} characters (this has {count})."));
    }
    if let Some(bad) = prefix
        .chars()
        .find(|c| !(c.is_ascii_alphanumeric() || *c == '-' || *c == '_'))
    {
        let shown = if bad == ' ' { "A space".to_owned() } else { format!("“{bad}”") };
        return Err(format!("{shown} is not allowed: use letters, digits, - and _ only."));
    }
    Ok(())
}

impl Settings {
    /// Checks every field, so a hand-edited file cannot bring invalid values in.
    pub fn validate(&self) -> Result<(), String> {
        check_prefix(&self.prefix).map_err(|e| format!("prefix: {e}"))?;
        let in_range = |v: i64, (lo, hi): (i64, i64)| (lo..=hi).contains(&v);
        if !in_range(self.jpeg_quality, QUALITY_RANGE) {
            return Err(format!("JPEG quality {} is outside 1–100", self.jpeg_quality));
        }
        if !in_range(self.max_width, WIDTH_RANGE) {
            return Err(format!("maximum width {} is outside 16–16384", self.max_width));
        }
        Ok(())
    }
}

/// What loading the settings file found.
pub enum Loaded {
    Missing,
    Read(Settings),
    Unusable(String),
}

pub fn load(path: &std::path::Path) -> Loaded {
    match fs::read_to_string(path) {
        Err(e) if e.kind() == io::ErrorKind::NotFound => Loaded::Missing,
        Err(e) => Loaded::Unusable(format!("Could not read {}: {e}", path.display())),
        Ok(text) => match serde_json::from_str::<Settings>(&text) {
            Err(e) => Loaded::Unusable(format!("Could not parse {}: {e}", path.display())),
            Ok(settings) => match settings.validate() {
                Ok(()) => Loaded::Read(settings),
                Err(e) => Loaded::Unusable(format!("Ignored {}: {e}", path.display())),
            },
        },
    }
}

/// Writes the settings; runs on a background thread.
pub fn save(path: &std::path::Path, settings: &Settings) -> Result<(), String> {
    let text = serde_json::to_string_pretty(settings).map_err(|e| e.to_string())?;
    let mut temp = path.as_os_str().to_owned();
    temp.push(".tmp");
    let temp = PathBuf::from(temp);
    fs::write(&temp, text)
        .and_then(|()| fs::rename(&temp, path))
        .map_err(|e| format!("Could not save {}: {e}", path.display()))
}

/// What the initializer built, for the close policy and for tests.
#[derive(Clone)]
pub struct Built {
    pub root: WidgetId,
    dirty: ReadSignal<bool>,
    close_refused: Signal<bool>,
}

pub type BuiltSlot = Rc<RefCell<Option<Built>>>;

/// The application, saving to and loading from `path`. The slot receives what
/// the initializer built.
pub fn settings_app(path: PathBuf) -> Result<(App, BuiltSlot), RuntimeError> {
    let built: BuiltSlot = Rc::default();
    let (slot, policy) = (built.clone(), built.clone());
    let app = App::new("Photo export settings", move |ui| {
        build(ui, path, &slot)?;
        Ok(())
    })
    .size(Size::new(560.0, 760.0)?)?
    .on_close(move |_, _| {
        let Some(built) = policy.borrow().clone() else {
            return Ok(NativeCloseAction::Close);
        };
        if built.dirty.get()? {
            built.close_refused.set(true)?;
            Ok(NativeCloseAction::KeepOpen)
        } else {
            Ok(NativeCloseAction::Close)
        }
    });
    Ok((app, built))
}

fn status_from(rejection: Option<&InputRejection>) -> FieldStatus {
    rejection.map_or(FieldStatus::Normal, |r| FieldStatus::error(r.message()))
}

fn build(ui: &mut Runtime, path: PathBuf, slot: &BuiltSlot) -> Result<WidgetId, AppError> {
    let (initial, load_message) = match load(&path) {
        Loaded::Missing => (Settings::default(), format!("No settings file yet; {} will be created on Apply.", path.display())),
        Loaded::Read(s) => (s, format!("Loaded {}.", path.display())),
        Loaded::Unusable(message) => (Settings::default(), format!("{message}. Using defaults.")),
    };

    // The applied settings, and one signal per edited field.
    let applied = ui.signal(initial.clone());
    let prefix = ui.signal(initial.prefix.clone());
    let format = ui.signal(initial.format);
    let quality = ui.signal(initial.jpeg_quality);
    let width = ui.signal(initial.max_width);
    let open_folder = ui.signal(initial.open_folder);

    let edited = prefix
        .read_only()
        .combine(&format.read_only(), |p, f| (p.clone(), *f))?
        .combine(&quality.read_only(), |(p, f), q| (p.clone(), *f, *q))?
        .combine(&width.read_only(), |(p, f, q), w| (p.clone(), *f, *q, *w))?
        .combine(&open_folder.read_only(), |(p, f, q, w), o| Settings {
            prefix: p.clone(),
            format: *f,
            jpeg_quality: *q,
            max_width: *w,
            open_folder: *o,
        })?;
    let dirty = edited.combine(&applied.read_only(), |e, a| e != a)?;
    let saving = ui.signal(false);
    let can_act = dirty.combine(&saving.read_only(), |d, s| *d && !*s)?;
    let status = ui.signal(load_message);
    let close_refused = ui.signal(false);

    let marker = dirty.map(|d| if *d { "● Unsaved changes".to_owned() } else { "No unsaved changes".to_owned() })?;
    let close_notice = close_refused.read_only().combine(&dirty, |refused, dirty| {
        if *refused && *dirty { CLOSE_REFUSED_MESSAGE.to_owned() } else { String::new() }
    })?;

    // Field statuses.
    let form = FormFields::new(ui);
    let prefix_status = form.field("prefix")?;
    let quality_status = form.field("jpeg-quality")?;
    let width_status = form.field("max-width")?;

    let report = prefix_status.clone();
    let prefix_input = TextField::new(prefix.clone())
        .normalize(|text| check_prefix(&text).map(|()| text).map_err(InputRejection::new))
        .on_rejection(move |_, r| report.set(status_from(r)))
        .max_length(Some(PREFIX_MAX))
        .counter(true);
    let report = quality_status.clone();
    let quality_input = IntegerInput::new(quality.clone())
        .range(QUALITY_RANGE.0, QUALITY_RANGE.1)?
        .on_rejection(move |_, r| report.set(status_from(r)));
    let report = width_status.clone();
    let width_input = IntegerInput::new(width.clone())
        .range(WIDTH_RANGE.0, WIDTH_RANGE.1)?
        .unit("px")
        .on_rejection(move |_, r| report.set(status_from(r)));
    let format_input = SegmentedControl::new(format.clone(), [
        Choice::new(Format::Png, "PNG"),
        Choice::new(Format::Jpeg, "JPEG"),
        Choice::new(Format::WebP, "WebP"),
    ])?;

    let output = Group::titled("Output", Column::new([
        Field::new(prefix_input.key("prefix"))
            .label("File name prefix")
            .help("1 to 32 letters, digits, - or _")
            .form(&prefix_status)
            .key("prefix-field"),
        Field::new(format_input.key("format"))
            .label("Format")
            .key("format-field"),
    ]).key("output-fields"));
    let quality_group = Group::titled("Quality", Column::new([
        Field::new(quality_input.key("jpeg-quality"))
            .label("JPEG quality")
            .help("1 to 100")
            .form(&quality_status)
            .key("jpeg-quality-field"),
        Field::new(width_input.key("max-width"))
            .label("Maximum width")
            .help("16 to 16384 pixels")
            .form(&width_status)
            .key("max-width-field"),
    ]).key("quality-fields"));
    let behavior = Group::titled("Behavior", Column::new([
        Toggle::new(open_folder.clone(), "Open folder after export").key("open-folder"),
    ]).key("behavior-fields"));

    let apply = {
        let (status, saving, applied, path) = (status.clone(), saving.clone(), applied.clone(), path.clone());
        let edited = edited.clone();
        Button::new("Apply", move |cx| {
            let settings = edited.get()?;
            saving.set(true)?;
            status.set("Saving…".to_owned())?;
            let (status, saving, applied, path) = (status.clone(), saving.clone(), applied.clone(), path.clone());
            let written = settings.clone();
            let _receipt = cx.spawn(
                move || save(&path, &written).map(|()| path),
                move |result| {
                    saving.set(false)?;
                    match result {
                        Ok(path) => {
                            applied.set(settings)?;
                            status.set(format!("Saved to {}.", path.display()))
                        }
                        Err(message) => status.set(format!("Save failed: {message}")),
                    }
                },
            )?;
            Ok(())
        })
        .enabled(can_act.clone())
    };
    let revert = {
        let (applied, prefix, format, quality, width, open_folder) =
            (applied.clone(), prefix.clone(), format.clone(), quality.clone(), width.clone(), open_folder.clone());
        let statuses = [prefix_status.clone(), quality_status.clone(), width_status.clone()];
        let status = status.clone();
        Button::new("Revert", move |_| {
            let a = applied.get()?;
            prefix.set(a.prefix)?;
            format.set(a.format)?;
            quality.set(a.jpeg_quality)?;
            width.set(a.max_width)?;
            open_folder.set(a.open_folder)?;
            for s in &statuses {
                s.clear()?;
            }
            status.set("Reverted to the applied settings.".to_owned())
        })
        .enabled(can_act)
    };

    let content = Column::new([
        Label::new("Photo export settings").key("title"),
        Label::new(status.read_only()).key("status"),
        output.key("output"),
        quality_group.key("quality"),
        behavior.key("behavior"),
        Row::new([
            apply.key("apply"),
            revert.key("revert"),
            Label::new(marker).key("dirty-marker"),
        ]).key("actions"),
        Label::new(close_notice).key("close-notice"),
    ]);
    let root = ui.mount_tree(
        ScrollArea::new(content.key("content"))
            .axes(ScrollAxes::Both)
            .key("settings"),
    )?;
    *slot.borrow_mut() = Some(Built { root: root.clone(), dirty, close_refused });
    Ok(root)
}
