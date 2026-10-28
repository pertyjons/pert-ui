**Verdict: not ready for implementation as written.** The geometry design is sound, but three things need settling first: the dependency on the parts checkpoint, the Tabs scroll timing, and a few missing documentation items. Everything below comes from reading the source; I didn't run anything.

## What I verified (no change needed)

- **Tabs clipping.** Hit testing (`tabs.rs:842-863`) checks the arrows before titles: x < inset is Previous, x ≥ width − inset is Next. Painting clips titles to `(inset, 0, width, height)` (`tabs.rs:1645-1648`). So intersecting `Header::rect` with `header_space()` and `self.height` gives exactly the area hit testing treats as a title. When the strip is narrower than two arrows, the title area is zero, so no title gets bounds. The runtime's `item_bounds` (`runtime/inspect.rs:548-569`) already turns a zero-area result into `None`.
- **Tabs identity.** `Header` keeps `key` and `id` from arrange (`tabs.rs:1255-1264`), and items come from the current `order`. Matching on both key and `identities[key]` means a tab recreated under the same key has no bounds until layout. A reordered tab keeps its old rectangle until layout, which matches what hit testing would hit, so that's consistent.
- **OptionList geometry.** `top(i) - offset` with `row` matches both paint (`select.rs:716-721`) and hit testing (`select.rs:188-200`). The offset is read live, so scrolling shows at once, as the contract requires. The list only keeps a height (`select.rs:129`) and paint gets its width from `p.size()`, so keeping the arranged `Size` is needed. Before the first arrange `row` is 0, so no rectangle has area.
- **Separators.** They become bands between options, not options (`menu.rs:988-997`, `index_at`), so they can't take an item index.
- **Disabled rows.** Activation is still refused: `hit` filters `is_enabled` (`select.rs:198`), a non-editable list ignores pointer input (`select.rs:538`), and the menu callback re-checks `enabled` and the live permission (`menu.rs:1077-1080`).
- **Schema.** `bounds` is optional and documented under schema 16 (`diagnostics.rs:416-453`). Filling it in more often doesn't add, remove or reinterpret a field, so no `SCHEMA_VERSION` bump is needed. No test asserts these items have `bounds: None`.
- **Runtime machinery.** Moving, scaling, clipping, suppression while hidden or after a failed layout, and detach/redock are already covered by the Table tests (`tests/collections/table_items.rs:337-900`). Reusing them, as the plan proposes, is justified.
- **No new public API**, and no privileged path: `local_bounds` is public, as the Table and ToastHost precedents show.

## Blockers

1. **Unstated dependency on `round-3-design-parts.md`.** Today the only route to menu rows is `find_descendant(&bar, "title/file")` followed by `"items"`. Those are structural keys, which `layout.rs:734-746` calls an implementation detail. The parts checkpoint exists to replace exactly that workaround (`&["title/file", "items"]`, `&["control"]`, `&["options"]`, `submenu/<key>` slots).
   - This plan asks for an "exact address route" and a doctest that "opens a menu and clicks a row" without saying which route to use or whether it lands before, with, or after the parts work.
   - Landing it first would publish the structural-key route as documentation.
   - The parts plan already asks for the same example ("opening File and clicking a menu row by item index").
   - **Fix:** make this plan depend on the parts checkpoint (or implement both together), name the routes it documents, and say which plan owns the menu doctest.

2. **Tabs scroll timing.** Wheel and arrow scrolling (`Tabs::scroll`, `tabs.rs:864-874`) update `offset` right away, but only invalidate layout. The offset is applied to `Header::rect` during arrange (`tabs.rs:1289-1292`).
   - Inspecting after `Runtime::pointer_input` without a flush would show the new `scroll_x` next to title bounds from the old position. That contradicts the `SemanticItem::bounds` rule and contracts.md:1551-1552, which say a scroll offset "shows at once".
   - It is consistent with today's hit testing, which is also stale until layout.
   - **Decide and state one of:**
     - (a) **Recommended:** keep the header rectangles unscrolled and subtract the live offset in `target`, `paint` and `inspect`, as Table and OptionList already do.
     - (b) Document that the strip's scroll takes effect at arrangement, and amend the contract wording.
   - Either way, add a test that scrolls without settling.

3. **Documentation, TODO and changelog are incomplete.**
   - **SemanticItem rustdoc:** it lists the built-ins that report bounds as only Table and ToastHost (`diagnostics.rs:369-371`). It needs the same update the plan gives `click_item` (`testing.rs:834-838`).
   - **Off-screen guidance:** `click_item` explains how to bring a scrolled-out table column into view, but has no equivalent for tab titles (arrows, wheel or keyboard) or list rows (wheel or keyboard).
   - **TODO.md:170-181:** the entry has to be narrowed in the same change. The Select anchor's ComboBox items (`select.rs:1321-1326`) will still have no bounds, by design: its rows are drawn by another component on another surface, and the runtime clips item bounds to the anchor. The parts plan says so; this plan should say it too, so the TODO entry doesn't keep "`Select`" open.
   - **CHANGELOG:** AGENTS.md requires an entry for the behavior change, and the plan doesn't mention it.

## Tests to add or specify (not blocking)

- **Closed list:** `click_item` on a closed menu's or Select's list. The Popover, Menu and Select (and MenuTitle) don't override `reveal_child`, so this should return `NotRevealed` (`AncestorRefused`) and run nothing. Document that too.
- **Disabled row:** state the expected outcome: the click succeeds through the driver, no command runs, and the menu stays open.
- **Narrow strip:** a strip narrower than two arrow widths gives no title any bounds.
- **Submenu row:** clicking a row inside an open submenu, using the parts route.
- **Detached title:** check whether a detached tab's title keeps its bounds (pressing it asks its window to activate). The plan says existing semantics are preserved but doesn't test this.

## Process note

The plan names Claude as both the design and implementation reviewer. AGENTS.md prefers a different model family from the author's, so record the author's family to show that's satisfied.

Once 1–3 are written into the plan, I'd consider it ready under the delegated round-3 plan.
