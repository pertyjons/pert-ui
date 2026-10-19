//! Public headless runtime, subscriptions, input and edit-lifecycle contracts.
use pert_ui_runtime::{
    BindingError, EditError, EditOutcome, InputOutcome, RuntimeError, prelude::*,
};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::Duration,
};

type TestResult = Result<(), Box<dyn std::error::Error>>;
type ProbeLog = Rc<RefCell<Vec<(i32, Invalidation)>>>;

struct Probe {
    value: ReadSignal<i32>,
    seen: Rc<RefCell<Vec<(i32, Invalidation)>>>,
    live: bool,
}
impl Component for Probe {
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        let value = cx.read_paint(&self.value)?;
        self.seen.borrow_mut().push((value, cx.invalidation()));
        if self.live {
            cx.animate()?;
        }
        Ok(())
    }
}
fn probe(value: &Signal<i32>, live: bool) -> (Probe, ProbeLog) {
    let seen = Rc::new(RefCell::new(Vec::new()));
    (
        Probe {
            value: value.read_only(),
            seen: seen.clone(),
            live,
        },
        seen,
    )
}

#[test]
fn signal_changes_are_coalesced_and_only_reach_dependents() -> TestResult {
    let mut runtime = Runtime::new();
    let a = runtime.signal(1);
    let b = runtime.signal(2);
    let (one, seen_one) = probe(&a, false);
    let (two, seen_two) = probe(&a, false);
    let (other, seen_other) = probe(&b, false);
    runtime.mount("one", one)?;
    runtime.mount("two", two)?;
    runtime.mount("other", other)?;
    assert_eq!(runtime.flush(Duration::ZERO)?.components_updated, 3);
    assert_eq!(runtime.next_deadline(), None);
    assert!(!a.set_changed(1)?);
    assert_eq!(runtime.next_deadline(), None);
    a.set(3)?;
    a.set(4)?;
    assert_eq!(
        seen_one.borrow().len(),
        1,
        "notification must not invoke callbacks"
    );
    assert_eq!(runtime.flush(Duration::ZERO)?.components_updated, 2);
    assert_eq!(seen_one.borrow().last(), Some(&(4, Invalidation::Paint)));
    assert_eq!(seen_two.borrow().len(), 2);
    assert_eq!(seen_other.borrow().len(), 1);
    assert_eq!(runtime.next_deadline(), None);
    Ok(())
}

struct Conditional {
    choose: ReadSignal<bool>,
    a: ReadSignal<i32>,
    b: ReadSignal<i32>,
    seen: Rc<RefCell<Vec<i32>>>,
}
impl Component for Conditional {
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        let value = if cx.read(&self.choose)? {
            cx.read(&self.a)?
        } else {
            cx.read(&self.b)?
        };
        self.seen.borrow_mut().push(value);
        Ok(())
    }
}

#[test]
fn conditional_dependencies_are_recollected_and_unmount_releases_them() -> TestResult {
    let mut runtime = Runtime::new();
    let choose = runtime.signal(true);
    let a = runtime.signal(1);
    let b = runtime.signal(2);
    let seen = Rc::new(RefCell::new(Vec::new()));
    let id = runtime.mount(
        "conditional",
        Conditional {
            choose: choose.read_only(),
            a: a.read_only(),
            b: b.read_only(),
            seen: seen.clone(),
        },
    )?;
    runtime.flush(Duration::ZERO)?;
    choose.set(false)?;
    runtime.flush(Duration::ZERO)?;
    a.set(3)?;
    assert_eq!(runtime.next_deadline(), None);
    b.set(4)?;
    runtime.flush(Duration::ZERO)?;
    assert_eq!(*seen.borrow(), [1, 2, 4]);
    runtime.unmount(&id)?;
    b.set(5)?;
    assert_eq!(runtime.next_deadline(), None);
    Ok(())
}

#[test]
fn hidden_live_views_sleep_and_return_with_current_data() -> TestResult {
    let mut runtime = Runtime::new();
    let signal = runtime.signal(1);
    let (component, seen) = probe(&signal, true);
    let id = runtime.mount("live", component)?;
    runtime.flush(Duration::ZERO)?;
    let deadline = runtime.next_deadline().ok_or("missing live deadline")?;
    assert_eq!(deadline, Duration::from_nanos(16_666_667));
    runtime.set_visible(&id, false)?;
    for i in 2..100 {
        signal.set(i)?;
    }
    assert_eq!(
        runtime.flush(Duration::from_secs(30))?.components_updated,
        0
    );
    assert_eq!(runtime.next_deadline(), None);
    assert_eq!(seen.borrow().len(), 1);
    runtime.set_visible(&id, true)?;
    assert_eq!(
        runtime.flush(Duration::from_secs(30))?.components_updated,
        1
    );
    assert_eq!(seen.borrow().last(), Some(&(99, Invalidation::Layout)));
    assert_eq!(
        runtime.next_deadline(),
        Some(Duration::from_secs(30) + deadline)
    );
    runtime.unmount(&id)?;
    assert_eq!(runtime.next_deadline(), None);
    Ok(())
}

