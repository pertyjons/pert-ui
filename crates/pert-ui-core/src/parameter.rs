use std::{num::NonZeroU64, sync::Arc};

use crate::{
    ChoiceId, ChoiceOption, NormalizedValue, ParameterFamily, ParameterValue,
    family::{BooleanDomain, ChoiceDomain, ContinuousDomain, SteppedDomain},
};

/// How a continuous domain maps to a control position, independent of units.
///
/// The set grows as general mappings are added, so matches need a wildcard arm.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[non_exhaustive]
pub enum ParameterMapping {
    /// Equal value differences occupy equal distances. This is the default.
    #[default]
    Linear,
    /// Equal positive value ratios occupy equal distances.
    ///
    /// Requires a strictly positive domain; zero and negative bounds are invalid.
    Logarithmic,
    /// The fraction of the range covered grows as the position raised to the
    /// exponent: a value is `minimum + (maximum - minimum) * position^exponent`.
    ///
    /// An exponent above one gives the lower end of the range more of the
    /// travel, below one the upper end; one is linear. Any finite domain is
    /// valid, including one that spans zero. Mathematically the mapping is
    /// strictly increasing and invertible; in `f32` it is nondecreasing and the
    /// bounds map exactly, but positions whose share of the range falls below
    /// the precision of the bound collapse onto it, so near the lower end of a
    /// high exponent several positions give the same value and round trips
    /// are approximate. For `-60..12` and exponent 10, every position up to
    /// about 0.125 gives `-60`.
    ///
    /// With the `serde` feature it is `{ "Power": exponent }`, and reading it
    /// validates the exponent as [`PowerExponent::new`] does.
    Power(PowerExponent),
}

/// The validated exponent of [`ParameterMapping::Power`]: finite and within
/// [`Self::MIN`] to [`Self::MAX`], inclusive.
///
/// The range limits how much of a control can collapse onto a bound in
/// `f32`; within it, a high exponent still gives a plateau at the lower end
/// whose width depends on the domain (see [`ParameterMapping::Power`]). Two
/// exponents are equal when their values are.
///
/// ```
/// # use pert_ui_core as pert_ui;
/// use pert_ui::{ParameterMapping, ParameterSpec, PowerExponent};
/// let fader = ParameterSpec::new("Gain", -60.0, 12.0, 0.0)?
///     .with_mapping(ParameterMapping::Power(PowerExponent::new(2.0)?))?;
/// let halfway = fader.normalize(&-42.0)?;
/// assert!((halfway.get() - 0.5).abs() < 1e-5);
/// assert!(PowerExponent::new(0.0).is_err());
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
///
/// # Serialization
///
/// With the `serde` feature an exponent is its `f32`, validated by
/// [`PowerExponent::new`] when read: a non-finite value or one outside
/// [`Self::MIN`] to [`Self::MAX`] is rejected.
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Deserialize, serde::Serialize),
    serde(try_from = "f32", into = "f32")
)]
pub struct PowerExponent(f32);
// A validated exponent is finite, so equality is total.
impl Eq for PowerExponent {}
impl PowerExponent {
    /// The smallest exponent, one tenth.
    pub const MIN: Self = Self(0.1);
    /// The largest exponent, ten.
    pub const MAX: Self = Self(10.0);
    /// Validates an exponent.
    ///
    /// # Errors
    /// [`ParameterError::InvalidExponent`] for a non-finite value or one outside
    /// [`Self::MIN`] to [`Self::MAX`].
    pub fn new(exponent: f32) -> Result<Self, ParameterError> {
        if exponent.is_finite() && (Self::MIN.0..=Self::MAX.0).contains(&exponent) {
            Ok(Self(exponent))
        } else {
            Err(ParameterError::InvalidExponent)
        }
    }
    /// The exponent.
    #[must_use]
    pub const fn get(self) -> f32 {
        self.0
    }
}
impl TryFrom<f32> for PowerExponent {
    type Error = ParameterError;
    /// Validates like [`PowerExponent::new`].
    fn try_from(exponent: f32) -> Result<Self, Self::Error> {
        Self::new(exponent)
    }
}
impl From<PowerExponent> for f32 {
    fn from(exponent: PowerExponent) -> Self {
        exponent.0
    }
}

