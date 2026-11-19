<!-- start 2026-11-16T16:08:11+01:00, waited 0 min, load 1.12 3.75 2.76
%CPU COMMAND
 1.2 jetbrains-toolb
 1.1 glycin-svg
 0.8 claude
 0.5 gnome-terminal-
 0.4 gnome-shell
-->
# Phase 1 probe

Definitions are in the probe's module documentation (`plans/rendering/phase1-probe/probe.rs`).

## Gallery pages at scale 1 (1280×800 physical)

Empty scene offscreen render + read-back: median 3.18 ms.

| Page | Rects | Masks drawn | Images | Vectors | Other | Mask resources | Mask KiB | Image KiB | Compose cold ms | (paint/text/geometry self ms) | Compose warm ms | CPU render ms | Present cold ms | Present warm ms | GPU first ms | GPU warm ms (p95) |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| layout | 28 | 43 | 1 | 0 | 23 | 41 | 111 | 1 | 1.65 | 1.52/0.00/0.00 | 0.075 | 5.3 | 1.44 | 0.74 | 7.87 | 3.95 (5.95) |
| splits | 24 | 38 | 1 | 0 | 26 | 36 | 94 | 1 | 1.44 | 1.34/0.00/0.00 | 0.079 | 4.8 | 0.62 | 0.53 | 3.05 | 2.93 (3.28) |
| shells | 28 | 45 | 1 | 0 | 33 | 40 | 116 | 1 | 1.50 | 1.38/0.00/0.00 | 0.081 | 5.6 | 0.69 | 0.48 | 3.59 | 3.01 (3.59) |

Mixer: 0 extra flushes over 20 ticks while waiting for the engine; their time is in compose.

## Mixer with live data

| Scenario | Frames | Compose first ms | Compose median ms (p95) | CPU median ms | Present median ms (p95) | GPU median ms (p95) | Mask resources (median) | Mask KiB (median) | New masks per frame, median (total) | New mask KiB total | Other commands (max) |
|---|---|---|---|---|---|---|---|---|---|---|---|
| mixer: one tick per frame | 20 | 0.02 | 0.07 (0.10) | 1.08 | 0.38 (0.45) | 2.78 (3.28) | 14 | 18 | 0 (0) | 0 | 21 |

Diagram live zoom ends at scale 1.419.

## Diagram editor zoom and pan

| Scenario | Frames | Compose first ms | Compose median ms (p95) | CPU median ms | Present median ms (p95) | GPU median ms (p95) | Mask resources (median) | Mask KiB (median) | New masks per frame, median (total) | New mask KiB total | Other commands (max) |
|---|---|---|---|---|---|---|---|---|---|---|---|
| diagram: live zoom steps | 6 | 0.23 | 0.22 (0.23) | 2.13 | 0.67 (2.26) | 3.82 (3.85) | 29 | 24 | 1 (9) | 7 | 19 |
| diagram: live zoom settle frame | 1 | 0.26 | 0.26 (0.26) | 1.84 | 0.54 (0.54) | 3.60 (3.60) | 28 | 25 | 2 (2) | 3 | 18 |
| diagram: settled zoom in and out | 6 | 0.23 | 0.22 (0.26) | 2.01 | 0.78 (2.04) | 3.80 (3.86) | 27 | 25 | 2 (15) | 18 | 19 |
| diagram: pan 7 px per frame | 10 | 0.12 | 0.16 (0.18) | 1.98 | 0.76 (2.00) | 3.79 (3.83) | 26 | 22 | 0 (0) | 0 | 19 |

Cards, pan: ends at 100.0%.

Cards, pan back: ends at 100.0%.

Cards, zoom in: ends at 123.1%.

Cards, zoom out: ends at 100.0%.

Cards, zoom out further: ends at 81.2%.

## Gallery zoom workload (64 cards)

