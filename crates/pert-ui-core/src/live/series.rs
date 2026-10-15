//! The series bridge: a single-producer, single-consumer ring of items.

use std::{cell::Cell, marker::PhantomData};

use super::{
    DrainReport, Incarnation, LiveError, LiveItem, PublishOutcome, Sample, SampleWindow, Sequence,
    SeriesConfig, Timestamp,
    sync::{Arc, AtomicBool, AtomicU64, Ordering},
};

/// One ring slot; every field is an atomic so a slot is never torn.
struct Slot {
    value: AtomicU64,
    sequence: AtomicU64,
    timestamp: AtomicU64,
    incarnation: AtomicU64,
}

impl Slot {
    fn new() -> Self {
        Self {
            value: AtomicU64::new(0),
            sequence: AtomicU64::new(0),
            timestamp: AtomicU64::new(0),
            incarnation: AtomicU64::new(0),
        }
    }
}

struct Shared {
    slots: Box<[Slot]>,
    /// Items ever published into a slot; the producer's counter.
    written: AtomicU64,
    /// Items ever consumed out of a slot; the consumer's counter.
    read: AtomicU64,
    /// The sequence of the last attempt whose outcome is committed.
    last_assigned: AtomicU64,
    dropped: AtomicU64,
    rejected: AtomicU64,
    incarnation: AtomicU64,
    closed: AtomicBool,
    disconnected: AtomicBool,
    exhausted: AtomicBool,
}

/// Creates a series bridge and returns its producer and consumer halves.
///
/// The ring is preallocated here, on the UI thread, at the configured
/// capacity; nothing allocates afterwards on the producer side. The endpoint
/// is `Send` and moves to the producer thread; the consumer stays on the UI
/// thread. Dropping either half is observed by the other, as documented on
/// [`SeriesEndpoint`] and [`SeriesConsumer`].
///
/// ```
/// use pert_ui_core::{Capacity, SeriesConfig, Timestamp, series_bridge};
///
/// let config = SeriesConfig::new(Capacity::new(64)?);
/// let (mut endpoint, mut consumer) = series_bridge::<f32>(config);
///
/// // On the producer thread: publish with bounded work and no allocation.
/// for tick in 1..=3 {
///     let _ = endpoint.publish(tick as f32 * 0.5, Timestamp::new(tick));
/// }
///
/// // On the UI thread: drain once per presentation deadline.
/// let report = consumer.drain();
/// assert_eq!(report.accepted, 3);
/// assert_eq!(report.missing, 0);
/// let values: Vec<f32> = consumer.window().iter().map(|entry| entry.sample.value).collect();
/// assert_eq!(values, [0.5, 1.0, 1.5]);
/// # Ok::<(), pert_ui_core::LiveError>(())
/// ```
pub fn series_bridge<T: LiveItem>(config: SeriesConfig) -> (SeriesEndpoint<T>, SeriesConsumer<T>) {
    let capacity = config.ring_capacity().get();
    let shared = Arc::new(Shared {
        slots: (0..capacity).map(|_| Slot::new()).collect(),
        written: AtomicU64::new(0),
        read: AtomicU64::new(0),
        last_assigned: AtomicU64::new(0),
        dropped: AtomicU64::new(0),
        rejected: AtomicU64::new(0),
        incarnation: AtomicU64::new(Incarnation::FIRST.get()),
        closed: AtomicBool::new(false),
        disconnected: AtomicBool::new(false),
        exhausted: AtomicBool::new(false),
    });
    let endpoint = SeriesEndpoint {
        shared: Arc::clone(&shared),
        capacity: capacity as u64,
        written: 0,
        last_sequence: None,
        incarnation: Incarnation::FIRST,
        exhausted: false,
        _item: PhantomData,
    };
    let consumer = SeriesConsumer {
        shared,
        config,
        capacity: capacity as u64,
        read: 0,
        incarnation: Incarnation::FIRST,
        next_expected: Some(Sequence::FIRST.get()),
        break_pending: false,
        window: SampleWindow::new(config.window_capacity()),
        report: DrainReport::default(),
        rejected_seen: 0,
        dropped_seen: 0,
        disconnected: false,
        exhausted: false,
    };
    (endpoint, consumer)
}

