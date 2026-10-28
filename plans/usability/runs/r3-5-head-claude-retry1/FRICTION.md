# Friction log

Copy this file to `/work/FRICTION.md`. Add one entry per event, in time
order, as it happens. When something costs you time, run `date +%H:%M` at
that moment and write the entry before you continue; do not estimate a
time. If you do write an entry later, give it the time you write it and add
`(written later)` after the time.

## Entries

### <time> — <category> — <severity>

- **Category:** one of
  - *discover*: could not find how to do it;
  - *understand*: found it, but it was unclear how it behaves;
  - *compile*: a compiler diagnostic that did not point to the fix;
  - *runtime*: behavior differed from the documentation or your expectation;
  - *docs-wrong*: the documentation is incorrect;
  - *missing*: there seems to be no way to do it;
  - *workaround*: you did it in a way you think the library did not intend;
  - *source*: you read the library's source (say why).
- **Severity:** *blocker* (you could not continue without it), *slow* (it
  cost noticeable time), *annoyance*.
- **Tried:** what you did.
- **Expected:** what you expected to happen or to find.
- **Resolved by:** what resolved it, or "unresolved".
- **Read:** the documentation pages or items you read for it.
- **Diagnostic:** for *compile*, the exact error code and the first lines.

### 18:01 — compile — annoyance

- **Category:** compile
- **Severity:** annoyance
- **Tried:** wrote `use pert_ui::prelude::*;` and used `UiSender`, `SendError`, `Mailbox` and `Capacity`, which the state guide ("Worker results and live data") and `Runtime::mailbox` present as the way for a long-lived worker to report.
- **Expected:** the names the guide uses for background work next to `Runtime::spawn` would come from the prelude, or the guide section would show its `use` line (its spawn example does not need them, the mailbox part has no example).
- **Resolved by:** `use pert_ui::{live::*, prelude::*};`. The crate index does say `live` is "for data from other threads", and `pert_ui::live` lists the mailbox family. Fixed in a minute.
- **Read:** guide/state (Worker results and live data), `Runtime::mailbox`, `Runtime::keep_alive` (its example imports `pert_ui::{Capacity, Runtime}`), `pert_ui::live`.
- **Diagnostic:** `error[E0425]: cannot find type `UiSender` in this scope`, `error[E0433]: cannot find type `SendError` in this scope` (rustc suggested `std::sync::mpsc::SendError`, which is the wrong type).

### 18:01 — compile — annoyance

- **Category:** compile
- **Severity:** annoyance
- **Tried:** `RuntimeError::from(ApplicationError::new(format!("missing {path:?}")))` to report a missing widget from my `build` function.
- **Expected:** a way to make an `ApplicationError` from a message.
- **Resolved by:** wrapping the message in `std::io::Error::other(...)`. `ApplicationError` only has `new(impl Error + Send + Sync)`, `get` and `downcast_ref`.
- **Read:** `struct.ApplicationError.html`.
- **Diagnostic:** `error[E0277]: the trait bound `String: std::error::Error` is not satisfied` … `required by a bound in pert_ui::ApplicationError::new`.

### 18:03 — workaround — annoyance (written later; happened while writing `build`, between 18:01 and 18:03)

- **Category:** workaround
- **Severity:** annoyance
- **Tried:** creating the mailbox whose handler applies reports to my model, where the model itself must hold the mailbox's `UiSender` to hand to each new job thread.
- **Expected:** a pattern in the guide for "a long-lived set of workers whose handler writes an application model that also starts the workers". The state guide's mailbox paragraph has no example. The only mailbox example is under `Runtime::keep_alive`, and it writes one signal.
- **Resolved by:** an `Rc<RefCell<Option<Rc<RefCell<Model>>>>>` slot captured by the handler and filled after the model is built (plus `Rc::new_cyclic` so cell buttons can reach the model). Works, but it is plumbing I wrote myself.
- **Read:** guide/state, `Runtime::mailbox`, `Mailbox`, `UiSender`.

### 18:03 — understand — annoyance (written later; design decision made between 18:01 and 18:03)

- **Category:** understand
- **Severity:** annoyance
- **Tried:** deciding how a table row should follow a job that reports 10 times a second.
- **Expected:** either a per-row binding or a clear statement that replacing the whole source is the normal path for live rows.
- **Resolved by:** `TableItems::cell` does say that it is "called again for every mounted row each time the table describes its rows, on a source replacement", and that a display control in a cell "takes the record's value itself". `ProgressBar::new(status)` per cell, with `TableController::set_source` on every report, works. Rows keep identity by key, so sorting and the Cancel button survive. I am not sure it is the intended cost model for many live rows, but the docs pointed there.
- **Read:** `trait.TableItems.html` (`cell`), `struct.TableController.html` (`set_source`), guide/state "Choosing a way" table, guide/testing "Tables" (its job/progress example was almost exactly my table).

