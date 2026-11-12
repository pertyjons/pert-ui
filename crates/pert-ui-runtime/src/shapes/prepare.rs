//! Preparation and reference coverage for analytic shapes.
//!
//! A shape (a rectangle with a size and a kind per corner, a circular arc or
//! a triangle) is prepared once per frame: mapped to physical pixels, snapped,
//! clipped to an integer span of frame pixels and expressed relative to that
//! span's origin, all in `f64`. The prepared parameters are `f32`, and
//! [`PreparedShape::coverage`] evaluates them per pixel center with the
//! closed-form distances [`PreparedShape`] documents. That evaluation is the
//! reference a renderer reproduces.
//!
//! [`Painter::draw_shape`](crate::Painter::draw_shape) records these shapes
//! as [`DrawCommand::DrawShape`](crate::DrawCommand::DrawShape); a backend
//! prepares each one with [`PreparedShape::new`].
use crate::{Color, GeometryError, Length, PathError, Point, Rect, Viewport};
use std::f32::consts::SQRT_2;

/// The margin, in physical pixels, by which a reference distance must clear a
/// coverage ramp's end for a pixel to count as clear rather than as an edge
/// pixel when two renderers are compared. Confirmed by the parity run on the
/// one adapter available; `docs/rendering-platform.md` records it.
pub(crate) const PARITY_MARGIN: f32 = 1.0 / 16.0;

/// Every prepared parameter and span extent lies within this many physical
/// pixels of the span's origin, so that one `f32` rounding is at most 1/256
/// pixel.
const DOMAIN: f64 = 65536.0;

/// A triangle with an edge shorter than this, in physical pixels after
/// narrowing to `f32`, draws nothing.
const SHORTEST_EDGE: f64 = 1.0 / 256.0;

/// How a corner departs from the rectangle's square corner.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum CornerKind {
    /// The rectangle's own corner.
    Square,
    /// A circular arc of the corner's size as radius.
    Round,
    /// A 45° chamfer that removes the corner's size from both sides.
    Cut,
}

/// One corner of a [`Shape`]: square, a circular arc, or a 45° chamfer.
///
/// A size is in logical pixels. Sizes that do not fit their rectangle are
/// scaled down together when the shape is drawn, as CSS does for overlapping
/// corner radii: a single corner may take a whole side.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Corner {
    kind: CornerKind,
    size: Length,
}

impl Corner {
    /// The rectangle's own corner.
    pub const SQUARE: Self = Self {
        kind: CornerKind::Square,
        size: Length::ZERO,
    };

    /// A circular corner of `radius` logical pixels.
    #[must_use]
    pub const fn round(radius: Length) -> Self {
        Self {
            kind: CornerKind::Round,
            size: radius,
        }
    }

    /// A chamfer that removes `size` logical pixels from both sides.
    #[must_use]
    pub const fn cut(size: Length) -> Self {
        Self {
            kind: CornerKind::Cut,
            size,
        }
    }

    /// The corner's kind.
    #[must_use]
    pub const fn kind(self) -> CornerKind {
        self.kind
    }

    /// The radius or cut in logical pixels; zero for [`Self::SQUARE`].
    #[must_use]
    pub const fn size(self) -> Length {
        self.size
    }
}

/// A corner's place on the rectangle, clockwise from the top left.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum CornerPosition {
    /// Toward negative x and negative y.
    TopLeft,
    /// Toward positive x and negative y.
    TopRight,
    /// Toward positive x and positive y.
    BottomRight,
    /// Toward negative x and positive y.
    BottomLeft,
}

impl CornerPosition {
    const ALL: [Self; 4] = [
        Self::TopLeft,
        Self::TopRight,
        Self::BottomRight,
        Self::BottomLeft,
    ];

    const fn index(self) -> usize {
        match self {
            Self::TopLeft => 0,
            Self::TopRight => 1,
            Self::BottomRight => 2,
            Self::BottomLeft => 3,
        }
    }
}

/// A rectangle's silhouette: one [`Corner`] per position. Dimensions are
/// logical pixels. The shape affects painting only; widgets retain their
/// rectangular layout and hit bounds.
///
/// ```
/// # use pert_ui_runtime as pert_ui;
/// use pert_ui::{Corner, CornerKind, CornerPosition, Length, Shape};
/// let shape = Shape::rounded(Length::new(8.0)?)
///     .with_corner(CornerPosition::TopRight, Corner::cut(Length::new(12.0)?));
/// assert_eq!(shape.corner(CornerPosition::TopRight).kind(), CornerKind::Cut);
/// assert_eq!(shape.corner(CornerPosition::BottomLeft).size().get(), 8.0);
/// assert_eq!(Shape::default(), Shape::RECTANGLE);
/// # Ok::<(), pert_ui::GeometryError>(())
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Shape([Corner; 4]);

impl Default for Shape {
    fn default() -> Self {
        Self::RECTANGLE
    }
}

impl Shape {
    /// Four square corners.
    pub const RECTANGLE: Self = Self([Corner::SQUARE; 4]);

    /// Four round corners of one radius.
    #[must_use]
    pub const fn rounded(radius: Length) -> Self {
        Self([Corner::round(radius); 4])
    }

    /// Round upper corners and square lower ones, for silhouettes such as
    /// tabs that attach to a boundary below.
    #[must_use]
    pub const fn rounded_top(radius: Length) -> Self {
        Self([
            Corner::round(radius),
            Corner::round(radius),
            Corner::SQUARE,
            Corner::SQUARE,
        ])
    }

    /// A chamfer at the upper right.
    #[must_use]
    pub const fn cut_top_right(size: Length) -> Self {
        Self([
            Corner::SQUARE,
            Corner::cut(size),
            Corner::SQUARE,
            Corner::SQUARE,
        ])
    }

    /// Replaces the corner at `position`.
    #[must_use]
    pub const fn with_corner(mut self, position: CornerPosition, corner: Corner) -> Self {
        self.0[position.index()] = corner;
        self
    }

    /// The corner at `position`.
    #[must_use]
    pub const fn corner(self, position: CornerPosition) -> Corner {
        self.0[position.index()]
    }
}

impl Shape {
    /// The shape with every corner size multiplied by `factor`.
    fn scaled(self, factor: f32) -> Result<Self, GeometryError> {
        let mut corners = self.0;
        for corner in &mut corners {
            corner.size = Length::new(corner.size.get() * factor)?;
        }
        Ok(Self(corners))
    }
}

impl Border {
    /// The border with its width multiplied by `factor`, a view scale.
    pub(crate) fn scaled(self, factor: f32) -> Result<Self, GeometryError> {
        Ok(Self {
            width: Length::new(self.width.get() * factor)?,
            paint: self.paint,
        })
    }
}

