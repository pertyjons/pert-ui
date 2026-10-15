//! Edit protocol contracts, including capacity and out-of-order message delivery.
use pert_ui_core::*;

type TestResult = Result<(), Box<dyn std::error::Error>>;
fn state() -> Result<ParameterState<f32>, Box<dyn std::error::Error>> {
    Ok(ParameterState::new(
        ParameterId::new(1, 2, "amount")?,
        ParameterSpec::new("Amount", 0.0, 10.0, 2.0)?,
    ))
}
fn begin(state: &mut ParameterState<f32>) -> Result<EditToken, EditError> {
    state.begin(state.snapshot().revision, EditSource::new(1))
}

#[test]
fn preview_effective_and_base_are_independent_and_commit_is_once() -> TestResult {
    let mut state = state()?;
    let token = begin(&mut state)?;
    assert_eq!(state.acknowledge(&token), Err(EditError::Pending));
    state.update(&token, EditSequence::new(1), 4.0)?;
    state.set_effective(Some(15.0))?;
    let snapshot = state.snapshot();
    assert_eq!(
        (
            snapshot.base,
            snapshot.preview,
            snapshot.effective,
            snapshot.revision
        ),
        (2.0, Some(4.0), 15.0, Revision::INITIAL)
    );
    let committed = state.commit(&token, EditSequence::new(2), 5.0)?;
    assert_eq!(
        committed,
        EditOutcome::Committed {
            value: 5.0,
            revision: state.snapshot().revision
        }
    );
    assert_eq!(state.snapshot().revision.get(), 1);
    assert_eq!(state.commit(&token, EditSequence::new(3), 9.0)?, committed);
    assert_eq!(state.cancel(&token)?, committed);
    assert_eq!(state.snapshot().base, 5.0);
    assert_eq!(state.snapshot().revision.get(), 1);
    assert_eq!(
        state.update(&token, EditSequence::new(4), 6.0),
        Err(EditError::Finished)
    );
    state.acknowledge(&token)?;
    assert_eq!(
        state.commit(&token, EditSequence::new(5), 7.0),
        Err(EditError::UnknownEdit)
    );
    state.set_effective(None)?;
    assert_eq!(state.snapshot().effective, 5.0);
    Ok(())
}

#[test]
fn stale_preview_invalid_preview_and_rejected_commit_have_distinct_outcomes() -> TestResult {
    let mut state = state()?;
    let token = begin(&mut state)?;
    state.update(&token, EditSequence::new(5), 6.0)?;
    assert_eq!(
        state.update(&token, EditSequence::new(4), f32::NAN)?,
        PreviewOutcome::Stale
    );
    assert_eq!(
        state.update(&token, EditSequence::new(6), 11.0),
        Err(EditError::InvalidValue(ParameterError::OutOfRange))
    );
    assert_eq!(state.snapshot().preview, Some(6.0));
    assert_eq!(
        state.commit(&token, EditSequence::new(5), 4.0)?,
        EditOutcome::Rejected(EditError::StaleSequence)
    );
    assert_eq!(
        (state.snapshot().base, state.snapshot().preview),
        (2.0, None)
    );
    let token = begin(&mut state)?;
    assert_eq!(
        state.commit(&token, EditSequence::new(1), f32::NAN)?,
        EditOutcome::Rejected(EditError::InvalidValue(ParameterError::NonFinite))
    );
    assert_eq!(state.snapshot().revision, Revision::INITIAL);
    Ok(())
}

#[test]
fn admission_rejects_busy_stale_read_only_without_side_effects() -> TestResult {
    let mut state = state()?;
    let token = begin(&mut state)?;
    assert_eq!(begin(&mut state), Err(EditError::Busy));
    state.cancel(&token)?;
    state.write_base(3.0)?;
    assert_eq!(
        state.begin(Revision::INITIAL, EditSource::new(1)),
        Err(EditError::StaleRevision)
    );
    state.replace_spec(state.spec().clone().read_only())?;
    assert_eq!(begin(&mut state), Err(EditError::ReadOnly));
    assert_eq!(state.snapshot().base, 3.0);
    Ok(())
}

