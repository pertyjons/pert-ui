# Acceptance: r3-4-head-claude-retry1 (task 4, testing an existing app)

Independent verifier: fresh Claude context (claude-opus-5-5). The session was
interrupted once by a service usage limit, before anything was copied
(`work/` was still empty), and was resumed in the same acceptance directory.
This report covers headless, sandboxed library-driven tests only. It makes no
claim of native execution or human usability.

**Verdict: met.** All three acceptance items are met.

## Protocol and metadata

| Check | Result |
|---|---|
| Kit `kit.env` vs run `conditions.env` (commit, dirty, version, toolchain, prepared) | Identical: `32dd9c4f667b50cfbbef7e46b18c0007e2b08ee2`, dirty=0, 0.3.88, rustc 1.99.1 (58cf8f9c3 2026-10-13), prepared 2026-10-28T16:41:41+01:00. They also match the `head` entry in `runs/review-r3/prelaunch.json`. |
| Session | outcome=ended, exit 0, brief 17:54:50, **ended=2026-10-28T17:57:32+01:00** (2.7 min) |
| Original source vs controller manifest | All 8 files matched before the copy and again after the final run (`original: True`). Nothing was written to RUN_DIR. |
| Copy | `rsync -a --exclude target --exclude .git --exclude .claude --exclude .codex RUN_DIR/work/ ACCEPTANCE_DIR/work/`. There are no symlinks in the source (checked with `find -type l`). Only the empty `.claude/.cc-writes` dirs were excluded. No credentials were copied. Copy hashes in `evidence/copy-initial-sha256.txt` equal the manifest. |
| Builds | All builds ran through `run.sh exec KIT ACCEPTANCE_DIR bash -c '...'` with `--locked` and the kit's vendored 0.3.88. The harness build log is in `log/builds.jsonl` and `log/builds/`. |
| Features | `cargo tree -e features -i pert-ui` shows `cpu` plus `default` (→ `desktop` → `gpu` → `native`), as required (`evidence/01-tester-suite.log`). |

**Protocol deviations:** none affecting the result. One verifier error was
found and corrected (see "Verifier errors").

## Production source vs fixture

I extracted the fixture with `git show 32dd9c4:plans/usability/task4-app/...`
into `evidence/fixture/` and diffed it (`evidence/fixture-diff.txt`, re-run as
`evidence/fixture-diff-final.txt`):

- `Cargo.toml`: `version = "0.3.86"` → `"0.3.88"`. This is the harness's version substitution and is allowed.
- `src/lib.rs`, `src/main.rs`, `README.md`: byte-identical to the fixture.
- Added: `Cargo.lock` (generated) and `tests/behaviors.rs`, `tests/capture.rs` (test-only).

The friction log's "Application changes: None" is accurate. No testability
change was made.

## Commands and results (evidence log in `evidence/`)

