//! Live-data source scheduling contracts: subscription-driven drains, no work
//! without change, hidden suspension and resume, notified delivery, teardown
//! and inspection, all headless.

use pert_ui_runtime::testing::HostIntegration;
use std::{
    cell::RefCell,
    rc::Rc,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

use pert_ui_runtime::{
    Capacity, Delivery, Freshness, PublishOutcome, RuntimeError, SeriesConfig, SeriesEndpoint,
    SeriesSource, Telemetry, Timestamp, WakeError, component::*, prelude::*, series_bridge,
};

type TestResult = Result<(), Box<dyn std::error::Error>>;
/// What one tracked read observed: the window length and the whole telemetry.
type SeriesLog = Rc<RefCell<Vec<(usize, Telemetry<u64>)>>>;
type SeriesFixture = (
    SeriesEndpoint<u64>,
    SeriesSource<u64>,
    SeriesReader,
    SeriesLog,
);

const INTERVAL: Duration = Duration::from_nanos(16_666_667);

struct SeriesReader {
    source: SeriesSource<u64>,
    seen: SeriesLog,
}
impl Component for SeriesReader {
    fn measure(
        &self,
        _: &pert_ui_runtime::MeasureContext<'_>,
        _: pert_ui_runtime::Constraints,
    ) -> Result<pert_ui_runtime::Size, RuntimeError> {
        Ok(pert_ui_runtime::Size::new(20.0, 20.0)?)
    }
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        let observed =
            cx.read_series(&self.source, |telemetry, window| (window.len(), *telemetry))?;
        self.seen.borrow_mut().push(observed);
        Ok(())
    }
}

fn series(
    runtime: &Runtime,
    ring: usize,
    delivery: Delivery,
) -> Result<SeriesFixture, Box<dyn std::error::Error>> {
    let config = SeriesConfig::new(Capacity::new(8)?).with_ring_capacity(Capacity::new(ring)?);
    let (endpoint, consumer) = series_bridge(config);
    let source = runtime.series_source(consumer, delivery);
    let seen = Rc::new(RefCell::new(Vec::new()));
    let reader = SeriesReader {
        source: source.clone(),
        seen: seen.clone(),
    };
    Ok((endpoint, source, reader, seen))
}

/// Flushes until the runtime has no work due at `now`, returning the updates.
fn settle(runtime: &mut Runtime, now: Duration) -> Result<usize, RuntimeError> {
    let mut updated = 0;
    while runtime
        .next_deadline()
        .is_some_and(|deadline| deadline <= now)
    {
        updated += runtime.flush(now)?.components_updated;
    }
    Ok(updated)
}

