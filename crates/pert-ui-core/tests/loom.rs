//! Model checks of the live-data protocols under every interleaving loom
//! explores. Compiled only with `--cfg loom`:
//!
//! ```sh
//! RUSTFLAGS="--cfg loom" cargo test -p pert-ui-core --test loom --release
//! ```
//!
//! Each model is two threads and a handful of operations, which is what makes
//! exhaustive exploration feasible; the evidence is "no violation within the
//! explored bound", not a proof beyond it.
#![cfg(loom)]

use loom::thread;
use pert_ui_core::{
    Capacity, PublishOutcome, SeriesConfig, SeriesConsumer, Timestamp, WakeFlag, series_bridge,
};

fn config(ring: usize) -> SeriesConfig {
    SeriesConfig::new(Capacity::new(8).expect("window"))
        .with_ring_capacity(Capacity::new(ring).expect("ring"))
}

/// A slot is never read while it is written, items arrive in order, and every
/// number is either delivered or reported lost, including after disconnection.
#[test]
fn ring_delivers_intact_items_and_accounts_for_every_number() {
    loom::model(|| {
        let (mut endpoint, mut consumer) = series_bridge::<u64>(config(2));
        let producer = thread::spawn(move || {
            let mut dropped = 0;
            for tick in 1..=3u64 {
                if endpoint.publish(tick * 10, Timestamp::new(tick)) == PublishOutcome::Overflowed {
                    dropped += 1;
                }
            }
            dropped
        });
        let mut delivered = 0u64;
        let mut previous = 0u64;
        let mut lost = 0u64;
        let mut check = |consumer: &SeriesConsumer<u64>| {
            let report = consumer.report();
            lost += report.missing;
            let new = report.accepted as usize;
            for entry in consumer.window().iter().skip(consumer.window().len() - new) {
                let sequence = entry.sample.sequence.get();
                assert!(sequence > previous, "items arrive in order");
                assert_eq!(
                    entry.sample.value,
                    sequence * 10,
                    "value and sequence belong together"
                );
                assert_eq!(
                    entry.sample.timestamp.get(),
                    sequence,
                    "timestamp belongs too"
                );
                previous = sequence;
                delivered += 1;
            }
        };
        consumer.drain();
        check(&consumer);
        let dropped = producer.join().expect("producer");
        consumer.drain();
        check(&consumer);
        assert!(
            consumer.report().disconnected,
            "the drain after the drop observes it"
        );
        assert_eq!(
            delivered + lost,
            3,
            "every number is delivered or reported lost"
        );
        assert_eq!(
            lost, dropped,
            "reported loss equals what the producer saw dropped"
        );
    });
}

/// A publish that raced the reset is either drained now or leaves a wake pending.
#[test]
fn wake_flag_never_loses_a_publish() {
    loom::model(|| {
        let flag = loom::sync::Arc::new(WakeFlag::new());
        // An earlier wake is still pending when the producer publishes.
        assert!(flag.request());
        let (mut endpoint, mut consumer) = series_bridge::<u64>(config(2));
        let producer = {
            let flag = loom::sync::Arc::clone(&flag);
            thread::spawn(move || {
                assert_eq!(
                    endpoint.publish(1, Timestamp::new(1)),
                    PublishOutcome::Accepted
                );
                // Publish first, then request: the caller told to wake would deliver it.
                let must_wake = flag.request();
                (endpoint, must_wake)
            })
        };
        // The UI handles the earlier wake: reset, then drain.
        flag.reset();
        consumer.drain();
        let drained = consumer.report().accepted == 1;
        let (_endpoint, must_wake) = producer.join().expect("producer");
        assert!(
            drained || must_wake || flag.is_pending(),
            "an undrained publish leaves a wake to deliver"
        );
    });
}

/// Checks what every drain must leave behind: this drain's items fit in the
/// window, and nothing in it belongs to another incarnation than the
/// consumer's or separates a value from its sequence.
fn check_window(consumer: &SeriesConsumer<u64>) {
    let window = consumer.window();
    assert!(
        consumer.report().accepted <= window.len() as u64,
        "accepted never exceeds what the window holds when ring <= window"
    );
    for entry in window.iter() {
        assert_eq!(
            entry.sample.incarnation,
            consumer.incarnation(),
            "no item of another incarnation shares the window"
        );
        assert_eq!(entry.sample.value, entry.sample.sequence.get() * 10);
    }
}

