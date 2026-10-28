//! A settings editor for a fictional photo-export tool.
//!
//! [`app`] builds the whole application; `main` runs it natively and the
//! tests start it with `App::headless`.
use std::path::{Path, PathBuf};

use pert_ui::prelude::*;
use serde::{Deserialize, Serialize};

/// The export format.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Format {
    Png,
    Jpeg,
    WebP,
}

/// What the settings file holds.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
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
            format: Format::Jpeg,
            quality: 85,
            max_width: 4096,
            open_folder: false,
        }
    }
}

pub const QUALITY_RANGE: (i64, i64) = (1, 100);
pub const WIDTH_RANGE: (i64, i64) = (16, 16384);

/// Checks a file name prefix: 1 to 32 characters, letters, digits, `-` and `_`.
pub fn check_prefix(text: &str) -> Result<(), String> {
    let count = text.chars().count();
    if count == 0 {
        return Err("Enter a prefix".to_owned());
    }
    if count > 32 {
        return Err(format!("At most 32 characters ({count} given)"));
    }
    if let Some(bad) = text.chars().find(|c| !(c.is_ascii_alphanumeric() || *c == '-' || *c == '_')) {
        let bad = if bad.is_whitespace() { "A space".to_owned() } else { format!("“{bad}”") };
        return Err(format!("{bad} is not allowed: use letters, digits, - and _"));
    }
    Ok(())
}

impl Settings {
    fn validate(&self) -> Result<(), String> {
        check_prefix(&self.prefix).map_err(|e| format!("prefix: {e}"))?;
        if !(QUALITY_RANGE.0..=QUALITY_RANGE.1).contains(&self.quality) {
            return Err(format!("quality {} is out of range", self.quality));
        }
        if !(WIDTH_RANGE.0..=WIDTH_RANGE.1).contains(&self.max_width) {
            return Err(format!("maximum width {} is out of range", self.max_width));
        }
        Ok(())
    }
}

/// Reads the settings file: `Ok(None)` when it does not exist.
pub fn load(path: &Path) -> Result<Option<Settings>, String> {
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(format!("Could not read {}: {e}", path.display())),
    };
    let settings: Settings = serde_json::from_str(&text)
        .map_err(|e| format!("Could not parse {}: {e}", path.display()))?;
    settings.validate().map_err(|e| format!("Invalid {}: {e}", path.display()))?;
    Ok(Some(settings))
}

/// Writes the settings file; runs on a worker thread.
pub fn save(path: &Path, settings: &Settings) -> Result<(), String> {
    let text = serde_json::to_string_pretty(settings).map_err(|e| e.to_string())?;
    std::fs::write(path, text).map_err(|e| format!("Could not save {}: {e}", path.display()))
}

/// What the initializer builds; the close policy and the tests read it.
pub struct Editor {
    pub path: PathBuf,
    pub prefix: Signal<String>,
    pub format: Signal<Format>,
    pub quality: Signal<i64>,
    pub max_width: Signal<i64>,
    pub open_folder: Signal<bool>,
    pub applied: Signal<Settings>,
    pub form: FormFields,
    pub prefix_field: FieldHandle,
    pub quality_field: FieldHandle,
    pub width_field: FieldHandle,
    /// The edited values differ from the applied ones.
    pub changed: ReadSignal<bool>,
    /// `changed`, or a field keeps a draft that is not saved.
    pub unsaved: ReadSignal<bool>,
    /// A save is running.
    pub saving: Signal<bool>,
    /// The application's message line: save results, load errors, close refusal.
    pub message: Signal<String>,
    pub root: WidgetId,
}

impl Editor {
    /// Whether closing must be refused.
    pub fn has_unsaved(&self) -> Result<bool, RuntimeError> {
        Ok(self.unsaved.get()? || self.form.summary().get()?.modified() > 0)
    }
}

/// Reports an editor's refusal on its field and its kept draft to the form.
macro_rules! report_to {
    ($input:expr, $field:expr) => {{
        let (status, mark) = ($field.clone(), $field.clone());
        $input
            .on_rejection(move |_, refusal| match refusal {
                Some(refusal) => status.set(FieldStatus::error(refusal.to_string())),
                None => status.clear().map(|_| ()),
            })
            .on_modified(move |_, modified| mark.set_modified(modified).map(|_| ()))
    }};
}

