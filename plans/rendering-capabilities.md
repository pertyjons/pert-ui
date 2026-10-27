# Rendering capabilities and performance

Status: **draft, open exploratory plan**, created 2026-10-26 at the
maintainer's request. Not scheduled: it follows the developer-usability
work (the [layering pass](usability/layering-plan.md) and round 2) unless
the maintainer decides otherwise. Nothing here is a commitment until the
investigation (phase 0) has reported and the maintainer has chosen what to
build.

## Why

The rendering path is deliberately small, and that has costs worth
re-examining:

- **Four primitives.** Widgets draw through `Painter`, which records a
  `DisplayList` of `FillRect`, `DrawMask` (a coverage mask times a color),
  `DrawImage` and `DrawVector`. There is no gradient, shadow, blur, opacity
  group, transform or shaped clip.
- **Coverage on the CPU.** Rounded corners, ellipses, arcs, triangles and
  vector paths are rasterized on the UI thread into antialiased masks
  (`crates/pert-ui-runtime/src/shapes.rs`, `vector_scene.rs`), cached per
  runtime and uploaded as textures. A new size, radius or scale means new
  rasterization and upload, so animated geometry and live vector data
  (traces, waveforms) cost CPU time every frame.
- **Three minimal shaders** in `pert-ui-wgpu` (solid quads, masks, images)
  draw everything as alpha-blended quads, six vertices each. The renderer
  re-encodes only changed commands and uploads changed byte ranges
  (`retained.rs`).
- **Measured so far:** a damage-rectangle path against full redraw showed
  no established gain on the measured device; surface acquisition (about
  8 ms, waiting for vsync) dominated, not drawing
  (`docs/verification.md`, the damage probe).

The maintainer's direction: the `DisplayList` may gain new parts; the
library may change its rendering structure where that makes it simpler,
faster or more capable (AGENTS.md's compatibility policy for the
unpublished library).

## Principles

- **Open first, measured always.** Phase 0 looks at several possibilities
  before any is chosen; every implementation phase states the gain it
  expects and measures it.
- **The boundaries stay.** No wgpu or other backend type reaches the
  runtime or the facade; widgets keep drawing through `Painter`; the CPU
  renderer stays in scope and renders every primitive, with a stated
  parity tolerance against the GPU path; captures keep using real
  backends; nothing enters a time-critical producer callback.
- **Simpler is a goal, not only faster.** A direction that removes code
  (for example the coverage-mask cache) counts in its favor.
- **Each implementation phase** gets a design checkpoint (Codex review,
  maintainer approval), CPU-parity and capture tests, the API listing
  gate, and native evidence on the platforms available, recording what
  could not be run.

## Phase 0: investigation

The first step, and the only one planned in detail. Its output is a
report in `plans/rendering/` with a ranked list of candidates, each with
its expected gain, cost, risk and the `DisplayList` change it needs; the
maintainer then chooses the implementation phases.

1. **Needs.** What the repository's applications and probes draw today
   and what they work around or cannot draw: the gallery, the mixer
   (meters, traces, knobs), the diagram editor (edges, selection,
   zoom), the log explorer and inventory (dense text and tables),
   floating windows, menus and popovers (separation from the content
   behind them), focus indication and themes. Include what a theme
   author would want to express (gradients on meters and buttons,
   elevation, glows) and what the real-time use cases need (waveforms,
   spectra, scrolling traces).
2. **Gradients in particular** (the maintainer's question: could they be
   added simply?).
   - Linear, radial and conic gradients; color stops; extend modes
     (pad, repeat, reflect).
   - The interpolation color space (sRGB, linear, Oklab) and how it
     interacts with the theme's color system and palettes.
   - Banding and dithering.
   - The representation: a `Paint` (solid, gradient, later image or
     pattern) shared by every fill and stroke, rather than a gradient
     variant per primitive.
   - The cost on the GPU (a few uniforms per instance) and on the CPU
     renderer, and the parity tolerance between them.
   - The `Painter` API, theme roles for gradients and what inspection
     and semantics report.
3. **Techniques in other libraries,** for their primitive set,
   antialiasing method, paint model, text path and CPU fallback: egui
   (tessellated meshes with feathered edges), iced and Slint (instanced
   SDF quads), vello (compute-based 2D), Skia and its Graphite backend,
   femtovg and NanoVG (stencil-and-cover paths with paints), Flutter's
   Impeller, and Zed's GPUI (instanced primitives and a glyph atlas).
   The aim is to learn which choices fit a retained, backend-independent
   display list with a CPU renderer, not to adopt a dependency by
   default.
4. **A frame-cost baseline** on real workloads (the gallery, the mixer
   with live data, the diagram editor while zooming): scene composition,
   renderer preparation, upload, GPU time, presentation wait, memory for
   masks and textures, and input-to-photon latency. Without it, no
   candidate's gain can be judged.
5. **Constraints.** What wgpu's portable feature set allows on the
   supported platforms (Vulkan, DX12, Metal) and on software
   presentation; MSRV and dependency policy; how each candidate affects
   capture, inspection, the CPU renderer and the `api/` listings.
