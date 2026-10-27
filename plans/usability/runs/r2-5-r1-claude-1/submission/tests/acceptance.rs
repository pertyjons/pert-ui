//! Headless acceptance checks. Plans come from a script instead of the random
//! source, and the jobs' clock only moves when a test calls `tick`.

use std::collections::VecDeque;
use std::time::Duration;

use job_monitor::headless::{Fallible, Session};
use job_monitor::monitor::JobState;
use job_monitor::sim::{JobPlan, Planner};
use pert_ui::{InspectValue, SortDirection, ToastStatus};

struct Script(VecDeque<JobPlan>);

impl Planner for Script {
    fn plan(&mut self, job_number: u32) -> JobPlan {
        self.0.pop_front().unwrap_or_else(|| panic!("no plan scripted for job {job_number}"))
    }
}

/// A job of `total` units, `step` units per tick, failing on tick `fail_at`.
fn plan(total: u32, step: u32, fail_at: Option<usize>) -> JobPlan {
    JobPlan { total, steps: vec![step; total.div_ceil(step) as usize], fail_at_tick: fail_at }
}

fn session(plans: impl IntoIterator<Item = JobPlan>) -> Fallible<Session> {
    Session::new(Box::new(Script(plans.into_iter().collect())))
}

fn summary(running: usize, done: usize, cancelled: usize, failed: usize) -> String {
    format!("Running: {running} · Done: {done} · Cancelled: {cancelled} · Failed: {failed}")
}

#[test]
fn jobs_progress_complete_and_update_the_summary() -> Fallible<()> {
    let mut s = session([plan(100, 10, None), plan(50, 5, None)])?;
    assert_eq!(s.summary()?, summary(0, 0, 0, 0));

    let first = s.start_job()?;
    let second = s.start_job()?;
    assert_eq!(s.cell_text(first, "name")?, "Job 1");
    assert_eq!(s.cell_text(second, "name")?, "Job 2");
    assert_eq!(s.cell_text(first, "state")?, "running");
    assert_eq!(s.cell_text(first, "progress")?, "0%");
    assert_eq!(s.summary()?, summary(2, 0, 0, 0));

    s.tick(3)?;
    assert_eq!(s.cell_text(first, "progress")?, "30%");
    assert_eq!(s.cell_text(second, "progress")?, "30%");
    assert_eq!(s.cell_text(first, "elapsed")?, "0.3 s");
    assert_eq!(s.summary()?, summary(2, 0, 0, 0));

    s.tick(7)?;
    for id in [first, second] {
        assert_eq!(s.job(id)?.state, JobState::Done);
        assert_eq!(s.cell_text(id, "state")?, "done");
        assert_eq!(s.cell_text(id, "progress")?, "100%");
        assert_eq!(s.cell_text(id, "elapsed")?, "1.0 s");
        // A finished job offers no Cancel action.
        assert!(s.ui.find(&s.monitor.root, &["content", "jobs", &format!("job-{id}"), "action", "cancel"])?.is_none());
    }
    assert_eq!(s.summary()?, summary(0, 2, 0, 0));
    Ok(())
}

#[test]
fn a_cancelled_job_stops_promptly() -> Fallible<()> {
    let mut s = session([plan(200, 10, None), plan(200, 10, None)])?;
    let cancelled = s.start_job()?;
    let other = s.start_job()?;
    s.tick(3)?;
    assert_eq!(s.cell_text(cancelled, "progress")?, "15%");

    s.click_cancel(cancelled)?;
    // The worker stops without waiting for another tick of its clock.
    s.wait_until("the job to report its cancellation", |m| {
        m.jobs()[0].state == JobState::Cancelled
    })?;
    assert_eq!(s.cell_text(cancelled, "state")?, "cancelled");
    assert_eq!(s.summary()?, summary(1, 0, 1, 0));

    // It stays where it stopped while the other job goes on.
    s.tick(5)?;
    let job = s.job(cancelled)?;
    assert_eq!((job.done, job.ticks), (30, 3));
    assert_eq!(s.cell_text(cancelled, "progress")?, "15%");
    assert_eq!(s.cell_text(cancelled, "elapsed")?, "0.3 s");
    assert_eq!(s.cell_text(other, "progress")?, "40%");
    assert!(s.ui.find(&s.monitor.root, &["content", "jobs", "job-1", "action", "cancel"])?.is_none());
    assert!(s.ui.find(&s.monitor.root, &["content", "jobs", "job-2", "action", "cancel"])?.is_some());
    Ok(())
}