#[test]
fn a_visible_subscriber_starts_drains_and_only_changes_update_it() -> TestResult {
    let mut runtime = Runtime::new();
    runtime.set_viewport(Viewport::new(100.0, 100.0, 1.0)?)?;
    let (mut endpoint, _source, reader, seen) = series(&runtime, 4, Delivery::Polled)?;
    let id = runtime.mount_tree(Element::new("meter", reader))?;
    // Before anything subscribes, the source needs nothing.
    let _ = endpoint.publish(2, Timestamp::new(1));
    assert_eq!(runtime.next_deadline(), Some(Duration::ZERO));
    // The first update subscribes; the runtime then activates the source,
    // which discards what accumulated before anyone watched. The discard is
    // a change, so the subscriber is evaluated once more and sees nothing.
    assert_eq!(settle(&mut runtime, Duration::ZERO)?, 2);
    assert_eq!(seen.borrow().len(), 2);
    assert!(
        seen.borrow()
            .iter()
            .all(|(_, t)| t.freshness == Freshness::Unavailable && t.latest.is_none())
    );
    assert_eq!(runtime.next_deadline(), Some(INTERVAL));

    // A drain that finds nothing evaluates nobody and reschedules.
    assert_eq!(runtime.flush(INTERVAL)?.components_updated, 0);
    assert_eq!(runtime.next_deadline(), Some(INTERVAL * 2));

    // Items between two drains reach the subscriber in one update.
    assert_eq!(
        endpoint.publish(5, Timestamp::new(2)),
        PublishOutcome::Accepted
    );
    assert_eq!(
        endpoint.publish(9, Timestamp::new(3)),
        PublishOutcome::Accepted
    );
    assert_eq!(runtime.flush(INTERVAL * 2)?.components_updated, 1);
    let (len, latest) = seen.borrow()[2];
    let sample = latest.latest.ok_or("sample")?;
    assert_eq!((len, sample.value, sample.timestamp.get()), (2, 9, 3));
    assert_eq!(latest.freshness, Freshness::Fresh);
    assert_eq!(latest.received, Some(INTERVAL * 2));

    // Quiet again: stale with the age since the last change, no update.
    assert_eq!(runtime.flush(INTERVAL * 3)?.components_updated, 0);
    let telemetry = _source.telemetry()?;
    assert_eq!(telemetry.freshness, Freshness::Stale { age: INTERVAL });
    assert_eq!(telemetry.latest.map(|sample| sample.value), Some(9));

    // Hidden: no deadline, no drain; the producer keeps publishing.
    runtime.set_visible(&id, false)?;
    settle(&mut runtime, INTERVAL * 3)?;
    assert_eq!(runtime.next_deadline(), None);
    let _ = endpoint.publish(7, Timestamp::new(4));
    assert_eq!(
        runtime.flush(Duration::from_secs(30))?.components_updated,
        0
    );
    assert_eq!(runtime.next_deadline(), None);

    // Shown again: resume discards the backlog and restarts from new items.
    runtime.set_visible(&id, true)?;
    settle(&mut runtime, Duration::from_secs(30))?;
    assert_eq!(seen.borrow().last().map(|(_, t)| t.latest), Some(None));
    let _ = endpoint.publish(3, Timestamp::new(5));
    let deadline = runtime.next_deadline().ok_or("deadline")?;
    assert_eq!(runtime.flush(deadline)?.components_updated, 1);
    let (len, telemetry) = *seen.borrow().last().ok_or("read")?;
    let sample = telemetry.latest.ok_or("sample")?;
    assert_eq!((len, sample.value), (1, 3));

    // Unmounted: the last subscriber is gone and nothing is scheduled.
    runtime.unmount(&id)?;
    settle(&mut runtime, deadline)?;
    assert_eq!(runtime.next_deadline(), None);
    Ok(())
}

#[test]
fn a_quiet_resume_keeps_the_window_and_a_lossy_one_clears_it() -> TestResult {
    let mut runtime = Runtime::new();
    runtime.set_viewport(Viewport::new(100.0, 100.0, 1.0)?)?;
    let (mut endpoint, source, reader, seen) = series(&runtime, 8, Delivery::Polled)?;
    let id = runtime.mount_tree(Element::new("meter", reader))?;
    settle(&mut runtime, Duration::ZERO)?;
    assert_eq!(
        endpoint.publish(4, Timestamp::new(1)),
        PublishOutcome::Accepted
    );
    assert_eq!(runtime.flush(INTERVAL)?.components_updated, 1);
    let before = source.telemetry()?;
    assert_eq!(before.latest.map(|sample| sample.value), Some(4));

    // Hidden and shown again with nothing published in between: nothing was
    // lost, so the last value stays, stale since it arrived, and the resume
    // is not a change.
    runtime.set_visible(&id, false)?;
    settle(&mut runtime, INTERVAL)?;
    runtime.set_visible(&id, true)?;
    settle(&mut runtime, INTERVAL * 3)?;
    let kept = source.telemetry()?;
    assert_eq!(kept.latest, before.latest);
    assert_eq!(kept.freshness, Freshness::Stale { age: INTERVAL * 2 });
    assert_eq!(kept.revision, before.revision);

    // Hidden while the producer publishes: the resume discards that backlog
    // and clears the window the subscriber last read, which is a change.
    runtime.set_visible(&id, false)?;
    settle(&mut runtime, INTERVAL * 3)?;
    let _ = endpoint.publish(5, Timestamp::new(2));
    runtime.set_visible(&id, true)?;
    settle(&mut runtime, INTERVAL * 4)?;
    let after = source.telemetry()?;
    assert!(after.latest.is_none());
    assert_eq!(after.freshness, Freshness::Unavailable);
    assert_ne!(after.revision, before.revision, "the clearing is a change");
    let (len, read) = *seen.borrow().last().ok_or("read")?;
    assert_eq!((len, read.latest, read.revision), (0, None, after.revision));
    drop(endpoint);
    Ok(())
}

