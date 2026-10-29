# Round 3 popup press focus: implementation evidence

This is the author's evidence for the implementation in
`/home/per/pert-ui-usability/r3-popup-worktree`. The worktree is at HEAD
`c19ae70` (0.3.92), uncommitted. The implementation follows
`plans/usability/round-3-design-popup-press-focus.md` revision 2 in the main
checkout (read only), with all five corrections from
`plans/usability/runs/review-r3/popup-design-review-r1.md` adopted.
`00-baseline-hashes.txt` records the hashes of both documents.

This evidence is not:

- a review or approval;
- the workspace gate, the API-listing gate, a native check or human-input
  evidence;
- a commit or a publication.

## Base

The worktree is HEAD plus the isolated, approved MultiSelect patch, copied
byte-identical from main:

- `multi_select.rs`
- `collections/multi_select_pointer.rs`
- the `mod` line in `collections/main.rs`

Those three files are **baseline, not part of this author delta**.
`final/baseline-unchanged.txt` shows they still equal
`/home/per/pert-ui-usability/r3-popup-approved-multiselect-base` byte for byte.

## Author delta (owned files only)

- `crates/pert-ui-runtime/src/runtime/surfaces.rs`:
  - adds the new `pub(super) fn admitted_interactive_surface(root)`;
  - adds one internal test, `only_a_current_interactive_admission_keeps_press_focus`,
    to the existing test module.
- `crates/pert-ui-runtime/src/runtime/pointer.rs`:
  - adds one check in the automatic press-focus walk. It comes after the
    unchanged `focusable()` check and the unchanged direct-target check;
  - extends the `Runtime::pointer_input` rustdoc.
- `crates/pert-ui-runtime/src/runtime.rs`: extends the `Component::focusable`
  rustdoc. This is a public focus doc only; the pointer loop lives in
  `pointer.rs`.
- `crates/pert-ui-runtime/src/popover.rs`: surface focus docs only, on the
  `SurfaceKind` enum and the `Popover` type.
- `crates/pert-ui/tests/foundation/popup_press_focus.rs` (new, 13 tests) and
  its `mod` line in `foundation/main.rs`. No new integration binary was added.

`final/scoped-delta.diff` is the export. `final/apply-check.txt` shows that it
applies to `git archive HEAD` plus the baseline snapshot and reproduces the
worktree files exactly. `final/files/` holds copies of the files and
`final/hashes.txt` their hashes.

The change adds no public API, field, schema, dependency or version change,
and makes no MultiSelect code change.

### The rule as implemented

```rust
if focused.as_ref().is_some_and(|focused| self.descendant(focused.0.serial, current))
    && self.admitted_interactive_surface(current)
{
    break;
}
```

- `focused` is the snapshot taken before the loop.
- `descendant` counts the root itself as inside.
- The query is true only for a record in the current window's
  `self.window.surfaces` that meets all four conditions:
  - `root == current`;
  - `admitted`;
  - `kind.interactive()`;
  - `self.surface_kind(root) == Some(kind)`.
- A root that does not qualify is passed, so the walk continues to an
  enclosing root.

## Layout

- `00-baseline-hashes.txt` and `baseline-sources/` hold the exact source bytes
  before any edit, plus the plan, review, baseline and toolchain hashes.
- `scripts/run.sh` runs every logged command and writes `runs/NN-<label>.log`.
  It never overwrites a log. Each log records:
  - the UTC start and finish;
  - the command, cwd, `CARGO_TARGET_DIR` and HEAD;
  - `git status`;
  - the hashes and mtimes of the owned and baseline files before the run, and
    their hashes after;
  - the full output and the exit status.
- `scripts/mutate.py` and `scripts/negative-control.sh` implement the negative
  controls described below.
- `negative-controls/<name>-round2/` holds each control's backup,
  `mutation.diff`, hashes and mtimes before, mutated and restored,
  `restore-check.txt` and `failed-tests.txt`. `round2-a.out` and
  `round2-b.out` are the console outputs. `negative-controls/baseline-runtime/`
  is round 1 (see run 12).
- `final/` holds the scoped delta, the file copies, the hashes, the status and
  the apply and baseline checks.

