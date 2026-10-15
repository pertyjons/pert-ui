//! Reading the interface from outside the UI thread, and asking for parts of
//! it rather than all of it.
//!
//! [`InspectionQuery`] selects components by stable key path, incarnation, key
//! or semantics and names which sections of a snapshot it wants;
//! [`Runtime::query`](crate::Runtime::query) answers it on the UI thread. An
//! [`InspectionService`] carries such requests from an application's own
//! tooling thread to the runtime and the owned answers back, so that a
//! debugger, test driver or MCP adapter the application writes never holds a
//! live UI handle. [`docs/contracts.md`] records the contract.
//!
//! [`docs/contracts.md`]: https://github.com/pertyjons/pert-ui/blob/main/docs/contracts.md

use std::{
    cell::RefCell,
    collections::{BTreeMap, VecDeque},
    fmt,
    rc::{Rc, Weak},
    sync::{
        self, Arc, Condvar, Mutex, MutexGuard, PoisonError,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

use crate::{
    Capture, CaptureError, CaptureRequest, ComponentInspection, DebugMode, DebugOverlay,
    InspectValue, InspectionSnapshot, LiveWaker, ParameterInspection, ProfileSnapshot, Runtime,
    RuntimeError, SemanticRole, SourceInspection, ValidationReport, WindowInspection, Workspace,
    WorkspaceError,
};

/// How a query finds components.
///
/// The interface has three kinds of identity and a selector uses each for what
/// it is good for: the addressable key path of
/// [workspace persistence](https://github.com/pertyjons/pert-ui/blob/main/docs/contracts.md),
/// which survives restarts and reordering; the runtime incarnation from an
/// earlier result, which never retargets; and the typed semantics, which find
/// a control the way a user describes it. A selector may match several
/// components, and a result lists every match in incarnation order rather than
/// picking one. A stale incarnation or an unknown path matches nothing, which
/// is an empty result, not an error and not a nearby component.
///
/// ```
/// use pert_ui_runtime::{SemanticRole, Selector};
/// let save = Selector::within(
///     Selector::path(["toolbar"]),
///     Selector::semantic(SemanticRole::Button, Some("Save")),
/// );
/// let every_tab = Selector::children(["main", "tabs"]);
/// assert_ne!(save, every_tab);
/// ```
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Deserialize, serde::Serialize),
    serde(deny_unknown_fields)
)]
pub enum Selector {
    /// The component that introduces the last segment of this addressable
    /// path. Structural wrappers beneath it share its address but never match.
    Path(Vec<String>),
    /// Every addressable child of the component at this path: each component
    /// whose path is this one plus one segment.
    Children(Vec<String>),
    /// Every addressable descendant of the component at this path.
    Descendants(Vec<String>),
    /// One runtime incarnation, [`ComponentInspection::id`]. A replacement
    /// mounted under the same key is a different incarnation and does not
    /// match.
    Id(u64),
    /// Every component with this caller key, wherever it is mounted.
    Key(String),
    /// Every component whose resolved semantics have this role and, when a
    /// name is given, exactly this name (the name a wrapping `Field` gives its
    /// control counts).
    Semantic {
        /// The required [`SemanticRole`].
        role: SemanticRole,
        /// The required name, or `None` for any name.
        name: Option<String>,
    },
    /// The component that owns keyboard focus in each window.
    Focused,
    /// Every component presented in this window incarnation,
    /// [`WindowInspection::id`].
    Window(u64),
    /// The components the second selector matches among the logical
    /// descendants of those the first matches.
    Within(Box<Selector>, Box<Selector>),
}

impl Selector {
    /// The component at this addressable key path.
    pub fn path(segments: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self::Path(segments.into_iter().map(Into::into).collect())
    }
    /// Every addressable child of the component at this path.
    pub fn children(segments: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self::Children(segments.into_iter().map(Into::into).collect())
    }
    /// Every addressable descendant of the component at this path.
    pub fn descendants(segments: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self::Descendants(segments.into_iter().map(Into::into).collect())
    }
    /// One runtime incarnation from an earlier result.
    #[must_use]
    pub const fn id(id: u64) -> Self {
        Self::Id(id)
    }
    /// Every component with this caller key.
    pub fn key(key: impl Into<String>) -> Self {
        Self::Key(key.into())
    }
    /// Every component with this role and, when given, this resolved name.
    pub fn semantic(role: SemanticRole, name: Option<impl Into<String>>) -> Self {
        Self::Semantic {
            role,
            name: name.map(Into::into),
        }
    }
    /// The focused component of each window.
    #[must_use]
    pub const fn focused() -> Self {
        Self::Focused
    }
    /// Every component presented in this window.
    #[must_use]
    pub const fn window(id: u64) -> Self {
        Self::Window(id)
    }
    /// What `inner` matches among the logical descendants of what `ancestor`
    /// matches.
    #[must_use]
    pub fn within(ancestor: Self, inner: Self) -> Self {
        Self::Within(Box::new(ancestor), Box::new(inner))
    }
}