| # | Command (all via `run.sh exec KIT ACCEPTANCE_DIR bash -c "cd /work/bookmarks && ..."`) | Result | Log |
|---|---|---|---|
| 1 | `cargo --version && rustc --version && cargo tree --locked -e features -i pert-ui && cargo test --locked` (unchanged submission) | build OK. behaviors 7/7 pass, capture 1/1 pass, exit 0 | `01-tester-suite.log` |
| 2 | Mutation 1 + `cargo test --locked` | `add_appends_a_trimmed_bookmark_at_the_end` FAILED at behaviors.rs:108 (`"  Docs  "` vs `"Docs"`). 6 pass, 1 fail, exit 101 | `02-mutation1-trim.log`, `mutation/mutation1.diff` |
| 3 | Mutation 2 + `cargo test --locked --test behaviors` | `ctrl_l_clears_the_filter_wherever_focus_is` FAILED at behaviors.rs:280 (`"rust"` vs `""`). exit 101 | `03-mutation2.log`, `mutation/mutation2.diff` |
| 4 | Mutation 3 (verifier's own) + `cargo test --locked --test behaviors` | `each_row_shows_name_url_and_remove_which_deletes_only_that_bookmark` FAILED at behaviors.rs:202 (removing one "Rust" also removed the other). exit 101 | `04-mutation3.log`, `mutation/mutation3.diff` |
| 5 | `cargo test --locked --test verifier_acceptance -- --nocapture` | 2/2 pass, **but invalid**: it ran against the stale mutation-3 binary (see Verifier errors) | `05-VERIFIER-ERROR-ran-against-stale-mutation3-build.log` |
| 6 | `cargo test --locked` | behaviors: 1 FAILED (remove test). **Verifier error**: stale mutation-3 binary, not an application failure | `06-VERIFIER-ERROR-stale-build-after-mutation3.log` |
| 7 | `sha256sum src/lib.rs && cargo clean --locked -p bookmarks && cargo test --locked -- --nocapture` (**final**) | lib.rs sha256 `242b409e…` (= original). Fresh build. behaviors 7/7, capture 1/1, verifier_acceptance 2/2 pass, exit 0 | `07-final-suite.log` |

Mutations (copy only; each was applied with `sed` and restored with
`cp -p evidence/mutation/lib.rs.orig work/bookmarks/src/lib.rs`, then `cmp`
confirmed it):

1. `src/lib.rs:93` `name: entered_name.trim().to_owned(),` → `name: entered_name.clone(),`. This reproduces the tester's recorded break.
2. `src/lib.rs:125` `vec![KeyChord::primary('l')]` → `vec![KeyChord::primary('k')]`. This reproduces the tester's second recorded break.
3. `src/lib.rs:108,113`: Remove keyed by `bookmark.name` instead of `bookmark.id`. This is my own independent break.

After the restore, `src/lib.rs` is byte-identical to the original (sha256
`242b409eab123c76aa1c9180addd31d6d82d402527644520f9c7f33ebbcc504a`, in
`evidence/copy-final-sha256.txt`). The final passing suite (#7) ran on this
restored source.

The tester's own record agrees. RUN_DIR `log/builds.jsonl` shows
`cargo test --test behaviors` exit 101 (trim test failed at :108), then
`cargo test` exit 101 (Ctrl+L test failed at :280), then a final `cargo test`
exit 0.

## Verifier errors (not application defects)

When restoring with `cp -p`, I put back the file's original 17:56 mtime. That
is older than the mutation-3 build fingerprint, so cargo reused the stale
mutated binary for runs #5 and #6. The evidence that this was a verifier
error:

- Only the remove test failed, which is exactly what mutation 3 breaks.
- `src/lib.rs` was byte-identical to the original.
- A forced rebuild (#7) passes.

Both logs are kept under `VERIFIER-ERROR` names. The verifier captures from
the stale build are kept in `evidence/captures/from-stale-mutation3-build/`;
they are byte-identical to the clean-build captures.

Mutation runs #2 to #4 are valid. Each `sed` gave the file a new mtime and
forced a rebuild, and each run failed only its own targeted test (for
example, run #3 passed the trim test). Run #1 was a fresh build. Nothing
remains unresolved.

## Per-item results

### 1. One test per listed behavior, all passing: **met**

All tests are in `work/bookmarks/tests/behaviors.rs`. Each mounts
`bookmarks::build` on a headless `Runtime`, drives it through
`testing::Driver`, and reads results back through `Driver::semantics` and
`address_children`. No test calls an internal app function. I checked the
kit source for how the Driver acts:

- `click` sends pointer press/release at the widget's bounds (`testing.rs:802`).
- `replace_text` focuses the field, selects all, then sends `text_input` or Backspace (`:1002`).
- `chord` sends modifier and key events through `command_key_input` (`:1130`).

| README behavior | Test | Notes |
|---|---|---|
| 1 Add (append at end, trim name, http/https) | `add_appends_a_trimmed_bookmark_at_the_end` (:94) | Checks keys 3 and 4, a trimmed `"  Docs  "`, and both http and https URLs. |
| 2 Refused add | `add_refuses_a_blank_name_or_a_bad_url_and_keeps_the_fields` (:121) | Covers an empty name, a spaces-only name, and name-checked-first. Covers 4 bad URLs including a leading space. Checks both error texts and that the fields keep their text. Checks that the next successful add clears the message. |
| 3 Fields cleared | `a_successful_add_clears_both_fields` (:163) | |
| 4 List and Remove | `each_row_shows_name_url_and_remove_which_deletes_only_that_bookmark` (:175) | Every row has name, url and Remove. Removing works correctly with a duplicate name present. |
| 5 Filter | `filter_hides_names_that_do_not_contain_it_ignoring_case_and_spaces` (:213) | Covers case, surrounding spaces, name-only matching, and blank or empty showing all. |
| 6 Status | `status_counts_all_or_shown_of_total` (:240) | Covers `N bookmarks`, `N of M`, `0 of 3`, blank filter, and `1 bookmarks`. |
| 7 Clear filter Ctrl+L wherever focus is | `ctrl_l_clears_the_filter_wherever_focus_is` (:269) | Tested with focus in the filter, focus in the name field (whose text is kept), and nothing focused. |

All 7 pass in the original run (#1) and the final run (#7).

I added an independent check, `tests/verifier_acceptance.rs`
`verifier_remove_in_filtered_view_and_clear_filter_from_button_focus`, which
passes in #7. It covers:

- Remove pressed in a filtered view deletes only that bookmark.
- Status goes from `2 of 3` to `1 of 2`.
- Ctrl+L with a non-editor (the Add button) focused clears the filter and restores `2 bookmarks`.

This extra check is corroboration only. The tester's tests already
establish every behavior.

### 2. CPU-renderer render of the main window to a PNG: **met**

`tests/capture.rs` mounts the app, drives a refused add, and enables capture
support. It then requests a window capture, serves it with
`CpuRenderer::new().serve_captures`, checks the image is 640×480 and not one
flat color, and writes `CARGO_TARGET_TMPDIR/bookmarks.png`. The test passes
(#1, #7).

The capture is preserved at `evidence/captures/tester-bookmarks.png` (sha256
`301f41ef…`, PNG 640×480 RGBA). Viewed, it shows the whole window: Name/URL
fields, Add, the URL error, Filter, `2 bookmarks`, and two rows with Remove.

The verifier test `verifier_cpu_capture_png_of_main_window` passes and
writes `evidence/captures/verifier-start.png` and `verifier-filtered.png`.
The filtered capture shows `1 of 3 bookmarks` and one row.

### 3. A test fails when a behavior is broken, recorded in the friction log: **met**

`work/FRICTION.md` entry "17:56 — runtime — (acceptance 3 …)" records the
trim break and the Ctrl+L break, with the failing test output and the
restore. Reproduced in the copy as #2 and #3, with the same failing tests
and lines. My independent mutation 3 (#4) was also caught. The source was
restored byte-for-byte and the final suite (#7) passes.

## Added verifier files (in the copy only)

- Test source: `work/bookmarks/tests/verifier_acceptance.rs`. This is a test-only entry point; no application code was changed.
- Captures: `work/bookmarks/target/tmp/{bookmarks,verifier-start,verifier-filtered}.png`, copied to `evidence/captures/`.
- Evidence: `evidence/01…07*.log`, `evidence/mutation/` (original lib.rs and the three diffs), `evidence/fixture/` and both fixture diffs, `evidence/copy-{initial,final}-sha256.txt`.

## Separate observations (not grading)

- No library or application defect was observed.
- Usability notes from the tester's friction log (documentation gaps, not defects):
  - Whether `Form`/`Field` keys are address segments is not documented.
  - The semantic value of an empty editor is not documented.
  - A text-reading helper on `Driver` is requested.
- The session took 2.7 minutes, with no blockers.
