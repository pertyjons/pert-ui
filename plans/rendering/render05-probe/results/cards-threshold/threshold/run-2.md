<!-- start 2026-11-16T17:49:07+01:00, waited 0 min, load 1.39 2.77 1.79
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
| cards: pan steps | 60 | 0.43 | 0.43 (0.59) | 2.49 | 0.54 (0.66) | 2.32 (3.25) | 27 | 46 | 0 (36) | 66 | 48 |
| cards: pan rest frame | 1 | 0.04 | 0.04 (0.04) | 2.56 | 0.45 (0.45) | 2.21 (2.21) | 27 | 46 | 0 (0) | 0 | 36 |
| cards: pan back steps | 60 | 0.35 | 0.43 (0.46) | 2.45 | 0.55 (0.59) | 2.33 (2.43) | 27 | 46 | 0 (36) | 63 | 48 |
| cards: pan back rest frame | 1 | 0.07 | 0.07 (0.07) | 2.39 | 0.49 (0.49) | 2.23 (2.23) | 27 | 43 | 0 (0) | 0 | 36 |
| cards: zoom in steps | 60 | 4.09 | 0.50 (0.54) | 2.49 | 0.37 (0.53) | 2.28 (2.40) | 7 | 10 | 0 (26) | 46 | 36 |
| cards: zoom in rest frame | 1 | 4.46 | 4.46 (4.46) | 1.12 | 0.35 (0.35) | 2.38 (2.38) | 0 | 0 | 0 (0) | 0 | 1 |
| cards: zoom out steps | 60 | 4.18 | 0.52 (0.99) | 2.29 | 0.39 (0.54) | 2.29 (2.44) | 7 | 523 | 0 (27) | 2517 | 36 |
| cards: zoom out rest frame | 1 | 4.04 | 4.04 (4.04) | 2.65 | 0.72 (0.72) | 2.46 (2.46) | 27 | 43 | 27 (27) | 43 | 36 |
| cards: zoom out further steps | 60 | 4.06 | 0.57 (0.79) | 1.77 | 0.94 (1.28) | 2.26 (2.39) | 94 | 156 | 0 (130) | 214 | 192 |
| cards: zoom out further rest frame | 1 | 4.73 | 4.73 (4.73) | 1.07 | 2.29 (2.29) | 3.60 (3.60) | 131 | 5 | 131 (131) | 5 | 192 |
<!-- end 2026-11-16T17:49:17+01:00, load 1.33 2.71 1.78, highest during the run 1.39
%CPU COMMAND
 1.2 jetbrains-toolb
 0.9 claude
 0.5 gnome-terminal-
 0.4 gnome-shell
 0.2 glycin-svg
-->
