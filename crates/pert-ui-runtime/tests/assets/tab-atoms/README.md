# Tab-atom test font

`TabAtomFixture.ttf` is a synthetic font for the runtime's literal-tab tests
(`src/text/tabs.rs`). It is original: `generate.py` draws every glyph as a
rectangle from scratch and reads no other font. It is not derived from the
bundled fonts or any third-party font.

- Glyph order: `.notdef`, `space`, `tab`, `X`, `A`, `B`, `mixed`, `backmixed`,
  `tabs`. The tests name these glyph IDs.
- `cmap` maps U+0009 to the visible `tab` glyph, and also maps space, `A`, `B`
  and `X`.
- `rlig` (DFLT and latn) substitutes `tab X` by `mixed`, `X tab` by `backmixed`
  and `tab tab` by `tabs`, so real shaping forms ligatures that mix a tab with
  printable text and ligatures of tabs alone.

The tests check this shaping through Parley rather than assuming it.

## Provenance

- Generator: `generate.py` in this directory, run with Python 3.14.8 and
  fontTools 4.60.1. It writes `TabAtomFixture.ttf` beside itself, or to the
  path given as its argument. Timestamps are fixed, so the output is
  reproducible with that fontTools version.
- Committed font: 1700 bytes, SHA-256
  `ef405e8ef511ced850becf815e71f0a4461f693a76b426814425fbded332450b`.
- The font is committed. Builds and tests read the committed file with
  `include_bytes!` and never run the generator, so neither Python nor
  fontTools is needed to build or test.

## License

The generator and the font are part of pert-ui and are licensed under
MIT OR Apache-2.0, like the rest of the project; see the repository's
`LICENSE-MIT` and `LICENSE-APACHE`. The font's name table records the same
license.
