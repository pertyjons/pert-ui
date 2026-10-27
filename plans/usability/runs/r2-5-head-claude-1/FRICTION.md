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

### My entries

### 18:40 — understand — annoyance

- **Category:** understand
- **Severity:** annoyance
- **Tried:** planning the progress column of the job table: a `ProgressBar` inside `TableItems::cell`. `ProgressBar::new` takes `impl Into<ReadSignal<ProgressStatus>>`; `TableItems::cell` "gets no state owner", so a cell cannot create a signal.
- **Expected:** a constant binding (e.g. `ReadSignal::constant(v)` or `From<T> for ReadSignal<T>`) for a value that the row description already carries, like `Checkbox::controlled` does for checkboxes.
- **Resolved by:** creating one `Signal<ProgressStatus>` per job on the UI thread when the job starts (the Start button's `EventContext` is a `StateOwner`) and keeping it in the job record. Works, but it is a second source of truth next to the row data I republish with `set_source`.
- **Read:** `struct.ProgressBar.html`, `struct.ReadSignal.html` (only `From<Signal<T>>` / `From<&ReadSignal<T>>` impls), `trait.TableItems.html` (`cell`), `trait.StateOwner.html`, guide `layout` § "An editable list of records".

### 18:44 — missing — slow

- **Category:** missing
- **Severity:** slow
- **Tried:** acceptance 3 needs a headless check that a failure is shown "as a notification". I pushed `Toast::new("Job 2 failed at 35%: …")` and queried `Selector::descendants(["monitor","toasts"])` with `Detail::Semantics`: zero components. The `toasts` component itself has `role: Notifications`, `value: Number(1.0)`, `items: []`, `name: None`; with `Detail::Full` its properties are only `visible_count`, `scroll_offset`, `paused`, `dismiss_label`, `presentation_layer`, `anchor_owner`. `Runtime::paint()` gives a `DisplayList` of vectors/masks with no text either.
- **Expected:** the testing guide says "reads what it shows through inspection"; I expected each visible toast's message (and action label) as a semantic item or child, like table cells, which are fully addressable.
- **Resolved by:** workaround: the application keeps each failure message next to its `ToastReceipt`; the test checks the message, `receipt.status() == ToastStatus::Visible` and `visible_count == 1` on the stack. It proves that a toast is shown, not which text it shows.
- **Read:** `struct.ToastHost.html`, `struct.ToastController.html`, `struct.Toast.html`, `struct.ToastReceipt.html`, `struct.ComponentInspection.html`, `struct.ComponentDetails.html`, `enum.Detail.html`, `enum.DrawCommand.html`, `Runtime::content_search` (searches semantics only), guide `surfaces` § Notifications, guide `testing`.

### 18:45 — source — annoyance

- **Category:** source
- **Severity:** annoyance
- **Tried:** read `/kit/vendor/pert-ui-runtime-0.3.75/src/notifications.rs` (`ToastHost`'s `inspect`, around line 808) to make sure there was no hidden way to read a visible toast's message in inspection.
- **Expected:** the docs to state what a `ToastHost` reports to inspection (as `ProgressBar`'s page does: "Inspection reports the ProgressIndicator role, the fraction as its value…"). `ToastHost`'s page only mentions the `dismiss_label` property.
- **Resolved by:** the source confirms only the count is reported; see the entry above.
- **Read:** source as above.

### 18:45 — understand — annoyance

- **Category:** understand
- **Severity:** annoyance
- **Tried:** `Table::new(&controller, 28.0)` with a `Button` ("Cancel") in one column. The first CPU capture showed each Cancel button taller than its row, overlapping the row below (no error and no validation issue: `validation_report()` was empty).
- **Expected:** either a row height that follows the theme (like everything else, which is metric-driven), or the `Table::new` docs saying that the row height must fit the cells' controls and pointing to `MetricRole::ControlHeight`.
- **Resolved by:** `ui.theme().metrics(theme.density()?)?.metric(MetricRole::ControlHeight) + 6.0` at build time. It is fixed after that, so a later density change will not resize rows.
- **Read:** `struct.Table.html` (`Table::new`), `struct.ThemeManager.html`, `struct.ThemeMetrics.html`.

### 18:45 — understand — annoyance

- **Category:** understand
- **Severity:** annoyance
- **Tried:** showing a failed or cancelled job's bar with `ProgressStatus::Failed` / `Cancelled`.
- **Expected:** the bar to keep showing how far the job got, in the error/muted color.
- **Resolved by:** accepted as documented ("a full track in the Error role"); the percentage label next to the bar keeps the real value ("15%" next to a full red bar). There is no status that has both a fraction and an outcome. Documented clearly, so this is a design gap rather than a docs problem.
- **Read:** `enum.ProgressStatus.html`, `struct.ProgressBar.html`.

## Closing

- **Result:** all five acceptance items are met and checked headlessly; `cargo test` runs 9 tests and all pass, and stayed green across 5 repeated runs.
  1. `tests/monitor.rs::jobs_progress_complete_and_update_the_summary`: clicks **Start job** twice, moves a `ManualClock` tick by tick, and reads the percentage labels, the bar's semantic value, elapsed time, state, the disappearance of Cancel and the summary line through inspection.
  2. `a_cancelled_job_stops`: clicks the row's Cancel, sees "cancelled", sees the worker thread end without the clock moving, then advances the clock 20 ticks and sees the row stay at 15%. `tests/app.rs` repeats this on the real `App` (`App::headless`) with the wall clock: the worker stops in under 50 ms.
  3. `a_failure_shows_in_its_row_and_as_a_notification`: the row shows "failed: I/O error at unit 35" at 35% and the bar reports `invalid`. The toast stack shows 1 notification, and the app's receipt for the failure message is `Visible`. Because of the gap in the 18:44 entry, the test cannot read the toast's text through the library.
  4. `sorting_orders_rows_by_each_column` sorts ascending and descending through the table's own Alt+Up/Down path on every sortable column and reads the row order from the rows' geometry; `a_header_click_sorts_and_flips` does it with pointer presses on the Name title.
  5. `cpu_capture_of_the_table` writes `captures/jobs.png` (900×420) with `CpuRenderer`: rows in done, failed, cancelled and running states, the sort arrow and the failure toast.
  - "No repaint while no job runs": `nothing_is_scheduled_while_no_job_runs` checks `next_deadline == None` and `components_updated == 0` before any job and after all jobs end. The one exception is a failure toast, whose 5 s expiry is a library deadline.
  - The native window itself was never opened (no display).
- **Concepts I had to learn:** `Runtime` / `App::new` initializer → `Element` keys and key paths → `Signal` / `ReadSignal` and flushes → `Runtime::mailbox` + `UiSender` for worker threads (and `keep_alive` for the mailbox) → `TableController` / `TableItems` / `TableSource` with `set_source` (the table never sorts: `on_sort` + republish) → `StateOwner` (an `EventContext` can create signals) → `ProgressStatus` / `ProgressBar` → `ToastController` / `ToastHost` / `ToastReceipt` → testing: `Driver` (`click`, `advance`, `wait_until`, `chord`), `Runtime::query` with `Selector` / `Detail`, the runtime clock vs. my own clock → `ThemeMetrics` / `MetricRole` for sizes → capture: `set_capture_support`, `request_capture`, `CpuRenderer::serve_captures`, `FrameImage::encode_png`.
- **What worked well:** the guide's state chapter ("which way to choose") pointed straight at `Runtime::mailbox` for long-lived workers, and the testing chapter had a copyable example for every step I needed, including the CPU capture. Inspection is excellent for tables: every cell is addressable by key path (`monitor/content/jobs/job-2/state/state`) with its text as the semantic name, and the progress bar reports its fraction and `invalid`. My first build compiled without errors, `validation_report()` was empty, and `FlushReport` made the "no repaint when idle" requirement directly testable. The table's sorting contract (indication only; the app sorts and republishes) is stated clearly and was easy to follow.
- **The three changes I want most:**
  1. Report each visible toast's message (and action label) in inspection, as semantic items or child components, so a test can check *what* notification is shown, not only how many.
  2. A constant `ReadSignal` (or controlled forms for `ProgressBar`, as `Checkbox::controlled` has) so `TableItems::cell` can show a progress bar from the row data, without per-job signals kept beside the republished rows.
  3. A table row height that can follow the theme (e.g. `Table::new_auto` / a `MetricRole`-based default), or at least a validation issue when a cell's control is taller than the row; and a `ProgressStatus` that keeps the fraction when a task fails or is cancelled.
