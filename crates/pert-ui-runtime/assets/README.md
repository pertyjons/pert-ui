# Bundled fonts

`NotoSans.ttf` is the unmodified Noto Sans variable font from
[Google Fonts](https://github.com/google/fonts/tree/main/ofl/notosans), downloaded
2026-09-11. Copyright belongs to the Noto Project Authors. It is redistributed
under the SIL Open Font License 1.1 in [OFL.txt](OFL.txt).

`NotoSansMono.ttf` is the unmodified Noto Sans Mono variable font from
[Google Fonts, commit 097bc1b8c04c3224087c2ae95f7a923859b778cf](https://github.com/google/fonts/tree/097bc1b8c04c3224087c2ae95f7a923859b778cf/ofl/notosansmono),
downloaded 2026-09-12. Copyright 2022 The Noto Project Authors. Its separate
SIL Open Font License 1.1 notice is in [OFL-NotoSansMono.txt](OFL-NotoSansMono.txt)
(only trailing whitespace was normalized). The font bytes have SHA-256
`2cb2adb378a8f574213e23df697050b83c54c27df465a2015552740b2769a081`.
`TextStyle::Monospace` selects its regular weight and normal width at 14 logical
pixels for aligned values, table cells and code. Other presets use Noto Sans.

The bytes are included in the library package; builds do not download fonts or
depend on installed system fonts. The initial text path uses the font's regular
weight and width. Missing characters render the font's missing-glyph mark, except literal tabs
which keep their shaped advance without tab-only ink. This is not an aligned
tab-stop model. A registered font may ligate tab and printable text into one
inseparable atom; that glyph retains its ink. Canonical bytes and Unicode
analysis are unchanged. This
bundled-font baseline does not claim complete script coverage or color emoji.

## Material Icons Outlined

`MaterialIconsOutlined-Regular.otf` and its complete upstream
`MaterialIconsOutlined-Regular.codepoints` catalog are unmodified files from
[google/material-design-icons, commit 40a7a292a79d9394157e1ea24f83d52d5e17c556](https://github.com/google/material-design-icons/tree/40a7a292a79d9394157e1ea24f83d52d5e17c556/font),
downloaded 2026-09-12. Copyright belongs to Google and the upstream contributors.
They are redistributed under Apache License 2.0, included verbatim in
[LICENSE-MaterialIcons.txt](LICENSE-MaterialIcons.txt). The pinned repository root
contains no separate NOTICE file. These asset terms are independent of the Rust
library's dual license; downstream distributions must retain applicable asset
licenses and notices.

SHA-256:

| File | SHA-256 |
|---|---|
| MaterialIconsOutlined-Regular.otf | `b63fa9edd75e3c20328e04ad31dcc38ce76411f3f9ea1a1ff87f49e5ba874b05` |
| MaterialIconsOutlined-Regular.codepoints | `c57e7279a7ef29925cd98e538c618a037a42b43120576f4660a6732c7a5a4d4a` |
| LICENSE-MaterialIcons.txt | `58d1e17ffe5109a7ae296caafcadfdbe6a7d176f0bc4ab01e12a689b0499d8bd` |

Library `Icon` roles map to named entries in this pinned catalog. Applications can
resolve every catalog entry with `FontManager::material_icon`, register their own
font bytes and override individual roles or an entire role mapping. Builds use
only checked-in assets, without downloads, font generation or system discovery.
