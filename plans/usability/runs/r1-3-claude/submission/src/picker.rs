//! `PointPicker`: a reusable two-dimensional point control.
//!
//! The picker edits two bound `Signal<f64>`s, `x` and `y`, each in 0.0..=1.0.
//! `x` grows to the right and `y` grows upwards, as on an XY pad.
//!
//! - A primary press inside the square moves the handle there and starts a
//!   drag; the drag captures the pointer, so it continues outside the square,
//!   clamped to its edges, until the matching release. Each change during the
//!   drag is written to the bindings at once, so other controls bound to them
//!   follow live.
//! - Escape, focus loss, hiding or any other loss of input context cancels a
//!   drag and writes back the point from before the press.
//! - When focused, the arrow keys move the point by [`STEP`], or [`STEP_LARGE`]
//!   with Shift.
//! - [`PointPicker::on_change`] reports every user change, including the
//!   restore of a cancelled drag; the owner sets the value by writing the
//!   bindings.
//! - Inspection reports a `Slider` whose value is text (`"x 0.50, y 0.50"`) and
//!   whose `x` and `y` properties are numbers.

use pert_ui::{
    ColorRole, Component, ComponentDetails, Constraints, DrawError, EventContext,
    InputOutcome, InspectValue, Invalidation, KeyCode, KeyEvent, KeyTransition, KeyboardState,
    Length, MeasureContext, MetricRole, Notifier, Painter, Point, PointerButton, PointerEvent,
    PointerKind, Rect, RuntimeError, SemanticAction, SemanticRole, SemanticValue, Semantics, Shape,
    Signal, Size, UpdateContext,
};

/// Arrow-key step.
pub const STEP: f64 = 0.01;
/// Arrow-key step with Shift held.
pub const STEP_LARGE: f64 = 0.1;

type ChangeHandler = Box<dyn FnMut(&mut EventContext, f64, f64) -> Result<(), RuntimeError>>;

/// A square area with a crosshair and a handle at the bound point.
pub struct PointPicker {
    x: Signal<f64>,
    y: Signal<f64>,
    name: String,
    on_change: Option<ChangeHandler>,
    /// The point shown, as last read or written.
    value: (f64, f64),
    /// The point before the press of the running drag.
    drag: Option<(f64, f64)>,
    /// A cancelled drag's point, written back in `deliver_notifications`.
    restore: Option<(f64, f64)>,
    notifier: Option<Notifier>,
}

impl PointPicker {
    /// Creates a picker editing `x` and `y`.
    pub fn new(x: Signal<f64>, y: Signal<f64>) -> Self {
        Self {
            x,
            y,
            name: "Point picker".to_owned(),
            on_change: None,
            value: (0.5, 0.5),
            drag: None,
            restore: None,
            notifier: None,
        }
    }

    /// Names the picker for inspection.
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// Reports each user change of the point, after the bindings hold it.
    pub fn on_change(
        mut self,
        handler: impl FnMut(&mut EventContext, f64, f64) -> Result<(), RuntimeError> + 'static,
    ) -> Self {
        self.on_change = Some(Box::new(handler));
        self
    }

    /// Writes a user change to the bindings and reports it.
    fn commit(&mut self, cx: &mut EventContext, point: (f64, f64)) -> Result<(), RuntimeError> {
        if point == self.value {
            return Ok(());
        }
        self.value = point;
        self.x.set(point.0)?;
        self.y.set(point.1)?;
        cx.invalidate(Invalidation::Paint);
        if let Some(handler) = self.on_change.as_mut() {
            handler(cx, point.0, point.1)?;
        }
        Ok(())
    }

    /// The square the picker draws in, at the top left of its allocation.
    fn square(size: Size) -> f32 {
        size.width().min(size.height())
    }

    /// Maps a local pointer position to a clamped point.
    fn point_at(side: f32, local: Point) -> (f64, f64) {
        if side <= 0.0 {
            return (0.5, 0.5);
        }
        let x = (local.x() / side).clamp(0.0, 1.0) as f64;
        let y = (1.0 - local.y() / side).clamp(0.0, 1.0) as f64;
        (x, y)
    }

