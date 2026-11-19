<!-- start 2026-11-16T17:49:36+01:00, waited 0 min, load 1.26 2.63 1.76
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
| cards: pan steps | 60 | 0.40 | 0.42 (0.56) | 2.50 | 0.53 (0.62) | 2.28 (3.45) | 27 | 46 | 0 (36) | 66 | 48 |
| cards: pan rest frame | 1 | 0.05 | 0.05 (0.05) | 2.57 | 0.47 (0.47) | 2.26 (2.26) | 27 | 46 | 0 (0) | 0 | 36 |
| cards: pan back steps | 60 | 0.38 | 0.42 (0.44) | 2.47 | 0.52 (0.61) | 2.32 (2.44) | 27 | 46 | 0 (36) | 63 | 48 |
| cards: pan back rest frame | 1 | 0.06 | 0.06 (0.06) | 2.42 | 0.34 (0.34) | 2.25 (2.25) | 27 | 43 | 0 (0) | 0 | 36 |
| cards: zoom in steps | 60 | 4.13 | 0.49 (0.53) | 2.51 | 0.39 (0.53) | 2.29 (2.45) | 7 | 10 | 0 (26) | 46 | 36 |
| cards: zoom in rest frame | 1 | 4.50 | 4.50 (4.50) | 1.12 | 0.34 (0.34) | 2.31 (2.31) | 0 | 0 | 0 (0) | 0 | 1 |
| cards: zoom out steps | 60 | 4.22 | 0.50 (0.98) | 2.29 | 0.40 (0.52) | 2.30 (2.45) | 7 | 523 | 0 (27) | 2517 | 36 |
| cards: zoom out rest frame | 1 | 4.08 | 4.08 (4.08) | 2.65 | 0.56 (0.56) | 2.49 (2.49) | 27 | 43 | 27 (27) | 43 | 36 |
| cards: zoom out further steps | 60 | 4.01 | 0.52 (0.76) | 1.77 | 0.83 (1.12) | 2.23 (2.40) | 94 | 156 | 0 (130) | 214 | 192 |
| cards: zoom out further rest frame | 1 | 4.77 | 4.77 (4.77) | 1.05 | 2.01 (2.01) | 3.45 (3.45) | 131 | 5 | 131 (131) | 5 | 192 |
<!-- end 2026-11-16T17:49:45+01:00, load 1.14 2.56 1.75, highest during the run 1.26
%CPU COMMAND
 1.2 jetbrains-toolb
 0.9 claude
 0.5 gnome-terminal-
 0.4 gnome-shell
 0.3 glycin-svg
-->