impl ShapeGeometry {
    /// Whether the geometry cannot touch a pixel of `clip` in a frame of
    /// `viewport`'s own extent. A rectangle covers no pixel outside itself.
    /// An arc's or a triangle's antialiasing reaches up to one physical
    /// pixel past its bounds. An arc's radius and width map by the
    /// viewport's scale while its center and the clip map by the frame's
    /// pixels per logical unit, which differ when the logical size is not a
    /// whole number of pixels, so its reach is taken in the frame's terms.
    pub(crate) fn misses(self, clip: Rect, viewport: crate::Viewport) -> bool {
        let ramp = 1.0 / viewport.scale();
        let (x0, y0, x1, y1) = match self {
            Self::Rect { rect, .. } => {
                let [x, y, w, h] = rect.values().map(f64::from);
                (x, y, x + w, y + h)
            }
            Self::Arc(arc) => {
                let reach = f64::from(arc.radius.get()) + f64::from(arc.width.get()) * 0.5 + ramp;
                // Logical units of each axis per logical unit of a length.
                // An axis without frame pixels gives no finite factor, and
                // then that axis drops nothing.
                let extent = viewport.frame_extent();
                let stretch = |pixels: u32, logical: f32| {
                    viewport.scale() * f64::from(logical) / f64::from(pixels)
                };
                let reach_x = reach * stretch(extent.width(), viewport.width());
                let reach_y = reach * stretch(extent.height(), viewport.height());
                let (x, y) = (f64::from(arc.center.x()), f64::from(arc.center.y()));
                (x - reach_x, y - reach_y, x + reach_x, y + reach_y)
            }
            Self::Triangle(triangle) => {
                let xs = triangle.0.map(|point| f64::from(point.x()));
                let ys = triangle.0.map(|point| f64::from(point.y()));
                (
                    xs.into_iter().fold(f64::INFINITY, f64::min) - ramp,
                    ys.into_iter().fold(f64::INFINITY, f64::min) - ramp,
                    xs.into_iter().fold(f64::NEG_INFINITY, f64::max) + ramp,
                    ys.into_iter().fold(f64::NEG_INFINITY, f64::max) + ramp,
                )
            }
        };
        let [cx, cy, cw, ch] = clip.values().map(f64::from);
        x1 <= cx || x0 >= cx + cw || y1 <= cy || y0 >= cy + ch
    }

    /// The geometry of a component at `origin` under the view scale `view`:
    /// positions are mapped and lengths multiplied. `None` when nothing is
    /// left to draw: an empty rectangle, or a triangle whose mapped points
    /// are collinear.
    pub(crate) fn placed(
        self,
        origin: (f64, f64),
        view: f32,
    ) -> Result<Option<Self>, GeometryError> {
        let point = |point: Point| {
            Point::new(
                (origin.0 + f64::from(view) * f64::from(point.x())) as f32,
                (origin.1 + f64::from(view) * f64::from(point.y())) as f32,
            )
        };
        Ok(match self {
            Self::Rect { rect, shape } => {
                let rect = crate::transform::map_rect(rect, origin, view)
                    .ok_or(GeometryError::NonFinite)?;
                (!rect.is_empty()).then_some(Self::Rect {
                    rect,
                    shape: shape.scaled(view)?,
                })
            }
            Self::Arc(arc) => Some(Self::Arc(ArcGeometry {
                center: point(arc.center)?,
                radius: Length::new(arc.radius.get() * view)?,
                width: Length::new(arc.width.get() * view)?,
                ..arc
            })),
            Self::Triangle(triangle) => {
                let [a, b, c] = triangle.0;
                TriangleGeometry::new([point(a)?, point(b)?, point(c)?]).map(Self::Triangle)
            }
        })
    }
}

/// A circular arc stroked with round caps, in window logical coordinates.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ArcGeometry {
    center: Point,
    radius: Length,
    start: f32,
    sweep: f32,
    width: Length,
}

impl ArcGeometry {
    /// An arc around `center` from `start_degrees`, clockwise from the
    /// positive x axis with y down, over `sweep_degrees`. The start is reduced
    /// modulo 360 in `f64`.
    ///
    /// # Errors
    /// [`PathError::InvalidParameter`] for a non-finite angle or a sweep
    /// outside `0..=360`.
    pub fn new(
        center: Point,
        radius: Length,
        start_degrees: f32,
        sweep_degrees: f32,
        width: Length,
    ) -> Result<Self, PathError> {
        if !start_degrees.is_finite()
            || !sweep_degrees.is_finite()
            || !(0.0..=360.0).contains(&sweep_degrees)
        {
            return Err(PathError::InvalidParameter);
        }
        // A tiny negative angle reduces to just below 360, which narrows to
        // 360 itself; that is the same direction as 0.
        let reduced = f64::from(start_degrees).rem_euclid(360.0) as f32;
        Ok(Self {
            center,
            radius,
            start: if reduced >= 360.0 { 0.0 } else { reduced },
            sweep: sweep_degrees,
            width,
        })
    }

    /// The circle's center.
    #[must_use]
    pub const fn center(self) -> Point {
        self.center
    }

    /// The radius of the stroke's center line.
    #[must_use]
    pub const fn radius(self) -> Length {
        self.radius
    }

    /// The reduced start angle, in `0..360`.
    #[must_use]
    pub const fn start_degrees(self) -> f32 {
        self.start
    }

    /// The sweep, in `0..=360`.
    #[must_use]
    pub const fn sweep_degrees(self) -> f32 {
        self.sweep
    }

    /// The stroke width.
    #[must_use]
    pub const fn width(self) -> Length {
        self.width
    }
}

/// A filled triangle in window logical coordinates; never degenerate.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TriangleGeometry([Point; 3]);

impl TriangleGeometry {
    /// `None` for collinear or coincident points, by the `f64` cross-product
    /// test `fill_triangle` applies.
    pub fn new(vertices: [Point; 3]) -> Option<Self> {
        let [a, b, c] = vertices;
        let dx = |p: Point| f64::from(p.x()) - f64::from(a.x());
        let dy = |p: Point| f64::from(p.y()) - f64::from(a.y());
        (dx(b) * dy(c) - dy(b) * dx(c) != 0.0).then_some(Self(vertices))
    }

    /// The three vertices, in the order given.
    #[must_use]
    pub const fn vertices(self) -> [Point; 3] {
        self.0
    }
}

