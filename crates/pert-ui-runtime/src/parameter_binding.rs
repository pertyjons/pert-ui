//! Local parameter bindings of every family and their authoritative owners.
//!
//! Opt-in host notifications with retained delivery failures live in
//! `delivery`, the family-tagged wrappers in `erased` and external commit
//! acknowledgement in `remote`.
mod delivery;
mod erased;
pub(crate) mod remote;

pub use delivery::{
    DeliveryError, DeliveryMonitor, DeliveryResolution, DeliverySnapshot, DeliveryStatus,
    PreviewEvent, SinkError, SinkErrorKind, TerminalEvent,
};
use delivery::{Registration, Tracked};
pub use erased::{
    AnyDeliveryMonitor, AnyParameter, AnyParameterHost, AnyPreviewEvent, AnyTerminalEvent,
};

use std::{
    cell::{Cell, RefCell},
    rc::{Rc, Weak},
};

use crate::{
    AnyParameterSnapshot, AnySpec, AnyValue, ChoiceId, EditError, EditOutcome, EditSequence,
    EditSource, EditToken, EventContext, ParameterFamily, ParameterId, ParameterSnapshot,
    ParameterSpec, ParameterState, ParameterValue, PreviewOutcome, ReadSignal, Revision, Runtime,
    RuntimeError, Signal,
    runtime::{ContextLoss, Shared},
};

