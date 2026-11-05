needs fixes

All findings are **should-fix**; no blocker identified.

1. **Resumed item scans bypass current modality.**
   [runtime/content_search.rs:150](/home/per/github/pert-ui/crates/pert-ui-runtime/src/runtime/content_search.rs:150)

   `ItemScan` emits saved hits without rechecking `modal_allows_in_own_window`, unlike `RowScan` immediately below it. With limit 1, return an owner’s name, open an already-mounted modal over it, then resume: the cursor still returns that blocked owner’s items. Opening the retained dialog need not change `structure_generation`.

   Check current modality before consuming the snapshot and discard the blocked scan, following the row-scan behavior. Add a continuation test that opens a modal between steps. This failure follows statically; the reproduction needs a run.

2. **Gallery search now indexes its previous search results as content.**
   [search.rs:43](/home/per/github/pert-ui/apps/gallery/src/search.rs:43), [quick_pick.rs:1491](/home/per/github/pert-ui/crates/pert-ui-runtime/src/quick_pick.rs:1491)

   The gallery searches hidden content and excludes only collection rows. A closed QuickPick retains its session, and `PickContent::inspect` exposes its ranked results as semantic items. The new traversal therefore imports the previous search’s results—including previously unmounted results—into the next search.

   Open search, close it, then reopen: entries such as `Results: <previous result>` can target the picker’s content component instead of the original widget or command. Repeated openings can accumulate these entries and consume the content limit. Some mounted-row contamination was possible previously; item searching substantially expands it.

   Exclude the search picker’s presentation subtree from its own search while retaining searchability of its gallery anchor. Test repeated openings and verify that content targets remain original targets. The visible outcome needs a run.

3. **R15’s central item/row continuation contract lacks the specified test.**
   [collections/content_search.rs:1243](/home/per/github/pert-ui/crates/pert-ui/tests/collections/content_search.rs:1243)

   The tiny-budget test uses components without `ContentRows`. It cannot detect premature `done`, lost row state after a name fills the limit, or incorrect ordering across items, unmounted rows, nested mounted descendants and an outside sibling.

   Add the specified nested-collection fixture with budget 1 and limit 1, asserting exact order and completion. Also test a new search **while the inserted item remains present**: the current test removes it at line 1285 before checking freshness, so it never proves that a new search sees additions. Run these tests after adding them.

4. **R14’s tests miss the geometry-specific cases that generic clipping tests cannot establish.**
   [pointer_items.rs:599](/home/per/github/pert-ui/crates/pert-ui/tests/surfaces/pointer_items.rs:599), [pointer_items.rs:643](/home/per/github/pert-ui/crates/pert-ui/tests/surfaces/pointer_items.rs:643)

   The stripe fixture never scrolls. The Accordion test opens the last section but never clicks a later header displaced by preceding expanded content. InlineChoice’s variable-width/icon segments are also untested.

   Consequently, omitted stripe-offset subtraction, omitted Accordion content height, or incorrect InlineChoice widths could pass the added tests. Add an overflowing, scrolled stripe with partial/full clipping; an Accordion click below open content; and a variable-width InlineChoice click. Reusing generic scale and failed-layout coverage is reasonable, but it does not replace these reporter-specific checks. Execution is required.

5. **R16’s identity test exercises binding updates, not redescription.**
   [transients.rs:1317](/home/per/github/pert-ui/crates/pert-ui/tests/surfaces/transients.rs:1317)

   Updating `label` leaves the component descriptions intact. The test therefore cannot detect reconciliation replacing the buttons, dropping their exposed-part declarations, or changing workspace paths—the identity guarantee explicitly documented for redescription.

   Re-describe the same keyed AlertDialog with changed fixed accept/cancel labels. Assert both retained IDs, updated semantic names, unchanged workspace paths and working exposed-part clicks. Keep the binding test separately. Run the resulting test.

The rectangle implementations otherwise appear consistent with their painting and hit-testing geometry. The confirmation lookup route matches the exposed-part scope walk; the documented deviation and Collapsible decision are reasonable. CHANGELOG, contracts, TODO narrowing and the API-listing additions are consistent with the implementation.

**Not examined:** deferred MultiSelect/QuickPick geometry design, unrelated repository repairs, or native rendering behavior. I ran no Cargo commands and did not regenerate API listings or independently verify the recorded gate results. Commit whitespace checking passed. No files were edited.