needs rework

Revision 2 resolves most r1 findings, but R18, R13, R14 and R15 still leave important implementation contracts unspecified. The verification exception in D8 also remains unapproved.

This is a static review of HEAD `621022a` and the working-tree specification. I read `AGENTS.md` first, edited no files and ran no Cargo commands. “Resolved” below means resolved **in the specification**, not verified by implementation or execution.

| r1 finding | Status | Assessment |
|---|---|---|
| R18: invalid `EachItem` return type | partly resolved | Concrete `RowItem` fixes the return type; erasure machinery and external conversion remain unresolved. |
| Scope / NATIVE-01 and NATIVE-04 | resolved | Remaining Linux desktops and the macOS interruption check are classified accurately. |
| R22: scope exception and editor baseline | resolved | Explicit exception; measurements respect the current limit. D6 still needs acceptance. |
| R19: application authority and Docked fallback | resolved | Matches the implementation. |
| R19: scheduling and request lifecycle | resolved | Public notification path exists; duplicate and failure scenarios are covered. |
| R15: component-only jump and positional index | resolved | No item selection or stale-index retargeting. |
| R15: bounded scanning, protection and freshness | partly resolved | Protection is specified; cursor integration and mutation semantics are not. |
| R14: actual reporters and geometry owners | partly resolved | Owners identified, but reporting on the child lists introduces an undesigned semantic/API change. |
| R14: stronger geometry tests | resolved | Filtering, scrolling, clipping and view scaling are covered. |
| R12: both-axis cost improvement | resolved | Row/column policy and counters added; new grid/worst-case claim needs correction below. |
| R12: complete summary membership | resolved | All participants, thresholds, samples and suppression are specified. |
| R13: public slot-validation mechanism | partly resolved | Public availability is required, but the mechanism itself is still undefined. |
| R21: compressor choice and decoding tests | resolved | Maintained compressor, independent decoding and stored-layout migrations specified. |
| Gates / versions | partly resolved | Dependency items and API gates improved; cross-package gating and D8 remain problematic. |
| R10: retained reconciliation | resolved | Both rename transitions require adoption and paint invalidation. |
| R17: metadata clearing and freshness | resolved | Complete replacement and last-painted semantics specified. |
| R20: shared helper and focused assertions | resolved | Button participates; tests target the anchors. |
| R08: `PathError` and normalization | resolved | The specified Runtime/Drawing split matches the code. |
| Slice 8: concrete native acceptance | resolved | Separate probes/backends and substantive interactions specified. |
| R16 terminology | resolved | Exact names and lookup route specified. |
| R05 already-built binary | resolved | Compilation precedes deletion and direct execution. |
| R07 diagnostic contract | resolved | Messages, labels and notes distinguished from surrounding rustc output. |
| Cross-slice integration rechecks | resolved | R17/R20 and R18’s effects on earlier slices are acknowledged. |

**Blocker**

- **R18 — finish the erased representation and make it usable by external collections.** [Spec:407](/home/per/github/pert-ui/plans/todo-repairs.md:407) permits a boxed component or an `Element`, then makes key assignment private. A plain `Box<dyn Component>` is insufficient: [Element:348](/home/per/github/pert-ui/crates/pert-ui-runtime/src/layout.rs:348) stores children, concrete type identity and a typed reconciliation function, captured by [Element::new:425](/home/per/github/pert-ui/crates/pert-ui-runtime/src/layout.rs:425). [`Component::reconcile`:1166](/home/per/github/pert-ui/crates/pert-ui-runtime/src/runtime.rs:1166) requires `Self: Sized`; it cannot be recovered by calling it on the erased component.

  A concrete erased value **is implementable**, but the spec should choose how: capture a complete `Element` while `C` is known, or retain a consuming factory/typed thunk that constructs one. Preserve children and reconciliation machinery as well as labels, addressing, parts and grow metadata. Test same-type redescription and initial children, not only heterogeneous leaves.

  Separately, private conversion gives built-in collections exclusive access to the new erased return value. External composites cannot reproduce the key-assignment behavior used by [VirtualList:2058](/home/per/github/pert-ui/crates/pert-ui-runtime/src/virtual_list.rs:2058) and [Tree:129](/home/per/github/pert-ui/crates/pert-ui-runtime/src/tree.rs:129). Provide a documented public consuming conversion under a supplied key, or another public equivalent. `EachItem`’s private conversion is precedent for implementation technique, not sufficient justification for this restriction.

