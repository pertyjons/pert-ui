//! Progress and activity presentation: how far a task has come, that it is
//! running when nobody knows how far, and how it ended.
//!
//! The application owns the task and publishes its status; this module only
//! presents it. Indeterminate activity is the one built-in presentation driven by
//! [`UpdateContext::animate`], so it is also that contract's standing test: it
//! must stop scheduling when hidden, when unmounted and when the task ends.
use crate::{
    ArrangeContext, ColorRole, Component, ComponentDetails, Constraints, DrawError, EventContext,
    InputOutcome, InspectValue, Invalidation, KeyEvent, KeyboardState, LayoutError, Length,
    MeasureContext, MetricRole, NormalizedValue, Painter, ReadSignal, Rect, RuntimeError,
    SemanticRole, SemanticValue, Semantics, Shape, Size, UpdateContext,
};
use std::time::Duration;

/// What an application reports about one task.
///
/// The default is [`ProgressStatus::Idle`]. The library never advances, times or
/// completes a task: every transition is a write to the application's binding.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[non_exhaustive]
pub enum ProgressStatus {
    /// Nothing is running and nothing has been reported; an empty track.
    #[default]
    Idle,
    /// Running, with no meaningful measure of how far. This is the only status
    /// that animates.
    Indeterminate,
    /// Running, this far along. Zero is an empty track and one a full one.
    Determinate(NormalizedValue),
    /// Finished successfully; a full track in the Success role.
    Succeeded,
    /// Ended in failure; a full track in the Error role. What went wrong is the
    /// application's message to present, for example through a `Field` or `Banner`.
    Failed,
    /// Stopped before finishing, by the user or the application; a full track in
    /// the Muted role.
    Cancelled,
}
impl ProgressStatus {
    /// The status for `completed` of `total` equal steps.
    ///
    /// A zero `total` has no measure and is [`ProgressStatus::Indeterminate`]. A
    /// `completed` above `total` is a full track rather than an error, because a
    /// count that overshoots an estimate is ordinary; it never means
    /// [`ProgressStatus::Succeeded`], which only the application can declare.
    ///
    /// ```
    /// # use pert_ui_runtime::{NormalizedValue, ProgressStatus};
    /// let half = NormalizedValue::try_from(0.5)?;
    /// assert_eq!(ProgressStatus::of(5, 10), ProgressStatus::Determinate(half));
    /// assert_eq!(ProgressStatus::of(12, 10), ProgressStatus::Determinate(NormalizedValue::MAX));
    /// assert_eq!(ProgressStatus::of(3, 0), ProgressStatus::Indeterminate);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    #[must_use]
    pub fn of(completed: u64, total: u64) -> Self {
        if total == 0 {
            return Self::Indeterminate;
        }
        // Both counts fit the quotient's precision needs: the result is a
        // presentation fraction, and the division of two u64 as f64 stays in 0..=1
        // after the minimum below.
        let fraction = (completed.min(total) as f64 / total as f64) as f32;
        NormalizedValue::try_from(fraction).map_or(Self::Indeterminate, Self::Determinate)
    }
    /// Whether the task is still running, determinate or not.
    #[must_use]
    pub const fn is_running(self) -> bool {
        matches!(self, Self::Indeterminate | Self::Determinate(_))
    }
    /// Whether the task has ended, whatever the outcome.
    #[must_use]
    pub const fn is_finished(self) -> bool {
        matches!(self, Self::Succeeded | Self::Failed | Self::Cancelled)
    }
    const fn name(self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::Indeterminate => "indeterminate",
            Self::Determinate(_) => "determinate",
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }
}

/// One sweep of the indeterminate segment across the track.
const SWEEP: Duration = Duration::from_millis(1500);
/// The indeterminate segment's share of the track width.
const SEGMENT: f32 = 0.3;

