//! Driving an application in a headless test, as a user would.
//!
//! [`Driver`] clicks a component or an item a component draws itself, such
//! as a table's column title, replaces or types text, presses keys and
//! chords, clears focus, settles the runtime after each step, checks that a
//! window stays quiet over an interval ([`QuietReport`]), finds a
//! component's [`Semantics`] and lists the components one address segment
//! below a parent in order ([`AddressChild`]); [`TestError`] names what a
//! step could not do.
//! Neither needs a window, a GPU or an operating-system service. The module
//! also re-exports what a headless test writes besides the driver and the
//! prelude, so one glob covers it:
//!
//! ```
//! # use pert_ui_runtime as pert_ui;
//! use pert_ui::prelude::*;
//! use pert_ui::testing::*;
//! ```
//!
//! - input by hand and the viewport: [`Viewport`], which every harness sets
//!   with [`Runtime::set_viewport`]; [`KeyAction`] and [`FocusDirection`], for
//!   key input and focus movement through [`Runtime::key_input`] and
//!   [`Runtime::focus_next`]; [`PointerEvent`], [`PointerKind`] with the
//!   [`Magnification`] of a pinch, and [`PointerButton`] for
//!   [`Runtime::pointer_input`]; [`CommandKeyInput`] for
//!   [`Runtime::command_key_input`]; the [`InputOutcome`] that pointer and key
//!   input return; and [`ClickCounter`], whose interval separates single
//!   clicks;
//! - what the driver's steps return: [`FlushReport`], which
//!   [`Driver::settle`] and [`Driver::advance`] return, the
//!   [`RevealRefusal`] that [`TestError::NotRevealed`] carries, the
//!   [`DrawError`] that [`TestError::Draw`] carries, and each
//!   [`AddressChild`] that [`Driver::address_children`] lists;
//! - semantics: [`Semantics`], which [`Driver::semantics`] returns, with
//!   [`SemanticRole`], [`SemanticValue`], [`SemanticAction`] and
//!   [`SemanticItem`];
//! - queries: [`InspectionQuery`], [`Selector`], [`Detail`] and [`Depth`] for
//!   [`Runtime::query`], and the [`QueryResult`] it returns, with its
//!   [`ComponentInspection`]s, their [`ComponentDetails`] and the
//!   [`InspectValue`] properties those hold;
//! - development validation: [`ValidationReport`] from
//!   [`Runtime::validation_report`], with its [`ValidationIssue`]s, their
//!   [`ValidationCode`] and its [`ValidationMarker`]s;
//! - captures: [`CaptureRequest`] and [`CaptureTarget`], with the
//!   [`NativeWindowId`] and [`Rect`] a target names, for
//!   [`Runtime::request_capture`], and the [`Pending`], [`InspectionReply`],
//!   [`WaitError`], [`Capture`], [`CaptureError`] and [`FrameImage`] it
//!   answers with;
//! - [`HostIntegration`], whose methods a test calls where it stands in for
//!   a host, such as selecting a native window or taking presentation
//!   requests. A test clears focus with [`Driver::clear_focus`] and checks
//!   idleness with [`Driver::quiet_for`] beside the driver's other actions.
//!
//! The key names a chord writes ([`KeyCode`], [`Modifiers`], [`CommandKey`])
//! and [`Point`] are in the prelude.
//!
//! # Membership
//!
//! `testing` holds the driver and its error and everything else a headless
//! test writes besides the prelude: the viewport, input by hand, the query,
//! validation and capture names, and every type those names hand a test. It
//! is closed: each type that a member's constructors, methods and associated
//! constants take or return, and each type a member's public fields and
//! variants hold, is in `testing` or in the prelude, and the same holds for
//! every name this adds. An error nested inside another error is reached
//! through `?` and `Display` and stays where it is. The exceptions below are
//! records a test reads by field or passes on, and each is named with its
//! module.
//!
//! # Where the other names are
//!
//! - The window, event, parameter and live-source answers of a query
//!   ([`WindowInspection`](crate::WindowInspection),
//!   [`EventHistory`](crate::EventHistory),
//!   [`ParameterInspection`](crate::ParameterInspection),
//!   [`SourceInspection`](crate::SourceInspection)) are records for an
//!   application's own tooling, in [`inspection`](crate::inspection).
//! - What a component reports about its geometry, invalidation and
//!   validation hints ([`BoxModelInspection`](crate::BoxModelInspection),
//!   [`Invalidation`](crate::Invalidation),
//!   [`ValidationHints`](crate::ValidationHints)) is read by field and is in
//!   [`component`](crate::component) and [`inspection`](crate::inspection).
//! - Key routing and key records a test's key input ignores:
//!   [`KeyDispatch`](crate::KeyDispatch), which
//!   [`Runtime::command_key_input`] returns, in [`host`](crate::host);
//!   [`KeyEvent`](crate::KeyEvent), which [`Runtime::key_input`] returns, in
//!   [`component`](crate::component) and [`inspection`](crate::inspection);
//!   and [`KeyTransition`](crate::KeyTransition) in
//!   [`component`](crate::component).
//! - A host's [`PhysicalExtent`](crate::PhysicalExtent), for
//!   [`Viewport::from_physical`], is in [`host`](crate::host); a test builds
//!   its viewport with [`Viewport::new`].
//! - The [`DisplayList`](crate::DisplayList) that [`Runtime::paint`] returns
//!   is passed on to a renderer, and is in [`component`](crate::component)
//!   and [`host`](crate::host). The identities of its resources,
//!   [`VectorId`](crate::VectorId), [`MaskId`](crate::MaskId) and
//!   [`ImageId`](crate::ImageId), which [`DrawError::MissingVector`],
//!   [`DrawError::MissingMask`] and [`DrawError::MissingImage`] hold, are in
//!   [`component`](crate::component).
//! - [`GeometryError`](crate::GeometryError), which [`Viewport::new`] and the
//!   other value constructors return, is propagated with `?` and stays at the
//!   crate root with the other errors; so does an error nested in another,
//!   such as the [`PathError`](crate::PathError) of [`DrawError::Path`].
//! - What the [`HostIntegration`] methods take and return, such as
//!   presentation requests, capture jobs and the clipboard service, is the
//!   [`host`](crate::host) module's vocabulary.
//!
//! These names stay at the crate root too, except [`HostIntegration`], which
//! is reached through [`host`](crate::host) and this module; the module only
//! gathers them. The `pert-ui` facade's `testing` module re-exports this one
//! and adds its CPU renderer with the renderer's error and its headless
//! application entry.
//!
//! The driver is a test operation: [`Driver::wait_until`] polls with short
//! sleeps for background work. The runtime's hosts and its inspection never
//! poll.
//!
//! ```
//! # use pert_ui_runtime as pert_ui;
//! use pert_ui::prelude::*;
//! use pert_ui::testing::*;
//!
//! /// The semantics of the component at `path` under `root`.
//! fn semantics_of(
//!     driver: &Driver<'_>,
//!     root: &WidgetId,
//!     path: &[&str],
//! ) -> Result<Semantics, TestError> {
//!     driver.semantics(&driver.find(root, path)?)
//! }
//!
//! let mut ui = Runtime::new();
//! ui.set_viewport(Viewport::new(400.0, 200.0, 1.0)?)?;
//! let name = ui.signal(String::from("Draft"));
//! let saves = ui.signal(0);
//! let counter = saves.clone();
//! let root = ui.mount_tree(Element::new("form", Column::new([
//!     Element::new("name", TextField::new(name.clone())),
//!     Element::new("save", Button::new("Save", move |_| counter.update(|n| *n += 1))),
//! ])))?;
//!
//! let mut driver = Driver::new(&mut ui);
//! driver.settle()?;
//! let field = driver.find(&root, &["name"])?;
//! driver.replace_text(&field, "Report")?;
//! driver.click(&driver.find(&root, &["save"])?)?;
//! assert_eq!(name.get()?, "Report");
//! assert_eq!(saves.get()?, 1);
//! assert_eq!(
//!     driver.semantics(&field)?.value,
//!     Some(SemanticValue::Text("Report".into())),
//! );
//! let save = semantics_of(&driver, &root, &["save"])?;
//! assert!(matches!(save.role, SemanticRole::Button));
//! assert_eq!(save.name.as_deref(), Some("Save"));
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
use crate::{
    CommandKey, FailurePhase, KeyChord, KeyCode, Modifiers, Point, RevealOutcome, Runtime,
    RuntimeError, TextCommand, TextCommandOutcome, WidgetId,
};
use std::time::{Duration, Instant};

