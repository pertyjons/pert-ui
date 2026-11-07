# vello spike results (2026-11-05)

Evidence for phase 0, item 6 of
[plans/rendering-capabilities.md](../../../rendering-capabilities.md). Every
number below was measured on the one machine described here; none is a claim
about other machines, platforms or drivers. How to reproduce:
[../README.md](../README.md). The raw outputs are `raw-workloads-{1,2,3}.md`
(three runs of `run.sh`) and `raw-gallery.md` (`gallery.sh`). Images, all at
scale 1: `panels-*.png` (whole panels frame per renderer, plus
`panels-translated_vello_cpu.png` from part 2), `corner-*.png` (200×200 crop
at the first panel's top-left corner) and `corner-x8-*.png` (25×25 of it,
8× nearest), `polyline-x4-*.png` (50×50 around the largest pert-ui CPU /
vello_cpu difference, 4×), `bars-*.png` (the first four meter bars) and the
gallery's `colors` page from pert-ui CPU and from the translation.

## Environment

| | |
|---|---|
| CPU | 12th Gen Intel Core i9-12900 (8 P-cores with HT + 8 E-cores, 24 logical CPUs, `lscpu` max 2401 MHz, governor `performance`) |
| Pinning | `taskset -c 2-7`: logical CPUs 2–7 = P-cores 1–3, both hyperthreads each; `available_parallelism` = 6, so vello_cpu's default is 5 worker threads |
| GPU | Intel UHD Graphics 770 (ADL-S GT1), integrated, Vulkan (wgpu `AdapterInfo` of vello_gpu's device; pert-ui-wgpu picks its own adapter on the same machine, which has only this GPU) |
| Driver | Intel open-source Mesa driver, Mesa 26.2.6 |
| Kernel | 7.2.15-200.fc44.x86_64 (Fedora 44, Wayland session) |
| Rust | rustc 1.99.1 (58cf8f9c3 2026-10-13), release profile, no debuginfo |
| Crates | vello_cpu 0.3.0 (features `std`, `u8_pipeline`, `multithreading`), vello_gpu 0.3.0 (`wgpu` only), vello_common 0.3.0, peniko 0.6.1, kurbo 0.13.1, fearless_simd 0.7.0, wgpu 30.0.1; pert-ui at commit 78246d1 (its crates unmodified) |

Other, unrelated processes (builds and tests of other projects, an IDE) ran
on the machine during the measurements and were not pinned away from CPUs
2–7; 1-minute load averages between 1.3 and 6.7 were recorded. The three
workload runs agree on the medians within a few percent for most rows; the
range column shows where they do not.

## Versions, MSRV, licence and weight (from the registry sources)

- vello_cpu, vello_gpu, vello_common and vello_gpu_shaders 0.3.0 all declare
  `rust-version = "1.89"` and `Apache-2.0 OR MIT`. pert-ui's MSRV (1.99.1) is
  above that.
- vello_gpu 0.3.0 depends on wgpu `30.0.0` (semver-compatible with pert-ui's
  30.0.1), so both run in one process and one binary; Cargo resolved a single
  wgpu 30.0.1. Its default features (`wgpu_default`) turn on wgpu's default
  features, i.e. its default backends, for every wgpu user in the build, so
  the spike disables them and enables wgpu's `vulkan` itself.
- Both use peniko 0.6.1 and kurbo 0.13.1 (through vello_common), the versions
  the rendering plan studies.
- Crates the two add to a pert-ui + pert-ui-wgpu build (`cargo tree`, with
  the features above): vello_cpu 18 (vello_common, peniko, kurbo, color,
  fearless_simd, guillotiere, euclid, polycool, and rayon + crossbeam +
  thread_local + ordered-channel for `multithreading`); vello_gpu 62 (sharing the vello_common group with vello_cpu), most of
  them build dependencies of vello_gpu_shaders' build script, which compiles
  WESL shaders with naga at build time (wesl, wgsl-parse, lalrpop, logos,
  regex, petgraph, sha3, …).

## Method notes that matter for reading the numbers

- Every workload changes every frame except the panels, and every renderer
  redraws the whole frame each time. pert-ui recomposes the panels each frame
  too (the component reads the frame signal), with its shape cache warm, so
  its compose times are for full repaints; vello has no retained scene, so
  its encode time is spent every frame (an application could re-render an
  unchanged vello_cpu context, which is not measured).
- *Frame total* = compose + render for pert-ui, encode + render for
  vello_cpu, and encode + prepare + GPU + read-back for vello_gpu. Producing
  the data (pert-ui's signal set + flush, the generators for vello) is left
  out of all totals.
- Both GPU renderers include a read-back to an RGBA8 buffer.
  `render_offscreen` does prepare, pass, completion and read-back in one call,
  so pert-ui wgpu has no finer breakdown through the public API. vello_gpu's
  render and read-back are two submissions with a blocking wait each, so its
  "submit + GPU completion" and "read-back" are separately measurable; a
  single submission would save one round trip. Presentation is not measured.
- vello_gpu renders into an `Rgba8Unorm` texture with a depth attachment
  (its documented overdraw reduction), as its `render_to_file` example does.
- Where pert-ui and vello differ in primitives, the spike matches pert-ui:
  borders are strokes of the outline inset by half the width with radius
  r − w/2 (pert-ui's concentric inward stroke), polyline caps are butt
  (pert-ui's `StrokeStyle::new` default; kurbo's is round), all rectangle
  edges lie on whole logical pixels.
- The gradient bars show the gap: pert-ui has no gradient paint, so the
  workload uses one CPU-built `RasterImage` of the meter gradient (1 texel
  wide, one texel per physical row, sRGB-interpolated like peniko's default)
  for all 32 bars, each drawn under a clip of its lit part. Banded
  `FillRect`s were not measured.
- vello_cpu "1 thread" is `num_threads: 0` (the single-threaded dispatcher),
  "5 workers" the multithreaded dispatcher. In multithreaded mode the
  encode time includes `flush`, which waits for the workers' path processing.

## Frame cost: median over three runs

Times in ms. *Frame total median* is the median of the three runs' medians
(200 frames after 20 warm-up frames each), with the range of the three; *p95*
is the median of the three runs' 95th percentiles; phases are medians.

### Panels (150 rounded + 100 plain) — scale 1

pert-ui command mix: 1151 FillRect, 1200 DrawMask (10 mask resources), no
images or vectors. Light variant: 161 FillRect, 160 DrawMask. Polyline: 1
FillRect + 1 DrawVector. Bars: 33 FillRect + 32 clipped DrawImage. (A gallery
page composes about 100 FillRects and 130–200 masks, so the full panels
scene is about 10× a page's command count; the light one is close to it.)

| Renderer | Frame total median (range over runs) | p95 | Phases (median) |
|---|---|---|---|
| pert-ui CPU | 4.180 (4.108–4.206) | 4.219 | compose (paint): 0.146<br>CpuRenderer::render: 4.035 |
| pert-ui wgpu | 4.854 (4.842–4.916) | 5.203 | compose (paint): 0.154<br>render_offscreen (prepare + GPU + read-back): 4.706 |
| vello_cpu 1 thread | 1.466 (1.438–1.491) | 1.509 | encode (+flush): 1.049<br>render: 0.412 |
| vello_cpu 5 workers | 0.543 (0.542–0.644) | 0.563 | encode (+flush): 0.347<br>render: 0.196 |
| vello_gpu | 4.290 (4.254–4.306) | 6.496 | encode (Scene): 1.083<br>prepare (Renderer::render): 0.158<br>submit + GPU completion: 1.663<br>read-back: 1.343 |

### Panels light (20 rounded + 20 plain) — scale 1

| Renderer | Frame total median (range over runs) | p95 | Phases (median) |
|---|---|---|---|
| pert-ui CPU | 2.576 (2.428–2.610) | 2.615 | compose (paint): 0.023<br>CpuRenderer::render: 2.553 |
| pert-ui wgpu | 3.983 (3.977–4.580) | 6.103 | compose (paint): 0.031<br>render_offscreen (prepare + GPU + read-back): 3.951 |
| vello_cpu 1 thread | 0.477 (0.466–0.477) | 0.485 | encode (+flush): 0.202<br>render: 0.274 |
| vello_cpu 5 workers | 0.220 (0.220–0.283) | 0.235 | encode (+flush): 0.100<br>render: 0.120 |
| vello_gpu | 4.425 (4.336–4.870) | 5.355 | encode (Scene): 0.233<br>prepare (Renderer::render): 0.095<br>submit + GPU completion: 2.171<br>read-back: 1.953 |

### Polyline 800 points — scale 1

| Renderer | Frame total median (range over runs) | p95 | Phases (median) |
|---|---|---|---|
| pert-ui CPU | 12.197 (12.143–12.474) | 12.486 | compose (paint): 0.028<br>CpuRenderer::render: 12.171 |
| pert-ui wgpu | 5.749 (5.677–6.090) | 7.290 | compose (paint): 0.028<br>render_offscreen (prepare + GPU + read-back): 5.716 |
| vello_cpu 1 thread | 0.602 (0.600–0.638) | 0.615 | encode (+flush): 0.416<br>render: 0.186 |
| vello_cpu 5 workers | 0.582 (0.571–0.689) | 0.754 | encode (+flush): 0.465<br>render: 0.116 |
| vello_gpu | 3.558 (3.261–4.700) | 6.524 | encode (Scene): 0.436<br>prepare (Renderer::render): 0.065<br>submit + GPU completion: 1.418<br>read-back: 1.600 |

### Polyline 4096 points — scale 1

| Renderer | Frame total median (range over runs) | p95 | Phases (median) |
|---|---|---|---|
| pert-ui CPU | 13.997 (13.384–14.429) | 20.662 | compose (paint): 0.112<br>CpuRenderer::render: 13.886 |
| pert-ui wgpu | 7.105 (6.844–12.438) | 8.590 | compose (paint): 0.109<br>render_offscreen (prepare + GPU + read-back): 6.994 |
| vello_cpu 1 thread | 1.015 (0.987–1.874) | 1.035 | encode (+flush): 0.801<br>render: 0.213 |
| vello_cpu 5 workers | 1.000 (0.981–4.859) | 1.301 | encode (+flush): 0.880<br>render: 0.118 |
| vello_gpu | 3.790 (3.423–3.956) | 5.564 | encode (Scene): 0.844<br>prepare (Renderer::render): 0.074<br>submit + GPU completion: 1.306<br>read-back: 1.507 |

### Gradient bars (32) — scale 1

| Renderer | Frame total median (range over runs) | p95 | Phases (median) |
|---|---|---|---|
| pert-ui CPU | 11.503 (11.277–11.826) | 11.745 | compose (paint): 0.010<br>CpuRenderer::render: 11.493 |
| pert-ui wgpu | 4.112 (4.064–4.123) | 4.392 | compose (paint): 0.010<br>render_offscreen (prepare + GPU + read-back): 4.103 |
| vello_cpu 1 thread | 0.711 (0.689–0.719) | 0.732 | encode (+flush): 0.029<br>render: 0.681 |
| vello_cpu 5 workers | 0.399 (0.386–0.402) | 0.423 | encode (+flush): 0.055<br>render: 0.338 |
| vello_gpu | 3.701 (2.148–3.829) | 4.660 | encode (Scene): 0.008<br>prepare (Renderer::render): 0.041<br>submit + GPU completion: 1.662<br>read-back: 1.847 |

### Panels (150 rounded + 100 plain) — scale 2

| Renderer | Frame total median (range over runs) | p95 | Phases (median) |
|---|---|---|---|
| pert-ui CPU | 15.559 (15.303–16.164) | 15.927 | compose (paint): 0.172<br>CpuRenderer::render: 15.382 |
| pert-ui wgpu | 9.213 (9.200–9.214) | 9.992 | compose (paint): 0.180<br>render_offscreen (prepare + GPU + read-back): 9.026 |
| vello_cpu 1 thread | 2.677 (2.651–2.764) | 2.833 | encode (+flush): 1.434<br>render: 1.224 |
| vello_cpu 5 workers | 0.974 (0.964–0.989) | 1.052 | encode (+flush): 0.476<br>render: 0.496 |
| vello_gpu | 10.538 (10.457–13.140) | 13.949 | encode (Scene): 1.490<br>prepare (Renderer::render): 0.331<br>submit + GPU completion: 4.135<br>read-back: 4.569 |

### Panels light (20 rounded + 20 plain) — scale 2

| Renderer | Frame total median (range over runs) | p95 | Phases (median) |
|---|---|---|---|
| pert-ui CPU | 13.568 (10.287–20.810) | 14.170 | compose (paint): 0.041<br>CpuRenderer::render: 13.527 |
| pert-ui wgpu | 8.844 (8.369–14.468) | 9.675 | compose (paint): 0.045<br>render_offscreen (prepare + GPU + read-back): 8.801 |
| vello_cpu 1 thread | 1.379 (1.351–3.296) | 1.990 | encode (+flush): 0.341<br>render: 1.060 |
| vello_cpu 5 workers | 0.611 (0.518–6.293) | 0.651 | encode (+flush): 0.174<br>render: 0.435 |
| vello_gpu | 9.214 (8.840–12.009) | 11.604 | encode (Scene): 0.374<br>prepare (Renderer::render): 0.186<br>submit + GPU completion: 3.724<br>read-back: 4.873 |

### Polyline 800 points — scale 2

| Renderer | Frame total median (range over runs) | p95 | Phases (median) |
|---|---|---|---|
| pert-ui CPU | 42.241 (42.171–42.866) | 44.724 | compose (paint): 0.032<br>CpuRenderer::render: 42.210 |
| pert-ui wgpu | 16.201 (15.444–25.647) | 16.802 | compose (paint): 0.037<br>render_offscreen (prepare + GPU + read-back): 16.164 |
| vello_cpu 1 thread | 1.852 (1.560–1.855) | 1.968 | encode (+flush): 0.834<br>render: 0.975 |
| vello_cpu 5 workers | 1.301 (1.136–1.718) | 1.648 | encode (+flush): 0.933<br>render: 0.365 |
| vello_gpu | 9.012 (8.847–9.851) | 11.089 | encode (Scene): 0.848<br>prepare (Renderer::render): 0.151<br>submit + GPU completion: 3.187<br>read-back: 4.797 |

### Polyline 4096 points — scale 2

| Renderer | Frame total median (range over runs) | p95 | Phases (median) |
|---|---|---|---|
| pert-ui CPU | 44.350 (44.192–44.535) | 46.260 | compose (paint): 0.107<br>CpuRenderer::render: 44.243 |
| pert-ui wgpu | 16.990 (16.918–17.048) | 18.374 | compose (paint): 0.112<br>render_offscreen (prepare + GPU + read-back): 16.877 |
| vello_cpu 1 thread | 2.104 (2.028–2.250) | 2.193 | encode (+flush): 1.198<br>render: 0.924 |
| vello_cpu 5 workers | 1.624 (1.566–1.807) | 2.113 | encode (+flush): 1.269<br>render: 0.347 |
| vello_gpu | 9.111 (8.679–9.337) | 10.551 | encode (Scene): 1.268<br>prepare (Renderer::render): 0.144<br>submit + GPU completion: 3.092<br>read-back: 4.553 |

### Gradient bars (32) — scale 2

| Renderer | Frame total median (range over runs) | p95 | Phases (median) |
|---|---|---|---|
| pert-ui CPU | 45.584 (45.372–45.691) | 46.323 | compose (paint): 0.011<br>CpuRenderer::render: 45.574 |
| pert-ui wgpu | 8.522 (8.483–8.608) | 8.991 | compose (paint): 0.015<br>render_offscreen (prepare + GPU + read-back): 8.507 |
| vello_cpu 1 thread | 2.163 (2.008–2.215) | 2.336 | encode (+flush): 0.051<br>render: 2.111 |
| vello_cpu 5 workers | 0.948 (0.935–0.975) | 0.996 | encode (+flush): 0.082<br>render: 0.864 |
| vello_gpu | 8.359 (8.085–8.983) | 10.752 | encode (Scene): 0.014<br>prepare (Renderer::render): 0.113<br>submit + GPU completion: 3.337<br>read-back: 4.841 |


Empty scene, render + read-back (median per run): pert-ui wgpu 2.97–3.06 ms at scale 1 and 6.90–7.13 ms at scale 2; vello_gpu 2.39–2.70 ms and 4.78–4.84 ms.

## Parity of the last frame (reference: pert-ui CPU)

Frame 219 of each workload; identical in all three runs.

Channel differences over R, G, B in 1/255. *Interior* = pixels whose 3×3 neighborhood in the reference is one color; *edge* = all others (antialiased edges, the line, gradient rows). Cells: max / mean / share of pixels > 2.

| Workload | Scale | Renderer | All | Interior | Edge (pixels) |
|---|---|---|---|---|---|
| Panels (150 rounded + 100 plain) | 1 | pert-ui wgpu | 1 / 0.001 / 0.00% | 0 / 0.000 / 0.00% | 1 / 0.008 / 0.00% (122466) |
| Panels (150 rounded + 100 plain) | 1 | vello_cpu 1 thread | 69 / 0.147 / 1.03% | 0 / 0.000 / 0.00% | 69 / 1.231 / 8.59% (122466) |
| Panels (150 rounded + 100 plain) | 1 | vello_cpu 5 workers | 69 / 0.147 / 1.03% | 0 / 0.000 / 0.00% | 69 / 1.231 / 8.59% (122466) |
| Panels (150 rounded + 100 plain) | 1 | vello_gpu | 70 / 0.148 / 1.02% | 0 / 0.000 / 0.00% | 70 / 1.242 / 8.50% (122466) |
| Panels (150 rounded + 100 plain) | 1 | vello_gpu vs vello_cpu 1 thread | 2 / 0.005 / 0.00% | 0 / 0.000 / 0.00% | 2 / 0.040 / 0.00% (123246) |
| Panels light (20 rounded + 20 plain) | 1 | pert-ui wgpu | 1 / 0.000 / 0.00% | 0 / 0.000 / 0.00% | 1 / 0.003 / 0.00% (46472) |
| Panels light (20 rounded + 20 plain) | 1 | vello_cpu 1 thread | 69 / 0.020 / 0.14% | 0 / 0.000 / 0.00% | 69 / 0.434 / 3.11% (46472) |
| Panels light (20 rounded + 20 plain) | 1 | vello_cpu 5 workers | 69 / 0.020 / 0.14% | 0 / 0.000 / 0.00% | 69 / 0.434 / 3.11% (46472) |
| Panels light (20 rounded + 20 plain) | 1 | vello_gpu | 69 / 0.020 / 0.14% | 0 / 0.000 / 0.00% | 69 / 0.439 / 3.12% (46472) |
| Panels light (20 rounded + 20 plain) | 1 | vello_gpu vs vello_cpu 1 thread | 2 / 0.001 / 0.00% | 0 / 0.000 / 0.00% | 2 / 0.014 / 0.00% (46548) |
| Polyline 800 points | 1 | pert-ui wgpu | 1 / 0.003 / 0.00% | 0 / 0.000 / 0.00% | 1 / 0.062 / 0.00% (57143) |
| Polyline 800 points | 1 | vello_cpu 1 thread | 134 / 0.955 / 3.44% | 0 / 0.000 / 0.00% | 134 / 17.110 / 61.58% (57143) |
| Polyline 800 points | 1 | vello_cpu 5 workers | 134 / 0.955 / 3.44% | 0 / 0.000 / 0.00% | 134 / 17.110 / 61.58% (57143) |
| Polyline 800 points | 1 | vello_gpu | 134 / 0.957 / 3.43% | 0 / 0.000 / 0.00% | 134 / 17.154 / 61.46% (57143) |
| Polyline 800 points | 1 | vello_gpu vs vello_cpu 1 thread | 1 / 0.006 / 0.00% | 0 / 0.000 / 0.00% | 1 / 0.114 / 0.00% (49669) |
| Polyline 800 points | 1 | vello_cpu coverage composited in linear light (post hoc) | 130 / 0.654 / 3.34% | 0 / 0.000 / 0.00% | 130 / 11.721 / 59.84% (57143) |
| Polyline 4096 points | 1 | pert-ui wgpu | 1 / 0.006 / 0.00% | 0 / 0.000 / 0.00% | 1 / 0.097 / 0.00% (60787) |
| Polyline 4096 points | 1 | vello_cpu 1 thread | 145 / 1.004 / 3.70% | 0 / 0.000 / 0.00% | 145 / 16.916 / 62.41% (60787) |
| Polyline 4096 points | 1 | vello_cpu 5 workers | 145 / 1.004 / 3.70% | 0 / 0.000 / 0.00% | 145 / 16.916 / 62.41% (60787) |
| Polyline 4096 points | 1 | vello_gpu | 146 / 1.007 / 3.70% | 0 / 0.000 / 0.00% | 146 / 16.960 / 62.27% (60787) |
| Polyline 4096 points | 1 | vello_gpu vs vello_cpu 1 thread | 1 / 0.006 / 0.00% | 0 / 0.000 / 0.00% | 1 / 0.117 / 0.00% (53174) |
| Polyline 4096 points | 1 | vello_cpu coverage composited in linear light (post hoc) | 133 / 0.666 / 3.57% | 0 / 0.000 / 0.00% | 133 / 11.213 / 60.16% (60787) |
| Gradient bars (32) | 1 | pert-ui wgpu | 0 / 0.000 / 0.00% | 0 / 0.000 / 0.00% | 0 / 0.000 / 0.00% (291774) |
| Gradient bars (32) | 1 | vello_cpu 1 thread | 1 / 0.031 / 0.00% | 0 / 0.000 / 0.00% | 1 / 0.110 / 0.00% (291774) |
| Gradient bars (32) | 1 | vello_cpu 5 workers | 1 / 0.031 / 0.00% | 0 / 0.000 / 0.00% | 1 / 0.110 / 0.00% (291774) |
| Gradient bars (32) | 1 | vello_gpu | 1 / 0.031 / 0.00% | 0 / 0.000 / 0.00% | 1 / 0.110 / 0.00% (291774) |
| Gradient bars (32) | 1 | vello_gpu vs vello_cpu 1 thread | 1 / 0.000 / 0.00% | 0 / 0.000 / 0.00% | 1 / 0.000 / 0.00% (274152) |
| Panels (150 rounded + 100 plain) | 2 | pert-ui wgpu | 1 / 0.001 / 0.00% | 0 / 0.000 / 0.00% | 1 / 0.007 / 0.00% (310664) |
| Panels (150 rounded + 100 plain) | 2 | vello_cpu 1 thread | 77 / 0.080 / 0.55% | 0 / 0.000 / 0.00% | 77 / 1.052 / 7.27% (310664) |
| Panels (150 rounded + 100 plain) | 2 | vello_cpu 5 workers | 77 / 0.080 / 0.55% | 0 / 0.000 / 0.00% | 77 / 1.052 / 7.27% (310664) |
| Panels (150 rounded + 100 plain) | 2 | vello_gpu | 78 / 0.081 / 0.55% | 0 / 0.000 / 0.00% | 78 / 1.062 / 7.25% (310664) |
| Panels (150 rounded + 100 plain) | 2 | vello_gpu vs vello_cpu 1 thread | 2 / 0.002 / 0.00% | 0 / 0.000 / 0.00% | 2 / 0.031 / 0.00% (312128) |
| Panels light (20 rounded + 20 plain) | 2 | pert-ui wgpu | 1 / 0.000 / 0.00% | 0 / 0.000 / 0.00% | 1 / 0.003 / 0.00% (116524) |
| Panels light (20 rounded + 20 plain) | 2 | vello_cpu 1 thread | 76 / 0.011 / 0.08% | 0 / 0.000 / 0.00% | 76 / 0.388 / 2.67% (116524) |
| Panels light (20 rounded + 20 plain) | 2 | vello_cpu 5 workers | 76 / 0.011 / 0.08% | 0 / 0.000 / 0.00% | 76 / 0.388 / 2.67% (116524) |
| Panels light (20 rounded + 20 plain) | 2 | vello_gpu | 76 / 0.011 / 0.08% | 0 / 0.000 / 0.00% | 76 / 0.391 / 2.66% (116524) |
| Panels light (20 rounded + 20 plain) | 2 | vello_gpu vs vello_cpu 1 thread | 2 / 0.000 / 0.00% | 0 / 0.000 / 0.00% | 2 / 0.011 / 0.00% (116700) |
| Polyline 800 points | 2 | pert-ui wgpu | 220 / 0.003 / 0.00% | 0 / 0.000 / 0.00% | 220 / 0.088 / 0.03% (157105) |
| ↳ | | largest pert-ui wgpu difference at physical (478, 610), frame 219: RGB pert-ui CPU [18, 22, 30], wgpu [98, 167, 250], vello_cpu [56, 89, 132] | | | |
| Polyline 800 points | 2 | vello_cpu 1 thread | 157 / 0.610 / 2.02% | 0 / 0.000 / 0.00% | 157 / 15.903 / 52.62% (157105) |
| Polyline 800 points | 2 | vello_cpu 5 workers | 157 / 0.610 / 2.02% | 0 / 0.000 / 0.00% | 157 / 15.903 / 52.62% (157105) |
| Polyline 800 points | 2 | vello_gpu | 157 / 0.612 / 2.01% | 0 / 0.000 / 0.00% | 157 / 15.943 / 52.52% (157105) |
| Polyline 800 points | 2 | vello_gpu vs vello_cpu 1 thread | 1 / 0.003 / 0.00% | 0 / 0.000 / 0.00% | 1 / 0.090 / 0.00% (134308) |
| Polyline 800 points | 2 | vello_cpu coverage composited in linear light (post hoc) | 155 / 0.449 / 1.98% | 0 / 0.000 / 0.00% | 155 / 11.714 / 51.70% (157105) |
| Polyline 4096 points | 2 | pert-ui wgpu | 1 / 0.002 / 0.00% | 0 / 0.000 / 0.00% | 1 / 0.048 / 0.00% (167191) |
| Polyline 4096 points | 2 | vello_cpu 1 thread | 156 / 0.642 / 2.15% | 0 / 0.000 / 0.00% | 156 / 15.729 / 52.71% (167191) |
| Polyline 4096 points | 2 | vello_cpu 5 workers | 156 / 0.642 / 2.15% | 0 / 0.000 / 0.00% | 156 / 15.729 / 52.71% (167191) |
| Polyline 4096 points | 2 | vello_gpu | 156 / 0.644 / 2.15% | 0 / 0.000 / 0.00% | 156 / 15.769 / 52.61% (167191) |
| Polyline 4096 points | 2 | vello_gpu vs vello_cpu 1 thread | 1 / 0.003 / 0.00% | 0 / 0.000 / 0.00% | 1 / 0.094 / 0.00% (144099) |
| Polyline 4096 points | 2 | vello_cpu coverage composited in linear light (post hoc) | 155 / 0.462 / 2.11% | 0 / 0.000 / 0.00% | 155 / 11.331 / 51.68% (167191) |
| Gradient bars (32) | 2 | pert-ui wgpu | 0 / 0.000 / 0.00% | 0 / 0.000 / 0.00% | 0 / 0.000 / 0.00% (725488) |
| Gradient bars (32) | 2 | vello_cpu 1 thread | 1 / 0.032 / 0.00% | 1 / 0.008 / 0.00% | 1 / 0.144 / 0.00% (725488) |
| Gradient bars (32) | 2 | vello_cpu 5 workers | 1 / 0.032 / 0.00% | 1 / 0.008 / 0.00% | 1 / 0.144 / 0.00% (725488) |
| Gradient bars (32) | 2 | vello_gpu | 1 / 0.032 / 0.00% | 1 / 0.008 / 0.00% | 1 / 0.143 / 0.00% (725488) |
| Gradient bars (32) | 2 | vello_gpu vs vello_cpu 1 thread | 1 / 0.000 / 0.00% | 0 / 0.000 / 0.00% | 1 / 0.001 / 0.00% (714080) |


## Part 2: the workloads' DisplayLists translated into vello_cpu (run 1)

Per frame: pert-ui compose (not in the totals), translation (walking the commands, issuing vello calls, converting and registering resources not seen before; with `flush`) and vello_cpu render. Parity against pert-ui CPU rendering the same DisplayList. Cells: median / p95 ms; parity max / mean / share > 2 over all, interior, edge.

| Workload | Scale | Threads | Translate | Render | Total | First-frame translate ms (new resources, KiB) | Parity all | Interior | Edge |
|---|---|---|---|---|---|---|---|---|---|
| Panels (150 rounded + 100 plain) | 1 | 1 | 0.349 / 0.359 | 0.816 / 0.836 | 1.164 / 1.195 | 0.804 (10, 6) | 42 / 0.097 / 0.89% | 0 / 0.000 / 0.00% | 42 / 0.808 / 7.41% |
| Panels (150 rounded + 100 plain) | 1 | 5 workers | 0.330 / 0.339 | 0.411 / 0.454 | 0.741 / 0.764 | 1.072 (10, 6) | 42 / 0.097 / 0.89% | 0 / 0.000 / 0.00% | 42 / 0.808 / 7.41% |
| Panels light (20 rounded + 20 plain) | 1 | 1 | 0.055 / 0.058 | 0.357 / 0.366 | 0.413 / 0.423 | 0.376 (10, 6) | 41 / 0.014 / 0.12% | 0 / 0.000 / 0.00% | 41 / 0.305 / 2.75% |
| Panels light (20 rounded + 20 plain) | 1 | 5 workers | 0.074 / 0.079 | 0.176 / 0.206 | 0.251 / 0.279 | 0.466 (10, 6) | 41 / 0.014 / 0.12% | 0 / 0.000 / 0.00% | 41 / 0.305 / 2.75% |
| Polyline 800 points | 1 | 1 | 0.437 / 0.449 | 0.197 / 0.203 | 0.634 / 0.650 | 0.718 (0, 0) | 134 / 0.955 / 3.44% | 0 / 0.000 / 0.00% | 134 / 17.110 / 61.58% |
| Polyline 800 points | 1 | 5 workers | 0.467 / 0.635 | 0.117 / 0.144 | 0.582 / 0.776 | 1.105 (0, 0) | 134 / 0.955 / 3.44% | 0 / 0.000 / 0.00% | 134 / 17.110 / 61.58% |
| Polyline 4096 points | 1 | 1 | 0.827 / 1.226 | 0.202 / 0.363 | 1.027 / 1.577 | 1.202 (0, 0) | 145 / 1.004 / 3.70% | 0 / 0.000 / 0.00% | 145 / 16.916 / 62.41% |
| Polyline 4096 points | 1 | 5 workers | 0.889 / 1.169 | 0.119 / 0.138 | 1.007 / 1.306 | 1.419 (0, 0) | 145 / 1.004 / 3.70% | 0 / 0.000 / 0.00% | 145 / 16.916 / 62.41% |
| Gradient bars (32) | 1 | 1 | 0.027 / 0.030 | 1.433 / 1.461 | 1.461 / 1.489 | 0.351 (1, 2) | 0 / 0.000 / 0.00% | 0 / 0.000 / 0.00% | 0 / 0.000 / 0.00% |
| Gradient bars (32) | 1 | 5 workers | 0.057 / 0.065 | 0.617 / 0.632 | 0.674 / 0.693 | 0.573 (1, 2) | 0 / 0.000 / 0.00% | 0 / 0.000 / 0.00% | 0 / 0.000 / 0.00% |
| Panels (150 rounded + 100 plain) | 2 | 1 | 0.375 / 0.410 | 2.168 / 2.470 | 2.543 / 2.864 | 1.177 (10, 24) | 43 / 0.051 / 0.46% | 0 / 0.000 / 0.00% | 43 / 0.675 / 6.06% |
| Panels (150 rounded + 100 plain) | 2 | 5 workers | 0.326 / 0.424 | 0.992 / 1.078 | 1.325 / 1.442 | 1.711 (10, 24) | 43 / 0.051 / 0.46% | 0 / 0.000 / 0.00% | 43 / 0.675 / 6.06% |
| Panels light (20 rounded + 20 plain) | 2 | 1 | 0.067 / 0.071 | 1.363 / 1.508 | 1.431 / 1.578 | 1.061 (10, 24) | 41 / 0.007 / 0.07% | 0 / 0.000 / 0.00% | 41 / 0.248 / 2.29% |
| Panels light (20 rounded + 20 plain) | 2 | 5 workers | 0.101 / 0.112 | 0.503 / 0.588 | 0.605 / 0.689 | 1.384 (10, 24) | 41 / 0.007 / 0.07% | 0 / 0.000 / 0.00% | 41 / 0.248 / 2.29% |
| Polyline 800 points | 2 | 1 | 0.799 / 0.824 | 0.849 / 0.945 | 1.647 / 1.749 | 1.886 (0, 0) | 157 / 0.610 / 2.02% | 0 / 0.000 / 0.00% | 157 / 15.903 / 52.62% |
| Polyline 800 points | 2 | 5 workers | 0.866 / 1.147 | 0.350 / 0.388 | 1.219 / 1.522 | 2.090 (0, 0) | 157 / 0.610 / 2.02% | 0 / 0.000 / 0.00% | 157 / 15.903 / 52.62% |
| Polyline 4096 points | 2 | 1 | 1.223 / 1.255 | 0.958 / 1.051 | 2.181 / 2.308 | 2.355 (0, 0) | 156 / 0.642 / 2.15% | 0 / 0.000 / 0.00% | 156 / 15.729 / 52.71% |
| Polyline 4096 points | 2 | 5 workers | 1.308 / 1.751 | 0.335 / 0.436 | 1.644 / 2.103 | 3.170 (0, 0) | 156 / 0.642 / 2.15% | 0 / 0.000 / 0.00% | 156 / 15.729 / 52.71% |
| Gradient bars (32) | 2 | 1 | 0.050 / 0.056 | 4.808 / 5.117 | 4.859 / 5.173 | 1.061 (1, 5) | 0 / 0.000 / 0.00% | 0 / 0.000 / 0.00% | 0 / 0.000 / 0.00% |
| Gradient bars (32) | 2 | 5 workers | 0.182 / 0.219 | 2.787 / 2.985 | 2.963 / 3.158 | 1.259 (1, 5) | 0 / 0.000 / 0.00% | 0 / 0.000 / 0.00% | 0 / 0.000 / 0.00% |

## Part 2 on the headless gallery (summary)

`gallery.sh`, pinned to CPUs 2–7, 39 pages, each page's DisplayList painted
once and rendered 30 times after one warm-up. Min / median / max over the
pages, ms (full table: `raw-gallery.md`):

| | Scale 1 | Scale 2 |
|---|---|---|
| pert-ui `CpuRenderer::render` | 3.89 / 6.70 / 16.11 | 14.37 / 24.87 / 62.88 |
| Translation, first frame (converting and registering every mask and image) | 0.26 / 0.35 / 0.50 | 0.61 / 1.24 / 1.76 |
| RGBA8 pixmap bytes registered on the first frame, KiB | 223 / 564 / 865 | 883 / 2169 / 3454 |
| Translation, warm | 0.03 / 0.05 / 0.12 | 0.03 / 0.06 / 0.15 |
| vello_cpu render, 1 thread | 0.36 / 0.50 / 1.21 | 1.18 / 1.65 / 4.31 |
| vello_cpu render, 5 workers | 0.18 / 0.25 / 0.45 | 0.43 / 0.58 / 1.37 |
| pert-ui CPU ÷ (translation + vello_cpu 1 thread) | 7.3 / 11.9 / 14.4 | 10.1 / 14.7 / 17.4 |
| Edge pixels: mean difference; max (median page / largest) | 2.41–6.15; 39 / 180 (`colors`; `shapes` 104) | 1.45–5.14; 39 / 182 (`colors`) |
| Opaque interior max | 0 on most pages, ≤ 25 on a few pixels | same |
| Translucent interior mean (31 of 39 pages have some) | 1.0–23.4, median 22.6 | 1.4–22.6, median 21.3 |

Every command of every page translated; none was unsupported.
`gallery-colors-pert_ui_CPU.png` and `gallery-colors-translated_vello_cpu.png`
show one page from both.

## Reading the results

**Cost.** On this machine vello_cpu draws every workload in a fraction of
pert-ui's CPU renderer time: 2.8–23× faster single-threaded, more with workers
on the fill-heavy panels (workers helped the single long polyline by 0–30%
only; its stroke is one path, processed while encoding). Some of pert-ui
CPU's cost is per pixel, not per command: the light panels scene (one
full-window fill plus 320 small commands) already costs 2.6 ms at scale 1
and 10–14 ms at scale 2 (the noisiest row). The
gradient bars expose the image path: 32 clipped `DrawImage`s cost pert-ui CPU
11.5 / 45.6 ms against vello_cpu's 0.7 / 2.2 ms for 32 gradient fills.
pert-ui's vector path (`stroke_polyline`) is the most expensive per command:
12–14 ms at scale 1 and ~42–44 ms at scale 2 in the CPU renderer, and also
dominant in the wgpu renderer, which rasterizes vector coverage on the CPU
(`VectorRasterizer`) before uploading it.

On the GPU side, both renderers' frame totals are dominated by fixed
round-trip cost on this integrated GPU: the empty scene's render + read-back
already costs 2.4–3.1 ms at scale 1 and 4.8–7.1 ms at scale 2, and vello_gpu's
two waits (completion, then read-back) are each 1.3–4.9 ms. vello_gpu was not
faster than vello_cpu for any workload here once read-back is included; it is
in the same range as pert-ui wgpu on panels and bars and 1.6–1.9× faster on
the polyline (where pert-ui wgpu spends its time on CPU vector coverage). A
presenting renderer would not read back, so these totals overstate a window
frame; presentation was not measured for either.

**Composition space (asked by the coordinator).** Both vello renderers
composite in sRGB-encoded space, from their sources:

- vello_cpu's u8 pipeline blends premultiplied *encoded* bytes directly:
  `alpha_composite_solid` / `alpha_composite_inner` compute
  `src + bg × (1 − src_alpha)` on the stored u8 values
  (`vello_cpu-0.3.0/src/fine/lowp/mod.rs:497–555`); solid colors enter as
  `AlphaColor<Srgb>` premultiplied in encoded space
  (`vello_common-0.3.0/src/paint.rs`, `impl From<AlphaColor<Srgb>> for
  Paint`). Nothing under `src/fine/` converts to linear light.
- vello_gpu's blend shader states "The colors are assumed to be in sRGB
  color space" (`vello_gpu_shaders-0.3.0/shaders/helpers/blend.wesl:256`),
  and its examples render to a non-sRGB `Rgba8Unorm` target, so the
  shader's arithmetic is on encoded values too.
- peniko's gradients interpolate in sRGB by default
  (`peniko-0.6.1/src/gradient.rs:18–21`, `DEFAULT_GRADIENT_COLOR_SPACE =
  ColorSpaceTag::Srgb`); other spaces are available per gradient.

There is no switch for linear-light compositing in either crate (linebender
issue 592 tracks it, not verified here beyond the sources above).

**Parity.** Opaque interiors are identical (max 0) in every workload and
renderer, and on the gallery pages except a few pixels. All differences sit
in edges and translucent pixels:

- *Same coverage, different blend space.* The translated panels draw
  pert-ui's own corner masks through vello: max 41–43, edge mean 0.25–0.81,
  only blend space differs. Translucent fills on the gallery pages (31 of 39
  pages, a translucent fill most pages share) differ by ~22/255 on average
  with max 25 over the whole flat area: that is the blend-space difference
  in its purest form, visible as a different color, not an edge artifact.
- *Different coverage as well.* vello's native rounded rectangles: max 69–78,
  edge mean 0.39–1.24 (pert-ui's corner masks take 4 sub-rows per pixel with
  analytic horizontal coverage; vello computes exact area coverage). The
  polyline differs most: edge mean 15.7–17.1 and max 134–157. Recompositing
  vello's own coverage of the line in linear light afterwards only lowers the
  edge mean to 11.2–11.7, so most of the polyline difference is coverage
  (pert-ui's stroke outline expansion and 4 sub-scanline rasterizer against
  vello's stroker and area coverage), not blend space. The 4× crops
  `polyline-x4-*.png` show pert-ui's line visibly softer.
- *Gradients:* vello's gradient against pert-ui's CPU-built image: max 1,
  mean 0.03 (LUT quantization). The translated bars are exact (max 0).
- *Same family:* vello_gpu against vello_cpu differs by at most 2/255 on
  every workload; pert-ui wgpu against pert-ui CPU by at most 1, except the
  observation below.

**Observation about pert-ui itself (a lead, not root-caused).** In the
800-point polyline at scale 2, frame 219, pert-ui wgpu and pert-ui CPU
differ by 220/255 on a few pixels (0.03% of edge pixels); the largest at
physical (478, 610), one column left of the plot's first point at logical
x = 240, where the butt cap of the steep first segment reaches left of the
first point: pert-ui CPU leaves the background `[18, 22, 30]`, pert-ui wgpu
draws almost full line color `[98, 167, 250]` and vello_cpu partial coverage
`[56, 89, 132]`. Earlier, shorter runs showed the same at another frame of
the 4096-point line. Both pert-ui renderers draw the same `VectorRasterizer`
coverage, so the CPU/GPU difference is in how that coverage mask is placed
or sampled at its left edge, and neither matches vello's partial coverage.
It is outside this spike's scope and not recorded in TODO.md here, since the
spike was told not to edit outside its directory.

## Part 2 feasibility: what maps and what does not

Every pert-ui command maps to vello_cpu without layers (`src/translate.rs`):

- `FillRect` → `fill_rect`, snapped to pert-ui's pixel-center rule so edges
  stay aliased as pert-ui draws them (vello would otherwise antialias
  fractional edges).
- `DrawMask` → an image paint with `TintMode::AlphaMask` and the color as
  tint, filled over the clipped destination; the stretch and quarter turn
  become the paint transform, so a clip crops without changing UVs. vello has
  no A8 image format: each mask becomes an RGBA8 pixmap, 4× pert-ui's mask
  bytes (the KiB column above), registered once per mask content and kept in
  the spike's cache (no eviction implemented).
- `DrawImage` → the same with `TintMode::Multiply` (or no tint for white).
  pert-ui's pixels are premultiplied in linear light before sRGB encoding, so
  the translator decodes, un-premultiplies in linear, encodes and lets vello
  premultiply in encoded space; opaque pixels pass unchanged. vello filters
  images in encoded space, pert-ui in linear light, so stretched images
  differ (the colors page's picker area).
- `DrawVector` → kurbo `BezPath` from the `VectorGeometry` elements, filled
  with its rule or stroked with width × scale, caps, joins and miter limit;
  the clip becomes `push_clip_rect`.
- Colors: pert-ui's straight linear RGBA → straight sRGB `AlphaColor`.

What cannot be matched: linear-light compositing and linear-light image
filtering (no option in vello), pert-ui's own coverage for vectors (vello
rasterizes the path itself, which is the point of using it, but changes the
pixels), and A8 storage of masks. Text arrives as pert-ui glyph masks and
maps fine; translating to vello's own glyph runs would need parley/glyph
data in the DisplayList, which it does not carry.

Cost of the translation itself on the gallery: 0.03–0.15 ms warm, 0.26–1.76
ms on a page's first frame (resource conversion). It was implemented for
vello_cpu only; a vello_gpu translation would have to upload masks and
images through `Renderer::upload_image` into its atlas, since vello_gpu
panics on `ImageSource::Pixmap` (see below).

## Documented and observed limitations (vello 0.3.0 sources)

- vello_gpu: mask layers panic — `Scene::push_layer` with a mask hits
  `unimplemented!("mask layers are currently not supported")`
  (`vello_gpu-0.3.0/src/scene.rs:645`; `push_mask_layer` documents "always
  panics", line 720). README "Current state": mask layers, complex filter
  graphs and certain blend modes for non-isolated blending panic.
- vello_gpu: destructive blend modes in non-isolated blending panic
  (`scene.rs:742–747`, "use clip layers instead").
- vello_gpu: `ImageSource::Pixmap` panics ("pixmap image sources are not
  supported by Vello GPU", `src/render/wgpu/mod.rs:701`, and
  `src/paint.rs:122`); images must be uploaded and referenced by id.
- vello_gpu: intermediate resource textures are capped at 4096 per side
  (`src/render/common.rs`, `MAX_RESOURCE_TEXTURE_DIMENSION_2D`).
- vello_cpu: complex filter graphs panic; in multithreaded mode even simple
  filters do (`src/dispatch/multi_threaded.rs:545`,
  `unimplemented!("Filter effects are not yet supported in multi-threaded
  rendering")`; README "Current state"); `push_filter_layer` is documented as
  incomplete and experimental (`src/render.rs:580`). External textures are
  unsupported (`src/fine/mod.rs:1087`).
- Both: glyph caching is experimental and "not recommended for use"
  (README / `lib.rs` "Current state"); the spike disables the `text`
  feature. Both: some failures panic instead of returning errors (vello_gpu
  README), and `RenderContext::render` asserts that all layers were popped.
- Both: scene and target sizes are `u16` (`RenderContext::new(width: u16,
  height: u16)`, `Scene::new`), so 65535 px per side at most.
- Clip layers (`push_clip_layer`) and clip rectangles are supported in both;
  the spike needed only `push_clip_rect`. None of the spike's calls panicked.

## Not done or not measured

- Presentation to a window, input-to-photon latency and GPU timestamp
  queries; GPU phases are wall-clock times around blocking waits.
- vello_gpu translation of the DisplayList (part 2 covers vello_cpu only).
- vello_cpu's `f32_pipeline` (`RenderMode::OptimizeQuality`); only the u8
  pipeline was built.
- Re-rendering an unchanged vello scene without re-encoding; banded
  `FillRect`s as the alternative gradient workaround.
- Memory use beyond the registered pixmap bytes; build time.
- Any platform other than this Linux/Wayland/Mesa machine.
