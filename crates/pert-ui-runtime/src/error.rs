//! Errors shared across modules: the application's own failures carried through
//! library callbacks, and the conversions of leaf errors into the higher levels.
//!
//! The policy these follow is in `docs/contracts.md`, "Error policy".
use std::{error::Error, fmt, sync::Arc};

/// An application's own failure, carried through a library callback that
/// returns [`RuntimeError`](crate::RuntimeError).
///
/// Component, command and initialization callbacks return library errors. When
/// the reason is the application's own, such as an invalid configuration file
/// or a refused engine command, wrap it here instead of borrowing a library
/// variant that means something else. [`RuntimeError::Application`](crate::RuntimeError::Application)
/// carries it, and `?` converts it. It also carries a
/// [`WorkspaceError`](crate::WorkspaceError) or
/// [`CaptureError`](crate::CaptureError) that a callback propagates with `?`
/// from a runtime method it called, and a [`WindowError`](crate::WindowError)
/// it propagates from a [`PresentationStatus::Failed`](crate::PresentationStatus::Failed)
/// report; [`Self::downcast_ref`] recovers it.
///
/// The value is shared, so cloning is cheap and the original error is kept
/// whole, reachable through [`Self::get`] and [`Self::downcast_ref`]. It is
/// transparent: `Display` shows the wrapped error's message and
/// [`source`](Error::source) continues with the wrapped error's own source, so
/// a reporter walking the chain prints each message once. Two values compare
/// equal only when they share one wrapped error (a clone of the same value):
/// the library cannot compare foreign errors, and equality by message would
/// equate unrelated failures.
///
/// ```
/// # use pert_ui_runtime as pert_ui;
/// use pert_ui::{ApplicationError, RuntimeError};
///
/// fn load() -> Result<u32, RuntimeError> {
///     let text = "not a number";
///     Ok(text.parse::<u32>().map_err(ApplicationError::new)?)
/// }
/// let error = load().unwrap_err();
/// assert_eq!(error.to_string(), "invalid digit found in string");
/// let RuntimeError::Application(failure) = &error else {
///     panic!("an application failure");
/// };
/// assert!(failure.downcast_ref::<std::num::ParseIntError>().is_some());
/// // A message alone is enough for a failure with no error type of its own.
/// let refused = RuntimeError::from(ApplicationError::from("the engine is offline"));
/// assert_eq!(refused.to_string(), "the engine is offline");
/// ```
#[derive(Clone)]
pub struct ApplicationError(Arc<dyn Error + Send + Sync + 'static>);

impl ApplicationError {
    /// Wraps an application error.
    ///
    /// A boxed `Box<dyn Error + Send + Sync>` is not itself an `Error`; it
    /// converts with [`ApplicationError::from`], which keeps the boxed error
    /// and its source chain, so `?` on such a result reads
    /// `.map_err(ApplicationError::from)?`. A non-`Send` `Box<dyn Error>`
    /// cannot be carried; wrap its message instead.
    pub fn new(error: impl Error + Send + Sync + 'static) -> Self {
        Self(Arc::new(error))
    }
    /// The wrapped error.
    #[must_use]
    pub fn get(&self) -> &(dyn Error + Send + Sync + 'static) {
        &*self.0
    }
    /// The wrapped error as its concrete type, when it is a `T`.
    #[must_use]
    pub fn downcast_ref<T: Error + 'static>(&self) -> Option<&T> {
        self.0.downcast_ref::<T>()
    }
}

impl From<String> for ApplicationError {
    /// An error whose message is `message`, for a failure with no error type
    /// of its own.
    fn from(message: String) -> Self {
        Self::from(Box::<dyn Error + Send + Sync>::from(message))
    }
}
impl From<&str> for ApplicationError {
    /// An error whose message is `message`, for a failure with no error type
    /// of its own.
    fn from(message: &str) -> Self {
        Self::from(Box::<dyn Error + Send + Sync>::from(message))
    }
}
impl From<Box<dyn Error + Send + Sync + 'static>> for ApplicationError {
    /// Takes over a boxed error, such as the result of `"message".into()`.
    fn from(error: Box<dyn Error + Send + Sync + 'static>) -> Self {
        Self(Arc::from(error))
    }
}

impl PartialEq for ApplicationError {
    /// Identity: equal only for values sharing one wrapped error.
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}
impl Eq for ApplicationError {}

impl fmt::Debug for ApplicationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("ApplicationError").field(&self.0).finish()
    }
}

impl fmt::Display for ApplicationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

impl Error for ApplicationError {
    /// The wrapped error's own source: the wrapped error's message is this
    /// value's message, so returning the wrapped error would print it twice.
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.0.source()
    }
}

/// Implements `From<leaf>` for a higher-level error through its `Runtime`
/// variant, so `?` reaches it from every leaf error `RuntimeError` wraps and a
/// leaf keeps exactly one form there.
///
/// This is the one list of leaf errors; the facade's `AppError` converts from
/// whatever `DrawError` converts from. The test at the end of this module fails
/// to compile when `RuntimeError` gains a leaf this list lacks.
macro_rules! leaf_conversions {
    ($target:ty => $variant:path) => {
        $crate::error::leaf_conversions!(@each $target => $variant;
            $crate::GeometryError, $crate::LayoutError, $crate::ChoiceError,
            $crate::BindingError, $crate::EditError, $crate::DeliveryError,
            $crate::ApplicationError, $crate::TextError, $crate::TextEditError,
            $crate::FontError, $crate::ClipboardError, $crate::NumberInputError,
            $crate::ParameterError, $crate::ParameterIdError,
            $crate::NormalizedValueError, $crate::LiveError);
    };
    (@each $target:ty => $variant:path; $($leaf:ty),+) => {
        $(
            impl From<$leaf> for $target {
                fn from(error: $leaf) -> Self {
                    $variant($crate::RuntimeError::from(error))
                }
            }
        )+
    };
}
pub(crate) use leaf_conversions;