**Should-fix**

- **R13 — there is no existing slot-height hint API.** [Spec:285](/home/per/github/pert-ui/plans/todo-repairs.md:285) refers to “the same public validation-hint mechanism,” but [`ValidationHints`:101](/home/per/github/pert-ui/crates/pert-ui-runtime/src/validation.rs:101) has a **boolean** `fixed_height` describing the reporting component’s own children. It cannot carry a per-child slot height or width. The existing check uses the parent’s box and a previously recorded measurement ([development.rs:710](/home/per/github/pert-ui/crates/pert-ui-runtime/src/validation/development.rs:710)).

  Specify the proposed public API, its owner, units, validation, clearing and successful-layout retention rules. Prefer an arrangement-time operation that measures through `ArrangeContext::measure` at the actual slot width with unbounded height, records the result and slot, and propagates measurement failures through layout. Validation/report reading should consume retained evidence.

  Extra development measurement is not inherently contrary to “unknown stays absent”: it makes a previously unknown measurement known. It must, however, use the ordinary dependency/cache path and respect successful-layout correlation. Cache hits also update the latest measurement ([tree.rs:1839](/home/per/github/pert-ui/crates/pert-ui-runtime/src/runtime/tree.rs:1839)); blindly probing during validation could overwrite the evidence used by other fixed-height checks. Update the public contract at [contracts.md:2109](/home/per/github/pert-ui/docs/contracts.md:2109), not just the diagnostic variant documentation.

- **R15 — `(component, next item index)` does not yet define a bounded continuation.** [Spec:362](/home/per/github/pert-ui/plans/todo-repairs.md:362) calls this the existing visit cursor, but today search has separate root discovery, a pending component stack, an optional `RowScan`, and visited collection revisions ([content_search.rs:236](/home/per/github/pert-ui/crates/pert-ui-runtime/src/content_search.rs:236)). Components are popped and charged once; row scanning has its own priority and completion logic ([runtime/content_search.rs:129](/home/per/github/pert-ui/crates/pert-ui-runtime/src/runtime/content_search.rs:129)).

  Define a new item-scan state, charge each examined item, and specify how it interacts with component matching, unmounted-row scanning, children and `done`. Otherwise a result limit reached on the component name can skip its items, or an unfinished item scan can be declared complete.

  Also choose mutation semantics. “Freshness as documented” points to no existing semantic-item revision contract. If an item is inserted before the saved index, live rescanning can duplicate an earlier hit. Choose a retained per-component snapshot or an explicit invalidation policy. Account for the cost of `inspect()`, whose current reporters construct whole item vectors; repeatedly reconstructing that vector for a tiny-budget continuation defeats the intended bound.

- **R14 — child-list reporting changes existing semantics and needs its own design.** [Spec:319](/home/per/github/pert-ui/plans/todo-repairs.md:319) proposes that popup lists report their own items. This does **not inherently conflict** with the round-3 parts checkpoint: that checkpoint explicitly directs Select consumers to its ListBox and keeps closed-anchor rectangles absent ([checkpoint:91](/home/per/github/pert-ui/plans/usability/round-3-design-parts.md:91)).

  It does conflict with assuming the current `VirtualList` already supplies that model. Its implementation deliberately does **not** enumerate rows as semantic items ([virtual_list.rs:2714](/home/per/github/pert-ui/crates/pert-ui-runtime/src/virtual_list.rs:2714)). MultiSelect’s child list is structural and exposes no options part ([multi_select.rs:880](/home/per/github/pert-ui/crates/pert-ui-runtime/src/multi_select.rs:880)); QuickPick’s structural list is recreated under a new key when results change ([quick_pick.rs:1369](/home/per/github/pert-ui/crates/pert-ui-runtime/src/quick_pick.rs:1369)).

  Choose a public reporting/geometry facility and lookup route. Define whether existing anchor/panel items remain, how filtered indices map to source rows, whether unmounted items are represented, and how labels, selection, disabled state, shortcuts and actions survive. QuickPick currently promises those details on its results ListBox ([quick_pick.rs:830](/home/per/github/pert-ui/crates/pert-ui-runtime/src/quick_pick.rs:830)); moving or duplicating them requires explicit documentation and tests, including disabled-item activation behavior.

