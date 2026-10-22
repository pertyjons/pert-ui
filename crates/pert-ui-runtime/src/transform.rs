//! Validated transforms: affine maps of path geometry, quarter turns of masks
//! and images, and the uniform view transform a container gives its children.
//!
//! There is deliberately no transform stack on [`Painter`](crate::Painter). Each
//! type here keeps axis-aligned rectangles meaningful where the pipeline depends
//! on them; `docs/rendering-platform.md` records the reasoning.
use crate::{GeometryError, Point, Rect};

/// A validated, invertible 2×3 affine map of local logical coordinates.
///
/// It maps a point `(x, y)` to `(a·x + c·y + e, b·x + d·y + f)`. Every entry is
/// finite and the determinant `a·d − b·c` is nonzero, so the inverse always
/// exists. Angles are degrees clockwise in screen coordinates, the convention
/// [`PathBuilder::arc`](crate::PathBuilder::arc) already uses.
///
/// A transform applies to geometry, through
/// [`Path::transformed`](crate::Path::transformed); it is not a drawing state.
/// Compose with [`Self::then`]:
///
/// ```
/// # use pert_ui_runtime as pert_ui;
/// use pert_ui::{Point, Transform2D};
/// let center = Point::new(10.0, 10.0)?;
/// let quarter = Transform2D::rotation_degrees(90.0)?.about(center)?;
/// let moved = quarter.apply(Point::new(20.0, 10.0)?)?;
/// assert!((moved.x() - 10.0).abs() < 1e-4 && (moved.y() - 20.0).abs() < 1e-4);
/// let back = quarter.inverse()?.apply(moved)?;
/// assert!((back.x() - 20.0).abs() < 1e-4 && (back.y() - 10.0).abs() < 1e-4);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Transform2D {
    // Column-major: [a, b, c, d, e, f].
    m: [f64; 6],
}

impl Default for Transform2D {
    fn default() -> Self {
        Self::IDENTITY
    }
}

impl Transform2D {
    /// Leaves every point where it is.
    pub const IDENTITY: Self = Self {
        m: [1.0, 0.0, 0.0, 1.0, 0.0, 0.0],
    };