/// A horizontal bar presenting a [`ProgressStatus`] binding.
///
/// The track is [`MetricRole::ProgressHeight`] tall (6 logical pixels at
/// Standard) in the Outline role, three themed ControlWidths wide by preference,
/// and fills the width it is given. The fill uses Primary while running, Success,
/// Error or Muted once finished, and nothing while idle. Beside a partial fill
/// the track leaves a gap of half the bar's height unpainted (half the
/// remaining track where less than twice that remains), so the background
/// separates a fill from a track of similar color. It is not focusable and
/// handles no input.
///
/// A determinate value is drawn where it is; it does not ease toward a new value,
/// so a status write costs one repaint and schedules nothing. While
/// [`ProgressStatus::Indeterminate`], a segment sweeps the track once every 1.5
/// seconds, positioned from the runtime's elapsed clock, by requesting
/// [`UpdateContext::animate`] on each update. Nothing else schedules work: the
/// request is not repeated once the status changes, and the runtime withdraws the
/// deadline while the bar is hidden or after it is unmounted, so an ended or
/// invisible task leaves no periodic wake. The sweep moves plain rectangles, so
/// a frame rasterizes no coverage and adds nothing to the shape cache.
///
/// Under [`MotionPreference::Reduced`](crate::MotionPreference::Reduced) the
/// indeterminate segment rests in the middle of the track instead of sweeping,
/// and the bar schedules nothing in any status.
///
/// Inspection reports the [`SemanticRole::ProgressIndicator`] role, the fraction
/// as its value while determinate, the status name as its description and
/// `invalid` after a failure. A `Field` around the bar names it.
///
/// Re-describing the bar under the same key with another status binding reads
/// the new binding from the next update; the sweep keeps its place, because its
/// phase comes from the runtime clock rather than from the binding.
///
/// ```
/// use pert_ui_runtime::{Element, ProgressBar, ProgressStatus, Runtime, Viewport};
/// use std::time::Duration;
/// let mut ui = Runtime::new();
/// ui.set_viewport(Viewport::new(320.0, 80.0, 1.0)?)?;
/// let status = ui.signal(ProgressStatus::Indeterminate);
/// ui.mount_tree(Element::new("export", ProgressBar::new(status.read_only())))?;
/// ui.flush(Duration::ZERO)?;
/// assert!(ui.next_deadline().is_some(), "running activity animates");
///
/// status.set(ProgressStatus::of(3, 4))?;
/// ui.flush(Duration::from_millis(20))?;
/// assert_eq!(ui.next_deadline(), None, "a known fraction schedules nothing");
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[must_use]
pub struct ProgressBar {
    source: ReadSignal<ProgressStatus>,
    status: ProgressStatus,
    phase: f32,
    /// The motion preference was reduced at the last update.
    still: bool,
}
impl ProgressBar {
    /// Follows an application-owned status binding.
    pub fn new(status: ReadSignal<ProgressStatus>) -> Self {
        Self {
            source: status,
            status: ProgressStatus::Idle,
            phase: 0.0,
            still: false,
        }
    }
    // The filled part of a track of this width, as `(start, width, role)`.
    fn fill(&self, width: f32) -> Option<(f32, f32, ColorRole)> {
        match self.status {
            ProgressStatus::Idle => None,
            ProgressStatus::Indeterminate if self.still => {
                // Reduced motion: the same segment at rest in the middle, which
                // no determinate fraction can look like.
                let segment = width * SEGMENT;
                Some(((width - segment) / 2.0, segment, ColorRole::Primary))
            }
            ProgressStatus::Indeterminate => {
                let segment = width * SEGMENT;
                // The segment enters from the left edge and leaves past the right
                // one, clipped to the track, so the sweep has no visible jump.
                let start = self.phase * (width + segment) - segment;
                let end = (start + segment).min(width);
                let start = start.max(0.0);
                Some((start, (end - start).max(0.0), ColorRole::Primary))
            }
            ProgressStatus::Determinate(value) => {
                Some((0.0, width * value.get(), ColorRole::Primary))
            }
            ProgressStatus::Succeeded => Some((0.0, width, ColorRole::Success)),
            ProgressStatus::Failed => Some((0.0, width, ColorRole::Error)),
            ProgressStatus::Cancelled => Some((0.0, width, ColorRole::Muted)),
        }
    }
}
impl Component for ProgressBar {
    fn reconcile(&mut self, next: Self) -> Option<Invalidation> {
        // The status and phase are read again from the new binding at update.
        if next.source.same_source(&self.source) {
            return None;
        }
        self.source = next.source;
        Some(Invalidation::Paint)
    }
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        // Geometry never depends on the status, so a change only repaints.
        self.status = cx.read_paint(&self.source)?;
        self.still = cx.motion() == crate::MotionPreference::Reduced;
        if self.status == ProgressStatus::Indeterminate && !self.still {
            let sweep = SWEEP.as_nanos();
            self.phase = (cx.now().as_nanos() % sweep) as f32 / sweep as f32;
            cx.animate()?;
        }
        Ok(())
    }
    fn measure(
        &self,
        cx: &MeasureContext<'_>,
        available: Constraints,
    ) -> Result<Size, RuntimeError> {
        let preferred = 3.0 * cx.metric(MetricRole::ControlWidth);
        Ok(Size::new(
            available
                .width()
                .map_or(preferred, |width| width.min(preferred)),
            cx.metric(MetricRole::ProgressHeight),
        )?)
    }
    fn arrange(&self, cx: &mut ArrangeContext<'_>, _: Size) -> Result<(), RuntimeError> {
        if cx.is_empty() {
            Ok(())
        } else {
            Err(LayoutError::InvalidChild.into())
        }
    }
    fn paint(&self, painter: &mut Painter) -> Result<(), DrawError> {
        let Some(size) = painter.size() else {
            return Ok(());
        };
        // The bar keeps its themed thickness, centered, in a taller allocation.
        let height = painter
            .metric(MetricRole::ProgressHeight)
            .min(size.height());
        let top = (size.height() - height) / 2.0;
        let track = painter.color(ColorRole::Outline);
        let Some((start, width, role)) = self.fill(size.width()).filter(|fill| fill.1 > 0.0) else {
            painter.fill_rect(Rect::new(0.0, top, size.width(), height)?, track);
            return Ok(());
        };
        // The track stops short of the fill on either side, leaving the
        // background visible between them, so a fill whose color is close to
        // the track's is still told apart from it. Next to a remainder too
        // narrow for the whole gap, the gap takes half of it, so some track
        // shows wherever part of the range remains.
        let gap = height / 2.0;
        let end = start + width;
        let before = start - gap.min(start / 2.0);
        let rest = size.width() - end;
        let after = end + gap.min(rest / 2.0);
        for (from, to) in [(0.0, before), (after, size.width())] {
            if to > from {
                painter.fill_rect(Rect::new(from, top, to - from, height)?, track);
            }
        }
        painter.fill_rect(Rect::new(start, top, width, height)?, painter.color(role));
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
            "status".into(),
            InspectValue::Text(self.status.name().into()),
        );
        let semantics = Semantics::new(SemanticRole::ProgressIndicator)
            .description(self.status.name())
            .invalid(self.status == ProgressStatus::Failed);
        details.semantics = Some(match self.status {
            ProgressStatus::Determinate(value) => {
                details
                    .properties
                    .insert("fraction".into(), InspectValue::Number(value.get().into()));
                semantics.value(SemanticValue::Number {
                    value: value.get().into(),
                    minimum: Some(0.0),
                    maximum: Some(1.0),
                })
            }
            _ => semantics,
        });
        details
    }
}

