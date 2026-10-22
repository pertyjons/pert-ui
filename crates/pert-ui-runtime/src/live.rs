//! UI-thread sources over the core live-data bridges: drain scheduling,
//! tracked reads, freshness and the coalescing wake behind notified delivery.

use std::{
    cell::{Cell, RefCell},
    rc::{Rc, Weak},
    sync::{Arc, Mutex},
    time::Duration,
};

use pert_ui_core::{Incarnation, LiveItem, Sample, SampleWindow, SeriesConsumer, WakeFlag};

use crate::{
    runtime::{Invalidation, RuntimeError, Shared},
    signal::Signal,
};

/// How a source learns that its producer published.
///
/// The default is [`Self::Polled`].
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum Delivery {
    /// The runtime drains the source at the presentation schedule while it has
    /// a visible subscriber. The default, and the only mode a real-time
    /// producer may use, since waking the event loop is forbidden on its path.
    #[default]
    Polled,
    /// The runtime drains the source when a [`LiveWaker`] delivered a wake. For
    /// producers that may reach the operating system, such as a worker thread
    /// that finished a computation; a restart or a drop of the endpoint must
    /// also notify, because nothing else drains a notified source.
    Notified,
}

/// How current a source's latest value is, as of the most recent drain.
///
/// The classification is source-level, so every subscriber sees the same
/// answer for one drain. A component that wants "changed since my previous
/// read" compares [`Telemetry::revision`] with the value it kept.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Deserialize, serde::Serialize),
    serde(deny_unknown_fields)
)]
pub enum Freshness {
    /// The most recent drain accepted a new value. Loss/rejection alone does
    /// not refresh a previously retained value.
    Fresh,
    /// The endpoint is connected but the most recent drain accepted no new
    /// value; it may still have reported loss or rejection.
    /// `age` is the runtime time since the last drain that accepted a value.
    Stale {
        /// Elapsed runtime time since the last accepted-value drain.
        age: Duration,
    },
    /// The endpoint has been dropped, so `latest` is final; or no value is
    /// retained, because nothing has ever been delivered, a new incarnation
    /// cleared it, or the source resumed after a period without a visible
    /// subscriber, lost items during
    /// it, and nothing has arrived since. Such a resume clears the retained
    /// window, since what it held no longer ends at the newest item, and that
    /// clearing advances [`Telemetry::revision`] like any other change. A
    /// resume after a suspension that lost nothing and did not restart keeps
    /// the window, which then reads as [`Freshness::Stale`] with its age since
    /// the last accepted value.
    Unavailable,
}

/// What a component reads from a source: the newest item and its status.
///
/// The revision advances on every drain that changed something, and the
/// receipt time is the runtime's elapsed clock at the last accepted-value
/// drain, never the producer's timestamp. An absent value is `latest == None`,
/// not an invented value. A disconnected source may retain its final value
/// while reporting [`Freshness::Unavailable`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Telemetry<T> {
    /// The most recently delivered item, if any.
    pub latest: Option<Sample<T>>,
    /// Advances on every drain that delivered, discarded or lost something,
    /// on a resume that cleared a non-empty window, and on disconnection.
    ///
    /// A change token: compare it for equality with a value kept earlier, never
    /// for order. It wraps at `u64::MAX` rather than stopping, so a change is
    /// always announced. It counts changed drains, not distinct runtime times:
    /// a notified wake or a resume after suspension may permit another change
    /// at the same time. Ordinary active polling drains at most once per time.
    pub revision: u64,
    /// Runtime time of the last drain that accepted a retained value. Loss-only
    /// changes preserve it; clearing the retained window resets it to `None`.
    pub received: Option<Duration>,
    /// How current `latest` is.
    pub freshness: Freshness,
    /// The incarnation the retained state belongs to.
    pub incarnation: Incarnation,
    /// Whether the endpoint has been dropped; everything it published is in.
    pub disconnected: bool,
}

/// Why a wake could not be delivered.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum WakeError {
    /// The event loop, or the runtime, is gone; no wake will ever be delivered.
    #[error("the live-data wake target has closed")]
    Closed,
}

