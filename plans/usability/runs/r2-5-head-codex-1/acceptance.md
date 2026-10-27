# Acceptance: r2-5-head-codex-1 (task 5, background job monitor)

Author check, 2026-10-27, kit `kit-aca8df3` (0.3.75). The submitted `work/`
(without `target/`, `.claude/` and `.git/`) was copied to
`~/pert-ui-usability/acceptance-r2-5-head-codex-1`, built with
`cargo build --all-targets` and tested with `cargo test` in the kit sandbox
(offline): builds without warnings from a clean target, and the tester's one
headless test (`tests/headless.rs` `full_headless_session_and_cpu_capture`)
passes. It rewrote `job-monitor.png` byte-identically to the submitted one.
The tester's test runs the real worker threads and mailbox with a
deterministic plan and a test-driven tick (`Monitor::tick`), which the task
allows. The author added one check, `tests/author.rs` with
`tests/author_common.rs` (source below), that runs the production
configuration (random sizes and steps, real 100 ms ticks, real clock) through
keyboard input and inspection only; it passes (about 4.5 s).

| Item | Result | Code | Check |
|---|---|---|---|
| 1. Start jobs, see progress, completion and the summary update | met | Start `src/lib.rs:269-298` (worker thread per job, `UiSender`), worker `:144-197`, mailbox handler `:243-266`, summary derived from the jobs signal `:299-308` | tester `tests/headless.rs:59-80` (clicks Start four times, summary from inspection "Running: 4 …", row percent "40%" from inspection, completion of job 1, summary later "Running: 1 Done: 1 Cancelled: 1 Failed: 1" `:107-110`); author check: production path, five starts, nonzero row percent, jobs 1, 3, 4 read "done"/"100%" from inspection, summary "Running: 0 Done: 3 Cancelled: 1 Failed: 1" |
| 2. Cancel a running job and see it stop | met | Cancel button sends `Command::Cancel` `:83-92`; worker sets Cancelled, sends and returns `:167`, `:194-196`; control removed on terminal state `:251-253` | tester `:81-97` (clicks Job 2's Cancel through `Driver::click`; state and unchanged progress read from the app's `jobs` signal, summary via inspection); author check: Cancel activated by key input on the production path, row state "cancelled" from inspection within 1 ms, row percent and elapsed text unchanged while the other jobs ran to completion (about 4 s) |
| 3. Failure in its row and as a notification | met | every fifth job gets `fail_at` `:278-282`; row state "failed: simulated error" `:76-81`; `Banner` (library notification surface, role Alert) set from the mailbox handler `:244-249`, `:316` | tester `:98-106` (row text and "Job 3 failed" from inspection); author check: production job 5 fails at a random point, its row reads "failed: simulated error", the `notification` widget is role `Alert` named "Notification: Job 5 failed — simulated error" |
| 4. Sorting by each column orders the rows correctly | met | `ordered` `src/lib.rs:107-127` (name by id, progress by exact fraction, state by its text, elapsed by duration, ties by id), applied on every refresh and on `Table::on_sort` `:309` | tester `:113-165`: Alt+ArrowUp/ArrowDown key input on the table body for all four columns, both directions, against fixed expected orders, and the painted row order read from inspection box positions |
| 5. CPU capture PNG with jobs in several states | met | `tests/headless.rs:166-175` (`request_capture`, `CpuRenderer::serve_captures`, `encode_png`) | `work/job-monitor.png` (1100×560): Job 1 done 100 %, Job 3 failed 20 %, Job 2 cancelled 12 %, Job 4 running 1 % with its Cancel button, sorted by elapsed descending, notification banner at the bottom; reproduced byte-identically by the author |

## Task-specific checks

- **Progress through the library's cross-thread facility:** met. Workers
  receive a plain `Job`, a `Plan`, a `std::sync::mpsc::Receiver` and a
  `UiSender<Job>` (`src/lib.rs:144-150`, spawn `:296`); all UI state is
  updated in the `ui.mailbox` handler on the UI thread (`:243-266`). A full
  mailbox is retried without dropping terminal states (`:183-193`).
- **Cancellation stops the worker:** met. The worker returns after sending
  the cancelled state (`:167`, `:194-196`); in production mode the
  `recv_timeout` wait makes it prompt. Author check: row state reads
  "cancelled" right after the key press and its progress and elapsed text no
  longer change.
- **Failure in the row and as a notification:** met (item 3), through the
  library's `Banner`.
- **Sorting by every column:** met (item 4).
- **No repaint while idle:** met. Tester: `next_deadline()` is `None` and
  `Driver::advance` reports no component updates and no deadline over 3 s
  (`tests/headless.rs:194-201`); it does not check paint requests. Author
  check: after all jobs stopped, `paint()`, then `next_deadline() == None`,
  `HostIntegration::needs_paint()` false and `flush` reports
  `components_updated == 0` at +1 s, +10 s and +60 s of runtime time.

## Triage notes

Not acceptance failures.

- The tester's idle check could not observe paint requests through
  `pert_ui::testing` (`Driver`, `FlushReport`); at 0.3.75 `needs_paint` is
  reachable only through the host trait `pert_ui::host::HostIntegration`
  (`pert-ui-runtime-0.3.75/src/host.rs:272-278`; the inherent method is
  `pub(crate)`, `src/runtime/compose.rs:342`). A lead for the testing
  facade, not a defect in this submission.
- The tester's cancellation stop check reads the app's own signal and sends
  ticks that the app no longer routes to the cancelled job, so on its own it
  could not fail; the author's production-path check is the evidence.
- Each cell build makes a fresh `ProgressStatus` signal with
  `Signal::sibling` (`src/lib.rs:70`); this is the documented per-row binding
  form.
- Name sorting is by job number, so "Job 10" follows "Job 9".

## Author check source

`author_common.rs` (identical for every task-5 run judged in this batch):

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

Wrapper (tests/author.rs):

```rust
// Author acceptance check (not the tester's): production configuration.
mod author_common;
use pert_ui::host::HostIntegration;
use pert_ui::prelude::*;
use pert_ui::testing::Viewport;
#[test]
fn author_production_session() -> Result<(), Box<dyn std::error::Error>> {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(1100.0, 600.0, 1.0)?)?;
    let m = job_monitor::build(&mut ui, None)?;
    let keys = author_common::Keys { table: "jobs", row: |n| n.to_string() };
    author_common::author_check(&mut ui, &m.root, &keys, |ui| ui.needs_paint())
}
```
