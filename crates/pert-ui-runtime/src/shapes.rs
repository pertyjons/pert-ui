//! Shapes drawn as one command each: rectangles with round or cut corners,
//! arcs and triangles, whose coverage a renderer computes per pixel.
use crate::{Color, DrawCommand, DrawError, Length, Painter, Point, Rect};

mod prepare;
pub use prepare::{
    ArcGeometry, Border, Boundary, Corner, CornerKind, CornerPosition, FrameMapping, Inner, Paint,
    PreparedCorner, PreparedShape, Shape, ShapeCoverage, ShapeGeometry, ShapeParameters,
    ShapePrepareError, ShapeStyle, TriangleGeometry,
};

impl Painter {
    /// Strokes a circular arc with round end caps in local logical coordinates.
    /// Angles are degrees clockwise from the rightward radius in screen coordinates.
    /// `start_degrees` may be any finite angle; `sweep_degrees` must be in 0..=360.
    /// Radius is measured to the stroke center. Zero sweep or width draws nothing;
    /// 360 degrees draws a complete ring. The stroke extends half its width on
    /// either side of the radius. Layout and hit bounds remain the widget's own.
    /// The width is not snapped to whole pixels. A stroke narrower than one
    /// physical pixel is drawn one pixel wide with its coverage scaled by its
    /// width, so it fades out as it narrows.
    ///
    /// The arc is recorded as one [`DrawCommand::DrawShape`]: a renderer
    /// computes its coverage per pixel, so nothing is rasterized or cached on
    /// the UI thread, and a new angle, size or width costs the same as a
    /// repeated one. No backend types are exposed.
    ///
    /// ```
    /// # use pert_ui_runtime as pert_ui;
    /// use pert_ui::{ColorRole, Component, DrawCommand, DrawError, Element, Length,
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
    /// let scene = ui.paint()?;
    /// assert!(matches!(scene.commands(), [DrawCommand::DrawShape { .. }]));
    /// assert!(scene.masks().is_empty());
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// # Errors
    /// Returns [`DrawError::Path`] with
    /// [`PathError::InvalidParameter`](crate::PathError::InvalidParameter) for a non-finite
    /// angle or a sweep outside `0..=360`, and [`DrawError::Runtime`] holding
    /// [`RuntimeError::Geometry`](crate::RuntimeError::Geometry) for unrepresentable
    /// bounds.
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
        Rect::new(
            center.x() - extent,
            center.y() - extent,
            extent * 2.0,
            extent * 2.0,
        )?;
        let arc = ArcGeometry::new(center, radius, start_degrees, sweep_degrees, width)?;
        self.list.fills.push(DrawCommand::DrawShape {
            geometry: ShapeGeometry::Arc(arc),
            fill: Some(Paint::Solid(color)),
            border: None,
            clip: self.clip,
        });
        Ok(())
    }
    /// Fills a triangle defined by three local logical points, in either winding order.
    /// Collinear/coincident points draw nothing. The triangle is recorded as
    /// one [`DrawCommand::DrawShape`], antialiased per pixel by the renderer
    /// at the current DPI/zoom; nothing is rasterized or cached on the UI
    /// thread. Clipping preserves coordinates. No backend types or
    /// application-side cache management are required.
    ///
    /// # Errors
    /// Returns [`DrawError::Runtime`] holding
    /// [`RuntimeError::Geometry`](crate::RuntimeError::Geometry) for an unrepresentable
    /// bounding rectangle.
    pub fn fill_triangle(&mut self, vertices: [Point; 3], color: Color) -> Result<(), DrawError> {
        // Collinear points draw nothing, whatever their extent.
        let Some(triangle) = TriangleGeometry::new(vertices) else {
            return Ok(());
        };
        let [a, b, c] = vertices;
        let x = a.x().min(b.x()).min(c.x());
        let y = a.y().min(b.y()).min(c.y());
        let width = a.x().max(b.x()).max(c.x()) - x;
        let height = a.y().max(b.y()).max(c.y()) - y;
        Rect::new(x, y, width, height)?;
        self.list.fills.push(DrawCommand::DrawShape {
            geometry: ShapeGeometry::Triangle(triangle),
            fill: Some(Paint::Solid(color)),
            border: None,
            clip: self.clip,
        });
        Ok(())
    }
    /// Fills a silhouette with antialiased coverage. A shape whose corners
    /// all have size zero, such as [`Shape::RECTANGLE`], is recorded as a
    /// [`DrawCommand::FillRect`], as [`Self::fill_rect`] records it; any other shape is
    /// recorded as [`Self::draw_shape`] records a fill, with that method's
    /// pixel snapping and corner rules. Zero area draws nothing. No backend
    /// or OS types are required.
    ///
    /// Fill and then [`Self::stroke_shape`] are two commands, each
    /// antialiased on its own; [`Self::draw_shape`] draws both as one
    /// silhouette.
    ///
    /// # Errors
    /// As [`Self::draw_shape`].
    pub fn fill_shape(&mut self, rect: Rect, shape: Shape, color: Color) -> Result<(), DrawError> {
        let square = [
            CornerPosition::TopLeft,
            CornerPosition::TopRight,
            CornerPosition::BottomRight,
            CornerPosition::BottomLeft,
        ]
        .iter()
        .all(|at| shape.corner(*at).size().get() == 0.0);
        if square {
            self.fill_rect(rect, color);
            return Ok(());
        }
        self.draw_shape(rect, shape, ShapeStyle::fill(color))
    }
    /// Draws a silhouette's fill, its border, or both as **one** command: the
    /// border lies inside `rect` over the fill, and the two are composited as
    /// one covered silhouette, so no background shows between them. Fill and
    /// then [`Self::stroke_shape`] are two commands, each antialiased on its
    /// own.
    ///
    /// The command is [`DrawCommand::DrawShape`]: a backend computes coverage
    /// per pixel from the geometry, so nothing is rasterized or cached on the
    /// UI thread, and any size, corner or border costs the same. The
    /// rectangle's edges snap to physical pixels as a filled rectangle's do,
    /// the border is a whole number of physical pixels wide (at least one for
    /// a positive width), and corners that do not fit are scaled down together
    /// as CSS does. A zero-area rectangle, a style with neither a fill nor a
    /// border of positive width, draws nothing.
    ///
    /// ```
    /// # use pert_ui_runtime as pert_ui;
    /// use pert_ui::{ColorRole, Component, Corner, CornerPosition, DrawCommand, DrawError,
    ///     Element, Length, Painter, Rect, Runtime, RuntimeError, Shape, ShapeStyle,
    ///     UpdateContext, Viewport};
    /// struct Card;
    /// impl Component for Card {
    ///     fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> { Ok(()) }
    ///     fn paint(&self, p: &mut Painter) -> Result<(), DrawError> {
    ///         let shape = Shape::rounded(Length::new(8.0)?)
    ///             .with_corner(CornerPosition::TopRight, Corner::cut(Length::new(12.0)?));
    ///         let style = ShapeStyle::fill(p.color(ColorRole::SurfaceContainer))
    ///             .with_border(Length::new(1.0)?, p.color(ColorRole::Outline));
    ///         p.draw_shape(Rect::new(10.0, 10.0, 200.0, 120.0)?, shape, style)
    ///     }
    /// }
    /// let mut ui = Runtime::new();
    /// ui.set_viewport(Viewport::new(240.0, 160.0, 1.0)?)?;
    /// ui.mount_tree(Element::new("card", Card))?;
    /// ui.flush(std::time::Duration::ZERO)?;
    /// let scene = ui.paint()?;
    /// assert!(matches!(scene.commands(), [DrawCommand::DrawShape { .. }]));
    /// assert!(scene.masks().is_empty());
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// # Errors
    /// None today; the result is reserved for geometry a later paint kind
    /// cannot represent.
    pub fn draw_shape(
        &mut self,
        rect: Rect,
        shape: Shape,
        style: ShapeStyle,
    ) -> Result<(), DrawError> {
        let [_, _, w, h] = rect.values();
        let border = style.border.filter(|border| border.width().get() > 0.0);
        if w == 0.0 || h == 0.0 || (style.fill.is_none() && border.is_none()) {
            return Ok(());
        }
        self.list.fills.push(DrawCommand::DrawShape {
            geometry: ShapeGeometry::Rect { rect, shape },
            fill: style.fill,
            border,
            clip: self.clip,
        });
        Ok(())
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
    /// Width is a validated logical length.
    ///
    /// The whole stroke lies inside `rect`. Path strokes, such as
    /// [`Self::stroke_path`], [`Self::stroke_polyline`] and
    /// [`Self::stroke_ellipse`], are centered on their outline instead, so half
    /// their width lies outside it; they take a
    /// [`StrokeStyle`](crate::StrokeStyle) for caps and joins and keep a width
    /// below one physical pixel as faint coverage rather than widening it. To
    /// keep a path stroke inside the same rectangle, stroke a path inset by
    /// half its width; the two agree in placement, not pixel for pixel.
    ///
    /// The stroke is recorded as [`Self::draw_shape`] records a border: its
    /// width is a whole number of physical pixels, at least one for a
    /// positive width, so its straight sides remain visible at fractional
    /// scales; zero width or area draws nothing. A stroke with no interior
    /// left fills the shape. Clipping preserves shape coordinates.
    ///
    /// # Errors
    /// As [`Self::draw_shape`].
    pub fn stroke_shape(
        &mut self,
        rect: Rect,
        shape: Shape,
        width: Length,
        color: Color,
    ) -> Result<(), DrawError> {
        #[cfg(test)]
        if width.get() != 0.0 && faults::fails(color) {
            return Err(DrawError::CoverageTooLarge);
        }
        self.draw_shape(rect, shape, ShapeStyle::border(width, color))
    }
}

