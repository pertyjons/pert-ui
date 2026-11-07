<!-- start 2026-11-05T18:03:33+01:00, waited 0 min, load 0.81 1.21 3.56
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
| Panels (150 rounded + 100 plain) | 1 | pert-ui wgpu | 1.009 / 1.240 | compose: 0.164 / 0.187<br>`render` call: 0.815 / 1.071<br>of which until `before_present`: 0.770 / 1.031 | not reachable through the public API | 0 |
| Panels light (20 rounded + 20 plain) | 1 | pert-ui wgpu | 0.432 / 0.514 | compose: 0.042 / 0.063<br>`render` call: 0.357 / 0.421<br>of which until `before_present`: 0.313 / 0.371 | not reachable through the public API | 0 |
| Polyline 800 points | 1 | pert-ui wgpu | 3.421 / 3.586 | compose: 0.030 / 0.041<br>`render` call: 3.349 / 3.498<br>of which until `before_present`: 3.306 / 3.436 | not reachable through the public API | 0 |
| Polyline 4096 points | 1 | pert-ui wgpu | 4.645 / 4.754 | compose: 0.106 / 0.118<br>`render` call: 4.451 / 4.552<br>of which until `before_present`: 4.407 / 4.502 | not reachable through the public API | 0 |
| Gradient bars (32) | 1 | pert-ui wgpu | 0.444 / 0.521 | compose: 0.016 / 0.034<br>`render` call: 0.396 / 0.448<br>of which until `before_present`: 0.351 / 0.394 | not reachable through the public API | 0 |
| Panels (150 rounded + 100 plain) | 1 | vello_gpu | 1.470 / 1.578 | encode: 1.124 / 1.194<br>acquire: 0.035 / 0.050<br>prepare (`Renderer::render`): 0.220 / 0.248<br>submit: 0.059 / 0.071<br>present: 0.040 / 0.050 | 2.136 / 3.179 | 0 |
| Panels light (20 rounded + 20 plain) | 1 | vello_gpu | 0.614 / 0.720 | encode: 0.291 / 0.352<br>acquire: 0.045 / 0.056<br>prepare (`Renderer::render`): 0.162 / 0.198<br>submit: 0.067 / 0.081<br>present: 0.044 / 0.053 | 1.878 / 3.209 | 0 |
| Polyline 800 points | 1 | vello_gpu | 0.740 / 0.808 | encode: 0.441 / 0.499<br>acquire: 0.041 / 0.052<br>prepare (`Renderer::render`): 0.142 / 0.161<br>submit: 0.064 / 0.072<br>present: 0.043 / 0.050 | 1.609 / 1.788 | 0 |
| Polyline 4096 points | 1 | vello_gpu | 1.141 / 1.195 | encode: 0.843 / 0.885<br>acquire: 0.044 / 0.053<br>prepare (`Renderer::render`): 0.143 / 0.168<br>submit: 0.065 / 0.072<br>present: 0.043 / 0.050 | 1.584 / 1.793 | 0 |
| Gradient bars (32) | 1 | vello_gpu | 0.301 / 0.344 | encode: 0.027 / 0.056<br>acquire: 0.045 / 0.062<br>prepare (`Renderer::render`): 0.116 / 0.133<br>submit: 0.067 / 0.073<br>present: 0.043 / 0.051 | 1.544 / 2.380 | 0 |
| Panels (150 rounded + 100 plain) | 2 | pert-ui wgpu | 0.875 / 1.026 | compose: 0.165 / 0.214<br>`render` call: 0.675 / 0.799<br>of which until `before_present`: 0.640 / 0.747 | not reachable through the public API | 0 |
| Panels light (20 rounded + 20 plain) | 2 | pert-ui wgpu | 0.415 / 0.500 | compose: 0.041 / 0.063<br>`render` call: 0.350 / 0.426<br>of which until `before_present`: 0.306 / 0.375 | not reachable through the public API | 0 |
| Polyline 800 points | 2 | pert-ui wgpu | 7.045 / 7.220 | compose: 0.034 / 0.040<br>`render` call: 6.962 / 7.134<br>of which until `before_present`: 6.918 / 7.090 | not reachable through the public API | 0 |
| Polyline 4096 points | 2 | pert-ui wgpu | 8.670 / 8.887 | compose: 0.110 / 0.127<br>`render` call: 8.471 / 8.690<br>of which until `before_present`: 8.426 / 8.646 | not reachable through the public API | 0 |
| Gradient bars (32) | 2 | pert-ui wgpu | 0.447 / 0.492 | compose: 0.016 / 0.034<br>`render` call: 0.398 / 0.437<br>of which until `before_present`: 0.352 / 0.389 | not reachable through the public API | 0 |
| Panels (150 rounded + 100 plain) | 2 | vello_gpu | 1.821 / 1.907 | encode: 1.410 / 1.457<br>acquire: 0.034 / 0.047<br>prepare (`Renderer::render`): 0.286 / 0.341<br>submit: 0.055 / 0.068<br>present: 0.038 / 0.046 | 3.961 / 4.914 | 0 |
| Panels light (20 rounded + 20 plain) | 2 | vello_gpu | 0.709 / 0.765 | encode: 0.377 / 0.425<br>acquire: 0.044 / 0.053<br>prepare (`Renderer::render`): 0.183 / 0.213<br>submit: 0.062 / 0.076<br>present: 0.042 / 0.049 | 3.531 / 3.727 | 0 |
| Polyline 800 points | 2 | vello_gpu | 1.101 / 1.159 | encode: 0.784 / 0.825<br>acquire: 0.045 / 0.054<br>prepare (`Renderer::render`): 0.161 / 0.185<br>submit: 0.065 / 0.070<br>present: 0.043 / 0.048 | 3.253 / 5.612 | 0 |
| Polyline 4096 points | 2 | vello_gpu | 1.533 / 1.582 | encode: 1.226 / 1.265<br>acquire: 0.042 / 0.050<br>prepare (`Renderer::render`): 0.160 / 0.181<br>submit: 0.062 / 0.080<br>present: 0.041 / 0.047 | 3.280 / 3.874 | 0 |
| Gradient bars (32) | 2 | vello_gpu | 0.301 / 0.341 | encode: 0.027 / 0.054<br>acquire: 0.045 / 0.059<br>prepare (`Renderer::render`): 0.117 / 0.142<br>submit: 0.066 / 0.077<br>present: 0.043 / 0.048 | 3.189 / 3.894 | 0 |
<!-- end 2026-11-05T18:05:20+01:00, load 0.37 0.91 3.18
%CPU COMMAND
 5.3 /usr/libexec/gnome-terminal-server
 4.1 /usr/bin/gnome-shell --mode=user
 3.9 codex
 1.4 claude
 1.4 ~/.local/share/JetBrains/Toolbox/bin/jetbrains-toolbox --minimize
-->
