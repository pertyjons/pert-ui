//! External commit acknowledgement over the same public control binding.
use super::*;
use crate::signal::ReceiptStatus;

/// An immutable, retryable commit proposal for an external authority.
///
/// Owned and `Send`; transport it off the UI thread. The full token includes the
/// parameter incarnation. The authority must deduplicate by token and connection
/// generation and accept at most once. The expected revision is the UI adapter's
/// revision; an adapter maps it to its own store's revision when necessary.
#[derive(Clone, Debug, PartialEq)]
pub struct RemoteCommit<T> {
    token: EditToken,
    generation: u64,
    expected: Revision,
    sequence: EditSequence,
    value: T,
}
impl<T> RemoteCommit<T> {
    /// Complete edit and target incarnation identity, suitable for deduplication.
    pub fn token(&self) -> &EditToken {
        &self.token
    }
    /// Connection generation. Reconnection invalidates all older proposals.
    pub fn generation(&self) -> u64 {
        self.generation
    }
    /// UI-observed base revision at submission.
    pub fn expected_revision(&self) -> Revision {
        self.expected
    }
    /// Full final candidate; a receiver does not need earlier preview messages.
    pub fn value(&self) -> &T {
        &self.value
    }
}

/// External decision for a [`RemoteCommit`].
#[derive(Clone, Debug, PartialEq)]
pub enum AuthorityResponse<T> {
    /// Accepted canonical value, which may correct the proposed value.
    Accepted(T),
    /// Refused without modifying the authoritative base.
    Rejected,
}

/// Observable external delivery/acceptance state of a parameter or receipt.
#[derive(Clone, Debug, PartialEq)]
pub enum AuthorityStatus<T> {
    /// Connected, with no submission yet.
    Ready,
    /// A full proposal is retained until a decision or explicit resynchronization.
    Pending(RemoteCommit<T>),
    /// A matched authority response or a local authoritative conflict ended this
    /// submission. A local `Rejected(Conflict)` retires the proposal in this
    /// adapter; it does not prove the external mutation was refused or never ran.
    /// Correlate the owned proposal with the transport and external store.
    Resolved {
        /// The proposal whose outcome is retained.
        request: RemoteCommit<T>,
        /// Final result; never `Submitted`.
        outcome: EditOutcome<T>,
    },
    /// Connection loss or teardown made the in-flight outcome unknown. Never
    /// retry this mutation after reconnect without host reconciliation.
    Unknown(RemoteCommit<T>),
    /// Disconnected without an in-flight proposal; editing is unavailable.
    Disconnected,
}

/// Retained authority response or local supersession for one submission.
///
/// [`Self::status`] reads the owned status, which stays readable after UI
/// shutdown. [`Self::status_binding`] mirrors it as a binding while the runtime
/// lives. Clones share this receipt, independently of edit guards or mounted
/// controls, and compare equal; two submissions are different receipts even
/// when they end in the same status.
#[derive(Clone)]
#[must_use = "retain the receipt to observe delayed or unknown acceptance"]
pub struct AuthorityReceipt<T>(ReceiptStatus<AuthorityStatus<T>>);

/// Shows the current status.
impl<T: ParameterValue> core::fmt::Debug for AuthorityReceipt<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("AuthorityReceipt")
            .field("status", &self.status())
            .finish()
    }
}
impl<T: ParameterValue> PartialEq for AuthorityReceipt<T> {
    fn eq(&self, other: &Self) -> bool {
        self.0.same(&other.0)
    }
}
impl<T: ParameterValue> Eq for AuthorityReceipt<T> {}
impl<T: ParameterValue> AuthorityReceipt<T> {
    /// Captures the current outcome without scheduling UI work, including after
    /// runtime shutdown.
    #[must_use]
    pub fn status(&self) -> AuthorityStatus<T> {
        self.0.get()
    }
    /// The same status as a binding, so a component that reads it during
    /// update is invalidated when the submission resolves or becomes unknown.
    /// Like every binding it reads `Err(RuntimeError::Closed)` after runtime
    /// shutdown; use [`Self::status`] then.
    pub fn status_binding(&self) -> ReadSignal<AuthorityStatus<T>> {
        self.0.binding()
    }
}

