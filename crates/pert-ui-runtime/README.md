# pert-ui-runtime

Backend-independent UI state, retained component lifecycle, input, inspection
and drawing commands for pert-ui. This **unreleased foundation** depends on
`pert-ui-core`, `thiserror`, Parley, Swash, `unicode-segmentation` and, behind
its opt-in `serde` feature, serde; it needs no window, GPU or operating-system
services.
Normal standalone applications use the `pert-ui` facade, which re-exports every
public item of this crate at its root and shows this documentation on its pages;
an application that depends on `pert-ui` writes `pert_ui::` wherever an example
on this page writes `pert_ui_runtime::`. The examples on the item pages are
written for the facade and import `pert_ui::…`; code that depends on this crate
directly writes `pert_ui_runtime::` in their place. Custom hosts and graphics
integrations can use this crate directly, supplying events and elapsed time and
consuming its owned `DisplayList`. Retained layout, standard controls, text/numeric editing, dialogs, menus,
notifications and a separately versioned `Workspace` document for the
arrangement are available.

Application code globs `prelude`, which holds what an application's first
program writes: the built-in components with their configuration types and
the names an application uses everywhere. Code that implements `Component`
also globs `component`, and a headless test globs `testing`. The thematic
modules gather the rest by need: `parameter` for authoritative values,
`commands` for commands and the command palette, `windowing` for windows,
regions, splits and tool windows, `inspection` for inspection, capture and
profiling, and `live` for data from other threads. Every name also stays at
the crate root.

```rust
use pert_ui_runtime::parameter::*;

let amount = ParameterSpec::new("Amount", -100.0, 100.0, 0.0)?;
let id = ParameterId::new(1, 42, "amount")?;
let position = NormalizedValue::try_from(0.75)?;
assert_eq!(amount.denormalize(position), 50.0);
assert_eq!(amount.normalize(&f32::NAN), Err(ParameterError::NonFinite));
assert_ne!(id, id.next_incarnation()?);
# Ok::<(), Box<dyn std::error::Error>>(())
```

Positions are dimensionless; canonical numeric values have application-defined
meaning. Linear mapping and editable metadata are defaults. An optional
`with_mapping(ParameterMapping::Logarithmic)` selects a strictly positive
logarithmic domain. Metadata does not hold a current value or enforce host edits.
Bindings and synchronous local edit sessions are implemented. Unit presets remain
future work. Four closed parameter families share one protocol: continuous
`f32` (`ParameterSpec::new`), exact stepped `i64` (`ParameterSpec::stepped`),
`bool` (`ParameterSpec::boolean`) and choices identified by stable `ChoiceId`s
(`ParameterSpec::choices`); nothing is erased into floats.

Parameter IDs include session, object, stable key and runtime incarnation. The
host assigns identifiers, retains the next incarnation after recreation and must
match the complete ID before applying edits. IDs do not provide a registry or
message routing on their own. `Runtime::parameter` registers each complete identity
once and gives controls a shared binding plus a separate authoritative host owner.

## Reading commands in another graphics integration

A custom widget uses the same public recorder as built-in components. The owned
result outlives the runtime and exposes primitives without graphics-library types:

```rust
use pert_ui_runtime::{Color, Component, DisplayList, DrawCommand, DrawError,
    Element, Painter, Rect, Runtime, RuntimeError, UpdateContext, Viewport};
use std::time::Duration;

struct Tile;
impl Component for Tile {
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> { Ok(()) }
    fn paint(&self, painter: &mut Painter) -> Result<(), DrawError> {
        painter.fill_rect(Rect::new(8.0, 8.0, 40.0, 20.0)?, Color::ACCENT);
        Ok(())
    }
}
fn read_fills(scene: &DisplayList) -> Result<Vec<(Rect, Color)>, &'static str> {
    scene.commands().iter().map(|command| match *command {
        DrawCommand::FillRect { rect, color } => Ok((rect, color)),
        _ => Err("unsupported drawing command"),
    }).collect()
}
let mut runtime = Runtime::new();
runtime.set_viewport(Viewport::new(100.0, 80.0, 1.5)?)?;
runtime.mount_tree(Element::new("tile", Tile))?;
runtime.flush(Duration::ZERO)?;
let scene = runtime.paint()?;
drop(runtime);
assert_eq!(scene.viewport().scale(), 1.5);
assert_eq!(read_fills(&scene)?[0].0.values(), [8.0, 8.0, 40.0, 20.0]);
# Ok::<(), Box<dyn std::error::Error>>(())
```

