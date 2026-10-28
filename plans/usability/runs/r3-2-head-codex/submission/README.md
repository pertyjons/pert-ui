# Task list

A desktop to-do application using `pert-ui` 0.3 with default features.
Task data lives in an application-owned `Signal<Vec<Task>>`; controlled
checkboxes and inline titles write changes back to that vector.

Run on a desktop: `cargo run --offline`.
Verify without a display: `cargo test --offline`.

- Type a title and press Enter or click Add. Whitespace-only titles are refused.
- Click a title to select its task; use Up/Down to navigate.
- Double-click a title, or press Enter on the selected row, to rename in place.
  Enter saves; Escape discards the draft.
- Remove deletes the selected visible task. Move up/down reorder it.
- Search filters titles without changing stored tasks. Hiding the active task
  clears it, and removal is guarded against hidden selections.
- Ctrl+N focuses task entry, Delete removes, and Alt+Up/Down reorder the selection.
- The status counts completed tasks across the entire underlying vector.

The headless test builds the same `App` as the desktop entry point and drives
it using `pert_ui::testing::Driver`. It asserts mounted row order, semantic
text/status, selection, focus and stored records; it never invokes model
mutation functions directly. No display or GPU is needed for that test.

See `FRICTION.md` for the development record.
