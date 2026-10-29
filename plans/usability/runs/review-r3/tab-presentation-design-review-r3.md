## Verdict: approved, with implementation requirements

The revision 3 B2 algorithm holds up against the installed Parley 0.12.0 source. I found no design correction it needs. What follows are implementation requirements and tests for B2, plus one addition to the B3 cost paragraph. I read code only: I ran, edited and delegated nothing, and wrote no memory or TODO entries.

### What the source confirms

- **The atom grouping is sound.**
  - `Run::clusters()` walks the line-scoped slice once, linearly (`parley-0.12.0/src/layout/run.rs:362-439`).
  - It moves to a new atom exactly when `Grapheme::is_atom_start()` is set (`parley_engine-0.12.0/src/shape/atom.rs:666-669`).
  - So grouping on "not `is_ligature_continuation`" (`layout/cluster.rs:199-206`) reproduces Parley's own atoms. That includes a grapheme spanning several shaped clusters, which comes out as one atom with neither ligature flag.
  - A tab is a Control grapheme, so a tab cluster's `text_range()` is exactly one `\t` byte. "Mixed" therefore means a multi-grapheme atom holding at least one `\t` cluster and at least one other cluster.
- **Atoms never cross a line or Run boundary.**
  - Breaking places whole atoms, including in emergency breaks (`line_break.rs:1181`, `:1299-1300`).
  - Line slices are atom-aligned (`atom.rs:273-289`).
  - Shaping runs end where the selected font changes (`shaper.rs:379-393`), so a shaped cluster can't span two runs.
  - So walking each line's runs visits every atom whole, exactly once.
- **The mixed decision doesn't depend on wrapping.** The atom partition comes from shaping, not width. The walk needs `lines()`, so it runs after `break_all_lines`. Pass 2 repeats break and align with the same width.
- **The style restoration is right, and more robust than a narrower rule would be.**
  - A glyph run takes its style from the atom's first logical character (`line.rs:456-486`). Pass 1 therefore hides the whole `\tX` atom; `X\t` keeps X's style.
  - Restoring every tab in a mixed atom also stays correct under Parley's noted TODO to use the shaped cluster's style instead (`line.rs:456`, `shaped_text.rs:491`).
  - Don't narrow the rule to "atoms whose first character is a tab".
  - With the override gone, the tab takes its span's resolved style again, so the atom gets exactly its pre-fix brush and color, even when it crosses a color span boundary.
- **The second pass is stable.**
  - Items split only on size, locale, variations, features or spacing (`parley-0.12.0/src/shape/mod.rs:151-160`).
  - The font selector resets its query only when the family id or attributes change, and a brush override clones the span's family (`shape/mod.rs:327-356`, `resolve/range.rs:99-136`).
  - So harfrust gets identical runs and the partition is the same. Not re-walking (no retry loop) is justified.
  - `has_mixed_style_atoms` adds metrics for the atom's other styles (`line_break.rs:435-443`, `data.rs:349-357`), but brush-only styles have identical metrics.
- **It covers every shaping route.** All routes meet in `build_layout` (`text.rs:1138`): `shape`, `plain_shape` (whose layout `fit_whole` reuses), ellipsis candidates through `plain_width`, and the pre-fix test switch. Cache hits return before it, and phase rendering re-rasterizes the stored final layout.
- **The data stays bounded.** Pass 2 pushes no more than pass 1, so the compile-time style bound still holds. The collected set is at most the number of tabs: about 8 K ranges, or a 2 KiB bitset.

### Requirements to record with the implementation

1. **Sort the collected tabs before rebuilding.** `Line::runs()` returns runs in visual order (`line.rs:112`), so tab ranges collected across bidi runs are not in ascending order. Mark the mixed tabs (sort the ranges, or set bits on the ascending tab list). Then rebuild the pass-2 overrides in one ascending merge. A merged consecutive-tab override must split: in `\t\t\tX` where only the last tab is in a mixed atom, the result is hidden `0..2` and not hidden `2..3`.
2. **Use the right walk.** Walk `layout.lines()` → `line.runs()` → `run.clusters()` once each, and skip the walk entirely for text without tabs. Don't walk through `GlyphRun::run()` per glyph run, because that rescans the whole run once per sub-run. Also avoid `Run::get`/`len` (each is O(n)) and per-tab `Cluster::from_byte_index`.
3. **Instrument the second build.** The test counter `shapings` counts calls to `build_layout`, not Parley builds. Count actual builds, or count the fallback separately, so that tests can assert one build for the bundled fonts and exactly two for the mixed fixture.
4. **Add a committed font fixture.** The repository has no font with a U+0009 glyph and a GSUB tab ligature; the existing tests only rewrap bundled bytes (`text.rs:1998-2008`). The required real-shaping test needs one, with:
   - license and provenance recorded, and no generation at build time;
   - cases for `\tX` (printable ink and color kept), `X\t`, a pure `\t\t` ligature (no ink), `\t\t\tX` (the range split) and a pure tab in the same text;
   - a test-only re-walk of the pass-2 layout asserting that no mixed atom carries HiddenTab and every pure-tab atom does.
5. **Extend the B3 cost paragraph.** Each override push does a `Vec` insert into the span list, which shifts every later span (`range.rs:111-117`, `:135`).
   - For plain text with ascending pushes that is near-constant.
   - With rich spans it is O(overrides × later spans).
   - The rare fallback path pays the build, break and align a second time.
   - The planned worst-case test with `MAX_SEGMENTS` spans and alternating tabs should be the evidence for this. Record it as an observation, not a bound.

The documented exception, that a custom font which ligates a tab with printable text keeps that glyph's ink, is consistent with all of this. I found nothing out of scope that needs a TODO.md entry.
