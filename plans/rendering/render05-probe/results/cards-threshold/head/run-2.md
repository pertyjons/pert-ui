<!-- start 2026-11-16T17:48:58+01:00, waited 0 min, load 1.38 2.82 1.79
%CPU COMMAND
 1.2 jetbrains-toolb
 0.9 claude
 0.5 gnome-terminal-
 0.4 gnome-shell
 0.2 glycin-svg
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
| cards: pan steps | 60 | 0.46 | 0.42 (0.62) | 2.50 | 0.54 (0.63) | 2.30 (3.97) | 27 | 46 | 0 (36) | 66 | 48 |
| cards: pan rest frame | 1 | 0.05 | 0.05 (0.05) | 2.56 | 0.39 (0.39) | 2.14 (2.14) | 27 | 46 | 0 (0) | 0 | 36 |
| cards: pan back steps | 60 | 0.35 | 0.42 (0.45) | 2.46 | 0.54 (0.59) | 2.36 (2.45) | 27 | 46 | 0 (36) | 63 | 48 |
| cards: pan back rest frame | 1 | 0.05 | 0.05 (0.05) | 2.41 | 0.32 (0.32) | 2.32 (2.32) | 27 | 43 | 0 (0) | 0 | 36 |
| cards: zoom in steps | 60 | 4.14 | 0.43 (0.51) | 2.49 | 0.25 (0.36) | 2.31 (2.47) | 7 | 10 | 0 (26) | 46 | 36 |
| cards: zoom in rest frame | 1 | 4.49 | 4.49 (4.49) | 1.13 | 0.22 (0.22) | 2.45 (2.45) | 0 | 0 | 0 (0) | 0 | 1 |
| cards: zoom out steps | 60 | 4.12 | 0.50 (0.98) | 2.28 | 0.39 (0.56) | 2.37 (2.58) | 7 | 523 | 0 (27) | 2517 | 36 |
| cards: zoom out rest frame | 1 | 4.06 | 4.06 (4.06) | 2.69 | 0.73 (0.73) | 2.52 (2.52) | 27 | 43 | 27 (27) | 43 | 36 |
| cards: zoom out further steps | 60 | 4.10 | 0.53 (0.76) | 1.76 | 0.87 (1.18) | 2.26 (2.40) | 94 | 156 | 0 (130) | 214 | 192 |
| cards: zoom out further rest frame | 1 | 4.70 | 4.70 (4.70) | 1.01 | 2.39 (2.39) | 3.32 (3.32) | 131 | 5 | 131 (131) | 5 | 192 |
<!-- end 2026-11-16T17:49:07+01:00, load 1.39 2.77 1.79, highest during the run 1.38
%CPU COMMAND
 100 ps
 1.2 jetbrains-toolb
 0.9 claude
 0.5 gnome-terminal-
 0.4 gnome-shell
-->
