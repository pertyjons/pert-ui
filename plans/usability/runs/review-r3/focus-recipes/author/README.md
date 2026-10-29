# Round 3 recipes: P7 Button-row list focus and P11 tab-close refocus

Author-side implementation evidence, 2026-10-29. Worktree
`/home/per/pert-ui-usability/r3-recipes-worktree`, detached at `8b2476a`
(uncommitted, nothing published), rustc 1.99.1. This is not an independent
review, a workspace/API gate, native evidence or a blind result. The root
integrator owns the gates and a fresh review.

Governing documents: main `plans/usability/round-3-design-menu-keyboard-diagnostics.md`
revision 3 and `runs/review-r3/menu-diagnostics-design-review-r3.md`, with
`runs/triage-r3/followup-probes/README.md` and `runs/triage-r3/author-followup-probes.md`
as the evidence the recipes rest on. None of those files was changed.

## Patch

`recipes.patch` holds the whole change (`git diff` with the two new files
intent-added for the diff only, then reset to untracked). `source/` holds copies
of the final files, and `source-sha256.txt` has their hashes. Every log records
the hashes it ran against.

| Path | Change |
|---|---|
| `crates/pert-ui/src/guide/layout.md` | New sections: "Focus after closing the selected tab", after "Adding and closing retained tabs", and "Button rows that keep list navigation", after "An editable list of records". Each has a complete doctest. |
| `crates/pert-ui/tests/collections/row_focus_recipe.rs` | New: 3 P7 tests |
| `crates/pert-ui/tests/collections/main.rs` | `mod row_focus_recipe;` |
| `crates/pert-ui/tests/surfaces/tab_focus_recipe.rs` | New: 6 P11 tests |
| `crates/pert-ui/tests/surfaces/main.rs` | `mod tab_focus_recipe;` only. The root's concurrent `reveal_diagnostics` module line is not copied. |

There are no runtime, public API, version, dependency, CHANGELOG or rustdoc
item changes, and no new test binaries. The examples and tests import only
`pert_ui::*`, `pert_ui::prelude::*` and `pert_ui::testing`. They make no
assumption about private keys or the order of children. Each fixture mounts one
root. Each P11 fixture and P7 fixture asserts an empty `validation_report()`.

## What the recipes do

**P7.**
- The list is mounted over an empty `ListSource` (`NoRows`). The application
  finds the list's `WidgetId`, then publishes `Button` rows that hold it.
- A row's callback logs the opened row, calls
  `ListSelection::set_active(Some(row))` and then
  `EventContext::request_focus_for(&list)`.
- `item_tab_stops(false)`. `VirtualList::on_activate` is unchanged and still
  handles only Enter.
- The guide also says that an application whose rows should focus the editor
  they open is choosing a different, explicit policy.

**P11.**
- `Tabs::on_select` writes every reported selection to an application
  `Signal<Option<String>>`. It is seeded with the first tab, because the
  initial selection is not reported.
- Only for `ChangeOrigin::Redescription` while `cx.focused()` does it queue
  `cx.defer` with `cx.id()` and the newly selected key. This includes a strip
  that already had focus before the removal.
- The deferred work:
  1. calls `ui.flush(ui.now())`;
  2. returns unless `ui.focused() == tabs` and the recorded selection is still
     the queued key;
  3. calls `Runtime::find_descendant(&tabs, app_unique_editor_key)`;
  4. ignores `None`, and otherwise calls `ui.focus(&editor)`.

### Why the deferred work flushes first (design precision, verified)

`runs/00-explore-race-{plain,nested}.log` record an exploratory test that I
later replaced; its source is `scratch/explore_race.rs`.
- A deferred task runs at the start of the next flush, in `flush_inner`, where
  `dispatch_tasks` comes before update and notifications.
- `Runtime::key_input` changes a focused strip's selection at once. Its
  notification flush then runs the queued task **before** it delivers the
  `User` `on_select`.
