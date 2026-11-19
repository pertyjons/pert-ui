<!-- start 2026-11-16T16:08:24+01:00, waited 0 min, load 1.31 3.71 2.76
%CPU COMMAND
 1.2 jetbrains-toolb
 1.0 glycin-svg
 0.8 claude
 0.5 gnome-terminal-
 0.4 gnome-shell
-->
# Phase 1 probe

Definitions are in the probe's module documentation (`plans/rendering/phase1-probe/probe.rs`).

## Gallery pages at scale 1 (1280×800 physical)

Empty scene offscreen render + read-back: median 3.08 ms.

| Page | Rects | Masks drawn | Images | Vectors | Other | Mask resources | Mask KiB | Image KiB | Compose cold ms | (paint/text/geometry self ms) | Compose warm ms | CPU render ms | Present cold ms | Present warm ms | GPU first ms | GPU warm ms (p95) |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| layout | 28 | 43 | 1 | 0 | 23 | 41 | 111 | 1 | 1.64 | 1.53/0.00/0.00 | 0.078 | 5.5 | 1.43 | 0.78 | 8.92 | 6.50 (7.15) |
| splits | 24 | 38 | 1 | 0 | 26 | 36 | 94 | 1 | 1.47 | 1.36/0.00/0.00 | 0.078 | 4.8 | 0.60 | 0.52 | 3.25 | 2.77 (3.37) |
| shells | 28 | 45 | 1 | 0 | 33 | 40 | 116 | 1 | 1.54 | 1.43/0.00/0.00 | 0.071 | 5.6 | 0.68 | 0.42 | 3.27 | 2.89 (3.14) |

Mixer: 0 extra flushes over 20 ticks while waiting for the engine; their time is in compose.

## Mixer with live data

| Scenario | Frames | Compose first ms | Compose median ms (p95) | CPU median ms | Present median ms (p95) | GPU median ms (p95) | Mask resources (median) | Mask KiB (median) | New masks per frame, median (total) | New mask KiB total | Other commands (max) |
|---|---|---|---|---|---|---|---|---|---|---|---|
| mixer: one tick per frame | 20 | 0.02 | 0.05 (0.08) | 1.05 | 0.26 (0.42) | 2.05 (2.22) | 14 | 18 | 0 (0) | 0 | 21 |

Diagram live zoom ends at scale 1.419.

## Diagram editor zoom and pan

| Scenario | Frames | Compose first ms | Compose median ms (p95) | CPU median ms | Present median ms (p95) | GPU median ms (p95) | Mask resources (median) | Mask KiB (median) | New masks per frame, median (total) | New mask KiB total | Other commands (max) |
|---|---|---|---|---|---|---|---|---|---|---|---|
| diagram: live zoom steps | 6 | 0.24 | 0.21 (0.25) | 2.10 | 0.61 (2.18) | 3.76 (3.80) | 29 | 24 | 1 (9) | 7 | 19 |
| diagram: live zoom settle frame | 1 | 0.23 | 0.23 (0.23) | 1.77 | 0.31 (0.31) | 3.54 (3.54) | 28 | 25 | 2 (2) | 3 | 18 |
| diagram: settled zoom in and out | 6 | 0.23 | 0.22 (0.25) | 1.98 | 0.59 (2.03) | 3.66 (3.77) | 27 | 25 | 2 (15) | 18 | 19 |
| diagram: pan 7 px per frame | 10 | 0.12 | 0.12 (0.16) | 1.96 | 0.61 (1.97) | 3.74 (3.81) | 26 | 22 | 0 (0) | 0 | 19 |

Cards, pan: ends at 100.0%.

Cards, pan back: ends at 100.0%.

Cards, zoom in: ends at 123.1%.

Cards, zoom out: ends at 100.0%.

Cards, zoom out further: ends at 81.2%.

## Gallery zoom workload (64 cards)

