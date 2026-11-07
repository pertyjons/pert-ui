<!-- start 2026-11-05T18:05:20+01:00, waited 0 min, load 0.37 0.91 3.18
%CPU COMMAND
 5.3 /usr/libexec/gnome-terminal-server
 4.1 /usr/bin/gnome-shell --mode=user
 3.9 codex
 1.4 claude
 1.4 ~/.local/share/JetBrains/Toolbox/bin/jetbrains-toolbox --minimize
-->
# Presentation without read-back

Monitors: HDMI-3 3440×1440 @ 99.981 Hz, scale factor 1

Frames paced to one per 30 ms; 150 measured frames after 20 warm-up frames per renderer, workload and scale. Times in ms, median / p95.

- pert-ui wgpu, scale 1: requested 1280×800, got 1280×800 physical (window scale factor 1).
- vello_gpu, scale 1: requested 1280×800, got 1280×800 physical (window scale factor 1).
  vello_gpu surface format Bgra8Unorm (formats offered: [Bgra8UnormSrgb, Rgba8UnormSrgb, Rgba16Float, Rgba16Unorm, Rgb10a2Unorm, Bgra8Unorm, Rgba8Unorm]), alpha Opaque, present mode Fifo, desired maximum frame latency 2.
- pert-ui wgpu, scale 2: requested 2560×1600, got 2560×1373 physical (window scale factor 1).
- vello_gpu, scale 2: requested 2560×1600, got 2560×1373 physical (window scale factor 1).
  vello_gpu surface format Bgra8Unorm (formats offered: [Bgra8UnormSrgb, Rgba8UnormSrgb, Rgba16Float, Rgba16Unorm, Rgb10a2Unorm, Bgra8Unorm, Rgba8Unorm]), alpha Opaque, present mode Fifo, desired maximum frame latency 2.

