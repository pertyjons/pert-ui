//! External authority through ordinary public parameter bindings.
use pert_ui::*;
type Result = std::result::Result<(), Box<dyn std::error::Error>>;
type Fixture = (Runtime, Parameter<f32>, RemoteHost<f32>);
fn fixture() -> std::result::Result<Fixture, Box<dyn std::error::Error>> {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(300.0, 200.0, 1.0)?)?;
    let (parameter, host) = ui.remote_parameter(
        ParameterId::new(1, 1, "amount")?,
        ParameterSpec::new("Amount", 0.0, 1.0, 0.25)?,
    )?;
    Ok((ui, parameter, host))
}
#[test]
fn submission_retains_preview_and_base_until_corrected_acceptance() -> Result {
    let (mut ui, parameter, host) = fixture()?;
    let id = ui.mount_tree(Element::new("slider", Slider::new(parameter.clone())))?;
    ui.flush(std::time::Duration::ZERO)?;
    ui.focus(&id)?;
    ui.key_input(KeyCode::ArrowRight, KeyAction::Press)?;
    ui.key_input(KeyCode::ArrowRight, KeyAction::Release)?;
    let request = host
        .request()
        .expect("ordinary slider submitted a proposal");
    assert_eq!(parameter.snapshot().get()?.base, 0.25);
    assert_ne!(*request.value(), 0.25);
    let receipt = host.receipt().unwrap();
    assert!(matches!(receipt.status(), AuthorityStatus::Pending(_)));
    assert_eq!(
        host.respond(&request, AuthorityResponse::Accepted(0.75))?,
        ResponseDisposition::Applied
    );
    assert_eq!(parameter.snapshot().get()?.base, 0.75);
    assert!(matches!(
        receipt.status(),
        AuthorityStatus::Resolved {
            outcome: EditOutcome::Committed { value: 0.75, .. },
            ..
        }
    ));
    assert_eq!(
        host.respond(&request, AuthorityResponse::Accepted(0.5))?,
        ResponseDisposition::AlreadyResolved
    );
    assert_eq!(parameter.snapshot().get()?.base, 0.75);
    Ok(())
}
#[test]
fn full_transport_guard_drop_rejection_and_invalid_correction_preserve_receipts() -> Result {
    let (mut ui, parameter, host) = fixture()?;
    let id = ui.mount_tree(Element::new("owner", Owner))?;
    let edit = ui.event(&id, |cx| {
        parameter.begin_edit(cx, parameter.snapshot().get().unwrap().revision)
    })??;
    assert!(matches!(
        edit.commit(0.6)?,
        EditOutcome::Submitted { value: 0.6 }
    ));
    let request = host.request().unwrap();
    let receipt = host.receipt().unwrap();
    let (sender, receiver) = std::sync::mpsc::sync_channel(1);
    sender.try_send(request.clone())?;
    assert!(sender.try_send(request.clone()).is_err());
    assert_eq!(
        host.request(),
        Some(request.clone()),
        "full transport does not consume request"
    );
    drop(edit);
    ui.unmount(&id)?;
    assert!(matches!(receipt.status(), AuthorityStatus::Pending(_)));
    assert!(
        host.respond(&request, AuthorityResponse::Accepted(f32::NAN))
            .is_err()
    );
    assert_eq!(host.request(), Some(request.clone()));
    let delivered = receiver.recv()?;
    host.respond(&delivered, AuthorityResponse::Rejected)?;
    assert!(matches!(
        receipt.status(),
        AuthorityStatus::Resolved {
            outcome: EditOutcome::Rejected(EditError::RejectedByHost),
            ..
        }
    ));
    assert_eq!(parameter.snapshot().get()?.base, 0.25);
    Ok(())
}
#[test]
fn disconnect_reconnect_conflict_and_shutdown_never_invent_acknowledgement() -> Result {
    let (mut ui, parameter, host) = fixture()?;
    let id = ui.mount_tree(Element::new("owner", Owner))?;
    let edit = ui.event(&id, |cx| {
        parameter.begin_edit(cx, parameter.snapshot().get().unwrap().revision)
    })??;
    edit.commit(0.6)?;
    let old = host.request().unwrap();
    let receipt = host.receipt().unwrap();
    host.disconnect()?;
    assert!(!parameter.snapshot().get()?.available);
    assert!(matches!(receipt.status(), AuthorityStatus::Unknown(_)));
    host.resynchronize(0.4)?;
    assert!(matches!(receipt.status(), AuthorityStatus::Unknown(_)));
    assert_eq!(
        host.respond(&old, AuthorityResponse::Accepted(0.6))?,
        ResponseDisposition::Stale
    );
    let edit = ui.event(&id, |cx| {
        parameter.begin_edit(cx, parameter.snapshot().get().unwrap().revision)
    })??;
    edit.commit(0.8)?;
    let request = host.request().unwrap();
    let receipt = host.receipt().unwrap();
    host.local_host().write_base(0.3)?;
    assert!(matches!(
        receipt.status(),
        AuthorityStatus::Resolved {
            outcome: EditOutcome::Rejected(_),
            ..
        }
    ));
    // This is the first authority reply, refused after local same-generation
    // supersession rather than after an earlier authority acknowledgement.
    assert_eq!(
        host.respond(&request, AuthorityResponse::Accepted(0.8))?,
        ResponseDisposition::AlreadyResolved
    );
    let edit = ui.event(&id, |cx| {
        parameter.begin_edit(cx, parameter.snapshot().get().unwrap().revision)
    })??;
    edit.commit(0.9)?;
    let receipt = host.receipt().unwrap();
    ui.shutdown();
    assert!(matches!(receipt.status(), AuthorityStatus::Unknown(_)));
    Ok(())
}
struct Owner;
impl Component for Owner {
    fn update(&mut self, _: &mut UpdateContext) -> std::result::Result<(), RuntimeError> {
        Ok(())
    }
}
#[test]
fn invalid_submission_is_terminal_and_disconnect_cancels_unsent_preview() -> Result {
    let (mut ui, parameter, host) = fixture()?;
    let id = ui.mount_tree(Element::new("owner", Owner))?;
    let edit = ui.event(&id, |cx| {
        parameter.begin_edit(cx, parameter.snapshot().get().unwrap().revision)
    })??;
    edit.update(0.5)?;
    assert!(matches!(edit.commit(f32::NAN)?, EditOutcome::Rejected(_)));
    assert!(parameter.snapshot().get()?.preview.is_none());
    assert!(host.request().is_none());
    let edit = ui.event(&id, |cx| {
        parameter.begin_edit(cx, parameter.snapshot().get().unwrap().revision)
    })??;
    edit.update(0.5)?;
    host.disconnect()?;
    assert_eq!(edit.outcome()?, Some(EditOutcome::Cancelled));
    assert!(edit.update(0.7).is_err());
    host.resynchronize(0.25)?;
    let edit = ui.event(&id, |cx| {
        parameter.begin_edit(cx, parameter.snapshot().get().unwrap().revision)
    })??;
    edit.commit(0.8)?;
    let receipt = host.receipt().unwrap();
    host.local_host().remove()?;
    assert!(matches!(receipt.status(), AuthorityStatus::Unknown(_)));
    Ok(())
}