#[test]
fn a_failure_shows_in_its_row_and_as_a_notification() -> Fallible<()> {
    let mut s = session([plan(100, 10, Some(4)), plan(100, 10, None)])?;
    let failing = s.start_job()?;
    let healthy = s.start_job()?;
    s.tick(4)?;
    assert_eq!(s.cell_text(failing, "state")?, "running");

    s.tick(1)?;
    assert_eq!(s.job(failing)?.state, JobState::Failed);
    assert_eq!(s.cell_text(failing, "state")?, "failed: checksum error at unit 40");
    assert_eq!(s.cell_text(failing, "progress")?, "40%");
    assert_eq!(s.cell_text(healthy, "state")?, "running");
    assert_eq!(s.summary()?, summary(1, 0, 0, 1));

    // A notification names the failure and is on screen.
    let notifications = s.monitor.notifications();
    assert_eq!(notifications.len(), 1);
    assert_eq!(notifications[0].job, failing);
    assert_eq!(notifications[0].message, "Job 1 failed: checksum error at unit 40");
    assert_eq!(notifications[0].receipt.status(), ToastStatus::Visible);
    let toasts = s.widget(&["toasts"])?;
    let id = s.ui.inspection_id(&toasts)?;
    let snapshot = s.ui.inspect();
    let host = snapshot.components.iter().find(|c| c.id == id).ok_or("toast host not inspected")?;
    assert_eq!(host.details.properties.get("visible_count"), Some(&InspectValue::Number(1.0)));
    // It expires after five seconds on the runtime clock; the row keeps the failure.
    s.advance_ui(Duration::from_secs(5))?;
    assert_eq!(notifications[0].receipt.status(), ToastStatus::Expired);
    assert_eq!(s.cell_text(failing, "state")?, "failed: checksum error at unit 40");
    Ok(())
}

/// Five jobs in every state, with distinct progress and elapsed times:
///
/// | job | state     | progress | elapsed |
/// |-----|-----------|----------|---------|
/// | 1   | done      | 100%     | 0.4 s   |
/// | 2   | running   | 6%       | 0.6 s   |
/// | 3   | failed    | 20%      | 0.3 s   |
/// | 4   | cancelled | 15%      | 0.3 s   |
/// | 5   | running   | 13%      | 0.2 s   |
pub fn mixed_session() -> Fallible<Session> {
    let mut s = session([
        plan(100, 25, None),
        plan(500, 5, None),
        plan(100, 10, Some(2)),
        plan(200, 10, None),
        plan(300, 20, None),
    ])?;
    s.start_job()?;
    s.start_job()?;
    s.tick(1)?;
    s.start_job()?;
    s.start_job()?;
    s.tick(3)?;
    s.click_cancel(4)?;
    s.wait_until("job 4 to stop", |m| m.jobs()[3].state == JobState::Cancelled)?;
    s.start_job()?;
    s.tick(2)?;
    let states: Vec<_> = s.monitor.jobs().iter().map(|job| (job.state, job.percent(), job.ticks)).collect();
    assert_eq!(
        states,
        [
            (JobState::Done, 100, 4),
            (JobState::Running, 6, 6),
            (JobState::Failed, 20, 3),
            (JobState::Cancelled, 15, 3),
            (JobState::Running, 13, 2),
        ]
    );
    Ok(s)
}

#[test]
fn every_column_sorts_both_ways() -> Fallible<()> {
    let mut s = mixed_session()?;
    assert_eq!(s.row_order()?, [1, 2, 3, 4, 5], "start order before any sort");
    let expected: [(&str, [u32; 5], [u32; 5]); 4] = [
        ("name", [1, 2, 3, 4, 5], [5, 4, 3, 2, 1]),
        ("progress", [2, 5, 4, 3, 1], [1, 3, 4, 5, 2]),
        // States sort by their names; equal keys keep start order.
        ("state", [4, 1, 3, 2, 5], [2, 5, 3, 1, 4]),
        ("elapsed", [5, 3, 4, 1, 2], [2, 1, 3, 4, 5]),
    ];
    for (column, ascending, descending) in expected {
        s.click_header(column)?;
        let sort = s.monitor.table.sort().get()?.ok_or("no sort indication")?;
        assert_eq!((sort.column.as_str(), sort.direction), (column, SortDirection::Ascending));
        assert_eq!(s.row_order()?, ascending, "{column} ascending");

        s.click_header(column)?;
        let sort = s.monitor.table.sort().get()?.ok_or("no sort indication")?;
        assert_eq!((sort.column.as_str(), sort.direction), (column, SortDirection::Descending));
        assert_eq!(s.row_order()?, descending, "{column} descending");
    }

    // The order holds while running jobs keep reporting.
    s.click_header("elapsed")?;
    s.tick(1)?;
    assert_eq!(s.row_order()?, [3, 4, 5, 1, 2], "elapsed ascending after a tick");
    Ok(())
}

#[test]
fn nothing_is_scheduled_while_no_job_runs() -> Fallible<()> {
    let mut s = session([plan(20, 10, None), plan(100, 10, Some(0))])?;
    s.ui.paint()?;
    assert_eq!(s.ui.next_deadline(), None, "idle before any job");
    assert!(!s.ui.needs_paint());

    s.start_job()?;
    s.start_job()?;
    s.tick(2)?;
    assert_eq!(s.monitor.counts().running, 0);
    // The failure's toast expires on the runtime clock; then nothing is due.
    s.advance_ui(Duration::from_secs(6))?;
    s.ui.paint()?;
    assert_eq!(s.ui.next_deadline(), None, "idle after the jobs ended");
    assert!(!s.ui.needs_paint(), "no repaint pending");
    let report = s.ui.flush(Duration::from_secs(60))?;
    assert_eq!(report.components_updated, 0);
    Ok(())
}

#[test]
fn capture_of_jobs_in_several_states() -> Fallible<()> {
    let mut s = mixed_session()?;
    let png = s.capture_png()?;
    assert!(png.starts_with(b"\x89PNG"));
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("captures");
    std::fs::create_dir_all(&dir)?;
    std::fs::write(dir.join("jobs.png"), png)?;
    Ok(())
}
