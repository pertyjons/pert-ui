pub(crate) mod application;
mod capture;
pub(crate) mod commands;
mod compose;
mod content_search;
mod drag;
pub(crate) mod fonts;
pub(crate) mod held;
mod inspect;
pub(crate) mod native_windows;
pub(crate) mod notify;
#[cfg(debug_assertions)]
mod overlay;
mod pointer;
pub(crate) mod presentation;
pub(crate) mod reveal;
pub(crate) mod settings;
mod surfaces;
pub(crate) mod tasks;
pub(crate) mod theme;
mod tree;
mod workspace;

use std::{
    cell::{Cell, RefCell},
    collections::BTreeMap,
    rc::{Rc, Weak},
    time::Duration,
};

use crate::signal::{ReadSignal, ReceiptStatus, Signal, Subscription};
use capture::PendingCapture;

/// Fallback interval when a window has no supplied display rate, shared by
/// [`UpdateContext::animate`] and polled live-data sources.
pub(crate) const PRESENTATION_INTERVAL: Duration = Duration::from_nanos(16_666_667);

/// Work a component must refresh. Layout invalidation also requires paint.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum Invalidation {
    /// Only presentation data changed; geometry can be reused.
    Paint,
    /// Measurements/placement and presentation may have changed.
    Layout,
}

/// The component where a failing flush or paint first failed, as
/// [`Runtime::last_failure`] reports it.
///
/// The error itself is the one the flush or paint returned; this says where it
/// arose, for a report a developer can act on.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct FailureSite {
    /// The keys from the root to the component the failure belongs to, the
    /// root's first: the component whose callback failed, or for
    /// [`FailurePhase::Reconcile`] the component whose requested children were
    /// rejected. Every key names a mounted component.
    pub path: Vec<String>,
    /// What failed for that component.
    pub phase: FailurePhase,
    /// For [`FailurePhase::Reconcile`], the keys from the requested children
    /// down to the rejected description: the duplicated key, the element
    /// whose type changed or whose description failed, the parent of an empty
    /// key, or a surviving child that manages its own children but was given
    /// some. These descriptions need not be mounted. Empty for the other
    /// phases, and for an error that names no element, such as an owner that
    /// cannot take children.
    pub description: Vec<String>,
}

/// What a [`FailureSite`] says failed: one of the component's callbacks, or
/// the runtime's reconciliation of the children it requested.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum FailurePhase {
    /// [`Component::update`].
    Update,
    /// [`Component::measure`] or [`Component::measure_limits`].
    Measure,
    /// [`Component::arrange`].
    Arrange,
    /// [`Component::paint`] or [`Component::paint_foreground`].
    Paint,
    /// The children the component requested through
    /// [`UpdateContext::request_children`], as [`crate::Dynamic`],
    /// [`crate::Show`] and [`crate::Each`] do, were rejected before they replaced the current ones: an
    /// empty or duplicated key, a changed component type under a kept key,
    /// or an error from [`Component::check_description`]. A panic is not
    /// reported here: it resumes unwinding.
    Reconcile,
}

impl std::fmt::Display for FailureSite {
    /// Writes the phase and the key path, as in `update of root/form/name`,
    /// followed for a rejected description by its keys, as in
    /// `reconciliation of root/list at items/row-3`.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let phase = match self.phase {
            FailurePhase::Update => "update",
            FailurePhase::Measure => "measurement",
            FailurePhase::Arrange => "arrangement",
            FailurePhase::Paint => "painting",
            FailurePhase::Reconcile => "reconciliation",
        };
        write!(f, "{phase} of {}", self.path.join("/"))?;
        if !self.description.is_empty() {
            write!(f, " at {}", self.description.join("/"))?;
        }
        Ok(())
    }
}

/// Opaque identity of one mounted component in one runtime.
///
/// Clones refer to the same incarnation. Unmounted handles never address a later
/// component, even if it uses the same key. Handles are UI-local, not `Send`/`Sync`,
/// and must not be persisted. Keeping a handle alive does not keep a mount alive.
#[derive(Clone)]
pub struct WidgetId(pub(crate) Rc<Node>);

impl WidgetId {
    pub(crate) fn incarnation(&self) -> u64 {
        self.0.serial
    }
}

impl PartialEq for WidgetId {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}
impl Eq for WidgetId {}
impl std::hash::Hash for WidgetId {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        std::ptr::hash(Rc::as_ptr(&self.0), state);
    }
}
impl std::fmt::Debug for WidgetId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WidgetId")
            .field("serial", &self.0.serial)
            .field("mounted", &self.0.mounted.get())
            .finish_non_exhaustive()
    }
}

