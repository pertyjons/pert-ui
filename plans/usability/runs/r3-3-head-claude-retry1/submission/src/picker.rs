//! A two-dimensional point picker: a square area with a crosshair and a
//! handle at the current point, `x` and `y` each in 0.0..=1.0.
//!
//! `x` grows to the right and `y` grows upwards, so (0, 0) is the bottom-left
//! corner. The picker writes two `Signal<f64>`s live (each step of a drag or a
//! key writes them), so anything else bound to the same signals follows; the
//! owner sets the value by writing those signals. `on_change` reports each
//! write the user makes.

use pert_ui::ApplicationError;
use pert_ui::component::*;

/// Reports a user's write of the point, as `(x, y)`.
pub type ChangeCallback = Box<dyn FnMut(&mut EventContext, (f64, f64)) -> Result<(), RuntimeError>>;

/// The keyboard step, and the step with Shift held.
pub const STEP: f64 = 0.01;
pub const SHIFT_STEP: f64 = 0.1;

pub struct PointPicker {
    x: Signal<f64>,
    y: Signal<f64>,
    name: String,
    /// The point the picker shows, as last read in `update`.
    shown: (f64, f64),
    /// The point from before the drag, while one is open.
    origin: Option<(f64, f64)>,
    /// A restore `cancel_input` could not make, for `deliver_notifications`.
    failed: Option<RuntimeError>,
    on_change: Option<ChangeCallback>,
}

impl PointPicker {
    pub fn new(x: &Signal<f64>, y: &Signal<f64>) -> Self {
        PointPicker {
            x: x.clone(),
            y: y.clone(),
            name: "Point".to_owned(),
            shown: (0.0, 0.0),
            origin: None,
            failed: None,
            on_change: None,
        }
    }

    /// Names the picker for inspection.
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// Reports each write the user makes: a drag step, a key step and
    /// Escape's restore. A restore from `cancel_input` is not reported.
    pub fn on_change(
        mut self,
        f: impl FnMut(&mut EventContext, (f64, f64)) -> Result<(), RuntimeError> + 'static,
    ) -> Self {
        self.on_change = Some(Box::new(f));
        self
    }

    /// Writes a user's point and reports it; called only from input callbacks.
    fn write(&mut self, cx: &mut EventContext, point: (f64, f64)) -> Result<(), RuntimeError> {
        let point = (clean(point.0), clean(point.1));
        let changed_x = self.x.set_changed(point.0)?;
        let changed_y = self.y.set_changed(point.1)?;
        if (changed_x || changed_y)
            && let Some(on_change) = self.on_change.as_mut()
        {
            on_change(cx, point)?;
        }
        Ok(())
    }

    /// Ends the drag and writes the point from before it back.
    fn restore(&mut self) -> Result<Option<(f64, f64)>, RuntimeError> {
        let Some(origin) = self.origin.take() else {
            return Ok(None);
        };
        self.x.set(origin.0)?;
        self.y.set(origin.1)?;
        Ok(Some(origin))
    }

    /// The square the picker draws in, at the top-left of its bounds.
    fn side(size: Size) -> f32 {
        size.width().min(size.height())
    }

    /// The point under a pointer position, clamped to the area's edges.
    fn at(&self, cx: &EventContext, event: &PointerEvent) -> Option<(f64, f64)> {
        let (local, size) = (cx.to_local(event.position())?, cx.size()?);
        let side = Self::side(size);
        if side <= 0.0 {
            return None;
        }
        // Pointer positions are f32; four decimals are finer than a pixel and
        // keep f32 noise such as 0.100000001 out of the numeric fields.
        let to_value = |fraction: f32| (f64::from(fraction) * 1e4).round() / 1e4;
        let x = to_value(local.x() / side).clamp(0.0, 1.0);
        let y = to_value(1.0 - local.y() / side).clamp(0.0, 1.0);
        Some((x, y))
    }
}

/// Keeps stepped values free of binary noise, such as 0.51 rather than
/// 0.51000000000000001.
fn clean(value: f64) -> f64 {
    ((value * 1e9).round() / 1e9).clamp(0.0, 1.0)
}

