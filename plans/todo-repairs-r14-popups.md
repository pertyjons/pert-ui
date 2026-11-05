# R14 checkpoint: MultiSelect and QuickPick popup item rectangles

Status: **revision 2, implemented in slice 4b**, 2026-11-04. Written for
slice 4a of [todo-repairs.md](todo-repairs.md) (R14, D9). Sections 1–8 are
the design as reviewed; line numbers are from commit 7eb3e6b, which includes
R15's item search. Section 9 records how slice 4b implemented it, the
remaining points of the second review, and where the implementation differs.

Record:
- **Revision 1:** the first draft.
- **Codex review** ([design-s4a.md](todo-repairs-review/design-s4a.md)):
  "needs rework". Option C was judged the right direction, with gaps in the
  contract: keyed placement validity, the R15 interaction, freshness of the
  semantic hook, QuickPick's wider inspection changes, and the wording of the
  alternatives and documentation.
- **Maintainer decisions, 2026-11-04:**
  - choose option C;
  - expose MultiSelect's trigger as `control` and document keyboard opening;
  - popup items use `checked` = `shown()` and `selected` = effective
    highlight, while the trigger keeps its committed-selection meaning;
  - distinct semantic reporters give distinct content-search hits, with no
    label-based de-duplication.
- **Revision 2:** addresses every review finding and records these decisions.
- **Codex recheck** ([design-s4a-r2.md](todo-repairs-review/design-s4a-r2.md)):
  "ready with changes": failed child reconciliation needs a validity and
  retry rule, the accepted external-list overlap needs a test, and the
  `ListItems::text` and closed-trigger documentation needs its exceptions.
  Slice 4b adopted all three (section 9).

## Why the reporters cannot give rectangles today

- MultiSelect's trigger reports one item per option
  (`multi_select.rs:1264-1269`). The popup's rows are `ChoiceRow` components
  inside a structural `VirtualList` (`multi_select.rs:880-886`). The popup is a
  `Popover` surface (`multi_select.rs:1708-1713`).
- QuickPick's `PickContent` reports the `ListBox` (`quick_pick.rs:1478-1506`,
  documented at `:830-834`). Its rows are `PickRow` components in a structural
  `VirtualList` keyed `results-{serial}` (`:1366-1377`). The serial changes
  whenever the query or the session generation changes (`:1423-1436`).
- `VirtualList` deliberately reports no items (`virtual_list.rs:2735-2740`).
  Its axis, offset and slots are private (`virtual_list.rs:562-567`,
  `:1604-1628`).
- The runtime turns `local_bounds` into window bounds using the *reporter's*
  committed origin, scale and clip, and only while the reporter is presented
  and has no uncommitted arrangement (`runtime/inspect.rs:194-205`,
  `:549-570`; `docs/contracts.md:1620-1634`).

## 1. The facility

Options considered:

- **A. The reporter computes rectangles from the list's public geometry.**
  Rejected for the current API. `Component::inspect(&self)` gets no context
  (`runtime.rs:2247`). The reporter has none of the list's retained placement
  data: the list's offset, line extents and slots are private, and
  `ArrangeContext` reaches only direct children (`layout.rs:1131`).
- **B. `ListController` exposes placed-row rectangles by key.** Not chosen,
  as a scope decision. Such an observation is possible in principle. But no
  public contract says which committed placement and which coordinate space
  it would report. Naive translation through the reporter's own placement of
  the list is wrong under a `child_transform` (`runtime.rs:1416`). A snapshot
  written during `arrange` could belong to a pass that later fails, while the
  runtime suppresses bounds only for the reporter's node
  (`inspect.rs:196-198`). It would also add geometry observation to a
  keyed-reveal handle (`virtual_list.rs:178-193`): an extra responsibility,
  not in itself a rule violation.
- **C. The list reports the items, as Select's ListBox does, as a
  `VirtualList` opt-in.** Chosen. The list owns the geometry, because it
  placed each row (`virtual_list.rs:2170-2183`). Being the reporter, it falls
  under the runtime's failed-layout suppression. It is a sibling of the
  already exposed `search` part, so it can be exposed as `options`, Select's
  vocabulary (`select.rs:1808-1811`, `:1474-1482`).
- **D. The runtime resolves an item's row.** Viable, but a larger design. An
  item would name a descendant row, and the runtime would fill the bounds
  from that row's committed geometry. Its key lookup, identity validation and
  failed-layout rules all need design. In today's tree, the reporters that
  would use it (`Panel`, `PickContent`) also contain `search`. Exposing them
  as parts would therefore stop part lookup at them, which changes the
  documented `search` route ([parts checkpoint](usability/round-3-design-parts.md),
  "Lookup scope"). That is a consequence of this tree arrangement, not of
  runtime row resolution as such.

