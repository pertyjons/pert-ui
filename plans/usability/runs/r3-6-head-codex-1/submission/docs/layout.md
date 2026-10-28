[pert_ui](../../index.html)::[guide](../index.html)

# Module layout Copy item path

[Source](../../../src/pert_ui/guide.rs.html#100)

Expand description

Layout and views: containers, region layouts, splits, tabs, windows,
workspaces, collections, zoomable content and drag and drop.

Content is a tree of keyed elements. Flow containers arrange siblings,
views bound regions of the window, and larger structures such as region
layouts, splits, tabs, floating windows and native windows keep each
view’s identity and local state while the user rearranges them. The
arrangement itself can be saved as a versioned workspace document.

## [§](#choosing-a-way)Choosing a way

**Children from data.** The first row is the way to start with for fixed
content; the others serve the need their last column names.

| Way | Use it when |
|----|----|
| Containers with fixed children, such as [`Column`](../../struct.Column.html "struct pert_ui::Column") and [`Row`](../../struct.Row.html "struct pert_ui::Row") | the structure does not depend on data |
| [`Each`](../../struct.Each.html "struct pert_ui::Each") | a short list of keyed items described from a binding, with no selection, keyboard navigation or filtering ([children from values](#children-from-values)) |
| [`VirtualList`](../../struct.VirtualList.html "struct pert_ui::VirtualList"), [`Table`](../../struct.Table.html "struct pert_ui::Table") or [`Tree`](../../struct.Tree.html "struct pert_ui::Tree") | a collection the user selects, filters, reorders or edits in place, or one too large to mount whole ([large collections](#large-collections)) |
| [`Dynamic`](../../struct.Dynamic.html "struct pert_ui::Dynamic") | one child whose description depends on a binding’s value |
| [`Show`](../../struct.Show.html "struct pert_ui::Show") | a child that exists only while a flag is true: hiding it unmounts it and drops its state |
| [`View::visible_signal`](../../struct.View.html#method.visible_signal "method pert_ui::View::visible_signal") | content a flag hides and shows, keeping its state and subscriptions while hidden |
| Advanced: [`Runtime::reconcile_children`](../../struct.Runtime.html#method.reconcile_children "method pert_ui::Runtime::reconcile_children") | code holding the runtime replaces a parent’s children itself |
| Advanced: [`UpdateContext::request_children`](../../struct.UpdateContext.html#method.request_children "method pert_ui::UpdateContext::request_children") | a custom component describes its own children ([custom widgets](../custom_widgets/index.html "mod pert_ui::guide::custom_widgets")) |

## [§](#views-groups-and-region-layouts)Views, groups and region layouts

[`RegionLayout`](../../struct.RegionLayout.html "struct pert_ui::RegionLayout")
supplies optional full-width top/bottom regions, middle-band left/right
regions, and a filling center. Nest another region layout in any region
without creating a window, event loop or modal scope. Edges size to
content; fixed edge extents remain explicit semantic choices. Each
region supplies both-axis overflow access. Present neighboring regions
have dividers, eight logical pixels wide at Standard density and locked
by default. At Standard density top/bottom viewport heights have a
24-pixel minimum, left/right widths 64 pixels, and center 64 by 64. When
these minima cannot fit, the region layout itself scrolls. Missing
regions add no divider. Give an edge a resize permission with
`left_resizable` (or another edge) to enable user resizing: `true`,
`false` or a `ReadSignal<bool>`, as a
[`Permission`](../../struct.Permission.html "struct pert_ui::Permission")
takes. `resizable` restricts every edge at once, and the stricter
permission wins; locking during a drag restores the preceding
preference.

``` rust
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
```

[`View::visible_signal`](../../struct.View.html#method.visible_signal "method pert_ui::View::visible_signal")
controls descendant visibility through a binding, preserving state and
suspending deadlines.
[`ScrollArea`](../../struct.ScrollArea.html "struct pert_ui::ScrollArea")
defaults to vertical scrolling and reveals focused descendants
automatically. Wheel/arrows/page keys scroll; Home/End reach the bounds.
Thin indicators use retained foreground drawing above children.

Custom containers share
[`Component::child_visible`](../../trait.Component.html#method.child_visible "method pert_ui::Component::child_visible"),
`can_reveal`, `reveal`, `clips_children` and `paint_foreground`, plus
[`ArrangeContext::place_clipped`](../../struct.ArrangeContext.html#method.place_clipped "method pert_ui::ArrangeContext::place_clipped")
for independent cell boundaries. Flow/view/group components allow
overflow to reach an enclosing scroll viewport; explicit
[`Container`](../../struct.Container.html "struct pert_ui::Container")
and `ScrollArea` clip. A component’s own drawing still clips to its
allocation. Successful arrangement invalidates that component’s drawing
cache, including geometry-dependent foreground. Unchanged branches
retain layout/drawing. Focus traversal skips targets that remain fully
clipped after a reveal attempt. No periodic timer powers scrolling.

## [§](#growing-shrinking-and-stretching-rows-and-columns)Growing, shrinking and stretching rows and columns

[`Element::with_grow`](../../struct.Element.html#method.with_grow "method pert_ui::Element::with_grow")
gives a child of a [`Row`](../../struct.Row.html "struct pert_ui::Row")
or [`Column`](../../struct.Column.html "struct pert_ui::Column") a share
of the space the container’s allocation leaves over, and
[`Align::Stretch`](../../enum.Align.html#variant.Stretch "variant pert_ui::Align::Stretch")
makes children fill the cross axis: a column’s width, or the height a
row is allocated. A row grown in a column therefore stretches tall
controls, such as a vertical
[`Slider`](../../struct.Slider.html "struct pert_ui::Slider") beside a
[`LevelMeter`](../../struct.LevelMeter.html "struct pert_ui::LevelMeter"),
over the whole strip. A row’s measured height stays its tallest child’s,
so it fills what its parent gives without asking for more; children keep
their own maximum and minimum heights. A custom container does the same
with the size its
[`Component::arrange`](../../trait.Component.html#method.arrange "method pert_ui::Component::arrange")
receives.

When a row’s width or a column’s height is bounded and the natural
lengths do not fit, the grown children give up the difference, by weight
and down to each one’s minimum. A row measures a child it shrank again
at its final width, so wrapped text makes the row taller and an ellipsis
label truncates. Children without a weight keep their length, so a row
of fixed content still overflows, which development validation reports.
[A progress column](#a-progress-column) puts a bar beside its percentage
in a narrow table cell.

``` rust
use pert_ui::{parameter::*, prelude::*};
use pert_ui::testing::*;
use pert_ui::Orientation;
use std::time::Duration;
let mut ui = Runtime::new();
ui.set_viewport(Viewport::new(200.0, 400.0, 1.0)?)?;
let (gain, _host) = ui.parameter(
    ParameterId::new(1, 1, "gain")?,
    ParameterSpec::new("Gain", 0.0, 1.0, 0.5)?,
)?;
let strip = ui.mount_tree(Element::new("strip", Column::new([
    Element::new("name", Label::new("Channel 1")),
    Element::new("controls", Row::new([
        Element::new("gain", Slider::new(gain).orientation(Orientation::Vertical)),
    ]).align(Align::Stretch)).with_grow(1),
])))?;
ui.flush(Duration::ZERO)?;
let fader = ui.find_descendant(&strip, "gain")?.ok_or("mounted")?;
let row = ui.find_descendant(&strip, "controls")?.ok_or("mounted")?;
assert_eq!(ui.bounds(&fader)?.map(|b| b.height()), ui.bounds(&row)?.map(|b| b.height()));
```

## [§](#independent-split-dividers)Independent split dividers

[`SplitPane`](../../struct.SplitPane.html "struct pert_ui::SplitPane")
defaults to equal shares and enabled resizing. `LeftRight`/`TopBottom`
name child placement. Choose a ratio or a fixed first/second extent only
when that has meaning for the workspace. Locking is independent of
sizing, so locked ratios still follow window proportions and locked
extents retain their preferred size.

``` rust
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
```

Drag the divider or focus it and use its axis arrows (8 pixels; Shift:
32). Release commits; Escape, context loss, locking or losing a child
cancels and restores the original preference. One input source owns a
resize at a time: arrow resizing is ignored during a pointer drag, and
pointer resize admission is ignored while resize keys remain held.
Arrows from focused child controls never resize an ancestor.

Hard minimums/maxima propagate through nested splits using the public,
cached
[`Component::measure_limits`](../../trait.Component.html#method.measure_limits "method pert_ui::Component::measure_limits")
contract. Infeasible minima remain scrollable inside
[`ScrollArea`](../../struct.ScrollArea.html "struct pert_ui::ScrollArea")/[`RegionLayout`](../../struct.RegionLayout.html "struct pert_ui::RegionLayout");
both maxima leave trailing spare space. Temporary clamping never
rewrites the preference. Unrepresentable aggregate minimums return a
layout error; upper bounds saturate at the largest representable logical
extent. Committed divider positions are part of the workspace document
(see [below](#workspaces)); undo of layout changes stays with the host.

Custom containers receive owned immediate
[`ChildInfo`](../../struct.ChildInfo.html "struct pert_ui::ChildInfo")
snapshots in
[`UpdateContext`](../../struct.UpdateContext.html "struct pert_ui::UpdateContext");
structural changes schedule an owner update.
[`Component::cancel_input`](../../trait.Component.html#method.cancel_input "method pert_ui::Component::cancel_input")
allows a custom interaction to restore layout state and invalidate it;
its default calls `input_reset` and invalidates paint, and an override
must release held-key actions itself, since the runtime does not call
`input_reset` separately.

## [§](#retained-tabs-and-resizable-edges)Retained tabs and resizable edges

[`Tabs`](../../struct.Tabs.html "struct pert_ui::Tabs") selects the
first keyed [`Tab`](../../struct.Tab.html "struct pert_ui::Tab") by
default. Inactive content retains state and suspends presentation
deadlines. The themed title strip (minimum 36 logical pixels at
Standard) scrolls overflowing titles with arrows or the wheel; selected
content has automatic overflow access. Left/Right/Home/End navigate
focused tabs; Ctrl+Tab also works from their content. There are no close
buttons by default.
[`Tabs::reorderable`](../../struct.Tabs.html#method.reorderable "method pert_ui::Tabs::reorderable")
lets the user move titles by dragging or with Alt+arrows, and asks the
application to accept each move. Selection survives reorder; removal
chooses the following neighbor, or the last remaining tab. Titles are
display metadata, independent of identity, and can change during keyed
reconciliation.

``` rust
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
```

## [§](#native-windows)Native windows

`Tabs::detachable(true)` lets a title be dragged out of its strip and
released to open its retained view in an independent native window. A
detachable strip reserves an `OpenInNew` marker slot in every title, so
the affordance appears without reflowing the strip: hovering a docked
title shows the marker in Muted and uses the Move cursor, while a title
being dragged out, and one whose view is already detached, become
emptied SurfaceContainer slots with a PanelBorder outline, a Muted title
and the marker in Primary. The marker is presentation only; the whole
title stays the hit area, and pressing a detached title activates its
window instead of duplicating content.

Tabs are not the only detachable unit.
[`EventContext::detach_view`](../../struct.EventContext.html#method.detach_view "method pert_ui::EventContext::detach_view")
and
[`Runtime::request_detach`](../../struct.Runtime.html#method.request_detach "method pert_ui::Runtime::request_detach")
move any mounted view, so an application can offer a button, a command
or its own gesture for a panel that is not a tab. The view keeps its
incarnation, local state, bindings and scheduling; closing the window
re-docks it under the default
[`DetachedClosePolicy`](../../enum.DetachedClosePolicy.html "enum pert_ui::DetachedClosePolicy").

Detachment is permitted per item, and nothing is detachable by accident:
a strip offers the gesture only after `Tabs::detachable(true)`, and
`Tab::detachable(false)` excludes one title from a strip that does. Both
are
[`Element::with_detachable`](../../struct.Element.html#method.with_detachable "method pert_ui::Element::with_detachable"),
which any view can set. A refusal is enforced centrally, so a host’s
`prepare_detach`, `request_detach` and `detach_view` all return
[`RuntimeError::NotDetachable`](../../enum.RuntimeError.html#variant.NotDetachable "variant pert_ui::RuntimeError::NotDetachable")
instead of moving the view, and
[`Tabs`](../../struct.Tabs.html "struct pert_ui::Tabs") offers that
title no marker, no Move cursor and no drag-out. Reconciliation updates
the permission without remounting; an already detached view keeps its
window and can still re-dock. Containers read the same metadata through
[`ChildInfo::detachable`](../../struct.ChildInfo.html#method.detachable "method pert_ui::ChildInfo::detachable").

With the `desktop` feature,
[`App`](../../struct.App.html "struct pert_ui::App") opens and closes
the native windows these requests describe; a custom host takes them
with
[`HostIntegration::take_presentation_requests`](../../host/trait.HostIntegration.html#tymethod.take_presentation_requests "method pert_ui::host::HostIntegration::take_presentation_requests")
and completes each one. A detached tab or tool window whose title
changes asks for its open window to be retitled with
`PresentationAction::Retitle`
([`EventContext::set_window_title`](../../struct.EventContext.html#method.set_window_title "method pert_ui::EventContext::set_window_title"));
a custom host sets the native title and completes the request. Native
multi-window support is experimental; see
[`NativeWindowOptions`](../../struct.NativeWindowOptions.html "struct pert_ui::NativeWindowOptions")
and `DetachedClosePolicy`.

## [§](#content-sized-splits-and-custom-selection-containers)Content-sized splits and custom selection containers

[`SplitSizing::CONTENT_FIRST`](../../struct.SplitSizing.html#associatedconstant.CONTENT_FIRST "associated constant pert_ui::SplitSizing::CONTENT_FIRST")
and `CONTENT_SECOND` follow child content until a resize commits a fixed
extent. Cancellation restores the content rule. RegionLayout edges reuse
this behavior and the same constrained split implementation. Custom
selection containers can use labeled
[`Element`](../../struct.Element.html "struct pert_ui::Element")
descriptions,
[`UpdateContext::children`](../../struct.UpdateContext.html#method.children "method pert_ui::UpdateContext::children")
snapshots and
[`EventContext::request_focus`](../../struct.EventContext.html#method.request_focus "method pert_ui::EventContext::request_focus");
the latter applies after updates/layout and ignores hidden, removed or
nonfocusable targets.

## [§](#fixed-chrome-with-independent-scrolling)Fixed chrome with independent scrolling

A fixed frame starts at its allocation origin, preserving its menu and
tab strip. Tabs own the scrollable content beneath their strip. Fixed
overflow clips below hard minima; enlarge the allocation to recover
clipped controls. `View::fixed()` provides the same local clipping
boundary for an individual view and
[`ScrollAxes::None`](../../enum.ScrollAxes.html#variant.None "variant pert_ui::ScrollAxes::None")
selects it on a viewport. These do not escape scrolling ancestors.

``` rust
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
```

## [§](#floating-windows-inside-a-view)Floating windows inside a view

Windows overlap inside a
[`WindowLayer`](../../struct.WindowLayer.html "struct pert_ui::WindowLayer").
Decorations, close, movement and corner resizing are defaults;
minimization is optional. Sizes include the title bar and frame, in
logical pixels. Every content value is an ordinary
[`Element`](../../struct.Element.html "struct pert_ui::Element"). Focus
a frame and use Alt+arrows to move or Alt+Shift+arrows to resize it.
Pointer drags commit on release; Escape, focus loss or hiding cancels
the tentative change.

``` rust
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
```

A fixed palette can supply a custom drag bar without standard
decorations. Titles use Button padding and vertical centering. Standard
title icons have no resting background; use
[`Button::frameless`](../../struct.Button.html#method.frameless "method pert_ui::Button::frameless")
or
[`Menu::icon`](../../struct.Menu.html#method.icon "associated function pert_ui::Menu::icon")
for matching custom actions.

``` rust
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
```

Applications own save/confirmation decisions. `on_close` can defer
acceptance;
[`WindowController::close`](../../struct.WindowController.html#method.close "method pert_ui::WindowController::close")
later accepts closure without repeating that callback. Closing removes
the subtree at a safe flush (hidden owners process it when next
visible). Closing is terminal: use a new description and keyed
reconciliation to reopen.

``` rust
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
```

Stacked custom containers use
[`Component::stacks_children`](../../trait.Component.html#method.stacks_children "method pert_ui::Component::stacks_children"),
[`Component::child_stacking_priority`](../../trait.Component.html#method.child_stacking_priority "method pert_ui::Component::child_stacking_priority"),
[`Component::layout_position`](../../trait.Component.html#method.layout_position "method pert_ui::Component::layout_position"),
and
[`MeasureContext::position`](../../struct.MeasureContext.html#method.position "method pert_ui::MeasureContext::position").
Priorities keep background content below floating children; focus raises
children within equal priorities. Request layout when stacking or
priorities change to refresh stationary hover and drag targets. That
refresh is a repeated pointer move
([`PointerEvent::is_refresh`](../../struct.PointerEvent.html#method.is_refresh "method pert_ui::PointerEvent::is_refresh")):
a custom container that moves children during a drag invalidates only
when the drag’s derived offset changes, as
[`Component::pointer_input`](../../trait.Component.html#method.pointer_input "method pert_ui::Component::pointer_input")
shows, or its own layout keeps producing the next move. They share
ordinary
[`Painter`](../../struct.Painter.html "struct pert_ui::Painter"),
clipping, input, identity, and lifecycle contracts.

Children that overlap on purpose must say so, or development validation
reports
[`ValidationCode::UnexpectedOverlap`](../../enum.ValidationCode.html#variant.UnexpectedOverlap "variant pert_ui::ValidationCode::UnexpectedOverlap").
A stacking parent permits overlap among its children, and a press or
focus then raises the child within its priority. For content whose order
is fixed by the application, such as shapes on a canvas in document
order, either return that order from
[`Component::child_stacking_priority`](../../trait.Component.html#method.child_stacking_priority "method pert_ui::Component::child_stacking_priority")
or leave stacking off and report
[`OverlapPolicy::Allow`](../../enum.OverlapPolicy.html#variant.Allow "variant pert_ui::OverlapPolicy::Allow")
in each overlapping child’s
[`ComponentDetails::validation`](../../struct.ComponentDetails.html#structfield.validation "field pert_ui::ComponentDetails::validation")
from
[`Component::inspect`](../../trait.Component.html#method.inspect "method pert_ui::Component::inspect").
Floating windows introduce no native window, event loop, worker
endpoint, or GPU dependency.

## [§](#workspaces)Workspaces

[`Runtime::capture_workspace`](../../struct.Runtime.html#method.capture_workspace "method pert_ui::Runtime::capture_workspace")
records the arrangement the user made, keyed by element key paths: split
and region divider positions, scroll positions, the selected tab,
floating window placement, table columns and zoom views, plus the
application zoom.
[`Workspace::encode`](../../struct.Workspace.html#method.encode "method pert_ui::Workspace::encode")
produces a separately versioned text document,
[`Workspace::decode`](../../struct.Workspace.html#method.decode "associated function pert_ui::Workspace::decode")
validates one, and
[`Runtime::restore_workspace`](../../struct.Runtime.html#method.restore_workspace "method pert_ui::Runtime::restore_workspace")
applies it and reports what it applied, deferred, ignored or rejected in
a
[`WorkspaceReport`](../../struct.WorkspaceReport.html "struct pert_ui::WorkspaceReport").
The document holds no application data, focus, active gestures or
detached native windows. Custom components take part through
[`Component::workspace_state`](../../trait.Component.html#method.workspace_state "method pert_ui::Component::workspace_state"),
which returns an error rather than `None` when it cannot build its
state: the capture then fails as a whole with
[`WorkspaceError::Participant`](../../enum.WorkspaceError.html#variant.Participant "variant pert_ui::WorkspaceError::Participant"),
naming that component’s path, instead of silently leaving it out.
[Getting
started](../getting_started/index.html "mod pert_ui::guide::getting_started")
has a complete capture and restore.

## [§](#children-from-values)Children from values

[`Dynamic`](../../struct.Dynamic.html "struct pert_ui::Dynamic"),
[`Show`](../../struct.Show.html "struct pert_ui::Show") and
[`Each`](../../struct.Each.html "struct pert_ui::Each") describe
children from a binding, so a view whose structure depends on data needs
no custom component. Each describes its children again only when the
value changes; children that keep their key keep their state and focus.
[`Show`](../../struct.Show.html "struct pert_ui::Show") unmounts its
child while the flag reads false, whereas
[`View::visible_signal`](../../struct.View.html#method.visible_signal "method pert_ui::View::visible_signal")
hides content and keeps it.

``` rust
use std::time::Duration;
use pert_ui::prelude::*;

let mut ui = Runtime::new();
let tasks = ui.signal(vec!["Write the plan".to_owned()]);
let details = ui.signal(false);
let (add, toggle) = (tasks.clone(), details.clone());
let view = Column::new([
    Each::new(&tasks, Clone::clone, |_, task| Label::new(task.as_str())).key("tasks"),
    Show::new(&details, |_| Label::new("Details are shown").key("text")).key("details"),
    // A task's text is its key here, so each new task gets distinct text.
    Button::new("Add", move |_| {
        add.update(|tasks| tasks.push(format!("Task {}", tasks.len() + 1)))
    })
    .key("add"),
    Button::new("Details", move |_| toggle.toggle()).key("toggle"),
]);
let root = ui.mount_tree(view.key("content"))?;
ui.flush(Duration::ZERO)?;
tasks.update(|tasks| tasks.push("Ship it".to_owned()))?;
ui.flush(Duration::ZERO)?;
assert!(ui.find(&root, &["tasks", "Ship it"])?.is_some());
```

`Each` describes every item and offers no selection, keyboard navigation
or filtering. For a list the user selects, filters, reorders or edits in
place, use a
[`VirtualList`](../../struct.VirtualList.html "struct pert_ui::VirtualList"),
as [an editable list of records](#an-editable-list-of-records) shows.

## [§](#large-collections)Large collections

[`VirtualList`](../../struct.VirtualList.html "struct pert_ui::VirtualList")
mounts only the items near its viewport, from a
[`ListSource`](../../struct.ListSource.html "struct pert_ui::ListSource")
whose
[`ListItems`](../../trait.ListItems.html "trait pert_ui::ListItems")
answer the count, the key at a position and the element at a position;
keys, not positions, identify rows, so a row keeps its state and focus
across a change of the source. The list keys each element it builds with
the key at its position. It lays out one or more lanes, horizontally or
vertically, with fixed or estimated extents.
[`Table`](../../struct.Table.html "struct pert_ui::Table") adds columns
that can be resized, sorted and moved, with its state in a
[`TableController`](../../struct.TableController.html "struct pert_ui::TableController"):
a header press or Alt+Up/Down on the active cell asks the application to
sort, and the controller fixes widths from code as a drag does.
[`Tree`](../../struct.Tree.html "struct pert_ui::Tree") shows a
[`TreeSource`](../../struct.TreeSource.html "struct pert_ui::TreeSource")
of nested rows whose expansion lives in a
[`TreeController`](../../struct.TreeController.html "struct pert_ui::TreeController").
All three share
[`ListSelection`](../../struct.ListSelection.html "struct pert_ui::ListSelection")
for single or multiple selection, and lists and trees can be made
reorderable by dragging.

A list takes a binding of a
[`ListSource`](../../struct.ListSource.html "struct pert_ui::ListSource"),
not the
[`ListItems`](../../trait.ListItems.html "trait pert_ui::ListItems")
value itself: wrap the rows with
[`ListSource::new`](../../struct.ListSource.html#method.new "associated function pert_ui::ListSource::new")
and hold the result in a signal, `ui.signal(ListSource::new(rows))`,
then pass that signal to
[`VirtualList::new`](../../struct.VirtualList.html#method.new "associated function pert_ui::VirtualList::new"),
or pass a derived binding such as
[`ListFilter::items`](../../struct.ListFilter.html#method.items "method pert_ui::ListFilter::items").
Writing a new `ListSource` to the signal is how the content changes.

A collection mounts the rows inside the height it is allocated, so it
needs a bounded one. A window or detached view, a
[`SplitPane`](../../struct.SplitPane.html "struct pert_ui::SplitPane")
pane, a
[`Container`](../../struct.Container.html "struct pert_ui::Container")
or
[`CollectionFilter`](../../struct.CollectionFilter.html "struct pert_ui::CollectionFilter")
with a bounded allocation, a
[`Tab::fixed`](../../struct.Tab.html#method.fixed "method pert_ui::Tab::fixed")
tab, a
[`RegionLayout::fixed`](../../struct.RegionLayout.html#method.fixed "method pert_ui::RegionLayout::fixed")
region, a
[`View::fixed`](../../struct.View.html#method.fixed "method pert_ui::View::fixed")
view and a grown child of a bounded
[`Column`](../../struct.Column.html "struct pert_ui::Column") give one:
a grown child of a column shrinks to what the other children leave, so a
toolbar above a grown collection works. Default
[`Tab::new`](../../struct.Tab.html#method.new "associated function pert_ui::Tab::new")
and
[`RegionLayout`](../../struct.RegionLayout.html "struct pert_ui::RegionLayout")
content and a
[`ScrollArea`](../../struct.ScrollArea.html "struct pert_ui::ScrollArea")
do not, since they scroll content at its natural size, which for a
collection is its whole content; in debug builds validation reports such
a collection as
[`ValidationCode::UnboundedCollection`](../../enum.ValidationCode.html#variant.UnboundedCollection "variant pert_ui::ValidationCode::UnboundedCollection").
A bounded [`Row`](../../struct.Row.html "struct pert_ui::Row") offers
its height to every child, so it bounds a vertical collection beside
other children, and a grown horizontal collection gives up the width its
siblings need, so a bounded row bounds a grown horizontal one too.

## [§](#large-data)Large data

Virtualization bounds the components; the application’s data and the
collection’s bookkeeping still cost in proportion to the rows they
touch:

- Replacing a source
  ([`TableController::set_source`](../../struct.TableController.html#method.set_source "method pert_ui::TableController::set_source"),
  a new
  [`ListSource`](../../struct.ListSource.html "struct pert_ui::ListSource")
  in the binding,
  [`TreeController::set_source`](../../struct.TreeController.html#method.set_source "method pert_ui::TreeController::set_source"))
  rebuilds the descriptions of the mounted rows, and an active filter
  reads every row’s text again. A tree source’s replacement also
  flattens its expanded branches again, asking each expanded node for
  its children. Coalesce appended data into batches, keep one source per
  view, and do not replace a source from a selection callback that
  changed nothing it shows.
- [`ListItems::index_of`](../../trait.ListItems.html#method.index_of "method pert_ui::ListItems::index_of")
  and
  [`TableItems::index_of`](../../trait.TableItems.html#method.index_of "method pert_ui::TableItems::index_of")
  default to a scan of every row, and reveal, anchoring and the active
  row call them; keep a reverse index from key to position in a large
  source.
- A filter joins each row’s
  [`TableItems::text`](../../trait.TableItems.html#method.text "method pert_ui::TableItems::text")
  cells into one string per row; offer the text a user searches for, and
  nothing more.
- [`TreeItems::children`](../../trait.TreeItems.html#tymethod.children "method pert_ui::TreeItems::children")
  returns every child key, and expanding a node indexes all of them;
  very wide nodes cost in proportion to their children.
- A table builds every column of each mounted row, also the columns
  scrolled out of view.

The [log explorer
example](https://github.com/pertyjons/pert-ui/blob/main/crates/pert-ui/examples/log_explorer/main.rs)
puts this together over a generated log of hundreds of thousands of
lines: bounded tables and a tree, row identities with a reverse index,
loading in batches on a worker, appended lines inserted into each
table’s order instead of rebuilding it, a live tail that follows the
last shown row, an Errors view that detaches, copying per table and
selectable detail text. Its headless session reports mounted counts and
flush times and checks that the counts stay bounded:
`cargo run -p pert-ui --example log_explorer --no-default-features`, or
open it with `cargo run -p pert-ui --example log_explorer -- --window`.

## [§](#an-editable-list-of-records)An editable list of records

The application keeps its records in its own value, here a
`Signal<Vec<Task>>`, and the list shows them through a
[`ListItems`](../../trait.ListItems.html "trait pert_ui::ListItems")
implementation over a copy of that value. Each piece then has one job:

- **Source.**
  [`ReadSignal::map`](../../struct.ReadSignal.html#method.map "method pert_ui::ReadSignal::map")
  wraps the records in a new
  [`ListSource`](../../struct.ListSource.html "struct pert_ui::ListSource")
  whenever they change, keyed by each record’s identity, never its
  position, and offering its title as
  [`ListItems::text`](../../trait.ListItems.html#method.text "method pert_ui::ListItems::text")
  for the filter.
- **Filter.** A
  [`ListFilter`](../../struct.ListFilter.html "struct pert_ui::ListFilter")
  over that source, shown by a
  [`CollectionFilter`](../../struct.CollectionFilter.html "struct pert_ui::CollectionFilter")
  field, publishes the rows that match in
  [`ListFilter::items`](../../struct.ListFilter.html#method.items "method pert_ui::ListFilter::items"),
  which the list binds instead of the source.
- **Selection and actions.** A
  [`ListSelection`](../../struct.ListSelection.html "struct pert_ui::ListSelection")
  makes the list a Tab stop that the arrows navigate;
  [`VirtualList::on_activate`](../../struct.VirtualList.html#method.on_activate "method pert_ui::VirtualList::on_activate")
  runs on Enter, here to rename the active row, and
  [`VirtualList::reorderable`](../../struct.VirtualList.html#method.reorderable "method pert_ui::VirtualList::reorderable")
  receives drags and Alt with an arrow as one
  [`ListDrop`](../../struct.ListDrop.html "struct pert_ui::ListDrop")
  whose position names keys, which the callback applies to the records.
- **Hidden selection.** A filter never prunes the selection. While the
  list has focus it moves the active row off a hidden row; while the
  user types in the field,
  [`CollectionFilter::hidden_active`](../../struct.CollectionFilter.html#method.hidden_active "method pert_ui::CollectionFilter::hidden_active")
  decides, and
  [`HiddenActive::Clear`](../../enum.HiddenActive.html#variant.Clear "variant pert_ui::HiddenActive::Clear")
  leaves no row active once the active row is hidden. Application writes
  and a filter that hides every row while the list has focus can still
  leave a hidden active row, so the Delete button below stays guarded:
  deriving `ListFilter::items()` with
  [`ListSelection::active`](../../struct.ListSelection.html#method.active "method pert_ui::ListSelection::active")
  through
  [`ReadSignal::combine`](../../struct.ReadSignal.html#method.combine "associated function pert_ui::ReadSignal::combine")
  and
  [`ListItems::index_of`](../../trait.ListItems.html#method.index_of "method pert_ui::ListItems::index_of")
  tells whether the active row is shown, and Delete is enabled only
  then.
- **Renaming.** The title is an
  [`InlineText`](../../struct.InlineText.html "struct pert_ui::InlineText")
  in [row
  mode](../../struct.InlineText.html#method.in_row "method pert_ui::InlineText::in_row"):
  a single press on it selects the row, as a press elsewhere in the row
  does, and a double click renames it. Enter on the list renames the
  active row: `on_activate` finds the row’s title below the list and
  sends it
  [`TextCommand::BeginEdit`](../../enum.TextCommand.html#variant.BeginEdit "variant pert_ui::TextCommand::BeginEdit").
  Enter in the editor saves and gives focus back to the list.

[`ListItems::build`](../../trait.ListItems.html#tymethod.build "method pert_ui::ListItems::build")
has no state owner, and the list describes all its mounted rows again
whenever a change of the records replaces the source, and also whenever
a scroll or resize mounts or unmounts another row. The row’s checkbox
and title are therefore
[controlled](../../struct.Checkbox.html#method.controlled "associated function pert_ui::Checkbox::controlled"):
each takes the record’s value from the row’s description and proposes
the user’s change to its callback, which writes it to the records. They
bind nothing, so describing the row again with the same value leaves an
open draft alone, and the changed record shows after the next flush. The
row’s
[`ProgressBar`](../../struct.ProgressBar.html "struct pert_ui::ProgressBar")
is a display control: it takes the record’s status with
[`ProgressBar::new`](../../struct.ProgressBar.html#method.new "associated function pert_ui::ProgressBar::new")
and needs neither a binding nor a callback, so a new status in the
records shows after the next flush too. An editing control without a
controlled form uses one of the two advanced forms
[`ListItems::build`](../../trait.ListItems.html#tymethod.build "method pert_ui::ListItems::build")
describes: a per-row binding from
[`Signal::sibling`](../../struct.Signal.html#method.sibling "method pert_ui::Signal::sibling"),
which suits a control without a draft, since every description of the
rows gives it a new binding, or a retained row component, which makes
its binding once and writes each new value to it, so a draft survives.

``` rust
use pert_ui::prelude::*;
use std::mem;

// Signal values are compared when written and cloned when read.
#[derive(Clone, PartialEq)]
struct Task { id: u64, title: String, done: bool, progress: ProgressStatus }

/// The records as rows: a copy for reading, and the signal to write back to.
struct Rows { tasks: Vec<Task>, store: Signal<Vec<Task>> }
impl ListItems for Rows {
    fn len(&self) -> usize { self.tasks.len() }
    fn key(&self, index: usize) -> Option<String> {
        Some(self.tasks.get(index)?.id.to_string())
    }
    fn build(&self, index: usize) -> Option<Element> {
        let task = self.tasks.get(index)?;
        // Controlled: the row's description carries the record's values.
        let (store, id) = (self.store.clone(), task.id);
        let check = Checkbox::controlled(task.done, "Done", move |_, done| {
            edit(&store, id, |task| task.done = done)
        });
        let store = self.store.clone();
        // In row mode a single press reaches the row and selects it.
        let title = InlineText::controlled(task.title.clone(), move |_, title| {
            edit(&store, id, |task| task.title = title.to_owned())
        })
        .in_row(true);
        // A display control takes the record's value itself.
        let progress = ProgressBar::new(task.progress);
        // The list keys the row with the task's key, replacing "row".
        Some(Row::new([
            check.key("done"),
            title.key("title").with_grow(1),
            progress.key("progress"),
        ]).key("row"))
    }
    fn text(&self, index: usize) -> Option<String> {
        Some(self.tasks.get(index)?.title.clone())
    }
}

fn edit(store: &Signal<Vec<Task>>, id: u64, change: impl FnOnce(&mut Task)) -> Result<(), RuntimeError> {
    store.update(|tasks| {
        if let Some(task) = tasks.iter_mut().find(|task| task.id == id) {
            change(task);
        }
    })
}

/// Applies a drop to the records: the dropped tasks move, in their order, to
/// the position, which names a key of the rows shown.
fn reorder(store: &Signal<Vec<Task>>, drop: &ListDrop<'_>) -> Result<bool, RuntimeError> {
    let (mut moved, mut rest): (Vec<Task>, Vec<Task>) = store
        .get()?
        .into_iter()
        .partition(|task| drop.items().contains(&task.id.to_string()));
    let at = |key: &str| rest.iter().position(|task| task.id.to_string() == key);
    let index = match drop.position() {
        ListDropPosition::Before(key) => at(key),
        ListDropPosition::After(key) => at(key).map(|index| index + 1),
        ListDropPosition::End => Some(rest.len()),
        _ => None,
    };
    let Some(index) = index else { return Ok(false) }; // refused
    rest.splice(index..index, mem::take(&mut moved));
    store.set(rest)?;
    Ok(true)
}

/// What the test below reaches: the records, the mounted root, the selection,
/// the filter and whether the active row is shown.
struct TaskList {
    tasks: Signal<Vec<Task>>,
    root: WidgetId,
    selection: ListSelection,
    filter: ListFilter,
    selected_shown: ReadSignal<bool>,
}

fn build(ui: &mut Runtime) -> Result<TaskList, RuntimeError> {
    let task = |id, title: &str| Task {
        id,
        title: title.to_owned(),
        done: false,
        progress: ProgressStatus::Idle,
    };
    let tasks = ui.signal(vec![
        task(1, "Buy milk"),
        task(2, "Write the plan"),
        task(3, "Ship it"),
    ]);
    let store = tasks.clone();
    let source = tasks.map(move |all| {
        ListSource::new(Rows { tasks: all.clone(), store: store.clone() })
    })?;
    let selection = ListSelection::new(ui, SelectionMode::Single);
    let filter = ListFilter::new(ui, source)?.selection(selection.clone());
    let selected_shown =
        ReadSignal::combine((filter.items(), selection.active()), |(rows, active)| {
            active.as_deref().is_some_and(|key| rows.index_of(key).is_some())
        })?;
    let (order, remove, active) = (tasks.clone(), tasks.clone(), selection.active());
    let list = VirtualList::new(filter.items(), 32.0)?
        .selection(selection.clone())
        // Enter renames the active row. The callback's context is the list's, and
        // the row's title is found below it by the row's key.
        .on_activate(|cx, key| {
            let (list, key) = (cx.id(), key.to_owned());
            cx.defer(move |ui| {
                let Some(title) = ui.find(&list, &[key.as_str(), "title"])? else {
                    return Ok(());
                };
                ui.text_command(&title, TextCommand::BeginEdit).map(|_| ())
            })?;
            Ok(())
        })
        .reorderable(move |_, drop| reorder(&order, drop));
    let delete = Button::new("Delete", move |_| {
        let Some(key) = active.get()? else { return Ok(()) };
        remove.update(|tasks| tasks.retain(|task| task.id.to_string() != key))
    })
    .enabled(&selected_shown);
    let root = ui.mount_tree(Column::new([
        CollectionFilter::new(ui, filter.clone(), Element::new("list", list))
            .hidden_active(HiddenActive::Clear)
            .key("tasks")
            .with_grow(1),
        delete.key("delete"),
    ]).key("app"))?;
    Ok(TaskList { tasks, root, selection, filter, selected_shown })
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    use pert_ui::testing::*;
    use std::time::Duration;

    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(400.0, 300.0, 1.0)?)?;
    let TaskList { tasks, root, selection, filter, selected_shown } = build(&mut ui)?;
    ui.flush(Duration::ZERO)?;
    let press = |ui: &mut Runtime, key| -> Result<(), RuntimeError> {
        ui.key_input(key, KeyAction::Press)?;
        ui.key_input(key, KeyAction::Release)?;
        Ok(())
    };
    let ids = |tasks: &Signal<Vec<Task>>| -> Result<Vec<u64>, RuntimeError> {
        Ok(tasks.get()?.iter().map(|task| task.id).collect())
    };

    // Select the first row and move it down with Alt+Down.
    let list = ui.find(&root, &["tasks", "list"])?.ok_or("the list")?;
    ui.focus(&list)?;
    press(&mut ui, KeyCode::ArrowDown)?;
    ui.key_input(KeyCode::AltLeft, KeyAction::Press)?;
    press(&mut ui, KeyCode::ArrowDown)?;
    ui.key_input(KeyCode::AltLeft, KeyAction::Release)?;
    assert_eq!(ids(&tasks)?, [2, 1, 3]);

    // Enter renames the active row once the next flush runs the deferred
    // command. The draft survives a change to another record, and Enter saves
    // it and gives focus back to the list.
    press(&mut ui, KeyCode::Enter)?;
    ui.flush(Duration::ZERO)?;
    ui.text_input("Buy oat milk")?; // replaces the selected title
    edit(&tasks, 3, |task| task.done = true)?;
    ui.flush(Duration::ZERO)?;
    press(&mut ui, KeyCode::Enter)?;
    ui.flush(Duration::ZERO)?;
    assert_eq!(tasks.get()?[1].title, "Buy oat milk");
    assert_eq!(ui.focused(), Some(list.clone()));

    // The checkbox proposes the change, which its callback writes to the record.
    let check = ui.find(&root, &["tasks", "list", "1", "done"])?.ok_or("a checkbox")?;
    ui.focus(&check)?;
    press(&mut ui, KeyCode::Space)?;
    assert!(tasks.get()?[1].done);

    // A new status in the records shows in the row's bar after the flush.
    edit(&tasks, 2, |task| task.progress = ProgressStatus::Succeeded)?;
    ui.flush(Duration::ZERO)?;
    let bar = ui.find(&list, &["2", "progress"])?.ok_or("a progress bar")?;
    let described = Driver::new(&mut ui).semantics(&bar)?.description;
    assert_eq!(described.as_deref(), Some("succeeded"));

    // A press on another row's title selects that row; a double click on a
    // title renames it. The driver's clicks are one frame apart.
    let title = |ui: &Runtime, key| ui.find(&root, &["tasks", "list", key, "title"]);
    let write = title(&ui, "2")?.ok_or("a title")?;
    let ship = title(&ui, "3")?.ok_or("a title")?;
    let mut driver = Driver::new(&mut ui);
    driver.click(&write)?;
    assert_eq!(selection.active().get()?.as_deref(), Some("2"));
    driver.click(&ship)?;
    driver.click(&ship)?;
    driver.type_text("Ship it today")?;
    driver.key(KeyCode::Enter)?;
    assert_eq!(tasks.get()?[2].title, "Ship it today");
    assert_eq!(selection.active().get()?.as_deref(), Some("3"));

    // Typing in the field hides the active row, which leaves no row active,
    // so Delete is disabled.
    let field = driver.find(&root, &["tasks", "filter-field", "filter"])?;
    driver.runtime_mut().focus(&field)?;
    driver.type_text("milk")?;
    assert_eq!(filter.counts().get()?.shown, 1);
    assert_eq!(selection.active().get()?, None);
    assert!(!selected_shown.get()?);
    Ok(())
}
```

A single press on a title selects its row, a double click renames it,
and Enter on the list renames the active row; with the label’s default
press behavior a press would focus the title instead and the row would
never see it. The [inventory
example](https://github.com/pertyjons/pert-ui/blob/main/crates/pert-ui/examples/inventory/main.rs)
applies the controlled checkbox to a table’s checkbox column, and a
[`Table`](../../struct.Table.html "struct pert_ui::Table") edits text
cells in place through its own editor instead.

## [§](#a-progress-column)A progress column

A
[`ProgressBar`](../../struct.ProgressBar.html "struct pert_ui::ProgressBar")
prefers three themed control widths, 192 logical pixels at Standard,
which a table cell is often narrower than once a percentage sits beside
it. With a growth weight in a
[`Row`](../../struct.Row.html "struct pert_ui::Row") beside the label,
the bar takes what the label leaves and, in a narrower cell, gives up
the difference down to no width, so the percentage stays in the cell.
The cell describes the record’s status with
[`ProgressBar::new`](../../struct.ProgressBar.html#method.new "associated function pert_ui::ProgressBar::new"),
since
[`TableItems::cell`](../../trait.TableItems.html#tymethod.cell "method pert_ui::TableItems::cell")
runs again whenever the table describes its rows.

``` rust
use pert_ui::prelude::*;
use pert_ui::Viewport;
use std::time::Duration;

struct Jobs(Vec<(&'static str, u64)>);
impl TableItems for Jobs {
    fn len(&self) -> usize { self.0.len() }
    fn key(&self, index: usize) -> Option<String> {
        self.0.get(index).map(|(name, _)| (*name).to_owned())
    }
    fn cell(&self, index: usize, column: &str) -> Option<Element> {
        let (name, done) = *self.0.get(index)?;
        Some(match column {
            "name" => Label::new(name).key("title"),
            "progress" => Row::new([
                ProgressBar::new(ProgressStatus::of(done, 100)).key("bar").with_grow(1),
                Label::new(format!("{done}%")).key("percent"),
            ])
            .key("cell"),
            _ => return None,
        })
    }
}
let mut ui = Runtime::new();
ui.set_viewport(Viewport::new(400.0, 200.0, 1.0)?)?;
let jobs = TableController::new(
    &ui,
    ListSelection::new(&ui, SelectionMode::Single),
    [
        TableColumn::new("name", "Job").width(Length::from(120)),
        TableColumn::new("progress", "Progress").width(Length::from(160)),
    ],
    TableSource::new(Jobs(vec![("render", 42), ("upload", 100)])),
)?;
let table = ui.mount_tree(Element::new("jobs", Table::new(&jobs, 28.0)?))?;
ui.flush(Duration::ZERO)?;
let row = ui.find_descendant(&table, "render")?.ok_or("mounted")?;
// The width and the right edge of a component of the row.
let span = |key: &str| -> Result<(f32, f32), Box<dyn std::error::Error>> {
    let id = ui.find_descendant(&row, key)?.ok_or("mounted")?;
    let bounds = ui.bounds(&id)?.ok_or("laid out")?;
    Ok((bounds.width(), bounds.x() + bounds.width()))
};
let ((_, cell_end), (bar, _), (_, percent_end)) = (span("cell")?, span("bar")?, span("percent")?);
assert!(bar < 192.0, "the bar gave up width");
assert!(percent_end <= cell_end, "the percentage stays in the cell");
#[cfg(debug_assertions)]
assert!(ui.validation_report().issues.is_empty(), "nothing escapes its cell");
```

## [§](#zoomable-content)Zoomable content

[`ZoomView`](../../struct.ZoomView.html "struct pert_ui::ZoomView")
places components at content-space points and lets the user pan and zoom
them uniformly with the wheel, pinch gestures and the keyboard, within
[`ScaleLimits`](../../struct.ScaleLimits.html "struct pert_ui::ScaleLimits");
a
[`ZoomController`](../../struct.ZoomController.html "struct pert_ui::ZoomController")
reads and sets the view from application code, serves one mounted view
at a time, and the view is part of the workspace. Views that zoom one
axis at a time, such as timelines and plots, map their own positions
with an
[`AxisMapping`](../../struct.AxisMapping.html "struct pert_ui::AxisMapping")
per axis and draw undistorted. Custom containers scale and offset their
children through
[`Component::child_transform`](../../trait.Component.html#method.child_transform "method pert_ui::Component::child_transform")
with a
[`ViewTransform`](../../struct.ViewTransform.html "struct pert_ui::ViewTransform");
clipping, hit testing and focus reveal follow it.

## [§](#drag-and-drop)Drag and drop

A component starts a drag session from a held pointer button with
[`EventContext::start_drag`](../../struct.EventContext.html#method.start_drag "method pert_ui::EventContext::start_drag"),
offering a typed
[`DragPayload`](../../struct.DragPayload.html "struct pert_ui::DragPayload"),
the permitted
[`DropOperations`](../../struct.DropOperations.html "struct pert_ui::DropOperations")
and a
[`DragPreview`](../../struct.DragPreview.html "struct pert_ui::DragPreview").
Components under the pointer receive
[`Component::drag_event`](../../trait.Component.html#method.drag_event "method pert_ui::Component::drag_event")
and answer with a
[`DropResponse`](../../enum.DropResponse.html "enum pert_ui::DropResponse");
on
[`DragPhase::Drop`](../../enum.DragPhase.html#variant.Drop "variant pert_ui::DragPhase::Drop")
the target performs the application edit, and the source learns the
outcome in
[`Component::drag_finished`](../../trait.Component.html#method.drag_finished "method pert_ui::Component::drag_finished").
Escape and focus loss cancel. Reorderable lists, trees and tabs, and tab
detachment, use the same public contract.

### [§](#retained-state-notifications)Retained state notifications

`Accordion::on_change` receives an `ExpansionChange`, the open keys and
their `ChangeOrigin`, after layout settles. User input, reveal,
restoration and redescription can all change the open set. Initial
expansion and restoration before first preparation are silent. A handler
may write signals; failure is observable without rollback or replay.

FloatingWindow placement builders store initial description data, so
construction from `UpdateContext` does not write signals. Extracting the
controller before calling the builders still addresses that window;
programmatic placement wins over initial geometry. Same-key
redescription keeps the mounted controller.

A `ListController` addresses one mounted list at a time, including
hidden lists. Reuse it for another list only after the prior one is
removed. Committed scroll, divider, region, label-column and window
geometry is reported live through `ScrollArea::on_scroll`,
`SplitPane::on_resize`, `RegionLayout::on_resize`,
`PropertyGrid::on_resize`, `FloatingWindow::on_frame` and the `Geometry`
kind of `ToolWindowLayout::on_change`, each with its `ChangeOrigin`,
once per flush and never for previews or cancelled gestures; it also
stays workspace and inspection state.
