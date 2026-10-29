# Round 3 checkpoint: empty raster bounds and hanging-caret reveal

Status: revision 4 adopts the independent implementation review corrections,
2026-10-29. Revision3 was independently reviewed before implementation; its
production policy is unchanged. Independent repair recheck remains required. These are newly verified follow-ups from the tab-coverage
repair and will be handled separately within the usability goal. The tab patch
keeps them explicitly documented until this checkpoint is verified.

## Verified findings and scope

The real base/repaired runs in
[tab repair evidence](runs/review-r3/tab-presentation/repair2/README.md) reproduce
both findings. Base is the actual archived 3bbfb813 source; repaired is the
isolated tab patch. No space substitution or private cfg reference was used
for the public TextArea runs.

1. TextService::rasterize_glyphs unions a 0-by-0 space raster's pen point into
   the canvas. In wrapped RTL text, an empty space therefore extends left bounds
   and changes public origin/extent, unlike the nonprinting tab. No coverage
   exists at that point. Source confirms there is no zero-area check before
   PlacedGlyph contributes bounds. This predates the tab change.
2. Editor::position tries to reveal the caret, then clamps horizontal scrolling
   to zero through layout width plus caret width minus viewport width. Hanging
   caret positions outside that box are clipped: typing a space at a full LTR
   line or pressing End makes the caret disappear in both base and repaired
   runs. RTL hanging tabs and the one-pixel right-edge caret also reproduce.
   The wheel handler independently clamps to zero/layout width, compounding
   the mismatch. This is a general whitespace/caret problem, not lost text.

Existing TEXT-01 large-buffer capacity and unrelated rendering TODOs remain out
of scope. No normalization, tab stops, Unicode/line-breaking change, new public
API, persisted field/schema or renderer/platform dependency is introduced.

## Proposed bounds policy

After normal phase-glyph cache lookup/admission, skip a raster whose width or
height is zero before icon alignment reference rendering, PlacedGlyph
construction and bounds union, in both default and phase paths. Retain cached
zero-byte entries so repeated spaces do not invoke Swash again on every line
cache miss.
This is a zero-area test, not glyph-id-zero suppression or an alpha-pixel scan.
Visible missing-glyph boxes remain drawn. Keep the original analysis string,
font metrics, advance and retained shaped layout, including icon nearest-text
metrics. Layout's box still supplies its normal extent, while only nonzero
raster rectangles can expand it. Document the resulting public origin/size
changes for hanging spaces and empty custom glyphs; public caret/selection/hit
coordinates use that same origin. No claim of preserved absolute coordinates
where a ghost raster alone set the old edge.

Keep phase cache/signature semantics correct: a zero-area glyph produces no
mask or extent even on cache hits; visible glyph identity/position still keys
coverage. No per-pixel emptiness search or timing improvement is claimed.

## Proposed wrapped-caret presentation policy

The [first review](runs/review-r3/whitespace-design-review-r1.md) rejects the
negative-offset policy because soft-wrapped prose would move sideways whenever
space hangs, and held Space would slide every line indefinitely. Accept the
reviewer's replacement: pin the displayed caret's x in wrapping TextArea to
its horizontal layout box. That box is max(layout.size.width, wrap width), with
caret width reserved at the end. One private helper returns this presentation
rectangle for caret paint, reveal and IME/input-state anchoring. Canonical text,
selection offsets, TextSelection geometry and Parley's hit/word policy stay
unchanged. A hit at/beyond a line edge retains the original shaping contract;
clicking a pinned presentation caret need not recover its exact canonical
hanging offset. Document the distinction explicitly.

Always clamp the wrapping caret x to [0, max(box.width-caret.width, 0)].
When the viewport is narrower than the caret, preserve the reveal loop's
existing end-wins placement and allow unavoidable clipping; do not force x=0
merely because the viewport is narrow.
Only wrapping editors pin: single-line, numeric and draft editors cannot hang
and are regression cases. Unbreakable text overflow retains scrolling.

