# Acceptance: r2-5-r1-codex-1 (task 5, background job monitor)

Author check, 2026-10-27, kit `kit-359122e-r2` (0.3.52). The submitted
`work/` (without `target/`, `.claude/` and `.git/`) was copied to
`~/pert-ui-usability/acceptance-r2-5-r1-codex-1`, built with
`cargo build --all-targets` and tested with `cargo test` in the kit sandbox
(offline): builds without warnings from a clean target, and the tester's one
check (`src/main.rs` `headless()`, run by `tests::headless_acceptance`)
passes. It rewrote `jobs.png` byte-identically to the submitted one. The
tester's check runs the real workers and mailbox with fixed sizes and a
test-driven tick over each worker's command channel, which the task allows.
The author added one check (source below), appended as a test module to the
acceptance copy of `src/main.rs`, that runs the production configuration
(`mount(ui, false)`: random sizes and steps, real 100 ms ticks, real clock)
through keyboard input and inspection only; it passes (about 4.2 s).

| Item | Result | Code | Check |
|---|---|---|---|
| 1. Start jobs, see progress, completion and the summary update | met | `Monitor::start` `src/main.rs:195-283` (worker thread `:228-281` with a `UiSender`), mailbox handler `Model::apply` `:145-169` via `ui.mailbox` `:322`, summary in `publish` `:133-144` | tester `:434-453` (Enter on Start five times, summary "Running: 5 …", row text "30%  (15 / 50 units)" and a progress-indicator value from inspection), completion and summary `:479-488`; author check: production path, nonzero row percent, jobs 1, 3, 4 read "done"/"100%" from inspection, final summary "Running: 0 Done: 3 Cancelled: 1 Failed: 1" |
| 2. Cancel a running job and see it stop | met | Cancel button sends `Command::Cancel` `:77-84`; the worker reports Cancelled and breaks `:244`, `:277-279` | tester `:455-468`, `:566-570` (reveals row 2, Enter on its Cancel, "cancelled" from inspection; its unchanged-progress check reads the app's model); author check: Cancel by key input on the production path, row "cancelled" from inspection about 13 ms later, row percent and elapsed text unchanged while the other jobs ran to completion |
| 3. Failure in its row and as a notification | met | every fifth job fails `:207-211`, `:250-256`; row state "failed: simulated processing error" `:92-98`; `Banner` (role Alert) shown by a visibility signal when a job fails `:159-167`, `:341-346` | tester `:480-484` (row text and notification text from inspection); author check: production job 5 fails at a random point, row reads "failed: simulated processing error", the `notification` widget is role `Alert` named "Job 5 failed: simulated processing error at 24 / 109 units" |
| 4. Sorting by each column orders the rows correctly | met | `Model::ordered` `:112-132` (name by text, progress by exact fraction, state by text, elapsed by duration, ties by id), applied in every `publish` and on `Table::on_sort` `:348` | tester `:489-540`: Alt+ArrowUp/ArrowDown key input on the focused table body for all four columns, both directions, sort state checked, rendered row order (names by inspected bounds) equal to the app's order, and that order checked monotonic in the column's value |
| 5. CPU capture PNG with jobs in several states | met | `:541-554` (`request_capture`, `CpuRenderer::serve_captures`, `encode_png`) | `work/jobs.png` (1200×700): Jobs 1 and 4 done, Job 2 cancelled, Job 3 running with Cancel, Job 5 failed, the failure banner above the table; reproduced byte-identically by the author |

## Task-specific checks

- **Progress through the library's cross-thread facility:** met. The worker
  closure owns its command receiver, sizes, seed and a `UiSender<Update>`
  (`:225-281`); rows, signals and the table are updated in the `ui.mailbox`
  handler on the UI thread (`:322`, `:145-169`). `deliver` retries a full
  mailbox and stops on closure (`:182-193`).
- **Cancellation stops the worker:** met. The worker's `recv_timeout` wakes
  on `Cancel`, reports Cancelled and breaks (`:244`, `:277-279`); `Drop for
  Monitor` closes the mailbox, cancels and joins every worker (`:285-295`).
  Author check as in item 2.
- **Failure in the row and as a notification:** met (item 3), through the
  library's `Banner`.
- **Sorting by every column:** met (item 4).
- **No repaint while idle:** met. Tester: after every job stopped,
  `paint()`, `next_deadline() == None`, and `needs_paint()` false after a
  further flush (`:575-580`). Author check: `next_deadline()` `None`,
  `needs_paint()` false and `components_updated == 0` at +1 s, +10 s and
  +60 s of runtime time.

## Triage notes

Not acceptance failures.

- Name sorting compares the text "Job N" (`:117`), so with ten or more jobs
  "Job 10" sorts between "Job 1" and "Job 2". The task does not define the
  name order and the checks use five jobs; noted, not failed.
- The mailbox handler panics on an unknown job id
  (`.expect("known worker")`, `:150`); unreachable as written, since rows are
  never removed.
- The tester's stop check reads the app's model and sends no further ticks to
  the cancelled job, so on its own it could not fail; the author's
  production-path check is the evidence.

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
        let (_monitor, root) = super::mount(&mut ui, false)?;
        let keys = super::author_common::Keys { table: "table", row: |n| n.to_string() };
        super::author_common::author_check(&mut ui, &root, &keys, |ui| ui.needs_paint())
    }
}
```