| Workload | Scale | Renderer | UI frame (start → after present) | Phases | GPU completion after submit | Frames not presented |
|---|---|---|---|---|---|---|
| Panels (150 rounded + 100 plain) | 1 | pert-ui wgpu | 0.987 / 1.197 | compose: 0.161 / 0.191<br>`render` call: 0.792 / 0.886<br>of which until `before_present`: 0.746 / 0.836 | not reachable through the public API | 0 |
| Panels light (20 rounded + 20 plain) | 1 | pert-ui wgpu | 0.448 / 0.507 | compose: 0.039 / 0.062<br>`render` call: 0.379 / 0.430<br>of which until `before_present`: 0.333 / 0.377 | not reachable through the public API | 0 |
| Polyline 800 points | 1 | pert-ui wgpu | 3.488 / 3.594 | compose: 0.033 / 0.042<br>`render` call: 3.405 / 3.511<br>of which until `before_present`: 3.360 / 3.466 | not reachable through the public API | 0 |
| Polyline 4096 points | 1 | pert-ui wgpu | 4.636 / 4.733 | compose: 0.108 / 0.119<br>`render` call: 4.437 / 4.543<br>of which until `before_present`: 4.392 / 4.494 | not reachable through the public API | 0 |
| Gradient bars (32) | 1 | pert-ui wgpu | 0.446 / 0.505 | compose: 0.016 / 0.030<br>`render` call: 0.398 / 0.445<br>of which until `before_present`: 0.351 / 0.398 | not reachable through the public API | 0 |
| Panels (150 rounded + 100 plain) | 1 | vello_gpu | 1.484 / 1.551 | encode: 1.124 / 1.180<br>acquire: 0.041 / 0.053<br>prepare (`Renderer::render`): 0.218 / 0.244<br>submit: 0.060 / 0.069<br>present: 0.043 / 0.048 | 2.154 / 3.114 | 0 |
| Panels light (20 rounded + 20 plain) | 1 | vello_gpu | 0.577 / 0.621 | encode: 0.267 / 0.309<br>acquire: 0.045 / 0.051<br>prepare (`Renderer::render`): 0.158 / 0.176<br>submit: 0.063 / 0.074<br>present: 0.043 / 0.049 | 1.821 / 2.848 | 0 |
| Polyline 800 points | 1 | vello_gpu | 0.734 / 0.779 | encode: 0.437 / 0.486<br>acquire: 0.045 / 0.051<br>prepare (`Renderer::render`): 0.140 / 0.157<br>submit: 0.062 / 0.070<br>present: 0.042 / 0.051 | 1.691 / 2.770 | 0 |
| Polyline 4096 points | 1 | vello_gpu | 1.119 / 1.176 | encode: 0.824 / 0.874<br>acquire: 0.043 / 0.051<br>prepare (`Renderer::render`): 0.141 / 0.158<br>submit: 0.062 / 0.068<br>present: 0.043 / 0.049 | 1.620 / 1.814 | 0 |
| Gradient bars (32) | 1 | vello_gpu | 0.283 / 0.319 | encode: 0.034 / 0.051<br>acquire: 0.039 / 0.054<br>prepare (`Renderer::render`): 0.111 / 0.125<br>submit: 0.058 / 0.071<br>present: 0.041 / 0.048 | 1.503 / 1.719 | 0 |
| Panels (150 rounded + 100 plain) | 2 | pert-ui wgpu | 0.977 / 1.037 | compose: 0.165 / 0.192<br>`render` call: 0.778 / 0.844<br>of which until `before_present`: 0.732 / 0.796 | not reachable through the public API | 0 |
| Panels light (20 rounded + 20 plain) | 2 | pert-ui wgpu | 0.453 / 0.588 | compose: 0.042 / 0.064<br>`render` call: 0.379 / 0.511<br>of which until `before_present`: 0.333 / 0.455 | not reachable through the public API | 0 |
| Polyline 800 points | 2 | pert-ui wgpu | 7.028 / 7.215 | compose: 0.035 / 0.038<br>`render` call: 6.942 / 7.132<br>of which until `before_present`: 6.896 / 7.072 | not reachable through the public API | 0 |
| Polyline 4096 points | 2 | pert-ui wgpu | 8.646 / 8.823 | compose: 0.112 / 0.121<br>`render` call: 8.449 / 8.606<br>of which until `before_present`: 8.401 / 8.555 | not reachable through the public API | 0 |
| Gradient bars (32) | 2 | pert-ui wgpu | 0.443 / 0.490 | compose: 0.017 / 0.031<br>`render` call: 0.398 / 0.436<br>of which until `before_present`: 0.351 / 0.384 | not reachable through the public API | 0 |
| Panels (150 rounded + 100 plain) | 2 | vello_gpu | 1.850 / 1.921 | encode: 1.409 / 1.460<br>acquire: 0.045 / 0.051<br>prepare (`Renderer::render`): 0.288 / 0.323<br>submit: 0.064 / 0.076<br>present: 0.043 / 0.050 | 4.072 / 4.315 | 0 |
| Panels light (20 rounded + 20 plain) | 2 | vello_gpu | 0.705 / 0.764 | encode: 0.372 / 0.418<br>acquire: 0.045 / 0.053<br>prepare (`Renderer::render`): 0.182 / 0.211<br>submit: 0.065 / 0.074<br>present: 0.043 / 0.048 | 3.555 / 4.306 | 0 |
| Polyline 800 points | 2 | vello_gpu | 1.093 / 1.149 | encode: 0.776 / 0.823<br>acquire: 0.046 / 0.054<br>prepare (`Renderer::render`): 0.161 / 0.186<br>submit: 0.066 / 0.074<br>present: 0.043 / 0.048 | 3.233 / 4.380 | 0 |
| Polyline 4096 points | 2 | vello_gpu | 1.534 / 1.593 | encode: 1.220 / 1.267<br>acquire: 0.046 / 0.053<br>prepare (`Renderer::render`): 0.160 / 0.181<br>submit: 0.064 / 0.071<br>present: 0.043 / 0.049 | 3.253 / 4.438 | 0 |
| Gradient bars (32) | 2 | vello_gpu | 0.300 / 0.340 | encode: 0.025 / 0.050<br>acquire: 0.046 / 0.058<br>prepare (`Renderer::render`): 0.116 / 0.131<br>submit: 0.067 / 0.076<br>present: 0.043 / 0.051 | 3.217 / 4.455 | 0 |
<!-- end 2026-11-05T18:07:07+01:00, load 0.42 0.76 2.88
%CPU COMMAND
 5.3 /usr/libexec/gnome-terminal-server
 4.1 /usr/bin/gnome-shell --mode=user
 3.9 codex
 1.4 claude
 1.4 ~/.local/share/JetBrains/Toolbox/bin/jetbrains-toolbox --minimize
-->
