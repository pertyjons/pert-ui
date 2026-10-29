# Integrated menu ownership and literal-tab coverage

Working0.3.92, 2026-10-29. The isolated menu and text patches received fresh
Codex approval after required transition/coordinate/doc repairs.
[Independent integrated review](../menu-text-integrated-review-r1.md) approved
source/integration with API/record conditions, now discharged below.

## Complete evidence

- ../gates/final-0.3.92/: all eleven required CI base commands exited0, with
  exact command/environment/timestamps and before/after source hashes equal.
  The review calls this twelve commands; that is a counting error in its raw
  prose, corrected here. The actual outcomes list is authoritative.
- api/: generation exited0, git status --porcelain -- api/ empty. The review
  read before outcome.json was present; its pending status is historical.
- native/: selected GPU and CPU gallery interaction checks exited0 on Fedora44,
  kernel7.2.12, GNOME/Wayland, rustc1.99.1, winit0.30.13/wgpu30.0.1. The adapter
  was not reported. Both new parked-title/duplicate-motion and parked-row
  assertions passed in the actual native window. Other checks explicitly
  SKIPPED by the name filter; no OS-adapter, human, Windows/macOS or tab-ink
  GPU-pixel claim. These runs fall within the unchanged-source gate window.
- negative-controls/: restoring each old gate rejects its named new gallery
  assertion; exact source restored/touched and restored run passes. Original
  logs/outcomes and runner bytes preserved. Exact diff reconstruction and the
  gallery hash are retrospective, explicitly marked in reconstruction.json;
  they are not fabricated contemporaneous records. Positive native/headless
  bytes correlate with the complete gate manifest.

## Author attempts and final review repairs

Gallery compile attempts1/2 guessed local_bounds as a readable field/getter;
it is the reporter's builder. Corrected to public SemanticItem.bounds, already
in window coordinates. Attempt3's broad filter ran only the existing hidden-
window probe; it is NOT scenario evidence. The separate steps-attempt1 log
runs cli::the_interaction_steps_pass_headlessly and passes. Negative controls
and native runs then exercise those same actual steps.

The reviewer requested TODO entries for empty-raster bounds and hanging-caret
reveal. Both are added and actively assigned their own approved-design step;
this checkpoint does not claim either repaired. The early controls fallback
sentence now qualifies literal tabs, matching the later paragraph.
Final-guide-doctest.log records cargo test --locked -p pert-ui --doc guide::controls;
final-guide-rustdoc.log records RUSTDOCFLAGS=-D warnings cargo doc --locked
-p pert-ui --no-deps. Both pass; these command descriptions are retrospective.
final-source-correlation.json proves those exact prose-only differences and
that all runtime/test/asset/manifest/API bytes still match the complete gate.
No unrelated workspace rerun is inferred or needed. Final whole-round review
remains owed after MultiSelect and whitespace follow-ups.