#[test]
fn shutdown_closes_the_waker_permanently() -> TestResult {
    let mut runtime = Runtime::new();
    let delivered = Arc::new(AtomicUsize::new(0));
    let waker = runtime.live_waker();
    runtime.shutdown();
    assert_eq!(waker.notify(), Err(WakeError::Closed));
    let counter = Arc::clone(&delivered);
    runtime.set_wake_delivery(move || {
        counter.fetch_add(1, Ordering::SeqCst);
        true
    });
    assert_eq!(waker.notify(), Err(WakeError::Closed));
    assert_eq!(runtime.live_waker().notify(), Err(WakeError::Closed));
    assert_eq!(
        delivered.load(Ordering::SeqCst),
        0,
        "no delivery after shutdown"
    );
    Ok(())
}

#[test]
fn a_series_subscriber_sees_the_window_and_the_losses() -> TestResult {
    let mut runtime = Runtime::new();
    runtime.set_viewport(Viewport::new(100.0, 100.0, 1.0)?)?;
    let (mut endpoint, source, reader, seen) = series(&runtime, 2, Delivery::Polled)?;
    runtime.mount_tree(Element::new("trace", reader))?;
    settle(&mut runtime, Duration::ZERO)?;
    for tick in 1..=3 {
        let _ = endpoint.publish(tick * 10, Timestamp::new(tick));
    }
    assert_eq!(runtime.flush(INTERVAL)?.components_updated, 1);
    let (len, telemetry) = *seen.borrow().last().ok_or("read")?;
    assert_eq!(len, 2);
    assert_eq!(telemetry.latest.map(|sample| sample.value), Some(20));
    assert_eq!(telemetry.freshness, Freshness::Fresh);
    assert!(telemetry.revision > 0);
    source.with_window(|telemetry, window| {
        assert_eq!(
            telemetry.latest.map(|sample| sample.sequence.get()),
            Some(2)
        );
        let entries: Vec<_> = window
            .iter()
            .map(|entry| (entry.sample.value, entry.discontinuity_before))
            .collect();
        assert_eq!(entries, [(10, false), (20, false)]);
    })?;
    // The dropped third item is missing; the next item is discontinuous.
    let _ = endpoint.publish(40, Timestamp::new(4));
    assert_eq!(runtime.flush(INTERVAL * 2)?.components_updated, 1);
    source.with_window(|_, window| {
        let last = window.iter().last().expect("item");
        assert_eq!((last.sample.value, last.discontinuity_before), (40, true));
    })?;
    assert_eq!(runtime.inspect().sources[0].dropped_total, 1);
    Ok(())
}

#[test]
fn invalid_only_drains_report_changes_without_refreshing_the_retained_value() -> TestResult {
    struct FloatReader(SeriesSource<f32>);
    impl Component for FloatReader {
        fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
            cx.read_series(&self.0, |_, _| ())
        }
        fn measure(
            &self,
            _: &pert_ui_runtime::MeasureContext<'_>,
            _: pert_ui_runtime::Constraints,
        ) -> Result<pert_ui_runtime::Size, RuntimeError> {
            Ok(pert_ui_runtime::Size::new(20.0, 20.0)?)
        }
    }
    let mut runtime = Runtime::new();
    runtime.set_viewport(Viewport::new(100.0, 100.0, 1.0)?)?;
    let (mut endpoint, consumer) = series_bridge(SeriesConfig::new(Capacity::new(4)?));
    let source = runtime.series_source(consumer, Delivery::Polled);
    runtime.mount_tree(Element::new("reader", FloatReader(source.clone())))?;
    settle(&mut runtime, Duration::ZERO)?;
    assert_eq!(
        endpoint.publish(2.0, Timestamp::new(1)),
        PublishOutcome::Accepted
    );
    runtime.flush(INTERVAL)?;
    let mut revision = source.telemetry()?.revision;
    for tick in 2..=3 {
        assert_eq!(
            endpoint.publish(f32::NAN, Timestamp::new(tick)),
            PublishOutcome::Invalid
        );
        assert_eq!(runtime.flush(INTERVAL * tick as u32)?.components_updated, 1);
        let observed = source.telemetry()?;
        assert_ne!(observed.revision, revision);
        revision = observed.revision;
        assert_eq!(observed.latest.map(|sample| sample.value), Some(2.0));
        assert_eq!(observed.received, Some(INTERVAL));
        assert_eq!(
            observed.freshness,
            Freshness::Stale {
                age: INTERVAL * (tick - 1) as u32
            }
        );
        assert_eq!(runtime.inspect().sources[0].freshness, observed.freshness);
    }
    let incarnation = endpoint.restart()?;
    runtime.flush(INTERVAL * 4)?;
    let observed = source.telemetry()?;
    assert_eq!(observed.latest, None);
    assert_eq!(observed.incarnation, incarnation);
    assert_eq!(observed.received, None);
    assert_eq!(observed.freshness, Freshness::Unavailable);
    Ok(())
}

