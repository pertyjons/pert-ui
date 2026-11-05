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
  windows, `SelectableText`, table cell editing (the gallery's Note column)
  and Tab traversal scopes have headless tests but no native evidence on any
  platform; the example windows were at most opened to see that they start
  and close.
  Run them by hand on Linux, Windows and macOS (pointer selection and
  copying, double-click and F2 editing, Tab through a scoped table, IME into
  a cell editor) and record the results.
  Partial Linux evidence, 2026-10-28: the diagram editor window's close
  policy (moved node, refused inspector draft) and the gallery's Note cell
  editing at three densities were hand-checked (`docs/verification.md`,
  0.3.83, 0.3.86, 0.3.87), and the gallery sign-in form's Revert; the rest
  of this entry stays open.
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