/// Immutable parameter metadata and domain for canonical values of type `T`.
///
/// `T` selects one of the four families of [`ParameterValue`]: continuous `f32`
/// ([`Self::new`]), stepped `i64` ([`Self::stepped`]), `bool` ([`Self::boolean`])
/// and [`ChoiceId`] ([`Self::choices`]). Every constructor takes a display name
/// and a default and validates both; a family-specific domain follows. Names,
/// defaults, validation and position mapping are shared operations; bounds, step,
/// mapping and options accessors exist only on the family they apply to, so a
/// boolean cannot carry numeric bounds and a choice cannot carry a mapping.
/// Validation never silently clamps, rounds or substitutes an option.
///
/// This owns no current value, identity, revision, preview, telemetry or edit
/// session. Values have application-defined meaning; no unit or domain semantics
/// are assigned. Specifications own their data, are immutable, cheap to clone
/// (choice options are shared) and `Send`/`Sync`, with no UI affinity. Metadata
/// may be shared by distinct parameters. Replacing it requires the host to
/// validate the current value and terminate incompatible edits. Read-only
/// metadata guides presentation; it does not enforce host authorization.
///
/// ```
/// # use pert_ui_core as pert_ui;
/// use pert_ui::{ChoiceId, ChoiceOption, NormalizedValue, ParameterSpec};
/// let amount = ParameterSpec::new("Amount", -100.0, 100.0, 0.0)?;
/// assert_eq!(amount.denormalize(NormalizedValue::try_from(0.75)?), 50.0);
///
/// let voices = ParameterSpec::stepped("Voices", 1, 16, 8)?;
/// assert_eq!(voices.normalize(&16)?, NormalizedValue::MAX);
///
/// let bypass = ParameterSpec::boolean("Bypass", false)?;
/// assert!(!bypass.default_value());
///
/// let lp = ChoiceId::new("lp")?;
/// let filter = ParameterSpec::choices(
///     "Filter",
///     [ChoiceOption::new(lp.clone(), "Low pass"), ChoiceOption::new(ChoiceId::new("hp")?, "High pass")],
///     lp,
/// )?;
/// assert_eq!(filter.options().len(), 2);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
///
/// # Serialization
///
/// With the `serde` feature a specification is `{ name, default, read_only,
/// domain }`, where `domain` holds the family's bounds: `{ minimum, maximum,
/// mapping }` for `f32`, `{ minimum, maximum, step }` for `i64`, `{}` for
/// `bool` and `{ options }` for [`ChoiceId`]. Reading one rebuilds it through
/// the constructors above, so an invalid domain or default is rejected, never
/// clamped.
#[must_use]
pub struct ParameterSpec<T: ParameterValue> {
    name: String,
    default: T,
    domain: T::Domain,
    read_only: bool,
}

impl<T: ParameterValue> Clone for ParameterSpec<T> {
    fn clone(&self) -> Self {
        Self {
            name: self.name.clone(),
            default: self.default.clone(),
            domain: self.domain.clone(),
            read_only: self.read_only,
        }
    }
}

impl<T: ParameterValue> std::fmt::Debug for ParameterSpec<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ParameterSpec")
            .field("name", &self.name)
            .field("default", &self.default)
            .field("domain", &self.domain)
            .field("read_only", &self.read_only)
            .finish()
    }
}

impl<T: ParameterValue> PartialEq for ParameterSpec<T> {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
            && self.default == other.default
            && self.domain == other.domain
            && self.read_only == other.read_only
    }
}

fn checked_name(name: impl Into<String>) -> Result<String, ParameterError> {
    let name = name.into();
    if name.trim().is_empty() {
        return Err(ParameterError::EmptyName);
    }
    Ok(name)
}

impl<T: ParameterValue> ParameterSpec<T> {
    fn build(name: String, default: T, domain: T::Domain) -> Result<Self, ParameterError> {
        T::validate(&domain, &default)?;
        Ok(Self {
            name,
            default,
            domain,
            read_only: false,
        })
    }

    /// Returns the family of this specification's canonical values.
    #[must_use]
    pub fn family(&self) -> ParameterFamily {
        T::FAMILY
    }

    /// Returns the display name verbatim; names are not unique parameter identities.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the validated initial/reset suggestion; does not change host state.
    #[must_use]
    pub fn default_value(&self) -> T {
        self.default.clone()
    }

    /// Returns whether controls may offer editing according to this metadata.
    ///
    /// Defaults to `true`. Host availability and authorization are separate.
    #[must_use]
    pub fn is_editable(&self) -> bool {
        !self.read_only
    }

    /// Marks this owned specification as read-only presentation metadata.
    ///
    /// Mapping remains available. This does not revoke an existing host edit
    /// session or prevent a host from changing the authoritative value.
    pub fn read_only(mut self) -> Self {
        self.read_only = true;
        self
    }

