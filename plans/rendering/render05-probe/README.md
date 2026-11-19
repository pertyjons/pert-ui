# RENDER-05: the offscreen wgpu regression after phase 1

Measurement record for the `TODO.md` entry RENDER-05, 2026-11-16, on the
machine of the phase 1 probe (Fedora 45, GNOME/Wayland, Intel UHD Graphics
770, Mesa 26.2.6, Vulkan), every run pinned to core 2 through
`plans/rendering/tools/quiet-run.sh` (highest load during a run: 1.67). The
harness is `plans/rendering/phase1-probe/` unchanged except for
`PROBE_ONLY=cards`, which runs only the gallery's zoom workload. `gpu_*` is
`Renderer::render_offscreen` with read-back after a present, so the scene
is already prepared; `present_*` is UI-thread time of `Renderer::render`.

The variants were working-tree snapshots (`git stash create`), listed by
label and hash in each `checkouts.txt`; those commits are unreferenced, so
Git may prune them, and the descriptions below are the record of what they
held. `before` is 38a9cf9, the
checkout before phase 1, and `head` is 5b35dfc (0.3.119).

## Cause

Fragment work in the shape shader accounts for the regression in the
measured scenarios: removing it brings them back to their level before
phase 1. In
`results/variants-1` (quick mode, three runs), a diagnostic variant whose
fragment stage returned the fill without evaluating anything (`const`)
brought the knob page from 4.51 to 3.61 ms and the J1 scale series from
13.5 to 11.5 ms, the level before phase 1 (3.67 and 10.7 ms in a quick
baseline run that is not kept). Phase 1 draws panels' and cards' interiors
(`solid_box`) through the shape shader, where before they were `FillRect`s
through the trivial fill shader; moving them back removed most of it. Mesa compiles the shape shader (453 instructions) to SIMD8, 16
and 32 without spills, so its width is not the limit.

## What was tried

| Variant | Change | Outcome |
|---|---|---|
| `flat` | The vertex stage reads the record and passes it as flat varyings; the fragment stage reads no storage | About half of the page regression back (knob 4.51 → 4.13 ms); with many tiny quads, zoomed-out cards 14 to 15 % slower than `head` (`results/cards`) |
| `flat` + boxes | Also a solid box of at least 4096 pixels drawn as a fill quad (label `flat` in `results/full-3`, `fix` in `results/cards`) | Pages +2.3 % against `before` in median, but the zoomed-out cards stayed slower: the flat varyings, not the boxes, cost there |
| `storage` (kept) | `head`'s shader without the solid-box branch; a solid box of at least 4096 pixels is a fill quad, a smaller rectangle one record over its span | Best overall; below |
| `every` | `storage` with every solid box a fill quad | Zoomed-out cards up to 6 % slower on the GPU and 11 % slower to present than with the threshold (`results/cards-threshold`) |

## Result (`results/full-3`, five alternating runs)

Ratios of medians against `before`, over the gallery's pages at three
scales (`gpu_warm_median_ms`, `gpu_first_ms`) and the frame series
(`gpu_median_ms`):

| Metric | `head` median (max) | `storage` median (max) |
|---|---|---|
| Page, warm | 1.063 (1.197) | 1.016 (1.185) |
| Page, first | 1.135 (1.269) | 1.037 (1.663) |
| Series | 1.058 (1.231) | 1.025 (1.067) |
| Present, page warm | 0.973 (1.005) | 1.001 (1.050) |

The knob page goes from 4.52 to 3.88 ms (3.67 before), the J1 scale series
from 11.66 to 10.04 ms (9.79 before), and the zoomed-out cards stay at
`head`'s level. By the phase 1 slowdown rule, `head` has 211 GPU rows
slower than `before` and `storage` 20, of which 11 are first renders.

What remains: the cards workload's pans and zooms are up to 7 % slower than
before phase 1; a suspected cause, not isolated, is the per-pixel
evaluation of many small shapes, which were cached masks before. The first
render and a few pages at scale 1.25 are bimodal (about 5 or 8 ms for the
first render) in every checkout, `before` included, so single rows there
are not evidence. The page present time is at `before`'s level, about 3 %
above `head`'s, and the diagram pan's present median is 0.84 ms (0.72
before, 0.80 at `head`); the boxes' extra vertices to encode are the
suspected cause.

## 2026-11-19: shapes prepared once on the CPU

The same harness on the same machine, `before` 38a9cf9 and `head` 3721c83
(0.3.121). `results/2026-11-19/full` (three rounds, GPU included) also holds
`now`, the working tree with the fill masks rasterized on the frame's pixels
(0.3.122, RENDER-04): its medians are within 1.5 % of `head` in every metric,
so that change costs nothing in these workloads. Three rounds are below the
summarizer's five, and the first renders at scale 1.25 are bimodal as before,
so single rows there are not evidence.

`results/2026-11-19/cpu` (five rounds, `PROBE_GPU=0`) compares `fix`, the
working tree where `CpuRenderer` draws each shape from the preparation its
validation made instead of preparing it twice. Other builds ran on other
cores during some rounds (highest load 1.89; the probe was pinned to core
2), and the spread within a label stayed below 3 %.

| CPU frame, median ms (range) | `before` | `head` | `fix` |
|---|---|---|---|
| Mixer, one tick per frame | 0.920 (0.916–0.923) | 1.031 (1.028–1.053) | 1.029 (1.025–1.032) |
| Cards, zoomed out further, steps | 1.658 (1.645–1.668) | 1.733 (1.727–1.784) | 1.724 (1.718–1.737) |
| Diagram, pan | 2.844 (2.841–2.870) | 1.894 (1.884–1.932) | 1.875 (1.866–1.879) |

Over all scenarios `fix` is 0.3 % faster than `head` in median (between 3 %
faster and 2 % slower). Preparing shapes twice is therefore not what makes
the mixer frame 0.11 ms slower than before phase 1: preparation is a small
part of a shape's cost and its per-pixel evaluation the rest. The single
preparation is kept as the simpler code. The mixer row is the only CPU row
slower than `before` by the phase 1 rule. Not tried: a cheaper evaluation of
strips without corners, and, for the GPU, retaining encoded geometry across
a pure translation. In the full run the diagram pan's present median is 0.82
to 0.84 ms (0.72 before) and the cards' GPU steps are about 5 % above
`before`, as recorded above.
