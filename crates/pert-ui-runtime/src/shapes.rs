//! Color-independent, bounded UI-thread shape coverage.
use crate::{Color, DrawError, Length, MaskImage, Painter, Point, Rect};
use std::collections::VecDeque;

/// The shape cache budget in bytes, a policy separate from mask size.
const CACHE_BYTES: usize = 16 * 1024 * 1024;
/// Arc coverage samples a square grid of this many points per pixel side.
const ARC_GRID: u32 = 4;
/// Shape coverage samples this many rows per pixel, each with analytic
/// horizontal coverage.
const SHAPE_ROWS: usize = 4;

/// Backend-independent rectangle silhouette. Dimensions are logical pixels.
/// Corners clamp to the available rectangle. The shape affects painting only;
/// widgets retain their rectangular layout and hit bounds.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum Shape {
    /// Four square corners.
    #[default]
    Rectangle,
    /// Equal circular corners; radius clamps to half the smaller side.
    Rounded(Length),
    /// A 45-degree cut at the upper right; cut clamps to the smaller side.
    CutTopRight(Length),
    /// Circular upper corners and square lower corners, for silhouettes such as
    /// tabs that attach to a boundary below. Radius clamps to half the smaller side.
    RoundedTop(Length),
}
#[derive(Clone, Copy, PartialEq)]
enum Coverage {
    Shape(Shape),
    Triangle([Point; 3]),
    Arc { start: f32, sweep: f32 },
}
#[derive(Clone, Copy, PartialEq)]
struct Key {
    width: f32,
    height: f32,
    scale: f64,
    shape: Coverage,
    stroke: Option<f32>,
}
#[derive(Default)]
pub(crate) struct ShapeCache {
    entries: VecDeque<(Key, MaskImage)>,
    bytes: usize,
}
impl ShapeCache {
    fn image(&mut self, key: Key) -> Result<MaskImage, DrawError> {
        if let Some((_, image)) = self.entries.iter().find(|(old, _)| *old == key) {
            return Ok(image.clone());
        }
        let width = (f64::from(key.width) * key.scale).ceil();
        let height = (f64::from(key.height) * key.scale).ceil();
        if !width.is_finite()
            || !height.is_finite()
            || width < 1.0
            || height < 1.0
            || width * height > MaskImage::MAX_PIXELS as f64
        {
            return Err(DrawError::InvalidImage);
        }
        let (width, height) = (width as u32, height as u32);
        let mut pixels = vec![0; width as usize * height as usize];
        let sx = f64::from(width) / f64::from(key.width);
        let sy = f64::from(height) / f64::from(key.height);
        if let Coverage::Arc { start, sweep } = key.shape {
            let half_stroke = f64::from(key.stroke.unwrap_or(0.0)) * 0.5;
            let center = f64::from(key.width) * 0.5;
            let radius = (center - half_stroke).max(0.0);
            let start = f64::from(start).to_radians();
            let sweep = f64::from(sweep).to_radians();
            let end = start + sweep;
            let endpoints = [
                (radius * start.cos(), radius * start.sin()),
                (radius * end.cos(), radius * end.sin()),
            ];
            for y in 0..height {
                for x in 0..width {
                    let mut covered = 0;
                    let grid = f64::from(ARC_GRID);
                    for dy in 0..ARC_GRID {
                        for dx in 0..ARC_GRID {
                            let px = (f64::from(x) + (f64::from(dx) + 0.5) / grid) / sx - center;
                            let py = (f64::from(y) + (f64::from(dy) + 0.5) / grid) / sy - center;
                            let distance = px.hypot(py);
                            if (distance - radius).abs() > half_stroke {
                                continue;
                            }
                            let angle = (py.atan2(px) - start).rem_euclid(std::f64::consts::TAU);
                            if angle <= sweep
                                || endpoints
                                    .iter()
                                    .any(|(ex, ey)| (px - ex).hypot(py - ey) <= half_stroke)
                            {
                                covered += 1;
                            }
                        }
                    }
                    pixels[y as usize * width as usize + x as usize] =
                        (covered * 255 / (ARC_GRID * ARC_GRID)) as u8;
                }
            }
            let image = MaskImage::new(width, height, pixels)?;
            self.insert(key, image.clone());
            return Ok(image);
        }
        for y in 0..height {
            // Vertical samples and analytic horizontal interval coverage.
            let spans = std::array::from_fn::<_, SHAPE_ROWS, _>(|sample| {
                let y = (f64::from(y) + (sample as f64 + 0.5) / SHAPE_ROWS as f64) / sy;
                (
                    span(key, y, 0.0),
                    key.stroke
                        .and_then(|stroke| span(key, y, f64::from(stroke))),
                )
            });
            for x in 0..width {
                let coverage = spans
                    .iter()
                    .map(|(outer, inner)| {
                        let overlap = |span: Option<(f64, f64)>| {
                            span.map_or(0.0, |(a, b)| {
                                ((b * sx).min(f64::from(x) + 1.0) - (a * sx).max(f64::from(x)))
                                    .clamp(0.0, 1.0)
                            })
                        };
                        (overlap(*outer) - overlap(*inner)).max(0.0)
                    })
                    .sum::<f64>()
                    / SHAPE_ROWS as f64;
                pixels[y as usize * width as usize + x as usize] = (coverage * 255.0).round() as u8;
            }
        }
        let image = MaskImage::new(width, height, pixels)?;
        self.insert(key, image.clone());
        Ok(image)
    }
    fn insert(&mut self, key: Key, image: MaskImage) {
        while self.entries.len() >= 128 || self.bytes + image.pixels().len() > CACHE_BYTES {
            if let Some((_, old)) = self.entries.pop_front() {
                self.bytes -= old.pixels().len();
            } else {
                break;
            }
        }
        self.bytes += image.pixels().len();
        self.entries.push_back((key, image.clone()));
    }
}
fn span(key: Key, y: f64, inset: f64) -> Option<(f64, f64)> {
    let (w, h) = (f64::from(key.width), f64::from(key.height));
    if y < inset || y >= h - inset || 2.0 * inset >= w.min(h) {
        return None;
    }
    let (mut left, mut right) = (inset, w - inset);
    match key.shape {
        Coverage::Arc { .. } => return None,
        Coverage::Triangle(vertices) => {
            let mut crossings = [0.0; 3];
            let mut count = 0;
            for i in 0..3 {
                let a = vertices[i];
                let b = vertices[(i + 1) % 3];
                let (ay, by) = (f64::from(a.y()), f64::from(b.y()));
                if (ay <= y && y < by) || (by <= y && y < ay) {
                    crossings[count] = f64::from(a.x())
                        + (y - ay) / (by - ay) * (f64::from(b.x()) - f64::from(a.x()));
                    count += 1;
                }
            }
            return (count == 2).then(|| {
                (
                    crossings[0].min(crossings[1]),
                    crossings[0].max(crossings[1]),
                )
            });
        }
        Coverage::Shape(Shape::Rectangle) => {}
        Coverage::Shape(Shape::Rounded(radius) | Shape::RoundedTop(radius)) => {
            let radius = (f64::from(radius.get()).min(w.min(h) / 2.0) - inset).max(0.0);
            let distance = if matches!(key.shape, Coverage::Shape(Shape::RoundedTop(_))) {
                y - inset
            } else {
                (y - inset).min(h - inset - y)
            };
            let dy = (radius - distance).max(0.0);
            let cut = radius - (radius * radius - dy * dy).max(0.0).sqrt();
            left += cut;
            right -= cut;
        }
        Coverage::Shape(Shape::CutTopRight(cut)) => {
            let cut = f64::from(cut.get()).min(w.min(h));
            right = right.min(w - cut + y - inset * std::f64::consts::SQRT_2);
        }
    }
    (right > left).then_some((left, right))
}
impl Painter {
    /// Strokes a circular arc with round end caps in local logical coordinates.
    /// Angles are degrees clockwise from the rightward radius in screen coordinates.
    /// `start_degrees` may be any finite angle; `sweep_degrees` must be in 0..=360.
    /// Radius is measured to the stroke center. Zero sweep or width draws nothing;
    /// 360 degrees draws a complete ring. The stroke extends half its width on
    /// either side of the radius. Layout and hit bounds remain the widget's own.
    ///
    /// Coverage uses sixteen samples per mask pixel at effective DPI/zoom and the
    /// shared 128-entry/16 MiB shape cache. Color/translation reuse coverage; angle,
    /// size or stroke changes may rasterize a new bounded mask on the UI thread.
    /// No
    /// backend types are exposed.
    ///
    /// ```
    /// use pert_ui_runtime::{ColorRole, Component, DrawError, Element, Length,
    ///     Painter, Point, Runtime, RuntimeError, UpdateContext, Viewport};
    /// struct ArcWidget;
    /// impl Component for ArcWidget {
    ///     fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> { Ok(()) }
    ///     fn paint(&self, p: &mut Painter) -> Result<(), DrawError> {
    ///         p.stroke_arc(Point::new(20.0, 20.0)?, Length::new(16.0)?, 135.0, 180.0,
    ///             Length::new(2.0)?, p.color(ColorRole::Primary))
    ///     }
    /// }
    /// let mut ui = Runtime::new();
    /// ui.set_viewport(Viewport::new(40.0, 40.0, 1.0)?)?;
    /// ui.mount_tree(Element::new("arc", ArcWidget))?;
    /// ui.flush(std::time::Duration::ZERO)?;
    /// ui.paint()?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// # Errors
    /// Returns [`DrawError::Path`] with
    /// [`PathError::InvalidParameter`](crate::PathError::InvalidParameter) for a non-finite
    /// angle or a sweep outside `0..=360`, [`DrawError::Runtime`] holding
    /// [`RuntimeError::Geometry`](crate::RuntimeError::Geometry) for unrepresentable
    /// bounds, and [`DrawError::InvalidImage`] when the mask exceeds
    /// [`MaskImage::MAX_PIXELS`](crate::MaskImage::MAX_PIXELS).
    pub fn stroke_arc(
        &mut self,
        center: Point,
        radius: Length,
        start_degrees: f32,
        sweep_degrees: f32,
        width: Length,
        color: Color,
    ) -> Result<(), DrawError> {
        if !start_degrees.is_finite()
            || !sweep_degrees.is_finite()
            || !(0.0..=360.0).contains(&sweep_degrees)
        {
            return Err(crate::PathError::InvalidParameter.into());
        }
        if sweep_degrees == 0.0 || width.get() == 0.0 {
            return Ok(());
        }
        let extent = radius.get() + width.get() * 0.5;
        let rect = Rect::new(
            center.x() - extent,
            center.y() - extent,
            extent * 2.0,
            extent * 2.0,
        )?;
        let image = self.shapes.borrow_mut().image(Key {
            width: extent * 2.0,
            height: extent * 2.0,
            scale: self.viewport().scale(),
            shape: Coverage::Arc {
                start: start_degrees.rem_euclid(360.0),
                sweep: sweep_degrees,
            },
            stroke: Some(width.get()),
        })?;
        self.draw_mask(&image, rect, color);
        Ok(())
    }
    /// Fills a triangle defined by three local logical points, in either winding order.
    /// Collinear/coincident points draw nothing. Coverage is antialiased at the
    /// current DPI/zoom and shares the bounded 128-entry, 16 MiB shape cache.
    /// Translation and color changes reuse coverage; clipping preserves coordinates.
    /// No backend types or
    /// application-side cache management are required.
    ///
    /// # Errors
    /// Returns [`DrawError::Runtime`] holding
    /// [`RuntimeError::Geometry`](crate::RuntimeError::Geometry) for an unrepresentable
    /// bounding rectangle and [`DrawError::InvalidImage`] when the bounding mask exceeds
    /// [`MaskImage::MAX_PIXELS`](crate::MaskImage::MAX_PIXELS).
    pub fn fill_triangle(&mut self, vertices: [Point; 3], color: Color) -> Result<(), DrawError> {
        let [a, b, c] = vertices;
        let dx = |p: Point| f64::from(p.x()) - f64::from(a.x());
        let dy = |p: Point| f64::from(p.y()) - f64::from(a.y());
        if dx(b) * dy(c) - dy(b) * dx(c) == 0.0 {
            return Ok(());
        }
        let x = a.x().min(b.x()).min(c.x());
        let y = a.y().min(b.y()).min(c.y());
        let width = a.x().max(b.x()).max(c.x()) - x;
        let height = a.y().max(b.y()).max(c.y()) - y;
        let rect = Rect::new(x, y, width, height)?;
        let mut local = vertices;
        for point in &mut local {
            *point = Point::new(point.x() - x, point.y() - y)?;
        }
        let image = self.shapes.borrow_mut().image(Key {
            width,
            height,
            scale: self.viewport().scale(),
            shape: Coverage::Triangle(local),
            stroke: None,
        })?;
        self.draw_mask(&image, rect, color);
        Ok(())
    }
    /// Fills a silhouette with antialiased, color-independent coverage.
    /// Coverage is prepared on the UI thread at effective DPI/zoom, cached per
    /// runtime (128 entries, 16 MiB), and reused across color changes. Live display
    /// lists can retain evicted masks. No backend or OS types are required.
    /// Ordinary shapes decompose into rectangles and cached corner masks, so large
    /// panels do not allocate full-panel images. Zero area draws nothing.
    /// Strokes with a cut
    /// larger than half the shorter side use full coverage, under the same limit.
    ///
    /// # Errors
    /// Returns [`DrawError::InvalidImage`] when a corner or full-coverage mask exceeds
    /// [`MaskImage::MAX_PIXELS`](crate::MaskImage::MAX_PIXELS), and [`DrawError::Runtime`] holding
    /// [`RuntimeError::Geometry`](crate::RuntimeError::Geometry) for derived rectangles or
    /// corner sizes that are not valid geometry.
    pub fn fill_shape(&mut self, rect: Rect, shape: Shape, color: Color) -> Result<(), DrawError> {
        if shape == Shape::Rectangle {
            self.fill_rect(rect, color);
            return Ok(());
        }
        self.shape(rect, shape, color, None)
    }
    /// Draws the keyboard focus ring inward from `rect` along `shape`, using
    /// `FocusRing` at `FocusWidth`. A positive stroke stays at least one
    /// physical pixel wide at fractional scale.
    pub(crate) fn focus_ring(&mut self, rect: Rect, shape: Shape) -> Result<(), DrawError> {
        self.stroke_shape(
            rect,
            shape,
            Length::new(self.metric(crate::MetricRole::FocusWidth))?,
            self.color(crate::ColorRole::FocusRing),
        )
    }
    /// Strokes a silhouette inward, without changing layout or painting its center.
    /// Width is a validated logical length. A positive width below one physical
    /// pixel is drawn one physical pixel wide so its straight sides remain visible
    /// at fractional scales; zero width/area draws nothing. Thick strokes fill the
    /// shape when there is no interior. Clipping preserves shape coordinates. Uses
    /// the same bounded coverage cache as `fill_shape`.
    ///
    /// # Errors
    /// As [`Self::fill_shape`].
    pub fn stroke_shape(
        &mut self,
        rect: Rect,
        shape: Shape,
        width: Length,
        color: Color,
    ) -> Result<(), DrawError> {
        if width.get() == 0.0 {
            return Ok(());
        }
        let min_width = (1.0 / self.viewport().scale()).min(f64::from(f32::MAX)) as f32;
        self.shape(rect, shape, color, Some(width.get().max(min_width)))
    }
    fn shape(
        &mut self,
        rect: Rect,
        shape: Shape,
        color: Color,
        stroke: Option<f32>,
    ) -> Result<(), DrawError> {
        let [x, y, w, h] = rect.values();
        if w == 0.0 || h == 0.0 {
            return Ok(());
        }
        let half = w.min(h) / 2.0;
        if stroke.is_some_and(|width| width >= half) {
            return self.fill_shape(rect, shape, color);
        }
        if shape == Shape::Rectangle {
            if let Some(s) = stroke {
                for rect in [
                    Rect::new(x, y, w, s)?,
                    Rect::new(x, y + h - s, w, s)?,
                    Rect::new(x, y + s, s, h - 2.0 * s)?,
                    Rect::new(x + w - s, y + s, s, h - 2.0 * s)?,
                ] {
                    self.fill_rect(rect, color);
                }
            } else {
                self.fill_rect(rect, color);
            }
            return Ok(());
        }
        let corner = match shape {
            Shape::Rounded(radius) | Shape::RoundedTop(radius) => radius.get().min(half),
            Shape::CutTopRight(cut) => cut.get().min(w.min(h)),
            Shape::Rectangle => 0.0,
        };
        if corner == 0.0 {
            return self.shape(rect, Shape::Rectangle, color, stroke);
        }
        if matches!(shape, Shape::CutTopRight(_)) && stroke.is_none() {
            // Only the cut corner needs coverage, independent of the panel extent.
            self.fill_rect(Rect::new(x, y, w - corner, h)?, color);
            self.fill_rect(
                Rect::new(x + w - corner, y + corner, corner, h - corner)?,
                color,
            );
            return self.mask_shape(
                Rect::new(x + w - corner, y, corner, corner)?,
                Shape::CutTopRight(Length::new(corner)?),
                color,
                None,
            );
        }
        let edge = corner.max(stroke.unwrap_or(0.0));
        if edge > half {
            // Unusually large cuts can intersect opposite corners. Preserve the
            // exact silhouette with bounded full coverage in that explicit case.
            return self.mask_shape(rect, shape, color, stroke);
        }
        let shape = match shape {
            Shape::Rounded(_) => Shape::Rounded(Length::new(corner)?),
            Shape::RoundedTop(_) => Shape::RoundedTop(Length::new(corner)?),
            _ => shape,
        };
        if let Some(s) = stroke {
            for rect in [
                Rect::new(x + edge, y, w - 2.0 * edge, s)?,
                Rect::new(x + edge, y + h - s, w - 2.0 * edge, s)?,
                Rect::new(x, y + edge, s, h - 2.0 * edge)?,
                Rect::new(x + w - s, y + edge, s, h - 2.0 * edge)?,
            ] {
                self.fill_rect(rect, color);
            }
        } else {
            self.fill_rect(Rect::new(x + edge, y, w - 2.0 * edge, h)?, color);
            self.fill_rect(Rect::new(x, y + edge, edge, h - 2.0 * edge)?, color);
            self.fill_rect(
                Rect::new(x + w - edge, y + edge, edge, h - 2.0 * edge)?,
                color,
            );
        }
        for (right, bottom) in [(false, false), (true, false), (false, true), (true, true)] {
            let cx = if right { x + w - edge } else { x };
            let cy = if bottom { y + h - edge } else { y };
            let template = Rect::new(
                if right { cx - edge } else { cx },
                if bottom { cy - edge } else { cy },
                edge * 2.0,
                edge * 2.0,
            )?;
            let shape = if (matches!(shape, Shape::CutTopRight(_)) && (!right || bottom))
                || (matches!(shape, Shape::RoundedTop(_)) && bottom)
            {
                Shape::Rectangle
            } else {
                shape
            };
            self.with_clip(Rect::new(cx, cy, edge, edge)?, |painter| {
                painter.mask_shape(template, shape, color, stroke)
            })?;
        }
        Ok(())
    }
    fn mask_shape(
        &mut self,
        rect: Rect,
        shape: Shape,
        color: Color,
        stroke: Option<f32>,
    ) -> Result<(), DrawError> {
        let [_, _, width, height] = rect.values();
        let key = Key {
            width,
            height,
            scale: self.viewport().scale(),
            shape: Coverage::Shape(shape),
            stroke,
        };
        let image = self.shapes.borrow_mut().image(key)?;
        self.draw_mask(&image, rect, color);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ColorRole, Component, Constraints, DrawCommand, MeasureContext, Palette, Runtime,
        RuntimeError, Size, UpdateContext, Viewport,
    };
    use std::time::Duration;

