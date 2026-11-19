# Candidate F: strokes covered by the renderers

Status: **slice 0 measured; go with the coverage atlas.** The tile lists
([results](stroke-probe/results/summary.md), 2026-11-12) cost 8–30 times
today's mask pass on the GPU and failed the gate. The alternative, per-segment
quads into a coverage atlas
([results](stroke-probe/results/quads/summary.md), 2026-11-13), is below
today's path in every changing workload, 1.7 to 27 times at scale 1, with
parity met. It missed the first gate's 1.0 ms bound, which this machine's
fixed cost of a GPU pass makes unreachable, and the maintainer revised the
[gate](#slice-0-measure-before-building) on 2026-11-13; the alternative
meets the revised gate, and the [renderers](#renderers) now describe it.
Written on branch `rendering-polyline`. [All five slices](#slices-after-a-go)
are done (0.3.118): the preparation and its reference coverage are public
in the runtime, both renderers draw every stroke from them, curves
flattened, and the after-measurement and the final review are recorded. The
maintainer decided on 2026-11-16 to replace the fallback's stroke code
([decision 4](#decisions)). This spec is a record of the phase: what it
says of the fallback's masks, `union.rs` and the outlining stroker
describes them before 0.3.121, which removed that code and made the masks
sample `PreparedStroke` ([spec](fallback-strokes.md)). The maintainer approved the five [decisions](#decisions) as
recommended. Names of new public items are still proposals, approved with
the slice that introduces them. The [review record](#review-record) lists
what the Codex reviews changed; the spec's last revision has not been
reviewed again.

## Goal

A stroked path that changes on every frame should cost its points, not a CPU
coverage mask. Today `Painter::stroke_polyline` records a `DrawVector`, and
every renderer lowers it through `VectorRasterizer` to a mask the size of the
stroke's bounds, which is then uploaded and drawn.

Measured (phase 0 report, quiet machine, Intel UHD Graphics 770): the mask
of a changing polyline over an 800×400 plot cost 3.0 ms of UI-thread time
at 800 points and 4.1 ms at 4096 points, on every frame. 0.3.112 (STROKE-01)
buckets the spans instead of sorting them, and the same coverage now takes
about 1.1 and 2.1 ms in `stroke_phase_profile` (pinned, machine not quiet).
That remaining cost, the mask's upload and the mask grid's placement error
are what this phase removes. The built-in `Trace` draws a min/max column
envelope instead of a line because of the cost.

What the phase should achieve, each measured in slice 0 and again at the end:

| Expectation | Measure |
|---|---|
| A changing polyline costs well under a millisecond of UI-thread time | Per-frame preparation time and uploaded bytes for the probe's 800- and 4096-point polylines, scales 1 and 2 |
| The GPU pass does not give the saving back | GPU time of the offscreen pass and of a window present, against today's mask path |
| The CPU renderer is not slower | CPU render time of the same frames |
| Strokes add no mask resource | Mask resources and bytes per frame in the probe, the diagram editor and the gallery's knob page |
| No stroke the renderers draw is misplaced by the mask grid | The 220/255 difference of RENDER-04 is gone for strokes; it remains for fills and for the fallback's masks |

The saving on the UI thread is an inference until slice 0 measures a
prototype. Fills are not part of this phase and keep their masks.

## Decisions

Approved by the maintainer on 2026-11-12, each as recommended.

1. **Coverage model.** Strokes change from four-sub-scanline area coverage to
   the distance ramp phase 1 uses for shapes. Edge pixels of every stroke
   change, and a stroke thinner than a pixel is drawn one pixel wide and
   faded by its width. How much pixels change is measured in slice 0, not
   promised here.
   Decided: yes; it is what lets both renderers evaluate one formula.
2. **Display-list contract.** Keep `DrawVector` and let the renderers consume
   strokes from it, with a public prepared reference (`PreparedStroke`), or
   add a new command. Decided: keep `DrawVector`. Its documentation
   already allows a backend to consume geometry directly, no painter method
   or consumer changes, and the vector table already interns the path.
3. **Scope.** Straight-segment strokes first; strokes with curves follow in a
   later slice of the same phase by flattening them to segments at
   preparation, so that every stroke has one coverage model in the
   renderers. Decided: yes.
4. **The fallback rasterizer.** `VectorRasterizer` and
   `DisplayList::rasterized` keep lowering strokes with today's area
   coverage, as a fallback for other backends and for tests that read masks.
   Its stroke code (`geometry/raster/union.rs` and the outlining stroker) is
   then no longer what pert-ui's renderers draw. Decided: keep it in this
   phase and decide its retirement at the end, when the numbers are in.
   Decided 2026-11-16: replaced in 0.3.121, the masks sampled from
   `PreparedStroke` ([spec](fallback-strokes.md)).
5. **The gate** of slice 0, below. Decided: as proposed there; revised by
   the maintainer on 2026-11-13 after the measurements, as recorded there.

## The covered region

A stroke at least one physical pixel wide covers the region
`geometry/raster/union.rs` covers today, and subpaths are normalized exactly
as its `pieces` does: repeated points are dropped, a closing point equal to
the first is dropped, a closed subpath with fewer than three points left is
open, and a subpath of one point is a point. The region is the union of:

- a body for each segment, of the stroke's width, extended by half the width
  at a square-capped end;
- at each interior vertex, and each vertex of a closed subpath, a join on the
  side the path turns away from: the circular sector, the bevel triangle, or
  the miter kite while `1 / cos` of half the turn is within the miter limit,
  else the bevel. A straight continuation has no join. A reversal has a half
  disc (round) or nothing (bevel, and miter, which falls back to it);
- at each round-capped end, the full disc around the end point, as today;
- for a point, the disc (round), an axis-aligned square (square) or nothing
  (butt).

A stroke narrower than one pixel is the one deliberate change of region: it
is evaluated as one pixel wide and its coverage is multiplied by its width
in pixels, the rule arcs follow. Self-overlap is covered once.

## Coverage

All values are physical pixels relative to the span's origin. `h` is the
evaluated half width (at least 0.5). A segment record holds its end points
`a` and `b`, its unit direction `d` and, at a joined end, the neighbor's
direction and the join's constants. A vertex two segments share is narrowed
to `f32` once and stored bit for bit in both records, and both evaluate the
tests at that vertex with the same expressions, so the regions below
partition the plane exactly in `f32`.

For a pixel center `p`, with `u = (p − a)·d`, `v = (p − b)·d` and
`s = d×(p − a)`, a segment yields these candidates:

1. **Body**, where `p` is not before a joined start (`u ≥ 0`) and not past a
   joined end (`v ≤ 0`); a capped end does not restrict it:
   `max(|s| − h, C_start, C_end, J_start, J_end)`, with
   - `C` at a capped end, where `e` is how far `p` lies beyond it (`−u` or
     `v`): `e` for a butt or round cap, `e − h` for a square cap;
   - `J = m·q − k` at an end with a bevel join, `q` being `p` minus the
     vertex, `m` the unit bisector toward the outer side and `k = h·cos` of
     half the turn. `m` comes from the sum of the outer normals for a turn
     below 90 degrees and from the difference of the directions otherwise;
     the sum of the normals of a near reversal is rounding noise (found by
     the slice 0 prototype). Both bodies lie wholly inside that half plane, so the
     term only matters next to the bevel.
2. **Join**, owned by the segment that ends at the vertex, where `v > 0` and
   `q·n < 0`, `n` being the next direction (the wedge neither body's side of
   the vertex covers): round `|q| − h`; bevel `max(|q| − h, m·q − k)`; miter
   `max(f·q, g·q) − h`, with `f` and `g` the outward normals of the two
   bodies. For a bevel at a reversal `m = d` and `k = 0`.
3. **Round cap**, everywhere: the distance to the end point, minus `h`.
4. **Point**: `|p − a| − h` (round) or `max(|x|, |y|) − h` about it (square).

Away from short capped segments, every candidate is continuous across the
boundary of its region with the neighboring candidate: at a butt or square
cap the body carries the cap's distance on both sides of the end line; a
body and its round or miter join give the same value on the line between
them; a bevel's line distance is part of both bodies and of the join.
Slice 1 tests each such boundary from both sides.

Two kinds of boundary are not continuous, and slice 0 measures how visible
they are in dense traces:

- **A capped segment shorter than a pixel.** Its body's distance is then
  dominated by its cap, so the body's coverage is below that of the join or
  body beside it. With `h = 1`, a butt start and a first segment 0.1 pixels
  long, the body gives 0.6 where the join next to it gives 1. The segment
  is narrower than a pixel there, so at most one column of pixel centers is
  affected.
- **A body's joined end line on the inner side of the turn**, where the next
  segment is too short to cover it: shorter than `h·|sin θ|` for a turn of
  `θ`. Today's coverage antialiases that edge. Giving the body a soft end
  there would instead dim every vertex, where both end lines meet, to half
  coverage, so the hard edge is kept.

The stroke's distance `D` at `p` is the smallest candidate over the pixel's
tile list, and its coverage is `ramp(D) · thin`, with
`ramp(D) = clamp(0.5 − D, 0, 1)` as for shapes and `thin` the width in pixels
of a stroke narrower than one, else 1. The source is the premultiplied
linear color times the coverage, blended once, source-over. Taking the
smallest distance is what covers an overlap once.

`D` is a bound, not the exact distance, within a pixel of a bevel or miter
corner and of a cap's corners, as a cut corner's is in phase 1. The ramp is
not area coverage and the spec does not promise area coverage.

## Preparation, shared by both renderers

`PreparedStroke::new(vector, origin, scale, clip, mapping)` in
`pert-ui-runtime`, beside `PreparedShape`, does everything that decides a
pixel, in `f64`, and narrows the results to `f32`:

1. **Mapping.** Positions map by the frame's pixels per logical unit on each
   axis, times the command's `scale`, plus its origin; the width maps by the
   viewport's scale times the command's `scale`, as lengths do in
   `PreparedShape`. There is no eight-pixel ceiling, so a magnified stroke
   stays sharp. Curves, once in scope, are flattened within 0.2 pixels at
   that factor under the existing flattening budget.
2. **Support.** In exact arithmetic a stroke's coverage can be nonzero up
   to `(h + 0.5) · R` pixels from a vertex, with `R` the largest of 1, `√2`
   for a square cap and the largest miter ratio that occurs. Narrowing a
   miter's normals to `f32` moves its tip by a share of that distance that
   grows with `R` (about `R · 2⁻²⁴`; a ratio of 40 000 moved it by ten
   pixels in the review's example). The support `S` therefore adds that
   share with a factor of safety, plus one pixel: proposed
   `S = (h + 0.5) · R · (1 + R · 2⁻²²) + 1`. Slice 1 derives the bound from
   the narrowed values and tests it adversarially; a miter whose bound
   cannot be met is refused by the domain rule below, not drawn short.
   Derived in slice 1 (`PreparedStroke`'s rustdoc, "Rounding"): a candidate
   differs from its `f64` value by less than `3.2 · 2⁻²⁴` of the distance to
   the vertex, a miter's tip moves by `R` times that, and the proposed
   factor covers it. Miters over 1 000 widths long are tested against it.
3. **Span.** The stroke's bounds grown by `S`, intersected with the frame
   and the snapped clip, by the rules and the code `PreparedShape` uses. An
   empty span or a zero width prepares nothing; the color is not an input,
   so a renderer skips a fully transparent stroke itself.
4. **Segments.** A segment farther than `S` from the span is dropped. A
   kept segment is clipped in `f64` to the span grown by `S`; a clipped end
   has neither cap nor join and does not restrict its body, since its vertex
   is more than `S` from the span and nothing of its cap or join reaches it.
   Directions come from the unclipped end points.
5. **Domain.** Recorded coordinates, the half width and `S` must lie within
   65 536 pixels of the span's origin, the bound phase 1 puts on its
   coordinates, lengths and reaches; that limits the width and the miter
   ratio for a given span. A stroke outside it is refused with a typed error
   rather than evaluated imprecisely. Slice 1 states, as phase 1 does, which
   intermediates that bounds and the rounding error it assumes.
6. **Tiles.** The span is divided into square tiles of pixels. A tile lists
   every segment that can have a candidate below 0.5 plus the parity margin
   at one of its pixel centers, decided in `f64` with padding for the `f32`
   evaluation; a list may hold more. Empty tiles are not recorded. The wgpu
   renderer needs no lists, only the tiles a segment's reach touches, merged
   into runs per row; the lists serve the reference and the CPU renderer.
7. **Budgets.** Per stroke: the tile references in total and the longest
   tile list, each with a fixed limit, and no list is allocated beyond
   them. The segments were bounded by the path's own limit of 65 536
   points until curves were flattened (slice 4), which added the same
   number as a budget of kept segments. Slice 1 chose 16-pixel tiles, 2²⁰ references and 2¹² segments in
   one tile: slice 0's largest workload, the 16 384-point noise at scale 2,
   recorded 358 279 references and a longest list of 229. Slice 2 kept
   them: the CPU renderer draws that workload in about 0.3 s, and the
   limits bound a stroke at 2²⁰ references of at most 256 pixels each. They
   are public as `PreparedStroke::MAX_TILE_REFERENCES` and
   `MAX_TILE_SEGMENTS`. A stroke over one is refused with
   `DrawError::TooComplex`, as `VectorRasterizer::MAX_COVERAGE_WORK`
   refuses one today; one outside the domain with
   `DrawError::Stroke(StrokePrepareError::OutOfRange)`. Geometry is
   thereby admitted or refused alike by both renderers; a device's buffer
   limits remain an additional, renderer-specific refusal, reported as the
   shape records' are.

`PreparedStroke::coverage(x, y)` evaluates the candidates of the pixel's
tile list in `f32`, in a fixed order of operations, and is the reference, as
`PreparedShape::coverage` is. It reports the coverage and, for the parity
classification, the smallest listed distance; a pixel whose list is empty or
whose smallest distance is at least 0.5 plus the margin is reported as
decided and uncovered. Because every segment below that threshold is
listed, a renderer that visits segments in another order, or more of them,
is meant to decide the same pixels; like the shapes' parity, that is a
target the corpus measures, not a proof. An external backend prepares strokes the same way
or keeps using `VectorRasterizer`.

## Renderers

**wgpu.** A stroke pipeline next to the shape pipeline, with the same
storage-buffer upload path and limit checks, as slice 0's quads path. Per
stroke: a header (span origin, `h`, `thin`, premultiplied color), its
segment records and its tile runs. Each stroke's span has a region of a
coverage page, a half-float texture at least 2048 pixels a side and never
smaller than the frame (slice 3: 8 bits moved the faintest coverage of a
bright stroke over a dark destination by several steps once encoded as
sRGB, and the corpus measured two). A first pass per page draws one quad per segment into its
stroke's region, under a scissor, with a maximum blend: the body, its caps
and a round or bevel join's wedge in one oriented quad, a miter's wedge in a
second, the segment's record reaching the fragment stage as flat varyings.
The maximum of the segments' coverage is the coverage of their smallest
distance, so a page holds the reference's coverage up to a half float's
precision. The scene pass draws the stroke's tile runs as ordinary quads
reading the page, so empty parts of the span cost no fragments and no pixel
is blended twice. Strokes that do not fit one page fill further pages, all
drawn before the scene pass, up to 128 MiB of whole pages at two bytes
per pixel, the first page always allowed; slice 0's probe refilled one
atlas between scene passes instead.
While the frame's strokes do not change, the pages keep their coverage and
the frame draws only the composite. A stroke's region is its whole span, so
a long diagonal takes a mostly empty box, as its mask did; packing by the
covered tiles is left for when it matters. Strokes keep their place in command order as a batch kind of their own, and
consecutive strokes share a batch. Records are in frame pixels. They are
retained as the shape records are: an unchanged command slot under an
unchanged frame mapping keeps its encoded records
(`RetainedGeometry`), so a scene that changes elsewhere does not prepare an
unchanged stroke again. A cache by content, across slots, is not planned.

**CPU.** The same values. It may visit pixels segment by segment instead of
tile by tile, keeping the smallest distance per pixel in a buffer over the
span before one blend. The buffer is four bytes per span pixel, within the
frame the renderer already bounds.

**Rasterizer.** `VectorRasterizer` gains a preparation that leaves strokes
in the scene for a renderer that consumes them (name to approve, for example
`prepare_fills`); `prepare` and `DisplayList::rasterized` behave as today.

Until a later phase gives vectors a `Paint`, a stroke has the command's one
color.

## Parity

As for shapes. A pixel is decided when its distance is at least half a pixel
plus the parity margin from zero on either side; decided pixels belong to
the exact or the tolerance tier. The others form the stroke's edge tier,
whose bound slice 0's prototype measures over a corpus: the three joins and
caps, closed subpaths, points, reversals, straight continuations, segments
shorter than the half width, shared vertices at sub-pixel phases, widths
from 0.25 to 12 pixels, miter limits up to the domain, scales 1, 1.25 and 2,
view scales below and above eight, clips that cut segments, and geometry far
from its span. The target is one step per channel, as for shapes, within
the same four-step limit; evidence from one adapter is recorded as such.
Capture is tested through a view transform and a crop.

## Slice 0: measure before building

No library change beyond STROKE-01, which is done. On a quiet machine, with
the pinned runs and the noise allowance of phase 1 (5 % and 0.05 ms),
medians and 95th percentiles:

1. **The baseline** is the mask path at 0.3.112, in the real-time probe's
   own intervals: rasterize, prepare (encoding and upload) and the GPU pass
   (encode, submit and wait for completion of one offscreen pass). A window
   present is measured separately. The CPU renderer's time for the same
   frames is measured too.

   Where the mask path's time went before 0.3.112, from a temporarily
   instrumented `stroke_phase_profile` (release, machine not quiet, timing
   prints included): of 5.2 ms for the 800-point trace, 3.5 ms was the
   comparison sort of the spans, 0.6 ms their generation and 0.9 ms the
   accumulation; of 48 ms for a steep alternation, 41 ms was the sort.
   After the bucketing the same strokes take about 1.1 ms, 2.1 ms (4096
   points) and 5.8 ms (pinned, uninstrumented, against 3.0, 4.0 and 28.5 ms
   in the same session).
2. **A prototype** under `plans/rendering/stroke-probe/`: the preparation
   above and the fragment evaluation, outside the library, measured in the
   same intervals. Workloads: the 800- and 4096-point probe polylines, a
   full-height noise trace, a stroke near each budget, crowded tiles, the
   diagram editor's edges, a knob, a view scale above eight, and a scene
   that changes while its strokes do not; scales 1 and 2.
3. **Pixels.** The edge-tier bound over the parity corpus, and how far the
   new coverage differs from today's masks, as a distribution.
4. **Tile size, record layout and budgets** are chosen from those numbers.

**Gate.** Go when all hold:

- for both probe polylines, prepare plus GPU pass is at most 1.0 ms and
  below the baseline's rasterize plus prepare plus GPU pass by more than the
  noise allowance;
- no workload is slower than the baseline in total, nor in the CPU renderer,
  beyond the allowance;
- the edge tier is within four steps on the corpus.

Otherwise stop, or evaluate the alternative below, and report.

The tile lists failed this gate. So did the alternative, on the 1.0 ms bound
(1.47 and 1.50 ms) and on the knob (0.07 ms above today for its second
render pass), while it was below today's path in every changing workload.
On this adapter the GPU interval is mostly fixed cost: uploading, submitting
and waiting take about 0.4 ms for a nearly empty frame at the idle clock,
and today's GPU pass for the 800-point trace alone takes about 1.0 ms. The
bound therefore measured the machine, not the stroke path.

**Revised gate** (the maintainer, 2026-11-13, after the measurements). Go
when all hold:

- for both probe polylines at scales 1 and 2, prepare plus GPU pass is
  below the baseline's rasterize plus prepare plus GPU pass by more than the
  noise allowance;
- no workload is slower than the baseline in total beyond the allowance
  plus 0.1 ms per frame, the fixed cost of a second render pass, nor in the
  CPU renderer beyond the allowance;
- the edge tier is within four steps on the corpus.

The alternative meets it: the probe polylines take 1.47 and 1.50 ms against
2.45 and 3.31 ms at scale 1 and 2.32 and 3.28 ms against 5.66 and 6.89 ms at
scale 2, of which preparation and upload is at most 0.34 ms; the knob is
0.07 ms above the baseline at both scales; the CPU side is faster in every
workload; the largest edge difference is 0.51 of a step. The revision was
made knowing these results, and is recorded as such.

**The alternative**, now the wgpu path. Render each stroke's coverage on the
GPU into a coverage target (8 bits in the probe, half floats since slice 3)
with a maximum blend, one quad per segment, then
composite it. It needs a coverage atlas, its allocation and a pass before
the composite.

## Slices after a go

1. `PreparedStroke` and its reference coverage in the runtime, crate-private,
   with tests against hand-computed distances and against today's pieces for
   the covered region. **Done** (0.3.113, `shapes/stroke.rs`): the tests
   also cover each region boundary from both sides, the tile lists against
   every segment, the support of long miters, clipping, far geometry and
   the refusals. The tile runs the wgpu path composites are left to slice
   3, as is a preparation without lists.
2. The public reference, the rasterizer's preparation that leaves strokes,
   and the CPU renderer. **Done** (0.3.114): `PreparedStroke`,
   `StrokeCoverage` and `StrokePrepareError` are public, `PreparedStroke::new`
   returns `DrawError` so that a renderer reports strokes as the rasterizer
   reported them, `VectorRasterizer::prepare_fills` leaves the strokes
   `PreparedStroke::accepts`, and the CPU renderer evaluates the reference
   over each stroke's tiles. It is faster than the masks in every measured
   workload but the 4096-point trace at scale 1, which is level
   (`docs/verification.md`, CPU rendering cost). The segment records stay
   crate-private until the wgpu renderer needs them. Until slice 3 the two
   renderers differ at a straight stroke's edge pixels.
3. The wgpu pipeline (the coverage atlas and its composite) and the parity
   corpus; the edge tier is recorded. **Done** (0.3.115): `StrokeSegment`
   and `PreparedStroke::segments` are public; the renderer keeps coverage
   pages, record buffers and each stroke's encoding per command slot,
   compared with the slot's geometry as well as its command, since a
   vector's ID is only an index in its scene. The stroke corpus measured
   one step for edge pixels on the one adapter (two where two strokes'
   edges overlap, each command rounding to 8 bits), within the shapes'
   tier. The real-time probe's frame took 1.2 ms for a changing 800-point
   polyline against 2.3 for the mask, and 2.2 against 3.3 ms for 4096
   points (`docs/verification.md`). Not done: a preparation without tile
   lists, which the probe had and which would roughly halve the 0.16 and
   0.54 ms the stroke's preparation and encoding take; a view scale in
   the corpus.
4. Strokes with curves. **Done** (0.3.117): the preparation flattens
   curves with the mask coverage's flattener, within 0.2 pixels on the axis
   that maps a unit to the most pixels, times the command's scale, and
   `PreparedStroke::accepts` is true for every stroke. As the outlining
   stroker does, a vertex inside a flattened curve has a round join
   whatever the style, and a cap or a join where a curve ends follows the
   curve's exact tangent: a point is added on the tangent, 0.35 of the
   chord from the vertex, which stays within a tenth of a pixel of the
   curve. A stroke that misses the frame or its clip is skipped before
   flattening. Kept segments have
   the budget `PreparedStroke::MAX_SEGMENTS` (65 536). A circle's distance
   is within 0.2 pixels of the exact ring's at four scales, and the parity
   corpus gained an ellipse and an open cubic and quadratic path.
5. After-measurement, the decision on the fallback's stroke code, the
   pre-merge gate and the final review. **Done** (0.3.118), except the
   decision. The [after-measurement](stroke-probe/results/after/summary.md)
   ran the branch and the mask path at 0.3.112 alternately on a quiet
   machine: a frame with a changing polyline takes 1.22 against 3.52 ms at
   800 points and 1.78 against 5.56 ms at 4096 in the wgpu renderer. The
   CPU renderer first failed its condition on one row, the 4096-point
   trace at scale 1 (7.95 against 7.16 ms), because it evaluated every
   pixel of a tile against the tile's whole list;
   `PreparedStroke::for_each_covered` evaluates each segment only where it
   can reach, gives the reference's coverage exactly, and makes every row 3
   to 37 times faster than the masks, unchanged scenes included. The
   fallback's stroke code (`geometry/raster/union.rs` and the outlining
   stroker) now serves only `VectorRasterizer::prepare` and
   `DisplayList::rasterized`, which no renderer of pert-ui draws; decision
   4 keeps it until the maintainer decides.

Each slice that changes what a user sees or calls updates the rustdoc
(`Painter::stroke_path`, `DrawCommand::DrawVector`), rendering-platform.md
and verification.md in the same change.

The built-in `Trace` moving from its column envelope to a line is a separate
decision after this phase, not part of it.

## What a user notices

- No painter method, command or widget changes.
- Strokes are antialiased by distance. Joins and caps at least a pixel wide
  keep their region; edge pixels change, and a hairline is drawn one pixel
  wide and faded by its width.
- A magnified stroke under a view transform is no longer stretched above
  eight pixels per unit.
- A stroke is refused as `TooComplex` by its segments and tile lists, where
  today the limit is counted in sub-scanline steps, and by the numeric
  domain, which is new.
- `DisplayList::rasterized` still returns stroke masks. Drawing them no
  longer reproduces what the renderers draw: the coverage model, hairlines,
  the eight-pixel ceiling, the placement of RENDER-04 and the admission
  limits all differ.

## Risks

- **GPU time on dense overlapping data.** Every segment's quad is shaded
  wherever it reaches, overlapping or not; the budget bounds it. Slice 0's
  16 384-point noise took 4.9 ms of GPU execution at scale 1.
- **Parity.** The GPU and the CPU must agree on which candidate is
  smallest; equal operation order per candidate and a commutative maximum of
  coverage make that hold, and the corpus checks it. The pages hold half
  floats.
- **A second render pass** costs about 0.07 ms per frame on slice 0's
  adapter even for a single short stroke.
- **Two stroke models** while the fallback keeps area coverage (decision 4).
- **Evidence** is from one adapter until Windows and macOS runs exist.

## Review record

Codex reviewed the first draft on 2026-11-12 (read-only, GPT family) and
verified the display-list proposal, the package boundary, the ordinary
joins' regions, the single blend and the baseline figures. Its findings, all
confirmed, changed the draft as follows:

1. Coverage jumped at a butt cap and at a bevel (blocker): the body now
   carries the cap's and the bevel's distance, and each region boundary is
   continuous and tested from both sides.
2. Round caps were half discs, not today's full discs (blocker): a round cap
   is a disc candidate everywhere.
3. Points, reversals, straight continuations and closure were unspecified:
   stated, following `pieces`.
4. The miter reach ignored the ramp (blocker): the support is
   `(h + 0.5) · R + 1`.
5. The numeric contract was incomplete: the mapping, the 65 536-pixel domain
   with a typed refusal, and shared vertices stored once.
6. A tile list could change the reported distance: the listing threshold
   includes the parity margin, and an unlisted pixel is decided.
7. One reference count was not a resource contract: segments, references and
   the longest list are counted before allocation; device limits are
   separate.
8. Retention, ordering, the fallback's differences and documentation timing
   were unstated: stated.
9. The profile could not attribute the cost: temporary instrumentation did,
   and STROKE-01 repaired the sort (0.3.112).
10. The gate mixed intervals: it now uses the probe's intervals, medians and
    95th percentiles, a noise allowance, and CPU and parity conditions.
11. "Slightly" and "a few levels" were unsupported: removed; slice 0
    measures the difference.

Codex rechecked the revision the same day, together with the STROKE-01
code, in which it found no defect. It sampled 132 300 points against
independently built pieces without a mismatch of the covered region in
exact arithmetic, and confirmed the bevel half plane, the hard inner edge
and why a soft end would dim vertices. Its remaining findings changed the
spec again:

1. Continuity still failed beside a short capped segment (blocker): stated
   as the first of two named discontinuities, to be measured, instead of
   claimed away.
2. One pixel of padding does not cover a large miter's `f32` narrowing
   (blocker): the support gains a term that grows with the ratio, to be
   derived and tested in slice 1, with refusal by the domain.
3. The domain and parity wording overstated what is established: narrowed
   to the bounded quantities, and parity called a measured target.
4. Retention contradicted the slot reuse of `RetainedGeometry`: strokes
   keep it.
5. "Distinct" points and the inner edge's shortness condition were wrong:
   corrected to the remaining count and `h·|sin θ|`.

Codex reviewed slice 0's quads path on 2026-11-13 (shaders and draw setup,
read-only). It found two quad bounds too small, both reproduced on the GPU
and repaired in the probe: a round cap's disc behind a short segment's
joined end, and a miter square recomputed in `f32` near a reversal, which
now takes the preparation's reach. Slice 3 inherits both rules; its other
proposals are measured in the [results](stroke-probe/results/quads/summary.md#after-a-review-of-the-shaders).

Codex reviewed slices 1 and 2 on 2026-11-13 (read-only, before slice 2 was
committed) and verified the normalization, the joins, the reaches and row
bands of the listing, the integer bounds, the CPU renderer's ordering and
`prepare_fills`' remapping. Its findings, all confirmed and repaired with a
regression test each:

1. A segment whose end points are both remote (1e20 pixels) was clipped by
   a parameter that could not resolve the frame, so its record collapsed to
   a point and its tiles lost it: the cut now comes from the line's
   two-point form, as the mask coverage's does.
2. The reference budget was checked after a segment had listed itself, so
   one large disc could allocate tens of megabytes first: the listing
   stops one past the budget. The eight bytes per tile of the span are now
   documented.
3. `PreparedStroke::tiles` overflowed at the last pixels of a frame as large
   as a `u32` holds.
4. The rounding derivation omitted the narrowing of the end points and the
   half width: restated with them; the bound of 1/32 pixel stands.
5. "The same region as the masks" held only for equal axis scales and a
   stroke at least a pixel wide: qualified, and unequal axes tested.
6. The refusal test could not show that the refusal precedes every pixel:
   renamed, and the verification note says it is read from the code.

Codex reviewed slice 3 on 2026-11-13 (read-only, before it was committed)
and found no rendering defect: it verified the record layout and the
fragment stage against `StrokeSegment::distance`, the quads' bounds for
every end and join kind, the composite's offsets and runs, the retention
and the limits' arithmetic. Its findings changed the slice as follows:

1. The coverage budget was the mask budget counted over whole pages, so a
   surface whose own page exceeded it could draw no stroke, and the
   documents described it per span pixel: it is now
   `Renderer::STROKE_COVERAGE_BUDGET` (128 MiB of whole pages, the first
   always allowed) with its own `LimitError::StrokeCoverage`, documented as
   it is counted.
2. The changelog promised one step on the corpus where the record says two
   at overlapping edges: restated.
3. `StrokeSegment::distance` and `PreparedStroke::segments` claimed equal
   distances and a finite reach without restricting them to the span:
   restated as equal coverage inside the span.
4. The parity test's allowance per edge command counted transparent strokes
   and could overflow: both corrected. That an overlap allowance can hide a
   two-step defect of one command stands as a limitation; the corpus's
   single-edge pixels, the great majority, are held to one step.

Codex reviewed slice 4 on 2026-11-13 (read-only, before it was committed).
Its findings, all confirmed and repaired with tests of its examples:

1. Joining a curve's chords in the stroke's style, and capping and joining
   its ends along the chords, moved the region by pixels for wide strokes
   and lost the round end of a curve that reverses: joins inside a curve are
   now round and its ends follow its tangents, as in the mask coverage.
2. A point subpath did not count toward the segment budget: it does. No
   test reaches that budget with a point.
3. Curves were flattened before a zero width, an empty clip or a stroke off
   the frame could skip them, so an invisible curve under a huge scale
   failed the frame: the path's bounds decide first.
4. The rendering contract's limits paragraph and the gallery's vector page
   still described straight strokes and masks: corrected.

What a curved stroke that does not change costs the CPU renderer per frame
without a cached mask was measured in slice 5: less than drawing the mask.

Codex reviewed the whole branch on 2026-11-13 (read-only, before slice 5 was
committed). It verified the slices' interactions: the round joins inside
curves against the shader's quads and the miter flag, the pre-flatten
rejection's reach against the later support, the listing, the clipping, the
record layout and the retention. Its findings changed the branch as follows:

1. A tangent pointing away from its chord, as from a control point just
   behind the vertex, was taken for a line's and got no point on it, so the
   cap followed the chord: the direction is now compared too, and the turn
   back is joined in the stroke's style.
2. The point on the tangent could leave the curve by more than the
   tolerance where a control point almost coincides with the vertex: it now
   stays within half the tolerance of the chord.
3. The after-measurement it read showed the CPU row that failed; the repair
   and the new measurement were made while it reviewed.
4. `for_each_covered` lacked its listing and changelog entry, and its test
   lacked far geometry, a long miter and a point: added.
5. The changelog's "region is unchanged" lacked the equal-axes condition:
   added.

Recorded in `TODO.md` rather than done: a test at `MAX_SEGMENTS`, and
parity fixtures under a view scale and for a miter near the evaluation
range. Evidence remains one adapter on Linux.
