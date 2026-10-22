//! Opt-in host notifications of edit outcomes and previews, with retained
//! delivery failures and their explicit recovery.
use super::*;

/// Why a notification registration, retry or resolution was refused.
///
/// Every refusal leaves registrations, retained reports and parameter state
/// unchanged.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum DeliveryError {
    /// Retrying an external proposal requires a registered submission callback.
    #[error("no submission sink is registered")]
    NoSubmissionSink,
    /// A sink of this kind is already registered; observers are never replaced.
    #[error("a delivery sink is already registered")]
    AlreadyRegistered,
    /// A preview sink needs a terminal sink first, so every audition has an end;
    /// retry needs a registered terminal sink.
    #[error("no terminal outcome sink is registered")]
    NoOutcomeSink,
    /// Registration changes and reconciliation require no active edit.
    #[error("the parameter has an active edit")]
    ActiveEdit,
    /// An unresolved delivery fault blocks new edits and registration changes
    /// until the application retries or resolves it.
    #[error("the parameter has an unresolved delivery fault")]
    Faulted,
    /// Recovery was requested without a retained delivery fault.
    #[error("the parameter has no delivery fault")]
    NotFaulted,
    /// The token does not identify the retained failed delivery, including a
    /// token of another incarnation.
    #[error("the token does not match the retained delivery")]
    UnknownEdit,
    /// The retained terminal event was already accepted by the sink; only an
    /// explicit resolution clears the remaining preview fault.
    #[error("the terminal event was already delivered")]
    AlreadyDelivered,
    /// Retirement requires the parameter to be removed first.
    #[error("the parameter has not been removed")]
    NotRemoved,
    /// The explicitly retried sink failed again; the monitor holds its error.
    #[error("the delivery sink failed")]
    SinkFailed,
}

/// The reason a delivery sink failed, as reported by the application's adapter.
///
/// The set is closed; [`Self::Other`] covers every reason the others do not.
/// `Display` writes a lowercase description, such as `downstream queue is full`.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum SinkErrorKind {
    /// A bounded downstream queue had no room.
    Full,
    /// The downstream receiver is gone.
    Closed,
    /// The adapter could not translate or accept the value.
    InvalidValue,
    /// Any other adapter failure.
    Other,
}
impl std::fmt::Display for SinkErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Full => "downstream queue is full",
            Self::Closed => "downstream receiver is closed",
            Self::InvalidValue => "adapter rejected the value",
            Self::Other => "adapter failed",
        })
    }
}

/// A delivery sink's failure: a closed reason plus an owned diagnostic message.
///
/// Foreign engine error objects never enter pert-ui signatures; an adapter keeps
/// its original error itself if it needs one. A sink returning this error keeps
/// its report in [`DeliveryMonitor`] or [`crate::SubmissionDelivery`] for explicit
/// recovery; external proposals remain retained by their authority owner.
///
/// ```
/// # use pert_ui_runtime as pert_ui;
/// use pert_ui::{SinkError, SinkErrorKind};
/// let error = SinkError::new(SinkErrorKind::Full, "engine queue full");
/// assert_eq!(error.kind(), SinkErrorKind::Full);
/// assert_eq!(error.to_string(), "downstream queue is full: engine queue full");
/// ```
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
#[error("{kind}: {message}")]
pub struct SinkError {
    kind: SinkErrorKind,
    message: String,
}

impl SinkError {
    /// Creates an error with a reason and an owned diagnostic message.
    #[must_use]
    pub fn new(kind: SinkErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }
    /// Returns the closed reason.
    #[must_use]
    pub fn kind(&self) -> SinkErrorKind {
        self.kind
    }
    /// Returns the diagnostic message.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }
}