/// Failure in UI-local state or component scheduling, the error of component
/// and runtime callbacks.
///
/// Leaf errors, such as [`GeometryError`](crate::GeometryError),
/// [`BindingError`](crate::BindingError) or [`ChoiceError`](crate::ChoiceError),
/// convert into it with `?`, and each has exactly one form here. An
/// application's own failure is [`Self::Application`]. Higher-level errors
/// ([`DrawError`](crate::DrawError), [`WorkspaceError`](crate::WorkspaceError),
/// [`WindowError`](crate::WindowError) and the facade's application error) wrap
/// it; it never wraps them structurally. The one exception is at the callback
/// boundary: `?` in an application callback converts a
/// [`WorkspaceError`](crate::WorkspaceError) or
/// [`CaptureError`](crate::CaptureError) returned by a runtime method, or the
/// [`WindowError`](crate::WindowError) a
/// [`PresentationReceipt`](crate::PresentationReceipt) reports in
/// [`PresentationStatus::Failed`](crate::PresentationStatus::Failed), into
/// [`Self::Application`], holding the whole error for downcasting, except that
/// one holding a runtime failure in its `Runtime` variant becomes that failure.
/// Through the facade such a callback failure is `AppError::Runtime`; code
/// that returns `AppError` directly, such as an `App` initializer, converts
/// these errors with `.map_err(RuntimeError::from)?`. The set grows with the
/// runtime, so matches need a wildcard arm. `docs/contracts.md` records this
/// error policy.
///
/// ```
/// # use pert_ui_runtime as pert_ui;
/// use pert_ui::{Choice, ChoiceError, RadioGroup, Runtime, RuntimeError};
/// fn group(ui: &Runtime) -> Result<RadioGroup<u8>, RuntimeError> {
///     let value = ui.signal(1);
///     Ok(RadioGroup::new(value, [Choice::new(1, "One"), Choice::new(1, "Also one")])?)
/// }
/// let ui = Runtime::new();
/// assert_eq!(
///     group(&ui).err(),
///     Some(RuntimeError::Choice(ChoiceError::DuplicateValue { first: 0, second: 1 }))
/// );
/// ```
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum RuntimeError {
    /// Presentation ceilings must be finite and between
    /// [`PresentationRate::MIN`](crate::PresentationRate::MIN) and
    /// [`PresentationRate::MAX`](crate::PresentationRate::MAX), 1 and 1000 Hz.
    #[error(
        "presentation rate must be finite and between {} and {} Hz",
        crate::PresentationRate::MIN.hertz(),
        crate::PresentationRate::MAX.hertz()
    )]
    InvalidPresentationRate,
    /// A parameter binding, its edit protocol or its host delivery refused the
    /// operation.
    #[error(transparent)]
    Binding(#[from] crate::BindingError),
    /// An application's own failure, returned from a callback through the
    /// library. The library never produces it. A boxed
    /// `Box<dyn Error + Send + Sync>` becomes one with
    /// [`ApplicationError::from`](crate::ApplicationError::from), which keeps
    /// its source chain; `RuntimeError` has no conversion from the box itself.
    #[error(transparent)]
    Application(#[from] crate::ApplicationError),
    /// The bounded application operation queue is full; the operation was refused.
    #[error("application operation queue is full")]
    TaskQueueFull,
    /// [`Runtime::MAX_BACKGROUND_TASKS`] background tasks wait for their
    /// results; [`Runtime::spawn`] started nothing.
    #[error("too many background tasks are waiting")]
    BackgroundFull,
    /// The operating system could not start a background thread for
    /// [`Runtime::spawn`]; nothing was started.
    #[error("a background thread could not start: {0}")]
    BackgroundSpawn(std::io::ErrorKind),
    /// Work started with [`Runtime::spawn`] panicked, so it has no result to
    /// deliver.
    #[error("background work panicked")]
    BackgroundPanicked,
    /// [`Runtime::MAX_INTERVALS`] interval timers run;
    /// [`Runtime::start_interval`] started nothing.
    #[error("too many interval timers are running")]
    IntervalsFull,
    /// Command registration requires a nonblank human-readable name.
    #[error("command name is empty")]
    EmptyCommandName,
    /// Two aliases/bindings match ambiguously in the same scope and phase; the
    /// chord is the one that conflicts. A held-key registration reports its
    /// conflicting key with the registration's modifiers.
    #[error("shortcut {0} is ambiguous in its scope")]
    ShortcutConflict(crate::KeyChord),
    /// The view's description forbids moving it to its own presentation root.
    #[error("view is not detachable")]
    NotDetachable,
    /// A command identity is foreign or no longer registered.
    #[error("command is no longer registered")]
    StaleCommand,
    /// A captured command target is hidden, removed or moved to another window.
    #[error("command target is no longer available")]
    StaleCommandTarget,
    /// The native presentation request queue holds
    /// [`Runtime::MAX_WINDOW_REQUESTS`] requests; the request was refused.
    #[error("native window request queue is full")]
    WindowRequestFull,
    /// The queued reveal request queue holds [`Runtime::MAX_REVEAL_REQUESTS`]
    /// requests; the request was refused.
    #[error("reveal request queue is full")]
    RevealQueueFull,
    /// The queued command invocation queue holds
    /// [`Runtime::MAX_QUEUED_COMMANDS`] invocations; the invocation was refused.
    #[error("command invocation queue is full")]
    CommandQueueFull,
    /// The view already occupies its own native presentation root.
    #[error("view is already detached")]
    AlreadyDetached,
    /// The requested root is not a detached view.
    #[error("window is not detached")]
    NotDetached,
    /// A prepared movement no longer matches the view presentation.
    #[error("presentation changed while native resources were prepared")]
    StalePresentation,
    /// A floating window requires a nonblank human-readable title, independently of its key.
    #[error("floating window title must not be empty")]
    EmptyWindowTitle,
    /// A modal chain already occupies the requested scope; the new dialog closes
    /// with a failure outcome and the existing chain remains active.
    #[error("modal scope already occupied")]
    ModalConflict,
    /// An input/focus target is blocked by an active modal surface.
    #[error("target is blocked by a modal surface")]
    ModalBlocked,
    /// A dialog is already open or its previous outcome has not been taken.
    #[error("dialog is open or has an unread outcome")]
    DialogBusy,
    /// A `DraftEditorController` is already used by another mounted
    /// [`DraftEditor`](crate::DraftEditor); the later mount fails its updates
    /// until the earlier one unmounts.
    #[error("draft editor controller is used by another mounted editor")]
    DraftEditorBusy,
    /// A notification queue has reached its documented visible plus pending capacity.
    #[error("notification queue is full")]
    ToastFull,

    /// Numeric input configuration or an external numeric value was rejected.
    #[error(transparent)]
    NumberInput(#[from] crate::NumberInputError),
    /// Text validation or composition input was rejected.
    #[error(transparent)]
    TextEdit(#[from] crate::TextEditError),
    /// A clipboard operation failed without deleting source text.
    #[error(transparent)]
    Clipboard(#[from] crate::ClipboardError),
    /// A parameter specification rejected a canonical value.
    #[error(transparent)]
    Parameter(#[from] crate::ParameterError),
    /// A parameter identity could not be constructed or advanced.
    #[error(transparent)]
    ParameterId(#[from] crate::ParameterIdError),
    /// A position could not become a normalized value.
    #[error(transparent)]
    NormalizedValue(#[from] crate::NormalizedValueError),
    /// A live-data bridge could not be configured or restarted.
    #[error(transparent)]
    Live(#[from] crate::LiveError),
    /// A choice or menu list is invalid, such as one with duplicate values or an
    /// empty action key.
    #[error(transparent)]
    Choice(#[from] crate::ChoiceError),
    /// A coordinate, extent, scale or transform was invalid.
    #[error(transparent)]
    Geometry(#[from] crate::GeometryError),
    /// A handle belongs to a different runtime.
    #[error("handle belongs to another runtime")]
    ForeignRuntime,
    /// A component has been unmounted.
    #[error("component is not mounted")]
    Unmounted,
    /// The component is hidden and cannot receive an editing event.
    #[error("component is hidden")]
    Hidden,
    /// A component key, the one held here, is already mounted under the same
    /// parent.
    #[error("component key `{0}` is already in use")]
    DuplicateKey(String),
    /// A key must contain a non-whitespace character.
    #[error("component key must not be empty")]
    EmptyKey,
    /// A key or address lookup below an ancestor matched more than one
    /// descendant, because keys are unique only among siblings; see
    /// [`Runtime::find_descendant`] and [`Runtime::find`]. It holds the key, or
    /// for an address lookup the address with its segments joined by `/`.
    #[error("several descendants share the key `{0}`")]
    AmbiguousKey(String),
    /// State writes during component evaluation are forbidden.
    #[error("state cannot be written during component update")]
    WriteDuringUpdate,
    /// State writes while recording drawing commands are forbidden.
    #[error("state cannot be written during paint")]
    WriteDuringPaint,
    /// State writes from an inspection provider are forbidden.
    #[error("state cannot be written during inspection")]
    WriteDuringInspection,
    /// State writes from a parameter delivery callback are forbidden; defer UI
    /// work to the next ordinary application event.
    #[error("state cannot be written during parameter delivery")]
    WriteDuringDelivery,
    /// A user-defined clone/equality operation reentered a borrowed signal.
    #[error("signal is already borrowed")]
    BorrowConflict,
    /// The owning runtime has been dropped.
    #[error("runtime is closed")]
    Closed,
    /// Supplied elapsed time moved backwards.
    #[error("runtime clock must be monotonic")]
    TimeReversed,
    /// A deadline or identity counter cannot advance.
    #[error("runtime counter or deadline exhausted")]
    Exhausted,
    /// One flush and its internal follow-ups exceeded [`Runtime::MAX_FLUSH_WAVES`].
    /// Pending updates/focus work remain available for explicit repair and retry.
    #[error("component updates did not settle within the flush wave limit")]
    UpdateLimit,
    /// A custom wake delay must be strictly positive.
    #[error("wake delay must be positive")]
    ZeroDelay,
    /// A user gesture owns the state the write addresses until it ends, such
    /// as a column width while its header edge is being dragged. Nothing was
    /// changed; retry after the gesture ends.
    #[error("a user gesture owns this state until it ends")]
    GestureInProgress,
    /// State writes during measurement or placement are forbidden.
    #[error("state cannot be written during layout")]
    WriteDuringLayout,
    /// Custom layout returned invalid geometry or child placement.
    #[error(transparent)]
    Layout(#[from] crate::LayoutError),
    /// A surviving child key, the one held here, cannot change its concrete
    /// component type.
    #[error("retained child `{0}` changed its component type")]
    ChildTypeChanged(String),
    /// Bundled-font text preparation failed.
    #[error(transparent)]
    Text(#[from] crate::TextError),
    /// Font registration, identity or icon mapping failed.
    #[error(transparent)]
    Font(#[from] crate::FontError),
    /// Capture requires a held button and cannot steal another component's capture.
    #[error("pointer capture is unavailable")]
    CaptureUnavailable,
    /// A drag session is already active or requested, or the source permits no
    /// operation.
    #[error("a drag session cannot start now")]
    DragUnavailable,
    /// A drop target accepted an operation its source does not permit.
    #[error("drop target accepted an operation the source does not permit")]
    DropNotPermitted,
    /// A retained subtree cannot move to the requested owner: the move would leave
    /// its native presentation root, the subtree contains a detached view, or the
    /// component is a promoted surface root anchored to its current owner.
    #[error("component cannot move to the requested owner")]
    NotMovable,
}

impl From<crate::EditError> for RuntimeError {
    /// The binding form, [`RuntimeError::Binding`] holding [`BindingError::Edit`](crate::BindingError::Edit).
    fn from(error: crate::EditError) -> Self {
        Self::Binding(error.into())
    }
}
impl From<crate::DeliveryError> for RuntimeError {
    /// The binding form, [`RuntimeError::Binding`] holding [`BindingError::Delivery`](crate::BindingError::Delivery).
    fn from(error: crate::DeliveryError) -> Self {
        Self::Binding(error.into())
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Phase {
    Idle,
    Update,
    Paint,
    Inspect,
    Layout,
    Delivery,
}

pub(crate) struct Shared {
    /// Where the latest failing flush or paint first failed in a component.
    failure: RefCell<Option<FailureSite>>,
    tasks: RefCell<std::collections::VecDeque<tasks::Queued>>,
    /// Set while deferred tasks are delivered, so a flush nested inside a task
    /// (an explicit flush, or one a reveal settles with) leaves the remaining
    /// and newly queued tasks to the delivering loop and the next flush.
    task_dispatching: Cell<bool>,
    pub(crate) computations: RefCell<BTreeMap<usize, Weak<dyn crate::signal::Computation>>>,
    application_modal: Cell<Option<u64>>,
    command_dispatching: Cell<bool>,
    command_requests: RefCell<Vec<commands::QueuedCommand>>,
    reveal_dispatching: Cell<bool>,
    reveal_requests: RefCell<std::collections::VecDeque<reveal::QueuedReveal>>,
    presentation_requests: RefCell<Vec<native_windows::PresentationRequest>>,
    windows: RefCell<Vec<Weak<native_windows::WindowShared>>>,
    modifiers: Cell<crate::Modifiers>,
    zoom: Cell<settings::UiZoom>,
    shapes: Rc<RefCell<crate::shapes::ShapeCache>>,
    theme: RefCell<theme::State>,
    font_subscribers: RefCell<fonts::FontSubscribers>,
    requested_removals: RefCell<BTreeMap<u64, Weak<Node>>>,
    requested_moves: RefCell<Vec<RequestedMove>>,
    requested_children: RefCell<Vec<(Weak<Node>, Vec<crate::Element>)>>,
    requested_text_commands: RefCell<Vec<(Weak<Node>, crate::TextCommand)>>,
    /// A source's `start_drag`, until the runtime adopts it after the callback.
    drag_request: RefCell<Option<drag::DragRequest>>,
    /// Source of the requested or active drag session.
    drag_source: Cell<Option<u64>>,
    /// The drag source lost its input context; the session cancels at the next
    /// opportunity with runtime access.
    drag_lost: Cell<bool>,
    /// Prevents terminal callbacks from admitting another drag during teardown.
    shutting_down: Cell<bool>,
    history: RefCell<crate::diagnostics::History>,
    overlay: Cell<crate::DebugOverlay>,
    motion: Cell<settings::MotionPreference>,
    text_rendering: Cell<settings::TextRenderingMode>,
    /// Observable views of the presentation settings, created on request.
    settings_signals: settings::SettingsSignals,
    /// Components whose last update read the motion preference.
    motion_subscribers: RefCell<Vec<Weak<Node>>>,
    /// The installed table of library strings.
    pub(crate) strings: RefCell<crate::UiStrings>,
    pub(crate) profiler: crate::profiling::Profiler,
    phase: Cell<Phase>,
    closed: Cell<bool>,
    pub(crate) now: Cell<Duration>,
    /// When the next component a continuous zoom stretched is prepared again;
    /// `None` while nothing is stretched. See [`VIEW_SCALE_SETTLE`].
    scale_settle: Cell<Option<Duration>>,
    /// Components drawn from coverage prepared at an earlier view scale, each at
    /// most once, in any window.
    stretched: RefCell<Vec<Weak<Node>>>,
    text: Rc<crate::text::TextCache>,
    clipboard: RefCell<Option<Box<dyn crate::Clipboard>>>,
    sources: RefCell<Vec<Weak<dyn crate::live::Drainable>>>,
    active_sources: RefCell<BTreeMap<usize, Weak<dyn crate::live::Drainable>>>,
    source_visibility_dirty: Cell<bool>,
    source_serial: Cell<u64>,
    pub(crate) wake: crate::live::Wake,
    /// Mailboxes from other threads, delivered at the start of each flush.
    mailboxes: RefCell<Vec<Weak<dyn crate::mailbox::Deliverable>>>,
    inspection_services: RefCell<Vec<Weak<crate::inspection::ServiceInner>>>,
    /// Depth of answering boundaries in progress: an outermost `flush` or
    /// `flush_windows`. Everything nested inside one, including the
    /// `flush_windows` a queued command runs before its action, answers
    /// nothing; the boundary answers once when it ends.
    answer_depth: Cell<u32>,
    /// Whether a host renders capture jobs; without one, requests are refused.
    capture_support: Cell<bool>,
    captures: RefCell<Vec<PendingCapture>>,
    next_capture: Cell<u64>,
    /// Parameters holding a terminal notification that could not be delivered
    /// synchronously, in the order they were queued.
    deferred: RefCell<Vec<Weak<dyn crate::parameter_binding::Deferred>>>,
    /// One mounted incarnation per `deliver_notifications` request, in global
    /// request order; repeated requests intentionally keep separate entries.
    notifications: RefCell<Vec<notify::NotificationRequest>>,
    /// Internal notification preparation must not replay external delivery frontiers.
    notification_settlement: Cell<bool>,
    notification_input: RefCell<Option<Rc<notify::InputNotificationState>>>,
    notification_origin: RefCell<Option<Weak<notify::InputNotificationState>>>,
}

impl Shared {
    /// Reads the clipboard service's text; see `Runtime::clipboard_text`.
    pub(crate) fn clipboard_read(&self) -> Result<String, RuntimeError> {
        self.writable()?;
        self.clipboard
            .try_borrow_mut()
            .map_err(|_| RuntimeError::BorrowConflict)?
            .as_mut()
            .ok_or(crate::ClipboardError::Unavailable)?
            .read_text()
            .map_err(Into::into)
    }
    /// Writes text to the clipboard service; see `Runtime::set_clipboard_text`.
    pub(crate) fn clipboard_write(&self, text: &str) -> Result<(), RuntimeError> {
        self.writable()?;
        self.clipboard
            .try_borrow_mut()
            .map_err(|_| RuntimeError::BorrowConflict)?
            .as_mut()
            .ok_or(crate::ClipboardError::Unavailable)?
            .write_text(text)
            .map_err(Into::into)
    }
    /// Marks every live window's composed scene stale, for a change that alters
    /// pixels without touching any component.
    fn damage_scenes(&self) {
        for window in self.windows.borrow().iter().filter_map(Weak::upgrade) {
            window.scene_dirty.set(true);
        }
    }
    pub(crate) fn activate_source(&self, source: Weak<dyn crate::live::Drainable>) {
        let key = source.as_ptr() as *const () as usize;
        self.active_sources.borrow_mut().insert(key, source);
    }
    fn scheduled_sources(&self) -> Vec<Rc<dyn crate::live::Drainable>> {
        if self.source_visibility_dirty.replace(false) {
            for source in self.sources.borrow().iter() {
                self.activate_source(source.clone());
            }
        }
        let mut active = self.active_sources.borrow_mut();
        active.retain(|_, weak| weak.upgrade().is_some_and(|s| s.eligible()));
        active.values().filter_map(Weak::upgrade).collect()
    }
    fn repace_sources(&self) {
        for source in self.scheduled_sources() {
            source.repace(self.now.get());
        }
    }
    pub(crate) fn check_evaluation(&self) -> Result<(), RuntimeError> {
        if self.phase.get() == Phase::Update {
            self.readable()
        } else {
            self.writable()
        }
    }
    pub(crate) fn evaluating(self: &Rc<Self>) -> impl Drop {
        let previous = self.phase.replace(Phase::Update);
        InspectionGuard {
            shared: self.clone(),
            previous,
        }
    }
    fn settle_computations(&self) -> Result<(), RuntimeError> {
        for _ in 0..Runtime::MAX_FLUSH_WAVES {
            let pending = std::mem::take(&mut *self.computations.borrow_mut());
            if pending.is_empty() {
                return Ok(());
            }
            for (key, weak) in &pending {
                if let Some(computation) = weak.upgrade()
                    && let Err(error) = computation.refresh()
                {
                    self.computations
                        .borrow_mut()
                        .extend(pending.range(*key..).map(|(k, v)| (*k, v.clone())));
                    return Err(error);
                }
            }
        }
        if self.computations.borrow().is_empty() {
            Ok(())
        } else {
            Err(RuntimeError::UpdateLimit)
        }
    }
    pub(crate) fn register_source(&self, source: Weak<dyn crate::live::Drainable>) {
        let mut sources = self.sources.borrow_mut();
        sources.retain(|weak| weak.strong_count() > 0);
        sources.push(source);
    }
    pub(crate) fn next_source_id(&self) -> u64 {
        let id = self.source_serial.get().wrapping_add(1);
        self.source_serial.set(id);
        id
    }
    /// Drains every due live-data source once for this flush time. Runs before
    /// the update waves so subscribers read what arrived.
    fn service_sources(&self, now: Duration, woke: bool) {
        let sources = self.scheduled_sources();
        for source in sources {
            source.service(now, woke);
        }
    }
    /// Captures all delivery frontiers before invoking any mailbox handler.
    fn deliver_mailboxes(&self) -> Result<(), RuntimeError> {
        let mailboxes: Vec<_> = {
            let mut mailboxes = self.mailboxes.borrow_mut();
            mailboxes.retain(|weak| weak.strong_count() > 0);
            mailboxes.iter().filter_map(Weak::upgrade).collect()
        };
        let fronts: Vec<_> = mailboxes
            .into_iter()
            .map(|mailbox| {
                let count = mailbox.pending_len();
                (mailbox, count)
            })
            .collect();
        for (mailbox, count) in fronts {
            mailbox.deliver(count)?;
        }
        Ok(())
    }
    fn mailbox_pending(&self) -> bool {
        self.mailboxes
            .borrow()
            .iter()
            .filter_map(Weak::upgrade)
            .any(|mailbox| mailbox.has_pending())
    }
    fn inspection_pending(&self) -> bool {
        self.inspection_services
            .borrow()
            .iter()
            .filter_map(Weak::upgrade)
            .any(|service| service.has_pending())
    }
    /// Answers every inspection request queued at every service.
    fn answer_inspection(&self, runtime: &mut Runtime, flush_error: Option<&crate::RuntimeError>) {
        let services: Vec<_> = {
            let mut services = self.inspection_services.borrow_mut();
            services.retain(|weak| weak.strong_count() > 0);
            services.iter().filter_map(Weak::upgrade).collect()
        };
        crate::inspection::answer(&services, runtime, flush_error);
    }
    /// The earliest drain any active source needs, or `now` when a wake is
    /// pending or a source has a visible subscriber but is not yet active.
    /// When a component a continuous zoom stretched is due to be prepared, or
    /// `None` when none is still mounted. Drops what was removed, so a removed
    /// view leaves no deadline behind.
    pub(crate) fn scale_settle_deadline(&self) -> Option<Duration> {
        let due = self.scale_settle.get()?;
        let mut stretched = self.stretched.borrow_mut();
        stretched.retain(|node| node.upgrade().is_some_and(|node| node.mounted.get()));
        if stretched.is_empty() {
            self.scale_settle.set(None);
            return None;
        }
        Some(due)
    }
    pub(crate) fn source_deadline(&self, now: Duration) -> Option<Duration> {
        if self.wake.is_pending() || self.mailbox_pending() || self.inspection_pending() {
            return Some(now);
        }
        self.scheduled_sources()
            .into_iter()
            .filter_map(|source| source.deadline(now))
            .min()
    }
    fn source_inspections(&self) -> Vec<crate::SourceInspection> {
        self.sources
            .borrow()
            .iter()
            .filter_map(Weak::upgrade)
            .map(|source| source.inspect())
            .collect()
    }
    pub(crate) fn record(
        &self,
        kind: crate::EventKind,
        component: Option<u64>,
        parameter: Option<&crate::ParameterId>,
        status: impl FnOnce() -> crate::EventStatus,
    ) {
        if self.history.borrow().mode == crate::DebugMode::Off {
            return;
        }
        let status = status();
        self.history
            .borrow_mut()
            .record(self.now.get(), kind, component, parameter, || status);
    }
    pub(crate) fn writable(&self) -> Result<(), RuntimeError> {
        if self.closed.get() {
            return Err(RuntimeError::Closed);
        }
        if self.phase.get() == Phase::Update {
            return Err(RuntimeError::WriteDuringUpdate);
        }
        if self.phase.get() == Phase::Paint {
            return Err(RuntimeError::WriteDuringPaint);
        }
        if self.phase.get() == Phase::Layout {
            return Err(RuntimeError::WriteDuringLayout);
        }
        if self.phase.get() == Phase::Inspect {
            return Err(RuntimeError::WriteDuringInspection);
        }
        if self.phase.get() == Phase::Delivery {
            return Err(RuntimeError::WriteDuringDelivery);
        }
        Ok(())
    }
    /// Whether a parameter sink may run now: an open runtime outside every
    /// evaluation, paint, inspection and delivery phase.
    pub(crate) fn can_deliver(&self) -> bool {
        !self.closed.get() && self.phase.get() == Phase::Idle
    }
    /// Marks a delivery callback; writes fail until the guard drops.
    pub(crate) fn enter_delivery(self: &Rc<Self>) -> impl Drop {
        let previous = self.phase.replace(Phase::Delivery);
        InspectionGuard {
            shared: self.clone(),
            previous,
        }
    }
    pub(crate) fn defer(&self, parameter: Weak<dyn crate::parameter_binding::Deferred>) {
        let mut queue = self.deferred.borrow_mut();
        if !queue.iter().any(|queued| queued.ptr_eq(&parameter)) {
            queue.push(parameter);
        }
    }
    /// Whether a queued notification is still undelivered; entries whose
    /// parameter was dropped or already delivered are pruned.
    fn has_deferred(&self) -> bool {
        let mut queue = self.deferred.borrow_mut();
        queue.retain(|weak| weak.upgrade().is_some_and(|parameter| parameter.pending()));
        !queue.is_empty()
    }
    /// Delivers queued terminal notifications when a sink may run, in queue order.
    /// A delivery that queues further work is settled in the same call; the
    /// queue holds each live parameter at most once, with at most one event each.
    pub(crate) fn dispatch_deferred(&self) {
        while self.can_deliver() {
            let queued = std::mem::take(&mut *self.deferred.borrow_mut());
            if queued.is_empty() {
                break;
            }
            for parameter in queued.iter().filter_map(Weak::upgrade) {
                parameter.dispatch_deferred();
            }
        }
    }
    /// Forgets the failure site of an earlier operation, at the start of an
    /// outermost flush, paint or interval service.
    pub(crate) fn reset_failure(&self) {
        self.failure.replace(None);
    }
    pub(crate) fn readable(&self) -> Result<(), RuntimeError> {
        if self.closed.get() {
            Err(RuntimeError::Closed)
        } else {
            Ok(())
        }
    }
}

/// One queued `UpdateContext::request_reparent`: the child, its new owner and the
/// insertion position among that owner's children.
type RequestedMove = (Weak<Node>, Weak<Node>, usize);

/// Highest physical-pixels-per-logical-pixel at which a magnifying view prepares
/// coverage. Beyond it the prepared coverage is stretched, which bounds cache
/// and mask sizes; a window whose own scale is higher keeps its own scale.
pub(crate) const MAX_VIEW_RASTER_SCALE: f64 = 8.0;

/// How long a component's view scale must rest before it is prepared again at
/// it, once the scale has changed at two different times within this interval.
///
/// A change of view scale invalidates the layout of every component under the
/// view, because text is prepared for the size it is shown at. A single change,
/// such as a zoom key or a restored workspace, is prepared at once, and so are
/// changes made at the same instant, such as nested views resolving in one
/// flush. A change that follows another at a later time within this interval is
/// part of a continuous zoom: the component keeps its prepared text and coverage,
/// which composition stretches to the new scale, and is prepared once when its
/// scale has rested this long. `docs/rendering-platform.md` records the measurement
/// behind it.
pub(crate) const VIEW_SCALE_SETTLE: Duration = Duration::from_millis(150);

pub(crate) fn raster_scale(window: f64, view: f32) -> f64 {
    let wanted = window * f64::from(view);
    if view <= 1.0 {
        wanted
    } else {
        wanted.min(window.max(MAX_VIEW_RASTER_SCALE))
    }
}

pub(crate) trait ContextLoss {
    fn cancel(&self);
}

pub(crate) struct Node {
    window: RefCell<Rc<native_windows::WindowShared>>,
    detached: Cell<bool>,
    /// Whether the view may move into its own native window, read when asked.
    detachable: RefCell<crate::Permission>,
    pub(crate) font_layout: [Cell<u64>; 3],
    pub(crate) font_paint: Cell<u64>,
    pub(crate) metric_layout: [Cell<u64>; 3],
    pub(crate) metric_paint: Cell<u64>,
    focus_reader: Cell<bool>,
    /// Whether the component's last update read the motion preference.
    motion_reader: Cell<bool>,
    /// Whether this node is already in the motion-preference subscriber list.
    motion_listed: Cell<bool>,
    focus_update_reader: Cell<bool>,
    /// Whether the component has requested its own children through
    /// `UpdateContext::request_children`, after which it owns them and a new
    /// description of it leaves them in place.
    self_managed: Cell<bool>,
    /// Constructor-generated parts are retained across compatible descriptions.
    initial_children_owned: bool,
    /// False until this component's latest requested children are applied.
    children_current: Cell<bool>,
    hover_within: Cell<bool>,
    hover_reader: Cell<bool>,
    input_cancelled: Cell<bool>,
    serial: u64,
    pub(crate) shared: Rc<Shared>,
    mounted: Cell<bool>,
    visible: Cell<bool>,
    local_visible: Cell<bool>,
    /// Lifetime owner. Mutable: `Runtime::reparent` moves a retained subtree to
    /// another owner without changing any incarnation.
    parent: RefCell<Option<Weak<Node>>>,
    parent_serial: Cell<Option<u64>>,
    composed: bool,
    layout_dirty: Cell<bool>,
    measured: Cell<Option<(crate::Constraints, crate::Size)>>,
    limits: Cell<Option<crate::SizeLimits>>,
    bounds: Cell<Option<crate::Rect>>,
    /// Allocated size in this component's own local pixels. `bounds` is the same
    /// allocation in the window, so the two differ by `scale`.
    size: Cell<Option<crate::Size>>,
    /// Accumulated view scale of this component's local space: the product of its
    /// presentation ancestors' `child_transform` scales, 1 for a surface or root.
    scale: Cell<f32>,
    /// The accumulated view scale this component's text and coverage were last
    /// prepared at. It follows `scale`, except during a continuous zoom, when
    /// drawing prepared at an earlier scale is stretched until the scale rests;
    /// see [`VIEW_SCALE_SETTLE`].
    prepared_scale: Cell<f32>,
    /// When `scale` last changed, which tells a continuous zoom from a single
    /// change.
    scale_changed: Cell<Option<Duration>>,
    /// Whether this node is in its window's list of stretched components.
    stretched: Cell<bool>,
    /// `scale` times this component's own `child_transform` scale, as last
    /// propagated to its children.
    content_scale: Cell<f32>,
    /// This component's `child_transform` offset as of its last arrangement.
    view_offset: Cell<crate::Point>,
    /// Where the presentation parent's last `arrange` placed this component, in
    /// the parent's content space, with the local clip it gave.
    placement: Cell<Option<(crate::Rect, Option<crate::Rect>)>>,
    /// The visible children this component's last `arrange` placed, in order;
    /// their placements are reused while it is only moved.
    arranged_children: RefCell<Vec<u64>>,
    clip: Cell<Option<crate::Rect>>,
    children_clip: Cell<Option<crate::Rect>>,
    dirty: Cell<Option<Invalidation>>,
    deadline: Cell<Option<Duration>>,
    presentation_suspended: Cell<bool>,
    animated: Cell<bool>,
    timer: Cell<Option<Duration>>,
    gestures: RefCell<Vec<Weak<dyn ContextLoss>>>,
}

impl Node {
    /// Effective visibility, the condition a focus request checks.
    pub(crate) fn is_visible(&self) -> bool {
        self.visible.get()
    }
    pub(crate) fn is_presented(&self) -> bool {
        self.visible.get()
            && self.window().visible.get()
            && !self.clip.get().is_some_and(crate::Rect::is_empty)
    }
    pub(crate) fn presentation_interval(&self) -> Duration {
        self.window().presentation_interval.get()
    }
    /// Physical pixels per local logical pixel at which this component's
    /// coverage is prepared: the window's effective scale times the accumulated
    /// view scale. When a view magnifies, the cap is the greater of the window
    /// scale and [`MAX_VIEW_RASTER_SCALE`], preserving a denser window.
    pub(crate) fn raster_scale(&self) -> f64 {
        raster_scale(
            self.window().viewport.get().scale(),
            self.prepared_scale.get(),
        )
    }
    /// Maps a window-logical point into this component's local space.
    fn window_to_local(&self, point: crate::Point) -> Option<crate::Point> {
        let [x, y, _, _] = self.bounds.get()?.values();
        let scale = f64::from(self.scale.get());
        crate::Point::new(
            ((f64::from(point.x()) - f64::from(x)) / scale) as f32,
            ((f64::from(point.y()) - f64::from(y)) / scale) as f32,
        )
        .ok()
    }
    fn window(&self) -> Rc<native_windows::WindowShared> {
        self.window.borrow().clone()
    }
    /// Lifetime owner, which presentation movement does not change.
    pub(crate) fn logical_parent(&self) -> Option<Rc<Node>> {
        self.parent.borrow().as_ref().and_then(Weak::upgrade)
    }
    pub(crate) fn logical_parent_serial(&self) -> Option<u64> {
        self.parent_serial.get()
    }

    fn invalidate_focus_path(self: &Rc<Self>) {
        let mut current = Some(self.clone());
        while let Some(node) = current {
            if node.focus_reader.get() || node.focus_update_reader.get() {
                node.invalidate(Invalidation::Paint);
            }
            current = node.presentation_parent();
        }
    }

    pub(crate) fn invalidate_presentation(self: &Rc<Self>, kind: Invalidation) {
        if !self.mounted.get() {
            return;
        }
        if kind == Invalidation::Paint && !self.is_presented() {
            self.presentation_suspended.set(true);
        } else {
            self.invalidate(kind);
        }
    }
    pub(crate) fn invalidate(self: &Rc<Self>, kind: Invalidation) {
        if !self.mounted.get() {
            return;
        }
        if kind == Invalidation::Layout {
            self.invalidate_layout();
        }
        self.dirty
            .set(Some(self.dirty.get().map_or(kind, |old| old.max(kind))));
        if self.visible.get() {
            self.window()
                .pending
                .borrow_mut()
                .insert(self.serial, Rc::downgrade(self));
        }
    }
    fn invalidate_layout(&self) {
        self.layout_dirty.set(true);
        self.measured.set(None);
        self.limits.set(None);
        if self.visible.get() && self.composed {
            self.window().layout_pending.set(true);
            if let Some(parent) = self.presentation_parent() {
                parent.invalidate_layout();
            }
        }
    }
    fn stop_deadline(&self) {
        if let Some(deadline) = self.deadline.take() {
            self.window()
                .deadlines
                .borrow_mut()
                .remove(&(deadline, self.serial));
        }
    }
    fn stop_timer(&self) {
        if let Some(timer) = self.timer.take() {
            self.window()
                .timers
                .borrow_mut()
                .remove(&(timer, self.serial));
        }
    }
    fn start_timer(self: &Rc<Self>, deadline: Duration) {
        self.stop_timer();
        self.timer.set(Some(deadline));
        self.window()
            .timers
            .borrow_mut()
            .insert((deadline, self.serial), Rc::downgrade(self));
    }
    fn lose_context(&self) {
        self.stop_timer();
        if self.shared.drag_source.get() == Some(self.serial) {
            self.shared.drag_lost.set(true);
        }
        if self.window().capture.get() == Some(self.serial) {
            self.window().capture.set(None);
        }
        let gestures = std::mem::take(&mut *self.gestures.borrow_mut());
        for gesture in gestures {
            if let Some(gesture) = gesture.upgrade() {
                gesture.cancel();
            }
        }
    }
}

/// A retained, general component evaluated only for pending work.
///
/// This is the backend-independent lifecycle and layout contract. Store
/// retained presentation data in the component. In `update`, read all current
/// dependencies through the context on every evaluation, including paint-only
/// passes; previous subscriptions are replaced. No state writes are permitted
/// during evaluation. Errors preserve dirtiness for a later retry.
/// Callbacks run on the runtime's owning thread. None is safe for time-critical producers.
///
/// A component is `'static`, as [`Any`](std::any::Any) requires: it owns its
/// data or shares it through an `Rc`, and never borrows from a local or from
/// the code that builds it. The same holds for the rows and services it hands
/// to the runtime; [`Self::content_rows`] shows the pattern.
///
/// # Hooks by concern
///
/// Only [`update`](Self::update) is required; every other hook has a
/// default. A typical widget implements `update`, [`measure`](Self::measure)
/// and [`paint`](Self::paint) first. An interactive one adds
/// [`focusable`](Self::focusable), [`pointer_input`](Self::pointer_input) or
/// [`keyboard_event`](Self::keyboard_event) and [`cancel_input`](Self::cancel_input),
/// and one that tools should understand adds [`inspect`](Self::inspect). The
/// table groups the hooks by concern; their declaration order below is
/// unchanged.
///
/// | Concern | Hooks |
/// |---|---|
/// | Lifecycle | [`update`](Self::update), [`reconcile`](Self::reconcile), [`check_description`](Self::check_description), [`mounted`](Self::mounted), [`visibility_changed`](Self::visibility_changed), [`unmounted`](Self::unmounted), [`take_children`](Self::take_children), [`retain_initial_children`](Self::retain_initial_children) |
/// | Pointer and keys | [`pointer_input`](Self::pointer_input), [`keyboard_event`](Self::keyboard_event), [`key_input`](Self::key_input), [`receives_unfocused_keys`](Self::receives_unfocused_keys), [`hover_changed`](Self::hover_changed), [`cursor`](Self::cursor), [`hit_test`](Self::hit_test), [`hit_test_foreground`](Self::hit_test_foreground), [`command_target`](Self::command_target), [`timer_elapsed`](Self::timer_elapsed), [`cancel_input`](Self::cancel_input), [`input_reset`](Self::input_reset) |
/// | Focus | [`focusable`](Self::focusable), [`tab_stop`](Self::tab_stop), [`descendant_tab_stops`](Self::descendant_tab_stops), [`tab_navigation`](Self::tab_navigation), [`focus_siblings_on_removal`](Self::focus_siblings_on_removal), [`focus_gained`](Self::focus_gained), [`focus_lost`](Self::focus_lost), [`focus_within_changed`](Self::focus_within_changed) |
/// | Text editing | [`text_key_event`](Self::text_key_event), [`text_input`](Self::text_input), [`text_preedit`](Self::text_preedit), [`text_input_state`](Self::text_input_state), [`text_command`](Self::text_command) |
/// | Drag and drop | [`drag_event`](Self::drag_event), [`drag_finished`](Self::drag_finished) |
/// | Layout | [`measure`](Self::measure), [`arrange`](Self::arrange), [`size_limits`](Self::size_limits), [`measure_limits`](Self::measure_limits), [`fills_row_height`](Self::fills_row_height), [`layout_position`](Self::layout_position), [`clips_children`](Self::clips_children), [`child_transform`](Self::child_transform) |
/// | Children and surfaces | [`child_visible`](Self::child_visible), [`child_surface`](Self::child_surface), [`dismiss_surface`](Self::dismiss_surface), [`stacks_children`](Self::stacks_children), [`child_stacking_priority`](Self::child_stacking_priority), [`child_redocked`](Self::child_redocked), [`can_reveal`](Self::can_reveal), [`reveal`](Self::reveal), [`reveal_child`](Self::reveal_child) |
/// | Painting | [`paint`](Self::paint), [`paint_foreground`](Self::paint_foreground) |
/// | Notifications | [`deliver_notifications`](Self::deliver_notifications) |
/// | Inspection and search | [`inspect`](Self::inspect), [`conceals_input`](Self::conceals_input), [`content_rows`](Self::content_rows), [`try_content_rows`](Self::try_content_rows) |
/// | Workspace | [`workspace_state`](Self::workspace_state), [`workspace_state_with_children`](Self::workspace_state_with_children), [`restore_workspace_state`](Self::restore_workspace_state), [`restore_workspace_state_with_children`](Self::restore_workspace_state_with_children) |
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a pert-ui component",
    label = "`{Self}` does not implement `Component`",
    note = "a component type implements `Component`, where only `update` is required, and owns its data (`'static`)",
    note = "if this value is an `Element`, from `.key(..)` or `Element::new`, it is already described: pass it as a child instead of describing it again"
)]
pub trait Component: std::any::Any {
    /// Applies a new same-key, same-type description to this mounted component.
    ///
    /// Called after new descendants have mounted successfully, before the next
    /// update/layout pass. Children have already been extracted from `next`.
    /// Copy configuration, bindings and callbacks from `next`, retaining local
    /// interaction state, drafts and caches that are still valid. Return the
    /// narrowest invalidation needed, or `None` when nothing changed.
    ///
    /// The default retains constructor values, for components configured solely
    /// through bindings. Components with replaceable description properties must
    /// override this method. No application writes, I/O or panics are permitted;
    /// this is an infallible property application, not an event callback. Changing
    /// an edit target requires explicitly cancelling the old edit before rebinding.
    /// Identity, focus and pointer capture are preserved by the runtime.
    ///
    /// Because this method cannot fail, it applies the new description without
    /// validating it. Validate bindings in [`Self::update`] as on the first
    /// update: an invalid replacement, such as a signal from another runtime,
    /// then fails that update, the error is returned from the flush, and the
    /// component stays dirty and keeps its previous presentation, so every
    /// later flush retries until a valid description replaces it. Check an
    /// implementation with [`crate::check_redescription`], which compares a
    /// re-described component with a fresh mount of the same description.
    fn reconcile(&mut self, _next: Self) -> Option<Invalidation>
    where
        Self: Sized,
    {
        None
    }

    /// Checks this description before the runtime mounts it or applies it to
    /// a mounted component. The default accepts every description.
    ///
    /// [`Runtime::mount_tree`] and [`Runtime::reconcile_children`], and the
    /// children a component requests through
    /// [`UpdateContext::request_children`], call it on every described
    /// component, recursively, after validating keys and before anything
    /// mounts, unmounts or is reconciled: an error here rejects the whole call
    /// and leaves the tree unchanged. Use it for errors a description can see
    /// on its own, such as invalid or duplicate keys among the children it
    /// builds, so that an invalid description cannot remove mounted children
    /// it fails to describe. It has no runtime access; bindings are checked in
    /// [`Self::update`], as [`Self::reconcile`] describes. It must be cheap,
    /// deterministic and free of side effects, and must not panic.
    ///
    /// # Errors
    /// Returns the error the call that mounts or reconciles this description
    /// reports, conventionally [`LayoutError::InvalidChild`](crate::LayoutError::InvalidChild).
    fn check_description(&self) -> Result<(), RuntimeError> {
        Ok(())
    }

    /// Declares a content command target independent of focusability or registered
    /// shortcuts. Pointer/focus interaction selects the nearest such presentation
    /// ancestor. Toolbar/menu components keep the default false to preserve it.
    fn command_target(&self) -> bool {
        false
    }

    /// Optional editing phase before capture bindings and ordinary bubbling.
    /// Custom editors return InsertText for printable insertion, Handled for
    /// editing chords (including disabled local undo), and Ignored to continue
    /// from their parent without dispatching the editing action twice.
    ///
    /// The phase starts at the focused component and passes on to the parent
    /// of each editor that returns Ignored, so a control that owns an embedded
    /// editor, such as a row with a [`DraftEditor`](crate::DraftEditor), answers
    /// the keys that editor leaves to it (Enter, Escape, stepping keys) before
    /// capture-scope commands and held-input registrations see them. The first
    /// component that returns NotAnEditor, the default, ends the phase, and
    /// ordinary bubbling starts at it. [`Runtime::command_key_input`], the
    /// native path, runs this phase; the headless [`Runtime::key_input`] runs
    /// it only for a fresh Escape while an interactive surface is open, so an
    /// editor answers Escape before the surface closes on both paths.
    ///
    /// # Errors
    /// An implementation returns an error when handling the key fails, such as a rejected
    /// binding write or edit commit. The error propagates from [`Runtime::key_input`] and
    /// the key reaches no other handler. The default never fails.
    fn text_key_event(
        &mut self,
        _cx: &mut EventContext,
        _input: crate::CommandKeyInput,
        _transition: crate::KeyTransition,
        _keyboard: &crate::KeyboardState,
    ) -> Result<crate::TextKeyOutcome, RuntimeError> {
        Ok(crate::TextKeyOutcome::NotAnEditor)
    }

    /// Performs a text command on this component's own text: a clipboard or
    /// selection action on its selection and caret, or beginning an edit.
    ///
    /// The runtime calls this for a [`TextCommand`](crate::TextCommand) queued with
    /// [`EventContext::request_text_command`], during the event phase of the flush
    /// that reaches it, so an implementation may write bindings and commit edits
    /// exactly as it does for ordinary key input, and synchronously for
    /// [`Runtime::text_command`]. Default reports `NotAnEditor`.
    ///
    /// An implementation reads and writes the clipboard through the context, keeps
    /// a failed clipboard operation from destroying text, and reports `Unavailable`
    /// rather than failing when the command does not apply to its current state.
    /// Only [`TextCommand::BeginEdit`](crate::TextCommand::BeginEdit) moves
    /// focus: an editor with a resting state answers it as its own editing key
    /// does, requesting focus for its editing part with
    /// [`EventContext::request_focus`] or
    /// [`EventContext::request_focus_for`], and reports `Handled` only when it
    /// began editing. The other commands never request focus.
    ///
    /// # Errors
    /// An implementation returns an error for a failed binding write or edit commit, not
    /// for a command that does not apply (that is `Unavailable`). The error propagates from
    /// [`Runtime::flush`]. The default never fails.
    fn text_command(
        &mut self,
        _cx: &mut EventContext,
        _command: crate::TextCommand,
    ) -> Result<crate::TextCommandOutcome, RuntimeError> {
        Ok(crate::TextCommandOutcome::NotAnEditor)
    }

    /// A retained child returned from a native window. Containers may select the
    /// child by its stable identity. Default does nothing. Called on the UI thread.
    /// The callback must not fail after moving presentation; update local state only.
    fn child_redocked(&mut self, _cx: &mut EventContext, _child: &WidgetId) {}

    /// Enables independent back-to-front ordering of immediate children. Default
    /// is false. Pointer presses and explicit/keyboard focus raise the containing
    /// child within equal [`Self::child_stacking_priority`] values before dispatch,
    /// including presses consumed by descendants. Logical
    /// child order and identities remain unchanged. Children intentionally overlap;
    /// the parent must provide geometry and clipping through ordinary layout.
    /// Promoted menus/dialogs retain their existing surface ordering and modality.
    /// Request layout when this flag changes to refresh stationary hover and drag targets.
    fn stacks_children(&self) -> bool {
        false
    }

    /// Relative presentation priority of one immediate child of a stacking parent.
    ///
    /// Default is zero for every child. When [`Self::stacks_children`] is true,
    /// larger priorities draw and hit-test above smaller priorities. Focus and
    /// pointer raising change order only among equal-priority children; their
    /// initial order follows logical child order. This changes neither layout,
    /// logical navigation nor retained identity. Promoted surfaces keep their
    /// separate surface order. Nonstacking parents ignore priorities.
    ///
    /// `key` is the immediate child's stable sibling key. This bounded UI-thread
    /// query must be read-only and use retained state prepared during update;
    /// request layout when priorities change, even with unchanged geometry, so
    /// stationary hover and drag targets refresh. Use [`Invalidation::Layout`] or
    /// [`UpdateContext::request_layout`]. Do not write signals or
    /// mutate retained state during the query. Stacking traversal allocates a
    /// child-order buffer and stably sorts it; priority queries may run repeatedly
    /// during sorting, so keep them cheap. Nonstacking parents avoid this work.
    ///
    /// ```
    /// # use pert_ui_runtime as pert_ui;
    /// use pert_ui::*;
    /// use std::time::Duration;
    /// struct Layers(Vec<Element>);
    /// impl Component for Layers {
    ///     fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
    ///         Ok(())
    ///     }
    ///     fn take_children(&mut self) -> Vec<Element> {
    ///         std::mem::take(&mut self.0)
    ///     }
    ///     fn stacks_children(&self) -> bool { true }
    ///     fn child_stacking_priority(&self, key: &str) -> i32 {
    ///         if key == "tool" { 1 } else { 0 }
    ///     }
    ///     fn measure(&self, _: &MeasureContext<'_>, space: Constraints)
    ///         -> Result<Size, RuntimeError> {
    ///         Ok(Size::new(space.width().unwrap_or(200.0), space.height().unwrap_or(100.0))?)
    ///     }
    ///     fn arrange(&self, cx: &mut ArrangeContext<'_>, size: Size)
    ///         -> Result<(), RuntimeError> {
    ///         for index in 0..cx.len() { cx.place(index, Rect::from_size(size))?; }
    ///         Ok(())
    ///     }
    /// }
    /// let mut ui = Runtime::new();
    /// ui.set_viewport(Viewport::new(200.0, 100.0, 1.0)?)?;
    /// let root = ui.mount_tree(Element::new("layers", Layers(vec![
    ///     Element::new("tool", Button::new("Tool", |_| Ok(()))),
    ///     Element::new("center", Button::new("Center", |_| Ok(()))),
    /// ])))?;
    /// ui.flush(Duration::ZERO)?;
    /// let tool = ui.find_descendant(&root, "tool")?.unwrap();
    /// let center = ui.find_descendant(&root, "center")?.unwrap();
    /// ui.focus(&center)?;
    /// ui.pointer_input(PointerEvent::new(Point::new(5.0, 5.0)?,
    ///     PointerKind::Press(PointerButton::Primary)))?;
    /// assert_eq!(ui.focused(), Some(tool));
    /// # Ok::<(), RuntimeError>(())
    /// ```
    fn child_stacking_priority(&self, _key: &str) -> i32 {
        0
    }

    /// Preferred parent-local logical origin for freely positioned containers.
    /// Ordinary flow containers ignore it. Default is zero. Change retained
    /// position in input/update and invalidate layout; never write during measure.
    fn layout_position(&self) -> crate::Point {
        crate::Point::default()
    }

    /// How this container presents its children: one uniform scale and one offset.
    ///
    /// Default is the identity. Children are measured, arranged and painted in
    /// content space and never learn that they are scaled, except that text and
    /// shape coverage is prepared at the displayed size so it stays sharp. A
    /// child's local point `p`, placed at `child_origin` by [`Self::arrange`],
    /// appears at `offset + scale * (child_origin + p)` in this component's local
    /// space. Transforms nest by multiplication. Clipping, hit testing and culling
    /// keep working on rectangles, and pointer positions reach children through
    /// [`EventContext::to_local`] as usual.
    ///
    /// Return retained state, and invalidate layout when it changes:
    /// `cx.invalidate(Invalidation::Layout)` from an event. A changed **offset**
    /// moves retained drawing, so a pan costs what scrolling costs. A changed
    /// **scale** additionally invalidates the layout of every component under
    /// this one, because text is prepared for the scale it is shown at. During a
    /// continuous zoom, when the scale changes again at a later time within
    /// 150 ms, the runtime instead stretches the drawing prepared at the earlier
    /// scale and prepares the components once the scale has rested that long,
    /// so each step of the gesture costs about what a pan costs. Both are
    /// ordinary layout invalidations of this container, so its ancestors are
    /// arranged again as for any other; nothing outside it is updated.
    ///
    /// Surfaces a descendant opens (popovers, menus, tooltips, dialogs, drag
    /// previews) are not scaled: they are placed against the anchor's displayed
    /// rectangle at the window's own scale. `docs/rendering-platform.md` has the contract.
    ///
    /// ```
    /// # use pert_ui_runtime as pert_ui;
    /// use pert_ui::*;
    /// struct Magnifier(ViewTransform, Vec<Element>);
    /// impl Component for Magnifier {
    ///     fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> { Ok(()) }
    ///     fn take_children(&mut self) -> Vec<Element> { std::mem::take(&mut self.1) }
    ///     fn child_transform(&self) -> ViewTransform { self.0 }
    /// }
    /// let mut ui = Runtime::new();
    /// ui.set_viewport(Viewport::new(200.0, 100.0, 1.0)?)?;
    /// let view = ViewTransform::new(ViewScale::new(2.0)?, Point::new(10.0, 0.0)?);
    /// let content = vec![Element::new("label", Label::new("Hi"))];
    /// let root = ui.mount_tree(Element::new("magnifier", Magnifier(view, content)))?;
    /// ui.flush(std::time::Duration::ZERO)?;
    /// let label = ui.children(&root)?[0].clone();
    /// let shown = ui.bounds(&label)?.expect("laid out");
    /// // The default arrangement fills the 200 × 100 allocation; shown at 200%
    /// // from an offset of ten pixels.
    /// assert_eq!(shown.values(), [10.0, 0.0, 400.0, 200.0]);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    fn child_transform(&self) -> crate::ViewTransform {
        crate::ViewTransform::IDENTITY
    }

    /// Promotes an immediate child to an anchored window presentation layer.
    /// Default is ordinary layout. The parent allocation is the anchor; runtime
    /// flips/constrains the child's preferred placement to the window and escapes
    /// ancestor clips while retaining logical ownership. Child visibility controls
    /// admission. Modal kinds allocate the full window/owner scope; context menus
    /// and submenus use the child's local placement and notifications use the
    /// window's bottom right. Implementations provide scroll access for oversized
    /// content and never promote interactive tooltip descendants.
    /// Changing a visible child's kind runs admission again, including conflict,
    /// cancellation and focus policy, while retaining its original opener until
    /// the surface closes. Failed admission preserves current modal blocking
    /// until the actual admission can be retried.
    fn child_surface(&self, _key: &str) -> Option<crate::SurfaceKind> {
        None
    }
    /// Closes this owner's surface in response to shared routing/lifecycle rules.
    /// Must hide its surface child on the next update and release local actions,
    /// except modal owners may ignore Escape when explicit response is required
    /// and ignore outside press, focus leaving and native focus loss. Ownership
    /// removal/hiding and admission failure must always terminate modal sessions.
    /// Do not activate application commands here.
    ///
    /// # Errors
    /// An implementation returns an error when releasing its local state fails. It
    /// propagates from the input call that dismissed the surface, or from the next
    /// [`Runtime::flush`] when an infallible teardown or focus-loss operation dismissed it.
    /// The default never fails.
    fn dismiss_surface(
        &mut self,
        _cx: &mut EventContext,
        _reason: crate::SurfaceDismissal,
    ) -> Result<(), RuntimeError> {
        Ok(())
    }

    /// Returns a cursor for a point in local logical coordinates, or `None` to
    /// defer to ancestors. Called read-only on the UI thread after layout; no
    /// writes or I/O are allowed. Captured components may return a resize cursor
    /// outside their bounds until the gesture ends. Default defers.
    fn cursor(&self, _point: crate::Point) -> Option<crate::CursorIcon> {
        None
    }

    /// Whether automatic keyboard traversal and pointer presses may focus this component.
    /// Default is false; explicit low-level `Runtime::focus` remains available.
    ///
    /// When the focused component becomes non-focusable (having been focusable when
    /// it received focus; explicit [`Runtime::focus`] on a non-focusable component is
    /// left alone), or is hidden or removed, focus moves at the end of the flush to
    /// its nearest presentation ancestor that is shown and focusable, and otherwise
    /// to the window's first focusable component. A focus request made in the same
    /// flush takes precedence. A pointer press directly on a non-focusable
    /// component whose subtree already holds focus leaves focus where it is.
    fn focusable(&self) -> bool {
        false
    }
    /// Whether keyboard traversal ([`Runtime::focus_next`], Tab and Shift+Tab)
    /// stops at this component.
    ///
    /// Default is [`Self::focusable`]. A focusable component whose focus moves
    /// by other keys, such as menu-bar titles that arrow keys move between
    /// while menu mode is on, returns false: Tab then leaves it for the next
    /// tab stop after it in navigation order, while pointer presses, explicit
    /// focus and the component's own focus requests still reach it.
    fn tab_stop(&self) -> bool {
        self.focusable()
    }
    /// Whether this component's descendants take part in keyboard traversal
    /// ([`Runtime::focus_next`], Tab and Shift+Tab).
    ///
    /// Default true. With false, traversal skips every descendant in this
    /// component's presentation subtree, at any depth, while the component
    /// itself stays a stop when [`Self::tab_stop`] holds. A surface (a popup,
    /// menu or dialog) starts a traversal scope of its own, as does a detached
    /// native window's root, so a dialog opened from inside this component
    /// keeps its stops. Pointer presses, explicit focus and the descendants'
    /// own focus requests still reach them, and focus restoration after hiding
    /// or removal is unaffected. Traversal from a focused descendant continues
    /// from its place in navigation order, which visits this component's
    /// ordinary descendants right after it: Tab moves past them and Shift+Tab
    /// reaches this component or the stop before it. A collection that keeps
    /// its items' content out of traversal, as
    /// [`VirtualList::item_tab_stops`](crate::VirtualList::item_tab_stops)
    /// does, provides another keyboard route to that content.
    fn descendant_tab_stops(&self) -> bool {
        true
    }
    /// Offered an unmodified Tab ([`FocusDirection::Forward`](crate::FocusDirection::Forward))
    /// or Shift+Tab ([`FocusDirection::Backward`](crate::FocusDirection::Backward))
    /// press or repeat that has reached default keyboard traversal, before the
    /// runtime moves focus. Return [`InputOutcome::Handled`](crate::InputOutcome::Handled) to claim it: traversal
    /// does not run, and the component moves focus itself if it wants to, for
    /// example to the next cell of a table it is editing. Default `Ignored`.
    ///
    /// The offer goes to the focused component and then to each ancestor
    /// ordinary bubbling reaches, innermost first, stopping at an ancestor a
    /// modal surface does not admit. Keys owned before traversal keep their
    /// owner: on [`Runtime::command_key_input`], the native path, a key held
    /// by an earlier press or a command, the editing phase
    /// ([`Self::text_key_event`]), capture bindings and held-input
    /// registrations all come first; [`Runtime::key_input`] offers the key
    /// where it would traverse. Claiming does not take the key's release,
    /// which is dispatched as before to the component focused when it
    /// arrives. `transition` is [`KeyTransition::Pressed`](crate::KeyTransition::Pressed)
    /// for a press and [`KeyTransition::Repeated`](crate::KeyTransition::Repeated)
    /// for an operating-system repeat; a release is never offered.
    ///
    /// # Errors
    /// An implementation returns an error when handling the key fails. It
    /// propagates from the key entry point, the key reaches no other handler,
    /// nothing traverses and the components offered the key are cleaned up
    /// as after a failed [`Self::keyboard_event`]. Effects the implementation
    /// committed before failing, such as a signal write, are not undone;
    /// whether a request it queued, such as a focus request, survives follows
    /// the entry point's failure cleanup, as for `keyboard_event`: the native
    /// path cancels pending focus requests. The default never fails.
    fn tab_navigation(
        &mut self,
        _cx: &mut EventContext,
        _direction: crate::FocusDirection,
        _transition: crate::KeyTransition,
    ) -> Result<crate::InputOutcome, RuntimeError> {
        Ok(crate::InputOutcome::Ignored)
    }
    /// Whether focus passes to a sibling of the same type when this component is
    /// removed while it holds focus.
    ///
    /// Default is false: a removed focused component hands focus to its nearest
    /// shown, focusable presentation ancestor, as [`Self::focusable`] describes.
    /// With true, when the parent's new description no longer lists this
    /// component, focus first goes to the nearest following sibling of the same
    /// component type in the previous child order that survives the description
    /// and can take focus, then to the nearest such preceding sibling, and only
    /// then to the ancestors. Siblings removed by the same description are
    /// skipped. Removal through [`Runtime::unmount`] or
    /// [`UpdateContext::request_unmount`] keeps the default fallback. A focus
    /// request made in the same flush still takes precedence.
    ///
    /// Removable items in a row, such as `Chip`, use this so that removing the
    /// focused item from the keyboard leaves focus on its neighbor and a
    /// repeated Delete continues along the row.
    fn focus_siblings_on_removal(&self) -> bool {
        false
    }
    /// Whether this component receives keys while nothing in its window has focus.
    ///
    /// Default is false. Keys normally reach only the focused component and bubble
    /// from it through its ancestors. With no focus, [`Runtime::key_input`] offers a
    /// key other than Tab to the first visible component, in tree order, that
    /// returns true and is not blocked by a modal surface; the key then bubbles
    /// from there as usual. A component that returns true must ignore keys it does
    /// not own, because it cannot rely on having been focused. `MenuBar` uses this
    /// so F10 works before anything has been focused.
    fn receives_unfocused_keys(&self) -> bool {
        false
    }
    /// Whether this component holds secret input, such as a password, that
    /// diagnostics must not record.
    ///
    /// Default is false. While it returns true, event history records no
    /// [`EventKind::KeyInput`](crate::EventKind::KeyInput) for a key delivered
    /// to this component, including the same key bubbling through its
    /// ancestors, so the physical keys typed into it cannot be read back.
    /// Committed text is never retained in history anyway; its byte count
    /// still is. A component that returns true should also report
    /// [`Semantics::protected`](crate::Semantics::protected) and keep its
    /// content out of [`Component::inspect`]. `TextField::concealed` uses it.
    fn conceals_input(&self) -> bool {
        false
    }
    /// Handles a pointer event, bubbling ignored events through logical ancestors.
    /// Captured events keep their owner even outside its clipped bounds.
    ///
    /// A move can arrive more than once at one position: after every layout or
    /// stacking change the runtime repeats the last position
    /// ([`PointerEvent::is_refresh`](crate::PointerEvent::is_refresh)), so that
    /// geometry that changed under a stationary pointer is followed. A handler
    /// must therefore converge: invalidate only when the state it derives from
    /// the event changed. Here a board moves its item by the drag's offset and
    /// requests layout only for a new offset; requesting it on every move would
    /// make each layout produce the next move until the flush fails with
    /// [`RuntimeError::UpdateLimit`].
    ///
    /// ```
    /// # use pert_ui_runtime as pert_ui;
    /// use pert_ui::*;
    /// use std::time::Duration;
    /// struct Board { item: Vec<Element>, at: (f32, f32), drag: Option<(Point, (f32, f32))> }
    /// impl Component for Board {
    ///     fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> { Ok(()) }
    ///     fn take_children(&mut self) -> Vec<Element> { std::mem::take(&mut self.item) }
    ///     fn arrange(&self, cx: &mut ArrangeContext<'_>, _: Size) -> Result<(), RuntimeError> {
    ///         cx.place(0, Rect::new(self.at.0, self.at.1, 40.0, 20.0)?)
    ///     }
    ///     fn pointer_input(&mut self, cx: &mut EventContext, event: PointerEvent)
    ///         -> Result<InputOutcome, RuntimeError> {
    ///         let Some(local) = cx.to_local(event.position()) else {
    ///             return Ok(InputOutcome::Ignored);
    ///         };
    ///         match event.kind() {
    ///             PointerKind::Press(PointerButton::Primary) => {
    ///                 cx.capture_pointer()?;
    ///                 self.drag = Some((local, self.at));
    ///             }
    ///             PointerKind::Move => {
    ///                 let Some((start, from)) = self.drag else { return Ok(InputOutcome::Ignored) };
    ///                 let at = (from.0 + local.x() - start.x(), from.1 + local.y() - start.y());
    ///                 // A repeated position derives the same offset: nothing to do.
    ///                 if at != self.at {
    ///                     self.at = at;
    ///                     cx.invalidate(Invalidation::Layout);
    ///                 }
    ///             }
    ///             PointerKind::Release(PointerButton::Primary) => {
    ///                 self.drag = None;
    ///                 cx.release_pointer();
    ///             }
    ///             _ => return Ok(InputOutcome::Ignored),
    ///         }
    ///         Ok(InputOutcome::Handled)
    ///     }
    /// }
    /// let mut ui = Runtime::new();
    /// ui.set_viewport(Viewport::new(200.0, 100.0, 1.0)?)?;
    /// let item = vec![Element::new("item", Label::new("Item"))];
    /// let board = ui.mount_tree(Element::new("board", Board { item, at: (0.0, 0.0), drag: None }))?;
    /// ui.flush(Duration::ZERO)?;
    /// let point = |x, y| Point::new(x, y);
    /// ui.pointer_input(PointerEvent::new(point(10.0, 10.0)?, PointerKind::Press(PointerButton::Primary)))?;
    /// ui.pointer_input(PointerEvent::new(point(60.0, 30.0)?, PointerKind::Move))?;
    /// ui.flush(Duration::ZERO)?; // settles: the repeated move changes nothing
    /// ui.pointer_input(PointerEvent::new(point(60.0, 30.0)?, PointerKind::Release(PointerButton::Primary)))?;
    /// let moved = ui.find_descendant(&board, "item")?.expect("the item");
    /// assert_eq!(ui.bounds(&moved)?.map(|b| (b.x(), b.y())), Some((50.0, 20.0)));
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// # Errors
    /// An implementation returns an error when handling the event fails. It propagates from
    /// [`Runtime::pointer_input`]; a component that held the pointer capture or a gesture
    /// then loses its input context through [`Self::cancel_input`]. The default never
    /// fails.
    fn pointer_input(
        &mut self,
        _: &mut EventContext,
        _: crate::PointerEvent,
    ) -> Result<crate::InputOutcome, RuntimeError> {
        Ok(crate::InputOutcome::Ignored)
    }
    /// Handles keyboard input with explicit propagation.
    ///
    /// Keys reach the focused component first and bubble through its ancestors
    /// until one returns `Handled`; a key nobody handles goes on to View, Window
    /// and Application commands. Return `Handled` only for a key this component
    /// acted on, and `Ignored` for the rest, so Escape, shortcuts and traversal
    /// keep working around it.
    ///
    /// The default calls [`Self::key_input`] and returns `Ignored`, so a
    /// component that only observes keys never swallows them. A component that
    /// consumes keys overrides this method and returns `Handled` for them.
    /// Use [`KeyboardState::control_key`](crate::KeyboardState::control_key)
    /// to interpret named actions such as Enter or navigation from the numpad;
    /// keep [`KeyEvent::code`](crate::KeyEvent::code) for gesture ownership and
    /// matching releases so simultaneous copies remain distinct.
    ///
    /// The runtime does not repaint for a key event: invalidate through the
    /// context when the key changes presentation.
    ///
    /// # Errors
    /// An implementation returns an error when handling the key fails; it propagates from
    /// [`Runtime::key_input`] and the key stops bubbling. The default forwards the error of
    /// [`Self::key_input`].
    fn keyboard_event(
        &mut self,
        cx: &mut EventContext,
        event: crate::KeyEvent,
        keyboard: &crate::KeyboardState,
    ) -> Result<crate::InputOutcome, RuntimeError> {
        self.key_input(cx, event, keyboard)?;
        Ok(crate::InputOutcome::Ignored)
    }

    /// Whether descendants clip to this allocation (default true).
    /// Flow/view/group containers return false so reported overflow can reach an
    /// enclosing scroll viewport. Their own drawing/hit box still clips to their
    /// allocation. Scroll viewports and explicit decorated Containers retain clipping.
    fn clips_children(&self) -> bool {
        true
    }
    /// Controls presentation of an immediate child by its stable sibling key.
    /// Hidden descendants retain state and suspend work. Defaults to true.
    /// Read bindings during update, then return retained state here; changes require
    /// layout invalidation. This combines with explicit runtime/ancestor visibility.
    fn child_visible(&self, _key: &str) -> bool {
        true
    }
    /// Whether this component can reveal clipped descendants by scrolling.
    /// Default is false. Automatic focus traversal includes clipped descendants
    /// only where an ancestor supports this public contract.
    fn can_reveal(&self) -> bool {
        false
    }
    /// Reveals a descendant's current window-coordinate rectangle, returning whether
    /// its placement changed. Called nearest ancestor first during focus transfer.
    /// Implementations clamp scrolling to content bounds and invalidate layout when
    /// returning true. Runtime lays out before asking the next ancestor.
    ///
    /// # Errors
    /// An implementation returns an error when scrolling fails, such as a rejected geometry
    /// value. It propagates from the focus operation that asked, such as
    /// [`Runtime::focus`], after focus has moved. The default never fails.
    fn reveal(&mut self, _cx: &mut EventContext, _rect: crate::Rect) -> Result<bool, RuntimeError> {
        Ok(false)
    }
    /// Opens the way to the immediate, mounted child `key` that
    /// [`Self::child_visible`] currently hides, for [`Runtime::reveal`],
    /// [`Runtime::reveal_and_focus`], [`EventContext::request_reveal`],
    /// [`EventContext::request_reveal_and_focus`] and [`Runtime::reveal_child`].
    ///
    /// The runtime calls it in an event phase, never during update or layout, and
    /// only for a child this component hides. A container that hides children in
    /// state it owns changes that state through the same transition its own
    /// input uses, so workspace capture and inspection see an ordinary change:
    /// `Tabs` selects the tab, a collapsible section expands. It then returns
    /// [`ChildReveal::Changed`](crate::ChildReveal::Changed), after which `child_visible(key)` must return
    /// true; the runtime applies the visibility, settles update and layout, and
    /// continues with the next container on the path. Invalidate as for any
    /// other state change. The hook must not move focus or scroll: focus is
    /// [`Runtime::reveal_and_focus`]'s choice, and scrolling follows through
    /// [`Self::reveal`].
    ///
    /// Return [`ChildReveal::ApplicationControlled`](crate::ChildReveal::ApplicationControlled) when the application owns
    /// the child's visibility, as for a `View` bound with `visible_signal`, and
    /// [`ChildReveal::Refused`](crate::ChildReveal::Refused) when the container will not show it. The default
    /// refuses, so a custom container opts in through this public hook exactly
    /// as the built-in ones do. [`ChildReveal::AlreadyVisible`](crate::ChildReveal::AlreadyVisible) reports a child
    /// that needs no change, such as a tab that lives in its own native window.
    ///
    /// # Errors
    /// An implementation returns an error when the change fails; it propagates from
    /// the reveal operation that asked, and containers opened before stay open.
    /// The default never fails.
    fn reveal_child(
        &mut self,
        _cx: &mut EventContext,
        _key: &str,
    ) -> Result<crate::ChildReveal, RuntimeError> {
        Ok(crate::ChildReveal::Refused)
    }
    /// Delivers the change notifications this component queued, after it asked
    /// for this call through a [`Notifier`](crate::Notifier).
    ///
    /// State that the user, a reveal, workspace restoration or a new
    /// description changes is often changed where no [`EventContext`] exists:
    /// [`Self::restore_workspace_state_with_children`] and [`Self::update`] cannot run
    /// application callbacks. A component that reports such changes records
    /// them, calls [`Notifier::request`](crate::Notifier::request), and runs its
    /// callbacks here, in an event phase, in the order it recorded them. The
    /// runtime calls this at the next event-phase point of the flush that
    /// follows the request, or of the flush in progress: after that flush's
    /// update and layout waves have settled, so the callbacks see the changed
    /// state and may write bindings, which the same flush then settles.
    ///
    /// Each request is one call, in the order of all components' requests, to
    /// a mounted component across visible presentation windows. Hidden native
    /// windows suspend requests until shown, without blocking visible windows;
    /// hidden components inside a visible window still receive callbacks. A
    /// component that requests once per change and delivers its oldest change
    /// per call, as `Tabs` does, keeps changes of several components in the
    /// order they happened. A component
    /// removed before delivery is not called. Requests made during delivery are
    /// delivered at the next settling wave of the same flush or in the next
    /// flush. The change itself is never undone by a failing callback.
    ///
    /// The default does nothing; `Tabs` uses this hook for
    /// [`Tabs::on_select`](crate::Tabs::on_select), through the same public
    /// mechanism a custom component uses.
    ///
    /// # Errors
    /// An implementation consumes the notification belonging to this request
    /// even when its callback fails, and returns that callback error. The runtime
    /// continues the remaining ready requests in the captured frontier, settles
    /// callback writes across visible windows, then returns the first error.
    /// A presentation-preparation failure retains the undelivered request and
    /// remainder for a later flush. The default never fails.
    fn deliver_notifications(&mut self, _cx: &mut EventContext) -> Result<(), RuntimeError> {
        Ok(())
    }

    /// Whether a [`Row`](crate::Row) stretches this component to its content height.
    ///
    /// Default is `false`: the row measures with the parent's offered height.
    /// Return `true` for a separator or other decoration that should contribute
    /// its unbounded natural height first, then fill the tallest row content,
    /// capped by the offered height. Hard minimums still apply. Vertical
    /// [`Divider`](crate::Divider)s use this same public extension point.
    /// The query reads retained properties during layout; changing the answer
    /// requires layout invalidation. It must not write state or schedule work.
    /// Opting in requests two measurement constraints in each row layout pass,
    /// for both measurement and arrangement. The runtime keeps one measurement
    /// cache per component, so implementations should keep measurement cheap
    /// and retain expensive preparation by its actual inputs. Ordinary siblings
    /// keep their unchanged constraints and caches; a settled row schedules no
    /// work through this preference.
    /// Other containers can read this preference through
    /// [`MeasureContext::fills_row_height`](crate::MeasureContext::fills_row_height).
    ///
    /// ```
    /// # use pert_ui_runtime as pert_ui;
    /// use pert_ui::{Component, Constraints, Container, Element, MeasureContext,
    ///     Row, Runtime, RuntimeError, Size, UpdateContext, Viewport};
    /// use std::time::Duration;
    /// struct Separator;
    /// impl Component for Separator {
    ///     fn fills_row_height(&self) -> bool { true }
    ///     fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> { Ok(()) }
    ///     fn measure(&self, _: &MeasureContext<'_>, space: Constraints) -> Result<Size, RuntimeError> {
    ///         Ok(Size::new(1.0, space.height().unwrap_or(32.0))?)
    ///     }
    /// }
    /// let mut ui = Runtime::new();
    /// ui.set_viewport(Viewport::new(300.0, 200.0, 1.0)?)?;
    /// let row = ui.mount_tree(Element::new("row", Row::new([
    ///     Element::new("content", Container::default().preferred_size(Size::from([40, 60]))),
    ///     Element::new("separator", Separator),
    /// ])))?;
    /// ui.flush(Duration::ZERO)?;
    /// let separator = ui.find_descendant(&row, "separator")?.unwrap();
    /// assert_eq!(ui.bounds(&separator)?.unwrap().height(), 60.0);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    fn fills_row_height(&self) -> bool {
        false
    }

    /// Hard outer limits applied to measurement and final allocation.
    /// Default permits any finite nonnegative size. Use retained values only;
    /// changes require layout invalidation. Overflow is clipped, never hidden by
    /// shrinking below a declared minimum. Spare space after a maximum stays empty.
    fn size_limits(&self) -> crate::SizeLimits {
        crate::SizeLimits::default()
    }
    /// Derives hard limits from visible children during layout. Default uses
    /// `size_limits`; composite controls override this to propagate descendant
    /// minimums before a parent allocates/clips them. Use the public child limits,
    /// retained state and validated geometry only.
    ///
    /// # Errors
    /// An implementation returns an error when its limits cannot be derived, such as a
    /// child measurement failure from the context or invalid geometry. Layout stays pending
    /// and the error propagates from [`Runtime::flush`]. The default never fails.
    fn measure_limits(
        &self,
        _cx: &crate::MeasureContext<'_>,
    ) -> Result<crate::SizeLimits, RuntimeError> {
        Ok(self.size_limits())
    }
    /// Keeps constructor-generated children when this component is described again.
    ///
    /// The default is `false`: incoming descendants are reconciled by key and
    /// type. Return `true` for a component whose initial parts belong to its
    /// retained controller. Initial children from [`Self::take_children`] mount
    /// normally, including while hidden. A compatible ancestor redescription
    /// applies this component's properties but discards its incoming descendants
    /// before their validation, metadata changes or mounting. Existing children
    /// keep their identities, bindings and state.
    ///
    /// The runtime captures this policy at mount; it cannot change during that
    /// incarnation. This query must be cheap, perform no I/O and mutate no
    /// application state. Implementations must reconcile replaceable properties
    /// themselves and regenerate parts from retained state through
    /// [`UpdateContext::request_children`]. Direct calls to
    /// [`Runtime::reconcile_children`] on this component still explicitly replace
    /// its children. Ordinary components that have requested children continue
    /// to reject incoming descendants during ancestor redescription.
    ///
    /// ```
    /// # use pert_ui_runtime as pert_ui;
    /// use pert_ui::{Column, Component, Element, Label, Runtime,
    ///     RuntimeError, UpdateContext};
    /// struct OwnedParts(Vec<Element>);
    /// impl Component for OwnedParts {
    ///     fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
    ///         Ok(())
    ///     }
    ///     fn retain_initial_children(&self) -> bool { true }
    ///     fn take_children(&mut self) -> Vec<Element> { std::mem::take(&mut self.0) }
    /// }
    /// let mut ui = Runtime::new();
    /// let root = ui.mount_tree(Element::new("root", Column::new([
    ///     Element::new("owner", OwnedParts(vec![
    ///         Element::new("part", Label::new("Mounted content")),
    ///     ])),
    /// ])))?;
    /// let part = ui.find_descendant(&root, "part")?.unwrap();
    /// ui.reconcile_children(&root, vec![Element::new("owner", OwnedParts(vec![
    ///     Element::new("part", Label::new("Ignored incoming content")),
    /// ]))])?;
    /// assert_eq!(ui.find_descendant(&root, "part")?, Some(part));
    /// # Ok::<(), RuntimeError>(())
    /// ```
    fn retain_initial_children(&self) -> bool {
        false
    }

    /// Transfers initial child descriptions into the common retained tree.
    ///
    /// Called once by `Element::new`, before mounting. Container builders keep
    /// their initial children until this call; subsequent structure changes use
    /// `Runtime::reconcile_children`. Default describes a leaf. This callback
    /// must not perform I/O or mutate application state.
    fn take_children(&mut self) -> Vec<crate::Element> {
        Vec::new()
    }
    /// Measures content in logical pixels under the available space.
    ///
    /// Default reports the largest child extent, or zero for a leaf. Implementations
    /// must use retained data, return validated finite sizes, and not write state.
    /// Measurements can exceed available space to preserve intrinsic minimums.
    ///
    /// # Errors
    /// An implementation returns an error when measuring fails; layout stays pending and
    /// the error propagates from [`Runtime::flush`]. The default forwards the errors of
    /// [`MeasureContext::measure`](crate::MeasureContext::measure) and returns
    /// [`RuntimeError::Geometry`] when the largest child extent is not a valid
    /// [`Size`](crate::Size).
    fn measure(
        &self,
        cx: &crate::MeasureContext<'_>,
        available: crate::Constraints,
    ) -> Result<crate::Size, RuntimeError> {
        let mut width: f32 = 0.0;
        let mut height: f32 = 0.0;
        for index in 0..cx.len() {
            let size = cx.measure(index, available)?;
            width = width.max(size.width());
            height = height.max(size.height());
        }
        Ok(crate::Size::new(width, height)?)
    }
    /// Places every visible child once, relative to this component's origin.
    ///
    /// Default fills the allocation with each child in order. Custom containers
    /// use the same public context as built-ins. No state writes or I/O are allowed.
    /// A component whose size, visible children and layout are unchanged is not
    /// arranged again when only its window position, view scale or ancestor clip
    /// changes, as scrolling, panning or a continuous zoom moves it: a leaf keeps
    /// its local arrangement and a container keeps the placements it gave its
    /// children, whose window geometry the runtime derives again. So an
    /// arrangement must depend only on the size, the measurement context and
    /// retained state; change retained state from an event or update and
    /// invalidate layout. Layout invalidation, including relevant font, scale or
    /// metric changes, arranges the component again.
    ///
    /// # Errors
    /// An implementation returns an error when it cannot place its children; layout stays
    /// pending and the error propagates from [`Runtime::flush`]. A child left unplaced is
    /// [`LayoutError::InvalidChild`](crate::LayoutError::InvalidChild). The default
    /// forwards the errors of [`ArrangeContext::place`](crate::ArrangeContext::place).
    fn arrange(
        &self,
        cx: &mut crate::ArrangeContext<'_>,
        size: crate::Size,
    ) -> Result<(), RuntimeError> {
        for index in 0..cx.len() {
            cx.place(index, crate::Rect::from_size(size))?;
        }
        Ok(())
    }

    /// Captures this component's restorable arrangement, or `None` to stay out
    /// of [workspace persistence](crate::Workspace) entirely, which is the
    /// default and right for almost every component.
    ///
    /// Return preferences the user arranged — a selected key, a proportion, an
    /// offset — never application data, focus, capture, gestures or drafts, and
    /// never a measured result: a restore writes the preference back and the
    /// next layout clamps it against the viewport it then has.
    ///
    /// The [`ViewState`](crate::ViewState) carries a short stable kind tag the
    /// component owns, such as `tabs`. It is matched against the document before
    /// anything is offered back, so a component of another kind at the same path
    /// is reported rather than misapplied. Called on the UI thread on explicit
    /// demand, never on a frame or timer; signal and parameter writes are
    /// rejected in this phase.
    ///
    /// Runtime invokes [`Self::workspace_state_with_children`], whose default
    /// delegates here. Child-dependent participants override that hook.
    ///
    /// [`Runtime::capture_workspace`] and [`Runtime::restore_workspace`] ask
    /// each component once per call and use that one answer both to decide
    /// whether it takes part and as its state, so an answer that would change
    /// between calls cannot leave a successful capture incomplete. A view that
    /// mounts later and takes a retained entry is asked once at that point.
    ///
    /// # Errors
    /// Returns the [`WorkspaceError`](crate::WorkspaceError) of the state it
    /// could not build, such as a key too long for workspace text or a signal
    /// that could not be read. Never turn such a failure into `None`: that
    /// would make a capture report success without this component. The
    /// failure fails the whole capture or restore with
    /// [`WorkspaceError::Participant`](crate::WorkspaceError::Participant),
    /// which names this component's path, so the application can fix the key
    /// or leave the component out of persistence.
    ///
    /// ```
    /// # use pert_ui_runtime as pert_ui;
    /// use pert_ui::{
    ///     Component, Element, Invalidation, RestoreRejection, Runtime, RuntimeError,
    ///     UpdateContext, ViewState, WorkspaceError, WorkspaceValue,
    /// };
    /// /// Remembers which of its named modes the user picked.
    /// struct ModePicker {
    ///     mode: String,
    /// }
    /// impl Component for ModePicker {
    ///     fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
    ///         Ok(())
    ///     }
    ///     fn workspace_state(&self) -> Result<Option<ViewState>, WorkspaceError> {
    ///         // `?` passes a mode name the document cannot hold on to the capture.
    ///         Ok(Some(
    ///             ViewState::new("mode-picker")?.with("mode", WorkspaceValue::text(&self.mode)?)?,
    ///         ))
    ///     }
    ///     fn restore_workspace_state(
    ///         &mut self,
    ///         state: &ViewState,
    ///     ) -> Result<Invalidation, RestoreRejection> {
    ///         state.expect_only(&["mode"])?;
    ///         self.mode = state.text("mode")?.to_owned();
    ///         Ok(Invalidation::Paint)
    ///     }
    /// }
    /// let mut ui = Runtime::new();
    /// ui.mount_tree(Element::new("picker", ModePicker { mode: "wide".into() }))?;
    /// assert_eq!(ui.capture_workspace()?.len(), 1);
    ///
    /// // A mode the document cannot hold fails the capture and names the picker.
    /// let mut ui = Runtime::new();
    /// ui.mount_tree(Element::new("picker", ModePicker { mode: "a\tb".into() }))?;
    /// let Err(WorkspaceError::Participant { path, .. }) = ui.capture_workspace() else {
    ///     panic!("the capture should fail");
    /// };
    /// assert_eq!(path, "picker");
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    fn workspace_state(&self) -> Result<Option<crate::ViewState>, crate::WorkspaceError> {
        Ok(None)
    }
    /// Captures arrangement preferences against the current mounted children.
    ///
    /// Runtime calls this hook on the UI thread under the inspection phase,
    /// without flushing updates, measuring children or dispatching events. The
    /// default delegates to [`Self::workspace_state`]. Override it when saved
    /// preferences depend on child identities. Snapshots include hidden and
    /// detached children in mounted order; use their current identities rather
    /// than cached layout rows. Handles are UI-local and must not be retained
    /// for another thread. Wrappers must forward this hook for child-dependent
    /// participants. See [`Self::restore_workspace_state_with_children`] for a
    /// complete public capture and restore example.
    ///
    /// # Errors
    /// As [`Self::workspace_state`]. Never conceal a failed capture as `None`.
    fn workspace_state_with_children(
        &self,
        _children: &[ChildInfo],
    ) -> Result<Option<crate::ViewState>, crate::WorkspaceError> {
        self.workspace_state()
    }
    /// Applies captured state this component produced earlier, returning the
    /// narrowest invalidation the change needs.
    ///
    /// The runtime invokes [`Self::restore_workspace_state_with_children`], whose
    /// default delegates here. Wrappers around child-dependent participants must
    /// forward that hook as well, so admission sees current mounted identities.
    ///
    /// Only called when [`workspace_state`](Self::workspace_state) returned a
    /// matching kind, so an implementation may expect its own fields. Validate
    /// before writing anything: a rejection must leave the component unchanged,
    /// and it is reported rather than silently dropped.
    /// [`ViewState::expect_only`](crate::ViewState::expect_only) keeps the field
    /// set closed, so a document from a newer build is refused instead of
    /// applied in part. The default refuses everything; participants override
    /// this hook or [`Self::restore_workspace_state_with_children`].
    ///
    /// # Errors
    /// Returns a [`RestoreRejection`](crate::RestoreRejection) for a missing,
    /// unknown, unresolvable or out-of-range value.
    fn restore_workspace_state(
        &mut self,
        _state: &crate::ViewState,
    ) -> Result<Invalidation, crate::RestoreRejection> {
        Err(crate::RestoreRejection::Unsupported)
    }
    /// Restores state using the current mounted children, before a pending update.
    ///
    /// The runtime calls this after checking the participant's kind. The default
    /// delegates to [`Self::restore_workspace_state`]. Override it when admission
    /// depends on child identities: these snapshots reflect committed child
    /// reconciliation even when this component's retained layout is still older.
    /// They include hidden and detached children in mounted order. Handles are
    /// UI-local; do not retain them for use from another thread. This callback
    /// neither measures children nor runs application events. Validate all state
    /// against the snapshot before changing local preferences. Apply arrangement
    /// preferences without dispatching input or measuring children.
    ///
    /// # Errors
    /// As [`Self::restore_workspace_state`]. A rejection leaves local state unchanged.
    ///
    /// ```
    /// # use pert_ui_runtime as pert_ui;
    /// use pert_ui::{
    ///     ChildInfo, Component, Element, Invalidation, RestoreRejection, Runtime,
    ///     Label, RuntimeError, UpdateContext, ViewState, WorkspaceError, WorkspaceValue,
    /// };
    /// struct Selection(String);
    /// impl Component for Selection {
    ///     fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
    ///         Ok(())
    ///     }
    ///     fn workspace_state(&self) -> Result<Option<ViewState>, WorkspaceError> {
    ///         Ok(Some(ViewState::new("selection")?
    ///             .with("selected", WorkspaceValue::text(&self.0)?)?))
    ///     }
    ///     fn workspace_state_with_children(
    ///         &self, children: &[ChildInfo],
    ///     ) -> Result<Option<ViewState>, WorkspaceError> {
    ///         let selected = children.iter().find(|child| child.key() == self.0)
    ///             .or_else(|| children.first());
    ///         let state = ViewState::new("selection")?;
    ///         Ok(Some(match selected {
    ///             Some(child) => state.with("selected", WorkspaceValue::text(child.key())?)?,
    ///             None => state,
    ///         }))
    ///     }
    ///     fn restore_workspace_state_with_children(
    ///         &mut self, state: &ViewState, children: &[ChildInfo],
    ///     ) -> Result<Invalidation, RestoreRejection> {
    ///         state.expect_only(&["selected"])?;
    ///         if children.is_empty() && state.get("selected").is_none() {
    ///             self.0.clear();
    ///             return Ok(Invalidation::Layout);
    ///         }
    ///         let selected = state.text("selected")?;
    ///         if !children.iter().any(|child| child.key() == selected) {
    ///             return Err(RestoreRejection::Unresolved(selected.to_owned()));
    ///         }
    ///         self.0 = selected.to_owned();
    ///         Ok(Invalidation::Layout)
    ///     }
    /// }
    /// let mut ui = Runtime::new();
    /// let parent = ui.mount_tree(Element::new("picker", Selection("old".into())))?;
    /// ui.reconcile_children(&parent, vec![Element::new("new", Label::new("New"))])?;
    /// // Capture and restore use the reconciled child without a flush.
    /// let saved = ui.capture_workspace()?;
    /// assert_eq!(saved.entries().next().unwrap().1.text("selected")?, "new");
    /// assert!(ui.restore_workspace(&saved)?.is_complete());
    /// ui.reconcile_children(&parent, vec![])?;
    /// assert!(ui.restore_workspace(&saved)?.rejected().any(|(_, reason)|
    ///     *reason == RestoreRejection::Unresolved("new".into())));
    /// let empty = ui.capture_workspace()?;
    /// assert!(empty.entries().next().unwrap().1.get("selected").is_none());
    /// assert!(ui.restore_workspace(&empty)?.is_complete());
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    fn restore_workspace_state_with_children(
        &mut self,
        state: &crate::ViewState,
        _children: &[ChildInfo],
    ) -> Result<Invalidation, crate::RestoreRejection> {
        self.restore_workspace_state(state)
    }
    /// Returns explicitly exposed state and actual measured box/style information.
    ///
    /// Called on demand during inspection. Development builds also collect this
    /// after this component updates, to validate measured layout/input metadata.
    /// Release flushing never calls this method automatically.
    /// Do not mutate application state or perform I/O. Signal/parameter writes
    /// are rejected in this phase. Unknown geometry must remain absent.
    fn inspect(&self) -> crate::ComponentDetails {
        crate::ComponentDetails::default()
    }
    /// Searchable rows this collection offers to
    /// [content search](Runtime::content_search) beyond its mounted children.
    ///
    /// Defaults to `None`: only mounted components are searched, through their
    /// semantics. `VirtualList` returns its rows, so `Table` and `Tree` offer
    /// theirs; a custom collection returns its own [`ContentRows`](crate::ContentRows).
    /// Called during a search under the inspection phase: answer from retained
    /// data, write no signal and perform no I/O. Runtime calls
    /// [`Self::try_content_rows`], whose default delegates here; collections
    /// with fallible reads override that method as well. VirtualList and Tree
    /// keep this optional hook for compatibility, returning `None` when rows
    /// cannot be read; use the fallible hook to distinguish that failure from
    /// a component that offers no rows.
    ///
    /// The returned rows are `'static`, like the component: they own their data
    /// or share it through an `Rc` that the component also holds. Rows that
    /// borrow from `&self` do not compile, and the `+ '_` bound rustc suggests
    /// then does not match this signature. Keep the data in an
    /// `Rc<RefCell<..>>` field and return rows holding a clone of that `Rc`.
    fn content_rows(&self) -> Option<Rc<dyn crate::ContentRows>> {
        None
    }
    /// Returns current searchable collection rows with read failures preserved.
    ///
    /// Content search and row jumps call this method. The default delegates to
    /// [`Self::content_rows`], preserving existing custom collections. Override
    /// this method when producing rows can fail, for example when a dirty
    /// derived binding cannot evaluate during inspection. Return `Ok(None)`
    /// only when the component offers no rows, never to conceal a read failure.
    /// Rows are owned UI-local handles retained by a search until it is dropped.
    /// Inspection must not evaluate dirty bindings, mutate state or perform I/O.
    ///
    /// ```
    /// # use pert_ui_runtime as pert_ui;
    /// use pert_ui::{Component, Element, ListItems, ListSource, Runtime, VirtualList};
    /// struct EmptyRows;
    /// impl ListItems for EmptyRows {
    ///     fn len(&self) -> usize { 0 }
    ///     fn key(&self, _: usize) -> Option<String> { None }
    ///     fn build(&self, _: usize) -> Option<Element> { None }
    /// }
    /// let ui = Runtime::new();
    /// let source = ui.signal(ListSource::new(EmptyRows));
    /// let list = VirtualList::new(source.read_only(), 24.0)?;
    /// let rows = list.try_content_rows()?.expect("a list offers its source");
    /// assert!(rows.is_empty());
    /// # Ok::<(), pert_ui::RuntimeError>(())
    /// ```
    ///
    /// # Errors
    /// Return the underlying read or state failure. Search forwards it as
    /// [`ContentSearchError::Runtime`](crate::ContentSearchError::Runtime);
    /// row jumps propagate it as [`RuntimeError`]. The default never fails.
    fn try_content_rows(&self) -> Result<Option<Rc<dyn crate::ContentRows>>, RuntimeError> {
        Ok(self.content_rows())
    }
    /// Records backend-independent drawing from retained data after an update.
    ///
    /// Defaults to no drawing. Unchanged components reuse their commands, even
    /// when another component animates or the OS requests the window again.
    /// Composed components draw locally within `painter.size()`; legacy roots
    /// continue to draw in window coordinates using `painter.viewport()`.
    /// Signal/parameter writes are rejected.
    ///
    /// # Errors
    /// An implementation returns a [`DrawError`](crate::DrawError), usually forwarded from
    /// a [`Painter`](crate::Painter) method; it propagates from [`Runtime::paint`]. The
    /// previous commands remain cached and the component stays pending for a later paint
    /// retry. The default never fails.
    fn paint(&self, _painter: &mut crate::Painter) -> Result<(), crate::DrawError> {
        Ok(())
    }
    /// Records decorations above this component's children, inside the same clip.
    /// Uses the same retained cache and restrictions as `paint`. Default draws nothing.
    /// Useful for scrollbars and focus outlines. Use `hit_test_foreground` to make
    /// foreground decorations interactive above children.
    ///
    /// # Errors
    /// As [`Self::paint`]. The default never fails.
    fn paint_foreground(&self, _painter: &mut crate::Painter) -> Result<(), crate::DrawError> {
        Ok(())
    }
    /// Tests this component's body at a component-local point in logical pixels.
    ///
    /// Default returns true throughout the already-clipped body. Return false
    /// over transparent gaps to let hit testing continue to lower drawing entries.
    /// This does not exclude children or interactive foreground decorations:
    /// they are tested independently in their drawing order. Ancestor clips,
    /// visibility and modal restrictions still apply. Pointer capture continues
    /// delivering events even outside the body. The result controls ordinary
    /// pointer input, hover, focus, cursor and drag-target selection. Surface
    /// outside-press dismissal uses the surface root's clipped bounding rectangle,
    /// including transparent body gaps; this query does not change its extent.
    ///
    /// This UI-thread query must be read-only: do not write signals, schedule
    /// work or mutate retained state. Use current arranged local geometry.
    ///
    /// ```
    /// # use pert_ui_runtime as pert_ui;
    /// use pert_ui::*;
    /// struct Hollow;
    /// impl Component for Hollow {
    ///     fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
    ///         Ok(())
    ///     }
    ///     fn hit_test(&self, point: Point) -> bool {
    ///         point.x() < 20.0
    ///     }
    ///     fn paint(&self, painter: &mut Painter) -> Result<(), DrawError> {
    ///         painter.fill_rect(Rect::new(0.0, 0.0, 20.0, 80.0)?, Color::ACCENT);
    ///         Ok(())
    ///     }
    /// }
    /// let mut runtime = Runtime::new();
    /// runtime.mount_tree(Element::new("hollow", Hollow))?;
    /// # Ok::<(), RuntimeError>(())
    /// ```
    fn hit_test(&self, _point: crate::Point) -> bool {
        true
    }
    /// Tests an interactive foreground decoration above this component's children.
    ///
    /// The point uses component-local logical pixels. Default returns false, so
    /// ordinary child hit testing is unchanged. Return true only over foreground
    /// controls drawn by [`Self::paint_foreground`]; those hits route pointer input,
    /// focus and cursor queries to this component before its children. Ancestor
    /// clipping, surfaces and modality still apply. This UI-thread query must be
    /// read-only, without signal writes, scheduling or changes to retained state.
    ///
    /// ```
    /// # use pert_ui_runtime as pert_ui;
    /// use pert_ui::*;
    /// struct Overlay;
    /// impl Component for Overlay {
    ///     fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
    ///         Ok(())
    ///     }
    ///     fn hit_test_foreground(&self, point: Point) -> bool {
    ///         (0.0..20.0).contains(&point.x()) && (0.0..20.0).contains(&point.y())
    ///     }
    ///     fn paint_foreground(&self, painter: &mut Painter) -> Result<(), DrawError> {
    ///         painter.fill_rect(Rect::new(0.0, 0.0, 20.0, 20.0)?, Color::ACCENT);
    ///         Ok(())
    ///     }
    ///     fn pointer_input(&mut self, _: &mut EventContext, _: PointerEvent)
    ///         -> Result<InputOutcome, RuntimeError> {
    ///         Ok(InputOutcome::Handled)
    ///     }
    /// }
    /// let mut runtime = Runtime::new();
    /// runtime.mount_tree(Element::new("overlay", Overlay))?;
    /// # Ok::<(), RuntimeError>(())
    /// ```
    fn hit_test_foreground(&self, _point: crate::Point) -> bool {
        false
    }
    /// Called once after a successful mount, before the first update.
    fn mounted(&mut self) {}
    /// Observes a physical key event after the window's pressed-key set is updated.
    ///
    /// Called by the default [`Self::keyboard_event`], which then lets the key
    /// bubble on to ancestors and commands; override `keyboard_event` instead
    /// to consume a key. All held keys are available together; ignore
    /// `Repeated` for one-shot actions. Signal/parameter writes are allowed
    /// here.
    ///
    /// # Errors
    /// An implementation returns an error when handling the key fails, such as a rejected
    /// binding write; it propagates from [`Runtime::key_input`]. The default never fails.
    fn key_input(
        &mut self,
        _context: &mut EventContext,
        _event: crate::KeyEvent,
        _keyboard: &crate::KeyboardState,
    ) -> Result<(), RuntimeError> {
        Ok(())
    }
    /// Returns a local IME candidate anchor for an editable text recipient.
    /// Default disables native composition. Called read-only after layout/paint;
    /// implementations must use their current caret and never mutate bindings.
    /// Advance [`TextInputState::session`](crate::TextInputState::session) when
    /// canceling composition, replacing bound text or repositioning its caret
    /// during composition. Never reuse a session; return `None` on exhaustion.
    fn text_input_state(&self) -> Option<crate::TextInputState> {
        None
    }
    /// Receives temporary composition without committing it to the text binding.
    /// Empty preedit cancels composition. Clear temporary state on context loss.
    ///
    /// # Errors
    /// An implementation returns an error when composition cannot be applied; it propagates
    /// from [`Runtime::text_preedit`] and physical key state remains unchanged. The default
    /// never fails.
    fn text_preedit(
        &mut self,
        _cx: &mut EventContext,
        _preedit: &crate::TextPreedit,
    ) -> Result<(), RuntimeError> {
        Ok(())
    }
    /// Receives committed UTF-8 text independently of physical keys and repeats.
    ///
    /// The native adapter delivers composed text here after preedit.
    ///  This callback does not change held keys.
    ///
    /// # Errors
    /// An implementation returns an error when the text cannot be committed, such as text
    /// beyond its limit or a rejected binding write; it propagates from
    /// [`Runtime::text_input`]. The default never fails.
    fn text_input(&mut self, _context: &mut EventContext, _text: &str) -> Result<(), RuntimeError> {
        Ok(())
    }
    /// Offers a drag session to this component, or tells it how its part ended.
    ///
    /// Default rejects, so a component is a drop target only by overriding this.
    /// While a session is active the runtime hit-tests the pointer with the
    /// ordinary rules (effective clip, drawing order, foreground regions, modal
    /// admission) although the source holds the capture, and asks each component
    /// from the hit one towards the root until one accepts. See
    /// [`DragPhase`](crate::DragPhase) for the four phases and
    /// [`DragRole`](crate::DragRole) for ancestors that are only told.
    ///
    /// - Read the payload with [`DragPayload::downcast_ref`](crate::DragPayload::downcast_ref) and
    ///   reject a type you do not recognize.
    /// - Accept with [`DragEvent::preferred`](crate::DragEvent::preferred) unless
    ///   the target has its own rule; an operation the source does not permit is
    ///   a `DropNotPermitted` error.
    /// - Compute the drop position from the latest event and the current layout.
    ///   `Over` is coalesced like pointer motion, and the collection may have
    ///   changed since the previous one; express the position against the
    ///   application's stable keys, never an index.
    /// - Answer `Rejected` for what is not yours, so an ancestor is asked, and
    ///   `Forbidden` to claim the drag and refuse it, which shows the no-drop
    ///   cursor. The target may change its answer on any `Over`; rejecting then
    ///   gives the drag up.
    /// - A component that claimed the drag receives exactly one `Leave` or `Drop`
    ///   afterwards, unless it is unmounted first. A new target's `Enter` may
    ///   precede the previous target's `Leave`; both arrive before any painting.
    /// - On `Drop`, perform the application edit now. **A move is one edit made
    ///   here**; the source does not delete what this target inserted. Answer
    ///   `Rejected` when the application refuses, and the source is told so.
    /// - The source may be its own target.
    ///
    /// Invalidate through the context when the indication changes. An observer
    /// that scrolls at its edge arms [`EventContext::set_timer`] and cancels it on
    /// `Leave`.
    ///
    /// # Errors
    /// An implementation returns an error when handling the phase fails, such as a failed
    /// application edit on `Drop`. Accepting an operation the source does not permit is
    /// [`RuntimeError::DropNotPermitted`], which the runtime raises itself. An error
    /// cancels the session, tells every participant, and propagates from the input call
    /// that delivered it. During shutdown, only the first error per cancelled
    /// operation is retained for [`Runtime::take_shutdown_errors`], whether from
    /// target, observer, preview or source. The default never fails.
    fn drag_event(
        &mut self,
        _cx: &mut EventContext,
        _event: &crate::DragEvent<'_>,
    ) -> Result<crate::DropResponse, RuntimeError> {
        Ok(crate::DropResponse::Rejected)
    }
    /// Tells the source how an accepted drag request or adopted session ended.
    /// Called exactly once while the source is mounted, including cancellation
    /// before adoption. An adopted session notifies its target/observers first,
    /// then releases capture and unmounts the preview before notifying the source.
    ///
    /// Restore presentation here, such as an emptied slot. Do not edit
    /// application data: a completed move was one edit on the target's side.
    /// Default does nothing.
    ///
    /// # Errors
    /// An implementation returns an error when restoring its presentation fails; it
    /// propagates from the input call or [`Runtime::flush`] that ended the session.
    /// During shutdown, only the first error per cancelled operation is retained
    /// for [`Runtime::take_shutdown_errors`], whether from target, observer, preview
    /// or source. Cleanup still notifies every participant.
    /// The default never fails.
    fn drag_finished(
        &mut self,
        _cx: &mut EventContext,
        _outcome: crate::DragOutcome,
    ) -> Result<(), RuntimeError> {
        Ok(())
    }

    /// Reports when the frontmost pointer enters/leaves this logical subtree,
    /// including native focus loss. Default does nothing. Called at the runtime's
    /// delivered event time, independently of pressed-button gestures and pointer
    /// bubbling; use it for local hover timing, not host actions or I/O. Hiding
    /// and removal cancel context through `cancel_input` instead.
    fn hover_changed(&mut self, _cx: &mut EventContext, _hovered: bool) {}

    /// Receives focus after the runtime has installed this recipient, at the
    /// event's elapsed time. Default does nothing. Use this infallible UI-thread
    /// lifecycle notification for local input/timing state; do not perform host
    /// actions or I/O. Physical held keys are not replayed. Fallible state work
    /// belongs in a subsequent update or explicit input callback.
    fn focus_gained(&mut self, _cx: &mut EventContext) {}

    /// Ends focus on an in-window transfer or window focus loss, after the runtime
    /// has released this component's pointer capture, edit sessions and deadlines.
    /// A transfer into this component's own subtree, such as into an embedded
    /// [`DraftEditor`](crate::DraftEditor), keeps them: the component still owns
    /// the interaction it started.
    ///
    /// Default calls [`Self::cancel_input`]. Local editors may save a valid draft
    /// here using ordinary signals. Always release local input/composition even
    /// when returning an error.
    /// Hiding, removal, shutdown and other cancellation use `cancel_input`
    /// instead and must not implicitly save drafts. Escape's ordinary delivery
    /// calls neither; a surface it closes hides what it showed, which calls
    /// `cancel_input` (see [`Runtime::key_input`]). No host edit is committed by
    /// default. This UI-thread callback must not recursively dispatch runtime work.
    ///
    /// # Errors
    /// An implementation returns an error when saving its draft fails, after releasing its
    /// local input. Errors do not veto the focus change: [`Runtime::focus`] returns them
    /// after transferring focus, and the infallible [`Runtime::clear_focus`] queues them
    /// for later [`Runtime::flush`] calls, in order. The default never fails.
    fn focus_lost(&mut self, cx: &mut EventContext) -> Result<(), RuntimeError> {
        self.cancel_input(cx);
        Ok(())
    }
    /// Reports that keyboard focus entered (`within` true) or left (`within`
    /// false) this component's subtree while the component itself is not the
    /// focused recipient, which gets [`Self::focus_gained`] and
    /// [`Self::focus_lost`] instead. Default does nothing.
    ///
    /// It is how a composite control applies a focus policy to the whole
    /// subtree it owns: a row that embeds a [`DraftEditor`](crate::DraftEditor)
    /// saves or cancels its draft here when focus leaves the row, with its own
    /// `&mut self` and context, in which writes are allowed.
    ///
    /// Delivered on the same occasions as `focus_gained`/`focus_lost`: a focus
    /// transfer inside a window ([`Runtime::focus`], a requested focus, Tab, a
    /// pointer press), native window focus loss ([`Runtime::clear_focus`],
    /// which hiding a native window with [`Runtime::set_window_visible`] also
    /// performs), the focused component moving into a native window of its
    /// own, and [`Runtime::reparent`] moving a subtree that holds focus, which
    /// tells ancestors of the old position that lost focus-within, innermost
    /// first, then those of the new position that gained it, outermost first,
    /// and queues their errors for later [`Runtime::flush`] calls. On a transfer the order is the old recipient's `focus_lost`, then
    /// ancestors that lost focus from the innermost outwards, then ancestors
    /// that gained it from the outermost inwards, then the new recipient's
    /// `focus_gained`. On native focus loss an ancestor is told before its own
    /// input is cancelled, so its policy still sees the state it applies to.
    ///
    /// The runtime remembers, per native window, which ancestors it last told
    /// that focus is within them, and tells a component only when that
    /// changes. Moving focus between two descendants, or from a descendant to
    /// the component itself, tells it nothing.
    ///
    /// Cancellation paths end focus without this notification, as they do
    /// without `focus_lost`, and must not save drafts: hiding a component with
    /// [`Runtime::set_visible`] or its parent's visibility, removal, a modal
    /// surface taking focus, the focused component becoming non-focusable, and
    /// shutdown. An ancestor that such a path cancels, hides or removes is
    /// forgotten and is not told later. An ancestor it leaves alone keeps
    /// focus-within as far as notifications go: when focus returns inside it,
    /// as when a row takes focus back from an editor it just made
    /// non-focusable, it is told nothing; when focus next settles outside it,
    /// it is told `false` then. Notifications are therefore not guaranteed to
    /// alternate; read `within` rather than toggling local state.
    ///
    /// # Errors
    /// An implementation returns an error when applying its policy fails, such
    /// as a rejected binding write. Errors do not veto a focus transfer and do
    /// not stop the other notifications: [`Runtime::focus`] returns the first
    /// error, from `focus_lost` or any notification, and queues the rest for
    /// later [`Runtime::flush`] calls; [`Runtime::clear_focus`] queues each one.
    /// Moving the focused component into a native window treats an error like a
    /// `focus_lost` error there: [`Runtime::commit_detach`] and
    /// [`Runtime::redock`] return the first one and do not move the
    /// presentation. The default never fails.
    fn focus_within_changed(
        &mut self,
        _cx: &mut EventContext,
        _within: bool,
    ) -> Result<(), RuntimeError> {
        Ok(())
    }
    /// Cancels component-owned input with access to its invalidation context.
    ///
    /// Runtime calls this on focus/context loss, hiding, removal and shutdown,
    /// and also while the component keeps focus, for example after a failed key
    /// operation or when it stops being focusable. Escape's ordinary delivery
    /// never calls it: an Escape press that reaches the focused component
    /// first ends its pointer capture, timer and edit sessions and then
    /// arrives as an ordinary key. An earlier consumer of Escape skips that
    /// cleanup, but closing a surface hides or removes what it showed, and
    /// that hiding or removal calls this method as it always does (see
    /// [`Runtime::key_input`]).
    /// Default invokes [`Self::input_reset`] and invalidates paint.
    ///
    /// It runs outside the evaluation phases, so an override may restore the
    /// component's own bindings here, such as a value it previewed during a
    /// drag; this is the only restore that also runs when the component is
    /// removed. It must not activate application actions: no callbacks,
    /// commands or user-write notifications. It returns nothing, because
    /// cancellation cannot be refused: a component whose restore write fails
    /// records the error and requests a [`Notifier`](crate::Notifier), so its
    /// [`Self::deliver_notifications`] returns the error from a later flush. A
    /// removed component is not called there, so its failure is not reported.
    ///
    /// **An override replaces that default, so it must release held-key actions
    /// itself**, by calling `input_reset` or doing the equivalent: the runtime
    /// does not call `input_reset` separately. An override may keep state it
    /// deliberately preserves, such as the draft of an editor that still owns
    /// focus (check [`EventContext::focused`]). Controls restoring layout
    /// preferences should override this and invalidate layout as well. Do not
    /// activate application actions from cancellation.
    fn cancel_input(&mut self, cx: &mut EventContext) {
        self.input_reset();
        cx.invalidate(Invalidation::Paint);
    }
    /// Releases component-owned held-key actions when its input context is lost.
    ///
    /// The default [`Self::cancel_input`] calls this, so without an override it
    /// runs on focus transfer/loss, hiding, unmount and runtime drop. A component
    /// that overrides `cancel_input` must call it from there (the runtime never
    /// calls it directly). A component starting sustained actions must release
    /// them here even without individual ups. Keep it idempotent: a component
    /// may reach it more than once for one loss of context.
    fn input_reset(&mut self) {}
    /// Runs a timed action whose deadline, set with [`EventContext::set_timer`],
    /// has elapsed.
    ///
    /// Called in the event phase of the first flush at or after the deadline,
    /// before that flush evaluates updates, so this callback may write bindings
    /// and commit parameter edits like ordinary input. `update` cannot: a timed
    /// repeat that must reach application state belongs here. The timer is
    /// one-shot; call `set_timer` again to continue a repeat. Hidden, unmounted
    /// and input-cancelled components never receive the call.
    ///  This UI-thread callback
    /// is not safe for time-critical producers and must return promptly. Default does nothing.
    ///
    /// # Errors
    /// An implementation returns an error when its timed action fails, such as a rejected
    /// edit; it propagates from [`Runtime::flush`] and the timer is not rearmed. The
    /// default never fails.
    fn timer_elapsed(&mut self, _cx: &mut EventContext) -> Result<(), RuntimeError> {
        Ok(())
    }
    /// Refreshes retained data according to the context's invalidation.
    ///
    /// Return read/scheduling errors to retain work for retry. This method must
    /// not perform recursive runtime updates or mutate signals during evaluation.
    ///
    /// # Errors
    /// An implementation returns an error when a read or scheduling request fails, such as
    /// `BorrowConflict` or `Exhausted`. The component stays pending so the work is retried,
    /// and the error propagates from [`Runtime::flush`].
    fn update(&mut self, context: &mut UpdateContext) -> Result<(), RuntimeError>;
    /// Called on a visibility transition, after hidden editing has been cancelled.
    fn visibility_changed(&mut self, _visible: bool) {}
    /// Called once after subscriptions, deadlines and edit ownership are released.
    fn unmounted(&mut self) {}
}

/// Owned UI-local snapshot of an immediate child for update or restoration.
/// Identity remains incarnation-specific; this snapshot does not keep a child mounted.
#[derive(Clone)]
pub struct ChildInfo {
    key: String,
    label: Option<String>,
    rich_label: Option<crate::RichText>,
    id: WidgetId,
    visible: bool,
    detachable: crate::Permission,
    focus_within: bool,
    captures_pointer: bool,
}

/// Shows the child's identity and state, not its labels or detach permission.
impl std::fmt::Debug for ChildInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ChildInfo")
            .field("key", &self.key)
            .field("id", &self.id)
            .field("visible", &self.visible)
            .field("focus_within", &self.focus_within)
            .field("captures_pointer", &self.captures_pointer)
            .finish_non_exhaustive()
    }
}
impl ChildInfo {
    /// Whether keyboard focus is on this child or inside it when captured.
    /// A container that mounts and unmounts children by position keeps
    /// such a child mounted, so focus, IME state and held keys are not lost.
    #[must_use]
    pub fn focus_within(&self) -> bool {
        self.focus_within
    }
    /// Whether this child or a descendant currently captures the pointer, so a
    /// gesture in progress is not cut short by unmounting its owner.
    #[must_use]
    pub fn captures_pointer(&self) -> bool {
        self.captures_pointer
    }
    /// Optional styled display metadata supplied by `Element::with_rich_label`.
    pub fn rich_label(&self) -> Option<&crate::RichText> {
        self.rich_label.as_ref()
    }
    /// Optional display label supplied by the child description, independent of its key.
    pub fn label(&self) -> Option<&str> {
        self.label.as_deref()
    }
    /// Stable sibling key; independent of display labels and runtime incarnation.
    pub fn key(&self) -> &str {
        &self.key
    }
    /// Identity usable in later public runtime operations on the same UI thread.
    pub fn id(&self) -> &WidgetId {
        &self.id
    }
    /// Effective presentation visibility when this snapshot was captured.
    pub fn visible(&self) -> bool {
        self.visible
    }
    /// Whether this child's description permits moving it into its own native
    /// window now, from `Element::with_detachable`: the current value of its
    /// permission, read without subscribing; a binding that cannot be read
    /// counts as a refusal. True by default. A container that offers a detach
    /// gesture must not offer it for a child that returns false; to follow a
    /// bound permission, read [`Self::detach_permission`] in its update.
    pub fn detachable(&self) -> bool {
        self.detachable.get().unwrap_or(false)
    }
    /// The child's detach permission from `Element::with_detachable`, for a
    /// container that offers a detach gesture: reading it with
    /// [`Permission::read`](crate::Permission::read) in the container's update
    /// subscribes the container, so its affordance follows a bound
    /// permission without polling. The runtime evaluates the same permission
    /// when a detach is requested.
    pub fn detach_permission(&self) -> &crate::Permission {
        &self.detachable
    }
}

/// Dependency collection and scheduling available while refreshing a component.
///
/// A context exists only during one update. Signal reads register a dependency
/// automatically; no subscriber callback runs during a write. Layout/paint are
/// invalidations are coalesced before the separate layout and paint phases.
pub struct UpdateContext {
    children: Vec<ChildInfo>,
    node: Rc<Node>,
    kind: Invalidation,
    subscriptions: Vec<Subscription>,
    deadline: Option<Duration>,
}

/// Shows the number of children it describes, not the component or its
/// subscriptions.
impl std::fmt::Debug for UpdateContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UpdateContext")
            .field("children", &self.children.len())
            .finish_non_exhaustive()
    }
}

impl UpdateContext {
    /// Creates a signal on this component's runtime, for a component that
    /// owns local state it creates while it describes its children and has no
    /// [`Runtime`] at hand, such as the open state of a [`crate::Menu`] built
    /// with [`crate::Menu::with_state`] from [`Component::update`].
    ///
    /// The signal is the same type [`Runtime::signal`] returns. Creating it
    /// does not subscribe this component; reading it through [`Self::read`]
    /// does, as for any signal.
    /// Its value lives as long as any clone of the handle: the component
    /// usually keeps one in a field or hands it to a child it describes, and
    /// the value is dropped with the last clone, at the latest when the
    /// component and the children holding it are unmounted. A signal created
    /// in every update is a new signal each time, so keep it when state must
    /// survive a description. Reads and writes after the runtime closes fail
    /// with [`RuntimeError::Closed`]. Thread restrictions match
    /// [`Runtime::signal`]: the handle is UI-thread local.
    ///
    /// ```
    /// # use pert_ui_runtime as pert_ui;
    /// use pert_ui::{Component, Element, Label, Runtime, RuntimeError, Signal, UpdateContext};
    ///
    /// /// Describes its child once, with state created during that update.
    /// #[derive(Default)]
    /// struct Counter {
    ///     count: Option<Signal<u32>>,
    /// }
    /// impl Component for Counter {
    ///     fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
    ///         if self.count.is_none() {
    ///             let count = cx.signal(3);
    ///             let text = format!("{}", count.get()?);
    ///             self.count = Some(count);
    ///             cx.request_children(vec![Element::new("text", Label::new(text))]);
    ///         }
    ///         Ok(())
    ///     }
    /// }
    ///
    /// let mut ui = Runtime::new();
    /// ui.mount_tree(Element::new("counter", Counter::default()))?;
    /// ui.flush(std::time::Duration::ZERO)?;
    /// let snapshot = ui.inspect();
    /// assert!(snapshot.components.iter().any(|item| item.key == "text"));
    /// # Ok::<(), RuntimeError>(())
    /// ```
    #[must_use]
    pub fn signal<T>(&self, value: T) -> crate::Signal<T> {
        crate::Signal::new(self.node.shared.clone(), value)
    }
    /// A handle through which this component asks for
    /// [`Component::deliver_notifications`], also from places without a
    /// context such as [`Component::restore_workspace_state`]. Keep it for the
    /// component's lifetime; it never retargets another component.
    #[must_use]
    pub fn notifier(&self) -> crate::Notifier {
        crate::Notifier::new(&self.node)
    }
    /// Cancels this component's pending [`EventContext::set_timer`] request,
    /// for an update that learns the timer's purpose is gone, such as an
    /// application write that discards a debounced edit. Does nothing without
    /// a pending request. Scheduling a timer still needs an event phase.
    pub fn cancel_timer(&self) {
        self.node.stop_timer();
    }
    /// Requests removal of this component and its descendants after successful
    /// updates, before layout. Requests coalesce by mounted incarnation. A failed
    /// update leaves the request pending for retry; removal uses ordinary gesture,
    /// subscription and focus cleanup. Never removes a later reuse of the key.
    pub fn request_unmount(&mut self) {
        self.node
            .shared
            .requested_removals
            .borrow_mut()
            .insert(self.node.serial, Rc::downgrade(&self.node));
    }

    /// Identity of the component being updated, valid until it is unmounted.
    /// Use it to name this component as the owner of a requested move.
    #[must_use]
    pub fn id(&self) -> WidgetId {
        WidgetId(self.node.clone())
    }

    /// Requests moving a retained subtree to `parent` at `index`, applied after
    /// this flush's updates and before layout.
    ///
    /// This is the deferred form of [`Runtime::reparent`] for components that
    /// restructure their own content, with the same contract: the subtree keeps
    /// its incarnations, local state and focus, and `index` is its insertion
    /// position among the new owner's children. Requests apply in order.
    ///
    /// A container that moves its own described content into an owner it
    /// contains, such as an overflow surface among its children, keeps
    /// that content described by its own description: when an application
    /// describes the container again, a moved child is matched by key where it
    /// is, with its state, and one the description omits unmounts. A container
    /// therefore records the moved children's identities across its own
    /// `Component::reconcile` rather than expecting them back among its
    /// children. `Toolbar` collapses groups into its overflow surface this way.
    /// A request whose child or parent is unmounted before it applies is
    /// discarded; any other rejection is returned from `Runtime::flush`, which
    /// leaves the tree unchanged and keeps the remaining requests queued.
    ///
    /// # Errors
    /// Returns `ForeignRuntime` immediately for a handle from another runtime.
    pub fn request_reparent(
        &mut self,
        child: &WidgetId,
        parent: &WidgetId,
        index: usize,
    ) -> Result<(), RuntimeError> {
        if !Rc::ptr_eq(&self.node.shared, &child.0.shared)
            || !Rc::ptr_eq(&self.node.shared, &parent.0.shared)
        {
            return Err(RuntimeError::ForeignRuntime);
        }
        self.node.shared.requested_moves.borrow_mut().push((
            Rc::downgrade(&child.0),
            Rc::downgrade(&parent.0),
            index,
        ));
        Ok(())
    }

    /// Requests that this component's children become `children`, applied after
    /// this flush's updates and before layout, exactly as
    /// [`Runtime::reconcile_children`] would: surviving keys keep their retained
    /// component, state, focus and capture, new keys mount and update before
    /// they are measured, and absent keys unmount with ordinary cleanup.
    ///
    /// This is how a container that decides its own content, such as a list
    /// that mounts only the rows near its viewport, changes structure from
    /// `update`. Requests coalesce: a later request from the same component in
    /// the same flush replaces the earlier one. A rejection, such as a duplicate
    /// key, is returned from `Runtime::flush`, which leaves the tree unchanged
    /// and keeps the remaining requests queued. Until a request succeeds,
    /// content search treats a collection offering [`ContentRows`](crate::ContentRows)
    /// as having obsolete retained child subtrees, even when its provider reports
    /// current mounted rows.
    /// A component must converge: the
    /// mounts it requests trigger another update wave inside the same flush,
    /// bounded by [`Runtime::MAX_FLUSH_WAVES`].
    ///
    /// The first request makes this component the owner of its children for
    /// the rest of its mount: when an application describes it again, through
    /// [`Runtime::reconcile_children`] on an ancestor, its own `reconcile`
    /// runs but its current children stay mounted with their state, and a
    /// description that gives it children is rejected with
    /// `LayoutError::InvalidChild`, unless it opted into
    /// [`Component::retain_initial_children`] at mount; that policy discards
    /// incoming generated descendants before admission.
    pub fn request_children(&mut self, children: Vec<crate::Element>) {
        self.node.self_managed.set(true);
        self.node.children_current.set(false);
        let mut queued = self.node.shared.requested_children.borrow_mut();
        queued.retain(|(owner, _)| owner.as_ptr() != Rc::as_ptr(&self.node));
        queued.push((Rc::downgrade(&self.node), children));
    }

    /// Runtime elapsed time for deterministic shared deadlines, never wall-clock time.
    pub fn now(&self) -> Duration {
        self.node.shared.now.get()
    }
    /// Whether the frontmost pointer target is this component or a descendant.
    /// Reading subscribes this component to ancestry transitions without polling.
    pub fn hovered(&self) -> bool {
        self.node.hover_reader.set(true);
        self.node.hover_within.get()
    }
    /// Whether this component or a logical descendant owns keyboard focus.
    /// Reading subscribes to focus-path changes; hidden views defer updates.
    pub fn focus_within(&self) -> bool {
        self.node.focus_update_reader.set(true);
        let mut focused = self
            .node
            .window()
            .focus_node
            .borrow()
            .as_ref()
            .and_then(Weak::upgrade);
        while let Some(node) = focused {
            if node.serial == self.node.serial {
                return true;
            }
            focused = node.presentation_parent();
        }
        false
    }

    /// Current immediate children, including hidden ones, in logical order.
    /// Structure changes schedule the owner's update; keys never substitute for IDs.
    pub fn children(&self) -> &[ChildInfo] {
        &self.children
    }
    /// Releases this component's capture/edit ownership during a state change such
    /// as disabling resizing. Restore local preferences in the component itself;
    /// this does not invoke `Component::cancel_input` recursively.
    ///
    /// It ends everything this component owns as input context, as focus loss
    /// does: pointer capture it holds is released, parameter edits it began are
    /// cancelled, its pending [`EventContext::set_timer`] deadline is dropped,
    /// and a drag session it is the source of is cancelled at the next flush.
    /// Restart a timer the component still needs after the change. A drag
    /// another component started, and capture held by another component, are
    /// left alone.
    pub fn cancel_input(&mut self) {
        self.node.lose_context();
    }

    /// Whether this component currently owns keyboard focus.
    #[must_use]
    pub fn focused(&self) -> bool {
        self.node.window().focused.get() == Some(self.node.serial)
    }
    /// Current logical window extent and native scale (zero-sized until supplied).
    #[must_use]
    pub fn viewport(&self) -> crate::Viewport {
        self.node.window().viewport.get()
    }

    /// Returns the strongest coalesced reason for this update.
    #[must_use]
    pub fn invalidation(&self) -> Invalidation {
        self.kind
    }

    /// Requests measurement and placement after this update, including ancestors
    /// whose preferred sizes can depend on this component. Requests coalesce in
    /// the current flush and do not schedule a second update or a timer.
    /// Use with `read_paint` when only some changes affect geometry, for example
    /// a numeric range acquiring another digit while ordinary values fit the
    /// reserved readout. Unchanged measurements must not request layout.
    pub fn request_layout(&mut self) {
        self.node.invalidate_layout();
    }

    /// Reads a binding and subscribes this component to layout-and-paint changes.
    ///
    /// # Errors
    /// Returns `ForeignRuntime`, `Closed` or `BorrowConflict` for an invalid read.
    pub fn read<T: Clone + 'static>(&mut self, signal: &ReadSignal<T>) -> Result<T, RuntimeError> {
        self.read_kind(signal, Invalidation::Layout)
    }

    /// Reads a binding that changes only presentation inside fixed geometry.
    ///
    /// # Errors
    /// Same read failures as [Self::read]. Component authors must use ordinary
    /// `read` when the value can affect measurements or placement, or explicitly
    /// call `request_layout` on the updates that change geometry.
    pub fn read_paint<T: Clone + 'static>(
        &mut self,
        signal: &ReadSignal<T>,
    ) -> Result<T, RuntimeError> {
        self.read_kind(signal, Invalidation::Paint)
    }

    fn read_kind<T: Clone + 'static>(
        &mut self,
        signal: &ReadSignal<T>,
        kind: Invalidation,
    ) -> Result<T, RuntimeError> {
        if !Rc::ptr_eq(&self.node.shared, signal.shared()) {
            return Err(RuntimeError::ForeignRuntime);
        }
        let value = signal.get()?;
        self.subscriptions.push(signal.subscribe(&self.node, kind));
        Ok(value)
    }

    /// Reads a series source and subscribes this component to its drains.
    ///
    /// The closure receives the latest status and the retained window, borrowed
    /// for the call so no item is copied. Subscribing is what keeps the source
    /// draining; the subscription declares paint-only invalidation, so a
    /// component whose geometry depends on the data calls
    /// [`Self::request_layout`] itself.
    ///
    /// # Errors
    /// Returns `ForeignRuntime` for a source of another runtime, or the read
    /// failures of [`SeriesSource::with_window`](crate::SeriesSource::with_window).
    pub fn read_series<T: crate::LiveItem, R>(
        &mut self,
        source: &crate::SeriesSource<T>,
        f: impl FnOnce(&crate::Telemetry<T>, &pert_ui_core::SampleWindow<T>) -> R,
    ) -> Result<R, RuntimeError> {
        if !Rc::ptr_eq(&self.node.shared, source.shared()) {
            return Err(RuntimeError::ForeignRuntime);
        }
        let _ = self.read_kind(
            &source.revision_signal().read_only(),
            crate::live::READ_KIND,
        )?;
        source.activate();
        source.with_window(f)
    }

    /// Requests another live presentation update at this window's presentation ceiling.
    ///
    /// Repeat while active; omitting this call stops animation. All requests in
    /// one flush share a deadline. Missed frames skip ahead without catch-up.
    /// Hidden, fully clipped and unmounted components have no live deadline.
    /// Revealing a retained component restarts its presentation. Unknown display
    /// rates use 60 Hz; see [`Runtime::set_presentation_rate`].
    ///
    /// # Errors
    /// Returns `Exhausted` if elapsed time cannot represent the next deadline.
    pub fn animate(&mut self) -> Result<(), RuntimeError> {
        self.node.animated.set(true);
        self.wake_after(self.node.presentation_interval())
    }

    /// Requests a one-shot wake for a meaningful delay, such as a caret blink.
    ///
    /// This extension is for component authors; normal live displays use `animate`.
    /// Multiple requests keep the earliest deadline. No wall-clock timer is started.
    ///
    /// # Errors
    /// Returns `ZeroDelay` for zero or `Exhausted` if the deadline overflows.
    pub fn wake_after(&mut self, delay: Duration) -> Result<(), RuntimeError> {
        if delay.is_zero() {
            return Err(RuntimeError::ZeroDelay);
        }
        let deadline = self
            .node
            .shared
            .now
            .get()
            .checked_add(delay)
            .ok_or(RuntimeError::Exhausted)?;
        self.deadline = Some(self.deadline.map_or(deadline, |old| old.min(deadline)));
        Ok(())
    }
}

/// Editing context scoped to one visible mounted component's event.
///
/// Created by [Runtime::event]. Parameter gestures begun here are cancelled on
/// focus loss, hiding, unmount or runtime drop, even if a gesture handle survives.
pub struct EventContext {
    pub(crate) node: Rc<Node>,
}

/// Shows nothing of the component it is scoped to.
impl std::fmt::Debug for EventContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EventContext").finish_non_exhaustive()
    }
}

impl EventContext {
    /// Creates a signal in this component's runtime, for state a component
    /// creates while it reacts to input, such as a menu it builds on demand.
    ///
    /// It is the same signal [`Runtime::signal`] and [`UpdateContext::signal`]
    /// create, with the same lifetime: the value lives as long as any clone of
    /// the handle, which is UI-thread local. Reads and writes after the runtime
    /// closes fail with [`RuntimeError::Closed`].
    #[must_use]
    pub fn signal<T>(&self, value: T) -> crate::Signal<T> {
        crate::Signal::new(self.node.shared.clone(), value)
    }
    /// Prepares text with the shared cache and current scale/style for hit testing
    /// or validation before a committed edit. Width is optional logical pixels.
    /// Returns the same bounded text errors as `MeasureContext::text`; this does
    /// not register layout dependencies or schedule rendering.
    ///
    /// # Errors
    /// Returns [`RuntimeError::Text`] for invalid geometry (a non-finite or negative width,
    /// a zero font size), an unreadable font or a result beyond a
    /// [`TextLimit`](crate::TextLimit); [`RuntimeError::TextEdit`] with
    /// [`TextEditError::TooLarge`](crate::TextEditError::TooLarge) for text longer than
    /// [`TextEditError::MAX_TEXT_BYTES`](crate::TextEditError::MAX_TEXT_BYTES); and
    /// `BorrowConflict` when text is prepared reentrantly.
    pub fn text(
        &self,
        text: &str,
        width: Option<f32>,
        style: crate::TextStyle,
    ) -> Result<crate::TextLayout, RuntimeError> {
        let size = self
            .node
            .shared
            .theme
            .borrow()
            .active_metrics()
            .metric(style.metric());
        self.node.shared.text.prepare(
            text,
            crate::text::TextRequest::new(style, size, &Cell::new(0))
                .width(width)
                .scale(self.node.raster_scale()),
        )
    }

    /// Current runtime elapsed time, for deterministic input timing such as
    /// double-click recognition. Native hosts advance this when dispatching events.
    ///
    /// It is the runtime's time, which only [`Runtime::flush`] and
    /// [`Runtime::service_intervals`] advance: input dispatched without one of
    /// them in between shares one instant. A headless host or test therefore
    /// advances time, for example by flushing with a later time, before the
    /// next input, as here, where two presses half a second apart are not a
    /// double click and two presses in one instant are.
    ///
    /// ```
    /// # use pert_ui_runtime as pert_ui;
    /// use pert_ui::*;
    /// use std::{cell::Cell, rc::Rc, time::Duration};
    /// struct Clicks { last: Option<Duration>, doubles: Rc<Cell<u32>> }
    /// impl Component for Clicks {
    ///     fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> { Ok(()) }
    ///     fn pointer_input(&mut self, cx: &mut EventContext, event: PointerEvent)
    ///         -> Result<InputOutcome, RuntimeError> {
    ///         if event.kind() != PointerKind::Press(PointerButton::Primary) {
    ///             return Ok(InputOutcome::Ignored);
    ///         }
    ///         let now = cx.now();
    ///         if self.last.is_some_and(|last| now - last <= Duration::from_millis(400)) {
    ///             self.doubles.set(self.doubles.get() + 1);
    ///             self.last = None;
    ///         } else {
    ///             self.last = Some(now);
    ///         }
    ///         Ok(InputOutcome::Handled)
    ///     }
    /// }
    /// let doubles = Rc::new(Cell::new(0));
    /// let mut ui = Runtime::new();
    /// ui.set_viewport(Viewport::new(100.0, 100.0, 1.0)?)?;
    /// ui.mount_tree(Element::new("area", Clicks { last: None, doubles: doubles.clone() }))?;
    /// let press = |ui: &mut Runtime| -> Result<(), RuntimeError> {
    ///     let at = Point::new(10.0, 10.0)?;
    ///     ui.pointer_input(PointerEvent::new(at, PointerKind::Press(PointerButton::Primary)))?;
    ///     ui.pointer_input(PointerEvent::new(at, PointerKind::Release(PointerButton::Primary)))?;
    ///     Ok(())
    /// };
    /// ui.flush(Duration::ZERO)?;
    /// press(&mut ui)?;
    /// ui.flush(Duration::from_millis(500))?; // the clock moves here
    /// press(&mut ui)?;
    /// assert_eq!(doubles.get(), 0);
    /// press(&mut ui)?; // same instant as the previous press
    /// assert_eq!(doubles.get(), 1);
    /// ui.service_intervals(Duration::from_millis(1500))?; // also moves the clock
    /// press(&mut ui)?;
    /// press(&mut ui)?;
    /// assert_eq!(doubles.get(), 2);
    /// assert_eq!(ui.now(), Duration::from_millis(1500));
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn now(&self) -> Duration {
        self.node.shared.now.get()
    }
    /// Converts a window-logical position into this component's local space.
    ///
    /// Pointer positions and component bounds share the window-logical space. A
    /// component reaches its own coordinates by subtracting its displayed origin
    /// and, inside a [view transform](crate::ViewTransform), dividing by the
    /// accumulated scale. This is that conversion, so the rule lives in one place
    /// rather than in every widget that handles a pointer:
    ///
    /// ```
    /// # use pert_ui_runtime as pert_ui;
    /// # use pert_ui::*;
    /// # struct W;
    /// # impl Component for W {
    /// # fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> { Ok(()) }
    /// fn pointer_input(&mut self, cx: &mut EventContext, e: PointerEvent)
    ///     -> Result<InputOutcome, RuntimeError> {
    ///     if let Some(local) = cx.to_local(e.position()) {
    ///         let _ = local.x();
    ///     }
    ///     Ok(InputOutcome::Handled)
    /// }
    /// # }
    /// ```
    ///
    /// Returns `None` before this component has been arranged, since it then has no
    /// origin to subtract. The result may fall outside the component's own bounds,
    /// which is normal while it holds a pointer capture and the pointer leaves it;
    /// callers that need containment test it themselves.
    #[must_use]
    pub fn to_local(&self, point: crate::Point) -> Option<crate::Point> {
        self.node.window_to_local(point)
    }
    /// Converts a window-logical displacement, such as the difference between two
    /// pointer positions, into this component's local pixels.
    ///
    /// Outside a [view transform](crate::ViewTransform) the two are equal. Inside
    /// one, a drag of ten window pixels over content shown at 200% moves five
    /// local pixels. Thresholds that describe the hand rather than the content,
    /// such as a click or drag distance, stay in window pixels.
    ///
    /// A displacement too large to express at a deeply nested small scale
    /// saturates at the largest finite value with its sign.
    #[must_use]
    pub fn to_local_delta(&self, delta: crate::Point) -> crate::Point {
        let scale = f64::from(self.node.scale.get());
        let largest = f64::from(f32::MAX);
        let local = |value: f32| (f64::from(value) / scale).clamp(-largest, largest) as f32;
        crate::Point::new(local(delta.x()), local(delta.y())).unwrap_or(delta)
    }
    /// Converts a window-logical rectangle, such as the one
    /// [`Component::reveal`] receives, into this component's local space, where
    /// it compares with [`Self::size`]. `None` before this component is arranged.
    #[must_use]
    pub fn to_local_rect(&self, rect: crate::Rect) -> Option<crate::Rect> {
        let origin = self
            .node
            .window_to_local(crate::Point::new(rect.x(), rect.y()).ok()?)?;
        let scale = self.node.scale.get();
        crate::Rect::new(
            origin.x(),
            origin.y(),
            rect.width() / scale,
            rect.height() / scale,
        )
        .ok()
    }
    /// This component's origin in its presentation parent's content space: the
    /// position the parent's `arrange` gave it, whatever view transforms apply.
    /// `None` at a root or before layout.
    #[must_use]
    pub fn position_in_parent(&self) -> Option<crate::Point> {
        let parent = self.node.presentation_parent()?;
        let [x, y, _, _] = self.node.bounds.get()?.values();
        let [px, py, _, _] = parent.bounds.get()?.values();
        let content = f64::from(parent.content_scale.get());
        let parent_scale = f64::from(parent.scale.get());
        // The parent's view offset is whatever separates its origin from the
        // origin of its content; recover the content position from the scales.
        let offset = parent.view_offset.get();
        crate::Point::new(
            ((f64::from(x) - f64::from(px) - parent_scale * f64::from(offset.x())) / content)
                as f32,
            ((f64::from(y) - f64::from(py) - parent_scale * f64::from(offset.y())) / content)
                as f32,
        )
        .ok()
    }
    /// Last successful allocation of the presentation parent in the parent's own
    /// local pixels, the space [`Self::position_in_parent`] is measured in when
    /// the parent applies no view transform of its own.
    #[must_use]
    pub fn parent_size(&self) -> Option<crate::Size> {
        self.node.presentation_parent()?.size.get()
    }
    /// Accumulated [view scale](crate::ViewTransform) of this component's local
    /// space: window-logical pixels per local pixel, 1 outside any view transform.
    #[must_use]
    pub fn view_scale(&self) -> f32 {
        self.node.scale.get()
    }
    /// Last successful allocation in this component's own local pixels, absent
    /// before layout and for legacy roots. Use this, not [`Self::bounds`], for
    /// local geometry: `bounds` is the window rectangle, which a view transform
    /// scales.
    #[must_use]
    pub fn size(&self) -> Option<crate::Size> {
        self.node.size.get()
    }

    /// Queues a text command for another component's own text, such as a
    /// context-menu item acting on the editor it was opened from, or a list's
    /// activation beginning the rename of a row's label.
    ///
    /// The runtime delivers it to [`Component::text_command`] in the event phase of
    /// this flush, in request order, so the receiving editor may commit edits as it
    /// does for key input. The clipboard is not read here, and only
    /// [`TextCommand::BeginEdit`](crate::TextCommand::BeginEdit) moves focus:
    /// the editor's focus request settles at the end of the same flush when that
    /// flush succeeds, as [`Self::request_focus`] describes. Routing
    /// follows [`Runtime::text_command`]: a target that is not an editor passes
    /// the command on to the focused component inside its subtree.
    /// A request whose target is unmounted or hidden before it applies is dropped;
    /// a target blocked by current modality returns `ModalBlocked` from the flush
    /// without reading the clipboard or changing the editor. The refused request
    /// is spent and later requests remain queued.
    /// An error from the target propagates from `Runtime::flush` and leaves the
    /// remaining requests queued. Requesting a command for a component that is not
    /// an editor is not an error and does nothing.
    ///
    /// # Errors
    /// Returns `ForeignRuntime` immediately for a handle from another runtime.
    pub fn request_text_command(
        &mut self,
        target: &WidgetId,
        command: crate::TextCommand,
    ) -> Result<(), RuntimeError> {
        if !Rc::ptr_eq(&self.node.shared, &target.0.shared) {
            return Err(RuntimeError::ForeignRuntime);
        }
        self.node
            .shared
            .requested_text_commands
            .borrow_mut()
            .push((Rc::downgrade(&target.0), command));
        Ok(())
    }
    /// Reads the host clipboard on the UI thread.
    ///
    /// # Errors
    /// Returns [`RuntimeError::Closed`] after shutdown, and `WriteDuringUpdate`,
    /// `WriteDuringLayout`, `WriteDuringPaint`, `WriteDuringInspection` or
    /// `WriteDuringDelivery` when called from inside that phase; `BorrowConflict` when
    /// called from inside the clipboard service;
    /// [`ClipboardError::Unavailable`](crate::ClipboardError::Unavailable) when no service
    /// is installed; and the service's own [`ClipboardError`](crate::ClipboardError), such
    /// as `NoText` or `Busy`.
    pub fn clipboard_text(&self) -> Result<String, RuntimeError> {
        self.node.shared.clipboard_read()
    }
    /// Writes the host clipboard on the UI thread. Cut should modify its binding
    /// only after this succeeds.
    ///
    /// # Errors
    /// As [`Self::clipboard_text`]; the service reports a failed write as its own
    /// [`ClipboardError`](crate::ClipboardError).
    pub fn set_clipboard_text(&self, text: &str) -> Result<(), RuntimeError> {
        self.node.shared.clipboard_write(text)
    }

    /// Current physical modifier state, including keys held before focus arrived.
    /// Window focus loss clears it independently of component gesture ownership.
    #[must_use]
    pub fn modifiers(&self) -> crate::Modifiers {
        self.node.shared.modifiers.get()
    }
    /// Identity of this event recipient, valid until unmount.
    #[must_use]
    pub fn id(&self) -> WidgetId {
        WidgetId(self.node.clone())
    }
    /// Enables/disables profiling from a UI event, retaining completed statistics.
    ///
    /// # Errors
    /// Returns [`RuntimeError::Closed`] after shutdown, and `WriteDuringUpdate`,
    /// `WriteDuringLayout`, `WriteDuringPaint`, `WriteDuringInspection` or
    /// `WriteDuringDelivery` when called from inside that phase.
    pub fn set_profiling(&self, enabled: bool) -> Result<(), RuntimeError> {
        self.node.shared.writable()?;
        self.node.shared.profiler.enable(enabled);
        Ok(())
    }
    /// Captures completed timing scopes without polling or live runtime handles.
    #[must_use]
    pub fn profile(&self) -> crate::ProfileSnapshot {
        self.node.shared.profiler.snapshot()
    }
    /// Clears completed and in-flight timings while preserving profiling enablement.
    ///
    /// # Errors
    /// Returns [`RuntimeError::Closed`] after shutdown, and `WriteDuringUpdate`,
    /// `WriteDuringLayout`, `WriteDuringPaint`, `WriteDuringInspection` or
    /// `WriteDuringDelivery` when called from inside that phase.
    pub fn clear_profile(&self) -> Result<(), RuntimeError> {
        self.node.shared.writable()?;
        self.node.shared.profiler.clear();
        Ok(())
    }
    /// A handle through which this component asks for
    /// [`Component::deliver_notifications`]; see [`UpdateContext::notifier`].
    #[must_use]
    pub fn notifier(&self) -> crate::Notifier {
        crate::Notifier::new(&self.node)
    }
    /// Requests focus for this recipient after pending updates and layout complete.
    /// The request is ignored if it becomes hidden, unmounted or nonfocusable.
    /// A recipient hidden only until an ancestor's update still pending in the
    /// same flush shows it, such as an editor its owner reveals, keeps the
    /// request until that update has run, so focus moves when the flush
    /// settles successfully. A flush that fails first leaves the request
    /// pending with that update, and a later flush retries both. A focus
    /// restoration meanwhile cancels it, except that a focused component
    /// containing the recipient, as a label contains its editor, keeps focus
    /// for the transfer even when it stopped being focusable to make it.
    /// Multiple requests before a flush use the latest recipient; IDs are never retargeted.
    /// Explicit focus transfers and window focus loss cancel any older request.
    pub fn request_focus(&self) {
        *self.node.window().requested_focus.borrow_mut() = Some(Rc::downgrade(&self.node));
        self.node.window().requested_within.set(false);
        self.node.invalidate(Invalidation::Paint);
    }
    /// Requests focus for another mount, under the rules of [`Self::request_focus`]:
    /// it moves after pending updates and layout complete, and is ignored if the
    /// target is then hidden, unmounted, nonfocusable or outside an open modal
    /// surface. The request belongs to the target's native window and replaces an
    /// earlier pending request there.
    ///
    /// This is how application coordination, such as a form sending the user to
    /// its first field in error from a submit callback, moves focus without access
    /// to the `Runtime`. The ID is never retargeted: a control mounted later under
    /// the same key is a different identity and is not focused.
    ///
    /// # Errors
    /// Returns `ForeignRuntime` for an ID of another runtime.
    pub fn request_focus_for(&self, target: &WidgetId) -> Result<(), RuntimeError> {
        if !Rc::ptr_eq(&self.node.shared, &target.0.shared) {
            return Err(RuntimeError::ForeignRuntime);
        }
        *target.0.window().requested_focus.borrow_mut() = Some(Rc::downgrade(&target.0));
        target.0.window().requested_within.set(false);
        target.0.invalidate(Invalidation::Paint);
        Ok(())
    }
    /// Requests focus for the first focusable component inside another mount,
    /// in Tab order, or for the mount itself when it is focusable, under the
    /// rules of [`Self::request_focus_for`]: it moves after pending updates and
    /// layout complete, replaces an earlier pending request in the target's
    /// window, and is ignored if the target is then hidden or unmounted, or
    /// holds no visible focusable component that an open modal surface allows.
    ///
    /// It returns focus to a region whose components the caller does not know,
    /// as a tool window does when Escape sends the user back to the main
    /// content: the target is the region's container, which need not be
    /// focusable itself.
    ///
    /// # Errors
    /// Returns `ForeignRuntime` for an ID of another runtime.
    pub fn request_focus_within(&self, target: &WidgetId) -> Result<(), RuntimeError> {
        self.request_focus_for(target)?;
        target.0.window().requested_within.set(true);
        Ok(())
    }
    /// Last successful allocation in window-logical coordinates, absent for legacy
    /// roots. Inside a [view transform](crate::ViewTransform) its extent is the
    /// displayed one; [`Self::size`] is the local allocation.
    #[must_use]
    pub fn bounds(&self) -> Option<crate::Rect> {
        self.node.bounds.get()
    }
    /// Last successful allocation of the logical parent, or None at a root.
    /// Coordinates are logical window pixels, like [`Self::bounds`].
    #[must_use]
    pub fn parent_bounds(&self) -> Option<crate::Rect> {
        self.node
            .presentation_parent()
            .and_then(|node| node.bounds.get())
    }
    /// Whether this recipient currently has keyboard focus.
    #[must_use]
    pub fn focused(&self) -> bool {
        self.node.window().focused.get() == Some(self.node.serial)
    }
    /// Whether the current pointer is inside this component's effective clip.
    #[must_use]
    pub fn pointer_inside(&self) -> bool {
        match (
            self.node.clip.get(),
            self.node.window().pointer_position.get(),
        ) {
            (Some(rect), Some(point)) => rect.contains(point),
            _ => false,
        }
    }
    /// The latest pointer position in this recipient's window, in window
    /// coordinates, or `None` before the first pointer event or after the
    /// pointer left the window.
    ///
    /// Pointer callbacks receive their position with the event. This is for
    /// work that another input triggers, such as re-checking which part of a
    /// component is under a stationary pointer after Escape cancels a drag.
    /// Convert it with [`Self::to_local`]; combine it with
    /// [`Self::pointer_inside`] to ignore a pointer over clipped-away parts.
    #[must_use]
    pub fn pointer_position(&self) -> Option<crate::Point> {
        self.node.window().pointer_position.get()
    }
    /// Keeps subsequent pointer events on this recipient until release/context loss.
    ///
    /// # Errors
    /// Returns [`RuntimeError::CaptureUnavailable`] when no pointer button is held or
    /// another component owns the capture.
    pub fn capture_pointer(&self) -> Result<(), RuntimeError> {
        if self.node.window().held_buttons.get() == 0
            || self
                .node
                .window()
                .capture
                .get()
                .is_some_and(|id| id != self.node.serial)
        {
            return Err(RuntimeError::CaptureUnavailable);
        }
        self.node.input_cancelled.set(false);
        self.node.window().capture.set(Some(self.node.serial));
        Ok(())
    }
    /// Releases this recipient's pointer capture, if any.
    pub fn release_pointer(&self) {
        if self.node.window().capture.get() == Some(self.node.serial) {
            self.node.window().capture.set(None);
        }
    }

    /// Invalidates retained local component data changed directly in an event.
    ///
    /// Signal bindings already invalidate automatically. Component authors use
    /// this for private state, requesting `Layout` when measurements can change.
    /// Invalidate only when that state changed: a layout is followed by a
    /// repeated pointer move ([`PointerEvent::is_refresh`](crate::PointerEvent::is_refresh)),
    /// so a move handler that invalidates layout unconditionally, or causes
    /// another layout through what it writes, never settles.
    pub fn invalidate(&self, kind: Invalidation) {
        self.node.invalidate(kind);
    }

    /// Schedules one event-phase wake for this recipient after `delay`.
    ///
    /// The runtime calls [`Component::timer_elapsed`] with an `EventContext` in the
    /// first flush at or after the deadline, before that flush's updates. A timed
    /// gesture such as a held step repeat or auto-scroll may therefore write
    /// bindings and commit parameter edits on time, exactly like ordinary input.
    /// [`UpdateContext::animate`] and [`UpdateContext::wake_after`] remain the
    /// presentation-only deadlines and are unaffected by this timer.
    ///
    /// A component owns at most one timer; this replaces a pending one. Repeat the
    /// call inside the callback to continue a repeat. Missed deadlines produce one
    /// call without catch-up, and the deterministic elapsed clock, not wall-clock
    /// time, decides eligibility: no OS timer or background executor is started,
    /// and the host wakes at [`Runtime::next_deadline`]. The timer is cancelled
    /// with this component's input context, so focus loss, hiding, unmounting,
    /// presentation movement and `Runtime::cancel_input` all stop it. An Escape
    /// press that reaches the focused component's ordinary delivery stops it
    /// silently, without [`Component::cancel_input`]. An earlier consumer of
    /// Escape (a drag, delayed help, an editor, an open surface; see
    /// [`Runtime::key_input`]) skips that step, so the timer keeps running
    /// unless the consumer's own lifecycle stops it, as closing a surface does
    /// for the components it hides. A hidden or removed component never
    /// receives the call.
    ///
    /// ```
    /// # use pert_ui_runtime as pert_ui;
    /// use pert_ui::{Component, Element, EventContext, InputOutcome, KeyAction,
    ///     KeyCode, KeyEvent, KeyTransition, KeyboardState, Runtime, RuntimeError, Signal,
    ///     UpdateContext};
    /// use std::time::Duration;
    ///
    /// struct HeldStep(Signal<i32>);
    /// impl Component for HeldStep {
    ///     fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> { Ok(()) }
    ///     fn focusable(&self) -> bool { true }
    ///     fn keyboard_event(
    ///         &mut self,
    ///         cx: &mut EventContext,
    ///         event: KeyEvent,
    ///         _: &KeyboardState,
    ///     ) -> Result<InputOutcome, RuntimeError> {
    ///         match event.transition {
    ///             // The press saves one step and arms the repeat.
    ///             KeyTransition::Pressed => {
    ///                 self.0.set(self.0.get()? + 1)?;
    ///                 cx.set_timer(Duration::from_millis(400))?;
    ///             }
    ///             KeyTransition::Released | KeyTransition::UnmatchedRelease => cx.cancel_timer(),
    ///             KeyTransition::Repeated => {}
    ///         }
    ///         Ok(InputOutcome::Handled)
    ///     }
    ///     fn timer_elapsed(&mut self, cx: &mut EventContext) -> Result<(), RuntimeError> {
    ///         self.0.set(self.0.get()? + 1)?;
    ///         cx.set_timer(Duration::from_millis(80))
    ///     }
    /// }
    ///
    /// let mut ui = Runtime::new();
    /// let steps = ui.signal(0);
    /// let stepper = ui.mount_tree(Element::new("stepper", HeldStep(steps.clone())))?;
    /// ui.focus(&stepper)?;
    /// ui.key_input(KeyCode::ArrowUp, KeyAction::Press)?;
    /// assert_eq!(steps.get()?, 1);
    /// ui.flush(Duration::from_millis(400))?;
    /// assert_eq!(steps.get()?, 2, "the held repeat committed when it was due");
    /// ui.key_input(KeyCode::ArrowUp, KeyAction::Release)?;
    /// ui.flush(Duration::from_secs(1))?;
    /// assert_eq!(steps.get()?, 2);
    /// assert_eq!(ui.next_deadline(), None);
    /// # Ok::<(), pert_ui::RuntimeError>(())
    /// ```
    ///
    /// # Errors
    /// Returns a runtime phase error (see [`Runtime`]),
    /// [`RuntimeError::ZeroDelay`] for a zero delay and
    /// [`RuntimeError::Exhausted`] when elapsed time cannot represent the
    /// deadline.
    pub fn set_timer(&self, delay: Duration) -> Result<(), RuntimeError> {
        self.node.shared.writable()?;
        if delay.is_zero() {
            return Err(RuntimeError::ZeroDelay);
        }
        let deadline = self
            .node
            .shared
            .now
            .get()
            .checked_add(delay)
            .ok_or(RuntimeError::Exhausted)?;
        self.node.start_timer(deadline);
        Ok(())
    }

    /// Cancels a pending [`Self::set_timer`] request; does nothing without one.
    /// Terminal gestures should cancel explicitly instead of relying on input
    /// cancellation, which is a lifecycle event rather than a release.
    pub fn cancel_timer(&self) {
        self.node.stop_timer();
    }
    pub(crate) fn attach(&mut self, gesture: &Rc<dyn ContextLoss>) {
        self.node.input_cancelled.set(false);
        let mut gestures = self.node.gestures.borrow_mut();
        gestures.retain(|gesture| gesture.strong_count() > 0);
        gestures.push(Rc::downgrade(gesture));
    }
    pub(crate) fn source(&self) -> u64 {
        self.node.serial
    }
}

/// A child's serial with the label and rich label a parent's read replaced.
type PreviousLabel = (u64, Option<String>, Option<crate::RichText>);

struct Entry {
    key: String,
    addressing: crate::layout::Addressing,
    label: Option<String>,
    rich_label: Option<crate::RichText>,
    /// A bound label the parent's update reads into `label` and `rich_label`.
    label_source: Option<crate::TextSource>,
    /// An icon composed before a bound label when it is read.
    label_leading: Option<crate::IconSource>,
    /// Whether the element takes part in content search (`Element::with_searchable`).
    searchable: bool,
    /// The share of spare main-axis space (`Element::with_grow`).
    grow: u16,
    node: Rc<Node>,
    component: Box<dyn Component>,
    drawing: Vec<crate::DrawCommand>,
    masks: Vec<crate::MaskImage>,
    images: Vec<crate::RasterImage>,
    vectors: Vec<crate::VectorGeometry>,
    foreground: usize,
    paint_dirty: Cell<bool>,
    text_drawn: bool,
    text_origin: Option<[u64; 2]>,
    text_mode: settings::TextRenderingMode,
    children: Vec<u64>,
    raised_children: Vec<u64>,
    kind: std::any::TypeId,
    composed: bool,
    subscriptions: Vec<Subscription>,
    /// The parent whose description lists this component: its lifetime parent,
    /// unless a move placed it deeper inside that parent's subtree, where it
    /// stays described by the parent (see `Runtime::homes`). `None` for roots.
    home: Option<u64>,
}

/// Observable work count and the earliest pending headless wake after a flush.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FlushReport {
    /// Number of component updates run by this flush, including presentation
    /// preparation and callback followups in other visible windows.
    pub components_updated: usize,
    /// Absolute elapsed deadline; `None` means no framework work is scheduled.
    pub next_deadline: Option<Duration>,
}

/// UI-local retained component runtime with a deterministic elapsed-time clock.
///
/// Create with defaults. `signal` creates local state;
/// [`mount_tree`](Self::mount_tree) registers a keyed component tree. `flush`
/// evaluates only pending visible components, not every mount.
/// The clock starts at zero and advances through flush operations. Each native
/// root has independent presentation state; the default App waits for eligible
/// deadlines or native events. Custom hosts select a root before local input,
/// flush and paint, and coordinate roots through application_deadline/flush_windows.
/// This runtime records drawing without OS windows, GPUs or background threads.
///
/// Input is delivered to its component at once, and the component writes what
/// its own rules write: a plain editor writes its binding immediately, while a
/// draft, a parameter proposal and a notification for a hidden window are not
/// written yet. The tree, derived bindings and layout show the result after
/// the next [`Self::flush`], so a test flushes before it inspects;
/// [`testing::Driver`](crate::testing::Driver) settles after every action.
///
/// Methods that change state first check the runtime. Their `# Errors`
/// sections call that check's failures *a runtime phase error*:
/// [`RuntimeError::Closed`] after [`Self::shutdown`], or
/// [`RuntimeError::WriteDuringUpdate`], [`RuntimeError::WriteDuringLayout`],
/// [`RuntimeError::WriteDuringPaint`], [`RuntimeError::WriteDuringInspection`]
/// or [`RuntimeError::WriteDuringDelivery`] from inside that phase. These are
/// the refusals of [`Signal::set`](crate::Signal::set) apart from its
/// `BorrowConflict`, and a refused call changes nothing.
///
/// ```compile_fail
/// # use pert_ui_runtime as pert_ui;
/// let runtime = pert_ui::Runtime::new();
/// std::thread::spawn(move || drop(runtime)); // UI ownership cannot cross threads.
/// ```
///
/// # Methods by audience
///
/// The methods are listed by who calls them. An application needs the first
/// group; tests and tools add the second, which includes the host methods a
/// headless test uses to stand in for a host; a custom host needs the third.
/// Each method's own page holds its contract.
///
/// **Applications.** What an application's own code calls.
///
/// - Content and state: [`new`](Self::new), [`signal`](Self::signal),
///   [`mount_tree`](Self::mount_tree), [`unmount`](Self::unmount),
///   [`reconcile_children`](Self::reconcile_children),
///   [`reparent`](Self::reparent), [`set_visible`](Self::set_visible),
///   [`keep_alive`](Self::keep_alive).
/// - Appearance: [`theme`](Self::theme), [`ui_settings`](Self::ui_settings),
///   [`fonts`](Self::fonts), [`icons`](Self::icons).
/// - Parameters: [`parameter`](Self::parameter),
///   [`parameter_any`](Self::parameter_any),
///   [`remote_parameter`](Self::remote_parameter).
/// - Work and timing: [`spawn`](Self::spawn), [`mailbox`](Self::mailbox),
///   [`defer`](Self::defer), [`start_interval`](Self::start_interval),
///   [`series_source`](Self::series_source), [`live_waker`](Self::live_waker).
/// - Commands and keys: [`register_command`](Self::register_command),
///   [`unregister_command`](Self::unregister_command),
///   [`rebind_command`](Self::rebind_command),
///   [`command_enabled`](Self::command_enabled),
///   [`command_enabled_signal`](Self::command_enabled_signal),
///   [`command_enabled_binding`](Self::command_enabled_binding),
///   [`command_in_modals`](Self::command_in_modals),
///   [`command_repeats`](Self::command_repeats),
///   [`command_name`](Self::command_name),
///   [`command_shortcuts`](Self::command_shortcuts),
///   [`command_info`](Self::command_info),
///   [`commands_for`](Self::commands_for),
///   [`command_context`](Self::command_context),
///   [`set_command_target`](Self::set_command_target),
///   [`invoke_command`](Self::invoke_command),
///   [`register_held_input`](Self::register_held_input),
///   [`unregister_held_input`](Self::unregister_held_input).
/// - Focus, reveal and search: [`focus`](Self::focus),
///   [`focus_next`](Self::focus_next), [`focused`](Self::focused),
///   [`can_focus`](Self::can_focus), [`reveal`](Self::reveal),
///   [`reveal_and_focus`](Self::reveal_and_focus),
///   [`reveal_child`](Self::reveal_child),
///   [`content_search`](Self::content_search),
///   [`jump_to_content`](Self::jump_to_content),
///   [`resume_content_jump`](Self::resume_content_jump).
/// - Windows: [`request_detach`](Self::request_detach),
///   [`request_redock`](Self::request_redock),
///   [`request_close`](Self::request_close),
///   [`request_window_title`](Self::request_window_title),
///   [`window_of`](Self::window_of), [`native_windows`](Self::native_windows),
///   [`primary_window`](Self::primary_window),
///   [`detached_view`](Self::detached_view),
///   [`contains_window`](Self::contains_window).
/// - Workspace: [`capture_workspace`](Self::capture_workspace),
///   [`restore_workspace`](Self::restore_workspace),
///   [`workspace_report`](Self::workspace_report).
/// - Clipboard and text: [`clipboard_text`](Self::clipboard_text),
///   [`set_clipboard_text`](Self::set_clipboard_text),
///   [`text_command`](Self::text_command).
/// - Lookups: [`find`](Self::find), [`find_descendant`](Self::find_descendant),
///   [`find_path`](Self::find_path), [`children`](Self::children),
///   [`key`](Self::key).
/// - Failures and shutdown: [`last_failure`](Self::last_failure),
///   [`shutdown`](Self::shutdown).
///
/// **Tests and inspection.** What tests and tools add; a test also calls the
/// application methods.
///
/// - Driving: [`flush`](Self::flush), [`now`](Self::now),
///   [`next_deadline`](Self::next_deadline), [`event`](Self::event),
///   [`try_event`](Self::try_event), [`key_input`](Self::key_input),
///   [`text_input`](Self::text_input), [`pointer_input`](Self::pointer_input),
///   [`mount`](Self::mount).
/// - Standing in for a host: [`set_viewport`](Self::set_viewport),
///   [`viewport`](Self::viewport),
///   [`command_key_input`](Self::command_key_input),
///   [`native_modifiers`](Self::native_modifiers),
///   [`keyboard`](Self::keyboard), [`paint`](Self::paint),
///   [`set_capture_support`](Self::set_capture_support).
/// - Inspection: [`inspect`](Self::inspect),
///   [`inspect_with`](Self::inspect_with), [`query`](Self::query),
///   [`query_with`](Self::query_with), [`inspection_id`](Self::inspection_id),
///   [`inspection_service`](Self::inspection_service),
///   [`validation_report`](Self::validation_report), [`bounds`](Self::bounds),
///   [`drag_active`](Self::drag_active), [`read_signal`](Self::read_signal).
/// - Diagnostics and capture: [`set_debug_mode`](Self::set_debug_mode),
///   [`clear_event_history`](Self::clear_event_history),
///   [`set_debug_overlay`](Self::set_debug_overlay),
///   [`debug_overlay`](Self::debug_overlay),
///   [`set_profiling`](Self::set_profiling), [`profile`](Self::profile),
///   [`clear_profile`](Self::clear_profile),
///   [`request_capture`](Self::request_capture).
///
/// **Hosts.** What a host that drives the runtime in place of `App` calls
/// besides the methods above; a headless test calls some of these too when it
/// stands in for a host.
///
/// - Event loop: [`flush_windows`](Self::flush_windows),
///   [`flush_commands`](Self::flush_commands),
///   [`has_command_requests`](Self::has_command_requests),
///   [`application_deadline`](Self::application_deadline),
///   [`interval_deadline`](Self::interval_deadline),
///   [`service_intervals`](Self::service_intervals),
///   [`service_inspection`](Self::service_inspection),
///   [`set_wake_delivery`](Self::set_wake_delivery).
/// - Windows and presentation: [`select_window`](Self::select_window),
///   [`current_window`](Self::current_window),
///   [`set_window_visible`](Self::set_window_visible),
///   [`set_presentation_rate`](Self::set_presentation_rate),
///   [`presentation_rate`](Self::presentation_rate),
///   [`needs_paint`](Self::needs_paint), [`invalidate`](Self::invalidate),
///   [`has_presentation_requests`](Self::has_presentation_requests),
///   [`take_presentation_requests`](Self::take_presentation_requests),
///   [`prepare_detach`](Self::prepare_detach),
///   [`commit_detach`](Self::commit_detach), [`redock`](Self::redock),
///   [`close_detached_window`](Self::close_detached_window),
///   [`native_window_options`](Self::native_window_options).
/// - Native input: [`text_preedit`](Self::text_preedit),
///   [`text_input_state`](Self::text_input_state),
///   [`cursor_icon`](Self::cursor_icon), [`clear_focus`](Self::clear_focus),
///   [`cancel_input`](Self::cancel_input).
/// - Captures: [`captures_supported`](Self::captures_supported),
///   [`pending_captures`](Self::pending_captures),
///   [`capture_scene`](Self::capture_scene),
///   [`complete_capture`](Self::complete_capture).
/// - Services and shutdown: [`set_clipboard`](Self::set_clipboard),
///   [`take_shutdown_errors`](Self::take_shutdown_errors).
pub struct Runtime {
    held_registrations: BTreeMap<u64, held::Registration>,
    held_owners: BTreeMap<crate::KeyCode, held::Owner>,
    held_serial: u64,
    suppressed_keys: std::collections::BTreeSet<crate::KeyCode>,
    command_presses: BTreeMap<crate::KeyCode, commands::CommandPress>,
    commands: BTreeMap<u64, commands::RegisteredCommand>,
    command_serial: u64,
    window: native_windows::WindowData,
    parked_windows: BTreeMap<u64, native_windows::WindowData>,
    primary_window: native_windows::NativeWindowId,
    window_serial: u64,
    flush_state: Rc<FlushState>,
    text: Rc<crate::text::TextCache>,
    #[cfg(debug_assertions)]
    validator: crate::validation::development::Validator,
    pub(crate) parameters: RefCell<Vec<Weak<dyn crate::parameter_binding::Registered>>>,
    keyboard: crate::KeyboardState,
    pub(crate) parameter_ids: std::collections::HashSet<crate::ParameterId>,
    pub(crate) shared: Rc<Shared>,
    entries: BTreeMap<u64, Entry>,
    keys: BTreeMap<(Option<u64>, String), u64>,
    /// Children moved deeper inside their description parent's subtree, by
    /// that parent and key: reconciling the parent matches them there. Every
    /// entry's component is owned by its home through lifetime parents.
    homes: BTreeMap<(u64, String), u64>,
    serial: u64,
    /// Advances whenever a component mounts or unmounts, so a resumable
    /// content search can tell that the tree it walks has changed.
    structure_generation: u64,
    /// The active drag session, at most one per runtime.
    drag: Option<drag::DragSession>,
    /// Decoded entries whose view was not mounted when the restore ran. They
    /// apply when a matching path mounts, at most once each.
    workspace_pending: Vec<(crate::WorkspacePath, crate::ViewState)>,
    workspace_report: crate::WorkspaceReport,
    /// Set while `set_window_visible(false)` hides the selected root: native
    /// hiding suspends presentation, so modal sessions stay open.
    native_hiding: bool,
    /// Cleanup failures retained independently of retired native windows.
    shutdown_errors: Vec<RuntimeError>,
    /// Values kept until shutdown (`Runtime::keep_alive`), dropped last-kept first.
    kept: Vec<Box<dyn std::any::Any>>,
    /// Background work started with `Runtime::spawn`, created on first use.
    background: Option<application::Background>,
    /// Running interval timers (`Runtime::start_interval`), by serial.
    intervals: BTreeMap<u64, application::Interval>,
    interval_serial: u64,
    /// Timers whose tick runs now, out of `intervals` but still counted.
    intervals_ticking: usize,
}

/// Shows the primary window, not the component tree or the application's
/// state. Inspect the tree with [`Runtime::inspect`].
impl std::fmt::Debug for Runtime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Runtime")
            .field("primary_window", &self.primary_window)
            .finish_non_exhaustive()
    }
}

