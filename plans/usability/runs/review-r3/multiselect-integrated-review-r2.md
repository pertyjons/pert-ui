**Verdict: I approve the declared integrated MultiSelect/popup-focus scope. Review1's blocker and all five recording conditions are discharged, and I found no new blockers.** One condition stays open, as you said: the clean committed-tree check, which can only run after the commit.

This was read-only. I couldn't hash files, so byte identity rests on the archived reconstruction and diffs plus reading the current source. I didn't re-audit the source that review1 already approved.

## Blocker: Escape step — fixed
- **Current source matches the archived diff.** `apps/gallery/src/interaction_check.rs:1108-1146` is the same as `review-repair1/gallery-review-repair.diff`.
- **First Escape:** the step flushes, then asserts `open == Bool(true)`. It then gets the search field through `exposed_part(&chooser, "search")`, finds the component with that exact inspection id, and asserts its value is `Text("")`.
- **Second Escape:** the step asserts the chooser is closed and the bound value is unchanged at `[piano, violin, bass, cello, flute]`.
- **The step doesn't assert that the query was non-empty before the first Escape.** It doesn't need to. The documented contract (`multi_select.rs:1315-1317`) says Escape on an empty query closes the popup. The default `EachToggle` mode doesn't change that (`:1356-1357`). So `open == true` after the first Escape is only possible if the `flu` query was still there. The step can no longer pass on the weaker behaviour.
- **The row helper is now exact.** `interaction_check.rs:109-135` collects the inspection ids of visible Checkbox components with the given name, then walks the chooser's children comparing `inspection_id`. Chips can't match: the only Checkbox role in `multi_select.rs` is the row (`:770`), and chips report `SemanticRole::Label` (`chip.rs:465`). The duplicate-key ambiguity is gone, and nothing can retarget to a chip.
- **The module order is fixed.** `foundation/main.rs:39` now has `mod popup_press_focus;` in alphabetical order, and the diff shows only that move.

## Review-repair1 evidence
- **The reconstruction is real, not just claimed.** `reconstruct-before.py.txt` reverses exactly the two diff regions and the module move, then asserts the results equal the full-gate manifest hashes. The results are honestly labelled as retrospective.
- **Only the two test files differ from the full gate.** `source-before.json` lists just the gallery file and `foundation/main.rs` as different. `multi_select.rs` is still `b6843e9c…`, so production bytes are unchanged since the 11-gate run. `source-after.json` (11:57:15) confirms nothing changed.
- **fmt, gallery Clippy and the focused foundation run all exit 0.** The foundation run passes 13 `popup_press_focus` tests.
- **Native GPU and CPU both pass with the new assertions.** `native-review-repair1/run1.log` shows 39 steps passing on each, including the "Escape clears search then closes" step. The run (11:54:55–11:56:09) finished inside the window that `source-after.json` confirms was unchanged.

## Review1 conditions
1. **API:** `api/outcome.json` records exit 0 at 11:48:58–11:49:58 and an empty `git status --porcelain -- api/`. The repair touched only the gallery app and the order of a test module, so the API listing can't have changed.
2. **Native correlation:** the README states that the original native run has no manifest of its own. Its link to the source is only that it ran inside the 11:44:21–11:51:38 gate window.
3. **Archive and verification row:** the native runs are archived, `platform.json` records GNOME Shell 50.6, and `docs/verification.md:661` has the 0.3.93 row. The row gives the platform, says the adapter wasn't reported, says other checks were skipped, and makes no claim about OS adapters, human testing or Windows/macOS.
4. **README:** it is current, not "work in progress".
5. **Overflow:** the README and the verification row both say the outside-ScrollArea path is shown only by headless and focused tests, not by the native run.

The repo copy of review1 has the same text as the external `result.md`.

## Minor record gaps (not blocking)
- **`gallery-headless.log` has no command line, timestamps or exit record.** It isn't in `run-checks.py`, so the before/after source records don't cover it. The native runs execute the same steps inside the covered window, so the corrected assertions are still shown to pass.
- **fmt (done 11:54:58) and Clippy (done 11:54:59) finished before `source-before.json` was written (11:55:18).** For those two, "no intervening edit" rests on the author's statement plus the reconstruction. The README's "just after job launch" wording is accurate.

I didn't review the whitespace implementation, the final whole-goal assessment, or anything about native OS adapters, human testing, other platforms or performance.