- So without the flush, the task saw the stale record `b` and `can_focus(editor
  b) == true`, and `focus(editor b)` returned `Ok`. Update then hid b, and focus
  fell back to the strip: a transient focus on a non-current editor.
- With `ui.flush(ui.now())` first, which `EventContext::defer` explicitly
  permits, `on_select(b→c User)` is delivered inside the task. The record reads
  `c`, `can_focus` reads `false`, and the key recheck refuses.

Without that flush, the selected-key recheck the design requires could not see
this change. No new API or inspection was needed.

The ordering depends on the input path:
- `Runtime::command_key_input` (the native keyboard path and `Driver::key`)
  flushes before routing. The queued work therefore runs first, and the key
  then reaches the refocused editor.
- `Driver::click` reveals, and so flushes, before pressing.
- Tabs select on release. A raw title press therefore runs the work while the
  old selection is still current.

The attempt logs below record all three.

## Tested behaviour

P7 (`row_focus_recipe`), 3 tests:
- click alpha → log `open:alpha`, focus on the list, active/selected alpha;
  Down → beta; Enter → `activate:beta`, focus on the list; Tab leaves to
  `after` in one step;
- the list is remounted through `Show`, so it has a new identity and the old
  one is `Unmounted`. Rows still holding the old ID: the click returns `Ok`,
  the stale request is ignored, the Button keeps focus, and Enter presses it
  again (`open:alpha` twice). After the rows are republished, the click focuses
  the new list and Up/Enter gives `activate:beta`;
- after unmount, `key(list)` is `Unmounted` while the source still holds the
  rows. Replacing the source drops them (a `Weak` token), so they are retained
  by nothing else.

P11 (`tab_focus_recipe`), 6 tests:
- focused selected editor removed → the first flush: strip focused, `queued:b`;
  the second flush: editor b focused, and typing writes `Ztext b`. Adding d
  keeps the selection, the focus, the editor identities of b and c, and the
  canonical texts;
- strip focused before the removal → also refocuses editor b (the intended rule);
- sidebar focused → nothing queued, the sidebar keeps focus and typing;
- focus moved to the sidebar after queuing and before the work runs, by
  `Runtime::focus` and by a raw host press/release → `focus-elsewhere:b`, and
  the sidebar keeps focus;
- raw `key_input` ArrowRight on the strip before the work runs → `stale:b`,
  selected c, focus stays on the strip, and the User change queues nothing;
- the newly selected tab has no editor (a preview Label) → `no-editor:preview`,
  focus stays on the strip, no error.

Guide doctests:
- P7: `opened == [alpha, beta]` and focus on the list after click/Down/Enter;
- P11: closing the focused editor's tab moves typing to the next editor
  (`"Edited. Text of second"`), and focus in the search field survives a later
  close.

## Commands (final, `runs/`)

All were run through `scripts/run.sh`, which records the date, HEAD, rustc,
the command, the owned-path hashes, the full output and the exit status.

| Log | Command | Result |
|---|---|---|
| 01 | `cargo fmt --all --check` | exit 0 |
| 02 | `cargo test --locked -p pert-ui --test collections row_focus_recipe` | 3 passed |
| 03 | `cargo test --locked -p pert-ui --test surfaces tab_focus_recipe` | 6 passed |
| 04 | `cargo test --locked -p pert-ui --doc guide::layout` | 14 passed (12 before + 2 new) |
| 05 | `cargo clippy --locked -p pert-ui --all-targets -- -D warnings` | exit 0 |
| 06 | `RUSTDOCFLAGS="-D warnings" cargo doc --locked -p pert-ui --no-deps` | exit 0 |
| 07 | the 02/03/04 set with `--no-default-features` | 3 / 6 / 14 passed |
| 08 | `cargo test --locked -p pert-ui --test collections` and `--test surfaces` (whole binaries) | 464 / 470 passed |

I did not run the workspace gate, the API listing, other feature configurations
or native checks, and I make no claim for them.

## Negative controls (`negative-controls/`)

