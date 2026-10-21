# Acceptance: r1-2-claude (task 2, task list)

Author check, 2026-10-21. The submitted `work/` (without `target/`, `.claude/`
and `.git/`) was copied to `~/pert-ui-usability/acceptance-r1-2-claude`,
built with `cargo build --all-targets` and tested with `cargo test` in the
kit sandbox (offline): builds without warnings, 7 of 7 tests in
`tests/headless.rs` pass.

The tester's checks drive the application through `command_key_input` (with
`native_modifiers`), `text_input` and `pointer_input`, and read results from
the task `Vec` signal, the application's `status` signal, the selection
signal and the mounted row keys. They do not read the status label, the row
titles or the checkbox values from inspection. The author therefore added
`tests/author.rs` in the acceptance copy only, with three tests:

- `author_shown_state_follows_the_vector` (passes): status label text,
  checkbox values and row titles read from `Runtime::inspect` follow checking,
  Alt+Down, Delete, and direct removal from and addition to the vector.
- `author_rename_shows_in_the_row` (passes): Enter on the selected row,
  typed text and Enter shows the new title in the row; Escape discards.
- `author_filter_with_trailing_space_still_acts_on_the_shown_selection`
  (fails): with "milk " typed in the search field, "Buy milk" is shown and
  selected and Remove is enabled in inspection (`disabled: false`), but
  Delete leaves the vector unchanged.

| Item | Result | Code | Check |
|---|---|---|---|
| 1. Add, remove, check, reorder update list and status | met | `src/lib.rs:152` `add` (Enter `:305`, button `:308`), `:168` `remove_selected`, `:71` checkbox `on_change`, `:186`/`:207` moves, `:267` `reorderable`; rows from the vector signal `:249`; status `:294`, label `:325` | `tests/headless.rs:149`, `:181`; author `author_shown_state_follows_the_vector` (status label and rows from inspection) |
| 2. Rename in place: Enter commits, Escape discards | met | `src/lib.rs:78` `InlineText` with `normalize` and `on_save`; Enter on the row opens it through a synthesized F2 (`:272`–`:290`) | `tests/headless.rs:257` (double-click), `:335` (Enter on the row); author `author_rename_shows_in_the_row` |
| 3. Filter hides, keeps selection meaningful, vector unchanged | partly met (doubtful between met and partly met, see below) | `src/lib.rs:255` `ListFilter`, `:291` `CollectionFilter`, `:258` `has_target` from `filter.items()`; `:130` `visible_keys` re-implements the filter for remove/move | `tests/headless.rs:284`, `:355`; author `author_filter_with_trailing_space_still_acts_on_the_shown_selection` fails |
| 4. Ctrl+N, Delete, Alt+Up, Alt+Down | met | `src/lib.rs:338`–`:372` application commands; Alt+arrows on the focused list through `reorderable` `:267` | `tests/headless.rs:154` (Ctrl+N), `:210`–`:221` (Alt+arrows on the list and through the commands), `:232` (Delete), `:371` (disabled when hidden) |
| 5. Headless check through simulated input | met | `tests/headless.rs` | all of the above; also a CPU capture (`:386`); `validation_report()` is printed but not asserted |

Item 3: hiding, case-insensitive matching, the unchanged vector and the
handling of a hidden selection (Remove, Move and the shortcuts disabled
through `Button::enabled` and `command_enabled_signal`) are implemented and
tested. The defect: the library trims the query before matching
(`FilterMatcher` docs: a blank query shows every row; `effective` trims), but
`visible_keys` (`src/lib.rs:131`–`:138`) matches the untrimmed query. With a
query that has trailing (or, depending on the title, leading) whitespace,
`has_target` (from `filter.items()`) says the selected task is shown and
enables Remove/Move and the shortcuts, while `remove_selected` and
`move_selected` treat it as hidden and silently do nothing. Delete, the
Remove/Move buttons and the application Alt+arrow commands are affected;
Alt+arrows on the focused list go through `reorderable` and still work. The
selection is then not kept meaningful for the actions offered on it. The
author's failing test demonstrates it. It is a narrow edge case (a
whitespace-padded query, e.g. "walk " while typing a longer query), which is
why the result is borderline; judged strictly it is partly met. Fixing it
would need `visible_keys` to read `filter.items()` (or trim the query).