- **R12 — the proposed algorithms do not establish the new grid or output-sensitive claim.** [Spec:246](/home/per/github/pert-ui/plans/todo-repairs.md:246) permits a one-axis sweep chosen by start spread, then promises near-linear disjoint grids and describes the remaining cost as output-sensitive.

  In a square grid, an ordinary active-set sweep compares rows within each active column even though none overlaps in both axes. Candidate comparisons can be superlinear with zero reported overlaps. A uniform grid also needs cell sizing and duplicate-pair handling before its bounds are meaningful.

  Limit the claim to the tested row/column cases, document cost in terms of candidate comparisons, or specify a stronger algorithm. Add a disjoint-grid counter test if retaining the grid promise. Compilation cannot establish these performance claims.

- **Verification — D8 remains a proposed exception, and slice 1 still spans packages.** [Working rules:67](/home/per/github/pert-ui/plans/todo-repairs.md:67) already exempt internal dependency-version changes, while [D8:605](/home/per/github/pert-ui/plans/todo-repairs.md:605) still awaits a maintainer decision. [`AGENTS.md`:262](/home/per/github/pert-ui/AGENTS.md:262) contains no external-dependency-only qualification.

  Mark D8 explicitly provisional and retain the governing gate until accepted. Alternatively avoid unnecessary version changes where authorized. Slice 1 also changes the native runner and probe package ([spec:134](/home/per/github/pert-ui/plans/todo-repairs.md:134)); package-local checks alone do not satisfy the cross-package row.

  Slice 6 correctly invokes the dependency row, but its listed gate omits explicit **consumer checks** ([spec:536](/home/per/github/pert-ui/plans/todo-repairs.md:536)). Reference the complete governing row, including packaging/consumer evidence and deferred native requirements. Keep R17/R20 together; separate the unrelated compression and clipboard updates into reviewable commits even if they share one verification batch.

**Nits**

- **R14 — qualify failed-layout absence.** [Spec:335](/home/per/github/pert-ui/plans/todo-repairs.md:335) says no rectangle exists after a failed layout. The contract suppresses item bounds after a failed layout **that arranged the reporting component** ([contracts.md:1620](/home/per/github/pert-ui/docs/contracts.md:1620)). Use that qualification and add the corresponding regression case.

The scope table otherwise accounts for TODO.md correctly. R23’s appearance in both tables is a cross-reference to the mixed entry, not another finding. NATIVE-01’s unavailable Linux work and TEXT-01’s design-only outcome remain explicit exceptions; neither can be described as repaired. No additional scope-table mismatch was found.

R19’s public path is available: `UpdateContext::notifier`, `Notifier::request` and `Component::deliver_notifications`; ToolWindows already stores the notifier and calls `request_detachments` from delivery. R08’s normalization is also correct: the blanket conversion maps `DrawError::Runtime` to `AppError::Runtime`, and other drawing errors—including `PathError`—to `Drawing`.

I did not examine compiled feasibility, generated API listings/rustdoc, actual doctest failures, diagnostics regeneration, compression correctness/performance, packaging/consumer execution or native behavior. I did not verify registry yank status or select a clipboard replacement. No findings were written to TODO.md because this review explicitly prohibits edits.