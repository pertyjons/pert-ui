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

```rust
use pert_ui_runtime::prelude::*;
use pert_ui_runtime::ParameterError;

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

## Local state and retained components

```rust
use std::time::Duration;
use pert_ui_runtime::prelude::*;

struct Label {
    text: ReadSignal<String>,
    retained: String,
}
impl Component for Label {
    fn update(&mut self, context: &mut UpdateContext) -> Result<(), RuntimeError> {
        self.retained = context.read(&self.text)?;
        Ok(())
    }
}

let mut runtime = Runtime::new();
let text = runtime.signal(String::from("Ready"));
let label = Label { text: text.read_only(), retained: String::new() };
let id = runtime.mount_tree(Element::new("status", label))?;
assert_eq!(runtime.flush(Duration::ZERO)?.components_updated, 1);
assert_eq!(runtime.next_deadline(), None);
text.set(String::from("Working"))?;
text.set(String::from("Done"))?;
assert_eq!(runtime.flush(Duration::ZERO)?.components_updated, 1);
runtime.set_visible(&id, false)?;
text.set(String::from("Hidden update"))?;
assert_eq!(runtime.next_deadline(), None);
runtime.unmount(&id)?;
# Ok::<(), RuntimeError>(())
```

This example drives runtime directly. The default native `App` supplies elapsed
time and waits for work/deadlines automatically. Custom hosts drive the same API. Components retain their own presentation
data. Every update recollects all dependencies; ordinary reads invalidate layout
and paint, while `read_paint` is for data confined to fixed geometry. No writes
are allowed during update, paint or inspection. Changed signals only enqueue dependents,
and hidden components retain dirtiness without evaluation. `animate()` uses the
per-window presentation ceiling (60 Hz when unknown); no catch-up frames are generated.
A timed action that must reach application state uses the separate event-phase
timer instead: `EventContext::set_timer` arms one deadline for the component and
`Component::timer_elapsed` runs in the event phase of the flush that reaches it,
so a held repeat or auto-scroll may write bindings and commit edits like ordinary
input. The timer is one-shot, replaces a pending one, and is cancelled with the
component's input context on focus loss, hiding, unmount or `cancel_input`.
Components record solid rectangles, shapes, vector paths, text and images through
`Painter`; runtime retains their commands and copies the visible scene on `paint`. Unchanged paint callbacks are
not repeated. GPU resources belong to the selected graphics adapter; text preparation
and coverage are cached separately from GPU resources.

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

## Local parameter gestures

```rust
use std::time::Duration;
use pert_ui_runtime::{prelude::*, EditOutcome, RuntimeError};

struct Control;
impl Component for Control {
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> { Ok(()) }
}
let mut runtime = Runtime::new();
let (amount, host) = runtime.parameter(
    ParameterId::new(1, 1, "amount")?, ParameterSpec::new("Amount", 0.0, 10.0, 2.0)?,
)?;
let control = runtime.mount_tree(Element::new("control", Control))?;
runtime.flush(Duration::ZERO)?;
let revision = amount.snapshot().get()?.revision;
let edit = runtime.try_event(&control, |cx| amount.begin_edit(cx, revision))??;
edit.update(4.0)?;
assert_eq!(amount.snapshot().get()?.base, 2.0);
assert_eq!(amount.snapshot().get()?.preview, Some(4.0));
let committed = edit.commit(5.0)?;
assert!(matches!(committed, EditOutcome::Committed { value: 5.0, .. }));
drop(edit); // acknowledges the reserved terminal receipt
host.write_base(6.0)?;
assert_eq!(amount.snapshot().get()?.base, 6.0);
# Ok::<(), Box<dyn std::error::Error>>(())
```

The host owner must stay alive: dropping it removes the parameter. Gesture drop,
Escape, hiding, focus loss and unmount cancel unfinished work. External base or
metadata changes reject conflicting edits. Effective scalar overlays remain
separate and may exceed the editable range. The local protocol retains up to
64 terminal receipts per parameter until acknowledged; guard drop acknowledges
automatically. Commit replay cannot write twice. Undo policy stays with the host.
Live data from a producer thread travels separately, through the bounded, lossy
series bridge and its UI-thread sources; see the live-data source contract in
the repository's design notes. Lifecycle cleanup may cancel during
evaluation; ordinary writes cannot.

## Parameter families and host notifications

Boolean, choice and stepped parameters bind to the standard and inline controls
through `for_parameter`; choice controls take their options from the
specification. A host that owns undo or forwards edits registers a terminal sink
and keeps its delivery monitor. Each changed activation is one begin/commit pair,
delivered once after the new state is published.

```rust
use std::{cell::RefCell, rc::Rc};
use pert_ui_runtime::prelude::*;
use pert_ui_runtime::{EditOutcome, KeyAction, KeyCode};

