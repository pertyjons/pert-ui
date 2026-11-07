# Where vello_cpu's speed comes from: CPU blend-storage variants (2026-11-05)

Follow-up 2 to [../vello-spike/results/results.md](../vello-spike/results/results.md).
This is a throwaway copy of `crates/pert-ui-cpu` (package `pert-ui-cpu-variants`,
with its own `[workspace]` and a path dependency on `../../../crates/pert-ui-runtime`).
`CpuRenderer::with_variant` selects how the frame stores and blends pixels.
Everything else is the original code: command validation, vector
preparation (`VectorRasterizer`), the clip and pixel-center span, mask and
image sampling (bilinear/nearest, clamp-to-edge, quarter turns), and the
"reuse the last result while the stored pixel repeats" shortcut for fills.

- **(0) current**: unchanged. The frame is 8-bit sRGB-encoded premultiplied.
  Each blend decodes the stored bytes (a lookup table), blends in f32 in
  linear light and encodes again (a binary search over 255 thresholds).
  Variant (0) reproduces `pert_ui::cpu::CpuRenderer` exactly: max difference
  0 on every scene.
- **(a) encoded 8-bit**: the frame is 8-bit sRGB-encoded premultiplied, and
  blends work directly on the encoded bytes.
  - Each command's color is converted to encoded premultiplied once.
  - Each raster image is converted once per content to encoded
    premultiplied bytes and filtered on those bytes, as vello does.
  - A mask scales the encoded color by its coverage.
  - Blending is integer src-over on the bytes (`d' = s + d·(255 − sa)/255`),
    after quantizing the f32 source once on a cache miss.
  - There is no per-pixel decode or encode.
- **(b) linear 16-bit**: the frame is linear-light premultiplied, **16 bits
  per channel (u16)**. Blending is the same f32 src-over as (0) but on
  `u16 / 65535`, and the frame is encoded to sRGB8 once at the end through a
  64 KiB lookup table, per channel and pixel.
- **(c) = (b) + a fast path**: an opaque `FillRect` stores its color across
  its covered rows without reading or blending the destination. This was the
  quick part of the optional "interior span" fast path. Masks and images are
  still sampled per pixel; no span fast path for mask interiors was written.

## Environment and reproduction

- The same machine as the spike: i9-12900 pinned to CPUs 2–7 (P-cores 1–3,
  both hyperthreads), Linux 7.2.15-200.fc44, rustc 1.99.1, release profile.
- vello_cpu 0.3.0, single-threaded, through the spike's translator
  (`../vello-spike/src/translate.rs`): its time is translation plus render.
- Two runs, 18:22–18:26 and 18:26–18:30, both started without waiting.
  - 1-minute load was 1.18 → 1.07 and 1.07 → 1.05.
  - The processes with the most CPU at both starts and ends were
    gnome-terminal-server, gnome-shell, codex, claude and jetbrains-toolbox,
    each below 6% of one CPU by `ps` (a lifetime average). Nothing sampled
    the machine during a run.
  - The two runs agree within 1–2% on every median quoted here, so the
    tables below are from run 1; `results-raw-{1,2}.md` hold both in full.
- Reproduce with `plans/rendering/cpu-blend-variants/run.sh 2`.
  - It builds `bench/bench.rs` through `bench.sh` into a scratch crate at
    `target/cpu-blend-variants/scratch`, next to copies of the gallery modules
    and the spike's `translate.rs`, `compare.rs`, `workloads.rs` and
    `pertui.rs`.
  - It then runs it pinned, after `../tools/quiet-run.sh` has waited for a
    1-minute load below 2.0. That script records the load and the busiest
    processes at the start and end.
- Method: per scene, every renderer renders the same DisplayList once to warm
  up and then 30 times, interleaved (0, a, b, c, vello, 0, …). Times are
  medians in ms.
- The (0) split comes from 10 extra renders through `profile_current`: the
  same code as (0), with timers around clear, vector preparation and each
  command kind.
- **Caveat: vector coverage is cached in this benchmark.** Re-rendering an
  unchanged DisplayList hits the `VectorRasterizer` cache, so the polyline
  rows exclude the per-frame CPU vector rasterization all pert-ui variants
  would pay for a changing line (3–4 ms in the earlier measurements).
  vello_cpu rasterizes the path on every render, so these rows favour the
  pert-ui variants.

## Synthetic workloads (frame 219), run 1