#[test]
fn shared_live_deadlines_skip_missed_frames_without_visiting_static_components() -> TestResult {
    let mut runtime = Runtime::new();
    let signal = runtime.signal(1);
    for i in 0..1_000 {
        let (component, _) = probe(&signal, false);
        runtime.mount(format!("static-{i}"), component)?;
    }
    for i in 0..2 {
        let (component, _) = probe(&signal, true);
        runtime.mount(format!("live-{i}"), component)?;
    }
    assert_eq!(runtime.flush(Duration::ZERO)?.components_updated, 1_002);
    assert_eq!(
        runtime.flush(Duration::from_millis(1))?.components_updated,
        0
    );
    assert_eq!(
        runtime.flush(Duration::from_secs(10))?.components_updated,
        2
    );
    assert_eq!(
        runtime.flush(Duration::from_secs(10))?.components_updated,
        0
    );
    assert_eq!(
        runtime.next_deadline(),
        Some(Duration::from_secs(10) + Duration::from_nanos(16_666_667))
    );
    assert_eq!(
        runtime.flush(Duration::ZERO),
        Err(RuntimeError::TimeReversed)
    );
    Ok(())
}

#[test]
fn keys_and_handles_never_retarget_after_remount_or_across_runtimes() -> TestResult {
    let mut runtime = Runtime::new();
    let signal = runtime.signal(1);
    let (component, _) = probe(&signal, false);
    let old = runtime.mount("same", component)?;
    let (duplicate, _) = probe(&signal, false);
    assert_eq!(
        runtime.mount("same", duplicate),
        Err(RuntimeError::DuplicateKey)
    );
    runtime.unmount(&old)?;
    let (component, _) = probe(&signal, false);
    let new = runtime.mount("same", component)?;
    assert_ne!(old, new);
    assert_eq!(runtime.event(&old, |_| ()), Err(RuntimeError::Unmounted));
    let foreign = Runtime::new();
    assert_eq!(
        foreign.invalidate(&new, Invalidation::Paint),
        Err(RuntimeError::ForeignRuntime)
    );
    runtime.set_visible(&new, false)?;
    assert_eq!(runtime.event(&new, |_| ()), Err(RuntimeError::Hidden));
    Ok(())
}

struct WritesDuringUpdate {
    signal: Signal<i32>,
    observed: Rc<Cell<Option<RuntimeError>>>,
}
impl Component for WritesDuringUpdate {
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        self.observed.set(self.signal.set(2).err());
        Ok(())
    }
}
#[test]
fn writes_are_rejected_during_update_and_handles_close_with_runtime() -> TestResult {
    let mut runtime = Runtime::new();
    let signal = runtime.signal(1);
    let observed = Rc::new(Cell::new(None));
    runtime.mount(
        "writer",
        WritesDuringUpdate {
            signal: signal.clone(),
            observed: observed.clone(),
        },
    )?;
    runtime.flush(Duration::ZERO)?;
    assert_eq!(observed.take(), Some(RuntimeError::WriteDuringUpdate));
    assert_eq!(signal.get()?, 1);
    drop(runtime);
    assert_eq!(signal.set(3), Err(RuntimeError::Closed));
    assert_eq!(signal.get(), Err(RuntimeError::Closed));
    Ok(())
}

struct FailsOnce {
    first: bool,
}
impl Component for FailsOnce {
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        if std::mem::replace(&mut self.first, false) {
            Err(RuntimeError::BorrowConflict)
        } else {
            Ok(())
        }
    }
}
#[test]
fn component_errors_preserve_pending_work_and_restore_write_phase() -> TestResult {
    let mut runtime = Runtime::new();
    let signal = runtime.signal(1);
    runtime.mount("failing", FailsOnce { first: true })?;
    let (component, _) = probe(&signal, false);
    runtime.mount("other", component)?;
    assert_eq!(
        runtime.flush(Duration::ZERO),
        Err(RuntimeError::BorrowConflict)
    );
    signal.set(2)?;
    assert_eq!(runtime.flush(Duration::ZERO)?.components_updated, 2);
    Ok(())
}

