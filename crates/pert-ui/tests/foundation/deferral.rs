//! Deferral in one window: `EventContext::defer` is cancelled before delivery
//! only by shutdown, `EventContext::defer_while_shown` also by its component
//! being hidden or removed, and the three calls share one bounded FIFO queue.
//!
//! Two kinds of test, marked on each: **new-behavior** tests cover what only
//! the rules of layering part L4a do, and name the assertion that a compiling
//! mutation restoring the old rule makes fail; **contract** tests check rules
//! the part keeps.
use pert_ui::testing::HostIntegration;
use pert_ui::*;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::Duration,
};

type TestResult = Result<(), Box<dyn std::error::Error>>;
type Log = Rc<RefCell<Vec<&'static str>>>;

/// Two buttons in a column, laid out and flushed.
fn buttons(ui: &mut Runtime) -> Result<(WidgetId, WidgetId), Box<dyn std::error::Error>> {
    ui.set_viewport(Viewport::new(300.0, 200.0, 1.0)?)?;
    let root = ui.mount_tree(Element::new(
        "root",
        Column::new([
            Element::new("a", Button::new("A", |_| Ok(()))),
            Element::new("b", Button::new("B", |_| Ok(()))),
        ]),
    ))?;
    ui.flush(Duration::ZERO)?;
    let a = ui.find(&root, &["a"])?.ok_or("a is mounted")?;
    let b = ui.find(&root, &["b"])?.ok_or("b is mounted")?;
    Ok((a, b))
}

/// An operation that appends `entry` to `log`.
fn logging(
    log: &Log,
    entry: &'static str,
) -> impl FnOnce(&mut Runtime) -> Result<(), RuntimeError> + 'static {
    let log = log.clone();
    move |_| {
        log.borrow_mut().push(entry);
        Ok(())
    }
}

/// New-behavior test 2: hiding the widget, or its native window, before the
/// flush no longer cancels `defer`. Named assertions: both receipts read
/// `Succeeded`. Mutation: `EventContext::defer` records `Owner::Shown`.
#[test]
fn defer_runs_after_its_widget_or_its_window_is_hidden() -> TestResult {
    let mut ui = Runtime::new();
    let (a, _) = buttons(&mut ui)?;
    let ran = Rc::new(Cell::new(false));
    let flag = ran.clone();
    let task = ui.event(&a, |cx| {
        cx.defer(move |_| {
            flag.set(true);
            Ok(())
        })
    })??;
    ui.set_visible(&a, false)?;
    ui.flush(Duration::ZERO)?;
    assert_eq!(task.status(), TaskStatus::Succeeded, "hidden widget");
    assert!(ran.get());

    // A minimized window hides every root; `flush_windows` then delivers the
    // queue once itself.
    let mut ui = Runtime::new();
    let (a, _) = buttons(&mut ui)?;
    let task = ui.event(&a, |cx| cx.defer(|_| Ok(())))??;
    ui.set_window_visible(false)?;
    ui.flush_windows(Duration::ZERO)?;
    assert_eq!(task.status(), TaskStatus::Succeeded, "hidden window");
    Ok(())
}

/// Contract test: `defer_while_shown` keeps the former `defer` rule in one
/// window. It is cancelled when its widget is hidden, when its window is
/// hidden and when it is unmounted (the last also in
/// `foundation::event_tasks_outlive_their_widget_but_not_shutdown`).
/// Contract mutation: the `Shown` arm ignores visibility; the `set_visible`
/// case fails.
#[test]
fn defer_while_shown_is_cancelled_when_its_widget_or_window_is_hidden() -> TestResult {
    let mut ui = Runtime::new();
    let (a, _) = buttons(&mut ui)?;
    let hidden = ui.event(&a, |cx| cx.defer_while_shown(|_| Ok(())))??;
    ui.set_visible(&a, false)?;
    ui.flush(Duration::ZERO)?;
    assert_eq!(hidden.status(), TaskStatus::Cancelled, "set_visible");

    let mut ui = Runtime::new();
    let (a, _) = buttons(&mut ui)?;
    let minimized = ui.event(&a, |cx| cx.defer_while_shown(|_| Ok(())))??;
    ui.set_window_visible(false)?;
    ui.flush_windows(Duration::ZERO)?;
    assert_eq!(
        minimized.status(),
        TaskStatus::Cancelled,
        "set_window_visible"
    );

    let mut ui = Runtime::new();
    let (a, _) = buttons(&mut ui)?;
    let removed = ui.event(&a, |cx| cx.defer_while_shown(|_| Ok(())))??;
    ui.unmount(&a)?;
    ui.flush(Duration::ZERO)?;
    assert_eq!(removed.status(), TaskStatus::Cancelled, "unmount");
    Ok(())
}

