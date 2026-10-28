//! Independent acceptance tests (verifier, acceptance copy only).
//!
//! Every observation goes through the library: pointer clicks through
//! `Driver`, state read from `Runtime::inspect` (paths, semantics, border
//! boxes), idleness from `Driver::quiet_for`, the capture from `CpuRenderer`.
//! The application's own model (`Monitor::jobs`) is never read. The public
//! `Env` replaces the clock and the planner, as the task allows. Worker
//! threads are observed through `/proc/self/task/*/comm` (the app names them
//! `job-N`), so run this binary with `--test-threads=1`.

use job_monitor::*;
use pert_ui::InspectionSnapshot;
use pert_ui::prelude::*;
use pert_ui::testing::*;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

type R<T = ()> = Result<T, Box<dyn std::error::Error>>;

const WAIT: Duration = Duration::from_secs(5);
const TABLE: [&str; 3] = ["monitor", "content", "jobs"];

fn plan(total: u64, steps: &[u64], fail_at_tick: Option<usize>) -> JobPlan {
    JobPlan { total, steps: steps.to_vec(), fail_at_tick }
}

// ---------------------------------------------------------------------------
// Inspection helpers
// ---------------------------------------------------------------------------

fn comp<'a>(s: &'a InspectionSnapshot, path: &[&str]) -> Option<&'a ComponentInspection> {
    s.components.iter().find(|c| {
        c.path
            .as_ref()
            .is_some_and(|p| p.iter().map(String::as_str).eq(path.iter().copied()))
    })
}

fn semantics<'a>(s: &'a InspectionSnapshot, path: &[&str]) -> Option<&'a Semantics> {
    comp(s, path)?.details.semantics.as_ref()
}

