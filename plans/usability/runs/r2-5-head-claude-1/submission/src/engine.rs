//! The simulated jobs: plans, the clock they run on and the worker threads.
//!
//! Nothing here touches UI objects. A worker reports through a `Report`
//! callback that the UI wires to a `UiSender`.

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

/// One tick of every job: 10 ticks per second.
pub const TICK: Duration = Duration::from_millis(100);

/// The time source of the workers.
pub trait Clock: Send + Sync + 'static {
    /// Time since an arbitrary fixed origin.
    fn now(&self) -> Duration;
    /// Blocks until `deadline`, returning early once `cancel` is set.
    fn sleep_until(&self, deadline: Duration, cancel: &AtomicBool);
}

/// The wall clock.
pub struct SystemClock {
    origin: Instant,
}

impl Default for SystemClock {
    fn default() -> Self {
        SystemClock { origin: Instant::now() }
    }
}

impl Clock for SystemClock {
    fn now(&self) -> Duration {
        self.origin.elapsed()
    }
    fn sleep_until(&self, deadline: Duration, cancel: &AtomicBool) {
        // Short slices so that a cancellation is seen within 5 ms.
        while !cancel.load(Ordering::Acquire) {
            let now = self.now();
            if now >= deadline {
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
            // The cancel flag does not notify the condvar, so poll it.
            now = self.moved.wait_timeout(now, Duration::from_millis(2)).unwrap().0;
        }
    }
}

/// What a job will do, decided when it starts.
#[derive(Clone, Debug, PartialEq)]
pub struct JobPlan {
    /// Total units, 50 to 500.
    pub size: u64,
    /// Units processed per tick, 5 to 20 each; tick `n` uses `steps[n % len]`.
    pub steps: Vec<u64>,
    /// The job fails once this many units are processed.
    pub fail_at: Option<u64>,
}

/// The random source of job plans.
pub trait Planner {
    fn plan(&mut self) -> JobPlan;
}

/// Random plans from a xorshift generator.
pub struct RandomPlanner {
    state: u64,
}

impl RandomPlanner {
    pub fn seeded(seed: u64) -> Self {
        RandomPlanner { state: seed | 1 }
    }

    pub fn from_time() -> Self {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0x9E37_79B9_7F4A_7C15);
        Self::seeded(nanos)
    }

    fn next(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        x
    }

    /// Uniform in `low..=high`.
    fn between(&mut self, low: u64, high: u64) -> u64 {
        low + self.next() % (high - low + 1)
    }
}

impl Planner for RandomPlanner {
    fn plan(&mut self) -> JobPlan {
        let size = self.between(50, 500);
        let steps = (0..32).map(|_| self.between(5, 20)).collect();
        let fail_at = (self.between(1, 5) == 1).then(|| self.between(1, size - 1));
        JobPlan { size, steps, fail_at }
    }
}

/// What a worker reports about its job.
#[derive(Clone, Debug, PartialEq)]
pub enum Report {
    Progress { id: u64, done: u64, elapsed: Duration },
    Done { id: u64, elapsed: Duration },
    Failed { id: u64, done: u64, elapsed: Duration, reason: String },
    /// The worker saw its cancel flag and stopped.
    Stopped { id: u64, done: u64, elapsed: Duration },
}

/// Counts the worker threads that are still running.
#[derive(Clone, Default)]
pub struct WorkerCount(Arc<AtomicUsize>);

impl WorkerCount {
    pub fn get(&self) -> usize {
        self.0.load(Ordering::Acquire)
    }
}

struct Live(WorkerCount);

impl Drop for Live {
    fn drop(&mut self) {
        (self.0).0.fetch_sub(1, Ordering::AcqRel);
    }
}

/// Starts a worker thread for one job. `report` returns `false` once nobody
/// listens any more, which ends the worker.
pub fn spawn_worker(
    id: u64,
    plan: JobPlan,
    clock: Arc<dyn Clock>,
    cancel: Arc<AtomicBool>,
    workers: &WorkerCount,
    mut report: impl FnMut(Report) -> bool + Send + 'static,
) {
    workers.0.fetch_add(1, Ordering::AcqRel);
    let live = Live(workers.clone());
    // Read here, not in the thread, so the job starts when it was asked to.
    let start = clock.now();
    std::thread::spawn(move || {
        let _live = live;
        let mut done = 0;
        for tick in 1u32.. {
            clock.sleep_until(start + TICK * tick, &cancel);
            let elapsed = clock.now() - start;
            if cancel.load(Ordering::Acquire) {
                report(Report::Stopped { id, done, elapsed });
                return;
            }
            let step = plan.steps[(tick as usize - 1) % plan.steps.len().max(1)];
            done = (done + step).min(plan.size);
            if let Some(fail_at) = plan.fail_at.filter(|&at| done >= at) {
                let reason = format!("I/O error at unit {fail_at}");
                report(Report::Failed { id, done: fail_at, elapsed, reason });
                return;
            }
            if done >= plan.size {
                report(Report::Done { id, elapsed });
                return;
            }
            if !report(Report::Progress { id, done, elapsed }) {
                return;
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn random_plans_stay_in_range() {
        let mut planner = RandomPlanner::seeded(42);
        let plans: Vec<_> = (0..1000).map(|_| planner.plan()).collect();
        for plan in &plans {
            assert!((50..=500).contains(&plan.size));
            assert!(plan.steps.iter().all(|s| (5..=20).contains(s)));
            assert!(plan.fail_at.is_none_or(|at| at >= 1 && at < plan.size));
        }
        let failing = plans.iter().filter(|p| p.fail_at.is_some()).count();
        assert!((150..250).contains(&failing), "about one in five fails: {failing}");
    }
}