    /// Checks an editable candidate without clamping, mutation or authorization.
    ///
    /// # Errors
    /// Continuous values return [`ParameterError::NonFinite`] or
    /// [`ParameterError::OutOfRange`]; stepped values return `OutOfRange` or
    /// [`ParameterError::OffLattice`]; choices return
    /// [`ParameterError::UnknownOption`]. Booleans are always valid.
    pub fn validate(&self, value: &T) -> Result<(), ParameterError> {
        T::validate(&self.domain, value)
    }

    /// Checks a read-only effective overlay, which may lie outside the editable domain.
    ///
    /// Continuous overlays must be finite; stepped overlays may be any `i64`,
    /// including off the lattice or outside the bounds; booleans are always valid;
    /// choice overlays must name a current option. Displays indicate clipping
    /// themselves and never normalize an out-of-domain overlay as if editable.
    ///
    /// # Errors
    /// Returns [`ParameterError::NonFinite`] or [`ParameterError::UnknownOption`].
    pub fn validate_effective(&self, value: &T) -> Result<(), ParameterError> {
        T::validate_effective(&self.domain, value)
    }

    /// Maps a valid canonical value to a control position.
    ///
    /// Endpoints map exactly to [`NormalizedValue::MIN`] and [`NormalizedValue::MAX`].
    /// Continuous interiors use the mapping with `f64` intermediates. Stepped
    /// values and choices map rank `k` of `N` intervals to the nearest `f32` of
    /// `k / N` (ties to even); a single-value domain maps to `MIN`. Booleans map
    /// to `MIN` and `MAX`. Interior round trips are approximate and may collide,
    /// and positions are presentation data: canonical values never pass through
    /// them to be stored or committed. Does not allocate.
    ///
    /// # Errors
    /// Returns the same errors as [`Self::validate`]. Never clamps external input.
    pub fn normalize(&self, value: &T) -> Result<NormalizedValue, ParameterError> {
        T::normalize(&self.domain, value)
    }

    /// Maps a position to a valid canonical value.
    ///
    /// Endpoints return the exact bounds, first and last options, `false` and
    /// `true`. Stepped values and choices round `position * N` to the nearest
    /// rank, ties toward the larger rank, interpreting the position as its exact
    /// binary rational with overflow-safe integer arithmetic. Booleans select
    /// `false` below one half and `true` at or above it. Continuous interiors are
    /// computed in `f64` and contained to the bounds; no bit-identical
    /// transcendental results across platforms are promised. Normalized choice
    /// positions are valid only for the current specification; actions carry a
    /// [`ChoiceId`], never a stale index. This does not allocate for numeric
    /// families, mutate state or submit an edit.
    pub fn denormalize(&self, position: NormalizedValue) -> T {
        T::denormalize(&self.domain, &self.default, position)
    }
}

impl ParameterSpec<f32> {
    /// Creates an editable finite continuous domain with linear mapping.
    ///
    /// Negative, zero and positive values are supported. Allocates an owned name;
    /// validation and mapping themselves do not allocate.
    ///
    /// # Errors
    /// Checks in order: [`ParameterError::EmptyName`] for a whitespace-only/empty
    /// name, [`ParameterError::NonFinite`] for any non-finite bound or default,
    /// [`ParameterError::InvalidRange`] unless `minimum < maximum`, then
    /// [`ParameterError::OutOfRange`] if the default is outside the inclusive range.
    pub fn new(
        name: impl Into<String>,
        minimum: f32,
        maximum: f32,
        default: f32,
    ) -> Result<Self, ParameterError> {
        let name = checked_name(name)?;
        if !minimum.is_finite() || !maximum.is_finite() || !default.is_finite() {
            return Err(ParameterError::NonFinite);
        }
        if minimum >= maximum {
            return Err(ParameterError::InvalidRange);
        }
        Self::build(
            name,
            default,
            ContinuousDomain {
                minimum,
                maximum,
                mapping: ParameterMapping::Linear,
            },
        )
    }

    /// Selects a mapping for this owned specification, preserving domain and default.
    ///
    /// This is an optional semantic choice; ordinary parameters use the default.
    ///
    /// # Errors
    /// Returns [`ParameterError::NonPositiveLogDomain`] when logarithmic mapping
    /// is requested for a domain whose minimum is zero or negative.
    pub fn with_mapping(mut self, mapping: ParameterMapping) -> Result<Self, ParameterError> {
        if mapping == ParameterMapping::Logarithmic && self.domain.minimum <= 0.0 {
            return Err(ParameterError::NonPositiveLogDomain);
        }
        self.domain.mapping = mapping;
        Ok(self)
    }

