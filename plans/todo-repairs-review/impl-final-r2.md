needs fixes

Static review of `587d99f` and `8f763d8`: no new production defect established. Most requested coverage is added, but the complete picker-hit assertion remains missing, and some presentation assertions are weaker than their descriptions.

| Finding in `impl-final.md` | Status | Assessment |
|---|---|---|
| 4b public extension parity | Resolved | Earlier repair retained. |
| 4b disabled-result docs | Resolved | Earlier correction retained. |
| 4b picker search coverage | **Partly** | Row target and stale-continuation checks added; complete hit sequence still unasserted. |
| 4b `report_items` cost wording | Resolved | Earlier correction retained. |
| S4-1 modality on continuation | Resolved | Earlier repair retained. |
| S4-2 picker search policy | Resolved | Earlier repair retained. |
| S4-3 item/row ordering | Resolved | `ItemOwner` genuinely nests the list; expected fields, text and targets span small and large steps. |
| S4-4 reporter geometry | Resolved | Earlier repair retained. |
| S4-5 AlertDialog redescription | Resolved | Earlier repair retained. |
| S5 serde migration | Resolved | Earlier repair retained. |
| S5 gallery bounds validation | Resolved | Earlier repair retained. |
| S5 typed reconciliation | Resolved | Earlier repair retained. |
| S5 R19 failure/pending coverage | Resolved | Earlier repair retained. |
| S5 example placeholder keys | Resolved | Earlier repair retained. |
| S5 review-record whitespace | Resolved | Scoped `git diff --check` passes. |
| S6 zero-width border metadata | Resolved | Earlier repair retained. |
| S6 metadata committed after paint | Resolved | New fault test should catch moving the whole record before the focus ring; execution remains unverified. |
| S6 R17/R20 verification | **Partly** | New fixtures help; presentation assertion gaps below remain. |
| S6 near-limit PNG round trip | Resolved | Earlier repair retained. |
| S6 FrameImage documentation | Resolved | Earlier correction retained. |
| Progress closure accuracy | **Partly** | Updated accurately about additions, but overstates completion of the assertions. |

“Retained” means these commits leave the previously reviewed repair unchanged, not that I repeated its entire original audit.

**Medium — Complete picker result sequence still missing.**
[popup_items.rs:553](/home/per/github/pert-ui/crates/pert-ui/tests/collections/popup_items.rs:553) partitions hits into items, Button names and ListBox reports. Those checks permit reordered hits and additional hits outside those subsets. The new mounted-row target check is useful, but does not complete the prior request. Compare the complete ordered result against explicit expected fields, text, roles and targets, including any deliberately searchable caption descendants.

**Low — “Current results only” is checked only for filtered subsets.**
[popup_items.rs:460](/home/per/github/pert-ui/crates/pert-ui/tests/collections/popup_items.rs:460) filters to ListBox hits; line 474 filters to item hits. Stale mounted-row hits could survive without failing either fresh-search assertion. The final target check merely excludes the old list rather than requiring the current list. Assert current list/row targets and absence of old-result hits across the full result set. The two `Stale` assertions themselves are meaningful.

**Low — Icon identification is a geometry heuristic.**
[disabled_button.rs:704](/home/per/github/pert-ui/crates/pert-ui/tests/presentation/disabled_button.rs:704) describes square masks as distinguishing icons from text. That is not a rendering invariant: text masks can be square, and icon masks follow glyph coverage and need not be square. The nonempty and singleton assertions prevent an empty match from passing, but do not establish mask identity. Identify the fixture’s Info coverage through a reference mask or controlled comparison, and describe the helper’s assumptions explicitly.

**Low — Chip-label and override assertions do not independently prove their claims.**
[disabled_button.rs:773](/home/per/github/pert-ui/crates/pert-ui/tests/presentation/disabled_button.rs:773) accepts any muted drawing within the chip, including its close icon. The subsequent allowed-color check also passes if the label disappears while the close icon remains. Identify and assert the label’s coverage separately. At [line 820](/home/per/github/pert-ui/crates/pert-ui/tests/presentation/disabled_button.rs:820), foreground inspection is checked only for Select; assert MultiSelect’s foreground override too.

**Low — Progress wording exceeds the evidence.**
[todo-repairs.md:1016](/home/per/github/pert-ui/plans/todo-repairs.md:1016) treats the requested coverage as complete despite the gaps above. Also qualify line 1027’s absolute “no public operation can” statement: the fixture lacks a public deterministic fault trigger, while text painting remains fallible after frame painting. The new changelog bullets otherwise accurately describe the test intentions. Recorded negative-control execution was not verified in this review.

The test-only hook is acceptable under AGENTS.md: [shapes.rs:509](/home/per/github/pert-ui/crates/pert-ui-runtime/src/shapes.rs:509) gates both the private fault module and stroke interception behind `cfg(test)`. Its thread-local mutable state is narrowly scoped fault injection, not a production ownership model or consumer-required privileged path. It adds no production runtime branch or public API. Scoped restoration would improve cleanup on panic.

The failed-paint test changes the background, injects failure at the later focus-ring stroke, checks the previous background, then verifies successful retry records the new background. It should catch the targeted whole-record ordering regression for both anchors. It does not independently test preservation of every record field; compilation, execution and mutation sensitivity need a run.

The branch is **not yet ready to merge from a code-review standpoint** until the remaining picker assertion requirement is completed and the presentation evidence is tightened.

I did not run Cargo, tests, Clippy, doctests, API generation, packaging or native checks, or validate recorded run results. I did not re-audit unrelated repairs or backends. No files were edited.