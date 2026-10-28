[pert_ui](index.html)

# Struct DialogController Copy item path

[Source](../src/pert_ui_runtime/dialog.rs.html#47)

``` rust
pub struct DialogController { /* private fields */ }
```

Expand description

Cloneable UI-thread controller for one retained dialog owner.

Opening reserves the outcome slot. A second open, including after
completion before taking the outcome, returns `DialogBusy`. Clones
address the same session; no executor, queue, callback or application
I/O is implied. Removal of the mounted owner records a dismissal even if
the request has not reached layout yet. Reads/writes follow signal phase
and runtime-lifetime restrictions.

One mounted dialog owner uses a controller at a time. The owner claims
it when it mounts, not when it is constructed, so a tree containing the
dialog can be described again with the same controller: the new
description is applied to the mounted owner. A second owner mounted with
the same controller while the first is still mounted stays closed and
fails its updates with
[`RuntimeError::DialogBusy`](enum.RuntimeError.html#variant.DialogBusy "variant pert_ui::RuntimeError::DialogBusy")
until the first unmounts.

## Implementations[§](#implementations)

[Source](../src/pert_ui_runtime/dialog.rs.html#63)[§](#impl-DialogController)

### impl [DialogController](struct.DialogController.html "struct pert_ui::DialogController")

[Source](../src/pert_ui_runtime/dialog.rs.html#71)

#### pub fn [new](#method.new)(ui: &impl [StateOwner](trait.StateOwner.html "trait pert_ui::StateOwner")) -\> [DialogController](struct.DialogController.html "struct pert_ui::DialogController")

Creates an idle controller belonging to `ui`.

`ui` is any
[`StateOwner`](trait.StateOwner.html "trait pert_ui::StateOwner"): the
[`Runtime`](struct.Runtime.html "struct pert_ui::Runtime") while the
application builds its tree, or the
[`UpdateContext`](struct.UpdateContext.html "struct pert_ui::UpdateContext")
or
[`EventContext`](struct.EventContext.html "struct pert_ui::EventContext")
of a component that builds this while it describes its children.

[Source](../src/pert_ui_runtime/dialog.rs.html#86)

#### pub fn [status](#method.status)(&self) -\> [ReadSignal](struct.ReadSignal.html "struct pert_ui::ReadSignal")\<[DialogStatus](enum.DialogStatus.html "enum pert_ui::DialogStatus")\>

Subscribable session state, including the retained terminal outcome.

[Source](../src/pert_ui_runtime/dialog.rs.html#100)

#### pub fn [open](#method.open)(&self) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[()](https://doc.rust-lang.org/1.99.1/std/primitive.unit.html), [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\>

Requests opening and reserves terminal storage. It is refused while
open, awaiting acknowledgement, or before the owner has observed closure
in a flush. This prevents finish/take/open within one event from
retaining the previous session’s affirmative focus.

##### [§](#errors)Errors

Returns
[`RuntimeError::DialogBusy`](enum.RuntimeError.html#variant.DialogBusy "variant pert_ui::RuntimeError::DialogBusy")
while a session is open, its outcome is untaken, or the owner has not
yet observed the closure in a flush; otherwise the errors of
[`Signal::get`](struct.Signal.html#method.get "method pert_ui::Signal::get")
and
[`Signal::set`](struct.Signal.html#method.set "method pert_ui::Signal::set")
on the status signal, such as `Closed` after shutdown or a phase write
error.

[Source](../src/pert_ui_runtime/dialog.rs.html#113)

#### pub fn [accept](#method.accept)(&self) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[bool](https://doc.rust-lang.org/1.99.1/std/primitive.bool.html), [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\>

Completes an open session with acceptance. Returns false if already
closed.

##### [§](#errors-1)Errors

Returns the errors of
[`Signal::get`](struct.Signal.html#method.get "method pert_ui::Signal::get")
and
[`Signal::set`](struct.Signal.html#method.set "method pert_ui::Signal::set")
on the status signal, such as `Closed` after shutdown or a phase write
error.

[Source](../src/pert_ui_runtime/dialog.rs.html#122)

#### pub fn [cancel](#method.cancel)(&self) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[bool](https://doc.rust-lang.org/1.99.1/std/primitive.bool.html), [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\>

Completes an open session with cancellation. Returns false if already
closed.

##### [§](#errors-2)Errors

Returns the errors of
[`Signal::get`](struct.Signal.html#method.get "method pert_ui::Signal::get")
and
[`Signal::set`](struct.Signal.html#method.set "method pert_ui::Signal::set")
on the status signal, such as `Closed` after shutdown or a phase write
error.

[Source](../src/pert_ui_runtime/dialog.rs.html#140)

#### pub fn [take_outcome](#method.take_outcome)(&self) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[Option](https://doc.rust-lang.org/1.99.1/core/option/enum.Option.html "enum core::option::Option")\<[DialogOutcome](enum.DialogOutcome.html "enum pert_ui::DialogOutcome")\>, [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\>

Takes the completed outcome and releases its slot. Returns None for an
idle/open session. Taking an outcome never opens another session
implicitly.

##### [§](#errors-3)Errors

Returns the errors of
[`Signal::get`](struct.Signal.html#method.get "method pert_ui::Signal::get")
and
[`Signal::set`](struct.Signal.html#method.set "method pert_ui::Signal::set")
on the status signal, such as `Closed` after shutdown or a phase write
error.

## Trait Implementations[§](#trait-implementations)

[Source](../src/pert_ui_runtime/dialog.rs.html#45)[§](#impl-Clone-for-DialogController)

### impl [Clone](https://doc.rust-lang.org/1.99.1/core/clone/trait.Clone.html "trait core::clone::Clone") for [DialogController](struct.DialogController.html "struct pert_ui::DialogController")

[Source](../src/pert_ui_runtime/dialog.rs.html#45)[§](#method.clone)

#### fn [clone](https://doc.rust-lang.org/1.99.1/core/clone/trait.Clone.html#tymethod.clone)(&self) -\> [DialogController](struct.DialogController.html "struct pert_ui::DialogController")

Returns a duplicate of the value. [Read
more](https://doc.rust-lang.org/1.99.1/core/clone/trait.Clone.html#tymethod.clone)

1.0.0 (const:
[unstable](https://github.com/rust-lang/rust/issues/142757 "Tracking issue for const_clone"))
·
[Source](https://doc.rust-lang.org/1.99.1/src/core/clone.rs.html#245-247)[§](#method.clone_from)

#### fn [clone_from](https://doc.rust-lang.org/1.99.1/core/clone/trait.Clone.html#method.clone_from)(&mut self, source: &Self)

Performs copy-assignment from `source`. [Read
more](https://doc.rust-lang.org/1.99.1/core/clone/trait.Clone.html#method.clone_from)

[Source](../src/pert_ui_runtime/dialog.rs.html#55)[§](#impl-Debug-for-DialogController)

### impl [Debug](https://doc.rust-lang.org/1.99.1/core/fmt/trait.Debug.html "trait core::fmt::Debug") for [DialogController](struct.DialogController.html "struct pert_ui::DialogController")

Shows the session status, or the error reading it after runtime
shutdown, and whether a mounted dialog owner uses this controller.

[Source](../src/pert_ui_runtime/dialog.rs.html#56)[§](#method.fmt)

#### fn [fmt](https://doc.rust-lang.org/1.99.1/core/fmt/trait.Debug.html#tymethod.fmt)(&self, f: &mut [Formatter](https://doc.rust-lang.org/1.99.1/core/fmt/struct.Formatter.html "struct core::fmt::Formatter")\<'\_\>) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[()](https://doc.rust-lang.org/1.99.1/std/primitive.unit.html), [Error](https://doc.rust-lang.org/1.99.1/core/fmt/struct.Error.html "struct core::fmt::Error")\>

Formats the value using the given formatter. [Read
more](https://doc.rust-lang.org/1.99.1/core/fmt/trait.Debug.html#tymethod.fmt)

## Auto Trait Implementations[§](#synthetic-implementations)

[§](#impl-RefUnwindSafe-for-DialogController)

### impl \![RefUnwindSafe](https://doc.rust-lang.org/1.99.1/core/panic/unwind_safe/trait.RefUnwindSafe.html "trait core::panic::unwind_safe::RefUnwindSafe") for [DialogController](struct.DialogController.html "struct pert_ui::DialogController")

[§](#impl-Send-for-DialogController)

### impl \![Send](https://doc.rust-lang.org/1.99.1/core/marker/trait.Send.html "trait core::marker::Send") for [DialogController](struct.DialogController.html "struct pert_ui::DialogController")

[§](#impl-Sync-for-DialogController)

### impl \![Sync](https://doc.rust-lang.org/1.99.1/core/marker/trait.Sync.html "trait core::marker::Sync") for [DialogController](struct.DialogController.html "struct pert_ui::DialogController")

[§](#impl-UnwindSafe-for-DialogController)

### impl \![UnwindSafe](https://doc.rust-lang.org/1.99.1/core/panic/unwind_safe/trait.UnwindSafe.html "trait core::panic::unwind_safe::UnwindSafe") for [DialogController](struct.DialogController.html "struct pert_ui::DialogController")

[§](#impl-Freeze-for-DialogController)

### impl [Freeze](https://doc.rust-lang.org/1.99.1/core/marker/trait.Freeze.html "trait core::marker::Freeze") for [DialogController](struct.DialogController.html "struct pert_ui::DialogController")

[§](#impl-Unpin-for-DialogController)

### impl [Unpin](https://doc.rust-lang.org/1.99.1/core/marker/trait.Unpin.html "trait core::marker::Unpin") for [DialogController](struct.DialogController.html "struct pert_ui::DialogController")

[§](#impl-UnsafeUnpin-for-DialogController)

### impl [UnsafeUnpin](https://doc.rust-lang.org/1.99.1/core/marker/trait.UnsafeUnpin.html "trait core::marker::UnsafeUnpin") for [DialogController](struct.DialogController.html "struct pert_ui::DialogController")

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

[Source](https://doc.rust-lang.org/1.99.1/src/core/clone.rs.html#648)[§](#impl-CloneToUninit-for-T)

### impl\<T\> [CloneToUninit](https://doc.rust-lang.org/1.99.1/core/clone/trait.CloneToUninit.html "trait core::clone::CloneToUninit") for T

where T:
[Clone](https://doc.rust-lang.org/1.99.1/core/clone/trait.Clone.html "trait core::clone::Clone"),

[Source](https://doc.rust-lang.org/1.99.1/src/core/clone.rs.html#650)[§](#method.clone_to_uninit)

#### unsafe fn [clone_to_uninit](https://doc.rust-lang.org/1.99.1/core/clone/trait.CloneToUninit.html#tymethod.clone_to_uninit)(&self, dest: [\*mut](https://doc.rust-lang.org/1.99.1/std/primitive.pointer.html) [u8](https://doc.rust-lang.org/1.99.1/std/primitive.u8.html))

🔬This is a nightly-only experimental API. (`clone_to_uninit`)

Performs copy-assignment from `self` to `dest`. [Read
more](https://doc.rust-lang.org/1.99.1/core/clone/trait.CloneToUninit.html#tymethod.clone_to_uninit)

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

[Source](https://doc.rust-lang.org/1.99.1/src/alloc/borrow.rs.html#72-74)[§](#impl-ToOwned-for-T)

### impl\<T\> [ToOwned](https://doc.rust-lang.org/1.99.1/alloc/borrow/trait.ToOwned.html "trait alloc::borrow::ToOwned") for T

where T:
[Clone](https://doc.rust-lang.org/1.99.1/core/clone/trait.Clone.html "trait core::clone::Clone"),

[Source](https://doc.rust-lang.org/1.99.1/src/alloc/borrow.rs.html#76)[§](#associatedtype.Owned)

#### type [Owned](https://doc.rust-lang.org/1.99.1/alloc/borrow/trait.ToOwned.html#associatedtype.Owned) = T

The resulting type after obtaining ownership.

[Source](https://doc.rust-lang.org/1.99.1/src/alloc/borrow.rs.html#77)[§](#method.to_owned)

#### fn [to_owned](https://doc.rust-lang.org/1.99.1/alloc/borrow/trait.ToOwned.html#tymethod.to_owned)(&self) -\> T

Creates owned data from borrowed data, usually by cloning. [Read
more](https://doc.rust-lang.org/1.99.1/alloc/borrow/trait.ToOwned.html#tymethod.to_owned)

[Source](https://doc.rust-lang.org/1.99.1/src/alloc/borrow.rs.html#81)[§](#method.clone_into)

#### fn [clone_into](https://doc.rust-lang.org/1.99.1/alloc/borrow/trait.ToOwned.html#method.clone_into)(&self, target: [&mut T](https://doc.rust-lang.org/1.99.1/std/primitive.reference.html))

Uses borrowed data to replace owned data, usually by cloning. [Read
more](https://doc.rust-lang.org/1.99.1/alloc/borrow/trait.ToOwned.html#method.clone_into)

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