### Proposed public items

**`ListItems::semantic_item(&self, index: usize) -> Option<SemanticItem>`.**
The default is `None`. It describes row `index` of this source for a list that
reports items. Contract:
- *Permitted reads:* data the source holds as plain values, and plain cells
  that the owner writes only in its own update. No signal or binding reads.
  Inspection never overlaps the owner's update, so these reads cannot fail.
- *Forbidden:* building rows, filtering, ranking, measuring, revealing,
  writing or publishing signals, scheduling, polling and I/O.
- *Field freshness:*
  - Membership, order, key, label, the option's own enabled flag and the
    shortcut come from the source snapshot the list adopted
    (`virtual_list.rs:1959-1991`).
  - Checked, highlight and permission-derived disabled state are as of the
    owner's last successful update, retained in plain cells. This is how
    `ChoiceRow` retains its fields (`multi_select.rs:618-640`).
  - Between a signal write and the next flush, the items show the previous
    state, as the rows do. A failed owner update keeps the last valid state.
  - No field is invented from an unavailable read, because the hook makes no
    fallible read. QuickPick's current `highlight.get().ok()`
    (`quick_pick.rs:1481`) is therefore replaced by a highlight retained in
    `PickContent::update`.
- *`None` below `len`:* reported as an unnamed item with no state and no
  actions, so an item's index always equals its row position.

**`VirtualList::report_items(role: SemanticRole) -> Self`.**
- The list reports `role` in place of `List`, with one item per row of its
  snapshot, in index order. The value and the `Scroll`/`Select` actions are
  unchanged.
- Each description replaces or removes the setting. It is adopted at the top
  of `reconcile`, before the `owned_callbacks` early return
  (`virtual_list.rs:1905-1913`). It publishes nothing and needs no
  invalidation, because the placement records below are kept whether or not
  the opt-in is set.
- *Cost:* one `semantic_item` call per row on each inspection of the list.
  That includes content search, which inspects every component it visits
  once.
- The `ListItems` promise that the list "never iterates every row"
  (`virtual_list.rs:25-26`) gets this exception, and the comment at
  `:2735-2737` is reworded.

**Keyed placement records, kept by the list:**
- **Content.** One record per placed child: its row key, its slot, and the
  viewport size of that arrangement. Each `arrange` replaces the whole set
  (`:2151-2183`).
- **Item rectangle.** Item `i` gets `local_bounds` only when the snapshot's
  key at `i` has a record. The rectangle is that slot intersected with the
  record's viewport. It is never recomputed from the live offset. A scroll
  moves rows at the layout it requests (`:1656-1663`), as it moves their
  border boxes.
- **Validity.**
  - *Source replacement:* when `update` sees a replaced source (`:1959`), it
    clears all records before building rows. A failed build after the
    snapshot is adopted (`:1989`, `:2063-2069`) therefore leaves no
    rectangles. So does reordering, or an insert or removal, until the next
    successful arrangement.
  - *Same-source update:* a `request_children` that changes the mounted set
    (`:2070`) drops the records of keys that leave the set or enter it.
    Only a key that stays mounted, and so keeps its retained component by
    keyed reconciliation, keeps its record. A newly mounted row, or a row
    that left and came back, never inherits an earlier rectangle.
  - *Unfinished construction:* no rectangle is reported while
    `rows_current` is false (`:1960`, `:2088`).
  - *Failed layout:* the runtime suppresses the items after a failed layout
    that arranged the list, and the list is re-arranged at the next one.
- **Recovery.** The next successful update and arrangement restore the
  rectangles.

