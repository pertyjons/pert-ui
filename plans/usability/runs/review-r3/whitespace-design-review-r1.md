**Verdict: not approved as written.** The empty-raster part can be approved once the corrections below are made. The horizontal-reveal policy is blocked by concrete counterexamples. I've given a replacement policy that I'd approve with the listed evidence. This is design review only: I made no edits, builds or runs, and I don't approve anything native, workspace-level or overall.

## Facts I checked in the source

- **Single-line, numeric and draft editors can't hang.** Parley 0.12 defaults to `WhiteSpaceCollapse::Preserve` (`style/mod.rs:38`). Under that rule, trailing whitespace hangs only by the amount it overflows the line (`line_break.rs:1924-1961`). Unwrapped layouts never overflow, so nothing hangs. These editors and the ellipsis fitting (`text.rs:1002`) are regression-only cases, not hanging cases. The plan should say so.
- **Wheel and reveal already disagree.** `position` clamps to `size.w + cw - w` (`text_fields.rs:2814`), but the wheel clamps to `size.w - w` (`:3513`). After an End reveal, a wheel toward the end therefore moves the text 1 px the other way and reports Handled. Sharing one helper fixes this, and a test should cover it.
- **Arrange always forces a reveal**, focused or not (`:2608`). The plan's claim that "unfocused editors keep their retained offset" is false for any relayout.
- **The IME anchor is already pinned into `inner`** (`:3095`).

## Blocker: sideways reveal in a soft-wrapped editor

A TextArea wraps to the inner width. The proposed interval scrolls every line sideways to follow a hanging caret:

1. **Typing prose jiggles the text.** A word ends with less room left than one space advance. Typing a space makes it hang, so the whole text shifts left by up to one space advance plus the caret width. The next letter wraps to the next line, the clamp brings the offset back near 0, and the text jumps back. This happens on an ordinary fraction of soft wraps. The base build already shifts by 1 px here; the proposal makes it larger.
2. **Long runs slide without limit.** Under Preserve, any number of spaces stays on the line and hangs. Holding Space slides every line's start out of view. RTL behaves the same way with negative offsets and a blank strip on the left.
3. **Unfocused editors shift too.** Because of the forced arrange reveal, an unfocused editor with a hanging caret moves its text on resize or theme change, even though no caret is painted.

**Recommended replacement:** in wrapping editors, pin the caret rectangle's x into the layout's own horizontal box. That box is the larger of `size().width()` and the wrap width, in painted-origin coordinates, minus the caret width. One private helper produces this pinned rectangle, and caret paint, reveal and `input_state` all use it.

- `scroll_x` stays non-negative. Overflow scrolling of unbreakable words is unchanged.
- The wheel and `position` share one interval; only the +cw mismatch gets fixed.
- No inspection semantics change. If negative offsets are kept anyway, the public `scroll_x` inspection property (`:3705`, documented in `number_inputs.rs:170`) needs documented sign semantics.
- Drop the plan's requirement that clicking the painted caret gives the canonical position. Pinning can't guarantee it. Require instead that a hit at or beyond the line end keeps Parley's existing `at_point` contract, and document that a pinned caret is drawn at the edge.
- The RTL 1 px right-edge caret is covered by the same rule.

## Required corrections, either way

1. **Cache order** (`text.rs:1385-1431`). Test for zero area *after* the cache lookup and admission, not "before cache admission". Otherwise every phase-line miss renders every space again through swash, which is a performance regression. The cache entries cost 0 bytes; leave admission as it is. Skip only the alignment render, the `PlacedGlyph` and the bounds union, in both the default and phase paths.
2. **`ink_bounds` and `composite` change too.** Whitespace-only text ends up with `ink = None` and no blank canvas-sized mask. A zero-area glyph that used to split brush chunks no longer does, so mask-part counts change. Require pixel-identical CPU captures, record the display-list part deltas, and check that icon centering through `ink` (used by `paint_icon`) is unchanged for the bundled icons.
3. **Rewrite the oracles from the tab checkpoint in the same change.**
   - `tabs.rs:290-299` `expected_canvas` explicitly counts empty rasters, and `assert_policy_canvas` compares against a reference built by the same service. Both must exclude zero-area placements independently, not by calling the implementation.
   - Re-derive the hanging-tab delta table.
   - Turn the RTL tab/space-twin divergence in `tab_whitespace.rs` into an equality assertion. That equality is the main positive proof.
4. **Define the base.** This checkpoint sits on the uncommitted tab patch in the main tree. "Base" must be that tree with recorded hashes, not 3bbfb81 alone.
5. **The wheel uses the frame's own geometry.** Store the interval (or the pinned caret) in `Prepared` during paint and arrange. Don't recompute it from `prepared.layout` with the current selection: after an edit that hasn't been painted yet, that combines a stale layout with a newer selection.
6. **Tiny viewports.** Build the interval so that min ≤ 0 ≤ max (`f32::clamp` panics otherwise). Drop the horizontal `.max(0.0)` at `:2810`, or make it the pin rule, and keep the vertical one. Pick one deterministic placement for when the inner width is smaller than the caret.
7. **Read-only editors.** Their caret isn't painted, so the visible proof there must be selection fill, not caret fill.
8. **Reword the plan's claims to match the code:** arrange always reveals; a wheel event before the next paint cancels an edit's pending reveal (existing behaviour; keep it and record it).

## Evidence adjustments

- Keep the public CPU-pixel assertions for caret fill, selection fill and the IME anchor at scale 1 and 1.25.
- Add the prose-jiggle and held-Space cases as public negative controls: the offset stays fixed and the caret stays inside `inner`.
- Add a negative control for the wheel +cw mismatch.
- Add a negative control that restores pre-lookup skipping, caught by a phase-glyph cache-hit assertion.
- Keep the old-clamp and ghost-raster negative controls.

## TODO entries touching these files

Only **TEXT-01** (`TODO.md:30`). It's correctly out of scope and none of the above changes it. I recorded nothing because this review was read-only. The +cw mismatch is in scope for this checkpoint and doesn't need a TODO entry.
