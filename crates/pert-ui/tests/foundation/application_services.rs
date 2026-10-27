//! Application services through the facade: values kept until shutdown,
//! background work delivered on the UI thread, and interval timers.
use pert_ui::testing::HostIntegration;
use pert_ui::*;
use std::{
    cell::RefCell,
    rc::Rc,
    sync::mpsc,
    time::{Duration, Instant},
};

type TestResult = Result<(), Box<dyn std::error::Error>>;
type Log = Rc<RefCell<Vec<&'static str>>>;

/// Records its name when dropped.
struct Noted(&'static str, Log);
impl Drop for Noted {
    fn drop(&mut self) {
        self.1.borrow_mut().push(self.0);
    }
}

/// A component that records its name when unmounted, by dropping its note.
struct Mounted {
    _note: Noted,
}
impl Component for Mounted {
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
}

#[test]
fn kept_values_drop_at_shutdown_after_components_and_last_kept_first() -> TestResult {
    let log: Log = Rc::default();
    let mut ui = Runtime::new();
    ui.mount_tree(
        Mounted {
            _note: Noted("component", log.clone()),
        }
        .key("component"),
    )?;
    ui.keep_alive(Noted("first", log.clone()));
    ui.keep_alive(Noted("second", log.clone()));
    ui.flush(Duration::ZERO)?;
    assert!(log.borrow().is_empty());
    ui.shutdown();
    assert_eq!(*log.borrow(), ["component", "second", "first"]);
    ui.keep_alive(Noted("late", log.clone()));
    assert_eq!(
        log.borrow().last(),
        Some(&"late"),
        "after shutdown it drops at once"
    );
    Ok(())
}

/// Flushes until `done` holds or a generous deadline passes.
fn flush_until(ui: &mut Runtime, mut done: impl FnMut() -> bool) -> Result<(), RuntimeError> {
    let start = Instant::now();
    while !done() && start.elapsed() < Duration::from_secs(10) {
        std::thread::sleep(Duration::from_millis(1));
        ui.flush(ui.now())?;
    }
    Ok(())
}

#[test]
fn background_results_are_delivered_on_the_ui_thread() -> TestResult {
    let mut ui = Runtime::new();
    let total = ui.signal(0_u64);
    let shown = total.clone();
    ui.spawn(|| (1..=100_u64).sum::<u64>(), move |sum| shown.set(sum))?;
    flush_until(&mut ui, || total.get().is_ok_and(|total| total != 0))?;
    assert_eq!(total.get()?, 5050);
    Ok(())
}

#[test]
fn a_cancelled_result_is_discarded_and_a_dropped_handle_is_not_a_cancellation() -> TestResult {
    let mut ui = Runtime::new();
    let delivered = ui.signal(Vec::<u32>::new());
    let (a, b) = (delivered.clone(), delivered.clone());
    let (release, gate) = mpsc::channel::<()>();
    let cancelled = ui.spawn(
        move || gate.recv().map(|()| 1_u32).unwrap_or(0),
        move |value| a.update(|values| values.push(value)),
    )?;
    drop(ui.spawn(|| 2_u32, move |value| b.update(|values| values.push(value)))?);
    cancelled.cancel();
    release.send(())?;
    flush_until(&mut ui, || {
        delivered.get().is_ok_and(|values| !values.is_empty())
    })?;
    // Give the cancelled result time to arrive too.
    std::thread::sleep(Duration::from_millis(20));
    ui.flush(ui.now())?;
    assert_eq!(delivered.get()?, [2]);
    Ok(())
}

#[test]
fn a_panicking_work_reports_an_error_and_delivers_nothing() -> TestResult {
    let mut ui = Runtime::new();
    let delivered = ui.signal(false);
    let flag = delivered.clone();
    ui.spawn(
        || -> u32 { panic!("deliberate panic in background work") },
        move |_| flag.set(true),
    )?;
    let start = Instant::now();
    let error = loop {
        std::thread::sleep(Duration::from_millis(1));
        match ui.flush(ui.now()) {
            Err(error) => break Some(error),
            Ok(_) if start.elapsed() > Duration::from_secs(10) => break None,
            Ok(_) => {}
        }
    };
    assert_eq!(error, Some(RuntimeError::BackgroundPanicked));
    assert!(!delivered.get()?);
    Ok(())
}

#[test]
fn background_work_is_bounded_and_ends_with_shutdown() -> TestResult {
    let mut ui = Runtime::new();
    let (release, gate) = mpsc::channel::<()>();
    let gate = std::sync::Arc::new(std::sync::Mutex::new(gate));
    let delivered = ui.signal(0);
    for _ in 0..Runtime::MAX_BACKGROUND_TASKS {
        let (gate, shown) = (gate.clone(), delivered.clone());
        ui.spawn(
            move || gate.lock().map(|gate| gate.recv().is_ok()).unwrap_or(false),
            move |_| shown.update(|n| *n += 1),
        )?;
    }
    let refused = ui.spawn(|| (), |()| Ok(()));
    assert_eq!(refused.err(), Some(RuntimeError::BackgroundFull));
    ui.shutdown();
    for _ in 0..Runtime::MAX_BACKGROUND_TASKS {
        release.send(())?;
    }
    assert_eq!(
        delivered.get().unwrap_or(0),
        0,
        "nothing is delivered after shutdown"
    );
    assert_eq!(
        ui.spawn(|| (), |()| Ok(())).err(),
        Some(RuntimeError::Closed)
    );
    Ok(())
}

#[test]
fn an_interval_ticks_on_time_without_catching_up() -> TestResult {
    let mut ui = Runtime::new();
    let ticks = ui.signal(Vec::<Duration>::new());
    let log = ticks.clone();
    ui.start_interval(Duration::from_secs(10), move |ui| {
        let now = ui.now();
        log.update(|ticks| ticks.push(now))
    })?;
    assert_eq!(ui.next_deadline(), Some(Duration::from_secs(10)));
    ui.flush(Duration::from_secs(10))?;
    // Three periods late: one tick, and the next is one period after it.
    ui.flush(Duration::from_secs(45))?;
    assert_eq!(ui.next_deadline(), Some(Duration::from_secs(55)));
    ui.flush(Duration::from_secs(55))?;
    assert_eq!(
        ticks.get()?,
        [
            Duration::from_secs(10),
            Duration::from_secs(45),
            Duration::from_secs(55)
        ]
    );
    Ok(())
}

#[test]
fn a_failing_tick_keeps_its_timer_and_a_tick_can_cancel_it() -> TestResult {
    let mut ui = Runtime::new();
    let count = ui.signal(0);
    let counter = count.clone();
    let failing = ui.start_interval(Duration::from_secs(1), move |_| {
        counter.update(|n| *n += 1)?;
        Err(RuntimeError::Exhausted)
    })?;
    assert_eq!(
        ui.flush(Duration::from_secs(1)).err(),
        Some(RuntimeError::Exhausted)
    );
    assert_eq!(
        ui.flush(Duration::from_secs(2)).err(),
        Some(RuntimeError::Exhausted)
    );
    assert_eq!(count.get()?, 2);
    failing.cancel();
    assert!(failing.is_cancelled());
    ui.flush(Duration::from_secs(3))?;
    assert_eq!(count.get()?, 2);
    let once = ui.signal(0);
    let (counter, handle) = (once.clone(), Rc::new(RefCell::new(None::<IntervalTimer>)));
    let own = handle.clone();
    let timer = ui.start_interval(Duration::from_secs(1), move |_| {
        if let Some(timer) = own.borrow().as_ref() {
            timer.cancel();
        }
        counter.update(|n| *n += 1)
    })?;
    *handle.borrow_mut() = Some(timer);
    ui.flush(Duration::from_secs(4))?;
    ui.flush(Duration::from_secs(5))?;
    assert_eq!(once.get()?, 1);
    assert_eq!(ui.next_deadline(), None);
    Ok(())
}

#[test]
fn intervals_run_with_every_window_hidden_and_refuse_bad_requests() -> TestResult {
    let mut ui = Runtime::new();
    let count = ui.signal(0);
    let counter = count.clone();
    ui.start_interval(Duration::from_secs(1), move |_| counter.update(|n| *n += 1))?;
    ui.set_window_visible(false)?;
    ui.flush_windows(Duration::from_secs(1))?;
    assert_eq!(count.get()?, 1);
    ui.service_intervals(Duration::from_secs(2))?;
    assert_eq!(count.get()?, 2);
    assert_eq!(
        ui.start_interval(Duration::ZERO, |_| Ok(())).err(),
        Some(RuntimeError::ZeroDelay)
    );
    assert_eq!(
        ui.service_intervals(Duration::from_secs(1)).err(),
        Some(RuntimeError::TimeReversed)
    );
    let mut timers = Vec::new();
    for _ in 1..Runtime::MAX_INTERVALS {
        timers.push(ui.start_interval(Duration::from_secs(60), |_| Ok(()))?);
    }
    assert_eq!(
        ui.start_interval(Duration::from_secs(60), |_| Ok(())).err(),
        Some(RuntimeError::IntervalsFull)
    );
    timers[0].cancel();
    assert!(
        ui.start_interval(Duration::from_secs(60), |_| Ok(()))
            .is_ok(),
        "a cancelled timer frees its place"
    );
    Ok(())
}

#[test]
fn a_timer_run_by_a_nested_flush_does_not_tick_twice() -> TestResult {
    let mut ui = Runtime::new();
    let ticks = ui.signal(0);
    // The first timer flushes from its tick, which runs the second one.
    ui.start_interval(Duration::from_secs(1), |ui| ui.flush(ui.now()).map(|_| ()))?;
    let counter = ticks.clone();
    ui.start_interval(Duration::from_secs(1), move |_| counter.update(|n| *n += 1))?;
    ui.flush(Duration::from_secs(1))?;
    assert_eq!(ticks.get()?, 1);
    Ok(())
}

#[test]
fn a_running_tick_still_counts_toward_the_timer_limit() -> TestResult {
    let mut ui = Runtime::new();
    for _ in 1..Runtime::MAX_INTERVALS {
        ui.start_interval(Duration::from_secs(60), |_| Ok(()))?;
    }
    let refused = Rc::new(RefCell::new(None));
    let outcome = refused.clone();
    ui.start_interval(Duration::from_secs(1), move |ui| {
        *outcome.borrow_mut() = Some(ui.start_interval(Duration::from_secs(60), |_| Ok(())).err());
        Ok(())
    })?;
    ui.flush(Duration::from_secs(1))?;
    assert_eq!(*refused.borrow(), Some(Some(RuntimeError::IntervalsFull)));
    Ok(())
}

#[test]
fn an_interval_error_names_no_component_from_an_earlier_failure() -> TestResult {
    let mut ui = Runtime::new();
    struct Broken;
    impl Component for Broken {
        fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
            Err(RuntimeError::Exhausted)
        }
    }
    let broken = ui.mount_tree(Broken.key("broken"))?;
    assert!(ui.flush(Duration::ZERO).is_err());
    assert!(ui.last_failure().is_some());
    ui.unmount(&broken)?;
    ui.start_interval(Duration::from_secs(1), |_| Err(RuntimeError::ZeroDelay))?;
    assert_eq!(
        ui.service_intervals(Duration::from_secs(1)).err(),
        Some(RuntimeError::ZeroDelay)
    );
    assert_eq!(ui.last_failure(), None);
    Ok(())
}

