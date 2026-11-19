<!-- start 2026-11-16T17:49:17+01:00, waited 0 min, load 1.33 2.71 1.78
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
| cards: pan steps | 60 | 0.41 | 0.43 (0.61) | 2.48 | 0.57 (0.71) | 2.26 (3.33) | 27 | 46 | 0 (36) | 66 | 48 |
| cards: pan rest frame | 1 | 0.05 | 0.05 (0.05) | 2.55 | 0.47 (0.47) | 2.25 (2.25) | 27 | 46 | 0 (0) | 0 | 36 |
| cards: pan back steps | 60 | 0.37 | 0.43 (0.46) | 2.46 | 0.58 (0.63) | 2.26 (2.41) | 27 | 46 | 0 (36) | 63 | 48 |
| cards: pan back rest frame | 1 | 0.05 | 0.05 (0.05) | 2.40 | 0.49 (0.49) | 2.21 (2.21) | 27 | 43 | 0 (0) | 0 | 36 |
| cards: zoom in steps | 60 | 4.10 | 0.50 (0.54) | 2.50 | 0.41 (0.56) | 2.27 (2.41) | 7 | 10 | 0 (26) | 46 | 36 |
| cards: zoom in rest frame | 1 | 4.45 | 4.45 (4.45) | 1.10 | 0.34 (0.34) | 2.36 (2.36) | 0 | 0 | 0 (0) | 0 | 1 |
| cards: zoom out steps | 60 | 4.20 | 0.50 (0.97) | 2.30 | 0.41 (0.57) | 2.30 (2.46) | 7 | 523 | 0 (27) | 2517 | 36 |
| cards: zoom out rest frame | 1 | 4.09 | 4.09 (4.09) | 2.67 | 0.74 (0.74) | 2.46 (2.46) | 27 | 43 | 27 (27) | 43 | 36 |
| cards: zoom out further steps | 60 | 4.17 | 0.57 (0.76) | 1.78 | 1.03 (1.49) | 2.29 (2.44) | 94 | 156 | 0 (130) | 214 | 192 |
| cards: zoom out further rest frame | 1 | 4.83 | 4.83 (4.83) | 1.01 | 2.41 (2.41) | 3.70 (3.70) | 131 | 5 | 131 (131) | 5 | 192 |
<!-- end 2026-11-16T17:49:26+01:00, load 1.23 2.67 1.77, highest during the run 1.33
%CPU COMMAND
 1.2 jetbrains-toolb
 0.9 claude
 0.5 gnome-terminal-
 0.4 gnome-shell
 0.3 glycin-svg
-->
