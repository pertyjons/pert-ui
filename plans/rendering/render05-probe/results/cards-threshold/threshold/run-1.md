<!-- start 2026-11-16T17:48:39+01:00, waited 0 min, load 1.63 2.96 1.81
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
| cards: pan steps | 60 | 0.43 | 0.43 (0.59) | 2.51 | 0.54 (0.62) | 2.29 (3.55) | 27 | 46 | 0 (36) | 66 | 48 |
| cards: pan rest frame | 1 | 0.05 | 0.05 (0.05) | 2.57 | 0.46 (0.46) | 2.18 (2.18) | 27 | 46 | 0 (0) | 0 | 36 |
| cards: pan back steps | 60 | 0.35 | 0.43 (0.45) | 2.47 | 0.55 (0.60) | 2.31 (2.44) | 27 | 46 | 0 (36) | 63 | 48 |
| cards: pan back rest frame | 1 | 0.05 | 0.05 (0.05) | 2.40 | 0.31 (0.31) | 2.20 (2.20) | 27 | 43 | 0 (0) | 0 | 36 |
| cards: zoom in steps | 60 | 4.10 | 0.50 (0.53) | 2.49 | 0.38 (0.52) | 2.27 (2.34) | 7 | 10 | 0 (26) | 46 | 36 |
| cards: zoom in rest frame | 1 | 4.46 | 4.46 (4.46) | 1.12 | 0.35 (0.35) | 2.29 (2.29) | 0 | 0 | 0 (0) | 0 | 1 |
| cards: zoom out steps | 60 | 4.16 | 0.50 (0.98) | 2.29 | 0.40 (0.57) | 2.29 (2.42) | 7 | 523 | 0 (27) | 2517 | 36 |
| cards: zoom out rest frame | 1 | 4.10 | 4.10 (4.10) | 2.67 | 0.77 (0.77) | 2.51 (2.51) | 27 | 43 | 27 (27) | 43 | 36 |
| cards: zoom out further steps | 60 | 4.10 | 0.57 (0.76) | 1.77 | 0.92 (1.27) | 2.25 (2.38) | 94 | 156 | 0 (130) | 214 | 192 |
| cards: zoom out further rest frame | 1 | 4.79 | 4.79 (4.79) | 1.06 | 2.24 (2.24) | 3.47 (3.47) | 131 | 5 | 131 (131) | 5 | 192 |
<!-- end 2026-11-16T17:48:48+01:00, load 1.46 2.88 1.80, highest during the run 1.63
%CPU COMMAND
 1.2 jetbrains-toolb
 0.9 claude
 0.5 gnome-terminal-
 0.4 gnome-shell
 0.2 glycin-svg
-->