let mut ui = Runtime::new();
let list = ChoiceId::new("list")?;
let (view_mode, host) = ui.parameter(
    ParameterId::new(1, 7, "view-mode")?,
    ParameterSpec::choices(
        "View mode",
        [
            ChoiceOption::new(list.clone(), "List"),
            ChoiceOption::new(ChoiceId::new("grid")?, "Grid"),
        ],
        list,
    )?,
)?;
let committed = Rc::new(RefCell::new(Vec::new()));
let log = committed.clone();
let monitor = host.on_outcome(move |event| {
    if let EditOutcome::Committed { value, .. } = &event.outcome {
        log.borrow_mut().push(value.clone()); // application undo, forwarding, ...
    }
    Ok(())
})?;
let group = ui.mount_tree(Element::new(
    "view-mode",
    SegmentedControl::for_parameter(view_mode.clone()),
))?;
ui.focus(&group)?;
ui.key_input(KeyCode::ArrowRight, KeyAction::Press)?;
assert_eq!(view_mode.snapshot().get()?.base.as_str(), "grid");
assert_eq!(committed.borrow().len(), 1);
assert!(!monitor.snapshot().is_faulted());
# Ok::<(), Box<dyn std::error::Error>>(())
```

A sink error is retained in the monitor, the parameter refuses new edits and
its controls show as disabled until the host retries or resolves the delivery.
`Committed` is the local store's acceptance, not an external authority acknowledgement.
Sinks run on the UI thread and cannot write to the runtime while they run.
Descriptor-driven panels use the family-tagged `AnySpec`, `AnyValue`,
`AnyParameter` and `AnyParameterHost`; `HostValue` translates an application's
own value types without narrowing them.

## Keyboard and inspection

`key_input` tracks every held physical key and classifies presses, repeats and
releases. A repeat for a key that is not held never establishes a press. Focused
components receive the full pressed set and can implement general
simultaneous-key actions. A key bubbles from the focused component through its
ancestors until one returns `Handled` from `keyboard_event`; the default returns
`Ignored` after calling `key_input`, so a component that only observes keys never
swallows Escape or shortcuts. `input_reset` releases component-owned actions on
context loss, called by the default `cancel_input`; `clear_focus` also clears
window keys. Committed UTF-8 text uses `text_input`,
independently of physical key positions. The default native adapter maps physical
keys and delivers native committed text and IME preedit through the public text
input contract.

`Runtime::inspect()` returns a versioned owned `InspectionSnapshot`, safe to send
to the application's own inspector or tooling thread. `Component::inspect` and `inspect_with` explicitly
contribute component and host state. Unmeasured box-model values stay absent,
including `BoxModelInspection::border`, which reports a painted outline's width
and color only after the paint that drew it.
`DebugMode::Inspect` enables a 256-event history. `DebugOverlay` separately draws
validation markers or component boxes into the scene in builds with debug
assertions. The library starts no server or network listener and ships no MCP
support; see the inspection design note for building one in the application.
`Runtime::query` answers an `InspectionQuery` for the components a `Selector`
finds, and `Runtime::inspection_service` lets a tool on another thread ask for
snapshots, queries, profiles, validation reports and workspace captures,
answered after the next flush. `Runtime::request_capture` asks for a
screenshot of a window, view or widget, which a host that declared
`set_capture_support` renders and the runtime crops to the target.
The `serde` feature adds `Serialize`/`Deserialize` to the snapshot, profile,
validation report and workspace document, validated on the way in.
History is off by default and causes no periodic scheduling. Success/failure and
cancellation are recorded; generic `event` only records dispatch, while `try_event`
records the application's returned result. Text input contents are not logged.

Requires Rust 1.99.1 or later, edition 2024. Licensed under either MIT or
Apache-2.0, at your option; both license texts are included in this package.

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
use pert_ui_runtime::{Color, Column, Container, Element, Grid, Length, Row,
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

## Text and actions

Default labels wrap to allocated width. Buttons supply themed padding, a default Standard 64×32 logical
minimum, visible focus, pointer capture and matched-release Space/Enter activation.
Ignored input bubbles through logical ancestors; Tab traverses visible focus targets.
Escape/context loss cancels activation, including capture owned by nonfocusable
custom components. Native repeats remain repeats after window focus loss.

```rust
use pert_ui_runtime::{
    Button, Column, Element, FocusDirection, Label, Runtime, RuntimeError, Viewport,
};
use std::time::Duration;
let mut ui = Runtime::new();
ui.set_viewport(Viewport::new(400.0, 200.0, 1.0)?)?;
let status = ui.signal("Ready".to_owned());
let action = status.clone();
ui.mount_tree(Element::new("content", Column::new([
    Element::new("status", Label::new(status.read_only())),
    Element::new("apply", Button::new("Apply", move |_| action.set("Applied".to_owned()))),
])))?;
ui.flush(Duration::ZERO)?;
ui.focus_next(FocusDirection::Forward)?;
ui.flush(Duration::ZERO)?;
let scene = ui.paint()?;
assert!(!scene.masks().is_empty());
# Ok::<(), Box<dyn std::error::Error>>(())
```

Parley shapes the bundled OFL-licensed Noto Sans font and Swash prepares coverage
on the UI thread. No system font discovery, color emoji or complete script coverage
is promised; unsupported characters use the font's missing-glyph mark. Body,
heading and caption presets default to 14/18/12 logical pixels, overridable
through theme metrics. Font assets and attribution
ship in the runtime package. Text admission limits and bounded cache behavior are
documented on `TextError` and `MeasureContext::text`; live owners retain resources
independently of service cache eviction. TextField/TextArea and the numeric inputs
provide editing/IME over the same text service.

## Views, groups and region layouts

`RegionLayout` supplies optional full-width top/bottom regions, middle-band left/right
regions, and a filling center. Nest another region layout in any region without creating
a window, event loop or modal scope. Edges size to content; fixed edge extents
remain explicit semantic choices. Each region supplies both-axis overflow access.
Present neighboring regions have dividers, eight logical pixels wide at Standard
density and locked by default. At Standard density top/bottom viewport heights
have a 24-pixel minimum, left/right widths 64 pixels, and center 64 by 64.
When these minima cannot fit, the region layout itself scrolls.
Missing regions add no divider. Give an edge a resize permission with `left_resizable`
(or another edge) to enable user resizing: `true`, `false` or a `ReadSignal<bool>`,
as a [`Permission`] takes. `resizable` restricts every edge at once, and the
stricter permission wins; locking during a drag restores the preceding preference.

```rust
use pert_ui_runtime::{Button, Element, Group, Label, RegionLayout, Runtime, Viewport};
use std::time::Duration;
let mut ui = Runtime::new();
ui.set_viewport(Viewport::new(640.0, 400.0, 1.0)?)?;
ui.mount_tree(Element::new("workspace", RegionLayout::new()
    .top(Element::new("toolbar", Button::new("Save", |_| Ok(()))))
    .left(Element::new("navigation", Label::new("Views")))
    .center(Element::new("editor", RegionLayout::new()
        .top(Element::new("local-tools", Label::new("Editor tools")))
        .center(Element::new("content", Group::titled("Details", Element::new("body", Label::new("Content")))))
    ))
    .bottom(Element::new("status", Label::new("Ready")))
))?;
ui.flush(Duration::ZERO)?;
let _scene = ui.paint()?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

`View::visible_signal` controls descendant visibility through a binding, preserving state
and suspending deadlines. `ScrollArea` defaults to vertical scrolling and reveals
focused descendants automatically. Wheel/arrows/page keys scroll; Home/End reach
the bounds. Thin indicators use retained foreground drawing above children.

Custom containers share `Component::child_visible`, `can_reveal`, `reveal`,
`clips_children` and `paint_foreground`, plus `ArrangeContext::place_clipped`
for independent cell boundaries. Flow/view/group components allow overflow
to reach an enclosing scroll viewport; explicit `Container` and `ScrollArea` clip.
A component's own drawing still clips to its allocation. Successful arrangement
invalidates that component's drawing cache, including geometry-dependent foreground.
Unchanged branches retain layout/drawing. Focus traversal skips targets that remain
fully clipped after a reveal attempt. No periodic timer powers scrolling.

A container that hides children in state it owns, as `Tabs`, `Collapsible` and
`Accordion` do, implements
`Component::reveal_child` so `Runtime::reveal`, `Runtime::reveal_and_focus` and
`EventContext::request_reveal` can open the way to a hidden target; the default
refuses.

## Independent split dividers

`SplitPane` defaults to equal shares and enabled resizing. `LeftRight`/`TopBottom`
name child placement. Choose a ratio or a fixed first/second extent only when that
has meaning for the workspace. Locking is independent of sizing, so locked ratios
still follow window proportions and locked extents retain their preferred size.

```rust
use pert_ui_runtime::{Container, Element, Length, Runtime, SplitAxis, SplitPane, SplitSizing, Viewport};
use std::time::Duration;
let mut ui = Runtime::new();
ui.set_viewport(Viewport::new(640.0, 400.0, 1.0)?)?;
let resize_enabled = ui.signal(true);
ui.mount_tree(Element::new("editor-log", SplitPane::new(SplitAxis::TopBottom,
    Element::new("editors", SplitPane::new(SplitAxis::LeftRight,
        Element::new("main", Container::default()),
        Element::new("details", Container::default()),
    ).sizing(SplitSizing::first(Length::from(240))).locked()),
    Element::new("log", Container::default()),
).sizing(SplitSizing::second(Length::from(100))).resizable(resize_enabled.read_only())))?;
ui.flush(Duration::ZERO)?;
resize_enabled.set(false)?; // cancels a tentative drag, retaining all children
ui.flush(Duration::ZERO)?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

Drag the divider or focus it and use its axis arrows (8 pixels; Shift: 32).
Release commits; Escape, context loss, locking or losing a child cancels and restores
the original preference. One input source owns a resize at a time: arrow resizing
is ignored during a pointer drag, and pointer resize admission is ignored while
resize keys remain held. Arrows from focused child controls never resize an ancestor.

Hard minimums/maxima propagate through nested splits using the public, cached
`Component::measure_limits` contract. Infeasible minima remain scrollable inside
`ScrollArea`/`RegionLayout`; both maxima leave trailing spare space. Temporary clamping
never rewrites the preference. Unrepresentable aggregate minimums return a layout
error; upper bounds saturate at the largest representable logical extent. `SplitPane` preferences are captured and restored with the `Workspace` document;
there is no host undo coupling.

Custom containers receive owned immediate `ChildInfo` snapshots in `UpdateContext`;
structural changes schedule an owner update. `Component::cancel_input` allows a
custom interaction to restore layout state and invalidate it; its default calls
`input_reset` and invalidates paint, and an override must release held-key
actions itself, since the runtime does not call `input_reset` separately.


## Retained tabs and resizable edges

`Tabs` selects the first keyed `Tab` by default. Inactive content retains state
and suspends presentation deadlines. The themed title strip (minimum 36 logical pixels at Standard) scrolls overflowing
titles with arrows or the wheel; selected content has automatic overflow access.
Left/Right/Home/End navigate focused tabs; Ctrl+Tab also works from their content.
There are no close buttons by default. Selection survives reorder; removal chooses
the following neighbor, or the last remaining tab. Titles are display metadata,
independent of identity, and can change during keyed reconciliation.

```rust
use pert_ui_runtime::{Button, Element, Label, Runtime, RegionLayout, Tab, Tabs, Viewport};
use std::time::Duration;
let mut ui = Runtime::new();
ui.set_viewport(Viewport::new(640.0, 400.0, 1.0)?)?;
let resize = ui.signal(true);
ui.mount_tree(Element::new("workspace", RegionLayout::new()
    .left(Element::new("navigation", Label::new("Navigation")))
    .left_resizable(resize.read_only())
    .center(Element::new("views", Tabs::new([
        Tab::new("editor", "Editor", Element::new("action", Button::new("Apply", |_| Ok(())))),
        Tab::new("overview", "Overview", Element::new("summary", Label::new("Overview content"))),
    ])))
))?;
ui.flush(Duration::ZERO)?;
resize.set(false)?;
ui.flush(Duration::ZERO)?;
let _scene = ui.paint()?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

