//! `PointPicker`: a reusable two-dimensional point picker.
//!
//! The picker edits two `Signal<f64>` bindings, `x` and `y`, each in
//! `0.0..=1.0`. `x` grows to the right and `y` grows upwards.
//!
//! - A primary press inside the area moves the handle there and starts a
//!   drag; the drag captures the pointer, so it continues outside the area,
//!   clamped to its edges, until the matching release. The bindings follow
//!   the drag live.
//! - Escape during a drag, focus loss, hiding or removal cancel the drag and
//!   restore the point from before the press.
//! - When focused, the arrow keys move the point by 0.01, by 0.1 with Shift.
//! - `on_change` reports each committed user edit: a drag's release or one
//!   key press. Application writes and cancelled drags are not reported.
//! - Inspection reports a `Slider` whose value is the text `"x, y"`, plus
//!   the `x` and `y` properties as numbers.

use pert_ui::component::*;

/// Fine keyboard step.
pub const STEP: f64 = 0.01;
/// Coarse keyboard step (with Shift).
pub const COARSE_STEP: f64 = 0.1;

type ChangeHandler = Box<dyn FnMut(&mut EventContext, (f64, f64)) -> Result<(), RuntimeError>>;

struct Drag {
    before: (f64, f64),
}

pub struct PointPicker {
    x: Signal<f64>,
    y: Signal<f64>,
    shown: (f64, f64),
    drag: Option<Drag>,
    name: String,
    on_change: Option<ChangeHandler>,
}

impl PointPicker {
    pub fn new(x: &Signal<f64>, y: &Signal<f64>) -> Self {
        PointPicker {
            x: x.clone(),
            y: y.clone(),
            shown: (0.5, 0.5),
            drag: None,
            name: "Point picker".to_owned(),
            on_change: None,
        }
    }

    /// Names the picker for inspection.
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// Reports each committed user edit with the point written.
    pub fn on_change(
        mut self,
        handler: impl FnMut(&mut EventContext, (f64, f64)) -> Result<(), RuntimeError> + 'static,
    ) -> Self {
        self.on_change = Some(Box::new(handler));
        self
    }

    /// The point currently shown.
    pub fn value(&self) -> (f64, f64) {
        self.shown
    }

    /// Margin between the allocation and the track, so the handle stays inside.
    fn margin(side: f32) -> f32 {
        (side * 0.04).clamp(4.0, 10.0)
    }

    /// The square the picker draws in, and the track the handle moves on.
    fn geometry(size: Size) -> (f32, f32, f32) {
        let side = size.width().min(size.height());
        let margin = Self::margin(side);
        (side, margin, (side - 2.0 * margin).max(1.0))
    }

    fn point_at(&self, size: Size, local: Point) -> (f64, f64) {
        let (_, margin, track) = Self::geometry(size);
        let x = ((local.x() - margin) / track) as f64;
        let y = 1.0 - ((local.y() - margin) / track) as f64;
        (x.clamp(0.0, 1.0), y.clamp(0.0, 1.0))
    }

    /// Writes the bindings; returns whether the point changed.
    fn write(&mut self, cx: &mut EventContext, point: (f64, f64)) -> Result<bool, RuntimeError> {
        let point = (clamp_unit(point.0), clamp_unit(point.1));
        let changed_x = self.x.set_changed(point.0)?;
        let changed_y = self.y.set_changed(point.1)?;
        if point != self.shown {
            self.shown = point;
            cx.invalidate(Invalidation::Paint);
        }
        Ok(changed_x || changed_y)
    }

    fn report(&mut self, cx: &mut EventContext) -> Result<(), RuntimeError> {
        let point = self.shown;
        match self.on_change.as_mut() {
            Some(handler) => handler(cx, point),
            None => Ok(()),
        }
    }

    /// Ends a drag and puts the point back where it was before the press.
    fn cancel_drag(&mut self, cx: &mut EventContext) -> Result<bool, RuntimeError> {
        let Some(drag) = self.drag.take() else { return Ok(false) };
        cx.release_pointer();
        self.write(cx, drag.before)?;
        Ok(true)
    }
}

fn clamp_unit(v: f64) -> f64 {
    if v.is_nan() { 0.0 } else { v.clamp(0.0, 1.0) }
}

/// Rounds away binary noise from repeated decimal steps.
fn tidy(v: f64) -> f64 {
    (v * 1e6).round() / 1e6
}