struct ParameterProbe {
    value: ReadSignal<pert_ui_runtime::ParameterSnapshot<f32>>,
    seen: Rc<RefCell<Vec<pert_ui_runtime::ParameterSnapshot<f32>>>>,
}
impl Component for ParameterProbe {
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        self.seen.borrow_mut().push(cx.read_paint(&self.value)?);
        Ok(())
    }
}
#[test]
fn shared_parameter_controls_observe_preview_conflict_and_effective_independently() -> TestResult {
    let mut runtime = Runtime::new();
    let (parameter, host) = runtime.parameter(
        ParameterId::new(1, 1, "x")?,
        ParameterSpec::new("X", 0.0, 10.0, 2.0)?,
    )?;
    let seen_a = Rc::new(RefCell::new(Vec::new()));
    let seen_b = Rc::new(RefCell::new(Vec::new()));
    let a = runtime.mount(
        "a",
        ParameterProbe {
            value: parameter.snapshot(),
            seen: seen_a.clone(),
        },
    )?;
    runtime.mount(
        "b",
        ParameterProbe {
            value: parameter.snapshot(),
            seen: seen_b.clone(),
        },
    )?;
    runtime.flush(Duration::ZERO)?;
    let revision = parameter.snapshot().get()?.revision;
    let edit = runtime.event(&a, |cx| parameter.begin_edit(cx, revision))??;
    edit.update(4.0)?;
    host.set_effective(Some(20.0))?;
    assert_eq!(runtime.flush(Duration::ZERO)?.components_updated, 2);
    assert_eq!(seen_a.borrow().last(), seen_b.borrow().last());
    let snapshot = parameter.snapshot().get()?;
    assert_eq!(
        (snapshot.base, snapshot.preview, snapshot.effective),
        (2.0, Some(4.0), 20.0)
    );
    host.write_base(7.0)?;
    assert_eq!(
        edit.commit(5.0)?,
        EditOutcome::Rejected(EditError::Conflict)
    );
    assert_eq!(parameter.snapshot().get()?.base, 7.0);
    Ok(())
}

#[test]
fn hiding_unmount_focus_loss_escape_and_guard_drop_cancel_gestures() -> TestResult {
    for cause in 0..6 {
        let mut runtime = Runtime::new();
        let (parameter, _host) = runtime.parameter(
            ParameterId::new(1, 1, "x")?,
            ParameterSpec::new("X", 0.0, 10.0, 2.0)?,
        )?;
        let dummy = runtime.signal(0);
        let (component, _) = probe(&dummy, false);
        let id = runtime.mount("edit", component)?;
        let edit = runtime.event(&id, |cx| {
            parameter.begin_edit(cx, parameter.snapshot().get()?.revision)
        })??;
        edit.update(4.0)?;
        match cause {
            0 => runtime.set_visible(&id, false)?,
            1 => runtime.unmount(&id)?,
            2 => runtime.clear_focus(),
            3 => {
                runtime.key_input(KeyCode::Escape, KeyAction::Press)?;
            }
            4 => {
                drop(runtime);
                assert_eq!(edit.outcome()?, Some(EditOutcome::Cancelled));
                continue;
            }
            _ => {
                drop(edit);
                assert_eq!(parameter.snapshot().get()?.preview, None);
                continue;
            }
        }
        assert_eq!(edit.outcome()?, Some(EditOutcome::Cancelled));
        assert_eq!(
            (
                parameter.snapshot().get()?.base,
                parameter.snapshot().get()?.preview
            ),
            (2.0, None)
        );
    }
    Ok(())
}

