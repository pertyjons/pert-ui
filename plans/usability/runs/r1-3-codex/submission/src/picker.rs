use pert_ui::*;

/// Normalized coordinates, with the origin at the upper left.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PickPoint {
    pub x: f64,
    pub y: f64,
}
impl PickPoint {
    pub fn new(x: f64, y: f64) -> Self {
        Self {
            x: if x.is_finite() {
                x.clamp(0.0, 1.0)
            } else {
                0.5
            },
            y: if y.is_finite() {
                y.clamp(0.0, 1.0)
            } else {
                0.5
            },
        }
    }
}
type Change = Box<dyn FnMut(&mut EventContext, PickPoint) -> Result<(), RuntimeError>>;
/// A fixed square control. Owners write the binding; callbacks report user edits,
/// including restoration after cancellation. Values are normalized on presentation.
pub struct PointPicker {
    value: Signal<PickPoint>,
    shown: PickPoint,
    before: Option<PickPoint>,
    changed: Option<Change>,
    cancellation_error: Option<RuntimeError>,
}
impl PointPicker {
    pub fn new(value: Signal<PickPoint>) -> Self {
        Self {
            value,
            shown: PickPoint::new(0.5, 0.5),
            before: None,
            changed: None,
            cancellation_error: None,
        }
    }
    pub fn on_change(
        mut self,
        f: impl FnMut(&mut EventContext, PickPoint) -> Result<(), RuntimeError> + 'static,
    ) -> Self {
        self.changed = Some(Box::new(f));
        self
    }
    fn change(&mut self, cx: &mut EventContext, value: PickPoint) -> Result<(), RuntimeError> {
        if value != self.shown {
            self.value.set(value)?;
            self.shown = value;
            cx.invalidate(Invalidation::Paint);
            if let Some(f) = &mut self.changed {
                f(cx, value)?;
            }
        }
        Ok(())
    }
    fn restore(&mut self, cx: &mut EventContext) -> Result<(), RuntimeError> {
        cx.release_pointer();
        if let Some(old) = self.before.take() {
            self.change(cx, old)?;
        }
        Ok(())
    }
    fn at(&mut self, cx: &mut EventContext, p: Point) -> Result<(), RuntimeError> {
        if let (Some(p), Some(s)) = (cx.to_local(p), cx.size()) {
            self.change(
                cx,
                PickPoint::new(
                    (p.x() / s.width().max(1.0)) as f64,
                    (p.y() / s.height().max(1.0)) as f64,
                ),
            )?;
        }
        Ok(())
    }
}
impl Component for PointPicker {
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        if let Some(error) = self.cancellation_error.take() {
            return Err(error);
        }
        let value = cx.read(&self.value)?;
        self.shown = PickPoint::new(value.x, value.y);
        Ok(())
    }
    fn focusable(&self) -> bool {
        true
    }
    fn size_limits(&self) -> SizeLimits {
        let size = Size::new(260.0, 260.0).unwrap();
        SizeLimits::new(size, Constraints::bounded(size)).unwrap()
    }
    fn measure(&self, _: &MeasureContext<'_>, _: Constraints) -> Result<Size, RuntimeError> {
        Ok(Size::new(260.0, 260.0)?)
    }
    fn pointer_input(
        &mut self,
        cx: &mut EventContext,
        e: PointerEvent,
    ) -> Result<InputOutcome, RuntimeError> {
        match e.kind() {
            PointerKind::Press(PointerButton::Primary) => {
                cx.request_focus();
                cx.capture_pointer()?;
                self.before = Some(self.shown);
                self.at(cx, e.position())?;
            }
            PointerKind::Move if self.before.is_some() => self.at(cx, e.position())?,
            PointerKind::Release(PointerButton::Primary) if self.before.is_some() => {
                self.at(cx, e.position())?;
                self.before = None;
                cx.release_pointer();
            }
            _ => return Ok(InputOutcome::Ignored),
        }
        Ok(InputOutcome::Handled)
    }
    fn keyboard_event(
        &mut self,
        cx: &mut EventContext,
        e: KeyEvent,
        keys: &KeyboardState,
    ) -> Result<InputOutcome, RuntimeError> {
        if !matches!(
            e.transition,
            KeyTransition::Pressed | KeyTransition::Repeated
        ) {
            return Ok(InputOutcome::Ignored);
        }
        let step = if keys.modifiers().shift { 0.1 } else { 0.01 };
        let (mut x, mut y) = (self.shown.x, self.shown.y);
        match keys.control_key(e) {
            KeyCode::ArrowLeft => x -= step,
            KeyCode::ArrowRight => x += step,
            KeyCode::ArrowUp => y -= step,
            KeyCode::ArrowDown => y += step,
            KeyCode::Escape if self.before.is_some() => {
                self.restore(cx)?;
                return Ok(InputOutcome::Handled);
            }
            _ => return Ok(InputOutcome::Ignored),
        }
        self.change(cx, PickPoint::new(x, y))?;
        Ok(InputOutcome::Handled)
    }
    fn cancel_input(&mut self, cx: &mut EventContext) {
        // Cancellation has no Result; surface an exceptional failed binding/callback
        // on the next evaluation rather than panic or silently discard it.
        self.cancellation_error = self.restore(cx).err();
        cx.invalidate(Invalidation::Paint);
    }
    fn inspect(&self) -> ComponentDetails {
        let mut details = ComponentDetails::default();
        details.properties.insert(
            "value".into(),
            InspectValue::Object(
                [
                    ("x".into(), InspectValue::Number(self.shown.x)),
                    ("y".into(), InspectValue::Number(self.shown.y)),
                ]
                .into(),
            ),
        );
        details.semantics = Some(Semantics::new(SemanticRole::Slider).name("Point").value(
            SemanticValue::Text(format!("x {:.2}, y {:.2}", self.shown.x, self.shown.y)),
        ));
        details
    }
    fn paint(&self, p: &mut Painter) -> Result<(), DrawError> {
        let Some(size) = p.size() else {
            return Ok(());
        };
        let (w, h) = (size.width(), size.height());
        let bounds = Rect::from_size(size);
        p.fill_rect(bounds, p.color(ColorRole::Surface));
        p.stroke_shape(
            bounds,
            Shape::Rectangle,
            Length::new(1.0)?,
            p.color(ColorRole::Outline),
        )?;
        let (x, y) = (self.shown.x as f32 * w, self.shown.y as f32 * h);
        p.fill_rect(
            Rect::new(x.clamp(0.0, w - 1.0), 0.0, 1.0, h)?,
            p.color(ColorRole::Muted),
        );
        p.fill_rect(
            Rect::new(0.0, y.clamp(0.0, h - 1.0), w, 1.0)?,
            p.color(ColorRole::Muted),
        );
        p.fill_ellipse(
            Rect::new(x - 6.0, y - 6.0, 12.0, 12.0)?,
            p.color(ColorRole::Primary),
        )?;
        p.stroke_ellipse(
            Rect::new(x - 6.0, y - 6.0, 12.0, 12.0)?,
            StrokeStyle::new(Length::new(2.0)?),
            p.color(ColorRole::OnPrimary),
        )?;
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
}
