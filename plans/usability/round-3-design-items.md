# Round 3 checkpoint: pointer-testable menu rows and tab titles

Status: proposed, 2026-10-28; implementation and execution evidence pending.
This checkpoint is driven by the retained head submission
[r3-6-head-claude-1](runs/r3-6-head-claude-1/FRICTION.md), its recorded
`ItemWithoutBounds` failures for menus and tabs, and TODO.md's existing
"built-in item reporters without rectangles" finding. It does not classify
all of that submission's other leads; independent triage is still underway.
The frozen experimental head stays at 32dd9c4 throughout the blind runs.

## Responsibility and scope

Give `Tabs` title items and the shared option list used by menus and `Select`
their real pointer geometry through the existing `SemanticItem::local_bounds`
contract. The public `Driver::click_item` remains the pointer-input route;
this retires application-computed coordinates and keyboard navigation as
workarounds for pointer testing of these particular items. Keyboard navigation
remains independently supported. No new constructor, controller, semantic
action, callback timing or selection ownership is introduced.

This resolves the tab and Select-list portions of the existing TODO; other
families in that entry remain separately tracked unless this round reaches
them. Menu rows share `OptionList` with Select, so the shared responsibility
is fixed together rather than adding a menu-only coordinate path.

## Geometry contract

- Use the rectangles painting and hit testing already use. Inspection reads
  retained geometry only and does not measure, arrange, publish or request
  work. All values are component-local logical pixels.
- Tabs use the retained `Header::rect`, matching the current tab's stable key
  and mounted `WidgetId` so stale cached geometry cannot be assigned to a
  recreated tab. Intersect it with `header_space()` and the title-strip
  height before publishing it: overflow arrows have their own hit regions
  and must never be reported as part of a title. A title entirely scrolled
  out of the header viewport has no rectangle. Preserve item order and all
  existing selection, move and detach semantics.
- `OptionList` uses `RowGeometry::top(index) - offset`, its measured row
  height, and its most recent arranged width and height. Retain the arranged
  `Size` in place of the current height-only cache. Clip each row to that
  viewport, excluding separator bands; a separator is not an item. Partly
  visible rows expose only their visible part. Disabled rows may have
  geometry, but the real pointer path continues to refuse their activation.
- Before the first arrangement no usable local rectangle exists. The runtime
  remains responsible for moving/scaling local rectangles into the committed
  window geometry, ancestor clipping, hidden/unpresented components, failed
  layout suppression, and detach/redock geometry validity, as contracts.md
  already states. Components must not fill window `bounds` directly.
- Geometry comes from current retained presentation, not a synthesized
  preferred size. Cached headers must not retarget removed/recreated tabs;
  zero-area intersections publish no rectangle.

The snapshot shape and field meanings do not change: item bounds are already
optional under inspection schema 16, and their meaning stays the same. No
schema bump or persisted workspace migration is proposed. Public signatures
and the API listing should remain unchanged; the listing gate still runs.

## Documentation and migration

Document each affected component's inspection items and pointer bounds, plus
its exact address route: tabs are addressed by tab key while titles are items
of the strip; menus report their rows on the option-list component; Select's
anchor and open list are distinct. Update `Driver::click_item`'s built-in
coverage statement and the testing guide without promising automatic
scroll-to-item behavior. No repository consumer needs a compatibility shim.
Add a complete public-API example that opens a menu and clicks a row, and a
complete example selecting a tab through its title item; run them as doctests.
Keep additional inspection/address documentation findings separately visible
in the final triage rather than silently declaring them resolved by geometry.

## Evidence required before closure

After the timed blind queues finish, establish the missing-capability failures
against the frozen head and add focused regression tests on the repaired code:

- Open File through pointer input and click a command row; assert the
  command ran once and the menu closed. Cover a separator, a disabled row,
  and a submenu branch's actual expansion behavior.
- Click an unselected tab title and assert selection/content visibility and
  its settled callback; exercise title overflow, a partly visible title and
  an offscreen title, showing that arrow regions are excluded.
- Reorder/recreate a tab under a reused key and verify its item bounds never
  name the old incarnation. Cover dynamic title width and view scale or
  ancestor clipping using the public inspection/input path.
- Open a Select list and click a visible option, including a scrolled or
  clipped row, checking the bound value and closure; separators in menus
  must not consume an item index or become clickable item rectangles.
- Confirm bounds are absent before first layout and while hidden, and that
  repeated inspection of a settled tree schedules no updates or repaint.
  Reuse existing failed-layout and detach/redock geometry tests where they
  already cover the unchanged runtime transformation rather than duplicating
  that machinery in every widget.

Run the workspace gate and API listing gate, focused contract tests,
warning-free rustdoc and new doctests. Record native evidence for affected
menu/tab/Select pointer paths where available; headless evidence does not
claim Windows/macOS support. Obtain fresh Claude design review before
implementation and implementation review after the gates. Reviewers author
no fixes and do not delegate.
