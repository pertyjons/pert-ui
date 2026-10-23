use std::collections::BTreeMap;

use crate::{ParameterError, ParameterId, ParameterSpec, ParameterValue};

/// An authoritative parameter revision, scoped to one parameter incarnation.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Deserialize, serde::Serialize),
    serde(transparent)
)]
pub struct Revision(u64);

impl Revision {
    /// The initial revision of a new parameter incarnation.
    pub const INITIAL: Self = Self(0);

    /// Returns the opaque monotonic revision number; do not use it as an identity.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }

    fn next(self) -> Result<Self, EditError> {
        self.0.checked_add(1).map(Self).ok_or(EditError::Exhausted)
    }
}

/// Ordered message number within an edit. Updates and commit must increase it.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct EditSequence(u64);

impl EditSequence {
    /// Constructs a sequence number. Zero is reserved as the begin baseline.
    #[must_use]
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// Returns the sequence number, scoped to its edit token.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

/// Opaque origin of an edit, such as a control or command source.
///
/// Whoever begins the edit assigns it. A host driving `ParameterState`
/// directly chooses its own numbers; the `pert-ui` runtime assigns each
/// component's mount serial, so outcomes can be routed back to the control
/// that began the edit. The value has no meaning beyond equality.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Deserialize, serde::Serialize),
    serde(transparent)
)]
pub struct EditSource(u64);

impl EditSource {
    /// Wraps a source number chosen by whoever begins the edit; zero is valid.
    #[must_use]
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// Returns the source number, for routing outcomes by source.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

/// Token issued by one parameter incarnation when an edit is admitted.
///
/// Contains the complete target, source and a non-reused serial. Clone for message
/// delivery outside time-critical callbacks. Hosts must not create multiple independent
/// states for the same parameter incarnation: counters are incarnation-local.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
#[must_use]
pub struct EditToken {
    target: ParameterId,
    serial: u64,
    source: EditSource,
}

impl EditToken {
    /// Returns the full target identity, including its runtime incarnation.
    pub fn target(&self) -> &ParameterId {
        &self.target
    }

    /// Returns the source supplied at begin.
    #[must_use]
    pub fn source(&self) -> EditSource {
        self.source
    }
}

/// Coherent local view of one parameter; telemetry is never its base value.
///
/// `Copy` when the canonical type is (every family except choices).
///
/// # Serialization
///
/// With the `serde` feature a snapshot is `{ edit_source, base, preview,
/// effective, revision, available }`, a closed set of fields. Reading one
/// rejects a value no member of the family can hold, such as a non-finite
/// continuous value or one too large for an `f32`, instead of storing
/// infinity. A snapshot carries no specification, so reading it alone cannot
/// check a domain; [`AnyParameterSnapshot`](crate::AnyParameterSnapshot)
/// reads a snapshot together with its specification and checks both.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct ParameterSnapshot<T> {
    /// Source currently owning the edit, even before its first preview update.
    pub edit_source: Option<EditSource>,
    /// Authoritative valid value in the specification's domain.
    pub base: T,
    /// Validated active preview, if any; never implicitly committed.
    pub preview: Option<T>,
    /// Effective overlay, or preview/base when no overlay is supplied. An overlay
    /// may lie outside the editable domain; see
    /// [`ParameterSpec::validate_effective`].
    pub effective: T,
    /// Authoritative revision; previews and effective overlays do not advance it.
    pub revision: Revision,
    /// False after removal/shutdown; retained values are last-known data.
    pub available: bool,
}

/// Result of a preview message; stale delivery is harmless and observable.
///
/// The set is closed by contract: a preview either replaced the previous one or
/// was not newer, so exhaustive matches on it are supported. Admission and
/// validation failures are [`EditError`]s, not outcomes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PreviewOutcome {
    /// The candidate replaced the previous preview.
    Applied,
    /// The sequence was not newer than the last accepted preview.
    Stale,
}