    /// Builds a transform from its six entries, mapping `(x, y)` to
    /// `(a·x + c·y + e, b·x + d·y + f)`.
    ///
    /// # Errors
    /// Returns [`GeometryError::NonFinite`] for a non-finite entry and
    /// [`GeometryError::Singular`] when the determinant is zero or not normal.
    pub fn new(a: f32, b: f32, c: f32, d: f32, e: f32, f: f32) -> Result<Self, GeometryError> {
        Self::checked([a, b, c, d, e, f].map(f64::from))
    }
    fn checked(m: [f64; 6]) -> Result<Self, GeometryError> {
        if m.iter().any(|value| !value.is_finite()) {
            return Err(GeometryError::NonFinite);
        }
        let determinant = m[0] * m[3] - m[1] * m[2];
        if !determinant.is_normal() {
            return Err(GeometryError::Singular);
        }
        Ok(Self { m })
    }
    /// Moves every point by `(x, y)` logical pixels.
    ///
    /// # Errors
    /// Returns [`GeometryError::NonFinite`] for a non-finite offset.
    pub fn translation(x: f32, y: f32) -> Result<Self, GeometryError> {
        Self::new(1.0, 0.0, 0.0, 1.0, x, y)
    }
    /// Scales about the origin by one factor per axis. A negative factor mirrors.
    ///
    /// # Errors
    /// Returns [`GeometryError::NonFinite`] for a non-finite factor and
    /// [`GeometryError::Singular`] for a zero factor.
    pub fn scale(x: f32, y: f32) -> Result<Self, GeometryError> {
        Self::new(x, 0.0, 0.0, y, 0.0, 0.0)
    }
    /// Rotates about the origin, degrees clockwise in screen coordinates.
    ///
    /// # Errors
    /// Returns [`GeometryError::NonFinite`] for a non-finite angle.
    pub fn rotation_degrees(degrees: f32) -> Result<Self, GeometryError> {
        if !degrees.is_finite() {
            return Err(GeometryError::NonFinite);
        }
        // Exact quarter turns keep axis-aligned geometry exactly axis-aligned.
        let (sin, cos) = match degrees.rem_euclid(360.0) {
            0.0 => (0.0, 1.0),
            90.0 => (1.0, 0.0),
            180.0 => (0.0, -1.0),
            270.0 => (-1.0, 0.0),
            _ => f64::from(degrees).to_radians().sin_cos(),
        };
        Self::checked([cos, sin, -sin, cos, 0.0, 0.0])
    }
    /// The same transform performed around `center` instead of the origin.
    ///
    /// # Errors
    /// Returns [`GeometryError::NonFinite`] when the recentered offset overflows.
    pub fn about(self, center: Point) -> Result<Self, GeometryError> {
        let (x, y) = (f64::from(center.x()), f64::from(center.y()));
        let [a, b, c, d, e, f] = self.m;
        Self::checked([a, b, c, d, e + x - a * x - c * y, f + y - b * x - d * y])
    }
    /// Applies `self` first and `next` afterwards.
    ///
    /// # Errors
    /// Returns [`GeometryError::NonFinite`] or [`GeometryError::Singular`] when
    /// the product leaves the representable, invertible range.
    pub fn then(self, next: Self) -> Result<Self, GeometryError> {
        let [a, b, c, d, e, f] = self.m;
        let [na, nb, nc, nd, ne, nf] = next.m;
        Self::checked([
            na * a + nc * b,
            nb * a + nd * b,
            na * c + nc * d,
            nb * c + nd * d,
            na * e + nc * f + ne,
            nb * e + nd * f + nf,
        ])
    }
    /// The transform that undoes this one.
    ///
    /// # Errors
    /// Returns [`GeometryError::NonFinite`] when the inverse of a nearly singular
    /// matrix is not representable.
    pub fn inverse(self) -> Result<Self, GeometryError> {
        let [a, b, c, d, e, f] = self.m;
        let determinant = a * d - b * c;
        Self::checked([
            d / determinant,
            -b / determinant,
            -c / determinant,
            a / determinant,
            (c * f - d * e) / determinant,
            (b * e - a * f) / determinant,
        ])
    }
    /// Maps one point.
    ///
    /// # Errors
    /// Returns [`GeometryError::NonFinite`] when the result is not a finite
    /// `f32` coordinate.
    pub fn apply(self, point: Point) -> Result<Point, GeometryError> {
        let (x, y) = self.map(f64::from(point.x()), f64::from(point.y()));
        Point::new(x as f32, y as f32)
    }
    pub(crate) fn map(self, x: f64, y: f64) -> (f64, f64) {
        let [a, b, c, d, e, f] = self.m;
        (a * x + c * y + e, b * x + d * y + f)
    }
    /// The six entries `[a, b, c, d, e, f]`, for uploading or inspection.
    #[must_use]
    pub fn entries(self) -> [f64; 6] {
        self.m
    }
}

/// One of the four rotations that keep an axis-aligned rectangle axis-aligned.
///
/// Masks, images, text and icons accept a quarter turn, clockwise in screen
/// coordinates. The destination rectangle is where the turned picture appears, so
/// after [`QuarterTurn::Right`] or [`QuarterTurn::Left`] a picture that was wide
/// occupies a tall rectangle. The same coverage or pixels are sampled, so a turn
/// rasterizes nothing and changes no cache key. The default is
/// [`QuarterTurn::None`].
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum QuarterTurn {
    /// Upright, as recorded.
    #[default]
    None,
    /// 90 degrees clockwise: the picture's top edge faces right. Text reads
    /// downwards.
    Right,
    /// 180 degrees: upside down.
    Half,
    /// 90 degrees counterclockwise: the picture's top edge faces left. Text reads
    /// upwards, the usual direction for a vertical axis label.
    Left,
}

