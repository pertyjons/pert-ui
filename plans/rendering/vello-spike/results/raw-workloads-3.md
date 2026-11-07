<!-- load average before: 4.31 7.76 7.98 -->
# vello spike measurements

vello_gpu adapter: AdapterInfo { name: "Intel(R) UHD Graphics 770 (ADL-S GT1)", vendor: 32902, device: 18048, device_type: IntegratedGpu, device_pci_bus_id: "0000:00:02.0", driver: "Intel open-source Mesa driver", driver_info: "Mesa 26.2.6", backend: Vulkan, subgroup_min_size: 8, subgroup_max_size: 32, transient_saves_memory: Some(false), limit_bucket: None }

200 measured frames after 20 warm-up frames per renderer, workload and scale. vello_cpu multithreaded uses 5 worker threads (available_parallelism − 1, capped at 8, vello's default). Times in ms, median / p95.

## Scale 1 (1280×800 physical)

Empty scene, render + read-back median: pert-ui wgpu 3.060 ms, vello_gpu 2.700 ms.

### Panels (150 rounded + 100 plain) — scale 1

pert-ui command mix: 1151 FillRect, 1200 DrawMask (10 mask resources, 1 KiB), 0 DrawImage, 0 DrawVector.

| Renderer | Frame total median / p95 ms | Phases median / p95 ms | Not in total: update median ms | Cold compose ms |
|---|---|---|---|---|
| pert-ui CPU | 4.180 / 4.219 | compose (paint): 0.143 / 0.149<br>CpuRenderer::render: 4.035 / 4.076 | 0.001 | 0.189 |
| pert-ui wgpu | 4.854 / 5.203 | compose (paint): 0.151 / 0.161<br>render_offscreen (prepare + GPU + read-back): 4.706 / 5.039 | 0.006 | 0.184 |
| vello_cpu 1 thread | 1.438 / 1.493 | encode (+flush): 1.045 / 1.067<br>render: 0.393 / 0.426 | – | – |
| vello_cpu 5 workers | 0.542 / 0.563 | encode (+flush): 0.345 / 0.360<br>render: 0.196 / 0.210 | – | – |
| vello_gpu | 4.254 / 6.496 | encode (Scene): 1.083 / 1.101<br>prepare (Renderer::render): 0.158 / 0.221<br>submit + GPU completion: 1.654 / 3.226<br>read-back: 1.331 / 2.277 | – | – |

### Panels light (20 rounded + 20 plain) — scale 1

pert-ui command mix: 161 FillRect, 160 DrawMask (10 mask resources, 1 KiB), 0 DrawImage, 0 DrawVector.

| Renderer | Frame total median / p95 ms | Phases median / p95 ms | Not in total: update median ms | Cold compose ms |
|---|---|---|---|---|
| pert-ui CPU | 2.576 / 2.615 | compose (paint): 0.023 / 0.026<br>CpuRenderer::render: 2.553 / 2.592 | 0.001 | 0.068 |
| pert-ui wgpu | 3.983 / 5.284 | compose (paint): 0.031 / 0.038<br>render_offscreen (prepare + GPU + read-back): 3.951 / 5.250 | 0.008 | 0.059 |
| vello_cpu 1 thread | 0.477 / 0.481 | encode (+flush): 0.201 / 0.204<br>render: 0.275 / 0.280 | – | – |
| vello_cpu 5 workers | 0.220 / 0.235 | encode (+flush): 0.099 / 0.106<br>render: 0.120 / 0.137 | – | – |
| vello_gpu | 4.425 / 5.316 | encode (Scene): 0.220 / 0.243<br>prepare (Renderer::render): 0.086 / 0.116<br>submit + GPU completion: 2.171 / 2.600<br>read-back: 1.953 / 2.389 | – | – |

### Polyline 800 points — scale 1

pert-ui command mix: 1 FillRect, 0 DrawMask (0 mask resources, 0 KiB), 0 DrawImage, 1 DrawVector.

| Renderer | Frame total median / p95 ms | Phases median / p95 ms | Not in total: update median ms | Cold compose ms |
|---|---|---|---|---|
| pert-ui CPU | 12.197 / 12.486 | compose (paint): 0.027 / 0.031<br>CpuRenderer::render: 12.171 / 12.459 | 0.019 | 0.030 |
| pert-ui wgpu | 6.090 / 7.330 | compose (paint): 0.027 / 0.032<br>render_offscreen (prepare + GPU + read-back): 6.064 / 7.301 | 0.022 | 0.028 |
| vello_cpu 1 thread | 0.600 / 0.606 | encode (+flush): 0.414 / 0.420<br>render: 0.186 / 0.189 | – | – |
| vello_cpu 5 workers | 0.582 / 0.767 | encode (+flush): 0.465 / 0.627<br>render: 0.117 / 0.144 | – | – |
| vello_gpu | 3.558 / 6.524 | encode (Scene): 0.432 / 0.448<br>prepare (Renderer::render): 0.063 / 0.100<br>submit + GPU completion: 1.418 / 2.226<br>read-back: 1.600 / 3.084 | – | – |

### Polyline 4096 points — scale 1

pert-ui command mix: 1 FillRect, 0 DrawMask (0 mask resources, 0 KiB), 0 DrawImage, 1 DrawVector.

| Renderer | Frame total median / p95 ms | Phases median / p95 ms | Not in total: update median ms | Cold compose ms |
|---|---|---|---|---|
| pert-ui CPU | 13.384 / 13.646 | compose (paint): 0.104 / 0.110<br>CpuRenderer::render: 13.281 / 13.536 | 0.058 | 0.107 |
| pert-ui wgpu | 6.844 / 8.386 | compose (paint): 0.104 / 0.117<br>render_offscreen (prepare + GPU + read-back): 6.739 / 8.284 | 0.063 | 0.120 |
| vello_cpu 1 thread | 0.987 / 0.997 | encode (+flush): 0.800 / 0.809<br>render: 0.187 / 0.190 | – | – |
| vello_cpu 5 workers | 1.000 / 1.301 | encode (+flush): 0.880 / 1.157<br>render: 0.118 / 0.143 | – | – |
| vello_gpu | 3.423 / 5.292 | encode (Scene): 0.823 / 0.852<br>prepare (Renderer::render): 0.058 / 0.091<br>submit + GPU completion: 1.180 / 2.136<br>read-back: 1.333 / 2.245 | – | – |

### Gradient bars (32) — scale 1

pert-ui command mix: 33 FillRect, 0 DrawMask (0 mask resources, 0 KiB), 32 DrawImage, 0 DrawVector.

| Renderer | Frame total median / p95 ms | Phases median / p95 ms | Not in total: update median ms | Cold compose ms |
|---|---|---|---|---|
| pert-ui CPU | 11.277 / 11.426 | compose (paint): 0.008 / 0.010<br>CpuRenderer::render: 11.271 / 11.418 | 0.003 | 0.012 |
| pert-ui wgpu | 4.123 / 5.583 | compose (paint): 0.010 / 0.015<br>render_offscreen (prepare + GPU + read-back): 4.112 / 5.568 | 0.008 | 0.009 |
| vello_cpu 1 thread | 0.689 / 0.721 | encode (+flush): 0.029 / 0.030<br>render: 0.660 / 0.691 | – | – |
| vello_cpu 5 workers | 0.386 / 0.412 | encode (+flush): 0.054 / 0.059<br>render: 0.332 / 0.353 | – | – |
| vello_gpu | 3.829 / 4.705 | encode (Scene): 0.008 / 0.014<br>prepare (Renderer::render): 0.041 / 0.106<br>submit + GPU completion: 1.783 / 2.170<br>read-back: 2.011 / 2.444 | – | – |


## Scale 2 (2560×1600 physical)

Empty scene, render + read-back median: pert-ui wgpu 6.945 ms, vello_gpu 4.838 ms.

### Panels (150 rounded + 100 plain) — scale 2

pert-ui command mix: 1151 FillRect, 1200 DrawMask (10 mask resources, 6 KiB), 0 DrawImage, 0 DrawVector.

| Renderer | Frame total median / p95 ms | Phases median / p95 ms | Not in total: update median ms | Cold compose ms |
|---|---|---|---|---|
| pert-ui CPU | 15.303 / 15.556 | compose (paint): 0.172 / 0.180<br>CpuRenderer::render: 15.131 / 15.381 | 0.004 | 0.267 |
| pert-ui wgpu | 9.214 / 9.992 | compose (paint): 0.180 / 0.188<br>render_offscreen (prepare + GPU + read-back): 9.035 / 9.805 | 0.016 | 0.261 |
| vello_cpu 1 thread | 2.764 / 2.802 | encode (+flush): 1.430 / 1.448<br>render: 1.336 / 1.364 | – | – |
| vello_cpu 5 workers | 0.964 / 1.031 | encode (+flush): 0.471 / 0.483<br>render: 0.492 / 0.555 | – | – |
| vello_gpu | 10.457 / 12.110 | encode (Scene): 1.490 / 1.562<br>prepare (Renderer::render): 0.320 / 0.369<br>submit + GPU completion: 4.110 / 5.070<br>read-back: 4.482 / 5.551 | – | – |

### Panels light (20 rounded + 20 plain) — scale 2

pert-ui command mix: 161 FillRect, 160 DrawMask (10 mask resources, 6 KiB), 0 DrawImage, 0 DrawVector.

| Renderer | Frame total median / p95 ms | Phases median / p95 ms | Not in total: update median ms | Cold compose ms |
|---|---|---|---|---|
| pert-ui CPU | 10.287 / 10.467 | compose (paint): 0.025 / 0.032<br>CpuRenderer::render: 10.259 / 10.440 | 0.002 | 0.149 |
| pert-ui wgpu | 8.369 / 8.957 | compose (paint): 0.038 / 0.043<br>render_offscreen (prepare + GPU + read-back): 8.329 / 8.920 | 0.014 | 0.115 |
| vello_cpu 1 thread | 1.379 / 1.990 | encode (+flush): 0.322 / 0.531<br>render: 1.060 / 1.465 | – | – |
| vello_cpu 5 workers | 0.518 / 0.570 | encode (+flush): 0.151 / 0.159<br>render: 0.366 / 0.420 | – | – |
| vello_gpu | 8.840 / 11.604 | encode (Scene): 0.364 / 0.407<br>prepare (Renderer::render): 0.163 / 0.200<br>submit + GPU completion: 3.582 / 5.041<br>read-back: 4.599 / 6.003 | – | – |

### Polyline 800 points — scale 2

pert-ui command mix: 1 FillRect, 0 DrawMask (0 mask resources, 0 KiB), 0 DrawImage, 1 DrawVector.

| Renderer | Frame total median / p95 ms | Phases median / p95 ms | Not in total: update median ms | Cold compose ms |
|---|---|---|---|---|
| pert-ui CPU | 42.171 / 42.685 | compose (paint): 0.030 / 0.034<br>CpuRenderer::render: 42.143 / 42.646 | 0.019 | 0.037 |
| pert-ui wgpu | 15.444 / 15.862 | compose (paint): 0.034 / 0.038<br>render_offscreen (prepare + GPU + read-back): 15.409 / 15.828 | 0.030 | 0.030 |
| vello_cpu 1 thread | 1.560 / 1.600 | encode (+flush): 0.749 / 0.763<br>render: 0.811 / 0.845 | – | – |
| vello_cpu 5 workers | 1.136 / 1.427 | encode (+flush): 0.796 / 1.061<br>render: 0.329 / 0.379 | – | – |
| vello_gpu | 9.851 / 12.000 | encode (Scene): 0.796 / 0.831<br>prepare (Renderer::render): 0.141 / 0.172<br>submit + GPU completion: 3.770 / 4.790<br>read-back: 5.130 / 6.370 | – | – |

### Polyline 4096 points — scale 2

pert-ui command mix: 1 FillRect, 0 DrawMask (0 mask resources, 0 KiB), 0 DrawImage, 1 DrawVector.

| Renderer | Frame total median / p95 ms | Phases median / p95 ms | Not in total: update median ms | Cold compose ms |
|---|---|---|---|---|
| pert-ui CPU | 44.192 / 44.713 | compose (paint): 0.107 / 0.112<br>CpuRenderer::render: 44.085 / 44.601 | 0.059 | 0.132 |
| pert-ui wgpu | 17.048 / 17.531 | compose (paint): 0.112 / 0.117<br>render_offscreen (prepare + GPU + read-back): 16.936 / 17.418 | 0.068 | 0.107 |
| vello_cpu 1 thread | 2.028 / 2.164 | encode (+flush): 1.174 / 1.205<br>render: 0.840 / 0.967 | – | – |
| vello_cpu 5 workers | 1.566 / 2.034 | encode (+flush): 1.247 / 1.683<br>render: 0.318 / 0.366 | – | – |
| vello_gpu | 9.337 / 11.216 | encode (Scene): 1.248 / 1.307<br>prepare (Renderer::render): 0.144 / 0.183<br>submit + GPU completion: 3.310 / 4.290<br>read-back: 4.637 / 5.625 | – | – |

### Gradient bars (32) — scale 2

pert-ui command mix: 33 FillRect, 0 DrawMask (0 mask resources, 0 KiB), 32 DrawImage, 0 DrawVector.

| Renderer | Frame total median / p95 ms | Phases median / p95 ms | Not in total: update median ms | Cold compose ms |
|---|---|---|---|---|
| pert-ui CPU | 45.584 / 46.323 | compose (paint): 0.010 / 0.013<br>CpuRenderer::render: 45.574 / 46.312 | 0.005 | 0.013 |
| pert-ui wgpu | 8.608 / 8.991 | compose (paint): 0.015 / 0.017<br>render_offscreen (prepare + GPU + read-back): 8.593 / 8.976 | 0.015 | 0.008 |
| vello_cpu 1 thread | 2.008 / 2.091 | encode (+flush): 0.050 / 0.052<br>render: 1.958 / 2.042 | – | – |
| vello_cpu 5 workers | 0.948 / 0.996 | encode (+flush): 0.081 / 0.102<br>render: 0.864 / 0.899 | – | – |
| vello_gpu | 8.983 / 11.107 | encode (Scene): 0.012 / 0.014<br>prepare (Renderer::render): 0.094 / 0.119<br>submit + GPU completion: 3.699 / 4.858<br>read-back: 5.165 / 6.542 | – | – |


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
| Panels (150 rounded + 100 plain) | 1 | 1 | 0.347 / 0.365 | 0.836 / 0.869 | 1.184 / 1.226 | 1.461 (10, 6) | 42 / 0.097 / 0.89% | 0 / 0.000 / 0.00% | 42 / 0.808 / 7.41% |
| Panels (150 rounded + 100 plain) | 1 | 5 workers | 0.325 / 0.345 | 0.408 / 0.458 | 0.731 / 0.791 | 1.024 (10, 6) | 42 / 0.097 / 0.89% | 0 / 0.000 / 0.00% | 42 / 0.808 / 7.41% |
| Panels light (20 rounded + 20 plain) | 1 | 1 | 0.055 / 0.058 | 0.388 / 0.406 | 0.443 / 0.464 | 0.246 (10, 6) | 41 / 0.014 / 0.12% | 0 / 0.000 / 0.00% | 41 / 0.305 / 2.75% |
| Panels light (20 rounded + 20 plain) | 1 | 5 workers | 0.073 / 0.080 | 0.177 / 0.187 | 0.251 / 0.265 | 0.626 (10, 6) | 41 / 0.014 / 0.12% | 0 / 0.000 / 0.00% | 41 / 0.305 / 2.75% |
| Polyline 800 points | 1 | 1 | 0.436 / 0.444 | 0.224 / 0.229 | 0.660 / 0.674 | 0.688 (0, 0) | 134 / 0.955 / 3.44% | 0 / 0.000 / 0.00% | 134 / 17.110 / 61.58% |
| Polyline 800 points | 1 | 5 workers | 0.465 / 0.633 | 0.117 / 0.148 | 0.582 / 0.778 | 0.782 (0, 0) | 134 / 0.955 / 3.44% | 0 / 0.000 / 0.00% | 134 / 17.110 / 61.58% |
| Polyline 4096 points | 1 | 1 | 0.819 / 0.826 | 0.225 / 0.228 | 1.045 / 1.053 | 1.090 (0, 0) | 145 / 1.004 / 3.70% | 0 / 0.000 / 0.00% | 145 / 16.916 / 62.41% |
| Polyline 4096 points | 1 | 5 workers | 0.890 / 1.170 | 0.117 / 0.140 | 1.007 / 1.311 | 1.843 (0, 0) | 145 / 1.004 / 3.70% | 0 / 0.000 / 0.00% | 145 / 16.916 / 62.41% |
| Gradient bars (32) | 1 | 1 | 0.027 / 0.029 | 1.424 / 1.460 | 1.451 / 1.488 | 0.224 (1, 2) | 0 / 0.000 / 0.00% | 0 / 0.000 / 0.00% | 0 / 0.000 / 0.00% |
| Gradient bars (32) | 1 | 5 workers | 0.055 / 0.062 | 1.004 / 1.101 | 1.060 / 1.158 | 0.434 (1, 2) | 0 / 0.000 / 0.00% | 0 / 0.000 / 0.00% | 0 / 0.000 / 0.00% |
| Panels (150 rounded + 100 plain) | 2 | 1 | 0.368 / 0.375 | 2.091 / 2.156 | 2.460 / 2.527 | 1.022 (10, 24) | 43 / 0.051 / 0.46% | 0 / 0.000 / 0.00% | 43 / 0.675 / 6.06% |
| Panels (150 rounded + 100 plain) | 2 | 5 workers | 0.317 / 0.346 | 0.984 / 1.124 | 1.299 / 1.445 | 1.807 (10, 24) | 43 / 0.051 / 0.46% | 0 / 0.000 / 0.00% | 43 / 0.675 / 6.06% |
| Panels light (20 rounded + 20 plain) | 2 | 1 | 0.065 / 0.073 | 1.252 / 1.343 | 1.317 / 1.410 | 1.009 (10, 24) | 41 / 0.007 / 0.07% | 0 / 0.000 / 0.00% | 41 / 0.248 / 2.29% |
| Panels light (20 rounded + 20 plain) | 2 | 5 workers | 0.098 / 0.116 | 0.513 / 0.594 | 0.612 / 0.695 | 1.376 (10, 24) | 41 / 0.007 / 0.07% | 0 / 0.000 / 0.00% | 41 / 0.248 / 2.29% |
| Polyline 800 points | 2 | 1 | 0.796 / 0.823 | 0.956 / 1.032 | 1.755 / 1.844 | 1.973 (0, 0) | 157 / 0.610 / 2.02% | 0 / 0.000 / 0.00% | 157 / 15.903 / 52.62% |
| Polyline 800 points | 2 | 5 workers | 0.862 / 1.153 | 0.338 / 0.372 | 1.199 / 1.511 | 2.250 (0, 0) | 157 / 0.610 / 2.02% | 0 / 0.000 / 0.00% | 157 / 15.903 / 52.62% |
| Polyline 4096 points | 2 | 1 | 1.219 / 1.253 | 0.955 / 1.050 | 2.176 / 2.327 | 2.385 (0, 0) | 156 / 0.642 / 2.15% | 0 / 0.000 / 0.00% | 156 / 15.729 / 52.71% |
| Polyline 4096 points | 2 | 5 workers | 1.301 / 1.747 | 0.343 / 0.384 | 1.645 / 2.154 | 3.032 (0, 0) | 156 / 0.642 / 2.15% | 0 / 0.000 / 0.00% | 156 / 15.729 / 52.71% |
| Gradient bars (32) | 2 | 1 | 0.047 / 0.056 | 4.856 / 5.089 | 4.903 / 5.141 | 0.996 (1, 5) | 0 / 0.000 / 0.00% | 0 / 0.000 / 0.00% | 0 / 0.000 / 0.00% |
| Gradient bars (32) | 2 | 5 workers | 0.170 / 0.207 | 2.643 / 2.944 | 2.819 / 3.107 | 1.341 (1, 5) | 0 / 0.000 / 0.00% | 0 / 0.000 / 0.00% | 0 / 0.000 / 0.00% |
<!-- load average after: 1.95 6.06 7.35 -->