/// Submission or reserved terminal result for one admitted edit.
///
/// `Submitted` is nonterminal and only used by external UI bindings. The
/// synchronous core returns terminal results, retained until acknowledgement.
///
/// Repeated commit/cancel calls return the original result, regardless of later
/// payloads. They cannot commit twice or cancel a completed commit. The local host
/// owns any undo recording; observe the first transition, not every replay.
/// `Committed` means the synchronous local store accepted the value and advanced
/// its revision; it does not mean an external engine received, applied, persisted
/// or recorded it. It is `Clone` but not `Copy`, because
/// [`Self::Rejected`]'s [`EditError`] can carry a choice identity.
///
/// The enum is `#[non_exhaustive]`: a later release may add a terminal kind.
/// Match it with a wildcard arm and treat an unknown outcome as a terminal
/// result that did not commit, as [`EditOutcome::Cancelled`] is.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum EditOutcome<T> {
    /// The UI submitted a value to an external authority; acceptance is pending.
    ///
    /// This is returned by an asynchronous UI binding, never by the synchronous
    /// [`ParameterState`]. It ends the local gesture without changing its base.
    /// The binding retains the external result separately after guard drop.
    Submitted {
        /// Validated candidate awaiting external acceptance or rejection.
        value: T,
    },
    /// One authoritative commit occurred.
    Committed {
        /// Accepted canonical value, even if equal to the previous base.
        value: T,
        /// New authoritative revision.
        revision: Revision,
    },
    /// The edit ended without changing the base.
    Cancelled,
    /// The edit ended without committing because its contract was rejected.
    Rejected(EditError),
}

/// Admission, delivery or validation failure in the local edit protocol.
///
/// It is `Clone` but not `Copy`: [`Self::InvalidValue`] holds a
/// [`ParameterError`], which can carry a choice identity.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum EditError {
    /// The external authority refused the submitted candidate.
    #[error("the external authority rejected the edit")]
    RejectedByHost,
    /// The target has been removed or the local host shut down.
    #[error("parameter is unavailable")]
    Unavailable,
    /// Another edit already owns this parameter incarnation.
    #[error("parameter already has an active edit")]
    Busy,
    /// The expected base revision is no longer current.
    #[error("parameter revision is stale")]
    StaleRevision,
    /// The specification is read-only.
    #[error("parameter is read-only")]
    ReadOnly,
    /// All [`ParameterState::MAX_TERMINAL_RECEIPTS`] terminal receipt slots are
    /// occupied until acknowledged.
    #[error("terminal receipt capacity is exhausted")]
    Capacity,
    /// Token target, source or serial does not match a retained edit.
    #[error("unknown edit token or wrong parameter incarnation")]
    UnknownEdit,
    /// A preview was delivered after the edit ended.
    #[error("edit already finished")]
    Finished,
    /// A terminal commit did not advance beyond the accepted preview sequence.
    #[error("commit sequence is stale")]
    StaleSequence,
    /// An external authoritative write or metadata change invalidated the edit.
    #[error("edit conflicts with an authoritative change")]
    Conflict,
    /// A value failed domain validation.
    #[error(transparent)]
    InvalidValue(#[from] ParameterError),
    /// Delivering an accepted preview to the host failed, so the edit ended
    /// without committing. See [`ParameterState::reject_delivery`].
    #[error("host delivery of the edit failed")]
    DeliveryFailed,
    /// A monotonic counter cannot advance; no counter wraps.
    #[error("parameter counter exhausted")]
    Exhausted,
    /// A pending edit cannot be acknowledged before a terminal outcome exists.
    #[error("edit has no terminal outcome yet")]
    Pending,
}

struct Receipt<T> {
    source: EditSource,
    outcome: Option<EditOutcome<T>>,
}

struct Active<T> {
    serial: u64,
    sequence: EditSequence,
    preview: Option<T>,
}

