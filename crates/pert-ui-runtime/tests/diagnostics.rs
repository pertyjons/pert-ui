//! Versioned inspection, bounded history and read-only tool integration.
use pert_ui_runtime::testing::HostIntegration;
use pert_ui_runtime::*;
use std::{collections::BTreeMap, time::Duration};

type TestResult = Result<(), Box<dyn std::error::Error>>;

struct Inspectable {
    value: Signal<i32>,
}
impl Component for Inspectable {
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        cx.read(&self.value)?;
        Ok(())
    }
    fn inspect(&self) -> ComponentDetails {
        assert_eq!(self.value.set(99), Err(RuntimeError::WriteDuringInspection));
        let mut details = ComponentDetails::default();
        details.properties.insert(
            "value".into(),
            InspectValue::Number(f64::from(self.value.get().unwrap())),
        );
        details
    }
}

#[test]
fn snapshot_is_owned_read_only_and_includes_explicit_host_state() -> TestResult {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<InspectionSnapshot>();
    let mut runtime = Runtime::new();
    let value = runtime.signal(3);
    let id = runtime.mount_tree(Element::new(
        "inspectable",
        Inspectable {
            value: value.clone(),
        },
    ))?;
    // Unknown geometry stays absent until the root is measured.
    assert_eq!(
        runtime.inspect().components[0].details.box_model.border_box,
        None
    );
    let (parameter, _host) = runtime.parameter(
        ParameterId::new(1, 2, "x")?,
        ParameterSpec::new("X", 0.0, 1.0, 0.5)?,
    )?;
    runtime.flush(Duration::ZERO)?;
    runtime.focus(&id)?;
    runtime.key_input(KeyCode::KeyA, KeyAction::Press)?;
    runtime.set_debug_mode(DebugMode::Inspect);
    let snapshot = runtime.inspect_with(|| {
        BTreeMap::from([("document".into(), InspectValue::Text("Untitled".into()))])
    });
    assert_eq!(snapshot.schema_version, 15);
    assert_eq!(snapshot.mode, DebugMode::Inspect);
    assert_eq!(snapshot.components.len(), 1);
    assert!(snapshot.components[0].focused);
    // Laid out without a viewport: the empty box layout assigns.
    assert_eq!(
        snapshot.components[0].details.box_model.border_box,
        Some([0.0; 4])
    );
    assert_eq!(
        snapshot.components[0].details.properties["value"],
        InspectValue::Number(3.0)
    );
    assert_eq!(
        snapshot.parameters[0].parameter,
        pert_ui_runtime::AnyParameterSnapshot::new(parameter.spec()?, parameter.snapshot().get()?)
    );
    assert_eq!(snapshot.pressed_keys, [KeyCode::KeyA]);
    assert!(
        snapshot.events.is_empty(),
        "diagnostics were off during prior operations"
    );
    assert_eq!(value.get()?, 3);
    runtime.set_visible(&id, false)?;
    assert!(!runtime.inspect().components[0].visible);
    assert!(
        snapshot.components[0].visible,
        "owned snapshot cannot change retroactively"
    );
    Ok(())
}

