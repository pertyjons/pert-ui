## Change scenarios on page `menus`

| Scenario | Frames | Compose median ms (p95) | New masks per frame (median) | New mask KiB per frame (median) | GPU offscreen median ms (p95) |
|---|---|---|---|---|---|
| resize +7 px per frame | 30 | 19.06 (20.08) | 0 | 0 | 7.73 (32.20) |
| scale 1 → 1.25 → 1.5 → 1.75 → 2 | 4 | 21.98 (22.28) | 31 | 416 | 14.19 (18.90) |

Profiler self time by operation (largest first):

- resize +7 px per frame: self ms per frame: Flush 18.11, Text 0.58, Measure 0.10, Arrange 0.08, Compose 0.07, Layout 0.05
- scale 1 → 1.25 → 1.5 → 1.75 → 2: self ms per frame: Flush 18.35, Text 2.74, Measure 0.12, Arrange 0.12, Compose 0.09, Layout 0.08

## Change scenarios on page `live`

| Scenario | Frames | Compose median ms (p95) | New masks per frame (median) | New mask KiB per frame (median) | GPU offscreen median ms (p95) |
|---|---|---|---|---|---|
| resize +7 px per frame | 30 | 97.04 (98.12) | 0 | 0 | 9.09 (34.68) |
| scale 1 → 1.25 → 1.5 → 1.75 → 2 | 4 | 117.40 (127.94) | 101 | 582 | 16.91 (21.50) |

Profiler self time by operation (largest first):

- resize +7 px per frame: self ms per frame: Text 70.05, Flush 22.32, Arrange 1.99, Measure 1.97, Compose 0.21, Layout 0.19
- scale 1 → 1.25 → 1.5 → 1.75 → 2: self ms per frame: Text 86.96, Flush 22.33, Arrange 2.20, Measure 2.02, Compose 0.21, Layout 0.19
