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

### 19:27 — discover — annoyance

- **Category:** discover
- **Severity:** annoyance
- **Tried:** reading the docs in `/kit/doc` without a browser.
- **Expected:** a text form of the guide.
- **Resolved by:** a small script that cuts rustdoc's `main-content` section and pipes it through `pandoc -t plain`. Works well; the guide is in module pages (`pert_ui/guide/*/index.html`).
- **Read:** `pert_ui/guide/index.html`.

### 19:36 (time not checked when written; I had estimated 19:40, corrected at 19:36) — understand — annoyance

- **Category:** understand
- **Severity:** annoyance
- **Tried:** choosing a feature set. The task says "`cpu` in addition to the defaults".
- **Expected:** —
- **Resolved by:** README "Cargo features" table is clear: `features = ["cpu"]` keeps `desktop`. First `cargo build` succeeded in 20 s. No friction, noted for completeness.
- **Read:** `/kit/readme/pert-ui.md`.

### 19:36 (time not checked when written; I had estimated 19:45, corrected at 19:36) — understand — slow

- **Category:** understand
- **Severity:** slow
- **Tried:** finding how a table cell can show a `ProgressBar`, since `ProgressBar::new` needs a `ReadSignal<ProgressStatus>` and `TableItems::cell` "gets no state owner".
- **Expected:** a constant binding, or a progress cell type.
- **Resolved by:** `Signal::sibling` docs explain the per-row binding; I chose instead to create one `Signal<ProgressStatus>` per job on the UI thread when the job starts (the Start button's `EventContext` is a `StateOwner`) and keep it in the job record. Took reading `TableItems::cell`, `Signal::sibling`, `StateOwner` and the state guide's "Choosing a way" table together.
- **Read:** `struct.ProgressBar`, `trait.TableItems`, `struct.Signal` (`sibling`), `trait.StateOwner`, `guide/state`.

### 19:36 (time not checked when written; I had estimated 19:47, corrected at 19:36) — understand — annoyance

- **Category:** understand
- **Severity:** annoyance
- **Tried:** reconciling "UI must not repaint while idle" with toasts.
- **Expected:** —
- **Resolved by:** `Toast` docs: a toast expires after 5 s of runtime time, so a failure notification necessarily schedules a deadline until it expires. `ProgressBar` docs show `Runtime::next_deadline()` as the idleness check; I will assert `next_deadline() == None` after toasts expire. Clear docs, just a design consideration.
- **Read:** `struct.Toast`, `struct.ProgressBar`, `guide/surfaces` (Notifications).

### 19:36 (time not checked when written; I had estimated 20:02, corrected at 19:36) — runtime — slow

- **Category:** runtime
- **Severity:** slow
- **Tried:** a progress cell `Row::new([ProgressBar::new(status).with_grow(1), Label::new("10%")])` in a flex column about 230 px wide.
- **Expected:** the grown bar takes what the label leaves, since `ProgressBar` "fills the width it is given".
- **Resolved by:** debug validation printed `[pert-ui validation ERROR EscapesParent] Component "percent" (#17): visible bounds leave its parent "progress" without a clip`. The bar's *preferred* width (three ControlWidths) is not shrunk by `Row`; the layout guide says a bounded Row "does not shrink their widths", but `ProgressBar` and `Row` pages don't connect this. Swapping the order moved the error to "bar". Fixed by wrapping the bar in `Container::new(bar).preferred_size(Size::new(24.0, 6.0)?)` with `with_grow(1)`. The validation message itself was good: it named the component and the parent.
- **Read:** `struct.Row`, `struct.ProgressBar`, `struct.Container`, `struct.SizeLimits`, `guide/layout` (Growing and stretching).

### 19:36 (time not checked when written; I had estimated 20:03, corrected at 19:36) — understand — annoyance

- **Category:** understand
- **Severity:** annoyance
- **Tried:** reading a table's rows in a test.
- **Expected:** some guesswork about the internal tree.
- **Resolved by:** a recursive dump with `Runtime::children`/`key`/`query(Selector::id(..)).detail(Detail::Semantics)` showed `jobs/body/<row key>/<column key>/<cell element>` and the semantic names. Worked the first time; the `Table` page does not state this path, which a tester needs (the list-row path *is* documented in the testing guide).
- **Read:** `guide/testing` (Reading what the runtime shows, List rows), `struct.Runtime` (`children`, `key`, `query`).

### 19:36 (time not checked when written; I had estimated 20:15, corrected at 19:36) — compile — annoyance

- **Category:** compile
- **Severity:** annoyance
- **Tried:** a test helper returning `Result<Semantics, _>` with only `use pert_ui::prelude::*; use pert_ui::testing::*;`, which the testing guide says is "one use block" that covers a test.
- **Expected:** `Semantics`, the return type of `Driver::semantics`, to be in `testing` (the guide lists `SemanticValue` there).
- **Resolved by:** `use pert_ui::Semantics;`.
- **Diagnostic:** `error[E0425]: cannot find type `Semantics` in this scope --> tests/headless.rs:77:83`
- **Read:** `guide/testing` intro.

### 19:36 (time not checked when written; I had estimated 20:17, corrected at 19:36) — understand — slow

- **Category:** understand
- **Severity:** slow
- **Tried:** `driver.find(&root, &["content", "jobs", "body"])`, after my tree dump (via `Runtime::children` + `Runtime::key`) printed `app / content / content / toolbar` and `jobs / body / job1 / state / state`.
- **Expected:** keys printed by `Runtime::key` to be path segments.
- **Resolved by:** trial and error: the table's `body` and the ToastHost's inner `content` are internal and add no segment; rows are `["content", "jobs", "job1", "state", "state"]`. The testing guide states this rule for `Each` rows, but the `Table` page doesn't, and `Runtime::key` returns the internal keys anyway, so the dump misleads. `TestError::NotFound { path }` doesn't say which segment failed.
- **Read:** `guide/testing` (List rows), `struct.Table`, `struct.Runtime` (`find`, `key`, `children`).

### 19:36 (time not checked when written; I had estimated 20:22, corrected at 19:36) — missing — slow

- **Category:** missing
- **Severity:** slow
- **Tried:** reading the text of a visible toast in a headless test, through `Driver::semantics` on the `toasts` part and through `InspectionQuery ... .detail(Detail::Full)`.
- **Expected:** each toast as a `SemanticItem` or child with its message as the name, as for other text a user sees.
- **Resolved by:** inspection gives only `value: Number(1.0)` (visible count) and properties `visible_count`, `paused`, `dismiss_label`, `scroll_offset`. Workaround: the app keeps each posted message with its `ToastReceipt`; the test checks the stack's visible count, `ToastStatus::Visible` and the stored message. The CPU capture shows the text visually.
- **Read:** `struct.ToastHost`, `struct.Toast`, `struct.ToastReceipt`, `enum.ToastStatus`, `enum.Detail`, `enum.DrawCommand`, `guide/surfaces` (Notifications).

### 19:36 (time not checked when written; I had estimated 20:24, corrected at 19:36) — source — annoyance

- **Category:** source
- **Severity:** annoyance
- **Tried:** reading `/kit/vendor/pert-ui-runtime-0.3.75/src/notifications.rs` (`ToastHost`'s `Component::inspect`).
- **Expected:** to confirm whether toast text appears anywhere in inspection, because the docs only mention the `dismiss_label` property and the docs can't prove a negative.
- **Resolved by:** confirmed: `inspect` reports only the count, offset, pause state and dismiss label.
- **Read:** source as above.

### 19:36 (time not checked when written; I had estimated 20:27, corrected at 19:36) — runtime — annoyance

- **Category:** runtime
- **Severity:** annoyance
- **Tried:** `Table::new(&controller, 28.0)` with a `Button` in a cell.
- **Expected:** a validation warning if the button doesn't fit the fixed row height, like the `EscapesParent` one for the progress row.
- **Resolved by:** the capture showed the Cancel button sticking out of its row; no validation output. Raised the row height to 36. A note on `Table::new` saying what row height fits a standard control (ControlHeight plus padding) would help.
- **Read:** `struct.Table` (`new`).

### 19:38 — understand — annoyance

- **Category:** understand
- **Severity:** annoyance
- **Tried:** checking "the UI must not repaint while no job is running" headlessly.
- **Expected:** a documented "is idle" query on `Runtime`.
- **Resolved by:** combining `FlushReport { components_updated, next_deadline }` with `HostIntegration::needs_paint()` (in the `host` module, a custom-host trait) after `Runtime::paint`. Found by grepping the docs for `needs_paint`; the testing guide could name this check, since "nothing scheduled while idle" is a selling point of the README.
- **Read:** `struct.FlushReport`, `host/trait.HostIntegration` (`needs_paint`), `struct.ProgressBar` (its `next_deadline` example).

## Closing

- **Result:** All five acceptance items are met, each checked by a headless test in `tests/headless.rs` (`cargo test`): (1) `jobs_progress_complete_and_update_the_summary`: progress %, bar value, elapsed, completion and the summary line, then an idle check (no updates, no deadline, `needs_paint()` false, also after advancing 30 s). (2) `a_cancelled_job_stops`: Cancel shows `cancelled` at once, the worker thread ends with the clock stopped, and no more units are processed after 5 more ticks. (3) `a_failure_shows_in_its_row_and_as_a_notification`: row text `failed: …`, the bar reports `failed`/`invalid`, the toast stack shows 1 toast, the posted message is checked through its receipt, and it expires after 5 s. (4) `sorting_by_each_column_orders_the_rows`: real header clicks via `pointer_input`, ascending and descending for name, progress, state (lifecycle order running < done < cancelled < failed) and elapsed, with the order holding while a job progresses. (5) `cpu_capture_of_the_table` writes `/work/capture.png` with jobs in all four states and the failure toast. `tests/real_clock.rs` also runs the actual `App` via `App::headless` with the wall clock and random source, and checks the random distribution (50–500 units, about 1 in 5 fails). Workers hold only a `UiSender<Report>`, an `Arc<AtomicBool>` and the clock. Not verified: a native window (no display; `cargo run` exits with winit's "neither WAYLAND_DISPLAY nor … DISPLAY is set").
- **Concepts I had to learn:** `Runtime` and `mount_tree` with keyed `Element`s; `Signal`/`ReadSignal`, and that a flush shows writes; `Mailbox`/`UiSender` for worker threads (handler runs at the start of a flush, may write signals); `StateOwner` (an `EventContext` can create signals); `TableController` + `TableSource`/`TableItems` and the rule that the app sorts its data in `on_sort` and republishes with `set_source`; cells get no state owner, so per-row bindings live in the app's records; `ProgressStatus` written by the app; `ToastController`/`ToastHost`/`ToastReceipt`; `Driver` (settle, wait_until, advance, find by key path), and that internal parts add no path segment; inspection queries and `Semantics`; `needs_paint`/`next_deadline` for idleness; capture with `CpuRenderer::serve_captures` and `encode_png`.
- **What worked well:** The guide gets you very far: the testing chapter had almost exactly the harness I needed (wait_until for background work, the capture recipe), and the state chapter's "Choosing a way" tables pointed straight to `Runtime::mailbox`. Item pages are precise about timing and errors. The first headless run worked; `Mailbox` → signal writes → `set_source` from the handler just worked. The debug validation message for the overflowing progress row named the exact component and parent. `Driver` plus a manual clock gave fully deterministic tests with real threads (stable across 18 repeated runs). The CPU capture looks like a finished app with no styling work.
- **The three changes I want most:** (1) Make toast text (and any other text a user sees) readable through inspection, e.g. each toast as a `SemanticItem` with its message. (2) Document the key-path rules for composite widgets on their own pages (`Table` rows are `[table, row, column, cell]`, `body` and ToastHost's `content` are skipped), and have `Runtime::key`/`children` or `TestError::NotFound` make the skipped segments visible. (3) Make content fit its slot: a `Row` that can shrink a child's preferred width (or a note on `ProgressBar` that it won't shrink), and validation, or a documented recommended height, when a control is taller than a `Table`'s fixed row. Smaller: put `Semantics` in `testing`, and document an "is the UI idle" check in the testing guide.
