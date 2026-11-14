<!-- start 2026-11-12T22:01:16+01:00, waited 0 min, load 1.23 1.01 1.00
%CPU COMMAND
 4.2 chrome
 2.5 gnome-terminal-
 2.4 gnome-shell
 1.9 chrome
 1.9 chrome
-->
# Stroke prototype: timing

Adapter: Intel(R) UHD Graphics 770 (ADL-S GT1) (Vulkan, Mesa 26.2.6). Tile size 16. 12 warm-up and 300 measured frames per workload. Prepare = preparation, encoding and upload; GPU = encode, submit and wait for one offscreen pass; CPU = preparation and per-pixel evaluation and blending into a linear frame; today's CPU = the CPU renderer's frame with the strokes minus without them. Presentation is not measured.

| Workload | Scale | Segments | References | Tiles | Longest list | Bytes | Prepare | GPU | CPU | Today: rasterize | Today: prepare | Today: GPU | Today: CPU |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| trace, 800 points | 1 | 799 | 2159 | 478 | 14 | 75100 | 0.151 / 0.156 | 1.320 / 2.602 | 2.522 / 2.570 | – | – | – | – |
| trace, 4096 points | 1 | 4095 | 6498 | 493 | 40 | 303880 | 0.531 / 0.559 | 3.092 / 3.550 | 6.752 / 6.905 | – | – | – | – |
| noise, 800 points | 1 | 799 | 8429 | 1163 | 15 | 122100 | 0.274 / 0.319 | 4.100 / 4.713 | 8.894 / 9.577 | – | – | – | – |
| steep alternation, 400 points | 1 | 399 | 11823 | 1326 | 10 | 115292 | 0.305 / 0.320 | 5.429 / 5.830 | 11.521 / 11.689 | – | – | – | – |
| noise, 16384 points | 1 | 16383 | 175888 | 1323 | 229 | 1794432 | 4.469 / 5.248 | 29.324 / 29.671 | 159.335 / 162.047 | – | – | – | – |
| diagram, 60 edges | 1 | 58 | 2035 | 2035 | 1 | 78828 | 0.647 / 0.667 | 1.593 / 1.698 | 3.755 / 3.818 | – | – | – | – |
| knob pointer | 1 | 1 | 2 | 2 | 1 | 168 | 0.008 / 0.008 | 0.376 / 0.414 | 0.004 / 0.004 | – | – | – | – |
| trace, 800 points, view scale 12 | 1 | 11 | 204 | 151 | 3 | 6384 | 0.063 / 0.083 | 0.769 / 1.117 | 0.562 / 1.002 | – | – | – | – |
| trace, 800 points, unchanged | 1 | 799 | 2191 | 476 | 13 | 75164 | 0.000 / 0.000 | 2.673 / 2.838 | 2.397 / 2.429 | – | – | – | – |
| trace, 800 points | 2 | 799 | 3655 | 1291 | 10 | 107100 | 0.227 / 0.242 | 2.139 / 2.363 | 4.299 / 4.411 | – | – | – | – |
| trace, 4096 points | 2 | 4095 | 8885 | 1337 | 20 | 340436 | 0.654 / 0.686 | 3.919 / 4.335 | 9.419 / 9.660 | – | – | – | – |
| noise, 800 points | 2 | 799 | 17313 | 4154 | 10 | 253348 | 0.560 / 0.633 | 7.382 / 8.445 | 17.968 / 18.666 | – | – | – | – |
| steep alternation, 400 points | 2 | 399 | 24926 | 5123 | 6 | 289208 | 0.719 / 0.773 | 7.659 / 8.444 | 23.297 / 24.348 | – | – | – | – |
| noise, 16384 points | 2 | 16383 | 358279 | 5123 | 128 | 2645596 | 8.238 / 9.737 | 54.908 / 56.933 | 320.790 / 325.781 | – | – | – | – |
| diagram, 60 edges | 2 | 58 | 4323 | 4323 | 1 | 161196 | 2.420 / 2.452 | 3.228 / 3.374 | 9.160 / 9.371 | – | – | – | – |
| knob pointer | 2 | 1 | 3 | 3 | 1 | 204 | 0.013 / 0.021 | 0.443 / 0.536 | 0.010 / 0.013 | – | – | – | – |
| trace, 800 points, view scale 12 | 2 | 11 | 576 | 447 | 3 | 17344 | 0.142 / 0.202 | 1.741 / 2.770 | 1.733 / 3.146 | – | – | – | – |
| trace, 800 points, unchanged | 2 | 799 | 3696 | 1289 | 9 | 107200 | 0.000 / 0.000 | 2.268 / 2.386 | 4.093 / 4.174 | – | – | – | – |
<!-- end 2026-11-12T22:05:13+01:00, load 0.99 1.00 1.00, highest during the run 1.38
%CPU COMMAND
 4.2 chrome
 2.4 gnome-terminal-
 2.3 gnome-shell
 1.9 chrome
 1.3 chrome
-->