#[doc(no_inline)]
pub use crate::host::HostIntegration;
// Input by hand, the viewport and what the driver's steps return.
#[doc(no_inline)]
pub use crate::{
    ClickCounter, CommandKeyInput, DrawError, FlushReport, FocusDirection, InputOutcome, KeyAction,
    Magnification, PointerButton, PointerEvent, PointerKind, RevealRefusal, Viewport,
};
// Reading what the runtime shows: semantics, queries and validation.
#[doc(no_inline)]
pub use crate::{
    ComponentDetails, ComponentInspection, Depth, Detail, InspectValue, InspectionQuery,
    QueryResult, Selector, SemanticAction, SemanticItem, SemanticRole, SemanticValue, Semantics,
    ValidationCode, ValidationIssue, ValidationMarker, ValidationReport,
};
// Captures.
#[doc(no_inline)]
pub use crate::{
    Capture, CaptureError, CaptureRequest, CaptureTarget, FrameImage, InspectionReply,
    NativeWindowId, Pending, Rect, WaitError,
};

/// Drives a runtime as a user would, one settled step at a time.
///
/// A driver borrows the [`Runtime`] and keeps no state of its own: its clock
/// is [`Runtime::now`], so a test builds one where it needs it,
/// `Driver::new(&mut ui)`, or keeps one for a block and reaches the runtime
/// through [`Self::runtime`] and [`Self::runtime_mut`]. Application state a
/// test reads, such as signals or an `Rc` model, stays outside it.
///
/// # Settling
///
/// Each action ends with [`Self::settle`], one [`Runtime::flush`] at
/// [`Self::FRAME`] past the runtime's clock. One flush settles what
/// `flush` settles: updates, layout, focus and hover feedback and the
/// notifications the input requested, within [`Runtime::MAX_FLUSH_WAVES`]. It
/// is not quiescence. Work a callback defers to a later flush, a background
/// result not yet delivered and timers due after the new time stay pending;
/// [`Self::wait_until`] and [`Self::advance`] reach them. Whether anything is
/// left, such as an animation or a timer that keeps running, is
/// [`Self::quiet_for`]'s check.
///
/// # Where input goes
///
/// Input goes where [`Runtime`]'s input methods send it: to the selected
/// window. The driver never selects another window itself, so a click followed
/// by typing stays in one place. A test of a detached window selects it with
/// [`HostIntegration::select_window`] first; [`Self::click`],
/// [`Self::click_item`] and [`Self::replace_text`] refuse a target in another
/// window ([`TestError::OtherWindow`]).
///
/// # Synthetic keys
///
/// [`Self::key`] and [`Self::chord`] enter through
/// [`Runtime::command_key_input`], the native host's entry point, so
/// application commands see them. They build [`CommandKeyInput::new`] events:
/// the base and logical identities are the same key, the physical code is the
/// one given, and no event carries text or a composition. They are for named
/// keys and command chords, not for typing, keyboard layouts, AltGr, dead keys
/// or an input method. Text is typed with [`Self::type_text`]. A test of a
/// printable key's native press, which carries text, of a layout or of a
/// composition builds [`CommandKeyInput`] itself, and a test of one
/// component's own key handling calls [`Runtime::key_input`].
///
/// # Neutral input and cleanup
///
/// [`Self::key`], [`Self::chord`], [`Self::replace_text`], [`Self::click`]
/// and [`Self::click_item`] require that no key is held and no modifier is reported
/// ([`Runtime::keyboard`]), and report [`TestError::InputHeld`] otherwise, so
/// a helper never releases a press the test made through the low-level
/// methods. The runtime exposes no pointer-button state: a test that pressed a
/// button through [`Runtime::pointer_input`] releases it before
/// [`Self::click`] or [`Self::click_item`].
///
/// A compound action releases everything it pressed, even when a step fails:
/// a key or button whose press returned an error is still released, because
/// delivery can report an unrelated error after admitting the press. Cleanup
/// goes on after a release fails. The action's own error is returned; when
/// cleanup failed too, [`TestError::Cleanup`] carries both, with the first
/// cleanup failure, and later cleanup failures of the same action are not
/// reported. When only cleanup failed, its first error is the result.
///
/// ```
/// # use pert_ui_runtime as pert_ui;
/// use pert_ui::prelude::*;
/// use pert_ui::testing::*;
///
/// let mut ui = Runtime::new();
/// ui.set_viewport(Viewport::new(300.0, 100.0, 1.0)?)?;
/// let text = ui.signal(String::new());
/// let field = ui.mount_tree(Element::new("field", TextField::new(text.clone())))?;
/// let mut driver = Driver::new(&mut ui);
/// driver.settle()?;
/// driver.runtime_mut().focus(&field)?;
/// driver.type_text("Hello")?;
/// assert_eq!(text.get()?, "Hello");
/// assert_eq!(driver.runtime().focused(), Some(field));
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[must_use = "a driver does nothing until an action is called on it"]
pub struct Driver<'a> {
    runtime: &'a mut Runtime,
}

/// Shows no runtime state: the runtime has its own inspection.
impl std::fmt::Debug for Driver<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Driver").finish_non_exhaustive()
    }
}

/// A modifier's flag in a [`Modifiers`] snapshot.
type ModifierFlag = fn(&mut Modifiers) -> &mut bool;

/// The synthetic order in which [`Driver::chord`] presses modifiers, with
/// each modifier's left physical key and its flag in a [`Modifiers`]
/// snapshot.
const MODIFIER_KEYS: [(KeyCode, ModifierFlag); 4] = [
    (KeyCode::ControlLeft, |m| &mut m.control),
    (KeyCode::AltLeft, |m| &mut m.alt),
    (KeyCode::ShiftLeft, |m| &mut m.shift),
    (KeyCode::SuperLeft, |m| &mut m.super_key),
];

/// The outcome of one compound action: the first error of its own steps and
/// the first error of the cleanup that releases what it pressed.
#[derive(Default)]
struct Steps {
    error: Option<TestError>,
    cleanup: Option<RuntimeError>,
}
impl Steps {
    /// Records an action step's error; returns whether the step succeeded.
    fn step<T>(&mut self, result: Result<T, RuntimeError>) -> bool {
        match result {
            Ok(_) => true,
            Err(error) => {
                if self.error.is_none() {
                    self.error = Some(error.into());
                }
                false
            }
        }
    }
    /// Records a cleanup step's error, keeping only the first.
    fn cleanup<T>(&mut self, result: Result<T, RuntimeError>) {
        if let Err(error) = result
            && self.cleanup.is_none()
        {
            self.cleanup = Some(error);
        }
    }
    fn finish(self) -> Result<(), TestError> {
        match (self.error, self.cleanup) {
            (None, None) => Ok(()),
            (None, Some(cleanup)) => Err(cleanup.into()),
            (Some(error), None) => Err(error),
            (Some(error), Some(cleanup)) => Err(TestError::Cleanup {
                error: Box::new(error),
                cleanup,
            }),
        }
    }
}

/// A named key's synthetic event: the physical code is the key, and so are
/// its base and logical identities.
fn named(code: KeyCode, action: KeyAction) -> CommandKeyInput {
    CommandKeyInput::new(code, action, CommandKey::Named(code))
}

impl<'a> Driver<'a> {
    /// The clock step of one [`Self::settle`]: 16 ms, about one frame at
    /// 60 Hz.
    pub const FRAME: Duration = Duration::from_millis(16);

    /// Drives `runtime` until the driver is dropped.
    pub fn new(runtime: &'a mut Runtime) -> Self {
        Self { runtime }
    }

    /// The driven runtime, for reading what it shows.
    pub fn runtime(&self) -> &Runtime {
        self.runtime
    }

    /// The driven runtime, for input or changes the driver has no action for,
    /// such as [`HostIntegration::select_window`] or [`Runtime::focus`].
    pub fn runtime_mut(&mut self) -> &mut Runtime {
        self.runtime
    }