impl Default for Runtime {
    fn default() -> Self {
        Self::new()
    }
}

impl Runtime {
    /// Maximum update/settlement waves in one flush, including internal follow-ups.
    ///
    /// Each wave evaluates the currently queued components once; layout/focus/hover
    /// settle after updates drain. This bounds repeated scheduling, not component
    /// count, callback duration or tree depth. A callback must still return promptly.
    pub const MAX_FLUSH_WAVES: usize = 64;

    fn consume_flush_wave(&mut self) -> Result<(), RuntimeError> {
        if self.flush_state.waves.get() >= Self::MAX_FLUSH_WAVES {
            self.flush_state.exhausted.set(true);
            #[cfg(debug_assertions)]
            {
                let pending = self
                    .window
                    .shared
                    .pending
                    .borrow()
                    .keys()
                    .filter_map(|serial| {
                        self.entries
                            .get(serial)
                            .map(|entry| (*serial, entry.key.clone()))
                    })
                    .collect();
                // Refresh recipients that scheduled work more than once are
                // evidence of a pointer-refresh loop; once may be legitimate.
                let refreshed = self
                    .flush_state
                    .refresh_work
                    .borrow()
                    .iter()
                    .filter(|(_, count)| **count >= 2)
                    .filter_map(|(serial, count)| {
                        self.entries
                            .get(serial)
                            .map(|entry| (*serial, entry.key.clone(), *count))
                    })
                    .collect();
                // The failing flush validates once it has returned.
                self.validator.update_limit(pending, refreshed);
            }
            return Err(RuntimeError::UpdateLimit);
        }
        self.flush_state.waves.set(self.flush_state.waves.get() + 1);
        Ok(())
    }

