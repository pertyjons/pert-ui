# Phase 0, item 2 (gradients) and item 6 (peniko as the paint model)

Contribution to `plans/rendering-capabilities.md` phase 0. Facts checked on
2026-11-05 against crates.io's API, docs.rs and the specifications as cited;
anything marked **(memory)** was not re-verified today. Repository paths are
at `rendering-phase0` (78246d1, 0.3.102). Everything under "design sketch"
is a sketch for discussion, not an API.

## A. Gradients

### A1. How pert-ui stores and blends color today

| Fact | Where |
|---|---|
| `Color` is straight-alpha **linear** sRGB RGBA, `[f32; 4]`, every channel validated finite and in `0..=1` | `crates/pert-ui-runtime/src/drawing.rs:240-245`, `:297` (`from_linear`) |
| sRGB entry points decode to linear on construction (`from_srgb`, `from_srgb8`, `from_hex`); `to_srgb8` rounds the encoded value | `drawing.rs:369-450` |
| `Color::mix` interpolates in **premultiplied linear light**; alpha 0 result is transparent black; endpoints bit-exact | `drawing.rs:585-640` |
| Every `DrawCommand` carries straight linear colors (`FillRect.color`, `DrawMask.color`, `DrawImage.tint`, `DrawVector.color`); no paint other than a solid color exists | `drawing.rs:1100-1162` |
| `RasterImage` texels are sRGB-encoded **premultiplied in linear light, then encoded** (straight 255 at α 128 stores 188, not 128) | `crates/pert-ui-runtime/src/resources.rs:208`, test at `:288-297` |
| GPU: solid quads and masks output straight color with `BlendState::ALPHA_BLENDING`; images output premultiplied with `PREMULTIPLIED_ALPHA_BLENDING` from an `Rgba8UnormSrgb` texture (hardware decode before filtering) | `crates/pert-ui-wgpu/src/rectangles.wgsl:11-13`, `masks.wgsl:15-18`, `images.wgsl:15-20`, `textures.rs:66-83`, `lib.rs:1244` |
| GPU surface is required to be an sRGB format (hardware encodes on write, decodes for blending) | `crates/pert-ui-wgpu/src/lib.rs:1200-1201` |
| Captures keep linear-premultiplied RGB encoded as sRGB, independent of compositor alpha | `crates/pert-ui-wgpu/src/lib.rs:330-345`, `:648-681` |
| CPU: source-over of linear premultiplied color; **each command's result is stored as 8-bit sRGB-encoded premultiplied bytes** before the next one blends (mirrors an sRGB attachment); decode by 256-entry LUT, encode by binary search over 255 midpoint thresholds | `crates/pert-ui-cpu/src/lib.rs:69-81`, `:307-321`, `:456-470`; `crates/pert-ui-cpu/src/srgb.rs:5-49` |
| CPU fill fast path reuses the last blend result while the stored bytes repeat (valid only because a solid source is constant) | `crates/pert-ui-cpu/src/lib.rs:386-410` |
| Parity contract: exact tier (opaque aligned fills in exact sRGB colors, nearest images, clear) byte-for-byte; tolerance tier (translucency, filtering, text, vectors, fractional edges, arbitrary linear colors) one step per channel | `docs/rendering-platform.md:86-106` |
| Theme: `ColorRole` palette tables keyed by role with const completeness checks; seed palettes are generated in **Oklch on linear sRGB** (an Oklab implementation already exists in-tree) | `crates/pert-ui-runtime/src/runtime/theme.rs:390`, `:541-596`, `:649-730`; `runtime/theme/seed.rs:1-7`, `:213-253` |
| Angle convention: **degrees, clockwise in screen coordinates from the rightward +x radius** (`Transform2D`, `PathBuilder::arc`, knobs) | `crates/pert-ui-runtime/src/transform.rs:13-14`, `numeric_controls.rs:9-10` |
| Own analogs of peniko `Fill`/`Style` already exist: `FillRule::{NonZero, EvenOdd}`, `VectorStyle::{Fill(FillRule), Stroke(StrokeStyle)}` | `crates/pert-ui-runtime/src/geometry.rs:31-40`, `vector_scene.rs:16-21` |
| Inspection reports a single effective `foreground`/`background` `[f32; 4]` (linear RGBA) and a border color; there is no notion of a non-solid paint | `crates/pert-ui-runtime/src/diagnostics.rs:132-167` |
| Today's workaround: the color picker builds its hue, saturation/value and alpha gradients as `RasterImage`s computed on the CPU | `crates/pert-ui-runtime/src/color_picker.rs:11`, `:824-860` |

**Implications.**

- *Interpolation space is a real choice, not a given.* The library's
  native space is linear light (`Color`, `mix`, blending). For black→white
  at t = 0.5: linear light gives linear 0.5 = sRGB byte 188 (`#bcbcbc`,
  visibly too light); sRGB-encoded gives 128; Oklab gives L = 0.5, which for
  a gray is Y = 0.125 = byte 99 (`#636363`). (Arithmetic from the formulas
  in `srgb.rs` and `seed.rs`; for achromatic colors Oklab L = ∛Y.) Linear
  light suits physical fades (glows, fading to transparent); Oklab gives
  perceptually even ramps (CSS's default, A2); sRGB-encoded matches design
  tools and legacy CSS.
- *Premultiplied alpha is already the house rule* (`mix`, the blend
  equation, image storage), so stops must interpolate premultiplied: a
  stop at transparent must not drag its RGB into the ramp. In a polar
  space (Oklch) hue is not premultiplied (CSS Color 4 §13.5, A2).
- *Stops are `Color`s,* hence linear and already validated; a stop
  conversion to Oklab reuses `seed.rs`'s matrices (the code would move to a
  shared private module, not be duplicated).
