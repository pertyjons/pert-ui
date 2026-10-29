**Verdict: approve.** The three blocking concerns from `disabled-button-implementation-review-r2.md` are resolved. What remains is minor record-keeping, none of which blocks. This approves the disabled Button checkpoint only; it says nothing about overall usability closure.

## Previous blocking concerns

1. **Final gate not finished: resolved.**
   - `full-gates-final/outcomes.json` lists all 11 base gate commands with timestamps (19:46–19:52 UTC, 2026-10-28). Each exited with status 0: format, Clippy, workspace tests, the five examples, doctests, strict rustdoc and diagnostics.
   - Strict rustdoc used `RUSTDOCFLAGS=-D warnings`, recorded in the outcome's environment field.
   - The workspace test log has a `test result: ok … 0 failed` line for every binary. All 6 `disabled_button` tests passed (`workspace-tests.log:2401-2408`, `:2548`, `:2559`).
   - The runner (`r3-disabled-button-final-gates.py.txt`) hashes the source before running and checks every hash again afterwards. It writes `source-unchanged.txt` only if every hash still matches, and that file exists.
   - This run covers the tightened code. Its hashes for `controls.rs`, `menu.rs`, `theme.rs` and `disabled_button.rs` differ from the initial run's.
   - `final-source-check.json` reports no mismatches against that manifest. Together with your statement that nothing has changed since, the final gate covers the current source.
2. **Evidence README out of date: resolved.**
   - `README.md:29-59` now describes the 6-test review run and its command, `captures-final/` with its provenance, the frameless capture, the `full-gates-final/` location and the API result.
   - It says honestly that the focused command strings were written down afterwards.
3. **Current captures and provenance: resolved.**
   - `captures-final/provenance.json` exists.
   - I viewed both PNGs. They match the current test scene (`disabled_button.rs:220-263`): enabled and disabled Apply, the empty and disabled fields, the disabled icon button, and "No frame" with no fill or outline.

The API gate is also clean. `api-generated-clean.json` records `api/generate.sh` exiting 0 and `git status --porcelain -- api/` printing nothing. The session-start git status also shows `api/` unmodified, and the final manifest includes the `api/` hashes. No feature-matrix or native checks are needed for this appearance-only change.

## Remaining risks (non-blocking)

- **Capture provenance is thin.**
  - The provenance file records only hashes and a description. It doesn't name the run that produced the images, a timestamp or the test file's hash.
  - The test writes to `target/tmp/disabled-button` (`disabled_button.rs:300-302`), and both the focused run and the workspace run overwrite it. So which run produced the copies isn't recorded.
  - Rendering is deterministic and the test is unchanged, so the risk is low. I couldn't compute the PNG hashes myself.
- **Some records have no timestamp or command.**
  - `final-source-check.json` and `api-generated-clean.json` have neither, and `r3-api-final-0.3.90.log` is just a list of files.
  - When the API step ran relative to the final source rests on the corroboration above, not on a direct record.
- **I couldn't open the full test log.**
  - The excerpt leaves out 501,535 overlap diagnostic lines, the same count as the initial run. I couldn't decompress `workspace-tests.full.log.gz` to check its hashes or see whether any `disabled_button` test emitted such lines. The tests pass either way.
  - The flood itself is already tracked in `TODO.md:193-207`.
- **The plan file wasn't updated.** `round-3-design-disabled-button.md:3-7` and `:121` still say final gates and the recheck "follow". It doesn't link the r2 review or `full-gates-final/`. `README.md:3-4` opens with the same wording. Update both along with recording this review.
- **Small notes:**
  - The `rustdoc.log` header omits `RUSTDOCFLAGS`, though `outcomes.json` records it.
  - The runner script writes to `/home/per/pert-ui-usability/r3-disabled-button-final-gates`. The README doesn't say that `full-gates-final/` is a copy of that directory.
- **Optional test gap from r2 is still open.** No test focuses an enabled frameless Button and checks its focus ring. `controls.rs:821-845` handles that case correctly when read, but nothing tests it.

## Limits

This covers Linux headless and CPU rendering only. I make no native, GPU, Windows/macOS or human-input claim. I only read files: I made no edits, ran no commands and delegated nothing.
