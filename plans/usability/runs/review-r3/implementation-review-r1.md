**Verdict: needs changes.** The runtime and library code for this checkpoint looks correct as far as I could read it, and I found no implementation defect. Three things stop me calling it ready: one native evidence step proves nothing, two "no extra work" tests can't fail, and the Menu part names have no test at all. I edited nothing, ran nothing and did not delegate.

## Blocking findings

**1. The native tab-title step doesn't do what its name says** (`apps/gallery/src/interaction_check.rs:830-845`)
- The step first calls `show(ui, scene, "tool-windows")`. That page belongs to `layout-category`, which is item 0 of `gallery-tabs` (`apps/gallery/src/gallery.rs:244-271`).
- It then clicks item 0 and checks that item 0 is selected. Item 0 was already selected before the click, so the check passes whether or not the click works.
- The step is named "item pointer bounds switch the category", and `docs/verification.md` counts it as native evidence for the tab-title pointer path. All it shows is that the title has bounds and that a press/release went through.
- The Select and menu steps are real: Select goes from "first" to index 2, and the menu opens About.
- **Fix:** click an unselected category's title, check that selection moved, then rerun both native checks (wgpu and CPU) and record the result again.

**2. Two "no extra work" tests can't detect a regression**
- `Driver::quiet_for` first flushes, then paints a baseline (`testing.rs:571-585`). Updates in that baseline flush aren't counted, so any invalidation already queued is absorbed before anything is measured.
- `names_replace_and_remove_without_remounting_or_scheduling_layout` (`tests/foundation/exposed_parts.rs:100-120`) has this problem. If someone re-added the parent layout invalidation at `tree.rs:667-669` for name-only changes, the test would still pass.
- The same applies to the "repeated inspection schedules nothing" check at `tests/surfaces/pointer_items.rs:130-134`.
- This contract is stated in `Element::expose_part`'s rustdoc, the CHANGELOG, `contracts.md` and `testing.md`. The code itself is right: name-only changes `continue` before any invalidation (`tree.rs:649-651`).
- **Fix:** assert right after the reconcile or inspection, before any flush. For example: `ComponentInspection::dirty` is `None` for both parent and child, `needs_paint()` is false, and `next_deadline()` is `None`. Use a child whose reconcile of an identical description returns `None`.

**3. Menu's part names are untested, and the guide overstates the examples**
- Nothing tests or demonstrates `Menu`'s `trigger` and `items` (`menu.rs:797-802`).
- Nothing checks the rev-3 decision that an icon menu's `trigger` is the inner Button rather than its Tooltip wrapper (`menu.rs:843-855`).
- The parts design also asked for evidence of multiple MenuBar titles and the same row key reused at a nested level. Neither appears in the tests; only the gallery covers multiple titles, and only indirectly.
- There is no pointer test that clicks a ContextMenu row.
- `testing.md` ("Composite parts") says "The component's rustdoc has complete pointer examples". That is true for Select, MenuBar and Tabs, but not for Menu or ContextMenu.
- **Fix:** add tests for:
  - `Menu::new` and `Menu::icon`: read the trigger's semantics, then click a row.
  - A MenuBar with two or more titles, and a row key (e.g. `run`) repeated inside a submenu, showing the two scopes stay separate.
  - A ContextMenu opened by a context-menu request, then a row clicked.

  Also add a Menu doctest.

## Should fix (not blocking)

4. **Tab recreation is only tested after the cache rebuilds** (`pointer_items.rs:164-172`).
   - There's a flush between removing and re-adding the tab, so the cached header has already been rebuilt.
   - The identity guard at `tabs.rs:1894-1896` could be deleted without any test failing.
   - Reordering under a reused key isn't covered.
   - The guard is extra safety: `click_item` flushes before reading bounds, and the identity map updates in the same flush that rebuilds the headers.
5. **Some error assertions only check `is_err()`.**
   - `exposed_parts.rs:226` should match `TestError::OtherWindow`.
   - `pointer_items.rs:56` and `:83` should match `NotRevealed`.
   - As written, a failure for the wrong reason would pass.