/// The results of queuing with each `EventContext` form from `cancel_input`.
type Attempts = Rc<RefCell<Vec<Result<TaskReceipt, RuntimeError>>>>;

/// Queues one operation of each `EventContext` form whenever its input is
/// cancelled, which removal does, and keeps the latest results.
struct QueuesOnCancel(Attempts);
impl Component for QueuesOnCancel {
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
    fn cancel_input(&mut self, cx: &mut EventContext) {
        *self.0.borrow_mut() = vec![
            cx.defer(|_| Ok(())),
            cx.defer_while_shown(|_| Ok(())),
            cx.spawn(|| (), |()| Ok(())),
        ];
    }
}

/// New-behavior test 7: shutdown refuses operations queued from the cleanup
/// callbacks it runs. Named assertion: each result is
/// `Err(RuntimeError::Closed)`. Mutation: `enqueue` drops its
/// `shutting_down` check, so each is `Ok` with a receipt left `Pending`.
#[test]
fn queuing_during_shutdown_is_refused() -> TestResult {
    let mut ui = Runtime::new();
    let attempts = Attempts::default();
    ui.mount_tree(Element::new("cleanup", QueuesOnCancel(attempts.clone())))?;
    ui.flush(Duration::ZERO)?;
    attempts.borrow_mut().clear();
    ui.shutdown();
    assert_eq!(
        *attempts.borrow(),
        vec![Err(RuntimeError::Closed); 3],
        "shutdown removed the component and refused its operations"
    );
    Ok(())
}

/// Contract test: the three calls share one FIFO queue, and a cancelled
/// `defer_while_shown` operation between the others is skipped without
/// reordering them.
#[test]
fn the_three_calls_run_in_enqueue_order() -> TestResult {
    let mut ui = Runtime::new();
    let (a, b) = buttons(&mut ui)?;
    let log = Log::default();
    let first = ui.event(&a, |cx| cx.defer(logging(&log, "defer")))??;
    let second = ui.defer(logging(&log, "runtime"))?;
    let skipped = ui.event(&b, |cx| cx.defer_while_shown(logging(&log, "skipped")))??;
    let third = ui.event(&a, |cx| cx.defer_while_shown(logging(&log, "shown")))??;
    let fourth = ui.event(&b, |cx| cx.defer(logging(&log, "defer again")))??;
    let fifth = ui.defer(logging(&log, "runtime again"))?;
    ui.set_visible(&b, false)?;
    ui.flush(Duration::ZERO)?;
    assert_eq!(
        *log.borrow(),
        ["defer", "runtime", "shown", "defer again", "runtime again"]
    );
    assert_eq!(skipped.status(), TaskStatus::Cancelled);
    for receipt in [first, second, third, fourth, fifth] {
        assert_eq!(receipt.status(), TaskStatus::Succeeded);
    }
    Ok(())
}