fn build(ui: &mut Runtime, path: PathBuf) -> Result<Editor, RuntimeError> {
    let (saved, message) = match load(&path) {
        Ok(Some(settings)) => (settings, format!("Loaded {}", path.display())),
        Ok(None) => (Settings::default(), "No settings file yet: defaults shown".to_owned()),
        Err(e) => (Settings::default(), format!("{e}; defaults shown")),
    };
    let prefix = ui.signal(saved.prefix.clone());
    let format = ui.signal(saved.format);
    let quality = ui.signal(saved.quality);
    let max_width = ui.signal(saved.max_width);
    let open_folder = ui.signal(saved.open_folder);
    let applied = ui.signal(saved);
    let saving = ui.signal(false);
    let message = ui.signal(message);

    let form = FormFields::new(ui);
    let prefix_field = form.field("prefix")?;
    let quality_field = form.field("quality")?;
    let width_field = form.field("max-width")?;

    let current = ReadSignal::combine(
        (&prefix, &format, &quality, &max_width, &open_folder),
        |(prefix, format, quality, max_width, open_folder)| Settings {
            prefix: prefix.clone(),
            format: *format,
            quality: *quality,
            max_width: *max_width,
            open_folder: *open_folder,
        },
    )?;
    let changed = ReadSignal::combine((&current, &applied), |(current, applied)| current != applied)?;
    let unsaved = ReadSignal::combine((&changed, &form.summary()), |(changed, summary)| {
        *changed || summary.modified() > 0
    })?;
    let can_apply = ReadSignal::combine((&changed, &saving), |(changed, saving)| *changed && !*saving)?;
    let can_revert = ReadSignal::combine((&unsaved, &saving), |(unsaved, saving)| *unsaved && !*saving)?;
    let marker = ReadSignal::combine((&unsaved, &saving), |(unsaved, saving)| {
        match (*saving, *unsaved) {
            (true, _) => "Saving…",
            (false, true) => "● Unsaved changes",
            (false, false) => "All changes applied",
        }
        .to_owned()
    })?;

    // Output.
    let prefix_input = report_to!(
        TextField::new(&prefix).normalize(|text| match check_prefix(&text) {
            Ok(()) => Ok(text),
            Err(e) => Err(InputRejection::new(e)),
        }),
        prefix_field
    );
    let format_input = Select::new(
        ui,
        &format,
        [
            Choice::new(Format::Png, "PNG"),
            Choice::new(Format::Jpeg, "JPEG"),
            Choice::new(Format::WebP, "WebP"),
        ],
    )?;
    let output = Group::titled(
        "Output",
        Column::new([
            Field::new(prefix_input.key("prefix"))
                .label("File name prefix")
                .help("1 to 32 letters, digits, - or _")
                .form(&prefix_field)
                .key("prefix-field"),
            Field::new(format_input.key("format")).label("Format").key("format-field"),
        ])
        .key("fields"),
    );

    // Quality.
    let quality_input = report_to!(
        IntegerInput::new(&quality).range(QUALITY_RANGE.0, QUALITY_RANGE.1)?,
        quality_field
    );
    let width_input = report_to!(
        IntegerInput::new(&max_width).range(WIDTH_RANGE.0, WIDTH_RANGE.1)?.unit("px"),
        width_field
    );
    let quality_group = Group::titled(
        "Quality",
        Column::new([
            Field::new(quality_input.key("quality"))
                .label("JPEG quality")
                .help("1 to 100")
                .form(&quality_field)
                .key("quality-field"),
            Field::new(width_input.key("max-width"))
                .label("Maximum width")
                .help("16 to 16384 pixels")
                .form(&width_field)
                .key("max-width-field"),
        ])
        .key("fields"),
    );

    // Behavior.
    let behavior = Group::titled(
        "Behavior",
        Column::new([Toggle::new(&open_folder, "Open folder after export").key("open-folder")])
            .key("fields"),
    );

    // Apply saves on a worker and records the values as applied when it succeeds.
    let apply = {
        let (current, applied, saving, message, path) =
            (current.clone(), applied.clone(), saving.clone(), message.clone(), path.clone());
        Button::new("Apply", move |cx| {
            let values = current.get()?;
            let path = path.clone();
            let (applied, saving, message) = (applied.clone(), saving.clone(), message.clone());
            saving.set(true)?;
            message.set("Saving…".to_owned())?;
            cx.spawn(
                move || save(&path, &values).map(|()| values),
                move |result| {
                    saving.set(false)?;
                    match result {
                        Ok(values) => {
                            applied.set(values)?;
                            message.set("Settings saved".to_owned())
                        }
                        Err(e) => message.set(format!("Save failed: {e}")),
                    }
                },
            )?;
            Ok(())
        })
        .enabled(&can_apply)
    };

    // Revert writes the applied values back, clears statuses and discards drafts.
    let revert = {
        let (prefix, format, quality, max_width, open_folder) =
            (prefix.clone(), format.clone(), quality.clone(), max_width.clone(), open_folder.clone());
        let (applied, message, form) = (applied.clone(), message.clone(), form.clone());
        let handles = [prefix_field.clone(), quality_field.clone(), width_field.clone()];
        Button::new("Revert", move |cx| {
            let saved = applied.get()?;
            prefix.set(saved.prefix)?;
            format.set(saved.format)?;
            quality.set(saved.quality)?;
            max_width.set(saved.max_width)?;
            open_folder.set(saved.open_folder)?;
            for handle in &handles {
                handle.clear()?;
            }
            form.discard_drafts(cx)?;
            message.set("Changes reverted".to_owned())?;
            Ok(())
        })
        .enabled(&can_revert)
    };

    let content = Column::new([
        Label::new("Export settings").style(TextStyle::Heading).key("title"),
        output.key("output"),
        quality_group.key("quality"),
        behavior.key("behavior"),
        Row::new([
            apply.key("apply"),
            revert.key("revert"),
            Label::new(marker).key("dirty"),
        ])
        .key("buttons"),
        Label::new(message.read_only()).key("message"),
    ])
    .align(Align::Stretch);
    let root = ui.mount_tree(
        ScrollArea::new(content.key("content")).key("settings"),
    )?;
    Ok(Editor {
        path,
        prefix,
        format,
        quality,
        max_width,
        open_folder,
        applied,
        form,
        prefix_field,
        quality_field,
        width_field,
        changed,
        unsaved,
        saving,
        message,
        root,
    })
}

/// The application, with its settings file at `path`.
pub fn app(path: PathBuf) -> App<Editor> {
    App::new("Export settings", move |ui| Ok(build(ui, path)?)).on_close(|_, editor, _| {
        if editor.has_unsaved()? {
            editor
                .message
                .set("Unsaved changes: apply or revert them before closing".to_owned())?;
            Ok(NativeCloseAction::KeepOpen)
        } else {
            Ok(NativeCloseAction::Close)
        }
    })
}