    /// Current elapsed scheduler time, advanced by `flush`. Reading does not
    /// capture inspection state, perform I/O or schedule work.
    #[must_use]
    pub fn now(&self) -> Duration {
        self.shared.now.get()
    }
    pub(crate) fn metric(&self, node: &Rc<Node>, role: crate::MetricRole) -> f32 {
        let mut theme = self.shared.theme.borrow_mut();
        theme.track_metrics(node);
        theme.active_metrics().metric(role)
    }
    /// Prepares plain text at `node`'s raster scale, replacing the request's.
    pub(crate) fn prepare_text(
        &self,
        node: &Node,
        text: &str,
        request: crate::text::TextRequest<'_>,
    ) -> Result<crate::TextLayout, RuntimeError> {
        let _timing = self
            .shared
            .profiler
            .span(crate::ProfileOperation::Text, None, "");
        self.text.prepare(text, request.scale(node.raster_scale()))
    }
    /// Prepares plain text to fit a width at `node`'s raster scale.
    pub(crate) fn prepare_fitted_text(
        &self,
        node: &Node,
        text: &str,
        request: crate::text::TextRequest<'_>,
        width: f32,
        overflow: crate::TextOverflow,
    ) -> Result<crate::FittedText, RuntimeError> {
        let _timing = self
            .shared
            .profiler
            .span(crate::ProfileOperation::Text, None, "");
        self.text
            .fit(text, request.scale(node.raster_scale()), width, overflow)
    }
    /// Prepares rich text at `node`'s raster scale, replacing the request's.
    pub(crate) fn prepare_rich_text(
        &self,
        node: &Node,
        text: &crate::RichText,
        request: crate::text::TextRequest<'_>,
    ) -> Result<crate::TextLayout, RuntimeError> {
        let _timing = self
            .shared
            .profiler
            .span(crate::ProfileOperation::Text, None, "");
        self.text
            .prepare_rich(text, request.scale(node.raster_scale()))
    }

