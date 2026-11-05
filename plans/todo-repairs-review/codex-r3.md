needs rework

R18’s blocker is resolved. R12 and verification are also resolved in the specification. R13 and R15 still need precise retention and continuation rules before their slices are implementation-ready. Slices 1–2 can proceed under the stated gates.

“Resolved” here means specified adequately, not implemented or verified. HEAD remains `621022a`.

| r2 finding | Status | Assessment |
|---|---|---|
| R18: erased representation and external conversion | resolved | Complete `Element` preserves typed reconciliation and children; public conversion removes the privileged path. |
| R13: public slot-validation mechanism | partly resolved | API, measurement and reporting are specified; measurement isolation and clearing rules remain problematic. |
| R15: bounded continuation and mutation | partly resolved | Snapshot, item charges and `done` are specified; RowScan ordering/state and snapshot cost need correction. |
| R14: popup reporting design | resolved as a checkpoint | Acceptable design boundary; D9 remains a maintainer decision, and deferred families remain unresolved repairs. |
| R12: unsupported performance claim | resolved | Promise narrowed to rows/columns; quadratic candidate cases acknowledged. |
| Verification / D8 | resolved | Full governing row applies until D8 is accepted; cross-package and consumer checks are explicit. |
| R14: failed-layout qualification | resolved | Both prose and regression test now identify failures that arranged the reporter. |

**Blockers**

None found in this revision.

**Should-fix**

1. **R13: the measurement path contradicts the promised isolation.**
   [Spec:300–309](/home/per/github/pert-ui/plans/todo-repairs.md:300) requires ordinary `ArrangeContext::measure` but says the probe never overwrites the latest measurement used by existing validation. That call reaches `measure_node`, which promotes cache hits and inserts fresh results as the latest measurement, notifying development bookkeeping in both cases ([tree.rs:1839](/home/per/github/pert-ui/crates/pert-ui-runtime/src/runtime/tree.rs:1839), [tree.rs:1870](/home/per/github/pert-ui/crates/pert-ui-runtime/src/runtime/tree.rs:1870)).

   A separate slot-evidence field alone does not provide isolation. Measuring a wrapper can also change descendants’ latest measurements.

   **Suggested change:** specify how diagnostic evidence is isolated while retaining normal cache and dependency tracking. For example, separate ordinary-validation evidence from cache recency, or define scoped preservation covering nested measurements. Require cache-hit, cache-miss and nested-wrapper tests demonstrating that the parent-box check retains the correct evidence.

   Measurement during arrangement **is allowed**: it is explicitly public ([layout.rs:1143](/home/per/github/pert-ui/crates/pert-ui-runtime/src/layout.rs:1143)), and forwarding wrappers already refresh measurements there ([contracts.md:2112](/home/per/github/pert-ui/docs/contracts.md:2112)). Neither `contracts.md` nor `internals.md` imposes a blanket prohibition.

2. **R13: clearing must follow the parent’s placement declaration, not the child’s arrangement callback.**
   [Spec:306](/home/per/github/pert-ui/plans/todo-repairs.md:306) clears evidence when the child is next “arranged without a slot declaration.” The declaration belongs to the parent. A parent can replace `place_fixed_slot` with ordinary placement at identical geometry while the child’s arrangement is skipped ([tree.rs:1925](/home/per/github/pert-ui/crates/pert-ui-runtime/src/runtime/tree.rs:1925)). Conversely, a child can arrange while its parent reuses retained placements ([tree.rs:1953](/home/per/github/pert-ui/crates/pert-ui-runtime/src/runtime/tree.rs:1953)).

   **Failure scenarios:** an obsolete slot warning survives ordinary placement, or a valid declaration disappears when only the child arranges.

   **Suggested change:** retain slot evidence alongside parent-owned placement metadata. A fresh ordinary placement clears it; retained placement preserves it. Stage replacement and clearing until successful layout, and discard staged changes on failure. Test identical-geometry transitions, retained placement, failed clearing and unmount.

