# Friction log

Copy this file to `/work/FRICTION.md`. Add one entry per event, in time
order, as it happens. When something costs you time, run `date +%H:%M` at
that moment and write the entry before you continue; do not estimate a
time. If you do write an entry later, give it the time you write it and add
`(written later)` after the time.

## Entries

Note: the six entries marked 19:20 were each written when the event happened, between 19:11 and 19:20, but I first stamped them with estimated times instead of running `date`. I corrected them to the time of the correction.

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

### 19:20 (written later) — docs-wrong — annoyance

- **Category:** docs-wrong
- **Severity:** annoyance
- **Tried:** Read the guide's "Run your first window" to learn how to add the dependency.
- **Expected:** `pert-ui = "0.3.52"` from the registry, as the README's feature table implies (`pert-ui = { version = "…", features = [...] }` appears in the diagnostics chapter).
- **Resolved by:** Ignoring the guide and using the registry version from the brief. The getting-started chapter says "Until a registry release exists, use a local checkout" with a `path = "../pert-ui/crates/pert-ui"` dependency, which contradicts the docs.rs-style rendering and the other chapter.
- **Read:** `pert_ui/guide/getting_started/index.html`, `pert_ui/guide/diagnostics/index.html` (Serialization).
- **Diagnostic:** n/a

### 19:20 (written later) — runtime — annoyance

- **Category:** runtime
- **Severity:** annoyance
- **Tried:** A table cell holding `Row::new([ProgressBar (with_grow(1)), Label "42%"])` in a 240 px fixed column.
- **Expected:** The grown bar to shrink so the bar and its percentage fit the cell.
- **Resolved by:** Widening the column. The validation message was clear (`[pert-ui validation ERROR EscapesParent] Component "percent" (#16): visible bounds leave its parent "progress" without a clip; ...`), and `Element::with_grow` does say "Rows never shrink a child", but nothing on `ProgressBar` or `Table` says that a bar's preferred width (three ControlWidths, 192 px) plus a label will overflow a typical column. A way to let a bar take "whatever is left" in a Row would help.
- **Read:** `struct.ProgressBar.html`, `struct.Element.html#method.with_grow`, `struct.Row.html`.
- **Diagnostic:** n/a (stderr validation report during `cargo test`)

### 19:20 (written later) — understand — annoyance

- **Category:** understand
- **Severity:** annoyance
- **Tried:** `ui.find_descendant(&row, "state")` to reach the label I keyed `"state"` inside the `state` column's cell.
- **Expected:** My label, since `TableItems::cell` says "the element's own key is free, since the table wraps each cell".
- **Resolved by:** The call returned `RuntimeError::AmbiguousKey`. The table's cell wrapper seems to use the column key, so "free" really means "free, but don't reuse the column key if you want to find it". I renamed my cell keys (`state-label`, ...).
- **Read:** `trait.TableItems.html` (`cell`), `struct.Runtime.html#method.find_descendant`.
- **Diagnostic:** test error `Error: AmbiguousKey`

### 19:20 (written later) — missing — slow

- **Category:** missing
- **Severity:** slow
- **Tried:** To check headlessly that the failure notification shows the right text, I searched `ui.inspect().components` for a semantics name containing the message, as I did for labels. I printed the inspection of a minimal `ToastHost` with one toast in a throwaway example.
- **Expected:** Each visible toast to appear as a component or as a `SemanticItem` carrying its message, as labels expose `semantics.name` and a `text` property.
- **Resolved by:** Partly unresolved. The `toasts` component reports `role: Notifications`, `value: 1`, `items: []` and properties `visible_count`, `paused`, `dismiss_label`, but never the message text. `DisplayList` commands carry no text either, and no doc page mentions how toast content can be inspected (grep over every page that mentions "toast"). The test now checks `ToastReceipt::status() == Visible`, the `visible_count` property and the message the application posted, and I checked the text by looking at a CPU capture.
- **Read:** `struct.ToastHost.html`, `struct.ToastQueue.html`, `struct.ToastReceipt.html`, `struct.Toast.html`, `enum.DrawCommand.html`, `struct.DisplayList.html`, guide `surfaces` (Notifications), guide `diagnostics` (Inspection).
- **Diagnostic:** n/a

### 19:20 (written later) — missing — annoyance

- **Category:** missing
- **Severity:** annoyance
- **Tried:** Showing a cancelled or failed job's bar at the point where it stopped, in the Muted or Error color.
- **Expected:** A terminal status that keeps its fraction, e.g. `Failed(NormalizedValue)`.
- **Resolved by:** Unresolved. `ProgressStatus::Failed` and `Cancelled` are documented as "a full track in the Error/Muted role", so a job cancelled at 8% shows a full grey bar next to my "8%" label (visible in `capture-window.png`). The only alternative is `Determinate`, which draws in Primary, as if still running. I kept the library's statuses and rely on the percentage label.
- **Read:** `enum.ProgressStatus.html`, `struct.ProgressBar.html`.
- **Diagnostic:** n/a

### 19:20 (written later) — workaround — annoyance

