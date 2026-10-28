[pert_ui](index.html)

# Struct EventContext Copy item path

[Source](../src/pert_ui_runtime/runtime.rs.html#3120)

``` rust
pub struct EventContext { /* private fields */ }
```

Expand description

Editing context scoped to one visible mounted component’s event.

Created by
[Runtime::event](struct.Runtime.html#method.event "method pert_ui::Runtime::event").
Parameter gestures begun here are cancelled on focus loss, hiding,
unmount or runtime drop, even if a gesture handle survives.

## Implementations[§](#implementations)

[Source](../src/pert_ui_runtime/runtime/commands.rs.html#1780)[§](#impl-EventContext)

### impl [EventContext](struct.EventContext.html "struct pert_ui::EventContext")

[Source](../src/pert_ui_runtime/runtime/commands.rs.html#1784)

#### pub fn [command_context](#method.command_context)(&self, source: [CommandSource](enum.CommandSource.html "enum pert_ui::CommandSource")) -\> [CommandContext](struct.CommandContext.html "struct pert_ui::CommandContext")

Captures the active content target independently of toolbar/menu focus.
Retain this context when opening a menu. A removed/moved target will
fail validation at invocation; a stale optional focused identity is
discarded.

[Source](../src/pert_ui_runtime/runtime/commands.rs.html#1820-1824)

#### pub fn [invoke_command](#method.invoke_command)( &self, id: &[CommandId](struct.CommandId.html "struct pert_ui::CommandId"), context: [CommandContext](struct.CommandContext.html "struct pert_ui::CommandContext"), ) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[CommandReceipt](struct.CommandReceipt.html "struct pert_ui::CommandReceipt"), [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\>

Queues the exact shared command from a button or captured menu target.
The next flush revalidates target, scope, enabled state and modality
before calling host code once, outside this component borrow, through
[`Runtime::invoke_command`](struct.Runtime.html#method.invoke_command "method pert_ui::Runtime::invoke_command")
with its side effects. Observe the receipt: action failures are retained
there, never silently lost or replayed. Associated keyboard text must
already be suppressed by routing.

##### [§](#errors)Errors

Returns `CommandQueueFull` when
[`Runtime::MAX_QUEUED_COMMANDS`](struct.Runtime.html#associatedconstant.MAX_QUEUED_COMMANDS "associated constant pert_ui::Runtime::MAX_QUEUED_COMMANDS")
invocations are waiting, or the runtime phase/shutdown error, without
queueing the invocation.

[Source](../src/pert_ui_runtime/runtime/drag.rs.html#66)[§](#impl-EventContext-1)

### impl [EventContext](struct.EventContext.html "struct pert_ui::EventContext")

[Source](../src/pert_ui_runtime/runtime/drag.rs.html#136-141)

#### pub fn [start_drag](#method.start_drag)( &self, payload: [DragPayload](struct.DragPayload.html "struct pert_ui::DragPayload"), permitted: impl [Into](https://doc.rust-lang.org/1.99.1/core/convert/trait.Into.html "trait core::convert::Into")\<[DropOperations](struct.DropOperations.html "struct pert_ui::DropOperations")\>, preview: [DragPreview](struct.DragPreview.html "struct pert_ui::DragPreview"), ) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[()](https://doc.rust-lang.org/1.99.1/std/primitive.unit.html), [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\>

Starts a drag session with this component as its source.

Call it from
[`Component::pointer_input`](trait.Component.html#method.pointer_input "method pert_ui::Component::pointer_input")
once an armed press has moved
[`DRAG_THRESHOLD`](constant.DRAG_THRESHOLD.html "constant pert_ui::DRAG_THRESHOLD")
window pixels, so a press without movement stays a click. The source
takes the pointer capture, as
[`Self::capture_pointer`](struct.EventContext.html#method.capture_pointer "method pert_ui::EventContext::capture_pointer")
would, and keeps it for the whole session. The runtime adopts the
session when this callback returns: it mounts the preview, suspends
ordinary hover, and from then on offers the drag to whatever is under
the pointer through
[`Component::drag_event`](trait.Component.html#method.drag_event "method pert_ui::Component::drag_event").

Every session ends with exactly one
[`Component::drag_finished`](trait.Component.html#method.drag_finished "method pert_ui::Component::drag_finished")
to a source that is still mounted: a drop, a refusal, a release that
nothing claimed, or a cancellation. Escape, native focus loss,
[`HostIntegration::cancel_input`](host/trait.HostIntegration.html#tymethod.cancel_input "method pert_ui::host::HostIntegration::cancel_input"),
a modal surface that blocks the source, an error from a participant, and
the source being hidden, removed or input-cancelled, and runtime
shutdown all cancel. A session is confined to the window it started in.

``` rust
use pert_ui::{component::*, prelude::*};
struct Chip { key: String, armed: Option<Point> }
struct ChipKey(String);
impl Component for Chip {
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> { Ok(()) }
    fn pointer_input(&mut self, cx: &mut EventContext, event: PointerEvent)
        -> Result<InputOutcome, RuntimeError> {
        match event.kind() {
            PointerKind::Press(PointerButton::Primary) => {
                self.armed = Some(event.position());
                cx.capture_pointer()?;
            }
            PointerKind::Move => {
                if let Some(start) = self.armed
                    && (event.position().x() - start.x())
                        .hypot(event.position().y() - start.y()) >= DRAG_THRESHOLD
                {
                    self.armed = None;
                    cx.start_drag(
                        DragPayload::new(ChipKey(self.key.clone())),
                        DropOperations::MOVE,
                        DragPreview::element(Element::new("chip", Label::new(self.key.as_str()))),
                    )?;
                }
            }
            PointerKind::Release(_) => self.armed = None,
            _ => return Ok(InputOutcome::Ignored),
        }
        Ok(InputOutcome::Handled)
    }
    fn drag_finished(&mut self, _: &mut EventContext, outcome: DragOutcome)
        -> Result<(), RuntimeError> {
        // Restore presentation only. A move was one edit on the target's side.
        let _ = outcome;
        Ok(())
    }
}
```

##### [§](#errors-1)Errors

Returns a runtime phase error (see
[`Runtime`](struct.Runtime.html "struct pert_ui::Runtime")), and
[`RuntimeError::Closed`](enum.RuntimeError.html#variant.Closed "variant pert_ui::RuntimeError::Closed")
also during shutdown terminal callbacks, which cannot admit a
replacement drag;
[`RuntimeError::DragUnavailable`](enum.RuntimeError.html#variant.DragUnavailable "variant pert_ui::RuntimeError::DragUnavailable")
while another session is active or requested, or for an empty set of
operations; and
[`RuntimeError::CaptureUnavailable`](enum.RuntimeError.html#variant.CaptureUnavailable "variant pert_ui::RuntimeError::CaptureUnavailable")
without a held button or while another component owns the capture.

[Source](../src/pert_ui_runtime/runtime/drag.rs.html#166)

#### pub fn [drag_active](#method.drag_active)(&self) -\> [bool](https://doc.rust-lang.org/1.99.1/std/primitive.bool.html)

Whether a drag session is active or requested in this runtime, for a
component that presents differently while something is being dragged.

[Source](../src/pert_ui_runtime/runtime/native_windows.rs.html#1218)[§](#impl-EventContext-2)

### impl [EventContext](struct.EventContext.html "struct pert_ui::EventContext")

[Source](../src/pert_ui_runtime/runtime/native_windows.rs.html#1220)

#### pub fn [native_window](#method.native_window)(&self) -\> [NativeWindowId](struct.NativeWindowId.html "struct pert_ui::NativeWindowId")

Window receiving this component event, independent of lifetime
ownership.

[Source](../src/pert_ui_runtime/runtime/native_windows.rs.html#1235-1239)

#### pub fn [detach_view](#method.detach_view)( &self, view: &[WidgetId](struct.WidgetId.html "struct pert_ui::WidgetId"), options: [NativeWindowOptions](struct.NativeWindowOptions.html "struct pert_ui::NativeWindowOptions"), ) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[PresentationReceipt](struct.PresentationReceipt.html "struct pert_ui::PresentationReceipt"), [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\>

Queues detachment of a retained view. Its lifetime owner is unchanged.
Observe the receipt for native failure; no GPU work runs inside the
callback.

##### [§](#errors-2)Errors

Returns `ForeignRuntime` or `Unmounted` for a foreign or unmounted view,
`NotDetachable` when its description forbids its own window,
[`RuntimeError::Closed`](enum.RuntimeError.html#variant.Closed "variant pert_ui::RuntimeError::Closed")
after shutdown, and `WriteDuringUpdate`, `WriteDuringLayout`,
`WriteDuringPaint`, `WriteDuringInspection` or `WriteDuringDelivery`
when called from inside that phase; and
[`RuntimeError::WindowRequestFull`](enum.RuntimeError.html#variant.WindowRequestFull "variant pert_ui::RuntimeError::WindowRequestFull")
when
[`Runtime::MAX_WINDOW_REQUESTS`](struct.Runtime.html#associatedconstant.MAX_WINDOW_REQUESTS "associated constant pert_ui::Runtime::MAX_WINDOW_REQUESTS")
requests are already queued. Hidden views and modal subtrees are refused
later, through the receipt.

[Source](../src/pert_ui_runtime/runtime/native_windows.rs.html#1264)

#### pub fn [close_window](#method.close_window)(&self) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[PresentationReceipt](struct.PresentationReceipt.html "struct pert_ui::PresentationReceipt"), [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\>

Requests host close/save admission for this native window. The main
window requests application exit; detached windows apply their
configured policy.

##### [§](#errors-3)Errors

Returns
[`RuntimeError::Closed`](enum.RuntimeError.html#variant.Closed "variant pert_ui::RuntimeError::Closed")
after shutdown, and `WriteDuringUpdate`, `WriteDuringLayout`,
`WriteDuringPaint`, `WriteDuringInspection` or `WriteDuringDelivery`
when called from inside that phase; and
[`RuntimeError::WindowRequestFull`](enum.RuntimeError.html#variant.WindowRequestFull "variant pert_ui::RuntimeError::WindowRequestFull")
when
[`Runtime::MAX_WINDOW_REQUESTS`](struct.Runtime.html#associatedconstant.MAX_WINDOW_REQUESTS "associated constant pert_ui::Runtime::MAX_WINDOW_REQUESTS")
requests are already queued.

[Source](../src/pert_ui_runtime/runtime/native_windows.rs.html#1279)

#### pub fn [redock_window](#method.redock_window)(&self) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[PresentationReceipt](struct.PresentationReceipt.html "struct pert_ui::PresentationReceipt"), [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\>

Queues return of this event’s detached window to its original owner.

##### [§](#errors-4)Errors

Returns
[`RuntimeError::Closed`](enum.RuntimeError.html#variant.Closed "variant pert_ui::RuntimeError::Closed")
after shutdown, and `WriteDuringUpdate`, `WriteDuringLayout`,
`WriteDuringPaint`, `WriteDuringInspection` or `WriteDuringDelivery`
when called from inside that phase; and
[`RuntimeError::WindowRequestFull`](enum.RuntimeError.html#variant.WindowRequestFull "variant pert_ui::RuntimeError::WindowRequestFull")
when
[`Runtime::MAX_WINDOW_REQUESTS`](struct.Runtime.html#associatedconstant.MAX_WINDOW_REQUESTS "associated constant pert_ui::Runtime::MAX_WINDOW_REQUESTS")
requests are already queued. A window that is not detached is refused
later, through the receipt.

[Source](../src/pert_ui_runtime/runtime/native_windows.rs.html#1300-1304)

#### pub fn [set_window_title](#method.set_window_title)( &self, window: &[NativeWindowId](struct.NativeWindowId.html "struct pert_ui::NativeWindowId"), title: impl [Into](https://doc.rust-lang.org/1.99.1/core/convert/trait.Into.html "trait core::convert::Into")\<[String](https://doc.rust-lang.org/1.99.1/alloc/string/struct.String.html "struct alloc::string::String")\>, ) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[PresentationReceipt](struct.PresentationReceipt.html "struct pert_ui::PresentationReceipt"), [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\>

Queues `title` as the native title of the open window `window`, such as
a detached view whose title changed. A request still queued for the same
window takes the new title instead of queuing another, and the returned
receipt observes that request. Tabs and tool windows call this for their
detached views; an application can call it for its own.

##### [§](#errors-5)Errors

Returns
[`RuntimeError::EmptyWindowTitle`](enum.RuntimeError.html#variant.EmptyWindowTitle "variant pert_ui::RuntimeError::EmptyWindowTitle")
for a blank title, `Unmounted` for a window identity that is not a
mounted presentation root of this runtime,
[`RuntimeError::Closed`](enum.RuntimeError.html#variant.Closed "variant pert_ui::RuntimeError::Closed")
after shutdown, `WriteDuringUpdate`, `WriteDuringLayout`,
`WriteDuringPaint`, `WriteDuringInspection` or `WriteDuringDelivery`
when called from inside that phase, and
[`RuntimeError::WindowRequestFull`](enum.RuntimeError.html#variant.WindowRequestFull "variant pert_ui::RuntimeError::WindowRequestFull")
when
[`Runtime::MAX_WINDOW_REQUESTS`](struct.Runtime.html#associatedconstant.MAX_WINDOW_REQUESTS "associated constant pert_ui::Runtime::MAX_WINDOW_REQUESTS")
requests are already queued and none of them retitles this window.

[Source](../src/pert_ui_runtime/runtime/native_windows.rs.html#1324-1327)

#### pub fn [activate_window](#method.activate_window)( &self, window: &[NativeWindowId](struct.NativeWindowId.html "struct pert_ui::NativeWindowId"), ) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[PresentationReceipt](struct.PresentationReceipt.html "struct pert_ui::PresentationReceipt"), [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\>

Requests native access without assuming OS focus or restoration
succeeds.

##### [§](#errors-6)Errors

Returns `Unmounted` for a window identity that is not a mounted
presentation root of this runtime,
[`RuntimeError::Closed`](enum.RuntimeError.html#variant.Closed "variant pert_ui::RuntimeError::Closed")
after shutdown, and `WriteDuringUpdate`, `WriteDuringLayout`,
`WriteDuringPaint`, `WriteDuringInspection` or `WriteDuringDelivery`
when called from inside that phase; and
[`RuntimeError::WindowRequestFull`](enum.RuntimeError.html#variant.WindowRequestFull "variant pert_ui::RuntimeError::WindowRequestFull")
when
[`Runtime::MAX_WINDOW_REQUESTS`](struct.Runtime.html#associatedconstant.MAX_WINDOW_REQUESTS "associated constant pert_ui::Runtime::MAX_WINDOW_REQUESTS")
requests are already queued.

[Source](../src/pert_ui_runtime/runtime/reveal.rs.html#207)[§](#impl-EventContext-3)

### impl [EventContext](struct.EventContext.html "struct pert_ui::EventContext")

[Source](../src/pert_ui_runtime/runtime/reveal.rs.html#266)

#### pub fn [request_reveal](#method.request_reveal)( &self, target: &[WidgetId](struct.WidgetId.html "struct pert_ui::WidgetId"), ) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[RevealReceipt](struct.RevealReceipt.html "struct pert_ui::RevealReceipt"), [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\>

Queues a reveal of `target` without moving focus for the end of the next
flush, and returns its receipt.
[`Self::request_reveal_and_focus`](struct.EventContext.html#method.request_reveal_and_focus "method pert_ui::EventContext::request_reveal_and_focus")
also focuses the target.

The reveal runs as
[`Runtime::reveal`](struct.Runtime.html#method.reveal "method pert_ui::Runtime::reveal")
after that flush’s updates and layout settle, so a surface this event
closes, such as a modal palette that jumps elsewhere, no longer blocks
the target. It runs even when the requesting component has been removed
meanwhile. A native window activation is requested when the target lives
in another window than this component. Requests run in the order they
were made; one made while reveals are running waits for the next flush,
which
[`Runtime::next_deadline`](struct.Runtime.html#method.next_deadline "method pert_ui::Runtime::next_deadline")
then reports as due.

The receipt always reaches a terminal
[`RevealStatus`](enum.RevealStatus.html "enum pert_ui::RevealStatus"):
the outcome, an operational error such as `Unmounted` for a target
removed before the reveal ran, or `Cancelled` at shutdown.

``` rust
use pert_ui::prelude::*;
use pert_ui::testing::*;
use pert_ui::{Tab, Tabs};
use std::time::Duration;

let mut ui = Runtime::new();
ui.set_viewport(Viewport::new(400.0, 300.0, 1.0)?)?;
let tabs = ui.mount_tree(Element::new(
    "tabs",
    Tabs::new([
        Tab::new("home", "Home", Element::new("welcome", Label::new("Welcome"))),
        Tab::new("settings", "Settings", Element::new("name", TextField::new(ui.signal(String::new())))),
    ]),
))?;
ui.flush(Duration::ZERO)?;
let name = ui.find_descendant(&tabs, "name")?.ok_or("not mounted")?;

// An internal link: activating the button opens the tab and focuses the field.
let target = name.clone();
let link = ui.mount_tree(Element::new(
    "edit-name",
    Button::new("Edit name", move |cx| {
        cx.request_reveal_and_focus(&target).map(|_| ())
    }),
))?;
ui.flush(Duration::ZERO)?;
ui.focus(&link)?;
ui.key_input(KeyCode::Enter, KeyAction::Press)?;
ui.key_input(KeyCode::Enter, KeyAction::Release)?;
ui.flush(Duration::ZERO)?;
assert_eq!(ui.focused(), Some(name));
```

##### [§](#errors-7)Errors

Returns `ForeignRuntime` for a target of another runtime,
[`RuntimeError::RevealQueueFull`](enum.RuntimeError.html#variant.RevealQueueFull "variant pert_ui::RuntimeError::RevealQueueFull")
when
[`Runtime::MAX_REVEAL_REQUESTS`](struct.Runtime.html#associatedconstant.MAX_REVEAL_REQUESTS "associated constant pert_ui::Runtime::MAX_REVEAL_REQUESTS")
requests are waiting, or the runtime phase/shutdown error, without
queueing the request.

[Source](../src/pert_ui_runtime/runtime/reveal.rs.html#279-282)

#### pub fn [request_reveal_and_focus](#method.request_reveal_and_focus)( &self, target: &[WidgetId](struct.WidgetId.html "struct pert_ui::WidgetId"), ) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[RevealReceipt](struct.RevealReceipt.html "struct pert_ui::RevealReceipt"), [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\>

Queues a reveal of `target` that also focuses it, for the end of the
next flush, and returns its receipt.

The reveal runs as
[`Runtime::reveal_and_focus`](struct.Runtime.html#method.reveal_and_focus "method pert_ui::Runtime::reveal_and_focus");
queueing, ordering, the receipt and window activation are those of
[`Self::request_reveal`](struct.EventContext.html#method.request_reveal "method pert_ui::EventContext::request_reveal").

##### [§](#errors-8)Errors

As
[`Self::request_reveal`](struct.EventContext.html#method.request_reveal "method pert_ui::EventContext::request_reveal").

[Source](../src/pert_ui_runtime/runtime/settings.rs.html#714)[§](#impl-EventContext-4)

### impl [EventContext](struct.EventContext.html "struct pert_ui::EventContext")

[Source](../src/pert_ui_runtime/runtime/settings.rs.html#717)

#### pub fn [ui_settings](#method.ui_settings)(&self) -\> [UiSettings](struct.UiSettings.html "struct pert_ui::UiSettings")

Shared UI-thread presentation settings for constructing initial parts or
changing preferences from an event. Clones share the same runtime.

[Source](../src/pert_ui_runtime/runtime/settings.rs.html#727)

#### pub fn [strings](#method.strings)(&self) -\> [UiStrings](struct.UiStrings.html "struct pert_ui::UiStrings")

The installed table of library strings, for text a component produces in
an event, such as a refusal message. It subscribes nothing; text a
component shows should be read in update through
[`UpdateContext::strings`](struct.UpdateContext.html#method.strings "method pert_ui::UpdateContext::strings").

[Source](../src/pert_ui_runtime/runtime/tasks.rs.html#205)[§](#impl-EventContext-5)

### impl [EventContext](struct.EventContext.html "struct pert_ui::EventContext")

[Source](../src/pert_ui_runtime/runtime/tasks.rs.html#309-312)

#### pub fn [defer](#method.defer)( &self, action: impl [FnOnce](https://doc.rust-lang.org/1.99.1/core/ops/function/trait.FnOnce.html "trait core::ops::function::FnOnce")(&mut [Runtime](struct.Runtime.html "struct pert_ui::Runtime")) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[()](https://doc.rust-lang.org/1.99.1/std/primitive.unit.html), [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\> + 'static, ) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[TaskReceipt](struct.TaskReceipt.html "struct pert_ui::TaskReceipt"), [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\>

Queues an application operation with mutable runtime access for the next
flush, before component evaluation. This is the deferral for application
callbacks: use it to create document parameters, register commands, open
or save a document or change a tree in response to input. Background
work starts with
[`Self::spawn`](struct.EventContext.html#method.spawn "method pert_ui::EventContext::spawn").

Timing class: [at the next
flush](https://github.com/pertyjons/pert-ui/blob/main/docs/contracts.md#at-the-next-flush).

The operation belongs to the application, not to the widget: only
runtime shutdown cancels it before delivery. It runs once at the next
flush even when the widget is hidden or removed first, so a callback on
a surface that closes, such as a
[`Select`](struct.Select.html "struct pert_ui::Select") change, a menu
row, a [`QuickPick`](struct.QuickPick.html "struct pert_ui::QuickPick")
choice or a dialog’s Save button, queues its work here, and the work
runs after the surface has closed. It also runs when the callback closes
or redocks its detached window. An operation whose targets may be gone
by then reports their error, such as `Unmounted`, from the flush; work a
component does on itself, which should be skipped with it, uses
[`Self::defer_while_shown`](struct.EventContext.html#method.defer_while_shown "method pert_ui::EventContext::defer_while_shown").

At delivery the operation runs with the widget’s window selected while
the widget is mounted, wherever detaching or redocking has moved it;
once the widget is gone, with the window it was queued in (the
[`Self::native_window`](struct.EventContext.html#method.native_window "method pert_ui::EventContext::native_window")
of this call) while that window is open; and otherwise with whichever
window the delivering flush has selected. Routing never cancels the
operation or fails. Work that must address the window the event came
from, wherever the widget is later, captures
[`Self::native_window`](struct.EventContext.html#method.native_window "method pert_ui::EventContext::native_window")
and selects it in the operation. After the operation the earlier
selection is restored while its window is open.

Cancellation reaches only queued operations. Once delivered, an
operation is executing and nothing cancels it: if it calls
[`Runtime::shutdown`](struct.Runtime.html#method.shutdown "method pert_ui::Runtime::shutdown"),
the shutdown cancels the operations still queued and refuses new ones,
and this operation’s receipt records its own result when it returns. If
it panics, its receipt reads `Cancelled` as it unwinds, and the
operations behind it stay queued until the next delivery or shutdown.

The
[`TaskReceipt`](struct.TaskReceipt.html "struct pert_ui::TaskReceipt")
may be dropped, as `cx.defer(...)?;` does: a failure also propagates
from the flush, and the only cancellations, shutdown and a panic as the
operation unwinds, are events the application sees itself. Keep it to
tell a shutdown’s cancellation from success afterwards, such as for a
save. Focus/capture are not transferred to replacement widgets. New
operations queued during delivery wait for the next flush. Errors
propagate from flush and remain in the receipt; partial application work
is not rolled back or replayed. A returned callback error is an explicit
application operation failure, including a nested runtime error it
chooses to propagate; a caught error does not classify subsequent
runtime work. Callbacks must return promptly. This is UI-thread work,
never a callback-safe endpoint.

The callback may call any runtime operation, including those that flush
internally, such as
[`Runtime::reveal_child`](struct.Runtime.html#method.reveal_child "method pert_ui::Runtime::reveal_child"),
[`Runtime::reveal`](struct.Runtime.html#method.reveal "method pert_ui::Runtime::reveal"),
[`Runtime::focus`](struct.Runtime.html#method.focus "method pert_ui::Runtime::focus")
and
[`Runtime::flush`](struct.Runtime.html#method.flush "method pert_ui::Runtime::flush")
itself. Such a nested flush settles the tree but delivers no further
deferred tasks: tasks queued before delivery started still run in order
after this one, and tasks queued from inside it wait for the next flush.

``` rust
use pert_ui::{
    Element, KeyAction, KeyCode, Label, PickChoice, PickItem, PickPending, PickStatus,
    QuickPick, QuickPickController, Runtime, Viewport,
};
use std::time::Duration;
let mut ui = Runtime::new();
ui.set_viewport(Viewport::new(800.0, 600.0, 1.0)?)?;
let picker = QuickPickController::new(&ui);
let opened = ui.signal(None::<String>);
let writer = opened.clone();
ui.mount_tree(Element::new(
    "app",
    QuickPick::new(
        picker.clone(),
        Element::new("doc", Label::new("Document")),
        move |cx, item| {
            // Answering `Pending` closes the picker, which hides the row
            // this callback runs for; the queued work still runs.
            let (result, key) = (writer.clone(), item.key.clone());
            cx.defer(move |_ui| {
                // Work that flushes, such as revealing a page, goes here.
                result.set(Some(key))
            })?;
            let done = writer.read_only();
            Ok(PickChoice::Pending(PickPending::new(done, |value| {
                value.as_ref().map(|_| Ok(()))
            })))
        },
    ),
))?;
picker.open(vec![PickItem::new("knobs", "Knobs")])?;
ui.flush(Duration::ZERO)?;
ui.key_input(KeyCode::Enter, KeyAction::Press)?;
ui.flush(Duration::ZERO)?;
assert_eq!(opened.get()?.as_deref(), Some("knobs"));
assert_eq!(picker.status()?, PickStatus::Closed);
```

##### [§](#errors-9)Errors

Returns `TaskQueueFull` at
[`Runtime::MAX_TASKS`](struct.Runtime.html#associatedconstant.MAX_TASKS "associated constant pert_ui::Runtime::MAX_TASKS")
pending operations, or the runtime phase/shutdown error, including from
cleanup callbacks that run during
[`Runtime::shutdown`](struct.Runtime.html#method.shutdown "method pert_ui::Runtime::shutdown"),
without accepting the operation.

[Source](../src/pert_ui_runtime/runtime/tasks.rs.html#393-396)

#### pub fn [defer_while_shown](#method.defer_while_shown)( &self, action: impl [FnOnce](https://doc.rust-lang.org/1.99.1/core/ops/function/trait.FnOnce.html "trait core::ops::function::FnOnce")(&mut [Runtime](struct.Runtime.html "struct pert_ui::Runtime")) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[()](https://doc.rust-lang.org/1.99.1/std/primitive.unit.html), [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\> + 'static, ) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[ShownTaskReceipt](struct.ShownTaskReceipt.html "struct pert_ui::ShownTaskReceipt"), [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\>

Queues an operation on this component’s behalf, cancelled when the
component is hidden or removed before the next flush. It is for custom
component authors (see
[`Component`](trait.Component.html "trait pert_ui::Component")), for
work a component does on itself, such as describing its own children
again from an input hook, that should end with the component as its
[`set_timer`](struct.EventContext.html#method.set_timer "method pert_ui::EventContext::set_timer")
deadline does. Application callbacks use
[`Self::defer`](struct.EventContext.html#method.defer "method pert_ui::EventContext::defer").

Timing class: [at the next
flush](https://github.com/pertyjons/pert-ui/blob/main/docs/contracts.md#at-the-next-flush).

The
[`ShownTaskReceipt`](struct.ShownTaskReceipt.html "struct pert_ui::ShownTaskReceipt")
reads `Cancelled` without running the operation when, at delivery, the
component is unmounted or not effectively visible: hidden itself or
through an ancestor, on a surface that has closed, or in a native window
that is hidden. Runtime shutdown also cancels it. Otherwise it runs with
the component’s window at delivery selected, wherever detaching or
redocking has moved it. Delivery order, statuses, errors, nested flushes
and capacity match
[`Self::defer`](struct.EventContext.html#method.defer "method pert_ui::EventContext::defer"),
with which it shares one queue.

Nothing but the receipt reports a cancellation by hiding or removal, so
its type is `#[must_use]` and discarding it after `?` warns, unlike
[`Self::defer`](struct.EventContext.html#method.defer "method pert_ui::EventContext::defer")’s
[`TaskReceipt`](struct.TaskReceipt.html "struct pert_ui::TaskReceipt").
Keep it, or bind it as `let _receipt` where skipping the work silently
is the intent.

``` rust
use pert_ui::{
    Component, Element, EventContext, InputOutcome, KeyAction, KeyCode, KeyEvent,
    KeyTransition, KeyboardState, Runtime, RuntimeError, ShownTaskReceipt, TaskStatus,
    UpdateContext,
};
use std::{cell::RefCell, rc::Rc, time::Duration};

/// Refreshes its own content one flush after Enter, while it is shown.
struct Refreshing(Rc<RefCell<Option<ShownTaskReceipt>>>);
impl Component for Refreshing {
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> { Ok(()) }
    fn focusable(&self) -> bool { true }
    fn keyboard_event(
        &mut self,
        cx: &mut EventContext,
        event: KeyEvent,
        _: &KeyboardState,
    ) -> Result<InputOutcome, RuntimeError> {
        if event.transition == KeyTransition::Pressed {
            // Describing this component's children again would go here.
            *self.0.borrow_mut() = Some(cx.defer_while_shown(|_ui| Ok(()))?);
        }
        Ok(InputOutcome::Handled)
    }
}

let mut ui = Runtime::new();
let receipt = Rc::new(RefCell::new(None));
let id = ui.mount_tree(Element::new("refreshing", Refreshing(receipt.clone())))?;
ui.focus(&id)?;
ui.key_input(KeyCode::Enter, KeyAction::Press)?;
// Hidden before the flush delivers it: the refresh is skipped.
ui.set_visible(&id, false)?;
ui.flush(Duration::ZERO)?;
let status = receipt.borrow().as_ref().map(ShownTaskReceipt::status);
assert_eq!(status, Some(TaskStatus::Cancelled));
```

##### [§](#errors-10)Errors

Returns `TaskQueueFull` at
[`Runtime::MAX_TASKS`](struct.Runtime.html#associatedconstant.MAX_TASKS "associated constant pert_ui::Runtime::MAX_TASKS")
pending operations, or the runtime phase/shutdown error, including from
cleanup callbacks that run during
[`Runtime::shutdown`](struct.Runtime.html#method.shutdown "method pert_ui::Runtime::shutdown"),
without accepting the operation.

[Source](../src/pert_ui_runtime/runtime/tasks.rs.html#481-485)

#### pub fn [spawn](#method.spawn)\<T\>( &self, work: impl [FnOnce](https://doc.rust-lang.org/1.99.1/core/ops/function/trait.FnOnce.html "trait core::ops::function::FnOnce")() -\> T + [Send](https://doc.rust-lang.org/1.99.1/core/marker/trait.Send.html "trait core::marker::Send") + 'static, deliver: impl [FnOnce](https://doc.rust-lang.org/1.99.1/core/ops/function/trait.FnOnce.html "trait core::ops::function::FnOnce")(T) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[()](https://doc.rust-lang.org/1.99.1/std/primitive.unit.html), [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\> + 'static, ) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[TaskReceipt](struct.TaskReceipt.html "struct pert_ui::TaskReceipt"), [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\>

where T:
[Send](https://doc.rust-lang.org/1.99.1/core/marker/trait.Send.html "trait core::marker::Send") +
'static,

Starts `work` on a background thread from a callback and hands its
result to `deliver` on the UI thread, as
[`Runtime::spawn`](struct.Runtime.html#method.spawn "method pert_ui::Runtime::spawn")
does.

Timing class: [at the next
flush](https://github.com/pertyjons/pert-ui/blob/main/docs/contracts.md#at-the-next-flush),
for the start and for `deliver`.

A callback has no runtime, so this queues the start: it is the operation
`|ui| ui.spawn(work, deliver).map(|_| ())` queued with
[`Self::defer`](struct.EventContext.html#method.defer "method pert_ui::EventContext::defer").
The thread starts at the beginning of the next flush, before component
evaluation, as every queued operation does. The start belongs to the
application, as `defer`’s operation does: it runs at the next flush
whatever happens to the widget or its window, and only shutdown cancels
it before it starts. Once started, the result follows
[`Runtime::spawn`](struct.Runtime.html#method.spawn "method pert_ui::Runtime::spawn")’s
rules: results are delivered in arrival order, a panicking `work` fails
the flush that receives its outcome with
[`RuntimeError::BackgroundPanicked`](enum.RuntimeError.html#variant.BackgroundPanicked "variant pert_ui::RuntimeError::BackgroundPanicked"),
and nothing is delivered after shutdown.

The
[`TaskReceipt`](struct.TaskReceipt.html "struct pert_ui::TaskReceipt")
reports the start, not the result:

- [`TaskStatus::Succeeded`](enum.TaskStatus.html#variant.Succeeded "variant pert_ui::TaskStatus::Succeeded"):
  the thread started.
- [`TaskStatus::Failed`](enum.TaskStatus.html#variant.Failed "variant pert_ui::TaskStatus::Failed"):
  [`Runtime::spawn`](struct.Runtime.html#method.spawn "method pert_ui::Runtime::spawn")
  refused with that error and nothing started, such as `BackgroundFull`
  while
  [`Runtime::MAX_BACKGROUND_TASKS`](struct.Runtime.html#associatedconstant.MAX_BACKGROUND_TASKS "associated constant pert_ui::Runtime::MAX_BACKGROUND_TASKS")
  tasks wait, `BackgroundSpawn` with the operating system’s reason, or
  `Exhausted`. As for every queued operation, the error also propagates
  from that flush.
- [`TaskStatus::Cancelled`](enum.TaskStatus.html#variant.Cancelled "variant pert_ui::TaskStatus::Cancelled"):
  the runtime shut down before the start.

It may be dropped, as `cx.spawn(...)?;` does, for the reasons
[`Self::defer`](struct.EventContext.html#method.defer "method pert_ui::EventContext::defer")
gives.

There is no handle to cancel the delivery. Code that must discard a
result that has not been delivered, or tie the start to a component,
queues
[`Runtime::spawn`](struct.Runtime.html#method.spawn "method pert_ui::Runtime::spawn")
itself, with
[`Self::defer`](struct.EventContext.html#method.defer "method pert_ui::EventContext::defer")
or
[`Self::defer_while_shown`](struct.EventContext.html#method.defer_while_shown "method pert_ui::EventContext::defer_while_shown"),
and keeps the
[`BackgroundTask`](struct.BackgroundTask.html "struct pert_ui::BackgroundTask")
it returns. `work` receives no UI handles; return what the interface
needs and let `deliver` write it to signals.

``` rust
use pert_ui::{Button, Element, KeyAction, KeyCode, Runtime};
use std::time::Duration;
let mut ui = Runtime::new();
let status = ui.signal("Ready".to_owned());
let shown = status.clone();
let count = Button::new("Count words", move |cx| {
    shown.set("Counting".to_owned())?;
    // Real work would read a file; the result carries any failure.
    let text = String::from("one two three");
    let shown = shown.clone();
    cx.spawn(
        move || text.split_whitespace().count(),
        move |words| shown.set(format!("{words} words")),
    )?;
    Ok(())
});
let button = ui.mount_tree(Element::new("count", count))?;
ui.flush(Duration::ZERO)?;
ui.focus(&button)?;
ui.key_input(KeyCode::Space, KeyAction::Press)?;
ui.key_input(KeyCode::Space, KeyAction::Release)?;
// A native host flushes when the result wakes it; a headless one polls.
while status.get()? == "Counting" {
    std::thread::sleep(Duration::from_millis(1));
    ui.flush(Duration::ZERO)?;
}
assert_eq!(status.get()?, "3 words");
```

##### [§](#errors-11)Errors

Returns `TaskQueueFull` at
[`Runtime::MAX_TASKS`](struct.Runtime.html#associatedconstant.MAX_TASKS "associated constant pert_ui::Runtime::MAX_TASKS")
pending operations, or the runtime phase/shutdown error, including from
cleanup callbacks that run during
[`Runtime::shutdown`](struct.Runtime.html#method.shutdown "method pert_ui::Runtime::shutdown"),
without accepting the operation.
[`Runtime::spawn`](struct.Runtime.html#method.spawn "method pert_ui::Runtime::spawn")’s
own refusals arrive later, in the receipt and from the flush that starts
the work.

[Source](../src/pert_ui_runtime/runtime.rs.html#3131)[§](#impl-EventContext-6)

### impl [EventContext](struct.EventContext.html "struct pert_ui::EventContext")

[Source](../src/pert_ui_runtime/runtime.rs.html#3140)

#### pub fn [signal](#method.signal)\<T\>(&self, value: T) -\> [Signal](struct.Signal.html "struct pert_ui::Signal")\<T\>

Creates a signal in this component’s runtime, for state a component
creates while it reacts to input, such as a menu it builds on demand.

It is the same signal
[`Runtime::signal`](struct.Runtime.html#method.signal "method pert_ui::Runtime::signal")
and
[`UpdateContext::signal`](struct.UpdateContext.html#method.signal "method pert_ui::UpdateContext::signal")
create, with the same lifetime: the value lives as long as any clone of
the handle, which is UI-thread local. Reads and writes after the runtime
closes fail with
[`RuntimeError::Closed`](enum.RuntimeError.html#variant.Closed "variant pert_ui::RuntimeError::Closed").

[Source](../src/pert_ui_runtime/runtime.rs.html#3155-3160)

#### pub fn [text](#method.text)( &self, text: &[str](https://doc.rust-lang.org/1.99.1/std/primitive.str.html), width: [Option](https://doc.rust-lang.org/1.99.1/core/option/enum.Option.html "enum core::option::Option")\<[f32](https://doc.rust-lang.org/1.99.1/std/primitive.f32.html)\>, style: [TextStyle](enum.TextStyle.html "enum pert_ui::TextStyle"), ) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[TextLayout](struct.TextLayout.html "struct pert_ui::TextLayout"), [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\>

Prepares text with the shared cache and current scale/style for hit
testing or validation before a committed edit. Width is optional logical
pixels. Returns the same bounded text errors as `MeasureContext::text`;
this does not register layout dependencies or schedule rendering.

##### [§](#errors-12)Errors

Returns
[`RuntimeError::Text`](enum.RuntimeError.html#variant.Text "variant pert_ui::RuntimeError::Text")
for invalid geometry (a non-finite or negative width, a zero font size),
an unreadable font or a result beyond a
[`TextLimit`](enum.TextLimit.html "enum pert_ui::TextLimit");
[`RuntimeError::TextEdit`](enum.RuntimeError.html#variant.TextEdit "variant pert_ui::RuntimeError::TextEdit")
with
[`TextEditError::TooLarge`](enum.TextEditError.html#variant.TooLarge "variant pert_ui::TextEditError::TooLarge")
for text longer than
[`TextEditError::MAX_TEXT_BYTES`](enum.TextEditError.html#associatedconstant.MAX_TEXT_BYTES "associated constant pert_ui::TextEditError::MAX_TEXT_BYTES");
and `BorrowConflict` when text is prepared reentrantly.

[Source](../src/pert_ui_runtime/runtime.rs.html#3234)

#### pub fn [now](#method.now)(&self) -\> [Duration](https://doc.rust-lang.org/1.99.1/core/time/struct.Duration.html "struct core::time::Duration")

Current runtime elapsed time, for deterministic input timing such as
double-click recognition. Native hosts advance this when dispatching
events.

It is the runtime’s time, which only
[`Runtime::flush`](struct.Runtime.html#method.flush "method pert_ui::Runtime::flush")
and
[`HostIntegration::service_intervals`](host/trait.HostIntegration.html#tymethod.service_intervals "method pert_ui::host::HostIntegration::service_intervals")
advance: input dispatched without one of them in between shares one
instant. A headless host or test therefore advances time, for example by
flushing with a later time, before the next input, as here, where two
presses half a second apart are not a double click and two presses in
one instant are.

``` rust
use pert_ui::testing::HostIntegration;
use pert_ui::{component::*, prelude::*};
use std::{cell::Cell, rc::Rc, time::Duration};
struct Clicks { last: Option<Duration>, doubles: Rc<Cell<u32>> }
impl Component for Clicks {
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> { Ok(()) }
    fn pointer_input(&mut self, cx: &mut EventContext, event: PointerEvent)
        -> Result<InputOutcome, RuntimeError> {
        if event.kind() != PointerKind::Press(PointerButton::Primary) {
            return Ok(InputOutcome::Ignored);
        }
        let now = cx.now();
        if self.last.is_some_and(|last| now - last <= Duration::from_millis(400)) {
            self.doubles.set(self.doubles.get() + 1);
            self.last = None;
        } else {
            self.last = Some(now);
        }
        Ok(InputOutcome::Handled)
    }
}
let doubles = Rc::new(Cell::new(0));
let mut ui = Runtime::new();
ui.set_viewport(Viewport::new(100.0, 100.0, 1.0)?)?;
ui.mount_tree(Element::new("area", Clicks { last: None, doubles: doubles.clone() }))?;
let press = |ui: &mut Runtime| -> Result<(), RuntimeError> {
    let at = Point::new(10.0, 10.0)?;
    ui.pointer_input(PointerEvent::new(at, PointerKind::Press(PointerButton::Primary)))?;
    ui.pointer_input(PointerEvent::new(at, PointerKind::Release(PointerButton::Primary)))?;
    Ok(())
};
ui.flush(Duration::ZERO)?;
press(&mut ui)?;
ui.flush(Duration::from_millis(500))?; // the clock moves here
press(&mut ui)?;
assert_eq!(doubles.get(), 0);
press(&mut ui)?; // same instant as the previous press
assert_eq!(doubles.get(), 1);
ui.service_intervals(Duration::from_millis(1500))?; // also moves the clock
press(&mut ui)?;
press(&mut ui)?;
assert_eq!(doubles.get(), 2);
assert_eq!(ui.now(), Duration::from_millis(1500));
```

[Source](../src/pert_ui_runtime/runtime.rs.html#3266)

#### pub fn [to_local](#method.to_local)(&self, point: [Point](struct.Point.html "struct pert_ui::Point")) -\> [Option](https://doc.rust-lang.org/1.99.1/core/option/enum.Option.html "enum core::option::Option")\<[Point](struct.Point.html "struct pert_ui::Point")\>

Converts a window-logical position into this component’s local space.

Pointer positions and component bounds share the window-logical space. A
component reaches its own coordinates by subtracting its displayed
origin and, inside a [view
transform](struct.ViewTransform.html "struct pert_ui::ViewTransform"),
dividing by the accumulated scale. This is that conversion, so the rule
lives in one place rather than in every widget that handles a pointer:

``` rust
fn pointer_input(&mut self, cx: &mut EventContext, e: PointerEvent)
    -> Result<InputOutcome, RuntimeError> {
    if let Some(local) = cx.to_local(e.position()) {
        let _ = local.x();
    }
    Ok(InputOutcome::Handled)
}
```

Returns `None` before this component has been arranged, since it then
has no origin to subtract. The result may fall outside the component’s
own bounds, which is normal while it holds a pointer capture and the
pointer leaves it; callers that need containment test it themselves.

[Source](../src/pert_ui_runtime/runtime.rs.html#3280)

#### pub fn [to_local_delta](#method.to_local_delta)(&self, delta: [Point](struct.Point.html "struct pert_ui::Point")) -\> [Point](struct.Point.html "struct pert_ui::Point")

Converts a window-logical displacement, such as the difference between
two pointer positions, into this component’s local pixels.

Outside a [view
transform](struct.ViewTransform.html "struct pert_ui::ViewTransform")
the two are equal. Inside one, a drag of ten window pixels over content
shown at 200% moves five local pixels. Thresholds that describe the hand
rather than the content, such as a click or drag distance, stay in
window pixels.

A displacement too large to express at a deeply nested small scale
saturates at the largest finite value with its sign.

[Source](../src/pert_ui_runtime/runtime.rs.html#3290)

#### pub fn [to_local_rect](#method.to_local_rect)(&self, rect: [Rect](struct.Rect.html "struct pert_ui::Rect")) -\> [Option](https://doc.rust-lang.org/1.99.1/core/option/enum.Option.html "enum core::option::Option")\<[Rect](struct.Rect.html "struct pert_ui::Rect")\>

Converts a window-logical rectangle, such as the one
[`Component::reveal`](trait.Component.html#method.reveal "method pert_ui::Component::reveal")
receives, into this component’s local space, where it compares with
[`Self::size`](struct.EventContext.html#method.size "method pert_ui::EventContext::size").
`None` before this component is arranged.

[Source](../src/pert_ui_runtime/runtime.rs.html#3307)

#### pub fn [position_in_parent](#method.position_in_parent)(&self) -\> [Option](https://doc.rust-lang.org/1.99.1/core/option/enum.Option.html "enum core::option::Option")\<[Point](struct.Point.html "struct pert_ui::Point")\>

This component’s origin in its presentation parent’s content space: the
position the parent’s `arrange` gave it, whatever view transforms apply.
`None` at a root or before layout.

[Source](../src/pert_ui_runtime/runtime.rs.html#3328)

#### pub fn [parent_size](#method.parent_size)(&self) -\> [Option](https://doc.rust-lang.org/1.99.1/core/option/enum.Option.html "enum core::option::Option")\<[Size](struct.Size.html "struct pert_ui::Size")\>

Last successful allocation of the presentation parent in the parent’s
own local pixels, the space
[`Self::position_in_parent`](struct.EventContext.html#method.position_in_parent "method pert_ui::EventContext::position_in_parent")
is measured in when the parent applies no view transform of its own.

[Source](../src/pert_ui_runtime/runtime.rs.html#3334)

#### pub fn [view_scale](#method.view_scale)(&self) -\> [f32](https://doc.rust-lang.org/1.99.1/std/primitive.f32.html)

Accumulated [view
scale](struct.ViewTransform.html "struct pert_ui::ViewTransform") of
this component’s local space: window-logical pixels per local pixel, 1
outside any view transform.

[Source](../src/pert_ui_runtime/runtime.rs.html#3342)

#### pub fn [size](#method.size)(&self) -\> [Option](https://doc.rust-lang.org/1.99.1/core/option/enum.Option.html "enum core::option::Option")\<[Size](struct.Size.html "struct pert_ui::Size")\>

Last successful allocation in this component’s own local pixels, absent
before the component’s first layout. Use this, not
[`Self::bounds`](struct.EventContext.html#method.bounds "method pert_ui::EventContext::bounds"),
for local geometry: `bounds` is the window rectangle, which a view
transform scales.

[Source](../src/pert_ui_runtime/runtime.rs.html#3376-3380)

#### pub fn [request_text_command](#method.request_text_command)( &mut self, target: &[WidgetId](struct.WidgetId.html "struct pert_ui::WidgetId"), command: [TextCommand](enum.TextCommand.html "enum pert_ui::TextCommand"), ) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[()](https://doc.rust-lang.org/1.99.1/std/primitive.unit.html), [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\>

Queues a text command for another component’s own text, such as a
context-menu item acting on the editor it was opened from, or a list’s
activation beginning the rename of a row’s label.

The runtime delivers it to
[`Component::text_command`](trait.Component.html#method.text_command "method pert_ui::Component::text_command")
in the event phase of this flush, in request order, so the receiving
editor may commit edits as it does for key input. The clipboard is not
read here, and only two commands request focus:
[`TextCommand::BeginEdit`](enum.TextCommand.html#variant.BeginEdit "variant pert_ui::TextCommand::BeginEdit")
moves it into the editing part, and
[`TextCommand::DiscardDraft`](enum.TextCommand.html#variant.DiscardDraft "variant pert_ui::TextCommand::DiscardDraft"),
when it ends an in-place session (such as an
[`InlineText`](struct.InlineText.html "struct pert_ui::InlineText")’s)
whose editor holds focus, returns it to the owning control, as Escape
does; an
[`InlineText`](struct.InlineText.html "struct pert_ui::InlineText") in
[row
mode](struct.InlineText.html#method.in_row "method pert_ui::InlineText::in_row"),
which is not focusable at rest, requests none, and focus falls back to
its nearest focusable ancestor, such as its list, as when Escape ends
the session. Either request settles at the end of the same flush when
that flush succeeds, as
[`Self::request_focus`](struct.EventContext.html#method.request_focus "method pert_ui::EventContext::request_focus")
describes. Routing follows
[`Runtime::text_command`](struct.Runtime.html#method.text_command "method pert_ui::Runtime::text_command"):
a target that is not an editor passes the command on to the focused
component inside its subtree. A request whose target is unmounted or
hidden before it applies is dropped; a target blocked by current
modality returns `ModalBlocked` from the flush without reading the
clipboard or changing the editor. The refused request is spent and later
requests remain queued. An error from the target propagates from
`Runtime::flush` and leaves the remaining requests queued. Requesting a
command for a component that is not an editor is not an error and does
nothing.

##### [§](#errors-13)Errors

Returns `ForeignRuntime` immediately for a handle from another runtime.

[Source](../src/pert_ui_runtime/runtime.rs.html#3401)

#### pub fn [clipboard_text](#method.clipboard_text)(&self) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[String](https://doc.rust-lang.org/1.99.1/alloc/string/struct.String.html "struct alloc::string::String"), [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\>

Reads the host clipboard on the UI thread.

##### [§](#errors-14)Errors

Returns
[`RuntimeError::Closed`](enum.RuntimeError.html#variant.Closed "variant pert_ui::RuntimeError::Closed")
after shutdown, and `WriteDuringUpdate`, `WriteDuringLayout`,
`WriteDuringPaint`, `WriteDuringInspection` or `WriteDuringDelivery`
when called from inside that phase; `BorrowConflict` when called from
inside the clipboard service;
[`ClipboardError::Unavailable`](enum.ClipboardError.html#variant.Unavailable "variant pert_ui::ClipboardError::Unavailable")
when no service is installed; and the service’s own
[`ClipboardError`](enum.ClipboardError.html "enum pert_ui::ClipboardError"),
such as `NoText` or `Busy`.

[Source](../src/pert_ui_runtime/runtime.rs.html#3410)

#### pub fn [set_clipboard_text](#method.set_clipboard_text)(&self, text: &[str](https://doc.rust-lang.org/1.99.1/std/primitive.str.html)) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[()](https://doc.rust-lang.org/1.99.1/std/primitive.unit.html), [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\>

Writes the host clipboard on the UI thread. Cut should modify its
binding only after this succeeds.

##### [§](#errors-15)Errors

As
[`Self::clipboard_text`](struct.EventContext.html#method.clipboard_text "method pert_ui::EventContext::clipboard_text");
the service reports a failed write as its own
[`ClipboardError`](enum.ClipboardError.html "enum pert_ui::ClipboardError").

[Source](../src/pert_ui_runtime/runtime.rs.html#3417)

#### pub fn [modifiers](#method.modifiers)(&self) -\> [Modifiers](struct.Modifiers.html "struct pert_ui::Modifiers")

Current physical modifier state, including keys held before focus
arrived. Window focus loss clears it independently of component gesture
ownership.

[Source](../src/pert_ui_runtime/runtime.rs.html#3422)

#### pub fn [id](#method.id)(&self) -\> [WidgetId](struct.WidgetId.html "struct pert_ui::WidgetId")

Identity of this event recipient, valid until unmount.

[Source](../src/pert_ui_runtime/runtime.rs.html#3431)

#### pub fn [set_profiling](#method.set_profiling)(&self, enabled: [bool](https://doc.rust-lang.org/1.99.1/std/primitive.bool.html)) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[()](https://doc.rust-lang.org/1.99.1/std/primitive.unit.html), [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\>

Enables/disables profiling from a UI event, retaining completed
statistics.

##### [§](#errors-16)Errors

Returns
[`RuntimeError::Closed`](enum.RuntimeError.html#variant.Closed "variant pert_ui::RuntimeError::Closed")
after shutdown, and `WriteDuringUpdate`, `WriteDuringLayout`,
`WriteDuringPaint`, `WriteDuringInspection` or `WriteDuringDelivery`
when called from inside that phase.

[Source](../src/pert_ui_runtime/runtime.rs.html#3438)

#### pub fn [profile](#method.profile)(&self) -\> [ProfileSnapshot](struct.ProfileSnapshot.html "struct pert_ui::ProfileSnapshot")

Captures completed timing scopes without polling or live runtime
handles.

[Source](../src/pert_ui_runtime/runtime.rs.html#3447)

#### pub fn [clear_profile](#method.clear_profile)(&self) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[()](https://doc.rust-lang.org/1.99.1/std/primitive.unit.html), [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\>

Clears completed and in-flight timings while preserving profiling
enablement.

##### [§](#errors-17)Errors

Returns
[`RuntimeError::Closed`](enum.RuntimeError.html#variant.Closed "variant pert_ui::RuntimeError::Closed")
after shutdown, and `WriteDuringUpdate`, `WriteDuringLayout`,
`WriteDuringPaint`, `WriteDuringInspection` or `WriteDuringDelivery`
when called from inside that phase.

[Source](../src/pert_ui_runtime/runtime.rs.html#3455)

#### pub fn [notifier](#method.notifier)(&self) -\> [Notifier](struct.Notifier.html "struct pert_ui::Notifier")

A handle through which this component asks for
[`Component::deliver_notifications`](trait.Component.html#method.deliver_notifications "method pert_ui::Component::deliver_notifications");
see
[`UpdateContext::notifier`](struct.UpdateContext.html#method.notifier "method pert_ui::UpdateContext::notifier").

[Source](../src/pert_ui_runtime/runtime.rs.html#3470)

#### pub fn [request_focus](#method.request_focus)(&self)

Requests focus for this recipient after pending updates and layout
complete. The request is ignored if it becomes hidden, unmounted or
nonfocusable. A recipient hidden only until an ancestor’s update still
pending in the same flush shows it, such as an editor its owner reveals,
keeps the request until that update has run, so focus moves when the
flush settles successfully. A flush that fails first leaves the request
pending with that update, and a later flush retries both. A focus
restoration meanwhile cancels it, except that a focused component
containing the recipient, as a label contains its editor, keeps focus
for the transfer even when it stopped being focusable to make it.
Multiple requests before a flush use the latest recipient; IDs are never
retargeted. Explicit focus transfers and window focus loss cancel any
older request.

[Source](../src/pert_ui_runtime/runtime.rs.html#3488)

#### pub fn [request_focus_for](#method.request_focus_for)(&self, target: &[WidgetId](struct.WidgetId.html "struct pert_ui::WidgetId")) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[()](https://doc.rust-lang.org/1.99.1/std/primitive.unit.html), [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\>

Requests focus for another mount, under the rules of
[`Self::request_focus`](struct.EventContext.html#method.request_focus "method pert_ui::EventContext::request_focus"):
it moves after pending updates and layout complete, and is ignored if
the target is then hidden, unmounted, nonfocusable or outside an open
modal surface. The request belongs to the target’s native window and
replaces an earlier pending request there.

This is how application coordination, such as a form sending the user to
its first field in error from a submit callback, moves focus without
access to the `Runtime`. The ID is never retargeted: a control mounted
later under the same key is a different identity and is not focused.

##### [§](#errors-18)Errors

Returns `ForeignRuntime` for an ID of another runtime.

[Source](../src/pert_ui_runtime/runtime.rs.html#3511)

#### pub fn [request_focus_within](#method.request_focus_within)( &self, target: &[WidgetId](struct.WidgetId.html "struct pert_ui::WidgetId"), ) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[()](https://doc.rust-lang.org/1.99.1/std/primitive.unit.html), [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\>

Requests focus for the first focusable component inside another mount,
in Tab order, or for the mount itself when it is focusable, under the
rules of
[`Self::request_focus_for`](struct.EventContext.html#method.request_focus_for "method pert_ui::EventContext::request_focus_for"):
it moves after pending updates and layout complete, replaces an earlier
pending request in the target’s window, and is ignored if the target is
then hidden or unmounted, or holds no visible focusable component that
an open modal surface allows.

It returns focus to a region whose components the caller does not know,
as a tool window does when Escape sends the user back to the main
content: the target is the region’s container, which need not be
focusable itself.

##### [§](#errors-19)Errors

Returns `ForeignRuntime` for an ID of another runtime.

[Source](../src/pert_ui_runtime/runtime.rs.html#3521)

#### pub fn [bounds](#method.bounds)(&self) -\> [Option](https://doc.rust-lang.org/1.99.1/core/option/enum.Option.html "enum core::option::Option")\<[Rect](struct.Rect.html "struct pert_ui::Rect")\>

Last successful allocation in window-logical coordinates, absent before
the component’s first layout. Inside a [view
transform](struct.ViewTransform.html "struct pert_ui::ViewTransform")
its extent is the displayed one;
[`Self::size`](struct.EventContext.html#method.size "method pert_ui::EventContext::size")
is the local allocation.

[Source](../src/pert_ui_runtime/runtime.rs.html#3527)

#### pub fn [parent_bounds](#method.parent_bounds)(&self) -\> [Option](https://doc.rust-lang.org/1.99.1/core/option/enum.Option.html "enum core::option::Option")\<[Rect](struct.Rect.html "struct pert_ui::Rect")\>

Last successful allocation of the logical parent, or None at a root.
Coordinates are logical window pixels, like
[`Self::bounds`](struct.EventContext.html#method.bounds "method pert_ui::EventContext::bounds").

[Source](../src/pert_ui_runtime/runtime.rs.html#3534)

#### pub fn [focused](#method.focused)(&self) -\> [bool](https://doc.rust-lang.org/1.99.1/std/primitive.bool.html)

Whether this recipient currently has keyboard focus.

[Source](../src/pert_ui_runtime/runtime.rs.html#3539)

#### pub fn [pointer_inside](#method.pointer_inside)(&self) -\> [bool](https://doc.rust-lang.org/1.99.1/std/primitive.bool.html)

Whether the current pointer is inside this component’s effective clip.

[Source](../src/pert_ui_runtime/runtime.rs.html#3558)

#### pub fn [pointer_position](#method.pointer_position)(&self) -\> [Option](https://doc.rust-lang.org/1.99.1/core/option/enum.Option.html "enum core::option::Option")\<[Point](struct.Point.html "struct pert_ui::Point")\>

The latest pointer position in this recipient’s window, in window
coordinates, or `None` before the first pointer event or after the
pointer left the window.

Pointer callbacks receive their position with the event. This is for
work that another input triggers, such as re-checking which part of a
component is under a stationary pointer after Escape cancels a drag.
Convert it with
[`Self::to_local`](struct.EventContext.html#method.to_local "method pert_ui::EventContext::to_local");
combine it with
[`Self::pointer_inside`](struct.EventContext.html#method.pointer_inside "method pert_ui::EventContext::pointer_inside")
to ignore a pointer over clipped-away parts.

[Source](../src/pert_ui_runtime/runtime.rs.html#3566)

#### pub fn [capture_pointer](#method.capture_pointer)(&self) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[()](https://doc.rust-lang.org/1.99.1/std/primitive.unit.html), [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\>

Keeps subsequent pointer events on this recipient until release/context
loss.

##### [§](#errors-20)Errors

Returns
[`RuntimeError::CaptureUnavailable`](enum.RuntimeError.html#variant.CaptureUnavailable "variant pert_ui::RuntimeError::CaptureUnavailable")
when no pointer button is held or another component owns the capture.

[Source](../src/pert_ui_runtime/runtime.rs.html#3582)

#### pub fn [release_pointer](#method.release_pointer)(&self)

Releases this recipient’s pointer capture, if any.

[Source](../src/pert_ui_runtime/runtime.rs.html#3596)

#### pub fn [invalidate](#method.invalidate)(&self, kind: [Invalidation](enum.Invalidation.html "enum pert_ui::Invalidation"))

Invalidates retained local component data changed directly in an event.

Signal bindings already invalidate automatically. Component authors use
this for private state, requesting `Layout` when measurements can
change. Invalidate only when that state changed: a layout is followed by
a repeated pointer move
([`PointerEvent::is_refresh`](struct.PointerEvent.html#method.is_refresh "method pert_ui::PointerEvent::is_refresh")),
so a move handler that invalidates layout unconditionally, or causes
another layout through what it writes, never settles.

[Source](../src/pert_ui_runtime/runtime.rs.html#3678)

#### pub fn [set_timer](#method.set_timer)(&self, delay: [Duration](https://doc.rust-lang.org/1.99.1/core/time/struct.Duration.html "struct core::time::Duration")) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[()](https://doc.rust-lang.org/1.99.1/std/primitive.unit.html), [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\>

Schedules one event-phase wake for this recipient after `delay`.

The runtime calls
[`Component::timer_elapsed`](trait.Component.html#method.timer_elapsed "method pert_ui::Component::timer_elapsed")
with an `EventContext` in the first flush at or after the deadline,
before that flush’s updates. A timed gesture such as a held step repeat
or auto-scroll may therefore write bindings and commit parameter edits
on time, exactly like ordinary input.
[`UpdateContext::animate`](struct.UpdateContext.html#method.animate "method pert_ui::UpdateContext::animate")
and
[`UpdateContext::wake_after`](struct.UpdateContext.html#method.wake_after "method pert_ui::UpdateContext::wake_after")
remain the presentation-only deadlines and are unaffected by this timer.

A component owns at most one timer; this replaces a pending one. Repeat
the call inside the callback to continue a repeat. Missed deadlines
produce one call without catch-up, and the deterministic elapsed clock,
not wall-clock time, decides eligibility: no OS timer or background
executor is started, and the host wakes at
[`Runtime::next_deadline`](struct.Runtime.html#method.next_deadline "method pert_ui::Runtime::next_deadline").
The timer is cancelled with this component’s input context, so focus
loss, hiding, unmounting, presentation movement and
`HostIntegration::cancel_input` all stop it. An Escape press that
reaches the focused component’s ordinary delivery stops it silently,
without
[`Component::cancel_input`](trait.Component.html#method.cancel_input "method pert_ui::Component::cancel_input").
An earlier consumer of Escape (a drag, delayed help, an editor, an open
surface; see
[`Runtime::key_input`](struct.Runtime.html#method.key_input "method pert_ui::Runtime::key_input"))
skips that step, so the timer keeps running unless the consumer’s own
lifecycle stops it, as closing a surface does for the components it
hides. A hidden or removed component never receives the call.

``` rust
use pert_ui::{Component, Element, EventContext, InputOutcome, KeyAction,
    KeyCode, KeyEvent, KeyTransition, KeyboardState, Runtime, RuntimeError, Signal,
    UpdateContext};
use std::time::Duration;

struct HeldStep(Signal<i32>);
impl Component for HeldStep {
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> { Ok(()) }
    fn focusable(&self) -> bool { true }
    fn keyboard_event(
        &mut self,
        cx: &mut EventContext,
        event: KeyEvent,
        _: &KeyboardState,
    ) -> Result<InputOutcome, RuntimeError> {
        match event.transition {
            // The press saves one step and arms the repeat.
            KeyTransition::Pressed => {
                self.0.set(self.0.get()? + 1)?;
                cx.set_timer(Duration::from_millis(400))?;
            }
            KeyTransition::Released | KeyTransition::UnmatchedRelease => cx.cancel_timer(),
            KeyTransition::Repeated => {}
        }
        Ok(InputOutcome::Handled)
    }
    fn timer_elapsed(&mut self, cx: &mut EventContext) -> Result<(), RuntimeError> {
        self.0.set(self.0.get()? + 1)?;
        cx.set_timer(Duration::from_millis(80))
    }
}

let mut ui = Runtime::new();
let steps = ui.signal(0);
let stepper = ui.mount_tree(Element::new("stepper", HeldStep(steps.clone())))?;
ui.focus(&stepper)?;
ui.key_input(KeyCode::ArrowUp, KeyAction::Press)?;
assert_eq!(steps.get()?, 1);
ui.flush(Duration::from_millis(400))?;
assert_eq!(steps.get()?, 2, "the held repeat committed when it was due");
ui.key_input(KeyCode::ArrowUp, KeyAction::Release)?;
ui.flush(Duration::from_secs(1))?;
assert_eq!(steps.get()?, 2);
assert_eq!(ui.next_deadline(), None);
```

##### [§](#errors-21)Errors

Returns a runtime phase error (see
[`Runtime`](struct.Runtime.html "struct pert_ui::Runtime")),
[`RuntimeError::ZeroDelay`](enum.RuntimeError.html#variant.ZeroDelay "variant pert_ui::RuntimeError::ZeroDelay")
for a zero delay and
[`RuntimeError::Exhausted`](enum.RuntimeError.html#variant.Exhausted "variant pert_ui::RuntimeError::Exhausted")
when elapsed time cannot represent the deadline.

[Source](../src/pert_ui_runtime/runtime.rs.html#3697)

#### pub fn [cancel_timer](#method.cancel_timer)(&self)

Cancels a pending
[`Self::set_timer`](struct.EventContext.html#method.set_timer "method pert_ui::EventContext::set_timer")
request; does nothing without one. Terminal gestures should cancel
explicitly instead of relying on input cancellation, which is a
lifecycle event rather than a release.

## Trait Implementations[§](#trait-implementations)

[Source](../src/pert_ui_runtime/runtime.rs.html#3125)[§](#impl-Debug-for-EventContext)

### impl [Debug](https://doc.rust-lang.org/1.99.1/core/fmt/trait.Debug.html "trait core::fmt::Debug") for [EventContext](struct.EventContext.html "struct pert_ui::EventContext")

Shows nothing of the component it is scoped to.

[Source](../src/pert_ui_runtime/runtime.rs.html#3126)[§](#method.fmt)

#### fn [fmt](https://doc.rust-lang.org/1.99.1/core/fmt/trait.Debug.html#tymethod.fmt)(&self, f: &mut [Formatter](https://doc.rust-lang.org/1.99.1/core/fmt/struct.Formatter.html "struct core::fmt::Formatter")\<'\_\>) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[()](https://doc.rust-lang.org/1.99.1/std/primitive.unit.html), [Error](https://doc.rust-lang.org/1.99.1/core/fmt/struct.Error.html "struct core::fmt::Error")\>

Formats the value using the given formatter. [Read
more](https://doc.rust-lang.org/1.99.1/core/fmt/trait.Debug.html#tymethod.fmt)

[Source](../src/pert_ui_runtime/signal.rs.html#184)[§](#impl-StateOwner-for-EventContext)

### impl [StateOwner](trait.StateOwner.html "trait pert_ui::StateOwner") for [EventContext](struct.EventContext.html "struct pert_ui::EventContext")

[Source](../src/pert_ui_runtime/signal.rs.html#185)[§](#method.signal-1)

#### fn [signal](trait.StateOwner.html#tymethod.signal)\<T\>(&self, value: T) -\> [Signal](struct.Signal.html "struct pert_ui::Signal")\<T\>

Creates a signal owned by this owner’s runtime, holding `value`.

[Source](../src/pert_ui_runtime/signal.rs.html#188)[§](#method.ui_settings-1)

#### fn [ui_settings](trait.StateOwner.html#tymethod.ui_settings)(&self) -\> [UiSettings](struct.UiSettings.html "struct pert_ui::UiSettings")

Shared UI-thread presentation settings for this owner’s runtime. [Read
more](trait.StateOwner.html#tymethod.ui_settings)

## Auto Trait Implementations[§](#synthetic-implementations)

[§](#impl-RefUnwindSafe-for-EventContext)

### impl \![RefUnwindSafe](https://doc.rust-lang.org/1.99.1/core/panic/unwind_safe/trait.RefUnwindSafe.html "trait core::panic::unwind_safe::RefUnwindSafe") for [EventContext](struct.EventContext.html "struct pert_ui::EventContext")

[§](#impl-Send-for-EventContext)

### impl \![Send](https://doc.rust-lang.org/1.99.1/core/marker/trait.Send.html "trait core::marker::Send") for [EventContext](struct.EventContext.html "struct pert_ui::EventContext")

[§](#impl-Sync-for-EventContext)

### impl \![Sync](https://doc.rust-lang.org/1.99.1/core/marker/trait.Sync.html "trait core::marker::Sync") for [EventContext](struct.EventContext.html "struct pert_ui::EventContext")

[§](#impl-UnwindSafe-for-EventContext)

### impl \![UnwindSafe](https://doc.rust-lang.org/1.99.1/core/panic/unwind_safe/trait.UnwindSafe.html "trait core::panic::unwind_safe::UnwindSafe") for [EventContext](struct.EventContext.html "struct pert_ui::EventContext")

[§](#impl-Freeze-for-EventContext)

### impl [Freeze](https://doc.rust-lang.org/1.99.1/core/marker/trait.Freeze.html "trait core::marker::Freeze") for [EventContext](struct.EventContext.html "struct pert_ui::EventContext")

[§](#impl-Unpin-for-EventContext)

### impl [Unpin](https://doc.rust-lang.org/1.99.1/core/marker/trait.Unpin.html "trait core::marker::Unpin") for [EventContext](struct.EventContext.html "struct pert_ui::EventContext")

[§](#impl-UnsafeUnpin-for-EventContext)

### impl [UnsafeUnpin](https://doc.rust-lang.org/1.99.1/core/marker/trait.UnsafeUnpin.html "trait core::marker::UnsafeUnpin") for [EventContext](struct.EventContext.html "struct pert_ui::EventContext")

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
