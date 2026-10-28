**Verdict: ready.** Revision 3 has no remaining concrete blockers. Both review-r2 blockers are closed, and the source agrees with the new contracts. I edited nothing and ran nothing. Since geometry rev 2 was ready on condition of this checkpoint, it's now unblocked too.

**Blocker 1 (scopes merging through structural nesting): resolved.**
- **Merge rule.** The plan now states it, and a nested composite gets its own scope by being exposed under a name. Only Own-keyed children without a declared part stop the walk.
- **Only two built-ins leak today.** They are the ones the plan names: MultiSelect (`multi_select.rs:748`) and QuickPick (`quick_pick.rs:1340-1346`). Both embed a SearchField structurally, and the plan isolates each as `search`.
- **Everything else is behind an Own key, so nothing leaks:**
  - Breadcrumb's overflow menu (`breadcrumb.rs:246`)
  - the tool-window options menu (`tool_windows/options.rs:352`)
  - the text fields in ColorPicker, FileBrowser and CollectionFilter (`color_picker.rs:705,711`, `file_browser/view.rs:155`, `collection_filter.rs:1110`)
  - HistoryNavigator's picker (`history_navigator.rs:339`)
- **One intended forward.** CommandPalette passes QuickPick's `search` straight through, because it wraps the picker directly (`command_palette.rs:532`). That's the plan's "directly delegated" example, so it should be listed in the docs.
- **Menu tree shape matches the plan.** Submenu panels are siblings of `items` inside each MenuBranch (`menu.rs:1041-1052, 1117`), so `submenu/<row-key>` and `items` share a level. The `clipboard` root fits on the text field's structural `menu` element (`menu.rs:662`), which is built when the field is constructed (`text_fields.rs:1110`). It can therefore be found while the menu is closed.

**Blocker 2 (window and error contracts): resolved.**
- **No window check, matching current code.** `Driver::find` has no selected-window check (`testing.rs:633-658`), and the plan now matches that.
- **Error variants are named and placed correctly.** `ForeignRuntime`, `Unmounted` and `EmptyKey` exist (`error.rs:225-229`). `EmptyExposedPartName` mirrors `EmptyKey`. `AmbiguousExposedPart` is a `RuntimeError`, and `ExposedPartNotFound` is a `TestError`.
- **Owner identity works.** The `u64` owner is the same serial as `WidgetId::incarnation` (`runtime.rs:127`) and the inspection id.

**Small wording fixes (not blocking):**
- **Define "blank".** Say it means empty after trimming, as `validate_traced` checks keys (`tree.rs:58`). "Compared exactly without trimming" should only describe matching, otherwise a whitespace-only name looks valid.
- **Error-variant test.** The exhaustive match in `error.rs:180-247` must include the two new `RuntimeError` variants. That's mechanical, but worth listing with the other gates.
- **Name-only redescription.** The plan should say plainly that the new name-only change doesn't reach the parent-layout invalidation at `tree.rs:575-577`. It does say this, but only in the decision list.
