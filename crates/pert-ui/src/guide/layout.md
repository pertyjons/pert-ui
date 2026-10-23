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

# Growing and stretching rows and columns

[`Element::with_grow`] gives a child of a [`Row`] or [`Column`] a share of the
space the container's allocation leaves over, and [`Align::Stretch`] makes
children fill the cross axis: a column's width, or the height a row is
allocated. A row grown in a column therefore stretches tall controls, such as a
vertical [`Slider`] beside a [`LevelMeter`], over the whole strip. A row's
measured height stays its tallest child's, so it fills what its parent gives
without asking for more; children keep their own maximum and minimum heights.
A custom container does the same with the size its
[`Component::arrange`] receives.

```rust
use pert_ui::prelude::*;
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
# Ok::<(), Box<dyn std::error::Error>>(())
```

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
That refresh is a repeated pointer move ([`PointerEvent::is_refresh`]): a
custom container that moves children during a drag invalidates only when the
drag's derived offset changes, as [`Component::pointer_input`] shows, or its
own layout keeps producing the next move.
They share ordinary
[`Painter`], clipping, input, identity, and lifecycle contracts.

Children that overlap on purpose must say so, or development validation reports
[`ValidationCode::UnexpectedOverlap`]. A stacking parent permits overlap among
its children, and a press or focus then raises the child within its priority. For
content whose order is fixed by the application, such as shapes on a canvas in
document order, either return that order from
[`Component::child_stacking_priority`] or leave stacking off and report
[`OverlapPolicy::Allow`] in each overlapping child's
[`ComponentDetails::validation`] from [`Component::inspect`]. Floating windows
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

# Children from values

[`Dynamic`], [`Show`] and [`Each`] describe children from a binding, so a view
whose structure depends on data needs no custom component. Each describes its
children again only when the value changes; children that keep their key keep
their state and focus. [`Show`] unmounts its child while the flag reads false,
whereas [`View::visible_signal`] hides content and keeps it.

```rust
use std::time::Duration;
use pert_ui::prelude::*;

let mut ui = Runtime::new();
let tasks = ui.signal(vec!["Write the plan".to_owned()]);
let details = ui.signal(false);
let (add, toggle) = (tasks.clone(), details.clone());
let view = Column::new([
    Each::new(&tasks, Clone::clone, |_, task| Label::new(task.as_str()).key("task"))
        .key("tasks"),
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
assert!(ui.find(&root, &["tasks", "items", "Ship it"])?.is_some());
# Ok::<(), Box<dyn std::error::Error>>(())
```

