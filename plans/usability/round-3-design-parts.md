# Round 3 checkpoint: addressable control parts

Status: proposed, 2026-10-28; not implemented or execution-verified.

The head settings-editor submission
[r3-1-head-claude](runs/r3-1-head-claude/FRICTION.md) cannot read its Select's
value through the component it mounted or through Driver's address walk.
The head notes-editor submission
[r3-6-head-claude-1](runs/r3-6-head-claude-1/FRICTION.md) cannot find a
MenuBar title by its reported key. Both resort to a recursive logical-key
lookup. This proposal gives these controls stable public address routes,
instead of documenting those traversal workarounds as the preferred API.
It complements the [item geometry checkpoint](round-3-design-items.md).

## Proposed routes

Keep existing logical keys and retained identities. Change only the explicit
addressing descriptions using the public Element::part/slot extension points:

- Select and InlineSelect: their internal ComboBox anchor, keyed `select`,
  becomes the leaf part `control`; the open option list, keyed `options`,
  becomes the leaf part `options`. Driver::find(owner, &["control"]) reaches
  the ComboBox semantics, and &["options"] reaches the ListBox items. The
  owner remains the surface wrapper; do not duplicate the anchor's semantics
  on it. Open the control by clicking the control part, then inspect/click
  the options part. Items of the closed ComboBox remain descriptions of
  options, not extra pointer rectangles for options that are not drawn.
- MenuBar titles: retain logical key `title/<menu-key>`, but give each title
  an inherited slot with that same name. It is one path segment, including
  the literal slash: &["title/file"]. Menu descendants are under that title
  slot, which disambiguates their repeated internal keys across titles.
- A menu level's OptionList becomes leaf part `items`. The root level is
  reached from its Menu, ContextMenu or MenuBar title with &["items"].
  Each submenu panel becomes an inherited slot `submenu/<row-key>`; its
  rows are at &["submenu/more", "items"], for example. Separators add no
  item or address segment. Nested submenus repeat that vocabulary.

The MenuBar route is therefore &["title/file", "items"], not a pair of
unrelated recursive searches. Select's popup wrappers and menu layout panels
stay internal structure. No application-supplied anchor/content addresses are
changed by this proposal; in particular, do not change Popover or Dialog
addressing or surface ownership as a side effect.

These routes replace manual tree walks and find_descendant guesses as the
recommended way to test these parts. Runtime::find_descendant remains the
public generic discovery operation. No public method or compatibility alias
is added, and no old supported address route to these currently structural
parts is removed. New part/slot names become documented reserved vocabulary.
The menu-key suffix retains exactly the application's existing stable key,
never a label, display index or caption.

## Invariants and persistence review

- Marking is fixed at mount; same-key redescription must retain the same
  part/slot choice and incarnation. Preserve focus, captures, callback
  routing, availability and parameter authority. Do not publish from
  reconciliation or add scheduling for addressing.
- No helper gains privileged access: an external composite reproduces this
  through public Element::part/slot, Runtime::find and Driver.
- Submenus need slots, not just parts: parts do not contribute their names
  to descendants. A single `items` part without title/submenu namespaces
  would make different levels ambiguous even while some are hidden.
- Public snapshot types and their field meanings do not change. The new
  paths are new addresses for formerly structural internal components.
- Confirm before adoption that these internal title, list and submenu-panel
  components currently contribute no workspace state, and that the changed
  namespaces do not contain capturable participants. Preserve all existing
  application-content workspace paths. Verify with a MenuBar whose content
  contains Tabs or splits and a workspace round trip. If any stored path is
  reinterpreted, stop and revise the checkpoint to a separately versioned
  schema change; unpublished API flexibility does not waive that rule.
- Keep logical-key-based discovery working; it is a supported generic API,
  although it is no longer needed for these specific control parts.

## Documentation and evidence

Document exact segments, which component reports which role/value/items,
closed/hidden list behavior, pointer item bounds and the distinction between
logical keys and public addresses. Add complete public-API examples for
reading a Select's current value and selecting an option, and for opening
File and clicking a menu row by item index. Document Menu and ContextMenu
submenu routes too, and cross-link from the testing guide. Migrate relevant
repository test consumers to prove the ordinary path; do not use private
imports or a test-only inspection escape hatch.

Before closure, against the frozen head establish the reported NoSemantics
and missing-address behavior; against the repair test control/list parts,
multiple MenuBar titles and nested submenus with repeated row keys, disabled
rows, dynamic option updates, same-key redescription and callback selection.
Check that old content addresses and workspace capture/restore are unchanged,
that logical-key lookup still returns the same IDs, and that inspection of
settled trees adds no work. Combine pointer assertions with the geometry
checkpoint's tests rather than inventing a semantic-action shortcut.

Run workspace and API-listing gates, focused addressing/persistence tests,
new doctests and warning-free rustdoc, and obtain fresh Claude design and
implementation review. This checkpoint does not mark pending execution or
native evidence as passed.
