//! The error policy in `docs/contracts.md`: message style, source chains and
//! the one upward conversion graph, checked on representative values.
use pert_ui_runtime::*;
use std::error::Error;

/// An application error with a cause of its own, to exercise a real chain.
#[derive(Debug)]
struct ConfigError(std::io::Error);
impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("configuration could not be loaded")
    }
}
impl Error for ConfigError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.0)
    }
}

fn application() -> ApplicationError {
    ApplicationError::new(ConfigError(std::io::Error::other("disk full")))
}

/// One value of each error type, with the wrapping variants filled in.
fn samples() -> Vec<Box<dyn Error>> {
    let runtime = [
        RuntimeError::Geometry(GeometryError::ZoomOutOfRange),
        RuntimeError::Geometry(GeometryError::ViewScaleOutOfRange),
        RuntimeError::Binding(BindingError::KindMismatch {
            expected: ParameterFamily::Continuous,
            found: ParameterFamily::Choice,
        }),
        RuntimeError::Binding(BindingError::Edit(EditError::InvalidValue(
            ParameterError::NonFinite,
        ))),
        RuntimeError::Binding(BindingError::Delivery(DeliveryError::Faulted)),
        RuntimeError::Choice(ChoiceError::DuplicateValue {
            first: 0,
            second: 2,
        }),
        RuntimeError::AmbiguousKey("apply".into()),
        RuntimeError::DuplicateKey("apply".into()),
        RuntimeError::ChildTypeChanged("apply".into()),
        RuntimeError::ShortcutConflict(KeyChord::primary('s')),
        RuntimeError::Layout(LayoutError::UnknownKey("files".into())),
        RuntimeError::Font(FontError::UnknownIcon("folder_opne".into())),
        RuntimeError::Font(FontError::MissingGlyph('q')),
        RuntimeError::NumberInput(NumberInputError::InvalidPrecision(16)),
        RuntimeError::Application(application()),
        RuntimeError::Text(TextError::TooLarge(TextLimit::Segments)),
        RuntimeError::Text(TextError::TooLarge(TextLimit::RasterSide)),
        RuntimeError::Text(TextError::TooLarge(TextLimit::Coverage)),
        RuntimeError::TextEdit(TextEditError::TooLarge),
        RuntimeError::Clipboard(ClipboardError::Failed("X11 timeout".into())),
        RuntimeError::Clipboard(ClipboardError::Failed(String::new())),
        RuntimeError::Layout(LayoutError::NoLanes),
        RuntimeError::Live(LiveError::CapacityExceeded(2_000_000)),
        RuntimeError::ParameterId(ParameterIdError::EmptyKey),
        RuntimeError::NormalizedValue(NormalizedValueError::OutOfRange),
        RuntimeError::UpdateLimit,
    ];
    let mut samples: Vec<Box<dyn Error>> = Vec::new();
    for error in runtime {
        samples.push(Box::new(DrawError::Runtime(error.clone())));
        samples.push(Box::new(WindowError::Runtime(error.clone())));
        samples.push(Box::new(WorkspaceError::Runtime(error.clone())));
        samples.push(Box::new(error));
    }
    samples.extend([
        Box::new(DrawError::Path(PathError::TooLarge)) as Box<dyn Error>,
        Box::new(WindowError::Creation("no display".into())),
        Box::new(WindowError::Drawing(DrawError::InvalidImage {
            width: 0,
            height: 4,
            bytes: 0,
        })),
        Box::new(DrawError::CoverageTooLarge),
        Box::new(SinkError::new(SinkErrorKind::Full, "engine queue full")),
        Box::new(WorkspaceError::TooManyEntries),
        Box::new(CaptureError::Backend("device lost".into())),
        Box::new(FontError::TooLarge),
        Box::new(NumberInputError::OffStep),
        Box::new(NumberLocaleError::SameSeparators {
            decimal: ',',
            grouping: ',',
        }),
        Box::new(TextEditError::InvalidSelection {
            anchor_byte: 1,
            focus_byte: 2,
        }),
        Box::new(WorkspaceError::Participant {
            path: "main/table".into(),
            source: Box::new(WorkspaceError::TextTooLong),
        }),
        Box::new(RestoreRejection::State(WorkspaceError::TextTooLong)),
        Box::new(RestoreRejection::Runtime(RuntimeError::Closed)),
        Box::new(CaptureError::Limit(LimitError::FrameSize {
            width: 5000,
            height: 5000,
        })),
        Box::new(ListingError::new(ListingErrorKind::NotFound, "not found").with_path("/missing")),
        Box::new(RequestError::Full),
        Box::new(WaitError::Timeout),
        Box::new(WakeError::Closed),
        Box::new(SendError::Full(1_u8)),
        Box::new(RestoreRejection::OutOfRange("width".into())),
    ]);
    samples
}