Share one nonnegative horizontal interval between position and wheel. Its
maximum is max(extent - inner.width, 0), with extent=max(layout.width, wrap width)
for wrapping editors and layout.width+caret.width for single-line/numeric/draft
editors. Wrapping editors need no extra caret-width allowance after pinning;
a nearly full nonoverflowing wrapped line must keep wheel propagation.
Store that interval with Prepared when layout/painting prepare it; a wheel
must not recompute using a stale prepared layout and a newer selection. Use the
same prepared geometry and display/preedit mapping as current caret painting.
No negative scroll_x or persisted/inspection sign change is introduced.

Arrange already forces reveal even when unfocused: preserve and document that
behavior rather than promise retained offsets through relayout. A wheel before
the next paint can cancel pending edit reveal as today; pin this with evidence.
Manual scrolling may move the caret out of view until the next reveal request.
Keep vertical scroll/scrollbar policy unchanged. No polling, whole-root rebuild
or extra shape merely for caret movement.

## Required evidence

- Public base/repaired observations of the known LTR/RTL examples, at scale 1
  and a fractional scale, with literal bytes/selection unchanged. Assert actual
  rendered caret fill and clipping/IME anchor, not just a private scroll number.
- LTR line End, typing the hanging space/tab, RTL line End and right-edge start;
  extend a selection through that whitespace. After requested reveal, the pinned
  caret is inside the effective viewport when physically possible. Hit testing
  at/beyond the line edge retains the canonical shaping contract; no inverse
  mapping from pinned caret to hanging offset is promised.
- Narrow and zero-area allocations, reflow/resize, wrap/font/scale changes,
  focus loss/regain and readonly field. Single-line and embedded numeric/draft
  editors keep their existing alignment, confirmation/cancellation and Tab/IME
  behavior. No callback timing, accepted write or undo change.
- Wheel at both nonnegative ends, then a key/edit restoring
  reveal, with no-op wheel propagation preserved. Vertical scrollbar and
  ordinary long-text reveal tests remain passing.
- Empty raster changes: LTR/RTL leading/trailing/wrapped spaces, tab-only and
  mixed font/ligature cases, ordinary unsupported-character ink, inline icons,
  phase cache miss/hit and nonzero phases. Assert size/origin against the
  independent union excluding zero-area rasters and record raw public deltas.
- Negative controls restoring empty-raster bounds and the old scroll clamp
  must fail their specific visible geometry/caret assertions, with exact source
  restoration and rebuild evidence. Keep earlier evidence immutable.
- Update TextLayout/size/TextArea/TextSelection rustdoc, controls/appearance,
  runtime/assets README, contracts, rendering notes and CHANGELOG to replace
  the temporary limitations accurately. Run focused contracts, strict docs,
  required workspace/API gate and independent implementation review. Native
  evidence must describe only the actual selected scenario/platform run.

Reviewer reads the source/evidence and declared policy, authors no fixes and
uses no other agents. Approve this design or identify concrete counterexamples
before implementation; do not infer successful checks from the proposal.


## Accepted review-1 evidence corrections

All review-1 requirements are adopted. The checkpoint base is the approved
uncommitted tab patch, captured by source hashes; 3bbfb813 alone is only the
older comparison, not this fix's implementation base. Update the independent
tab-canvas oracles to exclude zero-area placements explicitly and rederive raw
delta tables. RTL tab/space twin divergence becomes an equality assertion.
Whitespace-only zero-area glyphs have ink=None and no default blank mask;
positive-area transparent phase rasters can remain. Brush chunks/part counts
can change. Record those deltas and require pixel-identical CPU output where
mask partition, extent, destination and clipping are unchanged. Where ghost-edge removal deliberately changes
origin/extent, record raw fixed-coordinate before/after differences rather than
normalize them away or claim whole-frame pixel identity. Bundled icon centering
through ink stays unchanged; test it.

