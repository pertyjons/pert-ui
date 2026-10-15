//! Bounded delivery of owned values from other threads to the UI thread.
//!
//! [`docs/contracts.md`] records the contract. A [`Mailbox`] lives on the UI
//! thread and owns a handler; its [`UiSender`] handles are `Send` and carry
//! owned values into a bounded queue. Sending wakes the host through the same
//! delivery a [`LiveWaker`] uses, and the runtime hands each value to the
//! handler at the start of a later flush.
//!
//! [`docs/contracts.md`]: https://github.com/pertyjons/pert-ui/blob/main/docs/contracts.md
use std::{
    cell::RefCell,
    collections::VecDeque,
    fmt,
    rc::{Rc, Weak},
    sync::{Arc, Mutex, MutexGuard, PoisonError},
};

use crate::{Capacity, LiveWaker, RuntimeError};

/// Why [`UiSender::send`] refused a value. The value is returned unchanged.
///
/// The set is closed, like the standard library's channel errors, so a match
/// recovering the value from both variants needs no wildcard arm.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SendError<T> {
    /// The queue holds its capacity of undelivered values. Retry later, drop the
    /// value or coalesce it with a newer one; the mailbox never grows.
    Full(T),
    /// The mailbox was closed or dropped, or its runtime shut down. No later
    /// send can succeed.
    Closed(T),
}

impl<T> SendError<T> {
    /// Returns the refused value.
    pub fn into_inner(self) -> T {
        match self {
            Self::Full(value) | Self::Closed(value) => value,
        }
    }
}

impl<T> fmt::Debug for SendError<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Full(_) => "Full(..)",
            Self::Closed(_) => "Closed(..)",
        })
    }
}

impl<T> fmt::Display for SendError<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Full(_) => "the mailbox is full",
            Self::Closed(_) => "the mailbox has closed",
        })
    }
}

impl<T> std::error::Error for SendError<T> {}

struct Queue<T> {
    state: Mutex<QueueState<T>>,
    capacity: usize,
}

struct QueueState<T> {
    items: VecDeque<T>,
    closed: bool,
}

impl<T> Queue<T> {
    /// The queue only pushes, pops and clears under the lock, none of which can
    /// panic halfway, so a poisoned lock still guards consistent state.
    fn lock(&self) -> MutexGuard<'_, QueueState<T>> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// A `Send` handle that delivers owned values to a [`Mailbox`] on the UI thread.
///
/// Created by [`Mailbox::sender`]; clones share the same queue. Sending takes a
/// short lock and, after an accepted value, wakes the host's event loop, so it
/// belongs on ordinary threads such as workers or I/O completions. Never send
/// from a time-critical producer callback: those must not lock or reach the
/// operating system; publish through a live-data source instead.
pub struct UiSender<T> {
    queue: Arc<Queue<T>>,
    waker: LiveWaker,
}

impl<T> Clone for UiSender<T> {
    fn clone(&self) -> Self {
        Self {
            queue: Arc::clone(&self.queue),
            waker: self.waker.clone(),
        }
    }
}

impl<T> fmt::Debug for UiSender<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("UiSender")
            .field("capacity", &self.queue.capacity)
            .finish_non_exhaustive()
    }
}

impl<T: Send> UiSender<T> {
    /// Queues `value` for the mailbox's handler and wakes the host.
    ///
    /// Values from one sender arrive in the order they were accepted. An
    /// accepted value is delivered by a later flush unless the mailbox closes
    /// or its runtime shuts down first; closing drops undelivered values on the
    /// UI thread, and that is the only way an accepted value is lost.
    ///
    /// # Errors
    /// [`SendError::Full`] while the queue holds its capacity, and
    /// [`SendError::Closed`] once the mailbox has closed; both return the value.
    pub fn send(&self, value: T) -> Result<(), SendError<T>> {
        {
            let mut state = self.queue.lock();
            if state.closed {
                return Err(SendError::Closed(value));
            }
            if state.items.len() >= self.queue.capacity {
                return Err(SendError::Full(value));
            }
            state.items.push_back(value);
        }
        // A failed wake means the event loop has exited; the runtime closes the
        // mailbox when it shuts down, which the next send reports.
        let _ = self.waker.notify();
        Ok(())
    }

    /// Whether the mailbox has closed, so no send can succeed.
    pub fn is_closed(&self) -> bool {
        self.queue.lock().closed
    }
}

type Handler<T> = Box<dyn FnMut(T) -> Result<(), RuntimeError>>;

struct Inbox<T> {
    queue: Arc<Queue<T>>,
    handler: RefCell<Handler<T>>,
}

impl<T> Drop for Inbox<T> {
    fn drop(&mut self) {
        let dropped = {
            let mut state = self.queue.lock();
            state.closed = true;
            std::mem::take(&mut state.items)
        };
        // Undelivered values drop here, on the UI thread, outside the lock.
        drop(dropped);
    }
}

