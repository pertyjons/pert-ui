use std::{fmt, sync::Arc};

use crate::ParameterError;

/// Stable identity of one option in a choice parameter's domain.
///
/// An identity is an owned, nonempty string without leading or trailing Unicode
/// whitespace (`str::trim` semantics); interior whitespace is preserved and the
/// constructor rejects instead of trimming. Equality is case-sensitive string
/// equality without Unicode normalization. Identities are scoped to one
/// parameter's domain and keep their meaning for that parameter's lifetime:
/// reordering or relabeling options keeps values valid, while reusing an
/// identity for a different meaning does not. A semantic replacement needs a new
/// identity. Labels are presentation and never identity.
///
/// Clones share one immutable string, so copying a value into snapshots, events
/// and outcomes does not copy the text. The type is `Send` and `Sync`.
///
/// ```
/// use pert_ui_core::{ChoiceId, ParameterError};
/// let id = ChoiceId::new("low pass")?;
/// assert_eq!(id.as_str(), "low pass");
/// assert_eq!(ChoiceId::new(" padded"), Err(ParameterError::MalformedChoiceId));
/// # Ok::<(), ParameterError>(())
/// ```
///
/// # Serialization
///
/// With the `serde` feature an identity is its text, validated by
/// [`ChoiceId::new`] when read.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ChoiceId(Arc<str>);

impl ChoiceId {
    /// Validates and wraps an owned identity string.
    ///
    /// # Errors
    /// Returns [`ParameterError::MalformedChoiceId`] for an empty string or one
    /// with leading or trailing whitespace.
    pub fn new(id: impl Into<String>) -> Result<Self, ParameterError> {
        let id = id.into();
        if id.is_empty() || id.trim() != id {
            return Err(ParameterError::MalformedChoiceId);
        }
        Ok(Self(id.into()))
    }

    /// Returns the identity text verbatim.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ChoiceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl TryFrom<&str> for ChoiceId {
    type Error = ParameterError;

    /// Validates like [`ChoiceId::new`].
    fn try_from(id: &str) -> Result<Self, Self::Error> {
        Self::new(id)
    }
}

impl TryFrom<String> for ChoiceId {
    type Error = ParameterError;

    /// Validates like [`ChoiceId::new`].
    fn try_from(id: String) -> Result<Self, Self::Error> {
        Self::new(id)
    }
}

/// One option of a choice parameter: a stable identity and its display label.
///
/// The label is owned presentation text; it may be empty or equal to another
/// option's label, and it is never used as identity. The option order in a
/// specification is the presentation order and the rank used by normalized
/// positions; it is not an identity either.
///
/// ```
/// use pert_ui_core::{ChoiceId, ChoiceOption};
/// let option = ChoiceOption::new(ChoiceId::new("lp")?, "Low pass");
/// assert_eq!(option.id().as_str(), "lp");
/// assert_eq!(option.label(), "Low pass");
/// # Ok::<(), pert_ui_core::ParameterError>(())
/// ```
///
/// # Serialization
///
/// With the `serde` feature an option is `{ id, label }`.
#[derive(Clone, Debug, Eq, PartialEq)]
#[must_use]
pub struct ChoiceOption {
    id: ChoiceId,
    label: String,
}

// The serde representation; the public type documents it.
#[cfg(feature = "serde")]
mod serde_impls {
    use serde::{Deserialize, Deserializer, Serialize, Serializer, de::Error as _};

    use super::{ChoiceId, ChoiceOption};

    impl Serialize for ChoiceId {
        fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
            serializer.serialize_str(&self.0)
        }
    }

    impl<'de> Deserialize<'de> for ChoiceId {
        fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
            Self::new(String::deserialize(deserializer)?).map_err(D::Error::custom)
        }
    }

    #[derive(Deserialize, Serialize)]
    #[serde(deny_unknown_fields)]
    struct OptionRepr<'a> {
        id: ChoiceId,
        label: std::borrow::Cow<'a, str>,
    }

    impl Serialize for ChoiceOption {
        fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
            OptionRepr {
                id: self.id.clone(),
                label: std::borrow::Cow::Borrowed(&self.label),
            }
            .serialize(serializer)
        }
    }

    impl<'de> Deserialize<'de> for ChoiceOption {
        fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
            let repr = OptionRepr::deserialize(deserializer)?;
            Ok(Self::new(repr.id, repr.label.into_owned()))
        }
    }
}

impl ChoiceOption {
    /// Creates an option from a validated identity and any display label.
    pub fn new(id: ChoiceId, label: impl Into<String>) -> Self {
        Self {
            id,
            label: label.into(),
        }
    }

    /// Returns the stable identity.
    #[must_use]
    pub fn id(&self) -> &ChoiceId {
        &self.id
    }

    /// Returns the display label verbatim.
    #[must_use]
    pub fn label(&self) -> &str {
        &self.label
    }
}
