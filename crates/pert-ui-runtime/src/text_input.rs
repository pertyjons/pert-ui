//! Platform-independent text composition and clipboard contracts.
use crate::Rect;

/// Text content was rejected without changing the bound value.
///
/// An editor reports it for a refused edit, and every other text path reports
/// [`Self::TooLarge`] for text over [`Self::MAX_TEXT_BYTES`]:
/// labels, rich text, toasts and preedit share this one variant for that limit.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum TextEditError {
    /// Text, combined rich text or combined composition exceeds
    /// [`Self::MAX_TEXT_BYTES`] of UTF-8.
    #[error("text exceeds the {} byte limit", Self::MAX_TEXT_BYTES)]
    TooLarge,
    /// A single-line binding contains a line break, or text contains unsupported
    /// control characters. Tabs and LF are accepted only in multiline bindings.
    #[error("text contains unsupported control characters or line breaks")]
    InvalidText,
    /// Composition cursor offsets are not ordered UTF-8 boundaries within preedit.
    #[error("invalid composition cursor range")]
    InvalidComposition,
    /// Selection offsets are not ordered UTF-8 character boundaries within the
    /// text they select.
    #[error("invalid selection range")]
    InvalidSelection,
}
impl TextEditError {
    /// Maximum UTF-8 byte length of one prepared text or bound editor string (16 KiB).
    ///
    /// It lives on this error because it bounds every text path, not one widget:
    /// [`Self::TooLarge`] is the one variant that reports it.
    ///
    /// Use `str::len`, not a character count, when checking a value before writing
    /// a [`crate::TextField`] or [`crate::TextArea`] binding. The editors reject
    /// oversized external values without truncating or rewriting the binding.
    /// This is also the combined byte limit of a rich text request. Every text over
    /// it, whether edited, prepared for a label or queued in a toast, is rejected
    /// with [`Self::TooLarge`]. Passing this check does not bypass character
    /// validation or the raster limits named by [`TextLimit`](crate::TextLimit).
    pub const MAX_TEXT_BYTES: usize = 16 * 1024;
}

/// Temporary IME text, kept separate from the committed binding.
///
/// The cursor is a UTF-8 byte range within this text, from the start of the
/// composition's selection to its caret; `None` hides the composition caret. Empty text cancels preedit. Delivery never changes physical key state.
#[derive(Clone, Debug, Eq, PartialEq)]
#[must_use]
pub struct TextPreedit {
    text: String,
    cursor: Option<std::ops::Range<usize>>,
}
impl TextPreedit {
    /// Validates the 16 KiB limit and the optional cursor range, whose `start`
    /// and `end` must be ordered UTF-8 boundaries within `text`. An empty range
    /// is a caret with no selection.
    ///
    /// ```
    /// # use pert_ui_runtime::{TextEditError, TextPreedit};
    /// let preedit = TextPreedit::new("åä", Some(2..4))?;
    /// assert_eq!(preedit.cursor(), Some(2..4));
    /// // Byte 1 is inside `å`, and a reversed range is not ordered.
    /// assert_eq!(TextPreedit::new("å", Some(1..2)), Err(TextEditError::InvalidComposition));
    /// let reversed = std::ops::Range { start: 2, end: 1 };
    /// assert_eq!(TextPreedit::new("ab", Some(reversed)), Err(TextEditError::InvalidComposition));
    /// # Ok::<(), TextEditError>(())
    /// ```
    ///
    /// # Errors
    /// Returns [`TextEditError::TooLarge`] above 16 KiB and
    /// [`TextEditError::InvalidComposition`] for an unordered range or one that
    /// does not lie on character boundaries.
    pub fn new(
        text: impl Into<String>,
        cursor: Option<std::ops::Range<usize>>,
    ) -> Result<Self, TextEditError> {
        let text = text.into();
        if text.len() > crate::TextEditError::MAX_TEXT_BYTES {
            return Err(TextEditError::TooLarge);
        }
        if cursor.as_ref().is_some_and(|range| {
            range.start > range.end
                || !text.is_char_boundary(range.start)
                || !text.is_char_boundary(range.end)
        }) {
            return Err(TextEditError::InvalidComposition);
        }
        Ok(Self { text, cursor })
    }
    /// Uncommitted UTF-8 content.
    pub fn text(&self) -> &str {
        &self.text
    }
    /// Optional selected byte range within preedit, whose `end` is the caret.
    #[must_use]
    pub fn cursor(&self) -> Option<std::ops::Range<usize>> {
        self.cursor.clone()
    }
}

