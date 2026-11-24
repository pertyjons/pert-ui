# Gradient integration measurements

On 2026-11-24 the Linux prototype used the production CPU renderer and wgpu
on Intel UHD Graphics 770 with Vulkan. At measurement time the gallery showed a Gradients page
and a resizable Out-1 window with a 40-logical-pixel title gradient. The
application opts into the title paint. Both demonstrations can be opened
directly with `--page gradients` or `--page windows`.

## Pixel tests

The facade test `title_gradient_keeps_forty_logical_pixels_on_resize_and_clip`
reads a real window's rendered title at scales 1, 1.25 and 2, before and after
resizing its width from 320 to 500 and when the viewport cuts into the first
40 logical pixels. It checks pixel-center coordinates against expected sRGB
bytes. The runtime tests check the exact outgoing side of a repeated seam
and duplicated stops, including diagonal and circular boundaries. The
native CPU/GPU corpus checks the same boundary geometries as well as shape
fills, borders, clipping, alpha and interpolation spaces. The largest
gradient channel difference was one byte. These measurements cover this
adapter and the tested cases.

## Uploads

The isolated queue instrumentation counted each write and hashed its contents
per category in call order. Comparing reference 3448065 and candidate a85b6d0
gave equal byte counts and hashes on every sampled frame. The table gives
the sum of application bytes sent in each scenario.

| Scenario | Frames | Vertices | Shape records | Stroke records | Texture data | Gradient data |
|---|---:|---:|---:|---:|---:|---:|
| Mixer ticks | 300 | 656620 | 3024 | 0 | 18820 | 0 |
| Diagram live zoom | 30 | 1647648 | 23200 | 18684 | 99043 | 0 |
| Diagram zoom settle | 1 | 0 | 0 | 0 | 0 | 0 |
| Diagram pan | 60 | 11556884 | 64248 | 29816 | 60180 | 0 |
| Diagram settled zoom in/out | 16 | 909504 | 14912 | 10020 | 80488 | 0 |

A separate hardware test draws two shapes sharing the same pair of stops.
The initial render uploaded 96 bytes of placement and 96 bytes for the ramp.
Moving one fill by four pixels issued a four-byte placement write and zero
stop bytes. The test checks the ranges submitted to the real wgpu queue.

To reproduce the solid upload comparison from the repository root:

```sh
PROBE_UPLOADS=1 PROBE_ONLY=mixer-diagram RUNS=1 PIN=6,8 plans/rendering/phase1-probe/run.sh target/gradient-uploads baseline=3448065 candidate=a85b6d0
python3 plans/rendering/gradient-probe/compare.py target/gradient-uploads/{baseline,candidate}/run-1.md
```

Use uninstrumented binaries for timing. `paired.py REFERENCE CANDIDATE OUTPUT`
runs ten order-balanced pairs and records each sample. For a repeatability
control pass the same reference binary twice. `summarize.py OUTPUT` reports
the second, median and ninth ordered ratios for every scenario and metric.
The present column is host submission time. The GPU column includes offscreen
completion and readback. The single zoom-settle frame is descriptive.

## Timing under shared machine load

Ten uninstrumented pairs (reference 3448065, candidate 3d52030, cores 6 and
8) overlapped development builds. Median candidate/reference CPU time was
1.059 for the mixer and 1.000 for diagram pan. For GPU time it was 1.011
and 1.000 respectively. For presentation it was 1.029 and 0.963. The
second and ninth sorted ratios for mixer CPU were 0.961 and 1.837, so this
run cannot resolve the proposed 5% threshold. Four attempts in an earlier
same-binary run produced no samples because mixer ticks timed out.

## Quiet rerun and decision

The quiet runner admitted the series at a 1-minute load of 0.91 and
recorded a maximum of 1.00 over both series. On cores 6 and 8 it completed
ten same-binary pairs followed by ten pairs against a85b6d0. The reference
was 3448065. [Paired medians in milliseconds](results/integration-timing.tsv)
retain CPU, offscreen GPU and host presentation for each individual run.

For each multi-frame scenario the second and ninth same-binary CPU and
GPU ratios fell within 0.98–1.02. The table gives the median production
candidate/reference ratio and the ninth ordered ratio in parentheses. A
ninth ratio below 1.05 clears the 5% criterion.

| Scenario | CPU | Offscreen GPU | Host presentation |
|---|---:|---:|---:|
| Mixer ticks | 1.003 (1.005) | 0.998 (1.009) | 1.006 (1.028) |
| Diagram pan | 1.006 (1.012) | 1.000 (1.009) | 1.000 (1.014) |
| Diagram live zoom | 1.017 (1.029) | 0.996 (1.003) | 0.999 (1.066) |
| Diagram settled zoom | 1.004 (1.016) | 1.005 (1.025) | 0.969 (1.072) |

CPU and offscreen GPU meet the render-time gate on these workloads and this
adapter. Presentation meets it for the mixer and pan. For both zoom scenarios
the presentation interval crosses 1.05, and settled-zoom presentation
also missed the 2% same-binary repeatability target (median 1.045). The
hardware evidence supports the gradient prototype. Keep the branch separate
while a strict zoom-presentation bound remains unresolved. Conic geometry
is deferred.