/// The end of one admitted edit, delivered once to a terminal sink.
///
/// It carries the edit's identity and correlation data, the outcome and an owned
/// snapshot published before the call, so later reads cannot change its meaning.
/// `Committed` is the local store's acceptance, not an engine acknowledgement.
/// For remote parameters, correlate this token with the [`crate::AuthorityReceipt`]
/// retained for that submission's full token and connection generation. Keep
/// receipts per submission, rather than reading only the latest host receipt.
/// A disconnected request can stay [`crate::AuthorityStatus::Unknown`] even
/// when resynchronization or removal produces a local `Rejected` terminal event.
/// That event does not prove the external mutation never ran. A connected host
/// write can instead locally resolve the authority receipt as `Rejected(Conflict)`;
/// this supersession also does not prove external refusal. The host owns
/// reconciliation, deduplication and undo of externally unknown outcomes.
#[derive(Clone, Debug, PartialEq)]
pub struct TerminalEvent<T> {
    /// The edit identity: target incarnation, source and a non-reused serial.
    pub token: EditToken,
    /// The revision the edit began at.
    pub begin_revision: Revision,
    /// The base value when the edit began, an undo baseline for the application.
    pub begin_value: T,
    /// The last accepted preview's sequence, if any preview was accepted.
    pub last_preview_sequence: Option<EditSequence>,
    /// The first commit attempt's sequence whenever the edit attempted a commit,
    /// even if that commit was rejected. A submitted remote commit keeps it when
    /// the authority rejects the proposal or a conflicting write, replacement or
    /// removal ends the edit. `None` when the edit ended by cancellation or
    /// invalidation before any commit attempt.
    pub commit_sequence: Option<EditSequence>,
    /// The terminal outcome, identical to the edit session's retained outcome.
    pub outcome: EditOutcome<T>,
    /// The parameter state after the terminal transition was published.
    pub state: ParameterSnapshot<T>,
}

/// One accepted preview, delivered once and in increasing sequence order.
///
/// Delivered even when the value equals the previous preview. Stale and invalid
/// previews produce no event.
#[derive(Clone, Debug, PartialEq)]
pub struct PreviewEvent<T> {
    /// The edit identity.
    pub token: EditToken,
    /// The revision the edit began at.
    pub begin_revision: Revision,
    /// The base value when the edit began.
    pub begin_value: T,
    /// This preview's sequence within the edit.
    pub sequence: EditSequence,
    /// The accepted preview value.
    pub value: T,
}

/// Read-only delivery health of one parameter, shared by built-in and custom controls.
///
/// Recovery methods stay with the [`ParameterHost`]; availability after removal
/// is reported separately by the parameter snapshot. Pending engine work in an
/// adapter is not inferred from this local status.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum DeliveryStatus {
    /// No terminal sink is registered; only the local receipt protocol applies.
    #[default]
    Unobserved,
    /// A terminal sink is registered and no failure is retained.
    Ready,
    /// A delivery failed; new edits are refused until the host recovers.
    Faulted,
}

/// How the application resolves a retained delivery fault without a retry.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum DeliveryResolution {
    /// The application reconciled external state and takes responsibility for
    /// the retained outcome. This is its assertion, not library proof of engine
    /// state; an unknown engine result must not be labelled accepted.
    Reconciled,
    /// The parameter was removed and the adapter has released or transferred any
    /// audition cleanup. Clears the retained delivery; never resurrects the
    /// parameter.
    Retired,
}

/// Owned delivery report of one registration, readable after runtime closure.
///
/// Records registration closure separately from delivery health, so a closed
/// registration with a failure stays distinguishable from a clean closure.
/// Storage is bounded by event count, not bytes: at most one failed preview and
/// one terminal event.
#[derive(Clone, Debug, PartialEq)]
pub struct DeliverySnapshot<T> {
    /// Whether the sinks are still registered.
    pub registered: bool,
    /// The preview whose delivery failed, retained until explicit resolution.
    pub failed_preview: Option<PreviewEvent<T>>,
    /// A retained terminal event: undelivered after a sink error, or delivered
    /// but still held because a preview fault remains.
    pub terminal: Option<TerminalEvent<T>>,
    /// Whether the retained terminal event has been accepted by the sink.
    pub terminal_delivered: bool,
    /// The most recent sink error.
    pub last_error: Option<SinkError>,
}