/// Synchronous edit state machine for one parameter of any family, independent
/// of UI, OS and application processing.
///
/// The same protocol serves every family: validation and effective-overlay rules
/// come from the [`ParameterSpec<T>`]. One active edit is permitted. Begin
/// reserves one of [`Self::MAX_TERMINAL_RECEIPTS`] terminal receipt slots;
/// previews cannot consume them. Outcomes are retained until [Self::acknowledge].
/// No channels, locks, callbacks or implicit undo stack are involved. This is not
/// a callback-safe endpoint: construction/begin allocate and own heap data.
/// The host must keep exactly one state per complete parameter incarnation.
///
/// ```
/// # use pert_ui_core as pert_ui;
/// use pert_ui::*;
/// let mut state = ParameterState::new(
///     ParameterId::new(1, 2, "amount")?, ParameterSpec::new("Amount", 0.0, 10.0, 2.0)?,
/// );
/// let token = state.begin(Revision::INITIAL, EditSource::new(1))?;
/// state.update(&token, EditSequence::new(1), 4.0)?;
/// assert_eq!(state.snapshot().base, 2.0);
/// assert_eq!(state.snapshot().preview, Some(4.0));
/// state.commit(&token, EditSequence::new(2), 5.0)?;
/// assert_eq!(state.snapshot().base, 5.0);
/// state.acknowledge(&token)?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub struct ParameterState<T: ParameterValue> {
    id: ParameterId,
    spec: ParameterSpec<T>,
    base: T,
    revision: Revision,
    effective: Option<T>,
    available: bool,
    next_serial: u64,
    active: Option<Active<T>>,
    receipts: BTreeMap<u64, Receipt<T>>,
}

impl<T: ParameterValue> std::fmt::Debug for ParameterState<T> {
    /// Shows the identity, revision, availability, the serial of the active
    /// edit and how many terminal receipts are retained; values and the
    /// specification are available through [`ParameterState::snapshot`] and
    /// [`ParameterState::spec`].
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ParameterState")
            .field("id", &self.id)
            .field("revision", &self.revision)
            .field("available", &self.available)
            .field("active_serial", &self.active.as_ref().map(|a| a.serial))
            .field("receipts", &self.receipts.len())
            .finish_non_exhaustive()
    }
}

impl<T: ParameterValue> ParameterState<T> {
    /// How many terminal receipts one state retains at once: one per admitted
    /// edit, the active one included, until [`Self::acknowledge`] releases it.
    ///
    /// [`Self::begin`] returns [`EditError::Capacity`] while this many are
    /// retained. The bound keeps an unacknowledged consumer from growing the
    /// state without limit; previews never consume a receipt.
    pub const MAX_TERMINAL_RECEIPTS: usize = 64;

    /// Creates an available parameter at its specification's default and revision zero.
    #[must_use]
    pub fn new(id: ParameterId, spec: ParameterSpec<T>) -> Self {
        Self {
            id,
            base: spec.default_value(),
            spec,
            revision: Revision::INITIAL,
            effective: None,
            available: true,
            next_serial: 0,
            active: None,
            receipts: BTreeMap::new(),
        }
    }

    /// Returns the immutable complete target identity.
    pub fn id(&self) -> &ParameterId {
        &self.id
    }

    /// Returns the current validated specification.
    pub fn spec(&self) -> &ParameterSpec<T> {
        &self.spec
    }

    /// Returns base, preview and effective separately; allocates nothing (choice
    /// values share their identity text).
    #[must_use]
    pub fn snapshot(&self) -> ParameterSnapshot<T> {
        let preview = self
            .active
            .as_ref()
            .and_then(|active| active.preview.clone());
        ParameterSnapshot {
            edit_source: self
                .active
                .as_ref()
                .and_then(|active| self.receipts.get(&active.serial))
                .map(|receipt| receipt.source),
            base: self.base.clone(),
            effective: self
                .effective
                .clone()
                .or_else(|| preview.clone())
                .unwrap_or_else(|| self.base.clone()),
            preview,
            revision: self.revision,
            available: self.available,
        }
    }