#[test]
fn dropping_the_endpoint_is_observed_once_and_ends_scheduling_work() -> TestResult {
    let mut runtime = Runtime::new();
    runtime.set_viewport(Viewport::new(100.0, 100.0, 1.0)?)?;
    let (mut endpoint, source, reader, seen) = series(&runtime, 4, Delivery::Polled)?;
    runtime.mount_tree(Element::new("meter", reader))?;
    settle(&mut runtime, Duration::ZERO)?;
    let _ = endpoint.publish(4, Timestamp::new(1));
    drop(endpoint);
    assert_eq!(runtime.flush(INTERVAL)?.components_updated, 1);
    let (_, telemetry) = seen.borrow().last().copied().ok_or("read")?;
    assert!(telemetry.disconnected);
    assert_eq!(telemetry.freshness, Freshness::Unavailable);
    assert_eq!(telemetry.latest.map(|sample| sample.value), Some(4));
    assert!(source.telemetry()?.disconnected);
    assert_eq!(runtime.flush(INTERVAL * 2)?.components_updated, 0);
    Ok(())
}

#[test]
fn a_notified_source_drains_on_wakes_and_coalesces_them() -> TestResult {
    let mut runtime = Runtime::new();
    runtime.set_viewport(Viewport::new(100.0, 100.0, 1.0)?)?;
    let delivered = Arc::new(AtomicUsize::new(0));
    let counter = Arc::clone(&delivered);
    runtime.set_wake_delivery(move || {
        counter.fetch_add(1, Ordering::SeqCst);
        true
    });
    let (mut endpoint, _source, reader, seen) = series(&runtime, 4, Delivery::Notified)?;
    runtime.mount_tree(Element::new("meter", reader))?;
    settle(&mut runtime, Duration::ZERO)?;
    assert_eq!(
        runtime.next_deadline(),
        None,
        "a notified source has no cadence"
    );

    // Published but not notified: nothing drains, however much time passes.
    let _ = endpoint.publish(6, Timestamp::new(1));
    assert_eq!(runtime.flush(Duration::from_secs(1))?.components_updated, 0);

    // A notification from another thread makes the runtime due and wakes once.
    let waker = runtime.live_waker();
    let producer = std::thread::spawn(move || {
        let _ = endpoint.publish(8, Timestamp::new(2));
        waker.notify()?;
        waker.notify()?;
        Ok::<_, WakeError>(endpoint)
    });
    let endpoint = producer.join().map_err(|_| "producer")??;
    assert_eq!(
        delivered.load(Ordering::SeqCst),
        1,
        "the second notify coalesced"
    );
    assert_eq!(runtime.next_deadline(), Some(runtime.now()));
    assert_eq!(runtime.flush(Duration::from_secs(2))?.components_updated, 1);
    let (len, telemetry) = *seen.borrow().last().ok_or("read")?;
    let sample = telemetry.latest.ok_or("sample")?;
    assert_eq!(
        (len, sample.value),
        (2, 8),
        "one drain took both items the wake covered"
    );
    assert_eq!(runtime.next_deadline(), None);

    // Delivery that fails closes the waker and clears the flag.
    runtime.set_wake_delivery(|| false);
    let waker = runtime.live_waker();
    assert_eq!(waker.notify(), Err(WakeError::Closed));
    assert_eq!(runtime.next_deadline(), None);
    assert_eq!(waker.notify(), Err(WakeError::Closed));
    drop(endpoint);
    Ok(())
}

