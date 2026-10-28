# Round 2 fix plan, part P8 design checkpoint: tables in tests

Part P8 of [the round 2 fix plan](round-2-fix-plan.md#p8-tables-in-tests-0383)
closes R2-22, R2-23 and R2-27 by decision D8 (approved with the recommended
options on 2026-10-28). The plan leaves to this checkpoint the geometry
contract and its edge cases, the inspection schema step with decoder
validation, which items fill bounds now, the validation code, and Codex's
three qualifications (presentation geometry, correlation with layout,
`click_item` as pointer input). This document settles them, with the exact
API, behavior rules, migration manifest, tests, CHANGELOG, gate and native
evidence, before any code. Codex reviews it; the author approves it together
with Codex's review under the maintainer's delegation of 2026-10-28.

Every reference to library code is to the committed tree at 405501e, whose
library code is that of 84bb60f (0.3.76, P1); the plan's line numbers are at
26e6b5a and P1 moved many of them, so each is re-cited here at 405501e.
P2 was committed while this was written (2b38625, 0.3.77); it changes only
the `testing` module's re-exports and pages (`R/testing.rs` grows by about
90 lines above the driver, `R/lib.rs` the prelude), so T's line numbers move
and are relocated by content when P8 starts, as are the guide lines P2's
page edits move (for example `G/diagnostics.md`); P2's review fix
(bedddd4) and its later fixes change only tests and the plan, and P1's
review fix (0f9638f) changes rustdoc in `R/runtime/commands.rs` and
`R/text_fields/number_inputs.rs`, which P8 does not cite, and moves guide
lines (`G/diagnostics.md`, `G/testing.md`), relocated by content; revision
3 checked its new citations at 0eb8baf, where P1 and P2 are done. Path
prefixes are the plan's: `R/` = `crates/pert-ui-runtime/src/`, `F/` =
`crates/pert-ui/src/`, `G/` = `crates/pert-ui/src/guide/`; **TB** `R/table.rs`,
**T** `R/testing.rs`, **DG** `R/diagnostics.rs`, **TR**
`R/runtime/tree.rs`, **IN** `R/runtime/inspect.rs`, **V** `R/validation.rs`,
**VD** `R/validation/development.rs`, **U** `~/pert-ui-usability/` (frozen
runs).

**Dependencies.** P8 lands after P7 (0.3.82). Approved and committed:
P2 (`round-2-design-P2.md`, revision 3, 07eb0a4), P4
(`round-2-design-P4.md`, revision 3, 5942c54) and P5
(`round-2-design-P5.md`, revision 3, 416dba7); P6's design was approved
and committed while this was written (`round-2-design-P6.md`, revision 3,
fec6a5f), and P8 takes from it only `ProgressBar::new(ProgressStatus)`,
which D1 already fixes. Not approved: P7 (its design is under revision);
P8 takes from it only what the approved D7 guarantees, that a growing bar
beside its label in a narrow cell lays out without `EscapesParent`;
section 5 says what follows if P7's checkpoint turns out otherwise. P9
fills the item shape fixed here for toasts without a second schema step
(plan constraint "P8 before P9").

