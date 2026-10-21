# Acceptance: r1-2-codex (task 2, task list)

Author check, 2026-10-21. The submitted `work/` (without `target/` and the
CLI directories) was copied to `~/pert-ui-usability/acceptance-r1-2-codex`,
built with `cargo build --all-targets` and tested with `cargo test` in the kit
sandbox (offline): builds without warnings, both headless tests
(`tests::task_list_through_simulated_input`,
`tests::keyboard_rename_and_filter_selection_edges`) pass. The tests drive the
app with pointer presses on measured bounds, `command_key_input` with native
modifiers and `text_input`; they read the status line from inspection
(`Semantics::name` of the `status` label), the filtered set from the library's
`ListFilter::items()`, the selection from `ListSelection::active()`, and
order, titles and done state from the application's `Vec` signal.

The tester's checks never assert the *rendered* list after removal or reorder,
or which rows are hidden by the filter (they read the `Vec` and the filter's
output signal), and the Escape check would also pass if the draft had never
opened. An author test was therefore added in the acceptance copy only:
`src/author_tests.rs` `author_acceptance` (hooked in by `#[cfg(test)] mod
author_tests;` at the end of `src/main.rs`). It reads every UI result from the
mounted tree: visible row titles in on-screen order (inspection `visible` +
label `Semantics::name`, sorted by `bounds`), the status label from
inspection, and the in-place editor's visibility and `SemanticValue`. It
covers: Ctrl+N focus and Enter add with a blank refused; removing an element
directly from the `Vec` removes the row and re-inserting it restores it;
checking updates the shown status; Alt+Up/Alt+Down reorder the shown rows;
rename draft visible with the typed text, Escape hides it and keeps the old
title, Enter commits the new title on screen; mixed-case filter shows only the
match, `Vec` unchanged, hidden selection moves to the shown row, Escape clears
the filter; Delete with list focus removes the selected row and updates the
status; arrow keys move the selection. It passes (3 passed in total).

| Item | Result | Code | Check |
|---|---|---|---|
| 1. Add, remove, check, reorder update list and status | met | `src/main.rs:38` `Model::add`, `:53` `remove`, `:68` `shift`; list source derived from the `Vec` signal `:306`; checkbox `:215`; status `:317` | tester: `:487` (Enter and Add button add, blank refused, checkbox, Alt+arrows, Move buttons, Delete and Remove, status from inspection each time) and `:656` (uncheck); author `author_tests.rs:81` for the rendered rows |
| 2. Rename in place: Enter commits, Escape discards | met | `src/main.rs:80` `Title` component owning a `DraftEditorController` and `DraftEditor` child, `:149` key handling; start via `VirtualList::on_activate` `:316`; empty renames refused `:164` | tester `:547`–`:557`, `:612`–`:622` (Enter, type, Enter commits; Enter, type, Escape keeps the title); author check confirms the draft is visible before Escape and hidden after |
| 3. Filter hides, keeps selection meaningful, vector unchanged | met | `ListFilter`/`CollectionFilter` `:312`, `:349`; `SelectionGuard` `:255` moves a hidden selection to the first match or clears it; `remove`/`shift` act only on a visible selection `:31` | tester `:562`–`:570`, `:624`–`:649` (mixed case, one match, `Vec` unchanged, selection moved, no match clears selection, Remove and Delete change nothing); author check reads the hidden rows from the rendered tree |
| 4. Ctrl+N, Delete, Alt+Up, Alt+Down | met | application commands `:367`–`:400` | tester `:474` (Ctrl+N focus asserted), `:511`, `:528`, `:574`, `:648`, all through `command_key_input` with native modifiers |
| 5. Headless check through simulated input | met | `#[cfg(test)] mod tests` `src/main.rs:418` | both tests exercise items 1–4 through simulated input only; no model function is called |

Doubt between met and partly met, recorded explicitly: items 1 and 3 ask that
the *list* updates and that filtering *hides* rows, and the tester's tests
verify that only indirectly (the `Vec`, and the `ListFilter` output that feeds
the `VirtualList`), never on the rendered rows. The pilot was accepted with the
same pattern (order and titles read from the `Vec`), the inputs are all
simulated, the library-owned filter output is not an application model
function, and the author test confirms the rendered behavior, so the items are
judged met.

Task-specific checks from `acceptance.md`:

- The list is driven by the tester's `Vec`: met. The `VirtualList` source is
  `tasks.map(ListSource::new(Rows { tasks }))`; the author test removes an
  element from the `Vec` directly and the row disappears from the rendered
  list.
- In-place rename commits on Enter and is discarded on Escape: met (tester and
  author checks).
- The filter does not change the vector: met (tester asserts `Vec` equality
  through filtering and clearing).
- Ctrl+N, Delete, Alt+Up and Alt+Down through simulated key input: met.
- The status line follows checking and removing: met, read from inspection.

## Triage notes

These are observations for the library triage, not acceptance failures.