/// Which components around a match a query returns.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum Depth {
    /// The matches only. The default.
    #[default]
    Matches,
    /// The matches and their immediate logical children.
    Children,
    /// The matches and their whole logical subtrees.
    Subtree,
}

/// How much a query reports about each returned component.
///
/// `Component::inspect` runs only at [`Detail::Semantics`] and above, and only
/// for the returned components (plus the immediate parent that names one), so
/// a query costs in proportion to what it returns rather than to the
/// interface.
#[derive(Clone, Copy, Debug, Default, Eq, Ord, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum Detail {
    /// Identity, ancestry, window, key path, visibility, focus and scheduling
    /// state; `details` stays empty.
    Identity,
    /// Identity plus the allocated border box and effective clip, when layout
    /// has produced them. The default.
    #[default]
    Geometry,
    /// Geometry plus the typed semantics.
    Semantics,
    /// Everything [`Runtime::inspect`] reports: box model, properties and
    /// validation hints included.
    Full,
}

/// A request for parts of the interface: which components, at what depth and
/// detail, and which other sections of a snapshot.
///
/// Each section is absent from the [`QueryResult`] unless asked for, so an
/// absent section is distinguishable from an empty one. A query with no
/// selector returns no components.
///
/// ```
/// use pert_ui_runtime::{Depth, Detail, InspectionQuery, Selector};
/// let query = InspectionQuery::components(Selector::path(["mixer", "channel-3"]))
///     .depth(Depth::Children)
///     .detail(Detail::Semantics)
///     .limit(64)
///     .events_after(120);
/// assert_eq!(query.limit_value(), Some(64));
/// ```
#[derive(Clone, Debug, Default, PartialEq)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Deserialize, serde::Serialize),
    serde(deny_unknown_fields)
)]
#[must_use]
pub struct InspectionQuery {
    selector: Option<Selector>,
    depth: Depth,
    detail: Detail,
    limit: Option<usize>,
    parameters: bool,
    events_after: Option<u64>,
    windows: bool,
    sources: bool,
    validation: bool,
    application: bool,
}

impl InspectionQuery {
    /// A query for no components and no other section; add what is wanted.
    pub fn new() -> Self {
        Self::default()
    }
    /// A query for the components `selector` matches.
    pub fn components(selector: Selector) -> Self {
        Self {
            selector: Some(selector),
            ..Self::default()
        }
    }
    /// Selects the components the query returns.
    pub fn select(mut self, selector: Selector) -> Self {
        self.selector = Some(selector);
        self
    }
    /// Returns the matches only, their children or their subtrees.
    pub const fn depth(mut self, depth: Depth) -> Self {
        self.depth = depth;
        self
    }
    /// How much to report per component.
    pub const fn detail(mut self, detail: Detail) -> Self {
        self.detail = detail;
        self
    }
    /// At most this many components, in incarnation order;
    /// [`QueryResult::truncated`] reports when more matched.
    pub const fn limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }
    /// Includes every still-owned registered parameter.
    pub const fn parameters(mut self) -> Self {
        self.parameters = true;
        self
    }
    /// Includes the retained events with a sequence greater than `sequence`,
    /// with the eviction count and the history generation; zero asks for
    /// every retained event.
    pub const fn events_after(mut self, sequence: u64) -> Self {
        self.events_after = Some(sequence);
        self
    }
    /// Includes the native presentation roots.
    pub const fn windows(mut self) -> Self {
        self.windows = true;
        self
    }
    /// Includes the adopted live-data sources.
    pub const fn sources(mut self) -> Self {
        self.sources = true;
        self
    }
    /// Includes the development validation report.
    pub const fn validation(mut self) -> Self {
        self.validation = true;
        self
    }
    /// Includes the application's explicitly exposed properties: what the
    /// provider given to [`Runtime::query_with`] or
    /// [`InspectionService::set_application`] returns, or an empty map.
    pub const fn application(mut self) -> Self {
        self.application = true;
        self
    }
    /// The selector, if any.
    #[must_use]
    pub const fn selector(&self) -> Option<&Selector> {
        self.selector.as_ref()
    }
    /// The requested depth.
    #[must_use]
    pub const fn depth_value(&self) -> Depth {
        self.depth
    }
    /// The requested detail.
    #[must_use]
    pub const fn detail_value(&self) -> Detail {
        self.detail
    }
    /// The component limit, if any.
    #[must_use]
    pub const fn limit_value(&self) -> Option<usize> {
        self.limit
    }
    /// The event cursor, if events were requested.
    #[must_use]
    pub const fn events_after_value(&self) -> Option<u64> {
        self.events_after
    }
    /// Whether parameters were requested.
    #[must_use]
    pub const fn wants_parameters(&self) -> bool {
        self.parameters
    }
    /// Whether windows were requested.
    #[must_use]
    pub const fn wants_windows(&self) -> bool {
        self.windows
    }
    /// Whether live-data sources were requested.
    #[must_use]
    pub const fn wants_sources(&self) -> bool {
        self.sources
    }
    /// Whether the validation report was requested.
    #[must_use]
    pub const fn wants_validation(&self) -> bool {
        self.validation
    }
    /// Whether application properties were requested.
    #[must_use]
    pub const fn wants_application(&self) -> bool {
        self.application
    }
}

