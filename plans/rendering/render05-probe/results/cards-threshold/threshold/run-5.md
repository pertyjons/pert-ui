<!-- start 2026-11-16T17:50:33+01:00, waited 0 min, load 0.55 2.18 1.66
%CPU COMMAND
 1.2 jetbrains-toolb
 0.9 claude
 0.5 gnome-terminal-
 0.4 gnome-shell
 0.3 glycin-svg
-->
# Phase 1 probe

Definitions are in the probe's module documentation (`plans/rendering/phase1-probe/probe.rs`).

Cards, pan: ends at 100.0%.

Cards, pan back: ends at 100.0%.

Cards, zoom in: ends at 800.0%.

Cards, zoom out: ends at 100.0%.

Cards, zoom out further: ends at 12.5%.

## Gallery zoom workload (64 cards)

| Scenario | Frames | Compose first ms | Compose median ms (p95) | CPU median ms | Present median ms (p95) | GPU median ms (p95) | Mask resources (median) | Mask KiB (median) | New masks per frame, median (total) | New mask KiB total | Other commands (max) |
|---|---|---|---|---|---|---|---|---|---|---|---|
| cards: pan steps | 60 | 0.40 | 0.42 (0.59) | 2.52 | 0.55 (0.70) | 2.26 (3.40) | 27 | 46 | 0 (36) | 66 | 48 |
| cards: pan rest frame | 1 | 0.05 | 0.05 (0.05) | 2.58 | 0.47 (0.47) | 2.15 (2.15) | 27 | 46 | 0 (0) | 0 | 36 |
| cards: pan back steps | 60 | 0.34 | 0.42 (0.45) | 2.46 | 0.55 (0.60) | 2.29 (2.47) | 27 | 46 | 0 (36) | 63 | 48 |
| cards: pan back rest frame | 1 | 0.04 | 0.04 (0.04) | 2.40 | 0.45 (0.45) | 2.29 (2.29) | 27 | 43 | 0 (0) | 0 | 36 |
| cards: zoom in steps | 60 | 4.17 | 0.50 (0.54) | 2.49 | 0.37 (0.52) | 2.21 (2.39) | 7 | 10 | 0 (26) | 46 | 36 |
| cards: zoom in rest frame | 1 | 4.58 | 4.58 (4.58) | 1.13 | 0.30 (0.30) | 2.28 (2.28) | 0 | 0 | 0 (0) | 0 | 1 |
| cards: zoom out steps | 60 | 4.22 | 0.50 (0.99) | 2.30 | 0.39 (0.54) | 2.27 (2.47) | 7 | 523 | 0 (27) | 2517 | 36 |
| cards: zoom out rest frame | 1 | 4.13 | 4.13 (4.13) | 2.68 | 0.76 (0.76) | 2.33 (2.33) | 27 | 43 | 27 (27) | 43 | 36 |
| cards: zoom out further steps | 60 | 4.14 | 0.57 (0.76) | 1.75 | 0.94 (1.15) | 2.31 (2.49) | 94 | 156 | 0 (130) | 214 | 192 |
| cards: zoom out further rest frame | 1 | 4.83 | 4.83 (4.83) | 1.09 | 2.27 (2.27) | 3.42 (3.42) | 131 | 5 | 131 (131) | 5 | 192 |
<!-- end 2026-11-16T17:50:43+01:00, load 0.54 2.12 1.65, highest during the run 0.55
%CPU COMMAND
 1.2 jetbrains-toolb
 0.9 claude
 0.5 gnome-terminal-
 0.4 gnome-shell
 0.3 glycin-svg
-->