/// The producer half of a series bridge.
///
/// It is `Send` and not `Sync`: exactly one thread publishes at a time, and
/// every method takes `&mut self`, so a publish can never race a restart.
/// [`Self::publish`] is the real-time path; [`Self::restart`] and dropping the
/// endpoint are not, and a host with a real-time producer performs both from a
/// non-real-time thread, because the shared storage is freed by whichever half
/// is dropped last.
///
/// The publish path performs, in this order and nothing else: a branch on the
/// endpoint's own exhausted flag; an acquire load of the closed flag; the
/// checked increment of the sequence, which at its limit instead stores the
/// shared exhausted flag with release ordering and returns; for a float, one
/// finiteness branch, after which a rejected item takes a relaxed `fetch_add`
/// of the rejected total and a release store of the last assigned sequence
/// and returns; an acquire load of the consumer's read counter and the
/// full-ring comparison, after which a dropped item takes a relaxed
/// `fetch_add` of the dropped total and a release store of the last assigned
/// sequence and returns; otherwise four relaxed stores into the slot, a
/// release store of the written counter and a release store of the last
/// assigned sequence. It has nothing to allocate and nothing whose drop could
/// free memory. That is reviewed against this list, not measured by an
/// allocator.
#[must_use = "dropping the endpoint disconnects the source"]
pub struct SeriesEndpoint<T> {
    shared: Arc<Shared>,
    capacity: u64,
    written: u64,
    last_sequence: Option<Sequence>,
    incarnation: Incarnation,
    exhausted: bool,
    _item: PhantomData<Cell<T>>,
}

impl<T: LiveItem> SeriesEndpoint<T> {
    /// Publishes one item with bounded work and no allocation.
    ///
    /// Every attempt consumes the next sequence number before admission, so
    /// the consumer can report exactly which numbers it never received. The
    /// outcome is committed, a slot published or a loss counted, before the
    /// number becomes visible as assigned, which is what lets the consumer
    /// report a trailing loss without ever reporting an item that is still on
    /// its way.
    ///
    /// A full ring drops this item rather than evicting an older one: eviction
    /// from the producer side would need coordination with a concurrent reader,
    /// and a display path prefers a provably cheap producer over recency of
    /// the buffered tail. Buffered latency is bounded by the ring; the number
    /// of items a stalled consumer loses is not. The dropped total cannot
    /// wrap: every dropped item consumed a sequence number, and the sequence
    /// closes the endpoint as exhausted instead of wrapping.
    pub fn publish(&mut self, value: T, timestamp: Timestamp) -> PublishOutcome {
        if self.exhausted {
            return PublishOutcome::Exhausted;
        }
        if self.shared.closed.load(Ordering::Acquire) {
            return PublishOutcome::Closed;
        }
        let Some(sequence) = self
            .last_sequence
            .map_or(Some(Sequence::FIRST), Sequence::next)
        else {
            return self.exhaust();
        };
        self.last_sequence = Some(sequence);
        if !value.is_valid() {
            self.shared.rejected.fetch_add(1, Ordering::Relaxed);
            self.shared
                .last_assigned
                .store(sequence.get(), Ordering::Release);
            return PublishOutcome::Invalid;
        }
        let read = self.shared.read.load(Ordering::Acquire);
        if self.written.wrapping_sub(read) >= self.capacity {
            self.shared.dropped.fetch_add(1, Ordering::Relaxed);
            self.shared
                .last_assigned
                .store(sequence.get(), Ordering::Release);
            return PublishOutcome::Overflowed;
        }
        // The ring never holds more than `capacity` items, so the index fits.
        let slot = &self.shared.slots[(self.written % self.capacity) as usize];
        slot.value.store(value.to_bits(), Ordering::Relaxed);
        slot.sequence.store(sequence.get(), Ordering::Relaxed);
        slot.timestamp.store(timestamp.get(), Ordering::Relaxed);
        slot.incarnation
            .store(self.incarnation.get(), Ordering::Relaxed);
        self.written += 1;
        self.shared.written.store(self.written, Ordering::Release);
        self.shared
            .last_assigned
            .store(sequence.get(), Ordering::Release);
        PublishOutcome::Accepted
    }

    /// Declares a new incarnation after a reconnect or a new stream.
    ///
    /// Not a real-time operation. Every item published afterwards carries the
    /// new incarnation; the consumer observes the change at its next drain even
    /// when no item follows, discards what still carries the old one and clears
    /// its window.
    ///
    /// # Errors
    /// Returns [`LiveError::Exhausted`] when the incarnation counter cannot
    /// advance; the endpoint is then terminally closed.
    pub fn restart(&mut self) -> Result<Incarnation, LiveError> {
        if self.exhausted {
            return Err(LiveError::Exhausted);
        }
        let Some(next) = self.incarnation.next() else {
            let _ = self.exhaust();
            return Err(LiveError::Exhausted);
        };
        self.incarnation = next;
        self.shared.incarnation.store(next.get(), Ordering::Release);
        Ok(next)
    }