/// A parameter binding refused an operation: its edit protocol, its family or
/// its host delivery.
///
/// This is a leaf error. Binding operations return
/// [`RuntimeError`], which holds it as [`RuntimeError::Binding`]; the runtime's
/// own refusals, such as [`RuntimeError::WriteDuringDelivery`] inside a delivery
/// callback or [`RuntimeError::Closed`], stay other `RuntimeError` variants. A
/// failure therefore has one form: a control can treat `RuntimeError::Binding`
/// as a refused edit it reports locally and every other variant as a failure of
/// the component.
///
/// ```
/// use pert_ui_runtime::{
///     AnySpec, AnyValue, BindingError, ParameterFamily, ParameterId, ParameterSpec, Runtime,
///     RuntimeError,
/// };
/// let mut ui = Runtime::new();
/// let spec = AnySpec::from(ParameterSpec::new("Gain", 0.0, 1.0, 0.5)?);
/// let (_parameter, host) = ui.parameter_any(ParameterId::new(1, 1, "gain")?, spec)?;
/// assert_eq!(
///     host.write_base(AnyValue::from(true)),
///     Err(RuntimeError::Binding(BindingError::KindMismatch {
///         expected: ParameterFamily::Continuous,
///         found: ParameterFamily::Boolean,
///     }))
/// );
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum BindingError {
    /// The parameter edit protocol rejected the operation.
    #[error(transparent)]
    Edit(#[from] EditError),
    /// This complete parameter incarnation was already registered in this runtime.
    #[error("parameter incarnation was already registered")]
    DuplicateParameter,
    /// An erased value or specification belongs to another family. Nothing changed.
    #[error("expected a {expected} parameter value, found {found}")]
    KindMismatch {
        /// The family of the parameter.
        expected: ParameterFamily,
        /// The family that was supplied.
        found: ParameterFamily,
    },
    /// Host notification registration or recovery rejected the operation.
    #[error(transparent)]
    Delivery(#[from] DeliveryError),
}

struct Inner<T: ParameterValue> {
    remote: RefCell<Option<Rc<remote::RemoteState<T>>>>,
    shared: Rc<Shared>,
    this: Weak<Inner<T>>,
    id: ParameterId,
    state: RefCell<ParameterState<T>>,
    snapshot: Signal<ParameterSnapshot<T>>,
    status: Signal<DeliveryStatus>,
    registration: RefCell<Option<Registration<T>>>,
    registrations: Cell<u64>,
    tracked: RefCell<Option<Tracked<T>>>,
    deferred: RefCell<Option<TerminalEvent<T>>>,
}

trait OperationStatus {
    fn status(&self) -> crate::EventStatus;
}
impl OperationStatus for () {
    fn status(&self) -> crate::EventStatus {
        crate::EventStatus::Succeeded
    }
}
impl OperationStatus for EditToken {
    fn status(&self) -> crate::EventStatus {
        crate::EventStatus::Succeeded
    }
}
impl OperationStatus for PreviewOutcome {
    fn status(&self) -> crate::EventStatus {
        crate::EventStatus::Succeeded
    }
}
impl<T> OperationStatus for EditOutcome<T> {
    fn status(&self) -> crate::EventStatus {
        match self {
            Self::Committed { .. } | Self::Submitted { .. } => crate::EventStatus::Succeeded,
            Self::Cancelled => crate::EventStatus::Cancelled,
            Self::Rejected(error) => crate::EventStatus::Failed(error.to_string()),
            // `EditOutcome` is non-exhaustive; its documented reading of an
            // unknown outcome is a terminal result that did not commit.
            _ => crate::EventStatus::Cancelled,
        }
    }
}

/// A registered parameter as the runtime sees it: inspection and shutdown.
pub(crate) trait Registered {
    fn inspect(&self) -> crate::ParameterInspection;
    fn shut_down(&self);
}

/// A parameter holding a terminal notification queued for a later safe point.
pub(crate) trait Deferred {
    fn dispatch_deferred(&self);
    fn pending(&self) -> bool;
}

impl<T: ParameterValue> Registered for Inner<T> {
    fn inspect(&self) -> crate::ParameterInspection {
        let state = self.state.borrow();
        crate::ParameterInspection {
            id: self.id.clone(),
            parameter: AnyParameterSnapshot::new(state.spec().clone(), self.current_snapshot()),
            delivery: self.delivery_status(),
        }
    }
    fn shut_down(&self) {
        self.remove_and_release(true);
    }
}

impl<T: ParameterValue> Deferred for Inner<T> {
    fn dispatch_deferred(&self) {
        let event = self.deferred.borrow_mut().take();
        if let Some(event) = event {
            self.deliver_terminal(event);
        }
    }
    fn pending(&self) -> bool {
        self.deferred.borrow().is_some()
    }
}

impl<T: ParameterValue> Inner<T> {
    fn current_snapshot(&self) -> ParameterSnapshot<T> {
        let mut snapshot = self.state.borrow().snapshot();
        if self
            .remote
            .borrow()
            .as_ref()
            .is_some_and(|remote| !remote.connected.get())
        {
            snapshot.available = false;
        }
        snapshot
    }
    fn publish(&self) {
        let snapshot = self.current_snapshot();
        // Library-owned values: equality and drop run no user code, and callers
        // never receive a Ref into this cell, so publication cannot fail.
        self.snapshot.publish_trusted(snapshot);
    }
    fn change<R: OperationStatus>(
        &self,
        kind: crate::EventKind,
        change: impl FnOnce(&mut ParameterState<T>) -> Result<R, EditError>,
    ) -> Result<R, RuntimeError> {
        self.shared.writable().inspect_err(|error| {
            self.shared.record(kind, None, Some(&self.id), || {
                crate::EventStatus::Failed(error.to_string())
            })
        })?;
        // A queued notification for an earlier edit is delivered first.
        self.dispatch_deferred();
        let result = change(&mut self.state.borrow_mut());
        self.publish();
        self.shared
            .record(kind, None, Some(&self.id), || match &result {
                Ok(value) => value.status(),
                Err(error) => crate::EventStatus::Failed(error.to_string()),
            });
        let terminal = self.take_terminal();
        if let Some(remote) = self.remote.borrow().as_ref() {
            remote.synchronize(self);
        }
        if let Some(event) = terminal {
            self.deliver_terminal(event);
        }
        Ok(result?)
    }
    /// Builds the terminal event of the tracked edit once it has an outcome.
    fn take_terminal(&self) -> Option<TerminalEvent<T>> {
        let outcome = {
            let tracked = self.tracked.borrow();
            let token = &tracked.as_ref()?.token;
            self.state.borrow().outcome(token).ok().flatten()?
        };
        let tracked = self.tracked.borrow_mut().take()?;
        Some(TerminalEvent {
            token: tracked.token,
            begin_revision: tracked.begin_revision,
            begin_value: tracked.begin_value,
            last_preview_sequence: tracked.last_preview,
            commit_sequence: tracked.commit_sequence,
            outcome,
            state: self.current_snapshot(),
        })
    }
    /// Removal for host drop and runtime shutdown: ends an active edit, attempts
    /// its terminal notification, then releases the sinks.
    fn remove_and_release(&self, release: bool) {
        if let Some(remote) = self.remote.borrow().as_ref() {
            remote.retire();
        }
        self.dispatch_deferred();
        self.state.borrow_mut().remove();
        self.publish();
        self.shared.record(
            crate::EventKind::ParameterRemoved,
            None,
            Some(&self.id),
            || crate::EventStatus::Succeeded,
        );
        let terminal = self.take_terminal();
        if let Some(remote) = self.remote.borrow().as_ref() {
            remote.synchronize(self);
        }
        if let Some(event) = terminal {
            self.deliver_terminal(event);
        }
        if release {
            self.release();
        }
    }
}

/// UI-local editable binding for canonical values of family `T`.
///
/// Created with its [`ParameterHost`] by [`Runtime::parameter`]. Clones share one
/// local state. Controls read [`Self::snapshot`] and begin gestures in an event
/// context; they cannot perform authoritative external writes. Built-in and
/// custom controls use the same public session, snapshot and
/// [`Self::delivery_status`] interfaces. This handle is not `Send`/`Sync`; there
/// is no worker or callback-safe bridge.
///
/// Operations of a binding, its host and its sessions return [`RuntimeError`].
/// Each operation that changes state first checks the runtime; their `# Errors`
/// sections call that check's failures *a runtime phase error*:
/// [`RuntimeError::Closed`] after shutdown, or
/// [`RuntimeError::WriteDuringUpdate`], `WriteDuringLayout`,
/// `WriteDuringPaint`, `WriteDuringInspection` or `WriteDuringDelivery` from
/// inside that phase, including `WriteDuringDelivery` inside any host sink.
/// A phase error changes nothing. The edit, delivery and family refusals the
/// sections name arrive as [`RuntimeError::Binding`]: an [`EditError`] as
/// [`BindingError::Edit`], a [`DeliveryError`] as [`BindingError::Delivery`].
/// A control can therefore tell a refused edit, which it reports itself, from
/// any other runtime failure.
///
/// The handle stays on the UI thread; moving it to another thread does not
/// compile:
///
/// ```compile_fail
/// use pert_ui_runtime::{ParameterId, ParameterSpec, Runtime};
/// let runtime = Runtime::new();
/// let (parameter, _host) = runtime
///     .parameter(ParameterId::new(1, 1, "gain").unwrap(), ParameterSpec::new("Gain", 0.0, 1.0, 0.5).unwrap())
///     .unwrap();
/// std::thread::spawn(move || drop(parameter));
/// ```
pub struct Parameter<T: ParameterValue> {
    inner: Rc<Inner<T>>,
}

/// Shows the identity only; reading the state needs a live runtime, so use
/// [`Parameter::snapshot`] for values.
impl<T: ParameterValue> core::fmt::Debug for Parameter<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Parameter")
            .field("id", &self.inner.id)
            .finish_non_exhaustive()
    }
}

