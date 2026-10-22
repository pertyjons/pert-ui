//! Standard local text editors sharing public text/input extension points.
use crate::choice_controls::field_frame;
use crate::layout::rect;
use crate::scrollbar;
use crate::*;
use std::{
    cell::{Cell, RefCell},
    collections::VecDeque,
    ops::Range,
    rc::Rc,
    time::Duration,
};
use unicode_segmentation::UnicodeSegmentation;

mod inline;
pub(crate) use inline::InlineField;
mod draft;
pub use draft::{DraftAlign, DraftEditor, DraftEditorController, DraftSelection};
mod inline_text;
mod spaced_paste;
pub use inline_text::InlineText;
mod normalize;
pub use normalize::InputRejection;
pub(crate) use normalize::{EntryError, Normalizer, RejectionReport, SaveReport, normalize_text};
mod search;
pub use search::{SearchField, SearchKey};
mod number_inputs;
pub use number_inputs::{
    DecimalInput, InlineDecimal, InlineInteger, IntegerInput, NumberInputError,
};

const BLINK: Duration = Duration::from_millis(500);
type Submit = Box<dyn FnMut(&mut EventContext, &str) -> Result<(), RuntimeError>>;
type ModifiedReport = Box<dyn FnMut(&mut EventContext, bool) -> Result<(), RuntimeError>>;
/// Which application callback a requested delivery reports to.
#[derive(Clone, Copy)]
enum Report {
    Modified,
    Refusal,
    /// Applies the editor session's pending start.
    Start,
}
type RefusalReport =
    Box<dyn FnMut(&mut EventContext, Option<&TextEditError>) -> Result<(), RuntimeError>>;

/// A single-line editor bound to a UI-local `Signal<String>`.
///
/// Defaults to body typography and a 240-pixel preferred width. Changes immediately
/// update the binding, unless [normalization or length limits](#normalization)
/// or a [session](Self::session) enable draft mode;
/// Enter invokes an optional submit callback once per press.
/// Text is vertically centered in the field's available height.
/// Escape/focus loss discard only uncommitted IME text. External writes win, reset
/// selection and clear local undo. This is not an authoritative parameter gesture.
///
/// Both editors support arrows, Home/End, Shift selection, platform Ctrl/Cmd+A/C/X/V/Z,
/// redo with Shift+Ctrl/Cmd+Z (also Ctrl+Y outside macOS), word navigation/deletion,
/// pointer drag selection, double-click word selection and triple-click select-all.
/// Repeated editing keys are accepted; Tab/Shift+Tab traverse focus. Consecutive
/// clicks must be within 500 ms and four logical pixels of the previous one, as
/// [`ClickCounter`](crate::ClickCounter) counts them; a fourth click starts over.
/// Local undo keeps 32 edits.
///
/// Bound strings must be at most [`TextEditError::MAX_TEXT_BYTES`] bytes of UTF-8 and contain no control characters
/// or line separators; [`Self::check`] tells whether a string qualifies. Typed
/// text with a control character the field does not accept is rejected with
/// [`RuntimeError::TextEdit`]. An edit that would take the text past
/// [`TextEditError::MAX_TEXT_BYTES`] (typing, an IME commit or a paste into a full field) is
/// refused without an error, as any editor at its limit refuses input: the text,
/// selection and binding stay unchanged, and a composition that would not fit
/// is not shown until one that fits or the commit arrives.
/// Pasted line breaks (CR, LF, NEL, vertical tab, form feed and the Unicode line
/// and paragraph separators) and tabs become spaces, and other pasted control
/// characters are dropped. Preparation also obeys
/// `TextError` raster limits; an edit is prepared before writing the binding.
///
/// A bound value that does not qualify, written by the application, is refused
/// by the field rather than failing the runtime: the field keeps showing its
/// previous text, draws the error outline, reports `binding_error` in
/// inspection and to [`TextField::on_binding_refusal`], marks its semantics
/// invalid, and accepts no edits, so it never
/// overwrites the application's value. Selection and copy still work. The next
/// acceptable binding value ends the refusal. The binding itself is never
/// rewritten; check or clean text with [`Self::check`] before writing it.
///
/// Clipboard errors propagate (an empty/non-text clipboard is a no-op), and
/// failed cut preserves the text. No clipboard is installed
/// in a headless runtime unless the host calls `Runtime::set_clipboard`.
///
/// Both editors own a clipboard context menu, opened with the secondary pointer
/// button at the pointer or with the ContextMenu (Menu) key or Shift+F10 below
/// the field. Right-clicking leaves the caret and selection alone, so the menu
/// acts on what the field already shows.
/// It offers Copy, Cut, Paste, Paste Before and Paste After, which are the
/// [`TextCommand`] variants; an action that does not apply to the current
/// selection, clipboard or read-only state is shown grayed out rather than hidden.
/// Availability is decided once when the menu opens, from one clipboard read, so
/// an open menu polls nothing. A consumer can build the same menu from
/// [`ContextMenu`](crate::ContextMenu), [`MenuItem::enabled`] and
/// [`EventContext::request_text_command`](crate::EventContext::request_text_command).
///
/// Caret blinking uses one 500 ms deadline while focused and editable; hidden,
/// unfocused and read-only editors do not blink. Text edits retain outer geometry.
/// An enabled field uses SurfaceHover under the pointer without changing layout.
///
/// Describing a mounted field again applies the new binding, style,
/// permissions, normalizer and callbacks. A different binding ends the edit in
/// progress as an application write does: a composition and a draft are
/// dropped unsaved and undo is cleared. The same binding keeps them.
///
/// # Normalization
///
/// Disabling retains an unsaved draft but cannot save it, including on immediate
/// focus loss; reenabling permits a later explicit save or focus-loss save.
///
/// With [`Self::normalize`], [`Self::min_length`] or [`Self::max_length`],
/// the field holds a draft instead of writing the
/// binding on every edit. Enter (a fresh press, outside IME composition) and
/// focus transfer or window focus loss save a changed draft: the normalizer
/// adjusts or refuses it, and an accepted result is written once and replaces
/// the draft. The submit callback runs after a successful Enter save and
/// receives the saved text; a refused save does not call it. Escape discards
/// the draft, after first ending a composition or collapsing a selection as it
/// always does, and hiding discards it without saving. Undo and redo act on the
/// draft and never normalize; a save clears the local undo history. An
/// external write that changes the binding wins over the draft and discards it,
/// so a later focus loss has nothing to save.
///
/// ```
/// # use pert_ui_runtime as pert_ui;
/// use pert_ui::{Element, KeyAction, KeyCode, Runtime, TextField};
/// let mut ui = Runtime::new();
/// let tag = ui.signal(String::new());
/// let field = ui.mount_tree(Element::new("tag",
///     TextField::new(tag.clone()).normalize(|text| Ok(text.trim().to_lowercase()))))?;
/// ui.focus(&field)?;
/// ui.text_input("  Reports ")?;
/// assert_eq!(tag.get()?, ""); // still a draft
/// ui.key_input(KeyCode::Enter, KeyAction::Press)?;
/// assert_eq!(tag.get()?, "reports");
/// # Ok::<(), pert_ui::RuntimeError>(())
/// ```
///
/// ```
/// # use pert_ui_runtime as pert_ui;
/// use pert_ui::{Element, Runtime, TextField};
/// let mut ui = Runtime::new();
/// let name = ui.signal(String::from("Example"));
/// let field = ui.mount_tree(Element::new("name", TextField::new(name.clone())))?;
/// ui.focus(&field)?;
/// ui.text_input("New ")?;
/// assert_eq!(name.get()?, "New Example");
/// # Ok::<(), pert_ui::RuntimeError>(())
/// ```
#[must_use]
pub struct TextField(Editor);

/// A multiline local text editor with monospace typography by default.
///
/// Uses bundled Noto Sans Mono, a 320-pixel preferred width and six visible lines.
/// Soft wraps to its allocation and scrolls internally to reveal the caret;
/// wheel scrolls the text. Enter inserts LF, Tab traverses focus. Pasted CRLF,
/// CR, NEL, vertical tab, form feed and the Unicode line and paragraph
/// separators become LF, pasted tabs are kept and other pasted control
/// characters are dropped. Bound text accepts LF and tabs, but no other control
/// characters or line separators; text read from a file with CRLF line endings,
/// for example, must be converted before it is written to the binding.
/// Call `style` to override typography.
///
/// Text taller than the field shows the same vertical scrollbar as scroll areas,
/// views and windows: a track strip along the text's right edge with a thumb in
/// `ColorRole::ScrollbarThumb`, or `ScrollbarThumbHover` while the pointer is
/// over the strip or dragging it, sized by the theme's scrollbar metrics (8, 4 and at least 12 logical
/// pixels at Standard). It appears only while
/// the text overflows and overlays the text instead of rewrapping it, so showing
/// it never reflows the lines. Primary dragging preserves the thumb grab position;
/// clicking the track centers the thumb there and starts a drag. Release ends
/// dragging, and Escape or context loss cancels it at the current offset without
/// restoring the previous one. Scrolling never moves the caret or selection, and
/// the next caret movement, edit or resize reveals the caret again. Scrolling
/// repaints without remeasuring, and the pointer keeps the ordinary arrow over the
/// bar. A read-only field keeps its bar; a disabled one shows none, because it
/// does not scroll at all. Single-line fields scroll horizontally without a
/// scrollbar.
///
/// Binding ownership and refused bindings, [`TextEditError::MAX_TEXT_BYTES`]/preparation limits,
/// selection, clipboard, undo, cancellation and scheduling follow `TextField`.
/// This is plain text, with no syntax highlighting or host project undo policy.
/// An enabled field uses SurfaceHover under the pointer without changing layout.
///
/// # Normalization
///
/// [`Self::normalize`] follows [`TextField`'s normalization](TextField#normalization),
/// except that Enter inserts a line break, so only focus transfer or window
/// focus loss saves the draft.
///
/// ```
/// # use pert_ui_runtime as pert_ui;
/// use pert_ui::{Element, Runtime, TextArea};
/// let mut ui = Runtime::new();
/// let notes = ui.signal(String::from("First line\nSecond line"));
/// ui.mount_tree(Element::new("notes", TextArea::new(notes)))?;
/// # Ok::<(), pert_ui::RuntimeError>(())
/// ```
#[must_use]
pub struct TextArea(Editor);