#[test]
fn history_records_success_failure_cancellation_and_specific_keys() -> TestResult {
    let mut runtime = Runtime::new();
    runtime.set_debug_mode(DebugMode::Inspect);
    let value = runtime.signal(0);
    let id = runtime.mount_tree(Element::new("widget", Inspectable { value }))?;
    runtime.try_event(&id, |_| Ok::<_, &str>(()))??;
    assert_eq!(
        runtime.try_event(&id, |_| Err::<(), _>("application rejected"))?,
        Err("application rejected")
    );
    runtime.key_input(KeyCode::KeyA, KeyAction::Press)?;
    runtime.key_input(KeyCode::KeyA, KeyAction::Press)?;
    let (parameter, host) = runtime.parameter(
        ParameterId::new(1, 2, "x")?,
        ParameterSpec::new("X", 0.0, 1.0, 0.5)?,
    )?;
    let edit = runtime.event(&id, |cx| {
        parameter.begin_edit(cx, parameter.snapshot().get()?.revision)
    })??;
    assert!(host.write_base(f32::NAN).is_err());
    edit.cancel()?;
    let records = runtime.inspect().events;
    assert!(
        records
            .iter()
            .any(|r| r.kind == EventKind::Application && r.status == EventStatus::Succeeded)
    );
    assert!(
        records
            .iter()
            .any(|r| r.status == EventStatus::Failed("application rejected".into()))
    );
    assert!(
        records
            .iter()
            .any(|r| r.kind == EventKind::HostWrite && matches!(r.status, EventStatus::Failed(_)))
    );
    assert!(
        records
            .iter()
            .any(|r| r.kind == EventKind::EditCancel && r.status == EventStatus::Cancelled)
    );
    assert!(records.iter().any(|r| r.kind
        == EventKind::KeyInput(KeyEvent {
            code: KeyCode::KeyA,
            transition: KeyTransition::Repeated
        })));
    assert!(
        records
            .windows(2)
            .all(|pair| pair[0].sequence < pair[1].sequence)
    );
    Ok(())
}

#[test]
fn a_generic_event_is_dispatched_and_a_long_failure_message_is_truncated() -> TestResult {
    let mut runtime = Runtime::new();
    runtime.set_debug_mode(DebugMode::Inspect);
    let value = runtime.signal(0);
    let id = runtime.mount_tree(Element::new("widget", Inspectable { value }))?;
    // The runtime cannot know what an arbitrary closure achieved.
    runtime.event(&id, |_| ())?;
    let last = runtime.inspect().events.pop().ok_or("empty history")?;
    assert_eq!(last.kind, EventKind::Dispatch);
    assert_eq!(last.status, EventStatus::Dispatched);
    // Characters, not bytes: a multibyte message is cut on a character.
    let long = "é".repeat(EventStatus::MAX_MESSAGE_CHARS + 10);
    assert!(
        runtime
            .try_event(&id, |_| Err::<(), _>(long.as_str()))?
            .is_err()
    );
    let last = runtime.inspect().events.pop().ok_or("empty history")?;
    let expected: String = long.chars().take(EventStatus::MAX_MESSAGE_CHARS).collect();
    assert_eq!(last.status, EventStatus::Failed(expected));
    assert_eq!(EventStatus::MAX_MESSAGE_CHARS, 512);
    let exact = "x".repeat(EventStatus::MAX_MESSAGE_CHARS);
    assert!(
        runtime
            .try_event(&id, |_| Err::<(), _>(exact.as_str()))?
            .is_err()
    );
    let last = runtime.inspect().events.pop().ok_or("empty history")?;
    assert_eq!(last.status, EventStatus::Failed(exact));
    Ok(())
}

#[test]
fn history_is_bounded_clearable_and_never_schedules_work() -> TestResult {
    let mut runtime = Runtime::new();
    runtime.set_debug_mode(DebugMode::Inspect);
    for _ in 0..300 {
        runtime.clear_focus();
    }
    let snapshot = runtime.inspect();
    assert_eq!(InspectionSnapshot::MAX_EVENTS, 256);
    assert_eq!(snapshot.events.len(), InspectionSnapshot::MAX_EVENTS);
    assert_eq!(snapshot.dropped_events, 44);
    assert_eq!(runtime.next_deadline(), None);
    let last = snapshot.events.last().ok_or("empty history")?.sequence;
    runtime.clear_event_history();
    runtime.clear_focus();
    assert!(runtime.inspect().events[0].sequence > last);
    assert_eq!(runtime.inspect().dropped_events, 0);
    runtime.set_debug_mode(DebugMode::Off);
    for _ in 0..10 {
        runtime.clear_focus();
    }
    assert_eq!(runtime.inspect().events.len(), 1);
    Ok(())
}