/// The runtime's view of a mailbox, independent of its value type.
pub(crate) trait Deliverable {
    /// Captures this inbox's delivery frontier before any handler runs.
    fn pending_len(&self) -> usize;
    /// Hands at most the captured number of values to the handler, in order.
    fn deliver(&self, count: usize) -> Result<(), RuntimeError>;
    /// Whether values are waiting.
    fn has_pending(&self) -> bool;
    /// Refuses further sends and drops undelivered values.
    fn close(&self);
}

impl<T> Deliverable for Inbox<T> {
    fn pending_len(&self) -> usize {
        self.queue.lock().items.len()
    }
    fn deliver(&self, count: usize) -> Result<(), RuntimeError> {
        // Every inbox's count is captured before any handler runs. Values sent
        // by a handler therefore wait for the next flush, even in later inboxes.
        for _ in 0..count {
            // Borrowed before the value leaves the queue, so a conflict keeps
            // the value queued instead of dropping it.
            let mut handler = self
                .handler
                .try_borrow_mut()
                .map_err(|_| RuntimeError::BorrowConflict)?;
            let Some(value) = self.queue.lock().items.pop_front() else {
                break;
            };
            handler(value)?;
        }
        Ok(())
    }

    fn has_pending(&self) -> bool {
        !self.queue.lock().items.is_empty()
    }

    fn close(&self) {
        let dropped = {
            let mut state = self.queue.lock();
            state.closed = true;
            std::mem::take(&mut state.items)
        };
        drop(dropped);
    }
}

/// The UI-thread end of a bounded channel from other threads.
///
/// Created by [`Runtime::mailbox`](crate::Runtime::mailbox), which also takes the
/// handler. At the start of each flush the runtime hands every value that was
/// queued when delivery began to the handler, in order, on the UI thread; the
/// handler may write signals, so a worker's result reaches the components bound
/// to them. All inbox frontiers are captured before any handler runs, so
/// values sent between handlers wait for the next flush regardless of inbox
/// registration order. Concurrent worker sends are included only if accepted
/// before their inbox's frontier is captured.
///
/// Dropping the mailbox, calling [`Self::close`] or shutting the runtime down
/// closes it: every sender then reports [`SendError::Closed`] and undelivered
/// values are dropped on the UI thread. Keep the mailbox for as long as the
/// work it receives matters, for example in the component or application state
/// that started the work. The mailbox is UI-local and not `Send`.
///
/// ```
/// use pert_ui_runtime::{Capacity, Runtime};
/// let mut ui = Runtime::new();
/// let result = ui.signal(0_u64);
/// let target = result.clone();
/// let mailbox = ui.mailbox(Capacity::new(8)?, move |sum: u64| {
///     target.set(sum).map(|_| ())
/// })?;
/// let sender = mailbox.sender();
/// std::thread::spawn(move || {
///     let sum = (1..=100).sum();
///     // Full or Closed return the value; this worker has nothing to retry.
///     let _ = sender.send(sum);
/// })
/// .join()
/// .expect("the worker finishes");
/// ui.flush(std::time::Duration::ZERO)?;
/// assert_eq!(result.get()?, 5050);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[must_use = "dropping the mailbox closes it"]
pub struct Mailbox<T> {
    inbox: Rc<Inbox<T>>,
    waker: LiveWaker,
}

impl<T> Drop for Mailbox<T> {
    fn drop(&mut self) {
        // Runtime delivery retains an Inbox snapshot while handlers run.
        // Closing the public owner must take effect even during that snapshot.
        self.inbox.close();
    }
}

impl<T> fmt::Debug for Mailbox<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Mailbox")
            .field("capacity", &self.inbox.queue.capacity)
            .field("pending", &self.pending())
            .finish_non_exhaustive()
    }
}

impl<T: Send + 'static> Mailbox<T> {
    pub(crate) fn new(
        capacity: Capacity,
        handler: impl FnMut(T) -> Result<(), RuntimeError> + 'static,
        waker: LiveWaker,
    ) -> (Self, Weak<dyn Deliverable>) {
        let inbox = Rc::new(Inbox {
            queue: Arc::new(Queue {
                state: Mutex::new(QueueState {
                    items: VecDeque::new(),
                    closed: false,
                }),
                capacity: capacity.get(),
            }),
            handler: RefCell::new(Box::new(handler)),
        });
        let inbox_weak: Weak<Inbox<T>> = Rc::downgrade(&inbox);
        let deliverable: Weak<dyn Deliverable> = inbox_weak;
        (Self { inbox, waker }, deliverable)
    }
}

impl<T> Mailbox<T> {
    /// Returns a new `Send` handle for this mailbox's queue.
    pub fn sender(&self) -> UiSender<T> {
        UiSender {
            queue: Arc::clone(&self.inbox.queue),
            waker: self.waker.clone(),
        }
    }

    /// The number of values waiting for delivery.
    pub fn pending(&self) -> usize {
        self.inbox.queue.lock().items.len()
    }

    /// The most values the queue holds before [`UiSender::send`] reports full.
    pub fn capacity(&self) -> usize {
        self.inbox.queue.capacity
    }

    /// Closes the mailbox now, as dropping it would: senders report
    /// [`SendError::Closed`] and undelivered values are dropped.
    pub fn close(&self) {
        self.inbox.close();
    }
}