#[test]
fn terminal_sink_observes_resolved_authority_after_guard_drop() -> Result {
    let (mut ui, parameter, host) = fixture()?;
    let observed = std::rc::Rc::new(std::cell::Cell::new(false));
    let seen = observed.clone();
    let status = parameter.authority_status().unwrap();
    let _monitor = host.local_host().on_outcome(move |event| {
        assert!(matches!(event.outcome, EditOutcome::Committed { .. }));
        assert!(matches!(
            status.get().unwrap(),
            AuthorityStatus::Resolved { .. }
        ));
        seen.set(true);
        Ok(())
    })?;
    let id = ui.mount_tree(Element::new("owner", Owner))?;
    let edit = ui.event(&id, |cx| {
        parameter.begin_edit(cx, parameter.snapshot().get().unwrap().revision)
    })??;
    edit.commit(0.7)?;
    let request = host.request().unwrap();
    drop(edit);
    host.respond(&request, AuthorityResponse::Accepted(0.7))?;
    assert!(observed.get());
    Ok(())
}

#[test]
fn automatic_submission_retains_transport_errors_and_retries_only_explicitly() -> Result {
    let (mut ui, parameter, host) = fixture()?;
    let full = std::rc::Rc::new(std::cell::Cell::new(true));
    let blocked = full.clone();
    let calls = std::rc::Rc::new(std::cell::Cell::new(0));
    let observed = calls.clone();
    let status = parameter.authority_status().unwrap();
    let write = ui.signal(false);
    host.on_submit(move |request| {
        observed.set(observed.get() + 1);
        assert!(matches!(status.get().unwrap(),AuthorityStatus::Pending(p) if p == *request));
        assert_eq!(write.set(true), Err(RuntimeError::WriteDuringDelivery));
        if blocked.get() {
            Err(SinkError::new(SinkErrorKind::Full, "transport full"))
        } else {
            Ok(())
        }
    })?;
    let id = ui.mount_tree(Element::new("owner", Owner))?;
    let edit = ui.event(&id, |cx| {
        parameter.begin_edit(cx, parameter.snapshot().get().unwrap().revision)
    })??;
    assert!(matches!(edit.commit(0.6)?, EditOutcome::Submitted { .. }));
    let request = host.request().unwrap();
    assert_eq!(calls.get(), 1);
    assert!(
        matches!(host.submission_delivery().get()?,SubmissionDelivery::Failed(e) if e.kind()==SinkErrorKind::Full)
    );
    assert!(matches!(
        edit.commit(0.9)?,
        EditOutcome::Submitted { value: 0.6 }
    ));
    ui.flush(std::time::Duration::ZERO)?;
    assert_eq!(calls.get(), 1, "no automatic retries");
    drop(edit);
    full.set(false);
    host.retry_submission(&request)?;
    assert_eq!(calls.get(), 2);
    assert_eq!(
        host.submission_delivery().get()?,
        SubmissionDelivery::Delivered
    );
    assert!(matches!(
        host.retry_submission(&request),
        Err(RuntimeError::Binding(BindingError::Delivery(
            DeliveryError::AlreadyDelivered
        )))
    ));
    assert_eq!(
        parameter.snapshot().get()?.base,
        0.25,
        "handoff is not acceptance"
    );
    host.respond(&request, AuthorityResponse::Accepted(0.6))?;
    assert!(matches!(
        host.retry_submission(&request),
        Err(RuntimeError::Binding(BindingError::Delivery(
            DeliveryError::UnknownEdit
        )))
    ));
    host.disconnect()?;
    host.resynchronize(0.6)?;
    host.clear_submission_sink()?;
    let replacement = std::rc::Rc::new(std::cell::Cell::new(false));
    let observed = replacement.clone();
    host.on_submit(move |_| {
        observed.set(true);
        Ok(())
    })?;
    let edit = ui.event(&id, |cx| {
        parameter.begin_edit(cx, parameter.snapshot().get().unwrap().revision)
    })??;
    edit.commit(0.7)?;
    assert!(replacement.get());
    assert!(matches!(
        host.clear_submission_sink(),
        Err(RuntimeError::Binding(BindingError::Delivery(
            DeliveryError::ActiveEdit
        )))
    ));
    Ok(())
}

