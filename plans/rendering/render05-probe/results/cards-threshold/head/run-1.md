<!-- start 2026-11-16T17:48:29+01:00, waited 2 min, load 1.67 3.01 1.81
%CPU COMMAND
 1.2 jetbrains-toolb
 0.9 claude
 0.5 gnome-terminal-
 0.4 gnome-shell
 0.2 kworker/u97:9-btrfs-endio
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
| cards: pan steps | 60 | 0.39 | 0.43 (0.57) | 2.50 | 0.54 (0.60) | 2.33 (3.09) | 27 | 46 | 0 (36) | 66 | 48 |
| cards: pan rest frame | 1 | 0.05 | 0.05 (0.05) | 2.60 | 0.44 (0.44) | 2.26 (2.26) | 27 | 46 | 0 (0) | 0 | 36 |
| cards: pan back steps | 60 | 0.35 | 0.43 (0.46) | 2.47 | 0.55 (0.61) | 2.36 (2.46) | 27 | 46 | 0 (36) | 63 | 48 |
| cards: pan back rest frame | 1 | 0.05 | 0.05 (0.05) | 2.40 | 0.46 (0.46) | 2.29 (2.29) | 27 | 43 | 0 (0) | 0 | 36 |
| cards: zoom in steps | 60 | 4.20 | 0.49 (0.55) | 2.50 | 0.37 (0.53) | 2.31 (2.49) | 7 | 10 | 0 (26) | 46 | 36 |
| cards: zoom in rest frame | 1 | 4.50 | 4.50 (4.50) | 1.11 | 0.34 (0.34) | 2.53 (2.53) | 0 | 0 | 0 (0) | 0 | 1 |
| cards: zoom out steps | 60 | 4.21 | 0.51 (0.99) | 2.30 | 0.40 (0.55) | 2.36 (2.49) | 7 | 523 | 0 (27) | 2517 | 36 |
| cards: zoom out rest frame | 1 | 4.18 | 4.18 (4.18) | 2.66 | 0.77 (0.77) | 2.47 (2.47) | 27 | 43 | 27 (27) | 43 | 36 |
| cards: zoom out further steps | 60 | 4.12 | 0.56 (0.76) | 1.78 | 0.92 (1.39) | 2.25 (2.39) | 94 | 156 | 0 (130) | 214 | 192 |
| cards: zoom out further rest frame | 1 | 4.87 | 4.87 (4.87) | 1.01 | 2.46 (2.46) | 3.48 (3.48) | 131 | 5 | 131 (131) | 5 | 192 |
<!-- end 2026-11-16T17:48:39+01:00, load 1.63 2.96 1.81, highest during the run 1.67
%CPU COMMAND
 1.2 jetbrains-toolb
 0.9 claude
 0.5 gnome-terminal-
 0.4 gnome-shell
 0.2 kworker/u97:9-btrfs-endio
-->
