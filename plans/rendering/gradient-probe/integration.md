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

## Targeted zoom presentation check

This follow-up times the host call to present a frame on the native surface
for solid diagram scenes. It retains the original frame pacing, CPU render
and offscreen readback between successive presentations. Each binary runs
the same live and settled zoom trajectories once to warm up, then five
times from fresh editors. This yields 150 live and 80 settled presentations
per process. A skipped presentation invalidates that run. The isolated
frame that ends a live gesture is outside the live-step statistic.

The comparison uses reference 3448065 (before integration) and 11ddcfc
(the prototype with the gallery examples). Run the reference against itself
first, then compare reference and candidate. Each series has ten independent
process pairs with alternating order on cores 6 and 8. For each scenario,
take the ratio of the per-process median presentation times in each pair.
Use the previously agreed 2% same-binary repeatability target and the
second and ninth ordered candidate/reference ratios for the 5% gate. A
ninth ratio below 1.05 passes, a second ratio above 1.05 fails, and a range
straddling 1.05 leaves the bound unresolved.

From the repository root on the measuring desktop:

```sh
RUNS=0 plans/rendering/phase1-probe/run.sh target/zoom-presentation reference=3448065 final=11ddcfc
p=target/phase1-probe
s=plans/rendering/gradient-probe/paired.py
plans/rendering/tools/quiet-run.sh target/zoom-presentation/quiet.log sh -ec '
  python3 "$1" "$2" "$2" target/zoom-presentation/aa --workload zoom-present
  python3 "$1" "$2" "$3" target/zoom-presentation/ab --workload zoom-present
' sh "$s" "$p/target-reference/release/phase1-probe" "$p/target-final/release/phase1-probe"
python3 plans/rendering/gradient-probe/summarize.py target/zoom-presentation/aa
python3 plans/rendering/gradient-probe/summarize.py target/zoom-presentation/ab
```

**Result, 2026-11-24.** Both zoom scenarios pass the native host presentation
gate on this Linux desktop. Each process produced the full expected number of
presented frames. In the table, each percentage is a change relative to the
reference within a pair. The upper end of the last column is the ninth smallest change across
the ten candidate/reference pairs, the decision statistic.

| Scenario | Same-binary median change | Candidate median change | Candidate second to ninth |
|---|---:|---:|---:|
| Live zoom | +1.38% | +0.36% | −1.89% to +3.09% |
| Settled zoom | −0.94% | +1.90% | −0.40% to +4.31% |

The second-to-ninth same-binary ranges were −3.74% to +2.80% for live zoom
and −1.75% to +1.08% for settled zoom. Both control medians meet the 2%
repeatability target, with greater dispersion in live zoom. The quiet runner
started at load 1.37 and sampled a peak of 2.13. The system was shared.
The median of the individual process times moved from 0.517 to 0.520 ms
for live zoom and from 0.527 to 0.543 ms for settled zoom. Those absolute
times are medians across processes; the decision uses the paired ratios.
[Raw process medians and frame counts](results/zoom-presentation.tsv).

This resolves the zoom presentation uncertainty above. The performance
measurements support merging the linear and radial gradient implementation
for the measured workloads. Other adapters and platforms require their own
native evidence.
