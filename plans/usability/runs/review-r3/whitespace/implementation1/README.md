# Round 3 hanging whitespace: implementation evidence

Implementation author's evidence for `plans/usability/round-3-design-hanging-whitespace.md`
revision 3 (main checkout) with both design reviews, in the uncommitted worktree
`/home/per/pert-ui-usability/r3-whitespace-worktree`. It is not an independent
review. Nothing was staged or committed; no main checkout, other worktree, kit,
submission, manifest, lockfile, version, API listing, guide, changelog, TODO or
design note was edited. No native, GPU, workspace-wide or API-listing gate ran.

## Base

HEAD `17c6c3d39b3015a1563d7440541af87a1b7ee998` (0.3.91) with the approved
tab-ink patch already applied as uncommitted baseline. `00-baseline-hashes.txt`
shows the six owned baseline files byte-equal to both
`/home/per/pert-ui-usability/r3-whitespace-approved-tab-base` and the main
checkout's `plans/usability/runs/review-r3/whitespace/base`, and the fixture
assets unchanged. `baseline-src/` holds copies of those files as found.
`baseline-tree/` is `git archive 17c6c3d` plus those files and the fixture
assets; `01-baseline-tree.txt` shows its whole-tree digest equals the
unedited worktree's. Commit 3bbfb81 alone was not used.

## Layout

- `probe/`: public-facade probe (`src/main.rs`), one manifest per build
  (`baseline/` depends on `baseline-tree`, `implementation/` on the worktree),
  `run.sh` (editor observations), `run-raster.sh` (raster mode),
  `compare-frames.py`, outputs in `out/` and raw RGBA frames in `frames/`.
- `deltas/`: baseline-versus-implementation records (see Deltas).
- `findings/`: the Swash empty-outline observation.
- `negative-controls/`: `run.py`, per-control `.diff` and `.log`,
  `summary.txt`, `post-restore-rebuild.log`, `runner.out`.
- `checks/`: `run-checks.py`, one log per check, `run-checks.out`.
- `final-state/`: the patch against the approved-tab base, the tracked diff
  against HEAD and the rustdoc-only proof for `text_selection.rs`.
- `baseline-with-new-tests/` (+ `.sources.sha256`): baseline production with
  the final test files, for `deltas/new-tests-on-baseline.log`.

## Implemented policy

**Zero-area rasters** (`text.rs`, `TextService::rasterize_glyphs`): after the
phase-glyph cache lookup and admission and the raster-byte accounting, a raster
whose width or height is zero is skipped, before icon alignment, `PlacedGlyph`
construction and the bounds union. Default and phase paths share the loop. The
original string, analysis, advances, fonts, tab atom algorithm, signature,
cache keys and instrumentation are unchanged; zero-byte phase entries stay
cached.

**Wrapped caret** (`text_fields.rs`): `Prepared` stores a `Horizontal` value
computed with each arranged or painted layout:
`pin = Some(max(size.width, wrap))` for wrapping editors, `None` otherwise;
`max_scroll = max(extent - inner.width, 0)` with extent `max(size.width, wrap)`
or `size.width + caret.width`. Its pure `caret` method clamps x into
`[0, max(pin - caret.width, 0)]` and is the one pin used by paint, reveal
(`position`) and `input_state`. `position` clamps to `max_scroll`; the wheel
clamps to the stored `max_scroll`, never recomputing from a layout and a newer
selection. Selection, `TextSelection` geometry, selection fills, composition
underlines and hit testing are unpinned. The reveal loop's end-wins order, the
arrange-forced reveal, `input_state`'s freshness and final clamp, the vertical
range and the pending-edit/wheel interaction are unchanged.

**Rustdoc**: `TextLayout`, `TextLayout::size`, `TextArea` (temporary limitation
replaced; single-line range sentence), `TextSelection::at_point`/`caret`
(rustdoc only, `final-state/text_selection-rustdoc-only.txt`).

## Findings to decide

1. **Fractional-scale residual of the box formula.** Whitespace at the end of
   the text counts toward Parley's width up to the wrap width, and the measured
   size rounds that up to whole physical pixels. With a fractional physical wrap
   width (inner 38 at scale 1.25 is 47.5 pixels) the box is 38.4 > 38, so typing
   or holding a hanging space scrolls by `(ceil(w*s) - w*s)/s` = 0.4 logical
   pixels (baseline 1.4 and growing). It is constant for held Space and zero at
   scale 1 or with a whole-pixel wrap (asserted in
   `typing_and_holding_space_at_a_full_line_keeps_the_text_still`). This
   contradicts review 2's "a non-overflowing wrapped layout has box = wrap = w".
   The approved formula was kept; removing the residual would change either the
   box definition or the canvas rounding, which is a design decision.
2. **Swash renders an empty outline at a fractional offset on both axes as one
   pixel of zero coverage** (`findings/`). Parley's baselines are whole pixels,
   so the default path always gets 0-area space rasters and the fix is complete
   there (size, origin, ink, default masks). In the phase path (not the default
   `LineMask` rendering) a nonzero vertical phase gives that 1x1 blank raster,
   which the zero-area rule keeps: no visible pixel or public geometry change,
   but a blank phase mask for whitespace-only text and possible phase-part
   splits (`phase_spaces_with_a_vertical_offset_place_one_blank_pixel`). The
   design forbids a pixel scan, so this is disclosed, not removed.
3. **N3b is an equivalent mutant.** With the pin, restoring only the old reveal
   clamp `size + cw - w` changes nothing observable: a pinned caret's end never
   passes the box and the wheel never passes the stored maximum. The old clamp
   together with the raw caret (N3) fails 12 tests.
