//! Host-supplied presentation pacing, independent of native display handles.
use super::*;

/// Validated presentation ceiling in hertz, from 1 through 1000 inclusive.
///
/// A host supplies the current window's display rate when known. This is a
/// scheduling ceiling, not a guarantee of frame delivery or a producer sample
/// rate. Unknown/adaptive rates may remain absent; the runtime then uses 60 Hz.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PresentationRate(f64);
// A validated rate is finite, so equality is total.
impl Eq for PresentationRate {}
impl PartialOrd for PresentationRate {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for PresentationRate {
    /// Numeric order. `total_cmp` agrees with `<` and `==` here, because
    /// a rate is finite and at least `MIN`, so never NaN or zero.
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.total_cmp(&other.0)
    }
}
impl std::hash::Hash for PresentationRate {
    /// Hashes the bit pattern, which is unique per value because a rate is finite and at least `MIN`, so never NaN or zero.
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.to_bits().hash(state);
    }
}
impl PresentationRate {
    /// The lowest ceiling, 1 Hz.
    pub const MIN: Self = Self(1.0);
    /// The highest ceiling, 1000 Hz.
    pub const MAX: Self = Self(1000.0);
    /// Validates finite hertz in the supported 1–1000 Hz interval.
    ///
    /// # Errors
    /// Returns [`RuntimeError::InvalidPresentationRate`] for a non-finite rate
    /// or one outside [`Self::MIN`]`..=`[`Self::MAX`].
    pub fn new(hertz: f64) -> Result<Self, RuntimeError> {
        if !hertz.is_finite() || !(Self::MIN.0..=Self::MAX.0).contains(&hertz) {
            return Err(RuntimeError::InvalidPresentationRate);
        }
        Ok(Self(hertz))
    }
    /// The host-supplied ceiling in hertz.
    pub const fn hertz(self) -> f64 {
        self.0
    }
    pub(super) fn interval(self) -> Duration {
        Duration::from_secs_f64(1.0 / self.0)
    }
}
impl TryFrom<f64> for PresentationRate {
    type Error = RuntimeError;
    /// Validates like [`PresentationRate::new`].
    fn try_from(hertz: f64) -> Result<Self, Self::Error> {
        Self::new(hertz)
    }
}
impl From<PresentationRate> for f64 {
    /// The ceiling in hertz, as [`PresentationRate::hertz`] returns it.
    fn from(rate: PresentationRate) -> Self {
        rate.0
    }
}
impl Runtime {
    // Documented on `crate::host::HostIntegration::set_presentation_rate`.
    pub(crate) fn set_presentation_rate(
        &self,
        rate: Option<PresentationRate>,
    ) -> Result<(), RuntimeError> {
        self.shared.writable()?;
        if self.window.shared.presentation_rate.replace(rate) == rate {
            return Ok(());
        }
        self.window
            .shared
            .presentation_interval
            .set(rate.map_or(PRESENTATION_INTERVAL, PresentationRate::interval));
        let nodes: Vec<_> = self
            .window
            .shared
            .deadlines
            .borrow()
            .values()
            .filter_map(Weak::upgrade)
            .collect();
        for node in nodes {
            if node.animated.get() {
                node.stop_deadline();
                node.invalidate(Invalidation::Paint);
            }
        }
        self.shared.repace_sources();
        Ok(())
    }
    // Documented on `crate::host::HostIntegration::presentation_rate`.
    pub(crate) fn presentation_rate(&self) -> Option<PresentationRate> {
        self.window.shared.presentation_rate.get()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_the_documented_range_and_rejects_everything_else() {
        for hertz in [1.0, 59.94, 1000.0] {
            assert_eq!(
                PresentationRate::new(hertz).map(PresentationRate::hertz),
                Ok(hertz)
            );
        }
        for hertz in [
            0.0,
            0.999,
            -60.0,
            1000.001,
            f64::NAN,
            f64::INFINITY,
            f64::NEG_INFINITY,
        ] {
            assert_eq!(
                PresentationRate::new(hertz),
                Err(RuntimeError::InvalidPresentationRate),
                "{hertz}"
            );
        }
        let interval = PresentationRate::new(1000.0).map(PresentationRate::interval);
        assert_eq!(interval, Ok(Duration::from_millis(1)));
    }
}
