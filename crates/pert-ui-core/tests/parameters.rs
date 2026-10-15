//! Domain, mapping and identity contracts without a UI or host engine.

use pert_ui_core::{
    NormalizedValue, ParameterError, ParameterId, ParameterIdError, ParameterMapping, ParameterSpec,
};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn default_linear_mapping_supports_signed_domains() -> TestResult {
    let spec = ParameterSpec::new("Offset", -100.0, 100.0, 0.0)?;
    assert_eq!(spec.mapping(), ParameterMapping::Linear);
    assert!(spec.is_editable());
    assert_eq!(spec.default_value(), 0.0);
    for (value, position) in [
        (-100.0, 0.0),
        (-50.0, 0.25),
        (0.0, 0.5),
        (50.0, 0.75),
        (100.0, 1.0),
    ] {
        let position = NormalizedValue::try_from(position)?;
        assert_eq!(spec.normalize(&value)?, position);
        assert_eq!(spec.denormalize(position), value);
    }
    let display = spec.read_only();
    assert!(!display.is_editable());
    assert_eq!(display.denormalize(NormalizedValue::MAX), 100.0);
    Ok(())
}

#[test]
fn logarithmic_mapping_places_equal_ratios_at_equal_distances() -> TestResult {
    let spec = ParameterSpec::new("Ratio", 1.0, 10_000.0, 100.0)?
        .with_mapping(ParameterMapping::Logarithmic)?;
    for (value, position) in [
        (1.0, 0.0),
        (10.0, 0.25),
        (100.0, 0.5),
        (1_000.0, 0.75),
        (10_000.0, 1.0),
    ] {
        assert!((spec.normalize(&value)?.get() - position).abs() < 1e-6);
        let mapped = spec.denormalize(NormalizedValue::try_from(position)?);
        assert!((mapped - value).abs() <= value * 1e-6);
    }
    assert_eq!(
        spec.clone()
            .with_mapping(ParameterMapping::Linear)?
            .default_value(),
        100.0
    );
    Ok(())
}

#[test]
fn rejects_invalid_metadata_and_external_values_without_clamping() -> TestResult {
    for name in ["", " \t\n", "\u{2003}"] {
        assert_eq!(
            ParameterSpec::new(name, 0.0, 1.0, 0.5),
            Err(ParameterError::EmptyName)
        );
    }
    for (min, max) in [(1.0, 1.0), (2.0, 1.0), (-0.0, 0.0)] {
        assert_eq!(
            ParameterSpec::new("X", min, max, min),
            Err(ParameterError::InvalidRange)
        );
    }
    let spec = ParameterSpec::new("X", -1.0, 1.0, 0.0)?;
    for value in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        for (min, max, default) in [(value, 1.0, 0.0), (-1.0, value, 0.0), (-1.0, 1.0, value)] {
            assert_eq!(
                ParameterSpec::new("X", min, max, default),
                Err(ParameterError::NonFinite)
            );
        }
        assert_eq!(spec.normalize(&value), Err(ParameterError::NonFinite));
    }
    for value in [(-1.0_f32).next_down(), 1.0_f32.next_up()] {
        assert_eq!(spec.normalize(&value), Err(ParameterError::OutOfRange));
        assert_eq!(
            ParameterSpec::new("X", -1.0, 1.0, value),
            Err(ParameterError::OutOfRange)
        );
    }
    for minimum in [-1.0, 0.0, -0.0] {
        assert_eq!(
            ParameterSpec::new("X", minimum, 1.0, 1.0)?.with_mapping(ParameterMapping::Logarithmic),
            Err(ParameterError::NonPositiveLogDomain)
        );
    }
    Ok(())
}

#[test]
fn extreme_and_adjacent_domains_preserve_bounds_and_monotonicity() -> TestResult {
    for (min, max) in [
        (f32::MIN, f32::MAX),
        (-100.0, -1.0),
        (0.0, f32::from_bits(1)),
        (f32::from_bits(1), f32::MAX),
        (1.0, 1.0_f32.next_up()),
        (f32::MAX.next_down(), f32::MAX),
    ] {
        for mapping in [ParameterMapping::Linear, ParameterMapping::Logarithmic] {
            if mapping == ParameterMapping::Logarithmic && min <= 0.0 {
                continue;
            }
            let spec = ParameterSpec::new("X", min, max, min)?.with_mapping(mapping)?;
            assert_eq!(
                spec.denormalize(NormalizedValue::MIN).to_bits(),
                min.to_bits()
            );
            assert_eq!(
                spec.denormalize(NormalizedValue::MAX).to_bits(),
                max.to_bits()
            );
            assert_eq!(spec.normalize(&min)?, NormalizedValue::MIN);
            assert_eq!(spec.normalize(&max)?, NormalizedValue::MAX);
            let mut previous = min;
            let mut previous_position = NormalizedValue::MIN;
            for step in 0..=1_000 {
                let position = NormalizedValue::try_from(step as f32 / 1_000.0)?;
                let value = spec.denormalize(position);
                spec.validate(&value)?;
                assert!(value >= previous, "{mapping:?}: {min}..{max}");
                let mapped = spec.normalize(&value)?;
                assert!(mapped >= previous_position);
                previous = value;
                previous_position = mapped;
            }
        }
    }
    Ok(())
}

#[test]
fn representative_domains_round_trip_with_float_precision() -> TestResult {
    for (min, max, mapping) in [
        (-100.0, 100.0, ParameterMapping::Linear),
        (0.01, 10_000.0, ParameterMapping::Logarithmic),
    ] {
        let spec = ParameterSpec::new("X", min, max, min)?.with_mapping(mapping)?;
        for step in 0..=1_000 {
            let position = NormalizedValue::try_from(step as f32 / 1_000.0)?;
            let actual = spec.normalize(&spec.denormalize(position))?;
            assert!((actual.get() - position.get()).abs() < 1e-6);
        }
    }
    Ok(())
}

#[test]
fn identity_includes_session_object_key_and_incarnation() -> TestResult {
    use std::collections::HashMap;
    let id = ParameterId::new(1, 2, "amount")?;
    assert_eq!(id.clone(), ParameterId::new(1, 2, "amount")?);
    for other in [
        ParameterId::new(2, 2, "amount")?,
        ParameterId::new(1, 3, "amount")?,
        ParameterId::new(1, 2, "Amount")?,
        id.next_incarnation()?,
    ] {
        assert_ne!(id, other);
    }
    let replacement = id.next_incarnation()?;
    let parameters = HashMap::from([(replacement.clone(), "current")]);
    assert_eq!(parameters.get(&id), None);
    assert_eq!(parameters.get(&replacement), Some(&"current"));
    assert_eq!(
        (
            replacement.session(),
            replacement.object(),
            replacement.key(),
            replacement.incarnation()
        ),
        (1, 2, "amount", 1)
    );
    assert_eq!(
        ParameterId::new(1, 2, " \t"),
        Err(ParameterIdError::EmptyKey)
    );
    Ok(())
}
