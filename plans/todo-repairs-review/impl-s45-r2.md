needs fixes

Statuses below reflect static inspection, not test execution.

| Prior finding | Status | Assessment |
|---|---|---|
| S4-1: resumed items bypass modality | Resolved | Rechecks the owner’s current window modality before emitting saved items; continuation test exercises this. |
| S4-2: picker indexes previous results | Resolved, with regression | Self-contamination is prevented, but the exclusion affects every QuickPick. |
| S4-3: item/row continuation coverage | Partly | Added-item freshness is covered; the required nested collection and independent ordering assertions remain missing. |
| S4-4: reporter-specific geometry tests | Partly | Displaced Accordion header, variable-width InlineChoice and stripe scrolling are covered; partial clipping is not asserted. |
| S4-5: AlertDialog redescription | Partly | Tests retained IDs, new names and accept activation; workspace paths and cancel activation after redescription remain unchecked. |
| S5: serde return-type migration | Resolved | Explicit conversion preserves the bounds check. |
| S5: gallery bounds validation | Resolved | Both builders reject out-of-range indices before arithmetic. |
| S5: typed reconciliation coverage | Resolved | Direct `From<C>` exercises initial children, changed properties, stable identity and one mount. |
| S5: R19 failure/pending coverage | Partly | Failed receipt and pending collapse covered; queue refusal remains absent. |
| S5: example placeholder keys | Resolved | All cited example constructions migrated. |
| S5: review-record whitespace | Resolved | Combined fix range passes `git diff --check`. |

Remaining and new findings:

- **Should-fix — [quick_pick.rs:1107](/home/per/github/pert-ui/crates/pert-ui-runtime/src/quick_pick.rs:1107): the gallery repair changes every picker’s search behavior.** An open picker containing application values, such as the gallery’s named colors, now yields no surface content hits—even with hidden content included. This also affects `CommandPalette` through composition. The anchor’s searchability cannot override an excluded descendant. Scope exclusion to the search presentation, or provide an explicit per-picker policy. The new QuickPick rustdoc describes the exclusion, but its rationale assumes every picker mirrors other searchable content; the general content-search contract and manual omit this exception. I found no existing repository caller explicitly relying on searching an open picker, so this is a public behavioral regression, not an identified failing repository test.

- **Should-fix — [content_search.rs:1422](/home/per/github/pert-ui/crates/pert-ui/tests/collections/content_search.rs:1422): the ordering test remains weaker than R15 requires.** The item owner and collection are siblings; no component combines matching items with `ContentRows`, and no nested collection exists. Comparing tiny steps against the same implementation’s unbounded output lets both produce the same incorrect order. Counting 40 row texts also cannot establish uniqueness. Add the specified combined/nested fixture and assert an explicit sequence of fields, targets and row keys, including completion.

- **Should-fix — [tool_window_modes.rs:1352](/home/per/github/pert-ui/crates/pert-ui/tests/surfaces/tool_window_modes.rs:1352): a failed receipt does not test queue refusal.** Completing an accepted request with `Failed(Cancelled)` exercises receipt handling. A full queue instead makes `detach_view` fail synchronously and enters `request_detachments`’ separate error branch. Fill the queue and assert that error, Docked mode, preserved expansion, presentation-origin reporting and eventual docked presentation.

- **Should-fix — [pointer_items.rs:770](/home/per/github/pert-ui/crates/pert-ui/tests/surfaces/pointer_items.rs:770): partial stripe clipping remains unverified.** The test checks bounds presence and clicks the final button after scrolling to the end. It never establishes a partially clipped button or checks its clipped rectangle. Add an intermediate offset with explicit partial/full clipping assertions and a click within the visible portion.

- **Should-fix — [transients.rs:1378](/home/per/github/pert-ui/crates/pert-ui/tests/surfaces/transients.rs:1378): the redescription assertions omit part of the requested contract.** Stable IDs do not independently prove stable workspace paths. Capture both paths before redescription, compare afterward, and exercise cancel as well as accept using separate openings.

The ItemScan modality recheck introduces no defect I could establish. The pending-collapse test’s host assumption is sound: [windows.rs:66](/home/per/github/pert-ui/crates/pert-ui/src/platform/windows.rs:66) flushes before `prepare_detach`, which rejects hidden views. The test simulates host rejection rather than proving automatic request cancellation. The `#[cfg(feature = "native")]` AppError test gate correctly matches the module and export gates, including `desktop-cpu`.

I examined both fix diffs and relevant surrounding implementation, tests and contracts. I did not execute Cargo, regenerate API listings, verify recorded gate results, examine native rendering, or audit unrelated repairs and deferred popup geometry. No files were edited.