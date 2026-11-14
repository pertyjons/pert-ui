# After-measurement of candidate F

Measured 2026-11-13 on Intel UHD Graphics 770 (Mesa 26.2.6, Vulkan), Fedora
45, GNOME on Wayland, Rust 1.100.0: the branch at 0.3.118 against the mask
path at df45a0b (0.3.112, built with its pinned Rust 1.99.1), five rounds,
the two alternating, each run through `quiet-run.sh` (1-minute load below
2.0; the files record it). Medians of the runs' medians, with the range
across runs. The raw output is beside this file: `realtime-base-N.txt`,
`realtime-new-N.txt`, `cpu-base-N.txt` and `cpu-new-N.txt`.

## The wgpu renderer: a changing polyline

`pert-ui-wgpu`'s `native_realtime_workload_probe`, release, 960×640 at
scale 1 with an 800×400 plot; milliseconds per frame. The probe's columns
and waterfall rows did not change (1.21 and 1.65 ms in both).

| Workload | Version | Rasterize | Prepare | GPU pass | Total |
|---|---|---|---|---|---|
| Polyline, 800 points | 0.3.112 | 2.40 | 0.10 | 0.94 | 3.52 (3.40–3.53) |
| Polyline, 800 points | 0.3.118 | 0.00 | 0.16 | 0.99 | 1.22 (1.00–1.23) |
| Polyline, 4096 points | 0.3.112 | 4.40 | 0.07 | 0.86 | 5.56 (3.50–5.59) |
| Polyline, 4096 points | 0.3.118 | 0.00 | 0.30 | 1.27 | 1.78 (1.73–2.07) |

The baseline's rasterization is bimodal across runs, as
`docs/verification.md` records for it: the probe does not pin its thread.
The UI thread's share of a stroke, rasterize plus prepare, went from 2.5 to
0.16 ms and from 4.5 to 0.30 ms; the GPU pass grew by 0.05 and 0.41 ms.

## The CPU renderer

`pert-ui-cpu`'s `stroke_cost` example, release, pinned to one core: the
median render time of a frame less that of the frame without its strokes,
in milliseconds, over an 800×400 plot. The ranges across the five runs are
within 2 % of each median.

| Workload | Scale 1: masks | Scale 1: covered | Scale 2: masks | Scale 2: covered |
|---|---|---|---|---|
| Trace, 800 points | 5.40 | 0.58 | 19.87 | 1.28 |
| Trace, 4096 points | 7.16 | 2.14 | 23.12 | 3.58 |
| Noise, 800 points | 17.40 | 6.00 | 54.79 | 16.47 |
| Noise, 16 384 points | 248.2 | 75.4 | refused | 183.8 |
| Diagram, 60 edges | 49.19 | 1.91 | 193.6 | 5.18 |
| Trace, 800 points, unchanged | 4.72 | 0.57 | 18.48 | 1.28 |
| Ellipses, 60, unchanged | 4.52 | 1.11 | 16.79 | 2.79 |

The unchanged rows render one scene again and again with one renderer, so
the mask path has its coverage cached and pays only for drawing the masks;
the covered path prepares and evaluates the strokes on every frame and is
still faster.

Before `PreparedStroke::for_each_covered` (0.3.117 evaluated every pixel of
a tile against the tile's whole list) the 4096-point trace at scale 1 took
7.95 ms against the masks' 7.16, the one row slower than the baseline; the
other rows were between the two columns above.

## The spec's expectations

| Expectation | Result |
|---|---|
| A changing polyline costs well under a millisecond of UI-thread time | **Met at scale 1:** 0.16 ms (800 points) and 0.30 ms (4096) of preparation and upload. Scale 2 is not measured in the renderer; the probe's preparation took 0.12 and 0.34 ms there |
| The GPU pass does not give the saving back | **Met for the offscreen pass:** it grew by 0.05 and 0.41 ms while the UI thread saved 2.3 and 4.2 ms. A window present is not measured |
| The CPU renderer is not slower | **Met:** 3 to 37 times faster in every row |
| Strokes add no mask resource | **By construction and one check:** `VectorRasterizer::prepare_fills` leaves every stroke a vector (tested), and the real-time probe fails if a polyline frame holds a mask. The diagram editor and the gallery's knob page were not inspected separately |
| No stroke the renderers draw is misplaced by the mask grid | **By construction:** a stroke's span and its pixels are whole frame pixels in both renderers, and the parity corpus holds strokes at sub-pixel phases within one step. RENDER-04 remains for fills and for the fallback's masks |