struct WakeShared {
    flag: WakeFlag,
    deliver: DeliverySlot,
    /// Delivery failed; installing new delivery reopens the waker.
    closed: std::sync::atomic::AtomicBool,
    /// The runtime shut down; nothing reopens the waker, because no flush
    /// will ever drain what a wake would announce.
    shut_down: std::sync::atomic::AtomicBool,
}

impl WakeShared {
    fn is_closed(&self) -> bool {
        self.shut_down.load(std::sync::atomic::Ordering::Acquire)
            || self.closed.load(std::sync::atomic::Ordering::Acquire)
    }
}

/// A `Send` handle a non-real-time producer uses to have its source drained.
///
/// Created by [`Runtime::live_waker`](crate::Runtime::live_waker). Calling
/// [`Self::notify`] after publishing sets the runtime's pending flag and, when
/// a host installed delivery with
/// [`Runtime::set_wake_delivery`](crate::Runtime::set_wake_delivery), wakes
/// the event loop; concurrent notifications coalesce into one wake. The flag
/// protocol is [`WakeFlag`], and it is what makes a publish either drained by
/// the wake it raced or followed by a fresh one. A host without delivery, such
/// as a headless test, sees the pending wake in `Runtime::next_deadline`.
///
/// A notified source is drained by nothing else, so the host also notifies
/// after it restarts or drops such an endpoint; otherwise the restart or the
/// disconnection is never observed. Never call this from a real-time producer:
/// delivery reaches the operating system. Cloning shares the same flag.
#[derive(Clone)]
pub struct LiveWaker {
    shared: Arc<WakeShared>,
}

impl LiveWaker {
    /// Requests a drain of every notified source, after the publish it is for.
    ///
    /// # Errors
    /// Returns [`WakeError::Closed`] after the runtime shut down, which is
    /// permanent, or when delivery failed because the event loop has exited;
    /// the pending flag is then cleared so a later wake is not suppressed, and
    /// only newly installed delivery reopens the waker.
    pub fn notify(&self) -> Result<(), WakeError> {
        if self.shared.is_closed() {
            return Err(WakeError::Closed);
        }
        if !self.shared.flag.request() {
            return Ok(());
        }
        let delivered = lock_delivery(&self.shared.deliver, |deliver| {
            // Rechecked under the lock `Wake::close` takes, so a shutdown that
            // raced the check above is reported instead of a pending wake no
            // flush will ever take.
            let delivered = !self
                .shared
                .shut_down
                .load(std::sync::atomic::Ordering::Acquire)
                && deliver.as_ref().is_none_or(|deliver| deliver());
            if !delivered {
                // Closed while the lock is still held, so delivery that
                // `Wake::set_delivery` installs afterwards reopens the waker
                // instead of being overwritten by this failure.
                self.shared.flag.reset();
                self.shared
                    .closed
                    .store(true, std::sync::atomic::Ordering::Release);
            }
            delivered
        });
        if delivered {
            Ok(())
        } else {
            Err(WakeError::Closed)
        }
    }
}

impl core::fmt::Debug for LiveWaker {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("LiveWaker")
            .field("pending", &self.shared.flag.is_pending())
            .finish_non_exhaustive()
    }
}

/// The runtime's side of the wake protocol.
pub(crate) struct Wake {
    shared: Arc<WakeShared>,
}

impl Default for Wake {
    fn default() -> Self {
        Self {
            shared: Arc::new(WakeShared {
                flag: WakeFlag::new(),
                deliver: Mutex::new(None),
                closed: std::sync::atomic::AtomicBool::new(false),
                shut_down: std::sync::atomic::AtomicBool::new(false),
            }),
        }
    }
}

impl Wake {
    pub(crate) fn waker(&self) -> LiveWaker {
        LiveWaker {
            shared: Arc::clone(&self.shared),
        }
    }
    /// Installs delivery and reopens a waker whose delivery failed; ignored
    /// after shutdown, which no delivery can undo.
    pub(crate) fn set_delivery(&self, deliver: Box<dyn Fn() -> bool + Send + Sync>) {
        lock_delivery(&self.shared.deliver, |slot| {
            // Checked under the lock `close` also takes, so a shutdown cannot
            // interleave between the check and the install.
            if self
                .shared
                .shut_down
                .load(std::sync::atomic::Ordering::Acquire)
            {
                return;
            }
            *slot = Some(deliver);
            self.shared
                .closed
                .store(false, std::sync::atomic::Ordering::Release);
        });
    }
    pub(crate) fn is_pending(&self) -> bool {
        self.shared.flag.is_pending()
    }
    pub(crate) fn reset(&self) -> bool {
        self.shared.flag.reset()
    }
    pub(crate) fn close(&self) {
        self.shared
            .shut_down
            .store(true, std::sync::atomic::Ordering::Release);
        lock_delivery(&self.shared.deliver, |slot| *slot = None);
    }
}

