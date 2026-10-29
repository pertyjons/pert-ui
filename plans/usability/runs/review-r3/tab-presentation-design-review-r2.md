## Verdict: **approved, with conditions**

I found no reason in the source why the typed-brush marker can't keep the invariants revision 2 lists. Three gaps in the sketch would let a faithful implementation go wrong, though. B1 and B2 are design decisions; B3 corrects the plan's cost statement. Treat all three as conditions of this approval and record them in the checkpoint before any code is written. If the author rejects any of them, the approval lapses.

This verdict comes from reading code only: the checkout plus Parley 0.12.0 and parley_engine 0.12.0 under `~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/`. I ran and edited nothing, and wrote no memory or TODO entries. I did not review or approve revision 1's character substitution.

### What the source confirms

- **Analysis is unchanged.** Parley analyzes and shapes the same string (`parley-0.12.0/src/builder.rs:358-410`).
  - Analysis reads style runs only for non-default line-break or break-spaces settings, which pert-ui never sets (`analysis.rs:28-51`).
  - So bidi, grapheme/word boundaries and line-break classes are the same as today.
- **Shaping is unchanged.**
  - Items split only on size, locale, variations, features or spacing; brush is not among them (`shape/mod.rs:151-160`).
  - Font selection re-queries only when the font family or font attributes change (`:327-356`).
  - So harfrust receives the same buffers and produces the same `Run`s.
- **Selection is unchanged.** `TextSelection` uses only Parley's `Cursor`/`Selection` on `Run`s and clusters (`text_selection.rs:113-203`), and a brush change doesn't split a `Run`.
- **Line geometry is unchanged.**
  - A brush override copies the span's style and changes only the brush (`resolve/range.rs:99-136`).
  - Span metrics come from the family stack's first available font, the size and the line height (`style_metrics.rs:117-131`, `:348-368`).
  - pert-ui uses `LineHeight::Absolute`, so each line box is built from those per-style metrics (`layout/data.rs:58-70`).
  - The glyph-run baseline is `style_baseline(style_index)` (`layout/line.rs:508`), with vertical-align at baseline and parent 0, so it is equal too.
- **Glyph runs split exactly at the tab.**
  - A glyph run's style comes from each *atom's* first character (`line.rs:456-486`).
  - An atom is the smallest span with both grapheme and shaped-cluster edges (`parley_engine-0.12.0/src/shape/atom.rs:473-479`).
  - Shaping uses `MonotoneCharacters` (`shaper.rs:329`), and a tab is followed by a grapheme break (GB4).
  - So with the bundled fonts, `\t\u{301}` gives the mark its own atom with its span's style: the mark is not hidden.
- **Unsupported characters keep their ink.** U+000B is `ControlWhitespace` (`shape/cluster.rs:137-139`); Parley removes glyphs only for `Newline` (`shaped_text.rs:474-477`).
- **Every shaping route meets in `build_layout`.**
  - Normal preparation: `shape` (`text.rs:1092`).
  - Width measurement: `plain_shape` (`:865`), whose layout `fit_whole` reuses (`:1007-1012`).
  - Ellipsis candidates: through `plain_width` (`:942`).
  - Phase rendering re-rasterizes the stored, tagged `layout.shaped` (`:1423-1424`).
  - Cache hits return before the builder (`:665`, `:1050`).
- **Icon placement is preserved, as long as a hidden-tab run counts as text.**
  - Splitting one Parley run into sub-runs that share font metrics and baseline doesn't change which run is nearest under `min_by_key` index distance with first-index tie-breaking (`text.rs:1508-1529`).
  - A tab already in its own run (for example at a bidi level change) must stay a candidate; filtering it out would move icons.
- **Editor behaviour matches the evidence list.** The editor admits tabs only in multiline fields (`text_fields.rs:3711-3722`). Single-line paste maps tabs to spaces (`:2032-2058`).

### Conditions to record before implementation