`Tabs::detachable(true)` lets a title be dragged out of its strip and released to
open its retained view in an independent native window. A detachable strip reserves
an `OpenInNew` marker slot in every title, so the affordance appears without
reflowing the strip: hovering a docked title shows the marker in Muted and uses the
Move cursor, while a title being dragged out, and one whose view is already
detached, become emptied SurfaceContainer slots with a PanelBorder outline, a Muted
title and the marker in Primary. The marker is presentation only; the whole title
stays the hit area, and pressing a detached title activates its window instead of
duplicating content.

Tabs are not the only detachable unit. `EventContext::detach_view` and
`Runtime::request_detach` move any mounted view, so an application can offer a
button, a command or its own gesture for a panel that is not a tab. The view keeps
its incarnation, local state, bindings and scheduling; closing the window re-docks
it under the default `DetachedClosePolicy`.

Detachment is permitted per item, and nothing is detachable by accident: a strip
offers the gesture only after `Tabs::detachable(true)`, and `Tab::detachable(false)`
excludes one title from a strip that does. Both are `Element::with_detachable`,
which any view can set. A refusal is enforced centrally, so `prepare_detach`,
`request_detach` and `detach_view` all return `RuntimeError::NotDetachable` instead
of moving the view, and `Tabs` offers that title no marker, no Move cursor and no
drag-out. Reconciliation updates the permission without remounting; an already
detached view keeps its window and can still re-dock. Containers read the same
metadata through `ChildInfo::detachable`.

`SplitSizing::CONTENT_FIRST` and `CONTENT_SECOND` follow child content until a
resize commits a fixed extent. Cancellation restores the content rule. RegionLayout
edges reuse this behavior and the same constrained split implementation.
Custom selection containers can use labeled `Element` descriptions,
`UpdateContext::children` snapshots and `EventContext::request_focus`; the latter
applies after updates/layout and ignores hidden, removed or nonfocusable targets.


## Monospaced values

`TextStyle::Monospace` uses bundled Noto Sans Mono at a default 14 logical pixels, with the
same 1.4 line spacing as body text. Ordinary labels still use Noto Sans. No font
installation or runtime download is required. Use this preset for aligned values,
table cells or code; it is a typography choice, not a table widget. A label wraps
at its allocated width, so use separate grid cells for structured tables.

```rust
use pert_ui_runtime::{Element, Label, Runtime, TextStyle, Viewport};
use std::time::Duration;
let mut ui = Runtime::new();
ui.set_viewport(Viewport::new(320.0, 100.0, 1.0)?)?;
ui.mount_tree(Element::new("value", Label::new("0012   -1.50")
    .style(TextStyle::Monospace)))?;
ui.flush(Duration::ZERO)?;
let _scene = ui.paint()?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Fixed chrome with independent scrolling

A fixed frame starts at its allocation origin, preserving its menu and tab strip.
Tabs own the scrollable content beneath their strip. Fixed overflow clips below
hard minima; enlarge the allocation to recover clipped controls. `View::fixed()`
provides the same local clipping boundary for an individual view and
`ScrollAxes::None` selects it on a viewport. These do not escape scrolling ancestors.

```rust
use pert_ui_runtime::{Runtime, Viewport, RegionLayout, Element, Button, Tabs, Tab, Label};
use std::time::Duration;
let mut ui = Runtime::new();
ui.set_viewport(Viewport::new(640.0, 480.0, 1.0)?)?;
ui.mount_tree(Element::new("app", RegionLayout::new().fixed()
    .top(Element::new("menu", Button::new("Save", |_| Ok(()))))
    .center(Element::new("tabs", Tabs::new([
        Tab::new("first", "First", Element::new("content", Label::new("Long content"))),
        Tab::new("second", "Second", Element::new("content", Label::new("Other content"))),
    ])))))?;
ui.flush(Duration::ZERO)?;
ui.paint()?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Measuring UI work

Profiling is off by default and adds no scheduled work. Enable it while exercising
an interaction, then pause and capture completed scopes. Total includes nested
work; self time excludes measured descendants. These are elapsed UI-thread times,
including measurement overhead and waiting, not CPU percentages or GPU timings.

```rust
use pert_ui_runtime::{Runtime, Element, Label, Viewport, ProfileOperation};
use std::time::Duration;
let mut ui = Runtime::new();
ui.set_viewport(Viewport::new(640.0, 480.0, 1.0)?)?;
ui.set_profiling(true)?;
ui.mount_tree(Element::new("content", Label::new("Profile this view")))?;
ui.flush(Duration::ZERO)?;
ui.paint()?;
ui.set_profiling(false)?;
let report = ui.profile();
assert!(report.stats.iter().any(|row| row.operation == ProfileOperation::Paint));
for row in &report.stats {
    println!("{:?} {}: {} calls, {:?} total, {:?} self, {:?} max",
        row.operation, row.key, row.calls, row.total, row.self_time, row.maximum);
}
ui.clear_profile()?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Themes and color roles

Each runtime starts in dark mode with built-in dark and light palettes. The shared
`ThemeManager` changes modes and overrides colors on the UI thread. Overrides are
stored independently per mode. `Primary`, `Secondary`, `Error`, `Warning`, `Info`
and `Success` have explicit `On…` foreground roles; backgrounds, panels, buttons,
tabs and muted text have their own tokens. Some roles follow another by default,
which `ColorRole::follows` reports: the foregrounds of buttons, tabs, selections
and the subdued `*Container` fills follow `OnSurface`, `PanelBorder` follows
`Divider`, and pressed action controls follow the accent pair through
`ButtonPressed` and `OnButtonPressed`. A follower takes its source's current
color until it is overridden itself, and `Palette::with_default_color` makes it
follow again. Following copies a color; overrides never generate other colors.
Keyboard focus has its own roles, so overriding `Primary` recolors the accent and
pressed controls without changing the focus ring. The focus ring is drawn
inward as one `FocusRing` band at `FocusWidth`. A custom component draws the
same ring with `Painter::stroke_shape` and the focused element's bounds and shape.

```rust
use pert_ui_runtime::{Button, Color, ColorRole, Component, DrawError, Element, Painter,
              Palette, Rect, Row, Runtime, RuntimeError, ThemeMode, UpdateContext};

// External components use the same tracked color lookup as built-in controls.
struct Accent;
impl Component for Accent {
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> { Ok(()) }
    fn paint(&self, painter: &mut Painter) -> Result<(), DrawError> {
        painter.fill_rect(Rect::new(0.0, 0.0, 24.0, 24.0)?,
                          painter.color(ColorRole::Primary));
        Ok(())
    }
}
let mut ui = Runtime::new();
let theme = ui.theme();
// Color::from_linear takes linear channels, not sRGB bytes.
let light = Palette::light()
    .with_color(ColorRole::Primary, Color::from_linear(0.30, 0.04, 0.40, 1.0)?)
    .with_color(ColorRole::OnPrimary, Color::from_linear(1.0, 1.0, 1.0, 1.0)?);
