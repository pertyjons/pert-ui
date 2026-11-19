<!-- start 2026-11-16T17:12:50+01:00, waited 0 min, load 0.91 0.88 0.85
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
| cards: pan steps | 60 | 0.40 | 0.42 (0.55) | 2.51 | 0.54 (0.65) | 2.34 (3.56) | 27 | 46 | 0 (36) | 66 | 48 |
| cards: pan rest frame | 1 | 0.05 | 0.05 (0.05) | 2.56 | 0.47 (0.47) | 2.25 (2.25) | 27 | 46 | 0 (0) | 0 | 36 |
| cards: pan back steps | 60 | 0.34 | 0.43 (0.44) | 2.45 | 0.55 (0.60) | 2.32 (2.42) | 27 | 46 | 0 (36) | 63 | 48 |
| cards: pan back rest frame | 1 | 0.05 | 0.05 (0.05) | 2.39 | 0.46 (0.46) | 2.28 (2.28) | 27 | 43 | 0 (0) | 0 | 36 |
| cards: zoom in steps | 60 | 4.09 | 0.50 (0.51) | 2.49 | 0.38 (0.53) | 2.26 (2.40) | 7 | 10 | 0 (26) | 46 | 36 |
| cards: zoom in rest frame | 1 | 4.46 | 4.46 (4.46) | 1.10 | 0.33 (0.33) | 2.27 (2.27) | 0 | 0 | 0 (0) | 0 | 1 |
| cards: zoom out steps | 60 | 4.18 | 0.50 (0.97) | 2.32 | 0.40 (0.56) | 2.29 (2.49) | 7 | 523 | 0 (27) | 2517 | 36 |
| cards: zoom out rest frame | 1 | 4.09 | 4.09 (4.09) | 2.69 | 0.73 (0.73) | 2.51 (2.51) | 27 | 43 | 27 (27) | 43 | 36 |
| cards: zoom out further steps | 60 | 4.15 | 0.57 (0.76) | 1.76 | 0.91 (1.33) | 2.61 (2.82) | 94 | 156 | 0 (130) | 214 | 192 |
| cards: zoom out further rest frame | 1 | 4.68 | 4.68 (4.68) | 1.01 | 2.35 (2.35) | 3.95 (3.95) | 131 | 5 | 131 (131) | 5 | 192 |
<!-- end 2026-11-16T17:12:59+01:00, load 0.77 0.85 0.84, highest during the run 0.91
%CPU COMMAND
 1.2 jetbrains-toolb
 0.8 claude
 0.5 gnome-terminal-
 0.4 gnome-shell
 0.4 glycin-svg
-->