impl<T> DeliverySnapshot<T> {
    fn open() -> Self {
        Self {
            registered: true,
            failed_preview: None,
            terminal: None,
            terminal_delivered: false,
            last_error: None,
        }
    }
    /// Whether a delivery failure is retained: a failed preview, or a terminal
    /// event the sink has not accepted.
    #[must_use]
    pub fn is_faulted(&self) -> bool {
        self.failed_preview.is_some() || (self.terminal.is_some() && !self.terminal_delivered)
    }
}

/// The application's handle on one registration's delivery report.
///
/// Holds report storage only, not the parameter or the sink closures, so keeping
/// it alive keeps neither the runtime nor the application's captures alive.
/// [`Self::snapshot`] works after the runtime closed; [`Self::snapshot_binding`] binds
/// the report in update contexts. Dropping the monitor does not detach the sinks;
/// the host owns their lifetime. Check it before release at shutdown: discarding
/// every handle abandons the in-memory report, which is not durable messaging.
#[must_use = "a delivery monitor is the only record of failed host deliveries"]
pub struct DeliveryMonitor<T> {
    report: Rc<RefCell<DeliverySnapshot<T>>>,
    signal: ReadSignal<DeliverySnapshot<T>>,
}

/// Shows whether the report is registered and faulted, without cloning it.
impl<T> core::fmt::Debug for DeliveryMonitor<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let mut debug = f.debug_struct("DeliveryMonitor");
        // The report is only borrowed mutably inside library operations that
        // run no formatting, so this borrow does not fail in practice; a
        // failure omits the fields rather than panicking.
        if let Ok(report) = self.report.try_borrow() {
            debug
                .field("registered", &report.registered)
                .field("faulted", &report.is_faulted());
        }
        debug.finish_non_exhaustive()
    }
}

impl<T> Clone for DeliveryMonitor<T> {
    fn clone(&self) -> Self {
        Self {
            report: self.report.clone(),
            signal: self.signal.clone(),
        }
    }
}

impl<T: Clone> DeliveryMonitor<T> {
    /// Copies the current report. Works after the runtime closed.
    pub fn snapshot(&self) -> DeliverySnapshot<T> {
        self.report.borrow().clone()
    }
    /// Returns the report as a read-only signal for update contexts.
    #[must_use]
    pub fn snapshot_binding(&self) -> ReadSignal<DeliverySnapshot<T>> {
        self.signal.clone()
    }
}

type TerminalSink<T> = Box<dyn FnMut(&TerminalEvent<T>) -> Result<(), SinkError>>;
type PreviewSink<T> = Box<dyn FnMut(&PreviewEvent<T>) -> Result<(), SinkError>>;

pub(super) struct Registration<T> {
    // `None` only while that sink's call is running.
    terminal: Option<TerminalSink<T>>,
    preview: Option<PreviewSink<T>>,
    has_preview: bool,
    report: Rc<RefCell<DeliverySnapshot<T>>>,
    signal: Signal<DeliverySnapshot<T>>,
    serial: u64,
}

pub(super) struct Tracked<T> {
    pub(super) token: EditToken,
    pub(super) begin_revision: Revision,
    pub(super) begin_value: T,
    pub(super) last_preview: Option<EditSequence>,
    pub(super) commit_sequence: Option<EditSequence>,
}

