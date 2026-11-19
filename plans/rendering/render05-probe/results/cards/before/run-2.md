<!-- start 2026-11-16T17:10:37+01:00, waited 0 min, load 0.71 0.95 0.87
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
| cards: pan steps | 60 | 0.48 | 0.44 (0.57) | 2.83 | 0.61 (0.68) | 2.19 (2.69) | 29 | 46 | 0 (36) | 66 | 0 |
| cards: pan rest frame | 1 | 0.06 | 0.06 (0.06) | 2.89 | 0.47 (0.47) | 2.17 (2.17) | 29 | 46 | 0 (0) | 0 | 0 |
| cards: pan back steps | 60 | 0.36 | 0.43 (0.46) | 2.77 | 0.61 (0.66) | 2.20 (2.29) | 29 | 46 | 0 (36) | 63 | 0 |
| cards: pan back rest frame | 1 | 0.05 | 0.05 (0.05) | 2.73 | 0.32 (0.32) | 2.24 (2.24) | 29 | 44 | 0 (0) | 0 | 0 |
| cards: zoom in steps | 60 | 4.24 | 0.50 (0.52) | 2.91 | 0.38 (0.56) | 2.15 (2.29) | 9 | 10 | 0 (28) | 47 | 0 |
| cards: zoom in rest frame | 1 | 4.58 | 4.58 (4.58) | 1.34 | 0.33 (0.33) | 2.16 (2.16) | 0 | 0 | 0 (0) | 0 | 0 |
| cards: zoom out steps | 60 | 4.30 | 0.51 (1.00) | 2.69 | 0.39 (0.67) | 2.19 (2.32) | 9 | 530 | 0 (29) | 2524 | 0 |
| cards: zoom out rest frame | 1 | 4.13 | 4.13 (4.13) | 2.99 | 0.82 (0.82) | 2.29 (2.29) | 29 | 44 | 29 (29) | 44 | 0 |
| cards: zoom out further steps | 60 | 4.16 | 0.62 (0.79) | 1.68 | 1.18 (1.66) | 2.32 (2.59) | 96 | 156 | 0 (131) | 214 | 0 |
| cards: zoom out further rest frame | 1 | 4.91 | 4.91 (4.91) | 0.84 | 2.60 (2.60) | 3.77 (3.77) | 133 | 5 | 133 (133) | 5 | 0 |
<!-- end 2026-11-16T17:10:46+01:00, load 0.60 0.92 0.86, highest during the run 0.71
%CPU COMMAND
 1.2 jetbrains-toolb
 0.8 claude
 0.5 gnome-terminal-
 0.4 gnome-shell
 0.4 glycin-svg
-->
