# Round 2 fix plan, part P7 design checkpoint: rows that shrink

Part P7 of [the round 2 fix plan](round-2-fix-plan.md#p7-rows-that-shrink-0382)
closes R2-21 by decision D7 (approved with the recommended option on
2026-10-28). The plan leaves to this checkpoint the algorithm, the second
measurement's cost and where it may run, and the gallery pages whose layout
changes, with captures before and after; this document settles them, with
the documentation, migration manifest, tests, CHANGELOG, gate and native
evidence, before any code. Codex reviews it; the author approves it
together with Codex's review under the maintainer's delegation of
2026-10-28.

Every reference to library code is to the committed tree at `fec6a5f`,
whose library code is P1's (84bb60f, 0.3.76): the commits after it add only
design checkpoints. P2 to P6 change none of the code P7 changes except the
`ProgressBar` page and constructor (P6), so the implementer relocates a
citation by its content where an earlier part moved it. Path prefixes are
the plan's: `R/` = `crates/pert-ui-runtime/src/`, `F/` =
`crates/pert-ui/src/`, `G/` = `crates/pert-ui/src/guide/`; `T/` =
`crates/pert-ui/tests/`, `GA/` = `apps/gallery/src/`, **C**
`R/containers.rs`, **L** `R/layout.rs`, **U** `~/pert-ui-usability/`
(frozen runs).

**Dependency on P6.** P7 lands after P6 (0.3.81), whose design is approved
and committed ([round-2-design-P6.md](round-2-design-P6.md), fec6a5f). P7's
bar-plus-percent recipe, the gallery's progress column and the
`with_grow` example describe the bar with P6's fixed constructor,
`ProgressBar::new(ProgressStatus)` (P6 design, section 4), as the plan's
order requires ("P6 before P7", `round-2-fix-plan.md:1332`); P6's text says
nothing about row shrinking (P6 design, section 5, "Later parts"). Nothing
else of P6 is used.

Status: **revision 5 with the author's correction of Codex's fourth
recheck, approved 2026-10-28** by the author together with Codex's review,
under the maintainer's delegation of 2026-10-28, with P7-D1 as
recommended. Revisions
1 to 4 answered Codex's review and three rechecks; the third recheck
accepted revision 4's algorithm and left two should-fix points, which
revision 5 answers ([review record](#review-record)). One decision,
**P7-D1** (which measurements a component keeps and how their
dependencies are kept), is recommended below and approved. Codex's fourth
recheck left one wording point (the overflow floor omitted a growing
unshrunk fill child), which the author corrected as Codex proposed, with
test 18's variant (b).

## What testers built by hand, and what the library says

- **R2-21** (`plans/archive/developer-usability.md:939`): five task 5 runs,
  on both commits, put a `ProgressBar` with a growth weight beside its
  percentage `Label` in a table cell and got `EscapesParent` with the
  percentage clipped away (5-head-claude-2, 5-head-codex-1, 5-head-codex-2,
  5-r1-claude-1, 5-r1-claude-2). Severity slow on the head, annoyance on
  359122e.
- **The verified cause** (triage `runs/triage-r2/group-3.md:17-25`,
  `group-4.md:14-24`, `:39`, merged at `codex-review.md:79`): the bar
  measures `min(available, 3 × ControlWidth)`, 192 px at Standard, and a
  `Row` never shrinks a child. The probe: in a 230 px column the cell's row
  is 206 px, the bar keeps 192 px and the 28 px label ends at 240, past the
  cell; in a 167 px cell the same, with the label clipped in the capture.
- **Each run worked around it differently** (`group-3.md:100`, `:109`,
  `:115`, `:120`, NF-2 at `:259-265`): a small `Container` with a fixed
  preferred size around the bar (5-head-claude-2, U
  `r2-5-head-claude-2/work/src/lib.rs:321-326`; 5-r1-claude-1, U
  `r2-5-r1-claude-1/work/src/monitor.rs:216-218`), a `Column` of bar and
  percent with 64 px rows (5-head-codex-1), a wider column and a shorter
  label (5-head-codex-2). 5-head-claude-1 avoided it only because its
  column was wider than bar plus label.
- **What the library says:** `Element::with_grow` ends "Rows never shrink a
  child" (L:615); guide/layout.md says a bounded `Row` "does not shrink
  their widths" (`G/layout.md:458-460`); contracts.md gives the reason
  ("a narrower child could wrap to a height their measurement does not
  repeat", `docs/contracts.md:441-442`); the `ProgressBar` page says only
  "three themed ControlWidths wide by preference, and fills the width it is
  given" (`R/progress.rs:100-102`).

## 1. Today's behavior, from the code

- **Row measurement.** `flow_rects` (C:477-562) first measures every visible
  child of a row at an **unbounded width**: `(None, offered height)`, or
  `(None, None)` for a child that opts into `Component::fills_row_height`
  (C:487-502). The tallest result, capped by the offered height, is only
  the **fill offer**: fill children are measured again at it (C:503-508).
  The row's own natural height is the tallest child's, **uncapped**
  (C:545-547, :556), and ordinary alignment places children against it
  (C:718-721), as contracts.md states ("a row within its tallest child",
  `docs/contracts.md:445-449`). Children are then placed at their natural
  widths (C:513-552) and `flex` (C:566-793) grows and places them.
- **Growth.** `flex` shares spare main-axis space by weight up to each
  child's maximum, re-sharing what a maximum stops (C:591-638), for rows and
  columns alike.
- **The shortfall is a column's only.** The deficit code runs under
  `if column && grows && let Some(limit) = limit` (C:650-653): in f64, the
  growing children give up `used - limit` by weight, none below its
  minimum height, a capped child's remainder shared again among the others
  (C:654-705); `absorbed` records whether the whole deficit was taken
  (C:649, :687, :696). The comment gives the reason rows are excluded
  (C:639-646). A column then reports its limit when absorbed, otherwise the
  sum of its final lengths (C:763-777), and fills its bounded main axis
  when it grows or justifies (C:762, :778-787).
- **A row's width.** Without a shortfall, a growing row whose natural widths
  exceed its limit reports `limit.max(used)` = `used` (C:778-779): wider
  than it was offered. Its parent clips or not; development validation
  reports a child whose visible bounds leave its parent without a clip as
  `EscapesParent` (`R/validation/development.rs:618-632`).
- **Measure and arrange both run the algorithm.** `Row::measure` calls
  `flow_rects` with the offered constraints and `Row::arrange` with
  `Constraints::bounded(size)` (C:880-898), so each layout pass computes
  the row's placement twice, and every child is measured from both.
- **One measurement cache per component.** `Node::measured` holds one
  `(Constraints, Size)` (`R/runtime.rs:858`); `measure_node` returns it only
  for equal constraints and otherwise measures and replaces it
  (`R/runtime/tree.rs:1656-1690`); a layout invalidation clears it
  (`R/runtime.rs:978-981`). `Component::fills_row_height` documents the
  consequence: opting in "requests two measurement constraints in each row
  layout pass … The runtime keeps one measurement cache per component"
  (`R/runtime.rs:1739-1742`).
- **Children with no limits shrink to nothing.** `ProgressBar` has no
  `size_limits` or `measure_limits` (`R/progress.rs:194-231`), so its
  minimum is zero (`Component::size_limits` default, `R/runtime.rs:1781-1783`).
  `Label` wraps at its allocated width by default (`R/controls.rs:117`,
  `:132-133`) and truncates with `TextOverflow::Ellipsis` (`:135-148`). A gallery
  `tile` has a 24 px minimum (`GA/gallery.rs:705-716`).
- **A table cell** measures its content at the column width less twice
  `ControlPaddingX` (12 at Standard, `R/runtime/theme.rs:203`) and arranges
  it at that width and the row's height (`R/table.rs:948-983`), so a `Row`
  in a cell is bounded in both measurement and arrangement. `ControlWidth`
  is 64 and `ItemGap` 8 at Standard (`R/runtime/theme.rs:206`, `:199`).
- **Who has a growing child in a row today.** No built-in component gives
  one of its own children a weight: the runtime's only `with_grow` call is
  `Element::spacer` (L:663-665), whose natural width is zero. Rows that
  hold application elements (`ToolGroup` controls, `R/toolbar.rs:376`;
  window actions, `R/windows.rs:551`, `:609`) follow the weights the
  application gives. The maintained consumers with a growing row child are
  listed in section 7.

## 2. The rules

### 2.1 A row's shortfall

A row runs these steps in measurement and in arrangement, exactly as it
runs today's (arrangement with its bounded size):

1. **Natural pass, unchanged.** Each visible child *i* is measured at
   `(None, hᵢ)`, `hᵢ` being the offered height, or `None` for a fill child:
   natural width `wᵢ`, height `tᵢ`.
2. **When a shortfall applies.** Only when the row's offered width is
   bounded (`L`), at least one child has a weight above zero, and
   `D = Σ wᵢ + gap × (n − 1) − L` is above zero, computed in f64. Otherwise
   the row follows today's path with no further measurement: a row that
   fits, a row without a growing child, and a row offered an unbounded width
   (a horizontal `ScrollArea`, or another row's natural pass) are
   unchanged.
3. **Who gives up what.** The growing children give up `D` by weight, each
   down to its minimum width `mᵢ = limits(i).minimum().width()`; a child
   whose share would take it below its minimum gives only its room
   `wᵢ − mᵢ` and the rest is shared again among the others, until `D` is
   taken (`absorbed`) or every growing child is at its minimum. This is the
   column's arithmetic (C:654-705), in f64 yielding final lengths, with
   widths for heights. Children without a weight never give up width; a
   growing child with no room (a spacer, or one already at its minimum)
   gives none.
4. **Second measurement, ordinary children only.** Each ordinary child
   (one that does not fill the row's height) that gave up width, at final
   width `fᵢ < wᵢ`, is measured again at `(Some(fᵢ), hᵢ)`. Its **height**
   becomes that measurement's height; its **allocated width** is `fᵢ`,
   whatever width the measurement reports (a wider report overflows the
   child's own allocation, a narrower one leaves it room, as a column's
   allocation already differs from a measured width, C:536-543). A child
   that gave up nothing keeps its natural measurement. A **fill child is
   not measured at its final width without a height:** its provisional
   height stays its natural one, as today.
5. **The fill offer, today's rule.** `c = min(T, offered height)`, where
   `T` is the tallest provisional height: the ordinary children's after
   step 4 and the fill children's natural ones, as at C:500-503. Each fill
   child is then measured once more, as today: at `(Some(fᵢ), Some(c))`
   when it gave up width and at `(None, Some(c))` otherwise (C:504-508).
   That measurement is the fill child's **final** measurement: its height
   is its final height, and its width is its allocation when it did not
   give up width (today's rule, C:506 replaces its size) and `fᵢ` when it
   did.
6. **The row's natural height, today's rule.** The maximum of the
   **final** child heights: the ordinary children's after step 4 and the
   fill children's fill measurements, **uncapped**, as C:545-547 and :556
   compute it today from the final sizes. A shrunk child that wraps taller
   than the offered height makes the row that tall; a divider whose natural
   height (32 px) exceeds a smaller fill offer counts with its fill height,
   not its natural one. `Align` places children within this height, or the
   offered height for `Stretch` (C:718-721).
7. **Final allocations.** After step 5 every child has its final
   allocated width: `fᵢ` for a child that gave up width (a fill child
   included), its fill measurement's width for a fill child that did not,
   and its natural width for the others. Everything below is computed from
   these, never assumed from the natural pass. Let `S` be their sum plus
   the gaps, in f64.
8. **Decided once, from the natural pass.** Whether the row shrinks is
   decided by step 2 on the natural widths, and how much each growing child
   gives up by step 3; neither is revisited after the fill measurement, so
   the row never iterates (the fill offer depends on the heights the
   shortfall produces).
   - **No shortfall (step 2 false):** today's path exactly. Growth shares
     `L − S` (zero if negative) among the growing children up to their
     maximums, as `flex` does today from the final lengths (C:504-508
     replace the fill sizes before C:583-591 compute `used` and `spare`).
   - **Shortfall applied:** no child grows, even if fill children narrowed
     at their fill height and `S` ended below `L`.
9. **Spare space and `Justify`, from the final allocations.** The space
   `Justify` places (C:707-713) is `max(0, L − S)` after growth, as today.
   With a shortfall it is usually zero; it is positive only when a fill
   child narrowed at its fill height, and is then placed as `Justify` says
   instead of being assumed away.
10. **The row's width, from the final allocations.** The width the row
    reports is today's rule over `S` (C:760-787): a growing or justified
    row with a bounded width reports `max(L, S)`, any other `S` as its
    natural width. One exactness rule is the column's: when the shortfall
    was absorbed and every fill child kept its natural width, `S` equals
    `L` and is reported as `L` exactly, so f32 cancellation on huge
    natural widths cannot move the edge (C:644-646, :766-767). When `S`
    exceeds `L` the excess is the **residual overflow**: past the ungrown
    children's widths, the grown children's minimums and any width a fill
    child gained at its fill height. Validation reports it as today
    (`EscapesParent`).
11. **Fill widths that change at the fill height, both ways.** The public
    hook permits a fill child to report another width at its fill height
    (`R/runtime.rs:1731-1747`); built-in fill children (`Divider`) report
    the same width at every height (`R/views.rs:995-1006`), and a row whose
    fill children keep their width is laid out exactly as today's code
    would lay it out with the shortfall of steps 2 to 4 added:
    - **wider** (100 without a height, 300 with one, beside a growing 300 in
      a 250 px row): the natural deficit 150 is taken from the grower (300
      to 150); the fill child then takes 300; `S` is 450, the row reports
      450 and overflows by 200, which validation reports (test 18);
    - **narrower** (300, then 100, beside a growing 100): the natural
      deficit 150 takes the grower to 0 without absorbing it; the fill
      child then takes 100; `S` is 100, nobody grows back, the row reports
      250 (a growing row fills its bounded width) and `Justify` places the
      150 left over (test 19);
    - a natural pass that fits keeps today's behavior, also when a fill
      child then makes the row overflow (test 16).

### 2.2 The column, and one shortfall function

1. A column's results are unchanged, bit for bit, including the f64 case
   of a natural height in the hundreds of millions
   (`T/foundation/unbounded_collections.rs:223-249`). A column takes no
   second measurement: a height shortfall does not change the width a
   column offers its children (C:516).
2. The deficit arithmetic is one crate-private function in C used by both
   flows, called with main-axis lengths, weights and minimums, returning
   each child's final length or none and whether the deficit was absorbed
   (for example `fn shortfall(deficit: f64, growing: &[Shrinkable]) ->
   Shortfall`; private names at implementation). For both flows it runs in
   `flow_rects` after the natural measurement, so a row can measure again
   before it computes its height (2.1.4-6), and `flex` receives its result
   instead of computing it. The comment at C:639-646 is rewritten for both
   axes.

### 2.3 The second measurement: cost and where it may run

1. **Where.** Only in a `Row`'s `measure` and `arrange`, through
   `MeasureContext::measure` (public, L:1018-1027; the arrange context
   lends it, L:1053-1072), during the layout phase of a flush. Never in
   `update`, `paint`, input, inspection or validation; it writes no state
   and schedules nothing. Only for a growing child that gave up width under
   2.1.2's three conditions. `Column`, `Wrap`, `Grid` and custom
   containers measure as before.
2. **What a row asks of a child, per row computation (G3).** At most two
   constraints for every child. An ordinary child: A = `(None, h)` and,
   when it shrank, B = `(Some(f), h)`, with the same `h`. A fill child: A =
   `(None, None)` and its fill measurement, `(None, Some(c))` as today or
   `(Some(f), Some(c))` when it shrank (2.1.4-5). A row that fits asks what
   it asks today. A fill width that changes at the fill height adds no
   measurement: the row decides once and never iterates (2.1.7-11).
3. **What happens across a pass.** A row computes its placement in
   `measure` and again in `arrange` (section 1), and a component is asked
   again whenever an ancestor computes again. A row measured and arranged
   at one height (a root, a stretched row, a table cell) asks a child for
   the same pair twice; a row in a column is measured with an unbounded
   height (C:516) and arranged with its allocated one (C:887-892), so it
   asks each child for two pairs, four constraints. Nested shrinking rows
   repeat a parent's pairs once more in the child row's own arrangement.
   - **One entry (today):** the members of a pair evict each other, so
     every repetition misses, and a shrunk child that is itself a
     shrinking row recomputes its own shrunk child each time: the count
     multiplies with nesting (section 8, test 7).
   - **Unbounded retention per pass (revision 2's proposal):** no
     repetition at all, but storage proportional to the distinct requests,
     lazy pruning that leaves an idle component's old entries in place,
     overflow capacity to reclaim and dependency masks to recompute on
     every drop (Codex's recheck, points 1 and 3): more mechanism than the
     measured cases need.
4. **The rule (P7-D1, recommended): four measurements per component, the
   least recently used replaced, with dependency bits sticky until the
   component's layout is invalidated.**
   - **Store.** Each component keeps up to four `(Constraints, Size)`
     entries inline, in place of today's one (`R/runtime.rs:858`), with a
     small recency order. No pass counter, no stamps, no list that grows:
     nothing is allocated, aged, swept or polled.
   - **Lookup** returns an entry with equal constraints and makes it the
     most recently used: four comparisons at most.
   - **Miss.** The measurement is computed as today; the new entry
     replaces the least recently used one when all four are taken. Cost
     per lookup and per insertion: constant (four entries).
   - **Dependencies.** Before a miss, the component's measurement read
     masks (`metric_layout[1]`, `font_layout[1]`) are **no longer reset**
     (`R/runtime/tree.rs:1673-1674` resets them today): each computation
     adds its metric and font reads to them, and nothing removes a bit
     until the component's layout is invalidated: `invalidate_layout`
     (`R/runtime.rs:978-981`) clears the entries, as it clears the one
     today, and P7 adds the two measurement masks to what it clears. An
     evicted entry's bits stay, so a result computed from it
     anywhere, including an ancestor's retained measurement that read the
     child's result through `MeasureContext::measure` (L:1018-1027, which
     records nothing in the caller), stays covered: the notifier
     (`R/runtime/theme.rs:952-960`, `R/runtime/fonts.rs:19-23`)
     invalidates the child, and the invalidation reaches every ancestor
     (`R/runtime.rs:983-986`). The bits are conservative: a dependency of
     a measurement no longer used still invalidates, which costs a layout
     and never leaves a result stale. The limits slot `[0]` and the
     arrangement slot `[2]` keep today's reset, since each still holds one
     result.
   - **Storage.** Four entries of 24 bytes (constraints 16, size 8) and
     their order: about 100 bytes per component in place of today's 28;
     the masks are today's cells.
   - **Fallback, documented:** a component asked for more than four
     distinct constraints between invalidations computes again whatever
     was replaced. The result is always correct (each computation is
     today's); only the repetition returns.
5. **What it guarantees, and what it does not.**
   - **G1, within a pass:** a component asked for at most four distinct
     constraints in a layout pass computes each at most once in that pass,
     since a least-recently-used store of four never replaces one of the
     four most recently used. Covered: an ordinary or fill child of a row
     measured and arranged at one height or at two (2.3.3), and nested
     shrinking rows whose levels are offered one height (tests 7, 12, 13).
   - **G2, across passes:** a component whose layout was not invalidated
     reuses any of its four most recent measurements; a row redescribed
     with only a new `Justify` measures nothing of its children again
     (test 13).
   - **G3:** 2.3.2's two constraints per child per row computation.
   - **Not guaranteed:** a component asked for more than four distinct
     constraints in one pass, as a deeply nested shrinking row whose levels
     offer different heights can be, repeats measurements (the fallback);
     the number of distinct constraints still grows with the distinct
     heights ancestors offer, as it does today for children of unshrunk
     rows. Test 17 shows the fallback correct.
6. **Retention of settled layouts, unchanged.** A flush with no layout
   invalidation runs no layout (`R/runtime/tree.rs:1892-1894`), so a
   settled shrunk row measures nothing; an invalidated child invalidates
   its ancestors' caches, not its siblings', as today
   (`R/runtime.rs:978-987`). An idle subtree keeps its entries as it keeps
   its one entry today.
7. **No privileged path.** The second measurement uses the public
   `MeasureContext::measure`, `limits` and `grow`; a custom horizontal
   container can implement the same rule, and the store serves every
   component alike.

### 2.4 Consequences for existing layouts

1. A grown `ProgressBar` beside its percentage in a cell narrower than both
   gives up width, down to none (its minimum is zero), and the percentage
   stays inside the cell.
2. A grown wrapping `Label` (the default overflow) in a row narrower than
   its line wraps to more lines, and the row is that much taller. A grown
   `Label` with `TextOverflow::Ellipsis` truncates instead of overflowing,
   which the diagram editor's status bar already asks for (section 7).
3. A grown horizontal `VirtualList` in a bounded row is bounded: it gives up
   width to its siblings and mounts only what it shows. Today its natural
   width is its whole extent and the row never bounds it
   (`R/virtual_list.rs:391-393`, `G/layout.md:458-460`).
4. A grown child shrunk far, such as a wrapping label at a few pixels,
   becomes as tall as its wrapping makes it. A child that must keep a width
   has a minimum: `Container::new(child).size_limits(SizeLimits::at_least(..))`
   (`R/containers.rs:186-193`), or its own `Component::size_limits`; one
   that need not grow simply drops its weight.

## 3. Public API

No item is added, removed or changed; the `api/` listings do not change.
The change is behavior (2.1, 2.4) and the private cache (2.3.4). The
rustdoc of public items changes (section 4), and their claims are tested
(section 8).

**Anti-regrowth** (AGENTS.md, "Component API conventions"): nothing new is
offered; the behavior retires the workarounds testers wrote (a fixed-size
`Container` around the bar, a `Column` of bar and percent with taller rows,
a wider column) and the `with_grow` sentence that sent them there.

## 4. Documentation

Text describes P7's commit and names nothing a later part adds (in
particular not P8's cell-address documentation or its clipped-cell
validation).

- **`Element::with_grow`** (L:601-641): the shortfall sentence covers both
  axes and "Rows never shrink a child." goes. New text, in substance: "In a
  row whose width or a column whose height is bounded, a growing child also
  takes its share of a shortfall: when the natural lengths do not fit,
  growing children give up the difference by weight, down to each one's
  minimum, so a growing list or editor fills what the others leave instead
  of keeping its whole natural length, and a growing bar or label beside
  fixed content fits a narrow cell. A row measures each child it shrank
  again at its final width, so a label that wraps gets the height of its
  new lines and the row grows to it, and a label with
  [`TextOverflow::Ellipsis`] truncates. Children without a weight keep
  their natural length, so a container whose ungrown children and grown
  minimums exceed it still overflows, which development validation reports
  as [`ValidationCode::EscapesParent`]; a child that must keep a width keeps
  it as a minimum, for example through [`Container::size_limits`]." A
  second example: a 120 px window with
  `Row::new([ProgressBar::new(ProgressStatus::of(1, 2)).key("bar").with_grow(1),
  Label::new("50%").key("percent")])`, asserting that the percentage's
  right edge is at most 120 (its allocation, not its clipped bounds).
- **`Row`'s type page** (C:902-906, a doc string that does not mention
  growth today): one sentence appended: "Children with an
  [`Element::with_grow`] weight share the spare width, and give up width, down to their minimums, when the natural widths
  overflow a bounded width; a child that gave up width is measured again at
  its final width."
- **`Component::measure`** (`R/runtime.rs:1856-1860`): "A container may
  measure a child at several constraints in one layout pass, as a
  [`Row`](crate::Row) measures a child it shrinks at its natural and at its
  final width. The runtime keeps a component's four most recently used
  measurements until its layout is invalidated and reuses one for equal
  constraints, so a measurement must depend only on the constraints,
  retained data and the theme and font values it reads; every value any
  of its measurements read stays a dependency until the component's
  layout is invalidated. A component asked for more than four constraints
  is measured again, with the same result."
- **`Component::fills_row_height`** (`R/runtime.rs:1739-1744`): "The runtime
  keeps one measurement cache per component" becomes "The runtime keeps a
  component's four most recently used measurements (see
  [`Component::measure`])"; added: "When the row shrinks a fill child,
  its fill measurement uses its final width; its natural measurement stays
  the unbounded one, so it is still asked for two constraints. Whether
  the row shrinks, and by how much, is decided from the natural widths. A
  fill child the row did not shrink takes the width it reports at its fill
  height, and that width counts in the row's reported width, its overflow
  and the space `Justify` places; a fill child the row shrank keeps its
  final width whatever its fill measurement reports. When the natural
  widths fit, growing children share the space left after those widths,
  as before; after a shrink, no child grows back."
- **`MeasureContext::measure`** (L:1018) and **`ArrangeContext::measure`**
  (L:1066, "reusing its cache when constraints match"): "reusing one of
  the child's four most recent measurements at equal constraints, as
  [`Component::measure`] describes".
- **`ProgressBar`'s type page** (at HEAD `R/progress.rs:100-102`; P6 rewrites
  the paragraph around it): after "fills the width it is given": "With a
  growth weight in a [`Row`](crate::Row) it takes the width the row's other
  children leave, wider or narrower than its preference and down to none,
  so a bar beside its percentage fits a narrow table cell; see
  [`Element::with_grow`](crate::Element::with_grow)."
- **`VirtualList`, "A bounded extent"** (`R/virtual_list.rs:378-395`): the
  row sentence becomes "A bounded [`Row`] bounds a vertical list's height,
  which it offers to every child, and a grown horizontal list's width,
  which gives up what its siblings need; an ungrown horizontal list keeps
  its natural width, its whole content." The list of bounding parents adds
  "a grown child of a bounded [`Row`], along a horizontal axis".
- **The `UnboundedCollection` message** (`R/validation/development.rs:661`):
  "… View::fixed or a grown child of a bounded Column, or of a bounded Row
  for a horizontal collection) …". No test pins the message (`git grep`).
- **docs/contracts.md, "Layout, visibility and focus"**
  (`docs/contracts.md:433-450`): "When a column's height is bounded … Rows
  do not shrink: a narrower child could wrap to a height their measurement
  does not repeat." becomes: "When a row's width or a column's height is
  bounded and its children's natural lengths do not fit, the grown children
  give up the difference instead, in proportion to their weights and down
  to each one's minimum, so a grown child fills what the others leave; the
  container then overflows only past the ungrown children's lengths plus
  the grown children's minimums (for a row, the children's final widths
  where they were not shrunk: any unshrunk fill child, grown or not,
  counts with its width at its fill height, which may differ from its
  natural one, and every other ungrown child with its natural width). A
  row measures each child it shrank
  again at its final width and takes its height from those measurements, so
  a child that wraps gets the height of its new line count; a column's
  shrunk children keep the widths they were measured at. A fill child keeps
  today's rule: it is measured at the row's resolved height, capped by the
  offered height, and its result counts in the row's height. Whether a row
  shrinks, and by how much, is decided once from the natural widths; the
  row's reported width, its overflow and the space `Justify` places are
  computed from the final allocations. A fill child the row did not shrink
  is allocated the width it reports at its fill height; one the row shrank
  keeps its final width. When the natural widths fit, growing children
  share the space left after the final allocations, as before; after a
  shrink no child grows back, even if a fill child narrowed. The
  runtime keeps a component's four most recently used measurements, and
  every theme or font value they read stays a dependency until its layout
  is invalidated; measuring and arranging such a row again reuses them."
- **docs/manual.md:174** (the `Column`, `Row`, `Wrap` row): "in a column
  whose height is bounded it also takes a share of a shortfall … (rows do
  not shrink)" becomes "in a row whose width or a column whose height is
  bounded it also takes a share of a shortfall, down to the child's
  minimum, so a grown collection below a toolbar fills the rest and a grown
  bar beside its percentage fits a narrow cell".
- **guide/layout.md:**
  - "Growing and stretching rows and columns" (`G/layout.md:70-104`) is
    renamed "Growing, shrinking and stretching rows and columns" (no link
    targets its anchor, `git grep`) and gains, after the first paragraph:
    "When a row's width or a column's height is bounded and the natural
    lengths do not fit, the grown children give up the difference, by
    weight and down to each one's minimum. A row measures a child it shrank
    again at its final width, so wrapped text makes the row taller and an
    ellipsis label truncates. Children without a weight keep their length,
    so a row of fixed content still overflows, which development validation
    reports. [A progress column](#a-progress-column) puts a bar beside its
    percentage in a narrow table cell."
  - "Large collections" (`G/layout.md:448-460`): "it does not shrink their
    widths, so it never bounds a horizontal one" becomes "and a grown
    horizontal collection gives up the width its siblings need, so a
    bounded row bounds a grown horizontal one too".
  - **New section "A progress column"**, after "An editable list of
    records" and before "Zoomable content" (`G/layout.md:743`): one
    paragraph and a doctest. The paragraph: a `ProgressBar` prefers three
    themed control widths, 192 logical pixels at Standard, which a table
    cell is often narrower than once a percentage sits beside it; with a
    growth weight in a `Row` beside the label, the bar takes what the label
    leaves and, in a narrower cell, gives up the difference down to no
    width, so the percentage stays in the cell; the cell describes the
    record's status with `ProgressBar::new`, since `TableItems::cell` runs
    again whenever the table describes its rows. The doctest, prelude only,
    importing exactly what it uses (R2-14's rule):

    ```rust
    use pert_ui::prelude::*;
    use pert_ui::Viewport;
    use std::time::Duration;

    struct Jobs(Vec<(&'static str, u64)>);
    impl TableItems for Jobs {
        fn len(&self) -> usize { self.0.len() }
        fn key(&self, index: usize) -> Option<String> {
            self.0.get(index).map(|(name, _)| (*name).to_owned())
        }
        fn cell(&self, index: usize, column: &str) -> Option<Element> {
            let (name, done) = *self.0.get(index)?;
            Some(match column {
                "name" => Label::new(name).key("title"),
                "progress" => Row::new([
                    ProgressBar::new(ProgressStatus::of(done, 100)).key("bar").with_grow(1),
                    Label::new(format!("{done}%")).key("percent"),
                ])
                .key("cell"),
                _ => return None,
            })
        }
    }
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(400.0, 200.0, 1.0)?)?;
    let jobs = TableController::new(
        &ui,
        ListSelection::new(&ui, SelectionMode::Single),
        [
            TableColumn::new("name", "Job").width(Length::from(120)),
            TableColumn::new("progress", "Progress").width(Length::from(160)),
        ],
        TableSource::new(Jobs(vec![("render", 42), ("upload", 100)])),
    )?;
    let table = ui.mount_tree(Element::new("jobs", Table::new(&jobs, 28.0)?))?;
    ui.flush(Duration::ZERO)?;
    let row = ui.find_descendant(&table, "render")?.ok_or("mounted")?;
    // The width and the right edge of a component of the row.
    let span = |key: &str| -> Result<(f32, f32), Box<dyn std::error::Error>> {
        let id = ui.find_descendant(&row, key)?.ok_or("mounted")?;
        let bounds = ui.bounds(&id)?.ok_or("laid out")?;
        Ok((bounds.width(), bounds.x() + bounds.width()))
    };
    let ((_, cell_end), (bar, _), (_, percent_end)) = (span("cell")?, span("bar")?, span("percent")?);
    assert!(bar < 192.0, "the bar gave up width");
    assert!(percent_end <= cell_end, "the percentage stays in the cell");
    #[cfg(debug_assertions)]
    assert!(ui.validation_report().issues.is_empty(), "nothing escapes its cell");
    # Ok::<(), Box<dyn std::error::Error>>(())
    ```

    The content keys differ from the column keys, so `find_descendant`
    within a row is unambiguous. `Viewport` is outside the prelude
    (`R/lib.rs:412-494`) and is imported by name, as the chapter's other
    examples do since P1; the sketch is the doctest's substance, and the
    validation check is debug-only because development validation is.
- **The gallery** (section 6) and its README: `apps/gallery/README.md:16`
  (Layout › Layout) adds "in a narrow window the growing tile gives up
  width and wraps"; `:44` (Data & media › Collections) adds the Progress
  column and what narrowing it shows.
- **docs/verification.md**, native evidence: the hand check's row
  (section 9), or the record that it is missing.

## 5. Module membership, prelude closure and dependencies

- **No membership change and no new API,** so L2's closure rule and P2's
  `testing` rule have nothing to re-apply; P2's listing-driven test and
  `every_name` pass unchanged, and the Progress entry records their run.
- **P6:** the fixed `ProgressBar::new(ProgressStatus)` in the `with_grow`
  example, the recipe, the gallery column and tests 1 and 9 (introduction).
- **P8** follows P7 ("P7 before P8: the table chapter shows the progress
  cell laid out", `round-2-fix-plan.md:1333`); P7 provides the laid-out
  cell in guide/layout.md and the gallery. A cell that wraps to more lines
  than the table's row height is clipped by the cell as today
  (`R/table.rs:976-982`); reporting that is P8's validation, not P7's.
- **No dependency** on P2 to P5 beyond their being in the tree.

## 6. Gallery changes and captures

- **Collections table, a Progress column** (`GA/pages/data.rs`). The plan's
  native hand check needs "a table with the progress cell" and no gallery
  table has one (the only one is `GA/pages/data.rs:348-383`). P7 appends
  `TableColumn::new("progress", "Progress").width(Length::new(160.0)?).resizable(true)`
  after Note: not sortable, not movable, so Note keeps its place, the
  existing navigation test's six Right presses still reach Note
  (`GA/tests/data.rs:180-187`) and the move test's first three titles stay
  (`:559-602`). `Measurements::cell` (`GA/pages/data.rs:272-275`) returns,
  for `"progress"`, the recipe's cell: `Row::new([ProgressBar::new(ProgressStatus::of(u64::from(p), 100)).key("bar").with_grow(1),
  Label::new(format!("{p}%")).key("percent")]).key("cell")` with
  `p = number % 101` (`Measurements::number` gives a `u32`,
  `GA/pages/data.rs:254-264`; `ProgressStatus::of` takes `u64`,
  `R/progress.rs:61`, and P6 leaves its signature unchanged); `text` (`:278-289`) returns `format!("{p}%")`, the
  text the cell shows, as its comment requires. At 160 px the cell's 136 px
  are narrower than bar and label (192 + 8 + the label), so the shrink is
  active at the default width. The table's help (`:454-457`) adds: "The
  last column, Progress, draws a bar beside its percentage: drag its edge
  narrower and the bar gives up width so the percentage stays in the
  cell."
- **Layout page, Growth and alignment** (`GA/pages/layout.rs:131-180`): the
  `grow-row` stage's height goes from 56 to 88, so its growing tile, wrapped
  to two or three lines in a narrow window, is not clipped by the stage's
  `Container` (`GA/gallery.rs:699-703`); its help label adds "In a narrow
  window the growing tile gives up width and wraps, and Help stays at the
  end." No other code changes; the row already has the growing tile.
- **Which pages change, and captures.** By the static reading the only
  gallery row with a growing child is the layout page's `grow-row`
  (`git grep with_grow` over `apps/gallery`), whose natural widths fit its
  640 px stage, and no built-in component grows its own children (section
  1). The screenshot mode renders at 1280 × 800 only
  (`GA/screenshot.rs:21`). So at that size **no existing layout changes**;
  the captures differ only where P7 changes the gallery:
  - **Layout › Layout:** the taller `grow-row` stage moves the sections
    below it down by 32 px;
  - **Data & media › Collections:** the added column (scrolled out of view
    at first: the columns already need more than the table's width), which
    changes the horizontal scrollbar's thumb if the table is in the
    capture.

  **Procedure:** in one shared target directory (outside `/tmp`, which is
  RAM), for every page key the gallery registers (`places.page` calls in
  `GA/gallery.rs:244-357`), run `cargo run --locked -p pert-ui-gallery --
  --screenshot <page> window <dir>/<page>.png` at P6's commit (P7's parent)
  and at P7's, and compare the files (byte equality, or decoded pixels if
  the encoder differs). The Progress entry lists every page that differs;
  any page other than the two above is examined before the commit, and an
  unexplained difference is a defect, not a record. The shrink itself is
  shown by section 8's tests, the gallery test and the native hand check,
  not by a capture: no capture mode renders a narrow window.

## 7. Migration manifest

No public signature changes, so no consumer needs a source change for P7 to
compile. The manifest lists every maintained consumer with a growing row
child (`git grep 'with_grow\|Element::spacer'` at fec6a5f) and what changes
for it.

| Consumer | Effect | Change |
|---|---|---|
| Runtime rustdoc: `Element::with_grow` (L:601-641), `Element::spacer` (L:653-665) | the with_grow text (section 4); a spacer has no width to give | `with_grow` text and second example; `spacer` none |
| `crates/pert-ui/examples/diagram_editor/app.rs:782-790` (status row: file, a growing `Ellipsis` message, zoom) | a long message in a narrow window now truncates instead of overflowing past the zoom label | none; its headless session runs in the gate |
| `crates/pert-ui/examples/inventory/app.rs:715-721` (status row: growing wrapping message, totals) | a long message in a narrow window wraps and the bottom region grows to it | none; session in the gate |
| `crates/pert-ui/examples/log_explorer/app.rs:878-884` (toolbar with a growing `ProgressBar`) | in a narrow window the bar gives up width before "shown" overflows | none; session in the gate |
| `crates/pert-ui/examples/live_mixer.rs:446-470`, `:526-536`; `diagram_editor/app.rs:794`, `:803` | growth in columns | none |
| `dynamic_documents` example | no growing row child | none |
| Gallery `GA/pages/layout.rs:143-151` (`grow-row`) | shrinks and wraps in a narrow window | stage height and help (section 6) |
| Gallery `GA/pages/data.rs` | the Progress column (section 6) | added |
| Gallery tests `GA/tests/layout.rs:1118-1148`, `GA/tests/data.rs:142-227`, `:559-602` | unchanged at their viewports (the grow-row fits; the new column is last) | none; a new test (section 8, test 9) |
| `T/foundation/unbounded_collections.rs:420-449`, `rows_keep_their_lengths` | encodes the old rule: two grown 300 px leaves in a 400 px row keep 300 each | **migrated:** replaced by section 8's test 5, which asserts 200 and 200 for this configuration first; the file's module comment (`:1-3`) says "a bounded row or column" |
| Other facade tests with a growing row child: `T/foundation/flex_layout.rs` (`:73-93`, `:137-152`, `:206-228`, `:254-320`), `T/foundation/row_stretch.rs:50-53`, `:201-210`, `T/collections/controlled_rows.rs:74`, the rows around columns in `T/foundation/unbounded_collections.rs` | each row fits, has no room to give, or is a column's child; expected unchanged | none expected; a test that fails is examined, and one that encodes "rows never shrink" is migrated and named in the Progress entry |
| Guide: `G/layout.md:97`, `:571`, `:657` | a stretched row in a column; the editable list's growing title (it may shrink in a narrow viewport; P6's bar there has no weight and keeps its width) | none; the sections of section 4 |
| contracts.md, the manual, the gallery README | section 4 | edited |
| Runtime README (`crates/pert-ui-runtime/README.md:1334`, a `ToolGroup::spacer`), the facade README | no `Element` growth | none |
| Native checks (`apps/native-checks`), platform code, `crates/pert-ui-wgpu`, `crates/pert-ui-cpu` | no growing row child (`git grep`) | none |
| Diagnostics fixtures (`tests/diagnostics/ui/`) | none use growth | none; snapshots run in the gate |
| `plans/usability/task4-app` | no growth weight (`git grep`; its one `Row`, `src/lib.rs:109`, has none) | none: no fixture check |
| Frozen: submissions and probes under `plans/usability/runs/` and U (for example U `r2-5-head-claude-2/work/src/lib.rs:321-326`'s wrapper), `plans/usability/api-audit/inputs/`, `plans/archive/` | their workarounds keep working | not migrated |

## 8. Tests

- **New-behavior tests** each name the assertion that detects a wrong
  implementation and a compiling mutation that must make it fail.
- **Contract and compatibility tests** check what P7 keeps.

Facade tests join the `foundation` binary (test 14(d), the store test, is the one
runtime unit test): tests 5 and 10 in
`T/foundation/unbounded_collections.rs`, beside the column tests and the
list helpers (`mount` settles three flushes, `:79-92`; `unbounded`,
`:122-130`), the others in `T/foundation/flex_layout.rs`, which has the
`fixed`, `layout`, `bounds` and `no_gap` helpers
(`T/foundation/flex_layout.rs:10-51`). Test-local helpers: a `Counting` leaf
(natural size, optional minimum, an `Rc<Cell<u32>>` incremented in
`measure`) and a `Wrapping(area)` leaf whose measure returns
`(300, area / 300)` without a width and `(w', area / w')` for
`w' = min(w, 300)` with one. Every row states its gap: a row's default gap
is `ItemGap`, 8 px at Standard (C:831-834), and only test 1 and the
doctests keep it. Validation
assertions are under `#[cfg(debug_assertions)]`, as
`T/foundation/unbounded_collections.rs:122-130` does, since development
validation is a debug-build service; geometry assertions run in both
profiles.

### New-behavior tests

1. **A bar beside its percentage in a 167 px cell (R2-21).** A `Table` in a
   400 × 200 viewport with one 167 px column whose cell is the recipe's
   `Row` (`ProgressBar::new(ProgressStatus::of(42, 100))` growing, `"42%"`);
   the cell's content is 143 px wide at Standard. Named assertions: the
   percentage's right edge is at most the row's right edge; the bar's width
   plus the gap plus the percentage's width equals the row's width (within
   0.01); in debug builds no `EscapesParent` issue. Mutation: the
   shortfall stays a column's only (`if column && grows`); the percentage
   ends past the row and the issue is reported.
2. **A shrunk child is measured again (2.1.4-6).**
   - (a) A `Column` in a 200 × 100 viewport holding
     `Row::new([Wrapping(3000.0).key("text").with_grow(1), fixed("side", 50.0, 10.0)]).gap(no_gap()?)`.
     The deficit is 150, so `text` is 150 wide and measured at 150: height
     20. Named assertions: `text`'s bounds `[0, 0, 150, 20]`, the row's
     height 20 and `side` centered at `y` 5. Mutation: no second
     measurement (the natural height 10 stays; the row is 10 high).
   - (b) **The tallest-child rule is kept uncapped** (2.1.6): the same row
     as the root of a 200 × 30 viewport with `Wrapping(6000.0)`, whose
     final measurement is 40 high, above the 30 px offered. Named
     assertions: `text`'s bounds `[0, 0, 150, 40]` and `side` at `y` 15,
     centered against 40. Mutation: the row's natural height capped by the
     offered height, as revision 1 wrote (`side` at `y` 10).
3. **A fill child follows the new height.** A `Column` in a 140 px wide
   viewport holding `Row::new([Wrapping(3000.0).key("text").with_grow(1),
   Divider::new().vertical().key("divider"), fixed("side", 50.0, 10.0)]).gap(no_gap()?)`.
   A vertical divider is 25 px wide (twice `SectionGap`, 12, plus a 1 px
   line, `R/views.rs:935-940`, `:995-1006`) and measures `ControlHeight`,
   32 px, without a height offer (`R/runtime/theme.rs:205`), so the deficit
   is 235, `text` is 65 wide and about 46 high. Named assertions: `text` is
   taller than 32, and the divider's height equals the row's height, which
   equals `text`'s. Mutation: fill children measured at the tallest height
   of the natural measurements (the divider and the row's fill offer stay
   32).
4. **An ellipsis label truncates.** A `Column` in a 160 px viewport holding
   a row of a growing
   `Label::new("A status message much longer than its row").overflow(TextOverflow::Ellipsis)`
   and `Label::new("100%")`. Named assertions: the message's `truncated`
   inspection property is true and the second label's right edge is at most
   160. Mutation: as test 1 (`truncated` false, the label past the
   window).
5. **Weights and minimums, as the column** (in
   `T/foundation/unbounded_collections.rs`, replacing
   `rows_keep_their_lengths`, `:420-449`, beside the column test it
   transposes). First that test's configuration, two grown 300 px `Leaf`s
   in a 400 px row: widths 200 and 200 (they were 300). Then the
   transposition of `grown_children_shrink_by_weight_down_to_their_minimums`
   (`:163-195`): a 400 px root row of a fixed 100, `one` 300 with weight 1
   and `three` 300 with weight 3, no gap. Named assertions: widths 100, 225
   and 75; with a 250 px minimum on `three`, widths 250 and 50, the column
   test's heights. Mutations: the row shares the deficit equally (150 and
   150); the row ignores minimums (`three` at 75).
6. **Residual overflow stays and is reported.** A root `Column` in a 400 ×
   100 viewport holding
   `Container::new(row).padding(Length::from(20)).preferred_size(Size::from([240, 60]))`,
   so the row is 200 wide and the 20 px padding leaves room inside the
   container's clip for validation to see a child leave the row; no gap:
   - (a) `fixed` `a` 150 and `b` 100 and a growing `c` 40 wide: named
     assertions `c` 0 wide, `b`'s right edge 250 from the row's left edge,
     and in debug builds an `EscapesParent` issue naming `b`'s inspection
     id. Mutation: children without a weight shrink too (`b` fits).
   - (b) `fixed` `a` 50, a growing `b` 200 wide with a 150 px minimum and
     a `fixed` `d` 20 after it: natural 270, deficit 70, and `b` gives only
     its 50. Named assertions: `b` 150 wide, `d` at 200 from the row's left
     edge (220 to 240 in the window: inside the container's padding, so
     visible, and outside the row, which ends at 220), and in debug builds
     the `EscapesParent` issue for `d`; validation reports only a nonempty
     visible rectangle (`R/validation/development.rs:496`, `:613-624`), which
     revision 2's geometry (`d` from 270, past the container's 240 px clip)
     did not give. Mutation: minimums ignored in the shortfall (`b`
     computed at 130, `d` placed at 180, inside the row, no issue). The width
     assertion alone would not detect it: `arrange_node` constrains every
     allocation to the child's hard limits (`R/runtime/tree.rs:1704-1708`)
     and restores `b` to 150, so `d`'s position is what shows the wrong
     calculation.
7. **Nested shrinking rows measure each child once per width (P7-D1).** A
   root row in a 300 × 40 viewport: `[fixed 100, R1.with_grow(1)]`, `R1 =
   [fixed 50, R2.with_grow(1)]`, `R2 = [fixed 50, Counting(500 wide).with_grow(1)]`,
   no gaps, every row `Align::Stretch` (so every level offers the same 40 px
   height and the count measures widths only). The widths resolve to 200,
   150 and 100. Named assertions: the leaf is 100 wide and was measured
   exactly **twice** in the first flush (its natural width and its final
   one), and not again in a second flush with nothing changed. Mutation:
   a component keeps only its latest measurement, as today; the author's
   trace of measure, arrange and the nested arranges gives eight
   measurements, and the assertion is only that it is more than two. A
   two-entry store also passes this test (equal heights); test 13 is the
   case it fails.
8. **A fitting row measures once.** A root row in a 200 px viewport of a
   growing `Counting` 50 wide and `fixed` 50, no gap: named assertions the counting
   child is 150 wide (growth, unchanged) and measured exactly once.
   Mutation: every growing child is measured again at its final length
   (twice).
9. **The gallery's progress column** (`GA/tests/data.rs`, a new test beside
   `:142`): the collections page at 960 × 2000 as the existing tests open
   it; in row `sample-1`, the `progress` cell's `percent` label ends at or
   before the cell content's right edge, the bar is narrower than 192 px,
   and the validation report is empty. Mutation: as test 1.
10. **A grown horizontal list in a bounded row.** Mounted with `mount`, a
    300 × 100 root row of a `Leaf` 100 px wide and a horizontal `VirtualList` of
    200 items of 24 px (`.orientation(Orientation::Horizontal)`) with
    weight 1 and `.gap(Length::new(0.0)?)`, whose unbounded measurement is
    its whole 4800 px extent (`R/virtual_list.rs:2092-2111`). Named
    assertions: the list is 200 wide and mounts fewer than 30 items; in
    debug builds no `UnboundedCollection`. Mutation: as test 1 (the list is
    4800 wide, mounts every item and is reported). This is the claim the
    `VirtualList` and guide text of section 4 now make.
11. **Doctests:** `Element::with_grow`'s row example and the guide's "A
    progress column" (section 4). Named assertions: the percentage's right
    edge within the window or the cell, and the bar below 192 px. Mutation:
    as test 1.
12. **A shrunk fill child (G1, G3).** A root row in a 200 × 40 viewport,
    no gap, of a growing `FillProbe` and `fixed` 50. `FillProbe` is a test
    component with `fills_row_height` true, natural width 300, height
    `available.height().unwrap_or(20)` and a counter; it is asked for A =
    `(None, None)` and its fill measurement `(150, 20)` (2.1.4-5), in the
    row's measurement and again in its arrangement (the root is measured and
    arranged at the viewport size). A composite variant holds one
    `Counting` child that it measures at the constraints it receives. Named
    assertions: `FillProbe` and the composite's child are each measured
    exactly twice in the first flush and not at all in a second, and
    `FillProbe` is 150 wide and 20 high. Mutations: a component keeps only
    its latest measurement (four); a shrunk fill child is also measured at
    `(150, None)`, as revision 2 specified (three, and four under a
    two-entry store).