This example reads commands; it is not a second rasterizer. A graphics adapter
must draw them in order, clip to the logical viewport, honor scale, blend linear
straight-alpha colors and premultiplied image pixels, and convert to its output
color space. Unsupported commands
must produce an error. Window ownership, scheduling and presentation belong to the
host. The supplied wgpu adapter implements those drawing semantics independently
of the runtime. Default `App` currently selects that adapter only; other graphics
systems integrate through a custom host, without changing the widget contract.

## Vector paths

`PathBuilder` builds a `Path` of lines, Bézier curves and arcs, and constructors
cover polylines, polygons, ellipses, circles and rounded rectangles.
`Painter::fill_path` fills one with a `FillRule`, `Painter::stroke_path` strokes it
centered with a `StrokeStyle` of width, `LineCap`, `LineJoin` and miter limit, and
`Painter::stroke_polyline` records a path from a slice of points, the
usual form of a trace or graph. Each primitive preserves vector geometry in the display list. Adapters can
consume it directly or use the cached `VectorRasterizer` coverage fallback.

```rust
use pert_ui_runtime::{ColorRole, Component, DrawError, Element, Length, LineCap, Painter, Point,
    Runtime, RuntimeError, StrokeStyle, UpdateContext, Viewport};

struct Trace(Vec<Point>);
impl Component for Trace {
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> { Ok(()) }
    fn paint(&self, painter: &mut Painter) -> Result<(), DrawError> {
        let width = Length::new(2.0)?;
        let style = StrokeStyle::new(width).with_cap(LineCap::Round);
        painter.stroke_polyline(&self.0, style, painter.color(ColorRole::Primary))
    }
}
let points = (0..32)
    .map(|i| Point::new(i as f32 * 4.0, 20.0 + 16.0 * (i as f32 / 5.0).sin()))
    .collect::<Result<Vec<_>, _>>()?;
let mut runtime = Runtime::new();
runtime.set_viewport(Viewport::new(128.0, 40.0, 1.0)?)?;
runtime.mount_tree(Element::new("trace", Trace(points)))?;
runtime.flush(std::time::Duration::ZERO)?;
assert_eq!(runtime.paint()?.vectors().len(), 1);
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Color images

`RasterImage` holds decoded, validated pixels: RGBA8 in sRGB, premultiplied once
at construction from the straight alpha that decoders produce, at most 64 MiB.
`Image` places one in the layout with a fit, and `Painter::draw_image` records one
from a custom component. A scene stores equal images once, keyed by content, and
only images that a composed command references. This crate never decodes or loads
files; the `pert-ui` facade does that behind opt-in format features.

```rust
use pert_ui_runtime::{DrawCommand, Element, Image, ImageFit, ImageSampling,
    RasterImage, Runtime, Viewport};
use std::time::Duration;

// A 2x2 checker built in code: opaque black and white.
let pixels = [[0, 0, 0, 255], [255, 255, 255, 255], [255, 255, 255, 255], [0, 0, 0, 255]];
let checker = RasterImage::from_srgba8(2, 2, pixels.concat())?;
let mut ui = Runtime::new();
ui.set_viewport(Viewport::new(64.0, 32.0, 1.0)?)?;
ui.mount_tree(Element::new(
    "checker",
    Image::new(checker)
        .fit(ImageFit::Contain)
        .sampling(ImageSampling::Nearest)
        .description("Checkerboard"),
))?;
ui.flush(Duration::ZERO)?;
let scene = ui.paint()?;
assert_eq!(scene.images().len(), 1);
let DrawCommand::DrawImage { rect, image, .. } = scene.commands()[0] else {
    unreachable!("the only command draws the image");
};
// Contain keeps the square aspect inside the 64x32 window, centered.
assert_eq!(rect.values(), [16.0, 0.0, 32.0, 32.0]);
assert_eq!(scene.image(image).map(RasterImage::width), Some(2));
# Ok::<(), Box<dyn std::error::Error>>(())
```

An adapter samples an image as linear light, multiplies it by the command's
straight-alpha tint and blends the premultiplied result. See the
[raster image notes](https://github.com/pertyjons/pert-ui/blob/main/docs/rendering-platform.md)
for the pixel format, sampling and budgets.

## Retained composition and custom layout

`mount_tree` mounts an owned `Element` hierarchy. Keys are unique within each
parent; `reconcile_children` preserves same-key, same-type components and their
state. New constructor values for surviving components are discarded: use
bindings for content updates or a new key for replacement. `Runtime::reparent`
moves a retained subtree to another owner, or to another position under the same
owner, at a chosen index: incarnations, local state, subscriptions and focus
survive, and visibility is reevaluated against the new owner.
`UpdateContext::request_reparent` is the deferred form for a component that
restructures its own content during an update; `Toolbar` uses it to move a group
between the strip and the overflow surface. A move that would leave the native
presentation root, carry a detached view or relocate a promoted surface root
returns `NotMovable`. Local visibility is combined with ancestor visibility;
unmount recursively releases child resources. The existing `mount` API still
creates independent window-coordinate roots.

Custom containers implement `Component::measure` and `Component::arrange` through
public contexts. Measurement is cached by constraints and invalidation; placement
uses local logical coordinates. Runtime places and clips recorded rectangles when
composing the public display list, through the component's origin and the uniform
scale of any `Component::child_transform` above it. Layout errors leave work pending and preserve
the previously published geometry. Signal writes during layout are rejected.

```rust
use pert_ui_runtime::{
    ArrangeContext, Component, Constraints, Element, MeasureContext, Runtime,
    RuntimeError, Size, UpdateContext, Viewport,
};
use std::time::Duration;