Status: **revision 5 with the author's correction of Codex's fourth
recheck, approved 2026-10-28** by the author together with Codex's review
(fourth recheck: ready for approval, one nit on test 21d, corrected by the
author as Codex proposed), under the maintainer's delegation of
2026-10-28, with P8-D1 to P8-D4 as recommended. Revision 5 answered
Codex's third recheck, after its review of revision 1 and its rechecks of
revisions 2 and 3 ([review record](#review-record)).

## What testers built by hand, and what the library says

- **Header clicks by computed position (R2-27, results row at
  `plans/archive/developer-usability.md:945`).** 5-r1-claude-1 summed its
  own column widths from the table's left edge and clicked 8 px below the
  table's top (U `r2-5-r1-claude-1/work/src/headless.rs:131-143`);
  5-head-claude-2 took a mounted cell's x and width and clicked at the
  table's top plus 8 px, then advanced the clock past the double-click
  interval (U `r2-5-head-claude-2/work/tests/headless.rs:229-243`). The
  header titles are `SemanticItem`s on the table with no geometry
  (TB:3042-3062; DG:357-402), and `column_widths` (TB:3018-3031) is the only
  geometry inspection gives. Both codex runs used the keyboard route
  (Alt+Up/Down on the active column, TB:1212-1219; triage
  `runs/triage-r2/group-4.md:48`).
- **The cell address (R2-22, `plans/archive/developer-usability.md:940`).**
  5-head-claude-2 found by trial that rows are
  `["content", "jobs", "job1", "state", "state"]` and that `NotFound` did not
  say which segment failed (U `r2-5-head-claude-2/work/FRICTION.md:100`).
  `TableItems` says only "the element's own key is free, since the table
  wraps each cell" (TB:239); each cell is `Element::new(column.key,
  TableCell { .. })` (TB:531-538) inside `Element::new(row_key, TableRow
  { .. })` (TB:541-550) inside the structural `body` (TB:2270). Content keyed
  like its column makes `find_descendant(&row, key)` fail with
  `AmbiguousKey` (probe, triage `group-4.md:45`; `group-3.md:119`).
  `TestError::NotFound` holds the whole path (T:717-721).
- **Row height (R2-23, `plans/archive/developer-usability.md:941`).**
  5-head-claude-1 computed `ControlHeight + ROW_PADDING` through
  `theme.metrics(theme.density()?)?.metric(MetricRole::ControlHeight)`
  (U `r2-5-head-claude-1/work/src/lib.rs:420-421`); 5-head-claude-2 saw the
  Cancel button cut off in a capture, raised the row to 36 px and asked for
  a validation report (U `r2-5-head-claude-2/work/FRICTION.md:126-127`).
  Probe: a `Button` in a 28 px row has 32 px bounds and the report is empty
  (triage `group-3.md:39-44`).

## 1. Today's behavior, from the code

- **Cells.** `TableCell::arrange` measures its content at the cell's width
  less padding and at most the row's height, and places it at
  `min(measured, row)` height, centered (TB:967-982). `arrange_node`
  constrains every placement by the child's own limits (TR:1706), so a
  `Button`, whose minimum height is ControlHeight (`R/controls.rs:734-739`;
  28, 32 and 40 px at Compact, Standard and Comfortable, `R/runtime/theme.rs:205`,
  `:28-36`),
  keeps 32 px in a 28 px row. Controls without a hard minimum still
  **measure** ControlHeight whatever height is offered: a single-line
  `TextField` (`R/text_fields.rs:2496-2510`) and a `Checkbox` or `Toggle`
  (`R/choice_controls.rs:324-333`); the cell places them at the row's 28 px
  (TB:977), so they are drawn compressed below what they measured, and a
  wrapping `Label` (it wraps against the offered width,
  `R/controls.rs:297-314`) measured over several lines is drawn at the row's
  height with its lower lines cut off. `measure_node` caches each
  component's last measurement with the constraints it was asked under
  (TR:1656-1691, `Node::measured`, `R/runtime.rs:858`).
  `TableRow` places each cell with
  `place_clipped(cell, cell)` (TB:1078), so the excess is cut off.
  Validation's `EscapesParent` compares the clipped visible rectangle with
  the parent's box (VD:612-632, `visible_rect` VD:496-508), so a clipped
  child is never reported. Content keyed like the editor host is wrapped in
  a structural `CellContent` (TB:521-527, :917-933), and an edited cell holds
  a structural `EditHost` (`R/table/editing.rs:286-303`); both give their
  child exactly their own size (TB:930-932, `R/table/editing.rs:718-723`).
  Neither wrapper adds an address segment.
- **Header geometry.** `Table::arrange` sets the header height
  (`ControlHeight.min(size.height())`), the size, the grip width and the
  resolved columns (`Shared::resolve`, TB:362-400, `(x, width)` per column,
  TB:320-322) (TB:2465-2500). Paint fills each column's header rectangle
  `[start - offset, 0, width, header]` intersected with the table's size
  (TB:2516-2525); hit testing finds the column with `column_at` and a
  resize edge with `edge_at`, both in local coordinates adding the
  horizontal offset (TB:1898-1925), and a press checks the edge first
  (TB:2643-2659), so a press within ResizeWidth/2 of a column's trailing
  edge resizes instead of sorting (ResizeWidth 6, 8, 12 px,
  `R/runtime/theme.rs:211`). A title release sorts when it is still over
  the pressed column (TB:2730-2752). Horizontal scrolling changes the offset
  and invalidates paint only (`scroll`, TB:1861-1867); a column move
  resolves at once (`move_to`, TB:469-485); a restored workspace reorders
  `columns` without resolving until the next arrangement (TB:2216-2247),
  so `columns[i]` and `resolved[i]` describe different columns until then,
  for hit testing as for anything else that pairs them by index.
- **Inspection geometry.** `describe_component` fills `border_box` from the
  node's committed `bounds` and the clip from its committed `clip` at
  `Detail::Geometry` and above (IN:186-189); semantics, and so items, are
  present at `Detail::Semantics` and `Full` (IN:145-155;
  `R/inspection_service.rs:169-182`). Bounds are `[x, y, w·s, h·s]` in the
  presentation window's logical pixels, `s` the node's accumulated view
  scale (TR:1705-1708), and the clip is the inherited clip intersected with
  the bounds (TR:1710). `EventContext::to_local` is the inverse,
  `(p - origin) / s` (`R/runtime.rs:922-930`, `:3187-3189`). A promoted
  surface or a detached view inherits scale 1 (TR:797-806) and is arranged
  in its own window's coordinates (TR:1821-1879). **The view scale is not
  staged:** `apply_view_scale` replaces `node.scale` at once (TR:811-827),
  called by `reparent` before any layout (TR:765) and by `sync_view_scale`
  after a component's update changes its `child_transform` scale
  (TR:890-904, `R/runtime.rs:6174`), and `ComponentInspection::view_scale`
  reports that current value (IN:200), while `border_box` and the clip are
  the last committed layout's. Between such a change and a successful
  layout, the reported scale and the reported bounds describe different
  states. **Nor is the window:** detaching and redocking move a view's
  subtree with `move_presentation` (`R/runtime/native_windows.rs:630`,
  `:705`), which replaces each node's window at once (`:657`) and
  invalidates its layout (`:664`) but keeps its bounds and clip, while
  inspection reports the new window (IN:198); until the destination's
  layout succeeds, a record pairs the source window's geometry with the
  destination window. Hidden components keep
  their last bounds (TR:347-350); `Node::is_presented` is false for a hidden
  component, a hidden window or an empty clip (`R/runtime.rs:903-907`).
- **Layout staging.** `layout` collects every arrangement in `staged` and
  commits bounds, size, clip and placements only when the whole pass
  succeeded (TR:1893-1997; commit loop TR:1934-1992); a failure returns
  before the commit and before `layout_pending` is cleared (TR:1993). A
  component's own `arrange` writes its retained state at once (the table's
  `resolve`, TB:2480), so after a failed pass that state may describe the
  failed arrangement while its committed bounds are older. The next pass
  arranges a component again only when it is layout-dirty, its geometry
  differs from the committed one, or its clip changed (TR:1717-1724, the
  leaf shortcut TR:1728-1741, the retained-placement path TR:1748-1757); a
  component whose failed-pass inputs return to the committed ones is
  skipped and keeps the failed pass's retained state (by reading; test 10
  of section 7 shows it).
- **The validator** keeps per node the hints from the inspection after the
  component's last update (`R/runtime.rs:6106-6132`) and the geometry of the
  last commit (`Validator::geometry`, VD:105-124, called from TR:1983-1990),
  and decides whether to revalidate by comparing nodes field by field
  (`same_node`, VD:428-444, which lists each hint).
- **Envelopes.** `InspectionSnapshot` and `QueryResult` derive `Deserialize`
  with `deny_unknown_fields` and carry `schema_version` as a plain `u32`
  (DG:989-1036, `R/inspection_service.rs:372-401`); `SCHEMA_VERSION` is 15
  (DG:1046), and an external inspector is told to compare it before
  decoding (DG:1040-1046). `SemanticItem` and `ValidationHints` deny unknown
  fields (DG:377-383, V:95-100). The workspace document checks its version
  in its `Deserialize` impl with `WorkspaceError::UnsupportedVersion`
  (`R/workspace.rs:1729-1736`).
- **The driver.** `Driver::find` maps `Ok(None)` to `NotFound { path }`
  (T:359-365); `click` checks the window and neutral input, reveals, reads
  `Runtime::bounds` and presses and releases at the middle (T:425-449).
  `Runtime::find` refuses only several matches of the **last** segment
  (TR:322-329); a prefix can match several components.

## 2. The rules

### 2.1 Cell addresses (R2-22)

Documentation only; the addresses do not change.

1. A row is addressed by its key below the table, a cell by its column's
   key below the row, and the content `TableItems::cell` returned by its own
   key below the cell: `<table>/<row key>/<column key>/<content key>`. The
   `body` and the edit-end reporter are structural, and so are the two
   table wrappers a cell may hold (`CellContent`, `EditHost`), so none adds
   a segment; an edited cell's editor is at
   `<table>/<row key>/<column key>/<editor key>`.
2. Content may be keyed like its column (keys are unique among siblings);
   `find_descendant(&row, key)` then sees the cell and its content and fails
   with `AmbiguousKey`, and the full address resolves the content.
3. `Driver::address_children(&table)` (P5) lists the mounted rows in source
   order; `Runtime::children(&table)` lists the body and the reporter.

### 2.2 `TestError::NotFound`'s resolved prefix (R2-22)

1. `NotFound` gains `resolved: Vec<String>`: the longest leading part of
   `path`, shorter than `path`, that resolves below `from`. A prefix
   resolves when `Runtime::find(from, prefix)` returns a component **or
   refuses it with `AmbiguousKey`**, since `find` refuses only at the last
   segment and a prefix reaching two components still reached the path's
   level. The empty prefix (`from` itself) always resolves, so `resolved` is
   empty when not even the first segment does.
2. Computed only on the failure path, longest first: at most
   `path.len() - 1` further `find` calls. Other errors cannot occur for a
   `from` the first lookup accepted; one that does is returned as
   `TestError::Runtime`.
3. Message: `no component is mounted at r1/missing; the path resolves as
   far as r1`, and with an empty prefix `no component is mounted at
   r9/name; not even its first segment resolves`.

### 2.3 Item rectangles (R2-27)

The geometry contract, for built-in and external components alike.

1. **Reporting.** A component gives an item a rectangle with
   `SemanticItem::local_bounds(rect)`, in its **own local coordinates**: the
   origin at its top-left, in local logical pixels, the space its `paint`
   draws in and `EventContext::to_local` maps to. The rectangle is the one
   its painting and pointer handling use for the item now: computed from
   the geometry its last `arrange` retained and from state it changes
   outside layout that its painting and hit testing read, such as a scroll
   offset. An item without one has no bounds.
2. **Transform.** When an inspection describes a component with semantics
   (`Runtime::inspect`, `inspect_with`, and `query` at `Detail::Semantics`
   or `Full`, IN:139-213), the runtime sets every item's `bounds`:
   with the component's committed bounds origin `(bx, by)`, its committed
   view scale `s` (rule 8; the `view_scale` it reports) and its committed
   clip `C`, the window
   rectangle is `[bx + s·x, by + s·y, s·w, s·h]`, computed in `f64`, and
   `bounds` is its intersection with `C` as `[x, y, width, height]`. These
   are the values `border_box`, `view_scale` and the clip report, so
   `bounds` follows the committed **presentation** geometry exactly as they
   do: an item of a component in a promoted surface or a detached window is
   in that window's logical coordinates at that presentation's scale
   (TR:797-806), and `ComponentInspection::window` names the window.
3. **Absent** (`None`), never a guess, when: the item has no local
   rectangle; the component has no committed bounds or clip (before its
   first layout); its committed geometry is from another window than the
   one it is now presented in (rule 9); it is not presented
   (`Node::is_presented`: hidden, its
   window hidden, or clipped away entirely), although `border_box` keeps its
   last bounds; the visible part is empty, as for an item scrolled or
   clipped fully out of view; the arithmetic is not finite; or rule 5's
   guard is set.
4. **Pending layout.** Until a layout runs, a component's retained
   geometry and its committed bounds both describe the last successful
   layout, so a change not yet laid out shows in neither, as for
   `border_box`. State a component changes outside layout (rule 1) shows at
   once, as the next paint and the next pointer event see it.
5. **Failed layout (P8-D1).** Each node gains a crate-private flag,
   `arranged_uncommitted`, set immediately before `arrange_node` calls the
   component's `arrange` (TR:1782-1788) and cleared when the commit loop
   commits that node (TR:1934-1975). When `layout` fails after any
   arrangement, the runtime calls `invalidate_layout` (`R/runtime.rs:978-988`)
   on every node whose flag is set, a scan of the entries on the failure
   path only, so the next layout arranges each of them and their
   presentation ancestors again rather than skipping them by the shortcuts
   of TR:1717-1757. While a node's flag is set, its items have no bounds.
   So a failed pass never pairs its retained geometry with older committed
   bounds, and the next successful layout restores the pairing. Components
   the failed pass did not arrange keep the bounds of the last successful
   layout. The re-arrangement also repairs painting and hit testing of
   those components, which today keep the failed pass's retained state when
   the next pass skips them (section 1, "Layout staging"): a defect of the
   same kind in the code P8 touches, fixed here.
6. **Replacement.** The runtime sets every item's `bounds` by rules 2-5 and
   clears its local rectangle, so a value a component assigns to `bounds`
   directly is never reported, and a snapshot carries window bounds only.
7. **Cost.** Inspection adds a transform per item; layout adds one flag
   write, one scale and one window serial per staged component; a failed
   layout adds one scan of the entries; debug builds add 2.7 rule 2's list
   of changed measurements. Inspection never calls `arrange` and schedules nothing.
8. **The committed scale (P8-D4).** `Staged` (TR:6-24) gains the scale
   `arrange_node` computed the bounds with (TR:1707), on each of its pushes
   (the leaf shortcut and the full path), and the commit loop stores it in a
   new crate-private `Node::committed_scale: Cell<Option<f32>>` beside the
   bounds. Rule 2 uses it, never `node.scale`, so every input of the
   transform (origin, scale, clip) belongs to one successful layout: an
   inspection after `reparent` into another scale and before a flush, or
   after a scale-changing flush whose layout failed before reaching the
   component, reports the old bounds at the old scale, as rule 4 requires.
   `ComponentInspection::view_scale` is redefined to the same value: the
   scale of the committed geometry its `border_box`, clip and item bounds
   are in; before the component's first layout, when it has none of these,
   the scale it will be laid out at (`node.scale`). This is a
   reinterpretation of a field, part of schema 16 (2.6 rule 1); rule 9 gives
   the case of a moved component. A changed
   scale changes the component's window bounds (size × scale), so the next
   successful layout stages it, whether its layout was invalidated or, in a
   continuing zoom, its drawing is only stretched (TR:831-856). A changed
   scale does not always change the bounds, as for a component of zero
   size, so `arrange_node`'s early return (TR:1717-1724) also requires
   `committed_scale == Some(scale)`: a node whose scale changed is staged.
   It still reaches only the leaf shortcut or the retained-placement path
   when nothing else changed (TR:1728-1757), so no component is arranged
   for it. The two values therefore meet at every successful layout.
9. **The committed window.** The commit loop also records the serial of the
   node's window at commit in a crate-private
   `Node::committed_window: Cell<Option<u64>>`. While it differs from the
   node's current window (after a detach or a redock, until a layout of the
   destination commits the node), inspection reports no item bounds and no
   `border_box` or clip for the component, and `view_scale` is the scale it
   will be laid out at (`node.scale`), as before a first layout: its
   committed geometry belongs to a window it is no longer presented in. The
   withheld `border_box` and clip are a defect of the same kind fixed in the
   function P8 changes (IN:186-189); `Runtime::bounds` keeps documenting
   the last allocation (TR:347-350) and is unchanged. Every moved node has
   its layout invalidated (`R/runtime/native_windows.rs:664`), so the
   destination's next successful layout stages and commits each of them;
   one that fails before reaching the node leaves the mismatch, and
   nothing is reported until a layout succeeds.

### 2.4 The table's header items

1. Item `i` (column `i` in the current order, TB:3042-3062) gets
   `local_bounds([start - offset, 0, width, header])` from `resolved[i]`,
   the horizontal offset and the retained header height: the rectangle
   paint fills for that column (TB:2520-2525) and in which `column_at` finds
   it (TB:1915-1925). A column without a resolved entry, as before the first
   arrangement, gets none. Nothing else in the table's semantics changes.
2. **Restore resolves at once (same kind, fixed).** `restore_workspace_state`
   calls `shared.resolve(shared.width.get())` once the restore can no longer
   fail: after the fallible republication succeeded (TB:2255-2259), or after
   the column change when no reorder needs one, and never before it. The
   rollback path returns before resolving, so `resolved`, the horizontal
   offset (which `resolve` clamps, TB:398-399) and any width change still
   waiting for layout are exactly as before the call, as the contract that a
   rejected entry changes no preference requires (`docs/contracts.md:1435`).
   As `move_to` does after a move (TB:480), this keeps `columns` and
   `resolved` from being paired across a reorder: not by the items and not
   by hit testing, painting's sort marker or `edge_at`, which pair them by
   index today (section 1).
3. **Which items fill bounds in P8:** the table's header titles only. Every
   other built-in item reporter (section "Findings outside this part")
   reports none; `click_item` fails on their items with
   `ItemWithoutBounds`, and the `SemanticItem` page says that a component
   gives bounds only where it reports them.

### 2.5 `Driver::click_item`

`driver.click_item(&id, index)` clicks item `index` of `id`'s
`Semantics::items` with the primary button, then settles.

1. **Before any input, nothing changes on refusal:** `ForeignRuntime` or
   `Unmounted` as `TestError::Runtime`; `OtherWindow` for a target outside
   the selected window; `InputHeld`; then the items are read with
   `Driver::semantics`: `NoSemantics`, or `NoItem { key, index, count }`
   when `index >= count`.
2. **Reveal**, as `click` does (T:430-437): `NotRevealed` with the refusal,
   `Unmounted` for a target the reveal unmounted.
3. **Re-read** the items after the reveal, since revealing flushes and may
   scroll: `NoSemantics` or `NoItem` again if they changed;
   `ItemWithoutBounds { key, index }` when the item's `bounds` is `None`.
4. **Pointer input:** press and release at the centre of `bounds`, the
   item's visible part, through `Runtime::pointer_input`, with `click`'s
   cleanup rules, then one settle. What the press does is the component's:
   `click_item` guarantees no semantic action. On a `Table` header, a centre
   within the resize grip of a column's trailing edge, possible when a
   narrow visible part is left by horizontal scrolling, starts a resize, not
   a sort (TB:2643-2659), and the rustdoc says so. Whether the press reaches
   the component is not checked, as for `click`; two `click_item`s are one
   `FRAME` apart, so a component that counts clicks sees a double click.
5. It does not scroll a component's own content to the item: a table's
   column scrolled out of view has no bounds; the rustdoc names the routes
   (Left or Right on the focused body, which bring the active cell's column
   into view, or a horizontal wheel through `Runtime::pointer_input`).

### 2.6 Inspection schema 16 and decoder validation

1. `InspectionSnapshot::SCHEMA_VERSION` becomes **16**, for
   `SemanticItem::bounds`, `ValidationHints::fixed_height` and
   `ValidationCode::ExceedsFixedHeight` (each documented "Schema 16 added
   this field/code"), and for the reinterpreted
   `ComponentInspection::view_scale` (2.3 rule 8; documented "Schema 16
   made it the committed layout's scale"). `Workspace::SCHEMA_VERSION`
   stays 2.
2. **Decoder validation.** Both envelopes' `schema_version` fields get
   `#[cfg_attr(feature = "serde", serde(deserialize_with =
   "crate::diagnostics::current_schema"))]`, a crate-private function that
   reads a `u32` and returns `D::Error::custom` of a crate-private
   `UnsupportedSchema { found, supported }` (thiserror) for any value other
   than `SCHEMA_VERSION`, with the message `inspection schema version 15 is
   not supported (this build writes 16)`, the wording of
   `WorkspaceError::UnsupportedVersion` (`R/workspace.rs:618`). Serialization
   is unchanged. `deny_unknown_fields` stays on both types, so the closed
   schema is kept exactly; no mirror struct is needed (the plan's "for
   example `try_from`", see corrections). The obligation of an inspector to
   compare the version before decoding stays documented, for a tool that
   reads several revisions.
3. **Order.** `schema_version` becomes the first field of
   `InspectionSnapshot`, as it is of `QueryResult`, so serde visits it first:
   serializing an envelope directly with a serializer that keeps field
   order, such as `serde_json`, writes it first, and decoding such a
   document checks it before any other field. Nothing more is promised: the
   application chooses the format (`G/diagnostics.md:193`), and a format
   or an intermediate map may reorder keys. Decoding does not depend on the
   order: a document of another revision is rejected wherever its version
   is, possibly on an unknown or missing field first, and the page says so.
4. **Fields.** `SemanticItem::bounds` is `serde(default, with =
   "crate::serde_float::option_array4")`, as `border_box` (DG:111-115), so
   an item without the field decodes with no bounds; it serializes `null`
   when absent. The local rectangle is `#[serde(skip)]`.
   `ValidationHints::fixed_height` is required, as `interactive` (V:112) and
   schema 15's `warning` were.

