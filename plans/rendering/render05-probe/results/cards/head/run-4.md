<!-- start 2026-11-16T17:12:02+01:00, waited 0 min, load 0.51 0.82 0.83
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
| cards: pan steps | 60 | 0.48 | 0.42 (0.58) | 2.50 | 0.54 (0.60) | 2.33 (3.33) | 27 | 46 | 0 (36) | 66 | 48 |
| cards: pan rest frame | 1 | 0.05 | 0.05 (0.05) | 2.57 | 0.45 (0.45) | 2.27 (2.27) | 27 | 46 | 0 (0) | 0 | 36 |
| cards: pan back steps | 60 | 0.37 | 0.42 (0.46) | 2.45 | 0.54 (0.61) | 2.33 (2.44) | 27 | 46 | 0 (36) | 63 | 48 |
| cards: pan back rest frame | 1 | 0.05 | 0.05 (0.05) | 2.41 | 0.32 (0.32) | 2.30 (2.30) | 27 | 43 | 0 (0) | 0 | 36 |
| cards: zoom in steps | 60 | 4.30 | 0.50 (0.54) | 2.50 | 0.39 (0.53) | 2.33 (2.47) | 7 | 10 | 0 (26) | 46 | 36 |
| cards: zoom in rest frame | 1 | 4.55 | 4.55 (4.55) | 1.12 | 0.34 (0.34) | 2.55 (2.55) | 0 | 0 | 0 (0) | 0 | 1 |
| cards: zoom out steps | 60 | 4.30 | 0.51 (0.97) | 2.29 | 0.38 (0.54) | 2.34 (2.49) | 7 | 523 | 0 (27) | 2517 | 36 |
| cards: zoom out rest frame | 1 | 4.07 | 4.07 (4.07) | 2.69 | 0.72 (0.72) | 2.53 (2.53) | 27 | 43 | 27 (27) | 43 | 36 |
| cards: zoom out further steps | 60 | 4.08 | 0.56 (0.79) | 1.76 | 0.94 (1.33) | 2.26 (2.39) | 94 | 156 | 0 (130) | 214 | 192 |
| cards: zoom out further rest frame | 1 | 4.75 | 4.75 (4.75) | 0.99 | 2.31 (2.31) | 3.49 (3.49) | 131 | 5 | 131 (131) | 5 | 192 |
<!-- end 2026-11-16T17:12:12+01:00, load 0.43 0.79 0.82, highest during the run 0.51
%CPU COMMAND
 1.2 jetbrains-toolb
 0.8 claude
 0.5 gnome-terminal-
 0.4 gnome-shell
 0.4 glycin-svg
-->
