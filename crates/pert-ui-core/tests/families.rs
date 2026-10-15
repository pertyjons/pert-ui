//! Stepped, boolean and choice domains, their positions, and one shared edit
//! protocol exercised identically for all four families.
use std::num::NonZeroU64;

use pert_ui_core::*;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn step(value: u64) -> Result<NonZeroU64, Box<dyn std::error::Error>> {
    Ok(NonZeroU64::new(value).ok_or("zero step")?)
}
fn id(text: &str) -> Result<ChoiceId, ParameterError> {
    ChoiceId::new(text)
}
fn position(value: f32) -> Result<NormalizedValue, NormalizedValueError> {
    NormalizedValue::try_from(value)
}

#[test]
fn stepped_lattice_rejects_off_step_values_and_unaligned_maxima() -> TestResult {
    let spec = ParameterSpec::stepped("Octave", -3, 5, -1)?.with_step(step(2)?)?;
    for value in [-3, -1, 1, 3, 5] {
        spec.validate(&value)?;
    }
    for value in [0, 4, -2] {
        assert_eq!(spec.validate(&value), Err(ParameterError::OffLattice));
    }
    for value in [-5, 7, i64::MIN, i64::MAX] {
        assert_eq!(spec.validate(&value), Err(ParameterError::OutOfRange));
    }
    assert_eq!(spec.intervals(), 4);
    // An unaligned maximum is rejected instead of adding a short final step.
    assert_eq!(
        ParameterSpec::stepped("X", 0, 5, 0)?.with_step(step(2)?),
        Err(ParameterError::InvalidStep)
    );
    // A step larger than the span is valid only for a single value.
    assert_eq!(
        ParameterSpec::stepped("X", 0, 3, 0)?.with_step(step(10)?),
        Err(ParameterError::InvalidStep)
    );
    let single = ParameterSpec::stepped("X", 7, 7, 7)?.with_step(step(u64::MAX)?)?;
    assert_eq!(single.intervals(), 0);
    assert_eq!(single.normalize(&7)?, NormalizedValue::MIN);
    assert_eq!(single.denormalize(NormalizedValue::MAX), 7);
    // Off-lattice defaults are reported as such when the step changes.
    assert_eq!(
        ParameterSpec::stepped("X", 0, 10, 3)?.with_step(step(2)?),
        Err(ParameterError::OffLattice)
    );
    assert_eq!(
        ParameterSpec::stepped("X", 5, 4, 5),
        Err(ParameterError::InvalidRange)
    );
    assert_eq!(
        ParameterSpec::stepped("X", 0, 4, 9),
        Err(ParameterError::OutOfRange)
    );
    assert_eq!(
        ParameterSpec::stepped(" ", 0, 4, 0),
        Err(ParameterError::EmptyName)
    );
    Ok(())
}

#[test]
fn full_width_stepped_ranges_never_overflow() -> TestResult {
    let spec = ParameterSpec::stepped("Wide", i64::MIN, i64::MAX, 0)?;
    assert_eq!(spec.intervals(), u64::MAX);
    assert_eq!(spec.normalize(&i64::MIN)?, NormalizedValue::MIN);
    assert_eq!(spec.normalize(&i64::MAX)?, NormalizedValue::MAX);
    assert_eq!(spec.denormalize(NormalizedValue::MIN), i64::MIN);
    assert_eq!(spec.denormalize(NormalizedValue::MAX), i64::MAX);
    // 0.5 * (2^64 - 1) ties toward the larger rank: exactly zero.
    assert_eq!(spec.denormalize(position(0.5)?), 0);
    assert_eq!(spec.offset(i64::MAX, 1)?, i64::MAX);
    assert_eq!(spec.offset(i64::MIN, -1)?, i64::MIN);
    assert_eq!(spec.offset(0, i64::MIN)?, i64::MIN);
    assert_eq!(spec.offset(-1, i64::MAX)?, i64::MAX - 1);
    assert_eq!(spec.offset(3, 0)?, 3);
    let coarse = ParameterSpec::stepped("X", -3, 5, -1)?.with_step(step(2)?)?;
    assert_eq!(coarse.offset(0, 1), Err(ParameterError::OffLattice));
    Ok(())
}

