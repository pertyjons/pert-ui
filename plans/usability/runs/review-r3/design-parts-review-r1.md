I'm not making it ready for implementation yet. The routing itself works, but two things in the current code would break existing behaviour if the plan went ahead as written. I only read code: I edited nothing and ran no tests.

## Blocker 1: Turning the Select anchor into a part stops labels and Field names from reaching it

The naming walk treats only `Addressing::Structural` as see-through. A part or slot is not:

- `runtime/inspect.rs:118-137` `element_label_name` only climbs while `current.addressing == Structural`.
- `inspect.rs:351-363` `describe_many` stops collecting naming parents at the first non-structural entry.
- `inspect.rs:516` `name_children` requires `structural.contains(&child)` at every step.

Today the anchor (`select.rs:1664`) and its `anchor` wrapper are structural. That is how `Element::with_label("Mode")` on the Select element, and a wrapping `Field`'s name, error and warning, reach the ComboBox. `docs/contracts.md:1513-1521` makes this a contract and uses Select as its example. If the anchor becomes `.part("control")`, it loses the name, `labelled_by` and the Field's error/warning. Two existing tests would fail: `crates/pert-ui/tests/foundation/semantic_names.rs:49` (`a_composites_label_names_the_part_that_represents_it`) and `:74` (`a_field_names_a_composites_part_through_its_structure`). The menu `items` lists lose label inheritance the same way.

The plan says "Public snapshot types and their field meanings do not change" but doesn't cover naming at all. It needs to decide how a named part keeps naming transparency. One option is to separate "adds an address segment" from "transparent to naming" in the shared rule, so custom components get it too. It must then show the effect on parts that exist today: `tabs.rs:145` `viewport`, `region_layout.rs` dividers and shell, `windows.rs:518` `content`. Those two tests need to pass unchanged.

## Blocker 2: ContextMenu (and MenuBar) share their address namespace with application content

`ContextMenu::new` passes the application's `content` to `set_list_content` (`menu.rs:1439`). The content goes inside the structural `anchor` wrapper, so its own key addresses directly under the ContextMenu. The menu's `actions`, and with this plan the new part `items` and the slots `submenu/<k>`, would sit under the same owner path.

So content keyed `items`, which is an ordinary key and one contracts.md:258 itself uses, would collide at `<ctx>/items`:

- `Runtime::find` and `Driver::find` would return `AmbiguousKey`.
- If that content is a workspace participant (Table, Tabs, SplitPane), `workspace_index` (`runtime/workspace.rs:220`) fails the whole capture with `DuplicatePath`, because `serials.len() > 1`. Nonparticipants count towards collisions (`:191-196`).

That turns a currently valid saved arrangement into a failing capture, which is exactly what the plan's own stop rule covers. MenuBar has the same kind of risk: content, leading or trailing content keyed `title/<k>` sits at the same `bar/…` level as the new title slots (`menu_bar.rs:742, 773, 785`).

The plan's proposed check (a MenuBar with Tabs and a workspace round trip) wouldn't catch this. The plan needs to:
- name which owners host application content beside the reserved names;
- choose either a namespace that cannot collide or a documented, tested collision behaviour;
- add a ContextMenu test whose content is a participant keyed `items` (and the MenuBar equivalent) to the persistence evidence.

## Needed before implementation, but not blocking

- **Every built-in menu changes, not just Menu, ContextMenu and MenuBar.** "A menu level's OptionList becomes leaf part `items`" goes through `menu_panel`/`MenuBranch`. That also gives every `TextField` and `SelectableText` clipboard menu (`text_fields.rs:1110`, `selectable_text.rs:332`) an `items` part. These have no application content, so there's no collision, but their inspection paths and `Selector::path` matches change. The plan should either include and document these or limit the change to the three public menus.
- **Some observable values change.** Inspection `path` for these components goes from `None` to `Some(...)` (`inspect.rs:212`). Content-search hit paths change from the owner's path to e.g. `…/control` (`content_search.rs:313`). Both should be recorded in the CHANGELOG and covered by the evidence.
- **The rustdoc for `Element::part` and `Element::slot` doesn't fit this use.** It defines a part as "an internal element whose own state is worth persisting" and a slot as positions for "application-supplied children" (`layout.rs:751-776`). Here they would be addressing-only names on stateless internals, so that rustdoc and the contracts.md addressing text need updating together. Also, a slot named the same as its key (`title/<k>`, `submenu/<k>`) behaves exactly like removing `.structural()` (`tree.rs:2183-2184`, `workspace.rs:258-259`). The plan should pick one way to express it.
- **No route for opening a Menu or ContextMenu.** The plan documents the routes to their rows but not how a test opens a button Menu (its `trigger` stays structural) or a ContextMenu, so the "complete public-API examples" requirement isn't covered for them.

## Checked and correct

- **Lookups resolve as intended.** With title and submenu slots plus `items` parts, `find(bar, ["title/file","items"])`, `["submenu/more","items"]` and nested submenus are unique (`tree.rs:337-361`). Without the slots, `items` would be ambiguous across titles, as the plan says.
- **No route is removed.** All the affected elements are structural today (`select.rs:1664-1665`, `menu_bar.rs:721`, `menu.rs:1052, 1117`).
- **The slash in `title/file` is safe.** It works as a single segment because `WorkspacePath` escapes `/` (`workspace.rs:413, 1100`).
- **No affected component stores workspace state.** None of the components being marked (SelectAnchor, OptionList, MenuTitle, MenuBranch, the panels) implements `workspace_state`, so no new participants appear.
- **Ownership, identity and callbacks are unaffected.** Keys are unchanged, so reconciliation, `MenuBar::slots`/`update` lookups by `title/` key, and the open-state, callback and session wiring are untouched.
- **A custom component can reproduce it.** The plan uses only public `Element::part`/`slot` and adds no privileged path.
- **Popover and Dialog are untouched.** `set_list_content` doesn't need to change, so the "no Popover/Dialog addressing change" claim holds.
- **The motivating reports match the code:** r3-1's `NoSemantics` and empty `address_children`, and r3-6's failing `["title/file"]` lookup.
