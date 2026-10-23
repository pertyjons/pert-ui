//! Vector paths and strokes preserved in renderer-independent display lists.
//! The explicit coverage fallback lives behind `VectorRasterizer`, in the private
//! `raster` module.
mod raster;
use crate::{Color, DrawError, Length, Painter, Point, Rect};
pub(crate) use raster::{MAX_EDGES, coverage_bounds, rasterize_vector};
use std::sync::Arc;

/// Invalid vector path construction or stroke configuration.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum PathError {
    /// The path contains no segment: nothing but `move_to` calls, or nothing at all.
    #[error("path contains no segment")]
    Empty,
    /// A segment or close was added before the first `move_to`.
    #[error("path segment has no current point; begin with move_to")]
    MissingMoveTo,
    /// An angle, sweep, radius, miter limit or derived coordinate is non-finite
    /// or outside its documented range.
    #[error("path angle, radius, limit or coordinate is non-finite or out of range")]
    InvalidParameter,
    /// The path exceeds [`Path::MAX_POINTS`] points, or flattening it would
    /// exceed the documented segment budget.
    #[error("path exceeds its size limit")]
    TooLarge,
}

/// Which regions of a self-intersecting or nested fill are inside.
///
/// Subpaths are closed implicitly for filling. The default is [`FillRule::NonZero`].
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum FillRule {
    /// A point is inside when the signed count of crossings is nonzero, so nested
    /// subpaths with the same direction fill and opposite directions cut holes.
    #[default]
    NonZero,
    /// A point is inside when the number of crossings is odd, so every nested
    /// subpath alternates between filled and hole regardless of direction.
    EvenOdd,
}

/// Shape at the two ends of an open stroked subpath.
///
/// The default is [`LineCap::Butt`].
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum LineCap {
    /// The stroke ends exactly at the endpoint. A zero-length subpath draws nothing.
    #[default]
    Butt,
    /// The stroke extends half its width beyond the endpoint with a square end.
    /// A zero-length subpath draws an axis-aligned square of the stroke width.
    Square,
    /// A semicircle of the stroke width is added at the endpoint. A zero-length
    /// subpath draws a disc of the stroke width.
    Round,
}

/// Shape where two stroked segments meet.
///
/// The default is [`LineJoin::Miter`] with a limit of 4, as in SVG.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum LineJoin {
    /// The outer edges extend until they meet, unless the miter would be longer
    /// than the style's miter limit times the stroke width; then a bevel is drawn.
    #[default]
    Miter,
    /// The outer corners are connected by a straight edge.
    Bevel,
    /// A circular arc of the stroke width's diameter rounds the outer corner.
    Round,
}

/// Width, caps, joins and miter limit of a stroke, centered on the path.
///
/// Created with [`StrokeStyle::new`]: butt caps, miter joins and a miter limit of
/// 4. The width is a logical length, like every other length; half of it lies on
/// each side of the path. A zero width draws nothing, and widths below one
/// physical pixel draw proportionally fainter coverage rather than a minimum
/// hairline.
///
/// ```
/// # use pert_ui_runtime as pert_ui;
/// use pert_ui::{Length, LineCap, LineJoin, StrokeStyle};
/// let style = StrokeStyle::new(Length::new(2.0)?)
///     .with_cap(LineCap::Round)
///     .with_join(LineJoin::Bevel);
/// assert_eq!(style.width().get(), 2.0);
/// assert_eq!(style.miter_limit(), 4.0);
/// assert!(StrokeStyle::new(Length::new(1.0)?).with_miter_limit(0.5).is_err());
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
#[must_use]
pub struct StrokeStyle {
    width: Length,
    cap: LineCap,
    join: LineJoin,
    miter_limit: f32,
}
impl StrokeStyle {
    /// A stroke of the given logical width with butt caps, miter joins and a
    /// miter limit of 4.
    pub const fn new(width: Length) -> Self {
        Self {
            width,
            cap: LineCap::Butt,
            join: LineJoin::Miter,
            miter_limit: 4.0,
        }
    }
    /// Replaces the cap drawn at both ends of every open subpath.
    pub const fn with_cap(mut self, cap: LineCap) -> Self {
        self.cap = cap;
        self
    }
    /// Replaces the join drawn between consecutive segments.
    pub const fn with_join(mut self, join: LineJoin) -> Self {
        self.join = join;
        self
    }
    /// Replaces the miter limit: the longest permitted miter as a multiple of the
    /// stroke width. A join whose miter would be longer is beveled instead.
    ///
    /// # Errors
    /// Returns [`PathError::InvalidParameter`] unless the limit is finite and at
    /// least 1.
    pub fn with_miter_limit(mut self, limit: f32) -> Result<Self, PathError> {
        if !limit.is_finite() || limit < 1.0 {
            return Err(PathError::InvalidParameter);
        }
        self.miter_limit = limit;
        Ok(self)
    }
    /// Full stroke width in logical pixels.
    #[must_use]
    pub const fn width(self) -> Length {
        self.width
    }
    /// Cap at the ends of open subpaths.
    #[must_use]
    pub const fn cap(self) -> LineCap {
        self.cap
    }
    /// Join between consecutive segments.
    #[must_use]
    pub const fn join(self) -> LineJoin {
        self.join
    }
    /// Longest permitted miter as a multiple of the stroke width, at least 1.
    #[must_use]
    pub const fn miter_limit(self) -> f32 {
        self.miter_limit
    }
}