/// The installed delivery callback.
type DeliverySlot = Mutex<Option<Box<dyn Fn() -> bool + Send + Sync>>>;

/// Runs `f` under the delivery lock. A host callback that panicked while the
/// lock was held poisons it without breaking anything the lock guards, the
/// installed callback, so the lock is recovered: ignoring the poison instead
/// would make every later wake report success without being delivered.
fn lock_delivery<R>(
    slot: &DeliverySlot,
    f: impl FnOnce(&mut Option<Box<dyn Fn() -> bool + Send + Sync>>) -> R,
) -> R {
    let mut guard = slot
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    f(&mut guard)
}

/// One live-data source as captured by
/// [`Runtime::inspect`](crate::Runtime::inspect).
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
#[cfg_attr(
    feature = "serde",
    derive(serde::Deserialize, serde::Serialize),
    serde(deny_unknown_fields)
)]
pub struct SourceInspection {
    /// Runtime-assigned identity, stable for the source's lifetime.
    pub id: u64,
    /// Polled or notified.
    pub delivery: Delivery,
    /// The current revision.
    pub revision: u64,
    /// The incarnation the retained state belongs to.
    pub incarnation: u64,
    /// How current the latest value is.
    pub freshness: Freshness,
    /// Components subscribed through a tracked read.
    pub subscribers: usize,
    /// Subscribed components that are visible, which is what keeps drains going.
    pub visible_subscribers: usize,
    /// The next scheduled drain, if the source is active and polled.
    pub next_drain: Option<Duration>,
    /// Items waiting in the ring, not yet drained.
    pub fill: u64,
    /// Items retained in the window.
    pub retained: usize,
    /// Items the producer has dropped so far.
    pub dropped_total: u64,
    /// Items the producer has rejected so far.
    pub rejected_total: u64,
    /// Whether the endpoint has been dropped.
    pub disconnected: bool,
    /// Whether the endpoint closed itself because a counter could not advance.
    pub exhausted: bool,
}

/// State every source shares, independent of the item type.
struct State {
    id: u64,
    delivery: Delivery,
    revision: Signal<u64>,
    active: Cell<bool>,
    next_drain: Cell<Option<Duration>>,
    drained_at: Cell<Option<Duration>>,
    received_at: Cell<Option<Duration>>,
    last_delivered: Cell<bool>,
    /// A wake that arrived while the consumer was borrowed, as by a flush run
    /// from inside `with_window`; it is served by the next flush instead of
    /// being lost.
    retry_wake: Cell<bool>,
}

impl State {
    fn new(shared: &Rc<Shared>, delivery: Delivery) -> Self {
        Self {
            id: shared.next_source_id(),
            delivery,
            revision: Signal::new(shared.clone(), 0),
            active: Cell::new(false),
            next_drain: Cell::new(None),
            drained_at: Cell::new(None),
            received_at: Cell::new(None),
            last_delivered: Cell::new(false),
            retry_wake: Cell::new(false),
        }
    }

    fn shared(&self) -> &Rc<Shared> {
        self.revision.shared()
    }

    /// Decides whether this flush drains the source, and resumes it first when a
    /// visible subscriber appeared since the previous flush.
    fn begin(&self, now: Duration, woke: bool, disconnected: bool) -> Drain {
        let woke = woke || self.retry_wake.take();
        let (_, visible) = self.revision.subscriber_counts();
        if visible == 0 || disconnected {
            self.active.set(false);
            self.next_drain.set(None);
            return Drain::Skip;
        }
        if !self.active.get() {
            self.active.set(true);
            return Drain::Resume;
        }
        // One drain per instant bounds repeated flushes at the same time, but a
        // wake is consumed by the flush that passes it here: skipping it would
        // leave a notified publish unread, since nothing else drains the source.
        if self.drained_at.get() == Some(now) && !(woke && self.delivery == Delivery::Notified) {
            return Drain::Skip;
        }
        let due = match self.delivery {
            Delivery::Polled => self.next_drain.get().is_some_and(|at| at <= now),
            Delivery::Notified => woke,
        };
        if due { Drain::Now } else { Drain::Skip }
    }

