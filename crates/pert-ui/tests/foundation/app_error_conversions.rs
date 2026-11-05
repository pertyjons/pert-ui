//! `AppError` accepts exactly the errors `DrawError` converts from, and keeps
//! one form for a runtime failure.
//!
//! The list below is the one place outside the runtime that names every
//! accepted type: when `leaf_conversions!` in `pert-ui-runtime/src/error.rs`
//! gains a leaf error, add it here too.

use pert_ui::{
    AppError, ApplicationError, BindingError, ChoiceError, ClipboardError, DeliveryError,
    DrawError, EditError, FontError, GeometryError, LayoutError, LiveError, NormalizedValueError,
    NumberInputError, ParameterError, ParameterIdError, PathError, RuntimeError, TextEditError,
    TextError,
};

/// Converts with `?`, as an application's initialization closure does.
fn convert<E>(error: E) -> AppError
where
    AppError: From<E>,
{
    let result: Result<(), AppError> = (|| Err(error)?)();
    match result {
        Err(error) => error,
        Ok(()) => unreachable!("the closure always fails"),
    }
}

fn is_runtime(error: &AppError) -> bool {
    matches!(error, AppError::Runtime(_))
}

#[test]
fn every_runtime_leaf_error_becomes_the_runtime_variant() {
    let converted = [
        convert(GeometryError::NonFinite),
        convert(LayoutError::BlankKey),
        convert(ChoiceError::EmptyValue),
        convert(BindingError::DuplicateParameter),
        convert(EditError::Busy),
        convert(DeliveryError::NoSubmissionSink),
        convert(ApplicationError::new(std::io::Error::other("disk full"))),
        convert(TextError::Font),
        convert(TextEditError::TooLarge),
        convert(FontError::InvalidData),
        convert(ClipboardError::NoText),
        convert(NumberInputError::InvalidStep),
        convert(ParameterError::EmptyName),
        convert(ParameterIdError::EmptyKey),
        convert(NormalizedValueError::OutOfRange),
        convert(LiveError::ZeroCapacity),
        convert(RuntimeError::InvalidPresentationRate),
        convert(DrawError::Runtime(RuntimeError::InvalidPresentationRate)),
    ];
    for error in &converted {
        assert!(is_runtime(error), "{error:?} is not AppError::Runtime");
    }
}

#[test]
fn other_drawing_errors_become_the_drawing_variant() {
    for error in [convert(PathError::Empty), convert(DrawError::TooComplex)] {
        assert!(
            matches!(error, AppError::Drawing(_)),
            "{error:?} is not AppError::Drawing"
        );
    }
}
