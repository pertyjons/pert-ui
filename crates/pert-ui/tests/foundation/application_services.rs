//! Application services through the facade: values kept until shutdown,
//! background work delivered on the UI thread, and interval timers.
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
