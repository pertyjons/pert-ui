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
- [ ] **NATIVE-04** 2026-10-08, narrowed 2026-11-19
  `apps/gallery/src/interaction_check.rs` and
  `apps/gallery/src/workspace_check.rs` — Verification and review: the probes
  report a hidden window immediately and the native runner ends that process
  with the reported cause. A Wayland minimize can go unreported (winit
  0.30.13 delivers no `WindowEvent::Occluded` there and the surface did not
  report `Frame::Occluded`, Fedora 44 GNOME, 2026-11-05); the probes' and the
  runner's module documentation say so since 0.3.122, and that such a run
  ends by an unrelated failure, the timeout or an unaware pass. Still open:
  on macOS 26.7, activating Finder during `--interaction-check` previously
  stalled for an unidentified cause; that platform needs its own
  interruption check. A separate interruption cause for a window that
  presents no frame within a bound after an action would also cover Wayland,
  and needs a minimize by hand to verify.
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
- [ ] **RENDER-05** 2026-11-12, narrowed 2026-11-19
  `crates/pert-ui-cpu`, `crates/pert-ui-wgpu/src/shapes.rs` — Improvement
  (performance): 0.3.120 removed most of phase 1's offscreen wgpu
  slowdown (`plans/rendering/render05-probe/README.md`). What remains,
  measured again on 2026-11-19 on Intel UHD Graphics 770: the CPU
  renderer's mixer frame is 0.11 ms slower than at 0.3.103 (0.92 → 1.03
  ms); the gallery's card workload pans and zooms about 5 % slower on the
  GPU; and the diagram pan's present median is 0.82 to 0.84 ms (0.72 at
  0.3.103). Ruled out for the CPU: preparing each shape twice, removed in
  0.3.122 with a gain of 0.3 %, inside the noise, so the per-pixel
  evaluation of many small shapes is what is left there. Not yet measured:
  a cheaper evaluation of strips without corners (CPU and GPU), and
  retaining encoded geometry across a pure translation, for the fill quads
  of the diagram's boxes that are encoded again when every node moves.
- [ ] **LABEL-01** 2026-11-19 `crates/pert-ui-runtime/src/layout.rs`
  `Element::with_label` with `Element::structural` — Inspection and
  diagnostics (a composite without semantics of its own, such as a `Select`,
  is named by its element's label through its internal part): a `Select`
  element that is also structural leaves its control unnamed, while the same
  element without `structural` names it. Observed in the file dialog's type
  selector with a bound label; whether a fixed label behaves the same was
  not checked. Either carry the label through a structural element or
  document that `structural` drops it, with a test of both forms.
- [ ] **BROWSER-01** 2026-11-20 `crates/pert-ui-runtime/src/file_browser/view.rs`
  entry rows — Rust and data invariants (keep presentation formatting
  separate from canonical values): a listed name with a Unicode line
  separator (U+2028) is laid out on two lines and reports
  `ExceedsFixedHeight` in the fixed-height row, seen in the file dialog test
  `a_name_the_field_cannot_show_is_neither_suggested_nor_copied`. Show such
  names on one line, for example by presenting line and paragraph separators
  as a replacement glyph, without changing the entry's path, and test it.
