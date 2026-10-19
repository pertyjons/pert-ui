use std::{fmt::Debug, num::NonZeroU64, sync::Arc};

use crate::{
    AnyParameterSnapshot, AnySpec, AnyValue, ChoiceId, ChoiceOption, NormalizedValue,
    ParameterError, ParameterMapping, ParameterSnapshot, ParameterSpec,
};

/// The four closed canonical parameter families.
///
/// Every parameter belongs to exactly one family for its whole incarnation. The
/// set is closed: another family requires a library change, not an application
/// implementation of [`ParameterValue`]. Exhaustive matches on it are supported.
///
/// `Display` writes the lowercase family name, such as `continuous`, for
/// error messages.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ParameterFamily {
    /// Finite `f32` values in an inclusive range with a linear or logarithmic
    /// position mapping.
    Continuous,
    /// Exact `i64` values on a lattice of positive steps anchored at the minimum.
    Stepped,
    /// `false` and `true`.
    Boolean,
    /// One [`ChoiceId`] among an ordered, nonempty option set.
    Choice,
}
impl std::fmt::Display for ParameterFamily {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Continuous => "continuous",
            Self::Stepped => "stepped",
            Self::Boolean => "boolean",
            Self::Choice => "choice",
        })
    }
}

/// Canonical value type of one parameter family: `f32`, `i64`, `bool` or [`ChoiceId`].
///
/// The trait is sealed; only these four implementations exist. Generic code over
/// `T: ParameterValue` uses [`ParameterSpec<T>`] for validation and position
/// mapping and the conversion methods below for the tagged erased forms. The
/// methods are the implementations behind the `From`/`TryFrom` conversions of
/// [`AnyValue`], [`AnySpec`] and [`AnyParameterSnapshot`]; prefer those.
/// Values are cheap to clone: `ChoiceId` shares its text.
pub trait ParameterValue:
    sealed::Sealed + Clone + PartialEq + Debug + Send + Sync + 'static
{
    /// The family this canonical type represents.
    const FAMILY: ParameterFamily;

    /// Wraps a value in the tagged erased form, preserving its family.
    fn into_any(self) -> AnyValue;

    /// Extracts a value of this family, returning the input unchanged on a
    /// family mismatch. This checks the kind only, not a specification's domain.
    ///
    /// # Errors
    /// Returns the original value when it belongs to another family.
    fn from_any(value: AnyValue) -> Result<Self, AnyValue>;

    /// Wraps a specification in the tagged erased form.
    fn spec_into_any(spec: ParameterSpec<Self>) -> AnySpec;

    /// Extracts a specification of this family, returning the input unchanged on
    /// a family mismatch.
    ///
    /// # Errors
    /// Returns the original specification when it belongs to another family.
    fn spec_from_any(spec: AnySpec) -> Result<ParameterSpec<Self>, AnySpec>;

    /// Tags a specification together with a snapshot of the same family.
    fn snapshot_into_any(
        spec: ParameterSpec<Self>,
        state: ParameterSnapshot<Self>,
    ) -> AnyParameterSnapshot;
}

pub(crate) mod sealed {
    use super::*;

    /// Family-private domain storage and operations; not nameable outside the crate.
    pub trait Sealed: Sized {
        /// Family-specific domain data held by a specification.
        type Domain: Clone + Debug + PartialEq + Send + Sync + 'static;
        fn validate(domain: &Self::Domain, value: &Self) -> Result<(), ParameterError>;
        fn validate_effective(domain: &Self::Domain, value: &Self) -> Result<(), ParameterError>;
        /// Checks what every value of the family satisfies regardless of a
        /// domain, such as finiteness for `f32`; used where no specification
        /// is at hand, as when reading a snapshot on its own.
        fn validate_representable(value: &Self) -> Result<(), ParameterError>;
        fn normalize(
            domain: &Self::Domain,
            value: &Self,
        ) -> Result<NormalizedValue, ParameterError>;
        /// Maps a position to a valid value; `default` is the specification's
        /// validated default, a total fallback that valid domains never need.
        fn denormalize(domain: &Self::Domain, default: &Self, position: NormalizedValue) -> Self;
    }

    /// Continuous domain: finite inclusive bounds with `minimum < maximum`.
    #[derive(Clone, Copy, Debug, PartialEq)]
    pub struct ContinuousDomain {
        pub(crate) minimum: f32,
        pub(crate) maximum: f32,
        pub(crate) mapping: ParameterMapping,
    }

