| Scenario | Metric | before median (max), 5 runs | head median, 5 runs | Verdict |
|---|---|---|---|---|
| J1 `navigation`: resize +7 px per frame | compose_first_ms | 2.020 (2.028) | 1.972 |  |
| J1 `navigation`: resize +7 px per frame | compose_median_ms | 2.281 (2.367) | 2.267 |  |
| J1 `navigation`: resize +7 px per frame | compose_p95_ms | 3.636 (3.711) | 3.605 |  |
| J1 `navigation`: resize +7 px per frame | cpu_median_ms | 9.184 (9.369) | 7.169 | faster |
| J1 `navigation`: resize +7 px per frame | frames | 30.000 (30.000) | 30.000 |  |
| J1 `navigation`: resize +7 px per frame | gpu_median_ms | 6.272 (6.360) | 7.050 | slower |
| J1 `navigation`: resize +7 px per frame | gpu_p95_ms | 12.571 (16.080) | 12.779 |  |
| J1 `navigation`: resize +7 px per frame | mask_kib_median | 218 (218) | 218 | changed |
| J1 `navigation`: resize +7 px per frame | mask_resources_median | 55.000 (55.000) | 50.000 | changed |
| J1 `navigation`: resize +7 px per frame | new_mask_kib_total | 1362 (1362) | 1362 |  |
| J1 `navigation`: resize +7 px per frame | new_masks_median | 1.000 (1.000) | 1.000 |  |
| J1 `navigation`: resize +7 px per frame | new_masks_total | 23.000 (23.000) | 23.000 |  |
| J1 `navigation`: resize +7 px per frame | other_commands_max | 0.000 (0.000) | 33.000 | changed |
| J1 `navigation`: resize +7 px per frame | present_frames | 30.000 (30.000) | 30.000 |  |
| J1 `navigation`: resize +7 px per frame | present_median_ms | 1.630 (1.714) | 1.619 |  |
| J1 `navigation`: resize +7 px per frame | present_p95_ms | 1.734 (1.808) | 1.729 |  |
| J1 `navigation`: scale 1 → 1.25 → 1.5 → 1.75 → 2 | compose_first_ms | 6.338 (6.456) | 6.298 |  |
| J1 `navigation`: scale 1 → 1.25 → 1.5 → 1.75 → 2 | compose_median_ms | 7.593 (7.832) | 7.544 |  |
| J1 `navigation`: scale 1 → 1.25 → 1.5 → 1.75 → 2 | compose_p95_ms | 8.062 (8.331) | 7.985 |  |
| J1 `navigation`: scale 1 → 1.25 → 1.5 → 1.75 → 2 | cpu_median_ms | 27.818 (28.347) | 22.257 | faster |
| J1 `navigation`: scale 1 → 1.25 → 1.5 → 1.75 → 2 | frames | 4.000 (4.000) | 4.000 |  |
| J1 `navigation`: scale 1 → 1.25 → 1.5 → 1.75 → 2 | gpu_median_ms | 9.789 (9.946) | 11.656 | slower |
| J1 `navigation`: scale 1 → 1.25 → 1.5 → 1.75 → 2 | gpu_p95_ms | 13.230 (13.323) | 15.463 | slower |
| J1 `navigation`: scale 1 → 1.25 → 1.5 → 1.75 → 2 | mask_kib_median | 660 (660) | 659 | changed |
| J1 `navigation`: scale 1 → 1.25 → 1.5 → 1.75 → 2 | mask_resources_median | 51.000 (51.000) | 46.000 | changed |
| J1 `navigation`: scale 1 → 1.25 → 1.5 → 1.75 → 2 | new_mask_kib_total | 2342 (2342) | 2339 | changed |
| J1 `navigation`: scale 1 → 1.25 → 1.5 → 1.75 → 2 | new_masks_median | 49.000 (49.000) | 44.000 | changed |
| J1 `navigation`: scale 1 → 1.25 → 1.5 → 1.75 → 2 | new_masks_total | 198 (198) | 178 | changed |
| J1 `navigation`: scale 1 → 1.25 → 1.5 → 1.75 → 2 | other_commands_max | 0.000 (0.000) | 31.000 | changed |
| J1 `navigation`: scale 1 → 1.25 → 1.5 → 1.75 → 2 | present_frames | 4.000 (4.000) | 4.000 |  |
| J1 `navigation`: scale 1 → 1.25 → 1.5 → 1.75 → 2 | present_median_ms | 4.198 (4.329) | 4.120 |  |
| J1 `navigation`: scale 1 → 1.25 → 1.5 → 1.75 → 2 | present_p95_ms | 5.264 (5.320) | 5.048 |  |
| cards: pan back rest frame | compose_first_ms | 0.058 (0.059) | 0.046 |  |
| cards: pan back rest frame | compose_median_ms | 0.058 (0.059) | 0.046 |  |
| cards: pan back rest frame | compose_p95_ms | 0.058 (0.059) | 0.046 |  |
| cards: pan back rest frame | cpu_median_ms | 2.701 (2.750) | 2.387 | faster |
| cards: pan back rest frame | frames | 1.000 (1.000) | 1.000 |  |
| cards: pan back rest frame | gpu_median_ms | 2.116 (2.199) | 2.261 | within noise |
| cards: pan back rest frame | gpu_p95_ms | 2.116 (2.199) | 2.261 | within noise |
| cards: pan back rest frame | mask_kib_median | 43.609 (43.609) | 43.484 | changed |
| cards: pan back rest frame | mask_resources_median | 29.000 (29.000) | 27.000 | changed |
| cards: pan back rest frame | new_mask_kib_total | 0.000 (0.000) | 0.000 |  |
| cards: pan back rest frame | new_masks_median | 0.000 (0.000) | 0.000 |  |
| cards: pan back rest frame | new_masks_total | 0.000 (0.000) | 0.000 |  |
| cards: pan back rest frame | other_commands_max | 0.000 (0.000) | 36.000 | changed |
| cards: pan back rest frame | present_frames | 1.000 (1.000) | 1.000 |  |
| cards: pan back rest frame | present_median_ms | 0.477 (0.495) | 0.450 |  |
| cards: pan back rest frame | present_p95_ms | 0.477 (0.495) | 0.450 |  |
| cards: pan back steps | compose_first_ms | 0.360 (0.370) | 0.352 |  |
| cards: pan back steps | compose_median_ms | 0.446 (0.451) | 0.433 |  |
| cards: pan back steps | compose_p95_ms | 0.470 (0.493) | 0.461 |  |
| cards: pan back steps | cpu_median_ms | 2.782 (2.800) | 2.449 | faster |
| cards: pan back steps | end_scale | 1.000 (1.000) | 1.000 |  |
| cards: pan back steps | frames | 60.000 (60.000) | 60.000 |  |
| cards: pan back steps | gpu_median_ms | 2.201 (2.208) | 2.330 | slower |
| cards: pan back steps | gpu_p95_ms | 2.273 (2.303) | 2.433 | slower |
| cards: pan back steps | mask_kib_median | 46.070 (46.070) | 45.945 | changed |
| cards: pan back steps | mask_resources_median | 29.000 (29.000) | 27.000 | changed |
| cards: pan back steps | new_mask_kib_total | 63.281 (63.281) | 63.281 |  |
| cards: pan back steps | new_masks_median | 0.000 (0.000) | 0.000 |  |
| cards: pan back steps | new_masks_total | 36.000 (36.000) | 36.000 |  |
| cards: pan back steps | other_commands_max | 0.000 (0.000) | 48.000 | changed |
| cards: pan back steps | present_frames | 60.000 (60.000) | 60.000 |  |
| cards: pan back steps | present_median_ms | 0.633 (0.640) | 0.560 | faster |
| cards: pan back steps | present_p95_ms | 0.709 (0.712) | 0.621 | faster |
| cards: pan rest frame | compose_first_ms | 0.058 (0.059) | 0.047 |  |
| cards: pan rest frame | compose_median_ms | 0.058 (0.059) | 0.047 |  |
| cards: pan rest frame | compose_p95_ms | 0.058 (0.059) | 0.047 |  |
| cards: pan rest frame | cpu_median_ms | 2.894 (2.920) | 2.570 | faster |
| cards: pan rest frame | frames | 1.000 (1.000) | 1.000 |  |
| cards: pan rest frame | gpu_median_ms | 2.162 (2.220) | 2.290 | within noise |
| cards: pan rest frame | gpu_p95_ms | 2.162 (2.220) | 2.290 | within noise |
| cards: pan rest frame | mask_kib_median | 46.070 (46.070) | 45.945 | changed |
| cards: pan rest frame | mask_resources_median | 29.000 (29.000) | 27.000 | changed |
| cards: pan rest frame | new_mask_kib_total | 0.000 (0.000) | 0.000 |  |
| cards: pan rest frame | new_masks_median | 0.000 (0.000) | 0.000 |  |
| cards: pan rest frame | new_masks_total | 0.000 (0.000) | 0.000 |  |
| cards: pan rest frame | other_commands_max | 0.000 (0.000) | 36.000 | changed |
| cards: pan rest frame | present_frames | 1.000 (1.000) | 1.000 |  |
| cards: pan rest frame | present_median_ms | 0.491 (0.506) | 0.454 |  |
| cards: pan rest frame | present_p95_ms | 0.491 (0.506) | 0.454 |  |
| cards: pan steps | compose_first_ms | 0.366 (0.377) | 0.362 |  |
| cards: pan steps | compose_median_ms | 0.445 (0.452) | 0.437 |  |
| cards: pan steps | compose_p95_ms | 0.569 (0.579) | 0.540 |  |
| cards: pan steps | cpu_median_ms | 2.790 (2.800) | 2.463 | faster |
| cards: pan steps | end_scale | 1.000 (1.000) | 1.000 |  |
| cards: pan steps | frames | 60.000 (60.000) | 60.000 |  |
| cards: pan steps | gpu_median_ms | 2.209 (2.221) | 2.320 | within noise |
| cards: pan steps | gpu_p95_ms | 2.295 (2.344) | 2.451 | within noise |
| cards: pan steps | mask_kib_median | 46.070 (46.070) | 45.945 | changed |
| cards: pan steps | mask_resources_median | 29.000 (29.000) | 27.000 | changed |
| cards: pan steps | new_mask_kib_total | 65.742 (65.742) | 65.742 |  |
| cards: pan steps | new_masks_median | 0.000 (0.000) | 0.000 |  |
| cards: pan steps | new_masks_total | 36.000 (36.000) | 36.000 |  |
| cards: pan steps | other_commands_max | 0.000 (0.000) | 48.000 | changed |
| cards: pan steps | present_frames | 60.000 (60.000) | 60.000 |  |
| cards: pan steps | present_median_ms | 0.629 (0.633) | 0.563 | faster |
| cards: pan steps | present_p95_ms | 0.708 (0.714) | 0.630 | faster |
| cards: zoom in rest frame | compose_first_ms | 4.663 (4.709) | 4.546 |  |
| cards: zoom in rest frame | compose_median_ms | 4.663 (4.709) | 4.546 |  |
| cards: zoom in rest frame | compose_p95_ms | 4.663 (4.709) | 4.546 |  |
| cards: zoom in rest frame | cpu_median_ms | 1.305 (1.330) | 1.022 | faster |
| cards: zoom in rest frame | frames | 1.000 (1.000) | 1.000 |  |
| cards: zoom in rest frame | gpu_median_ms | 2.197 (2.218) | 2.459 | slower |
| cards: zoom in rest frame | gpu_p95_ms | 2.197 (2.218) | 2.459 | slower |
| cards: zoom in rest frame | mask_kib_median | 0.000 (0.000) | 0.000 |  |
| cards: zoom in rest frame | mask_resources_median | 0.000 (0.000) | 0.000 |  |
| cards: zoom in rest frame | new_mask_kib_total | 0.000 (0.000) | 0.000 |  |
| cards: zoom in rest frame | new_masks_median | 0.000 (0.000) | 0.000 |  |
| cards: zoom in rest frame | new_masks_total | 0.000 (0.000) | 0.000 |  |
| cards: zoom in rest frame | other_commands_max | 0.000 (0.000) | 1.000 | changed |
| cards: zoom in rest frame | present_frames | 1.000 (1.000) | 1.000 |  |
| cards: zoom in rest frame | present_median_ms | 0.327 (0.342) | 0.336 |  |
| cards: zoom in rest frame | present_p95_ms | 0.327 (0.342) | 0.336 |  |
| cards: zoom in steps | compose_first_ms | 4.179 (4.187) | 4.123 |  |
| cards: zoom in steps | compose_median_ms | 0.503 (0.507) | 0.501 |  |
| cards: zoom in steps | compose_p95_ms | 0.541 (0.545) | 0.529 |  |
| cards: zoom in steps | cpu_median_ms | 2.898 (2.916) | 2.497 | faster |
| cards: zoom in steps | end_scale | 8.000 (8.000) | 8.000 |  |
| cards: zoom in steps | frames | 60.000 (60.000) | 60.000 |  |
| cards: zoom in steps | gpu_median_ms | 2.145 (2.158) | 2.313 | slower |
| cards: zoom in steps | gpu_p95_ms | 2.272 (2.302) | 2.473 | slower |
| cards: zoom in steps | mask_kib_median | 9.828 (9.828) | 9.670 | changed |
| cards: zoom in steps | mask_resources_median | 9.000 (9.000) | 7.000 | changed |
| cards: zoom in steps | new_mask_kib_total | 46.588 (46.588) | 46.430 | changed |
| cards: zoom in steps | new_masks_median | 0.000 (0.000) | 0.000 |  |
| cards: zoom in steps | new_masks_total | 28.000 (28.000) | 26.000 | changed |
| cards: zoom in steps | other_commands_max | 0.000 (0.000) | 36.000 | changed |
| cards: zoom in steps | present_frames | 60.000 (60.000) | 60.000 |  |
| cards: zoom in steps | present_median_ms | 0.393 (0.404) | 0.379 |  |
| cards: zoom in steps | present_p95_ms | 0.603 (0.605) | 0.533 | faster |
| cards: zoom out further rest frame | compose_first_ms | 5.039 (5.144) | 4.774 |  |
| cards: zoom out further rest frame | compose_median_ms | 5.039 (5.144) | 4.774 |  |
| cards: zoom out further rest frame | compose_p95_ms | 5.039 (5.144) | 4.774 |  |
| cards: zoom out further rest frame | cpu_median_ms | 0.821 (0.836) | 0.906 | slower |
| cards: zoom out further rest frame | frames | 1.000 (1.000) | 1.000 |  |
| cards: zoom out further rest frame | gpu_median_ms | 3.426 (3.482) | 3.109 | faster |
| cards: zoom out further rest frame | gpu_p95_ms | 3.426 (3.482) | 3.109 | faster |
| cards: zoom out further rest frame | mask_kib_median | 5.190 (5.190) | 5.186 | changed |
| cards: zoom out further rest frame | mask_resources_median | 133 (133) | 131 | changed |
| cards: zoom out further rest frame | new_mask_kib_total | 5.190 (5.190) | 5.186 | changed |
| cards: zoom out further rest frame | new_masks_median | 133 (133) | 131 | changed |
| cards: zoom out further rest frame | new_masks_total | 133 (133) | 131 | changed |
| cards: zoom out further rest frame | other_commands_max | 0.000 (0.000) | 192 | changed |
| cards: zoom out further rest frame | present_frames | 1.000 (1.000) | 1.000 |  |
| cards: zoom out further rest frame | present_median_ms | 2.654 (2.751) | 2.383 | faster |
| cards: zoom out further rest frame | present_p95_ms | 2.654 (2.751) | 2.383 | faster |
| cards: zoom out further steps | compose_first_ms | 4.169 (4.196) | 4.131 |  |
| cards: zoom out further steps | compose_median_ms | 0.622 (0.625) | 0.571 |  |
| cards: zoom out further steps | compose_p95_ms | 0.800 (0.817) | 0.751 |  |
| cards: zoom out further steps | cpu_median_ms | 1.695 (1.716) | 1.763 | within noise |
| cards: zoom out further steps | end_scale | 0.125 (0.125) | 0.125 |  |
| cards: zoom out further steps | frames | 60.000 (60.000) | 60.000 |  |
| cards: zoom out further steps | gpu_median_ms | 2.269 (2.286) | 2.258 |  |
| cards: zoom out further steps | gpu_p95_ms | 2.424 (2.538) | 2.390 |  |
| cards: zoom out further steps | mask_kib_median | 156 (156) | 156 | changed |
| cards: zoom out further steps | mask_resources_median | 96.000 (96.000) | 94.000 | changed |
| cards: zoom out further steps | new_mask_kib_total | 214 (214) | 214 | changed |
| cards: zoom out further steps | new_masks_median | 0.000 (0.000) | 0.000 |  |
| cards: zoom out further steps | new_masks_total | 131 (131) | 130 | changed |
| cards: zoom out further steps | other_commands_max | 0.000 (0.000) | 192 | changed |
| cards: zoom out further steps | present_frames | 60.000 (60.000) | 60.000 |  |
| cards: zoom out further steps | present_median_ms | 1.180 (1.233) | 0.934 | faster |
| cards: zoom out further steps | present_p95_ms | 1.679 (1.755) | 1.348 | faster |
| cards: zoom out rest frame | compose_first_ms | 4.239 (4.540) | 4.157 |  |
| cards: zoom out rest frame | compose_median_ms | 4.239 (4.540) | 4.157 |  |
| cards: zoom out rest frame | compose_p95_ms | 4.239 (4.540) | 4.157 |  |
| cards: zoom out rest frame | cpu_median_ms | 3.025 (3.125) | 2.661 | faster |
| cards: zoom out rest frame | frames | 1.000 (1.000) | 1.000 |  |
| cards: zoom out rest frame | gpu_median_ms | 2.324 (2.432) | 2.543 | within noise |
| cards: zoom out rest frame | gpu_p95_ms | 2.324 (2.432) | 2.543 | within noise |
| cards: zoom out rest frame | mask_kib_median | 43.609 (43.609) | 43.484 | changed |
| cards: zoom out rest frame | mask_resources_median | 29.000 (29.000) | 27.000 | changed |
| cards: zoom out rest frame | new_mask_kib_total | 43.609 (43.609) | 43.484 | changed |
| cards: zoom out rest frame | new_masks_median | 29.000 (29.000) | 27.000 | changed |
| cards: zoom out rest frame | new_masks_total | 29.000 (29.000) | 27.000 | changed |
| cards: zoom out rest frame | other_commands_max | 0.000 (0.000) | 36.000 | changed |
| cards: zoom out rest frame | present_frames | 1.000 (1.000) | 1.000 |  |
| cards: zoom out rest frame | present_median_ms | 0.847 (0.858) | 0.754 |  |
| cards: zoom out rest frame | present_p95_ms | 0.847 (0.858) | 0.754 |  |
| cards: zoom out steps | compose_first_ms | 4.325 (4.369) | 4.226 |  |
| cards: zoom out steps | compose_median_ms | 0.506 (0.510) | 0.503 |  |
| cards: zoom out steps | compose_p95_ms | 1.012 (1.024) | 0.994 |  |
| cards: zoom out steps | cpu_median_ms | 2.679 (2.684) | 2.271 | faster |
| cards: zoom out steps | end_scale | 1.000 (1.000) | 1.000 |  |
| cards: zoom out steps | frames | 60.000 (60.000) | 60.000 |  |
| cards: zoom out steps | gpu_median_ms | 2.209 (2.241) | 2.354 | slower |
| cards: zoom out steps | gpu_p95_ms | 2.338 (2.355) | 2.481 | slower |
| cards: zoom out steps | mask_kib_median | 530 (530) | 523 | changed |
| cards: zoom out steps | mask_resources_median | 9.000 (9.000) | 7.000 | changed |
| cards: zoom out steps | new_mask_kib_total | 2524 (2524) | 2517 | changed |
| cards: zoom out steps | new_masks_median | 0.000 (0.000) | 0.000 |  |
| cards: zoom out steps | new_masks_total | 29.000 (29.000) | 27.000 | changed |
| cards: zoom out steps | other_commands_max | 0.000 (0.000) | 36.000 | changed |
| cards: zoom out steps | present_frames | 60.000 (60.000) | 60.000 |  |
| cards: zoom out steps | present_median_ms | 0.409 (0.417) | 0.401 |  |
| cards: zoom out steps | present_p95_ms | 0.645 (0.664) | 0.552 | faster |
| diagram: live zoom settle frame | compose_first_ms | 0.098 (0.114) | 0.097 |  |
| diagram: live zoom settle frame | compose_median_ms | 0.098 (0.114) | 0.097 |  |
| diagram: live zoom settle frame | compose_p95_ms | 0.098 (0.114) | 0.097 |  |
| diagram: live zoom settle frame | cpu_median_ms | 2.255 (2.277) | 1.414 | faster |
| diagram: live zoom settle frame | frames | 1.000 (1.000) | 1.000 |  |
| diagram: live zoom settle frame | gpu_median_ms | 3.686 (3.705) | 3.678 |  |
| diagram: live zoom settle frame | gpu_p95_ms | 3.686 (3.705) | 3.678 |  |
| diagram: live zoom settle frame | mask_kib_median | 21.869 (21.869) | 21.735 | changed |
| diagram: live zoom settle frame | mask_resources_median | 29.000 (29.000) | 26.000 | changed |
| diagram: live zoom settle frame | new_mask_kib_total | 0.000 (0.000) | 0.000 |  |
| diagram: live zoom settle frame | new_masks_median | 0.000 (0.000) | 0.000 |  |
| diagram: live zoom settle frame | new_masks_total | 0.000 (0.000) | 0.000 |  |
| diagram: live zoom settle frame | other_commands_max | 0.000 (0.000) | 15.000 | changed |
| diagram: live zoom settle frame | present_frames | 1.000 (1.000) | 1.000 |  |
| diagram: live zoom settle frame | present_median_ms | 0.412 (0.425) | 0.409 |  |
| diagram: live zoom settle frame | present_p95_ms | 0.412 (0.425) | 0.409 |  |
| diagram: live zoom steps | compose_first_ms | 0.246 (0.248) | 0.249 | within noise |
| diagram: live zoom steps | compose_median_ms | 0.248 (0.252) | 0.243 |  |
| diagram: live zoom steps | compose_p95_ms | 0.273 (0.278) | 0.268 |  |
| diagram: live zoom steps | cpu_median_ms | 2.474 (2.502) | 1.651 | faster |
| diagram: live zoom steps | frames | 30.000 (30.000) | 30.000 |  |
| diagram: live zoom steps | gpu_median_ms | 3.695 (3.744) | 3.803 | within noise |
| diagram: live zoom steps | gpu_p95_ms | 3.857 (3.892) | 3.930 | within noise |
| diagram: live zoom steps | mask_kib_median | 23.138 (23.138) | 22.576 | changed |
| diagram: live zoom steps | mask_resources_median | 33.000 (33.000) | 27.000 | changed |
| diagram: live zoom steps | new_mask_kib_total | 26.290 (26.290) | 25.960 | changed |
| diagram: live zoom steps | new_masks_median | 1.000 (1.000) | 1.000 |  |
| diagram: live zoom steps | new_masks_total | 35.000 (35.000) | 33.000 | changed |
| diagram: live zoom steps | other_commands_max | 0.000 (0.000) | 19.000 | changed |
| diagram: live zoom steps | present_frames | 30.000 (30.000) | 30.000 |  |
| diagram: live zoom steps | present_median_ms | 0.553 (0.561) | 0.554 |  |
| diagram: live zoom steps | present_p95_ms | 0.811 (0.862) | 0.788 |  |
| diagram: pan 7 px per frame | compose_first_ms | 0.117 (0.142) | 0.115 |  |
| diagram: pan 7 px per frame | compose_median_ms | 0.161 (0.162) | 0.163 | within noise |
| diagram: pan 7 px per frame | compose_p95_ms | 0.180 (0.197) | 0.174 |  |
| diagram: pan 7 px per frame | cpu_median_ms | 2.891 (2.895) | 1.937 | faster |
| diagram: pan 7 px per frame | frames | 60.000 (60.000) | 60.000 |  |
| diagram: pan 7 px per frame | gpu_median_ms | 3.788 (3.835) | 3.844 | within noise |
| diagram: pan 7 px per frame | gpu_p95_ms | 3.920 (5.751) | 3.942 |  |
| diagram: pan 7 px per frame | mask_kib_median | 22.355 (22.355) | 21.735 | changed |
| diagram: pan 7 px per frame | mask_resources_median | 33.000 (33.000) | 26.000 | changed |
| diagram: pan 7 px per frame | new_mask_kib_total | 0.000 (0.000) | 0.000 |  |
| diagram: pan 7 px per frame | new_masks_median | 0.000 (0.000) | 0.000 |  |
| diagram: pan 7 px per frame | new_masks_total | 0.000 (0.000) | 0.000 |  |
| diagram: pan 7 px per frame | other_commands_max | 0.000 (0.000) | 19.000 | changed |
| diagram: pan 7 px per frame | present_frames | 60.000 (60.000) | 60.000 |  |
| diagram: pan 7 px per frame | present_median_ms | 0.724 (0.731) | 0.797 | slower |
| diagram: pan 7 px per frame | present_p95_ms | 0.861 (0.873) | 0.887 | within noise |
| diagram: settled zoom in and out | compose_first_ms | 0.240 (0.249) | 0.231 |  |
| diagram: settled zoom in and out | compose_median_ms | 0.233 (0.236) | 0.214 |  |
| diagram: settled zoom in and out | compose_p95_ms | 0.302 (0.356) | 0.276 |  |
| diagram: settled zoom in and out | cpu_median_ms | 2.480 (2.491) | 1.647 | faster |
| diagram: settled zoom in and out | frames | 16.000 (16.000) | 16.000 |  |
| diagram: settled zoom in and out | gpu_median_ms | 3.724 (3.789) | 3.799 | within noise |
| diagram: settled zoom in and out | gpu_p95_ms | 3.814 (3.939) | 3.894 |  |
| diagram: settled zoom in and out | mask_kib_median | 26.002 (26.002) | 24.630 | changed |
| diagram: settled zoom in and out | mask_resources_median | 33.000 (33.000) | 27.000 | changed |
| diagram: settled zoom in and out | new_mask_kib_total | 73.906 (73.906) | 51.752 | changed |
| diagram: settled zoom in and out | new_masks_median | 4.000 (4.000) | 2.000 | changed |
| diagram: settled zoom in and out | new_masks_total | 58.000 (58.000) | 30.000 | changed |
| diagram: settled zoom in and out | other_commands_max | 0.000 (0.000) | 19.000 | changed |
| diagram: settled zoom in and out | present_frames | 16.000 (16.000) | 16.000 |  |
| diagram: settled zoom in and out | present_median_ms | 0.535 (0.558) | 0.535 |  |
| diagram: settled zoom in and out | present_p95_ms | 0.894 (0.919) | 0.862 |  |
| knob: gallery input page, Shift+Up (whole display list) | compose_first_ms | 1.383 (1.398) | 0.917 | faster |
| knob: gallery input page, Shift+Up (whole display list) | compose_median_ms | 1.602 (1.622) | 1.126 | faster |
| knob: gallery input page, Shift+Up (whole display list) | compose_p95_ms | 1.679 (1.689) | 1.183 | faster |
| knob: gallery input page, Shift+Up (whole display list) | cpu_median_ms | 5.605 (5.615) | 3.515 | faster |
| knob: gallery input page, Shift+Up (whole display list) | frames | 200 (200) | 200 |  |
| knob: gallery input page, Shift+Up (whole display list) | gpu_median_ms | 3.672 (3.700) | 4.519 | slower |
| knob: gallery input page, Shift+Up (whole display list) | gpu_p95_ms | 3.779 (3.832) | 4.666 | slower |
| knob: gallery input page, Shift+Up (whole display list) | mask_kib_median | 73.871 (73.871) | 69.758 | changed |
| knob: gallery input page, Shift+Up (whole display list) | mask_resources_median | 47.000 (47.000) | 38.000 | changed |
| knob: gallery input page, Shift+Up (whole display list) | new_mask_kib_total | 2461 (2461) | 1963 | changed |
| knob: gallery input page, Shift+Up (whole display list) | new_masks_median | 5.000 (5.000) | 3.000 | changed |
| knob: gallery input page, Shift+Up (whole display list) | new_masks_total | 994 (994) | 600 | changed |
| knob: gallery input page, Shift+Up (whole display list) | other_commands_max | 0.000 (0.000) | 45.000 | changed |
| knob: gallery input page, Shift+Up (whole display list) | present_frames | 200 (200) | 200 |  |
| knob: gallery input page, Shift+Up (whole display list) | present_median_ms | 0.601 (0.604) | 0.587 |  |
| knob: gallery input page, Shift+Up (whole display list) | present_p95_ms | 0.648 (0.655) | 0.620 |  |
| knob: isolated, no readout, host writes | compose_first_ms | 0.226 (0.231) | 0.003 | faster |
| knob: isolated, no readout, host writes | compose_median_ms | 0.261 (0.262) | 0.029 | faster |
| knob: isolated, no readout, host writes | compose_p95_ms | 0.272 (0.280) | 0.030 | faster |
| knob: isolated, no readout, host writes | cpu_median_ms | 0.107 (0.107) | 0.089 |  |
| knob: isolated, no readout, host writes | frames | 200 (200) | 200 |  |
| knob: isolated, no readout, host writes | gpu_median_ms | 0.893 (0.895) | 0.907 | within noise |
| knob: isolated, no readout, host writes | gpu_p95_ms | 1.058 (1.093) | 0.996 |  |
| knob: isolated, no readout, host writes | mask_kib_median | 2.531 (2.531) | 0.000 | changed |
| knob: isolated, no readout, host writes | mask_resources_median | 2.000 (2.000) | 0.000 | changed |
| knob: isolated, no readout, host writes | new_mask_kib_total | 248 (248) | 0.000 | changed |
| knob: isolated, no readout, host writes | new_masks_median | 1.000 (1.000) | 0.000 | changed |
| knob: isolated, no readout, host writes | new_masks_total | 196 (196) | 0.000 | changed |
| knob: isolated, no readout, host writes | other_commands_max | 0.000 (0.000) | 2.000 | changed |
| knob: isolated, no readout, host writes | present_frames | 200 (200) | 200 |  |
| knob: isolated, no readout, host writes | present_median_ms | 0.386 (0.393) | 0.370 |  |
| knob: isolated, no readout, host writes | present_p95_ms | 0.408 (0.418) | 0.398 |  |
| mixer: one tick per frame | compose_first_ms | 0.049 (0.051) | 0.044 |  |
| mixer: one tick per frame | compose_median_ms | 0.082 (0.085) | 0.078 |  |
| mixer: one tick per frame | compose_p95_ms | 0.115 (0.126) | 0.106 |  |
| mixer: one tick per frame | cpu_median_ms | 0.944 (0.945) | 1.055 | slower |
| mixer: one tick per frame | frames | 300 (300) | 300 |  |
| mixer: one tick per frame | gpu_median_ms | 2.192 (2.195) | 2.225 | within noise |
| mixer: one tick per frame | gpu_p95_ms | 2.273 (2.284) | 2.315 | within noise |
| mixer: one tick per frame | mask_kib_median | 18.845 (18.845) | 18.379 | changed |
| mixer: one tick per frame | mask_resources_median | 19.000 (19.000) | 14.000 | changed |
| mixer: one tick per frame | new_mask_kib_total | 0.000 (0.000) | 0.000 |  |
| mixer: one tick per frame | new_masks_median | 0.000 (0.000) | 0.000 |  |
| mixer: one tick per frame | new_masks_total | 0.000 (0.000) | 0.000 |  |
| mixer: one tick per frame | other_commands_max | 0.000 (0.000) | 21.000 | changed |
| mixer: one tick per frame | present_frames | 300 (300) | 300 |  |
| mixer: one tick per frame | present_median_ms | 0.432 (0.437) | 0.420 |  |
| mixer: one tick per frame | present_p95_ms | 0.459 (0.468) | 0.446 |  |
| page/chips/1 | compose_cold_ms | 1.963 (1.966) | 1.911 |  |
| page/chips/1 | compose_warm_ms | 0.057 (0.058) | 0.048 |  |
| page/chips/1 | cpu_median_ms | 7.493 (7.527) | 5.690 | faster |
| page/chips/1 | gpu_first_ms | 5.884 (6.059) | 7.133 | slower |
| page/chips/1 | gpu_warm_median_ms | 4.579 (4.586) | 5.294 | slower |
| page/chips/1 | gpu_warm_p95_ms | 4.654 (4.672) | 5.398 | slower |
| page/chips/1 | mask_kib | 162 (162) | 161 | changed |
| page/chips/1 | mask_resources | 61.000 (61.000) | 55.000 | changed |
| page/chips/1 | masks_drawn | 205 (205) | 63.000 | changed |
| page/chips/1 | other_commands | 0.000 (0.000) | 37.000 | changed |
| page/chips/1 | present_cold_ms | 0.688 (0.692) | 0.655 |  |
| page/chips/1 | present_warm_median_ms | 0.518 (0.522) | 0.500 |  |
| page/chips/1 | rects | 104 (104) | 14.000 | changed |
| page/chips/1.25 | compose_cold_ms | 2.258 (2.290) | 2.206 |  |
| page/chips/1.25 | compose_warm_ms | 0.058 (0.061) | 0.049 |  |
| page/chips/1.25 | cpu_median_ms | 12.224 (12.261) | 9.296 | faster |
| page/chips/1.25 | gpu_first_ms | 5.173 (8.314) | 6.187 |  |
| page/chips/1.25 | gpu_warm_median_ms | 5.984 (6.337) | 6.381 | within noise |
| page/chips/1.25 | gpu_warm_p95_ms | 6.655 (6.702) | 6.916 | within noise |
| page/chips/1.25 | mask_kib | 257 (257) | 255 | changed |
| page/chips/1.25 | mask_resources | 62.000 (62.000) | 56.000 | changed |
| page/chips/1.25 | masks_drawn | 206 (206) | 64.000 | changed |
| page/chips/1.25 | other_commands | 0.000 (0.000) | 37.000 | changed |
| page/chips/1.25 | present_cold_ms | 0.750 (0.759) | 0.734 |  |
| page/chips/1.25 | present_warm_median_ms | 0.520 (0.527) | 0.514 |  |
| page/chips/1.25 | rects | 104 (104) | 14.000 | changed |
| page/chips/2 | compose_cold_ms | 3.287 (3.290) | 3.181 |  |
| page/chips/2 | compose_warm_ms | 0.058 (0.060) | 0.049 |  |
| page/chips/2 | cpu_median_ms | 28.265 (28.334) | 20.831 | faster |
| page/chips/2 | gpu_first_ms | 11.443 (15.445) | 13.754 |  |
| page/chips/2 | gpu_warm_median_ms | 9.421 (9.485) | 10.155 | slower |
| page/chips/2 | gpu_warm_p95_ms | 9.653 (9.856) | 10.863 | slower |
| page/chips/2 | mask_kib | 654 (654) | 650 | changed |
| page/chips/2 | mask_resources | 62.000 (62.000) | 56.000 | changed |
| page/chips/2 | masks_drawn | 206 (206) | 64.000 | changed |
| page/chips/2 | other_commands | 0.000 (0.000) | 37.000 | changed |
| page/chips/2 | present_cold_ms | 0.893 (0.899) | 0.859 |  |
| page/chips/2 | present_warm_median_ms | 0.522 (0.527) | 0.511 |  |
| page/chips/2 | rects | 104 (104) | 14.000 | changed |
| page/collections/1 | compose_cold_ms | 2.207 (2.226) | 2.192 |  |
| page/collections/1 | compose_warm_ms | 0.073 (0.075) | 0.070 |  |
| page/collections/1 | cpu_median_ms | 7.714 (7.752) | 5.959 | faster |
| page/collections/1 | gpu_first_ms | 5.843 (5.962) | 7.278 | slower |
| page/collections/1 | gpu_warm_median_ms | 4.530 (4.581) | 5.319 | slower |
| page/collections/1 | gpu_warm_p95_ms | 4.617 (4.694) | 5.432 | slower |
| page/collections/1 | mask_kib | 181 (181) | 180 | changed |
| page/collections/1 | mask_resources | 63.000 (63.000) | 58.000 | changed |
| page/collections/1 | masks_drawn | 162 (162) | 62.000 | changed |
| page/collections/1 | other_commands | 0.000 (0.000) | 24.000 | changed |
| page/collections/1 | present_cold_ms | 0.685 (0.822) | 0.659 |  |
| page/collections/1 | present_warm_median_ms | 0.495 (0.518) | 0.489 |  |
| page/collections/1 | rects | 98.000 (98.000) | 16.000 | changed |
| page/collections/1.25 | compose_cold_ms | 2.551 (2.629) | 2.526 |  |
| page/collections/1.25 | compose_warm_ms | 0.077 (0.081) | 0.069 |  |
| page/collections/1.25 | cpu_median_ms | 12.651 (12.685) | 9.792 | faster |
| page/collections/1.25 | gpu_first_ms | 8.089 (8.216) | 6.274 |  |
| page/collections/1.25 | gpu_warm_median_ms | 6.342 (6.564) | 5.849 |  |
| page/collections/1.25 | gpu_warm_p95_ms | 6.623 (6.723) | 6.565 |  |
| page/collections/1.25 | mask_kib | 289 (289) | 288 | changed |
| page/collections/1.25 | mask_resources | 68.000 (68.000) | 63.000 | changed |
| page/collections/1.25 | masks_drawn | 167 (167) | 67.000 | changed |
| page/collections/1.25 | other_commands | 0.000 (0.000) | 24.000 | changed |
| page/collections/1.25 | present_cold_ms | 0.775 (0.833) | 0.742 |  |
| page/collections/1.25 | present_warm_median_ms | 0.509 (0.525) | 0.487 |  |
| page/collections/1.25 | rects | 98.000 (98.000) | 16.000 | changed |
| page/collections/2 | compose_cold_ms | 3.555 (3.582) | 3.513 |  |
| page/collections/2 | compose_warm_ms | 0.075 (0.089) | 0.068 |  |
| page/collections/2 | cpu_median_ms | 28.894 (28.988) | 21.899 | faster |
| page/collections/2 | gpu_first_ms | 11.519 (11.593) | 13.664 | slower |
| page/collections/2 | gpu_warm_median_ms | 9.532 (9.659) | 10.342 | slower |
| page/collections/2 | gpu_warm_p95_ms | 9.834 (9.941) | 10.864 | slower |
| page/collections/2 | mask_kib | 715 (715) | 714 | changed |
| page/collections/2 | mask_resources | 63.000 (63.000) | 58.000 | changed |
| page/collections/2 | masks_drawn | 162 (162) | 62.000 | changed |
| page/collections/2 | other_commands | 0.000 (0.000) | 24.000 | changed |
| page/collections/2 | present_cold_ms | 0.879 (0.930) | 0.849 |  |
| page/collections/2 | present_warm_median_ms | 0.508 (0.524) | 0.487 |  |
| page/collections/2 | rects | 98.000 (98.000) | 16.000 | changed |
| page/colors/1 | compose_cold_ms | 1.823 (1.887) | 1.773 |  |
| page/colors/1 | compose_warm_ms | 0.055 (0.061) | 0.048 |  |
| page/colors/1 | cpu_median_ms | 15.968 (15.997) | 14.504 | faster |
| page/colors/1 | gpu_first_ms | 5.897 (5.936) | 7.069 | slower |
| page/colors/1 | gpu_warm_median_ms | 4.599 (4.647) | 4.792 | within noise |
| page/colors/1 | gpu_warm_p95_ms | 4.665 (4.687) | 5.217 | slower |
| page/colors/1 | mask_kib | 146 (146) | 146 | changed |
| page/colors/1 | mask_resources | 41.000 (41.000) | 36.000 | changed |
| page/colors/1 | masks_drawn | 143 (143) | 37.000 | changed |
| page/colors/1 | other_commands | 0.000 (0.000) | 39.000 | changed |
| page/colors/1 | present_cold_ms | 0.697 (0.727) | 0.648 |  |
| page/colors/1 | present_warm_median_ms | 0.504 (0.508) | 0.491 |  |
| page/colors/1 | rects | 248 (248) | 119 | changed |
| page/colors/1.25 | compose_cold_ms | 2.078 (2.111) | 2.030 |  |
| page/colors/1.25 | compose_warm_ms | 0.056 (0.056) | 0.049 |  |
| page/colors/1.25 | cpu_median_ms | 25.476 (25.485) | 23.037 | faster |
| page/colors/1.25 | gpu_first_ms | 5.164 (8.258) | 6.201 |  |
| page/colors/1.25 | gpu_warm_median_ms | 6.063 (6.626) | 6.327 |  |
| page/colors/1.25 | gpu_warm_p95_ms | 6.709 (6.749) | 7.012 | within noise |
| page/colors/1.25 | mask_kib | 228 (228) | 227 | changed |
| page/colors/1.25 | mask_resources | 41.000 (41.000) | 36.000 | changed |
| page/colors/1.25 | masks_drawn | 143 (143) | 37.000 | changed |
| page/colors/1.25 | other_commands | 0.000 (0.000) | 39.000 | changed |
| page/colors/1.25 | present_cold_ms | 0.772 (0.791) | 0.697 | faster |
| page/colors/1.25 | present_warm_median_ms | 0.496 (0.513) | 0.490 |  |
| page/colors/1.25 | rects | 248 (248) | 119 | changed |
| page/colors/2 | compose_cold_ms | 2.963 (2.987) | 2.919 |  |
| page/colors/2 | compose_warm_ms | 0.055 (0.056) | 0.049 |  |
| page/colors/2 | cpu_median_ms | 62.350 (62.429) | 56.393 | faster |
| page/colors/2 | gpu_first_ms | 11.240 (11.750) | 13.512 | slower |
| page/colors/2 | gpu_warm_median_ms | 9.506 (9.557) | 10.068 | slower |
| page/colors/2 | gpu_warm_p95_ms | 9.658 (9.932) | 10.709 | slower |
| page/colors/2 | mask_kib | 579 (579) | 578 | changed |
| page/colors/2 | mask_resources | 41.000 (41.000) | 36.000 | changed |
| page/colors/2 | masks_drawn | 143 (143) | 37.000 | changed |
| page/colors/2 | other_commands | 0.000 (0.000) | 39.000 | changed |
| page/colors/2 | present_cold_ms | 0.946 (0.964) | 0.797 | faster |
| page/colors/2 | present_warm_median_ms | 0.496 (0.505) | 0.496 |  |
| page/colors/2 | rects | 248 (248) | 119 | changed |
| page/controls/1 | compose_cold_ms | 1.093 (1.125) | 1.040 |  |
| page/controls/1 | compose_warm_ms | 0.053 (0.053) | 0.044 |  |
| page/controls/1 | cpu_median_ms | 4.472 (4.609) | 3.188 | faster |
| page/controls/1 | gpu_first_ms | 5.142 (5.360) | 5.746 | slower |
| page/controls/1 | gpu_warm_median_ms | 4.184 (4.219) | 4.470 | slower |
| page/controls/1 | gpu_warm_p95_ms | 4.268 (4.327) | 4.569 | slower |
| page/controls/1 | mask_kib | 78.593 (78.593) | 77.640 | changed |
| page/controls/1 | mask_resources | 50.000 (50.000) | 42.000 | changed |
| page/controls/1 | masks_drawn | 195 (195) | 45.000 | changed |
| page/controls/1 | other_commands | 0.000 (0.000) | 38.000 | changed |
| page/controls/1 | present_cold_ms | 0.639 (0.666) | 0.547 | faster |
| page/controls/1 | present_warm_median_ms | 0.493 (0.507) | 0.483 |  |
| page/controls/1 | rects | 113 (113) | 13.000 | changed |
| page/controls/1.25 | compose_cold_ms | 1.243 (1.247) | 1.188 |  |
| page/controls/1.25 | compose_warm_ms | 0.053 (0.054) | 0.045 |  |
| page/controls/1.25 | cpu_median_ms | 7.163 (7.226) | 5.106 | faster |
| page/controls/1.25 | gpu_first_ms | 7.222 (7.423) | 5.117 |  |
| page/controls/1.25 | gpu_warm_median_ms | 6.114 (6.146) | 6.050 |  |
| page/controls/1.25 | gpu_warm_p95_ms | 6.191 (6.212) | 6.527 | slower |
| page/controls/1.25 | mask_kib | 125 (125) | 123 | changed |
| page/controls/1.25 | mask_resources | 50.000 (50.000) | 42.000 | changed |
| page/controls/1.25 | masks_drawn | 195 (195) | 45.000 | changed |
| page/controls/1.25 | other_commands | 0.000 (0.000) | 38.000 | changed |
| page/controls/1.25 | present_cold_ms | 0.659 (0.689) | 0.610 |  |
| page/controls/1.25 | present_warm_median_ms | 0.491 (0.502) | 0.492 |  |
| page/controls/1.25 | rects | 113 (113) | 13.000 | changed |
| page/controls/2 | compose_cold_ms | 1.748 (1.778) | 1.655 |  |
| page/controls/2 | compose_warm_ms | 0.054 (0.058) | 0.045 |  |
| page/controls/2 | cpu_median_ms | 16.856 (16.896) | 11.491 | faster |
| page/controls/2 | gpu_first_ms | 10.276 (10.307) | 11.230 | slower |
| page/controls/2 | gpu_warm_median_ms | 8.862 (8.893) | 9.074 | within noise |
| page/controls/2 | gpu_warm_p95_ms | 9.127 (9.238) | 9.387 | within noise |
| page/controls/2 | mask_kib | 311 (311) | 308 | changed |
| page/controls/2 | mask_resources | 50.000 (50.000) | 42.000 | changed |
| page/controls/2 | masks_drawn | 195 (195) | 45.000 | changed |
| page/controls/2 | other_commands | 0.000 (0.000) | 38.000 | changed |
| page/controls/2 | present_cold_ms | 0.772 (0.795) | 0.704 | faster |
| page/controls/2 | present_warm_median_ms | 0.497 (0.508) | 0.486 |  |
| page/controls/2 | rects | 113 (113) | 13.000 | changed |
| page/dragdrop/1 | compose_cold_ms | 1.750 (1.782) | 1.714 |  |
| page/dragdrop/1 | compose_warm_ms | 0.056 (0.060) | 0.049 |  |
| page/dragdrop/1 | cpu_median_ms | 7.320 (7.357) | 5.441 | faster |
| page/dragdrop/1 | gpu_first_ms | 5.891 (5.950) | 7.468 | slower |
| page/dragdrop/1 | gpu_warm_median_ms | 4.587 (4.610) | 5.490 | slower |
| page/dragdrop/1 | gpu_warm_p95_ms | 4.640 (4.681) | 5.526 | slower |
| page/dragdrop/1 | mask_kib | 164 (164) | 164 | changed |
| page/dragdrop/1 | mask_resources | 53.000 (53.000) | 48.000 | changed |
| page/dragdrop/1 | masks_drawn | 148 (148) | 48.000 | changed |
| page/dragdrop/1 | other_commands | 0.000 (0.000) | 26.000 | changed |
| page/dragdrop/1 | present_cold_ms | 0.632 (0.645) | 0.599 |  |
| page/dragdrop/1 | present_warm_median_ms | 0.494 (0.509) | 0.482 |  |
| page/dragdrop/1 | rects | 102 (102) | 20.000 | changed |
| page/dragdrop/1.25 | compose_cold_ms | 1.994 (2.019) | 1.963 |  |
| page/dragdrop/1.25 | compose_warm_ms | 0.055 (0.056) | 0.050 |  |
| page/dragdrop/1.25 | cpu_median_ms | 11.785 (11.865) | 8.745 | faster |
| page/dragdrop/1.25 | gpu_first_ms | 8.296 (8.351) | 6.425 |  |
| page/dragdrop/1.25 | gpu_warm_median_ms | 6.074 (6.457) | 5.540 |  |
| page/dragdrop/1.25 | gpu_warm_p95_ms | 6.603 (6.782) | 6.008 |  |
| page/dragdrop/1.25 | mask_kib | 253 (253) | 253 | changed |
| page/dragdrop/1.25 | mask_resources | 53.000 (53.000) | 48.000 | changed |
| page/dragdrop/1.25 | masks_drawn | 148 (148) | 48.000 | changed |
| page/dragdrop/1.25 | other_commands | 0.000 (0.000) | 26.000 | changed |
| page/dragdrop/1.25 | present_cold_ms | 0.692 (0.717) | 0.659 |  |
| page/dragdrop/1.25 | present_warm_median_ms | 0.493 (0.512) | 0.488 |  |
| page/dragdrop/1.25 | rects | 102 (102) | 20.000 | changed |
| page/dragdrop/2 | compose_cold_ms | 2.893 (2.901) | 2.852 |  |
| page/dragdrop/2 | compose_warm_ms | 0.055 (0.059) | 0.050 |  |
| page/dragdrop/2 | cpu_median_ms | 27.602 (27.751) | 20.034 | faster |
| page/dragdrop/2 | gpu_first_ms | 11.628 (11.789) | 14.182 | slower |
| page/dragdrop/2 | gpu_warm_median_ms | 9.542 (9.574) | 10.108 | slower |
| page/dragdrop/2 | gpu_warm_p95_ms | 9.780 (9.903) | 10.566 | slower |
| page/dragdrop/2 | mask_kib | 649 (649) | 648 | changed |
| page/dragdrop/2 | mask_resources | 53.000 (53.000) | 48.000 | changed |
| page/dragdrop/2 | masks_drawn | 148 (148) | 48.000 | changed |
| page/dragdrop/2 | other_commands | 0.000 (0.000) | 26.000 | changed |
| page/dragdrop/2 | present_cold_ms | 0.842 (0.852) | 0.800 |  |
| page/dragdrop/2 | present_warm_median_ms | 0.492 (0.504) | 0.486 |  |
| page/dragdrop/2 | rects | 102 (102) | 20.000 | changed |
| page/editing/1 | compose_cold_ms | 1.726 (1.735) | 1.714 |  |
| page/editing/1 | compose_warm_ms | 0.052 (0.053) | 0.046 |  |
| page/editing/1 | cpu_median_ms | 6.139 (6.176) | 4.521 | faster |
| page/editing/1 | gpu_first_ms | 5.499 (5.562) | 6.445 | slower |
| page/editing/1 | gpu_warm_median_ms | 4.381 (4.392) | 4.799 | slower |
| page/editing/1 | gpu_warm_p95_ms | 4.438 (4.465) | 4.822 | slower |
| page/editing/1 | mask_kib | 165 (165) | 165 | changed |
| page/editing/1 | mask_resources | 40.000 (40.000) | 35.000 | changed |
| page/editing/1 | masks_drawn | 137 (137) | 37.000 | changed |
| page/editing/1 | other_commands | 0.000 (0.000) | 22.000 | changed |
| page/editing/1 | present_cold_ms | 0.617 (0.655) | 0.585 |  |
| page/editing/1 | present_warm_median_ms | 0.474 (0.482) | 0.460 |  |
| page/editing/1 | rects | 102 (102) | 16.000 | changed |
| page/editing/1.25 | compose_cold_ms | 1.920 (1.932) | 1.902 |  |
| page/editing/1.25 | compose_warm_ms | 0.053 (0.053) | 0.047 |  |
| page/editing/1.25 | cpu_median_ms | 10.028 (10.068) | 7.405 | faster |
| page/editing/1.25 | gpu_first_ms | 4.831 (7.773) | 5.659 |  |
| page/editing/1.25 | gpu_warm_median_ms | 5.123 (6.149) | 5.851 |  |
| page/editing/1.25 | gpu_warm_p95_ms | 6.245 (6.403) | 6.477 | within noise |
| page/editing/1.25 | mask_kib | 258 (258) | 257 | changed |
| page/editing/1.25 | mask_resources | 40.000 (40.000) | 35.000 | changed |
| page/editing/1.25 | masks_drawn | 137 (137) | 37.000 | changed |
| page/editing/1.25 | other_commands | 0.000 (0.000) | 22.000 | changed |
| page/editing/1.25 | present_cold_ms | 0.686 (0.704) | 0.640 |  |
| page/editing/1.25 | present_warm_median_ms | 0.478 (0.483) | 0.463 |  |
| page/editing/1.25 | rects | 102 (102) | 16.000 | changed |
| page/editing/2 | compose_cold_ms | 2.836 (2.864) | 2.822 |  |
| page/editing/2 | compose_warm_ms | 0.052 (0.053) | 0.046 |  |
| page/editing/2 | cpu_median_ms | 23.281 (23.353) | 16.668 | faster |
| page/editing/2 | gpu_first_ms | 10.891 (15.568) | 12.711 |  |
| page/editing/2 | gpu_warm_median_ms | 8.964 (9.169) | 10.039 | slower |
| page/editing/2 | gpu_warm_p95_ms | 9.345 (9.473) | 10.446 | slower |
| page/editing/2 | mask_kib | 659 (659) | 658 | changed |
| page/editing/2 | mask_resources | 40.000 (40.000) | 35.000 | changed |
| page/editing/2 | masks_drawn | 137 (137) | 37.000 | changed |
| page/editing/2 | other_commands | 0.000 (0.000) | 22.000 | changed |
| page/editing/2 | present_cold_ms | 0.825 (0.844) | 0.774 |  |
| page/editing/2 | present_warm_median_ms | 0.474 (0.481) | 0.463 |  |
| page/editing/2 | rects | 102 (102) | 16.000 | changed |
| page/families/1 | compose_cold_ms | 0.995 (1.019) | 0.949 |  |
| page/families/1 | compose_warm_ms | 0.057 (0.057) | 0.045 |  |
| page/families/1 | cpu_median_ms | 3.834 (3.853) | 2.640 | faster |
| page/families/1 | gpu_first_ms | 5.150 (5.305) | 5.577 | slower |
| page/families/1 | gpu_warm_median_ms | 4.211 (4.219) | 4.441 | slower |
| page/families/1 | gpu_warm_p95_ms | 4.257 (4.269) | 4.497 | slower |
| page/families/1 | mask_kib | 55.436 (55.436) | 54.729 | changed |
| page/families/1 | mask_resources | 54.000 (54.000) | 46.000 | changed |
| page/families/1 | masks_drawn | 254 (254) | 48.000 | changed |
| page/families/1 | other_commands | 0.000 (0.000) | 49.000 | changed |
| page/families/1 | present_cold_ms | 0.669 (0.734) | 0.564 | faster |
| page/families/1 | present_warm_median_ms | 0.531 (0.545) | 0.494 |  |
| page/families/1 | rects | 161 (161) | 14.000 | changed |
| page/families/1.25 | compose_cold_ms | 1.107 (1.108) | 1.045 | faster |
| page/families/1.25 | compose_warm_ms | 0.058 (0.058) | 0.046 |  |
| page/families/1.25 | cpu_median_ms | 5.998 (6.034) | 4.117 | faster |
| page/families/1.25 | gpu_first_ms | 7.165 (7.279) | 5.019 |  |
| page/families/1.25 | gpu_warm_median_ms | 6.082 (6.091) | 5.228 | faster |
| page/families/1.25 | gpu_warm_p95_ms | 6.145 (6.175) | 6.249 | within noise |
| page/families/1.25 | mask_kib | 89.453 (89.453) | 88.346 | changed |
| page/families/1.25 | mask_resources | 54.000 (54.000) | 46.000 | changed |
| page/families/1.25 | masks_drawn | 254 (254) | 48.000 | changed |
| page/families/1.25 | other_commands | 0.000 (0.000) | 49.000 | changed |
| page/families/1.25 | present_cold_ms | 0.728 (0.746) | 0.618 | faster |
| page/families/1.25 | present_warm_median_ms | 0.541 (0.547) | 0.499 |  |
| page/families/1.25 | rects | 161 (161) | 14.000 | changed |
| page/families/2 | compose_cold_ms | 1.495 (1.517) | 1.394 | faster |
| page/families/2 | compose_warm_ms | 0.057 (0.063) | 0.046 |  |
| page/families/2 | cpu_median_ms | 14.376 (14.413) | 9.488 | faster |
| page/families/2 | gpu_first_ms | 10.130 (10.310) | 11.050 | slower |
| page/families/2 | gpu_warm_median_ms | 8.762 (8.831) | 9.143 | within noise |
| page/families/2 | gpu_warm_p95_ms | 9.121 (9.196) | 9.297 | within noise |
| page/families/2 | mask_kib | 220 (220) | 217 | changed |
| page/families/2 | mask_resources | 54.000 (54.000) | 46.000 | changed |
| page/families/2 | masks_drawn | 254 (254) | 48.000 | changed |
| page/families/2 | other_commands | 0.000 (0.000) | 49.000 | changed |
| page/families/2 | present_cold_ms | 0.845 (0.888) | 0.718 | faster |
| page/families/2 | present_warm_median_ms | 0.538 (0.546) | 0.490 |  |
| page/families/2 | rects | 161 (161) | 14.000 | changed |
| page/fields/1 | compose_cold_ms | 1.160 (1.166) | 1.125 |  |
| page/fields/1 | compose_warm_ms | 0.049 (0.050) | 0.042 |  |
| page/fields/1 | cpu_median_ms | 4.799 (4.839) | 3.243 | faster |
| page/fields/1 | gpu_first_ms | 5.376 (5.554) | 6.427 | slower |
| page/fields/1 | gpu_warm_median_ms | 4.345 (4.355) | 4.798 | slower |
| page/fields/1 | gpu_warm_p95_ms | 4.414 (4.452) | 4.951 | slower |
| page/fields/1 | mask_kib | 82.285 (82.285) | 81.973 | changed |
| page/fields/1 | mask_resources | 42.000 (42.000) | 37.000 | changed |
| page/fields/1 | masks_drawn | 148 (148) | 38.000 | changed |
| page/fields/1 | other_commands | 0.000 (0.000) | 26.000 | changed |
| page/fields/1 | present_cold_ms | 0.578 (0.592) | 0.548 |  |
| page/fields/1 | present_warm_median_ms | 0.476 (0.482) | 0.464 |  |
| page/fields/1 | rects | 100 (100) | 13.000 | changed |
| page/fields/1.25 | compose_cold_ms | 1.325 (1.335) | 1.295 |  |
| page/fields/1.25 | compose_warm_ms | 0.049 (0.050) | 0.043 |  |
| page/fields/1.25 | cpu_median_ms | 7.723 (7.789) | 5.231 | faster |
| page/fields/1.25 | gpu_first_ms | 4.911 (7.789) | 5.575 |  |
| page/fields/1.25 | gpu_warm_median_ms | 6.271 (6.325) | 5.955 |  |
| page/fields/1.25 | gpu_warm_p95_ms | 6.357 (6.391) | 6.857 | slower |
| page/fields/1.25 | mask_kib | 130 (130) | 129 | changed |
| page/fields/1.25 | mask_resources | 42.000 (42.000) | 37.000 | changed |
| page/fields/1.25 | masks_drawn | 148 (148) | 38.000 | changed |
| page/fields/1.25 | other_commands | 0.000 (0.000) | 26.000 | changed |
| page/fields/1.25 | present_cold_ms | 0.625 (0.637) | 0.603 |  |
| page/fields/1.25 | present_warm_median_ms | 0.483 (0.488) | 0.470 |  |
| page/fields/1.25 | rects | 100 (100) | 13.000 | changed |
| page/fields/2 | compose_cold_ms | 1.856 (1.869) | 1.820 |  |
| page/fields/2 | compose_warm_ms | 0.050 (0.054) | 0.043 |  |
| page/fields/2 | cpu_median_ms | 18.235 (18.329) | 11.794 | faster |
| page/fields/2 | gpu_first_ms | 10.674 (14.400) | 12.047 |  |
| page/fields/2 | gpu_warm_median_ms | 9.029 (9.167) | 9.904 | slower |
| page/fields/2 | gpu_warm_p95_ms | 9.327 (9.358) | 10.317 | slower |
| page/fields/2 | mask_kib | 328 (328) | 327 | changed |
| page/fields/2 | mask_resources | 42.000 (42.000) | 37.000 | changed |
| page/fields/2 | masks_drawn | 148 (148) | 38.000 | changed |
| page/fields/2 | other_commands | 0.000 (0.000) | 26.000 | changed |
| page/fields/2 | present_cold_ms | 0.753 (0.759) | 0.699 |  |
| page/fields/2 | present_warm_median_ms | 0.483 (0.485) | 0.475 |  |
| page/fields/2 | rects | 100 (100) | 13.000 | changed |
| page/files/1 | compose_cold_ms | 1.939 (1.958) | 1.911 |  |
| page/files/1 | compose_warm_ms | 0.056 (0.058) | 0.049 |  |
| page/files/1 | cpu_median_ms | 7.553 (7.623) | 5.378 | faster |
| page/files/1 | gpu_first_ms | 5.990 (6.059) | 7.300 | slower |
| page/files/1 | gpu_warm_median_ms | 4.644 (4.691) | 5.413 | slower |
| page/files/1 | gpu_warm_p95_ms | 4.715 (4.757) | 5.485 | slower |
| page/files/1 | mask_kib | 161 (161) | 160 | changed |
| page/files/1 | mask_resources | 56.000 (56.000) | 50.000 | changed |
| page/files/1 | masks_drawn | 161 (161) | 57.000 | changed |
| page/files/1 | other_commands | 0.000 (0.000) | 25.000 | changed |
| page/files/1 | present_cold_ms | 0.651 (0.661) | 0.616 |  |
| page/files/1 | present_warm_median_ms | 0.495 (0.511) | 0.485 |  |
| page/files/1 | rects | 102 (102) | 19.000 | changed |
| page/files/1.25 | compose_cold_ms | 2.198 (2.238) | 2.164 |  |
| page/files/1.25 | compose_warm_ms | 0.057 (0.057) | 0.050 |  |
| page/files/1.25 | cpu_median_ms | 12.156 (12.182) | 8.729 | faster |
| page/files/1.25 | gpu_first_ms | 8.066 (8.495) | 6.413 |  |
| page/files/1.25 | gpu_warm_median_ms | 6.354 (6.625) | 5.604 |  |
| page/files/1.25 | gpu_warm_p95_ms | 6.782 (6.846) | 6.379 |  |
| page/files/1.25 | mask_kib | 251 (251) | 251 | changed |
| page/files/1.25 | mask_resources | 56.000 (56.000) | 50.000 | changed |
| page/files/1.25 | masks_drawn | 161 (161) | 57.000 | changed |
| page/files/1.25 | other_commands | 0.000 (0.000) | 25.000 | changed |
| page/files/1.25 | present_cold_ms | 0.714 (0.742) | 0.683 |  |
| page/files/1.25 | present_warm_median_ms | 0.504 (0.515) | 0.488 |  |
| page/files/1.25 | rects | 102 (102) | 19.000 | changed |
| page/files/2 | compose_cold_ms | 3.177 (3.184) | 3.121 |  |
| page/files/2 | compose_warm_ms | 0.057 (0.059) | 0.051 |  |
| page/files/2 | cpu_median_ms | 28.261 (28.383) | 19.571 | faster |
| page/files/2 | gpu_first_ms | 11.898 (12.012) | 14.092 | slower |
| page/files/2 | gpu_warm_median_ms | 9.584 (9.692) | 10.206 | slower |
| page/files/2 | gpu_warm_p95_ms | 9.829 (10.078) | 10.649 | slower |
| page/files/2 | mask_kib | 637 (637) | 636 | changed |
| page/files/2 | mask_resources | 56.000 (56.000) | 50.000 | changed |
| page/files/2 | masks_drawn | 161 (161) | 57.000 | changed |
| page/files/2 | other_commands | 0.000 (0.000) | 25.000 | changed |
| page/files/2 | present_cold_ms | 0.847 (0.858) | 0.812 |  |
| page/files/2 | present_warm_median_ms | 0.502 (0.516) | 0.488 |  |
| page/files/2 | rects | 102 (102) | 19.000 | changed |
| page/fonts/1 | compose_cold_ms | 1.852 (1.883) | 1.829 |  |
| page/fonts/1 | compose_warm_ms | 0.066 (0.070) | 0.058 |  |
| page/fonts/1 | cpu_median_ms | 6.517 (6.568) | 5.187 | faster |
| page/fonts/1 | gpu_first_ms | 5.488 (5.593) | 6.624 | slower |
| page/fonts/1 | gpu_warm_median_ms | 4.392 (4.424) | 4.918 | slower |
| page/fonts/1 | gpu_warm_p95_ms | 4.448 (4.479) | 5.025 | slower |
| page/fonts/1 | mask_kib | 145 (145) | 145 | changed |
| page/fonts/1 | mask_resources | 85.000 (85.000) | 80.000 | changed |
| page/fonts/1 | masks_drawn | 221 (221) | 85.000 | changed |
| page/fonts/1 | other_commands | 0.000 (0.000) | 34.000 | changed |
| page/fonts/1 | present_cold_ms | 1.053 (1.061) | 1.030 |  |
| page/fonts/1 | present_warm_median_ms | 0.526 (0.534) | 0.513 |  |
| page/fonts/1 | rects | 124 (124) | 14.000 | changed |
| page/fonts/1.25 | compose_cold_ms | 2.103 (2.110) | 2.078 |  |
| page/fonts/1.25 | compose_warm_ms | 0.067 (0.070) | 0.060 |  |
| page/fonts/1.25 | cpu_median_ms | 10.650 (10.718) | 8.433 | faster |
| page/fonts/1.25 | gpu_first_ms | 7.718 (7.834) | 5.809 |  |
| page/fonts/1.25 | gpu_warm_median_ms | 6.074 (6.409) | 6.274 |  |
| page/fonts/1.25 | gpu_warm_p95_ms | 6.408 (6.497) | 6.920 | slower |
| page/fonts/1.25 | mask_kib | 225 (225) | 224 | changed |
| page/fonts/1.25 | mask_resources | 85.000 (85.000) | 80.000 | changed |
| page/fonts/1.25 | masks_drawn | 221 (221) | 85.000 | changed |
| page/fonts/1.25 | other_commands | 0.000 (0.000) | 34.000 | changed |
| page/fonts/1.25 | present_cold_ms | 1.113 (1.171) | 1.101 |  |
| page/fonts/1.25 | present_warm_median_ms | 0.525 (0.536) | 0.511 |  |
| page/fonts/1.25 | rects | 124 (124) | 14.000 | changed |
| page/fonts/2 | compose_cold_ms | 3.010 (3.051) | 2.966 |  |
| page/fonts/2 | compose_warm_ms | 0.067 (0.075) | 0.059 |  |
| page/fonts/2 | cpu_median_ms | 24.484 (24.575) | 18.953 | faster |
| page/fonts/2 | gpu_first_ms | 11.242 (15.706) | 12.774 |  |
| page/fonts/2 | gpu_warm_median_ms | 9.046 (9.241) | 9.702 | within noise |
| page/fonts/2 | gpu_warm_p95_ms | 9.350 (9.401) | 10.205 | slower |
| page/fonts/2 | mask_kib | 571 (571) | 569 | changed |
| page/fonts/2 | mask_resources | 85.000 (85.000) | 80.000 | changed |
| page/fonts/2 | masks_drawn | 221 (221) | 85.000 | changed |
| page/fonts/2 | other_commands | 0.000 (0.000) | 34.000 | changed |
| page/fonts/2 | present_cold_ms | 1.231 (1.238) | 1.208 |  |
| page/fonts/2 | present_warm_median_ms | 0.519 (0.537) | 0.512 |  |
| page/fonts/2 | rects | 124 (124) | 14.000 | changed |
| page/forms/1 | compose_cold_ms | 1.156 (1.165) | 1.132 |  |
| page/forms/1 | compose_warm_ms | 0.049 (0.051) | 0.044 |  |
| page/forms/1 | cpu_median_ms | 4.336 (4.344) | 3.162 | faster |
| page/forms/1 | gpu_first_ms | 5.141 (5.184) | 5.384 | within noise |
| page/forms/1 | gpu_warm_median_ms | 4.111 (4.162) | 4.274 | within noise |
| page/forms/1 | gpu_warm_p95_ms | 4.172 (4.241) | 4.361 | within noise |
| page/forms/1 | mask_kib | 80.889 (80.889) | 80.567 | changed |
| page/forms/1 | mask_resources | 45.000 (45.000) | 39.000 | changed |
| page/forms/1 | masks_drawn | 150 (150) | 40.000 | changed |
| page/forms/1 | other_commands | 0.000 (0.000) | 25.000 | changed |
| page/forms/1 | present_cold_ms | 0.532 (0.538) | 0.494 |  |
| page/forms/1 | present_warm_median_ms | 0.482 (0.496) | 0.479 |  |
| page/forms/1 | rects | 104 (104) | 15.000 | changed |
| page/forms/1.25 | compose_cold_ms | 1.309 (1.334) | 1.286 |  |
| page/forms/1.25 | compose_warm_ms | 0.049 (0.050) | 0.044 |  |
| page/forms/1.25 | cpu_median_ms | 6.902 (7.000) | 4.978 | faster |
| page/forms/1.25 | gpu_first_ms | 7.059 (7.113) | 7.611 | slower |
| page/forms/1.25 | gpu_warm_median_ms | 5.991 (6.038) | 5.813 |  |
| page/forms/1.25 | gpu_warm_p95_ms | 6.057 (6.151) | 6.158 | within noise |
| page/forms/1.25 | mask_kib | 128 (128) | 127 | changed |
| page/forms/1.25 | mask_resources | 45.000 (45.000) | 39.000 | changed |
| page/forms/1.25 | masks_drawn | 150 (150) | 40.000 | changed |
| page/forms/1.25 | other_commands | 0.000 (0.000) | 25.000 | changed |
| page/forms/1.25 | present_cold_ms | 0.594 (0.610) | 0.538 |  |
| page/forms/1.25 | present_warm_median_ms | 0.486 (0.488) | 0.475 |  |
| page/forms/1.25 | rects | 104 (104) | 15.000 | changed |
| page/forms/2 | compose_cold_ms | 1.847 (1.868) | 1.795 |  |
| page/forms/2 | compose_warm_ms | 0.049 (0.050) | 0.044 |  |
| page/forms/2 | cpu_median_ms | 16.379 (16.502) | 11.495 | faster |
| page/forms/2 | gpu_first_ms | 10.149 (10.275) | 10.615 | within noise |
| page/forms/2 | gpu_warm_median_ms | 8.717 (8.752) | 8.963 | within noise |
| page/forms/2 | gpu_warm_p95_ms | 9.016 (9.069) | 9.288 | within noise |
| page/forms/2 | mask_kib | 323 (323) | 321 | changed |
| page/forms/2 | mask_resources | 45.000 (45.000) | 39.000 | changed |
| page/forms/2 | masks_drawn | 150 (150) | 40.000 | changed |
| page/forms/2 | other_commands | 0.000 (0.000) | 25.000 | changed |
| page/forms/2 | present_cold_ms | 0.696 (0.706) | 0.640 | faster |
| page/forms/2 | present_warm_median_ms | 0.490 (0.494) | 0.473 |  |
| page/forms/2 | rects | 104 (104) | 15.000 | changed |
| page/images/1 | compose_cold_ms | 1.489 (1.505) | 1.461 |  |
| page/images/1 | compose_warm_ms | 0.051 (0.051) | 0.045 |  |
| page/images/1 | cpu_median_ms | 8.318 (8.347) | 6.694 | faster |
| page/images/1 | gpu_first_ms | 5.703 (5.772) | 6.614 | slower |
| page/images/1 | gpu_warm_median_ms | 4.461 (4.473) | 4.954 | slower |
| page/images/1 | gpu_warm_p95_ms | 4.521 (4.580) | 5.037 | slower |
| page/images/1 | mask_kib | 135 (135) | 134 | changed |
| page/images/1 | mask_resources | 41.000 (41.000) | 36.000 | changed |
| page/images/1 | masks_drawn | 132 (132) | 36.000 | changed |
| page/images/1 | other_commands | 0.000 (0.000) | 25.000 | changed |
| page/images/1 | present_cold_ms | 0.567 (0.635) | 0.518 |  |
| page/images/1 | present_warm_median_ms | 0.484 (0.498) | 0.471 |  |
| page/images/1 | rects | 97.000 (97.000) | 19.000 | changed |
| page/images/1.25 | compose_cold_ms | 1.730 (1.755) | 1.674 |  |
| page/images/1.25 | compose_warm_ms | 0.051 (0.052) | 0.046 |  |
| page/images/1.25 | cpu_median_ms | 13.351 (13.420) | 10.727 | faster |
| page/images/1.25 | gpu_first_ms | 7.899 (8.002) | 9.322 | slower |
| page/images/1.25 | gpu_warm_median_ms | 6.187 (6.386) | 6.578 | within noise |
| page/images/1.25 | gpu_warm_p95_ms | 6.472 (6.547) | 7.075 | slower |
| page/images/1.25 | mask_kib | 208 (208) | 208 | changed |
| page/images/1.25 | mask_resources | 41.000 (41.000) | 36.000 | changed |
| page/images/1.25 | masks_drawn | 132 (132) | 36.000 | changed |
| page/images/1.25 | other_commands | 0.000 (0.000) | 25.000 | changed |
| page/images/1.25 | present_cold_ms | 0.606 (0.630) | 0.574 |  |
| page/images/1.25 | present_warm_median_ms | 0.487 (0.503) | 0.470 |  |
| page/images/1.25 | rects | 97.000 (97.000) | 19.000 | changed |
| page/images/2 | compose_cold_ms | 2.458 (2.480) | 2.428 |  |
| page/images/2 | compose_warm_ms | 0.051 (0.051) | 0.046 |  |
| page/images/2 | cpu_median_ms | 31.903 (32.040) | 25.269 | faster |
| page/images/2 | gpu_first_ms | 11.003 (11.103) | 12.805 | slower |
| page/images/2 | gpu_warm_median_ms | 9.052 (9.070) | 9.738 | slower |
| page/images/2 | gpu_warm_p95_ms | 9.394 (9.664) | 10.529 | slower |
| page/images/2 | mask_kib | 534 (534) | 533 | changed |
| page/images/2 | mask_resources | 41.000 (41.000) | 36.000 | changed |
| page/images/2 | masks_drawn | 132 (132) | 36.000 | changed |
| page/images/2 | other_commands | 0.000 (0.000) | 25.000 | changed |
| page/images/2 | present_cold_ms | 0.724 (0.731) | 0.671 |  |
| page/images/2 | present_warm_median_ms | 0.491 (0.497) | 0.480 |  |
| page/images/2 | rects | 97.000 (97.000) | 19.000 | changed |
| page/input/1 | compose_cold_ms | 1.959 (2.069) | 1.922 |  |
| page/input/1 | compose_warm_ms | 0.052 (0.064) | 0.044 |  |
| page/input/1 | cpu_median_ms | 7.930 (7.964) | 6.112 | faster |
| page/input/1 | gpu_first_ms | 5.883 (5.996) | 7.463 | slower |
| page/input/1 | gpu_warm_median_ms | 4.597 (4.620) | 5.500 | slower |
| page/input/1 | gpu_warm_p95_ms | 4.649 (4.684) | 5.562 | slower |
| page/input/1 | mask_kib | 178 (178) | 177 | changed |
| page/input/1 | mask_resources | 46.000 (46.000) | 41.000 | changed |
| page/input/1 | masks_drawn | 177 (177) | 45.000 | changed |
| page/input/1 | other_commands | 0.000 (0.000) | 35.000 | changed |
| page/input/1 | present_cold_ms | 0.582 (0.736) | 0.532 |  |
| page/input/1 | present_warm_median_ms | 0.502 (0.510) | 0.485 |  |
| page/input/1 | rects | 123 (123) | 15.000 | changed |
| page/input/1.25 | compose_cold_ms | 2.257 (2.288) | 2.210 |  |
| page/input/1.25 | compose_warm_ms | 0.053 (0.058) | 0.044 |  |
| page/input/1.25 | cpu_median_ms | 13.041 (13.104) | 10.038 | faster |
| page/input/1.25 | gpu_first_ms | 8.382 (8.476) | 6.506 | faster |
| page/input/1.25 | gpu_warm_median_ms | 6.250 (6.385) | 5.684 |  |
| page/input/1.25 | gpu_warm_p95_ms | 6.749 (6.820) | 6.480 |  |
| page/input/1.25 | mask_kib | 278 (278) | 278 | changed |
| page/input/1.25 | mask_resources | 46.000 (46.000) | 41.000 | changed |
| page/input/1.25 | masks_drawn | 177 (177) | 45.000 | changed |
| page/input/1.25 | other_commands | 0.000 (0.000) | 35.000 | changed |
| page/input/1.25 | present_cold_ms | 0.657 (0.733) | 0.593 | faster |
| page/input/1.25 | present_warm_median_ms | 0.501 (0.505) | 0.488 |  |
| page/input/1.25 | rects | 123 (123) | 15.000 | changed |
| page/input/2 | compose_cold_ms | 3.253 (3.333) | 3.217 |  |
| page/input/2 | compose_warm_ms | 0.053 (0.058) | 0.045 |  |
| page/input/2 | cpu_median_ms | 29.968 (30.078) | 22.471 | faster |
| page/input/2 | gpu_first_ms | 11.490 (11.567) | 13.669 | slower |
| page/input/2 | gpu_warm_median_ms | 9.495 (9.608) | 9.907 | within noise |
| page/input/2 | gpu_warm_p95_ms | 9.717 (9.851) | 10.872 | slower |
| page/input/2 | mask_kib | 707 (707) | 706 | changed |
| page/input/2 | mask_resources | 46.000 (46.000) | 41.000 | changed |
| page/input/2 | masks_drawn | 177 (177) | 45.000 | changed |
| page/input/2 | other_commands | 0.000 (0.000) | 35.000 | changed |
| page/input/2 | present_cold_ms | 0.772 (0.788) | 0.717 | faster |
| page/input/2 | present_warm_median_ms | 0.506 (0.514) | 0.484 |  |
| page/input/2 | rects | 123 (123) | 15.000 | changed |
| page/layout/1 | compose_cold_ms | 1.539 (1.704) | 1.501 |  |
| page/layout/1 | compose_warm_ms | 0.058 (0.064) | 0.052 |  |
| page/layout/1 | cpu_median_ms | 6.215 (6.296) | 4.337 | faster |
| page/layout/1 | gpu_first_ms | 4.753 (6.505) | 4.577 |  |
| page/layout/1 | gpu_warm_median_ms | 3.051 (4.291) | 3.173 |  |
| page/layout/1 | gpu_warm_p95_ms | 5.038 (6.272) | 4.299 |  |
| page/layout/1 | mask_kib | 111 (111) | 111 | changed |
| page/layout/1 | mask_resources | 46.000 (46.000) | 41.000 | changed |
| page/layout/1 | masks_drawn | 131 (131) | 43.000 | changed |
| page/layout/1 | other_commands | 0.000 (0.000) | 23.000 | changed |
| page/layout/1 | present_cold_ms | 1.373 (1.539) | 1.327 |  |
| page/layout/1 | present_warm_median_ms | 0.518 (0.528) | 0.482 |  |
| page/layout/1 | rects | 99.000 (99.000) | 28.000 | changed |
| page/layout/1.25 | compose_cold_ms | 1.576 (1.594) | 1.543 |  |
| page/layout/1.25 | compose_warm_ms | 0.051 (0.055) | 0.045 |  |
| page/layout/1.25 | cpu_median_ms | 9.335 (9.440) | 6.502 | faster |
| page/layout/1.25 | gpu_first_ms | 7.918 (7.966) | 9.398 | slower |
| page/layout/1.25 | gpu_warm_median_ms | 6.182 (6.501) | 6.542 | within noise |
| page/layout/1.25 | gpu_warm_p95_ms | 6.496 (6.585) | 6.957 | slower |
| page/layout/1.25 | mask_kib | 172 (172) | 171 | changed |
| page/layout/1.25 | mask_resources | 46.000 (46.000) | 41.000 | changed |
| page/layout/1.25 | masks_drawn | 131 (131) | 43.000 | changed |
| page/layout/1.25 | other_commands | 0.000 (0.000) | 23.000 | changed |
| page/layout/1.25 | present_cold_ms | 0.956 (0.986) | 0.906 |  |
| page/layout/1.25 | present_warm_median_ms | 0.474 (0.491) | 0.467 |  |
| page/layout/1.25 | rects | 99.000 (99.000) | 28.000 | changed |
| page/layout/2 | compose_cold_ms | 2.277 (2.298) | 2.210 |  |
| page/layout/2 | compose_warm_ms | 0.051 (0.052) | 0.045 |  |
| page/layout/2 | cpu_median_ms | 21.860 (21.958) | 14.672 | faster |
| page/layout/2 | gpu_first_ms | 11.039 (11.234) | 13.068 | slower |
| page/layout/2 | gpu_warm_median_ms | 9.252 (9.304) | 9.945 | slower |
| page/layout/2 | gpu_warm_p95_ms | 9.535 (9.665) | 10.527 | slower |
| page/layout/2 | mask_kib | 440 (440) | 439 | changed |
| page/layout/2 | mask_resources | 46.000 (46.000) | 41.000 | changed |
| page/layout/2 | masks_drawn | 131 (131) | 43.000 | changed |
| page/layout/2 | other_commands | 0.000 (0.000) | 23.000 | changed |
| page/layout/2 | present_cold_ms | 1.090 (1.100) | 1.021 |  |
| page/layout/2 | present_warm_median_ms | 0.475 (0.480) | 0.463 |  |
| page/layout/2 | rects | 99.000 (99.000) | 28.000 | changed |
| page/live/1 | compose_cold_ms | 2.003 (2.070) | 1.915 |  |
| page/live/1 | compose_warm_ms | 0.110 (0.111) | 0.089 |  |
| page/live/1 | cpu_median_ms | 8.188 (8.233) | 6.327 | faster |
| page/live/1 | gpu_first_ms | 6.249 (6.383) | 6.688 | within noise |
| page/live/1 | gpu_warm_median_ms | 4.690 (4.876) | 5.126 | slower |
| page/live/1 | gpu_warm_p95_ms | 4.896 (4.935) | 5.194 | slower |
| page/live/1 | mask_kib | 192 (192) | 191 | changed |
| page/live/1 | mask_resources | 102 (102) | 94.000 | changed |
| page/live/1 | masks_drawn | 446 (446) | 94.000 | changed |
| page/live/1 | other_commands | 0.000 (0.000) | 88.000 | changed |
| page/live/1 | present_cold_ms | 1.567 (1.634) | 1.462 | faster |
| page/live/1 | present_warm_median_ms | 0.636 (0.642) | 0.616 |  |
| page/live/1 | rects | 343 (343) | 30.000 | changed |
| page/live/1.25 | compose_cold_ms | 2.218 (2.232) | 2.143 |  |
| page/live/1.25 | compose_warm_ms | 0.110 (0.112) | 0.090 |  |
| page/live/1.25 | cpu_median_ms | 13.638 (13.653) | 10.645 | faster |
| page/live/1.25 | gpu_first_ms | 8.350 (8.691) | 5.789 |  |
| page/live/1.25 | gpu_warm_median_ms | 6.824 (6.965) | 6.612 |  |
| page/live/1.25 | gpu_warm_p95_ms | 6.998 (7.006) | 7.087 | within noise |
| page/live/1.25 | mask_kib | 298 (298) | 297 | changed |
| page/live/1.25 | mask_resources | 102 (102) | 94.000 | changed |
| page/live/1.25 | masks_drawn | 446 (446) | 94.000 | changed |
| page/live/1.25 | other_commands | 0.000 (0.000) | 88.000 | changed |
| page/live/1.25 | present_cold_ms | 1.637 (1.669) | 1.473 | faster |
| page/live/1.25 | present_warm_median_ms | 0.638 (0.640) | 0.612 |  |
| page/live/1.25 | rects | 352 (352) | 30.000 | changed |
| page/live/2 | compose_cold_ms | 3.120 (3.131) | 3.017 |  |
| page/live/2 | compose_warm_ms | 0.110 (0.111) | 0.090 |  |
| page/live/2 | cpu_median_ms | 32.045 (32.273) | 24.229 | faster |
| page/live/2 | gpu_first_ms | 11.540 (11.988) | 12.833 | slower |
| page/live/2 | gpu_warm_median_ms | 9.885 (9.948) | 9.996 | within noise |
| page/live/2 | gpu_warm_p95_ms | 10.074 (10.267) | 10.645 | within noise |
| page/live/2 | mask_kib | 761 (761) | 758 | changed |
| page/live/2 | mask_resources | 102 (102) | 94.000 | changed |
| page/live/2 | masks_drawn | 446 (446) | 94.000 | changed |
| page/live/2 | other_commands | 0.000 (0.000) | 88.000 | changed |
| page/live/2 | present_cold_ms | 1.792 (1.855) | 1.656 |  |
| page/live/2 | present_warm_median_ms | 0.630 (0.646) | 0.619 |  |
| page/live/2 | rects | 352 (352) | 30.000 | changed |
| page/menus/1 | compose_cold_ms | 1.406 (1.433) | 1.393 |  |
| page/menus/1 | compose_warm_ms | 0.044 (0.049) | 0.040 |  |
| page/menus/1 | cpu_median_ms | 5.522 (5.599) | 4.262 | faster |
| page/menus/1 | gpu_first_ms | 4.805 (5.087) | 5.176 | within noise |
| page/menus/1 | gpu_warm_median_ms | 3.982 (4.061) | 4.097 | within noise |
| page/menus/1 | gpu_warm_p95_ms | 4.103 (4.140) | 4.160 | within noise |
| page/menus/1 | mask_kib | 136 (136) | 136 | changed |
| page/menus/1 | mask_resources | 31.000 (31.000) | 27.000 | changed |
| page/menus/1 | masks_drawn | 84.000 (84.000) | 28.000 | changed |
| page/menus/1 | other_commands | 0.000 (0.000) | 14.000 | changed |
| page/menus/1 | present_cold_ms | 0.504 (0.639) | 0.467 |  |
| page/menus/1 | present_warm_median_ms | 0.444 (0.457) | 0.442 |  |
| page/menus/1 | rects | 58.000 (58.000) | 15.000 | changed |
| page/menus/1.25 | compose_cold_ms | 1.605 (1.610) | 1.586 |  |
| page/menus/1.25 | compose_warm_ms | 0.045 (0.049) | 0.041 |  |
| page/menus/1.25 | cpu_median_ms | 8.888 (8.892) | 6.902 | faster |
| page/menus/1.25 | gpu_first_ms | 6.834 (7.075) | 4.502 |  |
| page/menus/1.25 | gpu_warm_median_ms | 5.846 (5.911) | 5.935 | within noise |
| page/menus/1.25 | gpu_warm_p95_ms | 5.983 (6.057) | 6.034 |  |
| page/menus/1.25 | mask_kib | 211 (211) | 211 | changed |
| page/menus/1.25 | mask_resources | 31.000 (31.000) | 27.000 | changed |
| page/menus/1.25 | masks_drawn | 84.000 (84.000) | 28.000 | changed |
| page/menus/1.25 | other_commands | 0.000 (0.000) | 14.000 | changed |
| page/menus/1.25 | present_cold_ms | 0.569 (0.580) | 0.510 |  |
| page/menus/1.25 | present_warm_median_ms | 0.439 (0.442) | 0.436 |  |
| page/menus/1.25 | rects | 58.000 (58.000) | 15.000 | changed |
| page/menus/2 | compose_cold_ms | 2.347 (2.387) | 2.305 |  |
| page/menus/2 | compose_warm_ms | 0.045 (0.047) | 0.041 |  |
| page/menus/2 | cpu_median_ms | 20.857 (20.995) | 15.774 | faster |
| page/menus/2 | gpu_first_ms | 10.043 (10.101) | 10.237 | within noise |
| page/menus/2 | gpu_warm_median_ms | 8.588 (8.639) | 8.631 |  |
| page/menus/2 | gpu_warm_p95_ms | 8.918 (8.951) | 8.959 | within noise |
| page/menus/2 | mask_kib | 541 (541) | 540 | changed |
| page/menus/2 | mask_resources | 31.000 (31.000) | 27.000 | changed |
| page/menus/2 | masks_drawn | 84.000 (84.000) | 28.000 | changed |
| page/menus/2 | other_commands | 0.000 (0.000) | 14.000 | changed |
| page/menus/2 | present_cold_ms | 0.653 (0.677) | 0.627 |  |
| page/menus/2 | present_warm_median_ms | 0.447 (0.450) | 0.435 |  |
| page/menus/2 | rects | 58.000 (58.000) | 15.000 | changed |
| page/meters/1 | compose_cold_ms | 0.912 (0.940) | 0.877 |  |
| page/meters/1 | compose_warm_ms | 0.046 (0.047) | 0.041 |  |
| page/meters/1 | cpu_median_ms | 4.103 (4.117) | 2.833 | faster |
| page/meters/1 | gpu_first_ms | 5.038 (5.168) | 5.168 | within noise |
| page/meters/1 | gpu_warm_median_ms | 4.055 (4.064) | 4.078 | within noise |
| page/meters/1 | gpu_warm_p95_ms | 4.122 (4.158) | 4.168 | within noise |
| page/meters/1 | mask_kib | 79.273 (79.273) | 78.633 | changed |
| page/meters/1 | mask_resources | 32.000 (32.000) | 26.000 | changed |
| page/meters/1 | masks_drawn | 106 (106) | 26.000 | changed |
| page/meters/1 | other_commands | 0.000 (0.000) | 20.000 | changed |
| page/meters/1 | present_cold_ms | 0.520 (0.534) | 0.501 |  |
| page/meters/1 | present_warm_median_ms | 0.458 (0.469) | 0.437 |  |
| page/meters/1 | rects | 82.000 (82.000) | 26.000 | changed |
| page/meters/1.25 | compose_cold_ms | 1.043 (1.080) | 0.991 |  |
| page/meters/1.25 | compose_warm_ms | 0.047 (0.049) | 0.042 |  |
| page/meters/1.25 | cpu_median_ms | 6.571 (6.635) | 4.540 | faster |
| page/meters/1.25 | gpu_first_ms | 6.907 (7.082) | 4.519 |  |
| page/meters/1.25 | gpu_warm_median_ms | 5.914 (5.950) | 5.590 | faster |
| page/meters/1.25 | gpu_warm_p95_ms | 5.990 (6.013) | 6.004 |  |
| page/meters/1.25 | mask_kib | 122 (122) | 121 | changed |
| page/meters/1.25 | mask_resources | 32.000 (32.000) | 26.000 | changed |
| page/meters/1.25 | masks_drawn | 106 (106) | 26.000 | changed |
| page/meters/1.25 | other_commands | 0.000 (0.000) | 20.000 | changed |
| page/meters/1.25 | present_cold_ms | 0.570 (0.597) | 0.540 |  |
| page/meters/1.25 | present_warm_median_ms | 0.460 (0.466) | 0.444 |  |
| page/meters/1.25 | rects | 82.000 (82.000) | 26.000 | changed |
| page/meters/2 | compose_cold_ms | 1.481 (1.507) | 1.422 |  |
| page/meters/2 | compose_warm_ms | 0.046 (0.046) | 0.041 |  |
| page/meters/2 | cpu_median_ms | 15.641 (15.750) | 10.361 | faster |
| page/meters/2 | gpu_first_ms | 10.113 (10.212) | 10.237 | within noise |
| page/meters/2 | gpu_warm_median_ms | 8.647 (8.714) | 8.638 |  |
| page/meters/2 | gpu_warm_p95_ms | 8.955 (8.978) | 8.938 |  |
| page/meters/2 | mask_kib | 314 (314) | 312 | changed |
| page/meters/2 | mask_resources | 32.000 (32.000) | 26.000 | changed |
| page/meters/2 | masks_drawn | 106 (106) | 26.000 | changed |
| page/meters/2 | other_commands | 0.000 (0.000) | 20.000 | changed |
| page/meters/2 | present_cold_ms | 0.676 (0.684) | 0.636 |  |
| page/meters/2 | present_warm_median_ms | 0.455 (0.461) | 0.446 |  |
| page/meters/2 | rects | 82.000 (82.000) | 26.000 | changed |
| page/navigation/1 | compose_cold_ms | 2.360 (2.402) | 2.342 |  |
| page/navigation/1 | compose_warm_ms | 0.053 (0.054) | 0.047 |  |
| page/navigation/1 | cpu_median_ms | 8.760 (8.767) | 6.890 | faster |
| page/navigation/1 | gpu_first_ms | 5.870 (5.888) | 7.373 | slower |
| page/navigation/1 | gpu_warm_median_ms | 4.573 (4.611) | 5.278 | slower |
| page/navigation/1 | gpu_warm_p95_ms | 4.636 (4.642) | 5.444 | slower |
| page/navigation/1 | mask_kib | 216 (216) | 216 | changed |
| page/navigation/1 | mask_resources | 51.000 (51.000) | 46.000 | changed |
| page/navigation/1 | masks_drawn | 164 (164) | 50.000 | changed |
| page/navigation/1 | other_commands | 0.000 (0.000) | 31.000 | changed |
| page/navigation/1 | present_cold_ms | 0.617 (0.639) | 0.603 |  |
| page/navigation/1 | present_warm_median_ms | 0.489 (0.504) | 0.483 |  |
| page/navigation/1 | rects | 115 (115) | 20.000 | changed |
| page/navigation/1.25 | compose_cold_ms | 2.729 (2.777) | 2.702 |  |
| page/navigation/1.25 | compose_warm_ms | 0.055 (0.055) | 0.047 |  |
| page/navigation/1.25 | cpu_median_ms | 14.383 (14.436) | 11.383 | faster |
| page/navigation/1.25 | gpu_first_ms | 8.439 (8.462) | 6.322 |  |
| page/navigation/1.25 | gpu_warm_median_ms | 6.358 (6.412) | 5.789 |  |
| page/navigation/1.25 | gpu_warm_p95_ms | 6.729 (6.832) | 6.466 |  |
| page/navigation/1.25 | mask_kib | 340 (340) | 339 | changed |
| page/navigation/1.25 | mask_resources | 51.000 (51.000) | 46.000 | changed |
| page/navigation/1.25 | masks_drawn | 164 (164) | 50.000 | changed |
| page/navigation/1.25 | other_commands | 0.000 (0.000) | 31.000 | changed |
| page/navigation/1.25 | present_cold_ms | 0.693 (0.727) | 0.650 |  |
| page/navigation/1.25 | present_warm_median_ms | 0.492 (0.506) | 0.488 |  |
| page/navigation/1.25 | rects | 115 (115) | 20.000 | changed |
| page/navigation/2 | compose_cold_ms | 4.005 (4.076) | 3.956 |  |
| page/navigation/2 | compose_warm_ms | 0.054 (0.054) | 0.047 |  |
| page/navigation/2 | cpu_median_ms | 32.754 (32.858) | 25.173 | faster |
| page/navigation/2 | gpu_first_ms | 11.491 (11.743) | 14.164 | slower |
| page/navigation/2 | gpu_warm_median_ms | 9.500 (9.599) | 10.432 | slower |
| page/navigation/2 | gpu_warm_p95_ms | 9.677 (9.828) | 10.973 | slower |
| page/navigation/2 | mask_kib | 863 (863) | 861 | changed |
| page/navigation/2 | mask_resources | 51.000 (51.000) | 46.000 | changed |
| page/navigation/2 | masks_drawn | 164 (164) | 50.000 | changed |
| page/navigation/2 | other_commands | 0.000 (0.000) | 31.000 | changed |
| page/navigation/2 | present_cold_ms | 0.838 (0.847) | 0.789 |  |
| page/navigation/2 | present_warm_median_ms | 0.498 (0.506) | 0.490 |  |
| page/navigation/2 | rects | 115 (115) | 20.000 | changed |
| page/numbers/1 | compose_cold_ms | 2.005 (2.015) | 1.981 |  |
| page/numbers/1 | compose_warm_ms | 0.050 (0.055) | 0.043 |  |
| page/numbers/1 | cpu_median_ms | 6.939 (6.966) | 5.187 | faster |
| page/numbers/1 | gpu_first_ms | 5.717 (5.803) | 6.816 | slower |
| page/numbers/1 | gpu_warm_median_ms | 4.494 (4.499) | 5.086 | slower |
| page/numbers/1 | gpu_warm_p95_ms | 4.544 (4.589) | 5.157 | slower |
| page/numbers/1 | mask_kib | 154 (154) | 154 | changed |
| page/numbers/1 | mask_resources | 48.000 (48.000) | 43.000 | changed |
| page/numbers/1 | masks_drawn | 149 (149) | 45.000 | changed |
| page/numbers/1 | other_commands | 0.000 (0.000) | 22.000 | changed |
| page/numbers/1 | present_cold_ms | 0.597 (0.625) | 0.566 |  |
| page/numbers/1 | present_warm_median_ms | 0.488 (0.499) | 0.463 |  |
| page/numbers/1 | rects | 104 (104) | 14.000 | changed |
| page/numbers/1.25 | compose_cold_ms | 2.277 (2.318) | 2.245 |  |
| page/numbers/1.25 | compose_warm_ms | 0.050 (0.054) | 0.043 |  |
| page/numbers/1.25 | cpu_median_ms | 11.392 (11.457) | 8.554 | faster |
| page/numbers/1.25 | gpu_first_ms | 8.061 (8.121) | 5.856 | faster |
| page/numbers/1.25 | gpu_warm_median_ms | 6.076 (6.506) | 5.934 |  |
| page/numbers/1.25 | gpu_warm_p95_ms | 6.568 (6.607) | 6.827 | within noise |
| page/numbers/1.25 | mask_kib | 243 (243) | 242 | changed |
| page/numbers/1.25 | mask_resources | 48.000 (48.000) | 43.000 | changed |
| page/numbers/1.25 | masks_drawn | 149 (149) | 45.000 | changed |
| page/numbers/1.25 | other_commands | 0.000 (0.000) | 22.000 | changed |
| page/numbers/1.25 | present_cold_ms | 0.656 (0.675) | 0.613 |  |
| page/numbers/1.25 | present_warm_median_ms | 0.488 (0.489) | 0.469 |  |
| page/numbers/1.25 | rects | 104 (104) | 14.000 | changed |
| page/numbers/2 | compose_cold_ms | 3.285 (3.315) | 3.246 |  |
| page/numbers/2 | compose_warm_ms | 0.050 (0.052) | 0.044 |  |
| page/numbers/2 | cpu_median_ms | 26.180 (26.287) | 19.044 | faster |
| page/numbers/2 | gpu_first_ms | 11.122 (11.313) | 13.091 | slower |
| page/numbers/2 | gpu_warm_median_ms | 9.269 (9.384) | 10.081 | slower |
| page/numbers/2 | gpu_warm_p95_ms | 9.522 (9.711) | 10.452 | slower |
| page/numbers/2 | mask_kib | 616 (616) | 615 | changed |
| page/numbers/2 | mask_resources | 48.000 (48.000) | 43.000 | changed |
| page/numbers/2 | masks_drawn | 149 (149) | 45.000 | changed |
| page/numbers/2 | other_commands | 0.000 (0.000) | 22.000 | changed |
| page/numbers/2 | present_cold_ms | 0.780 (0.793) | 0.735 |  |
| page/numbers/2 | present_warm_median_ms | 0.491 (0.493) | 0.473 |  |
| page/numbers/2 | rects | 104 (104) | 14.000 | changed |
| page/overlay/1 | compose_cold_ms | 1.491 (1.500) | 1.455 |  |
| page/overlay/1 | compose_warm_ms | 0.046 (0.047) | 0.041 |  |
| page/overlay/1 | cpu_median_ms | 6.277 (6.295) | 4.876 | faster |
| page/overlay/1 | gpu_first_ms | 5.177 (5.433) | 5.951 | slower |
| page/overlay/1 | gpu_warm_median_ms | 4.213 (4.231) | 4.564 | slower |
| page/overlay/1 | gpu_warm_p95_ms | 4.278 (4.339) | 4.636 | slower |
| page/overlay/1 | mask_kib | 155 (155) | 154 | changed |
| page/overlay/1 | mask_resources | 34.000 (34.000) | 27.000 | changed |
| page/overlay/1 | masks_drawn | 111 (111) | 27.000 | changed |
| page/overlay/1 | other_commands | 0.000 (0.000) | 21.000 | changed |
| page/overlay/1 | present_cold_ms | 0.485 (0.508) | 0.449 |  |
| page/overlay/1 | present_warm_median_ms | 0.457 (0.466) | 0.443 |  |
| page/overlay/1 | rects | 73.000 (73.000) | 13.000 | changed |
| page/overlay/1.25 | compose_cold_ms | 1.729 (1.755) | 1.679 |  |
| page/overlay/1.25 | compose_warm_ms | 0.046 (0.046) | 0.041 |  |
| page/overlay/1.25 | cpu_median_ms | 10.370 (10.392) | 8.068 | faster |
| page/overlay/1.25 | gpu_first_ms | 7.438 (7.474) | 8.419 | slower |
| page/overlay/1.25 | gpu_warm_median_ms | 6.179 (6.195) | 6.222 | within noise |
| page/overlay/1.25 | gpu_warm_p95_ms | 6.238 (6.262) | 6.684 | slower |
| page/overlay/1.25 | mask_kib | 241 (241) | 240 | changed |
| page/overlay/1.25 | mask_resources | 34.000 (34.000) | 27.000 | changed |
| page/overlay/1.25 | masks_drawn | 111 (111) | 27.000 | changed |
| page/overlay/1.25 | other_commands | 0.000 (0.000) | 21.000 | changed |
| page/overlay/1.25 | present_cold_ms | 0.532 (0.545) | 0.494 |  |
| page/overlay/1.25 | present_warm_median_ms | 0.458 (0.464) | 0.439 |  |
| page/overlay/1.25 | rects | 73.000 (73.000) | 13.000 | changed |
| page/overlay/2 | compose_cold_ms | 2.529 (2.542) | 2.460 |  |
| page/overlay/2 | compose_warm_ms | 0.046 (0.047) | 0.041 |  |
| page/overlay/2 | cpu_median_ms | 23.759 (23.902) | 18.071 | faster |
| page/overlay/2 | gpu_first_ms | 10.599 (10.629) | 11.545 | slower |
| page/overlay/2 | gpu_warm_median_ms | 8.917 (8.978) | 9.415 | within noise |
| page/overlay/2 | gpu_warm_p95_ms | 9.278 (9.377) | 9.759 | within noise |
| page/overlay/2 | mask_kib | 617 (617) | 614 | changed |
| page/overlay/2 | mask_resources | 34.000 (34.000) | 27.000 | changed |
| page/overlay/2 | masks_drawn | 111 (111) | 27.000 | changed |
| page/overlay/2 | other_commands | 0.000 (0.000) | 21.000 | changed |
| page/overlay/2 | present_cold_ms | 0.654 (0.684) | 0.599 |  |
| page/overlay/2 | present_warm_median_ms | 0.456 (0.470) | 0.443 |  |
| page/overlay/2 | rects | 73.000 (73.000) | 13.000 | changed |
| page/profiling/1 | compose_cold_ms | 1.031 (1.037) | 1.019 |  |
| page/profiling/1 | compose_warm_ms | 0.046 (0.046) | 0.041 |  |
| page/profiling/1 | cpu_median_ms | 4.563 (4.592) | 3.260 | faster |
| page/profiling/1 | gpu_first_ms | 5.174 (5.215) | 5.707 | slower |
| page/profiling/1 | gpu_warm_median_ms | 4.012 (4.113) | 4.422 | slower |
| page/profiling/1 | gpu_warm_p95_ms | 4.097 (4.226) | 4.516 | slower |
| page/profiling/1 | mask_kib | 92.729 (92.729) | 92.416 | changed |
| page/profiling/1 | mask_resources | 32.000 (32.000) | 27.000 | changed |
| page/profiling/1 | masks_drawn | 107 (107) | 27.000 | changed |
| page/profiling/1 | other_commands | 0.000 (0.000) | 20.000 | changed |
| page/profiling/1 | present_cold_ms | 0.475 (0.489) | 0.450 |  |
| page/profiling/1 | present_warm_median_ms | 0.455 (0.456) | 0.436 |  |
| page/profiling/1 | rects | 75.000 (75.000) | 13.000 | changed |
| page/profiling/1.25 | compose_cold_ms | 1.170 (1.182) | 1.141 |  |
| page/profiling/1.25 | compose_warm_ms | 0.046 (0.046) | 0.041 |  |
| page/profiling/1.25 | cpu_median_ms | 7.446 (7.486) | 5.283 | faster |
| page/profiling/1.25 | gpu_first_ms | 7.170 (7.220) | 7.899 | slower |
| page/profiling/1.25 | gpu_warm_median_ms | 6.025 (6.026) | 6.408 | slower |
| page/profiling/1.25 | gpu_warm_p95_ms | 6.087 (6.106) | 6.546 | slower |
| page/profiling/1.25 | mask_kib | 143 (143) | 143 | changed |
| page/profiling/1.25 | mask_resources | 32.000 (32.000) | 27.000 | changed |
| page/profiling/1.25 | masks_drawn | 107 (107) | 27.000 | changed |
| page/profiling/1.25 | other_commands | 0.000 (0.000) | 20.000 | changed |
| page/profiling/1.25 | present_cold_ms | 0.521 (0.551) | 0.498 |  |
| page/profiling/1.25 | present_warm_median_ms | 0.458 (0.463) | 0.442 |  |
| page/profiling/1.25 | rects | 75.000 (75.000) | 13.000 | changed |
| page/profiling/2 | compose_cold_ms | 1.697 (1.905) | 1.658 |  |
| page/profiling/2 | compose_warm_ms | 0.046 (0.047) | 0.041 |  |
| page/profiling/2 | cpu_median_ms | 17.448 (17.649) | 12.059 | faster |
| page/profiling/2 | gpu_first_ms | 10.274 (10.319) | 11.117 | slower |
| page/profiling/2 | gpu_warm_median_ms | 8.779 (8.810) | 9.176 | within noise |
| page/profiling/2 | gpu_warm_p95_ms | 9.044 (9.088) | 9.304 | within noise |
| page/profiling/2 | mask_kib | 368 (368) | 366 | changed |
| page/profiling/2 | mask_resources | 32.000 (32.000) | 27.000 | changed |
| page/profiling/2 | masks_drawn | 107 (107) | 27.000 | changed |
| page/profiling/2 | other_commands | 0.000 (0.000) | 20.000 | changed |
| page/profiling/2 | present_cold_ms | 0.629 (0.664) | 0.596 |  |
| page/profiling/2 | present_warm_median_ms | 0.457 (0.462) | 0.445 |  |
| page/profiling/2 | rects | 75.000 (75.000) | 13.000 | changed |
| page/progress/1 | compose_cold_ms | 1.859 (1.869) | 1.824 |  |
| page/progress/1 | compose_warm_ms | 0.054 (0.054) | 0.045 |  |
| page/progress/1 | cpu_median_ms | 7.146 (7.177) | 5.641 | faster |
| page/progress/1 | gpu_first_ms | 5.635 (5.676) | 6.554 | slower |
| page/progress/1 | gpu_warm_median_ms | 4.345 (4.479) | 4.998 | slower |
| page/progress/1 | gpu_warm_p95_ms | 4.468 (4.517) | 5.094 | slower |
| page/progress/1 | mask_kib | 172 (172) | 172 | changed |
| page/progress/1 | mask_resources | 56.000 (56.000) | 48.000 | changed |
| page/progress/1 | masks_drawn | 181 (181) | 49.000 | changed |
| page/progress/1 | other_commands | 0.000 (0.000) | 35.000 | changed |
| page/progress/1 | present_cold_ms | 0.657 (0.682) | 0.597 |  |
| page/progress/1 | present_warm_median_ms | 0.515 (0.522) | 0.497 |  |
| page/progress/1 | rects | 120 (120) | 20.000 | changed |
| page/progress/1.25 | compose_cold_ms | 2.141 (2.165) | 2.085 |  |
| page/progress/1.25 | compose_warm_ms | 0.055 (0.055) | 0.046 |  |
| page/progress/1.25 | cpu_median_ms | 11.583 (11.624) | 9.170 | faster |
| page/progress/1.25 | gpu_first_ms | 7.424 (7.794) | 5.818 |  |
| page/progress/1.25 | gpu_warm_median_ms | 5.838 (6.416) | 5.605 |  |
| page/progress/1.25 | gpu_warm_p95_ms | 6.417 (6.517) | 6.128 |  |
| page/progress/1.25 | mask_kib | 267 (267) | 266 | changed |
| page/progress/1.25 | mask_resources | 56.000 (56.000) | 48.000 | changed |
| page/progress/1.25 | masks_drawn | 183 (183) | 49.000 | changed |
| page/progress/1.25 | other_commands | 0.000 (0.000) | 35.000 | changed |
| page/progress/1.25 | present_cold_ms | 0.705 (0.748) | 0.677 |  |
| page/progress/1.25 | present_warm_median_ms | 0.512 (0.535) | 0.497 |  |
| page/progress/1.25 | rects | 121 (121) | 20.000 | changed |
| page/progress/2 | compose_cold_ms | 3.074 (3.089) | 2.988 |  |
| page/progress/2 | compose_warm_ms | 0.054 (0.055) | 0.045 |  |
| page/progress/2 | cpu_median_ms | 26.849 (26.984) | 20.767 | faster |
| page/progress/2 | gpu_first_ms | 11.163 (15.618) | 12.959 |  |
| page/progress/2 | gpu_warm_median_ms | 9.051 (9.248) | 9.903 | slower |
| page/progress/2 | gpu_warm_p95_ms | 9.401 (9.523) | 10.500 | slower |
| page/progress/2 | mask_kib | 682 (682) | 679 | changed |
| page/progress/2 | mask_resources | 56.000 (56.000) | 48.000 | changed |
| page/progress/2 | masks_drawn | 183 (183) | 49.000 | changed |
| page/progress/2 | other_commands | 0.000 (0.000) | 35.000 | changed |
| page/progress/2 | present_cold_ms | 0.870 (0.883) | 0.791 | faster |
| page/progress/2 | present_warm_median_ms | 0.511 (0.534) | 0.511 |  |
| page/progress/2 | rects | 120 (120) | 20.000 | changed |
| page/properties/1 | compose_cold_ms | 2.090 (2.092) | 1.598 | faster |
| page/properties/1 | compose_warm_ms | 0.054 (0.060) | 0.046 |  |
| page/properties/1 | cpu_median_ms | 6.543 (6.570) | 4.746 | faster |
| page/properties/1 | gpu_first_ms | 5.935 (5.985) | 7.399 | slower |
| page/properties/1 | gpu_warm_median_ms | 4.620 (4.653) | 5.376 | slower |
| page/properties/1 | gpu_warm_p95_ms | 4.688 (4.694) | 5.643 | slower |
| page/properties/1 | mask_kib | 126 (126) | 123 | changed |
| page/properties/1 | mask_resources | 65.000 (65.000) | 56.000 | changed |
| page/properties/1 | masks_drawn | 192 (192) | 60.000 | changed |
| page/properties/1 | other_commands | 0.000 (0.000) | 34.000 | changed |
| page/properties/1 | present_cold_ms | 0.918 (0.948) | 0.663 | faster |
| page/properties/1 | present_warm_median_ms | 0.517 (0.525) | 0.496 |  |
| page/properties/1 | rects | 121 (121) | 17.000 | changed |
| page/properties/1.25 | compose_cold_ms | 2.493 (2.508) | 1.796 | faster |
| page/properties/1.25 | compose_warm_ms | 0.055 (0.063) | 0.046 |  |
| page/properties/1.25 | cpu_median_ms | 10.553 (10.593) | 7.576 | faster |
| page/properties/1.25 | gpu_first_ms | 8.310 (8.504) | 6.574 | faster |
| page/properties/1.25 | gpu_warm_median_ms | 6.384 (6.741) | 5.626 | faster |
| page/properties/1.25 | gpu_warm_p95_ms | 6.766 (6.789) | 6.204 | faster |
| page/properties/1.25 | mask_kib | 201 (201) | 196 | changed |
| page/properties/1.25 | mask_resources | 65.000 (65.000) | 56.000 | changed |
| page/properties/1.25 | masks_drawn | 192 (192) | 60.000 | changed |
| page/properties/1.25 | other_commands | 0.000 (0.000) | 34.000 | changed |
| page/properties/1.25 | present_cold_ms | 0.993 (1.037) | 0.720 | faster |
| page/properties/1.25 | present_warm_median_ms | 0.516 (0.523) | 0.498 |  |
| page/properties/1.25 | rects | 121 (121) | 17.000 | changed |
| page/properties/2 | compose_cold_ms | 4.093 (4.106) | 2.551 | faster |
| page/properties/2 | compose_warm_ms | 0.055 (0.057) | 0.047 |  |
| page/properties/2 | cpu_median_ms | 24.725 (24.825) | 17.408 | faster |
| page/properties/2 | gpu_first_ms | 11.542 (11.561) | 14.296 | slower |
| page/properties/2 | gpu_warm_median_ms | 9.489 (9.586) | 10.671 | slower |
| page/properties/2 | gpu_warm_p95_ms | 9.711 (9.863) | 11.386 | slower |
| page/properties/2 | mask_kib | 502 (502) | 489 | changed |
| page/properties/2 | mask_resources | 65.000 (65.000) | 56.000 | changed |
| page/properties/2 | masks_drawn | 192 (192) | 60.000 | changed |
| page/properties/2 | other_commands | 0.000 (0.000) | 34.000 | changed |
| page/properties/2 | present_cold_ms | 1.132 (1.137) | 0.824 | faster |
| page/properties/2 | present_warm_median_ms | 0.515 (0.519) | 0.493 |  |
| page/properties/2 | rects | 121 (121) | 17.000 | changed |
| page/scroll/1 | compose_cold_ms | 1.321 (1.338) | 1.294 |  |
| page/scroll/1 | compose_warm_ms | 0.053 (0.061) | 0.046 |  |
| page/scroll/1 | cpu_median_ms | 5.388 (5.406) | 3.904 | faster |
| page/scroll/1 | gpu_first_ms | 5.538 (5.581) | 6.521 | slower |
| page/scroll/1 | gpu_warm_median_ms | 4.363 (4.395) | 4.901 | slower |
| page/scroll/1 | gpu_warm_p95_ms | 4.436 (4.489) | 5.008 | slower |
| page/scroll/1 | mask_kib | 104 (104) | 103 | changed |
| page/scroll/1 | mask_resources | 47.000 (47.000) | 42.000 | changed |
| page/scroll/1 | masks_drawn | 165 (165) | 43.000 | changed |
| page/scroll/1 | other_commands | 0.000 (0.000) | 32.000 | changed |
| page/scroll/1 | present_cold_ms | 0.608 (0.613) | 0.554 |  |
| page/scroll/1 | present_warm_median_ms | 0.500 (0.507) | 0.477 |  |
| page/scroll/1 | rects | 121 (121) | 21.000 | changed |
| page/scroll/1.25 | compose_cold_ms | 1.503 (1.526) | 1.463 |  |
| page/scroll/1.25 | compose_warm_ms | 0.054 (0.054) | 0.047 |  |
| page/scroll/1.25 | cpu_median_ms | 8.792 (8.843) | 6.325 | faster |
| page/scroll/1.25 | gpu_first_ms | 7.782 (7.845) | 5.836 |  |
| page/scroll/1.25 | gpu_warm_median_ms | 6.402 (6.407) | 6.385 |  |
| page/scroll/1.25 | gpu_warm_p95_ms | 6.442 (6.457) | 7.010 | slower |
| page/scroll/1.25 | mask_kib | 162 (162) | 161 | changed |
| page/scroll/1.25 | mask_resources | 47.000 (47.000) | 42.000 | changed |
| page/scroll/1.25 | masks_drawn | 165 (165) | 43.000 | changed |
| page/scroll/1.25 | other_commands | 0.000 (0.000) | 32.000 | changed |
| page/scroll/1.25 | present_cold_ms | 0.645 (0.681) | 0.593 |  |
| page/scroll/1.25 | present_warm_median_ms | 0.498 (0.523) | 0.481 |  |
| page/scroll/1.25 | rects | 121 (121) | 21.000 | changed |
| page/scroll/2 | compose_cold_ms | 2.140 (2.325) | 2.117 |  |
| page/scroll/2 | compose_warm_ms | 0.055 (0.057) | 0.049 |  |
| page/scroll/2 | cpu_median_ms | 20.454 (20.519) | 14.318 | faster |
| page/scroll/2 | gpu_first_ms | 10.770 (15.355) | 12.800 |  |
| page/scroll/2 | gpu_warm_median_ms | 9.174 (9.227) | 9.855 | slower |
| page/scroll/2 | gpu_warm_p95_ms | 9.318 (9.370) | 10.510 | slower |
| page/scroll/2 | mask_kib | 413 (413) | 412 | changed |
| page/scroll/2 | mask_resources | 47.000 (47.000) | 42.000 | changed |
| page/scroll/2 | masks_drawn | 165 (165) | 43.000 | changed |
| page/scroll/2 | other_commands | 0.000 (0.000) | 32.000 | changed |
| page/scroll/2 | present_cold_ms | 0.756 (0.765) | 0.700 |  |
| page/scroll/2 | present_warm_median_ms | 0.501 (0.512) | 0.489 |  |
| page/scroll/2 | rects | 121 (121) | 21.000 | changed |
| page/sections/1 | compose_cold_ms | 1.655 (1.668) | 1.619 |  |
| page/sections/1 | compose_warm_ms | 0.049 (0.050) | 0.043 |  |
| page/sections/1 | cpu_median_ms | 6.973 (7.022) | 5.173 | faster |
| page/sections/1 | gpu_first_ms | 5.831 (5.963) | 7.088 | slower |
| page/sections/1 | gpu_warm_median_ms | 4.510 (4.544) | 5.244 | slower |
| page/sections/1 | gpu_warm_p95_ms | 4.562 (4.610) | 5.330 | slower |
| page/sections/1 | mask_kib | 152 (152) | 152 | changed |
| page/sections/1 | mask_resources | 46.000 (46.000) | 41.000 | changed |
| page/sections/1 | masks_drawn | 153 (153) | 53.000 | changed |
| page/sections/1 | other_commands | 0.000 (0.000) | 25.000 | changed |
| page/sections/1 | present_cold_ms | 0.577 (0.584) | 0.546 |  |
| page/sections/1 | present_warm_median_ms | 0.493 (0.501) | 0.480 |  |
| page/sections/1 | rects | 103 (103) | 22.000 | changed |
| page/sections/1.25 | compose_cold_ms | 1.869 (1.884) | 1.856 |  |
| page/sections/1.25 | compose_warm_ms | 0.050 (0.051) | 0.044 |  |
| page/sections/1.25 | cpu_median_ms | 11.338 (11.345) | 8.458 | faster |
| page/sections/1.25 | gpu_first_ms | 8.157 (8.287) | 6.179 |  |
| page/sections/1.25 | gpu_warm_median_ms | 6.235 (6.346) | 6.254 |  |
| page/sections/1.25 | gpu_warm_p95_ms | 6.579 (6.605) | 7.021 | slower |
| page/sections/1.25 | mask_kib | 236 (236) | 236 | changed |
| page/sections/1.25 | mask_resources | 46.000 (46.000) | 41.000 | changed |
| page/sections/1.25 | masks_drawn | 153 (153) | 53.000 | changed |
| page/sections/1.25 | other_commands | 0.000 (0.000) | 25.000 | changed |
| page/sections/1.25 | present_cold_ms | 0.637 (0.698) | 0.610 |  |
| page/sections/1.25 | present_warm_median_ms | 0.493 (0.503) | 0.480 |  |
| page/sections/1.25 | rects | 103 (103) | 22.000 | changed |
| page/sections/2 | compose_cold_ms | 2.728 (2.806) | 2.671 |  |
| page/sections/2 | compose_warm_ms | 0.051 (0.057) | 0.044 |  |
| page/sections/2 | cpu_median_ms | 26.382 (26.544) | 19.106 | faster |
| page/sections/2 | gpu_first_ms | 11.314 (11.387) | 13.827 | slower |
| page/sections/2 | gpu_warm_median_ms | 9.308 (9.419) | 9.904 | slower |
| page/sections/2 | gpu_warm_p95_ms | 9.509 (9.708) | 10.727 | slower |
| page/sections/2 | mask_kib | 602 (602) | 601 | changed |
| page/sections/2 | mask_resources | 46.000 (46.000) | 41.000 | changed |
| page/sections/2 | masks_drawn | 153 (153) | 53.000 | changed |
| page/sections/2 | other_commands | 0.000 (0.000) | 25.000 | changed |
| page/sections/2 | present_cold_ms | 0.759 (0.795) | 0.729 |  |
| page/sections/2 | present_warm_median_ms | 0.499 (0.509) | 0.483 |  |
| page/sections/2 | rects | 103 (103) | 22.000 | changed |
| page/shapes/1 | compose_cold_ms | 0.974 (1.003) | 1.011 | within noise |
| page/shapes/1 | compose_warm_ms | 0.047 (0.052) | 0.042 |  |
| page/shapes/1 | cpu_median_ms | 5.854 (5.970) | 3.803 | faster |
| page/shapes/1 | gpu_first_ms | 5.453 (5.479) | 6.053 | slower |
| page/shapes/1 | gpu_warm_median_ms | 4.266 (4.293) | 4.279 |  |
| page/shapes/1 | gpu_warm_p95_ms | 4.353 (4.361) | 4.697 | slower |
| page/shapes/1 | mask_kib | 81.385 (81.385) | 81.072 | changed |
| page/shapes/1 | mask_resources | 31.000 (31.000) | 26.000 | changed |
| page/shapes/1 | masks_drawn | 106 (106) | 26.000 | changed |
| page/shapes/1 | other_commands | 0.000 (0.000) | 20.000 | changed |
| page/shapes/1 | present_cold_ms | 0.891 (0.896) | 0.621 | faster |
| page/shapes/1 | present_warm_median_ms | 0.474 (0.477) | 0.457 |  |
| page/shapes/1 | rects | 75.000 (75.000) | 13.000 | changed |
| page/shapes/1.25 | compose_cold_ms | 1.106 (1.120) | 1.147 | within noise |
| page/shapes/1.25 | compose_warm_ms | 0.047 (0.047) | 0.042 |  |
| page/shapes/1.25 | cpu_median_ms | 9.394 (9.480) | 6.010 | faster |
| page/shapes/1.25 | gpu_first_ms | 7.668 (7.736) | 8.716 | slower |
| page/shapes/1.25 | gpu_warm_median_ms | 5.988 (6.033) | 6.355 | slower |
| page/shapes/1.25 | gpu_warm_p95_ms | 6.266 (6.372) | 6.870 | slower |
| page/shapes/1.25 | mask_kib | 126 (126) | 125 | changed |
| page/shapes/1.25 | mask_resources | 31.000 (31.000) | 26.000 | changed |
| page/shapes/1.25 | masks_drawn | 106 (106) | 26.000 | changed |
| page/shapes/1.25 | other_commands | 0.000 (0.000) | 20.000 | changed |
| page/shapes/1.25 | present_cold_ms | 1.039 (1.050) | 0.664 | faster |
| page/shapes/1.25 | present_warm_median_ms | 0.481 (0.500) | 0.467 |  |
| page/shapes/1.25 | rects | 75.000 (75.000) | 13.000 | changed |
| page/shapes/2 | compose_cold_ms | 1.592 (1.612) | 1.636 | within noise |
| page/shapes/2 | compose_warm_ms | 0.047 (0.048) | 0.043 |  |
| page/shapes/2 | cpu_median_ms | 22.283 (22.401) | 13.701 | faster |
| page/shapes/2 | gpu_first_ms | 10.671 (10.783) | 11.953 | slower |
| page/shapes/2 | gpu_warm_median_ms | 8.980 (9.011) | 9.519 | slower |
| page/shapes/2 | gpu_warm_p95_ms | 9.250 (9.351) | 9.741 | within noise |
| page/shapes/2 | mask_kib | 322 (322) | 321 | changed |
| page/shapes/2 | mask_resources | 31.000 (31.000) | 26.000 | changed |
| page/shapes/2 | masks_drawn | 106 (106) | 26.000 | changed |
| page/shapes/2 | other_commands | 0.000 (0.000) | 20.000 | changed |
| page/shapes/2 | present_cold_ms | 1.502 (1.507) | 0.863 | faster |
| page/shapes/2 | present_warm_median_ms | 0.483 (0.497) | 0.458 |  |
| page/shapes/2 | rects | 75.000 (75.000) | 13.000 | changed |
| page/shells/1 | compose_cold_ms | 1.406 (1.421) | 1.373 |  |
| page/shells/1 | compose_warm_ms | 0.055 (0.059) | 0.046 |  |
| page/shells/1 | cpu_median_ms | 6.798 (6.834) | 4.388 | faster |
| page/shells/1 | gpu_first_ms | 6.112 (6.160) | 4.586 | faster |
| page/shells/1 | gpu_warm_median_ms | 4.639 (4.758) | 5.043 | slower |
| page/shells/1 | gpu_warm_p95_ms | 4.672 (4.784) | 5.541 | slower |
| page/shells/1 | mask_kib | 116 (116) | 116 | changed |
| page/shells/1 | mask_resources | 46.000 (46.000) | 40.000 | changed |
| page/shells/1 | masks_drawn | 173 (173) | 45.000 | changed |
| page/shells/1 | other_commands | 0.000 (0.000) | 33.000 | changed |
| page/shells/1 | present_cold_ms | 0.578 (0.617) | 0.555 |  |
| page/shells/1 | present_warm_median_ms | 0.495 (0.498) | 0.472 |  |
| page/shells/1 | rects | 125 (125) | 28.000 | changed |
| page/shells/1.25 | compose_cold_ms | 1.580 (1.582) | 1.548 |  |
| page/shells/1.25 | compose_warm_ms | 0.054 (0.054) | 0.046 |  |
| page/shells/1.25 | cpu_median_ms | 10.753 (10.791) | 7.012 | faster |
| page/shells/1.25 | gpu_first_ms | 5.441 (8.747) | 6.579 |  |
| page/shells/1.25 | gpu_warm_median_ms | 6.451 (6.827) | 5.854 |  |
| page/shells/1.25 | gpu_warm_p95_ms | 6.892 (6.997) | 6.475 |  |
| page/shells/1.25 | mask_kib | 180 (180) | 179 | changed |
| page/shells/1.25 | mask_resources | 46.000 (46.000) | 40.000 | changed |
| page/shells/1.25 | masks_drawn | 173 (173) | 45.000 | changed |
| page/shells/1.25 | other_commands | 0.000 (0.000) | 33.000 | changed |
| page/shells/1.25 | present_cold_ms | 0.614 (0.634) | 0.586 |  |
| page/shells/1.25 | present_warm_median_ms | 0.499 (0.505) | 0.483 |  |
| page/shells/1.25 | rects | 125 (125) | 28.000 | changed |
| page/shells/2 | compose_cold_ms | 2.277 (2.322) | 2.206 |  |
| page/shells/2 | compose_warm_ms | 0.054 (0.055) | 0.046 |  |
| page/shells/2 | cpu_median_ms | 25.351 (25.464) | 15.603 | faster |
| page/shells/2 | gpu_first_ms | 12.029 (12.140) | 14.056 | slower |
| page/shells/2 | gpu_warm_median_ms | 9.579 (9.807) | 10.650 | slower |
| page/shells/2 | gpu_warm_p95_ms | 9.868 (9.907) | 11.254 | slower |
| page/shells/2 | mask_kib | 461 (461) | 460 | changed |
| page/shells/2 | mask_resources | 46.000 (46.000) | 40.000 | changed |
| page/shells/2 | masks_drawn | 173 (173) | 45.000 | changed |
| page/shells/2 | other_commands | 0.000 (0.000) | 33.000 | changed |
| page/shells/2 | present_cold_ms | 0.731 (0.756) | 0.696 |  |
| page/shells/2 | present_warm_median_ms | 0.502 (0.530) | 0.487 |  |
| page/shells/2 | rects | 125 (125) | 28.000 | changed |
| page/splits/1 | compose_cold_ms | 1.374 (1.403) | 1.379 |  |
| page/splits/1 | compose_warm_ms | 0.057 (0.062) | 0.056 |  |
| page/splits/1 | cpu_median_ms | 6.046 (6.111) | 3.783 | faster |
| page/splits/1 | gpu_first_ms | 3.288 (3.874) | 3.722 |  |
| page/splits/1 | gpu_warm_median_ms | 3.933 (4.335) | 3.373 | faster |
| page/splits/1 | gpu_warm_p95_ms | 4.313 (4.472) | 4.460 |  |
| page/splits/1 | mask_kib | 94.724 (94.724) | 94.402 | changed |
| page/splits/1 | mask_resources | 42.000 (42.000) | 36.000 | changed |
| page/splits/1 | masks_drawn | 136 (136) | 38.000 | changed |
| page/splits/1 | other_commands | 0.000 (0.000) | 26.000 | changed |
| page/splits/1 | present_cold_ms | 0.612 (0.630) | 0.557 |  |
| page/splits/1 | present_warm_median_ms | 0.300 (0.313) | 0.272 |  |
| page/splits/1 | rects | 99.000 (99.000) | 24.000 | changed |
| page/splits/1.25 | compose_cold_ms | 1.461 (1.481) | 1.421 |  |
| page/splits/1.25 | compose_warm_ms | 0.050 (0.050) | 0.044 |  |
| page/splits/1.25 | cpu_median_ms | 9.125 (9.276) | 5.863 | faster |
| page/splits/1.25 | gpu_first_ms | 8.037 (8.076) | 5.945 |  |
| page/splits/1.25 | gpu_warm_median_ms | 5.942 (6.598) | 6.657 | within noise |
| page/splits/1.25 | gpu_warm_p95_ms | 6.569 (6.661) | 7.035 | slower |
| page/splits/1.25 | mask_kib | 148 (148) | 147 | changed |
| page/splits/1.25 | mask_resources | 42.000 (42.000) | 36.000 | changed |
| page/splits/1.25 | masks_drawn | 136 (136) | 38.000 | changed |
| page/splits/1.25 | other_commands | 0.000 (0.000) | 26.000 | changed |
| page/splits/1.25 | present_cold_ms | 0.575 (0.588) | 0.547 |  |
| page/splits/1.25 | present_warm_median_ms | 0.480 (0.492) | 0.474 |  |
| page/splits/1.25 | rects | 99.000 (99.000) | 24.000 | changed |
| page/splits/2 | compose_cold_ms | 2.087 (2.132) | 2.040 |  |
| page/splits/2 | compose_warm_ms | 0.050 (0.050) | 0.044 |  |
| page/splits/2 | cpu_median_ms | 21.502 (21.633) | 13.114 | faster |
| page/splits/2 | gpu_first_ms | 11.292 (11.369) | 13.267 | slower |
| page/splits/2 | gpu_warm_median_ms | 9.206 (9.266) | 9.685 | within noise |
| page/splits/2 | gpu_warm_p95_ms | 9.741 (9.787) | 10.355 | slower |
| page/splits/2 | mask_kib | 378 (378) | 376 | changed |
| page/splits/2 | mask_resources | 42.000 (42.000) | 36.000 | changed |
| page/splits/2 | masks_drawn | 136 (136) | 38.000 | changed |
| page/splits/2 | other_commands | 0.000 (0.000) | 26.000 | changed |
| page/splits/2 | present_cold_ms | 0.684 (0.688) | 0.646 |  |
| page/splits/2 | present_warm_median_ms | 0.482 (0.487) | 0.473 |  |
| page/splits/2 | rects | 99.000 (99.000) | 24.000 | changed |
| page/state/1 | compose_cold_ms | 1.685 (1.707) | 1.642 |  |
| page/state/1 | compose_warm_ms | 0.052 (0.053) | 0.044 |  |
| page/state/1 | cpu_median_ms | 6.801 (6.834) | 5.046 | faster |
| page/state/1 | gpu_first_ms | 5.803 (5.824) | 7.014 | slower |
| page/state/1 | gpu_warm_median_ms | 4.528 (4.535) | 5.196 | slower |
| page/state/1 | gpu_warm_p95_ms | 4.571 (4.585) | 5.289 | slower |
| page/state/1 | mask_kib | 145 (145) | 145 | changed |
| page/state/1 | mask_resources | 47.000 (47.000) | 42.000 | changed |
| page/state/1 | masks_drawn | 178 (178) | 46.000 | changed |
| page/state/1 | other_commands | 0.000 (0.000) | 35.000 | changed |
| page/state/1 | present_cold_ms | 0.587 (0.607) | 0.557 |  |
| page/state/1 | present_warm_median_ms | 0.499 (0.510) | 0.476 |  |
| page/state/1 | rects | 134 (134) | 26.000 | changed |
| page/state/1.25 | compose_cold_ms | 1.939 (1.951) | 1.900 |  |
| page/state/1.25 | compose_warm_ms | 0.054 (0.054) | 0.046 |  |
| page/state/1.25 | cpu_median_ms | 11.103 (11.172) | 8.288 | faster |
| page/state/1.25 | gpu_first_ms | 8.084 (8.178) | 6.073 |  |
| page/state/1.25 | gpu_warm_median_ms | 6.279 (6.421) | 5.481 |  |
| page/state/1.25 | gpu_warm_p95_ms | 6.586 (6.631) | 6.388 |  |
| page/state/1.25 | mask_kib | 225 (225) | 224 | changed |
| page/state/1.25 | mask_resources | 47.000 (47.000) | 42.000 | changed |
| page/state/1.25 | masks_drawn | 178 (178) | 46.000 | changed |
| page/state/1.25 | other_commands | 0.000 (0.000) | 35.000 | changed |
| page/state/1.25 | present_cold_ms | 0.663 (0.692) | 0.602 |  |
| page/state/1.25 | present_warm_median_ms | 0.493 (0.503) | 0.487 |  |
| page/state/1.25 | rects | 134 (134) | 26.000 | changed |
| page/state/2 | compose_cold_ms | 2.854 (2.866) | 2.754 |  |
| page/state/2 | compose_warm_ms | 0.053 (0.056) | 0.045 |  |
| page/state/2 | cpu_median_ms | 25.785 (25.815) | 18.497 | faster |
| page/state/2 | gpu_first_ms | 11.320 (14.736) | 13.588 |  |
| page/state/2 | gpu_warm_median_ms | 9.195 (9.405) | 10.233 | slower |
| page/state/2 | gpu_warm_p95_ms | 9.450 (9.599) | 10.745 | slower |
| page/state/2 | mask_kib | 575 (575) | 574 | changed |
| page/state/2 | mask_resources | 47.000 (47.000) | 42.000 | changed |
| page/state/2 | masks_drawn | 178 (178) | 46.000 | changed |
| page/state/2 | other_commands | 0.000 (0.000) | 35.000 | changed |
| page/state/2 | present_cold_ms | 0.800 (0.819) | 0.720 | faster |
| page/state/2 | present_warm_median_ms | 0.498 (0.502) | 0.486 |  |
| page/state/2 | rects | 134 (134) | 26.000 | changed |
| page/surfaces/1 | compose_cold_ms | 0.939 (0.946) | 0.902 |  |
| page/surfaces/1 | compose_warm_ms | 0.052 (0.054) | 0.045 |  |
| page/surfaces/1 | cpu_median_ms | 4.235 (4.244) | 2.887 | faster |
| page/surfaces/1 | gpu_first_ms | 5.237 (5.367) | 6.081 | slower |
| page/surfaces/1 | gpu_warm_median_ms | 4.163 (4.217) | 4.597 | slower |
| page/surfaces/1 | gpu_warm_p95_ms | 4.223 (4.316) | 4.641 | slower |
| page/surfaces/1 | mask_kib | 68.389 (68.389) | 68.076 | changed |
| page/surfaces/1 | mask_resources | 35.000 (35.000) | 30.000 | changed |
| page/surfaces/1 | masks_drawn | 135 (135) | 31.000 | changed |
| page/surfaces/1 | other_commands | 0.000 (0.000) | 27.000 | changed |
| page/surfaces/1 | present_cold_ms | 0.545 (0.548) | 0.505 |  |
| page/surfaces/1 | present_warm_median_ms | 0.464 (0.466) | 0.457 |  |
| page/surfaces/1 | rects | 99.000 (99.000) | 14.000 | changed |
| page/surfaces/1.25 | compose_cold_ms | 1.152 (1.178) | 1.114 |  |
| page/surfaces/1.25 | compose_warm_ms | 0.052 (0.056) | 0.045 |  |
| page/surfaces/1.25 | cpu_median_ms | 6.829 (6.881) | 4.628 | faster |
| page/surfaces/1.25 | gpu_first_ms | 7.317 (7.498) | 5.304 | faster |
| page/surfaces/1.25 | gpu_warm_median_ms | 6.129 (6.142) | 6.200 | within noise |
| page/surfaces/1.25 | gpu_warm_p95_ms | 6.206 (6.207) | 6.665 | slower |
| page/surfaces/1.25 | mask_kib | 120 (120) | 119 | changed |
| page/surfaces/1.25 | mask_resources | 36.000 (36.000) | 31.000 | changed |
| page/surfaces/1.25 | masks_drawn | 136 (136) | 32.000 | changed |
| page/surfaces/1.25 | other_commands | 0.000 (0.000) | 27.000 | changed |
| page/surfaces/1.25 | present_cold_ms | 0.602 (0.618) | 0.562 |  |
| page/surfaces/1.25 | present_warm_median_ms | 0.472 (0.477) | 0.462 |  |
| page/surfaces/1.25 | rects | 99.000 (99.000) | 14.000 | changed |
| page/surfaces/2 | compose_cold_ms | 1.618 (1.633) | 1.583 |  |
| page/surfaces/2 | compose_warm_ms | 0.052 (0.053) | 0.046 |  |
| page/surfaces/2 | cpu_median_ms | 16.117 (16.329) | 10.633 | faster |
| page/surfaces/2 | gpu_first_ms | 10.327 (10.466) | 11.291 | slower |
| page/surfaces/2 | gpu_warm_median_ms | 8.823 (8.906) | 9.308 | within noise |
| page/surfaces/2 | gpu_warm_p95_ms | 9.197 (9.257) | 9.729 | slower |
| page/surfaces/2 | mask_kib | 306 (306) | 305 | changed |
| page/surfaces/2 | mask_resources | 36.000 (36.000) | 31.000 | changed |
| page/surfaces/2 | masks_drawn | 136 (136) | 32.000 | changed |
| page/surfaces/2 | other_commands | 0.000 (0.000) | 27.000 | changed |
| page/surfaces/2 | present_cold_ms | 0.718 (0.724) | 0.673 |  |
| page/surfaces/2 | present_warm_median_ms | 0.468 (0.473) | 0.466 |  |
| page/surfaces/2 | rects | 99.000 (99.000) | 14.000 | changed |
| page/text-rendering/1 | compose_cold_ms | 1.391 (1.424) | 1.380 |  |
| page/text-rendering/1 | compose_warm_ms | 0.048 (0.048) | 0.043 |  |
| page/text-rendering/1 | cpu_median_ms | 5.979 (6.035) | 4.365 | faster |
| page/text-rendering/1 | gpu_first_ms | 5.551 (5.622) | 6.466 | slower |
| page/text-rendering/1 | gpu_warm_median_ms | 4.361 (4.381) | 4.849 | slower |
| page/text-rendering/1 | gpu_warm_p95_ms | 4.407 (4.481) | 4.916 | slower |
| page/text-rendering/1 | mask_kib | 134 (134) | 134 | changed |
| page/text-rendering/1 | mask_resources | 40.000 (40.000) | 35.000 | changed |
| page/text-rendering/1 | masks_drawn | 115 (115) | 35.000 | changed |
| page/text-rendering/1 | other_commands | 0.000 (0.000) | 19.000 | changed |
| page/text-rendering/1 | present_cold_ms | 0.588 (0.593) | 0.562 |  |
| page/text-rendering/1 | present_warm_median_ms | 0.462 (0.464) | 0.455 |  |
| page/text-rendering/1 | rects | 77.000 (77.000) | 14.000 | changed |
| page/text-rendering/1.25 | compose_cold_ms | 1.611 (1.623) | 1.564 |  |
| page/text-rendering/1.25 | compose_warm_ms | 0.048 (0.049) | 0.043 |  |
| page/text-rendering/1.25 | cpu_median_ms | 9.791 (9.853) | 7.177 | faster |
| page/text-rendering/1.25 | gpu_first_ms | 4.912 (7.668) | 5.673 |  |
| page/text-rendering/1.25 | gpu_warm_median_ms | 5.945 (6.322) | 5.469 |  |
| page/text-rendering/1.25 | gpu_warm_p95_ms | 6.314 (6.462) | 6.750 | within noise |
| page/text-rendering/1.25 | mask_kib | 208 (208) | 207 | changed |
| page/text-rendering/1.25 | mask_resources | 40.000 (40.000) | 35.000 | changed |
| page/text-rendering/1.25 | masks_drawn | 115 (115) | 35.000 | changed |
| page/text-rendering/1.25 | other_commands | 0.000 (0.000) | 19.000 | changed |
| page/text-rendering/1.25 | present_cold_ms | 0.635 (0.673) | 0.604 |  |
| page/text-rendering/1.25 | present_warm_median_ms | 0.464 (0.472) | 0.466 |  |
| page/text-rendering/1.25 | rects | 77.000 (77.000) | 14.000 | changed |
| page/text-rendering/2 | compose_cold_ms | 2.359 (2.408) | 2.318 |  |
| page/text-rendering/2 | compose_warm_ms | 0.048 (0.048) | 0.043 |  |
| page/text-rendering/2 | cpu_median_ms | 22.728 (22.872) | 16.172 | faster |
| page/text-rendering/2 | gpu_first_ms | 10.509 (10.792) | 12.665 | slower |
| page/text-rendering/2 | gpu_warm_median_ms | 9.073 (9.154) | 9.758 | slower |
| page/text-rendering/2 | gpu_warm_p95_ms | 9.269 (9.537) | 10.385 | slower |
| page/text-rendering/2 | mask_kib | 533 (533) | 531 | changed |
| page/text-rendering/2 | mask_resources | 40.000 (40.000) | 35.000 | changed |
| page/text-rendering/2 | masks_drawn | 115 (115) | 35.000 | changed |
| page/text-rendering/2 | other_commands | 0.000 (0.000) | 19.000 | changed |
| page/text-rendering/2 | present_cold_ms | 0.775 (0.793) | 0.733 |  |
| page/text-rendering/2 | present_warm_median_ms | 0.469 (0.473) | 0.465 |  |
| page/text-rendering/2 | rects | 77.000 (77.000) | 14.000 | changed |
| page/text/1 | compose_cold_ms | 1.796 (1.811) | 1.754 |  |
| page/text/1 | compose_warm_ms | 0.054 (0.055) | 0.046 |  |
| page/text/1 | cpu_median_ms | 6.363 (6.385) | 4.822 | faster |
| page/text/1 | gpu_first_ms | 5.535 (5.736) | 6.606 | slower |
| page/text/1 | gpu_warm_median_ms | 4.407 (4.442) | 4.883 | slower |
| page/text/1 | gpu_warm_p95_ms | 4.462 (4.521) | 5.001 | slower |
| page/text/1 | mask_kib | 134 (134) | 133 | changed |
| page/text/1 | mask_resources | 62.000 (62.000) | 57.000 | changed |
| page/text/1 | masks_drawn | 180 (180) | 58.000 | changed |
| page/text/1 | other_commands | 0.000 (0.000) | 31.000 | changed |
| page/text/1 | present_cold_ms | 0.972 (0.976) | 0.926 |  |
| page/text/1 | present_warm_median_ms | 0.499 (0.513) | 0.486 |  |
| page/text/1 | rects | 116 (116) | 17.000 | changed |
| page/text/1.25 | compose_cold_ms | 2.029 (2.041) | 1.981 |  |
| page/text/1.25 | compose_warm_ms | 0.054 (0.055) | 0.047 |  |
| page/text/1.25 | cpu_median_ms | 10.251 (10.287) | 7.796 | faster |
| page/text/1.25 | gpu_first_ms | 7.798 (7.852) | 5.669 |  |
| page/text/1.25 | gpu_warm_median_ms | 5.606 (6.392) | 6.438 | within noise |
| page/text/1.25 | gpu_warm_p95_ms | 6.487 (6.502) | 6.892 | slower |
| page/text/1.25 | mask_kib | 210 (210) | 209 | changed |
| page/text/1.25 | mask_resources | 62.000 (62.000) | 57.000 | changed |
| page/text/1.25 | masks_drawn | 180 (180) | 58.000 | changed |
| page/text/1.25 | other_commands | 0.000 (0.000) | 31.000 | changed |
| page/text/1.25 | present_cold_ms | 1.021 (1.059) | 0.974 |  |
| page/text/1.25 | present_warm_median_ms | 0.508 (0.512) | 0.491 |  |
| page/text/1.25 | rects | 116 (116) | 17.000 | changed |
| page/text/2 | compose_cold_ms | 2.890 (2.905) | 2.826 |  |
| page/text/2 | compose_warm_ms | 0.055 (0.055) | 0.047 |  |
| page/text/2 | cpu_median_ms | 23.927 (23.995) | 17.607 | faster |
| page/text/2 | gpu_first_ms | 11.040 (15.513) | 12.116 |  |
| page/text/2 | gpu_warm_median_ms | 9.009 (9.242) | 9.750 | slower |
| page/text/2 | gpu_warm_p95_ms | 9.329 (9.439) | 10.447 | slower |
| page/text/2 | mask_kib | 532 (532) | 530 | changed |
| page/text/2 | mask_resources | 62.000 (62.000) | 57.000 | changed |
| page/text/2 | masks_drawn | 180 (180) | 58.000 | changed |
| page/text/2 | other_commands | 0.000 (0.000) | 31.000 | changed |
| page/text/2 | present_cold_ms | 1.162 (1.209) | 1.107 |  |
| page/text/2 | present_warm_median_ms | 0.513 (0.530) | 0.496 |  |
| page/text/2 | rects | 116 (116) | 17.000 | changed |
| page/theme/1 | compose_cold_ms | 1.950 (1.970) | 1.933 |  |
| page/theme/1 | compose_warm_ms | 0.053 (0.057) | 0.046 |  |
| page/theme/1 | cpu_median_ms | 7.318 (7.326) | 5.832 | faster |
| page/theme/1 | gpu_first_ms | 5.563 (5.630) | 6.308 | slower |
| page/theme/1 | gpu_warm_median_ms | 4.393 (4.400) | 4.428 | within noise |
| page/theme/1 | gpu_warm_p95_ms | 4.450 (4.501) | 4.795 | slower |
| page/theme/1 | mask_kib | 186 (186) | 186 | changed |
| page/theme/1 | mask_resources | 51.000 (51.000) | 45.000 | changed |
| page/theme/1 | masks_drawn | 137 (137) | 45.000 | changed |
| page/theme/1 | other_commands | 0.000 (0.000) | 23.000 | changed |
| page/theme/1 | present_cold_ms | 0.664 (0.674) | 0.622 |  |
| page/theme/1 | present_warm_median_ms | 0.493 (0.501) | 0.480 |  |
| page/theme/1 | rects | 101 (101) | 31.000 | changed |
| page/theme/1.25 | compose_cold_ms | 2.290 (2.309) | 2.274 |  |
| page/theme/1.25 | compose_warm_ms | 0.054 (0.054) | 0.047 |  |
| page/theme/1.25 | cpu_median_ms | 12.128 (12.140) | 9.624 | faster |
| page/theme/1.25 | gpu_first_ms | 7.789 (7.841) | 5.584 | faster |
| page/theme/1.25 | gpu_warm_median_ms | 6.370 (6.412) | 6.660 | within noise |
| page/theme/1.25 | gpu_warm_p95_ms | 6.458 (6.463) | 7.001 | slower |
| page/theme/1.25 | mask_kib | 288 (288) | 287 | changed |
| page/theme/1.25 | mask_resources | 51.000 (51.000) | 45.000 | changed |
| page/theme/1.25 | masks_drawn | 147 (147) | 45.000 | changed |
| page/theme/1.25 | other_commands | 0.000 (0.000) | 28.000 | changed |
| page/theme/1.25 | present_cold_ms | 0.741 (0.871) | 0.685 |  |
| page/theme/1.25 | present_warm_median_ms | 0.501 (0.505) | 0.486 |  |
| page/theme/1.25 | rects | 106 (106) | 31.000 | changed |
| page/theme/2 | compose_cold_ms | 3.368 (3.393) | 3.325 |  |
| page/theme/2 | compose_warm_ms | 0.054 (0.055) | 0.048 |  |
| page/theme/2 | cpu_median_ms | 27.734 (27.844) | 21.586 | faster |
| page/theme/2 | gpu_first_ms | 10.883 (15.526) | 11.934 |  |
| page/theme/2 | gpu_warm_median_ms | 9.141 (9.260) | 9.936 | slower |
| page/theme/2 | gpu_warm_p95_ms | 9.344 (9.549) | 10.285 | slower |
| page/theme/2 | mask_kib | 739 (739) | 737 | changed |
| page/theme/2 | mask_resources | 51.000 (51.000) | 45.000 | changed |
| page/theme/2 | masks_drawn | 147 (147) | 45.000 | changed |
| page/theme/2 | other_commands | 0.000 (0.000) | 28.000 | changed |
| page/theme/2 | present_cold_ms | 0.870 (0.893) | 0.800 | faster |
| page/theme/2 | present_warm_median_ms | 0.503 (0.518) | 0.484 |  |
| page/theme/2 | rects | 106 (106) | 31.000 | changed |
| page/tool-windows/1 | compose_cold_ms | 1.918 (1.933) | 1.889 |  |
| page/tool-windows/1 | compose_warm_ms | 0.052 (0.052) | 0.047 |  |
| page/tool-windows/1 | cpu_median_ms | 7.673 (7.746) | 5.580 | faster |
| page/tool-windows/1 | gpu_first_ms | 5.905 (5.983) | 7.241 | slower |
| page/tool-windows/1 | gpu_warm_median_ms | 4.615 (4.634) | 5.327 | slower |
| page/tool-windows/1 | gpu_warm_p95_ms | 4.678 (4.743) | 5.409 | slower |
| page/tool-windows/1 | mask_kib | 173 (173) | 173 | changed |
| page/tool-windows/1 | mask_resources | 53.000 (53.000) | 47.000 | changed |
| page/tool-windows/1 | masks_drawn | 150 (150) | 50.000 | changed |
| page/tool-windows/1 | other_commands | 0.000 (0.000) | 26.000 | changed |
| page/tool-windows/1 | present_cold_ms | 0.598 (0.623) | 0.591 |  |
| page/tool-windows/1 | present_warm_median_ms | 0.482 (0.484) | 0.477 |  |
| page/tool-windows/1 | rects | 101 (101) | 23.000 | changed |
| page/tool-windows/1.25 | compose_cold_ms | 2.209 (2.215) | 2.167 |  |
| page/tool-windows/1.25 | compose_warm_ms | 0.052 (0.053) | 0.048 |  |
| page/tool-windows/1.25 | cpu_median_ms | 12.430 (12.447) | 9.100 | faster |
| page/tool-windows/1.25 | gpu_first_ms | 8.289 (8.368) | 6.301 |  |
| page/tool-windows/1.25 | gpu_warm_median_ms | 6.571 (6.688) | 6.397 |  |
| page/tool-windows/1.25 | gpu_warm_p95_ms | 6.755 (6.792) | 7.093 | within noise |
| page/tool-windows/1.25 | mask_kib | 272 (272) | 272 | changed |
| page/tool-windows/1.25 | mask_resources | 53.000 (53.000) | 47.000 | changed |
| page/tool-windows/1.25 | masks_drawn | 150 (150) | 50.000 | changed |
| page/tool-windows/1.25 | other_commands | 0.000 (0.000) | 26.000 | changed |
| page/tool-windows/1.25 | present_cold_ms | 0.654 (0.659) | 0.634 |  |
| page/tool-windows/1.25 | present_warm_median_ms | 0.498 (0.507) | 0.494 |  |
| page/tool-windows/1.25 | rects | 101 (101) | 23.000 | changed |
| page/tool-windows/2 | compose_cold_ms | 3.187 (3.195) | 3.128 |  |
| page/tool-windows/2 | compose_warm_ms | 0.053 (0.055) | 0.047 |  |
| page/tool-windows/2 | cpu_median_ms | 29.034 (29.107) | 20.390 | faster |
| page/tool-windows/2 | gpu_first_ms | 11.580 (11.662) | 14.020 | slower |
| page/tool-windows/2 | gpu_warm_median_ms | 9.540 (9.667) | 10.476 | slower |
| page/tool-windows/2 | gpu_warm_p95_ms | 9.847 (9.989) | 10.860 | slower |
| page/tool-windows/2 | mask_kib | 687 (687) | 686 | changed |
| page/tool-windows/2 | mask_resources | 53.000 (53.000) | 47.000 | changed |
| page/tool-windows/2 | masks_drawn | 150 (150) | 50.000 | changed |
| page/tool-windows/2 | other_commands | 0.000 (0.000) | 26.000 | changed |
| page/tool-windows/2 | present_cold_ms | 0.804 (0.826) | 0.775 |  |
| page/tool-windows/2 | present_warm_median_ms | 0.493 (0.509) | 0.484 |  |
| page/tool-windows/2 | rects | 101 (101) | 23.000 | changed |
| page/toolbar/1 | compose_cold_ms | 2.501 (2.553) | 2.433 |  |
| page/toolbar/1 | compose_warm_ms | 0.067 (0.076) | 0.053 |  |
| page/toolbar/1 | cpu_median_ms | 7.922 (7.931) | 6.496 | faster |
| page/toolbar/1 | gpu_first_ms | 5.697 (5.792) | 6.678 | slower |
| page/toolbar/1 | gpu_warm_median_ms | 4.570 (4.579) | 4.926 | slower |
| page/toolbar/1 | gpu_warm_p95_ms | 4.618 (4.640) | 5.076 | slower |
| page/toolbar/1 | mask_kib | 194 (194) | 194 | changed |
| page/toolbar/1 | mask_resources | 89.000 (89.000) | 83.000 | changed |
| page/toolbar/1 | masks_drawn | 331 (331) | 89.000 | changed |
| page/toolbar/1 | other_commands | 0.000 (0.000) | 56.000 | changed |
| page/toolbar/1 | present_cold_ms | 0.919 (1.010) | 0.822 | faster |
| page/toolbar/1 | present_warm_median_ms | 0.593 (0.598) | 0.543 |  |
| page/toolbar/1 | rects | 203 (203) | 17.000 | changed |
| page/toolbar/1.25 | compose_cold_ms | 2.848 (2.885) | 2.775 |  |
| page/toolbar/1.25 | compose_warm_ms | 0.068 (0.071) | 0.054 |  |
| page/toolbar/1.25 | cpu_median_ms | 12.623 (12.671) | 10.321 | faster |
| page/toolbar/1.25 | gpu_first_ms | 7.897 (7.993) | 5.848 | faster |
| page/toolbar/1.25 | gpu_warm_median_ms | 6.163 (6.278) | 6.643 | slower |
| page/toolbar/1.25 | gpu_warm_p95_ms | 6.519 (6.573) | 7.161 | slower |
| page/toolbar/1.25 | mask_kib | 306 (306) | 306 | changed |
| page/toolbar/1.25 | mask_resources | 89.000 (89.000) | 83.000 | changed |
| page/toolbar/1.25 | masks_drawn | 331 (331) | 89.000 | changed |
| page/toolbar/1.25 | other_commands | 0.000 (0.000) | 56.000 | changed |
| page/toolbar/1.25 | present_cold_ms | 0.946 (1.001) | 0.868 | faster |
| page/toolbar/1.25 | present_warm_median_ms | 0.588 (0.599) | 0.542 |  |
| page/toolbar/1.25 | rects | 203 (203) | 17.000 | changed |
| page/toolbar/2 | compose_cold_ms | 4.035 (4.098) | 3.941 |  |
| page/toolbar/2 | compose_warm_ms | 0.068 (0.069) | 0.053 |  |
| page/toolbar/2 | cpu_median_ms | 29.438 (29.575) | 23.651 | faster |
| page/toolbar/2 | gpu_first_ms | 11.153 (14.405) | 12.939 |  |
| page/toolbar/2 | gpu_warm_median_ms | 9.249 (9.340) | 9.882 | slower |
| page/toolbar/2 | gpu_warm_p95_ms | 9.592 (9.640) | 10.245 | slower |
| page/toolbar/2 | mask_kib | 774 (774) | 773 | changed |
| page/toolbar/2 | mask_resources | 89.000 (89.000) | 83.000 | changed |
| page/toolbar/2 | masks_drawn | 331 (331) | 89.000 | changed |
| page/toolbar/2 | other_commands | 0.000 (0.000) | 56.000 | changed |
| page/toolbar/2 | present_cold_ms | 1.088 (1.115) | 0.987 | faster |
| page/toolbar/2 | present_warm_median_ms | 0.595 (0.600) | 0.544 |  |
| page/toolbar/2 | rects | 203 (203) | 17.000 | changed |
| page/transforms/1 | compose_cold_ms | 1.941 (1.958) | 1.893 |  |
| page/transforms/1 | compose_warm_ms | 0.052 (0.052) | 0.044 |  |
| page/transforms/1 | cpu_median_ms | 8.400 (8.446) | 6.272 | faster |
| page/transforms/1 | gpu_first_ms | 6.067 (6.096) | 7.253 | slower |
| page/transforms/1 | gpu_warm_median_ms | 4.605 (4.692) | 5.297 | slower |
| page/transforms/1 | gpu_warm_p95_ms | 4.631 (4.756) | 5.536 | slower |
| page/transforms/1 | mask_kib | 188 (188) | 188 | changed |
| page/transforms/1 | mask_resources | 44.000 (44.000) | 39.000 | changed |
| page/transforms/1 | masks_drawn | 171 (171) | 43.000 | changed |
| page/transforms/1 | other_commands | 0.000 (0.000) | 31.000 | changed |
| page/transforms/1 | present_cold_ms | 0.612 (0.641) | 0.561 |  |
| page/transforms/1 | present_warm_median_ms | 0.491 (0.497) | 0.476 |  |
| page/transforms/1 | rects | 144 (144) | 41.000 | changed |
| page/transforms/1.25 | compose_cold_ms | 2.218 (2.230) | 2.172 |  |
| page/transforms/1.25 | compose_warm_ms | 0.053 (0.053) | 0.045 |  |
| page/transforms/1.25 | cpu_median_ms | 13.595 (13.598) | 10.226 | faster |
| page/transforms/1.25 | gpu_first_ms | 8.616 (8.951) | 6.513 |  |
| page/transforms/1.25 | gpu_warm_median_ms | 6.506 (6.843) | 5.903 |  |
| page/transforms/1.25 | gpu_warm_p95_ms | 6.866 (6.958) | 7.061 | within noise |
| page/transforms/1.25 | mask_kib | 296 (296) | 295 | changed |
| page/transforms/1.25 | mask_resources | 44.000 (44.000) | 39.000 | changed |
| page/transforms/1.25 | masks_drawn | 171 (171) | 43.000 | changed |
| page/transforms/1.25 | other_commands | 0.000 (0.000) | 31.000 | changed |
| page/transforms/1.25 | present_cold_ms | 0.645 (0.687) | 0.614 |  |
| page/transforms/1.25 | present_warm_median_ms | 0.493 (0.502) | 0.481 |  |
| page/transforms/1.25 | rects | 144 (144) | 41.000 | changed |
| page/transforms/2 | compose_cold_ms | 3.237 (3.280) | 3.175 |  |
| page/transforms/2 | compose_warm_ms | 0.053 (0.055) | 0.045 |  |
| page/transforms/2 | cpu_median_ms | 31.546 (31.769) | 22.877 | faster |
| page/transforms/2 | gpu_first_ms | 11.732 (12.025) | 14.432 | slower |
| page/transforms/2 | gpu_warm_median_ms | 9.484 (9.591) | 10.471 | slower |
| page/transforms/2 | gpu_warm_p95_ms | 9.755 (9.832) | 11.080 | slower |
| page/transforms/2 | mask_kib | 752 (752) | 750 | changed |
| page/transforms/2 | mask_resources | 44.000 (44.000) | 39.000 | changed |
| page/transforms/2 | masks_drawn | 171 (171) | 43.000 | changed |
| page/transforms/2 | other_commands | 0.000 (0.000) | 31.000 | changed |
| page/transforms/2 | present_cold_ms | 0.799 (0.813) | 0.752 |  |
| page/transforms/2 | present_warm_median_ms | 0.494 (0.517) | 0.482 |  |
| page/transforms/2 | rects | 144 (144) | 41.000 | changed |
| page/virtual/1 | compose_cold_ms | 1.245 (1.290) | 1.219 |  |
| page/virtual/1 | compose_warm_ms | 0.052 (0.052) | 0.045 |  |
| page/virtual/1 | cpu_median_ms | 5.396 (5.468) | 3.759 | faster |
| page/virtual/1 | gpu_first_ms | 5.377 (5.580) | 6.393 | slower |
| page/virtual/1 | gpu_warm_median_ms | 4.327 (4.359) | 4.747 | slower |
| page/virtual/1 | gpu_warm_p95_ms | 4.399 (4.409) | 4.966 | slower |
| page/virtual/1 | mask_kib | 101 (101) | 101 | changed |
| page/virtual/1 | mask_resources | 43.000 (43.000) | 38.000 | changed |
| page/virtual/1 | masks_drawn | 163 (163) | 43.000 | changed |
| page/virtual/1 | other_commands | 0.000 (0.000) | 30.000 | changed |
| page/virtual/1 | present_cold_ms | 0.596 (0.642) | 0.561 |  |
| page/virtual/1 | present_warm_median_ms | 0.487 (0.498) | 0.473 |  |
| page/virtual/1 | rects | 106 (106) | 14.000 | changed |
| page/virtual/1.25 | compose_cold_ms | 1.424 (1.432) | 1.367 |  |
| page/virtual/1.25 | compose_warm_ms | 0.052 (0.052) | 0.045 |  |
| page/virtual/1.25 | cpu_median_ms | 8.683 (8.736) | 6.090 | faster |
| page/virtual/1.25 | gpu_first_ms | 7.663 (7.815) | 9.210 | slower |
| page/virtual/1.25 | gpu_warm_median_ms | 6.322 (6.364) | 5.987 |  |
| page/virtual/1.25 | gpu_warm_p95_ms | 6.379 (6.427) | 6.585 | within noise |
| page/virtual/1.25 | mask_kib | 156 (156) | 156 | changed |
| page/virtual/1.25 | mask_resources | 43.000 (43.000) | 38.000 | changed |
| page/virtual/1.25 | masks_drawn | 163 (163) | 43.000 | changed |
| page/virtual/1.25 | other_commands | 0.000 (0.000) | 30.000 | changed |
| page/virtual/1.25 | present_cold_ms | 0.645 (0.655) | 0.604 |  |
| page/virtual/1.25 | present_warm_median_ms | 0.486 (0.491) | 0.473 |  |
| page/virtual/1.25 | rects | 106 (106) | 14.000 | changed |
| page/virtual/2 | compose_cold_ms | 2.042 (2.056) | 2.004 |  |
| page/virtual/2 | compose_warm_ms | 0.052 (0.053) | 0.045 |  |
| page/virtual/2 | cpu_median_ms | 20.427 (20.597) | 13.740 | faster |
| page/virtual/2 | gpu_first_ms | 10.840 (11.080) | 12.698 | slower |
| page/virtual/2 | gpu_warm_median_ms | 9.111 (9.218) | 9.811 | slower |
| page/virtual/2 | gpu_warm_p95_ms | 9.345 (9.402) | 10.453 | slower |
| page/virtual/2 | mask_kib | 401 (401) | 399 | changed |
| page/virtual/2 | mask_resources | 43.000 (43.000) | 38.000 | changed |
| page/virtual/2 | masks_drawn | 163 (163) | 43.000 | changed |
| page/virtual/2 | other_commands | 0.000 (0.000) | 30.000 | changed |
| page/virtual/2 | present_cold_ms | 0.796 (0.830) | 0.714 | faster |
| page/virtual/2 | present_warm_median_ms | 0.486 (0.492) | 0.481 |  |
| page/virtual/2 | rects | 106 (106) | 14.000 | changed |
| page/windows/1 | compose_cold_ms | 1.172 (1.179) | 1.098 |  |
| page/windows/1 | compose_warm_ms | 0.060 (0.067) | 0.051 |  |
| page/windows/1 | cpu_median_ms | 5.234 (5.262) | 3.117 | faster |
| page/windows/1 | gpu_first_ms | 5.821 (5.892) | 6.572 | slower |
| page/windows/1 | gpu_warm_median_ms | 4.454 (4.598) | 4.898 | slower |
| page/windows/1 | gpu_warm_p95_ms | 4.685 (4.748) | 5.009 | slower |
| page/windows/1 | mask_kib | 69.687 (69.687) | 68.983 | changed |
| page/windows/1 | mask_resources | 52.000 (52.000) | 45.000 | changed |
| page/windows/1 | masks_drawn | 225 (225) | 53.000 | changed |
| page/windows/1 | other_commands | 0.000 (0.000) | 39.000 | changed |
| page/windows/1 | present_cold_ms | 0.691 (0.711) | 0.612 | faster |
| page/windows/1 | present_warm_median_ms | 0.513 (0.519) | 0.497 |  |
| page/windows/1 | rects | 153 (153) | 21.000 | changed |
| page/windows/1.25 | compose_cold_ms | 1.300 (1.325) | 1.232 |  |
| page/windows/1.25 | compose_warm_ms | 0.060 (0.061) | 0.051 |  |
| page/windows/1.25 | cpu_median_ms | 8.306 (8.366) | 4.935 | faster |
| page/windows/1.25 | gpu_first_ms | 8.346 (8.470) | 5.716 | faster |
| page/windows/1.25 | gpu_warm_median_ms | 6.181 (6.632) | 5.992 |  |
| page/windows/1.25 | gpu_warm_p95_ms | 6.684 (6.794) | 6.721 |  |
| page/windows/1.25 | mask_kib | 110 (110) | 109 | changed |
| page/windows/1.25 | mask_resources | 52.000 (52.000) | 45.000 | changed |
| page/windows/1.25 | masks_drawn | 225 (225) | 53.000 | changed |
| page/windows/1.25 | other_commands | 0.000 (0.000) | 39.000 | changed |
| page/windows/1.25 | present_cold_ms | 0.741 (0.758) | 0.656 | faster |
| page/windows/1.25 | present_warm_median_ms | 0.519 (0.526) | 0.500 |  |
| page/windows/1.25 | rects | 153 (153) | 21.000 | changed |
| page/windows/2 | compose_cold_ms | 1.791 (1.804) | 1.698 |  |
| page/windows/2 | compose_warm_ms | 0.060 (0.061) | 0.051 |  |
| page/windows/2 | cpu_median_ms | 19.977 (20.076) | 11.348 | faster |
| page/windows/2 | gpu_first_ms | 11.183 (11.507) | 12.498 | slower |
| page/windows/2 | gpu_warm_median_ms | 9.307 (9.434) | 9.764 | within noise |
| page/windows/2 | gpu_warm_p95_ms | 9.666 (9.864) | 10.501 | slower |
| page/windows/2 | mask_kib | 278 (278) | 275 | changed |
| page/windows/2 | mask_resources | 52.000 (52.000) | 45.000 | changed |
| page/windows/2 | masks_drawn | 225 (225) | 53.000 | changed |
| page/windows/2 | other_commands | 0.000 (0.000) | 39.000 | changed |
| page/windows/2 | present_cold_ms | 0.840 (0.925) | 0.772 | faster |
| page/windows/2 | present_warm_median_ms | 0.520 (0.532) | 0.498 |  |
| page/windows/2 | rects | 153 (153) | 21.000 | changed |
| page/workspace/1 | compose_cold_ms | 1.129 (1.160) | 1.110 |  |
| page/workspace/1 | compose_warm_ms | 0.048 (0.049) | 0.043 |  |
| page/workspace/1 | cpu_median_ms | 4.136 (4.149) | 2.904 | faster |
| page/workspace/1 | gpu_first_ms | 4.875 (5.082) | 5.206 | within noise |
| page/workspace/1 | gpu_warm_median_ms | 4.029 (4.050) | 4.127 | within noise |
| page/workspace/1 | gpu_warm_p95_ms | 4.137 (4.163) | 4.208 | within noise |
| page/workspace/1 | mask_kib | 78.975 (78.975) | 78.653 | changed |
| page/workspace/1 | mask_resources | 43.000 (43.000) | 37.000 | changed |
| page/workspace/1 | masks_drawn | 121 (121) | 37.000 | changed |
| page/workspace/1 | other_commands | 0.000 (0.000) | 20.000 | changed |
| page/workspace/1 | present_cold_ms | 0.561 (0.618) | 0.525 |  |
| page/workspace/1 | present_warm_median_ms | 0.469 (0.482) | 0.466 |  |
| page/workspace/1 | rects | 82.000 (82.000) | 18.000 | changed |
| page/workspace/1.25 | compose_cold_ms | 1.293 (1.321) | 1.264 |  |
| page/workspace/1.25 | compose_warm_ms | 0.049 (0.050) | 0.044 |  |
| page/workspace/1.25 | cpu_median_ms | 6.587 (6.593) | 4.592 | faster |
| page/workspace/1.25 | gpu_first_ms | 7.045 (7.138) | 7.247 | within noise |
| page/workspace/1.25 | gpu_warm_median_ms | 5.949 (5.960) | 6.042 | within noise |
| page/workspace/1.25 | gpu_warm_p95_ms | 6.023 (6.157) | 6.106 |  |
| page/workspace/1.25 | mask_kib | 123 (123) | 122 | changed |
| page/workspace/1.25 | mask_resources | 43.000 (43.000) | 37.000 | changed |
| page/workspace/1.25 | masks_drawn | 121 (121) | 37.000 | changed |
| page/workspace/1.25 | other_commands | 0.000 (0.000) | 20.000 | changed |
| page/workspace/1.25 | present_cold_ms | 0.607 (0.624) | 0.583 |  |
| page/workspace/1.25 | present_warm_median_ms | 0.468 (0.470) | 0.458 |  |
| page/workspace/1.25 | rects | 82.000 (82.000) | 18.000 | changed |
| page/workspace/2 | compose_cold_ms | 1.791 (1.807) | 1.735 |  |
| page/workspace/2 | compose_warm_ms | 0.049 (0.052) | 0.044 |  |
| page/workspace/2 | cpu_median_ms | 15.626 (15.713) | 10.613 | faster |
| page/workspace/2 | gpu_first_ms | 10.039 (10.136) | 10.309 | within noise |
| page/workspace/2 | gpu_warm_median_ms | 8.604 (8.652) | 8.706 | within noise |
| page/workspace/2 | gpu_warm_p95_ms | 8.911 (8.975) | 9.055 | within noise |
| page/workspace/2 | mask_kib | 314 (314) | 313 | changed |
| page/workspace/2 | mask_resources | 43.000 (43.000) | 37.000 | changed |
| page/workspace/2 | masks_drawn | 121 (121) | 37.000 | changed |
| page/workspace/2 | other_commands | 0.000 (0.000) | 20.000 | changed |
| page/workspace/2 | present_cold_ms | 0.739 (0.749) | 0.691 |  |
| page/workspace/2 | present_warm_median_ms | 0.473 (0.483) | 0.458 |  |
| page/workspace/2 | rects | 82.000 (82.000) | 18.000 | changed |
