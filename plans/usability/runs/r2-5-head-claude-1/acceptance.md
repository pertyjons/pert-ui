# Acceptance: r2-5-head-claude-1 (task 5, background job monitor)

Author check, 2026-10-27, kit `kit-aca8df3` (pert-ui 0.3.75). The submitted
`work/` (without `target/`, `.claude/` and `.git/`) was copied to
`~/pert-ui-usability/acceptance-r2-5-head-claude-1`, built with
`cargo build --offline --all-targets` and tested with `cargo test --offline`
in the kit sandbox: builds without warnings; all tests pass (1 unit test in
`src/engine.rs`, 7 headless tests in `tests/monitor.rs`, 1 wall-clock test in
`tests/app.rs` on `App::headless`). The capture test rewrote
`captures/jobs.png` byte-identical to the submitted one; the author opened
it. The tester's checks drive a `Runtime` through `pert_ui::testing::Driver`
(clicks on Start job, Cancel and a column header, an Alt+Up/Alt+Down chord on
the table) with a `ManualClock` and scripted plans, and read results from
inspection (`Driver::semantics` names and values, an `InspectionQuery` with
geometry for row order). The application's own `jobs()` is used only to wait
for worker reports.

One author check was added in the acceptance copy only
(`acceptance-r2-5-head-claude-1/work/tests/author.rs`,
`author_no_repaint_while_idle`, passes), because the tester's idle check reads
the flush report but not whether a repaint is pending: two jobs (one fails,
so a toast is posted) run to their end; after the toast expires,
`Runtime::paint` leaves `needs_paint()` false and `next_deadline()` `None`,
and a flush a minute later updates no component and requests no paint.