| Scenario | Frames | Compose first ms | Compose median ms (p95) | CPU median ms | Present median ms (p95) | GPU median ms (p95) | Mask resources (median) | Mask KiB (median) | New masks per frame, median (total) | New mask KiB total | Other commands (max) |
|---|---|---|---|---|---|---|---|---|---|---|---|
| cards: pan steps | 6 | 0.36 | 0.38 (0.48) | 2.70 | 0.39 (1.54) | 2.16 (2.19) | 31 | 53 | 0 (4) | 9 | 40 |
| cards: pan rest frame | 1 | 0.04 | 0.04 (0.04) | 2.73 | 0.32 (0.32) | 2.01 (2.01) | 31 | 53 | 0 (0) | 0 | 40 |
| cards: pan back steps | 6 | 0.35 | 0.36 (0.37) | 2.70 | 0.36 (0.37) | 2.14 (2.17) | 31 | 53 | 0 (0) | 0 | 36 |
| cards: pan back rest frame | 1 | 0.04 | 0.04 (0.04) | 2.39 | 0.31 (0.31) | 2.15 (2.15) | 27 | 43 | 0 (0) | 0 | 36 |
| cards: zoom in steps | 6 | 4.08 | 0.45 (4.08) | 2.75 | 0.36 (0.54) | 2.14 (2.29) | 27 | 47 | 0 (26) | 46 | 36 |
| cards: zoom in rest frame | 1 | 3.90 | 3.90 (3.90) | 2.55 | 0.46 (0.46) | 2.21 (2.21) | 21 | 51 | 21 (21) | 51 | 24 |
| cards: zoom out steps | 6 | 4.06 | 0.52 (4.06) | 2.71 | 0.51 (0.54) | 2.17 (2.30) | 27 | 61 | 3 (26) | 60 | 36 |
| cards: zoom out rest frame | 1 | 4.04 | 4.04 (4.04) | 2.59 | 0.73 (0.73) | 2.45 (2.45) | 27 | 43 | 27 (27) | 43 | 36 |
| cards: zoom out further steps | 6 | 4.13 | 0.45 (4.13) | 2.60 | 0.40 (0.75) | 2.06 (2.37) | 31 | 49 | 0 (30) | 48 | 40 |
| cards: zoom out further rest frame | 1 | 4.11 | 4.11 (4.11) | 2.36 | 0.62 (0.62) | 2.20 (2.20) | 31 | 35 | 30 (30) | 34 | 40 |

## Knob

| Scenario | Frames | Compose first ms | Compose median ms (p95) | CPU median ms | Present median ms (p95) | GPU median ms (p95) | Mask resources (median) | Mask KiB (median) | New masks per frame, median (total) | New mask KiB total | Other commands (max) |
|---|---|---|---|---|---|---|---|---|---|---|---|
| knob: gallery input page, Shift+Up (whole display list) | 20 | 1.49 | 1.06 (1.26) | 3.54 | 0.45 (0.59) | 3.60 (3.71) | 38 | 70 | 3 (60) | 196 | 45 |
| knob: isolated, no readout, host writes | 20 | 0.00 | 0.01 (0.03) | 0.09 | 0.23 (0.38) | 0.86 (0.91) | 0 | 0 | 0 (0) | 0 | 2 |

j1-page: shells
heaviest-page: shells

## J1 change scenarios

| Scenario | Frames | Compose first ms | Compose median ms (p95) | CPU median ms | Present median ms (p95) | GPU median ms (p95) | Mask resources (median) | Mask KiB (median) | New masks per frame, median (total) | New mask KiB total | Other commands (max) |
|---|---|---|---|---|---|---|---|---|---|---|---|
| J1 `shells`: resize +7 px per frame | 5 | 0.76 | 0.98 (1.71) | 4.52 | 1.84 (1.94) | 6.94 (31.99) | 40 | 118 | 0 (1) | 49 | 33 |
| J1 `shells`: scale 1 → 1.25 → 1.5 → 1.75 → 2 | 4 | 3.29 | 4.22 (4.48) | 16.56 | 4.91 (5.48) | 11.46 (14.21) | 40 | 352 | 40 (160) | 1250 | 33 |
<!-- end 2026-11-16T16:08:29+01:00, load 1.21 3.64 2.74, highest during the run 1.31
%CPU COMMAND
 1.2 jetbrains-toolb
 1.0 glycin-svg
 0.8 claude
 0.5 gnome-terminal-
 0.4 gnome-shell
-->