    /// Returns the incarnation items are currently published in.
    pub const fn incarnation(&self) -> Incarnation {
        self.incarnation
    }

    /// Returns the sequence of the last publish attempt, if any.
    #[must_use]
    pub const fn last_sequence(&self) -> Option<Sequence> {
        self.last_sequence
    }

    fn exhaust(&mut self) -> PublishOutcome {
        self.exhausted = true;
        self.shared.exhausted.store(true, Ordering::Release);
        PublishOutcome::Exhausted
    }
}

impl<T> Drop for SeriesEndpoint<T> {
    /// Marks the source disconnected after everything already published.
    fn drop(&mut self) {
        self.shared.disconnected.store(true, Ordering::Release);
    }
}

impl<T> core::fmt::Debug for SeriesEndpoint<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("SeriesEndpoint")
            .field("written", &self.written)
            .field("last_sequence", &self.last_sequence)
            .field("incarnation", &self.incarnation)
            .field("exhausted", &self.exhausted)
            .finish_non_exhaustive()
    }
}

/// The consumer half of a series bridge.
///
/// The type is `Send` and `Sync`, but the protocol has one consumer, and
/// every method that moves data takes `&mut self`, so two drains never race.
/// The runtime keeps it on the UI thread, wrapped in a handle that schedules
/// drains and tracks subscriptions; that placement is the runtime's
/// convention, not a guarantee of this type. Each
/// [`Self::drain`] moves at most one ring of items into the retained
/// [`SampleWindow`], reports every loss it can prove, and never waits for the
/// producer. Dropping the consumer closes the bridge: the endpoint's next
/// publish returns [`PublishOutcome::Closed`].
///
/// A drain performs, in this order: an acquire load of the incarnation; an
/// acquire load of the disconnected flag; an acquire load of the exhausted
/// flag; an acquire load of the last assigned sequence; one acquire load of
/// the written counter, the drain frontier; relaxed loads of every slot up to
/// it; a release store of the read counter, only after the last slot field
/// was read; then relaxed loads of the rejected and dropped totals, which is
/// why those counts can run one drain ahead of the missing numbers they
/// correspond to (see [`DrainReport`]). The producer acquires
/// that counter before it overwrites a slot, so a slot is never read while it
/// is written, and a drain that observed the disconnected flag also observes
/// every item published before it.
#[must_use = "dropping the consumer closes the bridge"]
pub struct SeriesConsumer<T> {
    shared: Arc<Shared>,
    config: SeriesConfig,
    capacity: u64,
    read: u64,
    incarnation: Incarnation,
    // The lowest sequence not yet accounted for, or `None` once `u64::MAX` has
    // been: the counter cannot advance, so nothing further can be expected and
    // the consumer reports the source as exhausted.
    next_expected: Option<u64>,
    break_pending: bool,
    window: SampleWindow<T>,
    report: DrainReport,
    rejected_seen: u64,
    dropped_seen: u64,
    disconnected: bool,
    exhausted: bool,
}

impl<T: LiveItem> SeriesConsumer<T> {
    /// Moves what the producer published since the previous drain into the window.
    ///
    /// Bounded by the ring capacity. Items of an older incarnation than the
    /// current one are discarded; a newer incarnation, whether declared by a
    /// restart or carried by an item, clears the window first. The returned
    /// report is valid until the next drain or resume.
    pub fn drain(&mut self) -> &DrainReport {
        self.report.clear();
        let (last_assigned, frontier) = self.observe();
        let mut max_seen = self.next_expected.map_or(u64::MAX, |next| next - 1);
        while self.read < frontier {
            let slot = &self.shared.slots[(self.read % self.capacity) as usize];
            let value = T::from_bits(slot.value.load(Ordering::Relaxed));
            let sequence = slot.sequence.load(Ordering::Relaxed);
            let timestamp = Timestamp::new(slot.timestamp.load(Ordering::Relaxed));
            let incarnation = slot.incarnation.load(Ordering::Relaxed);
            self.read += 1;
            let (Some(sequence), Some(incarnation)) =
                (Sequence::new(sequence), Incarnation::from_raw(incarnation))
            else {
                self.discard(1);
                continue;
            };
            if incarnation < self.incarnation {
                self.discard(1);
                continue;
            }
            if incarnation > self.incarnation {
                self.begin_incarnation(incarnation);
            }
            if let Some(expected) = self.next_expected
                && sequence.get() > expected
            {
                self.report.record_missing(expected, sequence.get() - 1);
                self.break_pending = true;
            }
            self.window.push(
                Sample {
                    value,
                    incarnation,
                    sequence,
                    timestamp,
                },
                self.break_pending,
            );
            self.break_pending = false;
            self.report.accepted += 1;
            max_seen = max_seen.max(sequence.get());
            self.next_expected = sequence.get().checked_add(1);
        }
        self.shared.read.store(self.read, Ordering::Release);
        self.trailing_loss(last_assigned.max(max_seen));
        self.finish();
        &self.report
    }

