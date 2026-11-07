<!-- load average before: 6.65 9.23 8.42 -->
# vello spike measurements

vello_gpu adapter: AdapterInfo { name: "Intel(R) UHD Graphics 770 (ADL-S GT1)", vendor: 32902, device: 18048, device_type: IntegratedGpu, device_pci_bus_id: "0000:00:02.0", driver: "Intel open-source Mesa driver", driver_info: "Mesa 26.2.6", backend: Vulkan, subgroup_min_size: 8, subgroup_max_size: 32, transient_saves_memory: Some(false), limit_bucket: None }

200 measured frames after 20 warm-up frames per renderer, workload and scale. vello_cpu multithreaded uses 5 worker threads (available_parallelism − 1, capped at 8, vello's default). Times in ms, median / p95.

## Scale 1 (1280×800 physical)

Empty scene, render + read-back median: pert-ui wgpu 2.985 ms, vello_gpu 2.450 ms.

### Panels (150 rounded + 100 plain) — scale 1

pert-ui command mix: 1151 FillRect, 1200 DrawMask (10 mask resources, 1 KiB), 0 DrawImage, 0 DrawVector.

| Renderer | Frame total median / p95 ms | Phases median / p95 ms | Not in total: update median ms | Cold compose ms |
|---|---|---|---|---|
| pert-ui CPU | 4.108 / 4.173 | compose (paint): 0.150 / 0.154<br>CpuRenderer::render: 3.958 / 4.020 | 0.002 | 0.196 |
| pert-ui wgpu | 4.916 / 5.314 | compose (paint): 0.158 / 0.166<br>render_offscreen (prepare + GPU + read-back): 4.757 / 5.150 | 0.008 | 0.187 |
| vello_cpu 1 thread | 1.491 / 1.575 | encode (+flush): 1.070 / 1.124<br>render: 0.412 / 0.451 | – | – |
| vello_cpu 5 workers | 0.644 / 0.692 | encode (+flush): 0.421 / 0.459<br>render: 0.219 / 0.248 | – | – |
| vello_gpu | 4.290 / 6.700 | encode (Scene): 1.071 / 1.122<br>prepare (Renderer::render): 0.168 / 0.221<br>submit + GPU completion: 1.663 / 3.191<br>read-back: 1.352 / 2.546 | – | – |

### Panels light (20 rounded + 20 plain) — scale 1

pert-ui command mix: 161 FillRect, 160 DrawMask (10 mask resources, 1 KiB), 0 DrawImage, 0 DrawVector.

| Renderer | Frame total median / p95 ms | Phases median / p95 ms | Not in total: update median ms | Cold compose ms |
|---|---|---|---|---|
| pert-ui CPU | 2.428 / 2.502 | compose (paint): 0.024 / 0.030<br>CpuRenderer::render: 2.403 / 2.471 | 0.002 | 0.070 |
| pert-ui wgpu | 4.580 / 6.955 | compose (paint): 0.047 / 0.074<br>render_offscreen (prepare + GPU + read-back): 4.533 / 6.892 | 0.022 | 0.062 |
| vello_cpu 1 thread | 0.466 / 0.485 | encode (+flush): 0.210 / 0.223<br>render: 0.255 / 0.264 | – | – |
| vello_cpu 5 workers | 0.283 / 2.020 | encode (+flush): 0.133 / 1.869<br>render: 0.150 / 0.173 | – | – |
| vello_gpu | 4.336 / 6.554 | encode (Scene): 0.234 / 0.260<br>prepare (Renderer::render): 0.096 / 0.168<br>submit + GPU completion: 2.079 / 2.678<br>read-back: 1.947 / 2.525 | – | – |

### Polyline 800 points — scale 1

pert-ui command mix: 1 FillRect, 0 DrawMask (0 mask resources, 0 KiB), 0 DrawImage, 1 DrawVector.

| Renderer | Frame total median / p95 ms | Phases median / p95 ms | Not in total: update median ms | Cold compose ms |
|---|---|---|---|---|
| pert-ui CPU | 12.474 / 13.039 | compose (paint): 0.028 / 0.032<br>CpuRenderer::render: 12.445 / 13.006 | 0.020 | 0.031 |
| pert-ui wgpu | 5.677 / 7.290 | compose (paint): 0.030 / 0.035<br>render_offscreen (prepare + GPU + read-back): 5.645 / 7.257 | 0.027 | 0.028 |
| vello_cpu 1 thread | 0.638 / 0.644 | encode (+flush): 0.422 / 0.428<br>render: 0.216 / 0.219 | – | – |
| vello_cpu 5 workers | 0.571 / 0.754 | encode (+flush): 0.458 / 0.617<br>render: 0.113 / 0.140 | – | – |
| vello_gpu | 3.261 / 5.153 | encode (Scene): 0.436 / 0.456<br>prepare (Renderer::render): 0.065 / 0.099<br>submit + GPU completion: 1.230 / 2.198<br>read-back: 1.520 / 2.423 | – | – |

### Polyline 4096 points — scale 1

pert-ui command mix: 1 FillRect, 0 DrawMask (0 mask resources, 0 KiB), 0 DrawImage, 1 DrawVector.

| Renderer | Frame total median / p95 ms | Phases median / p95 ms | Not in total: update median ms | Cold compose ms |
|---|---|---|---|---|
| pert-ui CPU | 13.997 / 20.662 | compose (paint): 0.112 / 0.191<br>CpuRenderer::render: 13.886 / 20.533 | 0.062 | 0.104 |
| pert-ui wgpu | 7.105 / 8.590 | compose (paint): 0.109 / 0.116<br>render_offscreen (prepare + GPU + read-back): 6.994 / 8.478 | 0.066 | 0.110 |
| vello_cpu 1 thread | 1.015 / 1.035 | encode (+flush): 0.801 / 0.818<br>render: 0.213 / 0.220 | – | – |
| vello_cpu 5 workers | 0.981 / 1.287 | encode (+flush): 0.872 / 1.147<br>render: 0.114 / 0.141 | – | – |
| vello_gpu | 3.790 / 5.564 | encode (Scene): 0.844 / 0.891<br>prepare (Renderer::render): 0.074 / 0.122<br>submit + GPU completion: 1.306 / 2.220<br>read-back: 1.507 / 2.392 | – | – |

### Gradient bars (32) — scale 1

pert-ui command mix: 33 FillRect, 0 DrawMask (0 mask resources, 0 KiB), 32 DrawImage, 0 DrawVector.

| Renderer | Frame total median / p95 ms | Phases median / p95 ms | Not in total: update median ms | Cold compose ms |
|---|---|---|---|---|
| pert-ui CPU | 11.503 / 11.745 | compose (paint): 0.010 / 0.011<br>CpuRenderer::render: 11.493 / 11.735 | 0.005 | 0.012 |
| pert-ui wgpu | 4.112 / 4.247 | compose (paint): 0.010 / 0.013<br>render_offscreen (prepare + GPU + read-back): 4.103 / 4.237 | 0.007 | 0.010 |
| vello_cpu 1 thread | 0.719 / 0.732 | encode (+flush): 0.029 / 0.030<br>render: 0.690 / 0.702 | – | – |
| vello_cpu 5 workers | 0.399 / 0.423 | encode (+flush): 0.055 / 0.070<br>render: 0.338 / 0.363 | – | – |
| vello_gpu | 2.148 / 2.847 | encode (Scene): 0.007 / 0.009<br>prepare (Renderer::render): 0.027 / 0.053<br>submit + GPU completion: 0.972 / 1.291<br>read-back: 1.143 / 1.508 | – | – |


## Scale 2 (2560×1600 physical)

Empty scene, render + read-back median: pert-ui wgpu 7.130 ms, vello_gpu 4.780 ms.

### Panels (150 rounded + 100 plain) — scale 2

pert-ui command mix: 1151 FillRect, 1200 DrawMask (10 mask resources, 6 KiB), 0 DrawImage, 0 DrawVector.

| Renderer | Frame total median / p95 ms | Phases median / p95 ms | Not in total: update median ms | Cold compose ms |
|---|---|---|---|---|
| pert-ui CPU | 15.559 / 15.927 | compose (paint): 0.172 / 0.192<br>CpuRenderer::render: 15.382 / 15.743 | 0.005 | 0.307 |
| pert-ui wgpu | 9.200 / 9.938 | compose (paint): 0.172 / 0.182<br>render_offscreen (prepare + GPU + read-back): 9.026 / 9.769 | 0.016 | 0.256 |
| vello_cpu 1 thread | 2.651 / 2.833 | encode (+flush): 1.434 / 1.471<br>render: 1.219 / 1.367 | – | – |
| vello_cpu 5 workers | 0.974 / 1.057 | encode (+flush): 0.476 / 0.490<br>render: 0.496 / 0.578 | – | – |
| vello_gpu | 10.538 / 13.949 | encode (Scene): 1.479 / 2.643<br>prepare (Renderer::render): 0.331 / 0.657<br>submit + GPU completion: 4.135 / 4.751<br>read-back: 4.569 / 6.542 | – | – |

### Panels light (20 rounded + 20 plain) — scale 2

pert-ui command mix: 161 FillRect, 160 DrawMask (10 mask resources, 6 KiB), 0 DrawImage, 0 DrawVector.

| Renderer | Frame total median / p95 ms | Phases median / p95 ms | Not in total: update median ms | Cold compose ms |
|---|---|---|---|---|
| pert-ui CPU | 20.810 / 29.489 | compose (paint): 0.058 / 0.077<br>CpuRenderer::render: 20.752 / 29.414 | 0.023 | 0.216 |
| pert-ui wgpu | 8.844 / 9.675 | compose (paint): 0.045 / 0.050<br>render_offscreen (prepare + GPU + read-back): 8.801 / 9.628 | 0.017 | 0.143 |
| vello_cpu 1 thread | 1.351 / 1.423 | encode (+flush): 0.341 / 0.352<br>render: 1.009 / 1.082 | – | – |
| vello_cpu 5 workers | 0.611 / 0.651 | encode (+flush): 0.174 / 0.187<br>render: 0.435 / 0.478 | – | – |
| vello_gpu | 9.214 / 10.982 | encode (Scene): 0.374 / 0.407<br>prepare (Renderer::render): 0.186 / 0.214<br>submit + GPU completion: 3.724 / 4.667<br>read-back: 4.873 / 5.765 | – | – |

### Polyline 800 points — scale 2

pert-ui command mix: 1 FillRect, 0 DrawMask (0 mask resources, 0 KiB), 0 DrawImage, 1 DrawVector.

| Renderer | Frame total median / p95 ms | Phases median / p95 ms | Not in total: update median ms | Cold compose ms |
|---|---|---|---|---|
| pert-ui CPU | 42.866 / 50.007 | compose (paint): 0.032 / 0.046<br>CpuRenderer::render: 42.836 / 49.969 | 0.021 | 0.038 |
| pert-ui wgpu | 16.201 / 16.802 | compose (paint): 0.037 / 0.040<br>render_offscreen (prepare + GPU + read-back): 16.164 / 16.763 | 0.032 | 0.031 |
| vello_cpu 1 thread | 1.855 / 1.968 | encode (+flush): 0.834 / 0.874<br>render: 1.022 / 1.112 | – | – |
| vello_cpu 5 workers | 1.301 / 1.648 | encode (+flush): 0.933 / 1.256<br>render: 0.365 / 0.421 | – | – |
| vello_gpu | 8.847 / 11.089 | encode (Scene): 0.848 / 0.887<br>prepare (Renderer::render): 0.151 / 0.176<br>submit + GPU completion: 3.187 / 4.286<br>read-back: 4.625 / 5.931 | – | – |

### Polyline 4096 points — scale 2

pert-ui command mix: 1 FillRect, 0 DrawMask (0 mask resources, 0 KiB), 0 DrawImage, 1 DrawVector.

| Renderer | Frame total median / p95 ms | Phases median / p95 ms | Not in total: update median ms | Cold compose ms |
|---|---|---|---|---|
| pert-ui CPU | 44.535 / 46.829 | compose (paint): 0.108 / 0.115<br>CpuRenderer::render: 44.433 / 46.714 | 0.059 | 0.141 |
| pert-ui wgpu | 16.918 / 18.374 | compose (paint): 0.113 / 0.120<br>render_offscreen (prepare + GPU + read-back): 16.798 / 18.257 | 0.069 | 0.108 |
| vello_cpu 1 thread | 2.250 / 2.413 | encode (+flush): 1.238 / 1.309<br>render: 0.993 / 1.118 | – | – |
| vello_cpu 5 workers | 1.807 / 2.113 | encode (+flush): 1.469 / 1.750<br>render: 0.349 / 0.391 | – | – |
| vello_gpu | 8.679 / 10.417 | encode (Scene): 1.271 / 1.335<br>prepare (Renderer::render): 0.144 / 0.178<br>submit + GPU completion: 3.001 / 3.417<br>read-back: 4.272 / 5.288 | – | – |

### Gradient bars (32) — scale 2

pert-ui command mix: 33 FillRect, 0 DrawMask (0 mask resources, 0 KiB), 32 DrawImage, 0 DrawVector.

| Renderer | Frame total median / p95 ms | Phases median / p95 ms | Not in total: update median ms | Cold compose ms |
|---|---|---|---|---|
| pert-ui CPU | 45.691 / 51.218 | compose (paint): 0.011 / 0.016<br>CpuRenderer::render: 45.681 / 51.206 | 0.006 | 0.015 |
| pert-ui wgpu | 8.522 / 9.206 | compose (paint): 0.016 / 0.017<br>render_offscreen (prepare + GPU + read-back): 8.507 / 9.190 | 0.017 | 0.010 |
| vello_cpu 1 thread | 2.163 / 2.336 | encode (+flush): 0.051 / 0.057<br>render: 2.111 / 2.276 | – | – |
| vello_cpu 5 workers | 0.975 / 1.019 | encode (+flush): 0.092 / 0.103<br>render: 0.885 / 0.922 | – | – |
| vello_gpu | 8.359 / 10.752 | encode (Scene): 0.014 / 0.015<br>prepare (Renderer::render): 0.113 / 0.130<br>submit + GPU completion: 3.337 / 4.389<br>read-back: 4.841 / 6.602 | – | – |


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
| Panels (150 rounded + 100 plain) | 1 | 1 | 0.351 / 0.368 | 0.840 / 0.902 | 1.190 / 1.268 | 1.433 (10, 6) | 42 / 0.097 / 0.89% | 0 / 0.000 / 0.00% | 42 / 0.808 / 7.41% |
| Panels (150 rounded + 100 plain) | 1 | 5 workers | 0.328 / 0.355 | 0.446 / 0.485 | 0.773 / 0.826 | 0.947 (10, 6) | 42 / 0.097 / 0.89% | 0 / 0.000 / 0.00% | 42 / 0.808 / 7.41% |
| Panels light (20 rounded + 20 plain) | 1 | 1 | 0.056 / 0.059 | 0.363 / 0.369 | 0.420 / 0.425 | 0.355 (10, 6) | 41 / 0.014 / 0.12% | 0 / 0.000 / 0.00% | 41 / 0.305 / 2.75% |
| Panels light (20 rounded + 20 plain) | 1 | 5 workers | 0.082 / 2.583 | 0.204 / 0.263 | 0.288 / 2.842 | 0.544 (10, 6) | 41 / 0.014 / 0.12% | 0 / 0.000 / 0.00% | 41 / 0.305 / 2.75% |
| Polyline 800 points | 1 | 1 | 0.446 / 0.457 | 0.201 / 0.207 | 0.648 / 0.663 | 0.771 (0, 0) | 134 / 0.955 / 3.44% | 0 / 0.000 / 0.00% | 134 / 17.110 / 61.58% |
| Polyline 800 points | 1 | 5 workers | 0.501 / 0.784 | 0.121 / 0.154 | 0.627 / 0.909 | 1.285 (0, 0) | 134 / 0.955 / 3.44% | 0 / 0.000 / 0.00% | 134 / 17.110 / 61.58% |
| Polyline 4096 points | 1 | 1 | 0.842 / 0.861 | 0.206 / 0.222 | 1.047 / 1.080 | 1.300 (0, 0) | 145 / 1.004 / 3.70% | 0 / 0.000 / 0.00% | 145 / 16.916 / 62.41% |
| Polyline 4096 points | 1 | 5 workers | 0.915 / 1.207 | 0.119 / 0.143 | 1.033 / 1.346 | 1.834 (0, 0) | 145 / 1.004 / 3.70% | 0 / 0.000 / 0.00% | 145 / 16.916 / 62.41% |
| Gradient bars (32) | 1 | 1 | 0.028 / 0.033 | 1.434 / 1.540 | 1.462 / 1.573 | 0.346 (1, 2) | 0 / 0.000 / 0.00% | 0 / 0.000 / 0.00% | 0 / 0.000 / 0.00% |
| Gradient bars (32) | 1 | 5 workers | 0.057 / 0.063 | 0.640 / 0.649 | 0.697 / 0.711 | 0.546 (1, 2) | 0 / 0.000 / 0.00% | 0 / 0.000 / 0.00% | 0 / 0.000 / 0.00% |
| Panels (150 rounded + 100 plain) | 2 | 1 | 0.384 / 0.394 | 2.318 / 2.459 | 2.700 / 2.854 | 1.400 (10, 24) | 43 / 0.051 / 0.46% | 0 / 0.000 / 0.00% | 43 / 0.675 / 6.06% |
| Panels (150 rounded + 100 plain) | 2 | 5 workers | 0.327 / 0.423 | 0.989 / 1.057 | 1.327 / 1.437 | 1.686 (10, 24) | 43 / 0.051 / 0.46% | 0 / 0.000 / 0.00% | 43 / 0.675 / 6.06% |
| Panels light (20 rounded + 20 plain) | 2 | 1 | 0.068 / 0.071 | 1.385 / 1.447 | 1.454 / 1.518 | 3.893 (10, 24) | 41 / 0.007 / 0.07% | 0 / 0.000 / 0.00% | 41 / 0.248 / 2.29% |
| Panels light (20 rounded + 20 plain) | 2 | 5 workers | 0.101 / 0.111 | 0.517 / 0.623 | 0.619 / 0.725 | 1.348 (10, 24) | 41 / 0.007 / 0.07% | 0 / 0.000 / 0.00% | 41 / 0.248 / 2.29% |
| Polyline 800 points | 2 | 1 | 0.816 / 0.831 | 0.981 / 1.121 | 1.798 / 1.956 | 1.887 (0, 0) | 157 / 0.610 / 2.02% | 0 / 0.000 / 0.00% | 157 / 15.903 / 52.62% |
| Polyline 800 points | 2 | 5 workers | 0.890 / 1.173 | 0.356 / 0.474 | 1.317 / 1.596 | 2.156 (0, 0) | 157 / 0.610 / 2.02% | 0 / 0.000 / 0.00% | 157 / 15.903 / 52.62% |
| Polyline 4096 points | 2 | 1 | 1.288 / 2.122 | 1.086 / 2.655 | 2.374 / 4.737 | 2.402 (0, 0) | 156 / 0.642 / 2.15% | 0 / 0.000 / 0.00% | 156 / 15.729 / 52.71% |
| Polyline 4096 points | 2 | 5 workers | 1.855 / 1.948 | 0.409 / 0.595 | 2.272 / 2.497 | 3.253 (0, 0) | 156 / 0.642 / 2.15% | 0 / 0.000 / 0.00% | 156 / 15.729 / 52.71% |
| Gradient bars (32) | 2 | 1 | 0.051 / 0.056 | 4.705 / 4.957 | 4.757 / 5.012 | 1.060 (1, 5) | 0 / 0.000 / 0.00% | 0 / 0.000 / 0.00% | 0 / 0.000 / 0.00% |
| Gradient bars (32) | 2 | 5 workers | 0.118 / 0.134 | 1.730 / 1.786 | 1.848 / 1.916 | 1.286 (1, 5) | 0 / 0.000 / 0.00% | 0 / 0.000 / 0.00% | 0 / 0.000 / 0.00% |
<!-- load average after: 4.31 7.76 7.98 -->
