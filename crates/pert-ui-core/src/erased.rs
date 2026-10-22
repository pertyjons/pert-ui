use crate::{
    ChoiceId, ParameterFamily, ParameterSnapshot, ParameterSpec, ParameterValue, Revision,
};

/// A specification of any family, tagged by its family.
///
/// Use it where one collection holds parameters of several families, such as a
/// descriptor-driven panel. The tag preserves the family; a conversion to a
/// typed specification fails on a mismatch instead of reinterpreting it.
///
/// The set is closed by contract: one variant per family, closed with
/// [`ParameterFamily`], so exhaustive matches on it are supported.
///
/// ```
/// # use pert_ui_core as pert_ui;
/// use pert_ui::{AnySpec, ParameterFamily, ParameterSpec};
/// let spec = AnySpec::from(ParameterSpec::boolean("Bypass", false)?);
/// assert_eq!(spec.family(), ParameterFamily::Boolean);
/// assert!(ParameterSpec::<i64>::try_from(spec.clone()).is_err());
/// assert!(ParameterSpec::<bool>::try_from(spec).is_ok());
/// # Ok::<(), pert_ui::ParameterError>(())
/// ```
#[derive(Clone, Debug, PartialEq)]
#[must_use]
pub enum AnySpec {
    /// A continuous `f32` specification.
    Continuous(ParameterSpec<f32>),
    /// A stepped `i64` specification.
    Stepped(ParameterSpec<i64>),
    /// A boolean specification.
    Boolean(ParameterSpec<bool>),
    /// A choice specification.
    Choice(ParameterSpec<ChoiceId>),
}

impl AnySpec {
    /// Returns the family tag.
    #[must_use]
    pub fn family(&self) -> ParameterFamily {
        match self {
            Self::Continuous(_) => ParameterFamily::Continuous,
            Self::Stepped(_) => ParameterFamily::Stepped,
            Self::Boolean(_) => ParameterFamily::Boolean,
            Self::Choice(_) => ParameterFamily::Choice,
        }
    }

    /// Returns the display name.
    #[must_use]
    pub fn name(&self) -> &str {
        match self {
            Self::Continuous(spec) => spec.name(),
            Self::Stepped(spec) => spec.name(),
            Self::Boolean(spec) => spec.name(),
            Self::Choice(spec) => spec.name(),
        }
    }

    /// Returns the default value, tagged with the same family.
    pub fn default_value(&self) -> AnyValue {
        match self {
            Self::Continuous(spec) => AnyValue::Continuous(spec.default_value()),
            Self::Stepped(spec) => AnyValue::Stepped(spec.default_value()),
            Self::Boolean(spec) => AnyValue::Boolean(spec.default_value()),
            Self::Choice(spec) => AnyValue::Choice(spec.default_value()),
        }
    }

    /// Returns whether the metadata allows editing.
    #[must_use]
    pub fn is_editable(&self) -> bool {
        match self {
            Self::Continuous(spec) => spec.is_editable(),
            Self::Stepped(spec) => spec.is_editable(),
            Self::Boolean(spec) => spec.is_editable(),
            Self::Choice(spec) => spec.is_editable(),
        }
    }
}

impl<T: ParameterValue> From<ParameterSpec<T>> for AnySpec {
    fn from(spec: ParameterSpec<T>) -> Self {
        T::spec_into_any(spec)
    }
}

impl<T: ParameterValue> TryFrom<AnySpec> for ParameterSpec<T> {
    type Error = AnySpec;

    /// Extracts a typed specification, returning the input on a family mismatch.
    fn try_from(spec: AnySpec) -> Result<Self, Self::Error> {
        T::spec_from_any(spec)
    }
}