    /// Stepped domain: `minimum <= maximum`, both on the step lattice.
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub struct SteppedDomain {
        pub(crate) minimum: i64,
        pub(crate) maximum: i64,
        pub(crate) step: NonZeroU64,
    }

    /// Boolean domain; it carries no data.
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub struct BooleanDomain;

    /// Choice domain: a nonempty ordered option list with distinct identities.
    #[derive(Clone, Debug, Eq, PartialEq)]
    pub struct ChoiceDomain {
        pub(crate) options: Arc<[ChoiceOption]>,
    }
}
pub(crate) use sealed::{BooleanDomain, ChoiceDomain, ContinuousDomain, SteppedDomain};

impl SteppedDomain {
    /// Rank of a lattice value: `(value - minimum) / step`, computed in `i128`.
    pub(crate) fn rank(&self, value: i64) -> u64 {
        let offset = i128::from(value) - i128::from(self.minimum);
        // The domain was validated: the offset is within `0..=u64::MAX`.
        u64::try_from(offset / i128::from(self.step.get())).unwrap_or(u64::MAX)
    }
    /// Number of step intervals between the bounds.
    pub(crate) fn intervals(&self) -> u64 {
        self.rank(self.maximum)
    }
    /// Lattice value at a rank no greater than [`Self::intervals`].
    pub(crate) fn value_at(&self, rank: u64) -> i64 {
        let value = i128::from(self.minimum) + i128::from(rank) * i128::from(self.step.get());
        // A rank within `intervals` stays within `minimum..=maximum`.
        i64::try_from(value).unwrap_or(self.maximum)
    }
}

/// Correctly rounded `k / n` as `f32` (ties to even) for `0 < k < n`.
///
/// Uses exact integer arithmetic: the result is normal because `k / n >= 2^-64`.
fn ratio_to_f32(k: u64, n: u64) -> f32 {
    let (k, n) = (u128::from(k), u128::from(n));
    // Smallest e >= 1 with k * 2^e >= n, so 2^-e <= k/n < 2^(1-e).
    let mut e = (n.leading_zeros() as i32 - k.leading_zeros() as i32).unsigned_abs();
    e = e.saturating_sub(1).max(1);
    while (k << e) < n {
        e += 1;
    }
    // Mantissa M = round(k * 2^(e+23) / n) lies in 2^23..=2^24. Since
    // k * 2^e < 2n <= 2^65, the shifted numerator stays below 2^88.
    let numerator = k << (e + 23);
    let mut mantissa = numerator / n;
    let remainder = numerator % n;
    if 2 * remainder > n || (2 * remainder == n && mantissa & 1 == 1) {
        mantissa += 1;
    }
    let mut exponent = 127 - e;
    if mantissa == 1 << 24 {
        mantissa = 1 << 23;
        exponent += 1;
    }
    // Normal f32: biased exponent in 63..=127 and the implicit leading bit removed.
    f32::from_bits((exponent << 23) | (mantissa as u32 & 0x7f_ffff))
}

/// Rank `round(position * n)` with ties toward the larger rank, interpreting the
/// position as its exact binary rational.
fn rank_from_position(position: NormalizedValue, n: u64) -> u64 {
    let p = position.get();
    if p <= 0.0 {
        return 0;
    }
    if p >= 1.0 {
        return n;
    }
    let bits = p.to_bits();
    let biased = (bits >> 23) & 0xff;
    let fraction = u128::from(bits & 0x7f_ffff);
    // p = mantissa / 2^shift exactly.
    let (mantissa, shift) = if biased == 0 {
        (fraction, 149_u32)
    } else {
        (fraction | 1 << 23, 150 - biased)
    };
    let product = mantissa * u128::from(n);
    // product < 2^88, so a larger shift leaves a value below one half.
    if shift > 100 {
        return 0;
    }
    let whole = product >> shift;
    let remainder = product - (whole << shift);
    let rank = if remainder << 1 >= 1 << shift {
        whole + 1
    } else {
        whole
    };
    u64::try_from(rank).unwrap_or(n).min(n)
}

fn rank_to_position(k: u64, n: u64) -> NormalizedValue {
    if n == 0 || k == 0 {
        return NormalizedValue::MIN;
    }
    if k >= n {
        return NormalizedValue::MAX;
    }
    NormalizedValue::try_from(ratio_to_f32(k, n)).unwrap_or(NormalizedValue::MAX)
}

impl sealed::Sealed for f32 {
    type Domain = ContinuousDomain;

    fn validate(domain: &Self::Domain, value: &Self) -> Result<(), ParameterError> {
        if !value.is_finite() {
            return Err(ParameterError::NonFinite);
        }
        if *value < domain.minimum || *value > domain.maximum {
            return Err(ParameterError::OutOfRange);
        }
        Ok(())
    }