    /// Admits one edit at an expected revision and reserves its terminal receipt.
    ///
    /// # Errors
    /// Each leaves the state unchanged: [`EditError::Unavailable`] after
    /// [`Self::remove`], [`EditError::ReadOnly`] for a read-only specification,
    /// [`EditError::StaleRevision`] when `expected` is not the current revision,
    /// [`EditError::Busy`] while another edit is active,
    /// [`EditError::Capacity`] when [`Self::MAX_TERMINAL_RECEIPTS`] receipts are
    /// retained, and [`EditError::Exhausted`] when the edit serial cannot
    /// advance.
    pub fn begin(
        &mut self,
        expected: Revision,
        source: EditSource,
    ) -> Result<EditToken, EditError> {
        if !self.available {
            return Err(EditError::Unavailable);
        }
        if !self.spec.is_editable() {
            return Err(EditError::ReadOnly);
        }
        if expected != self.revision {
            return Err(EditError::StaleRevision);
        }
        if self.active.is_some() {
            return Err(EditError::Busy);
        }
        if self.receipts.len() >= Self::MAX_TERMINAL_RECEIPTS {
            return Err(EditError::Capacity);
        }
        let serial = self
            .next_serial
            .checked_add(1)
            .ok_or(EditError::Exhausted)?;
        let token = EditToken {
            target: self.id.clone(),
            serial,
            source,
        };
        self.receipts.insert(
            serial,
            Receipt {
                source,
                outcome: None,
            },
        );
        self.active = Some(Active {
            serial,
            sequence: EditSequence::new(0),
            preview: None,
        });
        self.next_serial = serial;
        Ok(token)
    }

    fn receipt(&self, token: &EditToken) -> Result<&Receipt<T>, EditError> {
        if token.target != self.id {
            return Err(EditError::UnknownEdit);
        }
        self.receipts
            .get(&token.serial)
            .filter(|r| r.source == token.source)
            .ok_or(EditError::UnknownEdit)
    }

    /// Returns a retained terminal outcome, or `None` while the edit is pending.
    ///
    /// # Errors
    /// Returns [EditError::UnknownEdit] for a wrong or acknowledged token.
    pub fn outcome(&self, token: &EditToken) -> Result<Option<EditOutcome<T>>, EditError> {
        Ok(self.receipt(token)?.outcome.clone())
    }

    /// Returns the sequence of the last accepted preview of the active edit
    /// owning this token, or `None` when no preview was accepted or the edit ended.
    ///
    /// # Errors
    /// Returns `UnknownEdit` for a wrong or acknowledged token.
    pub fn preview_sequence(&self, token: &EditToken) -> Result<Option<EditSequence>, EditError> {
        self.receipt(token)?;
        Ok(self
            .active
            .as_ref()
            .filter(|active| active.serial == token.serial && active.preview.is_some())
            .map(|active| active.sequence))
    }

    /// Applies a newer validated preview without changing the authoritative base.
    ///
    /// # Errors
    /// Wrong/acknowledged tokens return `UnknownEdit`; ended edits return `Finished`.
    /// A newer invalid candidate returns `InvalidValue` and leaves the edit active.
    /// Old sequences return `Stale` without inspecting or applying their payload.
    pub fn update(
        &mut self,
        token: &EditToken,
        sequence: EditSequence,
        value: T,
    ) -> Result<PreviewOutcome, EditError> {
        if self.receipt(token)?.outcome.is_some() {
            return Err(EditError::Finished);
        }
        let active = self.active.as_mut().ok_or(EditError::UnknownEdit)?;
        if sequence <= active.sequence {
            return Ok(PreviewOutcome::Stale);
        }
        self.spec.validate(&value)?;
        active.sequence = sequence;
        active.preview = Some(value);
        Ok(PreviewOutcome::Applied)
    }

    fn finish(&mut self, outcome: EditOutcome<T>) {
        if let Some(active) = self.active.take()
            && let Some(receipt) = self.receipts.get_mut(&active.serial)
        {
            receipt.outcome = Some(outcome);
        }
    }

    /// Commits a full final value once, or returns an already retained outcome.
    ///
    /// Invalid value, stale final sequence or revision exhaustion ends the edit
    /// with a reserved `Rejected` receipt and releases its preview. Even an equal
    /// value commit advances revision once. Replays never perform another write.
    ///
    /// # Errors
    /// Returns `UnknownEdit` for wrong/acknowledged tokens. Protocol rejections
    /// after admission are returned as [EditOutcome::Rejected], not lost errors.
    pub fn commit(
        &mut self,
        token: &EditToken,
        sequence: EditSequence,
        value: T,
    ) -> Result<EditOutcome<T>, EditError> {
        if let Some(outcome) = &self.receipt(token)?.outcome {
            return Ok(outcome.clone());
        }
        let active = self.active.as_ref().ok_or(EditError::UnknownEdit)?;
        let validation = if sequence <= active.sequence {
            Err(EditError::StaleSequence)
        } else {
            self.spec
                .validate(&value)
                .map_err(EditError::from)
                .and_then(|()| self.revision.next())
        };
        let outcome = match validation {
            Ok(revision) => {
                self.base = value.clone();
                self.revision = revision;
                EditOutcome::Committed { value, revision }
            }
            Err(error) => EditOutcome::Rejected(error),
        };
        self.finish(outcome.clone());
        Ok(outcome)
    }

