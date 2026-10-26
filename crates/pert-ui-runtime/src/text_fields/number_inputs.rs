//! Numeric drafts reuse the text editor without a string binding.
use super::*;
use crate::choice_controls::rounded;

/// Rejected numeric configuration, external value, or in-progress text.
///
/// Builder errors are returned to the caller. Invalid user text is retained and
/// marked in the field; it never writes the numeric signal or causes a runtime
/// failure. Inspection exposes that draft's error as `validation_error`, and a
/// refused save reports it to the editor's `on_rejection` as an
/// [`InputRejection`] with a message from the installed
/// [`UiStrings`](crate::UiStrings). An
/// invalid bound value written by the application is refused by the field and
/// reported in inspection as `binding_error`, also without a runtime failure.
///
/// [`Self::InvalidPrecision`] carries the rejected digit count. The variants
/// that reject a number stay unit, because a decimal editor's number is an
/// `f64` and the error keeps `Eq`; the caller holds the argument it passed.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum NumberInputError {
    /// Bounds are reversed or a decimal bound is not finite.
    #[error("numeric bounds must be finite and ordered")]
    InvalidRange,
    /// The increment is not strictly positive and finite.
    #[error("numeric step must be positive and finite")]
    InvalidStep,
    /// The fixed decimal precision held here exceeds [`Self::MAX_PRECISION`]
    /// fractional digits.
    #[error(
        "decimal precision {0} must be between 0 and {max}",
        max = Self::MAX_PRECISION
    )]
    InvalidPrecision(u8),
    /// Text is not a representable number of the required kind.
    #[error("enter a representable number")]
    InvalidNumber,
    /// A decimal value is NaN or infinite.
    #[error("decimal values must be finite")]
    NonFinite,
    /// A representable number is outside the configured inclusive bounds.
    #[error("number is outside the allowed range")]
    OutOfRange,
    /// A parameter-bound number is in range but not on the parameter's step lattice.
    #[error("number is not on the parameter's step")]
    OffStep,
    /// A parameter-bound editor takes its bounds, and for a stepped parameter
    /// its step, from the parameter's specification; `range` and such a `step`
    /// cannot define a second domain.
    #[error("the parameter specification supplies the bounds and step")]
    ParameterDomain,
}
impl NumberInputError {
    /// The most fractional digits a fixed decimal display precision may have: 15.
    /// It bounds
    /// `decimal_places` on [`DecimalInput`], [`InlineDecimal`] and the
    /// numeric controls; a larger value
    /// is [`Self::InvalidPrecision`].
    ///
    /// It lives on this error because it bounds several editors, not one:
    /// [`Self::InvalidPrecision`] is the one variant that reports it.
    pub const MAX_PRECISION: u8 = 15;
}

/// A single-line integer editor bound to a UI-local `Signal<i64>`, or to a
/// stepped [`Parameter<i64>`] with [`Self::for_parameter`].
///
/// Keeps exact integers across the entire `i64` range; never converts through a
/// float. Defaults to that full range, step 1, body typography and a 240-logical-
/// pixel preferred width. Optional leading `+`/`-` and surrounding whitespace are
/// accepted. Fractions, grouping separators and exponent notation are rejected.
///
/// Enter (fresh press, outside IME composition) or focus transfer/window focus
/// loss saves a valid draft. Escape first cancels IME composition while preserving
/// committed draft text; outside composition it restores the latest bound value
/// when the draft differs from it, and ends a selection drag. With nothing to
/// restore or end, Escape is left to the field's owner, as in [`TextField`], so a dialog or cell editor around the field
/// can close on it. Empty,
/// incomplete, invalid and out-of-range text stays editable with an error outline,
/// including after an unsuccessful save; the binding retains its previous value.
/// Hiding and unmount discard drafts without saving. Input/clipboard errors and
/// geometry changes clear transient input state while preserving a focused draft.
/// External changed values win over drafts and clear composition/undo.
///
/// An external value the editor does not accept (outside the bounds, off a
/// parameter's step, or a non-finite decimal) is refused by the field rather than
/// failing the runtime: the field discards any draft or gesture, keeps showing
/// its previous value, draws the error outline, reports `binding_error` in
/// inspection and marks its semantics invalid, and accepts no edits or steps, so
/// it never overwrites the application's value. Selection and copy still work.
/// The next acceptable bound value ends the refusal. The binding itself is never
/// rewritten.
///
/// Up/Down adjust the draft by the configured step, clamping only these steps to
/// the bounds, and [`Self::stepper`] adds step buttons. Repeats are accepted. Invalid drafts ignore stepping. Stepping does
/// not save until Enter/focus loss; focused wheel stepping follows the rules below.
/// Selection, clipboard chords, IME, 16 KiB text limits, 32-edit local undo and
/// caret scheduling follow [`TextField`]. [`Runtime::text_command`] and
/// [`EventContext::request_text_command`] act on the draft as the chords do;
/// pasted text is a draft like typed text. Number inputs have no clipboard
/// context menu. Saving/Escape clear local undo. This is local
/// state, not an authoritative parameter gesture or application undo transaction.
/// All handles and callbacks belong to the UI thread.
///
/// # Formatting
///
/// Without a [`NumberFormat`] or unit the field always shows its editing text.
/// With [`Self::format`] or [`Self::unit`], it rests showing the display text
/// followed by the unit while it is unfocused and holds the unchanged bound
/// value; focus or a changed draft shows the editing text, the number alone.
/// Switching between them never changes the value.
///
/// # Normalization
///
/// [`Self::normalize`] adjusts or refuses a changed value before it is
/// validated and saved:
///
/// ```
/// # use pert_ui_runtime as pert_ui;
/// use pert_ui::{DecimalInput, Element, KeyAction, KeyCode, Runtime};
/// let mut ui = Runtime::new();
/// let gain = ui.signal(0.0_f64);
/// let field = ui.mount_tree(Element::new("gain", DecimalInput::new(gain.clone())
///     .range(-12.0, 12.0)?
///     .normalize(|value| Ok((value * 10.0).round() / 10.0))?))?;
/// ui.focus(&field)?;
/// ui.text_input("3.14159")?;
/// ui.key_input(KeyCode::Enter, KeyAction::Press)?;
/// assert_eq!(gain.get()?, 3.1);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
///
/// # Parameter binding
///
/// [`Self::for_parameter`] edits a stepped parameter under its edit protocol.
/// The specification supplies bounds and step, and drafts validate exactly
/// against its lattice: off-step text is marked invalid, never rounded. Enter
/// commits a changed valid draft as one begin/commit pair at the observed
/// revision; an unchanged draft creates no edit. Unlike the local editor,
/// focus loss, hiding and removal *cancel* a parameter draft. An external
/// base or metadata change discards the draft before anything can commit it.
/// Shift keeps the parameter's step, since no finer lattice exists. Inline
/// gestures and parameter step buttons own one session from activation:
/// a held arrow key or step button
/// previews each step and commits on release, a value drag previews and
/// commits on release, and Escape or context loss cancels the whole gesture. A
/// focused wheel event is one complete edit. The editor is read-only while the
/// parameter is read-only, unavailable or has a delivery fault, or remote authority
/// is pending, unknown or disconnected. A normalizer is
/// refused with `ParameterDomain`: the specification's step lattice defines the
/// values.
///
/// ```
/// # use pert_ui_runtime as pert_ui;
/// use pert_ui::{Element, IntegerInput, KeyAction, KeyCode, Runtime};
/// let mut ui = Runtime::new();
/// let count = ui.signal(0_i64);
/// let field = ui.mount_tree(Element::new("count",
///     IntegerInput::new(count.clone()).range(0, 100)?.step(5)?))?;
/// ui.focus(&field)?;
/// ui.text_input("42")?;
/// assert_eq!(count.get()?, 0); // still a draft ("420")
/// ui.key_input(KeyCode::Escape, KeyAction::Press)?;
/// ui.key_input(KeyCode::Escape, KeyAction::Release)?;
/// ui.key_input(KeyCode::ArrowUp, KeyAction::Press)?;
/// ui.key_input(KeyCode::Enter, KeyAction::Press)?;
/// assert_eq!(count.get()?, 5);
/// # Ok::<(), pert_ui::RuntimeError>(())
/// ```
///
/// While focused and editable, each nonzero vertical wheel event over the field
/// steps the draft once: upward increases, downward decreases, clamped to bounds.
/// Holding Shift divides keyboard/wheel steps by ten (integer division for integers,
/// minimum 1; decimals retain at least the smallest positive `f64`). Control,
/// Alt and Super chords of the arrow keys do not step; they are left to the
/// editor and to command bindings.
/// Wheel magnitude is not a multiplier. Invalid drafts, IME composition and active
/// selection drags are preserved; those focused wheel events are consumed. Other
/// wheel input follows normal text scrolling/bubbling. Wheel steps save on Enter
/// or focus loss, share keyboard-step undo and can be canceled with Escape.
#[must_use]
pub struct IntegerInput(NumberEditor<i64>);
description_debug!(IntegerInput);

/// A single-line decimal editor bound to a UI-local `Signal<f64>`.
///
/// Defaults to the finite `f64` range, step 0.1, and the shortest round-trippable
/// display. Accepts `.` or `,` as a decimal separator, an optional leading sign,
/// surrounding whitespace and decimal exponents (`1.5e-3`). Grouping separators
/// are unsupported: `1,234` means 1.234. Display always uses `.`. NaN, infinity,
/// overflow and values outside configured bounds cannot be saved. Very small
/// inputs follow `f64` rounding, including underflow to zero. This is binary
/// floating point, not exact base-ten arithmetic.
///
/// [`Self::decimal_places`] affects resting and stepped display. Enter or focus loss without
/// a changed draft never rounds the bound value. Stepping uses the full numeric
/// draft, displaying the configured precision while retaining its exact value in
/// local undo/redo. Typed text remains unrestricted by display precision.
/// Other editing, focus,
/// cancellation, validation, ownership and scheduling rules follow [`IntegerInput`].
///
/// ```
/// # use pert_ui_runtime as pert_ui;
/// use pert_ui::{DecimalInput, Element, KeyAction, KeyCode, Runtime};
/// let mut ui = Runtime::new();
/// let amount = ui.signal(1.234567_f64);
/// let field = ui.mount_tree(Element::new("amount",
///     DecimalInput::new(amount.clone()).range(-10.0, 10.0)?.decimal_places(2)?))?;
/// ui.focus(&field)?;
/// ui.key_input(KeyCode::Enter, KeyAction::Press)?;
/// assert_eq!(amount.get()?, 1.234567); // display precision is not storage precision
/// ui.key_input(KeyCode::ArrowUp, KeyAction::Press)?;
/// ui.clear_focus(); // saves the stepped draft
/// assert!((amount.get()? - 1.334567).abs() < 1e-12);
/// # Ok::<(), pert_ui::RuntimeError>(())
/// ```
#[must_use]
///
/// [`Self::for_parameter`] binds an authoritative `Parameter<f32>` instead of
/// a local signal; that constructor documents conversion and cancellation.
pub struct DecimalInput(NumberEditor<f64>);
description_debug!(DecimalInput);

