//! Mailboxes: owned values from other threads reach a UI-thread handler.
use pert_ui_runtime::{Capacity, Runtime, RuntimeError, SendError};
use std::{
    cell::RefCell,
    rc::Rc,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

fn capacity(items: usize) -> Capacity {
    Capacity::new(items).unwrap()
}

/// Counts how many values have been dropped, wherever that happens.
struct Tracked(Arc<AtomicUsize>);
impl Drop for Tracked {
    fn drop(&mut self) {
        self.0.fetch_add(1, Ordering::SeqCst);
    }
}

#[test]
fn values_from_a_worker_reach_the_handler_in_order_and_mark_the_runtime_due() {
    let mut ui = Runtime::new();
    let received = Rc::new(RefCell::new(Vec::new()));
    let sink = received.clone();
    let mailbox = ui
        .mailbox(capacity(16), move |value: u32| {
            sink.borrow_mut().push(value);
            Ok(())
        })
        .unwrap();
    let sender = mailbox.sender();
    assert_eq!(ui.next_deadline(), None);
    std::thread::spawn(move || {
        for value in 0..10 {
            sender.send(value).unwrap();
        }
    })
    .join()
    .unwrap();
    assert_eq!(mailbox.pending(), 10);
    // A headless host without wake delivery sees the pending values as due.
    assert_eq!(ui.next_deadline(), Some(Duration::ZERO));
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(*received.borrow(), (0..10).collect::<Vec<_>>());
    assert_eq!(mailbox.pending(), 0);
    assert_eq!(ui.next_deadline(), None);
}

#[test]
fn the_handler_writes_signals_that_the_same_flush_shows() {
    let mut ui = Runtime::new();
    let result = ui.signal(String::new());
    let target = result.clone();
    let mailbox = ui
        .mailbox(capacity(1), move |text: String| target.set(text))
        .unwrap();
    mailbox.sender().send("decoded".to_owned()).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(result.get().unwrap(), "decoded");
}

#[test]
fn a_full_mailbox_returns_the_value_and_accepts_again_after_delivery() {
    let mut ui = Runtime::new();
    let mailbox = ui.mailbox(capacity(2), |_: u8| Ok(())).unwrap();
    let sender = mailbox.sender();
    sender.send(1).unwrap();
    sender.send(2).unwrap();
    assert_eq!(sender.send(3), Err(SendError::Full(3)));
    assert_eq!(mailbox.pending(), 2);
    ui.flush(Duration::ZERO).unwrap();
    sender.send(3).unwrap();
    assert_eq!(mailbox.capacity(), 2);
}

#[test]
fn closing_dropping_or_shutting_down_refuses_sends_and_drops_undelivered_values() {
    let mut ui = Runtime::new();
    let dropped = Arc::new(AtomicUsize::new(0));

    let closed = ui.mailbox(capacity(4), |_: Tracked| Ok(())).unwrap();
    let sender = closed.sender();
    sender.send(Tracked(dropped.clone())).unwrap();
    closed.close();
    assert_eq!(dropped.load(Ordering::SeqCst), 1);
    assert!(sender.is_closed());
    let refused = sender.send(Tracked(dropped.clone())).unwrap_err();
    assert!(matches!(refused, SendError::Closed(_)));
    drop(refused);
    assert_eq!(dropped.load(Ordering::SeqCst), 2);

    let dropped_mailbox = ui.mailbox(capacity(4), |_: Tracked| Ok(())).unwrap();
    let sender = dropped_mailbox.sender();
    sender.send(Tracked(dropped.clone())).unwrap();
    drop(dropped_mailbox);
    assert_eq!(dropped.load(Ordering::SeqCst), 3);
    assert!(sender.is_closed());

    let kept = ui.mailbox(capacity(4), |_: Tracked| Ok(())).unwrap();
    let sender = kept.sender();
    sender.send(Tracked(dropped.clone())).unwrap();
    ui.shutdown();
    assert_eq!(dropped.load(Ordering::SeqCst), 4);
    assert!(sender.is_closed());
    assert!(matches!(
        ui.mailbox(capacity(1), |_: u8| Ok(())),
        Err(RuntimeError::Closed)
    ));
}

#[test]
fn values_sent_while_a_flush_delivers_wait_for_the_next_flush() {
    let mut ui = Runtime::new();
    let received = Rc::new(RefCell::new(Vec::new()));
    let sink = received.clone();
    let echo = Rc::new(RefCell::new(None));
    let echo_sender = echo.clone();
    let mailbox = ui
        .mailbox(capacity(8), move |value: u32| {
            sink.borrow_mut().push(value);
            if let Some(sender) = echo_sender.borrow().as_ref()
                && value < 3
            {
                pert_ui_runtime::UiSender::send(sender, value + 1)
                    .map_err(|_| RuntimeError::Closed)?;
            }
            Ok(())
        })
        .unwrap();
    *echo.borrow_mut() = Some(mailbox.sender());
    mailbox.sender().send(0).unwrap();
    for (flush, expected) in [vec![0], vec![0, 1], vec![0, 1, 2], vec![0, 1, 2, 3]]
        .into_iter()
        .enumerate()
    {
        ui.flush(Duration::from_millis(flush as u64)).unwrap();
        assert_eq!(*received.borrow(), expected);
    }
    assert_eq!(mailbox.pending(), 0);
}

#[test]
fn a_handler_error_fails_the_flush_and_keeps_the_rest_queued() {
    let mut ui = Runtime::new();
    let received = Rc::new(RefCell::new(Vec::new()));
    let sink = received.clone();
    let mailbox = ui
        .mailbox(capacity(8), move |value: u32| {
            if value == 1 {
                return Err(RuntimeError::Exhausted);
            }
            sink.borrow_mut().push(value);
            Ok(())
        })
        .unwrap();
    let sender = mailbox.sender();
    for value in 0..4 {
        sender.send(value).unwrap();
    }
    assert!(matches!(
        ui.flush(Duration::ZERO),
        Err(RuntimeError::Exhausted)
    ));
    assert_eq!(*received.borrow(), [0]);
    assert_eq!(mailbox.pending(), 2);
    ui.flush(Duration::from_millis(1)).unwrap();
    assert_eq!(*received.borrow(), [0, 2, 3]);
}

#[test]
fn sends_between_mailboxes_wait_for_the_next_flush_in_either_registration_order() {
    for sender_first in [false, true] {
        let mut ui = Runtime::new();
        let target = Rc::new(RefCell::new(None::<pert_ui_runtime::UiSender<u32>>));
        let forwarding = target.clone();
        let received = Rc::new(RefCell::new(Vec::new()));
        let sink = received.clone();
        let forward = move |value: u32| {
            forwarding
                .borrow()
                .as_ref()
                .unwrap()
                .send(value)
                .map_err(|_| RuntimeError::Exhausted)
        };
        let record = move |value: u32| {
            sink.borrow_mut().push(value);
            Ok(())
        };
        let (a, b) = if sender_first {
            (
                ui.mailbox(capacity(4), forward).unwrap(),
                ui.mailbox(capacity(4), record).unwrap(),
            )
        } else {
            let b = ui.mailbox(capacity(4), record).unwrap();
            let a = ui.mailbox(capacity(4), forward).unwrap();
            (a, b)
        };
        *target.borrow_mut() = Some(b.sender());
        a.sender().send(1).unwrap();
        b.sender().send(10).unwrap();
        ui.flush(Duration::ZERO).unwrap();
        assert_eq!(*received.borrow(), [10]);
        assert_eq!(b.pending(), 1);
        assert_eq!(ui.next_deadline(), Some(Duration::ZERO));
        ui.flush(Duration::ZERO).unwrap();
        assert_eq!(*received.borrow(), [10, 1]);
        assert_eq!(b.pending(), 0);
    }
}

#[test]
fn dropping_a_later_mailbox_from_a_handler_closes_its_captured_front() {
    let mut ui = Runtime::new();
    let later = Rc::new(RefCell::new(None::<pert_ui_runtime::Mailbox<u32>>));
    let removing = later.clone();
    let a = ui
        .mailbox(capacity(1), move |_: u32| {
            removing.borrow_mut().take();
            Ok(())
        })
        .unwrap();
    let received = Rc::new(RefCell::new(Vec::new()));
    let sink = received.clone();
    let b = ui
        .mailbox(capacity(1), move |value: u32| {
            sink.borrow_mut().push(value);
            Ok(())
        })
        .unwrap();
    let sender = b.sender();
    sender.send(10).unwrap();
    *later.borrow_mut() = Some(b);
    a.sender().send(1).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert!(received.borrow().is_empty());
    assert!(sender.is_closed());
    assert_eq!(sender.send(20), Err(SendError::Closed(20)));
}

#[test]
fn an_accepted_send_wakes_the_host_and_a_refused_one_does_not() {
    let mut ui = Runtime::new();
    let wakes = Arc::new(AtomicUsize::new(0));
    let counter = wakes.clone();
    ui.set_wake_delivery(Box::new(move || {
        counter.fetch_add(1, Ordering::SeqCst);
        true
    }));
    let mailbox = ui.mailbox(capacity(1), |_: u8| Ok(())).unwrap();
    let sender = mailbox.sender();
    sender.send(1).unwrap();
    assert_eq!(wakes.load(Ordering::SeqCst), 1);
    // A refused value does not wake.
    assert!(sender.send(2).is_err());
    assert_eq!(wakes.load(Ordering::SeqCst), 1);
    ui.flush(Duration::ZERO).unwrap();
    sender.send(3).unwrap();
    assert_eq!(wakes.load(Ordering::SeqCst), 2);
}

#[test]
fn concurrent_workers_lose_nothing_they_were_told_was_accepted() {
    const WORKERS: u32 = 4;
    const EACH: u32 = 2_000;
    let mut ui = Runtime::new();
    let received = Rc::new(RefCell::new(Vec::new()));
    let sink = received.clone();
    let mailbox = ui
        .mailbox(capacity(64), move |value: (u32, u32)| {
            sink.borrow_mut().push(value);
            Ok(())
        })
        .unwrap();
    let workers: Vec<_> = (0..WORKERS)
        .map(|worker| {
            let sender = mailbox.sender();
            std::thread::spawn(move || {
                for index in 0..EACH {
                    let mut value = (worker, index);
                    // A full mailbox hands the value back; retry it.
                    while let Err(error) = sender.send(value) {
                        assert!(matches!(error, SendError::Full(_)));
                        value = error.into_inner();
                        std::thread::yield_now();
                    }
                }
            })
        })
        .collect();
    let mut now = Duration::ZERO;
    let deadline = std::time::Instant::now() + Duration::from_secs(60);
    while received.borrow().len() < (WORKERS * EACH) as usize {
        assert!(std::time::Instant::now() < deadline, "workers stalled");
        let before = mailbox.pending();
        now += Duration::from_millis(1);
        ui.flush(now).unwrap();
        assert!(before <= mailbox.capacity());
        std::thread::yield_now();
    }
    for worker in workers {
        worker.join().unwrap();
    }
    // Every worker's values arrived exactly once and in its own order.
    for worker in 0..WORKERS {
        let order: Vec<u32> = received
            .borrow()
            .iter()
            .filter(|(from, _)| *from == worker)
            .map(|(_, index)| *index)
            .collect();
        assert_eq!(order, (0..EACH).collect::<Vec<_>>());
    }
}