/// A restart racing a drain never files an item under the wrong incarnation,
/// keeps `accepted` within the window, and counts every discarded item as
/// missing exactly once.
#[test]
fn restart_racing_a_drain_never_mixes_incarnations() {
    loom::model(|| {
        let (mut endpoint, mut consumer) = series_bridge::<u64>(config(2));
        let producer = thread::spawn(move || {
            assert_eq!(
                endpoint.publish(10, Timestamp::new(1)),
                PublishOutcome::Accepted
            );
            let second = endpoint.restart().expect("restart");
            assert_eq!(
                endpoint.publish(20, Timestamp::new(2)),
                PublishOutcome::Accepted
            );
            (endpoint, second)
        });
        let (mut discarded, mut missing) = (0, 0);
        consumer.drain();
        check_window(&consumer);
        discarded += consumer.report().discarded;
        missing += consumer.report().missing;
        let (_endpoint, second) = producer.join().expect("producer");
        consumer.drain();
        check_window(&consumer);
        discarded += consumer.report().discarded;
        missing += consumer.report().missing;
        assert_eq!(consumer.incarnation(), second, "the restart is observed");
        let sequences: Vec<u64> = consumer
            .window()
            .iter()
            .map(|entry| entry.sample.sequence.get())
            .collect();
        assert_eq!(sequences, [2], "only the new incarnation's item remains");
        assert!(discarded <= 1);
        assert_eq!(
            missing, discarded,
            "a discarded item is missing exactly once"
        );
    });
}

/// A resume racing a publish keeps the window only when nothing was lost:
/// the drained item stays with a lossless report, or the window is emptied
/// with a reported loss. The window never joins the kept item to one the
/// resume discarded, and every number is delivered or counted missing, once.
#[test]
fn resume_racing_a_publish_accounts_for_every_number_once() {
    loom::model(|| {
        let (mut endpoint, mut consumer) = series_bridge::<u64>(config(2));
        assert_eq!(
            endpoint.publish(10, Timestamp::new(1)),
            PublishOutcome::Accepted
        );
        let first = *consumer.drain();
        assert_eq!((first.accepted, first.missing), (1, 0));
        let producer = thread::spawn(move || {
            for tick in 2..=3u64 {
                assert_eq!(
                    endpoint.publish(tick * 10, Timestamp::new(tick)),
                    PublishOutcome::Accepted,
                    "a ring of two holds both"
                );
            }
            endpoint
        });
        let resumed = *consumer.resume();
        let lossless = resumed.discarded == 0
            && resumed.missing == 0
            && resumed.rejected == 0
            && resumed.dropped == 0;
        let kept = sequences(&consumer);
        if lossless {
            assert_eq!(kept, [1], "a lossless resume keeps the drained item");
        } else {
            assert!(
                kept.is_empty(),
                "a resume that lost items clears the window"
            );
        }
        let (mut accepted, mut missing, mut discarded) = (
            first.accepted + resumed.accepted,
            resumed.missing,
            resumed.discarded,
        );
        let _endpoint = producer.join().expect("producer");
        let report = *consumer.drain();
        check_window(&consumer);
        accepted += report.accepted;
        missing += report.missing;
        discarded += report.discarded;
        let window = sequences(&consumer);
        for pair in window.windows(2) {
            assert_eq!(
                pair[1],
                pair[0] + 1,
                "the window never joins an item to one past a discarded item"
            );
        }
        if lossless {
            assert_eq!(window, [1, 2, 3], "nothing was lost, so all three remain");
        } else {
            assert_ne!(window.first(), Some(&1), "the pre-resume item left");
        }
        assert_eq!(accepted + missing, 3, "delivered or missing, never both");
        assert_eq!(discarded, missing, "only the discarded backlog is missing");
    });
}

/// Returns the sequence numbers the window holds, oldest first.
fn sequences(consumer: &SeriesConsumer<u64>) -> Vec<u64> {
    consumer
        .window()
        .iter()
        .map(|entry| entry.sample.sequence.get())
        .collect()
}

/// Dropping the consumer while the producer publishes closes the bridge: an
/// attempt either completes into the ring or reports `Closed`, and once
/// closed the endpoint stays closed. Loom also checks that the shared storage
/// is freed once, by whichever half is dropped last.
#[test]
fn consumer_drop_racing_a_publish_closes_the_bridge() {
    loom::model(|| {
        let (mut endpoint, consumer) = series_bridge::<u64>(config(2));
        let producer = thread::spawn(move || {
            let first = endpoint.publish(10, Timestamp::new(1));
            let second = endpoint.publish(20, Timestamp::new(2));
            (endpoint, first, second)
        });
        drop(consumer);
        let (mut endpoint, first, second) = producer.join().expect("producer");
        for outcome in [first, second] {
            assert!(
                matches!(outcome, PublishOutcome::Accepted | PublishOutcome::Closed),
                "{outcome:?}"
            );
        }
        if first == PublishOutcome::Closed {
            assert_eq!(second, PublishOutcome::Closed, "closed stays closed");
        }
        assert_eq!(
            endpoint.publish(30, Timestamp::new(3)),
            PublishOutcome::Closed,
            "a publish after the drop is observed as closed"
        );
    });
}
