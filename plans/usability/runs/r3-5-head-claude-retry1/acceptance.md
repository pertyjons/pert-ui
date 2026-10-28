# Acceptance: r3-5-head-claude-retry1 (task 5, background job monitor)

Independent acceptance by a fresh Claude verifier session (claude-opus-5-5),
2026-10-28, under `plans/usability/acceptance.md` (task 5 paragraph, with
the 2026-10-27 outside-row notification rule) and the round 3 rules.
Headless AI evidence only: this report claims neither native execution nor
human usability.

**Verdict: met.** All five numbered items are met. Each has passing,
library-driven evidence: the tester's checks and independent verifier
checks that never read the application's model.

## Protocol

- Kit metadata: `kit-r3-head/kit.env` and the run's `conditions.env` agree on
  commit `32dd9c4f667b50cfbbef7e46b18c0007e2b08ee2`, `dirty=0`, version
  `0.3.88`, toolchain `rustc 1.99.1 (58cf8f9c3 2026-10-13)` and prepared time
  `2026-10-28T16:41:41+01:00`. The sandbox reports the same rustc and cargo 1.99.1
  (`evidence/01-toolchain-features.log`).
- Session: brief delivered 17:57:32, ended `2026-10-28T18:04:10+01:00`,
  `outcome=ended`, exit 0 (6.6 min of a 90 min budget).
- Copy: `RUN_DIR/work` was copied to `work/` with rsync, leaving out
  `target`, `.git`, `.claude` and `.codex`. The source had no symlinks.
  No credentials or CLI homes were copied. All 8 copied files match
  `original-source-manifest.json`, both at copy time and after the final
  suite. The original also still matches (`evidence/copy-sha256.txt`,
  `evidence/07-hash-check.log`).
- All builds and tests ran through
  `run.sh exec /home/per/pert-ui-usability/kit-r3-head <ACCEPTANCE_DIR> …`.
  Cargo used the sandbox home's `/kit/vendor` offline source replacement,
  not workspace paths or an installed library.
- Features: `Cargo.toml` declares `pert-ui = { version = "0.3.88", features = ["cpu"] }`.
  `cargo tree -e features -i pert-ui` shows `cpu` and `default`, with
  `desktop`, `gpu` and `native` coming in through default.
- No application code was changed. The only additions are two test files
  and the captures they write (listed below). The tester's
  `cpu_capture_of_jobs_in_several_states` test rewrites `work/capture.png`
  with byte-identical content (SHA-256 `b4537dd6…`, the same as submitted).
- **Protocol deviations: none observed.**

## Build and test commands

