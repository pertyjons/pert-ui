//! Live-data contracts with a real producer thread: each drain stays within the
//! ring capacity, and what a starved consumer lost is reported, not hidden.
//!
//! The assertions are the documented invariants, not timings: which items
//! arrive in which drain depends on scheduling, but no drain may exceed the
//! ring, and every publish attempt must end up either delivered or missing.
//! The loom models in `loom.rs` are the memory-ordering evidence; this test
//! runs the same protocol across real threads and a real race.
#![cfg(not(loom))]

use pert_ui_core::{Capacity, PublishOutcome, SeriesConfig, Timestamp, series_bridge};
use std::{
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const RING: usize = 64;
/// Published while the consumer does not drain at all.
const STARVED: u64 = 1_000;
/// Published while the consumer drains concurrently.
const RUNNING: u64 = 200_000;
/// A generous bound on the whole run, so a hang fails instead of blocking CI.
const DEADLINE: Duration = Duration::from_secs(60);

#[test]
fn a_real_producer_is_drained_in_bounded_steps_and_a_starved_consumer_sees_its_loss() -> TestResult
{
    let config = SeriesConfig::new(Capacity::new(RING)?).with_ring_capacity(Capacity::new(RING)?);
    let (mut endpoint, mut consumer) = series_bridge::<u64>(config);
    let (starved, starved_done) = mpsc::channel();
    let producer = thread::spawn(move || {
        let (mut accepted, mut overflowed) = (0u64, 0u64);
        for n in 1..=STARVED + RUNNING {
            match endpoint.publish(n, Timestamp::new(n)) {
                PublishOutcome::Accepted => accepted += 1,
                PublishOutcome::Overflowed => overflowed += 1,
                other => return Err(format!("unexpected outcome {other:?}")),
            }
            if n == STARVED {
                starved.send(()).map_err(|error| error.to_string())?;
            }
        }
        // Dropping the endpoint here is what the consumer observes as the end.
        Ok((accepted, overflowed))
    });

    // The consumer has not drained while the producer published far more than
    // the ring holds; the first drain takes exactly one ring and reports the rest.
    starved_done.recv_timeout(DEADLINE)?;
    let first = *consumer.drain();
    assert_eq!(
        first.accepted, RING as u64,
        "one drain takes at most the ring"
    );
    assert!(first.dropped >= STARVED - RING as u64, "{first:?}");
    assert!(first.missing >= STARVED - RING as u64, "{first:?}");

    let (mut accepted, mut dropped, mut missing) = (first.accepted, first.dropped, first.missing);
    let mut break_checked = false;
    let started = Instant::now();
    loop {
        let report = *consumer.drain();
        assert!(report.accepted <= RING as u64, "{report:?}");
        assert_eq!(report.rejected, 0);
        assert_eq!(report.discarded, 0);
        accepted += report.accepted;
        dropped += report.dropped;
        missing += report.missing;
        // The first item delivered after the starved gap is marked as a break.
        if !break_checked && report.accepted > 0 {
            let window = consumer.window();
            let first_new = window.len() - report.accepted as usize;
            let entry = window.iter().nth(first_new).ok_or("first new item")?;
            assert!(entry.discontinuity_before, "{entry:?}");
            break_checked = true;
        }
        if report.disconnected {
            break;
        }
        assert!(started.elapsed() < DEADLINE, "the producer never finished");
        thread::yield_now();
    }

    let (published, overflowed) = producer.join().map_err(|_| "producer panicked")??;
    assert!(break_checked, "nothing arrived after the gap");
    // Every attempt consumed a sequence number and is either delivered or missing.
    assert_eq!(accepted + missing, STARVED + RUNNING);
    assert_eq!(accepted, published);
    assert_eq!(dropped, overflowed);
    assert_eq!(consumer.dropped_total(), overflowed);
    Ok(())
}
