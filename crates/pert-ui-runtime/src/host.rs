//! Driving a [`Runtime`] in place of the facade's `App`: the
//! [`HostIntegration`] trait and the names a host writes.
//!
//! A *host* owns the event loop around a runtime: it supplies viewports,
//! translates native input, flushes, paints and presents frames, completes
//! presentation requests and serves captures. The facade's `App` is one host;
//! a custom host, and a headless test that stands in for one, are others.
//! Applications that use `App` never import this module.
//!
//! [`HostIntegration`] holds the methods only a host calls. A host imports it
//! (`use pert_ui_runtime::host::HostIntegration;`, or a glob of this module)
//! and calls them on the runtime as ordinary methods; a test reaches the trait
//! through [`testing`](crate::testing). The module also re-exports what a host
//! writes while it drives the runtime, from event translation to frame
//! production; each name stays at the crate root too.
//!
//! One turn of a host's loop, without a window system:
//!
//! ```
//! # use pert_ui_runtime as pert_ui;
//! use pert_ui::host::*;
//! use pert_ui::{Element, Label};
//! use std::time::Duration;
//!
//! let mut ui = Runtime::new();
//! ui.set_viewport(Viewport::from_physical(PhysicalExtent::new(640, 400), 2.0)?)?;
//! ui.mount_tree(Element::new("title", Label::new("Hosted")))?;
//!
//! let report: FlushReport = ui.flush_windows(Duration::ZERO)?;
//! assert!(report.components_updated > 0);
//! for request in ui.take_presentation_requests()? {
//!     // This host has no native windows to create.
//!     request.complete(PresentationStatus::Failed(WindowError::Cancelled))?;
//! }
//! if ui.needs_paint() {
//!     let frame: DisplayList = ui.paint()?;
//!     assert!(!frame.is_empty());
//! }
//! ui.shutdown();
//! assert!(ui.take_shutdown_errors().is_empty());
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! The `pert-ui` facade's `host` module re-exports this one and carries a
//! complete custom-host example; [contracts.md's custom hosts] states what a
//! host must do.
//!
//! [contracts.md's custom hosts]: https://github.com/pertyjons/pert-ui/blob/main/docs/contracts.md#custom-hosts
use std::time::Duration;

#[doc(no_inline)]
pub use crate::{
    CaptureError, CaptureJob, Clipboard, ClipboardError, CommandKey, CommandKeyInput, CursorIcon,
    DisplayList, DrawError, FlushReport, FrameImage, Invalidation, KeyAction, KeyCode, KeyDispatch,
    KeyboardState, LimitError, Magnification, Modifiers, NativeWindowId, NativeWindowOptions,
    PhysicalExtent, Point, PointerButton, PointerEvent, PointerKind, PreparedDetach,
    PresentationAction, PresentationRate, PresentationRequest, PresentationStatus, Runtime,
    RuntimeError, TextInputState, TextPreedit, Viewport, WidgetId, WindowError,
};