theme.set_palette(ThemeMode::Light, light)?;
theme.set_mode(ThemeMode::Light)?;
ui.mount_tree(Element::new("content", Row::new([
    Element::new("accent", Accent),
    Element::new("toggle", Button::new("Toggle theme", move |_| {
        let next = match theme.mode()? {
            ThemeMode::Dark => ThemeMode::Light,
            ThemeMode::Light => ThemeMode::Dark,
        };
        theme.set_mode(next).map(|_| ())
    })),
])))?;
// Native App performs flush/paint and schedules presentation automatically.
ui.flush(std::time::Duration::ZERO)?;
assert_eq!(ui.paint()?.background(), light.color(ColorRole::Background));
# Ok::<(), Box<dyn std::error::Error>>(())
```

In `App::new`, use the supplied runtime in the same way. A theme edit invalidates
only components that painted with a changed role; hidden components refresh when
shown. This does not rebuild the tree or invalidate layout/text masks. Reads via
`ThemeManager::palette` are untracked snapshots; use `Painter::color` for adaptive
drawing. `Label::color_role` and `Container::background_role` select roles, while
`Label::color` and `Container::background` provide fixed per-component overrides.
The last literal/role builder wins. Existing `Color` constants stay fixed.

Custom graphics adapters must clear to `DisplayList::background()` before drawing
its commands. Custom hosts check `Runtime::needs_paint()` after events/flushes;
a background-only edit can require presentation without component updates.
An unchanged runtime schedules no theme timer. `needs_paint` scans mounted entries
for visible drawing damage. Theme writes compare the fixed palette and visit
registered painters; this is ordinary UI work, not a callback-safe API.
System preference detection, subtree theme providers, persistence, typography and
spacing overrides are outside this color-theme API. Foreground contrast remains
an explicit application choice when overriding colors; measure it with
`OpaqueColor::contrast_ratio`.

`Color` provides the arithmetic a custom component needs on resolved colors,
in linear light. `Color::with_alpha` replaces alpha, while `Color::scale_alpha`
fades a color and keeps the translucency a role already has. `Color::mix`
interpolates premultiplied channels, and `Color::over` composites onto an
`OpaqueColor`. Contrast is defined only for opaque colors, so a translucent
color is measured after compositing it over what lies behind it. The
measurements report; they never adjust or refuse a palette.

## Density and measurements

Dark/light mode and density are independent. Standard density is the default.
Compact and Comfortable change spacing and control sizes while retaining text
sizes. Local fixed lengths ignore density; theme role overrides remain live.

```rust
use pert_ui_runtime::*;
let mut ui = Runtime::new();
let theme = ui.theme();
theme.set_density(Density::Compact)?;
let metrics = theme.metrics(Density::Compact)?
    .with_metric(MetricRole::GroupPadding, Length::from(10))?;
theme.set_metrics(Density::Compact, metrics)?;
ui.mount_tree(Element::new("content",
    Container::new(Element::new("groups",
        Column::new([Element::new("group", Group::titled("Settings",
            Element::new("label", Label::new("Ready"))))])
            .gap(MetricRole::SectionGap)))
        .padding(MetricRole::ContentMargin)))?;
ui.flush(std::time::Duration::ZERO)?;
ui.paint()?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

Custom components read `MeasureContext::metric` during layout or `Painter::metric`
for paint-only measurements. Changes invalidate just their readers and affected
layout dependents. Hidden components defer work until shown. No timer is added.

## Boundaries, shapes and application zoom

Groups have a rounded border even without a title or focus. `Group::frameless`
removes the outline explicitly. RegionLayout regions have a Surface background and
share their dividing line. Fixed boundaries keep that line; resizable boundaries
add a grip, hover feedback and a directional OS cursor. Locking preserves the
allocation. `Divider` is a noninteractive separator for ordinary content.
Tabs have a 45-degree upper-right cut, a selected underline and a separate focus
outline. Hit regions remain rectangular and do not overlap adjacent tabs.

Application zoom is independent of density and color mode, and defaults to 100%.
It accepts 75–200%. The desktop host combines it with native DPI automatically,
including pointer/pixel-scroll conversion and sharp text preparation. Physical
window size remains unchanged; content wraps or scrolls in the new logical space.
Zoom and geometry-affecting theme changes cancel active input before relayout.
Selection, stable widget IDs, physical key state and split preferences are retained.
Hosts own preference persistence and any zoom keyboard shortcuts.

```rust
use pert_ui_runtime::*;
let mut ui = Runtime::new();
ui.set_viewport(Viewport::new(800.0, 600.0, 1.5)?)?; // native size/DPI, before zoom
let settings = ui.ui_settings();
settings.set_zoom(UiZoom::new(1.25)?)?;
ui.mount_tree(Element::new("group", Group::titled("Appearance",
    Element::new("content", Column::new([
        Element::new("label", Label::new("Readable at any density")),
        Element::new("divider", Divider::new()),
        Element::new("reset", Button::new("Reset zoom", move |_| {
            settings.set_zoom(UiZoom::default()).map(|_| ())
        })),
    ])))))?;
ui.flush(std::time::Duration::ZERO)?;
assert_eq!(ui.paint()?.viewport().scale(), 1.875);
# Ok::<(), Box<dyn std::error::Error>>(())
```

Custom widgets use the same shape cache, focus and cursor contracts. This complete
example uses a theme size in layout and a focus-sensitive rounded frame. The
runtime caches shape coverage separately from its color, bounded to 128 entries
and 16 MiB. Cached display lists may retain evicted masks. This is UI-thread work.

```rust
use pert_ui_runtime::*;
struct CustomPanel;
impl Component for CustomPanel {
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> { Ok(()) }
    fn measure(&self, cx: &MeasureContext<'_>, _: Constraints) -> Result<Size, RuntimeError> {
        Ok(Size::new(180.0, cx.metric(MetricRole::ControlHeight))?)
    }
    fn focusable(&self) -> bool { true }
    fn cursor(&self, _: Point) -> Option<CursorIcon> { Some(CursorIcon::Default) }
    fn paint(&self, painter: &mut Painter) -> Result<(), DrawError> {
        if let Some(size) = painter.size() {
            let rect = Rect::from_size(size);
            let radius = Length::new(painter.metric(MetricRole::CornerRadius))?;
            let width = Length::new(painter.metric(MetricRole::BorderWidth))?;
            let role = if painter.focus_within() { ColorRole::Primary } else { ColorRole::PanelBorder };
            painter.fill_shape(rect, Shape::Rounded(radius), painter.color(ColorRole::Surface))?;
            painter.stroke_shape(rect, Shape::Rounded(radius), width, painter.color(role))?;
        }
        Ok(())
    }
}
let mut ui = Runtime::new();
ui.set_viewport(Viewport::new(300.0, 100.0, 1.0)?)?;
ui.mount_tree(Element::new("custom", CustomPanel))?;
ui.flush(std::time::Duration::ZERO)?;
ui.paint()?;
# Ok::<(), Box<dyn std::error::Error>>(())
```


## Numeric text entry

`IntegerInput` binds exact `i64` values; `DecimalInput` binds finite `f64` values.
Text stays local until Enter or focus loss saves a valid draft; Escape restores
its saved value. Hiding/removal cancels. Invalid text remains visible with an error
outline and never overwrites the number. External changed values win over drafts.
Ranges and keyboard steps are optional; Up/Down edits the draft. Decimal display
precision does not round the stored value, and both `.` and `,` are accepted as
decimal separators (no grouping). These are local signals, not host edit sessions;
`IntegerInput::for_parameter` edits a stepped parameter instead, validating drafts
exactly against its step and cancelling them on focus loss.

