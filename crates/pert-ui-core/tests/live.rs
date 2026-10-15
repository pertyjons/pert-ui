//! Live-data source contracts: loss accounting, restart, resume, teardown and
//! validation, without a window, a runtime or, except where stated, a thread.

use pert_ui_core::{
    Capacity, Incarnation, LiveError, PublishOutcome, SeriesConfig, SeriesConsumer, SeriesEndpoint,
    Timestamp, WakeFlag, series_bridge,
};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn config(ring: usize, window: usize) -> Result<SeriesConfig, LiveError> {
    Ok(SeriesConfig::new(Capacity::new(window)?).with_ring_capacity(Capacity::new(ring)?))
}

fn series(
    ring: usize,
    window: usize,
) -> Result<(SeriesEndpoint<u64>, SeriesConsumer<u64>), LiveError> {
    Ok(series_bridge(config(ring, window)?))
}

fn values(consumer: &SeriesConsumer<u64>) -> Vec<(u64, bool)> {
    consumer
        .window()
        .iter()
        .map(|entry| (entry.sample.value, entry.discontinuity_before))
        .collect()
}

#[test]
fn capacity_rejects_invalid_values() -> TestResult {
    assert_eq!(Capacity::new(0), Err(LiveError::ZeroCapacity));
    assert_eq!(
        Capacity::new(Capacity::MAX.get() + 1),
        Err(LiveError::CapacityExceeded)
    );
    assert_eq!(Capacity::new(Capacity::MAX.get())?, Capacity::MAX);
    assert_eq!(Capacity::try_from(0), Err(LiveError::ZeroCapacity));
    assert_eq!(Capacity::try_from(8)?, Capacity::new(8)?);
    Ok(())
}

#[test]
fn series_delivers_in_order_with_identity_and_timestamps() -> TestResult {
    let (mut endpoint, mut consumer) = series(4, 8)?;
    for tick in 1..=3 {
        assert_eq!(
            endpoint.publish(tick * 10, Timestamp::new(tick * 100)),
            PublishOutcome::Accepted
        );
    }
    let report = consumer.drain();
    assert_eq!(
        (report.accepted, report.discarded, report.dropped),
        (3, 0, 0)
    );
    assert_eq!(report.missing, 0);
    let samples: Vec<_> = consumer
        .window()
        .iter()
        .map(|entry| {
            (
                entry.sample.value,
                entry.sample.sequence.get(),
                entry.sample.timestamp.get(),
                entry.sample.incarnation,
                entry.discontinuity_before,
            )
        })
        .collect();
    assert_eq!(
        samples,
        [
            (10, 1, 100, Incarnation::FIRST, false),
            (20, 2, 200, Incarnation::FIRST, false),
            (30, 3, 300, Incarnation::FIRST, false),
        ]
    );
    assert_eq!(
        consumer.drain().accepted,
        0,
        "a second drain finds nothing new"
    );
    Ok(())
}

#[test]
fn full_ring_drops_the_newest_and_reports_the_loss() -> TestResult {
    let (mut endpoint, mut consumer) = series(2, 8)?;
    assert_eq!(
        endpoint.publish(1, Timestamp::new(1)),
        PublishOutcome::Accepted
    );
    assert_eq!(
        endpoint.publish(2, Timestamp::new(2)),
        PublishOutcome::Accepted
    );
    assert_eq!(
        endpoint.publish(3, Timestamp::new(3)),
        PublishOutcome::Overflowed
    );
    assert_eq!(
        endpoint.publish(4, Timestamp::new(4)),
        PublishOutcome::Overflowed
    );
    assert_eq!(consumer.fill(), 2);
    let report = consumer.drain();
    assert_eq!((report.accepted, report.dropped), (2, 2));
    assert_eq!(report.missing, 2, "a trailing gap needs no later item");
    assert_eq!(consumer.dropped_total(), 2);
    // Room again: the next item follows the gap and is marked discontinuous.
    assert_eq!(
        endpoint.publish(5, Timestamp::new(5)),
        PublishOutcome::Accepted
    );
    let report = consumer.drain();
    assert_eq!((report.accepted, report.dropped), (1, 0));
    assert_eq!(report.missing, 0, "the loss was reported once");
    assert_eq!(values(&consumer), [(1, false), (2, false), (5, true)]);
    Ok(())
}