/// A one-line integer row with a caption, a draggable editable value and a unit,
/// bound to a UI-local `Signal<i64>`, or to a stepped [`Parameter<i64>`] with
/// [`Self::for_parameter`] under the rules of [`IntegerInput`]'s parameter
/// binding.
///
/// Inline controls fit toolbars, channel strips and status bars at ControlHeight
/// while following the ordinary theme metrics for density and zoom. The value
/// uses [`TextStyle::Monospace`] and its unit shares that style and baseline;
/// the secondary caption uses [`TextStyle::Caption`].
///
/// Click the value or press Enter/F2 to type; drag it horizontally to step once
/// per four logical pixels after a four-pixel activation threshold. Drags
/// preview locally and save on release; focused arrows and the wheel save
/// each step immediately outside text mode. Shift uses the fine step. Text
/// mode keeps the Enter and
/// focus-loss save, Escape cancel, IME, clipboard and undo rules of
/// [`IntegerInput`], including its text commands; outside text mode the row
/// answers [`TextCommand::BeginEdit`] by entering text mode as Enter/F2 does,
/// read-only text mode included, and every other text command with
/// [`TextCommandOutcome::NotAnEditor`]. `BeginEdit` answers
/// [`TextCommandOutcome::Unavailable`] while the row is disabled, has had no
/// valid value since it mounted, or is in a step gesture (a step drag, a
/// pressed stepper or a held step key, which then ends on its release as
/// before), and in text mode. Inspection reports text mode as the
/// `text_editing` property. Hiding and removal cancel. One Tab stop covers the caption,
/// value and unit, and resting focus schedules no caret work. The preferred width
/// reserves the formatted bounds, capped at eight characters for very large
/// ranges; a custom [`NumberFormat`] supplies an explicit width example. The
/// caption and unit are literal presentation text outside selection, clipboard
/// text and IME composition. Values, bounds, steps and validation follow
/// [`IntegerInput`].
///
/// ```
/// # use pert_ui_runtime as pert_ui;
/// use pert_ui::{Element, InlineInteger, KeyAction, KeyCode, Runtime};
/// let mut ui = Runtime::new();
/// let count = ui.signal(8_i64);
/// let field = ui.mount_tree(Element::new("count",
///     InlineInteger::new(count.clone()).range(0, 100)?.label("Vol").unit("%")))?;
/// ui.focus(&field)?;
/// ui.key_input(KeyCode::ArrowUp, KeyAction::Press)?;
/// assert_eq!(count.get()?, 9); // resting steps save immediately
/// ui.key_input(KeyCode::F2, KeyAction::Press)?;
/// ui.text_input("42")?;
/// ui.key_input(KeyCode::Enter, KeyAction::Press)?;
/// assert_eq!(count.get()?, 42);
/// # Ok::<(), pert_ui::RuntimeError>(())
/// ```
#[must_use]
pub struct InlineInteger(NumberEditor<i64>);
description_debug!(InlineInteger);

/// A one-line decimal row with a caption, a draggable editable value and a unit,
/// bound to a UI-local `Signal<f64>`.
///
/// Interaction, focus, width reservation and cancellation follow
/// [`InlineInteger`]; numeric semantics, precision and validation follow
/// [`DecimalInput`]. A [`NumberFormat`] converts between canonical values and
/// application units for display, editing and parsing without changing the
/// stored value.
///
/// ```
/// # use pert_ui_runtime as pert_ui;
/// use pert_ui::{Element, InlineDecimal, KeyAction, KeyCode, Runtime};
/// let mut ui = Runtime::new();
/// let seconds = ui.signal(0.25_f64);
/// let field = ui.mount_tree(Element::new("attack",
///     InlineDecimal::new(seconds.clone()).range(0.0, 2.0)?.step(0.05)?
///         .decimal_places(2)?.label("Attack").unit("s")))?;
/// ui.focus(&field)?;
/// ui.key_input(KeyCode::ArrowUp, KeyAction::Press)?;
/// assert!((seconds.get()? - 0.3).abs() < 1e-12);
/// # Ok::<(), pert_ui::RuntimeError>(())
/// ```
#[must_use]
///
/// [`Self::for_parameter`] also supports authoritative `Parameter<f32>` values,
/// with its documented conversion and focus-loss cancellation.
pub struct InlineDecimal(NumberEditor<f64>);
description_debug!(InlineDecimal);

trait Number: Copy + PartialEq + PartialOrd + std::fmt::Display + 'static {
    const MIN: Self;
    const MAX: Self;
    const STEP: Self;
    /// The role that refuses text this kind of number cannot parse.
    const UNPARSABLE: crate::StringRole;
    fn finite(self) -> bool;
    // Inspection only; an i64 beyond 2^53 loses precision here, which is why the
    // exact value stays available as text in the `value` property.
    fn to_f64(self) -> f64;
    fn positive(self) -> bool;
    fn fine_step(self) -> Self;
    fn parse(text: &str) -> Result<Self, NumberInputError>;
    fn format(self, precision: Option<u8>) -> String;
    fn increment(self, step: Self, up: bool, min: Self, max: Self) -> Self;
    fn drag(self, step: Self, count: i64, min: Self, max: Self) -> Self;
}
impl Number for i64 {
    const MIN: Self = Self::MIN;
    const MAX: Self = Self::MAX;
    const STEP: Self = 1;
    const UNPARSABLE: crate::StringRole = crate::StringRole::NotAWholeNumber;
    fn to_f64(self) -> f64 {
        self as f64
    }
    fn finite(self) -> bool {
        true
    }
    fn positive(self) -> bool {
        self > 0
    }
    fn fine_step(self) -> Self {
        (self / 10).max(1)
    }
    fn parse(text: &str) -> Result<Self, NumberInputError> {
        text.trim()
            .parse()
            .map_err(|_| NumberInputError::InvalidNumber)
    }
    fn format(self, _: Option<u8>) -> String {
        self.to_string()
    }
    fn drag(self, step: Self, count: i64, min: Self, max: Self) -> Self {
        (i128::from(self) + i128::from(step) * i128::from(count))
            .clamp(i128::from(min), i128::from(max)) as Self
    }
    fn increment(self, step: Self, up: bool, min: Self, max: Self) -> Self {
        if up {
            self.saturating_add(step)
        } else {
            self.saturating_sub(step)
        }
        .clamp(min, max)
    }
}
impl Number for f64 {
    const MIN: Self = -Self::MAX;
    const MAX: Self = Self::MAX;
    const STEP: Self = 0.1;
    const UNPARSABLE: crate::StringRole = crate::StringRole::NotANumber;
    fn to_f64(self) -> f64 {
        self
    }
    fn finite(self) -> bool {
        self.is_finite()
    }
    fn positive(self) -> bool {
        self > 0.0 && self.is_finite()
    }
    fn fine_step(self) -> Self {
        (self / 10.0).max(Self::from_bits(1))
    }
    fn parse(text: &str) -> Result<Self, NumberInputError> {
        let value: Self = text
            .trim()
            .replace(',', ".")
            .parse()
            .map_err(|_| NumberInputError::InvalidNumber)?;
        if value.is_finite() {
            Ok(value)
        } else {
            Err(NumberInputError::NonFinite)
        }
    }
    fn format(self, precision: Option<u8>) -> String {
        // A value shown as zero has no minus sign.
        crate::number_format::unsigned_zero(precision.map_or_else(
            || self.to_string(),
            |n| format!("{:.*}", usize::from(n), self),
        ))
    }
    fn drag(self, step: Self, count: i64, min: Self, max: Self) -> Self {
        decimal_steps(self, step, count as Self).clamp(min, max)
    }
    fn increment(self, step: Self, up: bool, min: Self, max: Self) -> Self {
        decimal_steps(self, step, if up { 1.0 } else { -1.0 }).clamp(min, max)
    }
}

/// Adds `count` steps to `value` as decimal numbers rather than binary ones.
///
/// Binary addition makes -5.7 + 0.1 equal -5.6000000000000005, and repeated
/// steps accumulate such errors into the stored value. The exact decimal sum
/// has no more fractional digits than the longer of `value` and `step` (as
/// their shortest round-trip text), so the binary sum is rounded to that many
/// digits. Values or steps needing more than
/// [`NumberInputError::MAX_PRECISION`] digits keep the binary sum.
fn decimal_steps(value: f64, step: f64, count: f64) -> f64 {
    fn fraction_digits(value: f64) -> usize {
        // `f64`'s `Display` is the shortest round-trip text and never uses an
        // exponent, so the digits after the point are the value's decimals.
        let text = value.abs().to_string();
        text.split_once('.')
            .map_or(0, |(_, fraction)| fraction.len())
    }
    let sum = value + step * count;
    let digits = fraction_digits(value).max(fraction_digits(step));
    if digits > usize::from(NumberInputError::MAX_PRECISION) || !sum.is_finite() {
        return sum;
    }
    // Adding 0.0 turns a rounded -0.0 into 0.0.
    format!("{sum:.digits$}")
        .parse::<f64>()
        .map_or(sum, |rounded| rounded + 0.0)
}

/// What an editor reads from its source during update.
struct NumberReading<T> {
    /// The authoritative value to display when no draft is pending.
    value: T,
    /// Bounds imposed by a parameter specification.
    domain: Option<(T, T)>,
    /// The step lattice a stepped parameter imposes; a continuous parameter
    /// leaves the editor's own step.
    lattice: Option<T>,
    /// Whether edits may begin; always for local signals.
    editable: bool,
    /// The parameter revision observed, the expected revision of the next edit.
    revision: Revision,
    /// A parameter's display name, used when no caption names the editor.
    name: Option<String>,
}

/// A local signal, or a parameter driven through its public edit protocol.
trait NumberSource<T> {
    /// The source as `Any`, so [`Self::same`] can compare concrete sources.
    fn as_any(&self) -> &dyn std::any::Any;
    /// Whether `other` is this very source: the same signal or parameter.
    fn same(&self, other: &dyn NumberSource<T>) -> bool;
    fn read(&self, cx: &mut UpdateContext) -> Result<NumberReading<T>, RuntimeError>;
    fn current(&self) -> Result<T, RuntimeError>;
    /// Parameters cancel drafts on focus loss and preview gestures.
    fn is_parameter(&self) -> bool {
        false
    }
    /// Whether the source's specification supplies the step, so the editor's
    /// own `step` cannot define another.
    fn supplies_step(&self) -> bool {
        self.is_parameter()
    }
    /// Whether this source still owns an unfinished preview gesture.
    fn owns_preview(&self) -> Result<bool, RuntimeError> {
        Ok(false)
    }
    /// Extra validation beyond the bounds, such as a parameter's step lattice.
    fn check(&self, _: T) -> Result<(), NumberInputError> {
        Ok(())
    }
    /// Starts a gesture session when none is active; `false` if refused.
    fn begin(&self, _: &mut EventContext, _: Revision) -> Result<bool, RuntimeError> {
        Ok(true)
    }
    /// Previews a value in the active gesture session, if any.
    fn preview(&self, _: T) -> Result<(), RuntimeError> {
        Ok(())
    }
    /// Saves a value: ends the active gesture session with it, or performs one
    /// begin/commit pair. Returns whether the value changed.
    fn commit(
        &self,
        cx: &mut EventContext,
        value: T,
        observed: Revision,
    ) -> Result<bool, RuntimeError>;
    /// Cancels the active gesture session, if any, notifying synchronously.
    fn cancel(&self) {}
    /// Drops the active gesture session outside a writable phase; the guard
    /// cancels it and the notification follows at the next safe point.
    fn abandon(&self) {}
    /// Advances a valid value by whole steps on a parameter's lattice, stopping
    /// at the bounds; `None` for local sources, which use their own step.
    fn offset(&self, _: T, _: i64) -> Option<T> {
        None
    }
}

impl<T: Number> NumberSource<T> for Signal<T> {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn same(&self, other: &dyn NumberSource<T>) -> bool {
        other
            .as_any()
            .downcast_ref::<Self>()
            .is_some_and(|other| other.read_only().same_source(&self.read_only()))
    }
    fn read(&self, cx: &mut UpdateContext) -> Result<NumberReading<T>, RuntimeError> {
        Ok(NumberReading {
            value: cx.read_paint(self)?,
            domain: None,
            lattice: None,
            editable: true,
            revision: Revision::INITIAL,
            name: None,
        })
    }
    fn current(&self) -> Result<T, RuntimeError> {
        self.get()
    }
    fn commit(&self, _: &mut EventContext, value: T, _: Revision) -> Result<bool, RuntimeError> {
        self.set_changed(value)
    }
}

/// A stepped parameter with the gesture session an editor owns.
struct IntegerParameter {
    parameter: Parameter<i64>,
    spec: RefCell<Option<ParameterSpec<i64>>>,
    session: RefCell<Option<EditSession<i64>>>,
}

fn refused<R>(result: Result<R, RuntimeError>) -> Result<Option<R>, RuntimeError> {
    match result {
        Ok(value) => Ok(Some(value)),
        Err(RuntimeError::Binding(_)) => Ok(None),
        Err(error) => Err(error),
    }
}