impl<T: ParameterValue> Clone for Parameter<T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<T: ParameterValue> Parameter<T> {
    /// Returns the complete immutable host-assigned identity.
    pub fn id(&self) -> &ParameterId {
        &self.inner.id
    }

    /// Returns a read-only observable snapshot of base, preview and effective values.
    ///
    /// Metadata replacements advance its revision too. Read this binding through
    /// the update context to invalidate only dependent controls automatically.
    #[must_use]
    pub fn snapshot(&self) -> ReadSignal<ParameterSnapshot<T>> {
        self.inner.snapshot.read_only()
    }

    /// Returns the read-only delivery health, [`DeliveryStatus::Unobserved`]
    /// unless the host registered a terminal sink.
    ///
    /// Every built-in parameter control observes [`DeliveryStatus::Faulted`] to
    /// stop interaction and show itself disabled; begins are refused while it
    /// lasts. Custom controls read the same signal.
    #[must_use]
    pub fn delivery_status(&self) -> ReadSignal<DeliveryStatus> {
        self.inner.status.read_only()
    }

    /// Clones the current specification. Observe the snapshot revision for changes.
    ///
    /// # Errors
    /// Returns `Closed` after the runtime is dropped.
    pub fn spec(&self) -> Result<ParameterSpec<T>, RuntimeError> {
        self.inner.shared.readable()?;
        Ok(self.inner.state.borrow().spec().clone())
    }

    /// Begins editing the currently observed base revision in this component event.
    ///
    /// Retain the returned guard for the gesture. Dropping it cancels unfinished
    /// work and releases its receipt; context loss cancels even if it is retained.
    /// A terminal notification caused by context loss or a dropped session is
    /// delivered at the next safe point: the next operation on this parameter or
    /// the next flush, which [`Runtime::next_deadline`] reports as due at once.
    ///
    /// # Errors
    /// Returns [`RuntimeError::ForeignRuntime`] for a context of another
    /// runtime; a runtime phase error (see [`Parameter`]); [`EditError::Unavailable`] for a
    /// removed parameter or a disconnected remote one;
    /// [`DeliveryError::Faulted`] while a delivery fault is retained; and the
    /// admission refusals of [`ParameterState::begin`]: [`EditError::ReadOnly`],
    /// [`EditError::StaleRevision`] when `expected` is not the current
    /// revision, [`EditError::Busy`], [`EditError::Capacity`] and
    /// [`EditError::Exhausted`]. A refused begin changes nothing.
    pub fn begin_edit(
        &self,
        context: &mut EventContext,
        expected: Revision,
    ) -> Result<EditSession<T>, RuntimeError> {
        let _timing = self
            .inner
            .shared
            .profiler
            .span(crate::ProfileOperation::EditBegin, None, "");
        let preflight = (|| {
            if !Rc::ptr_eq(&self.inner.shared, &context.node.shared) {
                return Err(RuntimeError::ForeignRuntime);
            }
            self.inner.shared.writable()?;
            self.inner.dispatch_deferred();
            if self
                .inner
                .remote
                .borrow()
                .as_ref()
                .is_some_and(|remote| !remote.connected.get())
            {
                return Err(EditError::Unavailable.into());
            }
            if self.inner.faulted() {
                return Err(DeliveryError::Faulted.into());
            }
            Ok::<(), RuntimeError>(())
        })();
        preflight.inspect_err(|error| {
            self.inner.shared.record(
                crate::EventKind::EditBegin,
                None,
                Some(&self.inner.id),
                || crate::EventStatus::Failed(error.to_string()),
            );
        })?;
        let token = self.inner.change(crate::EventKind::EditBegin, |state| {
            state.begin(expected, EditSource::new(context.source()))
        })?;
        *self.inner.tracked.borrow_mut() = Some(Tracked {
            token: token.clone(),
            begin_revision: expected,
            begin_value: self.inner.state.borrow().snapshot().base,
            last_preview: None,
            commit_sequence: None,
        });
        let gesture = Rc::new(Gesture {
            parameter: self.inner.clone(),
            token,
            sequence: Cell::new(0),
        });
        let cleanup: Rc<dyn ContextLoss> = gesture.clone();
        context.attach(&cleanup);
        Ok(EditSession { gesture })
    }
}