#[test]
fn an_interval_survives_focus_loss_and_cancelled_input_and_stops_at_shutdown() -> TestResult {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(200.0, 100.0, 1.0)?)?;
    let button = ui.mount_tree(Button::new("Focus", |_| Ok(())).key("button"))?;
    ui.flush(Duration::ZERO)?;
    ui.focus(&button)?;
    // Counted outside the runtime, so shutdown cannot hide a tick.
    let count = Rc::new(std::cell::Cell::new(0));
    let counter = count.clone();
    ui.start_interval(Duration::from_secs(1), move |_| {
        counter.set(counter.get() + 1);
        Ok(())
    })?;
    // What a native window losing focus does, and an explicit cancellation:
    // both end component input, not the application's timers.
    ui.clear_focus();
    ui.cancel_input()?;
    ui.flush(Duration::from_secs(1))?;
    assert_eq!(count.get(), 1);
    ui.cancel_input()?;
    ui.flush(Duration::from_secs(2))?;
    assert_eq!(count.get(), 2);
    ui.shutdown();
    assert_eq!(
        ui.service_intervals(Duration::from_secs(3)).err(),
        Some(RuntimeError::Closed)
    );
    assert_eq!(
        ui.flush(Duration::from_secs(4)).err(),
        Some(RuntimeError::Closed)
    );
    assert_eq!(count.get(), 2, "no tick during or after shutdown");
    Ok(())
}

