<!-- start 2026-11-16T17:11:24+01:00, waited 0 min, load 0.45 0.85 0.84
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
| cards: pan steps | 60 | 0.50 | 0.42 (0.60) | 2.51 | 0.54 (0.62) | 2.31 (2.90) | 27 | 46 | 0 (36) | 66 | 48 |
| cards: pan rest frame | 1 | 0.05 | 0.05 (0.05) | 2.57 | 0.44 (0.44) | 2.41 (2.41) | 27 | 46 | 0 (0) | 0 | 36 |
| cards: pan back steps | 60 | 0.35 | 0.43 (0.45) | 2.46 | 0.54 (0.59) | 2.34 (2.55) | 27 | 46 | 0 (36) | 63 | 48 |
| cards: pan back rest frame | 1 | 0.05 | 0.05 (0.05) | 2.40 | 0.44 (0.44) | 2.25 (2.25) | 27 | 43 | 0 (0) | 0 | 36 |
| cards: zoom in steps | 60 | 4.20 | 0.50 (0.53) | 2.50 | 0.37 (0.50) | 2.33 (2.52) | 7 | 10 | 0 (26) | 46 | 36 |
| cards: zoom in rest frame | 1 | 4.57 | 4.57 (4.57) | 1.12 | 0.33 (0.33) | 2.40 (2.40) | 0 | 0 | 0 (0) | 0 | 1 |
| cards: zoom out steps | 60 | 4.21 | 0.50 (0.98) | 2.29 | 0.39 (0.55) | 2.35 (2.45) | 7 | 523 | 0 (27) | 2517 | 36 |
| cards: zoom out rest frame | 1 | 4.08 | 4.08 (4.08) | 2.65 | 0.74 (0.74) | 2.50 (2.50) | 27 | 43 | 27 (27) | 43 | 36 |
| cards: zoom out further steps | 60 | 4.12 | 0.57 (0.75) | 1.75 | 0.93 (1.36) | 2.25 (2.38) | 94 | 156 | 0 (130) | 214 | 192 |
| cards: zoom out further rest frame | 1 | 4.80 | 4.80 (4.80) | 1.01 | 2.34 (2.34) | 3.46 (3.46) | 131 | 5 | 131 (131) | 5 | 192 |
<!-- end 2026-11-16T17:11:34+01:00, load 0.54 0.85 0.84, highest during the run 0.45
%CPU COMMAND
 1.2 jetbrains-toolb
 0.9 claude
 0.5 gnome-terminal-
 0.4 gnome-shell
 0.4 glycin-svg
-->
