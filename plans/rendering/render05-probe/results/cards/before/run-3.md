<!-- start 2026-11-16T17:11:15+01:00, waited 0 min, load 0.54 0.88 0.85
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
| cards: pan steps | 60 | 0.44 | 0.44 (0.62) | 2.86 | 0.59 (0.74) | 2.11 (2.50) | 29 | 46 | 0 (36) | 66 | 0 |
| cards: pan rest frame | 1 | 0.06 | 0.06 (0.06) | 2.88 | 0.48 (0.48) | 2.24 (2.24) | 29 | 46 | 0 (0) | 0 | 0 |
| cards: pan back steps | 60 | 0.37 | 0.44 (0.48) | 2.78 | 0.61 (0.67) | 2.20 (2.28) | 29 | 46 | 0 (36) | 63 | 0 |
| cards: pan back rest frame | 1 | 0.06 | 0.06 (0.06) | 2.70 | 0.51 (0.51) | 2.14 (2.14) | 29 | 44 | 0 (0) | 0 | 0 |
| cards: zoom in steps | 60 | 4.19 | 0.50 (0.55) | 2.90 | 0.37 (0.59) | 2.15 (2.30) | 9 | 10 | 0 (28) | 47 | 0 |
| cards: zoom in rest frame | 1 | 4.70 | 4.70 (4.70) | 1.27 | 0.33 (0.33) | 2.06 (2.06) | 0 | 0 | 0 (0) | 0 | 0 |
| cards: zoom out steps | 60 | 4.32 | 0.50 (0.98) | 2.69 | 0.40 (0.61) | 2.21 (2.39) | 9 | 530 | 0 (29) | 2524 | 0 |
| cards: zoom out rest frame | 1 | 4.14 | 4.14 (4.14) | 2.99 | 0.80 (0.80) | 2.42 (2.42) | 29 | 44 | 29 (29) | 44 | 0 |
| cards: zoom out further steps | 60 | 4.16 | 0.62 (0.83) | 1.69 | 1.17 (1.53) | 2.36 (2.56) | 96 | 156 | 0 (131) | 214 | 0 |
| cards: zoom out further rest frame | 1 | 5.00 | 5.00 (5.00) | 0.78 | 2.60 (2.60) | 3.83 (3.83) | 133 | 5 | 133 (133) | 5 | 0 |
<!-- end 2026-11-16T17:11:24+01:00, load 0.45 0.85 0.84, highest during the run 0.54
%CPU COMMAND
 1.2 jetbrains-toolb
 0.8 claude
 0.5 gnome-terminal-
 0.4 gnome-shell
 0.4 glycin-svg
-->
