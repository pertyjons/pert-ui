<!-- start 2026-11-16T17:49:27+01:00, waited 0 min, load 1.23 2.67 1.77
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
| cards: pan steps | 60 | 0.41 | 0.43 (0.56) | 2.50 | 0.53 (0.61) | 2.39 (5.37) | 27 | 46 | 0 (36) | 66 | 48 |
| cards: pan rest frame | 1 | 0.05 | 0.05 (0.05) | 2.57 | 0.45 (0.45) | 2.16 (2.16) | 27 | 46 | 0 (0) | 0 | 36 |
| cards: pan back steps | 60 | 0.35 | 0.43 (0.45) | 2.46 | 0.54 (0.60) | 2.38 (2.55) | 27 | 46 | 0 (36) | 63 | 48 |
| cards: pan back rest frame | 1 | 0.05 | 0.05 (0.05) | 2.41 | 0.45 (0.45) | 2.30 (2.30) | 27 | 43 | 0 (0) | 0 | 36 |
| cards: zoom in steps | 60 | 4.10 | 0.49 (0.52) | 2.50 | 0.38 (0.52) | 2.37 (2.57) | 7 | 10 | 0 (26) | 46 | 36 |
| cards: zoom in rest frame | 1 | 4.56 | 4.56 (4.56) | 1.12 | 0.33 (0.33) | 2.50 (2.50) | 0 | 0 | 0 (0) | 0 | 1 |
| cards: zoom out steps | 60 | 4.20 | 0.50 (0.99) | 2.29 | 0.40 (0.55) | 2.39 (2.55) | 7 | 523 | 0 (27) | 2517 | 36 |
| cards: zoom out rest frame | 1 | 4.16 | 4.16 (4.16) | 2.66 | 0.76 (0.76) | 2.49 (2.49) | 27 | 43 | 27 (27) | 43 | 36 |
| cards: zoom out further steps | 60 | 4.10 | 0.57 (0.74) | 1.77 | 0.96 (1.35) | 2.28 (2.42) | 94 | 156 | 0 (130) | 214 | 192 |
| cards: zoom out further rest frame | 1 | 4.84 | 4.84 (4.84) | 1.02 | 2.45 (2.45) | 3.44 (3.44) | 131 | 5 | 131 (131) | 5 | 192 |
<!-- end 2026-11-16T17:49:36+01:00, load 1.26 2.63 1.76, highest during the run 1.23
%CPU COMMAND
 1.2 jetbrains-toolb
 0.9 claude
 0.5 gnome-terminal-
 0.4 gnome-shell
 0.3 glycin-svg
-->