    fn cancel_drag(&mut self, cx: &mut EventContext) -> bool {
        let Some(before) = self.drag.take() else {
            return false;
        };
        cx.release_pointer();
        if before != self.value {
            self.restore = Some(before);
            if let Some(notifier) = &self.notifier {
                notifier.request();
            }
        }
        cx.invalidate(Invalidation::Paint);
        true
    }
}

fn clamp01(value: f64) -> f64 {
    if value.is_nan() { 0.0 } else { value.clamp(0.0, 1.0) }
}

/// Rounds a stepped value so that repeated steps land on exact hundredths.
fn round_step(value: f64) -> f64 {
    (clamp01(value) * 10_000.0).round() / 10_000.0
}

impl Component for PointPicker {
    fn reconcile(&mut self, next: Self) -> Option<Invalidation> {
        let rebound = !self.x.read_only().same_source(&next.x.read_only())
            || !self.y.read_only().same_source(&next.y.read_only());
        if rebound {
            // A drag belongs to the bindings it started on; the next update
            // shows the new bindings' point.
            self.drag = None;
            self.restore = None;
        }
        self.x = next.x;
        self.y = next.y;
        self.name = next.name;
        self.on_change = next.on_change;
        Some(Invalidation::Paint)
    }

    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        let x = cx.read_paint(&self.x.read_only())?;
        let y = cx.read_paint(&self.y.read_only())?;
        self.value = (clamp01(x), clamp01(y));
        if self.notifier.is_none() {
            self.notifier = Some(cx.notifier());
        }
        Ok(())
    }

    fn measure(&self, cx: &MeasureContext<'_>, available: Constraints) -> Result<Size, RuntimeError> {
        let mut side = cx.metric(MetricRole::ColorAreaSize);
        if let Some(width) = available.width() {
            side = side.min(width);
        }
        Ok(Size::new(side, side)?)
    }

    fn focusable(&self) -> bool {
        true
    }

    fn pointer_input(
        &mut self,
        cx: &mut EventContext,
        event: PointerEvent,
    ) -> Result<InputOutcome, RuntimeError> {
        let (Some(local), Some(size)) = (cx.to_local(event.position()), cx.size()) else {
            return Ok(InputOutcome::Ignored);
        };
        let side = Self::square(size);
        match event.kind() {
            PointerKind::Press(PointerButton::Primary) if self.drag.is_none() => {
                let inside = (0.0..=side).contains(&local.x()) && (0.0..=side).contains(&local.y());
                if !inside || event.is_outside() {
                    return Ok(InputOutcome::Ignored);
                }
                cx.capture_pointer()?;
                cx.request_focus();
                self.drag = Some(self.value);
                self.commit(cx, Self::point_at(side, local))?;
            }
            PointerKind::Move => {
                if self.drag.is_none() {
                    return Ok(InputOutcome::Ignored);
                }
                // A refreshed move at the same position derives the same
                // point, and `commit` then does nothing: the drag converges.
                self.commit(cx, Self::point_at(side, local))?;
            }
            PointerKind::Release(PointerButton::Primary) => {
                if self.drag.take().is_none() {
                    return Ok(InputOutcome::Ignored);
                }
                cx.release_pointer();
                cx.invalidate(Invalidation::Paint);
            }
            _ => return Ok(InputOutcome::Ignored),
        }
        Ok(InputOutcome::Handled)
    }

    fn keyboard_event(
        &mut self,
        cx: &mut EventContext,
        event: KeyEvent,
        keyboard: &KeyboardState,
    ) -> Result<InputOutcome, RuntimeError> {
        if !matches!(event.transition, KeyTransition::Pressed | KeyTransition::Repeated) {
            return Ok(InputOutcome::Ignored);
        }
        let key = keyboard.control_key(event);
        if key == KeyCode::Escape {
            if self.cancel_drag(cx) {
                return Ok(InputOutcome::Handled);
            }
            return Ok(InputOutcome::Ignored);
        }
        let modifiers = keyboard.chord_modifiers(event);
        if modifiers.control || modifiers.alt || modifiers.super_key {
            return Ok(InputOutcome::Ignored);
        }
        let step = if modifiers.shift { STEP_LARGE } else { STEP };
        let (dx, dy) = match key {
            KeyCode::ArrowLeft => (-step, 0.0),
            KeyCode::ArrowRight => (step, 0.0),
            KeyCode::ArrowUp => (0.0, step),
            KeyCode::ArrowDown => (0.0, -step),
            _ => return Ok(InputOutcome::Ignored),
        };
        if self.drag.is_some() {
            // Keys do not move the point while a pointer drag runs.
            return Ok(InputOutcome::Handled);
        }
        let (x, y) = self.value;
        self.commit(cx, (round_step(x + dx), round_step(y + dy)))?;
        Ok(InputOutcome::Handled)
    }

    fn cancel_input(&mut self, cx: &mut EventContext) {
        self.cancel_drag(cx);
        self.input_reset();
        cx.invalidate(Invalidation::Paint);
    }

    fn deliver_notifications(&mut self, cx: &mut EventContext) -> Result<(), RuntimeError> {
        if let Some(before) = self.restore.take() {
            self.commit(cx, before)?;
        }
        Ok(())
    }

    fn inspect(&self) -> ComponentDetails {
        let (x, y) = self.value;
        let mut details = ComponentDetails {
            semantics: Some(
                Semantics::new(SemanticRole::Slider)
                    .name(self.name.clone())
                    .value(SemanticValue::Text(format!("x {x:.2}, y {y:.2}")))
                    .actions([
                        SemanticAction::SetValue,
                        SemanticAction::Increment,
                        SemanticAction::Decrement,
                    ]),
            ),
            ..ComponentDetails::default()
        };
        details.validation.interactive = true;
        details.properties.insert("x".to_owned(), InspectValue::Number(x));
        details.properties.insert("y".to_owned(), InspectValue::Number(y));
        details
            .properties
            .insert("dragging".to_owned(), InspectValue::Bool(self.drag.is_some()));
        details
    }

    fn paint(&self, painter: &mut Painter) -> Result<(), DrawError> {
        let Some(size) = painter.size() else {
            return Ok(());
        };
        let side = Self::square(size);
        if side <= 0.0 {
            return Ok(());
        }
        let area = Rect::new(0.0, 0.0, side, side)?;
        let radius = Length::new(painter.metric(MetricRole::CornerRadius))?;
        let border = painter.metric(MetricRole::BorderWidth).max(1.0);
        painter.fill_shape(area, Shape::Rounded(radius), painter.color(ColorRole::SurfaceContainer))?;
        painter.stroke_shape(area, Shape::Rounded(radius), Length::new(border)?, painter.color(ColorRole::Outline))?;

        // Crosshair through the handle, clipped to the area.
        let (x, y) = self.value;
        let hx = x as f32 * side;
        let hy = (1.0 - y as f32) * side;
        let line = painter.color(ColorRole::Muted);
        painter.with_clip(area, |p| -> Result<(), DrawError> {
            p.fill_rect(Rect::new(0.0, (hy - border / 2.0).max(0.0), side, border)?, line);
            p.fill_rect(Rect::new((hx - border / 2.0).max(0.0), 0.0, border, side)?, line);
            Ok(())
        })?;

        // Handle.
        let r = (side * 0.04).clamp(5.0, 9.0);
        let handle = Rect::new(hx - r, hy - r, 2.0 * r, 2.0 * r)?;
        let (fill, ring) = if self.drag.is_some() {
            (ColorRole::ButtonPressed, ColorRole::OnButtonPressed)
        } else {
            (ColorRole::Primary, ColorRole::OnPrimary)
        };
        painter.fill_ellipse(handle, painter.color(fill))?;
        painter.stroke_ellipse(
            handle,
            pert_ui::StrokeStyle::new(Length::new(1.5)?),
            painter.color(ring),
        )?;

        // Focus indication, as the built-in focus ring draws it.
        if painter.focus_within() {
            let width = Length::new(painter.metric(MetricRole::FocusWidth))?;
            painter.stroke_shape(area, Shape::Rounded(radius), width, painter.color(ColorRole::FocusRing))?;
        }
        Ok(())
    }
}
