needs fixes

Static assessment only; “resolved” does not mean tests passed.

| Report finding | Status | Assessment |
|---|---|---|
| S4-1: resumed items bypass modality | Resolved | Modality recheck remains intact. |
| S4-2: picker indexes previous results / blanket exclusion regression | Resolved | Searchable by default; gallery search explicitly opts out. |
| S4-3: item/row continuation ordering | Partly | Explicit sequence and row keys added; nested collections, mounted descendants and target identities remain unchecked. |
| S4-4: reporter-specific geometry | Resolved | Intermediate scrolling now asserts partial clipping, fully clipped items and visible-part activation. |
| S4-5: AlertDialog redescription | Resolved | Adds path comparisons and cancel activation on a separate opening. |
| S5: serde return-type migration | Resolved | Previous repair retained. |
| S5: gallery bounds validation | Resolved | Previous repair retained. |
| S5: typed reconciliation coverage | Resolved | Previous repair retained. |
| S5: R19 failure/pending coverage | Resolved | Full-queue test covers synchronous refusal, Docked mode, expansion, origin and eventual presentation. |
| S5: example placeholder keys | Resolved | Previous repair retained. |
| S5: review-record whitespace | Resolved | Both requested commit diffs pass `--check`. |
| S6: zero-width border metadata | Resolved | Shared helper returns no border; anchor transition tested. |
| S6: anchors commit metadata before paint succeeds | Partly | Assignments correctly moved after fallible operations; requested failure-preservation regression test absent. |
| S6: R17/R20 verification | Partly | Adds command checks, selected chips, Surface override, zero width and pressed frameless state; gaps below remain. |
| S6: near-limit PNG round trip | Resolved | Independent decoding at exactly 64 MiB in release builds, plus extreme aspect ratios. |
| S6: FrameImage documentation | Resolved | Dependency-free claim removed; temporary allocations described. |

Remaining gaps from the original reports:

- **Medium — [content_search.rs:1543](/home/per/github/pert-ui/crates/pert-ui/tests/collections/content_search.rs:1543):** `Combined` has no child, and its inner list is never mounted. The new fixture therefore still omits nested collection continuation and the transition into mounted descendants. It also discards widget/collection identity when comparing hits. Add the requested nested fixture and compare complete expected targets.

- **Medium — [disabled_button.rs:564](/home/per/github/pert-ui/crates/pert-ui/tests/presentation/disabled_button.rs:564):** command assertions cover Dark mode only; Light mode still checks metadata alone. Adaptive anchor coverage changes only Surface, and icon/rich captions remain absent. A selected MultiSelect chip exists, but assertions do not independently verify its treatment. Complete these anchor-specific checks.

- **Medium — [disabled_button.rs:657](/home/per/github/pert-ui/crates/pert-ui/tests/presentation/disabled_button.rs:657):** unchanged deadlines and zero component updates cannot establish retained-paint reuse or exclude paint/layout invalidation. Enable profiling, inspect repeatedly, then flush and paint; assert no new layout or component paint work.

- **Low — [select.rs:1387](/home/per/github/pert-ui/crates/pert-ui-runtime/src/select.rs:1387), [multi_select.rs:1282](/home/per/github/pert-ui/crates/pert-ui-runtime/src/multi_select.rs:1282):** the metadata timing repair is correct locally, but no test forces a later paint failure and verifies preservation of the previous record.

No additional production defect was established in the two fixes.

I did not run Cargo, tests, API generation, packaging, native checks or rendering. Recorded gate results remain unverified. I did not audit `7cf441c`, subsequent releases or unrelated repairs; intervening code was consulted only where these fixes touch it. No files were edited.