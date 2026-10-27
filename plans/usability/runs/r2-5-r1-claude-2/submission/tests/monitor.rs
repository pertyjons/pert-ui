//! Headless checks of the job monitor, driven through pert-ui's own runtime,
//! input and capture facilities with a deterministic clock and random source.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::Duration;

use job_monitor::*;
use pert_ui::cpu::CpuRenderer;
use pert_ui::prelude::*;
use pert_ui::{
    CaptureRequest, CaptureTarget, InspectValue, KeyAction, KeyCode, Point, PointerButton, PointerEvent,
    PointerKind, ToastStatus, Viewport,
};

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// A clock the test advances one tick at a time.
struct StepPacer {
    ticks: Receiver<()>,
    processed: Sender<()>,
    count: u32,
}

impl Pacer for StepPacer {
    fn next_tick(&mut self, cancel: &CancelToken) -> Pace {
        if self.count > 0 {
            // The previous tick's event has been sent.
            let _ = self.processed.send(());
        }
        let elapsed = |n: u32| TICK * n;
        match self.ticks.recv() {
            Ok(()) => {
                self.count += 1;
                if cancel.is_cancelled() {
                    Pace::Cancelled(elapsed(self.count))
                } else {
                    Pace::Tick(elapsed(self.count))
                }
            }
            Err(_) => Pace::Cancelled(elapsed(self.count)),
        }
    }
}

/// The test's side of one job's clock.
struct Driver {
    ticks: Sender<()>,
    processed: Receiver<()>,
}

impl Driver {
    /// Advances one tick and waits until the worker has handled it. Returns
    /// false once the worker has ended.
    fn step(&self) -> bool {
        self.ticks.send(()).is_ok() && self.processed.recv().is_ok()
    }
}

#[derive(Clone, Default)]
struct Drivers(Rc<RefCell<HashMap<u32, Driver>>>);

impl Drivers {
    fn step(&self, id: u32, ticks: usize) {
        for _ in 0..ticks {
            if !self.0.borrow()[&id].step() {
                break;
            }
        }
    }
}

/// An environment with scripted plans (falling back to a seeded generator)
/// and stepped clocks.
fn test_env(plans: Vec<JobPlan>, drivers: &Drivers) -> Env {
    let mut plans = plans.into_iter();
    let mut rng = XorShift::new(7);
    let drivers = drivers.clone();
    Env {
        planner: Box::new(move |_| plans.next().unwrap_or_else(|| random_plan(&mut rng))),
        pacers: Box::new(move |id| {
            let (tick_tx, tick_rx) = channel();
            let (done_tx, done_rx) = channel();
            drivers.0.borrow_mut().insert(id, Driver { ticks: tick_tx, processed: done_rx });
            Box::new(StepPacer { ticks: tick_rx, processed: done_tx, count: 0 })
        }),
    }
}

fn plan(total: u32, step: u32, fail_at: Option<u32>) -> JobPlan {
    JobPlan { total, steps: vec![step], fail_at }
}

struct Session {
    ui: Runtime,
    monitor: MonitorHandle,
    drivers: Drivers,
    now: Duration,
}

impl Session {
    fn new(plans: Vec<JobPlan>) -> Result<Self, Box<dyn std::error::Error>> {
        let mut ui = Runtime::new();
        ui.set_viewport(Viewport::new(900.0, 420.0, 1.0)?)?;
        let drivers = Drivers::default();
        let monitor = build(&mut ui, test_env(plans, &drivers))?;
        let mut session = Session { ui, monitor, drivers, now: Duration::ZERO };
        session.flush()?;
        Ok(session)
    }

    fn flush(&mut self) -> Result<(), RuntimeError> {
        self.ui.flush(self.now)?;
        Ok(())
    }

    /// Advances the runtime clock by `by` and flushes.
    fn advance(&mut self, by: Duration) -> Result<(), RuntimeError> {
        self.now += by;
        self.flush()
    }

