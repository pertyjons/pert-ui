# TODO

Verified deviations from [AGENTS.md](AGENTS.md) or the design notes, and
concrete improvements, found outside the scope of the task that noticed them.
See "Reporting deviations and improvements" in AGENTS.md for the entry format
and lifecycle. Entries are suggestions for the maintainer to prioritize.

- [ ] **NATIVE-01** 2026-09-20 `docs/verification.md` — Verification and review:
  native gallery save/restart/restore passed on Fedora GNOME/Wayland, but Windows,
  macOS and the remaining Linux combinations in
  [native evidence](docs/verification.md#native-evidence) still lacks restart checks.
  Run the documented two-process check and record the results before shipping
  support for those platforms; headless checks do not replace native execution.
- [ ] **NATIVE-02** 2026-09-21 `docs/verification.md` — Verification and review: Fedora 44
  GNOME/Wayland has the injected-pointer native check and a hand check
  (2026-09-26), but Windows and macOS have no native evidence for the drag
  session or the tab drag-out: run `platform::drag_smoke` and the hand check on
  the gallery's Drag & drop page there and record the results in the evidence record.
- [ ] **NATIVE-03** 2026-10-05 `docs/verification.md` native evidence — Verification and review: floating tool-window frames and own native windows now have Linux evidence (Fedora 44 GNOME/Wayland, 2026-10-05) through the native gallery interaction check, but Windows and macOS have none. Run `cargo run --locked -p pert-ui-native-checks -- interaction` and the hand check listed in the evidence table there and record the results.
- [ ] **NATIVE-04** 2026-10-08 `apps/gallery/src/interaction_check.rs` and
  `apps/gallery/src/workspace_check.rs` — Verification and review: both probes
  now pace with update deadlines, report a hidden window immediately, and the
  native runner ends that process with the reported cause. Headless hide and
  resume tests and normal Fedora 44 GNOME/Wayland GPU and CPU runs pass. The
  actual Fedora minimize path still needs a native interruption run to verify
  that the compositor reports hiding and the runner returns the cause rather
  than an unrelated assertion or timeout. On macOS 26.7, activating Finder
  during `--interaction-check` previously stalled for an unidentified cause;
  that platform also needs a separate interruption check.
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
  become separate from the label and toast limits, which share
  `MAX_TEXT_BYTES` today.
- [ ] **NATIVE-05** 2026-10-20 `docs/verification.md` — Verification and
  review: the `diagram_editor`, `log_explorer` and `inventory` example
  windows, `SelectableText`, table cell editing (the gallery's Note column)
  and Tab traversal scopes have headless tests but no native evidence on any
  platform; the example windows were at most opened to see that they start
  and close.
  Run them by hand on Linux, Windows and macOS (pointer selection and
  copying, double-click and F2 editing, Tab through a scoped table, IME into
  a cell editor) and record the results.
- [ ] **TOOL-01** 2026-10-20 `crates/pert-ui-runtime/src/tool_windows/mod.rs`
  `ToolWindow::mode` — State and interaction contracts (found by
  plan-audit-repairs slice 6): `ToolWindow::mode` promises where the tool
  window "starts to be presented when it expands", and the type's
  **Own window** section says a window-mode panel opens in its own native
  window when expanded. A tool window described with `ToolWindowMode::Window`
  and `expanded(true)` opens no native window at mount, though: it is
  placed as a floating frame until the mode or expansion is set again
  through `ToolWindowController`, which requests the detach. Request the
  detach for an expanded window-mode description at mount (or after the
  first layout), and add a mount-time regression test.
- [ ] 2026-10-21 `NOW.md` — Reporting deviations and improvements: the
  handoff file was last updated 2026-10-15 and is stale. It gives the
  version as 0.2.96 (the workspace is at 0.3.52) and says the
  `app-author-improvements` branch is not merged, while 116ab0c and later
  commits merged it and the probe and audit plans that followed. Rewrite
  it for the current `main` and open plans, or remove it if
  `plans/` and the CHANGELOG are enough as the handoff.
- [ ] 2026-10-21 `docs/verification.md` "Workspace gate" — Verification and
  review: the command block presented as the workspace gate omits the
  packaged examples' headless sessions (`cargo run --locked -p pert-ui
  --features png,svg,cpu --example ...`) that AGENTS.md and
  `.github/workflows/ci.yml` include, so a reader following it runs less
  than CI; since slice 1 of the developer-usability plan it also lacks the
  API listing gate. Replace the block with a reference to the AGENTS.md
  gate, or copy the full list and keep it in step with the other two.
- [ ] 2026-10-21 `crates/pert-ui-runtime/src/capture.rs` `FrameImage::encode_png`
  — Concrete improvement (found by the developer-usability triage): the
  encoder writes stored, uncompressed deflate blocks without row filters,
  as its rustdoc says, for determinism without a dependency. A blind-test
  window capture of 1040×600 took 2.5 MB; a level-6 zlib encode of the same
  pixels takes 18 KB. A small deterministic deflate (or an optional
  pure-Rust one such as `miniz_oxide`, already in the lockfile through
  `png`) would make captures practical to attach and commit; keep the
  output deterministic and test it against a decoder.
