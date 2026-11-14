# Stroke prototype (slice 0 of candidate F)

The preparation and the per-pixel coverage of
[stroke-spec.md](../stroke-spec.md), built outside the library, to decide
the [gate](../stroke-spec.md#slice-0-measure-before-building) before any of
it enters `pert-ui`. It is evidence only, like the other probes here.

- `src/stroke.rs`: preparation in `f64` (normalization, joins, support,
  span, segment clipping, tile lists or cells, provisional budgets) and the
  `f32` reference, `segment_distance`.
- `src/stroke.wgsl`: the same formula on the GPU by two paths, each with a
  color and a coverage (R32Float) output:
  - tiles: one instanced quad per recorded tile, each pixel taking the
    smallest distance over the tile's list;
  - quads (the spec's alternative): one oriented quad per segment, with a
    second around a miter join's wedge, writing coverage into an R8 atlas
    with a maximum blend, then the stroke's cells composited from the atlas.
    The maximum of the segments' coverage is the coverage of their smallest
    distance, so the paths differ only by the atlas's 8-bit rounding.
- `src/cpu.rs`: the CPU side, the reference evaluated per tile and blended
  once per pixel.
- `src/gpu.rs`: a headless Vulkan device, the upload path, the atlas (its
  strokes packed in shelves, in batches when they do not fit; kept while
  the strokes do not change) and a model of today's mask path (a texture
  per changing mask, `masks.wgsl` unchanged).
- `src/today.rs`: today's path through the public facade: the painter's
  `DrawVector`, `VectorRasterizer` and `CpuRenderer`.
- `src/workloads.rs`: the timed workloads and the parity corpus.

It depends on this checkout's crates by path, so it measures whatever the
working tree holds; run it at the commit the results name.

## Modes

```sh
export CARGO_TARGET_DIR=target/stroke-probe
cargo run --offline --release --manifest-path plans/rendering/stroke-probe/Cargo.toml -- check
```

- `check`: for every corpus case, GPU coverage against the reference per
  pixel (decided pixels must be equal, within a step on the quads path;
  edge pixels report the largest difference), each tile list against every
  segment, and the support: no pixel outside the span may be covered unless
  the clip removes it. On the quads path the reference is every segment
  inside the span, so a cell the composite misses fails as a differing
  pixel. Fails on any violation.
- `compare`: the reference against what today's CPU renderer draws, as a
  distribution of coverage differences per corpus shape. `PROBE_DEBUG=1`
  lists the largest differences on stderr.
- `time`: per-frame medians and 95th percentiles at scales 1 and 2, in the
  real-time probe's intervals: prepare (preparation, encoding and upload),
  GPU (encode, submit and wait for the frame's offscreen passes), GPU
  execution (the passes' own timestamps, where the adapter has them; on
  the quads path also split into the atlas passes and the composite) and
  CPU, beside today's rasterize, prepare, GPU, GPU execution and CPU, drawn
  in the same frames so that both run at the same GPU clock, which the
  last column reports where the kernel exposes it. A workload today's path
  refuses is reported as refused.

`PROBE_PATH` selects the GPU path (`tiles`, the default, or `quads`),
`PROBE_TILE` the tile or cell size (default 16) and `PROBE_ATLAS` the
atlas's smallest side (default 2048; never smaller than the frame).
`PROBE_NO_TODAY=1` leaves today's path out of `time`, `PROBE_NO_TODAY_CPU=1`
only today's CPU renderer, `PROBE_NO_CPU=1` the prototype's CPU evaluation
(its column then reads zero), and `PROBE_QUICK=1` shortens every mode for a
smoke run. `run.sh RESULTS_DIR` runs all three, the timing pinned and on a
quiet machine.

## Limits of the model

- Today's GPU cost is modeled here, not measured in `pert-ui-wgpu`: its
  texture cache and batching differ. The real-time probe
  (`native_realtime_workload_probe`, verification.md) stays the reference
  for the two probe polylines and for a window present.
- Today's CPU cost is the CPU renderer's frame with the strokes minus an
  empty frame; the prototype's CPU cost is preparation plus evaluation and
  blending into a linear `f32` frame, not the CPU renderer's own buffer.
- A view scale is not recorded through the painter here, so the zoomed
  workload has no baseline.
- The parity check compares coverage in `f32`, not 8-bit colors; the color
  tier is measured when the pipeline enters `pert-ui-wgpu`.
- The GPU's clock follows its load: on the measured Intel adapter it stays
  between 300 and 650 MHz for light frames and reaches 1550 MHz only for
  the heaviest, so a light workload's times can double between runs. Rows
  are comparable within a run, where the new path and today's share the
  clock, not across runs.
- The corpus does not yet hold a miter ratio near the domain; slice 1 tests
  the support bound adversarially.

## Status

Measured on Intel UHD Graphics 770 (Mesa, Vulkan): the tile lists fail the
gate (2026-11-12, [results/summary.md](results/summary.md)); the quads path
(2026-11-13, [results/quads/summary.md](results/quads/summary.md)) fails the
first gate and meets the one the maintainer revised after it, and is the
spec's wgpu path.