impl NumberSource<i64> for IntegerParameter {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn same(&self, other: &dyn NumberSource<i64>) -> bool {
        other.as_any().downcast_ref::<Self>().is_some_and(|other| {
            other
                .parameter
                .snapshot()
                .same_source(&self.parameter.snapshot())
        })
    }
    fn read(&self, cx: &mut UpdateContext) -> Result<NumberReading<i64>, RuntimeError> {
        let (snapshot, spec, editable) =
            crate::control_binding::read_parameter(&self.parameter, cx)?;
        let step = i64::try_from(spec.step().get()).unwrap_or(i64::MAX);
        let reading = NumberReading {
            value: snapshot.preview.unwrap_or(snapshot.base),
            domain: Some((spec.minimum(), spec.maximum())),
            lattice: Some(step),
            editable,
            revision: snapshot.revision,
            name: Some(spec.name().to_owned()),
        };
        *self.spec.borrow_mut() = Some(spec);
        Ok(reading)
    }
    fn current(&self) -> Result<i64, RuntimeError> {
        let snapshot = self.parameter.snapshot().get()?;
        Ok(snapshot.preview.unwrap_or(snapshot.base))
    }
    fn is_parameter(&self) -> bool {
        true
    }
    fn owns_preview(&self) -> Result<bool, RuntimeError> {
        self.session
            .borrow()
            .as_ref()
            .map_or(Ok(false), |session| Ok(session.outcome()?.is_none()))
    }
    fn check(&self, value: i64) -> Result<(), NumberInputError> {
        match self
            .spec
            .borrow()
            .as_ref()
            .map(|spec| spec.validate(&value))
        {
            Some(Err(ParameterError::OffLattice)) => Err(NumberInputError::OffStep),
            Some(Err(_)) => Err(NumberInputError::OutOfRange),
            _ => Ok(()),
        }
    }
    fn begin(&self, cx: &mut EventContext, observed: Revision) -> Result<bool, RuntimeError> {
        if self.session.borrow().is_some() {
            return Ok(true);
        }
        let session = refused(self.parameter.begin_edit(cx, observed))?;
        let begun = session.is_some();
        *self.session.borrow_mut() = session;
        Ok(begun)
    }
    fn preview(&self, value: i64) -> Result<(), RuntimeError> {
        if let Some(session) = self.session.borrow().as_ref() {
            // A refused preview (for example after a conflicting host write)
            // leaves the draft; the ended session commits nothing.
            refused(session.update(value))?;
        }
        Ok(())
    }
    fn commit(
        &self,
        cx: &mut EventContext,
        value: i64,
        observed: Revision,
    ) -> Result<bool, RuntimeError> {
        let active = self.session.borrow_mut().take();
        let unchanged = self.parameter.snapshot().get()?.base == value;
        let session = match active {
            // A gesture that returns to the base value ends without an edit.
            Some(session) if unchanged => {
                refused(session.cancel())?;
                return Ok(false);
            }
            Some(session) => session,
            None if unchanged => return Ok(false),
            None => match refused(self.parameter.begin_edit(cx, observed))? {
                Some(session) => session,
                None => return Ok(false),
            },
        };
        Ok(matches!(
            refused(session.commit(value))?,
            Some(EditOutcome::Committed { .. } | EditOutcome::Submitted { .. })
        ))
    }
    fn cancel(&self) {
        let session = self.session.borrow_mut().take();
        if let Some(session) = session {
            // Outside a writable phase the guard's drop below cancels instead,
            // and the notification follows at the next safe point.
            if session.cancel().is_err() {
                drop(session);
            }
        }
    }
    fn abandon(&self) {
        let session = self.session.borrow_mut().take();
        drop(session);
    }
    fn offset(&self, value: i64, steps: i64) -> Option<i64> {
        self.spec
            .borrow()
            .as_ref()
            .and_then(|spec| spec.offset(value, steps).ok())
    }
}

/// A continuous parameter, edited as `f64` text with the gesture session an
/// editor owns.
///
/// Values cross at `f32` resolution. Reading shows the shortest decimal that
/// round-trips the parameter's `f32`, so `0.1` reads as `0.1` rather than as
/// its exact binary expansion; saving rounds the typed `f64` to the nearest
/// `f32`, which the specification then validates.
struct DecimalParameter {
    parameter: Parameter<f32>,
    spec: RefCell<Option<ParameterSpec<f32>>>,
    session: RefCell<Option<EditSession<f32>>>,
}

/// The shortest decimal that round-trips `value`, as `f64`.
fn shown(value: f32) -> f64 {
    value.to_string().parse().unwrap_or(f64::from(value))
}

/// The nearest `f32`, or `None` beyond its finite range.
fn narrowed(value: f64) -> Option<f32> {
    let narrowed = value as f32;
    narrowed.is_finite().then_some(narrowed)
}

impl NumberSource<f64> for DecimalParameter {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn same(&self, other: &dyn NumberSource<f64>) -> bool {
        other.as_any().downcast_ref::<Self>().is_some_and(|other| {
            other
                .parameter
                .snapshot()
                .same_source(&self.parameter.snapshot())
        })
    }
    fn read(&self, cx: &mut UpdateContext) -> Result<NumberReading<f64>, RuntimeError> {
        let (snapshot, spec, editable) =
            crate::control_binding::read_parameter(&self.parameter, cx)?;
        let reading = NumberReading {
            value: shown(snapshot.preview.unwrap_or(snapshot.base)),
            domain: Some((shown(spec.minimum()), shown(spec.maximum()))),
            lattice: None,
            editable,
            revision: snapshot.revision,
            name: Some(spec.name().to_owned()),
        };
        *self.spec.borrow_mut() = Some(spec);
        Ok(reading)
    }
    fn current(&self) -> Result<f64, RuntimeError> {
        let snapshot = self.parameter.snapshot().get()?;
        Ok(shown(snapshot.preview.unwrap_or(snapshot.base)))
    }
    fn is_parameter(&self) -> bool {
        true
    }
    fn supplies_step(&self) -> bool {
        false
    }
    fn owns_preview(&self) -> Result<bool, RuntimeError> {
        self.session
            .borrow()
            .as_ref()
            .map_or(Ok(false), |session| Ok(session.outcome()?.is_none()))
    }
    fn check(&self, value: f64) -> Result<(), NumberInputError> {
        let Some(value) = narrowed(value) else {
            return Err(NumberInputError::OutOfRange);
        };
        match self
            .spec
            .borrow()
            .as_ref()
            .map(|spec| spec.validate(&value))
        {
            Some(Err(_)) => Err(NumberInputError::OutOfRange),
            _ => Ok(()),
        }
    }
    fn begin(&self, cx: &mut EventContext, observed: Revision) -> Result<bool, RuntimeError> {
        if self.session.borrow().is_some() {
            return Ok(true);
        }
        let session = refused(self.parameter.begin_edit(cx, observed))?;
        let begun = session.is_some();
        *self.session.borrow_mut() = session;
        Ok(begun)
    }
    fn preview(&self, value: f64) -> Result<(), RuntimeError> {
        if let (Some(session), Some(value)) = (self.session.borrow().as_ref(), narrowed(value)) {
            refused(session.update(value))?;
        }
        Ok(())
    }
    fn commit(
        &self,
        cx: &mut EventContext,
        value: f64,
        observed: Revision,
    ) -> Result<bool, RuntimeError> {
        let Some(value) = narrowed(value) else {
            return Ok(false);
        };
        let active = self.session.borrow_mut().take();
        let unchanged = self.parameter.snapshot().get()?.base == value;
        let session = match active {
            Some(session) if unchanged => {
                refused(session.cancel())?;
                return Ok(false);
            }
            Some(session) => session,
            None if unchanged => return Ok(false),
            None => match refused(self.parameter.begin_edit(cx, observed))? {
                Some(session) => session,
                None => return Ok(false),
            },
        };
        Ok(matches!(
            refused(session.commit(value))?,
            Some(EditOutcome::Committed { .. } | EditOutcome::Submitted { .. })
        ))
    }
    fn cancel(&self) {
        let session = self.session.borrow_mut().take();
        // Outside a writable phase the guard's drop cancels instead, and the
        // notification follows at the next safe point.
        if let Some(session) = session
            && session.cancel().is_err()
        {
            drop(session);
        }
    }
    fn abandon(&self) {
        let session = self.session.borrow_mut().take();
        drop(session);
    }
}

/// A held step button: the press saves one step immediately, and an event-phase
/// timer saves one more step per elapsed interval until the hold ends. A
/// parameter-bound editor previews those steps and commits on release.
struct Hold {
    up: bool,
}
const HOLD_DELAY: Duration = Duration::from_millis(400);
const HOLD_INTERVAL: Duration = Duration::from_millis(80);