impl<T: ParameterValue> Inner<T> {
    pub(super) fn report(&self) -> Option<Rc<RefCell<DeliverySnapshot<T>>>> {
        self.registration
            .borrow()
            .as_ref()
            .map(|registration| registration.report.clone())
    }
    pub(super) fn delivery_status(&self) -> DeliveryStatus {
        match self.report() {
            None => DeliveryStatus::Unobserved,
            Some(report) if report.borrow().is_faulted() => DeliveryStatus::Faulted,
            Some(_) => DeliveryStatus::Ready,
        }
    }
    pub(super) fn faulted(&self) -> bool {
        self.delivery_status() == DeliveryStatus::Faulted
    }
    fn publish_report(&self, report: &Rc<RefCell<DeliverySnapshot<T>>>) {
        let copy = report.borrow().clone();
        let signal = self
            .registration
            .borrow()
            .as_ref()
            .filter(|registration| Rc::ptr_eq(&registration.report, report))
            .map(|registration| registration.signal.clone());
        if let Some(signal) = signal {
            signal.publish_trusted(copy);
        }
        self.status.publish_trusted(self.delivery_status());
    }
    /// Delivers now when a sink may run, otherwise queues for the next safe point.
    pub(super) fn deliver_terminal(&self, event: TerminalEvent<T>) {
        let Some(report) = self.report() else {
            return;
        };
        if !self.shared.can_deliver() {
            *self.deferred.borrow_mut() = Some(event);
            let this: Weak<dyn Deferred> = self.this.clone();
            self.shared.defer(this);
            return;
        }
        let result = self.call_terminal(&event);
        self.record_terminal(&report, event, result);
    }
    fn call_terminal(&self, event: &TerminalEvent<T>) -> Result<(), SinkError> {
        let taken = self
            .registration
            .borrow_mut()
            .as_mut()
            .and_then(|registration| Some((registration.terminal.take()?, registration.serial)));
        let Some((mut sink, serial)) = taken else {
            return Err(SinkError::new(
                SinkErrorKind::Closed,
                "terminal sink unavailable during its own call",
            ));
        };
        let result = {
            let _delivery = self.shared.enter_delivery();
            sink(event)
        };
        self.restore_terminal(sink, serial);
        result
    }
    fn restore_terminal(&self, sink: TerminalSink<T>, serial: u64) {
        let mut sink = Some(sink);
        if let Some(registration) = self.registration.borrow_mut().as_mut()
            && registration.serial == serial
        {
            registration.terminal = sink.take();
        }
        // A registration released during the call drops its sink here, after
        // every internal borrow is released.
        drop(sink);
    }
    fn record_terminal(
        &self,
        report: &Rc<RefCell<DeliverySnapshot<T>>>,
        event: TerminalEvent<T>,
        result: Result<(), SinkError>,
    ) {
        {
            let mut report = report.borrow_mut();
            match result {
                Ok(()) if report.failed_preview.is_none() => {
                    report.terminal = None;
                    report.terminal_delivered = false;
                }
                Ok(()) => {
                    report.terminal = Some(event);
                    report.terminal_delivered = true;
                }
                Err(error) => {
                    report.terminal = Some(event);
                    report.terminal_delivered = false;
                    report.last_error = Some(error);
                }
            }
        }
        self.publish_report(report);
    }
    pub(super) fn deliver_preview(&self, event: PreviewEvent<T>) {
        let Some(report) = self.report() else {
            return;
        };
        let taken = self
            .registration
            .borrow_mut()
            .as_mut()
            .and_then(|registration| Some((registration.preview.take()?, registration.serial)));
        let Some((mut sink, serial)) = taken else {
            return;
        };
        let result = {
            let _delivery = self.shared.enter_delivery();
            sink(&event)
        };
        let mut sink = Some(sink);
        if let Some(registration) = self.registration.borrow_mut().as_mut()
            && registration.serial == serial
        {
            registration.preview = sink.take();
        }
        drop(sink);
        if let Err(error) = result {
            let token = event.token.clone();
            {
                let mut report = report.borrow_mut();
                report.failed_preview = Some(event);
                report.last_error = Some(error);
            }
            self.publish_report(&report);
            // End the local edit; its terminal sink is attempted exactly once.
            let rejected = self.state.borrow_mut().reject_delivery(&token);
            debug_assert!(rejected.is_ok(), "an accepted preview has a live receipt");
            self.publish();
            if let Some(event) = self.take_terminal() {
                self.deliver_terminal(event);
            }
        }
        self.shared.dispatch_deferred();
    }
    /// Detaches the sinks. An undeliverable queued notification is stored in
    /// the report with a `Closed` error instead of being dropped.
    pub(super) fn release(&self) {
        let pending = self.deferred.borrow_mut().take();
        let registration = self.registration.borrow_mut().take();
        let Some(registration) = registration else {
            return;
        };
        {
            let mut report = registration.report.borrow_mut();
            report.registered = false;
            if let Some(event) = pending {
                report.terminal = Some(event);
                report.terminal_delivered = false;
                report.last_error = Some(SinkError::new(
                    SinkErrorKind::Closed,
                    "the registration closed before delivery",
                ));
            }
        }
        let copy = registration.report.borrow().clone();
        registration.signal.publish_trusted(copy);
        self.status.publish_trusted(DeliveryStatus::Unobserved);
        drop(registration);
    }
    pub(super) fn registration_change(&self) -> Result<(), RuntimeError> {
        self.shared.writable()?;
        self.dispatch_deferred();
        if self.state.borrow().snapshot().edit_source.is_some() {
            return Err(DeliveryError::ActiveEdit.into());
        }
        if self.faulted() {
            return Err(DeliveryError::Faulted.into());
        }
        Ok(())
    }
}

