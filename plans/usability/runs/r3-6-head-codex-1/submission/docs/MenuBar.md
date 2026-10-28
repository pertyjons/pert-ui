[pert_ui](index.html)

# Struct MenuBar Copy item path

[Source](../src/pert_ui_runtime/menu_bar.rs.html#625)

``` rust
pub struct MenuBar { /* private fields */ }
```

Expand description

An application menu bar: a row of titles such as File, Edit and Help,
each opening a menu, with desktop menu-bar pointer and keyboard
behavior.

Each top-level
[`MenuItem::submenu`](struct.MenuItem.html#method.submenu "associated function pert_ui::MenuItem::submenu")
becomes a title that opens its items; any other top-level item becomes a
title that runs its action directly, and top-level separators are
ignored. Rows keep every
[`MenuItem`](struct.MenuItem.html "struct pert_ui::MenuItem") feature:
nested submenus, separators, shortcuts, check and radio marks and
[`MenuItem::command`](struct.MenuItem.html#method.command "associated function pert_ui::MenuItem::command")
rows. Keys follow the [`Menu`](struct.Menu.html "struct pert_ui::Menu")
rules at every level.

Pointer: pressing a title opens its menu; while a menu is open, pointing
at another title switches to its menu, and pressing outside the menu,
including its own title, closes it.

Keyboard: F10 moves focus to the first enabled title. The titles are not
Tab stops otherwise, so the bar does not lengthen ordinary Tab
traversal. On a focused title, unmodified Left and Right move between
titles, Home and End go to the first and last, and unmodified
Enter/Space or Down/Up open its menu. Inside an open menu, unmodified
Left and Right move to the neighboring menu when the row does not open a
submenu. Modified chords of these keys are left to command bindings.
Escape closes one level at a time and, on a title, leaves the bar; F10
again leaves it from anywhere. Choosing an action also leaves the bar.
Leaving returns focus to what was focused when the bar was engaged.

F10 reaches the bar through ordinary key bubbling, like the Shift+F10 of
a
[`ContextMenu`](struct.ContextMenu.html "struct pert_ui::ContextMenu"):
from the bar itself, or from focused content inside it. Give the
application’s content to
[`Self::content`](struct.MenuBar.html#method.content "method pert_ui::MenuBar::content")
so the bar is its ancestor; F10 then works wherever focus is in that
content. With no focused component in the window, the first visible bar
receives F10 directly (see
[`Component::receives_unfocused_keys`](trait.Component.html#method.receives_unfocused_keys "method pert_ui::Component::receives_unfocused_keys")),
so it works before anything has been focused. Menu mnemonics such as
Alt+F are not provided.

A bar captures one
[`CommandContext`](struct.CommandContext.html "struct pert_ui::CommandContext")
when it is engaged by pointer or F10, before anything in it is focused,
so
[`MenuItem::command`](struct.MenuItem.html#method.command "associated function pert_ui::MenuItem::command")
rows act on the content target and focus from before the bar was used.
Every menu is an anchored popover below its title, following the
[`Popover`](struct.Popover.html "struct pert_ui::Popover") placement
rules. The bar spans the width it is given and clips titles that do not
fit. No native OS menu bar is created, including on macOS.

[`Self::leading`](struct.MenuBar.html#method.leading "method pert_ui::MenuBar::leading")
and
[`Self::trailing`](struct.MenuBar.html#method.trailing "method pert_ui::MenuBar::trailing")
add application content to the title row: before the first title, such
as an application mark, and at the row’s right edge, such as navigation
or window buttons. Slot content is ordinary content: its controls are
Tab stops in visual order (leading, then trailing, then
[`Self::content`](struct.MenuBar.html#method.content "method pert_ui::MenuBar::content")),
and F10, Left, Right, Home and End move between titles only, never into
a slot. The row is as tall as the tallest of the titles, the slots and
[`MetricRole::ControlHeight`](enum.MetricRole.html#variant.ControlHeight "variant pert_ui::MetricRole::ControlHeight");
slot content keeps its measured size and is centered vertically. When
the bar is too narrow, the titles keep their place after the leading
slot and the trailing slot moves right of the last title instead of
overlapping it, so whatever does not fit is clipped at the bar’s right
edge.

Reconciling a new description, for example one with an added “Recent
files” menu, takes its titles in their new order. Titles are matched by
item key: a surviving title keeps its identity, keyboard focus and open
menu, a new one mounts and a removed one unmounts, closing its menu. The
captured command context and keyboard menu mode survive as well. Hidden
titles and optional slots are omitted from layout; later titles compact
while slots retain their roles. F10 and title navigation skip hidden
titles without changing their retained identities, and showing them
again restores their ordinary position.

``` rust
use pert_ui::{
    Button, Column, CommandScope, Element, FocusDirection, KeyAction, KeyChord, KeyCode,
    MenuBar, MenuItem, Runtime, Viewport,
};
use std::time::Duration;
let mut ui = Runtime::new();
ui.set_viewport(Viewport::new(640.0, 480.0, 1.0)?)?;
let save = ui.register_command("Save", CommandScope::Application,
    vec![KeyChord::primary('s')], |_, _| Ok(()))?;
let wrap = ui.signal(true);
let bar = MenuBar::new(&ui, vec![
    MenuItem::submenu("file", "File", vec![
        MenuItem::command(&ui, "save", &save, |_, _| Ok(()))?,
        MenuItem::separator(),
        MenuItem::new("quit", "Quit", |_| Ok(())),
    ]),
    MenuItem::submenu("view", "View", vec![
        MenuItem::toggle("wrap", wrap.clone(), "Wrap lines"),
    ]),
])?
.content(Element::new("document", Button::new("Document", |_| Ok(()))));
ui.mount_tree(Element::new("window", Column::new([Element::new("bar", bar)])))?;
ui.flush(Duration::ZERO)?;

ui.focus_next(FocusDirection::Forward)?; // the document: titles are not Tab stops
let document = ui.focused();
assert!(document.is_some());
let press = |ui: &mut Runtime, key| -> Result<(), Box<dyn std::error::Error>> {
    ui.key_input(key, KeyAction::Press)?;
    ui.key_input(key, KeyAction::Release)?;
    ui.flush(Duration::ZERO)?;
    Ok(())
};
press(&mut ui, KeyCode::F10)?;
assert_ne!(ui.focused(), document); // the File title
press(&mut ui, KeyCode::Escape)?;
assert_eq!(ui.focused(), document);
```

## Implementations[§](#implementations)

[Source](../src/pert_ui_runtime/menu_bar.rs.html#651)[§](#impl-MenuBar)

### impl [MenuBar](struct.MenuBar.html "struct pert_ui::MenuBar")

[Source](../src/pert_ui_runtime/menu_bar.rs.html#666)

#### pub fn [new](#method.new)( ui: &impl [StateOwner](trait.StateOwner.html "trait pert_ui::StateOwner"), menus: [Vec](https://doc.rust-lang.org/1.99.1/alloc/vec/struct.Vec.html "struct alloc::vec::Vec")\<[MenuItem](struct.MenuItem.html "struct pert_ui::MenuItem")\>, ) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[MenuBar](struct.MenuBar.html "struct pert_ui::MenuBar"), [ChoiceError](enum.ChoiceError.html "enum pert_ui::ChoiceError")\>

Creates a bar from top-level items, usually one
[`MenuItem::submenu`](struct.MenuItem.html#method.submenu "associated function pert_ui::MenuItem::submenu")
per title. `ui` owns the bar’s internal state and all callbacks run on
its thread.

`ui` is any
[`StateOwner`](trait.StateOwner.html "trait pert_ui::StateOwner"): the
[`Runtime`](struct.Runtime.html "struct pert_ui::Runtime") while the
application builds its tree, or the
[`UpdateContext`](struct.UpdateContext.html "struct pert_ui::UpdateContext")
or
[`EventContext`](struct.EventContext.html "struct pert_ui::EventContext")
of a component that builds this while it describes its children.

##### [§](#errors)Errors

Returns
[`ChoiceError::EmptyValue`](enum.ChoiceError.html#variant.EmptyValue "variant pert_ui::ChoiceError::EmptyValue")
for an empty or whitespace key and
[`ChoiceError::DuplicateValue`](enum.ChoiceError.html#variant.DuplicateValue "variant pert_ui::ChoiceError::DuplicateValue")
for duplicate sibling keys, at every level.

[Source](../src/pert_ui_runtime/menu_bar.rs.html#741)

#### pub fn [content](#method.content)(self, content: [Element](struct.Element.html "struct pert_ui::Element")) -\> [MenuBar](struct.MenuBar.html "struct pert_ui::MenuBar")

Places `content` below the bar, filling the rest of the space it is
given, and makes the bar its ancestor so F10 reaches the bar from
focused content. Calling it again replaces the content.

[Source](../src/pert_ui_runtime/menu_bar.rs.html#772)

#### pub fn [leading](#method.leading)(self, content: [Element](struct.Element.html "struct pert_ui::Element")) -\> [MenuBar](struct.MenuBar.html "struct pert_ui::MenuBar")

Places `content` on the title row before the first title, for example an
application mark. It keeps its measured size and is centered vertically
in the row; its controls are ordinary Tab stops that F10 and the title
arrow keys do not reach. Calling it again replaces the content, and
reconciliation keeps its state by key like any child.

``` rust
use pert_ui::{Button, Element, FocusDirection, MenuBar, MenuItem, Runtime, Viewport};
use std::time::Duration;
let mut ui = Runtime::new();
ui.set_viewport(Viewport::new(640.0, 480.0, 1.0)?)?;
let bar = MenuBar::new(&ui, vec![
    MenuItem::submenu("file", "File", vec![MenuItem::new("quit", "Quit", |_| Ok(()))]),
])?
.leading(Element::new("mark", Button::new("App", |_| Ok(()))))
.trailing(Element::new("back", Button::new("Back", |_| Ok(()))));
let root = ui.mount_tree(Element::new("bar", bar))?;
ui.flush(Duration::ZERO)?;
// Slots are Tab stops in visual order; the titles are not.
ui.focus_next(FocusDirection::Forward)?;
assert_eq!(ui.focused(), ui.find_descendant(&root, "mark")?);
ui.focus_next(FocusDirection::Forward)?;
assert_eq!(ui.focused(), ui.find_descendant(&root, "back")?);
```

[Source](../src/pert_ui_runtime/menu_bar.rs.html#783)

#### pub fn [trailing](#method.trailing)(self, content: [Element](struct.Element.html "struct pert_ui::Element")) -\> [MenuBar](struct.MenuBar.html "struct pert_ui::MenuBar")

Places `content` on the title row at its right edge, for example
navigation or window buttons. It keeps its measured size and is centered
vertically; in a bar too narrow for it, it moves right of the last title
instead of overlapping it and is clipped at the bar’s edge. Its controls
are ordinary Tab stops after the leading slot’s. Calling it again
replaces the content.

## Trait Implementations[§](#trait-implementations)

[Source](../src/pert_ui_runtime/menu_bar.rs.html#844)[§](#impl-Component-for-MenuBar)

### impl [Component](trait.Component.html "trait pert_ui::Component") for [MenuBar](struct.MenuBar.html "struct pert_ui::MenuBar")

[Source](../src/pert_ui_runtime/menu_bar.rs.html#870)[§](#method.reconcile)

#### fn [reconcile](trait.Component.html#method.reconcile)(&mut self, next: [MenuBar](struct.MenuBar.html "struct pert_ui::MenuBar")) -\> [Option](https://doc.rust-lang.org/1.99.1/core/option/enum.Option.html "enum core::option::Option")\<[Invalidation](enum.Invalidation.html "enum pert_ui::Invalidation")\>

Takes the new titles, slots and content. The titles are reconciled
children; this folds the new description’s shared state into the mounted
one, so titles and menus from either description drive one bar.

[Source](../src/pert_ui_runtime/menu_bar.rs.html#845-849)[§](#method.focus_within_changed)

#### fn [focus_within_changed](trait.Component.html#method.focus_within_changed)( &mut self, \_: &mut [EventContext](struct.EventContext.html "struct pert_ui::EventContext"), within: [bool](https://doc.rust-lang.org/1.99.1/std/primitive.bool.html), ) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[()](https://doc.rust-lang.org/1.99.1/std/primitive.unit.html), [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\>

Reports that keyboard focus entered (`within` true) or left (`within`
false) this component’s subtree while the component itself is not the
focused recipient, which gets
[`Self::focus_gained`](trait.Component.html#method.focus_gained "method pert_ui::Component::focus_gained")
and
[`Self::focus_lost`](trait.Component.html#method.focus_lost "method pert_ui::Component::focus_lost")
instead. Default does nothing. [Read
more](trait.Component.html#method.focus_within_changed)

[Source](../src/pert_ui_runtime/menu_bar.rs.html#860)[§](#method.take_children)

#### fn [take_children](trait.Component.html#method.take_children)(&mut self) -\> [Vec](https://doc.rust-lang.org/1.99.1/alloc/vec/struct.Vec.html "struct alloc::vec::Vec")\<[Element](struct.Element.html "struct pert_ui::Element")\>

Transfers initial child descriptions into the common retained tree.
[Read more](trait.Component.html#method.take_children)

[Source](../src/pert_ui_runtime/menu_bar.rs.html#895)[§](#method.update)

#### fn [update](trait.Component.html#tymethod.update)(&mut self, cx: &mut [UpdateContext](struct.UpdateContext.html "struct pert_ui::UpdateContext")) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[()](https://doc.rust-lang.org/1.99.1/std/primitive.unit.html), [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\>

Refreshes retained data according to the context’s invalidation. [Read
more](trait.Component.html#tymethod.update)

[Source](../src/pert_ui_runtime/menu_bar.rs.html#925-929)[§](#method.measure)

#### fn [measure](trait.Component.html#method.measure)( &self, cx: &[MeasureContext](struct.MeasureContext.html "struct pert_ui::MeasureContext")\<'\_\>, available: [Constraints](struct.Constraints.html "struct pert_ui::Constraints"), ) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[Size](struct.Size.html "struct pert_ui::Size"), [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\>

Measures content in logical pixels under the available space. [Read
more](trait.Component.html#method.measure)

[Source](../src/pert_ui_runtime/menu_bar.rs.html#952)[§](#method.arrange)

#### fn [arrange](trait.Component.html#method.arrange)( &self, cx: &mut [ArrangeContext](struct.ArrangeContext.html "struct pert_ui::ArrangeContext")\<'\_\>, size: [Size](struct.Size.html "struct pert_ui::Size"), ) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[()](https://doc.rust-lang.org/1.99.1/std/primitive.unit.html), [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\>

Places every visible child once, relative to this component’s origin.
[Read more](trait.Component.html#method.arrange)

[Source](../src/pert_ui_runtime/menu_bar.rs.html#1004)[§](#method.clips_children)

#### fn [clips_children](trait.Component.html#method.clips_children)(&self) -\> [bool](https://doc.rust-lang.org/1.99.1/std/primitive.bool.html)

Whether descendants clip to this allocation (default true).
Flow/view/group containers return false so reported overflow can reach
an enclosing scroll viewport. Their own drawing/hit box still clips to
their allocation. Scroll viewports and explicit decorated Containers
retain clipping.

[Source](../src/pert_ui_runtime/menu_bar.rs.html#1007)[§](#method.receives_unfocused_keys)

#### fn [receives_unfocused_keys](trait.Component.html#method.receives_unfocused_keys)(&self) -\> [bool](https://doc.rust-lang.org/1.99.1/std/primitive.bool.html)

Whether this component receives keys while nothing in its window has
focus. [Read more](trait.Component.html#method.receives_unfocused_keys)

[Source](../src/pert_ui_runtime/menu_bar.rs.html#1010-1015)[§](#method.keyboard_event)

#### fn [keyboard_event](trait.Component.html#method.keyboard_event)( &mut self, cx: &mut [EventContext](struct.EventContext.html "struct pert_ui::EventContext"), e: [KeyEvent](struct.KeyEvent.html "struct pert_ui::KeyEvent"), keys: &[KeyboardState](struct.KeyboardState.html "struct pert_ui::KeyboardState"), ) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[InputOutcome](enum.InputOutcome.html "enum pert_ui::InputOutcome"), [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\>

Handles keyboard input with explicit propagation. [Read
more](trait.Component.html#method.keyboard_event)

[Source](../src/pert_ui_runtime/menu_bar.rs.html#1063)[§](#method.paint)

#### fn [paint](trait.Component.html#method.paint)(&self, p: &mut [Painter](struct.Painter.html "struct pert_ui::Painter")) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[()](https://doc.rust-lang.org/1.99.1/std/primitive.unit.html), [DrawError](enum.DrawError.html "enum pert_ui::DrawError")\>

Records backend-independent drawing from retained data after an update.
[Read more](trait.Component.html#method.paint)

[Source](../src/pert_ui_runtime/menu_bar.rs.html#1077)[§](#method.inspect)

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

[Source](../src/pert_ui_runtime/runtime.rs.html#1441)[§](#method.focusable)

#### fn [focusable](trait.Component.html#method.focusable)(&self) -\> [bool](https://doc.rust-lang.org/1.99.1/std/primitive.bool.html)

Whether automatic keyboard traversal and pointer presses may focus this
component. Default is false; explicit low-level `Runtime::focus` remains
available. [Read more](trait.Component.html#method.focusable)

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

[Source](../src/pert_ui_runtime/runtime.rs.html#1555)[§](#method.conceals_input)

#### fn [conceals_input](trait.Component.html#method.conceals_input)(&self) -\> [bool](https://doc.rust-lang.org/1.99.1/std/primitive.bool.html)

Whether this component holds secret input, such as a password, that
diagnostics must not record. [Read
more](trait.Component.html#method.conceals_input)

[Source](../src/pert_ui_runtime/runtime.rs.html#1630-1634)[§](#method.pointer_input)

#### fn [pointer_input](trait.Component.html#method.pointer_input)( &mut self, \_: &mut [EventContext](struct.EventContext.html "struct pert_ui::EventContext"), \_: [PointerEvent](struct.PointerEvent.html "struct pert_ui::PointerEvent"), ) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[InputOutcome](enum.InputOutcome.html "enum pert_ui::InputOutcome"), [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\>

Handles a pointer event, bubbling ignored events through logical
ancestors. Captured events keep their owner even outside its clipped
bounds. [Read more](trait.Component.html#method.pointer_input)

[Source](../src/pert_ui_runtime/runtime.rs.html#1681)[§](#method.child_visible)

#### fn [child_visible](trait.Component.html#method.child_visible)(&self, \_key: &[str](https://doc.rust-lang.org/1.99.1/std/primitive.str.html)) -\> [bool](https://doc.rust-lang.org/1.99.1/std/primitive.bool.html)

Controls presentation of an immediate child by its stable sibling key.
Hidden descendants retain state and suspend work. Defaults to true. Read
bindings during update, then return retained state here; changes require
layout invalidation. This combines with explicit runtime/ancestor
visibility.

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

[Source](../src/pert_ui_runtime/runtime.rs.html#1773)[§](#method.deliver_notifications)

#### fn [deliver_notifications](trait.Component.html#method.deliver_notifications)( &mut self, \_cx: &mut [EventContext](struct.EventContext.html "struct pert_ui::EventContext"), ) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[()](https://doc.rust-lang.org/1.99.1/std/primitive.unit.html), [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\>

Delivers the change notifications this component queued, after it asked
for this call through a
[`Notifier`](struct.Notifier.html "struct pert_ui::Notifier"). [Read
more](trait.Component.html#method.deliver_notifications)

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

[Source](../src/pert_ui_runtime/runtime.rs.html#2463-2467)[§](#method.drag_event)

#### fn [drag_event](trait.Component.html#method.drag_event)( &mut self, \_cx: &mut [EventContext](struct.EventContext.html "struct pert_ui::EventContext"), \_event: &[DragEvent](struct.DragEvent.html "struct pert_ui::DragEvent")\<'\_\>, ) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[DropResponse](enum.DropResponse.html "enum pert_ui::DropResponse"), [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\>

Offers a drag session to this component, or tells it how its part ended.
[Read more](trait.Component.html#method.drag_event)

[Source](../src/pert_ui_runtime/runtime.rs.html#2487-2491)[§](#method.drag_finished)

#### fn [drag_finished](trait.Component.html#method.drag_finished)( &mut self, \_cx: &mut [EventContext](struct.EventContext.html "struct pert_ui::EventContext"), \_outcome: [DragOutcome](enum.DragOutcome.html "enum pert_ui::DragOutcome"), ) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[()](https://doc.rust-lang.org/1.99.1/std/primitive.unit.html), [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\>

Tells the source how an accepted drag request or adopted session ended.
Called exactly once while the source is mounted, including cancellation
before adoption. An adopted session notifies its target/observers first,
then releases capture and unmounts the preview before notifying the
source. [Read more](trait.Component.html#method.drag_finished)

[Source](../src/pert_ui_runtime/runtime.rs.html#2500)[§](#method.hover_changed)

#### fn [hover_changed](trait.Component.html#method.hover_changed)(&mut self, \_cx: &mut [EventContext](struct.EventContext.html "struct pert_ui::EventContext"), \_hovered: [bool](https://doc.rust-lang.org/1.99.1/std/primitive.bool.html))

Reports when the frontmost pointer enters/leaves this logical subtree,
including native focus loss. Default does nothing. Called at the
runtime’s delivered event time, independently of pressed-button gestures
and pointer bubbling; use it for local hover timing, not host actions or
I/O. Hiding and removal cancel context through `cancel_input` instead.

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

[Source](../src/pert_ui_runtime/runtime.rs.html#2628)[§](#method.cancel_input)

#### fn [cancel_input](trait.Component.html#method.cancel_input)(&mut self, cx: &mut [EventContext](struct.EventContext.html "struct pert_ui::EventContext"))

Cancels component-owned input with access to its invalidation context.
[Read more](trait.Component.html#method.cancel_input)

[Source](../src/pert_ui_runtime/runtime.rs.html#2640)[§](#method.input_reset)

#### fn [input_reset](trait.Component.html#method.input_reset)(&mut self)

Releases component-owned held-key actions when its input context is
lost. [Read more](trait.Component.html#method.input_reset)

[Source](../src/pert_ui_runtime/runtime.rs.html#2657)[§](#method.timer_elapsed)

#### fn [timer_elapsed](trait.Component.html#method.timer_elapsed)(&mut self, \_cx: &mut [EventContext](struct.EventContext.html "struct pert_ui::EventContext")) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[()](https://doc.rust-lang.org/1.99.1/std/primitive.unit.html), [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\>

Runs a timed action whose deadline, set with
[`EventContext::set_timer`](struct.EventContext.html#method.set_timer "method pert_ui::EventContext::set_timer"),
has elapsed. [Read more](trait.Component.html#method.timer_elapsed)

[Source](../src/pert_ui_runtime/runtime.rs.html#2671)[§](#method.visibility_changed)

#### fn [visibility_changed](trait.Component.html#method.visibility_changed)(&mut self, \_visible: [bool](https://doc.rust-lang.org/1.99.1/std/primitive.bool.html))

Called on a visibility transition, after hidden editing has been
cancelled.

[Source](../src/pert_ui_runtime/runtime.rs.html#2673)[§](#method.unmounted)

#### fn [unmounted](trait.Component.html#method.unmounted)(&mut self)

Called once after subscriptions, deadlines and edit ownership are
released.

[Source](../src/pert_ui_runtime/menu_bar.rs.html#642)[§](#impl-Debug-for-MenuBar)

### impl [Debug](https://doc.rust-lang.org/1.99.1/core/fmt/trait.Debug.html "trait core::fmt::Debug") for [MenuBar](struct.MenuBar.html "struct pert_ui::MenuBar")

Shows only the type. A description’s content, bindings and callbacks are
not formatted; where the description is mounted as an
[`Element`](struct.Element.html "struct pert_ui::Element"), that
element’s `Debug` shows its key.

[Source](../src/pert_ui_runtime/menu_bar.rs.html#642)[§](#method.fmt)

#### fn [fmt](https://doc.rust-lang.org/1.99.1/core/fmt/trait.Debug.html#tymethod.fmt)(&self, f: &mut [Formatter](https://doc.rust-lang.org/1.99.1/core/fmt/struct.Formatter.html "struct core::fmt::Formatter")\<'\_\>) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[()](https://doc.rust-lang.org/1.99.1/std/primitive.unit.html), [Error](https://doc.rust-lang.org/1.99.1/core/fmt/struct.Error.html "struct core::fmt::Error")\>

Formats the value using the given formatter. [Read
more](https://doc.rust-lang.org/1.99.1/core/fmt/trait.Debug.html#tymethod.fmt)

## Auto Trait Implementations[§](#synthetic-implementations)

[§](#impl-Freeze-for-MenuBar)

### impl \![Freeze](https://doc.rust-lang.org/1.99.1/core/marker/trait.Freeze.html "trait core::marker::Freeze") for [MenuBar](struct.MenuBar.html "struct pert_ui::MenuBar")

[§](#impl-RefUnwindSafe-for-MenuBar)

### impl \![RefUnwindSafe](https://doc.rust-lang.org/1.99.1/core/panic/unwind_safe/trait.RefUnwindSafe.html "trait core::panic::unwind_safe::RefUnwindSafe") for [MenuBar](struct.MenuBar.html "struct pert_ui::MenuBar")

[§](#impl-Send-for-MenuBar)

### impl \![Send](https://doc.rust-lang.org/1.99.1/core/marker/trait.Send.html "trait core::marker::Send") for [MenuBar](struct.MenuBar.html "struct pert_ui::MenuBar")

[§](#impl-Sync-for-MenuBar)

### impl \![Sync](https://doc.rust-lang.org/1.99.1/core/marker/trait.Sync.html "trait core::marker::Sync") for [MenuBar](struct.MenuBar.html "struct pert_ui::MenuBar")

[§](#impl-UnwindSafe-for-MenuBar)

### impl \![UnwindSafe](https://doc.rust-lang.org/1.99.1/core/panic/unwind_safe/trait.UnwindSafe.html "trait core::panic::unwind_safe::UnwindSafe") for [MenuBar](struct.MenuBar.html "struct pert_ui::MenuBar")

[§](#impl-Unpin-for-MenuBar)

### impl [Unpin](https://doc.rust-lang.org/1.99.1/core/marker/trait.Unpin.html "trait core::marker::Unpin") for [MenuBar](struct.MenuBar.html "struct pert_ui::MenuBar")

[§](#impl-UnsafeUnpin-for-MenuBar)

### impl [UnsafeUnpin](https://doc.rust-lang.org/1.99.1/core/marker/trait.UnsafeUnpin.html "trait core::marker::UnsafeUnpin") for [MenuBar](struct.MenuBar.html "struct pert_ui::MenuBar")

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