#[test]
fn shutdown_releases_a_submission_callback_with_a_reentrant_owner_destructor() -> Result {
    let (mut ui, parameter, host) = fixture()?;
    let host = std::rc::Rc::new(host);
    let weak = std::rc::Rc::downgrade(&host);
    let captured = host.clone();
    host.on_submit(move |_| {
        let _ = captured.request();
        Ok(())
    })?;
    drop(host);
    assert!(weak.upgrade().is_some());
    ui.shutdown();
    assert!(weak.upgrade().is_none());
    assert!(parameter.snapshot().get().is_err());
    Ok(())
}

fn parameter_events<T: ParameterValue>(ui: &Runtime, parameter: &Parameter<T>) -> Vec<EventRecord> {
    ui.inspect()
        .events
        .into_iter()
        .filter(|event| event.parameter.as_ref() == Some(parameter.id()))
        .collect()
}
#[test]
fn remote_history_distinguishes_submission_rejection_and_explicit_cancel() -> Result {
    let (mut ui, parameter, host) = fixture()?;
    ui.set_debug_mode(DebugMode::Inspect);
    let owner = ui.mount_tree(Element::new("history-owner", Owner))?;
    host.on_submit(|_| Err(SinkError::new(SinkErrorKind::Full, "transport full")))?;
    let edit = ui.event(&owner, |cx| {
        parameter.begin_edit(cx, parameter.snapshot().get()?.revision)
    })??;
    ui.clear_event_history();
    edit.commit(0.6)?;
    let events = parameter_events(&ui, &parameter);
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].kind, EventKind::EditCommit);
    assert_eq!(
        events[0].status,
        EventStatus::Succeeded,
        "submission is retained despite failed transport"
    );
    let request = host.request().expect("retained");
    ui.clear_event_history();
    assert!(
        host.respond(&request, AuthorityResponse::Accepted(f32::NAN))
            .is_err()
    );
    assert_eq!(host.request(), Some(request.clone()));
    assert!(
        parameter_events(&ui, &parameter).is_empty(),
        "invalid response did not attempt a state transition"
    );
    host.respond(&request, AuthorityResponse::Rejected)?;
    let events = parameter_events(&ui, &parameter);
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].kind, EventKind::EditCommit);
    assert!(matches!(events[0].status, EventStatus::Failed(_)));
    let edit = ui.event(&owner, |cx| {
        parameter.begin_edit(cx, parameter.snapshot().get()?.revision)
    })??;
    ui.clear_event_history();
    edit.cancel()?;
    let events = parameter_events(&ui, &parameter);
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].kind, EventKind::EditCancel);
    assert_eq!(events[0].status, EventStatus::Cancelled);
    let edit = ui.event(&owner, |cx| {
        parameter.begin_edit(cx, parameter.snapshot().get()?.revision)
    })??;
    edit.commit(0.7)?;
    let request = host.request().expect("accepted proposal");
    ui.clear_event_history();
    host.respond(&request, AuthorityResponse::Accepted(0.65))?;
    let events = parameter_events(&ui, &parameter);
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].kind, EventKind::EditCommit);
    assert_eq!(events[0].status, EventStatus::Succeeded);
    assert_eq!(parameter.snapshot().get()?.base, 0.65);
    Ok(())
}
#[test]
fn early_parameter_admission_refusals_are_recorded_once_and_stay_opt_in_bounded() -> Result {
    let (mut ui, parameter, host) = fixture()?;
    let owner = ui.mount_tree(Element::new("owner", Owner))?;
    host.disconnect()?;
    let refused = || RuntimeError::from(EditError::Unavailable);
    assert_eq!(
        ui.event(&owner, |cx| parameter
            .begin_edit(cx, parameter.snapshot().get()?.revision))?
            .err(),
        Some(refused())
    );
    assert!(
        ui.inspect().events.is_empty(),
        "history disabled by default"
    );
    ui.set_debug_mode(DebugMode::Inspect);
    ui.clear_event_history();
    assert_eq!(
        ui.event(&owner, |cx| parameter
            .begin_edit(cx, parameter.snapshot().get()?.revision))?
            .err(),
        Some(refused())
    );
    let events = parameter_events(&ui, &parameter);
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].kind, EventKind::EditBegin);
    assert!(matches!(events[0].status, EventStatus::Failed(_)));
    let mut other = Runtime::new();
    let foreign_owner = other.mount_tree(Element::new("foreign", Owner))?;
    ui.clear_event_history();
    for _ in 0..InspectionSnapshot::MAX_EVENTS + 1 {
        assert_eq!(
            other
                .event(&foreign_owner, |cx| parameter
                    .begin_edit(cx, parameter.snapshot().get()?.revision))?
                .err(),
            Some(RuntimeError::ForeignRuntime)
        );
    }
    let snapshot = ui.inspect();
    assert_eq!(snapshot.events.len(), InspectionSnapshot::MAX_EVENTS);
    assert_eq!(snapshot.dropped_events, 1);
    assert!(
        snapshot
            .events
            .iter()
            .all(|event| event.kind == EventKind::EditBegin
                && matches!(event.status, EventStatus::Failed(_)))
    );
    host.resynchronize(0.25)?;
    let monitor = host
        .local_host()
        .on_outcome(|_| Err(SinkError::new(SinkErrorKind::Full, "terminal full")))?;
    let edit = ui.event(&owner, |cx| {
        parameter.begin_edit(cx, parameter.snapshot().get()?.revision)
    })??;
    edit.commit(0.6)?;
    let request = host.request().expect("pending");
    host.respond(&request, AuthorityResponse::Rejected)?;
    assert_eq!(parameter.delivery_status().get()?, DeliveryStatus::Faulted);
    ui.clear_event_history();
    assert_eq!(
        ui.event(&owner, |cx| parameter
            .begin_edit(cx, parameter.snapshot().get()?.revision))?
            .err(),
        Some(RuntimeError::from(DeliveryError::Faulted))
    );
    assert_eq!(parameter_events(&ui, &parameter).len(), 1);
    drop(monitor);
    Ok(())
}

