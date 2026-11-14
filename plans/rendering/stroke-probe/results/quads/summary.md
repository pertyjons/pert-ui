# The alternative measured: per-segment quads into a coverage atlas

Measured 2026-11-13 on Intel UHD Graphics 770 (Mesa 26.2.6, Vulkan), the
library at 857af2d (0.3.112), `PROBE_PATH=quads run.sh`, five pinned runs
through `quiet-run.sh` (1-minute load below 0.5). Milliseconds per frame,
medians of the runs' medians; `time-1.md` to `time-5.md` and `check.md` are
beside this file. `compare` is unchanged from [slice 0](../summary.md).

## The path

Each segment draws one oriented quad (a second around a miter join's
wedge) whose fragments write the segment's coverage into an R8 atlas with a
maximum blend; the maximum of the segments' coverage is the coverage of
their smallest distance, the spec's formula. Each stroke's span has a
region of the atlas (shelf packing; a 2048-pixel atlas holds every
workload's strokes in one batch except the diagram's, two at scale 1 and
eight at scale 2). The preparation records no lists, only the cells a
segment's reach touches, merged into runs; the composite draws those runs
from the atlas. While the strokes do not change the atlas keeps their
coverage and the frame draws only the composite.

What made it fast enough, each measured in the same frames as before it:

- the segment's record and its stroke's values travel to the fragment stage
  as flat varyings instead of being read from storage buffers per pixel,
  in the composite too: about 2.5 times less GPU execution;
- one quad per segment, the round or bevel join's wedge inside it: the
  atlas stage of the 4096-point trace halved (Gen12 shades each small
  triangle in threads of its own);
- cells instead of lists, `f64` lengths without `hypot` and each direction
  computed once: prepare and upload of the 4096-point trace from 0.54 to
  about 0.29 ms.

In the same frames, a smoke run before these changes put this path's GPU
execution 2 to 6 times below the tile lists'.

## The GPU clock

The adapter's clock follows its load: 300 to 650 MHz for light frames,
1550 MHz only for the 16 384-point noise. The 800-point trace ran at 300 MHz
in four of the five runs and at 650 MHz in one (GPU 1.02 ms instead of
1.38). `time` now draws the new path and today's in the same frames, so a
row compares them at the same clock; rows of different runs, including
slice 0's tile measurements, are not comparable this way.

## Gate

The first gate, which this path does not meet. The maintainer revised it on
2026-11-13 after these results, and this path meets the
[revised gate](../../../stroke-spec.md#slice-0-measure-before-building):
below today's path for both polylines at both scales, the knob 0.07 ms
above it within the 0.1 ms allowed for a second render pass.

| Condition | Result |
|---|---|
| Prepare + GPU pass ≤ 1.0 ms for both probe polylines, and below today's path | **Below today's, not within 1.0 ms.** 1.47 ms (800 points) and 1.50 ms (4096) at scale 1, against today's 2.45 and 3.31 ms in the same frames |
| No workload slower than today in total, nor in the CPU renderer | **All but the knob.** The knob's single short stroke takes 0.49 ms against 0.42 (0.56 against 0.49 at scale 2): the atlas pass is a second render pass. An unchanged trace is level (0.77 against 0.76). The CPU side is faster in every workload |
| Edge tier within four steps | **Met.** No decided pixel differs; the largest edge difference is 0.51 of a step, the atlas's 8-bit rounding |

Most of the GPU interval is not execution: the 800-point trace's pass takes
1.38 ms on the host and 0.43 ms on the GPU, and today's 0.97 and 0.21 ms
for the same frame. Uploading, submitting and waiting cost about 0.4 ms for
the knob's nearly empty frame and more with data to copy at 300 MHz. Today's
GPU pass for the 800-point trace alone is about as long as the 1.0 ms bound.

## Totals

Prepare plus GPU pass, against today's rasterize plus prepare plus GPU
pass; GPU execution in parentheses.

| Workload | Scale 1: new | Scale 1: today | Scale 2: new | Scale 2: today |
|---|---|---|---|---|
| Trace, 800 points | 1.47 (0.43) | 2.45 (0.21) | 2.32 (1.07) | 5.66 (0.77) |
| Trace, 4096 points | 1.50 (0.30) | 3.31 (0.10) | 3.28 (1.63) | 6.89 (0.78) |
| Noise, 800 points | 1.47 (0.47) | 8.30 (0.19) | 4.36 (3.05) | 18.41 (1.39) |
| Steep alternation, 400 points | 1.49 (0.50) | 8.70 (0.14) | 6.48 (5.11) | 20.81 (1.93) |
| Noise, 16 384 points | 9.06 (4.88) | 246.84 (0.24) | 12.62 (6.93) | refused |
| Diagram, 60 edges | 1.50 (0.42) | 5.67 (1.03) | 2.58 (1.17) | 15.77 (3.78) |
| Knob | 0.49 (0.08) | 0.42 (0.02) | 0.56 (0.09) | 0.49 (0.03) |
| Trace, unchanged | 0.77 (0.17) | 0.76 (0.21) | 1.65 (0.68) | 1.64 (0.82) |

At scale 2 the composite of a stroke that covers its whole span costs more
than today's mask quad over the same span (noise, 800 points: 2.36 against
1.39 ms); why is not established. It does not change a total.

## After a review of the shaders

Codex reviewed the quads path's shaders and draw setup read-only
(2026-11-13). It found two coverage bounds that were too small; both were
reproduced on the GPU with new corpus cases and repaired, and `check.md` is
regenerated with them:

- a round cap's disc reaches behind the other, joined end of a short
  segment: the body quad now extends by the reach at both ends of a segment
  with a round cap;
- the miter square's half-size was recomputed in the vertex stage from
  `1 + dot(d, n)`, which cancels near a reversal: the record now carries the
  preparation's `f64` reach in the miter's unused end `k`.

Of the simplifications tried, each as one change in two pinned rounds
against the repaired path, two are kept because the code gets shorter:

| Change | Effect on GPU execution | Kept |
|---|---|---|
| Triangle strips of four vertices instead of lists of six | Atlas stage 4 % (800 points) to 10 % (4096 points) shorter for the traces at 300 MHz, level elsewhere | Yes |
| No `discard` of uncovered atlas fragments (the maximum blend ignores zero) | Atlas stage about 3 % shorter for the noise and the steep alternation at scale 1, level elsewhere | Yes |
| `discard` of zero coverage in the composite | None | No |
| One composite rectangle when every cell of the span is marked | Composite 12 % shorter for the steep alternation at scale 1 only | No |

The timing files above predate these changes; none moves a total by more
than about 0.04 ms, and the gate's result is the same. The scale-2
composite's extra cost remains unexplained: one rectangle instead of cell
runs does not change it, and neither does disabling Mesa's color
compression (`INTEL_DEBUG=noccs`), under which the composite stays about
1.5 times today's mask pass.