| Item | Result | Code | Check |
|---|---|---|---|
| 1. Start jobs; progress, completion and summary update | met | Start `src/lib.rs:103-130` (plan, `Signal<ProgressStatus>`, worker), reports applied on the UI thread `:146-195`, summary and rows republished `:198-213`; worker loop `src/engine.rs:163-201` | `tests/monitor.rs:153-189` (two jobs started by click; percent, elapsed, state cells, the bar's semantic value and the summary line read through inspection after each tick; both done, summary `Done: 2`, no Cancel on a done row); `tests/app.rs:19-61` on the wall clock |
| 2. Cancel a running job and see it stop | met | Cancel button `src/lib.rs:292-299` → `cancel_job` `:132-143` (sets the flag, row cancelled at once); worker returns on the flag `src/engine.rs:178-184`, sleeps wake within 5 ms `:36-45` | `tests/monitor.rs:211-235` (Cancel clicked; state cell and summary from inspection; the worker thread count reaches 0 without the clock moving; 20 more ticks leave the percent cell at 15%); `tests/app.rs:44-55` (thread ends within 50 ms on the wall clock) |
| 3. Failure in its row and as a notification | met | `src/lib.rs:174-181` (state `failed: <reason>`, `ProgressStatus::Failed`), toast `:190-193` | `tests/monitor.rs:237-258` (state cell `failed: I/O error at unit 35`, percent, summary, bar `invalid` from inspection; the toast stack's semantic value goes 0 → 1; the toast's `ToastReceipt::status()` is `Visible`). The toast's text is not in inspection (library, see triage); the capture shows it |
| 4. Sorting by each column orders the rows | met | `src/lib.rs:236-251` (name by job number, progress by fraction, state running < done < cancelled < failed, elapsed; ties by start order; descending reverses), sort stored from `Table::on_sort` `:430-436` | `tests/monitor.rs:283-325` (name, progress, state and elapsed, both directions, via the table's Alt+Up/Alt+Down chord; row order from inspection geometry; order kept across a tick); `:346-367` (header clicks on Name sort ascending then descending) |
| 5. CPU capture (PNG) with jobs in several states | met | — | `tests/monitor.rs:327-344` (`set_capture_support`, window `request_capture`, `CpuRenderer::serve_captures`, `encode_png`); `captures/jobs.png` 900x420: rows done (green), failed (red text and bar), cancelled, two running with Cancel buttons, summary `Running: 2 Done: 1 Cancelled: 1 Failed: 1`, failure toast, Name sort indicator |

## Task-specific checks

- **Progress through the library's cross-thread facility:** met. Workers hold
  only a `UiSender<Report>` (from `Runtime::mailbox`, `src/lib.rs:411-415`),
  their plan, the clock and an `Arc<AtomicBool>`; progress reports are lossy on
  a full mailbox, terminal ones retried (`:222-234`). No UI object crosses a
  thread.
- **Cancellation stops the worker:** met (item 2): the thread ends (live-thread
  count 0) promptly, without another tick.
- **Failure in the row and as a notification:** met (item 3).
- **Sorting by every column:** met (item 4), every sortable column, both
  directions.
- **No repaint while idle:** met. Tester: `tests/monitor.rs:191-209` (no
  deadline and no component updated, before any job and after the jobs end,
  no worker thread left). Author: `author_no_repaint_while_idle` adds
  `needs_paint()` false after a paint and after a minute.

## Triage notes

Not acceptance failures.

- Library: the toast stack's inspection (`ToastStack::inspect`,
  `pert-ui-runtime-0.3.75/src/notifications.rs:808-836`) reports only
  `visible_count` (and the semantic value with the same count), not the
  messages, so a headless check cannot read which notification is shown. The
  tester kept its own list of messages with their `ToastReceipt`s
  (`src/lib.rs:98-99`, `:371-374`) to check the text.
- Sorting by keyboard requires focusing the table body (first child of the
  table, `tests/monitor.rs:130-135`) and setting the active row and column
  through the controller before the chord.

## Author check source

`acceptance-r2-5-head-claude-1/work/tests/author.rs`:

```rust
//! Author check (acceptance only): the tester's idle check reads the flush
//! report (no deadline, nothing updated). This adds the paint side: after the
//! jobs ended (one by failure, so a toast was posted and expired), nothing
//! needs painting and nothing is scheduled, also after a long wait.

use std::sync::Arc;
use std::time::Duration;

use job_monitor::engine::{JobPlan, ManualClock, Planner, TICK};
use job_monitor::{Environment, JobState, build};
use pert_ui::prelude::*;
use pert_ui::testing::*;

type R = Result<(), Box<dyn std::error::Error>>;

struct Plans(Vec<JobPlan>);
impl Planner for Plans {
    fn plan(&mut self) -> JobPlan {
        self.0.remove(0)
    }
}

#[test]
fn author_no_repaint_while_idle() -> R {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(900.0, 420.0, 1.0)?)?;
    let clock = Arc::new(ManualClock::default());
    let plans = Plans(vec![
        JobPlan { size: 20, steps: vec![10], fail_at: None },
        JobPlan { size: 40, steps: vec![10], fail_at: Some(15) },
    ]);
    let app = build(&mut ui, Environment { clock: clock.clone(), planner: Box::new(plans) })?;
    let mut d = Driver::new(&mut ui);
    d.settle()?;
    d.runtime_mut().paint()?;
    assert!(!d.runtime().needs_paint(), "idle before any job");
    assert_eq!(d.runtime().next_deadline(), None);

    let start = d.find(&app.root, &["content", "toolbar", "start"])?;
    for _ in 0..2 {
        d.click(&start)?;
        d.advance(ClickCounter::DEFAULT_INTERVAL + Duration::from_millis(1))?;
    }
    for _ in 0..3 {
        clock.advance(TICK);
        std::thread::sleep(Duration::from_millis(20));
    }
    let a = app.clone();
    d.wait_until(Duration::from_secs(5), |_| Ok(a.jobs().iter().all(|j| j.state != JobState::Running)))?;
    let w = app.workers.clone();
    d.wait_until(Duration::from_secs(5), |_| Ok(w.get() == 0))?;
    d.settle()?;
    let stack = d.find(&app.root, &["toasts"])?;
    assert!(matches!(d.semantics(&stack)?.value, Some(SemanticValue::Number { value, .. }) if value == 1.0));
    // Let the failure toast expire on the runtime clock.
    d.advance(Duration::from_secs(10))?;
    d.settle()?;
    d.runtime_mut().paint()?;
    assert!(!d.runtime().needs_paint(), "nothing to repaint once jobs ended");
    assert_eq!(d.runtime().next_deadline(), None, "nothing scheduled");
    d.advance(Duration::from_secs(60))?;
    let report = d.settle()?;
    assert_eq!(report.components_updated, 0);
    assert!(!d.runtime().needs_paint(), "still nothing to repaint after a minute");
    Ok(())
}
```