    fn finish(&self, now: Duration, changed: bool, delivered: bool) {
        self.drained_at.set(Some(now));
        self.last_delivered.set(delivered);
        if delivered {
            self.received_at.set(Some(now));
        }
        if changed {
            // A change token compared for equality, not a protocol counter: at
            // its limit it wraps, because stopping would swallow the change this
            // drain has to announce, and closing the source would discard data
            // over a number that orders nothing.
            self.revision
                .publish_presentation_copy(self.revision.get().unwrap_or(0).wrapping_add(1));
        }
        self.next_drain.set(match self.delivery {
            Delivery::Polled =>
            // A polled source drains at the same ceiling as animation.
            {
                self.revision
                    .presentation_interval()
                    .and_then(|interval| now.checked_add(interval))
            }
            Delivery::Notified => None,
        });
    }

    /// Nothing is scheduled without a visible subscriber, and nothing after the
    /// endpoint was dropped and its disconnection delivered: nothing can arrive.
    fn deadline(&self, now: Duration, disconnected: bool) -> Option<Duration> {
        let (_, visible) = self.revision.subscriber_counts();
        if visible == 0 || disconnected {
            self.active.set(false);
            self.next_drain.set(None);
            return None;
        }
        if !self.active.get() || self.retry_wake.get() {
            return Some(now);
        }
        if self.delivery == Delivery::Polled {
            self.next_drain.set(
                self.revision.presentation_interval().and_then(|interval| {
                    self.drained_at.get().unwrap_or(now).checked_add(interval)
                }),
            );
        }
        self.next_drain.get()
    }

    fn freshness(&self, now: Duration, disconnected: bool, has_value: bool) -> Freshness {
        if disconnected || !has_value {
            Freshness::Unavailable
        } else if self.last_delivered.get() {
            Freshness::Fresh
        } else {
            Freshness::Stale {
                age: self
                    .received_at
                    .get()
                    .map_or(Duration::ZERO, |at| now.saturating_sub(at)),
            }
        }
    }

    fn telemetry<T>(
        &self,
        latest: Option<Sample<T>>,
        incarnation: Incarnation,
        disconnected: bool,
    ) -> Telemetry<T> {
        let now = self.shared().now.get();
        Telemetry {
            freshness: self.freshness(now, disconnected, latest.is_some()),
            latest,
            revision: self.revision.get().unwrap_or(0),
            received: self.received_at.get(),
            incarnation,
            disconnected,
        }
    }

    fn inspection(&self, observed: Observed) -> SourceInspection {
        let (subscribers, visible_subscribers) = self.revision.subscriber_counts();
        let now = self.shared().now.get();
        SourceInspection {
            id: self.id,
            delivery: self.delivery,
            revision: self.revision.get().unwrap_or(0),
            incarnation: observed.incarnation.get(),
            freshness: self.freshness(now, observed.disconnected, observed.has_value),
            subscribers,
            visible_subscribers,
            next_drain: if self.active.get() {
                self.next_drain.get()
            } else {
                None
            },
            fill: observed.fill,
            retained: observed.retained,
            dropped_total: observed.dropped_total,
            rejected_total: observed.rejected_total,
            disconnected: observed.disconnected,
            exhausted: observed.exhausted,
        }
    }
}

/// What a consumer reports about itself, gathered for one inspection.
struct Observed {
    fill: u64,
    retained: usize,
    dropped_total: u64,
    rejected_total: u64,
    incarnation: Incarnation,
    disconnected: bool,
    exhausted: bool,
    has_value: bool,
}

enum Drain {
    Skip,
    Resume,
    Now,
}

/// A source the runtime services on every flush.
pub(crate) trait Drainable {
    fn eligible(&self) -> bool;
    fn repace(&self, now: Duration);
    fn service(&self, now: Duration, woke: bool);
    fn deadline(&self, now: Duration) -> Option<Duration>;
    fn inspect(&self) -> SourceInspection;
}

