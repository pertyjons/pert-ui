<!-- start 2026-11-16T16:08:20+01:00, waited 0 min, load 1.43 3.77 2.77
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

Empty scene offscreen render + read-back: median 3.14 ms.

| Page | Rects | Masks drawn | Images | Vectors | Other | Mask resources | Mask KiB | Image KiB | Compose cold ms | (paint/text/geometry self ms) | Compose warm ms | CPU render ms | Present cold ms | Present warm ms | GPU first ms | GPU warm ms (p95) |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| layout | 28 | 43 | 1 | 0 | 23 | 41 | 111 | 1 | 1.54 | 1.43/0.00/0.00 | 0.071 | 5.5 | 1.55 | 0.78 | 4.49 | 3.10 (3.81) |
| splits | 24 | 38 | 1 | 0 | 26 | 36 | 94 | 1 | 1.39 | 1.29/0.00/0.00 | 0.070 | 5.0 | 0.62 | 0.49 | 5.18 | 2.98 (3.00) |
| shells | 28 | 45 | 1 | 0 | 33 | 40 | 116 | 1 | 1.51 | 1.40/0.00/0.00 | 0.088 | 5.6 | 0.69 | 0.39 | 3.90 | 3.02 (3.10) |

Mixer: 0 extra flushes over 20 ticks while waiting for the engine; their time is in compose.

## Mixer with live data

| Scenario | Frames | Compose first ms | Compose median ms (p95) | CPU median ms | Present median ms (p95) | GPU median ms (p95) | Mask resources (median) | Mask KiB (median) | New masks per frame, median (total) | New mask KiB total | Other commands (max) |
|---|---|---|---|---|---|---|---|---|---|---|---|
| mixer: one tick per frame | 20 | 0.04 | 0.07 (0.10) | 1.06 | 0.41 (0.45) | 2.21 (2.31) | 14 | 18 | 0 (0) | 0 | 21 |

Diagram live zoom ends at scale 1.419.

## Diagram editor zoom and pan

| Scenario | Frames | Compose first ms | Compose median ms (p95) | CPU median ms | Present median ms (p95) | GPU median ms (p95) | Mask resources (median) | Mask KiB (median) | New masks per frame, median (total) | New mask KiB total | Other commands (max) |
|---|---|---|---|---|---|---|---|---|---|---|---|
| diagram: live zoom steps | 6 | 0.24 | 0.24 (0.25) | 2.14 | 0.77 (1.74) | 3.86 (3.98) | 29 | 24 | 1 (9) | 7 | 19 |
| diagram: live zoom settle frame | 1 | 0.25 | 0.25 (0.25) | 1.80 | 0.55 (0.55) | 3.70 (3.70) | 28 | 25 | 2 (2) | 3 | 18 |
| diagram: settled zoom in and out | 6 | 0.24 | 0.24 (0.30) | 2.11 | 0.69 (1.82) | 3.88 (3.99) | 27 | 25 | 2 (15) | 18 | 19 |
| diagram: pan 7 px per frame | 10 | 0.11 | 0.16 (0.19) | 1.97 | 0.78 (1.62) | 3.81 (6.13) | 26 | 22 | 0 (0) | 0 | 19 |

Cards, pan: ends at 100.0%.

Cards, pan back: ends at 100.0%.

Cards, zoom in: ends at 123.1%.

Cards, zoom out: ends at 100.0%.

Cards, zoom out further: ends at 81.2%.

## Gallery zoom workload (64 cards)