```rust
use pert_ui_runtime::{Column, DecimalInput, Element, IntegerInput, KeyAction, KeyCode, Runtime};
let mut ui = Runtime::new();
let count = ui.signal(12_i64);
let amount = ui.signal(1.234567_f64);
let root = ui.mount_tree(Element::new("numbers", Column::new([
    Element::new("count", IntegerInput::new(count.clone()).range(0, 100)?),
    Element::new("amount", DecimalInput::new(amount.clone()).decimal_places(2)?),
])))?;
let fields = ui.children(&root)?;
ui.focus(&fields[0])?;
ui.key_input(KeyCode::ArrowUp, KeyAction::Press)?;
ui.key_input(KeyCode::ArrowUp, KeyAction::Release)?;
assert_eq!(count.get()?, 12); // draft only
ui.focus(&fields[1])?; // saves count
assert_eq!(count.get()?, 13);
ui.clear_focus(); // unchanged rounded display preserves the full amount
assert_eq!(amount.get()?, 1.234567);
# Ok::<(), pert_ui_runtime::RuntimeError>(())
```

## Bounded updates and recovery

`Runtime::flush` limits update/settlement feedback to `Runtime::MAX_FLUSH_WAVES`,
including internal focus/hover/layout follow-ups. `RuntimeError::UpdateLimit`
preserves pending work for repair and explicit retry; development validation names
pending components. This does not bound the duration of a user callback or tree depth.
Custom hosts can call `Runtime::cancel_input` after a failed operation to clear
held actions and capture while preserving focus and numeric drafts without saving.

Escape in a text field cancels composition first, or collapses a selection and
ends dragging. With nothing to cancel it bubbles to ancestors. Numeric fields
retain their separate Escape contract: restore the saved value and consume the key.

Numeric Up/Down and wheel steps over the focused field display configured decimal
precision while retaining the exact numeric draft through undo/redo. Free typing
remains unrestricted. Shift divides arrow/wheel steps by ten (integer division,
minimum 1 for IntegerInput). A vertical wheel event steps once using the configured step;
up increases and down decreases, clamped to bounds. Enter/focus loss saves and
Escape cancels, as with keyboard steps. Unfocused fields preserve normal scrolling;
read-only, disabled, invalid and composing fields are not changed by wheel steps.

```rust
use pert_ui_runtime::{Element, IntegerInput, KeyAction, KeyCode, Point, PointerEvent,
            PointerKind, Runtime, Viewport};
let mut ui = Runtime::new();
ui.set_viewport(Viewport::new(240.0, 80.0, 1.0)?)?;
let value = ui.signal(10_i64);
let field = ui.mount_tree(Element::new("count", IntegerInput::new(value.clone()).step(2)?))?;
ui.focus(&field)?;
ui.pointer_input(PointerEvent::new(Point::new(20.0, 20.0)?,
    PointerKind::Scroll(Point::new(0.0, -40.0)?)))?;
assert_eq!(value.get()?, 10); // wheel changed only the local draft
ui.key_input(KeyCode::Enter, KeyAction::Press)?;
assert_eq!(value.get()?, 12);
# Ok::<(), Box<dyn std::error::Error>>(())
```


## Joined exclusive buttons

`SegmentedControl<T>` binds one `Signal<T>` to a horizontal row of equal-width
`Choice<T>` buttons. They share straight internal edges, with the same rounded
exterior corners as ordinary buttons. The selected segment uses `TabSelected`
with no underline or baseline beneath the group. Labels clip within constrained
segments.

The group has one Tab stop; arrows wrap past disabled options and Home/End choose
endpoints. Pointer activation requires matching press/release on the same value;
Space/Enter activate on release. Escape/context loss and dynamic option changes
cancel pending input. Absent external selections remain absent without rewriting
application state. Duplicate values are rejected. `SegmentedControl::bound(value,
options)` follows a reactive option list; `.disabled()` prevents editing. Optional `.on_change(...)` runs after a
user change, with the bound value already written even if the callback fails.
External updates and reselecting the same value do not invoke the callback.

```rust
use pert_ui_runtime::{Choice, Element, Runtime, SegmentedControl, ThemeMode};
let mut ui = Runtime::new();
let theme = ui.theme();
let mode = ui.signal(theme.mode()?);
ui.mount_tree(Element::new("appearance", SegmentedControl::new(mode, [
    Choice::new(ThemeMode::Dark, "Dark"),
    Choice::new(ThemeMode::Light, "Light"),
])?.on_change(move |_, value| theme.set_mode(*value).map(|_| ()))))?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Dialogs, menus and notifications

`Dialog` and `AlertDialog` own modal content inside the existing native window.
`DialogController` reserves one terminal outcome per opening. Take that outcome before
opening again; the owner must also have observed closure in a flush. Cancel,
acceptance, owner removal and failed admission remain distinguishable. Native
focus loss keeps dialogs open. No task action or I/O is implicit in dismissal.

```rust
use pert_ui_runtime::*;
use std::time::Duration;

let mut ui = Runtime::new();
ui.set_viewport(Viewport::new(640.0, 480.0, 1.0)?)?;
let confirmation = DialogController::new(&ui);
let show = confirmation.clone();
let root = ui.mount_tree(Element::new("confirmation", AlertDialog::new(
    confirmation.clone(),
    Element::new("open", Button::new("Delete item…", move |_| show.open())),
    "Delete item?", "This action requires confirmation.", "Delete",
)))?;
ui.flush(Duration::ZERO)?;
confirmation.open()?;
ui.flush(Duration::ZERO)?;
// Cancel is the initial focus target. Outside clicks do not dismiss.
ui.key_input(KeyCode::Escape, KeyAction::Press)?;
ui.key_input(KeyCode::Escape, KeyAction::Release)?;
ui.flush(Duration::ZERO)?;
assert_eq!(confirmation.take_outcome()?, Some(DialogOutcome::Cancelled));
ui.unmount(&root)?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

For arbitrary forms, use `Dialog::new(state, owner_content, dialog_content)`.
Its default scope is the window. `.view_modal()` instead blocks and clips within
the owner's allocated region: wrap the chosen view as `owner_content`. Nested
modals block their parents; conflicting independent scopes return `ModalConflict`
and record a failed outcome for the rejected session. `.require_response()`
consumes Escape until an explicit controller/button action completes the dialog.
Custom containers use the same public `Component::child_surface` contract and
`SurfaceKind` policies. `focus_gained` and `focus_lost` supply event-time lifecycle
notifications; physical held keys are never replayed on focus transfer.

```rust
use pert_ui_runtime::*;
use std::time::Duration;

let mut ui = Runtime::new();
ui.set_viewport(Viewport::new(640.0, 480.0, 1.0)?)?;
let state = DialogController::new(&ui);
let open = state.clone();
let cancel = state.clone();
let accept = state.clone();
let draft = ui.signal("Draft".to_owned());
ui.mount_tree(Element::new("view", Dialog::new(
    state,
    Element::new("owner", Column::new([
        Element::new("heading", Label::new("One independently blocked view")),
        Element::new("open", Button::new("Edit…", move |_| open.open())),
    ])),
    Element::new("form", Column::new([
        Element::new("draft", TextField::new(draft)),
        Element::new("actions", Row::new([
            Element::new("cancel", Button::new("Cancel", move |_| cancel.cancel().map(|_| ()))),
            Element::new("accept", Button::new("Accept", move |_| accept.accept().map(|_| ()))),
        ])),
    ])),
).view_modal()))?;
ui.flush(Duration::ZERO)?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

`MenuItem::submenu` composes keyed nested action lists. Right opens a branch;
Left/Escape returns one level; Enter/Space or a leaf click closes the whole chain
before its callback. Clicking an ancestor menu can directly select another item.
Lists skip disabled entries and scroll. `ContextMenu` opens on secondary press
at the pointer, or on the Menu key or Shift+F10 from focused content. Inputs
consumed by a custom child do not bubble to its surrounding context menu.

```rust
use pert_ui_runtime::*;
use std::time::Duration;

