<!-- start 2026-11-16T16:07:53+01:00, waited 0 min, load 1.46 3.99 2.82
%CPU COMMAND
 1.2 jetbrains-toolb
 0.8 claude
 0.6 glycin-svg
 0.5 gnome-terminal-
 0.4 gnome-shell
-->
# Phase 1 probe

Definitions are in the probe's module documentation (`plans/rendering/phase1-probe/probe.rs`).

## Gallery pages at scale 1 (1280×800 physical)

Empty scene offscreen render + read-back: median 3.26 ms.

| Page | Rects | Masks drawn | Images | Vectors | Other | Mask resources | Mask KiB | Image KiB | Compose cold ms | (paint/text/geometry self ms) | Compose warm ms | CPU render ms | Present cold ms | Present warm ms | GPU first ms | GPU warm ms (p95) |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| layout | 28 | 43 | 1 | 0 | 23 | 41 | 111 | 1 | 1.50 | 1.40/0.00/0.00 | 0.065 | 5.3 | 1.34 | 0.77 | 6.37 | 4.40 (5.96) |
| splits | 24 | 38 | 1 | 0 | 26 | 36 | 94 | 1 | 1.46 | 1.36/0.00/0.00 | 0.077 | 4.7 | 0.63 | 0.51 | 4.17 | 2.86 (3.46) |
| shells | 28 | 45 | 1 | 0 | 33 | 40 | 116 | 1 | 1.38 | 1.28/0.00/0.00 | 0.062 | 5.3 | 0.62 | 0.46 | 3.78 | 3.12 (3.78) |

Mixer: 0 extra flushes over 20 ticks while waiting for the engine; their time is in compose.

## Mixer with live data

| Scenario | Frames | Compose first ms | Compose median ms (p95) | CPU median ms | Present median ms (p95) | GPU median ms (p95) | Mask resources (median) | Mask KiB (median) | New masks per frame, median (total) | New mask KiB total | Other commands (max) |
|---|---|---|---|---|---|---|---|---|---|---|---|
| mixer: one tick per frame | 20 | 0.04 | 0.07 (0.12) | 1.06 | 0.41 (0.44) | 2.23 (2.28) | 14 | 18 | 0 (0) | 0 | 21 |

Diagram live zoom ends at scale 1.419.

## Diagram editor zoom and pan

| Scenario | Frames | Compose first ms | Compose median ms (p95) | CPU median ms | Present median ms (p95) | GPU median ms (p95) | Mask resources (median) | Mask KiB (median) | New masks per frame, median (total) | New mask KiB total | Other commands (max) |
|---|---|---|---|---|---|---|---|---|---|---|---|
| diagram: live zoom steps | 6 | 0.23 | 0.25 (0.26) | 2.13 | 0.75 (1.94) | 3.86 (3.96) | 29 | 24 | 1 (9) | 7 | 19 |
| diagram: live zoom settle frame | 1 | 0.25 | 0.25 (0.25) | 1.83 | 0.52 (0.52) | 3.71 (3.71) | 28 | 25 | 2 (2) | 3 | 18 |
| diagram: settled zoom in and out | 6 | 0.23 | 0.23 (0.28) | 2.01 | 0.77 (1.66) | 3.96 (4.02) | 27 | 25 | 2 (15) | 18 | 19 |
| diagram: pan 7 px per frame | 10 | 0.11 | 0.15 (0.17) | 1.97 | 0.77 (1.84) | 3.90 (3.98) | 26 | 22 | 0 (0) | 0 | 19 |

Cards, pan: ends at 100.0%.

Cards, pan back: ends at 100.0%.

Cards, zoom in: ends at 123.1%.

Cards, zoom out: ends at 100.0%.

Cards, zoom out further: ends at 81.2%.

## Gallery zoom workload (64 cards)

