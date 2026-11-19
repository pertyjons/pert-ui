<!-- start 2026-11-16T17:12:31+01:00, waited 0 min, load 0.62 0.82 0.83
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
| cards: pan steps | 60 | 0.43 | 0.44 (0.60) | 2.81 | 0.61 (0.68) | 2.17 (2.65) | 29 | 46 | 0 (36) | 66 | 0 |
| cards: pan rest frame | 1 | 0.06 | 0.06 (0.06) | 2.89 | 0.49 (0.49) | 2.13 (2.13) | 29 | 46 | 0 (0) | 0 | 0 |
| cards: pan back steps | 60 | 0.36 | 0.44 (0.46) | 2.77 | 0.62 (0.69) | 2.19 (2.25) | 29 | 46 | 0 (36) | 63 | 0 |
| cards: pan back rest frame | 1 | 0.06 | 0.06 (0.06) | 2.70 | 0.50 (0.50) | 2.06 (2.06) | 29 | 44 | 0 (0) | 0 | 0 |
| cards: zoom in steps | 60 | 4.21 | 0.50 (0.52) | 2.89 | 0.39 (0.59) | 2.15 (2.23) | 9 | 10 | 0 (28) | 47 | 0 |
| cards: zoom in rest frame | 1 | 4.60 | 4.60 (4.60) | 1.33 | 0.34 (0.34) | 2.21 (2.21) | 0 | 0 | 0 (0) | 0 | 0 |
| cards: zoom out steps | 60 | 4.39 | 0.50 (0.99) | 2.70 | 0.40 (0.62) | 2.20 (2.33) | 9 | 530 | 0 (29) | 2524 | 0 |
| cards: zoom out rest frame | 1 | 4.09 | 4.09 (4.09) | 2.98 | 0.82 (0.82) | 2.36 (2.36) | 29 | 44 | 29 (29) | 44 | 0 |
| cards: zoom out further steps | 60 | 4.13 | 0.62 (0.80) | 1.68 | 1.14 (1.64) | 2.28 (2.50) | 96 | 156 | 0 (131) | 214 | 0 |
| cards: zoom out further rest frame | 1 | 4.99 | 4.99 (4.99) | 0.81 | 2.61 (2.61) | 3.76 (3.76) | 133 | 5 | 133 (133) | 5 | 0 |
<!-- end 2026-11-16T17:12:40+01:00, load 0.98 0.89 0.85, highest during the run 0.62
%CPU COMMAND
 1.2 jetbrains-toolb
 0.8 claude
 0.5 gnome-terminal-
 0.4 gnome-shell
 0.4 glycin-svg
-->
