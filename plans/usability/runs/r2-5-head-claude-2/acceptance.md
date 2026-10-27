# Acceptance: r2-5-head-claude-2 (task 5, background job monitor)

Author check, 2026-10-27, kit `kit-aca8df3` (pert-ui 0.3.75). The submitted
`work/` (without `target/`, `.claude/` and `.git/`) was copied to
`~/pert-ui-usability/acceptance-r2-5-head-claude-2`, built with
`cargo build --offline --all-targets` and tested with `cargo test --offline`
in the kit sandbox: builds without warnings; all tests pass (5 headless tests
in `tests/headless.rs`, 2 in `tests/real_clock.rs`: the native app on
`App::headless` with the wall clock and the random planner, and the plan
distribution). The capture test rewrote `capture.png` byte-identical to the
submitted one; the author opened it. The tester's checks drive a `Runtime`
through `pert_ui::testing::Driver` (clicks on Start job and Cancel, pointer
presses on column headers) with a `ManualClock` and scripted plans, and read
cells, the summary, the bars and the toast stack through `Driver::semantics`.
The application's own `job()`/`jobs()` are used to wait for worker reports.

One author check was added in the acceptance copy only
(`acceptance-r2-5-head-claude-2/work/tests/author.rs`,
`author_sort_order_by_geometry`, passes), because the tester's sort test
reads row order from the table body's child order: it builds the same four
states, clicks each header twice and reads the rows' laid-out order from an
`InspectionQuery` with geometry. Every column sorts correctly in both
directions, and the child order equals the geometry order.

| Item | Result | Code | Check |
|---|---|---|---|
| 1. Start jobs; progress, completion and summary update | met | Start `src/lib.rs:187-211` (worker thread, `Signal<ProgressStatus>`), reports applied on the UI thread `:229-272`, rows and summary republished `:280-293`; worker loop `src/sim.rs:155-198` | `tests/headless.rs:117-160` (two jobs started by click; name, state, percent, elapsed, bar value and description, summary from inspection after each tick; both done, summary `Done: 2`, no Cancel on a finished row); `tests/real_clock.rs:9-41` (three random jobs on the wall clock, summary label from inspection) |
| 2. Cancel a running job and see it stop | met | Cancel button `src/lib.rs:339-346` → `cancel_job` `:215-227`; worker stops on the flag `src/sim.rs:168-171`, sleeps in 5 ms slices `:39-48` | `tests/headless.rs:162-195` (Cancel clicked; state cell, bar `cancelled`, summary from inspection; the worker's final report arrives without the clock moving; 5 more ticks leave the percent cell at 20%; idle afterwards) |
| 3. Failure in its row and as a notification | met | `src/lib.rs:252-259` (state `failed: <reason>`), `ProgressStatus::Failed` `:75-82`, toast `:267-270` | `tests/headless.rs:197-228` (state cell, percent, bar description `failed` and `invalid`, summary from inspection; toast stack count 0 → 1 → 0 after expiry; `ToastReceipt::status()` `Visible` then `Expired`). The toast's text is not in inspection (library, see triage); the capture shows it |
| 4. Sorting by each column orders the rows | met | `src/lib.rs:107-116` (`compare`: name by number, progress by fraction, state running < done < cancelled < failed, elapsed; ties by number), applied in `publish` `:280-289`, sort from `Table::on_sort` `:411-418` | `tests/headless.rs:277-326` (header clicks on every column, both directions, cell texts in shown order, order kept as progress changes); author check reads the order from geometry |
| 5. CPU capture (PNG) with jobs in several states | met | — | `tests/headless.rs:328-350` (`set_capture_support`, window `request_capture`, `CpuRenderer::serve_captures`, `encode_png`); `capture.png` 820x300: done, cancelled, failed (red), running with Cancel, summary `Running: 1 · Done: 1 · Cancelled: 1 · Failed: 1`, failure toast |

## Task-specific checks

- **Progress through the library's cross-thread facility:** met. Workers hold
  only a `UiSender<Report>` (from `Runtime::mailbox`, `src/lib.rs:396-401`),
  their plan, the clock and an `Arc<AtomicBool>`; progress is lossy on a full
  mailbox, the final report retried (`src/sim.rs:181-197`).
- **Cancellation stops the worker:** met (item 2), the worker's final report
  arrives without another tick and nothing changes afterwards.
- **Failure in the row and as a notification:** met (item 3).
- **Sorting by every column:** met (item 4).
- **No repaint while idle:** met. `tests/headless.rs:106-115` `assert_idle`
  (after `paint`, no component updated, no deadline, `needs_paint()` false),
  used after all jobs end, again after 30 s (`:155-158`) and after a
  cancellation (`:193`).

## Triage notes

Not acceptance failures.

- Library: the toast stack's inspection (`ToastStack::inspect`,
  `pert-ui-runtime-0.3.75/src/notifications.rs:808-836`) reports only the
  visible count, not the messages; the tester kept its own message list with
  `ToastReceipt`s (`src/lib.rs:145-147`, `:173-176`).
