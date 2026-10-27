//! Headless acceptance checks: the monitor on a `Runtime` driven by the
//! library's `Driver`, with a manual clock and scripted job plans.

use job_monitor::sim::{JobPlan, Planner, TICK};
use job_monitor::*;
use pert_ui::cpu::CpuRenderer;
use pert_ui::host::HostIntegration;
use pert_ui::prelude::*;
use pert_ui::Semantics;
use pert_ui::testing::*;
use std::collections::VecDeque;
use std::rc::Rc;
use std::sync::Arc;
use std::time::Duration;

type TestResult = Result<(), Box<dyn std::error::Error>>;

const WAIT: Duration = Duration::from_secs(10);

struct Script(VecDeque<JobPlan>);
impl Planner for Script {
    fn plan(&mut self, _: u64) -> JobPlan {
        self.0.pop_front().expect("a scripted plan for every job")
    }
}

fn plan(size: u64, rate: u64, fail_at: Option<u64>) -> JobPlan {
    JobPlan { size, rate: (rate, rate), fail_at, seed: 7 }
}

struct Harness {
    ui: Runtime,
    root: WidgetId,
    monitor: Rc<Monitor>,
    clock: Arc<ManualClock>,
}

fn harness(plans: Vec<JobPlan>) -> Result<Harness, Box<dyn std::error::Error>> {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(820.0, 300.0, 1.0)?)?;
    let clock = Arc::new(ManualClock::default());
    let built = build(&mut ui, Deps { clock: clock.clone(), planner: Box::new(Script(plans.into())) })?;
    Ok(Harness { ui, root: built.root, monitor: built.monitor, clock })
}

/// Clicks Start job.
fn start(driver: &mut Driver<'_>, root: &WidgetId) -> TestResult {
    let button = driver.find(root, &["content", "toolbar", "start"])?;
    driver.click(&button)?;
    Ok(())
}

/// Moves the workers' clock by one tick and waits until the UI shows each
/// listed job at its expected unit count.
fn tick(h: &Harness, driver: &mut Driver<'_>, expected: &[(&str, u64)]) -> TestResult {
    h.clock.advance(TICK);
    let monitor = h.monitor.clone();
    driver.wait_until(WAIT, |_| {
        Ok(expected.iter().all(|(name, done)| monitor.job(name).is_some_and(|job| job.done == *done)))
    })?;
    driver.settle()?;
    Ok(())
}

fn summary(driver: &Driver<'_>, root: &WidgetId) -> Result<String, Box<dyn std::error::Error>> {
    let label = driver.find(root, &["content", "toolbar", "summary"])?;
    Ok(driver.semantics(&label)?.name.unwrap_or_default())
}

/// The row keys in display order.
fn row_keys(driver: &Driver<'_>, root: &WidgetId) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let table = driver.find(root, &["content", "jobs"])?;
    let ui = driver.runtime();
    // The body is the table's first child; it adds no segment to row paths.
    let body = ui.children(&table)?.remove(0);
    Ok(ui.children(&body)?.iter().map(|row| ui.key(row).map(str::to_owned)).collect::<Result<_, _>>()?)
}

/// What a cell shows: the semantic name of the element inside it.
fn cell(driver: &Driver<'_>, root: &WidgetId, row: &str, path: &[&str]) -> Result<Semantics, Box<dyn std::error::Error>> {
    let mut full = vec!["content", "jobs", row];
    full.extend_from_slice(path);
    Ok(driver.semantics(&driver.find(root, &full)?)?)
}

fn text(driver: &Driver<'_>, root: &WidgetId, row: &str, path: &[&str]) -> Result<String, Box<dyn std::error::Error>> {
    Ok(cell(driver, root, row, path)?.name.unwrap_or_default())
}

fn bar_value(driver: &Driver<'_>, root: &WidgetId, row: &str) -> Result<Option<f64>, Box<dyn std::error::Error>> {
    Ok(match cell(driver, root, row, &["progress", "progress", "track", "bar"])?.value {
        Some(SemanticValue::Number { value, .. }) => Some(value),
        _ => None,
    })
}

/// How many toasts the notification stack shows, as inspection reports it.
fn visible_toasts(driver: &Driver<'_>, root: &WidgetId) -> Result<f64, Box<dyn std::error::Error>> {
    let toasts = driver.find(root, &["toasts"])?;
    Ok(match driver.semantics(&toasts)?.value {
        Some(SemanticValue::Number { value, .. }) => value,
        _ => 0.0,
    })
}

/// No framework work is scheduled and nothing needs drawing.
fn assert_idle(driver: &mut Driver<'_>) -> TestResult {
    driver.runtime_mut().paint()?;
    let report = driver.settle()?;
    assert_eq!(report.components_updated, 0, "an idle flush updates nothing");
    assert_eq!(report.next_deadline, None, "no deadline while idle");
    driver.runtime_mut().paint()?;
    assert!(!driver.runtime().needs_paint(), "nothing to repaint while idle");
    Ok(())
}

