# Phase 1 spec: analytic shapes (B + C)

Status: **approved by the maintainer on 2026-11-06**, after three Codex
reviews; see [Decisions](#decisions). Written 2026-11-06 on branch
`rendering-phase0`, after the [phase 0 decisions](phase0-report.md#decisions).
Nothing here exists yet. The names were approved; the API listing's diff
shows the final ones before merge. Once implemented, the normative
parts move into rustdoc and
[docs/rendering-platform.md](../../docs/rendering-platform.md), and this file
is deleted. A separate plan will order the work and the measurements; it
refers to this spec and does not change it.

## Goal

The silhouettes `Painter` draws today through the shape cache become one
backend primitive: rounded and cut rectangles, arcs and triangles. Both
renderers compute their coverage per pixel from shared prepared parameters
and the same closed-form formulas. Then:

- **No coverage is rasterized for shapes.** The UI thread rasterizes,
  caches or uploads no shape coverage or image resource. The knob's value
  arc today makes a new mask for every value.
- **A fill and a border drawn by one call are one command**, composited as
  one covered silhouette, so no background leaks between them.
- **Shapes stay sharp during a zoom gesture**, because nothing prepared is
  stretched.
- **Corners can differ:** a size and a kind per corner.

Gradients come in phase 2. This phase reserves their place (`Paint`).

## Scope and non-goals

In scope: every current user of the shape cache, which means `fill_shape`,
`stroke_shape`, the focus ring, `stroke_arc` and `fill_triangle`.

Not in scope:

- **Ellipses.** `fill_ellipse` and `stroke_ellipse` keep their path
  semantics through `Path::ellipse` and the vector rasterizer, unchanged.
  - A circle is a square `Shape` with round corners of half its side. This
    is exact, and `draw_shape` gives it a border.
  - A non-circular ellipse needs the true offset curve for its stroke,
    which a closed-form distance does not give. It stays a path until a
    later phase specifies one.
- **General paths, polylines and text.** `fill_path`, `stroke_path` and
  `stroke_polyline` stay on the vector rasterizer (candidate F,
  RENDER-04), and text and icons stay masks.
- **Gradients, shadows, rounded clips, group opacity and transforms:**
  later phases.
- **Inspection** is unchanged. Shapes are painted with solid colors, which
  `BoxModelInspection` already reports.

## Public types

All fields are private. Values are validated when constructed, so every
value a consumer can hold is drawable. All types are `Clone`, `Copy`,
`Debug` and `PartialEq`, so `DrawCommand` stays `Copy` and `PartialEq`.

### Shape (painter side)

```rust
/// One corner: square, a circular arc, or a 45° chamfer.
pub struct Corner { /* kind, size: Length */ }
impl Corner {
    pub const SQUARE: Self;
    pub const fn round(radius: Length) -> Self;
    pub const fn cut(size: Length) -> Self;   // removes `size` from both sides
    pub const fn kind(self) -> CornerKind;
    pub const fn size(self) -> Length;        // 0 for SQUARE
}
#[non_exhaustive]
pub enum CornerKind { Square, Round, Cut }
#[non_exhaustive]
pub enum CornerPosition { TopLeft, TopRight, BottomRight, BottomLeft }

/// A rectangle's silhouette: one corner per position. Painting only;
/// layout and hit bounds stay rectangular.
pub struct Shape { /* [Corner; 4] */ }
impl Default for Shape { /* RECTANGLE */ }
impl Shape {
    pub const RECTANGLE: Self;
    pub const fn rounded(radius: Length) -> Self;       // all four round
    pub const fn rounded_top(radius: Length) -> Self;   // top two round
    pub const fn cut_top_right(size: Length) -> Self;
    /// Replaces the corner at `position`.
    #[must_use]
    pub const fn with_corner(self, position: CornerPosition, corner: Corner) -> Self;
    pub const fn corner(self, position: CornerPosition) -> Corner;
}

/// What `draw_shape` paints. A later fill or border call replaces the
/// earlier one; neither accumulates.
pub struct ShapeStyle { /* fill: Option<Paint>, border: Option<Border> */ }
impl ShapeStyle {
    pub fn fill(color: Color) -> Self;
    pub fn border(width: Length, color: Color) -> Self;
    #[must_use] pub fn with_fill(self, color: Color) -> Self;
    #[must_use] pub fn with_border(self, width: Length, color: Color) -> Self;
}
```

A facade-only example, which becomes a doctest:

```rust
use pert_ui::prelude::*;
use pert_ui::{Corner, CornerPosition, Shape, ShapeStyle};
fn paint(p: &mut Painter) -> Result<(), DrawError> {
    let shape = Shape::rounded(Length::new(8.0)?)
        .with_corner(CornerPosition::TopRight, Corner::cut(Length::new(12.0)?));
    let style = ShapeStyle::fill(Color::from_linear(0.1, 0.1, 0.12, 0.85)?)
        .with_border(Length::new(1.0)?, p.color(ColorRole::Outline));
    p.draw_shape(Rect::new(10.0, 10.0, 200.0, 120.0)?, shape, style)
}
```

### Command (backend side)

```rust
// A new variant of the #[non_exhaustive] DrawCommand, itself #[non_exhaustive]:
DrawShape {
    geometry: ShapeGeometry,
    fill: Option<Paint>,
    border: Option<Border>,
    clip: Option<Rect>,
}

#[non_exhaustive]
pub enum ShapeGeometry {
    /// A rectangle in window logical coordinates with its corners.
    Rect { rect: Rect, shape: Shape },
    /// A circular arc stroked with round caps.
    Arc(ArcGeometry),
    /// A filled triangle; never degenerate.
    Triangle(TriangleGeometry),
}
pub struct ArcGeometry { /* center: Point, radius: Length, start: f32 in 0..360,
                            sweep: f32 in 0..=360, width: Length */ }
impl ArcGeometry {
    /// Reduces `start_degrees` modulo 360 in f64, as `stroke_arc` does today.
    pub fn new(center: Point, radius: Length, start_degrees: f32,
               sweep_degrees: f32, width: Length) -> Result<Self, PathError>;
    /* getters */
}
pub struct TriangleGeometry { /* [Point; 3] */ }
impl TriangleGeometry {
    /// `None` for collinear or coincident points, by today's f64 cross-product test.
    pub fn new(vertices: [Point; 3]) -> Option<Self>;
    /* getter */
}
#[non_exhaustive]
pub enum Paint { Solid(Color) }
pub struct Border { /* width: Length, paint: Paint */ }
impl Border {
    pub const fn new(width: Length, paint: Paint) -> Self;
    pub const fn width(self) -> Length;
    pub const fn paint(self) -> Paint;
}
// ArcGeometry: center(), radius(), start_degrees() (reduced, 0..360),
// sweep_degrees(), width(). TriangleGeometry: vertices().
```

Provenance and validity are separate concerns:

- **Validity.** Every `ShapeGeometry`, `Border` and `Paint` value is valid
  by construction.
- **Provenance.** `DrawCommand` is publicly constructible, as today, but
  only the runtime inserts commands into a `DisplayList`.
- **What a backend must check:**
  - a border on an `Arc` or `Triangle` is malformed. Preparation refuses it
    as `ShapePrepareError::BorderNotAllowed`, and the renderers wrap that
    in their typed errors (see [Errors](#errors)). `Unsupported` stays
    reserved for unknown features;
  - an unknown variant of `ShapeGeometry` or `Paint` is reported as
    unsupported, like an unknown command, and never dropped.

## Preparation, shared by both renderers

A public function prepares a `DrawShape` for a frame. Both renderers, and
any external backend, call it, so positions, snapping and clip spans are
decided once, in `f64`, by one piece of code:

```rust
pub struct FrameMapping { /* frame size, sx, sy, s */ }
impl FrameMapping {
    /// `GeometryError::Empty` for a zero frame or viewport extent.
    pub fn new(frame_width: u32, frame_height: u32, viewport: Viewport)
        -> Result<Self, GeometryError>;
}
pub struct PreparedShape { /* span, kind, f32 parameters */ }
impl PreparedShape {
    /// `Ok(None)`: nothing to draw (collapsed, empty span or clipped away).
    pub fn new(geometry: &ShapeGeometry, border: Option<&Border>,
               clip: Option<Rect>, mapping: &FrameMapping)
        -> Result<Option<Self>, ShapePrepareError>;
    /// Frame pixels to evaluate: `[x0, y0, x1, y1]`, half-open.
    pub fn span(&self) -> [u32; 4];
    /// The evaluated parameters, relative to the span origin, for a shader.
    pub fn parameters(&self) -> ShapeParameters;
    /// The reference evaluation of the frame pixel `(x, y)`, at its center
    /// `(x + 0.5, y + 0.5)`, in f32 as specified below. Outside `span()`
    /// the command leaves the pixel unchanged: zero coverage, an infinite
    /// outer distance, no inner distance, and `exact_arithmetic` true.
    pub fn coverage(&self, x: u32, y: u32) -> ShapeCoverage;
}
pub struct ShapeCoverage { /* private */ }
impl ShapeCoverage {
    pub const fn outer(self) -> f32;   // c_o
    pub const fn border(self) -> f32;  // c_b
    /// The outer distance `d_o` the ramp was applied to, in physical pixels,
    /// negative inside, before the thin-stroke factor.
    pub const fn outer_distance(self) -> f32;
    /// The inner distance `d_i`, when a border leaves an inner area.
    pub const fn inner_distance(self) -> Option<f32>;
    /// Whether the exact-arithmetic rule decided this pixel (see Parity tiers).
    pub const fn exact_arithmetic(self) -> bool;
}
```

- **`FrameMapping`** holds what `pert-ui-cpu` uses today. That is the
  frame's physical width and height, the factors
  `sx = frame_width / viewport.width()` and
  `sy = frame_height / viewport.height()` in f64, and the viewport scale
  `s` for lengths.
- **`ShapeParameters`** is a `#[non_exhaustive]` enum by kind, with
  the f32 values the formulas below name. It documents its layout for
  shader authors.
- **`coverage`** is the CPU renderer's evaluation, so the reference lives in
  the runtime and an external backend can test against it.
- **The reference distances** (`outer_distance`, `inner_distance`) and the
  `exact_arithmetic` flag are what the parity tiers classify on. A renderer
  test, in this repository or in an external backend, classifies its pixels
  from them instead of copying the formulas.
- **Only shape commands are prepared:** the function takes the geometry,
  not a `DrawCommand`, so other commands never reach it.

Steps, in this order:

1. **Map.** A position `x` maps to `x·sx` and `y` to `y·sy`. A length (a
   corner size, a border width, an arc radius or width) maps to `ℓ·s`.
2. **Snap the rectangle of `Rect` geometry.** Each edge becomes
   `E = ceil(e·sx − 0.5)` (`sy` for vertical edges). This is exactly the
   span rule `FillRect` uses in `pert-ui-cpu`. If the snapped width or
   height is 0, the result is `None`.
3. **Apply the overlap rule (CSS backgrounds 3)** to the mapped corner
   sizes against the snapped extents. For each side with a positive sum of
   its two corner sizes, take the side's length divided by that sum; if
   the smallest such ratio `f` is below 1, multiply every corner size by
   `f`. A side whose corners are both square imposes no limit.
4. **Border width.**
   - A border of zero width is absent: preparation and composition proceed
     without it, and the fill remains. A command whose only paint is such a
     border draws nothing, as `stroke_shape` with zero width does today.
   - For `Rect` geometry, `W = max(1, floor(w·s + 0.5))` whole physical
     pixels for a positive `w`.
   - For an arc, the width is `w·s`, unsnapped.
5. **Inner geometry.** If `2W ≥ min(snapped width, snapped height)`, there
   is no inner area: the whole silhouette takes the border paint, as
   `stroke_shape` fills today. Otherwise the inner rectangle is the snapped
   one inset by `W`. Its corners keep their kinds:
   - a round corner becomes `max(r − W, 0)`;
   - a cut corner becomes `max(r − (2 − √2)·W, 0)`.

   These are the exact inward offsets. No `Rect` is constructed for the
   inner geometry, so this step cannot fail.
6. **Span.** The pixels to evaluate are the bounding box grown by one
   pixel: the snapped rectangle, the arc's square of `R + t`, or the
   triangle's box, rounded outward to whole pixels. The span is
   intersected with the frame and with the clip, whose edges snap by the
   same rule as step 2. Every span edge is therefore an integer, and no
   rasterizer edge test decides a pixel: a GPU quad covers exactly this
   integer span. Each pixel in the span is evaluated, and the coverage
   formula decides it.
7. **Relocate and bound.** Every parameter is expressed relative to the
   span's origin, in f64, before it is narrowed to f32.
   - **`Rect` geometry:** an edge farther than `M` beyond the span, where
     `M` is the largest corner size plus 2 pixels, is moved to the span
     edge plus `M`. Coverage inside the span does not change: such an edge
     and its corner regions lie wholly outside the span, and a coverage of
     0 or 1 stays 0 or 1.
   - **Order:** the inner geometry (step 5) is derived before relocation,
     and outer and inner edges are relocated independently. The inset is
     never rebuilt from relocated outer edges.
   - **Domain:** after this, every parameter, and every point an evaluation
     forms from them after narrowing to f32, must lie within ±2^16 physical
     pixels of the span origin. Otherwise the result is `ShapePrepareError::OutOfRange`.
     Examples are an arc of radius 10^6 pixels and a triangle vertex far
     outside the frame. Compose already refuses non-finite mapped geometry
     with `GeometryError::NonFinite`.
   - **Collapse:** the triangle test runs again on the narrowed f32
     vertices, promoted to f64. If two coincide, an edge is shorter than
     1/256 physical pixel, or their cross product is zero, the triangle
     draws nothing (`Ok(None)`), whatever the logical test said. Evaluation
     therefore never meets a zero-length segment or a collapsed winding. A
     rectangle collapses by step 2.
   - **Trigonometry stays here:** the arc's `cos h`, `sin h` and the rotation
     by `−m` are computed in f64 and passed as f32 values. The per-pixel
     evaluation uses only `+ − × /`, `sqrt`, `min`, `max`, `abs` and
     comparisons.

## Coverage, the normative part

Both renderers evaluate these formulas in f32, for each pixel center
`p = (i + 0.5, j + 0.5)` in the span, relative to the span origin. The
formulas are the definition. Where one is a distance bound rather than the
exact Euclidean distance, it still defines the coverage.

### Distances

**Rect geometry.**
- Let `x0, y0, x1, y1` be the snapped edges.
- **Box distance:** `q.x = max(x0 − p.x, p.x − x1)` and
  `q.y = max(y0 − p.y, p.y − y1)`, which are differences of integers and
  half-integers, exact in f32 within the domain. Then
  `d_box = min(max(q.x, q.y), 0) + length(max(q, 0))`. Here, and wherever
  a length has a zero component, `length` returns the other component
  without a `sqrt`.
- **Corner terms.** Each corner with size `r > 0` contributes one term. For
  the top-left corner (the others mirror it):
  - **round:** only inside its region, `p.x < x0 + r` and `p.y < y0 + r`.
    With `c = (x0 + r, y0 + r)` the term is `length(p − c) − r`. Outside
    the region the corner contributes nothing.
  - **cut:** everywhere, as a half-plane:
    `(x0 + y0 + r − p.x − p.y) / √2`. Inside the rectangle the half-plane
    removes exactly the corner triangle, even when `r` reaches the full
    side.
- **Total:** `d = max(d_box, every corner term)`.

The distance is continuous:
- **Round corners** are bounded by each corner's own size, not by the
  centerlines, so a single corner may exceed half the rectangle, as the
  overlap rule allows. On a region's boundary the round term equals
  `d_box`. Outside the box this is the exact distance.
- **Cut corners** are a maximum of half-planes, with no region boundary and
  so no jump. It is a distance bound near the chamfer's ends.

**Arc.**
- `R` is the radius and `t = w/2` half the width.
- `m = start + sweep/2` in radians, clockwise from +x with y down, reduced
  in f64 during preparation. `p'` is `p − center` rotated by `−m`, and
  `h = sweep/2`.
- If `sweep = 360`: `d = |length(p') − R| − t`.
- Otherwise, with `p'' = (p'.x, |p'.y|)` and the prepared `(cos h, sin h)`:
  - if `p''.y·cos h ≤ p''.x·sin h` (inside the sector, valid for
    `0 < h < π`): `d = |length(p'') − R| − t`;
  - else `d = length(p'' − R·(cos h, sin h)) − t`, the round cap.

**Triangle:** the signed distance to the three edges. The magnitude is the
minimum distance to the three segments, and the sign is negative where all
three edge functions agree with the winding.

### Coverage

- **Ramp:** for a distance `d`, `c(d) = clamp(0.5 − d, 0, 1)`.
- **Thin strokes.** An arc whose width `w·s` is below one physical pixel is
  evaluated with width 1, and its coverage is multiplied by `w·s`. Its
  coverage therefore falls to zero as the width does, rather than
  approaching the ramp's 0.5. The same applies to the round caps and the
  full ring. `Rect` borders are at least one pixel (step 4), so the rule
  does not arise for them.
- **Outer and inner:** `c_o = c(d_o)` for the outer geometry. With a
  border and an inner area, `c_i = min(c(d_i), c_o)`. The `min` makes the
  nesting hold even where the formulas are approximate. With a border but
  no inner area, `c_i = 0`. Without a border, `c_i = c_o`. The border
  coverage is `c_b = c_o − c_i ≥ 0`.

### Composition

The fill lies under the border, as fill-then-`stroke_shape` paints today,
but inside **one** covered silhouette:

```text
src = B·c_b + F·(c_o − B.a·c_b)
```

- `F` and `B` are the premultiplied linear fill and border paints, zero
  when absent.
- In the interior, `c_o = 1` and `c_b = 0`, so `src = F`.
- In an opaque border band, `src = B`.
- At the outer edge with both paints opaque, the alpha is `c_o`. Coverage
  is not counted twice, which is the seam that two sequential commands
  produce.

`src` is blended once, source-over, onto the stored pixel, as for every
command. This does not reproduce the two-command pair byte for byte. Today
the result is stored between the fill and the border, and fractional edge
coverage is applied twice; that difference is the intended fix.

### Parity tiers

The CPU renderer is the reference. Classification works per pixel on the
CPU distances that `ShapeCoverage` reports, with a margin `δ` for the f32 differences between the
renderers. Those differences come from fused multiply-add contraction,
`sqrt` precision and evaluation order; trigonometry is computed once in
preparation.

`δ` is **provisional at 1/16 pixel**. The ±2^16 domain sets the magnitude
of the error: one rounding there is about 2^16·2^−24 = 1/256 pixel. No
operation-by-operation bound has been derived, so δ is set together with
`N` by the phase 1 parity run, including cases at the domain's edges. The
tiers below are test criteria for that corpus, not a proof for every input.

- **Exact arithmetic.** For a pixel of `Rect` geometry outside every outer
  and inner round-corner region, where every cut term is at most −0.5 −
  δ, the *saturated* coverage is decided by `q` alone:
  - either component ≥ 0.5 gives coverage 0;
  - otherwise both are ≤ −0.5, and the excluded corner terms leave
    coverage 1.

  `q` is computed by exact subtractions and `max`, so a renderer may take
  this early exit. The full distance formula may still use a `sqrt` there,
  for example diagonally outside a corner, but the coverage it produces is
  the same 0 or 1.
- **Clear.** A pixel is clear when, for every boundary that applies (the
  outer one, and the inner one with a border), `|d| ≥ 0.5 + δ`. Each such
  coverage is then 0 or 1 in both renderers.
- **Edge.** Every other pixel in the span.
- **Outside the span**, including pixels the clip removes, the command
  leaves the pixel unchanged, and the pixel keeps its destination's tier.
- **The classification** is therefore, per pixel:
  `exact_arithmetic || (|outer_distance| ≥ 0.5 + δ and the same for
  inner_distance when present)`. Such a pixel is exact-arithmetic or clear,
  and every other pixel is an edge pixel.

Tiers:

- **Exact (byte for byte):** exact-arithmetic and clear pixels, when the
  paints present are opaque exact-sRGB colors and the destination was in
  the exact tier.
- **±1 per channel:** exact-arithmetic and clear pixels otherwise, which
  means translucent paints, arbitrary linear colors, or a destination
  already in the ±1 tier. This is today's tolerance tier.
- **Edge tier, at most `N` per channel:** edge pixels. As decided, `N` is
  set by measurement in phase 1: the largest difference over the parity
  corpus on the available GPUs, at most 4. If the measurement exceeds 4,
  the implementation is corrected, not the tier. A destination that already
  differed adds its own tolerance.

Every span edge is an integer (preparation step 6). The documented
exception for edges the hardware snaps therefore does not arise for shapes,
clip edges included.

## Behavior changes

All of these are visible, and they are recorded in the CHANGELOG and the
rendering contract:

- **Border widths are whole physical pixels,** rounded to nearest with a
  minimum of 1.
  - Today `stroke_shape` only enforces the minimum, and the band's
    pixel-center edges make a fractional width 1 or 2 pixels wide
    depending on its phase.
  - At scale 1, a width of 1.49 now gives 1 pixel and 1.5 gives 2.
- **Shape edges follow `FillRect`'s pixel-center rule.**
  `docs/rendering-platform.md` says the renderer does not "silently
  round application coordinates". The contract is amended to say that
  rectangles and shape edges use this rule, as `FillRect` always has,
  while arcs, triangles, paths, text and images keep sub-pixel placement.
- **Corners:** a one-pixel distance ramp instead of four sampled rows. Arcs
  and triangles: the ramp instead of 4×4 supersampling.
- **`rounded_top` may be larger,** up to the full height when the width
  allows, instead of half the smaller side (CSS overlap rule).
- **Live zoom and magnification.** During a zoom gesture, retained shape
  commands are mapped by the stretch factor and evaluated at the resulting
  scale, so they stay sharp. Snapping and the minimum border width apply
  after that mapping. The eight-physical-pixels-per-logical-pixel
  preparation cap does not apply to shapes, because nothing is prepared.
  The contract's zoom section is amended to say so.
- **`fill_shape` and `stroke_shape` stay separate commands.** Their
  composition is as today, two commands, with the seam today has. Only
  `draw_shape` composites the two as one.

## Painter API

| Today | After phase 1 |
|---|---|
| `enum Shape { Rectangle, Rounded(Length), CutTopRight(Length), RoundedTop(Length) }` | `struct Shape` (above). Breaking. |
| `fill_shape(rect, shape, color)` | Kept; one `DrawShape`, or a `FillRect` for `Shape::RECTANGLE` as today. |
| `stroke_shape(rect, shape, width, color)` | Kept; one `DrawShape` with a border and no fill. |
| fill, then `stroke_shape`, in the same pass | New `draw_shape(rect, shape, ShapeStyle)`, one command. |
| `stroke_arc(…)`, `fill_triangle(…)` | Kept signatures and rules; `Arc` and `Triangle` geometry. |
| `fill_ellipse`, `stroke_ellipse` | Unchanged (paths). |

**What retires** (AGENTS.md "new way" rule):
- `ShapeCache`, the corner decomposition into `FillRect`s and clipped corner
  masks, and the arc and triangle masks. This resolves RENDER-01.
- **Built-in widgets** that fill and stroke one shape adjacently, in the
  same `paint` pass, migrate to `draw_shape`. Components that fill in
  `paint` and stroke in `paint_foreground`, so that the border lies over
  their children, keep the split: `containers.rs`, `views.rs`,
  `toolbar.rs` and `windows.rs` are examples. One command cannot sit both
  under and over the children. The pair stays valid for custom widgets.

**Migration:**

| Old | New |
|---|---|
| `Shape::Rounded(r)` | `Shape::rounded(r)` |
| `Shape::RoundedTop(r)` | `Shape::rounded_top(r)` |
| `Shape::CutTopRight(c)` | `Shape::cut_top_right(c)` |
| `Shape::Rectangle` | `Shape::RECTANGLE` |
| `match` on `Shape` | `Shape::corner` |

Repository consumers, examples and docs migrate in the same change.

## Errors

- **Construction:**
  - `Length` refuses non-finite and negative sizes and widths, as today.
  - `ArcGeometry::new` returns
    `PathError::InvalidParameter` for a non-finite angle or a sweep outside
    `0..=360`, and `stroke_arc` returns it as
    `DrawError::Path`, as today. `stroke_arc` keeps drawing nothing for a
    zero sweep or width, and `fill_triangle` for collinear points.
  - Unrepresentable bounds return
    `DrawError::Runtime(RuntimeError::Geometry(…))`, as today.
- **Shape masks are gone.** `DrawError::CoverageTooLarge` no longer arises
  for shapes, arcs or triangles. Its rustdoc
  (`drawing.rs`), and the error sections of `fill_shape`, `stroke_shape`,
  `stroke_arc` and `fill_triangle`, drop those cases; the variant stays for
  paths and text.
- **Preparation:** `ShapePrepareError`, `#[non_exhaustive]`, with
  `OutOfRange` (step 7) and `BorderNotAllowed` (a border on an arc or a
  triangle). The renderers wrap it in new typed variants, for example
  `RenderError::Shape(ShapePrepareError)` and
  `CpuRenderError::Shape(ShapePrepareError)`, not in the
  string-carrying `Graphics` or the feature-level `Unsupported`.
- **Captures and presentation keep the cause.** Today's conversions would
  lose it: the CPU capture conversion (`pert-ui-cpu/src/lib.rs`) and the
  native one (`pert-ui/src/platform/mod.rs`) turn unmatched errors into
  `CaptureError::Backend(String)`, and software presentation
  (`platform/presenter.rs`) turns CPU errors into
  `RenderError::Graphics(String)`. Instead:
  - a new `CaptureError::Shape(ShapePrepareError)`, serialized in
    serde's externally tagged form like `CaptureError::Limit`;
  - the three conversions map the shape variant to it, and to
    `RenderError::Shape`;
  - `docs/contracts.md`'s error contract lists the new cause.
- **GPU limit:** a new `LimitError::ShapeRecords { bytes, max }` when a
  frame's shape records exceed the device's
  `max_storage_buffer_binding_size`. It is checked before any allocation
  or upload. The existing `VertexBuffer` checks the maximum buffer size,
  which is a different limit.
- **Malformed command** (a border on an arc or triangle): the typed
  `BorderNotAllowed`, above.
- **RENDER-03,** the stale "ellipse" in `stroke_path`'s rustdoc, is fixed
  in the same change, since it describes `stroke_shape`.

## Renderers (informative)

The contract above is what binds. This is the intended implementation.

- **wgpu:** one pipeline for shapes.
  - Each prepared shape becomes a fixed-size record in a storage buffer.
    The vertex shader pulls the record by instance index and emits its
    integer span as a quad.
  - Consecutive shapes form one instanced draw, and draw order is kept by
    the existing batch boundaries.
  - Records travel through `retained.rs`'s changed ranges.
  - Core wgpu features only, on Vulkan, DX12 and Metal.
- **CPU:** per row, the span where the pixels are exact-arithmetic or clear
  inside is filled as a run, and only the remaining pixels evaluate the
  distance. This is the first solid-span fast path of J2.

## Verification

- **Preparation unit tests:**
  - snapping against `FillRect`'s span for the same rectangle, including
    viewports whose frame size rounds, such as width 3.4 at scale 1;
  - collapse to `None`;
  - the overlap rule with zero sides and one corner beyond half;
  - a cut corner's coverage while its size passes a pixel center,
    continuous with no jump;
  - zero-width borders, alone and with a fill;
  - the domain limit, with relocation of far edges and `OutOfRange` beyond
    it, and the triangle collapse;
  - inner corners;
  - clip spans at edges just around pixel centers;
  - the range limit;
  - angle reduction.
- **CPU renderer, hand-computed pixels**, at scales 1, 1.25 and 2:
  - corner sizes of 0, below one pixel, at the clamp, and a single corner
    past half (round and cut);
  - borders of 1 pixel, at half the extent and beyond;
  - fractional widths (1.49 and 1.5);
  - a translucent fill under a translucent border, and opaque paints at
    the outer edge (alpha equal to `c_o`);
  - arcs approaching zero width and zero sweep, and a full ring;
  - a degenerate triangle;
  - clips and a view scale.
- **CPU/GPU parity:** the corpus in `pert-ui-wgpu`'s `cpu_parity.rs` gains
  these cases at arbitrary sub-pixel phases, classified as above. It runs
  through `pert-ui-native-checks` on each available GPU, with the platform
  recorded. This run sets `N`.
- **Joins:** tabs and window title bars, where `rounded_top` meets a body
  or divider, and the split container frames, at scales 1, 1.25 and 2.
- **Zoom:** retained live zoom and magnification beyond 8×, with captures
  checked for crop and frame/state correlation under those transforms.
- **Contracts:**
  - a malformed or unknown command is reported, not drawn or dropped;
  - a frame of only shapes adds no mask resource;
  - a knob turned through many values makes no shape coverage and adds no
    mask or image resource. Command and record allocations are not
    covered by this claim.
- **Gates:**
  - the API listing gate (`api/`);
  - CHANGELOG entries for the breaking change, the behavior changes and
    the retired cache;
  - the workspace gate, as for a cross-package public API change.
- **Visual review:** before and after gallery captures at scales 1, 1.25
  and 2, with the differences summarized for the maintainer.

## Decisions

The maintainer decided the spec's open questions on 2026-11-06, each as
recommended:

1. **Snapping:** `Rect` geometry snaps to physical pixels.
2. **Fill under the border**, inside one silhouette.
3. **Overlap:** the CSS overlap rule, so `rounded_top` may grow.
4. **Ellipses** are left out of phase 1. Circles are rounded squares.
5. **API names** as proposed.

On 2026-11-07, while the [implementation plan](phase1-plan.md) was being
written, the maintainer added:

6. **Reference distances:** `ShapeCoverage` reports the distances and the
   exact-arithmetic flag that the parity tiers classify on, so a parity test
   needs no second copy of the formulas.

## Review record

**2026-11-06, first Codex review** (codex-cli 0.166.0, read-only, static).
Seventeen findings: two blockers, ten major and five minor. The author
checked each against the code and all held. This revision answers them:

- **Corner selection** (blocker): quadrant selection broke corners past
  half. Replaced by per-corner regions.
- **Ellipse border coverage** (blocker): could go negative. Ellipses left
  out of the phase, and `c_i ≤ c_o` made normative.
- **Composition** (major): the formula counted edge coverage twice.
  Replaced by `B·c_b + F·(c_o − B.a·c_b)`.
- **Snapping** (major): now on the renderer's own per-axis factors, in f64,
  in a shared preparation step.
- **Order of preparation** (major): normalization, snapping, collapse and
  inset now come in a fixed order, without fallible `Rect` construction.
- **Parity tiers** (major): a complete classification, with `δ`, the ±1
  tier kept, and border pixels covered.
- **Clips** (major): spans snap to whole pixels, so no rasterizer edge
  decides a pixel.
- **Thin arcs** (major): the thin-stroke rule.
- **Ellipse stroke semantics** (major): ellipses stay paths.
- **Split foreground frames** (major): kept as two commands.
- **Numeric ranges** (major): range checks and angle reduction in
  preparation, and today's f64 triangle test.
- **Validated public types** (major): private fields, with provenance kept
  separate from validity.
- **Minor:** the cut term is called a bound; constructors, defaults and
  replacement semantics are specified with an example; the error and doc
  migration is listed, with a new limit variant; the behavior changes are
  listed, with join and zoom tests; the allocation claim is scoped.

**2026-11-06, second Codex review.** It found 13 first-round findings
resolved and 4 partly resolved (6, 11, 12 and 14). It added seven: four
major and three minor, with no blocker. All held when checked; the cut
discontinuity was recomputed by hand. This revision answers them:

- **Cut corners** (major): a global half-plane, continuous.
- **f32 safety** (major): every parameter relocated to the span origin,
  far rectangle edges moved, a ±2^16 domain with `OutOfRange`, the
  triangle collapse rule, and trigonometry kept in preparation.
- **Exact classification** (major): straight edges are decided by exact
  subtractions and `max`; `δ = 1/16`, derived from the domain, is an
  inference that the corpus tests.
- **Typed malformed cause** (major): `ShapePrepareError::BorderNotAllowed`,
  with typed renderer wrappers.
- **Minor:** zero-width borders are absent; the public preparation and
  reference-evaluation API is specified; the doctest's `?` is fixed.

**2026-11-06, focused Codex recheck** of the second round's repairs. It
confirmed:
- the cut half-plane;
- far-edge relocation, with outer and inner geometry relocated
  independently, which is now stated;
- the ±2^16 domain as an admission rule;
- the arc sector test;
- the typed leaf errors and the shape of the preparation API.

It found five more, all held and answered:
- **Triangle collapse** (major): a triangle could collapse on narrowing to
  f32. The test now runs again on the narrowed vertices.
- **Captures and presentation** (major): their conversions dropped the
  typed cause into strings. `CaptureError::Shape` and the conversions are
  now specified.
- **δ** (moderate): now provisional, set with `N` by the parity run.
- **`coverage`** (minor): its frame-pixel coordinates and the result's
  accessors are specified.
- **The exact-arithmetic wording** (minor): saturated coverage is decided by
  `q`, with no claim that `sqrt` is absent.
