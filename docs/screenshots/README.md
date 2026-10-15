# Gallery screenshots

These images come from the [component gallery](../../apps/gallery/README.md),
using its public capture contract and CPU renderer. They show the gallery's
initial state in the default dark theme, except the Toolbar capture, which uses
the gallery's Violet preset on the dark palette. They are actual rendered UI,
without an operating-system window frame or a GPU presentation. A still image
does not demonstrate hover, focus, animation or interaction behavior.

The whole-window PNGs are 1280 × 800 pixels at scale 1. Component PNGs were
captured at scale 2, with the pixels outside the captured component removed by
the capture API. The WebP files used in the root README are lossless copies of
the matching PNGs. Choose PNG for inspection or reuse, and WebP when a smaller
web download is helpful.

## Views

| Screenshot | Gallery page | What it shows |
|---|---|---|
| [Toolbar](toolbar.png) | Controls › Toolbar | The dark Violet palette, grouped actions and readouts, an editable name, compact numeric controls and a status bar. |
| [Tool windows](tool-windows.png) | Layout › Tool windows | Docked panel stripes, a file tree, the main file view and a shared selection. |
| [Floating windows](floating-windows.png) | Surfaces › Floating windows | Overlapping in-view windows with controls, tabs and a resizable frame. |
| [Input & parameters](input-parameters.png) | Controls › Input & parameters | Multi-key input and several editors bound to one numeric parameter. More editors continue below the viewport. |
| [Controls](controls.png) | Controls › Controls | A checkbox and toggle sharing state, choice controls, and surface openers. |
| [Collections](collections.png) | Data & media › Collections | A virtual grid and tree over a keyed data source; the table continues below the viewport. |
| [Text input](text-input.png) | Text › Text input | Single-line and multiline editors, alignment, drafts and read-only text. |
| [Adaptive forms](adaptive-forms.png) | Controls › Adaptive forms | A responsive form beside a resizable preview pane. |
| [Images](images.png) | Data & media › Images | PNG fitting, pixel sampling and a worker-decoding example. |
| [Vector shapes](vector-shapes.png) | Data & media › Vector shapes | Filled and stroked paths drawn through the public `Painter` API. |
| [Themes](themes.png) | Appearance & tools › Themes | Surface groups and semantic color roles in the default palette. |

## Components

| Screenshot | What it shows |
|---|---|
| [Checkbox and toggle](checkbox-toggle.png) | Two controls sharing one Boolean value, beside a disabled checkbox. |
| [Image fits](image-fits.png) | Contain, cover and fill for one transparent PNG in equal display boxes. |
| [Virtual grid](virtual-grid.png) | A cropped `GridList` viewport on the Collections page. |
| [Painter shapes](painter-shapes.png) | A close view of path fills, caps, joins, ellipses and a trace. |

## Recreating a capture

From the repository root, run the gallery's `--screenshot` mode. The first
argument selects a page by key, the second selects the whole window or a
component, and the last is the output PNG path:

```sh
cargo run --locked -p pert-ui-gallery -- --screenshot --palette violet toolbar window docs/screenshots/toolbar.png
cargo run --locked -p pert-ui-gallery -- --screenshot --scale 2 controls key:boolean docs/screenshots/checkbox-toggle.png
```

The mode's full syntax and target choices are in the
[gallery guide](../../apps/gallery/README.md). Before
replacing a checked-in image, compare it visually with the current gallery and
keep its description accurate. Lossless WebP copies can be produced with
ImageMagick, for example:

```sh
magick docs/screenshots/toolbar.png -strip -define webp:lossless=true -quality 100 docs/screenshots/toolbar.webp
```