#[test]
fn installing_delivery_reopens_a_waker_whose_delivery_failed() -> TestResult {
    // Only the sequential contract is tested. The concurrent case, an install
    // that waits for the delivery lock while a failing delivery holds it, is
    // kept correct by closing the waker inside that lock; the gap the old code
    // left after unlocking was a few instructions wide, which no test can hit
    // reliably without a hook in the production path.
    let runtime = Runtime::new();
    runtime.set_wake_delivery(|| false);
    let waker = runtime.live_waker();
    assert_eq!(waker.notify(), Err(WakeError::Closed));
    assert_eq!(
        runtime.next_deadline(),
        None,
        "the failure cleared the flag"
    );

    let delivered = Arc::new(AtomicUsize::new(0));
    let counter = Arc::clone(&delivered);
    runtime.set_wake_delivery(move || {
        counter.fetch_add(1, Ordering::SeqCst);
        true
    });
    assert_eq!(waker.notify(), Ok(()), "the replacement reopened the waker");
    assert_eq!(delivered.load(Ordering::SeqCst), 1);
    assert_eq!(runtime.next_deadline(), Some(runtime.now()));
    Ok(())
}

#[test]
fn inspection_lists_sources_with_their_schedule() -> TestResult {
    let mut runtime = Runtime::new();
    runtime.set_viewport(Viewport::new(100.0, 100.0, 1.0)?)?;
    let (_meter_endpoint, _meter_source, meter_reader, _) = series(&runtime, 4, Delivery::Polled)?;
    let (_trace_endpoint, _trace_source, trace_reader, _) =
        series(&runtime, 4, Delivery::Notified)?;
    runtime.mount_tree(Element::new("meter", meter_reader))?;
    runtime.mount_tree(Element::new("trace", trace_reader))?;
    settle(&mut runtime, Duration::ZERO)?;
    let snapshot = runtime.inspect();
    assert_eq!(snapshot.schema_version, 15);
    let schedules: Vec<_> = snapshot
        .sources
        .iter()
        .map(|source| {
            (
                source.delivery,
                source.subscribers,
                source.visible_subscribers,
                source.next_drain,
            )
        })
        .collect();
    assert_eq!(
        schedules,
        [
            (Delivery::Polled, 1, 1, Some(INTERVAL)),
            (Delivery::Notified, 1, 1, None),
        ],
        "a polled source carries a cadence and a notified one does not"
    );
    Ok(())
}

#[test]
fn a_source_of_another_runtime_is_rejected() -> TestResult {
    let mut runtime = Runtime::new();
    let other = Runtime::new();
    let (_endpoint, _source, reader, _) = series(&other, 4, Delivery::Polled)?;
    runtime.mount_tree(Element::new("meter", reader))?;
    assert_eq!(
        runtime.flush(Duration::ZERO),
        Err(RuntimeError::ForeignRuntime)
    );
    Ok(())
}

#[test]
fn subscribers_in_two_native_roots_see_one_drain_for_one_time() -> TestResult {
    use pert_ui_runtime::{Column, NativeWindowOptions, Tab, Tabs, Viewport};
    let mut runtime = Runtime::new();
    runtime.set_viewport(Viewport::new(400.0, 300.0, 1.0)?)?;
    let (mut endpoint, source, docked, docked_seen) = series(&runtime, 8, Delivery::Polled)?;
    let detached_seen: SeriesLog = Rc::new(RefCell::new(Vec::new()));
    let moved = SeriesReader {
        source: source.clone(),
        seen: detached_seen.clone(),
    };
    let root = runtime.mount_tree(Element::new(
        "column",
        Column::new([
            Element::new(
                "tabs",
                Tabs::new([Tab::new("view", "View", Element::new("moved", moved))]),
            ),
            Element::new("docked", docked),
        ]),
    ))?;
    settle(&mut runtime, Duration::ZERO)?;
    let tabs = runtime.children(&root)?[0].clone();
    let view = runtime.children(&tabs)?[0].clone();
    let primary = runtime.primary_window();
    let prepared = runtime.prepare_detach(&view, NativeWindowOptions::new("View")?)?;
    let detached = runtime.commit_detach(prepared)?;
    assert_ne!(runtime.window_of(&view)?, primary);
    runtime.select_window(&primary)?;
    while runtime.next_deadline().is_some_and(|at| at <= INTERVAL) {
        runtime.flush_windows(INTERVAL)?;
    }
    let (before_docked, before_moved) = (docked_seen.borrow().len(), detached_seen.borrow().len());

    // One publish, then one shared time for both roots.
    assert_eq!(
        endpoint.publish(7, Timestamp::new(1)),
        PublishOutcome::Accepted
    );
    let now = INTERVAL * 4;
    let report = runtime.flush_windows(now)?;
    assert_eq!(report.components_updated, 2, "one reader per root");
    assert_eq!(
        runtime.current_window(),
        primary,
        "the selected root is retained"
    );
    let docked = docked_seen.borrow();
    let moved = detached_seen.borrow();
    assert_eq!(docked.len(), before_docked + 1);
    assert_eq!(moved.len(), before_moved + 1);
    let (a, b) = (docked.last().ok_or("docked")?, moved.last().ok_or("moved")?);
    // The second root did not drain again: both readers hold the same revision,
    // receipt time, freshness and window for that one drain.
    assert_eq!(a, b);
    assert_eq!(a.1.latest.map(|sample| sample.value), Some(7));
    assert_eq!(a.1.received, Some(now));
    assert_eq!(a.1.freshness, Freshness::Fresh);
    assert_eq!(runtime.inspect().sources[0].revision, a.1.revision);
    drop((docked, moved));

    // Flushing the same time again drains nothing and updates nobody.
    assert_eq!(runtime.flush_windows(now)?.components_updated, 0);
    assert!(runtime.contains_window(&detached));
    Ok(())
}

