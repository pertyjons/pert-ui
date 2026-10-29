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
  Partial Linux evidence, 2026-10-28: the diagram editor window's close
  policy (moved node, refused inspector draft) and the gallery's Note cell
  editing at three densities were hand-checked (`docs/verification.md`,
  0.3.83, 0.3.86, 0.3.87), and the gallery sign-in form's Revert; the rest
  of this entry stays open.
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
- [ ] 2026-10-28 `crates/pert-ui-runtime/src/virtual_list.rs` `ListItems::build`
  and `crates/pert-ui-runtime/src/tree.rs` `TreeItems::build` — developer
  usability (the same kind as the `Each::new` placeholder key that round 2
  part P5 resolved with `EachItem`): both return an `Element` whose key the
  collection replaces with the row's, so an implementation writes a
  placeholder key that never takes effect, as the diagram editor's rows do
  (`crates/pert-ui/examples/diagram_editor/app.rs`, `.key("row")`), or
  computes the key a second time to build the element
  (`apps/gallery/src/pages/data.rs`). They are trait-object methods, so a
  generic return such as `EachItem`'s does not apply. Give the traits an
  object-safe keyless row value (a breaking change to every
  implementation), and drop the placeholder keys from the examples. Found
  by round 2 part P5's checkpoint.
- [ ] 2026-10-28 `crates/pert-ui-runtime/src/virtual_list.rs` rows (and so
  `Tree` rows) — development validation (the same kind as R2-23, which round
  2 part P8 resolved for `Table` cells with `ValidationHints::fixed_height`):
  each row is placed with `place_clipped(child, slot, slot)`, so a row whose
  minimum height exceeds the list's row height is cut off with no report.
  The hint does not fit there, since the slot rather than the list gives the
  fixed height; give the list's rows the check, for example by comparing
  each row's measurement with its slot, and test it with a button in a short
  row. Found by round 2 part P8's checkpoint.
- [ ] 2026-10-28 built-in item reporters without rectangles — testing
  (R2-27's improvement for other controls): `Accordion` and `Collapsible`
  headers (`collapsible.rs`), the choice controls (`choice_controls.rs`),
  `MultiSelect` (`multi_select.rs`),
  `QuickPick` (`quick_pick.rs`) and the tool-window stripe
  (`tool_windows/stripe.rs`) report `SemanticItem`s without
  `SemanticItem::local_bounds`, so `Driver::click_item` refuses them with
  `ItemWithoutBounds`. Each that draws its rows could give their rectangles
  from the geometry its painting and hit testing use, as the table's titles
  do since 0.3.83, with a test that clicks one. Found by round 2 part P8's
  checkpoint.
- [ ] 2026-10-28 `crates/pert-ui-runtime/src/runtime/content_search.rs`
  `Runtime::content_search` — testing and search (round 2 decision D6):
  content search matches a component's semantic name and value only
  (`content_hit`), not its `Semantics::items`, so text exposed only through
  items (a toast's message, a menu command, a tab title other than the
  selected one, which its choice value carries) is never a hit, and a test
  or an application search cannot find a visible toast by its text. Extend
  the search to items (a hit naming the component and the item's index, with
  `jump_to_content` revealing and focusing the component), document it in
  `docs/contracts.md` ("Content search"), which states the limitation since
  0.3.84, and test it with a toast and a menu. Found by round 2 part P9.
- [ ] 2026-10-28 `crates/pert-ui-runtime/src/validation/development.rs`
  overlap check (`:744-760`) — Verification and review (development
  validation cost): the check compares every pair of visible components with
  `OverlapPolicy::Disallow` in the application, keeps the pairs that share a
  window and a parent, and writes one `[pert-ui validation ERROR
  UnexpectedOverlap]` line per new pair, so N overlapping components cost
  N(N-1)/2 comparisons and lines (500,500 for the 1,001 roots of the
  pixel-readback probe in the 2026-10-27 native run, before round 2 part P11
  allowed the probes' overlap). Group the peers by window and parent before
  comparing, and above a threshold summarize per parent, for example "N
  components overlap under <parent>; first pairs: ...", with a test of the
  summary, the threshold and the issues the report still lists.
  Round 3's workspace and runtime-serde gates also reproduce this flood in
  `tests/runtime.rs::shared_live_deadlines_skip_missed_frames_without_visiting_static_components`,
  whose deliberately coincident static/live roots do not declare allowed overlap.
  Mark that scheduling fixture's intentional overlap separately so its output
  does not bury unrelated failures; preserve the dedicated validation tests.
- [ ] 2026-10-28 `crates/pert-ui/tests/presentation/svg.rs`
  `an_image_element_reads_no_file_and_no_embedded_data` — verification
  (tests independent of leftover build state): the test writes into
  `env!("CARGO_TARGET_TMPDIR")` without creating it; Cargo creates that
  directory when it builds the test, so deleting `target/tmp` after a build
  (as a disk cleanup did on 2026-10-28) makes the next run of the unchanged
  binary fail with `NotFound`. Call `std::fs::create_dir_all` on the
  directory first.
- [ ] 2026-10-28 `crates/pert-ui-runtime/src/dialog.rs`
  `AlertDialog::take_children` — public inspection/testability improvement:
  the generated confirmation buttons are structural and declare no exposed
  part, so their semantic names can be inspected but a widget target requires
  traversing private `accept`/`cancel` keys. The Menu/Select parts checkpoint
  does not cover this control family. Expose those buttons in the existing
  `dialog/confirmation` scope, preserve workspace addressing and semantic
  naming, and add a public pointer test; this would give applications a stable
  route without private-key traversal. Verified during round 3's independent
  implementation review. The new docs no longer recommend that traversal.

- [ ] 2026-10-28 `crates/pert-ui-runtime/src/select.rs`
  `SelectAnchor::paint` and `crates/pert-ui-runtime/src/multi_select.rs`
  paint — presentation consistency: disabled dropdowns use Button fill and
  Muted text, the same text-only distinction reported for Button in usability
  round 3. A Button-only neutral disabled-frame repair does not change these
  separate paint paths. Decide and document a consistent disabled dropdown
  treatment using adaptive roles, then verify both palettes and shared
  caption/icon/arrow content without changing permission or selection behavior.
- [ ] 2026-10-28 `crates/pert-ui-runtime/src/controls.rs`
  `Button::inspect` — inspection improvement: painted button background,
  foreground and border colors are absent from its box model, while Container
  and Toolbar report theirs. Retain resolved paint metadata and expose only
  measured/current values through ComponentDetails, with disabled/theme
  redescription tests; inspection must not paint or schedule work.

- [ ] 2026-10-29 `crates/pert-ui-runtime/src/runtime/measurements.rs`
  `Measurements::is_empty` — verification: release-profile unit-test builds
  warn that this helper is unused. It is gated by cfg(test), but its sole
  caller in runtime/tree.rs is in cfg(all(test, debug_assertions)). The round-3
  text release logs reproduce the pre-existing warning. Align the helper's
  cfg with its only caller so release tests remain warning-free; preserve the
  debug measurement-eviction test. This is outside the usability repairs.
