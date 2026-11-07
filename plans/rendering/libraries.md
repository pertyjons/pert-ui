# Phase 0 item 3: techniques in other libraries (and item 5: wgpu portability)

Compiled 2026-11-05. **Sourcing:** each claim gives a source and a version or date.
Claims marked **(memory)** come from prior knowledge and were not checked against a
source in this session. Raw source files were read through `raw.githubusercontent.com`,
`skia.googlesource.com` and the GitHub API at the commit current on 2026-11-05 ("main
@2026-11-05"). wgpu facts come from the local `wgpu-types-30.0.1` / `wgpu-30.0.1`
sources in `~/.cargo/registry/src` (the same docs as docs.rs/wgpu/30.0.1). Current
versions come from the crates.io API (2026-11-05): egui/epaint 0.37.0 (2026-10-15),
iced 0.14.0 (2025-12-07; master is 0.15.0-dev), femtovg 0.28.0 (2026-10-27), vello 0.11.0,
vello_cpu 0.3.0 and vello_gpu 0.3.0 (all 2026-10-02), skia-safe 0.157.0 (2026-11-03),
Slint v1.18.1 (2026-09-21), wgpu 30.0.1 (2026-08-22).

## 1. Per library

### egui / epaint (0.35.0 read locally; 0.37.0 current)
- **Primitives:** `Shape::{Circle, Ellipse, Rect, Path, Text, Mesh, QuadraticBezier,
  CubicBezier, Vec}` (`epaint-0.35.0/src/shapes/shape.rs`). 0.37.0 adds a `RoundedRect`
  primitive (#8440, epaint CHANGELOG). The CPU tessellates everything into one
  indexed-triangle `Mesh`. The backend only draws textured, vertex-colored triangles.
- **AA:** "feathering". "Feathering works by making each edge into a thin gradient into
  transparency", with a default width of 1 physical px. "This makes shapes appear smoother,
  but requires more triangles" (`tessellator.rs` `TessellationOptions`). Rects, line
  segments and text snap to the pixel grid by default (`round_rects_to_pixels` and
  related options). Small filled circles can use pre-rasterized discs from the font atlas
  (`prerasterized_discs`).
- **Paint:** per-vertex color plus an optional texture (`Brush { fill_texture_id, uv }`).
  The only gradient is `Shape::gradient_rect`, a 4-vertex mesh with corner colors
  (`shape.rs:300`). There are no stops, radial or conic gradients.
- **Shadow:** `Shadow { offset, blur: u8, spread: u8, color }`. The blur is a wider
  feather ("We accomplish the blur by using a larger-than-normal feathering",
  `tessellator.rs:1864`), so the falloff is a linear ramp, not a Gaussian. The blur width
  is limited to the rect's smallest side.
- **Clip:** clip rectangles only (scissor per mesh).
- **Transforms:** translate and scale of shapes on the CPU (`shape_transform.rs`). There is
  no GPU transform.
- **Text:** glyphs are rasterized into one font/texture atlas, and 0.37 moves them into a
  dedicated `GlyphAtlas` (#8501). `FontColorTransferFunction` (formerly
  `AlphaFromCoverage`) adjusts the coverage curve.
- **Color space:** egui-wgpu blends in gamma space. Vertex colors are "gamma 0-1", and the
  shader converts and dithers the gamma-encoded values (`egui-wgpu-0.35.0/src/egui.wgsl`
  lines 5, 134-141).
- **CPU fallback:** egui ships no CPU rasterizer. Because the output is a plain triangle
  mesh, any triangle rasterizer can draw it, and parity reduces to the rasterization rules.
  Third-party software backends exist **(memory)**.
- **Trade-off:** one universal pipeline that is trivial to port. AA and shapes cost CPU
  tessellation every frame, which immediate mode redoes anyway. Feathered edges are not
  exact coverage, and linear-ramp "shadows" look different from CSS shadows.

### iced (0.14.0; shaders read at master @2026-11-05)
- **Primitives:** instanced quads (`quad/solid.wgsl`, `quad/gradient.wgsl`), triangle
  meshes (`triangle.wgsl`; `canvas` paths are tessellated with lyon, the `geometry`
  feature, `wgpu/Cargo.toml`), images and SVG (resvg rasterized into a guillotiere atlas),
  and text through cryoglyph (iced's glyphon fork) on cosmic-text (root `Cargo.toml`).
- **Quad AA:** a per-instance SDF. The instance carries `color, border_color,
  shadow_color, pos, scale, border_radius: vec4, border_width, shadow_offset,
  shadow_blur_radius, snap`. Coverage is `clamp(0.5 - dist, 0, 1)` on a quad expanded by
  1 px, with a single SDF since #2967 (0.14 CHANGELOG). Radii are clamped to half the
  smaller side.
- **Mesh AA:** MSAA with a sample count chosen by `Antialiasing`
  (`wgpu/src/triangle/msaa.rs`).
- **Shadow:** an SDF with a smoothstep falloff, not a Gaussian:
  `1 - smoothstep(-blur, blur, max(dist, 0))`, mixed only where the quad itself is
  transparent (`quad/shadow.wgsl`).
- **Paint:** solid or linear gradient. Gradients have at most 8 stops, interpolate with a
  **smoothstep** between stops (not linearly), and add ±0.3/255 noise as dithering. They
  `mix` in linear RGB (`quad/gradient.wgsl`, `color/linear_rgb.wgsl`).
- **Clip/transforms:** rectangular layer clips and translate/scale transformations
  **(memory)**. The quad shader has no rotation input.
- **CPU fallback:** `iced_tiny_skia` (0.14.1), composed with wgpu by
  `iced_renderer::fallback`. It does **not** reuse the GPU formula for the quad body: it
  builds a kurbo/tiny-skia rounded-rect path and fills it with tiny-skia's rasterizer. It
  ports the shadow SDF + smoothstep pixel by pixel into a temporary pixmap and maps
  gradients to `tiny_skia::LinearGradient` (`tiny_skia/src/engine.rs`). Parity is
  therefore approximate and nobody tests for it **(memory: no parity tests seen)**.
- **Trade-off:** cheap, batched rounded quads. The shadow is a "soft SDF" rather than a
  blur and diverges visibly from CSS for large radii.

### Slint (v1.18.1; renderer sources at master @2026-11-05)
- **Renderers** (docs.slint.dev "Backends & Renderers", 2026 page):
  - FemtoVG: OpenGL, plus `renderer-femtovg-wgpu`. Limitation: "Text and path rendering
    quality sometimes sub-optimal."
  - Skia: GL, Metal, Vulkan, D3D. "Heavy disk-footprint."
  - Software: `no_std`, line-by-line rendering.
  - Vello: experimental, "Requires a GPU that supports compute shaders… no software
    fallback."
  - Qt (QPainter).
- **Rounded rects:** FemtoVG fills paths (`rounded_rect` / `rounded_rect_varying`). Skia
  draws rrects natively. The software renderer computes analytic coverage per scanline
  (`draw_rounded_rectangle_line` with `shape_coverage`, `draw_functions.rs`).
- **Shadows:**
  - FemtoVG renders the shape into an offscreen texture, blurs it with femtovg's Gaussian
    filter ("kernel reaches ceil(3·sigma), and sigma is blur / 2") and caches the result in
    `box_shadow_cache` (`femtovg/itemrenderer.rs:538`).
  - The software renderer on master: "Box shadows, blurred analytically line by line,
    without a texture". It uses a weighted sum of up to 3 rounded rects convolved with a
    Gaussian, computed with Q15/Q16 fixed-point CDF tables (`software/box_shadow.rs`, last
    changed 2026-11-05). The docs page still lists "No support for drop-shadow-*" for the
    software renderer, so this support is newer than the docs.
- **Rounded clips:** FemtoVG renders the children into a layer image and fills the rounded
  path with that image as the paint (`visit_clip`, itemrenderer.rs:741). The software
  renderer has "no support for border-radius in combination with clip: true".
- **Paint:** the software renderer has linear, radial and conic gradients
  (`draw_linear_gradient`, `draw_radial_gradient`, `draw_conic_gradient`). FemtoVG and
  Skia map gradients to their own paints.
- **Transforms:** the GPU renderers support them. The software renderer: "No support for
  rotations or scaling", except whole-panel rotation.
- **Text:** the software renderer has its own glyph rasterization ("currently limited to
  western scripts"). FemtoVG uses its glyph atlas.
- **Parity:** the renderers are independent implementations. No cross-renderer pixel
  parity is claimed. Slint tests with per-renderer screenshot references **(memory)**.
- **Lesson:** Slint keeps a fixed-point software renderer with an analytic Gaussian shadow
  that needs no texture. This is evidence that a CPU path can compute the same
  closed-form shadow as a shader.

### vello family (Linebender)
- **vello 0.11.0** ("classic"; the README now lists it under `research/` as "the original
  compute-centric renderer"):
  - It encodes a PostScript-like scene (fills, strokes, layers, glyph runs, peniko brushes)
    and renders it entirely in **compute shaders**, using prefix sums, binning, coarse and
    fine rasterization. "Vello needs a GPU with support for compute shaders"
    (docs.rs/vello/0.11.0).
  - AA modes (`AaConfig`): `Area`, `Msaa8`, `Msaa16`. Area AA integrates winding over the
    pixel square. It is "very accurate… when the shape has winding number of 0 or 1
    everywhere, but can result in conflation artifacts otherwise."
  - Known issues **(memory)**: fixed-size intermediate buffers sized ahead of time, which
    have caused dropped geometry or readback stalls in past versions, and no CPU fallback
    that matches its pixels.
- **Sparse strips** (`vello_common`, `vello_cpu` 0.3.0, `vello_gpu` 0.3.0, formerly
  `vello_hybrid`):
  - Paths are flattened and tiled into 4×4 tiles (`Tile::WIDTH/HEIGHT = 4`, local
    `vello_common-0.0.9/src/tile.rs`). Edge tiles become "strips" with per-pixel alpha
    masks. Interior spans become solid "sparse fills" reconstructed from row windings
    (`strip.rs`), giving analytic area coverage.
  - `vello_cpu` does all stages on the CPU with SIMD and optional multithreading, a
    `u8_pipeline` default and an `f32_pipeline` "more accurate" option.
  - `vello_gpu` does the CPU stages up to strips, then rasterizes and composites on the
    GPU. Its wgpu backend "doesn't require compute shaders", and it has a WebGL2 backend.
    It depends on wgpu `^30.0.0` (README; docs.rs/vello_gpu/0.3.0).
  - Limitations, verbatim:
    - "Mask layers, complex filter graphs as well as certain blend modes" panic (vello_gpu).
    - "glyph caching" is experimental (both).
    - vello_gpu "does not match the compute-centric `vello` renderer's raw performance on
      dynamic and vector-heavy workloads."
  - `vello_common` has a `blurred_rounded_rect` primitive with a std-dev and corner radius
    (local 0.0.9).
- **Paint:** peniko `Brush` (solid, linear/radial/sweep gradient, image) and blend modes
  (`Mix`, `Compose`).
- **Parity:** vello_cpu and vello_gpu share strip generation in `vello_common`. Coverage
  therefore comes from the same code and only fine rasterization and compositing differ.
  This is an inference from the architecture, not a stated guarantee. Classic vello and
  vello_cpu are different pipelines.
- **MSRV:** 1.89 for vello, vello_cpu and vello_gpu (crates.io).

### Skia: Ganesh and Graphite (main @2026-11-05, commit 39b335b)
- **Graphite** (Chrome blog, 2025-07-08):
  - Each draw gets a depth value from its painter's order, so "opaque objects… eliminate
    overdraw". Clips are depth-only draws, so the shader "does not also depend on the
    state of the clip stack". This gives shaped clips without stencil bits.
  - Graphite "relies on MSAA where it can". Where it cannot, it uses CPU path
    rasterization into an atlas. It "consolidate[s] the number of rendering pipelines" and
    precompiles them at startup.
  - Recording is multithreaded.
  - The blog reports +15% on MotionMark 1.3 on an M3 MacBook Pro.
- **AnalyticRRectRenderStep** (`src/gpu/graphite/render/AnalyticRRectRenderStep.cpp`):
  - One instanced step draws filled rects, per-edge-AA quads, rrects with elliptical
    per-corner radii, strokes with any join, lines with caps and hairlines. The point is
    "to maximize batching… reduce the number pipeline specializations… and help us avoid
    triggering MSAA."
  - Each primitive is a single instance encoded in three `float4`s, with flags packed into
    signs.
  - AA uses a first-order distance estimate C(p)/|∇C(p)|, so it stays correct under any
    affine transform. Straight edges use interpolated linear level sets. Corners evaluate
    the ellipse distance in the fragment shader.
- **Other Graphite steps:** `AnalyticRRectBlurRenderStep` builds a 5×5 cell grid outset
  by 3 sigma. Also present: `AnalyticBlurRenderStep`, `CircularArcRenderStep`,
  `PerEdgeAAQuadRenderStep`, tessellation steps (`TessellateWedges/Curves/Strokes`, stencil
  plus `CoverBounds`), `CoverageMaskRenderStep` (atlas), `BitmapTextRenderStep`,
  `SDFTextRenderStep`, `SDFTextLCDRenderStep` and a `WideTileRenderStep` (purpose not
  checked).
- **Path strategy** (`RendererProvider.cpp`): "prefer vello > tessellation [w/ atlas] >
  raster atlas". Vello compute is used if built in and compute is available. Tessellation
  "requires MSAA" (≥4×). The raster path atlas is always supported.
- **Text:** A8/LCD glyph bitmaps in an atlas for ordinary sizes. Distance-field glyphs
  serve large or transformed text (Bitmap and SDF text steps exist, as listed above; the
  size thresholds come from **memory**).
- **Ganesh (memory):** analytic ops `GrFillRRectOp` and the oval/rrect ops (coverage
  computed in the shader, instanced). The rrect blur uses a precomputed blur-profile
  texture applied nine-patch style. Paths go through the tessellation/stencil-and-cover
  renderer, an atlas path renderer, triangulation, or a software path renderer as the last
  resort.
- **CPU (memory):** the raster backend (SkRasterPipeline, analytic-AA scan converter) is
  not pixel-identical to the GPU. Skia tests each config against its own baselines
  (Gold).
- **Trade-off:** maximum capability. The cost is a large code base, many pipelines and
  MSAA dependence for general paths.

### femtovg 0.28.0 / NanoVG
- **NanoVG** ("not actively maintained", README):
  - Convex fills are drawn directly with "fringe" AA geometry (`NVG_ANTIALIAS`: "the
    renderer adjusts the geometry to include anti-aliasing"). Concave fills use
    stencil-then-cover: an INCR/DECR_WRAP stencil pass (`nanovg_gl.h:1026`), the AA fringe
    and then a cover quad.
  - "Even-odd filling rule" by default, and holes via `nvgPathWinding`.
- **Paint (NanoVG):** one uniform-driven shader with a 3×3 paint transform, `extent`,
  `radius`, `feather`, inner and outer color. A linear gradient, a radial gradient and a
  **box gradient** all reduce to `sdroundrect()` + feather (`nanovg_gl.h:609,640`). The
  box gradient is "a feathered rounded rectangle, it is useful for rendering drop shadows"
  (`nanovg.h:403`). The shadow is therefore a linear ramp around an SDF, not a Gaussian.
- **Text (NanoVG):** fontstash/stb_truetype glyph atlas.
- **femtovg:** a NanoVG port with OpenGL and wgpu backends (`src/renderer/{opengl,wgpu}`),
  plus a no-op `void` renderer and no CPU renderer.
  - Gradients: "box, linear, radial (including two-point / focal), and conic, with
    multi-stop colors and gradient transforms".
  - Image filters: gaussian blur, color matrix, turbulence. Also composition modes, layers
    and luminance/alpha masks.
  - "Custom shaders and 3D transforms" are unsupported (README).
- **Trade-off:** small and portable, with general paths and paints. It needs a stencil
  buffer, fringe AA misbehaves on thin or self-overlapping geometry, and Slint calls the
  quality "sometimes sub-optimal".

### Flutter Impeller (Flutter 3.47, docs updated 2026-08-21; engine sources at master)
- **Goals:** offline-compiled shaders and all pipeline state objects built up front, to
  avoid shader-compile jank (docs.flutter.dev/perf/impeller). Impeller is the default on
  iOS, Android API 29+ and, as of 3.47, macOS/Linux/Windows. Web uses Skia.
- **AA:** "On desktop and mobile 4x MSAA is used for all rendering calls."
  - SDF rendering of standard primitives is on by default on desktop and off by default on
    mobile, "because SDF rendering puts more demand on the GPU… physical pixel sizes on
    desktop… any imperfection will be more evident" (docs.flutter.dev/perf/antialiasing).
  - The SDF shader is `uber_sdf.frag` / `uber_sdf_ssbo.frag`: rects, rrects, rounded
    superellipses, strokes, with "the size of a device pixel in local coordinates" passed
    in for AA width.
  - Gradients render "using a pre-baked gradient ramp texture" or "a storage buffer of
    gradient stops" (`uber_sdf_contents.h`).
- **Paths:** tessellated. `GeometryResult::Mode::{kNormal, kNonZero, kEvenOdd,
  kPreventOverdraw}` means "geometry may have overlapping triangles… should be stenciled
  with the NonZero fill rule", i.e. stencil-then-cover, AA'd by MSAA (`geometry.h`).
  Flutter issue #123671 discusses triangulation against stencil-then-cover and the stencil
  bits that clips consume.
- **Shadows/blur:** `rrect_blur.frag` is an analytic fast path "based on… Raph Levien's
  blog post 'Blurred rounded rectangles'". Other shapes fall back to a two-pass Gaussian
  (flutter/engine PR #51250, 2024-03).
- **Text:** `GlyphAtlas`, "a texture containing the bitmap representation of glyphs"
  (`typographer/glyph_atlas.h`), A8 or color.
- **CPU fallback:** none in Impeller. The legacy fallback is Skia/GL.

### Zed GPUI (crates/gpui_wgpu @main 2026-11-05; Linux moved from blade to wgpu in PR #46758, merged 2026-02-13; macOS keeps Metal)
- **Design** ("Leveraging Rust and the GPU to render user interfaces at 120 FPS", zed.dev,
  2023-03-07): a dedicated shader per primitive instead of a general vector renderer.
  Pathfinder "wasn't fast enough".
- **Primitives:** quads, shadows, underlines, monochrome sprites (glyphs/icons),
  polychrome sprites (images, emoji), subpixel sprites, paths and surfaces.
- **Instancing:** all instanced, with **vertex pulling from storage buffers**: `var<storage,
  read> b_quads: array<Quad>` with `load_quad(instance_id)` (`shaders_storage.wgsl`).
  `shaders_webgl.wgsl` decodes the same records from a texture where storage buffers are
  missing.
- **Quad AA:** SDF with a fixed `saturate(0.5 - sdf)`, `antialias_threshold = 0.5`, and no
  `fwidth`. It assumes pixel-aligned, untransformed quads.
- **Shadows:** Evan Wallace's technique. `erf` is closed-form along one axis, the Gaussian
  is integrated along the other with 4 samples, and the margin is `3 * blur_radius`. Inset
  shadows are supported.
- **Clipping:** rectangle `content_mask` only, applied by discard in the fragment stage.
  Shaped clips do not exist.
- **Gradients:** linear only, two colors, interpolated in sRGB or **Oklab**. There are also
  slash-stripe and checkerboard patterns.
- **Paths:** triangles rasterized into an intermediate texture with implicit-curve
  coverage (`s*s - t` normalized by `dpdx/dpdy`, Loop–Blinn style) **and MSAA** (sample
  count chosen from `[4, 2, 1]` by format support, `wgpu_renderer.rs:2610`), then
  composited as sprites.
- **Text:** OS or cosmic-text rasterization into an etagere atlas: R8Unorm for monochrome,
  at least 1024² and up to `max_texture_dimension_2d` (`wgpu_atlas.rs`). Glyphs have up to
  16 subpixel variants (blog). There is grayscale contrast/gamma correction in the shader.
  Subpixel (LCD) text uses **dual-source blending** when the device has it
  (`shaders_subpixel.wgsl`, `enable dual_source_blending`).
- **Presentation:** `Fifo` default, `desired_maximum_frame_latency: 2`
  (`wgpu_renderer.rs:472-473`).
- **CPU fallback:** none **(memory)**. It relies on a GPU driver, possibly a software one
  such as llvmpipe.
- **Reported results:** the move to wgpu cost ~20% less CPU time, with GPU time roughly the
  same (secondary summary, news.hada.io; not verified in the PR). An empty-window memory
  regression was reported on Windows.

### Makepad (brief; main @2026-11-05)
- Every widget is an instanced quad whose shader composes shapes with an `Sdf2d` helper
  (`box`, `circle`, `union`, `intersect`, `subtract`, `fill_keep`).
- `libs/math/src/shader_runtime.rs` contains a **CPU mirror** of `Sdf2d` "for CPU-side
  shape rendering" and shader validation. Its `fill_keep` computes
  `alpha = clamp(-d/aa + 0.5, 0, 1)` with `aa = 1.5` by default, then blends premultiplied
  over the existing result.
- This is the closest existing analogue to "the same SDF formula in shader and CPU".

## 2. Comparison

| | Primitive set | Shape AA | Paint | Shadow | Clip | Text | CPU path / parity |
|---|---|---|---|---|---|---|---|
| egui 0.37 | tessellated mesh only | 1 px feather geometry | vertex color, 4-color rect | wide feather (linear) | rect scissor | atlas, bitmap | none built in; mesh is portable |
| iced 0.14 | SDF quad, mesh, image, text | SDF `0.5-d`; MSAA for meshes | linear grad ≤8 stops, dither | SDF + smoothstep | rect | cryoglyph atlas | tiny-skia fallback, different formulas |
| Slint 1.18 | per-renderer items | femtovg fringe / Skia / SW analytic per line | lin/rad/conic (SW) | femtovg: offscreen Gaussian, cached; SW: analytic Gaussian | femtovg: layer + rrect path; SW: rect only | per renderer | SW is separate, no parity claim |
| vello 0.11 | paths, layers, glyphs | area (conflation) / MSAA8/16 | peniko brushes, blend modes | blurred rrect primitive | arbitrary clip layers | glyph runs | none (compute only) |
| vello_cpu/gpu 0.3 | same model via strips | analytic area in 4×4 tiles | peniko | blurred rrect | clip layers (mask layers panic in gpu) | glyph runs, cache experimental | shared strip stage; parity plausible, not claimed |
| Skia Graphite | analytic rrect/arc/quad instances, tess paths, atlas | analytic C/∣∇C∣; MSAA for paths | full shader graph | analytic rrect blur | depth-buffer clips (any shape) | bitmap atlas + SDF | raster backend separate |
| femtovg/NanoVG | paths | fringe + stencil-then-cover | lin/rad/box/conic, image | box gradient (feathered SDF); femtovg blur filter | scissor (+layers in femtovg) | glyph atlas | none |
| Impeller | tessellated paths, UberSDF shapes | 4× MSAA everywhere; SDF on desktop | ramp texture or SSBO stops | analytic rrect blur, else 2-pass Gaussian | stencil | glyph atlas | none (Skia legacy) |
| GPUI | instanced quad/shadow/sprite/path | SDF `0.5-d`; paths MSAA + implicit curves | linear, sRGB/Oklab, patterns | erf × 4-sample Gaussian | rect only | R8 atlas, subpixel via dual-source | none |
| Makepad | SDF shader per widget | SDF with `aa` width | in-shader | in-shader | rect | atlas **(memory)** | CPU mirror of Sdf2d |

## 3. Lessons for pert-ui (retained backend-independent DisplayList + CPU renderer with parity)

**What transfers well**
1. **Analytic rounded rect as one instanced primitive (B + C).** GPUI, iced, Graphite and
   Impeller's UberSDF all do this. The instance holds rect, per-corner radii, border width
   and color, fill paint index, and clip. The fragment formula is ~10 lines and is a pure
   function of the pixel center. The CPU renderer can evaluate *the same* f32 formula at
   each pixel center, as Makepad's CPU `Sdf2d` mirror does. That gives the best parity
   story of any option: differences reduce to f32 rounding and the GPU's sub-pixel
   rasterization snapping (already an exception in `docs/rendering-platform.md`).
   - Graphite's one-step-for-many-shapes encoding (rect, rrect, stroke, hairline,
     per-edge-AA quad) is the model for keeping pipeline count low.
   - This replaces the `shapes.rs` mask cache and uploads for ordinary shapes.
2. **erf box shadow (D).** It is closed-form and needs no render pass.
   - GPUI and Evan Wallace: exact erf along one axis, 4 Gaussian samples along the other.
   - Impeller and Raph Levien: an approximate analytic rrect blur.
   - Slint SW: a fixed-point line-by-line Gaussian on the CPU.
   - All are per-pixel functions, so CPU parity holds as for 1. Fix the erf approximation
     (GPUI's polynomial) and the sample count in the contract, so both renderers use one
     definition.
   - Avoid iced's smoothstep "shadow" and egui/NanoVG's linear-ramp feather. They are
     cheap but are not Gaussian, and theme authors expect CSS-like elevation.
3. **Glyph atlas (E).** Every GPU library uses one: A8 for coverage, RGBA for color glyphs,
   etagere/guillotiere packing. The CPU renderer reads the same swash bitmaps directly, so
   coverage parity is exact and only blending differs. SDF text (Skia) is only for
   large or transformed text and is optional.
4. **Gradients as a paint index (A).** iced (stops in the instance, ≤8), Impeller (ramp
   texture or an SSBO of stops), GPUI (two colors in sRGB/Oklab). A per-pixel gradient
   evaluation over a stop table is easy to mirror on the CPU. Add dithering as iced does:
   ±0.3/255 noise, but with a *deterministic* hash of pixel coordinates so CPU and GPU
   produce the same pattern.
5. **Vertex pulling from a storage buffer (C, F).** GPUI's `b_quads[instance_id]` uses one
   buffer per primitive kind and draws `draw(0..6, 0..n)`. This fits `retained.rs`'s
   changed-range uploads: an instance is a fixed-size record, so a change rewrites one
   record. The same mechanism carries polyline points for waveforms (F).
6. **MSAA only for the general-path tier.** GPUI uses MSAA for paths only, and
   analytic shapes skip it (Graphite's stated goal is to "avoid triggering MSAA").
   Impeller differs: it runs 4× MSAA for all rendering calls and adds SDF shapes on
   desktop.
   pert-ui can keep CPU-rasterized masks for `DrawVector` and move only common shapes to
   analytic shaders. That is a smaller step than GPU paths.

**What does not transfer**
- **Compute-only pipelines** (classic vello, Graphite's vello path atlas). Their AA and
  composition have no CPU twin, so CPU parity would mean writing a second renderer of
  equal scope. vello_cpu is a different pipeline from classic vello.
  - The sparse-strip pair (vello_cpu + vello_gpu) is the only option in this family with a
    structural parity story, through shared strip generation. Its documented panics (mask
    layers, some blend modes, complex filters) and experimental glyph cache are
    constraints the translation layer would have to exclude (item 6 of the plan).
- **Stencil-then-cover / fringe AA** (NanoVG, femtovg, Impeller paths). They need a
  stencil attachment and MSAA or fringe geometry. A CPU equivalent is a separate
  scan-converter whose AA differs from MSAA by design, so parity would need a loose
  tolerance.
- **egui's model.** CPU tessellation per frame is what pert-ui already pays for with masks.
  It moves cost around without removing it.
- **Graphite's depth-buffer clipping.** It is powerful for arbitrary clips but needs a
  depth attachment and a z order. A CPU renderer would emulate it with masks. Rounded
  clips are simpler as an extra rrect SDF evaluated per pixel (multiply coverage), which
  both renderers can do exactly. Group opacity and arbitrary clips need offscreen layers
  (Slint femtovg's approach), which is a later step (G/H).

**Known pitfalls**
- **Fixed 0.5 px AA threshold.** GPUI and iced use `saturate(0.5 - d)`, which assumes one
  local unit equals one device pixel. Under scale or rotation (G, the diagram-editor zoom)
  edges become too sharp or too soft.
  - Graphite divides by |∇C|, and Impeller passes the device pixel size in local units.
  - For parity, pass the pixel size or Jacobian as data, not `fwidth`. The CPU knows the
    affine matrix exactly, while `fwidth`/`dpdx` are computed per 2×2 quad on hardware
    (GPUI uses them for paths only).
- **Small sizes.**
  - SDF coverage is a 1-D linear ramp along the distance, not area coverage. It
    overestimates at corners with radius below ~1 px and for thin features below 1 px
    (hairlines, 1 px meter ticks). Clamp radius, use Graphite-style hairline handling, or
    keep exact-area rects for axis-aligned fills (the exact parity tier).
  - egui 0.36.2 fixed "thin, angled rectangles", which shows the class of bug.
- **Gamma of AA edges.** pert-ui picks an sRGB surface format (`pert-ui-wgpu/src/lib.rs`
  `find(|f| f.is_srgb())`), so the GPU blends in linear light. egui blends in gamma space,
  and GPUI applies contrast/gamma correction to glyph coverage.
  - Linear blending of coverage makes dark-on-light text and lines look thinner. The
    correction must be a defined function applied identically by the CPU renderer, or the
    tolerance tier widens.
  - Gradient interpolation space (iced: linear RGB; GPUI: sRGB or Oklab; peniko:
    configurable) is a separate choice from blending space. Decide it in the paint model.
- **Conflation.** Two coverage-AA shapes sharing an edge, such as adjacent meter segments,
  table cells or a fill under its border, each get 50% coverage at the seam and leak the
  background: 1−(0.5)² = 75% coverage instead of 100%. vello's area mode documents the
  same effect within one path.
  - Mitigations: draw border and fill in one instance (iced and Graphite do), snap
    axis-aligned edges (egui `round_rects_to_pixels`), or merge segments into one
    primitive.
  - MSAA avoids it for paths but costs memory.
- **Banding.** Dark, slow gradients in 8-bit show bands. Dither, deterministically for
  parity. Consider an `Rgba16Float` intermediate only if blur/backdrop work arrives.
- **Shadow cost.** An erf shadow quad covers the shape plus 3σ on each side, and that
  overdraw dominates for big blurs (GPUI and Graphite use a 3σ margin). Graphite's 5×5
  cell grid avoids shading the interior.

## 4. wgpu 30 portability constraints (candidates B–I)

pert-ui requests `Backends::{VULKAN, DX12, METAL}`, `required_features: empty`,
`Limits::default().using_resolution(adapter.limits())`, `sample_count: 1`, `Fifo` and
`desired_maximum_frame_latency = 2` (`crates/pert-ui-wgpu/src/lib.rs:270-278, 437-438,
703, 1211-1212`). GL and the browser are out of scope, which removes all `downlevel_*`
concerns below.

- **Instancing (C):** `draw(vertices, instances)` with `@builtin(instance_index)` is core
  WebGPU and works on all backends, GLES included. Vertex attributes per instance: core,
  with `max_vertex_buffers 8` and `max_vertex_attributes 16` in `Limits::defaults()`.
  `first_instance` in *direct* draws is core. In *indirect* draws it needs
  `INDIRECT_FIRST_INSTANCE`.
- **Vertex pulling (C, F):** storage buffers in the vertex stage are a downlevel flag
  (`DownlevelFlags::VERTEX_STORAGE`) that Vulkan, DX12 and Metal all have. Only GLES/WebGL
  lack it, which is why GPUI keeps a texture-decoding variant for WebGL.
  - `Limits::defaults()`: `max_storage_buffers_per_shader_stage 8`,
    `max_storage_buffer_binding_size 128 MiB`, `max_buffer_size 256 MiB`,
    `max_uniform_buffer_binding_size 64 KiB`, `max_bind_groups 4`. `downlevel_defaults`
    drop to 4 storage buffers, and WebGL2 to 0.
  - Instance records (~64–128 B) and waveform points fit easily.
  - `IMMEDIATES` (the renamed push constants: Vulkan, DX12, Metal) is optional and needs a
    feature plus a nonzero `max_immediate_size`, which defaults to 0. Do not depend on it.
- **Compute (F, vello):** `DownlevelFlags::COMPUTE_SHADERS` is present on all three
  targeted backends ("WebGL2, and GLES3.0 devices do not support compute").
  - Default workgroup limits: 256×256×64, 16 KiB shared memory, 65535 groups per
    dimension.
  - Writing a swapchain texture from compute needs storage usage on the surface format.
    `Rgba8Unorm` has storage read/write per WebGPU, but the usual `Bgra8Unorm*` surface
    formats need extra features. This is why compute renderers draw to an intermediate
    `Rgba8Unorm` and blit **(memory for the BGRA feature name)**.
- **Dual-source blending (E, subpixel text):** `Features::DUAL_SOURCE_BLENDING` is
  supported on Vulkan (`dualSrcBlend`), DX12 and Metal (MSL 1.2+). It is optional and must
  be requested, and pert-ui currently requests no features. GPUI keeps a fallback path,
  and it found a crash report from setting the flag without the device feature
  (`wgpu_renderer.rs:704`).
- **MSAA:** `guaranteed_format_features` gives `MULTISAMPLE_X4 | MULTISAMPLE_RESOLVE` for
  `R8Unorm`, `Rgba8UnormSrgb`, `Bgra8UnormSrgb` and `Rgba16Float`, so only 1 and 4 are
  guaranteed. 2, 8 and 16 need `TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES` and a per-format
  `sample_count_supported` query (`texture/format.rs:914-991, 1968`), which is how GPUI
  picks from `[4,2,1]`.
  - An MSAA target costs 4× the color memory plus a resolve. A CPU renderer cannot match
    MSAA's sample pattern, so analytic AA is the parity-friendly default.
- **Atlas and textures (E, H):**
  - `Limits::defaults()` max_texture_dimension_2d is **8192**, and `using_resolution`
    raises it to the adapter's value (typically 16384 on desktop GPUs **(memory)**).
  - `max_texture_array_layers` is 256, and `max_sampled_textures_per_shader_stage` is 16.
  - `TEXTURE_BINDING_ARRAY` (bindless arrays) is a native optional feature and
    `max_binding_array_elements_per_shader_stage` defaults to 0, so a portable atlas means
    one or a few 2D textures (GPUI starts at 1024² and grows to the limit) or a
    `texture_2d_array` within 256 layers.
- **Timestamp queries (baseline, item 4):**
  - `Features::TIMESTAMP_QUERY` (pass-level `timestamp_writes`) is listed for Vulkan,
    DX12, Metal, GL with `GL_ARB_timer_query`, and WebGPU.
  - `TIMESTAMP_QUERY_INSIDE_ENCODERS` and `TIMESTAMP_QUERY_INSIDE_PASSES` cover Vulkan,
    DX12 and Metal "AMD & Intel, not Apple GPUs". Inside-pass queries are "generally not
    available on tile-based rasterization GPUs".
  - Results go through `resolve_query_set` and are multiplied by
    `Queue::get_timestamp_period`.
  - Consequence: per-pass GPU time is portable, while per-draw timing is not available on
    Apple Silicon. The feature is optional, so request it only when available, in a
    diagnostics build.
- **Present modes and latency (I):**
  - `Fifo` is supported everywhere. Its queue is "approximately 3 frames long", and
    `get_current_texture()` "will block until there is a spot in the queue". This matches
    the ~8 ms acquire wait already measured.
  - `FifoRelaxed`: "AMD on Vulkan".
  - `Immediate`: "Most platforms except older DX12 and Wayland", with tearing.
  - `Mailbox`: "DX12 on Windows 10, NVidia on Vulkan and Wayland on Vulkan".
  - `AutoVsync` and `AutoNoVsync` are always-valid fallbacks.
  - `desired_maximum_frame_latency` maps per backend as follows:
    - Vulkan: swapchain images = latency + 1, clamped.
    - DX12: `SetMaximumFrameLatency(latency)`.
    - Metal: `maximumDrawableCount` = latency + 1, clamped.
  - The docs advise "Choose `1` to minimize latency… For applications like GUIs doing a
    small amount of GPU work each frame that need low latency, this is a reasonable choice".
    The cost is that CPU and GPU cannot overlap.
  - Mailbox interaction: on DX12 it caps the frame rate at latency × refresh rate. On
    Vulkan/Metal, latency 2 caps it at 2× refresh and 3 or more is unlimited
    (`surface.rs:935-968`).
  - Present feedback: wgpu offers `Adapter::get_presentation_timestamp` only to correlate
    clocks. Actual display timing (`VULKAN_GOOGLE_DISPLAY_TIMING`) is reachable only
    through `as_hal` and is "generally only available on Android".
  - Candidate I can therefore portably change only the mode, the latency (1 against 2) and
    the point in the frame where `get_current_texture` is called. Measuring photon time
    needs external equipment or per-OS APIs.
- **MSRV and dependency fit:** the rust-version of each candidate dependency is at or
  below pert-ui's 1.99.1: wgpu 30.0.1 needs 1.87.0, vello/vello_cpu/vello_gpu 1.89,
  femtovg 1.88, egui 1.95 and peniko 1.85 (crates.io).

## 5. Could not verify
- iced's clip and transform model (rect-only clips, translate/scale only) and cryoglyph's
  atlas details are from memory, as is whether iced tests wgpu against tiny-skia parity.
- Skia Ganesh specifics (GrFillRRectOp, blur-profile textures, path-renderer chain),
  Skia's SDF-text size thresholds, the CPU raster backend's relation to the GPU, and what
  Graphite's `WideTileRenderStep` does.
- Classic vello's buffer-allocation failure modes. Whether vello_cpu and vello_gpu are
  pixel-identical in practice: the parity is inferred from the shared `vello_common`, and
  the local copy read is 0.0.9, older than 0.3.0. The 4×4 tile size was read there.
- GPUI on Windows (renderer backend not checked); its "≈20% CPU" improvement (secondary
  source only); that it has no CPU renderer.
- Makepad's GPU-side AA width (`aa` derivation in the shader) and its text path; only the
  CPU mirror was read.
- Impeller's exact UberSDF AA function. Only the parameter comments were read, not the
  shader body.
- Slint's per-renderer test strategy. Whether the software renderer's analytic shadow is
  in a release: it is on master, and the docs page still lists drop shadows as
  unsupported.
- The BGRA storage feature name and typical adapter `max_texture_dimension_2d` (16384) are
  from memory. No device query was run, because builds were excluded.

## Sources
- wgpu 30.0.1: local `~/.cargo/registry/src/*/wgpu-types-30.0.1/src/{limits.rs, features.rs, surface.rs, texture/format.rs}`, `wgpu-30.0.1/src/api/adapter.rs` (= https://docs.rs/wgpu/30.0.1)
- epaint/egui-wgpu 0.35.0 local; changelog https://raw.githubusercontent.com/emilk/egui/main/crates/epaint/CHANGELOG.md
- iced: https://github.com/iced-rs/iced/tree/master/wgpu/src/shader (quad/solid.wgsl, quad/shadow.wgsl, quad/gradient.wgsl, color/linear_rgb.wgsl), tiny_skia/src/engine.rs, renderer/src/fallback.rs, CHANGELOG.md (0.14.0, 2025-12-07)
- Slint: https://docs.slint.dev/latest/docs/slint/guide/backends-and-renderers/backends_and_renderers/ ; https://github.com/slint-ui/slint/tree/master/internal/renderers (femtovg/itemrenderer.rs, software/box_shadow.rs, software/draw_functions.rs)
- vello: https://github.com/linebender/vello (README), https://docs.rs/vello/0.11.0 , https://docs.rs/vello/0.11.0/vello/enum.AaConfig.html , https://docs.rs/vello_cpu/0.3.0 , https://docs.rs/vello_gpu/0.3.0 ; local vello_common-0.0.9
- Skia: https://blog.google/chromium/introducing-skia-graphite-chromes/ (2025-07-08); https://skia.googlesource.com/skia/+/refs/heads/main/src/gpu/graphite/render/AnalyticRRectRenderStep.cpp ; .../graphite/RendererProvider.cpp ; .../graphite/render/ (listing)
- NanoVG: https://github.com/memononen/nanovg (README, src/nanovg_gl.h, src/nanovg.h); femtovg: https://github.com/femtovg/femtovg (README, src/renderer)
- Impeller: https://docs.flutter.dev/perf/impeller , https://docs.flutter.dev/perf/antialiasing (both 2026-08-21, Flutter 3.47); https://github.com/flutter/flutter/tree/master/engine/src/flutter/impeller (entity/shaders/rrect_blur.frag, uber_sdf_common.glsl, entity/contents/uber_sdf_contents.h, entity/geometry/geometry.h, typographer/glyph_atlas.h); https://github.com/flutter/flutter/issues/123671 ; https://github.com/flutter/engine/pull/51250
- GPUI: https://zed.dev/blog/videogame (2023-03-07); https://github.com/zed-industries/zed/tree/main/crates/gpui_wgpu/src (shaders.wgsl, shaders_storage.wgsl, shaders_webgl.wgsl, shaders_subpixel.wgsl, wgpu_renderer.rs, wgpu_atlas.rs); https://github.com/zed-industries/zed/pull/46758 ; https://news.hada.io/topic?id=26669
- Shadows: https://madebyevan.com/shaders/fast-rounded-rectangle-shadows/ ; https://raphlinus.github.io/graphics/2020/04/21/blurred-rounded-rects.html (cited by Impeller)
- Makepad: https://github.com/makepad/makepad/blob/main/libs/math/src/shader_runtime.rs
- Versions: https://crates.io/api/v1/crates/<name> (queried 2026-11-05)