/// The retained events after a cursor, with what a poller needs to trust them.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Deserialize, serde::Serialize),
    serde(deny_unknown_fields)
)]
pub struct EventHistory {
    /// Retained events with a sequence greater than the cursor, oldest first.
    pub events: Vec<crate::EventRecord>,
    /// Events evicted or rejected since this history began, cumulative. A
    /// reader that keeps a cursor detects a gap by the first returned sequence
    /// being more than one past its cursor, within one generation; a rise in
    /// this count alone only says the ring wrapped, which may have evicted
    /// events it had already read.
    pub dropped_events: u64,
    /// Advances when the history is cleared or its recording mode changes, so
    /// an empty reply after a reset is distinguishable from nothing new.
    pub generation: u64,
}

/// What a query returned. Sections the query did not ask for are `None`.
///
/// A result describes one flush's state, like a snapshot: `at` is the runtime
/// clock of the capture and `schema_version` the
/// [`InspectionSnapshot::SCHEMA_VERSION`] its records follow.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Deserialize, serde::Serialize),
    serde(deny_unknown_fields)
)]
pub struct QueryResult {
    /// The snapshot schema the records follow.
    pub schema_version: u32,
    /// The runtime clock at capture.
    pub at: Duration,
    /// The returned components in incarnation order, at the requested detail.
    pub components: Vec<ComponentInspection>,
    /// How many components the selector and depth produced before the limit.
    pub matched: usize,
    /// Whether the limit left some of them out.
    pub truncated: bool,
    /// Every still-owned parameter, when requested.
    pub parameters: Option<Vec<ParameterInspection>>,
    /// The events after the cursor, when requested.
    pub events: Option<EventHistory>,
    /// The native presentation roots, when requested.
    pub windows: Option<Vec<WindowInspection>>,
    /// The adopted live-data sources, when requested.
    pub sources: Option<Vec<SourceInspection>>,
    /// The validation report, when requested.
    pub validation: Option<ValidationReport>,
    /// The application's exposed properties, when requested.
    pub application: Option<BTreeMap<String, InspectValue>>,
}

/// A change to what the runtime records or draws for diagnosis.
///
/// These are the only commands an [`InspectionService`] carries, and only
/// when the application enabled them with
/// [`InspectionService::allow_diagnostic_commands`]. None touches a
/// parameter, a signal, a setting the application owns or its project.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum DiagnosticCommand {
    /// [`Runtime::set_debug_mode`].
    SetDebugMode(DebugMode),
    /// [`Runtime::clear_event_history`].
    ClearEventHistory,
    /// [`Runtime::set_debug_overlay`].
    SetOverlay(DebugOverlay),
    /// [`Runtime::set_profiling`].
    SetProfiling(bool),
    /// [`Runtime::clear_profile`].
    ClearProfile,
}

/// Why an [`InspectionClient`] refused a request. Nothing was queued.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum RequestError {
    /// The queue holds its capacity of requests. It empties when the next
    /// flush takes them to answer; dropping a pending reply does not
    /// withdraw its request. Wait for a reply, then ask again.
    #[error("the inspection service is full")]
    Full,
    /// The service was dropped or its runtime shut down; no later request
    /// can succeed.
    #[error("the inspection service has closed")]
    Closed,
    /// A command was sent to a service that has not enabled commands.
    #[error("diagnostic commands are disabled on this inspection service")]
    CommandsDisabled,
}

/// Why waiting for a reply ended without one.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum WaitError {
    /// The timeout passed. The request stays queued and can be waited for
    /// again.
    #[error("no reply within the timeout")]
    Timeout,
    /// The service or its runtime closed before answering, or the request
    /// was dropped while being answered. The request will never be answered.
    #[error("the request was cancelled")]
    Cancelled,
    /// The reply was already taken by an earlier `wait_timeout` or
    /// `try_reply`; a reply is handed out once.
    #[error("the reply was already taken")]
    Taken,
}

/// One answer from the UI thread.
#[derive(Clone, Debug, PartialEq)]
pub struct InspectionReply<T> {
    /// The requested value, captured after the flush that answered.
    pub value: T,
    /// The error that flush returned, if it failed. The value still describes
    /// the state the flush left behind, as [`Runtime::inspect`] would report
    /// it afterwards. The owned cause is safe to move to the requesting thread;
    /// application errors retain their original error object. Format it only at
    /// the application transport boundary when a text diagnostic is needed.
    pub flush_error: Option<RuntimeError>,
}

