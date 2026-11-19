<!-- start 2026-11-16T17:09:59+01:00, waited 1 min, load 1.38 1.09 0.91
%CPU COMMAND
 1.2 jetbrains-toolb
 0.9 claude
 0.5 gnome-terminal-
 0.4 gnome-shell
 0.1 kworker/u96:2-i915
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
| cards: pan steps | 60 | 0.41 | 0.44 (0.64) | 2.85 | 0.60 (0.72) | 2.18 (3.01) | 29 | 46 | 0 (36) | 66 | 0 |
| cards: pan rest frame | 1 | 0.06 | 0.06 (0.06) | 2.89 | 0.48 (0.48) | 2.16 (2.16) | 29 | 46 | 0 (0) | 0 | 0 |
| cards: pan back steps | 60 | 0.36 | 0.43 (0.47) | 2.80 | 0.62 (0.68) | 2.19 (2.35) | 29 | 46 | 0 (36) | 63 | 0 |
| cards: pan back rest frame | 1 | 0.06 | 0.06 (0.06) | 2.70 | 0.48 (0.48) | 2.02 (2.02) | 29 | 44 | 0 (0) | 0 | 0 |
| cards: zoom in steps | 60 | 4.21 | 0.50 (0.53) | 2.90 | 0.39 (0.59) | 2.16 (2.28) | 9 | 10 | 0 (28) | 47 | 0 |
| cards: zoom in rest frame | 1 | 4.56 | 4.56 (4.56) | 1.27 | 0.32 (0.32) | 2.22 (2.22) | 0 | 0 | 0 (0) | 0 | 0 |
| cards: zoom out steps | 60 | 4.34 | 0.50 (0.99) | 2.68 | 0.41 (0.60) | 2.24 (2.36) | 9 | 530 | 0 (29) | 2524 | 0 |
| cards: zoom out rest frame | 1 | 4.18 | 4.18 (4.18) | 2.98 | 0.84 (0.84) | 2.39 (2.39) | 29 | 44 | 29 (29) | 44 | 0 |
| cards: zoom out further steps | 60 | 4.13 | 0.62 (0.79) | 1.69 | 1.15 (1.67) | 2.31 (2.48) | 96 | 156 | 0 (131) | 214 | 0 |
| cards: zoom out further rest frame | 1 | 5.05 | 5.05 (5.05) | 0.80 | 2.40 (2.40) | 3.77 (3.77) | 133 | 5 | 133 (133) | 5 | 0 |
<!-- end 2026-11-16T17:10:08+01:00, load 1.17 1.05 0.90, highest during the run 1.38
%CPU COMMAND
 1.2 jetbrains-toolb
 0.8 claude
 0.5 gnome-terminal-
 0.4 gnome-shell
 0.3 glycin-svg
-->
