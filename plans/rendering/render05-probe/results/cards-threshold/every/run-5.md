<!-- start 2026-11-16T17:50:43+01:00, waited 0 min, load 0.54 2.12 1.65
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
| cards: pan steps | 60 | 0.40 | 0.43 (0.60) | 2.50 | 0.57 (0.64) | 2.24 (2.78) | 27 | 46 | 0 (36) | 66 | 48 |
| cards: pan rest frame | 1 | 0.05 | 0.05 (0.05) | 2.58 | 0.46 (0.46) | 2.15 (2.15) | 27 | 46 | 0 (0) | 0 | 36 |
| cards: pan back steps | 60 | 0.36 | 0.43 (0.46) | 2.46 | 0.58 (0.64) | 2.31 (2.41) | 27 | 46 | 0 (36) | 63 | 48 |
| cards: pan back rest frame | 1 | 0.04 | 0.04 (0.04) | 2.38 | 0.48 (0.48) | 2.26 (2.26) | 27 | 43 | 0 (0) | 0 | 36 |
| cards: zoom in steps | 60 | 4.17 | 0.50 (0.52) | 2.50 | 0.40 (0.56) | 2.29 (2.43) | 7 | 10 | 0 (26) | 46 | 36 |
| cards: zoom in rest frame | 1 | 4.45 | 4.45 (4.45) | 1.09 | 0.35 (0.35) | 2.43 (2.43) | 0 | 0 | 0 (0) | 0 | 1 |
| cards: zoom out steps | 60 | 4.27 | 0.50 (0.99) | 2.28 | 0.42 (0.58) | 2.33 (2.48) | 7 | 523 | 0 (27) | 2517 | 36 |
| cards: zoom out rest frame | 1 | 4.11 | 4.11 (4.11) | 2.67 | 0.76 (0.76) | 2.47 (2.47) | 27 | 43 | 27 (27) | 43 | 36 |
| cards: zoom out further steps | 60 | 4.16 | 0.56 (0.75) | 1.77 | 1.03 (1.35) | 2.35 (2.49) | 94 | 156 | 0 (130) | 214 | 192 |
| cards: zoom out further rest frame | 1 | 4.83 | 4.83 (4.83) | 1.01 | 2.20 (2.20) | 3.68 (3.68) | 131 | 5 | 131 (131) | 5 | 192 |
<!-- end 2026-11-16T17:50:52+01:00, load 0.53 2.07 1.64, highest during the run 0.54
%CPU COMMAND
 1.2 jetbrains-toolb
 0.9 claude
 0.5 gnome-terminal-
 0.4 gnome-shell
 0.3 glycin-svg
-->
