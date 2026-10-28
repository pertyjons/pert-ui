//! Independent acceptance checks added by the verifier (not by the tester).
//!
//! They drive the application only through pert-ui's own facilities: the
//! testing `Driver`'s pointer input (`click`, `click_item`), inspection
//! (`find`, `address_children`, `semantics`), `quiet_for` idle reports,
//! runtime profiling and the CPU renderer's capture service. The model
//! accessor `Monitor::jobs()` is deliberately not used. The only application
//! entry point used is the public `mount(ui, plans)`, whose plans replace the
//! random source as the task permits; one test uses no plans (production
//! random path).
//!
//! The thread-count check reads `/proc/self/task`, so run this file with
//! `--test-threads=1`.

use job_monitor::{Monitor, Plan, mount};
use pert_ui::ProfileOperation;
use pert_ui::{prelude::*, testing::*};
use std::time::{Duration, Instant};

type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;

const CAPTURE_DIR: &str = "/work/acceptance-captures";

fn plan(total: u32, step: u32, fail_at: Option<u32>) -> Plan {
    Plan { total, step: Some(step), fail_at }
}

/// The returned Monitor holds the mailbox; callers keep it alive.
fn setup(plans: Vec<Plan>) -> Result<(Runtime, Monitor)> {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(1000.0, 520.0, 1.0)?)?;
    let monitor = mount(&mut ui, plans)?;
    Ok((ui, monitor))
}

fn name(d: &Driver<'_>, from: &WidgetId, path: &[&str]) -> Result<String> {
    Ok(d.semantics(&d.find(from, path)?)?.name.unwrap_or_default())
}

fn table(d: &Driver<'_>, root: &WidgetId) -> Result<WidgetId> {
    Ok(d.find(root, &["jobs"])?)
}

/// Displayed cells of one row, read from inspection.
#[derive(Debug, Clone, PartialEq)]
struct RowView {
    key: String,
    name: String,
    percent: u32,
    state: String,
    elapsed: f64,
    has_cancel: bool,
}

fn rows(d: &Driver<'_>, root: &WidgetId) -> Result<Vec<RowView>> {
    let t = table(d, root)?;
    let mut out = vec![];
    for child in d.address_children(&t)? {
        let key = child.segment;
        let percent = name(d, &t, &[&key, "progress", "value", "percent"])?;
        let elapsed = name(d, &t, &[&key, "elapsed", "value"])?;
        out.push(RowView {
            name: name(d, &t, &[&key, "name", "value"])?,
            percent: percent.trim_end_matches('%').parse()?,
            state: name(d, &t, &[&key, "state", "value"])?,
            elapsed: elapsed.trim_end_matches(" s").parse()?,
            has_cancel: d.find(&t, &[&key, "action", "cancel"]).is_ok(),
            key,
        });
    }
    Ok(out)
}

fn row(d: &Driver<'_>, root: &WidgetId, key: &str) -> Result<RowView> {
    rows(d, root)?
        .into_iter()
        .find(|r| r.key == key)
        .ok_or_else(|| format!("no row {key}").into())
}