    /// Cancels an unfinished edit, or replays its existing terminal result.
    ///
    /// # Errors
    /// Returns `UnknownEdit` for wrong/acknowledged tokens. Never restores a saved
    /// baseline over an external write and never undoes an already committed edit.
    pub fn cancel(&mut self, token: &EditToken) -> Result<EditOutcome<T>, EditError> {
        if let Some(outcome) = &self.receipt(token)?.outcome {
            return Ok(outcome.clone());
        }
        self.finish(EditOutcome::Cancelled);
        Ok(EditOutcome::Cancelled)
    }

    /// Ends an unfinished edit because delivering its accepted preview to the host
    /// failed, or replays an existing terminal result.
    ///
    /// Preserves base and revision, clears the active preview while keeping any
    /// explicit effective overlay, and records
    /// `Rejected(`[`EditError::DeliveryFailed`]`)` in the reserved receipt. This is
    /// the terminal transition a delivery layer uses; it is not a setter for
    /// arbitrary outcomes.
    ///
    /// # Errors
    /// Returns `UnknownEdit` for wrong/acknowledged tokens.
    pub fn reject_delivery(&mut self, token: &EditToken) -> Result<EditOutcome<T>, EditError> {
        if let Some(outcome) = &self.receipt(token)?.outcome {
            return Ok(outcome.clone());
        }
        let outcome = EditOutcome::Rejected(EditError::DeliveryFailed);
        self.finish(outcome.clone());
        Ok(outcome)
    }

    /// Records external refusal of an admitted edit without changing its base.
    /// Replays return the original retained outcome, as for [`Self::cancel`].
    ///
    /// # Errors
    /// Returns `UnknownEdit` for wrong or acknowledged tokens.
    pub fn reject_authority(&mut self, token: &EditToken) -> Result<EditOutcome<T>, EditError> {
        if let Some(outcome) = &self.receipt(token)?.outcome {
            return Ok(outcome.clone());
        }
        let outcome = EditOutcome::Rejected(EditError::RejectedByHost);
        self.finish(outcome.clone());
        Ok(outcome)
    }

    /// Releases a terminal receipt after its consumer has handled the result.
    ///
    /// # Errors
    /// Returns `Pending` before completion, or `UnknownEdit` for unknown tokens.
    /// Subsequent messages for an acknowledged token are rejected, never replayed.
    pub fn acknowledge(&mut self, token: &EditToken) -> Result<(), EditError> {
        if self.receipt(token)?.outcome.is_none() {
            return Err(EditError::Pending);
        }
        self.receipts.remove(&token.serial);
        Ok(())
    }

    /// Performs an authoritative write and rejects any active edit as a conflict.
    ///
    /// # Errors
    /// Each leaves the state unchanged: [`EditError::Unavailable`] after
    /// [`Self::remove`], [`EditError::InvalidValue`] for a value the
    /// specification rejects and [`EditError::Exhausted`] when the revision
    /// cannot advance. Even an equal external write advances revision and
    /// invalidates the edit.
    pub fn write_base(&mut self, value: T) -> Result<(), EditError> {
        if !self.available {
            return Err(EditError::Unavailable);
        }
        self.spec.validate(&value)?;
        let revision = self.revision.next()?;
        self.finish(EditOutcome::Rejected(EditError::Conflict));
        self.base = value;
        self.revision = revision;
        Ok(())
    }

