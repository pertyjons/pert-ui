<!-- start 2026-11-16T17:12:21+01:00, waited 0 min, load 0.59 0.81 0.83
%CPU COMMAND
 1.2 jetbrains-toolb
 0.8 claude
 0.5 gnome-terminal-
 0.4 gnome-shell
 0.4 glycin-svg
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
| cards: pan steps | 60 | 0.40 | 0.42 (0.60) | 2.51 | 0.55 (0.70) | 2.34 (3.50) | 27 | 46 | 0 (36) | 66 | 48 |
| cards: pan rest frame | 1 | 0.05 | 0.05 (0.05) | 2.55 | 0.46 (0.46) | 2.24 (2.24) | 27 | 46 | 0 (0) | 0 | 36 |
| cards: pan back steps | 60 | 0.36 | 0.43 (0.45) | 2.45 | 0.55 (0.62) | 2.35 (2.49) | 27 | 46 | 0 (36) | 63 | 48 |
| cards: pan back rest frame | 1 | 0.04 | 0.04 (0.04) | 2.40 | 0.45 (0.45) | 2.18 (2.18) | 27 | 43 | 0 (0) | 0 | 36 |
| cards: zoom in steps | 60 | 4.09 | 0.50 (0.52) | 2.50 | 0.38 (0.52) | 2.27 (2.41) | 7 | 10 | 0 (26) | 46 | 36 |
| cards: zoom in rest frame | 1 | 4.54 | 4.54 (4.54) | 1.13 | 0.34 (0.34) | 2.29 (2.29) | 0 | 0 | 0 (0) | 0 | 1 |
| cards: zoom out steps | 60 | 4.24 | 0.50 (0.98) | 2.29 | 0.41 (0.56) | 2.28 (2.52) | 7 | 523 | 0 (27) | 2517 | 36 |
| cards: zoom out rest frame | 1 | 4.02 | 4.02 (4.02) | 2.65 | 0.75 (0.75) | 2.55 (2.55) | 27 | 43 | 27 (27) | 43 | 36 |
| cards: zoom out further steps | 60 | 4.09 | 0.57 (0.76) | 1.76 | 0.87 (1.30) | 2.56 (2.80) | 94 | 156 | 0 (130) | 214 | 192 |
| cards: zoom out further rest frame | 1 | 4.66 | 4.66 (4.66) | 1.06 | 2.28 (2.28) | 3.85 (3.85) | 131 | 5 | 131 (131) | 5 | 192 |
<!-- end 2026-11-16T17:12:31+01:00, load 0.62 0.82 0.83, highest during the run 0.59
%CPU COMMAND
 1.2 jetbrains-toolb
 0.8 claude
 0.5 gnome-terminal-
 0.4 gnome-shell
 0.4 glycin-svg
-->
