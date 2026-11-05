# Plan: repair the open TODO entries that need no Windows or macOS

Status: **ready to implement slice by slice, revision 7, 2026-11-04.** Codex
evaluated revisions 1–6 ([r1](todo-repairs-review/codex-r1.md) to
[r6](todo-repairs-review/codex-r6.md)). r6 judged it "ready with changes",
and revision 7 makes those changes (see "Review record"). Slices 4a and 7 and
the maintainer's checkpoints remain as stated. Every decision D1–D9 is accepted. Nothing is
implemented. Every type, variant and method named below as new is a proposal
until a slice implements and documents it.

## Background

On 2026-11-04 the maintainer asked for every [TODO.md](../TODO.md) entry that
does not depend on Windows or macOS to be repaired. `main` was at 621022a
(0.3.94) with a clean tree. The author read each of the 28 open entries
against that tree. All 28 still describe the code: none has been resolved
silently.

## Scope

### Excluded: needs Windows or macOS

| TODO entry | Why excluded |
|---|---|
| NATIVE-02 | Drag-session and tab drag-out evidence on Windows and macOS only. Fedora GNOME/Wayland passed. |
| NATIVE-03 | Floating tool-window and own-window evidence on Windows and macOS only. Fedora GNOME/Wayland passed. |

### Partly included: mixed entries