struct NumberEditor<T> {
    source: Box<dyn NumberSource<T>>,
    field: InlineField<T>,
    drag_remainder: f64,
    saved: Option<T>,
    formatted: String,
    min: T,
    max: T,
    step: T,
    precision: Option<u8>,
    hold: Option<Hold>,
    /// A parameter lattice: Shift keeps the step.
    lattice: bool,
    revision: Revision,
    /// The key owning an inline parameter gesture until its release.
    held_key: Option<KeyCode>,
    /// Context cleanup requested by redescription, even if it is reversed before update.
    cancel_pending: bool,
    name: Option<String>,
    /// Whether the source accepts edits, as last read; a parameter may not.
    editable: bool,
    /// Why the bound value was last refused, while it is refused. The editor
    /// then keeps its previous value and accepts no edits.
    refused: Option<NumberInputError>,
    /// The application's adjustment of a changed value before validation.
    normalizer: Option<Normalizer<T>>,
    report: RejectionReport,
    saved_report: SaveReport<T>,
    /// The host's edit this field reports into ([`IntegerInput::session`]).
    session: Option<EditorAttachment>,
    /// What the commit in progress completes, reported once at its outcome.
    pending_trigger: Option<EditorTrigger>,
    notifier: Option<crate::Notifier>,
    /// A start delivery is requested and not yet made.
    start_pending: bool,
}
impl<T: Number> NumberEditor<T> {
    fn new(source: Box<dyn NumberSource<T>>, inline: bool) -> Self {
        Self {
            source,
            field: InlineField::new(inline),
            drag_remainder: 0.0,
            saved: None,
            formatted: String::new(),
            min: T::MIN,
            max: T::MAX,
            step: T::STEP,
            precision: None,
            hold: None,
            lattice: false,
            revision: Revision::INITIAL,
            held_key: None,
            cancel_pending: false,
            name: None,
            editable: true,
            refused: None,
            normalizer: None,
            report: RejectionReport::default(),
            saved_report: SaveReport::default(),
            session: None,
            pending_trigger: None,
            notifier: None,
            start_pending: false,
        }
    }
    /// The session this field reports into: only a plain field over a local
    /// signal takes part, so an inline row or a parameter ignores it.
    fn session(&self) -> Option<&EditorAttachment> {
        self.session
            .as_ref()
            .filter(|_| !self.field.inline && !self.source.is_parameter())
    }
    /// Reports the outcome of the commit in progress, once.
    fn report_session(&mut self, outcome: EditorOutcome) {
        if let Some(trigger) = self.pending_trigger.take()
            && let Some(session) = self.session()
        {
            session.report(trigger, outcome);
        }
    }
    /// Commits for a session completion and reports its outcome.
    fn complete(
        &mut self,
        cx: &mut EventContext,
        trigger: EditorTrigger,
    ) -> Result<(), RuntimeError> {
        self.pending_trigger = Some(trigger);
        let result = self.commit(cx);
        // A commit that failed before reaching its outcome reports nothing.
        self.pending_trigger = None;
        result
    }
    /// The keys a field with a session keeps: Enter and Tab confirm, Escape
    /// cancels, each reported once. While a composition or selection drag is
    /// active Enter and Tab are kept without saving and Escape goes to the
    /// text editor, which ends the composition or drag only.
    fn session_key(
        &mut self,
        cx: &mut EventContext,
        event: KeyEvent,
        control: KeyCode,
        chord: Modifiers,
    ) -> Result<Option<InputOutcome>, RuntimeError> {
        if self.session().is_none()
            || !matches!(
                event.transition,
                KeyTransition::Pressed | KeyTransition::Repeated
            )
        {
            return Ok(None);
        }
        let busy = self.field.editor.preedit.is_some() || self.field.editor.dragging;
        let pressed = event.transition == KeyTransition::Pressed;
        match control {
            KeyCode::Escape if busy => self
                .field
                .editor
                .key(
                    cx,
                    KeyEvent {
                        code: control,
                        ..event
                    },
                )
                .map(Some),
            KeyCode::Escape => {
                if pressed {
                    self.sync(self.source.current()?)?;
                    self.cancel();
                    self.field.editor.changed(cx);
                    if let Some(session) = self.session() {
                        session.report(EditorTrigger::Escape, EditorOutcome::Cancelled);
                    }
                    self.report.clear(cx)?;
                }
                Ok(Some(InputOutcome::Handled))
            }
            KeyCode::Enter => {
                if pressed && !busy {
                    self.complete(cx, EditorTrigger::Enter)?;
                }
                Ok(Some(InputOutcome::Handled))
            }
            KeyCode::Tab if !(chord.control || chord.alt || chord.super_key) => self
                .session_tab(
                    cx,
                    crate::FocusDirection::from_shift(cx.modifiers().shift),
                    event.transition,
                )
                .map(Some),
            _ => Ok(None),
        }
    }
    /// Tab with a session: confirms and reports, or acts on a composition or
    /// drag only; the key is kept either way.
    fn session_tab(
        &mut self,
        cx: &mut EventContext,
        direction: crate::FocusDirection,
        transition: crate::KeyTransition,
    ) -> Result<InputOutcome, RuntimeError> {
        if self.session().is_none() {
            return Ok(InputOutcome::Ignored);
        }
        if transition == crate::KeyTransition::Pressed
            && self.field.editor.preedit.is_none()
            && !self.field.editor.dragging
            && !self.held()
        {
            self.complete(cx, EditorTrigger::Tab { direction })?;
        }
        Ok(InputOutcome::Handled)
    }
    /// Applies the session's pending start, from the field's own delivery.
    fn apply_start(&mut self, cx: &mut EventContext) -> Result<(), RuntimeError> {
        self.start_pending = false;
        let Some(start) = self.session().and_then(EditorAttachment::take_start) else {
            return Ok(());
        };
        self.sync(self.source.current()?)?;
        let editor = &mut self.field.editor;
        match start {
            EditorStart::SelectAll => editor.command(cx, TextCommand::SelectAll).map(|_| ()),
            EditorStart::CaretAtEnd => {
                let layout = editor.layout(cx)?;
                editor
                    .selection
                    .set(TextSelection::snapped(&layout, usize::MAX, usize::MAX));
                editor.changed(cx);
                Ok(())
            }
            EditorStart::Typed(text) => {
                editor.command(cx, TextCommand::SelectAll)?;
                editor.type_start(cx, &text)
            }
        }
    }
    /// Applies the effective read-only state: the editing permission, a
    /// non-editable source, or a refused bound value.
    fn lock(&mut self) {
        self.field
            .editor
            .set_owner_lock(!self.editable || self.refused.is_some());
    }
    /// Whether a step gesture, a held key or a pressed stepper is in progress.
    fn held(&self) -> bool {
        self.held_key.is_some() || self.hold.is_some() || self.field.pressed()
    }
    fn stop_hold(&mut self) {
        self.hold = None;
        self.field.pressed_button = None;
    }
    /// One step in a gesture: local sources save it; parameters preview it in
    /// the session begun at activation.
    fn gesture_step(&mut self, cx: &mut EventContext, up: bool) -> Result<(), RuntimeError> {
        let parameter = self.source.is_parameter();
        // With a session, steps edit the draft, so Escape can restore it.
        let saves = !parameter && self.session().is_none();
        let stepped = self.increment(cx, up, saves)?;
        if parameter {
            if let Ok(value) = self.draft() {
                self.source.preview(value)?;
            }
            Ok(())
        } else if stepped && saves {
            self.commit(cx)
        } else {
            // A refused step changes neither the draft nor the binding.
            Ok(())
        }
    }
    /// Starts a hold on a step button: saves (or previews) one step now and arms
    /// the repeat.
    fn begin_hold(&mut self, cx: &mut EventContext, up: bool) -> Result<(), RuntimeError> {
        if self.source.is_parameter() && !self.source.begin(cx, self.revision)? {
            return Ok(());
        }
        cx.capture_pointer()?;
        self.gesture_step(cx, up)?;
        self.hold = Some(Hold { up });
        self.field.pressed_button = Some(up);
        cx.set_timer(HOLD_DELAY)?;
        cx.invalidate(Invalidation::Paint);
        Ok(())
    }
    /// Saves one more step for a still-held button and arms the next repeat.
    fn repeat_hold(&mut self, cx: &mut EventContext) -> Result<(), RuntimeError> {
        let Some(hold) = &self.hold else {
            return Ok(());
        };
        let up = hold.up;
        self.gesture_step(cx, up)?;
        cx.set_timer(HOLD_INTERVAL)
    }
    /// Ends a hold. Local steps are already saved; a parameter gesture commits.
    fn end_hold(&mut self, cx: &mut EventContext) -> Result<(), RuntimeError> {
        cx.release_pointer();
        cx.cancel_timer();
        self.hold = None;
        self.field.pressed_button = None;
        if self.source.is_parameter() {
            self.commit(cx)?;
        }
        self.field.error = self.draft().err();
        cx.invalidate(Invalidation::Paint);
        Ok(())
    }
    /// The user-facing message for the field's own refusal of a draft.
    fn number_rejection(
        &self,
        strings: &crate::UiStrings,
        error: NumberInputError,
    ) -> InputRejection {
        let format = |value: T| value.format(self.precision);
        InputRejection::new(match error {
            NumberInputError::OutOfRange => strings.format(crate::UiMessage::NumberOutOfRange {
                min: &format(self.min),
                max: &format(self.max),
            }),
            NumberInputError::OffStep => strings.format(crate::UiMessage::NumberOffStep {
                step: &format(self.step),
            }),
            _ => strings.text(T::UNPARSABLE).to_owned(),
        })
    }
    fn validate(&self, value: T) -> Result<T, NumberInputError> {
        if !value.finite() {
            Err(NumberInputError::NonFinite)
        } else if value < self.min || value > self.max {
            Err(NumberInputError::OutOfRange)
        } else {
            self.source.check(value)?;
            Ok(value)
        }
    }
    fn fine_step(&self, cx: &EventContext) -> T {
        if cx.modifiers().shift && !self.lattice {
            self.step.fine_step()
        } else {
            self.step
        }
    }
    /// The value a save would write: the parsed draft, normalized when it
    /// differs from the bound value, then validated.
    fn draft(&self) -> Result<T, EntryError> {
        // Resting/stepped text can round even outside a narrow range. The
        // corresponding exact value is retained in the same undo snapshot.
        let value = match self.field.payload() {
            Some(value) => value,
            None => self.field.parse(T::parse)?,
        };
        self.entry(value)
    }
    /// Normalizes a value that differs from the bound value, then validates
    /// it. The bound value itself is never normalized: the application wrote it.
    fn entry(&self, value: T) -> Result<T, EntryError> {
        let value = match &self.normalizer {
            Some(normalize) if Some(value) != self.saved => normalize(value)?,
            _ => value,
        };
        Ok(self.validate(value)?)
    }
    /// The draft's own value, validated but not normalized: the base of a step.
    fn raw_draft(&self) -> Result<T, EntryError> {
        let value = match self.field.payload() {
            Some(value) => value,
            None => self.field.parse(T::parse)?,
        };
        Ok(self.validate(value)?)
    }
    /// Whether the draft still shows the bound value unchanged.
    fn pristine(&self) -> bool {
        self.field.draft_text() == self.formatted && self.field.payload() == self.saved
    }
    fn accept(&mut self, value: T) -> Result<(), RuntimeError> {
        self.field
            .display_value(value, value.format(self.precision))?;
        let formatted = self.field.editing_text(value, value.format(self.precision));
        self.field.editor.validate(&formatted)?;
        self.saved = Some(value);
        self.formatted = formatted;
        self.field.editor.sync_text(self.formatted.clone());
        self.field.editor.payload = Some(value);
        self.field.editor.cancel_composition();
        self.field.editor.undo.clear();
        self.field.editor.redo.clear();
        Ok(())
    }
    fn sync(&mut self, value: T) -> Result<(), RuntimeError> {
        // A refused value is kept out of the editor rather than failing every
        // flush and event: the draft ends, the previous value stays and edits
        // are refused, so the field never overwrites the application's value.
        if let Err(error) = self.validate(value) {
            if self.refused.is_none() {
                self.cancel();
            }
            self.refused = Some(error);
            self.lock();
            return Ok(());
        }
        if self.refused.take().is_some() {
            self.lock();
        }
        // The source's own preview is already in the draft. Keep its baseline,
        // pointer press and fractional drag remainder until the gesture ends.
        // A terminal conflict or a different value still synchronizes normally.
        if self.field.payload() == Some(value) && self.source.owns_preview()? {
            return Ok(());
        }
        if self.saved != Some(value) {
            if self.field.inline {
                self.field.stop_text();
            }
            self.drag_remainder = 0.0;
            self.accept(value)?;
        }
        Ok(())
    }
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        if self.session.is_some() && self.notifier.is_none() {
            self.notifier = Some(cx.notifier());
        }
        if !self.start_pending
            && self.session().is_some_and(EditorAttachment::has_start)
            && let Some(notifier) = &self.notifier
        {
            self.start_pending = true;
            notifier.request();
        }
        let reading = self.source.read(cx)?;
        if let Some((min, max)) = reading.domain {
            if (min, max) != (self.min, self.max) {
                cx.request_layout();
            }
            (self.min, self.max) = (min, max);
        }
        if let Some(step) = reading.lattice {
            self.step = step;
            self.lattice = true;
        }
        self.editable = reading.editable;
        self.lock();
        // A revoked permission ends the edit as disabling by redescription does.
        if self.field.editor.read_permissions(cx)?.disabled {
            self.cancel_pending = true;
        }
        self.name = reading.name;
        // A parameter's base or metadata change discards the draft before
        // anything can commit it.
        let lost = (!self.field.editor.enabled || self.field.editor.read_only)
            && (self.held_key.is_some() || self.hold.is_some() || self.field.pressed());
        if self.cancel_pending || lost {
            self.cancel_pending = false;
            cx.cancel_input();
            self.source.abandon();
            self.reset_draft();
        }
        if reading.revision != self.revision {
            self.revision = reading.revision;
            if self.saved.is_some() && self.source.is_parameter() {
                // Writes are refused during update: drop the session instead.
                self.source.abandon();
                self.reset_draft();
            }
        }
        self.sync(reading.value)?;
        if self.field.inline {
            if let Ok(value) = self.draft() {
                self.field
                    .display_value(value, value.format(self.precision))?;
            }
            self.field.error = self.draft().err();
        } else if let Some(value) = self.saved {
            // An ordinary field shows this display text at rest.
            self.field
                .display_value(value, value.format(self.precision))?;
        }
        self.field.update(cx)
    }
    fn commit(&mut self, cx: &mut EventContext) -> Result<(), RuntimeError> {
        self.sync(self.source.current()?)?;
        let mut wrote = None;
        if self.field.editor.enabled
            && !self.field.editor.read_only
            && let Ok(value) = self.draft()
        {
            // Prepare the post-save display before publishing a value.
            let text = self.field.editing_text(value, value.format(self.precision));
            self.field.editor.validate(&text)?;
            cx.text(&text, None, self.field.editor.style)?;
            self.field
                .display_value(value, value.format(self.precision))?;
            cx.text(&self.field.display, None, self.field.editor.style)?;
            cx.text(&self.field.suffix, None, self.field.editor.style)?;
            let committed = self.source.commit(cx, value, self.revision)?;
            if committed {
                wrote = Some(value);
            }
            if committed || !self.source.is_parameter() {
                self.accept(value)?;
            } else {
                // Unchanged or refused: show the authoritative value again.
                self.accept(self.source.current()?)?;
            }
            if self.field.inline {
                self.field.stop_text();
            }
        }
        if self.draft().is_err() {
            // An invalid draft cannot end a gesture; never leave its session open.
            self.source.cancel();
        }
        self.field.error = self.draft().err();
        self.field.editor.changed(cx);
        let error = self.field.error.clone();
        // Reported before the application's callbacks run, so a failing
        // callback cannot lose the completion. A read-only field keeps a
        // changed draft unsaved; a field refused since mount has no draft to
        // correct, although its empty text does not parse.
        let outcome = if wrote.is_some() {
            EditorOutcome::Saved
        } else if self.dirty() {
            EditorOutcome::Refused
        } else {
            EditorOutcome::Unchanged
        };
        self.report_session(outcome);
        // The field's own refusal of a changed draft is reported like the
        // normalizer's, with the installed strings' message; a draft that
        // was never changed refuses nothing.
        let error = match error {
            Some(EntryError::Number(error)) => (outcome == EditorOutcome::Refused)
                .then(|| EntryError::Rejected(self.number_rejection(&cx.strings(), error))),
            other => other,
        };
        // Both notifications are attempted, so a failing rejection report
        // cannot swallow the save; the first error is returned.
        let reported = self.report.outcome(cx, error.as_ref());
        let saved = match wrote {
            Some(value) => self.saved_report.saved(cx, &value),
            None => Ok(()),
        };
        reported.and(saved)
    }
    fn cancel(&mut self) {
        self.source.cancel();
        self.reset_draft();
    }
    /// Applies a new description of the same editor (`Component::reconcile`):
    /// its source, bounds, step, precision, presentation, permissions,
    /// normalizer and callbacks.
    ///
    /// A different source ends the edit in progress unsaved: a parameter's
    /// gesture session is dropped, which cancels it and publishes the
    /// parameter's snapshot at once, the lifecycle exception the contracts name
    /// for reconciliation, while its terminal event follows at the next safe
    /// point; a typed draft and a held step end, and the next update shows the
    /// new source's value. Disabling ends an edit the same way. A parameter source keeps the bounds and
    /// step of its specification.
    fn adopt(&mut self, next: Self) -> Option<Invalidation> {
        let mut invalidation = None;
        // Presentation changes re-format a resting value; an edit in progress
        // keeps its draft and is validated against the new settings on save.
        let resting = !(self.field.editing
            || self.hold.is_some()
            || self.held_key.is_some()
            || self.field.editor.text != self.formatted);
        let rebound = !self.source.same(next.source.as_ref());
        if rebound {
            self.source.abandon();
            self.reset_draft();
            self.source = next.source;
            self.saved = None;
            self.refused = None;
            self.revision = Revision::INITIAL;
            self.lattice = false;
            self.editable = true;
            self.name = None;
            invalidation = Some(Invalidation::Layout);
        }
        if !self.source.is_parameter()
            && (next.min, next.max, next.step) != (self.min, self.max, self.step)
        {
            (self.min, self.max, self.step) = (next.min, next.max, next.step);
            // The current value may fall outside the new range; the next
            // update refuses it then.
            if resting {
                self.saved = None;
            }
            invalidation = Some(Invalidation::Layout);
        }
        if next.precision != self.precision {
            self.precision = next.precision;
            if resting {
                self.saved = None;
            }
            invalidation = Some(Invalidation::Layout);
        }
        let held = self.held();
        let availability = self
            .field
            .editor
            .adopt_permissions(next.field.editor.enablement, next.field.editor.editability);
        if availability.disabled || (availability.locked && held) {
            self.source.abandon();
            self.reset_draft();
            self.cancel_pending = true;
        }
        if availability.changed {
            self.lock();
            invalidation = invalidation.max(Some(Invalidation::Paint));
        }
        let caption_changed = self.field.caption.reconcile(next.field.caption);
        if caption_changed
            || next.field.unit != self.field.unit
            || next.field.stepper != self.field.stepper
            || next.field.editor.style != self.field.editor.style
            || !NumberFormat::same_optional(&next.field.format, &self.field.format)
        {
            // Cloned formats share identity; new callbacks or metadata remeasure.
            self.field.unit = next.field.unit;
            self.field.stepper = next.field.stepper;
            self.field.format = next.field.format;
            self.field.set_style(next.field.editor.style);
            if resting {
                self.saved = None;
            }
            invalidation = Some(Invalidation::Layout);
        }
        self.normalizer = next.normalizer;
        if next
            .session
            .as_ref()
            .is_some_and(EditorAttachment::has_start)
        {
            // Update schedules the new edit's start.
            invalidation = invalidation.or(Some(Invalidation::Paint));
        }
        self.session = next.session;
        self.report.adopt(next.report);
        self.saved_report.adopt(next.saved_report);
        invalidation
    }
    /// Whether the draft differs from the saved value or its presentation.
    fn dirty(&self) -> bool {
        self.field.draft_text() != self.formatted || self.draft().ok() != self.saved
    }
    fn reset_draft(&mut self) {
        self.held_key = None;
        self.field.editor.sync_text(self.formatted.clone());
        self.field.editor.payload = self.saved;
        self.field.editor.undo.clear();
        self.field.editor.redo.clear();
        self.field.stop_text();
        self.drag_remainder = 0.0;
        self.stop_hold();
    }
    fn text_key(
        &mut self,
        cx: &mut EventContext,
        input: CommandKeyInput,
        transition: KeyTransition,
        keys: &KeyboardState,
    ) -> Result<InputOutcome, RuntimeError> {
        let editing = input.editing_event(transition);
        let event = KeyEvent {
            code: input.physical,
            ..editing
        };
        self.key(cx, event, editing.code, keys.chord_modifiers(event))
    }
    /// `chord` is the event's admission modifiers,
    /// [`KeyboardState::chord_modifiers`].
    fn key(
        &mut self,
        cx: &mut EventContext,
        event: KeyEvent,
        control: KeyCode,
        chord: Modifiers,
    ) -> Result<InputOutcome, RuntimeError> {
        self.sync(self.source.current()?)?;
        if !self.field.inline && self.hold.is_some() {
            // A held step button owns the field until release; Escape ends the
            // repeat. Local steps are already saved; parameter previews cancel.
            if control == KeyCode::Escape && event.transition == KeyTransition::Pressed {
                cx.cancel_timer();
                cx.release_pointer();
                if self.source.is_parameter() {
                    self.cancel();
                } else {
                    self.stop_hold();
                }
                cx.invalidate(Invalidation::Paint);
            }
            return Ok(InputOutcome::Handled);
        }
        if self.field.inline && !self.field.editing {
            if control == KeyCode::Escape {
                self.cancel();
                cx.cancel_timer();
                cx.release_pointer();
                cx.invalidate(Invalidation::Paint);
                self.report.clear(cx)?;
                return Ok(InputOutcome::Handled);
            }
            if self.field.pressed() || self.hold.is_some() {
                return Ok(InputOutcome::Handled);
            }
            if !self.field.editor.enabled {
                return Ok(InputOutcome::Ignored);
            }
            if event.transition == KeyTransition::Pressed
                && matches!(control, KeyCode::Enter | KeyCode::F2)
            {
                // Only a field refused since mount has no valid value to show.
                let Ok(value) = self.draft() else {
                    return Ok(InputOutcome::Ignored);
                };
                self.field
                    .begin_text(cx, value, value.format(self.precision))?;
                return Ok(InputOutcome::Handled);
            }
            // A matched release ends the held session; update cancels revoked permission.
            if event.transition == KeyTransition::Released && self.held_key == Some(event.code) {
                self.held_key = None;
                if self.field.editor.read_only {
                    self.cancel();
                } else {
                    self.commit(cx)?;
                }
                return Ok(InputOutcome::Handled);
            }
            if self.field.editor.read_only {
                return Ok(InputOutcome::Ignored);
            }
            let arrow = matches!(
                control,
                KeyCode::ArrowUp | KeyCode::ArrowDown | KeyCode::ArrowLeft | KeyCode::ArrowRight
            );
            let up = matches!(control, KeyCode::ArrowUp | KeyCode::ArrowRight);
            // Control, Alt and Super chords are left to command bindings; the
            // release of an owned key still ends its session above.
            if arrow
                && event.transition != KeyTransition::Released
                && self.held_key.is_none()
                && !crate::activation::steps_with(chord)
            {
                return Ok(InputOutcome::Ignored);
            }
            if arrow && self.source.is_parameter() {
                // One session per held key: press begins, repeats preview,
                // the matched release commits.
                match event.transition {
                    KeyTransition::Pressed if self.held_key.is_none() => {
                        if self.source.begin(cx, self.revision)? {
                            self.held_key = Some(event.code);
                            self.gesture_step(cx, up)?;
                        }
                    }
                    KeyTransition::Repeated if self.held_key == Some(event.code) => {
                        self.gesture_step(cx, up)?;
                    }
                    _ => {}
                }
                return Ok(InputOutcome::Handled);
            }
            if matches!(
                event.transition,
                KeyTransition::Pressed | KeyTransition::Repeated
            ) && arrow
            {
                if self.increment(cx, up, true)? {
                    self.commit(cx)?;
                }
                return Ok(InputOutcome::Handled);
            }
            return Ok(InputOutcome::Ignored);
        }
        if self.field.drafting() {
            return self.drafted_key(
                cx,
                chord,
                KeyEvent {
                    code: control,
                    ..event
                },
            );
        }
        if self.field.editor.enabled
            && let Some(outcome) = self.session_key(cx, event, control, chord)?
        {
            return Ok(outcome);
        }
        if !self.field.editor.enabled
            || self.field.editor.read_only
            || !matches!(
                event.transition,
                KeyTransition::Pressed | KeyTransition::Repeated
            )
            || self.field.editor.preedit.is_some()
        {
            return self.field.editor.key(
                cx,
                KeyEvent {
                    code: control,
                    ..event
                },
            );
        }
        match control {
            KeyCode::Enter => {
                if event.transition == KeyTransition::Pressed {
                    self.commit(cx)?;
                }
                Ok(InputOutcome::Handled)
            }
            KeyCode::Escape => {
                self.sync(self.source.current()?)?;
                if !self.dirty() && !self.field.editor.dragging && !self.source.owns_preview()? {
                    // Nothing to cancel: as in a text field, a refusal whose
                    // draft an application write discarded ends here and
                    // Escape bubbles to the owner. A selection drag is
                    // cancelled first.
                    self.report.clear(cx)?;
                    return Ok(InputOutcome::Ignored);
                }
                self.cancel();
                self.field.editor.changed(cx);
                self.report.clear(cx)?;
                Ok(InputOutcome::Handled)
            }
            KeyCode::ArrowUp | KeyCode::ArrowDown if crate::activation::steps_with(chord) => {
                self.increment(cx, control == KeyCode::ArrowUp, false)?;
                Ok(InputOutcome::Handled)
            }
            _ => self.field.editor.key(
                cx,
                KeyEvent {
                    code: control,
                    ..event
                },
            ),
        }
    }
    /// The keys an inline row's child editor leaves to it in text mode: Enter
    /// saves, Escape restores, Up and Down step the draft. Focus returns to the
    /// row when text mode ends by key.
    fn drafted_key(
        &mut self,
        cx: &mut EventContext,
        chord: Modifiers,
        event: KeyEvent,
    ) -> Result<InputOutcome, RuntimeError> {
        if !matches!(
            event.transition,
            KeyTransition::Pressed | KeyTransition::Repeated
        ) {
            return Ok(InputOutcome::Ignored);
        }
        match event.code {
            KeyCode::Enter => {
                if event.transition == KeyTransition::Pressed {
                    self.commit(cx)?;
                    if !self.field.editing {
                        cx.request_focus();
                    }
                }
                Ok(InputOutcome::Handled)
            }
            KeyCode::Escape => {
                self.sync(self.source.current()?)?;
                self.cancel();
                cx.request_focus();
                cx.invalidate(Invalidation::Paint);
                self.report.clear(cx)?;
                Ok(InputOutcome::Handled)
            }
            KeyCode::ArrowUp | KeyCode::ArrowDown if crate::activation::steps_with(chord) => {
                if self.field.editor.enabled && !self.field.editor.read_only {
                    self.increment(cx, event.code == KeyCode::ArrowUp, false)?;
                }
                Ok(InputOutcome::Handled)
            }
            _ => Ok(InputOutcome::Ignored),
        }
    }
    /// A clipboard command on the draft, with the chords' semantics. A resting
    /// inline row is not an editor, since it holds no draft to act on, except
    /// for [`TextCommand::BeginEdit`], which it answers as Enter and F2.
    fn command(
        &mut self,
        cx: &mut EventContext,
        command: TextCommand,
    ) -> Result<TextCommandOutcome, RuntimeError> {
        if command == TextCommand::BeginEdit {
            return self.begin_command(cx);
        }
        if self.field.inline && !self.field.editing {
            return Ok(TextCommandOutcome::NotAnEditor);
        }
        self.sync(self.source.current()?)?;
        if self.field.inline && !self.field.editing {
            // An external value arriving first ended text mode.
            return Ok(TextCommandOutcome::NotAnEditor);
        }
        if self.field.drafting() {
            // The row's editor answers; the runtime passes the command on.
            return Ok(TextCommandOutcome::NotAnEditor);
        }
        self.field.editor.command(cx, command)
    }
    /// [`TextCommand::BeginEdit`]: a resting inline row enters text mode as
    /// Enter and F2 do, read-only text mode included, but not during a step
    /// gesture, which keeps its own lifetime. A field that is always editing,
    /// and a row already in text mode, have nothing to begin.
    fn begin_command(&mut self, cx: &mut EventContext) -> Result<TextCommandOutcome, RuntimeError> {
        self.sync(self.source.current()?)?;
        if !self.field.inline || self.field.editing {
            return Ok(TextCommandOutcome::Unavailable);
        }
        // Narrower than the key path, which refuses only a drag and a pressed
        // stepper: text mode must not start under a held step key's preview.
        if self.held() || !self.field.editor.enabled {
            return Ok(TextCommandOutcome::Unavailable);
        }
        // Only a field refused since mount has no valid value to show.
        let Ok(value) = self.draft() else {
            return Ok(TextCommandOutcome::Unavailable);
        };
        self.field
            .begin_text(cx, value, value.format(self.precision))?;
        Ok(TextCommandOutcome::Handled)
    }
    /// Steps the draft from its own value. A step into the draft is
    /// normalized when the draft is saved; an `immediate` step is saved next,
    /// so one the normalizer or validation refuses is not taken. Returns
    /// whether the step was taken.
    fn increment(
        &mut self,
        cx: &mut EventContext,
        up: bool,
        immediate: bool,
    ) -> Result<bool, RuntimeError> {
        let Ok(value) = self.raw_draft() else {
            return Ok(false);
        };
        let step = self.fine_step(cx);
        let value = self
            .source
            .offset(value, if up { 1 } else { -1 })
            .unwrap_or_else(|| value.increment(step, up, self.min, self.max));
        if immediate && let Err(error) = self.entry(value) {
            // A refused step is a refused save, reported as one: the
            // normalizer's refusal, or the field's own of what it returned.
            let error = match error {
                EntryError::Number(error) => {
                    EntryError::Rejected(self.number_rejection(&cx.strings(), error))
                }
                other => other,
            };
            self.report.outcome(cx, Some(&error))?;
            return Ok(false);
        }
        self.field
            .replace_value(cx, value, value.format(self.precision))?;
        Ok(true)
    }
    fn pointer(
        &mut self,
        cx: &mut EventContext,
        event: PointerEvent,
    ) -> Result<InputOutcome, RuntimeError> {
        self.sync(self.source.current()?)?;
        if (self.field.inline || self.field.stepper)
            && matches!(event.kind(), PointerKind::Move | PointerKind::Leave)
        {
            let hovered = if event.kind() == PointerKind::Move && !event.is_outside() {
                self.field.hit_button(cx, event.position())
            } else {
                None
            };
            self.field.set_button_hover(cx, hovered);
        }
        if self.field.inline
            && !self.field.editing
            && self.field.editor.enabled
            && self.field.editor.read_only
            && self.field.hit_value(cx, event.position())
            && event.kind() == PointerKind::Press(PointerButton::Primary)
        {
            let Ok(value) = self.draft() else {
                return Ok(InputOutcome::Ignored);
            };
            self.field
                .begin_text(cx, value, value.format(self.precision))?;
            return Ok(InputOutcome::Handled);
        }
        if self.field.inline
            && !self.field.editing
            && self.field.editor.enabled
            && !self.field.editor.read_only
        {
            if self.hold.is_some() {
                if event.kind() == PointerKind::Release(PointerButton::Primary) {
                    self.end_hold(cx)?;
                }
                return Ok(InputOutcome::Handled);
            }
            if event.kind() == PointerKind::Press(PointerButton::Primary)
                && let Some(up) = self.field.hit_button(cx, event.position())
            {
                self.begin_hold(cx, up)?;
                return Ok(InputOutcome::Handled);
            }
            match event.kind() {
                PointerKind::Press(PointerButton::Primary)
                    if self.field.hit_value(cx, event.position()) =>
                {
                    self.drag_remainder = 0.0;
                    self.field.press(cx, event.position())?;
                    return Ok(InputOutcome::Handled);
                }
                PointerKind::Move if self.field.pressed() => {
                    self.drag_value(cx, event.position())?;
                    return Ok(InputOutcome::Handled);
                }
                PointerKind::Release(PointerButton::Primary) if self.field.pressed() => {
                    if !event.is_outside() {
                        self.drag_value(cx, event.position())?;
                    }
                    if self.field.release(cx) && self.field.hit_value(cx, event.position()) {
                        if let Ok(value) = self.draft() {
                            self.field
                                .begin_text(cx, value, value.format(self.precision))?;
                        }
                    } else {
                        self.commit(cx)?;
                    }
                    return Ok(InputOutcome::Handled);
                }
                _ => {}
            }
        }
        if !self.field.inline
            && self.field.stepper
            && self.field.editor.enabled
            && !self.field.editor.read_only
        {
            if self.hold.is_some() {
                if event.kind() == PointerKind::Release(PointerButton::Primary) {
                    self.end_hold(cx)?;
                }
                return Ok(InputOutcome::Handled);
            }
            if event.kind() == PointerKind::Press(PointerButton::Primary)
                && let Some(up) = self.field.hit_button(cx, event.position())
            {
                self.begin_hold(cx, up)?;
                return Ok(InputOutcome::Handled);
            }
        }
        if let PointerKind::Scroll(delta) = event.kind()
            && delta.y() != 0.0
            && (cx.focused() || self.field.drafting() && self.field.focused)
            && self.field.editor.enabled
            && !self.field.editor.read_only
        {
            // One configured step per vertical wheel event. Do not interrupt a
            // composition or selection drag, or redirect the scroll to a parent.
            if self.field.can_step_text() && !self.field.pressed() && self.hold.is_none() {
                let resting = self.field.inline && !self.field.editing;
                // A resting wheel event is one complete edit.
                if (!resting
                    || !self.source.is_parameter()
                    || self.source.begin(cx, self.revision)?)
                    && self.increment(cx, delta.y() < 0.0, resting)?
                    && resting
                {
                    self.commit(cx)?;
                }
            }
            return Ok(InputOutcome::Handled);
        }
        if self.field.inline && !self.field.editing {
            return Ok(InputOutcome::Ignored);
        }
        if self.field.drafting() {
            return self.field.text_pointer(cx, event);
        }
        self.field.editor.pointer(cx, event)
    }
    fn drag_value(&mut self, cx: &mut EventContext, point: Point) -> Result<(), RuntimeError> {
        if let Some(delta) = self.field.drag_delta(point) {
            self.drag_remainder += f64::from(delta) / 4.0;
            let count = self.drag_remainder.trunc() as i64;
            self.drag_remainder -= count as f64;
            if count != 0
                && let Ok(value) = self.raw_draft()
            {
                if self.source.is_parameter() && !self.source.begin(cx, self.revision)? {
                    return Ok(());
                }
                let step = self.fine_step(cx);
                let value = self
                    .source
                    .offset(value, count)
                    .unwrap_or_else(|| value.drag(step, count, self.min, self.max));
                self.field.editor.replace_with_payload(
                    cx,
                    0..self.field.editor.text.len(),
                    &self.field.editing_text(value, value.format(self.precision)),
                    true,
                    Some(value),
                )?;
                self.field
                    .display_value(value, value.format(self.precision))?;
                self.source.preview(value)?;
            }
        }
        Ok(())
    }
    fn width_hint(&self) -> String {
        let a = self.min.format(self.precision);
        let b = self.max.format(self.precision);
        // Full-range inputs use a practical fixed slot and clip longer values.
        let fallback = if a.len().max(b.len()) > 16 {
            "00000000".into()
        } else {
            "0".repeat(a.len().max(b.len()))
        };
        self.field.hint(&fallback)
    }
    /// Whether an ordinary field shows its display text: unfocused, holding
    /// the unchanged bound value, with a format or unit to show.
    fn resting(&self) -> bool {
        !self.field.inline
            && !self.field.focused
            && (self.field.format.is_some() || !self.field.suffix.is_empty())
            && self.saved.is_some()
            && self.pristine()
    }
    fn paint(&self, p: &mut Painter) -> Result<(), DrawError> {
        if self.field.inline {
            self.field.paint(p)?;
        } else if self.resting() {
            self.field.paint_rest(p)?;
        } else {
            self.field.editor.paint(p)?;
            self.field.paint_buttons(p)?;
        }
        if (self.draft().is_err() || self.refused.is_some())
            && let Some(size) = p.size()
        {
            p.stroke_shape(
                Rect::from_size(size),
                rounded(p.metric(MetricRole::CornerRadius))?,
                Length::new(p.metric(MetricRole::FocusWidth))?,
                p.color(ColorRole::Error),
            )?;
        }
        Ok(())
    }
    fn inspect(&self) -> ComponentDetails {
        let mut details = self.field.editor.inspect();
        if self.field.drafting() {
            // The resting store is idle while the child editor holds the draft;
            // its selection and composition are reported by that editor.
            details
                .properties
                .insert("text".into(), InspectValue::Text(self.field.draft_text()));
            details.properties.remove("selection");
            details.properties.remove("composing");
        }
        self.field.inspect(&mut details);
        if !self.field.inline {
            // An ordinary field at rest paints its display text instead.
            details
                .properties
                .insert("shows_display".into(), InspectValue::Bool(self.resting()));
        }
        let error = self.draft().err();
        details
            .properties
            .insert("valid".into(), InspectValue::Bool(error.is_none()));
        details
            .properties
            .insert("dirty".into(), InspectValue::Bool(self.dirty()));
        if let Some(value) = self.saved {
            // String preserves exact i64 identities and full decimal precision.
            details
                .properties
                .insert("value".into(), InspectValue::Text(value.to_string()));
        }
        let invalid = error.is_some() || self.refused.is_some();
        if let Some(error) = error {
            details.properties.insert(
                "validation_error".into(),
                InspectValue::Text(error.to_string()),
            );
        }
        if let Some(error) = self.refused {
            details.properties.insert(
                "binding_error".into(),
                InspectValue::Text(error.to_string()),
            );
        }
        let editable = self.field.editor.enabled && !self.field.editor.read_only;
        let name = if self.field.label().is_empty() {
            self.name.as_deref().unwrap_or_default()
        } else {
            self.field.label()
        };
        details.semantics = Some(
            Semantics::new(SemanticRole::NumberInput)
                .name(name)
                .value(match self.saved {
                    Some(value) => SemanticValue::Number {
                        value: value.to_f64(),
                        minimum: Some(self.min.to_f64()),
                        maximum: Some(self.max.to_f64()),
                    },
                    None => SemanticValue::Text(self.field.draft_text()),
                })
                .invalid(invalid)
                .read_only(self.field.editor.read_only)
                .disabled(!self.field.editor.enabled)
                .enabled_actions(
                    editable
                        .then_some([
                            SemanticAction::SetValue,
                            SemanticAction::Increment,
                            SemanticAction::Decrement,
                        ])
                        .into_iter()
                        .flatten(),
                ),
        );
        details
    }
}