#[test]
fn host_drop_marks_unavailable_and_recreation_requires_new_identity() -> TestResult {
    let mut runtime = Runtime::new();
    let id = ParameterId::new(1, 1, "x")?;
    let spec = ParameterSpec::new("X", 0.0, 1.0, 0.0)?;
    let (parameter, host) = runtime.parameter(id.clone(), spec.clone())?;
    let dummy = runtime.signal(0);
    let (component, _) = probe(&dummy, false);
    let widget = runtime.mount("edit", component)?;
    let edit = runtime.event(&widget, |cx| {
        parameter.begin_edit(cx, parameter.snapshot().get()?.revision)
    })??;
    drop(host);
    assert!(!parameter.snapshot().get()?.available);
    assert_eq!(
        edit.outcome()?,
        Some(EditOutcome::Rejected(EditError::Unavailable))
    );
    assert!(matches!(
        runtime.parameter(id.clone(), spec.clone()),
        Err(RuntimeError::Binding(BindingError::DuplicateParameter))
    ));
    let (replacement, _owner) = runtime.parameter(id.next_incarnation()?, spec)?;
    assert!(replacement.snapshot().get()?.available);
    assert_eq!(
        edit.commit(0.5)?,
        EditOutcome::Rejected(EditError::Unavailable)
    );
    assert_eq!(replacement.snapshot().get()?.base, 0.0);
    Ok(())
}

#[derive(Default)]
struct InputLog {
    events: Vec<(KeyEvent, usize)>,
    text: String,
    resets: usize,
}
struct InputProbe(Rc<RefCell<InputLog>>);
impl Component for InputProbe {
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
    fn key_input(
        &mut self,
        _: &mut EventContext,
        event: KeyEvent,
        keys: &KeyboardState,
    ) -> Result<(), RuntimeError> {
        self.0
            .borrow_mut()
            .events
            .push((event, keys.pressed().count()));
        Ok(())
    }
    fn text_input(&mut self, _: &mut EventContext, text: &str) -> Result<(), RuntimeError> {
        self.0.borrow_mut().text.push_str(text);
        Ok(())
    }
    fn input_reset(&mut self) {
        self.0.borrow_mut().resets += 1;
    }
}

#[test]
fn keyboard_tracks_simultaneous_keys_repeat_modifiers_and_text_separately() -> TestResult {
    let mut runtime = Runtime::new();
    let log = Rc::new(RefCell::new(InputLog::default()));
    let id = runtime.mount("input", InputProbe(log.clone()))?;
    runtime.focus(&id)?;
    for key in [
        KeyCode::KeyA,
        KeyCode::KeyS,
        KeyCode::KeyD,
        KeyCode::KeyF,
        KeyCode::KeyG,
        KeyCode::KeyH,
        KeyCode::KeyJ,
        KeyCode::KeyK,
        KeyCode::ShiftLeft,
        KeyCode::ShiftRight,
    ] {
        assert_eq!(
            runtime.key_input(key, KeyAction::Press)?.transition,
            KeyTransition::Pressed
        );
    }
    assert_eq!(runtime.keyboard().pressed().count(), 10);
    assert_eq!(
        runtime
            .key_input(KeyCode::KeyA, KeyAction::Press)?
            .transition,
        KeyTransition::Repeated
    );
    assert_eq!(runtime.keyboard().pressed().count(), 10);
    runtime.key_input(KeyCode::ShiftLeft, KeyAction::Release)?;
    assert!(runtime.keyboard().modifiers().shift);
    runtime.key_input(KeyCode::ShiftRight, KeyAction::Release)?;
    assert!(!runtime.keyboard().modifiers().shift);
    runtime.text_input("åΩ🎹")?;
    assert_eq!(log.borrow().text, "åΩ🎹");
    assert_eq!(runtime.keyboard().pressed().count(), 8);
    runtime.clear_focus();
    assert_eq!(log.borrow().resets, 1);
    assert_eq!(runtime.keyboard().pressed().count(), 0);
    assert_eq!(
        runtime
            .key_input(KeyCode::KeyA, KeyAction::Release)?
            .transition,
        KeyTransition::UnmatchedRelease
    );
    Ok(())
}

#[test]
fn focus_transfer_and_hiding_reset_component_owned_key_actions() -> TestResult {
    let mut runtime = Runtime::new();
    let first = Rc::new(RefCell::new(InputLog::default()));
    let second = Rc::new(RefCell::new(InputLog::default()));
    let a = runtime.mount("a", InputProbe(first.clone()))?;
    let b = runtime.mount("b", InputProbe(second.clone()))?;
    runtime.focus(&a)?;
    runtime.key_input(KeyCode::KeyA, KeyAction::Press)?;
    runtime.focus(&b)?;
    assert_eq!(first.borrow().resets, 1);
    assert!(runtime.keyboard().is_pressed(KeyCode::KeyA));
    runtime.set_visible(&b, false)?;
    assert_eq!(second.borrow().resets, 1);
    runtime.key_input(KeyCode::KeyA, KeyAction::Release)?;
    assert!(second.borrow().events.is_empty());
    Ok(())
}