impl<T: ParameterValue> ParameterHost<T> {
    /// Registers the terminal sink and returns the monitor of its delivery report.
    ///
    /// There is one terminal sink per parameter. The sink receives every
    /// subsequently admitted edit's [`TerminalEvent`] once. `Ok` means the adapter
    /// accepted responsibility for the event; it must not then silently drop it.
    /// Install in a writable phase with no active edit and no fault.
    ///
    /// # Errors
    /// Returns [`DeliveryError::AlreadyRegistered`], [`DeliveryError::ActiveEdit`],
    /// [`DeliveryError::Faulted`] or a runtime phase error, without registering.
    pub fn on_outcome(
        &self,
        sink: impl FnMut(&TerminalEvent<T>) -> Result<(), SinkError> + 'static,
    ) -> Result<DeliveryMonitor<T>, RuntimeError> {
        self.inner.registration_change()?;
        if self.inner.registration.borrow().is_some() {
            return Err(DeliveryError::AlreadyRegistered.into());
        }
        let report = Rc::new(RefCell::new(DeliverySnapshot::open()));
        let signal = self.inner.snapshot.sibling(DeliverySnapshot::open());
        let serial = self.inner.registrations.get().wrapping_add(1);
        self.inner.registrations.set(serial);
        *self.inner.registration.borrow_mut() = Some(Registration {
            terminal: Some(Box::new(sink)),
            preview: None,
            has_preview: false,
            report: report.clone(),
            signal: signal.clone(),
            serial,
        });
        self.inner.publish_report(&report);
        Ok(DeliveryMonitor {
            report,
            signal: signal.read_only(),
        })
    }

    /// Registers the preview sink for audition; requires a terminal sink.
    ///
    /// Accepted previews are delivered once in increasing sequence order. Register
    /// only when the host supports audition and its cleanup: every audition ends
    /// with the terminal event, whatever the outcome.
    ///
    /// # Errors
    /// Returns [`DeliveryError::NoOutcomeSink`], [`DeliveryError::AlreadyRegistered`],
    /// [`DeliveryError::ActiveEdit`], [`DeliveryError::Faulted`] or a runtime
    /// phase error, without registering.
    pub fn on_preview(
        &self,
        sink: impl FnMut(&PreviewEvent<T>) -> Result<(), SinkError> + 'static,
    ) -> Result<(), RuntimeError> {
        self.inner.registration_change()?;
        let mut registration = self.inner.registration.borrow_mut();
        let registration = registration.as_mut().ok_or(DeliveryError::NoOutcomeSink)?;
        if registration.has_preview {
            return Err(DeliveryError::AlreadyRegistered.into());
        }
        registration.preview = Some(Box::new(sink));
        registration.has_preview = true;
        Ok(())
    }

