<!-- start 2026-11-12T21:30:43+01:00, waited 0 min, load 0.64 1.22 1.12
%CPU COMMAND
66.6 bash
 3.9 chrome
 3.0 gnome-terminal-
 2.8 gnome-shell
 2.0 chrome
-->
# Stroke prototype: timing

Adapter: Intel(R) UHD Graphics 770 (ADL-S GT1) (Vulkan, Mesa 26.2.6). Tile size 8. 12 warm-up and 300 measured frames per workload. Prepare = preparation, encoding and upload; GPU = encode, submit and wait for one offscreen pass; CPU = preparation and per-pixel evaluation and blending into a linear frame; today's CPU = the CPU renderer's frame with the strokes minus without them. Presentation is not measured.

| Workload | Scale | Segments | References | Tiles | Longest list | Bytes | Prepare | GPU | CPU | Today: rasterize | Today: prepare | Today: GPU | Today: CPU |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| trace, 800 points | 1 | 799 | 3801 | 1318 | 9 | 108548 | 0.259 / 0.414 | 1.821 / 2.141 | 1.484 / 2.489 | – | – | – | – |
| trace, 4096 points | 1 | 4095 | 9324 | 1376 | 21 | 343440 | 0.753 / 0.799 | 1.847 / 2.132 | 3.608 / 3.744 | – | – | – | – |
| noise, 800 points | 1 | 799 | 17787 | 4141 | 10 | 254828 | 0.643 / 0.720 | 3.287 / 3.549 | 5.351 / 5.601 | – | – | – | – |
| steep alternation, 400 points | 1 | 399 | 25947 | 5151 | 6 | 294188 | 0.930 / 1.022 | 4.836 / 5.256 | 6.850 / 7.350 | – | – | – | – |
| noise, 16384 points | 1 | 16383 | 371745 | 5134 | 129 | 2699812 | 10.156 / 12.145 | 22.773 / 29.461 | 93.129 / 94.493 | – | – | – | – |
| diagram, 60 edges | 1 | 58 | 4417 | 4417 | 1 | 164580 | 2.447 / 2.481 | 1.650 / 1.751 | 4.402 / 4.469 | – | – | – | – |
| knob pointer | 1 | 1 | 3 | 3 | 1 | 204 | 0.008 / 0.009 | 0.378 / 0.416 | 0.004 / 0.004 | – | – | – | – |
| trace, 800 points, view scale 12 | 1 | 11 | 581 | 451 | 3 | 17492 | 0.132 / 0.199 | 0.814 / 1.167 | 0.522 / 0.921 | – | – | – | – |
| trace, 800 points, unchanged | 1 | 799 | 3829 | 1314 | 10 | 108532 | 0.000 / 0.000 | 1.806 / 1.832 | 1.239 / 1.256 | – | – | – | – |
| trace, 800 points | 2 | 799 | 7496 | 3476 | 6 | 192384 | 0.467 / 0.487 | 1.657 / 2.804 | 2.800 / 2.870 | – | – | – | – |
| trace, 4096 points | 2 | 4095 | 14856 | 3646 | 12 | 438208 | 1.073 / 1.111 | 2.730 / 3.076 | 5.526 / 5.724 | – | – | – | – |
| noise, 800 points | 2 | 799 | 39808 | 14335 | 7 | 669120 | 1.586 / 1.777 | 6.287 / 6.832 | 12.251 / 12.803 | – | – | – | – |
| steep alternation, 400 points | 2 | 399 | 58653 | 20246 | 4 | 908052 | 2.422 / 2.610 | 6.763 / 8.073 | 16.851 / 17.781 | – | – | – | – |
| noise, 16384 points | 2 | 16383 | 823566 | 20043 | 80 | 4984184 | 22.574 / 28.015 | 40.245 / 49.066 | 203.891 / 209.439 | – | – | – | – |
| diagram, 60 edges | 2 | 58 | 9834 | 9834 | 1 | 359592 | 9.388 / 9.506 | 2.840 / 3.017 | 13.705 / 14.005 | – | – | – | – |
| knob pointer | 2 | 1 | 10 | 10 | 1 | 456 | 0.014 / 0.022 | 0.417 / 0.510 | 0.009 / 0.012 | – | – | – | – |
| trace, 800 points, view scale 12 | 2 | 11 | 1883 | 1524 | 3 | 57036 | 0.389 / 0.612 | 1.961 / 3.118 | 1.800 / 3.190 | – | – | – | – |
| trace, 800 points, unchanged | 2 | 799 | 7510 | 3487 | 6 | 192792 | 0.000 / 0.000 | 1.622 / 3.035 | 2.364 / 2.428 | – | – | – | – |
<!-- end 2026-11-12T21:33:38+01:00, load 1.10 1.16 1.10, highest during the run 1.13
%CPU COMMAND
 4.0 chrome
 2.9 gnome-terminal-
 2.8 gnome-shell
 2.0 chrome
 1.5 claude
-->