Add public prose-jiggle and held-Space controls: hanging spaces/tabs keep the
caret inside the viewport, with at most the bounded outward-rounding residual
described below rather than progressively increasing offsets. Readonly visible proof uses
selection fill, since no caret is painted. Default/phase cache hit/miss tests
include a mutation that skips empty rasters before cache lookup, catching repeat
Swash work via cache-hit counters. Old bounds, old clamp/pin and wheel +cw
mismatch mutations must fail their intended visible/geometry assertions with
exact restore/rebuild proof. Include tiny/zero viewports without invalid clamp
intervals, preedit freshness and pending edit/wheel cancellation. No claimed
successful check precedes implementation. Fresh focused design recheck follows.


## Adopted revision-2 approval conditions

Both conditions of the [second design review](runs/review-r3/whitespace-design-review-r2.md)
are adopted above before implementation. The exact base record is
[base/baseline.json](runs/review-r3/whitespace/base/baseline.json): 17c6c3d plus
the approved tab patch, with text bytes equal to main's working 0.3.92. Old
repair2 runs remain the intentional historical bug evidence; the new base is
this approved-tab snapshot, not the older comparison commit.

The pin's presentation consequences are explicit: consecutive hanging caret
stops can share one drawn x; selection fill and composition underline remain
unpinned and clip normally. Hit testing retains Parley's raw contract, not an
inverse of the pinned caret. Input-state anchoring keeps the existing prepared-
layout/current-selection freshness and final viewport clamp; the shared pure
pin helper adds no new freshness guarantee. The wheel uses its stored Prepared
interval, never a new selection against an old layout.

Cache-order proof is phase-specific: mutate skipping before cache admission,
not the impossible pre-lookup area test. Assert zero-byte phase keys survive
admission and reuse on another phase-line miss without new entries (or use a
private cfg(test) render count); there is no production cache-hit counter.
Nearly full wrapped-line no-op wheel, tiny viewport end-wins, and the stated
presentation consequences are required evidence. All approval conditions are
now concrete; implementation may proceed with independent review afterwards.

## Adopted implementation-review consequences (revision 4)

The [first independent implementation review](runs/review-r3/whitespace/independent-review-r1.md)
confirms the production logic matches revision3, and requires precise claims
and explicit acceptance of measured presentation consequences. Root adopts:

- Keep max(layout.width, wrap) and outward physical-pixel rounding. At wrap38,
  scale1.25 the measured box is38.4: the observed scroll sequence is0 →0.4 →0
  for typing whitespace then the wrapping letter. Repeated whitespace retains
  the bounded residual rather than increasing it. Review2's inference that a
  nonoverflowing wrapped layout always has box=wrap was incorrect. No negative
  scrolling or changed canvas formula is introduced.
- The skip is dimensions-only. Positive-area custom-font space ink remains;
  equal advance alone does not imply space/tab geometry equality. Equivalence
  requires a zero-area space, matching shaping/line breaking and other glyphs.
  A phase raster can be1×1 with zero coverage; it remains eligible for a blank
  phase mask, affecting its extent/partition without changing default layout
  size/origin. No alpha scan is added.
- Accept fractional-edge resampling consequences when mask extents change.
  Stored frame pairs are60/72 identical. The RTL-leading-space quarter-origin
  pair changes11 pixels in one column, at most79/255 per channel, as its mask
  narrows192→190. Positive glyph placements remain; exact pixels require equal
  partition, extent, destination and clipping. Icon-only and leading-icon-label
  cases match; inline-icon text frames can differ when their masks narrow.
  Bundled icon centering is preserved; whole-frame identity is not promised.

Historical author reports and logs stay immutable. The repair record corrects
one baseline-width description (actual42→37 at scale1,40.8→36.8 at1.25), an
incorrect default/phase label in the Swash observation, and overly broad icon
pixel wording. Rustdoc/test labels are corrected and focused checks plus a fresh
independent recheck precede integration. These are qualifications of the
approved natural-canvas policy, not a new renderer or shaping implementation.
