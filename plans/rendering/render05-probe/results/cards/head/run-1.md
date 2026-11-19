<!-- start 2026-11-16T17:10:08+01:00, waited 0 min, load 1.17 1.05 0.90
%CPU COMMAND
 1.2 jetbrains-toolb
 0.8 claude
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
| cards: pan steps | 60 | 0.42 | 0.42 (0.58) | 2.50 | 0.53 (0.61) | 2.28 (3.51) | 27 | 46 | 0 (36) | 66 | 48 |
| cards: pan rest frame | 1 | 0.05 | 0.05 (0.05) | 2.57 | 0.44 (0.44) | 2.40 (2.40) | 27 | 46 | 0 (0) | 0 | 36 |
| cards: pan back steps | 60 | 0.37 | 0.43 (0.45) | 2.46 | 0.54 (0.59) | 2.31 (2.51) | 27 | 46 | 0 (36) | 63 | 48 |
| cards: pan back rest frame | 1 | 0.05 | 0.05 (0.05) | 2.40 | 0.45 (0.45) | 2.11 (2.11) | 27 | 43 | 0 (0) | 0 | 36 |
| cards: zoom in steps | 60 | 4.17 | 0.50 (0.53) | 2.49 | 0.39 (0.54) | 2.32 (2.55) | 7 | 10 | 0 (26) | 46 | 36 |
| cards: zoom in rest frame | 1 | 4.46 | 4.46 (4.46) | 1.13 | 0.32 (0.32) | 2.44 (2.44) | 0 | 0 | 0 (0) | 0 | 1 |
| cards: zoom out steps | 60 | 4.27 | 0.51 (0.98) | 2.29 | 0.39 (0.55) | 2.34 (2.54) | 7 | 523 | 0 (27) | 2517 | 36 |
| cards: zoom out rest frame | 1 | 4.11 | 4.11 (4.11) | 2.66 | 0.74 (0.74) | 2.51 (2.51) | 27 | 43 | 27 (27) | 43 | 36 |
| cards: zoom out further steps | 60 | 4.16 | 0.57 (0.75) | 1.76 | 0.91 (1.35) | 2.25 (2.40) | 94 | 156 | 0 (130) | 214 | 192 |
| cards: zoom out further rest frame | 1 | 4.71 | 4.71 (4.71) | 1.00 | 2.30 (2.30) | 3.49 (3.49) | 131 | 5 | 131 (131) | 5 | 192 |
<!-- end 2026-11-16T17:10:18+01:00, load 0.99 1.02 0.89, highest during the run 1.17
%CPU COMMAND
 1.2 jetbrains-toolb
 0.8 claude
 0.5 gnome-terminal-
 0.4 gnome-shell
 0.3 glycin-svg
-->
