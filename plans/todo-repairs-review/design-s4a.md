needs rework

Option C is the strongest direction, but the checkpoint should resolve geometry freshness and content-search behavior before authorizing implementation. These are gaps in the proposed contract, not reasons to abandon C.

**High — Retained rectangles need an explicit association with row identity and presentation.**

The note proposes retaining slots and pairing them with the retained source snapshot ([note:76](/home/per/github/pert-ui/plans/todo-repairs-r14-popups.md:76), [note:182](/home/per/github/pert-ui/plans/todo-repairs-r14-popups.md:182)). Those records can advance separately today:

- `VirtualList::update` adopts the source snapshot before constructing its rows, explicitly allowing construction to fail afterward ([virtual_list.rs:1989](/home/per/github/pert-ui/crates/pert-ui-runtime/src/virtual_list.rs:1989)).
- Mounted `(index, key)` records change during update; slots are placed later during arrangement ([virtual_list.rs:2063](/home/per/github/pert-ui/crates/pert-ui-runtime/src/virtual_list.rs:2063), [virtual_list.rs:2160](/home/per/github/pert-ui/crates/pert-ui-runtime/src/virtual_list.rs:2160)).
- Runtime suppression checks whether the **reporter was arranged without a successful commit**. It does not detect a source replacement or failed row construction before arrangement ([inspect.rs:195](/home/per/github/pert-ui/crates/pert-ui-runtime/src/runtime/inspect.rs:195)).

Consequently, retaining rectangles by index would let a reordered or newly inserted item inherit another row’s rectangle. Even key matching needs a rule for removed/recreated rows and failed descriptions. “One retained source snapshot” does not establish those invariants.

**Suggested change:** specify keyed placement records, their validity across source adoption and child reconciliation, and when bounds must become absent. A newly mounted row must never inherit a previous incarnation’s rectangle. Preserve old geometry only for a surviving row whose presentation still corresponds to it. Include the viewport used by that arrangement in the retained geometry. Add reorder, insert/remove, replacement, failed-build/reconciliation, and recovery tests—not only failed arrangement.

**High — The R15 interaction cannot remain an unspecified future decision.**

The note correctly identifies MultiSelect’s second item set, but asks to decide de-duplication “in R15” ([note:259](/home/per/github/pert-ui/plans/todo-repairs-r14-popups.md:259)). R15 is already implemented in this working tree:

- Item names produce `ContentField::Item` hits ([content_search.rs:149](/home/per/github/pert-ui/crates/pert-ui-runtime/src/runtime/content_search.rs:149)).
- Search subsequently visits mounted children ([content_search.rs:253](/home/per/github/pert-ui/crates/pert-ui-runtime/src/runtime/content_search.rs:253)).
- Item hits target their reporting component; their indices are explicitly positions, not identities ([content_search.rs:172](/home/per/github/pert-ui/crates/pert-ui-runtime/src/content_search.rs:172)).

This produces more than trigger-versus-popup duplication. MultiSelect can offer the trigger item, popup-list item, and mounted `ChoiceRow` name. Moving QuickPick’s report removes the old parent report, but its list items still overlap mounted `PickRow` names. An external opt-in list implementing `ListItems::text` can additionally overlap unmounted-row hits.

**Suggested change:** decide the observable policy here and add integration tests. My recommendation for this repair is to preserve exhaustive search and explicitly accept distinct hits from distinct semantic reporters. Do not silently deduplicate equal labels: different options may legitimately share labels, and these hits have different targets and jump behavior. If unique logical-option results are required, that needs a separate public ownership/association contract; the current item API cannot safely infer it.

Test open/closed and hidden-inclusive searches, filtered and offscreen items, duplicate labels, mounted descendants, and continuation invalidation after QuickPick re-keying.

**Medium — The semantic hook needs a complete freshness and failure contract.**

The proposed `semantic_item` promises retained data and no writes/I/O ([note:72](/home/per/github/pert-ui/plans/todo-repairs-r14-popups.md:72)), but MultiSelect’s proposed state comes from shared signals: `shown()`, effective highlight, and dropdown permission. Existing `ChoiceRow` inspection uses fields retained by its update ([multi_select.rs:618](/home/per/github/pert-ui/crates/pert-ui-runtime/src/multi_select.rs:618)); QuickPick’s existing parent inspection instead reads the highlight during inspection ([quick_pick.rs:1478](/home/per/github/pert-ui/crates/pert-ui-runtime/src/quick_pick.rs:1478)).

The proposal must distinguish retained option membership from the freshness of checked, highlighted and disabled state. It must also explain how unavailable reads are handled without concealing failures or manufacturing state.