/// Whether an external response changed state.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ResponseDisposition {
    /// The response produced the first terminal result.
    Applied,
    /// This proposal has a terminal result in the latest receipt; no second commit.
    /// After a newer proposal replaces that receipt, the old reply is `Stale`.
    /// This also covers the first authority reply after a same-generation host
    /// write locally resolved the proposal as `Rejected(Conflict)`. The reply is
    /// not applied; reconcile its content with the actual external store rather
    /// than treating every such result as a duplicate acknowledgement.
    AlreadyResolved,
    /// Its connection, target incarnation or edit is no longer current.
    Stale,
}

/// Delivery of the most recent proposal to the registered transport callback.
/// Transport handoff never proves external acceptance; observe `AuthorityStatus`
/// separately. Failed requests remain retained for an explicit retry.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SubmissionDelivery {
    /// No proposal has been handed to a registered callback yet.
    Unobserved,
    /// The callback accepted the proposal for transport, not for authoritative commit.
    Delivered,
    /// Transport refused it; the error and full proposal remain available.
    Failed(SinkError),
}
type SubmissionSink<T> = Box<dyn FnMut(&RemoteCommit<T>) -> Result<(), SinkError>>;

struct Pending<T> {
    request: RemoteCommit<T>,
    receipt: AuthorityReceipt<T>,
    guard_dropped: bool,
}
pub(super) struct RemoteState<T: ParameterValue> {
    pub(super) connected: Cell<bool>,
    generation: Cell<u64>,
    pending: RefCell<Option<Pending<T>>>,
    status: Signal<AuthorityStatus<T>>,
    last: RefCell<Option<AuthorityReceipt<T>>>,
    submission_sink: RefCell<Option<SubmissionSink<T>>>,
    submission_delivery: Signal<SubmissionDelivery>,
}
impl<T: ParameterValue> RemoteState<T> {
    fn deliver_submission(&self, shared: &Rc<Shared>) -> Result<(), RuntimeError> {
        let request = self
            .pending
            .borrow()
            .as_ref()
            .map(|p| p.request.clone())
            .ok_or(DeliveryError::UnknownEdit)?;
        let Some(mut sink) = self.submission_sink.borrow_mut().take() else {
            return Err(DeliveryError::NoSubmissionSink.into());
        };
        let result = {
            let _guard = shared.enter_delivery();
            sink(&request)
        };
        // Host drop can retire the parameter from inside a callback. Do not
        // resurrect its transport owner after that teardown.
        if self.connected.get() {
            *self.submission_sink.borrow_mut() = Some(sink);
        }
        self.submission_delivery.publish_trusted(match &result {
            Ok(()) => SubmissionDelivery::Delivered,
            Err(error) => SubmissionDelivery::Failed(error.clone()),
        });
        result.map_err(|_| DeliveryError::SinkFailed.into())
    }
    pub(super) fn retire(&self) {
        self.disconnect();
        let sink = self.submission_sink.borrow_mut().take();
        drop(sink);
    }
    pub(super) fn submitted(&self, token: &EditToken) -> Option<T> {
        self.pending
            .borrow()
            .as_ref()
            .filter(|p| &p.request.token == token)
            .map(|p| p.request.value.clone())
    }
    pub(super) fn drop_guard(&self, token: &EditToken) -> bool {
        let mut pending = self.pending.borrow_mut();
        if let Some(p) = pending.as_mut().filter(|p| &p.request.token == token) {
            p.guard_dropped = true;
            true
        } else {
            false
        }
    }
    fn publish(&self, value: AuthorityStatus<T>) {
        self.status.publish_trusted(value);
    }
    pub(super) fn synchronize(&self, parameter: &Inner<T>) {
        let outcome = self.pending.borrow().as_ref().and_then(|p| {
            parameter
                .state
                .borrow()
                .outcome(&p.request.token)
                .ok()
                .flatten()
        });
        if let Some(outcome) = outcome
            && let Some(pending) = self.pending.borrow_mut().take()
        {
            let status = if matches!(pending.receipt.status(), AuthorityStatus::Unknown(_)) {
                pending.receipt.status()
            } else {
                AuthorityStatus::Resolved {
                    request: pending.request.clone(),
                    outcome,
                }
            };
            pending.receipt.0.set(status.clone());
            self.publish(status);
            if pending.guard_dropped {
                let result = parameter
                    .state
                    .borrow_mut()
                    .acknowledge(&pending.request.token);
                debug_assert!(
                    result.is_ok(),
                    "the external binding owns a terminal receipt"
                );
            }
        }
    }
    pub(super) fn disconnect(&self) {
        self.connected.set(false);
        let status = if let Some(pending) = self.pending.borrow().as_ref() {
            let status = AuthorityStatus::Unknown(pending.request.clone());
            pending.receipt.0.set(status.clone());
            status
        } else {
            AuthorityStatus::Disconnected
        };
        self.publish(status);
    }
    pub(super) fn submit(
        &self,
        parameter: &Inner<T>,
        token: &EditToken,
        sequence: EditSequence,
        value: T,
    ) -> Result<EditOutcome<T>, RuntimeError> {
        if !self.connected.get() {
            return Err(EditError::Unavailable.into());
        }
        if let Some(value) = self.submitted(token) {
            return Ok(EditOutcome::Submitted { value });
        }
        let invalid = parameter.state.borrow().spec().validate(&value).is_err();
        if invalid {
            return parameter.change(crate::EventKind::EditCommit, |state| {
                state.commit(token, sequence, value)
            });
        }
        // Reserve the sequence used by the eventual accepted response, and the
        // revision its commit takes, before admitting the proposal: an
        // authority must never accept a value the local commit then refuses.
        let revision = parameter.state.borrow().snapshot().revision;
        sequence
            .get()
            .checked_add(1)
            .and_then(|_| revision.get().checked_add(1))
            .ok_or(EditError::Exhausted)
            .inspect_err(|error| {
                parameter.shared.record(
                    crate::EventKind::EditCommit,
                    None,
                    Some(&parameter.id),
                    || crate::EventStatus::Failed(error.to_string()),
                );
            })?;
        parameter.change(crate::EventKind::EditCommit, |state| {
            state.update(token, sequence, value.clone())
        })?;
        let request = RemoteCommit {
            token: token.clone(),
            generation: self.generation.get(),
            expected: parameter.state.borrow().snapshot().revision,
            sequence,
            value: value.clone(),
        };
        let status = AuthorityStatus::Pending(request.clone());
        let receipt =
            AuthorityReceipt(ReceiptStatus::new(parameter.shared.clone(), status.clone()));
        *self.last.borrow_mut() = Some(receipt.clone());
        *self.pending.borrow_mut() = Some(Pending {
            request,
            receipt,
            guard_dropped: false,
        });
        self.publish(status);
        self.submission_delivery
            .publish_trusted(SubmissionDelivery::Unobserved);
        if self.submission_sink.borrow().is_some() {
            // The proposal and typed delivery error are retained. Admission
            // succeeds even if the transport is full; the host retries explicitly.
            let _ = self.deliver_submission(&parameter.shared);
        }
        Ok(EditOutcome::Submitted { value })
    }
}

