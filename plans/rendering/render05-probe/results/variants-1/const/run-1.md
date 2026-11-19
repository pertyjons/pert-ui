<!-- start 2026-11-16T16:07:58+01:00, waited 0 min, load 1.35 3.93 2.80
%CPU COMMAND
 1.2 jetbrains-toolb
 0.9 glycin-svg
 0.8 claude
 0.5 gnome-terminal-
 0.4 gnome-shell
-->
# Phase 1 probe

Definitions are in the probe's module documentation (`plans/rendering/phase1-probe/probe.rs`).

## Gallery pages at scale 1 (1280×800 physical)

Empty scene offscreen render + read-back: median 3.42 ms.

| Page | Rects | Masks drawn | Images | Vectors | Other | Mask resources | Mask KiB | Image KiB | Compose cold ms | (paint/text/geometry self ms) | Compose warm ms | CPU render ms | Present cold ms | Present warm ms | GPU first ms | GPU warm ms (p95) |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| layout | 28 | 43 | 1 | 0 | 23 | 41 | 111 | 1 | 1.51 | 1.40/0.00/0.00 | 0.077 | 5.5 | 1.34 | 0.83 | 8.21 | 5.53 (7.37) |
| splits | 24 | 38 | 1 | 0 | 26 | 36 | 94 | 1 | 1.37 | 1.27/0.00/0.00 | 0.071 | 4.9 | 0.68 | 0.56 | 2.89 | 2.68 (3.59) |
| shells | 28 | 45 | 1 | 0 | 33 | 40 | 116 | 1 | 1.40 | 1.30/0.00/0.00 | 0.062 | 5.3 | 0.65 | 0.45 | 4.21 | 3.28 (4.85) |

Mixer: 0 extra flushes over 20 ticks while waiting for the engine; their time is in compose.

## Mixer with live data

| Scenario | Frames | Compose first ms | Compose median ms (p95) | CPU median ms | Present median ms (p95) | GPU median ms (p95) | Mask resources (median) | Mask KiB (median) | New masks per frame, median (total) | New mask KiB total | Other commands (max) |
|---|---|---|---|---|---|---|---|---|---|---|---|
| mixer: one tick per frame | 20 | 0.05 | 0.06 (0.10) | 1.07 | 0.40 (0.45) | 2.15 (2.28) | 14 | 18 | 0 (0) | 0 | 21 |

Diagram live zoom ends at scale 1.419.

## Diagram editor zoom and pan

| Scenario | Frames | Compose first ms | Compose median ms (p95) | CPU median ms | Present median ms (p95) | GPU median ms (p95) | Mask resources (median) | Mask KiB (median) | New masks per frame, median (total) | New mask KiB total | Other commands (max) |
|---|---|---|---|---|---|---|---|---|---|---|---|
| diagram: live zoom steps | 6 | 0.23 | 0.25 (0.26) | 2.12 | 0.77 (1.88) | 3.89 (3.98) | 29 | 24 | 1 (9) | 7 | 19 |
| diagram: live zoom settle frame | 1 | 0.26 | 0.26 (0.26) | 1.80 | 0.53 (0.53) | 3.79 (3.79) | 28 | 25 | 2 (2) | 3 | 18 |
| diagram: settled zoom in and out | 6 | 0.23 | 0.22 (0.27) | 2.00 | 0.75 (1.97) | 3.80 (3.94) | 27 | 25 | 2 (15) | 18 | 19 |
| diagram: pan 7 px per frame | 10 | 0.11 | 0.16 (0.16) | 1.97 | 0.77 (1.66) | 3.79 (3.90) | 26 | 22 | 0 (0) | 0 | 19 |

Cards, pan: ends at 100.0%.

Cards, pan back: ends at 100.0%.

Cards, zoom in: ends at 123.1%.

Cards, zoom out: ends at 100.0%.

Cards, zoom out further: ends at 81.2%.

## Gallery zoom workload (64 cards)

