<!-- start 2026-11-16T17:50:05+01:00, waited 0 min, load 0.81 2.39 1.71
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
| cards: pan steps | 60 | 0.40 | 0.42 (0.56) | 2.52 | 0.55 (0.63) | 2.30 (4.61) | 27 | 46 | 0 (36) | 66 | 48 |
| cards: pan rest frame | 1 | 0.04 | 0.04 (0.04) | 2.61 | 0.31 (0.31) | 2.19 (2.19) | 27 | 46 | 0 (0) | 0 | 36 |
| cards: pan back steps | 60 | 0.34 | 0.43 (0.46) | 2.46 | 0.56 (0.61) | 2.29 (2.42) | 27 | 46 | 0 (36) | 63 | 48 |
| cards: pan back rest frame | 1 | 0.04 | 0.04 (0.04) | 2.39 | 0.45 (0.45) | 2.30 (2.30) | 27 | 43 | 0 (0) | 0 | 36 |
| cards: zoom in steps | 60 | 4.08 | 0.50 (0.52) | 2.49 | 0.38 (0.54) | 2.26 (2.35) | 7 | 10 | 0 (26) | 46 | 36 |
| cards: zoom in rest frame | 1 | 4.50 | 4.50 (4.50) | 1.11 | 0.34 (0.34) | 2.35 (2.35) | 0 | 0 | 0 (0) | 0 | 1 |
| cards: zoom out steps | 60 | 4.17 | 0.50 (0.98) | 2.28 | 0.40 (0.56) | 2.27 (2.40) | 7 | 523 | 0 (27) | 2517 | 36 |
| cards: zoom out rest frame | 1 | 4.03 | 4.03 (4.03) | 2.65 | 0.73 (0.73) | 2.59 (2.59) | 27 | 43 | 27 (27) | 43 | 36 |
| cards: zoom out further steps | 60 | 4.07 | 0.56 (0.75) | 1.77 | 0.90 (1.31) | 2.31 (2.49) | 94 | 156 | 0 (130) | 214 | 192 |
| cards: zoom out further rest frame | 1 | 4.73 | 4.73 (4.73) | 1.07 | 2.25 (2.25) | 3.55 (3.55) | 131 | 5 | 131 (131) | 5 | 192 |
<!-- end 2026-11-16T17:50:14+01:00, load 0.76 2.33 1.70, highest during the run 0.81
%CPU COMMAND
 1.2 jetbrains-toolb
 0.9 claude
 0.5 gnome-terminal-
 0.4 gnome-shell
 0.3 glycin-svg
-->
