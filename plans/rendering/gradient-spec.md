# Phase 2 proposal: gradient paints

**Status:** Slice 0 authorized and measured, 2026-11-24. The
[prototype report](gradient-probe/README.md) records the results and
remaining integration gates. The public API awaits maintainer approval. The [roadmap](../rendering-capabilities.md)
and [phase 0 decisions](phase0-report.md#decisions) select pert-ui's own
renderers, Oklab by default and linear-light compositing. The API names,
limits and acceptance criteria below are proposals. Library implementation and
after-measurements follow a separate decision.

## What an application can draw

A custom component can fill a rectangle or vector path, stroke a polyline,
tint mask coverage, or give a rounded panel independent gradient fill and
border paints through the public facade. A meter can anchor one ramp to
its full track and crop the filled part as the reading changes. This
replaces stretched ramp images and hand-built strips. Raster images retain
their existing multiplicative color tint. Glyph shaping, SVG decoding and
the vector coverage algorithms have no new responsibilities.

Illustrative Rust for a component painting a track (proposed API):

```rust
let gradient = Gradient::linear(left, right, [(0.0, low), (0.65, warning), (1.0, high)])?;
painter.fill_rect(filled_bounds, Paint::Gradient(gradient));
```

Here the endpoints belong to the full track in component-local logical
pixels. The same immutable gradient can be retained across paints and
cloned cheaply.

For a pert-ui-drawn window title bar 320 logical pixels wide, fill the whole
bar with a horizontal Pad gradient from x = 0 to x = 40. The first 40 logical
pixels transition; the remaining 280 show the final stop color. Resizing the
bar keeps the transition 40 logical pixels wide. To reveal the ordinary bar
background beyond that point, paint the background first and restrict the
gradient overlay to the first 40 logical pixels. Verify both at scale 1
and 2, and after reducing the bar below 40 logical pixels (crop the ramp).
The system-owned native title bar lies outside the Painter surface.

Solid callers can still pass a `Color`. Add a variant beside
`Paint::Solid` and accept `impl Into<Paint>` wherever Painter accepts fill,
stroke or mask color, including shape style builders. Keep text, icon and
image convenience APIs taking Color where they express a uniform tint.
Document migration from Copy to Clone for Paint, Border and ShapeStyle,
and the loss of const construction for the `ShapeStyle` fill and border constructors and
replacement builders. Keep `Border::new(width, paint)` const where possible.

## Geometry and color contract

- Linear uses `t = dot(pixel − start, axis) / dot(axis, axis)` with
  `axis = end − start` and the pixel center in local logical coordinates.
- Radial uses concentric ellipses. Normalize the local displacement from
  the center by the two positive outer radii. With inner radius fraction
  i in [0,1), t = (length(normalized displacement) − i) / (1 − i).
- Sweep measures degrees clockwise from +x in y-down coordinates. Wrap
  angle minus start to [0,360) and divide by end minus start. At the center
  and on the starting ray use zero. Compute the span before normalizing start
  modulo 360°. Require a positive span at most 360°. With Pad a partial
  sweep holds its end color from its ending ray to the next starting ray.
- Extend defaults to Pad (clamp t). Repeat uses t − floor(t), including
  negative t. Reflect uses 1 − abs(1 − (t modulo 2)) with positive modulo.
  A repeated integer samples the beginning; an odd reflected integer
  samples the end. A full sweep closes on its starting ray.
- Accept 2 through 16 stops, finite offsets in [0,1], nondecreasing in
  caller order. Hold the nearest end color outside the stop interval.
  At coincident offsets the first color ends the incoming segment and
  the last starts the outgoing one. Exactly at the offset choose the
  last, including at 0 and 1. Keep discontinuities in the prepared ramp.
- Reject nonfinite inputs, coincident linear endpoints, invalid radii,
  angles, stops and unrepresentable prepared mappings with typed errors.
  Validate immutable descriptions on construction. Check frame-dependent
  numeric limits at preparation before issuing draws. Zero-area draws do
  no work. For very small spans and large offsets, the probe below must
  establish a representable evaluation domain for both renderers.
- Colors at stops use pert-ui's straight linear sRGB Color. Convert straight
  channels into Oklab (default), linear sRGB or encoded sRGB; multiply those
  components by alpha and interpolate them and alpha. At positive alpha
  divide out alpha, convert back to linear sRGB, clip out-of-gamut RGB to
  [0,1], then premultiply for compositing. At zero alpha return transparent
  black. Exact stop colors bypass the round trip. Test transparent red to
  opaque blue, equal-alpha black to white in all spaces, and an interior
  color requiring gamut clipping.

## Recording, composition and rendering

1. Separate authoring ownership from recorded references. In the public
   DisplayList, use a small RecordedPaint containing a solid or a ramp ID
   and its positioned geometry. Replace color on FillRect, DrawMask and
   DrawVector, and the paints inside DrawShape and its border. Publish
   read-only ramp lookup and the evaluation reference for external backends.
   Move PreparedShape::new's border input to optional width, since coverage
   only depends on width. Migrate repository consumers and API listings.
2. Prepare the immutable ramp once from at most 16 source stops. The
   baseline evaluator finds the enclosing interval and evaluates the color
   contract directly, using stored components premultiplied in the selected
   space. This gives every valid stop list a bounded representation,
   including 16 alternating black/white stops. Retain both sides of jumps.
   Compare phase 0's segmented approximation in slice 0: at most 64
   linear-premultiplied segments, used only where a bound of half an encoded
   byte/channel versus the direct reference before dithering is established
   (compare over opaque black and white and in alpha). Otherwise use direct
   evaluation on both backends. Budget exhaustion never invalidates a theme.
3. Intern ramp data independently of placement. Composition remaps
   component-local references into the owned scene after culling, including
   VectorRasterizer::prepare and prepare_fills when rebuilding scenes, and
   both shape paints; a live scene owns all referenced bytes after unmount.
   RecordedPaint stays Copy + PartialEq. IDs are local to a list.
   Cache across scenes by content hash plus equality of the immutable ramp;
   a changed stop at the same numeric ID must update the upload.
   A palette change invalidates its dependent components. Translation, scale,
   clipping and changing coverage alone reuse the ramp. Resolve geometry
   using the original local coordinates before clipping. In particular
   FillRect currently intersects its rect both in Painter and during composition.
   Keep the original endpoints through both intersections. A turned mask turns
   its source UVs; a paint in destination coordinates stays anchored there.
4. Evaluate t at the destination physical pixel center using FrameMapping
   and the inverse placement. Composition translates and uniformly scales
   local geometry. Prepare in f64 relative to the clipped span's origin
   before narrowing to checked f32 coefficients. Reject overflow or a
   collapsed nonzero span. CPU is the scalar reference. wgpu keeps inline
   solid colors and uses a separate buffer for positioned gradient records
   and ramp data. Preserve the solid shape upload stride.
   Apply coverage once. DrawShape keeps its documented border-over-fill
   equation with both colors evaluated at that pixel. The vector fill
   fallback applies paint at the destination after mask sampling; the
   GPU stroke atlas supplies only coverage. For premultiplied shape blending
   emit the covered premultiplied source. For straight-alpha rect, stroke
   and mask pipelines emit straight RGB and coverage times source alpha;
   zero-alpha source emits zero. Apply each factor once.
   Reuse solid row/box and rectangle batching fast paths only when their uniform-color premise
   holds. Backend capacity or unsupported paint produces a reported error.
5. Proposed dither: a fixed integer-indexed 8×8 Bayer matrix, identical in
   both backends. Use physical window pixel indices, including the origin
   for cropped captures. For a gradient source, perturb each straight
   encoded RGB channel by ((rank + 0.5)/64 − 0.5)/255, clamp, decode and
   premultiply again before coverage and linear source-over. Leave alpha
   and solid paints exact. At zero alpha skip the conversion; evaluate tiny
   positive alphas against the f64 reference without division by rounded-zero
   alpha. Slice 0 compares shared table-based transfer functions against the
   continuous sRGB functions, and measures dither on/off CPU and GPU cost.
   This source dither cannot remove bands caused by alpha quantization alone.
   Include an alpha-only dark scrim over light and dark grounds; record
   residual bands separately from opaque RGB-ramp banding.
   For the opaque banding gate, render a 1024×64 horizontal encoded-sRGB
   gray ramp from byte 32 to 49. Compare each aligned 8×8 tile's mean
   encoded intensity with the ideal mean before quantization. Proposed
   pass: the RMS of tile-mean errors is at most half the undithered RMS,
   and identical frames yield identical bytes. Inspect the result at 1:1
   for visible Bayer structure. Compare dithered CPU and GPU against each
   other at the same coordinates: at most one byte/channel per draw away
   from classified jumps, including coverage edges on an identical
   destination. At jumps use the side-specific tier below with the same
   Bayer rank. Report accumulated error for overlapping draws separately.

## Theme, inspection and first consumer

Use typed paint roles for button fills and the decorated floating Window
title surface, with defaults keyed by role and checked for completeness.
Preserve the existing button state colors and the title surface's
SurfaceContainer base and SurfaceHover overlay in their existing bounds.
Allow independent rest and hover overrides. Implement the title example
through these roles as well as a custom Painter. Relative geometry normally
uses fractions of the supplied local rect. For a fixed-width transition,
allow an endpoint offset in logical lengths from the rect's local left edge
(positive x points right in both LTR and RTL). This extends the phase 0 fraction-only
theme sketch; neither form depends on physical pixels or window position.
Stops can be literal colors or ColorRole references. Track paint-role reads
even for literal paints so replacing an override invalidates its readers.
Keep paint overrides in theme-manager state per mode, preserving Palette's
solid-color value semantics. Resolve per mode and track every color role read;
replacement and reset follow palette override semantics. A zero-size
target records nothing. Expose the same role resolution to custom
components. Keep seeded, light and dark solid defaults and existing
contrast behavior. When checking gradient contrast against a uniform
foreground on a known backdrop, use the minimum over every composited
segment, both sides of jumps and interior luminance crossings. Unknown
backdrops cannot yield a guaranteed ratio.

Advance InspectionSnapshot's schema from 18 to 19 (use the next free
version if another change lands first) for owned descriptions of the last
painted effective foreground, background and border. A gradient has no
single effective RGBA; describe its kind, resolved logical window geometry,
ordered original stops, interpolation and extend. Preserve absent/unmeasured
values and the distinction between absence and an observed gradient.
Retain border width and carry its paint. Validate closed DTOs and version
handling, with round-trip and rejection tests. Capture continues reading
actual rendered pixels.

