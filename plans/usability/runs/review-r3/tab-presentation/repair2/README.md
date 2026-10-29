# Round 3 literal tabs: revision-4 repair evidence

Implementation author's evidence for repairing the uncommitted tab-ink patch in
`/home/per/pert-ui-usability/r3-text-worktree` (HEAD `3bbfb81`, nothing
committed or staged) against revision 4 of
`plans/usability/round-3-design-tabs-whitespace.md`, the Codex implementation
review r1 and the Claude design review r4. The shaping algorithm is unchanged;
this repair adds tests, rustdoc and records. This is not an independent review.
No native, GPU, Windows/macOS or human-input evidence was produced.

The first implementation's records in `../r3-text-implementation-evidence/` are
untouched: `final-state/state.log` shows all 69 files byte-identical to the
hashes taken before this repair (`pre-repair/previous-evidence-files.sha256`).

## Layout

- `pre-repair/`: worktree state, owned-file hashes and copies of the two
  untracked test files as found, before any edit.
- `base-source.log`: the base fixture. `git archive 3bbfb81` (commit
  `3bbfb813a8c02c417d68386b0c5547b4b8139580`, version 0.3.89) was extracted to
  `/home/per/pert-ui-usability/r3-text-implementation2-base-3bbfb81`; this
  writes no git metadata. It records the archive hash, the tree hash and the
  base `text.rs` hash, which equals `git show 3bbfb81:…`. No worktree was
  added and base, main and kits were not modified.
- `facade-probe/`: one public-facade program, `src/main.rs`, built unchanged
  against the base extraction (`base/`, its own target dir) and the repaired
  worktree (`repaired/`). `attempt1/` and `attempt2/` are earlier runs:
  attempt 1 used 30–32 px inner widths that overflowed for an unrelated reason
  and an RTL arrow direction that left the line. `final/` is the run on the
  final source, with `sources.txt` hashes and `base-vs-repaired.diff`.
- `negative-controls/`: `run.py`, the final run's per-control logs,
  `summary.txt` and `run.out`. `attempt1/` is the first run, on an earlier
  `tabs.rs`. There, N8a was detected first by a baseline assertion, so the
  per-tab font checks were moved ahead of the layout comparison and the whole
  set rerun.
- `checks/`: the final targeted checks (`run-checks.py`), with command header,
  exit status and full output each; `run-checks.out` summarizes them.
- `final-state/`: final worktree state and hashes, the full tracked and
  untracked diffs, and the repair-only diffs against `pre-repair/`.

## Modified paths (this repair)

- `crates/pert-ui-runtime/src/text/tabs.rs`: tests only. `assert_matches_reference`
  now asserts the exact policy canvas, and four tests are new or extended.
- `crates/pert-ui-runtime/src/text.rs`: rustdoc only for `TextStyle::Monospace`,
  `TextLayout` and `TextLayout::size`.
- `crates/pert-ui-runtime/src/text_fields.rs`: TextArea rustdoc only.
- `crates/pert-ui-runtime/src/text_selection.rs`: rustdoc only, for `at_point`,
  `word_at_point`, `caret` and `rectangles`. Check 18 proves no non-doc line
  changed.
- `crates/pert-ui/tests/editing/tab_whitespace.rs`: four new facade tests.

Unchanged: the algorithm in `text.rs`/`tabs.rs`, the fixture assets,
`editing/main.rs`, manifests, lockfile, versions, CHANGELOG, guides, API
listings.

## Requirement to evidence

All values below are from `checks/06` (debug, `--nocapture`) unless named.

### Exact policy canvas (P1, review r4 gap 3)

`expected_canvas` computes the canvas independently of the implementation.
It is Parley's `width()` × `height()` box joined with swash raster bounds of
every glyph of the pre-fix reference that is outside an atom of tabs alone.
`assert_policy_canvas` then asserts:

- the repaired size and origin are exactly that canvas;
- the vertical extent, `origin[1]`, height and `first_baseline` equal the
  reference's.

It returns whether old tab ink alone defined an edge. Every reference
comparison now uses it, replacing the earlier `<=` width check. That covers
every existing case: 13 texts × Body/Monospace × 3 scales × 4 widths, the rich,
fixture, fit and ellipsis cases.

`hanging_tabs_shrink_the_extent_and_translate_right_to_left_public_coordinates`
runs at scale 1.0 and 1.25. Each case asserts its hanging classification from
Parley line metrics (`hanging_advance > 0` and a tab cluster in the hanging
range) and its tab-defined overhang. Each scale must show a nonzero LTR width
delta and a nonzero RTL `origin[0]` delta.

| Case (width) | Scale | Size before → after | Origin x before → after |
|---|---|---|---|
| LTR wrapped, hanging tab mid-line, Mono (38) | 1 | 41 → 34 | 0 → 0 |
| | 1.25 | 40.8 → 33.6 | 0 → 0 |
| LTR prose, Body (70) | 1 | 77 → 70 | 0 → 0 |
| | 1.25 | 76.8 → 70.4 | 0 → 0 |
| RTL wrapped, hanging tab mid-line, Arabic Mono (38) | 1 | 40 → 37 | 3 → 0 |
| | 1.25 | 40 → 36.8 | 3.2 → 0 |
| RTL tab partly hanging at text end (38) | 1 | 41 → 38 | 3 → 0 |
| | 1.25 | 41.6 → 38.4 | 3.2 → 0 |
| Tab before explicit newline (220); unbounded LTR/RTL ending in a tab; fixture mixed atom ending a wrapped line (34) | both | unchanged | unchanged |