`CARGO_TARGET_DIR` was the new directory
`/home/per/pert-ui-usability/r3-popup-implementation-target`. It is a build
cache, not evidence.

### Steps not captured by `run.sh`

- One `cargo check -p pert-ui-runtime` ran right after the first runtime edit,
  before the logging script existed. It used the same target dir and finished
  cleanly. It was a compile smoke check only.
- Read-only orientation commands (`git status`, `git log`, `cat`, `grep`,
  `sha256sum`). One `cd` landed briefly in the main checkout
  `/home/per/github/pert-ui` and ran only `git status` and `git log` there.
  Nothing was written outside the worktree and this directory.
- The scoped-delta apply check ran in a `mktemp` scratch copy, which was then
  removed. Its result is in `final/apply-check.txt`.

## Attempts (all logs kept)

| Runs | What | Result |
|---|---|---|
| 01 | New tests, first compile | Compile error: `MultiSelect::new` is fallible. `.unwrap()` added |
| 02 | New tests on the implementation | 8 pass, 5 fail. All 5 were my expectation errors (see the note after this table) |
| 03–04 | Temporary probes of the focus-within sequence | See the note. Probe tests removed by restoring the pre-probe file bytes |
| 05 | Corrected expectations | 13 pass |
| 06 | Runtime `surfaces` unit tests, including the new one | 5 pass |
| 07–09 | rustfmt check of owned files; formatting applied to the test file; recheck | Clean |
| 10–11 | Facade module; surfaces unit tests | 13 pass; 5 pass |
| 12–13 | Round-1 `baseline-runtime` control | Each looping test stopped at its first failing kind, which hid the other kinds' old results. Tests restructured (next row) |
| 14–17 | Restructuring: every kind is observed, then compared in one assertion. One compile error (`MultiSelectCommit` is non-exhaustive) | 13 pass |
| 18–43 | Negative controls, round 2 | See below |
| 44 | `cargo clippy --locked -p pert-ui-runtime --all-targets -- -D warnings` | Clean |
| 45 | `cargo clippy --locked -p pert-ui --test foundation -- -D warnings` | Clean |
| 46 | `cargo test --locked -p pert-ui-runtime` (unit, integration and all doctests) | All pass; 2 ignored, both pre-existing |
| 47 | `cargo test --locked -p pert-ui --tests` (every facade integration binary) | audit 185, collections 495, controls 371, editing 322, foundation 507, presentation 232, surfaces 514; lib 47 + 7 ignored (pre-existing). All pass |
| 48 | `RUSTDOCFLAGS="-D warnings" cargo doc --locked -p pert-ui-runtime --no-deps` | Clean |
| 49 | Same for `-p pert-ui` | Clean |
| 50 | `cargo fmt --check -p pert-ui-runtime -p pert-ui` | Clean |
| 51 | `git diff --check` | Clean. The new untracked file was also checked separately for trailing whitespace and tabs: none |

**Note on run 02.** The five failures were these:

- Bubbled presses reached non-leaf probes. Now only leaf probes take presses.
- One assertion pinned a modal release outcome. That pin was removed.
- Two expected logs were wrong:
  - Focus moving from `field` up to a focusable `popup` root correctly tells
    `panel` that focus-within is false.
  - In the notification case, focus moving out correctly tells `panel` and
    `popup` that focus-within is false.

The probes in runs 03–04 confirmed that this is consistent with the
`focus_within_changed` contract. It is **not** a finding.

## Actual old results

Run 18 used the full baseline runtime source. The restored-old-walk control in
run 20 gave the same failure values once sorted; it differs only in the
parallel output order. The values below are copied from the assertion output.

