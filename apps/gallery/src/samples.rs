//! Small consumer-authored components using the same public contract as built-ins.
use pert_ui::*;
use std::{cell::RefCell, time::Duration};

/// Status text in body typography, drawn by the components below.
#[derive(Default)]
struct Text {
    value: String,
    prepared: RefCell<Option<TextLayout>>,
}
impl Text {
    fn style(&self) -> TextStyle {
        TextStyle::Body
    }
    fn measure(
        &self,
        cx: &MeasureContext<'_>,
        available: Constraints,
    ) -> Result<Size, RuntimeError> {
        Ok(cx
            .text(&self.value, available.width(), self.style())?
            .size())
    }
    fn arrange(&self, cx: &ArrangeContext<'_>, size: Size) -> Result<(), RuntimeError> {
        *self.prepared.borrow_mut() = Some(cx.measurement().text(
            &self.value,
            Some(size.width()),
            self.style(),
        )?);
        Ok(())
    }
    fn paint(&self, painter: &mut Painter) -> Result<(), DrawError> {
        if let Some(text) = self.prepared.borrow().as_ref() {
            text.paint(painter, Point::ZERO, painter.color(ColorRole::OnSurface))?;
        }
        Ok(())
    }
    fn details(&self) -> ComponentDetails {
        let mut details = ComponentDetails::default();
        details
            .properties
            .insert("text".into(), InspectValue::Text(self.value.clone()));
        details
    }
}

pub(super) struct Pulse {
    running: ReadSignal<bool>,
    active: bool,
    updates: u64,
}
impl Pulse {
    pub(super) fn new(running: ReadSignal<bool>) -> Self {
        Self {
            running,
            active: false,
            updates: 0,
        }
    }
}
impl Component for Pulse {
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        self.active = cx.read_paint(&self.running)?;
        if self.active {
            // The count only picks the lit cell, so wrapping keeps the pulse going.
            self.updates = self.updates.wrapping_add(1);
            cx.wake_after(Duration::from_millis(250))?;
        }
        Ok(())
    }
    fn measure(&self, _: &MeasureContext<'_>, _: Constraints) -> Result<Size, RuntimeError> {
        Ok(Size::from([320, 32]))
    }
    fn paint(&self, painter: &mut Painter) -> Result<(), DrawError> {
        let width = painter.size().width();
        let cell = width / 12.0;
        for index in 0..12 {
            let color = if self.active && index == self.updates % 12 {
                painter.color(ColorRole::Primary)
            } else {
                painter.color(ColorRole::SurfaceHover)
            };
            painter.fill_rect(
                Rect::new(index as f32 * cell, 0.0, (cell - 4.0).max(0.0), 32.0)?,
                color,
            );
        }
        Ok(())
    }
    fn inspect(&self) -> ComponentDetails {
        let mut details = ComponentDetails::default();
        details
            .properties
            .insert("updates".into(), InspectValue::Number(self.updates as f64));
        details
            .properties
            .insert("running".into(), InspectValue::Bool(self.active));
        details
    }
}

#[derive(Default)]
pub(super) struct KeyPad {
    text: Text,
    focused: bool,
}
impl Component for KeyPad {
    fn focusable(&self) -> bool {
        true
    }
    fn size_limits(&self) -> SizeLimits {
        SizeLimits::at_least(Size::from([160, 100]))
    }
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        self.focused = cx.focused();
        if !self.focused {
            self.text.value =
                "Click here or focus with Tab.\nHold several letter or arrow keys together.".into();
        }
        Ok(())
    }
    fn keyboard_event(
        &mut self,
        cx: &mut EventContext,
        event: KeyEvent,
        keyboard: &KeyboardState,
    ) -> Result<InputOutcome, RuntimeError> {
        if event.code == KeyCode::Tab {
            return Ok(InputOutcome::Ignored);
        }
        self.text.value = format!(
            "Last: {:?} {:?}\nHeld: {:?}\nModifiers: {:?}",
            event.code,
            event.transition,
            keyboard.pressed().collect::<Vec<_>>(),
            keyboard.modifiers()
        );
        cx.invalidate(Invalidation::Layout);
        Ok(InputOutcome::Handled)
    }
    fn cancel_input(&mut self, cx: &mut EventContext) {
        self.text.value = "Input cancelled; no held actions.".into();
        cx.invalidate(Invalidation::Layout);
    }
    fn measure(
        &self,
        cx: &MeasureContext<'_>,
        available: Constraints,
    ) -> Result<Size, RuntimeError> {
        self.text.measure(cx, available)
    }
    fn arrange(&self, cx: &mut ArrangeContext<'_>, size: Size) -> Result<(), RuntimeError> {
        self.text.arrange(cx, size)
    }
    fn paint(&self, painter: &mut Painter) -> Result<(), DrawError> {
        let size = painter.size();
        painter.fill_rect(
            Rect::from_size(size),
            painter.color(ColorRole::SurfaceHover),
        );
        if self.focused {
            painter.fill_rect(
                Rect::new(
                    0.0,
                    (size.height() - 2.0).max(0.0),
                    size.width(),
                    size.height().min(2.0),
                )?,
                painter.color(ColorRole::Primary),
            );
        }
        self.text.paint(painter)
    }
    fn inspect(&self) -> ComponentDetails {
        self.text.details()
    }
}

