<!-- start 2026-11-12T21:51:26+01:00, waited 0 min, load 0.83 0.91 0.98
%CPU COMMAND
 4.1 chrome
 2.6 gnome-terminal-
 2.5 gnome-shell
 2.0 chrome
 1.4 chrome
-->
# Stroke prototype: timing

Adapter: Intel(R) UHD Graphics 770 (ADL-S GT1) (Vulkan, Mesa 26.2.6). Tile size 32. 12 warm-up and 300 measured frames per workload. Prepare = preparation, encoding and upload; GPU = encode, submit and wait for one offscreen pass; CPU = preparation and per-pixel evaluation and blending into a linear frame; today's CPU = the CPU renderer's frame with the strokes minus without them. Presentation is not measured.

| Workload | Scale | Segments | References | Tiles | Longest list | Bytes | Prepare | GPU | CPU | Today: rasterize | Today: prepare | Today: GPU | Today: CPU |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| trace, 800 points | 1 | 799 | 1434 | 174 | 20 | 62472 | 0.111 / 0.114 | 2.365 / 2.489 | 5.404 / 5.482 | – | – | – | – |
| trace, 4096 points | 1 | 4095 | 5215 | 178 | 70 | 288668 | 0.432 / 0.449 | 6.407 / 7.096 | 17.732 / 17.986 | – | – | – | – |
| noise, 800 points | 1 | 799 | 4398 | 323 | 26 | 79096 | 0.157 / 0.168 | 5.676 / 6.263 | 16.776 / 17.325 | – | – | – | – |
| steep alternation, 400 points | 1 | 399 | 5561 | 338 | 18 | 58628 | 0.129 / 0.148 | 6.695 / 7.408 | 19.111 / 19.540 | – | – | – | – |
| noise, 16384 points | 1 | 16383 | 91379 | 338 | 441 | 1424876 | 2.695 / 3.061 | 42.526 / 44.464 | 306.664 / 311.221 | – | – | – | – |
| diagram, 60 edges | 1 | 58 | 985 | 985 | 1 | 41028 | 0.185 / 0.191 | 2.263 / 2.359 | 5.336 / 5.399 | – | – | – | – |
| knob pointer | 1 | 1 | 1 | 1 | 1 | 132 | 0.008 / 0.009 | 0.374 / 0.410 | 0.003 / 0.004 | – | – | – | – |
| trace, 800 points, view scale 12 | 1 | 11 | 75 | 45 | 3 | 2476 | 0.051 / 0.059 | 0.982 / 1.404 | 0.753 / 1.376 | – | – | – | – |
| trace, 800 points, unchanged | 1 | 799 | 1459 | 172 | 18 | 62508 | 0.000 / 0.000 | 2.339 / 2.429 | 5.336 / 5.393 | – | – | – | – |
| trace, 800 points | 2 | 799 | 2119 | 475 | 14 | 74844 | 0.143 / 0.161 | 3.455 / 3.684 | 8.342 / 8.702 | – | – | – | – |
| trace, 4096 points | 2 | 4095 | 6298 | 484 | 41 | 302792 | 0.490 / 0.534 | 7.404 / 8.488 | 21.374 / 21.945 | – | – | – | – |
| noise, 800 points | 2 | 799 | 8275 | 1161 | 14 | 121420 | 0.261 / 0.283 | 7.709 / 8.638 | 32.896 / 34.149 | – | – | – | – |
| steep alternation, 400 points | 2 | 399 | 11562 | 1312 | 10 | 113800 | 0.274 / 0.302 | 7.864 / 8.569 | 45.131 / 46.084 | – | – | – | – |
| noise, 16384 points | 2 | 16383 | 171766 | 1321 | 227 | 1777880 | 3.991 / 4.750 | 83.818 / 93.289 | 597.988 / 607.989 | – | – | – | – |
| diagram, 60 edges | 2 | 58 | 2016 | 2016 | 1 | 78144 | 0.646 / 0.663 | 4.257 / 4.539 | 11.850 / 12.098 | – | – | – | – |
| knob pointer | 2 | 1 | 2 | 2 | 1 | 168 | 0.015 / 0.041 | 0.446 / 0.609 | 0.010 / 0.018 | – | – | – | – |
| trace, 800 points, view scale 12 | 2 | 11 | 203 | 150 | 3 | 6348 | 0.077 / 0.095 | 1.900 / 2.979 | 2.044 / 3.688 | – | – | – | – |
| trace, 800 points, unchanged | 2 | 799 | 2153 | 472 | 13 | 74884 | 0.000 / 0.000 | 3.693 / 3.845 | 8.298 / 8.411 | – | – | – | – |
<!-- end 2026-11-12T21:58:21+01:00, load 0.97 0.99 1.00, highest during the run 1.22
%CPU COMMAND
 4.1 chrome
 2.5 gnome-terminal-
 2.4 gnome-shell
 1.9 chrome
 1.3 chrome
-->
