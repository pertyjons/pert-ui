I approve the visible-ink policy, with conditions. Revision 4 needs three fixes before repairs start: one wording error, one requirement that can't be met as written, and test cases that are still too loose to fail on a regression. I read the code only. I ran, edited and delegated nothing, and wrote no memory or TODO entries.

## Why the natural ink policy is sound (no ghost geometry)

- **How the canvas is built today.** It is the union of Parley's box, `[0, 0, ceil(width()), ceil(height())]`, and every placed glyph's ink (`text.rs:1118`, `:1298-1467`). The public origin is `-left/scale, -top/scale` (`:1153`). Carets and selections add that origin; hit testing subtracts it (`text_selection.rs:168-182`, `:209-221`). So hit testing already uses the same translation by construction; no code change is needed there.
- **Parley's `width()` excludes hanging whitespace** (`parley-0.12.0/src/layout/line_break.rs:1632-1640`).
  - Tabs hang exactly like spaces under the `Preserve` + wrap defaults (`whitespace.rs:61-73`).
  - On soft-wrapped lines they hang fully. At a forced break or the end of the text, only the overflowing part hangs (`line_break.rs:1780-1791`).
  - In RTL, hanging whitespace is moved to negative x (`alignment.rs:84-88`).
- **What the repair does.** A hidden tab becomes geometrically a hanging space, which the library already ships.
- **The old box ink was a defect, not a contract.**
  - In wrapped RTL text it shifted every line right by about the box width, and the text could overflow its allocation.
  - It made wrapped widgets measure wider than their width (`selectable_text.rs:184`, `:350`).
  - It gave a narrow TextArea a horizontal scroll range (`text_fields.rs:2796`).
  - Keeping it as invisible geometry would preserve that defect and make tabs differ from spaces.
  - These effects come from reading the source; the required before/after values should confirm them.

## Gap 1: the "logical layout" wording is wrong

"Derive the canvas from logical layout… caret/selection positions in the logical layout remain unchanged" holds only in Parley coordinates. The logical box leaves out hanging whitespace. So after the change, caret and selection rectangles at a hanging tab can lie outside `[0, size]`, exactly as they already do for hanging spaces. Revision 4 must state:

- **The canvas definition:** Parley `width()` × `height()` (excluding hanging whitespace), unioned with drawn ink. Hidden tabs contribute no ink. A mixed-atom ligature is drawn ink.
- **What "tab-defined overhang" means:** a case where a hidden tab's glyph set a pre-fix canvas edge that no other drawn glyph or the logical box reaches.
- **The expected deltas:**
  - LTR: width shrinks, origin unchanged.
  - RTL: `origin[0]` drops to the visible-ink edge, so every public x moves left by the old overhang.
  - Vertical origin and `first_baseline` stay unchanged; assert that, because labels and fields align on the baseline.
- **Rustdoc:** `TextLayout::size` currently says "Logical extent including glyph overhangs" (`text.rs:210`). It and the `TextSelection::caret`/`rectangles`/`at_point` docs must say that geometry at hanging trailing whitespace (spaces and tabs) may fall outside `size()`.

## Gap 2: the TextArea requirement fails under the approved policy

- **The caret reaches hanging positions by keyboard.** On a soft-wrapped line, Parley's `line_end` returns `(line end, Upstream)`, which places the caret after the hanging whitespace (`editing/selection.rs:370-379`).
- **TextArea can't reveal it.** It wraps at the inner width, clamps scrolling to `[0, size.w + cw - w]` and clips painting to `inner` (`text_fields.rs:2586`, `:2796`, `:2941`). A caret at negative x (RTL) or beyond `size()` (LTR) is clipped and can't be scrolled to.
- **This already happens with spaces.** Any wrapped prose line that nearly fills the width behaves this way today.

So "the caret and selection remain in the revealed viewport" will fail at hanging positions. Choosing only non-hanging positions to make it pass would be a weak test.

**Requirement:** don't block the tab patch on an editor repair, and don't use ghost ink.

1. **Non-hanging positions:** carets at interior positions, before a hanging tab, and after a tab at a non-overflowing forced break or end must be inside the viewport once layout settles.
2. **Hanging positions:** LineEnd on the hanging line, and a selection that includes the hanging tab, must match a hanging-space twin of the same text: both clipped, with the same scroll offset.
3. **Record the change:** keep the pre-fix scroll extent and reveal outcome as an explicit change, and assert that wrapped text no longer has a horizontal scroll range (`size.width ≤ inner width`).
4. **Pre-fix values:** the facade can't call the `cfg(test)` pre-fix switch. Take pre-fix values from a recorded run on the base commit; the facade test asserts the repaired values.
5. **Separate defect:** TextArea can't reveal a caret at hanging whitespace. This is a whitespace-wide defect outside this scope. Report it and record it in TODO.md, under `text_fields.rs` `position` and the AGENTS.md reporting rule.

