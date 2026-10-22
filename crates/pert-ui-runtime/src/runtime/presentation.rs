//! Host-supplied presentation pacing, independent of native display handles.
use super::*;

/// Validated presentation ceiling in hertz, from 1 through 1000 inclusive.
///
/// A host supplies the current window's display rate when known. This is a
/// scheduling ceiling, not a guarantee of frame delivery or a producer sample
/// rate. Unknown/adaptive rates may remain absent; the runtime then uses 60 Hz.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PresentationRate(f64);
impl PresentationRate {
    /// Validates finite hertz in the supported 1–1000 Hz interval.
    ///
    /// # Errors
    /// Returns [`RuntimeError::InvalidPresentationRate`] for a non-finite rate
    /// or one outside that interval.
    pub fn new(hertz: f64) -> Result<Self, RuntimeError> {
        if !hertz.is_finite() || !(1.0..=1000.0).contains(&hertz) {
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
impl Runtime {
    /// Supplies the display's presentation ceiling for the selected window.
    ///
    /// `None` records that the rate is unknown and uses the deterministic 60 Hz
    /// fallback. The native `App` supplies this automatically on native event
    /// turns. Custom hosts may supply an intentional cap instead. Animation and
    /// polled live sources share this ceiling; notified sources drain on wakes.
    /// A polled source shared across windows follows its fastest presented subscriber. One-shot event timers are unaffected.
    ///
    /// # Errors
    /// Returns only a runtime phase error (see [`Runtime`]).
    pub fn set_presentation_rate(
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
    /// The supplied ceiling for this window, absent when unknown. Unlike the
    /// scheduling fallback, this query never invents a measured display rate.
    pub fn presentation_rate(&self) -> Option<PresentationRate> {
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