| # | Command (inside `run.sh exec KIT ACC`) | Result | Log |
|---|---|---|---|
| 1 | `cargo --version; rustc --version`; `cargo tree --locked -e features -i pert-ui` | ok; `cpu` and defaults | `evidence/01-toolchain-features.log` |
| 2 | `cargo build --locked --all-targets` | ok, no warnings | `evidence/02-build-and-tester-tests.log` |
| 3 | `cargo test --locked` (tester's suite, default parallel) | 8/8 passed (`tests/monitor.rs`) | `evidence/02-build-and-tester-tests.log` |
| 4 | `cargo test --locked --test acceptance_probe -- --nocapture` | ok; inspection dump | `evidence/03-probe.log` |
| 5 | `cargo test --locked --test acceptance -- --test-threads=1 --nocapture`, attempt 1 | 7/8; **verifier-test error**, see below | `evidence/04-acceptance-attempt1.log` |
| 6 | same, attempt 2 after correcting the verifier assertion | 8/8 passed | `evidence/05-acceptance-attempt2.log` |
| 7 | final: `cargo test --locked --all-targets -- --test-threads=1`; acceptance binary repeated 3×; tester binary in parallel | all passed: 8 + 1 + 8; repeats 8/8 ×3 | `evidence/06-final-suite.log` |

Verifier-test error in attempt 1: `item1_…` asserted that a done job's
ProgressBar reports fraction 1.0. By design the library's
`ProgressStatus::Succeeded` reports no fraction, only the description
`"succeeded"` (`pert-ui-runtime-0.3.88/src/progress.rs:146-151, 470-493`). The
assertion was corrected to read the bar's status description. This was not
an application defect. The corrected test also checks the "cancelled" and
"failed" bar statuses.

The acceptance binary uses `--test-threads=1` because it observes worker
threads by OS thread name (`job-N`), and parallel tests in one process would
reuse those names.

## Added verifier test sources and captures (acceptance copy only)

- `work/tests/acceptance.rs`: independent acceptance tests. Input goes
  through `Driver::click` (Start job, Cancel) and `Driver::click_item` (a
  pointer press on the Table header items). State is read through
  `Runtime::inspect()`: component paths, semantics names and values, and
  border boxes for the drawn row order. Idleness comes from
  `Driver::quiet_for` and `Runtime::flush` reports, and the capture from
  `CpuRenderer`. The tests never call `Monitor::jobs()` or read model
  fields. The clock and planner are replaced through the public `Env`, as
  the task allows. A `Drop` on the test harness advances the manual clock so
  that parked workers exit. This is test-only and does not change app
  behavior.
- `work/tests/acceptance_probe.rs`: an inspection dump used to ground the
  inspection paths (its output is `evidence/03-probe.log`).
- Captures: `work/acceptance-capture.png` (copy:
  `evidence/acceptance-capture.png`, SHA-256 `5fdd827f…`), 800×360 RGBA from
  `CpuRenderer`. The tester's capture as regenerated is
  `evidence/tester-test-capture.png`, and as submitted
  `evidence/submitted-capture.png` (identical).

No test entry point or test access was added to the application.

## Constraints from the task-specific paragraph

- **Cross-thread facility, no shared UI objects.** Workers get only
  `UiSender<Report>` (from `Runtime::mailbox`), `Arc<dyn Clock>`, an
  `Arc<AtomicBool>` cancel flag and a `JobPlan` (`src/lib.rs:202-249,
  382-386`). The mailbox handler applies reports and publishes on the UI
  thread (`src/lib.rs:536-541`). UI objects (`Rc`, signals, controllers)
  stay on the UI thread; `thread::spawn`'s `Send` bound would reject them.
  **Met.**
- **Cancellation stops the worker.** See item 2. The OS thread `job-N`
  disappears.
- **Failure in the row and as a notification.** See item 3. A `ToastStack`
  with role `Notifications` sits outside the table.
- **Sorting by every column.** See item 4.
- **No repaint while idle.** No timers or intervals exist in the app. The
  only repaint triggers are button handlers, sort callbacks and mailbox
  reports (`src/lib.rs:536-570`). Evidence through the library:
  - The tester's `quiet_for(1 s).is_idle()` holds before any job and after
    all jobs finish (`tests/monitor.rs:117,146`). With only a toast's expiry
    pending, `components_updated == 0` and `!needs_paint`
    (`tests/monitor.rs:199-201`). The wall-clock app check is at
    `tests/monitor.rs:313`.
  - Verifier `idle_check_detects_jobs_and_is_quiet_without_them` includes a
    positive control. While a real wall-clock job runs, flush reports show
    updates (25 components updated, paint needed 6 times in 500 ms). After
    it finishes, 44–49 consecutive 20 ms rounds over about 1 s of wall time
    each show `components_updated == 0`, `!needs_paint` and
    `quiet_for(100 ms).is_idle()`.
  - The cancel tests also show the window is idle once the last worker has
    ended.

  **Met** as far as the library's inspection and profiling show. Source
  observation, not a defect: after a cancel, the worker's final `Cancelled`
  report arrives at most one tick later and republishes the table once.

## Per-item results

### 1. Headless check: start, progress, completion, summary update. **Met**

- Code: `Model::start_job`, `apply`, `publish` and `summary`
  (`src/lib.rs:322-331, 365-440`). Columns are name, progress (ProgressBar and
  percentage Label), state and elapsed (`src/lib.rs:454-487, 519-530`).
- Tester: `starts_jobs_and_shows_progress_completion_and_summary`
  (`tests/monitor.rs:112-149`). It clicks Start, then reads row keys,
  percent, state, elapsed and summary through `Driver::semantics` and
  `address_children`. It synchronizes on `Monitor::jobs()`, a model read.
- Verifier: `item1_progress_completion_and_summary_through_inspection`
  (`tests/acceptance.rs`). It checks:
  - three jobs, one started later, named Job 1 to Job 3;
  - percent labels and ProgressIndicator values (0.333, 0.2), and bar status
    determinate, then succeeded;
  - elapsed 0.1 s, 0.3 s and 0.5 s;
  - the Cancel action present only while running;
  - every summary transition, from 0/0/0/0 through Running 2, then
    Running 1 · Done 1, to Done 3.

  It is synchronized only on inspection. Passed.

### 2. Headless check: cancel a running job and see it stop. **Met**

- Code: the Cancel button exists only for running rows
  (`src/lib.rs:465-474`). `Model::cancel` sets the flag and state
  (`src/lib.rs:389-398`). The worker checks the flag at each tick and exits
  (`src/lib.rs:229-234`). Later reports cannot change a stopped row
  (`src/lib.rs:407-410`).
- Tester: `cancelling_a_running_job_stops_it` (`tests/monitor.rs:151-179`).
  It clicks Cancel, then reads state, summary, the vanished Cancel and the
  frozen percent and elapsed through inspection. Its "stopped" flag and
  `done` assertions read the model.
- Verifier:
  - `item2_cancel_stops_the_worker_thread_and_the_row`: OS thread `job-1` is
    alive before Cancel and gone after the next tick, while `job-2` keeps
    running. Over the following ticks the row stays at 20%, bar 0.2, status
    "cancelled" and 0.2 s, while job 2 moves to 50%. Cancelling the last job
    also ends its thread, and the window is then idle.
  - `item2_cancel_is_prompt_on_the_wall_clock`: the real `app()` on
    `SystemClock` through `HeadlessApp`. The worker thread exited about
    91 ms after the Cancel click (limit 400 ms; tick is 100 ms). The row
    stays at 3% across 300 ms more, and the summary reads Cancelled 1.

  Passed.

### 3. Headless check: failure shown in its row and as a notification. **Met**

- Code: the worker sends `Outcome::Failed`. `apply` sets
  `JobState::Failed(reason)`, whose state cell shows
  `failed: <reason>`, and pushes a `Toast` with "Job N failed: …"
  (`src/lib.rs:236-241, 413-425`). `ToastHost` wraps the content
  (`src/lib.rs:571-587`). The failure rate is 1 in 5 (`src/lib.rs:141`).
  The tester's statistical check of that rate tests a model function
  (`tests/monitor.rs:318-333`); it is not acceptance evidence and none is
  needed for this item.
- Tester: `a_failure_shows_in_its_row_and_as_a_notification`
  (`tests/monitor.rs:181-203`). The row state text, percent and summary are
  read through inspection. The toast messages are read from the `toasts`
  stack's semantic items.
- Verifier: `item3_failure_in_row_and_notification`. Inspection of
  `["monitor","toasts"]` reports role `Notifications`, count 0 before the
  failure and 1, then 2, after it. The items name both failures exactly.
  The rows show `failed: I/O error at unit 20/30`, with the bar
  status "failed" and fraction kept, and no Cancel. The summary reads
  Failed 2, the failed worker threads exit, and an unaffected job still
  completes. The toast is outside the table subtree, and both captures show
  it with the actual message. Passed.

### 4. Sorting by each column orders the rows correctly. **Met**

- Code: four sortable `TableColumn`s, with `Table::on_sort` storing the
  `TableSort` and republishing (`src/lib.rs:523-526, 560-564`).
  `sort_jobs` (`src/lib.rs:334-347`) works as follows:
  - name sorts by job number, so Job 10 comes after Job 9;
  - progress sorts by fraction;
  - state sorts in the task's listed order: running, done, cancelled,
    failed;
  - elapsed sorts by duration;
  - descending reverses the order, and ties keep job order.
- Tester: `sorting_by_each_column_orders_the_rows` and
  `sorting_by_name_is_numeric` (`tests/monitor.rs:232-274`). They press the
  headers with `Driver::click_item` and check both directions through
  `address_children`.
- Verifier: `item4_sorting_by_each_column_orders_drawn_rows` and
  `item4_name_sort_is_by_job_number_beyond_nine`. Five jobs have distinct
  progress (6/20/30/50/100%) and elapsed (0.1 to 0.6 s) values. Each header
  is pressed from a scrambled starting order, so a no-op would fail. The
  order is read as drawn, from inspection border-box y positions.
  Ascending and descending are checked for every column. Descending state
  groups failed, cancelled, done, running. The order holds while a job
  keeps reporting. With 12 jobs the name order is numeric in both
  directions. Passed.
- Interpretation: the task names no state order. The app uses the task's
  listing order, and an alphabetical order would equally have been
  accepted.

### 5. CPU capture (PNG) of the table with jobs in several states. **Met**

- Tester: `cpu_capture_of_jobs_in_several_states` (`tests/monitor.rs:276-296`)
  writes `capture.png` through `CpuRenderer::serve_captures` at 800×360. The
  deterministic regeneration is byte-identical to the submission. Visually
  it shows done, running, cancelled and failed rows with bars,
  percentages, the summary line and a failure toast.
- Verifier: `item5_cpu_capture_png_of_several_states` writes
  `acceptance-capture.png`, 800×360 with a PNG signature. The same
  inspection shows rows in all four states. Visually it has the
  progress-sort indicator, five rows in all four states and two failure
  toasts. Passed.

## Aggregate

All five items are met, so `acceptance=met`, and the session end time is
recorded (`ended=2026-10-28T18:04:10+01:00`).

## Observations (not defects of the graded items)

- Usability: the toasts overlay the lower right of the table and can cover
  row cells, such as Job 2's elapsed cell in `evidence/acceptance-capture.png`.
- A failure may be scheduled on a job's last tick (`fail_at_tick` ranges
  over 1 to `steps.len()`), so a job can fail at 100% progress. That is still
  "a random point".
- The tester's tests synchronize on the public `Monitor::jobs()` model
  snapshot, and some assertions read it. The verifier tests do neither.
- Library: for this item there was no library defect. The attempt-1
  failure came from verifier expectations about `Succeeded` progress semantics
  and is documented, not an application or library failure.