### 18:03 — understand — annoyance

- **Category:** understand
- **Severity:** annoyance
- **Tried:** a Cancel `Button` inside a table cell whose callback calls `TableController::set_source` (replacing the very row that holds the button) and writes a signal.
- **Expected:** a statement that this is allowed from a cell's own callback, or a `BorrowConflict`. The `set_source` errors list `BorrowConflict` without saying when it happens.
- **Resolved by:** trying it. It works; the button disappears with the next description of the row. Side effect seen in the capture: the press also selects the row (documented under `Table`: "a press on a cell sets its column as well as its row"), so the cancelled row stays highlighted.
- **Read:** `TableController::set_source`, `Table` (description of presses on cells).

### 18:03 — runtime — annoyance (positive note, no difficulty)

- **Category:** runtime
- **Severity:** annoyance (no actual difficulty; recorded for completeness)
- **Tried:** the first full test run: start, progress, completion, summary, cancel, failure with a toast, sorting by every column through `Driver::click_item`, `quiet_for`, the CPU capture and `App::headless`.
- **Expected:** a round of debugging.
- **Resolved by:** all eight tests passed on the first run and stayed stable over six reruns. The guide's testing chapter (tables, toasts, background work, quiet window, capture) gave every call I needed. I did not read `/kit/vendor` at any point.
- **Read:** guide/testing, `ToastHost` (Inspection and tests), `testing::Driver`, `testing::QuietReport`.

## Closing

- **Result:** all five acceptance items are met; `cargo test` runs 8 headless tests in `tests/monitor.rs`:
  1. `starts_jobs_and_shows_progress_completion_and_summary`: clicks Start job twice, ticks a manual clock, and reads the percent, state and elapsed cells and the summary label through semantics. It also checks, with `Driver::quiet_for`, that the window is idle before any job and after all jobs end.
  2. `cancelling_a_running_job_stops_it`: clicks the row's Cancel button. The row shows "cancelled", the Cancel action disappears, the worker sends its final report at the next tick (`Job::stopped`), and progress stays frozen while the other job keeps going.
  3. `a_failure_shows_in_its_row_and_as_a_notification`: the state cell shows "failed: …", the bar is in the Failed state, and the toast stack's semantic items carry the message. While the toast waits to expire, nothing is updated or painted.
  4. `sorting_by_each_column_orders_the_rows` (ascending and descending for name, progress, state and elapsed, via `Driver::click_item` on the column titles; the order holds while a job keeps reporting) and `sorting_by_name_is_numeric`.
  5. `cpu_capture_of_jobs_in_several_states` writes `/work/capture.png` (800×360: done, running with Cancel, cancelled, failed, plus the failure toast) and checks that the validation report is empty.
  Also `the_app_runs_jobs_on_the_wall_clock` (the real `App` builder via `App::headless`, with real threads and the system clock) and a statistical test of the random plans (sizes, rates, about 1 in 5 failing). The native window itself was never run (no display).
- **Concepts I had to learn:** `Runtime` and `mount_tree`/`Element` keys; `Signal` and derived labels; prelude vs thematic modules (`live` for mailboxes); `Runtime::mailbox`/`UiSender` for worker → UI; `TableController`/`TableSource`/`TableItems` with the application owning sort order (`on_sort` + `set_source`) and keyed rows; cell addresses `<table>/<row>/<column>/<content>`; `ProgressStatus`; `ToastController`/`ToastHost` and reading toasts as semantic items; the testing `Driver` (settle, click, click_item, wait_until, address_children, quiet_for); `App<State>` + `App::headless`; capture requests served by `CpuRenderer`.
- **What worked well:** the guide's testing chapter. Its Tables section is practically this task (job names, progress bar + percent cell, sortable titles clicked with `click_item`). The doc snippets I adapted (table, toast, capture, headless app) worked as written. Inspection by key paths made the tests read like the UI. The idle check (`quiet_for`) gave direct evidence for the "no repaint while idle" requirement. The library only enables, never forces, its own threads, so a fake clock and fixed plans were easy to inject.
- **The three changes I want most:**
  1. Put the mailbox family (`Mailbox`, `UiSender`, `SendError`, `Capacity`) in the prelude, or show the `use pert_ui::live::*` line in the state guide's mailbox paragraph, with a full worker → model example (including the "model owns the sender, handler owns the model" cycle).
  2. Document whether calling `TableController::set_source` from a callback inside one of the table's own cells is allowed, and what the expected update path for live rows is (whole-source replacement at 10 Hz vs. something per-row).
  3. `ApplicationError::msg(impl Into<String>)` or `From<String>` for quick application errors.