/// Fault injection for the crate's own tests: a stroke in the chosen color
/// fails, so a test can show what a component keeps when a paint operation
/// after its first ones fails. Not compiled outside tests.
#[cfg(test)]
pub(crate) mod faults {
    use std::cell::Cell;

    thread_local! {
        static FAILING_STROKE: Cell<Option<[f32; 4]>> = const { Cell::new(None) };
    }

    /// Makes every stroke in `color` on this thread fail until the returned
    /// guard is dropped, also when a test panics.
    #[must_use = "strokes fail only while the guard lives"]
    pub(crate) fn fail_strokes_in(color: crate::Color) -> FailingStrokes {
        FAILING_STROKE.with(|cell| cell.set(Some(color.channels())));
        FailingStrokes(())
    }

    /// Ends [`fail_strokes_in`] when dropped.
    #[derive(Debug)]
    pub(crate) struct FailingStrokes(());

    impl Drop for FailingStrokes {
        fn drop(&mut self) {
            FAILING_STROKE.with(|cell| cell.set(None));
        }
    }

    pub(super) fn fails(color: crate::Color) -> bool {
        FAILING_STROKE.with(|cell| cell.get() == Some(color.channels()))
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
                Shape::rounded(Length::new(4.0)?),
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
            ui.mount_tree(crate::Element::new("ring", FocusRingScene))?;
            ui.flush(Duration::ZERO)?;
            let scene = ui.paint()?;
            let colors: Vec<_> = scene
                .commands()
                .iter()
                .filter_map(|command| match command {
                    DrawCommand::DrawShape {
                        border: Some(border),
                        ..
                    } => {
                        let Paint::Solid(color) = border.paint();
                        Some(color)
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

    /// A scene that paints whatever its closure draws.
    type Paints = Box<dyn Fn(&mut Painter) -> Result<(), DrawError>>;
    struct Drawn(Paints);

    impl Component for Drawn {
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
            (self.0)(painter)
        }
    }

    fn drawn(
        paint: impl Fn(&mut Painter) -> Result<(), DrawError> + 'static,
    ) -> Result<crate::DisplayList, Box<dyn std::error::Error>> {
        let mut ui = Runtime::new();
        ui.set_viewport(Viewport::new(200.0, 100.0, 1.0)?)?;
        ui.mount_tree(crate::Element::new("scene", Drawn(Box::new(paint))))?;
        ui.flush(Duration::ZERO)?;
        Ok(ui.paint()?)
    }

    fn shape_commands(scene: &crate::DisplayList) -> usize {
        scene
            .commands()
            .iter()
            .filter(|command| matches!(command, DrawCommand::DrawShape { .. }))
            .count()
    }

    #[test]
    fn draw_shape_is_one_command_without_coverage() -> Result<(), Box<dyn std::error::Error>> {
        let scene = drawn(|p| {
            let rect = Rect::new(10.0, 10.0, 100.0, 40.0)?;
            let shape = Shape::rounded(Length::new(8.0)?);
            let color = p.color(ColorRole::Primary);
            p.draw_shape(
                rect,
                shape,
                ShapeStyle::fill(color).with_border(Length::new(2.0)?, color),
            )?;
            // Nothing to draw: no area, no paint, or only a zero-width border.
            p.draw_shape(
                Rect::new(10.0, 10.0, 0.0, 40.0)?,
                shape,
                ShapeStyle::fill(color),
            )?;
            p.draw_shape(rect, shape, ShapeStyle::border(Length::new(0.0)?, color))
        })?;
        let [
            DrawCommand::DrawShape {
                geometry,
                fill,
                border,
                clip,
            },
        ] = scene.commands()
        else {
            return Err(format!("{:?}", scene.commands()).into());
        };
        assert!(matches!(geometry, ShapeGeometry::Rect { .. }));
        assert!(fill.is_some() && border.is_some_and(|border| border.width().get() == 2.0));
        // Composition gives the command its component's clip.
        assert_eq!(clip.map(Rect::values), Some([0.0, 0.0, 200.0, 100.0]));
        assert!(scene.masks().is_empty() && scene.images().is_empty());
        Ok(())
    }

    #[test]
    fn every_shape_method_records_one_shape_command() -> Result<(), Box<dyn std::error::Error>> {
        let scene = drawn(|p: &mut Painter| -> Result<(), DrawError> {
            let color = p.color(ColorRole::Primary);
            let rect = Rect::new(10.0, 10.0, 100.0, 40.0)?;
            p.fill_shape(rect, Shape::rounded(Length::new(8.0)?), color)?;
            p.stroke_shape(
                rect,
                Shape::rounded_top(Length::new(8.0)?),
                Length::new(1.0)?,
                color,
            )?;
            p.stroke_shape(rect, Shape::RECTANGLE, Length::new(1.0)?, color)?;
            p.focus_ring(rect, Shape::cut_top_right(Length::new(6.0)?))?;
            p.stroke_arc(
                Point::new(150.0, 50.0)?,
                Length::new(20.0)?,
                0.0,
                200.0,
                Length::new(3.0)?,
                color,
            )?;
            p.fill_triangle(
                [
                    Point::new(1.0, 60.0)?,
                    Point::new(9.0, 60.0)?,
                    Point::new(5.0, 70.0)?,
                ],
                color,
            )
        })?;
        assert_eq!(scene.commands().len(), 6);
        assert_eq!(shape_commands(&scene), 6);
        assert!(scene.masks().is_empty() && scene.images().is_empty());
        // A fill without corners stays a solid rectangle, and zero strokes,
        // empty arcs and collinear triangles draw nothing.
        let plain = drawn(|p| {
            let color = p.color(ColorRole::Primary);
            let rect = Rect::new(1.0, 1.0, 5.0, 5.0)?;
            p.fill_shape(rect, Shape::RECTANGLE, color)?;
            p.fill_shape(rect, Shape::rounded(Length::new(0.0)?), color)?;
            p.stroke_shape(
                rect,
                Shape::rounded(Length::new(2.0)?),
                Length::new(0.0)?,
                color,
            )?;
            p.stroke_shape(
                Rect::new(1.0, 1.0, 0.0, 5.0)?,
                Shape::RECTANGLE,
                Length::new(1.0)?,
                color,
            )?;
            p.stroke_arc(
                Point::new(50.0, 50.0)?,
                Length::new(20.0)?,
                0.0,
                0.0,
                Length::new(3.0)?,
                color,
            )?;
            p.stroke_arc(
                Point::new(50.0, 50.0)?,
                Length::new(20.0)?,
                0.0,
                90.0,
                Length::new(0.0)?,
                color,
            )?;
            p.fill_triangle(
                [
                    Point::new(1.0, 1.0)?,
                    Point::new(2.0, 2.0)?,
                    Point::new(3.0, 3.0)?,
                ],
                color,
            )?;
            // Collinear points draw nothing even where their bounds are
            // not representable; other points with such bounds are refused.
            p.fill_triangle(
                [
                    Point::new(-f32::MAX, 0.0)?,
                    Point::new(0.0, 0.0)?,
                    Point::new(f32::MAX, 0.0)?,
                ],
                color,
            )?;
            assert!(
                p.fill_triangle(
                    [
                        Point::new(-f32::MAX, 0.0)?,
                        Point::new(0.0, 1.0)?,
                        Point::new(f32::MAX, 0.0)?,
                    ],
                    color,
                )
                .is_err()
            );
            Ok(())
        })?;
        assert!(matches!(
            plain.commands(),
            [DrawCommand::FillRect { .. }, DrawCommand::FillRect { .. }]
        ));
        Ok(())
    }

    #[test]
    fn a_shape_the_clip_removes_costs_no_command() -> Result<(), Box<dyn std::error::Error>> {
        let scene = drawn(|p| {
            let color = p.color(ColorRole::Primary);
            let round = Shape::rounded(Length::new(4.0)?);
            let arc = |p: &mut Painter, x: f32| {
                p.stroke_arc(
                    Point::new(x, 20.0)?,
                    Length::new(8.0)?,
                    0.0,
                    360.0,
                    Length::new(4.0)?,
                    color,
                )
            };
            p.with_clip(Rect::new(100.0, 0.0, 50.0, 50.0)?, |p| {
                // Outside the clip, and touching it from outside: dropped.
                p.fill_shape(Rect::new(10.0, 10.0, 40.0, 20.0)?, round, color)?;
                p.fill_shape(Rect::new(60.0, 10.0, 40.0, 20.0)?, round, color)?;
                arc(p, 60.0)?;
                // Reaching into it, and an arc whose antialiasing may: kept.
                p.fill_shape(Rect::new(61.0, 10.0, 40.0, 20.0)?, round, color)?;
                arc(p, 89.5)
            })
        })?;
        assert_eq!(shape_commands(&scene), 2);
        assert_eq!(scene.commands().len(), 2);
        Ok(())
    }

    #[test]
    fn an_oversized_top_radius_follows_the_overlap_rule() -> Result<(), Box<dyn std::error::Error>>
    {
        // On a 100×20 rectangle a top radius of 15 fits by the overlap rule;
        // it is not clamped to half the smaller side.
        let scene = drawn(|p| {
            let color = p.color(ColorRole::Primary);
            p.fill_shape(
                Rect::new(10.0, 10.0, 100.0, 20.0)?,
                Shape::rounded_top(Length::new(15.0)?),
                color,
            )
        })?;
        let [DrawCommand::DrawShape { geometry, .. }] = scene.commands() else {
            return Err("not one shape".into());
        };
        let mapping = FrameMapping::new(200, 100, scene.viewport())?;
        let prepared = PreparedShape::new(geometry, None, None, &mapping)?.ok_or("nothing")?;
        let ShapeParameters::Rect { outer, .. } = prepared.parameters() else {
            return Err("not a rectangle".into());
        };
        assert_eq!(outer.corners()[0].size(), 15.0);
        Ok(())
    }

    #[test]
    fn a_knob_turned_through_many_values_adds_no_coverage_resource()
    -> Result<(), Box<dyn std::error::Error>> {
        use crate::{KeyAction, KeyCode, Knob, ParameterId, ParameterSpec};
        let mut ui = Runtime::new();
        ui.set_viewport(Viewport::new(120.0, 120.0, 1.25)?)?;
        let (level, _host) = ui.parameter(
            ParameterId::new(1, 1, "level")?,
            ParameterSpec::new("Level", 0.0, 1.0, 0.0)?,
        )?;
        let knob = ui.mount_tree(crate::Element::new("level", Knob::new(level.clone())))?;
        ui.flush(Duration::ZERO)?;
        ui.focus(&knob)?;
        let mut values = Vec::new();
        for _ in 0..200 {
            ui.key_input(KeyCode::ArrowUp, KeyAction::Press)?;
            ui.key_input(KeyCode::ArrowUp, KeyAction::Release)?;
            ui.flush(Duration::ZERO)?;
            let scene = ui.paint()?;
            assert!(scene.masks().is_empty() && scene.images().is_empty());
            assert!(shape_commands(&scene) > 0);
            values.push(level.snapshot().get()?.base);
        }
        values.dedup();
        assert!(values.len() > 50, "{} distinct values", values.len());
        Ok(())
    }

    #[test]
    fn composition_places_and_scales_a_shape() -> Result<(), Box<dyn std::error::Error>> {
        let length = |value: f32| Length::new(value);
        let rect = ShapeGeometry::Rect {
            rect: Rect::new(2.0, 3.0, 10.0, 20.0)?,
            shape: Shape::rounded(length(4.0)?),
        };
        // A component at (100, 50) under a view scale of 2.
        let Some(ShapeGeometry::Rect {
            rect: placed,
            shape,
        }) = rect.placed((100.0, 50.0), 2.0)?
        else {
            return Err("not a rectangle".into());
        };
        assert_eq!(placed.values(), [104.0, 56.0, 20.0, 40.0]);
        assert_eq!(shape, Shape::rounded(length(8.0)?));
        let arc = ShapeGeometry::Arc(ArcGeometry::new(
            Point::new(5.0, 6.0)?,
            length(10.0)?,
            30.0,
            90.0,
            length(2.0)?,
        )?);
        let Some(ShapeGeometry::Arc(placed)) = arc.placed((100.0, 50.0), 2.0)? else {
            return Err("not an arc".into());
        };
        assert_eq!(
            (placed.center(), placed.radius(), placed.width()),
            (Point::new(110.0, 62.0)?, length(20.0)?, length(4.0)?)
        );
        assert_eq!(
            (placed.start_degrees(), placed.sweep_degrees()),
            (30.0, 90.0)
        );
        let triangle = ShapeGeometry::Triangle(
            TriangleGeometry::new([
                Point::new(0.0, 0.0)?,
                Point::new(4.0, 0.0)?,
                Point::new(0.0, 4.0)?,
            ])
            .ok_or("degenerate")?,
        );
        let Some(ShapeGeometry::Triangle(placed)) = triangle.placed((100.0, 50.0), 2.0)? else {
            return Err("not a triangle".into());
        };
        assert_eq!(placed.vertices()[1], Point::new(108.0, 50.0)?);
        // The border scales with the view too.
        let border = Border::new(length(1.5)?, Paint::Solid(Color::WHITE)).scaled(2.0)?;
        assert_eq!(border.width(), length(3.0)?);
        Ok(())
    }

    /// A 40×40 component that draws one bordered shape, in `paint` or in
    /// `paint_foreground`.
    struct Tile {
        foreground: bool,
        /// How many times the tile was painted.
        paints: std::rc::Rc<std::cell::Cell<u32>>,
    }

    impl Tile {
        fn draw(p: &mut Painter) -> Result<(), DrawError> {
            let color = p.color(ColorRole::Primary);
            p.draw_shape(
                Rect::new(2.0, 3.0, 10.0, 20.0)?,
                Shape::rounded(Length::new(4.0)?),
                ShapeStyle::fill(color).with_border(Length::new(1.5)?, color),
            )?;
            p.fill_shape(
                Rect::new(20.0, 3.0, 10.0, 20.0)?,
                Shape::rounded(Length::new(4.0)?),
                color,
            )
        }
    }

    impl Component for Tile {
        fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
            Ok(())
        }

        fn measure(&self, _: &MeasureContext<'_>, _: Constraints) -> Result<Size, RuntimeError> {
            Ok(Size::new(40.0, 40.0)?)
        }

        fn paint(&self, painter: &mut Painter) -> Result<(), DrawError> {
            self.paints.set(self.paints.get() + 1);
            if self.foreground {
                Ok(())
            } else {
                Self::draw(painter)
            }
        }

        fn paint_foreground(&self, painter: &mut Painter) -> Result<(), DrawError> {
            if self.foreground {
                Self::draw(painter)
            } else {
                Ok(())
            }
        }
    }

    #[test]
    fn a_shape_under_a_view_transform_is_composed_at_its_displayed_size()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut ui = Runtime::new();
        ui.set_viewport(Viewport::new(400.0, 300.0, 1.0)?)?;
        let zoom = crate::ZoomController::new(&ui);
        let paints = std::rc::Rc::new(std::cell::Cell::new(0));
        ui.mount_tree(crate::Element::new(
            "canvas",
            crate::ZoomView::new([(
                Point::new(10.0, 5.0)?,
                crate::Element::new(
                    "tile",
                    Tile {
                        foreground: false,
                        paints: paints.clone(),
                    },
                ),
            )])
            .controller(&zoom),
        ))?;
        ui.flush(Duration::ZERO)?;
        let shapes = |ui: &mut Runtime| -> Result<Vec<(ShapeGeometry, Option<Border>)>, DrawError> {
            Ok(ui
                .paint()?
                .commands()
                .iter()
                .filter_map(|command| match command {
                    DrawCommand::DrawShape {
                        geometry,
                        border: border @ Some(_),
                        ..
                    } => Some((*geometry, *border)),
                    _ => None,
                })
                .collect())
        };
        let [(ShapeGeometry::Rect { rect, shape }, Some(border))] = shapes(&mut ui)?[..] else {
            return Err("not one bordered rectangle".into());
        };
        assert_eq!(rect.values(), [12.0, 8.0, 10.0, 20.0]);
        assert_eq!(shape, Shape::rounded(Length::new(4.0)?));
        assert_eq!(border.width().get(), 1.5);
        // Magnified three times and moved: positions map through the view,
        // and the corner and the border grow with it.
        zoom.set(crate::ViewTransform::new(
            crate::ViewScale::new(3.0)?,
            Point::new(7.0, 11.0)?,
        ))?;
        ui.flush(Duration::from_secs(1))?;
        let [(ShapeGeometry::Rect { rect, shape }, Some(border))] = shapes(&mut ui)?[..] else {
            return Err("not one bordered rectangle".into());
        };
        assert_eq!(
            rect.values(),
            [7.0 + 3.0 * 12.0, 11.0 + 3.0 * 8.0, 30.0, 60.0]
        );
        assert_eq!(shape, Shape::rounded(Length::new(12.0)?));
        assert_eq!(border.width().get(), 4.5);
        // Beyond the eight-fold cap of prepared coverage a shape still
        // follows the view: nothing was prepared that could be stretched.
        zoom.set(crate::ViewTransform::new(
            crate::ViewScale::new(16.0)?,
            Point::new(-150.0, -100.0)?,
        ))?;
        ui.flush(Duration::from_secs(2))?;
        let [(ShapeGeometry::Rect { rect, shape }, Some(border))] = shapes(&mut ui)?[..] else {
            return Err("not one bordered rectangle".into());
        };
        assert_eq!(
            rect.values(),
            [-150.0 + 16.0 * 12.0, -100.0 + 16.0 * 8.0, 160.0, 320.0]
        );
        assert_eq!(shape, Shape::rounded(Length::new(64.0)?));
        assert_eq!(border.width().get(), 24.0);
        // A change that follows within the live-zoom interval reuses the
        // retained drawing, and the shape is mapped rather than stretched.
        let painted = paints.get();
        zoom.set(crate::ViewTransform::new(
            crate::ViewScale::new(8.0)?,
            Point::new(-150.0, -100.0)?,
        ))?;
        ui.flush(Duration::from_millis(2010))?;
        let [(ShapeGeometry::Rect { rect, shape }, Some(border))] = shapes(&mut ui)?[..] else {
            return Err("not one bordered rectangle".into());
        };
        assert_eq!(
            rect.values(),
            [-150.0 + 8.0 * 12.0, -100.0 + 8.0 * 8.0, 80.0, 160.0]
        );
        assert_eq!(shape, Shape::rounded(Length::new(32.0)?));
        assert_eq!(border.width().get(), 12.0);
        assert_eq!(paints.get(), painted, "the tile was not painted again");
        Ok(())
    }

    #[test]
    fn foreground_drawing_records_shapes_too() -> Result<(), Box<dyn std::error::Error>> {
        let mut ui = Runtime::new();
        ui.set_viewport(Viewport::new(100.0, 100.0, 1.0)?)?;
        ui.mount_tree(crate::Element::new(
            "tile",
            Tile {
                foreground: true,
                paints: Default::default(),
            },
        ))?;
        ui.flush(Duration::ZERO)?;
        let scene = ui.paint()?;
        assert_eq!(shape_commands(&scene), 2);
        assert!(scene.masks().is_empty());
        Ok(())
    }
}