macro_rules! editor_api {
    ($name:ident, $multi:expr) => {
        impl $name {
            /// Creates an enabled editor. The binding is subscribed on the first
            /// update; a foreign-runtime binding fails that update, and a value
            /// the editor does not accept is refused as described above.
            pub fn new(value: impl Into<Signal<String>>) -> Self {
                let value: Signal<String> = value.into();
                Self(Editor::new(Some(value), $multi))
            }
            /// Checks whether `text` is acceptable as this editor's bound value,
            /// so an application can reject or convert text, such as a document
            /// read from a file, before writing it to the binding. This performs
            /// no text preparation and needs no runtime.
            ///
            /// # Errors
            /// Returns [`TextEditError::TooLarge`] above [`TextEditError::MAX_TEXT_BYTES`] bytes
            /// and [`TextEditError::InvalidText`] for a control character or a
            /// Unicode line or paragraph separator that the editor does not accept.
            pub fn check(text: &str) -> Result<(), TextEditError> {
                check_text(text, $multi)
            }
            /// Overrides the default typography using a bundled semantic style.
            pub fn style(mut self, style: TextStyle) -> Self {
                self.0.style = style;
                self
            }
            /// Allows selection/copy but prevents editing and native IME activation.
            /// Equivalent to `editable(false)`; a later permission cannot lift it.
            pub fn read_only(self) -> Self {
                self.editable(false)
            }
            /// Restricts editing with a fixed or reactive permission; editable by
            /// default. Repeated calls combine restrictions; every condition must
            /// allow, and same-key redescription replaces the permission with the
            /// new description's.
            ///
            /// While editing is denied the editor keeps focus, scrolling,
            /// selection and copying but refuses edits and native IME activation.
            /// Revoking it, observed at the next update, cancels a composition and
            /// a selection drag; an unsaved draft stays but is not saved, also not
            /// on focus loss, until editing is allowed again, which replays nothing.
            /// A binding from another runtime fails the update with
            /// [`RuntimeError::ForeignRuntime`].
            pub fn editable(mut self, permission: impl Into<Permission>) -> Self {
                self.0.restrict_editing(permission.into());
                self
            }
            /// Prevents interaction and automatic focus; still follows its binding.
            /// Equivalent to `enabled(false)`; a later permission cannot lift it.
            pub fn disabled(self) -> Self {
                self.enabled(false)
            }
            /// Restricts focus and input with a fixed or reactive permission;
            /// enabled by default. Repeated calls combine restrictions; every
            /// condition must allow, and same-key redescription replaces the
            /// permission with the new description's. A disabled editor still
            /// follows its binding.
            ///
            /// Revoking it, observed at the next update, makes the editor
            /// unfocusable and releases focus, cancels a composition and a
            /// selection drag, and keeps an unsaved draft without saving it,
            /// including on the focus loss the revocation causes. Enabling it
            /// again makes the draft editable and replays nothing. A binding from
            /// another runtime fails the update with
            /// [`RuntimeError::ForeignRuntime`].
            pub fn enabled(mut self, permission: impl Into<Permission>) -> Self {
                self.0.restrict_enablement(permission.into());
                self
            }
            /// Adjusts the entered text when it is saved, such as trimming it or
            /// changing its case, or refuses it with an [`InputRejection`].
            ///
            /// A normalizing editor holds a **draft**: edits stay local, and the
            /// binding is written only by a save of a changed draft, which runs
            /// the normalizer, checks the result as bound text ([`Self::check`]),
            /// prepares it and writes it. See [`Self#normalization`] for when
            /// saves happen. A refused draft writes nothing and stays for
            /// correction with the error outline; the refusal ends when the
            /// draft changes. Text the application writes to the binding is
            /// never normalized, and it wins over a draft. Without a normalizer
            /// the editor writes the binding on every edit, as by default.
            ///
            /// The function must be deterministic, bounded and free of side
            /// effects, binding writes and I/O, and idempotent: normalizing its
            /// own output must return that output unchanged, so a saved value
            /// stays as it is on the next save. It never sees IME preedit, only
            /// committed text. Installing another normalizer replaces this one.
            pub fn normalize(
                mut self,
                normalize: impl Fn(String) -> Result<String, InputRejection> + 'static,
            ) -> Self {
                self.0.normalizer = Some(Rc::new(normalize));
                self
            }
            /// Tells the application about the normalizer's refusals, so it
            /// can present them, for example by publishing a
            /// [`FieldStatus`](crate::FieldStatus) through its
            /// [`FieldHandle`](crate::FieldHandle).
            ///
            /// The callback runs on the UI thread in the event phase. It
            /// receives `Some` on each save the normalizer or a length limit
            /// ([`Self::min_length`], [`Self::max_length`]) refuses, also in an
            /// editor with limits and no normalizer, and on Enter in a required
            /// field whose unchanged text is below its minimum, including
            /// a repeated refusal of the same draft, and `None` once when a
            /// reported refusal ends: at a later save that succeeds or finds
            /// nothing to save (including focus loss), and at Escape. An
            /// application write or hiding that discards the draft happens
            /// outside an event, so the next save or Escape reports its end;
            /// that Escape still reaches the field's owner. Removal and
            /// shutdown report nothing: an owner that removes a field clears
            /// whatever status it published for it. Only [`InputRejection`]s
            /// are reported, not the field's own text checks.
            ///
            /// An error from the callback propagates from the input call that
            /// saved. The field has already applied the outcome by then: a
            /// refused draft is kept and marked, and a saved one is written. A
            /// failed `None` stays pending and is reported again at the next
            /// save or Escape. Replaces an earlier callback.
            ///
            /// ```
            /// # use pert_ui_runtime as pert_ui;
            /// use pert_ui::{
            ///     Element, Field, FieldStatus, FormFields, InputRejection, KeyAction, KeyCode,
            ///     Runtime, TextField,
            /// };
            /// let mut ui = Runtime::new();
            /// let form = FormFields::new(&ui);
            /// let handle = form.field("name")?;
            /// let status = handle.clone();
            /// let name = ui.signal(String::new());
            /// let field = TextField::new(name.clone())
            ///     .normalize(|text| {
            ///         let text = text.trim().to_owned();
            ///         if text.is_empty() { Err(InputRejection::new("Enter a name")) } else { Ok(text) }
            ///     })
            ///     .on_rejection(move |_, rejection| {
            ///         let next = rejection.map_or(FieldStatus::Normal, |r| FieldStatus::error(r.message()));
            ///         status.set(next)
            ///     });
            /// let root = ui.mount_tree(Element::new("name",
            ///     Field::new(Element::new("input", field)).label("Name").form(&handle)))?;
            /// let input = ui.find_descendant(&root, "input")?.expect("the field's control");
            /// ui.focus(&input)?;
            /// let enter = |ui: &mut Runtime| -> Result<(), pert_ui::RuntimeError> {
            ///     ui.key_input(KeyCode::Enter, KeyAction::Press)?;
            ///     ui.key_input(KeyCode::Enter, KeyAction::Release).map(|_| ())
            /// };
            /// ui.text_input("  ")?;
            /// enter(&mut ui)?;
            /// assert_eq!(handle.status().get()?, FieldStatus::error("Enter a name"));
            /// ui.text_input("Ada")?;
            /// enter(&mut ui)?;
            /// assert_eq!(handle.status().get()?, FieldStatus::Normal);
            /// assert_eq!(name.get()?, "Ada");
            /// # Ok::<(), Box<dyn std::error::Error>>(())
            /// ```
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
            /// Tells the application about each save that wrote the binding,
            /// with the saved (normalized) text, so it can act on a value the
            /// user finished entering, for example to update other state from
            /// it.
            ///
            /// Only an editor in draft mode saves, one with a normalizer, a
            /// length limit or, for a `TextField`, a
            /// [session](TextField::session): see [`Self::normalize`] for when. The callback runs on the UI thread in the event phase,
            /// after the binding holds the saved text, once per save that wrote
            /// it, whether Enter or focus loss saved it. It does not run for a
            /// refused draft, a draft that normalizes to the bound text (which
            /// writes nothing), Escape, application writes, hiding or removal.
            /// Without a normalizer or a length limit the editor writes the
            /// binding on every edit and never calls it; observe the binding
            /// instead. To save finished edits without adjusting or limiting
            /// the text, for example to record each one as one undo step,
            /// install the identity normalizer: `.normalize(Ok)`. A
            /// `TextField` then saves on Enter and on focus loss; in a
            /// `TextArea` Enter inserts a line break, so it saves on focus
            /// loss.
            ///
            /// The callback may write bindings. Its error propagates from the
            /// input call that saved; the text has been written by then. A
            /// failing [`Self::on_rejection`] report does not suppress it:
            /// both run, and the report's error is returned first.
            /// Replaces an earlier callback.
            pub fn on_save(
                mut self,
                saved: impl FnMut(&mut EventContext, &str) -> Result<(), RuntimeError> + 'static,
            ) -> Self {
                self.0.saved.set(Box::new(saved));
                self
            }
            /// Sets muted hint text shown while the editor is empty and nothing
            /// is being composed, such as "Name of the project". It is
            /// presentation only: never part of the value, of the semantic
            /// value or of inspection's `text`; inspection reports it as
            /// `placeholder`. Empty, the default, shows none.
            ///
            /// The hint is a [`TextSource`]: fixed plain or rich text, a
            /// binding the editor follows, or a [`StringRole`](crate::StringRole)
            /// that follows the installed string table. A binding or role is
            /// read in the editor's update; describing the editor again with
            /// the same source keeps the text already read.
            pub fn placeholder(mut self, text: impl Into<TextSource>) -> Self {
                self.0.placeholder = Caption::new(text);
                self
            }
            /// Marks the value as required by the application. Semantics
            /// report it ([`Semantics::required`]) and inspection reports
            /// `required`. It decides nothing by itself: an empty required
            /// editor is not an error unless the application publishes one,
            /// except that [`Self::min_length`] then applies to the empty
            /// value too. Wrap the editor in a [`Field`](crate::Field) with
            /// [`Field::required`](crate::Field::required) to show the marker
            /// beside the label. Default: not required.
            pub fn required(mut self, required: bool) -> Self {
                self.0.required = required;
                self
            }
            /// Sets a minimum length in grapheme clusters (user-perceived
            /// characters, not bytes or `char`s): refuses a saved value with
            /// fewer than `graphemes` of them, with an [`InputRejection`] that
            /// uses [`StringRole::AtLeastCharacters`](crate::StringRole::AtLeastCharacters)
            /// from the installed string table (English default:
            /// "Character count must be at least `count`").
            ///
            /// A length limit puts the editor in draft mode, as
            /// [`Self::normalize`] does: edits stay local and a save checks the
            /// limits after normalizing, then writes. An empty value passes
            /// unless the editor is [required](Self::required), so an optional
            /// field may stay empty. Typing is never stopped at a limit; the
            /// check happens when the draft is saved. Text the application
            /// writes is not checked. `None`, the default, sets no minimum.
            pub fn min_length(mut self, graphemes: Option<usize>) -> Self {
                self.0.min_length = graphemes;
                self
            }
            /// Sets a maximum length in grapheme clusters, as
            /// [`Self::min_length`] counts them: refuses a saved value with more
            /// than `graphemes` of them, with an [`InputRejection`] from
            /// [`StringRole::AtMostCharacters`](crate::StringRole::AtMostCharacters)
            /// in the installed string table (English default:
            /// "Character count must be at most `count`"). Draft mode and checking follow
            /// [`Self::min_length`]; typing past the limit is allowed and the
            /// [counter](Self::counter) shows it in the Error role. `None`, the
            /// default, sets no maximum.
            pub fn max_length(mut self, graphemes: Option<usize>) -> Self {
                self.0.max_length = graphemes;
                self
            }
            /// Shows the draft's grapheme count against its limit inside the
            /// field, in caption text: "12/50" against the maximum, or against
            /// the minimum when there is no maximum. The count is Muted, or in
            /// the Error role while it is above the maximum or below a nonzero
            /// minimum. Without a limit it shows the count alone. Its space is
            /// reserved for the widest count a draft can reach, so typing never
            /// reflows the field and no count is clipped. Default: off.
            pub fn counter(mut self, shown: bool) -> Self {
                self.0.counter = shown;
                self
            }
            /// Paints a small mark in the field's top end corner, in the
            /// Primary role, while an unsaved draft differs from the saved
            /// value. Only an editor in draft mode (with a
            /// [normalizer](Self::normalize), a length limit or, for a
            /// `TextField`, a [session](TextField::session)) holds such a
            /// draft; otherwise every edit is written at once and nothing is
            /// ever unsaved. Inspection reports the state as `dirty` and the
            /// semantics describe an unsaved draft with the installed
            /// [`UiStrings`](crate::UiStrings)'
            /// [`StringRole::TextModified`](crate::StringRole::TextModified),
            /// with or without the mark. Default: no mark.
            pub fn modified_indicator(mut self, shown: bool) -> Self {
                self.0.modified_indicator = shown;
                self
            }
            /// Calls `modified` in the event phase with `true` when the draft
            /// starts to differ from the saved value and `false` when it stops:
            /// after a save, Escape, an application write that discards the
            /// draft, or an edit back to the saved text. Changes made in an
            /// update, such as an application write, are delivered later in
            /// the same flush; each change is delivered once and in order. The
            /// callback may write signals, for example
            /// [`FieldHandle::set_modified`](crate::FieldHandle::set_modified)
            /// so a [`FormFields`](crate::FormFields) counts unsaved fields.
            /// Only an editor in draft mode ever reports `true`. Its error is
            /// returned from the flush or input call that delivered it.
            ///
            /// ```
            /// # use pert_ui_runtime as pert_ui;
            /// use pert_ui::*;
            /// let mut ui = Runtime::new();
            /// let form = FormFields::new(&ui);
            /// let handle = form.field("name")?;
            /// let name = ui.signal(String::new());
            /// let report = handle.clone();
            /// let field = ui.mount_tree(Element::new("name",
            ///     TextField::new(name.clone())
            ///         .max_length(Some(20))
            ///         .on_modified(move |_, modified| report.set_modified(modified).map(|_| ()))))?;
            /// ui.flush(std::time::Duration::ZERO)?;
            /// ui.focus(&field)?;
            /// ui.text_input("Ada")?;
            /// ui.flush(std::time::Duration::ZERO)?;
            /// assert_eq!(form.summary().get()?.modified(), 1);
            /// ui.key_input(KeyCode::Enter, KeyAction::Press)?;
            /// ui.key_input(KeyCode::Enter, KeyAction::Release)?;
            /// assert_eq!(name.get()?, "Ada");
            /// assert_eq!(form.summary().get()?.modified(), 0);
            /// # Ok::<(), RuntimeError>(())
            /// ```
            pub fn on_modified(
                mut self,
                modified: impl FnMut(&mut EventContext, bool) -> Result<(), RuntimeError> + 'static,
            ) -> Self {
                self.0.modified = Some(Box::new(modified));
                self
            }
            /// Reports when the field refuses a value the application wrote
            /// to its binding, with the reason, and with `None` when an
            /// acceptable value ends the refusal.
            ///
            /// A refused value, such as text longer than
            /// [`TextEditError::MAX_TEXT_BYTES`] or a control character the
            /// field does not accept, leaves the field showing its previous
            /// text, marked as refused and accepting no edits, as the type
            /// describes; this callback is how the application learns of it,
            /// for example to show a [`FieldStatus`](crate::FieldStatus) or
            /// shorten the text. It concerns only the application's own
            /// writes: refused typing and a normalizer's rejection, reported by
            /// [`Self::on_rejection`], never call it. A refusal that starts and
            /// ends within one flush reports nothing, and a later different
            /// reason reports again.
            ///
            /// The callback runs in an event phase after the flush that saw
            /// the change has settled, as [`Self::on_modified`] does, and may
            /// write bindings; its error is returned from the flush and does
            /// not undo the refusal. Its reports keep their order with the
            /// field's other reports, such as [`Self::on_modified`], and with
            /// other components' notifications. Describing the field again
            /// replaces the callback; a description without one stops the
            /// reports, and installing one on a field that already refuses its
            /// value reports that refusal.
            ///
            /// ```
            /// # use pert_ui_runtime as pert_ui;
            /// use pert_ui::{IntoElement, Runtime, TextArea, TextEditError, TextField};
            /// use std::time::Duration;
            /// let mut ui = Runtime::new();
            /// let body = ui.signal(String::new());
            /// let problem = ui.signal(None::<TextEditError>);
            /// let report = problem.clone();
            #[doc = concat!(" let editor = ", stringify!($name), "::new(&body)")]
            ///     .on_binding_refusal(move |_, refusal| report.set(refusal.cloned()));
            /// ui.mount_tree(editor.key("body"))?;
            /// ui.flush(Duration::ZERO)?;
            /// body.set("x".repeat(TextEditError::MAX_TEXT_BYTES + 1))?;
            /// ui.flush(Duration::ZERO)?;
            /// assert_eq!(problem.get()?, Some(TextEditError::TooLarge));
            /// body.set("short".to_owned())?;
            /// ui.flush(Duration::ZERO)?;
            /// assert_eq!(problem.get()?, None);
            /// # Ok::<(), pert_ui::RuntimeError>(())
            /// ```
            pub fn on_binding_refusal(
                mut self,
                refusal: impl FnMut(
                    &mut EventContext,
                    Option<&TextEditError>,
                ) -> Result<(), RuntimeError>
                + 'static,
            ) -> Self {
                self.0.refusal = Some(Box::new(refusal));
                self
            }
        }
        impl Component for $name {
            /// Applies the new binding, style, permissions, normalizer and
            /// callbacks; a different binding ends the edit in progress
            /// unsaved, as described on the type.
            fn reconcile(&mut self, next: Self) -> Option<Invalidation> {
                self.0.adopt(next.0)
            }
            fn focusable(&self) -> bool {
                self.0.enabled
            }
            fn conceals_input(&self) -> bool {
                self.0.concealed
            }
            fn cursor(&self, point: Point) -> Option<CursorIcon> {
                self.0.cursor(point)
            }
            fn text_input_state(&self) -> Option<TextInputState> {
                self.0.input_state()
            }
            fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
                self.0.update(cx)
            }
            fn measure(
                &self,
                cx: &MeasureContext<'_>,
                available: Constraints,
            ) -> Result<Size, RuntimeError> {
                self.0.measure(cx, available)
            }
            fn take_children(&mut self) -> Vec<Element> {
                self.0.menu_children()
            }
            fn child_visible(&self, key: &str) -> bool {
                key == "menu" && self.0.menu_shown()
            }
            fn child_surface(&self, key: &str) -> Option<SurfaceKind> {
                (key == "menu").then_some(SurfaceKind::ContextMenu)
            }
            fn dismiss_surface(
                &mut self,
                _: &mut EventContext,
                _: SurfaceDismissal,
            ) -> Result<(), RuntimeError> {
                self.0.close_menu()
            }
            fn arrange(&self, cx: &mut ArrangeContext<'_>, size: Size) -> Result<(), RuntimeError> {
                self.0.place_adornments(cx.measurement(), size)?;
                self.0.arrange(cx.measurement(), size)?;
                self.0.arrange_menu(cx, size)
            }
            fn paint(&self, p: &mut Painter) -> Result<(), DrawError> {
                self.0.paint(p)
            }
            fn hover_changed(&mut self, cx: &mut EventContext, hovered: bool) {
                self.0.set_hover(cx, hovered);
            }
            fn text_key_event(
                &mut self,
                cx: &mut EventContext,
                input: CommandKeyInput,
                transition: KeyTransition,
                _: &KeyboardState,
            ) -> Result<TextKeyOutcome, RuntimeError> {
                let outcome = self.0.key(cx, input.editing_event(transition));
                self.0.note_modified();
                if outcome? == InputOutcome::Handled {
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
                let outcome = self.0.key(
                    cx,
                    KeyEvent {
                        code: keys.control_key(e),
                        ..e
                    },
                );
                self.0.note_modified();
                outcome
            }
            fn text_command(
                &mut self,
                cx: &mut EventContext,
                command: TextCommand,
            ) -> Result<TextCommandOutcome, RuntimeError> {
                let outcome = self.0.command(cx, command);
                self.0.note_modified();
                outcome
            }
            fn text_input(
                &mut self,
                cx: &mut EventContext,
                text: &str,
            ) -> Result<(), RuntimeError> {
                let outcome = self.0.insert(cx, text).map(|_| ());
                self.0.note_modified();
                outcome
            }
            fn text_preedit(
                &mut self,
                cx: &mut EventContext,
                text: &TextPreedit,
            ) -> Result<(), RuntimeError> {
                self.0.compose(cx, text)
            }
            fn pointer_input(
                &mut self,
                cx: &mut EventContext,
                e: PointerEvent,
            ) -> Result<InputOutcome, RuntimeError> {
                let outcome = self.0.pointer(cx, e);
                self.0.note_modified();
                outcome
            }
            fn deliver_notifications(&mut self, cx: &mut EventContext) -> Result<(), RuntimeError> {
                self.0.deliver_report(cx)
            }
            fn focus_lost(&mut self, cx: &mut EventContext) -> Result<(), RuntimeError> {
                // Release local input first, as the default does, so an
                // uncommitted composition is dropped rather than saved. Only a
                // normalizing editor then holds a draft to save.
                self.0.reset();
                // Revealed concealed text is hidden again when focus leaves.
                self.0.set_revealed(cx, false)?;
                cx.invalidate(Invalidation::Paint);
                let outcome = if self.0.session.is_some() {
                    self.0.complete(cx, EditorTrigger::FocusLeft).map(|_| ())
                } else {
                    self.0.save(cx).map(|_| ())
                };
                self.0.note_modified();
                outcome
            }
            fn tab_navigation(
                &mut self,
                cx: &mut EventContext,
                direction: crate::FocusDirection,
                transition: crate::KeyTransition,
            ) -> Result<InputOutcome, RuntimeError> {
                let outcome = self.0.session_tab(cx, direction, transition);
                self.0.note_modified();
                outcome
            }
            fn visibility_changed(&mut self, visible: bool) {
                if !visible {
                    self.0.discard();
                    self.0.conceal_now();
                    self.0.note_modified();
                }
            }
            fn cancel_input(&mut self, cx: &mut EventContext) {
                // Any loss of the input context, including a modal taking
                // focus without `focus_lost`, hides revealed concealed text.
                self.input_reset_keeping_layout(cx);
                cx.invalidate(Invalidation::Paint);
            }
            fn input_reset(&mut self) {
                self.0.reset();
                self.0.conceal_now();
            }
            fn inspect(&self) -> ComponentDetails {
                self.0.inspect()
            }
        }
        impl $name {
            /// Releases local input and conceals revealed text, keeping the
            /// caret where layout allows.
            fn input_reset_keeping_layout(&mut self, cx: &mut EventContext) {
                self.0.reset();
                if self.0.set_revealed(cx, false).is_err() {
                    self.0.conceal_now();
                }
            }
        }
    };
}
editor_api!(TextField, false);
editor_api!(TextArea, true);
impl TextField {
    /// Makes this field the editor of a host's [`EditorSession`], such as a
    /// table cell's: it attaches to the edit open now
    /// ([`EditorSession::attach`]), so the host calls
    /// [`EditorSession::begin`] first.
    ///
    /// With a session the field is in draft mode whether or not it has a
    /// normalizer or limits, applies the edit's [`EditorStart`] from its next
    /// delivery once the host has focused it, and keeps Enter, Tab and
    /// Escape, reporting each once:
    ///
    /// - Enter and an unmodified Tab or Shift+Tab save as Enter does
    ///   (normalizer, length limits, required minimum, `on_save`) and report
    ///   [`EditorOutcome::Saved`], [`EditorOutcome::Unchanged`] (nothing to
    ///   write, a normalized text equal to the value, or an application write
    ///   that replaced the draft) or [`EditorOutcome::Refused`] (the draft
    ///   stays for correction, as does focus). A read-only field with a
    ///   changed draft reports `Refused`.
    /// - Escape discards the draft in one press and reports
    ///   [`EditorOutcome::Cancelled`]; no surface or ancestor sees it.
    /// - While a composition or a selection drag is active Enter and Tab are
    ///   kept without saving, Escape ends the composition or drag only, and
    ///   nothing is reported.
    /// - Focus leaving by other means saves as focus loss does and reports
    ///   [`EditorTrigger::FocusLeft`] with the outcome, unless an earlier
    ///   report already ended the edit. A hidden native window clears focus
    ///   too, and its report is delivered once the window is shown again,
    ///   as hidden windows hold notifications. Hiding and removal of the
    ///   field itself report nothing, and so does disabling it: revoking
    ///   [`Self::enabled`] releases focus as cancelled input, which keeps the
    ///   draft unsaved and the edit open for the host to end.
    ///
    /// The report comes before `on_save` runs; see [`EditorSession`] for
    /// delivery. Replaces an earlier session. Ordinary bubbling, traversal
    /// and command bindings see none of these keys while a session is set.
    pub fn session(mut self, session: &EditorSession) -> Self {
        self.0.session = Some(session.attach());
        self
    }
    /// Conceals the text, as for a password: each grapheme cluster is shown as
    /// one dot, and the field closes every way the content could leak.
    ///
    /// - The caret and selection move over the dots, so glyph widths reveal
    ///   nothing; a double click selects everything rather than a word, and a
    ///   word deletion (Ctrl+Backspace, Option+Backspace on macOS) removes
    ///   everything before or after the caret.
    /// - Copy and Cut are unavailable, by shortcut, context menu and
    ///   [`TextCommand`]; Paste Before and Paste After too. Paste still
    ///   works. The field offers no drag out of its text.
    /// - The field asks for no IME composition, so no candidate window shows
    ///   the text; typed key text arrives as committed text.
    /// - Inspection reports no `text` and no `selection`, only `length` (the
    ///   grapheme count) and `concealed`; semantics are
    ///   [protected](Semantics::protected) with no value; event history
    ///   records none of the keys typed into it
    ///   ([`Component::conceals_input`]).
    ///
    /// The binding holds the real text. Default: not concealed. Turning
    /// concealment on or off on a mounted field moves the caret to the start.
    ///
    /// ```
    /// # use pert_ui_runtime as pert_ui;
    /// use pert_ui::{Element, Runtime, TextField};
    /// let mut ui = Runtime::new();
    /// let password = ui.signal(String::new());
    /// let field = ui.mount_tree(Element::new("password",
    ///     TextField::new(password.clone()).concealed(true).revealable(true)))?;
    /// ui.focus(&field)?;
    /// ui.text_input("hunter2")?;
    /// assert_eq!(password.get()?, "hunter2");
    /// let details = ui.inspect().components.into_iter()
    ///     .find(|c| c.key == "password").unwrap().details;
    /// assert!(details.semantics.unwrap().protected);
    /// assert!(!details.properties.contains_key("text"));
    /// # Ok::<(), pert_ui::RuntimeError>(())
    /// ```
    pub fn concealed(mut self, concealed: bool) -> Self {
        self.0.concealed = concealed;
        self
    }
    /// Adds a reveal button (an eye icon) at the end of a
    /// [concealed](Self::concealed) field that shows the text in the clear
    /// while it is on. It belongs to the field's single Tab stop: click it, or
    /// press Alt+F8 in the focused field (as in Windows password boxes), to
    /// toggle. The text is concealed again when the field loses focus, is
    /// hidden or is removed, and when the button is withdrawn by a new
    /// description. While revealed, copying, composition and inspection stay
    /// as for concealed text. Inspection reports `revealed`. Default: no
    /// button; it has no effect on a field that is not concealed.
    pub fn revealable(mut self, revealable: bool) -> Self {
        self.0.revealable = revealable;
        self
    }
    /// Adds a clear button (a × icon) at the end of the field that empties
    /// the text, as the search field's does. It belongs to the field's single
    /// Tab stop and acts on a primary press. It shows only while the field
    /// has text and is editable (never read-only or disabled, and not during
    /// a composition), but its slot is always reserved so the text never
    /// reflows. Clearing is one undoable edit through the same path as
    /// typing: written at once without draft mode, and kept as a draft until
    /// it is saved in draft mode. It works on a [concealed](Self::concealed)
    /// field too. From the keyboard, select all and delete. Inspection
    /// reports `clear_shown`. Default: no button.
    ///
    /// ```
    /// # use pert_ui_runtime as pert_ui;
    /// use pert_ui::{Element, Runtime, TextField};
    /// let mut ui = Runtime::new();
    /// let query = ui.signal("draft".to_owned());
    /// ui.mount_tree(Element::new("name", TextField::new(query.clone()).clearable(true)))?;
    /// ui.flush(std::time::Duration::ZERO)?;
    /// let details = ui.inspect().components.into_iter()
    ///     .find(|c| c.key == "name").unwrap().details;
    /// assert_eq!(details.properties["clear_shown"], pert_ui::InspectValue::Bool(true));
    /// # Ok::<(), pert_ui::RuntimeError>(())
    /// ```
    pub fn clearable(mut self, clearable: bool) -> Self {
        self.0.clearable = clearable;
        self
    }
    /// Shows an icon before the text, Muted and at `IconSize`, such as a
    /// person before a user name. It is presentation only: the pointer edits
    /// the text as elsewhere in the field, and the icon is not part of the
    /// value or the name. Inspection reports `leading_icon`.
    pub fn leading_icon(mut self, icon: impl Into<IconSource>) -> Self {
        self.0.leading = Some(icon.into());
        self
    }
    /// Shows Muted text after the editing area, such as a unit or a domain
    /// ("ms", "@example.com"), in the field's text style. It is presentation
    /// only, never part of the value, and stays put while the text scrolls; it
    /// takes at most half of the room left for the text and is clipped beyond
    /// that. Inspection reports `suffix`. Empty, the default, shows none.
    pub fn suffix(mut self, text: impl Into<String>) -> Self {
        self.0.suffix = text.into();
        self
    }
    /// Invokes a UI-thread callback with the current committed text on a fresh
    /// Enter press. No implicit save/undo occurs; callback errors propagate.
    /// With [`Self::normalize`], Enter first saves the draft, and the callback
    /// runs only when that save succeeds, with the saved text.
    pub fn on_submit(
        mut self,
        submit: impl FnMut(&mut EventContext, &str) -> Result<(), RuntimeError> + 'static,
    ) -> Self {
        self.0.submit = Some(Box::new(submit));
        self
    }
}