    fn validate_effective(_: &Self::Domain, value: &Self) -> Result<(), ParameterError> {
        Self::validate_representable(value)
    }

    fn validate_representable(value: &Self) -> Result<(), ParameterError> {
        if value.is_finite() {
            Ok(())
        } else {
            Err(ParameterError::NonFinite)
        }
    }

    fn normalize(domain: &Self::Domain, value: &Self) -> Result<NormalizedValue, ParameterError> {
        Self::validate(domain, value)?;
        let value = *value;
        if value == domain.minimum {
            return Ok(NormalizedValue::MIN);
        }
        if value == domain.maximum {
            return Ok(NormalizedValue::MAX);
        }
        let min = f64::from(domain.minimum);
        let max = f64::from(domain.maximum);
        let value = f64::from(value);
        let fraction = match domain.mapping {
            ParameterMapping::Linear => (value - min) / (max - min),
            ParameterMapping::Logarithmic => {
                ((value - min) / min).ln_1p() / ((max - min) / min).ln_1p()
            }
            ParameterMapping::Power(exponent) => {
                ((value - min) / (max - min)).powf(1.0 / f64::from(exponent.get()))
            }
        };
        NormalizedValue::try_from(fraction as Self).map_err(|_| ParameterError::OutOfRange)
    }

    fn denormalize(domain: &Self::Domain, _: &Self, position: NormalizedValue) -> Self {
        if position == NormalizedValue::MIN {
            return domain.minimum;
        }
        if position == NormalizedValue::MAX {
            return domain.maximum;
        }
        let min = f64::from(domain.minimum);
        let max = f64::from(domain.maximum);
        let fraction = f64::from(position.get());
        let value = match domain.mapping {
            ParameterMapping::Linear => min + fraction * (max - min),
            ParameterMapping::Logarithmic => min * (fraction * ((max - min) / min).ln_1p()).exp(),
            ParameterMapping::Power(exponent) => {
                min + fraction.powf(f64::from(exponent.get())) * (max - min)
            }
        };
        value.clamp(min, max) as Self
    }
}

impl sealed::Sealed for i64 {
    type Domain = SteppedDomain;

    fn validate(domain: &Self::Domain, value: &Self) -> Result<(), ParameterError> {
        if *value < domain.minimum || *value > domain.maximum {
            return Err(ParameterError::OutOfRange);
        }
        let offset = i128::from(*value) - i128::from(domain.minimum);
        if offset % i128::from(domain.step.get()) != 0 {
            return Err(ParameterError::OffLattice);
        }
        Ok(())
    }

    fn validate_effective(_: &Self::Domain, _: &Self) -> Result<(), ParameterError> {
        Ok(())
    }

    fn validate_representable(_: &Self) -> Result<(), ParameterError> {
        Ok(())
    }

    fn normalize(domain: &Self::Domain, value: &Self) -> Result<NormalizedValue, ParameterError> {
        Self::validate(domain, value)?;
        Ok(rank_to_position(domain.rank(*value), domain.intervals()))
    }

    fn denormalize(domain: &Self::Domain, _: &Self, position: NormalizedValue) -> Self {
        domain.value_at(rank_from_position(position, domain.intervals()))
    }
}

impl sealed::Sealed for bool {
    type Domain = BooleanDomain;

    fn validate(_: &Self::Domain, _: &Self) -> Result<(), ParameterError> {
        Ok(())
    }

    fn validate_effective(_: &Self::Domain, _: &Self) -> Result<(), ParameterError> {
        Ok(())
    }

    fn validate_representable(_: &Self) -> Result<(), ParameterError> {
        Ok(())
    }

    fn normalize(_: &Self::Domain, value: &Self) -> Result<NormalizedValue, ParameterError> {
        Ok(if *value {
            NormalizedValue::MAX
        } else {
            NormalizedValue::MIN
        })
    }

    fn denormalize(_: &Self::Domain, _: &Self, position: NormalizedValue) -> Self {
        position.get() >= 0.5
    }
}

impl ChoiceDomain {
    pub(crate) fn index_of(&self, id: &ChoiceId) -> Option<usize> {
        self.options.iter().position(|option| option.id() == id)
    }
    fn intervals(&self) -> u64 {
        // Construction checked that the length fits u64.
        u64::try_from(self.options.len().saturating_sub(1)).unwrap_or(u64::MAX)
    }
}

impl sealed::Sealed for ChoiceId {
    type Domain = ChoiceDomain;