pub(super) struct Canvas {
    mask: MaskImage,
}
impl Canvas {
    pub(super) fn new() -> Result<Self, DrawError> {
        let pixels = (0..256)
            .map(|i| if (i / 16 + i % 16) % 4 < 2 { 255 } else { 70 })
            .collect();
        Ok(Self {
            mask: MaskImage::new(16, 16, pixels)?,
        })
    }
}
impl Component for Canvas {
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
    fn measure(&self, _: &MeasureContext<'_>, _: Constraints) -> Result<Size, RuntimeError> {
        Ok(Size::from([320, 112]))
    }
    fn paint(&self, painter: &mut Painter) -> Result<(), DrawError> {
        painter.fill_rect(
            Rect::new(0.0, 0.0, 320.0, 112.0)?,
            painter.color(ColorRole::Surface),
        );
        painter.fill_rect(
            Rect::new(8.0, 8.0, 88.0, 96.0)?,
            painter.color(ColorRole::Primary),
        );
        painter.with_clip(Rect::new(112.0, 8.0, 88.0, 96.0)?, |painter| {
            painter.fill_rect(
                Rect::new(96.0, 24.0, 128.0, 64.0)?,
                painter.color(ColorRole::OnSurface),
            );
            Ok::<(), DrawError>(())
        })?;
        painter.draw_mask(
            &self.mask,
            Rect::new(232.0, 8.0, 80.0, 96.0)?,
            painter.color(ColorRole::Primary),
        );
        Ok(())
    }
}

/// Reports the status of the latest operation receipt, such as a
/// `PresentationReceipt` or a `CommandReceipt`, without polling.
///
/// This is the gallery's one custom readout, kept as an explicit example of a
/// case a mapped `Label` cannot cover. A simple readout is
/// `Label::new(signal.map(format)?)`, but here the text depends on a signal
/// held inside another: the latest receipt is replaced on every operation, and
/// each receipt carries its own status binding. Reading both in `update`
/// subscribes this component to the current receipt's status, so it repaints
/// when that operation finishes, and a new receipt moves the subscription.
pub(super) struct ReceiptReadout<R, S> {
    source: ReadSignal<Option<R>>,
    status: fn(&R) -> ReadSignal<S>,
    format: fn(&S) -> String,
    idle: String,
    text: Text,
}
impl<R, S> ReceiptReadout<R, S> {
    /// Shows `idle` until `source` holds a receipt, then its `status` binding
    /// through `format`.
    pub(super) fn new(
        source: ReadSignal<Option<R>>,
        status: fn(&R) -> ReadSignal<S>,
        format: fn(&S) -> String,
        idle: impl Into<String>,
    ) -> Self {
        Self {
            source,
            status,
            format,
            idle: idle.into(),
            text: Text::default(),
        }
    }
}
impl<R: Clone + 'static, S: Clone + 'static> Component for ReceiptReadout<R, S> {
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        self.text.value = match cx.read(&self.source)? {
            Some(receipt) => (self.format)(&cx.read(&(self.status)(&receipt))?),
            None => self.idle.clone(),
        };
        Ok(())
    }
    fn measure(&self, cx: &MeasureContext<'_>, a: Constraints) -> Result<Size, RuntimeError> {
        self.text.measure(cx, a)
    }
    fn arrange(&self, cx: &mut ArrangeContext<'_>, s: Size) -> Result<(), RuntimeError> {
        self.text.arrange(cx, s)
    }
    fn paint(&self, p: &mut Painter) -> Result<(), DrawError> {
        self.text.paint(p)
    }
    fn inspect(&self) -> ComponentDetails {
        self.text.details()
    }
}