`scripts/negative_control.py` replaces one exact text, given in
`negative-controls/mutations/*.old/new`, runs the command, and restores the
file. The log records the sha256 before and after the restore, and they are
identical. Every control failed (exit 101) with the failure shown:

| Control | Mutation | Failing test and observation |
|---|---|---|
| nc-p11-origin | drop the `Redescription` check | stale test: the User change queued `c` and then `focus:c` (keyboard navigation on the strip jumps into the content) |
| nc-p11-recipient | drop `cx.focused()` | another-region test: the work is queued, and only the deferred recheck (`focus-elsewhere:b`) keeps the sidebar |
| nc-p11-deferred-focus | drop the deferred `ui.focused()==tabs` recheck | focus moved to the sidebar before the work: editor b (serial 16) steals focus from the sidebar (2) |
| nc-p11-both-focus | drop both focus guards | another-region and moved-elsewhere tests: focus is stolen |
| nc-p11-flush | drop the leading flush | stale test: `focus:b` instead of `stale:b` (transient focus on the hidden-to-be editor) |
| nc-p11-key | drop the selected-key recheck | stale test: `Error: Hidden` (focusing editor b after the flush hid it) |
| nc-p11-flush-and-key | drop both | stale test: `focus:b` |
| nc-p11-missing | a missing editor becomes an error | missing-editor test: `Error: Runtime(Unmounted)` |
| nc-p7-focus | drop `request_focus_for` | focus stays on the Button (6 instead of list 5); the remount test fails likewise |
| nc-p7-active | drop `set_active` | active is `None` after the click |
| nc-guide-p7-focus | drop `request_focus_for` in the guide | doctest: `["alpha", "alpha"]` instead of `["alpha", "beta"]` |
| nc-guide-p11-focus | drop both focus guards in the guide | doctest: the search field loses focus to an editor |

The recipient and deferred focus guards overlap by design: either one alone
keeps focus in another region. The single-guard controls show which test
records each guard's decision, and the combined control shows the steal.

The application-unique editor key has no separate control. A duplicate key
makes `find_descendant` return `AmbiguousKey`, which the runtime's own
`find_descendant` doctest covers. Duplicating it here would also break the
fixtures' own lookups.

## All attempts

- `iteration1/`: the complete first round of runs and controls (all
  passing/failing as in the final round) before three last edits:
  - the P7 fixture's validation assertion;
  - a multi-line `Files` struct and an `editor` lookup closure in the guide;
  - one sentence on `item_tab_stops`.
  The mutated regions are unchanged.
- Earlier, unlogged compile errors were fixed in the session:
  - borrow errors in the first drafts of the P7 tests and the explorer;
  - a doctest that addressed the editor with `Driver::find(&tabs,
    &[editor_key])`, which returned `NotFound`, since a tab key is an address
    level; it now uses `find_descendant`.
- First P11 test draft:
  - It used `Driver::key` and `Driver::click`. Both flush before their input,
    so they did not produce the intended interleaving.
  - Re-run as mutations, these are `attempt-driver-key` (the work runs first;
    the key goes to editor b; selection stays b), `attempt-driver-click` (the
    work runs first; `focus:b`; the click then moves focus to the sidebar) and
    `attempt-title-press` (the strip selects on release; the work ran while b
    was still selected; `focus:b`).
  - The tests now use raw `Runtime::key_input` and a raw host press/release.

## Limits

- Headless only; no native, Windows/macOS or human-input claim.
- The P11 stale-selection case arises only on input paths that deliver before
  flushing: `Runtime::key_input`, which some hosts and tests use, and other
  direct mutations. The native keyboard path flushes first.
- The P11 rule treats a strip focused before the removal like focus that fell
  back from the removed editor, by design. Nothing distinguishes them, and no
  focus history is kept.
- `ui.focus` focuses a component whose `focusable` is false, as documented. A
  disabled editor under the unique key would therefore receive focus. The
  design asked only to ignore a missing editor, so the recipe adds no
  `can_focus` check.