| Scene | Scale | (0) | (a) | (b) | (c) | vello_cpu | (0) split: clear / vectors / fills / masks / images | (0) = pert-ui-cpu |
|---|---|---|---|---|---|---|---|---|
| Empty scene (background only) | 1 | 0.49 | 0.75 | 2.82 | 2.84 | 0.33 | 0.50 / 0.00 / 0.00 / 0.00 / 0.00 | 0 |
| Panels (150 + 100) | 1 | 3.87 | 4.53 | 6.59 | 4.17 | 1.37 | 0.50 / 0.03 / 1.90 / 1.48 / 0.00 | 0 |
| Panels light (20 + 20) | 1 | 2.47 | 3.27 | 5.55 | 3.27 | 0.62 | 0.50 / 0.00 / 1.77 / 0.21 / 0.00 | 0 |
| Polyline 800 | 1 | 8.90 | 9.15 | 9.82 | 8.30 | 0.80 | 0.49 / 0.00 / 1.18 / 7.22 / 0.00 | 0 |
| Polyline 4096 | 1 | 8.99 | 9.20 | 9.88 | 8.35 | 1.24 | 0.49 / 0.00 / 1.18 / 7.31 / 0.00 | 0 |
| Gradient bars | 1 | 10.52 | 10.16 | 12.63 | 10.16 | 1.58 | 0.49 / 0.00 / 1.79 / 0.00 / 8.24 | 0 |
| Empty scene (background only) | 2 | 2.08 | 3.05 | 11.55 | 11.52 | 1.60 | 1.98 / 0.00 / 0.00 / 0.00 / 0.00 | 0 |
| Panels (150 + 100) | 2 | 14.54 | 17.60 | 25.98 | 16.94 | 3.39 | 1.98 / 0.03 / 7.23 / 5.64 / 0.00 | 0 |
| Panels light (20 + 20) | 2 | 9.51 | 13.28 | 22.10 | 13.55 | 2.21 | 1.98 / 0.00 / 7.12 / 0.81 / 0.00 | 0 |
| Polyline 800 | 2 | 34.46 | 36.08 | 38.92 | 33.21 | 2.63 | 1.99 / 0.00 / 4.63 / 28.19 / 0.00 | 0 |
| Polyline 4096 | 2 | 34.79 | 36.23 | 39.04 | 33.41 | 3.07 | 1.98 / 0.01 / 4.64 / 28.41 / 0.00 | 0 |
| Gradient bars | 2 | 42.14 | 40.33 | 50.57 | 41.17 | 5.48 | 1.98 / 0.00 / 7.59 / 0.00 / 32.85 | 0 |
| Scene | Scale | (0) | (a) | (b) | (c) | vello_cpu | (0) split: clear / vectors / fills / masks / images | (0) = pert-ui-cpu |
|---|---|---|---|---|---|---|---|---|
| layout | 1 | 5.76 | 6.48 | 8.33 | 5.43 | 0.69 | 0.49 / 0.01 / 2.21 / 3.03 / 0.01 | 0 |
| splits | 1 | 5.65 | 6.50 | 8.42 | 5.19 | 0.73 | 0.49 / 0.01 / 2.46 / 2.67 / 0.01 | 0 |
| shells | 1 | 6.56 | 7.57 | 9.48 | 5.64 | 0.77 | 0.50 / 0.01 / 2.88 / 3.20 / 0.01 | 0 |
| scroll | 1 | 5.35 | 6.07 | 7.91 | 5.22 | 0.70 | 0.50 / 0.01 / 2.03 / 2.81 / 0.01 | 0 |
| sections | 1 | 6.92 | 7.63 | 9.26 | 6.24 | 0.75 | 0.50 / 0.01 / 2.31 / 4.10 / 0.01 | 0 |

## Gallery pages (39 per scale), run 1

Each cell is min / median / max over the pages of each renderer's median
time in ms. *Gap closed* is `((0) − variant) / ((0) − vello_cpu)` per page,
as min / median / max. A negative value means the variant is slower than (0).

| Scale | (0) | (a) | (b) | (c) | vello_cpu | Gap closed by (a) | by (b) | by (c) |
|---|---|---|---|---|---|---|---|---|
| 1 | 3.79 / 6.48 / 15.40 | 4.44 / 7.06 / 14.53 | 6.39 / 8.69 / 16.50 | 4.29 / 5.94 / 13.57 | 0.58 / 0.73 / 1.51 | -0.21 / -0.12 / 0.06 | -0.84 / -0.41 / -0.08 | -0.16 / 0.08 / 0.18 |
| 2 | 13.95 / 23.93 / 59.90 | 17.03 / 26.94 / 57.44 | 25.01 / 33.38 / 64.83 | 17.18 / 23.27 / 53.95 | 2.19 / 2.75 / 5.42 | -0.27 / -0.15 / 0.05 | -0.96 / -0.47 / -0.09 | -0.28 / 0.03 / 0.15 |

Share of (0)'s time per part over the pages (min / median / max):

| Scale | Clear | Vector preparation | FillRect | DrawMask | DrawImage |
|---|---|---|---|---|---|
| 1 | 0.03 / 0.08 / 0.13 | 0.00 / 0.00 / 0.00 | 0.15 / 0.34 / 0.50 | 0.26 / 0.58 / 0.68 | 0.00 / 0.00 / 0.56 |
| 2 | 0.03 / 0.08 / 0.14 | 0.00 / 0.00 / 0.00 | 0.15 / 0.35 / 0.52 | 0.24 / 0.56 / 0.66 | 0.00 / 0.00 / 0.58 |