/// What a [`DrawCommand::DrawShape`](crate::DrawCommand::DrawShape) draws.
/// Every value is valid by construction.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub enum ShapeGeometry {
    /// A rectangle in window logical coordinates with its corners.
    Rect {
        /// The rectangle before snapping.
        rect: Rect,
        /// Its four corners.
        shape: Shape,
    },
    /// A circular arc stroked with round caps.
    Arc(ArcGeometry),
    /// A filled triangle.
    Triangle(TriangleGeometry),
}

/// Why a shape could not be prepared.
///
/// With the `serde` feature it implements `Serialize` and `Deserialize`.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[non_exhaustive]
pub enum ShapePrepareError {
    /// A parameter, the span or a point an evaluation forms from them (a
    /// pixel center, a corner's center, an arc's far side) lies farther than
    /// 65 536 physical pixels from the span's origin, where `f32` evaluation
    /// is no longer precise enough.
    #[error("shape geometry lies outside the range that can be evaluated")]
    OutOfRange,
    /// A border was given for an arc or a triangle, which have none.
    #[error("an arc or a triangle cannot have a border")]
    BorderNotAllowed,
}

/// What fills a shape or its border.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub enum Paint {
    /// One straight-alpha linear color.
    Solid(Color),
}

/// A rectangle's border: a width in logical pixels and a paint. It lies
/// inside the rectangle and is drawn in whole physical pixels, at least one
/// for a positive width; a zero width is no border.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Border {
    width: Length,
    paint: Paint,
}

impl Border {
    /// A border of `width` logical pixels.
    #[must_use]
    pub const fn new(width: Length, paint: Paint) -> Self {
        Self { width, paint }
    }

    /// The width in logical pixels.
    #[must_use]
    pub const fn width(self) -> Length {
        self.width
    }

    /// What the border is painted with.
    #[must_use]
    pub const fn paint(self) -> Paint {
        self.paint
    }
}

/// What [`Painter::draw_shape`](crate::Painter::draw_shape) paints: a fill, a
/// border, or both as one silhouette. A later fill or border replaces the
/// earlier one; neither accumulates.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ShapeStyle {
    pub(crate) fill: Option<Paint>,
    pub(crate) border: Option<Border>,
}

impl ShapeStyle {
    /// A fill and no border.
    #[must_use]
    pub const fn fill(color: Color) -> Self {
        Self {
            fill: Some(Paint::Solid(color)),
            border: None,
        }
    }

    /// A border of `width` logical pixels and no fill.
    #[must_use]
    pub const fn border(width: Length, color: Color) -> Self {
        Self {
            fill: None,
            border: Some(Border::new(width, Paint::Solid(color))),
        }
    }

    /// Sets the fill, replacing an earlier one.
    #[must_use]
    pub const fn with_fill(mut self, color: Color) -> Self {
        self.fill = Some(Paint::Solid(color));
        self
    }

    /// Sets the border, replacing an earlier one.
    #[must_use]
    pub const fn with_border(mut self, width: Length, color: Color) -> Self {
        self.border = Some(Border::new(width, Paint::Solid(color)));
        self
    }
}

/// How window logical coordinates map to a frame's physical pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FrameMapping {
    width: u32,
    height: u32,
    sx: f64,
    sy: f64,
    scale: f64,
}

impl FrameMapping {
    /// The mapping of `viewport` onto a frame of the given physical size:
    /// positions scale by the frame's extent over the viewport's, per axis,
    /// and lengths by the viewport's scale.
    ///
    /// # Errors
    /// [`GeometryError::Empty`] for a zero frame or viewport extent.
    pub fn new(
        frame_width: u32,
        frame_height: u32,
        viewport: Viewport,
    ) -> Result<Self, GeometryError> {
        if frame_width == 0
            || frame_height == 0
            || viewport.width() <= 0.0
            || viewport.height() <= 0.0
        {
            return Err(GeometryError::Empty);
        }
        Ok(Self {
            width: frame_width,
            height: frame_height,
            sx: f64::from(frame_width) / f64::from(viewport.width()),
            sy: f64::from(frame_height) / f64::from(viewport.height()),
            scale: viewport.scale(),
        })
    }

    /// The frame's width in physical pixels.
    #[must_use]
    pub const fn frame_width(&self) -> u32 {
        self.width
    }

    /// The frame's height in physical pixels.
    #[must_use]
    pub const fn frame_height(&self) -> u32 {
        self.height
    }

    /// The physical edge of a logical position: the first pixel whose center
    /// lies at or beyond it, which is the span rule solid rectangles use.
    fn snap_x(&self, x: f64) -> f64 {
        (x * self.sx - 0.5).ceil()
    }

    fn snap_y(&self, y: f64) -> f64 {
        (y * self.sy - 0.5).ceil()
    }
}

/// A corner as evaluated: its kind and its size in physical pixels. This is
/// output of [`PreparedShape::new`] for a renderer to read, not geometry to
/// construct.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PreparedCorner {
    kind: CornerKind,
    size: f32,
}

impl PreparedCorner {
    /// The corner's kind.
    #[must_use]
    pub const fn kind(self) -> CornerKind {
        self.kind
    }

    /// The radius or cut in physical pixels, after the overlap rule; zero
    /// when the corner contributes nothing.
    #[must_use]
    pub const fn size(self) -> f32 {
        self.size
    }
}

/// One boundary of a prepared rectangle, in physical pixels relative to the
/// span's origin. Output of [`PreparedShape::new`] for a renderer to read.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Boundary {
    edges: [f32; 4],
    corners: [PreparedCorner; 4],
}

impl Boundary {
    /// The edges `x0, y0, x1, y1`, relative to the span's origin. Snapping
    /// puts every edge on a whole pixel. An edge far outside the span is
    /// then moved to within the largest corner size plus two pixels of it,
    /// which leaves coverage in the span unchanged; a moved edge is a whole
    /// pixel only when that corner size is, and a backend keeps the value as
    /// given.
    #[must_use]
    pub const fn edges(self) -> [f32; 4] {
        self.edges
    }

    /// The corners, clockwise from the top left.
    #[must_use]
    pub const fn corners(self) -> [PreparedCorner; 4] {
        self.corners
    }