let mut ui = Runtime::new();
ui.set_viewport(Viewport::new(640.0, 480.0, 1.0)?)?;
let selected = ui.signal(false);
let write = selected.clone();
ui.mount_tree(Element::new("context", ContextMenu::new(
    &ui,
    Element::new("target", Button::new("Right-click or Shift+F10", |_| Ok(()))),
    vec![MenuItem::submenu("more", "More", vec![
        MenuItem::new("apply", "Apply", move |_| write.set(true)),
        MenuItem::new("unavailable", "Unavailable", |_| Ok(())).disabled(),
    ])],
)?))?;
ui.flush(Duration::ZERO)?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

Menu rows also cover separators, check-mark toggles, radio settings and
shortcuts. `MenuItem::command` presents a registered command with its live
shortcut and enabled binding, and runs it with the command context captured
when the menu opened. `MenuBar` arranges top-level submenus as a desktop menu
bar: titles open on press and switch while one is open, F10 focuses the titles,
arrows move between titles and menus, and leaving returns focus to the content.
Give the content to `MenuBar::content` so F10 reaches the bar from it. There are
no mnemonics and no native OS menus; see `docs/contracts.md` in the repository.

```rust
use pert_ui_runtime::*;
use std::time::Duration;

let mut ui = Runtime::new();
ui.set_viewport(Viewport::new(640.0, 480.0, 1.0)?)?;
let saved = ui.signal(false);
let write = saved.clone();
let save = ui.register_command("Save", CommandScope::Application,
    vec![KeyChord::primary('s')], move |_, _| write.set(true))?;
let grid = ui.signal(true);
let zoom = ui.signal(100u32);
let bar = MenuBar::new(&ui, vec![
    MenuItem::submenu("file", "File", vec![
        MenuItem::command(&ui, "save", &save, |_, _| Ok(()))?,
        MenuItem::separator(),
        MenuItem::new("close", "Close", |_| Ok(())),
    ]),
    MenuItem::submenu("view", "View", vec![
        MenuItem::toggle("grid", grid.clone(), "Show grid"),
        MenuItem::separator(),
        MenuItem::radio("normal", zoom.clone(), 100, "100%"),
        MenuItem::radio("large", zoom.clone(), 200, "200%"),
    ]),
])?
.content(Element::new("document", Button::new("Document", |_| Ok(()))));
let root = ui.mount_tree(Element::new("window", Column::new([Element::new("bar", bar)])))?;
ui.flush(Duration::ZERO)?;

// Focus the document, then F10, Down and Enter choose File › Save.
ui.focus_next(FocusDirection::Forward)?;
for key in [KeyCode::F10, KeyCode::ArrowDown, KeyCode::Enter] {
    ui.key_input(key, KeyAction::Press)?;
    ui.key_input(key, KeyAction::Release)?;
    ui.flush(Duration::ZERO)?;
}
ui.flush(Duration::ZERO)?;
assert!(saved.get()?);
assert_eq!(ui.focused().map(|id| ui.key(&id).map(str::to_owned)).transpose()?.as_deref(),
    Some("document"));
# let _ = root;
# Ok::<(), Box<dyn std::error::Error>>(())
```

`Banner` stays in ordinary layout until its visibility binding changes or its
owner is removed. `ToastHost` wraps application content and presents its queue
without stealing focus. Queue capacity is three visible plus sixteen pending;
overflow returns `ToastFull`. Text and action labels have a 16 KiB UTF-8 limit.
A started toast expires after five unpaused seconds on the runtime clock; pending
toasts do not age. Hover/focus pauses the visible stack at the runtime's delivered
input time. Hidden/occluded content schedules no presentation; started lifetimes
continue while hidden, and expired items are retired on resume. These timings do
not promise OS-event timestamp accuracy. Long content supports wheel scrolling and
keyboard selection. Enter/Space activates the selected action, Delete dismisses;
failed actions close once and report both `ActionFailed` and the runtime error.

```rust
use pert_ui_runtime::*;
use std::time::Duration;

let mut ui = Runtime::new();
ui.set_viewport(Viewport::new(640.0, 480.0, 1.0)?)?;
let banner_visible = ui.signal(true);
let hide_banner = banner_visible.clone();
let queue = ToastController::new(&ui);
let post = queue.clone();
let root = ui.mount_tree(Element::new("notifications", ToastHost::new(
    queue.clone(),
    Element::new("content", Column::new([
        Element::new("banner", Banner::new("Connection restored", [
            Element::new("dismiss", Button::new("Dismiss", move |_| hide_banner.set(false))),
        ]).visible_signal(banner_visible.read_only())),
        Element::new("notify", Button::new("Notify", move |_| {
            post.push(Toast::new("Saved").action("Show details", |_| Ok(())))?;
            Ok(())
        })),
    ])),
)))?;
let receipt = queue.push(Toast::new("Ready"))?;
ui.flush(Duration::ZERO)?;
assert_eq!(receipt.status(), ToastStatus::Visible);
ui.flush(Duration::from_secs(5))?;
assert_eq!(receipt.status(), ToastStatus::Expired);
ui.unmount(&root)?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Inline numeric rows

`InlineSlider`, `InlineInteger` and `InlineDecimal` are one-line controls with a
leading caption and a trailing value/unit, sized for toolbars, channel strips and
status bars. They are separate types from `Slider`, `IntegerInput` and
`DecimalInput`, not a mode of them, and they follow the ordinary theme metrics, so
density scales them like every other control. Click the value or press Enter/F2
to type; drag it horizontally to adjust. Focused arrows/wheel step; Shift adjusts
finely. The slider retains its parameter transactions. Local number drags save on
release and resting key/wheel steps save immediately; text drafts save on
Enter/focus loss. Escape cancels. Slider text cancels on focus loss, following
parameter ownership.

```rust
use pert_ui_runtime::{Column, Element, InlineDecimal, InlineInteger, InlineSlider,
    ParameterId, ParameterSpec, Runtime};
let mut ui = Runtime::new();
let count = ui.signal(8_i64);
let decimal = ui.signal(0.9_f64);
let (amount, _host) = ui.parameter(ParameterId::new(1, 1, "amount")?,
    ParameterSpec::new("Amount", 0.0, 2.0, 0.9)?)?;
ui.mount_tree(Element::new("controls", Column::new([
    Element::new("slider", InlineSlider::new(amount).label("Glide").unit("s")),
    Element::new("integer", InlineInteger::new(count).range(0, 100)?
        .label("Vol").unit("%")),
    Element::new("decimal", InlineDecimal::new(decimal).range(0.0, 2.0)?
        .decimal_places(2)?.label("Time").unit("s")),
])))?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

`NumberFormat<T>` supplies display text, editing text, parsing and an optional
value-dependent suffix, all in application-defined units. It remains UI-local;
canonical quantities and the parameter mapping retain their own validation.
Its width example reserves the entire value/unit slot, preventing adjacent
controls from moving as values change. Formatting never rounds stored values.

```rust
use pert_ui_runtime::{Element, InlineDecimal, NumberFormat, NumberInputError, Runtime};
let mut ui = Runtime::new();
let seconds = ui.signal(0.0009_f64);
let milliseconds = NumberFormat::new("9999.00 ms",
    |v: f64| format!("{:.2}", v * 1000.0),
    |s| s.trim().replace(',', ".").parse::<f64>().map(|v| v / 1000.0)
        .map_err(|_| NumberInputError::InvalidNumber))
    .edit_text(|v| (v * 1000.0).to_string());
ui.mount_tree(Element::new("delay", InlineDecimal::new(seconds)
    .range(0.0, 2.0)?.step(0.0001)?.label("Delay").unit("ms")
    .format(milliseconds)))?;
# Ok::<(), pert_ui_runtime::RuntimeError>(())
```

Text mode keeps the unit outside selection and IME content. Read-only local
number inputs still allow selection and copying; disabled inputs do not interact.
Custom format callbacks must be deterministic, bounded and free of side effects.
Their output uses the same text limits as other single-line editors. Longer output
clips to the reserved slot; editing scrolls within it. Neither resting focus nor
hidden controls schedule a caret timer.

