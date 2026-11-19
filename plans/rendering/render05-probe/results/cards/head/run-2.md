<!-- start 2026-11-16T17:10:46+01:00, waited 0 min, load 0.60 0.92 0.86
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
| cards: pan steps | 60 | 0.40 | 0.43 (0.62) | 2.52 | 0.54 (0.61) | 2.31 (2.62) | 27 | 46 | 0 (36) | 66 | 48 |
| cards: pan rest frame | 1 | 0.05 | 0.05 (0.05) | 2.56 | 0.47 (0.47) | 2.45 (2.45) | 27 | 46 | 0 (0) | 0 | 36 |
| cards: pan back steps | 60 | 0.35 | 0.43 (0.46) | 2.47 | 0.54 (0.59) | 2.35 (2.52) | 27 | 46 | 0 (36) | 63 | 48 |
| cards: pan back rest frame | 1 | 0.04 | 0.04 (0.04) | 2.39 | 0.49 (0.49) | 2.33 (2.33) | 27 | 43 | 0 (0) | 0 | 36 |
| cards: zoom in steps | 60 | 4.12 | 0.50 (0.53) | 2.50 | 0.37 (0.52) | 2.35 (2.55) | 7 | 10 | 0 (26) | 46 | 36 |
| cards: zoom in rest frame | 1 | 4.56 | 4.56 (4.56) | 1.16 | 0.35 (0.35) | 2.46 (2.46) | 0 | 0 | 0 (0) | 0 | 1 |
| cards: zoom out steps | 60 | 4.22 | 0.50 (0.97) | 2.30 | 0.39 (0.53) | 2.38 (2.56) | 7 | 523 | 0 (27) | 2517 | 36 |
| cards: zoom out rest frame | 1 | 4.05 | 4.05 (4.05) | 2.67 | 0.74 (0.74) | 2.45 (2.45) | 27 | 43 | 27 (27) | 43 | 36 |
| cards: zoom out further steps | 60 | 4.10 | 0.56 (0.72) | 1.77 | 0.95 (1.38) | 2.32 (2.48) | 94 | 156 | 0 (130) | 214 | 192 |
| cards: zoom out further rest frame | 1 | 4.82 | 4.82 (4.82) | 1.02 | 2.37 (2.37) | 3.58 (3.58) | 131 | 5 | 131 (131) | 5 | 192 |
<!-- end 2026-11-16T17:10:56+01:00, load 0.55 0.90 0.86, highest during the run 0.60
%CPU COMMAND
 1.2 jetbrains-toolb
 0.8 claude
 0.5 gnome-terminal-
 0.4 gnome-shell
 0.4 glycin-svg
-->