    /// Returns the inclusive lower bound in application-defined units.
    #[must_use]
    pub fn minimum(&self) -> f32 {
        self.domain.minimum
    }

    /// Returns the inclusive upper bound in application-defined units.
    #[must_use]
    pub fn maximum(&self) -> f32 {
        self.domain.maximum
    }

    /// Returns the value-to-position mapping, linear by default.
    #[must_use]
    pub fn mapping(&self) -> ParameterMapping {
        self.domain.mapping
    }
}

impl ParameterSpec<i64> {
    /// Creates an editable stepped domain with step 1.
    ///
    /// Integers are stored and edited exactly. Equal bounds define one value; the
    /// full `i64::MIN..=i64::MAX` range is valid. Use [`Self::with_step`] for a
    /// coarser lattice.
    ///
    /// # Errors
    /// Checks in order: [`ParameterError::EmptyName`],
    /// [`ParameterError::InvalidRange`] when `minimum > maximum`, then
    /// [`ParameterError::OutOfRange`] for a default outside the bounds.
    pub fn stepped(
        name: impl Into<String>,
        minimum: i64,
        maximum: i64,
        default: i64,
    ) -> Result<Self, ParameterError> {
        let name = checked_name(name)?;
        if minimum > maximum {
            return Err(ParameterError::InvalidRange);
        }
        Self::build(
            name,
            default,
            SteppedDomain {
                minimum,
                maximum,
                step: NonZeroU64::MIN,
            },
        )
    }

    /// Replaces the step, validating the bounds and default again.
    ///
    /// A candidate is valid exactly when it is in range and `(value - minimum)`
    /// is a multiple of the step. The maximum must lie on the same lattice; an
    /// unaligned maximum is rejected instead of introducing a shorter final step,
    /// so a step larger than the span is valid only for a single-value domain.
    /// For example, `-3..=5` with step 2 admits `-3, -1, 1, 3, 5`. Arithmetic is
    /// widened, so extreme bounds cannot overflow.
    ///
    /// # Errors
    /// Returns [`ParameterError::InvalidStep`] for an unaligned maximum and
    /// [`ParameterError::OffLattice`] for a default off the new lattice.
    pub fn with_step(mut self, step: NonZeroU64) -> Result<Self, ParameterError> {
        let span = i128::from(self.domain.maximum) - i128::from(self.domain.minimum);
        if span % i128::from(step.get()) != 0 {
            return Err(ParameterError::InvalidStep);
        }
        self.domain.step = step;
        self.validate(&self.default)?;
        Ok(self)
    }

    /// Returns the inclusive lower bound, which anchors the lattice.
    #[must_use]
    pub fn minimum(&self) -> i64 {
        self.domain.minimum
    }

    /// Returns the inclusive upper bound, which lies on the lattice.
    #[must_use]
    pub fn maximum(&self) -> i64 {
        self.domain.maximum
    }

    /// Returns the positive step, 1 unless [`Self::with_step`] replaced it.
    #[must_use]
    pub fn step(&self) -> NonZeroU64 {
        self.domain.step
    }

    /// Returns the number of steps between the bounds; zero for one value.
    #[must_use]
    pub fn intervals(&self) -> u64 {
        self.domain.intervals()
    }

    /// Advances a valid value by a whole number of steps, stopping at the bounds.
    ///
    /// This is the keyboard/stepper rule: the result stays on the lattice and is
    /// clamped only to the valid endpoints. Arithmetic is widened, so no step
    /// count can overflow.
    ///
    /// ```
    /// # use pert_ui_core as pert_ui;
    /// use std::num::NonZeroU64;
    /// use pert_ui::ParameterSpec;
    /// let spec = ParameterSpec::stepped("Octave", -3, 5, -1)?
    ///     .with_step(NonZeroU64::new(2).ok_or("zero")?)?;
    /// assert_eq!(spec.offset(-1, 2)?, 3);
    /// assert_eq!(spec.offset(-1, 100)?, 5);
    /// assert_eq!(spec.offset(-1, i64::MIN)?, -3);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// # Errors
    /// Returns the [`Self::validate`] error for an invalid starting value.
    pub fn offset(&self, value: i64, steps: i64) -> Result<i64, ParameterError> {
        self.validate(&value)?;
        let rank = i128::from(self.domain.rank(value)) + i128::from(steps);
        let rank = rank.clamp(0, i128::from(self.domain.intervals()));
        Ok(self
            .domain
            .value_at(u64::try_from(rank).unwrap_or_default()))
    }
}