/// A validated path operation in local logical coordinates.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PathElement {
    /// Starts a subpath at the given point.
    Move(Point),
    /// Straight segment to the endpoint.
    Line(Point),
    /// Quadratic segment: control point, endpoint.
    Quad(Point, Point),
    /// Cubic segment: first control, second control, endpoint.
    Cubic(Point, Point, Point),
    /// Closes the current subpath to its start.
    Close,
}
use PathElement as Verb;

/// An immutable vector outline of lines and Bézier curves in local logical pixels.
///
/// A path is a sequence of subpaths, each starting with a move. Build one with
/// [`PathBuilder`] or a constructor such as [`Path::polyline`] or
/// [`Path::ellipse`]. It is validated when built: every coordinate is finite and
/// the path has at least one segment and at most [`Path::MAX_POINTS`] points.
/// Clones share storage, so a component can retain a path and record it on every
/// paint without copying it. Paths are `Send` and `Sync` and hold no runtime state.
///
/// Recording preserves the path for renderer-side preparation; see
/// [`Painter::fill_path`]. There is no drawing transform: the path is drawn at
/// its own coordinates in the recording component's space, and
/// [`Path::transformed`] maps the geometry itself.
///
/// ```
/// # use pert_ui_runtime as pert_ui;
/// use pert_ui::{Path, PathBuilder, Point};
/// let wedge = PathBuilder::new()
///     .move_to(Point::new(0.0, 0.0)?)
///     .line_to(Point::new(16.0, 0.0)?)
///     .quad_to(Point::new(16.0, 16.0)?, Point::new(0.0, 16.0)?)
///     .close()
///     .build()?;
/// assert_eq!(wedge.bounds().values(), [0.0, 0.0, 16.0, 16.0]);
/// assert!(Path::polyline([]).is_err());
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct Path {
    verbs: Arc<[Verb]>,
    bounds: Rect,
}

impl Path {
    /// Validated operations in drawing order, for renderer tessellation or rasterization.
    pub fn elements(&self) -> &[PathElement] {
        &self.verbs
    }

    /// Largest number of points a path may contain, counting every endpoint and
    /// Bézier control point. Larger paths are rejected with [`PathError::TooLarge`].
    pub const MAX_POINTS: usize = 65_536;

