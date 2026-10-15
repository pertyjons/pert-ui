# pert-ui-cpu

CPU interpretation of the [pert-ui](https://docs.rs/pert-ui) retained drawing
commands. `CpuRenderer` turns a `DisplayList` into an owned `FrameImage` with no
window, GPU, display server or worker thread, following the same rendering model
as the hardware renderer: linear-light premultiplied blending stored as 8-bit
sRGB after each command, pixel-center coverage and the same text, vector and
image resources.

Uses:

- **Pixels without graphics hardware.** Tests, CI and documentation builds can
  render real frames and compare them with expected pixels.
- **Screenshots through inspection.** As a capture host,
  `CpuRenderer::serve_captures` answers window, component, key-path and region
  capture requests, including those an inspection client sends from another
  thread, so tooling can capture part of a view, a whole window or a single
  component headlessly.

Native presentation from CPU pixels and browser rendering are not part of this
crate. The rendering contract and its tolerances against the GPU renderer are
recorded in the repository's `docs/rendering-platform.md`.

Licensed under either the MIT or the Apache-2.0 license, at your option.
