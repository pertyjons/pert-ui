//! The simulated background jobs: their plans, the clock that paces them and
//! the worker loop. Nothing here knows about the UI; workers report through a
//! callback the UI side gives them, which is a mailbox sender.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// The period of one tick: ten ticks per second.
pub const TICK: Duration = Duration::from_millis(100);

/// What a job will do, decided on the UI thread when it starts.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JobPlan {
    /// Total size in units.
    pub total: u32,
    /// Units processed on each tick, in order; they add up to at least `total`.
    pub steps: Vec<u32>,
    /// The zero-based tick on which the job fails, if it does.
    pub fail_at_tick: Option<usize>,
}

/// Decides the plan of each new job.
pub trait Planner {
    fn plan(&mut self, job_number: u32) -> JobPlan;
}

/// A source of random numbers that tests can replace.
pub trait Random {
    fn next_u64(&mut self) -> u64;

    /// A value in `low..=high`.
    fn between(&mut self, low: u32, high: u32) -> u32 {
        low + (self.next_u64() % u64::from(high - low + 1)) as u32
    }
}

/// SplitMix64: small, fast and good enough for a simulation.
pub struct SplitMix64(pub u64);

impl SplitMix64 {
    pub fn from_time() -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0x5eed);
        SplitMix64(nanos)
    }
}

impl Random for SplitMix64 {
    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }
}

/// The task's rules: 50 to 500 units, 5 to 20 units per tick, and one job in
/// five fails at a random tick.
pub struct RandomPlanner<R: Random>(pub R);

impl<R: Random> Planner for RandomPlanner<R> {
    fn plan(&mut self, _job_number: u32) -> JobPlan {
        let total = self.0.between(50, 500);
        let mut steps = Vec::new();
        let mut sum = 0;
        while sum < total {
            let step = self.0.between(5, 20);
            steps.push(step);
            sum += step;
        }
        let fails = self.0.between(1, 5) == 1;
        let fail_at_tick = fails.then(|| self.0.between(0, steps.len() as u32 - 1) as usize);
        JobPlan { total, steps, fail_at_tick }
    }
}

/// Paces the workers. Shared by every worker thread.
pub trait Clock: Send + Sync + 'static {
    /// The tick count a new worker starts from.
    fn start(&self) -> u64;
    /// Blocks until the tick after `seen` has come, then advances `seen`.
    /// Returns `false` without waiting further once `cancel` is set.
    fn wait_tick(&self, seen: &mut u64, cancel: &AtomicBool) -> bool;
    /// Wakes every waiting worker, so a cancelled one stops at once.
    fn wake(&self);
}

/// Real time: one tick every [`TICK`].
#[derive(Default)]
pub struct RealClock {
    lock: Mutex<()>,
    changed: Condvar,
}

impl Clock for RealClock {
    fn start(&self) -> u64 {
        0
    }

    fn wait_tick(&self, seen: &mut u64, cancel: &AtomicBool) -> bool {
        let deadline = Instant::now() + TICK;
        let mut guard = self.lock.lock().unwrap_or_else(|e| e.into_inner());
        loop {
            if cancel.load(Ordering::SeqCst) {
                return false;
            }
            let now = Instant::now();
            if now >= deadline {
                *seen += 1;
                return true;
            }
            guard = self
                .changed
                .wait_timeout(guard, deadline - now)
                .unwrap_or_else(|e| e.into_inner())
                .0;
        }
    }

    fn wake(&self) {
        let _guard = self.lock.lock().unwrap_or_else(|e| e.into_inner());
        self.changed.notify_all();
    }
}

/// A clock that only moves when a test advances it.
#[derive(Default)]
pub struct ManualClock {
    ticks: Mutex<u64>,
    changed: Condvar,
}

impl ManualClock {
    /// Lets every worker process `ticks` more ticks.
    pub fn advance(&self, ticks: u64) {
        *self.ticks.lock().unwrap_or_else(|e| e.into_inner()) += ticks;
        self.changed.notify_all();
    }
}