#[test]
fn invalid_float_is_rejected_and_consumes_a_sequence() -> TestResult {
    let config = config(4, 4)?;
    let (mut endpoint, mut consumer) = series_bridge::<f32>(config);
    assert_eq!(
        endpoint.publish(1.0, Timestamp::new(1)),
        PublishOutcome::Accepted
    );
    assert_eq!(
        endpoint.publish(f32::NAN, Timestamp::new(2)),
        PublishOutcome::Invalid
    );
    assert_eq!(
        endpoint.publish(f32::INFINITY, Timestamp::new(3)),
        PublishOutcome::Invalid
    );
    assert_eq!(
        endpoint.publish(4.0, Timestamp::new(4)),
        PublishOutcome::Accepted
    );
    let report = consumer.drain();
    assert_eq!(
        (report.accepted, report.rejected, report.dropped),
        (2, 2, 0)
    );
    assert_eq!(report.missing, 2);
    let window: Vec<_> = consumer
        .window()
        .iter()
        .map(|entry| (entry.sample.value, entry.discontinuity_before))
        .collect();
    assert_eq!(window, [(1.0, false), (4.0, true)]);
    assert_eq!(consumer.rejected_total(), 2);
    Ok(())
}

#[test]
fn window_evicts_the_oldest_and_keeps_capacity() -> TestResult {
    let (mut endpoint, mut consumer) = series(8, 3)?;
    for tick in 1..=5 {
        let _ = endpoint.publish(tick, Timestamp::new(tick));
    }
    consumer.drain();
    assert_eq!(consumer.window().len(), 3);
    assert_eq!(consumer.window().capacity(), 3);
    assert_eq!(values(&consumer), [(3, false), (4, false), (5, false)]);
    Ok(())
}

#[test]
fn restart_discards_queued_old_items_and_clears_the_window() -> TestResult {
    let (mut endpoint, mut consumer) = series(8, 8)?;
    let _ = endpoint.publish(1, Timestamp::new(1));
    consumer.drain();
    let _ = endpoint.publish(2, Timestamp::new(2));
    let next = endpoint.restart()?;
    assert_eq!(next.get(), 2);
    let _ = endpoint.publish(3, Timestamp::new(3));
    let report = consumer.drain();
    assert!(report.incarnation_changed);
    assert_eq!((report.accepted, report.discarded), (1, 1));
    assert_eq!(consumer.incarnation(), next);
    assert_eq!(values(&consumer), [(3, true)]);
    Ok(())
}

#[test]
fn restart_without_a_following_publish_is_observed() -> TestResult {
    let (mut endpoint, mut consumer) = series(8, 8)?;
    let _ = endpoint.publish(1, Timestamp::new(1));
    consumer.drain();
    assert_eq!(consumer.window().len(), 1);
    let _restarted = endpoint.restart()?;
    let report = consumer.drain();
    assert!(report.incarnation_changed);
    assert!(consumer.window().is_empty());
    Ok(())
}

#[test]
fn dropping_the_consumer_closes_the_endpoint() -> TestResult {
    let (mut endpoint, consumer) = series(2, 2)?;
    drop(consumer);
    assert_eq!(
        endpoint.publish(1, Timestamp::new(1)),
        PublishOutcome::Closed
    );
    Ok(())
}

#[test]
fn dropping_the_endpoint_delivers_everything_then_disconnects() -> TestResult {
    let (mut endpoint, mut consumer) = series(4, 4)?;
    let _ = endpoint.publish(1, Timestamp::new(1));
    let _ = endpoint.publish(2, Timestamp::new(2));
    drop(endpoint);
    let report = consumer.drain();
    assert!(report.disconnected);
    assert_eq!(report.accepted, 2);
    assert!(consumer.is_disconnected());
    Ok(())
}