Heights, `origin[1]` and baselines are unchanged in every case.

The test also asserts:

- **Carets and selections:** every caret at every stop, every selection
  between stops, and `LineEnd`/`LineStart`/`Down` results equal the
  reference's rectangles minus the old origin plus the new one, within 1e-3.
  Nothing is normalized away. For example, RTL line 0 `LineEnd` (9,
  Upstream): caret x −1.0 → −4.0; selection `[-1.0, 0, 42.0, 20]` →
  `[-4.0, 0, 42.0, 20]`.
- **Hit testing, four point categories:**
  - (a) over a visible glyph;
  - (b) between the old and new origin;
  - (c) over the hanging tab: negative public x in RTL, beyond `size()` in LTR;
  - (d) far left, far right, above and below.

  The expected offset and affinity come from the repaired layout's own
  cluster offsets, using Parley's side rule and the independently expected
  origin. The pre-fix reference must give the same caret and `word_at_point`
  at the translated point. At the same fixed point, the reference differs:
  RTL (a) at x 6.5 gives (8, Upstream) repaired vs (8, Downstream) pre-fix;
  (b) at x 1.5 gives (8, Downstream) vs (9, Upstream).
- **Phase rendering:** phase-line coverage at four phases equals the
  reference's in Parley coordinates, apart from tab regions, so it is placed
  against the repaired origin. There is no tab ink.
- **Mixed atom:** it keeps the `X\t` ligature glyph. Parley does not hang a
  tab that continues a ligature, so that case does not hang.

### Ellipsis

In `cached_measured_and_ellipsized_tabs_stay_hidden`, `fit::candidate` drops
whitespace before "…", so no shown candidate ends at a tab. Two cases are
asserted to choose alike with equal sizes:

- At the width of `Name\t…`, both choose `Name…`, size 51.
- `Name\t` at exactly its width fits whole in both, size 48.

The earlier Ellipsis/MiddleEllipsis cases remain.

### Mixed fonts and icon (P2, review r4 gap 4)

`mixed_fonts_keep_each_tab_s_font_size_and_advance_and_icons_use_hidden_tab_metrics`
uses one rich layout of Body, Monospace and the registered fixture. It runs at
scales 1 and 1.25 and widths None, 64 and 120. Its tabs are:

| Byte | Position | Font, size |
|---|---|---|
| 2 | last byte of a Body span | Body 14 |
| 3 | first byte of a Monospace span (2\|3 straddles Body\|Mono) | Mono 14 |
| 8 | a fixture span's only content, the icon's nearest text | fixture 26 |
| 15 / 16 | Body\|fixture straddle | Body / fixture |
| 21–22 | fixture pure tab-tab ligature beside Body | fixture |
| 27 | fixture `X\t` mixed atom, which ends line 2 at width 64 | fixture |
| 33 | a Monospace tab after Body, hanging at a font boundary | Mono |

- **Per tab:** each tab's actual font identity, size, normalized coordinates,
  advance, line and x equal the reference's. Its font is the role or fixture
  font expected, and its brush is HiddenTab except byte 27.
- **Icon:** its alignment equals the reference's and is the hidden size-26
  tab's center: 16.9 at scale 1, 20.625 at 1.25. The nearest visible run's
  center would be 22.43 / 27.54, a difference of at least 1 px. Icon coverage
  equals the reference pixel for pixel.
- **Phase rendering:** four phases in both cache orders against a twin with
  U+E000 for each tab. Coverage is order-independent, has no tab ink, and
  equals the reference outside tab regions.

### Negative controls (final source, `negative-controls/summary.txt`)

All ten were detected, and the sources were restored with hashes verified:
`text.rs` `dce308f5…`, `tabs.rs` `2e405639…`.

- **N1–N6** repeat the first implementation's controls. The new mixed-font
  test also catches N2, N3, N5 and N6.
- **N7, nearest-run search skips hidden tabs:** the mixed-font test fails at
  the icon alignment assertion, and the rich test fails too.
- **N8a, the override also sets the first span's family:** fails at the
  per-tab font assertion (Monospace tab at byte 3 got font 23).
- **N8b, the override also sets the default size:** fails at the per-tab size
  assertion (byte 8 at 14 instead of 26).
- **N9, ghost geometry (hidden tabs keep raster bounds):** the hanging test
  fails on the policy horizontal canvas (41 vs 34), and three more tests fail.

### Facade, through the public API only (P1 TextArea and SelectableText)

The base values come from the real base build (`facade-probe/final/base.out`),
not a test flag. The facade tests in `tab_whitespace.rs` assert the repaired
values.

