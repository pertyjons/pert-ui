<!-- start 2026-11-16T17:49:55+01:00, waited 0 min, load 0.96 2.47 1.73
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
| cards: pan steps | 60 | 0.40 | 0.43 (0.60) | 2.50 | 0.54 (0.60) | 2.33 (3.45) | 27 | 46 | 0 (36) | 66 | 48 |
| cards: pan rest frame | 1 | 0.05 | 0.05 (0.05) | 2.58 | 0.45 (0.45) | 2.20 (2.20) | 27 | 46 | 0 (0) | 0 | 36 |
| cards: pan back steps | 60 | 0.35 | 0.43 (0.46) | 2.44 | 0.55 (0.60) | 2.32 (2.42) | 27 | 46 | 0 (36) | 63 | 48 |
| cards: pan back rest frame | 1 | 0.04 | 0.04 (0.04) | 2.42 | 0.25 (0.25) | 2.33 (2.33) | 27 | 43 | 0 (0) | 0 | 36 |
| cards: zoom in steps | 60 | 4.27 | 0.50 (0.53) | 2.49 | 0.37 (0.54) | 2.33 (2.47) | 7 | 10 | 0 (26) | 46 | 36 |
| cards: zoom in rest frame | 1 | 4.49 | 4.49 (4.49) | 1.10 | 0.34 (0.34) | 2.51 (2.51) | 0 | 0 | 0 (0) | 0 | 1 |
| cards: zoom out steps | 60 | 4.22 | 0.50 (0.99) | 2.29 | 0.39 (0.55) | 2.38 (2.48) | 7 | 523 | 0 (27) | 2517 | 36 |
| cards: zoom out rest frame | 1 | 4.12 | 4.12 (4.12) | 2.65 | 0.75 (0.75) | 2.46 (2.46) | 27 | 43 | 27 (27) | 43 | 36 |
| cards: zoom out further steps | 60 | 4.09 | 0.57 (0.76) | 1.75 | 0.90 (1.35) | 2.25 (2.43) | 94 | 156 | 0 (130) | 214 | 192 |
| cards: zoom out further rest frame | 1 | 4.73 | 4.73 (4.73) | 1.05 | 2.33 (2.33) | 3.52 (3.52) | 131 | 5 | 131 (131) | 5 | 192 |
<!-- end 2026-11-16T17:50:05+01:00, load 0.81 2.39 1.71, highest during the run 0.96
%CPU COMMAND
 1.2 jetbrains-toolb
 0.9 claude
 0.5 gnome-terminal-
 0.4 gnome-shell
 0.3 glycin-svg
-->
