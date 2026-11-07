# Presentation without read-back (2026-11-05)

Follow-up 1 to [results.md](results.md): pert-ui wgpu and vello_gpu
presenting each workload to a window surface, with no read-back. Measured on
the machine described in results.md (i9-12900 pinned to CPUs 2–7, Intel UHD
770, Mesa 26.2.6 Vulkan, kernel 7.2.15-200.fc44, GNOME Wayland, rustc
1.99.1); nothing here is a claim about another machine.

Reproduce: `plans/rendering/vello-spike/present.sh 3` (each run waits for a
1-minute load below 2.0 through `plans/rendering/tools/quiet-run.sh`, which
records load and the busiest processes at start and end), then
`python3 -I plans/rendering/vello-spike/summarize_present.py
plans/rendering/vello-spike/results/raw-present-*.md`. Raw:
`raw-present-{1,2,3}.md`. Code: `src/present.rs` (`vello-spike present`).

## Conditions

- Three runs, 18:01–18:07, back to back, none waited for quiet: 1-minute
  load 0.72 → 0.81, 0.81 → 0.37, 0.37 → 0.42. The processes with the most
  CPU at every start and end were gnome-terminal-server, gnome-shell, codex
  and claude, each below 6% of one CPU by `ps`, which reports a lifetime
  average. Nothing sampled the machine during a run; the low load at both
  ends and the agreement of the three runs are the evidence that it stayed
  quiet.
- Monitor: HDMI-3, 3440×1440 at 99.98 Hz, scale factor 1.
- Window sizes: 1280×800 requested and obtained for scale 1. 2560×1600 was
  requested for scale 2, but the compositor gave **2560×1373**, the
  monitor's height minus the panel and title bar. The scale-2 scenes
  therefore use a logical viewport of 1280×686.5. Content below that is
  cropped, the same for both renderers: the lowest panel rows and the bottom
  13.5 logical px of the meter.
- pert-ui wgpu: `Renderer::new(window, extent)` with its own configuration
  (an sRGB surface format, Fifo, desired maximum frame latency 2, opaque
  alpha).
- vello_gpu: its own device and surface on the same window type, with Fifo,
  desired maximum frame latency 2 and opaque alpha like pert-ui. The format
  is **`Bgra8Unorm`, not sRGB**: vello_gpu's shaders write sRGB-encoded
  values, so the surface must store them unchanged (an `*Srgb` format would
  encode them a second time). The surface offered Bgra8UnormSrgb,
  Rgba8UnormSrgb, Rgba16Float, Rgba16Unorm, Rgb10a2Unorm, Bgra8Unorm and
  Rgba8Unorm.
- Pacing: one frame per 30 ms (sleep to the next 30 ms boundary) at ~100 Hz
  Fifo, so a free swapchain image is always ready. vello_gpu's acquire time
  (≈ 0.04 ms) confirms that acquisition never blocked. pert-ui returned
  `Frame::Presented` for every frame, and vello_gpu acquired every frame.
- 150 measured frames after 20 warm-up frames per renderer, workload, scale
  and run. Every workload changes every frame except the panels, which are
  recomposed every frame (pert-ui) or re-encoded every frame (vello).
- **UI frame** = from the start of the frame to the return from present.
  - pert-ui: the runtime update (signal write, `flush` and generating the
    workload's data, about 0.03 ms: the UI frame minus the two phases below),
    compose (`Runtime::paint`) and the whole `Renderer::render` call, which acquires, prepares (including CPU vector coverage), encodes,
    submits, calls `before_present` and presents.
  - vello_gpu: generating the workload's data is **not** included; then
    encode (scene building, which processes paths into strips) plus acquire, prepare (`Renderer::render` recording and uploads), submit
    and present.
- **GPU completion** (vello_gpu only): from just before `queue.submit` to
  the return of a blocking `device.poll` for that submission. It is taken
  after present, outside the UI frame. It includes present (~0.04 ms) and the
  poll's wake-up latency.
- **pert-ui's GPU completion cannot be measured through the public API**
  (`render` returns after present, and there is no fence or submission index
  to wait on). The in-crate real-time probe's offscreen GPU pass (encode +
  submit + wait, 960×640, `plans/rendering/baseline-probe/results/realtime-cpu2.txt`)
  is the nearest measurement. Its medians (p95) are:
  - 0.49 ms (3.90) for min/max columns;
  - 0.44 ms (0.90) for the 800-point polyline;
  - 0.45 ms (0.77) for the 4096-point polyline;
  - 0.63 ms (0.67) for the waterfall image.

  These are different scenes at a smaller size, so they are not comparable
  cell by cell.