    /// The columns of the row at `y` (a pixel center's coordinate relative
    /// to the span) whose pixel centers lie at least half a pixel inside
    /// every edge and clear of every corner: `[first, past)` in pixel-center
    /// coordinates minus a half, or `None`.
    fn interior(&self, y: f32) -> Option<[f32; 2]> {
        let [x0, y0, x1, y1] = self.edges;
        if y < y0 + 0.5 || y > y1 - 0.5 {
            return None;
        }
        let (mut left, mut right) = (x0, x1);
        for position in CornerPosition::ALL {
            let PreparedCorner { kind, size } = self.corners[position.index()];
            if size <= 0.0 {
                continue;
            }
            let top = matches!(position, CornerPosition::TopLeft | CornerPosition::TopRight);
            let inward = if top { y - y0 } else { y1 - y };
            // How far from the corner's vertical edge a pixel center must
            // lie to be clear of the corner on this row.
            let clear = match kind {
                CornerKind::Square => continue,
                CornerKind::Round if inward < size => size,
                // A chamfer's coverage reaches 1 where the half-plane term
                // is at most -0.5; one pixel is a safe margin.
                CornerKind::Cut if inward < size + 1.0 => size + 1.0 - inward,
                _ => continue,
            };
            if matches!(
                position,
                CornerPosition::TopLeft | CornerPosition::BottomLeft
            ) {
                left = left.max(x0 + clear);
            } else {
                right = right.min(x1 - clear);
            }
        }
        // Pixel `i` has its center at `i + 0.5`; it qualifies when that lies
        // in `[left + 0.5, right - 0.5]`.
        let (first, past) = (left.ceil(), right.floor());
        (past > first).then_some([first, past])
    }
}

/// What lies inside a rectangle's border.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Inner {
    /// No border: the whole silhouette is fill.
    NoBorder,
    /// A border at least as wide as half the smaller extent: the whole
    /// silhouette is border.
    Filled,
    /// The border leaves this inner boundary.
    Area(Boundary),
}

/// The values the per-pixel evaluation uses, relative to the span's origin.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub enum ShapeParameters {
    /// A rectangle with corners and, with a border, its inner boundary.
    Rect {
        /// The silhouette.
        outer: Boundary,
        /// What the border leaves.
        inner: Inner,
    },
    /// A stroked arc.
    Arc {
        /// The circle's center.
        center: [f32; 2],
        /// The center line's radius.
        radius: f32,
        /// Half the evaluated width: at least half a pixel.
        half_width: f32,
        /// Cosine and sine of the sweep's middle direction.
        middle: [f32; 2],
        /// Cosine and sine of half the sweep.
        half_sweep: [f32; 2],
        /// Whether the sweep is the full circle, which has no caps.
        full: bool,
        /// The coverage factor of a stroke thinner than one pixel, else 1.
        thin: f32,
    },
    /// A triangle.
    Triangle {
        /// The three vertices.
        vertices: [[f32; 2]; 3],
        /// The sign of the winding: 1 or -1.
        winding: f32,
    },
}

/// The reference evaluation of one pixel.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ShapeCoverage {
    outer: f32,
    border: f32,
    outer_distance: f32,
    inner_distance: Option<f32>,
    exact_arithmetic: bool,
}

impl ShapeCoverage {
    const UNTOUCHED: Self = Self {
        outer: 0.0,
        border: 0.0,
        outer_distance: f32::INFINITY,
        inner_distance: None,
        exact_arithmetic: true,
    };

    /// The silhouette's coverage, `c_o`, in `0..=1`.
    #[must_use]
    pub const fn outer(self) -> f32 {
        self.outer
    }

    /// The border's share of that coverage, `c_b`, in `0..=outer`.
    #[must_use]
    pub const fn border(self) -> f32 {
        self.border
    }

    /// The outer distance the ramp was applied to, in physical pixels,
    /// negative inside, before the thin-stroke factor.
    #[must_use]
    pub const fn outer_distance(self) -> f32 {
        self.outer_distance
    }

    /// The inner distance, when a border leaves an inner area.
    #[must_use]
    pub const fn inner_distance(self) -> Option<f32> {
        self.inner_distance
    }

    /// Whether exact subtractions and comparisons alone decided this pixel's
    /// saturated coverage, so that every renderer agrees on it: a pixel of
    /// a rectangle outside every outer and inner round-corner region, with
    /// every cut term at most `−0.5` less the parity margin, where either
    /// component of `q` is at least `0.5` (coverage 0) or both are at most
    /// `−0.5` (coverage 1). A renderer may take this exit early.
    #[must_use]
    pub const fn exact_arithmetic(self) -> bool {
        self.exact_arithmetic
    }
}