    /// Discards everything in the ring without reading it, on resume.
    ///
    /// The runtime calls this at the first drain after a source regained a
    /// visible subscriber: the backlog is not recent, so it is thrown away by
    /// advancing the read counter to the frontier, counted as discarded and
    /// missing. When anything was lost during the suspension — an item
    /// discarded here, a missing sequence number, or an attempt the producer
    /// rejected or dropped — the window is cleared and restarts with what
    /// arrives next, so no trace joins items across the loss. When nothing
    /// was lost, the producer published nothing since the last drain, so the
    /// window still ends at the newest item and is kept; the next item
    /// continues it without a break, as after any quiet period. A new
    /// incarnation clears the window either way. A disconnection that
    /// happened during the suspension is still reported afterwards.
    pub fn resume(&mut self) -> &DrainReport {
        self.report.clear();
        let (last_assigned, frontier) = self.observe();
        let discarded = frontier - self.read;
        self.read = frontier;
        self.shared.read.store(self.read, Ordering::Release);
        self.discard(discarded);
        self.trailing_loss(last_assigned);
        self.finish();
        let report = &self.report;
        if report.discarded > 0 || report.missing > 0 || report.rejected > 0 || report.dropped > 0 {
            self.window.clear();
        }
        &self.report
    }

    /// Returns the retained items.
    #[must_use]
    pub const fn window(&self) -> &SampleWindow<T> {
        &self.window
    }

    /// Returns the report of the most recent drain or resume.
    #[must_use]
    pub const fn report(&self) -> &DrainReport {
        &self.report
    }

    /// Returns the bridge configuration.
    pub const fn config(&self) -> &SeriesConfig {
        &self.config
    }

    /// Returns the incarnation the window belongs to.
    pub const fn incarnation(&self) -> Incarnation {
        self.incarnation
    }

    /// Returns how many items are waiting in the ring, for inspection.
    #[must_use]
    pub fn fill(&self) -> u64 {
        self.shared.written.load(Ordering::Acquire) - self.read
    }

    /// Returns how many items the producer has dropped so far, for inspection.
    #[must_use]
    pub fn dropped_total(&self) -> u64 {
        self.shared.dropped.load(Ordering::Relaxed)
    }

    /// Returns how many items the producer has rejected so far, for inspection.
    #[must_use]
    pub fn rejected_total(&self) -> u64 {
        self.shared.rejected.load(Ordering::Relaxed)
    }

    /// Reports whether the endpoint has been dropped, as of the last drain.
    #[must_use]
    pub const fn is_disconnected(&self) -> bool {
        self.disconnected
    }

    /// Reports whether a protocol counter reached its limit, as of the last
    /// drain: the endpoint closed itself, or the final sequence number has been
    /// accounted for, so nothing further can be delivered.
    #[must_use]
    pub const fn is_exhausted(&self) -> bool {
        self.exhausted
    }

    /// Performs the ordered loads that start every drain and returns the last
    /// assigned sequence and the frontier.
    fn observe(&mut self) -> (u64, u64) {
        let incarnation = self.shared.incarnation.load(Ordering::Acquire);
        if let Some(incarnation) = Incarnation::from_raw(incarnation)
            && incarnation > self.incarnation
        {
            self.begin_incarnation(incarnation);
        }
        self.disconnected = self.shared.disconnected.load(Ordering::Acquire);
        self.exhausted = self.shared.exhausted.load(Ordering::Acquire);
        let last_assigned = self.shared.last_assigned.load(Ordering::Acquire);
        let frontier = self.shared.written.load(Ordering::Acquire);
        (last_assigned, frontier)
    }

    fn begin_incarnation(&mut self, incarnation: Incarnation) {
        self.incarnation = incarnation;
        self.window.clear();
        // Items this drain already delivered left with the cleared window;
        // `accepted` counts only what the drain leaves in it.
        self.report.accepted = 0;
        self.break_pending = true;
        self.report.incarnation_changed = true;
    }

    fn discard(&mut self, count: u64) {
        if count > 0 {
            self.report.discarded += count;
            self.break_pending = true;
        }
    }

