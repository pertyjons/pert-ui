//! Application-lifetime services: values kept until shutdown, work on
//! background threads delivered to the UI thread, and repeating application
//! timers.
use super::*;
use std::any::Any;

/// One unit of background work's outcome as the UI thread receives it: the
/// value, or `None` when the work panicked before returning one.
type Outcome = Option<Box<dyn Any + Send>>;

/// Hands a background value to its task's `deliver` on the UI thread.
type Deliver = Box<dyn FnOnce(Box<dyn Any + Send>) -> Result<(), RuntimeError>>;
/// An interval timer's tick.
type Tick = Box<dyn FnMut(&mut Runtime) -> Result<(), RuntimeError>>;

/// A background task waiting for its outcome on the UI thread.
struct Waiting {
    deliver: Deliver,
    cancelled: Rc<Cell<bool>>,
}

/// The runtime's background work: one mailbox that every worker answers
/// through, and the tasks whose answer has not arrived.
pub(super) struct Background {
    mailbox: crate::Mailbox<(u64, Outcome)>,
    waiting: Rc<RefCell<BTreeMap<u64, Waiting>>>,
    serial: u64,
}

/// A repeating application timer.
pub(super) struct Interval {
    due: Duration,
    period: Duration,
    tick: Tick,
    cancelled: Rc<Cell<bool>>,
}

/// Sends a worker's outcome once: the value it returned, or, when the work
/// panics and unwinding drops this, the absence of one.
struct Reply {
    sender: crate::UiSender<(u64, Outcome)>,
    id: u64,
    sent: bool,
}
impl Reply {
    fn send(mut self, value: Box<dyn Any + Send>) {
        self.sent = true;
        // A closed mailbox means the runtime shut down; the value is dropped
        // here, on the worker, as documented.
        let _closed = self.sender.send((self.id, Some(value)));
    }
}
impl Drop for Reply {
    fn drop(&mut self) {
        if !self.sent {
            let _closed = self.sender.send((self.id, None));
        }
    }
}

/// A handle to work started with [`Runtime::spawn`].
///
/// Dropping the handle does not cancel the work: the result is delivered all
/// the same. [`Self::cancel`] discards a result that has not been delivered
/// yet; the work itself runs to completion on its thread, since a thread
/// cannot be stopped from outside. The handle is UI-local.
#[derive(Clone, Debug)]
pub struct BackgroundTask {
    cancelled: Rc<Cell<bool>>,
}
impl BackgroundTask {
    /// Discards the result when it arrives, so `deliver` never runs. Has no
    /// effect after delivery.
    pub fn cancel(&self) {
        self.cancelled.set(true);
    }
}

/// A handle to a timer started with [`Runtime::start_interval`].
///
/// Dropping the handle does not stop the timer; [`Self::cancel`] does. The
/// handle is UI-local.
#[derive(Clone, Debug)]
pub struct IntervalTimer {
    cancelled: Rc<Cell<bool>>,
}
impl IntervalTimer {
    /// Stops the timer: it does not tick again, also when called from its own
    /// tick.
    pub fn cancel(&self) {
        self.cancelled.set(true);
    }
    /// Whether [`Self::cancel`] was called.
    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        self.cancelled.get()
    }
}

impl Runtime {
    /// The most background tasks that can wait for their results at once;
    /// [`Self::spawn`] refuses more.
    pub const MAX_BACKGROUND_TASKS: usize = 16;
    /// The most interval timers that can run at once;
    /// [`Self::start_interval`] refuses more.
    pub const MAX_INTERVALS: usize = 64;