/// The receipt of the last start a button's callback queued.
type KeptReceipt = Rc<RefCell<Option<TaskReceipt>>>;

/// A button whose callback counts words with `EventContext::spawn`, keeping
/// the receipt, and the signal the count reaches.
fn spawning_button(
    ui: &mut Runtime,
) -> Result<(WidgetId, Signal<usize>, KeptReceipt), RuntimeError> {
    ui.set_viewport(Viewport::new(200.0, 100.0, 1.0)?)?;
    let words = ui.signal(0_usize);
    let receipt: KeptReceipt = Rc::default();
    let (shown, kept) = (words.clone(), receipt.clone());
    let button = ui.mount_tree(
        Button::new("Count", move |cx| {
            let shown = shown.clone();
            *kept.borrow_mut() = Some(cx.spawn(
                || "one two three".split_whitespace().count(),
                move |count| shown.set(count),
            )?);
            Ok(())
        })
        .key("count"),
    )?;
    ui.flush(Duration::ZERO)?;
    Ok((button, words, receipt))
}

fn receipt_status(receipt: &KeptReceipt) -> Option<TaskStatus> {
    receipt.borrow().as_ref().map(TaskReceipt::status)
}

#[test]
fn a_callback_spawn_starts_at_the_next_flush_and_delivers_its_result() -> TestResult {
    let mut ui = Runtime::new();
    let (button, words, receipt) = spawning_button(&mut ui)?;
    ui.focus(&button)?;
    ui.key_input(KeyCode::Space, KeyAction::Press)?;
    ui.key_input(KeyCode::Space, KeyAction::Release)?;
    // Queued in the callback; the thread starts at the next flush.
    ui.flush(ui.now())?;
    assert_eq!(receipt_status(&receipt), Some(TaskStatus::Succeeded));
    flush_until(&mut ui, || words.get().is_ok_and(|words| words != 0))?;
    assert_eq!(words.get()?, 3);
    Ok(())
}