/// A shape ready to evaluate for one frame.
///
/// [`Self::new`] prepares a shape in `f64` and narrows the result to `f32`;
/// [`Self::coverage`] evaluates it. The two together define what a shape
/// covers. A backend that evaluates [`Self::parameters`] itself, as the wgpu
/// renderer does in its fragment stage, reproduces the formulas below.
///
/// # Preparation
///
/// With `sx` and `sy` the frame pixels per logical unit of each axis and `s`
/// the viewport scale ([`FrameMapping`]), in this order:
///
/// 1. **Map.** A position maps to `x·sx`, `y·sy`; a length (a corner size, a
///    border width, an arc's radius or width) maps to `ℓ·s`.
/// 2. **Snap a rectangle.** Each edge `e` becomes `ceil(e·sx − 0.5)` (`sy`
///    for the top and bottom edges), the rule a `FillRect` follows on the CPU. A
///    snapped width or height of zero draws nothing.
/// 3. **Overlap rule**, as CSS backgrounds: for each side whose two mapped
///    corner sizes have a positive sum, take the snapped side length divided
///    by that sum. If the smallest such ratio is below 1, every corner size
///    is multiplied by it. A side with two square corners imposes no limit.
/// 4. **Border width.** A zero width is no border. A rectangle's border is
///    `W = max(1, floor(w·s + 0.5))` whole pixels. An arc's width stays
///    `w·s`, unsnapped.
/// 5. **Inner geometry.** If `2W` is at least the smaller snapped extent,
///    there is no inner area and the whole silhouette is border. Otherwise
///    the inner rectangle is the snapped one inset by `W`, with each corner
///    keeping its kind: a round corner of size `r` becomes `max(r − W, 0)`
///    and a cut corner `max(r − (2 − √2)·W, 0)`, the exact inward offsets.
/// 6. **Span.** The pixels to evaluate are the bounding box grown by one
///    pixel (the snapped rectangle, the arc's square of radius plus half
///    width, or the triangle's box, rounded outward), intersected with the
///    frame and with the clip, whose edges snap as in step 2. Every span
///    edge is a whole pixel, so a GPU quad covers exactly the span and no
///    rasterizer edge test decides a pixel.
/// 7. **Relocate and bound.** Every parameter is expressed relative to the
///    span's origin before it is narrowed to `f32`.
///    - A rectangle edge farther beyond the span than `M`, the largest
///      corner size plus 2 pixels, moves to the span edge plus `M`; coverage
///      inside the span is unchanged. Inner edges were derived in step 5 and
///      are relocated on their own, never rebuilt from relocated outer ones.
///    - Every parameter, and every point an evaluation forms from them, must
///      then lie within 65 536 pixels of the span's origin, or preparation
///      fails with [`ShapePrepareError::OutOfRange`].
///    - A triangle is tested again on its narrowed vertices: two coincident
///      ones, an edge shorter than 1/256 pixel or a zero cross product draw
///      nothing.
///    - An arc's `cos h`, `sin h` and its rotation are computed here.
///      Evaluation uses only `+ − × /`, `sqrt`, `min`, `max`, `abs` and
///      comparisons.
///
/// # Distances
///
/// At a pixel center `p = (i + 0.5, j + 0.5)` relative to the span's origin,
/// in `f32`. A formula that is a bound rather than the Euclidean distance
/// still defines the coverage. `length` of a vector with a zero component
/// is the other component's magnitude, without a `sqrt`.
///
/// **Rectangle**, with edges `x0, y0, x1, y1`:
///
/// ```text
/// q     = (max(x0 − p.x, p.x − x1), max(y0 − p.y, p.y − y1))
/// d_box = min(max(q.x, q.y), 0) + length(max(q, 0))
/// d     = max(d_box, every corner term)
/// ```
///
/// A corner of size `r > 0` adds one term; for the top-left corner, the
/// others mirrored:
///
/// - round, only where `p.x < x0 + r` and `p.y < y0 + r`:
///   `length(p − (x0 + r, y0 + r)) − r`. A round corner is bounded by its
///   own size, not by the rectangle's centerlines, so one corner may take a
///   whole side.
/// - cut, everywhere, as a half-plane: `(x0 + y0 + r − p.x − p.y) / √2`.
///
/// **Arc**, with radius `R`, half width `t`, half sweep `h` and `p'` the
/// offset from the center rotated so that the arc's middle lies on `+x`:
///
/// ```text
/// full ring:                 d = |length(p') − R| − t
/// p'' = (p'.x, |p'.y|)
/// p''.y·cos h ≤ p''.x·sin h: d = |length(p'') − R| − t      (in the sector)
/// otherwise:                 d = length(p'' − R·(cos h, sin h)) − t   (cap)
/// ```
///
/// **Triangle:** the least distance to the three segments, negative where
/// the three edge functions agree with the winding.
///
/// # Coverage
///
/// ```text
/// c(d) = clamp(0.5 − d, 0, 1)
/// c_o  = c(d_outer)
/// c_i  = min(c(d_inner), c_o)   with a border and an inner area
///      = 0                      with a border and no inner area
///      = c_o                    without a border
/// c_b  = c_o − c_i
/// ```
///
/// An arc narrower than one physical pixel is evaluated with a width of one
/// pixel and its coverage multiplied by its width in pixels, caps and a full
/// ring included, so it fades out instead of approaching half coverage.
/// [`DrawCommand::DrawShape`](crate::DrawCommand::DrawShape) states how the
/// two coverages composite the fill and the border.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PreparedShape {
    span: [u32; 4],
    parameters: ShapeParameters,
}

/// A half-open span of frame pixels in `f64`, whole numbers within the frame.
#[derive(Clone, Copy)]
struct Span {
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
}

impl Span {
    /// The box `[x0, x1) × [y0, y1)` of whole pixels intersected with the
    /// frame and the snapped clip; `None` when nothing remains or the span
    /// exceeds the domain.
    fn new(
        bounds: [f64; 4],
        clip: Option<Rect>,
        mapping: &FrameMapping,
    ) -> Result<Option<Self>, ShapePrepareError> {
        let [mut x0, mut y0, mut x1, mut y1] = bounds;
        x0 = x0.max(0.0);
        y0 = y0.max(0.0);
        x1 = x1.min(f64::from(mapping.width));
        y1 = y1.min(f64::from(mapping.height));
        if let Some(clip) = clip {
            let [x, y, w, h] = clip.values().map(f64::from);
            x0 = x0.max(mapping.snap_x(x));
            y0 = y0.max(mapping.snap_y(y));
            x1 = x1.min(mapping.snap_x(x + w));
            y1 = y1.min(mapping.snap_y(y + h));
        }
        // NaN bounds compare false and so draw nothing.
        if !(x1 > x0 && y1 > y0) {
            return Ok(None);
        }
        if x1 - x0 > DOMAIN || y1 - y0 > DOMAIN {
            return Err(ShapePrepareError::OutOfRange);
        }
        Ok(Some(Self { x0, y0, x1, y1 }))
    }

    fn pixels(self) -> [u32; 4] {
        // Whole numbers within the frame, whose extent is a `u32`.
        [
            self.x0 as u32,
            self.y0 as u32,
            self.x1 as u32,
            self.y1 as u32,
        ]
    }
}

impl Span {
    /// How far the point `(x, y)`, relative to the span's origin, lies from
    /// the farthest pixel of the span, per axis. An evaluation subtracts a
    /// parameter from a pixel center, so this bounds what it forms.
    fn reach(self, x: f64, y: f64) -> [f64; 2] {
        [
            x.abs().max((x - (self.x1 - self.x0)).abs()),
            y.abs().max((y - (self.y1 - self.y0)).abs()),
        ]
    }
}

/// Narrows values that must lie within the domain.
fn narrow<const N: usize>(values: [f64; N]) -> Result<[f32; N], ShapePrepareError> {
    if values
        .iter()
        .any(|value| value.is_nan() || value.abs() > DOMAIN)
    {
        return Err(ShapePrepareError::OutOfRange);
    }
    Ok(values.map(|value| value as f32))
}

impl PreparedShape {
    /// The margin, in physical pixels, that separates a clear pixel from an
    /// edge pixel when two renderers are compared: a pixel is clear when each
    /// of its reference distances is at least half a pixel plus this margin
    /// from its boundary, so that its coverage is 0 or 1 in every renderer.
    /// A parity run confirmed it on the one adapter available; see the
    /// rendering contract in `docs/rendering-platform.md`.
    pub const PARITY_MARGIN: f32 = PARITY_MARGIN;

