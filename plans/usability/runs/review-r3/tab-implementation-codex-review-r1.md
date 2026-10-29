**Needs changes.** The coverage algorithm follows the approved design, but its geometry exception and part of the required font evidence remain unresolved.

1. **P1 — Public position preservation is not established for hanging tabs.**  
   [text.rs:1315](/home/per/pert-ui-usability/r3-text-worktree/crates/pert-ui-runtime/src/text.rs:1315) skips hidden runs before they contribute raster bounds. Those bounds determine `size` and `origin`. The tests then [allow shrinking and subtract each layout’s origin](/home/per/pert-ui-usability/r3-text-worktree/crates/pert-ui-runtime/src/text/tabs.rs:354) before comparing selection geometry.

   This proves Parley-coordinate preservation, but does not prove public-coordinate preservation. Public [caret/selection conversion](/home/per/pert-ui-usability/r3-text-worktree/crates/pert-ui-runtime/src/text_selection.rs:217) adds that origin; hit testing subtracts it. When hanging RTL tab ink previously established the left bound, removing it changes those positions at a fixed paint origin. TextArea also uses `size()` to clamp scrolling.

   **Required:** resolve the logical extent/origin contract, record concrete LTR/RTL wrapped before/after deltas, and test public caret rectangles, selections and hit testing at identical coordinates without normalizing away the origin. Include narrow TextArea selection/reveal evidence. An ink-only size reduction may be defensible, but the current exception does not establish that logical selection and positioning remain intact. I did not execute a reproduction.

2. **P2 — Required rich-text font-boundary coverage is missing.**  
   [The rich-span cases](/home/per/pert-ui-usability/r3-text-worktree/crates/pert-ui-runtime/src/text/tabs.rs:605) vary size, color, gaps and icons, but never font family. [The fixture cases](/home/per/pert-ui-usability/r3-text-worktree/crates/pert-ui-runtime/src/text/tabs.rs:718) use one registered font throughout; their cross-span case changes color only.

   **Required:** combine Body, Monospace and the registered fixture in one layout, with tabs at font boundaries and an icon whose nearest text metrics come from a hidden tab. Compare actual fonts, advances, visible coverage and alignment against the reference, including phase rendering and wrapping. Demonstrate sensitivity to an incorrect tab-font override. Existing evidence does not cover this approved condition.

3. **P2 — Two owned rustdoc descriptions overpromise blank tabs.**  
   [Monospace](/home/per/pert-ui-usability/r3-text-worktree/crates/pert-ui-runtime/src/text.rs:145) and [TextArea](/home/per/pert-ui-usability/r3-text-worktree/crates/pert-ui-runtime/src/text_fields.rs:240) describe tabs as blank without the registered-font mixed-ligature exception. Qualify that wording or link explicitly to `TextLayout`’s exception.

The remaining implementation checks out by source inspection: unchanged analysis text and resolved keys; shared shaping boundary for preparation, measurement, fit and ellipsis; retained final layout for phase rendering; actual Parley-build instrumentation; one atom scan followed by sorted exclusions and a consecutive-range merge; compile-time style bound; and no escaping transient borrows. Hidden runs are omitted from raster signatures while remaining icon-metric candidates. Glyph zero is not globally suppressed.

The tests exercise real fixture GSUB shaping, mixed prefix/suffix atoms, pure-tab ligatures, range splitting and final-layout purity. Fixture source, license, provenance and current hashes agree with the records. All six negative controls show assertion failures, and current source hashes match restoration evidence. The phase test exercises both cache orders; **N5 passes that particular test**, as disclosed, while other signature assertions detect it.

Facade editor tests and CPU captures use public facilities appropriately. `text_fields.rs` changes are rustdoc only. Facade exports, manifests, dependencies, lockfile and versions are untouched.

The integrating author still needs consistent wording in the runtime README, controls/appearance guides, assets README, contracts and rendering-platform documentation: shaped advance, no aligned tab stops, preserved canonical bytes/Unicode behavior, mixed-ligature exception, and unsupported-character ink. CHANGELOG, workspace and API-listing gates remain pending. Existing `TEXT-01` capacity and `NATIVE-05` evidence work remain open.

I authored no fixes, delegated nothing, and ran no builds or tests. Recorded targeted checks pass; this review establishes no workspace, API-generation, native/GPU, cross-platform or overall usability closure.