    /// An open path through the points in order, for traces and graphs.
    ///
    /// A single point is a zero-length subpath, which only round and square
    /// caps make visible when stroked.
    ///
    /// # Errors
    /// Returns [`PathError::Empty`] for no points, [`PathError::TooLarge`]
    /// above [`Path::MAX_POINTS`] and [`PathError::InvalidParameter`] when the
    /// points cannot be enclosed by finite `f32` extents after outward rounding.
    pub fn polyline(points: impl IntoIterator<Item = Point>) -> Result<Self, PathError> {
        Self::through(points, false)
    }
    /// A closed polygon through the points in order.
    ///
    /// # Errors
    /// As for [`Path::polyline`].
    pub fn polygon(points: impl IntoIterator<Item = Point>) -> Result<Self, PathError> {
        Self::through(points, true)
    }
    fn through(points: impl IntoIterator<Item = Point>, close: bool) -> Result<Self, PathError> {
        let mut points = points.into_iter();
        let first = points.next().ok_or(PathError::Empty)?;
        let mut builder = PathBuilder::new().move_to(first);
        let mut segments = 0;
        for point in points {
            builder = builder.line_to(point);
            segments += 1;
        }
        if segments == 0 {
            builder = builder.line_to(first);
        }
        if close {
            builder = builder.close();
        }
        builder.build()
    }
    /// The ellipse inscribed in a rectangle, as a closed clockwise outline of
    /// four cubic Bézier curves. A zero-width or zero-height rectangle gives a
    /// degenerate ellipse that fills nothing.
    ///
    /// # Errors
    /// Returns [`PathError::InvalidParameter`] if the rectangle's far edges are
    /// not representable as finite coordinates.
    pub fn ellipse(rect: Rect) -> Result<Self, PathError> {
        let rx = rect.width() / 2.0;
        let ry = rect.height() / 2.0;
        let center = point(rect.x() + rx, rect.y() + ry)?;
        PathBuilder::new()
            .ellipse_arc(center, rx, ry, 0.0, 360.0, false)
            .close()
            .build()
    }
    /// A circle around a center, as [`Path::ellipse`] of its bounding square.
    ///
    /// # Errors
    /// As for [`Path::ellipse`].
    pub fn circle(center: Point, radius: Length) -> Result<Self, PathError> {
        let r = radius.get();
        let rect = Rect::new(center.x() - r, center.y() - r, 2.0 * r, 2.0 * r)
            .map_err(|_| PathError::InvalidParameter)?;
        Self::ellipse(rect)
    }
    /// A rectangle with equal circular corners, closed and clockwise.
    ///
    /// The radius clamps to half the shorter side, as for
    /// [`Shape::Rounded`](crate::Shape::Rounded); zero gives square corners.
    ///
    /// # Errors
    /// As for [`Path::ellipse`].
    pub fn rounded_rect(rect: Rect, radius: Length) -> Result<Self, PathError> {
        let [x, y, w, h] = rect.values();
        let r = radius.get().min(w / 2.0).min(h / 2.0);
        let (right, bottom) = (x + w, y + h);
        if !right.is_finite() || !bottom.is_finite() {
            return Err(PathError::InvalidParameter);
        }
        if r <= 0.0 {
            return Self::polygon([
                point(x, y)?,
                point(right, y)?,
                point(right, bottom)?,
                point(x, bottom)?,
            ]);
        }
        PathBuilder::new()
            .move_to(point(x + r, y)?)
            .line_to(point(right - r, y)?)
            .ellipse_arc(point(right - r, y + r)?, r, r, 270.0, 90.0, true)
            .line_to(point(right, bottom - r)?)
            .ellipse_arc(point(right - r, bottom - r)?, r, r, 0.0, 90.0, true)
            .line_to(point(x + r, bottom)?)
            .ellipse_arc(point(x + r, bottom - r)?, r, r, 90.0, 90.0, true)
            .line_to(point(x, y + r)?)
            .ellipse_arc(point(x + r, y + r)?, r, r, 180.0, 90.0, true)
            .close()
            .build()
    }
    /// This outline mapped through an affine transform, as a new path.
    ///
    /// The result is exact: an affine map of a line, a quadratic or a cubic Bézier
    /// is a segment of the same kind through the mapped points, and arcs are
    /// already stored as cubics. It is an ordinary path in the same local space,
    /// so clipping, caching and rasterization treat it like any other.
    ///
    /// **Strokes are not distorted.** Only the outline is mapped; a
    /// [`StrokeStyle`] width stays in local logical pixels and is applied
    /// afterwards. A plot can therefore fit data to its area with a different
    /// factor per axis and still draw a trace of constant width. Multiply the width
    /// yourself when a uniformly scaled stroke is wanted.
    ///
    /// Geometry that changes every frame is new geometry every frame and is
    /// rasterized again, so a continuously rotating shape costs coverage work on
    /// each frame it is shown.
    ///
    /// ```
    /// # use pert_ui_runtime as pert_ui;
    /// use pert_ui::{Path, Point, Rect, Transform2D};
    /// let bar = Path::polygon([
    ///     Point::new(0.0, 0.0)?, Point::new(10.0, 0.0)?,
    ///     Point::new(10.0, 2.0)?, Point::new(0.0, 2.0)?,
    /// ])?;
    /// let upright = bar.transformed(&Transform2D::rotation_degrees(90.0)?)?;
    /// assert_eq!(upright.bounds(), Rect::new(-2.0, 0.0, 2.0, 10.0)?);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// # Errors
    /// Returns [`PathError::InvalidParameter`] when a mapped coordinate is not a
    /// finite `f32`, or the mapped points cannot be enclosed by finite `f32`
    /// extents after outward rounding.
    pub fn transformed(&self, transform: &crate::Transform2D) -> Result<Self, PathError> {
        let map = |p: &Point| {
            let (x, y) = transform.map(f64::from(p.x()), f64::from(p.y()));
            point64(x, y)
        };
        let mut verbs = Vec::with_capacity(self.verbs.len());
        for verb in self.verbs.iter() {
            verbs.push(match verb {
                Verb::Move(p) => Verb::Move(map(p)?),
                Verb::Line(p) => Verb::Line(map(p)?),
                Verb::Quad(a, b) => Verb::Quad(map(a)?, map(b)?),
                Verb::Cubic(a, b, c) => Verb::Cubic(map(a)?, map(b)?, map(c)?),
                Verb::Close => Verb::Close,
            });
        }
        bounded(verbs)
    }
    /// Smallest representable rectangle at the minimum coordinates containing
    /// every endpoint and control point, with extents rounded outward as needed.
    /// Far edges are evaluated by adding the stored origin and extent in `f64`,
    /// as for [`Rect::intersection`].
    ///
    /// Bézier control points may lie outside the curve, so this can exceed the
    /// drawn area; a stroke additionally extends beyond it by its width.
    #[must_use]
    pub const fn bounds(&self) -> Rect {
        self.bounds
    }
}