An external composite uses these items, `Element::expose_part` and
`Element::with_label` (`layout.rs:489`) exactly as the built-ins do; there is
no `pub(crate)` path. The sentence in `diagnostics.rs:378-379` ("real child
components report none") gains the opt-in exception. The new route,
`exposed_part` then `click_item`, replaces the guessed internal keys
(`anchor`, `content`) that today's tests use
(`tests/collections/multi_select.rs:178`).

## 2. Where the items live; open-only rectangles

- **MultiSelect trigger:** exposed as `control` at `multi_select.rs:1678`.
  It keeps its ComboBox items, which describe the committed selection
  (`:1076`, `:1266`), and never gives them rectangles, closed or open. This
  matches Select's anchor (`select.rs:1409-1414`) and the parts checkpoint.
  The trigger cannot express popup geometry anyway. Its items are moved by
  its own origin and cut to its own ancestor clip, while the popup is a
  separately placed surface. The rustdoc says that trigger indices do not map
  to popup indices, and that a test opens the popup from the keyboard
  (focus, then Enter or Down). The middle of the trigger can fall on a chip.
- **MultiSelect popup:** the list gets `report_items(ListBox)` and
  `.expose_part("options")` at `:886`. `Items` (`:523`) implements
  `semantic_item`.
- **QuickPick:** the ListBox moves from `PickContent` to the results list,
  which gets `report_items(ListBox)`, `with_label(list_name)` and
  `.expose_part("options")` at `quick_pick.rs:1373-1377`. `PickItems`
  (`:1517`) implements `semantic_item`. `PickContent` keeps its `query`,
  `results` and `message` properties, and its semantics go. `CommandPalette`
  forwards `options` inside `surface`, as it forwards `search`.
- **Intentional QuickPick inspection changes** (documented in rustdoc and the
  CHANGELOG):
  - With no results there is no ListBox. Today `PickContent` reports a named,
    empty ListBox (`:1491-1506`); the empty-state label now replaces the
    list (`:1358-1364`).
  - The ListBox gains the list's numeric row-count value and its
    overflow-dependent `Scroll` action (`virtual_list.rs:2754-2762`).
  - The reporter's identity changes on every query edit and every new
    session generation, even when the ranked results are identical.
- **Open only:** the popups are hidden while closed (`multi_select.rs:1708`,
  `quick_pick.rs:1133-1135`), and a hidden component's items have no bounds
  (`diagnostics.rs:442-450`).
  - A closed list with items is refused by `click_item` through reveal
    (`testing.rs:940-943`).
  - An empty list returns `NoItem` before any reveal, because `click_item`
    checks that the item exists first (`testing.rs:1019`). This covers
    MultiSelect's "No matches" list, which stays mounted with zero items.
  - QuickPick without results has no `options` part, so the lookup returns
    `ExposedPartNotFound`.
  - The `click_item` rustdoc states these cases.

## 3. Filtered positions and unmounted rows

The list's item `i` is the snapshot's row `i`:
- MultiSelect: `Filtered.rows[i]` (`multi_select.rs:460-480`), whose key is
  the value's stable hash key.
- QuickPick: `session.items[ranked[i]]` (`quick_pick.rs:1517-1559`), keyed by
  the `PickItem` key.

The trigger's items follow source-option order, so a test finds an option by
its name and never carries an index from the trigger to the popup. Rows that
are filtered out are not items. The following rows have no rectangle:
- rows outside the mounted window;
- rows kept mounted outside the viewport by focus, capture or
  `keep_mounted` (`virtual_list.rs:417-421`), because their slot misses the
  record's viewport;
- rows without a valid record (section 1).

A partly visible row reports only its visible part. `click_item` does not
scroll; Up/Down in the search field reveals the highlight
(`multi_select.rs:447`), and the wheel also scrolls the list.

## 4. Labels, state, shortcuts, actions; disabled activation

- **MultiSelect item:**
  - name: the choice label;
  - `checked`: whether the value is in `shown()` (`multi_select.rs:266-272`),
    which under `OnClose` is the draft the row draws;
  - `selected`: the effective highlight;
  - `disabled`: the dropdown is denied or the option is disabled;
  - action: `Toggle`.

  `Panel::update` retains the shown set, the highlighted key and the
  permission in plain cells for the hook to read.
- **QuickPick item:** unchanged from `quick_pick.rs:1491-1500`: the label,
  `selected` = the effective highlight, `disabled`, the shortcut and
  `Activate`. The highlight is retained in `PickContent::update`.
- `SemanticItem::action` drops actions on a disabled row
  (`diagnostics.rs:604-611`). The rows keep their own semantics (`ChoiceRow`
  as `Checkbox`, `multi_select.rs:771`; `PickRow` as `Button`).

Disabled rows keep their rectangles. `click_item` is pointer input
(`testing.rs:927-928`), and its rustdoc names this exception: the row
decides the outcome.
- A MultiSelect press on a disabled option arms nothing
  (`multi_select.rs:708-716`). No write, no `on_change`, and the popup stays
  open.
- QuickPick passes the disabled item to the choose callback
  (`quick_pick.rs:1714-1724`, documented at `:806-808`), even though the
  item reports no action.

## 5. QuickPick re-keying and stale geometry

A query edit or a new session makes a new list incarnation. The old one is
unmounted by the reconciliation in the next flush, not by the query write
itself. A `WidgetId` held from before that flush then fails with
`Unmounted`; it never yields old rectangles. Look the part up again after each
query or session change. A new list has no rectangles before its first
successful arrangement (section 1). Items and records always describe the
snapshot of the same incarnation, whose source is fixed per key
(`quick_pick.rs:1371`). A message-only redescription and a translated
`list_name` keep the key, the retained binding (`virtual_list.rs:512-515`)
and the identity; the label updates in place. The MultiSelect list is not
re-keyed: new rows arrive through its stable binding (`multi_select.rs:901-905`),
and a source replacement clears its records until the next arrangement.

## 6. Content search (R15), decided

R15 emits one `ContentField::Item` hit per matching item name, targeting the
reporting component (`runtime/content_search.rs:149-169`). It then visits the
mounted children (`:253-255`). An item index is a position, not an identity
(`content_search.rs:172-178`).

**Policy (maintainer decision):** search stays exhaustive. Distinct semantic
reporters give distinct hits, even with equal labels. There is no label-based
de-duplication: different options may share a label, and the hits have
different targets and jump behavior. Unique logical-option results would
need a separate public association contract, which is out of scope. Rows
stay searchable; `Element::with_searchable` is not used to hide them.

The overlaps this accepts, all documented in the MultiSelect, QuickPick and
`ListItems` rustdoc:
- **MultiSelect:** a trigger item hit (also while closed, for
  hidden-inclusive searches), a popup item hit, and the mounted `ChoiceRow`'s
  `Name` hit, all while open.
- **QuickPick:** a list item hit and the mounted `PickRow`'s `Name` hit.
  Moving the report removes `PickContent`'s hit.
- **External opt-in lists that implement `ListItems::text`:** their item hits
  can also overlap the `Row` hits of unmounted rows. Built-ins implement no
  `text`.

A query edit in QuickPick re-keys the list, so a search continued across it
returns `ContentSearchError::Stale` (`content_search.rs:246-250`).

## 7. API, tests, gate

**Public changes (proposals):**
- `ListItems::semantic_item` and `VirtualList::report_items`.
- New parts: MultiSelect `control` and `options`, QuickPick `options`.
- QuickPick's ListBox moves, with the changes listed in section 2.
- No inspection field is added (the schema stays at 18,
  `diagnostics.rs:1231`). No workspace path changes.

**Documentation:** update `SemanticItem` (`diagnostics.rs:367-379`),
`Driver::click_item` (`testing.rs:913-945`: built-in coverage, the
disabled-QuickPick exception, empty lists), `ListItems` (`:25-26`),
`VirtualList`, the MultiSelect inspection paragraph and its keyboard opening
(`multi_select.rs:1371-1376`), and QuickPick (`quick_pick.rs:830-834`). Each
new item gets a doctest. The CHANGELOG names every listed item with its
replacement usage. The TODO.md entry (`TODO.md:101-112`) is removed.

**Tests** (public API only; `tests/surfaces/pointer_items.rs` and the
collections binary):
- *Pointer:*
  - `click_item` on a non-first option: `EachToggle` writes and reports once;
    `OnClose` edits the draft. For QuickPick, it runs the callback with the
    right key.
  - Disabled rows, per section 4.
  - After filtering and after scrolling, a later row's bounds equal the row
    component's bounds cut to its clip.
  - A partly visible row; a clipped or unmounted row (`ItemWithoutBounds`);
    a pinned row outside the viewport.
  - Trigger items never have bounds.
  - A closed non-empty list is refused by reveal; an empty list gives
    `NoItem`.
  - A non-unit view transform.
- *Placement records:*
  - reorder, insert and remove, and source replacement: no inherited
    rectangle and none before the next arrangement;
  - a newly mounted row, and a row that leaves and returns;
  - a failed row build after snapshot adoption (a source whose `build`
    returns `None`), then recovery;
  - a failed arrangement, then recovery.
- *Semantic hook:*
  - permission redescription, denied and then restored;
  - an external write to the bound value;
  - a highlight change without source replacement, where no row is rebuilt;
  - `OnClose` commit and cancel, with the trigger's committed meaning
    unchanged until commit;
  - setting, replacing and removing the opt-in, including through an owner
    with `owned_callbacks`;
  - fixed, estimated, horizontal and multi-lane lists;
  - an external composite;
  - a default list still reports no items, and repeated inspection
    schedules nothing.
- *QuickPick identity:*
  - a query change with the same results, and a new session: new identity,
    old id `Unmounted` after the flush;
  - a message-only redescription and a translated `list_name`: same
    identity, updated name;
  - no results gives no ListBox and no part;
  - the value and `Scroll` action;
  - the `CommandPalette` route.
- *Content search:*
  - open and closed popups, with and without hidden inclusion;
  - filtered and offscreen items;
  - duplicate labels as distinct hits;
  - mounted row descendants beside item hits;
  - `Stale` after QuickPick re-keying mid-search.

**Gate:** the public-API row: the workspace gate, the API listing gate
(`api/generate.sh`), focused contract tests, doctests and warning-free
rustdoc. Then an independent Codex implementation review. This is headless
evidence only; no native evidence is claimed.

## 8. Fallback

If C fails in implementation, D is the alternative. It needs its own design
of lookup, identity and failed-layout rules. If neither is accepted:
- expose the lists as `options` parts anyway;
- tests click rows as components with `Driver::click`, found through their
  named `Checkbox`/`Button` semantics and `Driver::address_children`;
- `click_item` keeps refusing these items with `ItemWithoutBounds`, and the
  TODO.md entry stays, narrowed to that reason.

Under D9, this fallback needs the maintainer's explicit acceptance.

## 9. Implementation (slice 4b)

Implemented as designed, with the public items named in section 1:
`ListItems::semantic_item` and `VirtualList::report_items`, the MultiSelect
`control` and `options` parts and the QuickPick `options` part.

**Requested and adopted rows (second review, point 1).** The list keeps the
rows it requested (`mounted`) apart from the children the runtime adopted:
- Records are made only in `arrange`, which places the children the runtime
  actually holds, so a requested row that was not adopted never has one.
- A request drops the records of keys outside the new requested rows; a
  replaced source drops all of them. After a rejected request the surviving
  rows (requested before and again, and still mounted) keep the rectangles
  of their last arrangement, which match their committed boxes.
- A child that is mounted but no longer requested, left by a rejected
  request, is placed with an empty clip, so it shows nothing and the layout
  does not fail (before this change the list failed the layout with
  `InvalidChild` on every later flush until something updated it), and the
  list requests an update.
- Whether the runtime applied the list's latest request is the runtime's
  own record (the one content search reads to skip obsolete row children),
  read through crate-internal accessors on `UpdateContext` and
  `ArrangeContext`; comparing child keys would miss a rejected description
  of rows with unchanged keys, as after a replaced source with the same
  keys. `update` requests the rows again while that record says the last
  request was not applied, also when the source and the wanted rows did not
  change, and `arrange` requests that update. So once the rows can be
  admitted, the next flush recovers without a new source. While they are
  still rejected, each flush retries once and returns the rejection.
- The record is not public, so an external collection can observe a
  rejection only by comparing its children's keys; TODO.md records exposing
  it.
- After a replaced source whose rows were not adopted, no row has a
  rectangle, since the mounted rows still show the previous source; the
  first arrangement after a successful request restores them.

Tests (`tests/collections/list_items.rs`) cover a rejected admission with
every row surviving and with a row dropped, inspected right after the
failure and recovered without a new source, a retry that fails again, a
replaced source whose rows were rejected, and a rejected description of the
same rows. Negative controls fail them.

**The external overlap (point 2).** An external source implementing both
`semantic_item` and `text` gives an unmounted row an `Item` hit on the list
and a `Row` hit by key; a test asserts both hits' fields and targets.

**Documentation (point 3).** `ListItems::text` names the opt-in exception
to "found only while mounted" and the accepted double hit. The MultiSelect
rustdoc separates the visible trigger's item hits, found while closed, from
the hidden popup list and rows, found only by a hidden-inclusive search.

**Deviations from sections 1–8:**
- **QuickPick search.** The implementation was written while slice 4's
  first review fix excluded every picker surface from content search. The
  recheck of that fix replaced it with the opt-in `QuickPick::searchable`,
  true by default, and the merge into `todo-repairs` adapted the test: a
  searchable picker gives section 6's overlap (the result list's item hit
  beside the row's hit), and one with `searchable(false)`, as the gallery's
  search picker, is never found. A `Stale`-after-re-keying test through a
  picker was not added; the general `Stale` rule for a collection unmounted
  mid-search is unchanged and tested elsewhere.
- **View transform.** The non-unit view transform is tested on a generic
  list under a scaling `child_transform`, not through the popups, whose
  surfaces are placed by the runtime's surface rules.
- **Pinned rows.** A row kept mounted out of view is tested through
  `VirtualList::keep_mounted`, since the popup rows take neither focus nor a
  lasting capture.
- The MultiSelect list is not named (Select's `options` list is not
  either); QuickPick's is named by `list_name` through `Element::with_label`.
- The repository's gallery interaction check still reads the picker
  content's `query` and `results` properties through the internal `content`
  key: those properties are not on the `options` part, so no part replaces
  that lookup.
