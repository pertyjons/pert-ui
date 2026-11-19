<!-- start 2026-11-16T17:12:41+01:00, waited 0 min, load 0.98 0.89 0.85
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
| cards: pan steps | 60 | 0.40 | 0.43 (0.59) | 2.51 | 0.54 (0.60) | 2.33 (2.49) | 27 | 46 | 0 (36) | 66 | 48 |
| cards: pan rest frame | 1 | 0.05 | 0.05 (0.05) | 2.57 | 0.43 (0.43) | 2.26 (2.26) | 27 | 46 | 0 (0) | 0 | 36 |
| cards: pan back steps | 60 | 0.36 | 0.43 (0.47) | 2.45 | 0.54 (0.61) | 2.34 (2.45) | 27 | 46 | 0 (36) | 63 | 48 |
| cards: pan back rest frame | 1 | 0.05 | 0.05 (0.05) | 2.40 | 0.42 (0.42) | 2.23 (2.23) | 27 | 43 | 0 (0) | 0 | 36 |
| cards: zoom in steps | 60 | 4.12 | 0.50 (0.54) | 2.50 | 0.37 (0.53) | 2.34 (2.48) | 7 | 10 | 0 (26) | 46 | 36 |
| cards: zoom in rest frame | 1 | 4.52 | 4.52 (4.52) | 1.16 | 0.32 (0.32) | 2.53 (2.53) | 0 | 0 | 0 (0) | 0 | 1 |
| cards: zoom out steps | 60 | 4.26 | 0.50 (0.99) | 2.29 | 0.40 (0.58) | 2.34 (2.49) | 7 | 523 | 0 (27) | 2517 | 36 |
| cards: zoom out rest frame | 1 | 4.16 | 4.16 (4.16) | 2.66 | 0.74 (0.74) | 2.48 (2.48) | 27 | 43 | 27 (27) | 43 | 36 |
| cards: zoom out further steps | 60 | 4.07 | 0.56 (0.73) | 1.76 | 0.89 (1.23) | 2.26 (2.41) | 94 | 156 | 0 (130) | 214 | 192 |
| cards: zoom out further rest frame | 1 | 4.77 | 4.77 (4.77) | 1.00 | 2.32 (2.32) | 3.47 (3.47) | 131 | 5 | 131 (131) | 5 | 192 |
<!-- end 2026-11-16T17:12:50+01:00, load 0.91 0.88 0.85, highest during the run 0.98
%CPU COMMAND
 100 ps
 1.2 jetbrains-toolb
 0.8 claude
 0.5 gnome-terminal-
 0.4 gnome-shell
-->
