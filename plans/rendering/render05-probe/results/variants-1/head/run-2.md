<!-- start 2026-11-16T16:08:02+01:00, waited 0 min, load 1.32 3.88 2.79
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

Empty scene offscreen render + read-back: median 3.20 ms.

| Page | Rects | Masks drawn | Images | Vectors | Other | Mask resources | Mask KiB | Image KiB | Compose cold ms | (paint/text/geometry self ms) | Compose warm ms | CPU render ms | Present cold ms | Present warm ms | GPU first ms | GPU warm ms (p95) |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| layout | 28 | 43 | 1 | 0 | 23 | 41 | 111 | 1 | 1.65 | 1.54/0.00/0.00 | 0.094 | 5.5 | 1.44 | 0.79 | 4.26 | 3.28 (4.47) |
| splits | 24 | 38 | 1 | 0 | 26 | 36 | 94 | 1 | 1.56 | 1.44/0.00/0.00 | 0.073 | 5.0 | 0.62 | 0.55 | 3.60 | 2.95 (3.45) |
| shells | 28 | 45 | 1 | 0 | 33 | 40 | 116 | 1 | 1.54 | 1.43/0.00/0.00 | 0.073 | 5.6 | 0.93 | 0.53 | 4.02 | 3.21 (3.30) |

Mixer: 0 extra flushes over 20 ticks while waiting for the engine; their time is in compose.

## Mixer with live data

| Scenario | Frames | Compose first ms | Compose median ms (p95) | CPU median ms | Present median ms (p95) | GPU median ms (p95) | Mask resources (median) | Mask KiB (median) | New masks per frame, median (total) | New mask KiB total | Other commands (max) |
|---|---|---|---|---|---|---|---|---|---|---|---|
| mixer: one tick per frame | 20 | 0.02 | 0.06 (0.08) | 1.07 | 0.36 (0.44) | 2.18 (2.37) | 14 | 18 | 0 (0) | 0 | 21 |

Diagram live zoom ends at scale 1.419.

## Diagram editor zoom and pan

| Scenario | Frames | Compose first ms | Compose median ms (p95) | CPU median ms | Present median ms (p95) | GPU median ms (p95) | Mask resources (median) | Mask KiB (median) | New masks per frame, median (total) | New mask KiB total | Other commands (max) |
|---|---|---|---|---|---|---|---|---|---|---|---|
| diagram: live zoom steps | 6 | 0.23 | 0.23 (0.25) | 2.12 | 0.73 (2.11) | 3.87 (3.99) | 29 | 24 | 1 (9) | 7 | 19 |
| diagram: live zoom settle frame | 1 | 0.26 | 0.26 (0.26) | 1.86 | 0.39 (0.39) | 3.98 (3.98) | 28 | 25 | 2 (2) | 3 | 18 |
| diagram: settled zoom in and out | 6 | 0.23 | 0.22 (0.24) | 1.98 | 0.77 (1.85) | 3.89 (4.04) | 27 | 25 | 2 (15) | 18 | 19 |
| diagram: pan 7 px per frame | 10 | 0.11 | 0.16 (0.17) | 1.95 | 0.77 (1.91) | 3.89 (4.02) | 26 | 22 | 0 (0) | 0 | 19 |

Cards, pan: ends at 100.0%.

Cards, pan back: ends at 100.0%.

Cards, zoom in: ends at 123.1%.

Cards, zoom out: ends at 100.0%.

Cards, zoom out further: ends at 81.2%.

## Gallery zoom workload (64 cards)

