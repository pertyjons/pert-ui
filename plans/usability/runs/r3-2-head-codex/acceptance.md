# Acceptance: r3-2-head-codex (task 2, task list)

Independent acceptance verifier: fresh Claude session (claude-opus-5-5), 2026-10-28.
Procedure: `plans/usability/acceptance.md` (task-specific paragraph "2. Task list"),
round rules `plans/usability/round-3-plan.md`. All builds and tests ran through
`run.sh exec /home/per/pert-ui-usability/kit-r3-head /home/per/pert-ui-usability/acceptance-r3-2-head-codex <cmd>`
in the sandbox, against the kit's vendored pert-ui 0.3.88 only (offline, `--locked`).
This is headless library-driven evidence. It makes no claim of native execution or human usability.

**Verdict: met.** All five acceptance items are met.

## Protocol checks

| Check | Result |
|---|---|
| Kit `kit.env` vs run `conditions.env` | Identical: commit `32dd9c4f667b50cfbbef7e46b18c0007e2b08ee2`, dirty=0, version 0.3.88, `rustc 1.99.1 (58cf8f9c3 2026-10-13)`, prepared 2026-10-28T16:41:41+01:00 |
| Sandbox toolchain | `cargo 1.99.1`, `rustc 1.99.1 (58cf8f9c3 2026-10-13)` (`evidence/01-toolchain.log`) |
| Locked library | `Cargo.lock`: pert-ui, pert-ui-core, pert-ui-runtime, pert-ui-wgpu all 0.3.88 |
| Session end time | `ended=2026-10-28T17:02:23+01:00`, exit_status=0, outcome=ended. Brief was delivered at 16:59:16, so the session lasted about 3.1 min of the 90 min budget |
| Copy | `RUN_DIR/work` was copied with rsync, excluding `target`, `.git`, `.claude` and `.codex`. There were no symlinks and no credentials or CLI homes. The copy's hashes match `original-source-manifest.json` (`evidence/copy-sha256.txt`) |
| Original after session | Re-hashed at the end and identical to the manifest (`evidence/07-final-build-diff.log`). Nothing was written to RUN_DIR |
| Protocol deviations | None observed |

## Build and test commands and results