#[test]
fn series_resume_discards_the_backlog_as_a_loss() -> TestResult {
    let (mut endpoint, mut consumer) = series(3, 8)?;
    let _ = endpoint.publish(1, Timestamp::new(1));
    consumer.drain();
    // Hidden: the ring fills, then rejects everything newer.
    for tick in 2..=6 {
        let _ = endpoint.publish(tick, Timestamp::new(tick));
    }
    let report = consumer.resume();
    assert_eq!((report.discarded, report.dropped), (3, 2));
    assert_eq!(report.missing, 5);
    assert!(consumer.window().is_empty());
    assert_eq!(consumer.fill(), 0);
    let _ = endpoint.publish(7, Timestamp::new(7));
    let report = consumer.drain();
    assert_eq!(report.accepted, 1);
    assert_eq!(report.missing, 0);
    assert_eq!(values(&consumer), [(7, true)]);
    Ok(())
}

#[test]
fn series_resume_after_a_quiet_suspension_keeps_the_window() -> TestResult {
    let (mut endpoint, mut consumer) = series(3, 8)?;
    let _ = endpoint.publish(1, Timestamp::new(1));
    let _ = endpoint.publish(2, Timestamp::new(2));
    consumer.drain();
    // Nothing was published while hidden, so nothing was lost.
    let report = *consumer.resume();
    assert_eq!(
        (report.discarded, report.missing, report.accepted),
        (0, 0, 0)
    );
    assert_eq!(values(&consumer), [(1, false), (2, false)]);
    // The next item continues the window without a break.
    let _ = endpoint.publish(3, Timestamp::new(3));
    consumer.drain();
    assert_eq!(values(&consumer), [(1, false), (2, false), (3, false)]);
    Ok(())
}

#[test]
fn series_resume_after_any_loss_or_restart_clears_the_window() -> TestResult {
    // A rejected attempt stores nothing but consumes a sequence number.
    let (mut endpoint, mut consumer) = series_bridge::<f32>(config(3, 8)?);
    let _ = endpoint.publish(1.0, Timestamp::new(1));
    consumer.drain();
    assert_eq!(
        endpoint.publish(f32::NAN, Timestamp::new(2)),
        PublishOutcome::Invalid
    );
    let report = *consumer.resume();
    assert_eq!((report.discarded, report.missing), (0, 1));
    assert!(consumer.window().is_empty());

    // A restart with no following publish belongs to a new incarnation.
    let (mut endpoint, mut consumer) = series(3, 8)?;
    let _ = endpoint.publish(1, Timestamp::new(1));
    consumer.drain();
    let _restarted = endpoint.restart()?;
    assert!(consumer.resume().incarnation_changed);
    assert!(consumer.window().is_empty());
    Ok(())
}

#[test]
fn wake_flag_coalesces_and_reports_pending() {
    let flag = WakeFlag::new();
    assert!(!flag.is_pending());
    assert!(flag.request());
    assert!(!flag.request());
    assert!(flag.is_pending());
    assert!(flag.reset());
    assert!(!flag.reset());
    assert!(flag.request());
}

#[test]
fn producer_thread_never_yields_a_torn_or_reordered_item() -> TestResult {
    const ITEMS: u64 = 20_000;
    let (mut endpoint, mut consumer) = series(64, 64)?;
    let producer = std::thread::spawn(move || {
        for tick in 1..=ITEMS {
            let _ = endpoint.publish(tick * 3, Timestamp::new(tick * 7));
        }
    });
    let mut seen = 0u64;
    let mut previous = 0u64;
    let mut gaps = 0u64;
    loop {
        let report = consumer.drain();
        gaps += report.missing;
        let disconnected = report.disconnected;
        let accepted = report.accepted as usize;
        for entry in consumer
            .window()
            .iter()
            .skip(consumer.window().len() - accepted)
        {
            let sequence = entry.sample.sequence.get();
            assert!(sequence > previous, "order");
            assert_eq!(
                entry.sample.value,
                sequence * 3,
                "value belongs to its sequence"
            );
            assert_eq!(
                entry.sample.timestamp.get(),
                sequence * 7,
                "timestamp belongs to its sequence"
            );
            previous = sequence;
            seen += 1;
        }
        if disconnected {
            break;
        }
        std::thread::yield_now();
    }
    producer.join().map_err(|_| "producer panicked")?;
    assert_eq!(
        seen + gaps,
        ITEMS,
        "every number is either delivered or reported lost"
    );
    assert_eq!(consumer.dropped_total(), gaps);
    Ok(())
}
