# Friction log

## Entries

### 18:38 — understand — annoyance

- **Tried:** Read `ProgressBar::new` and `TableItems::cell` to put a progress bar in each table cell.
- **Expected:** A fixed or controlled progress value, like other row controls.
- **Resolved by:** The state guide explicitly recommends `Signal::sibling` for controls without a controlled form; use a sibling binding for each described bar.
- **Read:** `pert_ui/struct.ProgressBar.html`, `pert_ui/trait.TableItems.html`, `pert_ui/guide/state/index.html`, `pert_ui/struct.Signal.html`.

### 18:38 — understand — annoyance

- **Tried:** Read `Toast::new` and `ToastController::push` for failure notifications.
- **Expected:** A notification that can remain visible without scheduling idle work.
- **Resolved by:** Use a persistent in-layout notification instead; Toast has a default five-second lifetime and no lifetime builder in its documented API.
- **Read:** `pert_ui/struct.Toast.html`, `pert_ui/struct.ToastController.html`, `pert_ui/struct.Banner.html`.

### 18:39 — discover — annoyance

- **Tried:** Work out how an App initializer keeps the worker mailbox alive after returning.
- **Expected:** An application ownership hook.
- **Resolved by:** `Runtime::keep_alive` explicitly covers mailboxes in an initializer, including shutdown order.
- **Read:** `pert_ui/struct.Mailbox.html`, `pert_ui/struct.Runtime.html#method.keep_alive`.

### 18:41 — runtime — slow

- **Tried:** Put `ProgressBar` with `with_grow(1)` beside a percentage label in a table column with default width.
- **Expected:** The progress bar would shrink to leave space for the percentage.
- **Diagnostic:** `[pert-ui validation ERROR EscapesParent] Component "percent" (#17): visible bounds leave its parent "value" without a clip; correct the parent's arrangement, clip its children, or allow an intentional overlay: report OverlapPolicy::Allow in the component's ComponentDetails::validation.` Repeated for the other three rows.
- **Resolved by:** Stack the bar and percentage in a Column and increase row height to 64 pixels. The next run passed without validation messages; the PNG shows both fully within each cell. The layout guide says Row does not shrink child widths.
- **Read:** `pert_ui/struct.ProgressBar.html`, `pert_ui/guide/layout/index.html`.


## Closing

- **Result:** All five acceptance items met. The headless session clicks Start, observes progress/completion and summary semantics, clicks Cancel, observes failure in its row and persistent notification, exercises ascending and descending keyboard sorting for all four data columns and checks visible row geometry. `/work/job-monitor.png` is the CPU capture with running, done, cancelled and failed rows. After the final job completes, there are no deadlines and three one-second advances update zero components. Native presentation could not be checked without a display.
- **Concepts I had to learn:** Runtime and keyed Elements; Signals and derived bindings; mailbox/UiSender delivery; application-owned TableController sorting and TableItems sources; sibling bindings in table cells; keep_alive ownership; Driver, semantic/geometry inspection and CPU captures.
- **What worked well:** The guide directly matched worker updates, headless interactions and capture needs. Mailbox callbacks safely write UI signals. Table documentation explicitly says the application owns sorting. All application code compiled on the first build after consulting the API pages. No library source reads were needed.
- **The three changes I want most:** A controlled/fixed ProgressBar constructor for cells; a compact table example showing a bar plus percentage that fits a narrow column; a notification lifetime option allowing persistent toasts without idle deadlines.