#[test]
fn a_notified_source_observes_a_restart_and_a_drop_that_no_publish_follows() -> TestResult {
    let mut runtime = Runtime::new();
    runtime.set_viewport(Viewport::new(100.0, 100.0, 1.0)?)?;
    runtime.set_wake_delivery(|| true);
    let (mut endpoint, _source, reader, seen) = series(&runtime, 4, Delivery::Notified)?;
    runtime.mount_tree(Element::new("meter", reader))?;
    settle(&mut runtime, Duration::ZERO)?;
    let waker = runtime.live_waker();
    let _ = endpoint.publish(3, Timestamp::new(1));
    waker.notify()?;
    assert_eq!(runtime.flush(Duration::from_secs(1))?.components_updated, 1);
    assert_eq!(
        runtime.next_deadline(),
        None,
        "a notified source has no cadence"
    );
    let first = seen.borrow().last().ok_or("read")?.1;
    let incarnation = first.latest.ok_or("sample")?.incarnation;

    // A restart publishes nothing. Without the host's notify it stays unobserved,
    // however much time passes, because nothing schedules a drain.
    let restarted = endpoint.restart()?;
    assert!(restarted > incarnation);
    assert_eq!(runtime.flush(Duration::from_secs(2))?.components_updated, 0);
    assert_eq!(runtime.inspect().sources[0].incarnation, incarnation.get());
    waker.notify()?;
    assert_eq!(runtime.next_deadline(), Some(runtime.now()));
    assert_eq!(runtime.flush(Duration::from_secs(3))?.components_updated, 1);
    let (len, telemetry) = *seen.borrow().last().ok_or("read")?;
    assert_eq!(len, 0, "the new incarnation cleared the window");
    assert!(telemetry.revision != first.revision);
    assert_eq!(runtime.inspect().sources[0].incarnation, restarted.get());
    assert_eq!(runtime.next_deadline(), None);

    // The same holds for a drop after the last item was drained: the source keeps
    // claiming to be connected until the host's notify lets the runtime look.
    drop(endpoint);
    assert_eq!(runtime.flush(Duration::from_secs(4))?.components_updated, 0);
    assert!(!runtime.inspect().sources[0].disconnected);
    waker.notify()?;
    assert_eq!(runtime.flush(Duration::from_secs(5))?.components_updated, 1);
    let (_, telemetry) = *seen.borrow().last().ok_or("read")?;
    assert_eq!(telemetry.freshness, Freshness::Unavailable);
    assert!(runtime.inspect().sources[0].disconnected);
    assert_eq!(runtime.next_deadline(), None, "nothing further can arrive");
    Ok(())
}

