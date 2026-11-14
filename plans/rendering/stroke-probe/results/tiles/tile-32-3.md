<!-- start 2026-11-12T22:05:13+01:00, waited 0 min, load 0.99 1.00 1.00
%CPU COMMAND
 4.2 chrome
 2.4 gnome-terminal-
 2.3 gnome-shell
 1.9 chrome
 1.3 chrome
-->
# Stroke prototype: timing

Adapter: Intel(R) UHD Graphics 770 (ADL-S GT1) (Vulkan, Mesa 26.2.6). Tile size 32. 12 warm-up and 300 measured frames per workload. Prepare = preparation, encoding and upload; GPU = encode, submit and wait for one offscreen pass; CPU = preparation and per-pixel evaluation and blending into a linear frame; today's CPU = the CPU renderer's frame with the strokes minus without them. Presentation is not measured.

| Workload | Scale | Segments | References | Tiles | Longest list | Bytes | Prepare | GPU | CPU | Today: rasterize | Today: prepare | Today: GPU | Today: CPU |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| trace, 800 points | 1 | 799 | 1434 | 174 | 20 | 62472 | 0.111 / 0.122 | 2.353 / 2.476 | 5.422 / 5.576 | – | – | – | – |
| trace, 4096 points | 1 | 4095 | 5215 | 178 | 70 | 288668 | 0.432 / 0.473 | 6.400 / 7.058 | 17.744 / 18.261 | – | – | – | – |
| noise, 800 points | 1 | 799 | 4398 | 323 | 26 | 79096 | 0.163 / 0.190 | 5.609 / 6.212 | 17.021 / 17.825 | – | – | – | – |
| steep alternation, 400 points | 1 | 399 | 5561 | 338 | 18 | 58628 | 0.134 / 0.150 | 6.836 / 10.062 | 19.146 / 19.576 | – | – | – | – |
| noise, 16384 points | 1 | 16383 | 91379 | 338 | 441 | 1424876 | 2.735 / 3.104 | 47.030 / 51.830 | 307.486 / 314.135 | – | – | – | – |
| diagram, 60 edges | 1 | 58 | 985 | 985 | 1 | 41028 | 0.185 / 0.195 | 2.146 / 2.310 | 5.347 / 5.467 | – | – | – | – |
| knob pointer | 1 | 1 | 1 | 1 | 1 | 132 | 0.008 / 0.008 | 0.376 / 0.410 | 0.003 / 0.004 | – | – | – | – |
| trace, 800 points, view scale 12 | 1 | 11 | 75 | 45 | 3 | 2476 | 0.044 / 0.052 | 0.876 / 1.288 | 0.745 / 1.339 | – | – | – | – |
| trace, 800 points, unchanged | 1 | 799 | 1459 | 172 | 18 | 62508 | 0.000 / 0.000 | 2.318 / 2.418 | 5.336 / 5.379 | – | – | – | – |
| trace, 800 points | 2 | 799 | 2119 | 475 | 14 | 74844 | 0.142 / 0.151 | 3.466 / 3.693 | 8.322 / 8.467 | – | – | – | – |
| trace, 4096 points | 2 | 4095 | 6298 | 484 | 41 | 302792 | 0.489 / 0.517 | 7.229 / 8.188 | 21.335 / 21.757 | – | – | – | – |
| noise, 800 points | 2 | 799 | 8275 | 1161 | 14 | 121420 | 0.264 / 0.293 | 9.155 / 14.771 | 33.144 / 34.968 | – | – | – | – |
| steep alternation, 400 points | 2 | 399 | 11562 | 1312 | 10 | 113800 | 0.276 / 0.299 | 11.492 / 16.333 | 45.232 / 46.044 | – | – | – | – |
| noise, 16384 points | 2 | 16383 | 171766 | 1321 | 227 | 1777880 | 3.975 / 4.683 | 83.788 / 92.800 | 597.421 / 609.499 | – | – | – | – |
| diagram, 60 edges | 2 | 58 | 2016 | 2016 | 1 | 78144 | 0.645 / 0.664 | 4.254 / 4.536 | 11.867 / 12.120 | – | – | – | – |
| knob pointer | 2 | 1 | 2 | 2 | 1 | 168 | 0.013 / 0.021 | 0.439 / 0.519 | 0.010 / 0.012 | – | – | – | – |
| trace, 800 points, view scale 12 | 2 | 11 | 203 | 150 | 3 | 6348 | 0.076 / 0.095 | 1.881 / 2.978 | 2.031 / 3.703 | – | – | – | – |
| trace, 800 points, unchanged | 2 | 799 | 2153 | 472 | 13 | 74884 | 0.000 / 0.000 | 3.693 / 3.833 | 8.292 / 8.451 | – | – | – | – |
<!-- end 2026-11-12T22:12:11+01:00, load 0.96 0.97 1.00, highest during the run 1.16
%CPU COMMAND
 4.3 chrome
 2.3 gnome-terminal-
 2.3 gnome-shell
 1.9 chrome
 1.3 chrome
-->
