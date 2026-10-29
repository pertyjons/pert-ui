# Round 3 menu keyboard/pointer ownership — repair 2 evidence

Author-run evidence for the second repair of the menu ownership patch in
`/home/per/pert-ui-usability/r3-menu-worktree` (HEAD `b9e4c6c`, uncommitted).
It addresses both P2 findings of `/home/per/pert-ui-usability/r3-menu-codex-review1/result.md`.
This is author evidence, not a review, an approval, a workspace gate or native
or human-input evidence. The previous evidence,
`/home/per/pert-ui-usability/r3-menu-implementation-evidence`, was only read.
Its tree hash, recorded in `00-baseline-hashes.txt` before any edit and again in
`final/hashes.txt`, is unchanged (`08fb06c7…`).

## Layout

- **`00-baseline-hashes.txt`** — HEAD, status, the source hashes before the
  repair (equal to the previous `final/hashes.txt`) and the previous evidence's
  tree hash. `baseline-sources/` holds copies of those files.
- **`scripts/run.sh`** — runs every cargo/rustfmt command. It writes
  `runs/NN-<label>.log` and never overwrites one. Each log records the command,
  cwd, `CARGO_TARGET_DIR`, HEAD, status, source hashes and mtimes before the
  run, the full output, hashes after the run and the exit status.
- **`scripts/mutate.py`, `scripts/negative-control.sh`,
  `scripts/all-negative-controls.sh`** — the negative controls.
  - The `.round1.sh` copies are the versions that ran round 1.
  - Round 2 only added the `TAG` directory suffix.
- **`negative-controls/<mutation>[-round2]/`** — one directory per control:
  - the exact backups and `mutation.diff`;
  - hashes before the mutation and after the restore;
  - mtimes of the mutated and the restored files;
  - `restore-check.txt` and `failed-tests.txt`.

  `all-run{1,2}.out` hold the console output of each round.
- **`final/`**:
  - `tracked.diff`, the `git diff` of tracked files;
  - the final test file;
  - `runtime-since-previous-evidence.diff` and `tests-since-previous-evidence.diff`;
  - `since-negative-controls.diff`;
  - `hashes.txt`.

`CARGO_TARGET_DIR` was `/home/per/pert-ui-usability/r3-menu-implementation-target`,
the previous menu implementation's own build cache, reused. It is not evidence.

## Restore safety (the earlier stale-build incident)

`negative-control.sh` restores the backups by rewriting the bytes (`cat >`, not
`cp -p`) and then `touch`es every restored file. It fails unless:

- the restored hashes equal the backed-up ones;
- every restored mtime is newer than the mutated file's.

All 24 controls report `restored: identical hashes, newer mtimes`. Every
control log and every final build log (runs 34, 38, 44, 47) shows
`Compiling pert-ui-runtime`, so no build reused a mutated or stale runtime.

## Attempts (all kept)

| Runs | What | Result |
|---|---|---|
| 01–02 | Format the test file; first run of the new tests | 33/34. My Up case shifted R05, the repeat's row, under the pointer: a test-design error |
| 03–04 | rustfmt had reflowed the lines my scripted edit targeted, so the edit did not apply; run 04 repeated the unchanged source | same failure (wasted run, kept) |
| 05–06 | Fix applied with the edit tool; format; run | 34 pass |
| 07 | After the Menu/ContextMenu/Select rustdoc rewrite | 34 pass |
| 08–19 | Round 1: 12 mutations | all caught, restores identical. Combined loops hid which case caught which mutation. `clear_on_every_activation_press` escaped the pointer-first test, because motion legitimately takes ownership back |
| 20–21 | Tests split per case. The pointer-first test now uses layout under the still-pressed pointer instead of motion. A meaningless focus assertion was removed before the run | 44 pass |
| 22–33 | Round 2: the same 12 mutations against the split tests | matrix below, restores identical |
| 34–40 | Whole surfaces binary, `fmt --all --check`, runtime and facade Clippy `-D warnings`, doctests of the changed runtime files, rustdoc `-D warnings` for runtime and facade | all clean; surfaces 508 pass; 17 doctests pass |
| 41–47 | Final rerun of the same checks after one more rustdoc correction ("arrow" overclaimed Left/Right repeats and Select Left/Right) | all clean; surfaces 508 pass |