#[test]
fn same_time_hide_deadline_show_settles_and_restarts_at_display_rate() -> TestResult {
    let mut runtime = Runtime::new();
    runtime.set_viewport(Viewport::new(100.0, 100.0, 1.0)?)?;
    let (_endpoint, _source, reader, _seen) = series(&runtime, 8, Delivery::Polled)?;
    let id = runtime.mount_tree(Element::new("reader", reader))?;
    settle(&mut runtime, Duration::ZERO)?;
    runtime.set_visible(&id, false)?;
    let _ = runtime.next_deadline();
    runtime.set_visible(&id, true)?;
    for _ in 0..4 {
        runtime.flush(Duration::ZERO)?;
    }
    assert_eq!(runtime.next_deadline(), Some(INTERVAL));
    runtime.set_presentation_rate(Some(pert_ui_runtime::PresentationRate::new(120.0)?))?;
    assert_eq!(
        runtime.next_deadline(),
        Some(Duration::from_nanos(8_333_333))
    );
    Ok(())
}

#[test]
fn a_lossy_resume_can_advance_revision_again_at_the_same_time() -> TestResult {
    let mut runtime = Runtime::new();
    runtime.set_viewport(Viewport::new(100.0, 100.0, 1.0)?)?;
    let (mut endpoint, source, reader, _) = series(&runtime, 4, Delivery::Polled)?;
    let id = runtime.mount_tree(Element::new("reader", reader))?;
    settle(&mut runtime, Duration::ZERO)?;
    assert_eq!(
        endpoint.publish(5, Timestamp::new(1)),
        PublishOutcome::Accepted
    );
    runtime.flush(INTERVAL)?;
    let before = source.telemetry()?;
    assert_eq!(before.received, Some(INTERVAL));
    runtime.set_visible(&id, false)?;
    assert_eq!(runtime.next_deadline(), None);
    assert_eq!(
        endpoint.publish(7, Timestamp::new(2)),
        PublishOutcome::Accepted
    );
    runtime.set_visible(&id, true)?;
    settle(&mut runtime, INTERVAL)?;
    let resumed = source.telemetry()?;
    assert_ne!(resumed.revision, before.revision);
    assert_eq!(resumed.latest, None);
    assert_eq!(resumed.received, None);
    assert_eq!(resumed.freshness, Freshness::Unavailable);
    assert_eq!(runtime.next_deadline(), Some(INTERVAL * 2));
    Ok(())
}

#[test]
fn a_shared_source_uses_the_fastest_presented_window() -> TestResult {
    use pert_ui_runtime::{Element, NativeWindowOptions, PresentationRate, Viewport};
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(100.0, 100.0, 1.0)?)?;
    let (_endpoint, source, reader, seen) = series(&ui, 8, Delivery::Polled)?;
    let primary_reader = ui.mount_tree(Element::new("primary-reader", reader))?;
    let view = ui.mount_tree(Element::new(
        "detachable",
        pert_ui_runtime::Label::new("Window"),
    ))?;
    let prepared = ui.prepare_detach(&view, NativeWindowOptions::new("Second")?)?;
    let detached = ui.commit_detach(prepared)?;
    ui.select_window(&detached)?;
    ui.set_presentation_rate(Some(PresentationRate::new(120.0)?))?;
    ui.mount_tree(Element::new("second-reader", SeriesReader { source, seen }))?;
    for _ in 0..3 {
        ui.flush_windows(Duration::ZERO)?;
    }
    assert_eq!(
        ui.application_deadline(),
        Some(Duration::from_nanos(8_333_333))
    );
    ui.set_window_visible(false)?;
    ui.select_window(&ui.primary_window())?;
    ui.flush(Duration::ZERO)?;
    assert_eq!(ui.application_deadline(), Some(INTERVAL));
    ui.set_visible(&primary_reader, false)?;
    ui.flush(Duration::ZERO)?;
    assert_eq!(ui.application_deadline(), None);
    Ok(())
}

