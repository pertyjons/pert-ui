//! A reusable live-edit point control. Coordinates increase rightward and downward.
use pert_ui::ApplicationError;
use pert_ui::component::*;

type Change = dyn FnMut(&mut EventContext, [f64; 2]) -> Result<(), RuntimeError>;

pub struct PointPicker {
    x: Signal<f64>,
    y: Signal<f64>,
    shown: [f64; 2],
    origin: Option<[f64; 2]>,
    failed: Option<RuntimeError>,
    on_change: Box<Change>,
}

impl PointPicker {
    /// Owner-provided signals must contain finite values in 0..=1.
    pub fn new(x: Signal<f64>, y: Signal<f64>) -> Self {
        Self {
            x,
            y,
            shown: [0.5; 2],
            origin: None,
            failed: None,
            on_change: Box::new(|_, _| Ok(())),
        }
    }
    pub fn on_change(
        mut self,
        callback: impl FnMut(&mut EventContext, [f64; 2]) -> Result<(), RuntimeError> + 'static,
    ) -> Self {
        self.on_change = Box::new(callback);
        self
    }
    fn current(&self) -> Result<[f64; 2], RuntimeError> {
        Ok([self.x.get()?, self.y.get()?])
    }
    fn write(&mut self, cx: &mut EventContext, p: [f64; 2]) -> Result<(), RuntimeError> {
        let changed_x = self.x.set_changed(p[0])?;
        let changed_y = self.y.set_changed(p[1])?;
        if changed_x || changed_y {
            (self.on_change)(cx, p)?;
        }
        Ok(())
    }
    fn restore(&mut self) -> Result<Option<[f64; 2]>, RuntimeError> {
        let Some(p) = self.origin.take() else {
            return Ok(None);
        };
        self.x.set(p[0])?;
        self.y.set(p[1])?;
        Ok(Some(p))
    }
}
impl Component for PointPicker {
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        let p = [cx.read(&self.x.read_only())?, cx.read(&self.y.read_only())?];
        if !p.iter().all(|v| (0.0..=1.0).contains(v)) {
            return Err(
                ApplicationError::from("point coordinates must be finite and in 0..=1").into(),
            );
        }
        self.shown = p;
        Ok(())
    }
    fn measure(&self, _: &MeasureContext<'_>, _: Constraints) -> Result<Size, RuntimeError> {
        Ok(Size::new(260.0, 260.0)?)
    }
    fn size_limits(&self) -> SizeLimits {
        SizeLimits::exact(Size::from([260, 260]))
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
            PointerKind::Press(PointerButton::Primary) if self.origin.is_none() => {
                cx.capture_pointer()?;
                cx.request_focus();
                self.origin = Some(self.current()?);
            }
            PointerKind::Move if self.origin.is_some() => {}
            PointerKind::Release(PointerButton::Primary) if self.origin.is_some() => {
                // A release may arrive at a different position from the last move.
                if let (Some(p), Some(s)) = (cx.to_local(event.position()), cx.size()) {
                    self.write(
                        cx,
                        [
                            (p.x() / s.width()).clamp(0.0, 1.0) as f64,
                            (p.y() / s.height()).clamp(0.0, 1.0) as f64,
                        ],
                    )?;
                }
                self.origin = None;
                cx.release_pointer();
                return Ok(InputOutcome::Handled);
            }
            _ => return Ok(InputOutcome::Ignored),
        }
        if let (Some(p), Some(s)) = (cx.to_local(event.position()), cx.size()) {
            self.write(
                cx,
                [
                    (p.x() / s.width()).clamp(0.0, 1.0) as f64,
                    (p.y() / s.height()).clamp(0.0, 1.0) as f64,
                ],
            )?;
        }
        Ok(InputOutcome::Handled)
    }
    fn keyboard_event(
        &mut self,
        cx: &mut EventContext,
        event: KeyEvent,
        _: &KeyboardState,
    ) -> Result<InputOutcome, RuntimeError> {
        if event.transition == KeyTransition::Released {
            return Ok(InputOutcome::Ignored);
        }
        let mut p = self.current()?;
        let step = if cx.modifiers().shift { 0.1 } else { 0.01 };
        match event.code {
            KeyCode::Escape => {
                if let Some(p) = self.restore()? {
                    (self.on_change)(cx, p)?;
                } else {
                    return Ok(InputOutcome::Ignored);
                }
                return Ok(InputOutcome::Handled);
            }
            KeyCode::ArrowLeft => p[0] -= step,
            KeyCode::ArrowRight => p[0] += step,
            KeyCode::ArrowUp => p[1] -= step,
            KeyCode::ArrowDown => p[1] += step,
            _ => return Ok(InputOutcome::Ignored),
        }
        self.write(cx, p.map(|v| v.clamp(0.0, 1.0)))?;
        Ok(InputOutcome::Handled)
    }
    fn cancel_input(&mut self, cx: &mut EventContext) {
        if let Err(e) = self.restore() {
            self.failed = Some(e);
            cx.notifier().request();
        }
    }
    fn deliver_notifications(&mut self, _: &mut EventContext) -> Result<(), RuntimeError> {
        self.failed.take().map_or(Ok(()), Err)
    }
    fn paint(&self, p: &mut Painter) -> Result<(), DrawError> {
        let s = p.size();
        let bounds = Rect::from_size(s);
        p.fill_rect(bounds, p.color(ColorRole::Surface));
        let x = self.shown[0] as f32 * s.width();
        let y = self.shown[1] as f32 * s.height();
        p.fill_rect(
            Rect::new(x.min(s.width() - 1.0), 0.0, 1.0, s.height())?,
            p.color(ColorRole::Primary),
        );
        p.fill_rect(
            Rect::new(0.0, y.min(s.height() - 1.0), s.width(), 1.0)?,
            p.color(ColorRole::Primary),
        );
        let handle = Rect::new(
            (x - 6.0).clamp(0.0, s.width() - 12.0),
            (y - 6.0).clamp(0.0, s.height() - 12.0),
            12.0,
            12.0,
        )?;
        p.fill_rect(handle, p.color(ColorRole::Primary));
        if p.focus_within() {
            p.stroke_shape(
                bounds,
                Shape::Rectangle,
                Length::new(p.metric(MetricRole::FocusWidth))?,
                p.color(ColorRole::FocusRing),
            )?;
        }
        Ok(())
    }
    fn inspect(&self) -> ComponentDetails {
        ComponentDetails {
            semantics: Some(
                Semantics::new(SemanticRole::Slider)
                    .name("Point picker")
                    .value(SemanticValue::Text(format!(
                        "{}, {}",
                        self.shown[0], self.shown[1]
                    ))),
            ),
            properties: [
                ("x".into(), InspectValue::Number(self.shown[0])),
                ("y".into(), InspectValue::Number(self.shown[1])),
            ]
            .into(),
            ..ComponentDetails::default()
        }
    }
}
