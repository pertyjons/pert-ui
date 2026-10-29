# Round 3 menu keyboard/pointer ownership repair — implementation evidence

Author-run evidence for the implementation of the menu part of
`plans/usability/round-3-design-menu-keyboard-diagnostics.md` (revision 3,
approved by `runs/review-r3/menu-diagnostics-design-review-r3.md`), in the
worktree `/home/per/pert-ui-usability/r3-menu-worktree` at HEAD `b9e4c6c`,
uncommitted. This is author evidence, not review, approval, a workspace gate or
native/human input evidence.

## Layout

- `00-baseline-hashes.txt` — HEAD, clean status and source hashes before any edit,
  and how the target directory was created.
- `scripts/run.sh` — every cargo/rustfmt command ran through it. It writes
  `runs/NN-<label>.log` (never overwriting): command, cwd, `CARGO_TARGET_DIR`,
  HEAD, `git status`, source hashes before and after, full output, exit status.
- `scripts/mutate.py`, `scripts/negative-control.sh` — the negative controls (below).
- `negative-controls/<mutation>/` — per control: exact backups, `mutation.diff`,
  hashes before and after restore, `restore-check.txt`, failing test list.
- `debug-right/` — one temporary instrumentation of `select.rs` (see the incident
  below), with backup, output and identical restore hashes.
- `final/` — `tracked.diff` (the `git diff` of tracked files), a copy of the new test
  file and the final hashes.

Target directory: `/home/per/pert-ui-usability/r3-menu-implementation-target`, a
btrfs reflink copy of the idle `r3-text-implementation-target`, so the other
implementation's target was not written.

## Attempts and results (all kept)

| Runs | What | Result |
|---|---|---|
| 01–03 | Runtime lib/integration and every facade integration binary after the source change, before new tests | all pass |
| 04–05 | New test file: borrow errors (my test code) | compile failure, corrected |
| 06–07 | One test assumed Escape only cancels an armed Enter; the runtime's `surface_escape` dismisses the menu first (existing behavior) | 1 failure, test corrected to assert the actual dismissal |
| 08 | 26 new tests | pass |
| 09 | Negative control `old_gating` (pre-repair gating restored) | 21 of 26 fail, showing parked-pointer choice (`Gamma` for `Alpha`), stalls (`R04` for `R14`, `O07` for `O12`), title switches on refresh/repeat |
| 10–16 | Targeted mutations | each caught by its specific tests (table below) |
| 17–19 | `cargo fmt --all --check`: only the new file differed; `rustfmt --edition 2024` on that file only; recheck | clean |
| 20–21 | `cargo clippy --locked -p pert-ui-runtime --all-targets -- -D warnings`, same for `-p pert-ui` | clean |
| 22–23 | Runtime `--lib --tests`; facade `--tests` (all area binaries) | all pass (surfaces 490) |
| 24 | Runtime doctests of `menu.rs`, `menu_bar.rs`, `select.rs`, `activation.rs` | 17 pass |
| 25 | Facade doctests of the guides and the crate root | 104 pass |
| 26–27 | `RUSTDOCFLAGS="-D warnings" cargo doc --locked --no-deps` for runtime and facade | clean |
| 28–29 | Mutations `no_editable_clear`, `no_sideways_clear` against 26 tests | both survive (see below) |
| 30–31 | Two tests added; run 31 failed **on a stale mutated build** (incident) | invalid run |
| 32–34 | Repeats: pass, but possibly the instrumented build (incident) | not relied on |
| 35 | After `touch`, forced rebuild, `--nocapture`: no debug output | 28 pass |
| 36–39 | `no_sideways_clear` and `no_editable_clear` against 28 tests; `no_reset_clear`; `no_editable_or_reset_clear` | see below |
| 40–42 | Final `fmt --all --check`, facade Clippy, surfaces binary | clean; 492 pass |

Runtime source hashes in runs 20–27 equal the final ones; only the test file
changed after them, which runs 40–42 cover.

### Negative controls

| Mutation | Fails |
|---|---|
| `old_gating` | 21/26 |
| `record_after_editable` (positions recorded only while editable) | revoked/restored Select |
| `title_ignores_leave` | title round trip, content round trip |
| `bar_wide_position` (one position for the whole bar) | title round trip, content round trip |
| `no_key_armed_guard` | armed Enter/Space paths, armed-Enter release recovery |
| `any_wheel_gains` (no-op wheel takes ownership) | wheel test |
| `clear_on_rows_changed` (clear on changed menu rows/bound options) | menu rows-changed, bound Select options |
| `press_sets_no_highlight` | press highlight, branch click then Right/Left |
| `no_sideways_clear` | (run 29: none of 26) run 36: Right on a row without a submenu |
| `no_editable_clear` | none (runs 28, 37) |
| `no_reset_clear` | none (run 38) |
| `no_editable_or_reset_clear` | revoked/restored Select, disabled/re-enabled Select (run 39) |

The editable-false clears and the `input_reset` clear back each other up:
the runtime cancels input (`input_reset`) after an update that makes a component
unfocusable or moves focus away, so each alone is not observable through public
input; removing both is caught. The clears follow the approved design.
`on_left`'s clear is not isolated by a test: every current Left action hides the
level (`visibility_changed`) or closes the chain.

Every control restored the exact bytes (`restore-check.txt`: identical hashes).

### Incident: stale build after restore (runs 31–34)

`negative-control.sh` restored backups with `cp -p`, which keeps the backup's
older mtime, so Cargo kept the last (mutated) build when no other runtime file
changed. Run 31 therefore ran the `no_sideways_clear` mutant, which the new Right
test correctly fails; the source hash in its log is the restored one. The
`debug-right` restore had the same effect, so runs 32–34 may have run the
instrumented build. The script now `touch`es restored files (current
`negative-control.sh`; the version used for runs 09–29 is
`negative-controls/negative-control.runs-09-29.sh`). Run 35 is the forced rebuild.
Runs 09–29 are unaffected: each mutation rewrote a file, and the rustdoc edits
after run 16 gave both runtime files new mtimes before runs 20–27.
`negative-controls/mutate.runs-09-16.py` is the mutation script as used for runs
09–16; later mutations were only appended.

## Not covered here

No workspace gate, API listing, CHANGELOG, guide recipe, reveal-diagnostics change
or native injected-input check was run or written; the integrating author owns
them. No native, Windows/macOS or human-input claim follows from these headless tests.