    /// Prepares `geometry` for a frame. `border` is a rectangle's border,
    /// of which only the width matters here; a zero width is no border.
    /// `clip` is the command's clip in window logical coordinates.
    ///
    /// `Ok(None)` means there is nothing to draw: the shape collapsed, its
    /// span is empty or the clip removed it.
    ///
    /// # Errors
    /// [`ShapePrepareError::BorderNotAllowed`] for a border on an arc or a
    /// triangle, and [`ShapePrepareError::OutOfRange`] when a parameter or the
    /// span lies beyond the evaluation domain.
    pub fn new(
        geometry: &ShapeGeometry,
        border: Option<&Border>,
        clip: Option<Rect>,
        mapping: &FrameMapping,
    ) -> Result<Option<Self>, ShapePrepareError> {
        match *geometry {
            ShapeGeometry::Rect { rect, shape } => Self::rect(
                rect,
                shape,
                border.map(|border| border.width),
                clip,
                mapping,
            ),
            ShapeGeometry::Arc(_) | ShapeGeometry::Triangle(_) if border.is_some() => {
                Err(ShapePrepareError::BorderNotAllowed)
            }
            ShapeGeometry::Arc(arc) => Self::arc(arc, clip, mapping),
            ShapeGeometry::Triangle(triangle) => Self::triangle(triangle, clip, mapping),
        }
    }

    fn rect(
        rect: Rect,
        corners: Shape,
        border: Option<Length>,
        clip: Option<Rect>,
        mapping: &FrameMapping,
    ) -> Result<Option<Self>, ShapePrepareError> {
        // Steps 1 and 2: map and snap the edges.
        let [x, y, w, h] = rect.values().map(f64::from);
        let edges = [
            mapping.snap_x(x),
            mapping.snap_y(y),
            mapping.snap_x(x + w),
            mapping.snap_y(y + h),
        ];
        let (width, height) = (edges[2] - edges[0], edges[3] - edges[1]);
        if !(width > 0.0 && height > 0.0) {
            return Ok(None);
        }
        // Step 3: the overlap rule on the mapped corner sizes.
        let mut sizes = corners.0.map(|corner| match corner.kind {
            CornerKind::Square => 0.0,
            _ => f64::from(corner.size.get()) * mapping.scale,
        });
        // Each side with the corners at its ends: top, right, bottom, left.
        let sides = [(0, 1, width), (1, 2, height), (2, 3, width), (3, 0, height)];
        let factor = sides
            .iter()
            .filter_map(|&(a, b, length)| {
                let sum = sizes[a] + sizes[b];
                (sum > 0.0).then(|| length / sum)
            })
            .fold(1.0f64, f64::min);
        if factor < 1.0 {
            sizes = sizes.map(|size| size * factor);
        }
        let kinds = corners.0.map(|corner| corner.kind);
        // Step 4: a border is whole physical pixels, at least one.
        let border = border
            .map(|width| f64::from(width.get()) * mapping.scale)
            .filter(|width| *width > 0.0)
            .map(|width| (width + 0.5).floor().max(1.0));
        // Step 5: the inner geometry, from the unrelocated outer one.
        let inner = match border {
            None => None,
            Some(border) if 2.0 * border >= width.min(height) => Some(None),
            Some(border) => {
                let sizes = std::array::from_fn(|index| match kinds[index] {
                    CornerKind::Square => 0.0,
                    CornerKind::Round => (sizes[index] - border).max(0.0),
                    CornerKind::Cut => {
                        (sizes[index] - (2.0 - std::f64::consts::SQRT_2) * border).max(0.0)
                    }
                });
                Some(Some((
                    [
                        edges[0] + border,
                        edges[1] + border,
                        edges[2] - border,
                        edges[3] - border,
                    ],
                    sizes,
                )))
            }
        };
        // Step 6: the span, one pixel beyond the snapped rectangle.
        let Some(span) = Span::new(
            [
                edges[0] - 1.0,
                edges[1] - 1.0,
                edges[2] + 1.0,
                edges[3] + 1.0,
            ],
            clip,
            mapping,
        )?
        else {
            return Ok(None);
        };
        // Step 7: relative to the span, far edges moved in, then narrowed.
        let boundary = |edges: [f64; 4], sizes: [f64; 4]| -> Result<Boundary, ShapePrepareError> {
            let margin = sizes.iter().fold(0.0f64, |a, b| a.max(*b)) + 2.0;
            let relative = [
                (edges[0] - span.x0).max(-margin),
                (edges[1] - span.y0).max(-margin),
                (edges[2] - span.x0).min(span.x1 - span.x0 + margin),
                (edges[3] - span.y0).min(span.y1 - span.y0 + margin),
            ];
            let edges = narrow(relative)?;
            let sizes = narrow(sizes)?;
            // Pixel centers minus the edges stay in the domain as well.
            narrow(span.reach(relative[0], relative[1]))?;
            narrow(span.reach(relative[2], relative[3]))?;
            // The corner centers an evaluation forms stay in the domain too.
            for index in 0..4 {
                let (near_x, near_y) = match CornerPosition::ALL[index] {
                    CornerPosition::TopLeft => (relative[0] + sizes_at(&sizes, index), relative[1]),
                    CornerPosition::TopRight => {
                        (relative[2] - sizes_at(&sizes, index), relative[1])
                    }
                    CornerPosition::BottomRight => {
                        (relative[2] - sizes_at(&sizes, index), relative[3])
                    }
                    CornerPosition::BottomLeft => {
                        (relative[0] + sizes_at(&sizes, index), relative[3])
                    }
                };
                narrow([near_x, near_y])?;
            }
            Ok(Boundary {
                edges,
                corners: std::array::from_fn(|index| PreparedCorner {
                    kind: kinds[index],
                    size: sizes[index],
                }),
            })
        };
        let outer = boundary(edges, sizes)?;
        let inner = match inner {
            None => Inner::NoBorder,
            Some(None) => Inner::Filled,
            Some(Some((edges, sizes))) => Inner::Area(boundary(edges, sizes)?),
        };
        Ok(Some(Self {
            span: span.pixels(),
            parameters: ShapeParameters::Rect { outer, inner },
        }))
    }