| TODO entry | Included part | Remains open |
|---|---|---|
| NATIVE-01 | Nothing runnable here. The entry also needs Fedora KDE and Ubuntu 26.04 GNOME/KDE restart checks ([native evidence](../docs/verification.md#native-evidence)), which are not Windows or macOS but need machines this desktop is not. | Windows, macOS and the other Linux desktops. The entry is reworded to say so (slice 8). |
| NATIVE-04 | The Fedora minimize interruption runs (slice 8). | The macOS interruption check. No Windows interruption check is outstanding. |
| NATIVE-05 | The Linux hand checks (slice 8). | Windows and macOS. |
| `clipboard-win 5.4.1` yanked (R23) | The lockfile update and the Linux-side dependency gate (slice 6). | The Windows native clipboard check. |

### Included

| ID | TODO entry (file, item) | Kind |
|---|---|---|
| R01 | `NOW.md`: stale handoff | records |
| R02 | `docs/verification.md` "Workspace gate": incomplete command block | records |
| R03 | `runtime/held.rs` `HeldCancellation::FocusLost`: wrong link text | rustdoc |
| R04 | `runtime/measurements.rs` `Measurements::is_empty`: release-test warning | build hygiene |
| R05 | `tests/presentation/svg.rs`: test depends on `target/tmp` existing | test hygiene |
| R06 | `apps/native-checks` `HARDWARE`: adapter summary misses the text probe | tooling |
| R07 | `tests/diagnostics/ui/c01_*`, `c06_*`: implementor-list churn | verification |
| R08 | `pert-ui` `AppError`: unchecked list of convertible errors | public docs |
| R09 | Rustdoc and guide examples with unused imports | public examples |
| R10 | `diagram_editor` `NodeView`: label drawn under the rename editor | example |
| R11 | `DraftAlign`: no `Center` | public API |
| R12 | `validation/development.rs` overlap check: quadratic, flooding output | dev validation |
| R13 | `virtual_list.rs` rows: silent clipping of too-tall rows | dev validation |
| R14 | Built-in item reporters without `SemanticItem::local_bounds` | testing |
| R15 | `Runtime::content_search`: ignores `Semantics::items` | testing/search |
| R16 | `AlertDialog`: confirmation buttons not exposed as parts | inspection |
| R17 | `Button::inspect`: no paint colors in the box model | inspection |
| R18 | `ListItems::build` / `TreeItems::build`: placeholder row keys | public API (breaking) |
| R19 | `ToolWindow` in window mode, expanded at mount: no native window | contract defect |
| R20 | `SelectAnchor`/`MultiSelect`: disabled treatment differs from Button | presentation |
| R21 | `FrameImage::encode_png`: uncompressed output | dependency |
| R22 | TEXT-01: `TextArea` cannot hold a real document | large feature, **scope exception** (D6) |
| R23 | `Cargo.lock` `clipboard-win 5.4.1` yanked | dependency |

## Working rules

- One slice is one version, starting at 0.3.95. Each slice bumps the
  workspace patch version, the internal version requirements and
  `Cargo.lock`, as the maintainer's standing instruction says. Under D8
  (accepted) such a bump changes no external dependency, so it alone does not
  trigger the dependency row. Packaging and the consumer check run in slice 6
  and at the end of the plan.
- Each slice uses the AGENTS.md verification row for its highest-risk item,
  named in the slice. A slice that changes or could change a public item or
  the inspection contract runs the API listing gate. It names every changed
  item in its CHANGELOG entry and reviews the diagnostics snapshots it
  affects, regenerated from the actual output rather than from a list of
  expected files.
- Each slice removes the TODO.md entries it resolves, in the same commit. A
  slice that narrows an entry rewrites it to its actual remaining
  requirement.
- Breaking changes are allowed (unpublished library). Each records the
  replacement usage in the CHANGELOG and migrates the repository's consumers:
  examples, the gallery, guide chapters, tests and diagnostics snapshots.
- Before a slice touches its files, it re-reads its entries, because earlier
  slices may have moved line numbers.
- Slices 2–6 get a Codex implementation review (`codex exec -s read-only`)
  before merge. Slice 1 gets author review.
- Native evidence deferred to slice 8 stays listed as pending in each slice's
  CHANGELOG/Progress entry. No slice is called fully verified before it.
- Work happens on branch `todo-repairs` from `main`. Merging and pushing need
  the maintainer's instruction.

## Slices

### Slice 1: hygiene and records (R01–R07)

Version 0.3.95. Row: cross-package change (R06 touches the native runner and
the gallery's text probe, R05 the facade, R03/R04 the runtime). That means
the workspace checks, the API listing (expected unchanged), the diagnostics
snapshot test, and the release-profile runtime test for R04.

**R01 `NOW.md`.** Current state: "Updated 2026-10-15", version 0.2.96,
describes the color work as the latest state. Only `TODO.md` links to it.
Change (D1): delete it. First check that it holds no decision that exists
nowhere else; any such decision moves to the plan or design note it belongs
to. `plans/`, the CHANGELOG and plan status lines are the handoff.

**R02 verification gate.** Current state: `docs/verification.md:355-365` lists
five commands. AGENTS.md and `.github/workflows/ci.yml` also run the packaged
examples' headless sessions, the diagnostics snapshot test and the API listing
gate. Change: replace the block with a pointer to the AGENTS.md workspace gate
and the CI workflow as authoritative. Keep the explanatory diagnostics and
API-listing paragraphs that follow. This removes a third copy instead of
syncing it.

**R03 `HeldCancellation::FocusLost`.** Current state: `runtime/held.rs:16`,
link text `` `Runtime::clear_focus` `` targeting `HostIntegration::clear_focus`.
Change: link text `` `HostIntegration::clear_focus` ``, and mention
`Driver::clear_focus` (linked) for tests. Check: strict runtime rustdoc.

**R04 `Measurements::is_empty`.** Current state: `#[cfg(test)]`
(`runtime/measurements.rs:54`). Its only caller is under
`#[cfg(all(test, debug_assertions))]` (`runtime/tree.rs:2285`). Change: the
same cfg on the helper. Check: `cargo test --locked --release -p
pert-ui-runtime --lib` builds without the warning. The debug
measurement-eviction test still runs in the dev profile.

**R05 SVG test directory.** Current state: `tests/presentation/svg.rs:320` uses
`env!("CARGO_TARGET_TMPDIR")` without creating it. Change:
`std::fs::create_dir_all` first. Check: build the presentation test binary,
delete `target/tmp` **after** compilation, and run the already built binary
directly (not through Cargo, which would recreate the directory). It passes.

**R06 native-checks adapter summary.** Current state:
`apps/native-checks/src/main.rs:85` matches only `"Hardware: "`. The text
probe (in the gallery's wgpu text-mode check) prints
`"Text mode adapter: "`. Change: the text probe prints the documented
`Hardware: ADAPTER; details` line (the contract at `main.rs:43`, `:82`). Keep
its old line only if a test parses it. Tests:
- a runner unit test where log lines from two probes give a summary naming
  both adapters;
- a source assertion like `main.rs:836` that the text probe prints the marker.

Checks: `pert-ui-native-checks` and the probe's own package (Clippy, tests).
One native text-probe run is pending for slice 8.

**R07 diagnostics snapshot churn.** Current state: `c01`/`c06` `.stderr` list
rustc's first eight `Component` implementors. Several other snapshots have
similar lists. Change (D2): keep the cases and document in
`docs/verification.md` (diagnostics section) that a new built-in type changing
rustc's implementor list is expected churn, reviewed in the `.stderr` diff. The
pert-ui-authored messages, labels and notes are the intentional contract.
rustc's surrounding output is reviewed snapshot evidence. Check: the
diagnostics test passes unchanged.

### Slice 2: documentation checks and in-place editing (R08–R11)

Version 0.3.96. Row: public API (R11).

**R08 `AppError` conversion list.** Current state: the explicit list is on
the type's documentation (`crates/pert-ui/src/app.rs:8-20`, relative to the
doc block). It names `DrawError`, `RuntimeError`, `PathError` and the 16 leaf
errors of `leaf_conversions!` (`crates/pert-ui-runtime/src/error.rs:134-142`).
The blanket impl's documentation (about `:190-200`) already says
`DrawError: From<E>` decides and that the type's documentation "lists"
them. Change (D3):
1. Replace the type-level list with the rule ("exactly the errors `DrawError`
   converts from: …") and links to `DrawError`, `RuntimeError` and
   `PathError`.
2. Correct the impl documentation's "lists" wording.
3. Add a facade test that converts one value of each of the 19 accepted types
   with `?` and asserts the variant: `AppError::Runtime` for `RuntimeError`, the
   leaves and `DrawError::Runtime`; `AppError::Drawing` for `PathError` and the
   other `DrawError`s, as the code actually normalizes (verify first).
4. Add a comment at `leaf_conversions!` naming that test.

**R09 unused imports in examples.** The 2026-10-28 run listed failures in:
- `CommandPalette`
- `Runtime::content_search`
- `Runtime::reveal_child`
- `Runtime::capture_workspace`
- the `testing` module
- `TextField`/`TextArea::on_binding_refusal`
- `DraftEditor`
- `DraftEditorController::set_read_only`
- the guide's authoritative-values, appearance and controls chapters

Change:
1. Add `#![doc(test(attr(deny(unused_imports))))]` to the roots of
   `pert-ui-runtime` and `pert-ui`, and record the actual failures.
2. Remove each unused import.
3. Keep the attribute only with the maintainer's agreement (D4: a lint
   configuration change). If it is kept, the testing chapter's glob-only
   opening block gets a hidden `# #![allow(unused_imports)]`, its only
   exception.

Check: doctests of both crates.

**R10 diagram editor rename overlap.** Current state:
`examples/diagram_editor/canvas.rs:877` `NodeView` holds only `node`. Paint
(`:903-922`) always draws the label. `reconcile` (`:886`) returns `None` when
`node` is unchanged, which is the normal case when renaming starts. Change:
- add `renaming: bool`, set from the canvas's `renaming` (`:75`) where node
  views are described (`:443`);
- `reconcile` compares and adopts it and returns `Invalidation::Paint` on both
  transitions;
- paint skips the label while renaming;
- semantics keep the name;
- the rename editor uses `DraftAlign::Center` (R11).

Tests: a previously painted node entering rename paints no label text run, and
the editor is present. Commit, cancel and renaming another node each restore
the first node's label (display-list checks in the example's tests or headless
session). Native look: slice 8.

**R11 `DraftAlign::Center`.** Current state: `text_fields/draft.rs:26`,
`Start` (default) and `End`, `#[non_exhaustive]`. Change: add `Center` to the
shared editor-origin calculation. Text narrower than the editor is centered.
Wider text scrolls as `Start` does, keeping the caret visible. `Start` and
`End` behavior is unchanged. Tests:
- narrow text is centered: caret, selection rectangles and IME preedit
  position agree;
- wide text with the caret at the end or at the start;
- redescribing the alignment re-lays-out without losing the draft;
- `Start`/`End` regression cases.

Public: `api/`, CHANGELOG, and the guide's editing chapter if it mentions
`DraftAlign`.

Gate: the workspace gate, the API listing, the `diagram_editor` headless
session, and Codex.

### Slice 3: development validation (R12–R13)

Version 0.3.97. Row: public API/inspection contract. The API listing gate and
the serde configurations run if `ValidationIssue`'s serialized meaning
changes.

**R12 overlap check cost and flood.** Current state:
`validation/development.rs:744-760` filters `Disallow` nodes into `peers`,
compares all pairs, keeps pairs sharing a window and a parent, and emits one
issue per pair. `ValidationIssue::components` (`validation.rs:212`) means
every affected component, and overlays draw markers from it
(`development.rs:319`). Changes:
1. Group peers by (window, parent) and compare only within a group.
2. Inside a group, sort by the axis with the larger spread of rectangle
   starts and sweep along it with an active set. A disjoint row or column then
   costs O(n log n) candidate comparisons. Promise only that. The rustdoc
   states the cost in candidate comparisons, and says that a two-dimensional
   arrangement (a disjoint grid) or many genuine overlaps can still make
   candidate comparisons quadratic. Grouping by parent already bounds the
   common case.
3. Count pairs per parent while scanning, and build per-pair issues only
   while the count is at most the threshold. The threshold is 8 pairs, a
   private `const` documented in the validation rustdoc. At 8 or fewer pairs,
   each pair is its own issue. At 9 or more, one summary issue per parent:
   code `UnexpectedOverlap`, `components` = **all** distinct participating
   components in tree order (so overlays mark every one), and a message naming
   the parent, the component and pair counts and the first 4 pairs in
   deterministic order. No new public field. If the field's meaning or
   serialization changes after all, the inspection schema, API listings and
   serde tests follow.
4. The existing repeated-log suppression applies to the summary. It clears
   when the overlap goes away and reports again on reappearance.
5. Mark the intentional overlap in
   `tests/runtime.rs::shared_live_deadlines_skip_missed_frames_without_visiting_static_components`
   with the public allowed-overlap hint.

Tests:
- a `cfg(test)` candidate-comparison counter stays below a stated
  O(n log n)-shaped bound for 200 disjoint siblings in a row and in a column;
  no grid bound is asserted, because none is promised;
- exactly 8 pairs give 8 issues; 9 give one summary listing all participants;
- two parents give two summaries;
- markers cover every participant;
- the ordering is deterministic;
- suppression, clearing and reappearance;
- the existing validation tests pass unchanged.

Docs: where the validation report is described.

**R13 virtual list row clipping.** Current state: `virtual_list.rs:2121-2161`
places rows with `place_clipped(child, slot, slot)` and does not measure a
row's natural height. `ExceedsFixedHeight` (`validation.rs:209`) compares a
child's latest recorded measurement (`development.rs:710`) against its
**parent's** box when the parent sets the boolean
`ValidationHints::fixed_height` (`validation.rs:137`, R2-23). There is **no**
API that carries a per-child slot height. Change: add one public
arrangement-time operation, proposed `ArrangeContext::place_fixed_slot(child,
slot, clip)`, with the same placement as `place_clipped` plus a declaration
that `slot` is a fixed-size slot the child must fit:
- **Owner and units:** the arranging parent, in its local logical pixels, like
  every `ArrangeContext` rectangle. It validates the rectangle as
  `place_clipped` does.
- **Measurement: an ordinary measurement, in every build.** `place_fixed_slot`
  measures the child through `ArrangeContext::measure` at the slot's width
  with unbounded height, then places it exactly as `place_clipped` does. This
  is the same kind of request a `Column` makes for its children, so its
  result is legitimate ordinary evidence. It becomes the child's latest
  measurement, which is what `ExceedsFixedHeight`, update-time validation,
  `refresh_forwarded` and change notification already read. **No probe, no
  isolation and no second evidence store is needed.** Revisions 4 and 5 tried
  a debug-only probe that was isolated from the ordinary evidence. Codex r4
  and r5 showed that it leaks through or starves several readers
  (`runtime.rs:955`, `:961`, `:6243`; `tree.rs:1839-1887`, `:2231`, `:2263`;
  `layout.rs:1225`), so that approach is abandoned. The new slot operation
  makes its ordinary measurement in both profiles. This is not a claim of
  global debug/release parity, since `refresh_forwarded` already measures
  only in debug builds.
  - **Cost:** one additional top-level measurement request per
    `place_fixed_slot` call. It is a cache hit while equal constraints remain
    among the `RETAINED = 4` entries. A miss (after invalidation or eviction)
    can measure descendants recursively. A retained parent arrangement makes
    no placement-time request at all (`runtime/tree.rs:1957`). Only placed
    rows (visible plus overscan) are requested, never the whole collection.
    Estimated lists already measure mounted rows in `measure_lines`
    (`virtual_list.rs:1690`); horizontal lists use unbounded width and the
    slot's height (`:1708`), so their slot request has different constraints
    from that existing measurement. A cold `TreeRow` measures its content
    unbounded and then again at bounded height during arrangement
    (`tree.rs:755`, `:773`). The rustdoc of `place_fixed_slot` and of
    `VirtualList` states these rules.
  - **Horizontal lists:** the slot is fixed in width, so the check compares
    the measured width with the slot width. The operation's declaration names
    the fixed axis (or there are two operations); the slice chooses this and
    documents it. The slice measures the `log_explorer` and
    `inventory` headless sessions before and after, and records any change
    in their timing in the CHANGELOG.
  - Measurement errors propagate through layout as any measurement's do.
  - **Units:** `Node::measured_height` is scaled, while the declared slot is
    in the parent's local logical pixels. The comparison converts both to the
    same unit, with a test at a non-1 scale.
- **Slot record.** The slot height is parent-authored but stored on the child,
  as committed placement already is (`runtime.rs:900`). The internal
  `Placement` tuple (`tree.rs:24`, `(Rect, Option<Rect>)`) becomes an internal
  struct with an optional `slot` field, staged and committed with the
  placements a parent's arrangement produces (`tree.rs:21`, `:2185`). This is
  a new internal addition. Rules:
  - a fresh `place_fixed_slot` sets it;
  - a fresh ordinary placement of the child clears it, even at identical
    geometry;
  - a retained placement (the parent's arrangement is reused) preserves it;
  - a child arranging alone does not touch it;
  - setting and clearing are staged and adopted only when the layout
    succeeds, and discarded on failure;
  - unmount removes it;
  - a slot-only change (same rectangle, slot set or cleared) reaches
    validation even when the child's own arrangement is skipped.
- **Report:** under `cfg(debug_assertions)`, as for all development
  validation, the validator compares the child's latest measured height with
  its committed slot height and reports `ExceedsFixedHeight` when the
  measurement exceeds it. When the parent sets `fixed_height`, its box check
  continues to apply as it does today.
  Update that variant's rustdoc and the fixed-height section of
  `docs/contracts.md` (about `:2109`): the fixed height is the parent's box or
  a declared slot.

`VirtualList` (and so `Tree`, through `TreeRow`) switches its row placement to
the new operation. An external composite can do the same. Tests:
- a `VirtualList` row with a `Button` taller than the row height reports once
  per offending row component;
- a fitting row reports nothing;
- a forwarding wrapper row reports its child;
- a `Tree` row through `TreeRow` reports;
- scrolling does not report rows that fit, and does not duplicate reports;
- retention: switching from `place_fixed_slot` to ordinary placement at
  identical geometry clears the report; a retained parent placement keeps
  it; a failed layout neither adopts nor clears; unmount removes it;
- a measurement error during the slot measurement fails the layout and
  adopts no slot record;
- a later ordinary cache hit at the slot constraints with unchanged geometry
  leaves the report consistent;
- a forwarding wrapper row that was never measured before gets a
  measurement, and `refresh_forwarded` uses it;
- separate counters for measurement requests, cache hits and component
  `measure` calls show, for an unchanged visible row: one additional request
  per `place_fixed_slot` call, served as a hit; no request when the parent's
  arrangement is reused; and a miss after invalidation. Cover fixed and
  estimated lists, vertical and horizontal;
- a slot-only change with an unchanged rectangle updates the report;
- a non-1 scale compares in matching units;
- an external composite using `place_fixed_slot` gets the same check.

Gate: the workspace gate, the API listing, Codex.

### Slice 4: testing and inspection reach (R14–R16)

Version 0.3.98. Row: public API/inspection contract.

**R14 item rectangles.** Current state: none of the families below sets
`SemanticItem::local_bounds`. The reporters differ, and each must be named:
- **Accordion** reports one item per section (`collapsible.rs:352-357`). Its
  header rows' rectangles come from the Accordion's own header layout, in its
  local space.
- **Collapsible** reports no item for its header today. Decide whether it
  should (one item, the header). If not, it leaves this entry, and the entry
  says why.
- **Choice controls** (`choice_controls.rs`): each option's painted and
  hit-tested rectangle.
- **Tool-window stripe**: each stripe button's rectangle from the stripe's
  layout.

These four families own the geometry of the items they report, so they are
implemented directly.

**MultiSelect and QuickPick need a design checkpoint first (slice 4a).**
MultiSelect reports options on its trigger (`multi_select.rs:1265`). Its open
popup's rows are children of a structural `VirtualList` that exposes no
options part (`:880`). QuickPick reports items on its panel
(`quick_pick.rs:1494`), and its documented ListBox details live there
(`:830`). Its structural result list is recreated under a new key when the
results change (`:1369`). `VirtualList` deliberately does not enumerate its
rows as items (`virtual_list.rs:2714`). The reporter therefore does not own
the row geometry, and no public facility gives it the geometry. Before
implementing, a short design note `plans/todo-repairs-r14-popups.md` answers
the following, and gets a Codex review and the maintainer's checkpoint:
- which public facility lets a reporter take item rectangles from a child
  list's row placement (for example, the list exposing placed-row rectangles
  by row key to its owner), and whether an external composite can use it;
- whether the trigger's and panel's existing items remain where they are,
  with rectangles only while the popup is open (closed-anchor rectangles stay
  absent, as the [parts checkpoint](usability/round-3-design-parts.md)
  requires);
- how filtered positions map to source options, and that unmounted rows have
  no rectangle;
- how labels, selection, disabled state, shortcuts and actions stay where they
  are documented, including disabled-item activation through `click_item`;
- QuickPick's re-keyed list and stale-geometry behavior.

If the checkpoint finds no acceptable design, these two families stay in
TODO.md with the reason, and slice 4 completes without them.

For all families: rows scrolled fully out of view or clipped have no
rectangle. Partially visible rows follow what the table titles do today (check
`table.rs`), and the rule is documented. No rectangle exists before the first
layout, nor after a failed layout that arranged the reporting component (the
qualification in `docs/contracts.md` about `:1620`). Inspection computes and
schedules nothing.

Tests per family:
- `Driver::click_item` on a non-first item changes the state it stands for;
- after filtering (MultiSelect, QuickPick) and scrolling, a later row's
  rectangle is correct;
- a partially visible row and a fully clipped row;
- wrapped Accordion headers;
- a scrolled stripe;
- a non-unit view scale (view transform, not only device scale);
- absence before layout, and after a failed layout that arranged the
  reporter.

**R15 content search over items.** Current state: `content_hit`
(`runtime/content_search.rs:266`) matches `semantics.name` and `value` and
emits at most one hit per component. `jump_to_content` takes a
`ContentTarget` (`content_search.rs:149`) and items have no identity of their
own. `ContentField` (`:165`) is `#[non_exhaustive]` with `Name`, `Value` and
`Row`. Change:
- Add `ContentField::Item { index }` (or a separate optional index; the slice
  chooses the smaller public change). The hit's target stays the component's
  widget target.
- `jump_to_content` keeps revealing and focusing the **component** only.
  Selecting an item would exceed the entry and could change application
  state.
- The index is a presentation-time position, documented as such: it is not
  an identity, and a stale index is never used to retarget.
- **Item scan state.** Today the cursor (`content_search.rs:239`) holds
  pending components, an optional `RowScan` for unmounted rows and visited
  collection revisions. Components are charged once when popped
  (`runtime/content_search.rs:129`). Add an `ItemScan` beside `RowScan`:
  - When a component is visited, its name and value are matched as today.
    Its item texts are then **moved** (not cloned) out of the `Semantics`
    that the visit's single `inspect()` call already produced, into an owned
    snapshot `ItemScan { component, texts, next }`. Resuming an `ItemScan`
    never inspects its owner again or rebuilds the snapshot. Today
    `content_hit` owns and drops the inspection result, including on its
    early returns (`runtime/content_search.rs:272`). The slice therefore
    restructures it so the snapshot is taken before the visit returns, even
    when the name and value do not match.
  - **Cost accounting:** the one-time inspection and snapshot are part of the
    component's visit, which is charged as today. Their cost grows with the
    component's item count and text volume, and the work budget does not
    divide that cost. The budget documentation (`content_search.rs:232`) is
    qualified to say so. Matching the snapshot is resumable, with one work
    unit per examined item. Strictly bounded acquisition would need
    incremental item reporting, which is out of scope here and is recorded
    in TODO.md as a follow-up if Codex or the maintainer wants it.
  - **Order: the existing order is kept.** The component's name/value hit
    comes first, then its item hits by index, then its unmounted rows
    (`RowScan`, which already takes priority over `pending`,
    `runtime/content_search.rs:145`), then its children through `pending`.
    `ItemScan` takes priority over `RowScan`, and both over `pending`. There
    is one slot of each, which suffices because a component's `ItemScan`
    completes before its `RowScan` starts and its children are visited only
    after both. No nested continuation is needed.
  - **State captured before yielding:** when a name or item hit fills the
    result limit, the cursor has already recorded everything the visit
    determined: the `ItemScan` snapshot, the collection revision and the
    `mounted_current` information the `RowScan` needs, and the children to
    push. Resuming does not inspect the component again. The existing naming
    fallback, which inspects ancestors while visiting a descendant
    (`runtime/content_search.rs:323`), is unchanged and outside this
    promise.
  - If the result limit is reached on the name hit, the item scan is still
    saved and resumes on continuation.
  - `done` is false while an `ItemScan` or `RowScan` remains.
- **Mutation:** a continuation reports the snapshot taken when the component
  was visited. Items inserted or removed later are seen only by a new search.
  This gives no duplicates and no skipped items within one search, and the
  documentation says so. The hit's `text` is the snapshot's text.
- When `Semantics::protected` is set, item text is not searched, while the
  component name stays searchable as today.
- The rules for modality, excluded subtrees and `names_children` apply
  unchanged.
- Update the "Content search" section of `docs/contracts.md`.

Tests:
- a toast by its message;
- a menu command by its label;
- an unselected tab title;
- a protected component's items not found but its name found;
- a component whose name and one item both match gives two hits in order;
- a tiny result limit and work budget with continuation, without duplicates
  or skips, including a limit reached on the name hit;
- an item inserted before the saved index between calls: no duplicate, and a
  new search sees it;
- `done` stays false until the item scan and row scan both finish;
- nested collections with a one-unit budget and a one-result limit: a
  collection component with matching items, mounted descendants, unmounted
  matching rows and a matching outside sibling yields its hits in the
  documented order, with no duplicates or skips;
- an inspection-call counter (`cfg(test)`) shows that resuming an
  `ItemScan` does not inspect its owner or rebuild its snapshot;
- a modal blocking a window;
- `jump_to_content` on an item hit focuses the component.

**R16 AlertDialog parts.** Current state: `dialog.rs` builds the buttons under
private keys `accept`/`cancel` in scope `dialog/confirmation` (`:612`). The
rustdoc (`:512`) calls them internal, and the example (`:573`) finds `cancel`
by key. Change: expose them as parts named `accept` and `cancel` of the
existing addressable confirmation owner, through the public composite-part
mechanism. Document the route (the confirmation owner, then the exposed part)
and update the rustdoc and example. Tests:
- a public-API pointer test clicks each part through `Driver` and observes the
  outcome;
- redescription with changed labels (localization) keeps the buttons'
  identity, semantic names and workspace paths.

Gate: the workspace gate, the API listing, Codex.

### Slice 5: API repairs (R18–R19)

Version 0.3.99. Row: public API (R18 is breaking).

**R18 keyless list and tree rows.** Current state: `virtual_list.rs:105` `fn
build(&self, index: usize) -> Option<Element>` and `tree.rs:45` `fn
build(&self, key: &str) -> Option<Element>`. The collection replaces the
returned key, so the examples write `.key("row")`
(`virtual_list.rs:991`, `guide/layout.md:995`, the diagram editor). `EachItem`
(`described.rs:505`) is a **sealed trait** whose consuming conversion takes
`self`, so it cannot be a trait-object return type. Change:
- Add a concrete public type, proposed name `RowItem`, that wraps a
  **complete `Element`**. `impl<C: Component + 'static> From<C>` builds that
  element with `Element::new` while `C` is still known, under an internal
  placeholder key. So the typed reconciliation function, the component's
  type identity and its initial children are captured exactly as for any
  element. A `Box<dyn Component>` would lose them, because
  `Component::reconcile` needs `Self: Sized` (`runtime.rs:1166`) and `Element`
  stores a typed reconcile function (`layout.rs:348`, `:425`).
  `From<Element>` keeps the element whole: label, rich label, label source,
  icon slot, addressing, exposed part, permissions, searchability, grow and
  children.
- A **public** consuming conversion,
  `RowItem::into_element(self, key: impl Into<String>) -> Element`, replaces
  only the key. The built-in `VirtualList` and `Tree` use it, and so can an
  external collection, which keeps the privileged-path rule (`Element` has no
  public key setter, so this is the only public way to do it). Document
  ownership, and that the key must satisfy the ordinary sibling-key rules.
- Signatures become `fn build(&self, index: usize) -> Option<RowItem>` and
  `fn build(&self, key: &str) -> Option<RowItem>`.
- `EachItem`'s sealed conversion may delegate to the same placeholder-free
  path, so the two do not drift. It stays sealed, since it is generic and not
  a trait-object return.

Migrate every implementation: runtime docs, the facade guide, the gallery
`pages/data.rs`, the examples, tests, and the diagnostics snapshots the
compiler actually changes. CHANGELOG: replacement usage. Tests:
- an `Rc<dyn ListItems>` and an `Rc<dyn TreeItems>` example compiles and runs;
- heterogeneous rows (two component types);
- an `Element` row keeps its metadata and its initial children;
- redescribing a row with the same component type reconciles it in place (no
  remount) through the captured typed function;
- an external collection test (public API only) uses
  `RowItem::into_element` and gets the same keyed result as `VirtualList`;
- reordering keeps the same keyed row's `WidgetId`;
- removing and recreating a row makes the old `WidgetId` stale, and an event
  to it is refused.

**R19 tool window opens its own window at mount.** Current state:
`tool_windows/mod.rs:385` promises presentation "when it expands".
`request_detachments` (`:2094`) selects expanded window-mode windows not in
`detached`/`detaching` and runs only from controller changes (`:1462`).
`detachable` (`:400-405`) restricts the **user**: `set_mode` does not check
it, and neither does the detach helper. A failed request returns the panel to
**Docked**. Change:
- Application descriptions keep application authority. An initial
  `ToolWindowMode::Window` + `expanded(true)` is honored regardless of
  `detachable`, as `set_mode` is. Document this.
- After the first layout that mounts the host, the host calls the same
  `request_detachments` from the event phase through the public post-layout
  notification contract, never from reconciliation or arrangement, and
  publishes no signals there.
- The existing `detaching`/`detached` tracking prevents duplicates. There is
  no permanent once-flag, so a later dock-and-detach still works.
- A failed request behaves as the controller path does (back to Docked, with
  its report), and the rustdoc says so.
- The rustdoc explains why the request waits for layout. The helper accepts
  missing floating geometry, so the reason is event-phase scheduling, not
  geometry.

Tests (headless host):
- mount with Window+expanded produces one `PresentationAction::Detach`;
- Window+collapsed produces none until expanded;
- a second flush while pending produces no duplicate;
- successful completion;
- host queue refusal and a failed receipt (Docked plus report);
- collapse or removal while pending;
- docking followed by another detach;
- `detachable(false)` does not block the application's initial description.

Native: slice 8.

Gate: the workspace gate, the API listing, the gallery headless run, Codex.

### Slice 6: paint consistency, inspection colors and dependencies (R17, R20, R21, R23)

Version 0.3.100. Row: dependencies (R21, R23), so this slice also runs MSRV,
the supported feature configurations, release-profile tests and packaging.

R17 and R20 share Button paint and are done together.

**R20 disabled dropdowns.** Current state: `select.rs` (`SelectAnchor::paint`,
`:1351`) and `multi_select.rs` (`:1200`) paint disabled anchors with Button
fill and `Muted` text. A disabled framed Button (b9e4c6c, `controls.rs:824`)
uses `Surface` fill, `Muted` content and an inward `Outline`
(`docs/rendering-platform.md:24`). Change: one private helper for the neutral
inactive-control frame, used by Button, `SelectAnchor` and the MultiSelect
anchor. `InlineSelect` keeps its separate inline presentation. Update
`docs/rendering-platform.md:24`. Tests:
- per control, light and dark, assert the **anchor's own** frame and content
  commands (not scene-wide color presence);
- enabling restores the enabled treatment;
- revoking availability during a press does not activate or replay;
- adaptive role and `BorderWidth` changes;
- selected MultiSelect chips and icon/rich captions;
- `tests/presentation/disabled_button.rs` passes unchanged.

**R17 Button paint in inspection.** Current state: `Button`'s inspection
(`controls.rs:816-878`) reports semantics but no colors. Container and
Toolbar report theirs. Change: each paint replaces the complete retained
optional metadata (background, foreground, border), clearing what the new
state does not paint. Inspection reports the last painted values: absent
before the first paint, unchanged after a failed paint, and documented as
"last painted". It computes and schedules nothing. Tests:
- before paint, absent;
- enabled framed;
- disabled framed (the neutral frame);
- disabled to enabled removes the outline;
- frameless at rest has no background, while hovered or pressed frameless
  reports its frame;
- a theme change after one frame;
- a retained paint is reused;
- repeated inspection schedules no work.

**R21 compressed PNG captures.** Current state: `capture.rs:465-482`
documents, and `:616-710` implements, stored deflate blocks without row
filters, "deterministic and dependency-free". The test helper `scanlines`
(`:788`) parses stored blocks and is not an inflate decoder. Change (D5):
- Use `miniz_oxide` from `[workspace.dependencies]` (already in the lockfile
  through `png`/`flate2`) at a fixed level, with a fixed per-row filter rule
  (for example Sub or Up chosen by a documented heuristic, or one fixed
  filter).
- Determinism scope, as documented: the same input gives the same bytes for a
  given pert-ui version. A dependency update may change the bytes, never the
  pixels.
- Update the rustdoc's "stored blocks" and "no dependency" sentences.
- Keep the un-premultiply rules.

Tests:
- decode with an independent decoder: the `png` crate as a dev-dependency of
  `pert-ui-runtime`, from the workspace;
- migrate every test that relies on the stored layout, including the alpha
  tests;
- round trips of noisy pixels, a flat UI-like image, translucent pixels, a
  1×1 image, an all-transparent image, widths around the filter and block
  boundaries, and a large image near the maximum capture size;
- two calls give identical bytes;
- a measured size for a 1040×600 UI-like capture, recorded in the CHANGELOG,
  with no promised ratio.

**R23 `clipboard-win`.** `cargo update -p clipboard-win` to a non-yanked
compatible release. Check `cargo package` no longer warns. The Windows
clipboard check stays open: the TODO entry is narrowed to it.

Commits: R17+R20 together (shared Button paint), R21 alone, R23 alone, so
each is reviewable. They share one verification batch at the end of the
slice.

Gate: the complete dependency/feature/platform/release row of AGENTS.md on
Linux:
- the workspace checks and MSRV;
- every supported feature configuration the row lists;
- release-profile tests and the API listing;
- `cargo package` for the library packages, with the package list inspected;
- an isolated consumer check against the extracted packages;
- the headless gallery.

Native Windows evidence (clipboard) is recorded as missing, and the Linux
native checks are deferred to slice 8. Codex.

### Slice 7: large-document editing (R22, TEXT-01): scope exception

This entry asks for a decision and measurements before design. It does not fit
a repair slice. With the maintainer's agreement (D6), this plan only produces
the design plan `plans/document-editor.md`, and **TEXT-01 stays open**, pointing
to it. That is an explicit reduction of the requested scope, not a repair. The
design plan states:
1. The decision between extending `TextArea` and a separate document editor.
2. The baseline. Today's path rejects text over `MAX_TEXT_BYTES` (16 KiB,
   `text_input.rs:57`). The binding check is `check_text` (`text_fields.rs`
   about `:3814`), and the edit refusals are at about `:3189` and `:3256`.
   Revision 7 cited `text_input.rs:104`, which is the preedit check. The baseline is therefore
   measured within that limit (edit latency and memory at 4, 8 and 16 KiB),
   and the rejection behavior above it is recorded. Larger measurements (for
   example 1 MB of prose, 50,000 log lines) run only on an explicitly
   identified experimental path, never presented as current-path figures.
3. Per-paragraph preparation and visible-only rasterization; selection,
   caret, IME preedit and hit testing across paragraphs; the effect on the
   public `TextSelection`.
4. Separating editor capacity from the shared label/toast limit, and how
   `TextLimit::MAX_RASTER_SIDE` (8192, `text.rs:116`) stops bounding it.

The plan gets its own Codex design review and maintainer checkpoint.

### Slice 8: Linux native evidence (maintainer)

After slices 1–6, on Fedora 44 GNOME/Wayland. Each record gives the commit,
the command, the backend (GPU or CPU/softbuffer), the platform details
(kernel, adapter, driver), the result and any failures, in
`docs/verification.md`.
- **NATIVE-04 Linux part:** minimize the window during each selected probe:
  `interaction` with GPU, `interaction` with CPU, `workspace` with GPU and
  `workspace` with CPU. Expected: the runner ends that probe with the reported
  "hidden" cause, not an unrelated assertion or timeout. Record the cause line.
- **NATIVE-05 Linux remainder:**
  - `SelectableText` pointer selection and copying into another application;
  - table double-click and F2 editing, with commit and cancel;
  - Tab through a scoped table and out of it;
  - IME composition into a cell editor;
  - `diagram_editor`: add, move, connect, rename and undo nodes;
  - `log_explorer`: filter, search and scroll to the end with 10,000 lines;
  - `inventory`: sort, edit a cell, add and remove an item.
- **This plan's changes:**
  - R10/R11: the rename shows one centered label;
  - R19: an expanded window-mode tool window opens its own window at start;
  - R20: disabled dropdowns in both palettes;
  - R06: a text-probe run whose summary names its adapter.

Afterwards, NATIVE-01, NATIVE-04 and NATIVE-05 are rewritten to their actual
remaining requirements.

## Progress

- **Slice 1 (0.3.95), 2026-11-04.**
  - R02–R07 are done.
  - R01: the two decisions only `NOW.md` held moved to
    `plans/follow-ups.md`. `NOW.md` turned out to be an untracked local file
    (`.gitignore` had `/NOW.md`), so deleting it cannot be undone through
    Git. The maintainer chose deletion on 2026-11-04: the file and its
    `.gitignore` line are removed, and so is the TODO entry.
  - Checks:
    - the workspace gate as AGENTS.md lists it, all commands passing;
    - the API listing unchanged;
    - the diagnostics snapshots unchanged;
    - the release-profile runtime lib test built with no warning;
    - the built presentation binary passes after `target/tmp` is removed;
    - the `pert-ui-native-checks` tests, including the new adapter test.
  - Pending: one native text-probe run (slice 8).
  - Review: author review (no Codex review for slice 1, per the working
    rules).

- **Slice 2 (0.3.96), 2026-11-04.**
  - R08–R11 are done.
  - R09: the 2026-10-28 list reproduced exactly (14 examples). The lint is
    permanent (D4), and the pert-ui doctests pass in the no-default, `cpu`,
    `svg`, `image-formats`, `serde` and `desktop-cpu` configurations.
  - R10: the diagram editor's headless session counts painted node labels. A
    negative control (the paint skip removed) fails it.
  - R11: `api/` adds only `DraftAlign::Center`.
  - Checks: the workspace gate, all commands passing.
  - Codex implementation review ([impl-s2](todo-repairs-review/impl-s2.md)):
    "ready with fixes", no production defect, and two test gaps, both fixed:
    - a test now compares the selection fill, preedit underline, IME anchor
      and pointer presses at both text edges across Start, End and Center;
    - the session checks labels per node (only the renamed node hides its
      label, and renaming another node restores the first).
  - Pending: native look of the rename (slice 8).
  - Codex recheck ([impl-s2-r2](todo-repairs-review/impl-s2-r2.md)): "ready".

- **Slice 3 (0.3.97), 2026-11-04.**
  - R12 and R13 are done.
  - R12: the scheduling test's half-million-line overlap flood is gone; the
    gate log shrank from about 501,000 lines to 6,450.
  - R13: the new public `ArrangeContext::place_fixed_slot` measures the
    child in every build. The slot height travels with the committed
    placement, and the parent's commit passes it to the validator, since the
    children's geometry commits first.
  - The reports found rows that were already cut off, all fixed:
    - the built-in `FileBrowser` let names and failure text wrap in
      fixed-height rows;
    - three gallery lists clipped their rows.
  - Horizontal lists have no check yet (new TODO entry, since they need a
    width code).
  - The R13 tests use component `measure` call counts rather than separate
    request/hit counters: the runtime exposes no public request counter, and
    the cache-hit path is shown by an unchanged component call count.
  - Checks:
    - the workspace gate (the first run failed on formatting and a
      `type_complexity` lint, both fixed; Clippy and formatting rerun clean,
      all other commands passed);
    - `api/` adds only `place_fixed_slot`.
  - Codex implementation review ([impl-s3](todo-repairs-review/impl-s3.md)):
    "needs fixes", no blocker. All findings verified and fixed in a review
    commit (no version bump):
    - retained arrangements refresh the slot's window extent at the child's
      own geometry commit;
    - slot reports need committed geometry in a measured window;
    - `FileBrowser`'s failure-label factory and placeholders end in an
      ellipsis, and a narrow-row test covers before and after a string-table
      replacement;
    - the horizontal case is now implemented, not deferred:
      `place_fixed_slot` takes an `Orientation` axis, and the new
      `ValidationCode::ExceedsFixedWidth` moves the inspection schema to 18;
    - more contract tests (one report per row, scales, zoom, unmount,
      measurement failure, unmeasured window);
    - the timing evidence is recorded;
    - the documentation nits are fixed;
    - the TODO link to the uncommitted design plan is removed.
  - Deviations from revision 7, accepted by the author and named for the
    maintainer:
    - summary `components` are in ascending incarnation order, which is
      deterministic, rather than tree order;
    - the internal placement stays a tuple with a slot element, rather than a
      struct;
    - the zoom regression test passes even without the retained-scale
      refresh (the scenario did not reproduce through public API), so it
      documents the behavior rather than proving the fix;
    - request/hit counters are replaced by component measure counts.
  - Codex recheck ([impl-s3-r2](todo-repairs-review/impl-s3-r2.md)): "needs
    fixes" for test evidence only; the production code was judged sound.
    Fixed in a further review commit:
    - the zoom test now uses content close to the slot and fails with the
      retained-scale refresh removed (negative control run);
    - the failure test fails only the slot measurement (a new width forces it
      past the cache) and checks that nothing is reported or adopted until a
      layout succeeds;
    - an `ExceedsFixedWidth` issue and marker serde round trip;
    - narrow long loading, empty and listing-error placeholders (negative
      control run);
    - the debug/release sentence narrowed to this operation.
    Not added, and named for the maintainer: separate tests for child-only
    arrangement, scrolling a retained parent and a forwarding-wrapper row.

- **Slice 4 (0.3.98), 2026-11-04.**
  - R15 and R16 are done. R14 is done for the families that own their
    geometry: Accordion headers, RadioGroup/SegmentedControl/InlineChoice
    options and tool-window stripe buttons.
  - `Collapsible` keeps reporting no item, since the component is its header
    and an item would duplicate its content-search hit; the TODO entry says
    so.
  - MultiSelect and QuickPick wait for slice 4a: the design note
    [todo-repairs-r14-popups.md](todo-repairs-r14-popups.md), with a Codex
    review and the maintainer's checkpoint. The TODO entry is narrowed to
    them.
  - R16 deviates from the spec's wording: the parts are not on the
    `AlertDialog` itself. The exposed-part walk stops at the `dialog`
    surface, an ordinary addressable child, so the parts belong to the
    generated `dialog/confirmation` element and are reached with
    `Runtime::find` first. Making `dialog` a workspace part would change
    application content addresses.
  - R14 tests click a later item in each family; the generic clipping,
    scale and failed-layout rules come from the runtime's existing
    `local_bounds` handling and its tests.
  - Checks: the workspace gate, all commands passing; `api/` adds only
    `ContentField::Item`.
  - Codex implementation review ([impl-s4](todo-repairs-review/impl-s4.md)):
    "needs fixes", five should-fix findings, all fixed in a review commit
    after slice 5 (described in the 0.3.99 CHANGELOG entry):
    - resumed item scans recheck modality;
    - `QuickPick`'s surface is excluded from content search;
    - item/row/children ordering under one-unit steps;
    - a new search sees an added item;
    - the R14 and R16 tests the review asked for.
    Negative controls fail for the modality and picker fixes.
  - Pending: Codex recheck.

- **Slice 7 (design only), 2026-11-04.**
  - [plans/document-editor.md](document-editor.md) was written by an agent
    with a measured baseline; raw output is in
    `plans/document-editor-bench/results/`.
  - Codex reviewed it four times
    ([design-s7](todo-repairs-review/design-s7.md) to
    [design-s7-r4](todo-repairs-review/design-s7-r4.md)), and each revision
    adopted the findings.
  - After the fourth review the maintainer stopped iterating: the plan is
    accepted as direction. The fourth review's findings are the plan's open
    design items OD1–OD6, settled by its raster prototype and document/undo
    slices.
  - TEXT-01 stays open and points to the plan.
  - The design work found that a `TextArea` value within the byte limit can
    still fail a flush on the raster limit, now a TODO entry.

- **Slice 5 (0.3.99), 2026-11-04.**
  - R18 and R19 are done.
  - R18 migrated 96 implementations with ast-grep and a compiler-driven fix
    loop; public examples were cleaned by hand.
  - R19 requests the detach from the panel's update, through the layout's
    delivery.
  - Checks: the workspace gate (Clippy needed one test fix, then passed
    clean). `api/` changes `ListItems::build`/`TreeItems::build` and adds
    `RowItem`.
  - Codex implementation review ([impl-s5](todo-repairs-review/impl-s5.md)):
    "needs fixes". Its blocker was a serde-only test the migration loop
    missed, because the loop built only the default features. Fixed in a
    review commit:
    - the serde test's `.into()`;
    - bounds guards restored in two gallery sources;
    - the examples' and tests' remaining `.key("row")` removed;
    - a typed-reconciliation test through `From<C>` with initial children;
    - R19 tests for a refused mount-time detach (Docked, still expanded,
      reported as a presentation change) and a collapse while pending;
    - trailing spaces in the review records.
    Clippy now passes in every supported `pert-ui` feature configuration,
    after gating the `AppError` test on `native`.
  - Codex recheck of the slice 4 and 5 fixes
    ([impl-s45-r2](todo-repairs-review/impl-s45-r2.md)): "needs fixes".
    Its main point was that excluding every `QuickPick` surface from content
    search was a regression for pickers that list application values. The
    exclusion is now the opt-in `QuickPick::searchable(false)`, which the
    gallery's search picker uses. The remaining test gaps are closed:
    - an explicit order with row keys;
    - a full window request queue;
    - partial stripe clipping;
    - the alert's paths and cancel after a redescription.
  - Pending: Codex recheck; the slice 4a implementation (slice 4b).

- **Slice 6 (0.3.100), 2026-11-04.**
  - R17, R20, R21 and R23 are done, in two commits: the dependencies (R21,
    R23, with the version bump) and R17 with R20. The spec asked for three,
    but one lockfile change carries the clipboard update, the new dependency
    and the version, so R21 and R23 share a commit.
  - R20's shared routine also returns what it painted, so the Select and
    MultiSelect anchors report their colors like Button (R17), and the
    dropdown tests read each anchor's own inspection.
  - R21 uses `miniz_oxide` (D5) with min-sum filters; a synthetic
    1040×600 interface frame encodes to 4,370 bytes.
  - Checks: the workspace gate (`api/` unchanged); the dependency row after
    commit: release tests, the feature configurations, the headless gallery,
    packaging and the consumer check.
  - The dependency row passed on the clean tree at b0133d2 (release tests,
    every feature configuration, the headless gallery, packaging) and the
    packaged-consumer check for 0.3.100 passed.
  - Codex implementation review ([impl-s6](todo-repairs-review/impl-s6.md)):
    "needs fixes", five points, fixed after slice 4b's merge:
    - a zero border width reports no border;
    - the anchors record their colors only after every paint operation;
    - command-level anchor tests with an adaptive role and a zero border
      width, plus a pressed frameless button;
    - inspection checked to schedule nothing;
    - PNG round trips at extreme aspect ratios and, in the release profile,
      at the 64 MiB limit;
    - `FrameImage`'s "without any dependency" corrected, with encoding's
      temporary memory documented.
    Negative control run for the zero-width border.
  - Pending: Windows clipboard check (TODO); Codex recheck.

- **Slice 4b (0.3.101), 2026-11-04.**
  - MultiSelect and QuickPick popup item rectangles (option C of
    [todo-repairs-r14-popups.md](todo-repairs-r14-popups.md), revision 2,
    with the three points of design-s4a-r2.md).
  - Implemented by an agent in an isolated worktree from f7f5465, and
    cherry-picked onto the branch after slices 6 and the 4/5 recheck fixes.
  - The merge adapted one test to `QuickPick::searchable` (default true),
    and the note's section 9 records the deviations.
  - New public API: `ListItems::semantic_item` and
    `VirtualList::report_items`; new parts MultiSelect `control`/`options`
    and QuickPick `options`.
  - The TODO entry is resolved; a new one proposes exposing the
    child-request adoption record to external collections.
  - Evidence: the agent ran the full workspace gate in its worktree, with
    16 negative controls; the branch gate runs after the merge.
  - Codex implementation review ([impl-s4b](todo-repairs-review/impl-s4b.md)):
    "needs fixes", no blocker. All points are fixed:
    - `children_adopted` is public on `UpdateContext` and `ArrangeContext`,
      with a doctest of an external component retrying (its TODO entry is
      removed);
    - the disabled-result docs are corrected, and that TODO entry is removed;
    - the picker search test asserts exact hits;
    - the "computes nothing" wording is fixed.
  - The combined recheck of the slice 4/5/6 fixes
    ([impl-s456-r3](todo-repairs-review/impl-s456-r3.md)) asked for an
    explicit mounted-list order with targets, light-palette command checks
    and profiling, all added.
  - The final recheck ([impl-final](todo-repairs-review/impl-final.md)) found
    no production defect. It resolved the extension-parity and documentation
    points; the picker hit now also checks its target against the mounted
    row. The coverage it still asked for was added at the maintainer's
    request (8f763d8):
    - a search held across a picker query and a session change is stale,
      and a new search sees only the current results;
    - a list nested in an item-reporting component keeps its explicit order
      and targets across one-unit steps;
    - disabled anchors mute an icon caption, a chip keeps its container and
      mutes its label, and overridden Outline and Muted roles reach what
      they draw and report;
    - a paint that fails at the focus ring keeps both anchors' previous
      record. A test-only stroke fault in `shapes.rs` forces the failure:
      the test has no public way to fail a paint deterministically after
      the frame. Negative controls (the record set before the focus ring)
      fail for both anchors.
  - The recheck of those tests
    ([impl-final-r2](todo-repairs-review/impl-final-r2.md)) found no
    production defect and accepted the test-only hook, but called some
    assertions weaker than described. Changes made:
    - the picker search asserts its complete ordered result with roles,
      paths, visibility and targets;
    - the stale test checks every hit of the new searches, including that
      each row hit belongs to the current list (from the last recheck,
      [impl-final-r3](todo-repairs-review/impl-final-r3.md), "ready with
      nits");
    - the icon and the chip label are identified against a reference
      control that differs only in that part (masks are shared by content
      within a scene), not by mask shape;
    - the trigger's reported foreground is checked as well;
    - the fault is a guard that resets on drop. The negative control was
      rerun.
  - Pending: native checks.

## Decisions

The maintainer accepted every recommendation on 2026-11-04 (D6 earlier the
same day).

- **D1 (R01):** delete `NOW.md`.
- **D2 (R07):** document the implementor-list churn in snapshots instead of
  rewriting the cases.
- **D3 (R08):** the rustdoc states the `DrawError` rule, and a test covers all
  19 types.
- **D4 (R09):** keep `#![doc(test(attr(deny(unused_imports))))]` permanently
  in `pert-ui-runtime` and `pert-ui`, with the testing chapter's one marked
  exception. This is the lint configuration change AGENTS.md asks to discuss,
  and it is now agreed.
- **D5 (R21):** `miniz_oxide` from the workspace dependencies.
- **D6 (R22):** TEXT-01 only gets a design plan here, is not implemented now,
  and stays open.
- **D7 (R23):** include the Linux-side `clipboard-win` update, with the
  Windows check left open.
- **D8:** the per-slice version bump counts as routine, so it alone does not
  trigger the dependency row. Packaging and the consumer check run in slice 6
  and at the end of the plan.
- **D9 (R14):** MultiSelect and QuickPick item rectangles get a design
  checkpoint (slice 4a) before implementation. If no acceptable design is
  found, leaving them in TODO.md is decided at that checkpoint. It is not
  pre-authorized. **Checkpoint outcome, 2026-11-04:** the maintainer chose
  option C of [todo-repairs-r14-popups.md](todo-repairs-r14-popups.md), with
  Codex's positions on its open questions; the note is revised and rechecked
  before implementation (slice 4b).
- **D10 (R22), 2026-11-04:** the maintainer stopped iterating on the
  document-editor design after four reviews; it is committed as direction
  with open items OD1–OD6.

## Order and dependencies

Slices 1 → 6 in order. R11 lands before R10. Slices 3–6 change `api/`,
CHANGELOG and TODO.md, so they run sequentially. R17 and R20 share slice 6
because both touch Button paint. Slice 6's paint changes are rechecked
against slice 4's item rectangles (Select/MultiSelect). R18 (slice 5) changes
collection rows that slice 3's validation and slice 4's rectangles use, so
slice 5 re-runs their tests. Slice 7 can be written in parallel. Slice 8
follows slice 6.

## Review record

**Revision 1 → 2, Codex r1** ([prompt](todo-repairs-review/prompt-r1.md),
[report](todo-repairs-review/codex-r1.md)). Verdict "needs rework". The
author verified every point against the code and adopted all of them:
- **Blocker R18:** `EachItem` is a sealed trait with a consuming conversion.
  Replaced by a concrete `RowItem`.
- **Scope:** NATIVE-01 has Linux (KDE, Ubuntu) work, so it is now
  partly included. NATIVE-04 has no Windows part. R23 is included on the
  Linux side.
- **R22:** marked as an explicit scope exception. The baseline is measured
  within the current limit.
- **R19:** `detachable` is a user restriction and failure returns to Docked.
  The spec now gives event-phase scheduling and reuses the
  `detaching`/`detached` tracking, with a broader test set.
- **R15:** component-only jump, the index as a position rather than an
  identity, resumable bounded scanning, and protection at component level.
- **R14:** the actual reporters (Collapsible has none; the MultiSelect and
  QuickPick popup rows are child lists), stricter tests.
- **R12:** a sweep for both axes, a counted threshold, all participants in
  `components`.
- **R13:** a public slot hint mechanism that reuses `ExceedsFixedHeight`.
- **R21/D5:** `miniz_oxide`, an independent decoder, stored-layout tests
  migrated, unmeasured claims removed.
- **Gates:** slice 3 runs the API listing, the dependency items moved into
  slice 6 with the dependency row, and the version bumps follow D8.
- **R10:** `reconcile` must adopt `renaming`.
- **R17:** complete replacement of the retained metadata and freshness rules.
- **R20:** Button shares the helper, with anchor-specific assertions.
- **R08:** the list is at the type, and `PathError` and normalization are
  tested.
- **Slice 8:** concrete acceptance per probe and backend.
- **Nits:** R16 names fixed, R05 runs the built binary, R07 contract wording.

**Revision 2 → 3, Codex r2** ([prompt](todo-repairs-review/prompt-r2.md),
[report](todo-repairs-review/codex-r2.md)). Verdict "needs rework". It
resolved 17 of the r1 findings and partly resolved 6. The author verified the
new points (`Element` has no public key setter; `ValidationHints::fixed_height`
is a bool; `VirtualList` does not enumerate rows as items; the cursor has
`pending`, `RowScan` and `visited`) and adopted them all:
- **Blocker R18:** `RowItem` wraps a complete `Element` captured while `C` is
  known. Its key conversion `RowItem::into_element` is public, so external
  collections can reproduce it. Tests for same-type redescription and
  children were added.
- **R13:** a defined public operation `ArrangeContext::place_fixed_slot`, with
  owner, units, development-only measurement through `ArrangeContext::measure`,
  separate retained evidence, failed-layout rules, and the contract update.
- **R15:** an `ItemScan` cursor state with a one-time snapshot, a per-item
  charge, the ordering, the `done` rule and snapshot mutation semantics.
- **R14:** MultiSelect/QuickPick moved to a design checkpoint (slice 4a, D9).
  Failed-layout absence is qualified.
- **R12:** the cost promise is limited to rows and columns, with grid and
  overlap worst cases documented.
- **Gates:** D8 is marked provisional, with the full row until it is
  accepted. Slice 1 uses the cross-package row. Slice 6 names the complete
  row, including the consumer check, and uses separate commits.
- **D6** accepted by the maintainer.

**Revision 3 → 4, Codex r3** ([prompt](todo-repairs-review/prompt-r3.md),
[report](todo-repairs-review/codex-r3.md)). Verdict "needs rework", with no
blocker. R18, R12, the verification point and the R14 checkpoint were
resolved, and slices 1–2 were judged ready. The author verified the remaining
points (`measure_node` promotes the latest measurement at `tree.rs:1839`;
`RowScan` takes priority over `pending`; `development` is compiled under
`cfg(debug_assertions)`) and adopted them:
- **R13 isolation:** the parent-box check reads the measurement at the
  child's placed constraints instead of the latest one, with scoped
  preservation as the fallback, plus cache-hit, cache-miss and nested-wrapper
  tests.
- **R13 retention:** slot evidence is kept with the parent's placement
  record, cleared by a fresh ordinary placement, kept by a retained
  placement, and staged until the layout succeeds.
- **R15 order:** the existing order is kept (items, then unmounted rows, then
  children), so one slot of each scan state suffices. The state is captured
  before yielding, and there is a nested-collection test.
- **R15 cost:** the one-time inspection and snapshot are charged to the visit
  and documented as not divided by the budget. Strings are moved, and an
  inspection counter is tested.
- **Nits:** `cfg(debug_assertions)` is named; the parts-checkpoint link is
  fixed.
- **D9:** its fallback is a scope reduction that needs acceptance at the
  checkpoint.

**Revision 4 → 5, Codex r4** ([prompt](todo-repairs-review/prompt-r4.md),
[report](todo-repairs-review/codex-r4.md)). Verdict "needs rework", with no
blocker. Five of the six r3 points were resolved, and slices 1–2 remain
ready. The author verified the remaining points (`RETAINED = 4` with
least-recently-used eviction; `Placement` is a `(Rect, Option<Rect>)` tuple
on the child's node; the geometry commit reads `measured_height()`) and
adopted them:
- **R13 isolation:** the cache lookup is dropped, since a probe can evict the
  entry. The spec now uses independent debug-only validation evidence per
  node, which the probe-depth guard leaves untouched during probes and which
  the parent-box check, geometry commit and change drain read. There is an
  eviction regression test.
- **R13 storage:** the `Placement` tuple becomes an internal struct with an
  optional slot field: parent-authored, stored on the child, staged and
  committed with placements.
- **R15:** the counter promise is narrowed to resuming an `ItemScan`; the
  ancestor naming fallback is excluded; `content_hit` is restructured so the
  snapshot is taken before the visit returns.

**Revision 5 → 6, Codex r5** ([prompt](todo-repairs-review/prompt-r5.md),
[report](todo-repairs-review/codex-r5.md)). Verdict "needs rework", with no
blocker. The R13 placement storage and the R15 counter were resolved. The
isolated debug-only probe still missed readers: update-time height
(`runtime.rs:6243`, `:955`), forwarded constraints (`layout.rs:1225`,
`runtime.rs:961`) and change notification (`tree.rs:1887`). It also left
first-measured descendants without evidence. The author verified these and
**replaced the approach**: `place_fixed_slot` now makes an ordinary
measurement in every build, so the child's latest measurement is legitimate
evidence that every existing reader already uses. The probe, its isolation
and the second evidence store are gone. The cost (one cached measurement per
placed row) is documented and measured on two headless examples.

**Revision 6 → 7, Codex r6** ([prompt](todo-repairs-review/prompt-r6.md),
[report](todo-repairs-review/codex-r6.md)). Verdict "ready with changes". The
ordinary-measurement approach resolves the r5 findings, with no relayout loop
and no conflict with the retention rule. Adopted:
- the cost is now stated as one additional top-level request per
  `place_fixed_slot` call (a hit while it is cached, recursive on a miss, none
  for a retained arrangement), with request, hit and measure counters over
  fixed, estimated, vertical and horizontal lists;
- the global debug/release parity claim is removed;
- the implementation notes on units and on slot-only changes are added;
- a horizontal list's fixed axis is recorded as a slice decision.
