# Root repair after independent implementation review 1

2026-10-29. Raw implementation1 evidence and review remain unchanged. This
supplement supersedes only the inaccurate prose identified in the review.
Production logic and test assertions are unchanged; source-repair.json records
the three changed files before/after. Names/comments and rustdoc now state the
bounded rounding residual and conditional zero-area/equivalent-shaping rule.

Root explicitly adopts the observed consequences in MAIN design revision4:
max(layout.width, wrap) including outward physical rounding; dimension-only
zero-area skip without alpha scanning; and fractional-edge sampling changes
when mask extents change. The approved production formula is retained.

Corrections to historical author prose:

- Held-space offset can be0 →0.4 →0 at wrap38, scale1.25, for whitespace and
  then the wrapping letter. Repeated whitespace retains0.4. The old claim of
  universally fixed prose/held-space position was too broad. Nearly-full
  genuinely nonoverflowing wrapped lines still propagate no-op wheel input.
- Same advance alone is insufficient for space/tab geometry equality. The
  space must have a zero-area raster and the advances, line breaking and other
  glyphs must match. Positive-area custom-font ink remains eligible.
- Default whitespace-only text has ink=None and no blank default mask. A
  positive-area transparent1×1 raster can remain in the phase path, changing
  phase-mask extents/partitions without changing default size/origin.
- findings/swash-empty-outline-rasters.txt incorrectly labels its long1×1
  output block “default-path offsets”. That block is from the last temporary
  variant, which uses the phase path's y offset. Its concluding distinction
  and earlier default-path observations are the intended reading: default
  fractional-x/integral-y gives1×0 or0×0; fractional-y phase can give1×1 zero
  coverage. The original mislabeled artifact is preserved rather than edited.
- The report's selectable RTL space twin baseline “38→37” is incorrect. Actual
  stored output is width42→37, first raw caret x43→38 at scale1, and width
  40.8→36.8, first caret x42→38 at1.25. Use the raw probe, not the summary typo.
- Actual frame comparison is60/72 identical. “All bundled-icon cases” was too
  broad: icon-only and leading-icon-label frames match; inline-icons quarter-
  origin text frames can differ. Icon centering is unchanged. The RTL-leading-
  space quarter-origin pair changes11 pixels in one column, at most79/255,
  with mask width192→190. Exact frame identity requires unchanged mask
  partition, extent, destination and clipping; positive glyph placement alone
  does not imply it. Root accepts this natural-mask sampling consequence.
- N3b remains an equivalent mutant under the retained pin and wheel range.
  Its survival is disclosed; the combined old clamp/raw caret control fails.

Four actual focused commands in outcomes.json all exit0: formatting, the
CPU hanging-whitespace tests, all runtime doctests and warning-free runtime
rustdoc. They use the author's existing isolated target directory. No full
workspace, API or native evidence is inferred. A fresh read-only Codex review
must assess these corrections before integration; original source review1's
request-changes verdict is retained.