- **Category:** workaround
- **Severity:** annoyance
- **Tried:** Driving a sort the way a user does, by pressing a column title in a headless test.
- **Expected:** The header titles to be addressable (a key path or a component per title) so a test can find and click one, as it can a row or a button.
- **Resolved by:** The header is drawn by the `Table` itself ("drawn by the table"), so I computed the title's position from the table's `Runtime::bounds` plus my own fixed column widths and sent `PointerEvent`s there. This works only because all my columns have fixed widths. The keyboard route (Alt+Up/Down on an active cell) was the documented alternative. The sort itself worked first time: `on_sort` plus `TableController::set_source`.
- **Read:** `struct.Table.html`, `struct.TableController.html`, `struct.PointerEvent.html`, `enum.PointerKind.html`.
- **Diagnostic:** n/a

### 19:21 — docs-wrong — annoyance

- **Category:** docs-wrong
- **Severity:** annoyance
- **Tried:** Taking the capture reply on the UI thread after `CpuRenderer::serve_captures`, as the guide's Screenshots example does with `pending.wait()?`.
- **Expected:** One consistent rule.
- **Resolved by:** `Pending::wait` says "Never call this on the UI thread: the reply comes from that thread's next flush", but the guide (diagnostics, Screenshots) and `CpuRenderer::serve_captures` both call `pending.wait()` on the UI thread. That is safe there only because the reply has already arrived. I used `Pending::try_reply`, which states the non-blocking behavior. I also first requested a capture before `Runtime::set_capture_support(true)` and got `CaptureError::Unsupported` at once. That was my mistake (it is documented), but the error does not hint at the order.
- **Read:** `struct.Pending.html`, `cpu/struct.CpuRenderer.html`, guide `diagnostics` (Screenshots).
- **Diagnostic:** test error `Error: Unsupported`

## Closing

- **Result:** All five acceptance items are met and checked headlessly by `cargo test` (`tests/monitor.rs`, 6 tests, stable over 8 consecutive runs):
  1. `starts_jobs_and_shows_progress_completion_and_summary`: jobs start through the "Start job" button (key input), progress and percentages appear, jobs complete, and the summary line updates. When idle, `next_deadline() == None` and `needs_paint() == false`.
  2. `cancel_stops_a_running_job`: the row's Cancel button is clicked with pointer events. The job is cancelled, processes no further unit, its worker thread ends and the Cancel action disappears. `cancel_is_prompt_with_the_real_clock` shows that, with the real pacer, the cancelled state reaches the UI within 50 ms.
  3. `failure_shows_in_row_and_notification`: the row's state reads "failed: simulated error at unit 35" (through inspection), and the toast is Visible with `visible_count` 1. Its text is checked only as what the app posted, plus visually in `capture-failure.png` (see the "missing" entry). With no job running, the only remaining deadline is the toast's expiry, and after it nothing is scheduled.
  4. `sorting_by_each_column_orders_rows`: pressing each column title (pointer events) sorts ascending, then descending. The order is read from the mounted rows' on-screen y positions, for name, progress, state and elapsed. The order stays correct as progress changes under an active sort.
  5. `captures_the_table_as_png`: `capture-table.png` (the table component) and `capture-window.png` (the window) show jobs running, done, cancelled and failed, rendered by `CpuRenderer::serve_captures`.

  The same binary also has `cargo run -- --headless out.png`, a real-time session with random plans that writes a CPU capture. The native window could not be run here (no display: `EventLoop("... neither WAYLAND_DISPLAY nor WAYLAND_SOCKET nor DISPLAY is set.")`). Workers only send `JobEvent`s through a `UiSender`. A test may replace the random source (`Env::planner`) and the clock (`Env::pacers`).
- **Concepts I had to learn:** `Runtime` as both the app's and the test's host (`App::new` hands the same `&mut Runtime`); keyed `Element`s; `Signal`/`ReadSignal` bindings and `Label::new(signal.read_only())`; `Runtime::mailbox`/`UiSender` for worker results, with `keep_alive` for the mailbox; `flush(now)` as the deterministic clock, with `next_deadline`/`needs_paint` for idleness; `Table` + `TableController` + `TableItems`/`TableSource` with application-side sorting through `on_sort` and `set_source`; `ProgressBar`/`ProgressStatus`; `ToastQueue`/`ToastHost`/`ToastReceipt`; `Element::with_grow` for a bounded collection; `find_descendant`/`children`/`bounds`/`inspect` for headless checks; `set_capture_support` + `request_capture` + `CpuRenderer::serve_captures`.
- **What worked well:** The API reference is unusually complete and precise, and almost every item has a runnable example. The headless `Runtime` is the same object the `App` uses, so the tests drive the real UI with real key and pointer input and an explicit clock. Mailboxes made the "no UI objects on worker threads" rule natural. Idleness is directly testable (`next_deadline() == None`). Development validation reported my layout overflow with the exact component and parent. The CPU capture produced a PNG in a few lines. The whole application compiled on the first attempt and 5 of 6 tests passed on their first run.
- **The three changes I want most:**
  1. Expose toast messages (and other surface content) in inspection, as `SemanticItem`s or child components, so a headless test can assert what a notification says.
  2. Let terminal `ProgressStatus` values keep their fraction (e.g. `Failed(NormalizedValue)`), so a job cancelled or failed partway does not draw a full bar.
  3. Make table header titles addressable for tests (a key path per column title, or a `TableController` method that performs the same sort request as a header press). Also fix the getting-started chapter's "use a local checkout" dependency instructions and the `Pending::wait` / guide-example contradiction.