/// A clipboard action a text editor performs on its own selection and caret.
///
/// This is the contract behind the built-in editors' clipboard chords and their
/// context menu, and the same one a custom editor implements through
/// [`Component::text_command`](crate::Component::text_command). Requesting a
/// command never moves focus and never reads the clipboard on the caller's
/// behalf; the receiving editor decides what its own selection means.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum TextCommand {
    /// Writes the selection to the clipboard, leaving the text unchanged.
    Copy,
    /// Writes the selection to the clipboard, then deletes it as one undo step.
    /// A failed clipboard write preserves the text.
    Cut,
    /// Replaces the selection with the clipboard text, or inserts it at the caret
    /// when nothing is selected.
    Paste,
    /// Inserts the clipboard text immediately before the word or selection at the
    /// caret, as a separate word.
    ///
    /// "Before" is logical order, not a visual side, so the result is the same in
    /// right-to-left runs. The clipboard text is trimmed and a single space is
    /// added on a side whose neighbor is neither whitespace nor the end of the
    /// text. With no word at the caret the nearest preceding word is used, which
    /// may be on an earlier line; with no such word the caret is. Any selection
    /// stays selected.
    PasteBefore,
    /// Inserts the clipboard text immediately after the word or selection at the
    /// caret, as a separate word. Mirrors [`Self::PasteBefore`] in every respect,
    /// searching forward when the caret touches no word.
    PasteAfter,
    /// Selects all text, without changing it or touching the clipboard. A
    /// text editor and a [`SelectableText`](crate::SelectableText) answer it;
    /// an empty text reports `Unavailable`.
    SelectAll,
}

/// Outcome of a [`TextCommand`] delivered to a component.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum TextCommandOutcome {
    /// The component is not a text editor and ignored the command.
    NotAnEditor,
    /// The editor understood the command but had nothing to do: no selection to
    /// copy, no clipboard text to paste, a read-only or disabled field, or a
    /// paste the field refused because its text would exceed
    /// [`TextEditError::MAX_TEXT_BYTES`](crate::TextEditError::MAX_TEXT_BYTES).
    Unavailable,
    /// The editor carried out the command.
    Handled,
}

/// A platform clipboard operation failed; the edit must preserve its source text.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum ClipboardError {
    /// No service or supported compositor clipboard protocol is available.
    #[error("clipboard is unavailable")]
    Unavailable,
    /// Clipboard has no UTF-8 text.
    #[error("clipboard contains no text")]
    NoText,
    /// Another client holds the platform clipboard right now; the same
    /// operation may succeed when retried later.
    #[error("clipboard is held by another client")]
    Busy,
    /// Reading or writing the clipboard failed for a reason the service does
    /// not classify, such as a display-server timeout. The text is the service's
    /// own description, which may be empty; it is diagnostic, not a stable value
    /// to match on.
    #[error("clipboard operation failed{}", describe(.0))]
    Failed(String),
}

/// The service's description after a separator, or nothing when it has none.
fn describe(description: &str) -> String {
    if description.is_empty() {
        String::new()
    } else {
        format!(": {description}")
    }
}

/// UI-thread plain-text clipboard service for custom hosts.
///
/// The native App installs its adapter automatically. Headless runtimes start
/// without a service. Implementations must return failures, retain ownership of
/// copied data as required by their platform, and never access live widgets.
/// Calls are synchronous ordinary UI operations, never safe for time-critical callbacks.
/// An implementation is `'static`, as
/// [`Runtime::set_clipboard`](crate::Runtime::set_clipboard) requires: it owns
/// its storage or shares it through an `Rc`, such as a struct holding an
/// `Rc<RefCell<String>>`, never borrowing from a local.
pub trait Clipboard {
    /// Reads owned UTF-8 text.
    /// The service may allocate the complete clipboard payload; editor limits
    /// bound accepted text, not operating-system or custom service allocations.
    ///
    /// # Errors
    /// An implementation returns [`ClipboardError::Unavailable`] when no clipboard service
    /// or protocol is reachable, [`ClipboardError::NoText`] when the clipboard holds no
    /// UTF-8 text, [`ClipboardError::Busy`] when another client holds it right now, and
    /// [`ClipboardError::Failed`] with its own description for any other failure.
    fn read_text(&mut self) -> Result<String, ClipboardError>;
    /// Writes text; cut must delete its selection only after this succeeds.
    ///
    /// # Errors
    /// An implementation returns [`ClipboardError::Unavailable`], [`ClipboardError::Busy`]
    /// or [`ClipboardError::Failed`] under the conditions of [`Self::read_text`]; the
    /// caller keeps its text.
    fn write_text(&mut self, text: &str) -> Result<(), ClipboardError>;
}

/// Focused component's IME candidate anchor in local logical coordinates.
///
/// Return only for enabled editable components. The runtime translates and clips
/// this to the window; platform handles and composition text are never exposed.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextInputState {
    /// Component-local native composition session. Increase after canceling
    /// composition (including external replacement or caret repositioning during
    /// composition); never reuse a value. On exhaustion disable IME by returning
    /// no state. Hosts must reset native composition when this or owner changes.
    pub session: u64,
    /// Current caret rectangle, relative to the component allocation.
    pub caret: Rect,
}
