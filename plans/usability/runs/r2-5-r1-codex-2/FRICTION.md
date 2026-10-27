# Friction log

Copy this file to `/work/FRICTION.md`. Add one entry per event, in time
order, as it happens. When something costs you time, run `date +%H:%M` at
that moment and write the entry before you continue; do not estimate a
time. If you do write an entry later, give it the time you write it and add
`(written later)` after the time.

## Entries

### 18:52 — discover — annoyance

- **Tried:** Started with guide/getting_started and looked for the table row type (guessed `struct.TableRow.html`, which does not exist).
- **Expected:** A registry dependency example and a row data type.
- **Resolved by:** Used the brief's registry dependency instruction; `Table`'s example explains `TableItems` and `TableSource` instead of a row type.
- **Read:** `guide/getting_started/index.html`, `struct.Table.html`, `guide/layout/index.html`.
- **Diagnostic:** Documentation helper reported `FileNotFoundError: ... struct.TableRow.html`; no library/compiler failure.


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

### 18:56 — workaround — annoyance

- **Tried:** Wrote final run instructions and removed temporary documentation extracts in one shell call.
- **Expected:** Clean up only files created during this task.
- **Resolved by:** The environment rejected `rm -f` before executing the command. Wrote the README separately and deleted the extracts with `apply_patch` instead. This is tooling friction, not a pert-ui problem.
- **Read:** No library documentation.
- **Diagnostic:** `rm -f style commands are not permitted. Use a safer approach`.

## Closing

- **Result:** All five acceptance items met. `cargo test --offline` passed (1 test); `cargo run --offline -- --headless` passed. The check activates Start/Cancel through Runtime keyboard input, observes mailbox-driven progress, completion, summary changes, row failure text and notification text, sorts all four columns both ways through the table's keyboard callback and checks independent expected row orders. Cancelled workers exit within 250 ms. Future idle flushes update zero components and request no paint/deadline. CPU capture `jobs.png` shows running, done, cancelled and failed rows together. Native mode compiles but was not displayed (no display available).
- **Concepts I had to learn:** Stable Element keys and composition; Signal/ReadSignal; retained Runtime flush/deadlines; bounded Mailbox and Send-only UiSender; TableItems/TableSource and TableController; bounded grown collection layout; determinate ProgressStatus; semantic inspection and WidgetId lookup; keyboard sorting; CPU capture serving.
- **What worked well:** The crate-root map points to the right guide chapters. The state guide clearly separates UI state from worker messages and explains mailbox ownership. Table examples compile with the facade imports. The layout guide explicitly explains how to bound a table under a toolbar. ProgressBar documents that determinate values schedule no animation. The diagnostics capture recipe works headlessly without another image dependency. No library source reads were necessary; no library-specific compiler errors or runtime surprises occurred.
- **The three changes I want most:** (1) Add a registry dependency example alongside the checkout-only getting-started instructions. (2) Add a small background-job example combining mailbox ownership, cancellation, table source refresh and shutdown. (3) Add a compact headless interaction recipe that finds keyed controls, activates them, inspects displayed semantics and verifies idle scheduling in one place.
