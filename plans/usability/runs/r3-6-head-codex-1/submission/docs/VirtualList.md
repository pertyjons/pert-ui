[pert_ui](index.html)

# Struct VirtualList Copy item path

[Source](../src/pert_ui_runtime/virtual_list.rs.html#522)

``` rust
pub struct VirtualList { /* private fields */ }
```

Expand description

A list or grid that mounts only the items near its viewport, plus a
bounded overscan, and keeps the logical extent of every item.

It is vertical and single-lane by default, with a fixed item extent from
[`VirtualList::new`](struct.VirtualList.html#method.new "associated function pert_ui::VirtualList::new")
or measured items from
[`VirtualList::estimated`](struct.VirtualList.html#method.estimated "associated function pert_ui::VirtualList::estimated");
[`VirtualList::lanes`](struct.VirtualList.html#method.lanes "method pert_ui::VirtualList::lanes")
and
[`VirtualList::orientation`](struct.VirtualList.html#method.orientation "method pert_ui::VirtualList::orientation")
make it a grid or horizontal.

## [§](#a-bounded-extent)A bounded extent

The list mounts the rows inside the extent it is allocated, so it needs
a bounded height (or width, along a horizontal axis). Measured without a
bound, it asks for its whole content extent, and a parent that grants it
mounts every row. A window or detached view root, a
[`SplitPane`](struct.SplitPane.html "struct pert_ui::SplitPane") pane, a
[`Container`](struct.Container.html "struct pert_ui::Container") or
[`CollectionFilter`](struct.CollectionFilter.html "struct pert_ui::CollectionFilter")
with a bounded allocation, a
[`Tab::fixed`](struct.Tab.html#method.fixed "method pert_ui::Tab::fixed")
tab, a
[`RegionLayout::fixed`](struct.RegionLayout.html#method.fixed "method pert_ui::RegionLayout::fixed")
region, a
[`View::fixed`](struct.View.html#method.fixed "method pert_ui::View::fixed")
view, a grown child of a bounded
[`Column`](struct.Column.html "struct pert_ui::Column"), which shrinks
to what its siblings leave, and a grown child of a bounded
[`Row`](struct.Row.html "struct pert_ui::Row"), along a horizontal axis,
bound it. The default
[`Tab::new`](struct.Tab.html#method.new "associated function pert_ui::Tab::new")
and
[`RegionLayout`](struct.RegionLayout.html "struct pert_ui::RegionLayout")
content and a
[`ScrollArea`](struct.ScrollArea.html "struct pert_ui::ScrollArea") do
not, since they scroll their content at its natural size. A bounded
[`Row`](struct.Row.html "struct pert_ui::Row") bounds a vertical list’s
height, which it offers to every child, and a grown horizontal list’s
width, which gives up what its siblings need; an ungrown horizontal list
keeps its natural width, its whole content. In debug builds validation
reports a list of at least 100 rows that was given its whole extent
while less than half of it is visible, as
[`ValidationCode::UnboundedCollection`](enum.ValidationCode.html#variant.UnboundedCollection "variant pert_ui::ValidationCode::UnboundedCollection").

The list is its own scroll viewport: it scrolls by wheel, by the
keyboard when focused (arrows, Page Up/Down, Home, End) and by its
overlay scrollbar, and it honors the
[`Component::reveal`](trait.Component.html#method.reveal "method pert_ui::Component::reveal")
contract, so `Runtime::focus` and keyboard traversal scroll a mounted
row into view. Rows that are not mounted take part in neither;
application code reaches them through a
[`ListController`](struct.ListController.html "struct pert_ui::ListController"),
which scrolls to a key and mounts it.

A row that has keyboard focus or captures the pointer stays mounted
while it is outside the mounted range, so focus, IME composition and a
gesture in progress survive scrolling away; it is unmounted at the
list’s next update once neither holds, which a focus move triggers by
itself. Rows keep their identity across scrolling: a row that stays in
range keeps its component and state, and one that leaves and returns is
built again from
[`ListItems::build`](trait.ListItems.html#tymethod.build "method pert_ui::ListItems::build").

The list’s children are its mounted rows, which
[`Runtime::children`](struct.Runtime.html#method.children "method pert_ui::Runtime::children")
lists in index order: the visible lines and the overscan, with a row
pinned by focus or a pointer gesture and the
[`keep_mounted`](struct.VirtualList.html#method.keep_mounted "method pert_ui::VirtualList::keep_mounted")
row included wherever they are. A test lists them with their keys
through
[`Driver::address_children`](testing/struct.Driver.html#method.address_children "method pert_ui::testing::Driver::address_children").

[`VirtualList::lanes`](struct.VirtualList.html#method.lanes "method pert_ui::VirtualList::lanes")
makes it a grid: items fill lanes in order, each line holds that many
items, and every item gets an equal share of the cross extent.
[`VirtualList::orientation`](struct.VirtualList.html#method.orientation "method pert_ui::VirtualList::orientation")
makes it scroll along x instead, with the same rules and a horizontal
scrollbar; a wheel with only a vertical axis scrolls it too. The window
arithmetic is the public
[`VirtualAxis`](struct.VirtualAxis.html "struct pert_ui::VirtualAxis").

Without a selection the list is a scrolling viewport: unmodified arrows,
Page Up/Down, Home and End scroll, and it takes focus only while it
overflows. Attaching a
[`ListSelection`](struct.ListSelection.html "struct pert_ui::ListSelection")
with
[`VirtualList::selection`](struct.VirtualList.html#method.selection "method pert_ui::VirtualList::selection")
makes the list one Tab stop that navigates by item position, which
crosses items that are not mounted: arrows move the active item by one
item or one line, Home/End go to the ends, Page Up/Down move by the
viewport’s whole lines, and the list scrolls the active item into view
and mounts it. In
[`SelectionMode::Single`](enum.SelectionMode.html#variant.Single "variant pert_ui::SelectionMode::Single")
the selection follows the active item; in
[`SelectionMode::Multiple`](enum.SelectionMode.html#variant.Multiple "variant pert_ui::SelectionMode::Multiple")
Space toggles it, Shift extends a range from the anchor and Control
moves without selecting. Navigation chords with Super, or with Alt other
than
[reordering](struct.VirtualList.html#method.reorderable "method pert_ui::VirtualList::reorderable"),
are left to command bindings. A primary press on an item that the item’s
own content did not handle makes it active and selects it with the same
modifiers, Control toggling. Enter runs
[`VirtualList::on_activate`](struct.VirtualList.html#method.on_activate "method pert_ui::VirtualList::on_activate").
Selected items get a TabSelected fill beneath their content, and the
active item the
[`ColorRole::FocusRing`](enum.ColorRole.html#variant.FocusRing "variant pert_ui::ColorRole::FocusRing")
ring while the list has focus. An interactive row uses SurfaceHover
beneath its content while hovered, with selection taking precedence.
Interactive content inside an item keeps its own input, hover feedback
and Tab stop. With one lane the arrows across the axis are left
unhandled, so an owner such as a tree can give them a meaning.

Each item is placed in its lane’s share of the cross extent, along the
axis where its line starts in the list’s
[`VirtualAxis`](struct.VirtualAxis.html "struct pert_ui::VirtualAxis"),
and the content extent is the axis’s. With a fixed extent every line is
that long, an item’s own measurement does not change its slot, and an
item whose minimum is longer is clipped to it; with
[`VirtualList::estimated`](struct.VirtualList.html#method.estimated "associated function pert_ui::VirtualList::estimated")
each mounted line is as long as its longest item. Positions along the
axis are kept in `f64`, so a collection far longer than `f32` holds to
the pixel still places every item exactly. Mounting and unmounting
happen through
[`UpdateContext::request_children`](struct.UpdateContext.html#method.request_children "method pert_ui::UpdateContext::request_children")
after the list has seen its allocation, so the first frame after a
resize or a scroll may run one extra update wave inside the same flush;
nothing is scheduled once the range settles.

``` rust
use pert_ui::{Element, Label, ListItems, ListSource, Runtime, VirtualList};

struct Numbers(usize);
impl ListItems for Numbers {
    fn len(&self) -> usize {
        self.0
    }
    fn key(&self, index: usize) -> Option<String> {
        (index < self.0).then(|| format!("n-{index}"))
    }
    fn build(&self, index: usize) -> Option<Element> {
        self.key(index).map(|key| Element::new(key, Label::new(format!("Row {index}"))))
    }
    fn index_of(&self, key: &str) -> Option<usize> {
        key.strip_prefix("n-")?.parse().ok().filter(|index| *index < self.0)
    }
}

let mut ui = Runtime::new();
ui.set_viewport(pert_ui::Viewport::new(300.0, 200.0, 1.0)?)?;
let items = ui.signal(ListSource::new(Numbers(100_000)));
let list = ui.mount_tree(Element::new(
    "list",
    VirtualList::new(items.read_only(), 24.0)?,
))?;
ui.flush(std::time::Duration::ZERO)?;
// Only the rows near the 200-pixel viewport are mounted, not 100,000.
assert!(ui.children(&list)?.len() < 40);
```

Describing a mounted list again keeps its retained state and structure:
the source binding, axis, lanes, overscan, controller, selection,
[`active_outline`](struct.VirtualList.html#method.active_outline "method pert_ui::VirtualList::active_outline"),
[`item_tab_stops`](struct.VirtualList.html#method.item_tab_stops "method pert_ui::VirtualList::item_tab_stops")
and
[`keep_mounted`](struct.VirtualList.html#method.keep_mounted "method pert_ui::VirtualList::keep_mounted"),
which describe the rows it holds. Change rows through the source binding
and the controller, or give the list a new key to replace them. Its
callbacks and its permission follow the new description, including their
removal:
[`on_activate`](struct.VirtualList.html#method.on_activate "method pert_ui::VirtualList::on_activate"),
[`on_text_input`](struct.VirtualList.html#method.on_text_input "method pert_ui::VirtualList::on_text_input"),
[`on_selection_change`](struct.VirtualList.html#method.on_selection_change "method pert_ui::VirtualList::on_selection_change"),
[`reorderable`](struct.VirtualList.html#method.reorderable "method pert_ui::VirtualList::reorderable"),
[`placement`](struct.VirtualList.html#method.placement "method pert_ui::VirtualList::placement"),
[`reorder_permission`](struct.VirtualList.html#method.reorder_permission "method pert_ui::VirtualList::reorder_permission")
and
[`drag_operations`](struct.VirtualList.html#method.drag_operations "method pert_ui::VirtualList::drag_operations").
A description without `reorderable`, or with a fixed denial, cancels an
armed or active reorder at the update the re-description schedules, as
revoking the permission does, and a removed `on_selection_change`
discards the changes it has not yet delivered.

## Implementations[§](#implementations)

[Source](../src/pert_ui_runtime/virtual_list.rs.html#740)[§](#impl-VirtualList)

### impl [VirtualList](struct.VirtualList.html "struct pert_ui::VirtualList")

[Source](../src/pert_ui_runtime/virtual_list.rs.html#742)

#### pub const [DEFAULT_OVERSCAN](#associatedconstant.DEFAULT_OVERSCAN): [usize](https://doc.rust-lang.org/1.99.1/std/primitive.usize.html) = 4

The lines mounted before and after the visible ones by default.

[Source](../src/pert_ui_runtime/virtual_list.rs.html#758-761)

#### pub fn [new](#method.new)( items: impl [Into](https://doc.rust-lang.org/1.99.1/core/convert/trait.Into.html "trait core::convert::Into")\<[ReadSignal](struct.ReadSignal.html "struct pert_ui::ReadSignal")\<[ListSource](struct.ListSource.html "struct pert_ui::ListSource")\>\>, row_height: [f32](https://doc.rust-lang.org/1.99.1/std/primitive.f32.html), ) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[VirtualList](struct.VirtualList.html "struct pert_ui::VirtualList"), [GeometryError](enum.GeometryError.html "enum pert_ui::GeometryError")\>

Creates a vertical single-lane list over a binding of rows with a fixed
logical row height.

`items` is a binding of a
[`ListSource`](struct.ListSource.html "struct pert_ui::ListSource"), not
the [`ListItems`](trait.ListItems.html "trait pert_ui::ListItems")
value: wrap the rows with
[`ListSource::new`](struct.ListSource.html#method.new "associated function pert_ui::ListSource::new")
in a signal and pass the signal, or pass a derived binding such as
[`ListFilter::items`](struct.ListFilter.html#method.items "method pert_ui::ListFilter::items").

##### [§](#errors)Errors

Returns
[`GeometryError::NonFinite`](enum.GeometryError.html#variant.NonFinite "variant pert_ui::GeometryError::NonFinite")
for a row height that is not finite,
[`GeometryError::Negative`](enum.GeometryError.html#variant.Negative "variant pert_ui::GeometryError::Negative")
for a negative one and
[`GeometryError::Empty`](enum.GeometryError.html#variant.Empty "variant pert_ui::GeometryError::Empty")
for zero.

[Source](../src/pert_ui_runtime/virtual_list.rs.html#785-788)

#### pub fn [estimated](#method.estimated)( items: impl [Into](https://doc.rust-lang.org/1.99.1/core/convert/trait.Into.html "trait core::convert::Into")\<[ReadSignal](struct.ReadSignal.html "struct pert_ui::ReadSignal")\<[ListSource](struct.ListSource.html "struct pert_ui::ListSource")\>\>, estimate: [f32](https://doc.rust-lang.org/1.99.1/std/primitive.f32.html), ) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[VirtualList](struct.VirtualList.html "struct pert_ui::VirtualList"), [GeometryError](enum.GeometryError.html "enum pert_ui::GeometryError")\>

Creates a vertical single-lane list whose rows have their own heights.

Each row is `estimate` tall until it is mounted; the list then measures
it at the width of its lane and records the result in its
[`VirtualAxis`](struct.VirtualAxis.html "struct pert_ui::VirtualAxis"),
so the content extent and the scrollbar become exact for every row that
has been seen and stay an estimate for the rest. A row measured above
the viewport corrects the offset by its difference, which keeps what is
on screen where it was while scrolling upwards. A line of a grid is as
tall as its tallest item. Replacing the source forgets every
measurement, since a position may then hold another row. Measuring adds
one measurement per mounted row to each arrangement, and a changed
height may cost one more update wave inside the same flush.

##### [§](#errors-1)Errors

Returns
[`GeometryError::NonFinite`](enum.GeometryError.html#variant.NonFinite "variant pert_ui::GeometryError::NonFinite")
for an estimate that is not finite,
[`GeometryError::Negative`](enum.GeometryError.html#variant.Negative "variant pert_ui::GeometryError::Negative")
for a negative one and
[`GeometryError::Empty`](enum.GeometryError.html#variant.Empty "variant pert_ui::GeometryError::Empty")
for zero.

[Source](../src/pert_ui_runtime/virtual_list.rs.html#844)

#### pub fn [overscan](#method.overscan)(self, lines: [usize](https://doc.rust-lang.org/1.99.1/std/primitive.usize.html)) -\> [VirtualList](struct.VirtualList.html "struct pert_ui::VirtualList")

Sets how many lines beyond the visible ones stay mounted on each side,
so small scrolls reuse items instead of mounting. Zero is allowed.

[Source](../src/pert_ui_runtime/virtual_list.rs.html#855)

#### pub fn [lanes](#method.lanes)(self, lanes: [usize](https://doc.rust-lang.org/1.99.1/std/primitive.usize.html)) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[VirtualList](struct.VirtualList.html "struct pert_ui::VirtualList"), [LayoutError](enum.LayoutError.html "enum pert_ui::LayoutError")\>

Sets how many items share a line, which makes the list a grid. The item
extent given to
[`VirtualList::new`](struct.VirtualList.html#method.new "associated function pert_ui::VirtualList::new")
stays each line’s extent along the scrolling axis, and each item gets an
equal share of the cross extent.

##### [§](#errors-2)Errors

Returns
[`LayoutError::NoLanes`](enum.LayoutError.html#variant.NoLanes "variant pert_ui::LayoutError::NoLanes")
for zero lanes.

[Source](../src/pert_ui_runtime/virtual_list.rs.html#862)

#### pub fn [orientation](#method.orientation)(self, orientation: [Orientation](enum.Orientation.html "enum pert_ui::Orientation")) -\> [VirtualList](struct.VirtualList.html "struct pert_ui::VirtualList")

Selects the scrolling axis; vertical is the default. Horizontally, the
extent given to
[`VirtualList::new`](struct.VirtualList.html#method.new "associated function pert_ui::VirtualList::new")
is each item’s width.

[Source](../src/pert_ui_runtime/virtual_list.rs.html#873)

#### pub fn [controller](#method.controller)(self, controller: [ListController](struct.ListController.html "struct pert_ui::ListController")) -\> [VirtualList](struct.VirtualList.html "struct pert_ui::VirtualList")

Attaches a controller whose reveal requests this list serves.

The list serves requests made from here on. A request made earlier,
which another list with the same controller may already have served, is
not replayed, so a list rebuilt with a long-lived controller opens where
it starts rather than at an old target.

[Source](../src/pert_ui_runtime/virtual_list.rs.html#890)

#### pub fn [selection](#method.selection)(self, selection: [ListSelection](struct.ListSelection.html "struct pert_ui::ListSelection")) -\> [VirtualList](struct.VirtualList.html "struct pert_ui::VirtualList")

Attaches application-owned navigation and selection state, which makes
the list one Tab stop that navigates by item; see the type
documentation.

[Source](../src/pert_ui_runtime/virtual_list.rs.html#898)

#### pub fn [active_outline](#method.active_outline)(self, outline: [bool](https://doc.rust-lang.org/1.99.1/std/primitive.bool.html)) -\> [VirtualList](struct.VirtualList.html "struct pert_ui::VirtualList")

Whether the active item gets the list’s outline while it has focus; true
by default. An owner that marks the active item itself, as a table does
for its active cell, turns it off.

[Source](../src/pert_ui_runtime/virtual_list.rs.html#911)

#### pub fn [item_tab_stops](#method.item_tab_stops)(self, stops: [bool](https://doc.rust-lang.org/1.99.1/std/primitive.bool.html)) -\> [VirtualList](struct.VirtualList.html "struct pert_ui::VirtualList")

Whether interactive content inside items, such as a checkbox in a row,
is a stop of Tab traversal; true by default. With false the items’
content leaves traversal through
[`Component::descendant_tab_stops`](trait.Component.html#method.descendant_tab_stops "method pert_ui::Component::descendant_tab_stops"),
so Tab moves past the list in one step; pointer presses and explicit
focus still reach the content. The list keeps its own focusability: with
a selection it is one stop while it has items, without one only while it
overflows. An owner that turns this off gives keyboard users another way
to reach the content.

[Source](../src/pert_ui_runtime/virtual_list.rs.html#925)

#### pub fn [keep_mounted](#method.keep_mounted)(self, key: [ReadSignal](struct.ReadSignal.html "struct pert_ui::ReadSignal")\<[Option](https://doc.rust-lang.org/1.99.1/core/option/enum.Option.html "enum core::option::Option")\<[String](https://doc.rust-lang.org/1.99.1/alloc/string/struct.String.html "struct alloc::string::String")\>\>) -\> [VirtualList](struct.VirtualList.html "struct pert_ui::VirtualList")

Keeps the item whose key `key` holds mounted while the source holds it,
as the list keeps an item that holds focus or a pointer gesture:
scrolled out of the range it stays mounted, unrevealed and unpainted.
`None` keeps nothing extra. An owner that hosts state in one item, such
as a table cell’s editor that must outlive focus, sets it for as long as
that state lives. The list reads the signal in its update, so a change
applies at the next flush; an item the source no longer holds is
unmounted. Set at construction; describing the list again keeps the
first signal.

[Source](../src/pert_ui_runtime/virtual_list.rs.html#939-942)

#### pub fn [on_text_input](#method.on_text_input)( self, handler: impl [FnMut](https://doc.rust-lang.org/1.99.1/core/ops/function/trait.FnMut.html "trait core::ops::function::FnMut")(&mut [EventContext](struct.EventContext.html "struct pert_ui::EventContext"), &[str](https://doc.rust-lang.org/1.99.1/std/primitive.str.html)) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[()](https://doc.rust-lang.org/1.99.1/std/primitive.unit.html), [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\> + 'static, ) -\> [VirtualList](struct.VirtualList.html "struct pert_ui::VirtualList")

Calls `handler` with committed text (typed characters, or an IME’s
committed composition) delivered while the list itself holds focus; text
never reaches it while an item’s content is focused. The list has no
text input state, so an IME composes nothing over it. A table begins a
cell edit from typed text this way. The handler runs in the event phase;
its error propagates from the text input call. Describing the list again
replaces or removes the handler.

Timing class: [in the
input](https://github.com/pertyjons/pert-ui/blob/main/docs/contracts.md#in-the-input).

[Source](../src/pert_ui_runtime/virtual_list.rs.html#1011-1018)

#### pub fn [on_selection_change](#method.on_selection_change)( self, on_change: impl [FnMut](https://doc.rust-lang.org/1.99.1/core/ops/function/trait.FnMut.html "trait core::ops::function::FnMut")(&mut [EventContext](struct.EventContext.html "struct pert_ui::EventContext"), &[ListSelectionChange](struct.ListSelectionChange.html "struct pert_ui::ListSelectionChange")) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[()](https://doc.rust-lang.org/1.99.1/std/primitive.unit.html), [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\> + 'static, ) -\> [VirtualList](struct.VirtualList.html "struct pert_ui::VirtualList")

Reports every change of the list’s
[`ListSelection`](struct.ListSelection.html "struct pert_ui::ListSelection")
after it took effect, with its
[`ChangeOrigin`](enum.ChangeOrigin.html "enum pert_ui::ChangeOrigin"),
so an application can follow the user’s choice, for example to show the
chosen item in a detail view.

Timing class: [after
settling](https://github.com/pertyjons/pert-ui/blob/main/docs/contracts.md#after-settling).

The list observes its selection when it updates. A change it sees is
reported with the origin of the latest write:
[`ChangeOrigin::User`](enum.ChangeOrigin.html#variant.User "variant pert_ui::ChangeOrigin::User")
for a click or key on this list or another collection sharing the
selection, and
[`ChangeOrigin::Application`](enum.ChangeOrigin.html#variant.Application "variant pert_ui::ChangeOrigin::Application")
for a write through the
[`ListSelection`](struct.ListSelection.html "struct pert_ui::ListSelection")
methods or a controller, including a
[`CollectionFilter`](struct.CollectionFilter.html "struct pert_ui::CollectionFilter")
moving the active item off a row its new query hides. Writes made
between two updates of the list arrive as one change, carrying the
origin of the last of them. The selection the list first sees is not a
change, and a list that is hidden reports a change made meanwhile when
it is shown and updates again. Without a selection nothing is reported.

Unlike
[`Tabs::on_select`](struct.Tabs.html#method.on_select "method pert_ui::Tabs::on_select"),
which reports each settled tab change, this reports the bound selection
as the list observes it at its update, so the coalescing above is part
of its meaning. Changes are delivered in an event phase after the update
and layout waves of the flush have settled, one call per change, oldest
first, and an input entry point settles its flush before it returns, as
for every component notification. The callback may write bindings; an
error does not undo the change, and the first error is returned from the
flush. Describing the list again replaces the callback, which receives
the changes not yet delivered, or removes it, which discards them; a
callback installed by a re-description reports changes from the
selection it first sees.

``` rust
use pert_ui::prelude::*;
use pert_ui::{KeyAction, Viewport};
use std::time::Duration;
struct Fruit;
impl ListItems for Fruit {
    fn len(&self) -> usize { 3 }
    fn key(&self, index: usize) -> Option<String> {
        ["apple", "pear", "plum"].get(index).map(|key| (*key).to_owned())
    }
    fn build(&self, index: usize) -> Option<Element> {
        Some(Label::new(self.key(index)?).key("row"))
    }
}
let mut ui = Runtime::new();
ui.set_viewport(Viewport::new(200.0, 200.0, 1.0)?)?;
let items = ui.signal(ListSource::new(Fruit));
let selection = ListSelection::new(&ui, SelectionMode::Single);
let shown = ui.signal(None::<String>);
let detail = shown.clone();
let list = VirtualList::new(&items, 24.0)?
    .selection(selection.clone())
    .on_selection_change(move |_, change| detail.set(change.active.clone()));
let id = ui.mount_tree(list.key("fruit"))?;
ui.flush(Duration::ZERO)?;
ui.focus(&id)?;
ui.key_input(KeyCode::ArrowDown, KeyAction::Press)?;
ui.key_input(KeyCode::ArrowDown, KeyAction::Release)?;
assert_eq!(shown.get()?.as_deref(), Some("apple"));
```

[Source](../src/pert_ui_runtime/virtual_list.rs.html#1119-1122)

#### pub fn [on_activate](#method.on_activate)( self, activate: impl [FnMut](https://doc.rust-lang.org/1.99.1/core/ops/function/trait.FnMut.html "trait core::ops::function::FnMut")(&mut [EventContext](struct.EventContext.html "struct pert_ui::EventContext"), &[str](https://doc.rust-lang.org/1.99.1/std/primitive.str.html)) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[()](https://doc.rust-lang.org/1.99.1/std/primitive.unit.html), [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\> + 'static, ) -\> [VirtualList](struct.VirtualList.html "struct pert_ui::VirtualList")

Runs when Enter is pressed on the active item of a list with a
selection. The callback receives the item’s key and may write bindings;
failures propagate through input routing.

With this callback the list consumes the press of Enter, with any
modifiers, while it has an active item, in ordinary bubbling (stage 10
of
[`Runtime::command_key_input`](struct.Runtime.html#method.command_key_input "method pert_ui::Runtime::command_key_input")’s
routing order). A command bound to Enter in a
[`View`](enum.CommandScope.html#variant.View "variant pert_ui::CommandScope::View"),
[`Window`](enum.CommandScope.html#variant.Window "variant pert_ui::CommandScope::Window")
or
[`Application`](enum.CommandScope.html#variant.Application "variant pert_ui::CommandScope::Application")
scope therefore does not run while focus is in the list or in a row that
leaves Enter to it; only a
[`Capture`](enum.CommandScope.html#variant.Capture "variant pert_ui::CommandScope::Capture")
binding runs first. Without the callback the list leaves Enter to them.

Timing class: [in the
input](https://github.com/pertyjons/pert-ui/blob/main/docs/contracts.md#in-the-input).

[Source](../src/pert_ui_runtime/virtual_list.rs.html#1164-1167)

#### pub fn [reorderable](#method.reorderable)( self, on_drop: impl [FnMut](https://doc.rust-lang.org/1.99.1/core/ops/function/trait.FnMut.html "trait core::ops::function::FnMut")(&mut [EventContext](struct.EventContext.html "struct pert_ui::EventContext"), &[ListDrop](struct.ListDrop.html "struct pert_ui::ListDrop")\<'\_\>) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[bool](https://doc.rust-lang.org/1.99.1/std/primitive.bool.html), [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\> + 'static, ) -\> [VirtualList](struct.VirtualList.html "struct pert_ui::VirtualList")

Makes items draggable and the list a drop target for
[`ListDragItems`](struct.ListDragItems.html "struct pert_ui::ListDragItems"),
which is how rows are reordered and moved between lists.

Timing class: [in the
input](https://github.com/pertyjons/pert-ui/blob/main/docs/contracts.md#in-the-input).

A primary press on an item that the item’s own content did not handle
arms a drag; once it has moved
[`DRAG_THRESHOLD`](constant.DRAG_THRESHOLD.html "constant pert_ui::DRAG_THRESHOLD")
the list starts a [drag
session](struct.EventContext.html#method.start_drag "method pert_ui::EventContext::start_drag")
carrying the pressed item’s key, or every selected key when the pressed
item is part of a selection. Pressing a selected item narrows the
selection only on a release that did not become a drag. The preview is
the pressed item built again from
[`ListItems::build`](trait.ListItems.html#tymethod.build "method pert_ui::ListItems::build"),
or a count for several items.

While such a drag is over the list, a Primary insertion line marks the
boundary a drop would use, and resting near an edge of an overflowing
list scrolls it. On a drop `on_drop` receives a
[`ListDrop`](struct.ListDrop.html "struct pert_ui::ListDrop") whose
position names the application’s keys, never an index, and performs the
edit in the application’s model by replacing the
[`ListSource`](struct.ListSource.html "struct pert_ui::ListSource"): **a
move is this one edit**, not an insertion here plus a removal at the
source. Return `false` when the application refuses, and the source is
told the drop was refused. Items from another list arrive the same way,
with
[`ListDrop::is_internal`](struct.ListDrop.html#method.is_internal "method pert_ui::ListDrop::is_internal")
false.

A drag is never the only way: with a
[`ListSelection`](struct.ListSelection.html "struct pert_ui::ListSelection"),
Alt with an arrow along the list moves the active item, or the selection
containing it, by one place through the same callback.

The list consumes that chord, Alt alone with an arrow along its axis, in
ordinary bubbling (stage 10 of
[`Runtime::command_key_input`](struct.Runtime.html#method.command_key_input "method pert_ui::Runtime::command_key_input")’s
routing order), also while
[`Self::reorder_permission`](struct.VirtualList.html#method.reorder_permission "method pert_ui::VirtualList::reorder_permission")
denies reordering, when it moves the active item instead. A command
bound to it in a
[`View`](enum.CommandScope.html#variant.View "variant pert_ui::CommandScope::View"),
[`Window`](enum.CommandScope.html#variant.Window "variant pert_ui::CommandScope::Window")
or
[`Application`](enum.CommandScope.html#variant.Application "variant pert_ui::CommandScope::Application")
scope therefore does not run while focus is in the list; only a
[`Capture`](enum.CommandScope.html#variant.Capture "variant pert_ui::CommandScope::Capture")
binding runs first.

[Source](../src/pert_ui_runtime/virtual_list.rs.html#1180)

#### pub fn [reorder_permission](#method.reorder_permission)( self, permission: impl [Into](https://doc.rust-lang.org/1.99.1/core/convert/trait.Into.html "trait core::convert::Into")\<[Permission](struct.Permission.html "struct pert_ui::Permission")\>, ) -\> [VirtualList](struct.VirtualList.html "struct pert_ui::VirtualList")

Restricts pointer and keyboard reordering and accepting collection
drops. Default allowed; a callback installed through
[`Self::reorderable`](struct.VirtualList.html#method.reorderable "method pert_ui::VirtualList::reorderable")
is still required. Repeated restrictions combine through
[`Permission::and`](struct.Permission.html#method.and "method pert_ui::Permission::and").
Revocation clears an armed press/drop indication and cancels this list’s
active drag. Denied Alt-arrow reordering falls through to ordinary
selection navigation. Selection, activation and scrolling remain
available. Bound changes are observed during update without polling.
Describing the list again replaces the restriction with the new
description’s; a fixed denial there cancels like a revocation.

[Source](../src/pert_ui_runtime/virtual_list.rs.html#1187)

#### pub fn [drag_operations](#method.drag_operations)(self, operations: [DropOperations](struct.DropOperations.html "struct pert_ui::DropOperations")) -\> [VirtualList](struct.VirtualList.html "struct pert_ui::VirtualList")

The operations a drag started by this list permits; a move by default.
Has no effect without
[`Self::reorderable`](struct.VirtualList.html#method.reorderable "method pert_ui::VirtualList::reorderable").

[Source](../src/pert_ui_runtime/virtual_list.rs.html#1244-1247)

#### pub fn [placement](#method.placement)( self, rule: impl [Fn](https://doc.rust-lang.org/1.99.1/core/ops/function/trait.Fn.html "trait core::ops::function::Fn")(&\[[String](https://doc.rust-lang.org/1.99.1/alloc/string/struct.String.html "struct alloc::string::String")\], &[ListDropPoint](struct.ListDropPoint.html "struct pert_ui::ListDropPoint")) -\> [Option](https://doc.rust-lang.org/1.99.1/core/option/enum.Option.html "enum core::option::Option")\<[ListDropPlacement](struct.ListDropPlacement.html "struct pert_ui::ListDropPlacement")\> + 'static, ) -\> [VirtualList](struct.VirtualList.html "struct pert_ui::VirtualList")

Replaces the item boundaries a drop uses with a rule of the owner’s, as
[`Tree::reorderable`](struct.Tree.html#method.reorderable "method pert_ui::Tree::reorderable")
does to offer dropping into a node and after an enclosing level. Has no
effect without
[`Self::reorderable`](struct.VirtualList.html#method.reorderable "method pert_ui::VirtualList::reorderable").

Timing class: [in the
input](https://github.com/pertyjons/pert-ui/blob/main/docs/contracts.md#in-the-input),
on each movement of the drag.

While a
[`ListDragItems`](struct.ListDragItems.html "struct pert_ui::ListDragItems")
drag is over the list as the drop candidate, the rule receives the
dragged keys and a
[`ListDropPoint`](struct.ListDropPoint.html "struct pert_ui::ListDropPoint"),
and answers where a drop there goes and how the list marks it, or `None`
to forbid one there: the list then answers `Forbidden`, draws no mark,
and a release cancels. The position it answers is what the reorder
callback receives. It runs on the UI thread on every pointer movement of
the drag, so it should answer from the application’s model without
building rows.

``` rust
use pert_ui::prelude::*;
use pert_ui::GeometryError;
struct Groups;
impl ListItems for Groups {
    fn len(&self) -> usize { 2 }
    fn key(&self, index: usize) -> Option<String> {
        ["reports", "settings"].get(index).map(|key| (*key).to_owned())
    }
    fn build(&self, index: usize) -> Option<Element> {
        let key = self.key(index)?;
        Some(Element::new(key.clone(), Label::new(key)))
    }
}
let ui = Runtime::new();
let items = ui.signal(ListSource::new(Groups));
// Every drop goes into the group under the pointer, never between.
let list = VirtualList::new(items.read_only(), 24.0)?
    .reorderable(|_, drop| Ok(matches!(drop.position(), ListDropPosition::Into(_))))
    .placement(|_keys, at| {
        let (Some(index), Some(key)) = (at.index(), at.key()) else {
            return None;
        };
        Some(ListDropPlacement::new(
            ListDropPosition::Into(key.to_owned()),
            ListDropMark::Into(index),
        ))
    });
```

## Trait Implementations[§](#trait-implementations)

[Source](../src/pert_ui_runtime/virtual_list.rs.html#1875)[§](#impl-Component-for-VirtualList)

### impl [Component](trait.Component.html "trait pert_ui::Component") for [VirtualList](struct.VirtualList.html "struct pert_ui::VirtualList")

[Source](../src/pert_ui_runtime/virtual_list.rs.html#1876)[§](#method.deliver_notifications)

#### fn [deliver_notifications](trait.Component.html#method.deliver_notifications)( &mut self, cx: &mut [EventContext](struct.EventContext.html "struct pert_ui::EventContext"), ) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[()](https://doc.rust-lang.org/1.99.1/std/primitive.unit.html), [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\>

Delivers the change notifications this component queued, after it asked
for this call through a
[`Notifier`](struct.Notifier.html "struct pert_ui::Notifier"). [Read
more](trait.Component.html#method.deliver_notifications)

[Source](../src/pert_ui_runtime/virtual_list.rs.html#1888)[§](#method.reconcile)

#### fn [reconcile](trait.Component.html#method.reconcile)(&mut self, next: [VirtualList](struct.VirtualList.html "struct pert_ui::VirtualList")) -\> [Option](https://doc.rust-lang.org/1.99.1/core/option/enum.Option.html "enum core::option::Option")\<[Invalidation](enum.Invalidation.html "enum pert_ui::Invalidation")\>

Applies a new same-key, same-type description to this mounted component.
[Read more](trait.Component.html#method.reconcile)

[Source](../src/pert_ui_runtime/virtual_list.rs.html#1921)[§](#method.update)

#### fn [update](trait.Component.html#tymethod.update)(&mut self, cx: &mut [UpdateContext](struct.UpdateContext.html "struct pert_ui::UpdateContext")) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[()](https://doc.rust-lang.org/1.99.1/std/primitive.unit.html), [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\>

Refreshes retained data according to the context’s invalidation. [Read
more](trait.Component.html#tymethod.update)

[Source](../src/pert_ui_runtime/virtual_list.rs.html#2075)[§](#method.can_reveal)

#### fn [can_reveal](trait.Component.html#method.can_reveal)(&self) -\> [bool](https://doc.rust-lang.org/1.99.1/std/primitive.bool.html)

Whether this component can reveal clipped descendants by scrolling.
Default is false. Automatic focus traversal includes clipped descendants
only where an ancestor supports this public contract.

[Source](../src/pert_ui_runtime/virtual_list.rs.html#2079)[§](#method.content_rows)

#### fn [content_rows](trait.Component.html#method.content_rows)(&self) -\> [Option](https://doc.rust-lang.org/1.99.1/core/option/enum.Option.html "enum core::option::Option")\<[Rc](https://doc.rust-lang.org/1.99.1/alloc/rc/struct.Rc.html "struct alloc::rc::Rc")\<dyn [ContentRows](trait.ContentRows.html "trait pert_ui::ContentRows")\>\>

Searchable rows this collection offers to [content
search](struct.Runtime.html#method.content_search "method pert_ui::Runtime::content_search")
beyond its mounted children. [Read
more](trait.Component.html#method.content_rows)

[Source](../src/pert_ui_runtime/virtual_list.rs.html#2085)[§](#method.try_content_rows)

#### fn [try_content_rows](trait.Component.html#method.try_content_rows)(&self) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[Option](https://doc.rust-lang.org/1.99.1/core/option/enum.Option.html "enum core::option::Option")\<[Rc](https://doc.rust-lang.org/1.99.1/alloc/rc/struct.Rc.html "struct alloc::rc::Rc")\<dyn [ContentRows](trait.ContentRows.html "trait pert_ui::ContentRows")\>\>, [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\>

Returns current searchable collection rows with read failures preserved.
[Read more](trait.Component.html#method.try_content_rows)

[Source](../src/pert_ui_runtime/virtual_list.rs.html#2100)[§](#method.descendant_tab_stops)

#### fn [descendant_tab_stops](trait.Component.html#method.descendant_tab_stops)(&self) -\> [bool](https://doc.rust-lang.org/1.99.1/std/primitive.bool.html)

Whether this component’s descendants take part in keyboard traversal
([`Runtime::focus_next`](struct.Runtime.html#method.focus_next "method pert_ui::Runtime::focus_next"),
Tab and Shift+Tab). [Read
more](trait.Component.html#method.descendant_tab_stops)

[Source](../src/pert_ui_runtime/virtual_list.rs.html#2104)[§](#method.focusable)

#### fn [focusable](trait.Component.html#method.focusable)(&self) -\> [bool](https://doc.rust-lang.org/1.99.1/std/primitive.bool.html)

Whether automatic keyboard traversal and pointer presses may focus this
component. Default is false; explicit low-level `Runtime::focus` remains
available. [Read more](trait.Component.html#method.focusable)

[Source](../src/pert_ui_runtime/virtual_list.rs.html#2112-2116)[§](#method.measure)

#### fn [measure](trait.Component.html#method.measure)( &self, cx: &[MeasureContext](struct.MeasureContext.html "struct pert_ui::MeasureContext")\<'\_\>, available: [Constraints](struct.Constraints.html "struct pert_ui::Constraints"), ) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[Size](struct.Size.html "struct pert_ui::Size"), [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\>

Measures content in logical pixels under the available space. [Read
more](trait.Component.html#method.measure)

[Source](../src/pert_ui_runtime/virtual_list.rs.html#2134)[§](#method.arrange)

#### fn [arrange](trait.Component.html#method.arrange)( &self, cx: &mut [ArrangeContext](struct.ArrangeContext.html "struct pert_ui::ArrangeContext")\<'\_\>, size: [Size](struct.Size.html "struct pert_ui::Size"), ) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[()](https://doc.rust-lang.org/1.99.1/std/primitive.unit.html), [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\>

Places every visible child once, relative to this component’s origin.
[Read more](trait.Component.html#method.arrange)

[Source](../src/pert_ui_runtime/virtual_list.rs.html#2162-2166)[§](#method.pointer_input)

#### fn [pointer_input](trait.Component.html#method.pointer_input)( &mut self, cx: &mut [EventContext](struct.EventContext.html "struct pert_ui::EventContext"), event: [PointerEvent](struct.PointerEvent.html "struct pert_ui::PointerEvent"), ) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[InputOutcome](enum.InputOutcome.html "enum pert_ui::InputOutcome"), [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\>

Handles a pointer event, bubbling ignored events through logical
ancestors. Captured events keep their owner even outside its clipped
bounds. [Read more](trait.Component.html#method.pointer_input)

[Source](../src/pert_ui_runtime/virtual_list.rs.html#2334)[§](#method.text_input)

#### fn [text_input](trait.Component.html#method.text_input)( &mut self, cx: &mut [EventContext](struct.EventContext.html "struct pert_ui::EventContext"), text: &[str](https://doc.rust-lang.org/1.99.1/std/primitive.str.html), ) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[()](https://doc.rust-lang.org/1.99.1/std/primitive.unit.html), [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\>

Receives committed UTF-8 text independently of physical keys and
repeats. [Read more](trait.Component.html#method.text_input)

[Source](../src/pert_ui_runtime/virtual_list.rs.html#2341-2346)[§](#method.keyboard_event)

#### fn [keyboard_event](trait.Component.html#method.keyboard_event)( &mut self, cx: &mut [EventContext](struct.EventContext.html "struct pert_ui::EventContext"), event: [KeyEvent](struct.KeyEvent.html "struct pert_ui::KeyEvent"), keys: &[KeyboardState](struct.KeyboardState.html "struct pert_ui::KeyboardState"), ) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[InputOutcome](enum.InputOutcome.html "enum pert_ui::InputOutcome"), [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\>

Handles keyboard input with explicit propagation. [Read
more](trait.Component.html#method.keyboard_event)

[Source](../src/pert_ui_runtime/virtual_list.rs.html#2452)[§](#method.reveal)

#### fn [reveal](trait.Component.html#method.reveal)( &mut self, cx: &mut [EventContext](struct.EventContext.html "struct pert_ui::EventContext"), rect: [Rect](struct.Rect.html "struct pert_ui::Rect"), ) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[bool](https://doc.rust-lang.org/1.99.1/std/primitive.bool.html), [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\>

Reveals a descendant’s current window-coordinate rectangle, returning
whether its placement changed. Called nearest ancestor first during
focus transfer. Implementations clamp scrolling to content bounds and
invalidate layout when returning true. Runtime lays out before asking
the next ancestor. [Read more](trait.Component.html#method.reveal)

[Source](../src/pert_ui_runtime/virtual_list.rs.html#2466)[§](#method.hit_test_foreground)

#### fn [hit_test_foreground](trait.Component.html#method.hit_test_foreground)(&self, point: [Point](struct.Point.html "struct pert_ui::Point")) -\> [bool](https://doc.rust-lang.org/1.99.1/std/primitive.bool.html)

Tests an interactive foreground decoration above this component’s
children. [Read more](trait.Component.html#method.hit_test_foreground)

[Source](../src/pert_ui_runtime/virtual_list.rs.html#2470)[§](#method.input_reset)

#### fn [input_reset](trait.Component.html#method.input_reset)(&mut self)

Releases component-owned held-key actions when its input context is
lost. [Read more](trait.Component.html#method.input_reset)

[Source](../src/pert_ui_runtime/virtual_list.rs.html#2481)[§](#method.hover_changed)

#### fn [hover_changed](trait.Component.html#method.hover_changed)(&mut self, cx: &mut [EventContext](struct.EventContext.html "struct pert_ui::EventContext"), hovered: [bool](https://doc.rust-lang.org/1.99.1/std/primitive.bool.html))

Reports when the frontmost pointer enters/leaves this logical subtree,
including native focus loss. Default does nothing. Called at the
runtime’s delivered event time, independently of pressed-button gestures
and pointer bubbling; use it for local hover timing, not host actions or
I/O. Hiding and removal cancel context through `cancel_input` instead.

[Source](../src/pert_ui_runtime/virtual_list.rs.html#2488-2492)[§](#method.drag_event)

#### fn [drag_event](trait.Component.html#method.drag_event)( &mut self, cx: &mut [EventContext](struct.EventContext.html "struct pert_ui::EventContext"), event: &[DragEvent](struct.DragEvent.html "struct pert_ui::DragEvent")\<'\_\>, ) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[DropResponse](enum.DropResponse.html "enum pert_ui::DropResponse"), [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\>

Offers a drag session to this component, or tells it how its part ended.
[Read more](trait.Component.html#method.drag_event)

[Source](../src/pert_ui_runtime/virtual_list.rs.html#2552)[§](#method.drag_finished)

#### fn [drag_finished](trait.Component.html#method.drag_finished)( &mut self, cx: &mut [EventContext](struct.EventContext.html "struct pert_ui::EventContext"), \_: [DragOutcome](enum.DragOutcome.html "enum pert_ui::DragOutcome"), ) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[()](https://doc.rust-lang.org/1.99.1/std/primitive.unit.html), [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\>

Tells the source how an accepted drag request or adopted session ended.
Called exactly once while the source is mounted, including cancellation
before adoption. An adopted session notifies its target/observers first,
then releases capture and unmounts the preview before notifying the
source. [Read more](trait.Component.html#method.drag_finished)

[Source](../src/pert_ui_runtime/virtual_list.rs.html#2559)[§](#method.timer_elapsed)

#### fn [timer_elapsed](trait.Component.html#method.timer_elapsed)(&mut self, cx: &mut [EventContext](struct.EventContext.html "struct pert_ui::EventContext")) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[()](https://doc.rust-lang.org/1.99.1/std/primitive.unit.html), [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\>

Runs a timed action whose deadline, set with
[`EventContext::set_timer`](struct.EventContext.html#method.set_timer "method pert_ui::EventContext::set_timer"),
has elapsed. [Read more](trait.Component.html#method.timer_elapsed)

[Source](../src/pert_ui_runtime/virtual_list.rs.html#2574)[§](#method.paint)

#### fn [paint](trait.Component.html#method.paint)(&self, painter: &mut [Painter](struct.Painter.html "struct pert_ui::Painter")) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[()](https://doc.rust-lang.org/1.99.1/std/primitive.unit.html), [DrawError](enum.DrawError.html "enum pert_ui::DrawError")\>

Records backend-independent drawing from retained data after an update.
[Read more](trait.Component.html#method.paint)

[Source](../src/pert_ui_runtime/virtual_list.rs.html#2598)[§](#method.paint_foreground)

#### fn [paint_foreground](trait.Component.html#method.paint_foreground)(&self, painter: &mut [Painter](struct.Painter.html "struct pert_ui::Painter")) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[()](https://doc.rust-lang.org/1.99.1/std/primitive.unit.html), [DrawError](enum.DrawError.html "enum pert_ui::DrawError")\>

Records decorations above this component’s children, inside the same
clip. Uses the same retained cache and restrictions as `paint`. Default
draws nothing. Useful for scrollbars and focus outlines. Use
`hit_test_foreground` to make foreground decorations interactive above
children. [Read more](trait.Component.html#method.paint_foreground)

[Source](../src/pert_ui_runtime/virtual_list.rs.html#2655)[§](#method.inspect)

#### fn [inspect](trait.Component.html#method.inspect)(&self) -\> [ComponentDetails](struct.ComponentDetails.html "struct pert_ui::ComponentDetails")

Returns explicitly exposed state and actual measured box/style
information. [Read more](trait.Component.html#method.inspect)

[Source](../src/pert_ui_runtime/runtime.rs.html#1176)[§](#method.check_description)

#### fn [check_description](trait.Component.html#method.check_description)(&self) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[()](https://doc.rust-lang.org/1.99.1/std/primitive.unit.html), [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\>

Checks this description before the runtime mounts it or applies it to a
mounted component. The default accepts every description. [Read
more](trait.Component.html#method.check_description)

[Source](../src/pert_ui_runtime/runtime.rs.html#1183)[§](#method.command_target)

#### fn [command_target](trait.Component.html#method.command_target)(&self) -\> [bool](https://doc.rust-lang.org/1.99.1/std/primitive.bool.html)

Declares a content command target independent of focusability or
registered shortcuts. Pointer/focus interaction selects the nearest such
presentation ancestor. Toolbar/menu components keep the default false to
preserve it.

[Source](../src/pert_ui_runtime/runtime.rs.html#1207-1213)[§](#method.text_key_event)

#### fn [text_key_event](trait.Component.html#method.text_key_event)( &mut self, \_cx: &mut [EventContext](struct.EventContext.html "struct pert_ui::EventContext"), \_input: [CommandKeyInput](struct.CommandKeyInput.html "struct pert_ui::CommandKeyInput"), \_transition: [KeyTransition](enum.KeyTransition.html "enum pert_ui::KeyTransition"), \_keyboard: &[KeyboardState](struct.KeyboardState.html "struct pert_ui::KeyboardState"), ) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[TextKeyOutcome](enum.TextKeyOutcome.html "enum pert_ui::TextKeyOutcome"), [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\>

Optional editing phase before capture bindings and ordinary bubbling.
Custom editors return InsertText for printable insertion, Handled for
editing chords (including disabled local undo), and Ignored to continue
from their parent without dispatching the editing action twice. [Read
more](trait.Component.html#method.text_key_event)

[Source](../src/pert_ui_runtime/runtime.rs.html#1240-1244)[§](#method.text_command)

#### fn [text_command](trait.Component.html#method.text_command)( &mut self, \_cx: &mut [EventContext](struct.EventContext.html "struct pert_ui::EventContext"), \_command: [TextCommand](enum.TextCommand.html "enum pert_ui::TextCommand"), ) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[TextCommandOutcome](enum.TextCommandOutcome.html "enum pert_ui::TextCommandOutcome"), [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\>

Performs a text command on this component’s own text: a clipboard or
selection action on its selection and caret, or beginning an edit. [Read
more](trait.Component.html#method.text_command)

[Source](../src/pert_ui_runtime/runtime.rs.html#1251)[§](#method.child_redocked)

#### fn [child_redocked](trait.Component.html#method.child_redocked)(&mut self, \_cx: &mut [EventContext](struct.EventContext.html "struct pert_ui::EventContext"), \_child: &[WidgetId](struct.WidgetId.html "struct pert_ui::WidgetId"))

A retained child returned from a native window. Containers may select
the child by its stable identity. Default does nothing. Called on the UI
thread. The callback must not fail after moving presentation; update
local state only.

[Source](../src/pert_ui_runtime/runtime.rs.html#1261)[§](#method.stacks_children)

#### fn [stacks_children](trait.Component.html#method.stacks_children)(&self) -\> [bool](https://doc.rust-lang.org/1.99.1/std/primitive.bool.html)

Enables independent back-to-front ordering of immediate children.
Default is false. Pointer presses and explicit/keyboard focus raise the
containing child within equal
[`Self::child_stacking_priority`](trait.Component.html#method.child_stacking_priority "method pert_ui::Component::child_stacking_priority")
values before dispatch, including presses consumed by descendants.
Logical child order and identities remain unchanged. Children
intentionally overlap; the parent must provide geometry and clipping
through ordinary layout. Promoted menus/dialogs retain their existing
surface ordering and modality. Request layout when this flag changes to
refresh stationary hover and drag targets.

[Source](../src/pert_ui_runtime/runtime.rs.html#1324)[§](#method.child_stacking_priority)

#### fn [child_stacking_priority](trait.Component.html#method.child_stacking_priority)(&self, \_key: &[str](https://doc.rust-lang.org/1.99.1/std/primitive.str.html)) -\> [i32](https://doc.rust-lang.org/1.99.1/std/primitive.i32.html)

Relative presentation priority of one immediate child of a stacking
parent. [Read more](trait.Component.html#method.child_stacking_priority)

[Source](../src/pert_ui_runtime/runtime.rs.html#1331)[§](#method.layout_position)

#### fn [layout_position](trait.Component.html#method.layout_position)(&self) -\> [Point](struct.Point.html "struct pert_ui::Point")

Preferred parent-local logical origin for freely positioned containers.
Ordinary flow containers ignore it. Default is zero. Change retained
position in input/update and invalidate layout; never write during
measure.

[Source](../src/pert_ui_runtime/runtime.rs.html#1384)[§](#method.child_transform)

#### fn [child_transform](trait.Component.html#method.child_transform)(&self) -\> [ViewTransform](struct.ViewTransform.html "struct pert_ui::ViewTransform")

How this container presents its children: one uniform scale and one
offset. [Read more](trait.Component.html#method.child_transform)

[Source](../src/pert_ui_runtime/runtime.rs.html#1400)[§](#method.child_surface)

#### fn [child_surface](trait.Component.html#method.child_surface)(&self, \_key: &[str](https://doc.rust-lang.org/1.99.1/std/primitive.str.html)) -\> [Option](https://doc.rust-lang.org/1.99.1/core/option/enum.Option.html "enum core::option::Option")\<[SurfaceKind](enum.SurfaceKind.html "enum pert_ui::SurfaceKind")\>

Promotes an immediate child to an anchored window presentation layer.
Default is ordinary layout. The parent allocation is the anchor; runtime
flips/constrains the child’s preferred placement to the window and
escapes ancestor clips while retaining logical ownership. Child
visibility controls admission. Modal kinds allocate the full
window/owner scope; context menus and submenus use the child’s local
placement and notifications use the window’s bottom right.
Implementations provide scroll access for oversized content and never
promote interactive tooltip descendants. Changing a visible child’s kind
runs admission again, including conflict, cancellation and focus policy,
while retaining its original opener until the surface closes. Failed
admission preserves current modal blocking until the actual admission
can be retried.

[Source](../src/pert_ui_runtime/runtime.rs.html#1415-1419)[§](#method.dismiss_surface)

#### fn [dismiss_surface](trait.Component.html#method.dismiss_surface)( &mut self, \_cx: &mut [EventContext](struct.EventContext.html "struct pert_ui::EventContext"), \_reason: [SurfaceDismissal](enum.SurfaceDismissal.html "enum pert_ui::SurfaceDismissal"), ) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[()](https://doc.rust-lang.org/1.99.1/std/primitive.unit.html), [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\>

Closes this owner’s surface in response to shared routing/lifecycle
rules. Must hide its surface child on the next update and release local
actions, except modal owners may ignore Escape when explicit response is
required and ignore outside press, focus leaving and native focus loss.
Ownership removal/hiding and admission failure must always terminate
modal sessions. Do not activate application commands here. [Read
more](trait.Component.html#method.dismiss_surface)

[Source](../src/pert_ui_runtime/runtime.rs.html#1427)[§](#method.cursor)

#### fn [cursor](trait.Component.html#method.cursor)(&self, \_point: [Point](struct.Point.html "struct pert_ui::Point")) -\> [Option](https://doc.rust-lang.org/1.99.1/core/option/enum.Option.html "enum core::option::Option")\<[CursorIcon](enum.CursorIcon.html "enum pert_ui::CursorIcon")\>

Returns a cursor for a point in local logical coordinates, or `None` to
defer to ancestors. Called read-only on the UI thread after layout; no
writes or I/O are allowed. Captured components may return a resize
cursor outside their bounds until the gesture ends. Default defers.

[Source](../src/pert_ui_runtime/runtime.rs.html#1452)[§](#method.tab_stop)

#### fn [tab_stop](trait.Component.html#method.tab_stop)(&self) -\> [bool](https://doc.rust-lang.org/1.99.1/std/primitive.bool.html)

Whether keyboard traversal
([`Runtime::focus_next`](struct.Runtime.html#method.focus_next "method pert_ui::Runtime::focus_next"),
Tab and Shift+Tab) stops at this component. [Read
more](trait.Component.html#method.tab_stop)

[Source](../src/pert_ui_runtime/runtime.rs.html#1504-1509)[§](#method.tab_navigation)

#### fn [tab_navigation](trait.Component.html#method.tab_navigation)( &mut self, \_cx: &mut [EventContext](struct.EventContext.html "struct pert_ui::EventContext"), \_direction: [FocusDirection](enum.FocusDirection.html "enum pert_ui::FocusDirection"), \_transition: [KeyTransition](enum.KeyTransition.html "enum pert_ui::KeyTransition"), ) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[InputOutcome](enum.InputOutcome.html "enum pert_ui::InputOutcome"), [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\>

Offered an unmodified Tab
([`FocusDirection::Forward`](enum.FocusDirection.html#variant.Forward "variant pert_ui::FocusDirection::Forward"))
or Shift+Tab
([`FocusDirection::Backward`](enum.FocusDirection.html#variant.Backward "variant pert_ui::FocusDirection::Backward"))
press or repeat that has reached default keyboard traversal, before the
runtime moves focus. Return
[`InputOutcome::Handled`](enum.InputOutcome.html#variant.Handled "variant pert_ui::InputOutcome::Handled")
to claim it: traversal does not run, and the component moves focus
itself if it wants to, for example to the next cell of a table it is
editing. Default `Ignored`. [Read
more](trait.Component.html#method.tab_navigation)

[Source](../src/pert_ui_runtime/runtime.rs.html#1529)[§](#method.focus_siblings_on_removal)

#### fn [focus_siblings_on_removal](trait.Component.html#method.focus_siblings_on_removal)(&self) -\> [bool](https://doc.rust-lang.org/1.99.1/std/primitive.bool.html)

Whether focus passes to a sibling of the same type when this component
is removed while it holds focus. [Read
more](trait.Component.html#method.focus_siblings_on_removal)

[Source](../src/pert_ui_runtime/runtime.rs.html#1541)[§](#method.receives_unfocused_keys)

#### fn [receives_unfocused_keys](trait.Component.html#method.receives_unfocused_keys)(&self) -\> [bool](https://doc.rust-lang.org/1.99.1/std/primitive.bool.html)

Whether this component receives keys while nothing in its window has
focus. [Read more](trait.Component.html#method.receives_unfocused_keys)

[Source](../src/pert_ui_runtime/runtime.rs.html#1555)[§](#method.conceals_input)

#### fn [conceals_input](trait.Component.html#method.conceals_input)(&self) -\> [bool](https://doc.rust-lang.org/1.99.1/std/primitive.bool.html)

Whether this component holds secret input, such as a password, that
diagnostics must not record. [Read
more](trait.Component.html#method.conceals_input)

[Source](../src/pert_ui_runtime/runtime.rs.html#1674)[§](#method.clips_children)

#### fn [clips_children](trait.Component.html#method.clips_children)(&self) -\> [bool](https://doc.rust-lang.org/1.99.1/std/primitive.bool.html)

Whether descendants clip to this allocation (default true).
Flow/view/group containers return false so reported overflow can reach
an enclosing scroll viewport. Their own drawing/hit box still clips to
their allocation. Scroll viewports and explicit decorated Containers
retain clipping.

[Source](../src/pert_ui_runtime/runtime.rs.html#1681)[§](#method.child_visible)

#### fn [child_visible](trait.Component.html#method.child_visible)(&self, \_key: &[str](https://doc.rust-lang.org/1.99.1/std/primitive.str.html)) -\> [bool](https://doc.rust-lang.org/1.99.1/std/primitive.bool.html)

Controls presentation of an immediate child by its stable sibling key.
Hidden descendants retain state and suspend work. Defaults to true. Read
bindings during update, then return retained state here; changes require
layout invalidation. This combines with explicit runtime/ancestor
visibility.

[Source](../src/pert_ui_runtime/runtime.rs.html#1730-1734)[§](#method.reveal_child)

#### fn [reveal_child](trait.Component.html#method.reveal_child)( &mut self, \_cx: &mut [EventContext](struct.EventContext.html "struct pert_ui::EventContext"), \_key: &[str](https://doc.rust-lang.org/1.99.1/std/primitive.str.html), ) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[ChildReveal](enum.ChildReveal.html "enum pert_ui::ChildReveal"), [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\>

Opens the way to the immediate, mounted child `key` that
[`Self::child_visible`](trait.Component.html#method.child_visible "method pert_ui::Component::child_visible")
currently hides, for
[`Runtime::reveal`](struct.Runtime.html#method.reveal "method pert_ui::Runtime::reveal"),
[`Runtime::reveal_and_focus`](struct.Runtime.html#method.reveal_and_focus "method pert_ui::Runtime::reveal_and_focus"),
[`EventContext::request_reveal`](struct.EventContext.html#method.request_reveal "method pert_ui::EventContext::request_reveal"),
[`EventContext::request_reveal_and_focus`](struct.EventContext.html#method.request_reveal_and_focus "method pert_ui::EventContext::request_reveal_and_focus")
and
[`Runtime::reveal_child`](struct.Runtime.html#method.reveal_child "method pert_ui::Runtime::reveal_child").
[Read more](trait.Component.html#method.reveal_child)

[Source](../src/pert_ui_runtime/runtime.rs.html#1830)[§](#method.fills_row_height)

#### fn [fills_row_height](trait.Component.html#method.fills_row_height)(&self) -\> [bool](https://doc.rust-lang.org/1.99.1/std/primitive.bool.html)

Whether a [`Row`](struct.Row.html "struct pert_ui::Row") stretches this
component to its content height. [Read
more](trait.Component.html#method.fills_row_height)

[Source](../src/pert_ui_runtime/runtime.rs.html#1838)[§](#method.size_limits)

#### fn [size_limits](trait.Component.html#method.size_limits)(&self) -\> [SizeLimits](struct.SizeLimits.html "struct pert_ui::SizeLimits")

Hard outer limits applied to measurement and final allocation. Default
permits any finite nonnegative size. Use retained values only; changes
require layout invalidation. Overflow is clipped, never hidden by
shrinking below a declared minimum. Spare space after a maximum stays
empty.

[Source](../src/pert_ui_runtime/runtime.rs.html#1850-1853)[§](#method.measure_limits)

#### fn [measure_limits](trait.Component.html#method.measure_limits)( &self, \_cx: &[MeasureContext](struct.MeasureContext.html "struct pert_ui::MeasureContext")\<'\_\>, ) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[SizeLimits](struct.SizeLimits.html "struct pert_ui::SizeLimits"), [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\>

Derives hard limits from visible children during layout. Default uses
`size_limits`; composite controls override this to propagate descendant
minimums before a parent allocates/clips them. Use the public child
limits, retained state and validated geometry only. [Read
more](trait.Component.html#method.measure_limits)

[Source](../src/pert_ui_runtime/runtime.rs.html#1900)[§](#method.retain_initial_children)

#### fn [retain_initial_children](trait.Component.html#method.retain_initial_children)(&self) -\> [bool](https://doc.rust-lang.org/1.99.1/std/primitive.bool.html)

Keeps constructor-generated children when this component is described
again. [Read more](trait.Component.html#method.retain_initial_children)

[Source](../src/pert_ui_runtime/runtime.rs.html#1910)[§](#method.take_children)

#### fn [take_children](trait.Component.html#method.take_children)(&mut self) -\> [Vec](https://doc.rust-lang.org/1.99.1/alloc/vec/struct.Vec.html "struct alloc::vec::Vec")\<[Element](struct.Element.html "struct pert_ui::Element")\>

Transfers initial child descriptions into the common retained tree.
[Read more](trait.Component.html#method.take_children)

[Source](../src/pert_ui_runtime/runtime.rs.html#2056)[§](#method.workspace_state)

#### fn [workspace_state](trait.Component.html#method.workspace_state)(&self) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[Option](https://doc.rust-lang.org/1.99.1/core/option/enum.Option.html "enum core::option::Option")\<[ViewState](struct.ViewState.html "struct pert_ui::ViewState")\>, [WorkspaceError](enum.WorkspaceError.html "enum pert_ui::WorkspaceError")\>

Captures this component’s restorable arrangement, or `None` to stay out
of [workspace
persistence](struct.Workspace.html "struct pert_ui::Workspace")
entirely, which is the default and right for almost every component.
[Read more](trait.Component.html#method.workspace_state)

[Source](../src/pert_ui_runtime/runtime.rs.html#2073-2076)[§](#method.workspace_state_with_children)

#### fn [workspace_state_with_children](trait.Component.html#method.workspace_state_with_children)( &self, \_children: &\[[ChildInfo](struct.ChildInfo.html "struct pert_ui::ChildInfo")\], ) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[Option](https://doc.rust-lang.org/1.99.1/core/option/enum.Option.html "enum core::option::Option")\<[ViewState](struct.ViewState.html "struct pert_ui::ViewState")\>, [WorkspaceError](enum.WorkspaceError.html "enum pert_ui::WorkspaceError")\>

Captures arrangement preferences against the current mounted children.
[Read more](trait.Component.html#method.workspace_state_with_children)

[Source](../src/pert_ui_runtime/runtime.rs.html#2098-2101)[§](#method.restore_workspace_state)

#### fn [restore_workspace_state](trait.Component.html#method.restore_workspace_state)( &mut self, \_state: &[ViewState](struct.ViewState.html "struct pert_ui::ViewState"), ) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[Invalidation](enum.Invalidation.html "enum pert_ui::Invalidation"), [RestoreRejection](enum.RestoreRejection.html "enum pert_ui::RestoreRejection")\>

Applies captured state this component produced earlier, returning the
narrowest invalidation the change needs. [Read
more](trait.Component.html#method.restore_workspace_state)

[Source](../src/pert_ui_runtime/runtime.rs.html#2176-2180)[§](#method.restore_workspace_state_with_children)

#### fn [restore_workspace_state_with_children](trait.Component.html#method.restore_workspace_state_with_children)( &mut self, state: &[ViewState](struct.ViewState.html "struct pert_ui::ViewState"), \_children: &\[[ChildInfo](struct.ChildInfo.html "struct pert_ui::ChildInfo")\], ) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[Invalidation](enum.Invalidation.html "enum pert_ui::Invalidation"), [RestoreRejection](enum.RestoreRejection.html "enum pert_ui::RestoreRejection")\>

Restores state using the current mounted children, before a pending
update. [Read
more](trait.Component.html#method.restore_workspace_state_with_children)

[Source](../src/pert_ui_runtime/runtime.rs.html#2323)[§](#method.hit_test)

#### fn [hit_test](trait.Component.html#method.hit_test)(&self, \_point: [Point](struct.Point.html "struct pert_ui::Point")) -\> [bool](https://doc.rust-lang.org/1.99.1/std/primitive.bool.html)

Tests this component’s body at a component-local point in logical
pixels. [Read more](trait.Component.html#method.hit_test)

[Source](../src/pert_ui_runtime/runtime.rs.html#2363)[§](#method.mounted)

#### fn [mounted](trait.Component.html#method.mounted)(&mut self)

Called once after a successful mount, before the first update.

[Source](../src/pert_ui_runtime/runtime.rs.html#2375-2380)[§](#method.key_input)

#### fn [key_input](trait.Component.html#method.key_input)( &mut self, \_context: &mut [EventContext](struct.EventContext.html "struct pert_ui::EventContext"), \_event: [KeyEvent](struct.KeyEvent.html "struct pert_ui::KeyEvent"), \_keyboard: &[KeyboardState](struct.KeyboardState.html "struct pert_ui::KeyboardState"), ) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[()](https://doc.rust-lang.org/1.99.1/std/primitive.unit.html), [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\>

Observes a physical key event after the window’s pressed-key set is
updated. [Read more](trait.Component.html#method.key_input)

[Source](../src/pert_ui_runtime/runtime.rs.html#2389)[§](#method.text_input_state)

#### fn [text_input_state](trait.Component.html#method.text_input_state)(&self) -\> [Option](https://doc.rust-lang.org/1.99.1/core/option/enum.Option.html "enum core::option::Option")\<[TextInputState](struct.TextInputState.html "struct pert_ui::TextInputState")\>

Returns a local IME candidate anchor for an editable text recipient.
Default disables native composition. Called read-only after
layout/paint; implementations must use their current caret and never
mutate bindings. Advance
[`TextInputState::session`](struct.TextInputState.html#structfield.session "field pert_ui::TextInputState::session")
when canceling composition, replacing bound text or repositioning its
caret during composition. Never reuse a session; return `None` on
exhaustion.

[Source](../src/pert_ui_runtime/runtime.rs.html#2400-2404)[§](#method.text_preedit)

#### fn [text_preedit](trait.Component.html#method.text_preedit)( &mut self, \_cx: &mut [EventContext](struct.EventContext.html "struct pert_ui::EventContext"), \_preedit: &[TextPreedit](struct.TextPreedit.html "struct pert_ui::TextPreedit"), ) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[()](https://doc.rust-lang.org/1.99.1/std/primitive.unit.html), [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\>

Receives temporary composition without committing it to the text
binding. Empty preedit cancels composition. Clear temporary state on
context loss. [Read more](trait.Component.html#method.text_preedit)

[Source](../src/pert_ui_runtime/runtime.rs.html#2507)[§](#method.focus_gained)

#### fn [focus_gained](trait.Component.html#method.focus_gained)(&mut self, \_cx: &mut [EventContext](struct.EventContext.html "struct pert_ui::EventContext"))

Receives focus after the runtime has installed this recipient, at the
event’s elapsed time. Default does nothing. Use this infallible
UI-thread lifecycle notification for local input/timing state; do not
perform host actions or I/O. Physical held keys are not replayed.
Fallible state work belongs in a subsequent update or explicit input
callback.

[Source](../src/pert_ui_runtime/runtime.rs.html#2530)[§](#method.focus_lost)

#### fn [focus_lost](trait.Component.html#method.focus_lost)(&mut self, cx: &mut [EventContext](struct.EventContext.html "struct pert_ui::EventContext")) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[()](https://doc.rust-lang.org/1.99.1/std/primitive.unit.html), [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\>

Ends focus on an in-window transfer or window focus loss, after the
runtime has released this component’s pointer capture, edit sessions and
deadlines. A transfer into this component’s own subtree, such as into an
embedded
[`DraftEditor`](struct.DraftEditor.html "struct pert_ui::DraftEditor"),
keeps them: the component still owns the interaction it started. [Read
more](trait.Component.html#method.focus_lost)

[Source](../src/pert_ui_runtime/runtime.rs.html#2591-2595)[§](#method.focus_within_changed)

#### fn [focus_within_changed](trait.Component.html#method.focus_within_changed)( &mut self, \_cx: &mut [EventContext](struct.EventContext.html "struct pert_ui::EventContext"), \_within: [bool](https://doc.rust-lang.org/1.99.1/std/primitive.bool.html), ) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[()](https://doc.rust-lang.org/1.99.1/std/primitive.unit.html), [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\>

Reports that keyboard focus entered (`within` true) or left (`within`
false) this component’s subtree while the component itself is not the
focused recipient, which gets
[`Self::focus_gained`](trait.Component.html#method.focus_gained "method pert_ui::Component::focus_gained")
and
[`Self::focus_lost`](trait.Component.html#method.focus_lost "method pert_ui::Component::focus_lost")
instead. Default does nothing. [Read
more](trait.Component.html#method.focus_within_changed)

[Source](../src/pert_ui_runtime/runtime.rs.html#2628)[§](#method.cancel_input)

#### fn [cancel_input](trait.Component.html#method.cancel_input)(&mut self, cx: &mut [EventContext](struct.EventContext.html "struct pert_ui::EventContext"))

Cancels component-owned input with access to its invalidation context.
[Read more](trait.Component.html#method.cancel_input)

[Source](../src/pert_ui_runtime/runtime.rs.html#2671)[§](#method.visibility_changed)

#### fn [visibility_changed](trait.Component.html#method.visibility_changed)(&mut self, \_visible: [bool](https://doc.rust-lang.org/1.99.1/std/primitive.bool.html))

Called on a visibility transition, after hidden editing has been
cancelled.

[Source](../src/pert_ui_runtime/runtime.rs.html#2673)[§](#method.unmounted)

#### fn [unmounted](trait.Component.html#method.unmounted)(&mut self)

Called once after subscriptions, deadlines and edit ownership are
released.

[Source](../src/pert_ui_runtime/virtual_list.rs.html#597)[§](#impl-Debug-for-VirtualList)

### impl [Debug](https://doc.rust-lang.org/1.99.1/core/fmt/trait.Debug.html "trait core::fmt::Debug") for [VirtualList](struct.VirtualList.html "struct pert_ui::VirtualList")

Shows only the type. A description’s content, bindings and callbacks are
not formatted; where the description is mounted as an
[`Element`](struct.Element.html "struct pert_ui::Element"), that
element’s `Debug` shows its key.

[Source](../src/pert_ui_runtime/virtual_list.rs.html#597)[§](#method.fmt)

#### fn [fmt](https://doc.rust-lang.org/1.99.1/core/fmt/trait.Debug.html#tymethod.fmt)(&self, f: &mut [Formatter](https://doc.rust-lang.org/1.99.1/core/fmt/struct.Formatter.html "struct core::fmt::Formatter")\<'\_\>) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[()](https://doc.rust-lang.org/1.99.1/std/primitive.unit.html), [Error](https://doc.rust-lang.org/1.99.1/core/fmt/struct.Error.html "struct core::fmt::Error")\>

Formats the value using the given formatter. [Read
more](https://doc.rust-lang.org/1.99.1/core/fmt/trait.Debug.html#tymethod.fmt)

## Auto Trait Implementations[§](#synthetic-implementations)

[§](#impl-Freeze-for-VirtualList)

### impl \![Freeze](https://doc.rust-lang.org/1.99.1/core/marker/trait.Freeze.html "trait core::marker::Freeze") for [VirtualList](struct.VirtualList.html "struct pert_ui::VirtualList")

[§](#impl-RefUnwindSafe-for-VirtualList)

### impl \![RefUnwindSafe](https://doc.rust-lang.org/1.99.1/core/panic/unwind_safe/trait.RefUnwindSafe.html "trait core::panic::unwind_safe::RefUnwindSafe") for [VirtualList](struct.VirtualList.html "struct pert_ui::VirtualList")

[§](#impl-Send-for-VirtualList)

### impl \![Send](https://doc.rust-lang.org/1.99.1/core/marker/trait.Send.html "trait core::marker::Send") for [VirtualList](struct.VirtualList.html "struct pert_ui::VirtualList")

[§](#impl-Sync-for-VirtualList)

### impl \![Sync](https://doc.rust-lang.org/1.99.1/core/marker/trait.Sync.html "trait core::marker::Sync") for [VirtualList](struct.VirtualList.html "struct pert_ui::VirtualList")

[§](#impl-UnwindSafe-for-VirtualList)

### impl \![UnwindSafe](https://doc.rust-lang.org/1.99.1/core/panic/unwind_safe/trait.UnwindSafe.html "trait core::panic::unwind_safe::UnwindSafe") for [VirtualList](struct.VirtualList.html "struct pert_ui::VirtualList")

[§](#impl-Unpin-for-VirtualList)

### impl [Unpin](https://doc.rust-lang.org/1.99.1/core/marker/trait.Unpin.html "trait core::marker::Unpin") for [VirtualList](struct.VirtualList.html "struct pert_ui::VirtualList")

[§](#impl-UnsafeUnpin-for-VirtualList)

### impl [UnsafeUnpin](https://doc.rust-lang.org/1.99.1/core/marker/trait.UnsafeUnpin.html "trait core::marker::UnsafeUnpin") for [VirtualList](struct.VirtualList.html "struct pert_ui::VirtualList")

## Blanket Implementations[§](#blanket-implementations)

[Source](https://doc.rust-lang.org/1.99.1/src/core/any.rs.html#141)[§](#impl-Any-for-T)

### impl\<T\> [Any](https://doc.rust-lang.org/1.99.1/core/any/trait.Any.html "trait core::any::Any") for T

where T: 'static +
?[Sized](https://doc.rust-lang.org/1.99.1/core/marker/trait.Sized.html "trait core::marker::Sized"),

[Source](https://doc.rust-lang.org/1.99.1/src/core/any.rs.html#142)[§](#method.type_id)

#### fn [type_id](https://doc.rust-lang.org/1.99.1/core/any/trait.Any.html#tymethod.type_id)(&self) -\> [TypeId](https://doc.rust-lang.org/1.99.1/core/any/struct.TypeId.html "struct core::any::TypeId")

Gets the `TypeId` of `self`. [Read
more](https://doc.rust-lang.org/1.99.1/core/any/trait.Any.html#tymethod.type_id)

[Source](https://doc.rust-lang.org/1.99.1/src/core/borrow.rs.html#212)[§](#impl-Borrow%3CT%3E-for-T)

### impl\<T\> [Borrow](https://doc.rust-lang.org/1.99.1/core/borrow/trait.Borrow.html "trait core::borrow::Borrow")\<T\> for T

where T:
?[Sized](https://doc.rust-lang.org/1.99.1/core/marker/trait.Sized.html "trait core::marker::Sized"),

[Source](https://doc.rust-lang.org/1.99.1/src/core/borrow.rs.html#214)[§](#method.borrow)

#### fn [borrow](https://doc.rust-lang.org/1.99.1/core/borrow/trait.Borrow.html#tymethod.borrow)(&self) -\> [&T](https://doc.rust-lang.org/1.99.1/std/primitive.reference.html)

Immutably borrows from an owned value. [Read
more](https://doc.rust-lang.org/1.99.1/core/borrow/trait.Borrow.html#tymethod.borrow)

[Source](https://doc.rust-lang.org/1.99.1/src/core/borrow.rs.html#221)[§](#impl-BorrowMut%3CT%3E-for-T)

### impl\<T\> [BorrowMut](https://doc.rust-lang.org/1.99.1/core/borrow/trait.BorrowMut.html "trait core::borrow::BorrowMut")\<T\> for T

where T:
?[Sized](https://doc.rust-lang.org/1.99.1/core/marker/trait.Sized.html "trait core::marker::Sized"),

[Source](https://doc.rust-lang.org/1.99.1/src/core/borrow.rs.html#222)[§](#method.borrow_mut)

#### fn [borrow_mut](https://doc.rust-lang.org/1.99.1/core/borrow/trait.BorrowMut.html#tymethod.borrow_mut)(&mut self) -\> [&mut T](https://doc.rust-lang.org/1.99.1/std/primitive.reference.html)

Mutably borrows from an owned value. [Read
more](https://doc.rust-lang.org/1.99.1/core/borrow/trait.BorrowMut.html#tymethod.borrow_mut)

[Source](../src/zerocopy/pointer/invariant.rs.html#257)[§](#impl-CastableFrom%3CST,+Initialized,+Initialized%3E-for-DT)

### impl\<ST, DT\> [CastableFrom](../zerocopy/pointer/invariant/trait.CastableFrom.html "trait zerocopy::pointer::invariant::CastableFrom")\<ST, [Initialized](../zerocopy/pointer/invariant/enum.Initialized.html "enum zerocopy::pointer::invariant::Initialized"), [Initialized](../zerocopy/pointer/invariant/enum.Initialized.html "enum zerocopy::pointer::invariant::Initialized")\> for DT

where ST:
?[Sized](https://doc.rust-lang.org/1.99.1/core/marker/trait.Sized.html "trait core::marker::Sized"),
DT:
?[Sized](https://doc.rust-lang.org/1.99.1/core/marker/trait.Sized.html "trait core::marker::Sized"),

[Source](../src/zerocopy/pointer/invariant.rs.html#255)[§](#impl-CastableFrom%3CST,+Uninit,+Uninit%3E-for-DT)

### impl\<ST, DT\> [CastableFrom](../zerocopy/pointer/invariant/trait.CastableFrom.html "trait zerocopy::pointer::invariant::CastableFrom")\<ST, [Uninit](../zerocopy/pointer/invariant/enum.Uninit.html "enum zerocopy::pointer::invariant::Uninit"), [Uninit](../zerocopy/pointer/invariant/enum.Uninit.html "enum zerocopy::pointer::invariant::Uninit")\> for DT

where ST:
?[Sized](https://doc.rust-lang.org/1.99.1/core/marker/trait.Sized.html "trait core::marker::Sized"),
DT:
?[Sized](https://doc.rust-lang.org/1.99.1/core/marker/trait.Sized.html "trait core::marker::Sized"),

[Source](../src/downcast_rs/lib.rs.html#189)[§](#impl-Downcast-for-T)

### impl\<T\> [Downcast](../downcast_rs/trait.Downcast.html "trait downcast_rs::Downcast") for T

where T:
[Any](https://doc.rust-lang.org/1.99.1/core/any/trait.Any.html "trait core::any::Any"),

[Source](../src/downcast_rs/lib.rs.html#190)[§](#method.into_any)

#### fn [into_any](../downcast_rs/trait.Downcast.html#tymethod.into_any)(self: [Box](https://doc.rust-lang.org/1.99.1/alloc/boxed/struct.Box.html "struct alloc::boxed::Box")\<T\>) -\> [Box](https://doc.rust-lang.org/1.99.1/alloc/boxed/struct.Box.html "struct alloc::boxed::Box")\<dyn [Any](https://doc.rust-lang.org/1.99.1/core/any/trait.Any.html "trait core::any::Any")\>

Convert `Box<dyn Trait>` (where `Trait: Downcast`) to `Box<dyn Any>`.
`Box<dyn Any>` can then be further `downcast` into `Box<ConcreteType>`
where `ConcreteType` implements `Trait`.

[Source](../src/downcast_rs/lib.rs.html#191)[§](#method.into_any_rc)

#### fn [into_any_rc](../downcast_rs/trait.Downcast.html#tymethod.into_any_rc)(self: [Rc](https://doc.rust-lang.org/1.99.1/alloc/rc/struct.Rc.html "struct alloc::rc::Rc")\<T\>) -\> [Rc](https://doc.rust-lang.org/1.99.1/alloc/rc/struct.Rc.html "struct alloc::rc::Rc")\<dyn [Any](https://doc.rust-lang.org/1.99.1/core/any/trait.Any.html "trait core::any::Any")\>

Convert `Rc<Trait>` (where `Trait: Downcast`) to `Rc<Any>`. `Rc<Any>`
can then be further `downcast` into `Rc<ConcreteType>` where
`ConcreteType` implements `Trait`.

[Source](../src/downcast_rs/lib.rs.html#192)[§](#method.as_any)

#### fn [as_any](../downcast_rs/trait.Downcast.html#tymethod.as_any)(&self) -\> &(dyn [Any](https://doc.rust-lang.org/1.99.1/core/any/trait.Any.html "trait core::any::Any") + 'static)

Convert `&Trait` (where `Trait: Downcast`) to `&Any`. This is needed
since Rust cannot generate `&Any`’s vtable from `&Trait`’s.

[Source](../src/downcast_rs/lib.rs.html#193)[§](#method.as_any_mut)

#### fn [as_any_mut](../downcast_rs/trait.Downcast.html#tymethod.as_any_mut)(&mut self) -\> &mut (dyn [Any](https://doc.rust-lang.org/1.99.1/core/any/trait.Any.html "trait core::any::Any") + 'static)

Convert `&mut Trait` (where `Trait: Downcast`) to `&Any`. This is needed
since Rust cannot generate `&mut Any`’s vtable from `&mut Trait`’s.

[Source](../src/pert_ui_runtime/described.rs.html#508)[§](#impl-EachItem-for-C)

### impl\<C\> [EachItem](trait.EachItem.html "trait pert_ui::EachItem") for C

where C: [Component](trait.Component.html "trait pert_ui::Component") +
'static,

[Source](../src/yoke/erased.rs.html#22)[§](#impl-ErasedDestructor-for-T)

### impl\<T\> [ErasedDestructor](../yoke/erased/trait.ErasedDestructor.html "trait yoke::erased::ErasedDestructor") for T

where T: 'static,

[Source](https://doc.rust-lang.org/1.99.1/src/core/convert/mod.rs.html#788)[§](#impl-From%3CT%3E-for-T)

### impl\<T\> [From](https://doc.rust-lang.org/1.99.1/core/convert/trait.From.html "trait core::convert::From")\<T\> for T

[Source](https://doc.rust-lang.org/1.99.1/src/core/convert/mod.rs.html#791)[§](#method.from)

#### fn [from](https://doc.rust-lang.org/1.99.1/core/convert/trait.From.html#tymethod.from)(t: T) -\> T

Returns the argument unchanged.

[Source](../src/tracing/instrument.rs.html#325)[§](#impl-Instrument-for-T)

### impl\<T\> [Instrument](../tracing/instrument/trait.Instrument.html "trait tracing::instrument::Instrument") for T

[Source](../src/tracing/instrument.rs.html#86)[§](#method.instrument)

#### fn [instrument](../tracing/instrument/trait.Instrument.html#method.instrument)(self, span: [Span](../tracing/span/struct.Span.html "struct tracing::span::Span")) -\> [Instrumented](../tracing/instrument/struct.Instrumented.html "struct tracing::instrument::Instrumented")\<Self\> [ⓘ](#)

Instruments this type with the provided
[`Span`](../tracing/span/struct.Span.html "struct tracing::span::Span"),
returning an `Instrumented` wrapper. [Read
more](../tracing/instrument/trait.Instrument.html#method.instrument)

[Source](../src/tracing/instrument.rs.html#128)[§](#method.in_current_span)

#### fn [in_current_span](../tracing/instrument/trait.Instrument.html#method.in_current_span)(self) -\> [Instrumented](../tracing/instrument/struct.Instrumented.html "struct tracing::instrument::Instrumented")\<Self\> [ⓘ](#)

Instruments this type with the
[current](../tracing/span/struct.Span.html#method.current "associated function tracing::span::Span::current")
[`Span`](../tracing/span/struct.Span.html "struct tracing::span::Span"),
returning an `Instrumented` wrapper. [Read
more](../tracing/instrument/trait.Instrument.html#method.in_current_span)

[Source](https://doc.rust-lang.org/1.99.1/src/core/convert/mod.rs.html#770-772)[§](#impl-Into%3CU%3E-for-T)

### impl\<T, U\> [Into](https://doc.rust-lang.org/1.99.1/core/convert/trait.Into.html "trait core::convert::Into")\<U\> for T

where U:
[From](https://doc.rust-lang.org/1.99.1/core/convert/trait.From.html "trait core::convert::From")\<T\>,

[Source](https://doc.rust-lang.org/1.99.1/src/core/convert/mod.rs.html#780)[§](#method.into)

#### fn [into](https://doc.rust-lang.org/1.99.1/core/convert/trait.Into.html#tymethod.into)(self) -\> U

Calls `U::from(self)`.

That is, this conversion is whatever the implementation of
[`From`](https://doc.rust-lang.org/1.99.1/core/convert/trait.From.html "trait core::convert::From")`<T> for U`
chooses to do.

[Source](../src/pert_ui_runtime/layout.rs.html#420)[§](#impl-IntoElement-for-C)

### impl\<C\> [IntoElement](trait.IntoElement.html "trait pert_ui::IntoElement") for C

where C: [Component](trait.Component.html "trait pert_ui::Component") +
'static,

[Source](../src/pert_ui_runtime/layout.rs.html#416)[§](#method.key)

#### fn [key](trait.IntoElement.html#method.key)(self, key: impl [Into](https://doc.rust-lang.org/1.99.1/core/convert/trait.Into.html "trait core::convert::Into")\<[String](https://doc.rust-lang.org/1.99.1/alloc/string/struct.String.html "struct alloc::string::String")\>) -\> [Element](struct.Element.html "struct pert_ui::Element")

Describes this component under `key`, as
[`Element::new`](struct.Element.html#method.new "associated function pert_ui::Element::new")
does.

[Source](../src/zerocopy/pointer/invariant.rs.html#276)[§](#impl-Read%3CExclusive,+BecauseExclusive%3E-for-T)

### impl\<T\> [Read](../zerocopy/pointer/invariant/trait.Read.html "trait zerocopy::pointer::invariant::Read")\<[Exclusive](../zerocopy/pointer/invariant/enum.Exclusive.html "enum zerocopy::pointer::invariant::Exclusive"), [BecauseExclusive](../zerocopy/pointer/invariant/enum.BecauseExclusive.html "enum zerocopy::pointer::invariant::BecauseExclusive")\> for T

where T:
?[Sized](https://doc.rust-lang.org/1.99.1/core/marker/trait.Sized.html "trait core::marker::Sized"),

[Source](https://doc.rust-lang.org/1.99.1/src/core/convert/mod.rs.html#830-832)[§](#impl-TryFrom%3CU%3E-for-T)

### impl\<T, U\> [TryFrom](https://doc.rust-lang.org/1.99.1/core/convert/trait.TryFrom.html "trait core::convert::TryFrom")\<U\> for T

where U:
[Into](https://doc.rust-lang.org/1.99.1/core/convert/trait.Into.html "trait core::convert::Into")\<T\>,

[Source](https://doc.rust-lang.org/1.99.1/src/core/convert/mod.rs.html#834)[§](#associatedtype.Error)

#### type [Error](https://doc.rust-lang.org/1.99.1/core/convert/trait.TryFrom.html#associatedtype.Error) = [Infallible](https://doc.rust-lang.org/1.99.1/core/convert/enum.Infallible.html "enum core::convert::Infallible")

The type returned in the event of a conversion error.

[Source](https://doc.rust-lang.org/1.99.1/src/core/convert/mod.rs.html#837)[§](#method.try_from)

#### fn [try_from](https://doc.rust-lang.org/1.99.1/core/convert/trait.TryFrom.html#tymethod.try_from)(value: U) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<T, \<T as [TryFrom](https://doc.rust-lang.org/1.99.1/core/convert/trait.TryFrom.html "trait core::convert::TryFrom")\<U\>\>::[Error](https://doc.rust-lang.org/1.99.1/core/convert/trait.TryFrom.html#associatedtype.Error "type core::convert::TryFrom::Error")\>

Performs the conversion.

[Source](https://doc.rust-lang.org/1.99.1/src/core/convert/mod.rs.html#814-816)[§](#impl-TryInto%3CU%3E-for-T)

### impl\<T, U\> [TryInto](https://doc.rust-lang.org/1.99.1/core/convert/trait.TryInto.html "trait core::convert::TryInto")\<U\> for T

where U:
[TryFrom](https://doc.rust-lang.org/1.99.1/core/convert/trait.TryFrom.html "trait core::convert::TryFrom")\<T\>,

[Source](https://doc.rust-lang.org/1.99.1/src/core/convert/mod.rs.html#818)[§](#associatedtype.Error-1)

#### type [Error](https://doc.rust-lang.org/1.99.1/core/convert/trait.TryInto.html#associatedtype.Error) = \<U as [TryFrom](https://doc.rust-lang.org/1.99.1/core/convert/trait.TryFrom.html "trait core::convert::TryFrom")\<T\>\>::[Error](https://doc.rust-lang.org/1.99.1/core/convert/trait.TryFrom.html#associatedtype.Error "type core::convert::TryFrom::Error")

The type returned in the event of a conversion error.

[Source](https://doc.rust-lang.org/1.99.1/src/core/convert/mod.rs.html#821)[§](#method.try_into)

#### fn [try_into](https://doc.rust-lang.org/1.99.1/core/convert/trait.TryInto.html#tymethod.try_into)(self) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<U, \<U as [TryFrom](https://doc.rust-lang.org/1.99.1/core/convert/trait.TryFrom.html "trait core::convert::TryFrom")\<T\>\>::[Error](https://doc.rust-lang.org/1.99.1/core/convert/trait.TryFrom.html#associatedtype.Error "type core::convert::TryFrom::Error")\>

Performs the conversion.

[Source](../src/tracing/instrument.rs.html#393)[§](#impl-WithSubscriber-for-T)

### impl\<T\> [WithSubscriber](../tracing/instrument/trait.WithSubscriber.html "trait tracing::instrument::WithSubscriber") for T

[Source](../src/tracing/instrument.rs.html#176-178)[§](#method.with_subscriber)

#### fn [with_subscriber](../tracing/instrument/trait.WithSubscriber.html#method.with_subscriber)\<S\>(self, subscriber: S) -\> [WithDispatch](../tracing/instrument/struct.WithDispatch.html "struct tracing::instrument::WithDispatch")\<Self\>

where S:
[Into](https://doc.rust-lang.org/1.99.1/core/convert/trait.Into.html "trait core::convert::Into")\<[Dispatch](../tracing_core/dispatcher/struct.Dispatch.html "struct tracing_core::dispatcher::Dispatch")\>,

Attaches the provided
[`Subscriber`](../tracing_core/subscriber/trait.Subscriber.html "trait tracing_core::subscriber::Subscriber")
to this type, returning a
[`WithDispatch`](../tracing/instrument/struct.WithDispatch.html "struct tracing::instrument::WithDispatch")
wrapper. [Read
more](../tracing/instrument/trait.WithSubscriber.html#method.with_subscriber)

[Source](../src/tracing/instrument.rs.html#228)[§](#method.with_current_subscriber)

#### fn [with_current_subscriber](../tracing/instrument/trait.WithSubscriber.html#method.with_current_subscriber)(self) -\> [WithDispatch](../tracing/instrument/struct.WithDispatch.html "struct tracing::instrument::WithDispatch")\<Self\>

Attaches the current
[default](../tracing/dispatcher/index.html#setting-the-default-subscriber "mod tracing::dispatcher")
[`Subscriber`](../tracing_core/subscriber/trait.Subscriber.html "trait tracing_core::subscriber::Subscriber")
to this type, returning a
[`WithDispatch`](../tracing/instrument/struct.WithDispatch.html "struct tracing::instrument::WithDispatch")
wrapper. [Read
more](../tracing/instrument/trait.WithSubscriber.html#method.with_current_subscriber)
