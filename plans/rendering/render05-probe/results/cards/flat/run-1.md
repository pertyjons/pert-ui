<!-- start 2026-11-16T17:10:18+01:00, waited 0 min, load 0.99 1.02 0.89
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
| cards: pan steps | 60 | 0.44 | 0.43 (0.56) | 2.51 | 0.54 (0.61) | 2.34 (3.15) | 27 | 46 | 0 (36) | 66 | 48 |
| cards: pan rest frame | 1 | 0.05 | 0.05 (0.05) | 2.55 | 0.45 (0.45) | 2.20 (2.20) | 27 | 46 | 0 (0) | 0 | 36 |
| cards: pan back steps | 60 | 0.35 | 0.43 (0.46) | 2.45 | 0.55 (0.61) | 2.36 (2.45) | 27 | 46 | 0 (36) | 63 | 48 |
| cards: pan back rest frame | 1 | 0.05 | 0.05 (0.05) | 2.39 | 0.45 (0.45) | 2.29 (2.29) | 27 | 43 | 0 (0) | 0 | 36 |
| cards: zoom in steps | 60 | 4.11 | 0.50 (0.53) | 2.49 | 0.38 (0.51) | 2.27 (2.39) | 7 | 10 | 0 (26) | 46 | 36 |
| cards: zoom in rest frame | 1 | 4.49 | 4.49 (4.49) | 1.14 | 0.21 (0.21) | 2.36 (2.36) | 0 | 0 | 0 (0) | 0 | 1 |
| cards: zoom out steps | 60 | 4.08 | 0.50 (0.98) | 2.30 | 0.39 (0.55) | 2.31 (2.55) | 7 | 523 | 0 (27) | 2517 | 36 |
| cards: zoom out rest frame | 1 | 4.12 | 4.12 (4.12) | 2.66 | 0.74 (0.74) | 2.46 (2.46) | 27 | 43 | 27 (27) | 43 | 36 |
| cards: zoom out further steps | 60 | 4.11 | 0.56 (0.78) | 1.78 | 0.92 (1.35) | 2.65 (2.91) | 94 | 156 | 0 (130) | 214 | 192 |
| cards: zoom out further rest frame | 1 | 4.71 | 4.71 (4.71) | 0.99 | 2.31 (2.31) | 4.08 (4.08) | 131 | 5 | 131 (131) | 5 | 192 |
<!-- end 2026-11-16T17:10:27+01:00, load 0.83 0.98 0.88, highest during the run 0.99
%CPU COMMAND
 1.2 jetbrains-toolb
 0.9 claude
 0.5 gnome-terminal-
 0.4 gnome-shell
 0.4 glycin-svg
-->
