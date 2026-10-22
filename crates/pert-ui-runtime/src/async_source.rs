//! Bookkeeping for data a component asks its host to load asynchronously.
//!
//! A component that needs data it must not load itself, such as a directory
//! listing, issues a [`RequestTicket`] from its [`RequestTracker`] and hands it
//! to the host. The host does the work wherever it likes, usually on a worker
//! thread, and brings the result back to the UI thread through any channel it
//! chooses, for example a [`Mailbox`](crate::Mailbox). The component then asks
//! its tracker whether the ticket is still [`TicketStatus::Current`] before it
//! applies the result. How a component presents data that is loading or
//! failed, such as placeholder rows, stays its own decision.
use crate::RuntimeError;
use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::marker::PhantomData;
use std::rc::Rc;
use std::sync::Arc;

/// One request a [`RequestTracker`] issued: the key of what to load, its
/// generation and the tracker that issued it.
///
/// A ticket is `Send` when its key is, so the host can move it to a worker
/// thread and back with the result. Two tickets are equal when the same
/// tracker issued them for equal keys with the same generation. A ticket keeps
/// its tracker's identity alive, so a tracker created after the issuing one was
/// dropped can never mistake the ticket for its own.
#[derive(Clone, Debug)]
#[must_use = "a ticket the host never answers leaves its key pending"]
pub struct RequestTicket<K> {
    key: K,
    generation: u64,
    owner: Arc<()>,
}
impl<K> RequestTicket<K> {
    /// The key of what to load.
    pub fn key(&self) -> &K {
        &self.key
    }
    /// The generation: unique within its tracker and increasing with each
    /// ticket the tracker issues, for any key.
    #[must_use]
    pub fn generation(&self) -> u64 {
        self.generation
    }
}
impl<K: PartialEq> PartialEq for RequestTicket<K> {
    fn eq(&self, other: &Self) -> bool {
        self.key == other.key
            && self.generation == other.generation
            && Arc::ptr_eq(&self.owner, &other.owner)
    }
}
impl<K: Eq> Eq for RequestTicket<K> {}
impl<K: std::hash::Hash> std::hash::Hash for RequestTicket<K> {
    /// Hashes the key, the generation and the issuing tracker's identity, the
    /// parts equality compares, so a ticket can key a map when its key can.
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.key.hash(state);
        self.generation.hash(state);
        Arc::as_ptr(&self.owner).hash(state);
    }
}

/// Whether a result for a [`RequestTicket`] may still be applied, from
/// [`RequestTracker::status`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum TicketStatus {
    /// The ticket is the latest one for its key and still pending: apply its
    /// result, then call [`RequestTracker::complete`].
    Current,
    /// A newer ticket for the same key was issued; this result is stale.
    Superseded,
    /// Nothing is pending for the key any longer: the request was cancelled,
    /// or a result for it was already completed. A repeated delivery of the
    /// same result therefore reports this.
    Settled,
    /// Another tracker issued the ticket, including one that no longer exists.
    Foreign,
}

/// Issues [`RequestTicket`]s for keys and tells which results are still
/// wanted, so a component never applies a stale or foreign result.
///
/// Per key, only the latest ticket is pending: issuing a new one for the same
/// key supersedes the older, and [`Self::cancel`] or [`Self::retain`] drop keys
/// the component no longer wants. A result is applied only while
/// [`Self::status`] reports [`TicketStatus::Current`]; after applying it the
/// component calls [`Self::complete`]. When applying fails, for example because
/// a signal write was refused, the component leaves the ticket pending, so a
/// later delivery of the same result can still apply.
///
/// The tracker performs no I/O, starts no thread and schedules nothing; it is
/// plain UI-thread state and is neither `Send` nor `Sync`. Its tickets are
/// `Send` when their key is, so the host can carry them to a worker:
///
/// ```compile_fail
/// # use pert_ui_runtime as pert_ui;
/// use pert_ui::RequestTracker;
///
/// let tracker = RequestTracker::<String>::new();
/// std::thread::spawn(move || tracker.issue("songs".into()));
/// ```
///
/// ```
/// # use pert_ui_runtime as pert_ui;
/// use pert_ui::{RequestTracker, TicketStatus};
///
/// let tracker = RequestTracker::new();
/// let first = tracker.issue("songs")?;
/// let second = tracker.issue("songs")?; // the user asked again
/// assert_eq!(tracker.status(&first), TicketStatus::Superseded);
/// assert_eq!(tracker.status(&second), TicketStatus::Current);
/// // ...apply the second result, then:
/// assert!(tracker.complete(&second));
/// assert_eq!(tracker.status(&second), TicketStatus::Settled);
/// # Ok::<(), pert_ui::RuntimeError>(())
/// ```
#[derive(Debug)]
pub struct RequestTracker<K> {
    owner: Arc<()>,
    generation: Cell<u64>,
    pending: RefCell<BTreeMap<K, u64>>,
    /// Keeps the tracker on the thread that created it.
    ui_thread: PhantomData<Rc<()>>,
}
impl<K: Ord + Clone> Default for RequestTracker<K> {
    fn default() -> Self {
        Self::new()
    }
}
impl<K: Ord + Clone> RequestTracker<K> {
    /// Creates a tracker with nothing pending.
    #[must_use]
    pub fn new() -> Self {
        Self {
            owner: Arc::new(()),
            generation: Cell::new(0),
            pending: RefCell::new(BTreeMap::new()),
            ui_thread: PhantomData,
        }
    }