3. **R15: children-before-RowScan ordering needs additional traversal state.**
   [Spec:424](/home/per/github/pert-ui/plans/todo-repairs.md:424) orders component hits, items, children, then unmounted rows and calls that preservation of tree order. Today `RowScan` has priority over `pending`, and the cursor has only one row-scan slot ([runtime/content_search.rs:145](/home/per/github/pert-ui/crates/pert-ui-runtime/src/runtime/content_search.rs:145), [content_search.rs:247](/home/per/github/pert-ui/crates/pert-ui-runtime/src/content_search.rs:247)).

   Simply pushing children cannot produce the specified order. Deferring the parent’s `RowScan` while visiting a nested collection requires somewhere to preserve both continuations; otherwise the parent scan can be overwritten, skipped or resumed after unrelated siblings.

   **Suggested change:** either preserve existing ordering—component, items, unmounted rows, children—or explicitly introduce traversal frames/deferred row scans. Specify how collection revisions and `mounted_current` are captured before yielding on a name/item hit. Add a nested-collection test with matching items, mounted descendants, unmounted rows and an outside sibling, using one-unit budgets/result limits.

4. **R15: snapshot construction still has an unaccounted linear cost.**
   [Spec:419](/home/per/github/pert-ui/plans/todo-repairs.md:419) copies all item texts during one component visit, then charges only subsequent item examinations. Avoiding repeated `inspect()` solves repeated reconstruction, but a budget-one visit can still inspect and copy an arbitrarily large item collection. The existing public description says the budget counts work growing with a collection ([content_search.rs:232](/home/per/github/pert-ui/crates/pert-ui-runtime/src/content_search.rs:232)).

   **Suggested change:** explicitly describe the one-time inspection/snapshot allocation and text-volume cost as outside resumable item matching, and qualify the bounded-work documentation accordingly. Move already-owned strings rather than cloning them unnecessarily. If strict acquisition bounds are required, design incremental reporting separately. Add an inspection-call counter proving continuation does not rebuild the snapshot.

**Nits**

- **Name the actual development condition.** [Spec:300](/home/per/github/pert-ui/plans/todo-repairs.md:300) should say `#[cfg(debug_assertions)]`, with no extra measurement when that cfg is absent. Development validation is compiled under that condition ([validation.rs:5](/home/per/github/pert-ui/crates/pert-ui-runtime/src/validation.rs:5)); it is automatically populated independently of diagnostic-history mode ([runtime.rs:4148](/home/per/github/pert-ui/crates/pert-ui-runtime/src/runtime.rs:4148)). There is no separate runtime enable switch to infer.

- **Repair the parts-checkpoint link.** [Spec:371](/home/per/github/pert-ui/plans/todo-repairs.md:371) links to `round-3-design-parts.md:91`, treating the line suffix as part of the filename. Link to the Markdown file or an actual heading anchor.

R18’s internal placeholder is safe at description construction: [`Element::new`](/home/per/github/pert-ui/crates/pert-ui-runtime/src/layout.rs:425) captures children and typed machinery without validating the key; validation occurs before mounting. `RowItem::into_element(key)` changes an unmounted description, so it does not itself reassign a mounted identity. Ordinary mount/reconciliation validation must remain intact. `EachItem` can delegate internally while staying sealed; accepting `RowItem` as an `EachItem` would be a separate API choice, not a prerequisite.

Slice 4a is an acceptable boundary: it requires a concrete public facility, external-composite usability, semantic preservation and stale-geometry decisions before implementation. However, D9’s fallback is another possible scope reduction. Without maintainer acceptance, completing slice 4 cannot imply authorization to leave those repairs unfinished. D6 is accepted and does not need reopening.

I did not examine implementation execution, Cargo checks, generated API listings/rustdoc, diagnostics regeneration, compression correctness/performance, packaging/consumer execution or native behavior. I did not independently reassess every finding already resolved in r2 or verify registry yank status. No files were edited, including TODO.md, as requested.