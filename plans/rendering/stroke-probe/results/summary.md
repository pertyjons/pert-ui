# Slice 0 results: the gate is not met

The tile lists, measured here. The spec's alternative was measured next:
[quads/summary.md](quads/summary.md). The probe's preparation has changed
since (no `hypot`, directions computed once), so these prepare times are
those of 75ba447; their GPU rows ran at a clock this file did not record,
which [quads/summary.md](quads/summary.md#the-gpu-clock) explains.

Measured 2026-11-12 on Intel UHD Graphics 770 (Mesa 26.2.6, Vulkan), the
library at 75ba447 (0.3.112), the probe pinned to CPU 2 through
`quiet-run.sh` (1-minute load 0.95–1.6 during the runs). Milliseconds per
frame, medians of the runs' medians. The raw tables are beside this file:

- `check.md`, `compare.md`, `time-1.md` to `time-5.md`: `run.sh`, tile 16.
- `realtime-1.txt` to `realtime-3.txt`: today's path in `pert-ui-wgpu`
  (`native_realtime_workload_probe`), the reference for the two polylines.
- `tiles/`: tiles 8, 16 and 32, three rounds, without today's path.
- `execution/`: tiles 16 and 8, three rounds, with GPU timestamps; today's
  CPU renderer left out.

## Gate

| Condition | Result |
|---|---|
| Prepare + GPU pass ≤ 1.0 ms for both probe polylines, and below today's path | **Not met.** At scale 1, tile 16: 1.49 ms (800 points) and 3.46 ms (4096 points); tile 8: 2.07 and 2.51 ms. Today in `pert-ui-wgpu`: 2.25–2.30 and 3.27–3.33 ms |
| No workload slower than today | **Not met.** The 4096-point polyline at scale 1 (tile 16) and an unchanged polyline (both tile sizes) are slower |
| Edge tier within four steps | **Met.** No decided pixel differs; the largest edge difference is 0.001 of a step, over 2 436 corpus cases |

## Where the time goes

The preparation is cheap; the fragment stage is not. GPU execution from
the pass's timestamps, tile 16, against today's mask pass in the probe's
model:

| Workload, scale 1 | Prepare | GPU execution | Today: rasterize + prepare | Today: GPU execution |
|---|---|---|---|---|
| Polyline, 800 points | 0.15 | 0.83 | 1.50 | 0.10 |
| Polyline, 4096 points | 0.54 | 2.24 | 2.41 | 0.10 |
| Noise, 800 points | 0.28 | 3.19 | 7.43 | 0.19 |
| Steep alternation, 400 points | 0.31 | 4.50 | 7.85 | 0.14 |
| Noise, 16 384 points | 4.65 | 30.5 | 244.8 | 0.11 |
| Polyline, unchanged | 0 | 1.56 | 0 | 0.16 |

Each pixel of a tile evaluates every segment its list holds, so the work is
the tiles' area times their lists, most of it far from the line: a 1.5-pixel
trace covers a small share of a 16-pixel tile. Tile 8 lowers the execution
for long lists (4096 points: 1.11 ms) but raises it for short ones and
doubles the preparation. A submit-and-wait adds about 0.3–0.5 ms on the
host to every pass, new or today's.

Totals still favor the new path where today's rasterization is large: dense
noise, steep alternation, the diagram, and scale 2 (800 points: 2.19 ms
against 4.81). Today's path refuses the 16 384-point noise at scale 2
(`TooComplex`); the prototype draws it in 63 ms.

## The CPU side

The prototype's CPU evaluation is faster than today's CPU renderer in every
workload (800 points at scale 1: 2.5 against 8.8 ms; the diagram at scale
2: 10 against 354 ms). The comparison is approximate: today's cost is the
renderer's frame with the strokes minus an empty frame.

## Coverage against today's

`compare.md`: 50–56 % of the touched pixels lie within one step of today's
coverage, 71 % for points and reversals. The largest differences, up to
about 145 steps, are today's RENDER-04 error: a mask's first or last row
drawn whole at a pixel the geometry covers only partly. A horizontal edge
on a pixel's center, for example, is 0.5 in the new coverage, its true
area, and 1.0 today.

## A prototype error found and fixed

The first `compare` run showed full-pixel differences in the reversal
cases. The prototype took a bevel's outer bisector from the sum of the two
normals, which at a reversal is rounding noise when the directions are
rounded (0.9999999999999999 against -1); the bevel line then cut the body
away. It now takes the bisector from the difference of the directions for
turns of 90 degrees or more, as union.rs places a round join's, and
`check.md` and `compare.md` were run again. No timed workload has a bevel or
miter turn of 90 degrees or more, so the timings stand. The spec states the
rule for slice 1.
