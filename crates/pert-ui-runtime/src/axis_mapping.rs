//! Mapping a domain value to pixels along one axis, for views that zoom per axis.
use crate::GeometryError;

/// Maps a domain value, such as a time in seconds or a sample index, to a local
/// pixel along one axis, and back.
///
/// A timeline zooms in time while its rows keep their height, and a plot zooms
/// each axis independently. Neither is a drawing transform: text and strokes
/// stretched along one axis are never what such a view wants, so it maps
/// positions itself and draws undistorted, one `AxisMapping` per zooming axis.
/// `docs/rendering-platform.md` records why this is a mapping and not a transform.
///
/// The mapping is the domain value shown at pixel zero, [`Self::start`], and a
/// scale in [`Self::pixels_per_unit`]. Both are `f64` and a position is computed
/// relative to the start, so a value far from zero, such as sample 3,000,000,000
/// of a long recording, keeps its precision near the window, which a single
/// global `f32` coordinate would not. [`Self::pixel`] returns `f64`; narrow it to
/// `f32` for drawing after leaving out what lies outside the view.
///
/// ```
/// use pert_ui_runtime::AxisMapping;
/// // Ten seconds from 60 s, over 500 pixels.
/// let time = AxisMapping::new(60.0, 50.0)?;
/// assert_eq!(time.pixel(62.0), 100.0);
/// assert_eq!(time.value(250.0), 65.0);
/// // Zoom in twofold, keeping the moment under pixel 100 where it is.
/// let zoomed = time.zoomed_at(100.0, 100.0)?;
/// assert_eq!(zoomed.pixel(62.0), 100.0);
/// assert_eq!(zoomed.visible(500.0), (61.0, 66.0));
/// // Ruler marks at least 40 pixels apart: every half second.
/// let marks: Vec<f64> = zoomed.ticks(500.0, 40.0).collect();
/// assert_eq!(marks.first(), Some(&61.0));
/// assert_eq!(marks[1], 61.5);
/// # Ok::<(), pert_ui_runtime::GeometryError>(())
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AxisMapping {
    start: f64,
    pixels_per_unit: f64,
}

/// The most marks [`AxisMapping::ticks`] yields, whatever it is asked for.
const MAX_TICKS: usize = 10_000;