#[test]
fn a_refused_callback_spawn_fails_its_receipt_and_the_flush() -> TestResult {
    let mut ui = Runtime::new();
    let (button, words, _) = spawning_button(&mut ui)?;
    let (release, gate) = mpsc::channel::<()>();
    let gate = std::sync::Arc::new(std::sync::Mutex::new(gate));
    for _ in 0..Runtime::MAX_BACKGROUND_TASKS {
        let gate = gate.clone();
        ui.spawn(
            move || gate.lock().map(|gate| gate.recv().is_ok()).unwrap_or(false),
            |_| Ok(()),
        )?;
    }
    let receipt = ui.event(&button, |cx| cx.spawn(|| 1_usize, |_| Ok(())))??;
    assert_eq!(ui.flush(ui.now()).err(), Some(RuntimeError::BackgroundFull));
    assert_eq!(
        receipt.status(),
        TaskStatus::Failed(RuntimeError::BackgroundFull)
    );
    for _ in 0..Runtime::MAX_BACKGROUND_TASKS {
        release.send(())?;
    }
    assert_eq!(words.get()?, 0);
    Ok(())
}

#[test]
fn a_callback_spawn_outlives_its_widget_and_its_window() -> TestResult {
    let mut ui = Runtime::new();
    let (button, words, _) = spawning_button(&mut ui)?;
    let shown = words.clone();
    let removed = ui.event(&button, |cx| {
        cx.spawn(|| 2_usize, move |count| shown.set(count))
    })??;
    // The widget goes before the flush, as a dialog closes after Save.
    ui.unmount(&button)?;
    ui.flush(ui.now())?;
    assert_eq!(removed.status(), TaskStatus::Succeeded);
    flush_until(&mut ui, || words.get().is_ok_and(|words| words != 0))?;
    assert_eq!(words.get()?, 2);

    // Hiding does not cancel the start either.
    let (button, _, _) = spawning_button(&mut ui)?;
    let hidden = ui.event(&button, |cx| cx.spawn(|| 0_usize, |_| Ok(())))??;
    ui.set_visible(&button, false)?;
    ui.flush(ui.now())?;
    assert_eq!(hidden.status(), TaskStatus::Succeeded);

    // New-behavior test 6 of layering part L4a: a window that closes with its
    // widget before the flush no longer cancels the start. Named assertions:
    // the receipt reads `Succeeded` and the result is delivered. Mutation:
    // `EventContext::spawn` records the former window owner.
    let view = ui.mount_tree(Element::new("view", Label::new("Detached")))?;
    ui.flush(ui.now())?;
    let options =
        NativeWindowOptions::new("Detached")?.close_policy(DetachedClosePolicy::RemoveView);
    let prepared = ui.prepare_detach(&view, options)?;
    let detached = ui.commit_detach(prepared)?;
    ui.flush(ui.now())?;
    let delivered = ui.signal(0_usize);
    let target = delivered.clone();
    let closed = ui.event(&view, |cx| {
        cx.spawn(|| 5_usize, move |count| target.set(count))
    })??;
    ui.select_window(&detached)?;
    assert_eq!(ui.close_detached_window()?, PresentationStatus::Closed);
    assert!(!ui.contains_window(&detached));
    ui.select_window(&ui.primary_window())?;
    ui.flush(ui.now())?;
    assert_eq!(closed.status(), TaskStatus::Succeeded);
    flush_until(&mut ui, || delivered.get().is_ok_and(|count| count != 0))?;
    assert_eq!(delivered.get()?, 5);
    Ok(())
}

#[test]
fn a_callback_spawn_is_cancelled_by_shutdown_before_the_flush() -> TestResult {
    let mut ui = Runtime::new();
    let (button, _, _) = spawning_button(&mut ui)?;
    let receipt = ui.event(&button, |cx| {
        cx.spawn(
            || -> usize { panic!("shutdown must cancel the start") },
            |_| Ok(()),
        )
    })??;
    assert_eq!(receipt.status(), TaskStatus::Pending);
    ui.shutdown();
    assert_eq!(receipt.status(), TaskStatus::Cancelled);
    Ok(())
}
