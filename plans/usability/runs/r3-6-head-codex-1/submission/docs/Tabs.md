[pert_ui](index.html)

# Struct Tabs Copy item path

[Source](../src/pert_ui_runtime/tabs.rs.html#277)

``` rust
pub struct Tabs { /* private fields */ }
```

Expand description

Keyed views with first-tab selection, a themed tab strip (at least 36
logical pixels at Standard) and retained inactive state.

Titles use opaque cards with rounded upper corners (the theme corner
radius), square lower corners attached to the strip baseline and
rectangular hit bounds; the selected card has a distinct fill and accent
underline joining the full-width baseline directly beneath the strip.
Focus adds an independent silhouette outline. Default has no close
buttons, and titles move only through
[`Tabs::reorderable`](struct.Tabs.html#method.reorderable "method pert_ui::Tabs::reorderable").
Clicking a title selects on matched primary release. Focused unmodified
Left/Right/Home/End navigate; Ctrl+Tab (Shift reverses, no Alt or Super)
bubbles from content. Keyboard stepping skips detached tabs, and with
every tab detached the keys are ignored. The selected title scrolls into
view; overflow arrows and wheel scrolling expose other titles in narrow
allocations. Wheel input bubbles to ancestors when the title offset
cannot change. Hiding cancels held input. Removing or detaching the
selected tab selects the nearest following tab of the previously shown
order that is still present and docked, otherwise the nearest preceding
one, resolved by key so that tabs inserted in the same description do
not shift it; reordering preserves selection by key. Empty tabs have no
focus target. A strip described with its tabs is checked before anything
mounts: a blank key, two tabs with one key or a blank title fail the
mount or reconciliation with
[`LayoutError::BlankKey`](enum.LayoutError.html#variant.BlankKey "variant pert_ui::LayoutError::BlankKey"),
[`LayoutError::DuplicateKey`](enum.LayoutError.html#variant.DuplicateKey "variant pert_ui::LayoutError::DuplicateKey")
or
[`LayoutError::BlankLabel`](enum.LayoutError.html#variant.BlankLabel "variant pert_ui::LayoutError::BlankLabel"),
and the mounted strip, if any, is left as it was. Children handed to the
strip as raw elements (`Element::children` or
[`Runtime::reconcile_children`](struct.Runtime.html#method.reconcile_children "method pert_ui::Runtime::reconcile_children"))
must carry display labels too, normally from `Tab::into_element`; a
missing or whitespace-only label there returns
[`LayoutError::BlankLabel`](enum.LayoutError.html#variant.BlankLabel "variant pert_ui::LayoutError::BlankLabel")
from the update that reads it. Tab keys/titles/content are separate.
Native drag-out is opt-in through detachable. Detached content retains
its owner.

The strip takes part in
[`Workspace`](struct.Workspace.html "struct pert_ui::Workspace")
persistence as the `tabs` kind: it captures the selected tab key, and
each tab’s content has its own `viewport` part
(`tabs/<tab key>/viewport`), so per-tab scroll positions are captured
with it. docs/contracts.md describes the document.

Titles and overflow arrows use SurfaceHover while hovered, without
moving neighboring tabs.

A detachable strip reserves an `OpenInNew` marker slot in every title,
so revealing the marker never reflows the strip, including for tabs that
[`Tab::detachable(false)`](struct.Tab.html#method.detachable "method pert_ui::Tab::detachable")
excludes: those offer no marker, no Move cursor and no drag-out. A
permitted docked title shows the marker in Muted while the pointer is
over it and uses the Move cursor; a title being dragged out, and one
whose view is already detached, draw an emptied SurfaceContainer slot
with a PanelBorder outline, a Muted title and the marker in Primary. The
marker is presentation only: the whole title stays the hit area, and
pressing a detached title activates its window.

Application code selects a tab by key with
[`Runtime::reveal_child`](struct.Runtime.html#method.reveal_child "method pert_ui::Runtime::reveal_child")`(&tabs, key)`,
the same selection transition a click on the title runs, so workspace
capture records it and a detached tab’s window is asked for access
instead; focus stays where it is. The strip implements
[`Component::reveal_child`](trait.Component.html#method.reveal_child "method pert_ui::Component::reveal_child")
through that transition, so
[`Runtime::reveal`](struct.Runtime.html#method.reveal "method pert_ui::Runtime::reveal")
and
[`Runtime::reveal_and_focus`](struct.Runtime.html#method.reveal_and_focus "method pert_ui::Runtime::reveal_and_focus")
select the tab that holds their target. While a tab is being dragged
out, the strip refuses to change its selection; a request for the tab
already shown reports
[`ChildReveal::AlreadyVisible`](enum.ChildReveal.html#variant.AlreadyVisible "variant pert_ui::ChildReveal::AlreadyVisible"),
since nothing needs to change.

Describing a mounted strip again (through `Runtime::reconcile_children`)
applies its new
[`Self::detachable`](struct.Tabs.html#method.detachable "method pert_ui::Tabs::detachable")
setting and
[`Self::reorderable`](struct.Tabs.html#method.reorderable "method pert_ui::Tabs::reorderable")
callback, or removes them. The selection, title scrolling, detached
windows and a drag session in progress are retained; the session follows
the new settings from its next event.

## Implementations[§](#implementations)

[Source](../src/pert_ui_runtime/tabs.rs.html#344)[§](#impl-Tabs)

### impl [Tabs](struct.Tabs.html "struct pert_ui::Tabs")

[Source](../src/pert_ui_runtime/tabs.rs.html#347)

#### pub fn [new](#method.new)(tabs: impl [IntoIterator](https://doc.rust-lang.org/1.99.1/core/iter/traits/collect/trait.IntoIterator.html "trait core::iter::traits::collect::IntoIterator")\<Item = [Tab](struct.Tab.html "struct pert_ui::Tab")\>) -\> [Tabs](struct.Tabs.html "struct pert_ui::Tabs")

Collects tabs; the first tab is initially selected unless
[`Self::selected`](struct.Tabs.html#method.selected "method pert_ui::Tabs::selected")
names another, with all view state retained.

[Source](../src/pert_ui_runtime/tabs.rs.html#432)

#### pub fn [selected](#method.selected)(self, key: impl [Into](https://doc.rust-lang.org/1.99.1/core/convert/trait.Into.html "trait core::convert::Into")\<[String](https://doc.rust-lang.org/1.99.1/alloc/string/struct.String.html "struct alloc::string::String")\>) -\> [Tabs](struct.Tabs.html "struct pert_ui::Tabs")

Chooses the tab shown when the strip mounts, by key, instead of the
first one. It applies only at mount: a restored workspace, the user and
reveals decide after that, and describing the strip again with another
key does not change the retained selection. The initial selection is not
reported to
[`Self::on_select`](struct.Tabs.html#method.on_select "method pert_ui::Tabs::on_select").

The key must name one of the tabs given to
[`Self::new`](struct.Tabs.html#method.new "associated function pert_ui::Tabs::new");
otherwise mounting or reconciling the description fails with
[`LayoutError::UnknownKey`](enum.LayoutError.html#variant.UnknownKey "variant pert_ui::LayoutError::UnknownKey")
before anything changes. A strip whose tabs are handed over as raw
elements, which is what an empty `new` means, cannot check the key when
it is described, so an unknown key then falls back to the first tab at
the first update, as a restored selection that no longer exists would.

``` rust
use pert_ui::prelude::*;
use pert_ui::{LayoutError, Viewport};
use std::time::Duration;
fn tab(key: &str) -> Tab {
    Tab::new(key, key, Element::new("body", Label::new(key)))
}
let mut ui = Runtime::new();
ui.set_viewport(Viewport::new(400.0, 200.0, 1.0)?)?;
let tabs = ui.mount_tree(Element::new(
    "tabs",
    Tabs::new([tab("mix"), tab("edit")]).selected("edit"),
))?;
ui.flush(Duration::ZERO)?;
assert!(ui.find_descendant(&tabs, "edit")?.is_some());
assert!(ui.inspect().components.iter().any(|c| c.key == "edit" && c.visible));
assert_eq!(
    ui.mount_tree(Element::new("other", Tabs::new([tab("mix")]).selected("log"))),
    Err(RuntimeError::Layout(LayoutError::UnknownKey("log".into())))
);
```

[Source](../src/pert_ui_runtime/tabs.rs.html#472)

#### pub fn [detachable](#method.detachable)(self, permission: impl [Into](https://doc.rust-lang.org/1.99.1/core/convert/trait.Into.html "trait core::convert::Into")\<[Permission](struct.Permission.html "struct pert_ui::Permission")\>) -\> [Tabs](struct.Tabs.html "struct pert_ui::Tabs")

Enables dragging an inline tab outside its strip and releasing to
detach. Defaults to false. The native host creates an independent
decorated window on release; OS placement is not guaranteed and the
window does not follow the pointer. Existing detached headers request
access instead of duplicating content. The default App reports failed
native operations; inspection retains status.

The gesture is an ordinary [drag
session](struct.EventContext.html#method.start_drag "method pert_ui::EventContext::start_drag"),
built only on the public contract. A press on a title stays a selection
gesture while the pointer is over the strip or less than eight logical
pixels below it, and until it has moved
[`DRAG_THRESHOLD`](constant.DRAG_THRESHOLD.html "constant pert_ui::DRAG_THRESHOLD")
window pixels. Beyond both the session starts, with the rich title as
its preview. The strip forbids the drop over that same area, so
returning there shows the no-drop cursor and a release there detaches
nothing. A release anywhere else, over other content or outside the
window, ends the session as
[`DragOutcome::Unclaimed`](enum.DragOutcome.html#variant.Unclaimed "variant pert_ui::DragOutcome::Unclaimed")
and detaches the tab, if the view that was dragged is still that tab’s.
The native window takes the tab’s plain label as its title, which an
icon-only title keeps. The payload is private to the strip, so no other
target claims it. Escape, hiding and focus loss cancel.

Enabling it also turns on the drag affordance and detached presentation
described on [`Tabs`](struct.Tabs.html "struct pert_ui::Tabs"), and
widens every title by IconGap plus IconSize. Inspection reports
`detachable`, the `detached_tabs` count and, during a drag that would
currently detach on release, the `detaching` tab key. Detachment is not
limited to tabs: any mounted view can move to its own window through
[`EventContext::detach_view`](struct.EventContext.html#method.detach_view "method pert_ui::EventContext::detach_view")
or
[`Runtime::request_detach`](struct.Runtime.html#method.request_detach "method pert_ui::Runtime::request_detach"),
which is how an application offers a command or button instead of a
drag.

The strip offers detachment only after this is called; the permission
may be fixed or bound, and repeated calls combine restrictions. The
strip reads it in its update, so a bound permission turns the affordance
on and off without polling; same-key redescription replaces it. Revoking
it, or a tab’s own permission
([`Tab::detachable`](struct.Tab.html#method.detachable "method pert_ui::Tab::detachable")),
during a drag-out forbids that drop: the release detaches nothing. An
already detached tab keeps its window and can still re-dock.

[Source](../src/pert_ui_runtime/tabs.rs.html#556-559)

#### pub fn [reorderable](#method.reorderable)( self, on_reorder: impl [FnMut](https://doc.rust-lang.org/1.99.1/core/ops/function/trait.FnMut.html "trait core::ops::function::FnMut")(&mut [EventContext](struct.EventContext.html "struct pert_ui::EventContext"), &[ListDrop](struct.ListDrop.html "struct pert_ui::ListDrop")\<'\_\>) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[bool](https://doc.rust-lang.org/1.99.1/std/primitive.bool.html), [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\> + 'static, ) -\> [Tabs](struct.Tabs.html "struct pert_ui::Tabs")

Lets titles be dragged to a new place in the strip, and Alt with Left or
Right move the selected tab one place, with no other modifier.

Timing class: [in the
input](https://github.com/pertyjons/pert-ui/blob/main/docs/contracts.md#in-the-input).

The strip does not own its order: the application describes the tabs, so
`on_reorder` performs the move in the application’s model and describes
the tabs again in the new order. It receives a
[`ListDrop`](struct.ListDrop.html "struct pert_ui::ListDrop") naming the
moved tab’s key and a
[`ListDropPosition`](enum.ListDropPosition.html "enum pert_ui::ListDropPosition")
against the current keys, never an index, and returns whether it made
the edit. A position next to the moved tab itself leaves the order as it
is, which the application is free to ignore. Tab keys, and with them
each view’s retained state, are unchanged by a move. Refusing tells the
drag that the drop was refused.

A press on a title becomes a [drag
session](struct.EventContext.html#method.start_drag "method pert_ui::EventContext::start_drag")
once it has moved
[`DRAG_THRESHOLD`](constant.DRAG_THRESHOLD.html "constant pert_ui::DRAG_THRESHOLD")
window pixels, with the rich title as its preview; below that it is
still a click. While the drag is over the strip, or up to eight logical
pixels below it, a Primary insertion line marks where the tab would go,
and resting over an overflow arrow scrolls the titles. A drop there also
selects the moved tab, as a click would. With
[`Self::detachable`](struct.Tabs.html#method.detachable "method pert_ui::Tabs::detachable")
as well, a release where nothing takes the drag still moves the tab into
its own window; without it such a release does nothing. A detached tab’s
title cannot be dragged. Only this strip’s own tabs are accepted.

The callback runs on the strip, so
[`EventContext::id`](struct.EventContext.html#method.id "method pert_ui::EventContext::id")
names it, and
[`EventContext::defer`](struct.EventContext.html#method.defer "method pert_ui::EventContext::defer")
describes the tabs again once the event is over:

``` rust
use pert_ui::prelude::*;
use pert_ui::{KeyAction, Viewport};
use std::{cell::RefCell, rc::Rc, time::Duration};
fn tab(key: &str) -> Tab {
    Tab::new(key, key, Element::new("body", Label::new(key)))
}
let order = Rc::new(RefCell::new(vec!["mix".to_owned(), "edit".to_owned()]));
let model = order.clone();
let strip = Tabs::new(order.borrow().iter().map(|key| tab(key))).reorderable(
    move |cx, drop| {
        let mut order = model.borrow_mut();
        let moved = drop.items()[0].clone();
        let from = order.iter().position(|key| *key == moved).ok_or(RuntimeError::Unmounted)?;
        // Resolved before the removal: the position may name the moved tab.
        let boundary = match drop.position() {
            ListDropPosition::Before(key) => order.iter().position(|k| k == key),
            ListDropPosition::After(key) => order.iter().position(|k| k == key).map(|i| i + 1),
            _ => None,
        }
        .unwrap_or(order.len());
        order.remove(from);
        order.insert(boundary - usize::from(from < boundary), moved);
        let strip = cx.id();
        let tabs = order.iter().map(|key| tab(key).into_element()).collect();
        cx.defer(move |ui| ui.reconcile_children(&strip, tabs))?;
        Ok(true)
    },
);

let mut ui = Runtime::new();
ui.set_viewport(Viewport::new(400.0, 200.0, 1.0)?)?;
let tabs = ui.mount_tree(Element::new("tabs", strip))?;
ui.flush(Duration::ZERO)?;
// Alt+Right moves the selected tab, the first, one place.
ui.focus(&tabs)?;
ui.key_input(KeyCode::AltLeft, KeyAction::Press)?;
ui.key_input(KeyCode::ArrowRight, KeyAction::Press)?;
ui.flush(Duration::ZERO)?;
assert_eq!(*order.borrow(), ["edit", "mix"]);
let keys: Vec<_> = ui.children(&tabs)?.iter().map(|id| ui.key(id).map(str::to_owned)).collect::<Result<_, _>>()?;
assert_eq!(keys, ["edit", "mix"]);
```

[Source](../src/pert_ui_runtime/tabs.rs.html#632-636)

#### pub fn [on_select](#method.on_select)( self, on_select: impl [FnMut](https://doc.rust-lang.org/1.99.1/core/ops/function/trait.FnMut.html "trait core::ops::function::FnMut")(&mut [EventContext](struct.EventContext.html "struct pert_ui::EventContext"), &[SelectionChange](struct.SelectionChange.html "struct pert_ui::SelectionChange")) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[()](https://doc.rust-lang.org/1.99.1/std/primitive.unit.html), [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\> + 'static, ) -\> [Tabs](struct.Tabs.html "struct pert_ui::Tabs")

Reports every change of the selected tab after it took effect, with its
[`ChangeOrigin`](enum.ChangeOrigin.html "enum pert_ui::ChangeOrigin"),
so an application can follow the pages a user opens without asking the
strip.

Timing class: [after
settling](https://github.com/pertyjons/pert-ui/blob/main/docs/contracts.md#after-settling).

The origins are:

- [`ChangeOrigin::User`](enum.ChangeOrigin.html#variant.User "variant pert_ui::ChangeOrigin::User"):
  a click, a key or a reorder drop on the strip;
- [`ChangeOrigin::Reveal`](enum.ChangeOrigin.html#variant.Reveal "variant pert_ui::ChangeOrigin::Reveal"):
  a reveal, or
  [`Runtime::reveal_child`](struct.Runtime.html#method.reveal_child "method pert_ui::Runtime::reveal_child")
  selecting a tab by key, which is how an application navigates;
- [`ChangeOrigin::Restore`](enum.ChangeOrigin.html#variant.Restore "variant pert_ui::ChangeOrigin::Restore"):
  workspace restoration after the strip’s first update;
- [`ChangeOrigin::Redescription`](enum.ChangeOrigin.html#variant.Redescription "variant pert_ui::ChangeOrigin::Redescription"):
  the selected tab was removed from the description and the strip chose
  its neighbour, or tabs were described into an empty strip;
- [`ChangeOrigin::Presentation`](enum.ChangeOrigin.html#variant.Presentation "variant pert_ui::ChangeOrigin::Presentation"):
  the selected tab was detached into its own window and the strip chose
  its neighbour, or a detached tab returned and became the selection.

The initial selection, including one restored before the first update,
is not a change and is not reported; neither is a request for the tab
already shown, nor pressing a detached tab’s title, which activates its
window instead.

Changes are delivered through
[`Component::deliver_notifications`](trait.Component.html#method.deliver_notifications "method pert_ui::Component::deliver_notifications"):
in an event phase after the update and layout waves of the flush that
follows the change have settled (or of the flush in progress), one call
per change, oldest first, so several changes in one flush arrive in the
order they happened. Input entry points and the reveal operations,
including `Runtime::reveal_child`, settle in a flush of their own, so a
change they cause is delivered (and a callback error returned) before
they return. The callback runs on the strip
([`EventContext::id`](struct.EventContext.html#method.id "method pert_ui::EventContext::id")
names it) and may write bindings and queue work. An error does not undo
the change: the remaining changes are still delivered and the first
error is returned from the flush. A strip removed before delivery
reports nothing.

Describing the strip again replaces the callback; changes not yet
delivered go to the new one, and a description without a callback
discards them.

``` rust
use pert_ui::prelude::*;
use pert_ui::Viewport;
use std::{cell::RefCell, rc::Rc, time::Duration};
fn tab(key: &str) -> Tab {
    Tab::new(key, key, Element::new("body", Label::new(key)))
}
let seen = Rc::new(RefCell::new(Vec::new()));
let log = seen.clone();
let strip = Tabs::new([tab("mix"), tab("edit")]).on_select(move |_, change| {
    log.borrow_mut().push((change.selected.clone(), change.origin));
    Ok(())
});
let mut ui = Runtime::new();
ui.set_viewport(Viewport::new(400.0, 200.0, 1.0)?)?;
let tabs = ui.mount_tree(Element::new("tabs", strip))?;
ui.flush(Duration::ZERO)?;
assert!(seen.borrow().is_empty()); // the initial selection is not a change
ui.reveal_child(&tabs, "edit")?;
ui.flush(Duration::ZERO)?;
assert_eq!(*seen.borrow(), [(Some("edit".to_owned()), ChangeOrigin::Reveal)]);
```

## Trait Implementations[§](#trait-implementations)

[Source](../src/pert_ui_runtime/tabs.rs.html#938)[§](#impl-Component-for-Tabs)

### impl [Component](trait.Component.html "trait pert_ui::Component") for [Tabs](struct.Tabs.html "struct pert_ui::Tabs")

[Source](../src/pert_ui_runtime/tabs.rs.html#939)[§](#method.check_description)

#### fn [check_description](trait.Component.html#method.check_description)(&self) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[()](https://doc.rust-lang.org/1.99.1/std/primitive.unit.html), [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\>

Checks this description before the runtime mounts it or applies it to a
mounted component. The default accepts every description. [Read
more](trait.Component.html#method.check_description)

[Source](../src/pert_ui_runtime/tabs.rs.html#953)[§](#method.reconcile)

#### fn [reconcile](trait.Component.html#method.reconcile)(&mut self, next: [Tabs](struct.Tabs.html "struct pert_ui::Tabs")) -\> [Option](https://doc.rust-lang.org/1.99.1/core/option/enum.Option.html "enum core::option::Option")\<[Invalidation](enum.Invalidation.html "enum pert_ui::Invalidation")\>

Applies a new same-key, same-type description to this mounted component.
[Read more](trait.Component.html#method.reconcile)

[Source](../src/pert_ui_runtime/tabs.rs.html#995)[§](#method.workspace_state)

#### fn [workspace_state](trait.Component.html#method.workspace_state)(&self) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[Option](https://doc.rust-lang.org/1.99.1/core/option/enum.Option.html "enum core::option::Option")\<[ViewState](struct.ViewState.html "struct pert_ui::ViewState")\>, [WorkspaceError](enum.WorkspaceError.html "enum pert_ui::WorkspaceError")\>

Captures this component’s restorable arrangement, or `None` to stay out
of [workspace
persistence](struct.Workspace.html "struct pert_ui::Workspace")
entirely, which is the default and right for almost every component.
[Read more](trait.Component.html#method.workspace_state)

[Source](../src/pert_ui_runtime/tabs.rs.html#1010-1013)[§](#method.workspace_state_with_children)

#### fn [workspace_state_with_children](trait.Component.html#method.workspace_state_with_children)( &self, children: &\[[ChildInfo](struct.ChildInfo.html "struct pert_ui::ChildInfo")\], ) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[Option](https://doc.rust-lang.org/1.99.1/core/option/enum.Option.html "enum core::option::Option")\<[ViewState](struct.ViewState.html "struct pert_ui::ViewState")\>, [WorkspaceError](enum.WorkspaceError.html "enum pert_ui::WorkspaceError")\>

Captures arrangement preferences against the current mounted children.
[Read more](trait.Component.html#method.workspace_state_with_children)

[Source](../src/pert_ui_runtime/tabs.rs.html#1029-1032)[§](#method.restore_workspace_state)

#### fn [restore_workspace_state](trait.Component.html#method.restore_workspace_state)( &mut self, state: &[ViewState](struct.ViewState.html "struct pert_ui::ViewState"), ) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[Invalidation](enum.Invalidation.html "enum pert_ui::Invalidation"), [RestoreRejection](enum.RestoreRejection.html "enum pert_ui::RestoreRejection")\>

Applies captured state this component produced earlier, returning the
narrowest invalidation the change needs. [Read
more](trait.Component.html#method.restore_workspace_state)

[Source](../src/pert_ui_runtime/tabs.rs.html#1041-1045)[§](#method.restore_workspace_state_with_children)

#### fn [restore_workspace_state_with_children](trait.Component.html#method.restore_workspace_state_with_children)( &mut self, state: &[ViewState](struct.ViewState.html "struct pert_ui::ViewState"), children: &\[[ChildInfo](struct.ChildInfo.html "struct pert_ui::ChildInfo")\], ) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[Invalidation](enum.Invalidation.html "enum pert_ui::Invalidation"), [RestoreRejection](enum.RestoreRejection.html "enum pert_ui::RestoreRejection")\>

Restores state using the current mounted children, before a pending
update. [Read
more](trait.Component.html#method.restore_workspace_state_with_children)

[Source](../src/pert_ui_runtime/tabs.rs.html#1057)[§](#method.child_redocked)

#### fn [child_redocked](trait.Component.html#method.child_redocked)(&mut self, cx: &mut [EventContext](struct.EventContext.html "struct pert_ui::EventContext"), child: &[WidgetId](struct.WidgetId.html "struct pert_ui::WidgetId"))

A retained child returned from a native window. Containers may select
the child by its stable identity. Default does nothing. Called on the UI
thread. The callback must not fail after moving presentation; update
local state only.

[Source](../src/pert_ui_runtime/tabs.rs.html#1066)[§](#method.hover_changed)

#### fn [hover_changed](trait.Component.html#method.hover_changed)(&mut self, cx: &mut [EventContext](struct.EventContext.html "struct pert_ui::EventContext"), hovered: [bool](https://doc.rust-lang.org/1.99.1/std/primitive.bool.html))

Reports when the frontmost pointer enters/leaves this logical subtree,
including native focus loss. Default does nothing. Called at the
runtime’s delivered event time, independently of pressed-button gestures
and pointer bubbling; use it for local hover timing, not host actions or
I/O. Hiding and removal cancel context through `cancel_input` instead.

[Source](../src/pert_ui_runtime/tabs.rs.html#1071)[§](#method.take_children)

#### fn [take_children](trait.Component.html#method.take_children)(&mut self) -\> [Vec](https://doc.rust-lang.org/1.99.1/alloc/vec/struct.Vec.html "struct alloc::vec::Vec")\<[Element](struct.Element.html "struct pert_ui::Element")\>

Transfers initial child descriptions into the common retained tree.
[Read more](trait.Component.html#method.take_children)

[Source](../src/pert_ui_runtime/tabs.rs.html#1074)[§](#method.child_visible)

#### fn [child_visible](trait.Component.html#method.child_visible)(&self, key: &[str](https://doc.rust-lang.org/1.99.1/std/primitive.str.html)) -\> [bool](https://doc.rust-lang.org/1.99.1/std/primitive.bool.html)

Controls presentation of an immediate child by its stable sibling key.
Hidden descendants retain state and suspend work. Defaults to true. Read
bindings during update, then return retained state here; changes require
layout invalidation. This combines with explicit runtime/ancestor
visibility.

[Source](../src/pert_ui_runtime/tabs.rs.html#1077-1081)[§](#method.reveal_child)

#### fn [reveal_child](trait.Component.html#method.reveal_child)( &mut self, cx: &mut [EventContext](struct.EventContext.html "struct pert_ui::EventContext"), key: &[str](https://doc.rust-lang.org/1.99.1/std/primitive.str.html), ) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[ChildReveal](enum.ChildReveal.html "enum pert_ui::ChildReveal"), [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\>

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

[Source](../src/pert_ui_runtime/tabs.rs.html#1089)[§](#method.deliver_notifications)

#### fn [deliver_notifications](trait.Component.html#method.deliver_notifications)( &mut self, cx: &mut [EventContext](struct.EventContext.html "struct pert_ui::EventContext"), ) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[()](https://doc.rust-lang.org/1.99.1/std/primitive.unit.html), [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\>

Delivers the change notifications this component queued, after it asked
for this call through a
[`Notifier`](struct.Notifier.html "struct pert_ui::Notifier"). [Read
more](trait.Component.html#method.deliver_notifications)

[Source](../src/pert_ui_runtime/tabs.rs.html#1105)[§](#method.focusable)

#### fn [focusable](trait.Component.html#method.focusable)(&self) -\> [bool](https://doc.rust-lang.org/1.99.1/std/primitive.bool.html)

Whether automatic keyboard traversal and pointer presses may focus this
component. Default is false; explicit low-level `Runtime::focus` remains
available. [Read more](trait.Component.html#method.focusable)

[Source](../src/pert_ui_runtime/tabs.rs.html#1108)[§](#method.measure_limits)

#### fn [measure_limits](trait.Component.html#method.measure_limits)( &self, cx: &[MeasureContext](struct.MeasureContext.html "struct pert_ui::MeasureContext")\<'\_\>, ) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[SizeLimits](struct.SizeLimits.html "struct pert_ui::SizeLimits"), [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\>

Derives hard limits from visible children during layout. Default uses
`size_limits`; composite controls override this to propagate descendant
minimums before a parent allocates/clips them. Use the public child
limits, retained state and validated geometry only. [Read
more](trait.Component.html#method.measure_limits)

[Source](../src/pert_ui_runtime/tabs.rs.html#1116)[§](#method.update)

#### fn [update](trait.Component.html#tymethod.update)(&mut self, cx: &mut [UpdateContext](struct.UpdateContext.html "struct pert_ui::UpdateContext")) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[()](https://doc.rust-lang.org/1.99.1/std/primitive.unit.html), [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\>

Refreshes retained data according to the context’s invalidation. [Read
more](trait.Component.html#tymethod.update)

[Source](../src/pert_ui_runtime/tabs.rs.html#1213-1217)[§](#method.measure)

#### fn [measure](trait.Component.html#method.measure)( &self, cx: &[MeasureContext](struct.MeasureContext.html "struct pert_ui::MeasureContext")\<'\_\>, available: [Constraints](struct.Constraints.html "struct pert_ui::Constraints"), ) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[Size](struct.Size.html "struct pert_ui::Size"), [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\>

Measures content in logical pixels under the available space. [Read
more](trait.Component.html#method.measure)

[Source](../src/pert_ui_runtime/tabs.rs.html#1246)[§](#method.arrange)

#### fn [arrange](trait.Component.html#method.arrange)( &self, cx: &mut [ArrangeContext](struct.ArrangeContext.html "struct pert_ui::ArrangeContext")\<'\_\>, size: [Size](struct.Size.html "struct pert_ui::Size"), ) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[()](https://doc.rust-lang.org/1.99.1/std/primitive.unit.html), [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\>

Places every visible child once, relative to this component’s origin.
[Read more](trait.Component.html#method.arrange)

[Source](../src/pert_ui_runtime/tabs.rs.html#1307-1311)[§](#method.pointer_input)

#### fn [pointer_input](trait.Component.html#method.pointer_input)( &mut self, cx: &mut [EventContext](struct.EventContext.html "struct pert_ui::EventContext"), event: [PointerEvent](struct.PointerEvent.html "struct pert_ui::PointerEvent"), ) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[InputOutcome](enum.InputOutcome.html "enum pert_ui::InputOutcome"), [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\>

Handles a pointer event, bubbling ignored events through logical
ancestors. Captured events keep their owner even outside its clipped
bounds. [Read more](trait.Component.html#method.pointer_input)

[Source](../src/pert_ui_runtime/tabs.rs.html#1413-1418)[§](#method.keyboard_event)

#### fn [keyboard_event](trait.Component.html#method.keyboard_event)( &mut self, cx: &mut [EventContext](struct.EventContext.html "struct pert_ui::EventContext"), event: [KeyEvent](struct.KeyEvent.html "struct pert_ui::KeyEvent"), keyboard: &[KeyboardState](struct.KeyboardState.html "struct pert_ui::KeyboardState"), ) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[InputOutcome](enum.InputOutcome.html "enum pert_ui::InputOutcome"), [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\>

Handles keyboard input with explicit propagation. [Read
more](trait.Component.html#method.keyboard_event)

[Source](../src/pert_ui_runtime/tabs.rs.html#1490)[§](#method.input_reset)

#### fn [input_reset](trait.Component.html#method.input_reset)(&mut self)

Releases component-owned held-key actions when its input context is
lost. [Read more](trait.Component.html#method.input_reset)

[Source](../src/pert_ui_runtime/tabs.rs.html#1500-1504)[§](#method.drag_event)

#### fn [drag_event](trait.Component.html#method.drag_event)( &mut self, cx: &mut [EventContext](struct.EventContext.html "struct pert_ui::EventContext"), event: &[DragEvent](struct.DragEvent.html "struct pert_ui::DragEvent")\<'\_\>, ) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[DropResponse](enum.DropResponse.html "enum pert_ui::DropResponse"), [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\>

Offers a drag session to this component, or tells it how its part ended.
[Read more](trait.Component.html#method.drag_event)

[Source](../src/pert_ui_runtime/tabs.rs.html#1572)[§](#method.timer_elapsed)

#### fn [timer_elapsed](trait.Component.html#method.timer_elapsed)(&mut self, cx: &mut [EventContext](struct.EventContext.html "struct pert_ui::EventContext")) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[()](https://doc.rust-lang.org/1.99.1/std/primitive.unit.html), [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\>

Runs a timed action whose deadline, set with
[`EventContext::set_timer`](struct.EventContext.html#method.set_timer "method pert_ui::EventContext::set_timer"),
has elapsed. [Read more](trait.Component.html#method.timer_elapsed)

[Source](../src/pert_ui_runtime/tabs.rs.html#1588-1592)[§](#method.drag_finished)

#### fn [drag_finished](trait.Component.html#method.drag_finished)( &mut self, cx: &mut [EventContext](struct.EventContext.html "struct pert_ui::EventContext"), outcome: [DragOutcome](enum.DragOutcome.html "enum pert_ui::DragOutcome"), ) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[()](https://doc.rust-lang.org/1.99.1/std/primitive.unit.html), [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\>

Tells the source how an accepted drag request or adopted session ended.
Called exactly once while the source is mounted, including cancellation
before adoption. An adopted session notifies its target/observers first,
then releases capture and unmounts the preview before notifying the
source. [Read more](trait.Component.html#method.drag_finished)

[Source](../src/pert_ui_runtime/tabs.rs.html#1622)[§](#method.cursor)

#### fn [cursor](trait.Component.html#method.cursor)(&self, point: [Point](struct.Point.html "struct pert_ui::Point")) -\> [Option](https://doc.rust-lang.org/1.99.1/core/option/enum.Option.html "enum core::option::Option")\<[CursorIcon](enum.CursorIcon.html "enum pert_ui::CursorIcon")\>

Returns a cursor for a point in local logical coordinates, or `None` to
defer to ancestors. Called read-only on the UI thread after layout; no
writes or I/O are allowed. Captured components may return a resize
cursor outside their bounds until the gesture ends. Default defers.

[Source](../src/pert_ui_runtime/tabs.rs.html#1629)[§](#method.paint)

#### fn [paint](trait.Component.html#method.paint)(&self, painter: &mut [Painter](struct.Painter.html "struct pert_ui::Painter")) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[()](https://doc.rust-lang.org/1.99.1/std/primitive.unit.html), [DrawError](enum.DrawError.html "enum pert_ui::DrawError")\>

Records backend-independent drawing from retained data after an update.
[Read more](trait.Component.html#method.paint)

[Source](../src/pert_ui_runtime/tabs.rs.html#1787)[§](#method.inspect)

#### fn [inspect](trait.Component.html#method.inspect)(&self) -\> [ComponentDetails](struct.ComponentDetails.html "struct pert_ui::ComponentDetails")

Returns explicitly exposed state and actual measured box/style
information. [Read more](trait.Component.html#method.inspect)

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

[Source](../src/pert_ui_runtime/runtime.rs.html#1452)[§](#method.tab_stop)

#### fn [tab_stop](trait.Component.html#method.tab_stop)(&self) -\> [bool](https://doc.rust-lang.org/1.99.1/std/primitive.bool.html)

Whether keyboard traversal
([`Runtime::focus_next`](struct.Runtime.html#method.focus_next "method pert_ui::Runtime::focus_next"),
Tab and Shift+Tab) stops at this component. [Read
more](trait.Component.html#method.tab_stop)

[Source](../src/pert_ui_runtime/runtime.rs.html#1472)[§](#method.descendant_tab_stops)

#### fn [descendant_tab_stops](trait.Component.html#method.descendant_tab_stops)(&self) -\> [bool](https://doc.rust-lang.org/1.99.1/std/primitive.bool.html)

Whether this component’s descendants take part in keyboard traversal
([`Runtime::focus_next`](struct.Runtime.html#method.focus_next "method pert_ui::Runtime::focus_next"),
Tab and Shift+Tab). [Read
more](trait.Component.html#method.descendant_tab_stops)

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

[Source](../src/pert_ui_runtime/runtime.rs.html#1687)[§](#method.can_reveal)

#### fn [can_reveal](trait.Component.html#method.can_reveal)(&self) -\> [bool](https://doc.rust-lang.org/1.99.1/std/primitive.bool.html)

Whether this component can reveal clipped descendants by scrolling.
Default is false. Automatic focus traversal includes clipped descendants
only where an ancestor supports this public contract.

[Source](../src/pert_ui_runtime/runtime.rs.html#1699)[§](#method.reveal)

#### fn [reveal](trait.Component.html#method.reveal)( &mut self, \_cx: &mut [EventContext](struct.EventContext.html "struct pert_ui::EventContext"), \_rect: [Rect](struct.Rect.html "struct pert_ui::Rect"), ) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[bool](https://doc.rust-lang.org/1.99.1/std/primitive.bool.html), [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\>

Reveals a descendant’s current window-coordinate rectangle, returning
whether its placement changed. Called nearest ancestor first during
focus transfer. Implementations clamp scrolling to content bounds and
invalidate layout when returning true. Runtime lays out before asking
the next ancestor. [Read more](trait.Component.html#method.reveal)

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

[Source](../src/pert_ui_runtime/runtime.rs.html#1900)[§](#method.retain_initial_children)

#### fn [retain_initial_children](trait.Component.html#method.retain_initial_children)(&self) -\> [bool](https://doc.rust-lang.org/1.99.1/std/primitive.bool.html)

Keeps constructor-generated children when this component is described
again. [Read more](trait.Component.html#method.retain_initial_children)

[Source](../src/pert_ui_runtime/runtime.rs.html#2224)[§](#method.content_rows)

#### fn [content_rows](trait.Component.html#method.content_rows)(&self) -\> [Option](https://doc.rust-lang.org/1.99.1/core/option/enum.Option.html "enum core::option::Option")\<[Rc](https://doc.rust-lang.org/1.99.1/alloc/rc/struct.Rc.html "struct alloc::rc::Rc")\<dyn [ContentRows](trait.ContentRows.html "trait pert_ui::ContentRows")\>\>

Searchable rows this collection offers to [content
search](struct.Runtime.html#method.content_search "method pert_ui::Runtime::content_search")
beyond its mounted children. [Read
more](trait.Component.html#method.content_rows)

[Source](../src/pert_ui_runtime/runtime.rs.html#2258)[§](#method.try_content_rows)

#### fn [try_content_rows](trait.Component.html#method.try_content_rows)(&self) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[Option](https://doc.rust-lang.org/1.99.1/core/option/enum.Option.html "enum core::option::Option")\<[Rc](https://doc.rust-lang.org/1.99.1/alloc/rc/struct.Rc.html "struct alloc::rc::Rc")\<dyn [ContentRows](trait.ContentRows.html "trait pert_ui::ContentRows")\>\>, [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\>

Returns current searchable collection rows with read failures preserved.
[Read more](trait.Component.html#method.try_content_rows)

[Source](../src/pert_ui_runtime/runtime.rs.html#2285)[§](#method.paint_foreground)

#### fn [paint_foreground](trait.Component.html#method.paint_foreground)(&self, \_painter: &mut [Painter](struct.Painter.html "struct pert_ui::Painter")) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[()](https://doc.rust-lang.org/1.99.1/std/primitive.unit.html), [DrawError](enum.DrawError.html "enum pert_ui::DrawError")\>

Records decorations above this component’s children, inside the same
clip. Uses the same retained cache and restrictions as `paint`. Default
draws nothing. Useful for scrollbars and focus outlines. Use
`hit_test_foreground` to make foreground decorations interactive above
children. [Read more](trait.Component.html#method.paint_foreground)

[Source](../src/pert_ui_runtime/runtime.rs.html#2323)[§](#method.hit_test)

#### fn [hit_test](trait.Component.html#method.hit_test)(&self, \_point: [Point](struct.Point.html "struct pert_ui::Point")) -\> [bool](https://doc.rust-lang.org/1.99.1/std/primitive.bool.html)

Tests this component’s body at a component-local point in logical
pixels. [Read more](trait.Component.html#method.hit_test)

[Source](../src/pert_ui_runtime/runtime.rs.html#2359)[§](#method.hit_test_foreground)

#### fn [hit_test_foreground](trait.Component.html#method.hit_test_foreground)(&self, \_point: [Point](struct.Point.html "struct pert_ui::Point")) -\> [bool](https://doc.rust-lang.org/1.99.1/std/primitive.bool.html)

Tests an interactive foreground decoration above this component’s
children. [Read more](trait.Component.html#method.hit_test_foreground)

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

[Source](../src/pert_ui_runtime/runtime.rs.html#2416)[§](#method.text_input)

#### fn [text_input](trait.Component.html#method.text_input)( &mut self, \_context: &mut [EventContext](struct.EventContext.html "struct pert_ui::EventContext"), \_text: &[str](https://doc.rust-lang.org/1.99.1/std/primitive.str.html), ) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[()](https://doc.rust-lang.org/1.99.1/std/primitive.unit.html), [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\>

Receives committed UTF-8 text independently of physical keys and
repeats. [Read more](trait.Component.html#method.text_input)

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

[Source](../src/pert_ui_runtime/tabs.rs.html#343)[§](#impl-Debug-for-Tabs)

### impl [Debug](https://doc.rust-lang.org/1.99.1/core/fmt/trait.Debug.html "trait core::fmt::Debug") for [Tabs](struct.Tabs.html "struct pert_ui::Tabs")

Shows only the type. A description’s content, bindings and callbacks are
not formatted; where the description is mounted as an
[`Element`](struct.Element.html "struct pert_ui::Element"), that
element’s `Debug` shows its key.

[Source](../src/pert_ui_runtime/tabs.rs.html#343)[§](#method.fmt)

#### fn [fmt](https://doc.rust-lang.org/1.99.1/core/fmt/trait.Debug.html#tymethod.fmt)(&self, f: &mut [Formatter](https://doc.rust-lang.org/1.99.1/core/fmt/struct.Formatter.html "struct core::fmt::Formatter")\<'\_\>) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[()](https://doc.rust-lang.org/1.99.1/std/primitive.unit.html), [Error](https://doc.rust-lang.org/1.99.1/core/fmt/struct.Error.html "struct core::fmt::Error")\>

Formats the value using the given formatter. [Read
more](https://doc.rust-lang.org/1.99.1/core/fmt/trait.Debug.html#tymethod.fmt)

## Auto Trait Implementations[§](#synthetic-implementations)

[§](#impl-Freeze-for-Tabs)

### impl \![Freeze](https://doc.rust-lang.org/1.99.1/core/marker/trait.Freeze.html "trait core::marker::Freeze") for [Tabs](struct.Tabs.html "struct pert_ui::Tabs")

[§](#impl-RefUnwindSafe-for-Tabs)

### impl \![RefUnwindSafe](https://doc.rust-lang.org/1.99.1/core/panic/unwind_safe/trait.RefUnwindSafe.html "trait core::panic::unwind_safe::RefUnwindSafe") for [Tabs](struct.Tabs.html "struct pert_ui::Tabs")

[§](#impl-Send-for-Tabs)

### impl \![Send](https://doc.rust-lang.org/1.99.1/core/marker/trait.Send.html "trait core::marker::Send") for [Tabs](struct.Tabs.html "struct pert_ui::Tabs")

[§](#impl-Sync-for-Tabs)

### impl \![Sync](https://doc.rust-lang.org/1.99.1/core/marker/trait.Sync.html "trait core::marker::Sync") for [Tabs](struct.Tabs.html "struct pert_ui::Tabs")

[§](#impl-UnwindSafe-for-Tabs)

### impl \![UnwindSafe](https://doc.rust-lang.org/1.99.1/core/panic/unwind_safe/trait.UnwindSafe.html "trait core::panic::unwind_safe::UnwindSafe") for [Tabs](struct.Tabs.html "struct pert_ui::Tabs")

[§](#impl-Unpin-for-Tabs)

### impl [Unpin](https://doc.rust-lang.org/1.99.1/core/marker/trait.Unpin.html "trait core::marker::Unpin") for [Tabs](struct.Tabs.html "struct pert_ui::Tabs")

[§](#impl-UnsafeUnpin-for-Tabs)

### impl [UnsafeUnpin](https://doc.rust-lang.org/1.99.1/core/marker/trait.UnsafeUnpin.html "trait core::marker::UnsafeUnpin") for [Tabs](struct.Tabs.html "struct pert_ui::Tabs")

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