/// A canonical value of any family, tagged by its family.
///
/// The tag preserves the kind but is not proof of validity for a specification:
/// erased writes check the family first and then the domain. Choices stay
/// identities; they are never erased to indices or numbers.
///
/// The set is closed by contract: one variant per family, closed with
/// [`ParameterFamily`], so exhaustive matches on it are supported.
///
/// ```
/// # use pert_ui_core as pert_ui;
/// use pert_ui::{AnyValue, ParameterFamily};
/// let value = AnyValue::from(12_i64);
/// assert_eq!(value.family(), ParameterFamily::Stepped);
/// assert_eq!(i64::try_from(value.clone()), Ok(12));
/// assert!(bool::try_from(value).is_err());
/// ```
#[derive(Clone, Debug, PartialEq)]
pub enum AnyValue {
    /// A continuous value.
    Continuous(f32),
    /// A stepped value.
    Stepped(i64),
    /// A boolean value.
    Boolean(bool),
    /// A choice identity.
    Choice(ChoiceId),
}

impl AnyValue {
    /// Returns the family tag.
    #[must_use]
    pub fn family(&self) -> ParameterFamily {
        match self {
            Self::Continuous(_) => ParameterFamily::Continuous,
            Self::Stepped(_) => ParameterFamily::Stepped,
            Self::Boolean(_) => ParameterFamily::Boolean,
            Self::Choice(_) => ParameterFamily::Choice,
        }
    }
}

macro_rules! value_conversions {
    ($ty:ty) => {
        impl From<$ty> for AnyValue {
            fn from(value: $ty) -> Self {
                value.into_any()
            }
        }
        impl TryFrom<AnyValue> for $ty {
            type Error = AnyValue;

            /// Extracts a value of this family, returning the input on a mismatch.
            fn try_from(value: AnyValue) -> Result<Self, Self::Error> {
                <$ty as ParameterValue>::from_any(value)
            }
        }
    };
}
value_conversions!(f32);
value_conversions!(i64);
value_conversions!(bool);
value_conversions!(ChoiceId);

/// An owned, family-tagged view of one parameter: its specification together with
/// a snapshot of the same family.
///
/// Inspection and heterogeneous panels use it to keep typed specifications,
/// base, preview and effective values together without erasing choices to
/// indices or numbers. It is plain data: capturing it involves no live handle.
///
/// The set is closed by contract: one variant per family, closed with
/// [`ParameterFamily`], so exhaustive matches on it are supported.
///
/// # Serialization
///
/// With the `serde` feature a snapshot is tagged by its family, for example
/// `{ "Continuous": { spec, state } }`, with the forms of [`ParameterSpec`]
/// and [`ParameterSnapshot`]. Reading one also checks the values against the
/// specification it carries: `base` and `preview` must be valid in its domain,
/// as [`ParameterSpec::validate`] decides, and `effective` must be an overlay
/// the specification accepts, as [`ParameterSpec::validate_effective`]
/// decides. Anything else is rejected, never clamped.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum AnyParameterSnapshot {
    /// A continuous parameter.
    Continuous {
        /// Current specification.
        spec: ParameterSpec<f32>,
        /// Current values, revision and availability.
        state: ParameterSnapshot<f32>,
    },
    /// A stepped parameter.
    Stepped {
        /// Current specification.
        spec: ParameterSpec<i64>,
        /// Current values, revision and availability.
        state: ParameterSnapshot<i64>,
    },
    /// A boolean parameter.
    Boolean {
        /// Current specification.
        spec: ParameterSpec<bool>,
        /// Current values, revision and availability.
        state: ParameterSnapshot<bool>,
    },
    /// A choice parameter.
    Choice {
        /// Current specification.
        spec: ParameterSpec<ChoiceId>,
        /// Current values, revision and availability.
        state: ParameterSnapshot<ChoiceId>,
    },
}

impl AnyParameterSnapshot {
    /// Tags a specification and a snapshot of the same family.
    ///
    /// This does not check the values against the specification. A snapshot
    /// taken from a live `ParameterState` always lies in its domain; for values
    /// from anywhere else, use [`Self::try_new`], which applies the checks
    /// deserialization does.
    pub fn new<T: ParameterValue>(spec: ParameterSpec<T>, state: ParameterSnapshot<T>) -> Self {
        T::snapshot_into_any(spec, state)
    }

