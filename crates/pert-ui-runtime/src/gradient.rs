//! Immutable gradient descriptions and the reference sampler used by renderers.
use crate::{Color, FrameMapping, Point, Size};
use std::{
    hash::{DefaultHasher, Hash, Hasher},
    sync::Arc,
};

/// Space in which premultiplied stop components are interpolated.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum GradientSpace {
    /// Perceptual Oklab. Converted back to linear sRGB and clipped before blending.
    #[default]
    Oklab,
    /// Linear sRGB light intensities.
    Linear,
    /// Gamma-encoded sRGB components.
    Srgb,
}

/// Mapping of positions outside the gradient interval.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum GradientExtend {
    /// Hold the nearest endpoint.
    #[default]
    Pad,
    /// Tile the ramp; each integer samples its beginning.
    Repeat,
    /// Alternate forward and backward ramps; odd integers sample the end.
    Reflect,
}

/// An invalid gradient description or unrepresentable frame mapping.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum GradientError {
    /// Supply 2–16 stops with finite, nondecreasing offsets in 0..=1.
    #[error("invalid gradient stops")]
    Stops,
    /// The linear axis is empty, or the ellipse or its inner fraction is invalid.
    #[error("invalid gradient geometry")]
    Geometry,
    /// The mapped geometry exceeds the sampler's finite evaluation domain.
    #[error("gradient geometry exceeds the frame evaluation range")]
    OutOfRange,
}

/// Index into one owned scene's gradient table. Resolve through
/// [`DisplayList::gradient`](crate::DisplayList::gradient); never persist across frames.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct GradientId(pub(crate) usize);
impl GradientId {
    /// Index within the scene that issued this ID.
    #[must_use]
    pub const fn index(self) -> usize {
        self.0
    }
}

#[derive(Clone, Debug, PartialEq)]
struct Stops {
    original: Vec<(f32, Color)>,
    prepared: Vec<[f32; 4]>,
    space: GradientSpace,
    key: u64,
}
fn lab([r, g, b]: [f64; 3]) -> [f64; 3] {
    let l = (0.4122214708 * r + 0.5363325363 * g + 0.0514459929 * b).cbrt();
    let m = (0.2119034982 * r + 0.6806995451 * g + 0.1073969566 * b).cbrt();
    let s = (0.0883024619 * r + 0.2817188376 * g + 0.6299787005 * b).cbrt();
    [
        0.2104542553 * l + 0.793617785 * m - 0.0040720468 * s,
        1.9779984951 * l - 2.428592205 * m + 0.4505937099 * s,
        0.0259040371 * l + 0.7827717662 * m - 0.808675766 * s,
    ]
}
fn from_lab([light, a, b]: [f32; 3]) -> [f32; 3] {
    let l = (light + 0.39633778 * a + 0.21580376 * b).powi(3);
    let m = (light - 0.105561346 * a - 0.06385417 * b).powi(3);
    let s = (light - 0.08948418 * a - 1.2914855 * b).powi(3);
    [
        4.0767417 * l - 3.3077116 * m + 0.23096994 * s,
        -1.268438 * l + 2.6097574 * m - 0.3413194 * s,
        -0.0041960864 * l - 0.7034186 * m + 1.7076147 * s,
    ]
    .map(|v| v.clamp(0., 1.))
}
impl Stops {
    fn new(stops: &[(f32, Color)], space: GradientSpace) -> Result<Arc<Self>, GradientError> {
        if !(2..=Gradient::MAX_STOPS).contains(&stops.len())
            || stops.iter().any(|(t, _)| !(0. ..=1.).contains(t))
            || stops.windows(2).any(|w| w[0].0 > w[1].0)
        {
            return Err(GradientError::Stops);
        }
        Ok(Self::prepare(stops, space))
    }
    fn prepare(stops: &[(f32, Color)], space: GradientSpace) -> Arc<Self> {
        let mut hash = DefaultHasher::new();
        space.hash(&mut hash);
        let prepared = stops
            .iter()
            .map(|(t, color)| {
                for v in std::iter::once(*t).chain(color.channels()) {
                    (if v == 0. { 0 } else { v.to_bits() }).hash(&mut hash);
                }
                let [r, g, b, a] = color.channels();
                let components = match space {
                    GradientSpace::Oklab => lab([r, g, b].map(f64::from)).map(|v| v as f32),
                    GradientSpace::Linear => [r, g, b],
                    GradientSpace::Srgb => {
                        let [r, g, b, _] = color.to_srgb();
                        [r, g, b]
                    }
                };
                [components[0] * a, components[1] * a, components[2] * a, a]
            })
            .collect();
        Arc::new(Self {
            original: stops.to_vec(),
            prepared,
            space,
            key: hash.finish(),
        })
    }
}