macro_rules! number_shared {
    ($name:ident, $ty:ty) => {
        impl $name {
            /// Sets inclusive bounds; equal bounds describe one allowed value.
            ///
            /// # Errors
            /// Returns `InvalidRange` for reversed or non-finite bounds, and
            /// `ParameterDomain` for a parameter-bound editor. The bound value is
            /// checked on update; typed input is rejected, never clamped.
            pub fn range(mut self, min: $ty, max: $ty) -> Result<Self, NumberInputError> {
                if self.0.source.is_parameter() {
                    return Err(NumberInputError::ParameterDomain);
                }
                if !min.finite() || !max.finite() || min > max {
                    return Err(NumberInputError::InvalidRange);
                }
                self.0.min = min;
                self.0.max = max;
                Ok(self)
            }
            /// Sets the Up/Down draft increment in canonical numeric units.
            /// This is an increment, not a grid or a restriction on typed values.
            /// A decimal step is added as a decimal number: the sum keeps the
            /// fractional digits of the value or the step, whichever has more,
            /// so -5.7 stepped up by 0.1 is exactly -5.6 rather than a binary
            /// approximation. A value or step with more than
            /// [`NumberInputError::MAX_PRECISION`] fractional digits is added in
            /// binary.
            ///
            /// # Errors
            /// Returns `InvalidStep` unless the step is strictly positive and
            /// finite, and `ParameterDomain` for a parameter-bound editor.
            pub fn step(mut self, step: $ty) -> Result<Self, NumberInputError> {
                if self.0.source.supplies_step() {
                    return Err(NumberInputError::ParameterDomain);
                }
                if !step.positive() {
                    return Err(NumberInputError::InvalidStep);
                }
                self.0.step = step;
                Ok(self)
            }
            /// Overrides the value and unit typography, including text mode.
            /// The inline default is [`TextStyle::Monospace`]; the ordinary
            /// field default is [`TextStyle::Body`]. Inline captions keep
            /// [`TextStyle::Caption`].
            pub fn style(mut self, style: TextStyle) -> Self {
                self.0.field.set_style(style);
                self
            }
            /// Allows selection/copy but disables edits, stepping and native IME.
            /// Equivalent to `editable(false)`; a later permission cannot lift it.
            pub fn read_only(self) -> Self {
                self.editable(false)
            }
            /// Restricts edits and stepping with a fixed or reactive permission;
            /// editable by default. Repeated calls combine restrictions; every
            /// condition must allow, and same-key redescription replaces the
            /// permission with the new description's. A parameter's own
            /// editability remains an additional restriction.
            ///
            /// While editing is denied the field keeps focus, selection and
            /// copying. Revoking it, observed at the next update, cancels a
            /// composition, a held step, a stepper press and a drag; a
            /// parameter gesture in progress is cancelled without committing.
            /// A typed draft stays but is not saved while editing is denied,
            /// also not on focus loss. Allowing editing again replays nothing.
            /// A binding from another runtime fails the update with
            /// [`RuntimeError::ForeignRuntime`].
            pub fn editable(mut self, permission: impl Into<Permission>) -> Self {
                self.0.field.editor.restrict_editing(permission.into());
                self
            }
            /// Prevents interaction and automatic focus; still follows the binding.
            /// Equivalent to `enabled(false)`; a later permission cannot lift it.
            pub fn disabled(self) -> Self {
                self.enabled(false)
            }
            /// Restricts focus and input with a fixed or reactive permission;
            /// enabled by default. Repeated calls combine restrictions; every
            /// condition must allow, and same-key redescription replaces the
            /// permission with the new description's. A disabled field still
            /// follows its binding.
            ///
            /// Revoking it, observed at the next update, ends the edit in
            /// progress as disabling by redescription does: the field becomes
            /// unfocusable and releases focus, a composition, held step, stepper
            /// press or drag is cancelled, a parameter gesture is cancelled
            /// without committing, and a typed draft is discarded unsaved; the
            /// field shows the bound value again. Enabling it again replays
            /// nothing. A binding from another runtime fails the update with
            /// [`RuntimeError::ForeignRuntime`].
            pub fn enabled(mut self, permission: impl Into<Permission>) -> Self {
                self.0.field.editor.restrict_enablement(permission.into());
                self
            }
            /// Sets the literal trailing unit without converting values, empty
            /// by default. A [`NumberFormat`] with a dynamic unit overrides it.
            /// An inline row always shows it after the value. An ordinary field
            /// shows it after the display text while it rests (see
            /// [`IntegerInput`'s formatting](IntegerInput#formatting)); the
            /// editing text is the number alone.
            pub fn unit(mut self, unit: impl Into<String>) -> Self {
                self.0.field.unit = unit.into();
                self
            }
            /// Installs typed display/edit/parse callbacks and a width example.
            /// Formatting is presentation only; parsed values must satisfy the
            /// range and are never clamped. An inline row shows the display
            /// text outside text mode; an ordinary field shows it while it rests
            /// and the edit text while it is focused or holds a changed draft.
            /// The width example sizes only an inline row's value slot.
            pub fn format(mut self, format: NumberFormat<$ty>) -> Self {
                self.0.field.format = Some(format);
                self
            }
            /// Adds minus and plus buttons at the trailing edge, within the
            /// control's single Tab stop. A press applies one step of
            /// [`Self::step`], fine with Shift, to the draft's value. Local
            /// bindings save that value immediately, including a typed draft.
            /// Holding applies another step after 400 ms and every 80 ms through
            /// an event-phase timer. Escape, focus loss, hiding and removal stop
            /// local repeats while keeping steps already saved. Parameter-bound
            /// steps preview until matched release; Escape or context loss cancels
            /// the entire preview session. Steps never pass the bounds.
            /// A [normalizer](Self::normalize) adjusts each local step as it
            /// is saved, and a step it refuses changes neither the draft nor
            /// the binding; a step that normalizes back to the current value,
            /// such as a step of 0.01 rounded to tenths, therefore does
            /// nothing. Default is off.
            pub fn stepper(mut self, enabled: bool) -> Self {
                self.0.field.stepper = enabled;
                self
            }
            /// Adjusts a changed value before it is validated and saved, such as
            /// rounding it to tenths, or refuses it with an [`InputRejection`].
            ///
            /// Timing class: [in the input](https://github.com/pertyjons/pert-ui/blob/main/docs/contracts.md#in-the-input), at the save it checks.
            ///
            /// The order is parse, normalize, validate (finite, within the
            /// range), prepare the display and write; a failure at any stage
            /// writes nothing, and a normalized value outside the range is an
            /// error, never clamped. The draft shows the refusal live with the
            /// error outline, reported as `validation_error` in inspection, and
            /// stays for correction. Steps are user input. A step that saves at
            /// once (a step button, or an inline row's resting arrow key or
            /// wheel) is normalized as it is saved, and a step the normalizer
            /// refuses changes nothing. A step into a draft (Up/Down, the wheel
            /// or a drag while editing) moves the draft from its own value and
            /// is normalized when the draft is saved. The bound value is never
            /// normalized, whether the application wrote it or the draft is
            /// unchanged, so focus loss never rewrites the application's value.
            ///
            /// The function must be deterministic, bounded and free of side
            /// effects, binding writes and I/O, and idempotent: normalizing its
            /// own output returns that output unchanged. Installing another
            /// normalizer replaces this one.
            ///
            /// # Errors
            /// Returns `ParameterDomain` for a parameter-bound editor: the
            /// parameter's specification defines its values, so that every
            /// control on the parameter lands on the same ones.
            pub fn normalize(
                mut self,
                normalize: impl Fn($ty) -> Result<$ty, InputRejection> + 'static,
            ) -> Result<Self, NumberInputError> {
                if self.0.source.is_parameter() {
                    return Err(NumberInputError::ParameterDomain);
                }
                self.0.normalizer = Some(Rc::new(normalize));
                Ok(self)
            }
            /// Tells the application about each refused save, with the rules
            /// of [`TextField::on_rejection`]: `Some` on each save of a
            /// changed draft that is refused, and `None` once when a reported
            /// refusal ends at a later save or at Escape.
            ///
            /// Timing class: [in the input](https://github.com/pertyjons/pert-ui/blob/main/docs/contracts.md#in-the-input).
            ///
            /// A refusal is the normalizer's [`InputRejection`] or the field's
            /// own, with a message from the installed
            /// [`UiStrings`](crate::UiStrings): text that does not parse
            /// ([`StringRole::NotAWholeNumber`](crate::StringRole::NotAWholeNumber)
            /// for the integer editors,
            /// [`StringRole::NotANumber`](crate::StringRole::NotANumber) for
            /// the decimal ones), a number outside the bounds
            /// ([`StringRole::NumberOutOfRange`](crate::StringRole::NumberOutOfRange))
            /// and, for a parameter, a number off its step
            /// ([`StringRole::NumberOffStep`](crate::StringRole::NumberOffStep)).
            /// Inspection's `validation_error` keeps the developer-facing
            /// [`NumberInputError`] text. A draft the user never changed
            /// refuses nothing. The draft shows a refusal live, but the
            /// callback hears of it only when a save is attempted. A step
            /// that saves at once and is refused (a step button, including
            /// each held repeat, or an inline row's resting arrow or wheel) is
            /// a refused save and is reported.
            pub fn on_rejection(
                mut self,
                report: impl FnMut(
                    &mut EventContext,
                    Option<&InputRejection>,
                ) -> Result<(), RuntimeError>
                + 'static,
            ) -> Self {
                self.0.report.set(Box::new(report));
                self
            }
            /// Tells the application about each save that changed the bound
            /// value, with the saved value: Enter, focus loss, a step button
            /// (each held repeat), or an inline row's step, drag or wheel, after
            /// normalization and validation. For a parameter binding it runs
            /// after a local commit or a remote submission. A remote
            /// proposal stays visible with editing disabled until the authority
            /// responds; submission does not change the base or prove acceptance.
            /// The callback reports the proposal once, not later corrections or
            /// rejection. Its failure cannot undo a commit or retained submission.
            /// It runs on the UI thread in the event phase; it does not run for
            /// a refused or unchanged save, Escape, application writes, hiding
            /// or removal. It may write bindings. Its error propagates from
            /// the input call that saved, except for a held step button's
            /// later repeats, which save from a timer and return the error
            /// from the [`Runtime::flush`] that dispatched it. A failing
            /// rejection report does not suppress it. Replaces an earlier
            /// callback.
            ///
            /// Timing class: [in the input](https://github.com/pertyjons/pert-ui/blob/main/docs/contracts.md#in-the-input).
            pub fn on_save(
                mut self,
                saved: impl FnMut(&mut EventContext, $ty) -> Result<(), RuntimeError> + 'static,
            ) -> Self {
                let mut saved = saved;
                self.0
                    .saved_report
                    .set(Box::new(move |cx, value: &$ty| saved(cx, *value)));
                self
            }
        }
        impl Component for $name {
            /// Applies the new description; a different source ends the edit
            /// in progress unsaved.
            fn reconcile(&mut self, next: Self) -> Option<Invalidation> {
                self.0.adopt(next.0)
            }
            fn focusable(&self) -> bool {
                // In text mode focus belongs to the child editor.
                self.0.field.editor.enabled && !self.0.field.drafting()
            }
            fn child_visible(&self, key: &str) -> bool {
                self.0.field.child_visible(key)
            }
            fn cursor(&self, point: Point) -> Option<CursorIcon> {
                if self.0.field.drafting() {
                    return None;
                }
                if self.0.field.inline && !self.0.field.editing {
                    return (self.0.field.editor.enabled && self.0.field.local_value(point))
                        .then_some(if self.0.field.editor.read_only {
                            CursorIcon::Text
                        } else {
                            CursorIcon::ResizeHorizontal
                        });
                }
                if self.0.field.local_button(point) {
                    return self.0.field.editor.enabled.then_some(CursorIcon::Default);
                }
                self.0.field.editor.enabled.then_some(CursorIcon::Text)
            }
            fn text_input_state(&self) -> Option<TextInputState> {
                self.0.field.editor.input_state()
            }
            fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
                self.0.update(cx)
            }
            fn deliver_notifications(&mut self, cx: &mut EventContext) -> Result<(), RuntimeError> {
                self.0.apply_start(cx)
            }
            fn tab_navigation(
                &mut self,
                cx: &mut EventContext,
                direction: crate::FocusDirection,
                transition: crate::KeyTransition,
            ) -> Result<InputOutcome, RuntimeError> {
                self.0.session_tab(cx, direction, transition)
            }
            fn measure(
                &self,
                cx: &MeasureContext<'_>,
                available: Constraints,
            ) -> Result<Size, RuntimeError> {
                if self.0.field.inline {
                    self.0.field.measure(cx, &self.0.width_hint(), false)
                } else {
                    self.0.field.editor.measure(cx, available)
                }
            }
            fn arrange(&self, cx: &mut ArrangeContext<'_>, size: Size) -> Result<(), RuntimeError> {
                if self.0.field.inline {
                    self.0
                        .field
                        .arrange(cx.measurement(), size, &self.0.width_hint(), false)?;
                    self.0.field.place(cx)
                } else {
                    if !cx.is_empty() {
                        return Err(LayoutError::InvalidChild.into());
                    }
                    self.0.field.arrange_plain(cx.measurement(), size)
                }
            }
            fn paint(&self, p: &mut Painter) -> Result<(), DrawError> {
                self.0.paint(p)
            }
            fn hover_changed(&mut self, cx: &mut EventContext, hovered: bool) {
                if self.0.field.inline {
                    self.0.field.set_hover(cx, hovered);
                } else {
                    self.0.field.editor.set_hover(cx, hovered);
                }
            }
            fn text_key_event(
                &mut self,
                cx: &mut EventContext,
                input: CommandKeyInput,
                transition: KeyTransition,
                keys: &KeyboardState,
            ) -> Result<TextKeyOutcome, RuntimeError> {
                if self.0.field.drafting() {
                    // The keys the row's editor leaves to it, answered in the
                    // text phase so capture bindings never take them.
                    return Ok(
                        if self.0.text_key(cx, input, transition, keys)? == InputOutcome::Handled {
                            TextKeyOutcome::Handled
                        } else {
                            TextKeyOutcome::Ignored
                        },
                    );
                }
                if self.0.field.inline && !self.0.field.editing {
                    return Ok(TextKeyOutcome::NotAnEditor);
                }
                if self.0.text_key(cx, input, transition, keys)? == InputOutcome::Handled {
                    return Ok(TextKeyOutcome::Handled);
                }
                Ok(unhandled_text_key(cx, input, transition))
            }
            fn keyboard_event(
                &mut self,
                cx: &mut EventContext,
                e: KeyEvent,
                keys: &KeyboardState,
            ) -> Result<InputOutcome, RuntimeError> {
                self.0
                    .key(cx, e, keys.control_key(e), keys.chord_modifiers(e))
            }
            fn text_command(
                &mut self,
                cx: &mut EventContext,
                command: TextCommand,
            ) -> Result<TextCommandOutcome, RuntimeError> {
                self.0.command(cx, command)
            }
            fn text_input(
                &mut self,
                cx: &mut EventContext,
                text: &str,
            ) -> Result<(), RuntimeError> {
                if self.0.field.inline && !self.0.field.editing {
                    return Ok(());
                }
                self.0.field.editor.insert(cx, text).map(|_| ())
            }
            fn text_preedit(
                &mut self,
                cx: &mut EventContext,
                text: &TextPreedit,
            ) -> Result<(), RuntimeError> {
                if self.0.field.inline && !self.0.field.editing {
                    return Ok(());
                }
                self.0.field.editor.compose(cx, text)
            }
            fn pointer_input(
                &mut self,
                cx: &mut EventContext,
                e: PointerEvent,
            ) -> Result<InputOutcome, RuntimeError> {
                self.0.pointer(cx, e)
            }
            fn focus_lost(&mut self, cx: &mut EventContext) -> Result<(), RuntimeError> {
                if self.0.field.drafting() {
                    // Focus moved into the row's own editor.
                    return Ok(());
                }
                self.0.focus_left(cx)
            }
            fn focus_within_changed(
                &mut self,
                cx: &mut EventContext,
                within: bool,
            ) -> Result<(), RuntimeError> {
                if within || !self.0.field.drafting() {
                    return Ok(());
                }
                self.0.focus_left(cx)
            }
            fn visibility_changed(&mut self, visible: bool) {
                if !visible && self.0.field.drafting() {
                    self.0.cancel();
                }
            }
            fn input_reset(&mut self) {
                self.0.cancel();
            }
            fn timer_elapsed(&mut self, cx: &mut EventContext) -> Result<(), RuntimeError> {
                self.0.repeat_hold(cx)
            }
            fn cancel_input(&mut self, cx: &mut EventContext) {
                // Failed clipboard/key operations and geometry changes must not
                // discard text in a field that still owns focus, directly or
                // through its child editor. Escape restores explicitly in key();
                // hiding/removal already relinquish focus.
                if (cx.focused() || self.0.field.drafting())
                    && !self.0.field.pressed()
                    && self.0.hold.is_none()
                    && self.0.held_key.is_none()
                {
                    self.0.field.editor.reset();
                } else {
                    self.0.cancel();
                }
                cx.invalidate(Invalidation::Paint);
            }
            fn inspect(&self) -> ComponentDetails {
                self.0.inspect()
            }
        }
    };
}
impl<T: Number> NumberEditor<T> {
    /// Applies the focus-loss policy: a parameter draft is cancelled; a local
    /// draft is saved.
    fn focus_left(&mut self, cx: &mut EventContext) -> Result<(), RuntimeError> {
        let result = if self.field.pressed() || self.hold.is_some() || self.source.is_parameter() {
            self.cancel();
            Ok(())
        } else if self.session().is_some() {
            self.complete(cx, EditorTrigger::FocusLeft)
        } else {
            self.commit(cx)
        };
        self.field.editor.reset();
        cx.invalidate(Invalidation::Paint);
        result
    }
}
macro_rules! field_api {
    ($name:ident, $ty:ty) => {
        impl $name {
            /// Creates an enabled local numeric editor with defaults.
            /// The binding is validated/subscribed on first update. Foreign-runtime
            /// errors propagate; an invalid bound value is refused by the field
            /// without modifying it.
            pub fn new(value: impl Into<Signal<$ty>>) -> Self {
                let value: Signal<$ty> = value.into();
                Self(NumberEditor::new(Box::new(value), false))
            }
            /// Makes this field the editor of a host's
            /// [`EditorSession`](crate::EditorSession), such as a table cell's,
            /// as [`TextField::session`](crate::TextField::session) describes:
            /// it attaches to the edit open now, applies its start from its
            /// next delivery, keeps Enter, Tab and Escape and reports each
            /// once. Enter and Tab save a valid draft as Enter does and report
            /// `Saved` or `Unchanged`; an invalid or out-of-range draft, or a
            /// read-only field's changed draft, reports `Refused` and stays.
            /// Escape restores the value and reports `Cancelled`. With a
            /// session the arrow keys, wheel and step buttons change the
            /// draft instead of saving each step, so Escape restores what the
            /// steps changed; during a held step every key is consumed and
            /// nothing is reported. A field that refused its bound value has
            /// nothing to correct and reports `Unchanged`. Focus leaving
            /// reports `FocusLeft` with its save's outcome, delivered once a
            /// hidden native window is shown again. Disabling the field
            /// reports nothing: it releases focus as cancelled input, so the
            /// draft stays unsaved and the edit open. A field bound to
            /// a parameter ignores the session. Replaces an earlier session.
            pub fn session(mut self, session: &crate::EditorSession) -> Self {
                self.0.session = Some(session.attach());
                self
            }
        }
    };
}
macro_rules! inline_api {
    ($name:ident, $ty:ty) => {
        impl $name {
            /// Creates an enabled inline row with defaults and an empty caption
            /// and unit. The binding is validated/subscribed on first update;
            /// foreign-runtime errors propagate, and an invalid bound value is
            /// refused by the row without modifying it.
            pub fn new(value: impl Into<Signal<$ty>>) -> Self {
                let value: Signal<$ty> = value.into();
                Self(NumberEditor::new(Box::new(value), true))
            }
            /// Sets the leading caption, empty by default. It is presentation
            /// text outside selection and never part of the edited value.
            /// Without a caption, a row made with `for_parameter` is named
            /// after the parameter.
            ///
            /// The caption is a [`TextSource`]: fixed plain or rich text, or a
            /// binding the row follows. A binding is read in the row's update
            /// and remeasures the row when it changes; describing the row again
            /// with the same binding keeps the text already read.
            pub fn label(mut self, label: impl Into<TextSource>) -> Self {
                self.0.field.caption = crate::Caption::new(label);
                self
            }
        }
    };
}
macro_rules! precision_api {
    ($name:ident) => {
        impl $name {
            /// Sets resting and stepped display precision, from 0 through
            /// [`NumberInputError::MAX_PRECISION`] fractional digits.
            /// Typed drafts remain free-form. Keyboard/wheel steps preserve the exact numeric
            /// draft, including through undo/redo, even when the rounded text does not change.
            /// This rounds only text, never the stored value: a value with more
            /// digits is shown rounded but saved and stepped at full precision.
            /// To store rounded values, round them in `normalize` when they are
            /// saved. The default is shortest round-trippable formatting. No
            /// locale or grouping is applied.
            ///
            /// # Errors
            /// Returns [`NumberInputError::InvalidPrecision`] above
            /// [`NumberInputError::MAX_PRECISION`].
            pub fn decimal_places(mut self, digits: u8) -> Result<Self, NumberInputError> {
                if digits > NumberInputError::MAX_PRECISION {
                    return Err(NumberInputError::InvalidPrecision(digits));
                }
                self.0.precision = Some(digits);
                Ok(self)
            }
            /// The former name of [`Self::decimal_places`], with the same rules.
            ///
            /// # Errors
            /// Those of [`Self::decimal_places`].
            #[deprecated(note = "renamed to `decimal_places`, the name the sliders use")]
            pub fn precision(self, digits: u8) -> Result<Self, NumberInputError> {
                self.decimal_places(digits)
            }
        }
    };
}
fn integer_parameter(parameter: Parameter<i64>) -> Box<dyn NumberSource<i64>> {
    Box::new(IntegerParameter {
        parameter,
        spec: RefCell::new(None),
        session: RefCell::new(None),
    })
}
impl IntegerInput {
    /// Creates an editor bound to a stepped parameter; see the parameter-binding
    /// rules above. Keep the `ParameterHost` alive while editing is available.
    ///
    /// ```
    /// # use pert_ui_runtime as pert_ui;
    /// use pert_ui::{
    ///     Element, IntegerInput, KeyAction, KeyCode, ParameterId, ParameterSpec, Runtime,
    /// };
    /// let mut ui = Runtime::new();
    /// let (count, host) = ui.parameter(
    ///     ParameterId::new(1, 1, "count")?,
    ///     ParameterSpec::stepped("Count", 0, 100, 12)?,
    /// )?;
    /// let field = ui.mount_tree(Element::new("count", IntegerInput::for_parameter(count.clone())))?;
    /// ui.focus(&field)?;
    /// ui.key_input(KeyCode::ArrowUp, KeyAction::Press)?; // a draft, not an edit yet
    /// assert_eq!(count.snapshot().get()?.base, 12);
    /// ui.key_input(KeyCode::Enter, KeyAction::Press)?; // one begin/commit pair
    /// assert_eq!(count.snapshot().get()?.base, 13);
    ///
    /// host.write_base(20)?; // an authoritative application update
    /// assert!(host.write_base(101).is_err()); // rejected, never clamped
    /// assert_eq!(count.snapshot().get()?.base, 20);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn for_parameter(parameter: Parameter<i64>) -> Self {
        Self(NumberEditor::new(integer_parameter(parameter), false))
    }
}
impl InlineInteger {
    /// Creates an inline row bound to a stepped parameter, with the rules of
    /// [`IntegerInput::for_parameter`] and an empty caption and unit.
    pub fn for_parameter(parameter: Parameter<i64>) -> Self {
        Self(NumberEditor::new(integer_parameter(parameter), true))
    }
}
fn decimal_parameter(parameter: Parameter<f32>) -> Box<dyn NumberSource<f64>> {
    Box::new(DecimalParameter {
        parameter,
        spec: RefCell::new(None),
        session: RefCell::new(None),
    })
}
impl DecimalInput {
    /// Creates an editor bound to a continuous parameter, with the
    /// parameter-binding rules above. The specification supplies the bounds;
    /// `step` and `decimal_places` stay the editor's, since a continuous
    /// parameter has no lattice. Values cross at `f32` resolution: the field
    /// shows the shortest decimal that round-trips the parameter's value, and
    /// saving rounds the typed number to the nearest `f32`. Keep the
    /// `ParameterHost` alive while editing is available.
    ///
    /// ```
    /// # use pert_ui_runtime as pert_ui;
    /// use pert_ui::{
    ///     DecimalInput, Element, KeyAction, KeyCode, ParameterId, ParameterSpec, Runtime,
    /// };
    /// let mut ui = Runtime::new();
    /// let (gain, _host) = ui.parameter(
    ///     ParameterId::new(1, 1, "gain")?,
    ///     ParameterSpec::new("Gain", 0.0, 1.0, 0.1)?,
    /// )?;
    /// let field = DecimalInput::for_parameter(gain.clone()).step(0.05)?;
    /// let field = ui.mount_tree(Element::new("gain", field))?;
    /// ui.focus(&field)?;
    /// ui.key_input(KeyCode::ArrowUp, KeyAction::Press)?; // a draft
    /// ui.key_input(KeyCode::Enter, KeyAction::Press)?; // one begin/commit pair
    /// assert_eq!(gain.snapshot().get()?.base, 0.15);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn for_parameter(parameter: Parameter<f32>) -> Self {
        Self(NumberEditor::new(decimal_parameter(parameter), false))
    }
}
impl InlineDecimal {
    /// Creates an inline row bound to a continuous parameter, with the rules of
    /// [`DecimalInput::for_parameter`] and an empty caption and unit.
    pub fn for_parameter(parameter: Parameter<f32>) -> Self {
        Self(NumberEditor::new(decimal_parameter(parameter), true))
    }
}
number_shared!(IntegerInput, i64);
number_shared!(DecimalInput, f64);
number_shared!(InlineInteger, i64);
number_shared!(InlineDecimal, f64);
field_api!(IntegerInput, i64);
field_api!(DecimalInput, f64);
inline_api!(InlineInteger, i64);
inline_api!(InlineDecimal, f64);
precision_api!(DecimalInput);
precision_api!(InlineDecimal);
