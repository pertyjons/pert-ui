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
- [ ] 2026-10-23 `crates/pert-ui/src/app.rs` `AppError` — public API and
  documentation: its rustdoc lists every error type the blanket
  `impl<E> From<E> for AppError` accepts, repeating the runtime's
  `leaf_conversions!` set, and nothing checks the list. Generate the list from
  the macro or add a test that converts each listed type, so a new leaf error
  cannot go undocumented.
- [ ] 2026-10-23 `tests/diagnostics/ui/c01_*`, `c06_*` — verification: the
  `Component` snapshots include rustc's first eight implementors, so a new
  built-in component whose name sorts early changes them without any message
  change. Choose cases whose output omits the implementor list (for example a
  bound on a generic function with a local trait object), or document in
  `docs/verification.md` that such churn is expected and reviewed.
- [ ] 2026-10-26 `crates/pert-ui/examples/diagram_editor/canvas.rs`
  `NodeView::paint` — application manual/examples: while a node is renamed in
  place, `NodeView` still paints its centered label under the frameless
  `DraftEditor`, which draws its leading-aligned text over it, so the two
  overlap ("Decide" over "Decide" shifted). Seen in the 6f1 hand check on
  Linux. Leave the label out of the renamed node's paint (pass the renaming
  state into `NodeView`) or give the editor area the node's fill, and check
  it in the native window.
- [ ] 2026-10-26 `crates/pert-ui-runtime/src/text_fields/draft.rs`
  `DraftAlign` — concrete improvement: it offers only `Start` and `End`, so a
  centered label, such as a diagram node's, jumps to the leading edge when it
  is opened for editing. A `Center` variant (the enum is `#[non_exhaustive]`)
  would let in-place editors keep a centered label where it was; test the
  placement and caret scrolling for narrow and wide text.
- [ ] 2026-10-27 `Cargo.lock` `clipboard-win 5.4.1` — Rust and data invariants
  (dependencies): `cargo package --locked` warns that the locked
  `clipboard-win 5.4.1` (Windows clipboard, through `arboard`) is yanked on
  crates.io. Update it with `cargo update -p clipboard-win` to a non-yanked
  compatible release and run the dependency gate, including the Windows
  native clipboard check.
- [ ] 2026-10-27 `crates/pert-ui/examples/diagram_editor/app.rs`
  `close_requested` — draft policy (contracts.md "Draft policy", rule 10):
  the close policy asks only whether the document is dirty, so a draft the
  inspector's label `TextField` or a `DecimalInput` refuses
  (`app.rs:452-498`) stays unsaved through admission and is lost with the
  window without a prompt. The fields now report kept drafts through
  `on_modified`: feed a `FormFields` from them and keep the window open, or
  ask, while `FormSummary::modified` is non-zero, with a test through
  `HeadlessApp::admit_close`.
- [ ] 2026-10-27 `crates/pert-ui-runtime/src/described.rs` `Each::new` —
  developer usability (round 1 finding F9, triage group E r1-4-claude#4):
  `build` returns an `Element`, and the only way from a component
  to an `Element` is a key, which `Each` then replaces with the item's key.
  Every caller writes a placeholder key that never takes effect: `Each`'s own
  rustdoc (`.key("task")`), the guide's layout and testing chapters and the
  round 2 fixture (`plans/usability/task4-app/src/lib.rs`, `.key("row")`,
  which round 1's triage took for a dead key the kit could drop). Let
  `build` return any component, or give `Element` a keyless form that `Each`
  keys, and drop the placeholder keys from the examples.
- [ ] 2026-10-27 `crates/pert-ui-wgpu/src/native_smoke.rs` (pixel readback,
  realtime workload probe) and `crates/pert-ui/src/platform/smoke.rs`
  (`persistent-paint-error`) — Verification and review: layering L3 migrated
  these probes' legacy roots to `mount_tree` roots, which are laid out and
  overlap-validated, but the design's migration exceptions
  (`layering-design-L3.md` §2, items 1–2) did not list them and the native
  checks were not run for L3. The 2026-10-27 native run passed but logged
  500,503 `UnexpectedOverlap` lines, 499,500 of them the pairs of the
  pixel-readback probe's 1,000 stacked `static-N` roots, so its debug-profile
  update and compose timings now include writing them. Have the overlapping
  probe components report `OverlapPolicy::Allow` (or mount them under one
  stacking container), and rerun the native suite to see a log without
  validation errors. Separately, consider whether development validation
  should cap or summarize overlap reports, since it writes one line per
  overlapping pair.
- [ ] 2026-10-28 rustdoc and guide examples with unused imports — public
  examples (round 2 finding R2-14's rule that an example needs exactly what
  it imports): a run with `#![doc(test(attr(deny(unused_imports))))]` added
  for the run to `pert-ui-runtime`'s and `pert-ui`'s crate roots fails the
  examples of `CommandPalette`, `Runtime::content_search`,
  `Runtime::reveal_child`, `Runtime::capture_workspace`, the `testing`
  module, `TextField::on_binding_refusal` and `TextArea::on_binding_refusal`
  (one macro), `DraftEditor`, `DraftEditorController::set_read_only`, and
  the guide's
  authoritative-values, appearance and controls chapters (one, one and two
  blocks), each with an import it does not use; doctests allow unused code,
  so nothing reports it. The testing chapter's opening block, the two globs
  alone, is meant that way. Remove the unused imports, then consider keeping
  that attribute so the doctests report them (a lint configuration change to
  discuss with the maintainer); the opening block would then carry a hidden
  `#![allow(unused_imports)]`.
- [ ] 2026-10-28 `crates/pert-ui-runtime/src/runtime/held.rs`
  `HeldCancellation::FocusLost` — public API documentation: the link text
  reads `` `Runtime::clear_focus` `` but targets
  `HostIntegration::clear_focus`; since L3's N3 there is no public
  `Runtime::clear_focus`, so the rendered name points readers to a method
  that does not exist. Change the link text to
  `HostIntegration::clear_focus` (a test reaches it as
  `Driver::clear_focus`). Found by round 2 part P4's checkpoint.
