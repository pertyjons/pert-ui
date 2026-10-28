# Round 3 checkpoint: literal tab presentation

Status: revision 2 proposed, 2026-10-28; independent Claude re-review required
before implementation. Revision 1's tab-to-space substitution was not approved.
The review verified that substitution changes Unicode analysis, bidi field
order, grapheme/word boundaries and explicit caret-offset validity, as well as
ink. Revision 2 preserves those contracts and narrows the fix to coverage.

## Verified finding

`TextService` shapes literal U+0009 through the bundled font to glyph 0 and
rasterizes its missing-glyph box. The private-scope author probe and complete
output are in [tab-rendering](runs/triage-r3/probes/tab-rendering/README.md).
An independent task-6 acceptance CPU capture also shows the box between words
in TextArea. Canonical text and inspection/save bytes correctly retain the tab.

## Chosen responsibility and policy

A literal tab is non-printing: keep its shaped advance, Unicode analysis,
wrapping, bidi segment separation, grapheme/word boundaries and byte offsets,
but emit no coverage for that tab. Width remains the existing font-dependent
shaped advance; no aligned tab stops or code-editor indentation model is
introduced. Canonical input, clipboard/save contents and inspection values are
unchanged. Keyboard Tab traversal is unchanged. Other unsupported characters
keep their fallback/missing-glyph presentation.

The maintainer delegated this design choice with the persistent goal. The
author chooses this narrower policy over the reviewer's acceptable single-space
alternative because preserving the documented caret/grapheme and Unicode
behavior avoids degrading existing text in order to repair its ink. Font-dependent
advance is already observable; the new contract makes its non-printing treatment
explicit. No new public type, builder or application data normalization is needed.
This retires application display-string replacement and the missing-glyph box as
workarounds. It does not advertise a future tab-stop model as implemented.

## Private implementation sketch for review

- Keep the original resolved string in every analysis/build call and cache key.
  Do not substitute any character, including a same-byte-length one.
- Replace the private Parley brush parameter `usize` with a small typed brush
  (`Color(index)` and `HiddenTab`, with the default Color(0)). Retain ordinary
  color indices in resolved spans, raster signatures and mask parts where only
  drawable runs occur. No backend/facade signature exposes Parley or this type.
- In the sole `build_layout` boundary, apply ordinary font/size/brush spans as
  today. Then override only the brush of each exact U+0009 byte range with
  HiddenTab. A tab is already a control/grapheme boundary; test adjacent
  combining marks to ensure the property-only span does not hide them.
  Font, size, line height and all analysis text remain unchanged. Inline-gap
  indices and rich-span byte ranges remain unchanged.
- During coverage preparation, skip HiddenTab runs before rasterizing glyphs;
  never skip glyph 0 based on its ID. Produce no MaskPart for a tab. Repeated
  visible-line/phase rendering reads the same tagged retained layout.
- Raster signatures may omit the hidden run's ink but must retain canvas and
  all visible glyph positions. Icon alignment must still consider a hidden tab's
  font metrics as text where the existing nearest-text rule would, preserving
  existing placement; it must not index the color/icon arrays with HiddenTab.
  Audit every style.brush use. The shared shape boundary covers normal prepare,
  width measurement/fit reuse and ellipsis candidates.
- A cache hit does not reach the builder. Tab-free strings need no replacement
  allocation; the builder's existing per-span work remains. Additional hidden
  ranges are bounded by admitted text length. Do not claim a measured performance
  improvement or real-time safety.

## Required evidence

- Body and Monospace, leading/embedded/repeated tabs, wrapped text, rich font/
  size/color boundaries and adjacent inline gaps: tabs have their shaped advance
  but no ink. Compare visible glyph positions, line breaks, dimensions and
  selection geometry with the pre-fix tab reference, rather than a space string.
- Unsupported private/unassigned characters and U+000B still produce their
  actual fallback/missing-glyph ink. A combining mark after U+0009 is not hidden.
- Cache miss/hit and shaping counts, fit_whole's shape reuse, ellipsis with a
  tab in the shown candidate, default and phase-line raster modes.
- TextSelection canonical boundaries: A-tab-B caret stops, movement, selection
  width and offsets. Pin tab-plus-combining-mark, repeated-tab word movement and
  mixed bidi fields in both paragraph directions against the original Parley
  analysis; do not accept the revision-1 reordering/invalidated caret stop.
- Facade TextArea initial tabs and committed text: navigation, selection,
  backspace, binding/inspection, copy/paste and saved bytes retain U+0009.
  Keyboard Tab moves focus without insertion. Multiline paste retains tabs;
  existing single-line paste normalization still maps them to spaces.
- CPU capture has no ink in the tab's blank region while neighboring glyphs and
  layout remain in place. Reuse unchanged GPU backend evidence; no new native
  text, Windows/macOS or human-input claim from this test.
- Update TextLayout/TextArea/Monospace rustdoc, runtime README's fallback text,
  controls/appearance guides, assets README, contracts and rendering-platform
  together. Explain shaped blank advance and no aligned-tab-stop contract.
  TextSelection's canonical grapheme contract stays unchanged.
- Focused text/editor tests, affected doctests and warning-free rustdoc, required
  workspace gate, CHANGELOG and API generation to confirm no public signature
  diff. No implementation gate is claimed by this proposal.

Reviewers read the source and this decision, author no fixes and do not delegate.
Assess the typed-brush marker against Parley's analysis/style/run slicing, rich
spans, bidi, caret stops, icon alignment, raster caching and every shaping route.
If the marker cannot preserve those invariants, explain the concrete limitation
before implementation rather than silently accepting character substitution.