- *Intermediate colors can leave the sRGB gamut* when interpolating in
  Oklab/Oklch (the gamut is not convex there; Oklch hue arcs leave it
  easily). A per-channel clamp to `0..=1` is a documented, total
  presentation mapping (AGENTS.md permits that); CSS gamut-maps instead.
- *Banding is guaranteed for theme-style gradients* because both backends
  quantize to 8-bit sRGB after every command. Example: `Color::SURFACE`
  (0.045) to `Color::SURFACE_HOVER` (0.075) encode to bytes 60 and 77:
  17 steps across a 400 px button are 23 px bands, which Mach banding makes
  visible. Dark subtle gradients, the most likely theme use, are the worst
  case.
- *The CPU fast path for fills does not apply* to gradients; a gradient is
  per-pixel work on the CPU (A4).

### A2. Gradient kinds, interpolation and extend modes in the references

| | Linear | Radial | Conic / sweep | Interpolation | Extend |
|---|---|---|---|---|---|
| **CSS Images 4** ([ED](https://drafts.csswg.org/css-images-4/)) | angle or `to <side>` in the gradient box | **one center**, circle or ellipse, size keywords (§3.2) | `conic-gradient`, **0deg = up, clockwise** (§3.3.2) | `<color-interpolation-method>`; default **Oklab**, premultiplied (§3.5.2); transition hints set the halfway point (§3.5.2) | `repeating-*` variants (§3.4); otherwise pad |
| **CSS Color 4** ([ED §13](https://drafts.csswg.org/css-color-4/)) | | | | default Oklab, but **legacy sRGB colors (hex, rgb(), hsl()…) interpolate in gamma-encoded sRGB** (§13.3); premultiply, interpolate, unpremultiply; polar hue not premultiplied (§13.5); hue `shorter` (default), `longer`, `increasing`, `decreasing` (§13.6) | |
| **SVG 2** ([MDN radialGradient](https://developer.mozilla.org/en-US/docs/Web/SVG/Reference/Element/radialGradient)) | x1,y1→x2,y2 | **two-point**: `cx,cy,r` end circle, `fx,fy,fr` start (focal) circle | none | `color-interpolation` sRGB default, linearRGB optional **(memory)** | `spreadMethod` `pad`/`reflect`/`repeat`, default pad |
| **peniko 0.6.1** ([docs](https://docs.rs/peniko/0.6.1/peniko/struct.Gradient.html)) | start, end `kurbo::Point` | `RadialGradientPosition` start/end center + radius (two-point; `new_radial` = concentric with r0 = 0) | `SweepGradientPosition` center, start/end angle, **radians from +x, clockwise in y-down** | `interpolation_cs: ColorSpaceTag` (default **sRGB**), `hue_direction`, `InterpolationAlphaSpace` (default Premultiplied) | `Extend::{Pad (default), Repeat, Reflect}` |
| **Skia** (`include/effects/SkGradient.h`, [main](https://skia.googlesource.com/skia/+/main/include/effects/SkGradient.h)) | `LinearGradient` | `RadialGradient` (r > 0) and `TwoPointConicalGradient` | `SweepGradient`, **degrees from +x**, start < end else null | `Interpolation{fInPremul = kNo, fColorSpace = kDestination, fHueMethod = kShorter}`; spaces incl. sRGB-linear, Lab, OKLab, OKLCH (+ gamut-mapped variants) | `SkTileMode` Clamp, Repeat, Mirror, Decal **(memory for the names)** |
| **tiny-skia 0.12.0** ([docs](https://docs.rs/tiny-skia/0.12.0/tiny_skia/struct.RadialGradient.html)) | `LinearGradient` | `RadialGradient::new(start, start_radius, end, end_radius, …)` two-point conical; `None` for empty stops, negative radii, non-invertible transform, degenerate | `SweepGradient` (exists in 0.12; its index description is a copy-paste "A radial gradient") | no interpolation space; `ColorSpace::{Linear (default, = no decoding), Gamma2, SimpleSRGB, FullSRGBGamma}` interprets pixel values | `SpreadMode` Pad/Reflect/Repeat **(memory)** |

Notes for pert-ui:

- **Two-point conical vs simple radial.** The general two-point case
  (SVG `fr`/`fx`, peniko, Skia, tiny-skia) has focal sub-cases and
  undefined regions outside the cone (pixels must stay transparent).
  vello_common follows Skia's three-way split (concentric, "strip" when the
  radii are equal, general focal with an on-circle flag) — see C4. UI uses
  (glows, vignettes, knob highlights, radial meters) need concentric
  circles or ellipses, optionally with an inner radius: one `t = (|p − c| −
  r0)/(r1 − r0)` per pixel, after an inverse affine for ellipses. Recommend
  concentric (with an elliptical transform) first; focal later only with a
  demonstrated need. It maps to peniko as `new_two_point_radial(c, r0, c,
  r1)`.
- **Sweep convention.** CSS (0 = up) differs from peniko/Skia (0 = +x).
  pert-ui already measures angles in degrees clockwise from +x; a sweep
  should use the same, which translates to peniko by `to_radians()` only.
- **Extend.** Pad/Repeat/Reflect is universal; Skia's Decal (transparent
  outside) is the same as a clip and need not exist. Repeat/reflect are a
  `t` remapping (`fract`, triangle wave) before the ramp lookup; no ramp
  changes.
- **Hints and stop fixup.** CSS fixes up non-monotonic offsets by clamping
  to the largest preceding one (CSS Images 4 §3.5.3). pert-ui's rules
  (AGENTS.md: reject invalid explicit values) argue for **rejecting**
  non-monotonic or out-of-range offsets instead; equal adjacent offsets
  (hard stops) are legitimate and must be allowed. Transition hints are
  not needed initially.

### A3. Banding and dithering

- **When it matters.** 8-bit sRGB storage everywhere (A1); dark, low
  contrast, wide gradients (theme surfaces, meter backgrounds) produce
  bands several to tens of pixels wide. CSS Images 4 §3.5 explicitly lets
  UAs dither gradients; Chromium/Skia dither gradients **(memory)**.
  Light, high-contrast or short ramps (a two-color 20 px meter cap) rarely
  band visibly. Banding is not removed by more ramp precision: the
  quantization happens at the 8-bit attachment.
- **Common approaches.** (1) Ordered dither, e.g. an 8×8 Bayer matrix
  indexed by `(x & 7, y & 7)` — integer-indexed, structured but invisible
  at ±½ LSB. (2) A tiled blue-noise texture (e.g. 64×64 or 128×128; CC0
  sets by Christoph Peters exist **(memory)**) — best visual quality,
  needs a table/texture. (3) Interleaved gradient noise (Jimenez,
  SIGGRAPH 2014 **(memory)**): `fract(52.9829189 · fract(0.06711056·x +
  0.00583715·y))` — no texture, but evaluated in f32 with large pixel
  coordinates its value depends on GPU rounding/FMA, so CPU and GPU can
  disagree. (4) Triangular-PDF noise (two uniform samples) for an
  unbiased mean **(memory)**.
- **Where to add the noise.** The GPU writes linear light into an sRGB
  attachment and the hardware encodes and rounds. Noise must therefore
  have an amplitude of ±½ step **in the encoded domain**: compute
  `e = encode(c)`, add `(d − ½)/255`, write `decode(e')`. The CPU does the
  same arithmetic before its own encode. For translucent gradients the
  noise goes on the premultiplied source before blending (approximate,
  fine in practice).
- **CPU/GPU parity.** Key the pattern by **physical frame pixel position**
  (integer), never by gradient coordinates: then a scrolled gradient does
  not swim, captures with crops keep the same pattern, and both backends
  can index the same table exactly. Prefer an integer-indexed table
  (Bayer or a fixed blue-noise table embedded in both renderers, ideally
  one shared `const` array in the runtime) over IGN. The remaining
  difference is the sRGB encode rounding of the hardware vs `srgb::encode`
  — the existing one-step tolerance tier. Solid fills are never dithered,
  so the exact tier is unaffected.
- **Policy.** Dither on by default for gradients, with a renderer-level
  (not per-widget) switch only if captures for tests need undithered
  output; a test can also compare dithered output exactly since the
  pattern is deterministic.

### A4. Cost

**GPU (pert-ui-wgpu today):** six vertices per quad, vertex = position +
color (stride 32, `lib.rs:1233`), three pipelines. A gradient needs per
draw: kind, an inverse affine from frame pixels to gradient space (6 f32),
kind parameters (≤ 4 f32: points or radii or angles), extend, and stops.

- *Inline stops in vertex attributes* would multiply vertex size by the
  stop count; unattractive with six vertices per quad (C in the plan,
  instancing, would make it one record per quad).
- *Storage buffer of gradient records + a stop array,* one `u32` index per
  vertex or instance: portable (wgpu `Limits::default()` allows storage
  buffers in fragment shaders; pert-ui requests `Limits::default()` with no
  features, `lib.rs:437-438`), no texture, no filtering error, exact same
  piecewise-linear arithmetic as the CPU. Per pixel: a few FMAs for `t`
  (linear), a `sqrt` (radial), an `atan2` (sweep), then a loop or binary
  search over ≤ N segments. Re-encoding per changed command fits
  `retained.rs`'s byte-range uploads.
- *Ramp texture (LUT row in an atlas),* as vello does (256–4096 texels per
  gradient, `vello_common` C4) and Skia for many stops **(memory)**: one
  sample per pixel regardless of stops, but hardware bilinear weights have
  limited (≈8-bit) fractional precision and an 8-bit LUT adds its own
  quantization; parity with the CPU becomes "tolerance" rather than
  "same formula". A `Rgba16Float`/`Rgba32Float` LUT sampled nearest at a
  computed texel avoids filtering error at the cost of memory.
- **Recommendation:** resolve every gradient in the runtime (or a shared
  private helper) into a **bounded piecewise-linear ramp in linear
  premultiplied light** (subdivide Oklab/Oklch/sRGB-encoded segments to a
  tolerance, as `color::gradient` and vello_common do with 0.01), store it
  in the display list as an immutable resource like masks, and have both
  renderers evaluate the same segment list in f32. Cap segments (e.g. 64)
  and user stops (e.g. 16) as documented limits.

**CPU (pert-ui-cpu):** per covered pixel: inverse-map the pixel center,
compute `t`, extend, find the segment, lerp 4 channels, dither, blend
(LUT decode + threshold-search encode, as today). Scalar estimate
10–30 ns/pixel **(estimate, not measured; no builds were run)**, so a
400×40 button ≈ 0.2–0.5 ms and a full 1920×1080 background tens of ms —
acceptable for retained UI frames, and a reason to keep gradients out of
per-frame live data (meter bars redraw small rects only). Optimizations
available later: per-row incremental `t` for linear gradients, caching a
row for horizontal/vertical linear gradients.

**Parity tolerance:** with the same ramp, the same `t` formula in f32 and
the same integer-indexed dither, expect the existing one-step tier; the
sweep's `atan2` (GPU approximations differ) and the radial `sqrt` may
need a stated extra allowance near hard stops, to be measured in the
spike.

### A5. Design sketch: a pert-ui-owned `Paint` (not an API)

```rust
// DESIGN SKETCH, not an available API.
enum Paint {                          // shared by FillRect, DrawMask (tint), DrawVector, strokes
    Solid(Color),
    Gradient(GradientId),             // resource in the display list, like MaskId
}                                     // later: Image(ImageId, ImageSampling, Extend)

struct Gradient {                     // validated at construction, immutable
    geometry: GradientGeometry,
    stops: GradientStops,
    space: InterpolationSpace,        // Oklab (default), LinearLight, Srgb, Oklch(HueDirection)
    extend: Extend,                   // Pad (default), Repeat, Reflect
}
enum GradientGeometry {
    Linear { start: Point, end: Point },
    Radial { center: Point, radii: Size, inner: f32 /* fraction 0..1 */ },
    Sweep  { center: Point, start_degrees: f32, end_degrees: f32 }, // clockwise from +x
}
struct GradientStops(SmallVec<[(Offset, Color); 4]>);
```

- **Coordinates:** points in the component's local logical space, as
  rectangles are; the runtime maps them into window space when recorded,
  so view transforms (`ZoomView`) and `QuarterTurn`ed draws apply
  naturally. A gradient spanning a whole component should be expressible
  relative to a `Rect` (e.g. `Gradient::linear_across(rect, Direction)`),
  so widgets need not compute points.
- **Validation (typed `GradientError`, never clamping explicit values):**
  every coordinate, radius, angle and offset finite; offsets in `[0, 1]`
  and non-decreasing (equal allowed for hard stops); at least 2 stops,
  at most `Gradient::MAX_STOPS` (exact documented number); linear
  `start != end` beyond an epsilon, radial radii > 0 and inner < 1, sweep
  `start < end` and span ≤ 360°. vello_common instead silently falls back
  to the first stop's color for such inputs (C4) — pert-ui should refuse
  at construction. Equal colors everywhere could be normalized to
  `Paint::Solid` (an optimization, invisible to the caller).
- **Premultiplied interpolation always;** no `Unpremultiplied` option until
  a demonstrated need (peniko has one, C2).
- **Theme roles.** Follow the `ColorRole` pattern: a typed
  `PaintRole` (e.g. surface-accent, meter-fill, elevated-surface; names are
  placeholders) whose table is keyed by role with const completeness
  checks, defaulting **to a solid of an existing `ColorRole`**, so the
  built-in themes are unchanged and gradients are opt-in for theme
  authors. Stops should be able to refer to a `ColorRole` (resolved at
  paint time, tracked like `Painter::color`), so dark/light modes and
  seed palettes recolor gradients without a second definition. Geometry in
  a theme must be relative (direction + fractions of the component's
  rect), never absolute pixels. `Painter::paint(role)` would return the
  resolved `Paint`, as `Painter::color(role)` returns a `Color`.
- **Inspection/semantics.** `BoxModelInspection.background`/`foreground`
  stay "effective solid color" and become `None` for a gradient (absence
  = not a single color, consistent with "absent until measured"), plus a
  new optional paint description (kind, stops in linear RGBA, space,
  extend, geometry in window logical pixels) with `serde` support and
  `deny_unknown_fields`. Contrast checks against a gradient must use the
  worst contrast over the interpolated and composited ramp, including
  interior extrema: a ramp can pass through the foreground's luminance
  between stops (gray text over black-to-white reaches 1:1 although neither
  stop does). Checking the stops alone is an incomplete heuristic. Semantics
  (roles, values) are unaffected: paint is presentation. Captures need no
  change: they read real pixels.
- **What it replaces/advances (AGENTS.md "new way" rule):** the color
  picker's CPU-generated strip images (`color_picker.rs:824-860`) become
  gradients — the hue strip needs Oklch or HSV-like hue sweeps, so it may
  stay an image if `Oklch` does not reproduce HSV; to be checked.

## B. peniko

### B1. Facts

- **Current version 0.6.1, released 2026-05-15**; MSRV 1.85; licence
  `Apache-2.0 OR MIT` ([crates.io API](https://crates.io/api/v1/crates/peniko)).
  No release since; the plan's 2026-10-27 facts still hold.
- **Dependencies of 0.6.1**
  ([crates.io](https://crates.io/api/v1/crates/peniko/0.6.1/dependencies)):
  `color ^0.3.3` (no default features), `kurbo ^0.13.1` (no default
  features), `linebender_resource_handle ^0.1.1`, `smallvec ^1.15.1`;
  optional `bytemuck ^1.25.0`, `serde ^1.0.228`. Crate root re-exports
  `color`, `kurbo`, `linebender_resource_handle`
  ([docs](https://docs.rs/peniko/0.6.1/peniko/)).
- Already in pert-ui's `Cargo.lock`: `kurbo 0.13.1` (only via
  `usvg`/`svgtypes`, i.e. the facade's optional `svg` feature),
  `linebender_resource_handle 0.1.1` (via parley 0.12), `smallvec 1.16.2`,
  `bytemuck 1.25.2`. New: `color 0.3.3` (2026-05-05, MSRV 1.82) and
  `peniko` itself. Small weight; all MSRVs are far below pert-ui's 1.99.1.
- **Release history** ([CHANGELOG](https://raw.githubusercontent.com/linebender/peniko/main/CHANGELOG.md)):
  0.1.0 2024-02-14 · 0.1.1 2024-05-27 · 0.2.0 2024-09-19 · **0.3.0
  2024-12-18** (breaking: `color` crate replaces own color, `peniko::Color`
  = `AlphaColor<Srgb>`, `ColorStops` newtype, hue direction + interpolation
  space added, `with_alpha_factor` removed) · 0.3.1 2025-01-20 · 0.4.0
  2025-04-30 (`color` 0.3) · 0.3.2 2025-05-02 (semver trick) · 0.4.1
  2025-09-15 (`Font`/`Blob` from linebender_resource_handle) · **0.5.0
  2025-10-01** (breaking: `InterpolationAlphaSpace`, positions as structs,
  `Image`→`ImageBrush`+`ImageData`+`ImageSampler`, `Font`→`FontData`, sweep
  angles documented clockwise in y-down — the reversal of the earlier
  reading, kurbo 0.12, `Mix::Clip` deprecated) · **0.6.0 2026-01-09**
  (breaking: kurbo 0.13, `Mix::Clip` removed) · 0.6.1 2026-05-15
  (`BlendMode::is_destructive`, `with_stops` takes `ColorStops`, kurbo
  0.13.1/color 0.3.3). Unreleased section: empty.
- **kurbo cadence** ([crates.io](https://crates.io/api/v1/crates/kurbo)):
  0.12.0 2025-09-04, 0.13.0 2025-11-27, 0.13.1 2026-05-13 — every kurbo
  minor has forced a peniko minor so far.
- **1.0 plans:** none found (search of peniko issues and the web; no
  roadmap item). Open PR
  [linebender/peniko#165](https://github.com/linebender/peniko/pull/165)
  (2026-02-11, waywardmonkeys) proposes an `imaging_model` with clip types
  (`ClipGeometry`, `Clip` with fill rule and transform) "that we can evolve
  as we define the imaging model and move our renderers to using it";
  reviewers were undecided on scope and on vello depending on it. Signal:
  the clip/layer model is still being designed upstream.

### B2. Item by item against a pert-ui-owned model

| peniko 0.6.1 | Shape | pert-ui mapping / what pert-ui defines itself |
|---|---|---|
| `Brush<I = ImageBrush, G = Gradient>`: `Solid(AlphaColor<Srgb>)`, `Gradient(G)`, `Image(I)`; default transparent | generic over storage | **Mirror** as `Paint` (A5). Solid holds pert-ui's linear `Color`; peniko's solid is **gamma-encoded sRGB, straight, f32**: translation `Color::to_srgb()` → `AlphaColor::<Srgb>::new` (f32 rounding only). Gradient/Image as display-list resource ids, like peniko's `I`/`G` generics intend. |
| `Gradient { kind, extend, interpolation_cs, hue_direction, interpolation_alpha_space, stops }` | public fields, no validation | **Mirror the concepts, not the representation.** pert-ui validates at construction (peniko accepts anything; `with_stops` "clears existing stops and collects the new ones without checks"; negative alpha is "undefined", per `gradient.rs` 0.6.1). |
| `LinearGradientPosition { start, end }` (`kurbo::Point`) | | Same concept with pert-ui `Point`. |
| `RadialGradientPosition { start_center, start_radius, end_center, end_radius }`; `new_radial(center, r)` | two-point conical | pert-ui starts concentric (+ ellipse transform); maps to two-point with equal centers. Ellipses need peniko's brush transform (vello's `set_paint_transform`). |
| `SweepGradientPosition { center, start_angle, end_angle }`, **radians from +x, clockwise in y-down** (0.5.0+) | | pert-ui: **degrees clockwise from +x**, already its convention (`transform.rs:13`); translate with `to_radians()`. Direction identical; only the unit differs. |
| `ColorStop { offset: f32, color: DynamicColor }`, `ColorStops(SmallVec<[ColorStop; 4]>)` | inline 4 | Mirror (`SmallVec<[_; 4]>` is a fine choice); stops are linear `Color`s; translation can tag them `ColorSpaceTag::LinearSrgb` in `DynamicColor` with no conversion. pert-ui adds the max count, monotonicity and finiteness checks. |
| `interpolation_cs: ColorSpaceTag` (16 spaces incl. `Srgb`, `LinearSrgb`, `Oklab`, `Oklch`, `DisplayP3`…; `#[non_exhaustive]`), **default Srgb** | | pert-ui needs a small closed enum (Oklab default, LinearLight, Srgb, Oklch+hue); wide-gamut spaces are meaningless for an sRGB-only pipeline. Note the **default differs**: peniko sRGB, CSS Oklab. |
| `hue_direction: color::HueDirection { Shorter, Longer, Increasing, Decreasing }` (= CSS hue-interpolation-method) | | Mirror only inside an `Oklch` variant, so it cannot be set where it is meaningless. |
| `InterpolationAlphaSpace { Premultiplied (default), Unpremultiplied }` | | pert-ui: premultiplied only (house rule; `Color::mix`). |
| `Extend { Pad (default), Repeat, Reflect }` | | Mirror as is (also for future image paints, where peniko uses it per axis in `ImageSampler`). |
| `ImageBrush<D = ImageData> { image, sampler }`, `ImageData { data: Blob<u8>, format: Rgba8/Bgra8, alpha_type: Alpha/AlphaPremultiplied, width, height }`, `ImageSampler { x_extend, y_extend, quality: Low/Medium/High, alpha }` | | pert-ui has `DrawImage { image, tint, sampling: Linear/Nearest, turn, clip }` and `RasterImage` (premultiplied **in linear light**, then encoded, `resources.rs:288-297`). **Translation trap:** declaring pert-ui bytes `AlphaPremultiplied` to a renderer that premultiplies encoded values (as sRGB-space compositors do) misrenders translucent texels; convert to straight alpha or re-premultiply. pert-ui's `tint` (a color multiplier) has no peniko equivalent beyond `alpha` (vello_cpu has its own `Tint`). |
| `BlendMode { mix: Mix, compose: Compose }`; `Mix` 16 separable/non-separable modes (`Normal`…`Luminosity`); `Compose` 14 Porter-Duff (`Clear`…`PlusLighter`); default Normal/SrcOver; `is_destructive` (Clear, Copy, SrcIn, DestIn, SrcOut, DestAtop) | | pert-ui today: source-over only. For candidate G define only what is needed: group opacity (SrcOver layer with alpha) and perhaps `Plus` for glows. A full Mix×Compose matrix multiplies CPU/GPU parity work and some modes panic in vello_gpu (C3). If adopted later, the peniko split (mix vs compose) is the right shape to mirror. |
| `Fill { NonZero, EvenOdd }` | | Exists: `FillRule` (`geometry.rs:33`). |
| `Style { Fill(Fill), Stroke(kurbo::Stroke) }`, `StyleRef` | | Exists: `VectorStyle::{Fill(FillRule), Stroke(StrokeStyle)}` (`vector_scene.rs:16`). |
| `FontData`, `Blob` (from linebender_resource_handle) | | Already shared with parley 0.12; relevant to a vello glyph path, not to paint. |
| *(absent)* | | pert-ui must define itself: dithering (none in peniko or vello_common's encoder, C4), the CPU/GPU parity tolerance, gradient/paint validation and limits, theme paint roles, inspection DTOs, layer identity for the display list (candidate H), and the layer/clip model (peniko's is still a PR, B1). |

### B3. Options

1. **Mirror the concepts in pert-ui types only** (no peniko anywhere).
   Fits AGENTS.md (no dependency types in runtime/facade); validated
   types; pert-ui's own conventions (linear colors, degrees, premultiplied
   only, Oklab default). Cost: the types above (small), plus own ramp
   generation (Oklab code exists in `seed.rs`). No exposure to peniko's
   minor-per-kurbo breakage.
2. **Depend on peniko inside renderers only.** Only worth it if a renderer
   consumes peniko types, i.e. a vello backend; pert-ui's own renderers
   would gain nothing (they would still need the ramp, dither and parity
   code). Translation per command: color linear→sRGB f32, stops tagged
   LinearSrgb, degrees→radians, concentric→two-point, image
   re-premultiplication, `Rect`/`Point`/`Transform2D`→kurbo.
3. **Both** (recommended): pert-ui owns `Paint`/`Gradient` modelled on
   peniko's shape (the table above) so a translation is mechanical and
   lossless where both have the concept, and peniko appears only as a
   dependency of an optional vello backend crate if the spike earns one.
   Keep the field names and enum variant names close to peniko's where the
   semantics are equal (`Extend::{Pad, Repeat, Reflect}`, `HueDirection`'s
   four), and differ deliberately where pert-ui's conventions differ
   (linear colors, degrees, default space).

## C. vello family facts for the spike

### C1. Versions (crates.io API, 2026-11-05)

| Crate | Version | Released | MSRV | Licence | Notes |
|---|---|---|---|---|---|
| `vello` | 0.11.0 | 2026-10-02 | 1.89 | Apache-2.0 OR MIT | compute renderer; deps `peniko ^0.6.1`, `skrifa ^0.44.0`, `vello_encoding ^0.11`, optional `wgpu ^30.0.0` (`std`, `wgsl`) ([deps](https://crates.io/api/v1/crates/vello/0.11.0/dependencies)) |
| `vello_cpu` | 0.3.0 | 2026-10-02 | 1.89 | Apache-2.0 OR MIT | deps `vello_common ^0.3`, `hashbrown ^0.17.1`, `bytemuck`; optional `glifo ^0.4` (text), `png`, `rayon ^1.12`, `crossbeam-channel`, `ordered-channel`, `thread_local` (multithreading) ([deps](https://crates.io/api/v1/crates/vello_cpu/0.3.0/dependencies)) |
| `vello_gpu` | 0.3.0 | 2026-10-02 | 1.89 | Apache-2.0 OR MIT | deps `vello_common ^0.3` (`std`), `hashbrown`, `thiserror`, `log`; optional **`wgpu ^30.0.0`** (`std`, `wgsl`, no defaults), `vello_gpu_shaders`, `glifo`, web-sys/js-sys (webgl) ([deps](https://crates.io/api/v1/crates/vello_gpu/0.3.0/dependencies)). 0.1.0 (2026-08-26, MIT) was a name reservation |
| `vello_hybrid` | 0.2.0 (last) | 2026-08-07 | 1.88 | Apache-2.0 OR MIT | superseded by vello_gpu 0.3.0 (version numbers aligned with vello_cpu) |
| `vello_common` | 0.3.0 | 2026-10-02 | 1.89 | Apache-2.0 OR MIT | deps `peniko ^0.6.1` (bytemuck), `fearless_simd ^0.7`, `guillotiere ^0.7` (atlas), `smallvec`, `thiserror`, `log` ([deps](https://crates.io/api/v1/crates/vello_common/0.3.0/dependencies)) |
| `glifo` | 0.4.0 | 2026-10-02 | 1.89 | Apache-2.0 OR MIT | glyph rendering/caching, `skrifa ^0.44.0`, "under rapid development … consider it experimental" ([docs](https://docs.rs/glifo/0.4.0/glifo/)) |
| `anyrender` | 0.14.0 | 2026-10-03 | none stated | MIT OR Apache-2.0 | DioxusLabs; deps `kurbo ^0.13.1`, `peniko ^0.6` |

- **wgpu match:** `vello_gpu ^30.0.0` resolves with pert-ui's
  `wgpu = "30.0.1"` (`Cargo.toml:34`). Caveat: vello_gpu's default
  features include `wgpu_default` (wgpu's default backends); pert-ui uses
  `default-features = false`, so the spike must depend on vello_gpu with
  `default-features = false, features = ["wgpu", …]` or feature
  unification widens pert-ui's wgpu backends.
- **skrifa match:** glifo/vello want `skrifa ^0.44.0`; pert-ui's lock has
  0.44.0 via parley 0.12 — no duplicate font stack.
- **MSRV:** 1.89 ≤ pert-ui's 1.99.1.
- `vello_cpu`/`vello_gpu` sizes are `u16` width/height (≤ 65 535 px).

### C2. vello_cpu 0.3.0 ([crate docs](https://docs.rs/vello_cpu/0.3.0/vello_cpu/), [RenderContext](https://docs.rs/vello_cpu/0.3.0/vello_cpu/struct.RenderContext.html))

- Features: `std` (default; or `libm`), `png` (default; also for PNG
  glyphs), `text` (default; `glyph_run`), `u8_pipeline` (default, fast,
  `RenderMode::OptimizeSpeed`), `f32_pipeline` (accurate,
  `OptimizeQuality`, "especially useful for test snapshots"),
  `multithreading`. SIMD per `Level` (incl. WASM SIMD).
- `RenderSettings { level: Level, num_threads: u16 }` (threads only with
  `multithreading`). Multithreading means a rayon/crossbeam worker pool —
  an implicit background executor that AGENTS.md avoids by default; a
  spike should measure single-threaded first.
- Documented limitations: complex filter graphs panic; in multithreaded
  mode even simple filters panic (`push_layer` with a filter,
  `push_filter_layer`, `set_filter_effect`); `flush` is required before
  rendering in multithreaded mode or it panics; the `Resources` lifecycle
  is "awkward"; **glyph caching experimental, not recommended**.
- API: `set_paint(impl Into<PaintType>)`, `set_paint_transform(Affine)`,
  `set_transform`, `set_fill_rule`, `set_stroke`, `set_blend_mode`,
  `set_tint` (image paints), `fill_rect`, `stroke_rect`, `fill_path`,
  `stroke_path`, `fill_blurred_rounded_rect(rect, radius, std_dev,
  invert)` (box shadow; solid paint only), `glyph_run(resources,
  &FontData)`, `push_layer(clip_path, blend_mode, opacity, mask, filter)`,
  `push_clip_layer`, `push_clip_path`/`push_clip_rect`/`pop_clip`,
  `push_blend_layer`, `push_opacity_layer`, `push_mask_layer(Mask)` (mask
  must match the context size, ignores the transform), `pop_layer`,
  `set_mask`, `set_aliasing_threshold(Option<u8>)`, `render(target:
  PixmapMut, resources)`, `render_with(…, RasterizerSettings)` (offset,
  `TargetInit::Clear`). Images are registered in `Resources` and referred
  to by `ImageSource::OpaqueId`. `Pixmap` = premultiplied RGBA8.
- **Determinism:** not documented. The `f32_pipeline` being recommended
  for snapshots suggests reproducible output per configuration; outputs
  of `u8` vs `f32` pipelines and of different SIMD `Level`s are not
  claimed to be identical — the spike must test same-input/same-machine
  and cross-`Level` stability (unverified).
- **Color space (critical for parity):** the vello maintainers state that
  vello does antialiasing and compositing **in sRGB-encoded space**, with
  linear/conflation-free compositing a longer-term goal
  ([vello#592](https://github.com/linebender/vello/issues/592), raphlinus
  2024-06-24, still open, labelled `vello-research` 2026-10-28). No
  linear-blending option was found in the vello_cpu/vello_gpu 0.3.0 docs.
  pert-ui blends in **linear light** (A1). A vello backend would therefore
  differ from today's renderers on every translucent pixel, every AA edge
  and every mask/text edge by more than the one-step tolerance, unless
  pert-ui changed its documented blend space (a contract change in
  `docs/rendering-platform.md`). That is the spike's first question, before
  frame cost.

### C3. vello_gpu 0.3.0 ([crate docs](https://docs.rs/vello_gpu/0.3.0/vello_gpu/), [Renderer](https://docs.rs/vello_gpu/0.3.0/vello_gpu/struct.Renderer.html))

- Sparse strips: CPU does path processing, tiling and strip scheduling;
  the GPU rasterizes strips and composites. Features `wgpu` (default),
  `wgpu_default` (default), `text` (default), `webgl`.
- Documented: **mask layers, complex filter graphs and certain blend modes
  for non-isolated blending are not supported and panic**; "some other
  failures also panic" rather than return errors; glyph caching
  experimental; "slightly less mature" than vello_cpu; weaker than `vello`
  on dynamic vector-heavy scenes, strong on images, gradients and filters.
- `Renderer::new(&Device, &RenderTargetConfig { format, width: u16,
  height: u16 }) -> (Self, Resources)`;
  `render(&mut self, &Scene, &mut Resources, &Device, &Queue, &mut
  CommandEncoder, &RenderSize, &TextureView, depth_view:
  Option<&TextureView>, &TextureBindings, TargetInit) -> Result<(),
  RenderError>`; the depth view, if given, must be `Depth24Plus`.
  `upload_image(…, &impl AtlasWriter) -> ImageId`, `destroy_image`,
  `atlas_texture`; external textures via `TextureBindings`/`TextureId`
  (`RenderError::MissingTextureBinding`). It records into the caller's
  encoder, so it can share pert-ui's device, queue and encoder.
- The docs do not state whether the target may be an sRGB format or how
  it treats one (unverified; given C2's sRGB-space compositing, writing to
  pert-ui's sRGB surface would double-encode unless the spike uses a
  non-sRGB view — check first).
- Panics on unsupported input mean the translation must refuse those
  features up front (pert-ui returns typed errors, AGENTS.md); a spike
  must not route arbitrary blend modes or masks to it.

### C4. How vello takes glyphs, images, clips, layers, gradients

- **Glyphs:** `glyph_run(&mut Resources, &FontData)` →
  `GlyphRunBuilder` over positioned glyph ids (via `glifo`, `skrifa
  0.44`); parley's `FontData` (linebender_resource_handle) is the same type
  pert-ui's parley 0.12 produces, so laid-out runs translate without
  re-shaping. Hinting/caching live in glifo, experimental. Text through
  vello would replace pert-ui's own glyph masks — a separate parity
  question (text is in the tolerance tier).
- **Images:** registered resources (vello_cpu `Resources`, vello_gpu
  atlas via `guillotiere`), drawn as an image paint with
  `ImageSampler`-style extend/quality; pert-ui's quarter turns become a
  paint transform.
- **Clips/layers:** clip paths and rects (`push_clip_*`), clip layers,
  opacity layers, blend layers, mask layers (GPU: panics), all
  stack-based; pert-ui's display list has only rectangular per-command
  clips.
- **Gradients** (`vello_common 0.3.0` `encode.rs`,
  [source](https://docs.rs/crate/vello_common/0.3.0/source/src/encode.rs)):
  LUT of 256 (2 stops) / 512 (3) / 1024 (≥ 4) entries, raised for sharp
  stops, max `MAX_GRADIENT_LUT_SIZE = 4096`, `u8` or `f32` entries;
  non-sRGB interpolation spaces are approximated by inserting
  intermediate stops via `color::gradient` (tolerance argument 0.01) and
  then interpolating linearly **in sRGB-encoded components**;
  premultiplied by default; offsets outside `0..=1`/NaN, non-ascending
  stops, degenerate geometry and sweeps with `end <= start` **fall back to
  the first stop's color** (empty → black) instead of erroring; equal
  offsets nudged by 1e-7; radial split into concentric / strip / focal
  following Skia; **no dithering**; no stop-count limit.

### C5. anyrender 0.14.0 `PaintScene` as a checklist ([docs](https://docs.rs/anyrender/0.14.0/anyrender/trait.PaintScene.html))

`PaintScene: RenderContext` (not dyn-compatible). Required: `reset`;
`push_layer(fill, blend, alpha, transform, clip: &impl Shape, filter,
backdrop_filter)`; `push_clip_layer(fill, transform, clip)`; `pop_layer`;
`stroke(style, transform, brush, brush_transform, shape)`; `fill(fill,
transform, brush, brush_transform, shape)`; `draw_glyphs(font,
font_size, hint, normalized_coords, embolden, style, brush, brush_alpha,
transform, glyph_transform, glyphs)`; `draw_box_shadow(transform, rect,
color, radius, std_dev)`. Provided: `append_scene(scene, transform)`,
`draw_image(image, transform)`. Other traits: `RenderContext` (resource
registration, `ResourceId`, `RegisterResourceError`), `ImageRenderer`,
`WindowRenderer`; `Filter` (W3C Filter Effects).

Checklist against pert-ui's `DisplayList` today: fill ✓ (rect only, plus
vector masks), stroke ~ (vector geometry, CPU-rasterized), brush ✗ (solid
only → A5), brush transform ✗, glyphs ~ (pre-rasterized masks), image ✓
(quarter turns only), layer with blend/alpha ✗ (G), shaped clip ✗ (G),
box shadow ✗ (D), filters/backdrop filter ✗ (D), scene append/transform ~
(view transforms are applied at recording), resource registration ✓
(masks/images/vectors tables).

## What could not be verified

- CSS Color 4 / Images 4 text was read through a summarizing fetch of the
  editor's drafts (§ numbers as reported), not quoted from a fixed TR
  snapshot; SVG `color-interpolation` default and tiny-skia `SpreadMode`
  and Skia `SkTileMode` variant names are from memory.
- Chromium/Skia dithering gradients by default, the IGN constants, and
  the blue-noise source are from memory.
- vello_cpu determinism (same input, same/different `Level`, u8 vs f32
  pipeline) is undocumented.
- Whether vello_cpu/vello_gpu 0.3.0 still composite in sRGB-encoded
  space is inferred from vello#592 and the absence of any linear option or
  linearization in `vello_common`'s gradient encoder; not confirmed in
  their own docs or source. Likewise whether vello_cpu `Pixmap`
  premultiplies in encoded space, and how vello_gpu treats an sRGB target
  format.
- `vello_cpu` `RenderSettings` defaults (level, threads), the exact glyph
  types of its `GlyphRunBuilder` (the docs.rs page 404'd), and whether a
  `PaintType` gradient honours `interpolation_cs` identically on GPU and
  CPU.
- anyrender's `Paint`/`PaintRef` variants and `RenderContext` methods.
- peniko 1.0 plans: none found; absence of evidence only (no Zulip
  search).
- CPU gradient cost is an estimate; no build or measurement was run
  (timing measurements were running on the machine).