/// Immutable linear or concentric elliptical paint in component-local logical pixels.
/// Clones share their stops. New gradients use Oklab interpolation and pad extension.
/// Adjacent equal offsets create a jump; exactly at a jump the last stop wins.
/// Stop colors extend to the interval endpoints if those have no explicit stop.
///
/// ```
/// use pert_ui_runtime::{Color, Gradient, Point};
/// let title = Gradient::linear(Point::default(), Point::new(40., 0.)?,
///     [(0., Color::ACCENT), (1., Color::SURFACE)])?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct Gradient {
    geometry: [f64; 4],
    inner: Option<f32>,
    extend: GradientExtend,
    stops: Arc<Stops>,
}
impl Gradient {
    /// Maximum number of stops admitted at construction.
    pub const MAX_STOPS: usize = 16;
    /// Linear ramp along the segment from `start` to `end`; clipping never rescales it.
    ///
    /// # Errors
    /// Returns [`GradientError::Geometry`] for coincident endpoints or
    /// [`GradientError::Stops`] for invalid offsets or count.
    pub fn linear(
        start: Point,
        end: Point,
        stops: impl AsRef<[(f32, Color)]>,
    ) -> Result<Self, GradientError> {
        let [x, y] = [start.x(), start.y()].map(f64::from);
        let dx = f64::from(end.x()) - x;
        let dy = f64::from(end.y()) - y;
        if dx == 0. && dy == 0. {
            return Err(GradientError::Geometry);
        }
        Ok(Self {
            geometry: [x, y, dx, dy],
            inner: None,
            extend: GradientExtend::Pad,
            stops: Stops::new(stops.as_ref(), GradientSpace::Oklab)?,
        })
    }
    /// Elliptical ramp centered at `center`. Both outer radii are positive logical
    /// lengths. The inner ellipse has the same center and radii multiplied by
    /// `inner`; its boundary is position zero and the outer ellipse is position one.
    ///
    /// # Errors
    /// Returns [`GradientError::Geometry`] for a zero radius or an inner fraction
    /// outside `0..1`, and [`GradientError::Stops`] for invalid stops.
    pub fn radial(
        center: Point,
        radii: Size,
        inner: f32,
        stops: impl AsRef<[(f32, Color)]>,
    ) -> Result<Self, GradientError> {
        if radii.width() <= 0. || radii.height() <= 0. || !(0. ..1.).contains(&inner) {
            return Err(GradientError::Geometry);
        }
        Ok(Self {
            geometry: [center.x(), center.y(), radii.width(), radii.height()].map(f64::from),
            inner: Some(inner),
            extend: GradientExtend::Pad,
            stops: Stops::new(stops.as_ref(), GradientSpace::Oklab)?,
        })
    }
    /// Replaces the interpolation space, preparing the bounded stop list once.
    #[must_use]
    pub fn with_space(mut self, space: GradientSpace) -> Self {
        if self.space() != space {
            self.stops = Stops::prepare(&self.stops.original, space);
        }
        self
    }
    /// Replaces the extension used outside the unit interval.
    #[must_use]
    pub fn with_extend(mut self, extend: GradientExtend) -> Self {
        self.extend = extend;
        self
    }
    /// The chosen interpolation space.
    #[must_use]
    pub fn space(&self) -> GradientSpace {
        self.stops.space
    }
    /// The chosen extension.
    #[must_use]
    pub fn extend(&self) -> GradientExtend {
        self.extend
    }
    /// Original offsets and straight linear colors, in caller order.
    pub fn stops(&self) -> &[(f32, Color)] {
        &self.stops.original
    }
    /// Components premultiplied in the chosen interpolation space, parallel to [`Self::stops`].
    pub fn components(&self) -> &[[f32; 4]] {
        &self.stops.prepared
    }
    /// Inner radius fraction for an ellipse, absent for linear geometry.
    #[must_use]
    pub fn inner(&self) -> Option<f32> {
        self.inner
    }
    pub(crate) fn placed(&self, origin: (f64, f64), view: f32) -> Self {
        let [x, y, dx, dy] = self.geometry.map(|v| v * f64::from(view));
        Self {
            geometry: [origin.0 + x, origin.1 + y, dx, dy],
            ..self.clone()
        }
    }
    /// Prepares a window-space scene gradient for physical-pixel evaluation.
    /// Returns the pixel origin and two coefficients. For a linear gradient
    /// take their dot product with displacement; for an ellipse multiply
    /// displacement componentwise and take the length before applying `inner`.
    ///
    /// # Errors
    /// Returns [`GradientError::OutOfRange`] for origins beyond 2²² pixels,
    /// or coefficients that overflow, underflow, or exceed 65,536 per pixel.
    pub fn coefficients(&self, mapping: &FrameMapping) -> Result<[f32; 4], GradientError> {
        let [sx, sy, _] = mapping.factors();
        let [x, y, dx, dy] = self.geometry;
        let (a, b) = if self.inner.is_some() {
            (1. / dx / sx, 1. / dy / sy)
        } else {
            let norm = dx * dx + dy * dy;
            (dx / norm / sx, dy / norm / sy)
        };
        if (x * sx).abs() > 4194304.
            || (y * sy).abs() > 4194304.
            || [a, b].iter().any(|v| {
                !v.is_finite()
                    || v.abs() > 65536.
                    || (*v != 0. && (*v as f32).abs() < f32::MIN_POSITIVE)
            })
        {
            return Err(GradientError::OutOfRange);
        }
        Ok([x * sx, y * sy, a, b].map(|v| v as f32))
    }
    /// Evaluates a pixel from [`Self::coefficients`], returning premultiplied
    /// linear RGBA including ordered RGB dithering anchored in the physical frame.
    #[must_use]
    pub fn sample(&self, [ox, oy, a, b]: [f32; 4], x: u32, y: u32) -> [f32; 4] {
        let dx = (x as f32 + 0.5 - ox) * a;
        let dy = (y as f32 + 0.5 - oy) * b;
        let t = match self.inner {
            Some(inner) => ((dx * dx + dy * dy).sqrt() - inner) / (1. - inner),
            None => dx + dy,
        };
        let t = match self.extend {
            GradientExtend::Pad => t.clamp(0., 1.),
            GradientExtend::Repeat => t - t.floor(),
            GradientExtend::Reflect => 1. - (1. - t.rem_euclid(2.)).abs(),
        };
        let stops = self.stops();
        let end = stops.partition_point(|s| s.0 <= t);
        let lower = end.saturating_sub(1);
        let (offset, color) = stops[lower];
        let [r, g, b, alpha] = if end == 0 || end == stops.len() || offset == t {
            color.channels()
        } else {
            let f = (t - offset) / (stops[end].0 - offset);
            let left = self.components()[lower];
            let right = self.components()[end];
            let [r, g, b, alpha] = std::array::from_fn(|i| left[i] * (1. - f) + right[i] * f);
            if alpha <= 0. {
                return [0.; 4];
            }
            let rgb = [r / alpha, g / alpha, b / alpha];
            let [r, g, b] = match self.space() {
                GradientSpace::Oklab => from_lab(rgb),
                GradientSpace::Linear => rgb,
                GradientSpace::Srgb => {
                    rgb.map(|v| crate::drawing::srgb_decode(f64::from(v.clamp(0., 1.))))
                }
            };
            [r, g, b, alpha]
        };
        if alpha == 0. {
            return [0.; 4];
        }
        let mut rank = 0;
        for bit in 0..3 {
            let i = (x >> bit) & 1;
            let j = (y >> bit) & 1;
            rank = 4 * rank + 2 * (i ^ j) + j;
        }
        let d = ((f64::from(rank) + 0.5) / 64. - 0.5) / 255.;
        let [r, g, b] = [r, g, b].map(|v| {
            crate::drawing::srgb_decode(
                (crate::drawing::srgb_encode(f64::from(v.clamp(0., 1.))) + d).clamp(0., 1.),
            ) * alpha
        });
        [r, g, b, alpha]
    }
}
impl crate::resources::SceneResource for Gradient {
    fn content_key(&self) -> u64 {
        let mut h = DefaultHasher::new();
        self.stops.key.hash(&mut h);
        self.extend.hash(&mut h);
        self.inner
            .map(|v| if v == 0. { 0 } else { v.to_bits() })
            .hash(&mut h);
        self.geometry
            .map(|v| if v == 0. { 0 } else { v.to_bits() })
            .hash(&mut h);
        h.finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn invalid_geometry_or_stops_are_rejected_before_recording() {
        let a = Point::default();
        let b = Point::new(1., 0.).unwrap();
        let c = Color::WHITE;
        let stops = [(0., c), (1., c)];
        assert_eq!(Gradient::linear(a, a, stops), Err(GradientError::Geometry));
        for offsets in [[-0.1, 1.], [0., 1.1], [0.7, 0.6], [f32::NAN, 1.]] {
            assert_eq!(
                Gradient::linear(a, b, offsets.map(|t| (t, c))),
                Err(GradientError::Stops)
            );
        }
        assert_eq!(
            Gradient::linear(a, b, [(0., c); 17]),
            Err(GradientError::Stops)
        );
        assert_eq!(
            Gradient::linear(a, b, [(0., c); 1]),
            Err(GradientError::Stops)
        );
        assert_eq!(
            Gradient::radial(a, Size::from([0, 1]), 0., stops),
            Err(GradientError::Geometry)
        );
        assert_eq!(
            Gradient::radial(a, Size::from([1, 1]), 1., stops),
            Err(GradientError::Geometry)
        );
    }
}
