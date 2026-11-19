<!-- start 2026-11-16T17:11:53+01:00, waited 0 min, load 0.60 0.85 0.84
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
| cards: pan steps | 60 | 0.43 | 0.43 (0.59) | 2.81 | 0.60 (0.68) | 2.17 (2.72) | 29 | 46 | 0 (36) | 66 | 0 |
| cards: pan rest frame | 1 | 0.06 | 0.06 (0.06) | 2.89 | 0.49 (0.49) | 2.08 (2.08) | 29 | 46 | 0 (0) | 0 | 0 |
| cards: pan back steps | 60 | 0.36 | 0.44 (0.48) | 2.76 | 0.61 (0.68) | 2.21 (2.37) | 29 | 46 | 0 (36) | 63 | 0 |
| cards: pan back rest frame | 1 | 0.06 | 0.06 (0.06) | 2.70 | 0.47 (0.47) | 2.22 (2.22) | 29 | 44 | 0 (0) | 0 | 0 |
| cards: zoom in steps | 60 | 4.16 | 0.50 (0.52) | 2.89 | 0.39 (0.58) | 2.16 (2.26) | 9 | 10 | 0 (28) | 47 | 0 |
| cards: zoom in rest frame | 1 | 4.58 | 4.58 (4.58) | 1.28 | 0.34 (0.34) | 2.12 (2.12) | 0 | 0 | 0 (0) | 0 | 0 |
| cards: zoom out steps | 60 | 4.31 | 0.50 (0.99) | 2.70 | 0.41 (0.62) | 2.20 (2.33) | 9 | 530 | 0 (29) | 2524 | 0 |
| cards: zoom out rest frame | 1 | 4.14 | 4.14 (4.14) | 2.98 | 0.83 (0.83) | 2.45 (2.45) | 29 | 44 | 29 (29) | 44 | 0 |
| cards: zoom out further steps | 60 | 4.14 | 0.62 (0.80) | 1.69 | 1.17 (1.67) | 2.28 (2.51) | 96 | 156 | 0 (131) | 214 | 0 |
| cards: zoom out further rest frame | 1 | 4.94 | 4.94 (4.94) | 0.79 | 2.55 (2.55) | 3.65 (3.65) | 133 | 5 | 133 (133) | 5 | 0 |
<!-- end 2026-11-16T17:12:02+01:00, load 0.51 0.82 0.83, highest during the run 0.60
%CPU COMMAND
 1.2 jetbrains-toolb
 0.9 claude
 0.5 gnome-terminal-
 0.4 gnome-shell
 0.4 glycin-svg
-->