- **`a_narrow_text_area_treats_a_hanging_tab_as_a_hanging_space`** (LTR,
  62 px viewport, inner 38):
  - Interior (2), before the hanging tab (4) and text end (14) carets are
    painted inside the inner area.
  - LineEnd after the hanging tab (5) and the selection 0..5 are clipped.
  - `scroll_x` is 0 at every step, including after a 40 px sideways scroll.
  - Every step equals the hanging-space twin.
  - Base, tab: End gives `scroll_x` 4 with the caret still clipped, and a
    horizontal scroll range of 3–4 px exists (`scroll_x` 3 after a sideways
    scroll).
- **`typing_a_tab_at_a_full_line_end_matches_typing_a_space`** (real
  `text_input`): `aaaa` + tab + `b` matches `aaaa` + space + `b`.
  - After the tab: selection 5..5, caret clipped.
  - After `b`: caret `[20.4, 26, 1, 20]`, `scroll_x` 0.
  - Base, tab: `scroll_x` 4 after the tab and after `b`, so the whole text sat
    4 px left, with the caret at 16.4.
- **`a_narrow_right_to_left_text_area_keeps_its_hanging_tab_in_the_extent_of_a_space`**
  (Arabic, ArrowLeft moves forward):
  - 4..4 and 8..8 are painted (8..8 at x 16.4).
  - LineEnd 9..9 is clipped.
  - The 0..9 selection fill is `[12, 6, 38, 20]`.
  - The text-end caret is at x 33.2; base 36.2, the −3 translation.
  - No sideways scroll.
- **`right_to_left_selectable_text_selects_at_fixed_points_after_the_origin_moved`:**
  - Bounds are `[0, 0, 37, 59]`; base 38, its allocation, for a pre-fix
    layout of width 40.
  - Drags from x 35.5 to 1.5, 10, 20 and 34 select 0..8, 0..6, 0..4 and 0..0.
    At the same window points base selects 2..9, 2..8, 2..4 and 2..2.
  - Each drag equals `TextSelection::at_point` on the public layout. This is
    a same-layout regression guard, not editor evidence.
- **Unchanged:** the earlier facade tests (canonical bytes, Tab traversal, CPU
  ink, 300-row timing) and reused paste tests (`checks/11`).

## Verified editor limitation, for the integrator's TODO.md

Reproduced with real public input (keys, `text_input`) in both the base and
repaired builds (`facade-probe/final/*.out`):

- **The defect:** TextArea cannot reveal a caret at whitespace hanging past a
  wrapped line's end. `position` clamps `scroll_x` to
  `[0, size.w + caret_w - inner_w]` and paints clipped to `inner`.
- **LTR spaces, base and repaired:** typing `aaaa` then a space at a full
  38 px line leaves the caret clipped (`scroll_x` 1). End on a hanging line
  clips the caret too.
- **LTR tabs:** the same in the repaired build. In the base build the tab box
  added 4 px of scroll range, which shifted the text but still did not reveal
  the caret.
- **RTL:** the hanging-tab LineEnd caret is clipped in both builds.
- **Same family:** in wrapped RTL text the line-start caret at offset 0 sits
  at the inner right edge and is clipped by its own 1 px width. Parley
  right-aligns the line in the full width while `width()` excludes the
  hanging advance. This holds for base and repaired, tab and space.

This is whitespace-wide and outside this patch; no editor scrolling was
changed.

## Actual gaps and findings

- **The RTL hanging-space twin is not geometrically identical.** The bundled
  space glyph rasterizes to a 0×0 image whose pen point still joins canvas and
  ink bounds in `rasterize_glyphs`. A space hanging at x −4 therefore keeps
  `origin[0]` 5 and a 4–5 px scroll range, and its LineEnd caret is painted
  at the inner edge, where the hidden tab's is clipped. This is pre-existing
  (base and repaired are equal for spaces), and LTR twins are identical. The
  facade tests therefore assert the RTL tab values directly, not equality with
  the space twin. The integrator decides whether empty rasters should count.
- **TextArea rustdoc** says it "scrolls internally to reveal the caret". That
  does not hold at hanging whitespace; see the limitation above. It was left
  unchanged pending the TODO decision.
- **Timing:** one observation each, not bounds (`checks/07`, release).

  | Case | Repaired | Pre-fix reference |
  |---|---|---|
  | Worst admitted, plain | 41.4 ms | 38.2 ms |
  | Worst admitted, 1024 spans | 68.0 ms | 37.2 ms |

  16,385 styles against the bound of 33,796. The TextArea 300-row check here
  ran in debug only (`checks/10`, 234 ms); the earlier release observation
  is in the first evidence set.
- **Left for the integrator:** runtime README, controls/appearance guides,
  assets README, contracts and rendering-platform text, and the CHANGELOG
  "Changed" entry. It must cover the LTR extent shrink and RTL x translation
  for hanging tabs only; wrapped RTL text no longer shifting right; wrapped
  widgets and TextArea no longer measuring wider or scrolling sideways because
  of tab ink; and preserved canonical bytes, Unicode analysis, advances and
  Parley-coordinate carets. Also the workspace and API-listing gates, the
  TODO entry and the fresh independent review.