#[test]
fn jobs_progress_complete_and_update_the_summary() -> TestResult {
    let mut h = harness(vec![plan(100, 10, None), plan(50, 25, None)])?;
    let (root, mut ui) = (h.root.clone(), std::mem::replace(&mut h.ui, Runtime::new()));
    let mut driver = Driver::new(&mut ui);
    driver.settle()?;
    assert_eq!(summary(&driver, &root)?, "Running: 0 · Done: 0 · Cancelled: 0 · Failed: 0");

    start(&mut driver, &root)?;
    start(&mut driver, &root)?;
    assert_eq!(row_keys(&driver, &root)?, ["job1", "job2"]);
    assert_eq!(text(&driver, &root, "job1", &["name", "name"])?, "Job 1");
    assert_eq!(text(&driver, &root, "job1", &["state", "state"])?, "running");
    assert_eq!(summary(&driver, &root)?, "Running: 2 · Done: 0 · Cancelled: 0 · Failed: 0");

    tick(&h, &mut driver, &[("Job 1", 10), ("Job 2", 25)])?;
    assert_eq!(text(&driver, &root, "job1", &["progress", "progress", "percent"])?, "10%");
    assert_eq!(text(&driver, &root, "job2", &["progress", "progress", "percent"])?, "50%");
    assert!((bar_value(&driver, &root, "job1")?.unwrap() - 0.1).abs() < 1e-6);
    assert_eq!(text(&driver, &root, "job1", &["elapsed", "elapsed"])?, "0.1 s");

    tick(&h, &mut driver, &[("Job 1", 20), ("Job 2", 50)])?;
    assert_eq!(text(&driver, &root, "job2", &["state", "state"])?, "done");
    assert_eq!(text(&driver, &root, "job2", &["progress", "progress", "percent"])?, "100%");
    assert_eq!(cell(&driver, &root, "job2", &["progress", "progress", "track", "bar"])?.description.as_deref(), Some("succeeded"));
    assert!(driver.find(&root, &["content", "jobs", "job2", "action", "cancel"]).is_err(), "a finished job has no Cancel");
    assert_eq!(summary(&driver, &root)?, "Running: 1 · Done: 1 · Cancelled: 0 · Failed: 0");

    for step in 3..=10 {
        tick(&h, &mut driver, &[("Job 1", step * 10)])?;
    }
    let monitor = h.monitor.clone();
    driver.wait_until(WAIT, |_| Ok(monitor.jobs().iter().all(|job| job.worker_ended)))?;
    driver.settle()?;
    assert_eq!(text(&driver, &root, "job1", &["state", "state"])?, "done");
    assert_eq!(text(&driver, &root, "job1", &["elapsed", "elapsed"])?, "1.0 s");
    assert_eq!(summary(&driver, &root)?, "Running: 0 · Done: 2 · Cancelled: 0 · Failed: 0");

    // With no job running the UI neither updates nor repaints, however long it waits.
    assert_idle(&mut driver)?;
    driver.advance(Duration::from_secs(30))?;
    assert_idle(&mut driver)?;
    Ok(())
}

#[test]
fn a_cancelled_job_stops() -> TestResult {
    let mut h = harness(vec![plan(100, 10, None)])?;
    let (root, mut ui) = (h.root.clone(), std::mem::replace(&mut h.ui, Runtime::new()));
    let mut driver = Driver::new(&mut ui);
    driver.settle()?;
    start(&mut driver, &root)?;
    tick(&h, &mut driver, &[("Job 1", 10)])?;
    tick(&h, &mut driver, &[("Job 1", 20)])?;

    let cancel = driver.find(&root, &["content", "jobs", "job1", "action", "cancel"])?;
    driver.click(&cancel)?;
    // The row shows the state at once, without waiting for the worker.
    assert_eq!(text(&driver, &root, "job1", &["state", "state"])?, "cancelled");
    assert_eq!(cell(&driver, &root, "job1", &["progress", "progress", "track", "bar"])?.description.as_deref(), Some("cancelled"));
    assert!(driver.find(&root, &["content", "jobs", "job1", "action", "cancel"]).is_err());
    assert_eq!(summary(&driver, &root)?, "Running: 0 · Done: 0 · Cancelled: 1 · Failed: 0");

    // The worker thread ends promptly: without the clock moving at all.
    let monitor = h.monitor.clone();
    driver.wait_until(Duration::from_secs(1), |_| Ok(monitor.job("Job 1").unwrap().worker_ended))?;

    // And it processes nothing more however far the clock goes.
    for _ in 0..5 {
        h.clock.advance(TICK);
    }
    std::thread::sleep(Duration::from_millis(50));
    driver.settle()?;
    assert_eq!(h.monitor.job("Job 1").unwrap().done, 20);
    assert_eq!(text(&driver, &root, "job1", &["progress", "progress", "percent"])?, "20%");
    assert_eq!(text(&driver, &root, "job1", &["state", "state"])?, "cancelled");
    assert_idle(&mut driver)?;
    Ok(())
}