    fn arc(
        arc: ArcGeometry,
        clip: Option<Rect>,
        mapping: &FrameMapping,
    ) -> Result<Option<Self>, ShapePrepareError> {
        let radius = f64::from(arc.radius.get()) * mapping.scale;
        let width = f64::from(arc.width.get()) * mapping.scale;
        if arc.sweep == 0.0 || width <= 0.0 {
            return Ok(None);
        }
        // A stroke thinner than a pixel is evaluated one pixel wide and its
        // coverage scaled, so it fades out instead of resting at the ramp's
        // half.
        let (half_width, thin) = if width < 1.0 {
            (0.5, width)
        } else {
            (width / 2.0, 1.0)
        };
        let center = [
            f64::from(arc.center.x()) * mapping.sx,
            f64::from(arc.center.y()) * mapping.sy,
        ];
        let reach = radius + half_width;
        let Some(span) = Span::new(
            [
                (center[0] - reach).floor() - 1.0,
                (center[1] - reach).floor() - 1.0,
                (center[0] + reach).ceil() + 1.0,
                (center[1] + reach).ceil() + 1.0,
            ],
            clip,
            mapping,
        )?
        else {
            return Ok(None);
        };
        let half = f64::from(arc.sweep).to_radians() / 2.0;
        let middle = f64::from(arc.start).to_radians() + half;
        // A pixel center minus the arc's center, rotated, and then minus a
        // cap's center is at most this far from the origin.
        let [far_x, far_y] = span.reach(center[0] - span.x0, center[1] - span.y0);
        narrow([far_x.hypot(far_y) + radius])?;
        let [cx, cy, radius, half_width] =
            narrow([center[0] - span.x0, center[1] - span.y0, radius, half_width])?;
        Ok(Some(Self {
            span: span.pixels(),
            parameters: ShapeParameters::Arc {
                center: [cx, cy],
                radius,
                half_width,
                middle: [middle.cos() as f32, middle.sin() as f32],
                half_sweep: [half.cos() as f32, half.sin() as f32],
                full: arc.sweep == 360.0,
                thin: thin as f32,
            },
        }))
    }

    fn triangle(
        triangle: TriangleGeometry,
        clip: Option<Rect>,
        mapping: &FrameMapping,
    ) -> Result<Option<Self>, ShapePrepareError> {
        let mapped = triangle.0.map(|point| {
            [
                f64::from(point.x()) * mapping.sx,
                f64::from(point.y()) * mapping.sy,
            ]
        });
        let fold = |axis: usize, pick: fn(f64, f64) -> f64| {
            mapped
                .iter()
                .map(|point| point[axis])
                .reduce(pick)
                .unwrap_or(0.0)
        };
        let Some(span) = Span::new(
            [
                fold(0, f64::min).floor() - 1.0,
                fold(1, f64::min).floor() - 1.0,
                fold(0, f64::max).ceil() + 1.0,
                fold(1, f64::max).ceil() + 1.0,
            ],
            clip,
            mapping,
        )?
        else {
            return Ok(None);
        };
        let mut vertices = [[0.0f32; 2]; 3];
        for (vertex, point) in vertices.iter_mut().zip(mapped) {
            let relative = [point[0] - span.x0, point[1] - span.y0];
            narrow(span.reach(relative[0], relative[1]))?;
            *vertex = narrow(relative)?;
        }
        // The triangle may have collapsed on narrowing: test what will be
        // evaluated, so evaluation never meets a zero-length edge.
        let wide = vertices.map(|vertex| vertex.map(f64::from));
        let edge =
            |from: usize, to: usize| [wide[to][0] - wide[from][0], wide[to][1] - wide[from][1]];
        let (ab, ac) = (edge(0, 1), edge(0, 2));
        let cross = ab[0] * ac[1] - ab[1] * ac[0];
        let shortest = [edge(0, 1), edge(1, 2), edge(2, 0)]
            .iter()
            .map(|edge| edge[0].hypot(edge[1]))
            .fold(f64::INFINITY, f64::min);
        if cross == 0.0 || shortest < SHORTEST_EDGE {
            return Ok(None);
        }
        Ok(Some(Self {
            span: span.pixels(),
            parameters: ShapeParameters::Triangle {
                vertices,
                winding: if cross > 0.0 { 1.0 } else { -1.0 },
            },
        }))
    }

    /// The frame pixels to evaluate: `[x0, y0, x1, y1]`, half-open.
    #[must_use]
    pub const fn span(&self) -> [u32; 4] {
        self.span
    }

    /// The evaluated parameters, relative to the span's origin.
    #[must_use]
    pub const fn parameters(&self) -> ShapeParameters {
        self.parameters
    }

    /// The columns `[first, past)` of the frame row `row` where a rectangle
    /// is solid and uniform: every pixel there has an outer coverage of
    /// exactly 1 and the same border coverage, 0 inside the border's inner
    /// area or without a border, 1 where the border takes the whole
    /// silhouette. A renderer may fill them as one run with the
    /// [`Self::coverage`] of any one of them. `None` when the row has no such
    /// run, and always for arcs and triangles. The run is conservative: pixels
    /// outside it may be solid too.
    #[must_use]
    pub fn solid_run(&self, row: u32) -> Option<[u32; 2]> {
        let [x0, y0, x1, y1] = self.span;
        if row < y0 || row >= y1 {
            return None;
        }
        let ShapeParameters::Rect { outer, inner } = self.parameters else {
            return None;
        };
        let boundary = match inner {
            Inner::Area(inner) => inner,
            Inner::NoBorder | Inner::Filled => outer,
        };
        let [first, past] = boundary.interior((row - y0) as f32 + 0.5)?;
        // Relative to the span, clamped to it; whole numbers within `u32`.
        let width = (x1 - x0) as f32;
        let (first, past) = (first.clamp(0.0, width), past.clamp(0.0, width));
        (past > first).then(|| [x0 + first as u32, x0 + past as u32])
    }

    /// The frame pixels `[x0, y0, x1, y1]`, half-open, of a box inside a
    /// rectangle where every row's [`Self::solid_run`] covers the box's
    /// columns: every pixel there has an outer coverage of exactly 1 and the
    /// border coverage of any one of them. A renderer may draw the box as one
    /// uniform area and evaluate only the rest of [`Self::span`]. The box
    /// is conservative: it leaves out every row a corner reaches, so pixels
    /// outside it may be solid too, and a rectangle whose corners reach
    /// every row, as a pill's do, has none although its rows have runs.
    /// `None` then, and always for arcs and triangles.
    #[must_use]
    pub fn solid_box(&self) -> Option<[u32; 4]> {
        let [x0, y0, x1, y1] = self.span;
        let ShapeParameters::Rect { outer, inner } = self.parameters else {
            return None;
        };
        let boundary = match inner {
            Inner::Area(inner) => inner,
            Inner::NoBorder | Inner::Filled => outer,
        };
        // How many rows from its horizontal edge a corner reaches, as
        // `Boundary::interior` clears them.
        let reach = |position: CornerPosition| {
            let PreparedCorner { kind, size } = boundary.corners[position.index()];
            match kind {
                CornerKind::Round if size > 0.0 => size,
                CornerKind::Cut if size > 0.0 => size + 1.0,
                _ => 0.0,
            }
        };
        let [left, top, right, bottom] = boundary.edges;
        let top = top + reach(CornerPosition::TopLeft).max(reach(CornerPosition::TopRight));
        let bottom =
            bottom - reach(CornerPosition::BottomLeft).max(reach(CornerPosition::BottomRight));
        // Relative to the span, clamped to it; whole numbers within `u32`.
        let (width, height) = ((x1 - x0) as f32, (y1 - y0) as f32);
        let (first, past) = (
            left.ceil().clamp(0.0, width),
            right.floor().clamp(0.0, width),
        );
        let (above, below) = (
            top.ceil().clamp(0.0, height),
            bottom.floor().clamp(0.0, height),
        );
        (past > first && below > above).then(|| {
            [
                x0 + first as u32,
                y0 + above as u32,
                x0 + past as u32,
                y0 + below as u32,
            ]
        })
    }