- The progress bar is wrapped in a `Container` with a small preferred size
  (`src/lib.rs:321-326`); the source comment says the bar prefers three
  control widths, more than the cell has. Not verified by the author.

## Author check source

`acceptance-r2-5-head-claude-2/work/tests/author.rs`:

```rust
//! Author check (acceptance only): the tester's sort test reads row order from
//! the table body's child order. This reads the order the rows are laid out
//! in (inspection geometry, top to bottom) after header clicks on every
//! column, both directions, and compares it with the child order too.

use job_monitor::sim::{JobPlan, Planner, TICK};
use job_monitor::*;
use pert_ui::prelude::*;
use pert_ui::testing::*;
use std::collections::VecDeque;
use std::sync::Arc;
use std::time::Duration;

type R<T = ()> = Result<T, Box<dyn std::error::Error>>;

struct Script(VecDeque<JobPlan>);
impl Planner for Script {
    fn plan(&mut self, _: u64) -> JobPlan {
        self.0.pop_front().expect("plan")
    }
}

fn plan(size: u64, rate: u64, fail_at: Option<u64>) -> JobPlan {
    JobPlan { size, rate: (rate, rate), fail_at, seed: 1 }
}

fn tick(clock: &ManualClock, monitor: &Monitor, d: &mut Driver<'_>, exp: &[(&str, u64)]) -> R {
    clock.advance(TICK);
    d.wait_until(Duration::from_secs(10), |_| {
        Ok(exp.iter().all(|(n, v)| monitor.job(n).is_some_and(|j| j.done == *v)))
    })?;
    d.settle()?;
    Ok(())
}

fn click_start(d: &mut Driver<'_>, root: &WidgetId) -> R {
    let b = d.find(root, &["content", "toolbar", "start"])?;
    d.click(&b)?;
    d.advance(ClickCounter::DEFAULT_INTERVAL + Duration::from_millis(1))?;
    Ok(())
}

/// Row keys by laid-out top edge (geometry), from an inspection query.
fn geometry_order(ui: &Runtime) -> Vec<String> {
    let result = ui.query(
        &InspectionQuery::components(Selector::children(["app", "content", "jobs"])).detail(Detail::Geometry),
    );
    let mut rows: Vec<(f32, String)> = result
        .components
        .iter()
        .filter(|c| c.key.starts_with("job"))
        .map(|c| (c.details.box_model.border_box.map(|b| b[1]).unwrap_or(f32::MAX), c.key.clone()))
        .collect();
    rows.sort_by(|a, b| a.0.total_cmp(&b.0));
    rows.into_iter().map(|(_, k)| k).collect()
}

fn child_order(ui: &Runtime, root: &WidgetId) -> R<Vec<String>> {
    let table = ui.find(root, &["content", "jobs"])?.ok_or("table")?;
    let body = ui.children(&table)?.remove(0);
    Ok(ui.children(&body)?.iter().map(|r| ui.key(r).map(str::to_owned)).collect::<Result<_, _>>()?)
}

fn header_click(d: &mut Driver<'_>, root: &WidgetId, column: &str) -> R {
    let ui = d.runtime_mut();
    let table = ui.find(root, &["content", "jobs"])?.ok_or("table")?;
    let [_, ty, _, _] = ui.bounds(&table)?.ok_or("table bounds")?.values();
    let cell = ui.find(root, &["content", "jobs", "job1", column])?.ok_or("cell")?;
    let [x, _, w, _] = ui.bounds(&cell)?.ok_or("cell bounds")?.values();
    let p = Point::new(x + w / 2.0, ty + 8.0)?;
    for k in [PointerKind::Press(PointerButton::Primary), PointerKind::Release(PointerButton::Primary)] {
        ui.pointer_input(PointerEvent::new(p, k))?;
    }
    d.advance(ClickCounter::DEFAULT_INTERVAL + Duration::from_millis(1))?;
    Ok(())
}

#[test]
fn author_sort_order_by_geometry() -> R {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(820.0, 300.0, 1.0)?)?;
    let clock = Arc::new(ManualClock::default());
    let plans = vec![plan(75, 25, None), plan(400, 10, None), plan(100, 20, Some(80)), plan(100, 10, None)];
    let built = build(&mut ui, Deps { clock: clock.clone(), planner: Box::new(Script(plans.into())) })?;
    let (root, monitor) = (built.root.clone(), built.monitor.clone());
    let mut d = Driver::new(&mut ui);
    d.settle()?;
    // Job 1 done 100% 0.3 s; Job 2 cancelled 2% 0.1 s; Job 3 failed 80% 0.4 s; Job 4 running 20% 0.2 s.
    click_start(&mut d, &root)?;
    click_start(&mut d, &root)?;
    tick(&clock, &monitor, &mut d, &[("Job 1", 25), ("Job 2", 10)])?;
    let cancel = d.find(&root, &["content", "jobs", "job2", "action", "cancel"])?;
    d.click(&cancel)?;
    d.advance(ClickCounter::DEFAULT_INTERVAL + Duration::from_millis(1))?;
    click_start(&mut d, &root)?;
    tick(&clock, &monitor, &mut d, &[("Job 1", 50), ("Job 3", 20)])?;
    tick(&clock, &monitor, &mut d, &[("Job 1", 75), ("Job 3", 40)])?;
    d.wait_until(Duration::from_secs(10), |_| Ok(monitor.job("Job 1").unwrap().state == JobState::Done))?;
    click_start(&mut d, &root)?;
    tick(&clock, &monitor, &mut d, &[("Job 3", 60), ("Job 4", 10)])?;
    tick(&clock, &monitor, &mut d, &[("Job 3", 80), ("Job 4", 20)])?;
    d.wait_until(Duration::from_secs(10), |_| Ok(monitor.job("Job 3").unwrap().state == JobState::Failed))?;
    d.settle()?;

    let cases: [(&str, [&str; 4], [&str; 4]); 4] = [
        ("name", ["job1", "job2", "job3", "job4"], ["job4", "job3", "job2", "job1"]),
        ("progress", ["job2", "job4", "job3", "job1"], ["job1", "job3", "job4", "job2"]),
        ("state", ["job4", "job1", "job2", "job3"], ["job3", "job2", "job1", "job4"]),
        ("elapsed", ["job2", "job4", "job1", "job3"], ["job3", "job1", "job4", "job2"]),
    ];
    for (column, asc, desc) in cases {
        for expected in [asc, desc] {
            header_click(&mut d, &root, column)?;
            d.settle()?;
            let geo = geometry_order(d.runtime());
            assert_eq!(geo, expected, "{column} by geometry");
            assert_eq!(child_order(d.runtime(), &root)?, geo, "{column}: child order = geometry order");
        }
    }
    Ok(())
}
```
