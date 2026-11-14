<!-- start 2026-11-12T21:58:21+01:00, waited 0 min, load 0.97 0.99 1.00
%CPU COMMAND
 4.1 chrome
 2.5 gnome-terminal-
 2.4 gnome-shell
 1.9 chrome
 1.3 chrome
-->
# Stroke prototype: timing

Adapter: Intel(R) UHD Graphics 770 (ADL-S GT1) (Vulkan, Mesa 26.2.6). Tile size 8. 12 warm-up and 300 measured frames per workload. Prepare = preparation, encoding and upload; GPU = encode, submit and wait for one offscreen pass; CPU = preparation and per-pixel evaluation and blending into a linear frame; today's CPU = the CPU renderer's frame with the strokes minus without them. Presentation is not measured.

| Workload | Scale | Segments | References | Tiles | Longest list | Bytes | Prepare | GPU | CPU | Today: rasterize | Today: prepare | Today: GPU | Today: CPU |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| trace, 800 points | 1 | 799 | 3801 | 1318 | 9 | 108548 | 0.256 / 0.275 | 1.862 / 2.067 | 1.460 / 1.540 | – | – | – | – |
| trace, 4096 points | 1 | 4095 | 9324 | 1376 | 21 | 343440 | 0.755 / 0.833 | 1.809 / 2.027 | 3.600 / 3.877 | – | – | – | – |
| noise, 800 points | 1 | 799 | 17787 | 4141 | 10 | 254828 | 0.636 / 0.706 | 3.419 / 3.669 | 5.325 / 5.574 | – | – | – | – |
| steep alternation, 400 points | 1 | 399 | 25947 | 5151 | 6 | 294188 | 0.908 / 0.977 | 4.884 / 5.241 | 6.788 / 7.183 | – | – | – | – |
| noise, 16384 points | 1 | 16383 | 371745 | 5134 | 129 | 2699812 | 10.131 / 11.661 | 22.745 / 22.998 | 92.805 / 94.075 | – | – | – | – |
| diagram, 60 edges | 1 | 58 | 4417 | 4417 | 1 | 164580 | 2.445 / 2.475 | 1.662 / 1.755 | 4.393 / 4.450 | – | – | – | – |
| knob pointer | 1 | 1 | 3 | 3 | 1 | 204 | 0.008 / 0.009 | 0.376 / 0.419 | 0.004 / 0.004 | – | – | – | – |
| trace, 800 points, view scale 12 | 1 | 11 | 581 | 451 | 3 | 17492 | 0.133 / 0.203 | 0.813 / 1.177 | 0.526 / 0.929 | – | – | – | – |
| trace, 800 points, unchanged | 1 | 799 | 3829 | 1314 | 10 | 108532 | 0.000 / 0.000 | 1.790 / 1.838 | 1.238 / 1.279 | – | – | – | – |
| trace, 800 points | 2 | 799 | 7496 | 3476 | 6 | 192384 | 0.468 / 0.489 | 1.686 / 3.205 | 2.802 / 2.883 | – | – | – | – |
| trace, 4096 points | 2 | 4095 | 14856 | 3646 | 12 | 438208 | 1.077 / 1.104 | 2.765 / 3.064 | 5.529 / 5.669 | – | – | – | – |
| noise, 800 points | 2 | 799 | 39808 | 14335 | 7 | 669120 | 1.587 / 1.824 | 6.217 / 6.864 | 12.297 / 13.010 | – | – | – | – |
| steep alternation, 400 points | 2 | 399 | 58653 | 20246 | 4 | 908052 | 2.346 / 2.396 | 6.756 / 7.479 | 16.264 / 16.553 | – | – | – | – |
| noise, 16384 points | 2 | 16383 | 823566 | 20043 | 80 | 4984184 | 22.436 / 28.152 | 45.979 / 49.828 | 203.295 / 207.969 | – | – | – | – |
| diagram, 60 edges | 2 | 58 | 9834 | 9834 | 1 | 359592 | 9.415 / 9.551 | 2.782 / 2.977 | 13.799 / 14.070 | – | – | – | – |
| knob pointer | 2 | 1 | 10 | 10 | 1 | 456 | 0.017 / 0.028 | 0.471 / 0.571 | 0.011 / 0.014 | – | – | – | – |
| trace, 800 points, view scale 12 | 2 | 11 | 1883 | 1524 | 3 | 57036 | 0.394 / 0.613 | 1.534 / 3.057 | 1.807 / 3.183 | – | – | – | – |
| trace, 800 points, unchanged | 2 | 799 | 7510 | 3487 | 6 | 192792 | 0.000 / 0.000 | 1.601 / 2.965 | 2.394 / 2.577 | – | – | – | – |
<!-- end 2026-11-12T22:01:16+01:00, load 1.23 1.01 1.00, highest during the run 1.32
%CPU COMMAND
 4.2 chrome
 2.5 gnome-terminal-
 2.4 gnome-shell
 2.0 chrome
 1.9 chrome
-->