/// UI-thread adapter owner of an externally authoritative parameter.
///
/// Standard controls use the returned ordinary [`Parameter`]. Previews remain
/// local; committing returns `Submitted` and keeps the base unchanged until a
/// matched response. Only one proposal can be outstanding, and its complete value
/// and reserved terminal storage survive guard drop and transport backpressure.
/// [`Self::request`] returns the same proposal until resolved: a full transport
/// queue must leave it here and retry delivery, never invent acknowledgement.
///
/// This protocol does not send domain commands or own an application worker.
/// The host transports proposals, deduplicates mutations and routes responses
/// back with a UI mailbox. A disconnected in-flight mutation is `Unknown`, not
/// cancelled or automatically retried. Resynchronization starts a new connection
/// generation; the application owns undo and reconciliation of unknown outcomes.
/// Built-in controls keep a submitted proposal visible and refuse further editing
/// until resolution or recovery. Local terminal notifications describe the local
/// edit protocol; retain an authority receipt per full token and generation to
/// reconcile external acceptance. Resynchronization/removal may locally reject an
/// edit while its old authority receipt intentionally remains `Unknown`. A stale
/// response never reopens that old local edit; reconcile the external state first.
///
/// The following headless example exercises UI submission, failed transport handoff,
/// explicit retry, corrected acceptance, duplicate responses and disconnection.
/// A real transport moves the owned proposal to its worker and routes responses
/// back onto the UI thread with a mailbox; the host starts no worker itself.
///
/// ```
/// use pert_ui_runtime::{
///     ApplicationError, AuthorityResponse, AuthorityStatus, EditOutcome, Element, Label,
///     ParameterId, ParameterSpec, ResponseDisposition, Runtime, SinkError, SinkErrorKind,
///     SubmissionDelivery,
/// };
/// use std::{cell::Cell, rc::Rc, time::Duration};
/// let mut ui = Runtime::new();
/// let (parameter, host) = ui.remote_parameter(
///     ParameterId::new(1, 1, "amount")?,
///     ParameterSpec::new("Amount", 0.0, 1.0, 0.25)?,
/// )?;
/// let transport_ready = Rc::new(Cell::new(false));
/// let ready = transport_ready.clone();
/// host.on_submit(move |_| {
///     if ready.get() {
///         Ok(())
///     } else {
///         Err(SinkError::new(SinkErrorKind::Full, "transport is full"))
///     }
/// })?;
/// let owner = ui.mount_tree(Element::new("owner", Label::new("Amount")))?;
/// ui.flush(Duration::ZERO)?;
/// let edit = ui.event(&owner, |cx| {
///     parameter.begin_edit(cx, parameter.snapshot().get()?.revision)
/// })??;
/// assert_eq!(edit.commit(0.6)?, EditOutcome::Submitted { value: 0.6 });
/// let request = host
///     .request()
///     .ok_or_else(|| ApplicationError::from("missing proposal"))?;
/// let receipt = host
///     .receipt()
///     .ok_or_else(|| ApplicationError::from("missing receipt"))?;
/// assert_eq!(parameter.snapshot().get()?.base, 0.25);
/// assert!(matches!(
///     host.submission_delivery().get()?,
///     SubmissionDelivery::Failed(_)
/// ));
/// transport_ready.set(true);
/// host.retry_submission(&request)?;
/// assert_eq!(
///     host.submission_delivery().get()?,
///     SubmissionDelivery::Delivered
/// );
/// assert!(matches!(receipt.status(), AuthorityStatus::Pending(_))); // handoff is not acceptance
/// assert_eq!(
///     host.respond(&request, AuthorityResponse::Accepted(0.5))?,
///     ResponseDisposition::Applied
/// );
/// assert_eq!(parameter.snapshot().get()?.base, 0.5);
/// assert_eq!(
///     host.respond(&request, AuthorityResponse::Rejected)?,
///     ResponseDisposition::AlreadyResolved
/// );
/// let edit = ui.event(&owner, |cx| {
///     parameter.begin_edit(cx, parameter.snapshot().get()?.revision)
/// })??;
/// edit.commit(0.8)?;
/// let old = host
///     .request()
///     .ok_or_else(|| ApplicationError::from("missing proposal"))?;
/// let unknown = host
///     .receipt()
///     .ok_or_else(|| ApplicationError::from("missing receipt"))?;
/// host.disconnect()?;
/// assert!(matches!(unknown.status(), AuthorityStatus::Unknown(_)));
/// // Reconcile the real external store before installing its new baseline.
/// host.resynchronize(0.4)?;
/// assert_eq!(parameter.snapshot().get()?.base, 0.4);
/// assert!(matches!(unknown.status(), AuthorityStatus::Unknown(_)));
/// assert_eq!(
///     host.respond(&old, AuthorityResponse::Accepted(0.8))?,
///     ResponseDisposition::Stale
/// );
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[must_use = "dropping the authority owner makes the binding unavailable"]
pub struct RemoteHost<T: ParameterValue> {
    host: ParameterHost<T>,
    state: Rc<RemoteState<T>>,
}

