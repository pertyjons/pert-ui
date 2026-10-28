//! Headless checks of the job monitor, through pert-ui's testing driver.
//! The clock is manual and the job plans are fixed, so every run is the same.

use job_monitor::*;
use pert_ui::prelude::*;
use pert_ui::testing::*;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

type TestResult = Result<(), Box<dyn std::error::Error>>;

const WAIT: Duration = Duration::from_secs(5);

fn plan(total: u64, steps: &[u64], fail_at_tick: Option<usize>) -> JobPlan {
    JobPlan { total, steps: steps.to_vec(), fail_at_tick }
}

struct Harness {
    ui: Runtime,
    monitor: Monitor,
    clock: Arc<ManualClock>,
    ticks: u32,
}

impl Harness {
    fn new(plans: Vec<JobPlan>) -> Result<Harness, Box<dyn std::error::Error>> {
        let mut plans: HashMap<u32, JobPlan> =
            plans.into_iter().enumerate().map(|(i, p)| (i as u32 + 1, p)).collect();
        let clock = ManualClock::new();
        let env = Env {
            clock: clock.clone(),
            planner: Box::new(move |number: u32| plans.remove(&number).expect("a plan per job")),
        };
        let mut ui = Runtime::new();
        ui.set_viewport(Viewport::new(800.0, 360.0, 1.0)?)?;
        let monitor = build(&mut ui, env)?;
        Driver::new(&mut ui).settle()?;
        Ok(Harness { ui, monitor, clock, ticks: 0 })
    }

    fn driver(&mut self) -> Driver<'_> {
        Driver::new(&mut self.ui)
    }

    fn start(&mut self) -> TestResult {
        let start = self.monitor.start.clone();
        let mut driver = Driver::new(&mut self.ui);
        driver.click(&start)?;
        // Separate clicks, not a double click.
        driver.advance(ClickCounter::DEFAULT_INTERVAL + Duration::from_millis(1))?;
        Ok(())
    }

    /// Moves the job clock one tick and waits until every job that was
    /// running reported it (or stopped).
    fn tick(&mut self) -> TestResult {
        self.ticks += 1;
        self.clock.advance(TICK);
        let now = TICK * self.ticks;
        let monitor = &self.monitor;
        Driver::new(&mut self.ui).wait_until(WAIT, |_| {
            Ok(monitor
                .jobs()
                .iter()
                .all(|job| job.stopped || job.state != JobState::Running || job.elapsed >= now))
        })?;
        Ok(())
    }

    fn text_at(&mut self, path: &[&str]) -> Result<Option<String>, TestError> {
        let table = self.monitor.table.clone();
        let driver = self.driver();
        let id = driver.find(&table, path)?;
        Ok(driver.semantics(&id)?.name)
    }

    fn rows(&mut self) -> Result<Vec<String>, TestError> {
        let table = self.monitor.table.clone();
        Ok(self.driver().address_children(&table)?.into_iter().map(|row| row.segment).collect())
    }

    fn summary_text(&mut self) -> Result<Option<String>, TestError> {
        let label = self.monitor.summary_label.clone();
        Ok(self.driver().semantics(&label)?.name)
    }

    fn click_title(&mut self, index: usize) -> TestResult {
        let table = self.monitor.table.clone();
        let mut driver = self.driver();
        driver.click_item(&table, index)?;
        driver.advance(ClickCounter::DEFAULT_INTERVAL + Duration::from_millis(1))?;
        Ok(())
    }

    fn cancel(&mut self, row: &str) -> TestResult {
        let table = self.monitor.table.clone();
        let mut driver = self.driver();
        let button = driver.find(&table, &[row, "action", "cancel"])?;
        driver.click(&button)?;
        Ok(())
    }

    fn toast_messages(&mut self) -> Result<Vec<String>, TestError> {
        let root = self.monitor.root.clone();
        let driver = self.driver();
        let stack = driver.find(&root, &["toasts"])?;
        Ok(driver.semantics(&stack)?.items.into_iter().filter_map(|item| item.name).collect())
    }
}

