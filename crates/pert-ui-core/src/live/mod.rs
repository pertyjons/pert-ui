//! Bounded, lossy, freshness-aware delivery of live data from a producer with
//! its own timing to the UI.
//!
//! A host creates a [bridge](series_bridge) on the UI thread and receives two
//! halves: an endpoint, which is `Send` and publishes from one producer thread
//! with bounded work and no allocation, and a consumer, which drains a fixed
//! ring into a retained [`SampleWindow`] and reports exactly what was lost. The
//! bridge names no application domain: an item is a plain number and its
//! [`Timestamp`] counts whatever the producer's clock counts, so a host maps
//! its meaning onto them the way it maps a quantity onto a numeric parameter.
//!
//! Presentation policy belongs to the host, not to the bridge. A level indicator is
//! this bridge with a small window, which the widget reduces when it reads in
//! its update; a trace is the same bridge with a window as wide as the trace. The
//! library has no opinion about peaks, averages, producer cadence or units, and
//! a producer that must never lose a peak folds its own items before it
//! publishes them.
//!
//! The protocol is stated in the rustdoc of the halves that run it, step by
//! step, because it is the contract a real-time producer relies on; the design
//! note `docs/contracts.md` in the repository records why. The
//! UI-thread side that schedules drains and tracks subscriptions belongs to the
//! runtime, which builds on these types.

mod item;
mod series;
mod sync;
mod wake;

use std::{collections::VecDeque, num::NonZeroU64};

pub use item::LiveItem;
pub use series::{SeriesConsumer, SeriesEndpoint, series_bridge};
pub use wake::WakeFlag;

/// A producer's restart generation, so old and new data never share a window.
///
/// The endpoint owns it: [`SeriesEndpoint::restart`] advances it after a
/// reconnect or a new stream, and every item published afterwards carries the
/// new value. A consumer that meets a newer incarnation clears its retained
/// state and discards what still carries the old one; it never merges across
/// the boundary. This is the rule [`ParameterId`](crate::ParameterId) applies
/// to a recreated parameter, with the incarnation owned by the side that knows
/// about the restart.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[must_use]
pub struct Incarnation(NonZeroU64);

impl Incarnation {
    /// The incarnation every bridge starts in.
    pub const FIRST: Self = Self(NonZeroU64::MIN);

    /// Returns the generation number; `FIRST` is 1 and a restart adds one.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0.get()
    }

    pub(crate) const fn next(self) -> Option<Self> {
        match self.0.checked_add(1) {
            Some(next) => Some(Self(next)),
            None => None,
        }
    }

    pub(crate) const fn from_raw(raw: u64) -> Option<Self> {
        match NonZeroU64::new(raw) {
            Some(raw) => Some(Self(raw)),
            None => None,
        }
    }
}

/// The number an endpoint assigns to every publish attempt, in order.
///
/// Sequences start at [`Self::FIRST`] and grow by one per attempt for the
/// endpoint's lifetime, across restarts, whether the attempt was accepted,
/// overflowed or rejected. A number the consumer never receives is therefore a
/// loss, and the consumer can tell an omission from a reordering. The sequence
/// is also the item's ordinal position; there is no separate position type.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[must_use]
pub struct Sequence(NonZeroU64);

impl Sequence {
    /// The number of an endpoint's first publish attempt.
    pub const FIRST: Self = Self(NonZeroU64::MIN);

    /// Returns the number itself, starting at 1.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0.get()
    }

    /// Wraps a number, or returns `None` for zero, which no attempt ever gets.
    #[must_use]
    pub const fn new(number: u64) -> Option<Self> {
        match NonZeroU64::new(number) {
            Some(number) => Some(Self(number)),
            None => None,
        }
    }

    pub(crate) const fn next(self) -> Option<Self> {
        match self.0.checked_add(1) {
            Some(next) => Some(Self(next)),
            None => None,
        }
    }
}

