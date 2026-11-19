<!-- start 2026-11-16T17:48:48+01:00, waited 0 min, load 1.46 2.88 1.80
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
| cards: pan steps | 60 | 0.45 | 0.42 (0.58) | 2.50 | 0.57 (0.64) | 2.29 (4.67) | 27 | 46 | 0 (36) | 66 | 48 |
| cards: pan rest frame | 1 | 0.05 | 0.05 (0.05) | 2.57 | 0.33 (0.33) | 2.30 (2.30) | 27 | 46 | 0 (0) | 0 | 36 |
| cards: pan back steps | 60 | 0.35 | 0.43 (0.45) | 2.45 | 0.57 (0.65) | 2.24 (2.37) | 27 | 46 | 0 (36) | 63 | 48 |
| cards: pan back rest frame | 1 | 0.04 | 0.04 (0.04) | 2.39 | 0.51 (0.51) | 2.16 (2.16) | 27 | 43 | 0 (0) | 0 | 36 |
| cards: zoom in steps | 60 | 4.15 | 0.50 (0.52) | 2.49 | 0.41 (0.55) | 2.27 (2.39) | 7 | 10 | 0 (26) | 46 | 36 |
| cards: zoom in rest frame | 1 | 4.49 | 4.49 (4.49) | 1.12 | 0.34 (0.34) | 2.22 (2.22) | 0 | 0 | 0 (0) | 0 | 1 |
| cards: zoom out steps | 60 | 4.23 | 0.50 (0.99) | 2.28 | 0.41 (0.57) | 2.29 (2.43) | 7 | 523 | 0 (27) | 2517 | 36 |
| cards: zoom out rest frame | 1 | 4.12 | 4.12 (4.12) | 2.66 | 0.78 (0.78) | 2.37 (2.37) | 27 | 43 | 27 (27) | 43 | 36 |
| cards: zoom out further steps | 60 | 4.06 | 0.57 (0.74) | 1.76 | 1.01 (1.49) | 2.28 (2.42) | 94 | 156 | 0 (130) | 214 | 192 |
| cards: zoom out further rest frame | 1 | 4.79 | 4.79 (4.79) | 1.00 | 2.37 (2.37) | 3.56 (3.56) | 131 | 5 | 131 (131) | 5 | 192 |
<!-- end 2026-11-16T17:48:58+01:00, load 1.38 2.82 1.79, highest during the run 1.46
%CPU COMMAND
 1.2 jetbrains-toolb
 0.9 claude
 0.5 gnome-terminal-
 0.4 gnome-shell
 0.2 glycin-svg
-->