/// Incremental construction of a [`Path`].
///
/// Methods chain by value and never fail; the first invalid step is remembered
/// and reported by [`Self::build`]. A segment added before the first
/// [`Self::move_to`] is [`PathError::MissingMoveTo`]. After [`Self::close`], the
/// current point returns to the subpath's start, and a following segment starts a
/// new subpath there, as in SVG.
#[derive(Clone, Debug, Default)]
#[must_use]
pub struct PathBuilder {
    verbs: Vec<Verb>,
    points: usize,
    start: Option<Point>,
    current: Option<Point>,
    open: bool,
    error: Option<PathError>,
}

impl PathBuilder {
    /// An empty builder with no current point.
    pub fn new() -> Self {
        Self::default()
    }
    fn fail(mut self, error: PathError) -> Self {
        self.error.get_or_insert(error);
        self
    }
    fn push(mut self, verb: Verb, points: usize) -> Self {
        self.points += points;
        if self.points > Path::MAX_POINTS {
            return self.fail(PathError::TooLarge);
        }
        self.verbs.push(verb);
        self
    }
    // Starts a new subpath at the current point after a close.
    fn segment(self) -> Self {
        match (self.open, self.current) {
            (true, _) => self,
            (false, Some(point)) => {
                let mut builder = self.push(Verb::Move(point), 1);
                builder.start = Some(point);
                builder.open = true;
                builder
            }
            (false, None) => self.fail(PathError::MissingMoveTo),
        }
    }
    /// Begins a new subpath at a point, leaving the previous one open.
    pub fn move_to(mut self, point: Point) -> Self {
        self = self.push(Verb::Move(point), 1);
        self.start = Some(point);
        self.current = Some(point);
        self.open = true;
        self
    }
    /// Adds a straight segment from the current point.
    pub fn line_to(mut self, point: Point) -> Self {
        self = self.segment().push(Verb::Line(point), 1);
        self.current = Some(point);
        self
    }
    /// Adds a quadratic Bézier curve with one control point.
    pub fn quad_to(mut self, control: Point, point: Point) -> Self {
        self = self.segment().push(Verb::Quad(control, point), 2);
        self.current = Some(point);
        self
    }
    /// Adds a cubic Bézier curve with two control points.
    pub fn cubic_to(mut self, first: Point, second: Point, point: Point) -> Self {
        self = self.segment().push(Verb::Cubic(first, second, point), 3);
        self.current = Some(point);
        self
    }
    /// Adds a circular arc around `center`.
    ///
    /// Angles are degrees clockwise from the rightward radius in screen
    /// coordinates, as for [`Painter::stroke_arc`]. `start_degrees` may be any
    /// finite angle; `sweep_degrees` lies in `-360..=360`, positive clockwise. If
    /// a subpath is open, a straight segment joins its current point to the
    /// arc's start; otherwise the arc begins a new subpath. The arc is stored as
    /// cubic Bézier curves of at most 90 degrees each.
    ///
    /// An invalid angle is reported by [`Self::build`] as
    /// [`PathError::InvalidParameter`].
    pub fn arc(
        self,
        center: Point,
        radius: Length,
        start_degrees: f32,
        sweep_degrees: f32,
    ) -> Self {
        let r = radius.get();
        self.ellipse_arc(center, r, r, start_degrees, sweep_degrees, true)
    }
    fn ellipse_arc(
        mut self,
        center: Point,
        rx: f32,
        ry: f32,
        start: f32,
        sweep: f32,
        connect: bool,
    ) -> Self {
        if !start.is_finite() || !sweep.is_finite() || !(-360.0..=360.0).contains(&sweep) {
            return self.fail(PathError::InvalidParameter);
        }
        let (cx, cy) = (f64::from(center.x()), f64::from(center.y()));
        let (rx, ry) = (f64::from(rx), f64::from(ry));
        let at = |angle: f64| point64(cx + rx * angle.cos(), cy + ry * angle.sin());
        let start = f64::from(start).to_radians();
        let sweep = f64::from(sweep).to_radians();
        let first = match at(start) {
            Ok(first) => first,
            Err(error) => return self.fail(error),
        };
        self = if connect && self.open {
            self.line_to(first)
        } else {
            self.move_to(first)
        };
        let count = (sweep.abs() / std::f64::consts::FRAC_PI_2).ceil().max(1.0) as usize;
        let step = sweep / count as f64;
        let k = 4.0 / 3.0 * (step / 4.0).tan();
        for index in 0..count {
            let a0 = start + step * index as f64;
            let a1 = a0 + step;
            let controls = (|| {
                Ok::<_, PathError>((
                    point64(
                        cx + rx * (a0.cos() - k * a0.sin()),
                        cy + ry * (a0.sin() + k * a0.cos()),
                    )?,
                    point64(
                        cx + rx * (a1.cos() + k * a1.sin()),
                        cy + ry * (a1.sin() - k * a1.cos()),
                    )?,
                    at(a1)?,
                ))
            })();
            self = match controls {
                Ok((c1, c2, end)) => self.cubic_to(c1, c2, end),
                Err(error) => return self.fail(error),
            };
        }
        self
    }
    /// Closes the current subpath with a straight segment to its start.
    ///
    /// A closed subpath is stroked with a join at its start instead of two caps.
    /// Closing with no open subpath is [`PathError::MissingMoveTo`].
    pub fn close(mut self) -> Self {
        if !self.open {
            return self.fail(PathError::MissingMoveTo);
        }
        self = self.push(Verb::Close, 0);
        self.current = self.start;
        self.open = false;
        self
    }
    /// Validates and freezes the path.
    ///
    /// # Errors
    /// Returns the first error recorded while building, [`PathError::Empty`]
    /// when no segment was added, or [`PathError::InvalidParameter`] when the
    /// points cannot be enclosed by finite `f32` extents after outward rounding.
    pub fn build(self) -> Result<Path, PathError> {
        if let Some(error) = self.error {
            return Err(error);
        }
        bounded(self.verbs)
    }
}

