**Verdict: approve, subject to the implementation tests below and one factual correction to the plan before implementation.** Every decision from the first review now has a clear answer, and none of them conflict with each other. The remaining issue is a factual one in the list of affected composites, not an open design question.

## Decisions from the first review, checked against source

1. **Shared inactive frame.** Revision 2 now accepts this on purpose. The source confirms it: `field_frame` (`choice_controls.rs:148-173`) draws a Surface fill with an Outline at BorderWidth on CornerRadius. A disabled TextField paints Muted text inside that frame, and an empty field paints a Muted placeholder (`text_fields.rs:2920-2953`). The required capture puts an enabled button, a disabled button, an empty field with a placeholder and a disabled field side by side in both palettes, which is the right way to make the tradeoff reviewable. The plan doesn't say what result would fail that capture review, but that is a judgement for whoever reviews it.
2. **Shared roles and no override for disabled buttons alone.** The plan now says this explicitly, and Surface, Muted and Outline are documented as general roles (`theme.rs:418-431`). The plan's own docs list covers the wording to fix: `Button` "Resting action control fill" and `OnButton` (`theme.rs:446-451`), BorderWidth "Panel and separator stroke width" (`:105`), "opaque blue-gray action surface" (`controls.rs:434-435`), and `frameless` "resting fill" (`:634`).
3. **Paint order.** The plan's order is fill, then outline, then focus ring, then content. Both strokes draw inward (`shapes.rs:340-378`), so the ring covers the outline. Today the pressed branch (`controls.rs:810`) doesn't check `enabled`; it's only unreachable because of the resets at `:706-709` and `:741-745`. Choosing the disabled roles first removes that dependency.
   - One small difference: `field_frame` draws the ring *instead of* the outline, while Button will draw both. This is harmless, because a disabled button isn't focusable (`:731`), but the implementation should know it's intentional.
4. **BorderWidth edge cases.** Width 0 draws nothing (`shapes.rs:373`). A width of at least half the smaller side fills the whole shape (`:390-391`). An icon-only button's minimum width equals ControlHeight, so that case behaves the same way. Documenting this instead of clamping is consistent with "clamp only for documented limits".
5. **Gesture and contrast evidence.** The plan reuses the existing tests rather than duplicating them. `assert_readable` covers (Surface, Muted) at 4.5:1 and (Surface, Outline) at 3:1 (`crates/pert-ui/tests/presentation/theme.rs:413, 423`).
6. **Repaint without relayout.** This holds. `Painter::color` and `Painter::metric` record paint-only dependencies (`drawing.rs:1563-1578`), so the override test is meaningful.
7. **Select/MultiSelect and Button inspection colours.** The defect is confirmed (`select.rs:1275-1295`, `multi_select.rs:1049-1062`). Neither item is in `TODO.md` yet, which is expected since nothing has been implemented. Both still need recording.

## Correction needed: the list of affected composites

- **AlertDialog.** Revision 2 says its generated framed buttons "inherit the change". They do share the paint code, but they can never be disabled: they have no enablement and `dialog.rs` has none at all (`dialog.rs:626-633`). There is no visible change. The CHANGELOG must not claim one; at most it can say they share the paint path.
- **Menu trigger (missed by both the first review and revision 2).** `Menu::new` and `Menu::with_state` build a framed `Button` trigger and call `.disabled()` on it when no item can become enabled (`menu.rs:821-826`, `:900`). The Menu rustdoc says "Empty/all-disabled menus have a disabled trigger" (`:673-674`). This is the one built-in composite whose appearance actually changes. By contrast, the trigger of `Menu::icon` and `Menu::icon_with_state` (`:868`) and the tool-window panel's hide button (`tool_windows/panel.rs:140-143`) are frameless, so like HistoryNavigator they keep the text-only treatment.

Before implementation, revision 2 should fix the AlertDialog wording, name the Menu trigger as the inherited visible change, and add one focused test. That test should check that an empty or all-fixed-disabled `Menu`, reached through its exposed `trigger` part, paints the Surface fill and Outline stroke. It should also check that an icon menu's trigger stays without fill or outline. I didn't find any other internal framed Button that can be disabled. The only other disabled-Button constructions are the frameless ones above, and the `controls.rs:593` command enablement path, which the plan already covers.

## Tests the approval depends on

The tests revision 2 already lists, plus the Menu trigger test above:
- Revoking during a hold paints no ButtonPressed fill, and re-enabling brings the Button fill back.
- Overriding Surface, Outline, Muted or BorderWidth repaints without a relayout.
- BorderWidth 0 draws no stroke.
- A disabled frameless button draws no fill or stroke.
- A command button with a contextual predicate looks unchanged.
- The adjacent field/button CPU captures in both palettes.

## Limits

I only read the source and docs. I ran nothing, didn't look at any captures, made no edits and didn't delegate. This approves the revised policy only. It doesn't count as implementation, native or human evidence, or closure of the usability round.