Commands (all with `--locked` where cargo accepts it):

- `cargo test -p pert-ui --test surfaces [menu_pointer_ownership]`
- `cargo fmt --all --check`
- `cargo clippy -p pert-ui-runtime --all-targets -- -D warnings` and the same
  for `-p pert-ui`
- `cargo test -p pert-ui-runtime --doc -- src/menu.rs src/menu_bar.rs src/select.rs src/activation.rs`
- `RUSTDOCFLAGS="-D warnings" cargo doc -p pert-ui-runtime --no-deps` and the
  same for `-p pert-ui`

Not rerun: the full workspace tests and gates, the facade guide doctests,
runtime lib/integration tests, API listings and native checks. Runtime code is
unchanged since the previous evidence, which ran them. Only comments and
rustdoc changed (`final/runtime-since-previous-evidence.diff`; a filter for
non-comment `+`/`-` lines is empty).

`final/since-negative-controls.diff` is the only `select.rs` change after the
round-2 mutation base (`00b9a6f0…`): one rustdoc line. `menu.rs`, which no
control mutates, received the same wording fix. `menu_bar.rs` and
`activation.rs` are identical to the base.

## Negative controls, round 2 (runs 22–33)

Each mutation removes or widens exactly one ownership transition (`mutate.py`).
In the Fails column, "armed …" abbreviates the `an_armed_enter_or_space_keeps_…`
tests.

| Mutation | Fails |
|---|---|
| `arming_keeps_ownership` (an armed Enter/Space does not take the highlight) | 6 tests: armed menu across motion, wheel, layout, repeats+motion; armed Select across motion, wheel. Clicks need the press guard below |
| `move_ignores_armed_key` | 5 tests: armed menu across motion, repeats+motion; armed Select across motion; the previous armed and Escape tests |
| `press_ignores_armed_key` | 3 tests: armed menu across a click; armed Select across a click; previous armed test |
| `wheel_ignores_armed_key` | 3 tests: armed menu across the wheel; armed Select across the wheel; previous armed test |
| `key_arms_over_pointer` (shared activation: a key arms during a pointer press) | Enter/Space during a pointer press |
| `clear_on_every_activation_press` | Enter/Space during a pointer press; Shift+Enter |
| `separator_or_disabled_motion_ignored` | motion onto a separator or a disabled row |
| `repeat_keeps_ownership` (only a press, not an OS repeat, takes it) | the OS-repeat test |
| `clear_on_navigation_release` | Down's release after motion |
| `clear_on_modified_navigation` | navigation with modifiers |
| `clear_on_ignored_key` | an ignored letter; also the modifier chord tests, because a modifier key is itself an ignored key |
| `clear_on_sideways_release` | Right's release after motion |

Round 1 (runs 08–19) caught all 12 as well. Its failures are listed in each
control's `failed-tests.txt`.

## What the tests read

Every assertion reads public results:

- the highlighted row from `Driver::semantics` items;
- row geometry from the semantic item bounds;
- the menu rows' recorded actions;
- the Select's value signal and its `on_change` callback values;
- whether the menu is shown.

No assertion reads the private ownership flag or mirrors the implementation.
Ownership is observed by its effect: a refresh after rows move under a
stationary pointer (a redescription, header layout or wheel scroll) either
carries the highlight to the row under the pointer or leaves it.

## Limits

- Headless only: no native, injected-input, Windows/macOS or human-input claim.
- Idle Escape's preservation is not observable through a menu or Select: Escape
  dismisses the list, which clears ownership through the visibility change.
- Left's press clear remains without an isolating test (previous evidence):
  every Left action hides the level. The Left/Right release cases cover Right
  only.
- A pure refresh between an Enter/Space press and its release is covered by
  layout moving the menu (and wheel scrolling). Redescribing rows cancels the
  armed gesture by existing design, so it cannot serve as that case.