    /// Flushes once at [`Self::FRAME`] past [`Runtime::now`].
    ///
    /// This is the flush every action ends with. It settles what one
    /// [`Runtime::flush`] settles, not everything pending; see
    /// [settling](Self#settling). A test of repeated ticks, such as a
    /// debounce that counts frames, calls it in a loop.
    ///
    /// # Errors
    /// [`TestError::ClockOverflow`] when the clock cannot advance that far,
    /// and the errors of [`Runtime::flush`] as [`TestError::Runtime`].
    pub fn settle(&mut self) -> Result<FlushReport, TestError> {
        self.advance(Self::FRAME)
    }

    /// Flushes once at `by` past [`Runtime::now`].
    ///
    /// One deadline jump: missed deadlines produce one update, as
    /// [`Runtime::flush`] documents, so `advance(320 ms)` is not twenty
    /// 16 ms ticks. Use it to let an interval pass, such as a double-click
    /// interval between two separate clicks.
    ///
    /// # Errors
    /// [`TestError::ClockOverflow`] when the clock cannot advance by `by`,
    /// checked before anything runs, and the errors of [`Runtime::flush`] as
    /// [`TestError::Runtime`].
    ///
    /// ```
    /// # use pert_ui_runtime as pert_ui;
    /// use pert_ui::prelude::*;
    /// use pert_ui::testing::Driver;
    /// use std::time::Duration;
    ///
    /// let mut ui = Runtime::new();
    /// let mut driver = Driver::new(&mut ui);
    /// driver.advance(Duration::from_secs(1))?;
    /// assert_eq!(driver.runtime().now(), Duration::from_secs(1));
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn advance(&mut self, by: Duration) -> Result<FlushReport, TestError> {
        let at = self
            .runtime
            .now()
            .checked_add(by)
            .ok_or(TestError::ClockOverflow)?;
        Ok(self.runtime.flush(at)?)
    }

    /// Flushes until `condition` holds, for background work.
    ///
    /// Each round flushes at the same [`Runtime::now`], so the runtime clock
    /// does not move: work already due and delivered background results run,
    /// and future deadlines do not. When `condition` does not hold yet, the
    /// driver sleeps 1 ms of wall time and repeats, for at most `timeout` of
    /// wall time. It polls rather than installing
    /// [`HostIntegration::set_wake_delivery`], which would replace a delivery the test
    /// or its host set.
    ///
    /// # Errors
    /// [`TestError::TimedOut`] with the wall time waited when `condition`
    /// still does not hold after `timeout`; the errors of [`Runtime::flush`]
    /// and of `condition` as [`TestError::Runtime`].
    ///
    /// ```
    /// # use pert_ui_runtime as pert_ui;
    /// use pert_ui::prelude::*;
    /// use pert_ui::testing::Driver;
    /// use std::time::Duration;
    ///
    /// let mut ui = Runtime::new();
    /// let result = ui.signal(None::<u32>);
    /// let target = result.clone();
    /// let _task = ui.spawn(|| 6 * 7, move |answer| target.set(Some(answer)))?;
    /// let mut driver = Driver::new(&mut ui);
    /// driver.wait_until(Duration::from_secs(5), |_| Ok(result.get()?.is_some()))?;
    /// assert_eq!(result.get()?, Some(42));
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn wait_until(
        &mut self,
        timeout: Duration,
        mut condition: impl FnMut(&Runtime) -> Result<bool, RuntimeError>,
    ) -> Result<(), TestError> {
        let start = Instant::now();
        loop {
            self.runtime.flush(self.runtime.now())?;
            if condition(self.runtime)? {
                return Ok(());
            }
            let waited = start.elapsed();
            if waited >= timeout {
                return Err(TestError::TimedOut { waited });
            }
            std::thread::sleep(Duration::from_millis(1));
        }
    }

    /// Paints the selected window, lets `interval` of runtime time pass in
    /// [`Self::FRAME`] steps without painting, and reports what happened.
    ///
    /// This is the check that a window is idle: nothing updates, nothing is
    /// scheduled and nothing needs painting. A test paints first so that
    /// damage from before the call does not count, and lets time pass in
    /// frames because one jump of the whole interval runs a periodic
    /// component once, as [`Self::advance`] documents. The report's
    /// [`QuietReport::is_idle`] is the usual assertion; its fields say which
    /// kind of work remains.
    ///
    /// # The interval
    ///
    /// - **Baseline:** one [`Runtime::flush`] at the current
    ///   [`Runtime::now`], then one [`Runtime::paint`], whose display list is
    ///   dropped. Work already due when the call starts runs here and is not
    ///   counted. The baseline does not move the clock, so the clock moves by
    ///   exactly `interval`.
    /// - **Steps:** the clock advances to `now + interval` in [`Self::FRAME`]
    ///   steps, one [`Runtime::flush`] each, and nothing is painted between
    ///   them, so damage made anywhere in the interval is still pending at its
    ///   end. A final partial frame is one shorter step that lands exactly on
    ///   the end: `quiet_for(40 ms)` flushes at 16, 32 and 40 ms past the
    ///   start.
    /// - **A zero interval** makes no step: the report holds the state right
    ///   after the baseline, a check that nothing is scheduled now without
    ///   letting time pass.
    /// - **A refused baseline:** when the baseline flush leaves work queued,
    ///   such as a command a notification queued, the runtime refuses the
    ///   paint with [`DrawError::PendingUpdates`]. That work is activity, not
    ///   an error: the interval runs from the unpainted state and
    ///   [`QuietReport::baseline_painted`] is false, so the report is not
    ///   idle. The refusal is told apart from a component that returns
    ///   `PendingUpdates` itself by [`Runtime::last_failure`], which names no
    ///   [`FailurePhase::Paint`] site after a refusal; a component's own
    ///   error ends the call.
    ///
    /// # Scope
    ///
    /// The selected window's own updates, deadlines, timers and presentation,
    /// as [`Self::settle`] covers them, plus the work all windows share that
    /// [`Runtime::next_deadline`] includes: interval timers, view-scale
    /// settling, polled live sources, queued deferred tasks and background
    /// results that have arrived, whichever window they serve. Another window's own timer or animation does not
    /// make the report non-idle, because the driver's flushes do not run it;
    /// a test checks that window by selecting it with
    /// [`HostIntegration::select_window`] and calling `quiet_for` again.
    /// [`QuietReport::components_updated`] counts the notification followups
    /// a step delivers in other visible windows, as [`FlushReport`] does.
    ///
    /// A background worker still computing is not observed: its thread runs
    /// outside the runtime until its result arrives, so a report can be idle
    /// while a worker of [`Runtime::spawn`] is blocked or busy. A result that
    /// arrives during the interval is delivered by a step and can show as
    /// activity through what its delivery changes, but delivery alone does
    /// not make the report busy, and a result that arrives later is not
    /// foreseen. A test of
    /// background work waits for its result with [`Self::wait_until`] first,
    /// then checks that the window is quiet.
    ///
    /// A runtime operation nested inside a step runs flushes of its own: a
    /// queued command's admission flushes every eligible window, a queued
    /// reveal flushes its target's window, and a deferred task or interval
    /// tick may call any operation that flushes. Those flushes can run other
    /// windows' due work, and their updates are not counted. Command admission
    /// and reveals restore the selected window, so the final observations,
    /// [`QuietReport::next_deadline`] and [`QuietReport::needs_paint`], stay
    /// the selected window's.
    ///
    /// # Cost
    ///
    /// One flush per [`Self::FRAME`] of runtime time and no wall-clock sleep:
    /// a second is 63 flushes. Choose an interval longer than the longest
    /// period the test means to rule out: a [`Spinner`](crate::Spinner) steps
    /// every 100 ms and a focused text field's caret blinks every 500 ms. A
    /// deadline beyond the interval, such as a toast's expiry, still shows in
    /// [`QuietReport::next_deadline`], so it is never mistaken for idleness.
    ///
    /// No input is sent, so neutral input is not required. A component-owned
    /// held action that schedules work, such as a number field's held step
    /// button, is activity the report shows.
    ///
    /// # Errors
    /// [`TestError::ClockOverflow`] when `now + interval` overflows, checked
    /// before anything is flushed or painted. A flush's error, of the baseline
    /// or of a step, as [`TestError::Runtime`], ending the call with the clock
    /// where that flush left it. A paint error other than the refusal above,
    /// a component's own [`DrawError::PendingUpdates`] included, as
    /// [`TestError::Draw`], or as [`TestError::Runtime`] for
    /// [`DrawError::Runtime`].
    ///
    /// ```
    /// # use pert_ui_runtime as pert_ui;
    /// use pert_ui::prelude::*;
    /// use pert_ui::testing::*;
    /// use std::time::Duration;
    ///
    /// let mut ui = Runtime::new();
    /// ui.set_viewport(Viewport::new(300.0, 100.0, 1.0)?)?;
    /// let busy = ui.signal(true);
    /// ui.mount_tree(Element::new("busy", Spinner::bound(busy.clone())))?;
    /// let mut driver = Driver::new(&mut ui);
    /// driver.settle()?;
    /// // A spinner steps every 100 ms.
    /// let report = driver.quiet_for(Duration::from_millis(250))?;
    /// assert!(!report.is_idle());
    /// assert!(report.components_updated >= 2 && report.needs_paint);
    /// busy.set(false)?;
    /// assert!(driver.quiet_for(Duration::from_millis(250))?.is_idle());
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn quiet_for(&mut self, interval: Duration) -> Result<QuietReport, TestError> {
        let start = self.runtime.now();
        let end = start
            .checked_add(interval)
            .ok_or(TestError::ClockOverflow)?;
        self.runtime.flush(start)?;
        let baseline_painted = match self.runtime.paint() {
            Ok(_) => true,
            // The runtime's refusal records no failure site; a component's
            // own `PendingUpdates` records a paint site and is its error.
            Err(DrawError::PendingUpdates)
                if !self
                    .runtime
                    .last_failure()
                    .is_some_and(|site| site.phase == FailurePhase::Paint) =>
            {
                false
            }
            Err(error) => return Err(error.into()),
        };
        let mut components_updated = 0_usize;
        let mut at = start;
        while at < end {
            at = at
                .checked_add(Self::FRAME)
                .map_or(end, |next| next.min(end));
            let report = self.runtime.flush(at)?;
            components_updated = components_updated.saturating_add(report.components_updated);
        }
        Ok(QuietReport {
            components_updated,
            next_deadline: self.runtime.next_deadline(),
            needs_paint: HostIntegration::needs_paint(&*self.runtime),
            baseline_painted,
        })
    }

