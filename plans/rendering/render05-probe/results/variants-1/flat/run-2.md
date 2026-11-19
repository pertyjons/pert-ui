<!-- start 2026-11-16T16:08:07+01:00, waited 0 min, load 1.21 3.81 2.78
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

Empty scene offscreen render + read-back: median 3.16 ms.

| Page | Rects | Masks drawn | Images | Vectors | Other | Mask resources | Mask KiB | Image KiB | Compose cold ms | (paint/text/geometry self ms) | Compose warm ms | CPU render ms | Present cold ms | Present warm ms | GPU first ms | GPU warm ms (p95) |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| layout | 28 | 43 | 1 | 0 | 23 | 41 | 111 | 1 | 1.59 | 1.48/0.00/0.00 | 0.074 | 5.3 | 1.41 | 0.74 | 4.18 | 3.08 (3.63) |
| splits | 24 | 38 | 1 | 0 | 26 | 36 | 94 | 1 | 1.38 | 1.27/0.00/0.00 | 0.074 | 5.1 | 0.60 | 0.66 | 4.37 | 3.02 (3.67) |
| shells | 28 | 45 | 1 | 0 | 33 | 40 | 116 | 1 | 1.56 | 1.45/0.00/0.00 | 0.077 | 5.5 | 0.75 | 0.48 | 3.89 | 3.04 (3.05) |

Mixer: 0 extra flushes over 20 ticks while waiting for the engine; their time is in compose.

## Mixer with live data

| Scenario | Frames | Compose first ms | Compose median ms (p95) | CPU median ms | Present median ms (p95) | GPU median ms (p95) | Mask resources (median) | Mask KiB (median) | New masks per frame, median (total) | New mask KiB total | Other commands (max) |
|---|---|---|---|---|---|---|---|---|---|---|---|
| mixer: one tick per frame | 20 | 0.04 | 0.06 (0.08) | 1.07 | 0.39 (0.45) | 2.13 (2.32) | 14 | 18 | 0 (0) | 0 | 21 |

Diagram live zoom ends at scale 1.419.

## Diagram editor zoom and pan

| Scenario | Frames | Compose first ms | Compose median ms (p95) | CPU median ms | Present median ms (p95) | GPU median ms (p95) | Mask resources (median) | Mask KiB (median) | New masks per frame, median (total) | New mask KiB total | Other commands (max) |
|---|---|---|---|---|---|---|---|---|---|---|---|
| diagram: live zoom steps | 6 | 0.22 | 0.24 (0.26) | 2.13 | 0.77 (1.78) | 3.77 (3.99) | 29 | 24 | 1 (9) | 7 | 19 |
| diagram: live zoom settle frame | 1 | 0.26 | 0.26 (0.26) | 1.84 | 0.38 (0.38) | 3.78 (3.78) | 28 | 25 | 2 (2) | 3 | 18 |
| diagram: settled zoom in and out | 6 | 0.24 | 0.22 (0.25) | 2.00 | 0.78 (1.68) | 3.88 (6.14) | 27 | 25 | 2 (15) | 18 | 19 |
| diagram: pan 7 px per frame | 10 | 0.12 | 0.16 (0.19) | 1.97 | 0.77 (1.61) | 3.80 (6.04) | 26 | 22 | 0 (0) | 0 | 19 |

Cards, pan: ends at 100.0%.

Cards, pan back: ends at 100.0%.

Cards, zoom in: ends at 123.1%.

Cards, zoom out: ends at 100.0%.

Cards, zoom out further: ends at 81.2%.

## Gallery zoom workload (64 cards)