#[test]
fn unknown_authority_receipts_correlate_with_local_rejection_after_recovery_or_removal() -> Result {
    use std::{cell::RefCell, rc::Rc};
    for remove in [false, true] {
        let (mut ui, parameter, host) = fixture()?;
        let terminals = Rc::new(RefCell::new(Vec::new()));
        let recorded = terminals.clone();
        let _monitor = host.local_host().on_outcome(move |event| {
            recorded.borrow_mut().push(event.clone());
            Ok(())
        })?;
        let owner = ui.mount_tree(Element::new("owner", Owner))?;
        let edit = ui.event(&owner, |cx| {
            parameter.begin_edit(cx, parameter.snapshot().get()?.revision)
        })??;
        edit.commit(0.6)?;
        let request = host.request().expect("submitted");
        let receipt = host.receipt().expect("retained receipt");
        host.disconnect()?;
        if remove {
            host.local_host().remove()?;
        } else {
            host.resynchronize(0.4)?;
        }
        let AuthorityStatus::Unknown(unknown) = receipt.status() else {
            panic!("external outcome remains unknown")
        };
        assert_eq!(unknown, request);
        let events = terminals.borrow();
        assert_eq!(events.len(), 1);
        assert_eq!(&events[0].token, request.token());
        assert_eq!(
            events[0].outcome,
            EditOutcome::Rejected(if remove {
                EditError::Unavailable
            } else {
                EditError::Conflict
            })
        );
        assert!(events[0].commit_sequence.is_some());
        drop(events);
        assert_eq!(
            host.respond(&request, AuthorityResponse::Accepted(0.6))?,
            ResponseDisposition::Stale
        );
        assert!(matches!(receipt.status(), AuthorityStatus::Unknown(_)));
    }
    Ok(())
}