    fn profile_target(
        &self,
        id: &WidgetId,
        operation: crate::ProfileOperation,
    ) -> Option<crate::profiling::Span> {
        if let Some(entry) = self.entries.get(&id.0.serial)
            && Rc::ptr_eq(&entry.node, &id.0)
        {
            self.shared
                .profiler
                .span(operation, Some(id.0.serial), &entry.key)
        } else {
            self.shared.profiler.span(operation, None, "")
        }
    }
    /// Enables/disables UI elapsed-time profiling, off by default. Pausing retains
    /// aggregates but discards in-flight scopes. Adds no scheduled work. Timing
    /// includes waiting/preemption; it is not CPU percentage or GPU execution time.
    ///
    /// # Errors
    /// Returns [`RuntimeError::Closed`] after shutdown, and `WriteDuringUpdate`,
    /// `WriteDuringLayout`, `WriteDuringPaint`, `WriteDuringInspection` or
    /// `WriteDuringDelivery` when called from inside that phase.
    pub fn set_profiling(&self, enabled: bool) -> Result<(), RuntimeError> {
        self.shared.writable()?;
        self.shared.profiler.enable(enabled);
        Ok(())
    }
    /// Returns bounded owned timing data without scheduling work or measuring the
    /// snapshot itself. Only completed scopes are included; see `ProfileSnapshot`.
    #[must_use]
    pub fn profile(&self) -> crate::ProfileSnapshot {
        self.shared.profiler.snapshot()
    }
    /// Clears timing aggregates, recent samples and in-flight scopes, preserving enablement.
    ///
    /// # Errors
    /// Returns [`RuntimeError::Closed`] after shutdown, and `WriteDuringUpdate`,
    /// `WriteDuringLayout`, `WriteDuringPaint`, `WriteDuringInspection` or
    /// `WriteDuringDelivery` when called from inside that phase.
    pub fn clear_profile(&self) -> Result<(), RuntimeError> {
        self.shared.writable()?;
        self.shared.profiler.clear();
        Ok(())
    }

    /// Current component input focus, or `None` after context loss.
    #[must_use]
    pub fn focused(&self) -> Option<WidgetId> {
        self.window.focus.clone()
    }

    /// Returns the latest development errors and per-component marker data.
    ///
    /// Automatically populated after changed components are flushed, regardless
    /// of diagnostic history mode. No callbacks or validation run when reading
    /// this report. Default release builds always return a disabled empty report.
    ///
    /// Geometry is validated only for a native window whose latest layout ran
    /// with a viewport that has width and height: content flushed before the
    /// first [`set_viewport`](Self::set_viewport), or laid out while a window's
    /// viewport has no area, reports no bounds, clip, padding-exceeding-bounds,
    /// hit-area, parent or overlap issue, and the overlay draws no boxes for it. An empty report
    /// then says nothing about that geometry. Until the next layout after a
    /// viewport change, the report keeps the last validation and is
    /// [`pending`](crate::ValidationReport::pending), which covers every
    /// window: a validation run while another window's layout is pending
    /// leaves that window's geometry unchecked, and an issue it had is not
    /// logged again when it reappears. Rules that do not depend on
    /// geometry, such as keys, colors and shortcuts, apply in every window.
    #[must_use]
    pub fn validation_report(&self) -> crate::ValidationReport {
        #[cfg(debug_assertions)]
        {
            // Pending in any window: validation is application-wide.
            let pending = self
                .shared
                .windows
                .borrow()
                .iter()
                .filter_map(Weak::upgrade)
                .any(|window| !window.pending.borrow().is_empty() || window.layout_pending.get());
            self.validator.report(pending)
        }
        #[cfg(not(debug_assertions))]
        {
            crate::ValidationReport::default()
        }
    }

