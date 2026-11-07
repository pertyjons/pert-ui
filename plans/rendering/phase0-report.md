# Rendering capabilities: phase 0 report

Status: **Codex-reviewed and corrected; the maintainer decided on 2026-11-06**
(see [Decisions](#decisions)). Written
2026-11-05 on branch `rendering-phase0` at `78246d1` (0.3.102), for phase 0
of [the plan](../rendering-capabilities.md). Nothing here is a commitment;
the ranking at the end is a recommendation.

The detailed sections are separate files beside this one:

| Item | File |
|---|---|
| 1. Needs, and today's drawing path | [needs.md](needs.md) |
| 2. Gradients; 6. peniko and the vello facts | [gradients-peniko.md](gradients-peniko.md) |
| 3. Other libraries; 5. wgpu constraints | [libraries.md](libraries.md) |
| 4. Frame-cost baseline | this file, raw output in [baseline-probe/results/](baseline-probe/results/) |
| 6. vello_cpu / vello_gpu spike | this file, [vello-spike/](vello-spike/) |

Claims in the section files cite `file:line` or a URL with a version.
"(memory)" marks a fact that was not checked against a source, and
"Inference" marks a conclusion that was not measured.

## Summary

- **Static UI is cheap on the UI thread today.** On the measured iGPU, a
  gallery page composes in 0.05–0.13 ms when nothing changed. Presented to a
  window, a page-like scene costs 0.45 ms of UI-thread time per frame,
  `render` call included, and ten times the commands 0.99 ms. Its GPU
  completion time cannot be measured through the public API. Analytic shapes,
  instancing and a glyph atlas therefore cannot buy much UI-thread time on
  static pages. Their case is capability and simpler code.
- **The largest measured frame costs are not in GPU drawing:**
  - **Window resize or scale change:** 19–117 ms of UI work per frame, almost
    all in `Flush` and `Text`. Resizing caused next to no new coverage masks.
  - **Live polylines:** CPU vector rasterization takes 3–5 ms per frame on
    every change.
  - **Presentation:** a FIFO swap-chain acquisition waits about 8.5 ms.
  - **CPU renderer:** 4–16 ms per frame at scale 1 and 15–63 ms at scale 2,
    when software presentation draws whole frames.
- **What the applications cannot express:** gradients (the color picker fakes
  them with CPU images), shadows or elevation, rounded clips, per-corner
  radii, group opacity and rotation. A popover is separated from what lies
  behind it only by a 1 px border. A theme author has only solid colors and
  lengths.
- **Other libraries converge** on an analytic rounded-rectangle instance with
  a paint index (GPUI, iced, Skia Graphite, Impeller's SDF shapes), and on a
  closed-form box shadow: an erf in GPUI, an approximate analytic blur in
  Impeller, a smoothstep falloff in iced. The fragment formula is a pure
  function of the pixel center, so the CPU renderer can evaluate the same
  formula. That is the best parity story of
  any direction.
- **peniko** is a good model but not a dependency for the runtime or facade.
  It is pre-1.0, it releases a breaking minor for each kurbo release, and its
  clip/layer model is still being designed. The recommendation is to own a
  `Paint` shaped like peniko's.
- **vello** composites in sRGB-encoded space, while pert-ui blends in linear
  light; neither vello crate offers a linear-light mode. Translucent gallery
  fills therefore differ by about 22/255, so a vello backend cannot meet
  today's parity tiers without a contract change.
- **pert-ui's own CPU renderer is the slow one.** On the same gallery scenes,
  a complete single-threaded vello_cpu frame (translation, render and output
  copy) takes 0.58–1.51 ms, against pert-ui CPU's 3.8–15.4 ms in the same
  session (cpu-blend-variants). The render alone takes 0.36–1.21 ms against
  3.9–16.1 ms in the spike. That is a measured improvement area for software
  presentation (J2).
- **Path choice:** the measurements favour keeping the own renderers and
  linear light (path 1). Presenting without read-back, pert-ui wgpu spends
  less UI-thread time than vello_gpu on the panel scenes, and more on the
  CPU-rasterized polyline and on the gradient bars. Blending on sRGB-encoded
  bytes made pert-ui's CPU renderer no faster. Its time goes to per-pixel
  work per command, and a fast CPU renderer under the linear contract is
  still unproven work (J2).
- **Recommended first implementation phase:** B + C, analytic shape instances
  read from a storage buffer, with a paint slot designed in. Gradients (A) and
  box shadows (D) follow on the same primitive. The GPU polyline (F), the CPU
  renderer's speed (J2) and a latency experiment (I) are the items with a
  measured performance case.
- **The baseline is incomplete.** The mixer and the diagram editor, GPU time,
  upload, texture memory and input-to-photon latency were not measured; see
  [What the baseline does not cover](#what-the-baseline-does-not-cover).

## 4. Frame-cost baseline

### Environment and method

- **Machine:** Fedora 44, GNOME 50 on Wayland, kernel 7.2.15-200.fc44.x86_64,
  12th Gen Intel Core i9-12900, Intel UHD Graphics 770 (ADL-S GT1) with Mesa
  26.2.6 on Vulkan, rustc 1.99.1, release builds. Raw output and the
  environment are in [baseline-probe/results/](baseline-probe/results/).
- **Pinning:** runs were pinned with `taskset` to P-cores (CPUs 2–7, or CPU 2
  for single-thread probes). One E-core run (CPU 18) shows the spread.
- **Status:** these are local measurements on one device, not targets and not
  cross-platform results.

New for this phase: [baseline-probe/](baseline-probe/) (`run.sh` +
`probe.rs`). It mounts the real gallery headlessly at 1280×800 logical
pixels, scale 1 and 2, and for every page records:

- the command mix and resource bytes of the composed `DisplayList`;
- cold composition (first paint of a new runtime) and warm composition;
- the CPU renderer's time;
- the wgpu renderer's `render_offscreen` time: preparation, one pass,
  completion and read-back, against an empty scene's read-back.

Two change scenarios follow:

- a window resize of +7 logical pixels per frame over 30 frames;
- scale steps 1 → 1.25 → 1.5 → 1.75 → 2.

Both count the coverage masks each frame newly needs and break the runtime
profiler's self time down by operation. The existing probes were rerun at
HEAD:

- `native_realtime_workload_probe` and `native_damage_area_probe`
  (pert-ui-wgpu);
- `stroke_phase_profile` (pert-ui-runtime);
- the `frame_cost` example (pert-ui-cpu);
- the gallery's `--live-benchmark` and `--zoom-benchmark`.

### Gallery pages ([results/pages.md](baseline-probe/results/pages.md))

| Quantity, per page | Scale 1 (1280×800) | Scale 2 (2560×1600) |
|---|---|---|
| `FillRect` commands | 58–343 | 58–352 |
| `DrawMask` commands (shapes, text, icons) | 84–446 | 84–446 |
| Distinct mask resources | 31–102 | 31–102 |
| Mask bytes | 55–216 KiB | 220–863 KiB |
| Compose, cold | 0.12–0.67 ms | 0.14–1.73 ms |
| Compose, warm (nothing changed) | 0.05–0.13 ms | 0.05–0.14 ms |
| CPU renderer, whole frame | 3.9–16.1 ms | 14.8–62.9 ms |
| wgpu offscreen, warm, median | 2.5–4.6 ms | 8.1–9.4 ms |
| Read-back of an empty scene | 3.05 ms | 6.85 ms |

Reading:

- **The read-back dominates offscreen renders.** An empty scene's render and
  read-back takes 3.05 ms, about as much as a populated page; some pages
  measured below it (2.65 ms for forms, 2.66 ms for scroll), so the overhead
  is not a stable constant to subtract, and this table bounds no GPU time.
  The presentation measurement below replaces it: the `render` call takes
  0.37 ms of UI-thread time for a page-like scene and 0.79 ms for ten times
  its commands.
- **Composition time.** The slowest cold composition is the properties page:
  1.6 ms of `Paint` self time at scale 2. A shape-heavy widget there
  rasterizes new masks; inference: probably the knob arcs, which are 4×4
  supersampled per sample with `atan2`. The other cold compositions stay
  below 0.55 ms.
- **CPU renderer.** The colors page is the slowest, because of its stretched
  gradient images; the others take 3.9–9.0 ms at scale 1. The
  [CPU renderer section of verification.md](../../docs/verification.md#cpu-rendering-cost)
  recorded 41 ms for the `frame_cost` 1280×800 grid on 2026-10-05; today it
  takes 20.1 ms.

### Change scenarios ([results/](baseline-probe/results/))

| Page | Scenario | UI work per frame, median | New masks per frame | Largest profiler self times per frame | wgpu offscreen |
|---|---|---|---|---|---|
| navigation | resize | 21.5 ms | 1 (54 KiB) | Flush 18.7, Text 2.2 | 7.4 ms |
| navigation | scale step | 27.0 ms | 49 (658 KiB) | Flush 18.2, Text 8.0 | 12.2 ms |
| menus | resize | 19.1 ms | 0 | Flush 18.1, Text 0.6 | 7.7 ms |
| menus | scale step | 22.0 ms | 31 (416 KiB) | Flush 18.4, Text 2.7 | 14.2 ms |
| live | resize | 97.0 ms | 0 | Text 70.1, Flush 22.3 | 9.1 ms |
| live | scale step | 117.4 ms | 101 (582 KiB) | Text 87.0, Flush 22.3 | 16.9 ms |

The offscreen column includes resizing the render target and the read-back.
"UI work" is `set_viewport` + `flush` + `paint`.

- **Resizing creates almost no new coverage.** Corner masks are keyed by
  corner size and scale, not by rectangle size (needs.md §1.2). A resize
  costs 19–97 ms of UI work per frame, though: well beyond a 60 Hz frame
  even on a light page.
  - About 18–22 ms of it is `Flush` self time, that is, time outside every
    measured nested scope. It barely depends on the page.
  - On the live page, `Text` adds 70 ms.
  - This is not a rendering cost; see
    [findings outside this scope](#findings-outside-this-scope).
- **A scale change** brings 31–101 mask resources not in the previous
  frame, 0.4–0.7 MB. These are all of the scene's masks, text and icons
  included; the probe neither separates shape masks nor counts rasterizer
  calls. Either way, it is small against the text and flush time in the same
  frames.
- **Zoom** (`--zoom-benchmark`, [zoom.txt](baseline-probe/results/zoom.txt)):
  - A step during the gesture costs a median 0.67–0.87 ms, because prepared
    drawing is stretched.
  - Once the scale rests, re-preparation takes 10.9–60.6 ms, with about 770–830
    text-service calls and 384 updates.
  - The share of that time spent on shape masks was not separated.

### Real-time content ([realtime-cpu2.txt](baseline-probe/results/realtime-cpu2.txt))

Per-frame medians on CPU 2, 960×640, an 800×400 plot:

| Workload | Rasterize | Prepare | GPU pass | Total |
|---|---|---|---|---|
| Min/max columns, 4096 samples | 6 µs | 0.36 ms | 0.49 ms | 0.94 ms |
| Polyline, 800 points | 3.16 ms | 43 µs | 0.44 ms | 3.74 ms |
| Polyline, 4096 points | 4.29 ms | 41 µs | 0.45 ms | 4.97 ms |
| Waterfall image 512×256 | < 1 µs | 33 µs | 0.63 ms | 1.13 ms |

- **E-cores.** On an E-core (CPU 18) the polyline rasterization takes 5.0 and
  7.0 ms.
- **Change since September.** On 2026-09-30, `verification.md` recorded
  1.49–2.54 ms for 800 points. The recheck on a quiet machine found no
  regression within the repository's history; see
  [findings outside this scope](#findings-outside-this-scope).
- **Conclusion either way:** an antialiased trace that changes on every frame
  costs a fifth or more of a 60 Hz frame on the UI thread. The un-antialiased
  column envelope costs under 1 ms.

### Presentation and partial redraw ([damage.txt](baseline-probe/results/damage.txt))

- **Surface acquisition** takes a median 8.5 ms under `Fifo` with frame
  latency 2.
- **Partial redraw:** direct full rendering took a median 0.82 ms, and damage
  plus surface copy 0.74 ms. As in September, the difference is too small to
  justify layers (H) on this workload.

### What the baseline does not cover

Item 4 of the plan asks for the gallery, the mixer with live data and the
diagram editor while zooming, each with composition, preparation, upload,
GPU time, presentation wait, mask and texture memory, and input-to-photon
latency. This phase delivered part of it, so item 4 is **incomplete**:

- Measured: composition, the CPU renderer, offscreen render with read-back,
  presentation without read-back (UI-thread time), the presentation wait,
  mask bytes, and the gallery's zoom benchmark.
- Not measured:
  - the mixer and the diagram editor; the gallery's live and transforms
    pages stand in for them;
  - preparation and upload separately from the `render` call;
  - pert-ui's GPU time: wgpu's `TIMESTAMP_QUERY` is optional (libraries.md
    §4), and `render` exposes no submission to wait on;
  - texture pool memory;
  - input-to-photon latency and missed deadlines during real presentation;
  - Windows, macOS and other GPUs.

Question 6 below asks whether to complete these before phase 1 or accept the
reduced scope.

## Needs, libraries, gradients and peniko in brief

**Needs** ([needs.md](needs.md)).

What the drawing path does today:

- Every primitive takes one solid color. Clips are rectangles.
- A rounded rectangle becomes 3–4 `FillRect`s plus four corner masks taken
  from one template, held in a FIFO cache of 128 entries.
- Text is one mask per brush run. There is no atlas.
- Vector coverage is rasterized again whenever its geometry changes; the
  knob creates a new arc mask, pointer path and readout mask per value.
- The GPU draws six vertices per command and uses one texture and bind group
  per mask.

Who would benefit:

- **Gradients:** the color picker, meter zones and theme surfaces.
- **Shadows:** popovers, menus, toasts, floating windows and dialogs.
- **Rounded clips:** rounded panels and the window title bar.
- **Line primitive:** the built-in `Trace` and the diagram editor's edges.
- **Group opacity:** fades and the drag preview.

Correction to the plan: the mixer example has meters and sliders, but no knob
and no trace. Those live in the gallery.

**Other libraries** ([libraries.md](libraries.md)).

- **No library promises CPU/GPU parity.** Makepad's CPU mirror of its SDF
  helpers comes closest.
- **Pitfalls to design around:**
  - A fixed 0.5 px SDF threshold breaks under zoom and rotation; pass the
    pixel size as data rather than using `fwidth`.
  - Coverage blended in linear light makes thin dark strokes and text look
    thinner.
  - Abutting antialiased edges leak the background (conflation). Drawing
    fill and border in one instance avoids it.
  - Gradients band unless dithered.
  - A blurred shadow overdraws a 3σ margin.
- **wgpu 30 portability:** instancing, vertex pulling from storage buffers
  and compute are available on Vulkan, DX12 and Metal. MSAA only 1× and 4×
  are guaranteed.
  - Atlas size: textures default to 8192 px per side.
  - GPU timing: timestamps inside passes do not work on Apple GPUs.
  - Latency: `desired_maximum_frame_latency = 1` is what wgpu advises for
    low-latency GUIs. Mailbox and Immediate availability varies by platform.

**Gradients** ([gradients-peniko.md](gradients-peniko.md) A).

- **Interpolation:** Oklab by default, as CSS Color 4 does. Linear light and
  sRGB-encoded are options; the alpha is always premultiplied.
- **Resolution:** the runtime turns each gradient into a bounded
  piecewise-linear ramp in linear premultiplied light. Both renderers
  evaluate the same segments.
- **Dither:** an integer-indexed ordered or blue-noise pattern, keyed by
  physical pixel position and applied in the encoded domain. Both renderers
  compute the same values, and solid fills stay in the exact parity tier.
  Theme-style gradients will band without it; a surface-to-hover ramp spans
  only 17 byte steps.
- **Validation:** non-monotonic, out-of-range and non-finite stops are
  rejected at construction.
- **Inspection:** reports `None` as the solid background for a gradient, plus
  a new paint description; that is a schema bump.

**peniko** (B). Version 0.6.1 (2026-05-15), MSRV 1.85, Apache-2.0 OR MIT.

- **Releases:** three breaking minors in two years. Each kurbo minor has
  forced a peniko minor. There is no 1.0 plan.
- **Recommendation:** "both". pert-ui owns `Paint`/`Gradient` with peniko's
  shape and names where the semantics agree. It differs deliberately in
  using linear colors, degrees and an Oklab default. peniko appears only
  inside an optional vello backend, if one is ever earned.

**vello facts** (C). vello 0.11.0, vello_cpu 0.3.0 and vello_gpu 0.3.0 were
released 2026-10-02 (MSRV 1.89). vello_gpu needs wgpu ^30, which pert-ui
satisfies. The translation must avoid what panics:

- vello_gpu panics on mask layers, complex filters and some blend modes.
- vello_cpu's multithreaded mode panics on filters, and if `flush` is skipped.
- Glyph caching is experimental in both.

## 6. vello spike

Full tables, images and method are in
[vello-spike/results/results.md](vello-spike/results/results.md); the
[README](vello-spike/README.md) reproduces them. vello_cpu, vello_gpu and
vello_common 0.3.0 build on peniko 0.6.1 and kurbo 0.13.1. vello_gpu needs
wgpu 30.0.0, so Cargo resolved one wgpu 30.0.1 for pert-ui-wgpu and
vello_gpu in one binary. Its default features must be off, or they enable
wgpu's default backends for every wgpu user in the build. The spike adds 18
crates for vello_cpu (with rayon for multithreading) and 62 for vello_gpu,
mostly the build-time shader compiler.

**Caveat:** unrelated processes ran during the spike (1-minute load average
1.3–6.7), so it took three runs. The medians agree within a few percent
except where results.md gives a range.

### Same synthetic content in four renderers

Median frame totals in ms, at scale 1 / scale 2, with the GPU renderers
including a read-back:

| Workload | pert-ui CPU | pert-ui wgpu | vello_cpu, 1 thread | vello_cpu, 5 workers | vello_gpu |
|---|---|---|---|---|---|
| Panels: 150 rounded + 100 plain, about 10× a page's commands | 4.18 / 15.6 | 4.85 / 9.2 | 1.47 / 2.68 | 0.54 / 0.97 | 4.29 / 10.5 |
| Panels light: 20 + 20, close to a gallery page | 2.58 / 13.6 | 3.98 / 8.8 | 0.48 / 1.38 | 0.22 / 0.61 | 4.4 / 9.2 |
| Polyline, 800 points | 12.2 / 42.2 | 5.75 / 16.2 | 0.60 / 1.85 | 0.58 / 1.30 | 3.56 / 9.0 |
| Polyline, 4096 points | 14.0 / 44.4 | 7.1 / 17.0 | 1.02 / 2.10 | 1.00 / 1.62 | 3.79 / 9.1 |
| Gradient bars (pert-ui: a CPU-built image per bar) | 11.5 / 45.6 | 4.11 / 8.5 | 0.71 / 2.16 | 0.40 / 0.95 | 3.70 / 8.4 |

- **Read-back overhead.** An empty scene with read-back takes 3.0 / 7.0 ms in
  pert-ui wgpu and 2.4–2.7 / 4.8 ms in vello_gpu.
- **vello_gpu is bound by waits.** Its total is dominated by two blocking
  waits, GPU completion and read-back, of 1.3–4.9 ms each, so with read-back
  it never beat vello_cpu. Presentation without read-back was measured
  afterwards; see [the path choice](#results).
- **vello_cpu redraws the whole frame each time** and is still the fastest
  renderer on every workload. Its single-threaded polyline (0.6–1.0 ms)
  undercuts pert-ui's CPU stroke rasterization alone (3–5 ms, section 4).
- **vello re-encodes its scene every frame:** it has no retained scene.
  Rendering an unchanged vello_cpu context again was not measured.

### Parity, with pert-ui CPU as the reference

- **Opaque interiors are identical everywhere** (max 0). All differences lie
  on edges and translucent pixels.
- **Blend space**, from source:
  - vello_cpu's u8 pipeline composites source-over on the stored
    sRGB-encoded bytes.
  - vello_gpu's blend shader assumes sRGB-encoded colors.
  - peniko gradients interpolate in sRGB by default.
  - There is no linear-light switch.
  - Translating pert-ui's own corner masks isolates the blend-space effect:
    max 41–43, edge mean 0.25–0.81.
- **Coverage.** vello's native rounded rectangles reach max 69–78, edge mean
  0.4–1.2. The polyline differs most: edge mean 15.7–17.1, max 134–157.
  Compositing in linear light only lowers that edge mean to about 11.5, so
  most of the polyline difference is the rasterizers', not the blend space.
- **Gradients:** max 1, from LUT quantization.
- **Within each family:** vello_gpu against vello_cpu ≤ 2/255; pert-ui wgpu
  against pert-ui CPU ≤ 1, except one lead listed under
  [findings outside this scope](#findings-outside-this-scope).

### Translating a real `DisplayList` into vello_cpu

- **Every command of every gallery page translated; none was refused:**
  - `FillRect` → a rect fill, snapped to pert-ui's pixel-center rule;
  - `DrawMask` → an image paint with an alpha-mask tint;
  - `DrawImage` → an image paint, re-encoded from linear-premultiplied to
    sRGB-premultiplied;
  - `DrawVector` → a kurbo path;
  - clips → clip rects.
- **Translation cost:** 0.03–0.15 ms per warm frame and 0.26–1.76 ms on a
  page's first frame.
- **Pages, single thread:** the vello_cpu render alone takes 0.36 / 0.50 /
  1.2 ms (min / median / max) at scale 1 and 1.2 / 1.65 / 4.3 ms at scale 2;
  these phase timers exclude translation and the copy of the output buffer.
  pert-ui CPU takes 3.9 / 6.7 / 16.1 and 14 / 25 / 63 ms. The complete
  translator call, as timed in the blend benchmark below, takes 0.58 / 0.73 /
  1.51 ms at scale 1.
- **Translucent fills** on most pages differ by a mean of about 22/255 over
  the whole flat area. This is the blend space in its plainest form.
- **What does not map:**
  - linear-light compositing and image filtering;
  - image tints: vello multiplies encoded components (`TintMode::Multiply`),
    pert-ui multiplies linear ones, so a tinted image differs even when
    opaque and nearest-sampled (two byte-128 components give 61 in pert-ui
    and 64 in vello). No focused tinted-image comparison was run;
  - A8 masks: vello has no A8 format, so masks become RGBA8 at 4× the bytes
    (223–3454 KiB per page);
  - vello's own glyph runs: the `DisplayList` carries no glyph data.

### Limitations, from the vello 0.3.0 sources

- **vello_gpu:**
  - mask layers and destructive non-isolated blend modes panic;
  - an image source given as a pixmap panics; images must be uploaded;
  - resource textures are capped at 4096 px per side.
- **vello_cpu:**
  - complex filter graphs panic, and in multithreaded mode even simple
    filters do;
  - filters are experimental.
- **Both:**
  - glyph caching is experimental;
  - some failures panic instead of returning an error;
  - sizes are `u16`.
- Clip layers work in both. None of the spike's calls panicked.

### What the spike means for the ranking

- **vello as the GPU backend:** no gain on this iGPU with read-back, and it
  breaks the blend contract. Presented without read-back it is mixed; see
  [the path choice](#results). Not recommended now.
- **vello_cpu as a reference** is strong evidence that a CPU renderer for
  this display list can be an order of magnitude faster. The obvious
  techniques: sparse strips, SIMD, u8 arithmetic and no per-command sRGB
  round trip.
  - Option one: make pert-ui's CPU renderer faster under the linear-light
    contract.
  - Option two: change the contract to sRGB-encoded blending, as vello,
    egui and browsers do, and consider vello_cpu as an implementation inside
    `pert-ui-cpu`.
  - Either is J2's question; the second needs a contract change and a
    maintainer decision.
- **peniko's shape for `Paint`** is confirmed usable: the gradient
  translation was mechanical and exact.

## The path choice

The spike turns the ranking into a choice between two paths that must be
made first, because they change what every later phase builds. Mixing the
two is the worst option: vello_cpu as the CPU renderer beside pert-ui's own
GPU renderer would pair two different rasterizers, and the polyline's edges
differed by 16/255 on average between them. The renderers come as a pair.

| | **Path 1: own renderers, linear light** | **Path 2: the vello pair behind `DisplayList`** |
|---|---|---|
| CPU renderer | `pert-ui-cpu`, made faster (J2, option one) | vello_cpu inside `pert-ui-cpu` |
| GPU renderer | `pert-ui-wgpu`, with analytic shape instances (B + C) | vello_gpu inside `pert-ui-wgpu` |
| Blend contract | Unchanged: linear light | sRGB-encoded, as vello, CSS and egui do |
| CPU/GPU parity | Kept by evaluating the same formulas (today ≤ 1, apart from the lead below) | From vello's shared strip stage (measured ≤ 2/255) |
| `DisplayList` | New shape, paint and shadow commands | Shapes and glyphs recorded as geometry, not masks, so that vello's strength is used (masks become RGBA at 4× the bytes) |
| Gradients, clips, layers | Written and kept at parity by pert-ui | vello's, within its unsupported set (mask layers, some blend modes, filters) |
| Errors | Typed, as AGENTS.md requires | A translation layer must refuse what vello panics on |
| Retained work | Changed commands only | The scene is re-encoded every frame (about 1 ms for the full panels scene) |
| Dependencies | None new | 18 + 62 crates, pre-1.0; wgpu upgrades in step with vello_gpu |
| Themes | Alpha values keep their meaning | Every translucent role must be reviewed (22/255 shift) |

Two measurements decide whether path 2 is worth its costs. Both run in the
spike's harness on an idle machine:

1. **Presentation without read-back.** vello_gpu and pert-ui wgpu each
   present frames to a window surface, with frames paced so that acquisition
   does not block. The measurement is UI-thread time per frame and GPU
   completion time. The read-back that dominated the GPU numbers above is
   not part of normal presentation.
2. **Where vello_cpu's speed comes from.** A throwaway copy of `pert-ui-cpu`
   in two variants, rendering the same gallery scenes:
   - (a) blending directly on 8-bit sRGB-encoded values;
   - (b) a linear 16-bit frame encoded once at the end.

   If (b) closes most of the gap, path 1 keeps the linear contract without a
   speed penalty. If only (a) does, the blend contract itself is the price of
   CPU speed.

### Results

Both runs were gated by
[tools/quiet-run.sh](tools/quiet-run.sh), which waits for a 1-minute load
below 2.0 and records the load and the processes with the most CPU at the
start and the end. The load was 0.37–1.18 at those points; the script did
not sample during these runs (it now records the highest load sampled
during a run), and `ps` reports each process's lifetime average.
The evidence for a quiet run is therefore that load and repeated medians:
three runs for presentation and two for the blend variants, agreeing within
a few percent.

**1. Presentation without read-back**
([vello-spike/results/presentation.md](vello-spike/results/presentation.md)).

Setup:
- frames paced to one per 30 ms on a Fifo surface (a ~100 Hz monitor), so
  acquisition never blocked;
- vello_gpu presented into `Bgra8Unorm`, because its shaders write encoded
  values;
- the compositor gave 2560×1373 instead of 2560×1600 for scale 2.

UI-thread time from the start of a frame to after present, median ms, at
scale 1 / scale 2. The boundaries differ slightly: pert-ui's time includes
the runtime update (signal, `flush`, and generating the workload's data),
about 0.03 ms, while vello_gpu's excludes generating its data:

| Workload | pert-ui wgpu | vello_gpu |
|---|---|---|
| Panels, about 10× a page | 0.99 / 0.98 | 1.47 / 1.84 |
| Panels light, about a page | 0.45 / 0.45 | 0.58 / 0.71 |
| Polyline, 800 points | 3.49 / 7.05 | 0.74 / 1.09 |
| Polyline, 4096 points | 4.65 / 8.67 | 1.14 / 1.53 |
| Gradient bars | 0.44 / 0.45 | 0.29 / 0.30 |

- **GPU completion.** vello_gpu's GPU work, from submit to completion, took
  1.5–2.1 ms at 1280×800 and 3.2–4.1 ms at 2560×1373. pert-ui's cannot be
  reached through the public API; the in-crate probe's offscreen pass, on
  different scenes at 960×640, takes 0.44–0.63 ms.
- **The read-back was the cost before.** The full panels scene takes 0.99 ms
  in pert-ui wgpu here, against 4.85 ms with read-back.
- **pert-ui wgpu spends less UI-thread time on the two panel scenes; vello_gpu
  less on the polylines and the gradient bars.**
  - The polyline cost is the CPU vector coverage inside `render`, exactly
    what candidate F addresses.
  - The bars are 32 clipped images in pert-ui against 32 gradient fills in
    vello_gpu: 0.44 against 0.29 ms. Gradients (A) would replace the images.
  - This compares UI-thread time only. vello_gpu's GPU work takes longer than
    its UI-thread time; pert-ui's could not be measured, so GPU cost is not
    compared.

**2. Where vello_cpu's speed comes from**
([cpu-blend-variants/results.md](cpu-blend-variants/results.md)). Variant (0)
is today's renderer; its copy reproduces it with max difference 0. The other
variants:
- (a) blends on encoded 8-bit bytes;
- (b) keeps a linear u16 frame and encodes once at the end;
- (c) is (b) plus an opaque-fill fast path.

Gallery pages, ms (min / median / max), with vello_cpu single-threaded in the
same session:

| | (0) | (a) | (b) | (c) | vello_cpu |
|---|---|---|---|---|---|
| Scale 1 | 3.8 / 6.5 / 15.4 | 4.4 / 7.1 / 14.6 | 6.4 / 8.7 / 16.5 | 4.3 / 6.0 / 13.6 | 0.59 / 0.75 / 1.53 |
| Scale 2 | 13.9 / 23.9 / 59.8 | 17.0 / 26.9 / 57.4 | 25.1 / 33.4 / 64.9 | 17.2 / 23.3 / 54.0 | 2.2 / 2.7 / 5.5 |

- **Neither blend storage closes the gap.** Comparing the median pages,
  sRGB-encoded blending (a) takes 9% (scale 1) and 13% (scale 2) *longer*
  than today. The 16-bit linear frame (b) takes 34% and 40% longer, because
  the frame is twice the size and has a fixed final encode. Only the fill
  fast path (c) gains, 8% and 3%. (results.md states these as shares of the
  gap to vello_cpu, a different measure.)
- **Where today's time goes** on the median page, measured per command kind:
  - mask commands 56–58%: coordinate mapping, bilinear sampling, blending
    and encoding of every covered pixel, not sampling alone;
  - fills 34–35%;
  - clearing the frame 8%.
- **Pixel differences against (0):** (b) and (c) differ by at most 1. (a)
  shows the same translucent shift as vello: a mean of 21–22, max 47.
- **Inference:** vello_cpu's speed comes from its architecture rather than
  from these storage choices. On the translated gallery its masks are still
  bilinearly sampled images, so analytic shape coverage cannot explain that
  gain; solid spans filled as runs, u8 SIMD arithmetic and its pipeline are
  the candidates. The experiment shows that the two scalar storage variants
  did not close the gap. It does not show that color-space work costs
  nothing in an optimized renderer: how fast a linear-light CPU renderer can
  become is unproven, and that work is a cost of path 1.
- **Caveat:** the benchmark re-renders one `DisplayList`, so vector coverage
  stayed cached. The polyline rows leave out pert-ui's 3–4 ms of rasterizing
  a changing line.

**What the measurements decide**

- **The blend contract is not shown to be the price of speed.** The CPU
  renderer's cost is per-pixel work per command, and changing to
  sRGB-encoded blending made it no faster by itself.
- **On the GPU, path 2 shows no UI-thread gain that the own renderer cannot
  reach.** pert-ui wgpu is ahead on the panel scenes. vello_gpu is ahead on
  the line, which has its own candidate (F), and on the gradient bars, which
  gradients (A) address. GPU time was compared only for vello_gpu.
- **Recommendation: path 1**, on measured UI-thread costs, the preserved
  blend and parity contract, and the estimated cost of each path. Its open
  costs are a faster CPU renderer under linear light (J2) and the
  implementation of A, B, D and F. Keep linear light and the own renderers. Take
  from vello what transfers: analytic coverage for shapes (B), solid-span fast
  paths and SIMD in `pert-ui-cpu` (J2), and peniko's shape for `Paint` (A).
  vello stays a reference and a measuring stick.

## 5. Constraints

- **Boundaries.** No candidate needs a backend type in the runtime or facade.
  Every one keeps `Painter` as the widget interface. An optional vello
  backend would be its own crate, with peniko and kurbo behind it.
- **Capture.** A new primitive is captured exactly when both renderers draw
  it, because capture uses the real backend. A backdrop blur, group opacity
  and layers need offscreen passes in both renderers.
- **Inspection.** `BoxModelInspection` reports one solid background and
  border. Gradients and shadows need new DTO fields and a schema bump.
  Affine transforms would break the axis-aligned box model and the half-open
  hit regions.
- **API.**
  - A new `DrawCommand` variant is additive, because the enum is
    `#[non_exhaustive]`.
  - Replacing `color` with a paint on existing variants is a breaking
    listing change.
  - `Shape` is not `#[non_exhaustive]`, so per-corner radii or an ellipse
    variant break exhaustive matches.
  - All of these are acceptable while the library is unpublished, with the
    migration recorded.
- **Platforms.** Everything ranked below uses core wgpu on Vulkan, DX12 and
  Metal, with no optional features. GPU timestamps would be an optional
  diagnostics feature.
- **MSRV.** Every dependency considered is at or below 1.99.1.

## Ranked candidates

Gain is stated against the baseline. Cost and risk are author estimates.
The ranking assumes path 1; under path 2, B, A, D and G come from vello and
the work moves into the translation layer and the `DisplayList` change.

| Rank | Candidate | Expected gain | `DisplayList` change | Cost | Risk |
|---|---|---|---|---|---|
| 1 | **B + C: analytic shape instances.** Rounded rectangles with per-corner radii and a border in one record, plus ellipses and arcs. Records are read from a storage buffer; the CPU evaluates the same f32 formula. | **Capability and simplicity.** <br>• Removes `ShapeCache`, the corner splitting, and per-value arc masks and uploads. <br>• Fill and border in one primitive avoid conflation seams. <br>• Zoom rest and scale changes no longer re-rasterize shapes (their share of the 0.4–0.7 MB of new masks per scale step was not separated from text). <br>• The base for A, D and rounded clips. <br>• Frame time on static pages: little, since a page's `render` call already takes about 0.4 ms of UI-thread time. | New variant(s), e.g. a shape command with radii, border and paint. `Shape` gains variants or per-corner radii. | Medium | AA tier: an SDF ramp instead of 4-row coverage changes edge pixels; needs a stated tolerance. Radii below 1 px and hairlines need care. |
| 2 | **A: `Paint` with linear, radial and conic gradients.** Oklab default, deterministic dither. | **Theme capability.** <br>• Replaces the color picker's CPU images. <br>• Meter zones and surface gradients. | `Paint` replaces `color` in commands (breaking). A gradient-ramp resource arena. Inspection paint DTO. | Medium, on top of 1 | Dither and parity tolerance; the theme-role design; the inspection schema bump. |
| 3 | **D: erf box shadow.** | **Separation of transient surfaces**; elevation roles for themes. | A shadow command (rect, radii, blur, spread, color). | Small, on top of 1 | CPU cost of the 3σ overdraw. |
| 4 | **F: GPU polyline.** Antialiased segments from a point buffer. | **Measured:** 3–5 ms per frame of CPU rasterization for 800–4096 changing points could fall below 1 ms (inference). Antialiased traces at about the cost of the column envelope. | A polyline command, or `DrawVector` strokes consumed natively. | Medium–high | Join and cap parity with the CPU stroker, which RENDER-04's grid phase already breaks for masks. (The suspected September slowdown was rechecked: none.) |
| 5 | **J2: CPU renderer speed, under the linear contract.** Solid-span fast paths for fills and fully covered mask runs; SIMD; possibly rows on several threads. Analytic shape coverage comes with 1. | **Measured:** 4–16 ms per frame at scale 1 and 15–63 ms at scale 2 for software presentation and captures; vello_cpu does the same scenes 7–17× faster on one thread, warm translation included. | None | Medium, partly independent of 1 | How fast linear light can become is unproven (see the blend variants); parity with the GPU must hold. |
| 6 | **I: latency experiment.** Frame latency 1; Mailbox or FifoRelaxed where offered; where in the frame the surface is acquired. | **Measured:** an 8.5 ms acquisition wait. Gain on input-to-present unknown until measured. | None | Small | Stutter or throughput loss; needs a measurement method for input-to-present. |
| 7 | **G, part 1: rounded clips and group opacity.** | Rounded panel content and fades. | A clip shape on commands; layer push/pop. | Medium, after 1 | Opacity needs an offscreen layer in both renderers. |
| 8 | **E: glyph atlas.** | No measured hotspot. Text cost lies in shaping and preparation on the UI thread, which an atlas only partly addresses. | Glyph-run command; text leaves `masks()`. | Medium–high | Breaks consumers that read text as masks. Defer until a dense-table scroll measurement shows a need. |
| 9 | **G, part 2: affine transforms.** | Rotation (knob pointer, rotated text) without new geometry. | Transform on commands or groups. | High | Hit testing, inspection box model, CPU sampling. |
| — | **H: layers and partial redraw.** | No evidence: 0.74 against 0.82 ms. | Layer identity and revision. | High | Dropped until a workload needs it. |
| — | **vello_gpu as the GPU backend.** | On the UI thread, presented: slower on panels (0.58 against 0.45 ms for a page), faster on the polyline and the gradient bars. GPU time 1.5–4.1 ms, not comparable with pert-ui's. | None; a translation of 0.03–0.15 ms per frame | Medium | sRGB-encoded blending breaks parity (22/255 on translucent fills); panics in place of errors. Not recommended now. |

J2 (rank 5) in more detail: the blend variants put the cost in per-pixel
work per command, mask commands taking 56–58% and fills 34–35%, sampling,
blending and encoding together. Neither storage variant helped. The way
forward is architecture under the linear contract: the solid-span fast
paths, SIMD and threads above, plus analytic coverage for shapes shared with
B. Whether that reaches vello_cpu's speed is unproven.

One item found by the baseline lies outside rendering. It is listed
separately because it is the largest measured cost:

- **J1. UI work on resize and scale change.** 19–117 ms per frame, mostly
  `Flush` self time and `Text`. Located on 2026-11-06 in
  [j1-resize.md](j1-resize.md); RESIZE-01 to RESIZE-04 in TODO.md.

## Findings outside this scope

Verified in code and recorded in [TODO.md](../../TODO.md):

- **RENDER-01, cache eviction.** `ShapeCache` (`shapes.rs:49-52, :139-148`)
  is FIFO: a hit does not refresh the entry.
- **RENDER-02, scrim color.** The modal scrim is a literal
  `Color::from_linear(0, 0, 0, 0.35)` (`dialog.rs:470`, `quick_pick.rs:1371`)
  that no theme role controls.
- **RENDER-03, doc error.** `stroke_path`'s rustdoc says `stroke_shape`
  strokes an "ellipse inward" (`geometry.rs:689-690`), but `Shape` has no
  ellipse.
- **RENDER-04, vector mask grid (the spike's parity lead, reproduced).** In
  the 800-point polyline at scale 2, frame 219, pert-ui wgpu and pert-ui CPU
  differ by up to 220/255 on 0.03% of edge pixels. The worst pixel is
  physical (478, 610), where the butt cap of the steep first segment reaches
  past the first point. `vello-spike/src/bin/capdiag.rs` dumps the prepared
  mask:
  - `RasterGrid::new` (`geometry/raster.rs`) anchors the coverage grid at the
    geometry's own bounds, here the cap corner, so the `DrawMask` rectangle
    starts at physical x = 478.50122;
  - the CPU renderer leaves pixel 478 out, because its center 478.5 lies
    0.0012 px left of the edge;
  - wgpu draws nearly the full line color there (measured). Inferred: the
    hardware snaps the edge, 0.0012 px away, onto the center and keeps the
    pixel, which then samples the clamped edge texel with coverage 249.

  This is the exception `docs/rendering-platform.md` already places outside
  both tiers, so it is not a regression. The finding is that ordinary vector
  content triggers it, at arbitrary phase. The same anchoring resamples every
  vector whose bounds fall at a fractional physical phase, which is nearly
  every one, and the half-covered first column and row are dropped or kept
  whole. Aligning the final mask rectangle with physical pixels removes all
  three effects. That means the grid phase must include the command's origin
  and scale, which `VectorRasterizer::prepare` applies after rasterizing,
  and a raster scale capped below the view's still stretches.
  Candidate F would draw polylines without a mask; every other vector keeps
  the CPU rasterizer, so the fix stands on its own.

Measured but not located, and not entered in TODO.md until the cause is read:

- **Resize and scale-change UI cost, located since (J1).** The 18–22 ms of
  `Flush` self time and the live page's `Text` time are explained in
  [j1-resize.md](j1-resize.md) and recorded as RESIZE-01 to RESIZE-04 in
  TODO.md. They are a visibility walk over whole subtrees, a full layout
  invalidation on a same-scale resize, a 128-entry FIFO text cache that
  thrashes, and rasterizing text during measurement. With the first three
  removed in an experiment, a resize costs 0.5–2.2 ms instead of 18–93 ms.
- **Polyline rasterization, rechecked; no regression.** On the quiet machine,
  CPU 2, the 800-point line rasterizes in 3.02 ms and the 4096-point line in
  4.07–4.09 ms, steadily over three runs
  ([realtime-quiet-*.txt](baseline-probe/results/)). The repository's first
  commit, 0.3.0 (`0c7cfb2`, 2026-10-15), measures the same: 3.01 and 4.06 ms
  ([realtime-0.3.0-*.txt](baseline-probe/results/)). So nothing regressed
  within the recorded history.
  - The 1.49–2.54 ms that `docs/verification.md` records for 2026-09-30
    predates that history and cannot be reproduced here; the cause is
    unknown.
  - That table is dated evidence; a later edit of verification.md could add
    today's figures beside it.

## Questions for the maintainer

1. Which first phase: rank 1 alone, or 1 + 2 (shape instances with paints)?
2. The parity tolerance for analytic AA edges. Keep exact and ±1 per channel
   for interiors, and state a wider per-primitive tier for edges?
3. Gradient interpolation: Oklab as the default?
4. The path choice: the measurements favour path 1 (own renderers, linear
   light). Agreed?
5. Should J1, the resize cost, be investigated now, before or beside the
   rendering work?
6. The baseline is incomplete (mixer, diagram editor, GPU time, upload,
   texture memory, input-to-photon latency). Complete it before phase 1, or
   accept the reduced scope? Candidate I needs an input-to-photon method
   either way.
7. The original open questions 1–4 of the plan still stand. The baseline
   suggests an answer to question 1: gradients, shadows and rounded shapes,
   in that dependency order.

## Decisions

The maintainer answered the questions above on 2026-11-06:

1. **Path:** path 1, own renderers and linear light. vello stays a
   reference and a measuring stick.
2. **First phase:** B + C alone, analytic shape instances, with a paint slot
   designed in. Gradients follow as phase 2.
3. **Edge tolerance:** exact and ±1 stay for interiors; analytic edges get
   their own documented tier, set from measurements in phase 1.
4. **Gradient default:** Oklab, with linear light and sRGB-encoded as
   options.
5. **J1:** investigated now, beside the rendering work.
6. **Baseline:** the reduced scope is accepted for these decisions. Phase 1
   measures the mixer and the diagram editor before and after as its own
   baseline; latency is measured when candidate I is taken up.
7. **Original questions:**
   - capabilities after phase 1, in order: gradients (A), shadows (D),
     rounded clips and group opacity (G1); arbitrary rotation (G2) is not
     prioritized;
   - parity and the dependency question are answered by 3 and 1;
   - scheduling: the plan runs now, as the next work. Next step: phase 1's
     design note, Codex-reviewed and approved before implementation.

## Review record

2026-11-05, Codex (codex-cli 0.166.0, read-only, static review of commit
`7b80e06`; no builds or reruns). Fifteen findings, six major and nine
minor, no blocker; the author checked each against the code and result
files and all held. Corrected here and in the section and result files:

- **Presentation conclusion** (major): vello_gpu is also faster on the
  gradient bars; the comparison is UI-thread time only, and the two UI-frame
  timings had different boundaries (about 0.03 ms of runtime update in
  pert-ui's). Wording and `presentation.md` corrected.
- **GPU-work bound** (major): subtracting the empty read-back bounds
  nothing, since some pages measured below it. Replaced by the presentation
  measurement.
- **Scale-step masks** (major): the 0.4–0.7 MB are all new mask resources,
  text included, not shape re-rasterization.
- **CPU experiment** (major): the mask share includes blending and encoding;
  the conclusion is now conditional, and a fast linear-light CPU renderer is
  listed as unproven work and a cost of path 1.
- **Incomplete baseline** (major): marked incomplete, with question 6.
- **Path recommendation** (part of the first and fifth): limited to measured
  UI-thread costs, the contract and estimated costs.
- **Minor:** the vello_cpu frame numbers (render alone vs complete frame),
  percentages that were shares of the gap, RENDER-04's fix (final placement
  phase, capped raster scale), image tints in the translation, the quiet-run
  evidence (the script now samples load during a run), J2 ranked as a
  rendering candidate (rank 5), the gradient contrast rule, the iced and
  Impeller library claims, and quarter turns in the rotation need.

Not examined by the reviewer: builds and reruns, native presentation, the
hardware snapping in RENDER-04 (the `capdiag` dump is reproducible, not
saved), upstream claims that need network access, and other platforms.
