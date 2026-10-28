[pert_ui](index.html)

# Struct Runtime Copy item path

[Source](../src/pert_ui_runtime/runtime.rs.html#3894)

``` rust
pub struct Runtime { /* private fields */ }
```

Expand description

UI-local retained component runtime with a deterministic elapsed-time
clock.

Create with defaults. `signal` creates local state;
[`mount_tree`](struct.Runtime.html#method.mount_tree "method pert_ui::Runtime::mount_tree")
registers a keyed component tree. `flush` evaluates only pending visible
components, not every mount. The clock starts at zero and advances
through flush operations. Each native root has independent presentation
state; the default App waits for eligible deadlines or native events.
Custom hosts select a root before local input, flush and paint, and
coordinate roots through
[`application_deadline`](host/trait.HostIntegration.html#tymethod.application_deadline "method pert_ui::host::HostIntegration::application_deadline")
and
[`flush_windows`](host/trait.HostIntegration.html#tymethod.flush_windows "method pert_ui::host::HostIntegration::flush_windows")
of
[`HostIntegration`](host/trait.HostIntegration.html "trait pert_ui::host::HostIntegration").
This runtime records drawing without OS windows, GPUs or background
threads.

Input is delivered to its component at once, and the component writes
what its own rules write: a plain editor writes its binding immediately,
while a draft, a parameter proposal and a notification for a hidden
window are not written yet. The tree, derived bindings and layout show
the result after the next
[`Self::flush`](struct.Runtime.html#method.flush "method pert_ui::Runtime::flush"),
so a test flushes before it inspects;
[`testing::Driver`](testing/struct.Driver.html "struct pert_ui::testing::Driver")
settles after every action.

Methods that change state first check the runtime. Their `# Errors`
sections call that check’s failures *a runtime phase error*:
[`RuntimeError::Closed`](enum.RuntimeError.html#variant.Closed "variant pert_ui::RuntimeError::Closed")
after
[`Self::shutdown`](struct.Runtime.html#method.shutdown "method pert_ui::Runtime::shutdown"),
or
[`RuntimeError::WriteDuringUpdate`](enum.RuntimeError.html#variant.WriteDuringUpdate "variant pert_ui::RuntimeError::WriteDuringUpdate"),
[`RuntimeError::WriteDuringLayout`](enum.RuntimeError.html#variant.WriteDuringLayout "variant pert_ui::RuntimeError::WriteDuringLayout"),
[`RuntimeError::WriteDuringPaint`](enum.RuntimeError.html#variant.WriteDuringPaint "variant pert_ui::RuntimeError::WriteDuringPaint"),
[`RuntimeError::WriteDuringInspection`](enum.RuntimeError.html#variant.WriteDuringInspection "variant pert_ui::RuntimeError::WriteDuringInspection")
or
[`RuntimeError::WriteDuringDelivery`](enum.RuntimeError.html#variant.WriteDuringDelivery "variant pert_ui::RuntimeError::WriteDuringDelivery")
from inside that phase. These are the refusals of
[`Signal::set`](struct.Signal.html#method.set "method pert_ui::Signal::set")
apart from its `BorrowConflict`, and a refused call changes nothing.

[ⓘ](# "This example deliberately fails to compile")

``` rust
let runtime = pert_ui::Runtime::new();
std::thread::spawn(move || drop(runtime)); // UI ownership cannot cross threads.
```

## [§](#methods-by-audience)Methods by audience

The methods are listed by who calls them. An application needs the first
group; tests and tools add the second, which includes the host methods a
headless test uses to stand in for a host; a custom host also needs the
trait the third names. Each method’s own page holds its contract.

**Applications.** What an application’s own code calls.

- Content and state:
  [`new`](struct.Runtime.html#method.new "associated function pert_ui::Runtime::new"),
  [`signal`](struct.Runtime.html#method.signal "method pert_ui::Runtime::signal"),
  [`mount_tree`](struct.Runtime.html#method.mount_tree "method pert_ui::Runtime::mount_tree"),
  [`unmount`](struct.Runtime.html#method.unmount "method pert_ui::Runtime::unmount"),
  [`reconcile_children`](struct.Runtime.html#method.reconcile_children "method pert_ui::Runtime::reconcile_children"),
  [`reparent`](struct.Runtime.html#method.reparent "method pert_ui::Runtime::reparent"),
  [`set_visible`](struct.Runtime.html#method.set_visible "method pert_ui::Runtime::set_visible"),
  [`keep_alive`](struct.Runtime.html#method.keep_alive "method pert_ui::Runtime::keep_alive").
- Appearance:
  [`theme`](struct.Runtime.html#method.theme "method pert_ui::Runtime::theme"),
  [`ui_settings`](struct.Runtime.html#method.ui_settings "method pert_ui::Runtime::ui_settings"),
  [`fonts`](struct.Runtime.html#method.fonts "method pert_ui::Runtime::fonts"),
  [`icons`](struct.Runtime.html#method.icons "method pert_ui::Runtime::icons").
- Parameters:
  [`parameter`](struct.Runtime.html#method.parameter "method pert_ui::Runtime::parameter"),
  [`parameter_any`](struct.Runtime.html#method.parameter_any "method pert_ui::Runtime::parameter_any"),
  [`remote_parameter`](struct.Runtime.html#method.remote_parameter "method pert_ui::Runtime::remote_parameter").
- Work and timing:
  [`spawn`](struct.Runtime.html#method.spawn "method pert_ui::Runtime::spawn"),
  [`mailbox`](struct.Runtime.html#method.mailbox "method pert_ui::Runtime::mailbox"),
  [`defer`](struct.Runtime.html#method.defer "method pert_ui::Runtime::defer"),
  [`start_interval`](struct.Runtime.html#method.start_interval "method pert_ui::Runtime::start_interval"),
  [`series_source`](struct.Runtime.html#method.series_source "method pert_ui::Runtime::series_source"),
  [`live_waker`](struct.Runtime.html#method.live_waker "method pert_ui::Runtime::live_waker").
- Commands and keys:
  [`register_command`](struct.Runtime.html#method.register_command "method pert_ui::Runtime::register_command"),
  [`unregister_command`](struct.Runtime.html#method.unregister_command "method pert_ui::Runtime::unregister_command"),
  [`rebind_command`](struct.Runtime.html#method.rebind_command "method pert_ui::Runtime::rebind_command"),
  [`command_enabled`](struct.Runtime.html#method.command_enabled "method pert_ui::Runtime::command_enabled"),
  [`command_enabled_signal`](struct.Runtime.html#method.command_enabled_signal "method pert_ui::Runtime::command_enabled_signal"),
  [`command_enabled_binding`](struct.Runtime.html#method.command_enabled_binding "method pert_ui::Runtime::command_enabled_binding"),
  [`command_in_modals`](struct.Runtime.html#method.command_in_modals "method pert_ui::Runtime::command_in_modals"),
  [`command_repeats`](struct.Runtime.html#method.command_repeats "method pert_ui::Runtime::command_repeats"),
  [`command_name`](struct.Runtime.html#method.command_name "method pert_ui::Runtime::command_name"),
  [`command_shortcuts`](struct.Runtime.html#method.command_shortcuts "method pert_ui::Runtime::command_shortcuts"),
  [`command_info`](struct.Runtime.html#method.command_info "method pert_ui::Runtime::command_info"),
  [`commands_for`](struct.Runtime.html#method.commands_for "method pert_ui::Runtime::commands_for"),
  [`command_context`](struct.Runtime.html#method.command_context "method pert_ui::Runtime::command_context"),
  [`set_command_target`](struct.Runtime.html#method.set_command_target "method pert_ui::Runtime::set_command_target"),
  [`invoke_command`](struct.Runtime.html#method.invoke_command "method pert_ui::Runtime::invoke_command"),
  [`register_held_input`](struct.Runtime.html#method.register_held_input "method pert_ui::Runtime::register_held_input"),
  [`unregister_held_input`](struct.Runtime.html#method.unregister_held_input "method pert_ui::Runtime::unregister_held_input").
- Focus, reveal and search:
  [`focus`](struct.Runtime.html#method.focus "method pert_ui::Runtime::focus"),
  [`focus_next`](struct.Runtime.html#method.focus_next "method pert_ui::Runtime::focus_next"),
  [`focused`](struct.Runtime.html#method.focused "method pert_ui::Runtime::focused"),
  [`can_focus`](struct.Runtime.html#method.can_focus "method pert_ui::Runtime::can_focus"),
  [`reveal`](struct.Runtime.html#method.reveal "method pert_ui::Runtime::reveal"),
  [`reveal_and_focus`](struct.Runtime.html#method.reveal_and_focus "method pert_ui::Runtime::reveal_and_focus"),
  [`reveal_child`](struct.Runtime.html#method.reveal_child "method pert_ui::Runtime::reveal_child"),
  [`content_search`](struct.Runtime.html#method.content_search "method pert_ui::Runtime::content_search"),
  [`jump_to_content`](struct.Runtime.html#method.jump_to_content "method pert_ui::Runtime::jump_to_content"),
  [`resume_content_jump`](struct.Runtime.html#method.resume_content_jump "method pert_ui::Runtime::resume_content_jump").
- Windows:
  [`request_detach`](struct.Runtime.html#method.request_detach "method pert_ui::Runtime::request_detach"),
  [`request_redock`](struct.Runtime.html#method.request_redock "method pert_ui::Runtime::request_redock"),
  [`request_close`](struct.Runtime.html#method.request_close "method pert_ui::Runtime::request_close"),
  [`request_window_title`](struct.Runtime.html#method.request_window_title "method pert_ui::Runtime::request_window_title"),
  [`window_of`](struct.Runtime.html#method.window_of "method pert_ui::Runtime::window_of"),
  [`native_windows`](struct.Runtime.html#method.native_windows "method pert_ui::Runtime::native_windows"),
  [`primary_window`](struct.Runtime.html#method.primary_window "method pert_ui::Runtime::primary_window"),
  [`detached_view`](struct.Runtime.html#method.detached_view "method pert_ui::Runtime::detached_view"),
  [`contains_window`](struct.Runtime.html#method.contains_window "method pert_ui::Runtime::contains_window").
- Workspace:
  [`capture_workspace`](struct.Runtime.html#method.capture_workspace "method pert_ui::Runtime::capture_workspace"),
  [`restore_workspace`](struct.Runtime.html#method.restore_workspace "method pert_ui::Runtime::restore_workspace"),
  [`workspace_report`](struct.Runtime.html#method.workspace_report "method pert_ui::Runtime::workspace_report").
- Clipboard and text:
  [`clipboard_text`](struct.Runtime.html#method.clipboard_text "method pert_ui::Runtime::clipboard_text"),
  [`set_clipboard_text`](struct.Runtime.html#method.set_clipboard_text "method pert_ui::Runtime::set_clipboard_text"),
  [`text_command`](struct.Runtime.html#method.text_command "method pert_ui::Runtime::text_command").
- Lookups:
  [`find`](struct.Runtime.html#method.find "method pert_ui::Runtime::find"),
  [`find_descendant`](struct.Runtime.html#method.find_descendant "method pert_ui::Runtime::find_descendant"),
  [`children`](struct.Runtime.html#method.children "method pert_ui::Runtime::children"),
  [`key`](struct.Runtime.html#method.key "method pert_ui::Runtime::key").
- Failures and shutdown:
  [`last_failure`](struct.Runtime.html#method.last_failure "method pert_ui::Runtime::last_failure"),
  [`shutdown`](struct.Runtime.html#method.shutdown "method pert_ui::Runtime::shutdown").

**Tests and inspection.** What tests and tools add; a test also calls
the application methods.

- Driving:
  [`flush`](struct.Runtime.html#method.flush "method pert_ui::Runtime::flush"),
  [`now`](struct.Runtime.html#method.now "method pert_ui::Runtime::now"),
  [`next_deadline`](struct.Runtime.html#method.next_deadline "method pert_ui::Runtime::next_deadline"),
  [`event`](struct.Runtime.html#method.event "method pert_ui::Runtime::event"),
  [`try_event`](struct.Runtime.html#method.try_event "method pert_ui::Runtime::try_event"),
  [`key_input`](struct.Runtime.html#method.key_input "method pert_ui::Runtime::key_input"),
  [`text_input`](struct.Runtime.html#method.text_input "method pert_ui::Runtime::text_input"),
  [`pointer_input`](struct.Runtime.html#method.pointer_input "method pert_ui::Runtime::pointer_input").
- Standing in for a host, which a host calls as well:
  [`set_viewport`](struct.Runtime.html#method.set_viewport "method pert_ui::Runtime::set_viewport"),
  [`viewport`](struct.Runtime.html#method.viewport "method pert_ui::Runtime::viewport"),
  [`command_key_input`](struct.Runtime.html#method.command_key_input "method pert_ui::Runtime::command_key_input"),
  [`native_modifiers`](struct.Runtime.html#method.native_modifiers "method pert_ui::Runtime::native_modifiers"),
  [`keyboard`](struct.Runtime.html#method.keyboard "method pert_ui::Runtime::keyboard"),
  [`paint`](struct.Runtime.html#method.paint "method pert_ui::Runtime::paint"),
  [`set_capture_support`](struct.Runtime.html#method.set_capture_support "method pert_ui::Runtime::set_capture_support").
- Inspection:
  [`inspect`](struct.Runtime.html#method.inspect "method pert_ui::Runtime::inspect"),
  [`inspect_with`](struct.Runtime.html#method.inspect_with "method pert_ui::Runtime::inspect_with"),
  [`query`](struct.Runtime.html#method.query "method pert_ui::Runtime::query"),
  [`query_with`](struct.Runtime.html#method.query_with "method pert_ui::Runtime::query_with"),
  [`inspection_id`](struct.Runtime.html#method.inspection_id "method pert_ui::Runtime::inspection_id"),
  [`inspection_service`](struct.Runtime.html#method.inspection_service "method pert_ui::Runtime::inspection_service"),
  [`validation_report`](struct.Runtime.html#method.validation_report "method pert_ui::Runtime::validation_report"),
  [`bounds`](struct.Runtime.html#method.bounds "method pert_ui::Runtime::bounds"),
  [`drag_active`](struct.Runtime.html#method.drag_active "method pert_ui::Runtime::drag_active"),
  [`read_signal`](struct.Runtime.html#method.read_signal "method pert_ui::Runtime::read_signal").
- Diagnostics and capture:
  [`set_debug_mode`](struct.Runtime.html#method.set_debug_mode "method pert_ui::Runtime::set_debug_mode"),
  [`clear_event_history`](struct.Runtime.html#method.clear_event_history "method pert_ui::Runtime::clear_event_history"),
  [`set_debug_overlay`](struct.Runtime.html#method.set_debug_overlay "method pert_ui::Runtime::set_debug_overlay"),
  [`debug_overlay`](struct.Runtime.html#method.debug_overlay "method pert_ui::Runtime::debug_overlay"),
  [`set_profiling`](struct.Runtime.html#method.set_profiling "method pert_ui::Runtime::set_profiling"),
  [`profile`](struct.Runtime.html#method.profile "method pert_ui::Runtime::profile"),
  [`clear_profile`](struct.Runtime.html#method.clear_profile "method pert_ui::Runtime::clear_profile"),
  [`request_capture`](struct.Runtime.html#method.request_capture "method pert_ui::Runtime::request_capture").

**Hosts.** A host that drives the runtime in place of `App` imports
[`host::HostIntegration`](host/trait.HostIntegration.html "trait pert_ui::host::HostIntegration"),
whose page lists the methods only a host calls, by duty. It calls the
methods above as well, the “Standing in for a host” group in particular;
a headless test that stands in for a host reaches the trait through
[`testing`](../pert_ui_runtime/testing/index.html "mod pert_ui_runtime::testing").

## Implementations[§](#implementations)

[Source](../src/pert_ui_runtime/parameter_binding/erased.rs.html#5)[§](#impl-Runtime)

### impl [Runtime](struct.Runtime.html "struct pert_ui::Runtime")

[Source](../src/pert_ui_runtime/parameter_binding/erased.rs.html#15-19)

#### pub fn [parameter_any](#method.parameter_any)( &mut self, id: [ParameterId](struct.ParameterId.html "struct pert_ui::ParameterId"), spec: [AnySpec](enum.AnySpec.html "enum pert_ui::AnySpec"), ) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<([AnyParameter](enum.AnyParameter.html "enum pert_ui::AnyParameter"), [AnyParameterHost](enum.AnyParameterHost.html "enum pert_ui::AnyParameterHost")), [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\>

Creates a parameter from a family-tagged specification, returning tagged
handles of the same family.

Use it for descriptor-driven collections holding several families. Typed
and erased handles share identical validation, notification and failure
behavior; match the variants for the typed handles.

##### [§](#errors)Errors

Same as
[`Self::parameter`](struct.Runtime.html#method.parameter "method pert_ui::Runtime::parameter").

[Source](../src/pert_ui_runtime/parameter_binding/remote.rs.html#715)[§](#impl-Runtime-1)

### impl [Runtime](struct.Runtime.html "struct pert_ui::Runtime")

[Source](../src/pert_ui_runtime/parameter_binding/remote.rs.html#723-727)

#### pub fn [remote_parameter](#method.remote_parameter)\<T\>( &mut self, id: [ParameterId](struct.ParameterId.html "struct pert_ui::ParameterId"), spec: [ParameterSpec](struct.ParameterSpec.html "struct pert_ui::ParameterSpec")\<T\>, ) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<([Parameter](struct.Parameter.html "struct pert_ui::Parameter")\<T\>, [RemoteHost](struct.RemoteHost.html "struct pert_ui::RemoteHost")\<T\>), [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\>

where T:
[ParameterValue](trait.ParameterValue.html "trait pert_ui::ParameterValue"),

Creates a parameter edited by ordinary controls but committed only after
[`RemoteHost::respond`](struct.RemoteHost.html#method.respond "method pert_ui::RemoteHost::respond").
The initial authoritative base is the specification default; use
`resynchronize` before exposing another initial value.

##### [§](#errors-1)Errors

The errors of
[`Runtime::parameter`](struct.Runtime.html#method.parameter "method pert_ui::Runtime::parameter"):
a runtime phase error or
[`BindingError::DuplicateParameter`](enum.BindingError.html#variant.DuplicateParameter "variant pert_ui::BindingError::DuplicateParameter").

[Source](../src/pert_ui_runtime/parameter_binding.rs.html#976)[§](#impl-Runtime-2)

### impl [Runtime](struct.Runtime.html "struct pert_ui::Runtime")

[Source](../src/pert_ui_runtime/parameter_binding.rs.html#1005-1009)

#### pub fn [parameter](#method.parameter)\<T\>( &mut self, id: [ParameterId](struct.ParameterId.html "struct pert_ui::ParameterId"), spec: [ParameterSpec](struct.ParameterSpec.html "struct pert_ui::ParameterSpec")\<T\>, ) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<([Parameter](struct.Parameter.html "struct pert_ui::Parameter")\<T\>, [ParameterHost](struct.ParameterHost.html "struct pert_ui::ParameterHost")\<T\>), [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\>

where T:
[ParameterValue](trait.ParameterValue.html "trait pert_ui::ParameterValue"),

Creates one synchronous local parameter of family `T` and its
authoritative owner.

The parameter starts at the specification’s default and revision zero.
Controls clone the returned binding. Retain the host owner for as long
as the parameter exists. Registered identities, including removed ones,
remain reserved until runtime drop; recreate using `next_incarnation`,
not the old ID. Registration prunes dead inspection references in a
linear pass over the retained registry. Inspection also prunes; neither
operation schedules work.

``` rust
use pert_ui::{Element, ParameterId, ParameterSpec, Runtime, Slider};
let mut ui = Runtime::new();
let (amount, host) = ui.parameter(
    ParameterId::new(1, 1, "amount")?,
    ParameterSpec::new("Amount", 0.0, 1.0, 0.5)?,
)?;
ui.mount_tree(Element::new("amount", Slider::new(amount.clone())))?;
host.write_base(0.75)?; // an authoritative application update
assert_eq!(amount.snapshot().get()?.base, 0.75);
```

##### [§](#errors-2)Errors

Returns a runtime phase error (see
[`Parameter`](struct.Parameter.html "struct pert_ui::Parameter")), or
[`BindingError::DuplicateParameter`](enum.BindingError.html#variant.DuplicateParameter "variant pert_ui::BindingError::DuplicateParameter")
for a complete incarnation this runtime registered before, even one
since removed.

[Source](../src/pert_ui_runtime/runtime/application.rs.html#100)[§](#impl-Runtime-3)

### impl [Runtime](struct.Runtime.html "struct pert_ui::Runtime")

[Source](../src/pert_ui_runtime/runtime/application.rs.html#103)

#### pub const [MAX_BACKGROUND_TASKS](#associatedconstant.MAX_BACKGROUND_TASKS): [usize](https://doc.rust-lang.org/1.99.1/std/primitive.usize.html) = 16

The most background tasks that can wait for their results at once;
[`Self::spawn`](struct.Runtime.html#method.spawn "method pert_ui::Runtime::spawn")
refuses more.

[Source](../src/pert_ui_runtime/runtime/application.rs.html#106)

#### pub const [MAX_INTERVALS](#associatedconstant.MAX_INTERVALS): [usize](https://doc.rust-lang.org/1.99.1/std/primitive.usize.html) = 64

The most interval timers that can run at once;
[`Self::start_interval`](struct.Runtime.html#method.start_interval "method pert_ui::Runtime::start_interval")
refuses more.

[Source](../src/pert_ui_runtime/runtime/application.rs.html#140)

#### pub fn [keep_alive](#method.keep_alive)\<T\>(&mut self, value: T)

where T: 'static,

Keeps `value` alive until the runtime shuts down or is dropped.

This is the owner for values that code with only a `Runtime` creates
once and must not lose, such as a
[`Mailbox`](struct.Mailbox.html "struct pert_ui::Mailbox"), a
[`ParameterHost`](struct.ParameterHost.html "struct pert_ui::ParameterHost")
or a controller made by a [custom
host](../pert_ui_runtime/host/index.html "mod pert_ui_runtime::host"), a
helper that builds part of an application, or deferred work, where a
local value would be dropped when that code returns and, for a mailbox,
close it. An
[`App`](https://docs.rs/pert-ui/latest/pert_ui/struct.App.html)
initializer returns such values in its [application
state](https://docs.rs/pert-ui/latest/pert_ui/struct.App.html#application-state)
instead, which `App` keeps until the application ends.

[`Self::shutdown`](struct.Runtime.html#method.shutdown "method pert_ui::Runtime::shutdown")
drops kept values last, after components have unmounted and parameters
have ended their edits, in the reverse order of keeping, so a value kept
later, which may refer to one kept earlier, goes first. After shutdown a
value is dropped at once.

``` rust
use pert_ui::{Capacity, Runtime};
let mut ui = Runtime::new();
let saved = ui.signal(false);
let done = saved.clone();
let mailbox = ui.mailbox(Capacity::new(4)?, move |ok: bool| done.set(ok))?;
let sender = mailbox.sender();
ui.keep_alive(mailbox);
std::thread::spawn(move || sender.send(true)).join().unwrap()?;
ui.flush(std::time::Duration::ZERO)?;
assert!(saved.get()?);
```

[Source](../src/pert_ui_runtime/runtime/application.rs.html#210-214)

#### pub fn [spawn](#method.spawn)\<T\>( &mut self, work: impl [FnOnce](https://doc.rust-lang.org/1.99.1/core/ops/function/trait.FnOnce.html "trait core::ops::function::FnOnce")() -\> T + [Send](https://doc.rust-lang.org/1.99.1/core/marker/trait.Send.html "trait core::marker::Send") + 'static, deliver: impl [FnOnce](https://doc.rust-lang.org/1.99.1/core/ops/function/trait.FnOnce.html "trait core::ops::function::FnOnce")(T) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[()](https://doc.rust-lang.org/1.99.1/std/primitive.unit.html), [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\> + 'static, ) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[BackgroundTask](struct.BackgroundTask.html "struct pert_ui::BackgroundTask"), [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\>

where T:
[Send](https://doc.rust-lang.org/1.99.1/core/marker/trait.Send.html "trait core::marker::Send") +
'static,

Runs `work` on a new background thread and hands its result to `deliver`
on the UI thread, in the first flush after it arrives.

Timing class of `deliver`: [at the next
flush](https://github.com/pertyjons/pert-ui/blob/main/docs/contracts.md#at-the-next-flush).

This is for occasional work that must not block the interface, such as
reading or writing a file: each call starts one thread, and at most
[`Self::MAX_BACKGROUND_TASKS`](struct.Runtime.html#associatedconstant.MAX_BACKGROUND_TASKS "associated constant pert_ui::Runtime::MAX_BACKGROUND_TASKS")
tasks wait for their results at once. `work` receives no UI handles and
must not need any; return what the interface needs and let `deliver`
write it to signals. A failure that the application should show belongs
in the result, for example as a `Result` that `deliver` presents.
Arriving results wake the host and are delivered in arrival order, with
the runtime’s mailboxes; `deliver` may write bindings, and its error is
returned from that flush. Nothing here is safe for time-critical
producers, which use the live-data endpoints instead.

A component’s or control’s callback receives an
[`EventContext`](struct.EventContext.html "struct pert_ui::EventContext"),
not the runtime: it starts the work with
[`EventContext::spawn`](struct.EventContext.html#method.spawn "method pert_ui::EventContext::spawn"),
which queues this call for the start of the next flush. A callback that
must be able to cancel the delivery queues this call itself with
[`EventContext::defer`](struct.EventContext.html#method.defer "method pert_ui::EventContext::defer")
and keeps the returned handle.

The returned handle can
[`cancel`](struct.BackgroundTask.html#method.cancel "method pert_ui::BackgroundTask::cancel")
the delivery; dropping it does not. If `work` panics, nothing is
delivered and the flush that receives the outcome returns
[`RuntimeError::BackgroundPanicked`](enum.RuntimeError.html#variant.BackgroundPanicked "variant pert_ui::RuntimeError::BackgroundPanicked").
After
[`Self::shutdown`](struct.Runtime.html#method.shutdown "method pert_ui::Runtime::shutdown")
a result is dropped on its thread and never delivered; shutdown does not
wait for running work.

``` rust
use pert_ui::Runtime;
use std::time::Duration;
let mut ui = Runtime::new();
let status = ui.signal("Reading".to_owned());
let shown = status.clone();
// Real work would read or write a file; the result carries its error.
ui.spawn(
    || "42".parse::<u32>(),
    move |result| shown.set(match result {
        Ok(value) => format!("Read {value}"),
        Err(error) => format!("Reading failed: {error}"),
    }),
)?;
// A native host flushes when the result wakes it; a headless one polls.
while status.get()? == "Reading" {
    std::thread::sleep(Duration::from_millis(1));
    ui.flush(Duration::ZERO)?;
}
assert_eq!(status.get()?, "Read 42");
```

##### [§](#errors-3)Errors

`Closed` after shutdown and the phase errors of a write from inside
update, layout, paint, inspection or delivery; `BackgroundFull` while
[`Self::MAX_BACKGROUND_TASKS`](struct.Runtime.html#associatedconstant.MAX_BACKGROUND_TASKS "associated constant pert_ui::Runtime::MAX_BACKGROUND_TASKS")
tasks wait; `BackgroundSpawn` with the operating system’s reason when
the thread cannot be started. A refused call starts nothing.

[Source](../src/pert_ui_runtime/runtime/application.rs.html#320-324)

#### pub fn [start_interval](#method.start_interval)( &mut self, period: [Duration](https://doc.rust-lang.org/1.99.1/core/time/struct.Duration.html "struct core::time::Duration"), tick: impl [FnMut](https://doc.rust-lang.org/1.99.1/core/ops/function/trait.FnMut.html "trait core::ops::function::FnMut")(&mut [Runtime](struct.Runtime.html "struct pert_ui::Runtime")) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[()](https://doc.rust-lang.org/1.99.1/std/primitive.unit.html), [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\> + 'static, ) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[IntervalTimer](struct.IntervalTimer.html "struct pert_ui::IntervalTimer"), [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\>

Calls `tick` every `period` until it is cancelled or the runtime shuts
down, for application work on a schedule, such as saving a draft.

Timing class: [at the next
flush](https://github.com/pertyjons/pert-ui/blob/main/docs/contracts.md#at-the-next-flush).

The timer belongs to the application, not to a component or window:
focus changes, input cancellation and hidden windows do not stop it.
`tick` runs on the UI thread with runtime access, before component
updates, in a flush at or after its due time; the native host also runs
a due timer while a window’s redraw is withheld, for example while it is
minimized. The first tick is due one `period` after the call. A tick
that runs late does not catch up: the next one is due one `period` after
the late one ran, if its scheduled time has passed. A failing tick keeps
the timer running; its error is returned from that flush, or from
[`HostIntegration::service_intervals`](host/trait.HostIntegration.html#tymethod.service_intervals "method pert_ui::host::HostIntegration::service_intervals").
A running timer keeps the application’s next deadline set, so use it
only for work that needs it.

``` rust
use pert_ui::Runtime;
use std::time::Duration;
let mut ui = Runtime::new();
let saves = ui.signal(0);
let counter = saves.clone();
let timer = ui.start_interval(Duration::from_secs(30), move |_| counter.update(|n| *n += 1))?;
ui.flush(Duration::from_secs(29))?;
assert_eq!(saves.get()?, 0);
ui.flush(Duration::from_secs(30))?;
assert_eq!(saves.get()?, 1);
timer.cancel();
ui.flush(Duration::from_secs(90))?;
assert_eq!(saves.get()?, 1);
```

##### [§](#errors-4)Errors

`ZeroDelay` for a zero period; `Exhausted` when the first due time
overflows; `IntervalsFull` while
[`Self::MAX_INTERVALS`](struct.Runtime.html#associatedconstant.MAX_INTERVALS "associated constant pert_ui::Runtime::MAX_INTERVALS")
timers run; `Closed` after shutdown and the phase errors of a write from
inside update, layout, paint, inspection or delivery.

[Source](../src/pert_ui_runtime/runtime/application.rs.html#477)

#### pub fn [last_failure](#method.last_failure)(&self) -\> [Option](https://doc.rust-lang.org/1.99.1/core/option/enum.Option.html "enum core::option::Option")\<[FailureSite](struct.FailureSite.html "struct pert_ui::FailureSite")\>

Where the latest failing flush or paint first failed in a component: the
key path to it and what failed, a callback or the reconciliation of the
children it requested. A developer report, such as the native
application’s default error report, names it beside the error.

Each outermost
[`Self::flush`](struct.Runtime.html#method.flush "method pert_ui::Runtime::flush"),
[`Self::paint`](struct.Runtime.html#method.paint "method pert_ui::Runtime::paint"),
[`HostIntegration::flush_windows`](host/trait.HostIntegration.html#tymethod.flush_windows "method pert_ui::host::HostIntegration::flush_windows")
and
[`HostIntegration::service_intervals`](host/trait.HostIntegration.html#tymethod.service_intervals "method pert_ui::host::HostIntegration::service_intervals")
starts without a site once it is admitted, so `None` means the latest
one succeeded or failed outside any component, for example in a deferred
task. A call refused at entry (a time before the runtime’s current one,
a forbidden phase, after shutdown) returns that error and leaves an
earlier site in place; nested calls never reset it. A failure inside a
component’s child is reported at the child, where it arose, and the
first site of an operation is kept.

Children a component requested through
[`crate::UpdateContext::request_children`](struct.UpdateContext.html#method.request_children "method pert_ui::UpdateContext::request_children"),
as [`crate::Dynamic`](struct.Dynamic.html "struct pert_ui::Dynamic"),
[`crate::Show`](struct.Show.html "struct pert_ui::Show") and
[`crate::Each`](struct.Each.html "struct pert_ui::Each") do, are
reconciled during the flush. When they are rejected, the site is
[`crate::FailurePhase::Reconcile`](enum.FailurePhase.html#variant.Reconcile "variant pert_ui::FailurePhase::Reconcile")
at that component, with the rejected description’s keys in
[`crate::FailureSite::description`](struct.FailureSite.html#structfield.description "field pert_ui::FailureSite::description").
Calling
[`Self::reconcile_children`](struct.Runtime.html#method.reconcile_children "method pert_ui::Runtime::reconcile_children")
or
[`Self::mount_tree`](struct.Runtime.html#method.mount_tree "method pert_ui::Runtime::mount_tree")
directly returns the error to the caller and records no site; it leaves
an earlier one in place.

``` rust
use pert_ui::{Column, Component, FailurePhase, IntoElement, LayoutError,
    Runtime, RuntimeError, UpdateContext};
struct Broken;
impl Component for Broken {
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Err(LayoutError::InvalidChild.into())
    }
}
let mut ui = Runtime::new();
ui.mount_tree(Column::new([Broken.key("broken")]).key("form"))?;
assert!(ui.flush(std::time::Duration::ZERO).is_err());
let site = ui.last_failure().ok_or("a component failed")?;
assert_eq!(site.path, ["form", "broken"]);
assert_eq!(site.phase, FailurePhase::Update);
assert_eq!(site.to_string(), "update of form/broken");
```

[Source](../src/pert_ui_runtime/runtime/capture.rs.html#29)[§](#impl-Runtime-4)

### impl [Runtime](struct.Runtime.html "struct pert_ui::Runtime")

[Source](../src/pert_ui_runtime/runtime/capture.rs.html#76)

#### pub const [MAX_PENDING_CAPTURES](#associatedconstant.MAX_PENDING_CAPTURES): [usize](https://doc.rust-lang.org/1.99.1/std/primitive.usize.html) = 16

The most captures that may wait for the host at once; a further request
is refused as
[`CaptureError::Busy`](enum.CaptureError.html#variant.Busy "variant pert_ui::CaptureError::Busy").
Bounds the frames a host is asked to render and the images held.

[Source](../src/pert_ui_runtime/runtime/capture.rs.html#39)

#### pub fn [set_capture_support](#method.set_capture_support)(&self, supported: [bool](https://doc.rust-lang.org/1.99.1/std/primitive.bool.html))

Declares whether this host renders capture jobs.

Off by default, so a headless runtime answers every
[`CaptureRequest`](struct.CaptureRequest.html "struct pert_ui::CaptureRequest")
with
[`CaptureError::Unsupported`](enum.CaptureError.html#variant.Unsupported "variant pert_ui::CaptureError::Unsupported")
at once. The native `App` turns it on; a custom host that turns it on
commits to completing every job
[`pending_captures`](host/trait.HostIntegration.html#tymethod.pending_captures "method pert_ui::host::HostIntegration::pending_captures")
lists, with a frame or with an error.

[Source](../src/pert_ui_runtime/runtime/capture.rs.html#64-67)

#### pub fn [request_capture](#method.request_capture)( &self, request: [CaptureRequest](struct.CaptureRequest.html "struct pert_ui::CaptureRequest"), ) -\> [Pending](struct.Pending.html "struct pert_ui::Pending")\<[Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[Capture](struct.Capture.html "struct pert_ui::Capture"), [CaptureError](enum.CaptureError.html "enum pert_ui::CaptureError")\>\>

Asks for a screenshot from the UI thread; the reply arrives once the
host has rendered the frame, or at once with the error that refused the
request. The frame is rendered between this thread’s flushes, so
[`Pending::wait`](struct.Pending.html#method.wait "method pert_ui::Pending::wait")
here would block forever before the host has served the request: take
the reply with
[`Pending::try_reply`](struct.Pending.html#method.try_reply "method pert_ui::Pending::try_reply")
once it has. From another thread, use
[`InspectionClient::capture`](struct.InspectionClient.html#method.capture "method pert_ui::InspectionClient::capture").

Admission checks that a host renders captures and that the target is
mounted, visible and measured now; the geometry is taken again from the
state the rendered frame shows, and a target that disappeared by then is
reported rather than replaced. An admitted request marks the target’s
window for one repaint, which is the frame the host renders the capture
from; nothing polls.

[Source](../src/pert_ui_runtime/runtime/commands.rs.html#571)[§](#impl-Runtime-5)

### impl [Runtime](struct.Runtime.html "struct pert_ui::Runtime")

[Source](../src/pert_ui_runtime/runtime/commands.rs.html#607-613)

#### pub fn [register_command](#method.register_command)( &mut self, name: impl [Into](https://doc.rust-lang.org/1.99.1/core/convert/trait.Into.html "trait core::convert::Into")\<[String](https://doc.rust-lang.org/1.99.1/alloc/string/struct.String.html "struct alloc::string::String")\>, scope: [CommandScope](enum.CommandScope.html "enum pert_ui::CommandScope"), chords: [Vec](https://doc.rust-lang.org/1.99.1/alloc/vec/struct.Vec.html "struct alloc::vec::Vec")\<[KeyChord](struct.KeyChord.html "struct pert_ui::KeyChord")\>, action: impl [FnMut](https://doc.rust-lang.org/1.99.1/core/ops/function/trait.FnMut.html "trait core::ops::function::FnMut")(&mut [Runtime](struct.Runtime.html "struct pert_ui::Runtime"), &[CommandContext](struct.CommandContext.html "struct pert_ui::CommandContext")) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[()](https://doc.rust-lang.org/1.99.1/std/primitive.unit.html), [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\> + 'static, ) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[CommandId](struct.CommandId.html "struct pert_ui::CommandId"), [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\>

Registers a named host action and its chord aliases in one scope.

Lifetime is explicit: unregister it or remove its view/window scope to
release callbacks and bindings. Dropping the returned
[`CommandId`](struct.CommandId.html "struct pert_ui::CommandId") keeps
the command registered. Duplicate chords in the same scope, including
alias/fallback collisions, return `ShortcutConflict` in debug and
release. An empty chord list supports menu/button-only commands. Repeats
default off.

``` rust
use pert_ui::{Runtime, CommandScope, CommandSource, KeyChord, CommandOutcome};
let mut ui = Runtime::new();
let playing = ui.signal(false);
let state = playing.clone();
let play = ui.register_command("Play", CommandScope::Application,
    vec![KeyChord::primary('p')], move |_, _| {
        state.set(!state.get()?)?;
        Ok(())
    })?;
let context = ui.command_context(CommandSource::Programmatic);
assert_eq!(ui.invoke_command(&play, context)?, CommandOutcome::Invoked);
assert!(playing.get()?);
ui.unregister_command(&play)?;
```

##### [§](#errors-5)Errors

Returns
[`RuntimeError::Closed`](enum.RuntimeError.html#variant.Closed "variant pert_ui::RuntimeError::Closed")
after shutdown, and `WriteDuringUpdate`, `WriteDuringLayout`,
`WriteDuringPaint`, `WriteDuringInspection` or `WriteDuringDelivery`
when called from inside that phase; `EmptyCommandName` for a blank name;
`ForeignRuntime` or `Unmounted` for a view scope whose view is foreign
or unmounted, and `Unmounted` for a window scope that is no longer a
presentation root; `ShortcutConflict` when two chords overlap each other
or a chord of another command in the same scope; and `Exhausted` when
command identities run out.

[Source](../src/pert_ui_runtime/runtime/commands.rs.html#693)

#### pub fn [unregister_command](#method.unregister_command)(&mut self, id: &[CommandId](struct.CommandId.html "struct pert_ui::CommandId")) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[()](https://doc.rust-lang.org/1.99.1/std/primitive.unit.html), [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\>

Removes one exact registration, releasing its action and chord aliases.

##### [§](#errors-6)Errors

Returns
[`RuntimeError::Closed`](enum.RuntimeError.html#variant.Closed "variant pert_ui::RuntimeError::Closed")
after shutdown, and `WriteDuringUpdate`, `WriteDuringLayout`,
`WriteDuringPaint`, `WriteDuringInspection` or `WriteDuringDelivery`
when called from inside that phase; and `StaleCommand` for a command
that is unregistered or belongs to another runtime.

[Source](../src/pert_ui_runtime/runtime/commands.rs.html#708-712)

#### pub fn [rebind_command](#method.rebind_command)( &mut self, id: &[CommandId](struct.CommandId.html "struct pert_ui::CommandId"), chords: [Vec](https://doc.rust-lang.org/1.99.1/alloc/vec/struct.Vec.html "struct alloc::vec::Vec")\<[KeyChord](struct.KeyChord.html "struct pert_ui::KeyChord")\>, ) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[()](https://doc.rust-lang.org/1.99.1/std/primitive.unit.html), [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\>

Atomically replaces aliases after conflict validation, retaining the
command.

##### [§](#errors-7)Errors

Returns
[`RuntimeError::Closed`](enum.RuntimeError.html#variant.Closed "variant pert_ui::RuntimeError::Closed")
after shutdown, and `WriteDuringUpdate`, `WriteDuringLayout`,
`WriteDuringPaint`, `WriteDuringInspection` or `WriteDuringDelivery`
when called from inside that phase; `StaleCommand` for a command that is
unregistered or belongs to another runtime; and `ShortcutConflict` when
two chords overlap each other or a chord of another command in the same
scope. The old aliases stay on error.

[Source](../src/pert_ui_runtime/runtime/commands.rs.html#728)

#### pub fn [command_name](#method.command_name)(&self, id: &[CommandId](struct.CommandId.html "struct pert_ui::CommandId")) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<&[str](https://doc.rust-lang.org/1.99.1/std/primitive.str.html), [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\>

The host-supplied name of a registered command, for presenting it in a
menu, a button or a shortcut list.

##### [§](#errors-8)Errors

`StaleCommand` if the command was unregistered.

[Source](../src/pert_ui_runtime/runtime/commands.rs.html#739-742)

#### pub fn [command_shortcuts](#method.command_shortcuts)( &self, id: &[CommandId](struct.CommandId.html "struct pert_ui::CommandId"), ) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[ReadSignal](struct.ReadSignal.html "struct pert_ui::ReadSignal")\<[Vec](https://doc.rust-lang.org/1.99.1/alloc/vec/struct.Vec.html "struct alloc::vec::Vec")\<[KeyChord](struct.KeyChord.html "struct pert_ui::KeyChord")\>\>, [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\>

The command’s current chord aliases as a binding, first alias first.

The binding follows
[`Self::rebind_command`](struct.Runtime.html#method.rebind_command "method pert_ui::Runtime::rebind_command"),
so a menu row or a shortcut list that reads it shows a changed shortcut
without being rebuilt. After the command is unregistered the binding
keeps its last value.

##### [§](#errors-9)Errors

`StaleCommand` if the command was unregistered.

[Source](../src/pert_ui_runtime/runtime/commands.rs.html#754-757)

#### pub fn [command_enabled_binding](#method.command_enabled_binding)( &self, id: &[CommandId](struct.CommandId.html "struct pert_ui::CommandId"), ) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[Option](https://doc.rust-lang.org/1.99.1/core/option/enum.Option.html "enum core::option::Option")\<[ReadSignal](struct.ReadSignal.html "struct pert_ui::ReadSignal")\<[bool](https://doc.rust-lang.org/1.99.1/std/primitive.bool.html)\>\>, [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\>

The simple enabled binding installed by
[`Self::command_enabled_signal`](struct.Runtime.html#method.command_enabled_signal "method pert_ui::Runtime::command_enabled_signal"),
so a control presenting the command can gray out with it.

`None` means the command is always enabled or uses a contextual
predicate from
[`Self::command_enabled`](struct.Runtime.html#method.command_enabled "method pert_ui::Runtime::command_enabled"),
whose answer depends on the target and is known only per invocation or
through
[`Self::command_info`](struct.Runtime.html#method.command_info "method pert_ui::Runtime::command_info").

##### [§](#errors-10)Errors

`StaleCommand` if the command was unregistered.

[Source](../src/pert_ui_runtime/runtime/commands.rs.html#772-776)

#### pub fn [command_enabled](#method.command_enabled)( &mut self, id: &[CommandId](struct.CommandId.html "struct pert_ui::CommandId"), enabled: impl [Fn](https://doc.rust-lang.org/1.99.1/core/ops/function/trait.Fn.html "trait core::ops::function::Fn")(&[Runtime](struct.Runtime.html "struct pert_ui::Runtime"), &[CommandContext](struct.CommandContext.html "struct pert_ui::CommandContext")) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[bool](https://doc.rust-lang.org/1.99.1/std/primitive.bool.html), [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\> + 'static, ) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[()](https://doc.rust-lang.org/1.99.1/std/primitive.unit.html), [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\>

Installs a pure UI-thread contextual predicate shared by every source.
Signal writes during evaluation are rejected. Errors stop invocation.

Timing class:
[evaluation](https://github.com/pertyjons/pert-ui/blob/main/docs/contracts.md#evaluation).

##### [§](#errors-11)Errors

Returns
[`RuntimeError::Closed`](enum.RuntimeError.html#variant.Closed "variant pert_ui::RuntimeError::Closed")
after shutdown, and `WriteDuringUpdate`, `WriteDuringLayout`,
`WriteDuringPaint`, `WriteDuringInspection` or `WriteDuringDelivery`
when called from inside that phase; and `StaleCommand` for a command
that is unregistered or belongs to another runtime. Errors the predicate
itself returns surface later, from
[`Self::command_info`](struct.Runtime.html#method.command_info "method pert_ui::Runtime::command_info")
and
[`Self::invoke_command`](struct.Runtime.html#method.invoke_command "method pert_ui::Runtime::invoke_command").

[Source](../src/pert_ui_runtime/runtime/commands.rs.html#796)

#### pub fn [command_in_modals](#method.command_in_modals)( &mut self, id: &[CommandId](struct.CommandId.html "struct pert_ui::CommandId"), allowed: [bool](https://doc.rust-lang.org/1.99.1/std/primitive.bool.html), ) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[()](https://doc.rust-lang.org/1.99.1/std/primitive.unit.html), [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\>

Allows an application/capture command from inside an admitted modal
chain. Default false prevents host fallback actions from escaping dialog
editing. This never allows blocked targets outside the modal or bypasses
enabled state.

##### [§](#errors-12)Errors

Returns
[`RuntimeError::Closed`](enum.RuntimeError.html#variant.Closed "variant pert_ui::RuntimeError::Closed")
after shutdown, and `WriteDuringUpdate`, `WriteDuringLayout`,
`WriteDuringPaint`, `WriteDuringInspection` or `WriteDuringDelivery`
when called from inside that phase; and `StaleCommand` for a command
that is unregistered or belongs to another runtime.

[Source](../src/pert_ui_runtime/runtime/commands.rs.html#816)

#### pub fn [command_repeats](#method.command_repeats)( &mut self, id: &[CommandId](struct.CommandId.html "struct pert_ui::CommandId"), repeat: [bool](https://doc.rust-lang.org/1.99.1/std/primitive.bool.html), ) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[()](https://doc.rust-lang.org/1.99.1/std/primitive.unit.html), [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\>

Opts a command into OS repeat invocation. Only repeats of a physical
press admitted by this command invoke it; a repeat cannot start a new
command after focus or modifiers change. An unowned repeat is left to
component input and associated text insertion rather than reserved by a
matching command chord. Releases never invoke commands.

##### [§](#errors-13)Errors

Returns
[`RuntimeError::Closed`](enum.RuntimeError.html#variant.Closed "variant pert_ui::RuntimeError::Closed")
after shutdown, and `WriteDuringUpdate`, `WriteDuringLayout`,
`WriteDuringPaint`, `WriteDuringInspection` or `WriteDuringDelivery`
when called from inside that phase; and `StaleCommand` for a command
that is unregistered or belongs to another runtime.

[Source](../src/pert_ui_runtime/runtime/commands.rs.html#835)

#### pub fn [set_command_target](#method.set_command_target)( &mut self, target: [Option](https://doc.rust-lang.org/1.99.1/core/option/enum.Option.html "enum core::option::Option")\<&[WidgetId](struct.WidgetId.html "struct pert_ui::WidgetId")\>, ) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[()](https://doc.rust-lang.org/1.99.1/std/primitive.unit.html), [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\>

Selects an explicit content target independently of focus. None uses
only window/application scopes.

##### [§](#errors-14)Errors

Returns
[`RuntimeError::Closed`](enum.RuntimeError.html#variant.Closed "variant pert_ui::RuntimeError::Closed")
after shutdown, and `WriteDuringUpdate`, `WriteDuringLayout`,
`WriteDuringPaint`, `WriteDuringInspection` or `WriteDuringDelivery`
when called from inside that phase; `ForeignRuntime` or `Unmounted` for
a foreign or unmounted target; and `StaleCommandTarget` for a hidden
target or one in another window than the selected one.

[Source](../src/pert_ui_runtime/runtime/commands.rs.html#848)

#### pub fn [command_context](#method.command_context)(&self, source: [CommandSource](enum.CommandSource.html "enum pert_ui::CommandSource")) -\> [CommandContext](struct.CommandContext.html "struct pert_ui::CommandContext")

Captures this window’s current content target for a menu/button/host
action. Toolbar/menu focus does not replace a content target with an
unrelated view.

[Source](../src/pert_ui_runtime/runtime/commands.rs.html#904-908)

#### pub fn [command_info](#method.command_info)( &mut self, id: &[CommandId](struct.CommandId.html "struct pert_ui::CommandId"), context: &[CommandContext](struct.CommandContext.html "struct pert_ui::CommandContext"), ) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[CommandInfo](struct.CommandInfo.html "struct pert_ui::CommandInfo"), [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\>

Reads a command’s label/enabled state for the exact captured target.
Temporarily selects its window and discards stale optional focus,
matching invocation context. The caller’s selected root is restored even
on failure. A modal-blocked command reports disabled. The predicate is
read-only; this query does not flush pending UI changes or execute
queued actions. Modal admission uses current root visibility and
declared kind even before surface synchronization. A minimized
application-modal owner still blocks other windows, and incomplete
application-modal admission stays blocked. The read-only query makes one
pass over all mounted nodes to inspect surface declarations; pending
component updates remain unevaluated until a flush.

##### [§](#errors-15)Errors

Returns
[`RuntimeError::Closed`](enum.RuntimeError.html#variant.Closed "variant pert_ui::RuntimeError::Closed")
after shutdown, and `WriteDuringUpdate`, `WriteDuringLayout`,
`WriteDuringPaint`, `WriteDuringInspection` or `WriteDuringDelivery`
when called from inside that phase; `StaleCommandTarget` when the
context’s window is gone or hidden, its target is foreign, unmounted,
hidden or in another window, or the command’s scope does not cover the
target; `Unmounted` when the context’s window cannot be selected;
`StaleCommand` for a command that is unregistered or belongs to another
runtime; and the error of the command’s enabled predicate.

[Source](../src/pert_ui_runtime/runtime/commands.rs.html#984-987)

#### pub fn [commands_for](#method.commands_for)( &mut self, context: &[CommandContext](struct.CommandContext.html "struct pert_ui::CommandContext"), ) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[Vec](https://doc.rust-lang.org/1.99.1/alloc/vec/struct.Vec.html "struct alloc::vec::Vec")\<[CommandEntry](struct.CommandEntry.html "struct pert_ui::CommandEntry")\>, [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\>

Lists every registered command whose scope covers the captured context,
with its name, scope, shortcuts and enabled state, so an application can
present commands without keeping a second registry.

This is
[`Self::command_info`](struct.Runtime.html#method.command_info "method pert_ui::Runtime::command_info")
for all eligible commands at once, with its rules: the context’s window
is selected for the query and the caller’s selection restored, stale
optional focus is discarded, a modal-blocked command is listed as
disabled, and predicates run read-only. A predicate’s error is reported
in its own row’s
[`CommandEntry::enabled`](struct.CommandEntry.html#structfield.enabled "field pert_ui::CommandEntry::enabled"),
so one failing predicate cannot hide the other commands. Application and
capture commands are always eligible, window commands for their window,
and view commands when the context’s target lies in their view. Rows are
sorted by name in code-point order, ties in registration order; ranking
for a search is the caller’s.

The result is an owned snapshot. A command palette usually opens from a
command’s action, which receives the runtime and the invoking context:
list there, publish the rows to the palette, and invoke the chosen row’s
id, which revalidates it.

``` rust
use pert_ui::{CommandScope, CommandSource, KeyChord, Runtime};
let mut ui = Runtime::new();
let save = ui.register_command("Save", CommandScope::Application,
    vec![KeyChord::primary('s')], |_, _| Ok(()))?;
let export = ui.register_command("Export", CommandScope::Application,
    Vec::new(), |_, _| Ok(()))?;
let saving = ui.signal(false);
ui.command_enabled_signal(&export, saving.read_only())?;
let context = ui.command_context(CommandSource::Programmatic);
let rows = ui.commands_for(&context)?;
let names: Vec<_> = rows.iter().map(|row| (row.name.as_str(), row.enabled.clone())).collect();
assert_eq!(names, [("Export", Ok(false)), ("Save", Ok(true))]);
assert_eq!(rows[1].id, save);
assert_eq!(rows[1].shortcuts, [KeyChord::primary('s')]);
```

Mounted surface declarations are inspected once for the entire listing,
independently of the number of eligible commands.

##### [§](#errors-16)Errors

Returns
[`RuntimeError::Closed`](enum.RuntimeError.html#variant.Closed "variant pert_ui::RuntimeError::Closed")
after shutdown, and `WriteDuringUpdate`, `WriteDuringLayout`,
`WriteDuringPaint`, `WriteDuringInspection` or `WriteDuringDelivery`
when called from inside that phase; `StaleCommandTarget` when the
context’s window is gone or hidden or its target is foreign, unmounted,
hidden or in another window; and `Unmounted` when the context’s window
cannot be selected. Predicate errors are per row, not errors of the
listing.

[Source](../src/pert_ui_runtime/runtime/commands.rs.html#1088-1092)

#### pub fn [invoke_command](#method.invoke_command)( &mut self, id: &[CommandId](struct.CommandId.html "struct pert_ui::CommandId"), context: [CommandContext](struct.CommandContext.html "struct pert_ui::CommandContext"), ) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[CommandOutcome](enum.CommandOutcome.html "enum pert_ui::CommandOutcome"), [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\>

Invokes the shared action path for keyboard, menus, buttons and host
calls.

Revalidates context, scope, modality and enabled state each time. A
stale menu never retargets another editor. Results default to the
invoking window; the action may explicitly target another validated
window. Callback errors propagate without replaying an action or rolling
back completed host work.

Two side effects precede and follow the action:

- Before admission, every presentation root is flushed as by
  [`HostIntegration::flush_windows`](host/trait.HostIntegration.html#tymethod.flush_windows "method pert_ui::host::HostIntegration::flush_windows"),
  so a modal opened through a signal is admitted before the captured
  target is validated. A flush error, even from a component unrelated to
  the command, is returned before the command is admitted, and the
  action does not run.
- An action that returns an error is followed by
  [`HostIntegration::cancel_input`](host/trait.HostIntegration.html#tymethod.cancel_input "method pert_ui::host::HostIntegration::cancel_input"):
  held keys, pointer buttons, capture and component gestures in the
  selected window end, as after any failed input callback, and the error
  is returned.

Admission uses the synchronized surface frontier, without walking all
mounted nodes again.

The window selected before the call is selected again afterwards, unless
the action retired it.

##### [§](#errors-17)Errors

Returns
[`RuntimeError::Closed`](enum.RuntimeError.html#variant.Closed "variant pert_ui::RuntimeError::Closed")
after shutdown, and `WriteDuringUpdate`, `WriteDuringLayout`,
`WriteDuringPaint`, `WriteDuringInspection` or `WriteDuringDelivery`
when called from inside that phase; the flush errors of
[`HostIntegration::flush_windows`](host/trait.HostIntegration.html#tymethod.flush_windows "method pert_ui::host::HostIntegration::flush_windows"),
before admission; the context and command errors of
[`Self::command_info`](struct.Runtime.html#method.command_info "method pert_ui::Runtime::command_info");
`BorrowConflict` when the action invokes itself; and the action’s own
error, after
[`HostIntegration::cancel_input`](host/trait.HostIntegration.html#tymethod.cancel_input "method pert_ui::host::HostIntegration::cancel_input").
An error returned by the action is an explicit command failure,
including a nested runtime error that the action chooses to propagate.
Catch and handle an unrelated nested delivery error inside the action to
keep the command successful. A refused admission is not an error: it is
a
[`CommandOutcome`](enum.CommandOutcome.html "enum pert_ui::CommandOutcome")
such as `Blocked` or `Disabled`.

[Source](../src/pert_ui_runtime/runtime/commands.rs.html#1384-1387)

#### pub fn [command_key_input](#method.command_key_input)( &mut self, input: [CommandKeyInput](struct.CommandKeyInput.html "struct pert_ui::CommandKeyInput"), ) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[KeyDispatch](struct.KeyDispatch.html "struct pert_ui::KeyDispatch"), [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\>

Routes one translated native key exactly once. Text editing precedes
explicit capture, ordinary focus bubbling, target/window bindings and
app fallback. The returned flag controls associated native text
insertion. Native hosts must discard synthetic events before calling
this method.

The complete order, after a flush of every window; the first stage that
consumes the key ends routing. A fresh press is the first press of a
key, not a repeat.

1.  A repeat of a key that is not held (an orphan) is consumed and
    changes nothing.
2.  During a drag session, Escape’s fresh press cancels the drag.
3.  Keys already owned are consumed by their owner. First a held-input
    owner that is no longer available, or that a modal surface now
    blocks, is cancelled. Then:
    - a held key whose input was cancelled stays suppressed until its
      release or a new fresh press;
    - a key whose fresh press a held-input registration took sends its
      repeats and release to that registration;
    - a key whose fresh press matched a command binding keeps it: each
      repeat runs the command again when its press was admitted (a
      disabled command’s press was consumed without running), and the
      release ends it.
4.  A key that a modal surface forbids to commands is consumed.
5.  Shown delayed help takes an unmodified fresh Escape press.
6.  The text phase: from the focused component up, each
    [`Component::text_key_event`](trait.Component.html#method.text_key_event "method pert_ui::Component::text_key_event")
    may handle the key or insert its text; an editor that leaves it
    passes it to its parent, and the first component that is not an
    editor ends the phase. An active composition then consumes the key.
7.  On a fresh press, command bindings of
    [`CommandScope::Capture`](enum.CommandScope.html#variant.Capture "variant pert_ui::CommandScope::Capture"),
    then
    [`Runtime::register_held_input`](struct.Runtime.html#method.register_held_input "method pert_ui::Runtime::register_held_input")
    registrations, each taking the key as in stage 3. A repeat never
    starts a command.
8.  Escape’s fresh press dismisses the frontmost dismissible surface,
    and otherwise ends the focused component’s timers, capture and
    gesture contexts before it is delivered.
9.  An unmodified Tab or Shift+Tab, pressed or repeated, is offered to
    the focused component and its ancestors through
    [`Component::tab_navigation`](trait.Component.html#method.tab_navigation "method pert_ui::Component::tab_navigation");
    when none claims it, focus moves to the next or previous Tab stop.
    Either way the key is consumed.
10. Ordinary bubbling
    ([`Component::keyboard_event`](trait.Component.html#method.keyboard_event "method pert_ui::Component::keyboard_event")),
    from the component that ended the text phase upward.
11. On a fresh press, command bindings of the command target’s views,
    outward, then of the window, then of the application.

This is the entry point the native `App` uses, and the only one that
routes command bindings and
[`Runtime::register_held_input`](struct.Runtime.html#method.register_held_input "method pert_ui::Runtime::register_held_input")
registrations. A test or custom host that should behave as an
application does for its user uses this method rather than
[`Runtime::key_input`](struct.Runtime.html#method.key_input "method pert_ui::Runtime::key_input").
[`Driver::key`](testing/struct.Driver.html#method.key "method pert_ui::testing::Driver::key")
and
[`Driver::chord`](testing/struct.Driver.html#method.chord "method pert_ui::testing::Driver::chord")
send synthetic named keys and chords through it; a test of a layout, of
text a key carries or of a composition builds the
[`CommandKeyInput`](struct.CommandKeyInput.html "struct pert_ui::CommandKeyInput")
itself.

The key is delivered at once and its recipient writes what its own rules
write, as
[`Runtime::text_input`](struct.Runtime.html#method.text_input "method pert_ui::Runtime::text_input")
describes; the tree, derived bindings and layout show the result after
the next
[`Runtime::flush`](struct.Runtime.html#method.flush "method pert_ui::Runtime::flush").
During a drag session, Escape’s press cancels the drag before anything
else sees it, and a modifier change re-resolves the offered operation,
exactly as through
[`Runtime::key_input`](struct.Runtime.html#method.key_input "method pert_ui::Runtime::key_input").

##### [§](#errors-18)Errors

Returns
[`RuntimeError::Closed`](enum.RuntimeError.html#variant.Closed "variant pert_ui::RuntimeError::Closed")
after shutdown, and `WriteDuringUpdate`, `WriteDuringLayout`,
`WriteDuringPaint`, `WriteDuringInspection` or `WriteDuringDelivery`
when called from inside that phase; forwards the errors of the flush
that runs first, of the focused component’s key callbacks
([`Component::text_key_event`](trait.Component.html#method.text_key_event "method pert_ui::Component::text_key_event"),
[`Component::tab_navigation`](trait.Component.html#method.tab_navigation "method pert_ui::Component::tab_navigation"),
[`Component::keyboard_event`](trait.Component.html#method.keyboard_event "method pert_ui::Component::keyboard_event")),
of dismissing shown help or a surface, of held-input actions, of any
command invoked (see
[`Self::invoke_command`](struct.Runtime.html#method.invoke_command "method pert_ui::Runtime::invoke_command"))
and of the flush that delivers a notification the key requested; and
returns `ShortcutConflict` when bindings of more than one command in one
scope match the key. Direct dispatch errors and delivery failures
attributable to this key cancel input as by
[`HostIntegration::cancel_input`](host/trait.HostIntegration.html#tymethod.cancel_input "method pert_ui::host::HostIntegration::cancel_input");
an error from that cancellation is returned instead. Older/external
notification failures, including notifications generated by the initial
preflush, propagate without cancelling held input. Collective settlement
errors without a uniquely attributable request also preserve the current
gesture. An invoked command action’s returned error remains an explicit
command failure and cancels input, as documented by
[`Self::invoke_command`](struct.Runtime.html#method.invoke_command "method pert_ui::Runtime::invoke_command").
If the initial preflush prevents a held key’s release from dispatching,
cleanup still releases its physical state and held actions. Ordinary
preflush update failures, including failures in another visible window,
also cancel input in the selected window before returning the error.

All presentation roots are flushed before routing, so a newly shown
application dialog in another window also blocks text delivery. Modal
routing then checks synchronized surfaces rather than all mounted nodes.

[Source](../src/pert_ui_runtime/runtime/commands.rs.html#1639)[§](#impl-Runtime-6)

### impl [Runtime](struct.Runtime.html "struct pert_ui::Runtime")

[Source](../src/pert_ui_runtime/runtime/commands.rs.html#1655)

#### pub fn [native_modifiers](#method.native_modifiers)( &mut self, modifiers: [Modifiers](struct.Modifiers.html "struct pert_ui::Modifiers"), ) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[()](https://doc.rust-lang.org/1.99.1/std/primitive.unit.html), [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\>

Reconciles OS-reported modifiers, including keys held before native
focus arrived. This does not fabricate physical press/release events.
Native hosts call it for ModifiersChanged; native focus loss clears the
state.

During a drag session in this window, a change re-resolves the offered
operation, so the target sees an `Over` with the new modifiers before a
release performs it.

##### [§](#errors-19)Errors

Returns
[`RuntimeError::Closed`](enum.RuntimeError.html#variant.Closed "variant pert_ui::RuntimeError::Closed")
after shutdown, and `WriteDuringUpdate`, `WriteDuringLayout`,
`WriteDuringPaint`, `WriteDuringInspection` or `WriteDuringDelivery`
when called from inside that phase. During a drag session it forwards
the errors of re-resolving the offer: the target’s
[`Component::drag_event`](trait.Component.html#method.drag_event "method pert_ui::Component::drag_event")
error and `DropNotPermitted`.

[Source](../src/pert_ui_runtime/runtime/commands.rs.html#1726)[§](#impl-Runtime-7)

### impl [Runtime](struct.Runtime.html "struct pert_ui::Runtime")

[Source](../src/pert_ui_runtime/runtime/commands.rs.html#1764-1768)

#### pub fn [command_enabled_signal](#method.command_enabled_signal)( &mut self, id: &[CommandId](struct.CommandId.html "struct pert_ui::CommandId"), enabled: impl [Into](https://doc.rust-lang.org/1.99.1/core/convert/trait.Into.html "trait core::convert::Into")\<[ReadSignal](struct.ReadSignal.html "struct pert_ui::ReadSignal")\<[bool](https://doc.rust-lang.org/1.99.1/std/primitive.bool.html)\>\>, ) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[()](https://doc.rust-lang.org/1.99.1/std/primitive.unit.html), [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\>

Uses one reactive enabled value for this command and UI controls bound
to the same signal. For contextual predicates, use command_enabled and
query command_info for each captured target.

##### [§](#errors-20)Errors

Returns `ForeignRuntime` for a signal of another runtime and the
signal’s read errors (`Closed`, `BorrowConflict`), then the errors of
[`Self::command_enabled`](struct.Runtime.html#method.command_enabled "method pert_ui::Runtime::command_enabled").

[Source](../src/pert_ui_runtime/runtime/commands.rs.html#1842)[§](#impl-Runtime-8)

### impl [Runtime](struct.Runtime.html "struct pert_ui::Runtime")

[Source](../src/pert_ui_runtime/runtime/commands.rs.html#1848)

#### pub const [MAX_QUEUED_COMMANDS](#associatedconstant.MAX_QUEUED_COMMANDS): [usize](https://doc.rust-lang.org/1.99.1/std/primitive.usize.html) = 64

Maximum number of queued command invocations, from
[`EventContext::invoke_command`](struct.EventContext.html#method.invoke_command "method pert_ui::EventContext::invoke_command"),
waiting for the next flush. An invocation beyond it is refused with
[`RuntimeError::CommandQueueFull`](enum.RuntimeError.html#variant.CommandQueueFull "variant pert_ui::RuntimeError::CommandQueueFull");
the queue is emptied by every flush and by
[`HostIntegration::flush_commands`](host/trait.HostIntegration.html#tymethod.flush_commands "method pert_ui::host::HostIntegration::flush_commands").

[Source](../src/pert_ui_runtime/runtime/compose.rs.html#4)[§](#impl-Runtime-9)

### impl [Runtime](struct.Runtime.html "struct pert_ui::Runtime")

[Source](../src/pert_ui_runtime/runtime/compose.rs.html#32)

#### pub fn [paint](#method.paint)(&mut self) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[DisplayList](struct.DisplayList.html "struct pert_ui::DisplayList"), [DrawError](enum.DrawError.html "enum pert_ui::DrawError")\>

Composes cached drawing in root mount order and child presentation
order.

Only successfully updated components record again. Scene composition
copies commands but never re-runs unchanged components’ paint callbacks.
The desktop adapter calls this after flushing updates. No timers or GPU
work occur here.

The scene carries what can produce pixels. A component whose effective
clip has no area, such as an item scrolled out of a list, contributes no
command and no coverage resource, and neither does an individual command
that its clip removes; the cost of a composition follows the content on
screen rather than the content mounted. Such a component is still
retained, still measured and still records when it is invalidated, so
revealing it composes it again without repainting it.

##### [§](#errors-21)Errors

Returns
[`DrawError::Runtime`](enum.DrawError.html#variant.Runtime "variant pert_ui::DrawError::Runtime")
holding a runtime phase error (see
[`Runtime`](struct.Runtime.html "struct pert_ui::Runtime"));
[`DrawError::PendingUpdates`](enum.DrawError.html#variant.PendingUpdates "variant pert_ui::DrawError::PendingUpdates")
while updates, layout or queued requests wait for a flush; and a
component’s own drawing error as it returned it. Failed or panicking
callbacks keep their prior cache dirty.

`DrawError` does not convert into
[`RuntimeError`](enum.RuntimeError.html "enum pert_ui::RuntimeError"),
so a helper that flushes and paints, such as a test that inspects the
scene, returns `Result<_, DrawError>`: `RuntimeError` and every leaf
error convert into it with `?`. With the facade’s `native` feature its
`AppError` accepts both as well.

[Source](../src/pert_ui_runtime/runtime/content_search.rs.html#10)[§](#impl-Runtime-10)

### impl [Runtime](struct.Runtime.html "struct pert_ui::Runtime")

[Source](../src/pert_ui_runtime/runtime/content_search.rs.html#72)

#### pub fn [content_search](#method.content_search)(&self, query: [ContentQuery](struct.ContentQuery.html "struct pert_ui::ContentQuery")) -\> [ContentSearch](struct.ContentSearch.html "struct pert_ui::ContentSearch")

Starts a resumable search of the application’s content for `query`.

The search looks at what components already report through their
[`Semantics`](struct.Semantics.html "struct pert_ui::Semantics"): the
resolved name (a component’s own, its element’s label, or the one a
wrapping `Field` gives it) and the value text (text, a number, a
choice’s label). Hidden components are included unless the query
excludes them, so content in an unselected tab or a collapsed section is
found;
[`Self::jump_to_content`](struct.Runtime.html#method.jump_to_content "method pert_ui::Runtime::jump_to_content")
then opens the way to it. A collection that offers
[`ContentRows`](trait.ContentRows.html "trait pert_ui::ContentRows")
through
[`Component::try_content_rows`](trait.Component.html#method.try_content_rows "method pert_ui::Component::try_content_rows")
is also searched for rows it has not mounted; `VirtualList`, and so
`Table` and `Tree`, do when their source answers `text`.

Never found: a subtree excluded with
[`Element::with_searchable`](struct.Element.html#method.with_searchable "method pert_ui::Element::with_searchable"),
the value of a
[protected](struct.Semantics.html#method.protected "method pert_ui::Semantics::protected")
component, components a modal surface blocks, drawing without semantics,
and rows a source does not describe. Nothing runs until
[`ContentSearch::step`](struct.ContentSearch.html#method.step "method pert_ui::ContentSearch::step")
is called; each step visits components and rows within the query’s work
budget and result limit. Beyond that budget a step also reads the
revision of each collection the search has already entered, once, to
detect a replaced source, so its cost grows with the number of
collections entered.

A component reports the values it has read. One that was mounted hidden
and has never updated, such as a field on a page that has not been
shown, reports its initial state until it is shown once; a collection
offers its current bound rows even then and after a hidden source
replacement. Obsolete retained row children are skipped until
presentation catches up; searching does not update or mount them.

``` rust
use pert_ui::prelude::*;
use pert_ui::testing::*;
use pert_ui::{ContentQuery, ContentTarget, RevealOutcome, ContentJump};

let mut ui = Runtime::new();
ui.set_viewport(Viewport::new(400.0, 300.0, 1.0)?)?;
ui.mount_tree(Element::new("tabs", Tabs::new([
    Tab::new("general", "General", Element::new("intro", Label::new("Welcome"))),
    Tab::new("details", "Details", Element::new(
        "report", Button::new("Open report", |_| Ok(())),
    )),
])))?;
ui.flush(std::time::Duration::ZERO)?;

// The button sits in the unselected tab and is still found.
let step = ui.content_search(ContentQuery::new("report")?).step(&ui)?;
assert!(step.done);
let hit = &step.hits[0];
assert_eq!(hit.text, "Open report");
assert!(!hit.visible);
let ContentJump::Reveal(RevealOutcome::RevealedAndFocused { .. }) =
    ui.jump_to_content(&hit.target)?
else {
    panic!("the jump did not reach the button");
};
```

[Source](../src/pert_ui_runtime/runtime/content_search.rs.html#359)

#### pub fn [jump_to_content](#method.jump_to_content)( &mut self, target: &[ContentTarget](enum.ContentTarget.html "enum pert_ui::ContentTarget"), ) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[ContentJump](enum.ContentJump.html "enum pert_ui::ContentJump"), [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\>

Opens the way to a content hit and focuses it.

A mounted component is revealed and focused with
[`Self::reveal_and_focus`](struct.Runtime.html#method.reveal_and_focus "method pert_ui::Runtime::reveal_and_focus")
(a component that cannot take focus is only revealed). A row that is not
mounted is reached in three steps: the collection is revealed, the row
is requested by key through the collection’s
[`ContentRows`](trait.ContentRows.html "trait pert_ui::ContentRows") and
the runtime flushes until it is mounted, and then the row is revealed
and focused. Like the reveal operations it flushes, so it may be called
where those may: from application code, a command action or a deferred
task (`EventContext::defer`), never from a component callback.

The outcome is typed: a refused reveal is
`ContentJump::Reveal(RevealOutcome::Refused(..))`, a row the collection
could not mount is
[`ContentJump::Row`](enum.ContentJump.html#variant.Row "variant pert_ui::ContentJump::Row")
with its status, and a row the collection has not mounted after two
flushes is
[`ContentJump::Pending`](enum.ContentJump.html#variant.Pending "variant pert_ui::ContentJump::Pending"),
which
[`Self::resume_content_jump`](struct.Runtime.html#method.resume_content_jump "method pert_ui::Runtime::resume_content_jump")
finishes.

##### [§](#errors-22)Errors

The errors of
[`Self::reveal_and_focus`](struct.Runtime.html#method.reveal_and_focus "method pert_ui::Runtime::reveal_and_focus")
and
[`Self::flush`](struct.Runtime.html#method.flush "method pert_ui::Runtime::flush")
for a stale, foreign or unmounted target, and the collection’s request
failure for a row.

[Source](../src/pert_ui_runtime/runtime/content_search.rs.html#397-400)

#### pub fn [resume_content_jump](#method.resume_content_jump)( &mut self, pending: &[PendingJump](struct.PendingJump.html "struct pert_ui::PendingJump"), ) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[ContentJump](enum.ContentJump.html "enum pert_ui::ContentJump"), [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\>

Finishes a
[`ContentJump::Pending`](enum.ContentJump.html#variant.Pending "variant pert_ui::ContentJump::Pending")
jump: once the collection has mounted the row it reveals and focuses it,
as
[`Self::jump_to_content`](struct.Runtime.html#method.jump_to_content "method pert_ui::Runtime::jump_to_content")
would have; a row the collection could not mount ends as
[`ContentJump::Row`](enum.ContentJump.html#variant.Row "variant pert_ui::ContentJump::Row")
with its status; a row still being mounted returns the pending jump
again. It flushes only to reveal, so it may be called where the reveal
operations may.

##### [§](#errors-23)Errors

The errors of
[`Self::reveal_and_focus`](struct.Runtime.html#method.reveal_and_focus "method pert_ui::Runtime::reveal_and_focus")
for a stale or foreign row.

[Source](../src/pert_ui_runtime/runtime/drag.rs.html#171)[§](#impl-Runtime-11)

### impl [Runtime](struct.Runtime.html "struct pert_ui::Runtime")

[Source](../src/pert_ui_runtime/runtime/drag.rs.html#174)

#### pub fn [drag_active](#method.drag_active)(&self) -\> [bool](https://doc.rust-lang.org/1.99.1/std/primitive.bool.html)

Whether a drag session is active. Reading schedules nothing.

[Source](../src/pert_ui_runtime/runtime/fonts.rs.html#246)[§](#impl-Runtime-12)

### impl [Runtime](struct.Runtime.html "struct pert_ui::Runtime")

[Source](../src/pert_ui_runtime/runtime/fonts.rs.html#248)

#### pub fn [fonts](#method.fonts)(&self) -\> [FontManager](struct.FontManager.html "struct pert_ui::FontManager")

Font registration and semantic defaults for this runtime; no work is
scheduled.

[Source](../src/pert_ui_runtime/runtime/fonts.rs.html#254)

#### pub fn [icons](#method.icons)(&self) -\> [IconManager](struct.IconManager.html "struct pert_ui::IconManager")

Standard icon mappings for this runtime; no work is scheduled.

[Source](../src/pert_ui_runtime/runtime/held.rs.html#50)[§](#impl-Runtime-13)

### impl [Runtime](struct.Runtime.html "struct pert_ui::Runtime")

[Source](../src/pert_ui_runtime/runtime/held.rs.html#74-86)

#### pub fn [register_held_input](#method.register_held_input)( &mut self, owner: &[WidgetId](struct.WidgetId.html "struct pert_ui::WidgetId"), keys: [Vec](https://doc.rust-lang.org/1.99.1/alloc/vec/struct.Vec.html "struct alloc::vec::Vec")\<[KeyCode](enum.KeyCode.html "enum pert_ui::KeyCode")\>, modifiers: [Modifiers](struct.Modifiers.html "struct pert_ui::Modifiers"), action: impl [FnMut](https://doc.rust-lang.org/1.99.1/core/ops/function/trait.FnMut.html "trait core::ops::function::FnMut")([KeyCode](enum.KeyCode.html "enum pert_ui::KeyCode"), [KeyTransition](enum.KeyTransition.html "enum pert_ui::KeyTransition"), &[CommandContext](struct.CommandContext.html "struct pert_ui::CommandContext")) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[InputOutcome](enum.InputOutcome.html "enum pert_ui::InputOutcome"), [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\> + 'static, cancel: impl [FnMut](https://doc.rust-lang.org/1.99.1/core/ops/function/trait.FnMut.html "trait core::ops::function::FnMut")([KeyCode](enum.KeyCode.html "enum pert_ui::KeyCode"), [HeldCancellation](enum.HeldCancellation.html "enum pert_ui::HeldCancellation")) + 'static, ) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[HeldInputId](struct.HeldInputId.html "struct pert_ui::HeldInputId"), [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\>

Registers explicit physical-key capture for a visible owner in the
invoking window. Text editing/composition has precedence. Keys and exact
modifiers are position-based; modifier keys themselves cannot be owned.
Duplicate keys in overlapping registrations are rejected in every build
profile.

Timing class of `action`: [in the
input](https://github.com/pertyjons/pert-ui/blob/main/docs/contracts.md#in-the-input);
`cancel` runs wherever the input is cancelled.

Returning Handled for Pressed owns the key; repeats and release are
routed solely here. The release record is removed before calling action.
A callback error cancels any still-owned keys. Cancellation is a
separate infallible cleanup callback: it must release host resources
without activating commands. These UI-thread callbacks are not safe for
time-critical producers and must return promptly. Lifetime follows the
mounted owner, including native presentation movement; movement cancels
current presses while preserving the registration. Dropping the returned
[`HeldInputId`](struct.HeldInputId.html "struct pert_ui::HeldInputId")
keeps the registration.

##### [§](#errors-24)Errors

Returns
[`RuntimeError::Closed`](enum.RuntimeError.html#variant.Closed "variant pert_ui::RuntimeError::Closed")
after shutdown, and `WriteDuringUpdate`, `WriteDuringLayout`,
`WriteDuringPaint`, `WriteDuringInspection` or `WriteDuringDelivery`
when called from inside that phase; `ForeignRuntime` or `Unmounted` for
a foreign or unmounted owner; `ShortcutConflict` for a modifier key, a
key listed twice, or a key another registration of the same modifiers
already owns; and `Exhausted` when registration identities run out.

[Source](../src/pert_ui_runtime/runtime/held.rs.html#137)

#### pub fn [unregister_held_input](#method.unregister_held_input)( &mut self, id: &[HeldInputId](struct.HeldInputId.html "struct pert_ui::HeldInputId"), ) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[()](https://doc.rust-lang.org/1.99.1/std/primitive.unit.html), [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\>

Deregisters an exact identity, canceling its admitted presses once
before releasing callbacks. Late repeats/releases remain suppressed
until release.

##### [§](#errors-25)Errors

Returns
[`RuntimeError::Closed`](enum.RuntimeError.html#variant.Closed "variant pert_ui::RuntimeError::Closed")
after shutdown, and `WriteDuringUpdate`, `WriteDuringLayout`,
`WriteDuringPaint`, `WriteDuringInspection` or `WriteDuringDelivery`
when called from inside that phase; and `StaleCommand` for an identity
that is already unregistered or belongs to another registration.

[Source](../src/pert_ui_runtime/runtime/inspect.rs.html#4)[§](#impl-Runtime-14)

### impl [Runtime](struct.Runtime.html "struct pert_ui::Runtime")

[Source](../src/pert_ui_runtime/runtime/inspect.rs.html#7)

#### pub fn [clear_event_history](#method.clear_event_history)(&self)

Clears retained events and the eviction count; event serials never
restart, and the history generation advances so a poller can tell.

[Source](../src/pert_ui_runtime/runtime/inspect.rs.html#39)

#### pub fn [inspect](#method.inspect)(&self) -\> [InspectionSnapshot](struct.InspectionSnapshot.html "struct pert_ui::InspectionSnapshot")

Captures owned, versioned runtime/component/parameter state on demand.

May allocate proportional to the inspected state; never runs
automatically on a frame or timer. The result is Send/Sync and may be
handed to the application’s tooling thread, with no live UI references
or mutation capability. Arbitrary host data requires explicit
contribution via `inspect_with`. Dead parameter inspection references
are pruned during capture; identity reservations remain intact for the
runtime’s lifetime.

[Source](../src/pert_ui_runtime/runtime/inspect.rs.html#55-58)

#### pub fn [inspect_with](#method.inspect_with)( &self, application: impl [FnOnce](https://doc.rust-lang.org/1.99.1/core/ops/function/trait.FnOnce.html "trait core::ops::function::FnOnce")() -\> [BTreeMap](https://doc.rust-lang.org/1.99.1/alloc/collections/btree/map/struct.BTreeMap.html "struct alloc::collections::btree::map::BTreeMap")\<[String](https://doc.rust-lang.org/1.99.1/alloc/string/struct.String.html "struct alloc::string::String"), [InspectValue](enum.InspectValue.html "enum pert_ui::InspectValue")\>, ) -\> [InspectionSnapshot](struct.InspectionSnapshot.html "struct pert_ui::InspectionSnapshot")

Captures runtime state plus an explicit application-state provider.

Typed [`Semantics`](struct.Semantics.html "struct pert_ui::Semantics")
are resolved here: a component that names its children, such as a
`Field`, gives its name to an immediate child that has none and records
itself in `labelled_by`, and passes its description and invalid state to
every immediate child that reports semantics.

The provider runs synchronously on the UI thread with signal/parameter
writes forbidden, and must not mutate other host state or perform I/O.
Report only data intended for the connected inspection tool. This
contract exposes owned observations, not arbitrary Rust heap
serialization.

[Source](../src/pert_ui_runtime/runtime/inspect.rs.html#243)

#### pub fn [query](#method.query)(&self, query: &[InspectionQuery](struct.InspectionQuery.html "struct pert_ui::InspectionQuery")) -\> [QueryResult](struct.QueryResult.html "struct pert_ui::QueryResult")

Answers a
[query](struct.InspectionQuery.html "struct pert_ui::InspectionQuery")
on the UI thread, with no application properties.

Three costs, each stated on its own. Selection: an id is one lookup; a
key, focus or window selector compares retained state of every mounted
component; a path selector builds the address of every mounted
component, each a walk up its ancestors. A semantic selector runs
`Component::inspect` on every candidate it searches (all mounted
components unless `within` narrows it), whatever detail or limit the
query has, since role and name are only known that way. Description:
`Component::inspect` runs for the returned components, and the parent
that names one, only at
[`Detail::Semantics`](enum.Detail.html#variant.Semantics "variant pert_ui::Detail::Semantics")
and above. Nothing is scheduled and no index is kept.

[Source](../src/pert_ui_runtime/runtime/inspect.rs.html#250-254)

#### pub fn [query_with](#method.query_with)( &self, query: &[InspectionQuery](struct.InspectionQuery.html "struct pert_ui::InspectionQuery"), application: impl [FnOnce](https://doc.rust-lang.org/1.99.1/core/ops/function/trait.FnOnce.html "trait core::ops::function::FnOnce")() -\> [BTreeMap](https://doc.rust-lang.org/1.99.1/alloc/collections/btree/map/struct.BTreeMap.html "struct alloc::collections::btree::map::BTreeMap")\<[String](https://doc.rust-lang.org/1.99.1/alloc/string/struct.String.html "struct alloc::string::String"), [InspectValue](enum.InspectValue.html "enum pert_ui::InspectValue")\>, ) -\> [QueryResult](struct.QueryResult.html "struct pert_ui::QueryResult")

Answers a query, with `application` providing the exposed application
properties when the query asks for them; the provider runs under the
same restrictions as in
[`inspect_with`](struct.Runtime.html#method.inspect_with "method pert_ui::Runtime::inspect_with").

[Source](../src/pert_ui_runtime/runtime/native_windows.rs.html#300)[§](#impl-Runtime-15)

### impl [Runtime](struct.Runtime.html "struct pert_ui::Runtime")

[Source](../src/pert_ui_runtime/runtime/native_windows.rs.html#302)

#### pub fn [primary_window](#method.primary_window)(&self) -\> [NativeWindowId](struct.NativeWindowId.html "struct pert_ui::NativeWindowId")

The main presentation root, retained until runtime destruction.

[Source](../src/pert_ui_runtime/runtime/native_windows.rs.html#310)

#### pub fn [native_windows](#method.native_windows)(&self) -\> [Vec](https://doc.rust-lang.org/1.99.1/alloc/vec/struct.Vec.html "struct alloc::vec::Vec")\<[NativeWindowId](struct.NativeWindowId.html "struct pert_ui::NativeWindowId")\>

All mounted native presentation roots, in creation order.

[Source](../src/pert_ui_runtime/runtime/native_windows.rs.html#353)

#### pub fn [window_of](#method.window_of)(&self, view: &[WidgetId](struct.WidgetId.html "struct pert_ui::WidgetId")) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[NativeWindowId](struct.NativeWindowId.html "struct pert_ui::NativeWindowId"), [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\>

The view’s current presentation window.

##### [§](#errors-26)Errors

Returns `ForeignRuntime` for a view of another runtime and `Unmounted`
for an unmounted one.

[Source](../src/pert_ui_runtime/runtime/native_windows.rs.html#358)

#### pub fn [contains_window](#method.contains_window)(&self, window: &[NativeWindowId](struct.NativeWindowId.html "struct pert_ui::NativeWindowId")) -\> [bool](https://doc.rust-lang.org/1.99.1/std/primitive.bool.html)

Whether this identity still names an available presentation root.

[Source](../src/pert_ui_runtime/runtime/native_windows.rs.html#765)

#### pub fn [detached_view](#method.detached_view)(&self) -\> [Option](https://doc.rust-lang.org/1.99.1/core/option/enum.Option.html "enum core::option::Option")\<[WidgetId](struct.WidgetId.html "struct pert_ui::WidgetId")\>

Current detached root’s content, absent for the main window.

[Source](../src/pert_ui_runtime/runtime/native_windows.rs.html#1027)[§](#impl-Runtime-16)

### impl [Runtime](struct.Runtime.html "struct pert_ui::Runtime")

[Source](../src/pert_ui_runtime/runtime/native_windows.rs.html#1051)

#### pub const [MAX_WINDOW_REQUESTS](#associatedconstant.MAX_WINDOW_REQUESTS): [usize](https://doc.rust-lang.org/1.99.1/std/primitive.usize.html) = 64

Maximum number of native presentation requests waiting for the host.

One queue holds every detach, re-dock, close, activation and retitle
request, from
[`Self::request_detach`](struct.Runtime.html#method.request_detach "method pert_ui::Runtime::request_detach"),
[`Self::request_redock`](struct.Runtime.html#method.request_redock "method pert_ui::Runtime::request_redock"),
[`Self::request_close`](struct.Runtime.html#method.request_close "method pert_ui::Runtime::request_close"),
[`Self::request_window_title`](struct.Runtime.html#method.request_window_title "method pert_ui::Runtime::request_window_title")
and the
[`EventContext`](struct.EventContext.html "struct pert_ui::EventContext")
methods
[`EventContext::detach_view`](struct.EventContext.html#method.detach_view "method pert_ui::EventContext::detach_view"),
[`EventContext::redock_window`](struct.EventContext.html#method.redock_window "method pert_ui::EventContext::redock_window"),
[`EventContext::close_window`](struct.EventContext.html#method.close_window "method pert_ui::EventContext::close_window"),
[`EventContext::activate_window`](struct.EventContext.html#method.activate_window "method pert_ui::EventContext::activate_window")
and
[`EventContext::set_window_title`](struct.EventContext.html#method.set_window_title "method pert_ui::EventContext::set_window_title").
A request beyond it is refused with
[`RuntimeError::WindowRequestFull`](enum.RuntimeError.html#variant.WindowRequestFull "variant pert_ui::RuntimeError::WindowRequestFull")
without being queued; taking the requests with
[`HostIntegration::take_presentation_requests`](host/trait.HostIntegration.html#tymethod.take_presentation_requests "method pert_ui::host::HostIntegration::take_presentation_requests")
frees the queue.

[Source](../src/pert_ui_runtime/runtime/native_windows.rs.html#1067-1071)

#### pub fn [request_detach](#method.request_detach)( &self, view: &[WidgetId](struct.WidgetId.html "struct pert_ui::WidgetId"), options: [NativeWindowOptions](struct.NativeWindowOptions.html "struct pert_ui::NativeWindowOptions"), ) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[PresentationReceipt](struct.PresentationReceipt.html "struct pert_ui::PresentationReceipt"), [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\>

Queues a native detach; the default App prepares resources before
movement. Observe the receipt for typed native/cleanup failures.
Headless hosts consume
[`HostIntegration::take_presentation_requests`](host/trait.HostIntegration.html#tymethod.take_presentation_requests "method pert_ui::host::HostIntegration::take_presentation_requests").

##### [§](#errors-27)Errors

Returns
[`RuntimeError::Closed`](enum.RuntimeError.html#variant.Closed "variant pert_ui::RuntimeError::Closed")
after shutdown, and `WriteDuringUpdate`, `WriteDuringLayout`,
`WriteDuringPaint`, `WriteDuringInspection` or `WriteDuringDelivery`
when called from inside that phase; `ForeignRuntime` or `Unmounted` for
a foreign or unmounted view; `Hidden` for a hidden view; `NotDetachable`
when its description forbids its own window; `ModalBlocked` when the
subtree holds an open modal dialog; and
[`RuntimeError::WindowRequestFull`](enum.RuntimeError.html#variant.WindowRequestFull "variant pert_ui::RuntimeError::WindowRequestFull")
when
[`Runtime::MAX_WINDOW_REQUESTS`](struct.Runtime.html#associatedconstant.MAX_WINDOW_REQUESTS "associated constant pert_ui::Runtime::MAX_WINDOW_REQUESTS")
requests are already queued.

[Source](../src/pert_ui_runtime/runtime/native_windows.rs.html#1092-1095)

#### pub fn [request_redock](#method.request_redock)( &self, window: &[NativeWindowId](struct.NativeWindowId.html "struct pert_ui::NativeWindowId"), ) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[PresentationReceipt](struct.PresentationReceipt.html "struct pert_ui::PresentationReceipt"), [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\>

Queues return to the original owner; failure leaves the detached root
open.

##### [§](#errors-28)Errors

Returns `Unmounted` for a window identity that is not an available
presentation root,
[`RuntimeError::Closed`](enum.RuntimeError.html#variant.Closed "variant pert_ui::RuntimeError::Closed")
after shutdown, and `WriteDuringUpdate`, `WriteDuringLayout`,
`WriteDuringPaint`, `WriteDuringInspection` or `WriteDuringDelivery`
when called from inside that phase; and
[`RuntimeError::WindowRequestFull`](enum.RuntimeError.html#variant.WindowRequestFull "variant pert_ui::RuntimeError::WindowRequestFull")
when
[`Runtime::MAX_WINDOW_REQUESTS`](struct.Runtime.html#associatedconstant.MAX_WINDOW_REQUESTS "associated constant pert_ui::Runtime::MAX_WINDOW_REQUESTS")
requests are already queued.

[Source](../src/pert_ui_runtime/runtime/native_windows.rs.html#1116-1119)

#### pub fn [read_signal](#method.read_signal)\<T\>(&self, signal: &[ReadSignal](struct.ReadSignal.html "struct pert_ui::ReadSignal")\<T\>) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<T, [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\>

where T:
[Clone](https://doc.rust-lang.org/1.99.1/core/clone/trait.Clone.html "trait core::clone::Clone") +
'static,

Reads UI-local state without subscribing. Native hosts use this for
deferred admission; it never schedules polling.

##### [§](#errors-29)Errors

Returns `ForeignRuntime` for a signal of another runtime, `Closed` after
shutdown and `BorrowConflict` when the value is being written.

[Source](../src/pert_ui_runtime/runtime/native_windows.rs.html#1161-1164)

#### pub fn [request_close](#method.request_close)( &self, window: &[NativeWindowId](struct.NativeWindowId.html "struct pert_ui::NativeWindowId"), ) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[PresentationReceipt](struct.PresentationReceipt.html "struct pert_ui::PresentationReceipt"), [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\>

Requests close/save admission. The main window asks for application
exit; detached windows use their configured close policy after
acceptance.

The request reaches the host through
[`HostIntegration::take_presentation_requests`](host/trait.HostIntegration.html#tymethod.take_presentation_requests "method pert_ui::host::HostIntegration::take_presentation_requests")
as
[`PresentationAction::Close`](enum.PresentationAction.html#variant.Close "variant pert_ui::PresentationAction::Close").
The facade’s `App` admits it with its close policy; a host other than
`App` completes it in these steps:

1.  take the `Close` request;
2.  run its own admission, in `App::on_close`’s order: release focus in
    every window the close would discard (the detached window’s own, or
    every window for the main window) with
    [`HostIntegration::clear_focus`](host/trait.HostIntegration.html#tymethod.clear_focus "method pert_ui::host::HostIntegration::clear_focus"),
    flush them, run its policy, and on a synchronous refusal focus the
    released components again;
3.  on a refusal, complete the request with
    [`PresentationStatus::CloseRejected`](enum.PresentationStatus.html#variant.CloseRejected "variant pert_ui::PresentationStatus::CloseRejected");
    on an error, with
    [`PresentationStatus::Failed`](enum.PresentationStatus.html#variant.Failed "variant pert_ui::PresentationStatus::Failed")
    and the error as a
    [`WindowError`](enum.WindowError.html "enum pert_ui::WindowError");
4.  on a deferred decision, keep the request and read the decision
    signal with
    [`Self::read_signal`](struct.Runtime.html#method.read_signal "method pert_ui::Runtime::read_signal")
    after each flush; when its window is gone before a decision arrives,
    drop the request, so it reports `Failed(WindowError::Cancelled)`, as
    `App` does;
5.  on acceptance of the main window, end the application;
6.  on acceptance of a detached window, select it and call
    [`HostIntegration::close_detached_window`](host/trait.HostIntegration.html#tymethod.close_detached_window "method pert_ui::host::HostIntegration::close_detached_window"),
    which may still answer `CloseRejected` under
    [`DetachedClosePolicy::KeepOpen`](enum.DetachedClosePolicy.html#variant.KeepOpen "variant pert_ui::DetachedClosePolicy::KeepOpen");
7.  complete the request with the resulting status.

##### [§](#errors-30)Errors

Returns `Unmounted` for a window identity that is not an available
presentation root,
[`RuntimeError::Closed`](enum.RuntimeError.html#variant.Closed "variant pert_ui::RuntimeError::Closed")
after shutdown, and `WriteDuringUpdate`, `WriteDuringLayout`,
`WriteDuringPaint`, `WriteDuringInspection` or `WriteDuringDelivery`
when called from inside that phase; and
[`RuntimeError::WindowRequestFull`](enum.RuntimeError.html#variant.WindowRequestFull "variant pert_ui::RuntimeError::WindowRequestFull")
when
[`Runtime::MAX_WINDOW_REQUESTS`](struct.Runtime.html#associatedconstant.MAX_WINDOW_REQUESTS "associated constant pert_ui::Runtime::MAX_WINDOW_REQUESTS")
requests are already queued.

[Source](../src/pert_ui_runtime/runtime/native_windows.rs.html#1183-1187)

#### pub fn [request_window_title](#method.request_window_title)( &self, window: &[NativeWindowId](struct.NativeWindowId.html "struct pert_ui::NativeWindowId"), title: impl [Into](https://doc.rust-lang.org/1.99.1/core/convert/trait.Into.html "trait core::convert::Into")\<[String](https://doc.rust-lang.org/1.99.1/alloc/string/struct.String.html "struct alloc::string::String")\>, ) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[PresentationReceipt](struct.PresentationReceipt.html "struct pert_ui::PresentationReceipt"), [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\>

Queues `title` as the native title of the open window `window`, with the
coalescing and errors of
[`EventContext::set_window_title`](struct.EventContext.html#method.set_window_title "method pert_ui::EventContext::set_window_title"),
for application code outside a component, such as a main window title
that names the open document.

##### [§](#errors-31)Errors

Returns
[`RuntimeError::EmptyWindowTitle`](enum.RuntimeError.html#variant.EmptyWindowTitle "variant pert_ui::RuntimeError::EmptyWindowTitle")
for a blank title, `Unmounted` for a window identity that is not an
available presentation root,
[`RuntimeError::Closed`](enum.RuntimeError.html#variant.Closed "variant pert_ui::RuntimeError::Closed")
after shutdown, the phase errors of
[`Self::request_close`](struct.Runtime.html#method.request_close "method pert_ui::Runtime::request_close"),
and
[`RuntimeError::WindowRequestFull`](enum.RuntimeError.html#variant.WindowRequestFull "variant pert_ui::RuntimeError::WindowRequestFull")
when
[`Runtime::MAX_WINDOW_REQUESTS`](struct.Runtime.html#associatedconstant.MAX_WINDOW_REQUESTS "associated constant pert_ui::Runtime::MAX_WINDOW_REQUESTS")
requests are already queued and none of them retitles this window.

[Source](../src/pert_ui_runtime/runtime/pointer.rs.html#4)[§](#impl-Runtime-17)

### impl [Runtime](struct.Runtime.html "struct pert_ui::Runtime")

[Source](../src/pert_ui_runtime/runtime/pointer.rs.html#150)

#### pub fn [focus_next](#method.focus_next)( &mut self, direction: [FocusDirection](enum.FocusDirection.html "enum pert_ui::FocusDirection"), ) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[()](https://doc.rust-lang.org/1.99.1/std/primitive.unit.html), [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\>

Moves focus in visible tree order in `direction`, wrapping at either
end. Hidden or fully clipped components and nonfocusable containers are
skipped. Uses current layout.

##### [§](#errors-32)Errors

Returns
[`RuntimeError::Closed`](enum.RuntimeError.html#variant.Closed "variant pert_ui::RuntimeError::Closed")
after shutdown, and `WriteDuringUpdate`, `WriteDuringLayout`,
`WriteDuringPaint`, `WriteDuringInspection` or `WriteDuringDelivery`
when called from inside that phase; forwards the errors of the
[`Self::flush`](struct.Runtime.html#method.flush "method pert_ui::Runtime::flush")
that runs first when layout is pending, of an ancestor’s
[`Component::reveal`](trait.Component.html#method.reveal "method pert_ui::Component::reveal"),
and of
[`Self::focus`](struct.Runtime.html#method.focus "method pert_ui::Runtime::focus").

[Source](../src/pert_ui_runtime/runtime/pointer.rs.html#350)

#### pub fn [pointer_input](#method.pointer_input)( &mut self, event: [PointerEvent](struct.PointerEvent.html "struct pert_ui::PointerEvent"), ) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[InputOutcome](enum.InputOutcome.html "enum pert_ui::InputOutcome"), [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\>

Applies a pointer transition and dispatches through the frontmost
clipped tree.

While a [drag
session](struct.EventContext.html#method.start_drag "method pert_ui::EventContext::start_drag")
is active in this window, transitions drive the session instead: motion
resolves the drop target and moves the preview, the wheel still scrolls
what is under the pointer, and releasing the last button drops or
cancels. Ordinary hover, press focus and tooltips are suspended until
the session ends.

Press focuses the nearest focusable ancestor without scrolling the
already hit target. Explicit focus and keyboard traversal still reveal
their target. Ignored events bubble toward the root; they never fall
through to covered siblings. Capture is released after the last physical
button goes up or the owner loses context. Input state is updated even
if a callback returns an error. Coordinates are logical window pixels;
the platform adapter handles native DPI and wheel units.

The event is delivered at once and its recipient writes what its own
rules write, as
[`Self::text_input`](struct.Runtime.html#method.text_input "method pert_ui::Runtime::text_input")
describes; the tree, derived bindings and layout show the result after
the next
[`Self::flush`](struct.Runtime.html#method.flush "method pert_ui::Runtime::flush").
A click is a press and a release at one point inside the target’s
[`Self::bounds`](struct.Runtime.html#method.bounds "method pert_ui::Runtime::bounds"),
which are known once a flush has laid it out; a component scrolled out
of view or inside a closed container is revealed first with
[`Self::reveal`](struct.Runtime.html#method.reveal "method pert_ui::Runtime::reveal").
[`Driver::click`](testing/struct.Driver.html#method.click "method pert_ui::testing::Driver::click")
does all of this.

``` rust
use pert_ui::prelude::*;
use pert_ui::testing::*;
use std::time::Duration;

let mut ui = Runtime::new();
ui.set_viewport(Viewport::new(300.0, 100.0, 1.0)?)?;
let clicks = ui.signal(0);
let counter = clicks.clone();
let button = ui.mount_tree(Element::new("add",
    Button::new("Add", move |_| counter.update(|n| *n += 1))))?;
ui.flush(Duration::ZERO)?; // lays the button out
let [x, y, width, height] = ui.bounds(&button)?.ok_or("not laid out")?.values();
let middle = Point::new(x + width / 2.0, y + height / 2.0)?;
for kind in [
    PointerKind::Press(PointerButton::Primary),
    PointerKind::Release(PointerButton::Primary),
] {
    ui.pointer_input(PointerEvent::new(middle, kind))?;
}
assert_eq!(clicks.get()?, 1);
```

##### [§](#errors-33)Errors

Propagates pending update/layout errors, callback errors (including
those of the flush that delivers a notification the event requested) and
phase restrictions. Older or external notification failures, and
collective settlement errors without a uniquely attributable request,
propagate without cancelling the current capture or drag. A failure
attributable to this dispatch cancels it. If a preflush prevents the
last held button’s release from reaching its capture owner, cleanup
still ends that owner’s gesture. A previous recipient’s focus-loss error
is returned after delivering the press to its new target, without
canceling that target’s successful capture.

[Source](../src/pert_ui_runtime/runtime/reveal.rs.html#308)[§](#impl-Runtime-18)

### impl [Runtime](struct.Runtime.html "struct pert_ui::Runtime")

[Source](../src/pert_ui_runtime/runtime/reveal.rs.html#312)

#### pub const [MAX_REVEAL_REQUESTS](#associatedconstant.MAX_REVEAL_REQUESTS): [usize](https://doc.rust-lang.org/1.99.1/std/primitive.usize.html) = 16

Maximum number of reveals queued with
[`EventContext::request_reveal`](struct.EventContext.html#method.request_reveal "method pert_ui::EventContext::request_reveal")
and
[`EventContext::request_reveal_and_focus`](struct.EventContext.html#method.request_reveal_and_focus "method pert_ui::EventContext::request_reveal_and_focus")
waiting for a flush. A request beyond it is refused with
[`RuntimeError::RevealQueueFull`](enum.RuntimeError.html#variant.RevealQueueFull "variant pert_ui::RuntimeError::RevealQueueFull").

[Source](../src/pert_ui_runtime/runtime/reveal.rs.html#371)

#### pub fn [reveal](#method.reveal)( &mut self, target: &[WidgetId](struct.WidgetId.html "struct pert_ui::WidgetId"), ) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[RevealOutcome](enum.RevealOutcome.html "enum pert_ui::RevealOutcome"), [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\>

Makes `target` visible and scrolls its ancestors to show it, without
moving focus.

The runtime first flushes, then walks the target’s presentation
ancestors from the outermost inwards. Where a container’s
[`Component::child_visible`](trait.Component.html#method.child_visible "method pert_ui::Component::child_visible")
hides the next component on the path, it asks that container through
[`Component::reveal_child`](trait.Component.html#method.reveal_child "method pert_ui::Component::reveal_child"),
applies the change and settles update and layout before going on,
revalidating the same
[`WidgetId`](struct.WidgetId.html "struct pert_ui::WidgetId") after
every step. Scrolling then uses
[`Component::reveal`](trait.Component.html#method.reveal "method pert_ui::Component::reveal")
from the innermost scrolling ancestor outwards, as focus does. A target
in another native window than the selected one gets a requested
activation of its window, reported in the outcome.

The walk refuses, as a
[`RevealOutcome::Refused`](enum.RevealOutcome.html#variant.Refused "variant pert_ui::RevealOutcome::Refused"),
an explicitly hidden component on the path
([`RevealRefusal::Hidden`](enum.RevealRefusal.html#variant.Hidden "variant pert_ui::RevealRefusal::Hidden")),
a container that will not show it, application-controlled visibility,
and a target an open modal surface blocks; admission is checked before
anything opens. Containers opened before a refusal stay open. A target
that opening the path unmounts ends as
[`RevealOutcome::Unmounted`](enum.RevealOutcome.html#variant.Unmounted "variant pert_ui::RevealOutcome::Unmounted").

The selected window is the same afterwards, unless it was closed during
the walk; the target’s window is then selected.

``` rust
use pert_ui::prelude::*;
use pert_ui::testing::*;
use pert_ui::{RevealOutcome, Tab, Tabs};

let mut ui = Runtime::new();
ui.set_viewport(Viewport::new(400.0, 300.0, 1.0)?)?;
let tabs = ui.mount_tree(Element::new(
    "tabs",
    Tabs::new([
        Tab::new("general", "General", Element::new("general-text", Label::new("General"))),
        Tab::new("details", "Details", Element::new("details-text", Label::new("Details"))),
    ]),
))?;
ui.flush(std::time::Duration::ZERO)?;
let shown = |ui: &Runtime, key: &str| {
    ui.inspect().components.iter().any(|c| c.key == key && c.visible)
};
// The second tab's content is mounted, with its state, but hidden.
let details = ui.find_descendant(&tabs, "details-text")?.ok_or("not mounted")?;
assert!(!shown(&ui, "details-text"));

let outcome = ui.reveal(&details)?;
assert_eq!(outcome, RevealOutcome::Revealed { activation: None });
assert!(shown(&ui, "details-text"));
```

##### [§](#errors-34)Errors

Returns `ForeignRuntime` or `Unmounted` for a target that is not a
mounted component of this runtime when called, the runtime
phase/shutdown error, and errors of the flushes, of
[`Component::reveal_child`](trait.Component.html#method.reveal_child "method pert_ui::Component::reveal_child")
and of
[`Component::reveal`](trait.Component.html#method.reveal "method pert_ui::Component::reveal").

[Source](../src/pert_ui_runtime/runtime/reveal.rs.html#421)

#### pub fn [reveal_and_focus](#method.reveal_and_focus)( &mut self, target: &[WidgetId](struct.WidgetId.html "struct pert_ui::WidgetId"), ) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[RevealOutcome](enum.RevealOutcome.html "enum pert_ui::RevealOutcome"), [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\>

Reveals `target` as
[`Self::reveal`](struct.Runtime.html#method.reveal "method pert_ui::Runtime::reveal")
does, then focuses it.

A target that is not focusable, such as a `Group`, is revealed without
claiming focus and reports
[`RevealOutcome::Revealed`](enum.RevealOutcome.html#variant.Revealed "variant pert_ui::RevealOutcome::Revealed").
Modal admission is checked again before focus. Focus moves without
further scrolling, since the target has just been scrolled into view.

“Search the UI and jump there” is built from commands rather than a
destination registry: the application registers one command per
destination, whose action reveals and focuses it, and lists them with
[`Runtime::commands_for`](struct.Runtime.html#method.commands_for "method pert_ui::Runtime::commands_for").

``` rust
use pert_ui::prelude::*;
use pert_ui::testing::*;
use pert_ui::{CommandOutcome, CommandScope, CommandSource, Tab, Tabs};
use std::time::Duration;

let mut ui = Runtime::new();
ui.set_viewport(Viewport::new(400.0, 300.0, 1.0)?)?;
let tabs = ui.mount_tree(Element::new(
    "tabs",
    Tabs::new([
        Tab::new("home", "Home", Element::new("welcome", Label::new("Welcome"))),
        Tab::new("mixer", "Mixer", Element::new("master", Button::new("Master", |_| Ok(())))),
    ]),
))?;
ui.flush(Duration::ZERO)?;
let master = ui.find_descendant(&tabs, "master")?.ok_or("not mounted")?;
let target = master.clone();
let go = ui.register_command("Go to: Mixer", CommandScope::Application, vec![],
    move |ui, _| ui.reveal_and_focus(&target).map(|_| ()))?;

let context = ui.command_context(CommandSource::Programmatic);
assert_eq!(ui.invoke_command(&go, context)?, CommandOutcome::Invoked);
assert_eq!(ui.focused(), Some(master));
```

##### [§](#errors-35)Errors

The errors of
[`Self::reveal`](struct.Runtime.html#method.reveal "method pert_ui::Runtime::reveal"),
and the callback errors
[`Self::focus`](struct.Runtime.html#method.focus "method pert_ui::Runtime::focus")
forwards after a transfer, from `Component::focus_lost` and
`Component::focus_within_changed` (the first returned, later ones
queued), returned after the transfer completes.

[Source](../src/pert_ui_runtime/runtime/reveal.rs.html#465-469)

#### pub fn [reveal_child](#method.reveal_child)( &mut self, container: &[WidgetId](struct.WidgetId.html "struct pert_ui::WidgetId"), key: &[str](https://doc.rust-lang.org/1.99.1/std/primitive.str.html), ) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[ChildReveal](enum.ChildReveal.html "enum pert_ui::ChildReveal"), [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\>

Asks `container` to show its immediate child `key` through
[`Component::reveal_child`](trait.Component.html#method.reveal_child "method pert_ui::Component::reveal_child"),
the same transition a reveal uses, and settles update and layout.

This is how application code selects a tab by key:
`ui.reveal_child(&tabs, "mixer")` selects the tab exactly as a click on
its title does, so workspace capture records it, and it requests
activation of the tab’s window when the tab is detached. A child that is
already shown returns
[`ChildReveal::AlreadyVisible`](enum.ChildReveal.html#variant.AlreadyVisible "variant pert_ui::ChildReveal::AlreadyVisible")
without asking the container. Focus does not move.

``` rust
use pert_ui::{component::*, prelude::*};
use pert_ui::testing::*;
use pert_ui::{Tab, Tabs};

let mut ui = Runtime::new();
ui.set_viewport(Viewport::new(400.0, 300.0, 1.0)?)?;
let tabs = ui.mount_tree(Element::new(
    "tabs",
    Tabs::new([
        Tab::new("general", "General", Element::new("text", Label::new("General"))),
        Tab::new("mixer", "Mixer", Element::new("text", Label::new("Mixer"))),
    ]),
))?;
ui.flush(std::time::Duration::ZERO)?;

assert_eq!(ui.reveal_child(&tabs, "mixer")?, ChildReveal::Changed);
assert_eq!(ui.reveal_child(&tabs, "mixer")?, ChildReveal::AlreadyVisible);
```

##### [§](#errors-36)Errors

Returns `ForeignRuntime` or `Unmounted` for a container that is not a
mounted component of this runtime, `ModalBlocked` for one an open modal
surface blocks,
[`LayoutError::UnknownKey`](enum.LayoutError.html#variant.UnknownKey "variant pert_ui::LayoutError::UnknownKey")
when it has no immediate child `key`, the runtime phase/shutdown error,
and errors of the flushes and of the hook.

[Source](../src/pert_ui_runtime/runtime/settings.rs.html#540)[§](#impl-Runtime-19)

### impl [Runtime](struct.Runtime.html "struct pert_ui::Runtime")

[Source](../src/pert_ui_runtime/runtime/settings.rs.html#542)

#### pub fn [ui_settings](#method.ui_settings)(&self) -\> [UiSettings](struct.UiSettings.html "struct pert_ui::UiSettings")

Shared presentation settings. Default zoom is 100%, independent of
theme/DPI.

[Source](../src/pert_ui_runtime/runtime/settings.rs.html#551)

#### pub fn [viewport](#method.viewport)(&self) -\> [Viewport](struct.Viewport.html "struct pert_ui::Viewport")

Effective logical UI extent and physical pixels per logical UI pixel,
including user zoom. Custom hosts use its scale for pointer/pixel-wheel
conversion, and supply unzoomed native geometry to `set_viewport`.

[Source](../src/pert_ui_runtime/runtime/tasks.rs.html#489)[§](#impl-Runtime-20)

### impl [Runtime](struct.Runtime.html "struct pert_ui::Runtime")

[Source](../src/pert_ui_runtime/runtime/tasks.rs.html#493)

#### pub const [MAX_TASKS](#associatedconstant.MAX_TASKS): [usize](https://doc.rust-lang.org/1.99.1/std/primitive.usize.html) = 64

Maximum number of accepted, undelivered application operations, across
[`EventContext::defer`](struct.EventContext.html#method.defer "method pert_ui::EventContext::defer"),
[`EventContext::defer_while_shown`](struct.EventContext.html#method.defer_while_shown "method pert_ui::EventContext::defer_while_shown")
and
[`Self::defer`](struct.Runtime.html#method.defer "method pert_ui::Runtime::defer").

[Source](../src/pert_ui_runtime/runtime/tasks.rs.html#508-511)

#### pub fn [defer](#method.defer)( &self, action: impl [FnOnce](https://doc.rust-lang.org/1.99.1/core/ops/function/trait.FnOnce.html "trait core::ops::function::FnOnce")(&mut [Runtime](struct.Runtime.html "struct pert_ui::Runtime")) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[()](https://doc.rust-lang.org/1.99.1/std/primitive.unit.html), [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\> + 'static, ) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[TaskReceipt](struct.TaskReceipt.html "struct pert_ui::TaskReceipt"), [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\>

Queues an application-owned operation, independent of a widget lifetime.
Delivery, receipts, errors and capacity match
[`EventContext::defer`](struct.EventContext.html#method.defer "method pert_ui::EventContext::defer"),
including that only runtime shutdown cancels it before delivery, that
nothing cancels it once it executes and that its
[`TaskReceipt`](struct.TaskReceipt.html "struct pert_ui::TaskReceipt")
may be dropped. The selection is not changed: the operation runs with
whichever window the delivering flush has selected; select one
explicitly for window-specific work.

Timing class: [at the next
flush](https://github.com/pertyjons/pert-ui/blob/main/docs/contracts.md#at-the-next-flush).

##### [§](#errors-37)Errors

Returns `TaskQueueFull` or the runtime phase/shutdown error, `Closed`
from the moment
[`Self::shutdown`](struct.Runtime.html#method.shutdown "method pert_ui::Runtime::shutdown")
begins.

[Source](../src/pert_ui_runtime/runtime/theme.rs.html#1260)[§](#impl-Runtime-21)

### impl [Runtime](struct.Runtime.html "struct pert_ui::Runtime")

[Source](../src/pert_ui_runtime/runtime/theme.rs.html#1263)

#### pub fn [theme](#method.theme)(&self) -\> [ThemeManager](struct.ThemeManager.html "struct pert_ui::ThemeManager")

Returns the runtime’s shared theme manager, initially dark with built-in
palettes. Retain a clone in event callbacks to change modes or role
overrides at runtime.

[Source](../src/pert_ui_runtime/runtime/tree.rs.html#79)[§](#impl-Runtime-22)

### impl [Runtime](struct.Runtime.html "struct pert_ui::Runtime")

[Source](../src/pert_ui_runtime/runtime/tree.rs.html#99)

#### pub fn [mount_tree](#method.mount_tree)(&mut self, element: [Element](struct.Element.html "struct pert_ui::Element")) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[WidgetId](struct.WidgetId.html "struct pert_ui::WidgetId"), [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\>

Mounts an owned component tree with sibling-scoped stable keys.

Keys/identity capacity are validated before callbacks. Mount callbacks
run parent first; teardown runs children first in logical sibling order.
If a mount callback panics, completed mounts are cleaned up before
unwinding resumes. New handles never reuse a removed incarnation. No
callbacks may panic during teardown. First update/layout is scheduled
automatically.

##### [§](#errors-38)Errors

Nothing is mounted after an error: a runtime phase error (see
[`Runtime`](struct.Runtime.html "struct pert_ui::Runtime"));
[`RuntimeError::EmptyKey`](enum.RuntimeError.html#variant.EmptyKey "variant pert_ui::RuntimeError::EmptyKey")
for a key without a non-whitespace character anywhere in the tree;
[`RuntimeError::DuplicateKey`](enum.RuntimeError.html#variant.DuplicateKey "variant pert_ui::RuntimeError::DuplicateKey")
for siblings sharing a key or a root key already in use; the error a
component’s
[`Component::check_description`](trait.Component.html#method.check_description "method pert_ui::Component::check_description")
returns, conventionally
[`RuntimeError::Layout`](enum.RuntimeError.html#variant.Layout "variant pert_ui::RuntimeError::Layout")
with
[`LayoutError::InvalidChild`](enum.LayoutError.html#variant.InvalidChild "variant pert_ui::LayoutError::InvalidChild");
or
[`RuntimeError::Exhausted`](enum.RuntimeError.html#variant.Exhausted "variant pert_ui::RuntimeError::Exhausted")
when the tree needs more identities than remain.

[Source](../src/pert_ui_runtime/runtime/tree.rs.html#143)

#### pub fn [key](#method.key)(&self, id: &[WidgetId](struct.WidgetId.html "struct pert_ui::WidgetId")) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<&[str](https://doc.rust-lang.org/1.99.1/std/primitive.str.html), [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\>

Reads a mounted component’s stable sibling key. The key is for logical
structure discovery; retain `WidgetId` for event targets.

Every component has a key, but not every key is an address segment: a
structural wrapper’s key is no segment at all, and a slot’s or a part’s
segment is its slot or part name, not this key. The segment
[`Self::find`](struct.Runtime.html#method.find "method pert_ui::Runtime::find")
takes for each component one level below a parent is what
[`Driver::address_children`](testing/struct.Driver.html#method.address_children "method pert_ui::testing::Driver::address_children")
lists.

##### [§](#errors-39)Errors

Returns `ForeignRuntime` for an ID of another runtime and `Unmounted`
for an unmounted one.

[Source](../src/pert_ui_runtime/runtime/tree.rs.html#163)

#### pub fn [children](#method.children)(&self, parent: &[WidgetId](struct.WidgetId.html "struct pert_ui::WidgetId")) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[Vec](https://doc.rust-lang.org/1.99.1/alloc/vec/struct.Vec.html "struct alloc::vec::Vec")\<[WidgetId](struct.WidgetId.html "struct pert_ui::WidgetId")\>, [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\>

Returns ordered immediate children, including locally or ancestrally
hidden ones.

These are the logical children in description order, structure included:
a structural wrapper, such as an
[`Each`](struct.Each.html "struct pert_ui::Each")’s default container or
a [`Table`](struct.Table.html "struct pert_ui::Table")’s body, is listed
although addresses skip it. A virtualized collection, such as a
[`VirtualList`](struct.VirtualList.html "struct pert_ui::VirtualList"),
lists the rows it has mounted, in index order, including the offscreen
ones it retains; a row that is not mounted is not listed. This is the
structure walk for hosts and custom components. For the components one
address segment below a parent, as
[`Self::find`](struct.Runtime.html#method.find "method pert_ui::Runtime::find")
resolves them, a test uses
[`Driver::address_children`](testing/struct.Driver.html#method.address_children "method pert_ui::testing::Driver::address_children").

##### [§](#errors-40)Errors

Returns
[`RuntimeError::ForeignRuntime`](enum.RuntimeError.html#variant.ForeignRuntime "variant pert_ui::RuntimeError::ForeignRuntime")
for a parent of another runtime and
[`RuntimeError::Unmounted`](enum.RuntimeError.html#variant.Unmounted "variant pert_ui::RuntimeError::Unmounted")
for an unmounted one.

[Source](../src/pert_ui_runtime/runtime/tree.rs.html#228-232)

#### pub fn [find_descendant](#method.find_descendant)( &self, ancestor: &[WidgetId](struct.WidgetId.html "struct pert_ui::WidgetId"), key: &[str](https://doc.rust-lang.org/1.99.1/std/primitive.str.html), ) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[Option](https://doc.rust-lang.org/1.99.1/core/option/enum.Option.html "enum core::option::Option")\<[WidgetId](struct.WidgetId.html "struct pert_ui::WidgetId")\>, [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\>

Finds the unique descendant of `ancestor` whose key is `key`, at any
depth below it.

Walking
[`Self::children`](struct.Runtime.html#method.children "method pert_ui::Runtime::children")
and
[`Self::key`](struct.Runtime.html#method.key "method pert_ui::Runtime::key")
level by level needs every key on the way down, including the keys that
built-in composites give the parts they create, such as a dialog’s
anchor, a scroll area’s viewport or a group’s content. This lookup names
only the key the application chose, so a host can find its own component
without knowing how the built-ins in between are composed. Keys are
unique only among siblings, so the lookup never picks one of several
matches: if more than one descendant carries `key`, it fails with
[`RuntimeError::AmbiguousKey`](enum.RuntimeError.html#variant.AmbiguousKey "variant pert_ui::RuntimeError::AmbiguousKey").
Narrow the search by first finding a uniquely keyed ancestor and
searching below it, name the addressable path with
[`Self::find`](struct.Runtime.html#method.find "method pert_ui::Runtime::find"),
or walk
[`Self::children`](struct.Runtime.html#method.children "method pert_ui::Runtime::children")
and
[`Self::key`](struct.Runtime.html#method.key "method pert_ui::Runtime::key").
A common cause is a container keyed like the control it holds, such as a
[`ToolGroup`](struct.ToolGroup.html "struct pert_ui::ToolGroup") named
after its only control, or table content keyed like its column, below
the cell the table keys by the column
([`Table`](struct.Table.html#addresses-and-tests "struct pert_ui::Table"));
give the container a key of its own, or name the full address with
[`Self::find`](struct.Runtime.html#method.find "method pert_ui::Runtime::find").
`ancestor` itself is not a candidate.

It follows logical children, including hidden ones and those presented
in a detached window, exactly as
[`Self::children`](struct.Runtime.html#method.children "method pert_ui::Runtime::children")
lists them. It always visits the whole subtree below `ancestor` to rule
out a second match, so the cost grows with the subtree’s size; retain
the returned `WidgetId` rather than looking it up for every event.

##### [§](#errors-41)Errors

Rejects a foreign or unmounted `ancestor` and returns
[`RuntimeError::AmbiguousKey`](enum.RuntimeError.html#variant.AmbiguousKey "variant pert_ui::RuntimeError::AmbiguousKey")
when several descendants carry `key`. No matching descendant is
`Ok(None)`, not an error.

``` rust
use pert_ui::prelude::*;

let mut ui = Runtime::new();
let root = ui.mount_tree(Element::new("root", Column::new([
    Element::new("left", Row::new([Element::new("apply", Label::new("Apply"))])),
    Element::new("right", Row::new([
        Element::new("apply", Label::new("Apply")),
        Element::new("reset", Label::new("Reset")),
    ])),
])))?;
let reset = ui.find_descendant(&root, "reset")?.ok_or("not mounted")?;
assert_eq!(ui.key(&reset)?, "reset");
assert!(ui.find_descendant(&root, "missing")?.is_none());
// Two descendants are keyed "apply": the lookup refuses to choose.
assert_eq!(
    ui.find_descendant(&root, "apply"),
    Err(RuntimeError::AmbiguousKey("apply".into()))
);
let right = ui.find_descendant(&root, "right")?.ok_or("not mounted")?;
assert!(ui.find_descendant(&right, "apply")?.is_some());
```

[Source](../src/pert_ui_runtime/runtime/tree.rs.html#319-323)

#### pub fn [find](#method.find)( &self, root: &[WidgetId](struct.WidgetId.html "struct pert_ui::WidgetId"), segments: &\[&[str](https://doc.rust-lang.org/1.99.1/std/primitive.str.html)\], ) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[Option](https://doc.rust-lang.org/1.99.1/core/option/enum.Option.html "enum core::option::Option")\<[WidgetId](struct.WidgetId.html "struct pert_ui::WidgetId")\>, [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\>

Finds the descendant of `root` at an addressable path below it, skipping
the structure built-in composites create; an empty path returns `root`
itself.

This is the lookup for a component a description mounted, such as the
target of
[`Self::focus`](struct.Runtime.html#method.focus "method pert_ui::Runtime::focus"),
[`Self::request_detach`](struct.Runtime.html#method.request_detach "method pert_ui::Runtime::request_detach")
or a view-scoped command, without walking
[`Self::children`](struct.Runtime.html#method.children "method pert_ui::Runtime::children")
and
[`Self::key`](struct.Runtime.html#method.key "method pert_ui::Runtime::key")
by hand, and the path it takes is the one inspection reports
([`ComponentInspection::path`](struct.ComponentInspection.html#structfield.path "field pert_ui::ComponentInspection::path"))
and
[`Selector::path`](enum.Selector.html#method.path "associated function pert_ui::Selector::path")
matches. An inspection path starts with the root’s own key, so pass the
rest of it: `find(&root, &path[1..])`. Built-in containers add a segment
only where they document one, such as
[`RegionLayout`](struct.RegionLayout.html "struct pert_ui::RegionLayout")’s
region names `top`, `bottom`, `left`, `right` and `center`.

The segments follow the rules of [workspace
persistence](https://github.com/pertyjons/pert-ui/blob/main/docs/contracts.md#workspace-persistence)
and inspection’s `Selector::path`: an element contributes its key, a
slot contributes its slot name, a part contributes its part name for
itself only, and structural elements contribute nothing. `segments` is
the address relative to `root`: the segments contributed by the elements
strictly below `root` on the way down to the target, so a titled group’s
content is `["form"]` below the group’s key, not the frame and body
levels a walk of
[`Self::children`](struct.Runtime.html#method.children "method pert_ui::Runtime::children")
would have to name. The match is the element that introduces the last
segment; structural wrappers beneath it share its address but are not
matches. A drag preview never matches.

Unlike
[`Self::find_descendant`](struct.Runtime.html#method.find_descendant "method pert_ui::Runtime::find_descendant"),
every segment on the way down is named, so two application keys that
repeat in different containers are told apart by their containers’ keys.
Structural levels can still make one address reach several elements,
such as two structural wrappers each holding a child keyed alike; the
lookup then fails with
[`RuntimeError::AmbiguousKey`](enum.RuntimeError.html#variant.AmbiguousKey "variant pert_ui::RuntimeError::AmbiguousKey")
rather than picking one.

It follows logical children, including hidden ones and those presented
in a detached window, exactly as
[`Self::children`](struct.Runtime.html#method.children "method pert_ui::Runtime::children")
lists them. It visits every descendant reachable through structural
levels and matching segments, so the cost grows with that part of the
subtree; retain the returned `WidgetId` rather than looking it up for
every event.

##### [§](#errors-42)Errors

Returns
[`RuntimeError::ForeignRuntime`](enum.RuntimeError.html#variant.ForeignRuntime "variant pert_ui::RuntimeError::ForeignRuntime")
for a `root` of another runtime,
[`RuntimeError::Unmounted`](enum.RuntimeError.html#variant.Unmounted "variant pert_ui::RuntimeError::Unmounted")
for an unmounted one and
[`RuntimeError::AmbiguousKey`](enum.RuntimeError.html#variant.AmbiguousKey "variant pert_ui::RuntimeError::AmbiguousKey")
when several descendants have the address. An address that does not
resolve is `Ok(None)`, not an error.

``` rust
use pert_ui::prelude::*;

let mut ui = Runtime::new();
let root = ui.mount_tree(Element::new("root", Column::new([
    Element::new("left", Group::titled("Left", Element::new(
        "form",
        Row::new([Element::new("apply", Label::new("Apply"))]),
    ))),
    Element::new("right", Row::new([Element::new("apply", Label::new("Apply"))])),
])))?;
// The group's own frame and body levels are not part of the address.
let apply = ui.find(&root, &["left", "form", "apply"])?.ok_or("not mounted")?;
assert_eq!(ui.key(&apply)?, "apply");
// `find_descendant(&root, "apply")` would be ambiguous; an address is not.
assert!(ui.find(&root, &["right", "apply"])?.is_some());
assert!(ui.find(&root, &["left", "apply"])?.is_none());
assert_eq!(ui.find(&root, &[])?, Some(root.clone()));
```

[Source](../src/pert_ui_runtime/runtime/tree.rs.html#413)

#### pub fn [bounds](#method.bounds)(&self, id: &[WidgetId](struct.WidgetId.html "struct pert_ui::WidgetId")) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[Option](https://doc.rust-lang.org/1.99.1/core/option/enum.Option.html "enum core::option::Option")\<[Rect](struct.Rect.html "struct pert_ui::Rect")\>, [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\>

Returns the most recent allocated window-coordinate bounds, if laid out.

Bounds may describe the last successful layout while a new layout is
pending, and are absent before the first. Hidden components retain their
last bounds.

##### [§](#errors-43)Errors

Returns `ForeignRuntime` for an ID of another runtime and `Unmounted`
for an unmounted one.

[Source](../src/pert_ui_runtime/runtime/tree.rs.html#453)

#### pub fn [inspection_id](#method.inspection_id)(&self, id: &[WidgetId](struct.WidgetId.html "struct pert_ui::WidgetId")) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[u64](https://doc.rust-lang.org/1.99.1/std/primitive.u64.html), [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\>

Returns the
[`ComponentInspection::id`](struct.ComponentInspection.html#structfield.id "field pert_ui::ComponentInspection::id")
of a mounted component, so a test or tool finds that component’s entry
in a snapshot of this runtime without matching keys.

The value names this incarnation only: it is unique within the runtime,
is never reused for another component, and a component mounted again
under the same key gets a new one. It means nothing in another runtime’s
snapshot.

##### [§](#errors-44)Errors

Returns `ForeignRuntime` for an ID of another runtime and `Unmounted`
for an unmounted one.

One component’s entry is one
[`Self::query`](struct.Runtime.html#method.query "method pert_ui::Runtime::query")
with
[`Selector::id`](enum.Selector.html#method.id "associated function pert_ui::Selector::id"),
a single lookup, rather than a search of a whole
[`Self::inspect`](struct.Runtime.html#method.inspect "method pert_ui::Runtime::inspect")
snapshot:

``` rust
use pert_ui::{Button, Column, Detail, Element, InspectionQuery, Runtime, Selector};
let mut ui = Runtime::new();
let root = ui.mount_tree(Element::new("root", Column::new([
    Element::new("save", Button::new("Save", |_| Ok(()))),
])))?;
let save = ui.find(&root, &["save"])?.ok_or("not mounted")?;
let id = ui.inspection_id(&save)?;
let result = ui.query(
    &InspectionQuery::components(Selector::id(id)).detail(Detail::Semantics),
);
let entry = result.components.first().ok_or("no entry")?;
assert_eq!(entry.key, "save");
let semantics = entry.details.semantics.as_ref().ok_or("no semantics")?;
assert_eq!(semantics.name.as_deref(), Some("Save"));
```

[Source](../src/pert_ui_runtime/runtime/tree.rs.html#488-492)

#### pub fn [reconcile_children](#method.reconcile_children)( &mut self, parent: &[WidgetId](struct.WidgetId.html "struct pert_ui::WidgetId"), children: [Vec](https://doc.rust-lang.org/1.99.1/alloc/vec/struct.Vec.html "struct alloc::vec::Vec")\<[Element](struct.Element.html "struct pert_ui::Element")\>, ) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[()](https://doc.rust-lang.org/1.99.1/std/primitive.unit.html), [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\>

Reconciles a composed parent’s children by key and concrete component
type.

Surviving components retain their state, subscriptions and identity.
Their new descriptions are applied through
[`Component::reconcile`](trait.Component.html#method.reconcile "method pert_ui::Component::reconcile"),
which separates replaceable properties from retained local state.
Descendants are reconciled recursively, except for components opting
into
[`Component::retain_initial_children`](trait.Component.html#method.retain_initial_children "method pert_ui::Component::retain_initial_children"):
their incoming descendants are discarded before admission while their
mounted children stay. Display labels update independently of retained
component values. New mounts complete before removals/order changes; a
mounting panic cleans new mounts without changing surviving topology.
Removing children cancels edits and subscriptions recursively. A
surviving component that has requested its own children through
[`UpdateContext::request_children`](struct.UpdateContext.html#method.request_children "method pert_ui::UpdateContext::request_children"),
such as a `VirtualList`, keeps them: only its own description is
applied.

A child that was moved deeper into the subtree of the parent describing
it, as a container moves its content into its own overflow surface, is
matched by key where it is now (see
[`Self::reparent`](struct.Runtime.html#method.reparent "method pert_ui::Runtime::reparent")):
it keeps its owner, position and state, its new description is applied
there, and a description that omits it unmounts it. The owner holding it
keeps it through that owner’s own descriptions, which do not list it.

##### [§](#errors-45)Errors

Rejects invalid IDs, duplicate/empty keys (including a key that names a
child moved here from a parent that still describes it), changed
surviving component types, identity exhaustion, children described for a
surviving component that manages its own (`LayoutError::InvalidChild`,
except for
[`Component::retain_initial_children`](trait.Component.html#method.retain_initial_children "method pert_ui::Component::retain_initial_children"))
or forbidden phases before mutation.

[Source](../src/pert_ui_runtime/runtime/tree.rs.html#734-739)

#### pub fn [reparent](#method.reparent)( &mut self, child: &[WidgetId](struct.WidgetId.html "struct pert_ui::WidgetId"), parent: &[WidgetId](struct.WidgetId.html "struct pert_ui::WidgetId"), index: [usize](https://doc.rust-lang.org/1.99.1/std/primitive.usize.html), ) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[()](https://doc.rust-lang.org/1.99.1/std/primitive.unit.html), [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\>

Moves a mounted subtree to another owner, or to another position under
its current owner, without remounting it.

The subtree keeps its incarnations, keys, component values, local state,
subscriptions, parameter edits, pointer capture and keyboard focus: this
is a change of ownership, not a rebuild. `index` is the insertion
position among `parent`’s current children and is clamped to their
count; it decides layout and keyboard navigation order exactly like a
mounted position. Both owners and the moved subtree are scheduled for
layout.

Visibility is reevaluated against the new owner: a parent whose
`Component::child_visible` rejects the key, or that is itself hidden,
hides the subtree with the ordinary hiding rules, which cancel its
gestures, drafts and held actions. Moving into a visible owner shows it
again with its retained state. The new owner’s
`Component::child_surface` decides promotion as it does for a mounted
child.

The move also decides which description lists the subtree. Moved to an
owner inside the subtree of the parent whose description lists it, such
as a container’s own overflow surface, it stays described by that
parent:
[`Self::reconcile_children`](struct.Runtime.html#method.reconcile_children "method pert_ui::Runtime::reconcile_children")
on the parent matches it by key where it is now, applies its new
description there and unmounts it when the description omits it, while
descriptions of the owner holding it neither list nor remove it. Moved
anywhere else, `parent` describes it from then on, so a later
description of `parent` must list it to keep it and one of its former
parent no longer does. Moving it back into the describing parent’s
subtree keeps that parent. An owner that is itself unmounted takes the
moved children it holds with it.

``` rust
use pert_ui::{Column, Element, Label, Runtime};
let mut ui = Runtime::new();
let root = ui.mount_tree(Element::new("root", Column::new([
    Element::new("strip", Column::new([Element::new("tools", Label::new("Tools"))])),
    Element::new("overflow", Column::new([])),
])))?;
let slots = ui.children(&root)?;
let (strip, overflow) = (slots[0].clone(), slots[1].clone());
let tools = ui.children(&strip)?[0].clone();
ui.reparent(&tools, &overflow, 0)?;
assert_eq!(ui.children(&overflow)?, [tools.clone()]);
assert!(ui.children(&strip)?.is_empty());
ui.reparent(&tools, &strip, 0)?; // the same subtree returns
assert_eq!(ui.children(&strip)?, [tools]);
```

##### [§](#errors-46)Errors

Rejects foreign or unmounted handles, a parent inside the moved subtree
(`LayoutError::InvalidChild`), and a key already used by another child
of `parent` or by a child `parent` describes elsewhere (`DuplicateKey`).
Returns `NotMovable` when the move would leave the subtree’s native
presentation root, when the subtree contains a detached view, when the
moved component is itself a promoted surface root, whose presentation is
anchored to its current owner, or when the subtree holds a component
moved into it that would leave the subtree of the parent describing it.

[Source](../src/pert_ui_runtime/runtime/workspace.rs.html#19)[§](#impl-Runtime-23)

### impl [Runtime](struct.Runtime.html "struct pert_ui::Runtime")

[Source](../src/pert_ui_runtime/runtime/workspace.rs.html#64)

#### pub fn [capture_workspace](#method.capture_workspace)(&self) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[Workspace](struct.Workspace.html "struct pert_ui::Workspace"), [WorkspaceError](enum.WorkspaceError.html "enum pert_ui::WorkspaceError")\>

Captures the restorable arrangement of everything mounted in this
runtime.

The result is an owned document with no live handles: it may outlive the
components it came from and be stored, compared or discarded freely.
[`Workspace::encode`](struct.Workspace.html#method.encode "method pert_ui::Workspace::encode")
turns it into the text a host writes wherever it keeps such things; this
library never chooses that place, writes a file or saves on a timer.

Capture is explicit and synchronous. It schedules nothing, invalidates
nothing and measures nothing, and signal or parameter writes are
rejected while it runs. Entries are ordered by path, so capturing an
unchanged workspace twice produces identical text.

##### [§](#errors-47)Errors

Returns
[`WorkspaceError::Runtime`](enum.WorkspaceError.html#variant.Runtime "variant pert_ui::WorkspaceError::Runtime")
after shutdown, and a document error when the capture would exceed a
documented limit, when a participant’s path is not a valid
`WorkspacePath` (for example a key longer than a segment may be), or
when a participant shares its path with another component, which means a
container declared its slots incorrectly. Other components’ keys are not
validated: they are never written. A participant that cannot build its
state fails the whole capture with
[`WorkspaceError::Participant`](enum.WorkspaceError.html#variant.Participant "variant pert_ui::WorkspaceError::Participant"),
naming its path; a capture never succeeds without a participant that
failed.

``` rust
use pert_ui::{Element, Runtime, RuntimeError, SplitAxis, SplitPane, Label};
let mut ui = Runtime::new();
ui.mount_tree(Element::new(
    "main",
    SplitPane::new(
        SplitAxis::LeftRight,
        Element::new("left", Label::new("Left")),
        Element::new("right", Label::new("Right")),
    ),
))?;
let workspace = ui.capture_workspace()?;
assert_eq!(workspace.len(), 1);
let text = workspace.encode();
assert!(text.starts_with("pert-ui-workspace 2\n"));
```

[Source](../src/pert_ui_runtime/runtime/workspace.rs.html#122-125)

#### pub fn [restore_workspace](#method.restore_workspace)( &mut self, workspace: &[Workspace](struct.Workspace.html "struct pert_ui::Workspace"), ) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[WorkspaceReport](struct.WorkspaceReport.html "struct pert_ui::WorkspaceReport"), [WorkspaceError](enum.WorkspaceError.html "enum pert_ui::WorkspaceError")\>

Applies a decoded document to the mounted tree, reporting every entry.

Settings are applied first, then each entry in document order. An entry
whose view is not mounted yet is retained and applied when a matching
path mounts, at most once;
[`Runtime::workspace_report`](struct.Runtime.html#method.workspace_report "method pert_ui::Runtime::workspace_report")
reflects those later applications. A second restore replaces what the
first one retained.

While a drag session is active
([`Runtime::drag_active`](struct.Runtime.html#method.drag_active "method pert_ui::Runtime::drag_active"))
no entry is applied: a mounted participant’s entry is reported as
[`RestoreRejection::Busy`](enum.RestoreRejection.html#variant.Busy "variant pert_ui::RestoreRejection::Busy"),
and so is a retained entry whose view mounts during the drag. Either way
the entry is spent; restore again after the drag to apply it. The
document’s settings are not held back: zoom, theme mode and density
apply during a drag like at any other time. A changed zoom or
geometry-affecting density cancels captured gestures, focused held-key
actions and registered edits when geometry next settles. An active drag
ends with
[`DragOutcome::Cancelled`](enum.DragOutcome.html#variant.Cancelled "variant pert_ui::DragOutcome::Cancelled")
to its source. Density affects geometry when a visible layout reader
uses a changed metric. Unchanged settings keep gestures active. Hosts
defer changed settings when their interaction policy requires preserving
gestures.

Saved extents, proportions and offsets are preferences, not measured
results: they are written back as they were captured, and the next
layout clamps them against the viewport it then has. Restoring state for
a hidden view schedules no work for it; the view records the change when
revealed. Focus, pointer capture, gestures and drafts are never
restored, because they were never captured.

##### [§](#errors-48)Errors

Returns
[`WorkspaceError::Runtime`](enum.WorkspaceError.html#variant.Runtime "variant pert_ui::WorkspaceError::Runtime")
when the runtime is closed, when called during update, layout, paint,
inspection or delivery, or when a setting is refused. A refused setting
leaves the entries unapplied. A participant with an invalid or shared
mounted path, or a document entry whose path several mounted components
share, is rejected before settings or entries change, and so is a
mounted participant that cannot build its own state
([`WorkspaceError::Participant`](enum.WorkspaceError.html#variant.Participant "variant pert_ui::WorkspaceError::Participant")).

[Source](../src/pert_ui_runtime/runtime/workspace.rs.html#185)

#### pub fn [workspace_report](#method.workspace_report)(&self) -\> &[WorkspaceReport](struct.WorkspaceReport.html "struct pert_ui::WorkspaceReport")

What the last
[`Runtime::restore_workspace`](struct.Runtime.html#method.restore_workspace "method pert_ui::Runtime::restore_workspace")
did, including entries that applied later because their view mounted
after the restore.

Empty before the first restore.

[Source](../src/pert_ui_runtime/runtime.rs.html#3962)[§](#impl-Runtime-24)

### impl [Runtime](struct.Runtime.html "struct pert_ui::Runtime")

[Source](../src/pert_ui_runtime/runtime.rs.html#3968)

#### pub const [MAX_FLUSH_WAVES](#associatedconstant.MAX_FLUSH_WAVES): [usize](https://doc.rust-lang.org/1.99.1/std/primitive.usize.html) = 64

Maximum update/settlement waves in one flush, including internal
follow-ups.

Each wave evaluates the currently queued components once;
layout/focus/hover settle after updates drain. This bounds repeated
scheduling, not component count, callback duration or tree depth. A
callback must still return promptly.

[Source](../src/pert_ui_runtime/runtime.rs.html#4171)

#### pub const [MAX_OVERLAY_MARKERS](#associatedconstant.MAX_OVERLAY_MARKERS): [usize](https://doc.rust-lang.org/1.99.1/std/primitive.usize.html) = 64

Most marker outlines
[`DebugOverlay::Issues`](enum.DebugOverlay.html#variant.Issues "variant pert_ui::DebugOverlay::Issues")
draws in one window; the badge counts the rest.

[Source](../src/pert_ui_runtime/runtime.rs.html#4175)

#### pub const [MAX_OVERLAY_BOXES](#associatedconstant.MAX_OVERLAY_BOXES): [usize](https://doc.rust-lang.org/1.99.1/std/primitive.usize.html) = 1024

Most components
[`DebugOverlay::Boxes`](enum.DebugOverlay.html#variant.Boxes "variant pert_ui::DebugOverlay::Boxes")
outlines in one window.

[Source](../src/pert_ui_runtime/runtime.rs.html#4013)

#### pub fn [now](#method.now)(&self) -\> [Duration](https://doc.rust-lang.org/1.99.1/core/time/struct.Duration.html "struct core::time::Duration")

Current elapsed scheduler time, advanced by `flush`. Reading does not
capture inspection state, perform I/O or schedule work.

[Source](../src/pert_ui_runtime/runtime.rs.html#4088)

#### pub fn [set_profiling](#method.set_profiling)(&self, enabled: [bool](https://doc.rust-lang.org/1.99.1/std/primitive.bool.html)) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[()](https://doc.rust-lang.org/1.99.1/std/primitive.unit.html), [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\>

Enables/disables UI elapsed-time profiling, off by default. Pausing
retains aggregates but discards in-flight scopes. Adds no scheduled
work. Timing includes waiting/preemption; it is not CPU percentage or
GPU execution time.

##### [§](#errors-49)Errors

Returns
[`RuntimeError::Closed`](enum.RuntimeError.html#variant.Closed "variant pert_ui::RuntimeError::Closed")
after shutdown, and `WriteDuringUpdate`, `WriteDuringLayout`,
`WriteDuringPaint`, `WriteDuringInspection` or `WriteDuringDelivery`
when called from inside that phase.

[Source](../src/pert_ui_runtime/runtime.rs.html#4096)

#### pub fn [profile](#method.profile)(&self) -\> [ProfileSnapshot](struct.ProfileSnapshot.html "struct pert_ui::ProfileSnapshot")

Returns bounded owned timing data without scheduling work or measuring
the snapshot itself. Only completed scopes are included; see
`ProfileSnapshot`.

[Source](../src/pert_ui_runtime/runtime.rs.html#4105)

#### pub fn [clear_profile](#method.clear_profile)(&self) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[()](https://doc.rust-lang.org/1.99.1/std/primitive.unit.html), [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\>

Clears timing aggregates, recent samples and in-flight scopes,
preserving enablement.

##### [§](#errors-50)Errors

Returns
[`RuntimeError::Closed`](enum.RuntimeError.html#variant.Closed "variant pert_ui::RuntimeError::Closed")
after shutdown, and `WriteDuringUpdate`, `WriteDuringLayout`,
`WriteDuringPaint`, `WriteDuringInspection` or `WriteDuringDelivery`
when called from inside that phase.

[Source](../src/pert_ui_runtime/runtime.rs.html#4113)

#### pub fn [focused](#method.focused)(&self) -\> [Option](https://doc.rust-lang.org/1.99.1/core/option/enum.Option.html "enum core::option::Option")\<[WidgetId](struct.WidgetId.html "struct pert_ui::WidgetId")\>

Current component input focus, or `None` after context loss.

[Source](../src/pert_ui_runtime/runtime.rs.html#4136)

#### pub fn [validation_report](#method.validation_report)(&self) -\> [ValidationReport](struct.ValidationReport.html "struct pert_ui::ValidationReport")

Returns the latest development errors and per-component marker data.

Automatically populated after changed components are flushed, regardless
of diagnostic history mode. No callbacks or validation run when reading
this report. Default release builds always return a disabled empty
report.

Geometry is validated only for a native window whose latest layout ran
with a viewport that has width and height: content flushed before the
first
[`set_viewport`](struct.Runtime.html#method.set_viewport "method pert_ui::Runtime::set_viewport"),
or laid out while a window’s viewport has no area, reports no bounds,
clip, padding-exceeding-bounds, hit-area, parent or overlap issue, and
the overlay draws no boxes for it. An empty report then says nothing
about that geometry. Until the next layout after a viewport change, the
report keeps the last validation and is
[`pending`](struct.ValidationReport.html#structfield.pending "field pert_ui::ValidationReport::pending"),
which covers every window: a validation run while another window’s
layout is pending leaves that window’s geometry unchecked, and an issue
it had is not logged again when it reappears. Rules that do not depend
on geometry, such as keys, colors and shortcuts, apply in every window.

[Source](../src/pert_ui_runtime/runtime.rs.html#4161)

#### pub fn [set_debug_mode](#method.set_debug_mode)(&self, mode: [DebugMode](enum.DebugMode.html "enum pert_ui::DebugMode"))

Enables or disables bounded event-history recording.

Defaults to Off. Changing mode schedules no work and draws nothing; see
[`set_debug_overlay`](struct.Runtime.html#method.set_debug_overlay "method pert_ui::Runtime::set_debug_overlay")
for drawing. Off stops recording but retains existing history until
explicitly cleared or the runtime is dropped.

[Source](../src/pert_ui_runtime/runtime.rs.html#4186)

#### pub fn [set_debug_overlay](#method.set_debug_overlay)(&self, overlay: [DebugOverlay](enum.DebugOverlay.html "enum pert_ui::DebugOverlay"))

Selects the development drawing
[`paint`](struct.Runtime.html#method.paint "method pert_ui::Runtime::paint")
appends to every window’s scene. Defaults to
[`DebugOverlay::Off`](enum.DebugOverlay.html#variant.Off "variant pert_ui::DebugOverlay::Off").

A changed setting marks each window’s scene for one repaint and
schedules nothing else; setting the current value does nothing. While an
overlay is on, a completed validation pass that changes what it shows
marks the scene again, and nothing else does, so stable issues cost no
frames. The setting is accepted in every build, but only builds with
`debug_assertions` have validation data to draw.

[Source](../src/pert_ui_runtime/runtime.rs.html#4195)

#### pub fn [debug_overlay](#method.debug_overlay)(&self) -\> [DebugOverlay](enum.DebugOverlay.html "enum pert_ui::DebugOverlay")

The current development overlay setting.

[Source](../src/pert_ui_runtime/runtime.rs.html#4291-4295)

#### pub fn [try_event](#method.try_event)\<R, E\>( &mut self, id: &[WidgetId](struct.WidgetId.html "struct pert_ui::WidgetId"), event: impl [FnOnce](https://doc.rust-lang.org/1.99.1/core/ops/function/trait.FnOnce.html "trait core::ops::function::FnOnce")(&mut [EventContext](struct.EventContext.html "struct pert_ui::EventContext")) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<R, E\>, ) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<R, E\>, [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\>

where E:
[Display](https://doc.rust-lang.org/1.99.1/core/fmt/trait.Display.html "trait core::fmt::Display"),

Runs a fallible application event and records its reported
success/failure.

Timing class: [in the
input](https://github.com/pertyjons/pert-ui/blob/main/docs/contracts.md#in-the-input).

Unlike `event`, this can inspect the closure’s Result. Delivery failures
remain the outer runtime error; application failures remain the inner
error. Generic `event` records only `Dispatched`, not an invented
success result.

##### [§](#errors-51)Errors

Those of
[`Self::event`](struct.Runtime.html#method.event "method pert_ui::Runtime::event"):
target refusals before anything changes, and later focus-transfer errors
after focus has moved, in both cases without calling the closure or
recording its outcome.

[Source](../src/pert_ui_runtime/runtime.rs.html#4333)

#### pub fn [set_viewport](#method.set_viewport)(&self, viewport: [Viewport](struct.Viewport.html "struct pert_ui::Viewport")) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[()](https://doc.rust-lang.org/1.99.1/std/primitive.unit.html), [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\>

Supplies native logical window size and DPI scale before application
zoom. Changes invalidate layout at the next flush; active gestures
cancel before geometry changes. Do not pass an already zoomed viewport
back to this method.

A host presenting to a native surface builds the viewport with
[`Viewport::from_physical`](struct.Viewport.html#method.from_physical "associated function pert_ui::Viewport::from_physical"),
which retains the operating system’s physical extent; zoom leaves that
extent unchanged, and
[`Viewport::physical`](struct.Viewport.html#method.physical "method pert_ui::Viewport::physical")
is what the surface is sized from.

The desktop adapter calls this automatically, before it first flushes.
Headless callers can supply a viewport to test custom drawing; until
they do, layout runs against an empty one. Hidden components retain
dirtiness.

##### [§](#errors-52)Errors

Returns `Closed` after shutdown; `WriteDuringInspection` from a
[`command_enabled`](struct.Runtime.html#method.command_enabled "method pert_ui::Runtime::command_enabled")
predicate, which runs during inspection; `WriteDuringDelivery` from
inside a parameter delivery callback; and
`Geometry(GeometryError::NonFinite)` when the viewport divided by the UI
zoom overflows. Update, layout and paint run inside the runtime’s own
`&mut self` methods, so their phase errors cannot reach this `&self`
method.

[Source](../src/pert_ui_runtime/runtime.rs.html#4349)

#### pub fn [new](#method.new)() -\> [Runtime](struct.Runtime.html "struct pert_ui::Runtime")

Creates an empty runtime with no scheduled work, workers or timers.

[Source](../src/pert_ui_runtime/runtime.rs.html#4454)

#### pub fn [signal](#method.signal)\<T\>(&self, value: T) -\> [Signal](struct.Signal.html "struct pert_ui::Signal")\<T\>

Creates a signal owned by this runtime. Values may be general
application data.

[Source](../src/pert_ui_runtime/runtime.rs.html#4483-4487)

#### pub fn [series_source](#method.series_source)\<T\>( &self, consumer: [SeriesConsumer](struct.SeriesConsumer.html "struct pert_ui::SeriesConsumer")\<T\>, delivery: [Delivery](enum.Delivery.html "enum pert_ui::Delivery"), ) -\> [SeriesSource](struct.SeriesSource.html "struct pert_ui::SeriesSource")\<T\>

where T: [LiveItem](trait.LiveItem.html "trait pert_ui::LiveItem"),

Adopts the consumer half of a series bridge as a UI-thread source.

The runtime drains it while a visible component subscribes through
[`UpdateContext::read_series`](struct.UpdateContext.html#method.read_series "method pert_ui::UpdateContext::read_series");
see
[`SeriesSource`](struct.SeriesSource.html "struct pert_ui::SeriesSource")
for the schedule. The endpoint half goes to the producer thread.

``` rust
use pert_ui::{Capacity, Delivery, Runtime, SeriesConfig, Timestamp, series_bridge};

let runtime = Runtime::new();
let config = SeriesConfig::new(Capacity::new(256)?);
let (mut endpoint, consumer) = series_bridge::<f32>(config);
let source = runtime.series_source(consumer, Delivery::Polled);
let producer = std::thread::spawn(move || {
    let _ = endpoint.publish(0.5, Timestamp::new(1));
});
producer.join().expect("producer");
// Nothing subscribes yet, so nothing was drained and nothing is scheduled.
assert_eq!(runtime.next_deadline(), None);
assert!(source.telemetry()?.latest.is_none());
```

[Source](../src/pert_ui_runtime/runtime.rs.html#4499)

#### pub fn [live_waker](#method.live_waker)(&self) -\> [LiveWaker](struct.LiveWaker.html "struct pert_ui::LiveWaker")

Returns a `Send` handle that asks the runtime to drain notified sources.

Without
[`HostIntegration::set_wake_delivery`](host/trait.HostIntegration.html#tymethod.set_wake_delivery "method pert_ui::host::HostIntegration::set_wake_delivery")
a notification only marks the runtime due, which
[`Self::next_deadline`](struct.Runtime.html#method.next_deadline "method pert_ui::Runtime::next_deadline")
reports; a host that sleeps on an event loop installs delivery so the
notification also wakes it.

[Source](../src/pert_ui_runtime/runtime.rs.html#4526-4530)

#### pub fn [mailbox](#method.mailbox)\<T\>( &self, capacity: [Capacity](struct.Capacity.html "struct pert_ui::Capacity"), handler: impl [FnMut](https://doc.rust-lang.org/1.99.1/core/ops/function/trait.FnMut.html "trait core::ops::function::FnMut")(T) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[()](https://doc.rust-lang.org/1.99.1/std/primitive.unit.html), [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\> + 'static, ) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[Mailbox](struct.Mailbox.html "struct pert_ui::Mailbox")\<T\>, [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\>

where T:
[Send](https://doc.rust-lang.org/1.99.1/core/marker/trait.Send.html "trait core::marker::Send") +
'static,

Creates a bounded mailbox that delivers owned values from other threads
to `handler` on this runtime’s thread.

Timing class: [at the next
flush](https://github.com/pertyjons/pert-ui/blob/main/docs/contracts.md#at-the-next-flush).

Hand
[`Mailbox::sender`](struct.Mailbox.html#method.sender "method pert_ui::Mailbox::sender")
handles to workers; each accepted value wakes the host like a
[`LiveWaker`](struct.LiveWaker.html "struct pert_ui::LiveWaker") and
reaches `handler` at the start of a later
[`Self::flush`](struct.Runtime.html#method.flush "method pert_ui::Runtime::flush"),
in order, after live-data sources are drained and before components
update, so values the handler writes to signals are shown by that flush.
A flush delivers only what was queued when its delivery began: all inbox
frontiers are captured before any handler runs, so sends between
handlers wait for the next flush regardless of mailbox registration
order. Concurrent worker sends are included only if accepted before
their inbox’s frontier is captured. A handler error is returned from
that flush; the value it was given is consumed and the rest stay queued
for the next flush. The contract is in `docs/contracts.md`.

##### [§](#errors-53)Errors

Returns `Closed` after shutdown, `WriteDuringInspection` from a
[`command_enabled`](struct.Runtime.html#method.command_enabled "method pert_ui::Runtime::command_enabled")
predicate and `WriteDuringDelivery` from inside a parameter delivery
callback. The other phase errors cannot reach this `&self` method, as
for
[`Self::set_viewport`](struct.Runtime.html#method.set_viewport "method pert_ui::Runtime::set_viewport").

[Source](../src/pert_ui_runtime/runtime.rs.html#4555-4558)

#### pub fn [inspection_service](#method.inspection_service)( &self, capacity: [Capacity](struct.Capacity.html "struct pert_ui::Capacity"), ) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[InspectionService](struct.InspectionService.html "struct pert_ui::InspectionService"), [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\>

Creates an
[`InspectionService`](struct.InspectionService.html "struct pert_ui::InspectionService")
whose clients ask this runtime for snapshots, queries, profiles,
validation reports and workspace captures from other threads.

The queue holds at most `capacity` unanswered requests; nothing is
preallocated and no thread or timer starts. Requests are answered at the
end of each flush, and an accepted request makes the runtime due in
[`next_deadline`](struct.Runtime.html#method.next_deadline "method pert_ui::Runtime::next_deadline")
and wakes the host through the same delivery a
[`UiSender`](struct.UiSender.html "struct pert_ui::UiSender") uses.

##### [§](#errors-54)Errors

Returns `Closed` after shutdown, `WriteDuringInspection` from a
[`command_enabled`](struct.Runtime.html#method.command_enabled "method pert_ui::Runtime::command_enabled")
predicate and `WriteDuringDelivery` from inside a parameter delivery
callback. The other phase errors cannot reach this `&self` method, as
for
[`Self::set_viewport`](struct.Runtime.html#method.set_viewport "method pert_ui::Runtime::set_viewport").

[Source](../src/pert_ui_runtime/runtime.rs.html#4611-4615)

#### pub fn [event](#method.event)\<R\>( &mut self, id: &[WidgetId](struct.WidgetId.html "struct pert_ui::WidgetId"), event: impl [FnOnce](https://doc.rust-lang.org/1.99.1/core/ops/function/trait.FnOnce.html "trait core::ops::function::FnOnce")(&mut [EventContext](struct.EventContext.html "struct pert_ui::EventContext")) -\> R, ) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<R, [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\>

Dispatches a general event closure to a visible mount and focuses it.

Timing class: [in the
input](https://github.com/pertyjons/pert-ui/blob/main/docs/contracts.md#in-the-input).

Switching focus first cancels the old component’s active gestures. The
closure may write signals or begin/edit parameter gestures. Its return
value is passed through; fallible closures therefore yield a nested
result.

##### [§](#errors-55)Errors

The refusals of
[`Self::focus`](struct.Runtime.html#method.focus "method pert_ui::Runtime::focus")
(a runtime phase error,
[`RuntimeError::ForeignRuntime`](enum.RuntimeError.html#variant.ForeignRuntime "variant pert_ui::RuntimeError::ForeignRuntime"),
[`RuntimeError::Unmounted`](enum.RuntimeError.html#variant.Unmounted "variant pert_ui::RuntimeError::Unmounted"),
[`RuntimeError::Hidden`](enum.RuntimeError.html#variant.Hidden "variant pert_ui::RuntimeError::Hidden")
or
[`RuntimeError::ModalBlocked`](enum.RuntimeError.html#variant.ModalBlocked "variant pert_ui::RuntimeError::ModalBlocked"))
come before executing user code, leaving focus and the selected window
unchanged. The errors
[`Self::focus`](struct.Runtime.html#method.focus "method pert_ui::Runtime::focus")
forwards after the transfer, from
[`Component::focus_lost`](trait.Component.html#method.focus_lost "method pert_ui::Component::focus_lost"),
[`Component::focus_within_changed`](trait.Component.html#method.focus_within_changed "method pert_ui::Component::focus_within_changed"),
[`Component::reveal`](trait.Component.html#method.reveal "method pert_ui::Component::reveal")
or the flush that settles a scrolled ancestor, are returned after focus
has moved to the target, without running the closure.

[Source](../src/pert_ui_runtime/runtime.rs.html#4633)

#### pub fn [keyboard](#method.keyboard)(&self) -\> &[KeyboardState](struct.KeyboardState.html "struct pert_ui::KeyboardState")

Returns shared application physical-key state, independent of text/IME.

[Source](../src/pert_ui_runtime/runtime.rs.html#4677)

#### pub fn [focus](#method.focus)(&mut self, id: &[WidgetId](struct.WidgetId.html "struct pert_ui::WidgetId")) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[()](https://doc.rust-lang.org/1.99.1/std/primitive.unit.html), [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\>

Focuses a visible component, resetting the previous input/edit context.

Physical held keys stay tracked during an in-window focus transfer. The
new component receives only subsequent events, not fabricated key-downs.
Scrollable ancestors reveal the target; pointer presses focus their hit
target without this additional scrolling, and each ancestor that scrolls
is settled by a flush before this returns.

This is the low-level focus operation: unlike keyboard traversal and
pointer presses, it also focuses a component whose
[`Component::focusable`](trait.Component.html#method.focusable "method pert_ui::Component::focusable")
is false, as that method documents. To reveal a hidden target first, use
[`Self::reveal_and_focus`](struct.Runtime.html#method.reveal_and_focus "method pert_ui::Runtime::reveal_and_focus").

Focusing a component of another window selects that window. A refused
target leaves the selected window and its focus unchanged.

Focus does not move a text editor’s caret or selection: they stay where
the editor’s last edit or movement left them, which is the start of the
text for an editor not edited since it was mounted or since its binding
was written from outside. Typed text is inserted there. A pointer press
places the caret where it lands; to replace the text, select it all
first with
[`TextCommand::SelectAll`](enum.TextCommand.html#variant.SelectAll "variant pert_ui::TextCommand::SelectAll")
through
[`Self::text_command`](struct.Runtime.html#method.text_command "method pert_ui::Runtime::text_command").

##### [§](#errors-56)Errors

Before anything changes: a runtime phase error (see
[`Runtime`](struct.Runtime.html "struct pert_ui::Runtime"));
[`RuntimeError::ForeignRuntime`](enum.RuntimeError.html#variant.ForeignRuntime "variant pert_ui::RuntimeError::ForeignRuntime")
for an ID of another runtime;
[`RuntimeError::Unmounted`](enum.RuntimeError.html#variant.Unmounted "variant pert_ui::RuntimeError::Unmounted")
for an unmounted one;
[`RuntimeError::Hidden`](enum.RuntimeError.html#variant.Hidden "variant pert_ui::RuntimeError::Hidden")
for a hidden component; and
[`RuntimeError::ModalBlocked`](enum.RuntimeError.html#variant.ModalBlocked "variant pert_ui::RuntimeError::ModalBlocked")
for one a modal surface blocks.

After the transfer has completed, with focus on the target, the
callbacks’ errors are forwarded. The first error of the previous
recipient’s
[`Component::focus_lost`](trait.Component.html#method.focus_lost "method pert_ui::Component::focus_lost")
and of
[`Component::focus_within_changed`](trait.Component.html#method.focus_within_changed "method pert_ui::Component::focus_within_changed")
on the ancestors focus leaves and then enters is returned, and later
ones are queued for later
[`Self::flush`](struct.Runtime.html#method.flush "method pert_ui::Runtime::flush")
calls. Revealing the target then forwards an error of an ancestor’s
[`Component::reveal`](trait.Component.html#method.reveal "method pert_ui::Component::reveal")
or of the flush that settles a scrolled ancestor; such an error is
returned, and a callback error that would have been returned is queued
ahead of the later ones.

[Source](../src/pert_ui_runtime/runtime.rs.html#4724)

#### pub fn [can_focus](#method.can_focus)(&self, id: &[WidgetId](struct.WidgetId.html "struct pert_ui::WidgetId")) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[bool](https://doc.rust-lang.org/1.99.1/std/primitive.bool.html), [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\>

Whether `id` can take focus now: the component is mounted, visible,
focusable by its own
[`Component::focusable`](trait.Component.html#method.focusable "method pert_ui::Component::focusable")
(a disabled built-in control is not), and no modal surface of its window
blocks it.

[`Self::focus`](struct.Runtime.html#method.focus "method pert_ui::Runtime::focus")
refuses only the hidden, unmounted and modal-blocked cases and focuses a
component whose `focusable` is false. A host that gives focus back to a
component it took focus from asks this first, so a component disabled
meanwhile stays unfocused. It is a query only: it reveals nothing,
flushes nothing and changes no state, so its answer describes the state
since the last flush.

``` rust
use pert_ui::prelude::*;
use pert_ui::testing::*;
use std::time::Duration;

let mut ui = Runtime::new();
ui.set_viewport(Viewport::new(300.0, 100.0, 1.0)?)?;
let root = ui.mount_tree(Element::new("form", Column::new([
    Element::new("name", TextField::new(ui.signal(String::new()))),
    Element::new("title", Label::new("Name")),
])))?;
ui.flush(Duration::ZERO)?;
let name = ui.find(&root, &["name"])?.ok_or("not mounted")?;
let title = ui.find(&root, &["title"])?.ok_or("not mounted")?;
assert!(ui.can_focus(&name)?);
assert!(!ui.can_focus(&title)?); // a label takes no focus
ui.set_visible(&name, false)?;
assert!(!ui.can_focus(&name)?);
```

##### [§](#errors-57)Errors

[`RuntimeError::ForeignRuntime`](enum.RuntimeError.html#variant.ForeignRuntime "variant pert_ui::RuntimeError::ForeignRuntime")
for an ID of another runtime, and nothing else: an unmounted or hidden
component, including every component after
[`Self::shutdown`](struct.Runtime.html#method.shutdown "method pert_ui::Runtime::shutdown"),
answers `false`. Reading needs no phase check, since it writes nothing.

[Source](../src/pert_ui_runtime/runtime.rs.html#4988-4992)

#### pub fn [key_input](#method.key_input)( &mut self, code: [KeyCode](enum.KeyCode.html "enum pert_ui::KeyCode"), action: [KeyAction](enum.KeyAction.html "enum pert_ui::KeyAction"), ) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[KeyEvent](struct.KeyEvent.html "struct pert_ui::KeyEvent"), [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\>

Applies a physical key transition and dispatches it to the focused
component.

An ignored key bubbles through the focused component’s ancestors. With
nothing focused, a key other than Tab goes to the first visible
component that opts in through
[`Component::receives_unfocused_keys`](trait.Component.html#method.receives_unfocused_keys "method pert_ui::Component::receives_unfocused_keys"),
such as a `MenuBar`, and is otherwise dropped; Tab moves focus as usual.

Repeats do not add another held key. An orphan `Repeat`, for a key that
is not held (for example one pressed before focus arrived), never
establishes a press: it is returned as `Repeated` without reaching any
component, and the key’s later release is `UnmatchedRelease`, as through
[`Runtime::command_key_input`](struct.Runtime.html#method.command_key_input "method pert_ui::Runtime::command_key_input").

An Escape press (not a repeat or a release) goes to the first of these
that takes it:

1.  During a drag session it cancels the drag and reaches no component.
2.  Without modifiers, shown delayed help takes it and closes.
3.  While a popup, menu or dialog is open, it is offered to the focused
    editors as the native path’s text phase offers it (see
    [`Component::text_key_event`](trait.Component.html#method.text_key_event "method pert_ui::Component::text_key_event")),
    so an editor that answers it, such as a search field with a query,
    keeps the surface open.
4.  Otherwise the topmost interactive surface it may dismiss closes.
5.  Only then, when a component is focused, its pointer capture, its
    timer, a drag it started and its parameter edit sessions (begun with
    [`Parameter::begin_edit`](struct.Parameter.html#method.begin_edit "method pert_ui::Parameter::begin_edit"))
    end, without calling
    [`Component::cancel_input`](trait.Component.html#method.cancel_input "method pert_ui::Component::cancel_input"),
    and the component receives Escape as an ordinary key, bubbling as
    any key does. With nothing focused it goes to the unfocused-key
    recipient with no such cleanup.

Where an earlier step takes Escape, step 5’s cleanup of the focused
component does not run. The consumer’s own lifecycle still cleans up as
it always does: cancelling a drag releases the source’s pointer capture
and reports
[`DragOutcome::Cancelled`](enum.DragOutcome.html#variant.Cancelled "variant pert_ui::DragOutcome::Cancelled")
to its
[`Component::drag_finished`](trait.Component.html#method.drag_finished "method pert_ui::Component::drag_finished");
closing delayed help or a surface hides or removes what it showed, and
each component hidden or removed that way loses its capture, timer and
edit sessions and receives
[`Component::cancel_input`](trait.Component.html#method.cancel_input "method pert_ui::Component::cancel_input"),
as hiding with
[`Self::set_visible`](struct.Runtime.html#method.set_visible "method pert_ui::Runtime::set_visible")
does, with focus that was inside moving elsewhere. A component the
consumer does not touch keeps its capture, timer and edit sessions.
Released keys never automatically commit a generic gesture: a control
chooses its own confirmation key/action.

This is the headless entry point: it routes neither command bindings nor
[`Runtime::register_held_input`](struct.Runtime.html#method.register_held_input "method pert_ui::Runtime::register_held_input")
registrations, which only
[`Runtime::command_key_input`](struct.Runtime.html#method.command_key_input "method pert_ui::Runtime::command_key_input"),
the native path, routes. Use it to drive components directly; a test
that claims what an application’s user sees from a key uses
`command_key_input` instead.

The key is delivered at once and its recipient writes what its own rules
write, as
[`Self::text_input`](struct.Runtime.html#method.text_input "method pert_ui::Runtime::text_input")
describes; the tree, derived bindings and layout show the result after
the next
[`Self::flush`](struct.Runtime.html#method.flush "method pert_ui::Runtime::flush").

A synthesized press of a key that is physically held is a repeat
([`KeyTransition::Repeated`](enum.KeyTransition.html#variant.Repeated "variant pert_ui::KeyTransition::Repeated")),
as the keyboard state classifies it. Controls that act on a fresh press,
such as
[`InlineText`](struct.InlineText.html "struct pert_ui::InlineText")’s
F2, ignore it. To start editing from code, use
[`TextCommand::BeginEdit`](enum.TextCommand.html#variant.BeginEdit "variant pert_ui::TextCommand::BeginEdit").

##### [§](#errors-58)Errors

Propagates component input errors, and the errors of the flush that
delivers a notification the key requested, including a
[`Component::deliver_notifications`](trait.Component.html#method.deliver_notifications "method pert_ui::Component::deliver_notifications")
error; physical state remains updated so it cannot become stuck merely
because a component rejected an event. Older or external notification
errors propagate without cancelling this key’s input ownership after
dispatch. If a preflush prevents a held key’s release from reaching its
owner, cleanup still ends its held actions. Direct or attributable
failures clean up each mounted component actually called, including
keyboard and Escape-text bubbling. Preparation and callbacks may
transfer focus; cleanup never retargets the new focus. With no delivered
callback, failure cleanup retains the preflush owner.

[Source](../src/pert_ui_runtime/runtime.rs.html#5209)

#### pub fn [clipboard_text](#method.clipboard_text)(&self) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[String](https://doc.rust-lang.org/1.99.1/alloc/string/struct.String.html "struct alloc::string::String"), [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\>

Reads the clipboard’s text on the UI thread, from application code or a
command action, without involving any component or moving focus.

The native `App` installs the platform clipboard; a headless runtime has
none until
[`HostIntegration::set_clipboard`](host/trait.HostIntegration.html#tymethod.set_clipboard "method pert_ui::host::HostIntegration::set_clipboard")
installs a service.

##### [§](#errors-59)Errors

As
[`EventContext::clipboard_text`](struct.EventContext.html#method.clipboard_text "method pert_ui::EventContext::clipboard_text"):
`Closed` after shutdown, the phase errors of a write from inside update,
layout, paint, inspection or delivery, `BorrowConflict` from inside the
clipboard service,
[`ClipboardError::Unavailable`](enum.ClipboardError.html#variant.Unavailable "variant pert_ui::ClipboardError::Unavailable")
without a service, and the service’s own
[`ClipboardError`](enum.ClipboardError.html "enum pert_ui::ClipboardError").

[Source](../src/pert_ui_runtime/runtime.rs.html#5244)

#### pub fn [set_clipboard_text](#method.set_clipboard_text)(&self, text: &[str](https://doc.rust-lang.org/1.99.1/std/primitive.str.html)) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[()](https://doc.rust-lang.org/1.99.1/std/primitive.unit.html), [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\>

Writes text to the clipboard on the UI thread, from application code or
a command action, without involving any component or moving focus: a
“Copy” command that copies the application’s own data uses this.

``` rust
use pert_ui::testing::HostIntegration;
use pert_ui::prelude::*;
use pert_ui::{Clipboard, ClipboardError, CommandSource};
struct Memory(std::rc::Rc<std::cell::RefCell<String>>);
impl Clipboard for Memory {
    fn read_text(&mut self) -> Result<String, ClipboardError> { Ok(self.0.borrow().clone()) }
    fn write_text(&mut self, text: &str) -> Result<(), ClipboardError> {
        *self.0.borrow_mut() = text.to_owned();
        Ok(())
    }
}
let mut ui = Runtime::new();
let held = std::rc::Rc::new(std::cell::RefCell::new(String::new()));
ui.set_clipboard(Memory(held.clone()))?;
let copy = ui.register_command("Copy line", CommandScope::Application, vec![],
    |ui, _| ui.set_clipboard_text("12:00:01 INFO started"))?;
let context = ui.command_context(CommandSource::Programmatic);
ui.invoke_command(&copy, context)?;
assert_eq!(*held.borrow(), "12:00:01 INFO started");
assert_eq!(ui.clipboard_text()?, "12:00:01 INFO started");
```

##### [§](#errors-60)Errors

As
[`Self::clipboard_text`](struct.Runtime.html#method.clipboard_text "method pert_ui::Runtime::clipboard_text");
the service reports a failed write as its own
[`ClipboardError`](enum.ClipboardError.html "enum pert_ui::ClipboardError").

[Source](../src/pert_ui_runtime/runtime.rs.html#5308)

#### pub fn [text_input](#method.text_input)(&mut self, text: &[str](https://doc.rust-lang.org/1.99.1/std/primitive.str.html)) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[()](https://doc.rust-lang.org/1.99.1/std/primitive.unit.html), [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\>

Dispatches committed text to the focused component without changing
keys.

Empty text inserts nothing and leaves a selection in place, as an input
method’s empty commit must: deleting a selection takes a Backspace or
Delete key. The text is delivered at once and is written as the focused
component’s own rules write it: a plain text editor writes its binding
immediately, while a draft, a parameter proposal and a notification for
a hidden window are not written yet. Derived bindings, dynamic children
and layout show the result after the next
[`Self::flush`](struct.Runtime.html#method.flush "method pert_ui::Runtime::flush"),
so a test flushes before it inspects.

##### [§](#errors-61)Errors

Forwards the errors of the
[`Self::flush`](struct.Runtime.html#method.flush "method pert_ui::Runtime::flush")
that runs first, the component’s text-input error, and the errors of the
flush that delivers a notification the text requested, including a
[`Component::deliver_notifications`](trait.Component.html#method.deliver_notifications "method pert_ui::Component::deliver_notifications")
error. With no focus, text is ignored.

[Source](../src/pert_ui_runtime/runtime.rs.html#5474)

#### pub fn [set_visible](#method.set_visible)( &mut self, id: &[WidgetId](struct.WidgetId.html "struct pert_ui::WidgetId"), visible: [bool](https://doc.rust-lang.org/1.99.1/std/primitive.bool.html), ) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[()](https://doc.rust-lang.org/1.99.1/std/primitive.unit.html), [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\>

Changes presentation visibility, retaining component state and
subscriptions.

Hidden dirtiness accumulates but never schedules evaluation. Hiding
cancels edit ownership and deadlines. Showing schedules one layout/paint
refresh with current binding values; a component may then resume live
scheduling.

##### [§](#errors-62)Errors

Returns `ForeignRuntime` for an ID of another runtime, `Unmounted` for
an unmounted ID, which every ID is after shutdown, and
`WriteDuringDelivery` from inside a parameter delivery callback, such as
a sink that holds the runtime in a shared cell.

[Source](../src/pert_ui_runtime/runtime.rs.html#5519)

#### pub fn [unmount](#method.unmount)(&mut self, id: &[WidgetId](struct.WidgetId.html "struct pert_ui::WidgetId")) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[()](https://doc.rust-lang.org/1.99.1/std/primitive.unit.html), [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\>

Recursively releases a mount and descendants, children before their
parent.

Teardown, including the dismissal of a transient surface the subtree
owns, completes inside this call. Failed cleanup callbacks retain typed
errors for later flushes on a surviving window, or the shutdown report;
they keep a visible surviving window due until consumed and never retry
the removed owner.
[`Runtime::next_deadline`](struct.Runtime.html#method.next_deadline "method pert_ui::Runtime::next_deadline")
reports work afterwards only for what survives: a remaining parent that
must arrange its other children, or focus that was inside the subtree
and has to be restored, or a retained cleanup error.

##### [§](#errors-63)Errors

Returns a runtime phase error (see
[`Runtime`](struct.Runtime.html "struct pert_ui::Runtime")),
[`RuntimeError::ForeignRuntime`](enum.RuntimeError.html#variant.ForeignRuntime "variant pert_ui::RuntimeError::ForeignRuntime")
for an ID of another runtime or
[`RuntimeError::Unmounted`](enum.RuntimeError.html#variant.Unmounted "variant pert_ui::RuntimeError::Unmounted")
for an already unmounted one. Its old identity remains invalid after
another component is mounted using the same logical key.

[Source](../src/pert_ui_runtime/runtime.rs.html#5581-5585)

#### pub fn [text_command](#method.text_command)( &mut self, id: &[WidgetId](struct.WidgetId.html "struct pert_ui::WidgetId"), command: [TextCommand](enum.TextCommand.html "enum pert_ui::TextCommand"), ) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[TextCommandOutcome](enum.TextCommandOutcome.html "enum pert_ui::TextCommandOutcome"), [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\>

Performs a text command (clipboard, selection or beginning an edit) on
one component’s own text, for hosts and application chrome such as an
Edit menu acting on the focused editor, or a command that renames a
label.

Delivery is immediate and synchronous, unlike
[`EventContext::request_text_command`](struct.EventContext.html#method.request_text_command "method pert_ui::EventContext::request_text_command"),
which a component uses to reach another component during a flush. A
component that is not an editor reports `NotAnEditor` without an error;
when the focused component lies inside its subtree, the command goes on
to that one, so a composite control whose text mode is an embedded
editor (such as a row with a
[`DraftEditor`](struct.DraftEditor.html "struct pert_ui::DraftEditor"))
answers through it. A resting label has no focused descendant, so code
that begins an edit targets the label itself.

Two commands request focus, and none moves it out of the target control:
an editor that answers
[`TextCommand::BeginEdit`](enum.TextCommand.html#variant.BeginEdit "variant pert_ui::TextCommand::BeginEdit")
`Handled` is editing on return and has requested focus for its editing
part, and an in-place control that answers
[`TextCommand::DiscardDraft`](enum.TextCommand.html#variant.DiscardDraft "variant pert_ui::TextCommand::DiscardDraft")
`Handled` by ending a session whose editor held focus has requested
focus for itself, as Escape does. Either request moves focus at the next
flush, since this call does not flush. An
[`InlineText`](struct.InlineText.html "struct pert_ui::InlineText") in
[row
mode](struct.InlineText.html#method.in_row "method pert_ui::InlineText::in_row"),
which is not focusable at rest, requests none: at that flush focus falls
back to its nearest focusable ancestor, such as its list, as when Escape
ends the session.

##### [§](#errors-64)Errors

Rejects foreign, unmounted, hidden and modal-blocked IDs (`Hidden` and
`ModalBlocked`, as
[`Self::event`](struct.Runtime.html#method.event "method pert_ui::Runtime::event")
does) before the editor sees the command, so a hidden editor’s selection
is never cut or replaced. Clipboard failures and text the editor rejects
for its characters propagate from the receiving editor, which preserves
its text in that case; a paste that would exceed the editor’s size limit
is refused as `Unavailable` rather than failing.

[Source](../src/pert_ui_runtime/runtime.rs.html#5669)

#### pub fn [next_deadline](#method.next_deadline)(&self) -\> [Option](https://doc.rust-lang.org/1.99.1/core/option/enum.Option.html "enum core::option::Option")\<[Duration](https://doc.rust-lang.org/1.99.1/core/time/struct.Duration.html "struct core::time::Duration")\>

Returns the earliest deadline of the selected window and of the work all
windows share, or `None` when the runtime can sleep.

While immediate work waits, such as pending updates or layout, queued
commands, reveals and structure changes, deferred tasks, background
results that have arrived, queued focus errors, inspection requests or
notifications, it is the current
[`Self::now`](struct.Runtime.html#method.now "method pert_ui::Runtime::now").
A background worker still computing is not included: the runtime learns
of it only when its result arrives. Otherwise it is the earliest of the
selected window’s update deadlines and event timers and the shared
work’s deadlines: interval timers, view-scale settling and polled live
sources. Another window’s own deadlines are not included; a host that
drives every window reads
[`HostIntegration::application_deadline`](host/trait.HostIntegration.html#tymethod.application_deadline "method pert_ui::host::HostIntegration::application_deadline").

[Source](../src/pert_ui_runtime/runtime.rs.html#5797)

#### pub fn [flush](#method.flush)(&mut self, now: [Duration](https://doc.rust-lang.org/1.99.1/core/time/struct.Duration.html "struct core::time::Duration")) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[FlushReport](struct.FlushReport.html "struct pert_ui::FlushReport"), [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\>

Advances elapsed time, promotes due deadlines and evaluates pending
mounts.

Multiple writes before a flush coalesce. Static mounts are not scanned
on live ticks. Newly exposed descendants update before their first
measurement. Missed deadlines produce one update; no catch-up loop is
used. A failed component retains its work for retry, and other pending
mounts remain queued. Dependency sets are replaced even after a returned
error. Change notifications follow global request order across visible
windows; their presentation and callback writes settle with the same
wave budget. The previously selected window is restored when it remains
mounted. Hidden native windows defer notifications until shown.
Notification preparation does not repeat mailbox, task or timer delivery
frontiers.

Update/layout/focus/hover feedback is limited to
[`Self::MAX_FLUSH_WAVES`](struct.Runtime.html#associatedconstant.MAX_FLUSH_WAVES "associated constant pert_ui::Runtime::MAX_FLUSH_WAVES"),
shared by internal presentation follow-ups. Ordinary queued reveals use
their normal flush budgets independently of request count and path
depth; an exhausted notification budget is not restarted for pending
reveals. On exhaustion, repair the cause before retry; pending work
remains queued. Development validation names pending components. A wave
limit cannot interrupt a nonreturning callback or bound wall time.

##### [§](#errors-65)Errors

Returns `UpdateLimit` when feedback does not settle; rejects backwards
time. Propagates a component’s read/scheduling error or the oldest
queued error from an infallible window focus-loss callback.

[Source](../src/pert_ui_runtime/runtime.rs.html#6660)[§](#impl-Runtime-25)

### impl [Runtime](struct.Runtime.html "struct pert_ui::Runtime")

[Source](../src/pert_ui_runtime/runtime.rs.html#6669)

#### pub fn [shutdown](#method.shutdown)(&mut self)

Permanently closes the runtime, canceling input and unmounting all
lifetime roots before native resources are destroyed. Idempotent;
subsequent runtime mutations fail with Closed. Signal observers and host
services are released. Active and pending drags deliver cancellation
while participants are mounted; targets are told to leave before the
source receives its terminal outcome. Cleanup continues after callback
failure. The first error per cancelled drag operation and other queued
cleanup errors remain available through
[`HostIntegration::take_shutdown_errors`](host/trait.HostIntegration.html#tymethod.take_shutdown_errors "method pert_ui::host::HostIntegration::take_shutdown_errors").
New drags are refused during teardown.

## Trait Implementations[§](#trait-implementations)

[Source](../src/pert_ui_runtime/runtime.rs.html#3948)[§](#impl-Debug-for-Runtime)

### impl [Debug](https://doc.rust-lang.org/1.99.1/core/fmt/trait.Debug.html "trait core::fmt::Debug") for [Runtime](struct.Runtime.html "struct pert_ui::Runtime")

Shows the primary window, not the component tree or the application’s
state. Inspect the tree with
[`Runtime::inspect`](struct.Runtime.html#method.inspect "method pert_ui::Runtime::inspect").

[Source](../src/pert_ui_runtime/runtime.rs.html#3949)[§](#method.fmt)

#### fn [fmt](https://doc.rust-lang.org/1.99.1/core/fmt/trait.Debug.html#tymethod.fmt)(&self, f: &mut [Formatter](https://doc.rust-lang.org/1.99.1/core/fmt/struct.Formatter.html "struct core::fmt::Formatter")\<'\_\>) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[()](https://doc.rust-lang.org/1.99.1/std/primitive.unit.html), [Error](https://doc.rust-lang.org/1.99.1/core/fmt/struct.Error.html "struct core::fmt::Error")\>

Formats the value using the given formatter. [Read
more](https://doc.rust-lang.org/1.99.1/core/fmt/trait.Debug.html#tymethod.fmt)

[Source](../src/pert_ui_runtime/runtime.rs.html#3956)[§](#impl-Default-for-Runtime)

### impl [Default](https://doc.rust-lang.org/1.99.1/core/default/trait.Default.html "trait core::default::Default") for [Runtime](struct.Runtime.html "struct pert_ui::Runtime")

[Source](../src/pert_ui_runtime/runtime.rs.html#3957)[§](#method.default)

#### fn [default](https://doc.rust-lang.org/1.99.1/core/default/trait.Default.html#tymethod.default)() -\> [Runtime](struct.Runtime.html "struct pert_ui::Runtime")

Returns the “default value” for a type. [Read
more](https://doc.rust-lang.org/1.99.1/core/default/trait.Default.html#tymethod.default)

[Source](../src/pert_ui_runtime/runtime.rs.html#6770)[§](#impl-Drop-for-Runtime)

### impl [Drop](https://doc.rust-lang.org/1.99.1/core/ops/drop/trait.Drop.html "trait core::ops::drop::Drop") for [Runtime](struct.Runtime.html "struct pert_ui::Runtime")

[Source](../src/pert_ui_runtime/runtime.rs.html#6771)[§](#method.drop)

#### fn [drop](https://doc.rust-lang.org/1.99.1/core/ops/drop/trait.Drop.html#method.drop)(&mut self)

Executes the destructor for this type. [Read
more](https://doc.rust-lang.org/1.99.1/core/ops/drop/trait.Drop.html#method.drop)

[Source](https://doc.rust-lang.org/1.99.1/src/core/ops/drop.rs.html#267)[§](#method.pin_drop)

#### fn [pin_drop](https://doc.rust-lang.org/1.99.1/core/ops/drop/trait.Drop.html#method.pin_drop)(self: [Pin](https://doc.rust-lang.org/1.99.1/core/pin/struct.Pin.html "struct core::pin::Pin")\<&mut Self\>)

🔬This is a nightly-only experimental API. (`pin_ergonomics`)

Execute the destructor for this type, but different to
[`Drop::drop`](https://doc.rust-lang.org/1.99.1/core/ops/drop/trait.Drop.html#method.drop "method core::ops::drop::Drop::drop"),
it requires `self` to be pinned. [Read
more](https://doc.rust-lang.org/1.99.1/core/ops/drop/trait.Drop.html#method.pin_drop)

[Source](../src/pert_ui_runtime/host.rs.html#580)[§](#impl-HostIntegration-for-Runtime)

### impl [HostIntegration](host/trait.HostIntegration.html "trait pert_ui::host::HostIntegration") for [Runtime](struct.Runtime.html "struct pert_ui::Runtime")

[Source](../src/pert_ui_runtime/host.rs.html#581)[§](#method.flush_windows)

#### fn [flush_windows](host/trait.HostIntegration.html#tymethod.flush_windows)(&mut self, now: [Duration](https://doc.rust-lang.org/1.99.1/core/time/struct.Duration.html "struct core::time::Duration")) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[FlushReport](struct.FlushReport.html "struct pert_ui::FlushReport"), [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\>

Flushes eligible roots at a shared time, retaining the selected root.
Only each root’s pending components run; static roots do not rebuild.
Returns the total update count and earliest application deadline. When
every root is hidden, deferred tasks are still delivered once, so a task
cannot keep
[`Self::application_deadline`](host/trait.HostIntegration.html#tymethod.application_deadline "method pert_ui::host::HostIntegration::application_deadline")
due without being run. [Read
more](host/trait.HostIntegration.html#tymethod.flush_windows)

[Source](../src/pert_ui_runtime/host.rs.html#584)[§](#method.flush_commands)

#### fn [flush_commands](host/trait.HostIntegration.html#tymethod.flush_commands)(&mut self) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[()](https://doc.rust-lang.org/1.99.1/std/primitive.unit.html), [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\>

Processes one queued menu/button batch independently of presentation
availability. Native hosts call this at an event boundary even when
every root is hidden. Each receipt completes with revalidated admission;
commands targeting unavailable roots fail instead of waiting for an
unrelated redraw. Reentrant requests remain scheduled for a subsequent
batch. Ordinary App consumers do not need to call this; normal flushes
also process commands. [Read
more](host/trait.HostIntegration.html#tymethod.flush_commands)

[Source](../src/pert_ui_runtime/host.rs.html#587)[§](#method.has_command_requests)

#### fn [has_command_requests](host/trait.HostIntegration.html#tymethod.has_command_requests)(&self) -\> [bool](https://doc.rust-lang.org/1.99.1/std/primitive.bool.html)

Whether command admission is pending independently of visible render
work.

[Source](../src/pert_ui_runtime/host.rs.html#590)[§](#method.application_deadline)

#### fn [application_deadline](host/trait.HostIntegration.html#tymethod.application_deadline)(&self) -\> [Option](https://doc.rust-lang.org/1.99.1/core/option/enum.Option.html "enum core::option::Option")\<[Duration](https://doc.rust-lang.org/1.99.1/core/time/struct.Duration.html "struct core::time::Duration")\>

Earliest eligible work across all roots, independent of selected/focused
window. Known hidden roots are excluded. Native hosts additionally
account for OS redraw availability and recovery deadlines; reading never
polls.

[Source](../src/pert_ui_runtime/host.rs.html#593)[§](#method.interval_deadline)

#### fn [interval_deadline](host/trait.HostIntegration.html#tymethod.interval_deadline)(&self) -\> [Option](https://doc.rust-lang.org/1.99.1/core/option/enum.Option.html "enum core::option::Option")\<[Duration](https://doc.rust-lang.org/1.99.1/core/time/struct.Duration.html "struct core::time::Duration")\>

The earliest due time of a running interval timer, for a host that
schedules its own wakeups;
[`Runtime::next_deadline`](struct.Runtime.html#method.next_deadline "method pert_ui::Runtime::next_deadline")
and
[`Self::application_deadline`](host/trait.HostIntegration.html#tymethod.application_deadline "method pert_ui::host::HostIntegration::application_deadline")
include it.

[Source](../src/pert_ui_runtime/host.rs.html#596)[§](#method.service_intervals)

#### fn [service_intervals](host/trait.HostIntegration.html#tymethod.service_intervals)(&mut self, now: [Duration](https://doc.rust-lang.org/1.99.1/core/time/struct.Duration.html "struct core::time::Duration")) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[()](https://doc.rust-lang.org/1.99.1/std/primitive.unit.html), [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\>

Runs the interval timers due at `now` outside a flush, for a host that
cannot flush a window, such as one whose redraw the window system
withholds. An ordinary flush runs due timers itself. Advances the
runtime’s time to `now`. [Read
more](host/trait.HostIntegration.html#tymethod.service_intervals)

[Source](../src/pert_ui_runtime/host.rs.html#599)[§](#method.service_inspection)

#### fn [service_inspection](host/trait.HostIntegration.html#tymethod.service_inspection)(&mut self) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[()](https://doc.rust-lang.org/1.99.1/std/primitive.unit.html), [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\>

Answers every queued inspection request now, from the state the last
flush left, without flushing. [Read
more](host/trait.HostIntegration.html#tymethod.service_inspection)

[Source](../src/pert_ui_runtime/host.rs.html#602)[§](#method.set_wake_delivery)

#### fn [set_wake_delivery](host/trait.HostIntegration.html#tymethod.set_wake_delivery)(&self, deliver: impl [Fn](https://doc.rust-lang.org/1.99.1/core/ops/function/trait.Fn.html "trait core::ops::function::Fn")() -\> [bool](https://doc.rust-lang.org/1.99.1/std/primitive.bool.html) + [Send](https://doc.rust-lang.org/1.99.1/core/marker/trait.Send.html "trait core::marker::Send") + [Sync](https://doc.rust-lang.org/1.99.1/core/marker/trait.Sync.html "trait core::marker::Sync") + 'static)

Installs how a
[`LiveWaker`](struct.LiveWaker.html "struct pert_ui::LiveWaker") wakes
this host’s event loop; the callback runs on the notifying thread and
returns whether the wake was delivered. The default native `App`
installs its own; a custom host installs one that reaches its loop.
Replacing it reopens a waker that reported closed because delivery
failed. After
[`Runtime::shutdown`](struct.Runtime.html#method.shutdown "method pert_ui::Runtime::shutdown")
the call is ignored and every waker keeps reporting
[`WakeError::Closed`](enum.WakeError.html#variant.Closed "variant pert_ui::WakeError::Closed"),
since no flush will drain what a wake announces. A callback that panics
unwinds out of that `notify`, leaving the wake pending for the next
flush; later notifications still reach the callback.

[Source](../src/pert_ui_runtime/host.rs.html#605)[§](#method.select_window)

#### fn [select_window](host/trait.HostIntegration.html#tymethod.select_window)(&mut self, window: &[NativeWindowId](struct.NativeWindowId.html "struct pert_ui::NativeWindowId")) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[()](https://doc.rust-lang.org/1.99.1/std/primitive.unit.html), [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\>

Routes subsequent viewport/paint/input operations to a mounted window.
[Read more](host/trait.HostIntegration.html#tymethod.select_window)

[Source](../src/pert_ui_runtime/host.rs.html#608)[§](#method.current_window)

#### fn [current_window](host/trait.HostIntegration.html#tymethod.current_window)(&self) -\> [NativeWindowId](struct.NativeWindowId.html "struct pert_ui::NativeWindowId")

Presentation root used by viewport, painting and native input
operations. Selecting a root is routing, not a focus transition or
native activation.

[Source](../src/pert_ui_runtime/host.rs.html#611)[§](#method.set_window_visible)

#### fn [set_window_visible](host/trait.HostIntegration.html#tymethod.set_window_visible)(&mut self, visible: [bool](https://doc.rust-lang.org/1.99.1/std/primitive.bool.html)) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[()](https://doc.rust-lang.org/1.99.1/std/primitive.unit.html), [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\>

Supplies known native presentation visibility for the selected root.
[Read more](host/trait.HostIntegration.html#tymethod.set_window_visible)

[Source](../src/pert_ui_runtime/host.rs.html#614)[§](#method.set_presentation_rate)

#### fn [set_presentation_rate](host/trait.HostIntegration.html#tymethod.set_presentation_rate)( &self, rate: [Option](https://doc.rust-lang.org/1.99.1/core/option/enum.Option.html "enum core::option::Option")\<[PresentationRate](struct.PresentationRate.html "struct pert_ui::PresentationRate")\>, ) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[()](https://doc.rust-lang.org/1.99.1/std/primitive.unit.html), [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\>

Supplies the display’s presentation ceiling for the selected window.
[Read
more](host/trait.HostIntegration.html#tymethod.set_presentation_rate)

[Source](../src/pert_ui_runtime/host.rs.html#617)[§](#method.presentation_rate)

#### fn [presentation_rate](host/trait.HostIntegration.html#tymethod.presentation_rate)(&self) -\> [Option](https://doc.rust-lang.org/1.99.1/core/option/enum.Option.html "enum core::option::Option")\<[PresentationRate](struct.PresentationRate.html "struct pert_ui::PresentationRate")\>

The supplied ceiling for this window, absent when unknown. Unlike the
scheduling fallback, this query never invents a measured display rate.

[Source](../src/pert_ui_runtime/host.rs.html#620)[§](#method.needs_paint)

#### fn [needs_paint](host/trait.HostIntegration.html#tymethod.needs_paint)(&self) -\> [bool](https://doc.rust-lang.org/1.99.1/std/primitive.bool.html)

Whether visible drawing or the scene background needs fresh composition.
[Read more](host/trait.HostIntegration.html#tymethod.needs_paint)

[Source](../src/pert_ui_runtime/host.rs.html#623)[§](#method.invalidate)

#### fn [invalidate](host/trait.HostIntegration.html#tymethod.invalidate)( &self, id: &[WidgetId](struct.WidgetId.html "struct pert_ui::WidgetId"), kind: [Invalidation](enum.Invalidation.html "enum pert_ui::Invalidation"), ) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[()](https://doc.rust-lang.org/1.99.1/std/primitive.unit.html), [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\>

Invalidates a mount for external geometry/surface events. [Read
more](host/trait.HostIntegration.html#tymethod.invalidate)

[Source](../src/pert_ui_runtime/host.rs.html#626)[§](#method.has_presentation_requests)

#### fn [has_presentation_requests](host/trait.HostIntegration.html#tymethod.has_presentation_requests)(&self) -\> [bool](https://doc.rust-lang.org/1.99.1/std/primitive.bool.html)

Whether the native host has presentation operations to process.

[Source](../src/pert_ui_runtime/host.rs.html#629)[§](#method.take_presentation_requests)

#### fn [take_presentation_requests](host/trait.HostIntegration.html#tymethod.take_presentation_requests)( &self, ) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[Vec](https://doc.rust-lang.org/1.99.1/alloc/vec/struct.Vec.html "struct alloc::vec::Vec")\<[PresentationRequest](struct.PresentationRequest.html "struct pert_ui::PresentationRequest")\>, [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\>

Takes native work once, preserving queued order. Only native hosts need
this API; ordinary applications use App. Dropping requests cancels
receipts. [Read
more](host/trait.HostIntegration.html#tymethod.take_presentation_requests)

[Source](../src/pert_ui_runtime/host.rs.html#632-636)[§](#method.prepare_detach)

#### fn [prepare_detach](host/trait.HostIntegration.html#tymethod.prepare_detach)( &mut self, view: &[WidgetId](struct.WidgetId.html "struct pert_ui::WidgetId"), options: [NativeWindowOptions](struct.NativeWindowOptions.html "struct pert_ui::NativeWindowOptions"), ) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[PreparedDetach](struct.PreparedDetach.html "struct pert_ui::PreparedDetach"), [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\>

Reserves a native destination without changing mounted state or input.
[Read more](host/trait.HostIntegration.html#tymethod.prepare_detach)

[Source](../src/pert_ui_runtime/host.rs.html#639)[§](#method.commit_detach)

#### fn [commit_detach](host/trait.HostIntegration.html#tymethod.commit_detach)( &mut self, prepared: [PreparedDetach](struct.PreparedDetach.html "struct pert_ui::PreparedDetach"), ) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[NativeWindowId](struct.NativeWindowId.html "struct pert_ui::NativeWindowId"), [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\>

Commits movement after the native host successfully prepares all
resources. [Read
more](host/trait.HostIntegration.html#tymethod.commit_detach)

[Source](../src/pert_ui_runtime/host.rs.html#642)[§](#method.redock)

#### fn [redock](host/trait.HostIntegration.html#tymethod.redock)(&mut self, window: &[NativeWindowId](struct.NativeWindowId.html "struct pert_ui::NativeWindowId")) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[()](https://doc.rust-lang.org/1.99.1/std/primitive.unit.html), [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\>

Returns a detached window’s view to its retained original owner. [Read
more](host/trait.HostIntegration.html#tymethod.redock)

[Source](../src/pert_ui_runtime/host.rs.html#645)[§](#method.close_detached_window)

#### fn [close_detached_window](host/trait.HostIntegration.html#tymethod.close_detached_window)(&mut self) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[PresentationStatus](enum.PresentationStatus.html "enum pert_ui::PresentationStatus"), [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\>

Applies accepted close policy to the selected detached window.
Main-window shutdown belongs to the native host. Failed re-dock leaves
content available. [Read
more](host/trait.HostIntegration.html#tymethod.close_detached_window)

[Source](../src/pert_ui_runtime/host.rs.html#648)[§](#method.native_window_options)

#### fn [native_window_options](host/trait.HostIntegration.html#tymethod.native_window_options)(&self) -\> [Option](https://doc.rust-lang.org/1.99.1/core/option/enum.Option.html "enum core::option::Option")\<&[NativeWindowOptions](struct.NativeWindowOptions.html "struct pert_ui::NativeWindowOptions")\>

Options of the selected detached window; the main window uses App
defaults.

[Source](../src/pert_ui_runtime/host.rs.html#651)[§](#method.text_preedit)

#### fn [text_preedit](host/trait.HostIntegration.html#tymethod.text_preedit)(&mut self, preedit: &[TextPreedit](struct.TextPreedit.html "struct pert_ui::TextPreedit")) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[()](https://doc.rust-lang.org/1.99.1/std/primitive.unit.html), [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\>

Dispatches temporary IME composition to the current focused component.
Flushes pending binding changes first. No focus ignores the event.
Adapters must discard native events from a previous focus owner. [Read
more](host/trait.HostIntegration.html#tymethod.text_preedit)

[Source](../src/pert_ui_runtime/host.rs.html#654)[§](#method.text_input_state)

#### fn [text_input_state](host/trait.HostIntegration.html#tymethod.text_input_state)(&self) -\> [Option](https://doc.rust-lang.org/1.99.1/core/option/enum.Option.html "enum core::option::Option")\<([WidgetId](struct.WidgetId.html "struct pert_ui::WidgetId"), [TextInputState](struct.TextInputState.html "struct pert_ui::TextInputState"))\>

Returns the focused text recipient and its clipped window-logical IME
anchor. Absent for non-editable, hidden or fully clipped components.

[Source](../src/pert_ui_runtime/host.rs.html#657)[§](#method.cursor_icon)

#### fn [cursor_icon](host/trait.HostIntegration.html#tymethod.cursor_icon)(&self) -\> [CursorIcon](enum.CursorIcon.html "enum pert_ui::CursorIcon")

Resolves cursor appearance at the latest pointer position, respecting
capture, clipping and ancestor fallback. Uses current layout without
scheduling work; custom hosts call this after flush. Returns Default
outside the window. Component cursor callbacks run under the read-only
inspection restriction.

[Source](../src/pert_ui_runtime/host.rs.html#660)[§](#method.clear_focus)

#### fn [clear_focus](host/trait.HostIntegration.html#tymethod.clear_focus)(&mut self)

Handles window focus loss: clears all held keys and cancels focused
edits. [Read more](host/trait.HostIntegration.html#tymethod.clear_focus)

[Source](../src/pert_ui_runtime/host.rs.html#663)[§](#method.cancel_input)

#### fn [cancel_input](host/trait.HostIntegration.html#tymethod.cancel_input)(&mut self) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[()](https://doc.rust-lang.org/1.99.1/std/primitive.unit.html), [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\>

Cancels active input after a host/component failure without transferring
focus. [Read
more](host/trait.HostIntegration.html#tymethod.cancel_input)

[Source](../src/pert_ui_runtime/host.rs.html#666)[§](#method.captures_supported)

#### fn [captures_supported](host/trait.HostIntegration.html#tymethod.captures_supported)(&self) -\> [bool](https://doc.rust-lang.org/1.99.1/std/primitive.bool.html)

Whether a host has declared that it renders capture jobs.

[Source](../src/pert_ui_runtime/host.rs.html#669)[§](#method.pending_captures)

#### fn [pending_captures](host/trait.HostIntegration.html#tymethod.pending_captures)(&self) -\> [Vec](https://doc.rust-lang.org/1.99.1/alloc/vec/struct.Vec.html "struct alloc::vec::Vec")\<[CaptureJob](struct.CaptureJob.html "struct pert_ui::CaptureJob")\>

The captures waiting for a host to render their windows, oldest first.
[Read more](host/trait.HostIntegration.html#tymethod.pending_captures)

[Source](../src/pert_ui_runtime/host.rs.html#672)[§](#method.capture_scene)

#### fn [capture_scene](host/trait.HostIntegration.html#tymethod.capture_scene)( &mut self, job: &[CaptureJob](struct.CaptureJob.html "struct pert_ui::CaptureJob"), ) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[DisplayList](struct.DisplayList.html "struct pert_ui::DisplayList"), [CaptureError](enum.CaptureError.html "enum pert_ui::CaptureError")\>

Composes the scene a host renders for `job`: the job’s window, with
overlays only when the job asks for them. Selects that window, as
[`select_window`](host/trait.HostIntegration.html#tymethod.select_window "method pert_ui::host::HostIntegration::select_window")
does; a host presenting another window reselects it afterwards. Requires
flushed updates, like
[`paint`](struct.Runtime.html#method.paint "method pert_ui::Runtime::paint").
[Read more](host/trait.HostIntegration.html#tymethod.capture_scene)

[Source](../src/pert_ui_runtime/host.rs.html#675)[§](#method.complete_capture)

#### fn [complete_capture](host/trait.HostIntegration.html#tymethod.complete_capture)( &self, job: &[CaptureJob](struct.CaptureJob.html "struct pert_ui::CaptureJob"), frame: [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[FrameImage](struct.FrameImage.html "struct pert_ui::FrameImage"), [CaptureError](enum.CaptureError.html "enum pert_ui::CaptureError")\>, ) -\> [bool](https://doc.rust-lang.org/1.99.1/std/primitive.bool.html)

Hands a rendered frame, or the error that prevented one, to the capture
`job` names, answering its request. Returns whether the job was pending
here. The job is identified by the request it was listed for, as
[`capture_scene`](host/trait.HostIntegration.html#tymethod.capture_scene "method pert_ui::host::HostIntegration::capture_scene")
identifies it, so a job already completed, or one another runtime listed
even with the same id, completes nothing and returns `false`. [Read
more](host/trait.HostIntegration.html#tymethod.complete_capture)

[Source](../src/pert_ui_runtime/host.rs.html#678)[§](#method.set_clipboard)

#### fn [set_clipboard](host/trait.HostIntegration.html#tymethod.set_clipboard)( &mut self, clipboard: impl [Clipboard](trait.Clipboard.html "trait pert_ui::Clipboard") + 'static, ) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[()](https://doc.rust-lang.org/1.99.1/std/primitive.unit.html), [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\>

Installs a UI-thread clipboard service, replacing and dropping the old
service outside internal borrows. Runtime shutdown drops the service
even when UI handles survive. Native App configures this before
initialization; headless hosts may inject a service for deterministic
tests. [Read
more](host/trait.HostIntegration.html#tymethod.set_clipboard)

[Source](../src/pert_ui_runtime/host.rs.html#681)[§](#method.take_shutdown_errors)

#### fn [take_shutdown_errors](host/trait.HostIntegration.html#tymethod.take_shutdown_errors)(&mut self) -\> [Vec](https://doc.rust-lang.org/1.99.1/alloc/vec/struct.Vec.html "struct alloc::vec::Vec")\<[RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\>

Drains typed errors retained by infallible cleanup after
[`Runtime::shutdown`](struct.Runtime.html#method.shutdown "method pert_ui::Runtime::shutdown").
[Read
more](host/trait.HostIntegration.html#tymethod.take_shutdown_errors)

[Source](../src/pert_ui_runtime/signal.rs.html#168)[§](#impl-StateOwner-for-Runtime)

### impl [StateOwner](trait.StateOwner.html "trait pert_ui::StateOwner") for [Runtime](struct.Runtime.html "struct pert_ui::Runtime")

[Source](../src/pert_ui_runtime/signal.rs.html#169)[§](#method.signal-1)

#### fn [signal](trait.StateOwner.html#tymethod.signal)\<T\>(&self, value: T) -\> [Signal](struct.Signal.html "struct pert_ui::Signal")\<T\>

Creates a signal owned by this owner’s runtime, holding `value`.

[Source](../src/pert_ui_runtime/signal.rs.html#172)[§](#method.ui_settings-1)

#### fn [ui_settings](trait.StateOwner.html#tymethod.ui_settings)(&self) -\> [UiSettings](struct.UiSettings.html "struct pert_ui::UiSettings")

Shared UI-thread presentation settings for this owner’s runtime. [Read
more](trait.StateOwner.html#tymethod.ui_settings)

## Auto Trait Implementations[§](#synthetic-implementations)

[§](#impl-Freeze-for-Runtime)

### impl \![Freeze](https://doc.rust-lang.org/1.99.1/core/marker/trait.Freeze.html "trait core::marker::Freeze") for [Runtime](struct.Runtime.html "struct pert_ui::Runtime")

[§](#impl-RefUnwindSafe-for-Runtime)

### impl \![RefUnwindSafe](https://doc.rust-lang.org/1.99.1/core/panic/unwind_safe/trait.RefUnwindSafe.html "trait core::panic::unwind_safe::RefUnwindSafe") for [Runtime](struct.Runtime.html "struct pert_ui::Runtime")

[§](#impl-Send-for-Runtime)

### impl \![Send](https://doc.rust-lang.org/1.99.1/core/marker/trait.Send.html "trait core::marker::Send") for [Runtime](struct.Runtime.html "struct pert_ui::Runtime")

[§](#impl-Sync-for-Runtime)

### impl \![Sync](https://doc.rust-lang.org/1.99.1/core/marker/trait.Sync.html "trait core::marker::Sync") for [Runtime](struct.Runtime.html "struct pert_ui::Runtime")

[§](#impl-UnwindSafe-for-Runtime)

### impl \![UnwindSafe](https://doc.rust-lang.org/1.99.1/core/panic/unwind_safe/trait.UnwindSafe.html "trait core::panic::unwind_safe::UnwindSafe") for [Runtime](struct.Runtime.html "struct pert_ui::Runtime")

[§](#impl-Unpin-for-Runtime)

### impl [Unpin](https://doc.rust-lang.org/1.99.1/core/marker/trait.Unpin.html "trait core::marker::Unpin") for [Runtime](struct.Runtime.html "struct pert_ui::Runtime")

[§](#impl-UnsafeUnpin-for-Runtime)

### impl [UnsafeUnpin](https://doc.rust-lang.org/1.99.1/core/marker/trait.UnsafeUnpin.html "trait core::marker::UnsafeUnpin") for [Runtime](struct.Runtime.html "struct pert_ui::Runtime")

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
