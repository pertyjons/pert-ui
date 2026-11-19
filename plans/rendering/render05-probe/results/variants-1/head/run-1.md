<!-- start 2026-11-16T16:07:48+01:00, waited 2 min, load 1.59 4.06 2.83
%CPU COMMAND
 1.2 jetbrains-toolb
 0.8 claude
 0.5 gnome-terminal-
 0.4 gnome-shell
 0.2 kworker/u97:20-btrfs-endio
-->
# Phase 1 probe

Definitions are in the probe's module documentation (`plans/rendering/phase1-probe/probe.rs`).

## Gallery pages at scale 1 (1280×800 physical)

Empty scene offscreen render + read-back: median 3.19 ms.

| Page | Rects | Masks drawn | Images | Vectors | Other | Mask resources | Mask KiB | Image KiB | Compose cold ms | (paint/text/geometry self ms) | Compose warm ms | CPU render ms | Present cold ms | Present warm ms | GPU first ms | GPU warm ms (p95) |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| layout | 28 | 43 | 1 | 0 | 23 | 41 | 111 | 1 | 1.37 | 1.29/0.00/0.00 | 0.057 | 4.7 | 1.21 | 0.76 | 4.65 | 3.28 (3.37) |
| splits | 24 | 38 | 1 | 0 | 26 | 36 | 94 | 1 | 1.41 | 1.30/0.00/0.00 | 0.072 | 4.9 | 0.65 | 0.53 | 5.87 | 3.22 (4.60) |
| shells | 28 | 45 | 1 | 0 | 33 | 40 | 116 | 1 | 1.40 | 1.30/0.00/0.00 | 0.070 | 5.1 | 0.63 | 0.44 | 4.16 | 3.28 (3.57) |

Mixer: 0 extra flushes over 20 ticks while waiting for the engine; their time is in compose.

## Mixer with live data

| Scenario | Frames | Compose first ms | Compose median ms (p95) | CPU median ms | Present median ms (p95) | GPU median ms (p95) | Mask resources (median) | Mask KiB (median) | New masks per frame, median (total) | New mask KiB total | Other commands (max) |
|---|---|---|---|---|---|---|---|---|---|---|---|
| mixer: one tick per frame | 20 | 0.02 | 0.07 (0.10) | 1.06 | 0.41 (0.42) | 2.19 (2.27) | 14 | 18 | 0 (0) | 0 | 21 |

Diagram live zoom ends at scale 1.419.

## Diagram editor zoom and pan

| Scenario | Frames | Compose first ms | Compose median ms (p95) | CPU median ms | Present median ms (p95) | GPU median ms (p95) | Mask resources (median) | Mask KiB (median) | New masks per frame, median (total) | New mask KiB total | Other commands (max) |
|---|---|---|---|---|---|---|---|---|---|---|---|
| diagram: live zoom steps | 6 | 0.25 | 0.24 (0.26) | 2.10 | 0.74 (1.87) | 3.87 (3.99) | 29 | 24 | 1 (9) | 7 | 19 |
| diagram: live zoom settle frame | 1 | 0.26 | 0.26 (0.26) | 1.80 | 0.52 (0.52) | 3.73 (3.73) | 28 | 25 | 2 (2) | 3 | 18 |
| diagram: settled zoom in and out | 6 | 0.24 | 0.23 (0.27) | 1.99 | 0.78 (1.75) | 3.91 (4.00) | 27 | 25 | 2 (15) | 18 | 19 |
| diagram: pan 7 px per frame | 10 | 0.11 | 0.16 (0.16) | 1.97 | 0.78 (1.75) | 3.87 (3.91) | 26 | 22 | 0 (0) | 0 | 19 |

Cards, pan: ends at 100.0%.

Cards, pan back: ends at 100.0%.

Cards, zoom in: ends at 123.1%.

Cards, zoom out: ends at 100.0%.

Cards, zoom out further: ends at 81.2%.

## Gallery zoom workload (64 cards)

