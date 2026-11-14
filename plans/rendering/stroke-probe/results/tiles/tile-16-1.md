<!-- start 2026-11-12T21:33:38+01:00, waited 0 min, load 1.10 1.16 1.10
%CPU COMMAND
 4.0 chrome
 2.9 gnome-terminal-
 2.8 gnome-shell
 2.0 chrome
 1.5 claude
-->
# Stroke prototype: timing

Adapter: Intel(R) UHD Graphics 770 (ADL-S GT1) (Vulkan, Mesa 26.2.6). Tile size 16. 12 warm-up and 300 measured frames per workload. Prepare = preparation, encoding and upload; GPU = encode, submit and wait for one offscreen pass; CPU = preparation and per-pixel evaluation and blending into a linear frame; today's CPU = the CPU renderer's frame with the strokes minus without them. Presentation is not measured.

| Workload | Scale | Segments | References | Tiles | Longest list | Bytes | Prepare | GPU | CPU | Today: rasterize | Today: prepare | Today: GPU | Today: CPU |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| trace, 800 points | 1 | 799 | 2159 | 478 | 14 | 75100 | 0.150 / 0.156 | 1.397 / 2.681 | 2.521 / 2.565 | – | – | – | – |
| trace, 4096 points | 1 | 4095 | 6498 | 493 | 40 | 303880 | 0.524 / 0.539 | 3.121 / 3.543 | 6.714 / 6.848 | – | – | – | – |
| noise, 800 points | 1 | 799 | 8429 | 1163 | 15 | 122100 | 0.272 / 0.292 | 4.133 / 4.648 | 8.842 / 9.209 | – | – | – | – |
| steep alternation, 400 points | 1 | 399 | 11823 | 1326 | 10 | 115292 | 0.304 / 0.360 | 5.411 / 5.801 | 11.527 / 12.529 | – | – | – | – |
| noise, 16384 points | 1 | 16383 | 175888 | 1323 | 229 | 1794432 | 4.534 / 5.338 | 29.578 / 38.502 | 160.058 / 163.191 | – | – | – | – |
| diagram, 60 edges | 1 | 58 | 2035 | 2035 | 1 | 78828 | 0.647 / 0.666 | 1.705 / 3.118 | 3.744 / 3.826 | – | – | – | – |
| knob pointer | 1 | 1 | 2 | 2 | 1 | 168 | 0.008 / 0.009 | 0.377 / 0.403 | 0.003 / 0.004 | – | – | – | – |
| trace, 800 points, view scale 12 | 1 | 11 | 204 | 151 | 3 | 6384 | 0.063 / 0.083 | 0.763 / 1.113 | 0.560 / 0.992 | – | – | – | – |
| trace, 800 points, unchanged | 1 | 799 | 2191 | 476 | 13 | 75164 | 0.000 / 0.000 | 2.717 / 2.859 | 2.401 / 2.451 | – | – | – | – |
| trace, 800 points | 2 | 799 | 3655 | 1291 | 10 | 107100 | 0.226 / 0.235 | 2.157 / 2.404 | 4.305 / 4.394 | – | – | – | – |
| trace, 4096 points | 2 | 4095 | 8885 | 1337 | 20 | 340436 | 0.652 / 0.675 | 3.970 / 4.369 | 9.426 / 9.595 | – | – | – | – |
| noise, 800 points | 2 | 799 | 17313 | 4154 | 10 | 253348 | 0.561 / 0.628 | 7.299 / 8.466 | 17.898 / 18.698 | – | – | – | – |
| steep alternation, 400 points | 2 | 399 | 24926 | 5123 | 6 | 289208 | 0.718 / 0.771 | 7.722 / 8.730 | 23.322 / 24.467 | – | – | – | – |
| noise, 16384 points | 2 | 16383 | 358279 | 5123 | 128 | 2645596 | 8.309 / 9.867 | 55.052 / 64.489 | 321.420 / 328.953 | – | – | – | – |
| diagram, 60 edges | 2 | 58 | 4323 | 4323 | 1 | 161196 | 2.424 / 2.480 | 3.234 / 3.391 | 9.180 / 9.804 | – | – | – | – |
| knob pointer | 2 | 1 | 3 | 3 | 1 | 204 | 0.013 / 0.020 | 0.407 / 0.500 | 0.010 / 0.012 | – | – | – | – |
| trace, 800 points, view scale 12 | 2 | 11 | 576 | 447 | 3 | 17344 | 0.144 / 0.201 | 1.717 / 2.753 | 1.743 / 3.145 | – | – | – | – |
| trace, 800 points, unchanged | 2 | 799 | 3696 | 1289 | 9 | 107200 | 0.000 / 0.000 | 2.290 / 2.353 | 4.094 / 4.185 | – | – | – | – |
<!-- end 2026-11-12T21:37:38+01:00, load 1.15 1.11 1.09, highest during the run 1.33
%CPU COMMAND
 4.0 chrome
 2.9 gnome-terminal-
 2.7 gnome-shell
 2.0 chrome
 1.4 claude
-->