#[test]
fn connected_resynchronization_locally_supersedes_without_an_authority_response() -> Result {
    let (mut ui, parameter, host) = fixture()?;
    host.on_submit(|_| Ok(()))?;
    let owner = ui.mount_tree(Element::new("owner", Owner))?;
    let edit = ui.event(&owner, |cx| {
        parameter.begin_edit(cx, parameter.snapshot().get()?.revision)
    })??;
    edit.commit(0.6)?;
    let request = host.request().expect("submitted");
    let receipt = host
        .receipt()
        .expect("receipt retained before supersession");
    assert_eq!(
        host.submission_delivery().get()?,
        SubmissionDelivery::Delivered
    );
    assert_eq!(receipt.status(), AuthorityStatus::Pending(request.clone()));
    // The adapter installs a fresh baseline while connected. This local
    // conflict says nothing about the mutation already handed to transport.
    host.resynchronize(0.4)?;
    assert_eq!(
        receipt.status(),
        AuthorityStatus::Resolved {
            request: request.clone(),
            outcome: EditOutcome::Rejected(EditError::Conflict),
        }
    );
    assert_eq!(
        edit.outcome()?,
        Some(EditOutcome::Rejected(EditError::Conflict))
    );
    assert_eq!(parameter.snapshot().get()?.base, 0.4);
    assert_eq!(
        host.respond(&request, AuthorityResponse::Accepted(0.6))?,
        ResponseDisposition::Stale
    );
    assert_eq!(parameter.snapshot().get()?.base, 0.4);
    Ok(())
}