    /// Detaches both sinks and closes the monitor's registration.
    ///
    /// # Errors
    /// Returns [`DeliveryError::ActiveEdit`], [`DeliveryError::Faulted`] or a
    /// runtime phase error. Clearing without a registration is harmless.
    pub fn clear_sinks(&self) -> Result<(), RuntimeError> {
        self.inner.registration_change()?;
        self.inner.release();
        Ok(())
    }

    /// Explicitly calls the terminal sink again with the retained event.
    ///
    /// The event is immutable: later host writes do not change it. Success marks
    /// it delivered and clears the fault unless a preview fault remains, which
    /// needs [`Self::resolve_delivery`]. Failure replaces only the last error.
    ///
    /// # Errors
    /// Returns [`DeliveryError::NotFaulted`], [`DeliveryError::UnknownEdit`],
    /// [`DeliveryError::AlreadyDelivered`], [`DeliveryError::NoOutcomeSink`],
    /// [`DeliveryError::SinkFailed`] after a new sink error, or a runtime phase
    /// error. Refusals change nothing.
    pub fn retry_outcome(&self, token: &EditToken) -> Result<(), RuntimeError> {
        self.inner.shared.writable()?;
        let report = self.inner.report().ok_or(DeliveryError::NotFaulted)?;
        let event = {
            let report = report.borrow();
            if !report.is_faulted() {
                return Err(DeliveryError::NotFaulted.into());
            }
            let event = report
                .terminal
                .as_ref()
                .filter(|event| &event.token == token)
                .ok_or(DeliveryError::UnknownEdit)?;
            if report.terminal_delivered {
                return Err(DeliveryError::AlreadyDelivered.into());
            }
            event.clone()
        };
        if self
            .inner
            .registration
            .borrow()
            .as_ref()
            .is_none_or(|registration| registration.terminal.is_none())
        {
            return Err(DeliveryError::NoOutcomeSink.into());
        }
        let result = self.inner.call_terminal(&event);
        let failed = result.is_err();
        self.inner.record_terminal(&report, event, result);
        self.inner.shared.dispatch_deferred();
        if failed {
            Err(DeliveryError::SinkFailed.into())
        } else {
            Ok(())
        }
    }

    /// Clears a retained fault after the application's own recovery.
    ///
    /// `token` names the retained terminal event or failed preview. Resolution
    /// releases delivery storage only, never a core receipt an edit guard owns.
    /// Copy the report first if an audit trail is needed.
    ///
    /// # Errors
    /// Returns [`DeliveryError::NotFaulted`], [`DeliveryError::UnknownEdit`],
    /// [`DeliveryError::ActiveEdit`], [`DeliveryError::NotRemoved`] for
    /// `Retired` before removal, or a runtime phase error. Refusals change nothing.
    pub fn resolve_delivery(
        &self,
        token: &EditToken,
        resolution: DeliveryResolution,
    ) -> Result<(), RuntimeError> {
        self.inner.shared.writable()?;
        let report = self.inner.report().ok_or(DeliveryError::NotFaulted)?;
        {
            let current = report.borrow();
            if !current.is_faulted() {
                return Err(DeliveryError::NotFaulted.into());
            }
            let matches = current
                .terminal
                .as_ref()
                .is_some_and(|event| &event.token == token)
                || current
                    .failed_preview
                    .as_ref()
                    .is_some_and(|event| &event.token == token);
            if !matches {
                return Err(DeliveryError::UnknownEdit.into());
            }
        }
        let snapshot = self.inner.state.borrow().snapshot();
        if snapshot.edit_source.is_some() {
            return Err(DeliveryError::ActiveEdit.into());
        }
        if resolution == DeliveryResolution::Retired && snapshot.available {
            return Err(DeliveryError::NotRemoved.into());
        }
        {
            let mut report = report.borrow_mut();
            report.failed_preview = None;
            report.terminal = None;
            report.terminal_delivered = false;
            report.last_error = None;
        }
        self.inner.publish_report(&report);
        Ok(())
    }
}
