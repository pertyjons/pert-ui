<!-- start 2026-11-16T16:08:16+01:00, waited 0 min, load 1.12 3.75 2.76
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

Empty scene offscreen render + read-back: median 4.06 ms.

| Page | Rects | Masks drawn | Images | Vectors | Other | Mask resources | Mask KiB | Image KiB | Compose cold ms | (paint/text/geometry self ms) | Compose warm ms | CPU render ms | Present cold ms | Present warm ms | GPU first ms | GPU warm ms (p95) |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| layout | 28 | 43 | 1 | 0 | 23 | 41 | 111 | 1 | 1.59 | 1.49/0.00/0.00 | 0.081 | 5.3 | 1.43 | 0.79 | 5.89 | 3.01 (3.91) |
| splits | 24 | 38 | 1 | 0 | 26 | 36 | 94 | 1 | 1.43 | 1.31/0.00/0.00 | 0.074 | 4.8 | 0.64 | 0.55 | 6.03 | 4.84 (5.74) |
| shells | 28 | 45 | 1 | 0 | 33 | 40 | 116 | 1 | 1.51 | 1.40/0.00/0.00 | 0.085 | 5.7 | 0.67 | 0.41 | 7.12 | 4.49 (4.50) |

Mixer: 0 extra flushes over 20 ticks while waiting for the engine; their time is in compose.

## Mixer with live data

| Scenario | Frames | Compose first ms | Compose median ms (p95) | CPU median ms | Present median ms (p95) | GPU median ms (p95) | Mask resources (median) | Mask KiB (median) | New masks per frame, median (total) | New mask KiB total | Other commands (max) |
|---|---|---|---|---|---|---|---|---|---|---|---|
| mixer: one tick per frame | 20 | 0.04 | 0.06 (0.10) | 1.06 | 0.40 (0.46) | 2.09 (2.27) | 14 | 18 | 0 (0) | 0 | 21 |

Diagram live zoom ends at scale 1.419.

## Diagram editor zoom and pan

| Scenario | Frames | Compose first ms | Compose median ms (p95) | CPU median ms | Present median ms (p95) | GPU median ms (p95) | Mask resources (median) | Mask KiB (median) | New masks per frame, median (total) | New mask KiB total | Other commands (max) |
|---|---|---|---|---|---|---|---|---|---|---|---|
| diagram: live zoom steps | 6 | 0.23 | 0.23 (0.24) | 2.10 | 0.70 (2.20) | 3.91 (5.97) | 29 | 24 | 1 (9) | 7 | 19 |
| diagram: live zoom settle frame | 1 | 0.25 | 0.25 (0.25) | 1.82 | 0.52 (0.52) | 3.64 (3.64) | 28 | 25 | 2 (2) | 3 | 18 |
| diagram: settled zoom in and out | 6 | 0.26 | 0.25 (0.28) | 2.00 | 0.77 (2.05) | 3.89 (3.97) | 27 | 25 | 2 (15) | 18 | 19 |
| diagram: pan 7 px per frame | 10 | 0.11 | 0.16 (0.17) | 1.96 | 0.78 (1.99) | 3.78 (3.89) | 26 | 22 | 0 (0) | 0 | 19 |

Cards, pan: ends at 100.0%.

Cards, pan back: ends at 100.0%.

Cards, zoom in: ends at 123.1%.

Cards, zoom out: ends at 100.0%.

Cards, zoom out further: ends at 81.2%.

## Gallery zoom workload (64 cards)