## Pixel differences against (0)

Differences are in 1/255, over opaque interior pixels, translucent interior
pixels and edge pixels, classified as in the spike. Per-page values are in
the raw files.

- **(b) and (c)**: max 1 everywhere (interior 0, translucent ≤ 1, edge mean
  0.01–0.03). The difference comes only from dropping (0)'s 8-bit
  sRGB quantization after each command.
- **(a)**:
  - Gallery: opaque interior max 25 on a few pixels; translucent interior max
    47 with a median page mean of 22.3 (scale 1) and 20.8 (scale 2); edge max
    73, edge mean 2.3–6.3 (scale 1) and 1.3–5.2 (scale 2).
  - Synthetic workloads: edge max 42–54. The bars are identical, being opaque
    images over opaque tracks.
  - These are the same blend-space differences the spike measured between
    pert-ui and vello (translucent fills ≈ 22/255).

An earlier pair of gated runs used a first version of (a), which quantized
the source to bytes on every pixel, including fill-shortcut hits. It was
slower: gallery medians 7.3 / 27.7 ms, closing −15% / −18% of the gap. Its
raw files are not kept; the other variants measured the same within 1% in
those runs. Shorter unrecorded trials before that showed that f32 `round()`
(a libm call on baseline x86-64) and the binary-search encode in a
full-frame pass make (a) and (b) much slower still; both were replaced
before any recorded run.

## Reading

**What was measured.**

- On the gallery, the median page takes 6.5 ms in (0), 7.1 in (a), 8.7 in
  (b), 6.0 in (c) and 0.75 in vello_cpu at scale 1. At scale 2 the figures
  are 23.9, 26.9, 33.4, 23.3 and 2.7 ms.
- **Neither (a) nor (b) closes any of the gap on the median page.**
  - (a) is 11–15% of the gap *slower*.
  - (b) is 42–47% of the gap slower.
  - (c) closes 8% at scale 1 and 3% at scale 2 (at most 18%).
- **The blend storage is not what makes vello_cpu 7–17× faster.**
  - Encoded-space blending, the price of path 2's contract, buys no speed
    in this renderer.
  - A linear 16-bit frame costs speed. The frame doubles to 8 bytes per
    pixel, and the final encode pass adds about 2.3 ms at 1280×800 and 9.5 ms
    at 2560×1600: the empty scene takes 2.82 / 11.55 ms against 0.49 / 2.08.
- (a)'s empty scene is 0.26 / 0.96 ms slower than (0)'s even though no
  command is drawn and the clear code is the same. This is probably
  allocation or page-fault placement of the frame buffer, inferred and not
  investigated. Without it, (a)'s per-command work stays within about ±25%
  of (0)'s, slower on fills and faster on images:
  - full panels 3.78 against 3.38 ms;
  - light panels 2.52 against 1.98 ms;
  - bars 9.41 against 10.03 ms.

**Where (0)'s time goes (measured split).**

- Mask commands take 56–58% of (0)'s time on the median page. The timer
  wraps the whole command: per-pixel f64 coordinate mapping, a quarter-turn
  lookup, bilinear sampling, and the blend and encode of every covered
  pixel. The split does not separate sampling from blending.
- `FillRect` takes 34–35%: per-pixel loops even when the fill shortcut
  hits.
- The frame clear takes 8%.
- Images take 0% on most pages but up to 57–58% on image-heavy ones. The
  bars' 32 clipped images take 8.2 of 10.5 ms.
- In the synthetic polyline, the line's coverage mask (cached, see the
  caveat) takes 7.2 of 8.9 ms just to be sampled and blended over its
  800×400 area.

**Where vello_cpu's speed comes from (inferred, not measured here).**

- vello_cpu computes coverage per 4-row strip analytically and fills solid
  interior spans as runs (sparse strips). It has no per-pixel mask sampling
  for geometry and works in a SIMD u8/u16 pipeline (fearless_simd; the
  `u8_pipeline` feature).
- In pert-ui, per-pixel generic work per command dominates. This is
  consistent with the fact that the span fast path of (c) alone saves about
  2.3 ms on the light panels (5.56 → 3.28 ms), where the scene is one
  full-window fill plus small commands.
- A fast path like (c)'s on top of (0), without the 16-bit frame, would keep
  that saving without (b)'s fixed cost. That combination was not measured.

**Not done.**

- Span fast paths for mask interiors and SIMD.
- Measuring each variant in isolation instead of interleaved.
- Vector preparation per frame (cached here).
- Multithreaded vello_cpu (its numbers are in the spike).