#[test]
fn starts_jobs_and_shows_progress_completion_and_summary() -> TestResult {
    let mut h = Harness::new(vec![plan(50, &[10; 5], None), plan(100, &[20; 5], None)])?;
    assert_eq!(h.summary_text()?.as_deref(), Some("Running: 0 · Done: 0 · Cancelled: 0 · Failed: 0"));
    // Idle before any job: nothing scheduled, nothing to paint.
    assert!(h.driver().quiet_for(Duration::from_secs(1))?.is_idle());

    h.start()?;
    h.start()?;
    assert_eq!(h.rows()?, ["job-1", "job-2"]);
    assert_eq!(h.text_at(&["job-1", "name", "text"])?.as_deref(), Some("Job 1"));
    assert_eq!(h.text_at(&["job-2", "state", "text"])?.as_deref(), Some("running"));
    assert_eq!(h.summary_text()?.as_deref(), Some("Running: 2 · Done: 0 · Cancelled: 0 · Failed: 0"));

    h.tick()?;
    assert_eq!(h.text_at(&["job-1", "progress", "cell", "percent"])?.as_deref(), Some("20%"));
    assert_eq!(h.text_at(&["job-2", "progress", "cell", "percent"])?.as_deref(), Some("20%"));
    assert_eq!(h.text_at(&["job-1", "elapsed", "text"])?.as_deref(), Some("0.1 s"));
    h.tick()?;
    h.tick()?;
    assert_eq!(h.text_at(&["job-1", "progress", "cell", "percent"])?.as_deref(), Some("60%"));
    assert_eq!(h.text_at(&["job-2", "progress", "cell", "percent"])?.as_deref(), Some("60%"));
    h.tick()?;
    h.tick()?;
    assert_eq!(h.text_at(&["job-1", "progress", "cell", "percent"])?.as_deref(), Some("100%"));
    assert_eq!(h.text_at(&["job-1", "state", "text"])?.as_deref(), Some("done"));
    assert_eq!(h.text_at(&["job-2", "state", "text"])?.as_deref(), Some("done"));
    assert_eq!(h.text_at(&["job-2", "elapsed", "text"])?.as_deref(), Some("0.5 s"));
    assert_eq!(h.summary_text()?.as_deref(), Some("Running: 0 · Done: 2 · Cancelled: 0 · Failed: 0"));
    // A finished job has no Cancel action.
    let table = h.monitor.table.clone();
    assert!(h.driver().find(&table, &["job-1", "action", "cancel"]).is_err());

    // No job running: the window stays quiet.
    let report = h.driver().quiet_for(Duration::from_secs(1))?;
    assert!(report.is_idle(), "{report:?}");
    Ok(())
}

#[test]
fn cancelling_a_running_job_stops_it() -> TestResult {
    let mut h = Harness::new(vec![plan(100, &[10; 10], None), plan(100, &[10; 10], None)])?;
    h.start()?;
    h.start()?;
    h.tick()?;
    h.tick()?;
    assert_eq!(h.text_at(&["job-1", "progress", "cell", "percent"])?.as_deref(), Some("20%"));

    h.cancel("job-1")?;
    assert_eq!(h.text_at(&["job-1", "state", "text"])?.as_deref(), Some("cancelled"));
    assert_eq!(h.summary_text()?.as_deref(), Some("Running: 1 · Done: 0 · Cancelled: 1 · Failed: 0"));
    let table = h.monitor.table.clone();
    assert!(h.driver().find(&table, &["job-1", "action", "cancel"]).is_err());

    // The worker stops at its next tick, and its row no longer moves.
    h.tick()?;
    let monitor = &h.monitor;
    Driver::new(&mut h.ui).wait_until(WAIT, |_| Ok(monitor.jobs()[0].stopped))?;
    h.tick()?;
    h.tick()?;
    let jobs = h.monitor.jobs();
    assert_eq!((jobs[0].done, jobs[0].state.clone()), (20, JobState::Cancelled));
    assert_eq!(jobs[1].done, 50);
    assert_eq!(h.text_at(&["job-1", "progress", "cell", "percent"])?.as_deref(), Some("20%"));
    assert_eq!(h.text_at(&["job-1", "elapsed", "text"])?.as_deref(), Some("0.2 s"));
    assert_eq!(h.text_at(&["job-2", "progress", "cell", "percent"])?.as_deref(), Some("50%"));
    Ok(())
}

#[test]
fn a_failure_shows_in_its_row_and_as_a_notification() -> TestResult {
    let mut h = Harness::new(vec![plan(100, &[20; 5], Some(2))])?;
    h.start()?;
    h.tick()?;
    assert!(h.toast_messages()?.is_empty());
    h.tick()?;
    assert_eq!(
        h.text_at(&["job-1", "state", "text"])?.as_deref(),
        Some("failed: I/O error at unit 40")
    );
    assert_eq!(h.text_at(&["job-1", "progress", "cell", "percent"])?.as_deref(), Some("40%"));
    assert_eq!(h.summary_text()?.as_deref(), Some("Running: 0 · Done: 0 · Cancelled: 0 · Failed: 1"));
    assert_eq!(h.toast_messages()?, ["Job 1 failed: I/O error at unit 40"]);
    assert!(h.monitor.jobs()[0].stopped);

    // With nothing running only the toast's expiry is scheduled: no frame
    // is drawn while it waits.
    let report = h.driver().quiet_for(Duration::from_secs(1))?;
    assert_eq!(report.components_updated, 0, "{report:?}");
    assert!(!report.needs_paint, "{report:?}");
    Ok(())
}