#[cfg(test)]
mod tests {
    use crate::{
        ApplicationError, BindingError, ChoiceError, ClipboardError, DeliveryError, DrawError,
        EditError, FontError, GeometryError, LayoutError, LiveError, NormalizedValueError,
        NumberInputError, ParameterError, ParameterIdError, RuntimeError, TextEditError, TextError,
        WindowError, WorkspaceError,
    };

    /// Compiles only if every higher-level error converts from `E`, which
    /// [`leaf_conversions`](super::leaf_conversions) provides. The facade's
    /// `AppError` converts from whatever `DrawError` converts from, so this
    /// check covers it too.
    fn leaf<E>()
    where
        RuntimeError: From<E>,
        DrawError: From<E>,
        WindowError: From<E>,
        WorkspaceError: From<E>,
    {
    }

    /// Every leaf error `RuntimeError` wraps reaches every higher-level error
    /// through `?`. The match is exhaustive inside this crate, so a new
    /// variant does not compile until it is sorted here: a wrapped leaf is
    /// checked with [`leaf`], and a unit variant joins the last arm.
    #[test]
    fn every_wrapped_leaf_converts_into_every_higher_level_error() {
        let check = |error: &RuntimeError| match error {
            RuntimeError::Binding(binding) => {
                leaf::<BindingError>();
                match binding {
                    BindingError::Edit(_) => leaf::<EditError>(),
                    BindingError::Delivery(_) => leaf::<DeliveryError>(),
                    BindingError::DuplicateParameter | BindingError::KindMismatch { .. } => {}
                }
            }
            RuntimeError::Application(_) => leaf::<ApplicationError>(),
            RuntimeError::NumberInput(_) => leaf::<NumberInputError>(),
            RuntimeError::TextEdit(_) => leaf::<TextEditError>(),
            RuntimeError::Clipboard(_) => leaf::<ClipboardError>(),
            RuntimeError::Parameter(_) => leaf::<ParameterError>(),
            RuntimeError::ParameterId(_) => leaf::<ParameterIdError>(),
            RuntimeError::NormalizedValue(_) => leaf::<NormalizedValueError>(),
            RuntimeError::Live(_) => leaf::<LiveError>(),
            RuntimeError::Choice(_) => leaf::<ChoiceError>(),
            RuntimeError::Geometry(_) => leaf::<GeometryError>(),
            RuntimeError::Layout(_) => leaf::<LayoutError>(),
            RuntimeError::Text(_) => leaf::<TextError>(),
            RuntimeError::Font(_) => leaf::<FontError>(),
            RuntimeError::InvalidPresentationRate
            | RuntimeError::TaskQueueFull
            | RuntimeError::BackgroundFull
            | RuntimeError::BackgroundSpawn(_)
            | RuntimeError::BackgroundPanicked
            | RuntimeError::IntervalsFull
            | RuntimeError::EmptyCommandName
            | RuntimeError::ShortcutConflict(_)
            | RuntimeError::NotDetachable
            | RuntimeError::StaleCommand
            | RuntimeError::StaleCommandTarget
            | RuntimeError::WindowRequestFull
            | RuntimeError::RevealQueueFull
            | RuntimeError::CommandQueueFull
            | RuntimeError::AlreadyDetached
            | RuntimeError::NotDetached
            | RuntimeError::StalePresentation
            | RuntimeError::EmptyWindowTitle
            | RuntimeError::ModalConflict
            | RuntimeError::ModalBlocked
            | RuntimeError::DialogBusy
            | RuntimeError::DraftEditorBusy
            | RuntimeError::ToastFull
            | RuntimeError::ForeignRuntime
            | RuntimeError::Unmounted
            | RuntimeError::Hidden
            | RuntimeError::DuplicateKey(_)
            | RuntimeError::EmptyKey
            | RuntimeError::AmbiguousKey(_)
            | RuntimeError::WriteDuringUpdate
            | RuntimeError::WriteDuringPaint
            | RuntimeError::WriteDuringInspection
            | RuntimeError::WriteDuringDelivery
            | RuntimeError::BorrowConflict
            | RuntimeError::Closed
            | RuntimeError::TimeReversed
            | RuntimeError::Exhausted
            | RuntimeError::UpdateLimit
            | RuntimeError::ZeroDelay
            | RuntimeError::GestureInProgress
            | RuntimeError::WriteDuringLayout
            | RuntimeError::ChildTypeChanged(_)
            | RuntimeError::CaptureUnavailable
            | RuntimeError::DragUnavailable
            | RuntimeError::DropNotPermitted
            | RuntimeError::NotMovable => {}
        };
        check(&RuntimeError::Closed);
        check(&RuntimeError::Geometry(GeometryError::NonFinite));
    }
}
