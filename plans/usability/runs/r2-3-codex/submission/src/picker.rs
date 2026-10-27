//! A reusable two-axis control. Coordinates increase rightwards and downwards.
use pert_ui::component::*;

type Change = Box<dyn FnMut(&mut EventContext, (f64, f64)) -> Result<(), RuntimeError>>;

pub struct PointPicker {
    x: Signal<f64>,
    y: Signal<f64>,
    value: (f64, f64),
    before_drag: Option<(f64, f64)>,
    on_change: Option<Change>,
    restore_error: Option<RuntimeError>,
}
impl PointPicker {
    /// Owner writes these signals to set the value. Values should be finite and in 0..=1.
    pub fn new(x: Signal<f64>, y: Signal<f64>) -> Self {
        Self {
            x,
            y,
            value: (0.5, 0.5),
            before_drag: None,
            on_change: None,
            restore_error: None,
        }
    }
    /// Reports user changes, including an Escape restoration. Context cancellation
    /// restores the shared signals without invoking an application action.
    pub fn on_change(
        mut self,
        callback: impl FnMut(&mut EventContext, (f64, f64)) -> Result<(), RuntimeError> + 'static,
    ) -> Self {
        self.on_change = Some(Box::new(callback));
        self
    }
    fn write(
        &mut self,
        cx: &mut EventContext,
        value: (f64, f64),
        notify: bool,
    ) -> Result<(), RuntimeError> {
        let value = (value.0.clamp(0.0, 1.0), value.1.clamp(0.0, 1.0));
        if value == self.value {
            return Ok(());
        }
        self.x.set(value.0)?;
        self.y.set(value.1)?;
        self.value = value;
        cx.invalidate(Invalidation::Paint);
        if notify && let Some(callback) = &mut self.on_change {
            callback(cx, value)?;
        }
        Ok(())
    }
    fn move_to(&mut self, cx: &mut EventContext, point: Point) -> Result<(), RuntimeError> {
        // Keep the complete handle inside the square, including at the endpoints.
        self.write(
            cx,
            (
                ((point.x() - 12.0) / 236.0) as f64,
                ((point.y() - 12.0) / 236.0) as f64,
            ),
            true,
        )
    }
    fn restore(&mut self, cx: &mut EventContext, notify: bool) -> Result<(), RuntimeError> {
        cx.release_pointer();
        if let Some(old) = self.before_drag.take() {
            self.write(cx, old, notify)?;
        }
        Ok(())
    }
}
impl Component for PointPicker {
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        self.value = (
            cx.read(&self.x)?.clamp(0.0, 1.0),
            cx.read(&self.y)?.clamp(0.0, 1.0),
        );
        Ok(())
    }
    fn focusable(&self) -> bool {
        true
    }
    fn size_limits(&self) -> SizeLimits {
        SizeLimits::exact(Size::new(260.0, 260.0).unwrap())
    }
    fn measure(&self, _: &MeasureContext<'_>, _: Constraints) -> Result<Size, RuntimeError> {
        Ok(Size::new(260.0, 260.0)?)
    }
    fn pointer_input(
        &mut self,
        cx: &mut EventContext,
        event: PointerEvent,
    ) -> Result<InputOutcome, RuntimeError> {
        let Some(local) = cx.to_local(event.position()) else {
            return Ok(InputOutcome::Ignored);
        };
        match event.kind() {
            PointerKind::Press(PointerButton::Primary) => {
                cx.capture_pointer()?;
                if self.before_drag.is_none() {
                    self.before_drag = Some(self.value);
                }
                self.move_to(cx, local)?;
            }
            PointerKind::Move if self.before_drag.is_some() => self.move_to(cx, local)?,
            PointerKind::Release(PointerButton::Primary) if self.before_drag.is_some() => {
                self.move_to(cx, local)?;
                self.before_drag = None;
                cx.release_pointer();
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
        if event.transition == KeyTransition::Released {
            return Ok(InputOutcome::Ignored);
        }
        let step = if keyboard.modifiers().shift {
            0.1
        } else {
            0.01
        };
        let (x, y) = self.value;
        let value = match keyboard.control_key(event) {
            KeyCode::Escape if self.before_drag.is_some() => {
                self.restore(cx, true)?;
                return Ok(InputOutcome::Handled);
            }
            KeyCode::ArrowLeft => (x - step, y),
            KeyCode::ArrowRight => (x + step, y),
            KeyCode::ArrowUp => (x, y - step),
            KeyCode::ArrowDown => (x, y + step),
            _ => return Ok(InputOutcome::Ignored),
        };
        self.write(cx, value, true)?;
        Ok(InputOutcome::Handled)
    }
    fn focus_lost(&mut self, cx: &mut EventContext) -> Result<(), RuntimeError> {
        self.restore(cx, false)
    }
    fn cancel_input(&mut self, cx: &mut EventContext) {
        if let Err(error) = self.restore(cx, false) {
            self.restore_error = Some(error);
            cx.notifier().request();
        }
    }
    fn deliver_notifications(&mut self, _: &mut EventContext) -> Result<(), RuntimeError> {
        if let Some(error) = self.restore_error.take() {
            return Err(error);
        }
        Ok(())
    }
    fn paint(&self, p: &mut Painter) -> Result<(), DrawError> {
        let background = p.color(ColorRole::SurfaceContainer);
        let line = p.color(ColorRole::Outline);
        let accent = p.color(ColorRole::Primary);
        p.fill_rect(Rect::new(0.0, 0.0, 260.0, 260.0)?, background);
        let x = 12.0 + self.value.0 as f32 * 236.0;
        let y = 12.0 + self.value.1 as f32 * 236.0;
        p.fill_rect(Rect::new(x - 0.5, 12.0, 1.0, 236.0)?, line);
        p.fill_rect(Rect::new(12.0, y - 0.5, 236.0, 1.0)?, line);
        p.fill_ellipse(Rect::new(x - 7.0, y - 7.0, 14.0, 14.0)?, accent)?;
        p.fill_ellipse(
            Rect::new(x - 2.0, y - 2.0, 4.0, 4.0)?,
            p.color(ColorRole::OnPrimary),
        )?;
        let border = if p.focus_within() {
            p.color(ColorRole::FocusRing)
        } else {
            line
        };
        // Four inward edges leave the focus indication visible inside the clip.
        for (x, y, w, h) in [
            (0., 0., 260., 2.),
            (0., 258., 260., 2.),
            (0., 0., 2., 260.),
            (258., 0., 2., 260.),
        ] {
            p.fill_rect(Rect::new(x, y, w, h)?, border);
        }
        Ok(())
    }
    fn inspect(&self) -> ComponentDetails {
        ComponentDetails {
            semantics: Some(
                Semantics::new(SemanticRole::Slider)
                    .name("Point picker")
                    .description(
                        "Arrow keys move by 0.01; Shift moves by 0.1; Escape cancels a drag",
                    )
                    .value(SemanticValue::Text(format!(
                        "x={:.3}, y={:.3}",
                        self.value.0, self.value.1
                    ))),
            ),
            properties: [
                ("x".into(), InspectValue::Number(self.value.0)),
                ("y".into(), InspectValue::Number(self.value.1)),
            ]
            .into(),
            ..ComponentDetails::default()
        }
    }
}
