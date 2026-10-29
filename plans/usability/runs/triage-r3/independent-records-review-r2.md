**Verdict: approved, for the Round 3 records only.** That covers the recurrence matrices (md/json), the metrics and the new-findings ledger. It does not approve the whole round, the 0.3.92 integration or any runtime fix. I used only Read, Glob and Grep, with no edits, builds, execution or delegation.

The checkpoint is `main` at 17c6c3d plus the working tree from the start of this session.

## The six corrections from review 1

1. **R2-13 and BX2 are fixed.** The group N row is now HC1 P, HC2 P, HX1 —, HX2 —, BC1 —, BC2 —, BX1 —, BX2 P, and md and json agree.
   - The builder sets these overrides explicitly (`build-matrices.py.txt:43-44`).
   - I checked BX2 against its source: it renders `Label::new("")` (`main.rs:304`), reads the raw `.name` (`:476`) and asserts `None` (`:524`, `:638`).
   - The original reason at `initial-notes-paired.md:245` is left unchanged, and the matrix's correction section supersedes it.
2. **The timed bases are proper.**
   - **C5 R2-26:** `Job::status` (`lib.rs:306-313`) maps Cancelled and Failed to the reached fraction. The tester's own `cpu_capture_of_jobs_in_several_states` test runs it through `four_states()` (`tests/monitor.rs:213-230`, `:277-296`).
   - **C3 R2-15:** the tester's tests cover Escape restore (`tests/picker.rs:90-125`), focus loss through `clear_focus` into the real `cancel_input` (`:127-141`, `src/picker.rs:205`) and the focus ring (`:257-290`).
   - **X3 R2-15:** the correction says explicitly that its P does not establish `cancel_input`, and F12 stays —.
3. **F20 now uses one rule.** H/F20 is — in every column (override at `:46`). The group N P cells meet the stated rule of actually adopting and using an item-page example:
   - HX1's `ctrl()` (`main.rs:6-14`) copies the `KeyChord::new` doctest pattern (`commands.rs:41-66` in the head kit). The run read that page (FRICTION 16:50) and drives the chords through `Driver::chord`.
   - BC1 (FRICTION 16:54) and BC2 (FRICTION 17:38, "worked first time") adopted the `Tabs::reorderable` example.
4. **BC2 R2-4 is now —** (override at `:45`), with the correction stated. The stars on BC2 R2-1, BC1 R2-4 and the baseline R2-28 cells are kept.
5. **The P9 and P14 status is current.**
   - **P9:** the full gates have 11 zero exits (`full-gates/outcomes.json`), and API generation is recorded clean. The review r1 verdict stays "needs changes", its condition was discharged in `review-repair/author-adoption.md`, and the repair was committed at d479516. An integrated review is still owed.
   - **P14:** it now names the r1 gap and the repair. The Hidden/retry sentence is at `runtime/content_search.rs:398-399`.
6. **The command docs a consumer can read now say actions may flush.** `register_command` (`commands.rs:585-588`) and `invoke_command` (`:1061-1063`) both say the action may call operations that flush, and that a recursive call to the same command returns `BorrowConflict`.
   - At session start `commands.rs` was not modified in the working tree. It is in `focus-recipes/integrated/final-source-hashes.json`. The `commands-doctests` run (7 passed) and the strict `final-rustdoc` run both exited 0, and CHANGELOG 0.3.91 records the change.
   - Ledger row 30 now says correctly that this was not fixed at 0.3.89.

## Other checks

- **Metrics are unchanged:**
  - The rows and medians match what r1 approved: Claude 8.05/12.25, Codex 5.50/6.65, which I recomputed from the rows.
  - Neither metrics file was in the modified list at session start.
  - The no-causal and no-human-claim wording is intact.
- **md vs json:** every cell I compared across both groups agrees, and the run mapping is unchanged.
- **Recurrence statement:** "Only F3 recurs at the frozen head" still holds: C3, X3 and C5 in group H, HC1, HX1 and HX2 in group N.
- **Records left as they are:** the raw reports, acceptance criteria and frozen-head comparison are preserved. No acceptance-only result became a timed P, and no post-repair result is presented as a blind run.
- **Pending work is marked as pending:** the MultiSelect, whitespace, menu and tab-ink rows are explicitly pending or checkpointed and are not claimed fixed by older patches.

## Optional notes (not blocking)

- The BX2 R2-13 correction has no line references. Adding `main.rs:304, :476, :524, :638` would match the convention that every P cites run-specific evidence.
- The P9 ledger row could say that the full-gate runner's extra source guard exited 1. It did so on a TODO prose-only change, which `reveal-diagnostics/README.md:47-55` discloses, so this would only make the ledger self-contained.
- The md/json status lines still say "independent recheck pending". Updating them after this approval is the author's step.

**Limits:** I did not recompute any SHA-256 hashes. Without Git I could not check which commit added the `commands.rs` text; my evidence is the clean session-start status, the integrated logs and the CHANGELOG. I found no new issue outside this scope.