/// The methods a host calls to drive a [`Runtime`] in place of the facade's
/// `App`.
///
/// A host is code that owns the event loop around a runtime: the facade's
/// `App`, a custom host built on another window system or none, and a
/// headless test that stands in for one, for example to select a native
/// window or complete a presentation request. Applications that run under
/// `App` never call these methods, so they are kept off [`Runtime`]'s own
/// page and out of the prelude.
///
/// Import the trait to call its methods on a runtime:
/// `use pert_ui::host::HostIntegration;` with the `pert-ui` facade (or
/// `pert_ui_runtime::host` with this crate alone), or
/// `use pert_ui::testing::*;` in a test, which re-exports it. Without the
/// import, a call does not compile and rustc's help names the trait.
///
/// Only [`Runtime`] implements the trait. It is sealed, so it can gain
/// methods without breaking hosts, and it is not usable as a trait object:
/// [`set_wake_delivery`](Self::set_wake_delivery) and
/// [`set_clipboard`](Self::set_clipboard) take `impl` arguments. Every method
/// forwards to the runtime with its documented behavior; the trait adds no
/// state and no privileged path, and `App` calls it through the same import.
///
/// ```compile_fail
/// # use pert_ui_runtime as pert_ui;
/// use pert_ui::host::HostIntegration;
///
/// struct MyRuntime;
/// impl HostIntegration for MyRuntime {} // sealed: only `Runtime` implements it
/// ```
///
/// A host also calls inherent [`Runtime`] methods that tests and applications
/// share: [`set_viewport`](Runtime::set_viewport) for the window's size,
/// [`pointer_input`](Runtime::pointer_input),
/// [`command_key_input`](Runtime::command_key_input),
/// [`native_modifiers`](Runtime::native_modifiers),
/// [`text_input`](Runtime::text_input) and [`keyboard`](Runtime::keyboard)
/// for input, [`flush`](Runtime::flush) and [`paint`](Runtime::paint) for a
/// frame, [`set_capture_support`](Runtime::set_capture_support) before it
/// serves captures, and [`shutdown`](Runtime::shutdown).
///
/// The methods by duty:
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
///
/// [contracts.md's custom hosts] states the obligations a host takes on, such
/// as draining [`take_shutdown_errors`](Self::take_shutdown_errors) before it
/// drops the runtime.
///
/// [contracts.md's custom hosts]: https://github.com/pertyjons/pert-ui/blob/main/docs/contracts.md#custom-hosts
pub trait HostIntegration: sealed::HostIntegrationSeal {
    // Event loop
    /// Flushes eligible roots at a shared time, retaining the selected root.
    /// Only each root's pending components run; static roots do not rebuild.
    /// Returns the total update count and earliest application deadline.
    /// When every root is hidden, deferred tasks are still delivered once, so a
    /// task cannot keep [`Self::application_deadline`] due without being run.
    ///
    /// # Errors
    /// Returns the first error of [`Runtime::flush`] for a visible root, or of a
    /// task delivered while every root is hidden; the remaining roots
    /// are not flushed and the previously selected root is selected again. `Unmounted` when
    /// a root cannot be selected.
    fn flush_windows(&mut self, now: Duration) -> Result<FlushReport, RuntimeError>;
    /// Processes one queued menu/button batch independently of presentation
    /// availability. Native hosts call this at an event boundary even when every
    /// root is hidden. Each receipt completes with revalidated admission; commands
    /// targeting unavailable roots fail instead of waiting for an unrelated redraw.
    /// Reentrant requests remain scheduled for a subsequent batch. Ordinary App
    /// consumers do not need to call this; normal flushes also process commands.
    ///
    /// Every receipt of the batch reaches a terminal status before this returns,
    /// including when an action shuts the runtime down: the invocations after it
    /// finish as [`CommandStatus::Cancelled`](crate::CommandStatus::Cancelled).
    ///
    /// # Errors
    /// An action's own error is retained in its receipt, not returned. The
    /// runtime phase/shutdown error is returned before the batch starts;
    /// publishing a receipt's status cannot fail.
    fn flush_commands(&mut self) -> Result<(), RuntimeError>;
    /// Whether command admission is pending independently of visible render work.
    fn has_command_requests(&self) -> bool;
    /// Earliest eligible work across all roots, independent of selected/focused
    /// window. Known hidden roots are excluded. Native hosts additionally account
    /// for OS redraw availability and recovery deadlines; reading never polls.
    fn application_deadline(&self) -> Option<Duration>;
    /// The earliest due time of a running interval timer, for a host that
    /// schedules its own wakeups; [`Runtime::next_deadline`] and
    /// [`Self::application_deadline`] include it.
    #[must_use]
    fn interval_deadline(&self) -> Option<Duration>;
    /// Runs the interval timers due at `now` outside a flush, for a host that
    /// cannot flush a window, such as one whose redraw the window system
    /// withholds. An ordinary flush runs due timers itself. Advances the
    /// runtime's time to `now`.
    ///
    /// # Errors
    /// `TimeReversed` for a time before the runtime's current one, `Closed`
    /// after shutdown and the phase errors of a write from inside update,
    /// layout, paint, inspection or delivery, then the first error a tick
    /// returns; later due timers run in the next call or flush.
    fn service_intervals(&mut self, now: Duration) -> Result<(), RuntimeError>;
    /// Answers every queued inspection request now, from the state the last
    /// flush left, without flushing.
    ///
    /// For a host that withholds flushes: the native `App` does while a
    /// window's redraw is pending or its surface is unavailable, since a
    /// compositor may hold a minimized window's redraw indefinitely. Calling
    /// this there keeps a request from being stranded; the reply carries no
    /// flush error. An ordinary host never needs it, because every outermost
    /// [`flush`](Runtime::flush) and [`flush_windows`](Self::flush_windows)
    /// answers on its own. Answering during a flush, an update, a paint or an
    /// inspection is refused.
    ///
    /// # Errors
    /// Returns a runtime phase error (see [`Runtime`]), or [`RuntimeError::BorrowConflict`]
    /// when called during a flush or while inspection requests are being
    /// answered.
    fn service_inspection(&mut self) -> Result<(), RuntimeError>;
    /// Installs how a [`LiveWaker`](crate::LiveWaker) wakes this host's event
    /// loop; the callback runs on the notifying thread and returns whether the
    /// wake was delivered. The default native `App` installs its own; a custom
    /// host installs one that reaches its loop. Replacing it reopens a waker
    /// that reported closed because delivery failed. After
    /// [`Runtime::shutdown`] the call is ignored and every waker keeps reporting
    /// [`WakeError::Closed`](crate::WakeError::Closed), since no flush will
    /// drain what a wake announces. A callback that panics unwinds out of
    /// that `notify`, leaving the wake pending for the next flush; later
    /// notifications still reach the callback.
    fn set_wake_delivery(&self, deliver: impl Fn() -> bool + Send + Sync + 'static);
    // Windows and presentation
    /// Routes subsequent viewport/paint/input operations to a mounted window.
    ///
    /// Does not call component focus callbacks or request OS activation. Native
    /// hosts select the event's root before dispatch. Applications normally use
    /// the default App host.
    ///
    /// # Errors
    /// Returns [`RuntimeError::Closed`] after shutdown, and
    /// `WriteDuringUpdate`, `WriteDuringLayout`, `WriteDuringPaint`,
    /// `WriteDuringInspection` or `WriteDuringDelivery` when called from inside that phase;
    /// [`RuntimeError::ForeignRuntime`] for a window
    /// identity of another runtime; and `Unmounted` for one of this runtime that is retired
    /// or no longer mounted.
    fn select_window(&mut self, window: &NativeWindowId) -> Result<(), RuntimeError>;
    /// Presentation root used by viewport, painting and native input operations.
    /// Selecting a root is routing, not a focus transition or native activation.
    fn current_window(&self) -> NativeWindowId;
    /// Supplies known native presentation visibility for the selected root.
    ///
    /// Hidden/zero-sized roots stop deadlines and presentation work; showing
    /// refreshes retained state. Unknown native visibility must be left unchanged,
    /// and lack of keyboard focus must never be treated as hidden.
    ///
    /// Hiding ends input like a native focus loss: focus, held actions,
    /// gestures and nonmodal surfaces such as popovers and menus end. It only
    /// suspends an open modal `Dialog`, which stays open with its outcome
    /// pending, keeps an application-modal session blocking other roots, and is
    /// admitted again, with safe focus, when the root is shown. Hiding a view
    /// with [`Runtime::set_visible`] instead dismisses its dialogs as
    /// `SurfaceDismissal::OwnerHidden`.
    ///
    /// # Errors
    /// Returns [`RuntimeError::Closed`] after shutdown, and
    /// `WriteDuringUpdate`, `WriteDuringLayout`, `WriteDuringPaint`,
    /// `WriteDuringInspection` or `WriteDuringDelivery` when called from inside that phase.
    fn set_window_visible(&mut self, visible: bool) -> Result<(), RuntimeError>;
    /// Supplies the display's presentation ceiling for the selected window.
    ///
    /// `None` records that the rate is unknown and uses the deterministic 60 Hz
    /// fallback. The native `App` supplies this automatically on native event
    /// turns. Custom hosts may supply an intentional cap instead. Animation and
    /// polled live sources share this ceiling; notified sources drain on wakes.
    /// A polled source shared across windows follows its fastest presented subscriber. One-shot event timers are unaffected.
    ///
    /// # Errors
    /// Returns only a runtime phase error (see [`Runtime`]).
    fn set_presentation_rate(&self, rate: Option<PresentationRate>) -> Result<(), RuntimeError>;
    /// The supplied ceiling for this window, absent when unknown. Unlike the
    /// scheduling fallback, this query never invents a measured display rate.
    fn presentation_rate(&self) -> Option<PresentationRate>;
    /// Whether visible drawing or the scene background needs fresh composition.
    ///
    /// Custom hosts check this after events/flushes, then call `paint` after pending
    /// updates are flushed. Successful painting clears scene damage. Hidden component
    /// dirtiness alone does not request presentation. A capture waiting for this
    /// window keeps it true until the capture is served. A queued command does not
    /// make it true, although [`Runtime::paint`] refuses with
    /// [`DrawError::PendingUpdates`] until a flush dispatches the command.
    /// This query creates no timers.
    ///
    /// A test that checks idleness calls
    /// [`Driver::quiet_for`](crate::testing::Driver::quiet_for), which paints a
    /// baseline first.
    #[must_use]
    fn needs_paint(&self) -> bool;
    /// Invalidates a mount for external geometry/surface events.
    ///
    /// Normal bindings invalidate automatically; this is a platform/component
    /// extension for events such as resizing. Hidden mounts only retain dirtiness.
    ///
    /// # Errors
    /// Returns [`RuntimeError::ForeignRuntime`] for an ID of another runtime and
    /// [`RuntimeError::Unmounted`] for an unmounted one.
    fn invalidate(&self, id: &WidgetId, kind: Invalidation) -> Result<(), RuntimeError>;
    /// Whether the native host has presentation operations to process.
    fn has_presentation_requests(&self) -> bool;
    /// Takes native work once, preserving queued order. Only native hosts need
    /// this API; ordinary applications use App. Dropping requests cancels receipts.
    ///
    /// The host owns each request until it completes it with
    /// [`PresentationRequest::complete`] and the actual
    /// [`PresentationStatus`]; dropping an unfinished request completes it as
    /// `Failed(WindowError::Cancelled)`. A host other than the facade's `App`
    /// completes a close request as [`Runtime::request_close`] describes.
    ///
    /// # Errors
    /// Returns [`RuntimeError::Closed`] after shutdown, and
    /// `WriteDuringUpdate`, `WriteDuringLayout`, `WriteDuringPaint`,
    /// `WriteDuringInspection` or `WriteDuringDelivery` when called from inside that phase.
    fn take_presentation_requests(&self) -> Result<Vec<PresentationRequest>, RuntimeError>;
    /// Reserves a native destination without changing mounted state or input.
    ///
    /// Native creation failure can simply drop the token.
    ///
    /// ```
    /// # use pert_ui_runtime as pert_ui;
    /// use pert_ui::host::HostIntegration;
    /// use pert_ui::{Runtime, Element, Label, NativeWindowOptions};
    /// let mut ui = Runtime::new();
    /// let view = ui.mount_tree(Element::new("editor", Label::new("Retained")))?;
    /// let original = ui.window_of(&view)?;
    /// let prepared = ui.prepare_detach(&view, NativeWindowOptions::new("Editor")?)?;
    /// // A native host prepares its OS window and GPU surface here. Headless
    /// // tests can commit directly to exercise retained identity and cleanup.
    /// let detached = ui.commit_detach(prepared)?;
    /// assert_eq!(ui.window_of(&view)?, detached);
    /// ui.redock(&detached)?;
    /// assert_eq!(ui.window_of(&view)?, original);
    /// # Ok::<(), pert_ui::RuntimeError>(())
    /// ```
    ///
    /// # Errors
    /// Returns [`RuntimeError::Closed`] after shutdown, and
    /// `WriteDuringUpdate`, `WriteDuringLayout`, `WriteDuringPaint`,
    /// `WriteDuringInspection` or `WriteDuringDelivery` when called from inside that phase;
    /// `ForeignRuntime` or `Unmounted` for a foreign or unmounted view; `Hidden` for a
    /// hidden view; `NotDetachable` when its description forbids its own window;
    /// `ModalBlocked` when the subtree holds an open modal dialog; `AlreadyDetached` for a
    /// view that already has its own window; and `Exhausted` when window identities run
    /// out.
    fn prepare_detach(
        &mut self,
        view: &WidgetId,
        options: NativeWindowOptions,
    ) -> Result<PreparedDetach, RuntimeError>;
    /// Commits movement after the native host successfully prepares all resources.
    ///
    /// The same view incarnation and subscriptions move; no mount/unmount callback
    /// runs. Cleanup failure leaves presentation at its source. Completed numeric
    /// draft saves and input cancellation are not rolled back. A stale token fails
    /// without recreating an owner. Returns the new root and selects it for routing;
    /// on failure the root selected before the call stays selected.
    ///
    /// Initial geometry is validated against current UI zoom before cleanup.
    /// If cleanup callbacks change zoom, geometry is validated again before
    /// ownership moves; a rejection then leaves completed input cleanup in effect.
    ///
    /// Cleanup is confined to the moved subtree: its focused component loses
    /// focus through ordinary `focus_lost` policy, and its held keys, pointer
    /// capture, gestures and owned nonmodal surfaces end. Held keys, modifiers,
    /// pointer presses and surfaces owned elsewhere in the source window continue.
    ///
    /// # Errors
    /// Returns [`RuntimeError::Closed`] after shutdown, and
    /// `WriteDuringUpdate`, `WriteDuringLayout`, `WriteDuringPaint`,
    /// `WriteDuringInspection` or `WriteDuringDelivery` when called from inside that phase;
    /// `ForeignRuntime` or `Unmounted` for a foreign or unmounted view; `Hidden` for a
    /// hidden view; `NotDetachable` when its description forbids its own window;
    /// `ModalBlocked` when the subtree holds an open modal dialog (all checked again); and
    /// `StalePresentation` when the view moved or was detached since the token was
    /// prepared. It forwards the errors of applying the new window's size as its viewport.
    /// When the focused component inside the view saves on focus loss, an error
    /// from its [`Component::focus_lost`](crate::Component::focus_lost) or from an ancestor's
    /// [`Component::focus_within_changed`](crate::Component::focus_within_changed) is returned
    /// and the view stays where it was; further such errors wait for the next flush.
    fn commit_detach(&mut self, prepared: PreparedDetach) -> Result<NativeWindowId, RuntimeError>;
    /// Returns a detached window's view to its retained original owner.
    ///
    /// Revalidates the owner and modality before cleanup. Failure leaves the
    /// detached root available and the root selected before the call selected.
    /// On success retires its identity and selects the owner's window. Native
    /// hosts destroy the old resources after success. Input cleanup is confined
    /// to the moved view, as for [`Self::commit_detach`].
    ///
    /// # Errors
    /// Returns [`RuntimeError::Closed`] after shutdown, and
    /// `WriteDuringUpdate`, `WriteDuringLayout`, `WriteDuringPaint`,
    /// `WriteDuringInspection` or `WriteDuringDelivery` when called from inside that phase;
    /// `ForeignRuntime` for a window identity of another runtime; `Unmounted` for one
    /// that is retired or unmounted; `NotDetached` for a window that is not a detached view; `Hidden` when the view, its
    /// owner or the owner's window is hidden; `ModalBlocked` when the subtree holds an open
    /// modal dialog; and `ForeignRuntime` or `Unmounted` when the view or its owner is no
    /// longer mounted. A focus-loss error from the focused component or an ancestor is
    /// returned as for [`Self::commit_detach`], and the view stays detached.
    fn redock(&mut self, window: &NativeWindowId) -> Result<(), RuntimeError>;
    /// Applies accepted close policy to the selected detached window. Main-window
    /// shutdown belongs to the native host. Failed re-dock leaves content available.
    ///
    /// # Errors
    /// Returns `NotDetached` when the selected window is the main window. The `Redock`
    /// policy forwards the errors of [`Self::redock`] and the `RemoveView` policy those of
    /// [`Runtime::unmount`].
    fn close_detached_window(&mut self) -> Result<PresentationStatus, RuntimeError>;
    /// Options of the selected detached window; the main window uses App defaults.
    fn native_window_options(&self) -> Option<&NativeWindowOptions>;
    // Native input
    /// Dispatches temporary IME composition to the current focused component.
    /// Flushes pending binding changes first. No focus ignores
    /// the event. Adapters must discard native events from a previous focus owner.
    ///
    /// # Errors
    /// Returns [`RuntimeError::Closed`] after shutdown and the phase errors of any write,
    /// forwards the errors of the [`Runtime::flush`] that runs first, then the focused
    /// component's [`Component::text_preedit`](crate::Component::text_preedit) error, and then the errors of the flush
    /// that delivers a notification the composition requested, including a
    /// [`Component::deliver_notifications`](crate::Component::deliver_notifications) error.
    fn text_preedit(&mut self, preedit: &TextPreedit) -> Result<(), RuntimeError>;
    /// Returns the focused text recipient and its clipped window-logical IME
    /// anchor. Absent for non-editable, hidden or fully clipped components.
    fn text_input_state(&self) -> Option<(WidgetId, TextInputState)>;
    /// Resolves cursor appearance at the latest pointer position, respecting capture,
    /// clipping and ancestor fallback. Uses current layout without scheduling work;
    /// custom hosts call this after flush. Returns Default outside the window.
    /// Component cursor callbacks run under the read-only inspection restriction.
    fn cursor_icon(&self) -> CursorIcon;
    /// Handles window focus loss: clears all held keys and cancels focused edits.
    ///
    /// Calls `Component::focus_lost` for the focused recipient; local editors may
    /// save valid drafts. Errors are queued for `flush`, without preventing input
    /// cleanup. Other captured input and ancestor input handlers are canceled once
    /// per component, since key/pointer events may bubble. Late OS up notifications become
    /// `UnmatchedRelease`, preventing stuck-key actions.
    ///
    /// A headless test calls
    /// [`Driver::clear_focus`](crate::testing::Driver::clear_focus), which also
    /// settles.
    fn clear_focus(&mut self);
    /// Cancels active input after a host/component failure without transferring focus.
    ///
    /// Clears physical keys/buttons, requested focus and pointer capture, and calls
    /// [`Component::cancel_input`](crate::Component::cancel_input) on the focused/captured recipients and their
    /// ancestor paths, once per component (events may bubble). Authoritative
    /// gestures cancel; local numeric drafts remain while their field retains focus.
    /// Does not call `focus_lost`, save drafts or replay the failed event. Custom
    /// components must release held actions without activating application commands.
    /// Pending updates and diagnostic errors remain available for repair and retry.
    ///
    /// ```
    /// # use pert_ui_runtime as pert_ui;
    /// use pert_ui::host::HostIntegration;
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
    fn cancel_input(&mut self) -> Result<(), RuntimeError>;
    // Captures
    /// Whether a host has declared that it renders capture jobs.
    #[must_use]
    fn captures_supported(&self) -> bool;
    /// The captures waiting for a host to render their windows, oldest first.
    ///
    /// A host renders each job's window with the scene
    /// [`capture_scene`](Self::capture_scene) composes and completes it with
    /// [`complete_capture`](Self::complete_capture). A job stays listed until
    /// completed; a window whose surface cannot render now is completed with
    /// [`CaptureError::SurfaceUnavailable`]
    /// rather than left. [`needs_paint`](Self::needs_paint) is true while a
    /// job targets the selected window.
    #[must_use]
    fn pending_captures(&self) -> Vec<CaptureJob>;
    /// Composes the scene a host renders for `job`: the job's window, with
    /// overlays only when the job asks for them. Selects that window, as
    /// [`select_window`](Self::select_window) does; a host presenting another
    /// window reselects it afterwards. Requires flushed updates, like
    /// [`paint`](Runtime::paint).
    ///
    /// The job is looked up among this runtime's pending captures by its
    /// private request identity, so a stale job or one another runtime listed
    /// is refused even when its id matches; the pending request decides what
    /// is rendered.
    ///
    /// # Errors
    /// [`CaptureError::UnknownTarget`] for
    /// a job that is not pending here, because it was completed or another
    /// runtime listed it, and for a job whose window is gone; `paint`'s
    /// errors, such as pending updates, as
    /// [`CaptureError::Drawing`].
    /// Runtime causes from selecting or painting the window are retained as
    /// [`CaptureError::Runtime`]. The host
    /// completes the job with the owned error it got.
    fn capture_scene(&mut self, job: &CaptureJob) -> Result<DisplayList, CaptureError>;
    /// Hands a rendered frame, or the error that prevented one, to the
    /// capture `job` names, answering its request. Returns whether the job
    /// was pending here. The job is identified by the request it was listed
    /// for, as [`capture_scene`](Self::capture_scene) identifies it, so a job
    /// already completed, or one another runtime listed even with the same id,
    /// completes nothing and returns `false`.
    ///
    /// The frame is the job's window at its physical extent; the runtime
    /// crops it to the target's visible region as the current state has it,
    /// which is the state the frame shows when the host rendered it without
    /// flushing in between. A target that was unmounted, hidden or unmeasured
    /// since admission is reported, never replaced.
    fn complete_capture(&self, job: &CaptureJob, frame: Result<FrameImage, CaptureError>) -> bool;
    // Services and shutdown
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
    fn set_clipboard(&mut self, clipboard: impl Clipboard + 'static) -> Result<(), RuntimeError>;
    /// Drains typed errors retained by infallible cleanup after [`Runtime::shutdown`].
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
    /// use pert_ui::host::HostIntegration;
    /// use pert_ui::{component::*, prelude::*};
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
    fn take_shutdown_errors(&mut self) -> Vec<RuntimeError>;
}

impl HostIntegration for Runtime {
    fn flush_windows(&mut self, now: Duration) -> Result<FlushReport, RuntimeError> {
        Runtime::flush_windows(self, now)
    }
    fn flush_commands(&mut self) -> Result<(), RuntimeError> {
        Runtime::flush_commands(self)
    }
    fn has_command_requests(&self) -> bool {
        Runtime::has_command_requests(self)
    }
    fn application_deadline(&self) -> Option<Duration> {
        Runtime::application_deadline(self)
    }
    fn interval_deadline(&self) -> Option<Duration> {
        Runtime::interval_deadline(self)
    }
    fn service_intervals(&mut self, now: Duration) -> Result<(), RuntimeError> {
        Runtime::service_intervals(self, now)
    }
    fn service_inspection(&mut self) -> Result<(), RuntimeError> {
        Runtime::service_inspection(self)
    }
    fn set_wake_delivery(&self, deliver: impl Fn() -> bool + Send + Sync + 'static) {
        Runtime::set_wake_delivery(self, deliver)
    }
    fn select_window(&mut self, window: &NativeWindowId) -> Result<(), RuntimeError> {
        Runtime::select_window(self, window)
    }
    fn current_window(&self) -> NativeWindowId {
        Runtime::current_window(self)
    }
    fn set_window_visible(&mut self, visible: bool) -> Result<(), RuntimeError> {
        Runtime::set_window_visible(self, visible)
    }
    fn set_presentation_rate(&self, rate: Option<PresentationRate>) -> Result<(), RuntimeError> {
        Runtime::set_presentation_rate(self, rate)
    }
    fn presentation_rate(&self) -> Option<PresentationRate> {
        Runtime::presentation_rate(self)
    }
    fn needs_paint(&self) -> bool {
        Runtime::needs_paint(self)
    }
    fn invalidate(&self, id: &WidgetId, kind: Invalidation) -> Result<(), RuntimeError> {
        Runtime::invalidate(self, id, kind)
    }
    fn has_presentation_requests(&self) -> bool {
        Runtime::has_presentation_requests(self)
    }
    fn take_presentation_requests(&self) -> Result<Vec<PresentationRequest>, RuntimeError> {
        Runtime::take_presentation_requests(self)
    }
    fn prepare_detach(
        &mut self,
        view: &WidgetId,
        options: NativeWindowOptions,
    ) -> Result<PreparedDetach, RuntimeError> {
        Runtime::prepare_detach(self, view, options)
    }
    fn commit_detach(&mut self, prepared: PreparedDetach) -> Result<NativeWindowId, RuntimeError> {
        Runtime::commit_detach(self, prepared)
    }
    fn redock(&mut self, window: &NativeWindowId) -> Result<(), RuntimeError> {
        Runtime::redock(self, window)
    }
    fn close_detached_window(&mut self) -> Result<PresentationStatus, RuntimeError> {
        Runtime::close_detached_window(self)
    }
    fn native_window_options(&self) -> Option<&NativeWindowOptions> {
        Runtime::native_window_options(self)
    }
    fn text_preedit(&mut self, preedit: &TextPreedit) -> Result<(), RuntimeError> {
        Runtime::text_preedit(self, preedit)
    }
    fn text_input_state(&self) -> Option<(WidgetId, TextInputState)> {
        Runtime::text_input_state(self)
    }
    fn cursor_icon(&self) -> CursorIcon {
        Runtime::cursor_icon(self)
    }
    fn clear_focus(&mut self) {
        Runtime::clear_focus(self)
    }
    fn cancel_input(&mut self) -> Result<(), RuntimeError> {
        Runtime::cancel_input(self)
    }
    fn captures_supported(&self) -> bool {
        Runtime::captures_supported(self)
    }
    fn pending_captures(&self) -> Vec<CaptureJob> {
        Runtime::pending_captures(self)
    }
    fn capture_scene(&mut self, job: &CaptureJob) -> Result<DisplayList, CaptureError> {
        Runtime::capture_scene(self, job)
    }
    fn complete_capture(&self, job: &CaptureJob, frame: Result<FrameImage, CaptureError>) -> bool {
        Runtime::complete_capture(self, job, frame)
    }
    fn set_clipboard(&mut self, clipboard: impl Clipboard + 'static) -> Result<(), RuntimeError> {
        Runtime::set_clipboard(self, clipboard)
    }
    fn take_shutdown_errors(&mut self) -> Vec<RuntimeError> {
        Runtime::take_shutdown_errors(self)
    }
}

pub(crate) mod sealed {
    /// Seals [`HostIntegration`](super::HostIntegration). The distinct name
    /// keeps rustc from pointing at a dependency's own `Sealed` trait when a
    /// host tries to implement `HostIntegration`, and the message below
    /// replaces the private path in that error.
    #[diagnostic::on_unimplemented(
        message = "`HostIntegration` is sealed and cannot be implemented for `{Self}`",
        label = "not a pert-ui runtime",
        note = "only `Runtime` implements `HostIntegration`; a host imports the trait (`use pert_ui::host::HostIntegration;`) to call its methods on a `Runtime`"
    )]
    pub trait HostIntegrationSeal {}
    impl HostIntegrationSeal for crate::Runtime {}
}