enum SlotState<T> {
    Waiting,
    Answered(InspectionReply<T>),
    Taken,
    Cancelled,
}

struct Slot<T> {
    state: Mutex<SlotState<T>>,
    ready: Condvar,
}

impl<T> Slot<T> {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            state: Mutex::new(SlotState::Waiting),
            ready: Condvar::new(),
        })
    }
    /// The state only moves forward under the lock, so a poisoned lock still
    /// guards a consistent state.
    fn lock(&self) -> MutexGuard<'_, SlotState<T>> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }
    fn fill(&self, state: SlotState<T>) {
        let mut guard = self.lock();
        if matches!(*guard, SlotState::Waiting) {
            *guard = state;
        }
        drop(guard);
        self.ready.notify_all();
    }
    /// Takes an arrived reply, leaving the slot `Taken`; reports the other
    /// terminal states, and `Waiting` as `Timeout`.
    fn take(guard: &mut SlotState<T>) -> Result<InspectionReply<T>, WaitError> {
        match std::mem::replace(guard, SlotState::Taken) {
            SlotState::Answered(reply) => Ok(reply),
            SlotState::Cancelled => {
                *guard = SlotState::Cancelled;
                Err(WaitError::Cancelled)
            }
            SlotState::Taken => Err(WaitError::Taken),
            SlotState::Waiting => {
                *guard = SlotState::Waiting;
                Err(WaitError::Timeout)
            }
        }
    }
}

/// The UI thread's end of one request. Dropping it unanswered cancels the
/// request, so a batch that is dropped while a provider unwinds, or a queue
/// that closes, releases every waiter. A handle that outlives its batch, a
/// capture waiting for the host, also learns when its service closed.
pub(crate) struct ReplyHandle<T> {
    slot: Arc<Slot<T>>,
    answered: bool,
    closed: Option<Arc<AtomicBool>>,
}

impl<T> ReplyHandle<T> {
    pub(crate) fn new(closed: Option<Arc<AtomicBool>>) -> (Self, Pending<T>) {
        let slot = Slot::new();
        (
            Self {
                slot: slot.clone(),
                answered: false,
                closed,
            },
            Pending { slot },
        )
    }
    /// Whether the service this request came through has closed, so the
    /// request should be cancelled rather than answered.
    pub(crate) fn is_cancelled(&self) -> bool {
        self.closed
            .as_ref()
            .is_some_and(|closed| closed.load(Ordering::Acquire))
    }
    /// Releases the waiter with `value`.
    pub(crate) fn answer(mut self, value: T, flush_error: Option<&RuntimeError>) {
        self.answered = true;
        self.slot.fill(SlotState::Answered(InspectionReply {
            value,
            flush_error: flush_error.cloned(),
        }));
    }
}

impl<T> Drop for ReplyHandle<T> {
    fn drop(&mut self) {
        if !self.answered {
            self.slot.fill(SlotState::Cancelled);
        }
    }
}

/// A reply that has not necessarily arrived yet.
///
/// Returned by every [`InspectionClient`] request. A reply is handed out
/// once: after `wait_timeout` or `try_reply` returned it, later calls report
/// [`WaitError::Taken`]. Dropping the value discards the answer when it comes,
/// without affecting other requests; it does not withdraw the request from
/// the queue. `Send` and `Sync`, so a request may be made on one thread and
/// awaited on another.
#[must_use = "dropping a pending reply discards the answer"]
pub struct Pending<T> {
    slot: Arc<Slot<T>>,
}

impl<T> fmt::Debug for Pending<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let state = match &*self.slot.lock() {
            SlotState::Waiting => "waiting",
            SlotState::Answered(_) => "answered",
            SlotState::Taken => "taken",
            SlotState::Cancelled => "cancelled",
        };
        f.debug_struct("Pending").field("state", &state).finish()
    }
}

impl<T> Pending<T> {
    /// Blocks until the reply arrives or the request is cancelled. Never call
    /// this on the UI thread: the reply comes from that thread's next flush.
    ///
    /// # Errors
    /// [`WaitError::Cancelled`] when the service or runtime closed first, and
    /// [`WaitError::Taken`] when the reply was already taken.
    pub fn wait(self) -> Result<InspectionReply<T>, WaitError> {
        let mut guard = self.slot.lock();
        loop {
            match Slot::take(&mut guard) {
                Err(WaitError::Timeout) => {
                    guard = self
                        .slot
                        .ready
                        .wait(guard)
                        .unwrap_or_else(PoisonError::into_inner);
                }
                outcome => return outcome,
            }
        }
    }