/// A producer-supplied time, in whatever unit the producer's clock counts.
///
/// The library never converts it, attaches units to it or compares it with the
/// UI's receipt time; it travels with the item and a host interprets it. Two
/// timestamps are comparable when the host knows they come from the same clock,
/// which is always true within one bridge and never guaranteed across two.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[must_use]
pub struct Timestamp(u64);

impl Timestamp {
    /// Wraps a tick count from the producer's clock.
    pub const fn new(ticks: u64) -> Self {
        Self(ticks)
    }

    /// Returns the tick count.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

/// The result of one publish attempt, reported without touching the heap.
///
/// A real-time producer may ignore or count the outcome but must not act on it
/// in a way that blocks. Every outcome except `Accepted` is also observable on
/// the consumer side, as missing sequence numbers, a count or a terminal state.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[must_use = "a closed or exhausted endpoint will never accept again"]
#[non_exhaustive]
pub enum PublishOutcome {
    /// The item is in the ring and will reach the consumer.
    Accepted,
    /// The ring was full; the item was dropped, counted and its sequence
    /// reported as missing. A producer that must not lose this item keeps it
    /// and folds the next one into it.
    Overflowed,
    /// A non-finite float; the item was rejected, counted and its sequence
    /// reported as missing.
    Invalid,
    /// The consumer has been dropped; nothing was stored.
    Closed,
    /// A protocol counter could not advance; the endpoint is terminally closed
    /// rather than wrapping. At one step per nanosecond this takes centuries.
    Exhausted,
}

/// Why a bridge could not be configured or restarted.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum LiveError {
    /// A capacity of zero items.
    #[error("live-data capacity must be at least one item")]
    ZeroCapacity,
    /// A capacity above [`Capacity::MAX`].
    #[error("live-data capacity exceeds the documented maximum")]
    CapacityExceeded,
    /// A protocol counter could not advance; the endpoint is terminally closed.
    #[error("live-data counter exhausted")]
    Exhausted,
}

/// A validated number of items, for a ring or a retained window.
///
/// Zero is rejected, and so is anything above [`Self::MAX`], which bounds what
/// a display path may preallocate. A bridge has two capacities: the ring that
/// carries items between threads bounds one drain, and the window the consumer
/// keeps bounds what a trace can show. Neither grows.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[must_use]
pub struct Capacity(usize);

impl Capacity {
    /// The largest capacity, 1,048,576 items; a ring of that size preallocates 32 MiB.
    pub const MAX: Self = Self(1 << 20);

    /// Validates a capacity.
    ///
    /// # Errors
    /// Returns [`LiveError::ZeroCapacity`] for zero and
    /// [`LiveError::CapacityExceeded`] above [`Self::MAX`].
    pub const fn new(items: usize) -> Result<Self, LiveError> {
        if items == 0 {
            return Err(LiveError::ZeroCapacity);
        }
        if items > Self::MAX.0 {
            return Err(LiveError::CapacityExceeded);
        }
        Ok(Self(items))
    }

    /// Returns the number of items.
    #[must_use]
    pub const fn get(self) -> usize {
        self.0
    }
}

impl TryFrom<usize> for Capacity {
    type Error = LiveError;
    /// Validates like [`Capacity::new`].
    fn try_from(items: usize) -> Result<Self, Self::Error> {
        Self::new(items)
    }
}

/// How a series bridge is built: what the consumer retains, and what the ring
/// between the two threads carries.
///
/// The ring bounds one drain; the window bounds what a trace can show. Neither
/// grows. The ring starts at the window's capacity, which is the size a widget
/// that must see every item needs; [`Self::with_ring_capacity`] overrides it.
///
/// ```
/// use pert_ui_core::{Capacity, SeriesConfig};
///
/// // A trace, and a meter whose widget reduces every item it is given: the
/// // ring is at most the window, so one drain never overruns what is kept.
/// let trace = SeriesConfig::new(Capacity::new(512)?);
/// assert_eq!(trace.ring_capacity().get(), 512);
/// let meter = SeriesConfig::new(Capacity::new(8)?);
/// assert_eq!(meter.ring_capacity().get(), 8);
///
/// // A widget that only shows the newest items can buffer more than it keeps.
/// let newest = SeriesConfig::new(Capacity::new(8)?).with_ring_capacity(Capacity::new(64)?);
/// assert_eq!(newest.window_capacity().get(), 8);
/// assert_eq!(newest.ring_capacity().get(), 64);
/// # Ok::<(), pert_ui_core::LiveError>(())
/// ```
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[must_use]
pub struct SeriesConfig {
    window: Capacity,
    ring: Capacity,
}