    /// The reference evaluation of the frame pixel `(x, y)`, at its center
    /// `(x + 0.5, y + 0.5)`. Outside [`Self::span`] the shape leaves the pixel
    /// unchanged: zero coverage, an infinite outer distance, no inner
    /// distance, and exact arithmetic.
    pub fn coverage(&self, x: u32, y: u32) -> ShapeCoverage {
        let [x0, y0, x1, y1] = self.span;
        if x < x0 || x >= x1 || y < y0 || y >= y1 {
            return ShapeCoverage::UNTOUCHED;
        }
        // Whole numbers below 2^16 plus a half: exact in `f32`.
        let p = [(x - x0) as f32 + 0.5, (y - y0) as f32 + 0.5];
        match self.parameters {
            ShapeParameters::Rect { outer, inner } => {
                let (outer_distance, outer_exact) = outer.distance(p);
                let outer = ramp(outer_distance);
                let (inside, inner_distance, exact) = match inner {
                    Inner::NoBorder => (outer, None, outer_exact),
                    Inner::Filled => (0.0, None, outer_exact),
                    Inner::Area(inner) => {
                        let (distance, exact) = inner.distance(p);
                        // The minimum keeps the nesting where a formula is a
                        // bound rather than the exact distance.
                        (
                            ramp(distance).min(outer),
                            Some(distance),
                            outer_exact && exact,
                        )
                    }
                };
                ShapeCoverage {
                    outer,
                    border: outer - inside,
                    outer_distance,
                    inner_distance,
                    exact_arithmetic: exact,
                }
            }
            ShapeParameters::Arc {
                center,
                radius,
                half_width,
                middle,
                half_sweep,
                full,
                thin,
            } => {
                let (dx, dy) = (p[0] - center[0], p[1] - center[1]);
                let distance = if full {
                    (length(dx, dy) - radius).abs() - half_width
                } else {
                    // Rotate so the sweep's middle lies on +x, and fold.
                    let px = dx * middle[0] + dy * middle[1];
                    let py = (dy * middle[0] - dx * middle[1]).abs();
                    if py * half_sweep[0] <= px * half_sweep[1] {
                        (length(px, py) - radius).abs() - half_width
                    } else {
                        length(px - radius * half_sweep[0], py - radius * half_sweep[1])
                            - half_width
                    }
                };
                ShapeCoverage {
                    outer: ramp(distance) * thin,
                    border: 0.0,
                    outer_distance: distance,
                    inner_distance: None,
                    exact_arithmetic: false,
                }
            }
            ShapeParameters::Triangle { vertices, winding } => {
                let mut nearest = f32::INFINITY;
                let mut inside = true;
                for index in 0..3 {
                    let (a, b) = (vertices[index], vertices[(index + 1) % 3]);
                    let edge = [b[0] - a[0], b[1] - a[1]];
                    let to = [p[0] - a[0], p[1] - a[1]];
                    let along = ((to[0] * edge[0] + to[1] * edge[1])
                        / (edge[0] * edge[0] + edge[1] * edge[1]))
                        .clamp(0.0, 1.0);
                    nearest = nearest.min(length(to[0] - edge[0] * along, to[1] - edge[1] * along));
                    inside &= winding * (edge[0] * to[1] - edge[1] * to[0]) >= 0.0;
                }
                let distance = if inside { -nearest } else { nearest };
                ShapeCoverage {
                    outer: ramp(distance),
                    border: 0.0,
                    outer_distance: distance,
                    inner_distance: None,
                    exact_arithmetic: false,
                }
            }
        }
    }
}

fn sizes_at(sizes: &[f32; 4], index: usize) -> f64 {
    f64::from(sizes[index])
}

/// The coverage of a distance: one pixel wide, centered on the boundary.
fn ramp(distance: f32) -> f32 {
    (0.5 - distance).clamp(0.0, 1.0)
}

/// The length of `(x, y)`. With a zero component it is the other one's
/// magnitude, without a square root, so axis-aligned distances stay exact.
fn length(x: f32, y: f32) -> f32 {
    if x == 0.0 {
        y.abs()
    } else if y == 0.0 {
        x.abs()
    } else {
        (x * x + y * y).sqrt()
    }
}

impl Boundary {
    /// The signed distance bound at `p`, and whether exact subtractions and
    /// comparisons alone decide its saturated coverage.
    fn distance(&self, p: [f32; 2]) -> (f32, bool) {
        let [x0, y0, x1, y1] = self.edges;
        let qx = (x0 - p[0]).max(p[0] - x1);
        let qy = (y0 - p[1]).max(p[1] - y1);
        let mut distance = qx.max(qy).min(0.0) + length(qx.max(0.0), qy.max(0.0));
        let mut in_round_region = false;
        let mut cuts_clear = true;
        for position in CornerPosition::ALL {
            let PreparedCorner { kind, size } = self.corners[position.index()];
            if size <= 0.0 {
                continue;
            }
            // The corner's own edges, and the direction into the rectangle.
            let (ex, ey, sx, sy) = match position {
                CornerPosition::TopLeft => (x0, y0, 1.0, 1.0),
                CornerPosition::TopRight => (x1, y0, -1.0, 1.0),
                CornerPosition::BottomRight => (x1, y1, -1.0, -1.0),
                CornerPosition::BottomLeft => (x0, y1, 1.0, -1.0),
            };
            // How far `p` lies inward of the corner's edges, per axis.
            let (ix, iy) = (sx * (p[0] - ex), sy * (p[1] - ey));
            match kind {
                CornerKind::Square => {}
                CornerKind::Round => {
                    if ix < size && iy < size {
                        in_round_region = true;
                        distance = distance.max(length(ix - size, iy - size) - size);
                    }
                }
                CornerKind::Cut => {
                    let term = (size - ix - iy) / SQRT_2;
                    cuts_clear &= term <= -0.5 - PARITY_MARGIN;
                    distance = distance.max(term);
                }
            }
        }
        let saturated = qx >= 0.5 || qy >= 0.5 || (qx <= -0.5 && qy <= -0.5);
        (distance, !in_round_region && cuts_clear && saturated)
    }
}

#[cfg(test)]
mod tests;