impl Component for PointPicker {
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        let x = cx.read_paint(&self.x.read_only())?;
        let y = cx.read_paint(&self.y.read_only())?;
        // Refused readings, NaN included, keep what the picker shows.
        if !(0.0..=1.0).contains(&x) || !(0.0..=1.0).contains(&y) {
            return Err(ApplicationError::from("a point picker's x and y lie in 0..=1").into());
        }
        self.shown = (x, y);
        Ok(())
    }

    fn measure(&self, cx: &MeasureContext<'_>, _: Constraints) -> Result<Size, RuntimeError> {
        let side = cx.metric(MetricRole::ColorAreaSize);
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
        match event.kind() {
            PointerKind::Press(PointerButton::Primary) => {
                let (local, size) = match (cx.to_local(event.position()), cx.size()) {
                    (Some(local), Some(size)) => (local, size),
                    _ => return Ok(InputOutcome::Ignored),
                };
                let side = Self::side(size);
                if local.x() > side || local.y() > side {
                    return Ok(InputOutcome::Ignored); // outside the square
                }
                cx.capture_pointer()?;
                cx.request_focus();
                if self.origin.is_none() {
                    self.origin = Some((self.x.get()?, self.y.get()?));
                }
            }
            PointerKind::Move if self.origin.is_some() => {}
            PointerKind::Release(PointerButton::Primary) if self.origin.is_some() => {
                self.origin = None; // the drag's last write stands
                cx.release_pointer();
                return Ok(InputOutcome::Handled);
            }
            _ => return Ok(InputOutcome::Ignored),
        }
        if let Some(point) = self.at(cx, &event) {
            self.write(cx, point)?; // a repeated position writes nothing
        }
        Ok(InputOutcome::Handled)
    }

    fn keyboard_event(
        &mut self,
        cx: &mut EventContext,
        event: KeyEvent,
        keyboard: &KeyboardState,
    ) -> Result<InputOutcome, RuntimeError> {
        if event.transition == KeyTransition::Released {
            return Ok(InputOutcome::Ignored);
        }
        let step = if keyboard.modifiers().shift {
            SHIFT_STEP
        } else {
            STEP
        };
        let (x, y) = (self.x.get()?, self.y.get()?);
        let point = match event.code {
            KeyCode::Escape => {
                return match self.restore()? {
                    Some(origin) => {
                        if let Some(on_change) = self.on_change.as_mut() {
                            on_change(cx, origin)?; // the user's write too
                        }
                        Ok(InputOutcome::Handled)
                    }
                    None => Ok(InputOutcome::Ignored),
                };
            }
            KeyCode::ArrowLeft => (x - step, y),
            KeyCode::ArrowRight => (x + step, y),
            KeyCode::ArrowUp => (x, y + step),
            KeyCode::ArrowDown => (x, y - step),
            _ => return Ok(InputOutcome::Ignored),
        };
        self.write(cx, point)?;
        Ok(InputOutcome::Handled)
    }

    fn cancel_input(&mut self, cx: &mut EventContext) {
        if let Err(error) = self.restore() {
            self.failed = Some(error);
            cx.notifier().request();
        }
        self.input_reset();
        cx.invalidate(Invalidation::Paint);
    }

    fn deliver_notifications(&mut self, _: &mut EventContext) -> Result<(), RuntimeError> {
        self.failed.take().map_or(Ok(()), Err)
    }

    fn paint(&self, painter: &mut Painter) -> Result<(), DrawError> {
        let side = Self::side(painter.size());
        let area = Rect::new(0.0, 0.0, side, side)?;
        painter.fill_rect(area, painter.color(ColorRole::SurfaceContainer));
        let border = Length::new(painter.metric(MetricRole::BorderWidth))?;
        painter.stroke_shape(
            area,
            Shape::Rectangle,
            border,
            painter.color(ColorRole::Outline),
        )?;

        // The crosshair: one line across the whole area in each direction.
        let (px, py) = (
            self.shown.0 as f32 * side,
            (1.0 - self.shown.1 as f32) * side,
        );
        let line = 1.0_f32;
        let cross = painter.color(ColorRole::Muted);
        painter.fill_rect(
            Rect::new((px - line / 2.0).clamp(0.0, side - line), 0.0, line, side)?,
            cross,
        );
        painter.fill_rect(
            Rect::new(0.0, (py - line / 2.0).clamp(0.0, side - line), side, line)?,
            cross,
        );

        // The handle.
        let radius = 6.0_f32;
        let handle = Rect::new(px - radius, py - radius, 2.0 * radius, 2.0 * radius)?;
        painter.fill_ellipse(handle, painter.color(ColorRole::Primary))?;
        painter.stroke_ellipse(
            handle,
            StrokeStyle::new(Length::new(2.0)?),
            painter.color(ColorRole::OnPrimary),
        )?;

        if painter.focus_within() {
            let width = Length::new(painter.metric(MetricRole::FocusWidth))?;
            painter.stroke_shape(
                area,
                Shape::Rectangle,
                width,
                painter.color(ColorRole::FocusRing),
            )?;
        }
        Ok(())
    }

    fn inspect(&self) -> ComponentDetails {
        let (x, y) = self.shown;
        let mut details = ComponentDetails {
            semantics: Some(
                Semantics::new(SemanticRole::Slider)
                    .name(self.name.clone())
                    .value(SemanticValue::Text(format!("x {x}, y {y}")))
                    .actions([SemanticAction::Increment, SemanticAction::Decrement]),
            ),
            ..ComponentDetails::default()
        };
        details
            .properties
            .insert("x".to_owned(), InspectValue::Number(x));
        details
            .properties
            .insert("y".to_owned(), InspectValue::Number(y));
        details.properties.insert(
            "dragging".to_owned(),
            InspectValue::Bool(self.origin.is_some()),
        );
        details
    }
}