| Case | Old (`pointer_input` press result; focus; open; probe log) | New |
|---|---|---|
| Popover, ContextMenu, Submenu with a focused field, row press | `Ok(Ignored)`; `page`; closed; `field focus_lost`, `panel within false`, `popup within false`, `owner dismiss FocusLeft`. The row never got the press | `Ok(Handled)`; `field`; open; `row press` only |
| Dialog, field focused | `Err(ModalBlocked)`; `field`; open; `row press` (delivered before the error) | `Ok(Handled)`; `field`; open; `row press` |
| ApplicationDialog, field focused | `Err(ModalBlocked)`; same as Dialog | Same as Dialog |
| ViewDialog, field focused | `Ok(Handled)`; **`page`** (focus left the dialog); open; `field focus_lost`, `panel within false`, `popup within false`, `row press` | `Ok(Handled)`; `field`; open; `row press` |
| Dialog and ApplicationDialog with nothing focusable (root focused) | `Err(ModalBlocked)`; `popup`; open | `Ok(Handled)`; `popup` |
| ViewDialog with nothing focusable | `Ok(Handled)`; `page`; `popup focus_lost`, `row press` | `Ok(Handled)`; `popup`; `row press` |
| Nested: focus in the menu, press on a submenu row | `Ok(Ignored)` (both surfaces dismissed) | `Ok(Handled)`; `field`; both open |
| Detached window: Popover; Dialog | Popover as in row 1; Dialog `Err(ModalBlocked)` | Both `Ok(Handled)`; `field` |
| Changed to a Notification, then to a Dialog | Notification step identical, as expected; the Dialog step was `Err(ModalBlocked)` | Notification step: walk to `page`; Dialog step: `Ok(Handled)`, `field` |
| Short MultiSelect in an overflowing ScrollArea, EachToggle, after Enter on O01, then hover and press O03 | Press: focus `page`, closed, value `["o01"]`. Release: the same; reports `[["o01"]]`. The click was lost | Press: `search`, open, `["o01"]`. Release: `search`, open, `["o01","o03"]`; reports `[["o01"],["o01","o03"]]` |
| Same in OnClose | Press: focus `page`, closed, value **`["o01"]`** and reports `[["o01"]]`. This is a premature OnClose commit at the press, and the click was lost | Press and release: `search`, open, value `[]`, no report. Down/Enter/typing then work, and the outside press writes `["o01","o03","o04"]` once |

The other five tests pass on both the old and the new source, as intended:

- the focusable panel or root inside the surface still wins;
- the direct-target rule, inside and outside a surface;
- the walk without any surface;
- focus already outside the surface, the documented limit;
- hidden and closed surfaces.

The MultiSelect test asserts that the hovered row (O03) holds the highlight
before the press. The baseline already contains the approved fix for the stale
highlight, so the lost click cannot come from that older bug.

## Negative controls (round 2, runs 18–43)

For each control, `negative-control.sh` does the following:

1. Backs up the exact bytes of `pointer.rs` and `surfaces.rs`.
2. Applies the mutation. Each replacement must match exactly once; the
   `baseline` mutations write the files from `baseline-sources/`.
3. Touches the files.
4. Runs the facade `popup_press_focus` tests, plus the runtime `surfaces` unit
   tests for every mutation except the full baseline.
5. Restores the files by rewriting their bytes (`cat backup > file`, never
   `cp -p`), then touches them again.
6. Checks that the restored hashes are identical to the backups and that the
   restored mtimes are newer than the mutated ones.
7. Requires `Compiling pert-ui-runtime` in the mutated run's log.
8. Reruns the facade tests after the restore. That run must also show
   `Compiling pert-ui-runtime`, and it must pass.

Every control met all of these checks; see each `restore-check.txt`.

| Mutation | Facade result | Unit result | Intended failure |
|---|---|---|---|
| `baseline-runtime` (whole old runtime) | 8 fail | not run | The old results above |
| `old-walk` (baseline `pointer.rs` bytes) | 8 fail, same values | pass (5) | Short-popup click, value and focus; modal `ModalBlocked`; ViewDialog leaving; nested; detached |
| `no-containment` (any admitted root stops the walk) | 1 fail: `focus_already_outside_the_surface_takes_the_old_walk` | pass | Focus stays on `opener` instead of the old walk to `page` |
| `strict-containment` (the root itself is not inside) | 1 fail: `a_dialog_without_focusable_content_keeps_focus_on_its_root` | pass | Root-focused dialogs return to the old results |
| `before-focusable` (check runs before `focusable()`) | 1 fail: `a_focusable_component_inside_the_surface_still_takes_the_press` | pass | A focusable root no longer takes focus |
| `first-root-only` (only the first surface root is considered) | 1 fail: `a_press_on_a_submenu_keeps_focus_in_the_enclosing_menu` | pass | The submenu press walks out as before |
| `no-interactive` | 1 fail: `a_surface_changed_to_a_notification_no_longer_keeps_focus` | 1 fail (notification assertion) | A notification keeps focus |
| `no-admitted` | pass | 1 fail (failed-sync assertion) | Not publicly reachable; see below |
| `no-live-kind` | pass | 1 fail (changed-kind assertion) | See below |

