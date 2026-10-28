## Verdict: **needs changes**

Showing a literal tab as one space is a reasonable policy for this library. The plan needs one design correction before implementation, though: it claims the mapping applies "only for shaping", and that isn't true. Fix that claim, decide the contract it changes, and widen the required evidence.

The observed defect is real. `build_layout` passes the tab straight to Parley (`text.rs:1164-1209`), and `rasterize_glyphs` draws every glyph it is given, including glyph 0 (`text.rs:1272-1319`). The probe output confirms glyph 0 with an 8.4 px advance. This review covers only the design; I read code and edited or ran nothing.

### P1 (blocking): the swap also changes Unicode analysis, not just glyph choice

In Parley 0.12, one string feeds both text analysis and shaping. `build_into_layout` calls `analyze_text(lcx, text, …)` and then `shape_text(…, text, …)` (`parley-0.12.0/src/builder.rs:339-400`). `ranged_builder` only uses the length (`context.rs:96-107`). So replacing U+0009 with U+0020 changes four things beyond the glyph:

1. **Bidi order.** A tab is a segment separator in the bidi algorithm. Parley resets it, and any whitespace just before it, to the paragraph's base direction (`parley_engine-0.12.0/src/bidi.rs:111-127`). A space is ordinary whitespace and takes the direction of its neighbours.
   - Effect: in a left-to-right paragraph, right-to-left fields separated by tabs (for example `שם\tגיל`) merge into one right-to-left run, so the fields appear in reverse order. The same happens to left-to-right fields inside right-to-left text.
   - Today the order is correct and only the box is wrong. Left/Right caret movement (`text_selection.rs:140-141`) follows the new order.
2. **Caret stops.** For grapheme splitting, a tab is a control character, so the text always breaks after it. A space followed by a combining mark forms one grapheme.
   - Effect: with `"\t\u{301}"`, the canonical text has a boundary at byte 1 but the layout won't offer it. `TextSelection::new(layout, 1, 1)` would then return `InvalidSelection`, although its documentation promises that "the start of a grapheme" is valid (`text_selection.rs:54-68`).
   - The editor counts graphemes on the canonical text (`text_fields.rs:1502`, `:2570`), so its count and the caret stops would disagree. This is a degenerate input, but it contradicts the plan's claim that line and caret offsets remain valid.
3. **Word boundaries.** A run of spaces stays together as one word segment; each tab is separate. Word movement and double-click selection (`text_selection.rs:142-143`, `:178-183`) therefore change.
4. **Line-break class.** A tab and a space belong to different line-break classes, which differ only in rare cases (for example `"(\tx"`). Trailing-whitespace hanging is already identical (`parley-0.12.0/src/layout/whitespace.rs:64`).

**What to change:** state the contract as "a literal tab is laid out exactly as one U+0020 of its span: advance, wrapping, bidi and grapheme/word segmentation. The text, offsets and copies keep the tab." I recommend accepting that contract:

- Parley's builder has no way to analyse one string and shape another.
- No substitute character keeps the tab's bidi and grapheme behaviour while rendering blank. U+000B is classed as visible control whitespace, and U+001F would show the missing-glyph box.

Record the lost bidi segment separation as a known limitation, to be restored by any future tab-stop model, and pin it with a test. If the maintainer finds the right-to-left reordering unacceptable, the only narrower fix is the plan's own alternative of skipping tab clusters during rasterization. That keeps the font-dependent missing-glyph width, and I don't recommend it.

### P2: where the swap goes, and the cache (the plan is sound once these are pinned)

- **Placement.** Apply the swap inside `build_layout` and nowhere else, and pass the same swapped string to both `ranged_builder` and `build` (`text.rs:1166`, `:1209`). It is the only place in the workspace that calls `ranged_builder`. Both shaping routes reach it:
  - normal preparation: `prepare_resolved` → `shape` (`text.rs:1092`);
  - width measurement: `plain_shape` (`:865`), whose layout `fit_whole` reuses (`:1007-1012`).
