<!-- start 2026-11-05T18:01:45+01:00, waited 0 min, load 0.72 1.39 3.90
%CPU COMMAND
 5.3 /usr/libexec/gnome-terminal-server
 4.7 /bin/bash -c (agent shell command)
 4.1 /usr/bin/gnome-shell --mode=user
 3.9 codex
 1.5 claude -p Focused independent semantic/design reread of P1-P3 and C1-C3 in /tmp/phase10c-save-coordinator-design-re
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
| Panels (150 rounded + 100 plain) | 1 | pert-ui wgpu | 0.986 / 1.034 | compose: 0.162 / 0.188<br>`render` call: 0.793 / 0.834<br>of which until `before_present`: 0.747 / 0.783 | not reachable through the public API | 0 |
| Panels light (20 rounded + 20 plain) | 1 | pert-ui wgpu | 0.445 / 0.488 | compose: 0.040 / 0.061<br>`render` call: 0.374 / 0.407<br>of which until `before_present`: 0.328 / 0.357 | not reachable through the public API | 0 |
| Polyline 800 points | 1 | pert-ui wgpu | 3.493 / 3.588 | compose: 0.032 / 0.037<br>`render` call: 3.413 / 3.498<br>of which until `before_present`: 3.369 / 3.452 | not reachable through the public API | 0 |
| Polyline 4096 points | 1 | pert-ui wgpu | 4.704 / 4.834 | compose: 0.109 / 0.120<br>`render` call: 4.501 / 4.616<br>of which until `before_present`: 4.454 / 4.560 | not reachable through the public API | 0 |
| Gradient bars (32) | 1 | pert-ui wgpu | 0.441 / 0.534 | compose: 0.016 / 0.032<br>`render` call: 0.393 / 0.493<br>of which until `before_present`: 0.349 / 0.441 | not reachable through the public API | 0 |
| Panels (150 rounded + 100 plain) | 1 | vello_gpu | 1.453 / 1.551 | encode: 1.116 / 1.175<br>acquire: 0.037 / 0.048<br>prepare (`Renderer::render`): 0.218 / 0.246<br>submit: 0.057 / 0.070<br>present: 0.039 / 0.047 | 2.117 / 3.957 | 0 |
| Panels light (20 rounded + 20 plain) | 1 | vello_gpu | 0.543 / 0.619 | encode: 0.282 / 0.322<br>acquire: 0.033 / 0.048<br>prepare (`Renderer::render`): 0.152 / 0.176<br>submit: 0.055 / 0.068<br>present: 0.037 / 0.045 | 1.681 / 2.216 | 0 |
| Polyline 800 points | 1 | vello_gpu | 0.737 / 0.787 | encode: 0.441 / 0.494<br>acquire: 0.044 / 0.050<br>prepare (`Renderer::render`): 0.141 / 0.159<br>submit: 0.063 / 0.071<br>present: 0.043 / 0.049 | 1.646 / 2.722 | 0 |
| Polyline 4096 points | 1 | vello_gpu | 1.137 / 1.203 | encode: 0.839 / 0.895<br>acquire: 0.044 / 0.051<br>prepare (`Renderer::render`): 0.143 / 0.168<br>submit: 0.065 / 0.073<br>present: 0.043 / 0.051 | 1.629 / 1.813 | 0 |
| Gradient bars (32) | 1 | vello_gpu | 0.294 / 0.340 | encode: 0.023 / 0.051<br>acquire: 0.044 / 0.056<br>prepare (`Renderer::render`): 0.115 / 0.138<br>submit: 0.065 / 0.080<br>present: 0.042 / 0.048 | 1.586 / 1.721 | 0 |
| Panels (150 rounded + 100 plain) | 2 | pert-ui wgpu | 0.976 / 1.053 | compose: 0.166 / 0.195<br>`render` call: 0.777 / 0.840<br>of which until `before_present`: 0.731 / 0.795 | not reachable through the public API | 0 |
| Panels light (20 rounded + 20 plain) | 2 | pert-ui wgpu | 0.449 / 0.506 | compose: 0.041 / 0.062<br>`render` call: 0.377 / 0.432<br>of which until `before_present`: 0.332 / 0.381 | not reachable through the public API | 0 |
| Polyline 800 points | 2 | pert-ui wgpu | 7.067 / 7.187 | compose: 0.034 / 0.038<br>`render` call: 6.977 / 7.108<br>of which until `before_present`: 6.932 / 7.051 | not reachable through the public API | 0 |
| Polyline 4096 points | 2 | pert-ui wgpu | 8.745 / 8.874 | compose: 0.110 / 0.119<br>`render` call: 8.536 / 8.667<br>of which until `before_present`: 8.490 / 8.608 | not reachable through the public API | 0 |
| Gradient bars (32) | 2 | pert-ui wgpu | 0.447 / 0.512 | compose: 0.017 / 0.035<br>`render` call: 0.397 / 0.440<br>of which until `before_present`: 0.351 / 0.394 | not reachable through the public API | 0 |
| Panels (150 rounded + 100 plain) | 2 | vello_gpu | 1.844 / 1.912 | encode: 1.408 / 1.454<br>acquire: 0.041 / 0.050<br>prepare (`Renderer::render`): 0.290 / 0.329<br>submit: 0.061 / 0.073<br>present: 0.042 / 0.050 | 4.057 / 5.736 | 0 |
| Panels light (20 rounded + 20 plain) | 2 | vello_gpu | 0.699 / 0.769 | encode: 0.376 / 0.435<br>acquire: 0.040 / 0.049<br>prepare (`Renderer::render`): 0.181 / 0.206<br>submit: 0.061 / 0.076<br>present: 0.042 / 0.048 | 3.528 / 4.667 | 0 |
| Polyline 800 points | 2 | vello_gpu | 1.056 / 1.145 | encode: 0.778 / 0.827<br>acquire: 0.031 / 0.047<br>prepare (`Renderer::render`): 0.149 / 0.179<br>submit: 0.057 / 0.068<br>present: 0.038 / 0.048 | 3.134 / 4.420 | 0 |
| Polyline 4096 points | 2 | vello_gpu | 1.501 / 1.619 | encode: 1.227 / 1.284<br>acquire: 0.033 / 0.046<br>prepare (`Renderer::render`): 0.146 / 0.191<br>submit: 0.055 / 0.082<br>present: 0.038 / 0.050 | 3.056 / 3.361 | 0 |
| Gradient bars (32) | 2 | vello_gpu | 0.268 / 0.337 | encode: 0.043 / 0.053<br>acquire: 0.036 / 0.056<br>prepare (`Renderer::render`): 0.105 / 0.138<br>submit: 0.056 / 0.076<br>present: 0.038 / 0.050 | 3.063 / 4.125 | 0 |
<!-- end 2026-11-05T18:03:33+01:00, load 0.81 1.21 3.56
%CPU COMMAND
 5.3 /usr/libexec/gnome-terminal-server
 4.1 /usr/bin/gnome-shell --mode=user
 3.9 codex
 1.4 claude
 1.4 ~/.local/share/JetBrains/Toolbox/bin/jetbrains-toolbox --minimize
-->
