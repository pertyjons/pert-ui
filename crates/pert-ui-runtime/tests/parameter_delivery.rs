//! Host notification contracts: ordering, registration, reentrancy, failure
//! retention, recovery, deferred lifecycle delivery and shutdown, for typed,
//! erased and remote bindings.
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::Duration,
};

use pert_ui_runtime::{
    AnyParameter, AnyParameterHost, AnySpec, AnyTerminalEvent, AnyValue, AuthorityStatus,
    BindingError, ChoiceId, ChoiceOption, Component, DeliveryError, DeliveryResolution,
    DeliveryStatus, EditError, EditOutcome, EditSequence, Parameter, ParameterFamily, ParameterId,
    ParameterSpec, PreviewEvent, PreviewOutcome, RemoteCommit, RemoteHost, Revision, Runtime,
    RuntimeError, SinkError, SinkErrorKind, SubmissionDelivery, TerminalEvent, UpdateContext,
    WidgetId,
};

type TestResult = Result<(), Box<dyn std::error::Error>>;

struct Idle;
impl Component for Idle {
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
}

fn widget(ui: &mut Runtime, key: &str) -> Result<WidgetId, RuntimeError> {
    ui.mount(key, Idle)
}

type Log<T> = Rc<RefCell<Vec<T>>>;

#[test]
fn events_follow_publication_in_order_with_correlation_data() -> TestResult {
    let mut ui = Runtime::new();
    let (amount, host) = ui.parameter(
        ParameterId::new(1, 1, "amount")?,
        ParameterSpec::new("Amount", 0.0, 1.0, 0.5)?,
    )?;
    let terminals: Log<TerminalEvent<f32>> = Rc::default();
    let previews: Log<PreviewEvent<f32>> = Rc::default();
    let observed_bases: Log<f32> = Rc::default();
    let reader = amount.clone();
    let (log, bases) = (terminals.clone(), observed_bases.clone());
    let monitor = host.on_outcome(move |event| {
        // Reads are permitted and already see the published state.
        bases.borrow_mut().push(
            reader
                .snapshot()
                .get()
                .map_err(|e| SinkError::new(SinkErrorKind::Other, e.to_string()))?
                .base,
        );
        log.borrow_mut().push(event.clone());
        Ok(())
    })?;
    let log = previews.clone();
    host.on_preview(move |event| {
        log.borrow_mut().push(event.clone());
        Ok(())
    })?;
    assert_eq!(amount.delivery_status().get()?, DeliveryStatus::Ready);

    // A write without an active edit produces no event; there is no backfill.
    host.write_base(0.25)?;
    let revision = amount.snapshot().get()?.revision;
    let id = widget(&mut ui, "control")?;
    let edit = ui.event(&id, |cx| amount.begin_edit(cx, revision))??;
    assert_eq!(edit.update(0.4)?, PreviewOutcome::Applied);
    assert_eq!(edit.update(0.4)?, PreviewOutcome::Applied);
    assert!(edit.update(2.0).is_err()); // invalid: no event, no termination
    let outcome = edit.commit(0.6)?;
    assert_eq!(edit.commit(0.9)?, outcome); // replay: no second dispatch
    assert_eq!(edit.cancel()?, outcome);

    let previews = previews.borrow();
    assert_eq!(
        previews
            .iter()
            .map(|event| (event.sequence.get(), event.value))
            .collect::<Vec<_>>(),
        [(1, 0.4), (2, 0.4)]
    );
    assert!(previews.iter().all(|event| event.begin_value == 0.25
        && event.begin_revision == revision
        && &event.token == edit.token()));
    let terminals = terminals.borrow();
    assert_eq!(terminals.len(), 1);
    let event = &terminals[0];
    assert_eq!(&event.token, edit.token());
    assert_eq!(event.begin_value, 0.25);
    assert_eq!(event.begin_revision, revision);
    assert_eq!(event.last_preview_sequence, Some(EditSequence::new(2)));
    // The invalid preview consumed no sequence, so the commit used 3.
    assert_eq!(event.commit_sequence, Some(EditSequence::new(3)));
    assert_eq!(event.outcome, outcome);
    assert_eq!(event.state, amount.snapshot().get()?);
    assert_eq!(observed_bases.borrow().as_slice(), [0.6]);
    assert!(!monitor.snapshot().is_faulted());
    Ok(())
}

