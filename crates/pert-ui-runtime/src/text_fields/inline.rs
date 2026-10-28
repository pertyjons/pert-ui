//! Shared inline numeric presentation and an embedded, retained text editor.
use super::draft::DraftSlot;
use super::*;
use crate::choice_controls::field_frame;
use crate::layout::rect;

#[derive(Clone, Copy)]
struct Press {
    start: Point,
    last: Point,
    dragged: bool,
}
pub(crate) struct InlineField<T> {
    /// The resting draft store, and for a plain field its framed editor.
    pub(super) editor: Editor<Option<T>>,
    /// An inline row's text mode: a child `DraftEditor`, mounted through the
    /// public contract. Absent for a plain field.
    draft_slot: Option<DraftSlot<T>>,
    pub(crate) inline: bool,
    pub(crate) editing: bool,
    /// The caption before the value, from the owner's `label` builder.
    pub(crate) caption: crate::Caption,
    pub(crate) unit: String,
    pub(crate) format: Option<NumberFormat<T>>,
    pub(crate) display: String,
    pub(crate) suffix: String,
    pub(crate) focused: bool,
    pub(crate) error: Option<EntryError>,
    slot: Cell<Option<Rect>>,
    label_area: Cell<Option<Rect>>,
    baseline: Cell<f32>,
    pub(crate) track: Cell<Option<Rect>>,
    press: Option<Press>,
    pub(crate) stepper: bool,
    buttons: Cell<Option<(Rect, Rect)>>,
    pub(crate) pressed_button: Option<bool>,
    hovered: bool,
    hovered_button: Option<bool>,
}
impl<T: Copy + PartialEq + 'static> InlineField<T> {
    pub(crate) fn set_hover(&mut self, cx: &mut EventContext, hovered: bool) {
        let hovered = hovered && self.editor.enabled;
        if self.hovered != hovered {
            self.hovered = hovered;
            cx.invalidate(Invalidation::Paint);
        }
        if !hovered {
            self.set_button_hover(cx, None);
        }
    }
    pub(crate) fn set_button_hover(&mut self, cx: &mut EventContext, button: Option<bool>) {
        if self.hovered_button != button {
            self.hovered_button = button;
            cx.invalidate(Invalidation::Paint);
        }
    }
    /// Creates the shared field. An inline field embeds its editor in the
    /// value slot and rests without an active caret; a plain field keeps the
    /// ordinary framed editor.
    pub(crate) fn new(inline: bool) -> Self {
        let mut editor = Editor::new(None, false);
        if inline {
            editor.embedded = true;
            editor.active = false;
            editor.style = TextStyle::Monospace;
        }
        Self {
            editor,
            draft_slot: inline.then(|| DraftSlot::new(TextStyle::Monospace, DraftAlign::End)),
            inline,
            editing: false,
            caption: crate::Caption::new(""),
            unit: String::new(),
            format: None,
            display: String::new(),
            suffix: String::new(),
            focused: false,
            error: None,
            slot: Cell::new(None),
            label_area: Cell::new(None),
            baseline: Cell::new(0.0),
            track: Cell::new(None),
            press: None,
            stepper: false,
            buttons: Cell::new(None),
            pressed_button: None,
            hovered: false,
            hovered_button: None,
        }
    }
    /// Trailing space reserved for the step buttons, including their gap.
    fn stepper_width(&self, cx: &MeasureContext<'_>) -> f32 {
        if self.stepper {
            2.0 * cx.metric(MetricRole::IconSize) + cx.metric(MetricRole::IconGap)
        } else {
            0.0
        }
    }
    /// Makes text mode read-only: selection and copy stay available.
    pub(crate) fn set_read_only(&mut self) {
        self.editor.read_only = true;
    }
    /// Makes an open text mode read-only, or editable again, without ending
    /// it, as its owner's effective editability changes: the draft,
    /// selection and undo history stay. Without text mode it does nothing.
    pub(crate) fn set_text_read_only(&self, read_only: bool) {
        if let Some(slot) = &self.draft_slot {
            slot.draft.set_read_only(read_only);
        }
    }
    /// Overrides the value typography.
    pub(crate) fn set_style(&mut self, style: TextStyle) {
        self.editor.style = style;
        if let Some(slot) = &mut self.draft_slot {
            slot.set_style(style);
        }
    }
    /// Whether text mode runs in the child editor right now.
    pub(crate) fn drafting(&self) -> bool {
        self.editing && self.draft_slot.is_some()
    }
    /// The exact value the current draft stands for, if it still does.
    pub(crate) fn payload(&self) -> Option<T> {
        match &self.draft_slot {
            Some(slot) if self.editing => slot.draft.payload(),
            _ => self.editor.payload,
        }
    }
    /// The current draft text: the child editor's in text mode, otherwise the
    /// resting store's.
    pub(crate) fn draft_text(&self) -> String {
        match &self.draft_slot {
            Some(slot) if self.editing => slot.draft.text(),
            _ => self.editor.text.clone(),
        }
    }
    /// Whether the child editor, mounted under `key`, is shown.
    pub(crate) fn child_visible(&self, key: &str) -> bool {
        self.draft_slot
            .as_ref()
            .is_none_or(|slot| slot.child_visible(key))
    }
    /// Places the child editor in the value slot the last arrangement chose.
    pub(crate) fn place(&self, cx: &mut ArrangeContext<'_>) -> Result<(), RuntimeError> {
        match (&self.draft_slot, self.editor.area.get()) {
            (Some(slot), Some(area)) => slot.arrange(cx, area),
            _ => Ok(()),
        }
    }
    /// Which step button contains a window-coordinate point: `Some(true)` for
    /// plus, `Some(false)` for minus.
    pub(crate) fn hit_button(&self, cx: &EventContext, point: Point) -> Option<bool> {
        let (minus, plus) = self.buttons.get()?;
        let local = cx.to_local(point).unwrap_or(point);
        if plus.contains(local) {
            Some(true)
        } else if minus.contains(local) {
            Some(false)
        } else {
            None
        }
    }
    /// Checks a text and unit the field would show without a format, without
    /// changing anything: the check [`Self::display_value`] applies. It is
    /// pure, so an owner can refuse text before it commits it, and a
    /// reconcile can refuse fixed text at adoption.
    pub(crate) fn check_display(&self, display: &str, unit: &str) -> Result<(), TextEditError> {
        check_text(display, self.editor.multi)?;
        check_text(unit, self.editor.multi)
    }
    pub(crate) fn display_value(&mut self, value: T, fallback: String) -> Result<(), RuntimeError> {
        let display = self
            .format
            .as_ref()
            .map_or(fallback, |f| (f.display)(value));
        let suffix = self
            .format
            .as_ref()
            .and_then(|f| f.unit.as_ref())
            .map_or_else(|| self.unit.clone(), |f| f(value));
        self.editor.validate(&display)?;
        self.editor.validate(&suffix)?;
        if let Some(unit) = self.format.as_ref().and_then(|f| f.edit_unit.as_ref()) {
            self.editor.validate(unit)?;
        }
        self.display = display;
        self.suffix = suffix;
        Ok(())
    }
    fn unit_text(&self) -> &str {
        if self.editing
            && let Some(unit) = self.format.as_ref().and_then(|f| f.edit_unit.as_deref())
        {
            return unit;
        }
        &self.suffix
    }
    pub(crate) fn editing_text(&self, value: T, fallback: String) -> String {
        self.format.as_ref().map_or(fallback, |f| (f.edit)(value))
    }
    pub(crate) fn parse(
        &self,
        fallback: impl FnOnce(&str) -> Result<T, NumberInputError>,
    ) -> Result<T, NumberInputError> {
        if let Some(value) = self.payload() {
            return Ok(value);
        }
        let text = self.draft_text();
        self.format
            .as_ref()
            .map_or_else(|| fallback(&text), |f| (f.parse)(&text))
    }
    pub(crate) fn begin_text(
        &mut self,
        cx: &mut EventContext,
        value: T,
        fallback: String,
    ) -> Result<(), RuntimeError> {
        let text = self.editing_text(value, fallback);
        self.editor.validate(&text)?;
        let layout = cx.text(&text, None, self.editor.style)?;
        let edit_unit = self
            .format
            .as_ref()
            .and_then(|f| f.edit_unit.as_deref())
            .unwrap_or(&self.suffix);
        self.editor.validate(edit_unit)?;
        cx.text(edit_unit, None, self.editor.style)?;
        if let Some(slot) = &self.draft_slot {
            slot.begin(cx, &text, Some(value), self.editor.read_only)?;
            self.editing = true;
            self.error = None;
            self.press = None;
            return Ok(());
        }
        self.editor.sync_text(text);
        self.editor.payload = Some(value);
        self.editor
            .selection
            .set(TextSelection::snapped(&layout, 0, self.editor.text.len()));
        self.editor.active = true;
        self.editor.focused = cx.focused();
        self.editing = true;
        self.error = None;
        self.press = None;
        self.editor.changed(cx);
        Ok(())
    }
    pub(crate) fn stop_text(&mut self) {
        if let Some(slot) = &self.draft_slot {
            slot.end(None);
        }
        self.editing = false;
        self.editor.active = !self.inline;
        self.editor.reset();
        self.press = None;
        self.error = None;
    }
    /// The caption's plain text, empty without one.
    pub(crate) fn label(&self) -> &str {
        self.caption.text()
    }
    pub(crate) fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        // A bound caption is read here and subscribes the owner to it.
        if self.caption.update(cx)? {
            cx.request_layout();
        }
        if let Some(slot) = &mut self.draft_slot {
            slot.update(cx);
            // The session can end inside the editor, such as on unmount.
            if self.editing && !slot.is_active() {
                self.editing = false;
            }
            self.focused = cx.focus_within();
            return Ok(());
        }
        self.focused = cx.focused();
        self.editor.update(cx)
    }
    pub(crate) fn enabled(&mut self, enabled: bool) {
        self.editor.enabled = enabled;
    }
    pub(crate) fn input_state(&self) -> Option<TextInputState> {
        if self.draft_slot.is_some() {
            return None;
        }
        self.editor.input_state()
    }
    pub(crate) fn text(&mut self, cx: &mut EventContext, text: &str) -> Result<(), RuntimeError> {
        if self.editing && self.draft_slot.is_none() {
            self.editor.insert(cx, text)?;
        }
        Ok(())
    }
    pub(crate) fn preedit(
        &mut self,
        cx: &mut EventContext,
        text: &TextPreedit,
    ) -> Result<(), RuntimeError> {
        if self.editing && self.draft_slot.is_none() {
            self.editor.compose(cx, text)?;
        }
        Ok(())
    }
    pub(crate) fn can_step_text(&self) -> bool {
        match &self.draft_slot {
            // A wheel step must not replace a composition or a selection drag.
            Some(slot) => !slot.draft.busy(),
            None => self.editor.preedit.is_none() && !self.editor.dragging,
        }
    }
    pub(crate) fn replace_value(
        &mut self,
        cx: &mut EventContext,
        value: T,
        fallback: String,
    ) -> Result<(), RuntimeError> {
        if let Some(slot) = self.draft_slot.as_ref().filter(|_| self.editing) {
            let text = self.editing_text(value, fallback);
            return slot.draft.replace(cx, &text, Some(value)).map(|_| ());
        }
        self.editor
            .replace_with_payload(
                cx,
                0..self.editor.text.len(),
                &self.editing_text(value, fallback),
                true,
                Some(value),
            )
            .map(|_| ())
    }
    pub(crate) fn prepare_display(
        &mut self,
        cx: &EventContext,
        value: T,
        fallback: String,
    ) -> Result<(), RuntimeError> {
        self.display_value(value, fallback)?;
        cx.text(&self.display, None, self.editor.style)?;
        cx.text(&self.suffix, None, self.editor.style)?;
        Ok(())
    }
    pub(crate) fn composing(&self) -> bool {
        self.draft_slot.is_none() && self.editor.preedit.is_some()
    }
    pub(crate) fn key(
        &mut self,
        cx: &mut EventContext,
        e: KeyEvent,
    ) -> Result<InputOutcome, RuntimeError> {
        if self.draft_slot.is_some() {
            return Ok(InputOutcome::Ignored);
        }
        self.editor.key(cx, e)
    }
    /// A clipboard command on the text-mode draft; outside text mode the field
    /// is not an editor.
    pub(crate) fn command(
        &mut self,
        cx: &mut EventContext,
        command: TextCommand,
    ) -> Result<TextCommandOutcome, RuntimeError> {
        // The child editor answers for itself; the runtime passes a command
        // sent to the row on to it while it holds focus.
        if !self.editing || self.draft_slot.is_some() {
            return Ok(TextCommandOutcome::NotAnEditor);
        }
        self.editor.command(cx, command)
    }
    /// Answers a text-phase key while in text mode, as the standard editors
    /// do: `handled` is whether the row consumed the key, and an unconsumed
    /// character key without a command modifier is left for text insertion.
    /// Outside text mode the field is not an editor.
    pub(crate) fn text_key(
        &self,
        cx: &EventContext,
        input: CommandKeyInput,
        transition: KeyTransition,
        handled: bool,
    ) -> TextKeyOutcome {
        if handled {
            return TextKeyOutcome::Handled;
        }
        unhandled_text_key(cx, input, transition)
    }
    pub(crate) fn text_pointer(
        &mut self,
        cx: &mut EventContext,
        e: PointerEvent,
    ) -> Result<InputOutcome, RuntimeError> {
        if self.draft_slot.is_some() {
            // The child editor takes presses on the value itself; a press on
            // the caption, unit or padding stops at the row while it types.
            return Ok(if matches!(e.kind(), PointerKind::Scroll(_)) {
                InputOutcome::Ignored
            } else {
                InputOutcome::Handled
            });
        }
        self.editor.pointer(cx, e)
    }
    pub(crate) fn hint(&self, fallback: &str) -> String {
        self.format.as_ref().map_or_else(
            || {
                if self.unit.is_empty() {
                    fallback.into()
                } else {
                    format!("{fallback} {}", self.unit)
                }
            },
            |f| f.hint.clone(),
        )
    }
    pub(crate) fn measure(
        &self,
        cx: &MeasureContext<'_>,
        hint: &str,
        track: bool,
    ) -> Result<Size, RuntimeError> {
        self.editor.validate(self.label())?;
        self.editor.validate(hint)?;
        let label = cx
            .rich_text(&self.caption.display(), None, TextStyle::Caption)?
            .size();
        let value = cx.text(hint, None, self.editor.style)?.size();
        let gap = cx.metric(MetricRole::IconGap);
        let padding = cx.metric(MetricRole::ControlPaddingX) * 0.5;
        Size::new(
            2.0 * padding
                + value.width().max(cx.metric(MetricRole::IconSize))
                + if self.label().is_empty() {
                    0.0
                } else {
                    label.width() + gap
                }
                + if track {
                    cx.metric(MetricRole::ControlWidth) + gap
                } else {
                    0.0
                }
                + self.stepper_width(cx),
            (label.height().max(value.height()) + cx.metric(MetricRole::ControlPaddingY))
                .max(cx.metric(MetricRole::ControlHeight)),
        )
        .map_err(Into::into)
    }
    pub(crate) fn arrange(
        &self,
        cx: &MeasureContext<'_>,
        size: Size,
        hint: &str,
        track: bool,
    ) -> Result<(), RuntimeError> {
        let pad = (cx.metric(MetricRole::ControlPaddingX) * 0.5).min(size.width() * 0.5);
        let gap = cx.metric(MetricRole::IconGap);
        let label = cx
            .rich_text(&self.caption.display(), None, TextStyle::Caption)?
            .size();
        let value = cx.text(hint, None, self.editor.style)?.size();
        let caption_line = cx.text("Mg", None, TextStyle::Caption)?;
        let value_line = cx.text("Mg", None, self.editor.style)?;
        let (caption_above, caption_below) = caption_line.first_line_box().unwrap_or((0.0, 0.0));
        let (value_above, value_below) = value_line.first_line_box().unwrap_or((0.0, 0.0));
        let above = caption_above.max(value_above);
        let below = caption_below.max(value_below);
        self.baseline
            .set((size.height() - above - below).max(0.0) / 2.0 + above);
        let stepper = self
            .stepper_width(cx)
            .min((size.width() - 2.0 * pad).max(0.0));
        let available = (size.width() - 2.0 * pad - stepper).max(0.0);
        if self.stepper {
            let side = cx.metric(MetricRole::IconSize).min(size.height());
            let y = (size.height() - side) / 2.0;
            let plus_x = (size.width() - pad - side).max(0.0);
            let minus_x = (plus_x - gap - side).max(0.0);
            self.buttons.set(Some((
                rect(minus_x, y, side, side)?,
                rect(plus_x, y, side, side)?,
            )));
        } else {
            self.buttons.set(None);
        }
        let vw = value
            .width()
            .max(cx.metric(MetricRole::IconSize))
            .min(available);
        let rest = (available
            - vw
            - if self.label().is_empty() && !track {
                0.0
            } else {
                gap
            })
        .max(0.0);
        let lw = if self.label().is_empty() {
            0.0
        } else {
            label.width().min(rest)
        };
        self.label_area
            .set(Some(rect(pad, 0.0, lw, size.height())?));
        let left = pad
            + lw
            + if lw > 0.0 {
                gap.min((available - lw).max(0.0))
            } else {
                0.0
            };
        let slot_x = (size.width() - pad - stepper - vw).max(pad);
        self.track.set(
            track
                .then(|| {
                    rect(
                        left.min(slot_x),
                        0.0,
                        (slot_x - left - gap).max(0.0),
                        size.height(),
                    )
                })
                .transpose()?,
        );
        self.slot.set(Some(rect(slot_x, 0.0, vw, size.height())?));
        let hint_layout = cx.text(hint, None, self.editor.style)?;
        let (line_height, value_baseline) = if self.draft_slot.is_some() {
            (value_above + value_below, value_above)
        } else {
            (value.height(), hint_layout.first_baseline().unwrap_or(0.0))
        };
        self.editor_area(
            cx.text(self.unit_text(), None, self.editor.style)?.size(),
            gap,
            line_height,
            value_baseline,
        )?;
        if self.draft_slot.is_some() {
            return Ok(());
        }
        self.editor.arrange(cx, size)
    }
    fn editor_area(
        &self,
        suffix: Size,
        gap: f32,
        line_height: f32,
        value_baseline: f32,
    ) -> Result<(), RuntimeError> {
        if let Some(slot) = self.slot.get() {
            let [x, y, w, h] = slot.values();
            let unit = if self.unit_text().is_empty() {
                0.0
            } else {
                suffix.width() + gap
            };
            self.editor.area.set(Some(rect(
                x,
                y + self.baseline.get() - value_baseline,
                (w - unit).max(0.0),
                h.min(line_height),
            )?));
        }
        Ok(())
    }
    pub(crate) fn paint(&self, p: &mut Painter) -> Result<(), DrawError> {
        let size = p.size();
        let outer = Rect::from_size(size);
        field_frame(
            p,
            outer,
            self.focused,
            self.error.is_some(),
            self.hovered && self.editor.enabled,
        )?;
        if let Some(area) = self.label_area.get() {
            let text = p.rich_text(&self.caption.display(), None, TextStyle::Caption)?;
            let [x, _, _, _] = area.values();
            p.with_clip(area, |p| {
                text.paint(
                    p,
                    crate::Point::new(
                        x,
                        self.baseline.get() - text.first_baseline().unwrap_or(0.0),
                    )?,
                    p.color(ColorRole::Muted),
                )
            })?;
        }
        let suffix = p.text(self.unit_text(), None, self.editor.style)?;
        let display = p.text(&self.display, None, self.editor.style)?;
        if !self.drafting() {
            self.editor_area(
                suffix.size(),
                p.metric(MetricRole::IconGap),
                display.size().height(),
                display.first_baseline().unwrap_or(0.0),
            )?;
        }
        let color = p.color(if self.editor.enabled {
            ColorRole::OnSurface
        } else {
            ColorRole::Muted
        });
        if let Some(slot) = self.slot.get() {
            let [x, _, w, _] = slot.values();
            p.with_clip(slot, |p| {
                suffix.paint(
                    p,
                    crate::Point::new(
                        x + (w - suffix.size().width()).max(0.0),
                        self.baseline.get() - suffix.first_baseline().unwrap_or(0.0),
                    )?,
                    color,
                )
            })?;
        }
        if self.editing {
            // A child editor draws its own draft inside the value slot.
            if self.draft_slot.is_none() {
                self.editor.paint(p)?;
            }
        } else if let Some(area) = self.editor.area.get() {
            let [x, y, w, _] = area.values();
            p.with_clip(area, |p| {
                display.paint(
                    p,
                    crate::Point::new(x + (w - display.size().width()).max(0.0), y)?,
                    color,
                )
            })?;
        }
        self.paint_buttons(p)
    }
    /// Lays out an ordinary (framed) field: the editor keeps its padded area,
    /// less the trailing step buttons when they are shown.
    pub(crate) fn arrange_plain(
        &self,
        cx: &MeasureContext<'_>,
        size: Size,
    ) -> Result<(), RuntimeError> {
        let px = cx
            .metric(MetricRole::ControlPaddingX)
            .min(size.width() / 2.0);
        let py = cx
            .metric(MetricRole::ControlPaddingY)
            .min(size.height() / 2.0);
        let stepper = self
            .stepper_width(cx)
            .min((size.width() - 2.0 * px).max(0.0));
        if self.stepper {
            let side = cx.metric(MetricRole::IconSize).min(size.height());
            let y = (size.height() - side) / 2.0;
            let plus_x = (size.width() - px - side).max(0.0);
            let minus_x = (plus_x - cx.metric(MetricRole::IconGap) - side).max(0.0);
            self.buttons.set(Some((
                rect(minus_x, y, side, side)?,
                rect(plus_x, y, side, side)?,
            )));
            self.editor.area.set(Some(rect(
                px,
                py,
                (size.width() - 2.0 * px - stepper).max(0.0),
                (size.height() - 2.0 * py).max(0.0),
            )?));
        } else {
            self.buttons.set(None);
            self.editor.area.set(None);
        }
        self.editor.arrange(cx, size)
    }
    /// Paints an ordinary field at rest: its frame and the formatted display
    /// text with its unit, in place of the editing text.
    pub(crate) fn paint_rest(&self, p: &mut Painter) -> Result<(), DrawError> {
        let size = p.size();
        let outer = Rect::from_size(size);
        field_frame(
            p,
            outer,
            false,
            false,
            self.editor.hovered && self.editor.enabled,
        )?;
        let px = p
            .metric(MetricRole::ControlPaddingX)
            .min(size.width() / 2.0);
        let py = p
            .metric(MetricRole::ControlPaddingY)
            .min(size.height() / 2.0);
        let area = match self.editor.area.get() {
            Some(area) => area,
            None => Rect::new(
                px,
                py,
                (size.width() - 2.0 * px).max(0.0),
                (size.height() - 2.0 * py).max(0.0),
            )?,
        };
        let text = if self.suffix.is_empty() {
            self.display.clone()
        } else {
            format!("{} {}", self.display, self.suffix)
        };
        let layout = p.text(&text, None, self.editor.style)?;
        let [x, y, _, h] = area.values();
        let color = p.color(if self.editor.enabled {
            ColorRole::OnSurface
        } else {
            ColorRole::Muted
        });
        p.with_clip(area, |p| {
            layout.paint(
                p,
                crate::Point::new(x, y + (h - layout.size().height()).max(0.0) / 2.0)?,
                color,
            )
        })?;
        self.paint_buttons(p)
    }
    /// Whether a local point lies on a step button.
    pub(crate) fn local_button(&self, point: Point) -> bool {
        self.buttons
            .get()
            .is_some_and(|(minus, plus)| minus.contains(point) || plus.contains(point))
    }
    /// Paints the step buttons, when shown.
    pub(crate) fn paint_buttons(&self, p: &mut Painter) -> Result<(), DrawError> {
        if let Some((minus, plus)) = self.buttons.get() {
            for (rect, up, icon) in [(minus, false, Icon::Remove), (plus, true, Icon::Add)] {
                let pressed = self.pressed_button == Some(up);
                let hovered = self.hovered_button == Some(up)
                    && self.editor.enabled
                    && !self.editor.read_only;
                if pressed || hovered {
                    p.fill_shape(
                        rect,
                        crate::choice_controls::rounded(p.metric(MetricRole::CornerRadius))?,
                        p.color(if pressed {
                            ColorRole::ButtonPressed
                        } else {
                            ColorRole::ButtonHover
                        }),
                    )?;
                }
                p.icon(
                    icon,
                    rect,
                    p.color(if !self.editor.enabled || self.editor.read_only {
                        ColorRole::Muted
                    } else if pressed {
                        ColorRole::OnButtonPressed
                    } else {
                        ColorRole::OnSurface
                    }),
                )?;
            }
        }
        Ok(())
    }
    pub(crate) fn hit_value(&self, cx: &EventContext, point: Point) -> bool {
        self.local_value(cx.to_local(point).unwrap_or(point))
    }
    pub(crate) fn local_value(&self, point: Point) -> bool {
        self.slot.get().is_some_and(|slot| slot.contains(point))
    }
    pub(crate) fn press(
        &mut self,
        cx: &mut EventContext,
        point: Point,
    ) -> Result<(), RuntimeError> {
        cx.capture_pointer()?;
        self.press = Some(Press {
            start: point,
            last: point,
            dragged: false,
        });
        Ok(())
    }
    pub(crate) fn pressed(&self) -> bool {
        self.press.is_some()
    }
    pub(crate) fn drag_delta(&mut self, point: Point) -> Option<f32> {
        let press = self.press.as_mut()?;
        if !press.dragged && (point.x() - press.start.x()).abs() < crate::DRAG_THRESHOLD {
            return None;
        }
        press.dragged = true;
        let delta = point.x() - press.last.x();
        press.last = point;
        Some(delta)
    }
    pub(crate) fn release(&mut self, cx: &mut EventContext) -> bool {
        cx.release_pointer();
        self.press.take().is_some_and(|p| !p.dragged)
    }
    pub(crate) fn inspect(&self, details: &mut ComponentDetails) {
        details
            .properties
            .insert("inline".into(), InspectValue::Bool(self.inline));
        details
            .properties
            .insert("text_editing".into(), InspectValue::Bool(self.editing));
        details.properties.insert(
            "display_value".into(),
            InspectValue::Text(self.display.clone()),
        );
        details
            .properties
            .insert("unit".into(), InspectValue::Text(self.unit_text().into()));
        details
            .properties
            .insert("stepper".into(), InspectValue::Bool(self.stepper));
        if let Some(e) = &self.error {
            details
                .properties
                .insert("validation_error".into(), InspectValue::Text(e.to_string()));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{cell::Cell, rc::Rc, time::Duration};

    struct BaselineProbe(Rc<Cell<[f32; 3]>>);
    impl Component for BaselineProbe {
        fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
            Ok(())
        }
        fn arrange(&self, cx: &mut ArrangeContext<'_>, _: Size) -> Result<(), RuntimeError> {
            let m = cx.measurement();
            self.0.set([
                m.text("CPU", None, TextStyle::Caption)?
                    .first_baseline()
                    .unwrap(),
                m.text("42", None, TextStyle::Monospace)?
                    .first_baseline()
                    .unwrap(),
                m.text("ms", None, TextStyle::Monospace)?
                    .first_baseline()
                    .unwrap(),
            ]);
            Ok(())
        }
    }

    /// The value area leaves the unit suffix and the theme's icon gap, which
    /// grows with density, instead of a fixed four pixels.
    #[test]
    fn the_editor_area_reserves_the_suffix_and_the_density_gap() {
        let ui = Runtime::new();
        let mut field = InlineField::<f64>::new(true);
        field.suffix = "ms".into();
        field.slot.set(Some(rect(10.0, 0.0, 100.0, 20.0).unwrap()));
        let suffix = Size::new(14.0, 12.0).unwrap();
        let mut widths = Vec::new();
        for density in [Density::Compact, Density::Comfortable] {
            let gap = ui
                .theme()
                .metrics(density)
                .unwrap()
                .metric(MetricRole::IconGap);
            field.editor_area(suffix, gap, 12.0, 9.0).unwrap();
            let area = field.editor.area.get().unwrap();
            assert_eq!(area.width(), 100.0 - 14.0 - gap);
            widths.push(area.width());
        }
        assert!(widths[1] < widths[0]);
    }

    #[test]
    fn caption_value_and_unit_share_a_painted_baseline() {
        let mut ui = Runtime::new();
        ui.set_viewport(crate::Viewport::new(400.0, 80.0, 1.0).unwrap())
            .unwrap();
        let value = ui.signal("42".to_owned());
        let baselines = Rc::new(Cell::new([0.0; 3]));
        let root = ui
            .mount_tree(Element::new(
                "root",
                crate::Row::new([
                    Element::new("probe", BaselineProbe(baselines.clone())),
                    Element::new(
                        "readout",
                        crate::InlineReadout::new(value.read_only())
                            .label("CPU")
                            .unit("ms"),
                    ),
                ]),
            ))
            .unwrap();
        ui.flush(Duration::ZERO).unwrap();
        let palette = ui.theme().palette(crate::ThemeMode::Dark).unwrap();
        let mut caption = None;
        let mut values = Vec::new();
        let scene = ui.paint().unwrap();
        for command in scene.commands() {
            if let crate::DrawCommand::DrawMask { rect, color, .. } = command {
                if *color == palette.color(ColorRole::Muted) {
                    caption = Some(*rect);
                } else if *color == palette.color(ColorRole::OnSurface) {
                    values.push(*rect);
                }
            }
        }
        values.sort_by(|a, b| a.x().total_cmp(&b.x()));
        assert_eq!(values.len(), 2, "value and unit are separate masks");
        let caption = caption.unwrap();
        let [caption_offset, value_offset, unit_offset] = baselines.get();
        let caption_base = caption.y() + caption_offset;
        let value_base = values[0].y() + value_offset;
        let unit_base = values[1].y() + unit_offset;
        assert!((caption_base - value_base).abs() < 0.01);
        assert!((value_base - unit_base).abs() < 0.01);

        let readout = ui.children(&root).unwrap()[1].clone();
        ui.focus(&readout).unwrap();
        ui.key_input(KeyCode::F2, crate::KeyAction::Press).unwrap();
        ui.flush(Duration::ZERO).unwrap();
        let scene = ui.paint().unwrap();
        let editing_value = scene.commands().iter().find_map(|command| match command {
            crate::DrawCommand::DrawMask { rect, color, .. }
                if *color == palette.color(ColorRole::OnSurface) && rect.x() < values[1].x() =>
            {
                Some(*rect)
            }
            _ => None,
        });
        let editing_value = editing_value.unwrap();
        assert!(
            (editing_value.y() + value_offset - caption_base).abs() < 0.5,
            "draft text leaves the inline baseline: editing={editing_value:?}, value_offset={value_offset}, caption_base={caption_base}"
        );
    }
}