#[test]
fn inspection_panic_restores_normal_write_phase() -> TestResult {
    let runtime = Runtime::new();
    let value = runtime.signal(1);
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        runtime.inspect_with(|| panic!("provider panic"))
    }));
    assert!(result.is_err());
    value.set(2)?;
    Ok(())
}

#[test]
fn automatic_cleanup_does_not_log_cancellation_for_terminal_edits() -> TestResult {
    for terminal in ["commit", "cancel", "conflict", "remove", "reject"] {
        for cleanup in ["drop", "focus", "hide", "unmount"] {
            let mut runtime = Runtime::new();
            runtime.set_debug_mode(DebugMode::Inspect);
            let value = runtime.signal(0);
            let widget = runtime.mount_tree(Element::new("control", Inspectable { value }))?;
            runtime.focus(&widget)?;
            let (parameter, host) = runtime.parameter(
                ParameterId::new(1, 1, "value")?,
                ParameterSpec::new("Value", 0.0, 1.0, 0.0)?,
            )?;
            let edit = runtime.event(&widget, |cx| {
                parameter.begin_edit(cx, parameter.snapshot().get()?.revision)
            })??;
            match terminal {
                "commit" => {
                    edit.commit(0.5)?;
                }
                "cancel" => {
                    edit.cancel()?;
                }
                "conflict" => host.write_base(0.75)?,
                "remove" => host.remove()?,
                "reject" => {
                    edit.commit(f32::NAN)?;
                }
                _ => unreachable!(),
            }
            let outcome = edit.outcome()?;
            let snapshot = parameter.snapshot().get()?;
            runtime.clear_event_history();
            match cleanup {
                "drop" => (),
                "focus" => runtime.clear_focus(),
                "hide" => runtime.set_visible(&widget, false)?,
                "unmount" => runtime.unmount(&widget)?,
                _ => unreachable!(),
            }
            assert_eq!(edit.outcome()?, outcome);
            drop(edit);
            assert_eq!(parameter.snapshot().get()?, snapshot);
            assert!(
                runtime
                    .inspect()
                    .events
                    .iter()
                    .all(|event| event.kind != EventKind::EditCancel),
                "terminal {terminal}, cleanup {cleanup}"
            );
        }
    }
    Ok(())
}

#[test]
fn unfinished_cleanup_logs_one_cancel_and_reclaims_receipts() -> TestResult {
    let mut runtime = Runtime::new();
    runtime.set_debug_mode(DebugMode::Inspect);
    let value = runtime.signal(0);
    let widget = runtime.mount_tree(Element::new("control", Inspectable { value }))?;
    let (parameter, _host) = runtime.parameter(
        ParameterId::new(1, 1, "value")?,
        ParameterSpec::new("Value", 0.0, 1.0, 0.0)?,
    )?;
    // More than the 64 reserved receipt slots: every guard must reclaim its slot.
    for context_loss in (0..130).map(|n| n % 2 == 0) {
        runtime.focus(&widget)?;
        let edit = runtime.event(&widget, |cx| {
            parameter.begin_edit(cx, parameter.snapshot().get()?.revision)
        })??;
        edit.update(0.5)?;
        runtime.clear_event_history();
        if context_loss {
            runtime.clear_focus();
            assert_eq!(edit.outcome()?, Some(EditOutcome::Cancelled));
        }
        drop(edit);
        let cancellations: Vec<_> = runtime
            .inspect()
            .events
            .into_iter()
            .filter(|event| event.kind == EventKind::EditCancel)
            .collect();
        assert_eq!(cancellations.len(), 1);
        assert_eq!(cancellations[0].status, EventStatus::Cancelled);
        assert_eq!(parameter.snapshot().get()?.base, 0.0);
        assert_eq!(parameter.snapshot().get()?.preview, None);
    }
    Ok(())
}