/// How many dots the spinner's ring has, and therefore how many steps a turn.
pub(crate) const DOTS: u32 = 8;
/// How long the bright dot rests on each position.
pub(crate) const STEP: Duration = Duration::from_millis(100);
/// Opacity of every dot while the motion preference is reduced.
const STILL_ALPHA: f32 = 0.6;

/// A small animated mark saying that something is running, for places too small
/// for a [`ProgressBar`]: beside a label, in a toolbar, inside a button row.
///
/// It is an IconSize square holding a ring of eight dots in the Primary role. One
/// dot is bright and the ones behind it fade, and the bright one moves a position
/// every 100 milliseconds while the bound value is true. While it is false the
/// spinner draws nothing but keeps its size, so starting and stopping never
/// reflows the content beside it. It is not focusable and handles no input.
///
/// Dot opacity multiplies the configured Primary alpha, including reduced motion.
/// The dots never move: only their colors change. Every dot therefore reuses one
/// cached coverage mask, a frame rasterizes nothing, and the spinner wakes once
/// per step through [`UpdateContext::wake_after`], aligned to the step boundary of
/// the runtime's clock, instead of at the 60 Hz ceiling. The step is taken from
/// the clock, so a missed stretch produces one update. Nothing is scheduled while
/// the value is false, and the runtime withdraws the deadline while the spinner
/// is hidden or after it is unmounted.
///
/// Under [`MotionPreference::Reduced`](crate::MotionPreference::Reduced) a busy
/// spinner is a still ring of evenly lit dots and schedules nothing; resting, it
/// still draws nothing, so the ring alone says that something is running.
///
/// Inspection reports the [`SemanticRole::ProgressIndicator`] role with `busy` or
/// `idle` as its description. For a task with a known extent, or to show how a
/// task ended, use [`ProgressBar`].
///
/// Re-describing the spinner under the same key with another busy binding reads
/// the new binding from the next update; the lit dot keeps its step, because the
/// step comes from the runtime clock rather than from the binding.
///
/// ```
/// use pert_ui_runtime::{Element, Runtime, Spinner, Viewport};
/// use std::time::Duration;
/// let mut ui = Runtime::new();
/// ui.set_viewport(Viewport::new(100.0, 100.0, 1.0)?)?;
/// let busy = ui.signal(true);
/// ui.mount_tree(Element::new("saving", Spinner::new(busy.read_only())))?;
/// ui.flush(Duration::ZERO)?;
/// assert_eq!(ui.next_deadline(), Some(Duration::from_millis(100)));
///
/// busy.set(false)?;
/// ui.flush(Duration::from_millis(100))?;
/// assert_eq!(ui.next_deadline(), None, "an idle spinner schedules nothing");
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[must_use]
pub struct Spinner {
    source: ReadSignal<bool>,
    busy: bool,
    step: u32,
    /// The motion preference was reduced at the last update.
    still: bool,
}
impl Spinner {
    /// Follows an application-owned binding: true spins, false rests invisibly.
    pub fn new(busy: ReadSignal<bool>) -> Self {
        Self {
            source: busy,
            busy: false,
            step: 0,
            still: false,
        }
    }
}
impl Component for Spinner {
    fn reconcile(&mut self, next: Self) -> Option<Invalidation> {
        // Busy state and step are read again from the new binding at update.
        if next.source.same_source(&self.source) {
            return None;
        }
        self.source = next.source;
        Some(Invalidation::Paint)
    }
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        self.busy = cx.read_paint(&self.source)?;
        self.still = cx.motion() == crate::MotionPreference::Reduced;
        if self.busy && !self.still {
            let (now, step) = (cx.now().as_nanos(), STEP.as_nanos());
            self.step = ((now / step) % u128::from(DOTS)) as u32;
            // Wake at the next step boundary rather than a full step from now, so
            // a late update does not push every later step later.
            let remaining = step - now % step;
            cx.wake_after(Duration::from_nanos(remaining as u64))?;
        }
        Ok(())
    }
    fn measure(&self, cx: &MeasureContext<'_>, _: Constraints) -> Result<Size, RuntimeError> {
        let extent = cx.metric(MetricRole::IconSize);
        Ok(Size::new(extent, extent)?)
    }
    fn arrange(&self, cx: &mut ArrangeContext<'_>, _: Size) -> Result<(), RuntimeError> {
        if cx.is_empty() {
            Ok(())
        } else {
            Err(LayoutError::InvalidChild.into())
        }
    }
    fn paint(&self, painter: &mut Painter) -> Result<(), DrawError> {
        let Some(size) = painter.size() else {
            return Ok(());
        };
        if !self.busy {
            return Ok(());
        }
        paint_dots(painter, Rect::from_size(size), self.step, self.still)
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
        let state = if self.busy { "busy" } else { "idle" };
        let mut details = ComponentDetails::default();
        details
            .properties
            .insert("busy".into(), InspectValue::Bool(self.busy));
        details.semantics =
            Some(Semantics::new(SemanticRole::ProgressIndicator).description(state));
        details
    }
}