impl QuarterTurn {
    /// Whether this turn exchanges width and height.
    #[must_use]
    pub const fn swaps_axes(self) -> bool {
        matches!(self, Self::Right | Self::Left)
    }
    /// The extent a picture of `size` occupies after this turn.
    #[must_use]
    pub fn turned(self, size: crate::Size) -> crate::Size {
        if self.swaps_axes() {
            crate::Size::new(size.height(), size.width()).unwrap_or(size)
        } else {
            size
        }
    }
    /// Where a part of an upright picture lands once the picture is turned.
    ///
    /// `upright` is the picture's extent before turning and `part` a rectangle
    /// inside it; the result is that part's rectangle inside the turned extent
    /// [`Self::turned`] reports, from the turned picture's own top-left. Drawing
    /// each part there with this turn reproduces the whole picture turned.
    ///
    /// ```
    /// # use pert_ui_runtime as pert_ui;
    /// use pert_ui::{QuarterTurn, Rect, Size};
    /// let upright = Size::new(100.0, 20.0)?;
    /// let first_glyph = Rect::new(0.0, 0.0, 10.0, 20.0)?;
    /// // Read upwards, the start of the line is at the bottom.
    /// let placed = QuarterTurn::Left.place(upright, first_glyph)?;
    /// assert_eq!(placed.values(), [0.0, 90.0, 20.0, 10.0]);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// # Errors
    /// Returns [`GeometryError::NonFinite`] when a derived coordinate is not
    /// finite.
    pub fn place(self, upright: crate::Size, part: Rect) -> Result<Rect, GeometryError> {
        let [x, y, w, h] = part.values();
        let (width, height) = (upright.width(), upright.height());
        match self {
            Self::None => Ok(part),
            Self::Right => Rect::new(height - (y + h), x, h, w),
            Self::Half => Rect::new(width - (x + w), height - (y + h), w, h),
            Self::Left => Rect::new(y, width - (x + w), h, w),
        }
    }
    /// Source coordinates, as fractions of the upright picture, sampled at the
    /// destination's top-left, top-right, bottom-right and bottom-left corners.
    ///
    /// A backend draws the destination rectangle with these per-corner source
    /// coordinates; cropping by a clip interpolates between them.
    #[must_use]
    pub const fn corners(self) -> [[f32; 2]; 4] {
        const TL: [f32; 2] = [0.0, 0.0];
        const TR: [f32; 2] = [1.0, 0.0];
        const BR: [f32; 2] = [1.0, 1.0];
        const BL: [f32; 2] = [0.0, 1.0];
        match self {
            Self::None => [TL, TR, BR, BL],
            Self::Right => [BL, TL, TR, BR],
            Self::Half => [BR, BL, TL, TR],
            Self::Left => [TR, BR, BL, TL],
        }
    }
    /// Maps a point given as fractions of the destination rectangle to fractions
    /// of the upright source picture.
    #[must_use]
    pub fn source(self, u: f32, v: f32) -> [f32; 2] {
        match self {
            Self::None => [u, v],
            Self::Right => [v, 1.0 - u],
            Self::Half => [1.0 - u, 1.0 - v],
            Self::Left => [1.0 - v, u],
        }
    }
}

/// A validated uniform scale a container applies to its children.
///
/// Finite and within [`Self::MIN`]`..=`[`Self::MAX`]; anything else is rejected
/// rather than clamped, like [`UiZoom`](crate::UiZoom). The default is 1.
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct ViewScale(f32);

impl Default for ViewScale {
    fn default() -> Self {
        Self::ONE
    }
}

impl ViewScale {
    /// Smallest supported scale, a factor of 1/32.
    pub const MIN: Self = Self(1.0 / 32.0);
    /// Largest supported scale, a factor of 32.
    pub const MAX: Self = Self(32.0);
    /// No scaling.
    pub const ONE: Self = Self(1.0);
    /// The default smallest scale of a zoom gesture.
    pub(crate) const EIGHTH: Self = Self(0.125);
    /// The default largest scale of a zoom gesture.
    pub(crate) const EIGHT: Self = Self(8.0);