| Scenario | Frames | Compose first ms | Compose median ms (p95) | CPU median ms | Present median ms (p95) | GPU median ms (p95) | Mask resources (median) | Mask KiB (median) | New masks per frame, median (total) | New mask KiB total | Other commands (max) |
|---|---|---|---|---|---|---|---|---|---|---|---|
| cards: pan steps | 6 | 0.36 | 0.39 (0.47) | 2.69 | 0.54 (1.33) | 2.36 (2.46) | 31 | 53 | 0 (4) | 9 | 40 |
| cards: pan rest frame | 1 | 0.05 | 0.05 (0.05) | 2.78 | 0.46 (0.46) | 2.29 (2.29) | 31 | 53 | 0 (0) | 0 | 40 |
| cards: pan back steps | 6 | 0.37 | 0.44 (0.45) | 2.67 | 0.55 (0.56) | 2.33 (2.41) | 31 | 53 | 0 (0) | 0 | 36 |
| cards: pan back rest frame | 1 | 0.05 | 0.05 (0.05) | 2.41 | 0.47 (0.47) | 2.30 (2.30) | 27 | 43 | 0 (0) | 0 | 36 |
| cards: zoom in steps | 6 | 4.11 | 0.52 (4.11) | 2.74 | 0.53 (0.74) | 2.39 (2.50) | 27 | 47 | 0 (26) | 46 | 36 |
| cards: zoom in rest frame | 1 | 4.00 | 4.00 (4.00) | 2.59 | 0.66 (0.66) | 2.44 (2.44) | 21 | 51 | 21 (21) | 51 | 24 |
| cards: zoom out steps | 6 | 4.10 | 0.55 (4.10) | 2.71 | 0.56 (0.63) | 2.38 (2.48) | 27 | 61 | 3 (26) | 60 | 36 |
| cards: zoom out rest frame | 1 | 3.99 | 3.99 (3.99) | 2.59 | 0.76 (0.76) | 2.48 (2.48) | 27 | 43 | 27 (27) | 43 | 36 |
| cards: zoom out further steps | 6 | 4.03 | 0.52 (4.03) | 2.61 | 0.59 (1.00) | 2.32 (2.57) | 31 | 49 | 0 (30) | 48 | 40 |
| cards: zoom out further rest frame | 1 | 4.19 | 4.19 (4.19) | 2.37 | 0.84 (0.84) | 2.50 (2.50) | 31 | 35 | 30 (30) | 34 | 40 |

## Knob

| Scenario | Frames | Compose first ms | Compose median ms (p95) | CPU median ms | Present median ms (p95) | GPU median ms (p95) | Mask resources (median) | Mask KiB (median) | New masks per frame, median (total) | New mask KiB total | Other commands (max) |
|---|---|---|---|---|---|---|---|---|---|---|---|
| knob: gallery input page, Shift+Up (whole display list) | 20 | 0.91 | 1.11 (1.16) | 3.53 | 0.58 (0.61) | 4.13 (4.23) | 38 | 70 | 3 (60) | 196 | 45 |
| knob: isolated, no readout, host writes | 20 | 0.00 | 0.03 (0.03) | 0.09 | 0.37 (0.39) | 0.90 (0.93) | 0 | 0 | 0 (0) | 0 | 2 |

j1-page: shells
heaviest-page: shells

## J1 change scenarios

| Scenario | Frames | Compose first ms | Compose median ms (p95) | CPU median ms | Present median ms (p95) | GPU median ms (p95) | Mask resources (median) | Mask KiB (median) | New masks per frame, median (total) | New mask KiB total | Other commands (max) |
|---|---|---|---|---|---|---|---|---|---|---|---|
| J1 `shells`: resize +7 px per frame | 5 | 0.78 | 0.94 (1.71) | 4.51 | 1.62 (1.94) | 7.21 (32.34) | 40 | 118 | 0 (1) | 49 | 33 |
| J1 `shells`: scale 1 → 1.25 → 1.5 → 1.75 → 2 | 4 | 3.31 | 4.26 (4.58) | 16.55 | 4.28 (5.04) | 11.65 (15.89) | 40 | 352 | 40 (160) | 1250 | 33 |
<!-- end 2026-11-16T16:07:58+01:00, load 1.35 3.93 2.80, highest during the run 1.46
%CPU COMMAND
 1.2 jetbrains-toolb
 1.0 glycin-svg
 0.8 claude
 0.5 gnome-terminal-
 0.4 gnome-shell
-->