13. **Two heights in one pass, and retention across passes (G1, G2).** As
    `T/foundation/flex_layout.rs:206-228` does, a root `Column` holding a
    row, no gap, of a growing `Counting` 300 wide and 10 high and `fixed`
    50 in a 200 px viewport. The column measures the row with an unbounded
    height (C:516) and arranges it at its 10 px height (C:887-892), so the
    child is asked for `(None, None)`, `(150, None)`, `(None, 10)` and
    `(150, 10)`. The row is then redescribed through `reconcile_children`
    with `justify(Justify::End)`: its layout is invalidated (C:848-856),
    its children's are not (their `reconcile` returns `None`,
    `R/runtime.rs:1105-1110`). Named assertions: the counter is 4 after the
    first flush and still 4 after the redescription's flush, and the child
    is still 150 wide. Mutations: a two-entry store (8: the measurement's
    pair was replaced by the arrangement's); a component keeps only its
    latest measurement (8).
14. **Dependencies survive eviction (Codex points 1 and 6).**
    - (a) **One pass, metric.** A `Column` in a 200 px viewport holding a
      row, no gap, of two growing children with equal weights: `Probe`,
      whose measurement without a width is `(4 × ControlWidth, 10)` and
      with one `(w, IconSize)`, and a 200 px leaf `q`. The deficit is
      `4 × ControlWidth`, shared 1:1, so `Probe` is `2 × ControlWidth`
      wide: 128 at Standard. The test then replaces the Standard metrics
      with `ControlWidth` 72 (`ThemeManager::set_metrics` with
      `ThemeMetrics::for_density(Density::Standard).with_metric(..)`,
      `R/runtime/theme.rs:1108-1121`, `:299`) and flushes. Only the natural
      measurement read `ControlWidth`, and nothing else in the tree reads
      it (the row's gap is a fixed length). Named assertions: `Probe` 144
      and `q` 56 wide after the change. Mutation: the masks reset before
      each computation as today (`R/runtime/tree.rs:1673-1674`; after the
      final measurement they hold only `IconSize`, and `Probe` stays 128).
    - (b) **Several passes, an ancestor's retained result, metric (Codex's
      scenario).** A root row in a 300 × 40 viewport, no gap, of a growing
      `Column` holding `Probe` (without a width `(2 × ControlWidth, 10)`,
      reading `ControlWidth`; with one `(w, 10)`, reading nothing) and a
      growing test leaf `Sized(w)` whose `reconcile` adopts a new width and
      returns `Invalidation::Layout`. `Sized` is redescribed at 200, 210,
      220 and 230 with a flush after each: the column's natural
      measurement stays cached (the column is never invalidated), its final
      width changes each pass (114, 109, 104, 99), and the probe is asked
      for a new width each pass, so by the fourth pass its unbounded entry
      is the least recently used and replaced. The test then sets
      `ControlWidth` 72 and flushes. Named assertion: the column is 107
      wide (its natural width 144, deficit 74 shared 1:1). Mutations: the
      masks reset before each computation (today), or reduced to the
      entries still kept: the probe is not invalidated, the column's
      natural size stays 128, and the column stays 99 wide.
    - (c) **The same with a font.** As (b), but `Probe` measures without a
      width by preparing `"iiii WWWW"` in the body style and returning its
      width, and counts those unbounded measurements; after the four
      passes the test registers a monospace font and makes it the body
      role's default (`FontManager::set_default`, as
      `T/presentation/fonts_icons.rs:150-175` does) and flushes. Named
      assertion: the unbounded-measurement counter advanced by one (the
      probe was invalidated and measured again). Mutation: as (b) with
      `font_layout[1]` (the counter does not move).
    - (d) **The store** (a crate-private unit test in the runtime): five
      distinct insertions replace the least recently used; a hit refreshes
      an entry (insert A, B, C, D, hit A, insert E: B is gone, A kept); the
      masks keep an evicted entry's bits; `invalidate_layout` clears the
      entries and both masks. Mutations: a hit that does not refresh (A
      gone); eviction that clears the evicted entry's bits.