- **Library use.** Unlike the pilot, the tester used the library's
  collection stack: `VirtualList` over `ListSource`/`ListItems`, a
  single-mode `ListSelection`, `ListFilter` with `CollectionFilter` for the
  search field, `DraftEditor`/`DraftEditorController` for the in-place
  rename, `Checkbox`, `TextField::on_submit`, `Button` and application
  commands. Arrow-key selection is the `VirtualList`'s own.
- **Hand-built pieces.** It added a custom `Title` component wrapping
  `Label` + `DraftEditor`. It needed `retain_initial_children() -> true` to keep
  the editor and its owner on the same controller after the source was
  replaced (friction entry 15:46). The rename start is bridged from
  `VirtualList::on_activate` through an extra `edit: Signal<Option<u64>>` that
  each `Title` reads in `update` and acts on in `deliver_notifications`.
- **Selection relocation workaround.** `SelectionGuard` (`src/main.rs:255`) is a
  wrapper component that reads the tasks, query and selection in `update`
  only to subscribe (`let _ = cx.read(...)`). It requests a notification on
  every update and rewrites a hidden selection in `deliver_notifications`. The
  reason is that `ListFilter`/`CollectionFilter` keep a hidden selection by
  documented policy while the search field has focus (friction entry 15:45).
- **Discarded values and error swallowing.** `let _ = ui.register_command(...)`
  discards the `CommandId` three times (`:367`, `:380`, `:388`). It is not
  `#[must_use]` and produces no warning. `self.visible(t).unwrap_or(false)`
  swallows signal read errors. `mount` uses `.unwrap()` on `find_descendant`
  (`:364`, `:365`) in the production path. There are no compiler warnings.
- **Whole-list rebuild.** Every `Vec` change clones the whole vector into a new
  `ListSource`. Each `build` creates a fresh `DraftEditorController` and a
  per-row `map` over the whole vector for the title.
- **Shortcut modifier.** Ctrl+N is registered with `control: true`, not the
  platform primary modifier, as in the pilot. The task says Ctrl+N.
- **Accessibility gaps seen in inspection.** The row checkbox is
  `Checkbox::new("")`, so its semantics have no name. The focusable `Title`
  component reports no semantics.
- **Move under a filter.** `shift` swaps with the neighbour in the underlying
  `Vec`, which may be hidden, so the visible order need not change. The README
  does not mention this.
- **Delete in the search field.** An author probe (removed afterwards) showed
  that Delete with the caret in the search field edits the query text and
  does not remove a task, as the README says. The tester's first test presses
  Delete after ArrowDown/Home following the search input and does remove a
  task. This suggests that `CollectionFilter` moved focus into the list there.
  I inferred this from the outcome and did not trace it.
- **Test helper.** The test's `key` maps `KeyCode` to `CommandKey` by hand
  (`KeyN` → `Character('n')`).
- **`/kit/vendor` path in the transcript.** `layout.rs:587` appears once, in
  the output of the tester's first `cargo build` (transcript line 30). It is
  rustc's `note: method defined here --> /kit/vendor/pert-ui-runtime-0.3.52/src/layout.rs:587:12`
  for an `E0308` on `Element::with_grow` (a float passed where `u16` is
  expected). It is a compiler diagnostic location, not source reading. No
  command in the transcript opens a file under `/kit/vendor`. The tester read
  only the guide, the README and rendered rustdoc, which matches the friction
  log's "Source reading: None".
- **`work/docs`.** This holds the tester's own plain-text dumps of rendered
  rustdoc pages from `/kit/doc/pert_ui`. `read.py` is a small `HTMLParser`
  script that strips tags, scripts and styles. The `.txt` files group the
  pages the tester read: guide chapters (controls, diagnostics, layout), and
  item pages for lists, filters, the draft editor, `Component`, `Runtime`,
  signals, input and inspection. The tester then searched them with
  `rg`/`sed`. This is documentation, not library source.
- **Friction log times.** The entries are 15:42, 15:43, 15:45 and 15:46, inside
  `brief_delivered=15:41:24` and `ended=15:48:16`. Each one was written in a
  command that also ran `date +%H:%M`. Two headings were first typed in
  advance (15:44 and 15:45) and then corrected to the shell's output (15:45 and
  15:46) by `sed`/`replace` in the following command. The final times match
  the shell outputs (15:42, 15:43, 15:45, 15:46), so they are plausible.

## Author decision after the Codex review (2026-10-21)

Moving under a filter: the task does not say whether "move up or down"
counts among shown tasks or in the whole list. This submission moves the
task one place in the underlying vector, which may pass a hidden task and
leave the shown order unchanged; its README does not mention it. An author
probe in the acceptance copy (`author_move_past_hidden_task`) added "milk
A", "Hidden" and "milk B", filtered on "milk", selected "milk A" and
pressed Move down: the vector became `["Hidden", "milk A", "milk B"]` and
the shown order stayed `["milk A", "milk B"]`. That is accepted as a
consistent reading, since the action changes the vector as asked, and the
missing visible effect goes to the triage. It differs
from r1-2-claude's partly met item 3, where a shown and selected task's
Remove and Move are enabled and do nothing at all. Result unchanged: met.
