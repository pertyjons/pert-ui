# Acceptance: r3-5-head-codex (task 5, background job monitor)

Verifier: independent Claude acceptance session (claude-opus-5-5), 2026-10-28.
Procedure: `plans/usability/acceptance.md` (task-specific paragraph 5) and
round 3 rules. All builds and tests ran through
`run.sh exec /home/per/pert-ui-usability/kit-r3-head /home/per/pert-ui-usability/acceptance-r3-5-head-codex …`.
Nothing ran natively. This is not evidence of human usability.

**Result: met.** All five numbered items are met with passing library-driven evidence.

## Protocol checks

- Kit metadata equals the run's `conditions.env`: commit
  `32dd9c4f667b50cfbbef7e46b18c0007e2b08ee2`, dirty=0, version 0.3.88,
  `rustc 1.99.1 (58cf8f9c3 2026-10-13)`, prepared 2026-10-28T16:41:41+01:00.
  The sandbox reports cargo 1.99.1 / rustc 1.99.1, with `~/.cargo/config.toml` set to the
  vendored `/kit/vendor` source and offline mode (`evidence/01-build-and-features.log`).
- Session: brief 17:19:37, `ended=2026-10-28T17:22:23+01:00`, exit 0,
  outcome ended (2.8 min of the 90 min budget).
- Copy: `RUN_DIR/work` was copied to `ACCEPTANCE_DIR/work` with tar,
  excluding `target`, `.git`, `.claude` and `.codex`. The source had no symlinks and no
  `.git`/`.claude`/`.codex`; only `target` was excluded. No credentials or CLI homes
  were copied. Initial copy checksums equal the original
  (`evidence/original-work.sha256`, `evidence/copy-initial.sha256`).
- At the end, the original `RUN_DIR/work` (excluding target) still matches
  `evidence/original-work.sha256`. `evidence/copy-final.sha256` differs from
  the original only by the two added files (`tests/acceptance.rs`,
  `acceptance-captures/acceptance-states.png`). No application source,
  manifest or lockfile was edited. The tester's test rewrites `work/jobs.png` on every
  run. Its final bytes happen to equal the submitted file, which is also
  preserved as `evidence/jobs-submitted.png`.
- No protocol deviations observed. No test access or test entry point was
  added. The tests use the public `job_monitor::mount(ui, plans)` that the tester
  already exposed. The task allows its `Plan` list to replace the random source.
  Two tests pass no plans and exercise the production random path.

## Dependency features

`Cargo.toml`: `pert-ui = { version = "0.3.88", features = ["cpu"] }`, with defaults
on. `cargo tree -e features -i pert-ui` shows the features `cpu` and `default`
(→ `desktop` → `gpu` → `native`) enabled by `job-monitor`. `Cargo.lock` resolves
`pert-ui 0.3.88` from the (vendored) registry. No workspace path or other
library version is involved.

## Commands and results