/// Four jobs, one in each state, with distinct progress and elapsed times:
///
/// | job | state     | progress | elapsed |
/// |-----|-----------|----------|---------|
/// | 1   | done      | 100%     | 0.2 s   |
/// | 2   | running   | 10%      | 0.4 s   |
/// | 3   | cancelled | 30%      | 0.1 s   |
/// | 4   | failed    | 60%      | 0.3 s   |
fn four_states() -> Result<Harness, Box<dyn std::error::Error>> {
    let mut h = Harness::new(vec![
        plan(50, &[25, 25], None),
        plan(400, &[10; 40], None),
        plan(100, &[30; 4], None),
        plan(100, &[20; 5], Some(3)),
    ])?;
    for _ in 0..4 {
        h.start()?;
    }
    h.tick()?;
    h.cancel("job-3")?;
    h.tick()?;
    h.tick()?;
    h.tick()?;
    assert_eq!(h.summary_text()?.as_deref(), Some("Running: 1 · Done: 1 · Cancelled: 1 · Failed: 1"));
    Ok(h)
}

#[test]
fn sorting_by_each_column_orders_the_rows() -> TestResult {
    let mut h = four_states()?;
    assert_eq!(h.rows()?, ["job-1", "job-2", "job-3", "job-4"]);

    // Titles: 0 name, 1 progress, 2 state, 3 elapsed.
    let cases: [(usize, [&str; 4]); 4] = [
        (0, ["job-1", "job-2", "job-3", "job-4"]),
        (1, ["job-2", "job-3", "job-4", "job-1"]),
        (2, ["job-2", "job-1", "job-3", "job-4"]),
        (3, ["job-3", "job-1", "job-4", "job-2"]),
    ];
    for (title, ascending) in cases {
        h.click_title(title)?;
        assert_eq!(h.rows()?, ascending, "column {title} ascending");
        h.click_title(title)?;
        let mut descending = ascending;
        descending.reverse();
        assert_eq!(h.rows()?, descending, "column {title} descending");
    }

    // The order holds while the running job keeps reporting.
    h.click_title(1)?; // progress, ascending
    h.tick()?;
    assert_eq!(h.rows()?, ["job-2", "job-3", "job-4", "job-1"]);
    assert_eq!(h.text_at(&["job-2", "progress", "cell", "percent"])?.as_deref(), Some("12%"));
    Ok(())
}

#[test]
fn sorting_by_name_is_numeric() -> TestResult {
    let plans = (0..10).map(|_| plan(100, &[10; 10], None)).collect();
    let mut h = Harness::new(plans)?;
    for _ in 0..10 {
        h.start()?;
    }
    h.click_title(0)?;
    h.click_title(0)?;
    let rows = h.rows()?;
    assert_eq!(rows.first().map(String::as_str), Some("job-10"));
    assert_eq!(rows.last().map(String::as_str), Some("job-1"));
    Ok(())
}

#[test]
fn cpu_capture_of_jobs_in_several_states() -> TestResult {
    let mut h = four_states()?;
    let mut driver = h.driver();
    driver.runtime().set_capture_support(true);
    let pending = driver.runtime().request_capture(CaptureRequest::new(CaptureTarget::window(
        &driver.runtime().primary_window(),
    )));
    driver.settle()?;
    CpuRenderer::new().serve_captures(driver.runtime_mut());
    let capture = pending.try_reply()?.value?;
    let image: &FrameImage = &capture.image;
    assert_eq!((image.width(), image.height()), (800, 360));
    let file = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("capture.png");
    std::fs::write(&file, image.encode_png())?;
    assert!(std::fs::read(&file)?.starts_with(b"\x89PNG"));
    // Nothing taller than a row, nothing unbounded.
    #[cfg(debug_assertions)]
    assert!(driver.runtime().validation_report().issues.is_empty(), "{:?}", driver.runtime().validation_report());
    Ok(())
}

/// The native application's builder, started without a window, on the
/// wall clock with real worker threads.
#[test]
fn the_app_runs_jobs_on_the_wall_clock() -> TestResult {
    let env = Env {
        clock: Arc::new(SystemClock::new()),
        planner: Box::new(|_: u32| plan(60, &[20, 20, 20], None)),
    };
    let mut app = app(env).headless()?;
    let (ui, monitor) = app.runtime_and_state_mut();
    let mut driver = Driver::new(ui);
    driver.click(&monitor.start)?;
    driver.wait_until(WAIT, |_| Ok(monitor.jobs()[0].state == JobState::Done))?;
    assert!(monitor.jobs()[0].elapsed >= Duration::from_millis(300));
    assert_eq!(monitor.summary.get()?, "Running: 0 · Done: 1 · Cancelled: 0 · Failed: 0");
    assert!(driver.quiet_for(Duration::from_secs(1))?.is_idle());
    app.shutdown()?;
    Ok(())
}

#[test]
fn random_plans_stay_in_range_and_one_in_five_fails() {
    let mut rng = XorShift::new(42);
    let mut failures = 0;
    for _ in 0..5000 {
        let plan = JobPlan::random(&mut rng);
        assert!((50..=500).contains(&plan.total));
        assert!(plan.steps.iter().all(|step| (5..=20).contains(step)));
        assert!(plan.steps.iter().sum::<u64>() >= plan.total);
        if let Some(tick) = plan.fail_at_tick {
            assert!((1..=plan.steps.len()).contains(&tick));
            failures += 1;
        }
    }
    assert!((800..1200).contains(&failures), "{failures} failures in 5000");
}
