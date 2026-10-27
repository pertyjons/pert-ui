//! The native application, started headless, with the wall clock and the
//! real random source.

use job_monitor::sim::Planner;
use job_monitor::*;
use pert_ui::testing::*;
use std::time::Duration;

#[test]
fn the_app_runs_real_jobs_to_completion() -> Result<(), Box<dyn std::error::Error>> {
    let (app, built) = app(Deps::real());
    let mut app = app.headless()?;
    let (root, monitor) = {
        let built = built.get().ok_or("built")?;
        (built.root.clone(), built.monitor.clone())
    };
    let mut driver = Driver::new(app.runtime_mut());
    let start = driver.find(&root, &["content", "toolbar", "start"])?;
    for _ in 0..3 {
        driver.click(&start)?;
        driver.advance(ClickCounter::DEFAULT_INTERVAL + Duration::from_millis(1))?;
    }
    assert_eq!(monitor.counts().running + monitor.counts().done + monitor.counts().failed, 3);
    // 500 units at 5 per tick is the longest job: 100 ticks, 10 s.
    driver.wait_until(Duration::from_secs(15), |_| Ok(monitor.counts().running == 0))?;
    driver.settle()?;
    let counts = monitor.counts();
    assert_eq!(counts.done + counts.failed, 3);
    for job in monitor.jobs() {
        println!("{} size {} done {} {:?} {:?}", job.name, job.size, job.done, job.state, job.elapsed);
        assert!((50..=500).contains(&job.size));
        // Ten ticks a second, 5 to 20 units a tick.
        let ticks = job.elapsed.as_secs_f64() / 0.1;
        assert!(job.done as f64 <= 20.0 * ticks.ceil() + 1.0, "{} too fast", job.name);
    }
    let label = driver.find(&root, &["content", "toolbar", "summary"])?;
    assert_eq!(driver.semantics(&label)?.name, Some(counts.text()));
    drop(driver);
    app.shutdown()?;
    Ok(())
}

#[test]
fn the_random_plans_follow_the_task() {
    let mut planner = RandomPlanner(XorShift::new(42));
    let plans: Vec<_> = (1..=10_000).map(|n| planner.plan(n)).collect();
    assert!(plans.iter().all(|p| (50..=500).contains(&p.size) && p.rate == (5, 20)));
    assert!(plans.iter().all(|p| p.fail_at.is_none_or(|at| at > 0 && at < p.size)));
    let failing = plans.iter().filter(|p| p.fail_at.is_some()).count();
    assert!((1800..=2200).contains(&failing), "about one in five fails: {failing}");
}
