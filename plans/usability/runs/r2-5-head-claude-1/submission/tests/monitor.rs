//! Headless acceptance checks: the monitor is built on a `Runtime`, driven
//! through `pert_ui::testing::Driver` and read through inspection. Workers run
//! on a `ManualClock`, so every tick happens when the test says so.

use std::collections::VecDeque;
use std::sync::Arc;
use std::time::Duration;

use job_monitor::engine::{JobPlan, ManualClock, Planner, TICK};
use job_monitor::{Environment, JobState, MonitorHandles, build};
use pert_ui::prelude::*;
use pert_ui::testing::*;
use pert_ui::{CommandKey, KeyChord, Modifiers, SortDirection, ToastStatus};

type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;

const WAIT: Duration = Duration::from_secs(5);

/// Plans handed out in order; the last one repeats.
struct Scripted(VecDeque<JobPlan>);

impl Planner for Scripted {
    fn plan(&mut self) -> JobPlan {
        if self.0.len() > 1 { self.0.pop_front().unwrap() } else { self.0[0].clone() }
    }
}

fn plan(size: u64, step: u64, fail_at: Option<u64>) -> JobPlan {
    JobPlan { size, steps: vec![step], fail_at }
}

struct Harness {
    ui: Runtime,
    clock: Arc<ManualClock>,
    app: MonitorHandles,
}

impl Harness {
    fn new(plans: impl IntoIterator<Item = JobPlan>) -> Result<Self> {
        let mut ui = Runtime::new();
        ui.set_viewport(Viewport::new(900.0, 420.0, 1.0)?)?;
        let clock = Arc::new(ManualClock::default());
        let planner = Box::new(Scripted(plans.into_iter().collect()));
        let app = build(&mut ui, Environment { clock: clock.clone(), planner })?;
        let mut harness = Harness { ui, clock, app };
        harness.driver().settle()?;
        Ok(harness)
    }

    fn driver(&mut self) -> Driver<'_> {
        Driver::new(&mut self.ui)
    }

    fn find(&mut self, path: &[&str]) -> Result<WidgetId> {
        let root = self.app.root.clone();
        Ok(self.driver().find(&root, path)?)
    }

    /// The semantic name (text) of a component below the root.
    fn text(&mut self, path: &[&str]) -> Result<String> {
        let id = self.find(path)?;
        Ok(self.driver().semantics(&id)?.name.unwrap_or_default())
    }

    /// The text a job's row shows in one column.
    fn cell(&mut self, job: u64, column: &str) -> Result<String> {
        let row = format!("job-{job}");
        let path: Vec<&str> = match column {
            "progress" => vec!["content", "jobs", &row, "progress", "progress", "percent"],
            _ => vec!["content", "jobs", &row, column, column],
        };
        self.text(&path)
    }

    fn summary(&mut self) -> Result<String> {
        self.text(&["content", "toolbar", "summary"])
    }

    /// Clicks "Start job", then lets the button's click interval pass so the
    /// next click is a single click again.
    fn start_job(&mut self) -> Result {
        let start = self.find(&["content", "toolbar", "start"])?;
        let mut driver = self.driver();
        driver.click(&start)?;
        driver.advance(ClickCounter::DEFAULT_INTERVAL + Duration::from_millis(1))?;
        Ok(())
    }

    /// Moves the job clock by one tick and waits until every job that was
    /// running has reported for it.
    fn tick(&mut self) -> Result {
        let before = self.app.jobs();
        self.clock.advance(TICK);
        let app = &self.app;
        Driver::new(&mut self.ui).wait_until(WAIT, |_| {
            let now = app.jobs();
            Ok(before.iter().zip(&now).all(|(was, is)| {
                was.state != JobState::Running || is.state != JobState::Running || is.updates > was.updates
            }))
        })?;
        Ok(())
    }

    fn ticks(&mut self, n: usize) -> Result {
        (0..n).try_for_each(|_| self.tick())
    }

    /// Row keys of the table in the order they are shown, top to bottom.
    fn row_order(&mut self) -> Vec<String> {
        let result = self.ui.query(
            &InspectionQuery::components(Selector::children(["monitor", "content", "jobs"]))
                .detail(Detail::Geometry),
        );
        let mut rows: Vec<(f32, String)> = result
            .components
            .iter()
            .filter(|c| c.key.starts_with("job-"))
            .map(|c| {
                let top = c.details.box_model.border_box.map(|b| b[1]).unwrap_or(f32::MAX);
                (top, c.key.clone())
            })
            .collect();
        rows.sort_by(|a, b| a.0.total_cmp(&b.0));
        rows.into_iter().map(|(_, key)| key).collect()
    }

    /// Sorts through the table's own keyboard path: Alt+Up (ascending) or
    /// Alt+Down (descending) on an active cell of the column.
    fn sort_by(&mut self, column: &str, ascending: bool) -> Result {
        let table = self.find(&["content", "jobs"])?;
        let body = self.ui.children(&table)?[0].clone();
        self.ui.focus(&body)?;
        let first = self.row_order().first().cloned();
        self.app.table.selection().set_active(first)?;
        self.app.table.set_active_column(Some(column.to_owned()))?;
        let code = if ascending { KeyCode::ArrowUp } else { KeyCode::ArrowDown };
        let alt = Modifiers { alt: true, ..Modifiers::default() };
        self.driver().chord(KeyChord::new(CommandKey::Named(code), alt), code)?;
        Ok(())
    }

    /// How many toasts the notification stack shows. Inspection reports the
    /// count, not the messages.
    fn visible_toasts(&mut self) -> Result<f64> {
        let stack = self.find(&["toasts"])?;
        match self.driver().semantics(&stack)?.value {
            Some(SemanticValue::Number { value, .. }) => Ok(value),
            other => Err(format!("toast stack value {other:?}").into()),
        }
    }
}

