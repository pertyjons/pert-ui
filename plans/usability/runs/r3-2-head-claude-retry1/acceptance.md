# Acceptance: r3-2-head-claude-retry1 (task 2, task list)

Independent verifier: fresh Claude session (claude-opus-5-5), 2026-10-28.
Procedure: `plans/usability/acceptance.md` (task-specific paragraph **2. Task
list**), round rules `plans/usability/round-3-plan.md`. This is headless,
library-driven evidence only. It does not claim native execution or human
usability.

**Verdict: met.** All five numbered items are met, backed by passing
library-driven evidence from the tester's tests plus the verifier's own
tests.

## Protocol and metadata

- Kit `kit-r3-head/kit.env` matches the run's `conditions.env`:
  commit=32dd9c4f667b50cfbbef7e46b18c0007e2b08ee2, dirty=0, version=0.3.88,
  toolchain=rustc 1.99.1 (58cf8f9c3 2026-10-13), prepared=2026-10-28T16:41:41+01:00.
- Session: brief_delivered=2026-10-28T17:22:33+01:00,
  ended=2026-10-28T17:28:50+01:00 (6.3 min of a 90 min budget), exit 0, outcome=ended.
- Copy: `RUN_DIR/work` was copied to `ACCEPTANCE_DIR/work` with rsync, leaving out
  `target/`, `.git/`, `.claude/` and `.codex/`. The source had no symlinks and
  no credentials. All 7 files match `original-source-manifest.json`, both in
  the original and in the copy, at the end of the session
  (`evidence/09-original-hash-check.log`).
- All builds and tests ran via `run.sh exec kit-r3-head ACCEPTANCE_DIR …`.
  The library resolves from `/kit/vendor/pert-ui*-0.3.88` through the
  sandbox's offline vendored source (`evidence/07-library-source.log`). No
  workspace path or installed library was used.
- Features: `pert-ui = { version = "0.3.88", features = ["cpu"] }`. `cargo tree -e
  features` shows `default` (→ desktop → gpu → native) and `cpu`, which is
  what the task requires (`evidence/04-features.log`).
- Protocol deviations: none observed.

## Build and test commands (all through the harness)

