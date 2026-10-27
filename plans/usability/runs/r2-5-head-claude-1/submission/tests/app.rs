//! The native application started without a window (`App::headless`), with
//! the wall clock and real worker threads.

use std::cell::RefCell;
use std::rc::Rc;
use std::time::{Duration, Instant};

use job_monitor::engine::{JobPlan, Planner, SystemClock};
use job_monitor::{Environment, JobState, MonitorHandles, app};
use pert_ui::testing::*;

struct Plans(Vec<JobPlan>);
impl Planner for Plans {
    fn plan(&mut self) -> JobPlan {
        self.0.remove(0)
    }
}

#[test]
fn the_application_runs_jobs_on_the_wall_clock() -> Result<(), Box<dyn std::error::Error>> {
    let built: Rc<RefCell<Option<MonitorHandles>>> = Rc::default();
    let slot = built.clone();
    let env = Environment {
        clock: std::sync::Arc::new(SystemClock::default()),
        planner: Box::new(Plans(vec![
            JobPlan { size: 50, steps: vec![10], fail_at: None },
            JobPlan { size: 500, steps: vec![5], fail_at: None },
        ])),
    };
    let mut native = app(env, move |handles| *slot.borrow_mut() = Some(handles)).headless()?;
    let h = built.borrow().clone().ok_or("not built")?;

    let start = Driver::new(native.runtime_mut()).find(&h.root, &["content", "toolbar", "start"])?;
    let mut driver = Driver::new(native.runtime_mut());
    driver.click(&start)?;
    driver.advance(ClickCounter::DEFAULT_INTERVAL + Duration::from_millis(1))?;
    driver.click(&start)?;

    // Job 1 takes five ticks of 100 ms.
    let began = Instant::now();
    driver.wait_until(Duration::from_secs(5), |_| Ok(h.jobs()[0].state == JobState::Done))?;
    assert!(began.elapsed() >= Duration::from_millis(450), "10 ticks per second");
    assert!(h.jobs()[1].done > 0, "Job 2 is moving");

    // Cancel Job 2; its thread ends promptly on the wall clock.
    let cancel = driver.find(&h.root, &["content", "jobs", "job-2", "action", "cancel"])?;
    driver.click(&cancel)?;
    let asked = Instant::now();
    let workers = h.workers.clone();
    driver.wait_until(Duration::from_secs(1), |_| Ok(workers.get() == 0))?;
    assert!(asked.elapsed() < Duration::from_millis(50), "stopped in {:?}", asked.elapsed());
    let stopped_at = h.jobs()[1].done;
    std::thread::sleep(Duration::from_millis(300));
    driver.settle()?;
    assert_eq!(h.jobs()[1].done, stopped_at);
    assert_eq!(h.jobs()[1].state, JobState::Cancelled);
    assert_eq!(h.summary.get()?, "Running: 0   Done: 1   Cancelled: 1   Failed: 0");
    drop(driver);
    native.shutdown()?;
    Ok(())
}