    /// The component at the key path `path` below `from`, as
    /// [`Runtime::find`] resolves it.
    ///
    /// When nothing is mounted there, the error names the path and the
    /// longest leading part of it that does resolve below `from`, so a test
    /// sees which segment failed. A prefix resolves when `find` returns a
    /// component for it or refuses it as ambiguous, since `find` refuses
    /// only several matches of the last segment. Only the failure path looks
    /// for it, with at most one further lookup per shorter prefix.
    ///
    /// # Errors
    /// [`TestError::NotFound`] with the path and its resolved prefix when
    /// nothing is mounted there, and the errors of [`Runtime::find`] as
    /// [`TestError::Runtime`], such as `AmbiguousKey`.
    ///
    /// ```
    /// # use pert_ui_runtime as pert_ui;
    /// use pert_ui::prelude::*;
    /// use pert_ui::testing::*;
    ///
    /// let mut ui = Runtime::new();
    /// let root = ui.mount_tree(Element::new("form", Column::new([
    ///     Element::new("r1", Row::new([Element::new("name", Label::new("Name"))])),
    /// ])))?;
    /// let driver = Driver::new(&mut ui);
    /// let error = driver.find(&root, &["r1", "missing"]).unwrap_err();
    /// assert!(matches!(&error, TestError::NotFound { resolved, .. } if resolved == &["r1"]));
    /// assert_eq!(
    ///     error.to_string(),
    ///     "no component is mounted at r1/missing; the path resolves as far as r1",
    /// );
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn find(&self, from: &WidgetId, path: &[&str]) -> Result<WidgetId, TestError> {
        if let Some(found) = self.runtime.find(from, path)? {
            return Ok(found);
        }
        let mut resolved = &path[..0];
        for length in (1..path.len()).rev() {
            match self.runtime.find(from, &path[..length]) {
                Ok(Some(_)) | Err(RuntimeError::AmbiguousKey(_)) => {
                    resolved = &path[..length];
                    break;
                }
                Ok(None) => {}
                Err(error) => return Err(error.into()),
            }
        }
        let owned = |segments: &[&str]| -> Vec<String> {
            segments
                .iter()
                .map(|segment| (*segment).to_owned())
                .collect()
        };
        Err(TestError::NotFound {
            path: owned(path),
            resolved: owned(resolved),
        })
    }