fn report_changed(report: &pert_ui_core::DrainReport) -> bool {
    report.accepted > 0
        || report.discarded > 0
        || report.rejected > 0
        || report.dropped > 0
        || report.missing > 0
        || report.incarnation_changed
}

struct SeriesInner<T: LiveItem> {
    state: State,
    consumer: RefCell<SeriesConsumer<T>>,
    disconnected_seen: Cell<bool>,
}

impl<T: LiveItem> Drainable for SeriesInner<T> {
    fn eligible(&self) -> bool {
        let eligible =
            self.state.revision.subscriber_counts().1 > 0 && !self.disconnected_seen.get();
        if !eligible {
            self.state.active.set(false);
            self.state.next_drain.set(None);
        }
        eligible
    }
    fn repace(&self, now: Duration) {
        if self.state.active.get() && self.state.delivery == Delivery::Polled {
            self.state
                .next_drain
                .set(
                    self.state
                        .revision
                        .presentation_interval()
                        .and_then(|interval| {
                            self.state
                                .drained_at
                                .get()
                                .unwrap_or(now)
                                .checked_add(interval)
                        }),
                );
        }
    }
    fn service(&self, now: Duration, woke: bool) {
        let Ok(mut consumer) = self.consumer.try_borrow_mut() else {
            // The application is reading the window; keep this flush's wake
            // for the next one.
            if woke {
                self.state.retry_wake.set(true);
            }
            return;
        };
        // A resume that clears a non-empty window changes what a subscriber
        // reads, so it must advance the revision like any other change; one
        // that keeps the window after a quiet suspension changes nothing.
        let mut cleared = false;
        let report = match self.state.begin(now, woke, self.disconnected_seen.get()) {
            Drain::Skip => return,
            Drain::Resume => {
                let held = !consumer.window().is_empty();
                let report = *consumer.resume();
                cleared = held && consumer.window().is_empty();
                report
            }
            Drain::Now => *consumer.drain(),
        };
        let disconnected = report.disconnected;
        let changed =
            report_changed(&report) || cleared || disconnected != self.disconnected_seen.get();
        self.disconnected_seen.set(disconnected);
        if consumer.window().is_empty() {
            self.state.received_at.set(None);
        }
        self.state.finish(now, changed, report.accepted > 0);
    }
    fn deadline(&self, now: Duration) -> Option<Duration> {
        self.state.deadline(now, self.disconnected_seen.get())
    }
    fn inspect(&self) -> SourceInspection {
        let consumer = self.consumer.borrow();
        self.state.inspection(Observed {
            fill: consumer.fill(),
            retained: consumer.window().len(),
            dropped_total: consumer.dropped_total(),
            rejected_total: consumer.rejected_total(),
            incarnation: consumer.incarnation(),
            disconnected: consumer.is_disconnected(),
            exhausted: consumer.is_exhausted(),
            has_value: !consumer.window().is_empty(),
        })
    }
}

/// A UI-thread series source over a [`SeriesConsumer`].
///
/// Created by [`Runtime::series_source`](crate::Runtime::series_source).
/// Clones share the source. A component reads it with
/// [`UpdateContext::read_series`](crate::UpdateContext::read_series), which
/// subscribes it; the runtime then drains the bridge at the presentation
/// schedule, or on wakes for a notified source, for as long as a visible
/// subscriber exists, and invalidates subscribers only when a drain changed
/// something. A drain that found nothing costs one bounded check and no
/// update, layout or paint. When no visible subscriber remains, draining
/// stops; the producer keeps publishing, and the first drain after a
/// subscriber returns discards that backlog. When anything was discarded or
/// lost, it also clears the retained window and starts from what arrives
/// next; until something arrives the source reads `latest: None` with
/// [`Freshness::Unavailable`], at a new revision. When the producer published
/// nothing meanwhile and did not restart, the window is kept and reads as
/// [`Freshness::Stale`]. A new incarnation clears the window regardless of loss.
///
/// ```compile_fail
/// # use pert_ui_runtime as pert_ui;
/// let runtime = pert_ui::Runtime::new();
/// let config = pert_ui::SeriesConfig::new(
///     pert_ui::Capacity::new(8).unwrap(),
/// );
/// let (_endpoint, consumer) = pert_ui::series_bridge::<f32>(config);
/// let source = runtime.series_source(consumer, pert_ui::Delivery::Polled);
/// std::thread::spawn(move || drop(source)); // UI-thread-local
/// ```
pub struct SeriesSource<T: LiveItem> {
    inner: Rc<SeriesInner<T>>,
}