// Freezes validated operations, computing the bounds of their points.
fn bounded(verbs: Vec<Verb>) -> Result<Path, PathError> {
    let mut min = (f32::INFINITY, f32::INFINITY);
    let mut max = (f32::NEG_INFINITY, f32::NEG_INFINITY);
    let mut segments = false;
    for verb in &verbs {
        let points: &[Point] = match verb {
            Verb::Move(p) => std::slice::from_ref(p),
            Verb::Line(p) => {
                segments = true;
                std::slice::from_ref(p)
            }
            Verb::Quad(a, b) => {
                segments = true;
                &[*a, *b]
            }
            Verb::Cubic(a, b, c) => {
                segments = true;
                &[*a, *b, *c]
            }
            Verb::Close => {
                segments = true;
                &[]
            }
        };
        for p in points {
            min = (min.0.min(p.x()), min.1.min(p.y()));
            max = (max.0.max(p.x()), max.1.max(p.y()));
        }
    }
    if !segments {
        return Err(PathError::Empty);
    }
    let extent = |min: f32, max: f32| -> Result<f32, PathError> {
        let mut width = (f64::from(max) - f64::from(min)) as f32;
        // A finite subtraction may round inward. In particular, -1e20..50
        // must not advertise a far edge of zero and be culled before clipping.
        if f64::from(min) + f64::from(width) < f64::from(max) {
            width = width.next_up();
        }
        if !width.is_finite() {
            return Err(PathError::InvalidParameter);
        }
        Ok(width)
    };
    let bounds = Rect::new(min.0, min.1, extent(min.0, max.0)?, extent(min.1, max.1)?)
        .map_err(|_| PathError::InvalidParameter)?;
    Ok(Path {
        verbs: verbs.into(),
        bounds,
    })
}

