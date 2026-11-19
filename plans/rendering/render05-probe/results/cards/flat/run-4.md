<!-- start 2026-11-16T17:12:12+01:00, waited 0 min, load 0.43 0.79 0.82
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
| cards: pan steps | 60 | 0.43 | 0.44 (0.60) | 2.50 | 0.55 (0.64) | 2.34 (2.94) | 27 | 46 | 0 (36) | 66 | 48 |
| cards: pan rest frame | 1 | 0.05 | 0.05 (0.05) | 2.56 | 0.45 (0.45) | 2.23 (2.23) | 27 | 46 | 0 (0) | 0 | 36 |
| cards: pan back steps | 60 | 0.34 | 0.43 (0.45) | 2.45 | 0.55 (0.60) | 2.36 (2.45) | 27 | 46 | 0 (36) | 63 | 48 |
| cards: pan back rest frame | 1 | 0.05 | 0.05 (0.05) | 2.40 | 0.44 (0.44) | 2.46 (2.46) | 27 | 43 | 0 (0) | 0 | 36 |
| cards: zoom in steps | 60 | 4.12 | 0.50 (0.53) | 2.50 | 0.37 (0.54) | 2.25 (2.42) | 7 | 10 | 0 (26) | 46 | 36 |
| cards: zoom in rest frame | 1 | 4.44 | 4.44 (4.44) | 1.13 | 0.33 (0.33) | 2.31 (2.31) | 0 | 0 | 0 (0) | 0 | 1 |
| cards: zoom out steps | 60 | 4.16 | 0.50 (1.03) | 2.28 | 0.40 (0.54) | 2.29 (2.47) | 7 | 523 | 0 (27) | 2517 | 36 |
| cards: zoom out rest frame | 1 | 4.11 | 4.11 (4.11) | 2.67 | 0.76 (0.76) | 2.43 (2.43) | 27 | 43 | 27 (27) | 43 | 36 |
| cards: zoom out further steps | 60 | 4.16 | 0.57 (0.74) | 1.77 | 0.91 (1.35) | 2.59 (2.84) | 94 | 156 | 0 (130) | 214 | 192 |
| cards: zoom out further rest frame | 1 | 4.71 | 4.71 (4.71) | 1.03 | 2.11 (2.11) | 4.06 (4.06) | 131 | 5 | 131 (131) | 5 | 192 |
<!-- end 2026-11-16T17:12:21+01:00, load 0.59 0.81 0.83, highest during the run 0.43
%CPU COMMAND
 1.2 jetbrains-toolb
 0.8 claude
 0.5 gnome-terminal-
 0.4 gnome-shell
 0.4 glycin-svg
-->
