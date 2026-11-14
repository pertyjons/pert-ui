<!-- start 2026-11-12T21:44:34+01:00, waited 0 min, load 0.89 0.98 1.03
%CPU COMMAND
 4.0 chrome
 2.7 gnome-terminal-
 2.6 gnome-shell
 2.0 chrome
 1.4 claude
-->
# Stroke prototype: timing

Adapter: Intel(R) UHD Graphics 770 (ADL-S GT1) (Vulkan, Mesa 26.2.6). Tile size 8. 12 warm-up and 300 measured frames per workload. Prepare = preparation, encoding and upload; GPU = encode, submit and wait for one offscreen pass; CPU = preparation and per-pixel evaluation and blending into a linear frame; today's CPU = the CPU renderer's frame with the strokes minus without them. Presentation is not measured.

| Workload | Scale | Segments | References | Tiles | Longest list | Bytes | Prepare | GPU | CPU | Today: rasterize | Today: prepare | Today: GPU | Today: CPU |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| trace, 800 points | 1 | 799 | 3801 | 1318 | 9 | 108548 | 0.254 / 0.268 | 1.866 / 2.079 | 1.461 / 1.493 | – | – | – | – |
| trace, 4096 points | 1 | 4095 | 9324 | 1376 | 21 | 343440 | 0.752 / 0.771 | 1.922 / 3.653 | 3.599 / 3.653 | – | – | – | – |
| noise, 800 points | 1 | 799 | 17787 | 4141 | 10 | 254828 | 0.639 / 0.714 | 3.354 / 3.607 | 5.343 / 5.567 | – | – | – | – |
| steep alternation, 400 points | 1 | 399 | 25947 | 5151 | 6 | 294188 | 0.914 / 0.960 | 4.820 / 5.256 | 6.842 / 7.102 | – | – | – | – |
| noise, 16384 points | 1 | 16383 | 371745 | 5134 | 129 | 2699812 | 10.161 / 11.685 | 22.763 / 29.227 | 93.043 / 95.125 | – | – | – | – |
| diagram, 60 edges | 1 | 58 | 4417 | 4417 | 1 | 164580 | 2.446 / 2.473 | 1.669 / 1.749 | 4.396 / 4.470 | – | – | – | – |
| knob pointer | 1 | 1 | 3 | 3 | 1 | 204 | 0.008 / 0.009 | 0.374 / 0.404 | 0.004 / 0.004 | – | – | – | – |
| trace, 800 points, view scale 12 | 1 | 11 | 581 | 451 | 3 | 17492 | 0.133 / 0.200 | 0.806 / 1.170 | 0.522 / 0.919 | – | – | – | – |
| trace, 800 points, unchanged | 1 | 799 | 3829 | 1314 | 10 | 108532 | 0.000 / 0.000 | 1.748 / 1.833 | 1.235 / 1.261 | – | – | – | – |
| trace, 800 points | 2 | 799 | 7496 | 3476 | 6 | 192384 | 0.467 / 0.484 | 1.719 / 2.864 | 2.794 / 2.861 | – | – | – | – |
| trace, 4096 points | 2 | 4095 | 14856 | 3646 | 12 | 438208 | 1.076 / 1.105 | 2.748 / 3.097 | 5.519 / 5.637 | – | – | – | – |
| noise, 800 points | 2 | 799 | 39808 | 14335 | 7 | 669120 | 1.587 / 1.776 | 6.242 / 6.939 | 12.265 / 12.759 | – | – | – | – |
| steep alternation, 400 points | 2 | 399 | 58653 | 20246 | 4 | 908052 | 2.360 / 2.422 | 6.761 / 7.475 | 16.294 / 16.669 | – | – | – | – |
| noise, 16384 points | 2 | 16383 | 823566 | 20043 | 80 | 4984184 | 22.299 / 27.836 | 39.858 / 40.441 | 203.082 / 205.903 | – | – | – | – |
| diagram, 60 edges | 2 | 58 | 9834 | 9834 | 1 | 359592 | 9.400 / 9.540 | 2.845 / 3.007 | 13.731 / 14.215 | – | – | – | – |
| knob pointer | 2 | 1 | 10 | 10 | 1 | 456 | 0.015 / 0.029 | 0.341 / 0.546 | 0.010 / 0.014 | – | – | – | – |
| trace, 800 points, view scale 12 | 2 | 11 | 1883 | 1524 | 3 | 57036 | 0.394 / 0.615 | 1.664 / 3.083 | 1.814 / 3.211 | – | – | – | – |
| trace, 800 points, unchanged | 2 | 799 | 7510 | 3487 | 6 | 192792 | 0.000 / 0.000 | 1.588 / 2.940 | 2.368 / 2.459 | – | – | – | – |
<!-- end 2026-11-12T21:47:27+01:00, load 1.03 0.97 1.00, highest during the run 1.02
%CPU COMMAND
 4.1 chrome
 2.7 gnome-terminal-
 2.6 gnome-shell
 2.0 chrome
 1.4 claude
-->