The unit failure lines read one lower than the source lines in the final file
because each mutation removes a line above the assertion.

## Unreachable, redundant or unexercised checks (disclosed, not fabricated)

- **`admitted`.** A press first flushes pending work and then pending layout,
  so it synchronizes surfaces before the walk. A failed synchronization sets
  `layout_pending`. The next press's flush therefore either readmits the
  record or returns the error before the walk. Public input never shows the
  walk an unadmitted record, so only the internal test covers this guard.
- **The live `surface_kind` check.**
  - Public input reaches a stale kind only if an owner changes its
    `child_surface` answer without invalidating layout or changing focus,
    since synchronization runs when layout or surface focus changed.
  - I did not build that state through public tests. The guard is covered by
    the internal test (kind changed without synchronization).
  - Whether such an owner is supported is not stated in the `child_surface`
    rustdoc. This is an observation, not a verified defect.
- **Current-window records only.** The walk visits presentation ancestors of a
  hit in the selected window. A detached root has no presentation parent, so a
  parked window's record can never match a visited node. This is redundant by
  construction. The detached-window test shows the rule applies inside a
  detached native window.
- **Snapshot focus versus live focus.** Nothing changes focus inside the loop
  before it breaks, so the two are equivalent. Not mutated.
- **Order relative to the direct-target check.** Both checks break without
  moving focus, so swapping them cannot be observed. Not mutated.
- **Tab, keyboard traversal and capture routing.** The diff does not touch
  them. Explicit `Runtime::focus`, a handler's `request_focus_for` during the
  press, and outside-press dismissal are tested. Tab and capture got no new
  tests.
- **Hidden and closed surfaces.** These tests pass on both old and new source.
  They show that the pre-press flush leaves no stale surface or focus. They do
  not exercise a guard of the new query.
- **Fixture structure.** Every fixture is one composed root with
  non-overlapping placements. The exception is the detached-window test, which
  needs a primary root plus the detached root.

## Limits

- No workspace gate, API-listing gate, MSRV run, native check or gallery run;
  the root agent runs these later. No public item was added: the query is
  `pub(super)`.
- No changelog, manual, `docs/contracts.md`, guide, TODO or version change;
  the root agent owns those. The places needing updates are:
  - the surface focus paragraph at `docs/contracts.md` around line 984
    ("A nonmodal surface closes when focus moves outside its owner…");
  - the `ModalBlocked` result that is now gone, for presses inside Dialog and
    ApplicationDialog;
  - the TODO.md entry at line 277, which says to remove it when this repair
    lands.
- Headless only: no native or human pointer evidence.
- Documented remaining behavior: focus outside an admitted nonmodal surface
  that has nothing focusable (for example on its opener) takes the old walk,
  and a press on its content can still close it. The rustdoc states this
  qualifier; it does not claim that a press inside a popup never closes it.

## Verified finding for root (outside the owned scope, not edited)

Baseline file `crates/pert-ui/tests/collections/multi_select_pointer.rs`,
lines 102–106, the `Fx` doc comment. It says an overflowing scroll area "is a
focusable ancestor of the popup, which a press on a short list's row focuses
instead of keeping focus in the popup". With this change that no longer holds,
and `a_short_multi_select_in_a_scrolling_page_keeps_its_row_click` shows it.
Rule: AGENTS.md, "When behavior changes, update rustdoc, examples, tests and
affected design notes together". Suggested fix: when integrating, reword the
comment, keeping the filler only where the wheel test needs page scrolling.
The collections binary still passes unchanged (run 47).
