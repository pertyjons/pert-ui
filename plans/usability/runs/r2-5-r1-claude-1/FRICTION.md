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

### 18:49 — docs-wrong — annoyance

- **Category:** docs-wrong
- **Severity:** annoyance
- **Tried:** following the guide's "Run your first window" to add the dependency.
- **Expected:** a `pert-ui = "0.3.52"` line, since the crate is on a registry.
- **Resolved by:** ignoring the guide and using `pert-ui = { version = "0.3.52", features = ["cpu"] }` as the brief says. The guide says "Until a registry release exists, use a local checkout" and shows a `path = "../pert-ui/crates/pert-ui"` dependency, and it tells me to run `cargo run -p pert-ui --example ...` from a checkout. A registry user has neither. The examples it points to (dynamic_documents, live_mixer, log_explorer) are only links into the repository, which I cannot open offline.
- **Read:** `pert_ui/guide/getting_started/index.html`, `/kit/readme/pert-ui.md`.

### 18:49 — understand — annoyance

- **Category:** understand
- **Severity:** annoyance
- **Tried:** finding a way to give `ProgressBar` a constant per-row value inside a table cell that is rebuilt from a data snapshot.
- **Expected:** `ProgressBar::new(ProgressStatus)` or `From<T> for ReadSignal<T>`, the way `Label::new` takes a plain `String`.
- **Resolved by:** `ProgressBar::new` only takes `impl Into<ReadSignal<ProgressStatus>>`, and `ReadSignal` has no constant constructor. I keep one `Signal<ProgressStatus>` per job and create it with `EventContext::signal` (from `StateOwner`) in the Start button's callback. `Signal::sibling` would work too.
- **Read:** `struct.ProgressBar.html`, `struct.ReadSignal.html` (From impls), `trait.StateOwner.html`, `struct.Signal.html`.

### 18:53 — runtime — annoyance

- **Category:** runtime
- **Severity:** annoyance
- **Tried:** a table cell built as `Row::new([ProgressBar.key("bar").with_grow(1), Label("42%")])` in a 230 px column.
- **Expected:** the bar takes the space the percentage label leaves.
- **Resolved by:** development validation printed `[pert-ui validation ERROR EscapesParent] Component "percent" (#16): visible bounds leave its parent "progress" without a clip`. That message is good. The cause was spread over two pages: `ProgressBar` prefers "three themed ControlWidths" (`struct.ProgressBar.html`), and "Rows never shrink a child" sits at the very end of `Element::with_grow`. I wrapped the bar in `Container::new(..).preferred_size(Size::new(170.0, 8.0))`. A way to let one Row child shrink to the leftover width would be the natural fix.
- **Read:** `struct.ProgressBar.html`, `struct.Element.html#method.with_grow`, `struct.Row.html`, `struct.Container.html`.

### 18:53 — compile — annoyance

- **Category:** compile
- **Severity:** annoyance
- **Tried:** `b.x + b.width / 2.0` on the `Rect` returned by `Runtime::bounds`; its Debug output is `Rect([0.0, 0.0, 80.0, 32.0])`.
- **Expected:** public fields like `Size`/`Point` in other crates.
- **Resolved by:** `error[E0615]: attempted to take value of method x on type Rect`. The compiler suggested `x()`/`width()`, which worked.
- **Diagnostic:** `error[E0615]: attempted to take value of method `x` on type `Rect``

### 18:53 — understand — annoyance (positive note)

- **Category:** understand
- **Severity:** annoyance
- **Tried:** finding how to address a Cancel button inside a table row for a headless click.
- **Expected:** nothing in the Table page says what key path a cell gets.
- **Resolved by:** I dumped `ui.inspect().components` paths. They are `["monitor","content","jobs","job-1","action","cancel"]` (table key, row key, column key, cell element key), and `Runtime::find` takes the part below the root. Once dumped this was easy, but the Table page could state the address of a cell.
- **Read:** `struct.Runtime.html#method.find`, `struct.Table.html`, `struct.ComponentInspection.html`.

### 18:55 — missing — slow