    fn validate(domain: &Self::Domain, value: &Self) -> Result<(), ParameterError> {
        domain
            .index_of(value)
            .map(|_| ())
            .ok_or(ParameterError::UnknownOption)
    }

    fn validate_effective(domain: &Self::Domain, value: &Self) -> Result<(), ParameterError> {
        Self::validate(domain, value)
    }

    fn validate_representable(_: &Self) -> Result<(), ParameterError> {
        Ok(())
    }

    fn normalize(domain: &Self::Domain, value: &Self) -> Result<NormalizedValue, ParameterError> {
        let index = domain
            .index_of(value)
            .ok_or(ParameterError::UnknownOption)?;
        let index = u64::try_from(index).map_err(|_| ParameterError::DomainTooLarge)?;
        Ok(rank_to_position(index, domain.intervals()))
    }

    fn denormalize(domain: &Self::Domain, default: &Self, position: NormalizedValue) -> Self {
        let rank = rank_from_position(position, domain.intervals());
        let index = usize::try_from(rank).unwrap_or(usize::MAX);
        // The rank never exceeds the last index of a nonempty option set.
        domain
            .options
            .get(index)
            .map_or_else(|| default.clone(), |option| option.id().clone())
    }
}

macro_rules! family_conversions {
    ($ty:ty, $family:ident) => {
        impl ParameterValue for $ty {
            const FAMILY: ParameterFamily = ParameterFamily::$family;

            fn into_any(self) -> AnyValue {
                AnyValue::$family(self)
            }

            fn from_any(value: AnyValue) -> Result<Self, AnyValue> {
                match value {
                    AnyValue::$family(value) => Ok(value),
                    other => Err(other),
                }
            }

            fn spec_into_any(spec: ParameterSpec<Self>) -> AnySpec {
                AnySpec::$family(spec)
            }

            fn spec_from_any(spec: AnySpec) -> Result<ParameterSpec<Self>, AnySpec> {
                match spec {
                    AnySpec::$family(spec) => Ok(spec),
                    other => Err(other),
                }
            }

            fn snapshot_into_any(
                spec: ParameterSpec<Self>,
                state: ParameterSnapshot<Self>,
            ) -> AnyParameterSnapshot {
                AnyParameterSnapshot::$family { spec, state }
            }
        }
    };
}
family_conversions!(f32, Continuous);
family_conversions!(i64, Stepped);
family_conversions!(bool, Boolean);
family_conversions!(ChoiceId, Choice);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn family_display_names_are_the_documented_lowercase_names() {
        // `BindingError::KindMismatch` prints these names; they are documented.
        for (family, name) in [
            (ParameterFamily::Continuous, "continuous"),
            (ParameterFamily::Stepped, "stepped"),
            (ParameterFamily::Boolean, "boolean"),
            (ParameterFamily::Choice, "choice"),
        ] {
            assert_eq!(family.to_string(), name);
        }
    }

    #[test]
    fn ratio_rounding_matches_a_wide_reference() {
        for (k, n) in [
            (1, 3),
            (2, 3),
            (1, 7),
            (5, 9),
            (1, u64::MAX),
            (u64::MAX - 1, u64::MAX),
        ] {
            let exact = ratio_to_f32(k, n);
            let reference = (k as f64 / n as f64) as f32;
            // f64 double rounding may differ by one ulp; never more.
            assert!((exact.to_bits() as i64 - reference.to_bits() as i64).abs() <= 1);
            assert!(exact > 0.0 && exact <= 1.0);
        }
        // 1/3 is not a tie; 3/4 is exact.
        assert_eq!(ratio_to_f32(3, 4), 0.75);
        assert_eq!(ratio_to_f32(1, 3), 1.0_f32 / 3.0);
    }

    #[test]
    fn position_rank_rounds_ties_toward_the_larger_rank() -> Result<(), crate::NormalizedValueError>
    {
        let half = NormalizedValue::try_from(0.5)?;
        assert_eq!(rank_from_position(half, 1), 1);
        assert_eq!(rank_from_position(half, 3), 2);
        assert_eq!(rank_from_position(NormalizedValue::try_from(0.25)?, 2), 1);
        assert_eq!(rank_from_position(NormalizedValue::MIN, 9), 0);
        assert_eq!(rank_from_position(NormalizedValue::MAX, 9), 9);
        assert_eq!(rank_from_position(half, u64::MAX), 1 << 63);
        let tiny = NormalizedValue::try_from(f32::from_bits(1))?;
        assert_eq!(rank_from_position(tiny, u64::MAX), 0);
        Ok(())
    }
}