    /// Enables or disables bounded event-history recording.
    ///
    /// Defaults to Off. Changing mode schedules no work and draws nothing; see
    /// [`set_debug_overlay`](Self::set_debug_overlay) for drawing. Off stops
    /// recording but retains existing history until explicitly cleared or the
    /// runtime is dropped.
    pub fn set_debug_mode(&self, mode: crate::DebugMode) {
        let mut history = self.shared.history.borrow_mut();
        if history.mode != mode {
            history.mode = mode;
            history.generation = history.generation.saturating_add(1);
        }
    }

    /// Most marker outlines [`DebugOverlay::Issues`](crate::DebugOverlay::Issues)
    /// draws in one window; the badge counts the rest.
    pub const MAX_OVERLAY_MARKERS: usize = 64;

    /// Most components [`DebugOverlay::Boxes`](crate::DebugOverlay::Boxes)
    /// outlines in one window.
    pub const MAX_OVERLAY_BOXES: usize = 1024;

    /// Selects the development drawing [`paint`](Self::paint) appends to every
    /// window's scene. Defaults to [`DebugOverlay::Off`](crate::DebugOverlay::Off).
    ///
    /// A changed setting marks each window's scene for one repaint and schedules
    /// nothing else; setting the current value does nothing. While an overlay is
    /// on, a completed validation pass that changes what it shows marks the scene
    /// again, and nothing else does, so stable issues cost no frames. The setting
    /// is accepted in every build, but only builds with `debug_assertions` have
    /// validation data to draw.
    pub fn set_debug_overlay(&self, overlay: crate::DebugOverlay) {
        if self.shared.overlay.replace(overlay) != overlay {
            self.shared.damage_scenes();
            self.shared.settings_signals.debug_overlay.publish(overlay);
        }
    }

    /// The current development overlay setting.
    #[must_use]
    pub fn debug_overlay(&self) -> crate::DebugOverlay {
        self.shared.overlay.get()
    }

    /// Refreshes development validation outside a flush.
    ///
    /// Inside a flush (a component's requested move, children, removal, or a
    /// nested flush) the tree is between arrangements: moved, added or removed
    /// components and their siblings still carry the previous pass's bounds, so
    /// a pass then would log transient issues. The validator stays dirty and the
    /// outermost flush validates once it has settled, or after it failed.
    #[cfg(debug_assertions)]
    fn refresh_validation(&mut self) {
        if self.flush_state.depth.get() == 0 {
            self.refresh_validation_now();
        }
    }

    /// Validates after an outermost flush that failed before it settled, so the
    /// failure's own issues (an update limit, a duplicate key a component
    /// requested) are reported. A nested flush leaves this to the outermost one.
    #[cfg(debug_assertions)]
    fn validate_failed_flush(&mut self, result: &Result<FlushReport, RuntimeError>) {
        if result.is_err() {
            self.refresh_validation();
        }
    }

    /// Refreshes development validation and requests the one repaint an active
    /// overlay needs when the pass changed what it shows.
    #[cfg(debug_assertions)]
    fn refresh_validation_now(&mut self) {
        // Geometry is validated where it is current and has area: the window's
        // latest layout ran against its present viewport, which is not empty.
        let measured = self
            .shared
            .windows
            .borrow()
            .iter()
            .filter_map(Weak::upgrade)
            .filter(|window| {
                let viewport = window.viewport.get();
                viewport.width() > 0.0 && viewport.height() > 0.0 && !window.layout_pending.get()
            })
            .map(|window| window.serial)
            .collect();
        self.validator.set_measured(measured);
        let refreshed = self.validator.refresh();
        let damaged = match self.shared.overlay.get() {
            crate::DebugOverlay::Off => false,
            crate::DebugOverlay::Issues => refreshed.markers_changed,
            crate::DebugOverlay::Boxes => refreshed.ran,
        };
        if damaged {
            self.shared.damage_scenes();
        }
    }

    /// Appends the selected development overlay for the current window.
    #[cfg(debug_assertions)]
    fn compose_overlay(&self, scene: &mut crate::DisplayList) {
        let window = self.window.shared.serial;
        match self.shared.overlay.get() {
            crate::DebugOverlay::Off => {}
            crate::DebugOverlay::Issues => overlay::issues(
                &mut scene.fills,
                scene.viewport,
                // A marker whose component is gone belongs to no window any
                // more; every window counts it rather than none.
                self.validator
                    .markers()
                    .filter(|(owner, _)| owner.is_none_or(|owner| owner == window))
                    .map(|(_, marker)| marker),
                Self::MAX_OVERLAY_MARKERS,
            ),
            crate::DebugOverlay::Boxes => overlay::boxes(
                &mut scene.fills,
                scene.viewport,
                self.validator.boxes(window),
                Self::MAX_OVERLAY_BOXES,
            ),
        }
    }

    /// Runs a fallible application event and records its reported success/failure.
    ///
    /// Timing class: [in the input](https://github.com/pertyjons/pert-ui/blob/main/docs/contracts.md#in-the-input).
    ///
    /// Unlike `event`, this can inspect the closure's Result. Delivery failures
    /// remain the outer runtime error; application failures remain the inner error.
    /// Generic `event` records only `Dispatched`, not an invented success result.
    ///
    /// # Errors
    /// Those of [`Self::event`]: target refusals before anything changes, and
    /// later focus-transfer errors after focus has moved, in both cases without
    /// calling the closure or recording its outcome.
    pub fn try_event<R, E: std::fmt::Display>(
        &mut self,
        id: &WidgetId,
        event: impl FnOnce(&mut EventContext) -> Result<R, E>,
    ) -> Result<Result<R, E>, RuntimeError> {
        let result = self.event(id, event)?;
        self.shared.record(
            crate::EventKind::Application,
            Some(id.0.serial),
            None,
            || match &result {
                Ok(_) => crate::EventStatus::Succeeded,
                Err(error) => crate::EventStatus::Failed(error.to_string()),
            },
        );
        Ok(result)
    }

    /// Supplies native logical window size and DPI scale before application zoom.
    /// Changes invalidate layout at the next flush; active gestures cancel before
    /// geometry changes. Do not pass an already zoomed viewport back to this method.
    ///
    /// A host presenting to a native surface builds the viewport with
    /// [`Viewport::from_physical`](crate::Viewport::from_physical), which retains
    /// the operating system's physical extent; zoom leaves that extent unchanged,
    /// and [`Viewport::physical`](crate::Viewport::physical) is what the surface is
    /// sized from.
    ///
    /// The desktop adapter calls this automatically, before it first flushes.
    /// Headless callers can supply a viewport to test custom drawing; until
    /// they do, layout runs against an empty one. Hidden components retain
    /// dirtiness.
    ///
    /// # Errors
    /// Returns `Closed` after shutdown; `WriteDuringInspection` from a
    /// [`command_enabled`](Self::command_enabled) predicate, which runs during
    /// inspection;
    /// `WriteDuringDelivery` from inside a parameter delivery callback; and
    /// `Geometry(GeometryError::NonFinite)` when the viewport divided by the
    /// UI zoom overflows. Update, layout and paint run inside the runtime's
    /// own `&mut self` methods, so their phase errors cannot reach this
    /// `&self` method.
    pub fn set_viewport(&self, viewport: crate::Viewport) -> Result<(), RuntimeError> {
        let _timing = self
            .shared
            .profiler
            .span(crate::ProfileOperation::Viewport, None, "");
        self.shared.writable()?;
        self.native_viewport(viewport)?;
        if self.entries.is_empty() {
            self.window.shared.geometry_change.set(None);
            self.window.shared.layout_pending.set(false);
        }
        Ok(())
    }

    /// Creates an empty runtime with no scheduled work, workers or timers.
    #[must_use]
    pub fn new() -> Self {
        let text = Rc::new(crate::text::TextCache::default());
        let shared = Rc::new(Shared {
            failure: RefCell::new(None),
            tasks: RefCell::new(std::collections::VecDeque::new()),
            task_dispatching: Cell::new(false),
            computations: RefCell::new(BTreeMap::new()),
            application_modal: Cell::new(None),
            command_dispatching: Cell::new(false),
            command_requests: RefCell::new(Vec::new()),
            reveal_dispatching: Cell::new(false),
            reveal_requests: RefCell::new(std::collections::VecDeque::new()),
            presentation_requests: RefCell::new(Vec::new()),
            windows: RefCell::new(Vec::new()),
            modifiers: Cell::new(crate::Modifiers::default()),
            zoom: Cell::new(settings::UiZoom::default()),
            settings_signals: settings::SettingsSignals::default(),
            shapes: Rc::new(RefCell::new(crate::shapes::ShapeCache::default())),
            theme: RefCell::new(theme::State::default()),
            font_subscribers: RefCell::new(fonts::FontSubscribers::default()),
            requested_removals: RefCell::new(BTreeMap::new()),
            requested_moves: RefCell::new(Vec::new()),
            requested_children: RefCell::new(Vec::new()),
            requested_text_commands: RefCell::new(Vec::new()),
            drag_request: RefCell::new(None),
            drag_source: Cell::new(None),
            drag_lost: Cell::new(false),
            shutting_down: Cell::new(false),
            overlay: Cell::new(crate::DebugOverlay::Off),
            motion: Cell::new(settings::MotionPreference::default()),
            text_rendering: Cell::new(settings::TextRenderingMode::default()),
            motion_subscribers: RefCell::new(Vec::new()),
            strings: RefCell::new(crate::UiStrings::english()),
            history: RefCell::new(crate::diagnostics::History::default()),
            profiler: crate::profiling::Profiler::default(),
            phase: Cell::new(Phase::Idle),
            closed: Cell::new(false),
            now: Cell::new(Duration::ZERO),
            scale_settle: Cell::new(None),
            stretched: RefCell::new(Vec::new()),
            text: text.clone(),
            clipboard: RefCell::new(None),
            sources: RefCell::new(Vec::new()),
            active_sources: RefCell::new(BTreeMap::new()),
            source_visibility_dirty: Cell::new(false),
            source_serial: Cell::new(0),
            wake: crate::live::Wake::default(),
            mailboxes: RefCell::new(Vec::new()),
            inspection_services: RefCell::new(Vec::new()),
            answer_depth: Cell::new(0),
            capture_support: Cell::new(false),
            captures: RefCell::new(Vec::new()),
            next_capture: Cell::new(1),
            deferred: RefCell::new(Vec::new()),
            notifications: RefCell::new(Vec::new()),
            notification_settlement: Cell::new(false),
            notification_input: RefCell::new(None),
            notification_origin: RefCell::new(None),
        });
        let window = native_windows::WindowData::new(1, Rc::downgrade(&shared));
        let primary_window = native_windows::NativeWindowId(window.shared.clone());
        shared
            .windows
            .borrow_mut()
            .push(Rc::downgrade(&primary_window.0));
        Self {
            held_registrations: BTreeMap::new(),
            held_owners: BTreeMap::new(),
            held_serial: 0,
            suppressed_keys: std::collections::BTreeSet::new(),
            command_presses: BTreeMap::new(),
            commands: BTreeMap::new(),
            command_serial: 0,
            primary_window: primary_window.clone(),
            window,
            parked_windows: BTreeMap::new(),
            window_serial: 1,
            flush_state: Rc::default(),
            text,
            #[cfg(debug_assertions)]
            validator: crate::validation::development::Validator::default(),
            parameters: RefCell::new(Vec::new()),
            keyboard: crate::KeyboardState::default(),
            parameter_ids: std::collections::HashSet::new(),
            shared,
            entries: BTreeMap::new(),
            keys: BTreeMap::new(),
            homes: BTreeMap::new(),
            serial: 0,
            structure_generation: 0,
            drag: None,
            workspace_pending: Vec::new(),
            workspace_report: crate::WorkspaceReport::default(),
            native_hiding: false,
            shutdown_errors: Vec::new(),
            kept: Vec::new(),
            background: None,
            intervals: BTreeMap::new(),
            interval_serial: 0,
            intervals_ticking: 0,
        }
    }

    /// Creates a signal owned by this runtime. Values may be general application data.
    #[must_use]
    pub fn signal<T>(&self, value: T) -> Signal<T> {
        Signal::new(self.shared.clone(), value)
    }

    /// Adopts the consumer half of a series bridge as a UI-thread source.
    ///
    /// The runtime drains it while a visible component subscribes through
    /// [`UpdateContext::read_series`]; see [`SeriesSource`](crate::SeriesSource) for
    /// the schedule.
    /// The endpoint half goes to the producer thread.
    ///
    /// ```
    /// # use pert_ui_runtime as pert_ui;
    /// use pert_ui::{Capacity, Delivery, Runtime, SeriesConfig, Timestamp, series_bridge};
    ///
    /// let runtime = Runtime::new();
    /// let config = SeriesConfig::new(Capacity::new(256)?);
    /// let (mut endpoint, consumer) = series_bridge::<f32>(config);
    /// let source = runtime.series_source(consumer, Delivery::Polled);
    /// let producer = std::thread::spawn(move || {
    ///     let _ = endpoint.publish(0.5, Timestamp::new(1));
    /// });
    /// producer.join().expect("producer");
    /// // Nothing subscribes yet, so nothing was drained and nothing is scheduled.
    /// assert_eq!(runtime.next_deadline(), None);
    /// assert!(source.telemetry()?.latest.is_none());
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    #[must_use = "dropping the last clone of the source closes the bridge"]
    pub fn series_source<T: crate::LiveItem>(
        &self,
        consumer: crate::SeriesConsumer<T>,
        delivery: crate::Delivery,
    ) -> crate::SeriesSource<T> {
        crate::SeriesSource::new(&self.shared, consumer, delivery)
    }

    /// Returns a `Send` handle that asks the runtime to drain notified sources.
    ///
    /// Without [`Self::set_wake_delivery`] a notification only marks the
    /// runtime due, which [`Self::next_deadline`] reports; a host that sleeps
    /// on an event loop installs delivery so the notification also wakes it.
    #[must_use]
    pub fn live_waker(&self) -> crate::LiveWaker {
        self.shared.wake.waker()
    }

