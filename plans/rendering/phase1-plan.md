# Phase 1 plan: analytic shapes

Status: **decided, not started.** Written 2026-11-06 on branch
`rendering-phase0` at dc74870 (0.3.103). The maintainer answered its
questions on 2026-11-07 (see [Decisions](#decisions)) and asked that
implementation not start yet. It orders the work and the measurements for
the approved [phase 1 spec](phase1-shapes-spec.md). The one addition to the
spec, the reference distances (decision 3), is recorded in the spec's own
Decisions section.

## Goal

Implement the spec: one `DrawShape` command for rounded and cut rectangles,
arcs and triangles, prepared once by shared code and evaluated per pixel by
both renderers. The shape cache and its coverage masks go away, and so do the
corner decomposition and the arc and triangle masks.

What the phase should achieve, each part measured in slice 0 and again in
slice 6:

| Expectation | Measure |
|---|---|
| Shapes add no mask resource and no mask bytes | Mask resources and KiB in the composed display list, per gallery page at scales 1, 1.25 and 2 |
| Turning a knob adds no mask resource | Display-list mask resources added per value by an isolated knob with no readout text; composition time per value on the gallery's controls page |
| A zoom, live or settled, prepares no shape coverage | Composition time per zoom frame in the diagram editor, and the display-list masks each frame newly needs |
| The mixer's live updates cost no more | Composition, CPU render, and offscreen render with read-back, per frame with level updates |
| The CPU renderer is not materially slower | CPU render time per page and scenario |
| The GPU path is not slower end to end | Offscreen render with read-back, warm |
| Resize and scale changes keep the 0.3.103 gains | The J1 change scenarios on the page chosen before the change |

Two limits of these measures:

- **Masks are counted in the display list.** The probe counts them in the
  composed display list. It measures neither the rasterization done inside
  the renderers nor the bytes they upload, so the plan claims neither.
- **The GPU number is end to end.** It includes preparation, allocation,
  synchronization and read-back, which dominates (about 3 ms for an empty
  scene in phase 0). It cannot show the cost of GPU execution alone.

The CPU renderer's direction is not known in advance. It moves from sampling
a mask to evaluating a distance at edge pixels, with solid runs inside.

How a slowdown is judged:

- Slices 0 and 6 each build both checkouts and run them in five
  alternating, paired runs on the same machine. All samples are kept.
- A scenario counts as slower when the median of its five after-run
  medians exceeds the largest before-run median.
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
  - **Every code slice, 1 to 5:** the workspace gate and the API listing
    gate. The listing gate runs even where no public item changes, because a
    dependency requirement does.
  - **Slices 2–4, in addition:** release-profile tests and the feature
    configurations of `pert-ui` and `pert-ui-core` that the `headless` job
    in `.github/workflows/ci.yml` runs: no default features, `serde`, `cpu`,
    `desktop-cpu` and the image features. These slices change public API,
    errors and presentation across packages. The configurations cover
    `CaptureError`'s serde form and the separate software-only
    `RenderError`.
  - **Before merge, in slice 6:** every step the `headless` job runs beyond
    the workspace gate, among them the release tests, the `loom` models, the
    feature configurations, the headless gallery and library packaging. Then
    the MSRV build. These run on the final commit, after the last version
    bump, and cover all of the phase's dependency changes. The other checks
    above are not moved to slice 6; each slice still runs them.
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

**Compilation.** The harness uses only the public API. It counts commands
it does not know through a wildcard arm. Before any result counts, it is
compiled against 0.3.103 (dc74870), the before checkout. Slice 6
compiles the unchanged harness against the final checkout.

**Scenarios.** Each one reports:
- composition, cold and warm;
- CPU render;
- offscreen render with read-back, warm median and p95;
- display-list mask resources and KiB, and the masks each frame newly needs.

1. **Gallery pages** at scales 1, 1.25 and 2. This repeats `pages.md` and
   adds 1.25.
2. **Mixer with live data:** 300 ticks, one per frame.
3. **Diagram editor zoom:** a live zoom, with steps 16 ms apart inside the
   150 ms window, then the settle frame; a settled zoom series; and a pan.
4. **Knob.**
   - **Timing:** the gallery's controls page, with its first knob turned
     through 200 values. Its readout and the readouts sharing its parameter
     change text coverage too, so this scenario reports total display-list
     masks, labeled as such.
   - **Resource contract:** an isolated knob in a probe-local tree, with
     `show_value(false)` and no other reader of its parameter, turned
     through the same values.
5. **J1 change scenarios** (resize and scale step), as a regression check.
   - The page is the one the before-runs chose as the heaviest. Its key is
     recorded and reused after the change.
   - A page that becomes the heaviest after the change is reported
     separately.

The probe also writes CPU captures of every gallery page at scales 1, 1.25
and 2 for the visual review in slice 4. They are written below `target/` and
not committed.

Output: `phase1-probe/results/before/`, committed with the probe.

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

### 2. The command, the painter and the CPU renderer (0.3.105)

The public switch. After this slice, every shape a `Painter` draws reaches the
display list as `DrawShape`.

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
- **Painter:**
  - `fill_shape`, `stroke_shape`, the focus ring, `stroke_arc` and
    `fill_triangle` record `DrawShape`, with their documented rules (zero
    sweep, collinear points, zero width);
  - `ShapeCache`, the corner decomposition and the arc and triangle masks
    are deleted;
  - the error rustdoc drops `CoverageTooLarge` for shapes;
  - RENDER-01 is resolved and RENDER-03 fixed, and both TODO entries are
    removed.
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
  - per row, solid runs inside the shape are filled as runs;
  - the conversions in `pert-ui-cpu/src/lib.rs`,
    `pert-ui/src/platform/mod.rs` and `platform/presenter.rs` (both
    `presenting` and the capture path) keep the typed cause.
- **wgpu in this slice** refuses `DrawShape` with
  `RenderError::UnsupportedGraphics` until slice 3. Native GPU windows on the branch therefore cannot draw shapes
  between the two commits (decision 2).
- **Migration:**
  - every `Shape::…` call site in the runtime, the gallery, the examples,
    the tests and the guides (`appearance.md`, `custom-widgets.md`);
  - tests that counted the `FillRect`s and `DrawMask`s of a shape now
    assert `DrawShape`.
- **New tests:**
  - CPU renderer pixels, hand-computed, at scales 1, 1.25 and 2 (the
    spec's list);
  - composition through `draw_shape`: the alpha at an opaque outer edge
    equals `c_o`;
  - contracts: a malformed or unknown command is reported, a frame of only
    shapes adds no mask resource, and a knob turned through many values adds
    no mask or image resource.
- **Documentation:**
  - `docs/rendering-platform.md`: shape edges follow the pixel-center rule,
    border widths are whole physical pixels, and the zoom section and its
    cap change;
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
- **Tests without a GPU:**
  - record encoding;
  - retention, where an unchanged command is not re-encoded;
  - an unchanged scene rendered at different physical extents, including
    frame sizes that round, is re-encoded with records for each extent;
  - the limit check.
- **Parity corpus.** `cpu_parity.rs` gains the spec's shape cases at
  arbitrary sub-pixel phases, classified by tier from `ShapeCoverage`'s reference distances and
  exact-arithmetic flag (decision 3). Its
  run is slice 5.
- **Gate:** the full workspace gate, the API listing gate, and a native
  smoke run of the gallery on this machine with the platform recorded.
- **Codex** reviews the slice.

### 4. Built-in widgets on `draw_shape`, joins and zoom (0.3.107)

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
    the distance ramp on corners, and the larger possible `rounded_top`.
  - Native wgpu captures are taken on this machine.
  - The maintainer looks at the result before slice 5.
- **Gate and Codex** as for slice 2.

### 5. Parity run: N and δ (0.3.108)

- **Run.** `cargo run --locked -p pert-ui-native-checks` runs the parity
  corpus on each available GPU, with the platform recorded.
- **Results.**
  - `N` becomes the largest edge-tier difference measured, at most 4. If the
    measurement exceeds 4, the implementation is corrected, not the tier.
  - `δ` is confirmed or changed, including the cases at the domain's edges.
- **Documentation.** The tiers go into `docs/rendering-platform.md` and the
  module documentation of `cpu_parity.rs`.
- **Missing evidence.** GPUs and platforms that were not available, which
  includes Windows and macOS, are recorded as missing evidence.

### 6. Measurements after, and closing

- **Measurements.**
  - The slice 0 probe runs, unchanged, against the final commit on the same
    machine; results go to `phase1-probe/results/after/`.
  - A comparison against the expectations table goes into this plan.
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
  edges. The fix goes into the shader or the preparation, as the spec
  requires, and could delay slice 5.
- **Test migration.** Many runtime and facade tests assert on `FillRect`
  and `DrawMask` output. Thirty-eight facade test files mention `DrawMask`;
  how many of them depend on shapes is found in slice 2. That slice may be
  large, and its review should separate mechanical migration from behavior.
- **Visible changes.** The changes the spec lists (whole-pixel borders, the
  corner ramp, `rounded_top`) may look different enough to need a
  maintainer decision. Slice 4's review is where that is caught.
- **The branch between slices 2 and 3** cannot draw shapes on the GPU
  (decision 2).
- **Measurement noise.** Earlier runs were taken under load averages of
  11–13. The quiet-machine rule applies, and slices 0 and 6 use the same
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
   directly.
3. **Reference distances.** `ShapeCoverage` gains `outer_distance`,
   `inner_distance` and `exact_arithmetic`, the values the parity tiers
   classify on, so no copy of the formulas is needed. They arrive with
   `ShapeCoverage` in slice 1 and become public in slice 2.
   - The exact-arithmetic tier depends on corner regions and cut terms, not
     on distances alone, so it needs the flag as well as the two distances.

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