- **Category:** missing
- **Severity:** slow
- **Tried:** checking headlessly that a failure toast shows its message. I looked for a component under `["monitor","toasts"]` whose `Semantics::name` is the message, then tried `Runtime::content_search(ContentQuery::new("checksum"))`.
- **Expected:** inspection to report the visible toasts' text, as the Label in the row does. The guide's diagnostics chapter presents inspection as the way tools see what the app shows.
- **Resolved by:** workaround. The ToastHost inspects as one `SemanticRole::Notifications` component with `value: Number(1.0)`, `actions: [Dismiss]`, `items: []` and properties `visible_count`, `dismiss_label`, `paused` and so on. No message text appears. Content search found only the row label. I now keep each `ToastReceipt` with the message I pushed, the test asserts `ToastStatus::Visible`, and the CPU capture shows the text. A test cannot read back what a toast says through the library.
- **Read:** `struct.ToastHost.html`, `struct.ToastQueue.html`, `struct.ToastReceipt.html`, `struct.Runtime.html#method.content_search`, `struct.ContentHit.html`, `struct.Semantics.html`.

## Closing

- **Result:** All five acceptance items are met, and `cargo test` checks them headlessly (`tests/acceptance.rs`, 6 tests plus 2 unit tests in `src/sim.rs`; stable over 15 repeated runs):
  1. `jobs_progress_complete_and_update_the_summary` clicks Start job through `Runtime::pointer_input`. It reads the shown percentages, states, elapsed times and the summary Label through inspection (`Semantics::name`) as the jobs tick.
  2. `a_cancelled_job_stops_promptly` clicks a row's Cancel button. The job reports `cancelled` without another clock tick, then stays at 15% / 0.3 s while the other job goes on. The unit test `real_time_cancellation_is_prompt` checks the real clock stops in under 20 ms.
  3. `a_failure_shows_in_its_row_and_as_a_notification` sees the row read "failed: checksum error at unit 40", the toast receipt `Visible` and the host's `visible_count` at 1. The toast text itself cannot be read back (see 18:55).
  4. `every_column_sorts_both_ways` clicks each header title twice and checks the on-screen row order (by row bounds) and the header's sort indication, ascending then descending, for name, progress, state and elapsed.
  5. `capture_of_jobs_in_several_states` writes `captures/jobs.png` (done, running, failed, cancelled, plus the failure toast) through `Runtime::paint` and `CpuRenderer::render`. `cargo run -- --headless captures/headless.png` runs a seeded random session and writes another capture.

  The idle requirement is checked by `nothing_is_scheduled_while_no_job_runs`: `next_deadline() == None` and `!needs_paint()` before any job and after the jobs end and the toast expires. The native window could not be run (no display: winit's `neither WAYLAND_DISPLAY nor WAYLAND_SOCKET nor DISPLAY is set`), so the windowed path is only compiled, not seen.
  I did not read any library source under `/kit/vendor`.
- **Concepts I had to learn:** `Runtime` with an explicit elapsed clock (`flush(now)`, `next_deadline`); `Element` keys and `IntoElement::key`; signals (`Signal`/`ReadSignal`, `StateOwner::signal` from an `EventContext`); `Runtime::mailbox` and `UiSender` for worker reports, and `keep_alive` for the mailbox in an `App`; `TableController` + `TableItems` + `TableSource`, with the application sorting in `on_sort` and republishing through `set_source`; `ProgressStatus`/`ProgressBar`; `ToastQueue`/`ToastHost`/`ToastReceipt`; `Element::with_grow`, `Container::preferred_size` and the rule that rows never shrink; key paths for `Runtime::find` and inspection; `pointer_input` for headless clicks; `paint` + `CpuRenderer` for captures; development validation messages.
- **What worked well:** The headless story is excellent. A plain `Runtime` plus `set_viewport`, `pointer_input`, `find`, `bounds` and `inspect` let me click real buttons and header titles and read what the UI shows, with no test-only hooks in the app. The CPU capture worked first time. The mailbox contract (owned values, `SendError` returns the value, delivered at the start of a flush, handler may write signals) is precise and was exactly what a worker-progress app needs. The Table's "the application sorts its own data in `on_sort`" model is simple and well explained. Validation's EscapesParent message caught a real layout bug. Most of my code compiled on the first attempt, because the reference pages give full signatures and runnable examples.
- **The three changes I want most:**
  1. Let inspection (or content search) report the text of visible toasts, so a test can check what a notification says rather than only that one is visible.
  2. Fix the getting-started dependency instructions for registry users (version dependency, no `-p pert-ui --example` from a checkout), and ship the referenced examples (`live_mixer`, `dynamic_documents`, `log_explorer`) in the rendered docs or the package. They are only repository links now.
  3. A way for one `Row` child to shrink to the space its siblings leave (or a ProgressBar that prefers less than its allocation), plus a sentence on the Table page giving the key path of a cell (`table/row-key/column-key/cell-key`).