    struct FocusRingScene;

    impl Component for FocusRingScene {
        fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
            Ok(())
        }

        fn measure(
            &self,
            _: &MeasureContext<'_>,
            available: Constraints,
        ) -> Result<Size, RuntimeError> {
            Ok(Size::new(
                available.width().unwrap_or(0.0),
                available.height().unwrap_or(0.0),
            )?)
        }

        fn paint(&self, painter: &mut Painter) -> Result<(), DrawError> {
            painter.focus_ring(
                Rect::new(10.0, 10.0, 100.0, 30.0)?,
                Shape::Rounded(Length::new(4.0)?),
            )
        }
    }

    #[test]
    fn focus_ring_has_one_color_at_any_zoom() -> Result<(), Box<dyn std::error::Error>> {
        let palette = Palette::dark();
        let ring = palette.color(ColorRole::FocusRing);
        let contrast = palette.color(ColorRole::FocusRingContrast);
        for scale in [0.23, 0.69, 0.75, 0.99, 1.0, 1.25] {
            let mut ui = Runtime::new();
            ui.set_viewport(Viewport::new(120.0, 50.0, scale)?)?;
            ui.mount("ring", FocusRingScene)?;
            ui.flush(Duration::ZERO)?;
            let scene = ui.paint()?;
            let colors: Vec<_> = scene
                .commands()
                .iter()
                .filter_map(|command| match command {
                    DrawCommand::FillRect { color, .. } | DrawCommand::DrawMask { color, .. } => {
                        Some(*color)
                    }
                    _ => None,
                })
                .collect();
            assert!(colors.contains(&ring), "focus ring at scale {scale}");
            assert!(
                !colors.contains(&contrast),
                "contrast band at scale {scale}"
            );
        }
        Ok(())
    }

    #[test]
    fn byte_budget_evicts_large_entries_but_live_masks_remain_valid() {
        let mut cache = ShapeCache::default();
        let key = Key {
            width: 3072.0,
            height: 3072.0,
            scale: 1.0,
            shape: Coverage::Shape(Shape::Rounded(4.into())),
            stroke: None,
        };
        let large = MaskImage::new(3072, 3072, vec![255; 3072 * 3072]).unwrap();
        cache.insert(key, large.clone());
        cache.insert(
            Key {
                stroke: Some(1.0),
                ..key
            },
            large.clone(),
        );
        assert_eq!(cache.entries.len(), 1);
        assert_eq!(cache.bytes, 3072 * 3072);
        let exact = MaskImage::new(4096, 4096, vec![0; 4096 * 4096]).unwrap();
        cache.insert(
            Key {
                width: 4096.0,
                height: 4096.0,
                ..key
            },
            exact.clone(),
        );
        assert_eq!(cache.bytes, CACHE_BYTES);
        cache
            .image(Key {
                width: 8.0,
                height: 8.0,
                ..key
            })
            .unwrap();
        assert_eq!(cache.entries.len(), 1);
        assert_eq!(cache.bytes, 64);
        assert_eq!(large.pixels()[0], 255);
        assert_eq!(exact.pixels().len(), MaskImage::MAX_PIXELS);
    }
    #[test]
    fn coverage_cache_reuses_masks_bounds_entries_and_rejects_oversized_requests() {
        let mut cache = ShapeCache::default();
        let key = Key {
            width: 16.0,
            height: 16.0,
            scale: 1.25,
            shape: Coverage::Shape(Shape::Rounded(4.into())),
            stroke: Some(1.0),
        };
        let first = cache.image(key).unwrap();
        assert_eq!(
            first.pixels().as_ptr(),
            cache.image(key).unwrap().pixels().as_ptr()
        );
        for width in 17..150 {
            cache
                .image(Key {
                    width: width as f32,
                    ..key
                })
                .unwrap();
        }
        assert_eq!(cache.entries.len(), 128);
        assert!(cache.bytes <= CACHE_BYTES);
        assert!(first.pixels().iter().any(|alpha| *alpha > 0));
        assert!(
            cache
                .image(Key {
                    width: 4096.0,
                    height: 4097.0,
                    scale: 1.0,
                    ..key
                })
                .is_err()
        );
        assert!(
            cache
                .image(Key {
                    width: f32::MAX,
                    scale: f64::MAX,
                    ..key
                })
                .is_err()
        );
    }
}
