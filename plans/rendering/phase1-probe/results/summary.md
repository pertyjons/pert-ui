| Scenario | Metric | before median (max), 5 runs | after median, 5 runs | Verdict |
|---|---|---|---|---|
| J1 `navigation`: resize +7 px per frame | compose_first_ms | 2.022 (2.043) | 2.003 |  |
| J1 `navigation`: resize +7 px per frame | compose_median_ms | 2.309 (2.345) | 2.274 |  |
| J1 `navigation`: resize +7 px per frame | compose_p95_ms | 3.655 (3.679) | 3.643 |  |
| J1 `navigation`: resize +7 px per frame | cpu_median_ms | 9.365 (9.384) | 7.177 | faster |
| J1 `navigation`: resize +7 px per frame | frames | 30.000 (30.000) | 30.000 |  |
| J1 `navigation`: resize +7 px per frame | gpu_median_ms | 5.898 (5.943) | 6.624 | slower |
| J1 `navigation`: resize +7 px per frame | gpu_p95_ms | 12.174 (16.324) | 12.411 |  |
| J1 `navigation`: resize +7 px per frame | mask_kib_median | 218 (218) | 218 | changed |
| J1 `navigation`: resize +7 px per frame | mask_resources_median | 55.000 (55.000) | 50.000 | changed |
| J1 `navigation`: resize +7 px per frame | new_mask_kib_total | 1362 (1362) | 1362 |  |
| J1 `navigation`: resize +7 px per frame | new_masks_median | 1.000 (1.000) | 1.000 |  |
| J1 `navigation`: resize +7 px per frame | new_masks_total | 23.000 (23.000) | 23.000 |  |
| J1 `navigation`: resize +7 px per frame | other_commands_max | 0.000 (0.000) | 33.000 | changed |
| J1 `navigation`: resize +7 px per frame | present_frames | 30.000 (30.000) | 30.000 |  |
| J1 `navigation`: resize +7 px per frame | present_median_ms | 1.647 (1.651) | 1.619 |  |
| J1 `navigation`: resize +7 px per frame | present_p95_ms | 1.746 (1.814) | 1.716 |  |
| J1 `navigation`: scale 1 → 1.25 → 1.5 → 1.75 → 2 | compose_first_ms | 6.349 (6.400) | 6.378 |  |
| J1 `navigation`: scale 1 → 1.25 → 1.5 → 1.75 → 2 | compose_median_ms | 7.645 (7.677) | 7.688 | within noise |
| J1 `navigation`: scale 1 → 1.25 → 1.5 → 1.75 → 2 | compose_p95_ms | 8.093 (8.127) | 8.182 | within noise |
| J1 `navigation`: scale 1 → 1.25 → 1.5 → 1.75 → 2 | cpu_median_ms | 28.507 (28.545) | 22.419 | faster |
| J1 `navigation`: scale 1 → 1.25 → 1.5 → 1.75 → 2 | frames | 4.000 (4.000) | 4.000 |  |
| J1 `navigation`: scale 1 → 1.25 → 1.5 → 1.75 → 2 | gpu_median_ms | 9.332 (9.411) | 11.261 | slower |
| J1 `navigation`: scale 1 → 1.25 → 1.5 → 1.75 → 2 | gpu_p95_ms | 12.478 (12.779) | 14.910 | slower |
| J1 `navigation`: scale 1 → 1.25 → 1.5 → 1.75 → 2 | mask_kib_median | 660 (660) | 659 | changed |
| J1 `navigation`: scale 1 → 1.25 → 1.5 → 1.75 → 2 | mask_resources_median | 51.000 (51.000) | 46.000 | changed |
| J1 `navigation`: scale 1 → 1.25 → 1.5 → 1.75 → 2 | new_mask_kib_total | 2342 (2342) | 2339 | changed |
| J1 `navigation`: scale 1 → 1.25 → 1.5 → 1.75 → 2 | new_masks_median | 49.000 (49.000) | 44.000 | changed |
| J1 `navigation`: scale 1 → 1.25 → 1.5 → 1.75 → 2 | new_masks_total | 198 (198) | 178 | changed |
| J1 `navigation`: scale 1 → 1.25 → 1.5 → 1.75 → 2 | other_commands_max | 0.000 (0.000) | 31.000 | changed |
| J1 `navigation`: scale 1 → 1.25 → 1.5 → 1.75 → 2 | present_frames | 4.000 (4.000) | 4.000 |  |
| J1 `navigation`: scale 1 → 1.25 → 1.5 → 1.75 → 2 | present_median_ms | 4.078 (4.231) | 4.047 |  |
| J1 `navigation`: scale 1 → 1.25 → 1.5 → 1.75 → 2 | present_p95_ms | 5.082 (5.115) | 4.740 |  |
| cards: pan back rest frame | compose_first_ms | 0.055 (0.055) | 0.046 |  |
| cards: pan back rest frame | compose_median_ms | 0.055 (0.055) | 0.046 |  |
| cards: pan back rest frame | compose_p95_ms | 0.055 (0.055) | 0.046 |  |
| cards: pan back rest frame | cpu_median_ms | 2.761 (2.771) | 2.392 | faster |
| cards: pan back rest frame | frames | 1.000 (1.000) | 1.000 |  |
| cards: pan back rest frame | gpu_median_ms | 2.024 (2.078) | 2.152 | within noise |
| cards: pan back rest frame | gpu_p95_ms | 2.024 (2.078) | 2.152 | within noise |
| cards: pan back rest frame | mask_kib_median | 43.609 (43.609) | 43.484 | changed |
| cards: pan back rest frame | mask_resources_median | 29.000 (29.000) | 27.000 | changed |
| cards: pan back rest frame | new_mask_kib_total | 0.000 (0.000) | 0.000 |  |
| cards: pan back rest frame | new_masks_median | 0.000 (0.000) | 0.000 |  |
| cards: pan back rest frame | new_masks_total | 0.000 (0.000) | 0.000 |  |
| cards: pan back rest frame | other_commands_max | 0.000 (0.000) | 36.000 | changed |
| cards: pan back rest frame | present_frames | 1.000 (1.000) | 1.000 |  |
| cards: pan back rest frame | present_median_ms | 0.476 (0.489) | 0.445 |  |
| cards: pan back rest frame | present_p95_ms | 0.476 (0.489) | 0.445 |  |
| cards: pan back steps | compose_first_ms | 0.364 (0.372) | 0.358 |  |
| cards: pan back steps | compose_median_ms | 0.437 (0.440) | 0.430 |  |
| cards: pan back steps | compose_p95_ms | 0.472 (0.478) | 0.450 |  |
| cards: pan back steps | cpu_median_ms | 2.828 (2.833) | 2.459 | faster |
| cards: pan back steps | end_scale | 1.000 (1.000) | 1.000 |  |
| cards: pan back steps | frames | 60.000 (60.000) | 60.000 |  |
| cards: pan back steps | gpu_median_ms | 2.122 (2.128) | 2.238 | slower |
| cards: pan back steps | gpu_p95_ms | 2.215 (2.229) | 2.320 | within noise |
| cards: pan back steps | mask_kib_median | 46.070 (46.070) | 45.945 | changed |
| cards: pan back steps | mask_resources_median | 29.000 (29.000) | 27.000 | changed |
| cards: pan back steps | new_mask_kib_total | 63.281 (63.281) | 63.281 |  |
| cards: pan back steps | new_masks_median | 0.000 (0.000) | 0.000 |  |
| cards: pan back steps | new_masks_total | 36.000 (36.000) | 36.000 |  |
| cards: pan back steps | other_commands_max | 0.000 (0.000) | 48.000 | changed |
| cards: pan back steps | present_frames | 60.000 (60.000) | 60.000 |  |
| cards: pan back steps | present_median_ms | 0.634 (0.640) | 0.561 | faster |
| cards: pan back steps | present_p95_ms | 0.703 (0.758) | 0.625 | faster |
| cards: pan rest frame | compose_first_ms | 0.056 (0.057) | 0.046 |  |
| cards: pan rest frame | compose_median_ms | 0.056 (0.057) | 0.046 |  |
| cards: pan rest frame | compose_p95_ms | 0.056 (0.057) | 0.046 |  |
| cards: pan rest frame | cpu_median_ms | 2.947 (2.965) | 2.578 | faster |
| cards: pan rest frame | frames | 1.000 (1.000) | 1.000 |  |
| cards: pan rest frame | gpu_median_ms | 2.039 (2.099) | 2.201 | within noise |
| cards: pan rest frame | gpu_p95_ms | 2.039 (2.099) | 2.201 | within noise |
| cards: pan rest frame | mask_kib_median | 46.070 (46.070) | 45.945 | changed |
| cards: pan rest frame | mask_resources_median | 29.000 (29.000) | 27.000 | changed |
| cards: pan rest frame | new_mask_kib_total | 0.000 (0.000) | 0.000 |  |
| cards: pan rest frame | new_masks_median | 0.000 (0.000) | 0.000 |  |
| cards: pan rest frame | new_masks_total | 0.000 (0.000) | 0.000 |  |
| cards: pan rest frame | other_commands_max | 0.000 (0.000) | 36.000 | changed |
| cards: pan rest frame | present_frames | 1.000 (1.000) | 1.000 |  |
| cards: pan rest frame | present_median_ms | 0.479 (0.515) | 0.455 |  |
| cards: pan rest frame | present_p95_ms | 0.479 (0.515) | 0.455 |  |
| cards: pan steps | compose_first_ms | 0.373 (0.375) | 0.366 |  |
| cards: pan steps | compose_median_ms | 0.442 (0.455) | 0.429 |  |
| cards: pan steps | compose_p95_ms | 0.567 (0.572) | 0.543 |  |
| cards: pan steps | cpu_median_ms | 2.831 (2.860) | 2.452 | faster |
| cards: pan steps | end_scale | 1.000 (1.000) | 1.000 |  |
| cards: pan steps | frames | 60.000 (60.000) | 60.000 |  |
| cards: pan steps | gpu_median_ms | 2.124 (2.148) | 2.251 | within noise |
| cards: pan steps | gpu_p95_ms | 2.214 (2.224) | 2.329 | within noise |
| cards: pan steps | mask_kib_median | 46.070 (46.070) | 45.945 | changed |
| cards: pan steps | mask_resources_median | 29.000 (29.000) | 27.000 | changed |
| cards: pan steps | new_mask_kib_total | 65.742 (65.742) | 65.742 |  |
| cards: pan steps | new_masks_median | 0.000 (0.000) | 0.000 |  |
| cards: pan steps | new_masks_total | 36.000 (36.000) | 36.000 |  |
| cards: pan steps | other_commands_max | 0.000 (0.000) | 48.000 | changed |
| cards: pan steps | present_frames | 60.000 (60.000) | 60.000 |  |
| cards: pan steps | present_median_ms | 0.631 (0.634) | 0.557 | faster |
| cards: pan steps | present_p95_ms | 0.711 (0.740) | 0.624 | faster |
| cards: zoom in rest frame | compose_first_ms | 4.672 (4.700) | 4.657 |  |
| cards: zoom in rest frame | compose_median_ms | 4.672 (4.700) | 4.657 |  |
| cards: zoom in rest frame | compose_p95_ms | 4.672 (4.700) | 4.657 |  |
| cards: zoom in rest frame | cpu_median_ms | 1.352 (1.354) | 1.026 | faster |
| cards: zoom in rest frame | frames | 1.000 (1.000) | 1.000 |  |
| cards: zoom in rest frame | gpu_median_ms | 2.082 (2.104) | 2.318 | slower |
| cards: zoom in rest frame | gpu_p95_ms | 2.082 (2.104) | 2.318 | slower |
| cards: zoom in rest frame | mask_kib_median | 0.000 (0.000) | 0.000 |  |
| cards: zoom in rest frame | mask_resources_median | 0.000 (0.000) | 0.000 |  |
| cards: zoom in rest frame | new_mask_kib_total | 0.000 (0.000) | 0.000 |  |
| cards: zoom in rest frame | new_masks_median | 0.000 (0.000) | 0.000 |  |
| cards: zoom in rest frame | new_masks_total | 0.000 (0.000) | 0.000 |  |
| cards: zoom in rest frame | other_commands_max | 0.000 (0.000) | 1.000 | changed |
| cards: zoom in rest frame | present_frames | 1.000 (1.000) | 1.000 |  |
| cards: zoom in rest frame | present_median_ms | 0.319 (0.327) | 0.329 | within noise |
| cards: zoom in rest frame | present_p95_ms | 0.319 (0.327) | 0.329 | within noise |
| cards: zoom in steps | compose_first_ms | 4.207 (4.250) | 4.240 |  |
| cards: zoom in steps | compose_median_ms | 0.497 (0.500) | 0.499 |  |
| cards: zoom in steps | compose_p95_ms | 0.522 (0.534) | 0.531 |  |
| cards: zoom in steps | cpu_median_ms | 2.948 (2.951) | 2.497 | faster |
| cards: zoom in steps | end_scale | 8.000 (8.000) | 8.000 |  |
| cards: zoom in steps | frames | 60.000 (60.000) | 60.000 |  |
| cards: zoom in steps | gpu_median_ms | 2.071 (2.096) | 2.245 | slower |
| cards: zoom in steps | gpu_p95_ms | 2.161 (2.199) | 2.370 | slower |
| cards: zoom in steps | mask_kib_median | 9.828 (9.828) | 9.670 | changed |
| cards: zoom in steps | mask_resources_median | 9.000 (9.000) | 7.000 | changed |
| cards: zoom in steps | new_mask_kib_total | 46.588 (46.588) | 46.430 | changed |
| cards: zoom in steps | new_masks_median | 0.000 (0.000) | 0.000 |  |
| cards: zoom in steps | new_masks_total | 28.000 (28.000) | 26.000 | changed |
| cards: zoom in steps | other_commands_max | 0.000 (0.000) | 36.000 | changed |
| cards: zoom in steps | present_frames | 60.000 (60.000) | 60.000 |  |
| cards: zoom in steps | present_median_ms | 0.390 (0.394) | 0.373 |  |
| cards: zoom in steps | present_p95_ms | 0.608 (0.622) | 0.541 |  |
| cards: zoom out further rest frame | compose_first_ms | 5.014 (5.109) | 4.895 |  |
| cards: zoom out further rest frame | compose_median_ms | 5.014 (5.109) | 4.895 |  |
| cards: zoom out further rest frame | compose_p95_ms | 5.014 (5.109) | 4.895 |  |
| cards: zoom out further rest frame | cpu_median_ms | 0.794 (0.811) | 0.897 | slower |
| cards: zoom out further rest frame | frames | 1.000 (1.000) | 1.000 |  |
| cards: zoom out further rest frame | gpu_median_ms | 3.252 (3.295) | 2.989 |  |
| cards: zoom out further rest frame | gpu_p95_ms | 3.252 (3.295) | 2.989 |  |
| cards: zoom out further rest frame | mask_kib_median | 5.190 (5.190) | 5.186 | changed |
| cards: zoom out further rest frame | mask_resources_median | 133 (133) | 131 | changed |
| cards: zoom out further rest frame | new_mask_kib_total | 5.190 (5.190) | 5.186 | changed |
| cards: zoom out further rest frame | new_masks_median | 133 (133) | 131 | changed |
| cards: zoom out further rest frame | new_masks_total | 133 (133) | 131 | changed |
| cards: zoom out further rest frame | other_commands_max | 0.000 (0.000) | 192 | changed |
| cards: zoom out further rest frame | present_frames | 1.000 (1.000) | 1.000 |  |
| cards: zoom out further rest frame | present_median_ms | 2.675 (2.693) | 2.346 | faster |
| cards: zoom out further rest frame | present_p95_ms | 2.675 (2.693) | 2.346 | faster |
| cards: zoom out further steps | compose_first_ms | 4.197 (4.332) | 4.235 |  |
| cards: zoom out further steps | compose_median_ms | 0.616 (0.619) | 0.572 |  |
| cards: zoom out further steps | compose_p95_ms | 0.804 (0.820) | 0.758 |  |
| cards: zoom out further steps | cpu_median_ms | 1.699 (1.714) | 1.770 | within noise |
| cards: zoom out further steps | end_scale | 0.125 (0.125) | 0.125 |  |
| cards: zoom out further steps | frames | 60.000 (60.000) | 60.000 |  |
| cards: zoom out further steps | gpu_median_ms | 2.185 (2.248) | 2.172 |  |
| cards: zoom out further steps | gpu_p95_ms | 2.344 (2.428) | 2.283 |  |
| cards: zoom out further steps | mask_kib_median | 156 (156) | 156 | changed |
| cards: zoom out further steps | mask_resources_median | 96.000 (96.000) | 94.000 | changed |
| cards: zoom out further steps | new_mask_kib_total | 214 (214) | 214 | changed |
| cards: zoom out further steps | new_masks_median | 0.000 (0.000) | 0.000 |  |
| cards: zoom out further steps | new_masks_total | 131 (131) | 130 | changed |
| cards: zoom out further steps | other_commands_max | 0.000 (0.000) | 192 | changed |
| cards: zoom out further steps | present_frames | 60.000 (60.000) | 60.000 |  |
| cards: zoom out further steps | present_median_ms | 1.208 (1.231) | 0.926 | faster |
| cards: zoom out further steps | present_p95_ms | 1.684 (1.752) | 1.346 | faster |
| cards: zoom out rest frame | compose_first_ms | 4.166 (4.413) | 4.195 |  |
| cards: zoom out rest frame | compose_median_ms | 4.166 (4.413) | 4.195 |  |
| cards: zoom out rest frame | compose_p95_ms | 4.166 (4.413) | 4.195 |  |
| cards: zoom out rest frame | cpu_median_ms | 3.049 (3.106) | 2.664 | faster |
| cards: zoom out rest frame | frames | 1.000 (1.000) | 1.000 |  |
| cards: zoom out rest frame | gpu_median_ms | 2.310 (2.379) | 2.378 |  |
| cards: zoom out rest frame | gpu_p95_ms | 2.310 (2.379) | 2.378 |  |
| cards: zoom out rest frame | mask_kib_median | 43.609 (43.609) | 43.484 | changed |
| cards: zoom out rest frame | mask_resources_median | 29.000 (29.000) | 27.000 | changed |
| cards: zoom out rest frame | new_mask_kib_total | 43.609 (43.609) | 43.484 | changed |
| cards: zoom out rest frame | new_masks_median | 29.000 (29.000) | 27.000 | changed |
| cards: zoom out rest frame | new_masks_total | 29.000 (29.000) | 27.000 | changed |
| cards: zoom out rest frame | other_commands_max | 0.000 (0.000) | 36.000 | changed |
| cards: zoom out rest frame | present_frames | 1.000 (1.000) | 1.000 |  |
| cards: zoom out rest frame | present_median_ms | 0.830 (0.864) | 0.754 |  |
| cards: zoom out rest frame | present_p95_ms | 0.830 (0.864) | 0.754 |  |
| cards: zoom out steps | compose_first_ms | 4.317 (4.396) | 4.289 |  |
| cards: zoom out steps | compose_median_ms | 0.501 (0.503) | 0.502 |  |
| cards: zoom out steps | compose_p95_ms | 1.005 (1.016) | 0.995 |  |
| cards: zoom out steps | cpu_median_ms | 2.712 (2.727) | 2.280 | faster |
| cards: zoom out steps | end_scale | 1.000 (1.000) | 1.000 |  |
| cards: zoom out steps | frames | 60.000 (60.000) | 60.000 |  |
| cards: zoom out steps | gpu_median_ms | 2.121 (2.138) | 2.266 | slower |
| cards: zoom out steps | gpu_p95_ms | 2.267 (2.297) | 2.363 | within noise |
| cards: zoom out steps | mask_kib_median | 530 (530) | 523 | changed |
| cards: zoom out steps | mask_resources_median | 9.000 (9.000) | 7.000 | changed |
| cards: zoom out steps | new_mask_kib_total | 2524 (2524) | 2517 | changed |
| cards: zoom out steps | new_masks_median | 0.000 (0.000) | 0.000 |  |
| cards: zoom out steps | new_masks_total | 29.000 (29.000) | 27.000 | changed |
| cards: zoom out steps | other_commands_max | 0.000 (0.000) | 36.000 | changed |
| cards: zoom out steps | present_frames | 60.000 (60.000) | 60.000 |  |
| cards: zoom out steps | present_median_ms | 0.410 (0.419) | 0.394 |  |
| cards: zoom out steps | present_p95_ms | 0.629 (0.637) | 0.556 | faster |
| diagram: live zoom settle frame | compose_first_ms | 0.095 (0.097) | 0.091 |  |
| diagram: live zoom settle frame | compose_median_ms | 0.095 (0.097) | 0.091 |  |
| diagram: live zoom settle frame | compose_p95_ms | 0.095 (0.097) | 0.091 |  |
| diagram: live zoom settle frame | cpu_median_ms | 2.313 (2.356) | 1.428 | faster |
| diagram: live zoom settle frame | frames | 1.000 (1.000) | 1.000 |  |
| diagram: live zoom settle frame | gpu_median_ms | 3.440 (3.519) | 3.485 |  |
| diagram: live zoom settle frame | gpu_p95_ms | 3.440 (3.519) | 3.485 |  |
| diagram: live zoom settle frame | mask_kib_median | 21.869 (21.869) | 21.735 | changed |
| diagram: live zoom settle frame | mask_resources_median | 29.000 (29.000) | 26.000 | changed |
| diagram: live zoom settle frame | new_mask_kib_total | 0.000 (0.000) | 0.000 |  |
| diagram: live zoom settle frame | new_masks_median | 0.000 (0.000) | 0.000 |  |
| diagram: live zoom settle frame | new_masks_total | 0.000 (0.000) | 0.000 |  |
| diagram: live zoom settle frame | other_commands_max | 0.000 (0.000) | 15.000 | changed |
| diagram: live zoom settle frame | present_frames | 1.000 (1.000) | 1.000 |  |
| diagram: live zoom settle frame | present_median_ms | 0.405 (0.413) | 0.391 |  |
| diagram: live zoom settle frame | present_p95_ms | 0.405 (0.413) | 0.391 |  |
| diagram: live zoom steps | compose_first_ms | 0.247 (0.251) | 0.239 |  |
| diagram: live zoom steps | compose_median_ms | 0.247 (0.249) | 0.243 |  |
| diagram: live zoom steps | compose_p95_ms | 0.273 (0.279) | 0.262 |  |
| diagram: live zoom steps | cpu_median_ms | 2.529 (2.542) | 1.673 | faster |
| diagram: live zoom steps | frames | 30.000 (30.000) | 30.000 |  |
| diagram: live zoom steps | gpu_median_ms | 3.564 (3.586) | 3.612 | within noise |
| diagram: live zoom steps | gpu_p95_ms | 3.686 (3.719) | 3.718 |  |
| diagram: live zoom steps | mask_kib_median | 23.138 (23.138) | 22.576 | changed |
| diagram: live zoom steps | mask_resources_median | 33.000 (33.000) | 27.000 | changed |
| diagram: live zoom steps | new_mask_kib_total | 26.290 (26.290) | 25.960 | changed |
| diagram: live zoom steps | new_masks_median | 1.000 (1.000) | 1.000 |  |
| diagram: live zoom steps | new_masks_total | 35.000 (35.000) | 33.000 | changed |
| diagram: live zoom steps | other_commands_max | 0.000 (0.000) | 19.000 | changed |
| diagram: live zoom steps | present_frames | 30.000 (30.000) | 30.000 |  |
| diagram: live zoom steps | present_median_ms | 0.542 (0.546) | 0.550 | within noise |
| diagram: live zoom steps | present_p95_ms | 0.777 (0.862) | 0.851 |  |
| diagram: pan 7 px per frame | compose_first_ms | 0.113 (0.116) | 0.115 |  |
| diagram: pan 7 px per frame | compose_median_ms | 0.161 (0.163) | 0.162 |  |
| diagram: pan 7 px per frame | compose_p95_ms | 0.170 (0.173) | 0.181 | within noise |
| diagram: pan 7 px per frame | cpu_median_ms | 2.952 (2.964) | 2.102 | faster |
| diagram: pan 7 px per frame | frames | 60.000 (60.000) | 60.000 |  |
| diagram: pan 7 px per frame | gpu_median_ms | 3.632 (3.656) | 3.653 |  |
| diagram: pan 7 px per frame | gpu_p95_ms | 3.705 (3.742) | 3.749 | within noise |
| diagram: pan 7 px per frame | mask_kib_median | 22.355 (22.355) | 21.735 | changed |
| diagram: pan 7 px per frame | mask_resources_median | 33.000 (33.000) | 26.000 | changed |
| diagram: pan 7 px per frame | new_mask_kib_total | 0.000 (0.000) | 0.000 |  |
| diagram: pan 7 px per frame | new_masks_median | 0.000 (0.000) | 0.000 |  |
| diagram: pan 7 px per frame | new_masks_total | 0.000 (0.000) | 0.000 |  |
| diagram: pan 7 px per frame | other_commands_max | 0.000 (0.000) | 19.000 | changed |
| diagram: pan 7 px per frame | present_frames | 60.000 (60.000) | 60.000 |  |
| diagram: pan 7 px per frame | present_median_ms | 0.737 (0.746) | 0.787 | within noise |
| diagram: pan 7 px per frame | present_p95_ms | 0.875 (0.879) | 0.938 | slower |
| diagram: settled zoom in and out | compose_first_ms | 0.250 (0.252) | 0.236 |  |
| diagram: settled zoom in and out | compose_median_ms | 0.237 (0.241) | 0.218 |  |
| diagram: settled zoom in and out | compose_p95_ms | 0.323 (0.325) | 0.270 |  |
| diagram: settled zoom in and out | cpu_median_ms | 2.555 (2.560) | 1.658 | faster |
| diagram: settled zoom in and out | frames | 16.000 (16.000) | 16.000 |  |
| diagram: settled zoom in and out | gpu_median_ms | 3.583 (3.600) | 3.612 | within noise |
| diagram: settled zoom in and out | gpu_p95_ms | 3.694 (3.780) | 3.688 |  |
| diagram: settled zoom in and out | mask_kib_median | 26.002 (26.002) | 24.630 | changed |
| diagram: settled zoom in and out | mask_resources_median | 33.000 (33.000) | 27.000 | changed |
| diagram: settled zoom in and out | new_mask_kib_total | 73.906 (73.906) | 51.752 | changed |
| diagram: settled zoom in and out | new_masks_median | 4.000 (4.000) | 2.000 | changed |
| diagram: settled zoom in and out | new_masks_total | 58.000 (58.000) | 30.000 | changed |
| diagram: settled zoom in and out | other_commands_max | 0.000 (0.000) | 19.000 | changed |
| diagram: settled zoom in and out | present_frames | 16.000 (16.000) | 16.000 |  |
| diagram: settled zoom in and out | present_median_ms | 0.539 (0.571) | 0.515 |  |
| diagram: settled zoom in and out | present_p95_ms | 0.903 (0.930) | 0.934 | within noise |
| knob: gallery input page, Shift+Up (whole display list) | compose_first_ms | 1.387 (1.436) | 0.917 | faster |
| knob: gallery input page, Shift+Up (whole display list) | compose_median_ms | 1.611 (1.620) | 1.127 | faster |
| knob: gallery input page, Shift+Up (whole display list) | compose_p95_ms | 1.674 (1.681) | 1.175 | faster |
| knob: gallery input page, Shift+Up (whole display list) | cpu_median_ms | 5.771 (5.780) | 3.523 | faster |
| knob: gallery input page, Shift+Up (whole display list) | frames | 200 (200) | 200 |  |
| knob: gallery input page, Shift+Up (whole display list) | gpu_median_ms | 3.529 (3.539) | 4.292 | slower |
| knob: gallery input page, Shift+Up (whole display list) | gpu_p95_ms | 3.614 (3.624) | 4.385 | slower |
| knob: gallery input page, Shift+Up (whole display list) | mask_kib_median | 73.871 (73.871) | 69.758 | changed |
| knob: gallery input page, Shift+Up (whole display list) | mask_resources_median | 47.000 (47.000) | 38.000 | changed |
| knob: gallery input page, Shift+Up (whole display list) | new_mask_kib_total | 2461 (2461) | 1963 | changed |
| knob: gallery input page, Shift+Up (whole display list) | new_masks_median | 5.000 (5.000) | 3.000 | changed |
| knob: gallery input page, Shift+Up (whole display list) | new_masks_total | 994 (994) | 600 | changed |
| knob: gallery input page, Shift+Up (whole display list) | other_commands_max | 0.000 (0.000) | 45.000 | changed |
| knob: gallery input page, Shift+Up (whole display list) | present_frames | 200 (200) | 200 |  |
| knob: gallery input page, Shift+Up (whole display list) | present_median_ms | 0.600 (0.607) | 0.563 |  |
| knob: gallery input page, Shift+Up (whole display list) | present_p95_ms | 0.634 (0.644) | 0.603 |  |
| knob: isolated, no readout, host writes | compose_first_ms | 0.224 (0.225) | 0.003 | faster |
| knob: isolated, no readout, host writes | compose_median_ms | 0.259 (0.266) | 0.028 | faster |
| knob: isolated, no readout, host writes | compose_p95_ms | 0.272 (0.282) | 0.030 | faster |
| knob: isolated, no readout, host writes | cpu_median_ms | 0.106 (0.107) | 0.092 |  |
| knob: isolated, no readout, host writes | frames | 200 (200) | 200 |  |
| knob: isolated, no readout, host writes | gpu_median_ms | 0.888 (0.889) | 0.889 |  |
| knob: isolated, no readout, host writes | gpu_p95_ms | 1.072 (1.108) | 1.029 |  |
| knob: isolated, no readout, host writes | mask_kib_median | 2.531 (2.531) | 0.000 | changed |
| knob: isolated, no readout, host writes | mask_resources_median | 2.000 (2.000) | 0.000 | changed |
| knob: isolated, no readout, host writes | new_mask_kib_total | 248 (248) | 0.000 | changed |
| knob: isolated, no readout, host writes | new_masks_median | 1.000 (1.000) | 0.000 | changed |
| knob: isolated, no readout, host writes | new_masks_total | 196 (196) | 0.000 | changed |
| knob: isolated, no readout, host writes | other_commands_max | 0.000 (0.000) | 2.000 | changed |
| knob: isolated, no readout, host writes | present_frames | 200 (200) | 200 |  |
| knob: isolated, no readout, host writes | present_median_ms | 0.384 (0.389) | 0.384 |  |
| knob: isolated, no readout, host writes | present_p95_ms | 0.409 (0.422) | 0.408 |  |
| mixer: one tick per frame | compose_first_ms | 0.046 (0.056) | 0.038 |  |
| mixer: one tick per frame | compose_median_ms | 0.076 (0.078) | 0.071 |  |
| mixer: one tick per frame | compose_p95_ms | 0.112 (0.118) | 0.104 |  |
| mixer: one tick per frame | cpu_median_ms | 0.941 (0.943) | 1.056 | slower |
| mixer: one tick per frame | frames | 300 (300) | 300 |  |
| mixer: one tick per frame | gpu_median_ms | 2.113 (2.119) | 2.131 | within noise |
| mixer: one tick per frame | gpu_p95_ms | 2.199 (2.214) | 2.235 | within noise |
| mixer: one tick per frame | mask_kib_median | 18.845 (18.845) | 18.379 | changed |
| mixer: one tick per frame | mask_resources_median | 19.000 (19.000) | 14.000 | changed |
| mixer: one tick per frame | new_mask_kib_total | 0.000 (0.000) | 0.000 |  |
| mixer: one tick per frame | new_masks_median | 0.000 (0.000) | 0.000 |  |
| mixer: one tick per frame | new_masks_total | 0.000 (0.000) | 0.000 |  |
| mixer: one tick per frame | other_commands_max | 0.000 (0.000) | 21.000 | changed |
| mixer: one tick per frame | present_frames | 300 (300) | 300 |  |
| mixer: one tick per frame | present_median_ms | 0.431 (0.432) | 0.417 |  |
| mixer: one tick per frame | present_p95_ms | 0.455 (0.465) | 0.444 |  |
| page/chips/1 | compose_cold_ms | 1.957 (1.972) | 1.930 |  |
| page/chips/1 | compose_warm_ms | 0.057 (0.058) | 0.049 |  |
| page/chips/1 | cpu_median_ms | 7.678 (7.701) | 5.685 | faster |
| page/chips/1 | gpu_first_ms | 5.789 (5.796) | 7.184 | slower |
| page/chips/1 | gpu_warm_median_ms | 4.560 (4.593) | 5.199 | slower |
| page/chips/1 | gpu_warm_p95_ms | 4.614 (4.669) | 5.380 | slower |
| page/chips/1 | mask_kib | 162 (162) | 161 | changed |
| page/chips/1 | mask_resources | 61.000 (61.000) | 55.000 | changed |
| page/chips/1 | masks_drawn | 205 (205) | 63.000 | changed |
| page/chips/1 | other_commands | 0.000 (0.000) | 37.000 | changed |
| page/chips/1 | present_cold_ms | 0.683 (0.708) | 0.657 |  |
| page/chips/1 | present_warm_median_ms | 0.518 (0.522) | 0.504 |  |
| page/chips/1 | rects | 104 (104) | 14.000 | changed |
| page/chips/1.25 | compose_cold_ms | 2.262 (2.277) | 2.220 |  |
| page/chips/1.25 | compose_warm_ms | 0.058 (0.059) | 0.050 |  |
| page/chips/1.25 | cpu_median_ms | 12.504 (12.522) | 9.290 | faster |
| page/chips/1.25 | gpu_first_ms | 4.933 (8.253) | 5.971 |  |
| page/chips/1.25 | gpu_warm_median_ms | 6.005 (6.530) | 6.680 | within noise |
| page/chips/1.25 | gpu_warm_p95_ms | 6.651 (6.686) | 7.300 | slower |
| page/chips/1.25 | mask_kib | 257 (257) | 255 | changed |
| page/chips/1.25 | mask_resources | 62.000 (62.000) | 56.000 | changed |
| page/chips/1.25 | masks_drawn | 206 (206) | 64.000 | changed |
| page/chips/1.25 | other_commands | 0.000 (0.000) | 37.000 | changed |
| page/chips/1.25 | present_cold_ms | 0.753 (0.774) | 0.721 |  |
| page/chips/1.25 | present_warm_median_ms | 0.519 (0.527) | 0.515 |  |
| page/chips/1.25 | rects | 104 (104) | 14.000 | changed |
| page/chips/2 | compose_cold_ms | 3.276 (3.359) | 3.249 |  |
| page/chips/2 | compose_warm_ms | 0.058 (0.060) | 0.050 |  |
| page/chips/2 | cpu_median_ms | 28.961 (29.007) | 20.869 | faster |
| page/chips/2 | gpu_first_ms | 10.700 (10.979) | 13.419 | slower |
| page/chips/2 | gpu_warm_median_ms | 9.046 (9.119) | 10.283 | slower |
| page/chips/2 | gpu_warm_p95_ms | 9.285 (9.402) | 10.698 | slower |
| page/chips/2 | mask_kib | 654 (654) | 650 | changed |
| page/chips/2 | mask_resources | 62.000 (62.000) | 56.000 | changed |
| page/chips/2 | masks_drawn | 206 (206) | 64.000 | changed |
| page/chips/2 | other_commands | 0.000 (0.000) | 37.000 | changed |
| page/chips/2 | present_cold_ms | 0.881 (0.894) | 0.852 |  |
| page/chips/2 | present_warm_median_ms | 0.526 (0.532) | 0.508 |  |
| page/chips/2 | rects | 104 (104) | 14.000 | changed |
| page/collections/1 | compose_cold_ms | 2.201 (2.225) | 2.175 |  |
| page/collections/1 | compose_warm_ms | 0.075 (0.078) | 0.067 |  |
| page/collections/1 | cpu_median_ms | 7.902 (7.928) | 5.967 | faster |
| page/collections/1 | gpu_first_ms | 5.780 (5.893) | 7.218 | slower |
| page/collections/1 | gpu_warm_median_ms | 4.518 (4.585) | 5.155 | slower |
| page/collections/1 | gpu_warm_p95_ms | 4.634 (4.812) | 5.403 | slower |
| page/collections/1 | mask_kib | 181 (181) | 180 | changed |
| page/collections/1 | mask_resources | 63.000 (63.000) | 58.000 | changed |
| page/collections/1 | masks_drawn | 162 (162) | 62.000 | changed |
| page/collections/1 | other_commands | 0.000 (0.000) | 24.000 | changed |
| page/collections/1 | present_cold_ms | 0.698 (0.752) | 0.648 |  |
| page/collections/1 | present_warm_median_ms | 0.501 (0.517) | 0.483 |  |
| page/collections/1 | rects | 98.000 (98.000) | 16.000 | changed |
| page/collections/1.25 | compose_cold_ms | 2.554 (2.565) | 2.534 |  |
| page/collections/1.25 | compose_warm_ms | 0.077 (0.079) | 0.068 |  |
| page/collections/1.25 | cpu_median_ms | 12.938 (12.946) | 9.803 | faster |
| page/collections/1.25 | gpu_first_ms | 8.098 (8.236) | 9.479 | slower |
| page/collections/1.25 | gpu_warm_median_ms | 6.268 (6.308) | 6.271 |  |
| page/collections/1.25 | gpu_warm_p95_ms | 6.612 (6.649) | 6.895 | within noise |
| page/collections/1.25 | mask_kib | 289 (289) | 288 | changed |
| page/collections/1.25 | mask_resources | 68.000 (68.000) | 63.000 | changed |
| page/collections/1.25 | masks_drawn | 167 (167) | 67.000 | changed |
| page/collections/1.25 | other_commands | 0.000 (0.000) | 24.000 | changed |
| page/collections/1.25 | present_cold_ms | 0.759 (0.768) | 0.746 |  |
| page/collections/1.25 | present_warm_median_ms | 0.497 (0.512) | 0.497 |  |
| page/collections/1.25 | rects | 98.000 (98.000) | 16.000 | changed |
| page/collections/2 | compose_cold_ms | 3.560 (3.612) | 3.553 |  |
| page/collections/2 | compose_warm_ms | 0.075 (0.075) | 0.067 |  |
| page/collections/2 | cpu_median_ms | 29.752 (29.863) | 21.903 | faster |
| page/collections/2 | gpu_first_ms | 11.046 (11.078) | 13.417 | slower |
| page/collections/2 | gpu_warm_median_ms | 9.092 (9.186) | 10.220 | slower |
| page/collections/2 | gpu_warm_p95_ms | 9.280 (9.476) | 10.764 | slower |
| page/collections/2 | mask_kib | 715 (715) | 714 | changed |
| page/collections/2 | mask_resources | 63.000 (63.000) | 58.000 | changed |
| page/collections/2 | masks_drawn | 162 (162) | 62.000 | changed |
| page/collections/2 | other_commands | 0.000 (0.000) | 24.000 | changed |
| page/collections/2 | present_cold_ms | 0.888 (0.903) | 0.844 |  |
| page/collections/2 | present_warm_median_ms | 0.507 (0.511) | 0.486 |  |
| page/collections/2 | rects | 98.000 (98.000) | 16.000 | changed |
| page/colors/1 | compose_cold_ms | 1.806 (1.856) | 1.782 |  |
| page/colors/1 | compose_warm_ms | 0.055 (0.055) | 0.049 |  |
| page/colors/1 | cpu_median_ms | 16.117 (16.130) | 14.487 | faster |
| page/colors/1 | gpu_first_ms | 5.884 (6.030) | 7.012 | slower |
| page/colors/1 | gpu_warm_median_ms | 4.621 (4.636) | 5.228 | slower |
| page/colors/1 | gpu_warm_p95_ms | 4.676 (4.694) | 5.328 | slower |
| page/colors/1 | mask_kib | 146 (146) | 146 | changed |
| page/colors/1 | mask_resources | 41.000 (41.000) | 36.000 | changed |
| page/colors/1 | masks_drawn | 143 (143) | 37.000 | changed |
| page/colors/1 | other_commands | 0.000 (0.000) | 39.000 | changed |
| page/colors/1 | present_cold_ms | 0.695 (0.703) | 0.649 |  |
| page/colors/1 | present_warm_median_ms | 0.498 (0.522) | 0.489 |  |
| page/colors/1 | rects | 248 (248) | 119 | changed |
| page/colors/1.25 | compose_cold_ms | 2.084 (2.176) | 2.044 |  |
| page/colors/1.25 | compose_warm_ms | 0.055 (0.056) | 0.049 |  |
| page/colors/1.25 | cpu_median_ms | 25.692 (25.706) | 23.010 | faster |
| page/colors/1.25 | gpu_first_ms | 8.304 (8.352) | 5.809 |  |
| page/colors/1.25 | gpu_warm_median_ms | 6.357 (6.497) | 6.396 |  |
| page/colors/1.25 | gpu_warm_p95_ms | 6.777 (6.803) | 7.179 | slower |
| page/colors/1.25 | mask_kib | 228 (228) | 227 | changed |
| page/colors/1.25 | mask_resources | 41.000 (41.000) | 36.000 | changed |
| page/colors/1.25 | masks_drawn | 143 (143) | 37.000 | changed |
| page/colors/1.25 | other_commands | 0.000 (0.000) | 39.000 | changed |
| page/colors/1.25 | present_cold_ms | 0.785 (0.804) | 0.748 |  |
| page/colors/1.25 | present_warm_median_ms | 0.497 (0.501) | 0.486 |  |
| page/colors/1.25 | rects | 248 (248) | 119 | changed |
| page/colors/2 | compose_cold_ms | 2.975 (3.011) | 2.954 |  |
| page/colors/2 | compose_warm_ms | 0.055 (0.061) | 0.049 |  |
| page/colors/2 | cpu_median_ms | 62.903 (63.098) | 56.326 | faster |
| page/colors/2 | gpu_first_ms | 11.033 (11.192) | 13.151 | slower |
| page/colors/2 | gpu_warm_median_ms | 8.972 (9.152) | 9.812 | slower |
| page/colors/2 | gpu_warm_p95_ms | 9.280 (9.436) | 10.436 | slower |
| page/colors/2 | mask_kib | 579 (579) | 578 | changed |
| page/colors/2 | mask_resources | 41.000 (41.000) | 36.000 | changed |
| page/colors/2 | masks_drawn | 143 (143) | 37.000 | changed |
| page/colors/2 | other_commands | 0.000 (0.000) | 39.000 | changed |
| page/colors/2 | present_cold_ms | 0.946 (0.960) | 0.893 |  |
| page/colors/2 | present_warm_median_ms | 0.500 (0.507) | 0.490 |  |
| page/colors/2 | rects | 248 (248) | 119 | changed |
| page/controls/1 | compose_cold_ms | 1.086 (1.111) | 1.039 |  |
| page/controls/1 | compose_warm_ms | 0.052 (0.056) | 0.044 |  |
| page/controls/1 | cpu_median_ms | 4.612 (4.631) | 3.191 | faster |
| page/controls/1 | gpu_first_ms | 5.187 (5.290) | 5.790 | slower |
| page/controls/1 | gpu_warm_median_ms | 4.202 (4.220) | 4.464 | slower |
| page/controls/1 | gpu_warm_p95_ms | 4.276 (4.304) | 4.541 | slower |
| page/controls/1 | mask_kib | 78.593 (78.593) | 77.640 | changed |
| page/controls/1 | mask_resources | 50.000 (50.000) | 42.000 | changed |
| page/controls/1 | masks_drawn | 195 (195) | 45.000 | changed |
| page/controls/1 | other_commands | 0.000 (0.000) | 38.000 | changed |
| page/controls/1 | present_cold_ms | 0.605 (0.616) | 0.545 | faster |
| page/controls/1 | present_warm_median_ms | 0.494 (0.513) | 0.481 |  |
| page/controls/1 | rects | 113 (113) | 13.000 | changed |
| page/controls/1.25 | compose_cold_ms | 1.246 (1.271) | 1.199 |  |
| page/controls/1.25 | compose_warm_ms | 0.053 (0.054) | 0.045 |  |
| page/controls/1.25 | cpu_median_ms | 7.377 (7.385) | 5.108 | faster |
| page/controls/1.25 | gpu_first_ms | 7.234 (7.353) | 4.831 |  |
| page/controls/1.25 | gpu_warm_median_ms | 6.090 (6.128) | 6.143 | within noise |
| page/controls/1.25 | gpu_warm_p95_ms | 6.178 (6.220) | 6.573 | slower |
| page/controls/1.25 | mask_kib | 125 (125) | 123 | changed |
| page/controls/1.25 | mask_resources | 50.000 (50.000) | 42.000 | changed |
| page/controls/1.25 | masks_drawn | 195 (195) | 45.000 | changed |
| page/controls/1.25 | other_commands | 0.000 (0.000) | 38.000 | changed |
| page/controls/1.25 | present_cold_ms | 0.662 (0.672) | 0.623 |  |
| page/controls/1.25 | present_warm_median_ms | 0.502 (0.511) | 0.486 |  |
| page/controls/1.25 | rects | 113 (113) | 13.000 | changed |
| page/controls/2 | compose_cold_ms | 1.743 (1.749) | 1.672 |  |
| page/controls/2 | compose_warm_ms | 0.053 (0.054) | 0.045 |  |
| page/controls/2 | cpu_median_ms | 17.348 (17.376) | 11.503 | faster |
| page/controls/2 | gpu_first_ms | 9.880 (9.938) | 10.733 | slower |
| page/controls/2 | gpu_warm_median_ms | 8.481 (8.509) | 8.934 | within noise |
| page/controls/2 | gpu_warm_p95_ms | 8.600 (8.767) | 9.220 | slower |
| page/controls/2 | mask_kib | 311 (311) | 308 | changed |
| page/controls/2 | mask_resources | 50.000 (50.000) | 42.000 | changed |
| page/controls/2 | masks_drawn | 195 (195) | 45.000 | changed |
| page/controls/2 | other_commands | 0.000 (0.000) | 38.000 | changed |
| page/controls/2 | present_cold_ms | 0.781 (0.804) | 0.702 | faster |
| page/controls/2 | present_warm_median_ms | 0.501 (0.501) | 0.490 |  |
| page/controls/2 | rects | 113 (113) | 13.000 | changed |
| page/dragdrop/1 | compose_cold_ms | 1.744 (1.759) | 1.727 |  |
| page/dragdrop/1 | compose_warm_ms | 0.055 (0.055) | 0.050 |  |
| page/dragdrop/1 | cpu_median_ms | 7.495 (7.509) | 5.442 | faster |
| page/dragdrop/1 | gpu_first_ms | 5.882 (6.105) | 7.395 | slower |
| page/dragdrop/1 | gpu_warm_median_ms | 4.607 (4.610) | 5.449 | slower |
| page/dragdrop/1 | gpu_warm_p95_ms | 4.660 (4.752) | 5.518 | slower |
| page/dragdrop/1 | mask_kib | 164 (164) | 164 | changed |
| page/dragdrop/1 | mask_resources | 53.000 (53.000) | 48.000 | changed |
| page/dragdrop/1 | masks_drawn | 148 (148) | 48.000 | changed |
| page/dragdrop/1 | other_commands | 0.000 (0.000) | 26.000 | changed |
| page/dragdrop/1 | present_cold_ms | 0.640 (0.738) | 0.607 |  |
| page/dragdrop/1 | present_warm_median_ms | 0.499 (0.502) | 0.485 |  |
| page/dragdrop/1 | rects | 102 (102) | 20.000 | changed |
| page/dragdrop/1.25 | compose_cold_ms | 2.002 (2.002) | 1.985 |  |
| page/dragdrop/1.25 | compose_warm_ms | 0.056 (0.057) | 0.051 |  |
| page/dragdrop/1.25 | cpu_median_ms | 12.041 (12.071) | 8.743 | faster |
| page/dragdrop/1.25 | gpu_first_ms | 5.015 (8.372) | 6.167 |  |
| page/dragdrop/1.25 | gpu_warm_median_ms | 6.313 (6.403) | 6.164 |  |
| page/dragdrop/1.25 | gpu_warm_p95_ms | 6.705 (6.720) | 6.928 | within noise |
| page/dragdrop/1.25 | mask_kib | 253 (253) | 253 | changed |
| page/dragdrop/1.25 | mask_resources | 53.000 (53.000) | 48.000 | changed |
| page/dragdrop/1.25 | masks_drawn | 148 (148) | 48.000 | changed |
| page/dragdrop/1.25 | other_commands | 0.000 (0.000) | 26.000 | changed |
| page/dragdrop/1.25 | present_cold_ms | 0.705 (0.711) | 0.669 |  |
| page/dragdrop/1.25 | present_warm_median_ms | 0.501 (0.510) | 0.494 |  |
| page/dragdrop/1.25 | rects | 102 (102) | 20.000 | changed |
| page/dragdrop/2 | compose_cold_ms | 2.891 (2.935) | 2.884 |  |
| page/dragdrop/2 | compose_warm_ms | 0.056 (0.057) | 0.050 |  |
| page/dragdrop/2 | cpu_median_ms | 28.513 (28.846) | 20.035 | faster |
| page/dragdrop/2 | gpu_first_ms | 11.058 (16.008) | 13.624 |  |
| page/dragdrop/2 | gpu_warm_median_ms | 9.027 (9.163) | 10.172 | slower |
| page/dragdrop/2 | gpu_warm_p95_ms | 9.385 (9.492) | 10.631 | slower |
| page/dragdrop/2 | mask_kib | 649 (649) | 648 | changed |
| page/dragdrop/2 | mask_resources | 53.000 (53.000) | 48.000 | changed |
| page/dragdrop/2 | masks_drawn | 148 (148) | 48.000 | changed |
| page/dragdrop/2 | other_commands | 0.000 (0.000) | 26.000 | changed |
| page/dragdrop/2 | present_cold_ms | 0.846 (0.887) | 0.787 | faster |
| page/dragdrop/2 | present_warm_median_ms | 0.498 (0.501) | 0.492 |  |
| page/dragdrop/2 | rects | 102 (102) | 20.000 | changed |
| page/editing/1 | compose_cold_ms | 1.735 (1.751) | 1.728 |  |
| page/editing/1 | compose_warm_ms | 0.052 (0.052) | 0.047 |  |
| page/editing/1 | cpu_median_ms | 6.308 (6.320) | 4.525 | faster |
| page/editing/1 | gpu_first_ms | 5.502 (5.632) | 6.437 | slower |
| page/editing/1 | gpu_warm_median_ms | 4.394 (4.401) | 4.794 | slower |
| page/editing/1 | gpu_warm_p95_ms | 4.448 (4.504) | 4.925 | slower |
| page/editing/1 | mask_kib | 165 (165) | 165 | changed |
| page/editing/1 | mask_resources | 40.000 (40.000) | 35.000 | changed |
| page/editing/1 | masks_drawn | 137 (137) | 37.000 | changed |
| page/editing/1 | other_commands | 0.000 (0.000) | 22.000 | changed |
| page/editing/1 | present_cold_ms | 0.632 (0.696) | 0.584 |  |
| page/editing/1 | present_warm_median_ms | 0.475 (0.483) | 0.467 |  |
| page/editing/1 | rects | 102 (102) | 16.000 | changed |
| page/editing/1.25 | compose_cold_ms | 1.926 (1.946) | 1.929 |  |
| page/editing/1.25 | compose_warm_ms | 0.053 (0.057) | 0.048 |  |
| page/editing/1.25 | cpu_median_ms | 10.253 (10.259) | 7.421 | faster |
| page/editing/1.25 | gpu_first_ms | 4.675 (7.742) | 5.306 |  |
| page/editing/1.25 | gpu_warm_median_ms | 6.344 (6.357) | 4.991 | faster |
| page/editing/1.25 | gpu_warm_p95_ms | 6.407 (6.430) | 6.688 | within noise |
| page/editing/1.25 | mask_kib | 258 (258) | 257 | changed |
| page/editing/1.25 | mask_resources | 40.000 (40.000) | 35.000 | changed |
| page/editing/1.25 | masks_drawn | 137 (137) | 37.000 | changed |
| page/editing/1.25 | other_commands | 0.000 (0.000) | 22.000 | changed |
| page/editing/1.25 | present_cold_ms | 0.689 (0.729) | 0.630 |  |
| page/editing/1.25 | present_warm_median_ms | 0.484 (0.486) | 0.465 |  |
| page/editing/1.25 | rects | 102 (102) | 16.000 | changed |
| page/editing/2 | compose_cold_ms | 2.849 (2.890) | 2.854 |  |
| page/editing/2 | compose_warm_ms | 0.052 (0.053) | 0.046 |  |
| page/editing/2 | cpu_median_ms | 23.868 (23.946) | 16.711 | faster |
| page/editing/2 | gpu_first_ms | 10.476 (15.306) | 11.884 |  |
| page/editing/2 | gpu_warm_median_ms | 8.779 (8.807) | 9.647 | slower |
| page/editing/2 | gpu_warm_p95_ms | 9.040 (9.089) | 9.747 | slower |
| page/editing/2 | mask_kib | 659 (659) | 658 | changed |
| page/editing/2 | mask_resources | 40.000 (40.000) | 35.000 | changed |
| page/editing/2 | masks_drawn | 137 (137) | 37.000 | changed |
| page/editing/2 | other_commands | 0.000 (0.000) | 22.000 | changed |
| page/editing/2 | present_cold_ms | 0.810 (0.866) | 0.774 |  |
| page/editing/2 | present_warm_median_ms | 0.478 (0.489) | 0.463 |  |
| page/editing/2 | rects | 102 (102) | 16.000 | changed |
| page/families/1 | compose_cold_ms | 0.990 (1.001) | 0.948 |  |
| page/families/1 | compose_warm_ms | 0.057 (0.057) | 0.046 |  |
| page/families/1 | cpu_median_ms | 3.928 (3.948) | 2.642 | faster |
| page/families/1 | gpu_first_ms | 5.073 (5.165) | 5.562 | slower |
| page/families/1 | gpu_warm_median_ms | 4.160 (4.191) | 4.434 | slower |
| page/families/1 | gpu_warm_p95_ms | 4.241 (4.271) | 4.491 | slower |
| page/families/1 | mask_kib | 55.436 (55.436) | 54.729 | changed |
| page/families/1 | mask_resources | 54.000 (54.000) | 46.000 | changed |
| page/families/1 | masks_drawn | 254 (254) | 48.000 | changed |
| page/families/1 | other_commands | 0.000 (0.000) | 49.000 | changed |
| page/families/1 | present_cold_ms | 0.679 (0.700) | 0.555 | faster |
| page/families/1 | present_warm_median_ms | 0.534 (0.551) | 0.488 |  |
| page/families/1 | rects | 161 (161) | 14.000 | changed |
| page/families/1.25 | compose_cold_ms | 1.104 (1.129) | 1.049 |  |
| page/families/1.25 | compose_warm_ms | 0.057 (0.062) | 0.046 |  |
| page/families/1.25 | cpu_median_ms | 6.208 (6.214) | 4.124 | faster |
| page/families/1.25 | gpu_first_ms | 6.961 (7.205) | 4.664 |  |
| page/families/1.25 | gpu_warm_median_ms | 6.066 (6.076) | 5.892 |  |
| page/families/1.25 | gpu_warm_p95_ms | 6.118 (6.151) | 6.483 | slower |
| page/families/1.25 | mask_kib | 89.453 (89.453) | 88.346 | changed |
| page/families/1.25 | mask_resources | 54.000 (54.000) | 46.000 | changed |
| page/families/1.25 | masks_drawn | 254 (254) | 48.000 | changed |
| page/families/1.25 | other_commands | 0.000 (0.000) | 49.000 | changed |
| page/families/1.25 | present_cold_ms | 0.736 (0.748) | 0.609 | faster |
| page/families/1.25 | present_warm_median_ms | 0.539 (0.549) | 0.501 |  |
| page/families/1.25 | rects | 161 (161) | 14.000 | changed |
| page/families/2 | compose_cold_ms | 1.492 (1.504) | 1.401 | faster |
| page/families/2 | compose_warm_ms | 0.057 (0.062) | 0.046 |  |
| page/families/2 | cpu_median_ms | 14.927 (15.211) | 9.475 | faster |
| page/families/2 | gpu_first_ms | 9.714 (9.809) | 10.546 | slower |
| page/families/2 | gpu_warm_median_ms | 8.486 (8.509) | 8.841 | within noise |
| page/families/2 | gpu_warm_p95_ms | 8.602 (8.675) | 9.109 | within noise |
| page/families/2 | mask_kib | 220 (220) | 217 | changed |
| page/families/2 | mask_resources | 54.000 (54.000) | 46.000 | changed |
| page/families/2 | masks_drawn | 254 (254) | 48.000 | changed |
| page/families/2 | other_commands | 0.000 (0.000) | 49.000 | changed |
| page/families/2 | present_cold_ms | 0.832 (0.843) | 0.714 | faster |
| page/families/2 | present_warm_median_ms | 0.531 (0.556) | 0.499 |  |
| page/families/2 | rects | 161 (161) | 14.000 | changed |
| page/fields/1 | compose_cold_ms | 1.158 (1.179) | 1.142 |  |
| page/fields/1 | compose_warm_ms | 0.049 (0.049) | 0.043 |  |
| page/fields/1 | cpu_median_ms | 4.973 (4.991) | 3.253 | faster |
| page/fields/1 | gpu_first_ms | 5.422 (5.541) | 6.269 | slower |
| page/fields/1 | gpu_warm_median_ms | 4.295 (4.336) | 4.712 | slower |
| page/fields/1 | gpu_warm_p95_ms | 4.442 (4.485) | 4.884 | slower |
| page/fields/1 | mask_kib | 82.285 (82.285) | 81.973 | changed |
| page/fields/1 | mask_resources | 42.000 (42.000) | 37.000 | changed |
| page/fields/1 | masks_drawn | 148 (148) | 38.000 | changed |
| page/fields/1 | other_commands | 0.000 (0.000) | 26.000 | changed |
| page/fields/1 | present_cold_ms | 0.575 (0.586) | 0.535 |  |
| page/fields/1 | present_warm_median_ms | 0.480 (0.483) | 0.467 |  |
| page/fields/1 | rects | 100 (100) | 13.000 | changed |
| page/fields/1.25 | compose_cold_ms | 1.324 (1.336) | 1.302 |  |
| page/fields/1.25 | compose_warm_ms | 0.050 (0.054) | 0.044 |  |
| page/fields/1.25 | cpu_median_ms | 7.960 (7.966) | 5.242 | faster |
| page/fields/1.25 | gpu_first_ms | 4.633 (7.683) | 5.324 |  |
| page/fields/1.25 | gpu_warm_median_ms | 6.131 (6.359) | 4.933 | faster |
| page/fields/1.25 | gpu_warm_p95_ms | 6.425 (6.451) | 6.583 | within noise |
| page/fields/1.25 | mask_kib | 130 (130) | 129 | changed |
| page/fields/1.25 | mask_resources | 42.000 (42.000) | 37.000 | changed |
| page/fields/1.25 | masks_drawn | 148 (148) | 38.000 | changed |
| page/fields/1.25 | other_commands | 0.000 (0.000) | 26.000 | changed |
| page/fields/1.25 | present_cold_ms | 0.629 (0.742) | 0.599 |  |
| page/fields/1.25 | present_warm_median_ms | 0.489 (0.498) | 0.473 |  |
| page/fields/1.25 | rects | 100 (100) | 13.000 | changed |
| page/fields/2 | compose_cold_ms | 1.850 (1.851) | 1.827 |  |
| page/fields/2 | compose_warm_ms | 0.049 (0.049) | 0.043 |  |
| page/fields/2 | cpu_median_ms | 18.798 (18.829) | 11.814 | faster |
| page/fields/2 | gpu_first_ms | 10.249 (10.400) | 11.845 | slower |
| page/fields/2 | gpu_warm_median_ms | 8.703 (8.745) | 9.429 | slower |
| page/fields/2 | gpu_warm_p95_ms | 9.030 (9.109) | 9.684 | slower |
| page/fields/2 | mask_kib | 328 (328) | 327 | changed |
| page/fields/2 | mask_resources | 42.000 (42.000) | 37.000 | changed |
| page/fields/2 | masks_drawn | 148 (148) | 38.000 | changed |
| page/fields/2 | other_commands | 0.000 (0.000) | 26.000 | changed |
| page/fields/2 | present_cold_ms | 0.756 (0.789) | 0.712 |  |
| page/fields/2 | present_warm_median_ms | 0.481 (0.487) | 0.472 |  |
| page/fields/2 | rects | 100 (100) | 13.000 | changed |
| page/files/1 | compose_cold_ms | 1.939 (1.949) | 1.914 |  |
| page/files/1 | compose_warm_ms | 0.056 (0.056) | 0.050 |  |
| page/files/1 | cpu_median_ms | 7.770 (7.805) | 5.376 | faster |
| page/files/1 | gpu_first_ms | 5.971 (6.032) | 7.367 | slower |
| page/files/1 | gpu_warm_median_ms | 4.611 (4.682) | 5.313 | slower |
| page/files/1 | gpu_warm_p95_ms | 4.704 (4.720) | 5.466 | slower |
| page/files/1 | mask_kib | 161 (161) | 160 | changed |
| page/files/1 | mask_resources | 56.000 (56.000) | 50.000 | changed |
| page/files/1 | masks_drawn | 161 (161) | 57.000 | changed |
| page/files/1 | other_commands | 0.000 (0.000) | 25.000 | changed |
| page/files/1 | present_cold_ms | 0.648 (0.653) | 0.640 |  |
| page/files/1 | present_warm_median_ms | 0.499 (0.505) | 0.484 |  |
| page/files/1 | rects | 102 (102) | 19.000 | changed |
| page/files/1.25 | compose_cold_ms | 2.211 (2.226) | 2.186 |  |
| page/files/1.25 | compose_warm_ms | 0.056 (0.057) | 0.050 |  |
| page/files/1.25 | cpu_median_ms | 12.477 (12.503) | 8.730 | faster |
| page/files/1.25 | gpu_first_ms | 8.063 (8.459) | 6.021 |  |
| page/files/1.25 | gpu_warm_median_ms | 6.288 (6.580) | 5.573 |  |
| page/files/1.25 | gpu_warm_p95_ms | 6.824 (6.840) | 6.906 | within noise |
| page/files/1.25 | mask_kib | 251 (251) | 251 | changed |
| page/files/1.25 | mask_resources | 56.000 (56.000) | 50.000 | changed |
| page/files/1.25 | masks_drawn | 161 (161) | 57.000 | changed |
| page/files/1.25 | other_commands | 0.000 (0.000) | 25.000 | changed |
| page/files/1.25 | present_cold_ms | 0.721 (0.729) | 0.671 |  |
| page/files/1.25 | present_warm_median_ms | 0.499 (0.500) | 0.486 |  |
| page/files/1.25 | rects | 102 (102) | 19.000 | changed |
| page/files/2 | compose_cold_ms | 3.156 (3.245) | 3.148 |  |
| page/files/2 | compose_warm_ms | 0.057 (0.057) | 0.051 |  |
| page/files/2 | cpu_median_ms | 29.166 (29.193) | 19.585 | faster |
| page/files/2 | gpu_first_ms | 11.199 (11.364) | 13.537 | slower |
| page/files/2 | gpu_warm_median_ms | 9.021 (9.141) | 10.110 | slower |
| page/files/2 | gpu_warm_p95_ms | 9.289 (9.416) | 10.763 | slower |
| page/files/2 | mask_kib | 637 (637) | 636 | changed |
| page/files/2 | mask_resources | 56.000 (56.000) | 50.000 | changed |
| page/files/2 | masks_drawn | 161 (161) | 57.000 | changed |
| page/files/2 | other_commands | 0.000 (0.000) | 25.000 | changed |
| page/files/2 | present_cold_ms | 0.859 (0.892) | 0.814 |  |
| page/files/2 | present_warm_median_ms | 0.497 (0.508) | 0.495 |  |
| page/files/2 | rects | 102 (102) | 19.000 | changed |
| page/fonts/1 | compose_cold_ms | 1.869 (1.935) | 1.835 |  |
| page/fonts/1 | compose_warm_ms | 0.068 (0.069) | 0.060 |  |
| page/fonts/1 | cpu_median_ms | 6.689 (6.700) | 5.199 | faster |
| page/fonts/1 | gpu_first_ms | 5.438 (5.625) | 6.567 | slower |
| page/fonts/1 | gpu_warm_median_ms | 4.423 (4.450) | 4.923 | slower |
| page/fonts/1 | gpu_warm_p95_ms | 4.506 (4.542) | 5.038 | slower |
| page/fonts/1 | mask_kib | 145 (145) | 145 | changed |
| page/fonts/1 | mask_resources | 85.000 (85.000) | 80.000 | changed |
| page/fonts/1 | masks_drawn | 221 (221) | 85.000 | changed |
| page/fonts/1 | other_commands | 0.000 (0.000) | 34.000 | changed |
| page/fonts/1 | present_cold_ms | 1.067 (1.089) | 1.022 |  |
| page/fonts/1 | present_warm_median_ms | 0.520 (0.524) | 0.513 |  |
| page/fonts/1 | rects | 124 (124) | 14.000 | changed |
| page/fonts/1.25 | compose_cold_ms | 2.110 (2.134) | 2.093 |  |
| page/fonts/1.25 | compose_warm_ms | 0.069 (0.070) | 0.062 |  |
| page/fonts/1.25 | cpu_median_ms | 10.877 (10.913) | 8.432 | faster |
| page/fonts/1.25 | gpu_first_ms | 7.802 (7.857) | 5.516 |  |
| page/fonts/1.25 | gpu_warm_median_ms | 6.408 (6.427) | 6.012 |  |
| page/fonts/1.25 | gpu_warm_p95_ms | 6.475 (6.536) | 6.730 | within noise |
| page/fonts/1.25 | mask_kib | 225 (225) | 224 | changed |
| page/fonts/1.25 | mask_resources | 85.000 (85.000) | 80.000 | changed |
| page/fonts/1.25 | masks_drawn | 221 (221) | 85.000 | changed |
| page/fonts/1.25 | other_commands | 0.000 (0.000) | 34.000 | changed |
| page/fonts/1.25 | present_cold_ms | 1.127 (1.156) | 1.085 |  |
| page/fonts/1.25 | present_warm_median_ms | 0.520 (0.527) | 0.513 |  |
| page/fonts/1.25 | rects | 124 (124) | 14.000 | changed |
| page/fonts/2 | compose_cold_ms | 3.019 (3.061) | 3.008 |  |
| page/fonts/2 | compose_warm_ms | 0.068 (0.071) | 0.060 |  |
| page/fonts/2 | cpu_median_ms | 25.027 (25.080) | 18.954 | faster |
| page/fonts/2 | gpu_first_ms | 10.508 (10.634) | 12.027 | slower |
| page/fonts/2 | gpu_warm_median_ms | 8.863 (8.886) | 9.578 | slower |
| page/fonts/2 | gpu_warm_p95_ms | 9.135 (9.169) | 10.013 | slower |
| page/fonts/2 | mask_kib | 571 (571) | 569 | changed |
| page/fonts/2 | mask_resources | 85.000 (85.000) | 80.000 | changed |
| page/fonts/2 | masks_drawn | 221 (221) | 85.000 | changed |
| page/fonts/2 | other_commands | 0.000 (0.000) | 34.000 | changed |
| page/fonts/2 | present_cold_ms | 1.227 (1.235) | 1.207 |  |
| page/fonts/2 | present_warm_median_ms | 0.525 (0.529) | 0.510 |  |
| page/fonts/2 | rects | 124 (124) | 14.000 | changed |
| page/forms/1 | compose_cold_ms | 1.153 (1.164) | 1.141 |  |
| page/forms/1 | compose_warm_ms | 0.049 (0.049) | 0.043 |  |
| page/forms/1 | cpu_median_ms | 4.483 (4.495) | 3.161 | faster |
| page/forms/1 | gpu_first_ms | 5.044 (5.185) | 5.454 | slower |
| page/forms/1 | gpu_warm_median_ms | 4.092 (4.140) | 4.284 | within noise |
| page/forms/1 | gpu_warm_p95_ms | 4.201 (4.227) | 4.377 | within noise |
| page/forms/1 | mask_kib | 80.889 (80.889) | 80.567 | changed |
| page/forms/1 | mask_resources | 45.000 (45.000) | 39.000 | changed |
| page/forms/1 | masks_drawn | 150 (150) | 40.000 | changed |
| page/forms/1 | other_commands | 0.000 (0.000) | 25.000 | changed |
| page/forms/1 | present_cold_ms | 0.530 (0.541) | 0.500 |  |
| page/forms/1 | present_warm_median_ms | 0.487 (0.493) | 0.473 |  |
| page/forms/1 | rects | 104 (104) | 15.000 | changed |
| page/forms/1.25 | compose_cold_ms | 1.316 (1.328) | 1.283 |  |
| page/forms/1.25 | compose_warm_ms | 0.049 (0.050) | 0.044 |  |
| page/forms/1.25 | cpu_median_ms | 7.089 (7.106) | 4.987 | faster |
| page/forms/1.25 | gpu_first_ms | 7.057 (7.217) | 4.678 | faster |
| page/forms/1.25 | gpu_warm_median_ms | 6.012 (6.036) | 6.017 |  |
| page/forms/1.25 | gpu_warm_p95_ms | 6.074 (6.101) | 6.302 | within noise |
| page/forms/1.25 | mask_kib | 128 (128) | 127 | changed |
| page/forms/1.25 | mask_resources | 45.000 (45.000) | 39.000 | changed |
| page/forms/1.25 | masks_drawn | 150 (150) | 40.000 | changed |
| page/forms/1.25 | other_commands | 0.000 (0.000) | 25.000 | changed |
| page/forms/1.25 | present_cold_ms | 0.590 (0.597) | 0.538 |  |
| page/forms/1.25 | present_warm_median_ms | 0.492 (0.500) | 0.473 |  |
| page/forms/1.25 | rects | 104 (104) | 15.000 | changed |
| page/forms/2 | compose_cold_ms | 1.840 (1.853) | 1.813 |  |
| page/forms/2 | compose_warm_ms | 0.050 (0.058) | 0.043 |  |
| page/forms/2 | cpu_median_ms | 16.772 (16.801) | 11.527 | faster |
| page/forms/2 | gpu_first_ms | 9.731 (10.414) | 10.152 |  |
| page/forms/2 | gpu_warm_median_ms | 8.342 (8.397) | 8.577 | within noise |
| page/forms/2 | gpu_warm_p95_ms | 8.572 (8.610) | 8.834 | within noise |
| page/forms/2 | mask_kib | 323 (323) | 321 | changed |
| page/forms/2 | mask_resources | 45.000 (45.000) | 39.000 | changed |
| page/forms/2 | masks_drawn | 150 (150) | 40.000 | changed |
| page/forms/2 | other_commands | 0.000 (0.000) | 25.000 | changed |
| page/forms/2 | present_cold_ms | 0.693 (0.711) | 0.639 |  |
| page/forms/2 | present_warm_median_ms | 0.487 (0.495) | 0.467 |  |
| page/forms/2 | rects | 104 (104) | 15.000 | changed |
| page/images/1 | compose_cold_ms | 1.491 (1.511) | 1.480 |  |
| page/images/1 | compose_warm_ms | 0.051 (0.054) | 0.046 |  |
| page/images/1 | cpu_median_ms | 8.456 (8.475) | 6.711 | faster |
| page/images/1 | gpu_first_ms | 5.612 (5.761) | 6.582 | slower |
| page/images/1 | gpu_warm_median_ms | 4.406 (4.485) | 4.876 | slower |
| page/images/1 | gpu_warm_p95_ms | 4.502 (4.564) | 5.027 | slower |
| page/images/1 | mask_kib | 135 (135) | 134 | changed |
| page/images/1 | mask_resources | 41.000 (41.000) | 36.000 | changed |
| page/images/1 | masks_drawn | 132 (132) | 36.000 | changed |
| page/images/1 | other_commands | 0.000 (0.000) | 25.000 | changed |
| page/images/1 | present_cold_ms | 0.552 (0.621) | 0.552 |  |
| page/images/1 | present_warm_median_ms | 0.485 (0.493) | 0.476 |  |
| page/images/1 | rects | 97.000 (97.000) | 19.000 | changed |
| page/images/1.25 | compose_cold_ms | 1.716 (1.734) | 1.687 |  |
| page/images/1.25 | compose_warm_ms | 0.051 (0.052) | 0.046 |  |
| page/images/1.25 | cpu_median_ms | 13.588 (13.603) | 10.728 | faster |
| page/images/1.25 | gpu_first_ms | 7.847 (8.025) | 9.341 | slower |
| page/images/1.25 | gpu_warm_median_ms | 6.171 (6.495) | 6.886 | slower |
| page/images/1.25 | gpu_warm_p95_ms | 6.538 (6.569) | 7.022 | slower |
| page/images/1.25 | mask_kib | 208 (208) | 208 | changed |
| page/images/1.25 | mask_resources | 41.000 (41.000) | 36.000 | changed |
| page/images/1.25 | masks_drawn | 132 (132) | 36.000 | changed |
| page/images/1.25 | other_commands | 0.000 (0.000) | 25.000 | changed |
| page/images/1.25 | present_cold_ms | 0.615 (0.716) | 0.560 |  |
| page/images/1.25 | present_warm_median_ms | 0.489 (0.496) | 0.479 |  |
| page/images/1.25 | rects | 97.000 (97.000) | 19.000 | changed |
| page/images/2 | compose_cold_ms | 2.488 (2.518) | 2.461 |  |
| page/images/2 | compose_warm_ms | 0.051 (0.052) | 0.046 |  |
| page/images/2 | cpu_median_ms | 32.498 (32.544) | 25.288 | faster |
| page/images/2 | gpu_first_ms | 10.664 (10.748) | 12.291 | slower |
| page/images/2 | gpu_warm_median_ms | 8.909 (8.918) | 9.549 | slower |
| page/images/2 | gpu_warm_p95_ms | 9.195 (9.218) | 9.973 | slower |
| page/images/2 | mask_kib | 534 (534) | 533 | changed |
| page/images/2 | mask_resources | 41.000 (41.000) | 36.000 | changed |
| page/images/2 | masks_drawn | 132 (132) | 36.000 | changed |
| page/images/2 | other_commands | 0.000 (0.000) | 25.000 | changed |
| page/images/2 | present_cold_ms | 0.719 (0.731) | 0.673 |  |
| page/images/2 | present_warm_median_ms | 0.482 (0.490) | 0.480 |  |
| page/images/2 | rects | 97.000 (97.000) | 19.000 | changed |
| page/input/1 | compose_cold_ms | 1.958 (2.012) | 1.938 |  |
| page/input/1 | compose_warm_ms | 0.052 (0.053) | 0.044 |  |
| page/input/1 | cpu_median_ms | 8.144 (8.158) | 6.114 | faster |
| page/input/1 | gpu_first_ms | 5.857 (6.022) | 7.355 | slower |
| page/input/1 | gpu_warm_median_ms | 4.621 (4.632) | 5.394 | slower |
| page/input/1 | gpu_warm_p95_ms | 4.696 (4.724) | 5.539 | slower |
| page/input/1 | mask_kib | 178 (178) | 177 | changed |
| page/input/1 | mask_resources | 46.000 (46.000) | 41.000 | changed |
| page/input/1 | masks_drawn | 177 (177) | 45.000 | changed |
| page/input/1 | other_commands | 0.000 (0.000) | 35.000 | changed |
| page/input/1 | present_cold_ms | 0.591 (0.597) | 0.539 |  |
| page/input/1 | present_warm_median_ms | 0.501 (0.519) | 0.479 |  |
| page/input/1 | rects | 123 (123) | 15.000 | changed |
| page/input/1.25 | compose_cold_ms | 2.264 (2.279) | 2.211 |  |
| page/input/1.25 | compose_warm_ms | 0.052 (0.053) | 0.045 |  |
| page/input/1.25 | cpu_median_ms | 13.309 (13.333) | 10.045 | faster |
| page/input/1.25 | gpu_first_ms | 8.357 (8.449) | 6.133 | faster |
| page/input/1.25 | gpu_warm_median_ms | 6.075 (6.506) | 5.895 |  |
| page/input/1.25 | gpu_warm_p95_ms | 6.748 (6.846) | 6.536 |  |
| page/input/1.25 | mask_kib | 278 (278) | 278 | changed |
| page/input/1.25 | mask_resources | 46.000 (46.000) | 41.000 | changed |
| page/input/1.25 | masks_drawn | 177 (177) | 45.000 | changed |
| page/input/1.25 | other_commands | 0.000 (0.000) | 35.000 | changed |
| page/input/1.25 | present_cold_ms | 0.645 (0.653) | 0.585 |  |
| page/input/1.25 | present_warm_median_ms | 0.497 (0.504) | 0.480 |  |
| page/input/1.25 | rects | 123 (123) | 15.000 | changed |
| page/input/2 | compose_cold_ms | 3.246 (3.312) | 3.235 |  |
| page/input/2 | compose_warm_ms | 0.053 (0.053) | 0.045 |  |
| page/input/2 | cpu_median_ms | 30.830 (30.956) | 22.492 | faster |
| page/input/2 | gpu_first_ms | 11.062 (11.205) | 13.579 | slower |
| page/input/2 | gpu_warm_median_ms | 9.007 (9.121) | 10.206 | slower |
| page/input/2 | gpu_warm_p95_ms | 9.354 (9.470) | 10.694 | slower |
| page/input/2 | mask_kib | 707 (707) | 706 | changed |
| page/input/2 | mask_resources | 46.000 (46.000) | 41.000 | changed |
| page/input/2 | masks_drawn | 177 (177) | 45.000 | changed |
| page/input/2 | other_commands | 0.000 (0.000) | 35.000 | changed |
| page/input/2 | present_cold_ms | 0.780 (0.789) | 0.703 | faster |
| page/input/2 | present_warm_median_ms | 0.501 (0.511) | 0.483 |  |
| page/input/2 | rects | 123 (123) | 15.000 | changed |
| page/layout/1 | compose_cold_ms | 1.577 (1.631) | 1.535 |  |
| page/layout/1 | compose_warm_ms | 0.064 (0.066) | 0.058 |  |
| page/layout/1 | cpu_median_ms | 6.224 (6.304) | 4.317 | faster |
| page/layout/1 | gpu_first_ms | 3.829 (6.182) | 4.253 |  |
| page/layout/1 | gpu_warm_median_ms | 2.777 (3.495) | 2.879 |  |
| page/layout/1 | gpu_warm_p95_ms | 4.442 (6.736) | 3.688 | faster |
| page/layout/1 | mask_kib | 111 (111) | 111 | changed |
| page/layout/1 | mask_resources | 46.000 (46.000) | 41.000 | changed |
| page/layout/1 | masks_drawn | 131 (131) | 43.000 | changed |
| page/layout/1 | other_commands | 0.000 (0.000) | 23.000 | changed |
| page/layout/1 | present_cold_ms | 1.462 (1.514) | 1.376 |  |
| page/layout/1 | present_warm_median_ms | 0.477 (0.500) | 0.474 |  |
| page/layout/1 | rects | 99.000 (99.000) | 28.000 | changed |
| page/layout/1.25 | compose_cold_ms | 1.573 (1.576) | 1.553 |  |
| page/layout/1.25 | compose_warm_ms | 0.050 (0.056) | 0.045 |  |
| page/layout/1.25 | cpu_median_ms | 9.587 (9.633) | 6.505 | faster |
| page/layout/1.25 | gpu_first_ms | 7.875 (8.051) | 9.362 | slower |
| page/layout/1.25 | gpu_warm_median_ms | 6.152 (6.432) | 6.606 | within noise |
| page/layout/1.25 | gpu_warm_p95_ms | 6.483 (6.555) | 7.029 | slower |
| page/layout/1.25 | mask_kib | 172 (172) | 171 | changed |
| page/layout/1.25 | mask_resources | 46.000 (46.000) | 41.000 | changed |
| page/layout/1.25 | masks_drawn | 131 (131) | 43.000 | changed |
| page/layout/1.25 | other_commands | 0.000 (0.000) | 23.000 | changed |
| page/layout/1.25 | present_cold_ms | 0.962 (0.988) | 0.889 | faster |
| page/layout/1.25 | present_warm_median_ms | 0.469 (0.490) | 0.465 |  |
| page/layout/1.25 | rects | 99.000 (99.000) | 28.000 | changed |
| page/layout/2 | compose_cold_ms | 2.267 (2.289) | 2.248 |  |
| page/layout/2 | compose_warm_ms | 0.050 (0.051) | 0.046 |  |
| page/layout/2 | cpu_median_ms | 22.546 (22.584) | 14.676 | faster |
| page/layout/2 | gpu_first_ms | 10.563 (10.729) | 12.346 | slower |
| page/layout/2 | gpu_warm_median_ms | 8.825 (8.902) | 9.617 | slower |
| page/layout/2 | gpu_warm_p95_ms | 9.140 (9.211) | 10.134 | slower |
| page/layout/2 | mask_kib | 440 (440) | 439 | changed |
| page/layout/2 | mask_resources | 46.000 (46.000) | 41.000 | changed |
| page/layout/2 | masks_drawn | 131 (131) | 43.000 | changed |
| page/layout/2 | other_commands | 0.000 (0.000) | 23.000 | changed |
| page/layout/2 | present_cold_ms | 1.045 (1.110) | 0.973 | faster |
| page/layout/2 | present_warm_median_ms | 0.487 (0.489) | 0.469 |  |
| page/layout/2 | rects | 99.000 (99.000) | 28.000 | changed |
| page/live/1 | compose_cold_ms | 1.997 (2.010) | 1.931 |  |
| page/live/1 | compose_warm_ms | 0.111 (0.118) | 0.089 |  |
| page/live/1 | cpu_median_ms | 8.405 (8.434) | 6.323 | faster |
| page/live/1 | gpu_first_ms | 6.224 (6.402) | 6.576 | within noise |
| page/live/1 | gpu_warm_median_ms | 4.853 (4.881) | 5.066 | within noise |
| page/live/1 | gpu_warm_p95_ms | 4.908 (4.962) | 5.155 | within noise |
| page/live/1 | mask_kib | 192 (192) | 191 | changed |
| page/live/1 | mask_resources | 102 (102) | 94.000 | changed |
| page/live/1 | masks_drawn | 446 (446) | 94.000 | changed |
| page/live/1 | other_commands | 0.000 (0.000) | 88.000 | changed |
| page/live/1 | present_cold_ms | 1.579 (1.638) | 1.433 | faster |
| page/live/1 | present_warm_median_ms | 0.639 (0.656) | 0.618 |  |
| page/live/1 | rects | 343 (343) | 30.000 | changed |
| page/live/1.25 | compose_cold_ms | 2.224 (2.287) | 2.140 |  |
| page/live/1.25 | compose_warm_ms | 0.111 (0.113) | 0.090 |  |
| page/live/1.25 | cpu_median_ms | 13.952 (13.957) | 10.638 | faster |
| page/live/1.25 | gpu_first_ms | 8.609 (8.708) | 5.480 |  |
| page/live/1.25 | gpu_warm_median_ms | 6.653 (6.940) | 6.673 |  |
| page/live/1.25 | gpu_warm_p95_ms | 7.003 (7.038) | 7.066 | within noise |
| page/live/1.25 | mask_kib | 298 (298) | 297 | changed |
| page/live/1.25 | mask_resources | 102 (102) | 94.000 | changed |
| page/live/1.25 | masks_drawn | 446 (446) | 94.000 | changed |
| page/live/1.25 | other_commands | 0.000 (0.000) | 88.000 | changed |
| page/live/1.25 | present_cold_ms | 1.628 (1.649) | 1.487 | faster |
| page/live/1.25 | present_warm_median_ms | 0.637 (0.639) | 0.616 |  |
| page/live/1.25 | rects | 352 (352) | 30.000 | changed |
| page/live/2 | compose_cold_ms | 3.142 (3.248) | 3.029 |  |
| page/live/2 | compose_warm_ms | 0.112 (0.115) | 0.090 |  |
| page/live/2 | cpu_median_ms | 32.979 (32.983) | 24.249 | faster |
| page/live/2 | gpu_first_ms | 11.267 (11.461) | 12.203 | slower |
| page/live/2 | gpu_warm_median_ms | 9.488 (9.540) | 9.684 | within noise |
| page/live/2 | gpu_warm_p95_ms | 9.745 (9.935) | 9.992 | within noise |
| page/live/2 | mask_kib | 761 (761) | 758 | changed |
| page/live/2 | mask_resources | 102 (102) | 94.000 | changed |
| page/live/2 | masks_drawn | 446 (446) | 94.000 | changed |
| page/live/2 | other_commands | 0.000 (0.000) | 88.000 | changed |
| page/live/2 | present_cold_ms | 1.774 (1.834) | 1.632 | faster |
| page/live/2 | present_warm_median_ms | 0.638 (0.657) | 0.619 |  |
| page/live/2 | rects | 352 (352) | 30.000 | changed |
| page/menus/1 | compose_cold_ms | 1.435 (1.469) | 1.405 |  |
| page/menus/1 | compose_warm_ms | 0.045 (0.045) | 0.041 |  |
| page/menus/1 | cpu_median_ms | 5.635 (5.648) | 4.260 | faster |
| page/menus/1 | gpu_first_ms | 5.055 (5.085) | 5.139 | within noise |
| page/menus/1 | gpu_warm_median_ms | 4.038 (4.063) | 4.034 |  |
| page/menus/1 | gpu_warm_p95_ms | 4.129 (4.147) | 4.088 |  |
| page/menus/1 | mask_kib | 136 (136) | 136 | changed |
| page/menus/1 | mask_resources | 31.000 (31.000) | 27.000 | changed |
| page/menus/1 | masks_drawn | 84.000 (84.000) | 28.000 | changed |
| page/menus/1 | other_commands | 0.000 (0.000) | 14.000 | changed |
| page/menus/1 | present_cold_ms | 0.500 (0.531) | 0.457 |  |
| page/menus/1 | present_warm_median_ms | 0.442 (0.450) | 0.434 |  |
| page/menus/1 | rects | 58.000 (58.000) | 15.000 | changed |
| page/menus/1.25 | compose_cold_ms | 1.603 (1.624) | 1.608 |  |
| page/menus/1.25 | compose_warm_ms | 0.045 (0.050) | 0.041 |  |
| page/menus/1.25 | cpu_median_ms | 9.050 (9.059) | 6.899 | faster |
| page/menus/1.25 | gpu_first_ms | 4.323 (7.151) | 4.351 |  |
| page/menus/1.25 | gpu_warm_median_ms | 5.871 (5.934) | 5.904 |  |
| page/menus/1.25 | gpu_warm_p95_ms | 5.991 (5.999) | 6.020 | within noise |
| page/menus/1.25 | mask_kib | 211 (211) | 211 | changed |
| page/menus/1.25 | mask_resources | 31.000 (31.000) | 27.000 | changed |
| page/menus/1.25 | masks_drawn | 84.000 (84.000) | 28.000 | changed |
| page/menus/1.25 | other_commands | 0.000 (0.000) | 14.000 | changed |
| page/menus/1.25 | present_cold_ms | 0.531 (0.570) | 0.508 |  |
| page/menus/1.25 | present_warm_median_ms | 0.443 (0.450) | 0.431 |  |
| page/menus/1.25 | rects | 58.000 (58.000) | 15.000 | changed |
| page/menus/2 | compose_cold_ms | 2.338 (2.358) | 2.329 |  |
| page/menus/2 | compose_warm_ms | 0.045 (0.046) | 0.042 |  |
| page/menus/2 | cpu_median_ms | 21.296 (21.327) | 15.793 | faster |
| page/menus/2 | gpu_first_ms | 9.725 (14.289) | 9.842 |  |
| page/menus/2 | gpu_warm_median_ms | 8.287 (8.330) | 8.328 |  |
| page/menus/2 | gpu_warm_p95_ms | 8.515 (8.617) | 8.491 |  |
| page/menus/2 | mask_kib | 541 (541) | 540 | changed |
| page/menus/2 | mask_resources | 31.000 (31.000) | 27.000 | changed |
| page/menus/2 | masks_drawn | 84.000 (84.000) | 28.000 | changed |
| page/menus/2 | other_commands | 0.000 (0.000) | 14.000 | changed |
| page/menus/2 | present_cold_ms | 0.658 (0.674) | 0.637 |  |
| page/menus/2 | present_warm_median_ms | 0.437 (0.450) | 0.436 |  |
| page/menus/2 | rects | 58.000 (58.000) | 15.000 | changed |
| page/meters/1 | compose_cold_ms | 0.915 (0.921) | 0.886 |  |
| page/meters/1 | compose_warm_ms | 0.046 (0.047) | 0.042 |  |
| page/meters/1 | cpu_median_ms | 4.227 (4.234) | 2.831 | faster |
| page/meters/1 | gpu_first_ms | 5.039 (5.114) | 5.161 | within noise |
| page/meters/1 | gpu_warm_median_ms | 4.034 (4.041) | 4.050 | within noise |
| page/meters/1 | gpu_warm_p95_ms | 4.075 (4.092) | 4.170 | within noise |
| page/meters/1 | mask_kib | 79.273 (79.273) | 78.633 | changed |
| page/meters/1 | mask_resources | 32.000 (32.000) | 26.000 | changed |
| page/meters/1 | masks_drawn | 106 (106) | 26.000 | changed |
| page/meters/1 | other_commands | 0.000 (0.000) | 20.000 | changed |
| page/meters/1 | present_cold_ms | 0.534 (0.560) | 0.499 |  |
| page/meters/1 | present_warm_median_ms | 0.450 (0.457) | 0.438 |  |
| page/meters/1 | rects | 82.000 (82.000) | 26.000 | changed |
| page/meters/1.25 | compose_cold_ms | 1.038 (1.040) | 1.000 |  |
| page/meters/1.25 | compose_warm_ms | 0.046 (0.046) | 0.042 |  |
| page/meters/1.25 | cpu_median_ms | 6.733 (6.754) | 4.537 | faster |
| page/meters/1.25 | gpu_first_ms | 6.926 (7.156) | 4.398 | faster |
| page/meters/1.25 | gpu_warm_median_ms | 5.928 (5.959) | 5.971 | within noise |
| page/meters/1.25 | gpu_warm_p95_ms | 6.019 (6.058) | 6.017 |  |
| page/meters/1.25 | mask_kib | 122 (122) | 121 | changed |
| page/meters/1.25 | mask_resources | 32.000 (32.000) | 26.000 | changed |
| page/meters/1.25 | masks_drawn | 106 (106) | 26.000 | changed |
| page/meters/1.25 | other_commands | 0.000 (0.000) | 20.000 | changed |
| page/meters/1.25 | present_cold_ms | 0.567 (0.597) | 0.538 |  |
| page/meters/1.25 | present_warm_median_ms | 0.451 (0.460) | 0.442 |  |
| page/meters/1.25 | rects | 82.000 (82.000) | 26.000 | changed |
| page/meters/2 | compose_cold_ms | 1.489 (1.520) | 1.433 |  |
| page/meters/2 | compose_warm_ms | 0.047 (0.051) | 0.042 |  |
| page/meters/2 | cpu_median_ms | 16.056 (16.082) | 10.361 | faster |
| page/meters/2 | gpu_first_ms | 9.623 (14.398) | 9.785 |  |
| page/meters/2 | gpu_warm_median_ms | 8.276 (8.340) | 8.279 |  |
| page/meters/2 | gpu_warm_p95_ms | 8.450 (8.537) | 8.585 | within noise |
| page/meters/2 | mask_kib | 314 (314) | 312 | changed |
| page/meters/2 | mask_resources | 32.000 (32.000) | 26.000 | changed |
| page/meters/2 | masks_drawn | 106 (106) | 26.000 | changed |
| page/meters/2 | other_commands | 0.000 (0.000) | 20.000 | changed |
| page/meters/2 | present_cold_ms | 0.679 (0.691) | 0.647 |  |
| page/meters/2 | present_warm_median_ms | 0.457 (0.458) | 0.439 |  |
| page/meters/2 | rects | 82.000 (82.000) | 26.000 | changed |
| page/navigation/1 | compose_cold_ms | 2.371 (2.409) | 2.353 |  |
| page/navigation/1 | compose_warm_ms | 0.054 (0.055) | 0.047 |  |
| page/navigation/1 | cpu_median_ms | 8.944 (8.968) | 6.889 | faster |
| page/navigation/1 | gpu_first_ms | 5.906 (6.039) | 7.256 | slower |
| page/navigation/1 | gpu_warm_median_ms | 4.617 (4.634) | 5.363 | slower |
| page/navigation/1 | gpu_warm_p95_ms | 4.667 (4.691) | 5.430 | slower |
| page/navigation/1 | mask_kib | 216 (216) | 216 | changed |
| page/navigation/1 | mask_resources | 51.000 (51.000) | 46.000 | changed |
| page/navigation/1 | masks_drawn | 164 (164) | 50.000 | changed |
| page/navigation/1 | other_commands | 0.000 (0.000) | 31.000 | changed |
| page/navigation/1 | present_cold_ms | 0.625 (0.708) | 0.588 |  |
| page/navigation/1 | present_warm_median_ms | 0.489 (0.495) | 0.493 |  |
| page/navigation/1 | rects | 115 (115) | 20.000 | changed |
| page/navigation/1.25 | compose_cold_ms | 2.738 (2.776) | 2.702 |  |
| page/navigation/1.25 | compose_warm_ms | 0.055 (0.056) | 0.047 |  |
| page/navigation/1.25 | cpu_median_ms | 14.636 (14.650) | 11.364 | faster |
| page/navigation/1.25 | gpu_first_ms | 8.292 (8.336) | 6.026 |  |
| page/navigation/1.25 | gpu_warm_median_ms | 6.388 (6.481) | 6.402 |  |
| page/navigation/1.25 | gpu_warm_p95_ms | 6.714 (6.743) | 6.946 | within noise |
| page/navigation/1.25 | mask_kib | 340 (340) | 339 | changed |
| page/navigation/1.25 | mask_resources | 51.000 (51.000) | 46.000 | changed |
| page/navigation/1.25 | masks_drawn | 164 (164) | 50.000 | changed |
| page/navigation/1.25 | other_commands | 0.000 (0.000) | 31.000 | changed |
| page/navigation/1.25 | present_cold_ms | 0.680 (0.697) | 0.636 |  |
| page/navigation/1.25 | present_warm_median_ms | 0.494 (0.498) | 0.489 |  |
| page/navigation/1.25 | rects | 115 (115) | 20.000 | changed |
| page/navigation/2 | compose_cold_ms | 4.021 (4.053) | 4.037 |  |
| page/navigation/2 | compose_warm_ms | 0.054 (0.059) | 0.048 |  |
| page/navigation/2 | cpu_median_ms | 33.524 (33.570) | 25.181 | faster |
| page/navigation/2 | gpu_first_ms | 11.013 (11.029) | 13.567 | slower |
| page/navigation/2 | gpu_warm_median_ms | 9.004 (9.161) | 9.925 | slower |
| page/navigation/2 | gpu_warm_p95_ms | 9.268 (9.375) | 10.483 | slower |
| page/navigation/2 | mask_kib | 863 (863) | 861 | changed |
| page/navigation/2 | mask_resources | 51.000 (51.000) | 46.000 | changed |
| page/navigation/2 | masks_drawn | 164 (164) | 50.000 | changed |
| page/navigation/2 | other_commands | 0.000 (0.000) | 31.000 | changed |
| page/navigation/2 | present_cold_ms | 0.844 (0.856) | 0.774 | faster |
| page/navigation/2 | present_warm_median_ms | 0.499 (0.512) | 0.488 |  |
| page/navigation/2 | rects | 115 (115) | 20.000 | changed |
| page/numbers/1 | compose_cold_ms | 2.018 (2.043) | 1.994 |  |
| page/numbers/1 | compose_warm_ms | 0.050 (0.050) | 0.044 |  |
| page/numbers/1 | cpu_median_ms | 7.105 (7.126) | 5.187 | faster |
| page/numbers/1 | gpu_first_ms | 5.656 (5.773) | 6.830 | slower |
| page/numbers/1 | gpu_warm_median_ms | 4.479 (4.498) | 5.093 | slower |
| page/numbers/1 | gpu_warm_p95_ms | 4.595 (4.644) | 5.170 | slower |
| page/numbers/1 | mask_kib | 154 (154) | 154 | changed |
| page/numbers/1 | mask_resources | 48.000 (48.000) | 43.000 | changed |
| page/numbers/1 | masks_drawn | 149 (149) | 45.000 | changed |
| page/numbers/1 | other_commands | 0.000 (0.000) | 22.000 | changed |
| page/numbers/1 | present_cold_ms | 0.602 (0.629) | 0.560 |  |
| page/numbers/1 | present_warm_median_ms | 0.488 (0.493) | 0.471 |  |
| page/numbers/1 | rects | 104 (104) | 14.000 | changed |
| page/numbers/1.25 | compose_cold_ms | 2.290 (2.312) | 2.264 |  |
| page/numbers/1.25 | compose_warm_ms | 0.050 (0.051) | 0.044 |  |
| page/numbers/1.25 | cpu_median_ms | 11.612 (11.639) | 8.571 | faster |
| page/numbers/1.25 | gpu_first_ms | 8.054 (8.105) | 5.689 | faster |
| page/numbers/1.25 | gpu_warm_median_ms | 6.488 (6.513) | 6.117 |  |
| page/numbers/1.25 | gpu_warm_p95_ms | 6.607 (6.640) | 6.951 | within noise |
| page/numbers/1.25 | mask_kib | 243 (243) | 242 | changed |
| page/numbers/1.25 | mask_resources | 48.000 (48.000) | 43.000 | changed |
| page/numbers/1.25 | masks_drawn | 149 (149) | 45.000 | changed |
| page/numbers/1.25 | other_commands | 0.000 (0.000) | 22.000 | changed |
| page/numbers/1.25 | present_cold_ms | 0.662 (0.680) | 0.620 |  |
| page/numbers/1.25 | present_warm_median_ms | 0.478 (0.509) | 0.470 |  |
| page/numbers/1.25 | rects | 104 (104) | 14.000 | changed |
| page/numbers/2 | compose_cold_ms | 3.278 (3.301) | 3.291 |  |
| page/numbers/2 | compose_warm_ms | 0.050 (0.055) | 0.044 |  |
| page/numbers/2 | cpu_median_ms | 26.798 (26.924) | 19.042 | faster |
| page/numbers/2 | gpu_first_ms | 10.740 (15.543) | 12.826 |  |
| page/numbers/2 | gpu_warm_median_ms | 9.007 (9.027) | 9.780 | slower |
| page/numbers/2 | gpu_warm_p95_ms | 9.237 (9.327) | 10.396 | slower |
| page/numbers/2 | mask_kib | 616 (616) | 615 | changed |
| page/numbers/2 | mask_resources | 48.000 (48.000) | 43.000 | changed |
| page/numbers/2 | masks_drawn | 149 (149) | 45.000 | changed |
| page/numbers/2 | other_commands | 0.000 (0.000) | 22.000 | changed |
| page/numbers/2 | present_cold_ms | 0.765 (0.780) | 0.737 |  |
| page/numbers/2 | present_warm_median_ms | 0.488 (0.494) | 0.472 |  |
| page/numbers/2 | rects | 104 (104) | 14.000 | changed |
| page/overlay/1 | compose_cold_ms | 1.504 (1.520) | 1.482 |  |
| page/overlay/1 | compose_warm_ms | 0.046 (0.046) | 0.041 |  |
| page/overlay/1 | cpu_median_ms | 6.430 (6.452) | 4.876 | faster |
| page/overlay/1 | gpu_first_ms | 5.223 (5.370) | 5.912 | slower |
| page/overlay/1 | gpu_warm_median_ms | 4.212 (4.262) | 4.588 | slower |
| page/overlay/1 | gpu_warm_p95_ms | 4.292 (4.330) | 4.629 | slower |
| page/overlay/1 | mask_kib | 155 (155) | 154 | changed |
| page/overlay/1 | mask_resources | 34.000 (34.000) | 27.000 | changed |
| page/overlay/1 | masks_drawn | 111 (111) | 27.000 | changed |
| page/overlay/1 | other_commands | 0.000 (0.000) | 21.000 | changed |
| page/overlay/1 | present_cold_ms | 0.486 (0.496) | 0.443 |  |
| page/overlay/1 | present_warm_median_ms | 0.461 (0.468) | 0.434 |  |
| page/overlay/1 | rects | 73.000 (73.000) | 13.000 | changed |
| page/overlay/1.25 | compose_cold_ms | 1.720 (1.741) | 1.697 |  |
| page/overlay/1.25 | compose_warm_ms | 0.047 (0.051) | 0.040 |  |
| page/overlay/1.25 | cpu_median_ms | 10.559 (10.617) | 8.068 | faster |
| page/overlay/1.25 | gpu_first_ms | 7.351 (7.418) | 8.370 | slower |
| page/overlay/1.25 | gpu_warm_median_ms | 6.147 (6.173) | 6.432 | within noise |
| page/overlay/1.25 | gpu_warm_p95_ms | 6.223 (6.312) | 6.747 | slower |
| page/overlay/1.25 | mask_kib | 241 (241) | 240 | changed |
| page/overlay/1.25 | mask_resources | 34.000 (34.000) | 27.000 | changed |
| page/overlay/1.25 | masks_drawn | 111 (111) | 27.000 | changed |
| page/overlay/1.25 | other_commands | 0.000 (0.000) | 21.000 | changed |
| page/overlay/1.25 | present_cold_ms | 0.529 (0.576) | 0.477 |  |
| page/overlay/1.25 | present_warm_median_ms | 0.453 (0.462) | 0.444 |  |
| page/overlay/1.25 | rects | 73.000 (73.000) | 13.000 | changed |
| page/overlay/2 | compose_cold_ms | 2.525 (2.576) | 2.498 |  |
| page/overlay/2 | compose_warm_ms | 0.046 (0.047) | 0.041 |  |
| page/overlay/2 | cpu_median_ms | 24.331 (24.356) | 18.081 | faster |
| page/overlay/2 | gpu_first_ms | 10.175 (10.244) | 11.064 | slower |
| page/overlay/2 | gpu_warm_median_ms | 8.541 (8.551) | 9.106 | slower |
| page/overlay/2 | gpu_warm_p95_ms | 8.771 (8.827) | 9.305 | slower |
| page/overlay/2 | mask_kib | 617 (617) | 614 | changed |
| page/overlay/2 | mask_resources | 34.000 (34.000) | 27.000 | changed |
| page/overlay/2 | masks_drawn | 111 (111) | 27.000 | changed |
| page/overlay/2 | other_commands | 0.000 (0.000) | 21.000 | changed |
| page/overlay/2 | present_cold_ms | 0.646 (0.683) | 0.584 | faster |
| page/overlay/2 | present_warm_median_ms | 0.456 (0.461) | 0.445 |  |
| page/overlay/2 | rects | 73.000 (73.000) | 13.000 | changed |
| page/profiling/1 | compose_cold_ms | 1.033 (1.042) | 1.025 |  |
| page/profiling/1 | compose_warm_ms | 0.046 (0.047) | 0.041 |  |
| page/profiling/1 | cpu_median_ms | 4.712 (4.717) | 3.262 | faster |
| page/profiling/1 | gpu_first_ms | 5.125 (5.225) | 5.694 | slower |
| page/profiling/1 | gpu_warm_median_ms | 4.041 (4.129) | 4.400 | slower |
| page/profiling/1 | gpu_warm_p95_ms | 4.074 (4.194) | 4.462 | slower |
| page/profiling/1 | mask_kib | 92.729 (92.729) | 92.416 | changed |
| page/profiling/1 | mask_resources | 32.000 (32.000) | 27.000 | changed |
| page/profiling/1 | masks_drawn | 107 (107) | 27.000 | changed |
| page/profiling/1 | other_commands | 0.000 (0.000) | 20.000 | changed |
| page/profiling/1 | present_cold_ms | 0.482 (0.541) | 0.445 |  |
| page/profiling/1 | present_warm_median_ms | 0.452 (0.457) | 0.440 |  |
| page/profiling/1 | rects | 75.000 (75.000) | 13.000 | changed |
| page/profiling/1.25 | compose_cold_ms | 1.167 (1.182) | 1.152 |  |
| page/profiling/1.25 | compose_warm_ms | 0.046 (0.047) | 0.041 |  |
| page/profiling/1.25 | cpu_median_ms | 7.592 (7.602) | 5.294 | faster |
| page/profiling/1.25 | gpu_first_ms | 7.226 (7.262) | 7.973 | slower |
| page/profiling/1.25 | gpu_warm_median_ms | 6.014 (6.036) | 6.444 | slower |
| page/profiling/1.25 | gpu_warm_p95_ms | 6.051 (6.132) | 6.534 | slower |
| page/profiling/1.25 | mask_kib | 143 (143) | 143 | changed |
| page/profiling/1.25 | mask_resources | 32.000 (32.000) | 27.000 | changed |
| page/profiling/1.25 | masks_drawn | 107 (107) | 27.000 | changed |
| page/profiling/1.25 | other_commands | 0.000 (0.000) | 20.000 | changed |
| page/profiling/1.25 | present_cold_ms | 0.520 (0.525) | 0.483 |  |
| page/profiling/1.25 | present_warm_median_ms | 0.460 (0.467) | 0.434 |  |
| page/profiling/1.25 | rects | 75.000 (75.000) | 13.000 | changed |
| page/profiling/2 | compose_cold_ms | 1.675 (1.692) | 1.651 |  |
| page/profiling/2 | compose_warm_ms | 0.046 (0.047) | 0.041 |  |
| page/profiling/2 | cpu_median_ms | 17.897 (17.965) | 12.068 | faster |
| page/profiling/2 | gpu_first_ms | 9.826 (9.903) | 10.328 | within noise |
| page/profiling/2 | gpu_warm_median_ms | 8.389 (8.437) | 8.821 | within noise |
| page/profiling/2 | gpu_warm_p95_ms | 8.511 (8.558) | 9.119 | slower |
| page/profiling/2 | mask_kib | 368 (368) | 366 | changed |
| page/profiling/2 | mask_resources | 32.000 (32.000) | 27.000 | changed |
| page/profiling/2 | masks_drawn | 107 (107) | 27.000 | changed |
| page/profiling/2 | other_commands | 0.000 (0.000) | 20.000 | changed |
| page/profiling/2 | present_cold_ms | 0.621 (0.621) | 0.593 |  |
| page/profiling/2 | present_warm_median_ms | 0.455 (0.467) | 0.443 |  |
| page/profiling/2 | rects | 75.000 (75.000) | 13.000 | changed |
| page/progress/1 | compose_cold_ms | 1.860 (1.862) | 1.817 |  |
| page/progress/1 | compose_warm_ms | 0.053 (0.058) | 0.046 |  |
| page/progress/1 | cpu_median_ms | 7.295 (7.301) | 5.640 | faster |
| page/progress/1 | gpu_first_ms | 5.571 (5.633) | 6.593 | slower |
| page/progress/1 | gpu_warm_median_ms | 4.444 (4.457) | 4.867 | slower |
| page/progress/1 | gpu_warm_p95_ms | 4.475 (4.548) | 4.992 | slower |
| page/progress/1 | mask_kib | 172 (172) | 172 | changed |
| page/progress/1 | mask_resources | 56.000 (56.000) | 48.000 | changed |
| page/progress/1 | masks_drawn | 181 (181) | 49.000 | changed |
| page/progress/1 | other_commands | 0.000 (0.000) | 35.000 | changed |
| page/progress/1 | present_cold_ms | 0.643 (0.787) | 0.592 |  |
| page/progress/1 | present_warm_median_ms | 0.505 (0.520) | 0.497 |  |
| page/progress/1 | rects | 120 (120) | 20.000 | changed |
| page/progress/1.25 | compose_cold_ms | 2.145 (2.162) | 2.089 |  |
| page/progress/1.25 | compose_warm_ms | 0.054 (0.055) | 0.046 |  |
| page/progress/1.25 | cpu_median_ms | 11.820 (11.842) | 9.173 | faster |
| page/progress/1.25 | gpu_first_ms | 4.709 (7.861) | 5.553 |  |
| page/progress/1.25 | gpu_warm_median_ms | 6.423 (6.479) | 5.839 |  |
| page/progress/1.25 | gpu_warm_p95_ms | 6.518 (6.559) | 6.995 | slower |
| page/progress/1.25 | mask_kib | 267 (267) | 266 | changed |
| page/progress/1.25 | mask_resources | 56.000 (56.000) | 48.000 | changed |
| page/progress/1.25 | masks_drawn | 183 (183) | 49.000 | changed |
| page/progress/1.25 | other_commands | 0.000 (0.000) | 35.000 | changed |
| page/progress/1.25 | present_cold_ms | 0.724 (0.744) | 0.656 |  |
| page/progress/1.25 | present_warm_median_ms | 0.511 (0.517) | 0.504 |  |
| page/progress/1.25 | rects | 121 (121) | 20.000 | changed |
| page/progress/2 | compose_cold_ms | 3.074 (3.097) | 3.041 |  |
| page/progress/2 | compose_warm_ms | 0.054 (0.059) | 0.046 |  |
| page/progress/2 | cpu_median_ms | 27.552 (27.654) | 20.769 | faster |
| page/progress/2 | gpu_first_ms | 10.404 (10.640) | 12.407 | slower |
| page/progress/2 | gpu_warm_median_ms | 8.848 (8.923) | 9.764 | slower |
| page/progress/2 | gpu_warm_p95_ms | 9.073 (9.292) | 10.129 | slower |
| page/progress/2 | mask_kib | 682 (682) | 679 | changed |
| page/progress/2 | mask_resources | 56.000 (56.000) | 48.000 | changed |
| page/progress/2 | masks_drawn | 183 (183) | 49.000 | changed |
| page/progress/2 | other_commands | 0.000 (0.000) | 35.000 | changed |
| page/progress/2 | present_cold_ms | 0.849 (0.878) | 0.785 |  |
| page/progress/2 | present_warm_median_ms | 0.513 (0.523) | 0.504 |  |
| page/progress/2 | rects | 120 (120) | 20.000 | changed |
| page/properties/1 | compose_cold_ms | 2.091 (2.123) | 1.596 | faster |
| page/properties/1 | compose_warm_ms | 0.055 (0.055) | 0.047 |  |
| page/properties/1 | cpu_median_ms | 6.713 (6.757) | 4.748 | faster |
| page/properties/1 | gpu_first_ms | 5.910 (6.070) | 7.588 | slower |
| page/properties/1 | gpu_warm_median_ms | 4.559 (4.637) | 5.583 | slower |
| page/properties/1 | gpu_warm_p95_ms | 4.651 (4.714) | 5.663 | slower |
| page/properties/1 | mask_kib | 126 (126) | 123 | changed |
| page/properties/1 | mask_resources | 65.000 (65.000) | 56.000 | changed |
| page/properties/1 | masks_drawn | 192 (192) | 60.000 | changed |
| page/properties/1 | other_commands | 0.000 (0.000) | 34.000 | changed |
| page/properties/1 | present_cold_ms | 0.922 (0.940) | 0.633 | faster |
| page/properties/1 | present_warm_median_ms | 0.515 (0.528) | 0.497 |  |
| page/properties/1 | rects | 121 (121) | 17.000 | changed |
| page/properties/1.25 | compose_cold_ms | 2.486 (2.542) | 1.814 | faster |
| page/properties/1.25 | compose_warm_ms | 0.054 (0.055) | 0.047 |  |
| page/properties/1.25 | cpu_median_ms | 10.832 (10.850) | 7.586 | faster |
| page/properties/1.25 | gpu_first_ms | 8.335 (8.387) | 6.285 | faster |
| page/properties/1.25 | gpu_warm_median_ms | 6.175 (6.439) | 6.189 |  |
| page/properties/1.25 | gpu_warm_p95_ms | 6.678 (6.760) | 6.674 |  |
| page/properties/1.25 | mask_kib | 201 (201) | 196 | changed |
| page/properties/1.25 | mask_resources | 65.000 (65.000) | 56.000 | changed |
| page/properties/1.25 | masks_drawn | 192 (192) | 60.000 | changed |
| page/properties/1.25 | other_commands | 0.000 (0.000) | 34.000 | changed |
| page/properties/1.25 | present_cold_ms | 0.987 (1.018) | 0.693 | faster |
| page/properties/1.25 | present_warm_median_ms | 0.526 (0.528) | 0.509 |  |
| page/properties/1.25 | rects | 121 (121) | 17.000 | changed |
| page/properties/2 | compose_cold_ms | 4.069 (4.119) | 2.579 | faster |
| page/properties/2 | compose_warm_ms | 0.054 (0.056) | 0.047 |  |
| page/properties/2 | cpu_median_ms | 25.554 (25.607) | 17.429 | faster |
| page/properties/2 | gpu_first_ms | 11.080 (11.160) | 13.782 | slower |
| page/properties/2 | gpu_warm_median_ms | 9.142 (9.211) | 10.047 | slower |
| page/properties/2 | gpu_warm_p95_ms | 9.371 (9.516) | 10.665 | slower |
| page/properties/2 | mask_kib | 502 (502) | 489 | changed |
| page/properties/2 | mask_resources | 65.000 (65.000) | 56.000 | changed |
| page/properties/2 | masks_drawn | 192 (192) | 60.000 | changed |
| page/properties/2 | other_commands | 0.000 (0.000) | 34.000 | changed |
| page/properties/2 | present_cold_ms | 1.101 (1.147) | 0.808 | faster |
| page/properties/2 | present_warm_median_ms | 0.513 (0.523) | 0.512 |  |
| page/properties/2 | rects | 121 (121) | 17.000 | changed |
| page/scroll/1 | compose_cold_ms | 1.338 (1.344) | 1.296 |  |
| page/scroll/1 | compose_warm_ms | 0.054 (0.059) | 0.047 |  |
| page/scroll/1 | cpu_median_ms | 5.573 (5.624) | 3.907 | faster |
| page/scroll/1 | gpu_first_ms | 5.485 (5.563) | 6.663 | slower |
| page/scroll/1 | gpu_warm_median_ms | 4.389 (4.396) | 4.811 | slower |
| page/scroll/1 | gpu_warm_p95_ms | 4.430 (4.479) | 4.992 | slower |
| page/scroll/1 | mask_kib | 104 (104) | 103 | changed |
| page/scroll/1 | mask_resources | 47.000 (47.000) | 42.000 | changed |
| page/scroll/1 | masks_drawn | 165 (165) | 43.000 | changed |
| page/scroll/1 | other_commands | 0.000 (0.000) | 32.000 | changed |
| page/scroll/1 | present_cold_ms | 0.613 (0.651) | 0.550 | faster |
| page/scroll/1 | present_warm_median_ms | 0.503 (0.512) | 0.481 |  |
| page/scroll/1 | rects | 121 (121) | 21.000 | changed |
| page/scroll/1.25 | compose_cold_ms | 1.516 (1.522) | 1.483 |  |
| page/scroll/1.25 | compose_warm_ms | 0.055 (0.056) | 0.047 |  |
| page/scroll/1.25 | cpu_median_ms | 9.030 (9.060) | 6.331 | faster |
| page/scroll/1.25 | gpu_first_ms | 7.524 (7.759) | 5.535 |  |
| page/scroll/1.25 | gpu_warm_median_ms | 6.039 (6.385) | 6.071 |  |
| page/scroll/1.25 | gpu_warm_p95_ms | 6.206 (6.486) | 6.571 | within noise |
| page/scroll/1.25 | mask_kib | 162 (162) | 161 | changed |
| page/scroll/1.25 | mask_resources | 47.000 (47.000) | 42.000 | changed |
| page/scroll/1.25 | masks_drawn | 165 (165) | 43.000 | changed |
| page/scroll/1.25 | other_commands | 0.000 (0.000) | 32.000 | changed |
| page/scroll/1.25 | present_cold_ms | 0.647 (0.705) | 0.594 |  |
| page/scroll/1.25 | present_warm_median_ms | 0.504 (0.514) | 0.488 |  |
| page/scroll/1.25 | rects | 121 (121) | 21.000 | changed |
| page/scroll/2 | compose_cold_ms | 2.142 (2.152) | 2.126 |  |
| page/scroll/2 | compose_warm_ms | 0.054 (0.056) | 0.047 |  |
| page/scroll/2 | cpu_median_ms | 21.150 (21.348) | 14.318 | faster |
| page/scroll/2 | gpu_first_ms | 10.547 (10.595) | 12.181 | slower |
| page/scroll/2 | gpu_warm_median_ms | 8.810 (8.868) | 9.495 | slower |
| page/scroll/2 | gpu_warm_p95_ms | 9.090 (9.243) | 9.848 | slower |
| page/scroll/2 | mask_kib | 413 (413) | 412 | changed |
| page/scroll/2 | mask_resources | 47.000 (47.000) | 42.000 | changed |
| page/scroll/2 | masks_drawn | 165 (165) | 43.000 | changed |
| page/scroll/2 | other_commands | 0.000 (0.000) | 32.000 | changed |
| page/scroll/2 | present_cold_ms | 0.766 (0.795) | 0.700 | faster |
| page/scroll/2 | present_warm_median_ms | 0.502 (0.508) | 0.484 |  |
| page/scroll/2 | rects | 121 (121) | 21.000 | changed |
| page/sections/1 | compose_cold_ms | 1.642 (1.651) | 1.628 |  |
| page/sections/1 | compose_warm_ms | 0.050 (0.051) | 0.044 |  |
| page/sections/1 | cpu_median_ms | 7.168 (7.174) | 5.169 | faster |
| page/sections/1 | gpu_first_ms | 5.741 (5.844) | 7.111 | slower |
| page/sections/1 | gpu_warm_median_ms | 4.499 (4.514) | 5.173 | slower |
| page/sections/1 | gpu_warm_p95_ms | 4.588 (4.612) | 5.321 | slower |
| page/sections/1 | mask_kib | 152 (152) | 152 | changed |
| page/sections/1 | mask_resources | 46.000 (46.000) | 41.000 | changed |
| page/sections/1 | masks_drawn | 153 (153) | 53.000 | changed |
| page/sections/1 | other_commands | 0.000 (0.000) | 25.000 | changed |
| page/sections/1 | present_cold_ms | 0.584 (0.640) | 0.547 |  |
| page/sections/1 | present_warm_median_ms | 0.494 (0.502) | 0.484 |  |
| page/sections/1 | rects | 103 (103) | 22.000 | changed |
| page/sections/1.25 | compose_cold_ms | 1.864 (1.883) | 1.861 |  |
| page/sections/1.25 | compose_warm_ms | 0.051 (0.051) | 0.044 |  |
| page/sections/1.25 | cpu_median_ms | 11.613 (11.650) | 8.458 | faster |
| page/sections/1.25 | gpu_first_ms | 4.923 (8.133) | 5.819 |  |
| page/sections/1.25 | gpu_warm_median_ms | 5.911 (6.295) | 5.758 |  |
| page/sections/1.25 | gpu_warm_p95_ms | 6.598 (6.639) | 6.341 |  |
| page/sections/1.25 | mask_kib | 236 (236) | 236 | changed |
| page/sections/1.25 | mask_resources | 46.000 (46.000) | 41.000 | changed |
| page/sections/1.25 | masks_drawn | 153 (153) | 53.000 | changed |
| page/sections/1.25 | other_commands | 0.000 (0.000) | 25.000 | changed |
| page/sections/1.25 | present_cold_ms | 0.629 (0.658) | 0.601 |  |
| page/sections/1.25 | present_warm_median_ms | 0.498 (0.501) | 0.485 |  |
| page/sections/1.25 | rects | 103 (103) | 22.000 | changed |
| page/sections/2 | compose_cold_ms | 2.744 (2.795) | 2.719 |  |
| page/sections/2 | compose_warm_ms | 0.050 (0.051) | 0.045 |  |
| page/sections/2 | cpu_median_ms | 27.147 (27.199) | 19.118 | faster |
| page/sections/2 | gpu_first_ms | 10.937 (15.635) | 13.163 |  |
| page/sections/2 | gpu_warm_median_ms | 9.026 (9.045) | 9.905 | slower |
| page/sections/2 | gpu_warm_p95_ms | 9.240 (9.334) | 10.565 | slower |
| page/sections/2 | mask_kib | 602 (602) | 601 | changed |
| page/sections/2 | mask_resources | 46.000 (46.000) | 41.000 | changed |
| page/sections/2 | masks_drawn | 153 (153) | 53.000 | changed |
| page/sections/2 | other_commands | 0.000 (0.000) | 25.000 | changed |
| page/sections/2 | present_cold_ms | 0.766 (0.776) | 0.718 |  |
| page/sections/2 | present_warm_median_ms | 0.501 (0.506) | 0.479 |  |
| page/sections/2 | rects | 103 (103) | 22.000 | changed |
| page/shapes/1 | compose_cold_ms | 0.979 (0.980) | 0.958 |  |
| page/shapes/1 | compose_warm_ms | 0.047 (0.047) | 0.042 |  |
| page/shapes/1 | cpu_median_ms | 5.989 (5.998) | 4.394 | faster |
| page/shapes/1 | gpu_first_ms | 5.339 (5.462) | 6.066 | slower |
| page/shapes/1 | gpu_warm_median_ms | 4.199 (4.318) | 4.677 | slower |
| page/shapes/1 | gpu_warm_p95_ms | 4.308 (4.383) | 4.734 | slower |
| page/shapes/1 | mask_kib | 81.385 (81.385) | 81.072 | changed |
| page/shapes/1 | mask_resources | 31.000 (31.000) | 26.000 | changed |
| page/shapes/1 | masks_drawn | 106 (106) | 26.000 | changed |
| page/shapes/1 | other_commands | 0.000 (0.000) | 20.000 | changed |
| page/shapes/1 | present_cold_ms | 0.882 (0.889) | 0.868 |  |
| page/shapes/1 | present_warm_median_ms | 0.471 (0.490) | 0.465 |  |
| page/shapes/1 | rects | 75.000 (75.000) | 13.000 | changed |
| page/shapes/1.25 | compose_cold_ms | 1.107 (1.112) | 1.086 |  |
| page/shapes/1.25 | compose_warm_ms | 0.047 (0.047) | 0.042 |  |
| page/shapes/1.25 | cpu_median_ms | 9.620 (9.636) | 7.050 | faster |
| page/shapes/1.25 | gpu_first_ms | 7.503 (7.671) | 5.207 |  |
| page/shapes/1.25 | gpu_warm_median_ms | 6.004 (6.294) | 6.537 | within noise |
| page/shapes/1.25 | gpu_warm_p95_ms | 6.308 (6.385) | 6.795 | slower |
| page/shapes/1.25 | mask_kib | 126 (126) | 125 | changed |
| page/shapes/1.25 | mask_resources | 31.000 (31.000) | 26.000 | changed |
| page/shapes/1.25 | masks_drawn | 106 (106) | 26.000 | changed |
| page/shapes/1.25 | other_commands | 0.000 (0.000) | 20.000 | changed |
| page/shapes/1.25 | present_cold_ms | 1.056 (1.126) | 1.030 |  |
| page/shapes/1.25 | present_warm_median_ms | 0.477 (0.495) | 0.478 |  |
| page/shapes/1.25 | rects | 75.000 (75.000) | 13.000 | changed |
| page/shapes/2 | compose_cold_ms | 1.571 (1.595) | 1.546 |  |
| page/shapes/2 | compose_warm_ms | 0.047 (0.047) | 0.042 |  |
| page/shapes/2 | cpu_median_ms | 22.960 (23.010) | 16.377 | faster |
| page/shapes/2 | gpu_first_ms | 10.326 (10.401) | 11.107 | slower |
| page/shapes/2 | gpu_warm_median_ms | 8.650 (8.680) | 9.070 | within noise |
| page/shapes/2 | gpu_warm_p95_ms | 8.948 (8.996) | 9.368 | within noise |
| page/shapes/2 | mask_kib | 322 (322) | 321 | changed |
| page/shapes/2 | mask_resources | 31.000 (31.000) | 26.000 | changed |
| page/shapes/2 | masks_drawn | 106 (106) | 26.000 | changed |
| page/shapes/2 | other_commands | 0.000 (0.000) | 20.000 | changed |
| page/shapes/2 | present_cold_ms | 1.502 (1.536) | 1.484 |  |
| page/shapes/2 | present_warm_median_ms | 0.472 (0.488) | 0.469 |  |
| page/shapes/2 | rects | 75.000 (75.000) | 13.000 | changed |
| page/shells/1 | compose_cold_ms | 1.443 (1.455) | 1.379 |  |
| page/shells/1 | compose_warm_ms | 0.056 (0.060) | 0.046 |  |
| page/shells/1 | cpu_median_ms | 6.974 (7.007) | 4.380 | faster |
| page/shells/1 | gpu_first_ms | 6.092 (6.249) | 7.263 | slower |
| page/shells/1 | gpu_warm_median_ms | 4.640 (4.756) | 5.248 | slower |
| page/shells/1 | gpu_warm_p95_ms | 4.796 (4.878) | 5.597 | slower |
| page/shells/1 | mask_kib | 116 (116) | 116 | changed |
| page/shells/1 | mask_resources | 46.000 (46.000) | 40.000 | changed |
| page/shells/1 | masks_drawn | 173 (173) | 45.000 | changed |
| page/shells/1 | other_commands | 0.000 (0.000) | 33.000 | changed |
| page/shells/1 | present_cold_ms | 0.585 (0.737) | 0.555 |  |
| page/shells/1 | present_warm_median_ms | 0.469 (0.495) | 0.463 |  |
| page/shells/1 | rects | 125 (125) | 28.000 | changed |
| page/shells/1.25 | compose_cold_ms | 1.562 (1.582) | 1.538 |  |
| page/shells/1.25 | compose_warm_ms | 0.054 (0.054) | 0.046 |  |
| page/shells/1.25 | cpu_median_ms | 11.104 (11.115) | 7.018 | faster |
| page/shells/1.25 | gpu_first_ms | 5.162 (8.786) | 6.237 |  |
| page/shells/1.25 | gpu_warm_median_ms | 6.418 (6.906) | 5.840 |  |
| page/shells/1.25 | gpu_warm_p95_ms | 6.934 (6.995) | 6.702 |  |
| page/shells/1.25 | mask_kib | 180 (180) | 179 | changed |
| page/shells/1.25 | mask_resources | 46.000 (46.000) | 40.000 | changed |
| page/shells/1.25 | masks_drawn | 173 (173) | 45.000 | changed |
| page/shells/1.25 | other_commands | 0.000 (0.000) | 33.000 | changed |
| page/shells/1.25 | present_cold_ms | 0.616 (0.650) | 0.596 |  |
| page/shells/1.25 | present_warm_median_ms | 0.501 (0.513) | 0.486 |  |
| page/shells/1.25 | rects | 125 (125) | 28.000 | changed |
| page/shells/2 | compose_cold_ms | 2.263 (2.274) | 2.231 |  |
| page/shells/2 | compose_warm_ms | 0.053 (0.055) | 0.046 |  |
| page/shells/2 | cpu_median_ms | 26.397 (26.495) | 15.610 | faster |
| page/shells/2 | gpu_first_ms | 11.341 (11.660) | 13.645 | slower |
| page/shells/2 | gpu_warm_median_ms | 9.285 (9.396) | 9.875 | slower |
| page/shells/2 | gpu_warm_p95_ms | 9.534 (9.627) | 10.633 | slower |
| page/shells/2 | mask_kib | 461 (461) | 460 | changed |
| page/shells/2 | mask_resources | 46.000 (46.000) | 40.000 | changed |
| page/shells/2 | masks_drawn | 173 (173) | 45.000 | changed |
| page/shells/2 | other_commands | 0.000 (0.000) | 33.000 | changed |
| page/shells/2 | present_cold_ms | 0.735 (0.740) | 0.684 |  |
| page/shells/2 | present_warm_median_ms | 0.499 (0.501) | 0.486 |  |
| page/shells/2 | rects | 125 (125) | 28.000 | changed |
| page/splits/1 | compose_cold_ms | 1.429 (1.807) | 1.392 |  |
| page/splits/1 | compose_warm_ms | 0.063 (0.066) | 0.055 |  |
| page/splits/1 | cpu_median_ms | 6.279 (6.481) | 3.886 | faster |
| page/splits/1 | gpu_first_ms | 3.097 (5.226) | 3.577 |  |
| page/splits/1 | gpu_warm_median_ms | 3.424 (4.020) | 3.166 |  |
| page/splits/1 | gpu_warm_p95_ms | 4.176 (4.426) | 3.978 |  |
| page/splits/1 | mask_kib | 94.724 (94.724) | 94.402 | changed |
| page/splits/1 | mask_resources | 42.000 (42.000) | 36.000 | changed |
| page/splits/1 | masks_drawn | 136 (136) | 38.000 | changed |
| page/splits/1 | other_commands | 0.000 (0.000) | 26.000 | changed |
| page/splits/1 | present_cold_ms | 0.605 (0.629) | 0.583 |  |
| page/splits/1 | present_warm_median_ms | 0.302 (0.319) | 0.302 |  |
| page/splits/1 | rects | 99.000 (99.000) | 24.000 | changed |
| page/splits/1.25 | compose_cold_ms | 1.455 (1.483) | 1.430 |  |
| page/splits/1.25 | compose_warm_ms | 0.054 (0.055) | 0.044 |  |
| page/splits/1.25 | cpu_median_ms | 9.401 (9.425) | 5.863 | faster |
| page/splits/1.25 | gpu_first_ms | 8.086 (8.239) | 5.724 | faster |
| page/splits/1.25 | gpu_warm_median_ms | 6.085 (6.346) | 5.986 |  |
| page/splits/1.25 | gpu_warm_p95_ms | 6.529 (6.647) | 6.754 | within noise |
| page/splits/1.25 | mask_kib | 148 (148) | 147 | changed |
| page/splits/1.25 | mask_resources | 42.000 (42.000) | 36.000 | changed |
| page/splits/1.25 | masks_drawn | 136 (136) | 38.000 | changed |
| page/splits/1.25 | other_commands | 0.000 (0.000) | 26.000 | changed |
| page/splits/1.25 | present_cold_ms | 0.569 (0.621) | 0.541 |  |
| page/splits/1.25 | present_warm_median_ms | 0.484 (0.496) | 0.473 |  |
| page/splits/1.25 | rects | 99.000 (99.000) | 24.000 | changed |
| page/splits/2 | compose_cold_ms | 2.078 (2.093) | 2.062 |  |
| page/splits/2 | compose_warm_ms | 0.049 (0.050) | 0.045 |  |
| page/splits/2 | cpu_median_ms | 22.262 (22.295) | 13.138 | faster |
| page/splits/2 | gpu_first_ms | 10.874 (14.625) | 12.643 |  |
| page/splits/2 | gpu_warm_median_ms | 8.950 (9.048) | 9.878 | slower |
| page/splits/2 | gpu_warm_p95_ms | 9.196 (9.326) | 10.118 | slower |
| page/splits/2 | mask_kib | 378 (378) | 376 | changed |
| page/splits/2 | mask_resources | 42.000 (42.000) | 36.000 | changed |
| page/splits/2 | masks_drawn | 136 (136) | 38.000 | changed |
| page/splits/2 | other_commands | 0.000 (0.000) | 26.000 | changed |
| page/splits/2 | present_cold_ms | 0.675 (0.696) | 0.638 |  |
| page/splits/2 | present_warm_median_ms | 0.477 (0.492) | 0.470 |  |
| page/splits/2 | rects | 99.000 (99.000) | 24.000 | changed |
| page/state/1 | compose_cold_ms | 1.690 (1.700) | 1.665 |  |
| page/state/1 | compose_warm_ms | 0.053 (0.054) | 0.044 |  |
| page/state/1 | cpu_median_ms | 6.996 (7.039) | 5.055 | faster |
| page/state/1 | gpu_first_ms | 5.782 (5.848) | 6.984 | slower |
| page/state/1 | gpu_warm_median_ms | 4.511 (4.525) | 5.096 | slower |
| page/state/1 | gpu_warm_p95_ms | 4.569 (4.598) | 5.268 | slower |
| page/state/1 | mask_kib | 145 (145) | 145 | changed |
| page/state/1 | mask_resources | 47.000 (47.000) | 42.000 | changed |
| page/state/1 | masks_drawn | 178 (178) | 46.000 | changed |
| page/state/1 | other_commands | 0.000 (0.000) | 35.000 | changed |
| page/state/1 | present_cold_ms | 0.596 (0.630) | 0.565 |  |
| page/state/1 | present_warm_median_ms | 0.499 (0.508) | 0.479 |  |
| page/state/1 | rects | 134 (134) | 26.000 | changed |
| page/state/1.25 | compose_cold_ms | 1.932 (1.986) | 1.916 |  |
| page/state/1.25 | compose_warm_ms | 0.053 (0.054) | 0.045 |  |
| page/state/1.25 | cpu_median_ms | 11.393 (11.401) | 8.293 | faster |
| page/state/1.25 | gpu_first_ms | 8.112 (8.208) | 5.804 | faster |
| page/state/1.25 | gpu_warm_median_ms | 6.241 (6.481) | 5.850 |  |
| page/state/1.25 | gpu_warm_p95_ms | 6.583 (6.611) | 6.986 | slower |
| page/state/1.25 | mask_kib | 225 (225) | 224 | changed |
| page/state/1.25 | mask_resources | 47.000 (47.000) | 42.000 | changed |
| page/state/1.25 | masks_drawn | 178 (178) | 46.000 | changed |
| page/state/1.25 | other_commands | 0.000 (0.000) | 35.000 | changed |
| page/state/1.25 | present_cold_ms | 0.663 (0.731) | 0.611 |  |
| page/state/1.25 | present_warm_median_ms | 0.500 (0.503) | 0.488 |  |
| page/state/1.25 | rects | 134 (134) | 26.000 | changed |
| page/state/2 | compose_cold_ms | 2.812 (2.822) | 2.795 |  |
| page/state/2 | compose_warm_ms | 0.054 (0.054) | 0.045 |  |
| page/state/2 | cpu_median_ms | 26.392 (26.447) | 18.497 | faster |
| page/state/2 | gpu_first_ms | 10.791 (16.349) | 12.825 |  |
| page/state/2 | gpu_warm_median_ms | 9.020 (9.103) | 9.857 | slower |
| page/state/2 | gpu_warm_p95_ms | 9.250 (9.292) | 10.253 | slower |
| page/state/2 | mask_kib | 575 (575) | 574 | changed |
| page/state/2 | mask_resources | 47.000 (47.000) | 42.000 | changed |
| page/state/2 | masks_drawn | 178 (178) | 46.000 | changed |
| page/state/2 | other_commands | 0.000 (0.000) | 35.000 | changed |
| page/state/2 | present_cold_ms | 0.784 (0.886) | 0.723 |  |
| page/state/2 | present_warm_median_ms | 0.501 (0.508) | 0.494 |  |
| page/state/2 | rects | 134 (134) | 26.000 | changed |
| page/surfaces/1 | compose_cold_ms | 0.951 (0.959) | 0.915 |  |
| page/surfaces/1 | compose_warm_ms | 0.051 (0.062) | 0.045 |  |
| page/surfaces/1 | cpu_median_ms | 4.379 (4.405) | 2.889 | faster |
| page/surfaces/1 | gpu_first_ms | 5.316 (5.325) | 5.909 | slower |
| page/surfaces/1 | gpu_warm_median_ms | 4.230 (4.232) | 4.570 | slower |
| page/surfaces/1 | gpu_warm_p95_ms | 4.277 (4.291) | 4.647 | slower |
| page/surfaces/1 | mask_kib | 68.389 (68.389) | 68.076 | changed |
| page/surfaces/1 | mask_resources | 35.000 (35.000) | 30.000 | changed |
| page/surfaces/1 | masks_drawn | 135 (135) | 31.000 | changed |
| page/surfaces/1 | other_commands | 0.000 (0.000) | 27.000 | changed |
| page/surfaces/1 | present_cold_ms | 0.546 (0.648) | 0.502 |  |
| page/surfaces/1 | present_warm_median_ms | 0.467 (0.471) | 0.466 |  |
| page/surfaces/1 | rects | 99.000 (99.000) | 14.000 | changed |
| page/surfaces/1.25 | compose_cold_ms | 1.149 (1.174) | 1.126 |  |
| page/surfaces/1.25 | compose_warm_ms | 0.052 (0.054) | 0.046 |  |
| page/surfaces/1.25 | cpu_median_ms | 7.018 (7.035) | 4.628 | faster |
| page/surfaces/1.25 | gpu_first_ms | 4.574 (7.358) | 5.024 |  |
| page/surfaces/1.25 | gpu_warm_median_ms | 6.105 (6.150) | 6.115 |  |
| page/surfaces/1.25 | gpu_warm_p95_ms | 6.167 (6.181) | 6.752 | slower |
| page/surfaces/1.25 | mask_kib | 120 (120) | 119 | changed |
| page/surfaces/1.25 | mask_resources | 36.000 (36.000) | 31.000 | changed |
| page/surfaces/1.25 | masks_drawn | 136 (136) | 32.000 | changed |
| page/surfaces/1.25 | other_commands | 0.000 (0.000) | 27.000 | changed |
| page/surfaces/1.25 | present_cold_ms | 0.603 (0.610) | 0.565 |  |
| page/surfaces/1.25 | present_warm_median_ms | 0.468 (0.478) | 0.462 |  |
| page/surfaces/1.25 | rects | 99.000 (99.000) | 14.000 | changed |
| page/surfaces/2 | compose_cold_ms | 1.638 (1.648) | 1.588 |  |
| page/surfaces/2 | compose_warm_ms | 0.052 (0.056) | 0.046 |  |
| page/surfaces/2 | cpu_median_ms | 16.772 (16.789) | 10.633 | faster |
| page/surfaces/2 | gpu_first_ms | 10.025 (14.613) | 11.001 |  |
| page/surfaces/2 | gpu_warm_median_ms | 8.457 (8.504) | 9.140 | slower |
| page/surfaces/2 | gpu_warm_p95_ms | 8.663 (8.796) | 9.238 | slower |
| page/surfaces/2 | mask_kib | 306 (306) | 305 | changed |
| page/surfaces/2 | mask_resources | 36.000 (36.000) | 31.000 | changed |
| page/surfaces/2 | masks_drawn | 136 (136) | 32.000 | changed |
| page/surfaces/2 | other_commands | 0.000 (0.000) | 27.000 | changed |
| page/surfaces/2 | present_cold_ms | 0.728 (0.771) | 0.670 |  |
| page/surfaces/2 | present_warm_median_ms | 0.472 (0.480) | 0.464 |  |
| page/surfaces/2 | rects | 99.000 (99.000) | 14.000 | changed |
| page/text-rendering/1 | compose_cold_ms | 1.410 (1.420) | 1.386 |  |
| page/text-rendering/1 | compose_warm_ms | 0.048 (0.054) | 0.043 |  |
| page/text-rendering/1 | cpu_median_ms | 6.145 (6.157) | 4.360 | faster |
| page/text-rendering/1 | gpu_first_ms | 5.471 (5.580) | 6.382 | slower |
| page/text-rendering/1 | gpu_warm_median_ms | 4.350 (4.380) | 4.862 | slower |
| page/text-rendering/1 | gpu_warm_p95_ms | 4.417 (4.461) | 4.923 | slower |
| page/text-rendering/1 | mask_kib | 134 (134) | 134 | changed |
| page/text-rendering/1 | mask_resources | 40.000 (40.000) | 35.000 | changed |
| page/text-rendering/1 | masks_drawn | 115 (115) | 35.000 | changed |
| page/text-rendering/1 | other_commands | 0.000 (0.000) | 19.000 | changed |
| page/text-rendering/1 | present_cold_ms | 0.575 (0.643) | 0.546 |  |
| page/text-rendering/1 | present_warm_median_ms | 0.462 (0.475) | 0.466 |  |
| page/text-rendering/1 | rects | 77.000 (77.000) | 14.000 | changed |
| page/text-rendering/1.25 | compose_cold_ms | 1.594 (1.626) | 1.594 |  |
| page/text-rendering/1.25 | compose_warm_ms | 0.049 (0.052) | 0.044 |  |
| page/text-rendering/1.25 | cpu_median_ms | 10.031 (10.045) | 7.176 | faster |
| page/text-rendering/1.25 | gpu_first_ms | 7.736 (7.813) | 5.427 | faster |
| page/text-rendering/1.25 | gpu_warm_median_ms | 6.023 (6.330) | 6.001 |  |
| page/text-rendering/1.25 | gpu_warm_p95_ms | 6.319 (6.399) | 6.664 | within noise |
| page/text-rendering/1.25 | mask_kib | 208 (208) | 207 | changed |
| page/text-rendering/1.25 | mask_resources | 40.000 (40.000) | 35.000 | changed |
| page/text-rendering/1.25 | masks_drawn | 115 (115) | 35.000 | changed |
| page/text-rendering/1.25 | other_commands | 0.000 (0.000) | 19.000 | changed |
| page/text-rendering/1.25 | present_cold_ms | 0.621 (0.693) | 0.611 |  |
| page/text-rendering/1.25 | present_warm_median_ms | 0.460 (0.470) | 0.458 |  |
| page/text-rendering/1.25 | rects | 77.000 (77.000) | 14.000 | changed |
| page/text-rendering/2 | compose_cold_ms | 2.381 (2.437) | 2.354 |  |
| page/text-rendering/2 | compose_warm_ms | 0.048 (0.053) | 0.043 |  |
| page/text-rendering/2 | cpu_median_ms | 23.359 (23.452) | 16.187 | faster |
| page/text-rendering/2 | gpu_first_ms | 10.111 (15.417) | 11.917 |  |
| page/text-rendering/2 | gpu_warm_median_ms | 8.721 (8.730) | 9.442 | slower |
| page/text-rendering/2 | gpu_warm_p95_ms | 9.005 (9.031) | 9.750 | slower |
| page/text-rendering/2 | mask_kib | 533 (533) | 531 | changed |
| page/text-rendering/2 | mask_resources | 40.000 (40.000) | 35.000 | changed |
| page/text-rendering/2 | masks_drawn | 115 (115) | 35.000 | changed |
| page/text-rendering/2 | other_commands | 0.000 (0.000) | 19.000 | changed |
| page/text-rendering/2 | present_cold_ms | 0.755 (0.766) | 0.725 |  |
| page/text-rendering/2 | present_warm_median_ms | 0.460 (0.471) | 0.467 |  |
| page/text-rendering/2 | rects | 77.000 (77.000) | 14.000 | changed |
| page/text/1 | compose_cold_ms | 1.785 (1.811) | 1.756 |  |
| page/text/1 | compose_warm_ms | 0.054 (0.055) | 0.047 |  |
| page/text/1 | cpu_median_ms | 6.514 (6.519) | 4.825 | faster |
| page/text/1 | gpu_first_ms | 5.558 (5.581) | 6.432 | slower |
| page/text/1 | gpu_warm_median_ms | 4.338 (4.423) | 4.933 | slower |
| page/text/1 | gpu_warm_p95_ms | 4.471 (4.498) | 5.028 | slower |
| page/text/1 | mask_kib | 134 (134) | 133 | changed |
| page/text/1 | mask_resources | 62.000 (62.000) | 57.000 | changed |
| page/text/1 | masks_drawn | 180 (180) | 58.000 | changed |
| page/text/1 | other_commands | 0.000 (0.000) | 31.000 | changed |
| page/text/1 | present_cold_ms | 0.959 (0.983) | 0.920 |  |
| page/text/1 | present_warm_median_ms | 0.509 (0.514) | 0.485 |  |
| page/text/1 | rects | 116 (116) | 17.000 | changed |
| page/text/1.25 | compose_cold_ms | 2.017 (2.049) | 1.993 |  |
| page/text/1.25 | compose_warm_ms | 0.055 (0.059) | 0.047 |  |
| page/text/1.25 | cpu_median_ms | 10.495 (10.512) | 7.801 | faster |
| page/text/1.25 | gpu_first_ms | 7.800 (7.942) | 5.417 |  |
| page/text/1.25 | gpu_warm_median_ms | 6.407 (6.416) | 5.954 |  |
| page/text/1.25 | gpu_warm_p95_ms | 6.435 (6.479) | 6.265 |  |
| page/text/1.25 | mask_kib | 210 (210) | 209 | changed |
| page/text/1.25 | mask_resources | 62.000 (62.000) | 57.000 | changed |
| page/text/1.25 | masks_drawn | 180 (180) | 58.000 | changed |
| page/text/1.25 | other_commands | 0.000 (0.000) | 31.000 | changed |
| page/text/1.25 | present_cold_ms | 1.041 (1.070) | 1.012 |  |
| page/text/1.25 | present_warm_median_ms | 0.505 (0.511) | 0.496 |  |
| page/text/1.25 | rects | 116 (116) | 17.000 | changed |
| page/text/2 | compose_cold_ms | 2.881 (2.895) | 2.868 |  |
| page/text/2 | compose_warm_ms | 0.054 (0.055) | 0.047 |  |
| page/text/2 | cpu_median_ms | 24.502 (24.535) | 17.604 | faster |
| page/text/2 | gpu_first_ms | 10.266 (14.288) | 12.092 |  |
| page/text/2 | gpu_warm_median_ms | 8.820 (8.827) | 9.511 | slower |
| page/text/2 | gpu_warm_p95_ms | 9.086 (9.100) | 9.831 | slower |
| page/text/2 | mask_kib | 532 (532) | 530 | changed |
| page/text/2 | mask_resources | 62.000 (62.000) | 57.000 | changed |
| page/text/2 | masks_drawn | 180 (180) | 58.000 | changed |
| page/text/2 | other_commands | 0.000 (0.000) | 31.000 | changed |
| page/text/2 | present_cold_ms | 1.157 (1.164) | 1.112 |  |
| page/text/2 | present_warm_median_ms | 0.508 (0.522) | 0.494 |  |
| page/text/2 | rects | 116 (116) | 17.000 | changed |
| page/theme/1 | compose_cold_ms | 1.955 (1.964) | 1.937 |  |
| page/theme/1 | compose_warm_ms | 0.052 (0.053) | 0.047 |  |
| page/theme/1 | cpu_median_ms | 7.473 (7.503) | 5.837 | faster |
| page/theme/1 | gpu_first_ms | 5.407 (5.550) | 6.370 | slower |
| page/theme/1 | gpu_warm_median_ms | 4.362 (4.391) | 4.814 | slower |
| page/theme/1 | gpu_warm_p95_ms | 4.429 (4.470) | 4.890 | slower |
| page/theme/1 | mask_kib | 186 (186) | 186 | changed |
| page/theme/1 | mask_resources | 51.000 (51.000) | 45.000 | changed |
| page/theme/1 | masks_drawn | 137 (137) | 45.000 | changed |
| page/theme/1 | other_commands | 0.000 (0.000) | 23.000 | changed |
| page/theme/1 | present_cold_ms | 0.666 (0.675) | 0.635 |  |
| page/theme/1 | present_warm_median_ms | 0.508 (0.512) | 0.487 |  |
| page/theme/1 | rects | 101 (101) | 31.000 | changed |
| page/theme/1.25 | compose_cold_ms | 2.311 (2.332) | 2.280 |  |
| page/theme/1.25 | compose_warm_ms | 0.054 (0.055) | 0.048 |  |
| page/theme/1.25 | cpu_median_ms | 12.304 (12.350) | 9.622 | faster |
| page/theme/1.25 | gpu_first_ms | 7.736 (7.792) | 8.302 | slower |
| page/theme/1.25 | gpu_warm_median_ms | 6.340 (6.412) | 6.390 |  |
| page/theme/1.25 | gpu_warm_p95_ms | 6.409 (6.483) | 6.995 | slower |
| page/theme/1.25 | mask_kib | 288 (288) | 287 | changed |
| page/theme/1.25 | mask_resources | 51.000 (51.000) | 45.000 | changed |
| page/theme/1.25 | masks_drawn | 147 (147) | 45.000 | changed |
| page/theme/1.25 | other_commands | 0.000 (0.000) | 28.000 | changed |
| page/theme/1.25 | present_cold_ms | 0.747 (0.776) | 0.690 |  |
| page/theme/1.25 | present_warm_median_ms | 0.503 (0.527) | 0.490 |  |
| page/theme/1.25 | rects | 106 (106) | 31.000 | changed |
| page/theme/2 | compose_cold_ms | 3.363 (3.381) | 3.355 |  |
| page/theme/2 | compose_warm_ms | 0.054 (0.054) | 0.048 |  |
| page/theme/2 | cpu_median_ms | 28.328 (28.548) | 21.603 | faster |
| page/theme/2 | gpu_first_ms | 10.475 (10.771) | 11.587 | slower |
| page/theme/2 | gpu_warm_median_ms | 8.836 (8.886) | 9.413 | slower |
| page/theme/2 | gpu_warm_p95_ms | 9.092 (9.112) | 9.613 | slower |
| page/theme/2 | mask_kib | 739 (739) | 737 | changed |
| page/theme/2 | mask_resources | 51.000 (51.000) | 45.000 | changed |
| page/theme/2 | masks_drawn | 147 (147) | 45.000 | changed |
| page/theme/2 | other_commands | 0.000 (0.000) | 28.000 | changed |
| page/theme/2 | present_cold_ms | 0.875 (0.895) | 0.856 |  |
| page/theme/2 | present_warm_median_ms | 0.507 (0.534) | 0.492 |  |
| page/theme/2 | rects | 106 (106) | 31.000 | changed |
| page/tool-windows/1 | compose_cold_ms | 1.911 (1.926) | 1.908 |  |
| page/tool-windows/1 | compose_warm_ms | 0.052 (0.052) | 0.046 |  |
| page/tool-windows/1 | cpu_median_ms | 7.880 (7.901) | 5.586 | faster |
| page/tool-windows/1 | gpu_first_ms | 5.930 (5.984) | 7.224 | slower |
| page/tool-windows/1 | gpu_warm_median_ms | 4.568 (4.614) | 5.191 | slower |
| page/tool-windows/1 | gpu_warm_p95_ms | 4.724 (4.762) | 5.404 | slower |
| page/tool-windows/1 | mask_kib | 173 (173) | 173 | changed |
| page/tool-windows/1 | mask_resources | 53.000 (53.000) | 47.000 | changed |
| page/tool-windows/1 | masks_drawn | 150 (150) | 50.000 | changed |
| page/tool-windows/1 | other_commands | 0.000 (0.000) | 26.000 | changed |
| page/tool-windows/1 | present_cold_ms | 0.597 (0.625) | 0.599 |  |
| page/tool-windows/1 | present_warm_median_ms | 0.491 (0.500) | 0.484 |  |
| page/tool-windows/1 | rects | 101 (101) | 23.000 | changed |
| page/tool-windows/1.25 | compose_cold_ms | 2.192 (2.231) | 2.186 |  |
| page/tool-windows/1.25 | compose_warm_ms | 0.053 (0.055) | 0.048 |  |
| page/tool-windows/1.25 | cpu_median_ms | 12.734 (12.763) | 9.114 | faster |
| page/tool-windows/1.25 | gpu_first_ms | 8.298 (8.489) | 5.866 |  |
| page/tool-windows/1.25 | gpu_warm_median_ms | 6.086 (6.491) | 6.000 |  |
| page/tool-windows/1.25 | gpu_warm_p95_ms | 6.694 (6.833) | 6.793 |  |
| page/tool-windows/1.25 | mask_kib | 272 (272) | 272 | changed |
| page/tool-windows/1.25 | mask_resources | 53.000 (53.000) | 47.000 | changed |
| page/tool-windows/1.25 | masks_drawn | 150 (150) | 50.000 | changed |
| page/tool-windows/1.25 | other_commands | 0.000 (0.000) | 26.000 | changed |
| page/tool-windows/1.25 | present_cold_ms | 0.668 (0.707) | 0.656 |  |
| page/tool-windows/1.25 | present_warm_median_ms | 0.492 (0.502) | 0.490 |  |
| page/tool-windows/1.25 | rects | 101 (101) | 23.000 | changed |
| page/tool-windows/2 | compose_cold_ms | 3.185 (3.263) | 3.170 |  |
| page/tool-windows/2 | compose_warm_ms | 0.053 (0.055) | 0.048 |  |
| page/tool-windows/2 | cpu_median_ms | 29.828 (29.887) | 20.404 | faster |
| page/tool-windows/2 | gpu_first_ms | 11.017 (11.195) | 13.412 | slower |
| page/tool-windows/2 | gpu_warm_median_ms | 9.134 (9.205) | 10.031 | slower |
| page/tool-windows/2 | gpu_warm_p95_ms | 9.373 (9.419) | 10.438 | slower |
| page/tool-windows/2 | mask_kib | 687 (687) | 686 | changed |
| page/tool-windows/2 | mask_resources | 53.000 (53.000) | 47.000 | changed |
| page/tool-windows/2 | masks_drawn | 150 (150) | 50.000 | changed |
| page/tool-windows/2 | other_commands | 0.000 (0.000) | 26.000 | changed |
| page/tool-windows/2 | present_cold_ms | 0.792 (0.812) | 0.772 |  |
| page/tool-windows/2 | present_warm_median_ms | 0.499 (0.514) | 0.493 |  |
| page/tool-windows/2 | rects | 101 (101) | 23.000 | changed |
| page/toolbar/1 | compose_cold_ms | 2.510 (2.526) | 2.452 |  |
| page/toolbar/1 | compose_warm_ms | 0.068 (0.071) | 0.053 |  |
| page/toolbar/1 | cpu_median_ms | 8.031 (8.054) | 6.481 | faster |
| page/toolbar/1 | gpu_first_ms | 5.693 (5.773) | 6.725 | slower |
| page/toolbar/1 | gpu_warm_median_ms | 4.551 (4.575) | 4.986 | slower |
| page/toolbar/1 | gpu_warm_p95_ms | 4.599 (4.618) | 5.180 | slower |
| page/toolbar/1 | mask_kib | 194 (194) | 194 | changed |
| page/toolbar/1 | mask_resources | 89.000 (89.000) | 83.000 | changed |
| page/toolbar/1 | masks_drawn | 331 (331) | 89.000 | changed |
| page/toolbar/1 | other_commands | 0.000 (0.000) | 56.000 | changed |
| page/toolbar/1 | present_cold_ms | 0.913 (0.969) | 0.806 | faster |
| page/toolbar/1 | present_warm_median_ms | 0.589 (0.607) | 0.540 |  |
| page/toolbar/1 | rects | 203 (203) | 17.000 | changed |
| page/toolbar/1.25 | compose_cold_ms | 2.835 (2.853) | 2.787 |  |
| page/toolbar/1.25 | compose_warm_ms | 0.069 (0.072) | 0.056 |  |
| page/toolbar/1.25 | cpu_median_ms | 12.885 (12.902) | 10.330 | faster |
| page/toolbar/1.25 | gpu_first_ms | 7.878 (7.953) | 5.550 | faster |
| page/toolbar/1.25 | gpu_warm_median_ms | 6.021 (6.279) | 6.078 |  |
| page/toolbar/1.25 | gpu_warm_p95_ms | 6.458 (6.614) | 6.835 | within noise |
| page/toolbar/1.25 | mask_kib | 306 (306) | 306 | changed |
| page/toolbar/1.25 | mask_resources | 89.000 (89.000) | 83.000 | changed |
| page/toolbar/1.25 | masks_drawn | 331 (331) | 89.000 | changed |
| page/toolbar/1.25 | other_commands | 0.000 (0.000) | 56.000 | changed |
| page/toolbar/1.25 | present_cold_ms | 0.964 (0.978) | 0.867 | faster |
| page/toolbar/1.25 | present_warm_median_ms | 0.592 (0.598) | 0.549 |  |
| page/toolbar/1.25 | rects | 203 (203) | 17.000 | changed |
| page/toolbar/2 | compose_cold_ms | 4.010 (4.202) | 3.968 |  |
| page/toolbar/2 | compose_warm_ms | 0.068 (0.073) | 0.054 |  |
| page/toolbar/2 | cpu_median_ms | 30.040 (30.075) | 23.670 | faster |
| page/toolbar/2 | gpu_first_ms | 10.336 (10.698) | 12.304 | slower |
| page/toolbar/2 | gpu_warm_median_ms | 8.991 (9.017) | 9.589 | slower |
| page/toolbar/2 | gpu_warm_p95_ms | 9.261 (9.437) | 9.961 | slower |
| page/toolbar/2 | mask_kib | 774 (774) | 773 | changed |
| page/toolbar/2 | mask_resources | 89.000 (89.000) | 83.000 | changed |
| page/toolbar/2 | masks_drawn | 331 (331) | 89.000 | changed |
| page/toolbar/2 | other_commands | 0.000 (0.000) | 56.000 | changed |
| page/toolbar/2 | present_cold_ms | 1.105 (1.113) | 0.992 | faster |
| page/toolbar/2 | present_warm_median_ms | 0.592 (0.597) | 0.561 |  |
| page/toolbar/2 | rects | 203 (203) | 17.000 | changed |
| page/transforms/1 | compose_cold_ms | 1.946 (1.960) | 1.912 |  |
| page/transforms/1 | compose_warm_ms | 0.051 (0.052) | 0.045 |  |
| page/transforms/1 | cpu_median_ms | 8.589 (8.614) | 6.284 | faster |
| page/transforms/1 | gpu_first_ms | 6.024 (6.148) | 7.208 | slower |
| page/transforms/1 | gpu_warm_median_ms | 4.601 (4.696) | 5.241 | slower |
| page/transforms/1 | gpu_warm_p95_ms | 4.723 (4.781) | 5.326 | slower |
| page/transforms/1 | mask_kib | 188 (188) | 188 | changed |
| page/transforms/1 | mask_resources | 44.000 (44.000) | 39.000 | changed |
| page/transforms/1 | masks_drawn | 171 (171) | 43.000 | changed |
| page/transforms/1 | other_commands | 0.000 (0.000) | 31.000 | changed |
| page/transforms/1 | present_cold_ms | 0.604 (0.629) | 0.563 |  |
| page/transforms/1 | present_warm_median_ms | 0.496 (0.502) | 0.473 |  |
| page/transforms/1 | rects | 144 (144) | 41.000 | changed |
| page/transforms/1.25 | compose_cold_ms | 2.214 (2.298) | 2.192 |  |
| page/transforms/1.25 | compose_warm_ms | 0.052 (0.058) | 0.045 |  |
| page/transforms/1.25 | cpu_median_ms | 13.932 (13.942) | 10.228 | faster |
| page/transforms/1.25 | gpu_first_ms | 8.509 (8.660) | 6.183 |  |
| page/transforms/1.25 | gpu_warm_median_ms | 6.479 (6.794) | 6.053 |  |
| page/transforms/1.25 | gpu_warm_p95_ms | 6.871 (7.009) | 7.068 | within noise |
| page/transforms/1.25 | mask_kib | 296 (296) | 295 | changed |
| page/transforms/1.25 | mask_resources | 44.000 (44.000) | 39.000 | changed |
| page/transforms/1.25 | masks_drawn | 171 (171) | 43.000 | changed |
| page/transforms/1.25 | other_commands | 0.000 (0.000) | 31.000 | changed |
| page/transforms/1.25 | present_cold_ms | 0.652 (0.769) | 0.619 |  |
| page/transforms/1.25 | present_warm_median_ms | 0.494 (0.495) | 0.478 |  |
| page/transforms/1.25 | rects | 144 (144) | 41.000 | changed |
| page/transforms/2 | compose_cold_ms | 3.230 (3.247) | 3.204 |  |
| page/transforms/2 | compose_warm_ms | 0.052 (0.053) | 0.046 |  |
| page/transforms/2 | cpu_median_ms | 32.397 (32.432) | 22.867 | faster |
| page/transforms/2 | gpu_first_ms | 11.428 (11.558) | 13.721 | slower |
| page/transforms/2 | gpu_warm_median_ms | 9.182 (9.373) | 10.166 | slower |
| page/transforms/2 | gpu_warm_p95_ms | 9.436 (9.520) | 10.747 | slower |
| page/transforms/2 | mask_kib | 752 (752) | 750 | changed |
| page/transforms/2 | mask_resources | 44.000 (44.000) | 39.000 | changed |
| page/transforms/2 | masks_drawn | 171 (171) | 43.000 | changed |
| page/transforms/2 | other_commands | 0.000 (0.000) | 31.000 | changed |
| page/transforms/2 | present_cold_ms | 0.802 (0.819) | 0.768 |  |
| page/transforms/2 | present_warm_median_ms | 0.497 (0.507) | 0.483 |  |
| page/transforms/2 | rects | 144 (144) | 41.000 | changed |
| page/virtual/1 | compose_cold_ms | 1.255 (1.277) | 1.216 |  |
| page/virtual/1 | compose_warm_ms | 0.052 (0.054) | 0.045 |  |
| page/virtual/1 | cpu_median_ms | 5.574 (5.589) | 3.755 | faster |
| page/virtual/1 | gpu_first_ms | 5.515 (5.541) | 6.463 | slower |
| page/virtual/1 | gpu_warm_median_ms | 4.348 (4.379) | 4.664 | slower |
| page/virtual/1 | gpu_warm_p95_ms | 4.397 (4.427) | 4.953 | slower |
| page/virtual/1 | mask_kib | 101 (101) | 101 | changed |
| page/virtual/1 | mask_resources | 43.000 (43.000) | 38.000 | changed |
| page/virtual/1 | masks_drawn | 163 (163) | 43.000 | changed |
| page/virtual/1 | other_commands | 0.000 (0.000) | 30.000 | changed |
| page/virtual/1 | present_cold_ms | 0.606 (0.645) | 0.556 |  |
| page/virtual/1 | present_warm_median_ms | 0.486 (0.496) | 0.472 |  |
| page/virtual/1 | rects | 106 (106) | 14.000 | changed |
| page/virtual/1.25 | compose_cold_ms | 1.407 (1.420) | 1.381 |  |
| page/virtual/1.25 | compose_warm_ms | 0.052 (0.054) | 0.045 |  |
| page/virtual/1.25 | cpu_median_ms | 8.952 (8.962) | 6.092 | faster |
| page/virtual/1.25 | gpu_first_ms | 7.662 (7.725) | 9.181 | slower |
| page/virtual/1.25 | gpu_warm_median_ms | 6.043 (6.256) | 6.577 | slower |
| page/virtual/1.25 | gpu_warm_p95_ms | 6.335 (6.382) | 7.155 | slower |
| page/virtual/1.25 | mask_kib | 156 (156) | 156 | changed |
| page/virtual/1.25 | mask_resources | 43.000 (43.000) | 38.000 | changed |
| page/virtual/1.25 | masks_drawn | 163 (163) | 43.000 | changed |
| page/virtual/1.25 | other_commands | 0.000 (0.000) | 30.000 | changed |
| page/virtual/1.25 | present_cold_ms | 0.653 (0.705) | 0.595 |  |
| page/virtual/1.25 | present_warm_median_ms | 0.494 (0.500) | 0.482 |  |
| page/virtual/1.25 | rects | 106 (106) | 14.000 | changed |
| page/virtual/2 | compose_cold_ms | 2.022 (2.101) | 2.004 |  |
| page/virtual/2 | compose_warm_ms | 0.053 (0.053) | 0.045 |  |
| page/virtual/2 | cpu_median_ms | 21.078 (21.128) | 13.733 | faster |
| page/virtual/2 | gpu_first_ms | 10.494 (15.274) | 12.100 |  |
| page/virtual/2 | gpu_warm_median_ms | 8.849 (8.872) | 9.599 | slower |
| page/virtual/2 | gpu_warm_p95_ms | 9.090 (9.156) | 9.932 | slower |
| page/virtual/2 | mask_kib | 401 (401) | 399 | changed |
| page/virtual/2 | mask_resources | 43.000 (43.000) | 38.000 | changed |
| page/virtual/2 | masks_drawn | 163 (163) | 43.000 | changed |
| page/virtual/2 | other_commands | 0.000 (0.000) | 30.000 | changed |
| page/virtual/2 | present_cold_ms | 0.770 (0.780) | 0.706 | faster |
| page/virtual/2 | present_warm_median_ms | 0.491 (0.502) | 0.481 |  |
| page/virtual/2 | rects | 106 (106) | 14.000 | changed |
| page/windows/1 | compose_cold_ms | 1.158 (1.180) | 1.115 |  |
| page/windows/1 | compose_warm_ms | 0.059 (0.064) | 0.050 |  |
| page/windows/1 | cpu_median_ms | 5.433 (5.457) | 3.122 | faster |
| page/windows/1 | gpu_first_ms | 5.917 (6.024) | 6.421 | slower |
| page/windows/1 | gpu_warm_median_ms | 4.603 (4.633) | 4.835 | within noise |
| page/windows/1 | gpu_warm_p95_ms | 4.678 (4.712) | 4.982 | slower |
| page/windows/1 | mask_kib | 69.687 (69.687) | 68.983 | changed |
| page/windows/1 | mask_resources | 52.000 (52.000) | 45.000 | changed |
| page/windows/1 | masks_drawn | 225 (225) | 53.000 | changed |
| page/windows/1 | other_commands | 0.000 (0.000) | 39.000 | changed |
| page/windows/1 | present_cold_ms | 0.697 (0.703) | 0.608 | faster |
| page/windows/1 | present_warm_median_ms | 0.522 (0.524) | 0.500 |  |
| page/windows/1 | rects | 153 (153) | 21.000 | changed |
| page/windows/1.25 | compose_cold_ms | 1.297 (1.308) | 1.242 |  |
| page/windows/1.25 | compose_warm_ms | 0.060 (0.061) | 0.050 |  |
| page/windows/1.25 | cpu_median_ms | 8.599 (8.621) | 4.944 | faster |
| page/windows/1.25 | gpu_first_ms | 8.284 (8.408) | 5.465 | faster |
| page/windows/1.25 | gpu_warm_median_ms | 6.158 (6.530) | 6.253 |  |
| page/windows/1.25 | gpu_warm_p95_ms | 6.567 (6.769) | 6.613 |  |
| page/windows/1.25 | mask_kib | 110 (110) | 109 | changed |
| page/windows/1.25 | mask_resources | 52.000 (52.000) | 45.000 | changed |
| page/windows/1.25 | masks_drawn | 225 (225) | 53.000 | changed |
| page/windows/1.25 | other_commands | 0.000 (0.000) | 39.000 | changed |
| page/windows/1.25 | present_cold_ms | 0.747 (0.759) | 0.652 | faster |
| page/windows/1.25 | present_warm_median_ms | 0.512 (0.530) | 0.505 |  |
| page/windows/1.25 | rects | 153 (153) | 21.000 | changed |
| page/windows/2 | compose_cold_ms | 1.783 (1.794) | 1.722 |  |
| page/windows/2 | compose_warm_ms | 0.060 (0.062) | 0.051 |  |
| page/windows/2 | cpu_median_ms | 20.816 (20.856) | 11.374 | faster |
| page/windows/2 | gpu_first_ms | 11.077 (11.280) | 11.997 | slower |
| page/windows/2 | gpu_warm_median_ms | 9.043 (9.101) | 9.568 | slower |
| page/windows/2 | gpu_warm_p95_ms | 9.327 (9.520) | 9.830 | within noise |
| page/windows/2 | mask_kib | 278 (278) | 275 | changed |
| page/windows/2 | mask_resources | 52.000 (52.000) | 45.000 | changed |
| page/windows/2 | masks_drawn | 225 (225) | 53.000 | changed |
| page/windows/2 | other_commands | 0.000 (0.000) | 39.000 | changed |
| page/windows/2 | present_cold_ms | 0.858 (0.868) | 0.763 | faster |
| page/windows/2 | present_warm_median_ms | 0.524 (0.529) | 0.505 |  |
| page/windows/2 | rects | 153 (153) | 21.000 | changed |
| page/workspace/1 | compose_cold_ms | 1.132 (1.142) | 1.108 |  |
| page/workspace/1 | compose_warm_ms | 0.048 (0.049) | 0.044 |  |
| page/workspace/1 | cpu_median_ms | 4.245 (4.273) | 2.906 | faster |
| page/workspace/1 | gpu_first_ms | 4.943 (5.052) | 5.199 | within noise |
| page/workspace/1 | gpu_warm_median_ms | 4.057 (4.083) | 4.096 | within noise |
| page/workspace/1 | gpu_warm_p95_ms | 4.159 (4.211) | 4.213 | within noise |
| page/workspace/1 | mask_kib | 78.975 (78.975) | 78.653 | changed |
| page/workspace/1 | mask_resources | 43.000 (43.000) | 37.000 | changed |
| page/workspace/1 | masks_drawn | 121 (121) | 37.000 | changed |
| page/workspace/1 | other_commands | 0.000 (0.000) | 20.000 | changed |
| page/workspace/1 | present_cold_ms | 0.564 (0.614) | 0.532 |  |
| page/workspace/1 | present_warm_median_ms | 0.474 (0.480) | 0.461 |  |
| page/workspace/1 | rects | 82.000 (82.000) | 18.000 | changed |
| page/workspace/1.25 | compose_cold_ms | 1.295 (1.315) | 1.270 |  |
| page/workspace/1.25 | compose_warm_ms | 0.049 (0.052) | 0.044 |  |
| page/workspace/1.25 | cpu_median_ms | 6.754 (6.780) | 4.593 | faster |
| page/workspace/1.25 | gpu_first_ms | 7.068 (7.089) | 7.126 | within noise |
| page/workspace/1.25 | gpu_warm_median_ms | 5.944 (5.948) | 6.056 | within noise |
| page/workspace/1.25 | gpu_warm_p95_ms | 6.007 (6.033) | 6.109 | within noise |
| page/workspace/1.25 | mask_kib | 123 (123) | 122 | changed |
| page/workspace/1.25 | mask_resources | 43.000 (43.000) | 37.000 | changed |
| page/workspace/1.25 | masks_drawn | 121 (121) | 37.000 | changed |
| page/workspace/1.25 | other_commands | 0.000 (0.000) | 20.000 | changed |
| page/workspace/1.25 | present_cold_ms | 0.608 (0.618) | 0.579 |  |
| page/workspace/1.25 | present_warm_median_ms | 0.476 (0.489) | 0.469 |  |
| page/workspace/1.25 | rects | 82.000 (82.000) | 18.000 | changed |
| page/workspace/2 | compose_cold_ms | 1.784 (1.809) | 1.759 |  |
| page/workspace/2 | compose_warm_ms | 0.049 (0.049) | 0.044 |  |
| page/workspace/2 | cpu_median_ms | 16.099 (16.125) | 10.609 | faster |
| page/workspace/2 | gpu_first_ms | 9.633 (9.682) | 9.849 | within noise |
| page/workspace/2 | gpu_warm_median_ms | 8.329 (8.336) | 8.422 | within noise |
| page/workspace/2 | gpu_warm_p95_ms | 8.525 (8.580) | 8.598 | within noise |
| page/workspace/2 | mask_kib | 314 (314) | 313 | changed |
| page/workspace/2 | mask_resources | 43.000 (43.000) | 37.000 | changed |
| page/workspace/2 | masks_drawn | 121 (121) | 37.000 | changed |
| page/workspace/2 | other_commands | 0.000 (0.000) | 20.000 | changed |
| page/workspace/2 | present_cold_ms | 0.738 (0.765) | 0.702 |  |
| page/workspace/2 | present_warm_median_ms | 0.471 (0.478) | 0.462 |  |
| page/workspace/2 | rects | 82.000 (82.000) | 18.000 | changed |