    /// Keeps `value` alive until the runtime shuts down or is dropped.
    ///
    /// This is the owner for values an application creates once and must not
    /// lose, such as a [`Mailbox`](crate::Mailbox), a
    /// [`ParameterHost`](crate::ParameterHost) or a controller made in the
    /// [`App`](https://docs.rs/pert-ui/latest/pert_ui/struct.App.html)
    /// initializer, where a local value would be dropped when the initializer
    /// returns and, for a mailbox, close it.
    ///
    /// [`Self::shutdown`] drops kept values last, after components have
    /// unmounted and parameters have ended their edits, in the reverse order
    /// of keeping, so a value kept later, which may refer to one kept earlier,
    /// goes first. After shutdown a value is dropped at once.
    ///
    /// ```
    /// use pert_ui_runtime::{Capacity, Runtime};
    /// let mut ui = Runtime::new();
    /// let saved = ui.signal(false);
    /// let done = saved.clone();
    /// let mailbox = ui.mailbox(Capacity::new(4)?, move |ok: bool| done.set(ok))?;
    /// let sender = mailbox.sender();
    /// ui.keep_alive(mailbox);
    /// std::thread::spawn(move || sender.send(true)).join().unwrap()?;
    /// ui.flush(std::time::Duration::ZERO)?;
    /// assert!(saved.get()?);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn keep_alive<T: 'static>(&mut self, value: T) {
        if self.shared.closed.get() {
            drop(value);
            return;
        }
        self.kept.push(Box::new(value));
    }

    /// Runs `work` on a new background thread and hands its result to
    /// `deliver` on the UI thread, in the first flush after it arrives.
    ///
    /// This is for occasional work that must not block the interface, such
    /// as reading or writing a file: each call starts one thread, and at most
    /// [`Self::MAX_BACKGROUND_TASKS`] tasks wait for their results at once.
    /// `work` receives no UI handles and must not need any; return what the
    /// interface needs and let `deliver` write it to signals. A failure that
    /// the application should show belongs in the result, for example as a
    /// `Result` that `deliver` presents. Arriving results wake the host and
    /// are delivered in arrival order, with the runtime's mailboxes; `deliver`
    /// may write bindings, and its error is returned from that flush. Nothing
    /// here is safe for time-critical producers, which use the live-data
    /// endpoints instead.
    ///
    /// A component's or control's callback receives an
    /// [`EventContext`](crate::EventContext), not the runtime: it starts the
    /// work by queuing this call with
    /// [`EventContext::defer`](crate::EventContext::defer), whose operation
    /// runs at the start of the next flush with the runtime.
    ///
    /// The returned handle can [`cancel`](BackgroundTask::cancel) the
    /// delivery; dropping it does not. If `work` panics, nothing is delivered
    /// and the flush that receives the outcome returns
    /// [`RuntimeError::BackgroundPanicked`]. After [`Self::shutdown`] a result
    /// is dropped on its thread and never delivered; shutdown does not wait
    /// for running work.
    ///
    /// ```
    /// use pert_ui_runtime::Runtime;
    /// use std::time::Duration;
    /// let mut ui = Runtime::new();
    /// let status = ui.signal("Reading".to_owned());
    /// let shown = status.clone();
    /// // Real work would read or write a file; the result carries its error.
    /// ui.spawn(
    ///     || "42".parse::<u32>(),
    ///     move |result| shown.set(match result {
    ///         Ok(value) => format!("Read {value}"),
    ///         Err(error) => format!("Reading failed: {error}"),
    ///     }),
    /// )?;
    /// // A native host flushes when the result wakes it; a headless one polls.
    /// while status.get()? == "Reading" {
    ///     std::thread::sleep(Duration::from_millis(1));
    ///     ui.flush(Duration::ZERO)?;
    /// }
    /// assert_eq!(status.get()?, "Read 42");
    /// # Ok::<(), pert_ui_runtime::RuntimeError>(())
    /// ```
    ///
    /// # Errors
    /// `Closed` after shutdown and the phase errors of a write from inside
    /// update, layout, paint, inspection or delivery; `BackgroundFull` while
    /// [`Self::MAX_BACKGROUND_TASKS`] tasks wait; `BackgroundSpawn` with the
    /// operating system's reason when the thread cannot be started. A refused
    /// call starts nothing.
    pub fn spawn<T: Send + 'static>(
        &mut self,
        work: impl FnOnce() -> T + Send + 'static,
        deliver: impl FnOnce(T) -> Result<(), RuntimeError> + 'static,
    ) -> Result<BackgroundTask, RuntimeError> {
        self.shared.writable()?;
        if self.background.is_none() {
            let waiting: Rc<RefCell<BTreeMap<u64, Waiting>>> = Rc::default();
            let pending = waiting.clone();
            let capacity = crate::Capacity::new(Self::MAX_BACKGROUND_TASKS)?;
            let mailbox = self.mailbox(capacity, move |(id, outcome): (u64, Outcome)| {
                let Some(task) = pending.borrow_mut().remove(&id) else {
                    return Ok(());
                };
                match outcome {
                    None => Err(RuntimeError::BackgroundPanicked),
                    Some(_) if task.cancelled.get() => Ok(()),
                    Some(value) => (task.deliver)(value),
                }
            })?;
            self.background = Some(Background {
                mailbox,
                waiting,
                serial: 0,
            });
        }
        let background = self.background.as_mut().ok_or(RuntimeError::Closed)?;
        if background.waiting.borrow().len() >= Self::MAX_BACKGROUND_TASKS {
            return Err(RuntimeError::BackgroundFull);
        }
        background.serial = background
            .serial
            .checked_add(1)
            .ok_or(RuntimeError::Exhausted)?;
        let id = background.serial;
        let cancelled = Rc::new(Cell::new(false));
        background.waiting.borrow_mut().insert(
            id,
            Waiting {
                deliver: Box::new(move |value| match value.downcast::<T>() {
                    Ok(value) => deliver(*value),
                    // The worker boxed a `T` for this id.
                    Err(_) => Err(RuntimeError::BackgroundPanicked),
                }),
                cancelled: cancelled.clone(),
            },
        );
        let sender = background.mailbox.sender();
        let spawned = std::thread::Builder::new()
            .name("pert-ui background".to_owned())
            .spawn(move || {
                // Built on the running thread: a thread that never starts drops
                // its closure without answering, keeping the reservation exact.
                let reply = Reply {
                    sender,
                    id,
                    sent: false,
                };
                let value = work();
                reply.send(Box::new(value));
            });
        if let Err(error) = spawned {
            background.waiting.borrow_mut().remove(&id);
            return Err(RuntimeError::BackgroundSpawn(error.kind()));
        }
        Ok(BackgroundTask { cancelled })
    }

    /// Calls `tick` every `period` until it is cancelled or the runtime shuts
    /// down, for application work on a schedule, such as saving a draft.
    ///
    /// The timer belongs to the application, not to a component or window:
    /// focus changes, input cancellation and hidden windows do not stop it.
    /// `tick` runs on the UI thread with runtime access, before component
    /// updates, in a flush at or after its due time; the native host also runs
    /// a due timer while a window's redraw is withheld, for example while it
    /// is minimized. The first tick is due one `period` after the call. A tick
    /// that runs late does not catch up: the next one is due one `period`
    /// after the late one ran, if its scheduled time has passed. A failing
    /// tick keeps the timer running; its error is returned from that flush, or
    /// from [`Self::service_intervals`]. A running timer keeps the
    /// application's next deadline set, so use it only for work that needs it.
    ///
    /// ```
    /// use pert_ui_runtime::Runtime;
    /// use std::time::Duration;
    /// let mut ui = Runtime::new();
    /// let saves = ui.signal(0);
    /// let counter = saves.clone();
    /// let timer = ui.start_interval(Duration::from_secs(30), move |_| counter.update(|n| *n += 1))?;
    /// ui.flush(Duration::from_secs(29))?;
    /// assert_eq!(saves.get()?, 0);
    /// ui.flush(Duration::from_secs(30))?;
    /// assert_eq!(saves.get()?, 1);
    /// timer.cancel();
    /// ui.flush(Duration::from_secs(90))?;
    /// assert_eq!(saves.get()?, 1);
    /// # Ok::<(), pert_ui_runtime::RuntimeError>(())
    /// ```
    ///
    /// # Errors
    /// `ZeroDelay` for a zero period; `Exhausted` when the first due time
    /// overflows; `IntervalsFull` while [`Self::MAX_INTERVALS`] timers run;
    /// `Closed` after shutdown and the phase errors of a write from inside
    /// update, layout, paint, inspection or delivery.
    pub fn start_interval(
        &mut self,
        period: Duration,
        tick: impl FnMut(&mut Runtime) -> Result<(), RuntimeError> + 'static,
    ) -> Result<IntervalTimer, RuntimeError> {
        self.shared.writable()?;
        if period.is_zero() {
            return Err(RuntimeError::ZeroDelay);
        }
        self.intervals
            .retain(|_, interval| !interval.cancelled.get());
        // A timer whose tick is running is out of the map but still counts.
        if self.intervals.len() + self.intervals_ticking >= Self::MAX_INTERVALS {
            return Err(RuntimeError::IntervalsFull);
        }
        let due = self
            .shared
            .now
            .get()
            .checked_add(period)
            .ok_or(RuntimeError::Exhausted)?;
        self.interval_serial = self
            .interval_serial
            .checked_add(1)
            .ok_or(RuntimeError::Exhausted)?;
        let cancelled = Rc::new(Cell::new(false));
        self.intervals.insert(
            self.interval_serial,
            Interval {
                due,
                period,
                tick: Box::new(tick),
                cancelled: cancelled.clone(),
            },
        );
        Ok(IntervalTimer { cancelled })
    }

    /// The earliest due time of a running interval timer, for a host that
    /// schedules its own wakeups; [`Self::next_deadline`] and
    /// [`Self::application_deadline`] include it.
    #[must_use]
    pub fn interval_deadline(&self) -> Option<Duration> {
        self.intervals
            .values()
            .filter(|interval| !interval.cancelled.get())
            .map(|interval| interval.due)
            .min()
    }

    /// Runs the interval timers due at `now` outside a flush, for a host that
    /// cannot flush a window, such as one whose redraw the window system
    /// withholds. An ordinary flush runs due timers itself. Advances the
    /// runtime's time to `now`.
    ///
    /// # Errors
    /// `TimeReversed` for a time before the runtime's current one, `Closed`
    /// after shutdown and the phase errors of a write from inside update,
    /// layout, paint, inspection or delivery, then the first error a tick
    /// returns; later due timers run in the next call or flush.
    pub fn service_intervals(&mut self, now: Duration) -> Result<(), RuntimeError> {
        if now < self.shared.now.get() {
            return Err(RuntimeError::TimeReversed);
        }
        self.shared.writable()?;
        if self.flush_state.depth.get() == 0 && self.shared.answer_depth.get() == 0 {
            // An interval error is not a component's: a site of an earlier
            // operation must not be reported with it.
            self.shared.reset_failure();
        }
        self.shared.now.set(now);
        self.dispatch_intervals(now)
    }

    /// Runs every interval timer due at `now`, earliest first.
    pub(super) fn dispatch_intervals(&mut self, now: Duration) -> Result<(), RuntimeError> {
        self.intervals
            .retain(|_, interval| !interval.cancelled.get());
        let mut due: Vec<(Duration, u64)> = self
            .intervals
            .iter()
            .filter(|(_, interval)| interval.due <= now)
            .map(|(id, interval)| (interval.due, *id))
            .collect();
        due.sort_unstable();
        for (_, id) in due {
            // A nested flush or service in an earlier tick may have run and
            // rescheduled this timer already.
            if self
                .intervals
                .get(&id)
                .is_none_or(|interval| interval.due > now)
            {
                continue;
            }
            let Some(mut interval) = self.intervals.remove(&id) else {
                continue;
            };
            if interval.cancelled.get() {
                continue;
            }
            self.intervals_ticking += 1;
            let result = {
                let _errors = notify::InputErrorGuard::new(&self.shared);
                (interval.tick)(self)
            };
            self.intervals_ticking -= 1;
            let scheduled = interval.due.saturating_add(interval.period);
            interval.due = if scheduled > now {
                scheduled
            } else {
                now.saturating_add(interval.period)
            };
            if !interval.cancelled.get() && !self.shared.closed.get() {
                self.intervals.insert(id, interval);
            }
            result?;
        }
        Ok(())
    }

    /// Where the latest failing flush or paint first failed in a component:
    /// the key path to it and what failed, a callback or the reconciliation
    /// of the children it requested. A developer report, such as the native
    /// application's default error report, names it beside the error.
    ///
    /// Each outermost [`Self::flush`], [`Self::paint`],
    /// [`Self::flush_windows`] and [`Self::service_intervals`] starts without
    /// a site once it is admitted, so `None` means the latest one succeeded or
    /// failed outside any component, for example in a deferred task. A call
    /// refused at entry (a time before the runtime's current one, a forbidden
    /// phase, after shutdown) returns that error and leaves an earlier site in
    /// place; nested calls never reset it. A failure inside a component's child
    /// is reported at the child, where it arose, and the first site of an
    /// operation is kept.
    ///
    /// Children a component requested through
    /// [`crate::UpdateContext::request_children`], as [`crate::Dynamic`],
    /// [`crate::Show`] and [`crate::Each`] do, are reconciled during the
    /// flush. When they are rejected, the site is
    /// [`crate::FailurePhase::Reconcile`] at that component, with the rejected
    /// description's keys in [`crate::FailureSite::description`]. Calling
    /// [`Self::reconcile_children`] or [`Self::mount_tree`] directly returns
    /// the error to the caller and records no site; it leaves an earlier one
    /// in place.
    ///
    /// ```
    /// use pert_ui_runtime::{Column, Component, FailurePhase, IntoElement, LayoutError,
    ///     Runtime, RuntimeError, UpdateContext};
    /// struct Broken;
    /// impl Component for Broken {
    ///     fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
    ///         Err(LayoutError::InvalidChild.into())
    ///     }
    /// }
    /// let mut ui = Runtime::new();
    /// ui.mount_tree(Column::new([Broken.key("broken")]).key("form"))?;
    /// assert!(ui.flush(std::time::Duration::ZERO).is_err());
    /// let site = ui.last_failure().ok_or("a component failed")?;
    /// assert_eq!(site.path, ["form", "broken"]);
    /// assert_eq!(site.phase, FailurePhase::Update);
    /// assert_eq!(site.to_string(), "update of form/broken");
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    #[must_use]
    pub fn last_failure(&self) -> Option<crate::FailureSite> {
        self.shared.failure.borrow().clone()
    }

    /// Records the first component failure of the current flush or paint;
    /// an inner component fails before the ancestors its error passes through.
    pub(crate) fn note_failure(&self, serial: u64, phase: crate::FailurePhase) {
        self.note_failure_at(serial, phase, Vec::new());
    }

    /// [`Self::note_failure`] with the keys of a rejected description below
    /// the component, for [`crate::FailurePhase::Reconcile`].
    pub(crate) fn note_failure_at(
        &self,
        serial: u64,
        phase: crate::FailurePhase,
        description: Vec<String>,
    ) {
        if self.shared.failure.borrow().is_some() {
            return;
        }
        let mut path = Vec::new();
        let mut at = Some(serial);
        while let Some(serial) = at {
            let Some(entry) = self.entries.get(&serial) else {
                break;
            };
            path.push(entry.key.clone());
            at = entry.node.logical_parent_serial();
        }
        path.reverse();
        self.shared.failure.replace(Some(crate::FailureSite {
            path,
            phase,
            description,
        }));
    }

    /// Releases the application services at shutdown: timers stop, waiting
    /// background results are discarded, and kept values drop last-kept first.
    pub(super) fn shut_down_application_services(&mut self) {
        self.intervals.clear();
        if let Some(background) = self.background.take() {
            background.waiting.borrow_mut().clear();
            drop(background.mailbox);
        }
        while let Some(value) = self.kept.pop() {
            drop(value);
        }
    }
}
