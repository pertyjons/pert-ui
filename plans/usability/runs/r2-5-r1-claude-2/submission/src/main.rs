use std::time::{Duration, Instant};

use job_monitor::{Env, JobState, build};
use pert_ui::cpu::CpuRenderer;
use pert_ui::prelude::*;
use pert_ui::{CaptureRequest, CaptureTarget, KeyAction, KeyCode, Viewport};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    if let Some(out) = std::env::args().skip_while(|a| a != "--headless").nth(1) {
        return headless(&out);
    }
    App::new("Job monitor", |ui| {
        let handle = build(ui, Env::real())?;
        ui.keep_alive(handle);
        Ok(())
    })
    .run()?;
    Ok(())
}

fn press(ui: &mut Runtime, target: &WidgetId) -> Result<(), RuntimeError> {
    ui.focus(target)?;
    ui.key_input(KeyCode::Enter, KeyAction::Press)?;
    ui.key_input(KeyCode::Enter, KeyAction::Release)?;
    Ok(())
}

/// A real-time session without a display: starts six random jobs, cancels
/// one, waits for the rest and writes a CPU capture to `out`.
fn headless(out: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(900.0, 420.0, 1.0)?)?;
    let monitor = build(&mut ui, Env::real())?;
    let start = Instant::now();
    ui.flush(start.elapsed())?;
    let button = ui.find_descendant(&monitor.root, "start")?.ok_or("no start button")?;
    for _ in 0..6 {
        press(&mut ui, &button)?;
        ui.flush(start.elapsed())?;
    }
    let mut cancelled = false;
    let mut flushes = 0;
    while monitor.summary().running > 0 {
        // Sleep until the runtime has work, as a native host would.
        while ui.next_deadline().is_none() && monitor.summary().running > 0 {
            std::thread::sleep(Duration::from_millis(5));
        }
        ui.flush(start.elapsed())?;
        flushes += 1;
        if !cancelled && start.elapsed() > Duration::from_millis(800) {
            let table = ui.find_descendant(&monitor.root, "jobs")?.ok_or("no table")?;
            let running = monitor.rows().into_iter().find(|j| j.state == JobState::Running);
            if let Some(job) = running {
                let row = ui.find_descendant(&table, &format!("job-{}", job.id))?.ok_or("no row")?;
                let cancel = ui.find_descendant(&row, "cancel")?.ok_or("no cancel")?;
                press(&mut ui, &cancel)?;
                println!("cancel requested for {}", job.name);
            }
            cancelled = true;
        }
    }
    ui.flush(start.elapsed())?;
    println!("{} after {:.1} s and {flushes} flushes", monitor.summary_text()?, start.elapsed().as_secs_f64());
    for job in monitor.rows() {
        println!("  {:<6} {:>3}/{:<3} {:<9} {:.1} s", job.name, job.done, job.total, job.state.label(), job.elapsed.as_secs_f64());
    }
    println!("idle: next deadline {:?}", ui.next_deadline());

    ui.set_capture_support(true);
    let capture = ui.request_capture(CaptureRequest::new(CaptureTarget::window(&ui.primary_window())));
    ui.flush(start.elapsed())?;
    CpuRenderer::new().serve_captures(&mut ui);
    std::fs::write(out, capture.try_reply()?.value?.image.encode_png())?;
    println!("wrote {out}");
    Ok(())
}