impl SeriesConfig {
    /// Declares the retained window size; the ring starts at the same capacity.
    pub const fn new(window: Capacity) -> Self {
        Self {
            window,
            ring: window,
        }
    }

    /// Replaces the ring capacity, which bounds one drain.
    ///
    /// A ring larger than the window lets one drain deliver more items than the
    /// window keeps. The surplus is accepted, counted in
    /// [`DrainReport::accepted`] and then evicted by the newer items behind it;
    /// it is not loss and is not reported as missing, because the consumer did
    /// receive it. A widget that reduces what it is given, such as a meter
    /// holding a peak, must therefore keep the ring at or below the window, or
    /// it can miss an item that arrived and left between two reads. A widget
    /// that only draws the newest items is free to buffer more than it keeps.
    pub const fn with_ring_capacity(mut self, ring: Capacity) -> Self {
        self.ring = ring;
        self
    }

    /// Returns the retained window's capacity in items.
    pub const fn window_capacity(&self) -> Capacity {
        self.window
    }

    /// Returns the ring's capacity in items, which bounds one drain.
    pub const fn ring_capacity(&self) -> Capacity {
        self.ring
    }
}

/// One item as the consumer received it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Sample<T> {
    /// The published value.
    pub value: T,
    /// The incarnation it was published in.
    pub incarnation: Incarnation,
    /// Its sequence, which is also its position.
    pub sequence: Sequence,
    /// The producer's timestamp.
    pub timestamp: Timestamp,
}

/// What one drain found, beyond the items it delivered.
///
/// A consumer owns one report and rewrites it on every drain; read it before
/// the next drain. Every count is a delta since the previous drain, so a drain
/// that found nothing reports zeros.
///
/// The counts overlap: every rejected, dropped or discarded item also has a
/// sequence number that was never delivered, so it is counted again in
/// [`Self::missing`]. They need not land in the same drain. `rejected` and
/// `dropped` are read after the drain has fixed the bound it proves loss up
/// to, and a discarded item may carry a number above that bound, so these
/// counts can run one drain ahead of the missing numbers that correspond to
/// them. Summed over drains, `missing` is the loss; the other counts say why.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[non_exhaustive]
pub struct DrainReport {
    /// Items this drain delivered to the window; they are its newest entries.
    ///
    /// A ring larger than the window lets one drain deliver more than the
    /// window keeps, see [`SeriesConfig::with_ring_capacity`], so a consumer
    /// taking this drain's entries takes the newest
    /// `accepted.min(window.len())`. When a drain meets an item of a newer
    /// incarnation after it has already delivered items of the older one,
    /// those older items leave with the window it clears and are not counted
    /// here; they were received, so they are not in [`Self::missing`] either,
    /// and [`Self::incarnation_changed`] is set.
    pub accepted: u64,
    /// Items the consumer discarded: those of an older incarnation, and those
    /// thrown away on resume. Their numbers are also counted in
    /// [`Self::missing`], at this drain or the next.
    pub discarded: u64,
    /// Items the producer rejected as invalid since the previous drain. Their
    /// numbers are also counted in [`Self::missing`], at this drain or the
    /// next.
    pub rejected: u64,
    /// Items the producer dropped because the ring was full, since the
    /// previous drain. Their numbers are also counted in [`Self::missing`],
    /// at this drain or the next.
    pub dropped: u64,
    /// How many sequence numbers this drain proved were never received.
    ///
    /// Every publish attempt consumes a number, so this counts exactly the
    /// attempts that did not reach the window, including a trailing run the
    /// consumer can prove before any later item arrives. It says how much was
    /// lost; [`SampleWindowEntry::discontinuity_before`] says where a trace breaks.
    pub missing: u64,
    /// Whether this drain crossed into a newer incarnation and cleared its
    /// state, including items of the older incarnation it had delivered
    /// earlier in the same drain.
    pub incarnation_changed: bool,
    /// Whether the endpoint has been dropped; everything it published is in.
    pub disconnected: bool,
    /// Whether the endpoint closed itself because a counter could not advance.
    pub exhausted: bool,
}