fn point(x: f32, y: f32) -> Result<Point, PathError> {
    Point::new(x, y).map_err(|_| PathError::InvalidParameter)
}
fn point64(x: f64, y: f64) -> Result<Point, PathError> {
    point(x as f32, y as f32)
}

impl Painter {
    /// Fills a path with antialiased coverage in local logical coordinates.
    ///
    /// Every subpath is closed implicitly; `rule` decides which overlapping or
    /// nested regions are inside. Coverage approximates the area of each physical
    /// pixel inside the path at the effective scale, from four sub-scanlines per
    /// pixel row with exact horizontal overlap, quantized to 8 bits. Curves are
    /// flattened within 0.2 physical pixels. Overlapping parts of one path are
    /// covered once, so a translucent color never doubles up within it.
    ///
    /// Records original geometry in [`DrawCommand::DrawVector`](crate::DrawCommand::DrawVector).
    /// Adapters can consume curves directly or retain a [`crate::VectorRasterizer`]
    /// to lower them to coverage masks. The fallback implements the coverage
    /// rules above, clips before allocating pixels and caches unchanged geometry.
    /// Component recording itself does not flatten or allocate a coverage raster.
    ///
    /// # Errors
    /// A validated path cannot fail recording. Backend preparation can fail with
    /// `CoverageTooLarge` when the cropped coverage exceeds
    /// [`MaskImage::MAX_PIXELS`](crate::MaskImage::MAX_PIXELS) or `Path(TooLarge)`
    /// when flattening exceeds its segment budget, or [`DrawError::TooComplex`]
    /// when active coverage exceeds [`crate::VectorRasterizer::MAX_COVERAGE_WORK`].
    ///
    /// ```
    /// # use pert_ui_runtime as pert_ui;
    /// use pert_ui::*;
    /// struct Badge;
    /// impl Component for Badge {
    ///     fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
    ///         Ok(())
    ///     }
    ///     fn paint(&self, p: &mut Painter) -> Result<(), DrawError> {
    ///         let square = |x, y, side| -> Result<Vec<Point>, RuntimeError> {
    ///             Ok(vec![
    ///                 Point::new(x, y)?,
    ///                 Point::new(x + side, y)?,
    ///                 Point::new(x + side, y + side)?,
    ///                 Point::new(x, y + side)?,
    ///             ])
    ///         };
    ///         // Two nested squares; even-odd leaves the inner one as a hole.
    ///         let mut builder = PathBuilder::new();
    ///         for ring in [square(0.0, 0.0, 24.0)?, square(8.0, 8.0, 8.0)?] {
    ///             builder = builder.move_to(ring[0]);
    ///             for point in &ring[1..] {
    ///                 builder = builder.line_to(*point);
    ///             }
    ///             builder = builder.close();
    ///         }
    ///         p.fill_path(&builder.build()?, FillRule::EvenOdd, p.color(ColorRole::Primary))
    ///     }
    /// }
    /// let mut ui = Runtime::new();
    /// ui.set_viewport(Viewport::new(32.0, 32.0, 1.0)?)?;
    /// ui.mount("badge", Badge)?;
    /// ui.flush(std::time::Duration::ZERO)?;
    /// let scene = ui.paint()?.rasterized()?;
    /// let DrawCommand::DrawMask { mask, .. } = scene.commands()[0] else {
    ///     return Err("the rasterized path is one mask".into());
    /// };
    /// let mask = scene.mask(mask).ok_or("mask")?;
    /// assert_eq!(mask.pixels()[4 * 24 + 4], 255); // filled ring
    /// assert_eq!(mask.pixels()[12 * 24 + 12], 0); // hole
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn fill_path(
        &mut self,
        path: &Path,
        rule: FillRule,
        color: Color,
    ) -> Result<(), DrawError> {
        self.record_vector(path, crate::VectorStyle::Fill(rule), color);
        Ok(())
    }

