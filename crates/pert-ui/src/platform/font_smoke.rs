//! Scripted font and icon overrides presented by the real native renderer.
//!
//! Each override is checked against captures of the smoke's content rendered
//! by the same renderer before and after it, so an override that silently
//! falls back to the previous font or glyph fails instead of passing on a
//! merely presented frame.
use crate::*;

type Reply = Pending<Result<Capture, CaptureError>>;

pub(super) struct FontSmoke {
    root: WidgetId,
    font: FontId,
    glyph: IconGlyph,
    /// The capture asked for after the latest change, not yet answered.
    pending: Option<Reply>,
    /// The latest answered capture, which the next one must differ from.
    previous: Option<FrameImage>,
}
impl FontSmoke {
    pub(super) fn new(ui: &mut Runtime) -> Result<Self, AppError> {
        let font = ui
            .fonts()
            .register(FontData::bundled(FontRole::Monospace))?;
        let glyph = ui.fonts().material_icon("expand_circle_down")?;
        let root = ui.mount_tree(Element::new(
            "font-smoke",
            Column::new([
                Element::new(
                    "rich",
                    Label::new(
                        RichText::new()
                            .icon(Icon::Settings)
                            .gap()
                            .text("Settings ")
                            .span(
                                TextSpan::new("42.00")
                                    .font(font.clone())
                                    .color_role(ColorRole::Warning),
                            ),
                    ),
                ),
                Element::new(
                    "action",
                    Button::new("Save", |_| Ok(())).leading_icon(Icon::Save),
                ),
                Element::new(
                    "select",
                    Select::new(
                        ui,
                        ui.signal(0),
                        [
                            Choice::new(0, "First").leading_icon(Icon::Info),
                            Choice::new(1, "Second"),
                        ],
                    )?,
                ),
                Element::new("checkbox", Checkbox::new("Enabled", ui.signal(true))),
            ]),
        ))?;
        Ok(Self {
            root,
            font,
            glyph,
            pending: None,
            previous: None,
        })
    }
    pub(super) fn unmount(self, ui: &mut Runtime) -> Result<(), RuntimeError> {
        ui.unmount(&self.root)
    }
    fn request(&mut self, ui: &Runtime) {
        self.pending = Some(
            ui.request_capture(CaptureRequest::new(CaptureTarget::Path(vec![
                "font-smoke".into(),
            ]))),
        );
    }
    /// The requested capture once it has arrived, or `None` while it waits.
    fn answered(&mut self, stage: u8) -> Result<Option<FrameImage>, AppError> {
        let Some(pending) = &self.pending else {
            return Err(AppError::Graphics(format!(
                "font smoke stage {stage}: no capture was requested"
            )));
        };
        let reply = match pending.try_reply() {
            Err(WaitError::Timeout) => return Ok(None),
            reply => reply,
        };
        self.pending = None;
        let failed = |error: &dyn std::fmt::Display| {
            AppError::Graphics(format!("font smoke stage {stage} capture: {error}"))
        };
        Ok(Some(
            reply
                .map_err(|error| failed(&error))?
                .value
                .map_err(|error| failed(&error))?
                .image,
        ))
    }
    /// Runs one stage; `false` means it waits for a capture and runs again.
    ///
    /// Stage 35 captures the content before any override, then sets the body
    /// font. Stages 36 and 37 each require the capture asked for after the
    /// previous change to differ from the one before it, then change the
    /// chevron glyph or reset both overrides.
    pub(super) fn step(&mut self, ui: &Runtime, stage: u8) -> Result<bool, AppError> {
        if stage == 35 {
            if self.previous.is_none() {
                if self.pending.is_none() {
                    self.request(ui);
                }
                let Some(image) = self.answered(stage)? else {
                    return Ok(false);
                };
                self.previous = Some(image);
            }
            ui.fonts().set_default(FontRole::Body, self.font.clone())?;
            self.request(ui);
            return Ok(true);
        }
        let Some(image) = self.answered(stage)? else {
            return Ok(false);
        };
        if self.previous.as_ref() == Some(&image) {
            return Err(AppError::Graphics(format!(
                "font smoke stage {stage}: the override did not change the rendered pixels"
            )));
        }
        self.previous = Some(image);
        if stage == 36 {
            ui.icons().set(Icon::ChevronDown, self.glyph.clone())?;
            self.request(ui);
        } else {
            ui.fonts().reset_default(FontRole::Body)?;
            ui.icons().reset(Icon::ChevronDown)?;
        }
        Ok(true)
    }
}