    fn find(&self, key: &str) -> Result<WidgetId, Box<dyn std::error::Error>> {
        Ok(self
            .ui
            .find_descendant(&self.monitor.root, key)?
            .ok_or_else(|| format!("no component keyed {key}"))?)
    }

    /// Clicks the centre of a component with the primary button.
    fn click(&mut self, id: &WidgetId) -> TestResult {
        let rect = self.ui.bounds(id)?.ok_or("component has no bounds")?;
        let point = Point::new(rect.x() + rect.width() / 2.0, rect.y() + rect.height() / 2.0)?;
        self.click_at(point)
    }

    fn click_at(&mut self, point: Point) -> TestResult {
        for kind in [
            PointerKind::Move,
            PointerKind::Press(PointerButton::Primary),
            PointerKind::Release(PointerButton::Primary),
        ] {
            self.ui.pointer_input(PointerEvent::new(point, kind))?;
        }
        self.flush()?;
        Ok(())
    }

    /// Presses the "Start job" button through the keyboard.
    fn start_job(&mut self) -> TestResult {
        let start = self.find("start")?;
        self.ui.focus(&start)?;
        self.ui.key_input(KeyCode::Enter, KeyAction::Press)?;
        self.ui.key_input(KeyCode::Enter, KeyAction::Release)?;
        self.flush()?;
        Ok(())
    }

    /// Steps a job's clock and lets the UI receive what it sent.
    fn tick(&mut self, id: u32, ticks: usize) -> Result<(), RuntimeError> {
        self.drivers.step(id, ticks);
        self.advance(TICK)
    }

    fn job(&self, id: u32) -> JobView {
        self.monitor.rows().into_iter().find(|j| j.id == id).expect("job exists")
    }

    /// The row keys in the order the table draws them, top to bottom.
    fn shown_order(&self) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let table = self.find("jobs")?;
        let body = self.ui.children(&table)?[0].clone();
        let mut rows = Vec::new();
        for row in self.ui.children(&body)? {
            let y = self.ui.bounds(&row)?.map(|r| r.y()).unwrap_or(f32::MAX);
            rows.push((y, self.ui.key(&row)?.to_owned()));
        }
        rows.sort_by(|a, b| a.0.total_cmp(&b.0));
        Ok(rows.into_iter().map(|(_, key)| key).collect())
    }

    /// Text shown by the labels inside the row with `row_key`, via inspection.
    fn texts(&self) -> Vec<String> {
        self.ui
            .inspect()
            .components
            .into_iter()
            .filter_map(|c| c.details.semantics.and_then(|s| s.name))
            .collect()
    }

    /// Clicks the header title of a column.
    fn sort_by(&mut self, column: &str) -> TestResult {
        let table = self.ui.bounds(&self.find("jobs")?)?.ok_or("table has no bounds")?;
        let mut x = table.x();
        for (key, _, width) in COLUMNS {
            if key == column {
                break;
            }
            x += width;
        }
        self.click_at(Point::new(x + 12.0, table.y() + 8.0)?)
    }
}