    /// Blocks for at most `timeout`. On [`WaitError::Timeout`] the request
    /// stays queued and this value can be waited for again; the timeout does
    /// not withdraw it. A timeout too large to represent waits without bound.
    ///
    /// # Errors
    /// [`WaitError::Timeout`], [`WaitError::Cancelled`] or [`WaitError::Taken`].
    pub fn wait_timeout(&self, timeout: Duration) -> Result<InspectionReply<T>, WaitError> {
        let started = std::time::Instant::now();
        let mut guard = self.slot.lock();
        loop {
            match Slot::take(&mut guard) {
                Err(WaitError::Timeout) => {
                    let Some(remaining) = timeout.checked_sub(started.elapsed()) else {
                        return Err(WaitError::Timeout);
                    };
                    if remaining.is_zero() {
                        return Err(WaitError::Timeout);
                    }
                    guard = self
                        .slot
                        .ready
                        .wait_timeout(guard, remaining)
                        .unwrap_or_else(PoisonError::into_inner)
                        .0;
                }
                outcome => return outcome,
            }
        }
    }

    /// Takes the reply if it has arrived, without blocking.
    ///
    /// # Errors
    /// [`WaitError::Timeout`] while the request is still queued,
    /// [`WaitError::Cancelled`] once it never will be answered, and
    /// [`WaitError::Taken`] when the reply was already taken.
    pub fn try_reply(&self) -> Result<InspectionReply<T>, WaitError> {
        Slot::take(&mut self.slot.lock())
    }
}

enum Request {
    Snapshot(ReplyHandle<InspectionSnapshot>),
    Query(InspectionQuery, ReplyHandle<QueryResult>),
    Profile(ReplyHandle<ProfileSnapshot>),
    Validation(ReplyHandle<ValidationReport>),
    Workspace(ReplyHandle<Result<Workspace, WorkspaceError>>),
    Capture(CaptureRequest, ReplyHandle<Result<Capture, CaptureError>>),
    Command(DiagnosticCommand, ReplyHandle<()>),
}

impl Request {
    fn is_command(&self) -> bool {
        matches!(self, Self::Command(..))
    }
}

type CaptureSlot = Slot<Result<Capture, CaptureError>>;

struct QueueState {
    items: VecDeque<Request>,
    closed: bool,
    /// Every capture request still waiting for its reply, including admitted
    /// ones whose handle the runtime holds until the host renders them.
    /// Closing cancels them at once, so a waiter never depends on the host
    /// listing or completing jobs again. Answered and dropped requests are
    /// pruned whenever another capture is requested, so the list stays
    /// bounded by the captures actually outstanding.
    captures: Vec<sync::Weak<CaptureSlot>>,
}

struct Shared {
    state: Mutex<QueueState>,
    capacity: usize,
    commands: AtomicBool,
    /// Mirrors `QueueState::closed` for handles that left the queue.
    closed: Arc<AtomicBool>,
}

impl Shared {
    /// The queue only pushes, pops and takes under the lock, none of which can
    /// panic halfway, so a poisoned lock still guards consistent state.
    fn lock(&self) -> MutexGuard<'_, QueueState> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }
    fn close(&self) {
        let (cancelled, captures) = {
            let mut state = self.lock();
            state.closed = true;
            self.closed.store(true, Ordering::Release);
            (
                std::mem::take(&mut state.items),
                std::mem::take(&mut state.captures),
            )
        };
        // Each unanswered handle cancels its waiter as it drops.
        drop(cancelled);
        // An admitted capture's handle stays with the runtime until the host
        // lists or completes jobs; its waiter is released here instead. The
        // runtime's later answer finds the slot settled and changes nothing.
        for slot in captures.iter().filter_map(sync::Weak::upgrade) {
            slot.fill(SlotState::Cancelled);
        }
    }
}

/// A `Send` handle that carries inspection requests to the UI thread.
///
/// Created by [`InspectionService::client`]; clones share one queue. Each
/// request takes a short lock and, once accepted, wakes the host's event loop
/// the way a [`UiSender`](crate::UiSender) does, so it belongs on ordinary
/// threads: a tooling adapter, a test driver, an I/O task. Never request from
/// a time-critical producer callback.
///
/// Requests from one client are answered in the order they were accepted, at
/// the end of the runtime's next flush; see [`InspectionService`].
#[derive(Clone)]
pub struct InspectionClient {
    shared: Arc<Shared>,
    waker: LiveWaker,
}

impl fmt::Debug for InspectionClient {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("InspectionClient")
            .field("capacity", &self.shared.capacity)
            .field("closed", &self.is_closed())
            .finish()
    }
}

impl InspectionClient {
    fn enqueue(&self, request: Request) -> Result<(), RequestError> {
        self.enqueue_tracked(request, None)
    }

