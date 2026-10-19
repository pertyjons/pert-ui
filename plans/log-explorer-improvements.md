# Plan: improvements found by a log-explorer probe

Status: proposed 2026-10-19 after an independent Codex review of the
findings; being implemented on branch `log-explorer-improvements` (slice 0
documentation only, slice 2 in 0.3.24 while slice 1's checkpoint was under
review, slice 1 in 0.3.25). Every type and method named below as new is a
proposal until a slice implements and documents it.

## Background

On 2026-10-19, after the diagram editor of
[diagram-probe-improvements](diagram-probe-improvements.md), the maintainer's
session built a fourth application against the public facade only, chosen to
load what the earlier probes did not: large data. A log explorer generates a
200 000-line log, parses it on a worker in batches delivered through a
mailbox, and shows it in a sortable `Table` with resizable and movable
columns, a `CollectionFilter`, a level `MultiSelect`, a `Tree` grouping lines
by source (tens of thousands of children per source), a detachable Errors tab,
a detail pane, live tail with follow, content search, a workspace and an
inspection snapshot requested from another thread. A headless mode measured
release-build flush times and mounted component counts; the maintainer ran it
natively on Fedora 44 GNOME/Wayland, found that it worked well and missed
selecting and copying text that is not in a field. The application lived in a
session scratchpad; the observations are reported experiment results, not a
reproducible evidence record.