#[test]
fn stepped_positions_round_ties_up_and_collide_only_in_the_interior() -> TestResult {
    let spec = ParameterSpec::stepped("Voices", 0, 3, 0)?;
    assert_eq!(spec.normalize(&1)?.get(), 1.0 / 3.0);
    assert_eq!(spec.normalize(&2)?.get(), 2.0 / 3.0);
    // Position 0.5 lies exactly between ranks 1 and 2; ties go to the larger.
    assert_eq!(spec.denormalize(position(0.5)?), 2);
    assert_eq!(spec.denormalize(position(1.0 / 6.0 - 0.001)?), 0);
    for value in 0..=3 {
        assert_eq!(spec.denormalize(spec.normalize(&value)?), value);
    }
    // A huge domain cannot place every integer at a distinct f32 position.
    let wide = ParameterSpec::stepped("Wide", 0, 1 << 40, 0)?;
    assert_eq!(
        wide.normalize(&((1 << 39) + 1))?,
        wide.normalize(&(1 << 39))?
    );
    Ok(())
}

#[test]
fn booleans_split_positions_at_one_half() -> TestResult {
    let spec = ParameterSpec::boolean("Bypass", true)?;
    assert!(spec.default_value());
    assert_eq!(spec.normalize(&false)?, NormalizedValue::MIN);
    assert_eq!(spec.normalize(&true)?, NormalizedValue::MAX);
    assert!(!spec.denormalize(position(0.5_f32.next_down())?));
    assert!(spec.denormalize(position(0.5)?));
    assert_eq!(spec.family(), ParameterFamily::Boolean);
    assert_eq!(
        ParameterSpec::boolean("", false),
        Err(ParameterError::EmptyName)
    );
    Ok(())
}

#[test]
fn choice_identities_are_validated_and_independent_of_labels_and_order() -> TestResult {
    for bad in ["", " lp", "lp ", "\tlp", "lp\n"] {
        assert_eq!(ChoiceId::new(bad), Err(ParameterError::MalformedChoiceId));
    }
    assert_eq!(id("low pass")?.as_str(), "low pass");
    assert_ne!(id("LP")?, id("lp")?);

    let options = [
        ChoiceOption::new(id("lp")?, "Low pass"),
        ChoiceOption::new(id("bp")?, ""),
        ChoiceOption::new(id("hp")?, "Low pass"),
    ];
    let spec = ParameterSpec::choices("Filter", options.clone(), id("bp")?)?;
    assert_eq!(spec.normalize(&id("bp")?)?, position(0.5)?);
    assert_eq!(spec.denormalize(NormalizedValue::MAX), id("hp")?);
    assert_eq!(spec.index_of(&id("hp")?), Some(2));
    assert_eq!(
        spec.option(&id("hp")?).map(ChoiceOption::label),
        Some("Low pass")
    );
    assert_eq!(
        spec.validate(&id("notch")?),
        Err(ParameterError::UnknownOption)
    );
    assert_eq!(
        spec.normalize(&id("notch")?),
        Err(ParameterError::UnknownOption)
    );

    // Reordering keeps identities valid; only ranks move.
    let mut reordered = options.clone();
    reordered.reverse();
    let reordered = ParameterSpec::choices("Filter", reordered, id("bp")?)?;
    reordered.validate(&id("lp")?)?;
    assert_eq!(reordered.normalize(&id("lp")?)?, NormalizedValue::MAX);

    let single = ParameterSpec::choices("Mode", [ChoiceOption::new(id("on")?, "On")], id("on")?)?;
    assert_eq!(single.normalize(&id("on")?)?, NormalizedValue::MIN);
    assert_eq!(single.denormalize(NormalizedValue::MAX), id("on")?);

    assert_eq!(
        ParameterSpec::choices("Filter", [], id("lp")?),
        Err(ParameterError::EmptyOptions)
    );
    assert_eq!(
        ParameterSpec::choices(
            "Filter",
            [
                ChoiceOption::new(id("lp")?, "A"),
                ChoiceOption::new(id("lp")?, "B")
            ],
            id("lp")?
        ),
        Err(ParameterError::DuplicateOption)
    );
    assert_eq!(
        ParameterSpec::choices("Filter", options, id("notch")?),
        Err(ParameterError::UnknownOption)
    );
    Ok(())
}