| Scenario | Frames | Compose first ms | Compose median ms (p95) | CPU median ms | Present median ms (p95) | GPU median ms (p95) | Mask resources (median) | Mask KiB (median) | New masks per frame, median (total) | New mask KiB total | Other commands (max) |
|---|---|---|---|---|---|---|---|---|---|---|---|
| cards: pan steps | 6 | 0.38 | 0.45 (0.48) | 2.73 | 0.58 (1.51) | 2.24 (2.30) | 31 | 53 | 0 (4) | 9 | 40 |
| cards: pan rest frame | 1 | 0.05 | 0.05 (0.05) | 2.74 | 0.47 (0.47) | 2.13 (2.13) | 31 | 53 | 0 (0) | 0 | 40 |
| cards: pan back steps | 6 | 0.37 | 0.44 (0.45) | 2.70 | 0.55 (0.57) | 2.24 (2.29) | 31 | 53 | 0 (0) | 0 | 36 |
| cards: pan back rest frame | 1 | 0.04 | 0.04 (0.04) | 2.42 | 0.44 (0.44) | 2.17 (2.17) | 27 | 43 | 0 (0) | 0 | 36 |
| cards: zoom in steps | 6 | 4.16 | 0.53 (4.16) | 2.78 | 0.53 (0.75) | 2.27 (2.32) | 27 | 47 | 0 (26) | 46 | 36 |
| cards: zoom in rest frame | 1 | 4.03 | 4.03 (4.03) | 2.60 | 0.67 (0.67) | 2.24 (2.24) | 21 | 51 | 21 (21) | 51 | 24 |
| cards: zoom out steps | 6 | 4.06 | 0.54 (4.06) | 2.75 | 0.56 (0.68) | 2.21 (2.32) | 27 | 61 | 3 (26) | 60 | 36 |
| cards: zoom out rest frame | 1 | 4.09 | 4.09 (4.09) | 2.61 | 0.75 (0.75) | 2.40 (2.40) | 27 | 43 | 27 (27) | 43 | 36 |
| cards: zoom out further steps | 6 | 4.07 | 0.53 (4.07) | 2.62 | 0.58 (0.79) | 2.18 (2.37) | 31 | 49 | 0 (30) | 48 | 40 |
| cards: zoom out further rest frame | 1 | 4.14 | 4.14 (4.14) | 2.39 | 0.81 (0.81) | 2.30 (2.30) | 31 | 35 | 30 (30) | 34 | 40 |

## Knob

| Scenario | Frames | Compose first ms | Compose median ms (p95) | CPU median ms | Present median ms (p95) | GPU median ms (p95) | Mask resources (median) | Mask KiB (median) | New masks per frame, median (total) | New mask KiB total | Other commands (max) |
|---|---|---|---|---|---|---|---|---|---|---|---|
| knob: gallery input page, Shift+Up (whole display list) | 20 | 0.92 | 1.16 (1.26) | 3.57 | 0.51 (0.61) | 3.61 (3.74) | 38 | 70 | 3 (60) | 196 | 45 |
| knob: isolated, no readout, host writes | 20 | 0.00 | 0.03 (0.04) | 0.09 | 0.37 (0.39) | 0.84 (0.90) | 0 | 0 | 0 (0) | 0 | 2 |

j1-page: shells
heaviest-page: shells

## J1 change scenarios

| Scenario | Frames | Compose first ms | Compose median ms (p95) | CPU median ms | Present median ms (p95) | GPU median ms (p95) | Mask resources (median) | Mask KiB (median) | New masks per frame, median (total) | New mask KiB total | Other commands (max) |
|---|---|---|---|---|---|---|---|---|---|---|---|
| J1 `shells`: resize +7 px per frame | 5 | 0.78 | 0.97 (1.79) | 4.47 | 1.92 (2.06) | 7.39 (31.68) | 40 | 118 | 0 (1) | 49 | 33 |
| J1 `shells`: scale 1 → 1.25 → 1.5 → 1.75 → 2 | 4 | 3.30 | 4.27 (4.42) | 16.23 | 4.94 (5.95) | 11.49 (13.11) | 40 | 352 | 40 (160) | 1250 | 33 |
<!-- end 2026-11-16T16:08:15+01:00, load 1.12 3.75 2.76, highest during the run 1.12
%CPU COMMAND
 1.2 jetbrains-toolb
 1.0 glycin-svg
 0.8 claude
 0.5 gnome-terminal-
 0.4 gnome-shell
-->
