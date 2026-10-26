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

## Candidate directions

None is chosen. Phase 0 ranks them and may add or drop any.

- **A. A paint model with gradients.** `Paint` for fills and strokes:
  solid, linear, radial and conic gradients, with defined interpolation
  and dithering.
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