#[test]
fn erased_forms_preserve_the_family() -> TestResult {
    let spec = AnySpec::from(ParameterSpec::stepped("Count", 0, 9, 3)?);
    assert_eq!(spec.family(), ParameterFamily::Stepped);
    assert_eq!(spec.name(), "Count");
    assert_eq!(spec.default_value(), AnyValue::Stepped(3));
    assert!(spec.is_editable());
    let spec = match ParameterSpec::<bool>::try_from(spec) {
        Ok(_) => return Err("wrong family accepted".into()),
        Err(spec) => spec,
    };
    assert!(ParameterSpec::<i64>::try_from(spec).is_ok());

    let value = AnyValue::from(id("hp")?);
    assert_eq!(value.family(), ParameterFamily::Choice);
    assert_eq!(f32::try_from(value.clone()), Err(value.clone()));
    assert_eq!(
        ChoiceId::try_from(value).map_err(|_| "family mismatch")?,
        id("hp")?
    );

    let spec = ParameterSpec::boolean("On", false)?;
    let state = ParameterState::new(ParameterId::new(1, 1, "on")?, spec.clone());
    let snapshot = AnyParameterSnapshot::new(spec, state.snapshot());
    assert_eq!(snapshot.family(), ParameterFamily::Boolean);
    assert_eq!(snapshot.base(), AnyValue::Boolean(false));
    assert_eq!(snapshot.revision(), Revision::INITIAL);
    assert!(snapshot.available());
    assert_eq!(snapshot.name(), "On");
    Ok(())
}