4. **Line-mask edge resampling.** Where an old ghost raster had padded a mask
   with blank pixels, the trimmed mask now ends at the ink. Drawn at a
   fractional origin (default `LineMask`), the CPU renderer's resampling at
   that edge loses part of the outermost ink column (`rtl-leading-space`
   quarter: 11 pixels, up to 79/255). Every frame whose canvas and placements
   are unchanged is pixel-identical; see Deltas.

## Deltas (`deltas/`)

- `editor-probe-baseline-vs-implementation.diff`: the editor probe at scales 1
  and 1.25, same probe source (`50608d5c…`; raster runs `final2` use the later
  source with mask sizes). Every hanging LTR/RTL caret (End, typed space/tab,
  held Space, RTL start on the right edge, Shift+End) goes from no caret fill
  and no caret pixels to a whole caret at the inner edge (`[49, 6, 1, 20]` or
  `[12, 6, 1, 20]`) with matching IME anchor and CPU pixels. Offsets: held Space
  1, 6, 14, 22, 31, 39 -> 0; typed space 1 -> 0 (1.4 -> 0.4 at 1.25); prose
  0.796 -> 0; nearly full 0.6 -> 0 with the wheel Ignored instead of Handled;
  RTL space twin 5 -> 0 and now equal to the tab; overflow and single-line
  wheel ends now at the reveal range's end (139.4/139 -> 139/139 Ignored;
  single line 97 -> 98 with the whole caret). Tiny areas use end-wins (24.5:
  caret now visible). The pending-edit wheel still cancels the reveal.
- `raster-frames-final2.txt`: 72 frames (18 cases, 2 scales, whole and
  quarter origin). 60 pixel-identical, including all bundled-icon cases. The 12
  differing frames all changed mask extent: RTL wrapped hanging space
  (translation by the removed ghost edge, `rtl-wrapped-translation-check.txt`:
  exact at scale 1 except 4 columns the baseline had clipped at the widget,
  4 logical/5 physical pixels with resampling differences <= 11/255 at 1.25),
  and finding 4's edge resampling at quarter origins (colored space split,
  inline icons, RTL leading space) and two 1.25 frames of trimmed wrapped masks
  (<= 5/255). Mask deltas: whitespace-only 1 -> 0 masks; colored space split
  2 -> 1; inline icons' space-bearing parts narrower; wrapped hanging-space
  masks 51 -> 34 and 42 -> 37 logical.
- `baseline-hanging-tabs-table.log` / `implementation-hanging-tabs-table.log`:
  the hanging-tab test's printed table, rederived with the rewritten oracle;
  `hanging-tabs-table.diff` is empty because none of its cases has a space
  ghost edge. Space edges are the new twin test's cases.
- `new-tests-on-baseline.log`: the final tests on baseline production: 3
  runtime and 14 facade tests fail; the cache-admission, icon-ink, read-only
  fill and pending-wheel tests pass there as preservation tests.

## Negative controls (`negative-controls/summary.txt`)

`run1/` is the first run, which recorded only `text.rs` and `text_fields.rs`
hashes; a temporary diagnostic module was later added to and exactly removed
from `tabs.rs`, so all controls were rerun with every owned file's hash
recorded, equal to the final checked sources. Both runs give identical
failing-test lists.

Each mutation was applied by exact replacement, the file touched, focused
tests run (`--no-fail-fast`), the original bytes written back (not copied with
preserved times), touched and hash-verified. Every control log shows
`Compiling pert-ui-runtime`; the post-restore run recompiled and passed.

| Control | Mutation | Failing tests |
|---|---|---|
| N1 | zero-area skip removed | 7: empty-raster, twin, phase-blank (runtime); RTL twin, selectable RTL size, RTL caret, held Space (facade) |
| N2 | `Horizontal::caret` unpinned | 13 facade |
| N3 | old clamp and unpinned caret | 12 facade |
| N3b | old clamp only | 0 (equivalent mutant, finding 3) |
| N4 | old wheel range `size - w` | single-line wheel test |
| N4b | wheel `size + cw - w` for wrapping too | nearly-full wheel (Handled, not Ignored) |
| N5 | zero-area skip before phase admission | phase-cache test (space not admitted) |
| N6 | pin box = wrap only | 5 facade |
| N7 | pin box = layout only | 7 facade |

## Checks (`checks/run-checks.out`, all exit 0, sources unchanged throughout)

| # | Check | Result |
|---|---|---|
| 01 | `cargo fmt --all -- --check` | clean |
| 02-04 | Clippy `-D warnings`, all targets: runtime; facade `--features cpu`; facade default | clean |
| 05 | runtime `--lib --tests` | 371 passed |
| 06 | runtime `--release --lib text` (release boundary) | 58 passed |
| 07 | facade `editing`, `cpu` | 335 passed |
| 08 | facade `editing`, default features | 334 passed (the CPU-only test is gated) |
| 09 | facade `editing`, `cpu`, `--release`, filtered to `hanging_whitespace`, `tab_whitespace`, `text_fields` | 65 passed |
| 10 | facade `controls`, `cpu` (numeric and other single-line editors) | 371 passed |
| 11-12 | runtime and facade doctests | 415 and 121 passed |
| 13-14 | runtime and facade rustdoc, `RUSTDOCFLAGS=-D warnings` | clean |
| 15-16 | `git diff --check`; trailing whitespace in the untracked owned files | clean |
| 17 | `git status --short` | only owned paths and the baseline's untracked files |

`final-state/final-hashes.txt` records the final owned hashes and line deltas.

## Probe attempts

`probe/out/baseline-pre-edit-attempt1.out` and `-attempt2.out` are earlier
baseline runs before the pending-wheel and prose cases were corrected;
`implementation-first-edit.out` is the first implementation run;
`raster-*-pre-edit.out` and `raster-*-final.out` precede the `final2` mask
details. All are kept.