    /// Validates a factor.
    ///
    /// # Errors
    /// Returns [`GeometryError::NonFinite`] or [`GeometryError::ViewScaleOutOfRange`].
    pub fn new(factor: f32) -> Result<Self, GeometryError> {
        if !factor.is_finite() {
            return Err(GeometryError::NonFinite);
        }
        if !(Self::MIN.0..=Self::MAX.0).contains(&factor) {
            return Err(GeometryError::ViewScaleOutOfRange);
        }
        Ok(Self(factor))
    }
    /// The factor.
    #[must_use]
    pub const fn get(self) -> f32 {
        self.0
    }
}

impl TryFrom<f32> for ViewScale {
    type Error = GeometryError;
    /// Validates like [`ViewScale::new`].
    fn try_from(factor: f32) -> Result<Self, Self::Error> {
        Self::new(factor)
    }
}

/// How a container presents its children: one uniform scale and one offset.
///
/// Returned by [`Component::child_transform`](crate::Component::child_transform).
/// A child's local point `p`, placed at `child_origin` by the container's
/// `arrange`, appears at `offset + scale · (child_origin + p)` in the container's
/// own local space. The offset is in the container's local logical pixels and is
/// not scaled. Uniform scale with translation maps axis-aligned rectangles to
/// axis-aligned rectangles, so clipping, hit testing and culling keep working on
/// rectangles. The default is the identity.
///
/// ```
/// # use pert_ui_runtime as pert_ui;
/// use pert_ui::{Point, ViewScale, ViewTransform};
/// let view = ViewTransform::new(ViewScale::new(2.0)?, Point::new(10.0, 0.0)?);
/// let shown = view.apply(Point::new(3.0, 4.0)?)?;
/// assert_eq!((shown.x(), shown.y()), (16.0, 8.0));
/// let content = view.invert(shown)?;
/// assert_eq!((content.x(), content.y()), (3.0, 4.0));
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ViewTransform {
    scale: ViewScale,
    offset: Point,
}

impl ViewTransform {
    /// No scale and no offset.
    pub const IDENTITY: Self = Self {
        scale: ViewScale::ONE,
        offset: Point::ZERO,
    };
    /// A transform from its two validated parts.
    #[must_use]
    pub const fn new(scale: ViewScale, offset: Point) -> Self {
        Self { scale, offset }
    }
    /// The uniform scale.
    #[must_use]
    pub const fn scale(self) -> ViewScale {
        self.scale
    }
    /// The offset, in the container's local logical pixels.
    #[must_use]
    pub const fn offset(self) -> Point {
        self.offset
    }
    /// Whether children are presented exactly where they are arranged.
    #[must_use]
    pub fn is_identity(self) -> bool {
        self == Self::IDENTITY
    }
    /// Maps a content-space point into the container's local space.
    ///
    /// # Errors
    /// Returns [`GeometryError::NonFinite`] when the result overflows.
    pub fn apply(self, content: Point) -> Result<Point, GeometryError> {
        // In `f64`, like the runtime's own placement, so both agree for content
        // far from the origin.
        let scale = f64::from(self.scale.0);
        narrow(
            f64::from(self.offset.x()) + scale * f64::from(content.x()),
            f64::from(self.offset.y()) + scale * f64::from(content.y()),
        )
    }
    /// Maps a point in the container's local space back into content space.
    ///
    /// # Errors
    /// Returns [`GeometryError::NonFinite`] when the result overflows.
    pub fn invert(self, local: Point) -> Result<Point, GeometryError> {
        let (x, y) = self.invert64(local);
        narrow(x, y)
    }
    fn invert64(self, local: Point) -> (f64, f64) {
        let scale = f64::from(self.scale.0);
        (
            (f64::from(local.x()) - f64::from(self.offset.x())) / scale,
            (f64::from(local.y()) - f64::from(self.offset.y())) / scale,
        )
    }
    /// The transform with the same content point kept under `anchor` while the
    /// scale changes to `scale`: what zooming at the pointer needs. `anchor` is in
    /// the container's local space.
    ///
    /// # Errors
    /// Returns [`GeometryError::NonFinite`] when the new offset overflows.
    pub fn zoomed_at(self, scale: ViewScale, anchor: Point) -> Result<Self, GeometryError> {
        let (x, y) = self.invert64(anchor);
        let factor = f64::from(scale.0);
        let offset = narrow(
            f64::from(anchor.x()) - factor * x,
            f64::from(anchor.y()) - factor * y,
        )?;
        Ok(Self { scale, offset })
    }
}