impl ParameterSpec<bool> {
    /// Creates an editable boolean domain.
    ///
    /// # Errors
    /// Returns [`ParameterError::EmptyName`] for an empty/whitespace-only name.
    pub fn boolean(name: impl Into<String>, default: bool) -> Result<Self, ParameterError> {
        Self::build(checked_name(name)?, default, BooleanDomain)
    }
}

impl ParameterSpec<ChoiceId> {
    /// Creates an editable choice domain from ordered options and a default identity.
    ///
    /// A single option is valid. Option order is presentation order and the rank
    /// used by positions; identities stay stable through reorder and relabeling.
    ///
    /// # Errors
    /// Checks in order: [`ParameterError::EmptyName`],
    /// [`ParameterError::EmptyOptions`], [`ParameterError::DomainTooLarge`] when
    /// the option count does not fit `u64`, [`ParameterError::DuplicateOption`],
    /// then [`ParameterError::UnknownOption`] for a default not among the options.
    pub fn choices(
        name: impl Into<String>,
        options: impl IntoIterator<Item = ChoiceOption>,
        default: ChoiceId,
    ) -> Result<Self, ParameterError> {
        let name = checked_name(name)?;
        let options: Arc<[ChoiceOption]> = options.into_iter().collect();
        if options.is_empty() {
            return Err(ParameterError::EmptyOptions);
        }
        u64::try_from(options.len()).map_err(|_| ParameterError::DomainTooLarge)?;
        let mut ids: Vec<&ChoiceId> = options.iter().map(ChoiceOption::id).collect();
        ids.sort_unstable();
        if ids.windows(2).any(|pair| pair[0] == pair[1]) {
            return Err(ParameterError::DuplicateOption);
        }
        Self::build(name, default, ChoiceDomain { options })
    }

    /// Returns the options in presentation order.
    pub fn options(&self) -> &[ChoiceOption] {
        &self.domain.options
    }

    /// Returns the option with this identity, if present.
    #[must_use]
    pub fn option(&self, id: &ChoiceId) -> Option<&ChoiceOption> {
        self.domain
            .index_of(id)
            .and_then(|index| self.domain.options.get(index))
    }

    /// Returns the current zero-based presentation index of an identity.
    ///
    /// Indices change when options are reordered; store and send identities.
    #[must_use]
    pub fn index_of(&self, id: &ChoiceId) -> Option<usize> {
        self.domain.index_of(id)
    }
}

/// A parameter metadata or domain-validation failure, without host state.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum ParameterError {
    /// The display name contains no non-whitespace character.
    #[error("parameter display name must not be empty")]
    EmptyName,
    /// A continuous bound, default or candidate is NaN or either infinity.
    #[error("parameter values must be finite")]
    NonFinite,
    /// A continuous minimum is not below its maximum, or a stepped minimum exceeds
    /// its maximum.
    #[error("minimum exceeds maximum, or a continuous range has zero width")]
    InvalidRange,
    /// A default or candidate lies outside the inclusive domain.
    #[error("parameter value is outside its domain")]
    OutOfRange,
    /// Logarithmic mapping was requested with a non-positive lower bound.
    #[error("logarithmic parameter domain must be strictly positive")]
    NonPositiveLogDomain,
    /// A power mapping's exponent is not finite or lies outside
    /// [`PowerExponent::MIN`] to [`PowerExponent::MAX`].
    #[error("power mapping exponent must be finite and within 0.1..=10")]
    InvalidExponent,
    /// A stepped domain's maximum does not lie on the step lattice.
    #[error("parameter step does not divide the range")]
    InvalidStep,
    /// A stepped default or candidate is in range but not on the step lattice.
    #[error("parameter value is not on the step lattice")]
    OffLattice,
    /// A choice domain has no options.
    #[error("choice parameter needs at least one option")]
    EmptyOptions,
    /// Two options of a choice domain share an identity.
    #[error("choice options must have distinct identities")]
    DuplicateOption,
    /// A choice default or candidate names no current option.
    #[error("choice is not one of the current options")]
    UnknownOption,
    /// A choice identity is empty or has leading or trailing whitespace.
    #[error("choice identity must be nonempty without surrounding whitespace")]
    MalformedChoiceId,
    /// A choice domain has more options than a `u64` rank can count.
    #[error("choice domain is too large")]
    DomainTooLarge,
}

