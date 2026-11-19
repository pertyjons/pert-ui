<!-- start 2026-11-16T17:11:34+01:00, waited 0 min, load 0.54 0.85 0.84
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
| cards: pan steps | 60 | 0.40 | 0.43 (0.61) | 2.50 | 0.54 (0.65) | 2.34 (4.06) | 27 | 46 | 0 (36) | 66 | 48 |
| cards: pan rest frame | 1 | 0.05 | 0.05 (0.05) | 2.56 | 0.47 (0.47) | 2.32 (2.32) | 27 | 46 | 0 (0) | 0 | 36 |
| cards: pan back steps | 60 | 0.37 | 0.43 (0.45) | 2.45 | 0.56 (0.61) | 2.34 (2.45) | 27 | 46 | 0 (36) | 63 | 48 |
| cards: pan back rest frame | 1 | 0.05 | 0.05 (0.05) | 2.41 | 0.45 (0.45) | 2.23 (2.23) | 27 | 43 | 0 (0) | 0 | 36 |
| cards: zoom in steps | 60 | 4.16 | 0.50 (0.54) | 2.50 | 0.37 (0.51) | 2.27 (2.41) | 7 | 10 | 0 (26) | 46 | 36 |
| cards: zoom in rest frame | 1 | 4.50 | 4.50 (4.50) | 1.10 | 0.33 (0.33) | 2.36 (2.36) | 0 | 0 | 0 (0) | 0 | 1 |
| cards: zoom out steps | 60 | 4.24 | 0.50 (0.99) | 2.29 | 0.39 (0.56) | 2.31 (2.47) | 7 | 523 | 0 (27) | 2517 | 36 |
| cards: zoom out rest frame | 1 | 4.15 | 4.15 (4.15) | 2.66 | 0.76 (0.76) | 2.57 (2.57) | 27 | 43 | 27 (27) | 43 | 36 |
| cards: zoom out further steps | 60 | 4.14 | 0.57 (0.76) | 1.77 | 0.96 (1.37) | 2.60 (2.89) | 94 | 156 | 0 (130) | 214 | 192 |
| cards: zoom out further rest frame | 1 | 4.69 | 4.69 (4.69) | 1.06 | 2.32 (2.32) | 3.98 (3.98) | 131 | 5 | 131 (131) | 5 | 192 |
<!-- end 2026-11-16T17:11:43+01:00, load 0.61 0.86 0.84, highest during the run 0.54
%CPU COMMAND
 1.2 jetbrains-toolb
 0.8 claude
 0.5 gnome-terminal-
 0.4 gnome-shell
 0.4 glycin-svg
-->