    /// Strokes a path with a centered outline of the given style.
    ///
    /// Half the width lies on each side of the path, so the stroke extends
    /// past the path's bounds: by half the width along a segment, and further
    /// at miter joins and square caps. [`Self::stroke_shape`] strokes a
    /// rectangle, rounded rectangle or ellipse inward instead, keeping the
    /// whole stroke inside the shape. Open subpaths receive the
    /// style's cap at both ends; closed subpaths join at their start instead.
    /// Joins apply between consecutive segments, including between the flattened
    /// pieces of a curve, where the turn is small. Zero-length subpaths draw a
    /// dot or square for round and square caps and nothing for butt caps. A zero
    /// width draws nothing. The outline is filled as one union, so self-overlap
    /// and joins are covered once. Clipping, caching and errors are those of
    /// [`Self::fill_path`].
    ///
    /// Coverage is sampled as [`Self::fill_path`] samples it. A path of
    /// straight segments, with any join and cap, is covered directly from its
    /// segment bodies, joins and caps, with exact round joins and caps, without
    /// building an outline. Its cost grows with the number of pixel rows each
    /// segment, join and cap spans rather than with how many outline edges
    /// cross each row, which suits a trace that changes on every frame; a trace
    /// whose segments each span many rows still costs in proportion. A path
    /// with curves is outlined, with curves and round parts flattened within
    /// 0.2 physical pixels, and filled.
    ///
    /// # Errors
    /// As [`Self::fill_path`]: recording cannot fail, and backend preparation can fail for
    /// the coverage or flattening budget.
    pub fn stroke_path(
        &mut self,
        path: &Path,
        style: StrokeStyle,
        color: Color,
    ) -> Result<(), DrawError> {
        if style.width().get() != 0.0 {
            self.record_vector(path, crate::VectorStyle::Stroke(style), color);
        }
        Ok(())
    }