#[test]
fn registration_is_explicit_and_never_replaces_an_observer() -> TestResult {
    let mut ui = Runtime::new();
    let (flag, host) = ui.parameter(
        ParameterId::new(1, 1, "flag")?,
        ParameterSpec::boolean("Flag", false)?,
    )?;
    assert_eq!(
        host.on_preview(|_| Ok(())),
        Err(RuntimeError::Binding(BindingError::Delivery(
            DeliveryError::NoOutcomeSink
        )))
    );
    let monitor = host.on_outcome(|_| Ok(()))?;
    assert!(matches!(
        host.on_outcome(|_| Ok(())),
        Err(RuntimeError::Binding(BindingError::Delivery(
            DeliveryError::AlreadyRegistered
        )))
    ));
    host.on_preview(|_| Ok(()))?;
    assert_eq!(
        host.on_preview(|_| Ok(())),
        Err(RuntimeError::Binding(BindingError::Delivery(
            DeliveryError::AlreadyRegistered
        )))
    );
    let id = widget(&mut ui, "control")?;
    let edit = ui.event(&id, |cx| flag.begin_edit(cx, Revision::INITIAL))??;
    assert_eq!(
        host.clear_sinks(),
        Err(RuntimeError::Binding(BindingError::Delivery(
            DeliveryError::ActiveEdit
        )))
    );
    edit.commit(true)?;
    host.clear_sinks()?;
    assert!(!monitor.snapshot().registered);
    assert_eq!(flag.delivery_status().get()?, DeliveryStatus::Unobserved);
    // Dropping a monitor never detaches a sink.
    let counter = Rc::new(RefCell::new(0));
    let seen = counter.clone();
    drop(host.on_outcome(move |_| {
        *seen.borrow_mut() += 1;
        Ok(())
    })?);
    let edit = ui.event(&id, |cx| {
        flag.begin_edit(cx, flag.snapshot().get()?.revision)
    })??;
    edit.commit(false)?;
    assert_eq!(*counter.borrow(), 1);
    Ok(())
}

#[test]
fn sinks_cannot_write_during_delivery() -> TestResult {
    let mut ui = Runtime::new();
    let (count, host) = ui.parameter(
        ParameterId::new(1, 1, "count")?,
        ParameterSpec::stepped("Count", 0, 10, 0)?,
    )?;
    let signal = ui.signal(0);
    let errors: Log<RuntimeError> = Rc::default();
    let (log, writer) = (errors.clone(), signal.clone());
    let _monitor = host.on_outcome(move |_| {
        if let Err(error) = writer.set(1) {
            log.borrow_mut().push(error);
        }
        Ok(())
    })?;
    let id = widget(&mut ui, "control")?;
    let edit = ui.event(&id, |cx| count.begin_edit(cx, Revision::INITIAL))??;
    edit.commit(4)?;
    assert_eq!(
        errors.borrow().as_slice(),
        [RuntimeError::WriteDuringDelivery]
    );
    assert_eq!(signal.get()?, 0);
    // Writes work again once the callback returned.
    signal.set(2)?;
    assert_eq!(count.snapshot().get()?.base, 4);
    Ok(())
}