`Each` describes every item and offers no selection, keyboard navigation or
filtering. For a list the user selects, filters, reorders or edits in place,
use a [`VirtualList`], as [an editable list of
records](#an-editable-list-of-records) shows.

# Large collections

[`VirtualList`] mounts only the items near its viewport, from a [`ListSource`]
whose [`ListItems`] answer the count, the key at a position and the element at a
position; keys, not positions, identify rows, so a row keeps its state and focus
across a change of the source. The list keys each element it builds with the
key at its position. It lays out one or more lanes, horizontally or
vertically, with fixed or estimated extents. [`Table`] adds columns that can be
resized, sorted and moved, with its state in a [`TableController`]: a header
press or Alt+Up/Down on the active cell asks the application to sort, and the
controller fixes widths from code as a drag does. [`Tree`] shows a
[`TreeSource`] of nested rows whose expansion lives in a [`TreeController`]. All three
share [`ListSelection`] for single or multiple selection, and lists and trees can
be made reorderable by dragging.

A list takes a binding of a [`ListSource`], not the [`ListItems`] value itself:
wrap the rows with [`ListSource::new`] and hold the result in a signal,
`ui.signal(ListSource::new(rows))`, then pass that signal to
[`VirtualList::new`], or pass a derived binding such as [`ListFilter::items`].
Writing a new `ListSource` to the signal is how the content changes.

A collection mounts the rows inside the height it is allocated, so it needs a
bounded one. A window or detached view, a [`SplitPane`] pane, a [`Container`] or
[`CollectionFilter`] with a bounded allocation, a [`Tab::fixed`] tab, a
[`RegionLayout::fixed`] region, a [`View::fixed`] view and a grown child of a
bounded [`Column`] give one: a grown child of a column shrinks to what the
other children leave, so a toolbar above a grown collection works. Default
[`Tab::new`] and [`RegionLayout`] content and a [`ScrollArea`] do not, since
they scroll content at its natural size, which for a collection is its whole
content; in debug builds validation reports such a collection as
[`ValidationCode::UnboundedCollection`]. A bounded [`Row`] offers its height
to every child, so it bounds a vertical collection beside other children, but
it does not shrink their widths, so it never bounds a horizontal one.

# Large data

Virtualization bounds the components; the application's data and the
collection's bookkeeping still cost in proportion to the rows they touch:

- Replacing a source ([`TableController::set_source`], a new [`ListSource`] in
  the binding, [`TreeController::set_source`]) rebuilds the descriptions of the
  mounted rows, and an active filter reads every row's text again. A tree
  source's replacement also flattens its expanded branches again, asking
  each expanded node for its children. Coalesce
  appended data into batches, keep one source per view, and do not replace a
  source from a selection callback that changed nothing it shows.
- [`ListItems::index_of`] and [`TableItems::index_of`] default to a scan of
  every row, and reveal, anchoring and the active row call them; keep a
  reverse index from key to position in a large source.
- A filter joins each row's [`TableItems::text`] cells into one string per row;
  offer the text a user searches for, and nothing more.
- [`TreeItems::children`] returns every child key, and expanding a node indexes
  all of them; very wide nodes cost in proportion to their children.
- A table builds every column of each mounted row, also the columns scrolled
  out of view.

The [log explorer example](https://github.com/pertyjons/pert-ui/blob/main/crates/pert-ui/examples/log_explorer/main.rs)
puts this together over a generated log of hundreds of thousands of lines:
bounded tables and a tree, row identities with a reverse index, loading in
batches on a worker, appended lines inserted into each table's order instead
of rebuilding it, a live tail that follows the last shown row, an Errors view
that detaches, copying per table and selectable detail text. Its headless
session reports mounted counts and flush times and checks that the counts
stay bounded: `cargo run -p pert-ui --example log_explorer --no-default-features`,
or open it with `cargo run -p pert-ui --example log_explorer -- --window`.

# An editable list of records

The application keeps its records in its own value, here a
`Signal<Vec<Task>>`, and the list shows them through a [`ListItems`]
implementation over a copy of that value. Each piece then has one job:

- **Source.** [`ReadSignal::map`] wraps the records in a new [`ListSource`]
  whenever they change, keyed by each record's identity, never its position,
  and offering its title as [`ListItems::text`] for the filter.
- **Filter.** A [`ListFilter`] over that source, shown by a
  [`CollectionFilter`] field, publishes the rows that match in
  [`ListFilter::items`], which the list binds instead of the source.
- **Selection and actions.** A [`ListSelection`] makes the list one Tab stop
  that the arrows navigate; [`VirtualList::on_activate`] runs on Enter, and
  [`VirtualList::reorderable`] receives drags and Alt with an arrow as one
  [`ListDrop`] whose position names keys, which the callback applies to the
  records.
- **Hidden selection.** A filter never prunes the selection, and moves the
  active row off a hidden row only while the list has focus, so a selected
  row can be hidden while the user types in the field. Deriving
  `ListFilter::items()` with [`ListSelection::active`] through
  [`ReadSignal::combine`] and [`ListItems::index_of`] tells whether it is
  shown; the Delete button below is enabled only then.

A control in a row binds a signal, and [`ListItems::build`] has no state
owner, so a row makes its binding in one of two ways:

- **Per-row bindings.** `build` creates a binding with [`Signal::sibling`]
  holding the record's value, and the control's callback writes the change
  back to the records. The list describes all its mounted rows again, so each
  of them receives a new binding, whenever a change of the records replaces
  the source, and also whenever a scroll or resize mounts or unmounts another
  row. That suits a control without a draft, such as the checkbox below. A
  text editor ends its open draft when its binding changes, so a per-row
  binding would discard the user's typing whenever any record changes or the
  list scrolls another row into view.
- **Retained rows.** A row component makes its binding once, in its first
  update, and keeps it while the row stays mounted. A new description brings
  the record's current value, which [`Component::reconcile`] keeps and, after
  a [`Notifier`] request, [`Component::deliver_notifications`] writes to the
  binding, since neither an update nor a reconciliation may write. The title
  below works this way, so its draft survives scrolling and changes to other
  records.

```rust
use std::{mem, time::Duration};
use pert_ui::prelude::*;
use pert_ui::{ListDrop, ListDropPosition};

// Signal values are compared when written and cloned when read.
#[derive(Clone, PartialEq)]
struct Task { id: u64, title: String, done: bool }

/// The records as rows: a copy for reading, and the signal to write back to.
struct Rows { tasks: Vec<Task>, store: Signal<Vec<Task>> }
impl ListItems for Rows {
    fn len(&self) -> usize { self.tasks.len() }
    fn key(&self, index: usize) -> Option<String> {
        Some(self.tasks.get(index)?.id.to_string())
    }
    fn build(&self, index: usize) -> Option<Element> {
        let task = self.tasks.get(index)?;
        // Per-row binding: a new signal each time the row is described.
        let done = self.store.sibling(task.done);
        let (store, id) = (self.store.clone(), task.id);
        let check = Checkbox::new(done, "Done").on_change(move |_, done| {
            edit(&store, id, |task| task.done = done)
        });
        let title = Title {
            id, title: task.title.clone(), store: self.store.clone(),
            shown: None, notifier: None, stale: false,
        };
        // The list keys the row with the task's key, replacing "row".
        Some(Row::new([check.key("done"), Element::new("title", title).with_grow(1)]).key("row"))
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

/// A retained row part: it makes its title binding once, so describing the
/// rows again leaves an open draft alone.
struct Title {
    id: u64,
    title: String,
    store: Signal<Vec<Task>>,
    shown: Option<Signal<String>>,
    notifier: Option<Notifier>,
    stale: bool,
}
impl Component for Title {
    fn reconcile(&mut self, next: Self) -> Option<Invalidation> {
        if next.title == self.title {
            return None;
        }
        self.title = next.title;
        self.stale = true;
        Some(Invalidation::Paint)
    }
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        let notifier = self.notifier.get_or_insert_with(|| cx.notifier());
        if self.shown.is_none() {
            let shown = cx.signal(self.title.clone());
            let (store, id) = (self.store.clone(), self.id);
            cx.request_children(vec![Element::new(
                "editor",
                InlineText::new(&shown).on_save(move |_, title| {
                    edit(&store, id, |task| task.title = title.to_owned())
                }),
            )]);
            self.shown = Some(shown);
        } else if self.stale {
            notifier.request(); // the write waits for the event phase
        }
        Ok(())
    }
    fn deliver_notifications(&mut self, _: &mut EventContext) -> Result<(), RuntimeError> {
        if let (true, Some(shown)) = (mem::take(&mut self.stale), &self.shown) {
            shown.set_changed(self.title.clone())?;
        }
        Ok(())
    }
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

let mut ui = Runtime::new();
ui.set_viewport(Viewport::new(400.0, 300.0, 1.0)?)?;
let task = |id, title: &str| Task { id, title: title.to_owned(), done: false };
let tasks = ui.signal(vec![task(1, "Buy milk"), task(2, "Write the plan"), task(3, "Ship it")]);
let store = tasks.clone();
let source = tasks.map(move |all| {
    ListSource::new(Rows { tasks: all.clone(), store: store.clone() })
})?;
let selection = ListSelection::new(&ui, SelectionMode::Single);
let filter = ListFilter::new(&ui, source)?.selection(selection.clone());
let selected_shown = filter.items().combine(&selection.active(), |rows, active| {
    active.as_deref().is_some_and(|key| rows.index_of(key).is_some())
})?;
let opened = ui.signal(None::<String>);
let (open, order, remove, active) =
    (opened.clone(), tasks.clone(), tasks.clone(), selection.active());
let list = VirtualList::new(filter.items(), 32.0)?
    .selection(selection.clone())
    .on_activate(move |_, key| open.set(Some(key.to_owned())))
    .reorderable(move |_, drop| reorder(&order, drop));
let delete = Button::new("Delete", move |_| {
    let Some(key) = active.get()? else { return Ok(()) };
    remove.update(|tasks| tasks.retain(|task| task.id.to_string() != key))
})
.enabled(&selected_shown);
let root = ui.mount_tree(Column::new([
    CollectionFilter::new(&ui, filter.clone(), Element::new("list", list))
        .key("tasks")
        .with_grow(1),
    delete.key("delete"),
]).key("app"))?;
ui.flush(Duration::ZERO)?;
let press = |ui: &mut Runtime, key| -> Result<(), RuntimeError> {
    ui.key_input(key, KeyAction::Press)?;
    ui.key_input(key, KeyAction::Release)?;
    Ok(())
};
let ids = |tasks: &Signal<Vec<Task>>| -> Result<Vec<u64>, RuntimeError> {
    Ok(tasks.get()?.iter().map(|task| task.id).collect())
};

// Select the first row, move it down with Alt+Down and open it with Enter.
let list = ui.find(&root, &["tasks", "list"])?.ok_or("the list")?;
ui.focus(&list)?;
press(&mut ui, KeyCode::ArrowDown)?;
ui.key_input(KeyCode::AltLeft, KeyAction::Press)?;
press(&mut ui, KeyCode::ArrowDown)?;
ui.key_input(KeyCode::AltLeft, KeyAction::Release)?;
assert_eq!(ids(&tasks)?, [2, 1, 3]);
press(&mut ui, KeyCode::Enter)?;
assert_eq!(opened.get()?.as_deref(), Some("1"));

// The per-row checkbox writes the record.
let check = ui.find(&root, &["tasks", "list", "1", "done"])?.ok_or("a checkbox")?;
ui.focus(&check)?;
press(&mut ui, KeyCode::Space)?;
assert!(tasks.get()?[1].done);

// A title draft survives a change to another record.
let title = ui.find(&root, &["tasks", "list", "2", "title", "editor"])?.ok_or("a title")?;
ui.focus(&title)?;
press(&mut ui, KeyCode::F2)?;
ui.text_input("Write the whole plan")?;
edit(&tasks, 3, |task| task.done = true)?;
ui.flush(Duration::ZERO)?;
press(&mut ui, KeyCode::Enter)?;
assert_eq!(tasks.get()?[0].title, "Write the whole plan");

// Typing in the field hides the selected row, which disables Delete.
let field = ui.find(&root, &["tasks", "filter-field", "filter"])?.ok_or("the field")?;
ui.focus(&field)?;
ui.text_input("ship")?;
ui.flush(Duration::ZERO)?;
assert_eq!(filter.counts().get()?.shown, 1);
assert_eq!(selection.active().get()?.as_deref(), Some("1"));
assert!(!selected_shown.get()?);
# Ok::<(), Box<dyn std::error::Error>>(())
```

A press on the title focuses the title rather than selecting its row; Enter
or F2 on the focused title, or a double click, starts editing. The [inventory
example](https://github.com/pertyjons/pert-ui/blob/main/crates/pert-ui/examples/inventory/main.rs)
applies the retained form to a table's checkbox column, and a
[`Table`] edits text cells in place through its own editor instead.

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

`Accordion::on_change` receives an `ExpansionChange`, the open keys and their
`ChangeOrigin`, after layout settles. User input, reveal, restoration and
redescription can all change the open set. Initial expansion and restoration before first preparation are silent.
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