    /// Strokes an open polyline through points, the usual form of a trace or graph.
    ///
    /// Equivalent to stroking [`Path::polyline`] of the same points, without
    /// retaining a path. The stroke is centered on the line, as for
    /// [`Self::stroke_path`]; [`Self::stroke_shape`] is the inward stroke of a
    /// closed shape. A polyline has only straight segments, so it is
    /// covered on the fast path described at [`Self::stroke_path`], whatever
    /// its join. An empty slice draws nothing and a single point is a
    /// zero-length subpath. At most [`Path::MAX_POINTS`] points are accepted.
    ///
    /// # Errors
    /// Returns [`DrawError::Path`] with [`PathError::TooLarge`] above the point
    /// limit, otherwise as for [`Self::stroke_path`].
    ///
    /// ```
    /// # use pert_ui_runtime as pert_ui;
    /// use pert_ui::*;
    /// struct Trace(Vec<Point>);
    /// impl Component for Trace {
    ///     fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
    ///         Ok(())
    ///     }
    ///     fn paint(&self, p: &mut Painter) -> Result<(), DrawError> {
    ///         let style = StrokeStyle::new(Length::new(1.5)?)
    ///             .with_join(LineJoin::Round);
    ///         p.stroke_polyline(&self.0, style, p.color(ColorRole::Primary))
    ///     }
    /// }
    /// let points = (0..64)
    ///     .map(|i| Point::new(i as f32 * 2.0, 16.0 + 12.0 * (i as f32 * 0.3).sin()))
    ///     .collect::<Result<Vec<_>, _>>()?;
    /// let mut ui = Runtime::new();
    /// ui.set_viewport(Viewport::new(128.0, 32.0, 2.0)?)?;
    /// ui.mount("trace", Trace(points))?;
    /// ui.flush(std::time::Duration::ZERO)?;
    /// assert_eq!(ui.paint()?.vectors().len(), 1);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn stroke_polyline(
        &mut self,
        points: &[Point],
        style: StrokeStyle,
        color: Color,
    ) -> Result<(), DrawError> {
        if points.is_empty() {
            return Ok(());
        }
        self.stroke_path(&Path::polyline(points.iter().copied())?, style, color)
    }

    /// Fills the ellipse inscribed in a rectangle, as [`Path::ellipse`] filled
    /// with [`Self::fill_path`].
    ///
    /// # Errors
    /// As for [`Path::ellipse`] and [`Self::fill_path`].
    pub fn fill_ellipse(&mut self, rect: Rect, color: Color) -> Result<(), DrawError> {
        self.fill_path(&Path::ellipse(rect)?, FillRule::NonZero, color)
    }
    /// Strokes the ellipse inscribed in a rectangle, centered on its outline, as
    /// [`Path::ellipse`] stroked with [`Self::stroke_path`]. Unlike
    /// [`Self::stroke_shape`], which strokes inward, half the width lies outside
    /// the rectangle.
    ///
    /// # Errors
    /// As for [`Path::ellipse`] and [`Self::stroke_path`].
    pub fn stroke_ellipse(
        &mut self,
        rect: Rect,
        style: StrokeStyle,
        color: Color,
    ) -> Result<(), DrawError> {
        self.stroke_path(&Path::ellipse(rect)?, style, color)
    }
    fn record_vector(&mut self, path: &Path, style: crate::VectorStyle, color: Color) {
        let _timing = self
            .profiler
            .span(crate::ProfileOperation::Geometry, None, "");
        let vector = crate::VectorGeometry::new(path.clone(), style);
        let id = crate::VectorId(self.list.vectors.intern(&vector));
        self.list.fills.push(crate::DrawCommand::DrawVector {
            vector: id,
            origin: Point::default(),
            scale: 1.0,
            color,
            clip: self.clip,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_extent_beyond_f32_is_an_invalid_parameter() {
        let far = |x| Point::new(x, 0.0).unwrap();
        let points = [far(-3.0e38), far(3.0e38)];
        assert_eq!(Path::polyline(points), Err(PathError::InvalidParameter));
        assert_eq!(Path::polygon(points), Err(PathError::InvalidParameter));
        assert_eq!(
            PathBuilder::new()
                .move_to(points[0])
                .line_to(points[1])
                .build(),
            Err(PathError::InvalidParameter)
        );
    }
}