## UI-thread time per presented frame

Times in ms: the median over the three runs of each run's median, with the
range of the three; p95 is the median of the three runs' p95; phases are
medians.

| Workload | Scale | Renderer | UI frame median (range over runs) | p95 | Phases (median) | GPU completion median / p95 | Frames not presented (all runs) |
|---|---|---|---|---|---|---|---|
| Panels (150 rounded + 100 plain) | 1 | pert-ui wgpu | 0.987 (0.986–1.009) | 1.197 | compose: 0.162<br>`render` call: 0.793<br>of which until `before_present`: 0.747 | not reachable through the public API | 0 |
| Panels light (20 rounded + 20 plain) | 1 | pert-ui wgpu | 0.445 (0.432–0.448) | 0.507 | compose: 0.040<br>`render` call: 0.374<br>of which until `before_present`: 0.328 | not reachable through the public API | 0 |
| Polyline 800 points | 1 | pert-ui wgpu | 3.488 (3.421–3.493) | 3.588 | compose: 0.032<br>`render` call: 3.405<br>of which until `before_present`: 3.360 | not reachable through the public API | 0 |
| Polyline 4096 points | 1 | pert-ui wgpu | 4.645 (4.636–4.704) | 4.754 | compose: 0.108<br>`render` call: 4.451<br>of which until `before_present`: 4.407 | not reachable through the public API | 0 |
| Gradient bars (32) | 1 | pert-ui wgpu | 0.444 (0.441–0.446) | 0.521 | compose: 0.016<br>`render` call: 0.396<br>of which until `before_present`: 0.351 | not reachable through the public API | 0 |
| Panels (150 rounded + 100 plain) | 1 | vello_gpu | 1.470 (1.453–1.484) | 1.551 | encode: 1.124<br>acquire: 0.037<br>prepare (`Renderer::render`): 0.218<br>submit: 0.059<br>present: 0.040 | 2.136 / 3.179 | 0 |
| Panels light (20 rounded + 20 plain) | 1 | vello_gpu | 0.577 (0.543–0.614) | 0.621 | encode: 0.282<br>acquire: 0.045<br>prepare (`Renderer::render`): 0.158<br>submit: 0.063<br>present: 0.043 | 1.821 / 2.848 | 0 |
| Polyline 800 points | 1 | vello_gpu | 0.737 (0.734–0.740) | 0.787 | encode: 0.441<br>acquire: 0.044<br>prepare (`Renderer::render`): 0.141<br>submit: 0.063<br>present: 0.043 | 1.646 / 2.722 | 0 |
| Polyline 4096 points | 1 | vello_gpu | 1.137 (1.119–1.141) | 1.195 | encode: 0.839<br>acquire: 0.044<br>prepare (`Renderer::render`): 0.143<br>submit: 0.065<br>present: 0.043 | 1.620 / 1.813 | 0 |
| Gradient bars (32) | 1 | vello_gpu | 0.294 (0.283–0.301) | 0.340 | encode: 0.027<br>acquire: 0.044<br>prepare (`Renderer::render`): 0.115<br>submit: 0.065<br>present: 0.042 | 1.544 / 1.721 | 0 |
| Panels (150 rounded + 100 plain) | 2 | pert-ui wgpu | 0.976 (0.875–0.977) | 1.037 | compose: 0.165<br>`render` call: 0.777<br>of which until `before_present`: 0.731 | not reachable through the public API | 0 |
| Panels light (20 rounded + 20 plain) | 2 | pert-ui wgpu | 0.449 (0.415–0.453) | 0.506 | compose: 0.041<br>`render` call: 0.377<br>of which until `before_present`: 0.332 | not reachable through the public API | 0 |
| Polyline 800 points | 2 | pert-ui wgpu | 7.045 (7.028–7.067) | 7.215 | compose: 0.034<br>`render` call: 6.962<br>of which until `before_present`: 6.918 | not reachable through the public API | 0 |
| Polyline 4096 points | 2 | pert-ui wgpu | 8.670 (8.646–8.745) | 8.874 | compose: 0.110<br>`render` call: 8.471<br>of which until `before_present`: 8.426 | not reachable through the public API | 0 |
| Gradient bars (32) | 2 | pert-ui wgpu | 0.447 (0.443–0.447) | 0.492 | compose: 0.017<br>`render` call: 0.398<br>of which until `before_present`: 0.351 | not reachable through the public API | 0 |
| Panels (150 rounded + 100 plain) | 2 | vello_gpu | 1.844 (1.821–1.850) | 1.912 | encode: 1.409<br>acquire: 0.041<br>prepare (`Renderer::render`): 0.288<br>submit: 0.061<br>present: 0.042 | 4.057 / 4.914 | 0 |
| Panels light (20 rounded + 20 plain) | 2 | vello_gpu | 0.705 (0.699–0.709) | 0.765 | encode: 0.376<br>acquire: 0.044<br>prepare (`Renderer::render`): 0.182<br>submit: 0.062<br>present: 0.042 | 3.531 / 4.306 | 0 |
| Polyline 800 points | 2 | vello_gpu | 1.093 (1.056–1.101) | 1.149 | encode: 0.778<br>acquire: 0.045<br>prepare (`Renderer::render`): 0.161<br>submit: 0.065<br>present: 0.043 | 3.233 / 4.420 | 0 |
| Polyline 4096 points | 2 | vello_gpu | 1.533 (1.501–1.534) | 1.593 | encode: 1.226<br>acquire: 0.042<br>prepare (`Renderer::render`): 0.160<br>submit: 0.062<br>present: 0.041 | 3.253 / 3.874 | 0 |
| Gradient bars (32) | 2 | vello_gpu | 0.300 (0.268–0.301) | 0.340 | encode: 0.027<br>acquire: 0.045<br>prepare (`Renderer::render`): 0.116<br>submit: 0.066<br>present: 0.043 | 3.189 / 4.125 | 0 |


