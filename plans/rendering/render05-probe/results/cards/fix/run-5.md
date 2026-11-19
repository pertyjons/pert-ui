<!-- start 2026-11-16T17:13:00+01:00, waited 0 min, load 0.77 0.85 0.84
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
| cards: pan steps | 60 | 0.41 | 0.42 (0.59) | 2.51 | 0.54 (0.64) | 2.40 (5.74) | 27 | 46 | 0 (36) | 66 | 48 |
| cards: pan rest frame | 1 | 0.04 | 0.04 (0.04) | 2.58 | 0.45 (0.45) | 2.31 (2.31) | 27 | 46 | 0 (0) | 0 | 36 |
| cards: pan back steps | 60 | 0.36 | 0.43 (0.46) | 2.45 | 0.55 (0.61) | 2.35 (2.54) | 27 | 46 | 0 (36) | 63 | 48 |
| cards: pan back rest frame | 1 | 0.04 | 0.04 (0.04) | 2.38 | 0.45 (0.45) | 2.26 (2.26) | 27 | 43 | 0 (0) | 0 | 36 |
| cards: zoom in steps | 60 | 4.12 | 0.49 (0.53) | 2.50 | 0.38 (0.53) | 2.24 (2.51) | 7 | 10 | 0 (26) | 46 | 36 |
| cards: zoom in rest frame | 1 | 4.56 | 4.56 (4.56) | 1.10 | 0.34 (0.34) | 2.13 (2.13) | 0 | 0 | 0 (0) | 0 | 1 |
| cards: zoom out steps | 60 | 4.25 | 0.50 (1.00) | 2.29 | 0.41 (0.56) | 2.30 (2.57) | 7 | 523 | 0 (27) | 2517 | 36 |
| cards: zoom out rest frame | 1 | 4.06 | 4.06 (4.06) | 2.66 | 0.73 (0.73) | 2.51 (2.51) | 27 | 43 | 27 (27) | 43 | 36 |
| cards: zoom out further steps | 60 | 4.04 | 0.57 (0.78) | 1.77 | 0.91 (1.34) | 2.59 (2.86) | 94 | 156 | 0 (130) | 214 | 192 |
| cards: zoom out further rest frame | 1 | 4.66 | 4.66 (4.66) | 1.06 | 2.26 (2.26) | 3.93 (3.93) | 131 | 5 | 131 (131) | 5 | 192 |
<!-- end 2026-11-16T17:13:09+01:00, load 0.65 0.82 0.83, highest during the run 0.77
%CPU COMMAND
 1.2 jetbrains-toolb
 0.8 claude
 0.5 gnome-terminal-
 0.4 gnome-shell
 0.4 glycin-svg
-->