#[test]
fn jobs_progress_complete_and_update_the_summary() -> Result {
    let mut h = Harness::new([plan(100, 10, None), plan(60, 20, None)])?;
    assert_eq!(h.summary()?, "Running: 0   Done: 0   Cancelled: 0   Failed: 0");

    h.start_job()?;
    h.start_job()?;
    assert_eq!(h.summary()?, "Running: 2   Done: 0   Cancelled: 0   Failed: 0");
    assert_eq!(h.cell(1, "name")?, "Job 1");
    assert_eq!(h.cell(2, "name")?, "Job 2");
    assert_eq!(h.cell(1, "progress")?, "0%");

    h.tick()?;
    assert_eq!(h.cell(1, "progress")?, "10%");
    assert_eq!(h.cell(2, "progress")?, "33%");
    assert_eq!(h.cell(1, "elapsed")?, "0.1 s");
    assert_eq!(h.cell(1, "state")?, "running");
    // The bar reads the same fraction.
    let bar = h.find(&["content", "jobs", "job-1", "progress", "progress", "bar"])?;
    match h.driver().semantics(&bar)?.value {
        Some(SemanticValue::Number { value, .. }) => assert!((value - 0.1).abs() < 1e-6),
        other => panic!("bar value {other:?}"),
    }

    h.ticks(2)?; // Job 2: 60 units at 20 per tick
    assert_eq!(h.cell(2, "state")?, "done");
    assert_eq!(h.cell(2, "progress")?, "100%");
    assert_eq!(h.cell(1, "progress")?, "30%");
    assert_eq!(h.summary()?, "Running: 1   Done: 1   Cancelled: 0   Failed: 0");
    assert!(h.find(&["content", "jobs", "job-2", "action", "cancel"]).is_err(), "a done job has no Cancel");

    h.ticks(7)?;
    assert_eq!(h.cell(1, "state")?, "done");
    assert_eq!(h.cell(1, "elapsed")?, "1.0 s");
    assert_eq!(h.summary()?, "Running: 0   Done: 2   Cancelled: 0   Failed: 0");
    Ok(())
}

#[test]
fn nothing_is_scheduled_while_no_job_runs() -> Result {
    let mut h = Harness::new([plan(20, 10, None)])?;
    // Idle before any job: no deadline, and a flush updates nothing.
    let report = h.driver().settle()?;
    assert_eq!(report.next_deadline, None);
    assert_eq!(h.driver().settle()?.components_updated, 0);

    h.start_job()?;
    h.ticks(2)?;
    assert_eq!(h.cell(1, "state")?, "done");
    let mut driver = h.driver();
    driver.settle()?;
    let report = driver.settle()?;
    assert_eq!(report.next_deadline, None, "no timer once every job ended");
    assert_eq!(report.components_updated, 0, "nothing updates without a job");
    assert_eq!(h.app.workers.get(), 0);
    Ok(())
}