#[test]
fn messages_are_lowercase_sentences_without_debug_formatting() {
    for error in samples() {
        let message = error.to_string();
        let first = message.chars().next().expect("a message");
        assert!(!first.is_uppercase(), "starts in uppercase: {message}");
        assert!(!message.ends_with('.'), "ends with a period: {message}");
        for debug in ["Continuous", "Choice", "Full:", "Some(", "\"", "{", "::"] {
            assert!(!message.contains(debug), "shows {debug:?}: {message}");
        }
    }
}

#[test]
fn a_source_chain_prints_each_message_once() {
    for error in samples() {
        let mut current: &dyn Error = &*error;
        let mut messages = vec![current.to_string()];
        while let Some(source) = current.source() {
            let cause = source.to_string();
            assert!(
                !current.to_string().contains(&cause),
                "`{current}` repeats its source `{cause}`"
            );
            messages.push(cause);
            current = source;
        }
        let mut unique = messages.clone();
        unique.dedup();
        assert_eq!(unique, messages, "a message repeats in the chain");
    }
    // The application's error keeps its own cause in the chain.
    let error = RuntimeError::from(application());
    assert_eq!(error.to_string(), "configuration could not be loaded");
    assert_eq!(
        error.source().map(ToString::to_string).as_deref(),
        Some("disk full")
    );
}

/// A rejected key, identity or value is named in the message, so a report
/// says which one failed without the caller's own context.
#[test]
fn rejected_keys_and_values_are_named_in_the_message() {
    let cases: [(Box<dyn Error>, &str); 8] = [
        (
            Box::new(RuntimeError::AmbiguousKey("apply".into())),
            "several descendants share the key `apply`",
        ),
        (
            Box::new(LayoutError::DuplicateKey("files".into())),
            "two items share the key `files`",
        ),
        (
            Box::new(ChoiceError::DuplicateValue {
                first: 0,
                second: 2,
            }),
            "choices 0 and 2 have equal values",
        ),
        (
            Box::new(LiveError::CapacityExceeded(2_000_000)),
            "live-data capacity of 2000000 items exceeds the maximum of 1048576 items",
        ),
        (
            Box::new(NumberInputError::InvalidPrecision(16)),
            "decimal precision 16 must be between 0 and 15",
        ),
        (
            Box::new(FontError::UnknownIcon("folder_opne".into())),
            "no Material icon is named `folder_opne`",
        ),
        (
            Box::new(TextEditError::InvalidComposition {
                start_byte: 2,
                end_byte: 1,
            }),
            "composition cursor 2..1 is not an ordered range of character boundaries",
        ),
        (
            Box::new(CaptureError::Limit(LimitError::FrameSize {
                width: 5000,
                height: 5000,
            })),
            "a 5000×5000 frame exceeds the frame image size limit",
        ),
    ];
    for (error, message) in cases {
        assert_eq!(error.to_string(), message);
    }
    // The participant's cause is its source, not part of its message.
    let participant = WorkspaceError::Participant {
        path: "main/table".into(),
        source: Box::new(WorkspaceError::TextTooLong),
    };
    assert_eq!(
        participant.to_string(),
        "workspace participant `main/table` could not build its state"
    );
    assert_eq!(
        participant.source().map(ToString::to_string),
        Some(WorkspaceError::TextTooLong.to_string())
    );
    let rejection = RestoreRejection::Runtime(RuntimeError::Closed);
    assert_eq!(
        rejection.source().map(ToString::to_string),
        Some(RuntimeError::Closed.to_string())
    );
}

#[test]
fn limits_and_ranges_are_named_in_the_message() {
    assert_eq!(
        GeometryError::ViewScaleOutOfRange.to_string(),
        "view scale is outside 0.03125..=32"
    );
    assert_eq!(
        GeometryError::ZoomOutOfRange.to_string(),
        "zoom factor is outside 0.75..=2"
    );
    assert_eq!(
        TextEditError::TooLarge.to_string(),
        "text exceeds the 16384 byte limit"
    );
    assert_eq!(
        TextError::TooLarge(TextLimit::Segments).to_string(),
        "text preparation exceeds its limit of 1024 rich-text segments"
    );
    assert_eq!(
        BindingError::KindMismatch {
            expected: ParameterFamily::Continuous,
            found: ParameterFamily::Boolean,
        }
        .to_string(),
        "expected a continuous parameter value, found boolean"
    );
    assert_eq!(
        ClipboardError::Failed("X11 timeout".into()).to_string(),
        "clipboard operation failed: X11 timeout"
    );
    assert_eq!(
        ClipboardError::Failed(String::new()).to_string(),
        "clipboard operation failed"
    );
}