| # | Command (`run.sh exec KIT ACCEPTANCE_DIR …`) | Result | Log |
|---|---|---|---|
| 1 | `cargo --version` | cargo 1.99.1 | evidence/01-cargo-version.log |
| 2 | `cargo build --locked --all-targets` | builds, exit 0 | evidence/02-build.log |
| 3 | `cargo test --locked` (tester's suite only, before verifier tests were added) | 6/6 passed, exit 0 | evidence/03-tester-tests.log |
| 4 | `cargo tree --locked -e features -i pert-ui`; `cargo tree --depth 1` | default + cpu | evidence/04-features.log |
| 5 | `cargo test --locked --test acceptance` | 6/6 verifier tests passed | evidence/05-acceptance-tests-attempt1.log |
| 6 | `cargo test --locked` (final: tester + verifier) | 12/12 passed, exit 0 | evidence/06-final-suite.log |
| 7 | `cargo metadata` / sandbox cargo config | library from /kit/vendor | evidence/07-library-source.log |
| 8 | `cargo clippy --locked --all-targets` | clean, exit 0 | evidence/08-clippy.log |

The verifier tests passed on the first attempt. Nothing in the application was
changed. No test access or entry point was added: the tests use the public
`todo::app()`, its `headless()` (library `App`/`HeadlessApp`) and the public
`TaskList` state.

## Added verifier test source and captures

- `work/tests/acceptance.rs` (sha256 64f9d52f…e79614), with 6 tests:
  `vector_drives_rows`, `status_follows_check_uncheck_remove`,
  `rename_enter_commits_escape_discards`,
  `filter_keeps_vector_and_meaningful_selection`, `shortcuts_from_other_focus`,
  `add_updates_rows`.
- Capture: `work/acceptance-captures/after-adds.png` (CPU renderer, 960×640,
  sha256 f03c61e3…c505d1). It shows five rows, the search field, the
  Move/Remove buttons and "0 of 5 done". This is supporting evidence only and
  no item rests on it.
- The tester's own capture goes to `target/tmp/tasks.png` in the copy. It is
  a build output, and only the passing test is recorded.

## Implementation summary (source)

- `src/lib.rs:173-275` `build`: `tasks: Signal<Vec<Task>>` is the single store.
  Rows come from `tasks.map(… ListSource::new(Rows{…}))` (`:178-181`). A
  `ListFilter` with a `ListSelection` (Single) wraps the rows (`:182-183`), and
  a `VirtualList` shows them (`:204-224`), wrapped by `CollectionFilter` with
  `HiddenActive::Clear` (`:242-246`). The status is `tasks.map(… "{done} of {n}
  done")` in a `Label` (`:188-191`, `:248`).
- Each row (`:33-51`) has a controlled `Checkbox` and a controlled `InlineText`
  title, and both write back into the vector through `edit` (`:57-67`). An
  empty rename is ignored and keeps the old title (`:41-44`).
- Add: `TextField::on_submit` and the Add button call `Model::add`, which
  refuses titles that are empty after trimming (`:125-137`, `:227-231`).
  Remove, Move up and Move down act on the active task only when the filter
  shows it (`:140-169`), and are enabled through `selected_shown` (`:184-187`,
  `:233-237`). Moves step over hidden rows using the filtered rows.
- Activating a row (Enter or double click) defers `TextCommand::BeginEdit` on
  its title (`:206-215`). Drag reorder writes the vector (`:216-224`).
- Shortcuts are registered as Application-scope commands (`:258-272`):
  `KeyChord::primary('n')` (Ctrl on Linux) focuses the new-task field,
  `Delete` removes, and Alt+ArrowUp/ArrowDown move.

## Per-item results

### 1. Adding, removing, checking and reordering update the list and the status line — **met**

- Code: `src/lib.rs:125-137`, `:147-169`, `:36-38`, `:188-191`, `:216-224`.
- Tester evidence: `tests/headless.rs:34-91` `add_remove_check_reorder`. It
  adds with Enter and with a button click, refuses empty titles, clicks a
  checkbox, selects with a click, moves with the Down/Up buttons, removes with
  Remove, and asserts on the vector, on the rows (`address_children`) and on
  the status label's semantics name.
- Verifier evidence (`tests/acceptance.rs`):
  - `vector_drives_rows` covers the frozen check that "removing an item from
    the vector removes the row". It deletes from, pushes into and reverses
    the `Vec` signal directly. The inspected rows and the status follow, and
    row 2's address no longer resolves. A pushed `done` record shows as a
    checked checkbox (`SemanticValue::Bool(true)`).
  - `status_follows_check_uncheck_remove` covers the frozen check that "the
    status line follows checking and removing". It checks two rows, unchecks
    one (the inspected checkbox value flips each time), then removes a done
    task by button and another by Delete. The status reads 2 of 3, 1 of 3,
    0 of 2, 1 of 2, then 0 of 1.
  - `add_updates_rows` shows that Enter and Add create inspected rows with
    the right title value, that empty titles add nothing, and that the field
    clears after an add.

### 2. In-place rename commits with Enter and is discarded with Escape — **met**

- Code: `src/lib.rs:40-47` (controlled `InlineText`), `:206-215` (activation
  → BeginEdit).
- Tester evidence: `tests/headless.rs:94-135` `rename_in_place`. Arrow keys
  select, Enter opens the editor (focus is on `["2","title","editor"]`), typing
  and Enter commit, typing and Escape discard (the vector and the title's
  `SemanticValue::Text` are unchanged), and a double click opens editing.
- Verifier evidence: `rename_enter_commits_escape_discards`. A double click
  opens the editor. A replaced draft does not reach the vector before Enter,
  and Enter commits it to both the vector and the inspected value. Enter on
  the active row reopens the editor. Escape leaves the vector byte-equal and
  the inspected value unchanged, takes focus away from the editor, and keeps
  the rows intact.

### 3. Filtering hides non-matching tasks, keeps the selection meaningful and does not change the underlying vector — **met**

- Code: `src/lib.rs:182-187`, `:242-246` (`ListFilter`/`CollectionFilter`,
  which matches row `text` from `:52-54` without regard to case, and
  `HiddenActive::Clear`), `:140-145` (actions only on a shown active task).
- Tester evidence: `tests/headless.rs:138-175`
  `filter_hides_without_changing_tasks`. The query "SHIP" shows only row 3,
  the vector is unchanged and the selection is kept. "milk" hides the
  selected task, which clears the selection. Alt+Down on the only shown row
  changes nothing. Clearing the query restores all rows in vector order.
- Verifier evidence: `filter_keeps_vector_and_meaningful_selection`. The test
  types the mixed-case query "PlA" into the library search field, which shows
  only row 2 with the vector unchanged. The hidden selection is cleared, and
  Delete and Alt+Up then change nothing. The queries "i" and "IT" give the
  expected rows with the vector unchanged. Arrow keys select the shown row 3,
  and Delete removes exactly that task. Clearing the query shows the rest in
  vector order.

### 4. The keyboard shortcuts work — **met**

- Code: `src/lib.rs:258-272`.
- Tester evidence: `tests/headless.rs:178-225` `keyboard_shortcuts`. The test
  presses `Driver::chord(KeyChord::primary('n'), KeyN)` with the list focused,
  and focus moves to the new-task field. Delete inside that text field edits
  the text and removes no task. Alt+Down and Alt+Up move the task with the
  list focused and with the Add button focused. Delete with the list focused
  removes the selected task, and the status updates.
- Verifier evidence: `shortcuts_from_other_focus`. The test uses an explicit
  `Modifiers { control: true }` chord, so it is Ctrl+N and not only "primary".
  Ctrl+N works from the search field, the Add button and the list. Alt+Down
  and Alt+Up work with the Remove button focused, including no-ops at both
  ends, and are checked in both the vector and the inspected rows. Delete
  with the Add button focused removes the selected task.
  `filter_keeps_vector_and_meaningful_selection` and
  `status_follows_check_uncheck_remove` also cover Delete. All keys go
  through `testing::Driver::key` and `chord`, which run the runtime's
  `command_key_input` path.

### 5. A headless check exercises items 1 to 4 through simulated input — **met**

- The tester's `tests/headless.rs` drives every item with `testing::Driver`:
  clicks, `replace_text`, `type_text`, `key`, `chord` and `advance`. It reads
  results through `address_children`, `semantics`, `focused` and the
  library's `ListSelection` signals, plus the `Vec` signal for the underlying
  list. It never calls the application's model functions, which are
  private. `layout_validates` checks the layout validation report, and
  `capture_after_edits` renders through `CpuRenderer`. All 6 pass in the
  acceptance sandbox.
- The tester's suite already covered every item through simulated input. The
  verifier tests add the frozen vector-to-row direction, unchecking, explicit
  Ctrl, and Delete under a filter.

## Missing checks / limitations

- No native window run and no human usability claim. Pointer drag reorder
  (`reorderable`, `src/lib.rs:216-224`) was not exercised by either suite,
  and the task does not require it.
- Selection is read from the library's `ListSelection::active()` signal. The
  semantics snapshot has no per-row selected flag, so selection was not
  inspected through it.

## Usability observations (not defects)

- Each row shows a separate "Done" label next to its checkbox (see capture).
  This is cosmetic.
- FRICTION.md reports: `Runtime::find` returns `Option` and needs a hand-made
  `RuntimeError`, and `ListDropPosition` is borrowed and non-exhaustive, so
  the list's own move logic cannot be reused from buttons or commands. These
  are for triage and are not graded here.

## Aggregate

Items 1–5: all met. Aggregate **met**. The session end time is recorded, so
`accepted=2026-10-28T17:28:50+01:00`.

## Evidence log index

`evidence/01-cargo-version.log`, `02-build.log`, `03-tester-tests.log`,
`04-features.log`, `05-acceptance-tests-attempt1.log`, `06-final-suite.log`,
`07-library-source.log`, `08-clippy.log`, `09-original-hash-check.log`.