| # | Command (inside `run.sh exec KIT ACCEPTANCE_DIR`) | Result | Log |
|---|---|---|---|
| 1 | `cargo build --offline --locked --all-targets` | builds, 0 warnings, exit 0 | `evidence/01-build-and-features.log` |
| 2 | `cargo tree --offline --locked -e features -i pert-ui` | `cpu` + `default` | same |
| 3 | `cargo test --offline --locked` (tester's suite, 3 runs) | `monitor_acceptance_and_cpu_capture` ok ×3, exit 0 | `evidence/02-tester-tests.log` |
| 4 | `cargo test --offline --locked --test acceptance -- --test-threads=1 --nocapture` (attempt 1) | 5 passed, 1 failed: **verifier error** (my expected summary miscounted failed plans: 3 vs 2). The app's displayed `Done: 7 … Failed: 2` was correct for the plans | `evidence/03-acceptance-tests-attempt1.log` |
| 5 | same, after fixing only that expected string, 3 runs | 6 passed ×3, exit 0 | `evidence/04-acceptance-tests.log` |
| 6 | `cargo test --offline --locked -- --test-threads=1` (final full suite: tester's and verifier's) | 7 passed (6 acceptance + 1 tester), exit 0 | `evidence/05-final-full-suite.log` |

`--test-threads=1` is needed for the verifier's thread-count check in
`/proc/self/task`, which must not see other tests' workers.

## Added verifier tests and captures

- Test source: `work/tests/acceptance.rs`, written by the verifier.
  It uses only `Driver::click`, `click_item`, `advance`, `settle`, `find`,
  `address_children`, `semantics`, `quiet_for`, `Runtime::set_profiling`/
  `profile`, and `CpuRenderer::serve_captures`. It never reads the app's model
  (`Monitor::jobs()`).
- Captures: `work/acceptance-captures/acceptance-states.png` (copy in
  `evidence/acceptance-states.png`, 1000×520) from the verifier test.
  `evidence/jobs-tester-test-rerun.png` and `evidence/jobs-final-suite.png`
  were rewritten by the tester's test. `evidence/jobs-submitted.png` is the submitted file.
- Command-output log: `evidence/0*.log` (command lines are prefixed `$`).

## Source review (frozen task-5 checks)

- **Cross-thread facility, no shared UI objects.** `src/lib.rs:260-281`
  creates a `ui.mailbox(Capacity::new(4096), handler)` (`pert_ui::live::Mailbox`).
  The worker thread (`lib.rs:166-211`) captures only a `UiSender<Update>`, an
  `mpsc::Receiver<()>` for cancellation, the `Plan`, a seed and the id. It
  sends owned `Update` values and handles `SendError::Full` by retrying and
  `Closed` by exiting. All UI state (`Rc<RefCell<Model>>`, `Signal`s,
  `TableController`) is `!Send` and is touched only in the mailbox handler and
  in button and sort callbacks on the UI thread. The compiler enforces this for
  the `thread::spawn` closure.
- **Simulation.** Size `rng.range(50, 500)` (`lib.rs:147`). Steps are
  `rng.range(5, 20)` per tick (`lib.rs:174`). One tick is
  `recv_timeout(100 ms)` = 10 ticks/s (`lib.rs:171`). Names are `Job {id}`
  (`lib.rs:38-40`). Every fifth job (`id % 5 == 0`) gets a random failure point
  in `1..total-1` (`lib.rs:151`).
- **Cancellation.** The Cancel button sends on the job's channel
  (`lib.rs:95-102`). The worker's `recv_timeout` returns at once, the worker sends a
  `Cancelled` update and returns (`lib.rs:172, 207-209`).
- **Failure.** Row state text `failed: simulated error` (`lib.rs:24`). The
  handler sets the `notice` signal to
  `Notification: Job N failed — simulated error at x / y units` (`lib.rs:270-277`).
  The signal is bound to the label `notification`, a direct child of the monitor
  column below the table (`lib.rs:298`). This is outside the row, which qualifies
  under the recorded 2026-10-27 rule.
- **Sorting.** Header clicks call `on_sort` → `publish()`. Rows are re-sorted
  by the controller's `sort()` signal by name (string), progress fraction,
  state text or elapsed time, ties by id, and reversed for descending
  (`lib.rs:124-141, 294-297`). The `action` column is not sortable
  (`lib.rs:245`).
- **Summary.** Counts of the four states (`lib.rs:118-123`).
- **Idle.** No timer or interval exists; the UI updates only when mailbox
  messages arrive. Workers stop sending on any terminal state.

## Per item

### 1. Headless check: start jobs, see progress, completion and summary update — **met**

- Tester: `tests/headless.rs:46-90` clicks **Start job** 5× via
  `Driver::click` and reads the summary, row percent (`!= "0%"`, then
  `"100%"`) and states from `Driver::semantics`. Its wait conditions use the
  model accessor `monitor.jobs()`, but every assertion uses inspection.
- Verifier: `item1_progress_completion_summary` checks the summary after each click
  (Running 0 → 1 → 2) and an intermediate percent strictly between 0 and
  100 % (observed 10 %). It then checks completion (`done`, 100 %, Cancel action gone),
  displayed elapsed 1.0 s for 10 ticks (10 ticks/s), the summary
  `Running: 1 Done: 1 …`, the running job's elapsed advancing, and that
  cancelling updates the summary. `item1_production_random_job_completes` checks that a
  random-plan job (no plans) completes at 100 % within the bounds of its
  random size and step. Passed in all runs.

### 2. Headless check: cancel a running job and see it stop — **met**

- Tester: `tests/headless.rs:62-77` clicks the row's Cancel button and
  waits ≤250 ms for the model state. It then checks that the row shows `cancelled`
  and that the model's processed count stays fixed. The app's handler drops updates
  for non-running jobs (`lib.rs:264`), so this alone does not prove that the worker stopped.
- Verifier: `item2_cancel_stops_job_and_worker` clicks Cancel through
  `Driver::click`. The row shows `cancelled` within 5–15 ms of wall time across runs,
  the Cancel action disappears and the summary shows Cancelled 1. The OS thread count
  (`/proc/self/task`) rises by one when the job starts and returns to its baseline
  after the cancel, so the worker thread exits. After 600 ms of wall time the row's
  displayed values are unchanged and `quiet_for(1 s)` is idle.

### 3. Headless check: failure shown in its row and as a notification — **met**

- Tester: `tests/headless.rs:83-86` checks row 5 state `failed: simulated error`
  and that the `notification` label contains `Job 5 failed`.
- Verifier: in `item3_failure_in_row_and_notification`, the notification label is
  empty before the failure. After it, row 2 shows `failed: simulated error` at its
  failure point (45 %) with no Cancel. The label `notification` reads
  `Notification: Job 2 failed — simulated error at 45 / 100 units`.
  `address_children(root)` shows it as a sibling of `jobs` (outside the
  table). The summary shows Failed 1, and the notice persists while another job keeps
  running. `item3_production_fifth_job_fails` (random plans) checks that jobs 1–4 finish
  and job 5 fails at a random point below 100 % with the notification. Both
  captures show the failure in its row and the notification line below the table.

### 4. Sorting by each column orders the rows correctly — **met** (with observation)

- Tester: `tests/headless.rs:92-126` clicks all four sortable headers through
  `Driver::click_item` (pointer input) in both directions and compares row
  keys from `address_children`. Its expected order is computed from the model with
  the same comparator the app uses, so the check is tautological about
  correctness.
- Verifier: `item4_sorting_item5_capture_and_idle` uses 11 jobs in all four
  states (so names reach two digits). It clicks every sortable header
  twice through `click_item`, reads every row's displayed cells from inspection, and
  checks that the displayed values are monotonic: name (string), progress
  (displayed %), state (text) and elapsed (displayed s), ascending on the first click
  and descending on the second. It also checks that a later tick keeps the order. Passed in all runs.
- **Observation (for reviewer attention):** name sorting is plain string
  order. Ascending gives `Job 1, Job 10, Job 11, Job 2, … Job 9`
  (`evidence/04-acceptance-tests.log`, "name order"). I graded this as correct
  ordering of the text column, because the task does not require natural or
  numeric ordering. A user would probably expect `Job 2` before `Job 10`. State sorts
  alphabetically (`cancelled < done < failed… < running`).

### 5. CPU capture (PNG) of the table with jobs in several states — **met**

- Tester: `tests/headless.rs:127-137` enables capture support, requests a
  window capture, serves it with `CpuRenderer::serve_captures` and writes
  `/work/jobs.png` (1000×460). Viewed (`evidence/jobs-submitted.png` and
  reruns): running, done ×2, failed and cancelled rows, the summary, the sort indicator
  on Elapsed and the notification line.
- Verifier: `work/acceptance-captures/acceptance-states.png` (1000×520) was
  made through the same CPU capture path. Viewed: 11 rows with running, done,
  cancelled and two failed jobs, plus the summary `Running: 1 Done: 7 Cancelled: 1 Failed: 2`
  and the notification. The rows inspected at capture time are logged.

### Idle (task feature and frozen check): no repaint while no job runs

- Tester: `quiet_for(1 s).is_idle()` before jobs and `quiet_for(2 s)` after
  all jobs end (`tests/headless.rs:45, 149`).
- Verifier: idle before jobs. **Positive control**: with jobs running (after
  250 ms of wall time), `quiet_for(100 ms)` is *not* idle (15–25 components updated,
  `needs_paint: true`). After all jobs end plus 400 ms of wall time,
  `quiet_for(2 s)` gives `components_updated: 0, next_deadline: None,
  needs_paint: false`. With profiling on, a further `advance(2 s)` records
  only the driver's own single `Flush` and no `Update`, `Layout`, `Paint` or
  `Compose`. This is as far as the library's inspection and profiling show; a native
  frame loop was not observed.

## Missing evidence and limits

- No native window or event-loop run, and no human usability claim.
- `quiet_for` uses runtime time, while workers run in wall time. The verifier
  added wall-time sleeps before idle checks, but the idle evidence remains the
  library's headless report, not native frame counting.
- "One job in five fails" is implemented as every fifth job
  (deterministic position, random point), not a random 20 % chance. I accepted this
  as a reading of the requirement.
- The notification is a persistent plain label that shows only the most recent
  failure. This qualifies under the recorded rule. Discoverability (no toast, no
  history) is a usability observation, not a defect.

## Application vs verifier issues

- Application failures: none found.
- Verifier-test errors: one (attempt 1, wrong expected count in my own
  test). It was fixed in the test only, and every later run passed.
- Library observations: none blocking. The tester reported one compile
  friction: an `App::new` initializer error-type mismatch, resolved by the compiler hint.
