<!-- start 2026-11-12T21:37:38+01:00, waited 0 min, load 1.15 1.11 1.09
%CPU COMMAND
 100 ps
 4.0 chrome
 2.9 gnome-terminal-
 2.7 gnome-shell
 2.0 chrome
-->
# Stroke prototype: timing

Adapter: Intel(R) UHD Graphics 770 (ADL-S GT1) (Vulkan, Mesa 26.2.6). Tile size 32. 12 warm-up and 300 measured frames per workload. Prepare = preparation, encoding and upload; GPU = encode, submit and wait for one offscreen pass; CPU = preparation and per-pixel evaluation and blending into a linear frame; today's CPU = the CPU renderer's frame with the strokes minus without them. Presentation is not measured.

| Workload | Scale | Segments | References | Tiles | Longest list | Bytes | Prepare | GPU | CPU | Today: rasterize | Today: prepare | Today: GPU | Today: CPU |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| trace, 800 points | 1 | 799 | 1434 | 174 | 20 | 62472 | 0.111 / 0.116 | 2.373 / 2.477 | 5.407 / 5.488 | – | – | – | – |
| trace, 4096 points | 1 | 4095 | 5215 | 178 | 70 | 288668 | 0.435 / 0.475 | 6.403 / 7.073 | 17.735 / 17.981 | – | – | – | – |
| noise, 800 points | 1 | 799 | 4398 | 323 | 26 | 79096 | 0.159 / 0.193 | 5.668 / 6.099 | 16.878 / 17.876 | – | – | – | – |
| steep alternation, 400 points | 1 | 399 | 5561 | 338 | 18 | 58628 | 0.135 / 0.147 | 6.826 / 10.048 | 19.172 / 19.510 | – | – | – | – |
| noise, 16384 points | 1 | 16383 | 91379 | 338 | 441 | 1424876 | 2.728 / 3.097 | 42.711 / 51.885 | 307.060 / 313.007 | – | – | – | – |
| diagram, 60 edges | 1 | 58 | 985 | 985 | 1 | 41028 | 0.185 / 0.193 | 2.260 / 2.376 | 5.338 / 5.425 | – | – | – | – |
| knob pointer | 1 | 1 | 1 | 1 | 1 | 132 | 0.008 / 0.008 | 0.377 / 0.406 | 0.003 / 0.004 | – | – | – | – |
| trace, 800 points, view scale 12 | 1 | 11 | 75 | 45 | 3 | 2476 | 0.049 / 0.057 | 0.956 / 1.338 | 0.748 / 1.366 | – | – | – | – |
| trace, 800 points, unchanged | 1 | 799 | 1459 | 172 | 18 | 62508 | 0.000 / 0.000 | 2.352 / 2.464 | 5.345 / 5.450 | – | – | – | – |
| trace, 800 points | 2 | 799 | 2119 | 475 | 14 | 74844 | 0.143 / 0.152 | 3.457 / 3.708 | 8.354 / 8.528 | – | – | – | – |
| trace, 4096 points | 2 | 4095 | 6298 | 484 | 41 | 302792 | 0.489 / 0.518 | 7.375 / 8.491 | 21.338 / 21.861 | – | – | – | – |
| noise, 800 points | 2 | 799 | 8275 | 1161 | 14 | 121420 | 0.260 / 0.290 | 7.490 / 8.547 | 32.907 / 34.533 | – | – | – | – |
| steep alternation, 400 points | 2 | 399 | 11562 | 1312 | 10 | 113800 | 0.272 / 0.291 | 7.590 / 8.231 | 45.086 / 45.725 | – | – | – | – |
| noise, 16384 points | 2 | 16383 | 171766 | 1321 | 227 | 1777880 | 3.969 / 4.772 | 83.549 / 85.673 | 597.190 / 607.191 | – | – | – | – |
| diagram, 60 edges | 2 | 58 | 2016 | 2016 | 1 | 78144 | 0.652 / 0.705 | 4.172 / 4.453 | 11.969 / 13.215 | – | – | – | – |
| knob pointer | 2 | 1 | 2 | 2 | 1 | 168 | 0.016 / 0.028 | 0.467 / 0.596 | 0.010 / 0.013 | – | – | – | – |
| trace, 800 points, view scale 12 | 2 | 11 | 203 | 150 | 3 | 6348 | 0.080 / 0.098 | 1.874 / 2.939 | 2.066 / 3.730 | – | – | – | – |
| trace, 800 points, unchanged | 2 | 799 | 2153 | 472 | 13 | 74884 | 0.000 / 0.000 | 3.520 / 3.735 | 8.367 / 8.562 | – | – | – | – |
<!-- end 2026-11-12T21:44:34+01:00, load 0.89 0.98 1.03, highest during the run 1.15
%CPU COMMAND
 4.0 chrome
 2.7 gnome-terminal-
 2.6 gnome-shell
 2.0 chrome
 1.4 claude
-->