    /// Tags a specification and a snapshot after checking that the base, any
    /// preview and the effective value lie in the specification's domain, as
    /// deserialization does.
    ///
    /// # Errors
    /// Returns the first [`ParameterError`](crate::ParameterError) among the base, the preview and the
    /// effective value, in that order.
    ///
    /// ```
    /// # use pert_ui_core as pert_ui;
    /// use pert_ui::{AnyParameterSnapshot, ParameterId, ParameterSpec, ParameterState};
    /// let spec = ParameterSpec::new("Gain", 0.0, 1.0, 0.5)?;
    /// let state = ParameterState::new(ParameterId::new(1, 1, "gain").unwrap(), spec.clone());
    /// let mut snapshot = state.snapshot();
    /// assert!(AnyParameterSnapshot::try_new(spec.clone(), snapshot.clone()).is_ok());
    /// snapshot.base = 2.0;
    /// assert!(AnyParameterSnapshot::try_new(spec, snapshot).is_err());
    /// # Ok::<(), pert_ui::ParameterError>(())
    /// ```
    pub fn try_new<T: ParameterValue>(
        spec: ParameterSpec<T>,
        state: ParameterSnapshot<T>,
    ) -> Result<Self, crate::ParameterError> {
        spec.validate(&state.base)?;
        if let Some(preview) = &state.preview {
            spec.validate(preview)?;
        }
        spec.validate_effective(&state.effective)?;
        Ok(Self::new(spec, state))
    }

    /// Returns the family tag.
    #[must_use]
    pub fn family(&self) -> ParameterFamily {
        match self {
            Self::Continuous { .. } => ParameterFamily::Continuous,
            Self::Stepped { .. } => ParameterFamily::Stepped,
            Self::Boolean { .. } => ParameterFamily::Boolean,
            Self::Choice { .. } => ParameterFamily::Choice,
        }
    }

    /// Returns the display name from the specification.
    #[must_use]
    pub fn name(&self) -> &str {
        match self {
            Self::Continuous { spec, .. } => spec.name(),
            Self::Stepped { spec, .. } => spec.name(),
            Self::Boolean { spec, .. } => spec.name(),
            Self::Choice { spec, .. } => spec.name(),
        }
    }

    /// Returns the authoritative revision.
    #[must_use]
    pub fn revision(&self) -> Revision {
        match self {
            Self::Continuous { state, .. } => state.revision,
            Self::Stepped { state, .. } => state.revision,
            Self::Boolean { state, .. } => state.revision,
            Self::Choice { state, .. } => state.revision,
        }
    }

    /// Returns whether the parameter was still available when captured.
    #[must_use]
    pub fn available(&self) -> bool {
        match self {
            Self::Continuous { state, .. } => state.available,
            Self::Stepped { state, .. } => state.available,
            Self::Boolean { state, .. } => state.available,
            Self::Choice { state, .. } => state.available,
        }
    }

    /// Returns the authoritative base value, tagged.
    pub fn base(&self) -> AnyValue {
        match self {
            Self::Continuous { state, .. } => AnyValue::Continuous(state.base),
            Self::Stepped { state, .. } => AnyValue::Stepped(state.base),
            Self::Boolean { state, .. } => AnyValue::Boolean(state.base),
            Self::Choice { state, .. } => AnyValue::Choice(state.base.clone()),
        }
    }
}

// The serde representation; the public type documents it.
#[cfg(feature = "serde")]
mod serde_impls {
    use serde::{Deserialize, Deserializer, de::Error as _};

    use super::AnyParameterSnapshot;
    use crate::{ChoiceId, ParameterError, ParameterSnapshot, ParameterSpec, ParameterValue};