## Reading

- **pert-ui wgpu's UI frame is shorter on the two panel scenes; vello_gpu's
  on the polylines and the gradient bars.** This is UI-thread time only;
  pert-ui's GPU time could not be measured.
  - Panels: 0.99 ms at scale 1 against vello_gpu's 1.47, and 0.98 at scale 2
    against 1.84.
  - Light panels: 0.45 against 0.58 / 0.71.
  - Gradient bars: 0.44 against 0.29 / 0.30. Here vello_gpu is faster: 32
    gradient fills against pert-ui's 32 clipped images.
  - pert-ui's time barely depends on the frame size: masks, rectangles and
    images cost the same work per command at both scales.
  - vello_gpu's cost is mostly scene encoding (path processing into strips),
    every frame: 1.12 ms for the full panels at scale 1 and 1.41 at scale 2.
- **The polyline is pert-ui's weak case.**
  - pert-ui: 3.49 / 4.65 ms at scale 1 and 7.05 / 8.67 ms at scale 2 for 800 /
    4096 points, almost all of it inside `render` before `before_present`.
    That is the CPU vector coverage (`VectorRasterizer`); the in-crate probe
    measured 3.16 / 4.29 ms for it at 960×640.
  - vello_gpu: 0.74 / 1.14 and 1.09 / 1.53 ms.
- **vello_gpu's GPU completion**, measured from submit:
  - 1.5–2.1 ms (p95 1.7–3.2) at 1280×800;
  - 3.2–4.1 ms (p95 3.9–4.9) at 2560×1373.

  It runs after the UI thread has returned, so at these frame rates it
  limits throughput only when it exceeds the frame period. pert-ui's GPU
  time on the same scenes is unknown (see Conditions).
- **Read-back costs: the spike's earlier offscreen totals are dominated by
  it.**
  - Example: vello_gpu's full panels cost 4.29 ms at scale 1 with read-back,
    against a 1.47 ms UI frame plus 2.14 ms GPU completion here.
  - Example: pert-ui wgpu's full panels cost 4.85 ms with read-back, against
    a 0.99 ms UI frame here.
- Not measured: input-to-photon latency, compositor timing, GPU timestamps,
  presentation at a real 2560×1600, other present modes, and power.
