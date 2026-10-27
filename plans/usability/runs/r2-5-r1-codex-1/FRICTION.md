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

### 18:42 — understand — annoyance

- **Tried:** Followed the getting-started dependency instructions.
- **Expected:** A registry dependency matching this documentation.
- **Resolved by:** Used the tester brief's ordinary registry dependency, version 0.3.52 from the crate index; guide says to use a local checkout because no registry release exists.
- **Read:** `/kit/doc/pert_ui/guide/getting_started/index.html`, “Run your first window”; `/kit/doc/pert_ui/index.html`.

### 18:44 — compile — annoyance

- **Tried:** Wrote headless keyboard sorting and row-reveal checks from the Table guide; guessed `ListRevealStatus::Revealed` and `Modifiers::ALT`.
- **Expected:** These conventional names to exist; the table docs explain the gesture but do not show constructing Alt modifiers.
- **Resolved by:** The enum and struct reference pages show `Mounted(WidgetId)` and `Modifiers { alt: true, ..Default::default() }`. Also corrected capture window borrowing and removed `?` from infallible `clear_focus` using compiler diagnostics.
- **Read:** `/kit/doc/pert_ui/struct.Table.html`, `/kit/doc/pert_ui/enum.ListRevealStatus.html`, `/kit/doc/pert_ui/struct.Modifiers.html`.
- **Diagnostic:** `error[E0599]: no variant, associated function, or constant named Revealed found for enum pert_ui::ListRevealStatus`; `error[E0599]: no associated function or constant named ALT found for struct pert_ui::Modifiers`; `error[E0308]: mismatched types ... expected &NativeWindowId, found NativeWindowId`; `error[E0277]: the ? operator can only be applied to values that implement Try ... ui.clear_focus()?`.

## Closing

- **Result:** All five acceptance items met. `cargo run --offline -- --headless` passed; `cargo test --offline` runs the same session. Real Start and Cancel buttons are activated through Runtime keyboard input. Mailbox delivery updates visible progress, completion, cancellation, failure row, persistent Banner notification and summary. Keyboard header sorting is checked ascending and descending for every data column against actual row positions and independently ordered values. `jobs.png` is a CPU-rendered capture with running, done, cancelled and failed rows. After all jobs stop, advancing runtime time schedules no deadline and produces no repaint damage.
- **Concepts I had to learn:** Keyed Elements and retained identity; Signal/ReadSignal; UI-local Runtime and headless flush clock; retained Mailbox with Send UiSender; TableItems/TableSource/TableController and application-owned sorting; determinate ProgressStatus; Banner visibility; row reveal receipts; inspection semantics; native modifiers and keyboard input; CPU capture host; next_deadline and needs_paint.
- **What worked well:** The crate map led directly to the relevant guides. The worker mailbox example and CPU screenshot example were sufficient without reading vendor source. Compiler diagnostics were precise for capture borrowing. Retained keyed rows handled repeated source replacements and sorts correctly. The first complete headless run passed, and the PNG was legible without further layout tuning. No vendor source was consulted.
- **The three changes I want most:** Update the getting-started dependency instructions to match registry use; provide a complete sortable worker-progress table example; show modifier construction and row reveal status matching in the table's headless examples.