    /// Creates a bounded mailbox that delivers owned values from other threads to
    /// `handler` on this runtime's thread.
    ///
    /// Timing class: [at the next flush](https://github.com/pertyjons/pert-ui/blob/main/docs/contracts.md#at-the-next-flush).
    ///
    /// Hand [`Mailbox::sender`](crate::Mailbox::sender) handles to workers; each
    /// accepted value wakes the host like a [`LiveWaker`](crate::LiveWaker) and
    /// reaches `handler` at the start of a later [`Self::flush`], in order, after
    /// live-data sources are drained and before components update, so values the
    /// handler writes to signals are shown by that flush. A flush delivers only
    /// what was queued when its delivery began: all inbox frontiers are captured
    /// before any handler runs, so sends between handlers wait for the next
    /// flush regardless of mailbox registration order. Concurrent worker sends
    /// are included only if accepted before their inbox's frontier is captured.
    /// A handler error is returned from
    /// that flush; the value it was given is consumed and the rest stay queued
    /// for the next flush. The contract is in `docs/contracts.md`.
    ///
    /// # Errors
    /// Returns `Closed` after shutdown, `WriteDuringInspection` from a
    /// [`command_enabled`](Self::command_enabled) predicate and
    /// `WriteDuringDelivery` from inside a parameter delivery callback. The other phase errors cannot reach this
    /// `&self` method, as for [`Self::set_viewport`].
    pub fn mailbox<T: Send + 'static>(
        &self,
        capacity: crate::Capacity,
        handler: impl FnMut(T) -> Result<(), RuntimeError> + 'static,
    ) -> Result<crate::Mailbox<T>, RuntimeError> {
        self.shared.writable()?;
        let (mailbox, deliverable) =
            crate::Mailbox::new(capacity, handler, self.shared.wake.waker());
        let mut mailboxes = self.shared.mailboxes.borrow_mut();
        mailboxes.retain(|weak| weak.strong_count() > 0);
        mailboxes.push(deliverable);
        Ok(mailbox)
    }

    /// Creates an [`InspectionService`](crate::InspectionService) whose
    /// clients ask this runtime for snapshots, queries, profiles, validation
    /// reports and workspace captures from other threads.
    ///
    /// The queue holds at most `capacity` unanswered requests; nothing is
    /// preallocated and no thread or timer starts. Requests are answered at
    /// the end of each flush, and an accepted request makes the runtime due
    /// in [`next_deadline`](Self::next_deadline) and wakes the host through
    /// the same delivery a [`UiSender`](crate::UiSender) uses.
    ///
    /// # Errors
    /// Returns `Closed` after shutdown, `WriteDuringInspection` from a
    /// [`command_enabled`](Self::command_enabled) predicate and
    /// `WriteDuringDelivery` from inside a parameter delivery callback. The other phase errors cannot reach this
    /// `&self` method, as for [`Self::set_viewport`].
    pub fn inspection_service(
        &self,
        capacity: crate::Capacity,
    ) -> Result<crate::InspectionService, RuntimeError> {
        self.shared.writable()?;
        let (service, weak) = crate::InspectionService::new(capacity, self.shared.wake.waker());
        let mut services = self.shared.inspection_services.borrow_mut();
        services.retain(|weak| weak.strong_count() > 0);
        services.push(weak);
        Ok(service)
    }

    /// Answers every queued inspection request now, from the state the last
    /// flush left, without flushing.
    ///
    /// For a host that withholds flushes: the native `App` does while a
    /// window's redraw is pending or its surface is unavailable, since a
    /// compositor may hold a minimized window's redraw indefinitely. Calling
    /// this there keeps a request from being stranded; the reply carries no
    /// flush error. An ordinary host never needs it, because every outermost
    /// [`flush`](Self::flush) and [`flush_windows`](Self::flush_windows)
    /// answers on its own. Answering during a flush, an update, a paint or an
    /// inspection is refused.
    ///
    /// # Errors
    /// Returns a runtime phase error (see [`Runtime`]), or [`RuntimeError::BorrowConflict`]
    /// when called during a flush or while inspection requests are being
    /// answered.
    pub fn service_inspection(&mut self) -> Result<(), RuntimeError> {
        self.shared.writable()?;
        if self.flush_state.depth.get() > 0 || self.shared.answer_depth.get() > 0 {
            return Err(RuntimeError::BorrowConflict);
        }
        let shared = self.shared.clone();
        shared.answer_inspection(self, None);
        Ok(())
    }

    /// Installs how a [`LiveWaker`](crate::LiveWaker) wakes this host's event
    /// loop; the callback runs on the notifying thread and returns whether the
    /// wake was delivered. The default native `App` installs its own; a custom
    /// host installs one that reaches its loop. Replacing it reopens a waker
    /// that reported closed because delivery failed. After
    /// [`Self::shutdown`] the call is ignored and every waker keeps reporting
    /// [`WakeError::Closed`](crate::WakeError::Closed), since no flush will
    /// drain what a wake announces. A callback that panics unwinds out of
    /// that `notify`, leaving the wake pending for the next flush; later
    /// notifications still reach the callback.
    pub fn set_wake_delivery(&self, deliver: impl Fn() -> bool + Send + Sync + 'static) {
        self.shared.wake.set_delivery(Box::new(deliver));
    }

    /// Mounts an independent *legacy root*: one component with a nonempty
    /// root-scoped stable key that takes no part in layout.
    ///
    /// Prefer [`Self::mount_tree`] for anything that is laid out. A legacy root
    /// is never measured or arranged, so it has no allocation and paints in
    /// window coordinates through `painter.viewport()`, and it has no
    /// description to reconcile: a container mounted this way never lays out,
    /// draws or reconciles its children. It remains for components that only
    /// update or paint the whole window themselves, such as a binding observer
    /// or a test probe.
    ///
    /// The first update is scheduled once. Keeping a mount while changing its
    /// presentation order preserves identity; unmount/remount creates a new one.
    ///
    /// # Errors
    /// Each leaves other mounts in place: a runtime phase error (see [`Runtime`]);
    /// [`RuntimeError::EmptyKey`] for a key without a non-whitespace
    /// character; [`RuntimeError::DuplicateKey`] for a key another root already
    /// uses; or [`RuntimeError::Exhausted`] when no new identity remains.
    pub fn mount(
        &mut self,
        key: impl Into<String>,
        component: impl Component + 'static,
    ) -> Result<WidgetId, RuntimeError> {
        let _timing = self
            .shared
            .profiler
            .span(crate::ProfileOperation::Mount, None, "");
        let id = self.mount_description(crate::Element::new(key, component), None, false)?;
        self.apply_pending_workspace();
        Ok(id)
    }

    fn check(&self, id: &WidgetId) -> Result<(), RuntimeError> {
        if !Rc::ptr_eq(&self.shared, &id.0.shared) {
            return Err(RuntimeError::ForeignRuntime);
        }
        if !id.0.mounted.get() {
            return Err(RuntimeError::Unmounted);
        }
        Ok(())
    }

    /// Dispatches a general event closure to a visible mount and focuses it.
    ///
    /// Timing class: [in the input](https://github.com/pertyjons/pert-ui/blob/main/docs/contracts.md#in-the-input).
    ///
    /// Switching focus first cancels the old component's active gestures. The
    /// closure may write signals or begin/edit parameter gestures. Its return
    /// value is passed through; fallible closures therefore yield a nested result.
    ///
    /// # Errors
    /// The refusals of [`Self::focus`] (a runtime phase error,
    /// [`RuntimeError::ForeignRuntime`], [`RuntimeError::Unmounted`],
    /// [`RuntimeError::Hidden`] or [`RuntimeError::ModalBlocked`]) come before
    /// executing user code, leaving focus and the selected window unchanged.
    /// The errors [`Self::focus`] forwards after the transfer, from
    /// [`Component::focus_lost`], [`Component::focus_within_changed`],
    /// [`Component::reveal`] or the flush that settles a scrolled ancestor,
    /// are returned after focus has moved to the target, without running the
    /// closure.
    pub fn event<R>(
        &mut self,
        id: &WidgetId,
        event: impl FnOnce(&mut EventContext) -> R,
    ) -> Result<R, RuntimeError> {
        let _timing = self.profile_target(id, crate::ProfileOperation::Application);
        self.focus(id).inspect_err(|error| {
            self.shared
                .record(crate::EventKind::Dispatch, None, None, || {
                    crate::EventStatus::Failed(error.to_string())
                })
        })?;
        id.0.input_cancelled.set(false);
        let result = event(&mut EventContext { node: id.0.clone() });
        self.shared
            .record(crate::EventKind::Dispatch, Some(id.0.serial), None, || {
                crate::EventStatus::Dispatched
            });
        Ok(result)
    }

    /// Returns shared application physical-key state, independent of text/IME.
    pub fn keyboard(&self) -> &crate::KeyboardState {
        &self.keyboard
    }

    /// Focuses a visible component, resetting the previous input/edit context.
    ///
    /// Physical held keys stay tracked during an in-window focus transfer. The new
    /// component receives only subsequent events, not fabricated key-downs.
    /// Scrollable ancestors reveal the target; pointer presses focus their hit
    /// target without this additional scrolling, and each ancestor that
    /// scrolls is settled by a flush before this returns.
    ///
    /// This is the low-level focus operation: unlike keyboard traversal and
    /// pointer presses, it also focuses a component whose
    /// [`Component::focusable`] is false, as that method documents. To reveal
    /// a hidden target first, use [`Self::reveal_and_focus`].
    ///
    /// Focusing a component of another window selects that window. A refused
    /// target leaves the selected window and its focus unchanged.
    ///
    /// Focus does not move a text editor's caret or selection: they stay where
    /// the editor's last edit or movement left them, which is the start of the
    /// text for an editor not edited since it was mounted or since its binding
    /// was written from outside. Typed text is inserted there. A pointer press
    /// places the caret where it lands; to replace the text, select it all
    /// first with [`TextCommand::SelectAll`](crate::TextCommand::SelectAll)
    /// through [`Self::text_command`].
    ///
    /// # Errors
    /// Before anything changes: a runtime phase error (see [`Runtime`]);
    /// [`RuntimeError::ForeignRuntime`] for an ID of another runtime;
    /// [`RuntimeError::Unmounted`] for an unmounted one;
    /// [`RuntimeError::Hidden`] for a hidden component; and
    /// [`RuntimeError::ModalBlocked`] for one a modal surface blocks.
    ///
    /// After the transfer has completed, with focus on the target, the
    /// callbacks' errors are forwarded. The first error of the previous
    /// recipient's [`Component::focus_lost`] and of
    /// [`Component::focus_within_changed`] on the ancestors focus leaves and
    /// then enters is returned, and later ones are queued for later
    /// [`Self::flush`] calls. Revealing the target then forwards an error of
    /// an ancestor's [`Component::reveal`] or of the flush that settles a
    /// scrolled ancestor; such an error is returned, and a callback error
    /// that would have been returned is queued ahead of the later ones.
    pub fn focus(&mut self, id: &WidgetId) -> Result<(), RuntimeError> {
        let previous = self.current_window();
        self.select_window(&self.window_of(id)?)?;
        if let Err(error) = self.admit_focus(id) {
            self.restore_window(&previous)?;
            return Err(error);
        }
        self.focus_with_reveal(id, true)
    }
    /// Whether `id` can take focus now: the component is mounted, visible,
    /// focusable by its own [`Component::focusable`] (a disabled built-in
    /// control is not), and no modal surface of its window blocks it.
    ///
    /// [`Self::focus`] refuses only the hidden, unmounted and modal-blocked
    /// cases and focuses a component whose `focusable` is false. A host that
    /// gives focus back to a component it took focus from asks this first,
    /// so a component disabled meanwhile stays unfocused. It is a query only:
    /// it reveals nothing, flushes nothing and changes no state, so its
    /// answer describes the state since the last flush.
    ///
    /// ```
    /// # use pert_ui_runtime as pert_ui;
    /// use pert_ui::prelude::*;
    /// use std::time::Duration;
    ///
    /// let mut ui = Runtime::new();
    /// ui.set_viewport(Viewport::new(300.0, 100.0, 1.0)?)?;
    /// let root = ui.mount_tree(Element::new("form", Column::new([
    ///     Element::new("name", TextField::new(ui.signal(String::new()))),
    ///     Element::new("title", Label::new("Name")),
    /// ])))?;
    /// ui.flush(Duration::ZERO)?;
    /// let name = ui.find(&root, &["name"])?.ok_or("not mounted")?;
    /// let title = ui.find(&root, &["title"])?.ok_or("not mounted")?;
    /// assert!(ui.can_focus(&name)?);
    /// assert!(!ui.can_focus(&title)?); // a label takes no focus
    /// ui.set_visible(&name, false)?;
    /// assert!(!ui.can_focus(&name)?);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// # Errors
    /// [`RuntimeError::ForeignRuntime`] for an ID of another runtime, and
    /// nothing else: an unmounted or hidden component, including every
    /// component after [`Self::shutdown`], answers `false`. Reading needs no
    /// phase check, since it writes nothing.
    pub fn can_focus(&self, id: &WidgetId) -> Result<bool, RuntimeError> {
        match self.check(id) {
            Ok(()) => {}
            Err(RuntimeError::Unmounted) => return Ok(false),
            Err(error) => return Err(error),
        }
        Ok(id.0.visible.get()
            && self
                .entries
                .get(&id.0.serial)
                .is_some_and(|entry| entry.component.focusable())
            && self.modal_allows_in_own_window(id.0.serial))
    }
    /// The refusals of a focus transfer, checked in the target's window before
    /// anything changes.
    fn admit_focus(&self, id: &WidgetId) -> Result<(), RuntimeError> {
        self.check(id)?;
        if !id.0.visible.get() {
            return Err(RuntimeError::Hidden);
        }
        if !self.modal_allows(id.0.serial) {
            return Err(RuntimeError::ModalBlocked);
        }
        Ok(())
    }
    // Pointer presses already hit visible content; preserve that hit position.
    /// Presentation ancestors of a focused node, innermost first, excluding it.
    pub(super) fn focus_ancestors(node: &Rc<Node>) -> Vec<u64> {
        let mut serials = Vec::new();
        let mut current = node.presentation_parent();
        while let Some(parent) = current {
            serials.push(parent.serial);
            current = parent.presentation_parent();
        }
        serials
    }
    /// The ancestors last told that focus is within them and still mounted,
    /// visible and presented in this window, innermost first.
    pub(super) fn notified_focus_chain(&self) -> Vec<u64> {
        self.window
            .focus_within
            .iter()
            .copied()
            .filter(|serial| {
                self.entries.get(serial).is_some_and(|entry| {
                    entry.node.visible.get()
                        && Rc::ptr_eq(&entry.node.window(), &self.window.shared)
                })
            })
            .collect()
    }
    /// Ancestors that lose focus-within (innermost first) and gain it
    /// (outermost first) when focus moves from `old` to `new`, measured
    /// against what they were last told, and records the new chain.
    pub(super) fn focus_within_transfer(
        &mut self,
        old: Option<&WidgetId>,
        new: &WidgetId,
    ) -> (Vec<u64>, Vec<u64>) {
        let told = self.notified_focus_chain();
        let new_chain = Self::focus_ancestors(&new.0);
        let told_set: std::collections::BTreeSet<u64> = told.iter().copied().collect();
        let new_all: std::collections::BTreeSet<u64> =
            new_chain.iter().copied().chain([new.0.serial]).collect();
        let old_serial = old.map(|old| old.0.serial);
        let leaving = told
            .into_iter()
            .filter(|serial| !new_all.contains(serial))
            .collect();
        let entering = new_chain
            .iter()
            .rev()
            .copied()
            .filter(|serial| !told_set.contains(serial) && Some(*serial) != old_serial)
            .collect();
        self.window.focus_within = new_chain;
        (leaving, entering)
    }
    /// Keeps the first focus error as the call's result and queues the rest
    /// for later flushes, so none is dropped.
    fn keep_focus_errors(
        &mut self,
        result: &mut Result<(), RuntimeError>,
        errors: Vec<RuntimeError>,
    ) {
        for error in errors {
            if result.is_ok() {
                *result = Err(error);
            } else {
                self.window.focus_errors.push_back(error);
            }
        }
    }
    /// Tells each still-mounted component in `serials`, in order, that focus
    /// entered or left its subtree, and returns every error.
    pub(super) fn notify_focus_within(
        &mut self,
        serials: Vec<u64>,
        within: bool,
    ) -> Vec<RuntimeError> {
        let mut errors = Vec::new();
        for serial in serials {
            if let Some(entry) = self.entries.get_mut(&serial)
                && let Err(error) = entry.component.focus_within_changed(
                    &mut EventContext {
                        node: entry.node.clone(),
                    },
                    within,
                )
            {
                errors.push(error);
            }
        }
        errors
    }
    fn focus_without_reveal(&mut self, id: &WidgetId) -> Result<(), RuntimeError> {
        self.focus_with_reveal(id, false)
    }
    fn focus_with_reveal(&mut self, id: &WidgetId, reveal: bool) -> Result<(), RuntimeError> {
        let _timing = self.profile_target(id, crate::ProfileOperation::Focus);
        self.admit_focus(id)?;
        self.select_content_target(id.0.serial);
        self.raise_ancestors(id.0.serial);
        self.window.shared.requested_focus.borrow_mut().take();
        let mut focus_result = Ok(());
        if self.window.focus.as_ref() != Some(id) {
            let old = self.window.focus.clone();
            if let Some(old) = &old {
                self.note_surface_departures(old.0.serial, id.0.serial);
            }
            let (leaving, entering) = self.focus_within_transfer(old.as_ref(), id);
            if let Some(old) = self.window.focus.take() {
                old.0.invalidate_focus_path();
                // Focus moving into the old recipient's own subtree, as into a
                // row's embedded editor, keeps its gestures, capture and
                // timers: it still owns the interaction it started.
                let into_subtree = Self::focus_ancestors(&id.0).contains(&old.0.serial);
                if !into_subtree {
                    old.0.stop_deadline();
                    old.0.lose_context();
                    // Focus loss has already ended this incarnation's input.
                    // Later event failure cleanup must not repeat that lifecycle.
                    old.0.input_cancelled.set(true);
                }
                if let Some(entry) = self.entries.get_mut(&old.0.serial) {
                    focus_result = entry.component.focus_lost(&mut EventContext {
                        node: entry.node.clone(),
                    });
                }
                old.0.invalidate(Invalidation::Paint);
                #[cfg(debug_assertions)]
                self.validator
                    .flags(old.0.serial, old.0.visible.get(), false);
            }
            let errors = self.notify_focus_within(leaving, false);
            self.keep_focus_errors(&mut focus_result, errors);
            self.window.restore_focus = false;
            id.0.invalidate_focus_path();
            self.window.focus = Some(id.clone());
            self.window.focus_was_focusable = self
                .entries
                .get(&id.0.serial)
                .is_some_and(|entry| entry.component.focusable());
            self.window.shared.focused.set(Some(id.0.serial));
            *self.window.shared.focus_node.borrow_mut() = Some(Rc::downgrade(&id.0));
            // Gaining focus starts a fresh input context even without a key.
            id.0.input_cancelled.set(false);
            let errors = self.notify_focus_within(entering, true);
            self.keep_focus_errors(&mut focus_result, errors);
            if let Some(entry) = self.entries.get_mut(&id.0.serial) {
                entry.component.focus_gained(&mut EventContext {
                    node: entry.node.clone(),
                });
            }
            id.0.invalidate(Invalidation::Paint);
            #[cfg(debug_assertions)]
            self.validator.flags(id.0.serial, true, true);
            self.shared
                .record(crate::EventKind::Focus, Some(id.0.serial), None, || {
                    crate::EventStatus::Succeeded
                });
        }
        if reveal && let Err(error) = self.reveal_focus(id) {
            // The reveal error is returned; a held callback error is queued
            // ahead of the later ones rather than dropped.
            if let Err(held) = focus_result {
                self.window.focus_errors.push_front(held);
            }
            return Err(error);
        }
        focus_result
    }

    /// Applies a physical key transition and dispatches it to the focused component.
    ///
    /// An ignored key bubbles through the focused component's ancestors. With
    /// nothing focused, a key other than Tab goes to the first visible component
    /// that opts in through [`Component::receives_unfocused_keys`], such as a
    /// `MenuBar`, and is otherwise dropped; Tab moves focus as usual.
    ///
    /// Repeats do not add another held key. An orphan `Repeat`, for a key that
    /// is not held (for example one pressed before focus arrived), never
    /// establishes a press: it is returned as `Repeated` without reaching any
    /// component, and the key's later release is `UnmatchedRelease`, as through
    /// [`Runtime::command_key_input`].
    ///
    /// An Escape press (not a repeat or a release) goes to the first of these
    /// that takes it:
    ///
    /// 1. During a drag session it cancels the drag and reaches no component.
    /// 2. Without modifiers, shown delayed help takes it and closes.
    /// 3. While a popup, menu or dialog is open, it is offered to the focused
    ///    editors as the native path's text phase offers it (see
    ///    [`Component::text_key_event`]), so an editor that answers it, such
    ///    as a search field with a query, keeps the surface open.
    /// 4. Otherwise the topmost interactive surface it may dismiss closes.
    /// 5. Only then, when a component is focused, its pointer capture, its
    ///    timer, a drag it started and its parameter edit sessions (begun with
    ///    [`Parameter::begin_edit`](crate::Parameter::begin_edit)) end, without
    ///    calling [`Component::cancel_input`], and the component receives
    ///    Escape as an ordinary key, bubbling as any key does. With nothing
    ///    focused it goes to the unfocused-key recipient with no such cleanup.
    ///
    /// Where an earlier step takes Escape, step 5's cleanup of the focused
    /// component does not run. The consumer's own lifecycle still cleans up as
    /// it always does: cancelling a drag releases the source's pointer capture
    /// and reports [`DragOutcome::Cancelled`](crate::DragOutcome::Cancelled) to
    /// its [`Component::drag_finished`]; closing delayed help or a surface hides
    /// or removes what it showed, and each component hidden or removed that way
    /// loses its capture, timer and edit sessions and receives
    /// [`Component::cancel_input`], as hiding with [`Self::set_visible`] does,
    /// with focus that was inside moving elsewhere. A component the consumer
    /// does not touch keeps its capture, timer and edit sessions. Released keys
    /// never automatically commit a generic gesture: a control chooses its own
    /// confirmation key/action.
    ///
    /// This is the headless entry point: it routes neither command bindings nor
    /// [`Runtime::register_held_input`] registrations, which only
    /// [`Runtime::command_key_input`], the native path, routes. Use it to
    /// drive components directly; a test that claims what an application's
    /// user sees from a key uses `command_key_input` instead.
    ///
    /// The key is delivered at once and its recipient writes what its own
    /// rules write, as [`Self::text_input`] describes; the tree, derived
    /// bindings and layout show the result after the next [`Self::flush`].
    ///
    /// A synthesized press of a key that is physically held is a repeat
    /// ([`KeyTransition::Repeated`](crate::KeyTransition::Repeated)), as the
    /// keyboard state classifies it. Controls that act on a fresh press, such
    /// as [`InlineText`](crate::InlineText)'s F2, ignore it. To start editing
    /// from code, use [`TextCommand::BeginEdit`](crate::TextCommand::BeginEdit).
    ///
    /// # Errors
    /// Propagates component input errors, and the errors of the flush that
    /// delivers a notification the key requested, including a
    /// [`Component::deliver_notifications`] error; physical state remains
    /// updated so it cannot become stuck merely because a component rejected
    /// an event. Older or external notification errors propagate without cancelling
    /// this key's input ownership after dispatch. If a preflush prevents a held
    /// key's release from reaching its owner, cleanup still ends its held actions.
    /// Direct or attributable failures clean up each mounted component actually
    /// called, including keyboard and Escape-text bubbling. Preparation and
    /// callbacks may transfer focus; cleanup never retargets the new focus.
    /// With no delivered callback, failure cleanup retains the preflush owner.
    pub fn key_input(
        &mut self,
        code: crate::KeyCode,
        action: crate::KeyAction,
    ) -> Result<crate::KeyEvent, RuntimeError> {
        let _timing = self
            .shared
            .profiler
            .span(crate::ProfileOperation::KeyInput, None, "");
        self.shared.writable()?;
        let notifications = notify::InputNotificationGuard::new(self.shared.clone());
        let preflush_owner = self.window.focus.clone().or_else(|| {
            self.unfocused_key_recipient()
                .map(|serial| WidgetId(self.entries[&serial].node.clone()))
        });
        if let Some(owner) = &preflush_owner {
            owner.0.input_cancelled.set(false);
        }
        // Cancel pending coordinate changes while pre-release key ownership is
        // still observable. Physical state advances even when that flush fails.
        let geometry = if self.geometry_pending() {
            self.flush(self.shared.now.get()).map(|_| ())
        } else {
            Ok(())
        };
        let orphan = action == crate::KeyAction::Repeat && !self.keyboard.is_pressed(code);
        let event = self.keyboard.apply(code, action);
        let modifiers_changed =
            self.shared.modifiers.replace(self.keyboard.modifiers()) != self.keyboard.modifiers();
        notifications.activate();
        let mut recipients = Vec::new();
        let result = geometry.and_then(|_| {
            if orphan {
                // Nothing changed: an orphan repeat is refused before delivery.
                return Ok(());
            }
            if let Some(owner) = &self.window.focus {
                owner.0.input_cancelled.set(false);
            }
            if self.drag_in_window() {
                // Escape cancels the drag rather than reaching a surface or the
                // focused component; a modifier may change the offered operation.
                if event.code == crate::KeyCode::Escape
                    && event.transition == crate::KeyTransition::Pressed
                {
                    return self.finish_drag(false).map(|_| ());
                }
                if modifiers_changed {
                    self.resolve_drag()?;
                }
            }
            self.key_transition(event, &mut recipients)
        });
        // A notification the event requested fails like the event itself.
        let result = result.and_then(|()| self.deliver_input_notifications());
        if let Err(error) = result {
            if notifications.cancel_on_error(event.transition == crate::KeyTransition::Released) {
                // Preparation may transfer focus before dispatch, and callbacks
                // may transfer it again. Clean the actual called incarnations.
                // With no delivered callback, preserve preflush-release ownership.
                if recipients.is_empty() {
                    recipients.extend(preflush_owner);
                }
                for id in recipients {
                    if let Some(entry) = self.entries.get_mut(&id.0.serial)
                        && Rc::ptr_eq(&entry.node, &id.0)
                        && entry.node.mounted.get()
                        && !entry.node.input_cancelled.replace(true)
                    {
                        entry.node.lose_context();
                        entry.component.cancel_input(&mut EventContext {
                            node: entry.node.clone(),
                        });
                    }
                }
            }
            return Err(error);
        }
        Ok(event)
    }
    /// Offers a fresh Escape to the focused editors before an open surface
    /// closes on it, as [`Runtime::command_key_input`]'s text phase does, so
    /// an editor inside a popup (a search field clearing its query, a draft
    /// being discarded) answers the first Escape on both input paths. Runs
    /// only while an interactive surface is open, the one case where the
    /// order changes the outcome; returns whether an editor handled the key.
    fn escape_text_phase(
        &mut self,
        event: crate::KeyEvent,
        recipients: &mut Vec<WidgetId>,
    ) -> Result<bool, RuntimeError> {
        if !self.window.surfaces.iter().any(|s| s.kind.interactive()) {
            return Ok(false);
        }
        let input = crate::CommandKeyInput::new(
            crate::KeyCode::Escape,
            crate::KeyAction::Press,
            crate::CommandKey::Named(crate::KeyCode::Escape),
        );
        let mut serial = self.window.focus.as_ref().map(|id| id.0.serial);
        while let Some(current) = serial {
            let entry = self
                .entries
                .get_mut(&current)
                .ok_or(RuntimeError::Unmounted)?;
            entry.node.input_cancelled.set(false);
            recipients.push(WidgetId(entry.node.clone()));
            if let Some(input) = self.shared.notification_input.borrow().as_ref() {
                input.dispatched.set(true);
            }
            let outcome = entry.component.text_key_event(
                &mut EventContext {
                    node: entry.node.clone(),
                },
                input,
                event.transition,
                &self.keyboard,
            )?;
            match outcome {
                crate::TextKeyOutcome::Handled | crate::TextKeyOutcome::InsertText => {
                    return Ok(true);
                }
                crate::TextKeyOutcome::Ignored => {
                    serial = entry.node.presentation_parent_serial();
                }
                crate::TextKeyOutcome::NotAnEditor => return Ok(false),
            }
        }
        Ok(false)
    }
    fn unfocused_key_recipient(&self) -> Option<u64> {
        self.navigation_order().into_iter().find(|serial| {
            self.modal_navigation_allows(*serial)
                && self.entries[serial].component.receives_unfocused_keys()
        })
    }
    fn key_transition(
        &mut self,
        event: crate::KeyEvent,
        recipients: &mut Vec<WidgetId>,
    ) -> Result<(), RuntimeError> {
        if !self.window.shared.pending.borrow().is_empty() {
            self.flush(self.shared.now.get())?;
        }
        if event.code == crate::KeyCode::Escape
            && event.transition == crate::KeyTransition::Pressed
            && (self.tooltip_escape()?
                || self.escape_text_phase(event, recipients)?
                || self.surface_escape()?)
        {
            return Ok(());
        }
        if self.window.shared.layout_pending.get() {
            self.flush(self.shared.now.get())?;
        }
        if event.code == crate::KeyCode::Tab
            && !self.keyboard.modifiers().control
            && !self.keyboard.modifiers().alt
            && !self.keyboard.modifiers().super_key
            && self.entries.values().any(|entry| {
                Rc::ptr_eq(&entry.node.window(), &self.window.shared)
                    && entry.node.visible.get()
                    && entry.component.focusable()
            })
            && event.transition != crate::KeyTransition::Released
            && event.transition != crate::KeyTransition::UnmatchedRelease
        {
            if let Some(input) = self.shared.notification_input.borrow().as_ref() {
                input.dispatched.set(true);
            }
            if !self.offer_tab(event, Some(recipients))? {
                self.focus_next(self.tab_direction())?;
            }
        } else if let Some(id) = self.window.focus.clone() {
            if event.code == crate::KeyCode::Escape
                && event.transition == crate::KeyTransition::Pressed
            {
                id.0.lose_context();
            }
            if let Some(input) = self.shared.notification_input.borrow().as_ref() {
                input.dispatched.set(true);
            }
            self.dispatch_keyboard_tracked(id.0.serial, event, Some(recipients))?;
        } else if let Some(serial) = self.unfocused_key_recipient() {
            if let Some(input) = self.shared.notification_input.borrow().as_ref() {
                input.dispatched.set(true);
            }
            self.dispatch_keyboard_tracked(serial, event, Some(recipients))?;
        }
        Ok(())
    }

    /// Installs a UI-thread clipboard service, replacing and dropping the old
    /// service outside internal borrows. Runtime shutdown drops the service even
    /// when UI handles survive. Native App configures this before initialization;
    /// headless hosts may inject
    /// a service for deterministic tests.
    ///
    /// # Errors
    /// Returns [`RuntimeError::Closed`] after shutdown, and `WriteDuringUpdate`,
    /// `WriteDuringLayout`, `WriteDuringPaint`, `WriteDuringInspection` or
    /// `WriteDuringDelivery` when called from inside that phase; and `BorrowConflict` when
    /// called from inside the clipboard service. The old service is kept on error.
    pub fn set_clipboard(
        &mut self,
        clipboard: impl crate::Clipboard + 'static,
    ) -> Result<(), RuntimeError> {
        self.shared.writable()?;
        let previous = self
            .shared
            .clipboard
            .try_borrow_mut()
            .map_err(|_| RuntimeError::BorrowConflict)?
            .replace(Box::new(clipboard));
        drop(previous);
        Ok(())
    }
    /// Reads the clipboard's text on the UI thread, from application code or
    /// a command action, without involving any component or moving focus.
    ///
    /// The native `App` installs the platform clipboard; a headless runtime
    /// has none until [`Self::set_clipboard`] installs a service.
    ///
    /// # Errors
    /// As [`EventContext::clipboard_text`]: `Closed` after shutdown, the phase
    /// errors of a write from inside update, layout, paint, inspection or
    /// delivery, `BorrowConflict` from inside the clipboard service,
    /// [`ClipboardError::Unavailable`](crate::ClipboardError::Unavailable) without a
    /// service, and the service's own [`ClipboardError`](crate::ClipboardError).
    pub fn clipboard_text(&self) -> Result<String, RuntimeError> {
        self.shared.clipboard_read()
    }
    /// Writes text to the clipboard on the UI thread, from application code
    /// or a command action, without involving any component or moving focus:
    /// a "Copy" command that copies the application's own data uses this.
    ///
    /// ```
    /// # use pert_ui_runtime as pert_ui;
    /// use pert_ui::*;
    /// struct Memory(std::rc::Rc<std::cell::RefCell<String>>);
    /// impl Clipboard for Memory {
    ///     fn read_text(&mut self) -> Result<String, ClipboardError> { Ok(self.0.borrow().clone()) }
    ///     fn write_text(&mut self, text: &str) -> Result<(), ClipboardError> {
    ///         *self.0.borrow_mut() = text.to_owned();
    ///         Ok(())
    ///     }
    /// }
    /// let mut ui = Runtime::new();
    /// let held = std::rc::Rc::new(std::cell::RefCell::new(String::new()));
    /// ui.set_clipboard(Memory(held.clone()))?;
    /// let copy = ui.register_command("Copy line", CommandScope::Application, vec![],
    ///     |ui, _| ui.set_clipboard_text("12:00:01 INFO started"))?;
    /// let context = ui.command_context(CommandSource::Programmatic);
    /// ui.invoke_command(&copy, context)?;
    /// assert_eq!(*held.borrow(), "12:00:01 INFO started");
    /// assert_eq!(ui.clipboard_text()?, "12:00:01 INFO started");
    /// # Ok::<(), RuntimeError>(())
    /// ```
    ///
    /// # Errors
    /// As [`Self::clipboard_text`]; the service reports a failed write as its
    /// own [`ClipboardError`](crate::ClipboardError).
    pub fn set_clipboard_text(&self, text: &str) -> Result<(), RuntimeError> {
        self.shared.clipboard_write(text)
    }
    /// Returns the focused text recipient and its clipped window-logical IME
    /// anchor. Absent for non-editable, hidden or fully clipped components.
    pub fn text_input_state(&self) -> Option<(WidgetId, crate::TextInputState)> {
        let id = self.window.focus.as_ref()?;
        let entry = self.entries.get(&id.0.serial)?;
        if !id.0.visible.get() {
            return None;
        }
        let state = entry.component.text_input_state()?;
        let [x, y, _, _] = id.0.bounds.get()?.values();
        let caret = crate::transform::map_rect(
            state.caret,
            (f64::from(x), f64::from(y)),
            id.0.scale.get(),
        )?
        .intersection(id.0.clip.get()?);
        let [_, _, w, h] = caret.values();
        if w <= 0.0 || h <= 0.0 {
            return None;
        }
        Some((
            id.clone(),
            crate::TextInputState {
                caret,
                session: state.session,
            },
        ))
    }
    /// Dispatches temporary IME composition to the current focused component.
    /// Flushes pending binding changes first. No focus ignores
    /// the event. Adapters must discard native events from a previous focus owner.
    ///
    /// # Errors
    /// Returns [`RuntimeError::Closed`] after shutdown and the phase errors of any write,
    /// forwards the errors of the [`Self::flush`] that runs first, then the focused
    /// component's [`Component::text_preedit`] error, and then the errors of the flush
    /// that delivers a notification the composition requested, including a
    /// [`Component::deliver_notifications`] error.
    pub fn text_preedit(&mut self, preedit: &crate::TextPreedit) -> Result<(), RuntimeError> {
        self.shared.writable()?;
        self.flush(self.shared.now.get())?;
        if let Some(id) = self.window.focus.clone()
            && let Some(entry) = self.entries.get_mut(&id.0.serial)
        {
            entry.node.input_cancelled.set(false);
            entry
                .component
                .text_preedit(&mut EventContext { node: id.0.clone() }, preedit)?;
            id.0.invalidate(Invalidation::Paint);
        }
        self.deliver_input_notifications()
    }
    /// Dispatches committed text to the focused component without changing keys.
    ///
    /// Empty text inserts nothing and leaves a selection in place, as an input
    /// method's empty commit must: deleting a selection takes a Backspace or
    /// Delete key. The text is delivered at once and is written as the focused
    /// component's own rules write it: a plain text editor writes its binding
    /// immediately, while a draft, a parameter proposal and a notification for
    /// a hidden window are not written yet. Derived bindings, dynamic children
    /// and layout show the result after the next [`Self::flush`], so a test
    /// flushes before it inspects.
    ///
    /// # Errors
    /// Forwards the errors of the [`Self::flush`] that runs first, the component's
    /// text-input error, and the errors of the flush that delivers a notification
    /// the text requested, including a [`Component::deliver_notifications`] error.
    /// With no focus, text is ignored.
    pub fn text_input(&mut self, text: &str) -> Result<(), RuntimeError> {
        self.shared.writable()?;
        self.flush(self.shared.now.get())?;
        let _timing = self
            .shared
            .profiler
            .span(crate::ProfileOperation::TextInput, None, "");
        if let Some(id) = self.window.focus.clone()
            && let Some(entry) = self.entries.get_mut(&id.0.serial)
        {
            entry.node.input_cancelled.set(false);
            id.0.invalidate(Invalidation::Paint);
            let _timing = self.shared.profiler.span(
                crate::ProfileOperation::TextInput,
                Some(id.0.serial),
                &entry.key,
            );
            let result = entry
                .component
                .text_input(&mut EventContext { node: id.0.clone() }, text);
            self.shared.record(
                crate::EventKind::TextInput { bytes: text.len() },
                Some(id.0.serial),
                None,
                || match &result {
                    Ok(()) => crate::EventStatus::Succeeded,
                    Err(error) => crate::EventStatus::Failed(error.to_string()),
                },
            );
            result?;
        }
        self.deliver_input_notifications()
    }

    /// Cancels active input after a host/component failure without transferring focus.
    ///
    /// Clears physical keys/buttons, requested focus and pointer capture, and calls
    /// [`Component::cancel_input`] on the focused/captured recipients and their
    /// ancestor paths, once per component (events may bubble). Authoritative
    /// gestures cancel; local numeric drafts remain while their field retains focus.
    /// Does not call `focus_lost`, save drafts or replay the failed event. Custom
    /// components must release held actions without activating application commands.
    /// Pending updates and diagnostic errors remain available for repair and retry.
    ///
    /// ```
    /// # use pert_ui_runtime as pert_ui;
    /// use pert_ui::{Element, IntegerInput, Runtime};
    /// let mut ui = Runtime::new();
    /// let value = ui.signal(1_i64);
    /// let field = ui.mount_tree(Element::new("number", IntegerInput::new(value.clone())))?;
    /// ui.focus(&field)?;
    /// ui.text_input("2")?; // local draft, not saved
    /// ui.cancel_input()?;
    /// assert_eq!(value.get()?, 1);
    /// assert_eq!(ui.focused(), Some(field));
    /// # Ok::<(), pert_ui::RuntimeError>(())
    /// ```
    ///
    /// # Errors
    /// Returns only a runtime phase error (see [`Runtime`]), before changing state.
    pub fn cancel_input(&mut self) -> Result<(), RuntimeError> {
        self.shared.writable()?;
        self.cancel_drag_infallibly();
        self.cancel_held(crate::HeldCancellation::InputFailure);
        self.suppressed_keys
            .extend(self.command_presses.keys().copied());
        self.command_presses.clear();
        self.keyboard.clear();
        self.shared.modifiers.set(crate::Modifiers::default());
        self.window.buttons.clear();
        self.window.shared.held_buttons.set(0);
        self.window.shared.requested_focus.borrow_mut().take();
        self.window.restore_focus = false;
        let recipients = self.input_recipients();
        self.window.shared.capture.set(None);
        self.cancel_recipients(recipients);
        Ok(())
    }

    fn input_recipients(&self) -> Vec<u64> {
        let mut recipients = Vec::new();
        let mut seen = std::collections::BTreeSet::new();
        for mut serial in [
            self.window.shared.focused.get(),
            self.window.shared.capture.get(),
        ] {
            while let Some(id) = serial {
                if !seen.insert(id) {
                    break;
                }
                recipients.push(id);
                serial = self
                    .entries
                    .get(&id)
                    .and_then(|entry| entry.node.presentation_parent_serial());
            }
        }
        recipients
    }
    fn cancel_recipients(&mut self, recipients: impl IntoIterator<Item = u64>) {
        for serial in recipients {
            if let Some(entry) = self.entries.get_mut(&serial) {
                entry.node.lose_context();
                entry.node.stop_deadline();
                entry.node.animated.set(false);
                entry.node.input_cancelled.set(true);
                entry.component.cancel_input(&mut EventContext {
                    node: entry.node.clone(),
                });
                entry.node.invalidate(Invalidation::Paint);
            }
        }
    }

    /// Handles window focus loss: clears all held keys and cancels focused edits.
    ///
    /// Calls `Component::focus_lost` for the focused recipient; local editors may
    /// save valid drafts. Errors are queued for `flush`, without preventing input
    /// cleanup. Other captured input and ancestor input handlers are canceled once
    /// per component, since key/pointer events may bubble. Late OS up notifications become
    /// `UnmatchedRelease`, preventing stuck-key actions.
    pub fn clear_focus(&mut self) {
        if self.drag_in_window() || self.drag.is_none() {
            self.cancel_drag_infallibly();
        }
        self.cancel_held(crate::HeldCancellation::FocusLost);
        let recipients = self.input_recipients();
        self.close_all_surfaces(crate::SurfaceDismissal::WindowFocusLost);
        // Modal chains survive native focus loss; clearing physical input must
        // not turn switching applications into an implicit dialog response.
        self.window.surfaces.retain(|surface| surface.kind.modal());
        self.window.dismissed_buttons.clear();
        self.update_hover_path(None);
        let _timing = self
            .shared
            .profiler
            .span(crate::ProfileOperation::Focus, None, "");
        self.window.shared.requested_focus.borrow_mut().take();
        self.shared.record(crate::EventKind::Focus, None, None, || {
            crate::EventStatus::Succeeded
        });
        self.window.restore_focus = false;
        self.suppressed_keys
            .extend(self.command_presses.keys().copied());
        self.command_presses.clear();
        self.keyboard.clear();
        self.shared.modifiers.set(crate::Modifiers::default());
        self.window.buttons.clear();
        self.window.shared.held_buttons.set(0);
        let focused = self.window.shared.focused.get();
        self.window.shared.capture.set(None);
        // Ancestors told that focus is within them hear that it left before
        // their own input is cancelled, so their focus policy still sees the
        // state it applies to; everything else is cancelled first, as before.
        let told = self.notified_focus_chain();
        let (later, now): (Vec<u64>, Vec<u64>) = recipients
            .into_iter()
            .filter(|id| Some(*id) != focused)
            .partition(|id| told.contains(id));
        self.cancel_recipients(now);
        self.window.shared.pointer_position.set(None);
        self.window.pointer_target = None;
        self.window.shared.focused.set(None);
        self.window.shared.focus_node.borrow_mut().take();
        if let Some(id) = self.window.focus.take() {
            id.0.input_cancelled.set(true);
            id.0.invalidate_focus_path();
            id.0.stop_deadline();
            id.0.lose_context();
            if let Some(entry) = self.entries.get_mut(&id.0.serial)
                && let Err(error) = entry.component.focus_lost(&mut EventContext {
                    node: entry.node.clone(),
                })
            {
                self.window.focus_errors.push_back(error);
            }
            id.0.invalidate(Invalidation::Paint);
            #[cfg(debug_assertions)]
            self.validator.flags(id.0.serial, id.0.visible.get(), false);
        }
        self.window.focus_within.clear();
        let errors = self.notify_focus_within(told, false);
        self.window.focus_errors.extend(errors);
        self.cancel_recipients(later);
    }

    /// Changes presentation visibility, retaining component state and subscriptions.
    ///
    /// Hidden dirtiness accumulates but never schedules evaluation. Hiding cancels
    /// edit ownership and deadlines. Showing schedules one layout/paint refresh
    /// with current binding values; a component may then resume live scheduling.
    ///
    /// # Errors
    /// Returns `ForeignRuntime` for an ID of another runtime, `Unmounted` for an
    /// unmounted ID, which every ID is after shutdown, and
    /// `WriteDuringDelivery` from inside a parameter delivery callback, such as
    /// a sink that holds the runtime in a shared cell.
    pub fn set_visible(&mut self, id: &WidgetId, visible: bool) -> Result<(), RuntimeError> {
        let previous = self.current_window();
        self.select_window(&self.window_of(id)?)?;
        let result = self.set_visible_selected(id, visible);
        self.select_window(&previous)?;
        result
    }
    fn set_visible_selected(&mut self, id: &WidgetId, visible: bool) -> Result<(), RuntimeError> {
        let _timing = self.profile_target(id, crate::ProfileOperation::Visibility);
        self.shared.writable()?;
        self.check(id)?;
        id.0.local_visible.set(visible);
        let parent_visible =
            id.0.presentation_parent()
                .is_none_or(|parent| parent.visible.get());
        let allowed = id.0.presentation_parent_serial().is_none_or(|serial| {
            self.entries[&serial]
                .component
                .child_visible(&self.entries[&id.0.serial].key)
        });
        self.refresh_visibility(id.0.serial, parent_visible && allowed);
        if let Some(parent) = id.0.presentation_parent() {
            parent.invalidate(Invalidation::Layout);
        }
        #[cfg(debug_assertions)]
        self.refresh_validation();
        Ok(())
    }

    /// Recursively releases a mount and descendants, children before their parent.
    ///
    /// Teardown, including the dismissal of a transient surface the subtree owns,
    /// completes inside this call. Failed cleanup callbacks retain typed errors
    /// for later flushes on a surviving window, or the shutdown report; they
    /// keep a visible surviving window due until consumed and never retry
    /// the removed owner.
    /// [`Runtime::next_deadline`] reports work afterwards only for what survives:
    /// a remaining parent that must arrange its other children, or focus that was
    /// inside the subtree and has to be restored, or a retained cleanup error.
    ///
    /// # Errors
    /// Returns a runtime phase error (see [`Runtime`]), [`RuntimeError::ForeignRuntime`]
    /// for an ID of another runtime or [`RuntimeError::Unmounted`] for an
    /// already unmounted one. Its old identity remains invalid after another
    /// component is mounted using the same logical key.
    pub fn unmount(&mut self, id: &WidgetId) -> Result<(), RuntimeError> {
        let _timing = self.profile_target(id, crate::ProfileOperation::Unmount);
        self.shared.writable()?;
        self.check(id)?;
        if let Some(parent) = id.0.logical_parent() {
            if let Some(entry) = self.entries.get_mut(&parent.serial) {
                entry.children.retain(|serial| *serial != id.0.serial);
                entry
                    .raised_children
                    .retain(|serial| *serial != id.0.serial);
            }
            parent.invalidate(Invalidation::Layout);
        }
        self.remove_subtree(id.0.serial);
        #[cfg(debug_assertions)]
        self.refresh_validation();
        Ok(())
    }

    /// Invalidates a mount for external geometry/surface events.
    ///
    /// Normal bindings invalidate automatically; this is a platform/component
    /// extension for events such as resizing. Hidden mounts only retain dirtiness.
    ///
    /// # Errors
    /// Returns [`RuntimeError::ForeignRuntime`] for an ID of another runtime and
    /// [`RuntimeError::Unmounted`] for an unmounted one.
    pub fn invalidate(&self, id: &WidgetId, kind: Invalidation) -> Result<(), RuntimeError> {
        self.check(id)?;
        id.0.invalidate(kind);
        Ok(())
    }

    /// Performs a text command (clipboard, selection or beginning an edit) on one
    /// component's own text, for hosts and application chrome such as an Edit
    /// menu acting on the focused editor, or a command that renames a label.
    ///
    /// Delivery is immediate and synchronous, unlike
    /// [`EventContext::request_text_command`], which a component uses to reach
    /// another component during a flush. A component that is not an editor reports
    /// `NotAnEditor` without an error; when the focused component lies inside its
    /// subtree, the command goes on to that one, so a composite control whose text
    /// mode is an embedded editor (such as a row with a
    /// [`DraftEditor`](crate::DraftEditor)) answers through it. A resting
    /// label has no focused descendant, so code that begins an edit targets the
    /// label itself.
    ///
    /// No command moves focus except
    /// [`TextCommand::BeginEdit`](crate::TextCommand::BeginEdit): an editor
    /// that answers it `Handled` is editing on return, and its focus request
    /// moves focus at the next flush, since this call does not flush.
    ///
    /// # Errors
    /// Rejects foreign, unmounted, hidden and modal-blocked IDs (`Hidden` and
    /// `ModalBlocked`, as [`Self::event`]
    /// does) before the editor sees the command, so a hidden editor's selection
    /// is never cut or replaced. Clipboard failures and text the editor rejects for
    /// its characters propagate from the receiving editor, which preserves its
    /// text in that case; a paste that would exceed the editor's size limit is
    /// refused as `Unavailable` rather than failing.
    pub fn text_command(
        &mut self,
        id: &WidgetId,
        command: crate::TextCommand,
    ) -> Result<crate::TextCommandOutcome, RuntimeError> {
        self.check(id)?;
        if !id.0.visible.get() {
            return Err(RuntimeError::Hidden);
        }
        if !self.entries.contains_key(&id.0.serial) {
            return Err(RuntimeError::Unmounted);
        }
        if !self.modal_allows_in_own_window(id.0.serial) {
            return Err(RuntimeError::ModalBlocked);
        }
        self.deliver_text_command(&id.0, command)
    }

    /// Delivers a text command to `node`, or, when it is not an editor, to the
    /// focused component inside its subtree: a composite control whose text
    /// mode is an embedded editor child answers through that editor.
    fn deliver_text_command(
        &mut self,
        node: &Rc<Node>,
        command: crate::TextCommand,
    ) -> Result<crate::TextCommandOutcome, RuntimeError> {
        let Some(entry) = self.entries.get_mut(&node.serial) else {
            return Ok(crate::TextCommandOutcome::NotAnEditor);
        };
        entry.node.input_cancelled.set(false);
        let outcome = entry.component.text_command(
            &mut EventContext {
                node: entry.node.clone(),
            },
            command,
        )?;
        if outcome != crate::TextCommandOutcome::NotAnEditor {
            return Ok(outcome);
        }
        let focused = node
            .window()
            .focus_node
            .borrow()
            .as_ref()
            .and_then(Weak::upgrade);
        let Some(focused) = focused.filter(|focused| {
            focused.serial != node.serial && focused.visible.get() && {
                let mut current = focused.presentation_parent();
                let mut inside = false;
                while let Some(parent) = current {
                    if parent.serial == node.serial {
                        inside = true;
                        break;
                    }
                    current = parent.presentation_parent();
                }
                inside
            }
        }) else {
            return Ok(outcome);
        };
        let Some(entry) = self.entries.get_mut(&focused.serial) else {
            return Ok(outcome);
        };
        entry.node.input_cancelled.set(false);
        entry.component.text_command(
            &mut EventContext {
                node: entry.node.clone(),
            },
            command,
        )
    }

    /// Returns the earliest elapsed deadline, or `None` when the runtime can sleep.
    #[must_use]
    pub fn next_deadline(&self) -> Option<Duration> {
        if !self.shared.tasks.borrow().is_empty()
            || self.drag_needs_service()
            || !self.shared.computations.borrow().is_empty()
            || !self.shared.command_requests.borrow().is_empty()
            || !self.shared.reveal_requests.borrow().is_empty()
            || self.shared.has_deferred()
            || !self.window.focus_errors.is_empty()
            || self.window.surface_error.is_some()
            || self.window.restore_focus
            || self.window.shared.stack_hover_pending.get()
            || !self.shared.requested_removals.borrow().is_empty()
            || !self.shared.requested_moves.borrow().is_empty()
            || !self.shared.requested_children.borrow().is_empty()
            || !self.shared.requested_text_commands.borrow().is_empty()
            || !self.window.shared.pending.borrow().is_empty()
            || self.window.shared.layout_pending.get()
            || self.shared.inspection_pending()
            || self.notifications_pending()
        {
            return Some(self.shared.now.get());
        }
        let update = self
            .window
            .shared
            .deadlines
            .borrow()
            .first_key_value()
            .map(|((time, _), _)| *time);
        let timer = self
            .window
            .shared
            .timers
            .borrow()
            .first_key_value()
            .map(|((time, _), _)| *time);
        [
            update,
            timer,
            self.interval_deadline(),
            self.shared.scale_settle_deadline(),
            self.shared.source_deadline(self.shared.now.get()),
        ]
        .into_iter()
        .flatten()
        .min()
    }

    /// Delivers every event-phase timer that is due in the selected window, in
    /// deadline order. Each callback may write bindings, so it runs before the
    /// update waves of the same flush. A re-armed timer is strictly later than
    /// `now`, so this loop cannot starve the flush.
    fn dispatch_timers(&mut self, now: Duration) -> Result<(), RuntimeError> {
        loop {
            let due = self
                .window
                .shared
                .timers
                .borrow()
                .first_key_value()
                .map(|(key, _)| *key);
            let Some(key) = due.filter(|(time, _)| *time <= now) else {
                break;
            };
            let node = self
                .window
                .shared
                .timers
                .borrow_mut()
                .remove(&key)
                .and_then(|weak| weak.upgrade());
            let Some(node) = node else {
                continue;
            };
            node.timer.set(None);
            if !node.mounted.get() || !node.visible.get() {
                continue;
            }
            let Some(entry) = self.entries.get_mut(&key.1) else {
                continue;
            };
            let result = {
                let _timing = self.shared.profiler.span(
                    crate::ProfileOperation::Timer,
                    Some(key.1),
                    &entry.key,
                );
                entry.component.timer_elapsed(&mut EventContext { node })
            };
            self.shared.record(
                crate::EventKind::Timer,
                Some(key.1),
                None,
                || match &result {
                    Ok(()) => crate::EventStatus::Succeeded,
                    Err(error) => crate::EventStatus::Failed(error.to_string()),
                },
            );
            result?;
        }
        Ok(())
    }

    /// Advances elapsed time, promotes due deadlines and evaluates pending mounts.
    ///
    /// Multiple writes before a flush coalesce. Static mounts are not scanned on
    /// live ticks. Newly exposed descendants update before their first measurement.
    /// Missed deadlines produce one update; no catch-up loop is used.
    /// A failed component retains its work for retry, and other pending mounts
    /// remain queued. Dependency sets are replaced even after a returned error.
    /// Change notifications follow global request order across visible windows;
    /// their presentation and callback writes settle with the same wave budget.
    /// The previously selected window is restored when it remains mounted.
    /// Hidden native windows defer notifications until shown. Notification
    /// preparation does not repeat mailbox, task or timer delivery frontiers.
    ///
    /// Update/layout/focus/hover feedback is limited to [`Self::MAX_FLUSH_WAVES`],
    /// shared by internal presentation follow-ups. Ordinary queued reveals use
    /// their normal flush budgets independently of request count and path depth;
    /// an exhausted notification budget is not restarted for pending reveals.
    /// On exhaustion, repair the cause before retry;
    /// pending work remains queued. Development validation names pending components.
    /// A wave limit cannot interrupt a nonreturning callback or bound wall time.
    ///
    /// # Errors
    /// Returns `UpdateLimit` when feedback does not settle; rejects backwards time.
    /// Propagates a component's read/scheduling error or
    /// the oldest queued error from an infallible window focus-loss callback.
    pub fn flush(&mut self, now: Duration) -> Result<FlushReport, RuntimeError> {
        // Classification belongs to this flush's propagated error, not an error
        // a caller caught from an earlier invocation.
        if let Some(input) = self.shared.notification_input.borrow().as_ref() {
            input.error_from_current_input.set(None);
        }
        if self.shared.notification_settlement.get() && self.flush_state.depth.get() > 0 {
            if now < self.shared.now.get() {
                return Err(RuntimeError::TimeReversed);
            }
            self.shared.writable()?;
            return Ok(FlushReport {
                components_updated: self.settle_notification_presentation()?,
                next_deadline: self.next_deadline(),
            });
        }
        // A flush the runtime nests inside another (revealing focus scrolls
        // ancestors one flush each, a queued command flushes every window
        // before its action) and a per-window flush inside `flush_windows`
        // are not boundaries a reply may describe.
        if self.flush_state.depth.get() > 0 || self.shared.answer_depth.get() > 0 {
            let result = self.flush_inner(now);
            #[cfg(debug_assertions)]
            self.validate_failed_flush(&result);
            return result;
        }
        // A new outermost flush describes its own failure, if any.
        self.shared.reset_failure();
        let result = {
            let _boundary = AnswerBoundary::new(self.shared.clone());
            self.flush_inner(now)
        };
        #[cfg(debug_assertions)]
        self.validate_failed_flush(&result);
        self.answer_after_flush(result)
    }

    /// Answers queued inspection requests after an outermost flush, whether it
    /// succeeded or not, so a reply describes the state the flush left behind
    /// and an inspector called in to diagnose a failing flush is not left
    /// waiting. A successful report then states the deadline as it stands
    /// after answering, since the requests it counted are gone.
    fn answer_after_flush(
        &mut self,
        result: Result<FlushReport, RuntimeError>,
    ) -> Result<FlushReport, RuntimeError> {
        let flush_error = result.as_ref().err();
        let shared = self.shared.clone();
        shared.answer_inspection(self, flush_error);
        result.map(|report| FlushReport {
            next_deadline: self.next_deadline(),
            ..report
        })
    }

    fn flush_inner(&mut self, now: Duration) -> Result<FlushReport, RuntimeError> {
        let _timing = self
            .shared
            .profiler
            .span(crate::ProfileOperation::Flush, None, "");
        if now < self.shared.now.get() {
            return Err(RuntimeError::TimeReversed);
        }
        self.shared.writable()?;
        let external_origin = notify::NotificationOriginGuard::new(self.shared.clone(), None);
        self.shared.dispatch_deferred();
        if let Some(error) = self.window.focus_errors.pop_front() {
            return Err(error);
        }
        self.dispatch_queued_commands()?;
        let flush_guard = FlushGuard::new(self.flush_state.clone());
        self.apply_geometry_change()?;
        self.shared.now.set(now);
        self.dispatch_tasks()?;
        self.dispatch_intervals(now)?;
        self.dispatch_timers(now)?;
        let woke = self.shared.wake.reset();
        self.shared.service_sources(now, woke);
        self.shared.deliver_mailboxes()?;
        self.promote_due_deadlines(now);
        self.settle_view_scale(now);
        let mut count = self.settle_notification_presentation()?;
        drop(external_origin);
        let mut notification_error = None;
        loop {
            match self.dispatch_notifications() {
                Ok((updated, followup)) => {
                    count += updated;
                    if !followup {
                        break;
                    }
                }
                Err(error) => {
                    notification_error = Some(error);
                    break;
                }
            }
        }
        #[cfg(debug_assertions)]
        let outermost = flush_guard.outermost;
        // Ordinary reveals keep their normal flush budgets: request count and
        // path depth are not feedback. An exhausted notification budget must
        // not restart while pending reveals report their outcomes.
        let exhausted_guard = if self.flush_state.exhausted.get() {
            Some(flush_guard)
        } else {
            drop(flush_guard);
            None
        };
        // After the waves settle, so a surface closed in the same event (a
        // palette that jumps elsewhere) no longer blocks the target.
        let reveal_result = self.dispatch_reveal_requests();
        #[cfg(debug_assertions)]
        {
            // Keep exhaustion diagnostics until the boundary succeeds.
            if outermost {
                if notification_error.is_none() && reveal_result.is_ok() {
                    self.validator.clear_update_limit();
                }
                self.refresh_validation_now();
            }
        }
        drop(exhausted_guard);
        {
            let _origin = notify::NotificationOriginGuard::new(self.shared.clone(), None);
            self.shared.dispatch_deferred();
        }
        if let Some((error, current_input)) = notification_error {
            if let Some(input) = self.shared.notification_input.borrow().as_ref() {
                input.error_from_current_input.set(Some(current_input));
            }
            return Err(error);
        }
        reveal_result?;
        Ok(FlushReport {
            components_updated: count,
            next_deadline: self.next_deadline(),
        })
    }

    /// Settles presentation work only, without repeating external delivery frontiers.
    fn settle_notification_presentation(&mut self) -> Result<usize, RuntimeError> {
        if let Some(error) = self.window.focus_errors.pop_front() {
            return Err(error);
        }
        self.apply_geometry_change()?;
        let mut count = 0;
        loop {
            self.consume_flush_wave()?;
            self.shared.settle_computations()?;
            count += self.update_pending()?;
            if !self.window.shared.pending.borrow().is_empty() {
                continue;
            }
            self.apply_requested_moves()?;
            self.apply_requested_children()?;
            if !self.window.shared.pending.borrow().is_empty() {
                continue;
            }
            self.apply_requested_text_commands()?;
            self.apply_requested_removals()?;
            let had_layout = self.settle_layout()?;
            self.apply_requested_focus()?;
            self.refresh_stack_hover(had_layout)?;
            self.service_drag()?;
            // Ending a drag can request another hover/layout wave.
            if self.window.shared.pending.borrow().is_empty()
                && !self.window.shared.layout_pending.get()
                && !self.window.shared.stack_hover_pending.get()
                && self.shared.computations.borrow().is_empty()
            {
                break;
            }
        }
        Ok(count)
    }
    fn notification_presentation_pending(&self) -> bool {
        self.geometry_pending()
            || !self.window.shared.pending.borrow().is_empty()
            || self.window.shared.layout_pending.get()
            || !self.window.focus_errors.is_empty()
            || self.window.surface_error.is_some()
            || self.window.restore_focus
            || self.window.shared.requested_focus.borrow().is_some()
            || self.window.shared.stack_hover_pending.get()
            || !self.shared.computations.borrow().is_empty()
            || !self.shared.requested_removals.borrow().is_empty()
            || !self.shared.requested_moves.borrow().is_empty()
            || !self.shared.requested_children.borrow().is_empty()
            || !self.shared.requested_text_commands.borrow().is_empty()
            || self.drag_needs_service_in_window()
    }

    /// Promotes every deadline due by `now` to a paint invalidation.
    fn promote_due_deadlines(&self, now: Duration) {
        loop {
            let due = self
                .window
                .shared
                .deadlines
                .borrow()
                .first_key_value()
                .map(|(key, _)| *key);
            let Some(key) = due.filter(|(time, _)| *time <= now) else {
                break;
            };
            let weak = self.window.shared.deadlines.borrow_mut().remove(&key);
            if let Some(node) = weak.and_then(|weak| weak.upgrade()) {
                node.deadline.set(None);
                node.invalidate(Invalidation::Paint);
            }
        }
    }

    /// Evaluates every component queued at the start of this wave once,
    /// returning how many updated. A component the wave queues is left
    /// for the next wave.
    fn update_pending(&mut self) -> Result<usize, RuntimeError> {
        let pending: Vec<u64> = self
            .window
            .shared
            .pending
            .borrow()
            .keys()
            .copied()
            .collect();
        let mut count = 0;
        for serial in pending {
            if self.update_component(serial)? {
                count += 1;
            }
        }
        Ok(count)
    }

    /// Captures the actual children without forcing layout or component updates.
    fn child_snapshot(&self, serial: u64) -> Vec<ChildInfo> {
        let Some(entry) = self.entries.get(&serial) else {
            return Vec::new();
        };
        let window = entry.node.window();
        let focus_chain = self.ownership_chain(window.focused.get());
        let capture_chain = self.ownership_chain(window.capture.get());
        entry
            .children
            .iter()
            .filter_map(|serial| {
                let child = self.entries.get(serial)?;
                Some(ChildInfo {
                    detachable: child.node.detachable.borrow().clone(),
                    key: child.key.clone(),
                    label: child.label.clone(),
                    rich_label: child.rich_label.clone(),
                    id: WidgetId(child.node.clone()),
                    visible: child.node.visible.get(),
                    focus_within: focus_chain.contains(serial),
                    captures_pointer: capture_chain.contains(serial),
                })
            })
            .collect()
    }

    /// Reads the bound labels (`Element::with_label` with a binding) of the
    /// children `serials` with the parent's update context, storing the text
    /// in each child's metadata and in the context's child snapshot.
    ///
    /// Returns each changed child's previous label and rich label, which the
    /// caller restores when the parent's update fails.
    fn resolve_child_labels(
        &mut self,
        serials: &[u64],
        cx: &mut UpdateContext,
    ) -> Result<Vec<PreviousLabel>, RuntimeError> {
        let mut previous = Vec::new();
        for serial in serials {
            let Some(entry) = self.entries.get(serial) else {
                continue;
            };
            let Some(source) = entry
                .label_source
                .clone()
                .filter(crate::TextSource::is_bound)
            else {
                continue;
            };
            let leading = entry.label_leading.clone();
            let content = source.read(cx)?;
            let label = content.plain_text();
            let rich_label = match leading {
                Some(icon) => Some(content.leading_icon(icon)),
                None if content.as_plain().is_some() => None,
                None => Some(content),
            };
            if let Some(entry) = self.entries.get_mut(serial)
                && (entry.label.as_deref() != Some(label.as_str())
                    || entry.rich_label != rich_label)
            {
                previous.push((
                    *serial,
                    entry.label.replace(label.clone()),
                    std::mem::replace(&mut entry.rich_label, rich_label.clone()),
                ));
            }
            if let Some(info) = cx
                .children
                .iter_mut()
                .find(|info| info.id.0.serial == *serial)
            {
                info.label = Some(label);
                info.rich_label = rich_label;
            }
        }
        Ok(previous)
    }

    /// Runs one queued component's update, returning whether it ran: an
    /// unmounted, hidden or no longer dirty component is skipped.
    fn update_component(&mut self, serial: u64) -> Result<bool, RuntimeError> {
        #[cfg(debug_assertions)]
        let promoted = self.surface_kind(serial).is_some() || self.stacked_child(serial);
        self.window.shared.pending.borrow_mut().remove(&serial);
        let children = self.child_snapshot(serial);
        let Some(entry) = self.entries.get_mut(&serial) else {
            return Ok(false);
        };
        if !entry.node.visible.get() {
            return Ok(false);
        }
        let Some(kind) = entry.node.dirty.take() else {
            return Ok(false);
        };
        entry.node.stop_deadline();
        entry.subscriptions.clear();
        entry.node.animated.set(false);
        let was_focusable = entry.component.focusable();
        let mut context = UpdateContext {
            children,
            node: entry.node.clone(),
            kind,
            subscriptions: Vec::new(),
            deadline: None,
        };
        let mut child_serials = entry.children.clone();
        if entry.node.logical_parent().is_none() {
            // A root has no parent to read its own bound label.
            child_serials.push(serial);
        }
        // Bound child labels are read for this parent, so a change updates
        // it again and its children's metadata is current in its update.
        let resolved = self.resolve_child_labels(&child_serials, &mut context);
        let Some(entry) = self.entries.get_mut(&serial) else {
            return Ok(false);
        };
        entry.node.hover_reader.set(false);
        entry.node.focus_update_reader.set(false);
        entry.node.motion_reader.set(false);
        let guard = EvaluationGuard::new(entry.node.clone(), kind);
        let result = {
            let _timing = self.shared.profiler.span(
                crate::ProfileOperation::Update,
                Some(serial),
                &entry.key,
            );
            match &resolved {
                Ok(_) => entry.component.update(&mut context),
                Err(error) => Err(error.clone()),
            }
        };
        if result.is_err() {
            self.note_failure(serial, FailurePhase::Update);
        }
        if result.is_err()
            && let Ok(previous) = resolved
        {
            // A rejected update commits no child label it read, so the
            // children keep the metadata the parent last accepted.
            for (child, label, rich_label) in previous {
                if let Some(child) = self.entries.get_mut(&child) {
                    child.label = label;
                    child.rich_label = rich_label;
                }
            }
        }
        let Some(entry) = self.entries.get_mut(&serial) else {
            return Ok(false);
        };
        self.shared.record(
            crate::EventKind::Update,
            Some(serial),
            None,
            || match &result {
                Ok(()) => crate::EventStatus::Succeeded,
                Err(error) => crate::EventStatus::Failed(error.to_string()),
            },
        );
        entry.subscriptions = std::mem::take(&mut context.subscriptions);
        if result.is_ok() {
            entry.paint_dirty.set(true);
            #[cfg(debug_assertions)]
            {
                let previous = self.shared.phase.replace(Phase::Inspect);
                let _inspection_guard = InspectionGuard {
                    shared: self.shared.clone(),
                    previous,
                };
                let mut details = entry.component.inspect();
                if promoted {
                    details.validation.overlap = crate::OverlapPolicy::Allow;
                }
                if entry.composed {
                    details.box_model.border_box = entry.node.bounds.get().map(crate::Rect::values);
                    details.validation.clip = entry.node.clip.get().map(crate::Rect::values);
                }
                self.validator.update(
                    serial,
                    crate::validation::development::NodeData {
                        collection: (entry.component.as_ref() as &dyn std::any::Any)
                            .downcast_ref::<crate::VirtualList>()
                            .map(crate::VirtualList::extent_for_validation),
                        parent: entry.node.presentation_parent_serial(),
                        window: entry.node.window().serial,
                        key: entry.key.clone(),
                        visible: entry.node.visible.get(),
                        focused: self
                            .window
                            .focus
                            .as_ref()
                            .is_some_and(|id| id.0.serial == serial),
                        box_model: details.box_model,
                        hints: details.validation,
                    },
                );
            }
            if let Some(deadline) = context.deadline.filter(|_| entry.node.is_presented()) {
                entry.node.deadline.set(Some(deadline));
                self.window
                    .shared
                    .deadlines
                    .borrow_mut()
                    .insert((deadline, serial), Rc::downgrade(&entry.node));
            }
            entry
                .node
                .presentation_suspended
                .set(context.deadline.is_some() && !entry.node.is_presented());
            guard.succeeded.set(true);
        }
        drop(guard);
        result?;
        if was_focusable && !entry.component.focusable() {
            entry.node.lose_context();
            if !entry.node.input_cancelled.replace(true) {
                entry.component.cancel_input(&mut EventContext {
                    node: entry.node.clone(),
                });
            }
            if self
                .window
                .focus
                .as_ref()
                .is_some_and(|id| id.0.serial == serial)
            {
                entry.node.invalidate_focus_path();
                self.window.focus = None;
                self.window.shared.focused.set(None);
                self.window.shared.focus_node.borrow_mut().take();
                self.window.restore_focus = true;
                self.window.focus_fallback = Self::focus_fallback_of(&entry.node);
            }
        }
        self.refresh_child_visibility(serial);
        self.sync_view_scale(serial);
        Ok(true)
    }

    /// Applies the reparenting requests components made. On an error the
    /// unapplied requests stay queued ahead of any new ones.
    fn apply_requested_moves(&mut self) -> Result<(), RuntimeError> {
        let moves = std::mem::take(&mut *self.shared.requested_moves.borrow_mut());
        for (index, (child, parent, position)) in moves.iter().enumerate() {
            let (Some(child), Some(parent)) = (child.upgrade(), parent.upgrade()) else {
                continue;
            };
            if !child.mounted.get() || !parent.mounted.get() {
                continue;
            }
            if let Err(error) = self.reparent(&WidgetId(child), &WidgetId(parent), *position) {
                let mut queued = self.shared.requested_moves.borrow_mut();
                queued.splice(0..0, moves[index + 1..].to_vec());
                return Err(error);
            }
        }
        Ok(())
    }

    /// Reconciles the child descriptions components requested. On an error
    /// the unapplied requests stay queued ahead of any new ones.
    fn apply_requested_children(&mut self) -> Result<(), RuntimeError> {
        let requests = std::mem::take(&mut *self.shared.requested_children.borrow_mut());
        let mut requests = requests.into_iter();
        while let Some((owner, children)) = requests.next() {
            let Some(owner) = owner.upgrade() else {
                continue;
            };
            if !owner.mounted.get() {
                continue;
            }
            let mut trail = Vec::new();
            if let Err(error) =
                self.reconcile_children_traced(&WidgetId(owner.clone()), children, &mut trail)
            {
                trail.reverse();
                self.note_failure_at(owner.serial, FailurePhase::Reconcile, trail);
                let mut queued = self.shared.requested_children.borrow_mut();
                queued.splice(0..0, requests);
                return Err(error);
            }
            owner.children_current.set(true);
        }
        Ok(())
    }

    /// Delivers the text commands components requested to visible targets.
    /// On an error the undelivered commands stay queued ahead of any new ones.
    fn apply_requested_text_commands(&mut self) -> Result<(), RuntimeError> {
        let commands = std::mem::take(&mut *self.shared.requested_text_commands.borrow_mut());
        for (index, (target, command)) in commands.iter().enumerate() {
            let Some(node) = target.upgrade() else {
                continue;
            };
            if !node.mounted.get() || !node.visible.get() {
                continue;
            }
            if !self.entries.contains_key(&node.serial) {
                continue;
            }
            let result = if self.modal_allows_in_own_window(node.serial) {
                self.deliver_text_command(&node, *command)
            } else {
                Err(RuntimeError::ModalBlocked)
            };
            if let Err(error) = result {
                let mut queued = self.shared.requested_text_commands.borrow_mut();
                queued.splice(0..0, commands[index + 1..].to_vec());
                return Err(error);
            }
        }
        Ok(())
    }

    /// Unmounts the components whose removal was requested.
    fn apply_requested_removals(&mut self) -> Result<(), RuntimeError> {
        let removals = std::mem::take(&mut *self.shared.requested_removals.borrow_mut());
        for node in removals.into_values().filter_map(|node| node.upgrade()) {
            if node.mounted.get() {
                self.unmount(&WidgetId(node))?;
            }
        }
        Ok(())
    }

    /// Lays out pending geometry and resynchronizes anchored surfaces when
    /// layout ran or surface focus changed, returning whether layout was
    /// pending.
    fn settle_layout(&mut self) -> Result<bool, RuntimeError> {
        let had_layout = self.window.shared.layout_pending.get();
        self.layout()?;
        if had_layout
            || self.window.surface_error.is_some()
            || (!self.window.surfaces.is_empty()
                && self.window.surface_focus != self.window.shared.focused.get())
            || self.window.surfaces.iter().any(|surface| surface.departed)
        {
            self.sync_surfaces()?;
            self.window.surface_focus = self.window.shared.focused.get();
        }
        Ok(had_layout)
    }

    /// Clears focus, as a cancellation, from a component that was focusable
    /// when it received focus and no longer is, whatever changed it: its own
    /// update, or another component, as an owner ending its editor's session.
    fn drop_unfocusable_focus(&mut self) {
        let Some(id) = self.window.focus.clone() else {
            return;
        };
        if !self.window.focus_was_focusable {
            return;
        }
        let Some(entry) = self.entries.get_mut(&id.0.serial) else {
            return;
        };
        if entry.component.focusable() {
            return;
        }
        entry.node.lose_context();
        if !entry.node.input_cancelled.replace(true) {
            entry.component.cancel_input(&mut EventContext {
                node: entry.node.clone(),
            });
        }
        entry.node.invalidate_focus_path();
        self.window.focus = None;
        self.window.shared.focused.set(None);
        self.window.shared.focus_node.borrow_mut().take();
        self.window.restore_focus = true;
        self.window.focus_fallback = Self::focus_fallback_of(&id.0);
    }
    /// The target itself when it is focusable, otherwise the first focusable,
    /// visible, modally allowed component inside it in Tab order.
    fn first_focusable_within(&self, target: &Rc<Node>) -> Option<Rc<Node>> {
        let focusable = |serial: u64| {
            self.entries.get(&serial).is_some_and(|entry| {
                entry.node.visible.get() && self.modal_allows(serial) && entry.component.focusable()
            })
        };
        if focusable(target.serial) {
            return Some(target.clone());
        }
        self.navigation_order()
            .into_iter()
            .find(|serial| {
                *serial != target.serial
                    && self.descendant(*serial, target.serial)
                    && focusable(*serial)
            })
            .map(|serial| self.entries[&serial].node.clone())
    }
    /// Moves focus to a requested focusable target, then restores focus
    /// after the focused component was lost.
    fn apply_requested_focus(&mut self) -> Result<(), RuntimeError> {
        let request = self.window.shared.requested_focus.borrow_mut().take();
        let requested = request.as_ref().and_then(Weak::upgrade);
        let within = self.window.shared.requested_within.replace(false);
        // The one difference from dropping a hidden target's request: a
        // mounted target hidden only until an update still pending in this
        // flush shows it (an ancestor's, as the label whose text command
        // began an edit shows its editor) keeps the request for the next
        // pass of this flush, so focus reaches it once that update has run.
        // A flush that fails first leaves it pending, with that update, for
        // the next flush. The rest of this step runs as without the request,
        // and a focus restoration in it cancels the request, as any transfer
        // does, with one exception: a focused component that contains the
        // target hands focus to it, as the label does to its editor, so its
        // loss of focusability waits for that transfer instead of counting
        // as a cancellation now. A request dropped later, its target still
        // hidden, lets that loss restore focus in its own pass.
        if let Some(node) = &requested
            && node.mounted.get()
            && !node.visible.get()
            && self.ancestor_update_pending(node)
        {
            let hands_over = self
                .window
                .focus
                .as_ref()
                .is_some_and(|focus| Self::presentation_ancestor(node, focus.0.serial));
            if !hands_over {
                self.drop_unfocusable_focus();
            }
            if !self.window.restore_focus {
                *self.window.shared.requested_focus.borrow_mut() = request;
                self.window.shared.requested_within.set(within);
            }
            return self.restore_fallback_focus();
        }
        let requested = match requested {
            Some(node) if within && node.visible.get() => self.first_focusable_within(&node),
            other => other,
        };
        if let Some(node) = requested
            && node.visible.get()
            && self.modal_allows(node.serial)
            && self
                .entries
                .get(&node.serial)
                .is_some_and(|entry| entry.component.focusable())
        {
            self.focus(&WidgetId(node))?;
        }
        // After a request, which may move focus out of a component that made
        // itself non-focusable for that very reason.
        self.drop_unfocusable_focus();
        self.restore_fallback_focus()
    }
    /// Whether the component `ancestor` is a presentation ancestor of `node`.
    fn presentation_ancestor(node: &Rc<Node>, ancestor: u64) -> bool {
        let mut current = node.presentation_parent();
        while let Some(parent) = current {
            if parent.serial == ancestor {
                return true;
            }
            current = parent.presentation_parent();
        }
        false
    }
    /// Whether a presentation ancestor of `node` has an update pending in
    /// its window.
    fn ancestor_update_pending(&self, node: &Rc<Node>) -> bool {
        let pending = self.window.shared.pending.borrow();
        let mut current = node.presentation_parent();
        while let Some(parent) = current {
            if pending.contains_key(&parent.serial) {
                return true;
            }
            current = parent.presentation_parent();
        }
        false
    }
    /// Gives focus that a cancellation cleared back to the nearest ancestor
    /// that can hold it again.
    fn restore_fallback_focus(&mut self) -> Result<(), RuntimeError> {
        if self.window.restore_focus {
            self.window.restore_focus = false;
            // The nearest ancestor that can hold focus again takes it, as an
            // owner does when an embedded editor's session ends; otherwise
            // focus moves to the window's first focusable component.
            let fallback = std::mem::take(&mut self.window.focus_fallback);
            for node in fallback.iter().filter_map(Weak::upgrade) {
                if node.mounted.get()
                    && node.visible.get()
                    && self.modal_allows(node.serial)
                    && self
                        .entries
                        .get(&node.serial)
                        .is_some_and(|entry| entry.component.focusable())
                {
                    return self.focus(&WidgetId(node));
                }
            }
            self.focus_next(crate::FocusDirection::Forward)?;
        }
        Ok(())
    }
    /// Where focus goes back to after a cancellation clears it from `node`:
    /// its presentation ancestors, innermost first.
    pub(super) fn focus_fallback_of(node: &Rc<Node>) -> Vec<Weak<Node>> {
        let mut fallback = Vec::new();
        let mut current = node.presentation_parent();
        while let Some(parent) = current {
            fallback.push(Rc::downgrade(&parent));
            current = parent.presentation_parent();
        }
        fallback
    }

    /// Refreshes hover after layout or a stacking change; a failed refresh
    /// keeps the stacking change pending.
    fn refresh_stack_hover(&mut self, had_layout: bool) -> Result<(), RuntimeError> {
        let stack_changed = self.window.shared.stack_hover_pending.replace(false);
        if (had_layout || stack_changed)
            && let Err(error) = self.refresh_hover()
        {
            self.window.shared.stack_hover_pending.set(stack_changed);
            return Err(error);
        }
        Ok(())
    }
}