#[test]
fn a_cancelled_job_stops() -> Result {
    let mut h = Harness::new([plan(200, 10, None)])?;
    h.start_job()?;
    h.ticks(3)?;
    assert_eq!(h.cell(1, "progress")?, "15%");

    let cancel = h.find(&["content", "jobs", "job-1", "action", "cancel"])?;
    h.driver().click(&cancel)?;
    assert_eq!(h.cell(1, "state")?, "cancelled");
    assert_eq!(h.summary()?, "Running: 0   Done: 0   Cancelled: 1   Failed: 0");
    assert!(h.find(&["content", "jobs", "job-1", "action", "cancel"]).is_err(), "Cancel is gone");

    // The worker thread ends without the clock moving: the cancel wakes it.
    let workers = h.app.workers.clone();
    h.driver().wait_until(WAIT, |_| Ok(workers.get() == 0))?;

    // Time passes; the job does not move.
    h.clock.advance(TICK * 20);
    h.driver().settle()?;
    assert_eq!(h.cell(1, "progress")?, "15%");
    assert_eq!(h.cell(1, "state")?, "cancelled");
    assert_eq!(h.app.jobs()[0].done, 30);
    Ok(())
}

#[test]
fn a_failure_shows_in_its_row_and_as_a_notification() -> Result {
    let mut h = Harness::new([plan(100, 10, None), plan(100, 10, Some(35))])?;
    h.start_job()?;
    h.start_job()?;
    assert_eq!(h.visible_toasts()?, 0.0);
    h.ticks(4)?;
    assert_eq!(h.cell(2, "state")?, "failed: I/O error at unit 35");
    assert_eq!(h.cell(2, "progress")?, "35%");
    assert_eq!(h.cell(1, "state")?, "running");
    assert_eq!(h.summary()?, "Running: 1   Done: 0   Cancelled: 0   Failed: 1");

    h.driver().settle()?;
    assert_eq!(h.visible_toasts()?, 1.0);
    assert_eq!(
        h.app.notifications(),
        [("Job 2 failed at 35%: I/O error at unit 35".to_owned(), ToastStatus::Visible)]
    );
    let bar = h.find(&["content", "jobs", "job-2", "progress", "progress", "bar"])?;
    assert!(h.driver().semantics(&bar)?.invalid, "the bar reports the failure");
    Ok(())
}

/// Five jobs in all four states, started at different times.
fn mixed_states() -> Result<Harness> {
    let mut h = Harness::new([
        plan(50, 10, None),       // Job 1: done after 5 ticks
        plan(200, 10, Some(30)),  // Job 2: fails at its 3rd tick
        plan(400, 20, None),      // Job 3: cancelled
        plan(300, 5, None),       // Job 4: running, slow
        plan(120, 15, None),      // Job 5: running, fast
    ])?;
    h.start_job()?;
    h.tick()?;
    h.start_job()?;
    h.tick()?;
    h.start_job()?;
    h.start_job()?;
    h.ticks(2)?;
    let cancel = h.find(&["content", "jobs", "job-3", "action", "cancel"])?;
    h.driver().click(&cancel)?;
    h.start_job()?;
    h.ticks(3)?;
    Ok(h)
}

