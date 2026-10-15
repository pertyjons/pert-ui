//! Reusable presentations of a live series: a level and a trace.
//!
//! Both read a [`SeriesSource`] of `f32` values and place them on the scale of a
//! [`ParameterSpec`], the library's general description of a named, bounded
//! quantity with a linear or logarithmic mapping. Neither knows a unit or what
//! the values mean. Reading the source in `update` is what subscribes a
//! component, so a hidden, paused or unmounted widget drains nothing.
use crate::{
    ArrangeContext, ColorRole, Component, ComponentDetails, Constraints, DrawError, EventContext,
    InputOutcome, InspectValue, Invalidation, KeyEvent, KeyboardState, LayoutError, MeasureContext,
    MetricRole, NormalizedValue, Painter, ParameterSpec, ReadSignal, Rect, RuntimeError,
    SemanticRole, SemanticValue, Semantics, SeriesSource, Size, UpdateContext,
};
use std::time::Duration;

/// Which way a [`LevelMeter`] grows.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum MeterAxis {
    /// From the left edge toward the right. The default.
    #[default]
    Horizontal,
    /// From the bottom edge upward.
    Vertical,
}

/// Position of a sample on a scale, or `None` for a value that is not a number.
/// Finite values outside the scale rest at its nearest end: a display limit, the
/// one place this library clamps.
fn position(scale: &ParameterSpec<f32>, value: f32) -> Option<f32> {
    if !value.is_finite() {
        return None;
    }
    let clamped = value.clamp(scale.minimum(), scale.maximum());
    scale.normalize(&clamped).ok().map(NormalizedValue::get)
}

/// Whether a bound "running" flag allows reading; absent means always.
fn is_running(
    cx: &mut UpdateContext,
    running: Option<&ReadSignal<bool>>,
) -> Result<bool, RuntimeError> {
    running.map_or(Ok(true), |running| cx.read_paint(running))
}

fn leaf(cx: &ArrangeContext<'_>) -> Result<(), RuntimeError> {
    if cx.is_empty() {
        Ok(())
    } else {
        Err(LayoutError::InvalidChild.into())
    }
}

