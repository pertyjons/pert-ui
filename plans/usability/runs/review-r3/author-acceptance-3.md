# Author review: Codex's paired notes-editor submissions

2026-10-28, Codex. Independent reports, added input/inspection tests, final
outputs, source/copy differences and CPU captures reviewed. All four meet the
same sealed task-6 criteria; these are headless AI results, not native or human
usability evidence.

| Submission | Disposition | Basis |
| --- | --- | --- |
| r3-6-head-codex-1 | met | Independent folder with unsorted creation order and non-notes, pointer/keyboard opens and reopening, actual typing with line breaks, both Save routes and failures preserving marker/text, all invalid names, valid disk creation, all close choices and no-dialog unmodified close. Actual menu pointer helpers confirm the hovered row before pressing and assert resulting app effects. Both final suites pass; CPU two-tab capture viewed. Submitted src/main.rs is byte-identical at the final checks: the temporary test entry was removed in favor of a separate include! test wrapper. |
| r3-6-head-codex-2 | met | Own folder, every shortcut and pointer menu route, typing/status/disk effects, refused and boundary names, creation, all close outcomes and unmodified no-dialog checks. Pointer helper scans semantic highlight then presses/releases. Final full suite passes; CPU capture viewed. Original and copy production sources unchanged. |
| r3-6-baseline-codex-1 | met | Same frozen criteria on 0.3.75. Own folder; real Tab/arrow opening and pointer opening, all menu commands clicked, typing, refusal/disk outcomes, all close choices and no-dialog case. Toast existence comes from inspection role/count; source and CPU capture establish displayed I/O error under the sealed interpretation. Final full suite passes; two-tab capture viewed. Only src/main.rs has an appended cfg(test) module hook, reviewed line by line; archive retains it and its diff. |
| r3-6-baseline-codex-2 | met | Own folder and real Tab traversal, menu highlight plus pointer press/release, typing with literal tab and line breaks, canonical disk bytes, all invalid/boundary names and close outcomes, including failed Save retaining tab. Final suite and included probes pass. Failure CPU capture viewed and shows the actual message with both tabs and one modified. Only src/main.rs gains the reviewed appended cfg(test) module hook; no application behavior changes. |

The original source manifests and run-log hashes are verified again at adoption.
Author comparison of every existing Rust file and Cargo.toml in the acceptance
copies found only the two documented test entry points. No production submission
was repaired. Added tests and every retained hook/diff are archived separately.
The permitted hooks affect test builds only and use the same startup path.

The old toast-message inspection limitation remains separately recorded; its
capture/source evidence satisfies the frozen criterion, not a retroactive
relaxation. Menu/tab geometry and stable lookup gaps are library findings under
the first repair checkpoint. Replacing text in original tester tests was not
counted as proof of typing: the independent tests supply real typed input.

The divider drag is supplemental and not needed to meet a numbered item. One
baseline verifier did not exercise it; that missing supplemental check is
preserved. No native window or human usability claim follows from these grades.