/// Authoritative owner of a synchronous local parameter, separate from its controls.
///
/// Dropping the owner removes the incarnation, rejects an active edit as
/// unavailable, attempts its terminal notification and then releases the sinks.
/// The owner is intentionally not `Clone`; controls clone [`Parameter`]. It owns
/// no application processor, persistence or undo stack and is UI-thread-local.
///
/// # Host notifications
///
/// Delivery is opt-in. With no sink only the local receipt protocol applies. Call
/// [`Self::on_outcome`] before exposing editable controls to observe each edit's
/// end once, for application undo or forwarding, and optionally
/// [`Self::on_preview`] for audition. Dispatch is synchronous, after the whole
/// transition and snapshot publication, with no internal borrow held, before the
/// initiating operation returns. External writes, replacement and removal publish
/// their final state before the invalidated edit's terminal event. Replays, failed
/// begins, stale or invalid previews and writes without an active edit produce no
/// event, and there is no backfill of earlier edits.
///
/// During a sink call reads are permitted, but every write to the owning runtime,
/// its signals, bindings, registrations and sessions fails with
/// [`RuntimeError::WriteDuringDelivery`]; defer UI work to the next application
/// event. Cleanup a sink causes, such as dropping a session, applies at once and
/// is notified after the call returns, never recursively. Sinks must return
/// promptly, must not block, panic or call time-critical producer code; the library neither
/// preempts nor bounds their duration and does not catch panics. Sinks should not
/// capture strong handles that form ownership cycles.
///
/// | Situation | Result |
/// |---|---|
/// | Terminal sink returns `Ok` | The adapter accepted responsibility; this is not engine acknowledgement. |
/// | Terminal sink returns `Err` | The event and error are retained, the parameter is faulted and refuses edits; the local outcome is unchanged. |
/// | Preview sink returns `Err` | The preview and error are retained, the edit ends as `Rejected(DeliveryFailed)` with its preview cleared, and the terminal sink is attempted once. The fault remains even if that succeeds. |
/// | [`Self::retry_outcome`] | Calls the terminal sink again with the same event. Success clears a terminal-only fault. |
/// | [`Self::resolve_delivery`] | The application reconciled, or retired a removed parameter; clears the report. |
///
/// There is no automatic retry, rollback or polling. Local commit results are
/// unchanged by delivery failures: a commit still returns `Committed`. Explicit
/// [`Self::remove`] keeps the sinks for recovery; drop and runtime shutdown
/// release them, storing a notification that could not run with a
/// [`SinkErrorKind::Closed`] error. No callback runs after shutdown.
///
/// An application undo stack fed by terminal events, with a toggle as the
/// editing control:
///
/// ```
/// use std::{cell::RefCell, rc::Rc};
/// use pert_ui_runtime::{
///     EditOutcome, Element, KeyAction, KeyCode, ParameterId, ParameterSpec, Runtime, Toggle,
/// };
///
/// let mut ui = Runtime::new();
/// let (bypass, host) = ui.parameter(
///     ParameterId::new(1, 1, "bypass")?,
///     ParameterSpec::boolean("Bypass", false)?,
/// )?;
/// let undo = Rc::new(RefCell::new(Vec::new()));
/// let stack = undo.clone();
/// let monitor = host.on_outcome(move |event| {
///     if let EditOutcome::Committed { .. } = event.outcome {
///         stack.borrow_mut().push((event.token.clone(), event.begin_value));
///     }
///     Ok(())
/// })?;
/// let toggle = ui.mount_tree(Element::new("bypass", Toggle::for_parameter(bypass.clone())))?;
/// ui.focus(&toggle)?;
/// ui.key_input(KeyCode::Space, KeyAction::Press)?;
/// ui.key_input(KeyCode::Space, KeyAction::Release)?;
/// assert!(bypass.snapshot().get()?.base);
/// assert_eq!(undo.borrow().len(), 1);
/// assert!(!monitor.snapshot().is_faulted());
///
/// // Undo is an authoritative host write of the recorded value.
/// let (_, before) = undo.borrow_mut().pop().ok_or("an undo entry")?;
/// host.write_base(before)?;
/// assert!(!bypass.snapshot().get()?.base);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
///
/// The host side stays on the UI thread as well:
///
/// ```compile_fail
/// fn send<T: Send>() {}
/// send::<pert_ui_runtime::ParameterHost<f32>>();
/// ```
pub struct ParameterHost<T: ParameterValue> {
    inner: Rc<Inner<T>>,
}

/// Shows the identity only, like [`Parameter`]'s `Debug`.
impl<T: ParameterValue> core::fmt::Debug for ParameterHost<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("ParameterHost")
            .field("id", &self.inner.id)
            .finish_non_exhaustive()
    }
}

impl<T: ParameterValue> ParameterHost<T> {
    /// Returns the complete immutable host-assigned identity, the same as the
    /// bound [`Parameter::id`]. Readable after removal and runtime shutdown.
    pub fn id(&self) -> &ParameterId {
        &self.inner.id
    }

    /// Performs an external authoritative write, invalidating any active preview.
    ///
    /// Even an equal value advances the revision and ends an active edit as
    /// `Rejected(Conflict)`: the new base is published, then that edit's
    /// [`TerminalEvent`](crate::TerminalEvent), with its `begin_value`, is
    /// delivered to the terminal sink before this returns. Without an active
    /// edit no sink is called; an application that writes the base, for undo or
    /// reset, forwards the value to its engine itself. Engine acknowledgements of an edit the adapter
    /// already accepted are adapter bookkeeping, not a write: echoing them here
    /// would spuriously cancel a newer gesture.
    ///
    /// # Errors
    /// Returns a runtime phase error (see [`Parameter`]), or the refusals of
    /// [`ParameterState::write_base`]: [`EditError::Unavailable`] after
    /// removal, [`EditError::InvalidValue`] and [`EditError::Exhausted`]. A
    /// refused write changes nothing.
    pub fn write_base(&self, value: T) -> Result<(), RuntimeError> {
        let _timing = self
            .inner
            .shared
            .profiler
            .span(crate::ProfileOperation::HostWrite, None, "");
        self.inner
            .change(crate::EventKind::HostWrite, |state| state.write_base(value))
    }