/// A bar showing the peak of the values a live series delivered since the
/// previous update.
///
/// The scale is a [`ParameterSpec`]: its bounds are the ends of the bar, its
/// mapping places values between them, linearly or logarithmically, and its name
/// is the meter's accessible name. Until the source has delivered anything, and
/// after its endpoint is dropped, the bar rests at the scale's default value in
/// the Muted role. A connected source that delivered nothing new, which reads
/// as [`Freshness::Stale`](crate::Freshness::Stale) after every such drain,
/// keeps the held level in the meter's normal role: staleness carries no
/// threshold, so muting on it would flicker between a producer's bursts. An
/// application that wants to show a quiet producer reads the source's
/// [`Telemetry`](crate::Telemetry) age and binds its own indication.
/// Finite values beyond the scale rest at its nearest end;
/// non-finite values are ignored. No unit is assumed: an application may use
/// temperature bounds when its producer publishes temperatures.
///
/// Each update folds the items that arrived since the previous one into their
/// maximum, so a short peak between two frames is shown, and holds the previous
/// level when nothing arrived. Peak-over-arrivals is this widget's policy, not
/// the source's: the window it reads is unchanged for every other reader.
///
/// The meter folds only what the source's window retains. The peak guarantee
/// therefore needs a bridge whose ring capacity is at most its window
/// capacity, which is what [`SeriesConfig::new`](crate::SeriesConfig::new)
/// builds. A ring enlarged with
/// [`SeriesConfig::with_ring_capacity`](crate::SeriesConfig::with_ring_capacity)
/// lets one drain deliver more items than the window keeps; the older ones are
/// evicted before the meter reads, and a peak among them is not shown.
///
/// [`peak_hold`](Self::peak_hold) adds a marker at the highest level of the last
/// hold period. The marker is released by one [`UpdateContext::wake_after`]
/// deadline at the end of its hold, which is the only work the meter ever
/// schedules itself; the drains that feed it follow the source's own schedule.
/// While [`running_signal`](Self::running_signal) is bound and false the meter does not read
/// the source, so it is not subscribed and nothing is drained on its behalf.
///
/// The bar is [`MetricRole::MeterThickness`] across its axis, three themed
/// ControlWidths long by preference along it, and fills the length it is given.
/// Track in Outline, level in Primary unless [`color`](Self::color) says
/// otherwise, marker in OnSurface. A partial level leaves an unpainted gap of
/// half the bar's thickness before the remaining track; the gap shrinks to
/// half the remainder near full scale, so some track stays visible. It is not
/// focusable and handles no input.
/// Inspection reports the [`SemanticRole::Readout`] role with the level as a
/// number on the scale.
///
/// ```
/// use pert_ui_runtime::{
///     Capacity, Delivery, Element, LevelMeter, ParameterSpec, Runtime, SeriesConfig,
///     Timestamp, Viewport, series_bridge,
/// };
/// use std::time::Duration;
/// let mut ui = Runtime::new();
/// ui.set_viewport(Viewport::new(320.0, 80.0, 1.0)?)?;
/// let (mut endpoint, consumer) = series_bridge::<f32>(SeriesConfig::new(Capacity::new(64)?));
/// let source = ui.series_source(consumer, Delivery::Polled);
/// let scale = ParameterSpec::new("Level", 0.0, 1.0, 0.0)?;
/// ui.mount_tree(Element::new("meter", LevelMeter::new(source, scale)))?;
/// // The first update subscribes the meter; the runtime then activates the source.
/// while ui.next_deadline() == Some(Duration::ZERO) {
///     ui.flush(Duration::ZERO)?;
/// }
///
/// // Two items between frames: the meter shows the higher one.
/// endpoint.publish(0.75, Timestamp::new(1));
/// endpoint.publish(0.25, Timestamp::new(2));
/// let next = ui.next_deadline().expect("a visible reader keeps the source polled");
/// ui.flush(next)?;
/// let level = ui.inspect().components[0].details.properties["level"].clone();
/// assert_eq!(level, pert_ui_runtime::InspectValue::Number(0.75));
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
///
/// Re-describing the meter under the same key applies its scale, axis, color,
/// running binding and peak hold; a different source starts again from no
/// level, and removing the hold clears its marker.
#[must_use]
pub struct LevelMeter {
    source: SeriesSource<f32>,
    scale: ParameterSpec<f32>,
    axis: MeterAxis,
    color: ColorRole,
    running: Option<ReadSignal<bool>>,
    hold: Option<Duration>,
    /// Latest folded value on the scale, absent until something was delivered.
    level: Option<f32>,
    /// Held peak value and the runtime time it is released.
    peak: Option<(f32, Duration)>,
    /// Newest sequence already folded, so a retained window is not folded twice.
    folded_through: Option<u64>,
    available: bool,
}
impl LevelMeter {
    /// Shows `source` on `scale`, horizontally, in the Primary role.
    pub fn new(source: SeriesSource<f32>, scale: ParameterSpec<f32>) -> Self {
        Self {
            source,
            scale,
            axis: MeterAxis::Horizontal,
            color: ColorRole::Primary,
            running: None,
            hold: None,
            level: None,
            peak: None,
            folded_through: None,
            available: false,
        }
    }
    /// Chooses the direction the bar grows in.
    pub fn axis(mut self, axis: MeterAxis) -> Self {
        self.axis = axis;
        self
    }
    /// Chooses the role the level is drawn in. The track and marker keep theirs.
    pub fn color(mut self, role: ColorRole) -> Self {
        self.color = role;
        self
    }
    /// Reads the source only while `running` is true. While false the meter keeps
    /// what it shows, is not subscribed to the source and schedules nothing.
    pub fn running_signal(mut self, running: ReadSignal<bool>) -> Self {
        self.running = Some(running);
        self
    }
    /// Marks the highest level of the last `hold`; a zero duration adds no marker.
    pub fn peak_hold(mut self, hold: Duration) -> Self {
        self.hold = (!hold.is_zero()).then_some(hold);
        self
    }
    fn shown(&self) -> f32 {
        self.level
            .filter(|_| self.available)
            .unwrap_or_else(|| self.scale.default_value())
    }
    /// The rectangle from the scale's start to `position`, `thickness` across.
    fn bar(&self, size: Size, position: f32, thickness: f32) -> Result<Rect, crate::GeometryError> {
        match self.axis {
            MeterAxis::Horizontal => Rect::new(
                0.0,
                (size.height() - thickness) / 2.0,
                size.width() * position,
                thickness,
            ),
            MeterAxis::Vertical => {
                let length = size.height() * position;
                Rect::new(
                    (size.width() - thickness) / 2.0,
                    size.height() - length,
                    thickness,
                    length,
                )
            }
        }
    }
}
impl Component for LevelMeter {
    /// Applies every property of the new description. A new source starts the
    /// meter over, as a newly mounted one: its level, peak and folding
    /// position belong to the old source. A removed peak hold drops the marker.
    fn reconcile(&mut self, next: Self) -> Option<Invalidation> {
        let mut invalidation = None;
        let mut widen = |needed: Invalidation| {
            invalidation = invalidation.max(Some(needed));
        };
        if !next.source.same_source(&self.source) {
            self.source = next.source;
            self.level = None;
            self.peak = None;
            self.folded_through = None;
            self.available = false;
            widen(Invalidation::Paint);
        }
        if next.scale != self.scale {
            self.scale = next.scale;
            widen(Invalidation::Paint);
        }
        if next.axis != self.axis {
            self.axis = next.axis;
            widen(Invalidation::Layout);
        }
        if next.color != self.color {
            self.color = next.color;
            widen(Invalidation::Paint);
        }
        if !ReadSignal::same_optional(&next.running, &self.running) {
            self.running = next.running;
            widen(Invalidation::Paint);
        }
        if next.hold != self.hold {
            self.hold = next.hold;
            if self.hold.is_none() {
                self.peak = None;
            }
            widen(Invalidation::Paint);
        }
        invalidation
    }
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        if !is_running(cx, self.running.as_ref())? {
            return Ok(());
        }
        let folded_through = self.folded_through;
        let (peak, available) = cx.read_series(&self.source, |telemetry, window| {
            let mut peak: Option<f32> = None;
            let mut newest = None;
            for entry in window.iter() {
                let sequence = entry.sample.sequence.get();
                if folded_through.is_some_and(|folded| sequence <= folded) {
                    continue;
                }
                newest = Some(sequence);
                let value = entry.sample.value;
                if value.is_finite() {
                    peak = Some(peak.map_or(value, |high| high.max(value)));
                }
            }
            (
                newest.map(|newest| (peak, newest)),
                telemetry.latest.is_some() && !telemetry.disconnected,
            )
        })?;
        self.available = available;
        if let Some((peak, newest)) = peak {
            self.folded_through = Some(newest);
            if peak.is_some() {
                self.level = peak;
            }
        }
        let now = cx.now();
        if let Some(hold) = self.hold {
            let level = self.level.filter(|_| self.available);
            let expired = self.peak.is_none_or(|(_, release)| release <= now);
            let exceeded = level
                .zip(self.peak)
                .is_some_and(|(level, (held, _))| level >= held);
            if expired || exceeded {
                self.peak = level
                    .map(|level| now.checked_add(hold).map(|release| (level, release)))
                    .map(|peak| peak.ok_or(RuntimeError::Exhausted))
                    .transpose()?;
            }
            // A marker above the level has to come down even if the data stops.
            if let Some((held, release)) = self.peak
                && level.is_some_and(|level| held > level)
            {
                cx.wake_after(release.saturating_sub(now).max(Duration::from_nanos(1)))?;
            }
        }
        Ok(())
    }
    fn measure(
        &self,
        cx: &MeasureContext<'_>,
        available: Constraints,
    ) -> Result<Size, RuntimeError> {
        let length = 3.0 * cx.metric(MetricRole::ControlWidth);
        let thickness = cx.metric(MetricRole::MeterThickness);
        let fit = |limit: Option<f32>| limit.map_or(length, |limit| limit.min(length));
        Ok(match self.axis {
            MeterAxis::Horizontal => Size::new(fit(available.width()), thickness)?,
            MeterAxis::Vertical => Size::new(thickness, fit(available.height()))?,
        })
    }
    fn arrange(&self, cx: &mut ArrangeContext<'_>, _: Size) -> Result<(), RuntimeError> {
        leaf(cx)
    }
    fn paint(&self, painter: &mut Painter) -> Result<(), DrawError> {
        let Some(size) = painter.size() else {
            return Ok(());
        };
        let across = match self.axis {
            MeterAxis::Horizontal => size.height(),
            MeterAxis::Vertical => size.width(),
        };
        let thickness = painter.metric(MetricRole::MeterThickness).min(across);
        let level = position(&self.scale, self.shown()).unwrap_or(0.0);
        let length = match self.axis {
            MeterAxis::Horizontal => size.width(),
            MeterAxis::Vertical => size.height(),
        };
        let filled = length * level;
        let remainder = length - filled;
        let gap = if filled > 0.0 {
            (thickness / 2.0).min(remainder / 2.0)
        } else {
            0.0
        };
        if remainder > gap {
            let full = self.bar(size, 1.0, thickness)?;
            let track = match self.axis {
                MeterAxis::Horizontal => Rect::new(
                    full.x() + filled + gap,
                    full.y(),
                    remainder - gap,
                    thickness,
                )?,
                MeterAxis::Vertical => Rect::new(full.x(), full.y(), thickness, remainder - gap)?,
            };
            painter.fill_rect(track, painter.color(ColorRole::Outline));
        }
        let role = if self.available {
            self.color
        } else {
            ColorRole::Muted
        };
        let bar = self.bar(size, level, thickness)?;
        if !bar.is_empty() {
            painter.fill_rect(bar, painter.color(role));
        }
        if let Some(held) = self
            .peak
            .filter(|_| self.available)
            .and_then(|(held, _)| position(&self.scale, held))
        {
            let stroke = painter.metric(MetricRole::BorderWidth);
            let full = self.bar(size, held, thickness)?;
            let marker = match self.axis {
                MeterAxis::Horizontal => Rect::new(
                    (full.width() - stroke).max(0.0),
                    full.y(),
                    stroke.min(size.width()),
                    thickness,
                )?,
                MeterAxis::Vertical => {
                    Rect::new(full.x(), full.y(), thickness, stroke.min(size.height()))?
                }
            };
            painter.fill_rect(marker, painter.color(ColorRole::OnSurface));
        }
        Ok(())
    }
    fn keyboard_event(
        &mut self,
        _: &mut EventContext,
        _: KeyEvent,
        _: &KeyboardState,
    ) -> Result<InputOutcome, RuntimeError> {
        Ok(InputOutcome::Ignored)
    }
    fn inspect(&self) -> ComponentDetails {
        let mut details = ComponentDetails::default();
        let shown = self.shown();
        details
            .properties
            .insert("level".into(), InspectValue::Number(shown.into()));
        details
            .properties
            .insert("available".into(), InspectValue::Bool(self.available));
        if let Some((held, _)) = self.peak {
            details
                .properties
                .insert("peak".into(), InspectValue::Number(held.into()));
        }
        details.semantics = Some(
            Semantics::new(SemanticRole::Readout)
                .name(self.scale.name())
                .value(SemanticValue::Number {
                    value: shown.into(),
                    minimum: Some(self.scale.minimum().into()),
                    maximum: Some(self.scale.maximum().into()),
                }),
        );
        details
    }
}

