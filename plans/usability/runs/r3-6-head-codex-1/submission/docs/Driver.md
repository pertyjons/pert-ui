[pert_ui](../index.html)::[testing](index.html)

# Struct Driver Copy item path

[Source](../../src/pert_ui_runtime/testing.rs.html#264)

``` rust
pub struct Driver<'a> { /* private fields */ }
```

Expand description

Drives a runtime as a user would, one settled step at a time.

A driver borrows the
[`Runtime`](../struct.Runtime.html "struct pert_ui::Runtime") and keeps
no state of its own: its clock is
[`Runtime::now`](../struct.Runtime.html#method.now "method pert_ui::Runtime::now"),
so a test builds one where it needs it, `Driver::new(&mut ui)`, or keeps
one for a block and reaches the runtime through
[`Self::runtime`](struct.Driver.html#method.runtime "method pert_ui::testing::Driver::runtime")
and
[`Self::runtime_mut`](struct.Driver.html#method.runtime_mut "method pert_ui::testing::Driver::runtime_mut").
Application state a test reads, such as signals or an `Rc` model, stays
outside it.

## [§](#settling)Settling

Each action ends with
[`Self::settle`](struct.Driver.html#method.settle "method pert_ui::testing::Driver::settle"),
one
[`Runtime::flush`](../struct.Runtime.html#method.flush "method pert_ui::Runtime::flush")
at
[`Self::FRAME`](struct.Driver.html#associatedconstant.FRAME "associated constant pert_ui::testing::Driver::FRAME")
past the runtime’s clock. One flush settles what `flush` settles:
updates, layout, focus and hover feedback and the notifications the
input requested, within
[`Runtime::MAX_FLUSH_WAVES`](../struct.Runtime.html#associatedconstant.MAX_FLUSH_WAVES "associated constant pert_ui::Runtime::MAX_FLUSH_WAVES").
It is not quiescence. Work a callback defers to a later flush, a
background result not yet delivered and timers due after the new time
stay pending;
[`Self::wait_until`](struct.Driver.html#method.wait_until "method pert_ui::testing::Driver::wait_until")
and
[`Self::advance`](struct.Driver.html#method.advance "method pert_ui::testing::Driver::advance")
reach them. Whether anything is left, such as an animation or a timer
that keeps running, is
[`Self::quiet_for`](struct.Driver.html#method.quiet_for "method pert_ui::testing::Driver::quiet_for")’s
check.

## [§](#where-input-goes)Where input goes

Input goes where
[`Runtime`](../struct.Runtime.html "struct pert_ui::Runtime")’s input
methods send it: to the selected window. The driver never selects
another window itself, so a click followed by typing stays in one place.
A test of a detached window selects it with
[`HostIntegration::select_window`](../host/trait.HostIntegration.html#tymethod.select_window "method pert_ui::host::HostIntegration::select_window")
first;
[`Self::click`](struct.Driver.html#method.click "method pert_ui::testing::Driver::click"),
[`Self::click_item`](struct.Driver.html#method.click_item "method pert_ui::testing::Driver::click_item")
and
[`Self::replace_text`](struct.Driver.html#method.replace_text "method pert_ui::testing::Driver::replace_text")
refuse a target in another window
([`TestError::OtherWindow`](enum.TestError.html#variant.OtherWindow "variant pert_ui::testing::TestError::OtherWindow")).

## [§](#synthetic-keys)Synthetic keys

[`Self::key`](struct.Driver.html#method.key "method pert_ui::testing::Driver::key")
and
[`Self::chord`](struct.Driver.html#method.chord "method pert_ui::testing::Driver::chord")
enter through
[`Runtime::command_key_input`](../struct.Runtime.html#method.command_key_input "method pert_ui::Runtime::command_key_input"),
the native host’s entry point, so application commands see them. They
build
[`CommandKeyInput::new`](../struct.CommandKeyInput.html#method.new "associated function pert_ui::CommandKeyInput::new")
events: the base and logical identities are the same key, the physical
code is the one given, and no event carries text or a composition. They
are for named keys and command chords, not for typing, keyboard layouts,
AltGr, dead keys or an input method. Text is typed with
[`Self::type_text`](struct.Driver.html#method.type_text "method pert_ui::testing::Driver::type_text").
A test of a printable key’s native press, which carries text, of a
layout or of a composition builds
[`CommandKeyInput`](../struct.CommandKeyInput.html "struct pert_ui::CommandKeyInput")
itself, and a test of one component’s own key handling calls
[`Runtime::key_input`](../struct.Runtime.html#method.key_input "method pert_ui::Runtime::key_input").

## [§](#neutral-input-and-cleanup)Neutral input and cleanup

[`Self::key`](struct.Driver.html#method.key "method pert_ui::testing::Driver::key"),
[`Self::chord`](struct.Driver.html#method.chord "method pert_ui::testing::Driver::chord"),
[`Self::replace_text`](struct.Driver.html#method.replace_text "method pert_ui::testing::Driver::replace_text"),
[`Self::click`](struct.Driver.html#method.click "method pert_ui::testing::Driver::click")
and
[`Self::click_item`](struct.Driver.html#method.click_item "method pert_ui::testing::Driver::click_item")
require that no key is held and no modifier is reported
([`Runtime::keyboard`](../struct.Runtime.html#method.keyboard "method pert_ui::Runtime::keyboard")),
and report
[`TestError::InputHeld`](enum.TestError.html#variant.InputHeld "variant pert_ui::testing::TestError::InputHeld")
otherwise, so a helper never releases a press the test made through the
low-level methods. The runtime exposes no pointer-button state: a test
that pressed a button through
[`Runtime::pointer_input`](../struct.Runtime.html#method.pointer_input "method pert_ui::Runtime::pointer_input")
releases it before
[`Self::click`](struct.Driver.html#method.click "method pert_ui::testing::Driver::click")
or
[`Self::click_item`](struct.Driver.html#method.click_item "method pert_ui::testing::Driver::click_item").

A compound action releases everything it pressed, even when a step
fails: a key or button whose press returned an error is still released,
because delivery can report an unrelated error after admitting the
press. Cleanup goes on after a release fails. The action’s own error is
returned; when cleanup failed too,
[`TestError::Cleanup`](enum.TestError.html#variant.Cleanup "variant pert_ui::testing::TestError::Cleanup")
carries both, with the first cleanup failure, and later cleanup failures
of the same action are not reported. When only cleanup failed, its first
error is the result.

``` rust
use pert_ui::prelude::*;
use pert_ui::testing::*;

let mut ui = Runtime::new();
ui.set_viewport(Viewport::new(300.0, 100.0, 1.0)?)?;
let text = ui.signal(String::new());
let field = ui.mount_tree(Element::new("field", TextField::new(text.clone())))?;
let mut driver = Driver::new(&mut ui);
driver.settle()?;
driver.runtime_mut().focus(&field)?;
driver.type_text("Hello")?;
assert_eq!(text.get()?, "Hello");
assert_eq!(driver.runtime().focused(), Some(field));
```

## Implementations[§](#implementations)

[Source](../../src/pert_ui_runtime/testing.rs.html#335)[§](#impl-Driver%3C'a%3E)

### impl\<'a\> [Driver](struct.Driver.html "struct pert_ui::testing::Driver")\<'a\>

[Source](../../src/pert_ui_runtime/testing.rs.html#338)

#### pub const [FRAME](#associatedconstant.FRAME): [Duration](https://doc.rust-lang.org/1.99.1/core/time/struct.Duration.html "struct core::time::Duration")

The clock step of one
[`Self::settle`](struct.Driver.html#method.settle "method pert_ui::testing::Driver::settle"):
16 ms, about one frame at 60 Hz.

[Source](../../src/pert_ui_runtime/testing.rs.html#341)

#### pub fn [new](#method.new)(runtime: &'a mut [Runtime](../struct.Runtime.html "struct pert_ui::Runtime")) -\> [Driver](struct.Driver.html "struct pert_ui::testing::Driver")\<'a\>

Drives `runtime` until the driver is dropped.

[Source](../../src/pert_ui_runtime/testing.rs.html#346)

#### pub fn [runtime](#method.runtime)(&self) -\> &[Runtime](../struct.Runtime.html "struct pert_ui::Runtime")

The driven runtime, for reading what it shows.

[Source](../../src/pert_ui_runtime/testing.rs.html#352)

#### pub fn [runtime_mut](#method.runtime_mut)(&mut self) -\> &mut [Runtime](../struct.Runtime.html "struct pert_ui::Runtime")

The driven runtime, for input or changes the driver has no action for,
such as
[`HostIntegration::select_window`](../host/trait.HostIntegration.html#tymethod.select_window "method pert_ui::host::HostIntegration::select_window")
or
[`Runtime::focus`](../struct.Runtime.html#method.focus "method pert_ui::Runtime::focus").

[Source](../../src/pert_ui_runtime/testing.rs.html#366)

#### pub fn [settle](#method.settle)(&mut self) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[FlushReport](../struct.FlushReport.html "struct pert_ui::FlushReport"), [TestError](enum.TestError.html "enum pert_ui::testing::TestError")\>

Flushes once at
[`Self::FRAME`](struct.Driver.html#associatedconstant.FRAME "associated constant pert_ui::testing::Driver::FRAME")
past
[`Runtime::now`](../struct.Runtime.html#method.now "method pert_ui::Runtime::now").

This is the flush every action ends with. It settles what one
[`Runtime::flush`](../struct.Runtime.html#method.flush "method pert_ui::Runtime::flush")
settles, not everything pending; see
[settling](struct.Driver.html#settling "struct pert_ui::testing::Driver").
A test of repeated ticks, such as a debounce that counts frames, calls
it in a loop.

##### [§](#errors)Errors

[`TestError::ClockOverflow`](enum.TestError.html#variant.ClockOverflow "variant pert_ui::testing::TestError::ClockOverflow")
when the clock cannot advance that far, and the errors of
[`Runtime::flush`](../struct.Runtime.html#method.flush "method pert_ui::Runtime::flush")
as
[`TestError::Runtime`](enum.TestError.html#variant.Runtime "variant pert_ui::testing::TestError::Runtime").

[Source](../../src/pert_ui_runtime/testing.rs.html#394)

#### pub fn [advance](#method.advance)(&mut self, by: [Duration](https://doc.rust-lang.org/1.99.1/core/time/struct.Duration.html "struct core::time::Duration")) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[FlushReport](../struct.FlushReport.html "struct pert_ui::FlushReport"), [TestError](enum.TestError.html "enum pert_ui::testing::TestError")\>

Flushes once at `by` past
[`Runtime::now`](../struct.Runtime.html#method.now "method pert_ui::Runtime::now").

One deadline jump: missed deadlines produce one update, as
[`Runtime::flush`](../struct.Runtime.html#method.flush "method pert_ui::Runtime::flush")
documents, so `advance(320 ms)` is not twenty 16 ms ticks. Use it to let
an interval pass, such as a double-click interval between two separate
clicks.

##### [§](#errors-1)Errors

[`TestError::ClockOverflow`](enum.TestError.html#variant.ClockOverflow "variant pert_ui::testing::TestError::ClockOverflow")
when the clock cannot advance by `by`, checked before anything runs, and
the errors of
[`Runtime::flush`](../struct.Runtime.html#method.flush "method pert_ui::Runtime::flush")
as
[`TestError::Runtime`](enum.TestError.html#variant.Runtime "variant pert_ui::testing::TestError::Runtime").

``` rust
use pert_ui::prelude::*;
use pert_ui::testing::Driver;
use std::time::Duration;

let mut ui = Runtime::new();
let mut driver = Driver::new(&mut ui);
driver.advance(Duration::from_secs(1))?;
assert_eq!(driver.runtime().now(), Duration::from_secs(1));
```

[Source](../../src/pert_ui_runtime/testing.rs.html#433-437)

#### pub fn [wait_until](#method.wait_until)( &mut self, timeout: [Duration](https://doc.rust-lang.org/1.99.1/core/time/struct.Duration.html "struct core::time::Duration"), condition: impl [FnMut](https://doc.rust-lang.org/1.99.1/core/ops/function/trait.FnMut.html "trait core::ops::function::FnMut")(&[Runtime](../struct.Runtime.html "struct pert_ui::Runtime")) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[bool](https://doc.rust-lang.org/1.99.1/std/primitive.bool.html), [RuntimeError](../enum.RuntimeError.html "enum pert_ui::RuntimeError")\>, ) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[()](https://doc.rust-lang.org/1.99.1/std/primitive.unit.html), [TestError](enum.TestError.html "enum pert_ui::testing::TestError")\>

Flushes until `condition` holds, for background work.

Each round flushes at the same
[`Runtime::now`](../struct.Runtime.html#method.now "method pert_ui::Runtime::now"),
so the runtime clock does not move: work already due and delivered
background results run, and future deadlines do not. When `condition`
does not hold yet, the driver sleeps 1 ms of wall time and repeats, for
at most `timeout` of wall time. It polls rather than installing
[`HostIntegration::set_wake_delivery`](../host/trait.HostIntegration.html#tymethod.set_wake_delivery "method pert_ui::host::HostIntegration::set_wake_delivery"),
which would replace a delivery the test or its host set.

##### [§](#errors-2)Errors

[`TestError::TimedOut`](enum.TestError.html#variant.TimedOut "variant pert_ui::testing::TestError::TimedOut")
with the wall time waited when `condition` still does not hold after
`timeout`; the errors of
[`Runtime::flush`](../struct.Runtime.html#method.flush "method pert_ui::Runtime::flush")
and of `condition` as
[`TestError::Runtime`](enum.TestError.html#variant.Runtime "variant pert_ui::testing::TestError::Runtime").

``` rust
use pert_ui::prelude::*;
use pert_ui::testing::Driver;
use std::time::Duration;

let mut ui = Runtime::new();
let result = ui.signal(None::<u32>);
let target = result.clone();
let _task = ui.spawn(|| 6 * 7, move |answer| target.set(Some(answer)))?;
let mut driver = Driver::new(&mut ui);
driver.wait_until(Duration::from_secs(5), |_| Ok(result.get()?.is_some()))?;
assert_eq!(result.get()?, Some(42));
```

[Source](../../src/pert_ui_runtime/testing.rs.html#563)

#### pub fn [quiet_for](#method.quiet_for)( &mut self, interval: [Duration](https://doc.rust-lang.org/1.99.1/core/time/struct.Duration.html "struct core::time::Duration"), ) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[QuietReport](struct.QuietReport.html "struct pert_ui::testing::QuietReport"), [TestError](enum.TestError.html "enum pert_ui::testing::TestError")\>

Paints the selected window, lets `interval` of runtime time pass in
[`Self::FRAME`](struct.Driver.html#associatedconstant.FRAME "associated constant pert_ui::testing::Driver::FRAME")
steps without painting, and reports what happened.

This is the check that a window is idle: nothing updates, nothing is
scheduled and nothing needs painting. A test paints first so that damage
from before the call does not count, and lets time pass in frames
because one jump of the whole interval runs a periodic component once,
as
[`Self::advance`](struct.Driver.html#method.advance "method pert_ui::testing::Driver::advance")
documents. The report’s
[`QuietReport::is_idle`](struct.QuietReport.html#method.is_idle "method pert_ui::testing::QuietReport::is_idle")
is the usual assertion; its fields say which kind of work remains.

##### [§](#the-interval)The interval

- **Baseline:** one
  [`Runtime::flush`](../struct.Runtime.html#method.flush "method pert_ui::Runtime::flush")
  at the current
  [`Runtime::now`](../struct.Runtime.html#method.now "method pert_ui::Runtime::now"),
  then one
  [`Runtime::paint`](../struct.Runtime.html#method.paint "method pert_ui::Runtime::paint"),
  whose display list is dropped. Work already due when the call starts
  runs here and is not counted. The baseline does not move the clock, so
  the clock moves by exactly `interval`.
- **Steps:** the clock advances to `now + interval` in
  [`Self::FRAME`](struct.Driver.html#associatedconstant.FRAME "associated constant pert_ui::testing::Driver::FRAME")
  steps, one
  [`Runtime::flush`](../struct.Runtime.html#method.flush "method pert_ui::Runtime::flush")
  each, and nothing is painted between them, so damage made anywhere in
  the interval is still pending at its end. A final partial frame is one
  shorter step that lands exactly on the end: `quiet_for(40 ms)` flushes
  at 16, 32 and 40 ms past the start.
- **A zero interval** makes no step: the report holds the state right
  after the baseline, a check that nothing is scheduled now without
  letting time pass.
- **A refused baseline:** when the baseline flush leaves work queued,
  such as a command a notification queued, the runtime refuses the paint
  with
  [`DrawError::PendingUpdates`](../enum.DrawError.html#variant.PendingUpdates "variant pert_ui::DrawError::PendingUpdates").
  That work is activity, not an error: the interval runs from the
  unpainted state and
  [`QuietReport::baseline_painted`](struct.QuietReport.html#structfield.baseline_painted "field pert_ui::testing::QuietReport::baseline_painted")
  is false, so the report is not idle. The refusal is told apart from a
  component that returns `PendingUpdates` itself by
  [`Runtime::last_failure`](../struct.Runtime.html#method.last_failure "method pert_ui::Runtime::last_failure"),
  which names no
  [`FailurePhase::Paint`](../enum.FailurePhase.html#variant.Paint "variant pert_ui::FailurePhase::Paint")
  site after a refusal; a component’s own error ends the call.

##### [§](#scope)Scope

The selected window’s own updates, deadlines, timers and presentation,
as
[`Self::settle`](struct.Driver.html#method.settle "method pert_ui::testing::Driver::settle")
covers them, plus the work all windows share that
[`Runtime::next_deadline`](../struct.Runtime.html#method.next_deadline "method pert_ui::Runtime::next_deadline")
includes: interval timers, view-scale settling, polled live sources,
queued deferred tasks and background results that have arrived,
whichever window they serve. Another window’s own timer or animation
does not make the report non-idle, because the driver’s flushes do not
run it; a test checks that window by selecting it with
[`HostIntegration::select_window`](../host/trait.HostIntegration.html#tymethod.select_window "method pert_ui::host::HostIntegration::select_window")
and calling `quiet_for` again.
[`QuietReport::components_updated`](struct.QuietReport.html#structfield.components_updated "field pert_ui::testing::QuietReport::components_updated")
counts the notification followups a step delivers in other visible
windows, as
[`FlushReport`](../struct.FlushReport.html "struct pert_ui::FlushReport")
does.

A background worker still computing is not observed: its thread runs
outside the runtime until its result arrives, so a report can be idle
while a worker of
[`Runtime::spawn`](../struct.Runtime.html#method.spawn "method pert_ui::Runtime::spawn")
is blocked or busy. A result that arrives during the interval is
delivered by a step and can show as activity through what its delivery
changes, but delivery alone does not make the report busy, and a result
that arrives later is not foreseen. A test of background work waits for
its result with
[`Self::wait_until`](struct.Driver.html#method.wait_until "method pert_ui::testing::Driver::wait_until")
first, then checks that the window is quiet.

A runtime operation nested inside a step runs flushes of its own: a
queued command’s admission flushes every eligible window, a queued
reveal flushes its target’s window, and a deferred task or interval tick
may call any operation that flushes. Those flushes can run other
windows’ due work, and their updates are not counted. Command admission
and reveals restore the selected window, so the final observations,
[`QuietReport::next_deadline`](struct.QuietReport.html#structfield.next_deadline "field pert_ui::testing::QuietReport::next_deadline")
and
[`QuietReport::needs_paint`](struct.QuietReport.html#structfield.needs_paint "field pert_ui::testing::QuietReport::needs_paint"),
stay the selected window’s.

##### [§](#cost)Cost

One flush per
[`Self::FRAME`](struct.Driver.html#associatedconstant.FRAME "associated constant pert_ui::testing::Driver::FRAME")
of runtime time and no wall-clock sleep: a second is 63 flushes. Choose
an interval longer than the longest period the test means to rule out: a
[`Spinner`](../struct.Spinner.html "struct pert_ui::Spinner") steps
every 100 ms and a focused text field’s caret blinks every 500 ms. A
deadline beyond the interval, such as a toast’s expiry, still shows in
[`QuietReport::next_deadline`](struct.QuietReport.html#structfield.next_deadline "field pert_ui::testing::QuietReport::next_deadline"),
so it is never mistaken for idleness.

No input is sent, so neutral input is not required. A component-owned
held action that schedules work, such as a number field’s held step
button, is activity the report shows.

##### [§](#errors-3)Errors

[`TestError::ClockOverflow`](enum.TestError.html#variant.ClockOverflow "variant pert_ui::testing::TestError::ClockOverflow")
when `now + interval` overflows, checked before anything is flushed or
painted. A flush’s error, of the baseline or of a step, as
[`TestError::Runtime`](enum.TestError.html#variant.Runtime "variant pert_ui::testing::TestError::Runtime"),
ending the call with the clock where that flush left it. A paint error
other than the refusal above, a component’s own
[`DrawError::PendingUpdates`](../enum.DrawError.html#variant.PendingUpdates "variant pert_ui::DrawError::PendingUpdates")
included, as
[`TestError::Draw`](enum.TestError.html#variant.Draw "variant pert_ui::testing::TestError::Draw"),
or as
[`TestError::Runtime`](enum.TestError.html#variant.Runtime "variant pert_ui::testing::TestError::Runtime")
for
[`DrawError::Runtime`](../enum.DrawError.html#variant.Runtime "variant pert_ui::DrawError::Runtime").

``` rust
use pert_ui::prelude::*;
use pert_ui::testing::*;
use std::time::Duration;

let mut ui = Runtime::new();
ui.set_viewport(Viewport::new(300.0, 100.0, 1.0)?)?;
let busy = ui.signal(true);
ui.mount_tree(Element::new("busy", Spinner::bound(busy.clone())))?;
let mut driver = Driver::new(&mut ui);
driver.settle()?;
// A spinner steps every 100 ms.
let report = driver.quiet_for(Duration::from_millis(250))?;
assert!(!report.is_idle());
assert!(report.components_updated >= 2 && report.needs_paint);
busy.set(false)?;
assert!(driver.quiet_for(Duration::from_millis(250))?.is_idle());
```

[Source](../../src/pert_ui_runtime/testing.rs.html#633)

#### pub fn [find](#method.find)( &self, from: &[WidgetId](../struct.WidgetId.html "struct pert_ui::WidgetId"), path: &\[&[str](https://doc.rust-lang.org/1.99.1/std/primitive.str.html)\], ) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[WidgetId](../struct.WidgetId.html "struct pert_ui::WidgetId"), [TestError](enum.TestError.html "enum pert_ui::testing::TestError")\>

The component at the key path `path` below `from`, as
[`Runtime::find`](../struct.Runtime.html#method.find "method pert_ui::Runtime::find")
resolves it.

When nothing is mounted there, the error names the path and the longest
leading part of it that does resolve below `from`, so a test sees which
segment failed. A prefix resolves when `find` returns a component for it
or refuses it as ambiguous, since `find` refuses only several matches of
the last segment. Only the failure path looks for it, with at most one
further lookup per shorter prefix.

##### [§](#errors-4)Errors

[`TestError::NotFound`](enum.TestError.html#variant.NotFound "variant pert_ui::testing::TestError::NotFound")
with the path and its resolved prefix when nothing is mounted there, and
the errors of
[`Runtime::find`](../struct.Runtime.html#method.find "method pert_ui::Runtime::find")
as
[`TestError::Runtime`](enum.TestError.html#variant.Runtime "variant pert_ui::testing::TestError::Runtime"),
such as `AmbiguousKey`.

``` rust
use pert_ui::prelude::*;
use pert_ui::testing::*;

let mut ui = Runtime::new();
let root = ui.mount_tree(Element::new("form", Column::new([
    Element::new("r1", Row::new([Element::new("name", Label::new("Name"))])),
])))?;
let driver = Driver::new(&mut ui);
let error = driver.find(&root, &["r1", "missing"]).unwrap_err();
assert!(matches!(&error, TestError::NotFound { resolved, .. } if resolved == &["r1"]));
assert_eq!(
    error.to_string(),
    "no component is mounted at r1/missing; the path resolves as far as r1",
);
```

[Source](../../src/pert_ui_runtime/testing.rs.html#735)

#### pub fn [address_children](#method.address_children)( &self, parent: &[WidgetId](../struct.WidgetId.html "struct pert_ui::WidgetId"), ) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[Vec](https://doc.rust-lang.org/1.99.1/alloc/vec/struct.Vec.html "struct alloc::vec::Vec")\<[AddressChild](struct.AddressChild.html "struct pert_ui::testing::AddressChild")\>, [TestError](enum.TestError.html "enum pert_ui::testing::TestError")\>

The components one address segment below `parent`, in child order, each
with the segment
[`Runtime::find`](../struct.Runtime.html#method.find "method pert_ui::Runtime::find")
resolves it by.

A component is listed with segment `s` exactly when
`Runtime::find(parent, &[s])` matches it: `find` returns it, or refuses
with
[`RuntimeError::AmbiguousKey`](../enum.RuntimeError.html#variant.AmbiguousKey "variant pert_ui::RuntimeError::AmbiguousKey")
because a second listed component has the same segment. Both are then
listed, in child order, and `find` refuses that address. The walk goes
through structural levels in child order: an element contributes its key
and a slot its slot name, and nothing below either is listed; a part
contributes its part name and its descendants are listed too, since they
address without it.

The result is relative to `parent`, as `find` resolves from it, so a
structural parent, a slot or a part is a valid parent. It includes
hidden components, components presented in a detached window and every
row a virtualized collection has mounted, offscreen retained ones
included. A row that is not mounted is not listed: the collection’s
controller mounts it, such as
[`ListController::reveal`](../struct.ListController.html#method.reveal "method pert_ui::ListController::reveal")
or
[`TableController::reveal`](../struct.TableController.html#method.reveal "method pert_ui::TableController::reveal"),
and the next settle lists it.

| Parent | Result |
|----|----|
| An [`Each`](../struct.Each.html "struct pert_ui::Each") with its default container | its rows in list order, each under its item key: the container is structural |
| An `Each` with [`within`](../struct.Each.html#method.within "method pert_ui::Each::within") | the one container, under its key; `address_children(&driver.find(&each, &["tags"])?)` lists the rows |
| A [`VirtualList`](../struct.VirtualList.html "struct pert_ui::VirtualList") | every mounted row in index order: the visible lines, the overscan, a row pinned by focus or a pointer gesture and the `keep_mounted` row |
| A [`Table`](../struct.Table.html "struct pert_ui::Table") | its mounted rows in source order, with the same retention; its body and edit-end reporter are structural |
| A row the application marks [`Element::structural`](../struct.Element.html#method.structural "method pert_ui::Element::structural") | the row is looked through: its addressable children appear in its place |

Child order is description order. For every built-in collection that is
display order, reading order for a lane grid, and for an external
container it is whatever order the container describes. An `Each` item
keeps the addressing its element had; `Each` replaces only its key.

[`Selector::children`](../enum.Selector.html#method.children "associated function pert_ui::Selector::children")
of `parent`’s path matches the same set only when `parent` is an element
or a slot whose path no other element or slot has, and its results are
in incarnation order. Below a part, whose name its descendants do not
carry, it finds none of what this lists, and when two components share a
path it also returns the other one’s children.
[`Runtime::children`](../struct.Runtime.html#method.children "method pert_ui::Runtime::children")
stays the structure walk for hosts and custom components: it lists
structural wrappers too.

The call only reads: it flushes nothing and moves no clock. It visits
each component reachable from `parent` through structural and part
levels once.

##### [§](#errors-5)Errors

`ForeignRuntime` or `Unmounted` as
[`TestError::Runtime`](enum.TestError.html#variant.Runtime "variant pert_ui::testing::TestError::Runtime")
for a `parent` that is not a mounted component of this runtime, as
[`Runtime::children`](../struct.Runtime.html#method.children "method pert_ui::Runtime::children").
A leaf lists nothing.

``` rust
use pert_ui::prelude::*;
use pert_ui::testing::*;

let mut ui = Runtime::new();
ui.set_viewport(Viewport::new(300.0, 200.0, 1.0)?)?;
let tasks = ui.signal(vec!["write".to_owned(), "ship".to_owned()]);
let list = ui.mount_tree(
    Each::new(&tasks, Clone::clone, |_, task| Label::new(task.as_str())).key("tasks"),
)?;
let mut driver = Driver::new(&mut ui);
driver.settle()?;
tasks.set(vec!["ship".to_owned(), "write".to_owned()])?;
driver.settle()?;
let rows = driver.address_children(&list)?;
let segments: Vec<&str> = rows.iter().map(|row| row.segment.as_str()).collect();
assert_eq!(segments, ["ship", "write"]);
assert_eq!(rows[0].id, driver.find(&list, &["ship"])?);
```

[Source](../../src/pert_ui_runtime/testing.rs.html#755)

#### pub fn [semantics](#method.semantics)(&self, id: &[WidgetId](../struct.WidgetId.html "struct pert_ui::WidgetId")) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[Semantics](../struct.Semantics.html "struct pert_ui::Semantics"), [TestError](enum.TestError.html "enum pert_ui::testing::TestError")\>

The semantics `id` reports now, through one
[`Runtime::query`](../struct.Runtime.html#method.query "method pert_ui::Runtime::query")
with
[`Selector::id`](../enum.Selector.html#method.id "associated function pert_ui::Selector::id")
at
[`Detail::Semantics`](../enum.Detail.html#variant.Semantics "variant pert_ui::Detail::Semantics").

Static text, such as a label’s or a button’s, is the name; an editable
or stateful control’s content is the value. Empty text is no name: an
empty label’s `name` is `None`.

##### [§](#errors-6)Errors

[`TestError::NoSemantics`](enum.TestError.html#variant.NoSemantics "variant pert_ui::testing::TestError::NoSemantics")
for a component that reports none, and `ForeignRuntime` or `Unmounted`
as
[`TestError::Runtime`](enum.TestError.html#variant.Runtime "variant pert_ui::testing::TestError::Runtime")
for a handle that is not a mounted component of this runtime.

[Source](../../src/pert_ui_runtime/testing.rs.html#802)

#### pub fn [click](#method.click)(&mut self, id: &[WidgetId](../struct.WidgetId.html "struct pert_ui::WidgetId")) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[()](https://doc.rust-lang.org/1.99.1/std/primitive.unit.html), [TestError](enum.TestError.html "enum pert_ui::testing::TestError")\>

Clicks the middle of `id` with the primary button, then settles.

The target is revealed first with
[`Runtime::reveal`](../struct.Runtime.html#method.reveal "method pert_ui::Runtime::reveal"),
which opens hiding containers and scrolls it into view and settles
presentation itself. The press and release then go through
[`Runtime::pointer_input`](../struct.Runtime.html#method.pointer_input "method pert_ui::Runtime::pointer_input")
at the middle of
[`Runtime::bounds`](../struct.Runtime.html#method.bounds "method pert_ui::Runtime::bounds").
Whether the press reaches the target is not checked: a surface over it
is a result the test asserts on.

The clock moves only by the settle, so two clicks at one point are
[`Self::FRAME`](struct.Driver.html#associatedconstant.FRAME "associated constant pert_ui::testing::Driver::FRAME")
apart. That is within
[`ClickCounter::DEFAULT_INTERVAL`](../struct.ClickCounter.html#associatedconstant.DEFAULT_INTERVAL "associated constant pert_ui::ClickCounter::DEFAULT_INTERVAL"),
so a built-in control counts them as a double click, or continues a
press series that was already running; a component with its own interval
counts by that one. For two separate single clicks, put
[`Self::advance`](struct.Driver.html#method.advance "method pert_ui::testing::Driver::advance")
beyond the component’s interval between them.

Requires neutral keyboard input and no pointer button held; see [neutral
input](struct.Driver.html#neutral-input-and-cleanup "struct pert_ui::testing::Driver").

##### [§](#errors-7)Errors

Before any input: `ForeignRuntime` or `Unmounted` as
[`TestError::Runtime`](enum.TestError.html#variant.Runtime "variant pert_ui::testing::TestError::Runtime");
[`TestError::OtherWindow`](enum.TestError.html#variant.OtherWindow "variant pert_ui::testing::TestError::OtherWindow")
for a target outside the selected window;
[`TestError::InputHeld`](enum.TestError.html#variant.InputHeld "variant pert_ui::testing::TestError::InputHeld").
Then
[`TestError::NotRevealed`](enum.TestError.html#variant.NotRevealed "variant pert_ui::testing::TestError::NotRevealed")
with the refusal of a reveal that stopped, `Unmounted` as
[`TestError::Runtime`](enum.TestError.html#variant.Runtime "variant pert_ui::testing::TestError::Runtime")
for a target the reveal unmounted,
[`TestError::NotLaidOut`](enum.TestError.html#variant.NotLaidOut "variant pert_ui::testing::TestError::NotLaidOut")
for a target without bounds, and the errors of
[`Runtime::reveal`](../struct.Runtime.html#method.reveal "method pert_ui::Runtime::reveal"),
[`Runtime::pointer_input`](../struct.Runtime.html#method.pointer_input "method pert_ui::Runtime::pointer_input")
and the settle, with
[cleanup](struct.Driver.html#neutral-input-and-cleanup "struct pert_ui::testing::Driver")
as described.

[Source](../../src/pert_ui_runtime/testing.rs.html#924)

#### pub fn [click_item](#method.click_item)( &mut self, id: &[WidgetId](../struct.WidgetId.html "struct pert_ui::WidgetId"), index: [usize](https://doc.rust-lang.org/1.99.1/std/primitive.usize.html), ) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[()](https://doc.rust-lang.org/1.99.1/std/primitive.unit.html), [TestError](enum.TestError.html "enum pert_ui::testing::TestError")\>

Clicks the middle of the visible part of item `index` of `id`’s
[`Semantics::items`](../struct.Semantics.html#method.items "method pert_ui::Semantics::items")
with the primary button, then settles.

An item is a row a component draws itself rather than a child component,
such as a [`Table`](../struct.Table.html "struct pert_ui::Table")’s
column title, a menu command or a tab title; a component that gives an
item its rectangle
([`SemanticItem::local_bounds`](../struct.SemanticItem.html#method.local_bounds "method pert_ui::SemanticItem::local_bounds"))
reports it as
[`SemanticItem::bounds`](../struct.SemanticItem.html#structfield.bounds "field pert_ui::SemanticItem::bounds"),
and only such an item can be clicked. Among the built-in components, a
table’s column titles do, and a toast stack’s toasts, whose rectangle is
the whole toast rather than its action or Dismiss control, so a test
activates a toast through the stack’s keys
([`ToastHost`](../struct.ToastHost.html#inspection-and-tests "struct pert_ui::ToastHost")).

The items are read with
[`Self::semantics`](struct.Driver.html#method.semantics "method pert_ui::testing::Driver::semantics");
the component is then revealed with
[`Runtime::reveal`](../struct.Runtime.html#method.reveal "method pert_ui::Runtime::reveal"),
as
[`Self::click`](struct.Driver.html#method.click "method pert_ui::testing::Driver::click")
does, and its items are read again, since revealing flushes and may
scroll. The press and release go through
[`Runtime::pointer_input`](../struct.Runtime.html#method.pointer_input "method pert_ui::Runtime::pointer_input")
at the middle of the item’s bounds, the visible part of the item.

This is pointer input, not a semantic action: what the press does is the
component’s, and whether it reaches the component is not checked. On a
table title, a press within the resize grip of a column’s trailing edge
starts a resize rather than a sort, and the middle of a title that
horizontal scrolling leaves only a narrow visible part of can fall
there. The driver does not scroll a component’s own content to the item:
a table column scrolled out of view has no bounds; bring it into view
with Left or Right on the focused table, which scroll the active cell’s
column into view, or with a horizontal wheel through
[`Runtime::pointer_input`](../struct.Runtime.html#method.pointer_input "method pert_ui::Runtime::pointer_input").
A column can also be sorted from the keyboard, with Alt and Up or Down
on the active column.

The clock moves only by the settle, so two `click_item`s are
[`Self::FRAME`](struct.Driver.html#associatedconstant.FRAME "associated constant pert_ui::testing::Driver::FRAME")
apart and a component that counts clicks sees a double click, as for
[`Self::click`](struct.Driver.html#method.click "method pert_ui::testing::Driver::click");
put
[`Self::advance`](struct.Driver.html#method.advance "method pert_ui::testing::Driver::advance")
beyond its interval between them for two single clicks.

Requires neutral keyboard input and no pointer button held; see [neutral
input](struct.Driver.html#neutral-input-and-cleanup "struct pert_ui::testing::Driver").

##### [§](#errors-8)Errors

Before any input, with nothing changed: `ForeignRuntime` or `Unmounted`
as
[`TestError::Runtime`](enum.TestError.html#variant.Runtime "variant pert_ui::testing::TestError::Runtime");
[`TestError::OtherWindow`](enum.TestError.html#variant.OtherWindow "variant pert_ui::testing::TestError::OtherWindow")
for a target outside the selected window;
[`TestError::InputHeld`](enum.TestError.html#variant.InputHeld "variant pert_ui::testing::TestError::InputHeld");
[`TestError::NoSemantics`](enum.TestError.html#variant.NoSemantics "variant pert_ui::testing::TestError::NoSemantics")
for a component that reports none and
[`TestError::NoItem`](enum.TestError.html#variant.NoItem "variant pert_ui::testing::TestError::NoItem")
for an index it reports no item at. Then
[`TestError::NotRevealed`](enum.TestError.html#variant.NotRevealed "variant pert_ui::testing::TestError::NotRevealed")
with the refusal of a reveal that stopped, `Unmounted` as
[`TestError::Runtime`](enum.TestError.html#variant.Runtime "variant pert_ui::testing::TestError::Runtime")
for a target the reveal unmounted, `NoSemantics` or `NoItem` again when
the reveal changed the items,
[`TestError::ItemWithoutBounds`](enum.TestError.html#variant.ItemWithoutBounds "variant pert_ui::testing::TestError::ItemWithoutBounds")
for an item without bounds, and the errors of
[`Runtime::reveal`](../struct.Runtime.html#method.reveal "method pert_ui::Runtime::reveal"),
[`Runtime::pointer_input`](../struct.Runtime.html#method.pointer_input "method pert_ui::Runtime::pointer_input")
and the settle, with
[cleanup](struct.Driver.html#neutral-input-and-cleanup "struct pert_ui::testing::Driver")
as described.

``` rust
use pert_ui::prelude::*;
use pert_ui::testing::*;
use std::{cell::RefCell, rc::Rc};

struct Files;
impl TableItems for Files {
    fn len(&self) -> usize { 2 }
    fn key(&self, index: usize) -> Option<String> {
        (index < 2).then(|| format!("file{index}"))
    }
    fn cell(&self, index: usize, column: &str) -> Option<Element> {
        let text = match column {
            "name" => ["b.txt", "a.txt"].get(index)?,
            _ => ["2 KB", "1 KB"].get(index)?,
        };
        Some(Element::new("text", Label::new(*text)))
    }
}
let mut ui = Runtime::new();
ui.set_viewport(Viewport::new(320.0, 200.0, 1.0)?)?;
let controller = TableController::new(
    &ui,
    ListSelection::new(&ui, SelectionMode::Single),
    [
        TableColumn::new("name", "Name").sortable(true),
        TableColumn::new("size", "Size").sortable(true),
    ],
    TableSource::new(Files),
)?;
let requests = Rc::new(RefCell::new(Vec::new()));
let seen = requests.clone();
let table = ui.mount_tree(Element::new("files", Table::new(&controller, 24.0)?
    .on_sort(move |_, sort| {
        seen.borrow_mut().push((sort.column.clone(), sort.direction));
        Ok(())
    })))?;
let mut driver = Driver::new(&mut ui);
driver.settle()?;
driver.click_item(&table, 1)?;
assert_eq!(*requests.borrow(), [("size".to_owned(), SortDirection::Ascending)]);
```

[Source](../../src/pert_ui_runtime/testing.rs.html#1002)

#### pub fn [replace_text](#method.replace_text)( &mut self, id: &[WidgetId](../struct.WidgetId.html "struct pert_ui::WidgetId"), text: &[str](https://doc.rust-lang.org/1.99.1/std/primitive.str.html), ) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[()](https://doc.rust-lang.org/1.99.1/std/primitive.unit.html), [TestError](enum.TestError.html "enum pert_ui::testing::TestError")\>

Replaces the whole text of the editor `id` with `text`, then settles.

Focuses the editor, selects all of its text with
[`TextCommand::SelectAll`](../enum.TextCommand.html#variant.SelectAll "variant pert_ui::TextCommand::SelectAll")
through
[`Runtime::text_command`](../struct.Runtime.html#method.text_command "method pert_ui::Runtime::text_command"),
then types `text` with
[`Runtime::text_input`](../struct.Runtime.html#method.text_input "method pert_ui::Runtime::text_input"),
or presses Backspace when `text` is empty, because empty committed text
inserts nothing and leaves a selection. What is written is what the
editor’s own rules write, as with
[`Self::type_text`](struct.Driver.html#method.type_text "method pert_ui::testing::Driver::type_text").

Requires neutral keyboard input; see [neutral
input](struct.Driver.html#neutral-input-and-cleanup "struct pert_ui::testing::Driver").

##### [§](#errors-9)Errors

Before any input: `ForeignRuntime` or `Unmounted` as
[`TestError::Runtime`](enum.TestError.html#variant.Runtime "variant pert_ui::testing::TestError::Runtime");
[`TestError::OtherWindow`](enum.TestError.html#variant.OtherWindow "variant pert_ui::testing::TestError::OtherWindow")
for an editor outside the selected window;
[`TestError::InputHeld`](enum.TestError.html#variant.InputHeld "variant pert_ui::testing::TestError::InputHeld").
Then the errors of
[`Runtime::focus`](../struct.Runtime.html#method.focus "method pert_ui::Runtime::focus")
and
[`Runtime::text_command`](../struct.Runtime.html#method.text_command "method pert_ui::Runtime::text_command"),
[`TestError::NotAnEditor`](enum.TestError.html#variant.NotAnEditor "variant pert_ui::testing::TestError::NotAnEditor")
for a component that is not a text editor, and the errors of typing, of
the Backspace key with its
[cleanup](struct.Driver.html#neutral-input-and-cleanup "struct pert_ui::testing::Driver")
and of the settle.

``` rust
use pert_ui::prelude::*;
use pert_ui::testing::*;

let mut ui = Runtime::new();
ui.set_viewport(Viewport::new(300.0, 100.0, 1.0)?)?;
let text = ui.signal(String::from("Example"));
let field = ui.mount_tree(Element::new("field", TextField::new(text.clone())))?;
let mut driver = Driver::new(&mut ui);
driver.settle()?;
driver.replace_text(&field, "Replaced")?;
assert_eq!(text.get()?, "Replaced");
driver.replace_text(&field, "")?;
assert_eq!(text.get()?, "");
```

[Source](../../src/pert_ui_runtime/testing.rs.html#1030)

#### pub fn [type_text](#method.type_text)(&mut self, text: &[str](https://doc.rust-lang.org/1.99.1/std/primitive.str.html)) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[()](https://doc.rust-lang.org/1.99.1/std/primitive.unit.html), [TestError](enum.TestError.html "enum pert_ui::testing::TestError")\>

Types `text` at the caret of the focused component, then settles.

The text goes through
[`Runtime::text_input`](../struct.Runtime.html#method.text_input "method pert_ui::Runtime::text_input")
and is written by the editor’s own rules: a plain field writes its
binding, a field with a normalizer or a length limit holds a draft until
Enter or focus loss, and a parameter field holds its draft under the
edit protocol. Empty text inserts nothing. With nothing focused, the
text is ignored.

##### [§](#errors-10)Errors

The errors of
[`Runtime::text_input`](../struct.Runtime.html#method.text_input "method pert_ui::Runtime::text_input")
and of the settle.

[Source](../../src/pert_ui_runtime/testing.rs.html#1072)

#### pub fn [key](#method.key)(&mut self, code: [KeyCode](../enum.KeyCode.html "enum pert_ui::KeyCode")) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[()](https://doc.rust-lang.org/1.99.1/std/primitive.unit.html), [TestError](enum.TestError.html "enum pert_ui::testing::TestError")\>

Presses and releases the named key `code`, without modifiers, then
settles.

Both events are
[synthetic](struct.Driver.html#synthetic-keys "struct pert_ui::testing::Driver"):
`code` is the physical key and
[`CommandKey::Named`](../enum.CommandKey.html#variant.Named "variant pert_ui::CommandKey::Named")
its identity, with no text, so a command bound to the key runs and a
focused editor sees no typed character. Type text with
[`Self::type_text`](struct.Driver.html#method.type_text "method pert_ui::testing::Driver::type_text").

Requires neutral keyboard input; see [neutral
input](struct.Driver.html#neutral-input-and-cleanup "struct pert_ui::testing::Driver").

##### [§](#errors-11)Errors

[`TestError::InputHeld`](enum.TestError.html#variant.InputHeld "variant pert_ui::testing::TestError::InputHeld")
before any input, then the errors of
[`Runtime::command_key_input`](../struct.Runtime.html#method.command_key_input "method pert_ui::Runtime::command_key_input")
with
[cleanup](struct.Driver.html#neutral-input-and-cleanup "struct pert_ui::testing::Driver")
as described, and of the settle.

``` rust
use pert_ui::prelude::*;
use pert_ui::testing::*;

let mut ui = Runtime::new();
ui.set_viewport(Viewport::new(300.0, 100.0, 1.0)?)?;
let tag = ui.signal(String::new());
let field = ui.mount_tree(Element::new("tag",
    TextField::new(tag.clone()).normalize(|text| Ok(text.trim().to_owned()))))?;
let mut driver = Driver::new(&mut ui);
driver.settle()?;
driver.runtime_mut().focus(&field)?;
driver.type_text(" draft ")?;
assert_eq!(tag.get()?, ""); // a draft until Enter
driver.key(KeyCode::Enter)?;
assert_eq!(tag.get()?, "draft");
```

[Source](../../src/pert_ui_runtime/testing.rs.html#1130)

#### pub fn [chord](#method.chord)(&mut self, chord: [KeyChord](../struct.KeyChord.html "struct pert_ui::KeyChord"), code: [KeyCode](../enum.KeyCode.html "enum pert_ui::KeyCode")) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[()](https://doc.rust-lang.org/1.99.1/std/primitive.unit.html), [TestError](enum.TestError.html "enum pert_ui::testing::TestError")\>

Presses `chord` with `code` as its physical key, then settles.

The modifiers `chord` requires are pressed in a fixed synthetic order,
Control, Alt, Shift, then Super: each modifier’s left physical key, then
the modifiers held so far through
[`Runtime::native_modifiers`](../struct.Runtime.html#method.native_modifiers "method pert_ui::Runtime::native_modifiers").
`code` is then pressed and released carrying `chord`’s
[`KeyChord::key`](../struct.KeyChord.html#method.key "method pert_ui::KeyChord::key")
as its identity, and the modifiers are released in reverse order, each
followed by the smaller snapshot. The order is deterministic, not a
platform’s: winit reports modifiers before the key on Windows and after
it on macOS and Wayland, and macOS can report a modifier change with no
key event, so a component that reads modifiers during a modifier’s own
press can observe the difference.

`code` is the physical key because a character chord such as
[`KeyChord::primary`](../struct.KeyChord.html#method.primary "associated function pert_ui::KeyChord::primary")`('s')`
names no physical key; the driver does not guess a keyboard layout. The
events are
[synthetic](struct.Driver.html#synthetic-keys "struct pert_ui::testing::Driver").

Afterwards the runtime holds the empty modifier snapshot the chord
reported last, as after a native host’s report: physical key events do
not change
[`KeyboardState::modifiers`](../struct.KeyboardState.html#method.modifiers "method pert_ui::KeyboardState::modifiers")
until the next report, so a test that later holds a modifier by hand
across other input reports it with
[`Runtime::native_modifiers`](../struct.Runtime.html#method.native_modifiers "method pert_ui::Runtime::native_modifiers")
too.

Requires neutral keyboard input; see [neutral
input](struct.Driver.html#neutral-input-and-cleanup "struct pert_ui::testing::Driver").

##### [§](#errors-12)Errors

[`TestError::InputHeld`](enum.TestError.html#variant.InputHeld "variant pert_ui::testing::TestError::InputHeld")
before any input, then the errors of
[`Runtime::command_key_input`](../struct.Runtime.html#method.command_key_input "method pert_ui::Runtime::command_key_input")
and
[`Runtime::native_modifiers`](../struct.Runtime.html#method.native_modifiers "method pert_ui::Runtime::native_modifiers").
A failed step presses nothing further; every modifier pressed, the key
when it was pressed, and the empty modifier snapshot are still released,
with
[cleanup](struct.Driver.html#neutral-input-and-cleanup "struct pert_ui::testing::Driver")
as described. Then the errors of the settle.

``` rust
use pert_ui::prelude::*;
use pert_ui::testing::Driver;

let mut ui = Runtime::new();
let saves = ui.signal(0);
let counter = saves.clone();
ui.register_command("Save", CommandScope::Application, vec![KeyChord::primary('s')],
    move |_, _| counter.update(|n| *n += 1))?;
let mut driver = Driver::new(&mut ui);
driver.chord(KeyChord::primary('s'), KeyCode::KeyS)?;
assert_eq!(saves.get()?, 1);
assert_eq!(driver.runtime().keyboard().modifiers(), Modifiers::default());
```

[Source](../../src/pert_ui_runtime/testing.rs.html#1232)

#### pub fn [clear_focus](#method.clear_focus)(&mut self) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[()](https://doc.rust-lang.org/1.99.1/std/primitive.unit.html), [TestError](enum.TestError.html "enum pert_ui::testing::TestError")\>

Clears focus in the selected window as its deactivation does, then
settles.

This is
[`HostIntegration::clear_focus`](../host/trait.HostIntegration.html#tymethod.clear_focus "method pert_ui::host::HostIntegration::clear_focus"),
the call a native host makes when a window loses focus, followed by
[`Self::settle`](struct.Driver.html#method.settle "method pert_ui::testing::Driver::settle"),
as every action ends. Afterwards nothing in the selected window is
focused.

- **In the selected window:** nonmodal surfaces such as menus and
  popovers close and modal dialogs stay; pointer buttons, capture, hover
  and the pointer position are cleared; the focused component’s
  `focus_lost` runs, so a focused editor applies its focus-loss rules
  and saves a valid draft, and focus-within notifications are delivered.
  Another window’s focus is untouched: a test of a detached window
  selects it with
  [`HostIntegration::select_window`](../host/trait.HostIntegration.html#tymethod.select_window "method pert_ui::host::HostIntegration::select_window")
  first, as for input.
- **Runtime-wide:** pressed keys, command presses and reported modifiers
  are cleared, a drag in this window is cancelled, and every admitted
  held-input registration, in any window, is cancelled with
  [`HeldCancellation::FocusLost`](../enum.HeldCancellation.html#variant.FocusLost "variant pert_ui::HeldCancellation::FocusLost"),
  because its releases may no longer arrive.

Unlike
[`Self::key`](struct.Driver.html#method.key "method pert_ui::testing::Driver::key")
and
[`Self::click`](struct.Driver.html#method.click "method pert_ui::testing::Driver::click"),
it does not require [neutral
input](struct.Driver.html#neutral-input-and-cleanup "struct pert_ui::testing::Driver"):
releasing held keys and modifiers is what a deactivation does. Input is
neutral afterwards, so the next action is admitted.

##### [§](#errors-13)Errors

[`TestError::ClockOverflow`](enum.TestError.html#variant.ClockOverflow "variant pert_ui::testing::TestError::ClockOverflow")
when the settle’s clock cannot advance. A focus-loss or focus-within
callback’s error, which the deactivation queues, is returned by the
settle’s flush as
[`TestError::Runtime`](enum.TestError.html#variant.Runtime "variant pert_ui::testing::TestError::Runtime");
that flush delivers queued parameter terminal notifications first and
returns the error before it runs commands, tasks, timers or
presentation, so a further
[`Self::settle`](struct.Driver.html#method.settle "method pert_ui::testing::Driver::settle")
delivers the rest, and a later queued error comes from that next flush.
The other errors of
[`Runtime::flush`](../struct.Runtime.html#method.flush "method pert_ui::Runtime::flush")
as
[`TestError::Runtime`](enum.TestError.html#variant.Runtime "variant pert_ui::testing::TestError::Runtime").

``` rust
use pert_ui::prelude::*;
use pert_ui::testing::*;

let mut ui = Runtime::new();
ui.set_viewport(Viewport::new(300.0, 100.0, 1.0)?)?;
let tag = ui.signal(String::new());
let field = ui.mount_tree(Element::new("tag",
    TextField::new(tag.clone()).normalize(|text| Ok(text.trim().to_owned()))))?;
let mut driver = Driver::new(&mut ui);
driver.settle()?;
driver.runtime_mut().focus(&field)?;
driver.type_text(" draft ")?;
assert_eq!(tag.get()?, ""); // a draft until Enter or focus loss
driver.clear_focus()?;
assert_eq!(tag.get()?, "draft");
assert_eq!(driver.runtime().focused(), None);
```

## Trait Implementations[§](#trait-implementations)

[Source](../../src/pert_ui_runtime/testing.rs.html#269)[§](#impl-Debug-for-Driver%3C'_%3E)

### impl [Debug](https://doc.rust-lang.org/1.99.1/core/fmt/trait.Debug.html "trait core::fmt::Debug") for [Driver](struct.Driver.html "struct pert_ui::testing::Driver")\<'\_\>

Shows no runtime state: the runtime has its own inspection.

[Source](../../src/pert_ui_runtime/testing.rs.html#270)[§](#method.fmt)

#### fn [fmt](https://doc.rust-lang.org/1.99.1/core/fmt/trait.Debug.html#tymethod.fmt)(&self, f: &mut [Formatter](https://doc.rust-lang.org/1.99.1/core/fmt/struct.Formatter.html "struct core::fmt::Formatter")\<'\_\>) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[()](https://doc.rust-lang.org/1.99.1/std/primitive.unit.html), [Error](https://doc.rust-lang.org/1.99.1/core/fmt/struct.Error.html "struct core::fmt::Error")\>

Formats the value using the given formatter. [Read
more](https://doc.rust-lang.org/1.99.1/core/fmt/trait.Debug.html#tymethod.fmt)

## Auto Trait Implementations[§](#synthetic-implementations)

[§](#impl-RefUnwindSafe-for-Driver%3C'a%3E)

### impl\<'a\> \![RefUnwindSafe](https://doc.rust-lang.org/1.99.1/core/panic/unwind_safe/trait.RefUnwindSafe.html "trait core::panic::unwind_safe::RefUnwindSafe") for [Driver](struct.Driver.html "struct pert_ui::testing::Driver")\<'a\>

[§](#impl-Send-for-Driver%3C'a%3E)

### impl\<'a\> \![Send](https://doc.rust-lang.org/1.99.1/core/marker/trait.Send.html "trait core::marker::Send") for [Driver](struct.Driver.html "struct pert_ui::testing::Driver")\<'a\>

[§](#impl-Sync-for-Driver%3C'a%3E)

### impl\<'a\> \![Sync](https://doc.rust-lang.org/1.99.1/core/marker/trait.Sync.html "trait core::marker::Sync") for [Driver](struct.Driver.html "struct pert_ui::testing::Driver")\<'a\>

[§](#impl-UnwindSafe-for-Driver%3C'a%3E)

### impl\<'a\> \![UnwindSafe](https://doc.rust-lang.org/1.99.1/core/panic/unwind_safe/trait.UnwindSafe.html "trait core::panic::unwind_safe::UnwindSafe") for [Driver](struct.Driver.html "struct pert_ui::testing::Driver")\<'a\>

[§](#impl-Freeze-for-Driver%3C'a%3E)

### impl\<'a\> [Freeze](https://doc.rust-lang.org/1.99.1/core/marker/trait.Freeze.html "trait core::marker::Freeze") for [Driver](struct.Driver.html "struct pert_ui::testing::Driver")\<'a\>

[§](#impl-Unpin-for-Driver%3C'a%3E)

### impl\<'a\> [Unpin](https://doc.rust-lang.org/1.99.1/core/marker/trait.Unpin.html "trait core::marker::Unpin") for [Driver](struct.Driver.html "struct pert_ui::testing::Driver")\<'a\>

[§](#impl-UnsafeUnpin-for-Driver%3C'a%3E)

### impl\<'a\> [UnsafeUnpin](https://doc.rust-lang.org/1.99.1/core/marker/trait.UnsafeUnpin.html "trait core::marker::UnsafeUnpin") for [Driver](struct.Driver.html "struct pert_ui::testing::Driver")\<'a\>

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

[Source](../../src/zerocopy/pointer/invariant.rs.html#257)[§](#impl-CastableFrom%3CST,+Initialized,+Initialized%3E-for-DT)

### impl\<ST, DT\> [CastableFrom](../../zerocopy/pointer/invariant/trait.CastableFrom.html "trait zerocopy::pointer::invariant::CastableFrom")\<ST, [Initialized](../../zerocopy/pointer/invariant/enum.Initialized.html "enum zerocopy::pointer::invariant::Initialized"), [Initialized](../../zerocopy/pointer/invariant/enum.Initialized.html "enum zerocopy::pointer::invariant::Initialized")\> for DT

where ST:
?[Sized](https://doc.rust-lang.org/1.99.1/core/marker/trait.Sized.html "trait core::marker::Sized"),
DT:
?[Sized](https://doc.rust-lang.org/1.99.1/core/marker/trait.Sized.html "trait core::marker::Sized"),

[Source](../../src/zerocopy/pointer/invariant.rs.html#255)[§](#impl-CastableFrom%3CST,+Uninit,+Uninit%3E-for-DT)

### impl\<ST, DT\> [CastableFrom](../../zerocopy/pointer/invariant/trait.CastableFrom.html "trait zerocopy::pointer::invariant::CastableFrom")\<ST, [Uninit](../../zerocopy/pointer/invariant/enum.Uninit.html "enum zerocopy::pointer::invariant::Uninit"), [Uninit](../../zerocopy/pointer/invariant/enum.Uninit.html "enum zerocopy::pointer::invariant::Uninit")\> for DT

where ST:
?[Sized](https://doc.rust-lang.org/1.99.1/core/marker/trait.Sized.html "trait core::marker::Sized"),
DT:
?[Sized](https://doc.rust-lang.org/1.99.1/core/marker/trait.Sized.html "trait core::marker::Sized"),

[Source](../../src/downcast_rs/lib.rs.html#189)[§](#impl-Downcast-for-T)

### impl\<T\> [Downcast](../../downcast_rs/trait.Downcast.html "trait downcast_rs::Downcast") for T

where T:
[Any](https://doc.rust-lang.org/1.99.1/core/any/trait.Any.html "trait core::any::Any"),

[Source](../../src/downcast_rs/lib.rs.html#190)[§](#method.into_any)

#### fn [into_any](../../downcast_rs/trait.Downcast.html#tymethod.into_any)(self: [Box](https://doc.rust-lang.org/1.99.1/alloc/boxed/struct.Box.html "struct alloc::boxed::Box")\<T\>) -\> [Box](https://doc.rust-lang.org/1.99.1/alloc/boxed/struct.Box.html "struct alloc::boxed::Box")\<dyn [Any](https://doc.rust-lang.org/1.99.1/core/any/trait.Any.html "trait core::any::Any")\>

Convert `Box<dyn Trait>` (where `Trait: Downcast`) to `Box<dyn Any>`.
`Box<dyn Any>` can then be further `downcast` into `Box<ConcreteType>`
where `ConcreteType` implements `Trait`.

[Source](../../src/downcast_rs/lib.rs.html#191)[§](#method.into_any_rc)

#### fn [into_any_rc](../../downcast_rs/trait.Downcast.html#tymethod.into_any_rc)(self: [Rc](https://doc.rust-lang.org/1.99.1/alloc/rc/struct.Rc.html "struct alloc::rc::Rc")\<T\>) -\> [Rc](https://doc.rust-lang.org/1.99.1/alloc/rc/struct.Rc.html "struct alloc::rc::Rc")\<dyn [Any](https://doc.rust-lang.org/1.99.1/core/any/trait.Any.html "trait core::any::Any")\>

Convert `Rc<Trait>` (where `Trait: Downcast`) to `Rc<Any>`. `Rc<Any>`
can then be further `downcast` into `Rc<ConcreteType>` where
`ConcreteType` implements `Trait`.

[Source](../../src/downcast_rs/lib.rs.html#192)[§](#method.as_any)

#### fn [as_any](../../downcast_rs/trait.Downcast.html#tymethod.as_any)(&self) -\> &(dyn [Any](https://doc.rust-lang.org/1.99.1/core/any/trait.Any.html "trait core::any::Any") + 'static)

Convert `&Trait` (where `Trait: Downcast`) to `&Any`. This is needed
since Rust cannot generate `&Any`’s vtable from `&Trait`’s.

[Source](../../src/downcast_rs/lib.rs.html#193)[§](#method.as_any_mut)

#### fn [as_any_mut](../../downcast_rs/trait.Downcast.html#tymethod.as_any_mut)(&mut self) -\> &mut (dyn [Any](https://doc.rust-lang.org/1.99.1/core/any/trait.Any.html "trait core::any::Any") + 'static)

Convert `&mut Trait` (where `Trait: Downcast`) to `&Any`. This is needed
since Rust cannot generate `&mut Any`’s vtable from `&mut Trait`’s.

[Source](../../src/yoke/erased.rs.html#22)[§](#impl-ErasedDestructor-for-T)

### impl\<T\> [ErasedDestructor](../../yoke/erased/trait.ErasedDestructor.html "trait yoke::erased::ErasedDestructor") for T

where T: 'static,

[Source](https://doc.rust-lang.org/1.99.1/src/core/convert/mod.rs.html#788)[§](#impl-From%3CT%3E-for-T)

### impl\<T\> [From](https://doc.rust-lang.org/1.99.1/core/convert/trait.From.html "trait core::convert::From")\<T\> for T

[Source](https://doc.rust-lang.org/1.99.1/src/core/convert/mod.rs.html#791)[§](#method.from)

#### fn [from](https://doc.rust-lang.org/1.99.1/core/convert/trait.From.html#tymethod.from)(t: T) -\> T

Returns the argument unchanged.

[Source](../../src/tracing/instrument.rs.html#325)[§](#impl-Instrument-for-T)

### impl\<T\> [Instrument](../../tracing/instrument/trait.Instrument.html "trait tracing::instrument::Instrument") for T

[Source](../../src/tracing/instrument.rs.html#86)[§](#method.instrument)

#### fn [instrument](../../tracing/instrument/trait.Instrument.html#method.instrument)(self, span: [Span](../../tracing/span/struct.Span.html "struct tracing::span::Span")) -\> [Instrumented](../../tracing/instrument/struct.Instrumented.html "struct tracing::instrument::Instrumented")\<Self\> [ⓘ](#)

Instruments this type with the provided
[`Span`](../../tracing/span/struct.Span.html "struct tracing::span::Span"),
returning an `Instrumented` wrapper. [Read
more](../../tracing/instrument/trait.Instrument.html#method.instrument)

[Source](../../src/tracing/instrument.rs.html#128)[§](#method.in_current_span)

#### fn [in_current_span](../../tracing/instrument/trait.Instrument.html#method.in_current_span)(self) -\> [Instrumented](../../tracing/instrument/struct.Instrumented.html "struct tracing::instrument::Instrumented")\<Self\> [ⓘ](#)

Instruments this type with the
[current](../../tracing/span/struct.Span.html#method.current "associated function tracing::span::Span::current")
[`Span`](../../tracing/span/struct.Span.html "struct tracing::span::Span"),
returning an `Instrumented` wrapper. [Read
more](../../tracing/instrument/trait.Instrument.html#method.in_current_span)

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

[Source](../../src/zerocopy/pointer/invariant.rs.html#276)[§](#impl-Read%3CExclusive,+BecauseExclusive%3E-for-T)

### impl\<T\> [Read](../../zerocopy/pointer/invariant/trait.Read.html "trait zerocopy::pointer::invariant::Read")\<[Exclusive](../../zerocopy/pointer/invariant/enum.Exclusive.html "enum zerocopy::pointer::invariant::Exclusive"), [BecauseExclusive](../../zerocopy/pointer/invariant/enum.BecauseExclusive.html "enum zerocopy::pointer::invariant::BecauseExclusive")\> for T

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

[Source](../../src/tracing/instrument.rs.html#393)[§](#impl-WithSubscriber-for-T)

### impl\<T\> [WithSubscriber](../../tracing/instrument/trait.WithSubscriber.html "trait tracing::instrument::WithSubscriber") for T

[Source](../../src/tracing/instrument.rs.html#176-178)[§](#method.with_subscriber)

#### fn [with_subscriber](../../tracing/instrument/trait.WithSubscriber.html#method.with_subscriber)\<S\>(self, subscriber: S) -\> [WithDispatch](../../tracing/instrument/struct.WithDispatch.html "struct tracing::instrument::WithDispatch")\<Self\>

where S:
[Into](https://doc.rust-lang.org/1.99.1/core/convert/trait.Into.html "trait core::convert::Into")\<[Dispatch](../../tracing_core/dispatcher/struct.Dispatch.html "struct tracing_core::dispatcher::Dispatch")\>,

Attaches the provided
[`Subscriber`](../../tracing_core/subscriber/trait.Subscriber.html "trait tracing_core::subscriber::Subscriber")
to this type, returning a
[`WithDispatch`](../../tracing/instrument/struct.WithDispatch.html "struct tracing::instrument::WithDispatch")
wrapper. [Read
more](../../tracing/instrument/trait.WithSubscriber.html#method.with_subscriber)

[Source](../../src/tracing/instrument.rs.html#228)[§](#method.with_current_subscriber)

#### fn [with_current_subscriber](../../tracing/instrument/trait.WithSubscriber.html#method.with_current_subscriber)(self) -\> [WithDispatch](../../tracing/instrument/struct.WithDispatch.html "struct tracing::instrument::WithDispatch")\<Self\>

Attaches the current
[default](../../tracing/dispatcher/index.html#setting-the-default-subscriber "mod tracing::dispatcher")
[`Subscriber`](../../tracing_core/subscriber/trait.Subscriber.html "trait tracing_core::subscriber::Subscriber")
to this type, returning a
[`WithDispatch`](../../tracing/instrument/struct.WithDispatch.html "struct tracing::instrument::WithDispatch")
wrapper. [Read
more](../../tracing/instrument/trait.WithSubscriber.html#method.with_current_subscriber)