impl Component for PointPicker {
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        let x = cx.read_paint(&self.x.read_only())?;
        let y = cx.read_paint(&self.y.read_only())?;
        self.shown = (clamp_unit(x), clamp_unit(y));
        Ok(())
    }

    fn measure(&self, cx: &MeasureContext<'_>, available: Constraints) -> Result<Size, RuntimeError> {
        let preferred = cx.metric(MetricRole::ColorAreaSize);
        let side = match (available.width(), available.height()) {
            (Some(w), Some(h)) => preferred.min(w).min(h),
            (Some(w), None) => preferred.min(w),
            (None, Some(h)) => preferred.min(h),
            (None, None) => preferred,
        };
        Ok(Size::new(side, side)?)
    }

    fn focusable(&self) -> bool {
        true
    }

    fn pointer_input(&mut self, cx: &mut EventContext, event: PointerEvent) -> Result<InputOutcome, RuntimeError> {
        let Some(size) = cx.size() else { return Ok(InputOutcome::Ignored) };
        let Some(local) = cx.to_local(event.position()) else {
            return Ok(InputOutcome::Ignored);
        };
        match event.kind() {
            PointerKind::Press(PointerButton::Primary) => {
                let (side, _, _) = Self::geometry(size);
                let inside = local.x() >= 0.0 && local.y() >= 0.0 && local.x() <= side && local.y() <= side;
                if !inside || self.drag.is_some() {
                    return Ok(InputOutcome::Ignored);
                }
                cx.capture_pointer()?;
                cx.request_focus();
                self.drag = Some(Drag { before: self.shown });
                let point = self.point_at(size, local);
                self.write(cx, point)?;
            }
            PointerKind::Move => {
                if self.drag.is_none() {
                    return Ok(InputOutcome::Ignored);
                }
                // A refreshed move derives the same point, and `write` then changes nothing.
                let point = self.point_at(size, local);
                self.write(cx, point)?;
            }
            PointerKind::Release(PointerButton::Primary) => {
                let Some(drag) = self.drag.take() else { return Ok(InputOutcome::Ignored) };
                cx.release_pointer();
                let point = self.point_at(size, local);
                self.write(cx, point)?;
                if self.shown != drag.before {
                    self.report(cx)?;
                }
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
            return Ok(if self.cancel_drag(cx)? { InputOutcome::Handled } else { InputOutcome::Ignored });
        }
        let modifiers = keyboard.chord_modifiers(event);
        if modifiers.control || modifiers.alt || modifiers.super_key {
            return Ok(InputOutcome::Ignored);
        }
        let (dx, dy) = match key {
            KeyCode::ArrowLeft => (-1.0, 0.0),
            KeyCode::ArrowRight => (1.0, 0.0),
            KeyCode::ArrowUp => (0.0, 1.0),
            KeyCode::ArrowDown => (0.0, -1.0),
            _ => return Ok(InputOutcome::Ignored),
        };
        if self.drag.is_some() {
            // Keys do not move the point while a pointer gesture runs.
            return Ok(InputOutcome::Handled);
        }
        let step = if keyboard.modifiers().shift { COARSE_STEP } else { STEP };
        let (x, y) = self.shown;
        let next = (tidy(x + dx * step), tidy(y + dy * step));
        if self.write(cx, next)? {
            self.report(cx)?;
        }
        Ok(InputOutcome::Handled)
    }

    fn cancel_input(&mut self, cx: &mut EventContext) {
        self.input_reset();
        // Cancellation cannot fail; a failed restore leaves the bindings as the drag left them.
        let _ = self.cancel_drag(cx);
        cx.invalidate(Invalidation::Paint);
    }

    fn inspect(&self) -> ComponentDetails {
        let (x, y) = self.shown;
        let mut details = ComponentDetails {
            semantics: Some(
                Semantics::new(SemanticRole::Slider)
                    .name(self.name.clone())
                    .value(SemanticValue::Text(format!("{x:.2}, {y:.2}")))
                    .actions([SemanticAction::SetValue, SemanticAction::Increment, SemanticAction::Decrement]),
            ),
            ..ComponentDetails::default()
        };
        details.properties.insert("x".to_owned(), InspectValue::Number(x));
        details.properties.insert("y".to_owned(), InspectValue::Number(y));
        details.properties.insert("dragging".to_owned(), InspectValue::Bool(self.drag.is_some()));
        details
    }

    fn paint(&self, painter: &mut Painter) -> Result<(), DrawError> {
        let (side, margin, track) = Self::geometry(painter.size());
        let area = Rect::new(0.0, 0.0, side, side)?;
        let corner = Length::new(painter.metric(MetricRole::CornerRadius))?;
        let shape = Shape::Rounded(corner);
        let border = Length::new(painter.metric(MetricRole::BorderWidth).max(1.0))?;

        painter.fill_shape(area, shape, painter.color(ColorRole::SurfaceContainer))?;
        painter.stroke_shape(area, shape, border, painter.color(ColorRole::Outline))?;

        // Quarter grid.
        let grid = painter.color(ColorRole::Divider).scale_alpha(0.6)?;
        for i in 1..4 {
            let t = margin + track * i as f32 / 4.0;
            painter.fill_rect(Rect::new(t - 0.5, margin, 1.0, track)?, grid);
            painter.fill_rect(Rect::new(margin, t - 0.5, track, 1.0)?, grid);
        }

        // Crosshair through the point.
        let (x, y) = self.shown;
        let px = margin + track * x as f32;
        let py = margin + track * (1.0 - y as f32);
        let line = painter.color(ColorRole::OnSurface);
        painter.fill_rect(Rect::new(px - 0.75, margin, 1.5, track)?, line);
        painter.fill_rect(Rect::new(margin, py - 0.75, track, 1.5)?, line);

        // Handle.
        let radius = (margin - 1.0).max(3.0);
        let handle = Rect::new(px - radius, py - radius, 2.0 * radius, 2.0 * radius)?;
        painter.fill_ellipse(handle, painter.color(ColorRole::Primary))?;
        painter.stroke_ellipse(
            handle,
            StrokeStyle::new(Length::new(1.5)?),
            painter.color(ColorRole::OnPrimary),
        )?;

        // Focus ring, as the built-in controls draw it.
        if painter.focus_within() {
            let width = Length::new(painter.metric(MetricRole::FocusWidth))?;
            painter.stroke_shape(area, shape, width, painter.color(ColorRole::FocusRing))?;
        }
        Ok(())
    }
}