#[derive(Default)]
struct FlushState {
    depth: Cell<usize>,
    waves: Cell<usize>,
    exhausted: Cell<bool>,
    /// Per target, how often a refresh move left new work scheduled during
    /// the outermost flush: evidence for an `UpdateLimit` report.
    #[cfg(debug_assertions)]
    refresh_work: RefCell<BTreeMap<u64, usize>>,
    /// The component that handled the refresh move being dispatched, which
    /// may be an ancestor of its target.
    #[cfg(debug_assertions)]
    refresh_handler: Cell<Option<u64>>,
}
struct FlushGuard {
    state: Rc<FlushState>,
    outermost: bool,
}
impl FlushGuard {
    fn new(state: Rc<FlushState>) -> Self {
        let outermost = state.depth.get() == 0;
        if outermost {
            state.waves.set(0);
            state.exhausted.set(false);
            #[cfg(debug_assertions)]
            state.refresh_work.borrow_mut().clear();
        }
        state.depth.set(state.depth.get() + 1);
        Self { state, outermost }
    }
}
impl Drop for FlushGuard {
    fn drop(&mut self) {
        self.state.depth.set(self.state.depth.get() - 1);
        // Read in release too, where the development validator is absent.
        if self.outermost {
            self.state.waves.set(0);
            self.state.exhausted.set(false);
            #[cfg(debug_assertions)]
            self.state.refresh_work.borrow_mut().clear();
        }
    }
}

/// Marks an answering boundary while it runs; restored on unwinding too, so a
/// caught panic cannot leave every later flush silent.
pub(crate) struct AnswerBoundary {
    shared: Rc<Shared>,
}
impl AnswerBoundary {
    pub(crate) fn new(shared: Rc<Shared>) -> Self {
        shared.answer_depth.set(shared.answer_depth.get() + 1);
        Self { shared }
    }
}
impl Drop for AnswerBoundary {
    fn drop(&mut self) {
        self.shared
            .answer_depth
            .set(self.shared.answer_depth.get().saturating_sub(1));
    }
}

struct InspectionGuard {
    shared: Rc<Shared>,
    previous: Phase,
}
impl Drop for InspectionGuard {
    fn drop(&mut self) {
        self.shared.phase.set(self.previous);
    }
}

struct EvaluationGuard {
    node: Rc<Node>,
    kind: Invalidation,
    succeeded: Cell<bool>,
}
impl EvaluationGuard {
    fn new(node: Rc<Node>, kind: Invalidation) -> Self {
        node.shared.phase.set(Phase::Update);
        Self {
            node,
            kind,
            succeeded: Cell::new(false),
        }
    }
}
impl Drop for EvaluationGuard {
    fn drop(&mut self) {
        self.node.shared.phase.set(Phase::Idle);
        if !self.succeeded.get() {
            self.node.invalidate(self.kind);
        }
    }
}

impl Runtime {
    /// Permanently closes the runtime, canceling input and unmounting all lifetime
    /// roots before native resources are destroyed. Idempotent; subsequent runtime
    /// mutations fail with Closed. Signal observers and host services are released.
    /// Active and pending drags deliver cancellation while participants are mounted;
    /// targets are told to leave before the source receives its terminal outcome.
    /// Cleanup continues after callback failure. The first error per cancelled drag
    /// operation and other queued cleanup errors remain available through
    /// [`Self::take_shutdown_errors`]. New drags are refused during teardown.
    pub fn shutdown(&mut self) {
        if self.shared.closed.get() {
            return;
        }
        self.shared.shutting_down.set(true);
        self.retain_shutdown_errors();
        self.cancel_drag_infallibly();
        self.retain_shutdown_errors();
        self.shared.wake.close();
        self.shared.tasks.borrow_mut().clear();
        self.shared.scale_settle.set(None);
        self.shared.stretched.borrow_mut().clear();
        self.shared.computations.borrow_mut().clear();
        let mailboxes: Vec<_> = self
            .shared
            .mailboxes
            .borrow_mut()
            .drain(..)
            .filter_map(|weak| weak.upgrade())
            .collect();
        let services: Vec<_> = self
            .shared
            .inspection_services
            .borrow()
            .iter()
            .filter_map(Weak::upgrade)
            .collect();
        for service in services {
            service.close();
        }
        // Each unanswered capture cancels its waiter as it drops.
        self.shared.captures.borrow_mut().clear();
        for mailbox in mailboxes {
            mailbox.close();
        }

        self.cancel_held(crate::HeldCancellation::Shutdown);
        self.held_registrations.clear();
        let roots: Vec<_> = self
            .entries
            .iter()
            .filter(|(_, e)| e.node.logical_parent_serial().is_none())
            .map(|(id, _)| *id)
            .collect();
        for serial in roots {
            self.remove_subtree(serial);
        }
        for window in self.native_windows() {
            window.0.mounted.set(false);
        }
        self.window.shared.pending.borrow_mut().clear();
        self.shared.presentation_requests.borrow_mut().clear();
        let requests = std::mem::take(&mut *self.shared.command_requests.borrow_mut());
        for request in requests {
            request.cancel();
        }
        let reveals = std::mem::take(&mut *self.shared.reveal_requests.borrow_mut());
        for request in reveals {
            request.cancel();
        }
        self.commands.clear();
        self.command_presses.clear();
        // Parameters end their edits and deliver the terminal notifications
        // while reads still work, then release their sinks.
        self.shared.dispatch_deferred();
        let parameters: Vec<_> = self
            .parameters
            .borrow()
            .iter()
            .filter_map(Weak::upgrade)
            .collect();
        for parameter in parameters {
            parameter.shut_down();
        }
        self.retain_shutdown_errors();
        self.shared.closed.set(true);
        self.shared.font_subscribers.borrow_mut().0.clear();
        self.text.clear();
        let clipboard = self.shared.clipboard.borrow_mut().take();
        drop(clipboard);
        self.shut_down_application_services();
    }

    /// Drains typed errors retained by infallible cleanup after [`Self::shutdown`].
    ///
    /// Returns an empty list before shutdown without consuming pending errors.
    /// Within each window, queued focus/terminal-removal errors retain FIFO
    /// order, followed by its earliest coalesced surface-dismissal error.
    /// Retirement appends them after the surviving window's queued errors;
    /// shutdown drains at its retention points. There is no global chronological
    /// ordering across windows or error kinds.
    ///
    /// After shutdown this also includes earlier cleanup errors that were queued
    /// for a flush but had not yet been returned. Errors are drained from the
    /// selected window, then other windows in identity order at each retention
    /// point. Errors survive
    /// native-window retirement. A later call is empty. This performs no callbacks
    /// or flush.
    ///
    /// When several drag participants fail, drag cleanup retains its first error
    /// while still notifying the remaining participants. Read this report before
    /// dropping the runtime; dropping it releases any undrained errors.
    ///
    /// ```
    /// # use pert_ui_runtime as pert_ui;
    /// use pert_ui::*;
    /// struct Source;
    /// impl Component for Source {
    ///     fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> { Ok(()) }
    ///     fn measure(&self, _: &MeasureContext<'_>, _: Constraints) -> Result<Size, RuntimeError> {
    ///         Ok(Size::from([20, 20]))
    ///     }
    ///     fn drag_finished(&mut self, _: &mut EventContext, _: DragOutcome) -> Result<(), RuntimeError> {
    ///         Err(RuntimeError::Exhausted)
    ///     }
    /// }
    /// let mut ui = Runtime::new();
    /// ui.set_viewport(Viewport::new(100.0, 100.0, 1.0)?)?;
    /// let source = ui.mount_tree(Element::new("source", Source))?;
    /// ui.flush(ui.now())?;
    /// ui.pointer_input(PointerEvent::new(Point::new(5.0, 5.0)?,
    ///     PointerKind::Press(PointerButton::Primary)))?;
    /// ui.event(&source, |cx| cx.start_drag(DragPayload::new("item"),
    ///     DropOperations::MOVE, DragPreview::none()))??;
    /// assert!(ui.take_shutdown_errors().is_empty());
    /// ui.shutdown(); // cancels the accepted, still-pending request
    /// assert_eq!(ui.take_shutdown_errors(), [RuntimeError::Exhausted]);
    /// assert!(ui.take_shutdown_errors().is_empty());
    /// # Ok::<(), RuntimeError>(())
    /// ```
    #[must_use]
    pub fn take_shutdown_errors(&mut self) -> Vec<RuntimeError> {
        if !self.shared.closed.get() {
            return Vec::new();
        }
        std::mem::take(&mut self.shutdown_errors)
    }

    fn retain_shutdown_errors(&mut self) {
        self.shutdown_errors
            .extend(self.window.drain_cleanup_errors());
        for window in self.parked_windows.values_mut() {
            self.shutdown_errors.extend(window.drain_cleanup_errors());
        }
    }
}

impl Drop for Runtime {
    fn drop(&mut self) {
        self.shutdown();
    }
}

#[cfg(test)]
mod active_source_tests {
    use super::*;
    struct Source {
        eligible: Cell<bool>,
        checks: Cell<usize>,
        serviced: Cell<usize>,
    }
    impl crate::live::Drainable for Source {
        fn eligible(&self) -> bool {
            self.checks.set(self.checks.get() + 1);
            self.eligible.get()
        }
        fn repace(&self, _: Duration) {}
        fn service(&self, _: Duration, _: bool) {
            self.serviced.set(self.serviced.get() + 1);
        }
        fn deadline(&self, _: Duration) -> Option<Duration> {
            None
        }
        fn inspect(&self) -> crate::SourceInspection {
            panic!("scheduling must not inspect sources")
        }
    }
    #[test]
    fn ordinary_flushes_do_not_scan_dormant_source_registry() {
        let mut ui = Runtime::new();
        let sources: Vec<_> = (0..1000)
            .map(|_| {
                let source = Rc::new(Source {
                    eligible: Cell::new(false),
                    checks: Cell::new(0),
                    serviced: Cell::new(0),
                });
                ui.shared
                    .register_source(Rc::downgrade(&source) as Weak<dyn crate::live::Drainable>);
                source
            })
            .collect();
        sources[0].eligible.set(true);
        ui.shared.source_visibility_dirty.set(true);
        ui.flush(Duration::ZERO).unwrap();
        let dormant_checks = sources[1].checks.get();
        for tick in 1..61 {
            ui.flush(Duration::from_millis(tick)).unwrap();
            let _ = ui.next_deadline();
        }
        assert!(sources[0].serviced.get() > 1);
        assert!(
            sources[1..]
                .iter()
                .all(|s| s.serviced.get() == 0 && s.checks.get() == dormant_checks)
        );
    }
}

#[cfg(test)]
mod clipboard_tests {
    use super::*;

    struct Memory;
    impl crate::Clipboard for Memory {
        fn read_text(&mut self) -> Result<String, crate::ClipboardError> {
            Ok("held".to_owned())
        }
        fn write_text(&mut self, _: &str) -> Result<(), crate::ClipboardError> {
            Ok(())
        }
    }

    #[test]
    fn runtime_clipboard_text_is_refused_inside_every_phase_and_from_the_service()
    -> Result<(), RuntimeError> {
        let mut ui = Runtime::new();
        ui.set_clipboard(Memory)?;
        // Application code holds the runtime only between flushes, so these
        // phases are set directly, as a reentrant path would find them.
        for (phase, error) in [
            (Phase::Update, RuntimeError::WriteDuringUpdate),
            (Phase::Layout, RuntimeError::WriteDuringLayout),
            (Phase::Paint, RuntimeError::WriteDuringPaint),
            (Phase::Inspect, RuntimeError::WriteDuringInspection),
            (Phase::Delivery, RuntimeError::WriteDuringDelivery),
        ] {
            let previous = ui.shared.phase.replace(phase);
            assert_eq!(ui.clipboard_text(), Err(error.clone()));
            assert_eq!(ui.set_clipboard_text("x"), Err(error));
            ui.shared.phase.set(previous);
        }
        // Inside the service, as a reentrant call from it would be.
        {
            let _service = ui.shared.clipboard.borrow_mut();
            assert_eq!(ui.clipboard_text(), Err(RuntimeError::BorrowConflict));
            assert_eq!(
                ui.set_clipboard_text("x"),
                Err(RuntimeError::BorrowConflict)
            );
        }
        assert_eq!(ui.clipboard_text()?, "held");
        Ok(())
    }
}