#[test]
fn sorting_orders_rows_by_each_column() -> Result {
    let mut h = mixed_states()?;
    let view: Vec<_> = h.app.jobs().iter().map(|j| (j.name.clone(), j.state, j.done, j.elapsed)).collect();
    // name, state, done/size, elapsed
    // Job 1 done 50/50 0.5 s; Job 2 failed 30/200 0.3 s; Job 3 cancelled 40/400 0.2 s;
    // Job 4 running 25/300 0.5 s; Job 5 running 45/120 0.3 s.
    assert_eq!(
        view.iter().map(|v| v.1).collect::<Vec<_>>(),
        [JobState::Done, JobState::Failed, JobState::Cancelled, JobState::Running, JobState::Running],
        "{view:?}"
    );
    assert_eq!(h.row_order(), ["job-1", "job-2", "job-3", "job-4", "job-5"], "unsorted: start order");

    h.sort_by("name", false)?;
    assert_eq!(h.app.table.sort().get()?.map(|s| s.column), Some("name".into()));
    assert_eq!(h.row_order(), ["job-5", "job-4", "job-3", "job-2", "job-1"]);
    h.sort_by("name", true)?;
    assert_eq!(h.row_order(), ["job-1", "job-2", "job-3", "job-4", "job-5"]);

    // Progress: 10% (Job 3), ~8% (Job 4), 15% (Job 2), 37% (Job 5), 100% (Job 1).
    h.sort_by("progress", true)?;
    assert_eq!(h.row_order(), ["job-4", "job-3", "job-2", "job-5", "job-1"]);
    h.sort_by("progress", false)?;
    assert_eq!(h.row_order(), ["job-1", "job-5", "job-2", "job-3", "job-4"]);

    // State: running, done, cancelled, failed; ties by start order.
    h.sort_by("state", true)?;
    assert_eq!(h.row_order(), ["job-4", "job-5", "job-1", "job-3", "job-2"]);
    h.sort_by("state", false)?;
    assert_eq!(h.row_order(), ["job-2", "job-3", "job-1", "job-5", "job-4"]);

    // Elapsed: 0.2 s (Job 3), 0.3 s (Jobs 2, 5), 0.5 s (Jobs 1, 4).
    h.sort_by("elapsed", true)?;
    assert_eq!(h.row_order(), ["job-3", "job-2", "job-5", "job-1", "job-4"]);
    h.sort_by("elapsed", false)?;
    assert_eq!(h.row_order(), ["job-4", "job-1", "job-5", "job-2", "job-3"]);

    // The order holds while running jobs keep reporting.
    h.tick()?;
    assert_eq!(h.row_order()[..2], ["job-4", "job-1"]);
    Ok(())
}

#[test]
fn cpu_capture_of_the_table() -> Result {
    let mut h = mixed_states()?;
    h.sort_by("name", true)?;
    let mut driver = h.driver();
    driver.runtime().set_capture_support(true);
    let pending = driver
        .runtime()
        .request_capture(CaptureRequest::new(CaptureTarget::window(&driver.runtime().primary_window())));
    driver.settle()?;
    CpuRenderer::new().serve_captures(driver.runtime_mut());
    let capture = pending.wait()?.value?;
    assert_eq!((capture.image.width(), capture.image.height()), (900, 420));
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("captures");
    std::fs::create_dir_all(&dir)?;
    std::fs::write(dir.join("jobs.png"), capture.image.encode_png())?;
    Ok(())
}

#[test]
fn a_header_click_sorts_and_flips() -> Result {
    let mut h = mixed_states()?;
    let table = h.find(&["content", "jobs"])?;
    let [x, y, _, _] = h.ui.bounds(&table)?.ok_or("table laid out")?.values();
    // Inside the "Name" title, which is the first column of the header row.
    let title = Point::new(x + 20.0, y + 10.0)?;
    let click = |h: &mut Harness| -> Result {
        for kind in [PointerKind::Press(PointerButton::Primary), PointerKind::Release(PointerButton::Primary)] {
            h.ui.pointer_input(PointerEvent::new(title, kind))?;
        }
        h.driver().advance(ClickCounter::DEFAULT_INTERVAL + Duration::from_millis(1))?;
        Ok(())
    };
    click(&mut h)?;
    assert_eq!(h.app.table.sort().get()?.map(|s| (s.column, s.direction)), Some(("name".into(), SortDirection::Ascending)));
    assert_eq!(h.row_order(), ["job-1", "job-2", "job-3", "job-4", "job-5"]);
    click(&mut h)?;
    assert_eq!(h.app.table.sort().get()?.map(|s| s.direction), Some(SortDirection::Descending));
    assert_eq!(h.row_order(), ["job-5", "job-4", "job-3", "job-2", "job-1"]);
    Ok(())
}