fn row_path<'a>(row: &'a str, rest: &[&'a str]) -> Vec<&'a str> {
    let mut path = TABLE.to_vec();
    path.push(row);
    path.extend_from_slice(rest);
    path
}

fn cell(s: &InspectionSnapshot, row: &str, rest: &[&str]) -> Option<String> {
    semantics(s, &row_path(row, rest))?.name.clone()
}

fn state(s: &InspectionSnapshot, row: &str) -> Option<String> {
    cell(s, row, &["state", "text"])
}

fn percent(s: &InspectionSnapshot, row: &str) -> Option<String> {
    cell(s, row, &["progress", "cell", "percent"])
}

fn elapsed(s: &InspectionSnapshot, row: &str) -> Option<String> {
    cell(s, row, &["elapsed", "text"])
}

fn bar(s: &InspectionSnapshot, row: &str) -> Option<f64> {
    let sem = semantics(s, &row_path(row, &["progress", "cell", "bar"]))?;
    assert_eq!(sem.role, SemanticRole::ProgressIndicator);
    match sem.value.as_ref()? {
        SemanticValue::Number { value, .. } => Some(*value),
        _ => None,
    }
}

/// The bar's status as its semantics describe it ("determinate",
/// "succeeded", "failed", "cancelled"); a succeeded bar reports no fraction.
fn bar_status(s: &InspectionSnapshot, row: &str) -> Option<String> {
    semantics(s, &row_path(row, &["progress", "cell", "bar"]))?.description.clone()
}

fn has_cancel(s: &InspectionSnapshot, row: &str) -> bool {
    semantics(s, &row_path(row, &["action", "cancel"]))
        .is_some_and(|sem| sem.role == SemanticRole::Button && sem.name.as_deref() == Some("Cancel"))
}

fn summary(s: &InspectionSnapshot) -> Option<String> {
    semantics(s, &["monitor", "content", "toolbar", "summary"])?.name.clone()
}

/// Rows in the order they are drawn: row components below the table, sorted
/// by the top of their border box, named by their Name cell.
fn visual_rows(s: &InspectionSnapshot) -> Vec<String> {
    let mut rows: Vec<(f32, String)> = s
        .components
        .iter()
        .filter(|c| c.visible)
        .filter_map(|c| {
            let p = c.path.as_ref()?;
            if p.len() != 4 || !p.iter().map(String::as_str).take(3).eq(TABLE) || !p[3].starts_with("job-") {
                return None;
            }
            let y = c.details.box_model.border_box?[1];
            Some((y, cell(s, &p[3], &["name", "text"])?))
        })
        .collect();
    rows.sort_by(|a, b| a.0.total_cmp(&b.0));
    let ys: Vec<f32> = rows.iter().map(|r| r.0).collect();
    assert!(ys.windows(2).all(|w| w[0] < w[1]), "rows overlap: {ys:?}");
    rows.into_iter().map(|r| r.1).collect()
}

/// The notification stack: its role, its count and its messages.
fn toasts(s: &InspectionSnapshot) -> (Option<SemanticRole>, f64, Vec<String>) {
    let Some(sem) = semantics(s, &["monitor", "toasts"]) else {
        return (None, 0.0, Vec::new());
    };
    let count = match sem.value {
        Some(SemanticValue::Number { value, .. }) => value,
        _ => 0.0,
    };
    (Some(sem.role), count, sem.items.iter().filter_map(|i| i.name.clone()).collect())
}

/// The table's header titles, from its semantic items.
fn titles(s: &InspectionSnapshot) -> Vec<Option<String>> {
    semantics(s, &TABLE).map(|t| t.items.iter().map(|i| i.name.clone()).collect()).unwrap_or_default()
}

// ---------------------------------------------------------------------------
// Worker threads, as the OS sees them
// ---------------------------------------------------------------------------

fn thread_names() -> Vec<String> {
    std::fs::read_dir("/proc/self/task")
        .map(|dir| {
            dir.filter_map(|e| std::fs::read_to_string(e.ok()?.path().join("comm")).ok())
                .map(|n| n.trim().to_owned())
                .collect()
        })
        .unwrap_or_default()
}

fn thread_alive(name: &str) -> bool {
    thread_names().iter().any(|n| n == name)
}

/// Wall time until no thread has `name`, or None after `limit`.
fn thread_gone_within(name: &str, limit: Duration) -> Option<Duration> {
    let start = Instant::now();
    while start.elapsed() < limit {
        if !thread_alive(name) {
            return Some(start.elapsed());
        }
        std::thread::sleep(Duration::from_millis(1));
    }
    None
}

// ---------------------------------------------------------------------------
// A deterministic harness on the manual clock
// ---------------------------------------------------------------------------

struct H {
    ui: Runtime,
    monitor: Monitor,
    clock: Arc<ManualClock>,
}

impl H {
    fn new(plans: Vec<JobPlan>, width: f32, height: f32) -> R<H> {
        let mut plans: HashMap<u32, JobPlan> =
            plans.into_iter().enumerate().map(|(i, p)| (i as u32 + 1, p)).collect();
        let clock = ManualClock::new();
        let env = Env {
            clock: clock.clone(),
            planner: Box::new(move |n: u32| plans.remove(&n).expect("a plan per job")),
        };
        let mut ui = Runtime::new();
        ui.set_viewport(Viewport::new(width, height, 1.0)?)?;
        let monitor = build(&mut ui, env)?;
        Driver::new(&mut ui).settle()?;
        Ok(H { ui, monitor, clock })
    }

    fn snap(&self) -> InspectionSnapshot {
        self.ui.inspect()
    }

    fn click_start(&mut self) -> R {
        let start = self.monitor.start.clone();
        let mut d = Driver::new(&mut self.ui);
        d.click(&start)?;
        d.advance(ClickCounter::DEFAULT_INTERVAL + Duration::from_millis(1))?;
        Ok(())
    }

    fn click_cancel(&mut self, row: &str) -> R {
        let table = self.monitor.table.clone();
        let mut d = Driver::new(&mut self.ui);
        let button = d.find(&table, &[row, "action", "cancel"])?;
        d.click(&button)?;
        d.advance(ClickCounter::DEFAULT_INTERVAL + Duration::from_millis(1))?;
        Ok(())
    }

    fn click_title(&mut self, index: usize) -> R {
        let table = self.monitor.table.clone();
        let mut d = Driver::new(&mut self.ui);
        d.click_item(&table, index)?;
        d.advance(ClickCounter::DEFAULT_INTERVAL + Duration::from_millis(1))?;
        Ok(())
    }

    /// Moves the job clock and flushes until `ready` holds on an inspection.
    fn tick_until(&mut self, ready: impl Fn(&InspectionSnapshot) -> bool) -> R {
        self.clock.advance(TICK);
        Driver::new(&mut self.ui).wait_until(WAIT, |rt| Ok(ready(&rt.inspect())))?;
        Ok(())
    }
}

/// Releases workers still parked on the manual clock, so no `job-N` thread
/// outlives its test.
impl Drop for H {
    fn drop(&mut self) {
        self.clock.advance(Duration::from_secs(100_000));
    }
}

fn close(a: Option<f64>, b: f64) -> bool {
    a.is_some_and(|a| (a - b).abs() < 1e-4)
}

// ---------------------------------------------------------------------------
// Item 1: progress, completion, summary
// ---------------------------------------------------------------------------

#[test]
fn item1_progress_completion_and_summary_through_inspection() -> R {
    let mut h = H::new(
        vec![
            plan(60, &[20, 20, 20], None),
            plan(75, &[15; 5], None),
            plan(50, &[20, 20, 10], None),
        ],
        800.0,
        360.0,
    )?;
    let s = h.snap();
    assert_eq!(summary(&s).as_deref(), Some("Running: 0 · Done: 0 · Cancelled: 0 · Failed: 0"));
    assert!(visual_rows(&s).is_empty());
    assert_eq!(
        titles(&s)[..4],
        [Some("Name".into()), Some("Progress".into()), Some("State".into()), Some("Elapsed".into())]
    );

    h.click_start()?;
    h.click_start()?;
    let s = h.snap();
    assert_eq!(visual_rows(&s), ["Job 1", "Job 2"]);
    assert_eq!(state(&s, "job-1").as_deref(), Some("running"));
    assert_eq!(percent(&s, "job-1").as_deref(), Some("0%"));
    assert!(close(bar(&s, "job-1"), 0.0));
    assert!(has_cancel(&s, "job-1") && has_cancel(&s, "job-2"));
    assert_eq!(summary(&s).as_deref(), Some("Running: 2 · Done: 0 · Cancelled: 0 · Failed: 0"));

    h.tick_until(|s| percent(s, "job-1").as_deref() == Some("33%") && percent(s, "job-2").as_deref() == Some("20%"))?;
    let s = h.snap();
    assert!(close(bar(&s, "job-1"), 20.0 / 60.0) && close(bar(&s, "job-2"), 0.2));
    assert_eq!(elapsed(&s, "job-1").as_deref(), Some("0.1 s"));

    h.tick_until(|s| percent(s, "job-2").as_deref() == Some("40%"))?;
    h.tick_until(|s| state(s, "job-1").as_deref() == Some("done") && percent(s, "job-2").as_deref() == Some("60%"))?;
    let s = h.snap();
    assert_eq!(percent(&s, "job-1").as_deref(), Some("100%"));
    assert_eq!(bar_status(&s, "job-1").as_deref(), Some("succeeded"));
    assert_eq!(bar_status(&s, "job-2").as_deref(), Some("determinate"));
    assert_eq!(elapsed(&s, "job-1").as_deref(), Some("0.3 s"));
    assert!(!has_cancel(&s, "job-1") && has_cancel(&s, "job-2"));
    assert_eq!(summary(&s).as_deref(), Some("Running: 1 · Done: 1 · Cancelled: 0 · Failed: 0"));

    // A job started later gets the next name and its own schedule.
    h.click_start()?;
    let s = h.snap();
    assert_eq!(visual_rows(&s), ["Job 1", "Job 2", "Job 3"]);
    assert_eq!(summary(&s).as_deref(), Some("Running: 2 · Done: 1 · Cancelled: 0 · Failed: 0"));
    h.tick_until(|s| percent(s, "job-2").as_deref() == Some("80%") && percent(s, "job-3").as_deref() == Some("40%"))?;
    h.tick_until(|s| state(s, "job-2").as_deref() == Some("done") && percent(s, "job-3").as_deref() == Some("80%"))?;
    h.tick_until(|s| state(s, "job-3").as_deref() == Some("done"))?;
    let s = h.snap();
    assert_eq!(elapsed(&s, "job-2").as_deref(), Some("0.5 s"));
    assert_eq!(elapsed(&s, "job-3").as_deref(), Some("0.3 s"));
    assert_eq!(summary(&s).as_deref(), Some("Running: 0 · Done: 3 · Cancelled: 0 · Failed: 0"));
    for row in ["job-1", "job-2", "job-3"] {
        assert_eq!(percent(&s, row).as_deref(), Some("100%"));
        assert!(!has_cancel(&s, row));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Item 2: cancellation stops the worker
// ---------------------------------------------------------------------------

#[test]
fn item2_cancel_stops_the_worker_thread_and_the_row() -> R {
    let mut h = H::new(vec![plan(100, &[10; 10], None), plan(100, &[10; 10], None)], 800.0, 360.0)?;
    h.click_start()?;
    h.click_start()?;
    h.tick_until(|s| percent(s, "job-1").as_deref() == Some("10%") && percent(s, "job-2").as_deref() == Some("10%"))?;
    h.tick_until(|s| percent(s, "job-1").as_deref() == Some("20%") && percent(s, "job-2").as_deref() == Some("20%"))?;
    assert!(thread_alive("job-1") && thread_alive("job-2"), "{:?}", thread_names());

    h.click_cancel("job-1")?;
    let s = h.snap();
    assert_eq!(state(&s, "job-1").as_deref(), Some("cancelled"));
    assert!(!has_cancel(&s, "job-1") && has_cancel(&s, "job-2"));
    assert_eq!(summary(&s).as_deref(), Some("Running: 1 · Done: 0 · Cancelled: 1 · Failed: 0"));

    // The worker is parked until its next tick; at that tick it exits.
    h.tick_until(|s| percent(s, "job-2").as_deref() == Some("30%"))?;
    assert!(thread_gone_within("job-1", WAIT).is_some(), "job-1 still running: {:?}", thread_names());
    assert!(thread_alive("job-2"));

    for expected in ["40%", "50%"] {
        h.tick_until(|s| percent(s, "job-2").as_deref() == Some(expected))?;
    }
    let s = h.snap();
    assert_eq!(state(&s, "job-1").as_deref(), Some("cancelled"));
    assert_eq!(percent(&s, "job-1").as_deref(), Some("20%"));
    assert!(close(bar(&s, "job-1"), 0.2));
    assert_eq!(bar_status(&s, "job-1").as_deref(), Some("cancelled"));
    assert_eq!(elapsed(&s, "job-1").as_deref(), Some("0.2 s"));
    assert_eq!(summary(&s).as_deref(), Some("Running: 1 · Done: 0 · Cancelled: 1 · Failed: 0"));

    // Cancelling the last running job ends its thread too.
    h.click_cancel("job-2")?;
    h.clock.advance(TICK);
    assert!(thread_gone_within("job-2", WAIT).is_some());
    let mut d = Driver::new(&mut h.ui);
    d.wait_until(Duration::from_millis(200), |_| Ok(false)).ok();
    let s = d.runtime().inspect();
    assert_eq!(summary(&s).as_deref(), Some("Running: 0 · Done: 0 · Cancelled: 2 · Failed: 0"));
    assert_eq!(percent(&s, "job-2").as_deref(), Some("50%"));
    // Nothing is left to repaint once both workers ended.
    let report = d.quiet_for(Duration::from_secs(1))?;
    assert!(report.is_idle(), "{report:?}");
    Ok(())
}

/// The real application on the wall clock: Cancel stops the worker within
/// about one tick, and the window is idle afterwards.
#[test]
fn item2_cancel_is_prompt_on_the_wall_clock() -> R {
    let env = Env {
        clock: Arc::new(SystemClock::new()),
        planner: Box::new(|_: u32| plan(500, &[5; 100], None)),
    };
    let mut app = app(env).headless()?;
    let (ui, monitor) = app.runtime_and_state_mut();
    ui.set_viewport(Viewport::new(800.0, 360.0, 1.0)?)?;
    let mut d = Driver::new(ui);
    d.settle()?;
    d.click(&monitor.start)?;
    d.wait_until(WAIT, |rt| Ok(bar(&rt.inspect(), "job-1").is_some_and(|v| v >= 0.02)))?;
    assert!(thread_alive("job-1"));
    let button = d.find(&monitor.table, &["job-1", "action", "cancel"])?;
    d.click(&button)?;
    let clicked = Instant::now();
    let gone = thread_gone_within("job-1", WAIT).ok_or("worker did not stop")?;
    println!("worker job-1 exited {:?} after the Cancel click", gone);
    assert!(gone <= Duration::from_millis(400), "worker took {gone:?}");
    // Let the worker's last report arrive, then read the row twice.
    d.wait_until(Duration::from_millis(150), |_| Ok(false)).ok();
    let first = d.runtime().inspect();
    assert_eq!(state(&first, "job-1").as_deref(), Some("cancelled"));
    let frozen = percent(&first, "job-1");
    std::thread::sleep(Duration::from_millis(300));
    d.wait_until(Duration::from_millis(10), |_| Ok(false)).ok();
    let later = d.runtime().inspect();
    assert_eq!(percent(&later, "job-1"), frozen);
    assert!(frozen.as_deref() != Some("100%"));
    assert_eq!(summary(&later).as_deref(), Some("Running: 0 · Done: 0 · Cancelled: 1 · Failed: 0"));
    println!("cancelled at {frozen:?}, {:?} after click", clicked.elapsed());
    let report = d.quiet_for(Duration::from_secs(1))?;
    assert!(report.is_idle(), "{report:?}");
    app.shutdown()?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Item 3: failure in the row and as a notification
// ---------------------------------------------------------------------------

#[test]
fn item3_failure_in_row_and_notification() -> R {
    let mut h = H::new(
        vec![plan(100, &[10; 10], Some(3)), plan(100, &[25; 4], None), plan(100, &[20; 5], Some(1))],
        800.0,
        360.0,
    )?;
    h.click_start()?;
    h.click_start()?;
    h.click_start()?;
    let (role, count, messages) = toasts(&h.snap());
    assert_eq!((role, count), (Some(SemanticRole::Notifications), 0.0));
    assert!(messages.is_empty());

    h.tick_until(|s| state(s, "job-3").is_some_and(|t| t.starts_with("failed")))?;
    let s = h.snap();
    assert_eq!(state(&s, "job-3").as_deref(), Some("failed: I/O error at unit 20"));
    assert_eq!(percent(&s, "job-3").as_deref(), Some("20%"));
    assert!(close(bar(&s, "job-3"), 0.2));
    assert_eq!(bar_status(&s, "job-3").as_deref(), Some("failed"));
    assert!(!has_cancel(&s, "job-3"));
    assert_eq!(state(&s, "job-1").as_deref(), Some("running"));
    let (role, count, messages) = toasts(&s);
    assert_eq!((role, count), (Some(SemanticRole::Notifications), 1.0));
    assert_eq!(messages, ["Job 3 failed: I/O error at unit 20"]);
    // The notification lives outside the table.
    assert!(comp(&s, &["monitor", "toasts"]).is_some());
    assert_eq!(summary(&s).as_deref(), Some("Running: 2 · Done: 0 · Cancelled: 0 · Failed: 1"));
    assert!(thread_gone_within("job-3", WAIT).is_some());

    h.tick_until(|s| percent(s, "job-1").as_deref() == Some("20%"))?;
    h.tick_until(|s| state(s, "job-1").is_some_and(|t| t.starts_with("failed")))?;
    let s = h.snap();
    assert_eq!(state(&s, "job-1").as_deref(), Some("failed: I/O error at unit 30"));
    let (_, count, messages) = toasts(&s);
    assert_eq!(count, 2.0);
    assert!(messages.contains(&"Job 1 failed: I/O error at unit 30".to_owned()), "{messages:?}");
    assert!(messages.contains(&"Job 3 failed: I/O error at unit 20".to_owned()), "{messages:?}");
    assert_eq!(summary(&s).as_deref(), Some("Running: 1 · Done: 0 · Cancelled: 0 · Failed: 2"));
    // The other job is unaffected.
    h.tick_until(|s| state(s, "job-2").as_deref() == Some("done"))?;
    assert_eq!(summary(&h.snap()).as_deref(), Some("Running: 0 · Done: 1 · Cancelled: 0 · Failed: 2"));
    Ok(())
}

// ---------------------------------------------------------------------------
// Item 4: sorting by each column, read from drawn row order
// ---------------------------------------------------------------------------

/// Five jobs with distinct progress and elapsed values:
///
/// | job | final state | progress | elapsed |
/// |-----|-------------|----------|---------|
/// | 1   | running     | 6%       | 0.6 s   |
/// | 2   | done        | 100%     | 0.3 s   |
/// | 3   | cancelled   | 30%      | 0.2 s   |
/// | 4   | failed      | 20%      | 0.1 s   |
/// | 5   | failed      | 50%      | 0.5 s   |
fn five_jobs() -> R<H> {
    let mut h = H::new(
        vec![
            plan(500, &[5; 100], None),
            plan(50, &[20, 20, 10], None),
            plan(100, &[15; 7], None),
            plan(100, &[20; 5], Some(1)),
            plan(100, &[10; 10], Some(5)),
        ],
        800.0,
        360.0,
    )?;
    for _ in 0..5 {
        h.click_start()?;
    }
    h.tick_until(|s| state(s, "job-4").is_some_and(|t| t.starts_with("failed")) && percent(s, "job-1").as_deref() == Some("1%"))?;
    h.tick_until(|s| percent(s, "job-3").as_deref() == Some("30%") && percent(s, "job-1").as_deref() == Some("2%"))?;
    h.click_cancel("job-3")?;
    h.tick_until(|s| state(s, "job-2").as_deref() == Some("done") && percent(s, "job-1").as_deref() == Some("3%"))?;
    h.tick_until(|s| percent(s, "job-1").as_deref() == Some("4%"))?;
    h.tick_until(|s| state(s, "job-5").is_some_and(|t| t.starts_with("failed")) && percent(s, "job-1").as_deref() == Some("5%"))?;
    h.tick_until(|s| percent(s, "job-1").as_deref() == Some("6%"))?;
    let s = h.snap();
    assert_eq!(summary(&s).as_deref(), Some("Running: 1 · Done: 1 · Cancelled: 1 · Failed: 2"));
    let rows = |col: fn(&InspectionSnapshot, &str) -> Option<String>| -> Vec<Option<String>> {
        (1..=5).map(|i| col(&s, &format!("job-{i}"))).collect()
    };
    let o = |v: &[&str]| v.iter().map(|x| Some(x.to_string())).collect::<Vec<_>>();
    assert_eq!(rows(percent), o(&["6%", "100%", "30%", "20%", "50%"]));
    assert_eq!(rows(elapsed), o(&["0.6 s", "0.3 s", "0.2 s", "0.1 s", "0.5 s"]));
    Ok(h)
}

#[test]
fn item4_sorting_by_each_column_orders_drawn_rows() -> R {
    let mut h = five_jobs()?;
    assert_eq!(visual_rows(&h.snap()), ["Job 1", "Job 2", "Job 3", "Job 4", "Job 5"]);

    // Header items: 0 Name, 1 Progress, 2 State, 3 Elapsed; each sorted from
    // a different order so a no-op would fail.
    let cases: [(usize, [&str; 5]); 4] = [
        (1, ["Job 1", "Job 4", "Job 3", "Job 5", "Job 2"]), // 6 < 20 < 30 < 50 < 100 %
        (3, ["Job 4", "Job 3", "Job 2", "Job 5", "Job 1"]), // 0.1 < 0.2 < 0.3 < 0.5 < 0.6 s
        (0, ["Job 1", "Job 2", "Job 3", "Job 4", "Job 5"]),
        // running, done, cancelled, failed (the task's order); ties by job.
        (2, ["Job 1", "Job 2", "Job 3", "Job 4", "Job 5"]),
    ];
    for (title, ascending) in cases {
        // Start each column from a scrambled order (elapsed descending).
        h.click_title(3)?;
        if visual_rows(&h.snap())[0] != "Job 1" {
            h.click_title(3)?;
        }
        h.click_title(title)?;
        let s = h.snap();
        assert_eq!(visual_rows(&s), ascending, "column {title} ascending");
        h.click_title(title)?;
        let s = h.snap();
        let mut descending = ascending.map(str::to_owned).to_vec();
        descending.reverse();
        let drawn = visual_rows(&s);
        if title == 2 {
            // Descending state: failed (4, 5), cancelled, done, running.
            let states: Vec<String> = drawn
                .iter()
                .map(|n| state(&s, &format!("job-{}", &n[4..])).unwrap_or_default())
                .map(|t| t.split(':').next().unwrap_or_default().to_owned())
                .collect();
            assert_eq!(states, ["failed", "failed", "cancelled", "done", "running"], "{drawn:?}");
        } else {
            assert_eq!(drawn, descending, "column {title} descending");
        }
    }

    // The State column groups like states together in both directions.
    // Progress order holds while the running job keeps reporting.
    h.click_title(1)?;
    if visual_rows(&h.snap())[0] != "Job 1" {
        h.click_title(1)?;
    }
    h.tick_until(|s| percent(s, "job-1").as_deref() == Some("7%"))?;
    assert_eq!(visual_rows(&h.snap()), ["Job 1", "Job 4", "Job 3", "Job 5", "Job 2"]);
    Ok(())
}

#[test]
fn item4_name_sort_is_by_job_number_beyond_nine() -> R {
    let mut h = H::new((0..12).map(|_| plan(100, &[10; 10], None)).collect(), 800.0, 600.0)?;
    for _ in 0..12 {
        h.click_start()?;
    }
    let numeric: Vec<String> = (1..=12).map(|i| format!("Job {i}")).collect();
    // From elapsed (all equal) to name ascending, then descending.
    h.click_title(0)?;
    let first = visual_rows(&h.snap());
    let (asc, desc) = if first == numeric { (first, None) } else { (Vec::new(), Some(first)) };
    let (asc, desc) = match desc {
        None => {
            h.click_title(0)?;
            (asc, visual_rows(&h.snap()))
        }
        Some(desc) => {
            h.click_title(0)?;
            (visual_rows(&h.snap()), desc)
        }
    };
    assert_eq!(asc, numeric);
    let mut reversed = numeric.clone();
    reversed.reverse();
    assert_eq!(desc, reversed);
    Ok(())
}

// ---------------------------------------------------------------------------
// Item 5: CPU capture
// ---------------------------------------------------------------------------

#[test]
fn item5_cpu_capture_png_of_several_states() -> R {
    let mut h = five_jobs()?;
    h.click_title(1)?; // progress ascending, so the header shows a sort
    let mut d = Driver::new(&mut h.ui);
    d.runtime().set_capture_support(true);
    let pending = d
        .runtime()
        .request_capture(CaptureRequest::new(CaptureTarget::window(&d.runtime().primary_window())));
    d.settle()?;
    CpuRenderer::new().serve_captures(d.runtime_mut());
    let capture = pending.try_reply()?.value?;
    let image: &FrameImage = &capture.image;
    assert_eq!((image.width(), image.height()), (800, 360));
    let png = image.encode_png();
    let file = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("acceptance-capture.png");
    std::fs::write(&file, &png)?;
    assert!(std::fs::read(&file)?.starts_with(b"\x89PNG\r\n\x1a\n"));
    // The states shown in the captured frame, from the same inspection.
    let s = d.runtime().inspect();
    let shown: Vec<String> = (1..=5).filter_map(|i| state(&s, &format!("job-{i}"))).collect();
    for prefix in ["running", "done", "cancelled", "failed"] {
        assert!(shown.iter().any(|t| t.starts_with(prefix)), "{shown:?}");
    }
    println!("wrote {} ({} bytes)", file.display(), png.len());
    Ok(())
}

// ---------------------------------------------------------------------------
// Constraint: no repaint while no job is running
// ---------------------------------------------------------------------------

/// The library's idle check detects job activity (positive control) and
/// reports idle once no job runs, over real wall time with real threads.
#[test]
fn idle_check_detects_jobs_and_is_quiet_without_them() -> R {
    let env = Env {
        clock: Arc::new(SystemClock::new()),
        planner: Box::new(|_: u32| plan(100, &[10; 10], None)),
    };
    let mut app = app(env).headless()?;
    let (ui, monitor) = app.runtime_and_state_mut();
    ui.set_viewport(Viewport::new(800.0, 360.0, 1.0)?)?;
    let mut d = Driver::new(ui);
    d.settle()?;
    let before = d.quiet_for(Duration::from_secs(1))?;
    assert!(before.is_idle(), "{before:?}");

    d.click(&monitor.start)?;
    // Positive control: while the job runs, the library's flush reports over
    // ~500 ms of wall time see its updates and leave paint pending. (A
    // quiet_for baseline absorbs work delivered before the call, so the
    // control reads plain flush reports.)
    let start = Instant::now();
    let mut updated = 0;
    let mut paint_needed = 0;
    while start.elapsed() < Duration::from_millis(500) {
        std::thread::sleep(Duration::from_millis(20));
        let now = d.runtime().now();
        let r = d.runtime_mut().flush(now)?;
        updated += r.components_updated;
        paint_needed += usize::from(HostIntegration::needs_paint(d.runtime()));
        // Clear damage; a refusal for still-pending work just repeats next round.
        let _ = d.runtime_mut().paint();
    }
    println!("while running: {updated} components updated, paint needed {paint_needed} times");
    assert!(updated > 0 && paint_needed > 0, "no activity seen while a job ran");

    d.wait_until(WAIT, |rt| Ok(state(&rt.inspect(), "job-1").as_deref() == Some("done")))?;
    let _settled = d.quiet_for(Duration::from_millis(100))?;
    // No job running: over ~1 s of wall time nothing updates or needs paint.
    let start = Instant::now();
    let mut reports = 0;
    while start.elapsed() < Duration::from_secs(1) {
        std::thread::sleep(Duration::from_millis(20));
        let now = d.runtime().now();
        let f = d.runtime_mut().flush(now)?;
        assert_eq!(f.components_updated, 0, "an update with no job running: {f:?}");
        assert!(!HostIntegration::needs_paint(d.runtime()), "paint needed with no job running");
        let r = d.quiet_for(Duration::from_millis(100))?;
        assert!(r.is_idle(), "not idle after the job finished: {r:?}");
        reports += 1;
    }
    println!("after done: {reports} consecutive idle reports");
    app.shutdown()?;
    Ok(())
}
