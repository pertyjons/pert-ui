/// A host parameter's identity within a session, including its runtime incarnation.
///
/// The host supplies stable opaque session/object identifiers and a parameter key.
/// Zero is valid for either identifier. Keys are case-sensitive, preserved verbatim
/// and unrelated to display names or ordering. Construction starts incarnation zero.
/// Use [`Self::next_incarnation`] when deleting/recreating the same target, retaining
/// the returned identity in the host even if its view is unmounted.
///
/// The host must assign unique session/object identities and never reuse an old
/// complete ID while messages referencing it can remain. This value does not
/// allocate identities globally, register a target or reject messages itself.
/// A store must match the complete ID before applying a command. Runtime
/// incarnations are not a persisted project-format contract.
///
/// IDs own their key, are `Clone`, `Eq`, `Hash`, `Send` and `Sync`. Cloning or
/// advancing allocates a key copy; do this outside time-critical callbacks.
///
/// ```
/// use pert_ui_core::{ParameterId, ParameterIdError};
/// let threshold = ParameterId::new(1, 42, "threshold")?;
/// let replacement = threshold.next_incarnation()?;
/// assert_eq!(threshold.key(), replacement.key());
/// assert_ne!(threshold, replacement);
/// # Ok::<(), ParameterIdError>(())
/// ```
///
/// # Serialization
///
/// With the `serde` feature an identity is `{ session, object, key,
/// incarnation }`. The two 128-bit identifiers are decimal strings, because
/// most encodings, JSON among them, have no 128-bit number. Reading requires
/// the canonical text the writer produces, digits only without a sign or
/// leading zeros, so an identity round-trips byte for byte, and validates the
/// key as [`ParameterId::new`] does.
///
/// The format exists for inspection snapshots, which describe one running
/// session. `incarnation` is carried so a snapshot names the exact target it
/// was captured from; like the runtime incarnation itself, it is
/// inspection-only and not a persisted project-format contract, so a stored
/// identity must not be assumed to name the same incarnation in a later
/// session.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
#[must_use]
pub struct ParameterId {
    session: u128,
    object: u128,
    key: String,
    incarnation: u64,
}

impl ParameterId {
    /// Creates incarnation zero for a host-assigned session/object/key tuple.
    ///
    /// This allocates an owned key and performs no global uniqueness check. Repeated
    /// calls with the same tuple produce equal IDs; recreating a removed target
    /// requires [`Self::next_incarnation`], not another call to `new`.
    ///
    /// The key is compared verbatim, so like a [`ChoiceId`](crate::ChoiceId)
    /// it may not start or end with whitespace, which would make two keys that
    /// read the same distinct.
    ///
    /// # Errors
    /// Returns [`ParameterIdError::EmptyKey`] for an empty or whitespace-only key
    /// and [`ParameterIdError::PaddedKey`] for one with leading or trailing
    /// whitespace.
    pub fn new(
        session: u128,
        object: u128,
        key: impl Into<String>,
    ) -> Result<Self, ParameterIdError> {
        let key = key.into();
        if key.trim().is_empty() {
            return Err(ParameterIdError::EmptyKey);
        }
        if key.trim() != key {
            return Err(ParameterIdError::PaddedKey);
        }
        Ok(Self {
            session,
            object,
            key,
            incarnation: 0,
        })
    }

    /// Returns the host-assigned document/session identifier, not a display index.
    #[must_use]
    pub const fn session(&self) -> u128 {
        self.session
    }

    /// Returns the host-assigned object identifier, scoped to [`Self::session`].
    #[must_use]
    pub const fn object(&self) -> u128 {
        self.object
    }

    /// Returns the stable, case-sensitive parameter key within the host object.
    pub fn key(&self) -> &str {
        &self.key
    }

    /// Returns the runtime incarnation; equality and hashing include this value.
    #[must_use]
    pub const fn incarnation(&self) -> u64 {
        self.incarnation
    }

    /// Creates the next incarnation with the same logical session/object/key tuple.
    ///
    /// Leaves this ID unchanged. The host must retain the result as the current
    /// incarnation; calling this twice on the same old ID returns equal results.
    ///
    /// # Errors
    /// Returns [`ParameterIdError::IncarnationExhausted`] at `u64::MAX`; it never
    /// wraps to a previously used incarnation. Retire that logical target on failure.
    pub fn next_incarnation(&self) -> Result<Self, ParameterIdError> {
        let incarnation = self
            .incarnation
            .checked_add(1)
            .ok_or(ParameterIdError::IncarnationExhausted)?;
        Ok(Self {
            incarnation,
            ..self.clone()
        })
    }
}

/// Why a parameter identity could not be constructed or advanced.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum ParameterIdError {
    /// A parameter key contains no non-whitespace character.
    #[error("parameter key must not be empty")]
    EmptyKey,
    /// A parameter key starts or ends with whitespace.
    #[error("parameter key must not start or end with whitespace")]
    PaddedKey,
    /// The runtime incarnation cannot advance without wrapping.
    #[error("parameter incarnation exhausted; retire the logical target")]
    IncarnationExhausted,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn padded_keys_are_refused() {
        assert_eq!(
            ParameterId::new(1, 2, " cutoff"),
            Err(ParameterIdError::PaddedKey)
        );
        assert_eq!(
            ParameterId::new(1, 2, "cutoff\n"),
            Err(ParameterIdError::PaddedKey)
        );
        assert_eq!(
            ParameterId::new(1, 2, "  "),
            Err(ParameterIdError::EmptyKey)
        );
        assert!(ParameterId::new(1, 2, "filter cutoff").is_ok());
    }

    #[test]
    fn exhaustion_never_wraps_to_an_old_target() {
        let id = ParameterId {
            session: 1,
            object: 2,
            key: "cutoff".into(),
            incarnation: u64::MAX,
        };
        assert_eq!(
            id.next_incarnation(),
            Err(ParameterIdError::IncarnationExhausted)
        );
        assert_eq!(id.incarnation(), u64::MAX);
    }
}

// The serde representation; the public type documents it.
#[cfg(feature = "serde")]
mod serde_impls {
    use serde::{Deserialize, Deserializer, Serialize, Serializer, de::Error as _};

    use super::ParameterId;

    #[derive(Deserialize, Serialize)]
    #[serde(deny_unknown_fields)]
    struct Repr {
        session: String,
        object: String,
        key: String,
        incarnation: u64,
    }

    impl Serialize for ParameterId {
        fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
            Repr {
                session: self.session.to_string(),
                object: self.object.to_string(),
                key: self.key.clone(),
                incarnation: self.incarnation,
            }
            .serialize(serializer)
        }
    }

    impl<'de> Deserialize<'de> for ParameterId {
        fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
            let repr = Repr::deserialize(deserializer)?;
            // Only the canonical text the writer produces: `u128::from_str`
            // alone would also accept a sign and leading zeros.
            let parse = |text: &str| {
                text.parse::<u128>()
                    .ok()
                    .filter(|value| value.to_string() == text)
                    .ok_or_else(|| {
                        D::Error::custom(format_args!(
                            "expected a canonical decimal 128-bit identifier, found {text:?}"
                        ))
                    })
            };
            let mut id = Self::new(parse(&repr.session)?, parse(&repr.object)?, repr.key)
                .map_err(D::Error::custom)?;
            id.incarnation = repr.incarnation;
            Ok(id)
        }
    }
}
