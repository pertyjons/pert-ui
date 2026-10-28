## Verdict

- **Parts (`round-3-design-parts.md` rev 2): not ready yet.** It does fix both revision-1 blockers, but I found two new ones. Both are small edits to the plan.
- **Items/geometry (`round-3-design-items.md` rev 2): the design itself is ready.** It can't land until the parts checkpoint is approved, because it depends on it.

I only read the plans and source. I edited nothing and ran no tests.

## Parts: blockers

**1. Lookup scopes leak through composites that are nested structurally.**
- **The rule:** lookup walks through structural children and stops only at exposed parts and at ordinary keyed children.
- **Why it leaks:** a built-in that embeds another composite as a structural child therefore pulls that composite's parts into its own scope.
- **The plan's own clipboard-menu change creates instances right away:**
  - `MultiSelect` mounts its `SearchField` as a structural child (`multi_select.rs:748`).
  - The `SearchField` owns the clipboard menu (`text_fields/search.rs:580`), which leads down to the `items` list (`menu.rs:646, 1117`).
  - So `find_part(multi_select, "items")` returns the search field's clipboard rows, and `Driver::part`'s "available names" for `MultiSelect` lists `items`.
  - `QuickPick` does the same (`quick_pick.rs:1340-1346`).
  - A custom composite that wraps a `Select` structurally inherits `control` and `options` the same way.
- **What's missing:** the plan says a part name is a scoped selector, but it only promises unique names in the scopes it advertises.
- **Fix:**
  - State the merge rule in the plan.
  - Have built-ins that embed a part-exposing composite structurally expose that composite under its own name (e.g. `search`), so its parts stay in their own scope.
  - Add a test that lists the available names for those built-ins.
  - Tell custom-composite authors to do the same.

**2. The error and window contract is unspecified, and partly wrong about current code.**
- **Window check:** the plan says `Driver::part` gets "the same selected-window and identity checks as Driver::find". `Driver::find` has no window check (`testing.rs:633-658`); only the input steps call `require_selected` (`testing.rs:804, 926`).
  - Pick one behaviour. I recommend no window check: lookup is structural and should work for detached owners, as `find` does.
  - Then say what the "other-window IDs" test expects.
- **Error variants:** none are named.
  - `find_part` returns `RuntimeError`, so `AmbiguousPart` has to be a `RuntimeError` variant. Say so.
  - Name the `TestError` variant for "part not found", and say how it identifies the owner. Element keys aren't unique.
  - A blank name is meant to fail "like an element key" with "a typed LayoutError". But element keys fail as `RuntimeError::EmptyKey` (`tree.rs:58-60`); `LayoutError::BlankKey` is for the item keys a container validates. Name the actual variant.
- Without these names, the API listing and the CHANGELOG can't be reviewed against the plan.

## Parts: should fix (not blocking)

- **The name `part` now means two different things.** `Element::part` marks a workspace (saved-layout) position, `Element::expose_part` declares a lookup name, and `Driver::part` finds only the second kind. For example, `RegionLayout`'s dividers are reached with `find`, not `part`. The library is unpublished, so renaming one of them is allowed. At minimum, `Driver::part`'s docs should say this.
- **No way to list a composite's parts.** The r3-6 tester asked for a helper that lists addresses. Computing "available names" for the error already enumerates the scope, so a `Driver::parts(owner)` alongside `address_children` costs little. The facade's `testing` module lists its types explicitly (`pert-ui/src/lib.rs:359`), so any new type must be added there.
- **The inspection field has no owner.** A snapshot record saying `exposed_part: "items"` can't be resolved by external tooling. Snapshots don't say whether an element is structural, keyed or a workspace part, so a tool can't repeat the walk. Either add the scope owner's id (a short walk up through structural ancestors) or document that the field is descriptive only.
- **Icon menus:** for `Menu::icon_with_state`, the structural trigger is a `Tooltip` wrapping the inner `button` (`menu.rs:827-835`). Expose `trigger` on the button so `Driver::semantics` works.
- **Serde:** inspection types currently validate only `schema_version` (`diagnostics.rs:1148`). Putting a field-level `deserialize_with` on an `Option` field makes a missing field an error. That's fine under the closed schema 17, but the plan should say so.
- **Name-only redescription:** the metadata loop invalidates the parent's layout on any metadata change (`tree.rs:575-577`). A part-name change shouldn't trigger that.
- **Glob sentence:** "include the returned types in the globs" says nothing concrete, since no new types are introduced. Make it concrete or drop it.

## Parts: checked and correct

- **Naming is preserved.** Labels and Field names flow only through structural elements (`inspect.rs:118-137, 351-359`), and the new metadata doesn't change that. This fixes r1 blocker 1.
- **No collisions with application keys or saved workspaces.** `find`, `address_children` and workspace capture read only the existing addressing (`tree.rs:342, 393`; `workspace.rs:257`). This fixes r1 blocker 2.
  - Application content in a `ContextMenu` (`popover.rs:217-225`) and in `MenuBar`'s leading, trailing and content areas (`menu_bar.rs:742, 773, 785`) is keyed, so lookup stops there.
- **Every advertised route resolves uniquely.** No keyed element sits on the internal path:
  - Select: `select.rs:1663-1665`.
  - Menu: `menu.rs:783, 860, 1041-1052, 1117`.
  - MenuBar titles: `menu_bar.rs:698-721`; the title's menu is its child (`menu_bar.rs:232-234`).
- **Validation can happen before anything changes.** `validate_traced` runs before any mutation in both mount and reconcile (`tree.rs:105, 513`), and the metadata-change path (`tree.rs:991-1027`) can carry the new name.
- **Snapshot cost** is one optional string clone per record (`inspect.rs:208-226`).
- **Schema 17** follows the "a field was added" rule. The 15/16 example in the error message, the guide and `serialization.rs:763` need updating.
- **It solves both observed cases:**
  - r3-1: `part(&select, "control")` gives the ComboBox semantics.
  - r3-6: `part(bar, "title/file")`, click it, `part(title, "items")`, then `click_item` (with the geometry change).
- **No privileged path for built-ins.** Custom composites use the same public `Element` builder.
- **Opening a ContextMenu:** there is no driver helper for a secondary click, so the doctest has to use `pointer_input` or the context-menu key.

## Geometry (items)

All three revision-1 blockers are addressed: it now depends on the parts checkpoint, it picks the live scroll offset, and the docs, TODO and CHANGELOG items are listed.

**The live-offset design works.**
- `Header::rect` is read in exactly these places: `boundary_at` (`tabs.rs:666`), arrange (`tabs.rs:1281-1291`), `target` (`tabs.rs:861`), paint (`tabs.rs:1650, 1688`) and the drop line (`tabs.rs:1739-1742`).
- `header_space` (`tabs.rs:831-841`) and the clamp in `scroll` (`tabs.rs:864-874`) both read live values.
- Arrange's reveal logic already works in unscrolled coordinates.
- Item bounds are computed against committed geometry without a dirty check (`inspect.rs:195-207`), so inspecting right after a scroll will be consistent.
- Optional: `scroll` could then invalidate only paint instead of layout.

**OptionList is fine too.**
- The height is cached in arrange (`select.rs:513`).
- Paint already uses the full width (`select.rs:687`).
- Hit testing uses the live offset (`select.rs:195`).

The only gating item is that the plan says it lands with the "approved" parts checkpoint, which isn't approved yet.
