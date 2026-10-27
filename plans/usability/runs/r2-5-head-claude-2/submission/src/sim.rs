//! The simulated background jobs: their plans, the clock and random source
//! they use, and the worker loop. Nothing here touches UI objects: a worker
//! only holds a `UiSender`, its cancel flag and the clock.

use pert_ui::{SendError, UiSender};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

/// Ten ticks per second.
pub const TICK: Duration = Duration::from_millis(100);

/// Time as the workers see it, measured from an arbitrary origin.
pub trait Clock: Send + Sync + 'static {
    fn now(&self) -> Duration;
    /// Blocks until `deadline`, or returns early once `cancel` is set.
    fn sleep_until(&self, deadline: Duration, cancel: &AtomicBool);
}

/// The wall clock.
pub struct SystemClock(Instant);

impl SystemClock {
    pub fn new() -> Self {
        SystemClock(Instant::now())
    }
}

impl Default for SystemClock {
    fn default() -> Self {
        Self::new()
    }
}

impl Clock for SystemClock {
    fn now(&self) -> Duration {
        self.0.elapsed()
    }
    fn sleep_until(&self, deadline: Duration, cancel: &AtomicBool) {
        // Short slices so that a cancel is seen within 5 ms.
        loop {
            let now = self.now();
            if now >= deadline || cancel.load(Ordering::Acquire) {
                return;
            }
            std::thread::sleep((deadline - now).min(Duration::from_millis(5)));
        }
    }
}

/// A clock that moves only when a test calls [`ManualClock::advance`].
#[derive(Default)]
pub struct ManualClock {
    now: Mutex<Duration>,
    moved: Condvar,
}

impl ManualClock {
    pub fn advance(&self, by: Duration) {
        *self.now.lock().unwrap() += by;
        self.moved.notify_all();
    }
}

impl Clock for ManualClock {
    fn now(&self) -> Duration {
        *self.now.lock().unwrap()
    }
    fn sleep_until(&self, deadline: Duration, cancel: &AtomicBool) {
        let mut now = self.now.lock().unwrap();
        while *now < deadline && !cancel.load(Ordering::Acquire) {
            // The timeout lets a cancel through without a notification.
            now = self.moved.wait_timeout(now, Duration::from_millis(1)).unwrap().0;
        }
    }
}

/// A small xorshift generator; good enough for a simulation.
#[derive(Clone)]
pub struct XorShift(u64);

impl XorShift {
    pub fn new(seed: u64) -> Self {
        XorShift(seed.max(1))
    }
    pub fn from_time() -> Self {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0x9e37_79b9);
        XorShift::new(nanos ^ 0x9e37_79b9_7f4a_7c15)
    }
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }
    /// A value in `low..=high`.
    pub fn range(&mut self, low: u64, high: u64) -> u64 {
        low + self.next_u64() % (high - low + 1)
    }
}

/// Everything random about one job, decided when it starts.
#[derive(Clone, Debug, PartialEq)]
pub struct JobPlan {
    /// Total size in units.
    pub size: u64,
    /// Units processed per tick, drawn each tick from `rate.0..=rate.1`.
    pub rate: (u64, u64),
    /// The unit count at which the job fails, if it does.
    pub fail_at: Option<u64>,
    /// Seed of the worker's per-tick draws.
    pub seed: u64,
}

/// Decides the plan of each new job. The application's random source; tests
/// replace it with a fixed script.
pub trait Planner {
    fn plan(&mut self, number: u64) -> JobPlan;
}

/// The task's distribution: 50 to 500 units, 5 to 20 per tick, one job in
/// five fails at a random point.
pub struct RandomPlanner(pub XorShift);

impl Planner for RandomPlanner {
    fn plan(&mut self, _: u64) -> JobPlan {
        let rng = &mut self.0;
        let size = rng.range(50, 500);
        let fail_at = (rng.range(1, 5) == 1).then(|| rng.range(1, size - 1));
        JobPlan { size, rate: (5, 20), fail_at, seed: rng.next_u64() }
    }
}

/// What a worker reports to the UI thread.
#[derive(Debug)]
pub enum Report {
    Progress { id: u64, done: u64, elapsed: Duration },
    Finished { id: u64, done: u64, elapsed: Duration, end: End },
}

#[derive(Clone, Debug, PartialEq)]
pub enum End {
    Done,
    /// The worker saw its cancel flag and stopped.
    Cancelled,
    Failed(String),
}

/// The worker's loop: one step per tick until the job ends.
pub fn run_job(
    id: u64,
    plan: JobPlan,
    started: Duration,
    clock: Arc<dyn Clock>,
    cancel: Arc<AtomicBool>,
    sender: UiSender<Report>,
) {
    let mut rng = XorShift::new(plan.seed);
    let mut done = 0;
    let mut tick = 0u32;
    let end = loop {
        tick += 1;
        clock.sleep_until(started + TICK * tick, &cancel);
        if cancel.load(Ordering::Acquire) {
            break End::Cancelled;
        }
        done = (done + rng.range(plan.rate.0, plan.rate.1)).min(plan.size);
        if let Some(at) = plan.fail_at.filter(|at| done >= *at) {
            done = at;
            break End::Failed(format!("I/O error at unit {at}"));
        }
        if done >= plan.size {
            break End::Done;
        }
        let elapsed = clock.now() - started;
        match sender.send(Report::Progress { id, done, elapsed }) {
            // A dropped progress report is superseded by the next one.
            Ok(()) | Err(SendError::Full(_)) => {}
            Err(SendError::Closed(_)) => return,
        }
    };
    let mut report = Report::Finished { id, done, elapsed: clock.now() - started, end };
    // The final report must arrive: retry while the mailbox is full.
    loop {
        match sender.send(report) {
            Ok(()) | Err(SendError::Closed(_)) => return,
            Err(SendError::Full(back)) => {
                report = back;
                std::thread::sleep(Duration::from_millis(2));
            }
        }
    }
}
