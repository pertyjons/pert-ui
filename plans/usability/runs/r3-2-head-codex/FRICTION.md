# Friction log

Copy this file to `/work/FRICTION.md`. Add one entry per event, in time
order, as it happens. When something costs you time, run `date +%H:%M` at
that moment and write the entry before you continue; do not estimate a
time. If you do write an entry later, give it the time you write it and add
`(written later)` after the time.

## Entries

### 17:01 — compile — annoyance

- **Category:** compile
- **Severity:** annoyance
- **Tried:** Passed the layout guide's `build(&mut Runtime) -> Result<TaskList, RuntimeError>` directly to `App::new`.
- **Expected:** A runtime-building function to be accepted as the app initializer.
- **Resolved by:** Wrapping it in `|ui| Ok(build(ui)?)` to convert the error to `AppError`.
- **Read:** `guide/layout/index.html`, “An editable list of records”; `guide/testing/index.html`, “The close policy”; `App::new` signature in the compiler diagnostic.
- **Diagnostic:** `error[E0271]: expected build to return Result<_, AppError>, but it returns Result<TaskList, RuntimeError>` at `src/main.rs:141:51`. The diagnostic did identify the mismatch; this was a small composition snag between the two guide examples.

## Closing

- **Result:** All five acceptance items met. `cargo test --offline` passed `tests::full_headless_user_session` (1 passed). The test uses the same `App` initializer through `App::headless` and `testing::Driver`, simulating clicks, text, Enter/Escape, arrow keys, Ctrl+N, Delete and Alt+Up/Down. It verifies the vector, row order, semantic titles/status, selection and focus. No model mutation function is called by the test. No native display was available.
- **Concepts I had to learn:** `App` and `Runtime`; signals and derived values; keyed elements; `ListItems`/`ListSource` over a copied vector; controlled row controls; `ListSelection`; `ListFilter` and `CollectionFilter` hidden-active policy; inline editor drafts and child addresses; deferred BeginEdit; list reorder drops; command scopes and exact modifiers; `HeadlessApp`, `Driver` and semantics inspection.
- **What worked well:** The layout guide's “An editable list of records” recipe closely matched the task, including controlled checkboxes/titles and the hidden-selection guard. The testing guide explained exact row/editor paths and how to simulate real input. `InlineText` supplied Enter/Escape draft behavior and row selection cooperation. `VirtualList` supplied arrow navigation and Alt+arrow reorder. The first complete headless session passed. No library source was read.
- **The three changes I want most:** (1) Show a native `App` wrapper around the runtime-based editable-list recipe, including error conversion. (2) Extend that recipe with task entry and a derived completion count so a first-time reader can assemble the whole app in one place. (3) Cross-link the recipe directly to `CommandScope`'s routing explanation for adding Delete/Ctrl+N without interfering with editor keys. These are documentation improvements; I encountered no missing library capability.