    /// Queues `request`; a capture also registers its reply slot, under the
    /// same lock, so that closing can cancel it after admission.
    fn enqueue_tracked(
        &self,
        request: Request,
        capture: Option<sync::Weak<CaptureSlot>>,
    ) -> Result<(), RequestError> {
        {
            let mut state = self.shared.lock();
            if state.closed {
                return Err(RequestError::Closed);
            }
            if request.is_command() && !self.shared.commands.load(Ordering::Acquire) {
                return Err(RequestError::CommandsDisabled);
            }
            if state.items.len() >= self.shared.capacity {
                return Err(RequestError::Full);
            }
            state.items.push_back(request);
            if let Some(slot) = capture {
                state.captures.retain(|slot| {
                    slot.upgrade()
                        .is_some_and(|slot| matches!(*slot.lock(), SlotState::Waiting))
                });
                state.captures.push(slot);
            }
        }
        // A failed wake means the event loop has exited; the runtime closes the
        // service when it shuts down, which the next request reports.
        let _ = self.waker.notify();
        Ok(())
    }

    /// Asks for a whole [`InspectionSnapshot`], as [`Runtime::inspect_with`]
    /// captures it with the service's application provider.
    ///
    /// # Errors
    /// [`RequestError::Full`] or [`RequestError::Closed`].
    pub fn snapshot(&self) -> Result<Pending<InspectionSnapshot>, RequestError> {
        let (handle, pending) = ReplyHandle::new(Some(self.shared.closed.clone()));
        self.enqueue(Request::Snapshot(handle))?;
        Ok(pending)
    }

    /// Asks for the parts of the interface a query selects, as
    /// [`Runtime::query_with`] answers it.
    ///
    /// # Errors
    /// [`RequestError::Full`] or [`RequestError::Closed`].
    pub fn query(&self, query: InspectionQuery) -> Result<Pending<QueryResult>, RequestError> {
        let (handle, pending) = ReplyHandle::new(Some(self.shared.closed.clone()));
        self.enqueue(Request::Query(query, handle))?;
        Ok(pending)
    }

    /// Asks for [`Runtime::profile`].
    ///
    /// # Errors
    /// [`RequestError::Full`] or [`RequestError::Closed`].
    pub fn profile(&self) -> Result<Pending<ProfileSnapshot>, RequestError> {
        let (handle, pending) = ReplyHandle::new(Some(self.shared.closed.clone()));
        self.enqueue(Request::Profile(handle))?;
        Ok(pending)
    }

    /// Asks for [`Runtime::validation_report`].
    ///
    /// # Errors
    /// [`RequestError::Full`] or [`RequestError::Closed`].
    pub fn validation(&self) -> Result<Pending<ValidationReport>, RequestError> {
        let (handle, pending) = ReplyHandle::new(Some(self.shared.closed.clone()));
        self.enqueue(Request::Validation(handle))?;
        Ok(pending)
    }

    /// Asks for [`Runtime::capture_workspace`]; the reply carries its result,
    /// since a capture can be refused.
    ///
    /// # Errors
    /// [`RequestError::Full`] or [`RequestError::Closed`].
    pub fn workspace(&self) -> Result<Pending<Result<Workspace, WorkspaceError>>, RequestError> {
        let (handle, pending) = ReplyHandle::new(Some(self.shared.closed.clone()));
        self.enqueue(Request::Workspace(handle))?;
        Ok(pending)
    }

    /// Asks for a screenshot, as [`Runtime::request_capture`] does. The
    /// reply arrives once the host has rendered the frame, which is after the
    /// flush that admitted the request, or at once with the
    /// [`CaptureError`] that refused it. Closing the service cancels an
    /// admitted capture too, releasing its waiter at once rather than when
    /// the host next lists or completes capture jobs.
    ///
    /// # Errors
    /// [`RequestError::Full`] or [`RequestError::Closed`].
    pub fn capture(
        &self,
        request: CaptureRequest,
    ) -> Result<Pending<Result<Capture, CaptureError>>, RequestError> {
        let (handle, pending) = ReplyHandle::new(Some(self.shared.closed.clone()));
        let slot = Arc::downgrade(&pending.slot);
        self.enqueue_tracked(Request::Capture(request, handle), Some(slot))?;
        Ok(pending)
    }

    /// Asks the UI thread to apply a diagnostic command. The reply arrives
    /// once it was applied, before any read a later request gets.
    ///
    /// Permission is checked when the command is accepted and again when it
    /// would run: a command still queued when the application disables
    /// commands is not applied, and waiting for it reports
    /// [`WaitError::Cancelled`].
    ///
    /// # Errors
    /// [`RequestError::CommandsDisabled`] unless the service enabled commands,
    /// [`RequestError::Full`] or [`RequestError::Closed`].
    pub fn command(&self, command: DiagnosticCommand) -> Result<Pending<()>, RequestError> {
        let (handle, pending) = ReplyHandle::new(Some(self.shared.closed.clone()));
        self.enqueue(Request::Command(command, handle))?;
        Ok(pending)
    }

    /// Whether the service has closed, so no request can succeed.
    pub fn is_closed(&self) -> bool {
        self.shared.lock().closed
    }

    /// The number of accepted requests still in the queue. Requests being
    /// answered right now have left it.
    pub fn pending(&self) -> usize {
        self.shared.lock().items.len()
    }
}

