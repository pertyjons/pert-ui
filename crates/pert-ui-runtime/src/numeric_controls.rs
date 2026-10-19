//! General parameter gestures shared by the linear and inline sliders and
//! the knob.
use crate::choice_controls::{focus_ring, rounded};
use crate::layout::rect;
use crate::text_fields::InlineField;
use crate::*;
use std::cell::Cell;
use std::time::Duration;

/// Angle of a knob's minimum, in degrees clockwise from the rightward radius:
/// the lower left, so the arc opens at the bottom.
const KNOB_START_DEGREES: f32 = 135.0;
/// Angular travel of a knob from minimum to maximum. An angle rather than a
/// length, so it is not a theme metric, which zoom and DPI would scale.
const KNOB_SWEEP_DEGREES: f32 = 270.0;
/// Fraction of the dial's radius where the pointer line starts.
const KNOB_POINTER_INNER: f32 = 0.35;
/// Longest interval between two presses that a knob treats as a double-click,
/// the same interval `InlineText` uses.
pub(crate) const DOUBLE_CLICK: Duration = Duration::from_millis(500);

/// Direction of a linear slider. Positions increase rightward or upward.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Orientation {
    /// Along x; the default. A slider has its minimum on the left.
    #[default]
    Horizontal,
    /// Along y. A slider has its minimum at the bottom.
    Vertical,
}
/// Where a knob's value arc starts filling.
///
/// The arc always runs between this origin and the current value, so a
/// parameter whose rest position is inside its range, such as a balance or an
/// offset, can show its deviation in either direction. The knob attaches no
/// meaning to the origin: it is presentation only and never changes the value
/// or the edit protocol.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[non_exhaustive]
pub enum ArcOrigin {
    /// The parameter's minimum; the default. The arc grows clockwise with the value.
    #[default]
    Minimum,
    /// The normalized position of the parameter's
    /// [`ParameterSpec::default_value`]. The arc is empty at the default and
    /// grows toward either end.
    Default,
}
/// The presentation that maps pointer movement and paint for the shared control.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Shape {
    Linear,
    Rotary,
}
struct Drag {
    position: Point,
    normalized: f32,
    /// Whether the pointer has moved the value since the press.
    moved: bool,
}
struct NumericControl {
    field: InlineField<f32>,
    parameter: Parameter<f32>,
    spec: Option<ParameterSpec<f32>>,
    value: f32,
    normalized: f32,
    available: bool,
    /// A retained host delivery fault refuses edits until the host recovers.
    faulted: bool,
    authority_blocked: bool,
    /// The application's restriction of edits and focus; `enabled` holds its
    /// last read value.
    enablement: Permission,
    enabled: bool,
    focused: bool,
    hovered: bool,
    orientation: Orientation,
    shape: Shape,
    arc_origin: ArcOrigin,
    /// Time of a knob's last primary press, for double-click reset.
    last_press: Option<Duration>,
    show_value: bool,
    decimal_places: u8,
    unit: String,
    control: Cell<Option<Rect>>,
    readout: Cell<Option<Rect>>,
    reserved_columns: Cell<usize>,
    extent: Cell<f32>,
    inset: Cell<f32>,
    edit: Option<EditSession<f32>>,
    drag: Option<Drag>,
    key: Option<KeyCode>,
    error: Option<String>,
    outcome: Option<EditOutcome<f32>>,
}
impl NumericControl {
    fn new(parameter: Parameter<f32>, inline: bool, shape: Shape) -> Self {
        Self {
            field: InlineField::new(inline),
            parameter,
            spec: None,
            value: 0.0,
            normalized: 0.0,
            available: false,
            faulted: false,
            authority_blocked: false,
            enablement: Permission::allowed(),
            enabled: true,
            focused: false,
            hovered: false,
            orientation: Orientation::Horizontal,
            shape,
            arc_origin: ArcOrigin::Minimum,
            last_press: None,
            show_value: false,
            decimal_places: 2,
            unit: String::new(),
            control: Cell::new(None),
            readout: Cell::new(None),
            reserved_columns: Cell::new(0),
            extent: Cell::new(0.0),
            inset: Cell::new(0.0),
            edit: None,
            drag: None,
            key: None,
            error: None,
            outcome: None,
        }
    }
    fn inline_hint(&self) -> String {
        self.field.hint(&"0".repeat(self.columns()))
    }
    fn begin_text(&mut self, cx: &mut EventContext) -> Result<(), RuntimeError> {
        if self.edit.is_none()
            && self.begin(cx)?
            && let Err(error) = self
                .field
                .begin_text(cx, self.value, self.value.to_string())
        {
            self.edit = None;
            return Err(error);
        }
        Ok(())
    }
    fn step_text(&mut self, cx: &mut EventContext, up: bool) -> Result<(), RuntimeError> {
        if !self.field.can_step_text() {
            return Ok(());
        }
        let parsed = self.field.parse(|text| {
            text.trim()
                .replace(',', ".")
                .parse::<f32>()
                .map_err(|_| NumberInputError::InvalidNumber)
        });
        if let (Ok(value), Some(spec)) = (parsed, &self.spec)
            && let Ok(normalized) = spec.normalize(&value)
        {
            let step = if cx.modifiers().shift { 0.001 } else { 0.01 };
            let n = (normalized.get() + if up { step } else { -step }).clamp(0.0, 1.0);
            let value = spec.denormalize(NormalizedValue::try_from(n)?);
            self.field.replace_value(cx, value, value.to_string())?;
            self.field.error = None;
        }
        Ok(())
    }
    fn commit_text(&mut self, cx: &mut EventContext) -> Result<(), RuntimeError> {
        let candidate = self.field.parse(|text| {
            text.trim()
                .replace(',', ".")
                .parse::<f32>()
                .map_err(|_| NumberInputError::InvalidNumber)
        });
        let value = match candidate {
            Ok(value) if value.is_finite() => value,
            Ok(_) => {
                self.field.error = Some(NumberInputError::NonFinite.into());
                cx.invalidate(Invalidation::Paint);
                return Ok(());
            }
            Err(error) => {
                self.field.error = Some(error.into());
                cx.invalidate(Invalidation::Paint);
                return Ok(());
            }
        };
        if self
            .spec
            .as_ref()
            .is_none_or(|s| s.validate(&value).is_err())
        {
            self.field.error = Some(NumberInputError::OutOfRange.into());
            cx.invalidate(Invalidation::Paint);
            return Ok(());
        }
        self.field.prepare_display(cx, value, self.number(value))?;
        if let Some(edit) = &self.edit {
            let result = edit.update(value);
            if self.result(result)?.is_some() {
                self.value = value;
                if let Some(spec) = &self.spec {
                    self.normalized = spec.normalize(&value)?.get();
                }
                self.finish(cx)?;
            }
        }
        self.field.stop_text();
        cx.invalidate(Invalidation::Paint);
        Ok(())
    }
    fn scrub_value(&mut self, cx: &mut EventContext, point: Point) -> Result<(), RuntimeError> {
        if let Some(delta) = self.field.drag_delta(point) {
            if self.edit.is_none() && !self.begin(cx)? {
                return Ok(());
            }
            let fine = if cx.modifiers().shift { 0.1 } else { 1.0 };
            self.preview(cx, self.normalized + delta / 160.0 * fine)?;
        }
        Ok(())
    }
    fn number(&self, value: f32) -> String {
        // A value rounded to zero has no negative sign, including canonical -0.
        crate::number_format::unsigned_zero(format!(
            "{:.*}",
            usize::from(self.decimal_places),
            value
        ))
    }
    fn columns(&self) -> usize {
        self.spec.as_ref().map_or(1, |spec| {
            self.number(spec.minimum())
                .len()
                .max(self.number(spec.maximum()).len())
        })
    }
    /// The readout's text, or with `template` the text whose measured width
    /// reserves the readout slot. An installed format supplies both: its
    /// width hint reserves the slot, its display callback and its unit
    /// callback, or without one the literal unit, produce the text. Without a
    /// format the fixed decimal places pad the value to the wider endpoint.
    fn value_text(&self, template: bool) -> String {
        if let Some(format) = &self.field.format {
            if template {
                return format.hint.clone();
            }
            let mut text = (format.display)(self.value);
            let unit = format
                .unit
                .as_ref()
                .map_or_else(|| self.unit.clone(), |unit| unit(self.value));
            if !unit.is_empty() {
                text.push(' ');
                text.push_str(&unit);
            }
            return text;
        }
        let width = self.columns();
        let mut text = if template {
            "0".repeat(width)
        } else {
            format!("{:>width$}", self.number(self.value))
        };
        if !self.unit.is_empty() {
            text.push(' ');
            text.push_str(&self.unit);
        }
        text
    }
    fn value_size(&self, cx: &MeasureContext<'_>) -> Result<Size, RuntimeError> {
        if self.show_value {
            Ok(cx
                .text(&self.value_text(true), None, TextStyle::MonospaceCaption)?
                .size())
        } else {
            Ok(Size::ZERO)
        }
    }
    fn editable(&self) -> bool {
        self.enabled
            && self.available
            && !self.faulted
            && !self.authority_blocked
            && self.spec.as_ref().is_some_and(ParameterSpec::is_editable)
    }
    /// A binding refusal is reported by the control; any other runtime error
    /// is the component's failure.
    fn result<T>(&mut self, result: Result<T, RuntimeError>) -> Result<Option<T>, RuntimeError> {
        match result {
            Ok(value) => Ok(Some(value)),
            Err(error @ RuntimeError::Binding(_)) => {
                self.error = Some(error.to_string());
                if let Some(edit) = &self.edit {
                    self.outcome = edit.outcome()?;
                    if self.outcome.is_some() {
                        self.error = None;
                    }
                }
                self.edit = None;
                self.drag = None;
                self.key = None;
                Ok(None)
            }
            Err(error) => Err(error),
        }
    }
    fn begin(&mut self, cx: &mut EventContext) -> Result<bool, RuntimeError> {
        let snapshot = self.parameter.snapshot().get()?;
        let authority = self
            .parameter
            .authority_status()
            .map(|status| status.get())
            .transpose()?;
        self.authority_blocked = !crate::control_binding::authority_allows_edit(authority.as_ref());
        self.spec = Some(self.parameter.spec()?);
        self.available = snapshot.available;
        if !self.editable() {
            return Ok(false);
        }
        self.value = snapshot.preview.unwrap_or(snapshot.base);
        if let Some(spec) = &self.spec {
            self.normalized = spec.normalize(&self.value)?.get();
        }
        self.error = None;
        self.outcome = None;
        self.edit = self.result(self.parameter.begin_edit(cx, snapshot.revision))?;
        cx.invalidate(Invalidation::Paint);
        Ok(self.edit.is_some())
    }
    /// Whether a wheel step of `delta` in normalized travel changes the
    /// current value, before any edit begins.
    fn wheel_changes(&self, delta: f32) -> Result<bool, RuntimeError> {
        let snapshot = self.parameter.snapshot().get()?;
        let spec = self.parameter.spec()?;
        let value = snapshot.preview.unwrap_or(snapshot.base);
        let target = (spec.normalize(&value)?.get() + delta).clamp(0.0, 1.0);
        Ok(spec.denormalize(NormalizedValue::try_from(target)?) != value)
    }
    fn preview(&mut self, cx: &mut EventContext, normalized: f32) -> Result<(), RuntimeError> {
        // Candidate derives from finite geometry; this explicit control limit is the only clamp.
        let normalized = normalized.clamp(0.0, 1.0);
        let position = NormalizedValue::try_from(normalized)?;
        if let (Some(spec), Some(edit)) = (&self.spec, &self.edit) {
            let value = spec.denormalize(position);
            let result = edit.update(value);
            if self.result(result)?.is_some() {
                self.normalized = normalized;
                self.value = value;
            }
        }
        cx.invalidate(Invalidation::Paint);
        Ok(())
    }
    fn finish(&mut self, cx: &mut EventContext) -> Result<(), RuntimeError> {
        if let Some(edit) = self.edit.take() {
            let result = edit.commit(self.value);
            self.outcome = self.result(result)?;
        }
        self.drag = None;
        self.key = None;
        cx.release_pointer();
        cx.invalidate(Invalidation::Paint);
        Ok(())
    }
    // An unarranged control has no track geometry to miss, so every position hits.
    fn hit_control(&self, cx: &EventContext, point: Point) -> bool {
        match (cx.to_local(point), self.control.get()) {
            (Some(local), Some(control)) => control.contains(local),
            _ => true,
        }
    }
    fn coordinate(&self, point: Point) -> f32 {
        if self.shape == Shape::Rotary
            || !self.field.inline && self.orientation == Orientation::Vertical
        {
            -point.y()
        } else {
            point.x()
        }
    }
    fn apply_pointer(&mut self, cx: &mut EventContext, point: Point) -> Result<(), RuntimeError> {
        let Some(drag) = &self.drag else {
            return Ok(());
        };
        let fine = if cx.modifiers().shift { 0.1 } else { 1.0 };
        // Pointer positions are in the window; the track extent is local.
        let distance = (self.coordinate(point) - self.coordinate(drag.position)) / cx.view_scale();
        if distance == 0.0 {
            // No movement previews nothing, so a press and release without a
            // drag commit the starting value exactly rather than its
            // normalized round trip.
            return Ok(());
        }
        // A press that moved the value was a drag, not half a double-click.
        self.last_press = None;
        let normalized = drag.normalized + distance / self.extent.get().max(1.0) * fine;
        // Incremental baseline avoids jumps when Shift changes during a drag.
        self.preview(cx, normalized)?;
        if let Some(drag) = &mut self.drag {
            drag.position = point;
            drag.normalized = self.normalized;
            drag.moved = true;
        }
        Ok(())
    }
}
impl NumericControl {
    /// Ends every pending gesture, edit and typed draft without committing.
    ///
    /// Dropping the session cancels unfinished work through the parameter's
    /// own terminal path, so the host sees a cancellation rather than a
    /// silently abandoned edit. Needs no context, for reconciliation.
    fn abandon(&mut self) {
        self.field.stop_text();
        self.edit = None;
        self.drag = None;
        self.key = None;
    }
    /// The normalized position of the parameter's default, or the current
    /// position before a specification has been read.
    fn default_position(&self) -> Result<f32, RuntimeError> {
        Ok(self
            .spec
            .as_ref()
            .map(|s| s.normalize(&s.default_value()).map(NormalizedValue::get))
            .transpose()?
            .unwrap_or(self.normalized))
    }
    /// Paints a knob: a track arc, the value arc from its origin, a pointer,
    /// the readout, and the focus and error outlines.
    fn paint_knob(
        &self,
        p: &mut Painter,
        size: Size,
        control: Rect,
        enabled: bool,
    ) -> Result<(), DrawError> {
        let active = p.color(if enabled {
            ColorRole::Primary
        } else {
            ColorRole::Muted
        });
        let [x, y, side, _] = control.values();
        let stroke = p.metric(MetricRole::KnobStroke).min(side / 4.0);
        let radius = (side - stroke) / 2.0 - p.metric(MetricRole::FocusWidth);
        if radius > 0.0 && stroke > 0.0 {
            let center = Point::new(x + side / 2.0, y + side / 2.0)?;
            let disc = radius - stroke;
            if enabled && self.hovered && disc > 0.0 {
                p.fill_ellipse(
                    Rect::new(center.x() - disc, center.y() - disc, 2.0 * disc, 2.0 * disc)?,
                    p.color(ColorRole::SurfaceHover),
                )?;
            }
            let width = Length::new(stroke)?;
            let r = Length::new(radius)?;
            p.stroke_arc(
                center,
                r,
                KNOB_START_DEGREES,
                KNOB_SWEEP_DEGREES,
                width,
                p.color(ColorRole::Outline),
            )?;
            let n = self.normalized;
            let origin = match self.arc_origin {
                ArcOrigin::Minimum => 0.0,
                // A specification's own default always normalizes; before the
                // specification is read the arc starts at the minimum.
                ArcOrigin::Default => self
                    .spec
                    .as_ref()
                    .and_then(|s| s.normalize(&s.default_value()).ok())
                    .map_or(0.0, NormalizedValue::get),
            };
            p.stroke_arc(
                center,
                r,
                KNOB_START_DEGREES + KNOB_SWEEP_DEGREES * n.min(origin),
                KNOB_SWEEP_DEGREES * (n - origin).abs(),
                width,
                active,
            )?;
            let angle = (KNOB_START_DEGREES + KNOB_SWEEP_DEGREES * n).to_radians();
            let at = |fraction: f32| {
                Point::new(
                    center.x() + radius * fraction * angle.cos(),
                    center.y() + radius * fraction * angle.sin(),
                )
            };
            p.stroke_polyline(
                &[at(KNOB_POINTER_INNER)?, at(1.0)?],
                StrokeStyle::new(width).with_cap(LineCap::Round),
                p.color(if enabled {
                    ColorRole::OnSurface
                } else {
                    ColorRole::Muted
                }),
            )?;
        }
        if let Some(rect) = self.readout.get() {
            let color = p.color(if enabled {
                ColorRole::OnSurface
            } else {
                ColorRole::Muted
            });
            let text = p.text(&self.value_text(false), None, TextStyle::MonospaceCaption)?;
            let [x, y, width, _] = rect.values();
            // Padded default text fills its slot; formatted text, whose width
            // the hint reserved, ends at the slot's right edge so the value's
            // last digit and unit stay put as the value changes.
            let x = if self.field.format.is_some() {
                x + (width - text.size().width()).max(0.0)
            } else {
                x
            };
            p.with_clip(rect, |p| text.paint(p, x, y, color))?;
        }
        if self.focused {
            focus_ring(p, control)?;
        }
        if self.error.is_some() || matches!(self.outcome, Some(EditOutcome::Rejected(_))) {
            p.stroke_shape(
                Rect::from_size(size),
                rounded(p.metric(MetricRole::CornerRadius))?,
                Length::new(p.metric(MetricRole::FocusWidth))?,
                p.color(ColorRole::Error),
            )?;
        }
        Ok(())
    }
    /// Whether Escape has anything to cancel.
    fn busy(&self) -> bool {
        self.edit.is_some() || self.drag.is_some() || self.key.is_some() || self.field.pressed()
    }
}
impl Component for NumericControl {
    fn reconcile(&mut self, next: Self) -> Option<Invalidation> {
        let Self {
            field,
            parameter,
            enablement,
            enabled,
            orientation,
            show_value,
            decimal_places,
            unit,
            arc_origin,
            shape: _,
            last_press: _,
            spec: _,
            value: _,
            normalized: _,
            available: _,
            faulted: _,
            authority_blocked: _,
            focused: _,
            control: _,
            readout: _,
            reserved_columns: _,
            extent: _,
            inset: _,
            edit: _,
            drag: _,
            key: _,
            error: _,
            outcome: _,
            hovered: _,
        } = next;
        let mut invalidation = None;
        if !parameter.snapshot().same_source(&self.parameter.snapshot()) {
            // Cancel the old parameter's edit before rebinding, so no pending
            // gesture or typed value can commit to either parameter.
            self.abandon();
            // A click on the old parameter is not half of a double-click on
            // the new one.
            self.last_press = None;
            self.error = None;
            self.outcome = None;
            self.parameter = parameter;
            self.spec = None;
            invalidation = Some(Invalidation::Layout);
        }
        if !self.enablement.same_as(&enablement) {
            self.enablement = enablement;
            invalidation = invalidation.max(Some(Invalidation::Paint));
        }
        if let Some(enabled) = self.enablement.fixed().filter(|_| enabled != self.enabled) {
            // A fixed denial applies at once; a binding is read at update.
            if !enabled {
                self.abandon();
            }
            self.enabled = enabled;
            invalidation = invalidation.max(Some(Invalidation::Paint));
        }
        if arc_origin != self.arc_origin {
            self.arc_origin = arc_origin;
            invalidation = invalidation.max(Some(Invalidation::Paint));
        }
        let caption_changed = self.field.caption.reconcile(field.caption);
        if caption_changed
            || orientation != self.orientation
            || show_value != self.show_value
            || decimal_places != self.decimal_places
            || unit != self.unit
            || field.unit != self.field.unit
            || !NumberFormat::same_optional(&field.format, &self.field.format)
        {
            // Cloned formats share identity; new callbacks or metadata remeasure.
            self.orientation = orientation;
            self.show_value = show_value;
            self.decimal_places = decimal_places;
            self.unit = unit;
            self.field.unit = field.unit;
            self.field.format = field.format;
            invalidation = Some(Invalidation::Layout);
        }
        invalidation
    }
    fn focusable(&self) -> bool {
        // In text mode focus belongs to the row's child editor.
        self.editable() && !self.field.drafting()
    }
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        let enabled = self.enablement.read_paint(cx)?;
        if self.enabled && !enabled && self.busy() {
            // Revocation ends a gesture, held key or typed value without
            // committing it.
            self.input_reset();
            cx.cancel_input();
        }
        self.enabled = enabled;
        let snapshot = cx.read_paint(&self.parameter.snapshot())?;
        self.faulted = cx.read_paint(&self.parameter.delivery_status())? == DeliveryStatus::Faulted;
        let authority = self
            .parameter
            .authority_status()
            .map(|status| cx.read_paint(&status))
            .transpose()?;
        self.authority_blocked = !crate::control_binding::authority_allows_edit(authority.as_ref());
        self.spec = Some(self.parameter.spec()?);
        // An installed format reserves its own width hint, so only the
        // default fixed-decimal presentation follows the endpoints' digits.
        if self.field.format.is_none()
            && (self.field.inline || self.show_value)
            && self.columns() != self.reserved_columns.get()
        {
            cx.request_layout();
        }
        self.available = snapshot.available;
        self.value = snapshot.preview.unwrap_or(snapshot.base);
        if let Some(spec) = &self.spec {
            self.normalized = spec.normalize(&self.value)?.get();
        }
        if let Some(edit) = &self.edit
            && let Some(outcome) = edit.outcome()?
        {
            self.outcome = Some(outcome);
            self.field.stop_text();
            self.edit = None;
            self.drag = None;
            self.key = None;
            cx.cancel_input();
        }
        self.focused = cx.focused();
        if self.field.inline {
            if self.edit.is_none() && self.field.editing {
                self.field.stop_text();
            }
            self.field.enabled(self.editable());
            self.field
                .display_value(self.value, self.number(self.value))?;
            self.field.update(cx)?;
        }
        Ok(())
    }
    fn measure(&self, cx: &MeasureContext<'_>, _: Constraints) -> Result<Size, RuntimeError> {
        if self.field.inline {
            return self.field.measure(cx, &self.inline_hint(), true);
        }
        let short = cx.metric(MetricRole::ControlHeight);
        let long = cx.metric(MetricRole::ControlWidth) * 3.0;
        let gap = cx.metric(MetricRole::IconGap);
        let value = self.value_size(cx)?;
        if self.shape == Shape::Rotary {
            let diameter = cx.metric(MetricRole::KnobDiameter);
            return Ok(Size::new(
                diameter.max(value.width()),
                diameter
                    + if self.show_value {
                        gap + value.height()
                    } else {
                        0.0
                    },
            )?);
        }
        Ok(if self.orientation == Orientation::Vertical {
            Size::new(
                short.max(value.width()),
                long + if self.show_value {
                    gap + value.height()
                } else {
                    0.0
                },
            )?
        } else {
            Size::new(
                long + if self.show_value {
                    gap + value.width()
                } else {
                    0.0
                },
                short.max(value.height()),
            )?
        })
    }
    fn arrange(&self, cx: &mut ArrangeContext<'_>, size: Size) -> Result<(), RuntimeError> {
        if self.field.inline {
            self.field
                .arrange(cx.measurement(), size, &self.inline_hint(), true)?;
            self.field.place(cx)?;
        } else if !cx.is_empty() {
            return Err(LayoutError::InvalidChild.into());
        }
        let cx = cx.measurement();
        if self.field.inline {
            self.reserved_columns.set(self.columns());
            self.control.set(self.field.track.get());
            if let Some(rect) = self.control.get() {
                let [_, _, w, h] = rect.values();
                let inset = cx.metric(MetricRole::IconSize).min(w).min(h) / 2.0;
                self.inset.set(inset);
                self.extent.set((w - 2.0 * inset).max(1.0));
            }
            return Ok(());
        }
        let value_size = self.value_size(cx)?;
        let gap = cx.metric(MetricRole::IconGap);
        let (w, h) = (size.width(), size.height());
        if self.shape == Shape::Rotary {
            // The dial is the largest square above the readout, centered.
            let remaining = (h - if self.show_value {
                value_size.height() + gap
            } else {
                0.0
            })
            .max(0.0);
            let side = remaining.min(w);
            let y = (remaining + gap).min(h);
            let width = value_size.width().min(w);
            self.control
                .set(Some(rect((w - side) / 2.0, 0.0, side, side)?));
            self.readout.set(self.show_value.then_some(rect(
                (w - width) / 2.0,
                y,
                width,
                (h - y).min(value_size.height()),
            )?));
            if self.show_value {
                self.reserved_columns.set(self.columns());
            }
            self.inset.set(0.0);
            self.extent.set(cx.metric(MetricRole::KnobTravel).max(1.0));
            return Ok(());
        }
        let (control, value_rect) = if self.orientation == Orientation::Vertical {
            // The readout sits centered below the track.
            let remaining = (h - if self.show_value {
                value_size.height() + gap
            } else {
                0.0
            })
            .max(0.0);
            let y = (remaining + gap).min(h);
            let width = value_size.width().min(w);
            (
                rect(0.0, 0.0, w, remaining)?,
                rect(
                    (w - width) / 2.0,
                    y,
                    width,
                    (h - y).min(value_size.height()),
                )?,
            )
        } else {
            // The readout sits at the right of the track.
            let remaining = (w - if self.show_value {
                value_size.width() + gap
            } else {
                0.0
            })
            .max(0.0);
            let x = (remaining + gap).min(w);
            let height = value_size.height().min(h);
            (
                rect(0.0, 0.0, remaining, h)?,
                rect(
                    x,
                    (h - height) / 2.0,
                    (w - x).min(value_size.width()),
                    height,
                )?,
            )
        };
        self.readout.set(self.show_value.then_some(value_rect));
        if self.show_value {
            self.reserved_columns.set(self.columns());
        }
        self.control.set(Some(control));
        let [_, _, width, height] = control.values();
        let inset = cx.metric(MetricRole::IconSize).min(width).min(height) / 2.0;
        self.inset.set(inset);
        self.extent
            .set(if self.orientation == Orientation::Vertical {
                (height - 2.0 * inset).max(1.0)
            } else {
                (width - 2.0 * inset).max(1.0)
            });
        Ok(())
    }
    fn pointer_input(
        &mut self,
        cx: &mut EventContext,
        e: PointerEvent,
    ) -> Result<InputOutcome, RuntimeError> {
        if matches!(e.kind(), PointerKind::Move | PointerKind::Leave) {
            let over = e.kind() == PointerKind::Move
                && !e.is_outside()
                && cx.pointer_inside()
                && self.editable();
            let track = over && self.hit_control(cx, e.position());
            if self.hovered != track {
                self.hovered = track;
                cx.invalidate(Invalidation::Paint);
            }
            if self.field.inline {
                self.field.set_hover(
                    cx,
                    over && (track || self.field.hit_value(cx, e.position())),
                );
            }
        }
        if self.field.inline {
            if self.field.editing {
                if let PointerKind::Scroll(delta) = e.kind()
                    && delta.y() != 0.0
                    && (cx.focused() || self.field.focused)
                    && self.field.hit_value(cx, e.position())
                {
                    self.step_text(cx, delta.y() < 0.0)?;
                    return Ok(InputOutcome::Handled);
                }
                return self.field.text_pointer(cx, e);
            }
            if self.field.pressed() {
                match e.kind() {
                    PointerKind::Move => self.scrub_value(cx, e.position())?,
                    PointerKind::Release(PointerButton::Primary) => {
                        if !e.is_outside() {
                            self.scrub_value(cx, e.position())?;
                        }
                        let click = self.field.release(cx);
                        if click && self.field.hit_value(cx, e.position()) {
                            self.begin_text(cx)?;
                        } else if self.edit.is_some() {
                            self.finish(cx)?;
                        }
                    }
                    _ => {}
                }
                return Ok(InputOutcome::Handled);
            }
            if self.editable()
                && self.edit.is_none()
                && self.field.hit_value(cx, e.position())
                && e.kind() == PointerKind::Press(PointerButton::Primary)
            {
                self.field.press(cx, e.position())?;
                return Ok(InputOutcome::Handled);
            }
        }
        match e.kind() {
            PointerKind::Press(PointerButton::Primary)
                if self.edit.is_none() && self.key.is_none() =>
            {
                if !self.hit_control(cx, e.position()) {
                    return Ok(InputOutcome::Ignored);
                }
                let now = cx.now();
                if self.shape == Shape::Rotary {
                    let double = self
                        .last_press
                        .is_some_and(|last| now.saturating_sub(last) <= DOUBLE_CLICK);
                    self.last_press = None;
                    if double {
                        // A double-click is one complete edit to the default.
                        if self.begin(cx)? {
                            let default = self.default_position()?;
                            self.preview(cx, default)?;
                            self.finish(cx)?;
                        }
                        return Ok(InputOutcome::Handled);
                    }
                }
                if !self.begin(cx)? {
                    return Ok(InputOutcome::Handled);
                }
                if self.shape == Shape::Rotary {
                    // Only a press that began an edit can start a double-click.
                    self.last_press = Some(now);
                }
                cx.capture_pointer()?;
                // A knob moves relative to the press; only a slider jumps to it.
                if self.shape == Shape::Linear
                    && !cx.modifiers().shift
                    && let Some(local) = cx.to_local(e.position())
                    && let Some(control) = self.control.get()
                {
                    let [left, top, _, h] = control.values();
                    let value = if !self.field.inline && self.orientation == Orientation::Vertical {
                        (top + h - self.inset.get() - local.y()) / self.extent.get()
                    } else {
                        (local.x() - left - self.inset.get()) / self.extent.get()
                    };
                    self.preview(cx, value)?;
                }
                if self.edit.is_some() {
                    self.drag = Some(Drag {
                        position: e.position(),
                        normalized: self.normalized,
                        moved: false,
                    });
                }
            }
            PointerKind::Scroll(delta) if delta.y() != 0.0 && cx.focused() && self.editable() => {
                if !self.hit_control(cx, e.position())
                    && !(self.field.inline && self.field.hit_value(cx, e.position()))
                {
                    return Ok(InputOutcome::Ignored);
                }
                // A wheel event is one complete edit. Never steal an existing
                // pointer/key gesture or leave a session waiting for a release.
                let step = if cx.modifiers().shift { 0.001 } else { 0.01 };
                let direction = if delta.y() < 0.0 { 1.0 } else { -1.0 };
                // A step that would leave the value as it is, as at an endpoint,
                // starts no edit, so the host records nothing for it.
                if self.edit.is_none() && self.wheel_changes(direction * step)? && self.begin(cx)? {
                    self.preview(cx, self.normalized + direction * step)?;
                    self.finish(cx)?;
                }
            }
            PointerKind::Move if self.drag.is_some() => self.apply_pointer(cx, e.position())?,
            PointerKind::Release(PointerButton::Primary) if self.drag.is_some() => {
                if !e.is_outside() {
                    self.apply_pointer(cx, e.position())?;
                }
                if self.shape == Shape::Rotary && self.drag.as_ref().is_some_and(|d| !d.moved) {
                    // A knob click that moved nothing records no edit: the
                    // edit is cancelled, so a double-click commits only its
                    // reset.
                    self.edit = None;
                    self.drag = None;
                    cx.release_pointer();
                    cx.invalidate(Invalidation::Paint);
                } else {
                    self.finish(cx)?;
                }
            }
            _ => return Ok(InputOutcome::Ignored),
        }
        Ok(InputOutcome::Handled)
    }
    fn keyboard_event(
        &mut self,
        cx: &mut EventContext,
        e: KeyEvent,
        k: &KeyboardState,
    ) -> Result<InputOutcome, RuntimeError> {
        let control = k.control_key(e);
        if self.field.inline && self.field.editing {
            let drafting = self.field.drafting();
            if control == KeyCode::Escape && e.transition == KeyTransition::Pressed {
                self.input_reset();
                cx.release_pointer();
                if drafting {
                    cx.request_focus();
                }
                cx.invalidate(Invalidation::Paint);
                return Ok(InputOutcome::Handled);
            }
            if control == KeyCode::Enter
                && e.transition == KeyTransition::Pressed
                && !self.field.composing()
            {
                self.commit_text(cx)?;
                if drafting && !self.field.editing {
                    cx.request_focus();
                }
                return Ok(InputOutcome::Handled);
            }
            if matches!(control, KeyCode::ArrowUp | KeyCode::ArrowDown)
                && matches!(
                    e.transition,
                    KeyTransition::Pressed | KeyTransition::Repeated
                )
            {
                if !crate::activation::steps_with(k.chord_modifiers(e)) {
                    return Ok(InputOutcome::Ignored);
                }
                self.step_text(cx, control == KeyCode::ArrowUp)?;
                return Ok(InputOutcome::Handled);
            }
            return self.field.key(cx, KeyEvent { code: control, ..e });
        }
        if self.field.inline
            && !self.field.pressed()
            && matches!(control, KeyCode::Enter | KeyCode::F2)
            && e.transition == KeyTransition::Pressed
        {
            self.begin_text(cx)?;
            return Ok(InputOutcome::Handled);
        }
        if self.field.pressed() && control != KeyCode::Escape {
            return Ok(InputOutcome::Handled);
        }
        if control == KeyCode::Escape {
            // An idle control leaves Escape to View, Window and Application commands.
            if !self.busy() {
                return Ok(InputOutcome::Ignored);
            }
            self.input_reset();
            cx.release_pointer();
            cx.invalidate(Invalidation::Paint);
            return Ok(InputOutcome::Handled);
        }
        if !matches!(
            control,
            KeyCode::ArrowLeft
                | KeyCode::ArrowRight
                | KeyCode::ArrowUp
                | KeyCode::ArrowDown
                | KeyCode::PageUp
                | KeyCode::PageDown
                | KeyCode::Home
                | KeyCode::End
                | KeyCode::Backspace
        ) {
            return Ok(InputOutcome::Ignored);
        }
        if self.drag.is_some() {
            return Ok(InputOutcome::Handled);
        }
        match e.transition {
            // Control, Alt and Super chords are left to command bindings.
            KeyTransition::Pressed
                if self.key.is_none() && !crate::activation::steps_with(k.modifiers()) =>
            {
                return Ok(InputOutcome::Ignored);
            }
            KeyTransition::Pressed if self.key.is_none() => {
                if !self.begin(cx)? {
                    return Ok(InputOutcome::Handled);
                }
                self.key = Some(e.code);
            }
            KeyTransition::Repeated if self.key == Some(e.code) => {}
            KeyTransition::Released if self.key == Some(e.code) => {
                self.finish(cx)?;
                return Ok(InputOutcome::Handled);
            }
            _ => return Ok(InputOutcome::Handled),
        }
        let step = if k.modifiers().shift { 0.001 } else { 0.01 };
        let value = match control {
            KeyCode::Home => 0.0,
            KeyCode::End => 1.0,
            KeyCode::Backspace => self.default_position()?,
            KeyCode::PageUp => self.normalized + step * 10.0,
            KeyCode::PageDown => self.normalized - step * 10.0,
            KeyCode::ArrowLeft | KeyCode::ArrowDown => self.normalized - step,
            _ => self.normalized + step,
        };
        self.preview(cx, value)?;
        Ok(InputOutcome::Handled)
    }
    fn input_reset(&mut self) {
        self.field.stop_text();
        if let Some(edit) = &self.edit {
            // Cancel explicitly, so the terminal outcome is recorded here even
            // when no lost context cancelled the session first, as when focus
            // leaves the row's editor. Outside a writable phase the retained
            // outcome is read instead, and dropping the session cancels it.
            match edit.cancel() {
                Ok(outcome) => self.outcome = Some(outcome),
                Err(_) => match edit.outcome() {
                    Ok(outcome) => self.outcome = outcome,
                    Err(error) => self.error = Some(error.to_string()),
                },
            }
        }
        self.edit = None;
        self.drag = None;
        self.key = None;
        self.focused = false;
        self.hovered = false;
        self.last_press = None;
    }
    fn paint(&self, p: &mut Painter) -> Result<(), DrawError> {
        let Some(size) = p.size() else {
            return Ok(());
        };
        if self.field.inline {
            self.field.paint(p)?;
        }
        let enabled = self.editable();
        let active = p.color(if enabled {
            ColorRole::Primary
        } else {
            ColorRole::Muted
        });
        let neutral = p.color(ColorRole::Outline);
        let n = self.normalized;
        let Some(control) = self.control.get() else {
            return Ok(());
        };
        if self.shape == Shape::Rotary {
            return self.paint_knob(p, size, control, enabled);
        }
        let [x, y, width, height] = control.values();
        let vertical = !self.field.inline && self.orientation == Orientation::Vertical;
        let (short, long) = if vertical {
            (width, height)
        } else {
            (height, width)
        };
        let diameter = p.metric(MetricRole::IconSize).min(short).min(long);
        if enabled && self.hovered && !self.field.inline {
            let hover = if vertical {
                Rect::new(x + (width - diameter) / 2.0, y, diameter, height)?
            } else {
                Rect::new(x, y + (height - diameter) / 2.0, width, diameter)?
            };
            p.fill_shape(
                hover,
                rounded(p.metric(MetricRole::CornerRadius))?,
                p.color(ColorRole::SurfaceHover),
            )?;
        }
        let inset = diameter / 2.0;
        let extent = (long - diameter).max(0.0);
        let pos = inset + extent * if vertical { 1.0 - n } else { n };
        let thickness = (p.metric(MetricRole::BorderWidth) * 2.0).min(short);
        let rect = if vertical {
            Rect::new(x + (short - thickness) / 2.0, y + inset, thickness, extent)?
        } else {
            Rect::new(x + inset, y + (short - thickness) / 2.0, extent, thickness)?
        };
        let shape = rounded(p.metric(MetricRole::CornerRadius))?;
        p.fill_shape(rect, shape, neutral)?;
        let fill = if vertical {
            Rect::new(
                x + (short - thickness) / 2.0,
                y + pos,
                thickness,
                extent * n,
            )?
        } else {
            Rect::new(
                x + inset,
                y + (short - thickness) / 2.0,
                extent * n,
                thickness,
            )?
        };
        p.fill_shape(fill, shape, active)?;
        let breadth = diameter * 0.65;
        let thumb = if vertical {
            Rect::new(
                x + (short - breadth) / 2.0,
                y + pos - inset,
                breadth,
                diameter,
            )?
        } else {
            Rect::new(
                x + pos - inset,
                y + (short - breadth) / 2.0,
                diameter,
                breadth,
            )?
        };
        p.fill_shape(thumb, shape, active)?;
        if let Some(rect) = self.readout.get() {
            let color = p.color(if enabled {
                ColorRole::OnSurface
            } else {
                ColorRole::Muted
            });
            let text = p.text(&self.value_text(false), None, TextStyle::MonospaceCaption)?;
            let [x, y, width, _] = rect.values();
            // Padded default text fills its slot; formatted text, whose width
            // the hint reserved, ends at the slot's right edge so the value's
            // last digit and unit stay put as the value changes.
            let x = if self.field.format.is_some() {
                x + (width - text.size().width()).max(0.0)
            } else {
                x
            };
            p.with_clip(rect, |p| text.paint(p, x, y, color))?;
        }
        let rect = Rect::from_size(size);
        if self.focused {
            focus_ring(p, rect)?;
        }
        if self.error.is_some() || matches!(self.outcome, Some(EditOutcome::Rejected(_))) {
            p.stroke_shape(
                rect,
                rounded(p.metric(MetricRole::CornerRadius))?,
                Length::new(p.metric(MetricRole::FocusWidth))?,
                p.color(ColorRole::Error),
            )?;
        }
        Ok(())
    }
    fn inspect(&self) -> ComponentDetails {
        let mut d = ComponentDetails::default();
        if self.field.inline {
            self.field.inspect(&mut d);
        }
        d.validation.interactive = self.editable();
        d.properties
            .insert("value".into(), InspectValue::Number(self.value.into()));
        d.properties.insert(
            "normalized".into(),
            InspectValue::Number(self.normalized.into()),
        );
        if !self.field.inline && self.show_value {
            d.properties
                .insert("readout".into(), InspectValue::Text(self.value_text(false)));
        }
        d.properties
            .insert("editing".into(), InspectValue::Bool(self.edit.is_some()));
        d.properties
            .insert("editable".into(), InspectValue::Bool(self.editable()));
        if let Some(error) = &self.error {
            d.properties
                .insert("edit_error".into(), InspectValue::Text(error.clone()));
        }
        if let Some(outcome) = &self.outcome {
            d.properties.insert(
                "edit_outcome".into(),
                InspectValue::Text(format!("{outcome:?}")),
            );
        }
        // The inline caption is the row's own name; otherwise the parameter's.
        let name = if self.field.label().is_empty() {
            self.spec.as_ref().map_or("", |spec| spec.name())
        } else {
            self.field.label()
        };
        d.semantics = Some(
            Semantics::new(SemanticRole::Slider)
                .name(name)
                .value(SemanticValue::Number {
                    value: self.value.into(),
                    minimum: self.spec.as_ref().map(|spec| spec.minimum().into()),
                    maximum: self.spec.as_ref().map(|spec| spec.maximum().into()),
                })
                .invalid(
                    self.error.is_some()
                        || (self.field.inline && self.field.error.is_some())
                        || matches!(self.outcome, Some(EditOutcome::Rejected(_))),
                )
                .disabled(!self.editable())
                .enabled_actions([
                    SemanticAction::SetValue,
                    SemanticAction::Increment,
                    SemanticAction::Decrement,
                ]),
        );
        d
    }
}
macro_rules! numeric_widget {
    ($name:ident, $inline:expr, $shape:expr, $doc:literal) => {
        #[doc=$doc]
        ///
        /// Uses the parameter's linear/logarithmic mapping without domain-specific
        /// units. Pointer/key press begins one edit, moves/repeats preview, and
        /// matched release commits once. Escape, focus loss, hiding or removal
        /// cancel. External base writes/metadata changes win over an active edit.
        /// Arrows step 1% of normalized travel; Page keys step 10%; Shift reduces
        /// either by ten. Home/End reach endpoints; Backspace restores the default.
        /// Control, Alt and Super chords of these keys, including Up/Down while
        /// a typed value is edited, are left to command bindings.
        /// While focused, each nonzero vertical wheel event over the control steps
        /// 1% (0.1% with Shift), upward increasing, clamped to endpoints. Magnitude
        /// is not a multiplier. Each wheel event begins, previews and commits one
        /// edit immediately; a step that would not change the value, as at an
        /// endpoint, starts no edit. Wheel events during a pointer/key gesture are consumed
        /// without changing it. Unfocused, disabled/read-only/unavailable controls,
        /// slider readouts and horizontal-only input preserve normal scrolling.
        /// Only one pointer or physical key owns a gesture; unrelated releases
        /// cannot commit it. Read-only/unavailable parameters cannot start edits.
        /// Admission rejections appear as an error outline and owned inspection
        /// properties; runtime failures propagate. Effective telemetry is never edited.
        /// Values repaint only dependent controls, including cached text inside
        /// fixed readout slots. Range changes request layout only if the reserved
        /// digit count of the default presentation changes; an installed format
        /// reserves its own width hint. No timer or layout is scheduled for ordinary edits.
        /// Escape is consumed only while it cancels a gesture or typed value;
        /// otherwise it reaches View, Window and Application commands.
        /// Re-describing the control under the same key with another parameter
        /// cancels a pending edit of the old one, which reports its terminal
        /// outcome, before binding the new one; other properties apply in place.
        #[must_use]
        pub struct $name(NumericControl);
        impl $name {
            /// Creates an enabled control sharing the parameter and its edit protocol.
            /// Keep the separate host alive for as long as editing is available.
            /// Displays editable preview/base; remote proposals remain visible with
            /// input disabled while authority is pending, unknown or disconnected.
            /// Effective overlays are separate read-only data for custom displays.
            pub fn new(parameter: Parameter<f32>) -> Self {
                Self(NumericControl::new(parameter, $inline, $shape))
            }
            /// Sets fixed fractional digits, from 0 through
            /// [`NumberInputError::MAX_PRECISION`](crate::NumberInputError::MAX_PRECISION);
            /// the default is 2. Rounding affects presentation only; negative zero
            /// displays as zero. The number inputs take the same setting under
            /// the same name and limit.
            ///
            /// # Errors
            /// Returns [`NumberInputError::InvalidPrecision`](crate::NumberInputError::InvalidPrecision)
            /// above the limit.
            pub fn decimal_places(mut self, places: u8) -> Result<Self, crate::NumberInputError> {
                if places > crate::NumberInputError::MAX_PRECISION {
                    return Err(crate::NumberInputError::InvalidPrecision);
                }
                self.0.decimal_places = places;
                Ok(self)
            }
            /// Sets an optional literal unit, empty by default, without changing
            /// canonical values or the parameter mapping. Sliders append it to an
            /// enabled readout; inline rows show it after the value.
            /// This does not enable a slider readout; use `show_value(true)`.
            pub fn unit(mut self, unit: impl Into<String>) -> Self {
                let unit = unit.into();
                self.0.field.unit = unit.clone();
                self.0.unit = unit;
                self
            }
            /// Disables user editing and automatic focus; incoming values still
            /// repaint. Equivalent to `enabled(false)`; a later permission cannot
            /// lift the denial.
            pub fn disabled(self) -> Self {
                self.enabled(false)
            }
            /// Restricts edits and focus with a fixed or reactive permission;
            /// allowed by default. Repeated calls combine restrictions; every
            /// condition must allow. The parameter's own editability remains an
            /// additional restriction. Same-key redescription replaces the
            /// permission with the new description's.
            ///
            /// The control reads a bound permission in its update. Revoking it
            /// while the user drags, holds a key or types a value cancels that
            /// gesture without committing it, as Escape does: a parameter
            /// gesture reports its cancelled outcome and never commits. The
            /// control then stops being focusable, so focus moves away.
            /// Granting it again replays nothing. A binding from another
            /// runtime fails the update with [`RuntimeError::ForeignRuntime`].
            pub fn enabled(mut self, permission: impl Into<Permission>) -> Self {
                self.0.enablement = self.0.enablement.clone().and(permission);
                if let Some(enabled) = self.0.enablement.fixed() {
                    self.0.enabled = enabled;
                }
                self
            }
        }
        impl Component for $name {
            fn reconcile(&mut self, next: Self) -> Option<Invalidation> {
                self.0.reconcile(next.0)
            }
            fn cursor(&self, point: Point) -> Option<CursorIcon> {
                if self.0.field.drafting() {
                    return None;
                }
                if self.0.shape == Shape::Rotary
                    && self.0.editable()
                    && self.0.control.get().is_none_or(|c| c.contains(point))
                {
                    return Some(CursorIcon::ResizeVertical);
                }
                if self.0.field.inline && self.0.editable() && self.0.field.local_value(point) {
                    Some(if self.0.field.editing {
                        CursorIcon::Text
                    } else {
                        CursorIcon::ResizeHorizontal
                    })
                } else {
                    None
                }
            }
            fn text_input_state(&self) -> Option<TextInputState> {
                self.0.field.input_state()
            }
            fn text_input(
                &mut self,
                cx: &mut EventContext,
                text: &str,
            ) -> Result<(), RuntimeError> {
                self.0.field.text(cx, text)
            }
            fn text_preedit(
                &mut self,
                cx: &mut EventContext,
                text: &TextPreedit,
            ) -> Result<(), RuntimeError> {
                self.0.field.preedit(cx, text)
            }
            fn focusable(&self) -> bool {
                self.0.focusable()
            }
            fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
                self.0.update(cx)
            }
            fn measure(
                &self,
                cx: &MeasureContext<'_>,
                a: Constraints,
            ) -> Result<Size, RuntimeError> {
                self.0.measure(cx, a)
            }
            fn arrange(&self, cx: &mut ArrangeContext<'_>, s: Size) -> Result<(), RuntimeError> {
                self.0.arrange(cx, s)
            }
            fn pointer_input(
                &mut self,
                cx: &mut EventContext,
                e: PointerEvent,
            ) -> Result<InputOutcome, RuntimeError> {
                self.0.pointer_input(cx, e)
            }
            fn keyboard_event(
                &mut self,
                cx: &mut EventContext,
                e: KeyEvent,
                k: &KeyboardState,
            ) -> Result<InputOutcome, RuntimeError> {
                self.0.keyboard_event(cx, e, k)
            }
            fn text_key_event(
                &mut self,
                cx: &mut EventContext,
                input: CommandKeyInput,
                transition: KeyTransition,
                k: &KeyboardState,
            ) -> Result<TextKeyOutcome, RuntimeError> {
                // Only a typed value is text; the track and value gestures keep
                // the ordinary command routing.
                if self.0.field.drafting() {
                    // The keys the row's editor leaves to it, answered in the
                    // text phase so capture bindings never take them.
                    let handled = self
                        .0
                        .keyboard_event(cx, input.editing_event(transition), k)?
                        == InputOutcome::Handled;
                    return Ok(if handled {
                        TextKeyOutcome::Handled
                    } else {
                        TextKeyOutcome::Ignored
                    });
                }
                if !self.0.field.editing {
                    return Ok(TextKeyOutcome::NotAnEditor);
                }
                let handled = self
                    .0
                    .keyboard_event(cx, input.editing_event(transition), k)?
                    == InputOutcome::Handled;
                Ok(self.0.field.text_key(cx, input, transition, handled))
            }
            fn text_command(
                &mut self,
                cx: &mut EventContext,
                command: TextCommand,
            ) -> Result<TextCommandOutcome, RuntimeError> {
                self.0.field.command(cx, command)
            }
            fn input_reset(&mut self) {
                self.0.input_reset();
            }
            fn child_visible(&self, key: &str) -> bool {
                self.0.field.child_visible(key)
            }
            fn focus_lost(&mut self, cx: &mut EventContext) -> Result<(), RuntimeError> {
                if self.0.field.drafting() {
                    // Focus moved into the row's own editor.
                    return Ok(());
                }
                self.cancel_input(cx);
                Ok(())
            }
            fn focus_within_changed(
                &mut self,
                cx: &mut EventContext,
                within: bool,
            ) -> Result<(), RuntimeError> {
                // Focus left the row while typing: a parameter's typed value
                // is cancelled, as on any focus loss.
                if !within && self.0.field.drafting() {
                    self.cancel_input(cx);
                }
                Ok(())
            }
            fn paint(&self, p: &mut Painter) -> Result<(), DrawError> {
                self.0.paint(p)
            }
            fn hover_changed(&mut self, cx: &mut EventContext, hovered: bool) {
                if !hovered {
                    if self.0.hovered {
                        self.0.hovered = false;
                        cx.invalidate(Invalidation::Paint);
                    }
                    self.0.field.set_hover(cx, false);
                }
            }
            fn inspect(&self) -> ComponentDetails {
                self.0.inspect()
            }
        }
    };
}
macro_rules! readout_api {
    ($name:ident) => {
        impl $name {
            /// Shows or hides the numeric readout, off by default. It sits at
            /// the right (horizontal) or centered below (vertical).
            /// Fixed decimal places and monospaced padding reserve the larger of
            /// both formatted endpoints, including the sign and rounding carry;
            /// an installed [`format`](Self::format) reserves its width hint instead.
            /// Ordinary value changes preserve track length, readout origin and
            /// decimal alignment. Metadata, format or theme changes may resize it.
            /// Constrained allocations clip text without wrapping it or overlapping
            /// the control. Slider readouts do not start gestures.
            pub fn show_value(mut self, visible: bool) -> Self {
                self.0.show_value = visible;
                self
            }
            /// Installs an application format for the readout, replacing the
            /// fixed-decimal display: the readout shows the format's display
            /// text followed by its unit callback's text or, without one, the
            /// literal [`unit`](Self::unit). The format's width hint reserves
            /// the whole slot, so ordinary value changes keep the track length
            /// and the readout's position; formatted text ends at the slot's
            /// right edge. [`decimal_places`](Self::decimal_places) is ignored
            /// while a format is installed, and a range change no longer
            /// requests layout. This control never parses text, so the
            /// format's edit and parse callbacks are unused; formatting never
            /// quantizes the parameter. Describing the control again with an
            /// unchanged clone of the format retains its layout; a new format
            /// or changed metadata remeasures. The readout stays hidden until
            /// [`show_value`](Self::show_value) enables it.
            ///
            /// ```
            /// use pert_ui_runtime::*;
            /// let mut ui = Runtime::new();
            /// ui.set_viewport(Viewport::new(300.0, 100.0, 1.0)?)?;
            /// let (amount, _host) = ui.parameter(
            ///     ParameterId::new(1, 1, "amount")?,
            ///     ParameterSpec::new("Amount", 0.0, 1.0, 0.5)?,
            /// )?;
            /// // The percentage is presentation, not a library mapping.
            /// let percentage = NumberFormat::new(
            ///     "100.0 %",
            ///     |v: f32| format!("{:.1}", 100.0 * v),
            ///     |s| s.parse().map_err(|_| NumberInputError::InvalidNumber),
            /// )
            /// .unit(|_| "%".into());
            #[doc = concat!("let control = ", stringify!($name), "::new(amount).show_value(true).format(percentage);")]
            /// ui.mount_tree(Element::new("amount", control))?;
            /// ui.flush(std::time::Duration::ZERO)?;
            /// let readout = &ui.inspect().components[0].details.properties["readout"];
            /// assert_eq!(*readout, InspectValue::Text("50.0 %".into()));
            /// # Ok::<(), Box<dyn std::error::Error>>(())
            /// ```
            pub fn format(mut self, format: NumberFormat<f32>) -> Self {
                self.0.field.format = Some(format);
                self
            }
        }
    };
}
numeric_widget!(
    Slider,
    false,
    Shape::Linear,
    "A linear parameter slider, horizontal by default. Primary press sets a position; Shift-drag makes relative fine adjustments. The track and slim thumb use the button theme's corner radius; the thumb spans 65% of IconSize across its travel, within the full control hit area. An editable track shows SurfaceHover under the pointer without changing its size."
);
numeric_widget!(
    InlineSlider,
    true,
    Shape::Linear,
    "A one-line parameter row with a caption, a short track, an editable value and a unit, sharing one Tab stop. Inline controls fit toolbars, channel strips and status bars at ControlHeight and follow the ordinary theme metrics for density and zoom. The secondary caption uses `TextStyle::Caption`; the value and unit use `TextStyle::Monospace` on one baseline. Click the value or press Enter/F2 to type; horizontal value dragging begins after four logical pixels and uses 160 logical pixels for full travel, with Shift dividing sensitivity by ten. Track, key and wheel gestures keep the normal parameter edit protocol. Enter confirms typed values; Escape or loss of context cancels, and invalid text stays editable without changing the base. While typing, keys reach the value before application key commands, as in `TextField`, and `Runtime::text_command` acts on the typed text as the clipboard chords do; outside text mode the row is not a text editor. Resting focus schedules no caret work. The caption and unit are literal presentation text outside selection, clipboard text and IME composition. Editable track and value hover uses SurfaceHover without changing row geometry."
);
numeric_widget!(
    Knob,
    false,
    Shape::Rotary,
    "A rotary parameter control: the slider's edit protocol, keyboard and wheel behavior in a round dial.\n\nThe dial is [`MetricRole::KnobDiameter`] across by default, with a 270-degree track arc open at the bottom, a value arc in `ColorRole::Primary` from its [`ArcOrigin`] to the value, and a pointer line, all [`MetricRole::KnobStroke`] wide. A larger allocation centers the largest square that fits; the optional readout (`show_value`) sits centered below. A primary press on the dial begins one edit without changing the value; vertical dragging moves it, upward increasing, by the full range over [`MetricRole::KnobTravel`] logical pixels, and Shift divides the sensitivity by ten at any point in the drag. The matched release commits a drag that moved the value; a release without movement cancels the edit, so a click alone records no edit for the host. A second primary press within 500 ms of a press that began an edit and did not move the value is a double-click: it begins, previews and commits one edit to the parameter's default, as Backspace does. Hover shows `SurfaceHover` inside the arc, and the focus ring surrounds the dial.\n\n```\nuse pert_ui_runtime::{ArcOrigin, Element, KeyAction, KeyCode, Knob, ParameterId, ParameterSpec, Runtime, Viewport};\nlet mut ui = Runtime::new();\nui.set_viewport(Viewport::new(120.0, 120.0, 1.0)?)?;\nlet (balance, _host) = ui.parameter(\n    ParameterId::new(1, 1, \"balance\")?,\n    ParameterSpec::new(\"Balance\", -1.0, 1.0, 0.0)?,\n)?;\nlet knob = ui.mount_tree(Element::new(\"balance\", Knob::new(balance.clone()).arc_origin(ArcOrigin::Default).show_value(true)))?;\nui.flush(std::time::Duration::ZERO)?;\nassert!(ui.bounds(&knob)?.is_some_and(|dial| dial.width() > 0.0));\nui.focus(&knob)?;\nui.key_input(KeyCode::End, KeyAction::Press)?;\nui.key_input(KeyCode::End, KeyAction::Release)?;\nassert_eq!(balance.snapshot().get()?.base, 1.0);\nui.key_input(KeyCode::Backspace, KeyAction::Press)?;\nui.key_input(KeyCode::Backspace, KeyAction::Release)?;\nassert_eq!(balance.snapshot().get()?.base, 0.0);\n# Ok::<(), Box<dyn std::error::Error>>(())\n```"
);
readout_api!(Slider);
readout_api!(Knob);
impl Knob {
    /// Selects where the value arc starts, [`ArcOrigin::Minimum`] by default.
    /// Presentation only: the origin never changes the value, the mapping or
    /// the edit protocol, and changing it in place repaints without layout.
    pub fn arc_origin(mut self, origin: ArcOrigin) -> Self {
        self.0.arc_origin = origin;
        self
    }
}
impl Slider {
    /// Selects travel direction; vertical values increase upward. Preferred long
    /// extent is three themed ControlWidths, with one ControlHeight across it.
    ///
    /// The track spans the whole allocation the slider is given, such as the
    /// height of a row with [`Align::Stretch`](crate::Align::Stretch). A shown
    /// readout keeps its own extent below a vertical track (or right of a
    /// horizontal one); when the allocation cannot hold both, the readout is cut
    /// to what fits and the track takes the rest, down to nothing.
    pub fn orientation(mut self, orientation: Orientation) -> Self {
        self.0.orientation = orientation;
        self
    }
}

impl InlineSlider {
    /// Sets the row's leading caption, empty by default. Without a caption
    /// the row's accessible name is the parameter's name.
    ///
    /// The caption is a [`TextSource`]: fixed plain or rich text, or a
    /// binding the row follows. A binding is read in the row's update and
    /// remeasures the row when it changes; describing the row again with the
    /// same binding keeps the text already read.
    pub fn label(mut self, label: impl Into<TextSource>) -> Self {
        self.0.field.caption = Caption::new(label);
        self
    }
    /// Installs typed display/edit/parse callbacks with a width example that
    /// reserves value and unit together. Parsed values must be finite and within
    /// the current parameter domain; formatting never quantizes the parameter.
    pub fn format(mut self, format: NumberFormat<f32>) -> Self {
        self.0.field.format = Some(format);
        self
    }
}
