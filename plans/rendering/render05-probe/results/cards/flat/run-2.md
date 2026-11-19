<!-- start 2026-11-16T17:10:56+01:00, waited 0 min, load 0.55 0.90 0.86
%CPU COMMAND
 100 ps
 1.2 jetbrains-toolb
 0.8 claude
 0.5 gnome-terminal-
 0.4 gnome-shell
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
| cards: pan steps | 60 | 0.40 | 0.42 (0.60) | 2.59 | 0.54 (0.60) | 2.32 (2.49) | 27 | 46 | 0 (36) | 66 | 48 |
| cards: pan rest frame | 1 | 0.05 | 0.05 (0.05) | 2.64 | 0.46 (0.46) | 2.22 (2.22) | 27 | 46 | 0 (0) | 0 | 36 |
| cards: pan back steps | 60 | 0.35 | 0.42 (0.45) | 2.53 | 0.55 (0.59) | 2.33 (2.48) | 27 | 46 | 0 (36) | 63 | 48 |
| cards: pan back rest frame | 1 | 0.05 | 0.05 (0.05) | 2.47 | 0.43 (0.43) | 2.24 (2.24) | 27 | 43 | 0 (0) | 0 | 36 |
| cards: zoom in steps | 60 | 4.09 | 0.50 (0.54) | 2.57 | 0.38 (0.53) | 2.28 (2.40) | 7 | 10 | 0 (26) | 46 | 36 |
| cards: zoom in rest frame | 1 | 4.45 | 4.45 (4.45) | 1.31 | 0.33 (0.33) | 2.23 (2.23) | 0 | 0 | 0 (0) | 0 | 1 |
| cards: zoom out steps | 60 | 4.22 | 0.50 (0.99) | 2.37 | 0.39 (0.53) | 2.28 (2.44) | 7 | 523 | 0 (27) | 2517 | 36 |
| cards: zoom out rest frame | 1 | 4.01 | 4.01 (4.01) | 2.77 | 0.76 (0.76) | 2.44 (2.44) | 27 | 43 | 27 (27) | 43 | 36 |
| cards: zoom out further steps | 60 | 4.12 | 0.57 (0.74) | 1.86 | 0.92 (1.33) | 2.59 (2.84) | 94 | 156 | 0 (130) | 214 | 192 |
| cards: zoom out further rest frame | 1 | 4.77 | 4.77 (4.77) | 1.11 | 2.06 (2.06) | 3.97 (3.97) | 131 | 5 | 131 (131) | 5 | 192 |
<!-- end 2026-11-16T17:11:05+01:00, load 0.54 0.89 0.85, highest during the run 0.55
%CPU COMMAND
 1.2 jetbrains-toolb
 0.8 claude
 0.5 gnome-terminal-
 0.4 gnome-shell
 0.4 glycin-svg
-->
