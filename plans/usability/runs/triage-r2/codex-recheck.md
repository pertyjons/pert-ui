The revised triage supports recurrence aggregation and the task 5 comparison, with acceptance/completion-time conclusions still conditional on the notification decision. Static review only: I ran no Cargo commands and edited nothing.

**Disposition of my previous review**

| Previous point | Recheck |
|---|---|
| G3’s missing `Semantics` finding duplicates F3 | **Resolved.** H-C2 is `recurred`; NF-6 is withdrawn. |
| T1-X F3 was an unsupported pass | **Resolved:** `not reached`. |
| T3-X F33 conflated `SizeLimits` with `Size` | **Resolved:** `passed`; the example improvement is separate N11. |
| L4a supposedly invalidated `TaskReceipt`’s warning rationale | **Resolved.** G1 preserves shutdown/panic cancellation and treats reconsideration as optional. |
| Numeric semantic value described incorrectly | **Resolved.** G2 now correctly branches on the saved value. |
| “No projection” ignored `map` | **Resolved:** narrowed to writable field projection. |
| Successful conversions should produce F11 passes | **Rejected with adequate evidence.** Successful conversions worked at baseline; all four matrices consistently use `not reached`. |
| Task 4 should pass F19/F28 through fixture execution | **Rejected with adequate evidence.** Neither operation was tester-authored; both task 4 columns are `not reached`. |
| H-X1/H-X2 F29 passes were masked by fixed row heights | **Resolved:** both `not reached`. G1 also applies the exercise requirement to its unmeasured empty label. |
| F20/F23 needed a consistent recurrence rule | **Resolved.** Exposure alone is rejected for F20; B-C2’s actual unnamed `AmbiguousKey` makes F23 recur. |
| Missing H-X2 receipt warning and H-C2 error-construction failure | **Resolved:** both have diagnostic-derived entries. |
| T3-C discarded a failed cancellation restore | **Resolved:** separate implementation-error row, with F12’s pass qualified. |
| Initializer-slot coverage omitted T3-C and H-C1/C2 | **Resolved:** code-derived cross-references added. |
| Header-testing coverage omitted head | **Resolved:** H-C2 added; H-C1 also documented from code. |
| Idle-query coverage omitted H-X1 | **Resolved:** added as acceptance-derived evidence, including correction of the acceptance note’s false import restriction. |
| T2-X should count toward collection-order friction | **Rejected with adequate evidence.** Its own code found ordered children; bounds sorting was the author’s precaution. |
| Content search was described as semantics-only | **Resolved:** collection `ContentRows` acknowledged. |
| Cancellation-notifier conformity overstated removal-error reporting | **Resolved:** mounted-component limitation and untested failure paths stated. |
| Severity merging obscured differing tester experiences | **Resolved:** G4 declares its within-group rule and preserves the recorded severities. |
| Task 4 F9 should remain passed | **Rejected with adequate evidence under the author’s explicit scope.** Both runs show the implicit-container/ordered-example problem or workaround. |
| Baseline F3 inferred from imports; F16 across task 5 | **Retained appropriately.** Baseline F3 inference is labelled; F16 remains the display-control case on both commits. |
| Plain-Label notifications were categorical tester errors | **Resolved as triage:** both are explicitly pending the same maintainer decision. |
| F13 passes might falsely demonstrate adoption of `EventContext::spawn` | **Resolved:** both task 5 groups explicitly qualify what those passes establish. |

The proposed merged-findings accounting is now usable: F3 is removed from the new findings, unchanged baseline behavior is identified, and cross-group occurrences retain their provenance. For F9, G2 confines N1 to structural-wrapper documentation; the ordered-row recipe belongs to reopened F9.

**Consistency and remaining issues**

The author decisions are applied consistently to every affected matrix cell: all 16 F11 cells, both task 4 F19/F28 pairs, both task 4 F9 cells, and baseline F20/F23. Author acceptance checks support passes where they actually exercise behavior; fixture execution receives no discovery credit.

I found no new blocking classification or counting error. One nonblocking wording residue remains: G3’s `Semantics` entry (`~/pert-ui-usability/triage-r2/group-3.md:102`) says it is “only in `inspection`”; it is also exported from the root and `component`. Its absence from the two promised globs—and the F3 recurrence—remains correct.

The notification ambiguity is consistently recorded. Until the maintainer decides, accepted-completion times for the two X2 runs remain conditional. That does not prevent the recurrence comparison.

**Recurrence counts read from the matrices**

These count finding/run pairs, including code-inferred occurrences.

| Finding | G1: tasks 1–2 | G2: tasks 3–4 | G3: task 5 head | G4: task 5 baseline | Tasks 1–4 combined |
|---|---:|---:|---:|---:|---:|
| F1 | 0 | 0 | 0 | 4 | 0 |
| F2 | 0 | 0 | 0 | 4 | 0 |
| F3 | 1 | 3 | 1 | 2 | 4 |
| F9 | 0 | 2 | 0 | 0 | 2 |
| F10 | 0 | 0 | 0 | 1 | 0 |
| F16 | 0 | 0 | 4 | 4 | 0 |
| F23 | 0 | 0 | 0 | 1 | 0 |
| **Total** | **1** | **5** | **5** | **16** | **6** |

Every other finding—**F4–F8, including F7a/b/c; F11–F15; F17–F22; and F24–F34**—has **zero recurrences in every matrix**.

Thus tasks 1–4 have **6 occurrences across 2 findings**. Task 5 has **16 occurrences across 6 findings on 359122e**, versus **5 occurrences across 2 findings on aca8df3**. These support the observed comparison; they do not establish that every fix was exercised or that issues reported only at head originated there.

Verdict: ready.