#[test]
fn a_failure_shows_in_its_row_and_as_a_notification() -> TestResult {
    let mut h = harness(vec![plan(100, 10, Some(35)), plan(100, 10, None)])?;
    let (root, mut ui) = (h.root.clone(), std::mem::replace(&mut h.ui, Runtime::new()));
    let mut driver = Driver::new(&mut ui);
    driver.settle()?;
    start(&mut driver, &root)?;
    start(&mut driver, &root)?;
    for step in 1..=3 {
        tick(&h, &mut driver, &[("Job 1", step * 10), ("Job 2", step * 10)])?;
    }
    assert_eq!(visible_toasts(&driver, &root)?, 0.0);
    tick(&h, &mut driver, &[("Job 1", 35), ("Job 2", 40)])?;

    assert_eq!(text(&driver, &root, "job1", &["state", "state"])?, "failed: I/O error at unit 35");
    assert_eq!(text(&driver, &root, "job1", &["progress", "progress", "percent"])?, "35%");
    let bar = cell(&driver, &root, "job1", &["progress", "progress", "track", "bar"])?;
    assert_eq!(bar.description.as_deref(), Some("failed"));
    assert!(bar.invalid);
    assert_eq!(text(&driver, &root, "job2", &["state", "state"])?, "running");
    assert_eq!(summary(&driver, &root)?, "Running: 1 · Done: 0 · Cancelled: 0 · Failed: 1");
    assert_eq!(visible_toasts(&driver, &root)?, 1.0);
    let message = "Job 1 failed at 35%: I/O error at unit 35".to_owned();
    assert_eq!(h.monitor.notifications(), [(message.clone(), ToastStatus::Visible)]);

    // The notification expires on the runtime clock.
    driver.advance(Duration::from_secs(6))?;
    driver.settle()?;
    assert_eq!(visible_toasts(&driver, &root)?, 0.0);
    assert_eq!(h.monitor.notifications(), [(message, ToastStatus::Expired)]);
    Ok(())
}

/// Presses the header of `column` and returns the rows in display order.
fn sort_by(driver: &mut Driver<'_>, root: &WidgetId, column: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let table = driver.find(root, &["content", "jobs"])?;
    let first = row_keys(driver, root)?.remove(0);
    let ui = driver.runtime_mut();
    let [_, table_y, _, _] = ui.bounds(&table)?.ok_or("table laid out")?.values();
    let cell = ui.find(root, &["content", "jobs", first.as_str(), column])?.ok_or("cell")?;
    let [x, _, width, _] = ui.bounds(&cell)?.ok_or("cell laid out")?.values();
    let point = Point::new(x + width / 2.0, table_y + 8.0)?;
    for kind in [PointerKind::Press(PointerButton::Primary), PointerKind::Release(PointerButton::Primary)] {
        ui.pointer_input(PointerEvent::new(point, kind))?;
    }
    // Two header clicks must not count as a double click.
    driver.advance(ClickCounter::DEFAULT_INTERVAL + Duration::from_millis(1))?;
    row_keys(driver, root)
}

/// Jobs in four states with distinct progress and elapsed times:
///
/// | job | size | units/tick | starts | ends            | shows        |
/// |-----|------|------------|--------|-----------------|--------------|
/// | 1   | 75   | 25         | 0 ms   | done at 300     | 100%, 0.3 s  |
/// | 2   | 400  | 10         | 0 ms   | cancelled at 100| 2%, 0.1 s    |
/// | 3   | 100  | 20         | 100 ms | fails at 80, 500| 80%, 0.4 s   |
/// | 4   | 100  | 10         | 300 ms | running at 500  | 20%, 0.2 s   |
fn four_states(h: &Harness, driver: &mut Driver<'_>) -> TestResult {
    let root = h.root.clone();
    driver.settle()?;
    start(driver, &root)?;
    start(driver, &root)?;
    tick(h, driver, &[("Job 1", 25), ("Job 2", 10)])?;
    let cancel = driver.find(&root, &["content", "jobs", "job2", "action", "cancel"])?;
    driver.click(&cancel)?;
    start(driver, &root)?;
    tick(h, driver, &[("Job 1", 50), ("Job 3", 20)])?;
    let monitor = h.monitor.clone();
    tick(h, driver, &[("Job 1", 75), ("Job 3", 40)])?;
    driver.wait_until(WAIT, |_| Ok(monitor.job("Job 1").unwrap().state == JobState::Done))?;
    start(driver, &root)?;
    tick(h, driver, &[("Job 3", 60), ("Job 4", 10)])?;
    tick(h, driver, &[("Job 3", 80), ("Job 4", 20)])?;
    driver.wait_until(WAIT, |_| Ok(monitor.job("Job 3").unwrap().state == JobState::Failed))?;
    driver.settle()?;
    assert_eq!(summary(driver, &root)?, "Running: 1 · Done: 1 · Cancelled: 1 · Failed: 1");
    Ok(())
}

