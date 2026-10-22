//! Domain, mapping and identity contracts without a UI or host engine.

use pert_ui_core::{
    NormalizedValue, ParameterError, ParameterId, ParameterIdError, ParameterMapping,
    ParameterSpec, PowerExponent,
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
        for mapping in [
            ParameterMapping::Linear,
            ParameterMapping::Logarithmic,
            ParameterMapping::Power(PowerExponent::MIN),
            ParameterMapping::Power(PowerExponent::new(2.0)?),
            ParameterMapping::Power(PowerExponent::MAX),
        ] {
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

#[test]
fn power_mappings_round_trip_approximately_and_concentrate_resolution() -> TestResult {
    // Positions whose share of the range is below the bound's f32 precision
    // collapse onto it, so the tolerance grows with the exponent: about
    // (ulp(60) / 72)^(1 / exponent) for a high exponent here.
    for (exponent, tolerance) in [(0.5, 1e-4), (2.0, 1e-3), (3.0, 5e-3)] {
        // A domain that spans zero, which a logarithmic mapping cannot have.
        let spec = ParameterSpec::new("X", -60.0, 12.0, 0.0)?
            .with_mapping(ParameterMapping::Power(PowerExponent::new(exponent)?))?;
        for step in 0..=1_000 {
            let position = NormalizedValue::try_from(step as f32 / 1_000.0)?;
            let actual = spec.normalize(&spec.denormalize(position))?;
            assert!(
                (actual.get() - position.get()).abs() < tolerance,
                "{exponent}: {position:?} -> {actual:?}"
            );
        }
    }
    // An exponent above one gives the lower end more travel: halfway is a
    // quarter of the range.
    let fader = ParameterSpec::new("X", 0.0, 100.0, 0.0)?
        .with_mapping(ParameterMapping::Power(PowerExponent::new(2.0)?))?;
    assert!((fader.denormalize(NormalizedValue::try_from(0.5)?) - 25.0).abs() < 1e-4);
    // An exponent of one is linear in position, though a distinct mapping.
    let one = ParameterMapping::Power(PowerExponent::new(1.0)?);
    assert_ne!(one, ParameterMapping::Linear);
    let spec = ParameterSpec::new("X", 0.0, 10.0, 0.0)?.with_mapping(one)?;
    assert!((spec.normalize(&2.5)?.get() - 0.25).abs() < 1e-6);
    Ok(())
}

#[test]
fn power_exponents_are_validated() {
    for invalid in [0.0, -1.0, 0.09, 10.5, f32::NAN, f32::INFINITY] {
        assert_eq!(
            PowerExponent::new(invalid),
            Err(ParameterError::InvalidExponent),
            "{invalid}"
        );
        assert_eq!(
            PowerExponent::try_from(invalid),
            Err(ParameterError::InvalidExponent)
        );
    }
    for valid in [PowerExponent::MIN.get(), 1.0, PowerExponent::MAX.get()] {
        assert_eq!(PowerExponent::new(valid).map(PowerExponent::get), Ok(valid));
    }
}

#[test]
fn linear_and_logarithmic_positions_are_unchanged() -> TestResult {
    let linear = ParameterSpec::new("X", -100.0, 100.0, 0.0)?;
    assert_eq!(linear.normalize(&50.0)?.get(), 0.75);
    assert_eq!(linear.denormalize(NormalizedValue::try_from(0.25)?), -50.0);
    let log = ParameterSpec::new("X", 0.01, 10_000.0, 1.0)?
        .with_mapping(ParameterMapping::Logarithmic)?;
    assert!((log.normalize(&1.0)?.get() - 1.0 / 3.0).abs() < 1e-6);
    assert!((log.denormalize(NormalizedValue::try_from(2.0 / 3.0)?) - 100.0).abs() < 1e-2);
    Ok(())
}

#[test]
fn a_high_exponent_has_a_documented_plateau_at_the_lower_bound() -> TestResult {
    let spec = ParameterSpec::new("X", -60.0, 12.0, 0.0)?
        .with_mapping(ParameterMapping::Power(PowerExponent::MAX))?;
    let early = NormalizedValue::try_from(0.125)?;
    assert_eq!(spec.denormalize(early), -60.0, "f32 cannot hold the offset");
    assert_eq!(
        spec.normalize(&spec.denormalize(early))?,
        NormalizedValue::MIN
    );
    // Past the plateau the values increase again.
    assert!(spec.denormalize(NormalizedValue::try_from(0.5)?) > -60.0);
    Ok(())
}