    /// Replaces metadata atomically after validating the current base and overlay,
    /// and invalidates edits; see [`ParameterState::replace_spec`].
    ///
    /// # Errors
    /// Returns a runtime phase error (see [`Parameter`]), or the refusals of
    /// [`ParameterState::replace_spec`]: [`EditError::Unavailable`] after
    /// removal, [`EditError::InvalidValue`] when the new specification rejects
    /// the base or the overlay, and [`EditError::Exhausted`]. A refused
    /// replacement changes nothing.
    pub fn replace_spec(&self, spec: ParameterSpec<T>) -> Result<(), RuntimeError> {
        let _timing = self
            .inner
            .shared
            .profiler
            .span(crate::ProfileOperation::Metadata, None, "");
        self.inner
            .change(crate::EventKind::Metadata, |state| state.replace_spec(spec))
    }

    /// Supplies an effective overlay without modifying base or preview.
    ///
    /// `None` restores local preview/base following. This is a synchronous testing/
    /// host seam, not sequenced telemetry or a callback-safe sender. Validation
    /// follows [`ParameterSpec::validate_effective`]. Built-in parameter controls
    /// present editable preview/base, not this read-only overlay. Use a custom
    /// display subscribing to [`Parameter::snapshot`] to show effective values;
    /// validate against the editable spec to indicate clipping without normalizing
    /// or writing the overlay back.
    ///
    /// ```
    /// use pert_ui_runtime::{Element, InlineReadout, ParameterId, ParameterSpec, Runtime};
    /// let mut ui = Runtime::new();
    /// let spec = ParameterSpec::new("Amount", 0.0, 1.0, 0.25)?;
    /// let (parameter, host) = ui.parameter(ParameterId::new(1, 1, "amount")?, spec.clone())?;
    /// let display = parameter.snapshot().map(move |snapshot| {
    ///     let shown = snapshot.effective; // already falls back to preview/base
    ///     if spec.validate(&shown).is_err() {
    ///         format!("{shown} (outside editable domain)")
    ///     } else {
    ///         shown.to_string()
    ///     }
    /// })?;
    /// ui.mount_tree(Element::new(
    ///     "effective",
    ///     InlineReadout::new(display.clone()),
    /// ))?;
    /// host.set_effective(Some(1.5))?;
    /// assert_eq!(display.get()?, "1.5 (outside editable domain)");
    /// assert_eq!(parameter.snapshot().get()?.base, 0.25);
    /// host.set_effective(None)?;
    /// assert_eq!(display.get()?, "0.25");
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// # Errors
    /// Returns a runtime phase error (see [`Parameter`]), [`EditError::Unavailable`] after
    /// removal, or [`EditError::InvalidValue`] for an overlay
    /// [`ParameterSpec::validate_effective`] rejects. A refused overlay changes
    /// nothing.
    pub fn set_effective(&self, value: Option<T>) -> Result<(), RuntimeError> {
        let _timing = self
            .inner
            .shared
            .profiler
            .span(crate::ProfileOperation::Effective, None, "");
        self.inner.change(crate::EventKind::Effective, |state| {
            state.set_effective(value)
        })
    }

    /// Removes this incarnation. Retained controls become unavailable; an active
    /// edit ends as `Rejected(Unavailable)` and is notified. Sinks and any
    /// retained report stay available for [`Self::retry_outcome`] and
    /// [`Self::resolve_delivery`].
    ///
    /// # Errors
    /// Returns only a runtime phase error (see [`Parameter`]). Repeated removal is harmless.
    pub fn remove(&self) -> Result<(), RuntimeError> {
        self.inner.shared.writable()?;
        if let Some(remote) = self.inner.remote.borrow().as_ref() {
            remote.retire();
        }
        let _timing =
            self.inner
                .shared
                .profiler
                .span(crate::ProfileOperation::ParameterRemoved, None, "");
        self.inner
            .change(crate::EventKind::ParameterRemoved, |state| {
                state.remove();
                Ok(())
            })
    }
}

impl<T: ParameterValue> Drop for ParameterHost<T> {
    fn drop(&mut self) {
        let _timing =
            self.inner
                .shared
                .profiler
                .span(crate::ProfileOperation::ParameterRemoved, None, "");
        self.inner.remove_and_release(true);
    }
}

struct Gesture<T: ParameterValue> {
    parameter: Rc<Inner<T>>,
    token: EditToken,
    sequence: Cell<u64>,
}
impl<T: ParameterValue> ContextLoss for Gesture<T> {
    fn cancel(&self) {
        if self
            .parameter
            .remote
            .borrow()
            .as_ref()
            .is_some_and(|remote| remote.submitted(&self.token).is_some())
        {
            return;
        }
        let _timing =
            self.parameter
                .shared
                .profiler
                .span(crate::ProfileOperation::EditCancel, None, "");
        // The facade owns this token until guard drop; it has not been acknowledged.
        // No user callback executes while the core is borrowed.
        let result = {
            let mut state = self.parameter.state.borrow_mut();
            match state.outcome(&self.token) {
                Ok(Some(_)) => return,
                Ok(None) => state.cancel(&self.token),
                Err(error) => Err(error),
            }
        };
        self.parameter.shared.record(
            crate::EventKind::EditCancel,
            None,
            Some(&self.parameter.id),
            || match &result {
                Ok(value) => value.status(),
                Err(error) => crate::EventStatus::Failed(error.to_string()),
            },
        );
        debug_assert!(result.is_ok(), "retained gesture token must have a receipt");
        self.parameter.publish();
        // Lifecycle cleanup runs inside runtime operations: notify at the next
        // safe point rather than calling application code from here.
        if let Some(event) = self.parameter.take_terminal()
            && self.parameter.report().is_some()
        {
            *self.parameter.deferred.borrow_mut() = Some(event);
            let this: Weak<dyn Deferred> = self.parameter.this.clone();
            self.parameter.shared.defer(this);
        }
    }
}

