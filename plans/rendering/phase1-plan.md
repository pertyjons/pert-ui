# Phase 1 plan: analytic shapes

Status: **slices 0–6 done; slice 7 is done but for the merge: measurement, renderer repair (0.3.110), the specification's move, the final review with its repair (0.3.111) and the pre-merge gate. The maintainer approved the squash-merge into `main` on 2026-11-12.** Written 2026-11-06 on branch `rendering-phase0`
at dc74870 (0.3.103). The maintainer answered its questions on 2026-11-07
(see [Decisions](#decisions)). The slices were reordered on 2026-11-11 after
an evaluation of the plan (see [Revision](#revision-2026-11-11)). Slice 0's
before-measurement was last run on 2026-11-11 with the final probe; slice 1
is 6f3c81b, slice 2 e21ebf2 and slice 3 74cdd0f. Slice 4 set the edge tier
to one step and decided to go on (decision 6). Slice 5 made every shape a
`DrawShape` and removed the cache; slice 6, the widgets, is next.
It orders the work and the measurements
for the phase 1 spec, approved on 2026-11-06. The spec was deleted in slice 7,
as it said it would be: its rules are in the rustdoc of `PreparedShape`,
`DrawCommand::DrawShape` and the painter's shape methods and in
[docs/rendering-platform.md](../../docs/rendering-platform.md), and Git keeps
the file (last at 35ebfc6, `plans/rendering/phase1-shapes-spec.md`). "The
spec" below refers to it.

## Goal

Implement the spec: one `DrawShape` command for rounded and cut rectangles,
arcs and triangles, prepared once by shared code and evaluated per pixel by
both renderers. The shape cache and its coverage masks go away, and so do the
corner decomposition and the arc and triangle masks.

The phase is a foundation for gradients, shadows and rounded clips, and a
simplification. It is not expected to make frames faster: phase 0 and
slice 0 show that shapes cost little today (the mixer makes no new mask per
frame, and an isolated knob composes in about 0.2 ms per value). The
measurements below guard against regressions and confirm the resource
claims.

What the phase should achieve, each part measured in slice 0 and again in
slice 7:

| Expectation | Measure |
|---|---|
| Shapes add no mask resource and no mask bytes | Mask resources and KiB in the composed display list, per gallery page at scales 1, 1.25 and 2 |
| Turning a knob adds no mask resource | Display-list mask resources added per value by an isolated knob with no readout text; composition time per value on the gallery's `input` page |
| A zoom, live or at rest, costs no more, and its rest frame less where shapes were prepared | Composition time per zoom step and per rest frame in the gallery's zoom workload (64 cards) and in the diagram editor, and the display-list masks each frame newly needs |
| The mixer's live updates cost no more | Composition, CPU render and presentation, per frame with level updates |
| The CPU renderer is not materially slower | CPU render time per page and scenario |
| The GPU path costs no more UI-thread time | The `render` call presenting to a window without read-back, per page (first and repeated) and per frame; offscreen render with read-back as the capture path |
| Resize and scale changes keep the 0.3.103 gains | The J1 change scenarios on the page chosen before the change |

Three limits of these measures:

- **Masks are counted in the display list.** The probe counts them in the
  composed display list. It measures neither the rasterization done inside
  the renderers nor the bytes they upload, so the plan claims neither.
- **Shape and text masks are not told apart.** New masks per frame count
  text and icons too. Only the isolated knob, which has no text, shows shape
  masks alone; elsewhere the claim is that the count does not rise and falls
  by what shapes contributed.
- **No number is GPU execution time.** The presented `render` call is
  UI-thread time: acquisition, preparation, upload, encoding and submission.
  The offscreen number follows it on an already prepared scene and is
  dominated by the read-back (about 3 ms for an empty scene in phase 0). A
  more expensive fragment shader shows in neither; the plan claims nothing
  about it.

The CPU renderer's direction is not known in advance. It moves from sampling
a mask to evaluating a distance at edge pixels, with solid runs inside.

How a slowdown is judged:

- Slice 0 runs the before checkout in five rounds. Slice 7 builds both
  checkouts and runs them in five alternating, paired rounds on the same
  machine. All samples are kept.
- A scenario counts as slower when the median of its five after-run
  medians exceeds the largest before-run median by more than 5 % and by
  more than 0.05 ms. Slice 0's rounds differed by 1–2 %, so a smaller
  excess is reported as within noise. `summarize.py` applies the rule.
- Such a slowdown is reported to the maintainer before merge, not accepted
  silently.

## Working rules

- **Branch:** `rendering-phase1`, from `main` after `rendering-phase0` is
  squash-merged into it (decision 1).
- **Versions:** each slice that changes code bumps the workspace patch
  version, internal requirements and `Cargo.lock`, starting at 0.3.104.
- **CHANGELOG:** one section for the phase, `0.3.104–0.3.1xx: Analytic
  shapes`. Each slice adds to it: added and removed public items, every
  breaking change with its replacement, and the visible behavior changes.
  It records no measurements or review narrative.
- **Gates.** Every code slice bumps the version and so changes the internal
  dependency requirements. A slice therefore runs:
  - **Every code slice, 1 to 6:** the workspace gate and the API listing
    gate. The listing gate runs even where no public item changes, because a
    dependency requirement does.
  - **Slices 2, 3, 5 and 6, in addition:** release-profile tests and the feature
    configurations of `pert-ui` and `pert-ui-core` that the `headless` job
    in `.github/workflows/ci.yml` runs: no default features, `serde`, `cpu`,
    `desktop-cpu` and the image features. These slices change public API,
    errors and presentation across packages. The configurations cover
    `CaptureError`'s serde form and the separate software-only
    `RenderError`.
  - **Before merge, in slice 7:** every step the `headless` job runs beyond
    the workspace gate, among them the release tests, the `loom` models, the
    feature configurations, the headless gallery and library packaging. Then
    the MSRV build. These run on the final commit, after the last version
    bump, and cover all of the phase's dependency changes. The other checks
    above are not moved to slice 7; each slice still runs them.
  - **Exception (decision 5).** AGENTS.md's table puts dependency changes
    in the row that also runs packaging, the MSRV build and every feature
    configuration. A slice's version bump changes only the internal
    requirements between the workspace's own packages, so this plan runs
    that row once, on the final commit, instead of in every slice. The
    2026-11-11 review held that this departs from AGENTS.md; the maintainer
    approved the exception for this phase.
- **Review:** each code slice gets a Codex review (`codex exec -s
  read-only`). Its findings are checked and answered before the slice is
  committed, and the review record below lists them.
- **Documentation:** rustdoc, `docs/rendering-platform.md`,
  `docs/contracts.md` and the guides change in the same slice as the
  behavior they describe.
- **Commits:** a slice is committed when its gate and review have passed and
  the maintainer has asked for the commit.
- **Measurements:** run on a quiet machine with
  `plans/rendering/tools/quiet-run.sh`, pinned to one core. The maintainer
  reserves the machine when asked. Before and after are run on the same
  machine, with the same probe source.

## Slices

### 0. Baseline before the change (no version bump)

A probe, `plans/rendering/phase1-probe/`, built like `baseline-probe/`. Its
`run.sh` assembles a scratch crate per target checkout.

**Sources.**
- The gallery's and the two examples' sources come from that target
  checkout.
- The harness and the adapters are the probe's own files, the same for every
  checkout.
- The crate depends on the checkout by path, with the `png`, `svg` and `cpu`
  features. It has its own target directory per checkout.

**Adapters**, scratch-only. The examples' entry points are private, so the
script copies each example in as a module and appends an adapter to the
copied source; the adapter can then reach private items. The example files
in the repository do not change.

- **Mixer.** `live_mixer.rs` becomes the module `mixer`, with its `main`
  removed. The adapter calls `build` and stops the engine's own clock with
  `Command::Run(false)`, as the headless session does (`live_mixer.rs:599`).
  It then advances one `Command::Tick` per frame and waits, bounded, until
  every meter shows the tick.
- **Diagram editor.** `diagram_editor/` becomes the module `diagram`, with
  its root-relative `crate::` paths rewritten to `crate::diagram::`. The
  adapter calls `build(ui, dir)` with a fresh temporary directory.
- **Gallery zoom workload.** An adapter appended to the copied
  `pages/layout/transforms.rs` builds the canvas of the gallery's
  `--zoom-benchmark` from its private `card`.

**Compilation.** The harness uses only the public API. It counts commands
it does not know through a wildcard arm. Before any result counts, it is
compiled against 0.3.103, the before checkout: 38a9cf9 on `main`, whose
code is that of dc74870. Slice 7 compiles the unchanged harness against the
final checkout.

**Scenarios.** Each one reports:
- composition, cold and warm;
- CPU render;
- the presented `render` call without read-back, paced so that acquisition
  does not wait;
- offscreen render with read-back, median and p95;
- display-list mask resources and KiB, and the masks each frame newly needs.

1. **Gallery pages** at scales 1, 1.25 and 2. This repeats `pages.md` and
   adds 1.25.
2. **Mixer with live data:** 300 ticks, one per frame.
3. **Zoom.**
   - **Diagram editor:** a live zoom, with steps 16 ms apart inside the
     150 ms window, then the settle frame; a settled zoom series; and a pan.
     Its sample document is small, so this shows the application path more
     than the cost.
   - **Gallery zoom workload:** sixty-four cards of four controls in a
     `ZoomView`, driven by the wheel through the phases of
     `--zoom-benchmark` (pan, pan back, zoom in to 800 %, out to 100 % and
     further out to 12.5 %, where every card is in view), each gesture
     followed by its rest frame. The scale each phase ends at is recorded.
4. **Knob.**
   - **Timing:** the gallery's `input` page (Input & parameters), with its
     first knob turned through 200 values. Its readout and the readouts
     sharing its parameter change text coverage too, so this scenario
     reports total display-list masks, labeled as such.
   - **Resource contract:** an isolated knob in a probe-local tree, with
     `show_value(false)` and no other reader of its parameter, turned
     through the same values.
5. **J1 change scenarios** (resize and scale step), as a regression check.
   - The page is the one the before-runs chose as the heaviest. Its key is
     recorded and reused after the change.
   - A page that becomes the heaviest after the change is reported
     separately.

The probe also writes CPU captures of every gallery page at scales 1, 1.25
and 2 for the visual review in slice 6. They are written below `target/` and
not committed.

Output: `phase1-probe/results/before/`, committed with the probe. The run
is `plans/rendering/phase1-probe/run.sh plans/rendering/phase1-probe/results
before=38a9cf9`, five rounds through `quiet-run.sh`, pinned to core 2;
`run.sh` documents its options and output. `summarize.py` writes the
medians over the runs (`results/summary.md`) and, given two labels, applies
the slowdown rule above.

### 1. Preparation and reference coverage (0.3.104, crate-private)

The normative core, reviewed on its own before anything draws with it.

- **New module** `crates/pert-ui-runtime/src/shapes/prepare.rs` with:
  - `FrameMapping`;
  - `PreparedShape`, implementing preparation steps 1–7 in f64 and the
    distance and coverage formulas in f32;
  - `ShapeParameters`, `ShapeCoverage` and `ShapePrepareError`.
- **Interim geometry input.** The input is a crate-private geometry type,
  because the public `Shape` cannot change until slice 2 (it is still the
  enum that the shape cache draws). The module carries
  `#![cfg_attr(not(test), allow(dead_code))]` until slice 2 uses it.
- **Tests:**
  - every bullet of the spec's "Preparation unit tests";
  - coverage at hand-computed pixels for the cases the spec lists for the
    CPU renderer, evaluated directly;
  - seeded invariant loops over the domain: `0 ≤ c_i ≤ c_o ≤ 1` and
    `c_b ≥ 0`, continuity of cut corners while their size passes a pixel
    center, and saturation where the exact-arithmetic rule applies. No new
    dependency is added.
- **No public API change**, so there is no CHANGELOG entry. The API listing
  gate still runs, for the changed dependency requirements.
- **Gate:** the workspace gate and the API listing gate (see Working
  rules).
- **Codex** reviews the code against the spec's normative sections.

### 2. The command, `draw_shape` and the CPU renderer, beside the old path (0.3.105)

The new command arrives without removing anything that draws today. After
this slice the CPU renderer draws `DrawShape`, and nothing the repository
draws by default produces one.

- **Public items:**
  - painter side: the `Shape` struct, which replaces the enum and is
    breaking; `Corner`, `CornerKind`, `CornerPosition`, `ShapeStyle` and
    `Painter::draw_shape`;
  - backend side: `DrawCommand::DrawShape`, `ShapeGeometry`, `ArcGeometry`,
    `TriangleGeometry`, `Paint` and `Border`;
  - preparation: `FrameMapping`, `PreparedShape`, `ShapeParameters`,
    `ShapeCoverage` and `ShapePrepareError`, which become public;
  - errors: `CpuRenderError::Shape`, `CaptureError::Shape` (serde,
    externally tagged), and `RenderError::Shape`. The last is needed in
    both `pert-ui-wgpu`'s `RenderError` and the facade's software-only
    mirror (`platform/presenter.rs`), because software presentation converts
    into whichever of the two the build has;
  - re-exports from the facade.
- **Painter, interim rule:**
  - `draw_shape` records `DrawShape`;
  - `fill_shape`, `stroke_shape` and the focus ring keep recording through
    `ShapeCache` for the silhouettes the old enum could express (all four
    corners round with one radius, the top two round, the top right cut, or
    a rectangle), which is every call site in the repository after the
    mechanical migration below. A `Shape` the old path cannot express
    records `DrawShape`;
  - the old path keeps its own normalization: `rounded_top` clamps to half
    the smaller side there, not by the CSS overlap rule, which is applied
    only when `DrawShape` is prepared. A fill drawn by `draw_shape` under a
    focus ring or `stroke_shape` drawn by the old path can therefore
    disagree for an oversized top radius until slice 5; this is documented
    on the methods and tested (an oversized top radius through both paths,
    and a focused control);
  - `stroke_arc` and `fill_triangle` record masks as before;
  - the interim rule is documented on the methods and leaves in slice 5.
- **Interim switch for evidence:** `Runtime::set_shape_recording`, with a
  `ShapeRecording` of `Legacy` (the default, the rule above) or `Analytic`
  (every shape method, arcs and triangles included, records `DrawShape`, as
  after slice 5). It exists so that renderer tests, the parity corpus and
  the native checks can exercise every geometry through the real painter
  before the switch, because nothing outside the runtime can put a command
  into a `DisplayList`. It is public, documented as interim, and removed in
  slice 5 (decision 4).
- **Composition** (`runtime/compose.rs`) maps `DrawShape` through the
  component's origin and view scale:
  - positions are mapped, and corner sizes, border widths, arc radii and arc
    widths are multiplied by the view scale;
  - clips intersect as for `DrawMask`;
  - empty results are skipped.

  This mapping is what keeps shapes sharp during a live zoom.
- **CPU renderer:**
  - `DrawShape` is drawn through `PreparedShape` with the spec's
    composition;
  - per row, the solid run inside a rectangle (`PreparedShape::solid_run`)
    is filled with one coverage, and only the other pixels evaluate the
    distance;
  - the conversions in `pert-ui-cpu/src/lib.rs`,
    `pert-ui/src/platform/mod.rs` and `platform/presenter.rs` (both
    `presenting` and the capture path) keep the typed cause.
- **wgpu in this slice** refuses `DrawShape` with
  `RenderError::UnsupportedGraphics` until slice 3. Only a consumer's own
  `draw_shape` call or a new corner combination reaches that; the
  repository's widgets, gallery and examples keep drawing on the GPU.
- **Migration:** every `Shape::…` call site in the runtime, the gallery,
  the examples, the tests and the guides (`appearance.md`,
  `custom-widgets.md`) moves to the struct's constructors. The commands
  those calls record do not change, so tests that count `FillRect`s and
  `DrawMask`s stay as they are.
- **New tests:**
  - CPU renderer pixels, hand-computed, at scales 1, 1.25 and 2 (the
    spec's list);
  - composition through `draw_shape`: the alpha at an opaque outer edge
    equals `c_o`;
  - contracts: a frame of only `draw_shape` calls adds no mask resource,
    and with `ShapeRecording::Analytic` neither does one with arcs and
    triangles;
  - the malformed command (a border on an arc or a triangle): no painter
    method can record one, so it is tested where it is decided, in
    `PreparedShape::new`, and each renderer's conversion of
    `ShapePrepareError` into its typed error is unit-tested in that
    renderer's crate. No test claims to have sent such a command through a
    `DisplayList`.
- **Documentation:**
  - `docs/rendering-platform.md`: `DrawShape`'s pixel-center rule and
    whole-pixel border widths, stated for `draw_shape`;
  - `docs/contracts.md`: the new error cause;
  - the CHANGELOG, with the migration table from the spec.
- **Gate:** the full workspace gate and the API listing gate.
- **Codex** reviews the slice.

### 3. The wgpu shape pipeline (0.3.106)

- **Records.** A fixed-size record per prepared shape, holding the span,
  the kind, the parameters and both paints. The layout is documented next
  to `ShapeParameters`.
- **Pipeline.**
  - A storage buffer of records. The vertex shader pulls a record by
    instance index and emits its integer span as a quad.
  - A WGSL fragment shader evaluates the spec's formulas and composition in
    f32.
  - Consecutive shapes form one instanced draw, and the existing batch
    boundaries keep draw order.
  - Core wgpu features only.
- **Retention.**
  - `retained.rs` keeps encoded records per command, as it does for quads,
    and uploads only changed ranges.
  - Records depend on the target's physical size, unlike the normalized
    quads. The full `FrameMapping`, frame extent included, is therefore part
    of the reuse key. It applies both to the per-command reuse in
    `RetainedGeometry::prepare` (`retained.rs:25`) and to the whole-scene
    skip in `prepare_rasterized` (`lib.rs:789`).
  - `Renderer::resize` (`lib.rs:495`) changes the extent without
    invalidating either, so after a resize an equal scene is prepared again.
    An equal scene at an unchanged extent still encodes nothing.
- **Limits:** `LimitError::ShapeRecords { bytes, max }`, checked against the
  device's `max_storage_buffer_binding_size` before any allocation or
  upload. Preparation errors return the `RenderError::Shape` that slice 2
  added.
- **A parity scene for the native checks:** a component that draws the
  corpus (rectangles through `draw_shape`, arcs through `stroke_arc` and
  triangles through `fill_triangle`) on a runtime set to
  `ShapeRecording::Analytic`, since built-in drawing does not record
  `DrawShape` yet.
- **Tests without a GPU:**
  - record encoding;
  - retention, where an unchanged command is not re-encoded;
  - an unchanged scene rendered at different physical extents, including
    frame sizes that round, is re-encoded with records for each extent;
  - the limit check.
- **Parity corpus.** `cpu_parity.rs` gains the spec's shape cases at
  arbitrary sub-pixel phases, classified by tier from `ShapeCoverage`'s reference distances and
  exact-arithmetic flag (decision 3). Its
  run is slice 4.
- **Gate:** the full workspace gate, the API listing gate, and a native
  smoke run of the parity scene and the gallery on this machine with the
  platform recorded.
- **Codex** reviews the slice.

### 4. Parity run: N and δ, and the go/no-go (0.3.107)

The phase's largest technical risk, f32 differences between the CPU
reference and GPUs, is measured here, before anything that draws today is
removed.

- **Run.** `cargo run --locked -p pert-ui-native-checks` runs the parity
  corpus on each available GPU, with the platform recorded.
- **Results.**
  - `N` becomes the largest edge-tier difference measured, at most 4. If the
    measurement exceeds 4, the implementation is corrected, not the tier.
  - `δ` is confirmed or changed, including the cases at the domain's edges.
- **Go/no-go.** If `N ≤ 4` cannot be reached by correcting the shader or
  the preparation, work stops here and the maintainer decides: a narrower
  domain, a different ramp, or leaving `DrawShape` as an opt-in beside the
  old path. Until slice 5, every one of those is still open at no cost to
  existing drawing.
- **Documentation.** The tiers go into `docs/rendering-platform.md` and the
  module documentation of `cpu_parity.rs`.
- **Missing evidence.** GPUs and platforms that were not available, which
  includes Windows and macOS, are recorded as missing evidence.
- **Gate:** the workspace gate and the API listing gate.

### 5. The switch: every shape is `DrawShape`, and the cache goes (0.3.108)

- **Painter:**
  - `fill_shape`, `stroke_shape`, the focus ring, `stroke_arc` and
    `fill_triangle` record `DrawShape`, with their documented rules (zero
    sweep, collinear points, zero width); the interim rule of slice 2
    leaves, and `Runtime::set_shape_recording` and `ShapeRecording` are
    removed;
  - `ShapeCache`, the corner decomposition and the arc and triangle masks
    are deleted;
  - the error rustdoc drops `CoverageTooLarge` for shapes;
  - RENDER-01 is resolved and RENDER-03 fixed, and both TODO entries are
    removed.
- **Test migration:** tests that counted the `FillRect`s and `DrawMask`s of
  a shape now assert `DrawShape`.
- **What changes on screen:** this is the slice where built-in drawing
  takes the spec's behavior changes: snapped edges, whole-pixel borders,
  the distance ramp, the overlap rule and sharp shapes during a zoom. The
  slice changes no widget's drawing calls, so its review separates three
  things: the painter change, those rendering changes, and the mechanical
  update of test assertions.
- **New tests:** a knob turned through many values adds no mask or image
  resource.
- **Documentation:**
  - `docs/rendering-platform.md`: shape edges follow the pixel-center rule,
    border widths are whole physical pixels, and the zoom section and its
    cap change;
  - the CHANGELOG: the behavior changes and the retired cache.
- **Gate:** the full workspace gate, the API listing gate, and a native
  smoke run of the gallery with the platform recorded.
- **Codex** reviews the slice.

### 6. Built-in widgets on `draw_shape`, joins and zoom (0.3.109)

- **Migration to `draw_shape`.** Built-in widgets that fill and stroke one
  shape in the same `paint` pass move to `draw_shape`. Widgets that stroke in
  `paint_foreground` keep the pair, as the spec says (`containers.rs`,
  `views.rs`, `toolbar.rs`, `windows.rs`).
- **Join tests** at scales 1, 1.25 and 2:
  - tabs;
  - window title bars, where `rounded_top` meets the body or a divider;
  - split container frames.
- **Zoom tests:**
  - a retained live zoom, and magnification beyond 8×;
  - captures under those transforms, checked for crop and for frame and
    state correlation.
- **Visual review.**
  - The probe's captures are taken again and compared with slice 0's.
  - The differences are summarized for the maintainer: whole-pixel borders,
    the distance ramp on corners, arcs and small triangles, and the larger
    possible `rounded_top`.
  - Native wgpu captures are taken on this machine.
  - The maintainer looks at the result before slice 7.
- **Gate and Codex** as for slice 5.

### 7. Measurements after, and closing

- **Measurements.**
  - The slice 0 probe runs, unchanged, against the before checkout and the
    final commit in alternating rounds on the same machine; results go to
    `phase1-probe/results/`.
  - `summarize.py`'s comparison against the expectations table goes into
    this plan.
  - A regression is taken to the maintainer.
- **The spec's normative parts** are checked to be in rustdoc and in
  `docs/rendering-platform.md`; then the spec is deleted, as it says.
- **The pre-merge gate** from the Working rules: every step of the CI
  `headless` job beyond the workspace gate, and the MSRV build, on the final
  commit.
- **A final Codex review** covers the whole branch diff against the spec.
- **Merge.** After the maintainer approves, the branch is squash-merged
  into `main`. Pushing waits for the maintainer.

## Risks

- **GPU arithmetic.** f32 differences between GPUs could exceed `N = 4` at
  edges. Slice 4 measures it before the old path is removed, and its
  go/no-go names the ways out.
- **Two paths for three slices.** From slice 2 to slice 5 the cache path and
  `DrawShape` coexist, with an interim rule in the painter. The rule is
  small and documented, and slice 5 removes it.
- **Test migration.** Many runtime and facade tests assert on `FillRect`
  and `DrawMask` output. Thirty-eight facade test files mention `DrawMask`;
  how many of them depend on shapes is found in slice 5. That slice changes
  no widget's drawing calls, so the assertion updates can be reviewed apart
  from its rendering changes.
- **Visible changes.** The changes the spec lists (whole-pixel borders, the
  corner ramp, `rounded_top`) may look different enough to need a
  maintainer decision. Slice 6's review is where that is caught. The spec
  gives thin triangles no rule like the thin-stroke rule for arcs; small
  arrow tips are looked at there in particular.
- **Fragment cost is not measured.** No probe number is GPU execution time
  (see the limits above).
- **Measurement noise.** Earlier runs were taken under load averages of
  11–13. The quiet-machine rule applies, and slices 0 and 7 use the same
  machine.
- **Storage buffer limits.** `max_storage_buffer_binding_size` is at least
  128 MiB under wgpu's default limits, which far exceeds a UI frame's
  records. The limit error exists for correctness, not because the limit is
  expected to be reached.

## Decisions

The maintainer decided on 2026-11-07, each as recommended:

1. **Branch:** `rendering-phase0` (the phase 0 report, the spec and the
   RESIZE fixes in 0.3.103) is squash-merged into `main` first, and
   `rendering-phase1` starts from `main`. The resize gains do not wait for
   phase 1.
2. **The gap between slices 2 and 3 is accepted.** From slice 2 until
   slice 3, wgpu refuses `DrawShape` with a typed error. The branch is not
   merged or used for native evidence in between, and slice 3 follows
   directly. Since the 2026-11-11 revision the gap only affects a
   consumer's own `draw_shape` call: built-in drawing keeps the old path
   until slice 5.
3. **Reference distances.** `ShapeCoverage` gains `outer_distance`,
   `inner_distance` and `exact_arithmetic`, the values the parity tiers
   classify on, so no copy of the formulas is needed. They arrive with
   `ShapeCoverage` in slice 1 and become public in slice 2.
   - The exact-arithmetic tier depends on corner regions and cut terms, not
     on distances alone, so it needs the flag as well as the two distances.

On 2026-11-11, after the review of the revised order:

4. **Interim switch.** `Runtime::set_shape_recording` with `ShapeRecording`
   (`Legacy`, the default, or `Analytic`) is added in slice 2 and removed in
   slice 5, so that tests, the parity corpus and the native checks can
   record every geometry as `DrawShape` before the switch.
5. **Gates.** The dependency row's additional checks (packaging, the MSRV
   build, every feature configuration) run once, on the final commit, not in
   every slice. Slices run the gates the Working rules list.

6. **Go.** Slice 4's parity run measured an edge tier of 1, against the
   limit of 4, so the switch in slice 5 goes ahead. The maintainer chose
   this on 2026-11-11 without waiting for another adapter; other GPUs,
   Windows and macOS remain missing evidence.

## Revision 2026-11-11

After slice 0 the maintainer asked for an evaluation of the rendering plans
and approved its four recommendations:

1. **Slice order.** The command, `draw_shape` and the CPU renderer arrive
   beside the old path (slice 2), then the wgpu pipeline (3) and the parity
   run with a go/no-go (4). Only then does every shape become `DrawShape`
   and the cache go (5), followed by the widgets (6) and the closing
   measurements (7). Before, the public switch and the test migration came
   in slice 2, the branch could not draw shapes on the GPU until slice 3,
   and parity was measured last.
2. **Probe.** The gallery's zoom workload and the presented `render` call
   without read-back were added, and the before-measurement was run again.
   The diagram editor's sample is too small to show zoom cost, and the
   offscreen number is dominated by the read-back.
3. **Slowdown rule.** A threshold of 5 % and 0.05 ms, because the rounds
   differ by 1–2 %.
4. **The top-level plan** was brought up to date as a roadmap.

Codex reviewed the revised order the same day; see the review record.

## Review record

**2026-11-06, Codex review of the draft** (read-only, static). One blocker,
four major and four minor findings. The author checked the error, retention
and CI claims against the code, and all findings held. This revision
answers them:

- **`RenderError::Shape` was deferred to slice 3** (blocker), but slice 2's
  presentation conversion needs it, in both the wgpu error and the facade's
  software-only mirror. It moved to slice 2.
- **Example entry points** (major): `build` is private in the mixer and
  takes a directory in the diagram editor. The probe now specifies
  scratch-only adapters, deterministic mixer ticks, its module rewriting,
  and compilation against both checkouts.
- **Retention key** (major): records depend on the frame extent, which the
  current reuse keys and the whole-scene skip ignore. The full mapping is
  now part of both, with a test across extents.
- **Gates** (major): slices 2–4 now run the CI headless job's feature
  configurations and release tests, and packaging and the MSRV check run in
  slice 6.
- **Knob measurement** (major): the gallery knob changes text coverage too.
  An isolated knob without a readout now carries the resource contract.
  Display-list counts are labeled as such, and upload bytes are not claimed.
- **Minor:**
  - the J1 page is fixed from the before-runs;
  - before/after runs are repeated and paired, with a stated regression
    rule;
  - the parity alternative reproduces the f32 reference;
  - the GPU number is called offscreen render with read-back, end to end.

**2026-11-07, focused Codex recheck** of that revision and of the
maintainer's decisions. It found no blocker, confirmed eight of the nine
findings as resolved, and found two remaining issues. Both held and are
answered:

- **Gates** (major, partly resolved before): the headless gallery and
  `loom` steps were not scheduled, and slices 1 and 5 bumped versions
  without the API listing gate. Every code slice now runs the listing gate,
  and slice 6 runs every step of the `headless` job beyond the workspace
  gate.
- **Coverage outside the span** (minor, spec): the new accessors had no
  defined values outside `span()`, including pixels the clip removes. The
  spec now defines them there as an untouched pixel that keeps its
  destination's tier, and states the classification rule.

**2026-11-11, Codex review of the revised order** (codex-cli, read-only,
static, commit c46d45f). One blocker, one major and four minor findings.
The author checked each against the code and all held:

- **Parity before the switch** (blocker): `draw_shape` takes only
  rectangles, arcs and triangles stayed masks until slice 5, and nothing
  outside the runtime can insert a command, so slice 4 could not cover arcs,
  triangles or a malformed command. Slice 2 now adds the interim
  `Runtime::set_shape_recording`, removed in slice 5, and tests the malformed
  command where it is decided. The maintainer approved the switch
  (decision 4).
- **Gates** (major): consolidating the dependency row to the final commit
  departs from AGENTS.md. The maintainer approved it as an exception for
  this phase (decision 5).
- **Legacy clamp** (minor): the old path clamps `rounded_top` to half the
  smaller side, `DrawShape` uses the overlap rule. Slice 2 keeps the old
  normalization on the old path, documents the difference and tests it.
- **Cards workload** (minor): the probe left out the benchmark's pan back
  and its zoom out to 12.5 %. Both were added, the end scales are recorded,
  and the before-measurement was run again.
- **Summarizer** (minor): it judged incomplete evidence. It now refuses
  repeated or missing rows, values that are not finite, and a comparison
  with fewer than five runs or unequal counts.
- **Stale statements** (minor): slice 5 does change rendering; the plan now
  says what is mechanical there and what is not. The earlier review records
  above keep the slice numbers of their time (before the reorder).

Not examined by the reviewer: builds, tests, native checks, benchmarks,
captures and the coverage mathematics.

**2026-11-11, Codex review of slice 1** (read-only, static, the working
tree's `shapes/prepare.rs` and its tests against the spec's normative
sections). One major and three minor findings; all held and are answered:

- **Domain of derived points** (major): the ±2^16 check covered the arc's
  parameters but not what an evaluation forms from them, such as a rotated
  pixel offset. Preparation now bounds, per shape, the distance from each
  parameter to the span's farthest pixel, and for an arc that distance plus
  the radius; tests accept and refuse on either side of the limit.
- **Narrowing tests** (minor): the collapse test used an edge that was
  short already. Tests now build vertices that coincide, and that become
  collinear, only after narrowing, and check the 1/256 threshold exactly.
- **Relocation tests** (minor): they could not detect an inset rebuilt from
  relocated edges. A test now compares every span pixel with the
  unrelocated boundaries, with borders partly in view, each corner kind and
  a wide border whose inner edges are relocated on their own.
- **Thin caps** (minor): the assertion sampled the sector, not the cap. It
  now samples a pixel before the start with fractional cap coverage.

The reviewer found mapping, snapping, the overlap rule, the inner offsets,
relocation, the distances and the exact-arithmetic flag consistent with the
spec. Not examined: running tests, renderer integration and GPU parity.

**2026-11-11, Codex review of slice 2** (read-only, static, the working
tree against 84d6afb). Three major and two minor findings; all held and are
answered:

- **Solid runs** (major): the CPU renderer evaluated the distance for every
  pixel and only reused the blend. `PreparedShape::solid_run` now gives, per
  row, the columns of a rectangle that are solid and uniform, and the
  renderer fills them with one coverage; a test checks every run pixel.
- **API listings** (major): regenerated with the slice.
- **Verification** (major): added CPU pixels for cut corners, a corner
  below one pixel, a single corner past half and the overlap rule; arcs and
  triangles at scale 2; a component under a `ZoomView` whose composed shape
  is checked before and after a magnification; foreground drawing under
  both recordings; and the typed-cause conversions in `pert-ui-wgpu` and
  the facade. Not added: recording changes in a second window and in
  surfaces, which take the same composition path as the tested ones.
- **`DrawShape`** (minor) is now `#[non_exhaustive]` as a variant.
- **Prepared values** (minor): `PreparedCorner` and `Boundary` have private
  fields and getters, documented as renderer-facing output in physical
  pixels.

The reviewer found no changed output at a migrated call site, no stale
cache, no wrong blend-cache key and no dropped `DrawShape`; the conversions
the spec lists keep the cause. `AppError` still turns it into text, which is
recorded in TODO.md. Not examined: compilation, tests, rustdoc, native
rendering and performance.

**2026-11-11, Codex review of slice 3** (read-only, static, the working
tree against 09b6ee1). Four major and three minor findings; all held and are
answered:

- **Stale records after a failed preparation** (major): records were
  uploaded before the fallible texture preparation, while the encoding the
  next upload is compared with changed only on success. The upload now
  follows every fallible step, so the buffer and that encoding change
  together. No test reproduces the sequence, which needs a device.
- **Limit before allocation** (major): preparation takes the record budget
  and refuses a record before storing it; a test goes through preparation.
- **Parity corpus** (major): added a fixture whose corner, arc and triangle
  have parameters of 16 000 to 65 000 pixels, with a GPU-free test that
  they do, and fixtures for the border rounding thresholds, arcs approaching
  zero width and sweep, and nearly degenerate triangles.
- **API listings** (major): regenerated with the slice.
- **Damage probe** (minor): its pass now draws shape-only scenes.
- **Whole-scene skip** (minor): the decision is a function of its own with
  a test across extents.
- **Byte order** (minor): records are encoded little-endian, as WGSL lays
  host-shared values out.

The reviewer found no difference between the shader and the CPU reference
in formulas, operation order, corner indexing, flags, composition or
blending, and the record layout matches. Not examined: builds, tests,
native rendering and the backends' own implementations.

**2026-11-11, parity on this machine** (Fedora 44, Intel UHD Graphics 770,
Mesa, Vulkan), after the fixes: exact tier 0, tolerance tier 1, and the edge
tier's largest difference 1 over every shape fixture, the domain's edge
included. The gallery's interaction check passed on the GPU (39 steps).
Other GPUs, Windows and macOS were not available.

**2026-11-11, slice 4.** `N = 1`: the edge tier is one step per channel,
the largest difference over the corpus on the one adapter available (Intel
UHD Graphics 770, Mesa, Vulkan), and `δ = 1/16` pixel is confirmed there: no
decided pixel left its tier, the fixture near the domain's edge included.
The tiers are in `docs/rendering-platform.md`, the module documentation of
`cpu_parity.rs` and `docs/verification.md`. The check now fails above one
step, with a compile-time limit of four on any later raise. Missing
evidence: every other GPU, Windows and macOS.

**2026-11-12, slice 5.** Every painter shape method records one
`DrawShape`; `ShapeCache`, the corner decomposition, the arc and triangle
masks, `ShapeRecording` and `Runtime::set_shape_recording` are gone, and
RENDER-01 and RENDER-03 left `TODO.md`. A fill whose corners all have size
zero stays a `FillRect`, which widens the plan's "`Shape::RECTANGLE` fill"
to a radius metric of zero. `stroke_arc` and `fill_triangle` keep their
paint-time check that the bounds are representable. Test migration: the
facade tests that looked for a shape's `FillRect`s and `DrawMask`s read
`DrawShape` through shared helpers in `tests/support` (colors, extent),
and the tests that sampled mask pixels evaluate `PreparedShape` instead;
one test was renamed where its name described mask rasterization. No
expectation needed a moved rectangle or a changed size. New tests: a knob
through 200 values adds no mask or image, and a shape under a view
transform is mapped at 16× and during a live zoom without being painted
again.

**2026-11-12, Codex review of slice 5** (read-only, static, on the working
tree against b3557ca). Two major and four minor findings; all held:

- **Collinear triangles** (major): `fill_triangle` checked the bounds before
  the collinearity, so collinear points with unrepresentable bounds became
  an error. The collinearity check is first again, with a test of both
  cases.
- **A square fill and a square shape under a clip** (major): a clipped
  `FillRect` rounds its intersected edge in `f32`, so it can differ from a
  `DrawShape` with square corners by a pixel column at a half-pixel edge.
  This is `fill_rect`'s behavior from before the phase, and the zero-corner
  fill went through it before too, so nothing moved; the rustdoc no longer
  says the two cover the same pixels. Not changed here.
- **Weakened assertions** (minor): the toolbar and popup border tests match
  a border-only shape of the theme width again; the gallery's focus-ring
  test and its live workload guard read shapes; the theme-change test
  compares geometry, border width and clip.
- **Found through the workload guard:** composition kept a shape whose
  bounds lie outside a non-empty clip, where it drops a rectangle or a mask,
  so a scrolled-out control cost a command. Composition now drops it, with
  one physical pixel of margin for an arc's or a triangle's ramp, and a
  test.
- **Documentation** (minor): the contract and the CHANGELOG separate
  rectangle snapping from arcs and triangles, say that two calls are two
  silhouettes, and bound the zoom statement by the evaluation range and by
  what was checked (recorded commands, not captures).
- **CHANGELOG** (minor): the interim API's removal is named.

Not examined by the reviewer: builds, tests, captures and native behavior.
**2026-11-12, native run for slice 5** (Fedora 44, GNOME/Wayland, Intel UHD
Graphics 770, Mesa 26.2.6, Vulkan). The whole `pert-ui-native-checks` suite
passed except the gallery's GPU interaction check, which failed with a
clock error; it did so at b3557ca too, so the switch is not its cause. Its
CPU twin passed 39 steps. The cause was in the check: five steps clicked
through `testing::Driver`, whose actions advance the runtime's clock one
test frame each, and the window loop's next real-time flush could then be
earlier than the runtime's clock. The steps now click without moving the
clock, and after slice 5's commit the GPU check passed its 39 steps in three
runs by itself, so the gallery in a real GPU window draws its shapes through
`DrawShape` with every step passing. The readback harness
needed two repairs that the switch exposed: it prepared scenes for the
window's surface while reading an 8×8 attachment, which stretched
rectangles and masks but misplaces shapes, whose records are in frame
pixels; scenes are now prepared for the attachment, and the cut-corner
scene is drawn at the attachment's own extent.

### Slice 6 record (2026-11-12, 0.3.109)

**Migrated to `draw_shape`:** the shared control frame (Button and the
Select and MultiSelect anchors; a button's focus ring joins its fill), the
field frame, the toast, the detached tab slot, the chip and the link, and
the gallery's brand mark and the diagram editor's box node. Pairs with
content between the fill and the stroke, and the foreground frames, are
unchanged. `pert_ui::component` exports `ShapeStyle`, `Corner`,
`CornerKind` and `CornerPosition`, which a custom widget needs for
`draw_shape`; the plan did not name this.

**Tests** (`crates/pert-ui/tests/presentation/shape_joins.rs`, CPU pixels at
scales 1, 1.25 and 2): a resting tab against the strip divider (the selected
tab's indicator covers its own join), a floating window's title bar against
its body and frame, a container's fill and foreground frame around painted
content, a one-command frame without fill beside the border's curve, and a
capture under a 16× view and a live-zoom step, compared pixel for pixel
with the full frame and with the tile drawn directly at its displayed size,
with no new paint of the tile in the live step.

**Visual review.** The probe built from the slice 6 tree wrote CPU captures
of every gallery page at the three scales, compared with slice 0's
(`target/phase1-probe/captures/{before,after}`, not committed):

- Under 0.4 % of a page's pixels differ, on every page.
- At scales 1 and 2 the differences are corner ramps, of at most 18 steps
  per channel on most pages. Slider thumbs (the live, families and toolbar
  pages) differ by up to 102: their ends are rounder, since a radius of
  half the height is no longer reduced, and checkbox corners likewise.
- The only built-in triangles are resize grips. The windows and
  tool-windows pages differ by at most 17 steps at scales 1 and 2, so no
  grip changed visibly; a grip was not looked at by itself.
- At scale 1.25 borders differ by up to 160 steps: a border that was two
  pixels on one side and one on the other is now one pixel on every side,
  and a 2-unit checkbox stroke is the same width all round. This is the
  whole-pixel border rule.
- No page shows a missing or displaced element.

**Native wgpu captures** (this machine: Intel UHD Graphics 770, Mesa
26.2.6, Vulkan) were written by a scratch copy of the probe that saves
`Renderer::render_offscreen` beside each CPU capture. On the pages that
differ by more than one step, the differences sampled are rectangles
(`FillRect`) with an edge at a pixel center at scale 1.25, such as a
one-unit rule at y = 257.2: wgpu covers one row more than the CPU renderer.
That is the snapped-edge exception the rendering contract documents for
rectangles, and it predates phase 1. Not every differing pixel was traced
to its command.

**Gate:** the workspace gate, the API listing, release tests and the
feature configurations passed on the final tree; the workspace tests were
run again by themselves after a stopped earlier run's listing step removed
a file under them. The gallery's native interaction check passed its steps
on GPU and CPU. The whole native suite was not rerun.

**Codex review** (static, no builds): no blocker.
- **Button** (major): its fill and focus ring were still two adjacent
  calls. Merged in `control_frame`, with a test.
- **Zoom test** (major): it showed neither retention nor curved edges. It
  now counts the tile's paints and compares the frame with a direct
  drawing.
- **Container test** (minor): renamed, since two commands with one geometry
  are still composited twice; the content now paints, so the order is
  checked.
- **Documentation** (minor): the contract and the CHANGELOG name adjacent
  fills and outlines only.

Not examined by the reviewer: builds, tests, captures and native behavior.

### Slice 7 record (2026-11-12, 0.3.110)

**First after-measurement (35ebfc6, 0.3.109), five rounds each.**
Composition, presentation to the window and the mask counts were as
expected, but the renderers were slower: CPU render time in 122 of 136
rows, with a median ratio of 1.34 and at most 1.61, and the offscreen wgpu
render with read-back by up to 2.07 times in the change series (J1 scale
change 9.3 → 19.4 ms). The maintainer chose a repair before the merge.

**Cause and repair (0.3.110).** Both renderers evaluated every pixel of a
rectangle's span, including the inside of a frame without a fill. A scratch
build whose fragment stage returned the fill without evaluating removed the
wgpu difference, which confirmed it there.

- The CPU renderer fills each row's solid run as one run: skipped when its
  source adds nothing, written without reading when it is opaque, and
  blended with a memo otherwise. `FillRect` uses the same run fill.
- `PreparedShape::solid_box` names a box of rows clear of every corner. The
  wgpu renderer draws it from a record of its own that the fragment stage
  paints without a distance, leaves it out when it draws nothing, and
  evaluates only the strips around it. A rectangle has up to five records.
- Pixels are unchanged: the CPU/GPU parity check, the pixel read-back, the
  capture checks and the gallery interaction checks passed on this machine
  (Intel UHD Graphics 770, Mesa 26.2.6, Vulkan), and a unit test compares
  the run fill byte for byte with the per-pixel blend.

**Second after-measurement** (the 0.3.110 working tree, recorded in
`results/checkouts.txt`; five rounds each, load below 1 during the runs;
`results/summary.md` has every row):

| Expectation | Result |
|---|---|
| Shapes add no mask | Met: mask resources fall where shapes were masks (J1 page 55 → 50), and bytes do not rise. |
| Turning a knob adds no mask | Met: the isolated knob composes a value in 0.028 ms instead of 0.259 ms. |
| A zoom costs no more | Met for composition and presentation. |
| The mixer's live updates cost no more | Composition and presentation met. CPU render is 0.94 → 1.06 ms per frame, slower by 0.11 ms. |
| The CPU renderer is not materially slower | Met: faster in 132 of 136 rows, median ratio 0.72. Slower in two: the mixer above and one rest frame of the zoom workload (0.79 → 0.90 ms). |
| The GPU path costs no more UI-thread time | Met for the `render` call to the window by its medians (ratio 0.96 warm, 0.94 cold). One tail is slower: the 95th percentile of the diagram's pan, 0.88 → 0.94 ms. The offscreen render with read-back is slower: median ratio 1.07 on repeated page renders (at most 1.22), 1.15 on the first, and up to 1.22 in the series (knob on the input page 3.5 → 4.3 ms, J1 scale change 9.3 → 11.3 ms). |
| Resize and scale changes keep the gains | Composition and presentation met; the offscreen render as above. |

The cause of the remaining offscreen difference was not found. It may not
be the distance evaluation alone: in a smoke run the scratch build without
it was still slower than before phase 1 on one of the three pages tried
(2.7 → 3.0 ms); the shape pipeline reads its record from a storage buffer in
the fragment stage, which is a candidate, not a finding.

**Specification.** Its normative parts are in `PreparedShape`'s rustdoc and
`docs/rendering-platform.md`, and `phase1-shapes-spec.md` is deleted.

**Review.** Codex reviewed the repair and found no blocker. Its points were
taken: the box's paint is computed in the fragment stage as before, a test
covers commands whose record count changes ahead of retained ones, a test
compares the run fill with the blend, and the wording about what is skipped
was narrowed.

**Gate:** the workspace gate, the API listing, release tests and the
feature configurations passed on the repaired tree.

**Final review of the branch (Codex, at bc05bd6): one blocker, repaired in
0.3.111.** Composition dropped an arc by its logical bounds, but an arc's
center maps by the frame's pixels per logical unit and its radius by the
scale, which differ when the logical size is not a whole number of pixels:
in a 64.4-unit viewport an arc centered 1000 units outside a clip was
dropped although it covered pixels inside. The reach is now taken in the
frame's terms, with a test of that case. The review's other points were
taken: the slower 95th percentile above, `Boundary::edges` on moved edges,
the wgpu renderer's rustdoc on records, and two stale statements in the
rendering contract and the appearance guide.

**Pre-merge gate:** every step of CI's `headless` job, with the loom
models, each feature configuration's Clippy, tests and rustdoc, the
headless gallery and packaging, and the diagnostics and API listing jobs'
commands, passed on bc05bd6 and again on the repaired tree. The pinned
toolchain is the MSRV.

Codex rechecked the repair (b28cd8a): no blocker remains. Its wording
points were taken, and a triangle's allowance under a saturated frame
extent, which predates the repair, is in TODO.md.