    /// The components one address segment below `parent`, in child order,
    /// each with the segment [`Runtime::find`] resolves it by.
    ///
    /// A component is listed with segment `s` exactly when
    /// `Runtime::find(parent, &[s])` matches it: `find` returns it, or
    /// refuses with [`RuntimeError::AmbiguousKey`] because a second listed
    /// component has the same segment. Both are then listed, in child
    /// order, and `find` refuses that address. The walk goes through
    /// structural levels in child order: an element contributes its key and
    /// a slot its slot name, and nothing below either is listed; a part
    /// contributes its part name and its descendants are listed too, since
    /// they address without it.
    ///
    /// The result is relative to `parent`, as `find` resolves from it, so a
    /// structural parent, a slot or a part is a valid parent. It includes
    /// hidden components, components presented in a detached window and
    /// every row a virtualized collection has mounted, offscreen retained
    /// ones included. A row that is not mounted is not listed: the
    /// collection's controller mounts it, such as
    /// [`ListController::reveal`](crate::ListController::reveal) or
    /// [`TableController::reveal`](crate::TableController::reveal), and the
    /// next settle lists it.
    ///
    /// | Parent | Result |
    /// |---|---|
    /// | An [`Each`](crate::Each) with its default container | its rows in list order, each under its item key: the container is structural |
    /// | An `Each` with [`within`](crate::Each::within) | the one container, under its key; `address_children(&driver.find(&each, &["tags"])?)` lists the rows |
    /// | A [`VirtualList`](crate::VirtualList) | every mounted row in index order: the visible lines, the overscan, a row pinned by focus or a pointer gesture and the `keep_mounted` row |
    /// | A [`Table`](crate::Table) | its mounted rows in source order, with the same retention; its body and edit-end reporter are structural |
    /// | A row the application marks [`Element::structural`](crate::Element::structural) | the row is looked through: its addressable children appear in its place |
    ///
    /// Child order is description order. For every built-in collection that
    /// is display order, reading order for a lane grid, and for an external
    /// container it is whatever order the container describes. An `Each`
    /// item keeps the addressing its element had; `Each` replaces only its
    /// key.
    ///
    /// [`Selector::children`] of `parent`'s path matches the same set only
    /// when `parent` is an element or a slot whose path no other element or
    /// slot has, and its results are in incarnation order. Below a part,
    /// whose name its descendants do not carry, it finds none of what this
    /// lists, and when two components share a path it also returns the
    /// other one's children. [`Runtime::children`] stays the structure walk for hosts and
    /// custom components: it lists structural wrappers too.
    ///
    /// The call only reads: it flushes nothing and moves no clock. It visits
    /// each component reachable from `parent` through structural and part
    /// levels once.
    ///
    /// # Errors
    /// `ForeignRuntime` or `Unmounted` as [`TestError::Runtime`] for a
    /// `parent` that is not a mounted component of this runtime, as
    /// [`Runtime::children`]. A leaf lists nothing.
    ///
    /// ```
    /// # use pert_ui_runtime as pert_ui;
    /// use pert_ui::prelude::*;
    /// use pert_ui::testing::*;
    ///
    /// let mut ui = Runtime::new();
    /// ui.set_viewport(Viewport::new(300.0, 200.0, 1.0)?)?;
    /// let tasks = ui.signal(vec!["write".to_owned(), "ship".to_owned()]);
    /// let list = ui.mount_tree(
    ///     Each::new(&tasks, Clone::clone, |_, task| Label::new(task.as_str())).key("tasks"),
    /// )?;
    /// let mut driver = Driver::new(&mut ui);
    /// driver.settle()?;
    /// tasks.set(vec!["ship".to_owned(), "write".to_owned()])?;
    /// driver.settle()?;
    /// let rows = driver.address_children(&list)?;
    /// let segments: Vec<&str> = rows.iter().map(|row| row.segment.as_str()).collect();
    /// assert_eq!(segments, ["ship", "write"]);
    /// assert_eq!(rows[0].id, driver.find(&list, &["ship"])?);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn address_children(&self, parent: &WidgetId) -> Result<Vec<AddressChild>, TestError> {
        Ok(self
            .runtime
            .address_children_of(parent)?
            .into_iter()
            .map(|(segment, id)| AddressChild { segment, id })
            .collect())
    }

    /// The semantics `id` reports now, through one [`Runtime::query`] with
    /// [`Selector::id`] at [`Detail::Semantics`].
    ///
    /// Static text, such as a label's or a button's, is the name; an editable
    /// or stateful control's content is the value. Empty text is no name: an
    /// empty label's `name` is `None`.
    ///
    /// # Errors
    /// [`TestError::NoSemantics`] for a component that reports none, and
    /// `ForeignRuntime` or `Unmounted` as [`TestError::Runtime`] for a handle
    /// that is not a mounted component of this runtime.
    pub fn semantics(&self, id: &WidgetId) -> Result<Semantics, TestError> {
        let incarnation = self.runtime.inspection_id(id)?;
        let result = self.runtime.query(
            &InspectionQuery::components(Selector::id(incarnation)).detail(Detail::Semantics),
        );
        match result
            .components
            .into_iter()
            .next()
            .and_then(|component| component.details.semantics)
        {
            Some(semantics) => Ok(semantics),
            None => Err(TestError::NoSemantics {
                key: self.key_of(id)?,
            }),
        }
    }

    /// Clicks the middle of `id` with the primary button, then settles.
    ///
    /// The target is revealed first with [`Runtime::reveal`], which opens
    /// hiding containers and scrolls it into view and settles presentation
    /// itself. The press and release then go through
    /// [`Runtime::pointer_input`] at the middle of [`Runtime::bounds`].
    /// Whether the press reaches the target is not checked: a surface over it
    /// is a result the test asserts on.
    ///
    /// The clock moves only by the settle, so two clicks at one point are
    /// [`Self::FRAME`] apart. That is within
    /// [`ClickCounter::DEFAULT_INTERVAL`](crate::ClickCounter::DEFAULT_INTERVAL),
    /// so a built-in control counts them as a double click, or continues a
    /// press series that was already running; a component with its own
    /// interval counts by that one. For two separate single clicks, put
    /// [`Self::advance`] beyond the component's interval between them.
    ///
    /// Requires neutral keyboard input and no pointer button held; see
    /// [neutral input](Self#neutral-input-and-cleanup).
    ///
    /// # Errors
    /// Before any input: `ForeignRuntime` or `Unmounted` as
    /// [`TestError::Runtime`]; [`TestError::OtherWindow`] for a target outside
    /// the selected window; [`TestError::InputHeld`]. Then
    /// [`TestError::NotRevealed`] with the refusal of a reveal that stopped,
    /// `Unmounted` as [`TestError::Runtime`] for a target the reveal
    /// unmounted, [`TestError::NotLaidOut`] for a target without bounds, and
    /// the errors of [`Runtime::reveal`], [`Runtime::pointer_input`] and the
    /// settle, with [cleanup](Self#neutral-input-and-cleanup) as described.
    pub fn click(&mut self, id: &WidgetId) -> Result<(), TestError> {
        let key = self.key_of(id)?;
        self.require_selected(id, &key)?;
        self.require_neutral()?;
        match self.runtime.reveal(id)? {
            RevealOutcome::Refused(refusal) => {
                return Err(TestError::NotRevealed { key, refusal });
            }
            RevealOutcome::Unmounted => return Err(RuntimeError::Unmounted.into()),
            _ => {}
        }
        let [x, y, width, height] = self
            .runtime
            .bounds(id)?
            .ok_or(TestError::NotLaidOut { key })?
            .values();
        let at = Point::new(x + width / 2.0, y + height / 2.0).map_err(RuntimeError::from)?;
        let mut steps = Steps::default();
        steps.step(self.pointer(at, PointerKind::Press(PointerButton::Primary)));
        steps.cleanup(self.pointer(at, PointerKind::Release(PointerButton::Primary)));
        steps.finish()?;
        self.settle()?;
        Ok(())
    }

    /// Clicks the middle of the visible part of item `index` of `id`'s
    /// [`Semantics::items`] with the primary button, then settles.
    ///
    /// An item is a row a component draws itself rather than a child
    /// component, such as a [`Table`](crate::Table)'s column title, a menu
    /// command or a tab title; a component that gives an item its rectangle
    /// ([`SemanticItem::local_bounds`]) reports it as
    /// [`SemanticItem::bounds`], and only such an item can be clicked. Among
    /// the built-in components, a table's column titles do.
    ///
    /// The items are read with [`Self::semantics`]; the component is then
    /// revealed with [`Runtime::reveal`], as [`Self::click`] does, and its
    /// items are read again, since revealing flushes and may scroll. The press
    /// and release go through [`Runtime::pointer_input`] at the middle of the
    /// item's bounds, the visible part of the item.
    ///
    /// This is pointer input, not a semantic action: what the press does is
    /// the component's, and whether it reaches the component is not checked.
    /// On a table title, a press within the resize grip of a column's
    /// trailing edge starts a resize rather than a sort, and the middle of a
    /// title that horizontal scrolling leaves only a narrow visible part of
    /// can fall there. The driver does not scroll a component's own content
    /// to the item: a table column scrolled out of view has no bounds; bring
    /// it into view with Left or Right on the focused table, which scroll the
    /// active cell's column into view, or with a horizontal wheel through
    /// [`Runtime::pointer_input`]. A column can also be sorted from the
    /// keyboard, with Alt and Up or Down on the active column.
    ///
    /// The clock moves only by the settle, so two `click_item`s are
    /// [`Self::FRAME`] apart and a component that counts clicks sees a double
    /// click, as for [`Self::click`]; put [`Self::advance`] beyond its
    /// interval between them for two single clicks.
    ///
    /// Requires neutral keyboard input and no pointer button held; see
    /// [neutral input](Self#neutral-input-and-cleanup).
    ///
    /// # Errors
    /// Before any input, with nothing changed: `ForeignRuntime` or
    /// `Unmounted` as [`TestError::Runtime`]; [`TestError::OtherWindow`] for
    /// a target outside the selected window; [`TestError::InputHeld`];
    /// [`TestError::NoSemantics`] for a component that reports none and
    /// [`TestError::NoItem`] for an index it reports no item at. Then
    /// [`TestError::NotRevealed`] with the refusal of a reveal that stopped,
    /// `Unmounted` as [`TestError::Runtime`] for a target the reveal
    /// unmounted, `NoSemantics` or `NoItem` again when the reveal changed the
    /// items, [`TestError::ItemWithoutBounds`] for an item without bounds,
    /// and the errors of [`Runtime::reveal`], [`Runtime::pointer_input`] and
    /// the settle, with [cleanup](Self#neutral-input-and-cleanup) as
    /// described.
    ///
    /// ```
    /// # use pert_ui_runtime as pert_ui;
    /// use pert_ui::prelude::*;
    /// use pert_ui::testing::*;
    /// use std::{cell::RefCell, rc::Rc};
    ///
    /// struct Files;
    /// impl TableItems for Files {
    ///     fn len(&self) -> usize { 2 }
    ///     fn key(&self, index: usize) -> Option<String> {
    ///         (index < 2).then(|| format!("file{index}"))
    ///     }
    ///     fn cell(&self, index: usize, column: &str) -> Option<Element> {
    ///         let text = match column {
    ///             "name" => ["b.txt", "a.txt"].get(index)?,
    ///             _ => ["2 KB", "1 KB"].get(index)?,
    ///         };
    ///         Some(Element::new("text", Label::new(*text)))
    ///     }
    /// }
    /// let mut ui = Runtime::new();
    /// ui.set_viewport(Viewport::new(320.0, 200.0, 1.0)?)?;
    /// let controller = TableController::new(
    ///     &ui,
    ///     ListSelection::new(&ui, SelectionMode::Single),
    ///     [
    ///         TableColumn::new("name", "Name").sortable(true),
    ///         TableColumn::new("size", "Size").sortable(true),
    ///     ],
    ///     TableSource::new(Files),
    /// )?;
    /// let requests = Rc::new(RefCell::new(Vec::new()));
    /// let seen = requests.clone();
    /// let table = ui.mount_tree(Element::new("files", Table::new(&controller, 24.0)?
    ///     .on_sort(move |_, sort| {
    ///         seen.borrow_mut().push((sort.column.clone(), sort.direction));
    ///         Ok(())
    ///     })))?;
    /// let mut driver = Driver::new(&mut ui);
    /// driver.settle()?;
    /// driver.click_item(&table, 1)?;
    /// assert_eq!(*requests.borrow(), [("size".to_owned(), SortDirection::Ascending)]);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn click_item(&mut self, id: &WidgetId, index: usize) -> Result<(), TestError> {
        let key = self.key_of(id)?;
        self.require_selected(id, &key)?;
        self.require_neutral()?;
        self.item(id, &key, index)?;
        match self.runtime.reveal(id)? {
            RevealOutcome::Refused(refusal) => {
                return Err(TestError::NotRevealed { key, refusal });
            }
            RevealOutcome::Unmounted => return Err(RuntimeError::Unmounted.into()),
            _ => {}
        }
        let [x, y, width, height] = self
            .item(id, &key, index)?
            .bounds
            .ok_or(TestError::ItemWithoutBounds { key, index })?;
        let at = Point::new(x + width / 2.0, y + height / 2.0).map_err(RuntimeError::from)?;
        let mut steps = Steps::default();
        steps.step(self.pointer(at, PointerKind::Press(PointerButton::Primary)));
        steps.cleanup(self.pointer(at, PointerKind::Release(PointerButton::Primary)));
        steps.finish()?;
        self.settle()?;
        Ok(())
    }

    /// Item `index` of `id`'s semantics, as an inspection reports it now.
    fn item(&self, id: &WidgetId, key: &str, index: usize) -> Result<SemanticItem, TestError> {
        let mut items = self.semantics(id)?.items;
        let count = items.len();
        if index < count {
            Ok(items.swap_remove(index))
        } else {
            Err(TestError::NoItem {
                key: key.to_owned(),
                index,
                count,
            })
        }
    }

    /// Replaces the whole text of the editor `id` with `text`, then settles.
    ///
    /// Focuses the editor, selects all of its text with
    /// [`TextCommand::SelectAll`] through [`Runtime::text_command`], then
    /// types `text` with [`Runtime::text_input`], or presses Backspace when
    /// `text` is empty, because empty committed text inserts nothing and
    /// leaves a selection. What is written is what the editor's own rules
    /// write, as with [`Self::type_text`].
    ///
    /// Requires neutral keyboard input; see
    /// [neutral input](Self#neutral-input-and-cleanup).
    ///
    /// # Errors
    /// Before any input: `ForeignRuntime` or `Unmounted` as
    /// [`TestError::Runtime`]; [`TestError::OtherWindow`] for an editor outside
    /// the selected window; [`TestError::InputHeld`]. Then the errors of
    /// [`Runtime::focus`] and [`Runtime::text_command`],
    /// [`TestError::NotAnEditor`] for a component that is not a text editor,
    /// and the errors of typing, of the Backspace key with its
    /// [cleanup](Self#neutral-input-and-cleanup) and of the settle.
    ///
    /// ```
    /// # use pert_ui_runtime as pert_ui;
    /// use pert_ui::prelude::*;
    /// use pert_ui::testing::*;
    ///
    /// let mut ui = Runtime::new();
    /// ui.set_viewport(Viewport::new(300.0, 100.0, 1.0)?)?;
    /// let text = ui.signal(String::from("Example"));
    /// let field = ui.mount_tree(Element::new("field", TextField::new(text.clone())))?;
    /// let mut driver = Driver::new(&mut ui);
    /// driver.settle()?;
    /// driver.replace_text(&field, "Replaced")?;
    /// assert_eq!(text.get()?, "Replaced");
    /// driver.replace_text(&field, "")?;
    /// assert_eq!(text.get()?, "");
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn replace_text(&mut self, id: &WidgetId, text: &str) -> Result<(), TestError> {
        let key = self.key_of(id)?;
        self.require_selected(id, &key)?;
        self.require_neutral()?;
        self.runtime.focus(id)?;
        if self.runtime.text_command(id, TextCommand::SelectAll)? == TextCommandOutcome::NotAnEditor
        {
            return Err(TestError::NotAnEditor { key });
        }
        if text.is_empty() {
            self.tap(named(KeyCode::Backspace, KeyAction::Press))?;
        } else {
            self.runtime.text_input(text)?;
        }
        self.settle()?;
        Ok(())
    }

    /// Types `text` at the caret of the focused component, then settles.
    ///
    /// The text goes through [`Runtime::text_input`] and is written by the
    /// editor's own rules: a plain field writes its binding, a field with a
    /// normalizer or a length limit holds a draft until Enter or focus loss,
    /// and a parameter field holds its draft under the edit protocol. Empty
    /// text inserts nothing. With nothing focused, the text is ignored.
    ///
    /// # Errors
    /// The errors of [`Runtime::text_input`] and of the settle.
    pub fn type_text(&mut self, text: &str) -> Result<(), TestError> {
        self.runtime.text_input(text)?;
        self.settle()?;
        Ok(())
    }

    /// Presses and releases the named key `code`, without modifiers, then
    /// settles.
    ///
    /// Both events are [synthetic](Self#synthetic-keys): `code` is the
    /// physical key and [`CommandKey::Named`] its identity, with no text, so
    /// a command bound to the key runs and a focused editor sees no typed
    /// character. Type text with [`Self::type_text`].
    ///
    /// Requires neutral keyboard input; see
    /// [neutral input](Self#neutral-input-and-cleanup).
    ///
    /// # Errors
    /// [`TestError::InputHeld`] before any input, then the errors of
    /// [`Runtime::command_key_input`] with
    /// [cleanup](Self#neutral-input-and-cleanup) as described, and of the
    /// settle.
    ///
    /// ```
    /// # use pert_ui_runtime as pert_ui;
    /// use pert_ui::prelude::*;
    /// use pert_ui::testing::*;
    ///
    /// let mut ui = Runtime::new();
    /// ui.set_viewport(Viewport::new(300.0, 100.0, 1.0)?)?;
    /// let tag = ui.signal(String::new());
    /// let field = ui.mount_tree(Element::new("tag",
    ///     TextField::new(tag.clone()).normalize(|text| Ok(text.trim().to_owned()))))?;
    /// let mut driver = Driver::new(&mut ui);
    /// driver.settle()?;
    /// driver.runtime_mut().focus(&field)?;
    /// driver.type_text(" draft ")?;
    /// assert_eq!(tag.get()?, ""); // a draft until Enter
    /// driver.key(KeyCode::Enter)?;
    /// assert_eq!(tag.get()?, "draft");
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn key(&mut self, code: KeyCode) -> Result<(), TestError> {
        self.require_neutral()?;
        self.tap(named(code, KeyAction::Press))?;
        self.settle()?;
        Ok(())
    }

    /// Presses `chord` with `code` as its physical key, then settles.
    ///
    /// The modifiers `chord` requires are pressed in a fixed synthetic order,
    /// Control, Alt, Shift, then Super: each modifier's left physical key,
    /// then the modifiers held so far through [`Runtime::native_modifiers`].
    /// `code` is then pressed and released carrying `chord`'s
    /// [`KeyChord::key`] as its identity, and the modifiers are released in
    /// reverse order, each followed by the smaller snapshot. The order is
    /// deterministic, not a platform's: winit reports modifiers before the
    /// key on Windows and after it on macOS and Wayland, and macOS can report
    /// a modifier change with no key event, so a component that reads
    /// modifiers during a modifier's own press can observe the difference.
    ///
    /// `code` is the physical key because a character chord such as
    /// [`KeyChord::primary`]`('s')` names no physical key; the driver does not
    /// guess a keyboard layout. The events are
    /// [synthetic](Self#synthetic-keys).
    ///
    /// Afterwards the runtime holds the empty modifier snapshot the chord
    /// reported last, as after a native host's report: physical key events do
    /// not change [`KeyboardState::modifiers`](crate::KeyboardState::modifiers)
    /// until the next report, so a test that later holds a modifier by hand
    /// across other input reports it with [`Runtime::native_modifiers`] too.
    ///
    /// Requires neutral keyboard input; see
    /// [neutral input](Self#neutral-input-and-cleanup).
    ///
    /// # Errors
    /// [`TestError::InputHeld`] before any input, then the errors of
    /// [`Runtime::command_key_input`] and [`Runtime::native_modifiers`]. A
    /// failed step presses nothing further; every modifier pressed, the key
    /// when it was pressed, and the empty modifier snapshot are still
    /// released, with [cleanup](Self#neutral-input-and-cleanup) as described.
    /// Then the errors of the settle.
    ///
    /// ```
    /// # use pert_ui_runtime as pert_ui;
    /// use pert_ui::prelude::*;
    /// use pert_ui::testing::Driver;
    ///
    /// let mut ui = Runtime::new();
    /// let saves = ui.signal(0);
    /// let counter = saves.clone();
    /// ui.register_command("Save", CommandScope::Application, vec![KeyChord::primary('s')],
    ///     move |_, _| counter.update(|n| *n += 1))?;
    /// let mut driver = Driver::new(&mut ui);
    /// driver.chord(KeyChord::primary('s'), KeyCode::KeyS)?;
    /// assert_eq!(saves.get()?, 1);
    /// assert_eq!(driver.runtime().keyboard().modifiers(), Modifiers::default());
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn chord(&mut self, chord: KeyChord, code: KeyCode) -> Result<(), TestError> {
        self.require_neutral()?;
        let mut wanted = chord.modifiers();
        let mut held = Modifiers::default();
        let mut pressed = Vec::new();
        let mut steps = Steps::default();
        for (modifier, flag) in MODIFIER_KEYS {
            if !std::mem::take(flag(&mut wanted)) {
                continue;
            }
            // Pressed even when the press reports an error, so it is released.
            pressed.push((modifier, flag));
            *flag(&mut held) = true;
            if !steps.step(
                self.runtime
                    .command_key_input(named(modifier, KeyAction::Press)),
            ) || !steps.step(self.runtime.native_modifiers(held))
            {
                break;
            }
        }
        if steps.error.is_none() {
            steps.step(self.runtime.command_key_input(CommandKeyInput::new(
                code,
                KeyAction::Press,
                chord.key(),
            )));
            steps.cleanup(self.runtime.command_key_input(CommandKeyInput::new(
                code,
                KeyAction::Release,
                chord.key(),
            )));
        }
        for (modifier, flag) in pressed.into_iter().rev() {
            steps.cleanup(
                self.runtime
                    .command_key_input(named(modifier, KeyAction::Release)),
            );
            *flag(&mut held) = false;
            steps.cleanup(self.runtime.native_modifiers(held));
        }
        steps.finish()?;
        self.settle()?;
        Ok(())
    }

    /// Clears focus in the selected window as its deactivation does, then
    /// settles.
    ///
    /// This is [`HostIntegration::clear_focus`], the call a native host makes
    /// when a window loses focus, followed by [`Self::settle`], as every
    /// action ends. Afterwards nothing in the selected window is focused.
    ///
    /// - **In the selected window:** nonmodal surfaces such as menus and
    ///   popovers close and modal dialogs stay; pointer buttons, capture,
    ///   hover and the pointer position are cleared; the focused component's
    ///   `focus_lost` runs, so a focused editor applies its focus-loss rules
    ///   and saves a valid draft, and focus-within notifications are
    ///   delivered. Another window's focus is untouched: a test of a detached
    ///   window selects it with [`HostIntegration::select_window`] first, as
    ///   for input.
    /// - **Runtime-wide:** pressed keys, command presses and reported
    ///   modifiers are cleared, a drag in this window is cancelled, and every
    ///   admitted held-input registration, in any window, is cancelled with
    ///   [`HeldCancellation::FocusLost`](crate::HeldCancellation::FocusLost),
    ///   because its releases may no longer arrive.
    ///
    /// Unlike [`Self::key`] and [`Self::click`], it does not require
    /// [neutral input](Self#neutral-input-and-cleanup): releasing held keys
    /// and modifiers is what a deactivation does. Input is neutral
    /// afterwards, so the next action is admitted.
    ///
    /// # Errors
    /// [`TestError::ClockOverflow`] when the settle's clock cannot advance. A
    /// focus-loss or focus-within callback's error, which the deactivation
    /// queues, is returned by the settle's flush as [`TestError::Runtime`];
    /// that flush delivers queued parameter terminal notifications first and
    /// returns the error before it runs commands, tasks, timers or
    /// presentation, so a further [`Self::settle`] delivers the rest, and a
    /// later queued error comes from that next flush. The other errors of
    /// [`Runtime::flush`] as [`TestError::Runtime`].
    ///
    /// ```
    /// # use pert_ui_runtime as pert_ui;
    /// use pert_ui::prelude::*;
    /// use pert_ui::testing::*;
    ///
    /// let mut ui = Runtime::new();
    /// ui.set_viewport(Viewport::new(300.0, 100.0, 1.0)?)?;
    /// let tag = ui.signal(String::new());
    /// let field = ui.mount_tree(Element::new("tag",
    ///     TextField::new(tag.clone()).normalize(|text| Ok(text.trim().to_owned()))))?;
    /// let mut driver = Driver::new(&mut ui);
    /// driver.settle()?;
    /// driver.runtime_mut().focus(&field)?;
    /// driver.type_text(" draft ")?;
    /// assert_eq!(tag.get()?, ""); // a draft until Enter or focus loss
    /// driver.clear_focus()?;
    /// assert_eq!(tag.get()?, "draft");
    /// assert_eq!(driver.runtime().focused(), None);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn clear_focus(&mut self) -> Result<(), TestError> {
        HostIntegration::clear_focus(self.runtime);
        self.settle()?;
        Ok(())
    }

    /// Presses `press`'s key and always releases it, recording both.
    fn tap(&mut self, press: CommandKeyInput) -> Result<(), TestError> {
        let mut steps = Steps::default();
        steps.step(self.runtime.command_key_input(press));
        steps.cleanup(self.runtime.command_key_input(CommandKeyInput {
            action: KeyAction::Release,
            ..press
        }));
        steps.finish()
    }

    fn pointer(&mut self, at: Point, kind: PointerKind) -> Result<InputOutcome, RuntimeError> {
        self.runtime.pointer_input(PointerEvent::new(at, kind))
    }

    fn key_of(&self, id: &WidgetId) -> Result<String, TestError> {
        Ok(self.runtime.key(id)?.to_owned())
    }

    /// Refuses a target outside the selected window, before anything changes.
    fn require_selected(&self, id: &WidgetId, key: &str) -> Result<(), TestError> {
        if self.runtime.window_of(id)? == self.runtime.current_window() {
            Ok(())
        } else {
            Err(TestError::OtherWindow {
                key: key.to_owned(),
            })
        }
    }

    /// Refuses held keys or reported modifiers, before anything changes.
    fn require_neutral(&self) -> Result<(), TestError> {
        let keyboard = self.runtime.keyboard();
        if keyboard.pressed().next().is_none() && keyboard.modifiers() == Modifiers::default() {
            Ok(())
        } else {
            Err(TestError::InputHeld)
        }
    }
}