    fn trailing_loss(&mut self, bound: u64) {
        if let Some(expected) = self.next_expected
            && bound >= expected
        {
            self.report.record_missing(expected, bound);
            self.break_pending = true;
            self.next_expected = bound.checked_add(1);
        }
    }

    fn finish(&mut self) {
        let rejected = self.shared.rejected.load(Ordering::Relaxed);
        let dropped = self.shared.dropped.load(Ordering::Relaxed);
        self.report.rejected = rejected.saturating_sub(self.rejected_seen);
        self.report.dropped = dropped.saturating_sub(self.dropped_seen);
        self.rejected_seen = rejected;
        self.dropped_seen = dropped;
        // The producer flags exhaustion only at the publish after its last
        // sequence; the consumer's own counter reaching its limit is the same
        // terminal state, observed one step earlier.
        self.exhausted |= self.next_expected.is_none();
        self.report.disconnected = self.disconnected;
        self.report.exhausted = self.exhausted;
    }
}

impl<T> Drop for SeriesConsumer<T> {
    /// Closes the bridge; the endpoint's next publish returns `Closed`.
    fn drop(&mut self) {
        self.shared.closed.store(true, Ordering::Release);
    }
}

impl<T: LiveItem> core::fmt::Debug for SeriesConsumer<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("SeriesConsumer")
            .field("config", &self.config)
            .field("read", &self.read)
            .field("incarnation", &self.incarnation)
            .field("next_expected", &self.next_expected)
            .field("window_len", &self.window.len())
            .field("disconnected", &self.disconnected)
            .field("exhausted", &self.exhausted)
            .finish_non_exhaustive()
    }
}

#[cfg(all(test, not(loom)))]
mod tests {
    use super::*;
    use crate::Capacity;

    fn near_limit() -> (SeriesEndpoint<f32>, SeriesConsumer<f32>) {
        let capacity = Capacity::new(8).expect("a valid capacity");
        let (mut endpoint, mut consumer) = series_bridge::<f32>(SeriesConfig::new(capacity));
        // Reaching the limit by publishing would take centuries, so the test
        // places both sides two steps before it.
        endpoint.last_sequence = Sequence::new(u64::MAX - 2);
        consumer.next_expected = Some(u64::MAX - 1);
        (endpoint, consumer)
    }

    #[test]
    fn the_final_sequence_is_delivered_and_exhausts_both_sides_without_wrapping() {
        let (mut endpoint, mut consumer) = near_limit();
        assert_eq!(
            endpoint.publish(1.0, Timestamp::new(1)),
            PublishOutcome::Accepted
        );
        assert_eq!(
            endpoint.publish(2.0, Timestamp::new(2)),
            PublishOutcome::Accepted
        );
        assert_eq!(endpoint.last_sequence().map(Sequence::get), Some(u64::MAX));
        let report = *consumer.drain();
        assert_eq!((report.accepted, report.missing), (2, 0));
        assert!(report.exhausted && consumer.is_exhausted());
        assert_eq!(consumer.next_expected, None);
        // The producer's counter cannot advance either, and says so terminally.
        assert_eq!(
            endpoint.publish(3.0, Timestamp::new(3)),
            PublishOutcome::Exhausted
        );
        let report = *consumer.drain();
        assert_eq!((report.accepted, report.missing), (0, 0));
        assert!(report.exhausted);
    }

    #[test]
    fn an_incarnation_at_its_limit_closes_the_source_instead_of_wrapping() {
        let (mut endpoint, mut consumer) = near_limit();
        endpoint.incarnation = Incarnation::from_raw(u64::MAX).expect("nonzero");
        assert_eq!(endpoint.restart(), Err(LiveError::Exhausted));
        assert_eq!(
            endpoint.publish(1.0, Timestamp::new(1)),
            PublishOutcome::Exhausted
        );
        let report = *consumer.drain();
        assert_eq!(report.accepted, 0);
        assert!(report.exhausted && consumer.is_exhausted());
    }

    #[test]
    fn a_final_sequence_that_was_lost_is_counted_once() {
        let (mut endpoint, mut consumer) = near_limit();
        assert_eq!(
            endpoint.publish(f32::NAN, Timestamp::new(1)),
            PublishOutcome::Invalid
        );
        assert_eq!(
            endpoint.publish(f32::NAN, Timestamp::new(2)),
            PublishOutcome::Invalid
        );
        let report = *consumer.drain();
        assert_eq!((report.accepted, report.missing), (0, 2));
        assert!(report.exhausted);
        // A second drain and a resume must not count the same loss again.
        assert_eq!(consumer.drain().missing, 0);
        assert_eq!(consumer.resume().missing, 0);
    }
}
