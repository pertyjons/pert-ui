**Verdict: approved for implementation, on two conditions.** Revision 2 is coherent with review 1. I found no counterexample to the pinned-caret model, the empty-raster ordering or the adopted evidence. Two places in the text are underspecified, though, and could produce a regression or a meaningless test. Fix them in the plan first; each needs one sentence. This approves the design only. I made no edits, builds or runs, and this is not workspace, native or full-usability approval.

## Facts that support coherence

- **The caret always fits the box.** Wrapping TextAreas lay out at `inner.width().max(1.0)` (`text_fields.rs:2604`, `:2946`), and the caret is exactly one logical pixel wide (`text_selection.rs:189`). So the box, `max(size.w, wrap)`, is at least as wide as the caret. Clamping x to `[0, max(box − cw, 0)]` stays valid and finite at every allocation.
- **No sideways shift without overflow.** With the pin, a non-overflowing wrapped layout has box = wrap = w. The pinned caret's end is then at most w, so reveal never scrolls horizontally. That resolves prose jiggle, held Space and the arrange-forced reveal of unfocused editors (`:2608`).
- **The empty-raster skip has a clean insertion point.** It goes right after phase-glyph admission (`text.rs:1412-1431`) and before the alignment render (`:1440`). Byte accounting is unaffected because the entries are zero bytes. A raster that is empty in one phase but not another only changes phase parts, and `phase_parts` already re-offsets those against the default origin (`:1516-1524`). Size and origin come only from the default path.

## Condition 1: the wrapping interval must not include +cw

The plan shares "the existing nonnegative horizontal interval (including the caret-width allowance)" with the wheel. Today the wheel's upper bound is `size.w − w` (`:3513`). Applying the `+cw` bound to wrapping editors gives this counterexample:

- **Setup:** a TextArea that doesn't overflow, where the widest line is within 1 px of the wrap width.
- **Today:** a horizontal wheel finds nothing to scroll, returns Ignored and propagates to the parent.
- **Under the plan:** the text shifts by up to 1 px and the wheel returns Handled.

That breaks the plan's own "no-op wheel propagation preserved" requirement. With the pin, wrapping editors never need the allowance for reveal. **Specify:** `max = max(extent − w, 0)`, where extent is the box for wrapping editors and `size.w + cw` for single-line, numeric and draft editors. That keeps today's single-line behaviour and stays independent of the caret, so it can be stored in `Prepared`. Add this case to the wheel evidence.

## Condition 2: the narrow-width rule names the wrong quantity

"Inner width too narrow to contain the caret → pin at the start of the box" can't trigger, because the box is always at least the caret's width. The real case is a viewport narrower than the caret (w < 1). There the reveal loop (`:2805-2811`) applies the end check last, so the caret's end stays in view and its start is clipped.

One reading of the plan's sentence would instead move the caret to x = 0 regardless of its offset. **State one rule:** the pin always clamps x to `[0, max(box − cw, 0)]`. Then either keep the reveal's existing end-wins choice for w < cw or switch to start-wins, and test whichever you pick.

## Non-blocking clarifications

1. **The cache-order negative control.** A "skip before cache lookup" mutation can't be built: the area is only known after a lookup or render. The meaningful mutation skips before admission. The default path has no per-glyph cache, so this control applies only to the phase path. There is also no production cache-hit counter; the only counters are local to the benchmark (`text.rs:1790`). Instead, assert that the zero-byte key is present in `phase_glyphs` after a phase-line miss and that a second miss adds no entries, or add a private `cfg(test)` render counter.
2. **Base hashes.** The plan says the base is "captured by source hashes" but doesn't name the record. Point it at the exact manifest, either `tab-presentation/repair3/checks/14-owned-file-hashes.log` or the `final-0.3.92` provenance. The findings section still cites repair2 as evidence; check that this is intended.
3. **Documentation of the pin's side effects:**
   - Consecutive hanging caret stops are drawn at the same pinned x, so arrow presses there show no visible movement.
   - Selection fill and the composition underline stay unpinned (`text_fields.rs:2973`, `:2995`). Fill over hanging whitespace can extend past the pinned caret and be clipped.
4. **IME freshness.** `input_state` recomputes the caret from the prepared layout plus the current selection (`:3084-3087`). That is the same stale combination the plan forbids for the wheel. It's a pre-existing issue and the pin doesn't make it worse. State whether the shared helper keeps that freshness or uses a rectangle stored with `Prepared`.

TEXT-01 (`TODO.md`) is still the only TODO entry for these files, and it is still out of scope. I recorded nothing, since this review was read-only.