    /// Replaces metadata atomically, terminating an active edit.
    ///
    /// Validates the current base and any explicit effective overlay against the
    /// new specification and checks revision capacity first; any failure leaves
    /// metadata, values, revision and the active edit unchanged. On success the
    /// active edit ends as `Rejected(Conflict)`, even for a relabeling, and the
    /// revision advances. An overlay is never cleared implicitly: removing the
    /// selected option or the option an overlay names is rejected, so migrate by
    /// adding the new option, writing the base or overlay, then removing the old
    /// one.
    ///
    /// # Errors
    /// [`EditError::Unavailable`] after [`Self::remove`],
    /// [`EditError::InvalidValue`] when the new specification rejects the base
    /// or the explicit effective overlay, and [`EditError::Exhausted`] when the
    /// revision cannot advance.
    pub fn replace_spec(&mut self, spec: ParameterSpec<T>) -> Result<(), EditError> {
        if !self.available {
            return Err(EditError::Unavailable);
        }
        spec.validate(&self.base)?;
        if let Some(effective) = &self.effective {
            spec.validate_effective(effective)?;
        }
        let revision = self.revision.next()?;
        self.finish(EditOutcome::Rejected(EditError::Conflict));
        self.spec = spec;
        self.revision = revision;
        Ok(())
    }

    /// Supplies an effective overlay; `None` restores preview/base following.
    ///
    /// This local seam has no timestamp, sequence, buffering or callback-safety claim.
    /// It never changes the base, revision or active edit. Overlays follow
    /// [`ParameterSpec::validate_effective`]: finite continuous values and any
    /// stepped value are retained even outside the editable domain, for displays
    /// to indicate clipping themselves; choice overlays must name a current option.
    ///
    /// # Errors
    /// Returns `Unavailable` after removal and `InvalidValue` for an overlay the
    /// specification rejects.
    pub fn set_effective(&mut self, value: Option<T>) -> Result<(), EditError> {
        if !self.available {
            return Err(EditError::Unavailable);
        }
        if let Some(value) = &value {
            self.spec.validate_effective(value)?;
        }
        self.effective = value;
        Ok(())
    }

    /// Permanently removes this incarnation, retaining terminal receipts.
    ///
    /// Also serves synchronous host shutdown. Active edits end as unavailable;
    /// recreating the object requires a new identity incarnation and state.
    pub fn remove(&mut self) {
        self.finish(EditOutcome::Rejected(EditError::Unavailable));
        self.available = false;
    }
}

// The serde representation; the public type documents it.
#[cfg(feature = "serde")]
mod serde_impls {
    use serde::{Deserialize, Deserializer, de::Error as _};

    use super::{EditSource, ParameterSnapshot, Revision};
    use crate::ParameterValue;

    #[derive(Deserialize)]
    #[serde(deny_unknown_fields, rename = "ParameterSnapshot")]
    struct Repr<T> {
        edit_source: Option<EditSource>,
        base: T,
        preview: Option<T>,
        effective: T,
        revision: Revision,
        available: bool,
    }

    impl<'de, T> Deserialize<'de> for ParameterSnapshot<T>
    where
        T: ParameterValue + Deserialize<'de>,
    {
        fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
            let repr = Repr::<T>::deserialize(deserializer)?;
            let check = |field: &str, value: &T| {
                T::validate_representable(value)
                    .map_err(|error| D::Error::custom(format_args!("{field}: {error}")))
            };
            check("base", &repr.base)?;
            if let Some(preview) = &repr.preview {
                check("preview", preview)?;
            }
            check("effective", &repr.effective)?;
            Ok(Self {
                edit_source: repr.edit_source,
                base: repr.base,
                preview: repr.preview,
                effective: repr.effective,
                revision: repr.revision,
                available: repr.available,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exhausted_revision_rejects_without_partial_commit() -> Result<(), Box<dyn std::error::Error>>
    {
        let mut state = ParameterState::new(
            ParameterId::new(1, 1, "x")?,
            ParameterSpec::new("X", 0.0, 1.0, 0.0)?,
        );
        state.revision = Revision(u64::MAX);
        let token = state.begin(state.revision, EditSource::new(1))?;
        assert_eq!(
            state.commit(&token, EditSequence::new(1), 1.0)?,
            EditOutcome::Rejected(EditError::Exhausted)
        );
        assert_eq!(state.snapshot().base, 0.0);
        state.next_serial = u64::MAX;
        assert_eq!(
            state.begin(state.revision, EditSource::new(1)),
            Err(EditError::Exhausted)
        );
        Ok(())
    }
}
