needs rework

Revision 4 resolves five of the six r3 points at the specification level. R13 isolation still rests on unavailable or evictable evidence. Slices 1–2 remain ready; the whole specification is not yet ready to implement unchanged.

| r3 finding | Status | Assessment |
|---|---|---|
| R13 isolation | partly resolved | Recognizes the problem, but the preferred cache lookup cannot guarantee the required evidence; fallback needs precision. |
| R13 retention | resolved | Parent-declaration ownership, clearing, retention and successful-layout adoption are specified correctly. Storage must be clarified below. |
| R15 ordering | resolved | Name/value → items → RowScan → children matches existing traversal precedence. |
| R15 snapshot cost | resolved | Explicitly acknowledges acquisition cost outside resumable matching and requires moving owned texts. |
| Development cfg nit | resolved | Correctly names `cfg(debug_assertions)`. |
| Parts-checkpoint link nit | resolved | Now links to the existing Markdown file. |

“Resolved” means adequately specified, not implemented or tested. HEAD is `621022a`.

**Should-fix — high: R13’s preferred evidence source is neither defined nor reliably retained.**

[Spec:313–318](/home/per/github/pert-ui/plans/todo-repairs.md:313) proposes looking up “the measurement at the constraints the child was actually placed with” and asserts that a probe cannot displace it.

Placement carries a rectangle, not measurement constraints: [layout.rs:1270](/home/per/github/pert-ui/crates/pert-ui-runtime/src/layout.rs:1270). Allocation dimensions do not identify the earlier measurement request, which may have unbounded height. Moreover, `measure_node` keys cached results by the caller’s `available` constraints while passing size-limit-adjusted constraints to the component: [tree.rs:1841](/home/per/github/pert-ui/crates/pert-ui-runtime/src/runtime/tree.rs:1841), [tree.rs:1866](/home/per/github/pert-ui/crates/pert-ui-runtime/src/runtime/tree.rs:1866), [tree.rs:1873](/home/per/github/pert-ui/crates/pert-ui-runtime/src/runtime/tree.rs:1873).

Even with the correct key, the cache holds only four entries and evicts the least recently used: [measurements.rs:10](/home/per/github/pert-ui/crates/pert-ui-runtime/src/runtime/measurements.rs:10), [measurements.rs:33](/home/per/github/pert-ui/crates/pert-ui-runtime/src/runtime/measurements.rs:33). A probe **can** displace evidence. Falling back to allocated geometry would lose the natural-height evidence the existing check needs: [development.rs:721](/home/per/github/pert-ui/crates/pert-ui-runtime/src/validation/development.rs:721).

Suggested spec change: choose independent ordinary-validation evidence, or make scoped preservation the explicit approach. Preserve the actual pre-probe size and constraints throughout nested measurements, including on error; restoring “marks” cannot recover an evicted entry. Ordinary cache/dependency updates may remain. Cover both geometry commits and the measurement-change drain, which currently rereads the latest cache entry: [tree.rs:2231](/home/per/github/pert-ui/crates/pert-ui-runtime/src/runtime/tree.rs:2231), [tree.rs:2263](/home/per/github/pert-ui/crates/pert-ui-runtime/src/runtime/tree.rs:2263). Add an eviction regression and remove the placed-constraints assertion from the isolation test.

**Should-fix — medium: identify the placement storage being extended.**

[Spec:321–330](/home/per/github/pert-ui/plans/todo-repairs.md:321) refers to a “parent-owned placement record.” There is no dedicated structure of that description today. `Placement` is a tuple of rectangle and optional clip: [tree.rs:24](/home/per/github/pert-ui/crates/pert-ui-runtime/src/runtime/tree.rs:24). Committed placement lives on the **child’s node**, although the parent supplies it: [runtime.rs:900](/home/per/github/pert-ui/crates/pert-ui-runtime/src/runtime.rs:900). Parent arrangement stages and commits those tuples: [tree.rs:21](/home/per/github/pert-ui/crates/pert-ui-runtime/src/runtime/tree.rs:21), [tree.rs:2185](/home/per/github/pert-ui/crates/pert-ui-runtime/src/runtime/tree.rs:2185).

Suggested spec change: explicitly extend that tuple into an internal placement record carrying optional slot evidence, or specify equivalent staged storage. Describe it as **parent-authored**, distinguish storage from ownership, and preserve the stated retention rules. This is a feasible implementation addition, not existing infrastructure.

**Should-fix — low: narrow R15’s inspection-counter promise.**

The claim that continuation “never re-inspects a component” is broader than the current search supports: [spec:497](/home/per/github/pert-ui/plans/todo-repairs.md:497). Naming fallback inspects ancestor components while visiting descendants: [content_search.rs:323](/home/per/github/pert-ui/crates/pert-ui-runtime/src/runtime/content_search.rs:323).

Suggested spec change: require that **resuming an ItemScan does not inspect its owner or reconstruct its item snapshot**. Leave existing ancestor naming inspection outside that assertion unless caching it is deliberately added.

R15’s ordering and yield-state requirements otherwise hold up. Today children are queued before matching, and collection revision/RowScan state is installed after pushing a name hit but before the loop can yield: [content_search.rs:223](/home/per/github/pert-ui/crates/pert-ui-runtime/src/runtime/content_search.rs:223). Mounted-child indexing remains resumable inside RowScan. The required implementation refactor is to retain semantics even when no name/value matches: `content_hit` currently owns and drops that inspection result, including through early returns: [content_search.rs:272](/home/per/github/pert-ui/crates/pert-ui-runtime/src/runtime/content_search.rs:272). Snapshot creation must therefore precede returning from the component visit. One ItemScan slot and one RowScan slot suffice under the specified precedence.

I did not reassess other repairs already accepted in r3, execute Cargo or native checks, or examine generated API/rustdoc, packaging, diagnostics snapshots or performance. This was a static review. No files were edited, including TODO.md.