| Scenario | Frames | Compose first ms | Compose median ms (p95) | CPU median ms | Present median ms (p95) | GPU median ms (p95) | Mask resources (median) | Mask KiB (median) | New masks per frame, median (total) | New mask KiB total | Other commands (max) |
|---|---|---|---|---|---|---|---|---|---|---|---|
| cards: pan steps | 6 | 0.36 | 0.45 (0.47) | 2.71 | 0.56 (1.31) | 2.30 (2.32) | 31 | 53 | 0 (4) | 9 | 40 |
| cards: pan rest frame | 1 | 0.05 | 0.05 (0.05) | 2.70 | 0.46 (0.46) | 2.36 (2.36) | 31 | 53 | 0 (0) | 0 | 40 |
| cards: pan back steps | 6 | 0.37 | 0.44 (0.48) | 2.75 | 0.53 (0.55) | 2.34 (2.41) | 31 | 53 | 0 (0) | 0 | 36 |
| cards: pan back rest frame | 1 | 0.05 | 0.05 (0.05) | 2.40 | 0.46 (0.46) | 2.20 (2.20) | 27 | 43 | 0 (0) | 0 | 36 |
| cards: zoom in steps | 6 | 4.12 | 0.51 (4.12) | 2.75 | 0.55 (0.73) | 2.40 (2.44) | 27 | 47 | 0 (26) | 46 | 36 |
| cards: zoom in rest frame | 1 | 3.99 | 3.99 (3.99) | 2.56 | 0.67 (0.67) | 2.45 (2.45) | 21 | 51 | 21 (21) | 51 | 24 |
| cards: zoom out steps | 6 | 4.04 | 0.51 (4.04) | 2.69 | 0.53 (0.55) | 2.36 (2.43) | 27 | 61 | 3 (26) | 60 | 36 |
| cards: zoom out rest frame | 1 | 3.99 | 3.99 (3.99) | 2.58 | 0.74 (0.74) | 2.59 (2.59) | 27 | 43 | 27 (27) | 43 | 36 |
| cards: zoom out further steps | 6 | 4.08 | 0.48 (4.08) | 2.59 | 0.43 (0.72) | 2.25 (2.47) | 31 | 49 | 0 (30) | 48 | 40 |
| cards: zoom out further rest frame | 1 | 4.09 | 4.09 (4.09) | 2.36 | 0.63 (0.63) | 2.36 (2.36) | 31 | 35 | 30 (30) | 34 | 40 |

## Knob

| Scenario | Frames | Compose first ms | Compose median ms (p95) | CPU median ms | Present median ms (p95) | GPU median ms (p95) | Mask resources (median) | Mask KiB (median) | New masks per frame, median (total) | New mask KiB total | Other commands (max) |
|---|---|---|---|---|---|---|---|---|---|---|---|
| knob: gallery input page, Shift+Up (whole display list) | 20 | 0.91 | 1.09 (1.14) | 3.51 | 0.57 (0.59) | 4.46 (4.60) | 38 | 70 | 3 (60) | 196 | 45 |
| knob: isolated, no readout, host writes | 20 | 0.00 | 0.01 (0.03) | 0.09 | 0.25 (0.40) | 1.12 (1.24) | 0 | 0 | 0 (0) | 0 | 2 |

j1-page: shells
heaviest-page: shells

## J1 change scenarios

| Scenario | Frames | Compose first ms | Compose median ms (p95) | CPU median ms | Present median ms (p95) | GPU median ms (p95) | Mask resources (median) | Mask KiB (median) | New masks per frame, median (total) | New mask KiB total | Other commands (max) |
|---|---|---|---|---|---|---|---|---|---|---|---|
| J1 `shells`: resize +7 px per frame | 5 | 0.77 | 0.96 (1.86) | 4.44 | 1.62 (2.04) | 11.66 (32.47) | 40 | 118 | 0 (1) | 49 | 33 |
| J1 `shells`: scale 1 → 1.25 → 1.5 → 1.75 → 2 | 4 | 3.30 | 4.29 (4.47) | 17.21 | 4.37 (5.26) | 12.48 (16.40) | 40 | 352 | 40 (160) | 1250 | 33 |
<!-- end 2026-11-16T16:07:53+01:00, load 1.46 3.99 2.82, highest during the run 1.59
%CPU COMMAND
 1.2 jetbrains-toolb
 0.8 claude
 0.6 glycin-svg
 0.5 gnome-terminal-
 0.4 gnome-shell
-->
