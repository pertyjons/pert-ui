[pert_ui](index.html)

# Struct Tab Copy item path

[Source](../src/pert_ui_runtime/tabs.rs.html#12)

``` rust
pub struct Tab { /* private fields */ }
```

Expand description

Owned tab description with a stable key, display title and content.
Titles are independent of identity; inactive content stays mounted but
hidden.

## Implementations[§](#implementations)

[Source](../src/pert_ui_runtime/tabs.rs.html#20)[§](#impl-Tab)

### impl [Tab](struct.Tab.html "struct pert_ui::Tab")

[Source](../src/pert_ui_runtime/tabs.rs.html#35)

#### pub fn [new](#method.new)( key: impl [Into](https://doc.rust-lang.org/1.99.1/core/convert/trait.Into.html "trait core::convert::Into")\<[String](https://doc.rust-lang.org/1.99.1/alloc/string/struct.String.html "struct alloc::string::String")\>, title: impl [Into](https://doc.rust-lang.org/1.99.1/core/convert/trait.Into.html "trait core::convert::Into")\<[TextSource](struct.TextSource.html "struct pert_ui::TextSource")\>, content: [Element](struct.Element.html "struct pert_ui::Element"), ) -\> [Tab](struct.Tab.html "struct pert_ui::Tab")

Creates a tab with automatic two-axis content scrolling and no close
button. The key must not be blank and must be unique in its strip, and
the title must not be blank;
[`Tabs`](struct.Tabs.html "struct pert_ui::Tabs") checks both when it is
described
([`LayoutError::BlankKey`](enum.LayoutError.html#variant.BlankKey "variant pert_ui::LayoutError::BlankKey"),
[`LayoutError::DuplicateKey`](enum.LayoutError.html#variant.DuplicateKey "variant pert_ui::LayoutError::DuplicateKey"),
[`LayoutError::BlankLabel`](enum.LayoutError.html#variant.BlankLabel "variant pert_ui::LayoutError::BlankLabel")).
Title size follows text admission.

The title is a
[`TextSource`](struct.TextSource.html "struct pert_ui::TextSource"):
fixed plain or rich text, or a binding the tab follows without being
described again, for example a translated title or one that marks
unsaved changes. A bound title is checked when it is read: a blank read
keeps the last title and fails the strip’s update with
[`LayoutError::BlankLabel`](enum.LayoutError.html#variant.BlankLabel "variant pert_ui::LayoutError::BlankLabel").
A tab detached into its own window opens it with the current title, and
the strip asks for the window to be retitled when the title changes
([`EventContext::set_window_title`](struct.EventContext.html#method.set_window_title "method pert_ui::EventContext::set_window_title")).

[Source](../src/pert_ui_runtime/tabs.rs.html#57)

#### pub fn [fixed](#method.fixed)(self) -\> [Tab](struct.Tab.html "struct pert_ui::Tab")

Fits content to the tab allocation and clips overflow instead of
scrolling it. Use for layouts containing their own scroll areas or
splits. Nested scroll areas still scroll and reveal focus; oversized
content outside them is clipped. The default from
[`Self::new`](struct.Tab.html#method.new "associated function pert_ui::Tab::new")
remains automatic two-axis scrolling.

``` rust
use pert_ui::{Element, Label, ScrollArea, Tab, Tabs};
let tabs = Tabs::new([Tab::new(
    "items", "Items",
    Element::new("items-scroll", ScrollArea::new(
        Element::new("content", Label::new("Scrollable content")),
    )),
).fixed()]);
```

[Source](../src/pert_ui_runtime/tabs.rs.html#83)

#### pub fn [detachable](#method.detachable)(self, permission: impl [Into](https://doc.rust-lang.org/1.99.1/core/convert/trait.Into.html "trait core::convert::Into")\<[Permission](struct.Permission.html "struct pert_ui::Permission")\>) -\> [Tab](struct.Tab.html "struct pert_ui::Tab")

Restricts moving this one tab into its own native window with a fixed or
reactive
[`Permission`](struct.Permission.html "struct pert_ui::Permission").

Defaults to permitted, which only matters in a strip that opted into
[`Tabs::detachable`](struct.Tabs.html#method.detachable "method pert_ui::Tabs::detachable").
A forbidden tab keeps the ordinary card, cursor and selection behavior
and offers no drag-out; the rest of the strip is unaffected. The strip
reads a bound permission in its update, so the affordance follows it
without polling. Repeated calls combine restrictions. This is
`Element::with_detachable` on the tab’s view, so the same refusal also
applies to `Runtime::request_detach`, which reads the current value when
asked.

``` rust
use pert_ui::{Element, Label, Tab, Tabs};
let tabs = Tabs::new([
    Tab::new("editor", "Editor", Element::new("a", Label::new("Editable"))),
    Tab::new("log", "Log", Element::new("b", Label::new("Stays docked")))
        .detachable(false),
])
.detachable(true);
```

[Source](../src/pert_ui_runtime/tabs.rs.html#98)

#### pub fn [rich_title](#method.rich_title)(self, title: [RichText](struct.RichText.html "struct pert_ui::RichText")) -\> [Tab](struct.Tab.html "struct pert_ui::Tab")

Replaces the title with styled text and font icons, retaining the tab
key and the
[`Self::leading_icon`](struct.Tab.html#method.leading_icon "method pert_ui::Tab::leading_icon")
slot. Icon-only content keeps the existing title as its human-readable
name.

``` rust
use pert_ui::{Element, Icon, Label, RichText, Tab};
let tab = Tab::new("settings", "Settings", Element::new("body", Label::new("Options")))
    .rich_title(RichText::new().icon(Icon::Settings)).into_element();
assert_eq!(tab.label(), Some("Settings"));
```

[Source](../src/pert_ui_runtime/tabs.rs.html#118)

#### pub fn [leading_icon](#method.leading_icon)(self, icon: impl [Into](https://doc.rust-lang.org/1.99.1/core/convert/trait.Into.html "trait core::convert::Into")\<[IconSource](enum.IconSource.html "enum pert_ui::IconSource")\>) -\> [Tab](struct.Tab.html "struct pert_ui::Tab")

Replaces the leading icon slot: one icon shown before the title with the
theme’s icon gap, independent of the title itself. A second call
replaces the icon rather than adding one, and
[`Self::rich_title`](struct.Tab.html#method.rich_title "method pert_ui::Tab::rich_title")
replaces the title while keeping the slot. Compose several icons in a
rich title when more than one is wanted.

``` rust
use pert_ui::{Element, Icon, Label, RichText, Tab};
let tab = Tab::new("mix", "Mix", Element::new("body", Label::new("Mixer")))
    .leading_icon(Icon::Settings)
    .leading_icon(Icon::Folder)
    .into_element();
assert_eq!(tab.label(), Some("Mix"));
assert_eq!(tab.rich_label(), Some(&RichText::from("Mix").leading_icon(Icon::Folder)));
```

[Source](../src/pert_ui_runtime/tabs.rs.html#124)

#### pub fn [into_element](#method.into_element)(self) -\> [Element](struct.Element.html "struct pert_ui::Element")

Returns the ordinary labeled view description, including for
reconciliation. Reusing its key retains state; changing its title
updates metadata independently.

## Trait Implementations[§](#trait-implementations)

[Source](../src/pert_ui_runtime/tabs.rs.html#19)[§](#impl-Debug-for-Tab)

### impl [Debug](https://doc.rust-lang.org/1.99.1/core/fmt/trait.Debug.html "trait core::fmt::Debug") for [Tab](struct.Tab.html "struct pert_ui::Tab")

Shows the item’s key. Its content, bindings and callbacks are not
formatted.

[Source](../src/pert_ui_runtime/tabs.rs.html#19)[§](#method.fmt)

#### fn [fmt](https://doc.rust-lang.org/1.99.1/core/fmt/trait.Debug.html#tymethod.fmt)(&self, f: &mut [Formatter](https://doc.rust-lang.org/1.99.1/core/fmt/struct.Formatter.html "struct core::fmt::Formatter")\<'\_\>) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[()](https://doc.rust-lang.org/1.99.1/std/primitive.unit.html), [Error](https://doc.rust-lang.org/1.99.1/core/fmt/struct.Error.html "struct core::fmt::Error")\>

Formats the value using the given formatter. [Read
more](https://doc.rust-lang.org/1.99.1/core/fmt/trait.Debug.html#tymethod.fmt)

[Source](../src/pert_ui_runtime/tabs.rs.html#151)[§](#impl-From%3CTab%3E-for-Element)

### impl [From](https://doc.rust-lang.org/1.99.1/core/convert/trait.From.html "trait core::convert::From")\<[Tab](struct.Tab.html "struct pert_ui::Tab")\> for [Element](struct.Element.html "struct pert_ui::Element")

[Source](../src/pert_ui_runtime/tabs.rs.html#156)[§](#method.from)

#### fn [from](https://doc.rust-lang.org/1.99.1/core/convert/trait.From.html#tymethod.from)(tab: [Tab](struct.Tab.html "struct pert_ui::Tab")) -\> [Element](struct.Element.html "struct pert_ui::Element")

Converts as
[`Tab::into_element`](struct.Tab.html#method.into_element "method pert_ui::Tab::into_element")
does, so a list of tabs can be collected into the `Vec<Element>` that
[`Runtime::reconcile_children`](struct.Runtime.html#method.reconcile_children "method pert_ui::Runtime::reconcile_children")
takes with `.map(Element::from)` or `.map(Into::into)`.

## Auto Trait Implementations[§](#synthetic-implementations)

[§](#impl-RefUnwindSafe-for-Tab)

### impl \![RefUnwindSafe](https://doc.rust-lang.org/1.99.1/core/panic/unwind_safe/trait.RefUnwindSafe.html "trait core::panic::unwind_safe::RefUnwindSafe") for [Tab](struct.Tab.html "struct pert_ui::Tab")

[§](#impl-Send-for-Tab)

### impl \![Send](https://doc.rust-lang.org/1.99.1/core/marker/trait.Send.html "trait core::marker::Send") for [Tab](struct.Tab.html "struct pert_ui::Tab")

[§](#impl-Sync-for-Tab)

### impl \![Sync](https://doc.rust-lang.org/1.99.1/core/marker/trait.Sync.html "trait core::marker::Sync") for [Tab](struct.Tab.html "struct pert_ui::Tab")

[§](#impl-UnwindSafe-for-Tab)

### impl \![UnwindSafe](https://doc.rust-lang.org/1.99.1/core/panic/unwind_safe/trait.UnwindSafe.html "trait core::panic::unwind_safe::UnwindSafe") for [Tab](struct.Tab.html "struct pert_ui::Tab")

[§](#impl-Freeze-for-Tab)

### impl [Freeze](https://doc.rust-lang.org/1.99.1/core/marker/trait.Freeze.html "trait core::marker::Freeze") for [Tab](struct.Tab.html "struct pert_ui::Tab")

[§](#impl-Unpin-for-Tab)

### impl [Unpin](https://doc.rust-lang.org/1.99.1/core/marker/trait.Unpin.html "trait core::marker::Unpin") for [Tab](struct.Tab.html "struct pert_ui::Tab")

[§](#impl-UnsafeUnpin-for-Tab)

### impl [UnsafeUnpin](https://doc.rust-lang.org/1.99.1/core/marker/trait.UnsafeUnpin.html "trait core::marker::UnsafeUnpin") for [Tab](struct.Tab.html "struct pert_ui::Tab")

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

[Source](https://doc.rust-lang.org/1.99.1/src/core/convert/mod.rs.html#791)[§](#method.from-1)

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