| Scenario | Frames | Compose first ms | Compose median ms (p95) | CPU median ms | Present median ms (p95) | GPU median ms (p95) | Mask resources (median) | Mask KiB (median) | New masks per frame, median (total) | New mask KiB total | Other commands (max) |
|---|---|---|---|---|---|---|---|---|---|---|---|
| cards: pan steps | 6 | 0.35 | 0.43 (0.54) | 2.70 | 0.56 (1.40) | 2.34 (2.46) | 31 | 53 | 0 (4) | 9 | 40 |
| cards: pan rest frame | 1 | 0.05 | 0.05 (0.05) | 2.69 | 0.32 (0.32) | 2.21 (2.21) | 31 | 53 | 0 (0) | 0 | 40 |
| cards: pan back steps | 6 | 0.35 | 0.43 (0.45) | 2.67 | 0.56 (0.57) | 2.31 (2.40) | 31 | 53 | 0 (0) | 0 | 36 |
| cards: pan back rest frame | 1 | 0.05 | 0.05 (0.05) | 2.41 | 0.46 (0.46) | 2.20 (2.20) | 27 | 43 | 0 (0) | 0 | 36 |
| cards: zoom in steps | 6 | 4.10 | 0.52 (4.10) | 2.75 | 0.54 (0.74) | 2.39 (2.49) | 27 | 47 | 0 (26) | 46 | 36 |
| cards: zoom in rest frame | 1 | 3.98 | 3.98 (3.98) | 2.57 | 0.63 (0.63) | 2.39 (2.39) | 21 | 51 | 21 (21) | 51 | 24 |
| cards: zoom out steps | 6 | 4.18 | 0.52 (4.18) | 2.73 | 0.55 (0.67) | 2.36 (2.40) | 27 | 61 | 3 (26) | 60 | 36 |
| cards: zoom out rest frame | 1 | 4.00 | 4.00 (4.00) | 2.58 | 0.77 (0.77) | 2.52 (2.52) | 27 | 43 | 27 (27) | 43 | 36 |
| cards: zoom out further steps | 6 | 4.08 | 0.52 (4.08) | 2.61 | 0.58 (0.80) | 2.31 (2.59) | 31 | 49 | 0 (30) | 48 | 40 |
| cards: zoom out further rest frame | 1 | 4.19 | 4.19 (4.19) | 2.38 | 0.61 (0.61) | 2.64 (2.64) | 31 | 35 | 30 (30) | 34 | 40 |

## Knob

| Scenario | Frames | Compose first ms | Compose median ms (p95) | CPU median ms | Present median ms (p95) | GPU median ms (p95) | Mask resources (median) | Mask KiB (median) | New masks per frame, median (total) | New mask KiB total | Other commands (max) |
|---|---|---|---|---|---|---|---|---|---|---|---|
| knob: gallery input page, Shift+Up (whole display list) | 20 | 0.90 | 1.11 (1.14) | 3.53 | 0.57 (0.61) | 4.11 (4.24) | 38 | 70 | 3 (60) | 196 | 45 |
| knob: isolated, no readout, host writes | 20 | 0.00 | 0.03 (0.03) | 0.09 | 0.37 (0.40) | 0.86 (1.03) | 0 | 0 | 0 (0) | 0 | 2 |

j1-page: shells
heaviest-page: shells

## J1 change scenarios

| Scenario | Frames | Compose first ms | Compose median ms (p95) | CPU median ms | Present median ms (p95) | GPU median ms (p95) | Mask resources (median) | Mask KiB (median) | New masks per frame, median (total) | New mask KiB total | Other commands (max) |
|---|---|---|---|---|---|---|---|---|---|---|---|
| J1 `shells`: resize +7 px per frame | 5 | 0.76 | 0.96 (1.77) | 4.49 | 1.47 (1.74) | 7.63 (36.15) | 40 | 118 | 0 (1) | 49 | 33 |
| J1 `shells`: scale 1 → 1.25 → 1.5 → 1.75 → 2 | 4 | 3.33 | 4.38 (4.52) | 16.36 | 3.82 (4.75) | 11.74 (14.26) | 40 | 352 | 40 (160) | 1250 | 33 |
<!-- end 2026-11-16T16:08:11+01:00, load 1.12 3.75 2.76, highest during the run 1.21
%CPU COMMAND
 1.2 jetbrains-toolb
 1.1 glycin-svg
 0.8 claude
 0.5 gnome-terminal-
 0.4 gnome-shell
-->