#[test]
fn gesture_drop_reclaims_receipts_and_commits_remain_committed() -> TestResult {
    let mut runtime = Runtime::new();
    let (parameter, _owner) = runtime.parameter(
        ParameterId::new(1, 1, "x")?,
        ParameterSpec::new("X", 0.0, 1.0, 0.0)?,
    )?;
    let dummy = runtime.signal(0);
    let (component, _) = probe(&dummy, false);
    let id = runtime.mount("control", component)?;
    for _ in 0..100 {
        let revision = parameter.snapshot().get()?.revision;
        let edit = runtime.event(&id, |cx| parameter.begin_edit(cx, revision))??;
        assert!(parameter.snapshot().get()?.edit_source.is_some());
        let outcome = edit.commit(1.0)?;
        assert_eq!(edit.commit(0.0)?, outcome);
        drop(edit);
        assert_eq!(parameter.snapshot().get()?.base, 1.0);
        assert_eq!(parameter.snapshot().get()?.edit_source, None);
    }
    assert_eq!(parameter.snapshot().get()?.revision.get(), 100);
    Ok(())
}

struct PanicOnce(bool);
impl Component for PanicOnce {
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        assert!(!std::mem::replace(&mut self.0, false), "component panic");
        Ok(())
    }
}
#[test]
fn unwinding_an_update_preserves_work_and_restores_phase() -> TestResult {
    let mut runtime = Runtime::new();
    let signal = runtime.signal(0);
    runtime.mount("panic", PanicOnce(true))?;
    assert!(
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(
            || runtime.flush(Duration::ZERO)
        ))
        .is_err()
    );
    signal.set(1)?;
    assert_eq!(runtime.flush(Duration::ZERO)?.components_updated, 1);
    Ok(())
}

struct MultiRead {
    value: ReadSignal<i32>,
    seen: Rc<Cell<Option<Invalidation>>>,
}
impl Component for MultiRead {
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        cx.read_paint(&self.value)?;
        cx.read(&self.value)?;
        self.seen.set(Some(cx.invalidation()));
        Ok(())
    }
}
#[test]
fn shared_reads_keep_the_strongest_dependency_kind() -> TestResult {
    let mut runtime = Runtime::new();
    let signal = runtime.signal(0);
    let seen = Rc::new(Cell::new(None));
    let id = runtime.mount(
        "read",
        MultiRead {
            value: signal.read_only(),
            seen: seen.clone(),
        },
    )?;
    runtime.flush(Duration::ZERO)?;
    for value in 1..3 {
        signal.set(value)?;
        runtime.flush(Duration::ZERO)?;
        assert_eq!(seen.get(), Some(Invalidation::Layout));
    }
    runtime.unmount(&id)?;
    signal.set(3)?;
    assert_eq!(runtime.next_deadline(), None);
    Ok(())
}

struct BadMount;
impl Component for BadMount {
    fn mounted(&mut self) {
        panic!("mount failed");
    }
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
}
#[test]
fn failed_mount_does_not_reserve_its_key_and_focus_reset_repaints() -> TestResult {
    let mut runtime = Runtime::new();
    assert!(
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(
            || runtime.mount("key", BadMount)
        ))
        .is_err()
    );
    let signal = runtime.signal(0);
    let (component, seen) = probe(&signal, false);
    let id = runtime.mount("key", component)?;
    runtime.flush(Duration::ZERO)?;
    runtime.focus(&id)?;
    runtime.flush(Duration::ZERO)?;
    runtime.clear_focus();
    assert_eq!(runtime.flush(Duration::ZERO)?.components_updated, 1);
    assert_eq!(seen.borrow().last(), Some(&(0, Invalidation::Paint)));
    Ok(())
}

/// A held repeat driven by an event-phase timer: the press saves one step and
/// every elapsed timer saves another without waiting for release.
struct Repeater {
    value: Signal<i32>,
    delay: Duration,
    interval: Duration,
    ticks: Rc<Cell<usize>>,
}
impl Component for Repeater {
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
    fn keyboard_event(
        &mut self,
        cx: &mut EventContext,
        event: KeyEvent,
        _: &KeyboardState,
    ) -> Result<InputOutcome, RuntimeError> {
        match event.transition {
            KeyTransition::Pressed => {
                self.value.set(self.value.get()? + 1)?;
                cx.set_timer(self.delay)?;
            }
            KeyTransition::Released | KeyTransition::UnmatchedRelease => cx.cancel_timer(),
            KeyTransition::Repeated => {}
        }
        Ok(InputOutcome::Handled)
    }
    fn timer_elapsed(&mut self, cx: &mut EventContext) -> Result<(), RuntimeError> {
        self.ticks.set(self.ticks.get() + 1);
        self.value.set(self.value.get()? + 1)?;
        cx.set_timer(self.interval)
    }
}

