**Verdict: I approve the runtime, test and documentation integration in the declared scope. One gallery step needs a fix before its evidence is adopted: the Escape step claims more than it checks. A few recording conditions also remain.**

This was a read-only review. Without shell access I couldn't run `git diff`, so I compared bytes through the recorded hash chains and the archived copies, and read the source and logs directly.

## Source integration
- **Hashes match the approvals.** Root `multi_select.rs` is `b6843e9c…`, the approved bytes. The four popup files and `popup_press_focus.rs` match `popup/implementation1/final/hashes.txt`. The `pointer.rs` code in root matches the approved scoped diff. `admitted_interactive_surface` is `pub(super)`, so no public API was added.
- **The .89–.92 work is preserved.** The pre-integration hashes of `pointer.rs`, `surfaces.rs`, `runtime.rs` and `popover.rs` equal the committed 0.3.92 gate manifest. That manifest's `multi_select.rs` (`92d35eef`) is also the isolated MultiSelect baseline at 17c6c3d, so .92 never touched that file and copying in the approved bytes reverts nothing. The tabs, menu, Button and reveal files are not modified. Every earlier native gallery step still passes in `run1.log`.
- **Fixture comment:** `multi_select_pointer.rs:102-106` only replaces the outdated comment. Line positions and test counts are identical to the archived `4c2d71df…` copy.
- **Test rename:** `a_click_on_a_row_toggles_it_without_moving_focus` became `a_completed_click_on_a_short_list_keeps_search_focus`, with a matching structure count. `collections/main.rs` equals the approved `47e0b167`. `foundation/main.rs` has the same module set as the approved file.
- **Documentation:** the CHANGELOG, contracts (lines 527-530 and 994-1004), manual (lines 352-357 and 525-529) and controls guide (lines 168-176) match designs rev3 and rev2, including the limit for focus that is already outside the surface. The version is 0.3.93 in `Cargo.toml`, in all 8 `Cargo.lock` entries and in the task4 manifest. No MultiSelect or popup TODO entry is left over.

## Gallery steps (`interaction_check.rs:1040-1111`)
- **Helper:** `multi_select_row` looks rows up by Checkbox role and name in the public semantics, then by key within the chooser. It does not use hard-coded private keys.
- **Long-list click:** the step checks that focus returns to search, then Down and Enter. The check asserts the actual value `[piano, violin, bass, cello]`, and Cello does follow Bass in `INSTRUMENTS`. In the before-repair run this step passed while the short-popup step failed on the same scrollable page. That shows the long-list path really was exercised.
- **Filtered short popup:** typing `flu` leaves only Flute. The step asserts search focus and the value with flute. Before the repair it failed exactly at `short-popup click lost search focus` (`main.rs:521`), and after the repair it passes.
- **Before/after logs and correlation:** both logs are preserved. `initial-correlation.json` is honestly marked `retrospective: true`. Its gallery hash `37b631…` still equals the current source and the final-gates manifest.

## Blocker
**`"MultiSelect: Escape clears search then closes"` only checks that `open == false` after two Escapes.** It would also pass if the first Escape closed the popup without clearing the query. So neither the headless nor the native log is evidence that the query is cleared first. The fix is to assert, after the first Escape, that `open` is still true and the search text is empty, and optionally that the value is unchanged after closing. The alternative is to rename the step to what it checks. Either way, rerun the headless test and the filtered native check, because the native log quotes the old step name.

## Gates
- **Full CI gate:** all 11 commands in `r3-multiselect-popup-final-gates/outcomes.json` exit 0. `source-unchanged.txt` reads "All captured source bytes unchanged during gates."
- **Workspace tests:** the log shows every new popup and MultiSelect test passing, plus `the_interaction_steps_pass_headlessly`.
- **Native check:** the selected `interaction_check` passed on both GPU and CPU (39 steps each). The adapter was not reported, and every other native check was skipped by the filter. Like you said, this is not the full native suite and not OS-adapter, human, Windows/macOS or tab-pixel evidence.

## Conditions before commit
1. **API listing:** the generation result is still pending. Record the actual outcome and a clean `git status --porcelain -- api/`.
2. **Native source correlation:** the native run (11:45:54–11:47:37) has no source manifest of its own. It is tied to the source only because it ran inside the gate window (11:44:21–11:51:38). Say so in the record.
3. **Archive and record the native run:** move it into `runs/review-r3` and add a 0.3.93 row to `docs/verification.md` with the platform and limits.
4. **Stale README:** `multiselect-integrated/README.md` still says "Work in progress" and lists source review, integrated tests, workspace gates and native checks as owed.
5. **Native short-popup claim:** the native run does not assert that the chips page actually overflows at the native window size. Claim the outside-ScrollArea path only from the headless evidence, or add an overflow assertion to the step.

## Minor points (not blocking)
- `mod popup_press_focus;` was appended after a blank line at the end of `foundation/main.rs` rather than in alphabetical order.
- Chips and rows share `option-<hash>` keys, so `multi_select_row` returns `AmbiguousKey` for an option that is already chosen. It fails loudly and never clicks the wrong row, and Bass and Flute are fine. Comparing `ui.inspection_id(&row)` with `component.id` would make the lookup exact.

The final whole-goal assessment and the review of the runtime whitespace change are still owed.
