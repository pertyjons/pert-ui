# Acceptance: r2-5-r1-codex-2 (task 5, background job monitor)

Author check, 2026-10-27, kit `kit-359122e-r2` (0.3.52). The submitted
`work/` (without `target/`, `.claude/` and `.git/`) was copied to
`~/pert-ui-usability/acceptance-r2-5-r1-codex-2`, built with
`cargo build --all-targets` and tested with `cargo test` in the kit sandbox
(offline): builds without warnings from a clean target, and the tester's one
check (`src/main.rs` `headless()`, run by `tests::acceptance`) passes. It
rewrote `jobs.png` byte-identically to the submitted one. The tester's check
runs the real workers and mailbox with fixed plans and test-driven ticks over
each worker's command channel, which the task allows. The author added one
check (source below), appended as a test module to the acceptance copy of
`src/main.rs`, that runs the production configuration (`mount(ui)` with no
plans: random sizes and steps, real 100 ms ticks, real clock) through keyboard
input and inspection only; it passes (about 3.1 s).

| Item | Result | Code | Check |
|---|---|---|---|
| 1. Start jobs, see progress, completion and the summary update | met | `Monitor::start` `src/main.rs:299-328` (worker thread `:325` with a `UiSender`), worker `:77-151`, mailbox handler `Store::report` `:206-227` via `ui.mailbox` `:364`, summary in `publish` `:169-193` | tester `:470-500`, `:519-525` (Space on Start five times, summary "Running: 5", row text "40% (20/50)" from inspection, job 1 completes, summary "Running: 2 Done: 1 Cancelled: 1 Failed: 1" from inspection); author check: production path, nonzero row percent, jobs 1, 3, 4 read "done"/"100%" from inspection, final summary "Running: 0 Done: 3 Cancelled: 1 Failed: 1" |
| 2. Cancel a running job and see it stop | met | Cancel button calls `Store::cancel` `:265-275`, which sends `Command::Cancel` and marks the row cancelled `:194-205`; the worker reports and returns `:100`, `:125-149` | tester `:501-518` (Space on Job 2's Cancel; the worker thread's `JoinHandle::is_finished` within 250 ms; state and units from the app's model, summary "Cancelled: 1" from inspection `:525`); author check: Cancel by key input on the production path, row "cancelled" from inspection about 3 ms later, row percent and elapsed text unchanged while the other jobs ran to completion |
| 3. Failure in its row and as a notification | met (maintainer decision) | every fifth job fails `:306-310`, `:103-110`; row label "failed: simulated error" `:258-263`; the "notification" is a plain `Label` bound to a text signal `:387`, set in `Store::report` `:220-225` | tester `:493-499` (row text and notification text from inspection); author check: production job 5 fails at a random point, row reads "failed: simulated error", the `notification` widget is role `Label` named "Notification: Job 5 failed — simulated processing error". The failure is not shown in a library notification surface (`Banner` or `Toast` from the library's notifications module); a permanent status label reading "Notifications: no failures" until a failure is a status line, not a notification (the README calls it a "persistent notification banner") |
| 4. Sorting by each column orders the rows correctly | met | `compare` `:158-167` (name by text, progress by fraction, state by text, elapsed by duration, ties by id), applied in every `publish` and on `Table::on_sort` `:384` | tester `:526-572`: Alt+ArrowUp/ArrowDown key input on the focused table for all four columns, both directions, sort state checked, mounted row keys compared with fixed expected orders (rows in four states) |
| 5. CPU capture PNG with jobs in several states | met | `:573-580` (`request_capture` of the `monitor` path, `CpuRenderer::serve_captures`, `encode_png`) | `work/jobs.png` (1000×520), sorted by elapsed descending: Job 1 done, Job 5 failed, Jobs 4 and 3 running with Cancel, Job 2 cancelled, notification text at the bottom; reproduced byte-identically by the author |

## Task-specific checks

- **Progress through the library's cross-thread facility:** met. The worker
  owns its command receiver, plan and a `UiSender<Report>` (`:77`, spawn
  `:325`); rows, signals and the table are updated in the `ui.mailbox`
  handler on the UI thread (`:364`, `:206-227`). A full mailbox is retried
  while still honoring Cancel and Stop (`:123-146`).
- **Cancellation stops the worker:** met. The worker reports Cancelled and
  returns (`:100`, `:125-149`); the tester's check observes the thread
  finish within 250 ms; `Drop for Monitor` closes the mailbox, stops and
  joins every worker. Author check as in item 2.
- **Failure in the row and as a notification:** met by the maintainer's
  decision (item 3): row yes, notification an ordinary label.
- **Sorting by every column:** met (item 4).
- **No repaint while idle:** met. Tester: after every worker joined,
  `paint()`, `next_deadline() == None`, `needs_paint()` false and
  `components_updated == 0` at 1 s and 60 s (`:587-596`). Author check: the
  same at +1 s, +10 s and +60 s of runtime time on the production path.

## Triage notes

Not acceptance failures.

- Item 3 is a judgement on what counts as a notification: the library's
  notifications module provides `Banner` (persistent, role Alert) and
  `Toast`/`ToastHost`; this run shows the failure in a plain label. If the
  author accepts any visible failure text outside the row, item 3 and the run
  become met (session ended 2026-10-27T18:56:48+01:00).
- Name sorting compares the text "Job N" (`:160`), so with ten or more jobs
  "Job 10" sorts between "Job 1" and "Job 2". The task does not define the
  name order and the checks use five jobs; noted, not failed.
- Each cell build makes a fresh `ProgressStatus` signal with
  `Signal::sibling` on the summary signal (`:246`); the documented per-row
  binding form.

## Author check source

`author_common.rs` (identical for every task-5 run judged in this batch), copied to `src/author_common.rs`:

```rust
// Author acceptance check for task 5 (background job monitor), identical for
// every run; only the wrapper (how the app is built, its widget keys) differs.
// Production configuration: real random sizes and steps, real 100 ms ticks,
// real clock. Drives the UI through keyboard input on the real buttons and
// reads results from inspection only.
use pert_ui::{KeyAction, KeyCode, Runtime, WidgetId};
use std::time::{Duration, Instant};

type R<T = ()> = Result<T, Box<dyn std::error::Error>>;

pub struct Keys {
    pub table: &'static str,
    pub row: fn(u32) -> String,
}

fn activate(ui: &mut Runtime, id: &WidgetId) -> R {
    ui.focus(id)?;
    ui.key_input(KeyCode::Space, KeyAction::Press)?;
    ui.key_input(KeyCode::Space, KeyAction::Release)?;
    ui.flush(ui.now())?;
    Ok(())
}

/// Name of the first visible component under cell (row, col) whose name satisfies `pred`.
fn cell_text(ui: &Runtime, k: &Keys, row: u32, col: &str, pred: impl Fn(&str) -> bool) -> Option<String> {
    let rk = (k.row)(row);
    ui.inspect().components.into_iter().filter(|c| c.visible).find_map(|c| {
        let p = c.path?;
        let i = p.windows(3).position(|w| w[0] == k.table && w[1] == rk && w[2] == col)?;
        if p.len() <= i + 3 {
            return None; // the cell itself
        }
        let n = c.details.semantics?.name?;
        pred(&n).then_some(n)
    })
}

fn key_text(ui: &Runtime, root: &WidgetId, key: &str) -> R<(String, String)> {
    let id = ui.find_descendant(root, key)?.ok_or("keyed widget missing")?;
    let iid = ui.inspection_id(&id)?;
    let c = ui.inspect().components.into_iter().find(|c| c.id == iid).ok_or("not inspected")?;
    let s = c.details.semantics.ok_or("no semantics")?;
    Ok((format!("{:?}", s.role), s.name.unwrap_or_default()))
}

fn wait(ui: &mut Runtime, limit: Duration, mut cond: impl FnMut(&Runtime) -> bool) -> R {
    let end = Instant::now() + limit;
    loop {
        ui.flush(ui.now())?;
        if cond(ui) {
            return Ok(());
        }
        if Instant::now() > end {
            return Err("timed out".into());
        }
        std::thread::sleep(Duration::from_millis(5));
    }
}

pub fn author_check(ui: &mut Runtime, root: &WidgetId, k: &Keys, needs_paint: fn(&Runtime) -> bool) -> R {
    ui.flush(ui.now())?;
    let start = ui.find_descendant(root, "start")?.ok_or("start missing")?;
    for _ in 0..5 {
        activate(ui, &start)?;
    }
    let (_, s) = key_text(ui, root, "summary")?;
    println!("AUTHOR after 5 starts: summary {s:?}");
    assert!(s.contains("Running: 5"), "summary {s}");
    // Item 1: progress reaches the row (production path).
    let pct = |t: &str| t.contains('%');
    wait(ui, Duration::from_secs(3), |ui| {
        cell_text(ui, k, 2, "progress", pct).is_some_and(|t| !t.starts_with("0%"))
    })?;
    // Item 2: cancel Job 2 through its row's Cancel button.
    let row2 = ui.find_descendant(root, &(k.row)(2))?.ok_or("row 2 missing")?;
    let cancel = ui.find_descendant(&row2, "cancel")?.ok_or("cancel missing")?;
    activate(ui, &cancel)?;
    let t0 = Instant::now();
    wait(ui, Duration::from_secs(1), |ui| {
        cell_text(ui, k, 2, "state", |t| t.contains("cancelled")).is_some()
    })?;
    println!("AUTHOR job 2 shows cancelled {:?} after the key press", t0.elapsed());
    let frozen = (
        cell_text(ui, k, 2, "progress", pct),
        cell_text(ui, k, 2, "elapsed", |_| true),
    );
    println!("AUTHOR job 2 at cancel: {frozen:?}");
    // Remaining jobs run to completion or failure in real time.
    wait(ui, Duration::from_secs(20), |ui| {
        key_text(ui, root, "summary").is_ok_and(|(_, s)| s.contains("Running: 0"))
    })?;
    let after = (
        cell_text(ui, k, 2, "progress", pct),
        cell_text(ui, k, 2, "elapsed", |_| true),
    );
    assert_eq!(frozen, after, "cancelled job 2 changed after cancellation");
    assert!(cell_text(ui, k, 2, "state", |t| t.contains("cancelled")).is_some());
    for id in [1, 3, 4] {
        let st = cell_text(ui, k, id, "state", |_| true);
        let p = cell_text(ui, k, id, "progress", pct);
        println!("AUTHOR job {id}: state {st:?} progress {p:?}");
        assert_eq!(st.as_deref(), Some("done"));
        assert!(p.is_some_and(|p| p.starts_with("100%")));
    }
    // Item 3: job 5 fails at a random point; row and notification surface.
    let st5 = cell_text(ui, k, 5, "state", |_| true).unwrap_or_default();
    let p5 = cell_text(ui, k, 5, "progress", pct);
    let (role, note) = key_text(ui, root, "notification")?;
    println!("AUTHOR job 5: state {st5:?} progress {p5:?}; notification {role} {note:?}");
    assert!(st5.contains("failed"));
    assert!(note.contains("Job 5") && note.contains("fail"));
    let (_, s) = key_text(ui, root, "summary")?;
    println!("AUTHOR final summary {s:?}");
    assert!(s.contains("Running: 0") && s.contains("Done: 3") && s.contains("Cancelled: 1") && s.contains("Failed: 1"));
    // Idle: nothing scheduled, nothing to paint, no component work.
    ui.paint()?;
    assert_eq!(ui.next_deadline(), None, "deadline while idle");
    assert!(!needs_paint(ui), "paint requested while idle");
    for secs in [1, 10, 60] {
        let report = ui.flush(ui.now() + Duration::from_secs(secs))?;
        assert_eq!(report.components_updated, 0, "component updates while idle");
        assert_eq!(report.next_deadline, None);
        assert!(!needs_paint(ui), "paint requested while idle");
    }
    println!("AUTHOR idle: no deadline, no paint request, no component updates over 60 s of runtime time");
    Ok(())
}
```

Wrapper, appended to the acceptance copy of `src/main.rs` only (the submitted `work/` is unchanged):

```rust
// ---- Author acceptance check (added in the acceptance copy only) ----
#[cfg(test)]
#[path = "author_common.rs"]
mod author_common;
#[cfg(test)]
mod author_test {
    
    #[test]
    fn author_production_session() -> Result<(), Box<dyn std::error::Error>> {
        let mut ui = pert_ui::Runtime::new();
        ui.set_viewport(pert_ui::Viewport::new(1100.0, 600.0, 1.0)?)?;
        let (_monitor, root) = super::mount(&mut ui)?;
        let keys = super::author_common::Keys { table: "jobs", row: |n| format!("job-{n}") };
        super::author_common::author_check(&mut ui, &root, &keys, |ui| ui.needs_paint())
    }
}
```

**Maintainer decision, 2026-10-27:** the task names no component, so a
visible failure message outside the row meets item 3 (the literal reading of
the frozen brief). The run is met; that `Toast` does not point to `Banner` is
recorded as a finding of round 2.
