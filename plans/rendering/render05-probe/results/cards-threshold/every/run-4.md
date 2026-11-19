<!-- start 2026-11-16T17:50:14+01:00, waited 0 min, load 0.76 2.33 1.70
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
| cards: pan steps | 60 | 0.39 | 0.43 (0.58) | 2.51 | 0.57 (0.67) | 2.26 (3.96) | 27 | 46 | 0 (36) | 66 | 48 |
| cards: pan rest frame | 1 | 0.05 | 0.05 (0.05) | 2.57 | 0.48 (0.48) | 2.20 (2.20) | 27 | 46 | 0 (0) | 0 | 36 |
| cards: pan back steps | 60 | 0.35 | 0.43 (0.46) | 2.47 | 0.58 (0.64) | 2.19 (2.38) | 27 | 46 | 0 (36) | 63 | 48 |
| cards: pan back rest frame | 1 | 0.04 | 0.04 (0.04) | 2.41 | 0.47 (0.47) | 2.19 (2.19) | 27 | 43 | 0 (0) | 0 | 36 |
| cards: zoom in steps | 60 | 4.14 | 0.51 (0.52) | 2.50 | 0.40 (0.54) | 2.23 (2.48) | 7 | 10 | 0 (26) | 46 | 36 |
| cards: zoom in rest frame | 1 | 4.54 | 4.54 (4.54) | 1.10 | 0.35 (0.35) | 2.34 (2.34) | 0 | 0 | 0 (0) | 0 | 1 |
| cards: zoom out steps | 60 | 4.21 | 0.50 (0.98) | 2.30 | 0.41 (0.58) | 2.29 (2.43) | 7 | 523 | 0 (27) | 2517 | 36 |
| cards: zoom out rest frame | 1 | 4.06 | 4.06 (4.06) | 2.65 | 0.78 (0.78) | 2.48 (2.48) | 27 | 43 | 27 (27) | 43 | 36 |
| cards: zoom out further steps | 60 | 4.09 | 0.57 (0.75) | 1.77 | 1.03 (1.49) | 2.36 (2.50) | 94 | 156 | 0 (130) | 214 | 192 |
| cards: zoom out further rest frame | 1 | 4.76 | 4.76 (4.76) | 0.99 | 2.35 (2.35) | 3.66 (3.66) | 131 | 5 | 131 (131) | 5 | 192 |
<!-- end 2026-11-16T17:50:24+01:00, load 0.64 2.25 1.68, highest during the run 0.76
%CPU COMMAND
 1.2 jetbrains-toolb
 0.9 claude
 0.5 gnome-terminal-
 0.4 gnome-shell
 0.3 glycin-svg
-->