#[test]
fn live_publication_does_not_update_a_clipped_subscriber_of_an_active_source() -> TestResult {
    use pert_ui_runtime::{Length, Row, Viewport};
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(20.0, 20.0, 1.0)?)?;
    let (mut endpoint, source, visible, seen) = series(&ui, 8, Delivery::Polled)?;
    let clipped_seen = Rc::new(RefCell::new(Vec::new()));
    ui.mount_tree(Element::new(
        "row",
        Row::new([
            Element::new("visible", visible),
            Element::new(
                "clipped",
                SeriesReader {
                    source,
                    seen: clipped_seen.clone(),
                },
            ),
        ])
        .gap(Length::new(0.0)?),
    ))?;
    settle(&mut ui, Duration::ZERO)?;
    let (before_visible, before_clipped) = (seen.borrow().len(), clipped_seen.borrow().len());
    assert_eq!(
        endpoint.publish(7, Timestamp::new(1)),
        PublishOutcome::Accepted
    );
    ui.flush(INTERVAL)?;
    assert_eq!(seen.borrow().len(), before_visible + 1);
    assert_eq!(clipped_seen.borrow().len(), before_clipped);
    ui.set_viewport(Viewport::new(40.0, 20.0, 1.0)?)?;
    ui.flush(INTERVAL)?;
    assert!(clipped_seen.borrow().len() > before_clipped);
    assert_eq!(
        clipped_seen
            .borrow()
            .last()
            .unwrap()
            .1
            .latest
            .unwrap()
            .value,
        7
    );
    Ok(())
}

#[test]
fn a_wake_at_the_instant_of_the_last_drain_still_drains() -> TestResult {
    let mut runtime = Runtime::new();
    runtime.set_viewport(Viewport::new(100.0, 100.0, 1.0)?)?;
    runtime.set_wake_delivery(|| true);
    let (mut endpoint, _source, reader, seen) = series(&runtime, 4, Delivery::Notified)?;
    runtime.mount_tree(Element::new("meter", reader))?;
    settle(&mut runtime, Duration::ZERO)?;
    let waker = runtime.live_waker();
    let at = Duration::from_secs(1);

    let _ = endpoint.publish(6, Timestamp::new(1));
    waker.notify()?;
    assert_eq!(runtime.flush(at)?.components_updated, 1);

    // Published and notified after that drain, flushed again at the same time:
    // the flush consumes the wake, so it must drain rather than skip.
    let _ = endpoint.publish(8, Timestamp::new(2));
    waker.notify()?;
    assert_eq!(runtime.flush(at)?.components_updated, 1);
    let (_, telemetry) = *seen.borrow().last().ok_or("read")?;
    assert_eq!(telemetry.latest.ok_or("sample")?.value, 8);
    assert_eq!(runtime.next_deadline(), None);
    Ok(())
}

#[test]
fn a_delivery_callback_that_panicked_does_not_swallow_later_wakes() -> TestResult {
    let mut runtime = Runtime::new();
    let calls = Arc::new(AtomicUsize::new(0));
    let counter = Arc::clone(&calls);
    runtime.set_wake_delivery(move || {
        assert!(
            counter.fetch_add(1, Ordering::SeqCst) > 0,
            "the first delivery panics"
        );
        true
    });
    let waker = runtime.live_waker();
    let panicked = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| waker.notify()));
    assert!(panicked.is_err(), "the host caught the delivery panic");
    // The panic left the flag pending; the flush takes it, and the next notify
    // must reach delivery through the poisoned lock rather than report a wake
    // nobody delivered.
    runtime.flush(Duration::ZERO)?;
    waker.notify()?;
    assert_eq!(calls.load(Ordering::SeqCst), 2);
    Ok(())
}

#[test]
fn a_wake_flushed_while_the_window_is_read_is_served_later() -> TestResult {
    let mut runtime = Runtime::new();
    runtime.set_viewport(Viewport::new(100.0, 100.0, 1.0)?)?;
    runtime.set_wake_delivery(|| true);
    let (mut endpoint, source, reader, seen) = series(&runtime, 4, Delivery::Notified)?;
    runtime.mount_tree(Element::new("meter", reader))?;
    settle(&mut runtime, Duration::ZERO)?;
    let _ = endpoint.publish(5, Timestamp::new(1));
    runtime.live_waker().notify()?;
    // A flush run while the application reads the window cannot drain it.
    source.with_window(|_, _| runtime.flush(Duration::from_secs(1)))??;
    assert!(
        runtime.next_deadline().is_some(),
        "the wake stays due for the next flush"
    );
    settle(&mut runtime, Duration::from_secs(2))?;
    let (_, telemetry) = *seen.borrow().last().ok_or("read")?;
    assert_eq!(telemetry.latest.map(|sample| sample.value), Some(5));
    Ok(())
}