/// Contract test: `Runtime::MAX_TASKS` counts the three calls together; the
/// next one is refused whichever call queues it; a cancelled operation keeps
/// its place until the next delivery frees it.
#[test]
fn capacity_is_shared_by_the_three_calls() -> TestResult {
    let mut ui = Runtime::new();
    let (a, _) = buttons(&mut ui)?;
    let mut receipts = Vec::new();
    for index in 0..Runtime::MAX_TASKS {
        receipts.push(match index % 3 {
            0 => ui.event(&a, |cx| cx.defer(|_| Ok(())))??,
            1 => ui.event(&a, |cx| cx.defer_while_shown(|_| Ok(())))??,
            _ => ui.defer(|_| Ok(()))?,
        });
    }
    let full = Err(RuntimeError::TaskQueueFull);
    assert_eq!(ui.event(&a, |cx| cx.defer(|_| Ok(())))?, full);
    assert_eq!(ui.event(&a, |cx| cx.defer_while_shown(|_| Ok(())))?, full);
    assert_eq!(ui.event(&a, |cx| cx.spawn(|| (), |()| Ok(())))?, full);
    assert_eq!(ui.defer(|_| Ok(())), full);

    // Hiding the widget dooms its `defer_while_shown` operations, but they
    // keep their places until delivery.
    ui.set_visible(&a, false)?;
    assert_eq!(ui.defer(|_| Ok(())), full);
    ui.flush(Duration::ZERO)?;
    for (index, receipt) in receipts.iter().enumerate() {
        let expected = if index % 3 == 1 {
            TaskStatus::Cancelled
        } else {
            TaskStatus::Succeeded
        };
        assert_eq!(receipt.status(), expected, "operation {index}");
    }
    for _ in 0..Runtime::MAX_TASKS {
        let _receipt = ui.defer(|_| Ok(()))?;
    }
    Ok(())
}

/// Contract test: a flush nested in an operation delivers nothing more. The
/// operations of each form queued before delivery run after it, in order;
/// one queued inside it waits for the next flush.
#[test]
fn a_nested_flush_keeps_the_order_of_every_form() -> TestResult {
    let mut ui = Runtime::new();
    let (a, _) = buttons(&mut ui)?;
    let log = Log::default();
    let inner = Rc::new(RefCell::new(None));
    let (outer, slot) = (log.clone(), inner.clone());
    let first = ui.event(&a, |cx| {
        cx.defer(move |ui| {
            *slot.borrow_mut() = Some(ui.defer(logging(&outer, "inner"))?);
            ui.flush(ui.now())?;
            outer.borrow_mut().push("first");
            Ok(())
        })
    })??;
    let second = ui.event(&a, |cx| cx.defer_while_shown(logging(&log, "second")))??;
    let third = ui.defer(logging(&log, "third"))?;
    let fourth = ui.event(&a, |cx| cx.defer(logging(&log, "fourth")))??;
    ui.flush(Duration::ZERO)?;
    assert_eq!(*log.borrow(), ["first", "second", "third", "fourth"]);
    for receipt in [&first, &second, &third, &fourth] {
        assert_eq!(receipt.status(), TaskStatus::Succeeded);
    }
    let inner = inner.borrow_mut().take().ok_or("queued inside")?;
    assert_eq!(inner.status(), TaskStatus::Pending);
    ui.flush(Duration::ZERO)?;
    assert_eq!(inner.status(), TaskStatus::Succeeded);
    assert_eq!(log.borrow().last(), Some(&"inner"));
    Ok(())
}

/// Contract test: a failing operation records `Failed`, fails the flush and
/// is not replayed; the operations behind it run at the next flush.
#[test]
fn a_failing_operation_fails_the_flush_and_is_not_replayed() -> TestResult {
    let mut ui = Runtime::new();
    let (a, _) = buttons(&mut ui)?;
    let attempts = Rc::new(Cell::new(0));
    let counted = attempts.clone();
    let failing = ui.event(&a, |cx| {
        cx.defer(move |_| {
            counted.set(counted.get() + 1);
            Err(RuntimeError::EmptyKey)
        })
    })??;
    let log = Log::default();
    let later = ui.event(&a, |cx| cx.defer_while_shown(logging(&log, "later")))??;
    assert_eq!(ui.flush(Duration::ZERO), Err(RuntimeError::EmptyKey));
    assert_eq!(failing.status(), TaskStatus::Failed(RuntimeError::EmptyKey));
    assert_eq!(later.status(), TaskStatus::Pending);
    ui.flush(Duration::ZERO)?;
    assert_eq!(later.status(), TaskStatus::Succeeded);
    assert_eq!(*log.borrow(), ["later"]);
    ui.flush(Duration::ZERO)?;
    assert_eq!(attempts.get(), 1, "never replayed");
    Ok(())
}