struct Prepared {
    layout: TextLayout,
    inner: Rect,
    origin: [f32; 2],
}
#[derive(Clone)]
struct Snapshot<M> {
    payload: M,
    text: String,
    selection: TextSelection,
    /// The selection's offsets are in the concealed form of `text`.
    masked: bool,
}
// Wrapper-owned edit state travels with text through undo. Plain text uses (),
// numeric editors retain the exact value represented by a rounded step display.
/// The built-in clipboard menu a bound editor owns.
///
/// Availability is decided once when the menu opens, from the selection and one
/// clipboard read, and published through bindings the retained list reevaluates.
pub(crate) struct FieldMenu {
    pub(crate) open: Signal<bool>,
    /// Last value read through the editor's update, so `child_visible` reports a
    /// subscribed decision. An untracked read here would never reevaluate, and the
    /// menu would stay on screen after an action closed it.
    pub(crate) shown: bool,
    pub(crate) available: Vec<(crate::TextCommand, Signal<bool>)>,
    /// Filled by the editor's first update, so the actions can name their target.
    ///
    /// Weak on purpose: the menu's actions live in the editor's own child subtree,
    /// so holding the editor's node here would keep the mount alive forever.
    pub(crate) target: Rc<RefCell<Option<std::rc::Weak<crate::runtime::Node>>>>,
    pub(crate) position: Cell<Option<Point>>,
    pub(crate) items: Option<Element>,
}