#[test]
fn terminal_failure_is_retained_blocks_edits_and_recovers_explicitly() -> TestResult {
    let mut ui = Runtime::new();
    let (amount, host) = ui.parameter(
        ParameterId::new(1, 1, "amount")?,
        ParameterSpec::new("Amount", 0.0, 1.0, 0.0)?,
    )?;
    let failures = Rc::new(RefCell::new(2));
    let calls: Log<EditOutcome<f32>> = Rc::default();
    let (remaining, log) = (failures.clone(), calls.clone());
    let monitor = host.on_outcome(move |event| {
        log.borrow_mut().push(event.outcome.clone());
        if *remaining.borrow() > 0 {
            *remaining.borrow_mut() -= 1;
            return Err(SinkError::new(SinkErrorKind::Full, "queue full"));
        }
        Ok(())
    })?;
    let id = widget(&mut ui, "control")?;
    let edit = ui.event(&id, |cx| amount.begin_edit(cx, Revision::INITIAL))??;
    // The local commit still succeeds; the delivery failure is reported apart.
    let outcome = edit.commit(0.5)?;
    assert!(matches!(outcome, EditOutcome::Committed { value, .. } if value == 0.5));
    drop(edit);
    let report = monitor.snapshot();
    assert!(report.is_faulted());
    assert_eq!(
        report.last_error.as_ref().map(SinkError::kind),
        Some(SinkErrorKind::Full)
    );
    let token = report
        .terminal
        .as_ref()
        .map(|event| event.token.clone())
        .ok_or("terminal event retained")?;
    assert_eq!(amount.delivery_status().get()?, DeliveryStatus::Faulted);
    assert!(matches!(
        ui.event(&id, |cx| amount
            .begin_edit(cx, amount.snapshot().get()?.revision))?,
        Err(RuntimeError::Binding(BindingError::Delivery(
            DeliveryError::Faulted
        )))
    ));
    // Authoritative writes stay available for reconciliation and cannot rewrite
    // the retained event.
    host.write_base(0.75)?;
    assert_eq!(
        monitor.snapshot().terminal.map(|event| event.outcome),
        Some(outcome)
    );
    assert!(matches!(
        host.clear_sinks(),
        Err(RuntimeError::Binding(BindingError::Delivery(
            DeliveryError::Faulted
        )))
    ));
    // A wrong token changes nothing.
    let other = ParameterId::new(1, 2, "other")?;
    let (other, _other_host) = ui.parameter(other, ParameterSpec::new("O", 0.0, 1.0, 0.0)?)?;
    let foreign = ui.event(&id, |cx| other.begin_edit(cx, Revision::INITIAL))??;
    assert_eq!(
        host.retry_outcome(foreign.token()),
        Err(RuntimeError::Binding(BindingError::Delivery(
            DeliveryError::UnknownEdit
        )))
    );
    // First retry fails again and replaces only the last error.
    assert_eq!(
        host.retry_outcome(&token),
        Err(RuntimeError::Binding(BindingError::Delivery(
            DeliveryError::SinkFailed
        )))
    );
    assert!(monitor.snapshot().is_faulted());
    host.retry_outcome(&token)?;
    let report = monitor.snapshot();
    assert!(!report.is_faulted());
    assert!(report.terminal.is_none());
    assert_eq!(amount.delivery_status().get()?, DeliveryStatus::Ready);
    assert_eq!(calls.borrow().len(), 3);
    assert_eq!(
        host.retry_outcome(&token),
        Err(RuntimeError::Binding(BindingError::Delivery(
            DeliveryError::NotFaulted
        )))
    );
    Ok(())
}

#[test]
fn preview_failure_ends_the_edit_and_attempts_its_terminal_once() -> TestResult {
    let mut ui = Runtime::new();
    let (amount, host) = ui.parameter(
        ParameterId::new(1, 1, "amount")?,
        ParameterSpec::new("Amount", 0.0, 1.0, 0.0)?,
    )?;
    let terminals: Log<EditOutcome<f32>> = Rc::default();
    let log = terminals.clone();
    let monitor = host.on_outcome(move |event| {
        log.borrow_mut().push(event.outcome.clone());
        Ok(())
    })?;
    host.on_preview(|_| Err(SinkError::new(SinkErrorKind::Closed, "audition gone")))?;
    host.set_effective(Some(0.9))?;
    let id = widget(&mut ui, "control")?;
    let edit = ui.event(&id, |cx| amount.begin_edit(cx, Revision::INITIAL))??;
    // Accepted locally, so `Applied`, but the session has ended.
    assert_eq!(edit.update(0.3)?, PreviewOutcome::Applied);
    let rejected = EditOutcome::Rejected(EditError::DeliveryFailed);
    assert_eq!(edit.outcome()?, Some(rejected.clone()));
    assert_eq!(edit.commit(0.4)?, rejected.clone());
    assert_eq!(terminals.borrow().as_slice(), [rejected]);
    let snapshot = amount.snapshot().get()?;
    assert_eq!(
        (
            snapshot.base,
            snapshot.preview,
            snapshot.effective,
            snapshot.revision
        ),
        (0.0, None, 0.9, Revision::INITIAL)
    );
    let report = monitor.snapshot();
    assert!(report.is_faulted());
    assert!(report.terminal_delivered);
    let failed = report.failed_preview.ok_or("failed preview retained")?;
    assert_eq!(failed.value, 0.3);
    assert_eq!(
        host.retry_outcome(&failed.token),
        Err(RuntimeError::Binding(BindingError::Delivery(
            DeliveryError::AlreadyDelivered
        )))
    );
    drop(edit);
    host.resolve_delivery(&failed.token, DeliveryResolution::Reconciled)?;
    assert!(!monitor.snapshot().is_faulted());
    assert_eq!(
        host.resolve_delivery(&failed.token, DeliveryResolution::Reconciled),
        Err(RuntimeError::Binding(BindingError::Delivery(
            DeliveryError::NotFaulted
        )))
    );
    Ok(())
}