15. **A fill child's final height sets the row's height (Codex point 2).**
    A root row in a 200 × 20 viewport, no gap, of a growing
    `Wrapping(1500.0)`, a vertical `Divider` and `fixed("side", 50.0,
    10.0)`. Natural widths 300 + 25 + 50: the text gives up 175 and is 125
    wide and 12 high. The tallest provisional height is the divider's 32,
    so the fill offer is `min(32, 20) = 20` and the divider's final height
    20. Named assertions: the divider 20 high, `side` at `y` 5 and the text
    at `y` 4, centered against 20. Mutation: the row's height taken before
    the fill measurement, as revision 2 specified (the divider's natural 32
    counts: `side` at `y` 11).
16. **A fill child wider at its fill height, no shortfall (2.1.11).** A root row in a
    250 × 40 viewport, no gap, of a test fill component `WideFill`
    (`(100, 10)` without a height, `(300, h)` with one), `fixed` 50 and a
    growing `Counting` 50 wide. The natural pass fits (200 ≤ 250), so no
    shortfall applies. Named assertions, today's geometry: `WideFill` 300
    wide, the counting child 50 wide at `x` 350, and the counting child
    measured once (no second measurement). Mutation: the shortfall
    computed from the widths after the fill measurement (the counting child
    shrinks to 0).