impl AxisMapping {
    /// A mapping that shows `start` at pixel zero and `pixels_per_unit` pixels
    /// per domain unit.
    ///
    /// # Errors
    /// Returns [`GeometryError::NonFinite`] when either is not finite and
    /// [`GeometryError::NonPositiveScale`] when the scale is not a positive
    /// normal number.
    pub fn new(start: f64, pixels_per_unit: f64) -> Result<Self, GeometryError> {
        if !start.is_finite() || !pixels_per_unit.is_finite() {
            return Err(GeometryError::NonFinite);
        }
        if !(pixels_per_unit > 0.0 && pixels_per_unit.is_normal()) {
            return Err(GeometryError::NonPositiveScale);
        }
        Ok(Self {
            start,
            pixels_per_unit,
        })
    }
    /// The domain value shown at pixel zero.
    #[must_use]
    pub const fn start(self) -> f64 {
        self.start
    }
    /// Pixels per domain unit.
    #[must_use]
    pub const fn pixels_per_unit(self) -> f64 {
        self.pixels_per_unit
    }
    /// The pixel at which `value` is shown, relative to the start.
    #[must_use]
    pub fn pixel(self, value: f64) -> f64 {
        (value - self.start) * self.pixels_per_unit
    }
    /// The domain value shown at `pixel`.
    #[must_use]
    pub fn value(self, pixel: f64) -> f64 {
        self.start + pixel / self.pixels_per_unit
    }
    /// The domain range shown by `extent` pixels from pixel zero, as start and
    /// end.
    #[must_use]
    pub fn visible(self, extent: f64) -> (f64, f64) {
        (self.start, self.value(extent))
    }
    /// The mapping at another scale that keeps the value under the pixel
    /// `anchor` where it is: what zooming at the pointer needs.
    ///
    /// The start moves by `anchor · (1/old − 1/new)`, computed without the
    /// anchor's absolute value, so zooming far from zero loses no precision and
    /// zooming to the same scale returns the mapping unchanged.
    ///
    /// # Errors
    /// As [`Self::new`] for the scale; [`GeometryError::NonFinite`] for a
    /// non-finite anchor or a start that overflows.
    pub fn zoomed_at(self, pixels_per_unit: f64, anchor: f64) -> Result<Self, GeometryError> {
        let zoomed = Self::new(self.start, pixels_per_unit)?;
        if !anchor.is_finite() {
            return Err(GeometryError::NonFinite);
        }
        if pixels_per_unit == self.pixels_per_unit {
            return Ok(self);
        }
        let shift = anchor * (self.pixels_per_unit.recip() - pixels_per_unit.recip());
        Self::new(zoomed.start + shift, pixels_per_unit)
    }
    /// The mapping with the content moved by `pixels`, positive to move it
    /// toward higher pixels, which shows earlier values.
    ///
    /// # Errors
    /// Returns [`GeometryError::NonFinite`] when the start overflows.
    pub fn panned(self, pixels: f64) -> Result<Self, GeometryError> {
        Self::new(
            self.start - pixels / self.pixels_per_unit,
            self.pixels_per_unit,
        )
    }
    /// The smallest step of 1, 2 or 5 times a power of ten, in domain units,
    /// whose marks lie at least `min_pixels` apart; `None` unless `min_pixels`
    /// is positive and finite.
    #[must_use]
    pub fn tick_step(self, min_pixels: f64) -> Option<f64> {
        if !(min_pixels > 0.0 && min_pixels.is_finite()) {
            return None;
        }
        let raw = min_pixels / self.pixels_per_unit;
        let base = 10f64.powf(raw.log10().floor());
        [1.0, 2.0, 5.0, 10.0]
            .into_iter()
            .map(|multiple| multiple * base)
            .find(|step| *step >= raw)
            .filter(|step| step.is_finite() && *step > 0.0)
    }
    /// The domain values of ruler marks inside the `extent` pixels from pixel
    /// zero, at [`Self::tick_step`]`(min_pixels)`: finite, strictly increasing
    /// and within the visible range. Each is computed from its index, so marks
    /// do not accumulate rounding error. At most 10,000 marks are yielded.
    /// Nothing is yielded for an invalid spacing or extent, and marks stop where
    /// `f64` can no longer tell one from the next, as it cannot for fine marks
    /// far from zero.
    pub fn ticks(self, extent: f64, min_pixels: f64) -> impl Iterator<Item = f64> {
        let step = self
            .tick_step(min_pixels)
            .filter(|_| extent.is_finite() && extent >= 0.0);
        let (first, start, end, step) = match step {
            Some(step) => (
                (self.start / step).ceil(),
                self.start,
                self.value(extent),
                step,
            ),
            None => (f64::NAN, 0.0, 0.0, 1.0),
        };
        let mut previous = f64::NEG_INFINITY;
        (0..MAX_TICKS)
            .map(move |index| (first + index as f64) * step)
            .skip_while(move |value| value.is_finite() && *value < start)
            .take_while(move |value| {
                let next = value.is_finite() && *value > previous && *value <= end;
                previous = *value;
                next
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_invalid_mappings() {
        assert_eq!(
            AxisMapping::new(f64::NAN, 1.0),
            Err(GeometryError::NonFinite)
        );
        assert_eq!(
            AxisMapping::new(0.0, f64::INFINITY),
            Err(GeometryError::NonFinite)
        );
        assert_eq!(
            AxisMapping::new(0.0, 0.0),
            Err(GeometryError::NonPositiveScale)
        );
        assert_eq!(
            AxisMapping::new(0.0, -2.0),
            Err(GeometryError::NonPositiveScale)
        );
        assert_eq!(
            AxisMapping::new(0.0, f64::MIN_POSITIVE / 2.0),
            Err(GeometryError::NonPositiveScale)
        );
    }

    #[test]
    fn keeps_precision_far_from_zero() {
        let samples = AxisMapping::new(3_000_000_000.0, 0.5).unwrap();
        assert_eq!(samples.pixel(3_000_000_001.0), 0.5);
        let zoomed = samples.zoomed_at(4.0, 200.0).unwrap();
        assert_eq!(zoomed.value(200.0), samples.value(200.0));
    }

    #[test]
    fn panning_moves_content_toward_higher_pixels() {
        let time = AxisMapping::new(10.0, 20.0).unwrap();
        let panned = time.panned(40.0).unwrap();
        assert_eq!(panned.pixel(10.0), 40.0);
    }

    #[test]
    fn tick_steps_are_one_two_or_five() {
        let axis = AxisMapping::new(0.0, 100.0).unwrap();
        assert_eq!(axis.tick_step(40.0), Some(0.5));
        assert_eq!(axis.tick_step(100.0), Some(1.0));
        assert_eq!(axis.tick_step(150.0), Some(2.0));
        assert_eq!(axis.tick_step(0.0), None);
        assert_eq!(axis.tick_step(f64::NAN), None);
        let marks: Vec<f64> = AxisMapping::new(-0.3, 100.0)
            .unwrap()
            .ticks(250.0, 100.0)
            .collect();
        assert_eq!(marks, [0.0, 1.0, 2.0]);
        assert_eq!(axis.ticks(f64::NAN, 10.0).count(), 0);
        assert_eq!(axis.ticks(1e12, 1e-9).count(), MAX_TICKS);
    }

    #[test]
    fn ticks_stay_finite_increasing_and_visible() {
        // Far from zero, one-unit marks are no longer distinguishable.
        let far = AxisMapping::new(1e20, 1.0).unwrap();
        let marks: Vec<f64> = far.ticks(1.0, 1.0).collect();
        assert!(marks.len() <= 1, "{}", marks.len());
        // An index that overflows yields nothing rather than infinities.
        let huge = AxisMapping::new(-1e308, 1e10).unwrap();
        assert!(huge.ticks(100.0, 1.0).all(f64::is_finite));
        let axis = AxisMapping::new(-3.7, 10.0).unwrap();
        let marks: Vec<f64> = axis.ticks(100.0, 20.0).collect();
        assert!(marks.windows(2).all(|pair| pair[0] < pair[1]));
        let (start, end) = axis.visible(100.0);
        assert!(marks.iter().all(|mark| (start..=end).contains(mark)));
    }

    #[test]
    fn zooming_to_the_same_scale_changes_nothing() {
        let tiny = AxisMapping::new(1.0, 1e-16).unwrap();
        assert_eq!(tiny.zoomed_at(1e-16, 1.0), Ok(tiny));
        let fine = AxisMapping::new(0.0, 1e-307).unwrap();
        assert_eq!(fine.zoomed_at(1e-307, 1000.0), Ok(fine));
        assert!(
            fine.zoomed_at(2e-307, 1.0).is_ok(),
            "no infinity minus infinity"
        );
        assert_eq!(fine.zoomed_at(1.0, f64::NAN), Err(GeometryError::NonFinite));
    }
}