## Inline controls and toolbars

The `Inline*` family are one-line controls for toolbars, channel strips, status
bars and browser headers. Each fits a ControlHeight row, follows the ordinary
theme metrics for density and zoom, owns one Tab stop and keeps arrow keys for
itself. Icon-only members carry an accessible name that inspection and delayed
help present. `InlineToggle` is a latched square button bound to a `Signal<bool>`;
`InlineChoice` is an exclusive selection with square icon segments; `InlineSelect`
is a caption, current value and chevron opening the `Select` list; `InlineText` is
an editable label whose draft saves on Enter or focus loss and cancels on Escape;
`InlineReadout` is a read-only caption/value/unit row; the numeric rows above
accept `stepper(true)` for minus/plus buttons. "Compact" refers only to
`Density::Compact`.

`Toolbar` hosts them in a strip with a SurfaceContainer fill and a PanelBorder
line. Groups (`ToolGroup::new`, whose controls are built and mounted once) move
behind a "More" button when the strip is narrower than its content, starting with
`collapse_first` groups, then from the trailing end and never an `essential`
group; `ToolbarOverflow::Wrap` wraps lines instead. A separator follows the group
before it and a spacer pushes later groups to the trailing edge. A collapsing
group is the same retained subtree moved to the overflow surface with
`UpdateContext::request_reparent`, keeping its state and its position among its
peers; it is hidden while that surface is closed, so an active gesture or draft
in it cancels under the hiding rules.
`ToolGroup::visible_signal` binds a group's presence to a `ReadSignal<bool>` for
contextual groups, such as the options of the selected tool: a hidden group
takes no space, never collapses and cannot be focused. Nest placement, not
toolbars: a global strip in a region layout's top slot and a local strip in a
view are siblings.

```rust
use pert_ui_runtime::{Button, Choice, Element, Icon, InlineChoice, InlineReadout, InlineSelect,
    InlineText, InlineToggle, Runtime, RuntimeError, ToolGroup, Toolbar};
let mut ui = Runtime::new();
let looping = ui.signal(false);
let snap = ui.signal("1/16");
let tool = ui.signal("select");
let name = ui.signal(String::from("Lead"));
let position = ui.signal(String::from("001:01:000"));
let toolbar = Toolbar::new(&ui, [
    ToolGroup::new("transport", [
        Element::new("play", Button::icon(Icon::PlayArrow, "Play", |_| Ok(())).frameless()),
    ]).essential(),
    ToolGroup::new("loop", [
        Element::new("loop", InlineToggle::new(looping, "Loop").icon(Icon::Repeat)),
    ]),
    ToolGroup::new("snap", [Element::new("snap",
        InlineSelect::new(&ui, snap, [Choice::new("1/4", "1/4"), Choice::new("1/16", "1/16")])
?
            .label("Snap"))]),
    ToolGroup::new("tools", [Element::new("mode",
        InlineChoice::new(tool, [
            Choice::icon("select", Icon::Edit, "Select"),
            Choice::icon("erase", Icon::Delete, "Erase"),
        ])?)]),
    ToolGroup::spacer("spacer"),
    ToolGroup::new("session", [
        Element::new("name", InlineText::new(name).placeholder("Untitled")),
        Element::new("pos", InlineReadout::new(position.read_only()).label("Pos")
            .width_hint("000:00:000")),
    ]).collapse_first(),
])?;
ui.mount_tree(Element::new("toolbar", toolbar))?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

The toolbar owns no commands: buttons carry their own activation and toggles
their own signals, so the host decides what play or loop mean. Overflow is
decided from the final allocation during layout and applied in the same flush
through `ArrangeContext::request_update`, a public hook custom containers can use
for the same purpose.

## Fonts, rich labels and icons

Body/heading text defaults to bundled Noto Sans, monospace to Noto Sans Mono, and
icons to Material Icons Outlined. The text fonts retain their SIL OFL 1.1 licenses;
the icon font and codepoint catalog retain Apache 2.0. The runtime package includes
all asset licenses and provenance. Distributing applications must retain applicable
font notices. No installation, network request or system font discovery is needed.

A runtime owns registrations. `FontData::from_static` borrows embedded bytes;
`FontData::from_owned` takes bytes loaded by the application. Single-face TTF/OTF
outline fonts are supported, including variable fonts at default axes. Collections,
web fonts and color-only glyphs are unsupported. Registration validates before
changing defaults and returns stable UI-local `FontId`s. Equal internal font names
do not overwrite earlier registrations. Up to `FontManager::MAX_CUSTOM_FONTS`
(256) custom registrations of at most `FontData::MAX_BYTES` (16 MiB) each are
admitted, within the `FontManager::MAX_TOTAL_BYTES` (64 MiB) total including
bundled fonts.

This complete example registers a separate copy of the bundled monospace font;
an application can substitute its own `FontData` without changing the other calls:

```rust
use pert_ui_runtime::{FontData, FontRole, Runtime};

let ui = Runtime::new();
let fonts = ui.fonts();
let alternate = fonts.register(FontData::bundled(FontRole::Monospace))?;
fonts.set_default(FontRole::Body, alternate.clone())?;
assert_eq!(fonts.default_font(FontRole::Heading)?, alternate);
fonts.reset_default(FontRole::Body)?;
# Ok::<(), pert_ui_runtime::RuntimeError>(())
```

Heading inherits Body until separately overridden. Explicit segment fonts take
precedence over roles. Ordinary text then falls back to the original bundled text
fonts; remaining missing characters use the missing-glyph mark. Registration alone
does not schedule work. Changed defaults remeasure only affected layout readers;
fixed-geometry `Painter` readers repaint. Hidden readers defer work. Handles are
weak, mutations obey the runtime's phase restrictions, and shutdown returns `Closed`.
Registrations remain immutable until shutdown; no unregister API is provided.

One `RichText` is a Unicode paragraph with styled segments, shared baseline,
bidirectional shaping and wrapping. Icons use IconSize, and `gap`/icon helpers use
IconGap. Span sizes are logical pixels before DPI and zoom. Preparation limits
include `TextEditError::MAX_TEXT_BYTES` (16 KiB) of UTF-8,
`RichText::MAX_SEGMENTS` (1,024) segments and the shared raster limits of
`TextLimit`. Palette
changes recolor semantic spans without reshaping their coverage. Existing plain
and rich captions share one `TextSource`, which also accepts a binding to
reactive rich content. Text fields continue to edit plain strings.

```rust
use pert_ui_runtime::{Button, ColorRole, Column, Element, FontRole, Icon, Label,
              RichText, Runtime, TextSpan, Viewport};
use std::time::Duration;

let mut ui = Runtime::new();
ui.set_viewport(Viewport::new(400.0, 160.0, 1.25)?)?;
let mono = ui.fonts().default_font(FontRole::Monospace)?;
let title = RichText::new().icon(Icon::Settings).gap().text("Settings ")
    .span(TextSpan::new("42.00").font(mono).color_role(ColorRole::Warning));
ui.mount_tree(Element::new("content", Column::new([
    Element::new("title", Label::new(title)),
    Element::new("save", Button::new("Save", |_| Ok(())).leading_icon(Icon::Save)),
])))?;
ui.flush(Duration::ZERO)?;
assert!(!ui.paint()?.masks().is_empty());
# Ok::<(), Box<dyn std::error::Error>>(())
```

Standard icons are semantic roles. Every built-in select/submenu arrow, tab overflow
arrow, checkbox and radio indicator uses the same public font-icon path. Slider
tracks, switch geometry and resize grips remain geometric controls.
`TextSource` arguments, `Choice::rich_label`, `MenuItem::label`,
`Tab::rich_title` and boolean-control `label` accept the same content. Icon-only buttons require an
inspection name; compose `Tooltip` to show that name on hover/focus:

```rust
use pert_ui_runtime::{Button, Element, Icon, Runtime, Tooltip};