/// One local edit gesture with automatic sequence ordering and terminal cleanup.
///
/// It is not cloneable or transferable between threads. Repeated terminal methods
/// replay the first outcome while the guard exists. Dropping it cancels unfinished
/// work and acknowledges the reserved receipt. Automatic cleanup of a finished
/// gesture emits no cancellation event. Context loss also cancels unfinished work; use
/// [`Self::outcome`] to inspect the retained result before dropping the guard.
/// Terminal lifecycle cleanup is permitted during an update; ordinary writes and
/// explicit gesture methods are still rejected in that phase.
///
/// ```compile_fail
/// fn send<T: Send>() {}
/// send::<pert_ui_runtime::EditSession<f32>>();
/// ```
#[must_use = "dropping an edit session cancels unfinished work"]
pub struct EditSession<T: ParameterValue> {
    gesture: Rc<Gesture<T>>,
}

/// Shows the edit identity only; [`EditSession::outcome`] reads the result.
impl<T: ParameterValue> core::fmt::Debug for EditSession<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("EditSession")
            .field("token", &self.gesture.token)
            .finish_non_exhaustive()
    }
}

impl<T: ParameterValue> EditSession<T> {
    fn next_sequence(&self) -> Result<EditSequence, EditError> {
        self.gesture
            .sequence
            .get()
            .checked_add(1)
            .map(EditSequence::new)
            .ok_or(EditError::Exhausted)
    }

    /// Returns the edit identity, as carried by host notifications.
    pub fn token(&self) -> &EditToken {
        &self.gesture.token
    }

    /// Sends a validated preview with the next sequence, leaving the base unchanged.
    ///
    /// An accepted preview is delivered to a registered preview sink before this
    /// returns. If that delivery fails, this still returns `Applied`, but the edit
    /// has ended as `Rejected(DeliveryFailed)`; [`Self::outcome`] shows it.
    ///
    /// # Errors
    /// Returns a runtime phase error (see [`Parameter`]); [`EditError::Pending`] once a
    /// remote binding has submitted this edit's proposal;
    /// [`EditError::Finished`] after the edit ended; [`EditError::InvalidValue`]
    /// for a candidate the specification rejects; and [`EditError::Exhausted`]
    /// when the sequence cannot advance. Invalid candidates do not consume a
    /// sequence or terminate the gesture.
    pub fn update(&self, value: T) -> Result<PreviewOutcome, RuntimeError> {
        self.gesture.parameter.shared.writable()?;
        let parameter = &self.gesture.parameter;
        if parameter
            .remote
            .borrow()
            .as_ref()
            .is_some_and(|remote| remote.submitted(&self.gesture.token).is_some())
        {
            return Err(EditError::Pending.into());
        }
        let _timing =
            parameter
                .shared
                .profiler
                .span(crate::ProfileOperation::EditPreview, None, "");
        let sequence = self.next_sequence()?;
        let preview = value.clone();
        let result = parameter.change(crate::EventKind::EditPreview, |state| {
            state.update(&self.gesture.token, sequence, value)
        })?;
        self.gesture.sequence.set(sequence.get());
        if result == PreviewOutcome::Applied {
            let event = {
                let mut tracked = parameter.tracked.borrow_mut();
                tracked
                    .as_mut()
                    .filter(|tracked| tracked.token == self.gesture.token)
                    .map(|tracked| {
                        tracked.last_preview = Some(sequence);
                        PreviewEvent {
                            token: tracked.token.clone(),
                            begin_revision: tracked.begin_revision,
                            begin_value: tracked.begin_value.clone(),
                            sequence,
                            value: preview,
                        }
                    })
            };
            if let Some(event) = event {
                parameter.deliver_preview(event);
            }
        }
        Ok(result)
    }

    /// Commits a full final value once, or returns the previously retained outcome.
    /// A remote binding returns `Submitted` while external acceptance is pending;
    /// its base is unchanged until `RemoteHost::respond`. Repeated submission
    /// returns the original candidate and does not send a new mutation.
    ///
    /// # Errors
    /// Returns a runtime phase error (see [`Parameter`]), [`EditError::Exhausted`] when the
    /// sequence, or for a remote binding the revision its acceptance takes,
    /// cannot advance, and for a remote binding [`EditError::Unavailable`]
    /// while it is disconnected. Core commit validation failure is a terminal
    /// [`EditOutcome::Rejected`] result, releasing the preview, not an error.
    pub fn commit(&self, value: T) -> Result<EditOutcome<T>, RuntimeError> {
        let parameter = &self.gesture.parameter;
        let _timing = parameter
            .shared
            .profiler
            .span(crate::ProfileOperation::EditCommit, None, "");
        parameter.shared.writable().inspect_err(|error| {
            parameter.shared.record(
                crate::EventKind::EditCommit,
                None,
                Some(&parameter.id),
                || crate::EventStatus::Failed(error.to_string()),
            );
        })?;
        if let Some(outcome) = self.outcome()? {
            parameter.shared.record(
                crate::EventKind::EditCommit,
                None,
                Some(&parameter.id),
                || outcome.status(),
            );
            return Ok(outcome);
        }
        if let Some(value) = parameter
            .remote
            .borrow()
            .as_ref()
            .and_then(|r| r.submitted(&self.gesture.token))
        {
            parameter.shared.record(
                crate::EventKind::EditCommit,
                None,
                Some(&parameter.id),
                || crate::EventStatus::Succeeded,
            );
            return Ok(EditOutcome::Submitted { value });
        }
        let sequence = self.next_sequence().inspect_err(|error| {
            parameter.shared.record(
                crate::EventKind::EditCommit,
                None,
                Some(&parameter.id),
                || crate::EventStatus::Failed(error.to_string()),
            );
        })?;
        if let Some(tracked) = parameter
            .tracked
            .borrow_mut()
            .as_mut()
            .filter(|tracked| tracked.token == self.gesture.token)
        {
            tracked.commit_sequence.get_or_insert(sequence);
        }
        if let Some(remote) = parameter.remote.borrow().as_ref() {
            let result = remote.submit(parameter, &self.gesture.token, sequence, value)?;
            self.gesture.sequence.set(sequence.get());
            return Ok(result);
        }
        let result = parameter.change(crate::EventKind::EditCommit, |state| {
            state.commit(&self.gesture.token, sequence, value)
        })?;
        self.gesture.sequence.set(sequence.get());
        Ok(result)
    }