impl FieldMenu {
    /// The five clipboard actions, in the order the menu shows them.
    const ACTIONS: [(crate::TextCommand, &'static str, crate::StringRole); 5] = [
        (
            crate::TextCommand::Copy,
            "copy",
            crate::StringRole::TextCopy,
        ),
        (crate::TextCommand::Cut, "cut", crate::StringRole::TextCut),
        (
            crate::TextCommand::Paste,
            "paste",
            crate::StringRole::TextPaste,
        ),
        (
            crate::TextCommand::PasteBefore,
            "paste-before",
            crate::StringRole::TextPasteBefore,
        ),
        (
            crate::TextCommand::PasteAfter,
            "paste-after",
            crate::StringRole::TextPasteAfter,
        ),
    ];

    fn new(value: &Signal<String>) -> Result<Self, ChoiceError> {
        let strings = value.shared().strings.borrow().clone();
        Self::with_actions(&Self::ACTIONS, &strings, |initial| value.sibling(initial))
    }
    /// A menu of `actions`, with its state in signals `signal` creates.
    pub(crate) fn with_actions(
        actions: &[(crate::TextCommand, &'static str, crate::StringRole)],
        strings: &crate::UiStrings,
        mut signal: impl FnMut(bool) -> Signal<bool>,
    ) -> Result<Self, ChoiceError> {
        let open = signal(false);
        let target: Rc<RefCell<Option<std::rc::Weak<crate::runtime::Node>>>> =
            Rc::new(RefCell::new(None));
        let mut available = Vec::new();
        let mut items = Vec::new();
        for &(command, key, role) in actions {
            let enabled = signal(false);
            available.push((command, enabled.clone()));
            let target = target.clone();
            // The rows follow the installed string table while the menu is
            // retained; the current text is the fallback.
            let label = TextSource::library(role, strings.text(role).to_owned());
            let item = MenuItem::new(key, label, move |cx: &mut EventContext| {
                // The action names its editor explicitly; the runtime delivers
                // it in this flush's event phase, so focus is never moved.
                if let Some(node) = target.borrow().as_ref().and_then(std::rc::Weak::upgrade) {
                    cx.request_text_command(&WidgetId(node), command)?;
                }
                Ok(())
            })
            .enabled(enabled.read_only());
            items.push(item);
        }
        Ok(Self {
            items: Some(crate::menu::owned_menu(&open, items)?),
            open,
            shown: false,
            available,
            target,
            position: Cell::new(None),
        })
    }
}

/// Places an open clipboard menu at its request position, or below its owner
/// for a keyboard request, clamped to the viewport, the way every other
/// context menu is placed.
pub(crate) fn place_menu(
    cx: &mut ArrangeContext<'_>,
    size: Size,
    menu: Option<&FieldMenu>,
) -> Result<(), RuntimeError> {
    if cx.is_empty() {
        return Ok(());
    }
    let viewport = cx.measurement().viewport();
    let wanted = cx.measure(
        0,
        Constraints::new(Some(viewport.width()), Some(viewport.height()))?,
    )?;
    let (x, y) = menu
        .and_then(|menu| menu.position.get())
        .map_or((0.0, size.height()), |p| (p.x(), p.y()));
    cx.place(
        0,
        rect(
            x,
            y,
            wanted.width().min(viewport.width()),
            wanted.height().min(viewport.height()),
        )?,
    )
}

/// Whether the platform's command modifier is held: Command on macOS, Control
/// without Alt elsewhere. The editors and `SelectableText` share it.
pub(crate) fn command_modifier(m: crate::Modifiers) -> bool {
    if cfg!(target_os = "macos") {
        m.super_key
    } else {
        m.control && !m.alt
    }
}

/// Whether the platform's word-movement modifier is held: Option on macOS,
/// Control without Alt elsewhere.
pub(crate) fn word_modifier(m: crate::Modifiers) -> bool {
    if cfg!(target_os = "macos") {
        m.alt
    } else {
        m.control && !m.alt
    }
}

/// The selection movement a navigation key asks for, as the editors map it:
/// arrows by grapheme or, with the word modifier, by word; Up and Down by
/// line; Home and End to the line's ends; with the command modifier, to the
/// text's ends (and on macOS Command+Left/Right to the line's ends). Shift is
/// the caller's: it extends instead of moving.
pub(crate) fn key_movement(code: KeyCode, m: crate::Modifiers) -> Option<TextMovement> {
    let (command, word) = (command_modifier(m), word_modifier(m));
    match code {
        KeyCode::ArrowLeft if cfg!(target_os = "macos") && command => Some(TextMovement::LineStart),
        KeyCode::ArrowRight if cfg!(target_os = "macos") && command => Some(TextMovement::LineEnd),
        KeyCode::ArrowLeft => Some(if word {
            TextMovement::WordLeft
        } else {
            TextMovement::Left
        }),
        KeyCode::ArrowRight => Some(if word {
            TextMovement::WordRight
        } else {
            TextMovement::Right
        }),
        KeyCode::ArrowUp => Some(if command {
            TextMovement::Start
        } else {
            TextMovement::Up
        }),
        KeyCode::ArrowDown => Some(if command {
            TextMovement::End
        } else {
            TextMovement::Down
        }),
        KeyCode::Home => Some(if command {
            TextMovement::Start
        } else {
            TextMovement::LineStart
        }),
        KeyCode::End => Some(if command {
            TextMovement::End
        } else {
            TextMovement::LineEnd
        }),
        _ => None,
    }
}

/// What applying permissions changed, so an owner can end its own work.
#[derive(Clone, Copy, Debug, Default)]
pub(super) struct Availability {
    /// The effective state or the permissions changed.
    pub(super) changed: bool,
    /// The editor was enabled and is not now.
    pub(super) disabled: bool,
    /// The editor accepted edits and does not now.
    pub(super) locked: bool,
}

pub(super) struct Editor<M = ()> {
    area: Cell<Option<Rect>>,
    bar_extents: Cell<scrollbar::Extents>,
    embedded: bool,
    /// A single line shorter than the editing area sits flush with its end,
    /// as a right-aligned value display does.
    align_end: bool,
    active: bool,
    payload: M,
    value: Option<Signal<String>>,
    text: String,
    initialized: bool,
    multi: bool,
    style: TextStyle,
    /// Whether the editor takes focus and input now: `enablement` as last read.
    enabled: bool,
    /// Whether edits are refused now: `editability` as last read, or a lock
    /// its owner adds (`owner_lock`).
    read_only: bool,
    /// The described availability; see [`Editor::read_permissions`].
    enablement: Permission,
    /// The described editability; see [`Editor::read_permissions`].
    editability: Permission,
    /// Whether `editability` allowed edits when last read.
    editable: bool,
    /// Edits refused by the owner for its own reasons, such as a parameter
    /// that is not editable or a refused bound value.
    owner_lock: bool,
    /// Whether an update revoked `enablement` since the owner last asked, for
    /// an owner with pending work to discard, such as a debounced query.
    revoked: bool,
    /// Why the bound value was last refused, while it is refused. The editor then
    /// keeps showing its previous text and accepts no edits until the binding
    /// holds acceptable text again.
    rejected: Option<TextEditError>,
    focused: bool,
    submit: Option<Submit>,
    /// The application's commit-time adjustment. A bound editor with one is in
    /// draft mode: edits stay local until a save writes the normalized text.
    normalizer: Option<Normalizer<String>>,
    /// Edits stay local without a normalizer; the owner writes the binding
    /// itself, as a debounced search field does when its deadline passes.
    deferred: bool,
    /// The bound text as last observed or written, which separates an external
    /// write from the editor's own draft.
    observed: String,
    /// Why the last save refused the draft, until the draft changes.
    rejection: Option<EntryError>,
    report: RejectionReport,
    /// The application's callback after a save wrote the binding.
    saved: SaveReport<str>,
    selection: Cell<TextSelection>,
    prepared: RefCell<Option<Prepared>>,
    scroll: Cell<[f32; 2]>,
    reveal: Cell<bool>,
    bar_drag: Option<f32>,
    /// The pointer rests over the scrollbar, presented as draggable.
    bar_hover: bool,
    hovered: bool,
    preedit: Option<(TextPreedit, Range<usize>)>,
    ime_session: Option<u64>,
    dragging: bool,
    clicks: u8,
    /// The pointer's press series; `clicks` cycles through it.
    click_series: crate::ClickCounter,
    blink: bool,
    blink_at: Option<Duration>,
    restart_blink: bool,
    undo: VecDeque<Snapshot<M>>,
    redo: VecDeque<Snapshot<M>>,
    menu: Option<FieldMenu>,
    /// Muted hint shown while the text is empty and nothing is composed.
    placeholder: Caption,
    /// The application requires a value: semantics report it, and an empty
    /// value is then subject to `min_length`.
    required: bool,
    /// Grapheme-counted length limits checked when a draft is saved; either
    /// one puts a bound editor in draft mode.
    min_length: Option<usize>,
    max_length: Option<usize>,
    /// Shows the grapheme count against the limit inside the field.
    counter: bool,
    /// The content is secret: shown as one dot per grapheme, never copied,
    /// composed by an IME, inspected or recorded.
    concealed: bool,
    /// A reveal button inside the field shows the concealed text on request.
    revealable: bool,
    /// The concealed text is currently shown in the clear.
    revealed: bool,
    reveal_rect: Cell<Option<Rect>>,
    reveal_hover: bool,
    counter_rect: Cell<Option<Rect>>,
    /// A clear button at the end of a single-line field empties the text.
    clearable: bool,
    clear_rect: Cell<Option<Rect>>,
    clear_hover: bool,
    /// An icon before the text, presentation only.
    leading: Option<IconSource>,
    leading_rect: Cell<Option<Rect>>,
    /// Muted text after the editing area, such as a unit; presentation only.
    suffix: String,
    suffix_rect: Cell<Option<Rect>>,
    /// Paints a mark while an unsaved draft differs from the saved value.
    modified_indicator: bool,
    /// The application's callback when the draft starts or stops differing
    /// from the saved value, delivered through `notifier`.
    modified: Option<ModifiedReport>,
    /// The modified state last delivered to `modified`.
    modified_reported: bool,
    /// A delivery is requested and not yet made.
    modified_pending: Cell<bool>,
    notifier: Option<crate::Notifier>,
    /// The table's description of an unsaved draft, read in update.
    modified_text: String,
    /// The application's callback when its bound value is refused or the
    /// refusal ends, delivered through `notifier`.
    refusal: Option<RefusalReport>,
    /// The refusal last delivered to `refusal`.
    refusal_reported: Option<TextEditError>,
    /// A refusal delivery is requested and not yet made.
    refusal_pending: Cell<bool>,
    /// The report each requested delivery is for, oldest first: one entry per
    /// notifier request, so each delivery makes its own report in order.
    reports: RefCell<std::collections::VecDeque<Report>>,
    /// The host's edit this editor reports into ([`TextField::session`]).
    session: Option<EditorAttachment>,
    /// What the save in progress completes, reported once at its outcome.
    pending_trigger: Option<EditorTrigger>,
    /// A start delivery is requested and not yet made.
    start_pending: Cell<bool>,
}
/// The character a concealed editor shows for each grapheme of its text.
const CONCEAL_DOT: &str = "\u{2022}";
/// The concealed form of `text`: one dot per grapheme cluster.
fn concealed_text(text: &str) -> String {
    CONCEAL_DOT.repeat(text.graphemes(true).count())
}
/// The offset in [`concealed_text`] of the grapheme boundary at or after the
/// byte offset `real` of `text`.
fn concealed_offset(text: &str, real: usize) -> usize {
    text.grapheme_indices(true)
        .take_while(|(index, _)| *index < real)
        .count()
        * CONCEAL_DOT.len()
}
/// The byte offset in `text` of the grapheme a concealed offset points at.
fn revealed_offset(text: &str, shown: usize) -> usize {
    text.grapheme_indices(true)
        .nth(shown / CONCEAL_DOT.len())
        .map_or(text.len(), |(index, _)| index)
}
impl<M: Copy + Default + PartialEq> Editor<M> {
    fn new(value: Option<Signal<String>>, multi: bool) -> Self {
        // The keys in `FieldMenu::ACTIONS` are distinct constants, so validation
        // cannot fail; `field_menu_is_always_constructible` keeps a later edit
        // from silently leaving a bound editor without its menu.
        let menu = value.as_ref().and_then(|v| FieldMenu::new(v).ok());
        Self {
            payload: M::default(),
            value,
            text: String::new(),
            initialized: false,
            multi,
            style: if multi {
                TextStyle::Monospace
            } else {
                TextStyle::Body
            },
            enabled: true,
            read_only: false,
            enablement: Permission::allowed(),
            editability: Permission::allowed(),
            editable: true,
            owner_lock: false,
            revoked: false,
            rejected: None,
            focused: false,
            submit: None,
            normalizer: None,
            deferred: false,
            observed: String::new(),
            rejection: None,
            report: RejectionReport::default(),
            saved: SaveReport::default(),
            selection: Cell::default(),
            prepared: RefCell::new(None),
            area: Cell::new(None),
            bar_extents: Cell::new(scrollbar::Extents::STANDARD),
            embedded: false,
            align_end: false,
            active: true,
            scroll: Cell::new([0.0; 2]),
            reveal: Cell::new(true),
            bar_drag: None,
            bar_hover: false,
            hovered: false,
            preedit: None,
            ime_session: Some(0),
            dragging: false,
            clicks: 0,
            click_series: crate::ClickCounter::new(),
            blink: true,
            blink_at: None,
            restart_blink: true,
            menu,
            undo: VecDeque::new(),
            redo: VecDeque::new(),
            placeholder: Caption::new(TextSource::default()),
            required: false,
            min_length: None,
            max_length: None,
            counter: false,
            concealed: false,
            revealable: false,
            revealed: false,
            reveal_rect: Cell::new(None),
            reveal_hover: false,
            counter_rect: Cell::new(None),
            clearable: false,
            clear_rect: Cell::new(None),
            clear_hover: false,
            leading: None,
            leading_rect: Cell::new(None),
            suffix: String::new(),
            suffix_rect: Cell::new(None),
            modified_indicator: false,
            modified: None,
            modified_reported: false,
            modified_pending: Cell::new(false),
            notifier: None,
            modified_text: String::new(),
            refusal: None,
            refusal_reported: None,
            refusal_pending: Cell::new(false),
            reports: RefCell::new(std::collections::VecDeque::new()),
            session: None,
            pending_trigger: None,
            start_pending: Cell::new(false),
        }
    }
    /// Whether the text is shown as dots: concealed and not revealed.
    fn masked(&self) -> bool {
        self.concealed && !self.revealed
    }
    /// `text` as this editor lays it out and selects in it.
    fn shown<'a>(&self, text: &'a str) -> std::borrow::Cow<'a, str> {
        if self.masked() {
            std::borrow::Cow::Owned(concealed_text(text))
        } else {
            std::borrow::Cow::Borrowed(text)
        }
    }
    /// A byte offset of `text` as an offset in its shown form.
    fn shown_offset(&self, text: &str, real: usize) -> usize {
        if self.masked() {
            concealed_offset(text, real)
        } else {
            real
        }
    }
    /// The selection, which lives in the shown text, as a range of `self.text`.
    fn real_range(&self) -> Range<usize> {
        let range = self.selection.get().byte_range();
        if self.masked() {
            revealed_offset(&self.text, range.start)..revealed_offset(&self.text, range.end)
        } else {
            range
        }
    }
    /// Lays out `text` as shown.
    fn layout_of(&self, cx: &EventContext, text: &str) -> Result<TextLayout, RuntimeError> {
        cx.text(&self.shown(text), self.width(), self.style)
    }
    /// Whether a length limit is set.
    fn limited(&self) -> bool {
        self.min_length.is_some() || self.max_length.is_some()
    }
    /// Checks a normalized draft against the length limits, counting
    /// grapheme clusters. An empty value is subject to the minimum only when
    /// the editor is required, so an optional field may stay empty.
    /// The refusal message comes from the installed
    /// [`UiStrings`](crate::UiStrings).
    fn check_length(&self, strings: &crate::UiStrings, text: &str) -> Result<(), InputRejection> {
        let count = text.graphemes(true).count();
        if let Some(min) = self.min_length
            && count < min
            && (count > 0 || self.required)
        {
            return Err(InputRejection::new(
                strings.format(crate::UiMessage::AtLeastCharacters { count: min }),
            ));
        }
        if let Some(max) = self.max_length
            && count > max
        {
            return Err(InputRejection::new(
                strings.format(crate::UiMessage::AtMostCharacters { count: max }),
            ));
        }
        Ok(())
    }
    /// Shows or conceals the text again, keeping the caret and selection on
    /// the same graphemes.
    fn set_revealed(&mut self, cx: &mut EventContext, revealed: bool) -> Result<(), RuntimeError> {
        if !self.concealed || self.revealed == revealed {
            return Ok(());
        }
        let selection = self.selection.get();
        let masked = self.masked();
        let real = |offset| {
            if masked {
                revealed_offset(&self.text, offset)
            } else {
                offset
            }
        };
        let (anchor, focus) = (real(selection.anchor_byte()), real(selection.focus_byte()));
        self.revealed = revealed;
        let layout = self.layout_of(cx, &self.text)?;
        self.selection.set(TextSelection::new(
            &layout,
            self.shown_offset(&self.text, anchor),
            self.shown_offset(&self.text, focus),
        ));
        self.changed(cx);
        Ok(())
    }
    /// Conceals revealed text where no layout is available, such as when the
    /// editor is hidden; the caret returns to the start.
    fn conceal_now(&mut self) {
        if self.revealed {
            self.revealed = false;
            self.selection.set(TextSelection::default());
            self.reveal.set(true);
        }
    }
    fn validate(&self, text: &str) -> Result<(), RuntimeError> {
        check_text(text, self.multi).map_err(Into::into)
    }
    /// Whether edits are refused: a read-only field, or a refused binding.
    fn locked(&self) -> bool {
        self.read_only || self.rejected.is_some()
    }
    /// Whether edits stay local until a save: a bound editor with a
    /// normalizer or a length limit.
    fn drafting(&self) -> bool {
        (self.normalizer.is_some() || self.limited() || self.session.is_some())
            && self.value.is_some()
    }
    /// Whether a draft differs from the bound text.
    fn dirty(&self) -> bool {
        self.drafting() && self.text != self.observed
    }
    /// Requests one delivery to the modified callback when the draft's
    /// modified state differs from the state last delivered. Callable in any
    /// phase; the callback runs in the event phase of the same flush.
    fn note_modified(&self) {
        if self.modified.is_some()
            && !self.modified_pending.get()
            && self.dirty() != self.modified_reported
            && let Some(notifier) = &self.notifier
        {
            self.modified_pending.set(true);
            self.reports.borrow_mut().push_back(Report::Modified);
            notifier.request();
        }
    }
    /// Requests one delivery to the refusal callback when the refusal differs
    /// from the one last delivered.
    fn note_refusal(&self) {
        if self.refusal.is_some()
            && !self.refusal_pending.get()
            && self.rejected != self.refusal_reported
            && let Some(notifier) = &self.notifier
        {
            self.refusal_pending.set(true);
            self.reports.borrow_mut().push_back(Report::Refusal);
            notifier.request();
        }
    }
    /// Makes the report the oldest requested delivery is for.
    fn deliver_report(&mut self, cx: &mut EventContext) -> Result<(), RuntimeError> {
        let report = self.reports.borrow_mut().pop_front();
        match report {
            Some(Report::Modified) => self.deliver_modified(cx),
            Some(Report::Refusal) => self.deliver_refusal(cx),
            Some(Report::Start) => self.apply_start(cx),
            None => Ok(()),
        }
    }
    /// Delivers a changed refusal to the application's callback. Without a
    /// callback nothing counts as reported, so one installed later still
    /// learns of the current refusal.
    fn deliver_refusal(&mut self, cx: &mut EventContext) -> Result<(), RuntimeError> {
        self.refusal_pending.set(false);
        let Some(report) = &mut self.refusal else {
            return Ok(());
        };
        if self.rejected == self.refusal_reported {
            return Ok(());
        }
        self.refusal_reported.clone_from(&self.rejected);
        report(cx, self.refusal_reported.as_ref())
    }
    /// Delivers a changed modified state to the application's callback.
    fn deliver_modified(&mut self, cx: &mut EventContext) -> Result<(), RuntimeError> {
        self.modified_pending.set(false);
        let now = self.dirty();
        if now == self.modified_reported {
            return Ok(());
        }
        self.modified_reported = now;
        match &mut self.modified {
            Some(report) => report(cx, now),
            None => Ok(()),
        }
    }
    /// Writes edited text to the binding, except in draft mode, where only a
    /// save writes.
    fn publish_text(&mut self, text: &str) -> Result<(), RuntimeError> {
        if let Some(value) = &self.value
            && !self.drafting()
            && !self.deferred
        {
            value.set(text.to_owned())?;
            self.observed = text.to_owned();
        }
        Ok(())
    }
    /// Saves on an explicit request, Enter: as [`Self::save`], except that an
    /// unchanged draft is still checked against the length limits, so an
    /// empty required field with a minimum length is refused rather than
    /// submitted. Focus loss keeps [`Self::save`], so moving through an
    /// untouched field reports nothing.
    fn submit_save(&mut self, cx: &mut EventContext) -> Result<bool, RuntimeError> {
        if let Some(value) = self.value.clone().filter(|_| self.drafting())
            && self.enabled
            && !self.locked()
            && self.text == self.observed
            // An application write not yet observed is left to `save`.
            && value.get()? == self.observed
            && let Err(error) = self.check_length(&cx.strings(), &self.text)
        {
            let error = EntryError::from(error);
            cx.invalidate(Invalidation::Paint);
            self.rejection = Some(error.clone());
            self.report_session(EditorOutcome::Refused);
            self.report.outcome(cx, Some(&error))?;
            return Ok(false);
        }
        self.save(cx)
    }
    /// Reports the outcome of the save in progress to the editor session,
    /// once, when a session trigger is waiting for it.
    fn report_session(&mut self, outcome: EditorOutcome) {
        if let Some(trigger) = self.pending_trigger.take()
            && let Some(session) = &self.session
        {
            session.report(trigger, outcome);
        }
    }
    /// Saves for a session completion and reports its outcome: an explicit
    /// confirmation (Enter, Tab) validates as Enter does, a departure as focus
    /// loss does. Returns whether the draft was saved or found unchanged.
    fn complete(
        &mut self,
        cx: &mut EventContext,
        trigger: EditorTrigger,
    ) -> Result<bool, RuntimeError> {
        self.pending_trigger = Some(trigger);
        let result = if trigger == EditorTrigger::FocusLeft {
            self.save(cx)
        } else {
            self.submit_save(cx)
        };
        match &result {
            Ok(saved) => self.report_session(if *saved {
                EditorOutcome::Unchanged
            } else {
                EditorOutcome::Refused
            }),
            Err(_) => self.pending_trigger = None,
        }
        result
    }
    /// The keys an editor with a session keeps: Enter and Tab confirm, Escape
    /// cancels, each reported once. While a composition or selection drag is
    /// active Enter and Tab are kept without saving and Escape is left to
    /// ordinary handling, which ends the composition or drag only.
    fn session_key(
        &mut self,
        cx: &mut EventContext,
        e: KeyEvent,
    ) -> Result<Option<InputOutcome>, RuntimeError> {
        if self.session.is_none() || self.multi {
            return Ok(None);
        }
        let busy = self.preedit.is_some() || self.dragging || self.bar_drag.is_some();
        let m = cx.modifiers();
        match e.code {
            KeyCode::Escape if !busy => {
                if e.transition == KeyTransition::Pressed {
                    self.discard();
                    self.changed(cx);
                    if let Some(session) = &self.session {
                        session.report(EditorTrigger::Escape, EditorOutcome::Cancelled);
                    }
                    self.report.clear(cx)?;
                }
                Ok(Some(InputOutcome::Handled))
            }
            KeyCode::Enter => {
                if !busy
                    && e.transition == KeyTransition::Pressed
                    && self.complete(cx, EditorTrigger::Enter)?
                    && let Some(submit) = &mut self.submit
                {
                    (submit)(cx, &self.text)?;
                }
                Ok(Some(InputOutcome::Handled))
            }
            KeyCode::Tab if !(m.control || m.alt || m.super_key) => self
                .session_tab(cx, crate::FocusDirection::from_shift(m.shift), e.transition)
                .map(Some),
            _ => Ok(None),
        }
    }
    /// Tab with a session: confirms and reports, or acts on a composition or
    /// drag only; the key is kept either way. Without a session it is left
    /// to traversal.
    fn session_tab(
        &mut self,
        cx: &mut EventContext,
        direction: crate::FocusDirection,
        transition: crate::KeyTransition,
    ) -> Result<InputOutcome, RuntimeError> {
        if self.session.is_none() || self.multi {
            return Ok(InputOutcome::Ignored);
        }
        if transition == crate::KeyTransition::Pressed
            && self.preedit.is_none()
            && !self.dragging
            && self.bar_drag.is_none()
        {
            self.complete(cx, EditorTrigger::Tab { direction })?;
        }
        Ok(InputOutcome::Handled)
    }
    /// Applies the session's pending start, from the editor's own delivery.
    fn apply_start(&mut self, cx: &mut EventContext) -> Result<(), RuntimeError> {
        self.start_pending.set(false);
        let Some(start) = self.session.as_ref().and_then(EditorAttachment::take_start) else {
            return Ok(());
        };
        match start {
            EditorStart::SelectAll => self.command(cx, TextCommand::SelectAll).map(|_| ()),
            EditorStart::CaretAtEnd => {
                let layout = self.layout(cx)?;
                self.selection
                    .set(TextSelection::new(&layout, usize::MAX, usize::MAX));
                self.changed(cx);
                Ok(())
            }
            EditorStart::Typed(text) => {
                self.command(cx, TextCommand::SelectAll)?;
                self.type_start(cx, &text)
            }
        }
    }
    /// Replaces the selected value with a typed start's text, which may be
    /// empty, as one undoable edit. Text this editor cannot hold fails with
    /// its text error and leaves the value selected; a read-only editor keeps
    /// its value.
    fn type_start(&mut self, cx: &mut EventContext, text: &str) -> Result<(), RuntimeError> {
        self.validate(text)?;
        if !self.enabled || self.locked() {
            return Ok(());
        }
        self.cancel_composition();
        let range = self.real_range();
        self.replace(cx, range, text, true).map(|_| ())
    }
    /// Saves a changed draft: normalizes it, checks and prepares the result,
    /// then writes it and clears the local undo history. Returns `false` when
    /// the normalizer or the check refused the draft, which then stays for
    /// correction, and when an application write not yet observed discarded
    /// it, and while the field is read-only with an unsaved draft, which is
    /// kept but not saved. Outside draft mode, and for an unchanged draft,
    /// there is nothing to save and this returns `true`.
    fn save(&mut self, cx: &mut EventContext) -> Result<bool, RuntimeError> {
        if !self.enabled {
            return Ok(false);
        }
        let Some(value) = self.value.clone().filter(|_| self.drafting()) else {
            return Ok(true);
        };
        // An application write since the last update wins over the draft, as
        // it would have at that update.
        let current = value.get()?;
        if current != self.observed {
            if check_text(&current, self.multi).is_ok() {
                self.observed.clone_from(&current);
                self.sync_text(current);
            } else {
                self.discard();
            }
            self.rejection = None;
            self.changed(cx);
            self.report_session(EditorOutcome::Unchanged);
            self.report.clear(cx)?;
            return Ok(false);
        }
        if self.locked() || self.text == self.observed {
            self.report_session(if self.text == self.observed {
                EditorOutcome::Unchanged
            } else {
                EditorOutcome::Refused
            });
            self.report.clear(cx)?;
            // A read-only field keeps an unsaved draft, which is not saved, so
            // the save does not report success for it. (Its key handling
            // already refuses Enter while locked.)
            return Ok(self.text == self.observed);
        }
        let normalized = match &self.normalizer {
            Some(normalizer) => normalize_text(normalizer, self.text.clone(), self.multi),
            None => Ok(self.text.clone()),
        }
        .and_then(|text| {
            self.check_length(&cx.strings(), &text)
                .map(|()| text)
                .map_err(EntryError::from)
        });
        let text = match normalized {
            Ok(text) => text,
            Err(error) => {
                // The field shows the refusal whether or not the report
                // reaches the application.
                cx.invalidate(Invalidation::Paint);
                self.rejection = Some(error.clone());
                self.report_session(EditorOutcome::Refused);
                self.report.outcome(cx, Some(&error))?;
                return Ok(false);
            }
        };
        let layout = self.layout_of(cx, &text)?;
        // A normalized text equal to the bound value writes nothing, so it
        // is no save to report.
        let wrote = value.set_changed(text.clone())?;
        self.observed.clone_from(&text);
        if text != self.text {
            self.cancel_composition();
            self.selection
                .set(TextSelection::new(&layout, usize::MAX, usize::MAX));
            self.text = text;
            self.payload = M::default();
        }
        self.undo.clear();
        self.redo.clear();
        self.rejection = None;
        self.changed(cx);
        // Reported before the application's callbacks run, so a failing
        // callback cannot lose the completion.
        self.report_session(if wrote {
            EditorOutcome::Saved
        } else {
            EditorOutcome::Unchanged
        });
        // Both notifications are attempted, so a failing rejection report
        // cannot swallow the save; the first error is returned.
        let reported = self.report.clear(cx);
        let saved = if wrote {
            let saved = self.observed.clone();
            self.saved.saved(cx, &saved)
        } else {
            Ok(())
        };
        reported.and(saved).map(|()| true)
    }
    /// Discards a draft, restoring the bound text.
    fn discard(&mut self) {
        if self.dirty() {
            self.sync_text(self.observed.clone());
        }
        self.rejection = None;
    }
    /// Applies a new description of the same editor (`Component::reconcile`):
    /// its binding, typography, permissions, normalizer and callbacks.
    ///
    /// A different binding ends the edit in progress the way an application
    /// write does: an IME composition, a draft and a refused draft are dropped
    /// unsaved, local undo is cleared, and the next update shows the new
    /// binding's text. The same binding keeps the draft, selection and undo.
    /// Leaving draft mode (removing the normalizer) drops an unsaved draft;
    /// entering it starts from the bound text. Becoming read-only or disabled
    /// cancels a composition and a drag, as those states already refuse edits.
    pub(super) fn adopt(&mut self, next: Self) -> Option<Invalidation> {
        let rebound = match (&self.value, &next.value) {
            (Some(old), Some(new)) => !old.read_only().same_source(&new.read_only()),
            (None, None) => false,
            _ => true,
        };
        let next_drafting = next.normalizer.is_some() || next.limited() || next.session.is_some();
        let mut invalidation: Option<Invalidation> = None;
        let mut widen = |level: Invalidation| {
            invalidation = Some(invalidation.map_or(level, |current| current.max(level)));
        };
        if rebound {
            self.value = next.value;
            self.initialized = false;
            self.observed.clear();
            self.rejected = None;
            self.rejection = None;
            self.cancel_composition();
            self.dragging = false;
            self.undo.clear();
            self.redo.clear();
            widen(Invalidation::Paint);
        }
        if self.drafting() && !next_drafting {
            // Leaving draft mode: the draft was never saved.
            self.discard();
            widen(Invalidation::Paint);
        }
        self.normalizer = next.normalizer;
        if next
            .session
            .as_ref()
            .is_some_and(EditorAttachment::has_start)
        {
            // Update schedules the new edit's start.
            widen(Invalidation::Paint);
        }
        self.session = next.session;
        if (self.min_length, self.max_length, self.required)
            != (next.min_length, next.max_length, next.required)
        {
            // A refusal under the old limits ends with them.
            self.rejection = None;
            self.min_length = next.min_length;
            self.max_length = next.max_length;
            self.required = next.required;
            widen(Invalidation::Paint);
        }
        if self.placeholder.reconcile(next.placeholder) {
            widen(Invalidation::Paint);
        }
        if (self.clearable, &self.leading, &self.suffix)
            != (next.clearable, &next.leading, &next.suffix)
        {
            self.clearable = next.clearable;
            self.leading = next.leading;
            self.suffix = next.suffix;
            self.clear_hover = false;
            widen(Invalidation::Layout);
        }
        if self.modified_indicator != next.modified_indicator {
            self.modified_indicator = next.modified_indicator;
            widen(Invalidation::Paint);
        }
        self.modified = next.modified;
        if self.refusal.is_none() && next.refusal.is_some() {
            // A newly installed callback learns of a refusal already in place,
            // which takes an update to notice.
            self.refusal_reported = None;
            widen(Invalidation::Paint);
        }
        self.refusal = next.refusal;
        if self.counter != next.counter || self.revealable != next.revealable {
            self.counter = next.counter;
            self.revealable = next.revealable;
            if !self.revealable {
                self.conceal_now();
            }
            widen(Invalidation::Layout);
        }
        if self.concealed != next.concealed {
            // The selection lives in the shown text, which changes form; no
            // layout is available here, so the caret returns to the start.
            self.cancel_composition();
            self.concealed = next.concealed;
            self.revealed = false;
            self.selection.set(TextSelection::default());
            self.reveal.set(true);
            widen(Invalidation::Layout);
        }
        if self.style != next.style {
            self.style = next.style;
            widen(Invalidation::Layout);
        }
        if self
            .adopt_permissions(next.enablement, next.editability)
            .changed
        {
            widen(Invalidation::Paint);
        }
        self.submit = next.submit;
        self.report.adopt(next.report);
        self.saved.adopt(next.saved);
        invalidation
    }
    fn line_breaks(&self, text: &str) -> String {
        let text = text
            .replace("\r\n", "\n")
            .replace(['\r', '\u{2028}', '\u{2029}'], "\n");
        if self.multi {
            text
        } else {
            text.replace(['\n', '\t'], " ")
        }
    }
    /// Clipboard text as the editor inserts it: every line break, including
    /// NEL, vertical tab and form feed, becomes LF (a space in a single-line
    /// field), a tab stays a tab in a multiline field and becomes a space
    /// otherwise, and every other control character is dropped.
    fn paste_text(&self, text: &str) -> String {
        let text: String = text
            .replace("\r\n", "\n")
            .chars()
            .filter_map(|c| match c {
                '\r' | '\u{0B}' | '\u{0C}' | '\u{85}' | '\u{2028}' | '\u{2029}' => Some('\n'),
                '\n' | '\t' => Some(c),
                c if c.is_control() => None,
                c => Some(c),
            })
            .collect();
        self.line_breaks(&text)
    }
    /// Combines a description's availability restriction into its permission.
    pub(super) fn restrict_enablement(&mut self, permission: Permission) {
        self.enablement = std::mem::take(&mut self.enablement).and(permission);
        if let Some(enabled) = self.enablement.fixed() {
            self.enabled = enabled;
        }
    }
    /// Combines a description's editing restriction into its permission.
    pub(super) fn restrict_editing(&mut self, permission: Permission) {
        self.editability = std::mem::take(&mut self.editability).and(permission);
        if let Some(editable) = self.editability.fixed() {
            self.editable = editable;
            self.read_only = !editable || self.owner_lock;
        }
    }
    /// Sets the owner's own lock and the effective read-only state.
    pub(super) fn set_owner_lock(&mut self, locked: bool) {
        self.owner_lock = locked;
        let read_only = !self.editable || locked;
        if read_only && !self.read_only {
            self.lock_edits();
        }
        self.read_only = read_only;
    }
    /// Ends what a newly refused edit or input leaves behind: a composition
    /// and the drags in progress.
    fn lock_edits(&mut self) {
        self.cancel_composition();
        self.dragging = false;
        self.bar_drag = None;
    }
    /// Applies the effective availability and editability, cancelling what a
    /// revocation ends. Drafts are never touched here.
    fn set_availability(&mut self, enabled: bool, editable: bool) -> Availability {
        let read_only = !editable || self.owner_lock;
        let disabled = self.enabled && !enabled;
        let locked = !self.read_only && read_only;
        if disabled || locked {
            self.lock_edits();
        }
        let changed = self.enabled != enabled || self.read_only != read_only;
        self.enabled = enabled;
        self.editable = editable;
        self.read_only = read_only;
        Availability {
            changed,
            disabled,
            locked,
        }
    }
    /// Adopts a description's permissions. A fixed permission applies at
    /// once; a bound one keeps the current state until the next update reads
    /// it, so redescription never briefly grants what a binding denies.
    pub(super) fn adopt_permissions(
        &mut self,
        enablement: Permission,
        editability: Permission,
    ) -> Availability {
        let replaced =
            !self.enablement.same_as(&enablement) || !self.editability.same_as(&editability);
        let enabled = enablement.fixed().unwrap_or(self.enabled);
        let editable = editability.fixed().unwrap_or(self.editable);
        self.enablement = enablement;
        self.editability = editability;
        let mut availability = self.set_availability(enabled, editable);
        availability.changed |= replaced;
        availability
    }
    /// Reads both permissions in an update, subscribing to their bindings,
    /// and applies them. A revoked enablement is also recorded until the
    /// owner takes it with [`Self::take_revoked`].
    ///
    /// # Errors
    /// The runtime error of reading a bound permission, such as
    /// [`RuntimeError::ForeignRuntime`].
    pub(super) fn read_permissions(
        &mut self,
        cx: &mut UpdateContext,
    ) -> Result<Availability, RuntimeError> {
        let enabled = self.enablement.read_paint(cx)?;
        let editable = self.editability.read_paint(cx)?;
        let availability = self.set_availability(enabled, editable);
        if availability.disabled || availability.locked {
            // A held gesture, such as a selection drag, ends with the
            // permission that allowed it.
            cx.cancel_input();
        }
        self.revoked |= availability.disabled;
        Ok(availability)
    }
    /// The permissions' current values, read without subscribing, for work
    /// delivered before the update that would observe a change, such as a
    /// debounce deadline.
    ///
    /// # Errors
    /// The runtime error of reading a bound permission.
    pub(super) fn permits_now(&self) -> Result<(bool, bool), RuntimeError> {
        Ok((
            self.enabled && self.enablement.get()?,
            !self.owner_lock && self.editability.get()?,
        ))
    }
    /// Takes the record of a revoked enablement since it was last taken.
    pub(super) fn take_revoked(&mut self) -> bool {
        std::mem::take(&mut self.revoked)
    }
    fn cancel_composition(&mut self) {
        self.preedit = None;
        self.ime_session = self.ime_session.and_then(|n| n.checked_add(1));
    }
    fn reset(&mut self) {
        self.cancel_composition();
        self.dragging = false;
        self.bar_drag = None;
        self.bar_hover = false;
        self.hovered = false;
        self.reveal_hover = false;
        self.clear_hover = false;
        self.click_series.reset();
        self.clicks = 0;
        self.focused = false;
        self.blink_at = None;
        self.restart_blink = true;
    }
    fn changed(&mut self, cx: &mut EventContext) {
        self.reveal.set(true);
        self.restart_blink = true;
        cx.invalidate(Invalidation::Paint);
    }
    fn width(&self) -> Option<f32> {
        self.multi.then(|| {
            self.prepared
                .borrow()
                .as_ref()
                .map_or(300.0, |p| p.inner.width().max(1.0))
        })
    }
    fn layout(&self, cx: &EventContext) -> Result<TextLayout, RuntimeError> {
        self.layout_of(cx, &self.text)
    }
    /// Whether `command` would do something in the editor's current state.
    ///
    /// `clipboard` is whether the clipboard currently holds text; the caller reads
    /// it once when it needs the answer, so this never performs clipboard I/O.
    fn command_available(&self, command: crate::TextCommand, clipboard: bool) -> bool {
        use crate::TextCommand as C;
        if !self.enabled {
            return false;
        }
        match command {
            // Concealed text never reaches the clipboard, and a spaced paste
            // would place text by word boundaries the dots do not show.
            C::Copy | C::Cut | C::PasteBefore | C::PasteAfter if self.concealed => false,
            C::Copy => !self.selection.get().byte_range().is_empty(),
            C::Cut => !self.locked() && !self.selection.get().byte_range().is_empty(),
            C::Paste | C::PasteBefore | C::PasteAfter => !self.locked() && clipboard,
            C::SelectAll => !self.text.is_empty(),
        }
    }
    /// Opens the clipboard menu, at a pointer position or anchored below the field
    /// for a keyboard request.
    ///
    /// Availability is decided once here: the selection is known, and the clipboard
    /// is read exactly once, so opening the menu performs no repeated I/O and the
    /// retained list needs no polling.
    fn open_menu(
        &mut self,
        cx: &mut EventContext,
        position: Option<Point>,
    ) -> Result<InputOutcome, RuntimeError> {
        // An editor without a menu, such as an embedded one, never reads the
        // clipboard for it, so a failing clipboard cannot swallow the event.
        if !self.enabled || self.menu.is_none() {
            return Ok(InputOutcome::Ignored);
        }
        let clipboard = match cx.clipboard_text() {
            Ok(text) => !text.trim().is_empty(),
            // An absent or empty clipboard disables pasting rather than failing;
            // a real read failure still propagates.
            Err(RuntimeError::Clipboard(ClipboardError::NoText | ClipboardError::Unavailable)) => {
                false
            }
            Err(error) => return Err(error),
        };
        let decisions: Vec<_> = self
            .menu
            .as_ref()
            .map(|menu| {
                menu.available
                    .iter()
                    .map(|(command, signal)| {
                        (signal.clone(), self.command_available(*command, clipboard))
                    })
                    .collect()
            })
            .unwrap_or_default();
        let Some(menu) = &self.menu else {
            return Ok(InputOutcome::Ignored);
        };
        for (signal, available) in decisions {
            signal.set(available)?;
        }
        menu.position.set(position);
        menu.open.set(true)?;
        cx.invalidate(Invalidation::Layout);
        Ok(InputOutcome::Handled)
    }
    fn menu_children(&mut self) -> Vec<Element> {
        self.menu
            .as_mut()
            .and_then(|menu| menu.items.take())
            .into_iter()
            .collect()
    }
    fn menu_shown(&self) -> bool {
        self.menu.as_ref().is_some_and(|menu| menu.shown)
    }
    fn close_menu(&mut self) -> Result<(), RuntimeError> {
        if let Some(menu) = &self.menu {
            menu.open.set(false)?;
        }
        Ok(())
    }
    fn arrange_menu(&self, cx: &mut ArrangeContext<'_>, size: Size) -> Result<(), RuntimeError> {
        place_menu(cx, size, self.menu.as_ref())
    }
    fn command(
        &mut self,
        cx: &mut EventContext,
        command: crate::TextCommand,
    ) -> Result<crate::TextCommandOutcome, RuntimeError> {
        use crate::{TextCommand as C, TextCommandOutcome as R};
        if !self.enabled {
            return Ok(R::Unavailable);
        }
        // A clipboard read failure other than an empty clipboard must propagate
        // rather than silently becoming an unavailable command.
        let read = |cx: &mut EventContext| match cx.clipboard_text() {
            Ok(text) => Ok(Some(text)),
            Err(RuntimeError::Clipboard(ClipboardError::NoText)) => Ok(None),
            Err(error) => Err(error),
        };
        match command {
            C::Copy | C::Cut | C::PasteBefore | C::PasteAfter if self.concealed => {
                Ok(R::Unavailable)
            }
            C::Copy | C::Cut => {
                let cut = command == C::Cut;
                let range = self.selection.get().byte_range();
                if range.is_empty() || (cut && self.locked()) {
                    return Ok(R::Unavailable);
                }
                // Cut deletes only after the write succeeds, so a clipboard
                // failure cannot lose the text.
                cx.set_clipboard_text(&self.text[range.clone()])?;
                if cut {
                    // An edit during composition starts a new IME session, so
                    // the host resets native composition before its next commit.
                    self.cancel_composition();
                    self.replace(cx, range, "", true)?;
                }
                Ok(R::Handled)
            }
            C::Paste => {
                if self.locked() {
                    return Ok(R::Unavailable);
                }
                let text = read(cx)?.map(|text| self.paste_text(&text));
                match text {
                    Some(text) if !text.is_empty() => {
                        self.cancel_composition();
                        Ok(if self.insert(cx, &text)? {
                            R::Handled
                        } else {
                            R::Unavailable
                        })
                    }
                    _ => Ok(R::Unavailable),
                }
            }
            C::PasteBefore | C::PasteAfter => {
                if self.locked() {
                    return Ok(R::Unavailable);
                }
                let Some(text) = read(cx)? else {
                    return Ok(R::Unavailable);
                };
                let normalized = self.paste_text(&text);
                let Some((at, insert, keep)) = spaced_paste::resolve(
                    &self.text,
                    self.selection.get().byte_range(),
                    &normalized,
                    command == C::PasteAfter,
                ) else {
                    return Ok(R::Unavailable);
                };
                self.cancel_composition();
                if !self.replace(cx, at..at, &insert, true)? {
                    return Ok(R::Unavailable);
                }
                // `replace` leaves a collapsed caret after the insertion; a spaced
                // paste places text beside its target and keeps that target selected.
                let layout = self.layout(cx)?;
                self.selection
                    .set(TextSelection::new(&layout, keep.start, keep.end));
                self.changed(cx);
                Ok(R::Handled)
            }
            C::SelectAll => {
                if self.text.is_empty() {
                    return Ok(R::Unavailable);
                }
                let layout = self.layout(cx)?;
                self.selection
                    .set(TextSelection::new(&layout, 0, usize::MAX));
                self.changed(cx);
                Ok(R::Handled)
            }
        }
    }
    fn sync_text(&mut self, text: String) {
        if !self.initialized || text != self.text {
            self.text = text;
            self.payload = M::default();
            self.initialized = true;
            self.selection.set(TextSelection::default());
            self.cancel_composition();
            self.undo.clear();
            self.redo.clear();
            self.reveal.set(true);
            self.restart_blink = true;
        }
    }
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        self.update_inner(cx)?;
        if (self.modified.is_some() || self.refusal.is_some() || self.session.is_some())
            && self.notifier.is_none()
        {
            self.notifier = Some(cx.notifier());
        }
        if !self.start_pending.get()
            && self
                .session
                .as_ref()
                .is_some_and(EditorAttachment::has_start)
            && let Some(notifier) = &self.notifier
        {
            self.start_pending.set(true);
            self.reports.borrow_mut().push_back(Report::Start);
            notifier.request();
        }
        self.note_refusal();
        if self.drafting() {
            // Read through the table so another installed table relabels it.
            self.modified_text = cx
                .strings()?
                .text(crate::StringRole::TextModified)
                .to_owned();
        }
        self.note_modified();
        Ok(())
    }
    fn update_inner(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        self.read_permissions(cx)?;
        // The hint is painted, never measured.
        self.placeholder.update_paint(cx)?;
        if let Some(menu) = &mut self.menu {
            // The menu's actions name this editor explicitly rather than acting on
            // whatever happens to be focused when one is chosen.
            let mut target = menu.target.borrow_mut();
            if target.is_none() {
                *target = Some(Rc::downgrade(&cx.id().0));
            }
            drop(target);
            // Subscribing here is what makes the surface disappear again once an
            // action, Escape or an outside press closes it.
            menu.shown = cx.read(&menu.open)?;
        }
        if let Some(value) = &self.value {
            let text = cx.read_paint(value)?;
            // Refused text is kept out of the editor rather than failing every
            // flush: the field keeps its previous text, marks the refusal and
            // accepts no edits, so it never overwrites the application's value.
            match check_text(&text, self.multi) {
                Ok(()) => {
                    self.rejected = None;
                    // Only a changed binding is an external write; in draft
                    // mode it wins over the draft.
                    if !self.initialized || text != self.observed {
                        self.observed.clone_from(&text);
                        self.rejection = None;
                        self.sync_text(text);
                    }
                }
                Err(error) => {
                    if self.rejected.is_none() {
                        self.cancel_composition();
                        self.dragging = false;
                        // A refused value ends a draft unsaved.
                        self.discard();
                    }
                    self.rejected = Some(error);
                }
            }
        }
        let focused = cx.focused() && self.enabled && self.active;
        if focused != self.focused {
            self.restart_blink = true;
        }
        self.focused = focused;
        if focused && !self.locked() {
            let now = cx.now();
            if self.restart_blink {
                self.blink = true;
                self.blink_at = now.checked_add(BLINK);
                self.restart_blink = false;
            } else if self.blink_at.is_some_and(|t| now >= t) {
                self.blink = !self.blink;
                self.blink_at = now.checked_add(BLINK);
            }
            let next = self.blink_at.ok_or(RuntimeError::Exhausted)?;
            cx.wake_after(next.saturating_sub(now).max(Duration::from_nanos(1)))?;
        } else {
            self.blink_at = None;
            // The caret starts blinking afresh once it shows again, such as
            // when a focused read-only field becomes editable.
            self.restart_blink = true;
        }
        Ok(())
    }
    fn measure(
        &self,
        cx: &MeasureContext<'_>,
        available: Constraints,
    ) -> Result<Size, RuntimeError> {
        let line = cx.text("Mg", None, self.style)?.size().height();
        let height = (line * if self.multi { 6.0 } else { 1.0 }
            + 2.0 * cx.metric(MetricRole::ControlPaddingY))
        .max(cx.metric(MetricRole::ControlHeight));
        let width = if self.multi { 320.0_f32 } else { 240.0_f32 };
        Ok(Size::new(
            available.width().map_or(width, |w| w.min(width)),
            height,
        )?)
    }
    fn display(&self) -> (String, Option<Range<usize>>) {
        if self.masked() {
            // A concealed editor never composes, so only its text is shown.
            return (concealed_text(&self.text), None);
        }
        if let Some((preedit, range)) = &self.preedit {
            let mut text = self.text.clone();
            text.replace_range(range.clone(), preedit.text());
            (text, Some(range.start..range.start + preedit.text().len()))
        } else {
            (self.text.clone(), None)
        }
    }
    fn display_selection(&self, layout: &TextLayout) -> TextSelection {
        if let Some((preedit, range)) = &self.preedit {
            let cursor = preedit.cursor_byte_range().unwrap_or(0..0);
            TextSelection::new(layout, range.start + cursor.start, range.start + cursor.end)
        } else {
            self.selection.get()
        }
    }
    fn arrange(&self, cx: &MeasureContext<'_>, size: Size) -> Result<(), RuntimeError> {
        self.bar_extents.set(scrollbar::Extents::read(cx));
        let px = cx
            .metric(MetricRole::ControlPaddingX)
            .min(size.width() / 2.0);
        let py = cx
            .metric(MetricRole::ControlPaddingY)
            .min(size.height() / 2.0);
        let inner = Rect::new(
            px,
            py,
            (size.width() - 2.0 * px).max(0.0),
            (size.height() - 2.0 * py).max(0.0),
        )?;
        let inner = self.area.get().unwrap_or(inner);
        let layout = cx.text(
            &self.display().0,
            self.multi.then_some(inner.width().max(1.0)),
            self.style,
        )?;
        let caret = self.display_selection(&layout).caret(&layout)?;
        self.reveal.set(true);
        let origin = self.position(&layout, inner, caret);
        *self.prepared.borrow_mut() = Some(Prepared {
            layout,
            inner,
            origin,
        });
        Ok(())
    }
    /// The counter's text for the current draft.
    fn counter_text(&self) -> String {
        let count = self.text.graphemes(true).count();
        match self.max_length.or(self.min_length) {
            Some(limit) => format!("{count}/{limit}"),
            None => count.to_string(),
        }
    }
    /// Whether the counter shows a count the length limits would refuse.
    fn counter_over(&self) -> bool {
        let count = self.text.graphemes(true).count();
        self.max_length.is_some_and(|max| count > max)
            || self
                .min_length
                .is_some_and(|min| count < min && (count > 0 || self.required))
    }
    /// The widest counter text the editor can show: the count's digits for
    /// the most graphemes a draft can hold (at most one per byte of
    /// [`TextEditError::MAX_TEXT_BYTES`](crate::TextEditError::MAX_TEXT_BYTES)),
    /// because typing past the maximum is allowed until a save refuses it. The
    /// reserved space therefore never changes and never clips a count.
    fn counter_template(&self) -> String {
        let count = "8".repeat(crate::TextEditError::MAX_TEXT_BYTES.to_string().len());
        match self.max_length.or(self.min_length) {
            Some(limit) => format!("{count}/{limit}"),
            None => count,
        }
    }
    /// Reserves the leading icon, the suffix, the counter, the clear button
    /// and the reveal button inside the frame and sets the editing area to
    /// what is left; with none of them, the editing area is the padded frame
    /// as before. From the end inwards: reveal button, clear button, counter,
    /// suffix; the leading icon sits at the start.
    fn place_adornments(&self, cx: &MeasureContext<'_>, size: Size) -> Result<(), RuntimeError> {
        let single = !self.multi;
        let reveal = self.concealed && self.revealable && single;
        let clear = self.clearable && single;
        let leading = self.leading.is_some() && single;
        let suffix = !self.suffix.is_empty() && single;
        self.reveal_rect.set(None);
        self.clear_rect.set(None);
        self.counter_rect.set(None);
        self.leading_rect.set(None);
        self.suffix_rect.set(None);
        if !reveal && !self.counter && !clear && !leading && !suffix {
            self.area.set(None);
            return Ok(());
        }
        let px = cx
            .metric(MetricRole::ControlPaddingX)
            .min(size.width() / 2.0);
        let py = cx
            .metric(MetricRole::ControlPaddingY)
            .min(size.height() / 2.0);
        let gap = cx.metric(MetricRole::IconGap);
        let (mut x, y) = (px, py);
        let mut w = (size.width() - 2.0 * px).max(0.0);
        let mut h = (size.height() - 2.0 * py).max(0.0);
        // A square icon button at the end, its slot reserved even while it is
        // not shown so the text never reflows.
        let end_button = |w: &mut f32| -> Result<Rect, RuntimeError> {
            let side = cx.metric(MetricRole::IconSize).min(h).min(*w / 2.0);
            let button = rect(x + *w - side, (size.height() - side) / 2.0, side, side)?;
            *w = (*w - side - gap).max(0.0);
            Ok(button)
        };
        if reveal {
            self.reveal_rect.set(Some(end_button(&mut w)?));
        }
        if clear {
            self.clear_rect.set(Some(end_button(&mut w)?));
        }
        if self.counter {
            let template = cx.text(&self.counter_template(), None, TextStyle::Caption)?;
            let width = template.size().width().min(w);
            let line = template.size().height().min(h);
            if self.multi {
                // A multiline editor keeps a caption line at its bottom.
                self.counter_rect
                    .set(Some(rect(x + w - width, y + h - line, width, line)?));
                h = (h - line - gap / 2.0).max(0.0);
            } else {
                self.counter_rect
                    .set(Some(rect(x + w - width, y, width, h)?));
                w = (w - width - gap).max(0.0);
            }
        }
        if suffix {
            let text = cx.text(&self.suffix, None, self.style)?;
            let width = text.size().width().min(w / 2.0);
            self.suffix_rect
                .set(Some(rect(x + w - width, y, width, h)?));
            w = (w - width - gap).max(0.0);
        }
        if leading {
            let side = cx.metric(MetricRole::IconSize).min(h).min(w / 2.0);
            self.leading_rect
                .set(Some(rect(x, (size.height() - side) / 2.0, side, side)?));
            x += side + gap;
            w = (w - side - gap).max(0.0);
        }
        self.area.set(Some(rect(x, y, w, h)?));
        Ok(())
    }
    /// Paints the leading icon, the suffix, the clear button and the
    /// modified mark.
    fn paint_adornments(&self, p: &mut Painter, size: Size) -> Result<(), DrawError> {
        let muted = p.color(ColorRole::Muted);
        if let (Some(icon), Some(area)) = (&self.leading, self.leading_rect.get()) {
            p.icon(icon.clone(), area, muted)?;
        }
        if let Some(area) = self.suffix_rect.get() {
            let text = p.text(&self.suffix, None, self.style)?;
            let [x, y, _, h] = area.values();
            p.with_clip(area, |p| {
                text.paint(
                    p,
                    crate::Point::new(x, y + (h - text.size().height()).max(0.0) / 2.0)?,
                    muted,
                )
            })?;
        }
        if self.clear_shown()
            && let Some(button) = self.clear_rect.get()
        {
            if self.clear_hover {
                p.fill_shape(
                    button,
                    crate::choice_controls::rounded(p.metric(MetricRole::CornerRadius))?,
                    p.color(ColorRole::ButtonHover),
                )?;
            }
            p.icon(Icon::Close, button, p.color(ColorRole::OnSurface))?;
        }
        if self.modified_indicator && self.dirty() {
            // A dot in the top end corner, inside the frame's padding, sized
            // from the focus ring so it scales with the theme's density.
            let side = (2.0 * p.metric(MetricRole::FocusWidth))
                .min(p.metric(MetricRole::ControlPaddingY))
                .max(1.0);
            let inset = p.metric(MetricRole::FocusWidth);
            let dot = Rect::new(size.width() - inset - side, inset, side, side)?;
            p.fill_shape(
                dot,
                crate::choice_controls::rounded(side / 2.0)?,
                p.color(ColorRole::Primary),
            )?;
        }
        Ok(())
    }
    /// Whether the clear button is offered now: a clearable, editable field
    /// with text and no composition in progress.
    fn clear_shown(&self) -> bool {
        self.clearable
            && !self.multi
            && self.enabled
            && !self.locked()
            && !self.text.is_empty()
            && self.preedit.is_none()
    }
    /// Whether a point in local coordinates is over the shown clear button.
    fn over_clear(&self, point: Point) -> bool {
        self.clear_shown() && self.clear_rect.get().is_some_and(|r| r.contains(point))
    }
    /// Empties the text as one undoable edit, through the same path as typing:
    /// written at once without draft mode, kept as a draft until saved in it.
    fn clear_text(&mut self, cx: &mut EventContext) -> Result<(), RuntimeError> {
        if !self.clear_shown() {
            return Ok(());
        }
        self.dragging = false;
        self.click_series.reset();
        self.clicks = 0;
        let len = self.text.len();
        self.replace(cx, 0..len, "", true)?;
        Ok(())
    }
    /// Whether a point in local coordinates is over the reveal button.
    fn over_reveal(&self, point: Point) -> bool {
        self.concealed
            && self.revealable
            && self.reveal_rect.get().is_some_and(|r| r.contains(point))
    }
    fn position(&self, layout: &TextLayout, inner: Rect, caret: Rect) -> [f32; 2] {
        let [px, py, w, h] = inner.values();
        let [x, y, cw, ch] = caret.values();
        let mut scroll = self.scroll.get();
        if self.reveal.replace(false) {
            for (axis, start, end, space) in [(0, x, x + cw, w), (1, y, y + ch, h)] {
                if start < scroll[axis] {
                    scroll[axis] = start;
                }
                if end > scroll[axis] + space {
                    scroll[axis] = (end - space).max(0.0);
                }
            }
        }
        scroll[0] = scroll[0].clamp(0.0, (layout.size().width() + cw - w).max(0.0));
        scroll[1] = scroll[1].clamp(0.0, (layout.size().height() - h).max(0.0));
        self.scroll.set(scroll);
        let spare = w - layout.size().width() - cw;
        let x = if self.align_end && !self.multi && spare > 0.0 {
            px + spare
        } else {
            px - scroll[0]
        };
        let y = if self.multi {
            py - scroll[1]
        } else if self.embedded {
            let (above, below) = layout.first_line_box().unwrap_or((0.0, 0.0));
            py + ((h - above - below).max(0.0) / 2.0) + above
                - layout.first_baseline().unwrap_or(0.0)
                - scroll[1]
        } else {
            py + ((h - layout.size().height()).max(0.0) / 2.0) - scroll[1]
        };
        [x, y]
    }
    // Vertical scrollbar geometry inside `inner` for a wrapped text extent:
    // the track hit strip, the visible thumb and the maximum scroll offset. This is
    // the scroll areas' own bar, so it comes from the shared geometry. Absent unless
    // the text overflows an editor that scrolls vertically.
    fn bar_geometry(
        &self,
        inner: Rect,
        extent: f32,
    ) -> Result<Option<scrollbar::ScrollbarGeometry>, crate::GeometryError> {
        let [_, _, iw, ih] = inner.values();
        if !self.multi || !self.enabled || extent - ih <= 0.0 || ih <= 0.0 || iw <= 0.0 {
            return Ok(None);
        }
        scrollbar::geometry(
            self.bar_extents.get(),
            inner,
            true,
            extent,
            self.scroll.get()[1],
            0.0,
        )
        .map(Some)
    }
    // The same geometry from the retained arrangement, for input and hit testing.
    fn bar(&self) -> Result<Option<scrollbar::ScrollbarGeometry>, crate::GeometryError> {
        let prepared = self.prepared.borrow();
        let Some(p) = prepared.as_ref() else {
            return Ok(None);
        };
        self.bar_geometry(p.inner, p.layout.size().height())
    }
    // Moves the scroll offset so the grabbed thumb position follows the pointer.
    fn bar_drag_to(&self, cx: &mut EventContext, point: Point) -> Result<(), RuntimeError> {
        let Some(grab) = self.bar_drag else {
            return Ok(());
        };
        let Some(next) = self
            .bar()?
            .and_then(|bar| bar.offset_for_pointer(point, grab))
        else {
            return Ok(());
        };
        let old = self.scroll.get();
        if next != old[1] {
            self.scroll.set([old[0], next]);
            self.reveal.set(false);
            cx.invalidate(Invalidation::Paint);
        }
        Ok(())
    }
    /// Whether the pointer of `e` rests over this editor's scrollbar track.
    fn over_bar(&self, cx: &EventContext, e: PointerEvent) -> Result<bool, RuntimeError> {
        if e.is_outside() || !cx.pointer_inside() {
            return Ok(false);
        }
        let point = Self::local(cx, e)?;
        Ok(self.bar()?.is_some_and(|bar| bar.track.contains(point)))
    }
    fn set_bar_hover(&mut self, cx: &mut EventContext, hover: bool) {
        if self.bar_hover != hover {
            self.bar_hover = hover;
            cx.invalidate(Invalidation::Paint);
        }
    }
    fn set_hover(&mut self, cx: &mut EventContext, hovered: bool) {
        let hovered = hovered && self.enabled && !self.embedded;
        if self.hovered != hovered {
            self.hovered = hovered;
            cx.invalidate(Invalidation::Paint);
        }
    }
    // Local pointer position relative to this editor's own bounds. An unarranged
    // editor has no origin to subtract, so the window position is already local.
    fn local(cx: &EventContext, e: PointerEvent) -> Result<Point, RuntimeError> {
        Ok(cx.to_local(e.position()).unwrap_or(e.position()))
    }
    fn cursor(&self, point: Point) -> Option<CursorIcon> {
        if !self.enabled {
            return None;
        }
        if self.over_reveal(point) || self.over_clear(point) {
            return Some(CursorIcon::Default);
        }
        let over_bar = self
            .bar()
            .ok()
            .flatten()
            .is_some_and(|bar| self.bar_drag.is_some() || bar.track.contains(point));
        Some(if over_bar {
            CursorIcon::Default
        } else {
            CursorIcon::Text
        })
    }
    fn paint(&self, p: &mut Painter) -> Result<(), DrawError> {
        let Some(size) = p.size() else {
            return Ok(());
        };
        let px = p
            .metric(MetricRole::ControlPaddingX)
            .min(size.width() / 2.0);
        let py = p
            .metric(MetricRole::ControlPaddingY)
            .min(size.height() / 2.0);
        let inner = Rect::new(
            px,
            py,
            (size.width() - 2.0 * px).max(0.0),
            (size.height() - 2.0 * py).max(0.0),
        )?;
        let inner = self.area.get().unwrap_or(inner);
        let [_, _, w, _] = inner.values();
        let (text, composition) = self.display();
        let layout = p.text(&text, self.multi.then_some(w.max(1.0)), self.style)?;
        let selection = self.display_selection(&layout);
        let caret = selection.caret(&layout)?;
        let origin = self.position(&layout, inner, caret);
        if !self.embedded {
            field_frame(
                p,
                Rect::from_size(size),
                self.focused,
                self.rejected.is_some() || self.rejection.is_some(),
                self.hovered && self.enabled,
            )?;
        }
        p.with_clip(inner, |p| -> Result<(), DrawError> {
            if self.text.is_empty() && self.preedit.is_none() && !self.placeholder.text().is_empty()
            {
                let hint = p.rich_text(
                    &self.placeholder.display(),
                    self.multi.then_some(w.max(1.0)),
                    self.style,
                )?;
                hint.paint(
                    p,
                    crate::Point::new(origin[0], origin[1])?,
                    p.color(ColorRole::Muted),
                )?;
            }
            let selection_rects = selection.rectangles(&layout)?;
            for r in &selection_rects {
                p.fill_rect(offset(*r, origin)?, p.color(ColorRole::TextSelection));
            }
            layout.paint(
                p,
                crate::Point::new(origin[0], origin[1])?,
                p.color(if self.enabled {
                    ColorRole::OnSurface
                } else {
                    ColorRole::Muted
                }),
            )?;
            for r in selection_rects {
                p.with_clip(offset(r, origin)?, |p| {
                    layout.paint(
                        p,
                        crate::Point::new(origin[0], origin[1])?,
                        p.color(ColorRole::OnTextSelection),
                    )
                })?;
            }
            if let Some(range) = composition {
                for r in TextSelection::new(&layout, range.start, range.end).rectangles(&layout)? {
                    let [x, y, w, h] = r.values();
                    p.fill_rect(
                        Rect::new(origin[0] + x, origin[1] + y + h - 1.0, w, 1.0)?,
                        p.color(ColorRole::Primary),
                    );
                }
            }
            if self.focused
                && !self.locked()
                && self.blink
                && self
                    .preedit
                    .as_ref()
                    .is_none_or(|(t, _)| t.cursor_byte_range().is_some())
            {
                p.fill_rect(offset(caret, origin)?, p.color(ColorRole::OnSurface));
            }
            Ok(())
        })?;
        if let Some(bar) = self.bar_geometry(inner, layout.size().height())? {
            let active = self.bar_hover || self.bar_drag.is_some();
            p.fill_rect(bar.thumb, scrollbar::thumb_color(p, active));
        }
        if self.counter
            && let Some(area) = self.counter_rect.get()
        {
            let count = p.text(&self.counter_text(), None, TextStyle::Caption)?;
            let [x, y, cw, ch] = area.values();
            let role = if !self.enabled {
                ColorRole::Muted
            } else if self.counter_over() {
                ColorRole::Error
            } else {
                ColorRole::Muted
            };
            p.with_clip(area, |p| {
                count.paint(
                    p,
                    crate::Point::new(
                        x + cw - count.size().width(),
                        y + (ch - count.size().height()).max(0.0) / 2.0,
                    )?,
                    p.color(role),
                )
            })?;
        }
        if self.concealed
            && self.revealable
            && let Some(button) = self.reveal_rect.get()
        {
            if self.reveal_hover && self.enabled {
                p.fill_shape(
                    button,
                    crate::choice_controls::rounded(p.metric(MetricRole::CornerRadius))?,
                    p.color(ColorRole::ButtonHover),
                )?;
            }
            p.icon(
                if self.revealed {
                    Icon::VisibilityOff
                } else {
                    Icon::Visibility
                },
                button,
                p.color(if self.enabled {
                    ColorRole::OnSurface
                } else {
                    ColorRole::Muted
                }),
            )?;
        }
        self.paint_adornments(p, size)?;
        *self.prepared.borrow_mut() = Some(Prepared {
            layout,
            inner,
            origin,
        });
        Ok(())
    }
    fn input_state(&self) -> Option<TextInputState> {
        // A concealed editor asks for no IME, so no candidate window ever
        // shows its text; typed key text still arrives as committed text.
        if !self.enabled || self.locked() || !self.active || self.concealed {
            return None;
        }
        let prepared = self.prepared.borrow();
        let p = prepared.as_ref()?;
        let [x, y, w, h] = offset(
            self.display_selection(&p.layout).caret(&p.layout).ok()?,
            p.origin,
        )
        .ok()?
        .values();
        let [ix, iy, iw, ih] = p.inner.values();
        let w = w.min(iw);
        let h = h.min(ih);
        let caret = Rect::new(x.clamp(ix, ix + iw - w), y.clamp(iy, iy + ih - h), w, h).ok()?;
        Some(TextInputState {
            caret,
            session: self.ime_session?,
        })
    }
    fn replace(
        &mut self,
        cx: &mut EventContext,
        range: Range<usize>,
        insert: &str,
        history: bool,
    ) -> Result<bool, RuntimeError> {
        self.replace_with_payload(cx, range, insert, history, M::default())
    }
    fn replace_with_payload(
        &mut self,
        cx: &mut EventContext,
        range: Range<usize>,
        insert: &str,
        history: bool,
        payload: M,
    ) -> Result<bool, RuntimeError> {
        let mut candidate = self.text.clone();
        candidate.replace_range(range.clone(), insert);
        if candidate.len() > crate::TextEditError::MAX_TEXT_BYTES {
            // A full field refuses the edit like any editor at its limit;
            // valid input is not a component failure.
            return Ok(false);
        }
        self.validate(&candidate)?;
        let layout = self.layout_of(cx, &candidate)?;
        self.publish_text(&candidate)?;
        if candidate != self.text {
            self.rejection = None;
        }
        if history && (candidate != self.text || payload != self.payload) {
            let masked = self.masked();
            Self::push(
                &mut self.undo,
                Snapshot {
                    payload: self.payload,
                    text: self.text.clone(),
                    selection: self.selection.get(),
                    masked,
                },
            );
            self.redo.clear();
        }
        let caret = self.shown_offset(&candidate, range.start + insert.len());
        self.text = candidate;
        self.payload = payload;
        self.selection
            .set(TextSelection::new(&layout, caret, caret));
        self.preedit = None;
        self.changed(cx);
        Ok(true)
    }
    fn push(queue: &mut VecDeque<Snapshot<M>>, snapshot: Snapshot<M>) {
        if queue.len() == 32 {
            queue.pop_front();
        }
        queue.push_back(snapshot);
    }
    /// Inserts at the composition or selection; `false` when nothing changed,
    /// including an edit refused at the byte limit without an error.
    fn insert(&mut self, cx: &mut EventContext, text: &str) -> Result<bool, RuntimeError> {
        if !self.enabled || self.locked() || text.is_empty() {
            return Ok(false);
        }
        let normalized = self.line_breaks(text);
        let range = self
            .preedit
            .as_ref()
            .map_or_else(|| self.real_range(), |(_, r)| r.clone());
        self.replace(cx, range, &normalized, true)
    }
    fn compose(&mut self, cx: &mut EventContext, text: &TextPreedit) -> Result<(), RuntimeError> {
        if !self.enabled || self.locked() || self.concealed {
            return Ok(());
        }
        if text.text().is_empty() {
            self.preedit = None;
            self.changed(cx);
            return Ok(());
        }
        let range = self
            .preedit
            .as_ref()
            .map_or_else(|| self.selection.get().byte_range(), |(_, r)| r.clone());
        let mut candidate = self.text.clone();
        candidate.replace_range(range.clone(), text.text());
        if candidate.len() > crate::TextEditError::MAX_TEXT_BYTES {
            // A composition that would not fit is not shown, like a refused
            // edit, until one that fits or the commit arrives.
            if self.preedit.take().is_some() {
                self.changed(cx);
            }
            return Ok(());
        }
        self.validate(text.text())?;
        self.validate(&candidate)?;
        cx.text(&candidate, self.width(), self.style)?;
        self.preedit = Some((text.clone(), range));
        self.changed(cx);
        Ok(())
    }
    fn undo(&mut self, cx: &mut EventContext, redo: bool) -> Result<(), RuntimeError> {
        let snapshot = if redo {
            self.redo.back()
        } else {
            self.undo.back()
        }
        .cloned();
        if let Some(snapshot) = snapshot {
            let layout = self.layout_of(cx, &snapshot.text)?;
            self.publish_text(&snapshot.text)?;
            self.rejection = None;
            // A snapshot taken while the text was shown in the other form
            // (concealed or revealed) moves its selection into this form.
            let selection = if snapshot.masked == self.masked() {
                snapshot.selection
            } else {
                let convert = |offset| {
                    if snapshot.masked {
                        revealed_offset(&snapshot.text, offset)
                    } else {
                        concealed_offset(&snapshot.text, offset)
                    }
                };
                TextSelection::new(
                    &layout,
                    convert(snapshot.selection.anchor_byte()),
                    convert(snapshot.selection.focus_byte()),
                )
            };
            let old = Snapshot {
                payload: std::mem::replace(&mut self.payload, snapshot.payload),
                text: std::mem::replace(&mut self.text, snapshot.text),
                selection: self.selection.get(),
                masked: self.masked(),
            };
            self.selection.set(selection);
            if redo {
                self.redo.pop_back();
                Self::push(&mut self.undo, old);
            } else {
                self.undo.pop_back();
                Self::push(&mut self.redo, old);
            }
            self.changed(cx);
        }
        Ok(())
    }
    fn key(&mut self, cx: &mut EventContext, e: KeyEvent) -> Result<InputOutcome, RuntimeError> {
        if !self.enabled {
            return Ok(InputOutcome::Ignored);
        }
        if !matches!(
            e.transition,
            KeyTransition::Pressed | KeyTransition::Repeated
        ) {
            return Ok(InputOutcome::Ignored);
        }
        if let Some(outcome) = self.session_key(cx, e)? {
            return Ok(outcome);
        }
        let m = cx.modifiers();
        let command = if cfg!(target_os = "macos") {
            m.super_key
        } else {
            m.control && !m.alt
        };
        let word = if cfg!(target_os = "macos") {
            m.alt
        } else {
            m.control && !m.alt
        };
        if e.code == KeyCode::Escape {
            if self.bar_drag.take().is_some() {
                // The drag's emphasis ends; as on release, the bar stays
                // emphasized only if the stationary pointer is still over it.
                cx.release_pointer();
                let over = match cx.pointer_position() {
                    Some(point) => {
                        self.over_bar(cx, PointerEvent::new(point, PointerKind::Move))?
                    }
                    None => false,
                };
                self.set_bar_hover(cx, over);
                cx.invalidate(Invalidation::Paint);
                return Ok(InputOutcome::Handled);
            }
            if self.preedit.is_none()
                && !self.dragging
                && self.selection.get().byte_range().is_empty()
            {
                if !self.dirty() && self.rejection.is_none() {
                    // A refusal whose draft an application write discarded
                    // ends here; Escape still bubbles to the owner.
                    self.report.clear(cx)?;
                    return Ok(InputOutcome::Ignored);
                }
                self.discard();
                self.changed(cx);
                self.report.clear(cx)?;
                return Ok(InputOutcome::Handled);
            }
            if self.preedit.is_some() {
                self.cancel_composition();
            } else {
                let caret = self.selection.get().focus_byte();
                let layout = self.layout(cx)?;
                self.selection
                    .set(TextSelection::new(&layout, caret, caret));
            }
            self.dragging = false;
            cx.release_pointer();
            self.changed(cx);
            return Ok(InputOutcome::Handled);
        }
        if self.preedit.is_some() {
            return Ok(InputOutcome::Handled);
        }
        if e.requests_context_menu(m) {
            return self.open_menu(cx, None);
        }
        if self.concealed
            && self.revealable
            && e.code == KeyCode::F8
            && m.alt
            && !m.control
            && !m.super_key
        {
            if e.transition == KeyTransition::Pressed {
                self.set_revealed(cx, !self.revealed)?;
            }
            return Ok(InputOutcome::Handled);
        }
        let layout = self.layout(cx)?;
        if command && e.code == KeyCode::KeyA {
            self.selection
                .set(TextSelection::new(&layout, 0, usize::MAX));
            self.changed(cx);
            return Ok(InputOutcome::Handled);
        }
        // The chords and the context menu share one implementation, so a custom
        // editor reproducing the contract behaves identically for both.
        if command && matches!(e.code, KeyCode::KeyC | KeyCode::KeyX) {
            let which = if e.code == KeyCode::KeyX {
                crate::TextCommand::Cut
            } else {
                crate::TextCommand::Copy
            };
            self.command(cx, which)?;
            return Ok(InputOutcome::Handled);
        }
        if command && e.code == KeyCode::KeyV {
            self.command(cx, crate::TextCommand::Paste)?;
            return Ok(InputOutcome::Handled);
        }
        if command
            && (e.code == KeyCode::KeyZ || (!cfg!(target_os = "macos") && e.code == KeyCode::KeyY))
        {
            if !self.locked() {
                self.undo(cx, m.shift || e.code == KeyCode::KeyY)?;
            }
            return Ok(InputOutcome::Handled);
        }
        if let Some(movement) = key_movement(e.code, m) {
            self.selection.set(self.selection.get().moved(
                &layout,
                movement,
                crate::SelectionExtension::from_shift(m.shift),
            ));
            self.changed(cx);
            return Ok(InputOutcome::Handled);
        }
        if self.locked() {
            return Ok(InputOutcome::Ignored);
        }
        match e.code {
            KeyCode::Backspace | KeyCode::Delete => {
                let mut range = self.real_range();
                if range.is_empty() {
                    // Word boundaries of concealed text are secret, so a word
                    // deletion takes everything before or after the caret.
                    if self.concealed && word {
                        if e.code == KeyCode::Backspace {
                            range.start = 0;
                        } else {
                            range.end = self.text.len();
                        }
                    } else if e.code == KeyCode::Backspace {
                        range.start = if word {
                            self.text[..range.start]
                                .unicode_word_indices()
                                .next_back()
                                .map_or(0, |(i, _)| i)
                        } else {
                            self.text[..range.start]
                                .grapheme_indices(true)
                                .next_back()
                                .map_or(0, |(i, _)| i)
                        };
                    } else {
                        range.end = if word {
                            self.text[range.end..]
                                .unicode_word_indices()
                                .next()
                                .map_or(self.text.len(), |(i, w)| range.end + i + w.len())
                        } else {
                            self.text[range.end..]
                                .graphemes(true)
                                .next()
                                .map_or(range.end, |s| range.end + s.len())
                        };
                    }
                }
                self.replace(cx, range, "", true)?;
            }
            KeyCode::Enter => {
                if self.multi {
                    self.insert(cx, "\n")?;
                } else if e.transition == KeyTransition::Pressed
                    && self.submit_save(cx)?
                    && let Some(submit) = &mut self.submit
                {
                    (submit)(cx, &self.text)?;
                }
            }
            _ => return Ok(InputOutcome::Ignored),
        }
        Ok(InputOutcome::Handled)
    }
    fn pointer(
        &mut self,
        cx: &mut EventContext,
        e: PointerEvent,
    ) -> Result<InputOutcome, RuntimeError> {
        if !self.enabled {
            return Ok(InputOutcome::Ignored);
        }
        if self.concealed && self.revealable {
            if matches!(e.kind(), PointerKind::Move | PointerKind::Leave) {
                let over = e.kind() == PointerKind::Move
                    && !e.is_outside()
                    && self.over_reveal(Self::local(cx, e)?);
                if self.reveal_hover != over {
                    self.reveal_hover = over;
                    cx.invalidate(Invalidation::Paint);
                }
            }
            if e.kind() == PointerKind::Press(PointerButton::Primary)
                && !e.is_outside()
                && self.over_reveal(Self::local(cx, e)?)
            {
                // The button toggles without moving the caret or starting a
                // selection drag; it is part of the field's own Tab stop.
                self.click_series.reset();
                self.clicks = 0;
                self.set_revealed(cx, !self.revealed)?;
                return Ok(InputOutcome::Handled);
            }
        }
        if self.clearable && !self.multi {
            if matches!(e.kind(), PointerKind::Move | PointerKind::Leave) {
                let over = e.kind() == PointerKind::Move
                    && !e.is_outside()
                    && self.over_clear(Self::local(cx, e)?);
                if self.clear_hover != over {
                    self.clear_hover = over;
                    cx.invalidate(Invalidation::Paint);
                }
            }
            if e.kind() == PointerKind::Press(PointerButton::Primary)
                && !e.is_outside()
                && self.over_clear(Self::local(cx, e)?)
            {
                // Like the reveal button, the clear button acts on press,
                // starts no selection drag and is part of the field's Tab stop.
                self.clear_text(cx)?;
                return Ok(InputOutcome::Handled);
            }
        }
        // The bar under the pointer shows that it can be dragged; leaving the
        // field delivers `Leave`, which clears it.
        if self.bar_drag.is_none() && matches!(e.kind(), PointerKind::Move | PointerKind::Leave) {
            let over = e.kind() == PointerKind::Move && self.over_bar(cx, e)?;
            self.set_bar_hover(cx, over);
        }
        if e.kind() == PointerKind::Press(PointerButton::Secondary) {
            // Right-clicking keeps the caret and any selection exactly as they are,
            // so the menu acts on what the field already shows as selected.
            let local = cx.to_local(e.position());
            return self.open_menu(cx, local);
        }
        if let PointerKind::Scroll(delta) = e.kind() {
            let Some(p) = self
                .prepared
                .borrow()
                .as_ref()
                .map(|p| (p.layout.size(), p.inner.values()))
            else {
                return Ok(InputOutcome::Ignored);
            };
            let old = self.scroll.get();
            let next = [
                (old[0] + delta.x()).clamp(0.0, (p.0.width() - p.1[2]).max(0.0)),
                (old[1] + delta.y()).clamp(0.0, (p.0.height() - p.1[3]).max(0.0)),
            ];
            if next == old {
                return Ok(InputOutcome::Ignored);
            }
            self.scroll.set(next);
            self.reveal.set(false);
            cx.invalidate(Invalidation::Paint);
            return Ok(InputOutcome::Handled);
        }
        if e.kind() == PointerKind::Release(PointerButton::Primary) {
            if self.bar_drag.is_some() {
                self.bar_drag_to(cx, Self::local(cx, e)?)?;
                self.bar_drag = None;
                // The drag's emphasis ends; the bar stays emphasized only if
                // the pointer is still over it.
                let over = self.over_bar(cx, e)?;
                self.set_bar_hover(cx, over);
                cx.invalidate(Invalidation::Paint);
            }
            self.dragging = false;
            cx.release_pointer();
            return Ok(InputOutcome::Handled);
        }
        if e.kind() == PointerKind::Move && self.bar_drag.is_some() {
            self.bar_drag_to(cx, Self::local(cx, e)?)?;
            return Ok(InputOutcome::Handled);
        }
        if e.kind() == PointerKind::Press(PointerButton::Primary)
            && !e.is_outside()
            && let Some(bar) = self.bar()?
        {
            let point = Self::local(cx, e)?;
            if bar.track.contains(point) {
                let grab = bar.grab_offset(point);
                cx.capture_pointer()?;
                self.bar_drag = Some(grab);
                // A scroll gesture must not extend a word/select-all click run.
                self.click_series.reset();
                self.clicks = 0;
                self.bar_drag_to(cx, point)?;
                return Ok(InputOutcome::Handled);
            }
        }
        if !matches!(e.kind(), PointerKind::Press(PointerButton::Primary))
            && !(e.kind() == PointerKind::Move && self.dragging)
        {
            return Ok(InputOutcome::Ignored);
        }
        if e.kind() == PointerKind::Press(PointerButton::Primary) {
            self.cancel_composition();
        }
        let layout = self.layout(cx)?;
        let origin = self
            .prepared
            .borrow()
            .as_ref()
            .map_or([0.0; 2], |p| p.origin);
        // Local, then relative to the prepared text's own origin inside the field.
        let local = cx.to_local(e.position()).unwrap_or(e.position());
        let point = Point::new(local.x() - origin[0], local.y() - origin[1])?;
        if e.kind() == PointerKind::Press(PointerButton::Primary) {
            let consecutive =
                self.click_series
                    .press(cx.now(), e.position(), PointerButton::Primary)
                    > 1;
            self.clicks = if consecutive { self.clicks % 3 + 1 } else { 1 };
            let selected = match self.clicks {
                // Word boundaries of concealed text are secret, so a double
                // click selects everything.
                2 if self.concealed => TextSelection::new(&layout, 0, usize::MAX),
                2 => TextSelection::word_at_point(&layout, point),
                3 => TextSelection::new(&layout, 0, usize::MAX),
                _ => self.selection.get().at_point(
                    &layout,
                    point,
                    crate::SelectionExtension::from_shift(cx.modifiers().shift),
                ),
            };
            self.selection.set(selected);
            self.dragging = true;
            cx.capture_pointer()?;
        } else if self.clicks != 3 {
            self.selection.set(self.selection.get().at_point(
                &layout,
                point,
                crate::SelectionExtension::Extend,
            ));
        }
        self.changed(cx);
        Ok(InputOutcome::Handled)
    }
    fn inspect(&self) -> ComponentDetails {
        let mut d = ComponentDetails::default();
        let count = self.text.graphemes(true).count();
        if self.concealed {
            // Concealed content never leaves the field: only its length.
            d.properties
                .insert("concealed".into(), InspectValue::Bool(true));
            d.properties
                .insert("length".into(), InspectValue::Number(count as f64));
            if self.revealable {
                d.properties
                    .insert("revealed".into(), InspectValue::Bool(self.revealed));
            }
        } else {
            d.properties
                .insert("text".into(), InspectValue::Text(self.text.clone()));
        }
        if !self.placeholder.text().is_empty() {
            d.properties.insert(
                "placeholder".into(),
                InspectValue::Text(self.placeholder.text().to_owned()),
            );
        }
        if self.required {
            d.properties
                .insert("required".into(), InspectValue::Bool(true));
        }
        if let Some(min) = self.min_length {
            d.properties
                .insert("min_length".into(), InspectValue::Number(min as f64));
        }
        if let Some(max) = self.max_length {
            d.properties
                .insert("max_length".into(), InspectValue::Number(max as f64));
        }
        if self.counter {
            d.properties
                .insert("counter".into(), InspectValue::Text(self.counter_text()));
        }
        d.properties
            .insert("multiline".into(), InspectValue::Bool(self.multi));
        d.properties
            .insert("read_only".into(), InspectValue::Bool(self.read_only));
        if let Some(error) = self.rejected {
            d.properties.insert(
                "binding_error".into(),
                InspectValue::Text(error.to_string()),
            );
        }
        if let Some(error) = &self.rejection {
            // A normalizer's message may quote the entered text, so a
            // concealed field reports only that a save was refused.
            if self.concealed {
                d.properties
                    .insert("validation_refused".into(), InspectValue::Bool(true));
            } else {
                d.properties.insert(
                    "validation_error".into(),
                    InspectValue::Text(error.to_string()),
                );
            }
        }
        if self.drafting() {
            d.properties
                .insert("dirty".into(), InspectValue::Bool(self.dirty()));
        }
        if self.modified_indicator {
            d.properties
                .insert("modified_indicator".into(), InspectValue::Bool(true));
        }
        if self.clearable && !self.multi {
            d.properties
                .insert("clear_shown".into(), InspectValue::Bool(self.clear_shown()));
        }
        if !self.suffix.is_empty() && !self.multi {
            d.properties
                .insert("suffix".into(), InspectValue::Text(self.suffix.clone()));
        }
        if self.leading.is_some() && !self.multi {
            d.properties
                .insert("leading_icon".into(), InspectValue::Bool(true));
        }
        d.properties
            .insert("enabled".into(), InspectValue::Bool(self.enabled));
        if !self.concealed {
            d.properties.insert(
                "selection".into(),
                InspectValue::Text(format!(
                    "{}..{}",
                    self.selection.get().anchor_byte(),
                    self.selection.get().focus_byte()
                )),
            );
        }
        d.properties.insert(
            "composing".into(),
            InspectValue::Bool(self.preedit.is_some()),
        );
        d.properties.insert(
            "scroll_x".into(),
            InspectValue::Number(f64::from(self.scroll.get()[0])),
        );
        d.properties.insert(
            "scroll_y".into(),
            InspectValue::Number(f64::from(self.scroll.get()[1])),
        );
        d.properties.insert(
            "typography".into(),
            InspectValue::Text(format!("{:?}", self.style)),
        );
        d.validation.interactive = self.enabled;
        let mut semantics = Semantics::new(if self.multi {
            SemanticRole::MultilineTextInput
        } else {
            SemanticRole::TextInput
        });
        if self.dirty() && !self.modified_text.is_empty() {
            // An unsaved draft is described, whether or not it is painted.
            semantics = semantics.description(self.modified_text.clone());
        }
        d.semantics = Some(
            semantics
                .protected(self.concealed)
                .value(SemanticValue::Text(self.text.clone()))
                .required(self.required)
                .read_only(self.locked())
                .invalid(self.rejected.is_some() || self.rejection.is_some())
                .disabled(!self.enabled)
                .enabled_actions((!self.locked()).then_some(SemanticAction::SetValue)),
        );
        d
    }
}
/// Checks text against a bound editor's rules: at most [`TextEditError::MAX_TEXT_BYTES`]
/// bytes, and no control characters or Unicode line separators, except LF and
/// tab in a multiline editor.
fn check_text(text: &str, multi: bool) -> Result<(), TextEditError> {
    if text.len() > crate::TextEditError::MAX_TEXT_BYTES {
        return Err(TextEditError::TooLarge);
    }
    if text.chars().any(|c| {
        (c.is_control() && !(multi && matches!(c, '\n' | '\t')))
            || matches!(c, '\u{2028}' | '\u{2029}')
    }) {
        return Err(TextEditError::InvalidText);
    }
    Ok(())
}
fn offset(rect: Rect, origin: [f32; 2]) -> Result<Rect, crate::GeometryError> {
    let [x, y, w, h] = rect.values();
    Rect::new(x + origin[0], y + origin[1], w, h)
}

