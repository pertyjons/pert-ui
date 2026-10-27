//! The job monitor. `job_monitor` opens a window (`--software` presents it with
//! the CPU renderer); `job_monitor --headless [out.png]` runs a scripted
//! session with no display and writes a CPU capture.

use std::sync::Arc;

use job_monitor::headless::{Fallible, Session};
use job_monitor::monitor::{self, JobState};
use job_monitor::sim::{RandomPlanner, RealClock, SplitMix64};
use pert_ui::prelude::*;

fn main() -> Fallible<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().map(String::as_str) == Some("--headless") {
        let out = args.get(1).cloned().unwrap_or_else(|| "jobs.png".to_owned());
        return headless(&out);
    }
    let software = args.iter().any(|arg| arg == "--software");
    let app = App::new("Job monitor", |ui| {
        let monitor = monitor::build(
            ui,
            Arc::new(RealClock::default()),
            Box::new(RandomPlanner(SplitMix64::from_time())),
        )?;
        // The monitor owns the mailbox the workers report to.
        ui.keep_alive(monitor);
        Ok(())
    })
    .size(Size::new(820.0, 520.0)?)?;
    let app = if software { app.software_rendering() } else { app };
    app.run()?;
    Ok(())
}

/// Starts eight random jobs, cancels one, runs the clock and writes a capture.
fn headless(out: &str) -> Fallible<()> {
    let mut session = Session::new(Box::new(RandomPlanner(SplitMix64(2026))))?;
    for _ in 0..4 {
        session.start_job()?;
    }
    session.tick(5)?;
    for _ in 0..4 {
        session.start_job()?;
    }
    session.tick(3)?;
    if let Some(job) = session.monitor.jobs().iter().find(|job| job.state == JobState::Running) {
        session.click_cancel(job.id)?;
        let id = job.id;
        session.wait_until("the cancellation", |m| {
            m.jobs().iter().any(|job| job.id == id && job.state == JobState::Cancelled)
        })?;
    }
    while !session.monitor.jobs().iter().any(|job| job.state == JobState::Done) {
        session.tick(1)?;
    }
    for job in session.monitor.jobs() {
        println!("{:<7} {:>4}% {:<40} {}", job.name, job.percent(), job.state_text(), job.elapsed_text());
    }
    println!("{}", session.summary()?);
    std::fs::write(out, session.capture_png()?)?;
    println!("wrote {out}");
    Ok(())
}