#[test]
fn cancel_and_external_conflicts_never_restore_an_old_baseline() -> TestResult {
    let mut state = state()?;
    let first = begin(&mut state)?;
    state.update(&first, EditSequence::new(1), 4.0)?;
    assert_eq!(state.cancel(&first)?, EditOutcome::Cancelled);
    assert_eq!(
        (state.snapshot().base, state.snapshot().preview),
        (2.0, None)
    );
    let second = begin(&mut state)?;
    state.update(&second, EditSequence::new(1), 8.0)?;
    state.write_base(7.0)?;
    assert_eq!(
        state.cancel(&second)?,
        EditOutcome::Rejected(EditError::Conflict)
    );
    assert_eq!(
        state.commit(&second, EditSequence::new(2), 8.0)?,
        EditOutcome::Rejected(EditError::Conflict)
    );
    assert_eq!(
        (state.snapshot().base, state.snapshot().preview),
        (7.0, None)
    );
    Ok(())
}

#[test]
fn debug_output_names_the_target_and_the_edit_bookkeeping() -> TestResult {
    let mut state = state()?;
    let _token = begin(&mut state)?;
    let text = format!("{state:?}");
    for part in [
        "ParameterState",
        "\"amount\"",
        "active_serial: Some(1)",
        "receipts: 1",
        "available: true",
    ] {
        assert!(text.contains(part), "{part} in {text}");
    }
    Ok(())
}

#[test]
fn terminal_capacity_is_reserved_at_begin_and_reclaimed_only_by_ack() -> TestResult {
    let mut state = state()?;
    // The documented limit is part of the contract.
    assert_eq!(ParameterState::<f32>::MAX_TERMINAL_RECEIPTS, 64);
    let mut tokens = Vec::new();
    for _ in 0..ParameterState::<f32>::MAX_TERMINAL_RECEIPTS {
        let token = begin(&mut state)?;
        for sequence in 1..=100 {
            state.update(&token, EditSequence::new(sequence), 3.0)?;
        }
        state.cancel(&token)?;
        tokens.push(token);
    }
    assert_eq!(begin(&mut state), Err(EditError::Capacity));
    for token in &tokens {
        assert_eq!(state.outcome(token)?, Some(EditOutcome::Cancelled));
    }
    state.acknowledge(&tokens[0])?;
    let admitted = begin(&mut state)?;
    assert!(matches!(
        state.commit(&admitted, EditSequence::new(1), 4.0)?,
        EditOutcome::Committed { .. }
    ));
    Ok(())
}

#[test]
fn removal_shutdown_and_recreation_reject_delayed_tokens() -> TestResult {
    let mut original = state()?;
    let token = begin(&mut original)?;
    original.update(&token, EditSequence::new(1), 4.0)?;
    original.remove();
    assert!(!original.snapshot().available);
    assert_eq!(
        original.outcome(&token)?,
        Some(EditOutcome::Rejected(EditError::Unavailable))
    );
    assert_eq!(begin(&mut original), Err(EditError::Unavailable));
    let mut replacement =
        ParameterState::new(original.id().next_incarnation()?, original.spec().clone());
    let current = begin(&mut replacement)?;
    assert_eq!(
        replacement.update(&token, EditSequence::new(2), 8.0),
        Err(EditError::UnknownEdit)
    );
    assert_eq!(replacement.cancel(&token), Err(EditError::UnknownEdit));
    assert_eq!(replacement.outcome(&current)?, None);
    assert_eq!(replacement.snapshot().base, 2.0);
    Ok(())
}

#[test]
fn failed_host_changes_are_atomic_and_valid_metadata_changes_cancel() -> TestResult {
    let mut state = state()?;
    let token = begin(&mut state)?;
    state.update(&token, EditSequence::new(1), 4.0)?;
    let before = state.snapshot();
    assert!(state.write_base(11.0).is_err());
    assert!(state.set_effective(Some(f32::INFINITY)).is_err());
    assert!(
        state
            .replace_spec(ParameterSpec::new("Other", 5.0, 10.0, 5.0)?)
            .is_err()
    );
    assert_eq!(state.snapshot(), before);
    state.replace_spec(ParameterSpec::new("Renamed", 0.0, 20.0, 0.0)?)?;
    assert_eq!(
        state.outcome(&token)?,
        Some(EditOutcome::Rejected(EditError::Conflict))
    );
    assert_eq!(state.spec().name(), "Renamed");
    Ok(())
}