### 2.7 Row height and `ExceedsFixedHeight` (R2-23)

The plan's requirement is kept: a cell whose content **measured** taller
than the row is reported (plan D8, `round-2-fix-plan.md:548-552`).

1. **The hint (P8-D3).** `ValidationHints::fixed_height: bool`, default
   false: the component gives its children a fixed height they must fit,
   with no way to scroll to the rest, as a table cell gives the row's.
2. **The measurement it is checked against** is the one the parent
   obtained: the size `measure_node` last returned for the child, cached in
   `Node::measured` with the constraints the parent asked under
   (TR:1688-1689). For a cell that is the measurement `TableCell::arrange`
   requests at the cell's width and the row's height (TB:976), already
   raised to the child's minimum (`limits.constrain`, TR:1688). Its height
   times the child's committed scale is recorded for the validator, which
   holds it in `NodeData` (VD:9-20):
   - **with geometry:** `Validator::geometry` (VD:105-124) gains a
     `measured_height: Option<f32>` argument, which the commit loop passes
     (TR:1983-1990), and the update-time record (`R/runtime.rs:6116-6131`)
     passes the cached value too;
   - **when only the measurement changed:** a parent can measure a clean
     child under new constraints and place it in the same rectangle, so the
     child is neither staged (TR:1717-1724) nor updated. In debug builds,
     each window's shared state holds a development-only set of node
     serials, `measure_changes: RefCell<BTreeSet<u64>>`, and no node holds
     any flag of its own, so nothing can be left marked:
     - **enqueue:** `measure_node` (TR:1656-1691), when it stores a size
       that differs from the cached one (an empty cache counts as
       different), inserts the node's serial into the set of the node's
       current window (`node.window()`); a set holds a serial at most once;
     - **drain:** after the commit loop of a **successful** layout of a
       window, the runtime passes each serial in that window's set that is
       still mounted and still in that window to a new
       `Validator::measured(id, height)` with its cached measurement (dirty
       only when the value changed), and empties the set; a failed layout
       leaves it, so its changes are committed with the next successful one,
       which reads the cache as it then is;
     - **move:** `move_presentation` (`R/runtime/native_windows.rs:642-666`)
       removes each moved serial from the source window's set beside the
       other per-window state it clears there (`:651-655`). The destination
       does not inherit it: the same function invalidates each moved node's
       layout (`:664`), which empties its measurement cache
       (`R/runtime.rs:978-981`), so the destination's next measurement of
       it counts as changed and inserts it into the destination's set;
     - **retirement:** a retired window's set goes with its state
       (`retire_window`, `R/runtime/native_windows.rs:730-745`); its views
       were moved out first (`:705-709`), so it holds nothing then;
     - **unmount:** removing a node deletes its serial from its window's set
       beside `self.validator.remove(serial)` (TR:1596).

     Each set is bounded by the mounted nodes of its window, and nothing is
     inserted by a flush that measures nothing, so an idle flush runs no
     validation pass. Release builds compile none of it.

   A child its parent never measured has none and is checked by its bounds
   alone. No measurement runs during inspection or validation.
3. **Why not a field a component reports.** The validator reads a
   component's `ValidationHints` from the inspection after its **update**
   (`R/runtime.rs:6106-6131`), before layout, and a cell is not updated
   again when only its layout changes; a measurement the cell put in its
   own hints would be one layout late and absent at the first. The
   measurement is the runtime's own result of the public
   `MeasureContext::measure`, so any external grid that sets the public hint
   gets the same report with no further hook; the public metadata is the
   hint.
4. **The check** (VD, in `check`, beside `EscapesParent`): for a visible
   component whose presentation parent is visible, in the same window, has
   `fixed_height` and a nonempty border box, the component's height is the
   larger of its recorded measured height and its unclipped border-box
   height; when that height, placed at the component's top, reaches below
   the parent's bottom, or its border box starts above the parent's top, by
   more than `GEOMETRY_TOLERANCE`, it is reported as
   `ValidationCode::ExceedsFixedHeight` with components `[component,
   parent]` and the message `Component "name" (#12): measured 32 px tall in
   "name" (#9), which gives its children a fixed 28 px height, so it is
   compressed or cut off; give the parent more height, such as a taller
   Table row, or use smaller content`. It is a geometric code (VD:388-400),
   so it is reported only for measured windows, and like all validation only
   in debug builds.
5. **Revalidation.** `same_node` (VD:428-444) compares `fixed_height` and
   the measured height, and `Validator::geometry`'s unchanged-early-return
   (VD:113-118) includes the measured height, so a change of either alone
   revalidates; otherwise a hint switched on or off on an unchanged
   component would never be reported or cleared.
6. **The table sets it at one boundary per cell.** `CellContent` and
   `EditHost` forward their child's measurement and place it at their own
   size (TB:925-930, `R/table/editing.rs:712-723`), so a hint on the cell and
   on the wrapper would report one problem twice, as `[wrapper, cell]` and
   `[content, wrapper]`, which deduplication keeps apart (VD:295-297). The
   boundary is the innermost table component around the application's
   content: `TableCell` reports `fixed_height` only when its child is the
   content itself, and `CellContent` and `EditHost` report it (each gets an
   `inspect` returning only the hint). `TableCell` learns which where
   `Rows::build` chooses its child (TB:519-538): a field recording that the
   child is the editor host or the content wrapper. The default `reconcile`
   keeps the mounted component and ignores the new description
   (`R/runtime.rs:1105-1110`), and a cell changes child when an edit begins
   or ends, so `TableCell` gains a `reconcile` that takes the field from the
   new description and, when it changed, returns `Some(Invalidation::Paint)`;
   that queues the cell's update (`Node::invalidate`, `R/runtime.rs:962-976`),
   after which development validation reads its hints again
   (`R/runtime.rs:6106-6131`). Each affected cell therefore has exactly one issue,
   naming the application's content or editor; its second component is the
   wrapper, whose key is the table's (`content`, `cell editor`), and the
   `Table` page says so.
7. **What it catches:** a `Button` or a container holding one (laid out
   taller); a `TextField`, `Checkbox`, `Toggle` or an in-place editor that
   measured ControlHeight in a shorter row (drawn compressed); a wrapping
   `Label` measured over more lines than the row holds (cut off). A label
   with `overflow(TextOverflow::Ellipsis)` measures one line and fits a row
   of a line's height.
8. **`Table::new` guidance:** the row is fixed; a cell gives its content at
   most that height; choose it for the tallest content: every single-line
   control, such as a button, a text field or a checkbox, measures at least
   ControlHeight (`R/controls.rs:734-739`, `R/text_fields.rs:2502-2504`,
   `R/choice_controls.rs:331-332`), so a row with controls, or with an
   editable column, is at least ControlHeight tall; text-only rows fit one
   line of body text with `TextOverflow::Ellipsis`; a larger control such
   as a `Knob` (KnobDiameter, `R/runtime/theme.rs:220`) needs its own height.
   ControlHeight is read as `let theme = ui.theme();
   theme.metrics(theme.density()?)?.metric(MetricRole::ControlHeight)`. A
   mounted table keeps the height it was created with, also across a
   density change and when described again (TB:1274-1276), and development
   validation then reports `ExceedsFixedHeight`; an application that lets
   the user switch density gives the table the height of the largest preset
   it offers, the largest of `theme.metrics(density)?.metric(ControlHeight)`
   over those `Density` values (40 px for `Comfortable` with the default
   metrics, `R/runtime/theme.rs:205`).

