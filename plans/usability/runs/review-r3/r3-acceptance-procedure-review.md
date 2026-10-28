The prompt is **not ready yet**. The sandbox rules, the never-repair rule, the requirement that checks go through library facilities, and the way it defers to the frozen task 6 notification rule are all sound. I found three blocking defects and several smaller ones. As instructed, I read only the three files, so the questions about `run.sh`, `summarize.py` and `conditions.env` are open, not confirmed defects.

## Blocking

1. **Nothing checks that the right kit was used (prompt lines 9, 17–22).** `{kit}` is filled in by hand, and the prompt never compares it with what the run actually used. If the head and baseline kits get swapped, one of two things happens. The submission might fail to build, which looks like a submission failure. Or it might build and be graded against the wrong library, which is worse for the paired task 6 comparison. Fix: before copying, read `RUN_DIR/conditions.env` and confirm that its kit path/commit and the kit's packaged version (0.3.88 head, 0.3.75 baseline) match KIT. If they don't, stop and record a protocol deviation.

2. **The result files may be written to the wrong place.** `acceptance.md` step 4 says the results go to "the run's `acceptance.md`" and "the run's `acceptance.env` for `summarize.py`". The prompt writes both to `ACCEPTANCE_DIR` and forbids editing the run. Unless `summarize.py` reads `ACCEPTANCE_DIR`, or `ACCEPTANCE_DIR` sits inside the run in the place it expects, acceptance results won't be counted. Confirm against `summarize.py` and state the location explicitly.

3. **Grading of incomplete submissions isn't defined, so separate sessions will diverge.**
   - The aggregate isn't defined. Nothing says when the result is `partly` and when it is `not-met`, so two fresh sessions can grade the same state differently. That is a grading asymmetry across runs.
   - Unbuildable or timed-out submissions aren't covered. `acceptance.md` says "Unfinished submissions are evaluated the same way, item by item", but the prompt never mentions them. It also doesn't say how to grade code that is implemented but can't be driven because the submission doesn't build.
   - "Per-numbered-item" (line 37) doesn't name its source. The task's numbered items and the checks in the task-specific paragraph of `acceptance.md` are different lists. The prompt should say to grade the task's numbered items and apply every paragraph check to the item it governs.
   - A missing end time isn't handled. A killed session may have no end time in `conditions.env`, and the prompt doesn't say what to do then.

   Suggested rules:
   - An item is **met** only when it is implemented and a passing check drives it through the library.
   - It is **partly met** when it is implemented but its check is missing or failing, or when it is only partially implemented.
   - It is **not met** when it is absent or wrong.
   - An unbuildable submission cannot meet any item that needs a check.
   - The aggregate is `met` when every item is met, `not-met` when no item is met, and `partly` otherwise.
   - Without an end time, write no `accepted=` line and record the gap.

## Moderate

4. **The original submission is protected by instruction only.** Nothing proves it stayed unchanged.
   - Have the session record a checksum manifest of `RUN_DIR/work` before copying and re-check it at the end.
   - Copy symlinks as symlinks rather than following them. Following them could pull in files from outside the run, including credentials.
   - Treat any symlink that points out of the copy as a deviation, since a test could reach the original through it.
   - Require `ACCEPTANCE_DIR` to be new or empty. `acceptance.md` says "fresh"; the prompt only says "separate".

5. **Build features are missing.** Procedure step 1 says to build and test "with the task's features". The prompt doesn't mention features, and task 6's PNG depends on the CPU renderer.

6. **The task 1 wording differs from the frozen check.**
   - "Native close admission" (line 33) reverses the frozen check, which is "close refusal goes through the library's close request". Say to exercise refusal with unsaved changes and admission after resolving them.
   - "Refused draft followed by Revert" isn't in the frozen task 1 paragraph. Confirm it comes from a numbered task item; otherwise it is a criterion added after the freeze.

7. **The task 6 restatement leaves out the two checks most likely to be skipped.** One is "the author's own folder, with a non-`.txt` file and names out of creation order"; the tester's tests can't satisfy it by definition. The other is "an unmodified tab closes with no dialog". Either list every check that only the author can run, or drop the partial list and point to the paragraph.

8. **Task 4 has no method.** The prompt doesn't say how to establish "all pass against the unchanged app" (for example, by diffing against the kit's original app). Line 26 may also be read as forbidding the deliberate break from being reproduced temporarily in the copy. Clarify both.

## Minor

- "Never edit the repository" (line 4) conflicts with writing `ACCEPTANCE_DIR` if that directory is inside the checkout. Add an exception for it.
- "Containing exactly `acceptance=…`" followed by "add `accepted=`" reads as a contradiction. Say the file contains exactly one or two lines.
- `{task}` needs to supply both the task number (used on lines 31 and 33) and the task file path ("Read the task").
- Excluding `.git` goes beyond the procedure's exclusions (`target/`, `.claude/`). It's harmless, but `acceptance.md` should record it as a deliberate difference.

Once 1–3 are fixed and 4–8 are clarified, I'd consider it ready.