What worked well, once the collections had a bounded height: loading 200 000
lines in batches (169 ms of flushing), filtering them (86 ms), stepped content
search with its work budget (23 420 hits in 28 ms), sorting and level filters
(10–15 ms per flush after the application's own sort), expanding a source of
20 000 lines (21 ms) and stepping inside it (under 1 ms), an inspection
snapshot from another thread (3 ms), workspace capture and restore, and
`Tab::fixed`, `RegionLayout::fixed`, `SplitPane`, `Container` and
`CollectionFilter` keeping the table's mounted rows to a screenful.

What the author ran into, verified in code (file references are to the
checkout on 2026-10-19) or by measurement:

- **Virtualization is silently defeated by ordinary layout.** A `Table`,
  `VirtualList` or `Tree` mounts every row when its height is unbounded:
  5 000 rows mounted about 15 000 components instead of about 90, and the
  probe's first layout mounted 105 057 components, with 3.2 s load flushes
  and a 1 s sort flush. The chain: `Column` measures every child with an
  unbounded height (`containers.rs`, flex measurement); `VirtualList::measure`
  returns the whole collection extent when unbounded (`virtual_list.rs`);
  flex distribution only adds positive surplus and never shrinks a grown
  child; the list then takes its allocation as its viewport and mounts the
  rows inside it. `ScrollArea` removes the height constraint by design, and
  `Tab::new` and `RegionLayout::new` put content in two-axis scroll areas by
  default (`Tab::fixed`, `RegionLayout::fixed`, `View::fixed` and
  `ScrollAxes::None` do not). The contracts state the grow-only rule, but
  the manual teaches `with_grow(1)` for "an editor that fills the rest of a
  window" and says of collections only that "only nearby rows mount", the
  layout guide's large-collections section omits the prerequisite, and no
  development diagnostic reports a collection that mounted its whole
  extent.
- **Copying from a command is awkward.** `Runtime` has no clipboard text
  setter; `EventContext::set_clipboard_text` does. A command action can reach
  one through `Runtime::event`, which focuses the target and can therefore
  run focus-loss behavior or be refused by a modal surface.
- **Display text cannot be selected.** `Label` has no selection; table cells
  and tree rows select rows, not text. The public parts for a custom
  selectable text exist (`TextSelection`, a read-only `DraftEditor` session,
  read-only `TextField`/`TextArea`), but no component shows selectable,
  copyable, wrapping display text, and a read-only field looks like an input.
  Label text is also bounded by `TextEditError::MAX_TEXT_BYTES`, so one very
  long record fails preparation even in an ellipsis cell (TODO.md TEXT-01
  records the related editor limit).
- **Ellipsis labels shape their text more than once.** A newly mounted
  table cell's `Label` prepares in `measure` and again in `arrange`, and each
  ellipsis preparation measures the full string's width with a fresh shaping
  layout before consulting the remembered fit (`text.rs`, `fit_plain`). The
  profile showed about 71 text preparations per wheel step for about 27 new
  cells; preparation calls include cache hits, so the share of real shaping
  must be measured, not inferred.
- **Large-data costs the documentation does not name.** A replaced table
  source is rescanned by an active filter and rebuilds the mounted rows'
  descriptions; a replaced tree source re-flattens its expanded branches;
  `TreeItems::children` returns every child key, so a million-child node
  allocates and indexes a million keys although only a screenful mounts;
  `TableItems::index_of` defaults to a linear scan, which reveal and anchoring
  call; a table builds every column of each mounted row, including
  horizontally clipped columns; filter text joins every cell's text into a new
  string per row. Each is reasonable alone; together they decide whether a
  live, sorted, filtered view stays fast, and an author learns them from a
  profiler.
- **Probe defects found by the review**, recorded so the example in slice 5
  does not repeat them: row identity equated with store position (wrong
  after a skipped line), a linear `index_of`, follow revealing the last
  stored line rather than the last shown one and discarding the receipt, the
  tail worker ignoring a closed mailbox, the Errors tab's sort and copy
  routed to the main table, an unreadable file reported as an empty success,
  and timestamp slicing that can panic on malformed input.

## Working rules

The rules of [app-author-improvements](app-author-improvements.md#working-rules)
apply unchanged: one slice per commit with its rustdoc, examples, gallery,
manual and CHANGELOG; the maintainer's patch version bump per slice; an
independent Codex review before a slice is called merge-ready; the contracts
read before each slice; and the gate table there. Breaking changes are
permitted while the library is unpublished, with replacement usage recorded
and repository consumers migrated in the same slice.

Additionally:

- **Design checkpoint.** Slices 1, 3 and 4 begin with a chosen alternative,
  a compiling API sketch and an enumerated list of contract tests, reviewed
  before implementation starts.
- Performance claims are measured before and after on a defined workload
  (the probe's 200 000-line table and its wheel-step profile, kept as a
  benchmark in slice 5), in the release profile, and recorded with the
  platform in [verification](../docs/verification.md); nothing is claimed
  from compilation.
- `TODO.md` TEXT-01 (large-buffer `TextArea`) stays separate; slice 3 states
  how its component relates to it.

## Slices in order

### 0. Documentation of collection sizing and large-data costs

Status: done (documentation only, no version bump). The interim clipboard
route was left out: slice 2 adds the runtime methods it would describe.

- The manual's collection paragraph, the layout guide's large-collections
  section and the `VirtualList`, `Table` and `Tree` rustdoc state that a
  collection mounts the rows inside its allocated extent, so it needs a
  bounded height, and name the containers that bound it (a window or
  native view root, `SplitPane`, `Container` with a bounded allocation,
  `CollectionFilter`, `Tab::fixed`, `RegionLayout::fixed`, `View::fixed`)
  and those that do not (`Column`/`Row` as a grown child, default `Tabs`
  and `RegionLayout`, `ScrollArea`). The manual's `with_grow` row says that
  it does not shrink a child below its natural size. Where slice 1 changes
  this, it updates the same text.
- The layout guide gains a large-data section: what source replacement,
  active filters, tree re-flattening, `TreeItems::children`, the default
  `index_of`, per-row column building and filter text joining cost, and how
  an application keeps them bounded (reverse indexes, coalesced batches,
  separate sources per view, selection callbacks that do not republish).
- `EventContext::set_clipboard_text` and the commands section name the route
  from a command until slice 2 lands.
- Documentation only; no version bump.

### 1. Collections in unbounded layout

Status: implemented in 0.3.25 as the revised checkpoint decided; the implementation review's fixes (f64 shrinking, diagnostics refreshed with each arrangement, the Row documentation and the missing tests) are in 0.3.26. The
diagnostic compares the visible share of the list's own bounds in window
coordinates, so a scaling ancestor cancels out; the validator's change
detection (`same_node`) had to compare the new collection data. Tests:
`crates/pert-ui/tests/foundation/unbounded_collections.rs` (shrinking by
weight and minimum, the floor, nested columns, rows unchanged, a short list
in a scroll area, the diagnostic for scroll area, default tab and region,
its absence for bounded containers and a self-scrolling tall list, a
clipped panel and an estimated list), the serde round trip in
`serialization.rs`, and the migrated `row_stretch.rs`. Not covered by a
test of its own: a scaling `child_transform`, `Justify::SpaceBetween` after
shrinking and clearing by redescription.

- Decide how the default composition keeps virtualization. Alternatives,
  each with its compatibility cost:
  - Grown children of a bounded `Row`/`Column` may shrink to their minimum
    when the natural sizes overflow, so "toolbar plus collection that fills
    the rest" works; this changes the documented grow-only rule for every
    grown child and needs remeasurement of width-dependent content.
  - A collection reports a small natural size along its scrolling axis
    (for example its minimum rows) when unbounded, and grows when given
    space; this changes intrinsic sizing in scroll areas and of ungrown
    collections.
  - A development diagnostic (`ValidationCode` variant) when a virtualized
    collection's settled allocation mounts more rows than its window can
    show, naming the collection and the unbounded ancestor; low risk, and
    useful whatever else is chosen.
  The checkpoint picks one sizing behavior or none, and the diagnostic.
- Expected results for: a toolbar and a grown table in a column; nested
  rows and columns; a collection in a `ScrollArea` on purpose (all rows
  shown, small lists); default `Tabs` and `RegionLayout`; `SplitPane`; a
  collection with an estimated extent; minimum sizes; and same-key
  redescription.
- Contract tests: mounted row counts per composition, the diagnostic's
  presence and absence (a short list fully visible in a scroll area is not
  reported), and the gallery and examples migrated where they relied on the
  old behavior.

#### Design checkpoint (2026-10-19)

Investigation (`containers.rs`, `flex`): a row or column measures each
child with an unbounded main axis, adds only positive spare space to grown
children (`(limit - used).max(0.0)`), and reports `limit.max(used)` when it
has grown children, so a grown child that is larger than the space never
shrinks and the container overflows its allocation. `VirtualList::measure`
returns its whole extent when unbounded, and `arrange` takes its allocation
as its viewport. Validation hints are part of the inspection schema
(`ValidationHints` and `ValidationCode` serialize under `serde`, with
`InspectionSnapshot::SCHEMA_VERSION` 13).

Chosen alternative (revised after review): grown children of a bounded
column shrink, and a diagnostic reports a virtualized collection that has
its whole content extent but shows only part of it. A collection's natural
size stays its whole extent (changing it would change every intrinsic
measurement, including deliberately unclipped lists in scroll areas).

- **Shrinking grown children of a column.** When a column's height is
  bounded and the children's natural heights plus gaps exceed it, the
  deficit is taken from the grown children by weight, none below its
  minimum (`size_limits`/`measure_limits`), sharing again among the others
  when one reaches its minimum, as growing shares surplus up to maxima.
  Children without a weight keep their natural height. The column then
  reports `max(limit, floor)`, where the floor is the ungrown children's
  natural heights plus the grown children's minimums plus the gaps; above
  the floor it overflows as today. Width does not depend on the height a
  child is given, so nothing is measured again. **Rows are unchanged**: a
  row that shrank a grown text child would have to measure its wrapped height
  again and place the cross axis after it, which this slice does not take
  on; the documentation says so. `Wrap` and unbounded columns (a scroll
  area's content) are unchanged, so lists placed in a scroll area on purpose
  still show every row.
- **Diagnostic.** `ValidationCode::UnboundedCollection`: in a measured
  window, a visible `VirtualList` (also the body of a `Table` or `Tree`)
  with at least 100 rows whose content extent does not exceed its allocated
  extent along its axis, so it cannot scroll itself and mounts every row
  (overscan alone mounting all rows of a list that scrolls does not count),
  while less than half of its allocated extent along its axis is visible in
  its window (the node's clip intersected with its bounds, in window
  coordinates) is reported. The message names the list's key and its owner's
  key, the allocated and visible extents and the row count, says that it
  mounts every row instead of scrolling, and points to the bounded-extent
  documentation. The runtime records the list's row count, mounted range and
  axis crate-privately beside the data it already feeds the validator
  (recognizing the list by downcasting the component), so `ValidationHints`
  and `ComponentDetails` do not change. A list that scrolls itself (its range
  is a part of its rows) is never reported, so a deliberately tall nested
  list is not; a short list (under 100 rows) shown whole in a scroll area is
  not either. Adding the variant changes the serialized inspection schema:
  schema 14, named in the changelog. Release builds have no validator.
- Documentation: the manual's `with_grow` row, the contracts' flex rule, the
  layout guide's collection paragraph and the `VirtualList` bounded-extent
  section (slice 0) say that a grown child of a bounded column now shrinks,
  so a toolbar above a grown collection in a column works, and that rows do
  not shrink.

Contract tests (facade, `foundation` area):

1. A column with a toolbar and a grown table of 5 000 rows in an 800 × 600
   window mounts a screenful of rows, the table fills the height left by the
   toolbar, and no diagnostic is reported.
2. Two grown children with weights 1 and 3 whose natural heights overflow
   shrink by weight; one reaching its minimum stops, the other takes the
   rest; ungrown children keep their natural height.
3. The floor: an ungrown child taller than the bound beside a grown one with
   a minimum reports and overflows at ungrown natural plus grown minimum
   plus gaps; a propagated `measure_limits` minimum and an explicit maximum
   on the grown child are respected.
4. Nested columns, grown at both levels and only at the outer one; a grown
   row inside a column (its height shrinks, its width and wrapping do not
   change); `Align::Stretch` and `Justify::SpaceBetween` after shrinking.
5. A row whose children overflow keeps today's lengths.
6. A column in a vertical scroll area: nothing shrinks and every row of a
   short list shows; no diagnostic.
7. The diagnostic is reported for a table in a default `Tab`, a default
   `RegionLayout` center and a `ScrollArea` (naming `body` and its table's
   key), and for a horizontal list in a horizontal scroll area; not for the
   same table in `Tab::fixed`, `RegionLayout::fixed`, `View::fixed`, a
   `SplitPane` or the shrinking column; not for a 1 200-pixel list that
   scrolls itself inside a 600-pixel window, nor for a scrolling list whose
   overscan mounts all its rows; not for a 50-row list shown whole in a
   scroll area. It is reported for a list shorter than its window that a
   clipping panel mostly hides. With a scaling `child_transform` and at least
   100 rows, a visible share below half is reported and one at and above half
   is not. An estimated-extent list is judged after its extents settle. The
   issue clears when the layout is fixed by redescription.
8. Schema: an inspection snapshot carries schema 14, and the serde round trip
   of a report with the new code works (`serde` configuration).
9. Migration: `crates/pert-ui/tests/foundation/row_stretch.rs` expects a grown
   row with a stretched divider to keep 380 px where 300 remain; it now
   shrinks to 300, which the test states deliberately. The `live_mixer` and
   `diagram_editor` examples and the gallery's headless run are checked at a
   small window size (their sessions use 900 × 520 and 1200 × 800).

Codex reviewed the first version of this checkpoint (2026-10-19) and did not
approve it: shrinking rows without measuring wrapped text again loses
content, a window-size comparison both misses defeated lists in clipped
panels and reports deliberately tall nested ones, `Table` itself does not
scroll (its body list does), the overflow floor ignored ungrown natural
sizes, and the compatibility and test lists were not concrete. The revision
above limits shrinking to columns, defines the diagnostic on the list's own
extent, names the attribution and the floor, and lists the migration and the
added tests. A second review approved the revision once the content-extent
condition (overscan can mount every row of a list that scrolls) and the
clipped-panel, transform-threshold and estimated-extent tests were added.

### 2. Clipboard text from the runtime

Status: implemented in 0.3.24; review fixes (phase, reentrancy and service-error tests) in 0.3.26. The headless runtime has no clipboard until
one is installed (the plan's "in-memory clipboard" was wrong); the methods
then report `ClipboardError::Unavailable`. Tests:
`crates/pert-ui/tests/foundation/runtime_clipboard.rs` and the
`Runtime::set_clipboard_text` doctest.

- `Runtime::clipboard_text` and `Runtime::set_clipboard_text`, with the
  `EventContext` methods' errors, phase restrictions and concealment rules,
  usable from command actions and application code on the UI thread without
  moving focus. The headless runtime keeps its in-memory clipboard; the
  native one uses the platform clipboard as `EventContext` does.
- Contract tests: a command that copies, refusal during update, layout,
  paint and inspection, and that focus does not move.

### 3. Selectable display text

Status: implemented in 0.3.27 as the revised checkpoint decided. The
editors' movement mapping and clipboard menu are shared crate-internal
pieces; the menu is built by the component's first update. Tests:
`crates/pert-ui/tests/editing/selectable_text.rs` and the `SelectableText`
doctest.
The implementation review's fixes are in 0.3.28: cancelled input ends a
drag, native keys are read by their layout identity in the text phase, and
a redescription with another source ends a drag even when its text is equal.

- A component for read-only display text that the user can select and copy:
  wrapping, keyboard and pointer selection with the editors' click rules
  (`ClickCounter`), Ctrl/Cmd+A and copy, a context menu with Copy, focus as
  one Tab stop, bound text that keeps a selection when it still fits, and
  typed semantics. Alternatives: a new `SelectableText`, or a
  `Label::selectable` option. AGENTS.md keeps captions and adornments
  separate from rich text composition, which favors a separate component.
  The checkpoint decides, and states its size limit relative to TEXT-01 and
  whether it prepares only visible paragraphs.
- Not in this slice: text selection across table cells or tree rows; a
  table offers row copying through slice 2 and the example.
- Contract tests: selection by drag, double and triple click, keyboard and
  select-all, copy through the clipboard, rebinding, focus and semantics,
  and the size limit's refusal.

#### Design checkpoint (2026-10-19)

Investigation: `Label` prepares and paints text with no selection state.
The text editors (`text_fields.rs`, `Editor`) support a read-only, embedded
(frameless) mode, but they are built for editing: a bound `Signal<String>`,
internal scrolling with a six-line preferred height for `TextArea`, a caret,
IME, undo and a private field menu; a read-only `TextArea` therefore looks and
sizes like an input. The public `TextSelection` (`text_selection.rs`) offers
what a display needs over a `TextLayout`: `at_point`, `word_at_point`,
`moved` with visual movements, `range` and `rectangles`. `TextCommand` has
`Copy`, `Cut`, `Paste` and the word insertion; `DraftEditor` serves an
owner's context menu through `Component::text_command`.

Chosen alternative (revised after review): a new component,
`SelectableText`, rather than a `Label` option or a frameless read-only
`TextArea`. `Label` deliberately never takes focus, and AGENTS.md keeps
captions separate from rich composition; a read-only `TextArea` keeps input
sizing, internal scrolling and a writable binding. Read-only editors already
suppress IME and the caret blink, so the component shares their key mapping
and their clipboard menu rather than re-deriving them: the editors' movement
mapping in `Editor::key` and the `FieldMenu` are made crate-internal shared
pieces, and the component itself is otherwise built on `TextLayout` and
`TextSelection`.

API sketch:

```rust,ignore
pub struct SelectableText { /* source, style, wrap, selection, clicks, menu */ }
impl SelectableText {
    /// Fixed text, a `String` or `RichText` binding or a string-table role;
    /// shown and copied as plain text.
    pub fn new(text: impl Into<TextSource>) -> Self;
    pub fn style(self, style: TextStyle) -> Self;   // default Body
    pub fn wrap(self, wrap: bool) -> Self;          // default true
}
```

- **Presentation:** measures and wraps like a multiline `Label`: no frame,
  the height of all lines and no internal scrolling (a long text goes in a
  `ScrollArea`). Selected regions are every rectangle
  `TextSelection::rectangles` returns for the painted layout, so a
  mixed-direction line shows each visual region, filled with
  `ColorRole::TextSelection` and its glyphs repainted in
  `OnTextSelection`, as the editors do. A focus ring shows only while
  focused. A rich source is projected to its plain text (styling, icons and
  gaps are not shown, and icon-only content shows nothing), so what is
  copied is what is shown; the documentation says so.
- **Pointer:** a primary press focuses it, places the anchor and starts a
  capture drag that extends the selection; Shift+press extends from the
  anchor; a `ClickCounter` series selects the word on the second press and
  everything on the third, cycling as the editors do. Release ends the drag.
  There is no automatic scrolling of an enclosing scroll area while dragging
  and no reveal of the selection's end beyond the whole widget's; this slice
  narrows to that, and the documentation says so.
- **Keyboard (focused):** the editors' movement mapping, shared: with Shift
  the arrows, Home/End (and their line and document forms) and the
  platform's word keys (Alt on macOS, Control elsewhere) extend the
  selection; without Shift they collapse it as the editors do for a
  selection; the primary modifier with A selects all and with C copies.
  Repeats act like presses. Committed text and IME preedit change nothing,
  and `text_input_state` is absent.
- **Escape, in the text phase:** `Component::text_key_event` handles a fresh
  Escape before capture bindings and surface dismissal: the first Escape
  ends an active drag (keeping the selection), otherwise it collapses a
  nonempty selection; with neither it returns `NotAnEditor`, so Escape
  bubbles and closes an enclosing surface. Both `key_input` and
  `command_key_input` follow this order.
- **Context menu and commands:** a secondary press, the ContextMenu key or
  Shift+F10 opens the shared clipboard menu with Copy (enabled only with a
  selection, `StringRole::TextCopy`) and Select all; it keeps the selection,
  closes on the usual surface dismissals and returns focus. Its Copy is a
  queued `TextCommand::Copy` against this component.
  `Component::text_command` answers `Copy` (writes the selection through
  `EventContext::set_clipboard_text`, propagating a clipboard error;
  `Unavailable` without a selection) and `Unavailable` for every mutating
  command (`Cut`, `Paste` and the word insertions); the text never changes.
- **Bound text:** a new text keeps the anchor and focus offsets when both
  are inside it, snapped to grapheme boundaries of the new layout (so a
  combining mark or an emoji sequence never splits) with the direction kept;
  otherwise the selection collapses to the start. A replacement during a
  drag ends the drag. Same-key redescription with the same source keeps the
  selection; another source adopts it like a replacement. Text that becomes
  empty makes it unfocusable and releases focus and capture.
- **Refusal:** text the display cannot prepare, either longer than
  `TextEditError::MAX_TEXT_BYTES` or beyond the raster limits at the current
  width, style or scale, is refused without failing the flush: the
  component keeps showing the last text it accepted with its selection (or
  nothing if none was accepted yet), reports the reason as a `text_error`
  inspection property and marks its semantics invalid, and recovers when a
  later text or size fits. `ForeignRuntime` and other binding errors
  propagate. Paragraph-partitioned preparation for long text remains TODO.md
  TEXT-01.
- **Semantics:** `SemanticRole::Label`, named by its whole plain text,
  `read_only`, with a `selection` inspection property `anchor..focus` in
  bytes (direction visible). Displayed text is inspectable and searchable,
  like a label; it is no place for secrets.
- Gallery: the Text page shows it; the manual's text table and the controls
  guide name it.

Contract tests (facade, `editing` area):

1. A drag selects a range and Ctrl/Cmd+C copies exactly that text through an
   installed clipboard; the press focused it; Shift+press extends.
2. Double and triple clicks select a word and everything; a fourth starts
   over.
3. The movement matrix on the editors' bindings, including word movement
   with the platform's word modifier, Shift extension, collapsing, repeats
   and select-all; committed text and preedit leave the text unchanged and
   `text_input_state` absent.
4. Escape through both `key_input` and `command_key_input`: ends a drag,
   then collapses, then bubbles; inside a popover with a selection, the first
   Escape collapses and the popover stays, the second closes it; a capture
   command on Escape does not run while the selection collapses.
5. The context menu: a secondary press opens it with Copy enabled for a
   selection and disabled without one; Copy copies after the menu closes;
   the selection survives; Escape closes the menu and focus returns.
6. `Runtime::text_command`: `Copy` copies and reports `Unavailable` without
   a selection; `Cut`, `Paste` and the word insertions report `Unavailable`;
   a failing clipboard propagates its error.
7. Bound text: a still-valid selection is kept and snapped (combining mark,
   ZWJ emoji), a reversed selection keeps its direction, an invalid one
   collapses, a replacement during a drag ends it, empty text releases
   focus; a role source follows a replaced string table; a rich source shows
   and copies its plain text and icon-only content shows nothing.
8. Wrapping and painting: the height grows with lines at a narrow width; a
   selection across wrapped lines paints one region per line; a
   mixed-direction selection paints every rectangle `rectangles` returns
   and none between them.
9. Refusal: text above `MAX_TEXT_BYTES` and text beyond the raster limit at a
   narrow width keep the previous text, report `text_error` and recover; a
   foreign binding fails the update.
10. Focus: one Tab stop in traversal order both ways; empty text is not one.
11. Semantics name and the `selection` property; content search finds it.

Codex reviewed the first version of this checkpoint (2026-10-19) and did not
approve it: `Label` fails the flush on oversized text rather than refusing it,
Escape needed the text phase and a consistent order, selection reveal in a
scroll area was unspecified, the promised context menu had vanished from the
design, character-boundary preservation contradicted grapheme snapping, bidi
painting and selected glyph colors were missing, and the movement matrix was
vague. The revision above answers each.

### 4. Ellipsis text preparation

Status: implemented in 0.3.29 as the revised checkpoint decided; tests in
`text.rs` (`shapings` counter). A wheel step over the `log_explorer`
example's 50 000-line table went from 4.78 to 4.13 ms (release, Fedora 44).
After the implementation review (0.3.31) the eviction tests use real
eviction, icon and gap entries are covered, and CPU captures of 56 ellipsis
cases (four scales, seven widths, both ellipsis modes, warm and cold passes)
match 0.3.28 pixel for pixel.

- Measure first: split the profile's text preparation into cache hits,
  shaping and rasterization for the table benchmark, and record the
  numbers.
- Then reuse a label's fitted result when width, text, style and scale are
  unchanged between measure and arrange, and consult a remembered fit before
  shaping the full string, keeping font, style and scale invalidation and
  the raster limits intact.
- Acceptance: fewer shaping passes per newly mounted cell on the benchmark,
  unchanged output (CPU capture comparison of an ellipsis table), and the
  existing text tests green.

#### Design checkpoint (2026-10-19)

Investigation (`text.rs`, `TextCache::fit_plain`): an ellipsis label
prepares in `measure` and again in `arrange`, both through `fit_plain`. Each
call first measures the whole string with `plain_width`, which builds a new
shaping layout every time and is not cached, even when the cache already
holds the full prepared layout of that text at that size; a truncated text's
remembered cut is consulted only after that full measurement. So a fitting
cell shapes its text twice uncached plus once for the cached layout, and a
truncated one shapes the full string twice plus the search once. Baseline on
the probe's 200 000-line table (release, Fedora 44): about 4.2 ms per
120-pixel wheel step, with text preparation the largest self time in the
profile (56 ms over 20 steps).

Chosen change (revised after review), inside the text service only (no
public API):

- **The full width from the cache.** `fit_plain` answers "does the whole
  text fit" from a cached prepared layout when one has exactly the key
  `plain_width` would shape: `width: None`, the effective scale
  (`effective_scale(request.scale, [font_size])`), the role's default font
  index and size, and the plain-content predicates `prepare_plain` already
  checks (no gaps, inherited color, no icon brush, one span covering the
  text, or none for empty text). The width is `layout.shaped.width() /
  scale`, the value `plain_width` computes, never `size().width()`, which
  adds rounding and overhangs and could change a fit decision. The hit
  records the role's font read as `prepare_plain` does. Wrapped and rich
  entries never answer it.
- **One shaping for a cold fitting text.** When the cache has no such
  entry, `plain_width` shapes as today and hands its shaped layout on, so
  preparing the full text rasterizes that layout instead of shaping it
  again. A cold fitting text shapes once and a warm one not at all; since
  the unwrapped key has no width, a changed allocation width that still
  fits shapes nothing either.
- **The remembered cut first.** A truncated text consults its `FitKey`
  before measuring the full string. A cut is remembered only for a text
  that did not fit at that key, so with one the full string is not shaped
  again; the candidate is prepared through the cache (and prepared again
  if the cache evicted it). The existing overhang check after the search
  (the step back while the prepared candidate is still too wide) stays.
- Both cache keys contain the resolved font index, and replacing a role's
  font changes the index it resolves to, so neither answer outlives a font
  change. The raster limit stays
  protected: the full string is still never rasterized merely to learn its
  width.

Tests (runtime unit tests in `text.rs`, with a test-only shaping counter on
`build_layout` beside the existing `fit_searches` and `resolutions`):

1. A cold fitting ellipsis label shapes once; measure, arrange, a repaint
   and a changed width that still fits shape nothing more.
2. A truncated one shapes the full string once, then the remembered cut
   answers later passes without shaping it again.
3. Cold and warm results are identical (shown text, size and the prepared
   glyph signature) for widths just below, at and just above the text's
   advance width, and for a text whose glyph overhang exceeds its advance.
4. A wrapped (`width: Some`) or rich (colored, icon, gap) entry for the same
   text does not answer the full width; a different size, effective scale or
   role resolving to another font misses (roles sharing a font may hit); scales above `MAX_FONT_PIXELS` that reduce to the same
   effective scale hit.
5. The role's font read is recorded on the cache hit; replacing the role's
   font shapes again.
6. Evicting the prepared layout keeps a remembered cut usable (the
   candidate is prepared again), and evicting the cut (beyond
   `FIT_MEMORY`) searches again with the same result.
7. A full text wider than the raster limit whose truncation fits succeeds
   without preparing the full string; a truncation that is itself beyond the
   limit at a very wide allocation is refused with `TextLimit::RasterSide`.
8. The existing ellipsis tests stay green.

Codex reviewed the first version of this checkpoint (2026-10-19) and did
not approve it: the cache equivalence was not exact (`width: None`, effective
scale, plain predicates) and used a width that includes overhangs, "shapes
once" contradicted the cold path, and the tests left out equivalence,
eviction, font and raster cases. The revision above answers each; a second
review approved it, correcting how font replacement keeps the caches sound.

Measured afterwards on the same benchmark and recorded in
[verification](../docs/verification.md).

### 5. A large-data example

Status: implemented in 0.3.30 as `crates/pert-ui/examples/log_explorer/`.
The session copies through the main table's view-scoped binding; the
Errors table's binding is the same code under its own view, not separately
exercised headless. The implementation review's fixes are in 0.3.31: tail startup
after loading with a shared identity allocator, loss accounting across
stopped workers, follow and the shown count after the filter field, date
and time validation and a bounded inspection wait.

- Ship the log explorer, reduced to what it demonstrates, as a package
  example (headless measured session by default, `--window` with `native`):
  bounded layout for every collection, row identities independent of
  storage with a reverse index for `index_of`, batched loading with typed
  failures, a sorted and filtered table, a tree over sources, follow that
  reveals the last shown row and observes the receipt, a detachable Errors
  view with its own sort and copy routed by command context, copying through
  slice 2, selectable detail text through slice 3, content search and an
  inspection snapshot from another thread. Its headless session reports
  mounted counts and flush times, and asserts the bounded counts.
- The guide's large-data section links it.

## Optional follow-ups

Taken up only when an application needs them, each with its own checkpoint:
an indexed or paged child source for trees with extreme fan-out and
incremental flattening; horizontal virtualization of table cells; an
incremental source-update API (append, replace a range) with its
consistency and invalidation contract; text selection across collection rows;
bounded preparation of very long display text (with TEXT-01).

## Not in this plan

A log viewer or log-format component in the library, accessibility
integration, and file dialogs (app-author plan slice 8).

## Review record

- 2026-10-19, Codex (`codex exec -s read-only`), review of the findings log
  and the probe sources: confirmed the virtualization finding with the full
  measurement-to-mount chain and the containers that do and do not bound a
  collection; narrowed the clipboard finding to an awkward route
  (`Runtime::event` focuses its target) and the selectable-text finding to a
  missing component (custom ones are possible with `TextSelection` and a
  read-only `DraftEditor`); confirmed repeated ellipsis shaping while warning
  that preparation counts include cache hits; added the large-data costs and
  the probe defects listed above. It recommended documentation and a
  diagnostic before any sizing change.