## Task-specific checks

- List driven by the tester's `Vec`: met. The `ListSource` is a `map` of the
  vector signal (`src/lib.rs:249`); the author test removes and adds tasks in
  the vector directly and the rows follow.
- In-place rename commits on Enter, discarded on Escape: met.
- The filter does not change the vector: met (`tests/headless.rs:295`, `:319`).
- Ctrl+N, Delete, Alt+Up, Alt+Down through simulated key input: met, with
  `command_key_input` and `native_modifiers`. Limitation the tester recorded:
  Alt+Up/Down do nothing while a text field has focus, because the editor
  consumes them before application commands.
- Status line follows checking and removing: met; the tester reads the
  status signal, the author test reads the label from inspection.

## Triage notes

- Rename from the keyboard: `on_activate` defers a closure that focuses the
  row's `InlineText` and sends itself an F2 press/release with
  `Runtime::key_input` (`src/lib.rs:272`–`:290`), because `InlineText` has no
  programmatic start (same workaround as the pilot).
- Per-row state: each `ListItems::build` creates throwaway signals with
  `Signal::sibling` seeded from the vector (`src/lib.rs:68`–`:69`), and the
  controls' callbacks write the vector back. It works; `sibling` is
  documented for components owning popup state, not for this.
- `visible_keys` (`src/lib.rs:130`) re-implements the filter with
  `FilterMatcher::substring` instead of reading the filter's items, which
  `has_target` already uses; this duplication causes the item 3 defect.
- Discarded `#[must_use]` values: four `CommandId`s from `register_command`
  and one `TaskReceipt` from `EventContext::defer` produced `unused_must_use`
  warnings during the session; the tester silenced them with `let _ =`
  (`src/lib.rs:274`, `:339`). Errors returned inside the deferred closure are
  therefore not observed. The final build has no warnings.
- `Modifiers`, `KeyChord`, `CommandKey`, `CommandScope`, `Point`,
  `PointerEvent` and other input types were imported from the crate root
  because they are not in the prelude (E0422 on `Modifiers`).
- `CaptureTarget::window` takes `&NativeWindowId`; the guide prose suggested
  the value (E0308).
- The tester's `validation_report()` result is only printed; the
  `serve_captures` count is ignored. The last filter test sets the query
  signal directly instead of typing it (the earlier filter test types it).
- The guide's getting-started dependency instructions (path checkout) were
  reported as contradicting the registry crate in the kit.

Vendor path: the transcript's `/kit/vendor/pert-ui-runtime-0.3.52/src/runtime/notify.rs:242`
appears only in a compiler diagnostic, the E0422 for `Modifiers`, where rustc
suggested the similarly named `Notifier` and showed its definition (tool
result at 13:44:24Z). The tester did not read library source; its only other
`/kit/vendor` access was `ls /kit/vendor | head -50` while exploring the kit
at the start. The closing says "I never needed the library source".

## Friction log times

`brief_delivered=15:40:24`, `ended=15:48:02`. Entries: 15:43 (two),
15:45, 15:47 (five). All lie inside the window, and the transcript shows each
time was taken from `date +%H:%M` in the command that wrote the entry
(13:43:02Z, 13:45:54Z, 13:47:33Z). The five 15:47 entries were written in one
batch: some describe earlier events (the `Modifiers` compile error occurred at
15:44, the F2 workaround and filter findings around 15:46), so their times are
write times, not event times. Not plausible: the 15:43 discover entry claims
"~25 min of reading", but documentation reading ran from about 15:40:37 to
15:42:51 and the whole session lasted 7.6 minutes.

## Author decision (2026-10-21)

Item 3 stays partly met. The defect is confirmed in the source
(`src/lib.rs:130-138`): the application repeats the filter's matching
without the library's trimming, so with a trailing space in the query a
shown and selected task has Remove and Move enabled while they do nothing.
That is a selection that is not meaningful for the task's actions. Result:
partly met; no accepted completion time.