| Scenario | Frames | Compose first ms | Compose median ms (p95) | CPU median ms | Present median ms (p95) | GPU median ms (p95) | Mask resources (median) | Mask KiB (median) | New masks per frame, median (total) | New mask KiB total | Other commands (max) |
|---|---|---|---|---|---|---|---|---|---|---|---|
| cards: pan steps | 6 | 0.36 | 0.41 (0.55) | 2.70 | 0.54 (1.37) | 2.37 (2.53) | 31 | 53 | 0 (4) | 9 | 40 |
| cards: pan rest frame | 1 | 0.05 | 0.05 (0.05) | 2.71 | 0.49 (0.49) | 2.32 (2.32) | 31 | 53 | 0 (0) | 0 | 40 |
| cards: pan back steps | 6 | 0.36 | 0.44 (0.45) | 2.69 | 0.55 (0.60) | 2.39 (2.46) | 31 | 53 | 0 (0) | 0 | 36 |
| cards: pan back rest frame | 1 | 0.05 | 0.05 (0.05) | 2.45 | 0.47 (0.47) | 2.22 (2.22) | 27 | 43 | 0 (0) | 0 | 36 |
| cards: zoom in steps | 6 | 4.12 | 0.52 (4.12) | 2.77 | 0.55 (0.73) | 2.44 (2.54) | 27 | 47 | 0 (26) | 46 | 36 |
| cards: zoom in rest frame | 1 | 4.01 | 4.01 (4.01) | 2.60 | 0.69 (0.69) | 2.55 (2.55) | 21 | 51 | 21 (21) | 51 | 24 |
| cards: zoom out steps | 6 | 4.14 | 0.53 (4.14) | 2.76 | 0.56 (0.66) | 2.43 (2.50) | 27 | 61 | 3 (26) | 60 | 36 |
| cards: zoom out rest frame | 1 | 4.05 | 4.05 (4.05) | 2.59 | 0.74 (0.74) | 2.43 (2.43) | 27 | 43 | 27 (27) | 43 | 36 |
| cards: zoom out further steps | 6 | 4.29 | 0.53 (4.29) | 2.60 | 0.61 (1.03) | 2.36 (2.44) | 31 | 49 | 0 (30) | 48 | 40 |
| cards: zoom out further rest frame | 1 | 4.16 | 4.16 (4.16) | 2.37 | 0.80 (0.80) | 2.53 (2.53) | 31 | 35 | 30 (30) | 34 | 40 |

## Knob

| Scenario | Frames | Compose first ms | Compose median ms (p95) | CPU median ms | Present median ms (p95) | GPU median ms (p95) | Mask resources (median) | Mask KiB (median) | New masks per frame, median (total) | New mask KiB total | Other commands (max) |
|---|---|---|---|---|---|---|---|---|---|---|---|
| knob: gallery input page, Shift+Up (whole display list) | 20 | 0.87 | 1.12 (1.16) | 3.52 | 0.59 (0.62) | 4.18 (4.27) | 38 | 70 | 3 (60) | 196 | 45 |
| knob: isolated, no readout, host writes | 20 | 0.00 | 0.03 (0.03) | 0.09 | 0.37 (0.41) | 0.90 (1.04) | 0 | 0 | 0 (0) | 0 | 2 |

j1-page: shells
heaviest-page: shells

## J1 change scenarios

| Scenario | Frames | Compose first ms | Compose median ms (p95) | CPU median ms | Present median ms (p95) | GPU median ms (p95) | Mask resources (median) | Mask KiB (median) | New masks per frame, median (total) | New mask KiB total | Other commands (max) |
|---|---|---|---|---|---|---|---|---|---|---|---|
| J1 `shells`: resize +7 px per frame | 5 | 0.76 | 0.96 (1.75) | 4.44 | 1.49 (1.72) | 7.28 (32.35) | 40 | 118 | 0 (1) | 49 | 33 |
| J1 `shells`: scale 1 → 1.25 → 1.5 → 1.75 → 2 | 4 | 3.28 | 4.23 (4.51) | 16.26 | 4.57 (5.67) | 12.19 (15.22) | 40 | 352 | 40 (160) | 1250 | 33 |
<!-- end 2026-11-16T16:08:24+01:00, load 1.31 3.71 2.76, highest during the run 1.43
%CPU COMMAND
 1.2 jetbrains-toolb
 1.0 glycin-svg
 0.8 claude
 0.5 gnome-terminal-
 0.4 gnome-shell
-->