/// Polls in wall time, settling the runtime so delivered mailbox messages
/// apply, until `cond` holds on inspected state.
fn poll(
    d: &mut Driver<'_>,
    timeout: Duration,
    mut cond: impl FnMut(&Driver<'_>) -> Result<bool>,
) -> Result {
    let start = Instant::now();
    loop {
        d.settle()?;
        if cond(d)? {
            return Ok(());
        }
        if start.elapsed() > timeout {
            return Err(format!("condition not met within {timeout:?}").into());
        }
        std::thread::sleep(Duration::from_millis(2));
    }
}

fn threads() -> usize {
    std::fs::read_dir("/proc/self/task").map(|d| d.count()).unwrap_or(0)
}

fn summary(d: &Driver<'_>, root: &WidgetId) -> Result<String> {
    name(d, root, &["summary"])
}

/// Item 1: starting jobs through the Start button, seeing intermediate
/// progress, completion and the summary line update, all from inspection.
/// Also checks the tick rate (10 ticks/s) from the displayed elapsed time.
#[test]
fn item1_progress_completion_summary() -> Result {
    // 100 units at 10 per tick => 10 ticks => about 1.0 s at 10 ticks/s.
    let (mut ui, monitor) = setup(vec![plan(100, 10, None), plan(500, 5, None)])?;
    let root = &monitor.root;
    let mut d = Driver::new(&mut ui);
    d.settle()?;
    assert_eq!(summary(&d, &root)?, "Running: 0   Done: 0   Cancelled: 0   Failed: 0");
    assert!(rows(&d, &root)?.is_empty());
    let start = d.find(&root, &["start"])?;
    d.click(&start)?;
    assert_eq!(summary(&d, &root)?, "Running: 1   Done: 0   Cancelled: 0   Failed: 0");
    d.advance(Duration::from_millis(400))?; // two single clicks, not a double click
    d.click(&start)?;
    assert_eq!(summary(&d, &root)?, "Running: 2   Done: 0   Cancelled: 0   Failed: 0");
    let r = rows(&d, &root)?;
    assert_eq!(r.iter().map(|r| r.name.as_str()).collect::<Vec<_>>(), ["Job 1", "Job 2"]);
    assert!(r.iter().all(|r| r.state == "running" && r.percent == 0 && r.has_cancel));

    // Intermediate progress is visible (strictly between 0 and 100 %).
    poll(&mut d, Duration::from_secs(2), |d| {
        let p = row(d, &root, "1")?.percent;
        Ok(p > 0 && p < 100)
    })?;
    let mid = row(&d, &root, "1")?;
    eprintln!("intermediate row 1: {mid:?}");
    assert_eq!(mid.state, "running");

    // Completion.
    poll(&mut d, Duration::from_secs(3), |d| Ok(row(d, &root, "1")?.state == "done"))?;
    let done = row(&d, &root, "1")?;
    eprintln!("completed row 1: {done:?}");
    assert_eq!(done.percent, 100);
    assert!(!done.has_cancel, "a finished job keeps no Cancel action");
    assert!(
        (0.85..=1.6).contains(&done.elapsed),
        "10 ticks should take about 1 s at 10 ticks/s, displayed {} s",
        done.elapsed
    );
    assert_eq!(summary(&d, &root)?, "Running: 1   Done: 1   Cancelled: 0   Failed: 0");
    let job2 = row(&d, &root, "2")?;
    assert!(job2.percent > 0 && job2.state == "running");
    // Elapsed of the running job advances while it runs.
    let e1 = job2.elapsed;
    poll(&mut d, Duration::from_secs(2), |d| Ok(row(d, &root, "2")?.elapsed > e1))?;

    // Clean up.
    let t = table(&d, &root)?;
    let cancel = d.find(&t, &["2", "action", "cancel"])?;
    d.click(&cancel)?;
    poll(&mut d, Duration::from_secs(1), |d| Ok(row(d, &root, "2")?.state == "cancelled"))?;
    assert_eq!(summary(&d, &root)?, "Running: 0   Done: 1   Cancelled: 1   Failed: 0");
    Ok(())
}

/// Item 1 with the production random source (no plans): a job completes
/// with total 50..=500 at 5..=20 units per tick, so within 0.25 to 10 s.
#[test]
fn item1_production_random_job_completes() -> Result {
    let (mut ui, monitor) = setup(vec![])?;
    let root = &monitor.root;
    let mut d = Driver::new(&mut ui);
    d.settle()?;
    let start = d.find(&root, &["start"])?;
    d.click(&start)?;
    poll(&mut d, Duration::from_secs(12), |d| Ok(row(d, &root, "1")?.state != "running"))?;
    let r = row(&d, &root, "1")?;
    eprintln!("random job 1: {r:?}");
    assert_eq!((r.state.as_str(), r.percent), ("done", 100));
    // ceil(50/20) = 3 ticks minimum, ceil(500/5) = 100 ticks maximum.
    assert!((0.25..=11.0).contains(&r.elapsed), "elapsed {}", r.elapsed);
    assert_eq!(summary(&d, &root)?, "Running: 0   Done: 1   Cancelled: 0   Failed: 0");
    Ok(())
}

/// Item 2: cancelling a running job through its Cancel button. The row
/// shows "cancelled" promptly, its displayed progress stays fixed, the
/// Cancel action disappears, and the worker thread exits (OS thread count).
#[test]
fn item2_cancel_stops_job_and_worker() -> Result {
    let (mut ui, monitor) = setup(vec![plan(500, 5, None)])?;
    let root = &monitor.root;
    let mut d = Driver::new(&mut ui);
    d.settle()?;
    let before = threads();
    let start = d.find(&root, &["start"])?;
    d.click(&start)?;
    poll(&mut d, Duration::from_secs(1), |_| Ok(threads() == before + 1))?;
    poll(&mut d, Duration::from_secs(2), |d| Ok(row(d, &root, "1")?.percent >= 2))?;
    let t = table(&d, &root)?;
    let cancel = d.find(&t, &["1", "action", "cancel"])?;
    let clicked = Instant::now();
    d.click(&cancel)?;
    poll(&mut d, Duration::from_millis(300), |d| Ok(row(d, &root, "1")?.state == "cancelled"))?;
    let latency = clicked.elapsed();
    let stopped = row(&d, &root, "1")?;
    eprintln!("cancelled after {latency:?}: {stopped:?}");
    assert!(latency < Duration::from_millis(300));
    assert!(stopped.percent < 100);
    assert!(!stopped.has_cancel);
    assert_eq!(summary(&d, &root)?, "Running: 0   Done: 0   Cancelled: 1   Failed: 0");
    // The worker thread exits.
    poll(&mut d, Duration::from_millis(300), |_| Ok(threads() == before))?;
    eprintln!("threads before start {before}, after cancel {}", threads());
    // Nothing changes afterwards, in wall time.
    std::thread::sleep(Duration::from_millis(600));
    d.settle()?;
    assert_eq!(row(&d, &root, "1")?, stopped);
    let quiet = d.quiet_for(Duration::from_secs(1))?;
    assert!(quiet.is_idle(), "{quiet:?}");
    Ok(())
}

/// Item 3: a failure is shown in its row and as a notification outside
/// the row (the "notification" label, a direct child of the monitor, not
/// of the table).
#[test]
fn item3_failure_in_row_and_notification() -> Result {
    let (mut ui, monitor) = setup(vec![plan(500, 5, None), plan(100, 10, Some(45))])?;
    let root = &monitor.root;
    let mut d = Driver::new(&mut ui);
    d.settle()?;
    assert_eq!(name(&d, &root, &["notification"])?, "");
    let start = d.find(&root, &["start"])?;
    d.click(&start)?;
    d.advance(Duration::from_millis(400))?;
    d.click(&start)?;
    poll(&mut d, Duration::from_secs(3), |d| Ok(row(d, &root, "2")?.state != "running"))?;
    let failed = row(&d, &root, "2")?;
    eprintln!("failed row: {failed:?}");
    assert_eq!(failed.state, "failed: simulated error");
    assert_eq!(failed.percent, 45, "stops at its failure point");
    assert!(!failed.has_cancel);
    let notice = name(&d, &root, &["notification"])?;
    eprintln!("notification: {notice:?}");
    assert!(notice.contains("Job 2 failed"), "{notice}");
    let top: Vec<String> = d.address_children(&root)?.into_iter().map(|c| c.segment).collect();
    eprintln!("monitor children: {top:?}");
    assert!(top.contains(&"notification".to_string()) && top.contains(&"jobs".to_string()));
    assert_eq!(summary(&d, &root)?, "Running: 1   Done: 0   Cancelled: 0   Failed: 1");
    // The other job keeps running; the notification persists.
    std::thread::sleep(Duration::from_millis(300));
    d.settle()?;
    assert_eq!(row(&d, &root, "1")?.state, "running");
    assert_eq!(name(&d, &root, &["notification"])?, notice);
    let t = table(&d, &root)?;
    let cancel = d.find(&t, &["1", "action", "cancel"])?;
    d.click(&cancel)?;
    poll(&mut d, Duration::from_secs(1), |d| Ok(row(d, &root, "1")?.state == "cancelled"))?;
    Ok(())
}

/// Item 3 with the production random source: every fifth job fails at a
/// random point (the app's interpretation of "one job in five").
#[test]
fn item3_production_fifth_job_fails() -> Result {
    let (mut ui, monitor) = setup(vec![])?;
    let root = &monitor.root;
    let mut d = Driver::new(&mut ui);
    d.settle()?;
    let start = d.find(&root, &["start"])?;
    for _ in 0..5 {
        d.click(&start)?;
        d.advance(Duration::from_millis(400))?;
    }
    poll(&mut d, Duration::from_secs(12), |d| {
        Ok(rows(d, &root)?.iter().all(|r| r.state != "running"))
    })?;
    let r = rows(&d, &root)?;
    eprintln!("random jobs: {r:#?}");
    for job in &r[..4] {
        assert_eq!((job.state.as_str(), job.percent), ("done", 100), "{job:?}");
    }
    assert_eq!(r[4].state, "failed: simulated error");
    assert!(r[4].percent < 100);
    assert!(name(&d, &root, &["notification"])?.contains("Job 5 failed"));
    assert_eq!(summary(&d, &root)?, "Running: 0   Done: 4   Cancelled: 0   Failed: 1");
    Ok(())
}

fn ordered<T: PartialOrd + std::fmt::Debug>(values: &[T], descending: bool) -> bool {
    values.windows(2).all(|w| if descending { w[0] >= w[1] } else { w[0] <= w[1] })
}

/// Items 4 and 5 and idleness: eleven jobs in all four states, sorted by
/// each column through header clicks in both directions, checked against
/// the displayed cell values (not the app's comparator); then a CPU capture
/// and idle checks with a positive control and profiling.
#[test]
fn item4_sorting_item5_capture_and_idle() -> Result {
    std::fs::create_dir_all(CAPTURE_DIR)?;
    let plans = vec![
        plan(50, 25, None),        // 1 done fast
        plan(500, 5, None),        // 2 cancelled later
        plan(100, 20, Some(60)),   // 3 failed at 60 %
        plan(500, 5, None),        // 4 stays running
        plan(200, 20, None),       // 5 done after 1 s
        plan(400, 10, Some(40)),   // 6 failed at 10 %
        plan(50, 10, None),        // 7..11 done
        plan(60, 20, None),
        plan(80, 20, None),
        plan(50, 5, None),
        plan(90, 15, None),
    ];
    let (mut ui, monitor) = setup(plans)?;
    ui.set_profiling(true)?;
    let root = &monitor.root;
    let mut d = Driver::new(&mut ui);
    d.settle()?;
    // Idle before any job, and idle according to profiling: no Paint.
    let q = d.quiet_for(Duration::from_secs(1))?;
    assert!(q.is_idle(), "{q:?}");
    let start = d.find(&root, &["start"])?;
    for i in 0..11 {
        d.click(&start)?;
        d.advance(Duration::from_millis(400))?;
        if i == 1 || i == 4 {
            std::thread::sleep(Duration::from_millis(120));
        }
    }
    // Positive control: while jobs run, the window is not idle.
    std::thread::sleep(Duration::from_millis(250));
    let busy = d.quiet_for(Duration::from_millis(100))?;
    eprintln!("while running: {busy:?}");
    assert!(!busy.is_idle(), "running jobs must update the UI");

    poll(&mut d, Duration::from_secs(3), |d| Ok(row(d, &root, "2")?.percent >= 3))?;
    let t = table(&d, &root)?;
    let cancel = d.find(&t, &["2", "action", "cancel"])?;
    d.click(&cancel)?;
    poll(&mut d, Duration::from_secs(4), |d| {
        let r = rows(d, &root)?;
        Ok(r.iter().filter(|r| r.state == "running").count() == 1
            && row(d, &root, "4")?.state == "running")
    })?;
    let states: std::collections::BTreeSet<String> =
        rows(&d, &root)?.into_iter().map(|r| r.state).collect();
    assert_eq!(
        states.into_iter().collect::<Vec<_>>(),
        ["cancelled", "done", "failed: simulated error", "running"]
    );
    let headers: Vec<Option<String>> =
        d.semantics(&t)?.items.into_iter().map(|i| i.name).collect();
    eprintln!("header items: {headers:?}");

    for (index, column) in ["name", "progress", "state", "elapsed"].into_iter().enumerate() {
        for descending in [false, true] {
            d.click_item(&t, index)?;
            d.advance(Duration::from_millis(400))?;
            let r = rows(&d, &root)?;
            let keys: Vec<&str> = r.iter().map(|r| r.key.as_str()).collect();
            eprintln!("sort {column} descending={descending}: {keys:?}");
            let ok = match column {
                "name" => ordered(&r.iter().map(|r| r.name.clone()).collect::<Vec<_>>(), descending),
                "progress" => ordered(&r.iter().map(|r| r.percent).collect::<Vec<_>>(), descending),
                "state" => ordered(&r.iter().map(|r| r.state.clone()).collect::<Vec<_>>(), descending),
                "elapsed" => ordered(&r.iter().map(|r| r.elapsed).collect::<Vec<_>>(), descending),
                _ => unreachable!(),
            };
            assert!(ok, "{column} descending={descending} not ordered: {r:#?}");
            assert_eq!(r.len(), 11);
            if column == "name" {
                let names: Vec<&str> = r.iter().map(|r| r.name.as_str()).collect();
                eprintln!("name order: {names:?}");
            }
        }
    }

    // A new update while sorted keeps the order (the running job ticks).
    std::thread::sleep(Duration::from_millis(250));
    d.settle()?;
    let r = rows(&d, &root)?;
    assert!(ordered(&r.iter().map(|r| r.elapsed).collect::<Vec<_>>(), true), "{r:#?}");

    // Item 5: our own CPU capture with all four states on screen.
    d.runtime().set_capture_support(true);
    let pending = d
        .runtime()
        .request_capture(CaptureRequest::new(CaptureTarget::window(&d.runtime().primary_window())));
    d.settle()?;
    let shown = rows(&d, &root)?;
    CpuRenderer::new().serve_captures(d.runtime_mut());
    let capture = pending.try_reply()?.value?;
    let path = format!("{CAPTURE_DIR}/acceptance-states.png");
    std::fs::write(&path, capture.image.encode_png())?;
    eprintln!("capture {}x{} at {path}; rows at capture: {shown:#?}", capture.image.width(), capture.image.height());
    assert_eq!((capture.image.width(), capture.image.height()), (1000, 520));

    // Cancel the last running job; afterwards the window is idle.
    let t = table(&d, &root)?;
    let cancel = d.find(&t, &["4", "action", "cancel"])?;
    d.click(&cancel)?;
    poll(&mut d, Duration::from_secs(1), |d| Ok(row(d, &root, "4")?.state == "cancelled"))?;
    assert_eq!(summary(&d, &root)?, "Running: 0   Done: 7   Cancelled: 2   Failed: 2");
    d.clear_focus()?;
    std::thread::sleep(Duration::from_millis(400)); // wall time for any straggler message
    let q = d.quiet_for(Duration::from_secs(2))?;
    eprintln!("idle after all jobs ended: {q:?}");
    assert!(q.is_idle(), "{q:?}");
    assert!(!q.needs_paint && q.next_deadline.is_none() && q.components_updated == 0);
    // Profiling over a further idle stretch: no update, layout or paint work.
    d.runtime().clear_profile()?;
    std::thread::sleep(Duration::from_millis(300));
    d.advance(Duration::from_secs(2))?;
    let profile = d.runtime().profile();
    let work: Vec<_> = profile
        .stats
        .iter()
        .filter(|s| {
            matches!(
                s.operation,
                ProfileOperation::Update | ProfileOperation::Layout | ProfileOperation::Paint | ProfileOperation::Compose
            )
        })
        .collect();
    eprintln!("idle profile: {:?}", profile.stats);
    assert!(work.is_empty(), "idle work recorded: {work:?}");
    Ok(())
}
