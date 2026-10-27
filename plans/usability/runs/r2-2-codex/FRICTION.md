# Friction log

Copy this file to `/work/FRICTION.md`. Add one entry per event, in time
order, as it happens. When something costs you time, run `date +%H:%M` at
that moment and write the entry before you continue; do not estimate a
time. If you do write an entry later, give it the time you write it and add
`(written later)` after the time.

## Entries

### 18:50 — discover — annoyance

- **Tried:** Writing explicit Ctrl+N and Alt+arrow shortcuts alongside the guide's `KeyChord::primary` example; initially sketched `KeyChord::ctrl` and `.alt()` conveniences.
- **Expected:** Symmetric convenience constructors for Control and Alt.
- **Resolved by:** `KeyChord::new(CommandKey::…, Modifiers { control/alt: true, ..Default::default() })`. The Modifiers page explicitly explains the field syntax. Corrected before the compiler reached the app.
- **Read:** `/kit/doc/pert_ui/struct.KeyChord.html`, `/kit/doc/pert_ui/struct.Modifiers.html`.

### 18:51 — understand — annoyance

- **Tried:** Checking what happens to selection after removing its record, using the editable-list guide's guarded Delete pattern.
- **Expected:** Removal would select a neighbor or clear the removed key.
- **Resolved by:** `ListSelection` documents that selection is never pruned. The app will explicitly select the next visible neighbor, then the previous, or clear it after removal.
- **Read:** `/kit/doc/pert_ui/guide/layout/index.html` (An editable list of records), `/kit/doc/pert_ui/struct.ListSelection.html`.

## Closing

- **Result:** All five acceptance items met. `cargo check --offline` succeeded. `cargo test --offline` passed all three headless tests. Tests drive the same `build` function as the native app through pert-ui's `Driver` clicks, text input, keys and chords; no test calls application edit/add/remove/reorder functions. They assert the vector, rendered row order and titles through inspection, selected keys, filter counts and status-line semantics. No display was used.
- **Concepts I had to learn:** keyed Elements and retained identity; Signal/ReadSignal and derived values; ListItems/ListSource over an owned Vec; controlled row widgets; ListSelection and ListFilter; bounded VirtualList layout; InlineText row mode and deferred BeginEdit; command scopes and explicit modifiers; Driver settling and semantic inspection.
- **What worked well:** The crate landing page points clearly to the guide. The layout guide's complete editable-list example closely matches this task and explains why controlled widgets preserve drafts. The testing chapter documents text replacement, key routing, clicks, settling and stable row paths well enough to implement the headless checks without reading library source. Everything compiled on the first completed check; the first three input tests passed immediately. No library source was read.
- **The three changes I want most:**
  1. Add symmetric explicit Control/Alt shortcut helpers or place a non-primary modifier example directly on KeyChord's page.
  2. Extend the editable-list example's Delete action to clear selection or choose a neighbor, making the non-pruning selection contract visible in executable code.
  3. Include a compact complete task-list example with new-task entry, status, and application shortcut tests alongside the existing editable-records example.
