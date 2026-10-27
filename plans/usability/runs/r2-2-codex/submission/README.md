# Task list

A small pert-ui 0.3.75 application. All records live in an application-owned
`Signal<Vec<Task>>`; list rows propose changes back to that vector.

Run the desktop app where a display is available:

```sh
cargo run --offline
```

Verify without a display:

```sh
cargo test --offline
```

The three tests use pert-ui's `Driver` to click, type and press keys against
the same UI built by the desktop app. They check records, displayed row order
and titles, status text, filtering and selection.

- Enter in the new-task field or Add creates a task; whitespace-only titles are refused.
- Click a title to select it; use Up/Down to navigate the list.
- Double-click a title or press Enter on the selected row to rename it.
  Enter commits; Escape discards the draft.
- Remove deletes the selection; Move up/down changes its position.
- Ctrl+N focuses the new-task field, Delete removes the selection,
  and Alt+Up/Down reorders it. Text editors retain their normal editing keys.
- The search field filters titles case-insensitively. Hidden active rows are
  cleared. Removal selects the next visible neighbor, then the previous one.
- The status line counts all tasks, including tasks hidden by search.

See FRICTION.md for the contemporaneous documentation experience and closing notes.
