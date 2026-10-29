# Round 3 checkpoint: literal tab presentation

Status: revision 4 approved with all recorded corrections below, 2026-10-29. The first
implementation passed focused checks but its fresh Codex review requires
explicit public geometry semantics, mixed-font evidence and qualified rustdoc.
Revision 3 remains the approved shaping algorithm. Revision 1's tab-to-space
substitution was not approved.
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
- Raster signatures omit every HiddenTab run completely (B1); they never
  record hidden glyphs as an ordinary color run. Retain canvas and all visible
  glyph positions. This distinguishes tab ink from an unsupported character
  with the same glyph-0 tuples in the phase-line cache. Icon alignment must still consider a hidden tab's
  font metrics as text where the existing nearest-text rule would, preserving
  existing placement; it must not index the color/icon arrays with HiddenTab.
  Audit every style.brush use. The shared shape boundary covers normal prepare,
  width measurement/fit reuse and ellipsis candidates.
- A cache hit does not reach the builder. Tab-free strings need no replacement
  allocation. Merge consecutive tabs into ascending overrides after ordinary
  spans. Keep markers out of Resolved so cached_plain still matches. Extra
  ranges create style-table entries/font-metric queries and glyph sub-runs;
  drawable sub-runs can add scaler builds and raster-signature allocations.
  Hidden sub-runs are skipped before scaler construction. Record this per-tab
  work, not a measured performance improvement or real-time safety claim (B3).
  Tie a conservative style bound, 2 * MAX_TEXT_BYTES + MAX_SEGMENTS + 4,
  to u16::MAX with a compile-time assertion; the existing 16 KiB/segment
  admission limits keep this below Parley's style-table limit.

## Accepted conditional review decisions

The [revision-2 review](runs/review-r3/tab-presentation-design-review-r2.md)
is accepted in full. B1 and B3 are specified above. For B2, preserve a registered
font's mixed tab/non-tab atom instead of hiding neighboring text:

1. Build the tagged layout on the original string, then walk logical grapheme
   clusters grouped into shaped atoms using is_ligature_start/continuation.
   A tab-only atom is hideable. An atom containing both tab and non-tab text
   must keep its existing glyph coverage; it cannot be separated faithfully.
2. Collect the exact tab-byte ranges in those mixed atoms. If none exist, keep
   this single shaped layout (the bundled-font path). If any exist, rebuild
   once on the same original text with ordinary spans but without HiddenTab
   overrides on those particular tabs. Other pure-tab atoms stay hidden.
   Brush-only properties never change shaping/analysis, so the atom partition
   and fallback decision cannot change on that second pass; no retry loop.
3. Keep original font/size/color for the whole mixed atom, canonical text,
   advances and caret/bidi/wrap analysis. Document this exception: a custom font
   that ligates a tab with printable text keeps that inseparable glyph's ink.
   Prefer showing that glyph to erasing a neighboring character.
4. The scan is linear in shaped clusters and text ranges; it does not rescan
   the whole run for every GlyphRun. Only the unusual mixed-atom font path
   pays a second shape. No normal-shaping-count or timing claim includes it.
   Test a registered-font mixed tab ligature with real shaping and assert the
   printable glyph remains; test tab-only ligatures separately.

The required pre-fix test reference skips overrides in the same shape boundary.
Compare line/selection geometry exactly where stable and positioned glyphs with
small rounding tolerance. Add phase-cache collision tests for A-tab-B versus
A-U+E000-B in both cache orders, tab-only ink=None/caret tests, bidi/icon
alignment, adjacent combining marks, worst admitted alternating tabs with rich
spans and a measured TextArea timing observation. These extend the evidence
list below; none is claimed to pass before implementation.

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

## Accepted revision-3 implementation requirements

The [focused recheck](runs/review-r3/tab-presentation-design-review-r3.md)
approves the algorithm. Walk lines, their runs and logical clusters exactly
once, grouping atoms when a cluster is not a ligature continuation. Atoms never
cross a run/line slice, even under emergency wrapping. Skip this scan entirely
without tabs. Sort mixed-tab byte ranges before the second ascending merge,
splitting a consecutive override around every retained mixed tab. Count actual
Parley builds: one for bundled fonts, exactly two for a mixed-atom fallback.
The second layout is final; no per-GlyphRun scan or retry loop.

Each range push can shift later style spans: rich text can cost
O(overrides * later spans), in addition to style metric queries and sub-run
allocations. The rare fallback repeats shape, break and align once. The worst
admitted rich/tab test records an observation, not a general performance bound.
Commit a tiny self-owned synthetic font fixture with U+0009 mappings and real
GSUB mixed/pure tab ligatures, source/provenance and license; no build-time
generation or installed-font dependency. Verify mixed prefix/suffix, pure-tab
ligature, three-tabs-plus-X range splitting, and a test-only final-layout
re-walk. Original style/color and ordinary neighboring ink must survive.