6. **The text editors don't document their new `clipboard` part.**
   - `TextField`, `TextArea` and `SearchField` now each expose one, but their rustdoc doesn't say so. Only the guide table and the ContextMenu, MultiSelect and QuickPick docs mention it.
   - `items` below `clipboard` is never looked up in a test.
7. **The new "Exposing composite parts" section in `custom-widgets.md` reads awkwardly.**
   - It opens with "uses that same public contract", but nothing before it says which contract.
   - It doesn't say why the nested Select is exposed rather than made structural.
   - The unrelated paragraph about f32/f64 pointer precision sits under this heading.
8. **The 8-px narrow-strip test may not test what it intends.** If Tabs is laid out at its 96-px minimum (`tabs.rs:1155`), the assertion passes because the window clips the titles, not because the strip is narrower than its arrows. I couldn't confirm which happens without running it.
9. **Smaller test gaps:**
   - Lookup forwarding through workspace part/slot wrappers (only structural wrappers are tested).
   - The drag-preview skip.
   - OptionList having no bounds before its first layout.
   - The CommandPalette route.
10. **Nits:**
    - The Tabs doctest imports `windowing::*` without using it. It fits the existing TODO entry about unused doctest imports.
    - The testing-guide table leaves out ContextMenu's `items`.
    - The status line in `round-3-design-parts.md` still says "not implemented".
    - The CHANGELOG doesn't label schema 17 as a breaking persistence change, as the schema-16 entry did. It also has an extra blank line before 0.3.88.

## Checked and correct (by reading)

- **Validation:**
  - A blank name fails before anything changes, for both mount and reconcile, and the last valid metadata is kept.
  - The owner's identity is checked before the name.
  - Names match exactly.
  - Duplicates are listed but a singular lookup refuses them.
  - The two lookup paths share one walk.
- **Lookup walk:**
  - It stops at Own-keyed children and at exposed parts, and skips the drag preview.
  - It doesn't flush, focus or check the selected window, so lookups work for parts in other windows.
- **Serde:** the new field is required and may be null, blank names are rejected, and other schema versions are rejected.
- **Tabs:**
  - Every use of the cached title rectangle now applies the live scroll offset (`boundary_at`, `target`, paint, the drop line and inspection).
  - Arrange's reveal logic still works in unscrolled coordinates.
  - The detach marker is presentation only, so clicking a title's centre still selects it.
- **OptionList:** row bounds are clipped to the arranged viewport, and there are none before layout.
- **Unchanged contracts:** workspace paths and round trip, Field naming, and the QuickPick `surface` → `search` → `clipboard` route.
- **Exports and docs:**
  - The `ApplicationError` component-module export, with a test that uses only the component glob.
  - The facade's testing re-export and the closure seed.
  - The Dialog, `reveal_child` and VirtualList documentation repairs match the source.

## Outside this checkpoint

- `AlertDialog`'s new docs make its private `accept` and `cancel` keys a documented route through `find_descendant` (`dialog.rs` Inspection section). That's the kind of private-key route this round is retiring. It's worth a TODO entry for exposing those buttons as parts later; it isn't a blocker.
- The TODO entry for item reporters without rectangles was narrowed correctly.
- The literal-tab rendering defect and its proposal in `round-3-design-tabs-whitespace.md` were not reviewed and are not approved.

## Evidence limits

- **Recorded results I relied on:**
  - Workspace tests attempt 3 (all new tests listed as ok) and API generation attempt 2.
  - Every command in `outcomes.json` exited 0: format, Clippy, the five examples, doctests, `-D warnings` rustdoc, diagnostics, and the serde runs.
  - Native checks passed for both wgpu and CPU on Fedora 44 / GNOME Wayland. The adapter wasn't reported and the GNOME version was recorded by hand. Other native checks were skipped by the name filter and aren't claimed.
- I couldn't confirm the order of the gate logs relative to the later prose edits.
- **What this verdict covers:** only this repair checkpoint. Independent acceptance, full recurrence triage and overall round 3 completion are still open.