/// Contract test: shutdown cancels the queued operations of every form, and
/// the runtime refuses new ones afterwards. The `EventContext` forms are
/// refused from the moment shutdown begins (`queuing_during_shutdown_is_refused`);
/// no event reaches a component afterwards.
#[test]
fn shutdown_cancels_every_form() -> TestResult {
    let mut ui = Runtime::new();
    let (a, _) = buttons(&mut ui)?;
    let receipts = [
        ui.event(&a, |cx| cx.defer(|_| panic!("shutdown must cancel")))??,
        ui.event(&a, |cx| {
            cx.defer_while_shown(|_| panic!("shutdown must cancel"))
        })??,
        ui.event(&a, |cx| {
            cx.spawn(|| panic!("shutdown must cancel the start"), |()| Ok(()))
        })??,
        ui.defer(|_| panic!("shutdown must cancel"))?,
    ];
    ui.shutdown();
    for receipt in &receipts {
        assert_eq!(receipt.status(), TaskStatus::Cancelled);
    }
    assert_eq!(ui.defer(|_| Ok(())), Err(RuntimeError::Closed));
    Ok(())
}

/// Contract test: an operation that shuts the runtime down is executing, so
/// it records its own result; the operations behind it are cancelled, none
/// stays `Pending`, and a `Runtime::defer` after the shutdown call is
/// refused. Contract mutation: cancel the executing operation's receipt on
/// shutdown; the `Succeeded` assertion fails.
#[test]
fn an_operation_that_shuts_down_records_its_own_result() -> TestResult {
    for fail in [false, true] {
        let mut ui = Runtime::new();
        let (a, _) = buttons(&mut ui)?;
        let refused = Rc::new(RefCell::new(None));
        let slot = refused.clone();
        let first = ui.event(&a, |cx| {
            cx.defer(move |ui| {
                ui.shutdown();
                *slot.borrow_mut() = Some(ui.defer(|_| Ok(())));
                if fail {
                    Err(RuntimeError::EmptyKey)
                } else {
                    Ok(())
                }
            })
        })??;
        let second = ui.event(&a, |cx| {
            cx.defer_while_shown(|_| panic!("shutdown must cancel"))
        })??;
        let third = ui.defer(|_| panic!("shutdown must cancel"))?;
        let _whatever = ui.flush(Duration::ZERO);
        let expected = if fail {
            TaskStatus::Failed(RuntimeError::EmptyKey)
        } else {
            TaskStatus::Succeeded
        };
        assert_eq!(first.status(), expected, "fail: {fail}");
        assert_eq!(second.status(), TaskStatus::Cancelled, "fail: {fail}");
        assert_eq!(third.status(), TaskStatus::Cancelled, "fail: {fail}");
        assert_eq!(
            refused.borrow_mut().take(),
            Some(Err(RuntimeError::Closed)),
            "fail: {fail}"
        );
    }
    Ok(())
}

/// Contract test: an operation that panics reads `Cancelled` once the unwind
/// is caught, and the operation behind it stays queued until the next flush.
#[test]
fn a_panicking_operation_reads_cancelled_and_the_next_waits() -> TestResult {
    let mut ui = Runtime::new();
    let (a, _) = buttons(&mut ui)?;
    let panicking = ui.event(&a, |cx| cx.defer(|_| panic!("the operation panics")))??;
    let log = Log::default();
    let behind = ui.event(&a, |cx| cx.defer(logging(&log, "behind")))??;
    let unwound =
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| ui.flush(Duration::ZERO)));
    assert!(unwound.is_err(), "the panic propagates from the flush");
    assert_eq!(panicking.status(), TaskStatus::Cancelled);
    assert_eq!(behind.status(), TaskStatus::Pending);
    ui.flush(Duration::ZERO)?;
    assert_eq!(behind.status(), TaskStatus::Succeeded);
    assert_eq!(*log.borrow(), ["behind"]);
    Ok(())
}