Migrate the color picker's alpha strip first. Compare alpha, checkerboard
compositing, endpoints and pointer-to-value mapping at fractional scale.
Keep its HSV hue strip and two-dimensional saturation/value image until
a separate equivalence test warrants replacing them.

## Delivery and acceptance

- **Slice 0, before changing the API.** Prototype a ramp and both samplers.
  Include clipped linear tracks, radial center/inner edge, sweep seams and
  hard stops at and next to pixel centers, repeated negative coordinates,
  translucent fills, mixed shape borders, fractional placement, zoom and
  scales 1, 1.25 and 2. Compare to high-precision interpolation separately
  from CPU/GPU parity. Report worst error and locations, render times and
  upload bytes for changing geometry, changing stops and repeated ramps.
  Benchmark dark panels, the alpha strip, the phase 0 gradient bars, and
  the existing solid mixer and diagram baselines. Record machine, backend,
  release build and variability. Submit cap, error bounds, numeric domain
  and benchmark results for decision before proceeding. Define a measured
  error margin in t for hard stops, Repeat wraps and sweep seams. Test
  phase/angle comparisons with signed ray tests near seams. Outside the
  margin both backends must choose the same side; inside, accept only the
  two adjacent one-sided limits within the ordinary blending tolerance.
  Record affected pixel counts and maximum screen-space width; reject a
  mapping if the ambiguity exceeds half a physical pixel or spans multiple
  jumps. Exact phase ties constructed in shared arithmetic follow the
  specified side. Test both sides and the tie independently.
