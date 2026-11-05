# Manual

The application documentation is the guide in the `pert-ui` crate's rustdoc:
one chapter per area of the library, each item's reference page beside it,
and every example a doctest that imports only the public API. Open it
locally with

```sh
cargo doc --locked -p pert-ui --open
```

and follow `pert_ui::guide`. This page tells you where to start and which
chapter covers what. The [contracts](contracts.md) explain state and
identity, and [rendering and platform](rendering-platform.md) explains
display behavior.

## Start in a few minutes

The library has not been released to a registry. Add the facade from your
local checkout to your application's `Cargo.toml`, and use the toolchain in
the checkout's `rust-toolchain.toml`:

```toml
[dependencies]
pert-ui = { path = "../pert-ui/crates/pert-ui" }
```

The guide's getting-started chapter is the complete first program, kept as
the compiled package example
[`getting_started.rs`](../crates/pert-ui/examples/getting_started.rs); run it
from the checkout with `cargo run -p pert-ui --example getting_started`.

The default `desktop` feature supplies `App`, native input and GPU
rendering. `desktop-cpu`, with `default-features = false`, supplies the same
`App` presented by the CPU renderer and leaves wgpu out of the build, for
tools and machines without a usable GPU. `png`, `jpeg` and the other
image-format features add decoders; owned `RasterImage` pixels need none of
them. `svg` adds SVG loading and its component, `cpu` the CPU renderer for
headless captures, and `serde` serialization for the supported inspection
and workspace documents. The crate's front page lists every feature.

## Guide chapters

The chapter sources are in [`crates/pert-ui/src/guide/`](../crates/pert-ui/src/guide/);
their item links resolve only in the generated documentation.

| Chapter | Covers |
|---|---|
| Getting started | A first window, composition and keys, the data contracts, a saved workspace and a custom widget. |
| State | Choosing between signals, parameters and live data; signals and controls; worker results, application state and intervals; a settings form; bounded updates, failures and shutdown. |
| Authoritative values | Parameters, local edit gestures, host notifications, effective overlays and external authority. |
| Layout and views | Rows, columns, grids, regions, splits, tabs, floating and native windows, workspaces, children from values, large collections, table cell editing, folder browsing, revealing, content search and navigation history, zoomable content and drag and drop. |
| Controls | Text and actions, boolean and choice controls, sliders, text and numeric editors, selectable text, progress, colors, inline controls and toolbars. |
| Surfaces | Selects, popovers, tooltips, menus and commands, dialogs and notifications. |
| Appearance | Themes and color roles, density, zoom, shapes, library strings and number formats, fonts, icons, images and SVG. |
| Diagnostics | Keyboard input, inspection, screenshots, serialization and profiling. |
| Testing an application | A headless driver: reading what the runtime shows, clicks, text, in-place editors, keys, settling, list rows, tables, background work, the close policy and a CPU capture. |
| Custom widgets | A component that follows a binding, phases and writes, pointer gestures, a control that edits a value, clicks, held keys, described children, inspection and an inspector over an application document. |

## Example applications

The packaged examples are applications with headless sessions, run from the
checkout:

- [`live_mixer`](../crates/pert-ui/examples/live_mixer.rs) connects a
  producer thread to meters and parameters, with undo, retry of undelivered
  edits and a saved layout:
  `cargo run -p pert-ui --example live_mixer --no-default-features`.
- [`diagram_editor`](../crates/pert-ui/examples/diagram_editor/main.rs) is a
  canvas widget with an application around it:
  `cargo run -p pert-ui --example diagram_editor --no-default-features --features cpu`.
- [`log_explorer`](../crates/pert-ui/examples/log_explorer/main.rs) shows
  bounded collections over hundreds of thousands of lines, worker loading, a
  live tail and copying:
  `cargo run -p pert-ui --example log_explorer --no-default-features`.
- [`inventory`](../crates/pert-ui/examples/inventory/main.rs) edits a table
  in place like a spreadsheet, with commands, undo and row copy and paste:
  `cargo run -p pert-ui --example inventory --no-default-features`.
- [`dynamic_documents`](../crates/pert-ui/examples/dynamic_documents.rs)
  opens documents with dynamic resources and external acceptance of edits,
  headless only: `cargo run -p pert-ui --example dynamic_documents`.

The first four also open a window with `-- --window`. The
[gallery guide](../apps/gallery/README.md) shows every component on live
pages, and [verification](verification.md) records execution evidence and
the native-platform gaps.