let mut ui = Runtime::new();
ui.mount_tree(Element::new("close", Tooltip::new(
    Element::new("button", Button::icon(Icon::Close, "Close panel", |_| Ok(()))),
    "Close panel",
)))?;
# Ok::<(), pert_ui_runtime::RuntimeError>(())
```

An icon font swap needs a codepoint mapping because different fonts use different
characters for the same symbol. `icon_glyph` validates a character in one exact
registered font; no icon fallback occurs. `material_icon` resolves any snake_case
name in the bundled catalog through a setup-time linear lookup. `set_overrides`
atomically replaces all role overrides; omitted roles use their bundled glyphs.
Duplicate roles and foreign fonts reject the entire change.

```rust
use pert_ui_runtime::{Icon, Runtime};

let ui = Runtime::new();
let alternate = ui.fonts().material_icon("expand_circle_down")?;
ui.icons().set(Icon::ChevronDown, alternate)?;
// Existing selects and custom uses of ChevronDown follow this mapping.
ui.icons().set_overrides([])?;
# Ok::<(), pert_ui_runtime::RuntimeError>(())
```

Custom widgets use `MeasureContext::rich_text` when content affects layout,
`Painter::rich_text` inside fixed geometry, and `Painter::icon` to fit icon ink
uniformly into reserved bounds. All produce ordinary public `TextLayout`/mask
commands; no shaping or renderer dependency appears in their signatures.

This custom label uses only the public extension points and retains its measured
layout, just like the built-in label:

```rust
use pert_ui_runtime::{ArrangeContext, Component, Constraints, DrawError, Element, Icon,
    MeasureContext, Painter, RichText, Runtime, RuntimeError, Size, TextLayout,
    TextStyle, UpdateContext, Viewport, ColorRole, Point};
use std::{cell::RefCell, time::Duration};

struct Caption {
    content: RichText,
    layout: RefCell<Option<TextLayout>>,
}
impl Component for Caption {
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> { Ok(()) }
    fn measure(&self, cx: &MeasureContext<'_>, space: Constraints) -> Result<Size, RuntimeError> {
        Ok(cx.rich_text(&self.content, space.width(), TextStyle::Body)?.size())
    }
    fn arrange(&self, cx: &mut ArrangeContext<'_>, size: Size) -> Result<(), RuntimeError> {
        *self.layout.borrow_mut() = Some(cx.measurement().rich_text(
            &self.content, Some(size.width()), TextStyle::Body,
        )?);
        Ok(())
    }
    fn paint(&self, painter: &mut Painter) -> Result<(), DrawError> {
        if let Some(layout) = self.layout.borrow().as_ref() {
            layout.paint(painter, Point::ZERO, painter.color(ColorRole::OnSurface))?;
        }
        Ok(())
    }
}
let mut ui = Runtime::new();
ui.set_viewport(Viewport::new(240.0, 80.0, 1.0)?)?;
ui.mount_tree(Element::new("caption", Caption {
    content: RichText::from("Details").leading_icon(Icon::Info),
    layout: RefCell::new(None),
}))?;
ui.flush(Duration::ZERO)?;
assert!(!ui.paint()?.masks().is_empty());
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Floating windows inside a view

Windows overlap inside a `WindowLayer`. Decorations, close, movement and corner
resizing are defaults; minimization is optional. Sizes include the title bar and
frame, in logical pixels. Every content value is an ordinary `Element`. Focus a
frame and use Alt+arrows to move or Alt+Shift+arrows to resize it. Pointer drags
commit on release; Escape, focus loss or hiding cancels the tentative change.

```rust
use pert_ui_runtime::{Element, FloatingWindow, Label, Point, Runtime, Size, Tabs, Tab,
              TextField, View, Viewport, WindowLayer, WindowStatus};
use std::time::Duration;
let mut ui = Runtime::new();
ui.set_viewport(Viewport::new(800.0, 600.0, 1.0)?)?;
let editor = FloatingWindow::new(&ui, "editor", "Properties",
    Element::new("tabs", Tabs::new([
        Tab::new("general", "General", Element::new("name",
            TextField::new(ui.signal("Untitled".to_owned())))),
        Tab::new("details", "Details", Element::new("label", Label::new("Details"))),
    ])))
    .initial_size(Size::from([400, 280]))?
    .position(Point::new(40.0, 30.0)?)?
    .minimizable(true);
let controller = editor.controller();
let root = ui.mount_tree(Element::new("view", View::new(Element::new("windows",
    WindowLayer::new([editor])))))?;
ui.flush(Duration::ZERO)?;
let _drawing = ui.paint()?;
controller.minimize()?;
ui.flush(Duration::ZERO)?;
assert_eq!(controller.status().get()?, WindowStatus::Minimized);
assert!(ui.next_deadline().is_none());
controller.restore()?;
ui.flush(Duration::ZERO)?;
ui.unmount(&root)?;
assert_eq!(controller.status().get()?, WindowStatus::Closed);
# Ok::<(), Box<dyn std::error::Error>>(())
```

A fixed palette can supply a custom drag bar without standard decorations. Titles
use Button padding and vertical centering. Standard title icons have no resting
background; use `Button::frameless` or `Menu::icon` for matching custom actions.

```rust
use pert_ui_runtime::{Button, Element, FloatingWindow, Icon, Label, Point, Runtime, Size,
              Tooltip, WindowLayer, Viewport};
use std::time::Duration;
let mut ui = Runtime::new();
ui.set_viewport(Viewport::new(700.0, 500.0, 1.0)?)?;
let palette = FloatingWindow::new(&ui, "palette", "Tools",
    Element::new("save", Button::icon(Icon::Save, "Save", |_| Ok(()))))
    .decorated(false)
    .drag_area(Element::new("handle", Label::new("Drag tools")))
    .fixed_size(Size::from([240, 100]))?;
let inspector = FloatingWindow::new(&ui, "inspector", "Inspector",
    Element::new("content", Label::new("Ordinary window content")))
    .position(Point::new(160.0, 80.0)?)?
    .title_action(Element::new("info", Tooltip::new(
        Element::new("button", Button::icon(Icon::Info, "Information", |_| Ok(())).frameless()),
        "Information")));
ui.mount_tree(Element::new("layer", WindowLayer::new([palette, inspector])))?;
ui.flush(Duration::ZERO)?;
let _drawing = ui.paint()?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

Applications own save/confirmation decisions. `on_close` can defer acceptance;
`WindowController::close` later accepts closure without repeating that callback. Closing
removes the subtree at a safe flush (hidden owners process it when next visible).
Closing is terminal: use a new description and keyed reconciliation to reopen.

```rust
use pert_ui_runtime::{Element, FloatingWindow, Label, Runtime, Viewport, WindowCloseAction,
              WindowLayer, WindowStatus};
use std::time::Duration;
let mut ui = Runtime::new();
ui.set_viewport(Viewport::new(700.0, 500.0, 1.0)?)?;
let confirmation_requested = ui.signal(false);
let requested = confirmation_requested.clone();
let window = FloatingWindow::new(&ui, "document", "Document",
    Element::new("body", Label::new("Unsaved content")))
    .on_close(move |_| {
        requested.set(true)?;
        Ok(WindowCloseAction::KeepOpen)
    });
let controller = window.controller();
let layer = ui.mount_tree(Element::new("windows", WindowLayer::new([window])))?;
ui.flush(Duration::ZERO)?;
// After the application's confirmation/save operation succeeds:
controller.close()?;
ui.flush(Duration::ZERO)?;
assert_eq!(controller.status().get()?, WindowStatus::Closed);
assert!(ui.children(&layer)?.is_empty());
# Ok::<(), Box<dyn std::error::Error>>(())
```

Stacked custom containers use `Component::stacks_children`,
`Component::layout_position`, and `MeasureContext::position`. They share ordinary
`Painter`, clipping, input, identity, and lifecycle contracts. Floating windows
introduce no native window, event loop, worker endpoint, or GPU dependency.