**Suggested change:** document the timing and ownership of each field, permissible reads, and failure behavior. Require no row building, filtering, measuring, revealing, signal publication or polling from the hook. Test permission redescription, external value changes, highlight changes without source replacement, and `OnClose` commit/cancellation.

The opt-in itself can conform to reconciliation rules: replace/remove it with each description, without publishing signals. Ensure its adoption occurs even through VirtualList’s existing `owned_callbacks` early-return branch ([virtual_list.rs:1908](/home/per/github/pert-ui/crates/pert-ui-runtime/src/virtual_list.rs:1908)).

**Medium — QuickPick’s observable changes are broader than moving the reporter.**

The statement that “only the component that reports it” changes is too strong ([note:112](/home/per/github/pert-ui/plans/todo-repairs-r14-popups.md:112)):

- Today `PickContent` reports a named ListBox even with no results ([quick_pick.rs:1491](/home/per/github/pert-ui/crates/pert-ui-runtime/src/quick_pick.rs:1491)). C removes that ListBox when the empty-state label replaces the list.
- Today that report has no value or component actions. C retains VirtualList’s numeric row-count value and overflow-dependent `Scroll` action ([virtual_list.rs:2754](/home/per/github/pert-ui/crates/pert-ui-runtime/src/virtual_list.rs:2754)).
- The new reporter’s identity changes on every query or session-generation change, including changes that yield the same ranked results—not only when result contents differ ([quick_pick.rs:1423](/home/per/github/pert-ui/crates/pert-ui-runtime/src/quick_pick.rs:1423)).

**Suggested change:** name these as intentional inspection changes and test them. Retain the recommendation to reacquire `options` after query changes; clarify that `Unmounted` follows reconciliation, not merely writing the query signal. Test same-results query changes, new sessions, message-only redescription and translated `list_name`.

**Low — Tighten the alternatives and documentation claims.**

The rejection of **A** is convincing for the current API: inspection has no context, and the reporter lacks the list’s retained placement data.

The rejection of **B** is a reasonable scope decision, but transforms make naïve translation incorrect rather than making geometry observation inherently impossible. The stronger objection is the missing public committed-placement/coordinate-space contract. Extending a reveal controller into geometry observation is additional responsibility, not itself an AGENTS.md violation.

**D** remains a viable but larger alternative. Its key lookup, identity validation and failed-layout rules require design. The scope objection is accurate for exposing `Panel`/`PickContent` directly under the existing tree: exposed parts stop traversal ([parts checkpoint:58](/home/per/github/pert-ui/plans/usability/round-3-design-parts.md:58)). Present that as a consequence of this particular exposure/tree arrangement, rather than an inherent property of runtime row resolution.

Also update `ListItems`’ existing “never iterates every row” promise for the opt-in ([virtual_list.rs:25](/home/per/github/pert-ui/crates/pert-ui-runtime/src/virtual_list.rs:25)). Document that inspection enumerates the entire source, including when content search acquires its item snapshot. The note’s closed-list reveal claim needs a nonempty qualification: `click_item` checks item existence before reveal, so an empty list returns `NoItem` first ([testing.rs:1015](/home/per/github/pert-ui/crates/pert-ui-runtime/src/testing.rs:1015)).

**Positions on the open questions**

1. **Choose C**, after the changes above. It keeps geometry with its owner and can give external composites the same public path as built-ins. No new inspection field means retaining schema 18 is reasonable.
2. **Expose MultiSelect `control`.** It provides useful Select parity and removes dependence on the internal `anchor` key. Document keyboard opening because the trigger center can hit a chip.
3. **Use `checked = shown()` and `selected = effective highlight`.** This distinguishes the displayed draft selection from navigation. Keep the trigger’s existing committed-selection meaning and document that its indices do not map to filtered popup indices.
4. **Decide search behavior now.** Accept and document distinct reporter hits for this slice; do not introduce label-based deduplication.

The label, shortcut and item-action preservation described in section 4 matches current code. Its disabled-pointer distinction is also accurate: MultiSelect refuses the toggle, whereas QuickPick deliberately offers disabled items to its callback. Preserve that behavior and make the `click_item` documentation exception explicit.

The proposed test list otherwise covers the principal pointer and geometry cases well. Add the identity, semantics and search cases above, plus opt-in replacement/removal and generic fixed/estimated, horizontal and multi-lane list coverage. The stated public-API gate is appropriate: full workspace gate, API listings, focused contracts, doctests, warning-free rustdoc and independent implementation review. It establishes headless evidence only.

I read AGENTS.md first and examined the proposal, R14/D9, relevant contracts, runtime inspection/search/layout/testing paths, both controls, VirtualList, Select, CommandPalette forwarding and relevant tests. I did not run Cargo, tests, doctests, API generation, native checks or generated-documentation review. I made no file changes; this review concerns the current working tree, including its existing uncommitted R15 changes.