#[test]
fn geometry_constructors_share_one_error_and_name_the_reason() {
    assert_eq!(Point::new(f32::NAN, 0.0), Err(GeometryError::NonFinite));
    assert_eq!(Size::new(-1.0, 0.0), Err(GeometryError::Negative));
    assert_eq!(Length::new(f32::INFINITY), Err(GeometryError::NonFinite));
    assert_eq!(Rect::new(0.0, 0.0, 1.0, -1.0), Err(GeometryError::Negative));
    assert_eq!(
        Viewport::new(10.0, 10.0, 0.0).err(),
        Some(GeometryError::NonPositiveScale)
    );
    assert_eq!(Transform2D::scale(0.0, 1.0), Err(GeometryError::Singular));
    assert_eq!(
        AxisMapping::new(0.0, -1.0),
        Err(GeometryError::NonPositiveScale)
    );
    assert_eq!(
        Magnification::new(0.0),
        Err(GeometryError::NonPositiveScale)
    );
    assert_eq!(UiZoom::new(3.0), Err(GeometryError::ZoomOutOfRange));
    assert_eq!(UiZoom::new(f64::NAN), Err(GeometryError::NonFinite));
    assert_eq!(UiZoom::MIN.get(), 0.75);
    assert_eq!(UiZoom::MAX.get(), 2.0);
    let small = Size::new(10.0, 10.0).expect("valid size");
    assert_eq!(
        SizeLimits::new(small, Constraints::new(Some(5.0), None).expect("valid")),
        Err(GeometryError::InvertedBounds)
    );
}

/// Validated quantities share one shape: `new` and `TryFrom` agree, `get`
/// reads the plain value, and documented limits are values of the type.
#[test]
fn validated_quantities_share_one_shape() {
    assert_eq!(Length::try_from(-1.0), Err(GeometryError::Negative));
    assert_eq!(Length::try_from(4.0).map(Length::get), Ok(4.0));
    assert_eq!(
        PresentationRate::try_from(0.5),
        Err(RuntimeError::InvalidPresentationRate)
    );
    assert_eq!(
        PresentationRate::try_from(120.0).map(PresentationRate::hertz),
        Ok(120.0)
    );
    assert_eq!(ViewScale::new(ViewScale::MIN.get()), Ok(ViewScale::MIN));
    assert_eq!(ViewScale::new(ViewScale::MAX.get()), Ok(ViewScale::MAX));
    assert_eq!(UiZoom::new(UiZoom::MAX.get()), Ok(UiZoom::MAX));
    assert_eq!(NormalizedValue::MAX.get(), 1.0);
}

#[test]
fn leaf_errors_have_one_form_at_every_level() {
    let leaf = GeometryError::NonFinite;
    let runtime = RuntimeError::Geometry(leaf);
    assert_eq!(RuntimeError::from(leaf), runtime);
    assert_eq!(DrawError::from(leaf), DrawError::Runtime(runtime.clone()));
    assert_eq!(
        WindowError::from(leaf),
        WindowError::Runtime(runtime.clone())
    );
    assert_eq!(
        WorkspaceError::from(leaf),
        WorkspaceError::Runtime(runtime.clone())
    );
    // A drawing failure that holds a runtime error keeps the runtime form.
    assert_eq!(
        WindowError::from(DrawError::Runtime(runtime.clone())),
        WindowError::Runtime(runtime)
    );
    assert_eq!(
        WindowError::from(DrawError::InvalidColor),
        WindowError::Drawing(DrawError::InvalidColor)
    );
    // Edit and delivery refusals are binding failures, whichever way they arrive.
    assert_eq!(
        RuntimeError::from(EditError::Busy),
        RuntimeError::Binding(BindingError::Edit(EditError::Busy))
    );
    assert_eq!(
        RuntimeError::from(DeliveryError::Faulted),
        RuntimeError::Binding(BindingError::Delivery(DeliveryError::Faulted))
    );
    assert_eq!(
        RuntimeError::from(ChoiceError::EmptyValue),
        RuntimeError::Choice(ChoiceError::EmptyValue)
    );
    assert_eq!(
        RuntimeError::from(LiveError::ZeroCapacity),
        RuntimeError::Live(LiveError::ZeroCapacity)
    );
}

#[test]
fn application_errors_compare_by_identity() {
    let failure = application();
    assert_eq!(failure, failure.clone());
    assert_ne!(
        failure,
        application(),
        "equal messages are different failures"
    );
    assert!(failure.downcast_ref::<ConfigError>().is_some());
    assert_eq!(
        RuntimeError::from(failure.clone()),
        RuntimeError::Application(failure)
    );
}
