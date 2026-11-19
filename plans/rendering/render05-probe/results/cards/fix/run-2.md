<!-- start 2026-11-16T17:11:05+01:00, waited 0 min, load 0.54 0.89 0.85
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
| cards: pan steps | 60 | 0.40 | 0.43 (0.58) | 2.54 | 0.54 (0.64) | 2.37 (3.24) | 27 | 46 | 0 (36) | 66 | 48 |
| cards: pan rest frame | 1 | 0.05 | 0.05 (0.05) | 2.57 | 0.49 (0.49) | 2.35 (2.35) | 27 | 46 | 0 (0) | 0 | 36 |
| cards: pan back steps | 60 | 0.35 | 0.43 (0.47) | 2.46 | 0.56 (0.61) | 2.38 (2.48) | 27 | 46 | 0 (36) | 63 | 48 |
| cards: pan back rest frame | 1 | 0.04 | 0.04 (0.04) | 2.39 | 0.45 (0.45) | 2.23 (2.23) | 27 | 43 | 0 (0) | 0 | 36 |
| cards: zoom in steps | 60 | 4.17 | 0.50 (0.54) | 2.49 | 0.40 (0.55) | 2.26 (2.42) | 7 | 10 | 0 (26) | 46 | 36 |
| cards: zoom in rest frame | 1 | 4.46 | 4.46 (4.46) | 1.14 | 0.39 (0.39) | 2.35 (2.35) | 0 | 0 | 0 (0) | 0 | 1 |
| cards: zoom out steps | 60 | 4.22 | 0.50 (1.00) | 2.28 | 0.41 (0.56) | 2.30 (2.48) | 7 | 523 | 0 (27) | 2517 | 36 |
| cards: zoom out rest frame | 1 | 4.09 | 4.09 (4.09) | 2.67 | 0.79 (0.79) | 2.58 (2.58) | 27 | 43 | 27 (27) | 43 | 36 |
| cards: zoom out further steps | 60 | 4.03 | 0.57 (0.78) | 1.76 | 0.86 (1.31) | 2.59 (2.80) | 94 | 156 | 0 (130) | 214 | 192 |
| cards: zoom out further rest frame | 1 | 4.67 | 4.67 (4.67) | 1.06 | 2.23 (2.23) | 3.76 (3.76) | 131 | 5 | 131 (131) | 5 | 192 |
<!-- end 2026-11-16T17:11:15+01:00, load 0.54 0.88 0.85, highest during the run 0.54
%CPU COMMAND
 1.2 jetbrains-toolb
 0.8 claude
 0.5 gnome-terminal-
 0.4 gnome-shell
 0.4 glycin-svg
-->
