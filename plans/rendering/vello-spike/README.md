# vello_cpu / vello_gpu measurement spike

Throwaway evidence for phase 0, item 6 of
[plans/rendering-capabilities.md](../../rendering-capabilities.md): what
vello_cpu 0.3.0 and vello_gpu 0.3.0 cost and look like on the same content as
pert-ui's CPU and wgpu renderers. Nothing here is a dependency of the
library, and the crate never joins the repository workspace (its own
`[workspace]` table, `publish = false`). The results and their reading are in
[results/results.md](results/results.md).

## What it does

`src/workloads.rs` describes each workload once, in logical pixels; every
renderer draws it from that data:

- **Panels**: 150 rounded rectangles (radius 4–8 px, 1 px inward border in
  another color) in a grid plus 100 plain rectangles inside them; also a light
  variant with 20 + 20 that is closer to a gallery page's command count.
- **Polyline**: a 1.5 px stroked polyline with round joins and butt caps of 800
  and of 4096 points in an 800×400 plot, changing every frame (the signal of
  `crates/pert-ui-wgpu/src/realtime_probe.rs`).
- **Gradient bars**: 32 meter bars whose lit part shows a 3-stop vertical
  gradient, levels changing every frame. pert-ui has no gradient: it draws one
  CPU-built `RasterImage` of the whole meter gradient (1 texel wide, one texel
  per physical row), shared by all bars and drawn per bar under a clip of the
  lit part, the cheapest way an application can do it today. vello draws a
  peniko linear gradient (sRGB interpolation, the default) per bar.

At 1280×800 logical pixels, scale 1 and 2, `src/main.rs` measures 200 frames
after 20 warm-up frames per renderer:

- pert-ui CPU and wgpu: a headless `pert_ui::Runtime` with one component
  (`src/pertui.rs`, public facade only) that paints through `fill_shape`,
  `stroke_shape`, `stroke_polyline`, `fill_rect` and `draw_image`; compose is
  `Runtime::paint`, render is `CpuRenderer::render` or
  `pert_ui_wgpu::Renderer::render_offscreen` (which always reads back).
- vello_cpu (`src/vello.rs`), single-threaded and with the `multithreading`
  feature's default worker count: encode (the drawing calls plus `flush`)
  and `render` into a pixmap.
- vello_gpu on its own wgpu 30.0.1 Vulkan device (same wgpu version as
  pert-ui-wgpu, so one process and one binary): scene encoding,
  `Renderer::render` (CPU preparation and recording), submit + blocking wait
  for completion, then a separate copy + map + wait read-back to RGBA8.

It then compares the last frames' pixels against pert-ui CPU, saves PNGs and
crops, and (part 2) translates the pert-ui `DisplayList` of each workload
command by command into vello_cpu (`src/translate.rs`). `gallery.sh` runs the
same translator over every headless gallery page (`gallery/gallery_probe.rs`
in a scratch crate beside a copy of the gallery's modules).

## Reproduce

Needs an unlocked desktop session with a GPU (a window opens briefly because
pert-ui's wgpu renderer needs a surface; every frame is rendered offscreen).
From the repository root:

```sh
plans/rendering/vello-spike/run.sh 3       # workloads, 3 pinned runs
plans/rendering/vello-spike/gallery.sh     # part 2 on the gallery, CPU only
plans/rendering/vello-spike/present.sh 3   # follow-up: presentation, 3 gated runs
```

`present.sh` runs `vello-spike present` (`src/present.rs`): both GPU
renderers present every workload to a window surface without read-back
(results in `results/presentation.md`). Each run first waits for a quiet
machine through `../tools/quiet-run.sh`. The blend-storage follow-up lives
in `../cpu-blend-variants/`.

`python3 -I plans/rendering/vello-spike/summarize.py
plans/rendering/vello-spike/results/raw-workloads-*.md` prints the
cross-run timing tables that `results/results.md` contains, and
`summarize_present.py` does the same for `results/raw-present-*.md`.

Both scripts build with `--release --target-dir target/vello-spike` in this checkout
and reuse the crate's `Cargo.lock`, which started as a copy of the
repository's so shared dependencies keep their versions. `run.sh` pins to
CPUs 2–7 with `taskset`; `gallery.sh` does not pin (prefix it with
`taskset -c 2-7`). Environment variables: `SPIKE_FRAMES` (measured frames,
default 200), `SPIKE_OUT` (image directory, default `results`),
`SPIKE_SKIP_TRANSLATE` (skip part 2), `SPIKE_SAVE_PAGE=<key>` for
`gallery.sh` (save that page's frames as PNGs).

`cargo run --release --target-dir ../../../target/vello-spike --bin capdiag
[FRAME]` (from this directory, CPU only)
reproduces the CPU/GPU parity lead: it prints the polyline's prepared mask
rectangle at scale 2 and the mask texels around physical (478, 610). See
RENDER-04 in the phase 0 report.
