# Task list

A small Rust task list using `pert-ui` 0.3.52 with default features.

```sh
cargo run --offline
```

The native window needs a supported desktop display. Run the simulated-input
checks without a display:

```sh
cargo test --offline
```

Add tasks with Enter or Add. Click a title to select it, or focus the list and
use arrows. Enter starts in-place rename; Enter saves and Escape discards it.
F2 also starts rename when a title has focus. Empty additions and renames are
refused. Leaving a rename by clicking elsewhere discards the draft.

Ctrl+N focuses the new-task field. Delete removes the selected task; Alt+Up and
Alt+Down reorder it. Remove and move buttons provide the same actions. Text
editors retain their normal editing keys while focused.

Search matches title substrings without regard to case. If search hides the
selected task, selection moves to a matching task or clears when none match.
Filtering leaves the task vector untouched; the status counts all tasks.

The two headless tests use `Runtime` pointer events, text input, native modifiers
and command-key input. They inspect the rendered widget semantics and task data
without invoking model mutation methods. No library source was read.

See [FRICTION.md](FRICTION.md) for the documentation and integration record.