## 3. Public API

In `pert_ui_runtime` (crate root, `component` and `inspection` as today;
`SemanticItem` also in `testing` by P2):

```rust
pub struct SemanticItem {
    // … the seven existing fields, unchanged …
    /// The visible part of the row, `[x, y, width, height]` in the logical
    /// pixels of the component's window (`ComponentInspection::window`), the
    /// coordinates of `BoxModelInspection::border_box`. [2.3 rules 2-6: filled
    /// by the runtime from `local_bounds`; absent cases; pending and failed
    /// layouts; a directly assigned value is replaced.] Schema 16 added this
    /// field.
    #[cfg_attr(feature = "serde", serde(default, with = "crate::serde_float::option_array4"))]
    pub bounds: Option<[f32; 4]>,
    #[cfg_attr(feature = "serde", serde(skip))]
    local: Option<Rect>, // private; cleared by the runtime (2.3 rule 6)
}
impl SemanticItem {
    /// Gives the row a rectangle in the reporting component's own local
    /// coordinates. [2.3 rule 1: the space of `paint` and
    /// `EventContext::to_local`; the geometry its painting and pointer
    /// handling use; the runtime transforms and clips it into `bounds`.
    /// Example: a custom two-segment component reporting both segments,
    /// mounted at an offset; `Driver::semantics` gives their window bounds.]
    #[must_use]
    pub const fn local_bounds(mut self, rect: Rect) -> Self;
}

pub struct ValidationHints {
    // … overlap, clip, interactive, shortcuts, unchanged …
    /// Whether this component gives its children a fixed height they must
    /// fit, as a table cell gives its row's: a visible child that measured
    /// or is laid out taller is reported as
    /// `ValidationCode::ExceedsFixedHeight`, although this component
    /// compresses or cuts it off. [2.7 rules 1-4.] Schema 16 added this
    /// field.
    pub fixed_height: bool,
}

pub struct ComponentInspection {
    // … other fields unchanged …
    /// The accumulated view scale of the committed geometry this record
    /// reports: the scale `details.box_model.border_box`, the clip and every
    /// item's bounds are in, from the same successful layout; 1 outside any
    /// view transform. Before the component's first layout, and while its
    /// committed geometry is from another window than the one it is now in
    /// (after a detach or redock, until that window's layout succeeds, when
    /// `border_box` and the clip are absent too), the scale it will be laid
    /// out at. [2.3 rules 8-9.] Schema 11 added this field; schema 16 made it
    /// the committed layout's scale.
    pub view_scale: f32,
}

#[non_exhaustive]
pub enum ValidationCode {
    // … existing codes …
    /// A visible component measured or is laid out taller than a parent
    /// whose `ValidationHints::fixed_height` is set, such as a control in a
    /// `Table` row lower than ControlHeight, so the parent compresses or cuts
    /// it off. [2.7 rules 2, 4, 7.] Schema 16 added this code.
    ExceedsFixedHeight,
}

impl InspectionSnapshot {
    /// [Today's text, DG:1040-1046, with: 16; that deserializing a snapshot
    /// or a `QueryResult` rejects any other revision, with the message of
    /// 2.6 rule 2; that a document whose version is not first may fail on
    /// another field first.]
    pub const SCHEMA_VERSION: u32 = 16;
}
// `InspectionSnapshot::schema_version` moves to the first field (2.6 rule 3).
```

In `pert_ui_runtime::testing`, re-exported by `pert_ui::testing`:

```rust
impl<'a> Driver<'a> {
    /// Clicks the middle of the visible part of item `index` of `id`'s
    /// `Semantics::items` with the primary button, then settles. [2.5 rules
    /// 1-5: what an item is (a row the component draws itself, such as a
    /// `Table`'s column title); pointer input with no guaranteed action; the
    /// resize-grip case; no scrolling of the component's own content and the
    /// table's routes; double clicks as for `click`; neutral input. # Errors
    /// in 2.5's order. Example: a two-column table whose `on_sort` records the
    /// request; `click_item(&table, 1)` reports the second column ascending.]
    pub fn click_item(&mut self, id: &WidgetId, index: usize) -> Result<(), TestError>;
}

#[non_exhaustive]
pub enum TestError {
    /// No component is mounted at this key path ([`Driver::find`]).
    #[error("no component is mounted at {}; {}", /* 2.2 rule 3 */)]
    NotFound {
        /// The path that was looked up, one key per segment.
        path: Vec<String>,
        /// The longest leading part of `path` that resolves [2.2 rule 1];
        /// empty when not even the first segment does.
        resolved: Vec<String>,
    },
    /// The component reports no item at this index ([`Driver::click_item`]).
    #[error("`{key}` reports {count} items, none at index {index}")]
    NoItem {
        /// The component's key.
        key: String,
        /// The index asked for.
        index: usize,
        /// How many items the component reports.
        count: usize,
    },
    /// The item has no visible rectangle to click ([`Driver::click_item`]):
    /// its component gives it none, or it is scrolled or clipped out of view.
    #[error("item {index} of `{key}` has no visible bounds")]
    ItemWithoutBounds {
        /// The component's key.
        key: String,
        /// The item's index.
        index: usize,
    },
    // … the other variants, including P4's, unchanged …
}
```

- **Names.** `click_item` is the plan's. `bounds` pairs with `border_box`,
  whose coordinates it shares and its rustdoc states; `local_bounds` names
  the space it takes.
  `ExceedsFixedHeight` and `fixed_height` name the general condition, not the
  table, since an external grid uses the same hint. `NoItem` and
  `ItemWithoutBounds` follow `NoSemantics` and `NotLaidOut`.
- **Representation (P8-D2):** one public window rectangle as `[f32; 4]`,
  as every inspection rectangle is (`border_box`, the clip,
  `ValidationMarker::bounds`); `Rect` has no `serde` (`R/drawing.rs:81-83`).
  The builder takes a `Rect`, which is already validated.
- **`#[must_use]`** on `local_bounds` as on every `SemanticItem` builder; none
  on `click_item`, which returns a `Result`.
- **No `Runtime` method:** the bounds are in every inspection; the driver
  only reads them.

## 4. Documentation

Text describes P8's own commit; nothing names a P9 item.

- **`SemanticItem`** (DG:357-375): rows "have no identity or focus of their
  own; a component may give each its rectangle (`local_bounds`), which an
  inspection reports in window coordinates as `bounds`"; the example gains
  `local_bounds`. **`Semantics::items`** (DG:535-537) likewise.
  **`Detail::Semantics`** (`R/inspection_service.rs:177-178`): "with each
  item's window bounds".
- **`BoxModelInspection::border_box`** (DG:111-115) and
  **`ValidationHints::clip`**: absent while the component's last committed
  geometry is from another window (2.3 rule 9).
- **`Component::inspect`** (`R/runtime.rs:2116-2122`): a paragraph on item
  rectangles: local coordinates from the geometry painting and hit testing
  use, transformed and clipped by the runtime, none after a failed layout
  that arranged the component until one succeeds.
- **`Table`** (TB:1171-1243), a section "Addresses and tests": 2.1's address
  rules with a `find` line in the first example
  (`ui.find(&table, &["n3", "square", "value"])`), the collision, P5's
  `Driver::address_children`, the header items and their bounds,
  `Driver::click_item` and the keyboard route (Alt+Up/Down on the active
  column). **`Table::new`** (TB:1514-1521): 2.7 rule 8 with the metric
  lookup as a doctest. **`TableItems`** (TB:236-240): "the element's own key
  is free" becomes the address of rule 2.1 and the collision; **`cell`**
  (TB:260-271) names the address.
- **`Runtime::find_descendant`** (TR:165-167): the common-cause sentence
  adds "or table content keyed like its column".
- **`Driver::find`** (T:352-358): `NotFound` names the resolved prefix.
- **`testing` module page** (T:1-6): the driver also "clicks an item a
  component draws itself, such as a table's column title".
- **The testing chapter** (`G/testing.md`), a new section "Tables" after
  "List rows" (which P5 rewrites), one doctest: a job table whose rows are
  records with a name and a `ProgressStatus`, the progress cell P7's
  bar-plus-percent cell from `G/layout.md` as P7 leaves it, with
  `ProgressBar::new(job.status)`, the row height from ControlHeight; it
  lists the rows with `address_children`, reads a cell's content by its full
  address and shows `find_descendant`'s ambiguity for content keyed like its
  column, reads `semantics(&table)?.items` with their bounds, sorts with
  `click_item` and asserts the new row order after `on_sort` republishes,
  notes the keyboard route and asserts that `validation_report()` has no
  issue. It links the `Table` page's address section.
- **The diagnostics chapter** (`G/diagnostics.md:189-213`): decoding rejects
  another schema revision.
- **docs/contracts.md:** "Inspection and diagnostics" (`:1448-1474`) gains
  the item geometry contract (2.3 rules 2-5 and 8, including what
  `view_scale` is) and the decoder check;
  "Inspection in custom components" (`:1931-1937`) gains `local_bounds` and
  `ValidationHints::fixed_height` as the extension points the table uses.
- **docs/manual.md** (`:213-227`, the table paragraph): the row height
  holds the tallest content, at least ControlHeight with controls or an
  editable column; content that measured taller is reported as
  `ExceedsFixedHeight`; the cell address.
- **docs/verification.md** native evidence: section 8's row.

## 5. Module membership and dependencies

- **P2 (approved, 07eb0a4).** P2's "Later parts" (`round-2-design-P2.md:756-759`)
  expected `click_item` and the resolved prefix to add no type beyond
  `WidgetId` and `String`, and item bounds to use `Rect`. They add `usize`
  and `Vec<String>` fields and a `Rect` argument, and the field is
  `[f32; 4]`: no pert-ui type outside `testing` and the prelude, so P2's
  assertion 1 passes without an exclusion; `ValidationHints` stays one of
  P2's exclusions (component and inspection), and the new `ValidationCode`
  variant is in a `testing` member. No `testing` or prelude name changes, and
  P2's `every_name` lists are unchanged. P8 runs P2's listing-driven check
  on the regenerated listings and records the result in Progress.
- **Prelude closure (L2 D1):** P8 adds no API to a prelude member: `Table`
  and `TableItems` change only in rustdoc, `SemanticItem` and
  `ValidationHints` are not prelude members, and `Driver` is in `testing`.
- **P4 (approved, 5942c54):** P8's driver tests join
  `crates/pert-ui-runtime/tests/testing.rs`, which P4 and P5 extend; P4's
  `TestError::Draw` is untouched.
