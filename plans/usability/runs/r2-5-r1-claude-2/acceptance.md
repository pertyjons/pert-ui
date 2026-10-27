# Acceptance: r2-5-r1-claude-2 (task 5, background job monitor)

Author check, 2026-10-27, kit `kit-359122e-r2` (pert-ui 0.3.52). The
submitted `work/` (without `target/`, `.claude/` and `.git/`) was copied to
`~/pert-ui-usability/acceptance-r2-5-r1-claude-2`, built with
`cargo build --offline --all-targets` and tested with `cargo test --offline`
in the kit sandbox: builds without warnings; all 6 headless tests in
`tests/monitor.rs` pass. They rewrote `capture-window.png`,
`capture-table.png` and `capture-failure.png` byte-identical to the submitted
ones; the author opened them. The tester's checks drive a `Runtime` with
keyboard (Enter on the focused Start button, Space on Cancel) and pointer
input (Cancel, column headers), with a per-job stepped clock (`Pacer`) and
scripted plans, and read row order from laid-out bounds, the toast stack's
`visible_count` from inspection and, in places, text from inspection. The
`--headless` mode of the binary (`src/main.rs:30-76`, wall clock) was not run.

Two author checks were added in the acceptance copy only
(`acceptance-r2-5-r1-claude-2/work/tests/author.rs`, both pass), because the
tester's checks read job 1's completion, the final summary and the cancelled
job's stopped progress from the application's own model (`rows()`,
`summary()`), and other texts only as "some label in the snapshot shows
this":

- `author_progress_completion_summary_by_inspection`: two jobs started by
  Enter on Start job; each row's name, percent, state and elapsed cells and
  the summary label read from `Runtime::inspect` after 3, 3 and 10 ticks;
  both rows `done`, `100%`, job 1 `1.0 s`, summary
  `Running: 0   Done: 2   Cancelled: 0   Failed: 0`.
- `author_cancelled_row_stops_by_inspection`: Cancel clicked at 20%; the row
  shows `cancelled` and the summary `Cancelled: 1` once the worker reports;
  after further ticks for both jobs the cancelled row's percent (20%),
  elapsed and state are unchanged while the other job reaches 40%.