#[test]
fn starts_jobs_and_shows_progress_completion_and_summary() -> TestResult {
    let mut s = Session::new(vec![plan(100, 10, None), plan(60, 20, None)])?;
    assert_eq!(s.monitor.summary_text()?, "Running: 0   Done: 0   Cancelled: 0   Failed: 0");

    s.start_job()?;
    s.start_job()?;
    assert_eq!(s.monitor.summary(), Summary { running: 2, ..Summary::default() });
    assert_eq!(s.shown_order()?, ["job-1", "job-2"]);

    s.tick(1, 3)?;
    let job = s.job(1);
    assert_eq!((job.done, job.state, job.elapsed), (30, JobState::Running, TICK * 3));
    let texts = s.texts();
    assert!(texts.iter().any(|t| t == "30%"), "percentage shown: {texts:?}");
    assert!(texts.iter().any(|t| t == "0.3 s"), "elapsed shown: {texts:?}");

    // Job 2 finishes after three ticks of 20 units.
    s.tick(2, 3)?;
    assert_eq!(s.job(2).state, JobState::Done);
    assert_eq!(s.monitor.summary(), Summary { running: 1, done: 1, ..Summary::default() });
    assert_eq!(s.monitor.summary_text()?, "Running: 1   Done: 1   Cancelled: 0   Failed: 0");
    assert!(s.texts().iter().any(|t| t == "done"));
    assert!(s.texts().iter().any(|t| t.starts_with("Running: 1   Done: 1")), "summary label shown");

    s.tick(1, 10)?;
    assert_eq!(s.job(1).state, JobState::Done);
    assert_eq!(s.job(1).done, 100);
    assert_eq!(s.monitor.summary(), Summary { done: 2, ..Summary::default() });

    // Idle: nothing is scheduled and nothing needs painting.
    s.ui.paint()?;
    assert_eq!(s.ui.next_deadline(), None);
    assert!(!s.ui.needs_paint());
    Ok(())
}

#[test]
fn cancel_stops_a_running_job() -> TestResult {
    let mut s = Session::new(vec![plan(200, 10, None), plan(200, 10, None)])?;
    s.start_job()?;
    s.start_job()?;
    s.tick(1, 4)?;
    s.tick(2, 4)?;
    assert_eq!(s.job(1).done, 40);

    // Press the Cancel button in job 1's row.
    let table = s.find("jobs")?;
    let row = s.ui.find_descendant(&table, "job-1")?.ok_or("row 1 mounted")?;
    let cancel = s.ui.find_descendant(&row, "cancel")?.ok_or("cancel button")?;
    s.click(&cancel)?;

    // The worker notices at its next wake-up and ends.
    s.tick(1, 1)?;
    assert_eq!(s.job(1).state, JobState::Cancelled);
    assert_eq!(s.job(1).done, 40, "no unit processed after the cancel");
    assert!(!s.drivers.0.borrow()[&1].step(), "the worker thread has ended");
    s.advance(TICK)?;
    assert_eq!(s.job(1).done, 40);
    assert!(s.texts().iter().any(|t| t == "cancelled"));
    // The cancelled row offers no Cancel action any more.
    let row = s.ui.find_descendant(&table, "job-1")?.ok_or("row 1 mounted")?;
    assert!(s.ui.find_descendant(&row, "cancel")?.is_none());

    // The other job is unaffected.
    s.tick(2, 2)?;
    assert_eq!((s.job(2).done, s.job(2).state), (60, JobState::Running));
    assert_eq!(s.monitor.summary(), Summary { running: 1, cancelled: 1, ..Summary::default() });
    Ok(())
}

#[test]
fn cancel_is_prompt_with_the_real_clock() -> TestResult {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(900.0, 420.0, 1.0)?)?;
    let env = Env {
        planner: Box::new(|_| plan(500, 5, None)),
        pacers: Box::new(|_| Box::new(RealPacer::new())),
    };
    let monitor = build(&mut ui, env)?;
    let start = std::time::Instant::now();
    let clock = |ui: &mut Runtime| ui.flush(start.elapsed()).map(|_| ());
    clock(&mut ui)?;
    let button = ui.find_descendant(&monitor.root, "start")?.ok_or("start")?;
    ui.focus(&button)?;
    ui.key_input(KeyCode::Enter, KeyAction::Press)?;
    ui.key_input(KeyCode::Enter, KeyAction::Release)?;
    while monitor.rows()[0].done < 10 {
        std::thread::sleep(Duration::from_millis(5));
        clock(&mut ui)?;
    }
    let table = ui.find_descendant(&monitor.root, "jobs")?.ok_or("table")?;
    let cancel = ui.find_descendant(&table, "cancel")?.ok_or("cancel")?;
    ui.focus(&cancel)?;
    ui.key_input(KeyCode::Space, KeyAction::Press)?;
    ui.key_input(KeyCode::Space, KeyAction::Release)?;
    let cancelled_at = std::time::Instant::now();
    while monitor.rows()[0].state == JobState::Running {
        assert!(cancelled_at.elapsed() < Duration::from_millis(50), "cancel took too long");
        std::thread::sleep(Duration::from_millis(1));
        clock(&mut ui)?;
    }
    assert_eq!(monitor.rows()[0].state, JobState::Cancelled);
    Ok(())
}

