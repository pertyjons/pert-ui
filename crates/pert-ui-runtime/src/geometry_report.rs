//! Reporting committed geometry changes from the event phase.

use crate::{ChangeOrigin, Notifier};
use std::cell::{Cell, RefCell};

/// Tracks one committed geometry value a component reports to an application
/// callback: the value last reported, the origin of a change not yet
/// delivered and the notifier that schedules its delivery.
///
/// A component records each change with its origin from any phase, including
/// arrangement; the first record of a flush asks for one
/// [`Component::deliver_notifications`](crate::Component::deliver_notifications)
/// call, in which [`Self::take`] yields the current value and the last origin
/// once, unless the value equals the one last reported. Interior mutability
/// lets `arrange` and other `&self` paths record.
pub(crate) struct GeometryReport<T> {
    reported: RefCell<T>,
    pending: Cell<Option<ChangeOrigin>>,
    requested: Cell<bool>,
    notifier: RefCell<Option<Notifier>>,
}

impl<T: Clone + PartialEq> GeometryReport<T> {
    /// Starts with `initial` as the value already known to the application.
    pub(crate) fn new(initial: T) -> Self {
        Self {
            reported: RefCell::new(initial),
            pending: Cell::new(None),
            requested: Cell::new(false),
            notifier: RefCell::new(None),
        }
    }
    /// Keeps the component's notifier, obtained in its update or an event,
    /// and asks for a delivery a change recorded before it had one needs.
    pub(crate) fn set_notifier(&self, notifier: impl FnOnce() -> Notifier) {
        if self.notifier.borrow().is_none() {
            self.notifier.replace(Some(notifier()));
        }
        if self.pending.get().is_some() {
            self.request();
        }
    }
    /// Records a change with its origin, replacing the origin of an earlier
    /// change in the same flush.
    pub(crate) fn record(&self, origin: ChangeOrigin) {
        self.pending.set(Some(origin));
        self.request();
    }
    /// Records a change only when no other change waits, for an adjustment
    /// such as clamping that completes a change already recorded.
    pub(crate) fn record_if_idle(&self, origin: ChangeOrigin) {
        if self.pending.get().is_none() {
            self.record(origin);
        }
    }
    fn request(&self) {
        if self.requested.get() {
            return;
        }
        if let Some(notifier) = self.notifier.borrow().as_ref() {
            notifier.request();
            self.requested.set(true);
        }
    }
    /// The change to report in the delivery: `current` with the last origin,
    /// or `None` when nothing was recorded or the value is unchanged. Either
    /// way `current` becomes the value last reported, so nothing is replayed.
    pub(crate) fn take(&self, current: T) -> Option<(T, ChangeOrigin)> {
        self.requested.set(false);
        let origin = self.pending.take()?;
        let changed = *self.reported.borrow() != current;
        self.reported.replace(current.clone());
        changed.then_some((current, origin))
    }
    /// Forgets a change not yet delivered and takes `current` as known, for a
    /// description without a callback.
    pub(crate) fn discard(&self, current: T) {
        self.pending.set(None);
        self.reported.replace(current);
    }
}
