# Author review of the first completed independent verifiers

2026-10-28, Codex. Reviewed reports, per-item checks, public-input helpers,
final command outputs, original/copy hashes and captures before adoption.
These are exploratory AI/headless acceptance results, not native execution
or human usability scores. No production submission was repaired.

| Submission | Author disposition | Basis |
| --- | --- | --- |
| r3-6-head-claude-1 | met | Own unordered folder/non-notes; real list/editor/divider input; every shortcut and pointer menu action; all invalid-name cases, disk writes and deterministic failures; close choices and unmodified no-dialog close; two-tab modified capture viewed. Verifier-only helper assumptions and corrections are disclosed. |
| r3-6-head-claude-2-retry1 | met | Independent own folder, listing and reopen; text/marker/status, disk and failed-save preservation; invalid and boundary names; pointer New/Save/Close, close outcomes and no-dialog case; source untouched and final tests pass. Capture viewed: two tabs, one modified. |
| r3-6-baseline-claude-1 | met | Same frozen criteria against 0.3.75; the visible error Banner qualifies under the sealed notification interpretation. Own folder and pointer menu checks supplement tester keyboard checks. Failure preserves text/marker/tab, close outcomes and disk effects checked; capture viewed. |
| r3-6-baseline-claude-2-retry1 | met | Own folder, all menu actions reached by real pointer input and all shortcuts; invalid/boundary names and disk effects; failed saves preserve modification/tab with notification existence in inspection. Toast message discoverability is a separate old-library observation. Capture viewed: two tabs, one modified. |
| r3-2-head-claude-retry1 | met | Vector-to-row updates exercised deliberately for the source-of-truth criterion; interactive add/check/uncheck/rename/remove/reorder/filter/shortcuts use public input and inspect results. Hidden selection is refused meaningfully. Final tester and independent suites pass; supporting capture viewed. |

For all five, the controller reports exit 0 and original_source_unchanged=True.
Author recomputed every original-source manifest hash and compared every existing
Rust file and Cargo.toml in the acceptance copy with the original: no differences.
The added verifier tests are separate files and will be preserved with their
command outputs. Actual terminal outputs, rather than report prose alone,
establish the final passing runs.

Task-6 menu helpers use real pointer positions from inspected geometry/highlight;
app effects and menu closure establish which command ran. They do not call the
app's model action functions. No requirement here demands that the old library
already support click_item for menu rows; that missing capability is separately
verified and repaired on the working tree.

Further observations remain visible for full independent triage, including the
literal-tab glyph box reproduced by the author. Passing acceptance does not
close usability findings or the persistent goal. The other verifiers and the
full recurrence matrices are still pending.
