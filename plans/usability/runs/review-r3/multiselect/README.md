# Round 3 MultiSelect pointer ownership and click focus — implementation evidence

Author-run evidence for the implementation in
`/home/per/pert-ui-usability/r3-multiselect-worktree` (HEAD `17c6c3d`,
uncommitted) of `plans/usability/round-3-design-multiselect-pointer.md`
revision 3 (main checkout, read only; sha256 in `00-baseline-hashes.txt`),
with the review-1 corrections and review-2 ordering condition and pins adopted.
This is author evidence: not a review, approval, workspace gate, API-listing
gate, native check or human-input evidence. No commit was made.

Changed files (the owned scope only):

- `crates/pert-ui-runtime/src/multi_select.rs` (modified)
- `crates/pert-ui/tests/collections/multi_select_pointer.rs` (new, 29 tests)
- `crates/pert-ui/tests/collections/main.rs` (one `mod` line)

## Layout

- `00-baseline-hashes.txt`: HEAD, clean status, source hashes before any edit,
  design/review hashes, toolchain. `baseline-sources/` holds those exact files.
- `scripts/run.sh`: runs every cargo/rustfmt command and writes
  `runs/NN-<label>.log`, never overwriting one. Each log records the UTC start
  and finish, command, cwd, `CARGO_TARGET_DIR`, HEAD, `git status`, source
  hashes and mtimes before, the full output, hashes after and the exit status.
- `scripts/mutate.py`, `scripts/negative-control.sh`,
  `scripts/all-negative-controls.sh`, `scripts/all-negative-controls-final.sh`:
  the negative controls (below).
- `negative-controls/<mutation>[-round2|-final]/`: the backup, `mutation.diff`,
  hashes before/mutated/restored, mtimes, `restore-check.txt`,
  `failed-tests.txt`. `all-run1.out`, `round2-*.out` and `all-final.out` are
  the console outputs; `final-summary.md` tabulates the final round.
- `temporary-probe-scrollarea.rs.txt`: the probe test run 06 used, then removed.
- `final/`: `tracked.diff`, copies of the final `multi_select.rs` and new test
  file, `status.txt`, `hashes.txt`.

`CARGO_TARGET_DIR` was the new
`/home/per/pert-ui-usability/r3-multiselect-implementation-target`
(a build cache, not evidence).

## Attempts (all logs kept)

