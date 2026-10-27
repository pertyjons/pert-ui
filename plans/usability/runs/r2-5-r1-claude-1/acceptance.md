# Acceptance: r2-5-r1-claude-1 (task 5, background job monitor)

Author check, 2026-10-27, kit `kit-359122e-r2` (pert-ui 0.3.52). The
submitted `work/` (without `target/`, `.claude/` and `.git/`) was copied to
`~/pert-ui-usability/acceptance-r2-5-r1-claude-1`, built with
`cargo build --offline --all-targets` and tested with `cargo test --offline`
in the kit sandbox: builds without warnings; all tests pass (2 unit tests in
`src/sim.rs`, among them real-time cancellation within 20 ms, and 6 headless
tests in `tests/acceptance.rs`). The capture test rewrote `captures/jobs.png`
byte-identical to the submitted one; the author opened it. The tester's
checks drive a `Runtime` through the application's `Session`
(`src/headless.rs`), which uses only the library's public facilities:
pointer input at a component's bounds or a header position, `flush`, and
`Runtime::inspect` semantics names for cells and the summary
(`shown_text`, `src/headless.rs:146-161`), with a `ManualClock` and scripted
plans. The application's own `jobs()` is used to wait for worker reports.
No author check was needed.

| Item | Result | Code | Check |
|---|---|---|---|
| 1. Start jobs; progress, completion and summary update | met | Start `src/monitor.rs:380-406` (job record, `Signal<ProgressStatus>`, worker thread), reports applied on the UI thread `:342-375`, republished `:308-317`; worker loop `src/sim.rs:196-220` | `tests/acceptance.rs:33-63` (two jobs started by click; name, state, percent, elapsed and summary from inspection after 3 and 10 ticks; both done, summary `Done: 2`, no Cancel on a finished row) |
| 2. Cancel a running job and see it stop | met | Cancel button `src/monitor.rs:228-236` (sets the flag, wakes the clock); `run_job` stops on `wait_tick` returning false `src/sim.rs:201-204`; real clock `:104-127` | `tests/acceptance.rs:65-91` (Cancel clicked; the worker's `Cancelled` report arrives without a tick; state, summary from inspection; 5 more ticks leave percent 15% and elapsed 0.3 s while the other job goes on; no Cancel on the row); `src/sim.rs:250-270` (real clock, stop within 20 ms) |
| 3. Failure in its row and as a notification | met | `src/monitor.rs:354-358` (state `failed: <reason>`, `ProgressStatus::Failed`), toast `:362-373` | `tests/acceptance.rs:93-124` (state cell, percent, summary from inspection; the toast host's `visible_count` property 1; `ToastReceipt::status()` `Visible`, `Expired` after 5 s on the runtime clock, the row keeps the failure). The toast's text is not in inspection (library, see triage); the capture shows it |
| 4. Sorting by each column orders the rows | met | `src/monitor.rs:167-177` (`compare`: name by number, progress by exact fraction, state by its name, elapsed by ticks), ties by start order `:150-163`, sort from `Table::on_sort` `:409-412` | `tests/acceptance.rs:167-195` (header clicks on every column, both directions, sort indication from `TableController::sort`, row order from laid-out bounds; order kept across a tick). State sorts alphabetically by its label (cancelled, done, failed, running), a consistent order by the column's text |
| 5. CPU capture (PNG) with jobs in several states | met | `src/headless.rs:198-203` (`Runtime::paint`, `CpuRenderer::render`, `encode_png`) | `tests/acceptance.rs:218-227`; `captures/jobs.png` 820x420: done, running with Cancel, failed (red text and bar), cancelled, summary `Running: 2 · Done: 1 · Cancelled: 1 · Failed: 1`, failure toast. A CPU render of the painted scene rather than the capture service; both are the library's CPU renderer |

## Task-specific checks

- **Progress through the library's cross-thread facility:** met. Workers hold
  a `UiSender<JobEvent>` (from `Runtime::mailbox`, `src/monitor.rs:342`)
  wrapped in `MailboxReporter` (`:247-264`: progress lossy on a full mailbox,
  terminal reports retried), their plan, the clock and an `Arc<AtomicBool>`.
- **Cancellation stops the worker:** met (item 2); the row's cancelled state
  comes from the worker's own final report.
- **Failure in the row and as a notification:** met (item 3).
- **Sorting by every column:** met (item 4).
- **No repaint while idle:** met. `tests/acceptance.rs:197-216` (before any
  job and after both jobs end and the toast expires: no deadline,
  `needs_paint()` false after a paint, a flush a minute later updates no
  component).

## Triage notes

Not acceptance failures.

- Library: the toast stack's inspection (`ToastStack::inspect`,
  `pert-ui-runtime-0.3.52/src/notifications.rs:797-826`) reports only the
  visible count, not the messages; the tester kept its own `Notification`
  list with `ToastReceipt`s (`src/monitor.rs:119-126`, `:362-373`).
- Header clicks are positioned from the application's own fixed column widths
  (`src/headless.rs:132-143`); the progress bar is wrapped in a `Container`
  with a fixed preferred width (`src/monitor.rs:216-218`, comment: the bar
  prefers three control widths and rows never shrink a child). Not verified
  by the author.