    /// Issues a ticket for `key`, making it the key's only pending request and
    /// superseding an older ticket for the same key.
    ///
    /// # Errors
    /// [`RuntimeError::Exhausted`] once `u64::MAX` tickets have been issued;
    /// nothing changes then, and every later call fails the same way.
    pub fn issue(&self, key: K) -> Result<RequestTicket<K>, RuntimeError> {
        let generation = self
            .generation
            .get()
            .checked_add(1)
            .ok_or(RuntimeError::Exhausted)?;
        self.generation.set(generation);
        self.pending.borrow_mut().insert(key.clone(), generation);
        Ok(RequestTicket {
            key,
            generation,
            owner: self.owner.clone(),
        })
    }

    /// Whether a result for `ticket` may still be applied.
    pub fn status(&self, ticket: &RequestTicket<K>) -> TicketStatus {
        if !Arc::ptr_eq(&ticket.owner, &self.owner) {
            return TicketStatus::Foreign;
        }
        match self.pending.borrow().get(&ticket.key) {
            Some(generation) if *generation == ticket.generation => TicketStatus::Current,
            Some(generation) if *generation > ticket.generation => TicketStatus::Superseded,
            _ => TicketStatus::Settled,
        }
    }

    /// Marks `ticket`'s result as applied, ending its key's pending request.
    /// Returns false, and changes nothing, unless the ticket was
    /// [`TicketStatus::Current`].
    pub fn complete(&self, ticket: &RequestTicket<K>) -> bool {
        if self.status(ticket) != TicketStatus::Current {
            return false;
        }
        self.pending.borrow_mut().remove(&ticket.key);
        true
    }

    /// Cancels the pending request for `key`, so a result for it becomes
    /// [`TicketStatus::Settled`]. Returns whether one was pending.
    pub fn cancel(&self, key: &K) -> bool {
        self.pending.borrow_mut().remove(key).is_some()
    }

    /// Keeps the pending requests whose keys `wanted` accepts and cancels the
    /// others.
    pub fn retain(&self, mut wanted: impl FnMut(&K) -> bool) {
        self.pending.borrow_mut().retain(|key, _| wanted(key));
    }

    /// Whether a request for `key` is pending.
    #[must_use]
    pub fn is_pending(&self, key: &K) -> bool {
        self.pending.borrow().contains_key(key)
    }

    /// Sets the last issued generation, so tests can reach exhaustion.
    #[cfg(test)]
    pub(crate) fn set_generation(&self, generation: u64) {
        self.generation.set(generation);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exhaustion_is_an_error_that_changes_nothing() {
        let tracker = RequestTracker::new();
        let pending = tracker.issue(1).unwrap();
        tracker.set_generation(u64::MAX - 1);
        let last = tracker.issue(2).unwrap();
        assert_eq!(last.generation(), u64::MAX);
        assert_eq!(tracker.issue(1).unwrap_err(), RuntimeError::Exhausted);
        assert_eq!(tracker.issue(3).unwrap_err(), RuntimeError::Exhausted);
        // The refused issue superseded nothing and added nothing.
        assert_eq!(tracker.status(&pending), TicketStatus::Current);
        assert!(!tracker.is_pending(&3));
    }
}
