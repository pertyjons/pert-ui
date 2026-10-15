//! Commit-time normalization shared by every input field.
use super::*;

/// Why a field's normalizer refused the value the user entered.
///
/// A normalizer, installed with `normalize` on [`TextField`], [`TextArea`],
/// [`InlineText`], [`IntegerInput`], [`DecimalInput`], [`InlineInteger`] or
/// [`InlineDecimal`], returns this to refuse a value instead of adjusting it.
/// The field then writes nothing: it keeps the draft for correction, draws the
/// error outline, marks its semantics invalid and reports the message as
/// `validation_error` in inspection. The message is plain text meant for the
/// user, such as "Enter a whole number of tenths". The field's
/// `on_rejection` callback passes it to the application, for example to show
/// it as the field's [`FieldStatus`](crate::FieldStatus).
///
/// ```
/// use pert_ui_runtime::InputRejection;
/// let rejection = InputRejection::new("Name must not be empty");
/// assert_eq!(rejection.message(), "Name must not be empty");
/// assert_eq!(rejection.to_string(), "Name must not be empty");
/// ```
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
#[error("{message}")]
pub struct InputRejection {
    message: String,
}
impl InputRejection {
    /// Creates a rejection with a user-visible message.
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
    /// The user-visible message.
    pub fn message(&self) -> &str {
        &self.message
    }
}

/// A field's normalizer: the application's adjustment of an entered value.
pub(crate) type Normalizer<T> = Rc<dyn Fn(T) -> Result<T, InputRejection>>;

/// Why a draft cannot be saved: the field's own validation or the normalizer.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum EntryError {
    /// The numeric draft does not parse or is out of the field's domain.
    Number(NumberInputError),
    /// Normalized text is not acceptable bound text.
    Text(TextEditError),
    /// The normalizer refused the value.
    Rejected(InputRejection),
}
impl std::fmt::Display for EntryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Number(error) => error.fmt(f),
            Self::Text(error) => error.fmt(f),
            Self::Rejected(rejection) => rejection.fmt(f),
        }
    }
}
impl From<NumberInputError> for EntryError {
    fn from(error: NumberInputError) -> Self {
        Self::Number(error)
    }
}
impl From<InputRejection> for EntryError {
    fn from(rejection: InputRejection) -> Self {
        Self::Rejected(rejection)
    }
}

/// Normalizes entered text and checks the result as bound text of an editor
/// with the given line policy.
pub(crate) fn normalize_text(
    normalizer: &Normalizer<String>,
    text: String,
    multi: bool,
) -> Result<String, EntryError> {
    let text = normalizer(text)?;
    check_text(&text, multi).map_err(EntryError::Text)?;
    Ok(text)
}

/// An application callback told about a normalizer's refusals.
pub(crate) type RejectionCallback =
    Box<dyn FnMut(&mut EventContext, Option<&InputRejection>) -> Result<(), RuntimeError>>;

/// Reports a field's normalizer refusals to the application: `Some` on each
/// save attempt the normalizer refuses, and `None` once when a reported
/// refusal ends. Every call happens in the event phase.
#[derive(Default)]
pub(crate) struct RejectionReport {
    callback: Option<RejectionCallback>,
    /// A refusal was reported and has not been reported as ended.
    reported: bool,
}
impl RejectionReport {
    pub(crate) fn set(&mut self, callback: RejectionCallback) {
        self.callback = Some(callback);
    }
    /// Takes a new description's callback, or none, keeping whether a refusal
    /// is still reported: its end goes to the new callback.
    pub(crate) fn adopt(&mut self, next: Self) {
        self.callback = next.callback;
    }
    /// Reports the outcome of a save attempt: its error, or `None` when the
    /// draft was saved or there was nothing to save.
    pub(crate) fn outcome(
        &mut self,
        cx: &mut EventContext,
        error: Option<&EntryError>,
    ) -> Result<(), RuntimeError> {
        match error {
            Some(EntryError::Rejected(rejection)) => {
                self.reported = true;
                match &mut self.callback {
                    Some(callback) => callback(cx, Some(rejection)),
                    None => Ok(()),
                }
            }
            _ => self.clear(cx),
        }
    }
    /// Reports that a reported refusal ended, once. A failed report stays
    /// pending, so the next save or Escape reports it again.
    pub(crate) fn clear(&mut self, cx: &mut EventContext) -> Result<(), RuntimeError> {
        if !self.reported {
            return Ok(());
        }
        if let Some(callback) = &mut self.callback {
            callback(cx, None)?;
        }
        self.reported = false;
        Ok(())
    }
}

/// The application's `on_save` callback with a locally written or submitted value.
pub(crate) type SaveCallback<T> = Box<dyn FnMut(&mut EventContext, &T) -> Result<(), RuntimeError>>;

/// Reports each user save accepted as a local write or a remote submission.
pub(crate) struct SaveReport<T: ?Sized> {
    callback: Option<SaveCallback<T>>,
}
impl<T: ?Sized> Default for SaveReport<T> {
    fn default() -> Self {
        Self { callback: None }
    }
}
impl<T: ?Sized> SaveReport<T> {
    pub(crate) fn set(&mut self, callback: SaveCallback<T>) {
        self.callback = Some(callback);
    }
    /// Takes a new description's callback, or none.
    pub(crate) fn adopt(&mut self, next: Self) {
        self.callback = next.callback;
    }
    /// Reports a value a save has just written to the binding.
    pub(crate) fn saved(&mut self, cx: &mut EventContext, value: &T) -> Result<(), RuntimeError> {
        match &mut self.callback {
            Some(callback) => callback(cx, value),
            None => Ok(()),
        }
    }
}