| Item | Result | Code | Check |
|---|---|---|---|
| 1. Start jobs; progress, completion and summary update | met | Start `src/lib.rs:318-345` (per-row signals, worker thread), events applied on the UI thread `:347-403`, summary and rows republished `:421-436`; worker loop `:175-205` | tester `tests/monitor.rs:208-243` (partly from the model); author `author_progress_completion_summary_by_inspection` |
| 2. Cancel a running job and see it stop | met | Cancel button `src/lib.rs:488-492` (`CancelToken::cancel`, `:84-102`, wakes the worker's wait); `run_job` stops on `Pace::Cancelled` `:185-191`; real pacer `:136-147` | tester `tests/monitor.rs:245-277` (Cancel by pointer; the worker thread ends; `cancelled` shown; no Cancel on the row), `:279-312` (wall clock, Space on Cancel, cancelled within 50 ms); author `author_cancelled_row_stops_by_inspection` |
| 3. Failure in its row and as a notification | met | `src/lib.rs:373-392` (state `failed: <reason>`, `ProgressStatus::Failed`), toast `:393-401` | `tests/monitor.rs:314-368` (the row's `state-label` from inspection `failed: simulated error at unit 35`; toast stack `visible_count` 1; `ToastReceipt::status()` `Visible`, `Expired` after 6 s; `capture-failure.png` shows the toast). The toast's text is not in inspection (library, see triage) |
| 4. Sorting by each column orders the rows | met | `src/lib.rs:405-418` (`compare`: name by number, progress by fraction, state running < done < cancelled < failed, elapsed; ties by number), sort from `Table::on_sort` `:602-606` | `tests/monitor.rs:370-425` (header clicks on every column, both directions, row order from laid-out bounds, sort indication from `TableController::sort`; order follows progress changes) |
| 5. CPU capture (PNG) with jobs in several states | met | — | `tests/monitor.rs:427-466` (`set_capture_support`, window and table-component `request_capture`, `CpuRenderer::serve_captures` serves 2, `encode_png`); `capture-window.png` 900x420: rows sorted by state, two running with Cancel, done, cancelled, failed (red bar), summary `Running: 2 Done: 1 Cancelled: 1 Failed: 1`, failure toast; `capture-table.png` the table alone |

## Task-specific checks

- **Progress through the library's cross-thread facility:** met. Workers hold
  a `UiSender<JobEvent>` (from `Runtime::mailbox`, `src/lib.rs:568-573`),
  their plan, a `Pacer` and a `CancelToken`; progress is lossy, terminal
  events retried while the mailbox is full (`:161-172`).
- **Cancellation stops the worker:** met (item 2); the cancelled state comes
  from the worker's own final event, and the thread ends.
- **Failure in the row and as a notification:** met (item 3).
- **Sorting by every column:** met (item 4).
- **No repaint while idle:** met. `tests/monitor.rs:238-241` (after both jobs
  end: no deadline, `needs_paint()` false after a paint) and `:354-366` (the
  only deadline left is the toast's expiry; after it, none and nothing to
  paint).

## Triage notes

Not acceptance failures.

- Library: the toast stack's inspection (`ToastStack::inspect`,
  `pert-ui-runtime-0.3.52/src/notifications.rs:797-826`) reports only the
  visible count, not the messages (the test says so, `tests/monitor.rs:344`);
  the tester kept its own message list with `ToastReceipt`s
  (`src/lib.rs:311`, `:543-546`).
- Every row cell is bound to per-job signals created as siblings of a
  placeholder `Signal<()>` (`src/lib.rs:314`, `:323-337`) so that the
  monitor can create signals outside an event context.
- Header clicks are positioned from the application's own fixed column
  widths (`tests/monitor.rs:194-205`).

## Author check source

`acceptance-r2-5-r1-claude-2/work/tests/author.rs`:

```rust
//! Author check (acceptance only). The tester's checks read completion and
//! the final summary, and the cancelled job's stopped progress, partly from
//! the application's own model. This reads each row's cells and the summary
//! line through inspection, driving Start and Cancel through input.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::Duration;

use job_monitor::*;
use pert_ui::prelude::*;
use pert_ui::{KeyAction, KeyCode, Point, PointerButton, PointerEvent, PointerKind, Viewport};

type R<T = ()> = Result<T, Box<dyn std::error::Error>>;

struct StepPacer { ticks: Receiver<()>, processed: Sender<()>, count: u32 }
impl Pacer for StepPacer {
    fn next_tick(&mut self, cancel: &CancelToken) -> Pace {
        if self.count > 0 { let _ = self.processed.send(()); }
        match self.ticks.recv() {
            Ok(()) => {
                self.count += 1;
                if cancel.is_cancelled() { Pace::Cancelled(TICK * self.count) } else { Pace::Tick(TICK * self.count) }
            }
            Err(_) => Pace::Cancelled(TICK * self.count),
        }
    }
}

#[derive(Clone, Default)]
struct Steps(Rc<RefCell<HashMap<u32, (Sender<()>, Receiver<()>)>>>);
impl Steps {
    fn step(&self, id: u32, n: usize) {
        for _ in 0..n {
            let map = self.0.borrow();
            let (tx, rx) = &map[&id];
            if tx.send(()).is_err() || rx.recv().is_err() { break; }
        }
    }
}

struct S { ui: Runtime, m: MonitorHandle, steps: Steps, now: Duration }

impl S {
    fn new(plans: Vec<JobPlan>) -> R<S> {
        let mut ui = Runtime::new();
        ui.set_viewport(Viewport::new(900.0, 420.0, 1.0)?)?;
        let steps = Steps::default();
        let st = steps.clone();
        let mut plans = plans.into_iter();
        let env = Env {
            planner: Box::new(move |_| plans.next().expect("plan")),
            pacers: Box::new(move |id| {
                let (ttx, trx) = channel();
                let (dtx, drx) = channel();
                st.0.borrow_mut().insert(id, (ttx, drx));
                Box::new(StepPacer { ticks: trx, processed: dtx, count: 0 })
            }),
        };
        let m = build(&mut ui, env)?;
        let mut s = S { ui, m, steps, now: Duration::ZERO };
        s.ui.flush(s.now)?;
        Ok(s)
    }
    fn tick(&mut self, id: u32, n: usize) -> R {
        self.steps.step(id, n);
        // Let the mailbox deliver what the worker sent.
        for _ in 0..20 {
            self.now += Duration::from_millis(10);
            self.ui.flush(self.now)?;
            std::thread::sleep(Duration::from_millis(2));
        }
        Ok(())
    }
    fn start(&mut self) -> R {
        let b = self.ui.find_descendant(&self.m.root, "start")?.ok_or("start")?;
        self.ui.focus(&b)?;
        self.ui.key_input(KeyCode::Enter, KeyAction::Press)?;
        self.ui.key_input(KeyCode::Enter, KeyAction::Release)?;
        self.ui.flush(self.now)?;
        Ok(())
    }
    fn name_of(&self, id: &WidgetId) -> R<String> {
        let iid = self.ui.inspection_id(id)?;
        let c = self.ui.inspect().components.into_iter().find(|c| c.id == iid).ok_or("not inspected")?;
        Ok(c.details.semantics.and_then(|s| s.name).unwrap_or_default())
    }
    fn row(&self, job: u32) -> R<WidgetId> {
        let table = self.ui.find_descendant(&self.m.root, "jobs")?.ok_or("table")?;
        Ok(self.ui.find_descendant(&table, &format!("job-{job}"))?.ok_or("row")?)
    }
    fn cell(&self, job: u32, key: &str) -> R<String> {
        let row = self.row(job)?;
        let w = self.ui.find_descendant(&row, key)?.ok_or_else(|| format!("{key} in job-{job}"))?;
        self.name_of(&w)
    }
    fn summary(&self) -> R<String> {
        let w = self.ui.find_descendant(&self.m.root, "summary")?.ok_or("summary")?;
        self.name_of(&w)
    }
    fn click(&mut self, id: &WidgetId) -> R {
        let r = self.ui.bounds(id)?.ok_or("bounds")?;
        let p = Point::new(r.x() + r.width() / 2.0, r.y() + r.height() / 2.0)?;
        for k in [PointerKind::Move, PointerKind::Press(PointerButton::Primary), PointerKind::Release(PointerButton::Primary)] {
            self.ui.pointer_input(PointerEvent::new(p, k))?;
        }
        self.ui.flush(self.now)?;
        Ok(())
    }
}

fn plan(total: u32, step: u32) -> JobPlan { JobPlan { total, steps: vec![step], fail_at: None } }

#[test]
fn author_progress_completion_summary_by_inspection() -> R {
    let mut s = S::new(vec![plan(100, 10), plan(60, 20)])?;
    assert_eq!(s.summary()?, "Running: 0   Done: 0   Cancelled: 0   Failed: 0");
    s.start()?;
    s.start()?;
    assert_eq!(s.summary()?, "Running: 2   Done: 0   Cancelled: 0   Failed: 0");
    assert_eq!(s.cell(1, "name-label")?, "Job 1");
    assert_eq!(s.cell(2, "name-label")?, "Job 2");
    s.tick(1, 3)?;
    assert_eq!(s.cell(1, "percent")?, "30%");
    assert_eq!(s.cell(1, "elapsed-label")?, "0.3 s");
    assert_eq!(s.cell(1, "state-label")?, "running");
    assert_eq!(s.cell(2, "percent")?, "0%");
    s.tick(2, 3)?;
    assert_eq!(s.cell(2, "state-label")?, "done");
    assert_eq!(s.cell(2, "percent")?, "100%");
    assert_eq!(s.summary()?, "Running: 1   Done: 1   Cancelled: 0   Failed: 0");
    s.tick(1, 10)?;
    assert_eq!(s.cell(1, "state-label")?, "done");
    assert_eq!(s.cell(1, "percent")?, "100%");
    assert_eq!(s.cell(1, "elapsed-label")?, "1.0 s");
    assert_eq!(s.summary()?, "Running: 0   Done: 2   Cancelled: 0   Failed: 0");
    Ok(())
}

#[test]
fn author_cancelled_row_stops_by_inspection() -> R {
    let mut s = S::new(vec![plan(200, 10), plan(200, 10)])?;
    s.start()?;
    s.start()?;
    s.tick(1, 4)?;
    s.tick(2, 4)?;
    assert_eq!(s.cell(1, "percent")?, "20%");
    let row = s.row(1)?;
    let cancel = s.ui.find_descendant(&row, "cancel")?.ok_or("cancel")?;
    s.click(&cancel)?;
    s.tick(1, 1)?; // the worker wakes, sees the token and ends
    assert_eq!(s.cell(1, "state-label")?, "cancelled");
    assert_eq!(s.summary()?, "Running: 1   Done: 0   Cancelled: 1   Failed: 0");
    // The step pacer stamps the cancellation with the tick that woke the
    // worker; what matters is that nothing changes afterwards.
    let elapsed = s.cell(1, "elapsed-label")?;
    s.tick(1, 5)?; // no further ticks reach a stopped worker
    s.tick(2, 4)?;
    assert_eq!(s.cell(1, "percent")?, "20%");
    assert_eq!(s.cell(1, "elapsed-label")?, elapsed);
    assert_eq!(s.cell(1, "state-label")?, "cancelled");
    assert_eq!(s.cell(2, "percent")?, "40%");
    Ok(())
}
```