17. **More than four constraints: the fallback (G1's limit).** A test
    container `Cycler` measures its one `Echo` child at a list of distinct
    width constraints, twice over, in its `measure`, and checks each result.
    `Echo` is a counting probe whose answer depends on the constraint:
    `(w, 1000 / w)` for `Some(w)`. With widths 10, 20, 30 and 40 the
    expected sizes are `(10, 100)`, `(20, 50)`, `(30, 33.33…)` and
    `(40, 25)`, and with 50 added, `(50, 20)`. Named assertions: with four
    constraints `Echo` is measured four times; with five, ten times; in
    both, every result `Cycler` receives equals the expected size for its
    constraint. Mutations: a store of three (eight measurements for four
    constraints); a lookup that, on a hit, returns the size of the most
    recently inserted entry instead of the matching one (the same counts,
    and `Cycler`'s size check fails).
18. **A fill child wider at its fill height, with a shortfall (2.1.11,
    Codex's first configuration).** A test `Reporter` component measures
    its one child at `(Some(250), None)`, records the width the child
    reports in a cell of its own, and returns a width of 250 (the child's
    height) as its own measurement; it arranges the child at 250 × that
    height and overrides `clips_children` to return `false`, so the child's
    overflow is not clipped at the Reporter (the default is `true`,
    `R/runtime.rs:1624-1629`, and the descendant clip is intersected with
    the allocation, `R/runtime/tree.rs:1709-1715`). The Reporter is the
    child of a `Container` 290 px wide (`preferred_size`) with 20 px
    padding, in a viewport wider than 290, so the Reporter is allocated 250
    at `x` 20 and the container's clip ends at 290. The child
    is a row, no gap, of `WideFill` (`(100, 10)` without a height, `(300,
    h)` with one) and a growing `Counting` 300 wide. Named assertions: the
    counting child is 150 wide at `x` 300 from the row's left edge,
    `WideFill` 300 wide, the row's reported width 450, and in debug builds
    an `EscapesParent` issue for `WideFill`, whose visible part runs 20 px
    past the row into the container's padding (the counting child, from
    320 in the window, is clipped away entirely and cannot be reported).
    Mutations: the
    reported width taken as `L` when the natural deficit was absorbed (250);
    `WideFill` placed at its natural width (the counting child at `x`
    100). **Variant (b), a growing fill child at its minimum** (Codex's
    fourth recheck): the same fixture with `WideFill` growing (weight 1,
    minimum width 100) beside the growing `Counting` 300 wide with minimum
    0. The natural deficit is 150; `WideFill` cannot give up width, the
    counting child shrinks to 150. Named assertions: `WideFill` 300 wide,
    the counting child 150 wide, the row's reported width 450. Mutation:
    a grown fill child counted with its minimum instead of its fill-height
    width (reported width 250).
19. **A fill child narrower at its fill height, with a shortfall (2.1.11,
    Codex's second configuration).** The same `Reporter` fixture (250 px,
    the child's reported width recorded separately, `clips_children`
    false, inside the 290 px container with 20 px padding), the row with
    `justify(Justify::Center)`, no gap, of `NarrowFill` (`(300, 10)` without
    a height, `(100, h)` with one) and a growing `Counting` 100 wide. Named
    assertions: the counting child 0 wide, `NarrowFill` 100 wide at `x` 75
    (the 150 left over centered), the row's reported width 250, and no
    validation issue. Mutations: no spare space assumed after a shortfall
    (`NarrowFill` at `x` 0); the reported width from the natural floor (300);
    growth after the shortfall (the counting child 150 wide).

### Contract and compatibility tests

- **Columns are unchanged:** every other test of
  `T/foundation/unbounded_collections.rs` passes with its assertions
  unchanged, including the f64 case (`:223-249`), the floor with gaps
  (`:309-333`), stretch and space-between of shrunk children (`:335-353`),
  the nested columns (`:398-418`) and the bounded row beside a vertical list
  (`:452-472`): the shared function (2.2) gives the column's results.
- **Rows that fit, and rows at their minimums, are unchanged:** every test
  of `T/foundation/flex_layout.rs` passes unchanged, among them growth by
  weight up to a maximum (`:73-93`), the growing list that fills the rest
  of a row (`:254-270`), the huge spare space (`:272-288`) and
  `hard_minimums_that_do_not_fit_leave_no_spare_space` (`:290-320`), whose
  grown child is already at its minimum.
- **Stretched rows and dividers:** `T/foundation/row_stretch.rs` unchanged,
  including the divider that follows a column-shrunk row (`:163-187`).
- **The facade's other areas, the runtime's tests and every doctest**
  (`cargo test --workspace`, `--doc`).
- **The packaged examples' headless sessions** (all five), the headless
  gallery (`cargo run --locked -p pert-ui-gallery -- --headless`), the
  gallery's tests including `the_layout_page_grows_justifies_and_stretches_its_flex_samples`
  (`GA/tests/layout.rs:1118-1148`), and section 6's captures.
- **Release profile:** `cargo test --locked --release -p pert-ui --test
  foundation`, since the new tests split assertions on
  `debug_assertions`.
- **No API change:** `api/generate.sh` leaves `api/` unchanged; P2's
  listing-driven test passes.

## 9. Scope, version and evidence

- **`api/` diff:** none.
- **Breaking:** behavior only, as the plan says.
- **CHANGELOG** for 0.3.82, citing this checkpoint:
  - `Breaking (R2-21, behavior):` a grown child of a row whose width is
    bounded now gives up width when the children's natural widths do not
    fit, by weight and down to its minimum, as a grown child of a bounded
    column gives up height since 0.3.25. The row measures each child it
    shrank again at its final width, so a wrapping label gets its new
    height and the row grows to it, and an ellipsis label truncates; a
    grown horizontal collection in a bounded row is now bounded. The row
    overflows only past its ungrown children's natural widths plus its
    grown children's minimums, where any fill child the row did not
    shrink, grown or not, counts with its width at its fill height
    instead; development validation still reports that overflow as
    `EscapesParent`. Whether the row shrinks, and by how much, is decided
    once from the natural widths; its reported width, overflow and
    justified spare space are computed from the final allocations. An
    unshrunk fill child is allocated the width it reports at its fill
    height; a shrunk one keeps its final width. Growth when the natural
    widths fit works as before; after a shrink no child grows back. Rows
    that fit, rows
    without a growing child and rows offered an unbounded width are
    unchanged. Replacement for a
    layout that relied on a grown child keeping its natural width: drop
    its growth weight, or keep a minimum with
    `Container::new(child).size_limits(SizeLimits::at_least(..))` or the
    component's own `size_limits`.
  - `Changed (P7-D1):` each component keeps its four most recently used
    measurements instead of one, so a row that measures a shrunk child at
    its natural and its final width, measured and arranged at one height
    or two, reuses them; more than four constraints are measured again,
    with the same result. `Component::measure`,
    `Component::fills_row_height`, `MeasureContext::measure` and
    `ArrangeContext::measure` say so.
  - `Fixed (same kind, P7-D1):` a component's theme and font dependencies
    from its measurements stay recorded until its layout is invalidated.
    Before, they were reset before each new measurement, so an ancestor's
    cached measurement computed from an earlier result of the component
    could miss a metric or font change that only that result had read.
  - `Documentation (R2-21):` `Element::with_grow` (with a row example),
    `Row`, `ProgressBar` (a grown bar beside its percentage),
    `VirtualList`'s bounded extent and the `UnboundedCollection` message,
    contracts.md, the manual, and guide/layout.md: "Growing, shrinking and
    stretching rows and columns", the collections paragraph and the new "A
    progress column".
  - `Gallery:` the Collections table's Progress column; the Layout page's
    growth stage is taller and says what a narrow window shows.
  - No public item is added, removed or changed.
- **Version:** 0.3.82 after P6's 0.3.81: the workspace version, the internal
  version requirements and `Cargo.lock`. If the order changes, the heading
  follows the actual version.
- **Gate** (the plan's, `round-2-fix-plan.md:1039-1041`): the public-API row
  for a behavior change across packages: the workspace checks (formatting,
  workspace Clippy, workspace tests with section 8, the five packaged
  examples' headless sessions, doctests, warning-free rustdoc, the
  diagnostics snapshots), the API listing gate (expected: no diff), the
  headless gallery, section 6's captures compared before and after, the
  release-profile `foundation` run, the mutations of section 8 recorded in
  the Progress entry, and Codex's implementation review. No fixture check
  (section 7).
- **Native evidence:** a Linux hand check on an unlocked desktop with a
  hardware GPU, `cargo run --locked -p pert-ui-gallery`:
  - Layout › Layout, Growth and alignment: narrow the window until the
    `grow-row` stage is narrower than its tiles' natural widths; the
    growing tile gives up width and wraps, Help stays at the stage's end,
    and widening restores one line;
  - Data & media › Collections: scroll the table to Progress and drag its
    right edge narrower; the bar narrows and the percentage stays in the
    cell; widen it and the bar grows past its preference.

  Recorded with the platform in the Progress entry and
  `docs/verification.md`'s native evidence; if not run, both record it as
  missing. Windows and macOS stay missing (NATIVE-01 to NATIVE-05).
- **Persistence, real time, backends:** no persisted schema, live-data
  endpoint or backend type is involved; the GPU and CPU renderers draw the
  same display list.
- **TODO.md:** the entries touching P7's files are NATIVE-05 (the example
  windows and the gallery table, no native evidence; not resolved) and the
  2026-10-28 entry on examples with unused imports (the guide chapters it
  names do not include layout.md; P7's new examples import exactly what
  they use). Neither is resolved by P7.

## Corrections to the plan

- **The second measurement's cost.** D7's "a layout pass measures a shrunk
  child twice" holds per row computation, but a row computes in both
  `measure` and `arrange` (C:880-898) and the component cache holds one
  measurement (`R/runtime.rs:858`, `R/runtime/tree.rs:1661-1690`), so the
  natural and final measurements evict each other and nested shrinking
  rows multiply the count (section 8, test 7). P7-D1 keeps four
  measurements, least recently used replaced, with dependency bits sticky
  until the layout is invalidated (2.3.4).
- **More text becomes false than the plan lists.** Besides `with_grow`,
  guide/layout.md and `ProgressBar`: `VirtualList`'s bounded-extent
  paragraph (`R/virtual_list.rs:391-393`) and guide/layout.md:458-460
  (a bounded row never bounds a horizontal collection), contracts.md:441-442,
  manual.md:174, `Component::fills_row_height`'s cache sentence
  (`R/runtime.rs:1741-1742`), `Row`'s page and the validation message's
  list of bounding parents. Section 4 changes each.
- **The native hand check's table.** No gallery table has a progress cell
  at fec6a5f (the only table, `GA/pages/data.rs:348-383`, shows text);
  section 6 adds the column the plan's hand check names.
- **The replacement usage.** "A child that must keep its width sets a
  minimum through `SizeLimits`": an `Element` has no limits builder; the
  route is a `Container` around it with `size_limits`, or the component's
  own `size_limits`, or, where growth is not needed, no weight.
- **The captures.** The screenshot mode renders only 1280 × 800
  (`GA/screenshot.rs:21`), where no existing gallery row overflows, so the
  comparison shows P7's own gallery changes and guards against unintended
  ones; the narrow behavior is evidenced by tests and the hand check.
- **A test encodes the old rule.** `rows_keep_their_lengths`
  (`T/foundation/unbounded_collections.rs:420-449`) asserts that two grown
  300 px children of a 400 px row keep 300 each; the plan's test list does
  not name it. Section 8's test 5 replaces it (manifest, section 7).
- **Unchanged rows, precisely.** "Only rows whose natural widths do not fit
  change" also needs a growing child and a bounded offered width (2.1.2): a
  row in a horizontal scroll area and another row's natural pass keep
  natural widths.

## Findings outside this part

None outside the part's scope. One same-kind defect in code P7 changes is
fixed here: `measure_node` resets a component's measurement dependency
masks before each new measurement (`R/runtime/tree.rs:1673-1674`), so an
ancestor's cached measurement computed from an earlier result of the
component is not invalidated by a metric or font change only that result
read; P7-D1's sticky bits fix it (test 14(b), CHANGELOG `Fixed:`). The rows
that place application elements (`ToolGroup`, window actions) apply the
rule to weights the application gives; no built-in component weights its
own children (section 1).

## Not in this part

- **A shortfall for `Wrap` or `Grid`:** neither grows children
  (`docs/contracts.md:449-450`); no run needed it.
- **A smaller natural width for `ProgressBar`** (D7's alternative A): bars
  that do not grow keep their 192 px preference.
- **Reporting a cell whose content is taller than its row:** P8 (D8).
- **A narrow-window capture mode** for the gallery's screenshots.

## Decisions

- **P7-D1: which measurements a component keeps, and how their
  dependencies are kept.** Revised twice after Codex's review: revision 1
  recommended two entries, revision 2 measurements retained per pass with
  per-entry masks; revision 3 settles on the simplest rule that meets the
  measured cases and is never wrong outside them.
  - **Recommended: four entries, least recently used replaced, dependency
    bits sticky until the layout is invalidated** (2.3.4). Fixed size
    (about 100 bytes per component, no allocation), constant-time lookup
    and insertion, nothing aged or swept, and no union to maintain:
    eviction never removes a bit, so no retained result anywhere, an
    ancestor's included, loses a dependency (test 14). Four is what the
    measured cases need: two constraints per child per row computation
    (2.3.2, with the fill child's unbounded final-width measurement
    removed), at two heights when a row is measured and arranged
    differently (test 13). Beyond four the documented fallback recomputes,
    always correctly (test 17).
  - **Alternative A: keep one entry,** with the sticky bits. No memory
    change; the repetition multiplies with nesting (test 7's mutation), and
    a shrunk child is measured twice per row computation with nothing
    reused between them: a row in a column computes four measurements of
    it in the first pass (test 13) and four more whenever it is laid out
    again, where four entries reuse all four.
  - **Alternative B: two entries.** Half the storage; covers rows measured
    and arranged at one height, not a row in a column (test 13: eight
    measurements instead of four).
  - **Alternative C: measurements retained for the current and previous
    pass** (revision 2). No repetition within a pass for any number of
    constraints, but storage proportional to the distinct requests, an
    overflow list whose capacity must be reclaimed, lazy ageing, and masks
    to recompute or keep sticky; more mechanism than the measured cases
    need, for a guarantee beyond them.
  - **Alternative D: a per-pass memo table** keyed by component and
    constraints, cleared after each layout, beside today's entry. A
    reusable table keeps its capacity, so it allocates only while it grows,
    but it is a second cache beside the node's and gives no reuse across
    passes.
  - **Alternative E: measure again only while arranging.** No extra
    measurement in `measure`, but the row would report its height from the
    natural measurement, the wrong height D7 exists to fix.
  - **The dependency rule is not optional with any option above one
    entry**, and the sticky form is also a same-kind fix of today's code:
    an ancestor's cached measurement computed from a child's earlier result
    is not covered by the child's masks once the child is measured again
    at other constraints (test 14(b) fails today too).

## Review record

- **Revision 1 (2026-10-28):** written from the code at fec6a5f (library
  code as at 84bb60f, 0.3.76).
- **Revision 1, Codex review (2026-10-28):** needs revision, one blocking
  point, five should-fix points and a nit; record
  `~/pert-ui-usability/codex-prompts/codex-design-P7.out`. Each point was
  checked against the committed code (fec6a5f; P2's later commit changes
  none of the cited code); none was rejected.
  1. **Dependencies of retained measurements (blocking):** confirmed.
     `measure_node` resets the node's measurement read masks before each
     miss (`R/runtime/tree.rs:1673-1674`), and the theme and font
     notifiers invalidate only through those masks
     (`R/runtime/theme.rs:952-960`, `R/runtime/fonts.rs:19-23`), so a second
     retained entry would lose the first's dependencies. Revision 2 gives
     every retained measurement its own masks, makes the node's masks their
     union, recomputed on insertion and drop and cleared on invalidation,
     counts them in the storage cost (2.3.4), and adds test 14 (metric and
     font dependencies read by different entries, and the store's drop and
     clear).
  2. **The tallest-child rule:** confirmed; C:503 caps only the height
     offered to fill children, while the row's natural height is the
     tallest child's, uncapped (C:545-547, :556, :718-721;
     `docs/contracts.md:445-449`). Revision 2 states that in section 1,
     keeps it after the second measurement and caps only the fill offer
     (2.1.5-6), and adds test 2(b), a shrunk child taller than the offered
     height.
  3. **The fill child's three constraints:** confirmed; with two entries
     replaced oldest first, repeating A, B, C misses all three, and
     `fills_row_height` is public, so the child may be a composite.
     Reassessed together with point 1 (2.3.3): two entries bound the cost
     only for ordinary children of a row measured and arranged at one
     height, three or four only for one more case each. Revision 2
     recommends measurements retained per layout pass (2.3.4), states the
     guarantee as G1 to G3 with what is not guaranteed (2.3.5), rewrites
     P7-D1 with the fixed capacities as alternative B, and adds test 12 (a
     shrunk fill child and a composite one, three constraints, measured
     three times) and test 13 (retention across passes). Test 7 is kept and
     says that it does not distinguish two entries.
  4. **Test 3's geometry:** confirmed; a vertical divider is 25 px wide and
     32 px high without an offer (`R/views.rs:935-940`, `:995-1006`,
     `R/runtime/theme.rs:205`). Revision 2 uses a 140 px viewport, so `text`
     is 65 wide and about 46 high, above 32, and the mutation leaves the
     divider at 32.
  5. **Test 6(b)'s mutation:** confirmed; `arrange_node` constrains every
     allocation to the child's hard limits (`R/runtime/tree.rs:1704-1708`).
     Revision 2 adds a following child `d` and asserts its position (250;
     the mutation places it at 180).
  6. **Test 10's gap:** confirmed (C:831-834, `R/runtime/theme.rs:199`).
     Revision 2 sets a zero gap there and states every test row's gap
     (section 8's introduction; tests 6 and 8 also lacked it).
  7. **Nit, the gallery's integer conversion:** confirmed
     (`GA/pages/data.rs:254`, `R/progress.rs:61`); revision 2 writes
     `ProgressStatus::of(u64::from(p), 100)`.
- **Revision 2, Codex recheck (2026-10-28):** needs revision, one blocking
  point and five should-fix points; record
  `~/pert-ui-usability/codex-prompts/codex-design-P7-r2.out`. Each was
  checked against the committed code (fec6a5f; the commits up to 0f9638f
  Codex read add only designs); none was rejected. Following the author's
  guidance, revision 3 takes the simplest rule that meets the measured
  cases.
  1. **An ancestor's retained result loses a child's dependency
     (blocking):** confirmed. `MeasureContext::measure` forwards to the
     child and records nothing in the caller (L:1018-1027), a hit skips the
     subtree (`R/runtime/tree.rs:1661-1666`), and the notifiers see only
     the child's masks (`R/runtime/theme.rs:952-960`). Revision 3 makes the
     measurement bits sticky until the component's layout is invalidated,
     never reduced by eviction, and clears them in `invalidate_layout`
     (2.3.4); test 14(b) is Codex's column-and-sibling scenario over four
     passes with a metric change, 14(c) the same with a font. The same
     hazard exists in today's one-entry code, so the fix is also recorded
     as a same-kind fix (findings, CHANGELOG).
  2. **Final fill results:** confirmed (C:500-508, :545-547, :556). Revision
     3 restates 2.1.4-6: a shrunk fill child is no longer measured at its
     final width without a height (its provisional height stays the
     natural one, as today); the fill offer is today's; the row's natural
     height is the maximum of the final child heights, fill measurements
     included. Height-dependent fill widths are settled in 2.1.9: the
     shortfall uses the natural pass's widths, the row never iterates, and
     a wider width at the fill height is overflow, as today. G3 becomes two
     constraints per child per row computation. Tests 15 (a divider whose
     fill height, 20, sets the row's height) and 16 (a fill child 100 wide
     without a height and 300 with one, laid out as today) are added, test
     12 is restated.
  3. **Retention, storage and maintenance:** confirmed for revision 2's
     unbounded per-pass store. Revision 3 replaces it with a fixed store of
     four entries, least recently used replaced, with a documented
     fallback that recomputes, always correctly (2.3.4-5): fixed storage
     (about 100 bytes), constant lookup and insertion, no ageing, sweeps,
     polling, overflow or reclamation, and idle subtrees retain their
     entries as today. G1 is stated only for components asked for at most
     four constraints in a pass; test 17 checks four constraints (four
     measurements) and the fallback with five (ten, every result correct);
     test 14(b) is the multi-pass retention case. P7-D1 lists revision 2's
     store as alternative C, and alternative D (a per-pass memo) notes that
     a reusable table keeps its capacity.
  4. **Test 13's count:** confirmed; a row in a column is measured with an
     unbounded height and arranged with its allocated one (C:516,
     :887-892), so its child gets four constraints. Revision 3 expects four
     and four after the redescription (eight under a two-entry store or
     the single entry).
  5. **Test 6(b)'s visibility:** confirmed (`R/runtime.rs:1624-1628`,
     `R/validation/development.rs:496`, `:613-624`). Revision 3 uses
     `a = 50`, so `d` is at 200 from the row (220 to 240 in the window),
     visible inside the padding and outside the row; the mutation places it
     at 180.
  6. **The font variant's assertion:** confirmed. Test 14(c) counts the
     probe's unbounded measurements, which prepare text in the body role,
     and asserts that the counter advances after the body role's default
     font changes; the per-miss reset mutation leaves it unchanged.
- **Revision 3, Codex second recheck (2026-10-28):** needs revision, one
  blocking point, one should-fix point and a nit; it accepted the
  four-entry store with sticky dependency bits. Record
  `~/pert-ui-usability/codex-prompts/codex-design-P7-r3.out`. Each point
  was checked against the committed code; none was rejected.
  1. **Final fill widths against the shortfall and placement rules
     (blocking):** confirmed. Today `flow_rects` replaces a fill child's
     size with its fill measurement (C:504-508) before `flex` computes the
     lengths, `used` and the spare space (C:583-591), the justified start
     (C:707-713) and the reported width (C:760-787), all from final sizes;
     revision 3 kept the fill child's final width but still assumed no
     spare space and a width of `L` after an absorbed shortfall. Revision 4,
     as the author directed, decides the shortfall once from the natural
     widths and never iterates, and computes growth (when no shortfall
     applied, as today), the space `Justify` places and the reported width
     from the final allocations (2.1.7-11); the column's exact `L` is used
     only when the shortfall was absorbed and every fill child kept its
     width. A row whose fill children keep their width is therefore laid
     out exactly as today's code plus steps 2 to 4. The rule is stated in
     `fills_row_height`'s page, contracts.md and the CHANGELOG, and 2.3.2
     says it adds no measurement. Tests 18 and 19 are Codex's two
     configurations (wider: reported 450 and overflow reported; narrower:
     no regrowth, 150 px centered, reported 250); test 16 keeps the case
     without a shortfall. The growth amount follows today's code (final
     allocations) rather than the natural pass, since deciding it from the
     natural widths would change today's layout of test 16's row.
  2. **The fallback test's result check:** confirmed; `Counting` returned
     a fixed size. Test 17 now uses an `Echo` probe answering
     `(w, 1000 / w)`, lists the expected sizes and adds a mutation that
     keeps the counts and returns another entry's size.
  3. **Nit, alternative A's cost:** confirmed (C:516, :887-892). P7-D1's
     alternative A now says twice per row computation, four measurements
     in test 13's first pass and four more whenever the row is laid out
     again.
- **Revision 4, Codex third recheck (2026-10-28):** needs revision, two
  should-fix points; the algorithm of 2.1.7-11, test 17 and alternative A
  accepted. Record `~/pert-ui-usability/codex-prompts/codex-design-P7-r4.out`.
  Both were checked against the committed code; neither was rejected, and
  nothing else changed.
  1. **Shrunk and unshrunk fill widths in the public text:** confirmed;
     rules 2.1.5 and 2.1.7 keep `fᵢ` for a shrunk fill child, but the
     proposed `fills_row_height` text, contracts.md and the CHANGELOG said
     any width at the fill height counts. Revision 5 says in all three
     that only an unshrunk fill child takes its fill-height width and a
     shrunk one keeps its final width, qualifies the overflow floor (an
     unshrunk fill child's width at its fill height, every other ungrown
     child's natural width), and keeps growth when the natural widths fit
     apart from the absence of regrowth after a shrink.
  2. **The `Reporter` fixture:** confirmed; the default
     `clips_children() == true` (`R/runtime.rs:1624-1629`) and the clip
     intersection (`R/runtime/tree.rs:1709-1715`) would clip the overflow
     before validation saw it (`R/validation/development.rs:496-506`,
     `:613-624`). Tests 18 and 19 now specify a 250 px `Reporter` that
     records its child's reported width separately, returns `false` from
     `clips_children` and sits in a 290 px `Container` with 20 px padding.
- **Revision 5, Codex fourth recheck (2026-10-28):** one should-fix: the
  contracts and CHANGELOG passages counted growing children only by their
  minimums, omitting a growing fill child the row did not shrink, which
  counts with its fill-height width (rule 2.1.10 already did). The author
  qualified both passages and added test 18's variant (b) exactly as Codex
  proposed; no algorithm decision reopened. Approved by the author under
  the maintainer's delegation, with P7-D1 (four entries per component,
  least recently used replaced, sticky dependency bits) as recommended.
