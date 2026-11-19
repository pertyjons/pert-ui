# The fallback's strokes from the reference

Status: **implemented in 0.3.121.** Proposed 2026-11-16, after the
maintainer chose to replace the fallback's stroke code (decision 4 of the
[stroke spec](stroke-spec.md)), revised once after a Codex review
([record](#review-record)) and approved by the maintainer the same day.
One change during implementation is recorded under decision 1: a stroke's
mask carries no clip.

## Goal

`VectorRasterizer::prepare` and `DisplayList::rasterized` still lower a
stroke to a mask with the fallback's own stroke model: the outlining stroker
in `geometry/raster.rs` and, for straight segments, the direct union
coverage in `geometry/raster/union.rs`. That model is area coverage on four
sub-scanlines, on a grid anchored at the stroke's own bounds. The renderers
draw strokes from `PreparedStroke`, a distance ramp at frame pixel centers.
A custom renderer that uses the fallback therefore draws every stroke's
edges differently from pert-ui's renderers, and the repository keeps two
stroke models to maintain and test.

After this change the fallback's stroke masks are sampled from
`PreparedStroke`: a renderer that draws them gets the reference's coverage,
quantized to eight bits, at the frame pixels the built-in renderers cover.
The outlining stroker, `union.rs` and their tests are removed. Fills are
unchanged; their masks and RENDER-04 belong to [candidate
K](../rendering-capabilities.md).

## Decisions

1. **Where the mask lies.** The stroke is prepared with
   `PreparedStroke::new` for the command's origin, scale and clip and the
   frame `FrameMapping::new(viewport.frame_extent(), viewport)`, built once
   per scene at the first stroke. A frame extent with a zero dimension has
   no pixels: its strokes are skipped, as a wholly clipped command is, and
   the mapping is not built. The mask is the stroke's span, one texel per
   frame pixel, holding `round(coverage · 255)` from
   `PreparedStroke::for_each_covered` and zero elsewhere; a stroke that
   covers nothing is skipped. The command becomes a `DrawMask` whose rect
   is the span divided by the mapping's per-axis factors and stored as
   `f32`, without a clip: the span is already cut to the clip on the
   frame's pixels. The first implementation kept the command's clip, and
   the wgpu comparison failed on a fixture whose clip edge, 10.6 + 5.9,
   lies on pixel centers: the span kept the row, by `f64` arithmetic, and
   the wgpu renderer's own clip of the mask dropped it.

   The placement is therefore approximate, not exact: narrowing the rect
   moves its edges by about `2⁻²⁴` of their logical coordinate, and a
   renderer that samples bilinearly blends that fraction of a neighboring
   texel. Within a frame of at most 16 384 pixels per axis, every position
   lies within `2⁻⁸` pixel of its frame pixel center, counting the
   renderers' own `f32` arithmetic, which changes a sampled coverage by at
   most that fraction of the step between
   neighboring texels. The rustdoc states this bound. A renderer with
   another frame size than the viewport's frame extent stretches the mask
   as it would any mask; the fallback takes no frame size, and adding one
   is not proposed.
2. **No magnification ceiling for strokes.** The fallback holds vectors to
   eight physical pixels per geometry unit under a magnifying view and
   stretches the rest. A stroke prepared from the reference is clipped to
   the frame, so its mask is bounded by the frame and needs no ceiling, as
   in the renderers. Fills keep the ceiling.
3. **The cache.** A stroke's mask is reused when all inputs of its
   preparation are equal: the vector (path and style), the command's
   origin and scale, the mapping's `sx`, `sy` and viewport scale, and the
   pixel box the span is cut to, the frame intersected with the snapped
   clip. Within one scene and the previous one this reuses every stroke
   that did not move, the case the cache exists for. A window resize
   changes the frame and so prepares strokes again, where today a resize
   that leaves a vector wholly visible reuses it; fills keep that. A key
   on the stroke's own extent instead would need a bound containing the
   final span before flattening, and the path's bounds are not one: the
   tangent neighbors `stroke_subpaths` inserts can lie outside the control
   points' hull (the review's counterexample). Reusing masks across
   whole-pixel moves was rejected as well: the mapping's `f64` arithmetic
   is not exactly translation invariant, so a reused mask could differ
   from a fresh one.
4. **Limits and errors.** A stroke in the fallback has the renderers'
   limits and errors: `PreparedStroke::MAX_SEGMENTS`, the tile budgets and
   `DrawError::TooComplex`, `StrokePrepareError::OutOfRange`, the flattening
   budget, and `DrawError::CoverageTooLarge` when the span's mask would
   exceed `MaskImage::MAX_PIXELS`, checked before the mask is allocated and
   after a successful preparation. `VectorRasterizer::MAX_COVERAGE_WORK`
   then bounds fills only. `VectorRasterizer::MAX_STROKE_EDGES` is removed:
   no outline is built any more. Its replacement is the `PreparedStroke`
   budgets. A failure leaves the input scene and the last successful cache
   usable, as today.

   Strokes the fallback accepted and now refuses: a stroke whose span is
   taller or wider than 65 536 pixels, which needs a frame that large, is
   refused with `OutOfRange`, as the renderers refuse it. Strokes it
   refused and now accepts: a long trace whose outline exceeded
   `MAX_COVERAGE_WORK` or `MAX_STROKE_EDGES` but fits the tile budgets.
   The breaking changes are accepted while the library is unpublished and
   are listed in the CHANGELOG.
5. **What the masks look like.** The mask covers the span, the stroke's
   reach plus padding clipped to the frame, rather than the outline's
   bounds, so it has transparent margins, and its edge pixels hold the
   distance ramp instead of area coverage. A stroke narrower than a pixel
   is one pixel wide and faded, as in the renderers.
6. **What stays in `raster.rs`.** Flattening with exact tangents, which
   `stroke_subpaths` needs, the fill scan conversion and the fill's bounds.
   The stroke branch of `coverage_bounds` and whatever else only the
   stroker read go.

## Tests

- **Reference parity, in linear coverage.** For a corpus of strokes
  (straight and curved, including the review's near-reversal quadratic
  with a square cap; every cap and join; a hairline below one pixel and
  coverage near zero; a clipped stroke; a fractional origin; window scale
  1.25 and a fractional viewport size; a magnifying view beyond eight
  pixels per unit; a closed subpath and a point; a small span far from the
  frame's origin in a 16 384-pixel frame), each texel equals
  `round(coverage · 255)` of `PreparedStroke::coverage` at its frame pixel,
  and mapping the rect back gives the span within `2⁻⁸` pixel.
- **The CPU renderer agrees.** `pert-ui-cpu` renders the corpus directly
  and after `DisplayList::rasterized`, over a dark and a light background,
  with an opaque and a translucent color and with two overlapping strokes.
  The frames are compared after decoding to linear light: each channel
  within the quantization (half of 1/255 coverage times the color) plus the
  placement bound of decision 1. Encoded bytes are not compared, since
  near black one coverage step is several encoded levels.
- **The wgpu renderer.** The renderer parity corpus in `pert-ui-wgpu` gains
  the fallback's stroke masks drawn by the wgpu renderer against the CPU
  renderer's direct strokes, in the same linear-light comparison. This
  needs a GPU: it is run on this machine and recorded with its platform,
  and Windows and macOS evidence stays missing, as for the rest of the
  renderer.
- **Cache.** The same scene twice prepares once. Each key field is
  changed on its own (the path, the style, the origin, the command scale,
  each of `sx`, `sy` and the viewport scale with the frame extent unchanged
  where possible, and a clip that cuts the span) and must prepare again and
  match a fresh preparation. A failure keeps the previous cache, and the
  next successful preparation reuses it.
- **Errors.** One test per path through `prepare`: `TooComplex` from a tile
  budget (`MAX_SEGMENTS` reports through the same path and is tested with
  `PreparedStroke`), `OutOfRange`, the flattening budget's
  `Path(TooLarge)`, and `CoverageTooLarge`. A scene with a zero-sized
  viewport, with only fills, and with only strokes prepares without
  building a mapping it cannot. A zigzag over `MAX_COVERAGE_WORK` is still
  refused as a fill and is accepted as a stroke.
- **Migrated facade tests.** The tests in
  `crates/pert-ui/tests/foundation/vector_geometry.rs` and `transforms.rs`
  that read stroke masks, and the gallery's vector-shapes page test, assert
  behavior at frame pixels through a helper that maps a frame pixel into
  the mask, instead of mask-local offsets and tight bounds: caps, joins,
  miter limits, dots, hairlines, remote endpoints and transforms keep their
  statements. The 100 000-pixel-tall frame case expects `OutOfRange`;
  frames of 100 and 60 000 pixels keep the rows-to-the-bottom statement.
- Removed with the code: the union-versus-outline comparisons, the direct
  coverage tests and the ignored `stroke_phase_profile`. The flattening
  tests stay.

Each new test is checked by a mutation that breaks what it states, as in
phase 1.

## Documentation

Rustdoc of `VectorRasterizer` (the limits, the ceiling, the cache), its
`prepare` and `prepare_fills`, `DisplayList::rasterized`,
`Painter::stroke_path` (which describes the outlining, the area sampling
and the fallback's separate errors), `PreparedStroke` (which describes the
fallback's different coverage), `DrawCommand::DrawVector`,
`DrawError::TooComplex` and `CoverageTooLarge`; `docs/rendering-platform.md`
(vectors and the budget paragraph); `docs/verification.md` (the
`stroke_phase_profile` section goes, the stroke-cost history stays, and the
wgpu run is recorded); decision 4 of the stroke spec and row F of the
capabilities plan; the CHANGELOG with the removed constant, the changed
stroke masks and the strokes now refused or accepted; the `api/` listing.

## Verification

One slice, one version. It changes public behavior and removes a public
item, so it runs the workspace gate, the API listing gate, the wgpu
comparison above on this machine and a Codex review of the change. No
measurement is planned: the built-in renderers do not use the fallback, and
the gallery's pages do not change.

## Review record

Codex reviewed the first draft on 2026-11-16. It changed:

- Decision 3: the first draft keyed the cache on a span formed before
  flattening, assuming it contained the final one. Tangent neighbors break
  that (a quadratic `(0,0) → (−0.001,0) → (100,0)` with a square cap at
  origin `(10.2, 10)` widens the span by a column). The key is now every
  input of the preparation, and a resize prepares strokes again.
- The CPU comparison was one encoded level; quantization alone exceeds
  that near black. It is now in linear light, over more scenes.
- Decision 1 claimed exact placement; it now states the `f32` bound, and
  the corpus tests it at a large frame position. The wgpu comparison was
  added.
- The documentation list gained `Painter::stroke_path` and
  `PreparedStroke`; the facade and gallery tests that read stroke masks
  are migrated, and strokes now refused or accepted are recorded.
- The key names `sx`, `sy` and the viewport scale; the zero-frame case,
  each error path and cache recovery after a failure are specified and
  tested.

Codex reviewed the implementation on 2026-11-16. It changed:

- A stroke mask without a clip could draw a pixel outside the clip in a
  frame far beyond 16 384 pixels (16 777 216 by 1 at a fractional scale),
  where narrowing the rectangle moves an edge across a pixel center. Such
  a mask now keeps the command's clip; one placed on the pixel edges, as in
  every frame within the range, carries none.
- The migrated cap and dot tests assert every pixel of the expected box
  fully covered and every other one clear, as the originals did.
- The rendering contract qualifies the placement bound by the frame size,
  and comments and the stroke spec no longer describe the removed code as
  current.