/// Shows the identity, connection state and generation.
impl<T: ParameterValue> core::fmt::Debug for RemoteHost<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("RemoteHost")
            .field("id", self.host.id())
            .field("connected", &self.state.connected.get())
            .field("generation", &self.state.generation.get())
            .finish_non_exhaustive()
    }
}
impl<T: ParameterValue> RemoteHost<T> {
    /// Registers the UI-thread transport callback before editing begins.
    ///
    /// It runs once after a proposal and its receipt are installed. Returning
    /// `Ok` means transport handoff only. A typed failure is retained in
    /// [`Self::submission_delivery`], leaves the proposal pending, and never
    /// retries automatically. Notify the UI when transport can accept data and
    /// call [`Self::retry_submission`] with the retained proposal. The callback
    /// must return promptly; state writes are rejected during delivery. Route
    /// external responses through an ordinary UI mailbox. Avoid capturing this
    /// host strongly in its own callback; use a transport handle to avoid an
    /// ownership cycle. Runtime shutdown releases callbacks. This is not a
    /// callback-safe endpoint.
    ///
    /// # Errors
    /// Returns phase/closed/unavailable errors, `AlreadyRegistered` for a second
    /// callback, or `ActiveEdit` while a gesture or proposal is outstanding.
    pub fn on_submit(
        &self,
        sink: impl FnMut(&RemoteCommit<T>) -> Result<(), SinkError> + 'static,
    ) -> Result<(), RuntimeError> {
        self.host.inner.shared.writable()?;
        let snapshot = self.host.inner.current_snapshot();
        if !snapshot.available {
            return Err(EditError::Unavailable.into());
        }
        if snapshot.edit_source.is_some() {
            return Err(DeliveryError::ActiveEdit.into());
        }
        let mut current = self.state.submission_sink.borrow_mut();
        if current.is_some() {
            return Err(DeliveryError::AlreadyRegistered.into());
        }
        *current = Some(Box::new(sink));
        Ok(())
    }
    /// Removes the transport callback after all local/pending edits have ended.
    /// Use after resynchronization to install a new connection's sender. The
    /// last delivery report and independent authority receipts remain unchanged.
    ///
    /// # Errors
    /// Returns phase/closed errors or `ActiveEdit` while a gesture/proposal remains.
    pub fn clear_submission_sink(&self) -> Result<(), RuntimeError> {
        self.host.inner.shared.writable()?;
        if self
            .host
            .inner
            .state
            .borrow()
            .snapshot()
            .edit_source
            .is_some()
        {
            return Err(DeliveryError::ActiveEdit.into());
        }
        let sink = self.state.submission_sink.borrow_mut().take();
        drop(sink);
        Ok(())
    }
    /// Reactive transport status, distinct from authoritative acceptance. The
    /// full failed proposal stays in [`Self::request`]. Reads require a live runtime.
    pub fn submission_delivery(&self) -> ReadSignal<SubmissionDelivery> {
        self.state.submission_delivery.read_only()
    }
    /// Explicitly retries a still-pending proposal after transport backpressure.
    ///
    /// # Errors
    /// Returns phase/closed/unavailable errors, `UnknownEdit` for a superseded
    /// proposal, `AlreadyDelivered` after successful handoff, `NoSubmissionSink`
    /// without a callback, or `SinkFailed` with its typed error retained.
    pub fn retry_submission(&self, request: &RemoteCommit<T>) -> Result<(), RuntimeError> {
        self.host.inner.shared.writable()?;
        if !self.state.connected.get() {
            return Err(EditError::Unavailable.into());
        }
        if !self
            .state
            .pending
            .borrow()
            .as_ref()
            .is_some_and(|p| p.request == *request)
        {
            return Err(DeliveryError::UnknownEdit.into());
        }
        if self.state.submission_delivery.get()? == SubmissionDelivery::Delivered {
            return Err(DeliveryError::AlreadyDelivered.into());
        }
        self.state.deliver_submission(&self.host.inner.shared)
    }
    /// Clones the retained proposal, or returns none while idle/disconnected.
    pub fn request(&self) -> Option<RemoteCommit<T>> {
        if !self.state.connected.get() {
            return None;
        }
        self.state
            .pending
            .borrow()
            .as_ref()
            .map(|p| p.request.clone())
    }
    /// The most recent submission's independent, UI-local receipt.
    /// Retain it after each `Submitted` result or submitting input dispatch,
    /// before another submission or supersession can replace the latest receipt.
    /// `Pending`, `Unknown` and `Resolved` carry the full owned proposal used for
    /// correlation. Local `Resolved` conflicts are not external acknowledgements.
    pub fn receipt(&self) -> Option<AuthorityReceipt<T>> {
        self.state.last.borrow().clone()
    }
    /// The synchronous owner for authoritative external writes, metadata and
    /// terminal notifications. A write conflicts with any pending proposal,
    /// locally resolving its receipt as `Rejected(Conflict)`. This retires local
    /// work without proving whether that external mutation ran; the host must
    /// reconcile the transport and real authoritative store.
    pub fn local_host(&self) -> &ParameterHost<T> {
        &self.host
    }
    /// Applies a matched decision once. Corrected accepted values are validated
    /// before mutation. Stale/duplicate decisions never disturb a newer edit.
    /// A same-generation host write can locally supersede a submission before
    /// its first authority reply. While the superseded proposal remains the
    /// latest receipt, that reply returns `AlreadyResolved` and is not applied,
    /// even when it reports acceptance; the host must reconcile its content with
    /// the external store. Resynchronization changes the generation,
    /// so replies to its superseded submissions instead return `Stale`.
    ///
    /// # Errors
    /// Returns phase/closed errors or value/revision validation errors. An invalid
    /// corrected value leaves the proposal pending for explicit repair.
    pub fn respond(
        &self,
        request: &RemoteCommit<T>,
        response: AuthorityResponse<T>,
    ) -> Result<ResponseDisposition, RuntimeError> {
        self.host.inner.shared.writable()?;
        if request.token.target() != &self.host.inner.id
            || request.generation != self.state.generation.get()
            || !self.state.connected.get()
        {
            return Ok(ResponseDisposition::Stale);
        }
        let matches = self
            .state
            .pending
            .borrow()
            .as_ref()
            .is_some_and(|p| p.request == *request);
        if !matches {
            return Ok(if self.receipt().is_some_and(|receipt| matches!(receipt.status(), AuthorityStatus::Resolved { request: prior, .. } if prior == *request)) {
                ResponseDisposition::AlreadyResolved
            } else { ResponseDisposition::Stale });
        }
        match response {
            AuthorityResponse::Accepted(value) => {
                self.host
                    .inner
                    .state
                    .borrow()
                    .spec()
                    .validate(&value)
                    .map_err(EditError::from)?;
                let sequence = EditSequence::new(
                    request
                        .sequence
                        .get()
                        .checked_add(1)
                        .ok_or(EditError::Exhausted)?,
                );
                self.host
                    .inner
                    .change(crate::EventKind::EditCommit, |state| {
                        state.commit(&request.token, sequence, value)
                    })?;
            }
            AuthorityResponse::Rejected => {
                self.host
                    .inner
                    .change(crate::EventKind::EditCommit, |state| {
                        state.reject_authority(&request.token)
                    })?;
            }
        }
        Ok(ResponseDisposition::Applied)
    }
    /// Marks connection loss without guessing whether an in-flight mutation ran.
    /// The binding becomes unavailable; pending receipt storage is retained.
    ///
    /// # Errors
    /// Returns [`RuntimeError::Closed`] after shutdown, and `WriteDuringUpdate`,
    /// `WriteDuringLayout`, `WriteDuringPaint`, `WriteDuringInspection` or
    /// `WriteDuringDelivery` when called from inside that phase; the binding then stays
    /// connected. When no mutation is pending it also forwards the
    /// [`EditError`](crate::EditError), as
    /// [`RuntimeError::Binding`](crate::RuntimeError::Binding), of cancelling the open
    /// local gesture.
    pub fn disconnect(&self) -> Result<(), RuntimeError> {
        self.host.inner.shared.writable()?;
        self.state.disconnect();
        if self.state.pending.borrow().is_none() {
            let token = self
                .host
                .inner
                .tracked
                .borrow()
                .as_ref()
                .map(|t| t.token.clone());
            if let Some(token) = token {
                self.host
                    .inner
                    .change(crate::EventKind::EditCancel, |state| state.cancel(&token))?;
            }
        }
        self.host.inner.publish();
        Ok(())
    }
    /// Installs a validated fresh authoritative base and starts a new connection
    /// generation. A formerly unknown receipt remains unknown: a snapshot alone
    /// cannot prove whether that old mutation was applied. No mutation is retried.
    /// Disconnect before recovering a pending proposal if its external outcome is
    /// unknown, preserving that state in its retained receipt. Called while still
    /// connected, this instead locally resolves a pending receipt as
    /// `Rejected(Conflict)`; that local supersession is not an authority refusal.
    /// Both paths make old-generation responses stale. Reconcile the external
    /// store before installing its baseline, regardless of the local receipt.
    ///
    /// # Errors
    /// Returns phase, unavailable, invalid-value or counter-exhaustion errors
    /// without reopening the connection.
    pub fn resynchronize(&self, value: T) -> Result<(), RuntimeError> {
        let generation = self
            .state
            .generation
            .get()
            .checked_add(1)
            .ok_or(EditError::Exhausted)?;
        self.host.write_base(value)?;
        self.state.generation.set(generation);
        self.state.connected.set(true);
        self.state.publish(AuthorityStatus::Ready);
        self.host.inner.publish();
        Ok(())
    }
}
impl<T: ParameterValue> Parameter<T> {
    /// External acceptance state for a remote binding; absent for a local one.
    /// Read during update to display pending/unknown outcomes independently of
    /// the base, optimistic preview and effective value. Built-in controls allow
    /// edits for a local binding (`None`), or `Ready`/`Resolved` remote state,
    /// subject to availability, editable metadata and delivery health. `Pending`,
    /// `Unknown` and `Disconnected` block new gestures while retaining display.
    /// Custom controls can reproduce this policy with an update-phase binding read.
    pub fn authority_status(&self) -> Option<ReadSignal<AuthorityStatus<T>>> {
        self.inner
            .remote
            .borrow()
            .as_ref()
            .map(|state| state.status.read_only())
    }
}
impl Runtime {
    /// Creates a parameter edited by ordinary controls but committed only after
    /// [`RemoteHost::respond`]. The initial authoritative base is the specification
    /// default; use `resynchronize` before exposing another initial value.
    ///
    /// # Errors
    /// Same identity and phase checks as [`Runtime::parameter`].
    pub fn remote_parameter<T: ParameterValue>(
        &mut self,
        id: ParameterId,
        spec: ParameterSpec<T>,
    ) -> Result<(Parameter<T>, RemoteHost<T>), RuntimeError> {
        let (parameter, host) = self.parameter(id, spec)?;
        let state = Rc::new(RemoteState {
            connected: Cell::new(true),
            generation: Cell::new(0),
            pending: RefCell::new(None),
            status: self.signal(AuthorityStatus::Ready),
            last: RefCell::new(None),
            submission_sink: RefCell::new(None),
            submission_delivery: self.signal(SubmissionDelivery::Unobserved),
        });
        *host.inner.remote.borrow_mut() = Some(state.clone());
        Ok((parameter, RemoteHost { host, state }))
    }
}