#[test]
fn lifecycle_cancellation_is_notified_at_the_next_safe_point() -> TestResult {
    let mut ui = Runtime::new();
    let (amount, host) = ui.parameter(
        ParameterId::new(1, 1, "amount")?,
        ParameterSpec::new("Amount", 0.0, 1.0, 0.0)?,
    )?;
    let terminals: Log<EditOutcome<f32>> = Rc::default();
    let log = terminals.clone();
    let _monitor = host.on_outcome(move |event| {
        log.borrow_mut().push(event.outcome.clone());
        Ok(())
    })?;
    ui.flush(Duration::ZERO)?;
    assert_eq!(ui.next_deadline(), None);
    let id = widget(&mut ui, "control")?;
    let edit = ui.event(&id, |cx| amount.begin_edit(cx, Revision::INITIAL))??;
    edit.update(0.5)?;
    ui.unmount(&id)?;
    assert_eq!(edit.outcome()?, Some(EditOutcome::Cancelled));
    // Not called from inside the unmount; due immediately instead.
    assert!(terminals.borrow().is_empty());
    assert_eq!(ui.next_deadline(), Some(Duration::ZERO));
    ui.flush(Duration::ZERO)?;
    assert_eq!(terminals.borrow().as_slice(), [EditOutcome::Cancelled]);
    // Dropping the retained guard later neither cancels nor notifies again.
    drop(edit);
    ui.flush(Duration::ZERO)?;
    assert_eq!(terminals.borrow().len(), 1);

    // A dropped unfinished session is delivered before the next operation.
    let id = widget(&mut ui, "again")?;
    let edit = ui.event(&id, |cx| amount.begin_edit(cx, Revision::INITIAL))??;
    drop(edit);
    host.write_base(0.25)?;
    assert_eq!(
        terminals.borrow().as_slice(),
        [EditOutcome::Cancelled, EditOutcome::Cancelled]
    );
    Ok(())
}

#[test]
fn removal_keeps_sinks_for_recovery_and_retirement_requires_it() -> TestResult {
    let mut ui = Runtime::new();
    let (amount, host) = ui.parameter(
        ParameterId::new(1, 1, "amount")?,
        ParameterSpec::new("Amount", 0.0, 1.0, 0.0)?,
    )?;
    let monitor =
        host.on_outcome(|_| Err(SinkError::new(SinkErrorKind::Other, "engine offline")))?;
    let id = widget(&mut ui, "control")?;
    let edit = ui.event(&id, |cx| amount.begin_edit(cx, Revision::INITIAL))??;
    edit.cancel()?;
    let token = edit.token().clone();
    drop(edit);
    assert_eq!(
        host.resolve_delivery(&token, DeliveryResolution::Retired),
        Err(RuntimeError::Binding(BindingError::Delivery(
            DeliveryError::NotRemoved
        )))
    );
    host.remove()?;
    assert!(monitor.snapshot().registered);
    host.resolve_delivery(&token, DeliveryResolution::Retired)?;
    assert!(!monitor.snapshot().is_faulted());
    assert!(!amount.snapshot().get()?.available);
    Ok(())
}