impl Clock for ManualClock {
    fn start(&self) -> u64 {
        *self.ticks.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn wait_tick(&self, seen: &mut u64, cancel: &AtomicBool) -> bool {
        let mut ticks = self.ticks.lock().unwrap_or_else(|e| e.into_inner());
        loop {
            if cancel.load(Ordering::SeqCst) {
                return false;
            }
            if *ticks > *seen {
                *seen += 1;
                return true;
            }
            ticks = self.changed.wait(ticks).unwrap_or_else(|e| e.into_inner());
        }
    }

    fn wake(&self) {
        let _ticks = self.ticks.lock().unwrap_or_else(|e| e.into_inner());
        self.changed.notify_all();
    }
}

/// How a job stands after a report.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Outcome {
    Running,
    Done,
    Cancelled,
    Failed(String),
}

/// One report from a worker. Owned and `Send`: it is all the worker shares.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JobEvent {
    pub id: u32,
    pub done: u32,
    pub ticks: u32,
    pub outcome: Outcome,
}

/// Where a worker sends its reports. `send` returns `false` when nobody
/// listens any more, which ends the job. `terminal` reports must not be lost;
/// running reports may be dropped when the receiver is busy.
pub trait Reporter: Send + 'static {
    fn send(&self, event: JobEvent, terminal: bool) -> bool;
}

/// Runs one job to its end on the calling thread.
pub fn run_job(id: u32, plan: JobPlan, clock: Arc<dyn Clock>, cancel: Arc<AtomicBool>, out: impl Reporter) {
    let mut seen = clock.start();
    let mut done = 0;
    for (tick, step) in plan.steps.iter().enumerate() {
        let ticks = tick as u32;
        if !clock.wait_tick(&mut seen, &cancel) {
            out.send(JobEvent { id, done, ticks, outcome: Outcome::Cancelled }, true);
            return;
        }
        let ticks = ticks + 1;
        if plan.fail_at_tick == Some(tick) {
            let reason = format!("checksum error at unit {done}");
            out.send(JobEvent { id, done, ticks, outcome: Outcome::Failed(reason) }, true);
            return;
        }
        done = (done + step).min(plan.total);
        if done >= plan.total {
            out.send(JobEvent { id, done, ticks, outcome: Outcome::Done }, true);
            return;
        }
        if !out.send(JobEvent { id, done, ticks, outcome: Outcome::Running }, false) {
            return;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn random_plans_follow_the_rules() {
        let mut planner = RandomPlanner(SplitMix64(7));
        let mut failures = 0;
        for n in 1..=1000 {
            let plan = planner.plan(n);
            assert!((50..=500).contains(&plan.total));
            assert!(plan.steps.iter().all(|s| (5..=20).contains(s)));
            assert!(plan.steps.iter().sum::<u32>() >= plan.total);
            if let Some(tick) = plan.fail_at_tick {
                assert!(tick < plan.steps.len());
                failures += 1;
            }
        }
        assert!((150..=250).contains(&failures), "about one in five fail: {failures}");
    }

    struct Channel(std::sync::mpsc::Sender<(JobEvent, Instant)>);
    impl Reporter for Channel {
        fn send(&self, event: JobEvent, _: bool) -> bool {
            self.0.send((event, Instant::now())).is_ok()
        }
    }

    #[test]
    fn real_time_cancellation_is_prompt() {
        let clock: Arc<dyn Clock> = Arc::new(RealClock::default());
        let cancel = Arc::new(AtomicBool::new(false));
        let (tx, rx) = std::sync::mpsc::channel();
        let plan = JobPlan { total: 500, steps: vec![5; 100], fail_at_tick: None };
        let worker = {
            let (clock, cancel) = (clock.clone(), cancel.clone());
            std::thread::spawn(move || run_job(1, plan, clock, cancel, Channel(tx)))
        };
        let (first, _) = rx.recv().unwrap();
        assert_eq!((first.done, first.outcome), (5, Outcome::Running));
        std::thread::sleep(Duration::from_millis(30));
        let cancelled_at = Instant::now();
        cancel.store(true, Ordering::SeqCst);
        clock.wake();
        let (last, at) = rx.iter().last().unwrap();
        worker.join().unwrap();
        assert_eq!(last.outcome, Outcome::Cancelled);
        assert!(at - cancelled_at < Duration::from_millis(20), "stopped in {:?}", at - cancelled_at);
    }
}
