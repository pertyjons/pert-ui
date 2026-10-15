//! Consumer contract exercised exclusively through the public facade.

use pert_ui::NormalizedValueError;
use pert_ui::prelude::*;

#[test]
fn a_consumer_can_validate_and_report_input_errors() -> Result<(), NormalizedValueError> {
    let position = NormalizedValue::try_from(0.75)?;
    assert_eq!(position.get(), 0.75);

    let error = match NormalizedValue::try_from(2.0) {
        Ok(_) => panic!("invalid input must be rejected"),
        Err(error) => error,
    };
    assert_eq!(error, NormalizedValueError::OutOfRange);
    let diagnostic: &dyn std::error::Error = &error;
    assert!(!diagnostic.to_string().is_empty());
    Ok(())
}

#[test]
fn consumers_can_compose_general_parameter_defaults() -> Result<(), Box<dyn std::error::Error>> {
    let spec = ParameterSpec::new("Scale", 1.0, 100.0, 10.0)?
        .with_mapping(ParameterMapping::Logarithmic)?;
    let id = ParameterId::new(1, 2, "scale")?;
    assert_eq!(spec.normalize(&spec.default_value())?.get(), 0.5);
    assert_eq!(spec.denormalize(NormalizedValue::MAX), 100.0);
    assert_ne!(id, id.next_incarnation()?);
    Ok(())
}

#[cfg(feature = "desktop")]
#[test]
fn invalid_app_configuration_fails_before_native_startup() {
    let invalid = pert_ui::App::new("   ", |_| panic!("invalid title must skip initialization"));
    assert!(matches!(
        invalid.run(),
        Err(pert_ui::AppError::InvalidTitle)
    ));
    // A test runs off the main thread, where the event loop cannot exist: the
    // call fails while creating it, before the host's content is built.
    let initialized = std::rc::Rc::new(std::cell::Cell::new(false));
    let flag = initialized.clone();
    let off_main = pert_ui::App::new("Example", move |_| {
        flag.set(true);
        Ok(())
    });
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| off_main.run()));
    assert!(
        outcome.is_err(),
        "the windowing backend refuses this thread"
    );
    assert!(!initialized.get(), "initialization did not run");
}