#[test]
fn host_drop_delivers_then_releases_and_shutdown_notifies_before_closing() -> TestResult {
    let mut ui = Runtime::new();
    let (amount, host) = ui.parameter(
        ParameterId::new(1, 1, "amount")?,
        ParameterSpec::new("Amount", 0.0, 1.0, 0.0)?,
    )?;
    let terminals: Log<EditOutcome<f32>> = Rc::default();
    let log = terminals.clone();
    let monitor = host.on_outcome(move |event| {
        log.borrow_mut().push(event.outcome.clone());
        Ok(())
    })?;
    let id = widget(&mut ui, "control")?;
    let edit = ui.event(&id, |cx| amount.begin_edit(cx, Revision::INITIAL))??;
    drop(host);
    let unavailable = EditOutcome::Rejected(EditError::Unavailable);
    assert_eq!(
        terminals.borrow().as_slice(),
        std::slice::from_ref(&unavailable)
    );
    assert!(!monitor.snapshot().registered);
    assert!(!monitor.snapshot().is_faulted());
    drop(edit);

    // Shutdown unmounts the editing component first, which cancels the edit; the
    // notification is delivered while reads still work, then sinks are released.
    let (gain, gain_host) = ui.parameter(
        ParameterId::new(1, 2, "gain")?,
        ParameterSpec::new("Gain", 0.0, 1.0, 0.0)?,
    )?;
    let reads: Log<bool> = Rc::default();
    let (log, reader, seen) = (terminals.clone(), gain.clone(), reads.clone());
    let gain_monitor = gain_host.on_outcome(move |event| {
        seen.borrow_mut().push(reader.snapshot().get().is_ok());
        log.borrow_mut().push(event.outcome.clone());
        Ok(())
    })?;
    let edit = ui.event(&id, |cx| gain.begin_edit(cx, Revision::INITIAL))??;
    drop(ui);
    assert_eq!(
        terminals.borrow().as_slice(),
        [unavailable, EditOutcome::Cancelled]
    );
    assert_eq!(reads.borrow().as_slice(), [true]);
    // The owned report stays readable after closure; no callback runs later.
    assert!(!gain_monitor.snapshot().registered);
    drop(edit);
    drop(gain_host);
    assert_eq!(terminals.borrow().len(), 2);
    assert_eq!(gain.snapshot().get(), Err(RuntimeError::Closed));
    Ok(())
}

#[test]
fn an_undeliverable_release_is_stored_as_closed_instead_of_dropped() -> TestResult {
    let mut ui = Runtime::new();
    let (first, first_host) = ui.parameter(
        ParameterId::new(1, 1, "first")?,
        ParameterSpec::boolean("First", false)?,
    )?;
    let (second, second_host) = ui.parameter(
        ParameterId::new(1, 2, "second")?,
        ParameterSpec::boolean("Second", false)?,
    )?;
    let second_monitor = second_host.on_outcome(|_| Ok(()))?;
    // The first sink drops the second parameter's owner inside its callback.
    let owned = Rc::new(RefCell::new(Some(second_host)));
    let slot = owned.clone();
    let _first_monitor = first_host.on_outcome(move |_| {
        slot.borrow_mut().take();
        Ok(())
    })?;
    let id = widget(&mut ui, "control")?;
    let pending = ui.event(&id, |cx| second.begin_edit(cx, Revision::INITIAL))??;
    let edit = ui.event(&id, |cx| first.begin_edit(cx, Revision::INITIAL))??;
    edit.commit(true)?;
    assert!(owned.borrow().is_none());
    let report = second_monitor.snapshot();
    assert!(!report.registered);
    assert!(report.is_faulted());
    assert_eq!(
        report.last_error.as_ref().map(SinkError::kind),
        Some(SinkErrorKind::Closed)
    );
    assert_eq!(
        report.terminal.map(|event| event.outcome),
        Some(EditOutcome::Rejected(EditError::Unavailable))
    );
    assert_eq!(
        pending.outcome()?,
        Some(EditOutcome::Rejected(EditError::Unavailable))
    );
    Ok(())
}

#[test]
fn a_sink_may_drop_its_own_owner() -> TestResult {
    let mut ui = Runtime::new();
    let (flag, host) = ui.parameter(
        ParameterId::new(1, 1, "flag")?,
        ParameterSpec::boolean("Flag", false)?,
    )?;
    let owned = Rc::new(RefCell::new(Some(host)));
    let slot = owned.clone();
    let monitor = owned
        .borrow()
        .as_ref()
        .ok_or("owner")?
        .on_outcome(move |_| {
            slot.borrow_mut().take();
            Ok(())
        })?;
    let id = widget(&mut ui, "control")?;
    let edit = ui.event(&id, |cx| flag.begin_edit(cx, Revision::INITIAL))??;
    edit.commit(true)?;
    assert!(owned.borrow().is_none());
    assert!(!flag.snapshot().get()?.available);
    let report = monitor.snapshot();
    assert!(!report.registered);
    assert!(!report.is_faulted());
    Ok(())
}