#[test]
fn event_phase_timers_commit_held_repeats_and_stop_with_their_input_context() -> TestResult {
    let mut runtime = Runtime::new();
    let value = runtime.signal(0);
    let ticks = Rc::new(Cell::new(0));
    let id = runtime.mount(
        "repeater",
        Repeater {
            value: value.clone(),
            delay: Duration::from_millis(400),
            interval: Duration::from_millis(80),
            ticks: ticks.clone(),
        },
    )?;
    runtime.flush(Duration::ZERO)?;
    runtime.focus(&id)?;
    runtime.flush(Duration::ZERO)?;
    assert_eq!(runtime.next_deadline(), None, "an idle component sleeps");
    runtime.key_input(KeyCode::ArrowUp, KeyAction::Press)?;
    assert_eq!(value.get()?, 1, "the press saves one step immediately");
    assert_eq!(
        runtime.inspect().components[0].next_deadline,
        Some(Duration::from_millis(400)),
        "inspection reports the pending timer"
    );
    runtime.flush(Duration::from_millis(399))?;
    assert_eq!(value.get()?, 1, "no repeat before the delay");
    assert_eq!(
        runtime.next_deadline(),
        Some(Duration::from_millis(400)),
        "the runtime wakes for the timer, not on a frame ceiling"
    );
    // A missed deadline produces one call, not a catch-up burst.
    runtime.flush(Duration::from_millis(600))?;
    assert_eq!(ticks.get(), 1);
    assert_eq!(value.get()?, 2, "the repeat committed without a release");
    assert_eq!(
        runtime.inspect().components[0].next_deadline,
        Some(Duration::from_millis(680))
    );
    runtime.flush(Duration::from_millis(680))?;
    assert_eq!(value.get()?, 3);
    runtime.key_input(KeyCode::ArrowUp, KeyAction::Release)?;
    assert_eq!(
        runtime.inspect().components[0].next_deadline,
        None,
        "release cancels the timer"
    );
    runtime.flush(Duration::from_secs(2))?;
    assert_eq!(value.get()?, 3);
    assert_eq!(runtime.next_deadline(), None);
    // Hiding cancels a pending timer with the rest of the input context.
    runtime.key_input(KeyCode::ArrowUp, KeyAction::Press)?;
    assert_eq!(
        runtime.inspect().components[0].next_deadline,
        Some(Duration::from_millis(2400))
    );
    runtime.set_visible(&id, false)?;
    assert_eq!(
        runtime.inspect().components[0].next_deadline,
        None,
        "hiding cancels the timer"
    );
    runtime.flush(Duration::from_secs(3))?;
    assert_eq!(value.get()?, 4, "only the press was saved");
    assert_eq!(ticks.get(), 2);
    Ok(())
}

#[test]
fn timer_requests_reject_zero_delays_and_report_callback_failures() -> TestResult {
    struct Failing {
        armed: bool,
    }
    impl Component for Failing {
        fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
            Ok(())
        }
        fn keyboard_event(
            &mut self,
            cx: &mut EventContext,
            _: KeyEvent,
            _: &KeyboardState,
        ) -> Result<InputOutcome, RuntimeError> {
            assert_eq!(cx.set_timer(Duration::ZERO), Err(RuntimeError::ZeroDelay));
            cx.set_timer(Duration::from_millis(10))?;
            self.armed = true;
            Ok(InputOutcome::Handled)
        }
        fn timer_elapsed(&mut self, _: &mut EventContext) -> Result<(), RuntimeError> {
            Err(RuntimeError::Unmounted)
        }
    }
    let mut runtime = Runtime::new();
    let id = runtime.mount("failing", Failing { armed: false })?;
    runtime.flush(Duration::ZERO)?;
    runtime.focus(&id)?;
    runtime.key_input(KeyCode::KeyA, KeyAction::Press)?;
    assert_eq!(
        runtime.flush(Duration::from_millis(10)),
        Err(RuntimeError::Unmounted)
    );
    assert_eq!(
        runtime.inspect().components[0].next_deadline,
        None,
        "a failed timer is not rearmed automatically"
    );
    runtime.flush(Duration::from_millis(20))?;
    Ok(())
}
