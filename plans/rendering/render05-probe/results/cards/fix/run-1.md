<!-- start 2026-11-16T17:10:27+01:00, waited 0 min, load 0.83 0.98 0.88
%CPU COMMAND
 1.2 jetbrains-toolb
 0.9 claude
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
| cards: pan steps | 60 | 0.42 | 0.43 (0.60) | 2.51 | 0.54 (0.60) | 2.33 (4.61) | 27 | 46 | 0 (36) | 66 | 48 |
| cards: pan rest frame | 1 | 0.05 | 0.05 (0.05) | 3.16 | 0.46 (0.46) | 2.30 (2.30) | 27 | 46 | 0 (0) | 0 | 36 |
| cards: pan back steps | 60 | 0.35 | 0.43 (0.45) | 2.46 | 0.55 (0.61) | 2.35 (2.44) | 27 | 46 | 0 (36) | 63 | 48 |
| cards: pan back rest frame | 1 | 0.05 | 0.05 (0.05) | 2.39 | 0.43 (0.43) | 2.27 (2.27) | 27 | 43 | 0 (0) | 0 | 36 |
| cards: zoom in steps | 60 | 4.10 | 0.50 (0.53) | 2.50 | 0.40 (0.53) | 2.24 (2.37) | 7 | 10 | 0 (26) | 46 | 36 |
| cards: zoom in rest frame | 1 | 4.48 | 4.48 (4.48) | 1.13 | 0.34 (0.34) | 2.28 (2.28) | 0 | 0 | 0 (0) | 0 | 1 |
| cards: zoom out steps | 60 | 4.18 | 0.50 (1.01) | 2.31 | 0.41 (0.55) | 2.28 (2.60) | 7 | 523 | 0 (27) | 2517 | 36 |
| cards: zoom out rest frame | 1 | 4.08 | 4.08 (4.08) | 2.65 | 0.76 (0.76) | 2.52 (2.52) | 27 | 43 | 27 (27) | 43 | 36 |
| cards: zoom out further steps | 60 | 4.04 | 0.57 (0.81) | 1.78 | 0.90 (1.32) | 2.63 (2.79) | 94 | 156 | 0 (130) | 214 | 192 |
| cards: zoom out further rest frame | 1 | 4.68 | 4.68 (4.68) | 1.06 | 2.21 (2.21) | 3.93 (3.93) | 131 | 5 | 131 (131) | 5 | 192 |
<!-- end 2026-11-16T17:10:37+01:00, load 0.71 0.95 0.87, highest during the run 0.83
%CPU COMMAND
 1.2 jetbrains-toolb
 0.9 claude
 0.5 gnome-terminal-
 0.4 gnome-shell
 0.4 glycin-svg
-->
