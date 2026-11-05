ready with changes

Revision 2 resolves most of design-s4a.md. Option C remains appropriate, but failed child reconciliation needs an explicit validity and recovery rule before implementation.

| Previous finding | Status | Assessment |
|---|---|---|
| Placement identity and freshness | **Partly resolved** | Keyed slots, retained viewport, source-replacement clearing and leave/re-enter invalidation address ordinary lifecycle changes. Failed reconciliation remains unspecified. |
| R15 overlap policy | **Resolved** | Exhaustive search, distinct reporter hits and no label deduplication are explicit. One test case should be added below. |
| Semantic-hook freshness/failure contract | **Resolved** | Signal reads are forbidden; snapshot fields and owner-retained state have defined freshness. Failed updates preserve valid state. |
| Opt-in reconciliation | **Resolved** | Replacement/removal occurs before the `owned_callbacks` early return. |
| QuickPick inspection changes | **Resolved** | Empty-state removal, value/actions, query/session identity changes and post-flush `Unmounted` are acknowledged and covered by proposed tests. |
| Alternatives, documentation and empty-list qualification | **Resolved** | B/D are accurately scoped; inspection enumeration is explicit; empty/nonempty refusal cases are distinguished. |
| Maintainer’s open questions | **Resolved** | C, `control`, draft/highlight meanings and exhaustive search are recorded consistently. |

Remaining findings:

1. **Medium — Failed child reconciliation is different from failed row construction.**
   [plans/todo-repairs-r14-popups.md:135](/home/per/github/pert-ui/plans/todo-repairs-r14-popups.md:135) and [the placement tests:321](/home/per/github/pert-ui/plans/todo-repairs-r14-popups.md:321).

   The list records `mounted = wanted` and eventually sets `rows_current = true` **before** the runtime accepts the requested children ([virtual_list.rs:2119](/home/per/github/pert-ui/crates/pert-ui-runtime/src/virtual_list.rs:2119)). Reconciliation can then reject successfully built descriptions, leaving the previous tree. The rejected request itself is consumed; only subsequent requests remain queued ([runtime.rs:6359](/home/per/github/pert-ui/crates/pert-ui-runtime/src/runtime.rs:6359)).

   Consequently, `rows_current` alone does not establish completed reconciliation, and a subsequent same-source update can skip requesting children because `wanted == self.mounted`. The note’s recovery promise needs more than the current failed-`build` test.

   **Suggested change:** distinguish requested rows from successfully adopted children, specify record validity and retry behavior after rejection, and add a test where `build` succeeds but child admission fails. Inspect immediately after failure, then recover without replacing the source. Verify surviving geometry, absent new-row geometry and no inheritance after actual removal/recreation.

2. **Low — Explicitly test the accepted external-list overlap.**
   [plans/todo-repairs-r14-popups.md:282](/home/per/github/pert-ui/plans/todo-repairs-r14-popups.md:282) and [the search tests:348](/home/per/github/pert-ui/plans/todo-repairs-r14-popups.md:348).

   The policy accepts an unmounted row producing both an `Item` hit and a `Row` hit, but the checklist explicitly tests only mounted descendants beside item hits.

   **Suggested change:** add an external opt-in source implementing both hooks; assert the two hits have their respective fields and targets. Keep this distinct from duplicate-label testing.

3. **Low — Complete the documentation exceptions.**
   [plans/todo-repairs-r14-popups.md:116](/home/per/github/pert-ui/plans/todo-repairs-r14-popups.md:116) and [the overlap description:277](/home/per/github/pert-ui/plans/todo-repairs-r14-popups.md:277).

   Updating “never iterates every row” is correct, but `ListItems::text` also promises that its default permits discovery only while mounted ([virtual_list.rs:164](/home/per/github/pert-ui/crates/pert-ui-runtime/src/virtual_list.rs:164)). Opt-in semantic items invalidate that qualification. Also, a closed MultiSelect’s visible trigger remains searchable without hidden inclusion.

   **Suggested change:** update the `text` documentation exception and distinguish visible trigger hits from hidden popup hits when closed.

I examined the proposal, prior review, relevant contracts and static source paths at 7eb3e6b and in the working tree. I treated the pending modality/self-picker fixes as supplied context, not verified completion. I did not audit unrelated working-tree changes, execute Cargo or tests, generate API listings/docs, inspect native behavior, or review an implementation of revision 2. No files were changed.