**B1. Hidden runs must be distinguishable in the raster signature.** The sketch says the signature "may omit" the hidden run; that is too loose.
- The phase-line cache key is only `signature + scale + phase` (`text.rs:426-431`, `:1415-1421`). Unlike `shape`'s reuse check (`:1105-1109`), it doesn't compare the resolved text.
- Every missing character shapes to glyph 0 with the same per-font advance (probe `output.txt:8`). So `"A\tB"` and `"A\u{E000}B"` have identical glyph tuples and canvas.
- If the hidden run is recorded as an ordinary color run, phase mode can paint the box for the tab, or drop it for the private-use character.
- **Decision:** leave hidden runs out of `RasterRun`s (recommended), or record them with a typed marker. Never record them as `Color(n)`. Alignment must still iterate all glyph runs.

**B2. Decide what happens when a font merges a tab with a neighbour.**
- A cluster takes its first character's style, and a ligature spans graphemes (`shaped_text.rs:489-491`; Parley's own test `three_graphemes_one_cluster`, `:744-764`).
- With a registered font that maps U+0009 and ligates it, a `\tX` atom would be tagged HiddenTab, and X's ink would disappear.
- The bundled fonts can't trigger this, because they have no U+0009 glyph.
- **Decision:** hide only atoms made entirely of tabs, using `Cluster::is_ligature_start`/`is_ligature_continuation` (`layout/cluster.rs:199-206`); anything mixed keeps its ink (recommended). Or document it as a known limitation for registered fonts.

**B3. Correct the plan's cost paragraph.**
- The ranged style builder never merges equal styles that aren't adjacent. Each tab-separated piece becomes its own style-table entry, and the builder panics past `u16::MAX` styles (`range.rs:139-172`, assertion at `:164`).
- Each style costs a font query when metrics are resolved (`style_metrics.rs:117-124`).
- Each extra glyph run costs a swash scaler build (`text.rs:1265-1271`) and its own `RasterRun` allocations.
- The worst case is about 2 × 8192 + spans styles, under the limit only because of `MAX_TEXT_BYTES` = 16 KiB (`text_input.rs:57`, enforced at `text.rs:785`).
- **What to do:**
  - Add a compile-time assertion that ties that bound to `MAX_TEXT_BYTES` and `MAX_SEGMENTS`.
  - Push the tab overrides after the span properties, in ascending order, with runs of consecutive tabs merged.
  - Keep HiddenTab out of `Resolved`, or `cached_plain` (`:683-706`) stops matching.
  - Skip hidden runs before building the scaler.
  - Name this per-tab work in the plan instead of "bounded by admitted text length".

### Tests the implementation must add (beyond the plan's list)

1. **Pre-fix reference.** Build it with a test-only switch inside the same `build_layout` that skips the override.
   - Line breaks, line metrics and `size()` should match exactly.
   - Compare glyph x positions within a small tolerance. Splitting a run changes how Parley adds up offsets (per atom at `line.rs:464-510`, per glyph at `:337-346`), so the last bits can differ after a hidden run.
2. **Phase collision (B1).** Render `"A\tB"` and `"A\u{E000}B"` in phase-line mode, in both orders. The box appears only for U+E000.
3. **Atom purity (B2).** In bundled Body and Monospace, every tab cluster is a single-grapheme atom. In `\t\u{301}`, the mark keeps its ink.
4. **Icon alignment.** Place an icon next to a tab that has its own run (a bidi field boundary) and next to a run that the split divides. Alignment must equal the pre-fix reference.
5. **Worst case (B3).**
   - A 16 KiB alternating `"a\t"` text, plain and with `MAX_SEGMENTS` spans: no panic, and the style count stays within the bound.
   - A TextArea holding tab-separated text: record its timing as an observation only, not a performance claim.
6. **Tab-only text.** No mask parts, `ink` is `None`, `size` equals the advance, and painting and caret stops still work.
7. **U+000B ink.** Test it through `TextService` or a label. Editors reject U+000B or turn it into LF, so a TextArea can't hold it.

I found nothing outside this scope that needs a TODO.md entry.