/// Vector geometry drawn through the public `Painter` path API: fill rules, the
/// three caps and joins, ellipses, a rounded rectangle and a sampled trace. Paths
/// are rebuilt on each paint, which only happens when the theme or scale changes.
pub(super) struct VectorShapes;
impl VectorShapes {
    fn star(cx: f32, cy: f32, outer: f32) -> Result<Path, DrawError> {
        // A five-pointed star drawn in one stroke, so its center overlaps itself.
        let points = (0..5)
            .map(|i| {
                let angle = (-90.0 + 144.0 * i as f32).to_radians();
                Point::new(cx + outer * angle.cos(), cy + outer * angle.sin())
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Path::polygon(points)?)
    }
}
impl Component for VectorShapes {
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
    fn measure(&self, _: &MeasureContext<'_>, _: Constraints) -> Result<Size, RuntimeError> {
        Ok(Size::new(640.0, 180.0)?)
    }
    fn paint(&self, p: &mut Painter) -> Result<(), DrawError> {
        let point = |x: f32, y: f32| Point::new(x, y);
        let length = |value: f32| Length::new(value);
        let primary = p.color(ColorRole::Primary);
        let secondary = p.color(ColorRole::Secondary);
        let outline = p.color(ColorRole::Outline);
        // Fill rules: the same self-overlapping star, nonzero and even-odd.
        p.fill_path(&Self::star(40.0, 44.0, 34.0)?, FillRule::NonZero, primary)?;
        p.fill_path(&Self::star(120.0, 44.0, 34.0)?, FillRule::EvenOdd, primary)?;
        // Caps, left to right: butt, square and round, over a guide at the endpoints.
        for (row, cap) in [LineCap::Butt, LineCap::Square, LineCap::Round]
            .into_iter()
            .enumerate()
        {
            let y = 100.0 + 22.0 * row as f32;
            p.stroke_polyline(
                &[point(20.0, y)?, point(140.0, y)?],
                StrokeStyle::new(length(10.0)?).with_cap(cap),
                secondary,
            )?;
            p.stroke_polyline(
                &[point(20.0, y)?, point(140.0, y)?],
                StrokeStyle::new(length(1.0)?),
                outline,
            )?;
        }
        // Joins, left to right: miter, bevel and round.
        for (column, join) in [LineJoin::Miter, LineJoin::Bevel, LineJoin::Round]
            .into_iter()
            .enumerate()
        {
            let x = 180.0 + 60.0 * column as f32;
            p.stroke_polyline(
                &[
                    point(x, 70.0)?,
                    point(x + 20.0, 20.0)?,
                    point(x + 40.0, 70.0)?,
                ],
                StrokeStyle::new(length(10.0)?).with_join(join),
                secondary,
            )?;
        }
        // Ellipses and a rounded rectangle, stroked on their outlines.
        p.fill_ellipse(Rect::new(180.0, 100.0, 70.0, 50.0)?, primary)?;
        p.stroke_ellipse(
            Rect::new(270.0, 100.0, 70.0, 50.0)?,
            StrokeStyle::new(length(3.0)?),
            secondary,
        )?;
        p.stroke_path(
            &Path::rounded_rect(Rect::new(360.0, 20.0, 80.0, 50.0)?, length(12.0)?)?,
            StrokeStyle::new(length(2.0)?),
            outline,
        )?;
        // A sampled trace, the shape of a scope or graph: one mask for all points.
        let trace = (0..=160)
            .map(|i| {
                let t = i as f32 / 160.0;
                point(
                    360.0 + 260.0 * t,
                    125.0 + 35.0 * (t * 18.0).sin() * (1.0 - t),
                )
            })
            .collect::<Result<Vec<_>, _>>()?;
        p.stroke_polyline(
            &trace,
            StrokeStyle::new(length(1.5)?).with_join(LineJoin::Round),
            primary,
        )
    }
    fn inspect(&self) -> ComponentDetails {
        ComponentDetails {
            semantics: Some(
                Semantics::new(SemanticRole::Image)
                    .name("Fill rules, caps, joins, ellipses and a trace"),
            ),
            ..ComponentDetails::default()
        }
    }
}

/// A row of three children that a binding can arrange wrongly on purpose, so the
/// development overlay has something to mark: the second child then overlaps the
/// first and the third hangs out below its parent.
pub(super) struct BrokenLayout {
    broken: ReadSignal<bool>,
    arranged_broken: bool,
    children: Vec<Element>,
}
impl BrokenLayout {
    const GAP: f32 = 8.0;
    pub(super) fn new(broken: ReadSignal<bool>, children: [Element; 3]) -> Self {
        Self {
            broken,
            arranged_broken: false,
            children: children.into(),
        }
    }
    fn sizes(cx: &MeasureContext<'_>) -> Result<Vec<Size>, RuntimeError> {
        (0..cx.len())
            .map(|index| cx.measure(index, Constraints::new(None, None)?))
            .collect()
    }
}
impl Component for BrokenLayout {
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        self.arranged_broken = cx.read(&self.broken)?;
        Ok(())
    }
    fn take_children(&mut self) -> Vec<Element> {
        std::mem::take(&mut self.children)
    }
    fn clips_children(&self) -> bool {
        false
    }
    fn measure(&self, cx: &MeasureContext<'_>, _: Constraints) -> Result<Size, RuntimeError> {
        let sizes = Self::sizes(cx)?;
        let width = sizes
            .iter()
            .map(|size| size.width() + Self::GAP)
            .sum::<f32>()
            - Self::GAP;
        let height = sizes.iter().map(|size| size.height()).fold(0.0, f32::max);
        Ok(Size::new(width.max(0.0), height)?)
    }
    fn arrange(&self, cx: &mut ArrangeContext<'_>, _: Size) -> Result<(), RuntimeError> {
        let sizes = Self::sizes(cx.measurement())?;
        let mut x = 0.0;
        for (index, size) in sizes.iter().enumerate() {
            let (dx, dy) = match (self.arranged_broken, index) {
                (true, 1) => (-(sizes[0].width() / 2.0 + Self::GAP), 0.0),
                (true, 2) => (0.0, size.height() / 2.0),
                _ => (0.0, 0.0),
            };
            cx.place(index, Rect::new(x + dx, dy, size.width(), size.height())?)?;
            x += size.width() + Self::GAP;
        }
        Ok(())
    }
}
