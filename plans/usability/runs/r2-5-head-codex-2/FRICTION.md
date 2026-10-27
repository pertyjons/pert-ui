# Friction log

## Entries

### 19:00 — discover — annoyance

- **Tried:** Looked for a collections guide and a `TableState` page while following the table API.
- **Expected:** A guide dedicated to collections and a state type similar to other widgets.
- **Resolved by:** Listing the rendered guide pages and reading `Table`'s example, which names `TableController` and `TableItems`; collections live in the layout guide.
- **Read:** `pert_ui/guide/index.html`, `pert_ui/guide/layout/index.html`, `pert_ui/struct.Table.html`.

### 19:00 — understand — annoyance

- **Tried:** Read `Toast` and `ToastHost` for failure notifications.
- **Expected:** A persistent notification without any future repaint after jobs stop.
- **Resolved by:** Using a persistent notification label instead: Toast has a five-second lifetime and an animated presentation, which would schedule work after the final job finishes.
- **Read:** `pert_ui/struct.Toast.html`, `pert_ui/struct.ToastHost.html`, `pert_ui/struct.ProgressBar.html`.

### 19:02 — compile — annoyance

- **Tried:** Called `Runtime::shutdown()?` at the end of the headless check, following the guide's `HeadlessApp::shutdown()?` example.
- **Expected:** The runtime shutdown to have the same fallible return convention.
- **Resolved by:** `Runtime::shutdown` returns `()`; removed `?`. This was my mistaken transfer between two different types, and the compiler pointed directly to it.
- **Read:** `pert_ui/guide/testing/index.html`, `pert_ui/struct.Runtime.html`.
- **Diagnostic:** `error[E0277]: the ? operator can only be applied to values that implement Try` at `d.runtime_mut().shutdown()?;`; `the ? operator cannot be applied to type ()`.

### 19:03 — runtime — slow

- **Tried:** Used `find_descendant(row, "state")` to assert a table cell's label.
- **Expected:** To find the application-owned label inside the cell.
- **Resolved by:** The table also creates a wrapper with the column's key. Giving the label a distinct `state-label` key avoids the ambiguous match. `TableItems` says the table wraps each cell, but does not state the wrapper's key; no source read was needed.
- **Read:** `pert_ui/trait.TableItems.html`, `pert_ui/struct.Runtime.html#method.find_descendant`.
- **Diagnostic:** `called Result::unwrap() on an Err value: AmbiguousKey("state")`.

### 19:03 — runtime — slow

- **Tried:** Put a growing ProgressBar and a percentage/units label in a Row in equally sized table columns.
- **Expected:** The progress cell would fit both children in its column.
- **Resolved by:** Allocating more width to Progress and simplifying the label to a percentage; the bar and text have natural minimum widths that exceeded the equal column allocation.
- **Read:** `pert_ui/struct.Row.html`, `pert_ui/struct.TableColumn.html`, `pert_ui/struct.ProgressBar.html`.
- **Diagnostic:** `[pert-ui validation ERROR EscapesParent] Component "percent" (#15): visible bounds leave its parent "progress" without a clip; correct the parent's arrangement, clip its children, or allow an intentional overlay: report OverlapPolicy::Allow in the component's ComponentDetails::validation.`

### 19:03 — runtime — annoyance

- **Tried:** Waited for Job 1 to finish and Job 5 to fail, then asserted a summary expecting Job 3 to have finished too.
- **Expected:** Equal-length jobs to complete together.
- **Resolved by:** Waiting explicitly for Job 3 as well. Independent thread scheduling and mailbox delivery make this test assumption invalid; this was application-test friction, not a library defect.
- **Read:** `pert_ui/guide/testing/index.html` (Background work), `pert_ui/struct.Runtime.html#method.mailbox`.
- **Diagnostic:** Summary assertion: left `Some("Running: 2   Done: 1   Cancelled: 1   Failed: 1")`, right `Some("Running: 1   Done: 2   Cancelled: 1   Failed: 1")`.

## Closing

- **Result:** All five acceptance items met. `cargo test --offline` and `cargo run --offline -- --headless` drive the real Runtime using Driver, keyboard/button input, mailbox delivery and semantic inspection. Checks cover progress, completion, counts, prompt cancellation with no further progress, failure row and persistent notification, ascending and descending mounted row order for every sortable column, and no idle deadline or repaint. CPU capture `jobs.png` shows all four states. Native window execution was not tested because there is no display.
- **Concepts I had to learn:** Runtime and keyed Elements; signals and derived updates; mailbox ownership and keep_alive; deferred runtime work from EventContext; TableItems/TableSource and TableController; determinate ProgressStatus; Driver settling versus background wait_until; semantic inspection; CPU capture support; host needs_paint and deadlines.
- **What worked well:** The crate overview quickly linked the relevant guide chapters. The state and testing examples mapped directly to worker delivery and headless verification. The first application build needed no library API corrections. Table sorting callbacks, keyed row retention and CPU capture worked as documented. No library source was read.
- **The three changes I want most:** Document generated table cell wrapper keys with a lookup example; include a progress-bar-plus-percentage table example with practical column sizing; offer or document a persistent notification option that schedules no idle animation or expiry.