#[test]
fn sorting_by_each_column_orders_the_rows() -> TestResult {
    let mut h = harness(vec![
        plan(75, 25, None),
        plan(400, 10, None),
        plan(100, 20, Some(80)),
        plan(100, 10, None),
    ])?;
    let (root, mut ui) = (h.root.clone(), std::mem::replace(&mut h.ui, Runtime::new()));
    let mut driver = Driver::new(&mut ui);
    four_states(&h, &mut driver)?;
    let column_texts = |driver: &Driver<'_>, path: &[&str]| -> Result<Vec<String>, Box<dyn std::error::Error>> {
        row_keys(driver, &root)?.iter().map(|row| text(driver, &root, row, path)).collect()
    };

    // Progress: ascending, then descending on a second press.
    assert_eq!(sort_by(&mut driver, &root, "progress")?, ["job2", "job4", "job3", "job1"]);
    assert_eq!(column_texts(&driver, &["progress", "progress", "percent"])?, ["2%", "20%", "80%", "100%"]);
    assert_eq!(sort_by(&mut driver, &root, "progress")?, ["job1", "job3", "job4", "job2"]);

    // State: in lifecycle order running, done, cancelled, failed.
    assert_eq!(sort_by(&mut driver, &root, "state")?, ["job4", "job1", "job2", "job3"]);
    assert_eq!(
        column_texts(&driver, &["state", "state"])?,
        ["running", "done", "cancelled", "failed: I/O error at unit 80"]
    );
    assert_eq!(sort_by(&mut driver, &root, "state")?, ["job3", "job2", "job1", "job4"]);

    // Elapsed.
    assert_eq!(sort_by(&mut driver, &root, "elapsed")?, ["job2", "job4", "job1", "job3"]);
    assert_eq!(column_texts(&driver, &["elapsed", "elapsed"])?, ["0.1 s", "0.2 s", "0.3 s", "0.4 s"]);
    assert_eq!(sort_by(&mut driver, &root, "elapsed")?, ["job3", "job1", "job4", "job2"]);

    // Name.
    assert_eq!(sort_by(&mut driver, &root, "name")?, ["job1", "job2", "job3", "job4"]);
    assert_eq!(column_texts(&driver, &["name", "name"])?, ["Job 1", "Job 2", "Job 3", "Job 4"]);
    assert_eq!(sort_by(&mut driver, &root, "name")?, ["job4", "job3", "job2", "job1"]);

    // The order holds while the running job progresses.
    assert_eq!(sort_by(&mut driver, &root, "progress")?, ["job2", "job4", "job3", "job1"]);
    tick(&h, &mut driver, &[("Job 4", 30)])?;
    tick(&h, &mut driver, &[("Job 4", 40)])?;
    tick(&h, &mut driver, &[("Job 4", 50)])?;
    assert_eq!(row_keys(&driver, &root)?, ["job2", "job4", "job3", "job1"]);
    for done in 6..=9 {
        tick(&h, &mut driver, &[("Job 4", done * 10)])?;
    }
    assert_eq!(row_keys(&driver, &root)?, ["job2", "job3", "job4", "job1"], "90% passed 80%");
    Ok(())
}

#[test]
fn cpu_capture_of_the_table() -> TestResult {
    let mut h = harness(vec![
        plan(75, 25, None),
        plan(400, 10, None),
        plan(100, 20, Some(80)),
        plan(100, 10, None),
    ])?;
    let mut ui = std::mem::replace(&mut h.ui, Runtime::new());
    let mut driver = Driver::new(&mut ui);
    four_states(&h, &mut driver)?;
    driver.runtime().set_capture_support(true);
    let pending = driver
        .runtime()
        .request_capture(CaptureRequest::new(CaptureTarget::window(&driver.runtime().primary_window())));
    driver.settle()?;
    CpuRenderer::new().serve_captures(driver.runtime_mut());
    let capture = pending.wait()?.value?;
    assert_eq!((capture.image.width(), capture.image.height()), (820, 300));
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("capture.png");
    std::fs::write(&path, capture.image.encode_png())?;
    Ok(())
}