- **Keep it out of `resolve` and `prepare_plain`.** The cache key and the plain-text lookup compare canonical text (`text.rs:683-706`, `:1050`).
- **Rendering paths.** Phase rendering re-rasterizes the stored `layout.shaped` without reshaping (`text.rs:1410-1424`), so it is covered.
- **Editor paths.** TextArea's `arrange` and `layout_of` go through `cx.text` (`text_fields.rs:2553`, `:1489`). So does the preedit display string (`:2522-2525`).
- **Paths that never see a tab.** Concealed text never contains one, and icon characters cannot be whitespace or control characters (`text/registry.rs:139`). Inline-gap indices (`text.rs:1198-1207`) and span ranges are unchanged because the swap keeps byte lengths.
- **Cache behaviour.** `"A\tB"` and `"A B"` stay separate cache entries. Raster reuse in `shape` requires equal resolved text (`text.rs:1105-1109`), so the two can't alias. The phase-line cache may share coverage between them, which is correct because their glyphs are identical.
- **Allocation.** Cache hits return before `build_layout` is reached (`text.rs:665`, `:1050`). Use a borrowed string when there is no tab, so tab-free text allocates nothing extra.

### P3: is single-space presentation suitable?

Yes. It is a total presentation mapping as AGENTS.md permits, and it adds no tab-width or tab-stop contract. Two usability costs must be documented, though, not implied away:

- TextArea deliberately admits tabs (`text_fields.rs:2028-2040`, `:3693-3704`). With this change a tab looks the same as a space (about 3.6 px at Body size) and tab indentation almost disappears.
- `TextStyle::Monospace` advertises use for "code" (`text.rs:138-143`). Tab-indented code will show one column per tab, not aligned columns. Say so on both TextArea and Monospace.

### P4: documentation that must change together

These sentences currently say unsupported characters use the missing-glyph mark; each needs the tab rule added:

- `text.rs:180-181`
- `crates/pert-ui-runtime/README.md:497` and `:1392`
- `guide/controls.md:74` and `guide/appearance.md:343`
- `assets/README.md:19`

Also update:

- `docs/rendering-platform.md`, "Text, icons and images" (lines 148-155)
- `docs/contracts.md`, "Text and draft editing" (lines 1228-1236)
- the caret-stop paragraph in `TextSelection`'s docs, if the grapheme edge case is accepted
- CHANGELOG, as a behaviour change

The `api/` listings should come out unchanged; confirm that rather than assume it.

### Tests to require

The plan's evidence list plus these:

- **Layout equivalence.** For Body and Monospace, with leading, embedded and repeated tabs, a tab text and its all-space version give identical glyph IDs, positions and `size()`. Include:
  - wrapped text where a tab is the break point, giving the same lines and hanging as the space version;
  - a tab at a rich-span boundary and inside a different-size span (its advance should equal that span's space);
  - a tab next to `Inline::Gap`.
- **Unsupported characters keep the box.** An unassigned or private-use character and U+000B still produce glyph 0 and ink.
- **Every shaping route.** Cover a cache miss, a cache hit (the shaping count doesn't increase), the `fit_whole` reuse of shaped text, and truncated ellipsis candidates that contain tabs. Render in both the default line-mask mode and phase-line mode.
- **Selection.** On `"A\tB"`, every byte is a valid caret stop, Left/Right step over exactly one byte, the caret x after the tab equals the caret x after the space in `"A B"`, and the selection rectangle is one space wide.
- **Pinned behaviour.** Fix in tests the chosen behaviour for bidi order (`"אב\tגד"`, and left-to-right text inside right-to-left), for word movement over `"\t\t"`, and for `"\t\u{301}"`.
- **TextArea through the facade.** Start with an initial tab, then deliver `"\t"` as committed text: navigation, selection and backspace act on exactly the tab byte, and the binding, inspection value, clipboard copy and saved value keep U+0009. A keyboard Tab press still moves focus and inserts nothing. Paste still keeps tabs in multiline editors and turns them into spaces in single-line fields.
- **Capture.** A CPU capture of `"A\tB"` has no ink over the tab and matches the `"A B"` capture. The GPU renders from the same display list, so reuse the existing backend evidence and claim no new native or Windows/macOS evidence.

There's no existing TODO.md entry for this; the findings fall within this checkpoint's scope.