6. **peniko as the paint model** (the maintainer's request, 2026-10-27).
   [peniko](https://docs.rs/peniko) is Linebender's renderer-independent
   crate of paint and composition types: `Brush` (solid, gradient,
   image), `Gradient` with linear, radial and sweep positions,
   `ColorStops` and an interpolation alpha space, `Extend`, `ImageBrush`,
   `BlendMode` (`Mix` and `Compose`), `Fill` and `Style`. It depends on
   `kurbo` (geometry) and `color`, not on any renderer; vello builds on
   it, and so does the Linebender stack the text path already uses
   (parley, swash). Study:
   - Whether its model fits candidate A and G as the shape of a
     pert-ui-owned `Paint`, blend mode and layer, item by item, and what
     pert-ui must define itself: sweep direction and angle unit (peniko
     reversed the sweep direction's description in 0.5), the
     interpolation color space against the theme's color system, alpha
     interpolation, dithering and the CPU/GPU parity tolerance.
   - Its stability. As of 2026-10-27 it is pre-1.0 (0.6.1, 2026-05-15)
     with no stated 1.0 plan, and broke its API in 0.3 (the move to
     `color`), 0.5 (gradient positions as structs, `Image` to
     `ImageBrush`, `Font` to `FontData`) and 0.6 (kurbo 0.13); each kurbo
     release has meant a peniko release. Its types therefore cannot
     appear in the runtime or facade API (AGENTS.md); the question is
     whether to mirror its concepts in pert-ui's own types, depend on it
     inside a renderer only, or both.
   - What depending on it inside `pert-ui-cpu` and `pert-ui-wgpu` would
     cost: dependency weight, MSRV (1.85 in 0.6), licence (Apache-2.0 or
     MIT), and the translation from pert-ui's types.
   - With it, the renderers of the same family as a backend option
     (question 3): `vello_cpu` and `vello_gpu` (formerly `vello_hybrid`),
     both 0.3.0 and both on the sparse-strips pipeline, `vello_gpu` on
     wgpu 30 as pert-ui is. A spike that renders the `DisplayList`
     through both would measure frame cost and CPU/GPU parity against
     today's renderers. Their documentation states that mask layers,
     complex filter graphs and some blend modes panic, which the
     translation would have to exclude, and that glyph caching is
     experimental.
   - `anyrender`'s `PaintScene` trait (fill, stroke, layers, clip
     layers, glyph runs, box shadow, image) only as a checklist of the
     primitives a display list needs, not as a dependency: the
     `DisplayList` already is pert-ui's backend abstraction, and that
     trait's signatures use kurbo and peniko types.

## Candidate directions

None is chosen. Phase 0 ranks them and may add or drop any.

- **A. A paint model with gradients.** `Paint` for fills and strokes:
  solid, linear, radial and conic gradients, with defined interpolation
  and dithering. peniko's types (phase 0, item 6) are the reference
  model to compare against.
- **B. Analytic shapes.** Rounded rectangles with per-corner radii and
  borders, ellipses and arcs computed in the fragment shader from
  distance fields, and by the same formula in the CPU renderer. It
  removes CPU coverage masks for ordinary shapes, their cache and their
  uploads, and makes animated sizes and radii free.
- **C. Instanced rendering.** One instance per primitive instead of six
  vertices: less data, smaller uploads, simpler batching.
- **D. Shadows and blur.** Analytic box shadows (an error-function
  falloff, no blur pass) for elevation; a true backdrop blur only if the
  needs survey shows a use worth its render passes.
- **E. A glyph atlas.** Glyphs rasterized once into a shared atlas and
  drawn as instanced quads, for dense text.
- **F. Lines and live vector data on the GPU.** A polyline or
  line-strip primitive with antialiased segments read from a buffer, so a
  new waveform costs only its points; full GPU path rendering
  (tessellation or compute) studied as a larger later step.
- **G. Shaped clips, opacity groups and transforms.** Rounded clips,
  group opacity (fade a whole panel), and affine transforms for knobs,
  rotation and zoom, each with its effect on hit testing and capture
  stated.
- **H. Layers and partial redraw.** Cached textures for expensive static
  subtrees, with a stable layer identity and revision in the
  `DisplayList`; only if the baseline shows a workload that needs it,
  since the damage probe found no gain for a cheap scene.
- **I. Latency.** Present mode, frame queue depth and frame start
  relative to vsync, for input-to-photon time, which matters more than
  drawing cost for playing and recording music.
- **J. Anything phase 0 finds** with comparable value.

## Open questions for the maintainer

1. Which visual capabilities matter most for the intended applications:
   gradients, shadows, blur, transforms, or others?
2. Is exact pixel parity between the CPU and GPU renderers required, or a
   stated tolerance per primitive?
3. May the investigation consider a rendering dependency (such as vello)
   as an option, or should the renderer stay self-written?
4. Where does this plan run relative to the usability work and round 2?

## Review record

None yet. Phase 0's report is reviewed by Codex before the maintainer
chooses.
