Content is a tree of keyed elements. Flow containers arrange siblings, views
bound regions of the window, and larger structures such as region layouts,
splits, tabs, floating windows and native windows keep each view's identity and
local state while the user rearranges them. The arrangement itself can be saved
as a versioned workspace document.

# Views, groups and region layouts

[`RegionLayout`] supplies optional full-width top/bottom regions, middle-band left/right
regions, and a filling center. Nest another region layout in any region without creating
a window, event loop or modal scope. Edges size to content; fixed edge extents
remain explicit semantic choices. Each region supplies both-axis overflow access.
Present neighboring regions have dividers, eight logical pixels wide at Standard
density and locked by default. At Standard density top/bottom viewport heights
have a 24-pixel minimum, left/right widths 64 pixels, and center 64 by 64. When these minima cannot fit, the region layout itself scrolls.
Missing regions add no divider. Give an edge a resize permission with `left_resizable`
(or another edge) to enable user resizing: `true`, `false` or a `ReadSignal<bool>`,
as a [`Permission`] takes. `resizable` restricts every edge at once, and the
stricter permission wins; locking during a drag restores the preceding preference.

```rust
use pert_ui::{Button, Element, Group, Label, RegionLayout, Runtime, Viewport};
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

[`View::visible_signal`] controls descendant visibility through a binding, preserving state
and suspending deadlines. [`ScrollArea`] defaults to vertical scrolling and reveals
focused descendants automatically. Wheel/arrows/page keys scroll; Home/End reach
the bounds. Thin indicators use retained foreground drawing above children.

Custom containers share [`Component::child_visible`], `can_reveal`, `reveal`,
`clips_children` and `paint_foreground`, plus [`ArrangeContext::place_clipped`]
for independent cell boundaries. Flow/view/group components allow overflow
to reach an enclosing scroll viewport; explicit [`Container`] and `ScrollArea` clip.
A component's own drawing still clips to its allocation. Successful arrangement
invalidates that component's drawing cache, including geometry-dependent foreground.
Unchanged branches retain layout/drawing. Focus traversal skips targets that remain
fully clipped after a reveal attempt. No periodic timer powers scrolling.

# Independent split dividers

[`SplitPane`] defaults to equal shares and enabled resizing. `LeftRight`/`TopBottom`
name child placement. Choose a ratio or a fixed first/second extent only when that
has meaning for the workspace. Locking is independent of sizing, so locked ratios
still follow window proportions and locked extents retain their preferred size.

```rust
use pert_ui::{Container, Element, Length, Runtime, SplitAxis, SplitPane, SplitSizing, Viewport};
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
[`Component::measure_limits`] contract. Infeasible minima remain scrollable inside
[`ScrollArea`]/[`RegionLayout`]; both maxima leave trailing spare space. Temporary clamping
never rewrites the preference. Unrepresentable aggregate minimums return a layout
error; upper bounds saturate at the largest representable logical extent.
Committed divider positions are part of the workspace document (see
[below](#workspaces)); undo of layout changes stays with the host.

Custom containers receive owned immediate [`ChildInfo`] snapshots in [`UpdateContext`];
structural changes schedule an owner update. [`Component::cancel_input`] allows a
custom interaction to restore layout state and invalidate it; its default calls
`input_reset` and invalidates paint, and an override must release held-key
actions itself, since the runtime does not call `input_reset` separately.

# Retained tabs and resizable edges

[`Tabs`] selects the first keyed [`Tab`] by default. Inactive content retains state
and suspends presentation deadlines. The themed title strip (minimum 36 logical pixels at Standard) scrolls overflowing
titles with arrows or the wheel; selected content has automatic overflow access.
Left/Right/Home/End navigate focused tabs; Ctrl+Tab also works from their content.
There are no close buttons by default. [`Tabs::reorderable`] lets the user move
titles by dragging or with Alt+arrows, and asks the application to accept each move. Selection survives reorder; removal chooses
the following neighbor, or the last remaining tab. Titles are display metadata,
independent of identity, and can change during keyed reconciliation.

```rust
use pert_ui::{Button, Element, Label, Runtime, RegionLayout, Tab, Tabs, Viewport};
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

# Native windows

`Tabs::detachable(true)` lets a title be dragged out of its strip and released to
open its retained view in an independent native window. A detachable strip reserves
an `OpenInNew` marker slot in every title, so the affordance appears without
reflowing the strip: hovering a docked title shows the marker in Muted and uses the
Move cursor, while a title being dragged out, and one whose view is already
detached, become emptied SurfaceContainer slots with a PanelBorder outline, a Muted
title and the marker in Primary. The marker is presentation only; the whole title
stays the hit area, and pressing a detached title activates its window instead of
duplicating content.

Tabs are not the only detachable unit. [`EventContext::detach_view`] and
[`Runtime::request_detach`] move any mounted view, so an application can offer a
button, a command or its own gesture for a panel that is not a tab. The view keeps
its incarnation, local state, bindings and scheduling; closing the window re-docks
it under the default [`DetachedClosePolicy`].

Detachment is permitted per item, and nothing is detachable by accident: a strip
offers the gesture only after `Tabs::detachable(true)`, and `Tab::detachable(false)`
excludes one title from a strip that does. Both are [`Element::with_detachable`],
which any view can set. A refusal is enforced centrally, so `prepare_detach`,
`request_detach` and `detach_view` all return [`RuntimeError::NotDetachable`] instead
of moving the view, and [`Tabs`] offers that title no marker, no Move cursor and no
drag-out. Reconciliation updates the permission without remounting; an already
detached view keeps its window and can still re-dock. Containers read the same
metadata through [`ChildInfo::detachable`].

With the `desktop` feature, [`App`] opens and closes the native windows these
requests describe; a custom host takes them with
[`Runtime::take_presentation_requests`] and completes each one. A detached tab or
tool window whose title changes asks for its open window to be retitled with
`PresentationAction::Retitle` ([`EventContext::set_window_title`]); a custom host
sets the native title and completes the request. Native multi-window support is
experimental; see [`NativeWindowOptions`] and `DetachedClosePolicy`.

# Content-sized splits and custom selection containers

[`SplitSizing::CONTENT_FIRST`] and `CONTENT_SECOND` follow child content until a
resize commits a fixed extent. Cancellation restores the content rule. RegionLayout
edges reuse this behavior and the same constrained split implementation.
Custom selection containers can use labeled [`Element`] descriptions,
[`UpdateContext::children`] snapshots and [`EventContext::request_focus`]; the latter
applies after updates/layout and ignores hidden, removed or nonfocusable targets.

# Fixed chrome with independent scrolling

A fixed frame starts at its allocation origin, preserving its menu and tab strip.
Tabs own the scrollable content beneath their strip. Fixed overflow clips below
hard minima; enlarge the allocation to recover clipped controls. `View::fixed()`
provides the same local clipping boundary for an individual view and
[`ScrollAxes::None`] selects it on a viewport. These do not escape scrolling ancestors.

```rust
use pert_ui::{Runtime, Viewport, RegionLayout, Element, Button, Tabs, Tab, Label};
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

# Floating windows inside a view

Windows overlap inside a [`WindowLayer`]. Decorations, close, movement and corner
resizing are defaults; minimization is optional. Sizes include the title bar and
frame, in logical pixels. Every content value is an ordinary [`Element`]. Focus a
frame and use Alt+arrows to move or Alt+Shift+arrows to resize it. Pointer drags
commit on release; Escape, focus loss or hiding cancels the tentative change.

```rust
use pert_ui::{Element, FloatingWindow, Label, Point, Runtime, Size, Tabs, Tab,
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
background; use [`Button::frameless`] or [`Menu::icon`] for matching custom actions.

```rust
use pert_ui::{Button, Element, FloatingWindow, Icon, Label, Point, Runtime, Size,
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
[`WindowController::close`] later accepts closure without repeating that callback. Closing
removes the subtree at a safe flush (hidden owners process it when next visible).
Closing is terminal: use a new description and keyed reconciliation to reopen.

```rust
use pert_ui::{Element, FloatingWindow, Label, Runtime, Viewport, WindowCloseAction,
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

Stacked custom containers use [`Component::stacks_children`],
[`Component::child_stacking_priority`], [`Component::layout_position`], and
[`MeasureContext::position`]. Priorities keep background content below floating
children; focus raises children within equal priorities. Request layout when
stacking or priorities change to refresh stationary hover and drag targets.
They share ordinary
[`Painter`], clipping, input, identity, and lifecycle contracts. Floating windows
introduce no native window, event loop, worker endpoint, or GPU dependency.

# Workspaces

[`Runtime::capture_workspace`] records the arrangement the user made, keyed by
element key paths: split and region divider positions, scroll positions, the
selected tab, floating window placement, table columns and zoom views, plus the
application zoom. [`Workspace::encode`] produces a separately versioned text
document, [`Workspace::decode`] validates one, and [`Runtime::restore_workspace`]
applies it and reports what it applied, deferred, ignored or rejected in a
[`WorkspaceReport`]. The document holds no application data, focus, active
gestures or detached native windows. Custom components take part through
[`Component::workspace_state`], which returns an error rather than `None`
when it cannot build its state: the capture then fails as a whole with
[`WorkspaceError::Participant`], naming that component's path, instead of
silently leaving it out. [Getting started](crate::guide::getting_started)
has a complete capture and restore.

# Large collections

[`VirtualList`] mounts only the items near its viewport, from a [`ListSource`]
whose [`ListItems`] answer the count, the key at a position and the element at a
position; keys, not positions, identify rows, so a row keeps its state and focus
across a change of the source. It lays out one or more lanes, horizontally or
vertically, with fixed or estimated extents. [`Table`] adds columns that can be
resized, sorted and moved, with its state in a [`TableController`]: a header
press or Alt+Up/Down on the active cell asks the application to sort, and the
controller fixes widths from code as a drag does. [`Tree`] shows a
[`TreeSource`] of nested rows whose expansion lives in a [`TreeController`]. All three
share [`ListSelection`] for single or multiple selection, and lists and trees can
be made reorderable by dragging.

# Zoomable content

[`ZoomView`] places components at content-space points and lets the user pan and
zoom them uniformly with the wheel, pinch gestures and the keyboard, within
[`ScaleLimits`]; a [`ZoomController`] reads and sets the view from application code,
serves one mounted view at a time, and the view is part of the workspace. Views that zoom one axis at a time, such
as timelines and plots, map their own positions with an [`AxisMapping`] per axis
and draw undistorted. Custom containers scale and offset their children through
[`Component::child_transform`] with a [`ViewTransform`]; clipping, hit testing and
focus reveal follow it.

# Drag and drop

A component starts a drag session from a held pointer button with
[`EventContext::start_drag`], offering a typed [`DragPayload`], the permitted
[`DropOperations`] and a [`DragPreview`]. Components under the pointer receive
[`Component::drag_event`] and answer with a [`DropResponse`]; on [`DragPhase::Drop`] the target
performs the application edit, and the source learns the outcome in
[`Component::drag_finished`]. Escape and focus loss cancel. Reorderable lists,
trees and tabs, and tab detachment, use the same public contract.


## Retained state notifications

`Accordion::on_change` receives the open keys and a `ChangeOrigin`, after layout
settles. User input, reveal, restoration and redescription can all change the
open set. Initial expansion and restoration before first preparation are silent.
A handler may write signals; failure is observable without rollback or replay.

FloatingWindow placement builders store initial description data, so construction
from `UpdateContext` does not write signals. Extracting the controller before
calling the builders still addresses that window; programmatic placement wins
over initial geometry. Same-key redescription keeps the mounted controller.

A `ListController` addresses one mounted list at a time, including hidden lists.
Reuse it for another list only after the prior one is removed. Committed scroll,
divider, region, label-column and window geometry is reported live through
`ScrollArea::on_scroll`, `SplitPane::on_resize`, `RegionLayout::on_resize`,
`PropertyGrid::on_resize`, `FloatingWindow::on_frame` and the `Geometry` kind of
`ToolWindowLayout::on_change`, each with its `ChangeOrigin`, once per flush and
never for previews or cancelled gestures; it also stays workspace and inspection
state.