## Revision 4: visible ink bounds and public coordinates

The [independent implementation review](runs/review-r3/tab-implementation-codex-review-r1.md)
found that removing hanging tab ink can shrink the raster canvas and translate
its origin, especially in narrow wrapped RTL text. The implementation's tests
normalized away that origin before comparison. That establishes preservation
of Parley coordinates, not preservation of public caret positions at a fixed
paint origin. The review is accepted; those tests alone are insufficient.

The proposed contract is to derive the canvas from Parley width()/height() (excluding hanging whitespace) and actual
visible ink using the existing bounds calculation, with hidden tabs excluded
from ink. We will not retain invisible missing-glyph boxes as geometry. Shaped
advances, Unicode analysis, line breaking, and caret/selection positions in
Parley coordinates remain unchanged. Hanging-whitespace geometry may fall
outside size(), as already happens for hanging spaces. Public size and origin may change where old
tab ink overhung that layout; consequently public caret and selection rectangles
may translate by the resulting origin delta. Hit testing must use the same
translation. This is an explicit behavioral change, not a claim that all public
positions remain identical. Normal cases with no tab-defined overhang retain
size, origin and public coordinates. No canonical offset is coerced or removed.

Before adoption, record concrete pre-fix versus repaired values for LTR and RTL
wrapped hanging tabs: size, origin, line bounds, public caret rectangles and
selection rectangles. Test public hit testing at identical fixed points in both
layouts, without origin normalization. Assert the expected coordinates and
canonical offsets separately, including the intentional translation where it
occurs; do not assert equal outcomes where the coordinate system changed.
Also test the corresponding translated point to establish caret/hit coherence.
Use the pre-fix test-only shaping reference, not a space replacement.

Add narrow facade TextArea cases for non-hanging positions, which must reveal
inside the viewport, and deliberately hanging LineEnd/selection positions,
which must match a hanging-space twin's clipping and scroll offset. Record
pre-fix values from an actual base-commit run; the facade cannot use the private
cfg(test) reference. Verify the suspected whitespace-wide inability to reveal
hanging carets with spaces and tabs; if reproduced, report it to the integrator
for TODO.md rather than repairing editor scrolling in this tab-ink patch.
Do not require the impossible all-positions-in-viewport assertion or select
only non-hanging positions to conceal the limitation.

Complete the previously required mixed-font evidence in one rich layout using
Body, Monospace and the registered synthetic fixture, with tabs at font
boundaries, wrapping and phase rendering. Include an adjacent icon whose nearest
text metrics come from a hidden tab. Compare actual chosen fonts, advances,
visible coverage and icon placement with the pre-fix reference, and prove an
incorrect tab-font override fails the test. Qualify Monospace and TextArea's
blank-tab wording with the registered-font mixed-atom exception and link the
TextLayout contract. Integrating documentation and changelog must describe the
public origin/extent change as well as preserved logical behavior.

A fresh read-only review must approve this policy before implementation repairs.
The independent reviewer then rechecks the completed P1/P2 evidence before the
patch is integrated. Earlier runs and negative-control records stay immutable.


### Accepted revision-4 review requirements

The [revision-4 review](runs/review-r3/tab-presentation-design-review-r4.md)
approves this policy conditional on its four detailed gaps being adopted. They
are adopted in full, including the enumerated LTR/RTL hanging/partly hanging,
forced-break/unbounded and mixed-atom cases; actual hanging classification;
nonzero LTR width and RTL origin deltas; independent expected visible-ink union;
unchanged vertical origin/baseline; scale 1 plus fractional scale; raw public
caret/selection/hit/word-at-point assertions for its four point categories with
canonical offset and affinity checks; ellipsis candidate comparison; and the
recommended public SelectableText RTL selection case. A same-layout round trip
is only a regression guard, not editor evidence. Tab-defined overhang means an
old tab glyph alone defined a canvas edge beyond both logical box and other ink.

The review's exact mixed-font case list, per-tab font/size/coordinates/advance
assertions, discriminating hidden-tab nearest-icon metrics, two additional
negative controls, phase/cache-order checks and repaired-origin placement are
required. TextLayout::size and TextSelection caret/rectangles/at_point rustdoc
must explain hanging-whitespace geometry outside size. Documentation must
name intentional LTR extent shrink and RTL x translation as behavioral changes.
No ghost geometry is introduced. The suspected hanging-whitespace editor issue
must be reproduced before a verified TODO finding is recorded. This approval
permits scoped repairs; fresh independent evidence recheck still precedes adoption.