| Runs | What | Result |
|---|---|---|
| 01 | Existing MultiSelect tests on the unmodified HEAD source | 35 pass |
| 02–03 | New tests against the unmodified source | Compile error: `text_preedit` is the `HostIntegration` trait method; import added |
| 04–05 | Same, compiling | 21 of 27 fail. 4 of them show focus on `page`: my fixture's overflowing `ScrollArea` (see finding 1) |
| 06 | Temporary probe of that case on the unmodified source | Short list: press focuses `page`, popup closes, click writes nothing; long list: focus `list`, toggles |
| 07–08 | Page made to overflow only in the wheel-bubble test; rerun on the unmodified source | **20 of 27 fail, all on the two target defects** (snap-back/retarget to the pointer row; long-list focus left on the list) |
| 09–10 | Implementation; first run | 60 pass, 2 fail: my test-design errors (12 px page scroll kept the same row under a centred point; after the control's window Leave, the repeated move is a re-entry, an accepted-residual motion) |
| 11–12 | Both tests corrected | 62 pass |
| 13–14 | Comment-style fixes; added the disabled-press test and the denied repeated-position step | 63 pass |
| 15–36 | Negative controls, round 1 | See below; `record-after-denied-return` survived |
| 37–38 | Added `motion_made_while_denied_still_counts_after_the_restore` | 64 pass |
| 39–40 | Round 2 of the two recording controls | Both fail as intended |
| 41–42 | Clippy `-p pert-ui-runtime --all-targets` and `-p pert-ui --test collections`, `-D warnings` | Clean |
| 43 | MultiSelect doctests (`-p pert-ui-runtime --doc multi_select`) | 3 pass |
| 44 | `RUSTDOCFLAGS="-D warnings" cargo doc -p pert-ui-runtime --no-deps` | Clean |
| 45 | Same with `--document-private-items` (exploratory, not a gate) | Fails on 249 pre-existing `redundant explicit link target` notes across the crate; none on a changed line |
| 46 | Whole facade `collections` binary | 495 pass |
| 47 | `cargo test -p pert-ui-runtime` (unit tests and all doctests) | 770 pass, 2 ignored (pre-existing) |
| 48–49 | `rustfmt --check` on the owned files; `cargo fmt -p pert-ui-runtime -p pert-ui --check` | Clean |
| 50–71 | Negative controls, final round, on the final sources | See below |
| 72–73 | Final MultiSelect tests; final Clippy of the test binary | 64 pass; clean |

The runtime source hash in runs 41–47 and 72 is the final one (`b6843e9c…`).
After runs 41–47, only comments in the new test file changed (run 48 onwards,
recompiled in 50–73).

## Negative controls

`negative-control.sh` copies the exact source bytes, applies one named
mutation (`mutate.py`; each replacement must match exactly once), touches the
file and runs the MultiSelect tests. It then restores by rewriting the bytes
(`cat >`, never `cp -p`) and touching the file again. It fails unless the
restored hash equals the backup's, the restored mtime is newer than the
mutated one, and the test log shows `Compiling pert-ui-runtime`. All 22
final-round controls and the round-1/round-2 controls report
`restored: identical hash, newer mtime; mutated build recompiled pert-ui-runtime`,
and the source hash after every round equals the pre-round hash
(`pre-negative-controls-hash.txt`, `pre-final-controls-hashes.txt`).
Before round 2 the script gained only the optional `TAG` directory suffix
(round 1 ran it without one); the restore logic is unchanged.

Final round (64 tests), from `negative-controls/final-summary.md`:

| Mutation | Failing tests |
|---|---|
| `baseline` (HEAD source restored) | 21: the defect reproduction |
| `unconditional-refresh-highlight` (the design's required control) | 19, on highlight, values and focus |
| `refresh-can-own` | 12 |
| `no-search-key-clear` | 12 |
| `row-leave-clears` (the rejected Leave design) | 7 |
| `no-position-memory` | 5 |
| `no-focus-return` | 4 |
| `leave-not-recorded`, `gain-after-highlighted-guard`, `no-panel-permission-clear`, `focus-after-report`, `focus-on-any-release` | 2 each |
| `record-after-denied-return`, `disabled-motion-no-gain`, `disabled-press-no-gain`, `no-panel-exit-clear` (the panel-grow control), `no-query-clear`, `release-no-regain`, `no-lifecycle-clears` | 1 each |
| `no-begin-clear`, `no-close-clear`, `no-visibility-clear` | **0 each** |

The three individually surviving resets are mutually redundant through public
input: every close calls `finish` and hides the panel, and every reopen calls
`begin`, so any one of them suffices. Removing all three fails the
close/hide-reopen test. They are kept because the design requires each
explicitly.

## Findings for the integrating root (not fixed; outside the owned scope)

1. **Verified new defect: a short popup inside an overflowing `ScrollArea`.**
   On the unmodified source (run 06), a primary press on a row of a
   non-scrolling popup list focuses the nearest focusable ancestor. That is
   the scrolling page, outside the popup, so the popup closes at the press
   and the click writes nothing. The design assumes a short list keeps search
   focus, which holds only without a focusable ancestor. The release-time
   repair cannot reach this case, so it is unchanged by this patch. A fix
   needs a design/runtime decision, for example stopping the press-focus walk
   at a surface boundary, or a focusable but non-tab-stop popup part.
2. The existing test name
   `multi_select.rs::a_click_on_a_row_toggles_it_without_moving_focus` still
   passes (short list) but echoes the replaced wording.
3. The guide/manual/CHANGELOG/TODO updates the design lists belong to the
   root: the controls guide needs the pointer-highlight, stationary-refresh,
   wheel/list-key and completed-click focus policy; the P13 TODO entry names
   `Row::pointer_input`, actually `ChoiceRow::pointer_input`.

## Limits

- Headless public-input evidence only; no native, OS, platform, human or
  blind-run evidence. Display scale/zoom coordinate changes are documented by
  construction (a changed logical position is motion) and not tested.
- `Panel::unmounted`'s same-identity guard and the per-update id refresh are
  not distinguishable through public input (the panel is never remounted
  under one `Shared`); the removal-during-capture test covers the behaviour.
- No workspace gate, API listing, MSRV, feature matrix or packaging check was
  run (root's integration). Public signatures are unchanged by inspection.
