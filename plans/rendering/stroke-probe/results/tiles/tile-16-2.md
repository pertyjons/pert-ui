<!-- start 2026-11-12T21:47:27+01:00, waited 0 min, load 1.03 0.97 1.00
%CPU COMMAND
 4.1 chrome
 2.7 gnome-terminal-
 2.6 gnome-shell
 2.0 chrome
 1.4 claude
-->
# Stroke prototype: timing

Adapter: Intel(R) UHD Graphics 770 (ADL-S GT1) (Vulkan, Mesa 26.2.6). Tile size 16. 12 warm-up and 300 measured frames per workload. Prepare = preparation, encoding and upload; GPU = encode, submit and wait for one offscreen pass; CPU = preparation and per-pixel evaluation and blending into a linear frame; today's CPU = the CPU renderer's frame with the strokes minus without them. Presentation is not measured.

| Workload | Scale | Segments | References | Tiles | Longest list | Bytes | Prepare | GPU | CPU | Today: rasterize | Today: prepare | Today: GPU | Today: CPU |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| trace, 800 points | 1 | 799 | 2159 | 478 | 14 | 75100 | 0.152 / 0.160 | 2.343 / 2.705 | 2.541 / 2.624 | – | – | – | – |
| trace, 4096 points | 1 | 4095 | 6498 | 493 | 40 | 303880 | 0.525 / 0.541 | 3.149 / 3.566 | 6.720 / 6.789 | – | – | – | – |
| noise, 800 points | 1 | 799 | 8429 | 1163 | 15 | 122100 | 0.270 / 0.281 | 4.157 / 4.688 | 8.833 / 9.164 | – | – | – | – |
| steep alternation, 400 points | 1 | 399 | 11823 | 1326 | 10 | 115292 | 0.305 / 0.311 | 5.407 / 5.833 | 11.515 / 11.635 | – | – | – | – |
| noise, 16384 points | 1 | 16383 | 175888 | 1323 | 229 | 1794432 | 4.471 / 5.299 | 29.316 / 29.672 | 159.409 / 161.508 | – | – | – | – |
| diagram, 60 edges | 1 | 58 | 2035 | 2035 | 1 | 78828 | 0.652 / 0.677 | 1.621 / 1.796 | 3.771 / 3.921 | – | – | – | – |
| knob pointer | 1 | 1 | 2 | 2 | 1 | 168 | 0.008 / 0.008 | 0.376 / 0.406 | 0.004 / 0.004 | – | – | – | – |
| trace, 800 points, view scale 12 | 1 | 11 | 204 | 151 | 3 | 6384 | 0.063 / 0.083 | 0.793 / 1.116 | 0.556 / 0.995 | – | – | – | – |
| trace, 800 points, unchanged | 1 | 799 | 2191 | 476 | 13 | 75164 | 0.000 / 0.000 | 2.711 / 2.857 | 2.402 / 2.449 | – | – | – | – |
| trace, 800 points | 2 | 799 | 3655 | 1291 | 10 | 107100 | 0.226 / 0.238 | 2.165 / 3.884 | 4.292 / 4.398 | – | – | – | – |
| trace, 4096 points | 2 | 4095 | 8885 | 1337 | 20 | 340436 | 0.657 / 0.751 | 3.910 / 4.321 | 9.457 / 10.496 | – | – | – | – |
| noise, 800 points | 2 | 799 | 17313 | 4154 | 10 | 253348 | 0.557 / 0.625 | 7.549 / 8.619 | 17.833 / 18.593 | – | – | – | – |
| steep alternation, 400 points | 2 | 399 | 24926 | 5123 | 6 | 289208 | 0.715 / 0.736 | 7.609 / 8.384 | 23.282 / 23.561 | – | – | – | – |
| noise, 16384 points | 2 | 16383 | 358279 | 5123 | 128 | 2645596 | 8.225 / 9.731 | 55.025 / 62.729 | 320.830 / 325.747 | – | – | – | – |
| diagram, 60 edges | 2 | 58 | 4323 | 4323 | 1 | 161196 | 2.415 / 2.446 | 3.229 / 3.375 | 9.155 / 9.317 | – | – | – | – |
| knob pointer | 2 | 1 | 3 | 3 | 1 | 204 | 0.013 / 0.020 | 0.285 / 0.456 | 0.010 / 0.012 | – | – | – | – |
| trace, 800 points, view scale 12 | 2 | 11 | 576 | 447 | 3 | 17344 | 0.148 / 0.203 | 1.710 / 2.755 | 1.756 / 3.182 | – | – | – | – |
| trace, 800 points, unchanged | 2 | 799 | 3696 | 1289 | 9 | 107200 | 0.000 / 0.000 | 2.250 / 2.386 | 4.110 / 4.660 | – | – | – | – |
<!-- end 2026-11-12T21:51:25+01:00, load 0.83 0.91 0.98, highest during the run 1.03
%CPU COMMAND
 4.1 chrome
 2.6 gnome-terminal-
 2.5 gnome-shell
 2.0 chrome
 1.4 chrome
-->