#[test]
fn erased_handles_check_the_family_before_any_mutation() -> TestResult {
    let mut ui = Runtime::new();
    let low = ChoiceId::new("low")?;
    let high = ChoiceId::new("high")?;
    let spec = ParameterSpec::choices(
        "Range",
        [
            ChoiceOption::new(low.clone(), "Low"),
            ChoiceOption::new(high.clone(), "High"),
        ],
        low.clone(),
    )?;
    let (parameter, host) =
        ui.parameter_any(ParameterId::new(1, 1, "range")?, AnySpec::from(spec))?;
    assert_eq!(parameter.family(), ParameterFamily::Choice);
    assert_eq!(host.family(), ParameterFamily::Choice);
    let events: Log<AnyTerminalEvent> = Rc::default();
    let log = events.clone();
    let _monitor = host.on_outcome(move |event| {
        log.borrow_mut().push(event.clone());
        Ok(())
    })?;
    let before = parameter.snapshot()?;
    assert_eq!(
        host.write_base(AnyValue::Stepped(1)),
        Err(RuntimeError::Binding(BindingError::KindMismatch {
            expected: ParameterFamily::Choice,
            found: ParameterFamily::Stepped
        }))
    );
    assert_eq!(
        host.replace_spec(AnySpec::from(ParameterSpec::boolean("X", true)?)),
        Err(RuntimeError::Binding(BindingError::KindMismatch {
            expected: ParameterFamily::Choice,
            found: ParameterFamily::Boolean
        }))
    );
    assert_eq!(parameter.snapshot()?, before);
    host.write_base(AnyValue::Choice(high.clone()))?;
    assert_eq!(parameter.snapshot()?.base(), AnyValue::Choice(high));
    let AnyParameter::Choice(typed) = &parameter else {
        return Err("tagged as a choice".into());
    };
    let id = widget(&mut ui, "control")?;
    let edit = ui.event(&id, |cx| {
        typed.begin_edit(cx, typed.snapshot().get()?.revision)
    })??;
    edit.commit(low.clone())?;
    assert!(matches!(
        events.borrow().as_slice(),
        [AnyTerminalEvent::Choice(event)]
            if event.outcome == EditOutcome::Committed { value: low.clone(), revision: Revision::INITIAL.max(event.state.revision) }
    ));
    assert!(matches!(host, AnyParameterHost::Choice(_)));
    let inspection = ui.inspect();
    assert_eq!(inspection.parameters[0].delivery, DeliveryStatus::Ready);
    assert_eq!(
        inspection.parameters[0].parameter.base(),
        AnyValue::Choice(low)
    );
    Ok(())
}

#[test]
fn a_delivered_deferred_notification_leaves_nothing_due() -> TestResult {
    let mut ui = Runtime::new();
    let (amount, host) = ui.parameter(
        ParameterId::new(1, 1, "amount")?,
        ParameterSpec::new("Amount", 0.0, 1.0, 0.0)?,
    )?;
    let _monitor = host.on_outcome(|_| Ok(()))?;
    ui.flush(Duration::ZERO)?;
    for _ in 0..3 {
        let id = widget(&mut ui, "control")?;
        let edit = ui.event(&id, |cx| {
            amount.begin_edit(cx, amount.snapshot().get()?.revision)
        })??;
        ui.unmount(&id)?;
        drop(edit);
        assert_eq!(ui.next_deadline(), Some(Duration::ZERO));
        // Delivered by the next operation on the parameter, not only by a flush.
        host.write_base(0.5)?;
        assert_eq!(ui.next_deadline(), None);
    }
    Ok(())
}

#[test]
fn every_erased_write_of_another_family_changes_nothing() -> TestResult {
    let mut ui = Runtime::new();
    let id = ParameterId::new(1, 1, "amount")?;
    let spec = ParameterSpec::new("Amount", 0.0, 1.0, 0.5)?;
    let (parameter, host) = ui.parameter_any(id.clone(), AnySpec::from(spec))?;
    assert_eq!(host.id(), &id);
    assert_eq!(parameter.id(), host.id());
    host.set_effective(Some(AnyValue::Continuous(0.75)))?;
    let before = parameter.snapshot()?;
    let mismatch = |found| {
        RuntimeError::Binding(BindingError::KindMismatch {
            expected: ParameterFamily::Continuous,
            found,
        })
    };
    assert_eq!(
        host.write_base(AnyValue::Boolean(true)),
        Err(mismatch(ParameterFamily::Boolean))
    );
    assert_eq!(
        host.replace_spec(AnySpec::from(ParameterSpec::boolean("Amount", true)?)),
        Err(mismatch(ParameterFamily::Boolean))
    );
    assert_eq!(
        host.set_effective(Some(AnyValue::Stepped(1))),
        Err(mismatch(ParameterFamily::Stepped))
    );
    let after = parameter.snapshot()?;
    assert_eq!(
        after, before,
        "base, overlay and specification are unchanged"
    );
    assert_eq!(after.revision(), before.revision());
    Ok(())
}