/// What an editor answers for a key its editing did not handle: a key that
/// types text claims that text, whether or not the editor accepts it (read
/// only, refused binding), so a command bound to the same key never runs while
/// an editor is focused. Other keys bubble.
pub(crate) fn unhandled_text_key(
    cx: &EventContext,
    input: CommandKeyInput,
    transition: KeyTransition,
) -> TextKeyOutcome {
    let modifiers = cx.modifiers();
    if input.has_text
        && !modifiers.super_key
        && (!modifiers.control || modifiers.alt)
        && matches!(transition, KeyTransition::Pressed | KeyTransition::Repeated)
    {
        TextKeyOutcome::InsertText
    } else {
        TextKeyOutcome::Ignored
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `Editor::new` falls back to no menu if construction ever failed, which would
    /// silently remove the clipboard menu from every bound field. The keys are
    /// distinct constants today; this keeps that true.
    #[test]
    fn field_menu_is_always_constructible() {
        let ui = Runtime::new();
        let value = ui.signal(String::new());
        assert!(FieldMenu::new(&value).is_ok());
        let mut keys: Vec<_> = FieldMenu::ACTIONS.iter().map(|(_, key, _)| *key).collect();
        keys.sort_unstable();
        let unique = keys.len();
        keys.dedup();
        assert_eq!(keys.len(), unique, "menu action keys must stay distinct");
        assert!(
            keys.iter().all(|key| !key.trim().is_empty()),
            "menu action keys must stay nonempty"
        );
        assert!(Editor::<()>::new(Some(value), false).menu.is_some());
    }
}