| # | Command (via `run.sh exec KIT ACC …`) | Result | Log |
|---|---|---|---|
| 1 | `cargo build --offline --locked` (original source) | ok. 1 warning: `dead_code` on `TaskList` fields that only the tests read | `evidence/02-tester-build-test.log` |
| 2 | `cargo test --offline --locked` (tester's tests only) | ok: 1 passed (`tests::full_headless_user_session`) | same |
| 3 | `cargo tree --offline --locked -e features -i pert-ui` | Only the `default` features (`default → desktop → gpu → native`). The task requires default features, and `cpu` is optional and unused | same |
| 4 | `cargo test --offline --locked -- --nocapture --test-threads=1` (with verifier tests), attempt 1 | 2 passed, 6 failed. **All failures were verifier-test errors**, not app defects. My helper read `Semantics.items` on the list, but the VirtualList reports `Choice{selected,count}` and per-row title semantics instead. Also, two clicks on the same title in a row without advancing time were treated as a double-click, which began an edit | `evidence/03-acceptance-attempt1.log` |
| 5 | Semantics probe (temporary `src/acceptance_probe.rs` and its mod line, since removed; source preserved as `evidence/acceptance_probe.rs`) | Showed the list's semantics shape (`role: List`, `value: Choice{selected, count}`, `items: []`) and that a row element has no semantics of its own | `evidence/04-semantics-probe.log` |
| 6 | same as 4, attempt 2 | 7 passed, 1 failed. Another verifier assumption: beginning an edit selects the whole title, so typing replaced it rather than appending. The tester's own test relies on the same behavior | `evidence/05-acceptance-attempt2.log` |
| 7 | same as 4, **final** | **ok: 8 passed, 0 failed** (7 verifier tests + 1 tester test) | `evidence/06-acceptance-attempt3.log` |
| 8 | `cargo build --offline --locked` (final copy) and `diff -r` against the original | ok. The only differences are the added test file and a test-only module line | `evidence/07-final-build-diff.log` |

## Added verifier tests (acceptance copy only)

- `work/src/acceptance_tests.rs`: new file, preserved in the copy.
- `work/src/main.rs`: test-only entry point appended after the original line 390. The original 390 lines are byte-identical:
  ```rust
  // Verifier test entry point (acceptance copy only; test builds only).
  #[cfg(test)]
  mod acceptance_tests;
  ```
  This line changes no application behavior: it compiles only under `cfg(test)`. No production code was changed.
- Captures: none. The task does not require a capture, and the submission does not enable `cpu`.

Each test builds the submitted `app()` with `App::headless()`. It drives the app only through `pert_ui::testing::Driver` input: `click`, `key`, `chord`, `replace_text`, `type_text`, and `advance` to separate single clicks. In two places the test focuses the list directly with `runtime_mut().focus`, as setup only, before keyboard input. Results are read through inspection:

- `address_children` for mounted row order;
- each title's `semantics().value`;
- the list's `Choice.selected`;
- the status label's semantics `name`;
- a checkbox's `Bool` value and a button's `disabled`;
- `runtime().focused()`.

The tests also read the app-owned `Signal<Vec<Task>>` and the library `ListSelection`. The only direct vector writes are in `acc_vector_drives_rows_and_status`, which checks that the vector drives the rows. No application model function (`add`, `remove`, `move_selected`, `edit`) is called.

| Test | What it establishes |
|---|---|
| `acc_vector_drives_rows_and_status` | **Task check "list driven by the Vec"**: `retain` on the vector removes the row: its address is gone, the titles are `Alpha, Charlie` and the status is `0 of 2 done`. Pushing a done task and reversing the vector re-orders the rows to `99,3,1`, sets the status to `1 of 3 done` and ticks the new row's checkbox |
| `acc_check_uncheck_remove_reorder_status` | Checking and unchecking move the status 2→1. Removing a selected done task with the Remove button gives `0 of 2 done`. Move up at the top and Move down at the bottom do nothing, and Move down swaps. The selection follows the task's identity |
| `acc_single_selection_pointer_and_arrows` | Clicking selects one task, and clicking another replaces it. Up arrows navigate and clamp at the first row. The `selected` set has size 1 |
| `acc_inline_rename_enter_escape` | Double-click starts an in-place edit. The vector is unchanged while the draft is open, and Enter commits it. Escape discards the draft, so the vector and title semantics stay unchanged. The next edit starts from the committed title. Renaming in a filtered view changes the right task |
| `acc_filter_selection_and_vector` | Mixed-case filters match (`mIlK`, `charlie`). A selected task that stays visible stays selected. Arrows move among visible rows only (3→1, skipping hidden 2). When a selected task is filtered out, the selection is cleared and Remove is disabled. Delete and Alt+Up then leave the vector unchanged. A filter with no matches empties the list, and clearing it restores all rows. The vector is equal before and after every filter step |
| `acc_keyboard_shortcuts` | Ctrl+N focuses the new-task field from both the search field and the list, and does not insert an "n". With the list focused, Alt+Down twice and Alt+Up reorder the vector and the rows, are no-ops at the bottom, and keep the selection. Delete removes only the selected task and updates the status |
| `acc_observe_delete_in_text_field` | Observation only (not graded; it prints its result) |

## Per-item results

| Item | Result | Source evidence | Check evidence |
|---|---|---|---|
| 1. Adding, removing, checking and reordering update the list and the status line | **met** | `src/main.rs:49-68` (`add`: trim, refuse empty); `:69-81` (`remove`); `:82-106` (`move_selected`); `:107-125` (`reorder` for VirtualList Alt+arrows); `:27-29` (controlled `Checkbox` writes the vector); `:137-142` (`ListSource` mapped from the `Signal<Vec<Task>>`); `:150-156` (status `"{done} of {len} done"` derived from the vector) | Tester: `tests::full_headless_user_session` (`main.rs:255-389`). Verifier: `acc_vector_drives_rows_and_status`, `acc_check_uncheck_remove_reorder_status` |
| 2. In-place rename commits with Enter, discarded with Escape | **met** | `src/main.rs:31-34` (`InlineText::controlled(...).in_row(true)` writes the vector on commit); `:160-169` (Enter/activate → `TextCommand::BeginEdit` on the row's title) | Tester: `main.rs:310-328`, `356-362`. Verifier: `acc_inline_rename_enter_escape` |
| 3. Filtering hides non-matching tasks, keeps selection meaningful, vector unchanged | **met** | `src/main.rs:144` (`ListFilter` over the source, sharing the selection); `:186-188` (`CollectionFilter` with `HiddenActive::Clear`); `:145-149, :74-76, :88-90, :178-182` (`shown` guard: actions and the Delete command act only on a visible selected task) | Tester: `main.rs:339-354`. Verifier: `acc_filter_selection_and_vector`, which adds a retained visible selection, visible-only arrows, and Delete and Alt+Up refused for a hidden selection |
| 4. The keyboard shortcuts work | **met** | `src/main.rs:196-207` (Ctrl+N application command → focus the field); `:209-214` (Delete application command → `remove`); `:170` (`VirtualList::reorderable`: the library maps Alt+Up/Down to a `ListDrop` that `reorder` applies to the vector) | Tester: `main.rs:300-308`, `363-381`. Verifier: `acc_keyboard_shortcuts`, `acc_filter_selection_and_vector` |
| 5. A headless check exercises items 1–4 through simulated input | **met** | The tester's test builds the same `App` with `app().headless()` and uses `testing::Driver` click, key, chord, replace and type input. It asserts row addresses, semantics, focus and the vector, and calls no model mutation function | Tester: `tests::full_headless_user_session` passes (`evidence/02-…`, `evidence/06-…`). Its gaps are covered by the verifier tests: direct vector changes driving rows, unchecking, and a retained visible selection under the filter |

**Aggregate: met.** Every item is met. accepted = session end 2026-10-28T17:02:23+01:00.

## Observations (not graded; usability notes)

- Alt+Up/Alt+Down are handled by the focused `VirtualList`, not by an application-wide command. After clicking the "Move down" button, focus stayed on the button and Alt+Up did nothing. Output: `OBSERVE alt-up-after-button: focused_is_list=false tasks=["Alpha", "Charlie", "Bravo"]`. The task does not specify a scope for this shortcut. When the list has focus, which is where selection happens, the shortcut works.
- Delete in a focused text field edits the text and does not remove the selected task. Output: `OBSERVE delete-in-search: search=Some(Text("")) tasks=["Alpha","Bravo","Charlie"]`. This is sensible precedence.
- Beginning an in-place edit selects the whole title, so typing replaces it. Clicking an already-selected title again within the double-click interval starts an edit.
- In inspection, the VirtualList reports selection only as `Choice{selected: index, count}`, with an empty `items`. Its rows have no semantics of their own; titles are read from each row's child components. My first attempt assumed `items`, which cost one verifier iteration. This is a possible inspection-discoverability lead for triage, not an application defect.
- The friction log reports one compile error, E0271: the guide's `Result<_, RuntimeError>` builder was passed to `App::new`, which expects `AppError`. The tester resolved it with `|ui| Ok(build(ui)?)`.

## Missing evidence

None for the graded items. There is no native-display execution and no capture, and neither is required by task 2.

## Paths

- Report: `ACCEPTANCE_DIR/acceptance.md`; result: `ACCEPTANCE_DIR/acceptance.env`
- Added test source: `ACCEPTANCE_DIR/work/src/acceptance_tests.rs`, plus the 4 appended lines in `ACCEPTANCE_DIR/work/src/main.rs`
- Probe source (not compiled in the final suite): `ACCEPTANCE_DIR/evidence/acceptance_probe.rs`
- Command-output log: `ACCEPTANCE_DIR/evidence/` (`01-toolchain.log`, `02-tester-build-test.log`, `03-acceptance-attempt1.log`, `04-semantics-probe.log`, `05-acceptance-attempt2.log`, `06-acceptance-attempt3.log`, `07-final-build-diff.log`, `copy-sha256.txt`)