#[test]
fn an_erased_terminal_event_exposes_its_baseline_and_outcome() -> TestResult {
    let mut ui = Runtime::new();
    let (parameter, host) = ui.parameter_any(
        ParameterId::new(1, 1, "steps")?,
        AnySpec::from(ParameterSpec::stepped("Steps", 0, 8, 2)?),
    )?;
    let events: Log<AnyTerminalEvent> = Rc::default();
    let log = events.clone();
    let _monitor = host.on_outcome(move |event| {
        log.borrow_mut().push(event.clone());
        Ok(())
    })?;
    let AnyParameter::Stepped(typed) = &parameter else {
        return Err("tagged as stepped".into());
    };
    let id = widget(&mut ui, "control")?;
    let edit = ui.event(&id, |cx| {
        typed.begin_edit(cx, typed.snapshot().get()?.revision)
    })??;
    edit.commit(5)?;
    drop(edit);
    let edit = ui.event(&id, |cx| {
        typed.begin_edit(cx, typed.snapshot().get()?.revision)
    })??;
    edit.cancel()?;
    let events = events.borrow();
    let [committed, cancelled] = events.as_slice() else {
        return Err("two terminal events".into());
    };
    assert_eq!(committed.begin_value(), AnyValue::Stepped(2));
    assert!(matches!(
        committed.outcome(),
        EditOutcome::Committed {
            value: AnyValue::Stepped(5),
            ..
        }
    ));
    assert_eq!(cancelled.begin_value(), AnyValue::Stepped(5));
    assert_eq!(cancelled.outcome(), EditOutcome::Cancelled);
    Ok(())
}

type Remote = (Runtime, Parameter<f32>, RemoteHost<f32>, WidgetId);

fn remote() -> Result<Remote, Box<dyn std::error::Error>> {
    let mut ui = Runtime::new();
    let (parameter, host) = ui.remote_parameter(
        ParameterId::new(1, 1, "amount")?,
        ParameterSpec::new("Amount", 0.0, 1.0, 0.25)?,
    )?;
    let id = widget(&mut ui, "control")?;
    Ok((ui, parameter, host, id))
}

/// Submits `value` from a fresh gesture and returns the retained proposal.
fn submit(
    ui: &mut Runtime,
    parameter: &Parameter<f32>,
    id: &WidgetId,
    value: f32,
) -> Result<RemoteCommit<f32>, Box<dyn std::error::Error>> {
    let edit = ui.event(id, |cx| {
        parameter.begin_edit(cx, parameter.snapshot().get()?.revision)
    })??;
    assert!(matches!(edit.commit(value)?, EditOutcome::Submitted { .. }));
    drop(edit);
    match parameter
        .authority_status()
        .ok_or("a remote binding")?
        .get()?
    {
        AuthorityStatus::Pending(request) => Ok(request),
        other => Err(format!("expected a pending proposal, found {other:?}").into()),
    }
}

#[test]
fn a_retry_without_a_submission_sink_is_refused_without_change() -> TestResult {
    let (mut ui, parameter, host, id) = remote()?;
    let request = submit(&mut ui, &parameter, &id, 0.6)?;
    let before = parameter.snapshot().get()?;
    assert_eq!(
        host.retry_submission(&request),
        Err(RuntimeError::Binding(BindingError::Delivery(
            DeliveryError::NoSubmissionSink
        )))
    );
    assert_eq!(parameter.snapshot().get()?, before);
    assert_eq!(host.request(), Some(request));
    assert_eq!(
        host.submission_delivery().get()?,
        SubmissionDelivery::Unobserved
    );
    Ok(())
}