    /// Cancels an unfinished gesture or returns its existing terminal result.
    /// A submitted remote proposal cannot be revoked here: returns `Submitted`
    /// and preserves its retained request/receipt for host reconciliation.
    ///
    /// # Errors
    /// Returns only a runtime phase error (see [`Parameter`]). The session's own token always
    /// has a receipt, so [`EditError::UnknownEdit`] would indicate a
    /// token-lifecycle defect.
    pub fn cancel(&self) -> Result<EditOutcome<T>, RuntimeError> {
        self.gesture.parameter.shared.writable()?;
        let parameter = &self.gesture.parameter;
        if let Some(value) = parameter
            .remote
            .borrow()
            .as_ref()
            .and_then(|remote| remote.submitted(&self.gesture.token))
        {
            return Ok(EditOutcome::Submitted { value });
        }
        let _timing = parameter
            .shared
            .profiler
            .span(crate::ProfileOperation::EditCancel, None, "");
        parameter.change(crate::EventKind::EditCancel, |state| {
            state.cancel(&self.gesture.token)
        })
    }

    /// Returns the retained terminal result, even after context/runtime loss.
    ///
    /// # Errors
    /// A valid facade-owned guard always has a receipt; `UnknownEdit` would indicate
    /// a token-lifecycle error. `None` means the gesture is still pending.
    pub fn outcome(&self) -> Result<Option<EditOutcome<T>>, EditError> {
        self.gesture
            .parameter
            .state
            .borrow()
            .outcome(&self.gesture.token)
    }
}

impl<T: ParameterValue> Drop for EditSession<T> {
    fn drop(&mut self) {
        if self
            .gesture
            .parameter
            .remote
            .borrow()
            .as_ref()
            .is_some_and(|remote| remote.drop_guard(&self.gesture.token))
        {
            return;
        }
        self.gesture.cancel();
        let result = self
            .gesture
            .parameter
            .state
            .borrow_mut()
            .acknowledge(&self.gesture.token);
        debug_assert!(
            result.is_ok(),
            "guard acknowledges exactly one retained receipt"
        );
    }
}