- **P5 (approved, 416dba7):** the `Table` page and the chapter's "Tables"
  read rows with `Driver::address_children`, as P5's "Later parts"
  (`round-2-design-P5.md:458-460`) expects.
- **P6 (approved, fec6a5f):** the chapter's cell uses
  `ProgressBar::new(status)`, P6's fixed form (its split table,
  `round-2-design-P6.md:144`); P6's "Later parts" expects P8's chapter to use
  it (`:491-493`).
- **P7 (not approved):** the chapter reuses P7's cell rather than restating
  it, and asserts an empty validation report, which D7 guarantees for a
  growing bar beside its label. If P7's approved design does not give that
  guarantee at the chapter's widths, the chapter widens the progress column
  and the Progress entry records it; nothing else in P8 depends on P7.
- **P9:** fills `local_bounds` for toasts; the shape and schema 16 are
  final here.

## 6. Migration manifest

**Rule:** at P8's base commit (P7's), `git grep` for `NotFound {`,
`schema_version`, `SCHEMA_VERSION`, `view_scale`, `ValidationHints {` and
`Table::new(` in the maintained consumers; each hit is migrated as below or shown
unaffected in the Progress entry.

| Consumer | P8 |
|---|---|
| Runtime: T, DG, `R/inspection_service.rs`, IN (the transform and `view_scale` from the committed scale), TR (`Staged`'s scale, the commit, the flag and the failure-path invalidation, the measured height passed to the validator), `R/runtime.rs` (`Node::arranged_uncommitted`, `committed_scale` and `committed_window`, no measurement flag on the node, the update-time validator record, `Component::inspect`), TB, `R/table/editing.rs`, V, VD (`NodeData`, `Validator::geometry`, the new `Validator::measured`, `same_node`, `geometric`, `check`), TB (`TableCell`'s wrapper field and `reconcile`), TR also (insertion into the set in `measure_node`, the drain after a successful layout, removal on unmount beside `validator.remove`), `R/runtime/native_windows.rs` (the debug-only per-window `measure_changes: RefCell<BTreeSet<u64>>` in `WindowShared`, initialized empty with each window, and its removal of moved serials in `move_presentation`) | section 2 and 4 |
| Readers of `ComponentInspection::view_scale` | none in the repository reads it before a flush (`git grep` at 405501e: the `view_scale` hits in tests are `Painter::view_scale`, `EventContext::view_scale` and `ZoomView`'s property), so no consumer changes; the field's rustdoc changes |
| `crates/pert-ui-runtime/tests/testing.rs:280-284` (`NotFound { path }` literal) | adds `resolved: vec![]` |
| `crates/pert-ui-runtime/tests/diagnostics.rs:55`, `tests/live_sources.rs:434` (`schema_version, 15`) | 16 |
| `crates/pert-ui/tests/foundation/serialization.rs:682-700` (asserts `SCHEMA_VERSION == 15`) | `>= 15`, as the schema-14 test writes it (`:474-476`); the new schema-16 test pins 16 |
| `crates/pert-ui/tests/foundation/component_module.rs:135-142` (`ValidationHints` literal) | unchanged: it ends with `..ValidationHints::default()` |
| The gallery's table (`apps/gallery/src/pages/data.rs:467`, 28 px rows, an editable Note column with a cell editor, `:373-375`, `:479`) | **migrated to the largest preset:** the gallery builds its collections page once (`apps/gallery/src/gallery.rs:339`) and switches density on the mounted page (`:463-470`), while a mounted table keeps its row height (TB:1274-1276); so the row height becomes the largest ControlHeight over `Compact`, `Standard` and `Comfortable`, read from `ui.theme().metrics(..)` when the page is built (40 px with the default metrics), and holds the Note editor at every density the gallery offers. A new gallery test (`apps/gallery/src/tests/data.rs`) mounts the gallery at Standard, switches to Comfortable with `set_density`, opens a Note edit and asserts no `ExceedsFixedHeight`; the existing validation-asserting tests and the interaction check (`apps/gallery/src/interaction_check.rs:860`) stay clean; the page's rows are visibly taller (section 8, native evidence) |
| `inventory`'s table (`examples/inventory/app.rs:627`, 28 px, a controlled `Checkbox` and editable columns, `:75`, `:137-145`) | **migrated:** the row height becomes ControlHeight of the density in effect, read from the theme, as the guidance shows: the example offers no density switch (`git grep` of `examples/inventory`); its headless session shows no `ExceedsFixedHeight`, also during an edit |
| `log_explorer`'s table (`examples/log_explorer/app.rs:797`, 22 px, one-line `Ellipsis` labels, `:76-78`) | unchanged if its session shows no `ExceedsFixedHeight`; otherwise its row height becomes one body line from the theme, recorded in Progress |
| `Table`'s rustdoc examples (TB:1244-1272 labels at 24 px; TB:1348-1400 a `TextField` editor at 24 px) | the editing example takes its row height from ControlHeight, as the guidance it sits beside says; the label example is unchanged |
| Facade and runtime tests with tables (the `Table::new` hits, rows of 20 to 28 px, several with checkboxes or editors) | a test whose validation assertions see the new code raises its row height to ControlHeight (listed in Progress), unless the short row is its subject; others are unchanged, the issue only logged |
| Guide: `G/testing.md` ("Tables"), `G/diagnostics.md`; any table P7 adds to `G/layout.md` (its row holds its content, checked by the rule above) | section 4 |
| docs/contracts.md, docs/manual.md, docs/verification.md | section 4 and 8 |
| Packaged examples' sessions, the native checks, the READMEs, the diagnostics fixtures (`tests/diagnostics/ui/`) | unchanged: none names `NotFound`, `SemanticItem`, `ValidationHints` or a schema version (`git grep`); no `#[diagnostic::on_unimplemented]` changes |
| `plans/usability/task4-app` | unchanged: it uses none of these items (`git grep`), so no fixture check |
| Frozen: the submissions and probes under `plans/usability/runs/` and U, such as U `r2-5-r1-claude-1/work/src/headless.rs:131-143` and U `r2-5-head-claude-2/work/tests/headless.rs:229-243` | not migrated |

## 7. Tests

Driver tests are in `crates/pert-ui-runtime/tests/testing.rs`; table tests
in the facade's `collections` binary (`tests/collections/table.rs`); the
geometry contract for custom components in `foundation`
(`tests/foundation/semantics.rs`); the generic hint in
`crates/pert-ui-runtime/tests/validation.rs`; serde in
`tests/foundation/serialization.rs` (`--features serde`). Validation tests
are debug-only, as the existing ones. A table helper mounts it inside a
padded `Container`, so its origin is not the window's.

- **New-behavior tests** name the assertion that detects a wrong
  implementation and a compiling mutation that must make it fail; the
  Progress entry records each mutation's result. A test that fails only
  because a new item does not compile is not evidence.
- **Contract and compatibility tests** check rules P8 keeps or states.

### New-behavior tests

1. **Resolved prefix.** `find(&root, &["r1", "missing"])` is
   `NotFound { path: [r1, missing], resolved: [r1] }` with 2.2's message;
   `["missing", "child"]` gives `resolved: []`; below two structural
   wrappers each holding an `a`, `["a", "x"]` gives `resolved: [a]`.
   Mutations: `resolved` = the path without its last segment (the second
   case fails); `Ok(Some)` only counts (the third fails).
2. **Header bounds and a sort.** A table of two sortable columns, fixed
   80 px and flex. Named assertion: item `i`'s bounds are
   `[table.x + start_i, table.y, width_i, ControlHeight]`, and
   `click_item(&table, 1)` makes `on_sort` report column 1 ascending; after
   `advance` past the click interval, a second call reports descending.
   Mutations: report the local rectangle unmoved (window = local; the
   container's offset makes the click miss); use `click` on the table (the
   body takes it, no sort).
3. **Partly clipped after horizontal scrolling.** Columns wider than the
   table, scrolled by a horizontal wheel so column 1 is cut on the left.
   Named assertion: item 1's bounds are its visible part (left edge at the
   table's left), and `click_item(&table, 1)` sorts column 1. Mutation: do
   not intersect with the clip (the centre falls outside the table).
4. **View scale.** The table inside a `ZoomView` at 200%. Named assertion:
   each item's width is twice its column width and `click_item` sorts.
   Mutation: ignore the scale.
5. **Fully out of view.** Column 0 scrolled entirely out. Named assertion:
   item 0's bounds are `None`, `click_item(&table, 0)` is
   `ItemWithoutBounds { key, index: 0 }` and no sort ran. Mutation: report
   an empty intersection as `Some`.
6. **Index out of range, nothing changed.** The table inside a scrolled
   `ScrollArea` with the table out of view. Named assertion:
   `click_item(&table, 9)` is `NoItem { key, index: 9, count: 2 }` and the
   scroll area's offset is unchanged. Mutation: check the index only after
   the reveal (the offset changed).
7. **A reveal that scrolls.** Same setup, `click_item(&table, 0)`. Named
   assertion: the sort runs. Mutation: click at the bounds read before the
   reveal.
8. **Before the first layout.** Named assertion: a mounted, unflushed
   table's items and a custom component's items with `local_bounds` have
   `bounds == None`. Mutation: treat missing node bounds as the origin.
9. **Pending layout.** `TableController::set_column_width` without a flush.
   Named assertion: the items' bounds equal those before the call; after a
   flush they show the new width. Mutation: the table computes its rects
   from `columns`' widths instead of `resolved` (the first assertion fails).
10. **Failed layout.** The table beside a later sibling whose `arrange`
    fails while a signal says so; the viewport narrows, so the failed pass
    arranges the table at a new width. Named assertions: after the failed
    flush, the table's items have no bounds while its `border_box` is the
    old one; a component after the sibling, which the pass never reached,
    keeps its items' bounds; after the failure clears and the viewport
    returns to its first width, one flush gives the table's items the bounds
    they had before and `column_widths` equals the first widths. Mutations:
    no guard (the first assertion fails: the failed pass's widths at old
    bounds); no failure-path invalidation (the last assertion fails: the
    table is skipped and keeps the failed pass's columns). The setup must
    make both mutations fail; Progress records it.
11. **Hidden.** `ui.set_visible(&table, false)` and a flush. Named
    assertion: items have no bounds while `Runtime::bounds(&table)` is still
    `Some`. Mutation: drop the `is_presented` check.
12. **Promoted surface and detached window, settled.** A table in a
    `Popover` whose owner is inside a `ZoomView` at 200%, and a table in a
    `View` inside that `ZoomView`, detached and flushed. Named assertions:
    in both, item widths equal
    the column widths (scale 1) and the bounds lie within the table's
    `border_box`; the detached table's `ComponentInspection::window` is the
    detached window, `click_item` refuses with `OtherWindow` until that
    window is selected, then sorts. Mutation: take the scale from the
    logical parent's content scale (widths double).
13. **Restore resolves at once.** Restore a workspace entry that swaps two
    columns of different widths, inspect before a flush. Named assertion:
    item 0's name is the restored first column and its width that column's
    width. Mutation: no `resolve` in the restore (item 0 gets the old first
    column's width).
14. **A rejected restore keeps the presentation.** Columns overflowing,
    scrolled to a nonzero offset, and a `set_column_width` not yet laid
    out; then a restore whose entry reorders the columns and narrows them so
    the scroll range would shrink, with its republication made to fail.
    Named assertion: the restore is rejected with `RestoreRejection::Runtime`
    and, before and after the next flush, `scroll_x`, `column_widths` and the
    items' bounds equal those of a twin table given the same width change
    and no restore. Mutation: resolve before the republication, or again
    after the rollback (the offset is clamped and lost). The failure is
    injected in `R/table.rs`'s unit tests, which can hold the controller's
    private items signal borrowed across the call so that `republish`'s
    write fails with `BorrowConflict`; if the signal's borrow rules do not
    give that, a `#[cfg(test)]` failure switch on `republish` does, and the
    Progress entry says which.
15. **Reparented across scales, before a flush.** A table moved with
    `Runtime::reparent` from a plain `Column` into a `ZoomView` at 200%,
    inspected before any flush. Named assertions: `view_scale` is 1, each
    item's width equals its column width and lies within the unchanged
    `border_box`; after a flush, `view_scale` is 2 and the widths double.
    Mutation: the transform and `view_scale` read `node.scale` (IN:200), so
    the widths double against the old bounds.
16. **A scale-changing flush that fails before the target.** A `ZoomView`
    whose content is a failing component followed by the table; zooming to
    200% and flushing fails in the first child's `arrange`, so the table is
    never arranged and its flag stays clear. Named assertions: the table's
    `view_scale` is 1 and its items' bounds are those before the zoom, at
    the unchanged `border_box`; after the failure clears, a flush gives
    scale 2. Mutation: as test 15.
16a. **Detached and redocked, before a flush.** A laid-out table in a
    `View`, then `commit_detach` and an inspection before any flush. Named
    assertions: `ComponentInspection::window` is the detached window, the
    table's items have no bounds, `border_box` and the clip are absent and
    `view_scale` is the scale it will be laid out at; after a flush, all are
    present in the detached window's coordinates. The same after `redock`,
    with the source and destination swapped. Mutation: drop the
    committed-window comparison (source-window rectangles are reported as
    the destination's).
16b. **The destination's layout fails before the table.** As 16a, with a
    failing component placed before the table in the `View`, failing in
    the detached window's first layout. Named assertion: after the failed
    flush the table still has no item bounds, `border_box` or clip (its
    committed window is the source); after the failure clears, a flush gives
    them in the detached window. Mutation: as 16a.
16c. **A zero-size component in a continuing zoom.** A `ZoomView` holding a
    zero-size component (an empty `Label`), zoomed in steps closer together
    than `VIEW_SCALE_SETTLE`, a successful flush after each. Named
    assertion: after each flush the component's `view_scale` equals the
    zoom. Mutation: remove the `committed_scale` comparison from the early
    return (the scale stays at the first step's).
17. **Row height, laid out taller.** A 32 px `Button` (Standard density) in
    a 28 px row. Named assertion: one `ExceedsFixedHeight` per mounted row
    with components `[button, cell]` and 2.7's message; a 32 px row reports
    none. Mutations: no hint on `TableCell`; compare the clipped visible
    rectangle instead of the border box (none reported).
18. **Row height, measured taller, one issue per cell.** In 28 px rows: a
    `TextField`, a `Checkbox`, and an editable column whose cell is then
    edited with a `TextField` editor (`EditHost`); a wrapping `Label` in a
    column too narrow for its text on one line. Named assertions: the
    report's `ExceedsFixedHeight` issues are exactly one per affected
    mounted cell, with components `[field, cell]`, `[checkbox, cell]`,
    `[label, cell]` and, while the edit is open, `[editor, EditHost]` and no
    `[EditHost, cell]`; when the edit ends, the editable cell's issue set is
    what it was before; none is reported for the single-line controls and
    the editor in 32 px rows, for the wrapping label in a row that holds its
    two lines (at Body 14 px and line height 1.4 × size,
    `R/runtime/theme.rs:225`, `R/text.rs:1171`, about 39.2 px: a 40 px row),
    or for the same label with `TextOverflow::Ellipsis` in a 28 px row.
    Mutations: check the border box
    only, without the recorded measurement (none reported); set the hint on
    `TableCell` unconditionally (the extra `[EditHost, cell]` issue);
    `TableCell` without its `reconcile` (the cell keeps reporting the hint
    after the edit begins, giving the extra issue).
19. **Through the content wrapper, one issue.** Content keyed `cell editor`
    (wrapped by `CellContent`) holding a `Button`. Named assertion: exactly
    one issue for the cell, `[button, CellContent]`, and none naming the
    cell. Mutations: the hint on `TableCell` only (the issue names the
    wrapper as the content); on both (two issues).
20. **A custom fixed-height parent.** A custom component whose `inspect`
    sets `fixed_height` and which places a `Button` at its own 20 px height,
    clipped. Named assertion: reported; with the hint false, not. Mutation:
    the check reads a table-private marker instead of the hint.
21. **The hint alone changes.** The custom parent of test 20, its hint a
    signal its `update` reads, switched false, true, false with nothing else
    changing, a flush after each. Named assertion: the issue is absent,
    present, absent. Mutation: leave `fixed_height` out of `same_node`
    (the report never changes).
21a. **The measurement alone changes.** A custom parent with
    `fixed_height`, 20 px tall, whose `arrange` measures its `Label` child
    at a width read from a signal (narrow: two lines; wide: one) and always
    places it at the same 20 px rectangle; the child stays clean and its
    rectangle unchanged. Named assertions: switching narrow, wide, narrow,
    a flush after each, makes the issue present, absent, present. Mutation:
    record the measurement only with `Validator::geometry` (the issue never
    appears, since the child is never staged).
21b. **Across a move.** The parent of 21a inside a `View`: a measurement
    change whose primary layout fails (a failing sibling outside the view),
    then `commit_detach` and a detached-window layout that fails too, then a
    successful primary layout; then `redock`, a successful primary layout,
    and measurement-only changes narrow, wide, narrow. Named assertions: the
    successful primary layout after the detach leaves the issue state as it
    was (the primary drain no longer holds the moved node, so it commits no
    measurement the detached window's failed layout produced); after the
    redock the issue follows each measurement-only change: present, absent,
    present. Mutation: add a per-node queued marker, as revision 3 had,
    without resetting it on a move (after the redock the changes are never
    recorded). The drain's "still in that window" filter already keeps the
    source from committing a moved node, so the source-set removal is
    tested by 21d instead.
21d. **Move cleanup (unit).** In TR's test module, where the per-window
    sets are visible: a measurement change whose layout fails, so the
    child's serial is in the primary window's set; then `commit_detach`.
    Named assertion, immediately after the move and before any flush: the
    primary window's set does not contain the serial. Then a second
    measurement change whose layout in the detached window fails, and an
    assertion that the detached window's set now holds the serial; the
    test keeps the detached `NativeWindowId` (its handle retains the
    window's shared state, `R/runtime/native_windows.rs:125-131`), redocks,
    and right after `redock` returns (which moves and retires at once,
    `:705-709`) asserts through that handle that the set no longer holds
    the serial. Mutation: omit the removal in `move_presentation`.
21e. **Unmount cleanup (unit).** In TR's test module: a measurement change
    whose layout fails, so the serial is queued; then the child is
    unmounted. Named assertion, immediately after the unmount and before
    any successful layout: its window's set does not contain the serial.
    Mutation: omit the removal beside `validator.remove`.
21f. **Idle flushes (contract).** A unit test in TR's test module beside the
    existing pass-count tests (TR:2114-2124), since `Validator::passes` is
    crate-private: after 21a's last change, three flushes with nothing
    changed run no validation pass.
22. **Both envelopes, `--features serde`.** A custom component reporting
    two items, one with `local_bounds` and one without, in a snapshot and in
    a `QueryResult` at `Detail::Semantics`. Named assertions: both round-trip
    with `bounds` `Some` and `None`; `SCHEMA_VERSION` and the captured
    version are 16; an empty snapshot (`Runtime::new().inspect()`) and an
    empty `QueryResult` (no selector) decode at 16; the same documents with
    15 and with 17 fail with 2.6's message naming the found version; an
    unknown top-level field fails; an item whose `"bounds":null` is removed
    decodes with `bounds == None`. Mutations: drop `deserialize_with` (15
    decodes); drop `serde(default)` on `bounds` (the last fails).
23. **The new hint and code persist, and the hint is required.** A
    snapshot at `Detail::Full` of test 17's table, which holds a
    `ValidationHints` with `fixed_height: true` and a report with an
    `ExceedsFixedHeight` issue and marker. Named assertions: it round-trips
    with both; the same document with `"fixed_height"` removed from one
    component's hints fails to decode. Mutation: `serde(default)` on
    `fixed_height` (the document decodes).

### Contract and compatibility tests

- **Cell addresses (2.1):** content keyed `state` in column `state` is at
  `[row, "state", "state"]`, `find(&table, &[row, "state"])` is the cell
  (role `Cell`), `find_descendant(&row, "state")` is
  `Err(AmbiguousKey("state"))`; an edited cell's editor is at
  `[row, column, editor key]`; the `Table` page's and `TableItems`' doctests.
- **The resize grip (2.5 rule 4):** a column scrolled until only 4 px of it
  remain (Standard grip 8 px): `click_item` on it runs no sort, and the
  column becomes user-sized at its width, the documented resize. The
  sorting tests choose geometry away from the grips.
- **`click_item`'s refusals:** `OtherWindow` and `InputHeld` before anything
  changes; `NoSemantics` for a component without semantics; `NoItem` with
  `count: 0` for a `Label`; `ItemWithoutBounds` for a `Tabs` title (2.4
  rule 3).
- **Replacement (2.3 rule 6):** a custom component that assigns
  `item.bounds = Some(..)` without `local_bounds` reports `None`; a local
  rectangle partly outside the component is cut to it.
- **Unchanged:** every existing table, inspection and validation test; the
  table's other semantics; `Runtime::bounds`; `Workspace::SCHEMA_VERSION`
  (2); release builds report no validation (existing pattern).
- **Field order (2.6 rule 3):** `serde_json::to_string` of a snapshot and
  of a `QueryResult`, serialized directly, starts with
  `{"schema_version":16`; no intermediate `serde_json::Value` is used,
  since without `preserve_order` (not enabled, `Cargo.toml:21`) its maps
  sort their keys, and the feature is not added for this test. Separately,
  decoding does not depend on the order: a revision-16 document with
  `schema_version` moved to the end decodes, and the same with 15 is
  rejected.
- **Module membership:** P2's listing-driven check on the regenerated
  listings; the prelude tests unchanged.
- **Doctests:** `SemanticItem`, `SemanticItem::local_bounds`,
  `Driver::click_item`, `Driver::find`, `Table` (addresses), `Table::new`
  (metric lookup), `TableItems`, the testing chapter's "Tables", the
  diagnostics chapter's serialization block.

## 8. Scope, version and evidence

- **`api/` diff:** added `Driver::click_item`, `TestError::NoItem` and
  `ItemWithoutBounds` with their fields, `SemanticItem::bounds`,
  `SemanticItem::local_bounds`, `ValidationHints::fixed_height`,
  `ValidationCode::ExceedsFixedHeight`; changed `TestError::NotFound`
  (`resolved`), `InspectionSnapshot`'s field order and, where the listing
  shows it, `SCHEMA_VERSION`'s value; in `api/pert-ui-runtime/*.txt` and every
  `api/pert-ui/*.txt`.
- **Collision review:** `click_item`, `local_bounds`, `fixed_height`,
  `ExceedsFixedHeight`, `NoItem` and `ItemWithoutBounds` occur nowhere in the
  repository's code (`git grep` at 405501e); none is a type, so no glob
  changes.
- **CHANGELOG** for 0.3.83, citing this checkpoint:
  - `Added (R2-27):` `SemanticItem::local_bounds` and `SemanticItem::bounds`:
    a component gives an item a rectangle in its own coordinates and an
    inspection reports its visible part in window coordinates, following the
    view scale, clips, promoted surfaces and detached windows; the table's
    header titles report theirs. `Driver::click_item`, with
    `TestError::NoItem` and `TestError::ItemWithoutBounds`.
  - `Added (R2-23):` `ValidationHints::fixed_height` and
    `ValidationCode::ExceedsFixedHeight`: development validation reports a
    table cell's content that measured or is laid out taller than its row,
    such as a checkbox, a text field or an editor in a row lower than
    ControlHeight.
  - `Breaking (R2-22):` `TestError::NotFound` gains `resolved`, the longest
    prefix of the path that resolves. Replacement usage: match
    `TestError::NotFound { path, .. }`; a test that builds the value adds
    `resolved`.
  - `Breaking (R2-27, persistence):` inspection schema 16; deserializing an
    `InspectionSnapshot` or `QueryResult` of another revision fails with
    "inspection schema version N is not supported (this build writes 16)",
    `schema_version` is a snapshot's first field, and
    `ComponentInspection::view_scale` is the scale of the committed layout
    the record's geometry is from. Replacement usage:
    capture snapshots again with this runtime; a tool that reads several
    revisions compares `schema_version` before decoding.
  - `Breaking:` `ValidationHints` has a new public field. Replacement usage:
    build it from `..ValidationHints::default()`, as its page says.
  - `Changed (same kind):` after a failed layout, the next layout arranges
    again every component the failed one arranged; a restored workspace
    resolves the table's columns once it succeeds; after a detach or redock,
    inspection reports no `border_box` or clip for a moved component until
    its new window's layout commits it.
  - `Changed (R2-23):` `inventory`'s table takes its row height from
    ControlHeight, which its checkboxes and editors measure, and the
    gallery's from the largest ControlHeight of the densities it offers.
  - `Documentation (R2-22, R2-23):` the cell address on `Table`,
    `TableItems` and `TableItems::cell`, and on `find_descendant`; the row
    height on `Table::new`; the testing chapter's "Tables"; the contracts
    and the manual.
- **TODO.md:** section "Findings outside this part"'s entries, after a
  search. The 2026-10-28 entry on examples with unused imports names the
  `testing` module's example, whose page P8 edits; P8 does not resolve it.
- **Version:** 0.3.83 after P7's 0.3.82: the workspace version, the internal
  version requirements and `Cargo.lock`. If the order changes, the heading
  follows the actual version.
- **Gate:** the public-API row with persistence, as the plan states: the
  workspace checks (formatting, Clippy, tests including P2's listing-driven
  check and section 7, the five packaged examples' headless sessions,
  doctests, warning-free rustdoc); the headless gallery, since the new code
  could fire in its tables; the API listing gate with its diff reviewed
  against the CHANGELOG; Clippy, tests and rustdoc with
  `-p pert-ui --features serde`, and Clippy and tests with
  `-p pert-ui --no-default-features --features serde`; `git diff --check`;
  the diagnostics snapshots (`cargo test --locked -p pert-ui-diagnostics --
  --ignored`), which the workspace gate for a public-API and persistence
  change includes (`AGENTS.md:261`, `:277`), though no shaped message
  changes; the mutations of section 7 in Progress; Codex's implementation
  review. Not required: `-p pert-ui-core --features serde` (no core type
  changes; `Rect` is a runtime type, `R/drawing.rs:81-83`), MSRV, packaging
  and the fixture check.
- **Native evidence:** a Linux hand check on an unlocked desktop with a
  hardware GPU: in the native gallery's data page, click a sortable title
  (sorts, a second click flips), narrow the window until the columns
  overflow, scroll horizontally and click the visible part of a partly cut
  title (sorts unless the press is on its edge's grip); and look at the
  data page's rows, now the largest preset's ControlHeight (40 px), at
  Compact, Standard and Comfortable, switched on the mounted gallery,
  editing a Note cell at each, whose editor is not compressed.
  Recorded with the platform in Progress and `docs/verification.md`;
  otherwise recorded as missing there. Windows and macOS stay missing
  (NATIVE-01 to NATIVE-05).
- **Real time, backends:** none involved.

## Corrections to the plan

- **A failed layout withholds bounds.** The plan's test expects a failed
  layout to keep the items' bounds of the last successful one. A component
  the failed pass arranged has already replaced its retained geometry
  (TB:2480), and the runtime holds no copy of its earlier rectangles, so
  keeping them would need a second report at arrange (P8-D1, alternative
  A). Its items have no bounds until a layout succeeds; components the pass
  did not arrange keep theirs; the next layout re-arranges the former,
  which also fixes their painting and hit testing (2.3 rule 5).
- **The measurement comes from the runtime, not from the cell's hints.**
  The plan's "measured taller" is kept (2.7). The cell's own measurement
  cannot reach the validator through the hints it reports, which are read
  after its update and before layout (2.7 rule 3); the validator records the
  measurement `measure_node` returned to the cell, with the public hint as
  the extension point.
- **The view scale was not part of the committed geometry.** Codex's
  review of revision 1 showed `view_scale` updated before layout
  (TR:765, :816, IN:200); the transform and the field now use the scale
  committed with the bounds (2.3 rule 8, P8-D4), a reinterpretation of the
  field inside schema 16.
- **A schema field beyond items.** The report needs a public hint so an
  external grid can use it (AGENTS.md, "Avoid privileged paths"), which
  adds `ValidationHints::fixed_height` to schema 16 and is a `Breaking:`
  entry for struct literals.
- **Consumers with short rows.** Measuring keeps the requirement and
  reaches controls the plan's tests did not name: the gallery's and
  `inventory`'s 28 px rows hold a checkbox or editors that measure 32 px, so
  both are migrated (section 6).
- **The decoder check is per field.** `deserialize_with` on
  `schema_version` keeps `deny_unknown_fields` on the types and needs no
  mirror struct that each later field must repeat; the plan's
  `try_from` was an example. The typed error is crate-private: serde
  reports its own error type with the message, as the workspace document's
  `Deserialize` does (`R/workspace.rs:1729-1736`), so a public type would be
  one no decoder returns.
- **An ambiguous prefix counts as resolved** (2.2 rule 1): `find` refuses
  only at the last segment (TR:322-329).
- **`click_item` reads the items before the reveal too**, so a bad index
  changes nothing, as every other refusal (2.5 rule 1); the plan named only
  the re-read.
- **A target in another window** is the existing `TestError::OtherWindow`,
  not a new variant.
- **Line numbers.** The plan's `R/diagnostics.rs:1004` is DG:1046,
  `:359-384` is DG:357-402, `R/table.rs:2638-2646` is TB:2643-2659 and
  `R/table.rs:3030-3060` is TB:3042-3062 at 405501e.

## Findings outside this part

- `R/virtual_list.rs` rows (and so `Tree` rows) — the same kind as R2-23:
  each row is placed with `place_clipped(child, slot, slot)`
  (`R/virtual_list.rs:2132`), so a row whose minimum height exceeds the
  list's row height is cut off with no report. The hint does not fit there
  (the slot, not the list, is fixed); a fix gives the list's rows the
  check, for example by comparing each row with its slot. TODO.md entry; not
  fixed.
- Built-in item reporters without rectangles — R2-27's improvement for
  other controls: `Tabs` titles (`R/tabs.rs:1842`), `Accordion` and
  `Collapsible` headers (`R/collapsible.rs:357`, `:1466`), choice controls
  (`R/choice_controls.rs:1437`), `Select` and its list (`R/select.rs:805`,
  `:1321`), `MultiSelect` (`R/multi_select.rs:1109`), `QuickPick`
  (`R/quick_pick.rs:1491`) and the tool-window stripe
  (`R/tool_windows/stripe.rs:1073`) report items that `click_item` cannot
  click. Each that draws its rows could give `local_bounds`. TODO.md entry;
  not fixed.

## Not in this part

- **Scrolling a component's content to an item** from the driver: no run
  needed it, and the table's keyboard and wheel routes exist (2.5 rule 5).
- **A theme-following default row height** (D8's alternative); the
  guidance and the report cover a row too low for its content.
- **Content search over items:** D6 and P9's TODO entry.
- **Item bounds for toasts:** P9, on this shape.

## Decisions

- **P8-D1: tying item rectangles to the last successful layout.**
  - **Recommended: the per-node guard with re-arrangement after a failure**
    (2.3 rule 5). Items keep D8's builder in `inspect`, so rectangles come
    from the state painting and hit testing use, including state changed
    outside layout such as the table's scroll offset; a pending layout pairs
    the last layout's retained geometry with its bounds; a failed pass is
    never paired, and the next layout repairs it.
  - **Alternative A: record rectangles at arrange** through an
    `ArrangeContext` method, staged and committed with the node. Exactly the
    last successful layout's, but paint-only geometry goes stale: the
    table's horizontal scroll invalidates only paint (TB:1861-1867) and
    would have to re-arrange, and it is a second reporting API beside D8's
    builder.
  - **Alternative B: withhold every item's bounds in a window whose last
    layout failed.** Simpler, but a successful pass that skips a component
    the failed one arranged (TR:1717-1757) pairs its stale state with the
    committed bounds again once the flag clears.
- **P8-D2: where the rectangles live in `SemanticItem`.**
  - **Recommended: one public window field, `bounds: Option<[f32; 4]>`,**
    with the local rectangle private and set by `local_bounds`. The
    snapshot carries one meaning per field, as `border_box`; a component
    cannot report window geometry it does not know.
  - **Alternative A: two public fields,** `local` and `bounds`: the first is
    meaningless in a snapshot and the second in a component's report.
  - **Alternative B: `Option<Rect>`:** `Rect` has no serde, and every other
    inspection rectangle is an array.
- **P8-D3: how the row-height report is extended.**
  - **Recommended: a public hint and a code, checked against the
    measurement the parent obtained** (2.7). Built-ins use only the public
    hint, so an external grid gets the same report (AGENTS.md, no privileged
    paths); one schema-16 field; the measurement is the runtime's result of
    `MeasureContext::measure`, so the plan's "measured taller" holds,
    including controls that measure ControlHeight without a hard minimum.
    It migrates the two maintained tables with 28 px rows (section 6).
  - **Alternative A: a check the validator applies to table cells only:** no
    schema field, but a privileged path.
  - **Alternative B: report every child laid out beyond a clipping parent:**
    no hint, but `ScrollArea`, `VirtualList` and every clipping container
    clip on purpose, so the report would be noise.
  - **Alternative C: the laid-out bounds only** (revision 1). No measurement
    recorded and no consumer migrated, but a `TextField`, `Checkbox`,
    `Toggle`, an in-place editor or a wrapped `Label` in a short row would
    go unreported, which narrows the plan's requirement.
- **P8-D4: what `ComponentInspection::view_scale` means.**
  - **Recommended: the scale of the committed geometry** (2.3 rules 8-9),
    staged and committed with the bounds and the window; before the first
    layout, and while the committed geometry is from another window, the
    scale the component will be laid out at. The record then describes one
    state, and item bounds are computed from it. Nothing in the repository
    reads the field before a flush (section 6).
  - **Alternative: keep `view_scale` current and add a separate committed
    scale** used only by the transform. No reinterpretation, but one record
    would report a scale its own `border_box` and items are not in during a
    pending or failed layout.

## Review record

- **Revision 1 (2026-10-28):** written from the code at 405501e (library
  code as at 84bb60f, 0.3.76); P6's design (fec6a5f) and P2's implementation
  (2b38625) were committed meanwhile, and the dependency paragraph and
  section 5 cite them.
- **Revision 1, Codex review (2026-10-28):** needs revision, two blocking
  points, four should-fix and two nits; record
  `~/pert-ui-usability/codex-prompts/codex-design-P8.out` (read against
  2b38625). Each point was checked against the code before revision 2
  applied it; none was rejected.
  - **Blocking, the scale was not committed:** confirmed: `reparent` calls
    `apply_view_scale` before layout (TR:765), which replaces `node.scale`
    (TR:816), as `sync_view_scale` does after an update (TR:890-904), and
    inspection reads it (IN:200), so old bounds met a new scale with the
    guard clear. Revision 2 stages and commits the scale with the bounds and
    uses it for the transform and for `view_scale`, now defined as the
    committed layout's scale (2.3 rule 8, P8-D4, schema 16), with tests 15
    and 16 (a reparent across scales before a flush; a scale-changing flush
    failing before the target).
  - **Blocking, "measured taller" narrowed:** confirmed: a `TextField`
    (`R/text_fields.rs:2502-2504`) and a `Checkbox`
    (`R/choice_controls.rs:331-332`) measure ControlHeight without a hard
    minimum, and the cell places them lower (TB:977). Following the author's
    guidance, revision 2 keeps the requirement: the validator records the
    measurement the cell obtained (`measure_node`'s cached result) with each
    commit, the public hint selects the parents, and a component-reported
    field is shown not to work, since hints are read after update and before
    layout (2.7 rules 2-3). Test 18 covers a text field, a checkbox, an
    editor and a wrapped label; the gallery's and `inventory`'s 28 px tables
    are migrated (section 6); the narrower scope is alternative C of P8-D3.
  - **Should-fix, restore rollback:** confirmed: `resolve` clamps the offset
    (TB:398-399), so resolving before the fallible republication, or again
    after the rollback (TB:2255-2259), could not restore it. Revision 2
    resolves only once the restore has succeeded (2.4 rule 2), with test 14
    (nonzero offset, a pending width change, injected republication
    failure).
  - **Should-fix, a hint change alone:** confirmed: `same_node` lists each
    hint (VD:428-444). Revision 2 adds `fixed_height` and the measured
    height to it and to `Validator::geometry`'s early return (2.7 rule 5),
    with test 21 (false, true, false) and its mutation.
  - **Should-fix, serialization coverage:** revision 2 adds test 23 (a
    persisted true hint and an `ExceedsFixedHeight` issue and marker, and
    rejection without `fixed_height`, mutation `serde(default)`), and moves
    the field-order check to the contract tests.
  - **Should-fix, the diagnostics snapshots:** confirmed (`AGENTS.md:261`,
    `:277`); the gate includes them.
  - **Nit, density name:** `Comfortable` (`R/runtime/theme.rs:28-36`).
  - **Nit, field order:** the promise is limited to direct serialization
    with an order-preserving serializer such as `serde_json`, and decoding
    is shown not to depend on order (2.6 rule 3, contract tests).
- **Revision 2, Codex recheck (2026-10-28):** needs revision, two blocking
  points and three should-fix; record
  `~/pert-ui-usability/codex-prompts/codex-design-P8-r2.out` (read against
  0eb8baf). Revision 1's points 3-7 resolved. Each new point was checked
  against the code at 0eb8baf before revision 3 applied it; none was
  rejected.
  - **Blocking, the window was not committed:** confirmed:
    `move_presentation` replaces a node's window at once
    (`R/runtime/native_windows.rs:657`) and only invalidates its layout
    (`:664`), for a detach (`:630`) and a redock (`:705`), while inspection
    reports the new window (IN:198). Revision 3 commits the window with the
    geometry and, while they differ, reports no item bounds, `border_box` or
    clip and gives `view_scale` as before a first layout (2.3 rule 9, the
    `view_scale` rustdoc, P8-D4), with tests 16a (detach and redock before
    a flush) and 16b (the destination's layout failing before the table).
  - **Blocking, a measurement change could bypass the record:** confirmed:
    `measure_node` updates the cache (TR:1689) while `arrange_node` returns
    early for a clean child in an unchanged rectangle (TR:1717-1723), so
    neither the commit's `Validator::geometry` call (TR:1983) nor an update
    records it. Revision 3 tracks changed measurements in a debug-only,
    per-window list of nodes, each at most once, committed to the validator
    after a successful layout only (2.7 rule 2), with test 21a (unchanged
    allocation, a measurement changing both ways; idle flushes run no
    validation pass).
  - **Should-fix, a changed scale with unchanged bounds:** confirmed for a
    zero-size component in a continuing zoom (TR:831-855, :1717-1723).
    Revision 3 adds `committed_scale == Some(scale)` to the early return,
    which leads only to the leaf shortcut or the retained-placement path,
    so no extra `arrange` runs (2.3 rule 8), with test 16c.
  - **Should-fix, duplicate wrapper reports:** confirmed: `CellContent` and
    `EditHost` forward measurement and placement (TB:925-930,
    `R/table/editing.rs:712-723`), and the two issues have different
    component lists, so deduplication keeps both (VD:295-297). Revision 3
    puts the hint on the innermost table component around the content
    only: `TableCell` omits it when its child is a wrapper, and a
    `reconcile` keeps that field current across an edit, since the default
    ignores the new description (`R/runtime.rs:1105-1110`) (2.7 rule 6).
    Tests 18 and 19 assert the exact issue sets.
  - **Should-fix, the field-order test:** confirmed: without
    `preserve_order` (`Cargo.toml:21`), a `serde_json::Value` sorts its
    keys. Revision 3 checks the first key of `serde_json::to_string` of each
    envelope and keeps order-independent decoding as its own contract test.
- **Revision 3, Codex second recheck (2026-10-28):** needs revision, one
  blocking point and two should-fix; record
  `~/pert-ui-usability/codex-prompts/codex-design-P8-r3.out` (read against
  0eb8baf). Each was checked against the code before revision 4 applied it;
  none was rejected.
  - **Blocking, the measurement queue across a move:** confirmed: a redock
    moves the subtree and retires the source window
    (`R/runtime/native_windows.rs:705-709`, `:730-745`), so revision 3's
    per-node queued flag could stay set in a list that is never drained,
    and later measurement-only changes would never be queued again.
    Revision 4 drops the per-node flag: each window holds a set of serials,
    a move removes the moved serials from the source's set, the
    destination's next measurement inserts them again (a move empties the
    measurement cache), retirement drops an empty set and unmount removes
    the serial (2.7 rule 2). Test 21b runs failed layout, detach, failed
    detached layout, successful primary layout, redock, successful layout
    and measurement-only changes; 21c keeps the idle-flush contract.
  - **Should-fix, the gallery's density switch:** confirmed: the gallery
    builds the collections page once (`apps/gallery/src/gallery.rs:339`),
    switches density on it (`:463-470`), and a mounted table keeps its row
    height (TB:1274-1276), so a Standard-sized row compresses the
    Comfortable editor (40 px, `R/runtime/theme.rs:205`). Revision 4 gives
    the gallery's table the largest preset's ControlHeight, adds a gallery
    test that switches the mounted gallery from Standard to Comfortable and
    edits Note, adds the density case to the `Table::new` guidance, and
    updates the manifest, CHANGELOG and native expectations; `inventory`
    has no density switch and keeps the current density's ControlHeight.
  - **Should-fix, the wrapping label's fitting row:** confirmed: two Body
    lines are about 39.2 px (`R/runtime/theme.rs:225`, `R/text.rs:1171`),
    which a 32 px row cannot hold. Test 18 keeps 32 px fitting rows for the
    single-line controls and the editor and gives the wrapping label a
    40 px row; the `Ellipsis` assertion stays separate.
- **Revision 4, Codex third recheck (2026-10-28):** the behavior rules are
  accepted; one should-fix and a nit; record
  `~/pert-ui-usability/codex-prompts/codex-design-P8-r4.out` (read against
  9e0e301). Both confirmed.
  - **Should-fix, the cleanup tests could not detect their omissions:** the
    drain filters to nodes still in its window, so a stale source entry
    commits nothing, and 21c checked unmount removal after a drain.
    Revision 5 adds new-behavior unit tests 21d (the source set excludes
    the moved serial right after a detach and a redock, before any flush)
    and 21e (a serial queued through a failed layout is gone right after an
    unmount, before a successful layout), each with the omitted cleanup as
    its mutation; 21b keeps the per-node-marker mutation, and the idle-flush
    contract is 21f.
  - **Nit, the manifest:** it names the per-window `BTreeSet` in
    `WindowShared` and `R/runtime/native_windows.rs` (initialization and
    move cleanup), and no node flag.
- **Revision 5, Codex fourth recheck (2026-10-28):** ready for approval;
  one nit: test 21d's redock half checked an already-empty set and
  observed a window `redock` retires synchronously. The author changed
  21d to queue a measurement through a failed detached-window layout,
  assert it is in that set, and check the set through the kept
  `NativeWindowId` right after `redock` returns, as Codex proposed.
  Approved by the author under the maintainer's delegation, with P8-D1 to
  P8-D4 as recommended.