/// One protocol, one harness: every family must behave identically.
fn shared_protocol<T: ParameterValue>(
    spec: ParameterSpec<T>,
    first: T,
    second: T,
    invalid: Option<T>,
) -> TestResult {
    let target = ParameterId::new(9, 9, "value")?;
    let mut state = ParameterState::new(target.clone(), spec.clone());
    let base = spec.default_value();
    assert_eq!(state.snapshot().base, base);

    // Concurrent begin and stale revision.
    let token = state.begin(Revision::INITIAL, EditSource::new(3))?;
    assert_eq!(
        state.begin(Revision::INITIAL, EditSource::new(4)),
        Err(EditError::Busy)
    );
    assert_eq!(state.snapshot().edit_source.map(EditSource::get), Some(3));

    // Previews: accepted, stale, invalid without termination.
    assert_eq!(
        state.update(&token, EditSequence::new(2), first.clone())?,
        PreviewOutcome::Applied
    );
    assert_eq!(state.preview_sequence(&token)?, Some(EditSequence::new(2)));
    assert_eq!(
        state.update(&token, EditSequence::new(1), second.clone())?,
        PreviewOutcome::Stale
    );
    if let Some(invalid) = invalid.clone() {
        assert!(matches!(
            state.update(&token, EditSequence::new(3), invalid),
            Err(EditError::InvalidValue(_))
        ));
    }
    assert_eq!(state.snapshot().preview, Some(first.clone()));
    assert_eq!(state.snapshot().base, base);

    // Commit once, replay afterwards.
    let committed = state.commit(&token, EditSequence::new(4), second.clone())?;
    assert_eq!(
        committed,
        EditOutcome::Committed {
            value: second.clone(),
            revision: Revision::INITIAL.max(state.snapshot().revision)
        }
    );
    assert_eq!(state.snapshot().revision.get(), 1);
    assert_eq!(state.cancel(&token)?, committed);
    assert_eq!(state.reject_delivery(&token)?, committed);
    state.acknowledge(&token)?;

    // A rejected commit leaves the base alone.
    if let Some(invalid) = invalid {
        let token = state.begin(state.snapshot().revision, EditSource::new(1))?;
        assert!(matches!(
            state.commit(&token, EditSequence::new(1), invalid)?,
            EditOutcome::Rejected(EditError::InvalidValue(_))
        ));
        assert_eq!(state.snapshot().base, second);
        state.acknowledge(&token)?;
    }

    // Delivery failure: preview cleared, base and revision kept, overlay kept.
    let revision = state.snapshot().revision;
    state.set_effective(Some(first.clone()))?;
    let token = state.begin(revision, EditSource::new(1))?;
    state.update(&token, EditSequence::new(1), first.clone())?;
    assert_eq!(
        state.reject_delivery(&token)?,
        EditOutcome::Rejected(EditError::DeliveryFailed)
    );
    let snapshot = state.snapshot();
    assert_eq!(
        (
            snapshot.preview,
            snapshot.revision,
            snapshot.effective.clone()
        ),
        (None, revision, first.clone())
    );
    assert_eq!(
        state.cancel(&token)?,
        EditOutcome::Rejected(EditError::DeliveryFailed)
    );
    state.acknowledge(&token)?;
    state.set_effective(None)?;

    // External write wins over an active edit.
    let token = state.begin(revision, EditSource::new(1))?;
    state.write_base(first.clone())?;
    assert_eq!(
        state.outcome(&token)?,
        Some(EditOutcome::Rejected(EditError::Conflict))
    );
    state.acknowledge(&token)?;

    // Receipt exhaustion and acknowledgement.
    let mut tokens = Vec::new();
    for _ in 0..ParameterState::<T>::MAX_TERMINAL_RECEIPTS {
        let token = state.begin(state.snapshot().revision, EditSource::new(1))?;
        state.cancel(&token)?;
        tokens.push(token);
    }
    assert_eq!(
        state.begin(state.snapshot().revision, EditSource::new(1)),
        Err(EditError::Capacity)
    );
    for token in &tokens {
        state.acknowledge(token)?;
    }

    // Removal rejects the active edit as unavailable; recreation needs a new incarnation.
    let token = state.begin(state.snapshot().revision, EditSource::new(1))?;
    state.remove();
    assert_eq!(
        state.outcome(&token)?,
        Some(EditOutcome::Rejected(EditError::Unavailable))
    );
    assert_eq!(state.write_base(second), Err(EditError::Unavailable));
    let recreated = ParameterState::new(target.next_incarnation()?, spec);
    assert_eq!(recreated.outcome(&token), Err(EditError::UnknownEdit));
    Ok(())
}

#[test]
fn every_family_follows_the_same_edit_protocol() -> TestResult {
    shared_protocol(
        ParameterSpec::new("Amount", 0.0, 1.0, 0.5)?,
        0.25,
        0.75,
        Some(2.0),
    )?;
    shared_protocol(
        ParameterSpec::stepped("Count", 0, 10, 4)?.with_step(step(2)?)?,
        2,
        8,
        Some(3),
    )?;
    shared_protocol(ParameterSpec::boolean("On", false)?, true, false, None)?;
    let options = [
        ChoiceOption::new(id("a")?, "A"),
        ChoiceOption::new(id("b")?, "B"),
        ChoiceOption::new(id("c")?, "C"),
    ];
    shared_protocol(
        ParameterSpec::choices("Mode", options, id("a")?)?,
        id("b")?,
        id("c")?,
        Some(id("z")?),
    )
}