    /// The same shape as the public enum's derived `Serialize`.
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields, rename = "AnyParameterSnapshot")]
    enum Repr {
        Continuous {
            spec: ParameterSpec<f32>,
            state: ParameterSnapshot<f32>,
        },
        Stepped {
            spec: ParameterSpec<i64>,
            state: ParameterSnapshot<i64>,
        },
        Boolean {
            spec: ParameterSpec<bool>,
            state: ParameterSnapshot<bool>,
        },
        Choice {
            spec: ParameterSpec<ChoiceId>,
            state: ParameterSnapshot<ChoiceId>,
        },
    }

    /// Checks the values against the specification and tags the pair.
    fn validated<T: ParameterValue>(
        spec: ParameterSpec<T>,
        state: ParameterSnapshot<T>,
    ) -> Result<AnyParameterSnapshot, String> {
        let field = |name: &str, result: Result<(), ParameterError>| {
            result.map_err(|error| format!("{name}: {error}"))
        };
        field("base", spec.validate(&state.base))?;
        if let Some(preview) = &state.preview {
            field("preview", spec.validate(preview))?;
        }
        field("effective", spec.validate_effective(&state.effective))?;
        Ok(AnyParameterSnapshot::new(spec, state))
    }

    impl<'de> Deserialize<'de> for AnyParameterSnapshot {
        fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
            match Repr::deserialize(deserializer)? {
                Repr::Continuous { spec, state } => validated(spec, state),
                Repr::Stepped { spec, state } => validated(spec, state),
                Repr::Boolean { spec, state } => validated(spec, state),
                Repr::Choice { spec, state } => validated(spec, state),
            }
            .map_err(D::Error::custom)
        }
    }
}

/// Translation between an application's own value type and a canonical family.
///
/// Implement this in the adapter layer, on a wrapper the adapter owns: Rust's
/// orphan rule prevents implementing it for a foreign application type from a
/// third crate, and pert-ui never depends on application code. It converts values
/// only, not descriptors, formatting or identity.
///
/// Both directions are fallible, because host `f64`, integer and newtype domains
/// may exceed what a family can represent. A successful conversion must preserve
/// meaning and round-trip to the same host value; reject unrepresentable values
/// rather than narrowing them. Specification checks still apply after
/// conversion. An adapter must not advertise a domain whose valid edits it
/// cannot translate back; a failure of [`Self::from_canonical`] while delivering
/// an edit is an adapter contract failure, not permission to discard the edit.
///
/// ```
/// # use pert_ui_core as pert_ui;
/// use pert_ui::HostValue;
///
/// /// An adapter-owned wrapper around an application's percentage.
/// #[derive(Debug, PartialEq)]
/// struct Percentage(f64);
///
/// #[derive(Debug, thiserror::Error)]
/// #[error("percentage is not exactly representable as f32")]
/// struct Lossy;
///
/// impl HostValue for Percentage {
///     type Canonical = f32;
///     type Error = Lossy;
///     fn to_canonical(&self) -> Result<f32, Lossy> {
///         let value = self.0 as f32;
///         if f64::from(value) == self.0 { Ok(value) } else { Err(Lossy) }
///     }
///     fn from_canonical(value: f32) -> Result<Self, Lossy> {
///         Ok(Self(f64::from(value)))
///     }
/// }
///
/// assert_eq!(Percentage(50.0).to_canonical()?, 50.0);
/// assert!(Percentage(0.1).to_canonical().is_err());
/// assert_eq!(Percentage::from_canonical(50.0)?, Percentage(50.0));
/// # Ok::<(), Lossy>(())
/// ```
pub trait HostValue: Sized {
    /// The canonical family this host type translates to.
    type Canonical: ParameterValue;
    /// The adapter's own conversion error.
    type Error: std::error::Error + 'static;

    /// Converts a host value to its canonical value without narrowing.
    ///
    /// # Errors
    /// Returns the adapter's error when the value is not exactly representable.
    fn to_canonical(&self) -> Result<Self::Canonical, Self::Error>;

    /// Converts a canonical value back to the host type.
    ///
    /// # Errors
    /// Returns the adapter's error when the host type cannot represent it.
    fn from_canonical(value: Self::Canonical) -> Result<Self, Self::Error>;
}