// The serde representation; the public type documents it.
#[cfg(feature = "serde")]
mod serde_impls {
    use std::num::NonZeroU64;

    use serde::{Deserialize, Deserializer, Serialize, Serializer, de::Error as _};

    use super::{ChoiceId, ChoiceOption, ParameterMapping, ParameterSpec};

    #[derive(Deserialize, Serialize)]
    #[serde(deny_unknown_fields)]
    struct Repr<T, D> {
        name: String,
        default: T,
        read_only: bool,
        domain: D,
    }

    #[derive(Deserialize, Serialize)]
    #[serde(deny_unknown_fields)]
    struct Continuous {
        minimum: f32,
        maximum: f32,
        mapping: ParameterMapping,
    }

    #[derive(Deserialize, Serialize)]
    #[serde(deny_unknown_fields)]
    struct Stepped {
        minimum: i64,
        maximum: i64,
        step: NonZeroU64,
    }

    #[derive(Deserialize, Serialize)]
    #[serde(deny_unknown_fields)]
    struct Boolean {}

    #[derive(Deserialize, Serialize)]
    #[serde(deny_unknown_fields)]
    struct Choice {
        options: Vec<ChoiceOption>,
    }

    fn finish<T: super::ParameterValue>(
        spec: ParameterSpec<T>,
        read_only: bool,
    ) -> ParameterSpec<T> {
        if read_only { spec.read_only() } else { spec }
    }

    impl Serialize for ParameterSpec<f32> {
        fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
            Repr {
                name: self.name.clone(),
                default: self.default,
                read_only: self.read_only,
                domain: Continuous {
                    minimum: self.domain.minimum,
                    maximum: self.domain.maximum,
                    mapping: self.domain.mapping,
                },
            }
            .serialize(serializer)
        }
    }

    impl<'de> Deserialize<'de> for ParameterSpec<f32> {
        fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
            let repr: Repr<f32, Continuous> = Repr::deserialize(deserializer)?;
            let spec = Self::new(
                repr.name,
                repr.domain.minimum,
                repr.domain.maximum,
                repr.default,
            )
            .and_then(|spec| spec.with_mapping(repr.domain.mapping))
            .map_err(D::Error::custom)?;
            Ok(finish(spec, repr.read_only))
        }
    }

    impl Serialize for ParameterSpec<i64> {
        fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
            Repr {
                name: self.name.clone(),
                default: self.default,
                read_only: self.read_only,
                domain: Stepped {
                    minimum: self.domain.minimum,
                    maximum: self.domain.maximum,
                    step: self.domain.step,
                },
            }
            .serialize(serializer)
        }
    }

    impl<'de> Deserialize<'de> for ParameterSpec<i64> {
        fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
            let repr: Repr<i64, Stepped> = Repr::deserialize(deserializer)?;
            let spec = Self::stepped(
                repr.name,
                repr.domain.minimum,
                repr.domain.maximum,
                repr.default,
            )
            .and_then(|spec| spec.with_step(repr.domain.step))
            .map_err(D::Error::custom)?;
            Ok(finish(spec, repr.read_only))
        }
    }

    impl Serialize for ParameterSpec<bool> {
        fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
            Repr {
                name: self.name.clone(),
                default: self.default,
                read_only: self.read_only,
                domain: Boolean {},
            }
            .serialize(serializer)
        }
    }

    impl<'de> Deserialize<'de> for ParameterSpec<bool> {
        fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
            let repr: Repr<bool, Boolean> = Repr::deserialize(deserializer)?;
            let spec = Self::boolean(repr.name, repr.default).map_err(D::Error::custom)?;
            Ok(finish(spec, repr.read_only))
        }
    }

    impl Serialize for ParameterSpec<ChoiceId> {
        fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
            Repr {
                name: self.name.clone(),
                default: self.default.clone(),
                read_only: self.read_only,
                domain: Choice {
                    options: self.domain.options.to_vec(),
                },
            }
            .serialize(serializer)
        }
    }

    impl<'de> Deserialize<'de> for ParameterSpec<ChoiceId> {
        fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
            let repr: Repr<ChoiceId, Choice> = Repr::deserialize(deserializer)?;
            let spec = Self::choices(repr.name, repr.domain.options, repr.default)
                .map_err(D::Error::custom)?;
            Ok(finish(spec, repr.read_only))
        }
    }
}