#[test]
fn replacement_validates_base_and_overlay_atomically() -> TestResult {
    let options = |ids: &[&str]| -> Result<Vec<ChoiceOption>, ParameterError> {
        ids.iter()
            .map(|text| Ok(ChoiceOption::new(id(text)?, text.to_uppercase())))
            .collect()
    };
    let mut state = ParameterState::new(
        ParameterId::new(1, 1, "mode")?,
        ParameterSpec::choices("Mode", options(&["a", "b", "c"])?, id("a")?)?,
    );
    state.write_base(id("b")?)?;
    state.set_effective(Some(id("c")?))?;
    let revision = state.snapshot().revision;

    // Removing the selected option or the overlay's option is rejected atomically.
    let token = state.begin(revision, EditSource::new(1))?;
    assert_eq!(
        state.replace_spec(ParameterSpec::choices(
            "Mode",
            options(&["a", "c"])?,
            id("a")?
        )?),
        Err(EditError::InvalidValue(ParameterError::UnknownOption))
    );
    assert_eq!(
        state.replace_spec(ParameterSpec::choices(
            "Mode",
            options(&["a", "b"])?,
            id("a")?
        )?),
        Err(EditError::InvalidValue(ParameterError::UnknownOption))
    );
    assert_eq!(state.outcome(&token)?, None);
    assert_eq!(state.snapshot().revision, revision);

    // A reorder keeps the selection; it still ends the active edit.
    state.replace_spec(ParameterSpec::choices(
        "Mode",
        options(&["c", "b", "a", "d"])?,
        id("a")?,
    )?)?;
    assert_eq!(
        state.outcome(&token)?,
        Some(EditOutcome::Rejected(EditError::Conflict))
    );
    assert_eq!(state.snapshot().base, id("b")?);
    assert_eq!(state.snapshot().effective, id("c")?);
    assert_eq!(state.spec().index_of(&id("b")?), Some(1));

    // Staged migration: move base and overlay first, then remove the option.
    state.write_base(id("d")?)?;
    state.set_effective(None)?;
    state.replace_spec(ParameterSpec::choices(
        "Mode",
        options(&["a", "d"])?,
        id("a")?,
    )?)?;
    assert_eq!(
        state.set_effective(Some(id("b")?)),
        Err(EditError::InvalidValue(ParameterError::UnknownOption))
    );
    Ok(())
}

#[test]
fn stepped_overlays_may_leave_the_editable_lattice() -> TestResult {
    let mut state = ParameterState::new(
        ParameterId::new(1, 1, "count")?,
        ParameterSpec::stepped("Count", 0, 10, 0)?.with_step(step(5)?)?,
    );
    state.set_effective(Some(7))?;
    state.set_effective(Some(-40))?;
    assert_eq!(state.snapshot().effective, -40);
    // Replacing the spec keeps an off-domain numeric overlay.
    state.replace_spec(ParameterSpec::stepped("Count", 0, 20, 0)?.with_step(step(10)?)?)?;
    assert_eq!(state.snapshot().effective, -40);
    assert_eq!(
        state.write_base(5),
        Err(EditError::InvalidValue(ParameterError::OffLattice))
    );
    let mut continuous = ParameterState::new(
        ParameterId::new(1, 1, "gain")?,
        ParameterSpec::new("Gain", 0.0, 1.0, 0.0)?,
    );
    assert_eq!(
        continuous.set_effective(Some(f32::NAN)),
        Err(EditError::InvalidValue(ParameterError::NonFinite))
    );
    continuous.set_effective(Some(3.0))?;
    Ok(())
}

#[test]
fn host_values_round_trip_through_an_adapter_wrapper() -> TestResult {
    #[derive(Debug, PartialEq)]
    struct Semitones(i32);
    #[derive(Debug, thiserror::Error)]
    #[error("out of range")]
    struct Range;
    impl HostValue for Semitones {
        type Canonical = i64;
        type Error = Range;
        fn to_canonical(&self) -> Result<i64, Range> {
            Ok(i64::from(self.0))
        }
        fn from_canonical(value: i64) -> Result<Self, Range> {
            i32::try_from(value).map(Self).map_err(|_| Range)
        }
    }
    let spec = ParameterSpec::stepped("Transpose", -24, 24, 0)?;
    let canonical = Semitones(-7).to_canonical()?;
    spec.validate(&canonical)?;
    assert_eq!(Semitones::from_canonical(canonical)?, Semitones(-7));
    assert!(Semitones::from_canonical(i64::MAX).is_err());
    Ok(())
}
