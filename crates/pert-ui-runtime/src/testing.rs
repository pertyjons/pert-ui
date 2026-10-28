//! Driving an application in a headless test, as a user would.
//!
//! [`Driver`] clicks a component, replaces or types text, presses keys and
//! chords, settles the runtime after each step and finds a component's
//! [`Semantics`]; [`TestError`] names what a step could not do. Neither needs a
//! window, a GPU or an operating-system service. The module also re-exports
//! what a headless test writes besides the driver, so one glob covers it:
//!
//! ```
//! # use pert_ui_runtime as pert_ui;
//! use pert_ui::prelude::*;
//! use pert_ui::testing::*;
//! ```
//!
//! - [`Viewport`], which every harness sets with [`Runtime::set_viewport`];
//! - [`KeyAction`] and [`FocusDirection`], for key input and focus movement
//!   by hand through [`Runtime::key_input`] and [`Runtime::focus_next`];
//! - [`PointerEvent`], [`PointerKind`] and [`PointerButton`] for
//!   [`Runtime::pointer_input`], [`CommandKeyInput`] for
//!   [`Runtime::command_key_input`], and the [`InputOutcome`] that pointer
//!   and key input return;
//! - [`SemanticValue`], for asserting what a component's semantics report,
//!   and [`ClickCounter`], whose interval separates single clicks;
//! - [`InspectionQuery`], [`Selector`] and [`Detail`], for reading what the
//!   runtime shows through [`Runtime::query`];
//! - [`CaptureRequest`], [`CaptureTarget`] and [`FrameImage`], for a
//!   rendered capture;
//! - [`HostIntegration`], whose methods a test calls where it stands in for
//!   a host, such as selecting a native window, taking presentation requests
//!   or clearing focus as a window's deactivation would.
//!
//! The key names a chord writes ([`KeyCode`], [`Modifiers`], [`CommandKey`])
//! and [`Point`] are in the prelude.
//!
//! These names stay at the crate root too, except [`HostIntegration`], which
//! is reached through [`host`](crate::host) and this module; the module only
//! gathers them. The
//! `pert-ui` facade's `testing` module re-exports this one and adds its CPU
//! renderer and its headless application entry.
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
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
use crate::{
    CommandKey, FlushReport, KeyChord, KeyCode, Modifiers, Point, RevealOutcome, RevealRefusal,
    Runtime, RuntimeError, Semantics, TextCommand, TextCommandOutcome, WidgetId,
};
use std::time::{Duration, Instant};

#[doc(no_inline)]
pub use crate::host::HostIntegration;
#[doc(no_inline)]
pub use crate::{
    CaptureRequest, CaptureTarget, ClickCounter, CommandKeyInput, Detail, FocusDirection,
    FrameImage, InputOutcome, InspectionQuery, KeyAction, PointerButton, PointerEvent, PointerKind,
    Selector, SemanticValue, Viewport,
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
/// [`Self::wait_until`] and [`Self::advance`] reach them.
///
/// # Where input goes
///
/// Input goes where [`Runtime`]'s input methods send it: to the selected
/// window. The driver never selects another window itself, so a click followed
/// by typing stays in one place. A test of a detached window selects it with
/// [`HostIntegration::select_window`] first; [`Self::click`] and
/// [`Self::replace_text`] refuse a target in another window
/// ([`TestError::OtherWindow`]).
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
/// [`Self::key`], [`Self::chord`], [`Self::replace_text`] and [`Self::click`]
/// require that no key is held and no modifier is reported
/// ([`Runtime::keyboard`]), and report [`TestError::InputHeld`] otherwise, so
/// a helper never releases a press the test made through the low-level
/// methods. The runtime exposes no pointer-button state: a test that pressed a
/// button through [`Runtime::pointer_input`] releases it before
/// [`Self::click`].
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

    /// The component at the key path `path` below `from`, as
    /// [`Runtime::find`] resolves it.
    ///
    /// # Errors
    /// [`TestError::NotFound`] naming the path when nothing is mounted there,
    /// and the errors of [`Runtime::find`] as [`TestError::Runtime`], such as
    /// `AmbiguousKey`.
    pub fn find(&self, from: &WidgetId, path: &[&str]) -> Result<WidgetId, TestError> {
        self.runtime
            .find(from, path)?
            .ok_or_else(|| TestError::NotFound {
                path: path.iter().map(|segment| (*segment).to_owned()).collect(),
            })
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
    #[error("no component is mounted at {}", path.join("/"))]
    NotFound {
        /// The path that was looked up, one key per segment.
        path: Vec<String>,
    },
    /// The target is in another window than the selected one, which receives
    /// the driver's input. Nothing changed; select the target's window with
    /// [`HostIntegration::select_window`] first.
    #[error("`{key}` is not in the selected window")]
    OtherWindow {
        /// The target's key.
        key: String,
    },
    /// Revealing the target before a click stopped ([`Driver::click`]).
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
    /// The component reports no semantics ([`Driver::semantics`]).
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
    /// ([`Driver::advance`]).
    #[error("the runtime clock cannot advance that far")]
    ClockOverflow,
    /// A key is held or a modifier is reported, so the driver's keys would not
    /// be what it sends ([`Driver::key`], [`Driver::chord`],
    /// [`Driver::replace_text`], [`Driver::click`]). Release them first.
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
}