fn narrow(x: f64, y: f64) -> Result<Point, GeometryError> {
    Point::new(x as f32, y as f32)
}

/// Maps a local rectangle through an origin and a uniform scale, in `f64`, so
/// distant content multiplied by a large scale keeps its precision near the window.
pub(crate) fn map_rect(rect: Rect, origin: (f64, f64), scale: f32) -> Option<Rect> {
    let scale = f64::from(scale);
    let [x, y, w, h] = rect.values().map(f64::from);
    Rect::new(
        (origin.0 + scale * x) as f32,
        (origin.1 + scale * y) as f32,
        (scale * w) as f32,
        (scale * h) as f32,
    )
    .ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_invalid_transforms() {
        assert_eq!(
            Transform2D::scale(0.0, 1.0),
            Err(GeometryError::Singular),
            "a collapsed axis has no inverse"
        );
        assert_eq!(
            Transform2D::translation(f32::NAN, 0.0),
            Err(GeometryError::NonFinite)
        );
        assert_eq!(
            Transform2D::rotation_degrees(f32::INFINITY),
            Err(GeometryError::NonFinite)
        );
        assert_eq!(ViewScale::new(0.0), Err(GeometryError::ViewScaleOutOfRange));
        assert_eq!(
            ViewScale::new(64.0),
            Err(GeometryError::ViewScaleOutOfRange)
        );
        assert_eq!(ViewScale::new(f32::NAN), Err(GeometryError::NonFinite));
    }

    #[test]
    fn composition_applies_left_to_right() {
        let moved = Transform2D::translation(10.0, 0.0).unwrap();
        let doubled = Transform2D::scale(2.0, 2.0).unwrap();
        let p = Point::new(1.0, 1.0).unwrap();
        let a = moved.then(doubled).unwrap().apply(p).unwrap();
        assert_eq!((a.x(), a.y()), (22.0, 2.0));
        let b = doubled.then(moved).unwrap().apply(p).unwrap();
        assert_eq!((b.x(), b.y()), (12.0, 2.0));
    }

    #[test]
    fn quarter_turn_corners_agree_with_source() {
        for turn in [
            QuarterTurn::None,
            QuarterTurn::Right,
            QuarterTurn::Half,
            QuarterTurn::Left,
        ] {
            let corners = turn.corners();
            for (corner, (u, v)) in
                corners
                    .iter()
                    .zip([(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)])
            {
                assert_eq!(*corner, turn.source(u, v), "{turn:?}");
            }
        }
    }

    #[test]
    fn zooming_keeps_the_anchor_fixed() {
        let view = ViewTransform::new(ViewScale::new(1.5).unwrap(), Point::new(7.0, -3.0).unwrap());
        let anchor = Point::new(40.0, 25.0).unwrap();
        let before = view.invert(anchor).unwrap();
        let zoomed = view
            .zoomed_at(ViewScale::new(4.0).unwrap(), anchor)
            .unwrap();
        let after = zoomed.invert(anchor).unwrap();
        assert!((before.x() - after.x()).abs() < 1e-4);
        assert!((before.y() - after.y()).abs() < 1e-4);
    }
}