struct Content;
impl Component for Content {
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> { Ok(()) }
    fn measure(&self, _: &MeasureContext<'_>, _: Constraints) -> Result<Size, RuntimeError> {
        Ok(Size::new(120.0, 40.0)?)
    }
}
struct Fill;
impl Component for Fill {
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> { Ok(()) }
    fn arrange(&self, cx: &mut ArrangeContext<'_>, size: Size) -> Result<(), RuntimeError> {
        for index in 0..cx.len() {
            cx.place(index, pert_ui_runtime::Rect::from_size(size))?;
        }
        Ok(())
    }
}
let mut ui = Runtime::new();
ui.set_viewport(Viewport::new(640.0, 480.0, 1.0)?)?;
let root = ui.mount_tree(Element::new("view", Fill).children([
    Element::new("content", Content),
]))?;
ui.flush(Duration::ZERO)?;
assert_eq!(ui.children(&root)?.len(), 1);
ui.set_visible(&root, false)?;
assert!(ui.next_deadline().is_none());
ui.unmount(&root)?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

Measured bounds and effective clips are included in owned inspection snapshots.
Unknown padding/style remains absent. Developer overlap checks compare siblings,
not parents against their own children. Labels and buttons use the text service and pointer/focus routing. Native OS dialogs are not
supplied. Screenshot capture goes through `Runtime::request_capture`, which a host
that renders captures answers, as described above.

## Layout primitives

`Row`, `Column`, `Wrap` and `Grid` use the theme item gap (8 logical pixels at Standard). Wrapping preserves
source order and puts exact fits on the current row. `Container` is transparent
with zero padding by default. Use a container only when content needs decoration,
padding or a preferred size; ordinary rows/columns need no spacing setup.

```rust
use pert_ui_runtime::{Color, Column, Container, Element, Grid, Row,
    Runtime, Size, Track, Viewport, Wrap};
use std::time::Duration;
let tile = |key: &str| -> Result<Element, pert_ui_runtime::GeometryError> {
    Ok(Element::new(key, Container::default()
        .preferred_size(Size::new(80.0, 40.0)?)
        .background(Color::ACCENT)))
};
let content = Column::new([
    Element::new("row", Row::new([tile("first")?, tile("second")?])),
    Element::new("wrap", Wrap::new([tile("first")?, tile("second")?, tile("third")?])),
    Element::new("grid", Grid::new([Track::CONTENT, Track::weighted(1.0)?],
        [tile("first")?, tile("second")?])?),
]);
let mut ui = Runtime::new();
ui.set_viewport(Viewport::new(200.0, 300.0, 1.0)?)?;
ui.mount_tree(Element::new("content", content))?;
ui.flush(Duration::ZERO)?;
let scene = ui.paint()?;
assert!(!scene.is_empty());
# Ok::<(), Box<dyn std::error::Error>>(())
```

Clipping is resolved into the public rectangle commands, so existing display-list
adapters need no new clip-stack protocol. `Painter::with_clip` supports nested local
clips through the same mechanism. Raw primitives report overflow by their measured extent
and clip content at presentation time; `ScrollArea` and `RegionLayout`, described
below, give that overflow scrolling access.

## Where the rest is documented

Applications use the `pert-ui` facade, whose
[guide](https://docs.rs/pert-ui/latest/pert_ui/guide/index.html) covers
state, authoritative values, layout and views, controls, surfaces,
appearance, diagnostics, testing and custom widgets, with every example a
doctest. The item pages of this crate hold each item's complete contract.
The sections above cover what a custom host or graphics integration that
depends on this crate directly needs beyond the guide.

Requires Rust 1.99.1 or later, edition 2024. Licensed under either MIT or
Apache-2.0, at your option; both license texts are included in this package.
