# TODO

Verified deviations from [AGENTS.md](AGENTS.md) or the design notes, and
concrete improvements, found outside the scope of the task that noticed them.
See "Reporting deviations and improvements" in AGENTS.md for the entry format
and lifecycle. Entries are suggestions for the maintainer to prioritize.

- [ ] **NATIVE-01** 2026-09-20 `docs/verification.md` — Verification and review:
  the native gallery save/restart/restore passes on Fedora 44 GNOME/Wayland
  (last on 2026-11-05, 0.3.101), but Windows, macOS, Fedora KDE and Ubuntu
  26.04 GNOME/KDE from the [native evidence](docs/verification.md#native-evidence)
  matrix have no restart check. Run `cargo run --locked -p pert-ui-native-checks
  -- workspace_restart` on each and record the results before shipping support
  for those platforms; headless checks do not replace native execution.
- [ ] **NATIVE-02** 2026-09-21 `docs/verification.md` — Verification and review: Fedora 44
  GNOME/Wayland has the injected-pointer native check and a hand check
  (2026-09-26), but Windows and macOS have no native evidence for the drag
  session or the tab drag-out: run `platform::drag_smoke` and the hand check on
  the gallery's Drag & drop page there and record the results in the evidence record.
- [ ] **NATIVE-03** 2026-10-05 `docs/verification.md` native evidence — Verification and review: floating tool-window frames and own native windows now have Linux evidence (Fedora 44 GNOME/Wayland, 2026-10-05) through the native gallery interaction check, but Windows and macOS have none. Run `cargo run --locked -p pert-ui-native-checks -- interaction` and the hand check listed in the evidence table there and record the results.
- [ ] **NATIVE-04** 2026-10-08 `apps/gallery/src/interaction_check.rs` and
  `apps/gallery/src/workspace_check.rs` — Verification and review: the probes
  report a hidden window immediately and the native runner ends that process
  with the reported cause, but on Fedora 44 GNOME/Wayland a minimize is never
  reported (2026-11-05, 0.3.101, `docs/verification.md`): winit 0.30.13
  delivers no `WindowEvent::Occluded` on Wayland and the surface did not
  report `Frame::Occluded`, so the GPU interaction probe failed on an
  unrelated step, the CPU one timed out and the workspace check passed
  unaware. Fix: document in the probes' and the runner's rustdoc that a
  Wayland minimize is not observable and ends a probe by an unrelated
  failure or the timeout, or give the probes a separate interruption cause
  for a window that presents no frame within a bound after an action. On
  macOS 26.7, activating Finder during `--interaction-check` previously
  stalled for an unidentified cause; that platform still needs its own
  interruption check.
- [ ] **TEXT-01** 2026-10-18 `crates/pert-ui-runtime/src/text_fields.rs`
  `TextArea` — Improvement (plan slice 6, large-buffer editing): an editor's
  text is limited to `TextEditError::MAX_TEXT_BYTES` (16 KiB) and is prepared
  as one `TextLayout`, which `TextLimit::MAX_RASTER_SIDE` also bounds (about
  400 lines at scale 1), so a document editor cannot hold a real document.
  Decide between extending `TextArea` and a separate document editor, then
  prepare text per paragraph and rasterize only visible paragraphs, carrying
  selection, caret, IME and movement across paragraph boundaries (the public
  `TextSelection` is built on one `TextLayout`). Measure edit latency and
  memory on a defined corpus before choosing a capacity; see
  `plans/app-author-improvements.md` slice 6. The editor capacity must also
  become separate from the other text limits that share `MAX_TEXT_BYTES`
  today: labels, toasts, IME preedit, combined rich text, ellipsis fitting and
  label help tooltips. A compile-time check (`text/tabs.rs` `MAX_STYLES`) ties
  it to Parley's style limit, so it cannot exceed about 32 KiB.
  `UiStrings::MAX_TEXT_BYTES` (1024) is a separate limit. The design plan is
  [plans/document-editor.md](plans/document-editor.md) (2026-11-04, accepted
  as direction, not scheduled; its raster admission and memory guarantees are
  open items OD1–OD6).
- [ ] **NATIVE-05** 2026-10-20 `docs/verification.md` — Verification and
  review: the `diagram_editor`, `log_explorer` and `inventory` example
  windows, `SelectableText`, table cell editing and Tab traversal scopes have
  Linux hand-check evidence (Fedora 44 GNOME/Wayland, 2026-10-28 and
  2026-11-05) but none on Windows or macOS. Run them by hand there (the
  examples' main tasks, pointer selection and copying into another
  application, double-click and F2 editing with commit and cancel, Tab
  through a scoped table, IME into a cell editor) and record the results.
  IME composition into a cell editor has no native evidence on any platform:
  the maintainer deferred it on Linux on 2026-11-05 together with script
  coverage beyond the bundled fonts (`plans/follow-ups.md`).
- [ ] 2026-10-27 `Cargo.lock` `clipboard-win` — Rust and data invariants
  (dependencies): the yanked `clipboard-win 5.4.1` (Windows clipboard, through
  `arboard`) was updated to 5.4.3 on 2026-11-04 (0.3.100), and the Linux
  dependency gate passed. The Windows native clipboard check of the update is
  still missing; run it on Windows and record the result.
- [ ] 2026-11-04 `crates/pert-ui-runtime/src/text_fields.rs` `TextField`/
  `TextArea` binding refusal and raster limits — State and interaction
  contracts: the editors' rustdoc says a bound value that does not qualify
  "is refused by the field rather than failing the runtime", but
  `TextArea::check` and the binding refusal test only the byte limit and the
  characters. A value within `TextEditError::MAX_TEXT_BYTES` can still exceed
  `TextLimit::MAX_RASTER_SIDE` (8192 px). With 16,380 bytes of prose in a
  320 px `TextArea` at scale 1, the first `Runtime::flush` returns
  `Err(Text(TooLarge(RasterSide)))` (author probe, 2026-11-04, found by the
  TEXT-01 design work). An edit that crosses the raster limit also differs from
  one at the byte limit. The byte-limit edit is refused silently, but the
  raster-limit edit makes `Runtime::text_input` return the same error. In the
  author's probe this happened at step 165 of typing into 8 KiB at 320 px, and
  the binding was not overwritten. Fix: either refuse a bound value whose
  preparation exceeds a raster limit through the same binding refusal (keep
  the previous text, report `binding_error`), or document the raster limit as
  part of the editor's acceptance and make `check` say it cannot see it. Make
  the edit path refuse silently like the byte limit, or document the error.
  Add tests at narrow width and scale 2. `plans/document-editor.md` slice 6
  removes the cause for values within capacity; this entry covers the
  current contract until then.
- [ ] **RENDER-02** 2026-11-05 `crates/pert-ui-runtime/src/dialog.rs:470`,
  `crates/pert-ui-runtime/src/quick_pick.rs:1371` — Improvement: the modal
  scrim is the literal `Color::from_linear(0.0, 0.0, 0.0, 0.35)` in two
  places, so no theme or palette can change it and the two can drift apart.
  Give it a `ColorRole` with a default in the role table and use it in both,
  documenting the role in the appearance guide.
- [ ] **RENDER-04** 2026-11-05 `crates/pert-ui-runtime/src/geometry/raster.rs`
  `RasterGrid::new` — Improvement: vector coverage is rasterized on a grid
  anchored at the geometry's own bounds, so a prepared `DrawMask` rectangle
  has an arbitrary fractional physical position. A vector at a fractional
  phase, nearly every one, is then resampled bilinearly, and its first column and row straddle a pixel: the
  pixel-center rule drops or keeps that half-covered pixel whole. When the
  edge falls within the GPU's subpixel precision of a pixel center, the
  documented exception in `docs/rendering-platform.md` (rendering tiers)
  occurs on ordinary content. In the phase 0 polyline (800 points, scale 2,
  frame 219) the mask's left edge lies at physical 478.50122. The CPU
  renderer leaves pixel (478, 610) as background, and wgpu draws edge texel
  coverage 249 there: a 220/255 difference. Rasterize so that the final
  `DrawMask` rectangle lies on physical pixels: the grid phase must include
  the command's origin and scale, which `VectorRasterizer::prepare` applies
  after rasterizing (`vector_scene.rs`), so the phase joins the cache key.
  A raster scale capped below the view's (`raster_scale`) still stretches.
  Then add CPU/GPU parity fixtures with vector edges at arbitrary phase.
  Reproduction:
  `plans/rendering/vello-spike/src/bin/capdiag.rs`.
- [ ] 2026-11-11 `crates/pert-ui/src/app.rs` `AppError::from_renderer` —
  typed errors (Rust and data invariants): a presentation failure with
  `RenderError::Shape` becomes `AppError::Graphics` text, although captures
  keep the typed `ShapePrepareError`. The phase 1 spec named only the capture
  and renderer conversions. Decide whether `AppError` gets a typed cause for
  it (it is the component's failure, like `Drawing`), and add it with a test.
- [ ] **RENDER-05** 2026-11-12 `crates/pert-ui-wgpu/src/shapes.wgsl`,
  `crates/pert-ui-wgpu/src/shapes.rs` — Improvement (performance): after
  phase 1 the offscreen wgpu render with read-back, the capture path, is
  slower than at 0.3.103 on Intel UHD Graphics 770: a median of 7 % on a
  repeated page render and at most 22 %
  (`plans/rendering/phase1-probe/results/summary.md`, `gpu_*` rows).
  Presenting to a window is not slower by its medians; one tail is, the
  diagram pan's 95th percentile (0.88 → 0.94 ms). The cause is not found; the
  fragment stage reading its 144-byte record from the storage buffer for
  every pixel is a candidate. Measure with the phase 1 probe whether passing
  the parameters as instance vertex attributes, or evaluating the edge
  strips with a smaller record, removes it. The CPU renderer's mixer frame
  is likewise 0.11 ms slower (0.94 → 1.06 ms): many small shapes are
  prepared one by one.
- [ ] 2026-11-12 `crates/pert-ui-runtime/src/shapes/prepare.rs`
  `ShapeGeometry::misses` — Rust and data invariants (a total rule): a
  triangle's allowance is one pixel by the viewport's scale, while its
  vertices and the clip map by the frame's pixels per logical unit. They
  agree for any practical frame, but not when `Viewport::frame_extent`
  saturates at `u32::MAX` (found by the phase 1 final review with a viewport
  of `(64, 1e-10)` at scale `1e10`): a triangle covering a pixel inside its
  clip is then dropped. Take the allowance per axis in the frame's terms, as
  the arc's reach is, and add the case to the tests.