| Scenario | Frames | Compose first ms | Compose median ms (p95) | CPU median ms | Present median ms (p95) | GPU median ms (p95) | Mask resources (median) | Mask KiB (median) | New masks per frame, median (total) | New mask KiB total | Other commands (max) |
|---|---|---|---|---|---|---|---|---|---|---|---|
| cards: pan steps | 6 | 0.36 | 0.40 (0.47) | 2.71 | 0.54 (1.30) | 2.14 (2.28) | 31 | 53 | 0 (4) | 9 | 40 |
| cards: pan rest frame | 1 | 0.05 | 0.05 (0.05) | 2.71 | 0.50 (0.50) | 2.16 (2.16) | 31 | 53 | 0 (0) | 0 | 40 |
| cards: pan back steps | 6 | 0.36 | 0.43 (0.45) | 2.67 | 0.55 (0.56) | 2.19 (2.21) | 31 | 53 | 0 (0) | 0 | 36 |
| cards: pan back rest frame | 1 | 0.04 | 0.04 (0.04) | 2.44 | 0.48 (0.48) | 2.11 (2.11) | 27 | 43 | 0 (0) | 0 | 36 |
| cards: zoom in steps | 6 | 4.20 | 0.52 (4.20) | 2.75 | 0.52 (0.72) | 2.23 (2.38) | 27 | 47 | 0 (26) | 46 | 36 |
| cards: zoom in rest frame | 1 | 4.07 | 4.07 (4.07) | 2.56 | 0.67 (0.67) | 2.22 (2.22) | 21 | 51 | 21 (21) | 51 | 24 |
| cards: zoom out steps | 6 | 4.10 | 0.51 (4.10) | 2.73 | 0.55 (0.70) | 2.23 (2.35) | 27 | 61 | 3 (26) | 60 | 36 |
| cards: zoom out rest frame | 1 | 4.12 | 4.12 (4.12) | 2.59 | 0.75 (0.75) | 2.33 (2.33) | 27 | 43 | 27 (27) | 43 | 36 |
| cards: zoom out further steps | 6 | 4.07 | 0.52 (4.07) | 2.60 | 0.60 (0.79) | 2.18 (2.28) | 31 | 49 | 0 (30) | 48 | 40 |
| cards: zoom out further rest frame | 1 | 4.15 | 4.15 (4.15) | 2.38 | 0.63 (0.63) | 2.52 (2.52) | 31 | 35 | 30 (30) | 34 | 40 |

## Knob

| Scenario | Frames | Compose first ms | Compose median ms (p95) | CPU median ms | Present median ms (p95) | GPU median ms (p95) | Mask resources (median) | Mask KiB (median) | New masks per frame, median (total) | New mask KiB total | Other commands (max) |
|---|---|---|---|---|---|---|---|---|---|---|---|
| knob: gallery input page, Shift+Up (whole display list) | 20 | 0.89 | 1.12 (1.16) | 3.53 | 0.58 (0.61) | 3.70 (3.75) | 38 | 70 | 3 (60) | 196 | 45 |
| knob: isolated, no readout, host writes | 20 | 0.00 | 0.03 (0.04) | 0.09 | 0.36 (0.38) | 0.88 (0.92) | 0 | 0 | 0 (0) | 0 | 2 |

j1-page: shells
heaviest-page: shells

## J1 change scenarios

| Scenario | Frames | Compose first ms | Compose median ms (p95) | CPU median ms | Present median ms (p95) | GPU median ms (p95) | Mask resources (median) | Mask KiB (median) | New masks per frame, median (total) | New mask KiB total | Other commands (max) |
|---|---|---|---|---|---|---|---|---|---|---|---|
| J1 `shells`: resize +7 px per frame | 5 | 0.77 | 0.97 (1.74) | 4.48 | 1.67 (1.71) | 7.35 (32.14) | 40 | 118 | 0 (1) | 49 | 33 |
| J1 `shells`: scale 1 → 1.25 → 1.5 → 1.75 → 2 | 4 | 3.30 | 4.29 (4.54) | 16.44 | 4.67 (5.30) | 10.66 (14.26) | 40 | 352 | 40 (160) | 1250 | 33 |
<!-- end 2026-11-16T16:08:02+01:00, load 1.32 3.88 2.79, highest during the run 1.35
%CPU COMMAND
 1.2 jetbrains-toolb
 0.9 glycin-svg
 0.8 claude
 0.5 gnome-terminal-
 0.4 gnome-shell
-->