impl Runtime {
    /// Creates one synchronous local parameter of family `T` and its authoritative owner.
    ///
    /// The parameter starts at the specification's default and revision zero.
    /// Controls clone the returned binding. Retain the host owner for as long as
    /// the parameter exists. Registered identities, including removed ones, remain
    /// reserved until runtime drop; recreate using `next_incarnation`, not the old
    /// ID. Registration prunes dead inspection references in a linear pass over
    /// the retained registry. Inspection also prunes; neither operation schedules
    /// work.
    ///
    /// ```
    /// use pert_ui_runtime::{Element, ParameterId, ParameterSpec, Runtime, Slider};
    /// let mut ui = Runtime::new();
    /// let (amount, host) = ui.parameter(
    ///     ParameterId::new(1, 1, "amount")?,
    ///     ParameterSpec::new("Amount", 0.0, 1.0, 0.5)?,
    /// )?;
    /// ui.mount_tree(Element::new("amount", Slider::new(amount.clone())))?;
    /// host.write_base(0.75)?; // an authoritative application update
    /// assert_eq!(amount.snapshot().get()?.base, 0.75);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// # Errors
    /// Returns a runtime phase error (see [`Parameter`]), or [`BindingError::DuplicateParameter`]
    /// for a complete incarnation this runtime registered before, even one
    /// since removed.
    pub fn parameter<T: ParameterValue>(
        &mut self,
        id: ParameterId,
        spec: ParameterSpec<T>,
    ) -> Result<(Parameter<T>, ParameterHost<T>), RuntimeError> {
        self.shared.writable()?;
        if !self.parameter_ids.insert(id.clone()) {
            return Err(BindingError::DuplicateParameter.into());
        }
        let state = ParameterState::new(id.clone(), spec);
        let snapshot = self.signal(state.snapshot());
        let status = self.signal(DeliveryStatus::Unobserved);
        let inner = Rc::new_cyclic(|this| Inner {
            remote: RefCell::new(None),
            shared: self.shared.clone(),
            this: this.clone(),
            id,
            snapshot,
            status,
            state: RefCell::new(state),
            registration: RefCell::new(None),
            registrations: Cell::new(0),
            tracked: RefCell::new(None),
            deferred: RefCell::new(None),
        });
        let registered: Rc<dyn Registered> = inner.clone();
        let mut parameters = self.parameters.borrow_mut();
        parameters.retain(|weak| weak.strong_count() > 0);
        parameters.push(Rc::downgrade(&registered));
        Ok((
            Parameter {
                inner: inner.clone(),
            },
            ParameterHost { inner },
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refused_commit_phase_and_sequence_reservation_attempts_are_recorded_once()
    -> Result<(), Box<dyn std::error::Error>> {
        for remote in [false, true] {
            let mut runtime = Runtime::new();
            runtime.set_debug_mode(crate::DebugMode::Inspect);
            let spec = ParameterSpec::new("Commit", 0.0, 1.0, 0.25)?;
            let id = ParameterId::new(1, 1, "commit")?;
            let (parameter, local, authority) = if remote {
                let (parameter, host) = runtime.remote_parameter(id, spec)?;
                (parameter, None, Some(host))
            } else {
                let (parameter, host) = runtime.parameter(id, spec)?;
                (parameter, Some(host), None)
            };
            let owner = runtime.mount("owner", crate::Label::new("Owner"))?;
            let edit = runtime.event(&owner, |cx| {
                parameter.begin_edit(cx, parameter.snapshot().get()?.revision)
            })??;
            runtime.clear_event_history();
            let delivery = parameter.inner.shared.enter_delivery();
            assert_eq!(edit.commit(0.6), Err(RuntimeError::WriteDuringDelivery));
            drop(delivery);
            let events = runtime.inspect().events;
            assert_eq!(events.len(), 1);
            assert_eq!(events[0].kind, crate::EventKind::EditCommit);
            assert!(matches!(events[0].status, crate::EventStatus::Failed(_)));
            for sequence in [u64::MAX, if remote { u64::MAX - 1 } else { u64::MAX }] {
                runtime.clear_event_history();
                edit.gesture.sequence.set(sequence);
                assert_eq!(
                    edit.commit(0.6),
                    Err(RuntimeError::from(EditError::Exhausted))
                );
                let events = runtime.inspect().events;
                assert_eq!(events.len(), 1);
                assert_eq!(events[0].kind, crate::EventKind::EditCommit);
                assert!(matches!(events[0].status, crate::EventStatus::Failed(_)));
                assert_eq!(parameter.snapshot().get()?.base, 0.25);
                assert_eq!(parameter.snapshot().get()?.preview, None);
                assert!(
                    authority
                        .as_ref()
                        .is_none_or(|host| host.request().is_none())
                );
            }
            edit.gesture.sequence.set(0);
            edit.cancel()?;
            drop((local, authority));
        }
        Ok(())
    }

    #[test]
    fn begin_refusals_during_delivery_and_after_shutdown_are_recorded_once()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut runtime = Runtime::new();
        runtime.set_debug_mode(crate::DebugMode::Inspect);
        let (parameter, _host) = runtime.parameter(
            ParameterId::new(1, 1, "phase")?,
            ParameterSpec::new("Phase", 0.0, 1.0, 0.25)?,
        )?;
        let owner = runtime.mount("owner", crate::Label::new("Owner"))?;
        let mut context = EventContext {
            node: owner.0.clone(),
        };
        let revision = parameter.snapshot().get()?.revision;
        runtime.clear_event_history();
        let delivery = parameter.inner.shared.enter_delivery();
        assert_eq!(
            parameter.begin_edit(&mut context, revision).err(),
            Some(RuntimeError::WriteDuringDelivery)
        );
        drop(delivery);
        let events = runtime.inspect().events;
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].kind, crate::EventKind::EditBegin);
        assert_eq!(events[0].parameter.as_ref(), Some(parameter.id()));
        assert!(matches!(events[0].status, crate::EventStatus::Failed(_)));
        runtime.shutdown();
        runtime.clear_event_history();
        assert_eq!(
            parameter.begin_edit(&mut context, revision).err(),
            Some(RuntimeError::Closed)
        );
        let events = runtime.inspect().events;
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].kind, crate::EventKind::EditBegin);
        assert!(matches!(events[0].status, crate::EventStatus::Failed(_)));
        Ok(())
    }

    #[test]
    fn inspection_registry_reclaims_dead_entries_without_releasing_identities()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut runtime = Runtime::new();
        let spec = ParameterSpec::new("Value", 0.0, 1.0, 0.0)?;
        let original = ParameterId::new(1, 1, "value")?;
        let (retained, owner) = runtime.parameter(original.clone(), spec.clone())?;
        drop(owner);
        for object in 2..1002 {
            let (parameter, host) =
                runtime.parameter(ParameterId::new(1, object, "value")?, spec.clone())?;
            assert_eq!(runtime.parameters.borrow().len(), 2);
            drop((parameter, host));
        }
        // Inspect without another registration: prune the final dead entry too.
        let snapshot = runtime.inspect();
        assert_eq!(runtime.parameters.borrow().len(), 1);
        assert_eq!(snapshot.parameters.len(), 1);
        assert_eq!(snapshot.parameters[0].id, original);
        assert!(!snapshot.parameters[0].parameter.available());
        drop(retained);
        assert!(runtime.inspect().parameters.is_empty());
        assert!(runtime.parameters.borrow().is_empty());
        assert!(matches!(
            runtime.parameter(original.clone(), spec.clone()),
            Err(RuntimeError::Binding(BindingError::DuplicateParameter))
        ));
        let (_parameter, _host) = runtime.parameter(original.next_incarnation()?, spec)?;
        assert_eq!(runtime.inspect().parameters.len(), 1);
        assert_eq!(runtime.next_deadline(), None);
        Ok(())
    }
}