type Provider = Box<dyn Fn() -> BTreeMap<String, InspectValue>>;

/// The UI-thread state of one service; the runtime keeps a weak reference.
pub(crate) struct ServiceInner {
    shared: Arc<Shared>,
    application: RefCell<Option<Provider>>,
}

impl Drop for ServiceInner {
    fn drop(&mut self) {
        self.shared.close();
    }
}

impl ServiceInner {
    /// Takes every request queued now; later ones wait for the next answer.
    fn take_batch(&self) -> Vec<Request> {
        let mut state = self.shared.lock();
        state.items.drain(..).collect()
    }
    /// Whether requests are waiting.
    pub(crate) fn has_pending(&self) -> bool {
        !self.shared.lock().items.is_empty()
    }
    /// Refuses further requests and cancels waiting ones.
    pub(crate) fn close(&self) {
        self.shared.close();
    }
}

/// Answers every request queued at every service when the call begins.
///
/// The batches are taken first, across all services, so a request made while
/// answering, by any thread or by an application provider, waits for the
/// next flush. Every command is then applied before any read is answered,
/// whichever service it came from; among commands, and among reads, each
/// service's order is kept. A handle dropped unanswered, for example while a
/// provider unwinds, cancels its waiter.
pub(crate) fn answer(
    services: &[Rc<ServiceInner>],
    runtime: &mut Runtime,
    flush_error: Option<&RuntimeError>,
) {
    let batches: Vec<(Rc<ServiceInner>, Vec<Request>)> = services
        .iter()
        .map(|service| (service.clone(), service.take_batch()))
        .collect();
    let mut reads = Vec::new();
    for (service, batch) in batches {
        for request in batch {
            match request {
                // Permission is checked again when the command would run: one
                // queued before the application disabled commands is dropped
                // unapplied, which cancels its waiter.
                Request::Command(_, handle) if !service.shared.commands.load(Ordering::Acquire) => {
                    drop(handle);
                }
                Request::Command(command, handle) => {
                    match command {
                        DiagnosticCommand::SetDebugMode(mode) => runtime.set_debug_mode(mode),
                        DiagnosticCommand::ClearEventHistory => runtime.clear_event_history(),
                        DiagnosticCommand::SetOverlay(overlay) => {
                            runtime.set_debug_overlay(overlay)
                        }
                        // Profiling refuses only a closed runtime; a closed
                        // runtime has already cancelled this request.
                        DiagnosticCommand::SetProfiling(enabled) => {
                            let _ = runtime.set_profiling(enabled);
                        }
                        DiagnosticCommand::ClearProfile => {
                            let _ = runtime.clear_profile();
                        }
                    }
                    handle.answer((), flush_error);
                }
                read => reads.push((service.clone(), read)),
            }
        }
    }
    for (service, request) in reads {
        let provider = service.application.borrow();
        let application = || {
            provider
                .as_ref()
                .map_or_else(BTreeMap::new, |provide| provide())
        };
        match request {
            Request::Snapshot(handle) => {
                handle.answer(runtime.inspect_with(application), flush_error);
            }
            Request::Query(query, handle) => {
                handle.answer(runtime.query_with(&query, application), flush_error);
            }
            Request::Profile(handle) => handle.answer(runtime.profile(), flush_error),
            Request::Validation(handle) => {
                handle.answer(runtime.validation_report(), flush_error);
            }
            Request::Workspace(handle) => {
                handle.answer(runtime.capture_workspace(), flush_error);
            }
            Request::Capture(request, handle) => {
                runtime.admit_capture(request, handle, flush_error)
            }
            Request::Command(..) => {}
        }
    }
}