impl DrainReport {
    pub(crate) fn clear(&mut self) {
        *self = Self::default();
    }

    pub(crate) fn record_missing(&mut self, first: u64, last: u64) {
        if last >= first {
            self.missing = self.missing.saturating_add(last - first + 1);
        }
    }
}

/// One entry of a retained series window.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SampleWindowEntry<T> {
    /// The item.
    pub sample: Sample<T>,
    /// Whether the items before this one are not continuous with it: a loss,
    /// a restart, a resume or a rejected item came between. A trace draws a
    /// break here instead of a line.
    pub discontinuity_before: bool,
}

/// The ordered, bounded items a series consumer retains for display.
///
/// The oldest item leaves when the window is full. Continuity is explicit:
/// every entry says whether it follows its predecessor without loss, so a
/// trace never joins items across an unreported loss.
#[derive(Clone, Debug, PartialEq)]
pub struct SampleWindow<T> {
    entries: VecDeque<SampleWindowEntry<T>>,
    capacity: usize,
}

impl<T: LiveItem> SampleWindow<T> {
    pub(crate) fn new(capacity: Capacity) -> Self {
        Self {
            entries: VecDeque::with_capacity(capacity.get()),
            capacity: capacity.get(),
        }
    }

    pub(crate) fn push(&mut self, sample: Sample<T>, discontinuity_before: bool) {
        if self.entries.len() == self.capacity {
            self.entries.pop_front();
        }
        self.entries.push_back(SampleWindowEntry {
            sample,
            discontinuity_before,
        });
    }

    pub(crate) fn clear(&mut self) {
        self.entries.clear();
    }

    /// Returns the entries, oldest first.
    ///
    /// The iterator is double-ended, so `.rev()` walks from the newest entry
    /// and a trace that draws the newest N entries of a wide window can take
    /// them without visiting the rest.
    ///
    /// ```
    /// use pert_ui_core::{Capacity, SeriesConfig, Timestamp, series_bridge};
    ///
    /// let (mut endpoint, mut consumer) =
    ///     series_bridge::<u64>(SeriesConfig::new(Capacity::new(8)?));
    /// for tick in 1..=5 {
    ///     let _ = endpoint.publish(tick * 10, Timestamp::new(tick));
    /// }
    /// let _ = consumer.drain();
    /// let newest: Vec<u64> = consumer
    ///     .window()
    ///     .iter()
    ///     .rev()
    ///     .take(2)
    ///     .map(|entry| entry.sample.value)
    ///     .collect();
    /// assert_eq!(newest, [50, 40]);
    /// assert_eq!(consumer.window().latest().map(|entry| entry.sample.value), Some(50));
    /// # Ok::<(), pert_ui_core::LiveError>(())
    /// ```
    pub fn iter(
        &self,
    ) -> impl DoubleEndedIterator<Item = &SampleWindowEntry<T>> + ExactSizeIterator + '_ {
        self.entries.iter()
    }

    /// Returns the newest entry, or `None` when nothing is retained.
    ///
    /// Constant time, unlike `iter().last()`.
    #[must_use]
    pub fn latest(&self) -> Option<&SampleWindowEntry<T>> {
        self.entries.back()
    }

    /// Returns how many items are retained.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Reports whether nothing is retained.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Returns the capacity in items, which never changes.
    #[must_use]
    pub const fn capacity(&self) -> usize {
        self.capacity
    }
}