#[test]
fn a_retry_while_disconnected_is_unavailable_without_change() -> TestResult {
    let (mut ui, parameter, host, id) = remote()?;
    host.on_submit(|_| Err(SinkError::new(SinkErrorKind::Full, "transport full")))?;
    let request = submit(&mut ui, &parameter, &id, 0.6)?;
    host.disconnect()?;
    let before = parameter.snapshot().get()?;
    let delivery = host.submission_delivery().get()?;
    assert_eq!(
        host.retry_submission(&request),
        Err(RuntimeError::Binding(BindingError::Edit(
            EditError::Unavailable
        )))
    );
    assert_eq!(parameter.snapshot().get()?, before);
    assert_eq!(host.submission_delivery().get()?, delivery);
    assert!(matches!(
        host.receipt().ok_or("a receipt")?.status(),
        AuthorityStatus::Unknown(unknown) if unknown == request
    ));
    Ok(())
}

#[test]
fn a_retry_whose_sink_fails_again_keeps_the_proposal_and_the_new_error() -> TestResult {
    let (mut ui, parameter, host, id) = remote()?;
    let attempts = Rc::new(Cell::new(0));
    let counter = attempts.clone();
    host.on_submit(move |_| {
        counter.set(counter.get() + 1);
        Err(SinkError::new(
            SinkErrorKind::Full,
            format!("attempt {}", counter.get()),
        ))
    })?;
    let request = submit(&mut ui, &parameter, &id, 0.6)?;
    let before = parameter.snapshot().get()?;
    assert_eq!(
        host.retry_submission(&request),
        Err(RuntimeError::Binding(BindingError::Delivery(
            DeliveryError::SinkFailed
        )))
    );
    assert_eq!(attempts.get(), 2);
    assert_eq!(
        host.submission_delivery().get()?,
        SubmissionDelivery::Failed(SinkError::new(SinkErrorKind::Full, "attempt 2"))
    );
    let after = parameter.snapshot().get()?;
    assert_eq!(after, before);
    assert_eq!(after.base, 0.25, "a failed handoff is not acceptance");
    assert_eq!(host.request(), Some(request.clone()));
    assert!(matches!(
        host.receipt().ok_or("a receipt")?.status(),
        AuthorityStatus::Pending(pending) if pending == request
    ));
    Ok(())
}

#[test]
fn an_idle_disconnect_reports_disconnected_and_makes_the_binding_unavailable() -> TestResult {
    let (_ui, parameter, host, _id) = remote()?;
    let status = parameter.authority_status().ok_or("a remote binding")?;
    assert_eq!(status.get()?, AuthorityStatus::Ready);
    let revision = parameter.snapshot().get()?.revision;
    host.disconnect()?;
    assert_eq!(status.get()?, AuthorityStatus::Disconnected);
    let snapshot = parameter.snapshot().get()?;
    assert!(!snapshot.available);
    assert_eq!(snapshot.revision, revision);
    assert_eq!(host.request(), None);
    assert!(host.receipt().is_none(), "nothing was submitted");
    Ok(())
}

#[test]
fn binding_handles_format_their_identity() -> TestResult {
    let mut ui = Runtime::new();
    let (parameter, host) = ui.parameter(
        ParameterId::new(1, 1, "amount")?,
        ParameterSpec::new("Amount", 0.0, 1.0, 0.5)?,
    )?;
    let monitor = host.on_outcome(|_| Ok(()))?;
    for text in [
        format!("{parameter:?}"),
        format!("{host:?}"),
        format!("{:?}", parameter.snapshot()),
        format!("{monitor:?}"),
    ] {
        assert!(!text.is_empty());
    }
    assert!(format!("{parameter:?}").contains("amount"));
    assert!(format!("{monitor:?}").contains("registered: true"));
    Ok(())
}

#[test]
fn a_submitted_commit_ended_by_a_conflict_keeps_its_commit_sequence() -> TestResult {
    let (mut ui, parameter, host, id) = remote()?;
    let events: Log<TerminalEvent<f32>> = Rc::default();
    let log = events.clone();
    let _monitor = host.local_host().on_outcome(move |event| {
        log.borrow_mut().push(event.clone());
        Ok(())
    })?;
    submit(&mut ui, &parameter, &id, 0.6)?;
    host.local_host().write_base(0.3)?;
    let events = events.borrow();
    let [event] = events.as_slice() else {
        return Err("one terminal event".into());
    };
    assert_eq!(event.outcome, EditOutcome::Rejected(EditError::Conflict));
    assert_eq!(event.commit_sequence, Some(EditSequence::new(1)));
    Ok(())
}