## Gap 3: exact raw-coordinate and hit-test cases

The current `assert_matches_reference` subtracts each layout's origin. When `hangs()` is true it only checks that the width shrank (`tabs.rs:358-374`), and no case is required to actually hang. Required:

- **Cases:**
  - LTR and RTL wrapped text with a hanging tab on a middle line.
  - RTL with a tab partly hanging past the end.
  - A tab before an explicit newline that doesn't overflow.
  - Unbounded single-line LTR and RTL text ending in a tab, which doesn't hang. This proves the "normal cases unchanged" claim with exact size and origin.
  - A fixture mixed atom ending a wrapped line. It keeps its ink, so pre-fix size and origin must be identical.
- **Classification check:** classify each case from Parley line metrics (`hanging_advance > 0` and the tab cluster inside the hanging range). Require at least one non-zero RTL `origin[0]` delta and one non-zero LTR width delta, so a case can't quietly stop testing translation.
- **Exact policy assertion:** compute the expected size and origin independently, as the union of the logical box and the pre-fix reference's non-tab glyph ink. Then assert:
  - repaired caret and selection rectangles equal the reference rectangles minus the old origin plus the new one, at no origin normalization;
  - vertical origin and baseline are unchanged;
  - the literal values are recorded in the evidence.
- **Scale:** run at scale 1.0 and at one fractional scale; the origin is rounded to physical pixels.
- **Hit-test points:**
  - (a) over a visible glyph whose public x moved;
  - (b) inside the band between the old and new origin;
  - (c) over the hanging region: negative x in RTL, beyond `size()` in LTR;
  - (d) outside points that snap to the nearest caret.
- **How to check each point:** assert the repaired byte offset and affinity from Parley's clusters, not from the other layout. Then assert the reference gives the same result at the translated point. Include `word_at_point`.
- **Weight:** label the caret-to-hit round trip inside one layout as a regression guard. It is structurally guaranteed, so it is not editor evidence.
- **Ellipsis:** for a candidate ending at a tab, assert the chosen candidate equals the reference's, or record a change. Fitting compares `size().width()` (`text.rs:982`).
- **SelectableText (recommended):** one wrapped RTL hanging-tab case with pointer selection at fixed points. It is the other public selection surface.

## Gap 4: mixed-font evidence needs concrete cases and negative controls

- **One rich layout combining Body, Monospace and the fixture, with tabs:**
  - as the last byte of a span;
  - as the first byte of a span;
  - as a span's only content;
  - as `\t|\t` straddling a font boundary (no cross-font ligature; both hidden);
  - in a fixture pure `\t\t` ligature beside Body;
  - in a fixture mixed atom at a wrap point;
  - as a hanging tab at a font boundary, which also feeds Gap 3.
- **Per-tab assertions:** for every tab cluster, assert the actual chosen font id, size, normalized coordinates and advance against the reference. The bundled fonts map the tab to glyph 0, so record whatever font fallback actually picked.
- **Icon placement:** the nearest text run must be a hidden-tab run whose metrics differ enough from the next visible run that skipping hidden runs moves the icon by at least 1 physical px. Add negative controls:
  - excluding `HiddenTab` from the nearest-run search must fail;
  - an override that also sets the tab's font family or size (for example to Body where the fixture maps U+0009) must fail on font or advance assertions.
  - Record them like the existing six, leaving the earlier records untouched.
- **Phase rendering:** cover several non-zero phases and both cache orders. Compare coverage outside tab regions against the reference, and confirm parts are placed against the repaired origin (`text.rs:1494-1504`).

## Documentation and changelog

- **CHANGELOG ("Changed"):** for hanging tabs only, the extent shrinks and the RTL origin shifts. Wrapped RTL text no longer shifts right, and wrapped widgets and TextArea no longer measure wider or scroll horizontally because of tab ink. Canonical bytes, Unicode analysis, advances and Parley-coordinate carets are unchanged.
- **Rustdoc:** qualify Monospace (`text.rs:145`) and TextArea (`text_fields.rs:240`), and link the `TextLayout` exception, as revision 4 already requires. Also update `size()` and the `TextSelection` docs as in Gap 1.

Once these are written into revision 4, the policy is approved for repair. The independent recheck of the completed P1/P2 evidence is still required before integration.
