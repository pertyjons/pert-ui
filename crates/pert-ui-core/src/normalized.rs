/// A finite, dimensionless control position in the inclusive range `0.0..=1.0`.
///
/// This is a position, not a value in application units such as distance or angle.
/// Conversion rejects invalid input instead of clamping it. Negative zero is
/// accepted and stored as positive zero. There is no implicit default position.
///
/// Values are owned, immutable, `Copy`, `Send` and `Sync`. Construction and reads
/// do not allocate or interact with UI state or a scheduler.
///
/// ```
/// # use pert_ui_core as pert_ui;
/// use pert_ui::{NormalizedValue, NormalizedValueError};
///
/// let midpoint = NormalizedValue::try_from(0.5)?;
/// assert!(midpoint > NormalizedValue::MIN);
/// assert!(midpoint < NormalizedValue::MAX);
/// # Ok::<(), NormalizedValueError>(())
/// ```
///
/// # Ordering and hashing
///
/// Because a position is always finite and never negative zero, its equality
/// and order are total: it implements `Eq`, `Ord` and `Hash`, so positions can
/// be sorted, deduplicated and used as map keys directly. The order is the
/// numeric order of [`Self::get`], and two positions are equal exactly when
/// their `f32` values are.
///
/// ```
/// # use pert_ui_core as pert_ui;
/// use pert_ui::{NormalizedValue, NormalizedValueError};
///
/// let mut positions = [0.75, 0.25, 1.0, 0.25]
///     .into_iter()
///     .map(NormalizedValue::try_from)
///     .collect::<Result<Vec<_>, _>>()?;
/// positions.sort();
/// positions.dedup();
/// let raw: Vec<f32> = positions.iter().map(|value| value.get()).collect();
/// assert_eq!(raw, [0.25, 0.75, 1.0]);
/// # Ok::<(), NormalizedValueError>(())
/// ```
///
/// # Serialization
///
/// With the `serde` feature a position is its `f32`, validated exactly as
/// `NormalizedValue::try_from` does when read.
#[derive(Clone, Copy, Debug, PartialEq)]
#[must_use]
pub struct NormalizedValue(f32);

// The invariant (finite, no negative zero) makes `f32` equality reflexive.
impl Eq for NormalizedValue {}

impl PartialOrd for NormalizedValue {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for NormalizedValue {
    /// Numeric order. `f32::total_cmp` agrees with `<` and `==` here, because
    /// a position is never NaN and never negative zero.
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.total_cmp(&other.0)
    }
}

impl std::hash::Hash for NormalizedValue {
    /// Hashes the bit pattern, which is unique per value because negative
    /// zero is stored as positive zero.
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.to_bits().hash(state);
    }
}

impl NormalizedValue {
    /// The minimum position, exactly `0.0`.
    pub const MIN: Self = Self(0.0);

    /// The maximum position, exactly `1.0`.
    pub const MAX: Self = Self(1.0);

    /// Returns the finite position in `0.0..=1.0`, without conversion to host units.
    #[must_use]
    pub const fn get(self) -> f32 {
        self.0
    }
}

impl TryFrom<f32> for NormalizedValue {
    type Error = NormalizedValueError;

    /// Validates a dimensionless control position without clamping.
    ///
    /// # Errors
    ///
    /// Returns [`NormalizedValueError::NonFinite`] for NaN or infinity, or
    /// [`NormalizedValueError::OutOfRange`] for finite input outside `0.0..=1.0`.
    fn try_from(value: f32) -> Result<Self, Self::Error> {
        if !value.is_finite() {
            return Err(NormalizedValueError::NonFinite);
        }
        if !(0.0..=1.0).contains(&value) {
            return Err(NormalizedValueError::OutOfRange);
        }
        Ok(Self(if value == 0.0 { 0.0 } else { value }))
    }
}

/// Why a floating-point input could not become a [`NormalizedValue`].
///
/// Validation checks finiteness before range. The error contains no host or
/// backend state and can be copied or sent between threads.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum NormalizedValueError {
    /// The input was NaN, positive infinity or negative infinity.
    #[error("normalized position must be finite")]
    NonFinite,
    /// The input was finite but below `0.0` or above `1.0`.
    #[error("normalized position must be within 0.0..=1.0")]
    OutOfRange,
}

#[cfg(test)]
mod tests {
    use super::{NormalizedValue, NormalizedValueError};

    #[test]
    fn accepts_endpoints_and_nearest_interior_values() {
        for raw in [0.0, f32::from_bits(1), 0.5, 1.0_f32.next_down(), 1.0] {
            assert_eq!(
                NormalizedValue::try_from(raw).map(NormalizedValue::get),
                Ok(raw)
            );
        }
        assert_eq!(NormalizedValue::try_from(0.0), Ok(NormalizedValue::MIN));
        assert_eq!(NormalizedValue::try_from(1.0), Ok(NormalizedValue::MAX));
    }

    #[test]
    fn canonicalizes_negative_zero() {
        let bits = NormalizedValue::try_from(-0.0).map(|value| value.get().to_bits());
        assert_eq!(bits, Ok(0.0_f32.to_bits()));
    }

    #[test]
    fn total_order_and_hash_agree_with_the_f32_values() {
        use std::{
            cmp::Ordering,
            collections::hash_map::DefaultHasher,
            hash::{Hash, Hasher},
        };
        let hash = |value: NormalizedValue| {
            let mut hasher = DefaultHasher::new();
            value.hash(&mut hasher);
            hasher.finish()
        };
        let raws = [
            -0.0,
            0.0,
            f32::from_bits(1),
            f32::MIN_POSITIVE,
            0.25,
            0.5,
            1.0_f32.next_down(),
            1.0,
        ];
        for a in raws {
            for b in raws {
                let (x, y) = (
                    NormalizedValue::try_from(a).expect("valid"),
                    NormalizedValue::try_from(b).expect("valid"),
                );
                assert_eq!(Some(x.cmp(&y)), a.partial_cmp(&b), "{a} vs {b}");
                assert_eq!(x.partial_cmp(&y), a.partial_cmp(&b), "{a} vs {b}");
                assert_eq!(x == y, a == b, "{a} vs {b}");
                if x.cmp(&y) == Ordering::Equal {
                    assert_eq!(hash(x), hash(y), "{a} vs {b}");
                }
            }
        }
    }

    #[test]
    fn rejects_non_finite_input() {
        for raw in [f32::NAN, -f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            assert_eq!(
                NormalizedValue::try_from(raw),
                Err(NormalizedValueError::NonFinite)
            );
        }
    }

    #[test]
    fn rejects_values_immediately_outside_range_without_clamping() {
        for raw in [0.0_f32.next_down(), 1.0_f32.next_up(), f32::MIN, f32::MAX] {
            assert_eq!(
                NormalizedValue::try_from(raw),
                Err(NormalizedValueError::OutOfRange)
            );
        }
    }
}

// The serde representation; the public type documents it.
#[cfg(feature = "serde")]
mod serde_impls {
    use serde::{Deserialize, Deserializer, Serialize, Serializer, de::Error as _};

    use super::NormalizedValue;

    impl Serialize for NormalizedValue {
        fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
            serializer.serialize_f32(self.0)
        }
    }

    impl<'de> Deserialize<'de> for NormalizedValue {
        fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
            Self::try_from(f32::deserialize(deserializer)?).map_err(D::Error::custom)
        }
    }
}
