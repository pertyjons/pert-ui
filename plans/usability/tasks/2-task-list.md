# Task: task list

Build a small to-do application.

## Features

- A list of tasks, each with a title and a done checkbox. The list is
  driven by your own `Vec` of task values: the UI shows whatever the vector
  holds, and every change goes through it.
- Add a task from a text field (Enter or an Add button); empty titles are
  refused.
- Remove the selected task.
- Reorder: move the selected task up or down.
- Rename a task in place: start editing its title inside the list, commit
  with Enter, cancel with Escape.
- Selection: one task at a time, by pointer and with the arrow keys.
- A search field that filters the visible tasks by title, case-insensitive,
  without changing the underlying list.
- Keyboard shortcuts: Ctrl+N focuses the new-task field, Delete removes the
  selected task, Alt+Up and Alt+Down move it.
- A status line: "N of M done".

## Constraints

- `pert-ui` with its default features, plus `cpu` if you want captures.
- Verification without a display: tests or a headless mode that drive the
  application through the library's own facilities, not by calling your
  own model functions directly.

## Acceptance

1. Adding, removing, checking and reordering update the list and the status
   line.
2. In-place rename commits with Enter and is discarded with Escape.
3. Filtering hides non-matching tasks, keeps the selection meaningful and
   does not change the underlying vector.
4. The keyboard shortcuts work.
5. A headless check exercises items 1 to 4 through simulated input.