| Scenario | Frames | Compose first ms | Compose median ms (p95) | CPU median ms | Present median ms (p95) | GPU median ms (p95) | Mask resources (median) | Mask KiB (median) | New masks per frame, median (total) | New mask KiB total | Other commands (max) |
|---|---|---|---|---|---|---|---|---|---|---|---|
| cards: pan steps | 6 | 0.35 | 0.44 (0.52) | 2.70 | 0.57 (1.63) | 2.30 (2.42) | 31 | 53 | 0 (4) | 9 | 40 |
| cards: pan rest frame | 1 | 0.05 | 0.05 (0.05) | 2.72 | 0.47 (0.47) | 2.29 (2.29) | 31 | 53 | 0 (0) | 0 | 40 |
| cards: pan back steps | 6 | 0.37 | 0.44 (0.45) | 2.67 | 0.55 (0.57) | 2.31 (2.38) | 31 | 53 | 0 (0) | 0 | 36 |
| cards: pan back rest frame | 1 | 0.04 | 0.04 (0.04) | 2.39 | 0.44 (0.44) | 2.34 (2.34) | 27 | 43 | 0 (0) | 0 | 36 |
| cards: zoom in steps | 6 | 4.15 | 0.52 (4.15) | 2.78 | 0.54 (0.72) | 2.39 (2.62) | 27 | 47 | 0 (26) | 46 | 36 |
| cards: zoom in rest frame | 1 | 3.97 | 3.97 (3.97) | 2.55 | 0.65 (0.65) | 2.54 (2.54) | 21 | 51 | 21 (21) | 51 | 24 |
| cards: zoom out steps | 6 | 4.10 | 0.54 (4.10) | 2.72 | 0.56 (0.67) | 2.42 (2.56) | 27 | 61 | 3 (26) | 60 | 36 |
| cards: zoom out rest frame | 1 | 4.04 | 4.04 (4.04) | 2.58 | 0.75 (0.75) | 2.56 (2.56) | 27 | 43 | 27 (27) | 43 | 36 |
| cards: zoom out further steps | 6 | 4.16 | 0.53 (4.16) | 2.59 | 0.57 (0.82) | 2.34 (2.81) | 31 | 49 | 0 (30) | 48 | 40 |
| cards: zoom out further rest frame | 1 | 4.11 | 4.11 (4.11) | 2.36 | 0.79 (0.79) | 2.48 (2.48) | 31 | 35 | 30 (30) | 34 | 40 |

## Knob

| Scenario | Frames | Compose first ms | Compose median ms (p95) | CPU median ms | Present median ms (p95) | GPU median ms (p95) | Mask resources (median) | Mask KiB (median) | New masks per frame, median (total) | New mask KiB total | Other commands (max) |
|---|---|---|---|---|---|---|---|---|---|---|---|
| knob: gallery input page, Shift+Up (whole display list) | 20 | 0.91 | 1.12 (1.16) | 3.51 | 0.58 (0.63) | 4.51 (4.60) | 38 | 70 | 3 (60) | 196 | 45 |
| knob: isolated, no readout, host writes | 20 | 0.00 | 0.03 (0.05) | 0.09 | 0.37 (0.39) | 0.88 (0.94) | 0 | 0 | 0 (0) | 0 | 2 |

j1-page: shells
heaviest-page: shells

## J1 change scenarios

| Scenario | Frames | Compose first ms | Compose median ms (p95) | CPU median ms | Present median ms (p95) | GPU median ms (p95) | Mask resources (median) | Mask KiB (median) | New masks per frame, median (total) | New mask KiB total | Other commands (max) |
|---|---|---|---|---|---|---|---|---|---|---|---|
| J1 `shells`: resize +7 px per frame | 5 | 0.76 | 0.95 (1.74) | 4.46 | 1.56 (1.82) | 7.39 (32.09) | 40 | 118 | 0 (1) | 49 | 33 |
| J1 `shells`: scale 1 → 1.25 → 1.5 → 1.75 → 2 | 4 | 3.32 | 4.24 (4.42) | 16.37 | 4.62 (5.83) | 13.53 (15.81) | 40 | 352 | 40 (160) | 1250 | 33 |
<!-- end 2026-11-16T16:08:20+01:00, load 1.43 3.77 2.77, highest during the run 1.43
%CPU COMMAND
 1.2 jetbrains-toolb
 1.0 glycin-svg
 0.8 claude
 0.5 gnome-terminal-
 0.4 gnome-shell
-->