| Scenario | Frames | Compose first ms | Compose median ms (p95) | CPU median ms | Present median ms (p95) | GPU median ms (p95) | Mask resources (median) | Mask KiB (median) | New masks per frame, median (total) | New mask KiB total | Other commands (max) |
|---|---|---|---|---|---|---|---|---|---|---|---|
| cards: pan steps | 6 | 0.37 | 0.45 (0.48) | 2.70 | 0.54 (1.51) | 2.35 (2.45) | 31 | 53 | 0 (4) | 9 | 40 |
| cards: pan rest frame | 1 | 0.05 | 0.05 (0.05) | 2.70 | 0.47 (0.47) | 2.34 (2.34) | 31 | 53 | 0 (0) | 0 | 40 |
| cards: pan back steps | 6 | 0.37 | 0.44 (0.45) | 2.66 | 0.56 (0.58) | 2.34 (2.39) | 31 | 53 | 0 (0) | 0 | 36 |
| cards: pan back rest frame | 1 | 0.05 | 0.05 (0.05) | 2.38 | 0.43 (0.43) | 2.25 (2.25) | 27 | 43 | 0 (0) | 0 | 36 |
| cards: zoom in steps | 6 | 4.18 | 0.51 (4.18) | 2.77 | 0.54 (0.74) | 2.44 (2.52) | 27 | 47 | 0 (26) | 46 | 36 |
| cards: zoom in rest frame | 1 | 3.98 | 3.98 (3.98) | 2.55 | 0.68 (0.68) | 2.57 (2.57) | 21 | 51 | 21 (21) | 51 | 24 |
| cards: zoom out steps | 6 | 4.10 | 0.57 (4.10) | 2.70 | 0.55 (0.67) | 2.39 (2.57) | 27 | 61 | 3 (26) | 60 | 36 |
| cards: zoom out rest frame | 1 | 4.07 | 4.07 (4.07) | 2.59 | 0.76 (0.76) | 2.45 (2.45) | 27 | 43 | 27 (27) | 43 | 36 |
| cards: zoom out further steps | 6 | 4.14 | 0.54 (4.14) | 2.60 | 0.59 (0.79) | 2.31 (2.47) | 31 | 49 | 0 (30) | 48 | 40 |
| cards: zoom out further rest frame | 1 | 4.09 | 4.09 (4.09) | 2.38 | 0.82 (0.82) | 2.52 (2.52) | 31 | 35 | 30 (30) | 34 | 40 |

## Knob

| Scenario | Frames | Compose first ms | Compose median ms (p95) | CPU median ms | Present median ms (p95) | GPU median ms (p95) | Mask resources (median) | Mask KiB (median) | New masks per frame, median (total) | New mask KiB total | Other commands (max) |
|---|---|---|---|---|---|---|---|---|---|---|---|
| knob: gallery input page, Shift+Up (whole display list) | 20 | 0.91 | 1.12 (1.17) | 3.53 | 0.57 (0.66) | 4.62 (7.25) | 38 | 70 | 3 (60) | 196 | 45 |
| knob: isolated, no readout, host writes | 20 | 0.00 | 0.03 (0.03) | 0.09 | 0.37 (0.41) | 0.90 (1.00) | 0 | 0 | 0 (0) | 0 | 2 |

j1-page: shells
heaviest-page: shells

## J1 change scenarios

| Scenario | Frames | Compose first ms | Compose median ms (p95) | CPU median ms | Present median ms (p95) | GPU median ms (p95) | Mask resources (median) | Mask KiB (median) | New masks per frame, median (total) | New mask KiB total | Other commands (max) |
|---|---|---|---|---|---|---|---|---|---|---|---|
| J1 `shells`: resize +7 px per frame | 5 | 0.76 | 0.95 (1.68) | 4.43 | 1.85 (1.90) | 7.32 (32.48) | 40 | 118 | 0 (1) | 49 | 33 |
| J1 `shells`: scale 1 → 1.25 → 1.5 → 1.75 → 2 | 4 | 3.36 | 4.29 (4.48) | 16.26 | 4.45 (5.14) | 13.85 (16.63) | 40 | 352 | 40 (160) | 1250 | 33 |
<!-- end 2026-11-16T16:08:07+01:00, load 1.21 3.81 2.78, highest during the run 1.32
%CPU COMMAND
 1.2 jetbrains-toolb
 1.0 glycin-svg
 0.8 claude
 0.5 gnome-terminal-
 0.4 gnome-shell
-->