#[test]
fn failure_shows_in_row_and_notification() -> TestResult {
    let mut s = Session::new(vec![plan(100, 10, Some(35)), plan(100, 10, None)])?;
    s.start_job()?;
    s.start_job()?;
    s.tick(1, 3)?;
    assert_eq!(s.job(1).state, JobState::Running);
    s.tick(1, 1)?;
    let job = s.job(1);
    assert_eq!((job.state, job.done), (JobState::Failed, 35));
    assert_eq!(s.monitor.summary(), Summary { running: 1, failed: 1, ..Summary::default() });

    // In its row.
    let table = s.find("jobs")?;
    let row = s.ui.find_descendant(&table, "job-1")?.ok_or("row 1")?;
    let state = s.ui.find_descendant(&row, "state-label")?.ok_or("state cell")?;
    let shown = s
        .ui
        .inspect()
        .components
        .into_iter()
        .find(|c| c.id == s.ui.inspection_id(&state).unwrap())
        .and_then(|c| c.details.semantics.and_then(|s| s.name));
    assert_eq!(shown.as_deref(), Some("failed: simulated error at unit 35"));

    // And as a notification.
    let toasts = s.monitor.toasts();
    assert_eq!(toasts.len(), 1);
    assert_eq!(toasts[0].0, "Job 1 failed: simulated error at unit 35");
    assert_eq!(toasts[0].1.status(), ToastStatus::Visible);
    // Inspection does not expose a toast's text; it does count the visible ones.
    let stack = s.ui.inspect().components.into_iter().find(|c| c.key == "toasts").ok_or("toast stack")?;
    assert_eq!(stack.details.properties.get("visible_count"), Some(&InspectValue::Number(1.0)));
    s.ui.set_capture_support(true);
    let capture = s.ui.request_capture(CaptureRequest::new(CaptureTarget::window(&s.ui.primary_window())));
    s.flush()?;
    CpuRenderer::new().serve_captures(&mut s.ui);
    let image = capture.try_reply()?.value?.image;
    std::fs::write(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("capture-failure.png"), image.encode_png())?;

    // The notification expires and, with job 2 finished, the UI goes idle.
    s.tick(2, 10)?;
    assert_eq!(s.monitor.summary().running, 0);
    // No job runs: the only wake-up left is the toast's expiry, not a frame loop.
    s.ui.paint()?;
    let deadline = s.ui.next_deadline().ok_or("toast expiry scheduled")?;
    assert!(deadline > s.now + Duration::from_secs(4), "deadline {deadline:?} at {:?}", s.now);
    s.advance(Duration::from_secs(6))?;
    assert_eq!(toasts[0].1.status(), ToastStatus::Expired);
    s.flush()?;
    s.ui.paint()?;
    assert_eq!(s.ui.next_deadline(), None);
    assert!(!s.ui.needs_paint());
    Ok(())
}