/// The UI-thread end of an inspection channel from other threads.
///
/// Created by [`Runtime::inspection_service`]. The application keeps it for
/// as long as tools may connect and hands [`InspectionClient`]s to the threads
/// that ask. At the end of every outermost flush, after tasks, mailboxes,
/// component updates and layout, the runtime answers the requests that were
/// queued when answering began, so a reply describes the state that flush
/// presents. A request that arrives while the runtime is idle wakes the host
/// for that one flush; nothing polls, and no thread, listener or timer is
/// started. A flush that fails still answers, from the state it left behind,
/// with its owned runtime cause in [`InspectionReply::flush_error`]. A host that
/// withholds flushes, such as the native `App` while a window is minimized,
/// answers from the last flushed state with
/// [`Runtime::service_inspection`], so a request is never stranded.
///
/// Dropping the service, calling [`Self::close`] or shutting the runtime down
/// closes it: queued requests resolve as [`WaitError::Cancelled`] and later
/// ones are refused as [`RequestError::Closed`]. The service is UI-local and
/// not `Send`.
///
/// A tool thread holds only the `Send` client and the owned replies. It
/// handles every [`WaitError`]: a timeout leaves the request queued, so it may
/// wait again.
///
/// ```
/// use pert_ui_runtime::{
///     Capacity, DebugOverlay, Depth, Detail, DiagnosticCommand, Element, InspectionQuery, Label,
///     Runtime, Selector, WaitError,
/// };
/// use std::time::Duration;
///
/// type ToolError = Box<dyn std::error::Error + Send + Sync>;
///
/// let mut ui = Runtime::new();
/// ui.mount_tree(Element::new("title", Label::new("Hello")))?;
/// let service = ui.inspection_service(Capacity::new(8)?)?;
/// service.set_application(|| Default::default()); // optional, as `inspect_with` takes one
/// service.allow_diagnostic_commands(true); // read-only unless enabled
/// let client = service.client();
///
/// let tool = std::thread::spawn(move || -> Result<usize, ToolError> {
///     let query = InspectionQuery::components(Selector::key("title"))
///         .depth(Depth::Children)
///         .detail(Detail::Semantics);
///     // Refused as Full, Closed or, for a command, CommandsDisabled.
///     let pending = client.query(query)?;
///     let found = loop {
///         match pending.wait_timeout(Duration::from_secs(2)) {
///             Ok(reply) => break reply.value.components.len(),
///             Err(WaitError::Timeout) => continue, // still queued; wait again or drop it
///             Err(WaitError::Cancelled) => return Err("the service closed".into()),
///             Err(WaitError::Taken) => return Err("the reply was already taken".into()),
///             Err(error) => return Err(error.into()),
///         }
///     };
///     client
///         .command(DiagnosticCommand::SetOverlay(DebugOverlay::Boxes))?
///         .wait()?;
///     Ok(found)
/// });
/// // A request wakes the host's event loop for the flush that answers it;
/// // here the flushes are driven by hand.
/// while !tool.is_finished() {
///     ui.flush(Duration::ZERO)?;
///     std::thread::yield_now();
/// }
/// let found = tool.join().expect("the tool thread does not panic")?;
/// assert_eq!(found, 1);
/// # Ok::<(), ToolError>(())
/// ```
#[must_use = "dropping the service closes it"]
pub struct InspectionService {
    inner: Rc<ServiceInner>,
    waker: LiveWaker,
}

impl fmt::Debug for InspectionService {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("InspectionService")
            .field("capacity", &self.inner.shared.capacity)
            .field("pending", &self.pending())
            .field("commands", &self.commands_allowed())
            .finish()
    }
}

impl InspectionService {
    pub(crate) fn new(capacity: crate::Capacity, waker: LiveWaker) -> (Self, Weak<ServiceInner>) {
        let inner = Rc::new(ServiceInner {
            shared: Arc::new(Shared {
                state: Mutex::new(QueueState {
                    items: VecDeque::new(),
                    closed: false,
                    captures: Vec::new(),
                }),
                capacity: capacity.get(),
                commands: AtomicBool::new(false),
                closed: Arc::new(AtomicBool::new(false)),
            }),
            application: RefCell::new(None),
        });
        let weak = Rc::downgrade(&inner);
        (Self { inner, waker }, weak)
    }

    /// Returns a new `Send` handle for this service's queue.
    pub fn client(&self) -> InspectionClient {
        InspectionClient {
            shared: Arc::clone(&self.inner.shared),
            waker: self.waker.clone(),
        }
    }

    /// Installs the application-state provider that snapshots and queries
    /// asking for `application` use, as [`Runtime::inspect_with`] takes one.
    /// It runs on the UI thread while answering, with signal and parameter
    /// writes forbidden; report only what the connected tool should see.
    pub fn set_application(&self, provider: impl Fn() -> BTreeMap<String, InspectValue> + 'static) {
        *self.inner.application.borrow_mut() = Some(Box::new(provider));
    }

    /// Whether clients may send [`DiagnosticCommand`]s. Off by default: a
    /// service is read-only until the application decides otherwise.
    /// Disabling takes effect for commands already queued too: they are
    /// cancelled rather than applied at the next flush.
    pub fn allow_diagnostic_commands(&self, allowed: bool) {
        self.inner.shared.commands.store(allowed, Ordering::Release);
    }

    /// Whether diagnostic commands are enabled.
    pub fn commands_allowed(&self) -> bool {
        self.inner.shared.commands.load(Ordering::Acquire)
    }

    /// The number of accepted requests still in the queue; see
    /// [`InspectionClient::pending`].
    pub fn pending(&self) -> usize {
        self.inner.shared.lock().items.len()
    }

    /// The most requests the queue holds before a client reports full. A
    /// request leaves the queue when answering begins, not when its reply is
    /// dropped, so a client that hit `Full` waits for a flush.
    pub fn capacity(&self) -> usize {
        self.inner.shared.capacity
    }

    /// Closes the service now, as dropping it would: clients report
    /// [`RequestError::Closed`] and waiting requests are cancelled.
    pub fn close(&self) {
        self.inner.close();
    }
}