- **Proposed probe gates.** Undithered direct evaluation versus f64: at
  most one encoded byte/channel on black and white, including alpha, away
  from classified jumps. Check the segmented representation against its
  stricter bound separately. On the solid baselines require unchanged
  uploaded bytes and median render time within 5% of baseline in paired
  runs (rerun if variability obscures 5%). Report gradient time separately
  against the stretched-image baseline and seek a maintainer decision on
  measured cost before adoption. Preserve existing coverage tiers.
- **Runtime and renderer slice.** Land public descriptions, validation,
  owned recording and consumption by both renderers together. Verify
  coverage against the existing shape/stroke tiers, gradient interiors
  within one encoded byte per channel between backends, and the existing
  exact solid tier. Revisit limits or evaluation if the probe misses a gate.
  Exercise detached scenes, retained repaint, culled resources and cropped
  captures. An unchanged scene must produce identical captures.
- **Consumer slice.** Ship role resolution, inspection migration, picker
  migration, a facade-only custom-widget example and gallery coverage.
  Verify reset, theme switching, schema rejection, no hidden-view repaint
  and the gray-text/black-to-white interior contrast minimum. Repeat the
  benchmarks; investigate solid-path regressions outside baseline noise
  before merge, including the existing RENDER-05 workloads.
- Follow the [repository verification gates](../../AGENTS.md) for the
  public API and affected native backends. Update rustdoc, contracts,
  rendering-platform, manual, changelog and generated API listings with
  the implementation. Record platforms actually exercised, the independent
  review and the maintainer decision alongside the measurements.

## Independent review

Claude Code reviewed the draft against the runtime and renderer sources on
2026-11-24. Its verdict on the first draft was to revise before approval.
The revised probe compares direct evaluation with bounded segmentation,
measures dither cost and alpha-only banding, and classifies discontinuity
pixels. The recording contract now covers scene-local ramp identity,
upload stride, f64 preparation and pipeline-specific output. The fixed-width
title-bar case is included in the consumer checks. Numerical limits and
performance thresholds remain proposals pending measurement.

In a second review Claude found no blocker to the probe and confirmed the
40-pixel title example. It requested a banding metric, a dithered parity
threshold and an explicit RTL anchor. The dither gate and theme coordinates
above now specify these. Full implementation awaits the probe results and
maintainer approval.