#[test]
fn sorting_by_each_column_orders_rows() -> TestResult {
    // Job 1: done quickly (short elapsed). Job 2: failed. Job 3: cancelled.
    // Job 4: running, far along. Job 5: running, barely started.
    let mut s = Session::new(vec![
        plan(50, 25, None),
        plan(100, 10, Some(42)),
        plan(400, 10, None),
        plan(100, 10, None),
        plan(400, 5, None),
    ])?;
    for _ in 0..5 {
        s.start_job()?;
    }
    s.tick(1, 2)?; // done at 0.2 s
    s.tick(2, 5)?; // failed at 42/100, 0.5 s
    s.tick(3, 3)?; // 30/400, then cancelled at 0.4 s
    s.monitor.rows().iter().find(|j| j.id == 3).expect("job 3");
    let table = s.find("jobs")?;
    let row = s.ui.find_descendant(&table, "job-3")?.ok_or("row 3")?;
    let cancel = s.ui.find_descendant(&row, "cancel")?.ok_or("cancel 3")?;
    s.click(&cancel)?;
    s.tick(3, 1)?;
    s.tick(4, 8)?; // 80/100 running, 0.8 s
    s.tick(5, 1)?; // 5/400 running, 0.1 s
    let states: Vec<_> = s.monitor.rows().iter().map(|j| j.state).collect();
    assert_eq!(
        states,
        [JobState::Done, JobState::Failed, JobState::Cancelled, JobState::Running, JobState::Running]
    );

    let keys = |ids: &[u32]| ids.iter().map(|id| format!("job-{id}")).collect::<Vec<_>>();
    let cases: [(&str, [u32; 5], [u32; 5]); 4] = [
        // progress fractions: 1.0, 0.42, 0.075, 0.8, 0.0125
        ("progress", [5, 3, 2, 4, 1], [1, 4, 2, 3, 5]),
        // Running < Done < Cancelled < Failed, ties by id
        ("state", [4, 5, 1, 3, 2], [2, 3, 1, 5, 4]),
        // elapsed: 0.2, 0.5, 0.4, 0.8, 0.1 s
        ("elapsed", [5, 1, 3, 2, 4], [4, 2, 3, 1, 5]),
        ("name", [1, 2, 3, 4, 5], [5, 4, 3, 2, 1]),
    ];
    for (column, ascending, descending) in cases {
        s.sort_by(column)?;
        assert_eq!(s.shown_order()?, keys(&ascending), "{column} ascending");
        let sort = s.monitor.table.sort().get()?.ok_or("sort indication")?;
        assert_eq!(sort.column, column);
        s.sort_by(column)?;
        assert_eq!(s.shown_order()?, keys(&descending), "{column} descending");
    }

    // A sorted table keeps its order as progress changes it.
    s.sort_by("progress")?; // name descending -> progress ascending
    s.tick(5, 30)?; // 155/400 = 0.3875
    assert_eq!(s.shown_order()?, keys(&[3, 5, 2, 4, 1]));
    Ok(())
}

#[test]
fn captures_the_table_as_png() -> TestResult {
    let mut s = Session::new(vec![
        plan(50, 25, None),
        plan(100, 10, Some(42)),
        plan(400, 10, None),
        plan(100, 10, None),
        plan(400, 5, None),
    ])?;
    for _ in 0..5 {
        s.start_job()?;
    }
    s.tick(1, 2)?;
    s.tick(2, 5)?;
    s.tick(3, 3)?;
    let table = s.find("jobs")?;
    let row = s.ui.find_descendant(&table, "job-3")?.ok_or("row 3")?;
    let cancel = s.ui.find_descendant(&row, "cancel")?.ok_or("cancel 3")?;
    s.click(&cancel)?;
    s.tick(3, 1)?;
    s.tick(4, 6)?;
    s.tick(5, 9)?;
    s.sort_by("state")?;

    s.ui.set_capture_support(true);
    let window = s.ui.request_capture(CaptureRequest::new(CaptureTarget::window(&s.ui.primary_window())));
    let jobs = s
        .ui
        .request_capture(CaptureRequest::new(CaptureTarget::Component(s.ui.inspection_id(&table)?)));
    s.flush()?;
    let mut renderer = CpuRenderer::new();
    assert_eq!(renderer.serve_captures(&mut s.ui), 2);
    let window = window.try_reply()?.value?;
    let jobs = jobs.try_reply()?.value?;
    let out = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    std::fs::write(out.join("capture-window.png"), window.image.encode_png())?;
    std::fs::write(out.join("capture-table.png"), jobs.image.encode_png())?;
    assert!(jobs.image.width() > 0 && jobs.image.height() > 0);
    Ok(())
}