/// What happened in the selected window over a [`Driver::quiet_for`]
/// interval.
///
/// [`Self::is_idle`] is the usual assertion; the fields say which kind of
/// work remains: components that updated during the interval, a deadline
/// still scheduled at its end, a window that needs painting, or a baseline
/// that could not be painted. It is not a [`FlushReport`]: it aggregates the
/// reports the driver's interval steps return, without the counts of flushes
/// nested inside them, and adds the window's paint need. Only the driver
/// constructs it, and fields may be added.
///
/// The report is `#[must_use]`, so `driver.quiet_for(interval)?;`, which
/// checks nothing, warns:
///
/// ```compile_fail
/// #![deny(unused_must_use)]
/// # use pert_ui_runtime as pert_ui;
/// use pert_ui::testing::{Driver, TestError};
/// use std::time::Duration;
/// fn check(driver: &mut Driver<'_>) -> Result<(), TestError> {
///     driver.quiet_for(Duration::from_millis(100))?; // denied: the report is discarded
///     Ok(())
/// }
/// ```
///
/// Reading it compiles under the same lint:
///
/// ```
/// #![deny(unused_must_use)]
/// # use pert_ui_runtime as pert_ui;
/// use pert_ui::testing::{Driver, TestError};
/// use std::time::Duration;
/// fn check(driver: &mut Driver<'_>) -> Result<(), TestError> {
///     let report = driver.quiet_for(Duration::from_millis(100))?;
///     assert!(report.is_idle());
///     Ok(())
/// }
/// # let mut ui = pert_ui::Runtime::new();
/// # check(&mut Driver::new(&mut ui))?;
/// # Ok::<(), TestError>(())
/// ```
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
#[must_use = "a quiet interval checks nothing unless its report is read"]
pub struct QuietReport {
    /// The components the interval's flushes updated: the saturating sum of
    /// each step's [`FlushReport::components_updated`], including the
    /// notification followups a step delivers in other visible windows. The
    /// baseline flush's updates are not counted. A component of another
    /// window that a binding change only marks pending is updated when that
    /// window flushes, and is not counted. Nor are the updates of a flush
    /// nested inside a step, such as a queued command's admission, a queued
    /// reveal, or a deferred task or interval tick that flushes; a test that
    /// needs them observes the components themselves.
    pub components_updated: usize,
    /// [`Runtime::next_deadline`] after the last step, or after the baseline
    /// for a zero interval: absolute runtime time, as
    /// [`FlushReport::next_deadline`] is, and the current time while
    /// immediate work waits. It holds the selected window's update deadlines
    /// and timers and the work all windows share (interval timers,
    /// view-scale settling, polled live sources, queued deferred tasks and
    /// background results that have arrived), not another window's own
    /// deadlines and not a background worker still computing. `None` when
    /// nothing is scheduled.
    pub next_deadline: Option<Duration>,
    /// [`HostIntegration::needs_paint`] after the last step, or after the
    /// baseline for a zero interval: whether the selected window needs
    /// painting at the interval's end. A capture waiting for this window keeps
    /// it true.
    pub needs_paint: bool,
    /// Whether the baseline paint succeeded. False when the runtime refused
    /// it with [`DrawError::PendingUpdates`] because the baseline flush left
    /// work queued, which keeps the report from being idle; see
    /// [`Driver::quiet_for`].
    pub baseline_painted: bool,
}
impl QuietReport {
    /// The baseline was painted, and then nothing updated, nothing is
    /// scheduled and nothing needs painting.
    #[must_use]
    pub fn is_idle(&self) -> bool {
        self.baseline_painted
            && self.components_updated == 0
            && self.next_deadline.is_none()
            && !self.needs_paint
    }
}