/// The retained window of a live series drawn as a line over time, oldest at the
/// left and newest at the right.
///
/// The vertical scale is a [`ParameterSpec`], as for [`LevelMeter`]: its maximum
/// is the top edge, its minimum the bottom, its mapping places values between
/// them and its name is the accessible name. Finite values beyond the scale rest
/// at its nearest edge. A non-finite value is not drawn and breaks the line, as
/// does a discontinuity the source reports, so a gap in the data is a gap on
/// screen rather than an invented segment. How much time the trace spans is the
/// source's retained capacity; the trace keeps no history of its own.
///
/// Each physical pixel column draws the span between the lowest and highest
/// value that falls in it, joined to the previous column's last value, so a
/// window far denser than the screen keeps its peaks and costs one rectangle per
/// column, at most one per retained item. Drawing is plain rectangles: a new
/// window rasterizes no coverage and adds nothing to the shape cache. The trace
/// copies the window's values into a buffer it reuses, so a steady stream
/// allocates nothing after the first full window.
///
/// The trace draws only the line, in Primary unless [`color`](Self::color) says
/// otherwise, and Muted once the source's endpoint is dropped; a stale but
/// connected source keeps its normal role, as for [`LevelMeter`]. Put a background
/// or a grid in a sibling beneath it: a separately retained sibling is not
/// repainted when the data changes. While [`running_signal`](Self::running_signal) is bound and
/// false the trace does not read the source, so it is not subscribed and nothing
/// is drained on its behalf. The trace schedules no work of its own.
///
/// It prefers three themed ControlWidths by eight
/// [`MetricRole::MeterThickness`] and fills the width it is given. It is not
/// focusable and handles no input. Inspection reports the
/// [`SemanticRole::Image`] role and the number of retained points.
///
/// ```
/// use pert_ui_runtime::{
///     Capacity, Delivery, Element, ParameterSpec, Runtime, SeriesConfig, Timestamp, Trace,
///     Viewport, series_bridge,
/// };
/// use std::time::Duration;
/// let mut ui = Runtime::new();
/// ui.set_viewport(Viewport::new(320.0, 200.0, 1.0)?)?;
/// let (mut endpoint, consumer) = series_bridge::<f32>(SeriesConfig::new(Capacity::new(256)?));
/// let source = ui.series_source(consumer, Delivery::Polled);
/// let scale = ParameterSpec::new("Signal", -1.0, 1.0, 0.0)?;
/// ui.mount_tree(Element::new("trace", Trace::new(source, scale)))?;
/// // The first update subscribes the trace; the runtime then activates the source.
/// while ui.next_deadline() == Some(Duration::ZERO) {
///     ui.flush(Duration::ZERO)?;
/// }
/// for tick in 0..100_u64 {
///     endpoint.publish((tick as f32 * 0.1).sin(), Timestamp::new(tick));
/// }
/// let next = ui.next_deadline().expect("a visible reader keeps the source polled");
/// ui.flush(next)?;
/// assert!(!ui.paint()?.commands().is_empty());
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
///
/// Re-describing the trace under the same key applies its scale, color and
/// running binding; a different source starts again with no points.
#[must_use]
pub struct Trace {
    source: SeriesSource<f32>,
    scale: ParameterSpec<f32>,
    color: ColorRole,
    running: Option<ReadSignal<bool>>,
    /// Retained value and whether a break precedes it; placed on the scale at
    /// paint, so a new scale needs no new read. Reused between updates.
    points: Vec<(f32, bool)>,
    connected: bool,
}
impl Trace {
    /// Draws `source` on `scale` in the Primary role.
    pub fn new(source: SeriesSource<f32>, scale: ParameterSpec<f32>) -> Self {
        Self {
            source,
            scale,
            color: ColorRole::Primary,
            running: None,
            points: Vec::new(),
            connected: true,
        }
    }
    /// Chooses the role the line is drawn in.
    pub fn color(mut self, role: ColorRole) -> Self {
        self.color = role;
        self
    }
    /// Reads the source only while `running` is true. While false the trace keeps
    /// what it shows, is not subscribed to the source and schedules nothing.
    pub fn running_signal(mut self, running: ReadSignal<bool>) -> Self {
        self.running = Some(running);
        self
    }
}
impl Component for Trace {
    /// Applies every property of the new description. A new source clears the
    /// drawn points until the next update reads it.
    fn reconcile(&mut self, next: Self) -> Option<Invalidation> {
        let mut invalidation = None;
        if !next.source.same_source(&self.source) {
            self.source = next.source;
            self.points.clear();
            self.connected = true;
            invalidation = Some(Invalidation::Paint);
        }
        if next.scale != self.scale {
            self.scale = next.scale;
            invalidation = Some(Invalidation::Paint);
        }
        if next.color != self.color {
            self.color = next.color;
            invalidation = Some(Invalidation::Paint);
        }
        if !ReadSignal::same_optional(&next.running, &self.running) {
            self.running = next.running;
            invalidation = Some(Invalidation::Paint);
        }
        invalidation
    }
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        if !is_running(cx, self.running.as_ref())? {
            return Ok(());
        }
        let points = &mut self.points;
        self.connected = cx.read_series(&self.source, |telemetry, window| {
            points.clear();
            points.extend(
                window
                    .iter()
                    .map(|entry| (entry.sample.value, entry.discontinuity_before)),
            );
            !telemetry.disconnected
        })?;
        Ok(())
    }
    fn measure(
        &self,
        cx: &MeasureContext<'_>,
        available: Constraints,
    ) -> Result<Size, RuntimeError> {
        let width = 3.0 * cx.metric(MetricRole::ControlWidth);
        let height = 8.0 * cx.metric(MetricRole::MeterThickness);
        Ok(Size::new(
            available.width().map_or(width, |limit| limit.min(width)),
            available.height().map_or(height, |limit| limit.min(height)),
        )?)
    }
    fn arrange(&self, cx: &mut ArrangeContext<'_>, _: Size) -> Result<(), RuntimeError> {
        leaf(cx)
    }
    fn paint(&self, painter: &mut Painter) -> Result<(), DrawError> {
        let Some(size) = painter.size() else {
            return Ok(());
        };
        let count = self.points.len();
        if count == 0 || size.width() <= 0.0 || size.height() <= 0.0 {
            return Ok(());
        }
        let scale = painter.viewport().scale();
        let pixel = (1.0 / scale) as f32;
        let columns = ((f64::from(size.width()) * scale).ceil() as usize).clamp(1, count);
        let width = size.width() / columns as f32;
        let color = painter.color(if self.connected {
            self.color
        } else {
            ColorRole::Muted
        });
        for column in 0..columns {
            let first = column * count / columns;
            let end = (column + 1) * count / columns;
            // Join to the previous column's last point unless a break or a
            // value that is not a number separates them. Breaks inside one
            // column are narrower than a pixel and are not drawn.
            let joined = first > 0
                && !self.points[first].1
                && position(&self.scale, self.points[first].0).is_some();
            let from = if joined { first - 1 } else { first };
            let span = self.points[from..end]
                .iter()
                .filter_map(|&(value, _)| position(&self.scale, value))
                .fold(None, |span: Option<(f32, f32)>, point| {
                    Some(span.map_or((point, point), |(low, high)| {
                        (low.min(point), high.max(point))
                    }))
                });
            let Some((low, high)) = span else {
                continue;
            };
            let top = (1.0 - high) * size.height();
            let bottom = (1.0 - low) * size.height();
            let top = top.min(size.height() - pixel).max(0.0);
            painter.fill_rect(
                Rect::new(
                    column as f32 * width,
                    top,
                    width,
                    (bottom - top).max(pixel).min(size.height() - top),
                )?,
                color,
            );
        }
        Ok(())
    }
    fn keyboard_event(
        &mut self,
        _: &mut EventContext,
        _: KeyEvent,
        _: &KeyboardState,
    ) -> Result<InputOutcome, RuntimeError> {
        Ok(InputOutcome::Ignored)
    }
    fn inspect(&self) -> ComponentDetails {
        let mut details = ComponentDetails::default();
        details.properties.insert(
            "points".into(),
            InspectValue::Number(self.points.len() as f64),
        );
        details
            .properties
            .insert("connected".into(), InspectValue::Bool(self.connected));
        details.semantics = Some(Semantics::new(SemanticRole::Image).name(self.scale.name()));
        details
    }
}