/// Draws the busy spinner's ring of dots in `area` at `step`, or the still
/// ring under reduced motion; shared with a pending [`crate::Field`].
pub(crate) fn paint_dots(
    painter: &mut Painter,
    area: Rect,
    step: u32,
    still: bool,
) -> Result<(), DrawError> {
    let [x, y, width, height] = area.values();
    let extent = width.min(height);
    let dot = extent / 5.0;
    let radius = (extent - dot) / 2.0;
    let (cx, cy) = (x + width / 2.0, y + height / 2.0);
    let primary = painter.color(ColorRole::Primary);
    let shape = Shape::Rounded(Length::new(dot / 2.0)?);
    for index in 0..DOTS {
        // Positions run clockwise from the top. `behind` counts how far a dot
        // trails the bright one, and the trail fades to a faint resting dot.
        let angle = std::f32::consts::TAU * index as f32 / DOTS as f32;
        let behind = (step + DOTS - index) % DOTS;
        let alpha = if still {
            STILL_ALPHA
        } else {
            (1.0 - behind as f32 / DOTS as f32).max(0.15)
        };
        painter.fill_shape(
            Rect::new(
                cx + radius * angle.sin() - dot / 2.0,
                cy - radius * angle.cos() - dot / 2.0,
                dot,
                dot,
            )?,
            shape,
            primary.scale_alpha(alpha)?,
        )?;
    }
    Ok(())
}
