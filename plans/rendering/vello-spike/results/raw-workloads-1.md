<!-- load average before: 6.44 9.56 8.39 -->
# vello spike measurements

vello_gpu adapter: AdapterInfo { name: "Intel(R) UHD Graphics 770 (ADL-S GT1)", vendor: 32902, device: 18048, device_type: IntegratedGpu, device_pci_bus_id: "0000:00:02.0", driver: "Intel open-source Mesa driver", driver_info: "Mesa 26.2.6", backend: Vulkan, subgroup_min_size: 8, subgroup_max_size: 32, transient_saves_memory: Some(false), limit_bucket: None }

200 measured frames after 20 warm-up frames per renderer, workload and scale. vello_cpu multithreaded uses 5 worker threads (available_parallelism − 1, capped at 8, vello's default). Times in ms, median / p95.

## Scale 1 (1280×800 physical)

Empty scene, render + read-back median: pert-ui wgpu 2.968 ms, vello_gpu 2.386 ms.

### Panels (150 rounded + 100 plain) — scale 1

pert-ui command mix: 1151 FillRect, 1200 DrawMask (10 mask resources, 1 KiB), 0 DrawImage, 0 DrawVector.

| Renderer | Frame total median / p95 ms | Phases median / p95 ms | Not in total: update median ms | Cold compose ms |
|---|---|---|---|---|
| pert-ui CPU | 4.206 / 4.298 | compose (paint): 0.146 / 0.155<br>CpuRenderer::render: 4.061 / 4.148 | 0.001 | 0.195 |
| pert-ui wgpu | 4.842 / 5.127 | compose (paint): 0.154 / 0.162<br>render_offscreen (prepare + GPU + read-back): 4.688 / 4.981 | 0.006 | 0.184 |
| vello_cpu 1 thread | 1.466 / 1.509 | encode (+flush): 1.049 / 1.083<br>render: 0.418 / 0.431 | – | – |
| vello_cpu 5 workers | 0.543 / 0.561 | encode (+flush): 0.347 / 0.358<br>render: 0.195 / 0.215 | – | – |
| vello_gpu | 4.306 / 6.366 | encode (Scene): 1.084 / 1.097<br>prepare (Renderer::render): 0.158 / 0.224<br>submit + GPU completion: 1.703 / 2.785<br>read-back: 1.343 / 2.376 | – | – |

### Panels light (20 rounded + 20 plain) — scale 1

pert-ui command mix: 161 FillRect, 160 DrawMask (10 mask resources, 1 KiB), 0 DrawImage, 0 DrawVector.

| Renderer | Frame total median / p95 ms | Phases median / p95 ms | Not in total: update median ms | Cold compose ms |
|---|---|---|---|---|
| pert-ui CPU | 2.610 / 2.647 | compose (paint): 0.023 / 0.024<br>CpuRenderer::render: 2.587 / 2.624 | 0.001 | 0.069 |
| pert-ui wgpu | 3.977 / 6.103 | compose (paint): 0.030 / 0.038<br>render_offscreen (prepare + GPU + read-back): 3.945 / 6.066 | 0.007 | 0.058 |
| vello_cpu 1 thread | 0.477 / 0.485 | encode (+flush): 0.202 / 0.207<br>render: 0.274 / 0.280 | – | – |
| vello_cpu 5 workers | 0.220 / 0.230 | encode (+flush): 0.100 / 0.106<br>render: 0.120 / 0.126 | – | – |
| vello_gpu | 4.870 / 5.355 | encode (Scene): 0.233 / 0.245<br>prepare (Renderer::render): 0.095 / 0.113<br>submit + GPU completion: 2.412 / 2.599<br>read-back: 2.148 / 2.403 | – | – |

### Polyline 800 points — scale 1

pert-ui command mix: 1 FillRect, 0 DrawMask (0 mask resources, 0 KiB), 0 DrawImage, 1 DrawVector.

| Renderer | Frame total median / p95 ms | Phases median / p95 ms | Not in total: update median ms | Cold compose ms |
|---|---|---|---|---|
| pert-ui CPU | 12.143 / 12.358 | compose (paint): 0.028 / 0.030<br>CpuRenderer::render: 12.116 / 12.330 | 0.018 | 0.029 |
| pert-ui wgpu | 5.749 / 7.239 | compose (paint): 0.028 / 0.032<br>render_offscreen (prepare + GPU + read-back): 5.716 / 7.211 | 0.024 | 0.027 |
| vello_cpu 1 thread | 0.602 / 0.615 | encode (+flush): 0.416 / 0.427<br>render: 0.185 / 0.191 | – | – |
| vello_cpu 5 workers | 0.689 / 0.750 | encode (+flush): 0.591 / 0.611<br>render: 0.116 / 0.144 | – | – |
| vello_gpu | 4.700 / 6.645 | encode (Scene): 0.446 / 0.457<br>prepare (Renderer::render): 0.107 / 0.153<br>submit + GPU completion: 1.953 / 2.279<br>read-back: 2.137 / 2.664 | – | – |

### Polyline 4096 points — scale 1

pert-ui command mix: 1 FillRect, 0 DrawMask (0 mask resources, 0 KiB), 0 DrawImage, 1 DrawVector.

| Renderer | Frame total median / p95 ms | Phases median / p95 ms | Not in total: update median ms | Cold compose ms |
|---|---|---|---|---|
| pert-ui CPU | 14.429 / 22.123 | compose (paint): 0.114 / 0.201<br>CpuRenderer::render: 14.316 / 21.909 | 0.064 | 0.105 |
| pert-ui wgpu | 12.438 / 14.674 | compose (paint): 0.193 / 0.218<br>render_offscreen (prepare + GPU + read-back): 12.203 / 14.482 | 0.111 | 0.183 |
| vello_cpu 1 thread | 1.874 / 2.050 | encode (+flush): 1.390 / 1.473<br>render: 0.478 / 0.624 | – | – |
| vello_cpu 5 workers | 4.859 / 7.819 | encode (+flush): 4.324 / 7.200<br>render: 0.532 / 0.716 | – | – |
| vello_gpu | 3.956 / 6.875 | encode (Scene): 0.897 / 1.447<br>prepare (Renderer::render): 0.095 / 0.251<br>submit + GPU completion: 1.354 / 2.386<br>read-back: 1.518 / 2.705 | – | – |

### Gradient bars (32) — scale 1

pert-ui command mix: 33 FillRect, 0 DrawMask (0 mask resources, 0 KiB), 32 DrawImage, 0 DrawVector.

| Renderer | Frame total median / p95 ms | Phases median / p95 ms | Not in total: update median ms | Cold compose ms |
|---|---|---|---|---|
| pert-ui CPU | 11.826 / 12.121 | compose (paint): 0.010 / 0.011<br>CpuRenderer::render: 11.817 / 12.110 | 0.004 | 0.012 |
| pert-ui wgpu | 4.064 / 4.392 | compose (paint): 0.010 / 0.015<br>render_offscreen (prepare + GPU + read-back): 4.055 / 4.377 | 0.009 | 0.007 |
| vello_cpu 1 thread | 0.711 / 0.744 | encode (+flush): 0.030 / 0.032<br>render: 0.681 / 0.713 | – | – |
| vello_cpu 5 workers | 0.402 / 0.432 | encode (+flush): 0.057 / 0.063<br>render: 0.345 / 0.377 | – | – |
| vello_gpu | 3.701 / 4.660 | encode (Scene): 0.008 / 0.013<br>prepare (Renderer::render): 0.042 / 0.082<br>submit + GPU completion: 1.662 / 1.884<br>read-back: 1.847 / 2.128 | – | – |


## Scale 2 (2560×1600 physical)

Empty scene, render + read-back median: pert-ui wgpu 6.899 ms, vello_gpu 4.789 ms.

### Panels (150 rounded + 100 plain) — scale 2

pert-ui command mix: 1151 FillRect, 1200 DrawMask (10 mask resources, 6 KiB), 0 DrawImage, 0 DrawVector.

| Renderer | Frame total median / p95 ms | Phases median / p95 ms | Not in total: update median ms | Cold compose ms |
|---|---|---|---|---|
| pert-ui CPU | 16.164 / 30.596 | compose (paint): 0.185 / 0.299<br>CpuRenderer::render: 15.975 / 30.324 | 0.006 | 0.267 |
| pert-ui wgpu | 9.213 / 10.101 | compose (paint): 0.184 / 0.197<br>render_offscreen (prepare + GPU + read-back): 9.025 / 9.914 | 0.016 | 0.270 |
| vello_cpu 1 thread | 2.677 / 2.870 | encode (+flush): 1.456 / 1.481<br>render: 1.224 / 1.396 | – | – |
| vello_cpu 5 workers | 0.989 / 1.052 | encode (+flush): 0.484 / 0.498<br>render: 0.504 / 0.566 | – | – |
| vello_gpu | 13.140 / 18.173 | encode (Scene): 3.287 / 3.689<br>prepare (Renderer::render): 0.638 / 0.711<br>submit + GPU completion: 4.216 / 7.282<br>read-back: 4.993 / 7.378 | – | – |

### Panels light (20 rounded + 20 plain) — scale 2

pert-ui command mix: 161 FillRect, 160 DrawMask (10 mask resources, 6 KiB), 0 DrawImage, 0 DrawVector.

| Renderer | Frame total median / p95 ms | Phases median / p95 ms | Not in total: update median ms | Cold compose ms |
|---|---|---|---|---|
| pert-ui CPU | 13.568 / 14.170 | compose (paint): 0.041 / 0.046<br>CpuRenderer::render: 13.527 / 14.123 | 0.005 | 0.198 |
| pert-ui wgpu | 14.468 / 16.627 | compose (paint): 0.062 / 0.078<br>render_offscreen (prepare + GPU + read-back): 14.406 / 16.553 | 0.020 | 0.166 |
| vello_cpu 1 thread | 3.296 / 3.455 | encode (+flush): 0.878 / 0.910<br>render: 2.414 / 2.570 | – | – |
| vello_cpu 5 workers | 6.293 / 9.168 | encode (+flush): 3.796 / 6.655<br>render: 2.415 / 2.827 | – | – |
| vello_gpu | 12.009 / 16.728 | encode (Scene): 0.937 / 1.022<br>prepare (Renderer::render): 0.427 / 0.469<br>submit + GPU completion: 4.241 / 6.859<br>read-back: 6.356 / 8.562 | – | – |

### Polyline 800 points — scale 2

pert-ui command mix: 1 FillRect, 0 DrawMask (0 mask resources, 0 KiB), 0 DrawImage, 1 DrawVector.

| Renderer | Frame total median / p95 ms | Phases median / p95 ms | Not in total: update median ms | Cold compose ms |
|---|---|---|---|---|
| pert-ui CPU | 42.241 / 44.724 | compose (paint): 0.032 / 0.036<br>CpuRenderer::render: 42.210 / 44.690 | 0.021 | 0.063 |
| pert-ui wgpu | 25.647 / 33.781 | compose (paint): 0.058 / 0.073<br>render_offscreen (prepare + GPU + read-back): 25.590 / 33.709 | 0.053 | 0.030 |
| vello_cpu 1 thread | 1.852 / 2.042 | encode (+flush): 0.877 / 0.925<br>render: 0.975 / 1.123 | – | – |
| vello_cpu 5 workers | 1.718 / 1.895 | encode (+flush): 1.276 / 1.385<br>render: 0.427 / 0.522 | – | – |
| vello_gpu | 9.012 / 10.955 | encode (Scene): 0.870 / 0.922<br>prepare (Renderer::render): 0.186 / 0.213<br>submit + GPU completion: 3.120 / 3.985<br>read-back: 4.797 / 6.352 | – | – |

### Polyline 4096 points — scale 2

pert-ui command mix: 1 FillRect, 0 DrawMask (0 mask resources, 0 KiB), 0 DrawImage, 1 DrawVector.

| Renderer | Frame total median / p95 ms | Phases median / p95 ms | Not in total: update median ms | Cold compose ms |
|---|---|---|---|---|
| pert-ui CPU | 44.350 / 46.260 | compose (paint): 0.107 / 0.112<br>CpuRenderer::render: 44.243 / 46.151 | 0.060 | 0.120 |
| pert-ui wgpu | 16.990 / 18.732 | compose (paint): 0.112 / 0.121<br>render_offscreen (prepare + GPU + read-back): 16.877 / 18.620 | 0.070 | 0.102 |
| vello_cpu 1 thread | 2.104 / 2.193 | encode (+flush): 1.198 / 1.217<br>render: 0.924 / 0.982 | – | – |
| vello_cpu 5 workers | 1.624 / 2.118 | encode (+flush): 1.269 / 1.703<br>render: 0.347 / 0.442 | – | – |
| vello_gpu | 9.111 / 10.551 | encode (Scene): 1.268 / 1.332<br>prepare (Renderer::render): 0.177 / 0.201<br>submit + GPU completion: 3.092 / 3.752<br>read-back: 4.553 / 5.512 | – | – |

### Gradient bars (32) — scale 2

pert-ui command mix: 33 FillRect, 0 DrawMask (0 mask resources, 0 KiB), 32 DrawImage, 0 DrawVector.

| Renderer | Frame total median / p95 ms | Phases median / p95 ms | Not in total: update median ms | Cold compose ms |
|---|---|---|---|---|
| pert-ui CPU | 45.372 / 46.282 | compose (paint): 0.011 / 0.013<br>CpuRenderer::render: 45.360 / 46.270 | 0.006 | 0.014 |
| pert-ui wgpu | 8.483 / 8.868 | compose (paint): 0.015 / 0.017<br>render_offscreen (prepare + GPU + read-back): 8.466 / 8.853 | 0.017 | 0.010 |
| vello_cpu 1 thread | 2.215 / 2.357 | encode (+flush): 0.051 / 0.055<br>render: 2.165 / 2.303 | – | – |
| vello_cpu 5 workers | 0.935 / 0.976 | encode (+flush): 0.082 / 0.096<br>render: 0.852 / 0.886 | – | – |
| vello_gpu | 8.085 / 9.713 | encode (Scene): 0.014 / 0.015<br>prepare (Renderer::render): 0.127 / 0.144<br>submit + GPU completion: 3.166 / 3.984<br>read-back: 4.716 / 5.646 | – | – |


## Parity of the last frame (reference: pert-ui CPU)

Channel differences over R, G, B in 1/255. *Interior* = pixels whose 3×3 neighborhood in the reference is one color; *edge* = all others (antialiased edges, the line, gradient rows). Cells: max / mean / share of pixels > 2.

| Workload | Scale | Renderer | All | Interior | Edge (pixels) |
|---|---|---|---|---|---|
| Panels (150 rounded + 100 plain) | 1 | pert-ui wgpu | 1 / 0.001 / 0.00% | 0 / 0.000 / 0.00% | 1 / 0.008 / 0.00% (122466) |
| Panels (150 rounded + 100 plain) | 1 | vello_cpu 1 thread | 69 / 0.147 / 1.03% | 0 / 0.000 / 0.00% | 69 / 1.231 / 8.59% (122466) |
| Panels (150 rounded + 100 plain) | 1 | vello_cpu 5 workers | 69 / 0.147 / 1.03% | 0 / 0.000 / 0.00% | 69 / 1.231 / 8.59% (122466) |
| Panels (150 rounded + 100 plain) | 1 | vello_gpu | 70 / 0.148 / 1.02% | 0 / 0.000 / 0.00% | 70 / 1.242 / 8.50% (122466) |
| Panels (150 rounded + 100 plain) | 1 | vello_gpu vs vello_cpu 1 thread | 2 / 0.005 / 0.00% | 0 / 0.000 / 0.00% | 2 / 0.040 / 0.00% (123246) |
| Panels light (20 rounded + 20 plain) | 1 | pert-ui wgpu | 1 / 0.000 / 0.00% | 0 / 0.000 / 0.00% | 1 / 0.003 / 0.00% (46472) |
| Panels light (20 rounded + 20 plain) | 1 | vello_cpu 1 thread | 69 / 0.020 / 0.14% | 0 / 0.000 / 0.00% | 69 / 0.434 / 3.11% (46472) |
| Panels light (20 rounded + 20 plain) | 1 | vello_cpu 5 workers | 69 / 0.020 / 0.14% | 0 / 0.000 / 0.00% | 69 / 0.434 / 3.11% (46472) |
| Panels light (20 rounded + 20 plain) | 1 | vello_gpu | 69 / 0.020 / 0.14% | 0 / 0.000 / 0.00% | 69 / 0.439 / 3.12% (46472) |
| Panels light (20 rounded + 20 plain) | 1 | vello_gpu vs vello_cpu 1 thread | 2 / 0.001 / 0.00% | 0 / 0.000 / 0.00% | 2 / 0.014 / 0.00% (46548) |
| Polyline 800 points | 1 | pert-ui wgpu | 1 / 0.003 / 0.00% | 0 / 0.000 / 0.00% | 1 / 0.062 / 0.00% (57143) |
| Polyline 800 points | 1 | vello_cpu 1 thread | 134 / 0.955 / 3.44% | 0 / 0.000 / 0.00% | 134 / 17.110 / 61.58% (57143) |
| Polyline 800 points | 1 | vello_cpu 5 workers | 134 / 0.955 / 3.44% | 0 / 0.000 / 0.00% | 134 / 17.110 / 61.58% (57143) |
| Polyline 800 points | 1 | vello_gpu | 134 / 0.957 / 3.43% | 0 / 0.000 / 0.00% | 134 / 17.154 / 61.46% (57143) |
| Polyline 800 points | 1 | vello_gpu vs vello_cpu 1 thread | 1 / 0.006 / 0.00% | 0 / 0.000 / 0.00% | 1 / 0.114 / 0.00% (49669) |
| Polyline 800 points | 1 | vello_cpu coverage composited in linear light (post hoc) | 130 / 0.654 / 3.34% | 0 / 0.000 / 0.00% | 130 / 11.721 / 59.84% (57143) |
| Polyline 4096 points | 1 | pert-ui wgpu | 1 / 0.006 / 0.00% | 0 / 0.000 / 0.00% | 1 / 0.097 / 0.00% (60787) |
| Polyline 4096 points | 1 | vello_cpu 1 thread | 145 / 1.004 / 3.70% | 0 / 0.000 / 0.00% | 145 / 16.916 / 62.41% (60787) |
| Polyline 4096 points | 1 | vello_cpu 5 workers | 145 / 1.004 / 3.70% | 0 / 0.000 / 0.00% | 145 / 16.916 / 62.41% (60787) |
| Polyline 4096 points | 1 | vello_gpu | 146 / 1.007 / 3.70% | 0 / 0.000 / 0.00% | 146 / 16.960 / 62.27% (60787) |
| Polyline 4096 points | 1 | vello_gpu vs vello_cpu 1 thread | 1 / 0.006 / 0.00% | 0 / 0.000 / 0.00% | 1 / 0.117 / 0.00% (53174) |
| Polyline 4096 points | 1 | vello_cpu coverage composited in linear light (post hoc) | 133 / 0.666 / 3.57% | 0 / 0.000 / 0.00% | 133 / 11.213 / 60.16% (60787) |
| Gradient bars (32) | 1 | pert-ui wgpu | 0 / 0.000 / 0.00% | 0 / 0.000 / 0.00% | 0 / 0.000 / 0.00% (291774) |
| Gradient bars (32) | 1 | vello_cpu 1 thread | 1 / 0.031 / 0.00% | 0 / 0.000 / 0.00% | 1 / 0.110 / 0.00% (291774) |
| Gradient bars (32) | 1 | vello_cpu 5 workers | 1 / 0.031 / 0.00% | 0 / 0.000 / 0.00% | 1 / 0.110 / 0.00% (291774) |
| Gradient bars (32) | 1 | vello_gpu | 1 / 0.031 / 0.00% | 0 / 0.000 / 0.00% | 1 / 0.110 / 0.00% (291774) |
| Gradient bars (32) | 1 | vello_gpu vs vello_cpu 1 thread | 1 / 0.000 / 0.00% | 0 / 0.000 / 0.00% | 1 / 0.000 / 0.00% (274152) |
| Panels (150 rounded + 100 plain) | 2 | pert-ui wgpu | 1 / 0.001 / 0.00% | 0 / 0.000 / 0.00% | 1 / 0.007 / 0.00% (310664) |
| Panels (150 rounded + 100 plain) | 2 | vello_cpu 1 thread | 77 / 0.080 / 0.55% | 0 / 0.000 / 0.00% | 77 / 1.052 / 7.27% (310664) |
| Panels (150 rounded + 100 plain) | 2 | vello_cpu 5 workers | 77 / 0.080 / 0.55% | 0 / 0.000 / 0.00% | 77 / 1.052 / 7.27% (310664) |
| Panels (150 rounded + 100 plain) | 2 | vello_gpu | 78 / 0.081 / 0.55% | 0 / 0.000 / 0.00% | 78 / 1.062 / 7.25% (310664) |
| Panels (150 rounded + 100 plain) | 2 | vello_gpu vs vello_cpu 1 thread | 2 / 0.002 / 0.00% | 0 / 0.000 / 0.00% | 2 / 0.031 / 0.00% (312128) |
| Panels light (20 rounded + 20 plain) | 2 | pert-ui wgpu | 1 / 0.000 / 0.00% | 0 / 0.000 / 0.00% | 1 / 0.003 / 0.00% (116524) |
| Panels light (20 rounded + 20 plain) | 2 | vello_cpu 1 thread | 76 / 0.011 / 0.08% | 0 / 0.000 / 0.00% | 76 / 0.388 / 2.67% (116524) |
| Panels light (20 rounded + 20 plain) | 2 | vello_cpu 5 workers | 76 / 0.011 / 0.08% | 0 / 0.000 / 0.00% | 76 / 0.388 / 2.67% (116524) |
| Panels light (20 rounded + 20 plain) | 2 | vello_gpu | 76 / 0.011 / 0.08% | 0 / 0.000 / 0.00% | 76 / 0.391 / 2.66% (116524) |
| Panels light (20 rounded + 20 plain) | 2 | vello_gpu vs vello_cpu 1 thread | 2 / 0.000 / 0.00% | 0 / 0.000 / 0.00% | 2 / 0.011 / 0.00% (116700) |
| Polyline 800 points | 2 | pert-ui wgpu | 220 / 0.003 / 0.00% | 0 / 0.000 / 0.00% | 220 / 0.088 / 0.03% (157105) |
| ↳ | | largest pert-ui wgpu difference at physical (478, 610), frame 219: RGB pert-ui CPU [18, 22, 30], wgpu [98, 167, 250], vello_cpu [56, 89, 132] | | | |
| Polyline 800 points | 2 | vello_cpu 1 thread | 157 / 0.610 / 2.02% | 0 / 0.000 / 0.00% | 157 / 15.903 / 52.62% (157105) |
| Polyline 800 points | 2 | vello_cpu 5 workers | 157 / 0.610 / 2.02% | 0 / 0.000 / 0.00% | 157 / 15.903 / 52.62% (157105) |
| Polyline 800 points | 2 | vello_gpu | 157 / 0.612 / 2.01% | 0 / 0.000 / 0.00% | 157 / 15.943 / 52.52% (157105) |
| Polyline 800 points | 2 | vello_gpu vs vello_cpu 1 thread | 1 / 0.003 / 0.00% | 0 / 0.000 / 0.00% | 1 / 0.090 / 0.00% (134308) |
| Polyline 800 points | 2 | vello_cpu coverage composited in linear light (post hoc) | 155 / 0.449 / 1.98% | 0 / 0.000 / 0.00% | 155 / 11.714 / 51.70% (157105) |
| Polyline 4096 points | 2 | pert-ui wgpu | 1 / 0.002 / 0.00% | 0 / 0.000 / 0.00% | 1 / 0.048 / 0.00% (167191) |
| Polyline 4096 points | 2 | vello_cpu 1 thread | 156 / 0.642 / 2.15% | 0 / 0.000 / 0.00% | 156 / 15.729 / 52.71% (167191) |
| Polyline 4096 points | 2 | vello_cpu 5 workers | 156 / 0.642 / 2.15% | 0 / 0.000 / 0.00% | 156 / 15.729 / 52.71% (167191) |
| Polyline 4096 points | 2 | vello_gpu | 156 / 0.644 / 2.15% | 0 / 0.000 / 0.00% | 156 / 15.769 / 52.61% (167191) |
| Polyline 4096 points | 2 | vello_gpu vs vello_cpu 1 thread | 1 / 0.003 / 0.00% | 0 / 0.000 / 0.00% | 1 / 0.094 / 0.00% (144099) |
| Polyline 4096 points | 2 | vello_cpu coverage composited in linear light (post hoc) | 155 / 0.462 / 2.11% | 0 / 0.000 / 0.00% | 155 / 11.331 / 51.68% (167191) |
| Gradient bars (32) | 2 | pert-ui wgpu | 0 / 0.000 / 0.00% | 0 / 0.000 / 0.00% | 0 / 0.000 / 0.00% (725488) |
| Gradient bars (32) | 2 | vello_cpu 1 thread | 1 / 0.032 / 0.00% | 1 / 0.008 / 0.00% | 1 / 0.144 / 0.00% (725488) |
| Gradient bars (32) | 2 | vello_cpu 5 workers | 1 / 0.032 / 0.00% | 1 / 0.008 / 0.00% | 1 / 0.144 / 0.00% (725488) |
| Gradient bars (32) | 2 | vello_gpu | 1 / 0.032 / 0.00% | 1 / 0.008 / 0.00% | 1 / 0.143 / 0.00% (725488) |
| Gradient bars (32) | 2 | vello_gpu vs vello_cpu 1 thread | 1 / 0.000 / 0.00% | 0 / 0.000 / 0.00% | 1 / 0.001 / 0.00% (714080) |

## Part 2: pert-ui DisplayList translated into vello_cpu

Per frame: pert-ui compose (not in the totals), translation (walking the commands, issuing vello calls, converting and registering resources not seen before; with `flush`) and vello_cpu render. Parity against pert-ui CPU rendering the same DisplayList. Cells: median / p95 ms; parity max / mean / share > 2 over all, interior, edge.

| Workload | Scale | Threads | Translate | Render | Total | First-frame translate ms (new resources, KiB) | Parity all | Interior | Edge |
|---|---|---|---|---|---|---|---|---|---|
| Panels (150 rounded + 100 plain) | 1 | 1 | 0.349 / 0.359 | 0.816 / 0.836 | 1.164 / 1.195 | 0.804 (10, 6) | 42 / 0.097 / 0.89% | 0 / 0.000 / 0.00% | 42 / 0.808 / 7.41% |
| Panels (150 rounded + 100 plain) | 1 | 5 workers | 0.330 / 0.339 | 0.411 / 0.454 | 0.741 / 0.764 | 1.072 (10, 6) | 42 / 0.097 / 0.89% | 0 / 0.000 / 0.00% | 42 / 0.808 / 7.41% |
| Panels light (20 rounded + 20 plain) | 1 | 1 | 0.055 / 0.058 | 0.357 / 0.366 | 0.413 / 0.423 | 0.376 (10, 6) | 41 / 0.014 / 0.12% | 0 / 0.000 / 0.00% | 41 / 0.305 / 2.75% |
| Panels light (20 rounded + 20 plain) | 1 | 5 workers | 0.074 / 0.079 | 0.176 / 0.206 | 0.251 / 0.279 | 0.466 (10, 6) | 41 / 0.014 / 0.12% | 0 / 0.000 / 0.00% | 41 / 0.305 / 2.75% |
| Polyline 800 points | 1 | 1 | 0.437 / 0.449 | 0.197 / 0.203 | 0.634 / 0.650 | 0.718 (0, 0) | 134 / 0.955 / 3.44% | 0 / 0.000 / 0.00% | 134 / 17.110 / 61.58% |
| Polyline 800 points | 1 | 5 workers | 0.467 / 0.635 | 0.117 / 0.144 | 0.582 / 0.776 | 1.105 (0, 0) | 134 / 0.955 / 3.44% | 0 / 0.000 / 0.00% | 134 / 17.110 / 61.58% |
| Polyline 4096 points | 1 | 1 | 0.827 / 1.226 | 0.202 / 0.363 | 1.027 / 1.577 | 1.202 (0, 0) | 145 / 1.004 / 3.70% | 0 / 0.000 / 0.00% | 145 / 16.916 / 62.41% |
| Polyline 4096 points | 1 | 5 workers | 0.889 / 1.169 | 0.119 / 0.138 | 1.007 / 1.306 | 1.419 (0, 0) | 145 / 1.004 / 3.70% | 0 / 0.000 / 0.00% | 145 / 16.916 / 62.41% |
| Gradient bars (32) | 1 | 1 | 0.027 / 0.030 | 1.433 / 1.461 | 1.461 / 1.489 | 0.351 (1, 2) | 0 / 0.000 / 0.00% | 0 / 0.000 / 0.00% | 0 / 0.000 / 0.00% |
| Gradient bars (32) | 1 | 5 workers | 0.057 / 0.065 | 0.617 / 0.632 | 0.674 / 0.693 | 0.573 (1, 2) | 0 / 0.000 / 0.00% | 0 / 0.000 / 0.00% | 0 / 0.000 / 0.00% |
| Panels (150 rounded + 100 plain) | 2 | 1 | 0.375 / 0.410 | 2.168 / 2.470 | 2.543 / 2.864 | 1.177 (10, 24) | 43 / 0.051 / 0.46% | 0 / 0.000 / 0.00% | 43 / 0.675 / 6.06% |
| Panels (150 rounded + 100 plain) | 2 | 5 workers | 0.326 / 0.424 | 0.992 / 1.078 | 1.325 / 1.442 | 1.711 (10, 24) | 43 / 0.051 / 0.46% | 0 / 0.000 / 0.00% | 43 / 0.675 / 6.06% |
| Panels light (20 rounded + 20 plain) | 2 | 1 | 0.067 / 0.071 | 1.363 / 1.508 | 1.431 / 1.578 | 1.061 (10, 24) | 41 / 0.007 / 0.07% | 0 / 0.000 / 0.00% | 41 / 0.248 / 2.29% |
| Panels light (20 rounded + 20 plain) | 2 | 5 workers | 0.101 / 0.112 | 0.503 / 0.588 | 0.605 / 0.689 | 1.384 (10, 24) | 41 / 0.007 / 0.07% | 0 / 0.000 / 0.00% | 41 / 0.248 / 2.29% |
| Polyline 800 points | 2 | 1 | 0.799 / 0.824 | 0.849 / 0.945 | 1.647 / 1.749 | 1.886 (0, 0) | 157 / 0.610 / 2.02% | 0 / 0.000 / 0.00% | 157 / 15.903 / 52.62% |
| Polyline 800 points | 2 | 5 workers | 0.866 / 1.147 | 0.350 / 0.388 | 1.219 / 1.522 | 2.090 (0, 0) | 157 / 0.610 / 2.02% | 0 / 0.000 / 0.00% | 157 / 15.903 / 52.62% |
| Polyline 4096 points | 2 | 1 | 1.223 / 1.255 | 0.958 / 1.051 | 2.181 / 2.308 | 2.355 (0, 0) | 156 / 0.642 / 2.15% | 0 / 0.000 / 0.00% | 156 / 15.729 / 52.71% |
| Polyline 4096 points | 2 | 5 workers | 1.308 / 1.751 | 0.335 / 0.436 | 1.644 / 2.103 | 3.170 (0, 0) | 156 / 0.642 / 2.15% | 0 / 0.000 / 0.00% | 156 / 15.729 / 52.71% |
| Gradient bars (32) | 2 | 1 | 0.050 / 0.056 | 4.808 / 5.117 | 4.859 / 5.173 | 1.061 (1, 5) | 0 / 0.000 / 0.00% | 0 / 0.000 / 0.00% | 0 / 0.000 / 0.00% |
| Gradient bars (32) | 2 | 5 workers | 0.182 / 0.219 | 2.787 / 2.985 | 2.963 / 3.158 | 1.259 (1, 5) | 0 / 0.000 / 0.00% | 0 / 0.000 / 0.00% | 0 / 0.000 / 0.00% |
<!-- load average after: 6.65 9.23 8.42 -->