/// One component one address segment below a parent, as
/// [`Driver::address_children`] lists it.
///
/// `segment` is what [`Runtime::find`] and [`Driver::find`] take for this
/// component below the parent: an element's key, but a slot's or a part's
/// name, which is not what [`Runtime::key`] reads for it. Only the driver
/// constructs one; a test reads its fields.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct AddressChild {
    /// The address segment this component introduces below the parent: an
    /// element's key, or a slot's or a part's name.
    pub segment: String,
    /// The component.
    pub id: WidgetId,
}

/// Why a [`Driver`] step could not do what it was asked.
///
/// A runtime failure keeps its one form, [`Self::Runtime`]; the other variants
/// name what the driver refused or could not find. This is not an `AppError`:
/// a test returns `Result<(), Box<dyn std::error::Error>>`, which both
/// convert into.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum TestError {
    /// The runtime refused or failed an operation the step made.
    #[error(transparent)]
    Runtime(#[from] RuntimeError),
    /// No component is mounted at this key path ([`Driver::find`]).
    #[error(
        "no component is mounted at {}; {}",
        path.join("/"),
        if resolved.is_empty() {
            "not even its first segment resolves".to_owned()
        } else {
            format!("the path resolves as far as {}", resolved.join("/"))
        }
    )]
    NotFound {
        /// The path that was looked up, one key per segment.
        path: Vec<String>,
        /// The longest leading part of `path`, shorter than it, that resolves
        /// below the component the lookup started from: [`Runtime::find`]
        /// returns a component for it or refuses it as ambiguous. Empty when
        /// not even the first segment does.
        resolved: Vec<String>,
    },
    /// The component reports no item at this index ([`Driver::click_item`]).
    #[error("`{key}` reports {count} items, none at index {index}")]
    NoItem {
        /// The component's key.
        key: String,
        /// The index asked for.
        index: usize,
        /// How many items the component reports.
        count: usize,
    },
    /// The item has no visible rectangle to click ([`Driver::click_item`]):
    /// its component gives it none, or it is scrolled or clipped out of view.
    #[error("item {index} of `{key}` has no visible bounds")]
    ItemWithoutBounds {
        /// The component's key.
        key: String,
        /// The item's index.
        index: usize,
    },
    /// The target is in another window than the selected one, which receives
    /// the driver's input. Nothing changed; select the target's window with
    /// [`HostIntegration::select_window`] first.
    #[error("`{key}` is not in the selected window")]
    OtherWindow {
        /// The target's key.
        key: String,
    },
    /// Revealing the target before a click stopped ([`Driver::click`],
    /// [`Driver::click_item`]).
    #[error("`{key}` could not be revealed: {refusal:?}")]
    NotRevealed {
        /// The target's key.
        key: String,
        /// Where and why the reveal stopped.
        refusal: RevealRefusal,
    },
    /// The target has no bounds to click after its reveal ([`Driver::click`]).
    ///
    /// The reveal flushes, and layout gives every revealed component its
    /// bounds, so this is the driver's typed report of a broken layout
    /// invariant rather than a panic; a test should not reach it.
    #[error("`{key}` has not been laid out")]
    NotLaidOut {
        /// The target's key.
        key: String,
    },
    /// The target is not a text editor ([`Driver::replace_text`]).
    #[error("`{key}` is not a text editor")]
    NotAnEditor {
        /// The target's key.
        key: String,
    },
    /// The component reports no semantics ([`Driver::semantics`],
    /// [`Driver::click_item`]).
    #[error("`{key}` reports no semantics")]
    NoSemantics {
        /// The component's key.
        key: String,
    },
    /// The condition of [`Driver::wait_until`] did not hold in time.
    #[error("the condition did not hold within {waited:?}")]
    TimedOut {
        /// The wall time waited.
        waited: Duration,
    },
    /// The runtime clock cannot advance that far without overflowing
    /// ([`Driver::advance`], [`Driver::quiet_for`] and the settle every
    /// action ends with).
    #[error("the runtime clock cannot advance that far")]
    ClockOverflow,
    /// A key is held or a modifier is reported, so the driver's keys would not
    /// be what it sends ([`Driver::key`], [`Driver::chord`],
    /// [`Driver::replace_text`], [`Driver::click`], [`Driver::click_item`]).
    /// Release them first.
    #[error("a key or modifier is held; release it before driving input")]
    InputHeld,
    /// A step failed, and releasing what the action had pressed failed too.
    #[error("{error}; releasing the pressed input also failed")]
    Cleanup {
        /// The step's own error.
        error: Box<TestError>,
        /// The first cleanup failure; later ones in the same action are not
        /// reported.
        #[source]
        cleanup: RuntimeError,
    },
    /// Painting the selected window failed ([`Driver::quiet_for`]'s baseline
    /// paint), such as a component whose paint returned
    /// [`DrawError::TooComplex`].
    ///
    /// The driver and `From<DrawError> for TestError` report a runtime
    /// failure as [`Self::Runtime`] instead, so neither produces
    /// [`DrawError::Runtime`] here.
    ///
    /// ```
    /// # use pert_ui_runtime as pert_ui;
    /// use pert_ui::prelude::*;
    /// use pert_ui::testing::*;
    ///
    /// /// Flushes and paints the selected window, as a host's frame does.
    /// fn frame(ui: &mut Runtime) -> Result<(), TestError> {
    ///     ui.flush(ui.now())?;
    ///     ui.paint()?;
    ///     Ok(())
    /// }
    ///
    /// let mut ui = Runtime::new();
    /// ui.set_viewport(Viewport::new(300.0, 100.0, 1.0)?)?;
    /// ui.mount_tree(Element::new("title", Label::new("Hello")))?;
    /// frame(&mut ui)?;
    /// assert_eq!(TestError::from(DrawError::TooComplex), TestError::Draw(DrawError::TooComplex));
    /// let runtime = RuntimeError::Closed;
    /// assert_eq!(TestError::from(DrawError::Runtime(runtime.clone())), TestError::Runtime(runtime));
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    #[error(transparent)]
    Draw(DrawError),
}

/// Keeps a runtime failure in its one form: [`DrawError::Runtime`] becomes
/// [`TestError::Runtime`], every other variant [`TestError::Draw`]. A test
/// helper that returns `TestError` applies `?` to [`Runtime::paint`].
impl From<DrawError> for TestError {
    fn from(error: DrawError) -> Self {
        match error {
            DrawError::Runtime(error) => Self::Runtime(error),
            error => Self::Draw(error),
        }
    }
}