impl<T: LiveItem> Clone for SeriesSource<T> {
    fn clone(&self) -> Self {
        Self {
            inner: Rc::clone(&self.inner),
        }
    }
}

impl<T: LiveItem> SeriesSource<T> {
    pub(crate) fn new(
        shared: &Rc<Shared>,
        consumer: SeriesConsumer<T>,
        delivery: Delivery,
    ) -> Self {
        let inner = Rc::new(SeriesInner {
            state: State::new(shared, delivery),
            consumer: RefCell::new(consumer),
            disconnected_seen: Cell::new(false),
        });
        shared.register_source(Rc::downgrade(&inner) as Weak<dyn Drainable>);
        Self { inner }
    }

    /// Whether two handles share one source, as clones of one handle do.
    ///
    /// Compares identity without draining or reading. Two sources over
    /// different bridges are never the same, whatever they hold. A component
    /// uses this in `Component::reconcile` to keep what it folded from a source
    /// that a new description did not replace.
    #[must_use]
    pub fn same_source(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.inner, &other.inner)
    }

    pub(crate) fn shared(&self) -> &Rc<Shared> {
        self.inner.state.shared()
    }

    pub(crate) fn activate(&self) {
        self.inner.repace(self.shared().now.get());
        self.shared()
            .activate_source(Rc::downgrade(&self.inner) as Weak<dyn Drainable>);
    }
    pub(crate) fn revision_signal(&self) -> &Signal<u64> {
        &self.inner.state.revision
    }

    /// Reads the latest status and the retained window without subscribing,
    /// for event and application code.
    ///
    /// # Errors
    /// Returns `BorrowConflict` when called while the source is being drained
    /// or read, or `Closed` after the runtime was dropped.
    pub fn with_window<R>(
        &self,
        f: impl FnOnce(&Telemetry<T>, &SampleWindow<T>) -> R,
    ) -> Result<R, RuntimeError> {
        self.shared().readable()?;
        let consumer = self
            .inner
            .consumer
            .try_borrow()
            .map_err(|_| RuntimeError::BorrowConflict)?;
        let window = consumer.window();
        let telemetry = self.inner.state.telemetry(
            window.latest().map(|entry| entry.sample),
            consumer.incarnation(),
            consumer.is_disconnected(),
        );
        Ok(f(&telemetry, window))
    }

    /// Reads the latest status without subscribing.
    ///
    /// # Errors
    /// Same failures as [`Self::with_window`].
    pub fn telemetry(&self) -> Result<Telemetry<T>, RuntimeError> {
        self.with_window(|telemetry, _| *telemetry)
    }
}

impl<T: LiveItem> core::fmt::Debug for SeriesSource<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("SeriesSource")
            .field("id", &self.inner.state.id)
            .field("delivery", &self.inner.state.delivery)
            .finish_non_exhaustive()
    }
}

/// The invalidation a source read declares: data changes presentation only.
pub(crate) const READ_KIND: Invalidation = Invalidation::Paint;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loss_only_accounting_does_not_refresh_a_retained_value() {
        let ui = crate::Runtime::new();
        let state = State::new(&ui.shared, Delivery::Polled);
        let first = Duration::from_secs(1);
        state.finish(first, true, true);
        let mut revision = state.revision.get().unwrap();
        // A pure dropped-count change can be observed after another drain's
        // read frontier; it changes status without delivering a new sample.
        let mut loss = pert_ui_core::DrainReport::default();
        loss.dropped = 1;
        for second in [2, 3] {
            let now = Duration::from_secs(second);
            state.finish(now, report_changed(&loss), loss.accepted > 0);
            assert_ne!(state.revision.get().unwrap(), revision);
            revision = state.revision.get().unwrap();
            assert_eq!(state.received_at.get(), Some(first));
            assert_eq!(
                state.freshness(now, false, true),
                Freshness::Stale { age: now - first }
            );
        }
    }
}
