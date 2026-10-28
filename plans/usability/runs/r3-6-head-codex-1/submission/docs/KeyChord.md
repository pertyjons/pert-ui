[pert_ui](index.html)

# Struct KeyChord Copy item path

[Source](../src/pert_ui_runtime/runtime/commands.rs.html#27)

``` rust
pub struct KeyChord { /* private fields */ }
```

Expand description

Key and exact modifier requirements. Primary means Command on macOS and
Control elsewhere. Physical fallback is explicit and only used when no
logical identity is available; it never runs alongside a logical match.

## Implementations[§](#implementations)

[Source](../src/pert_ui_runtime/runtime/commands.rs.html#32)[§](#impl-KeyChord)

### impl [KeyChord](struct.KeyChord.html "struct pert_ui::KeyChord")

[Source](../src/pert_ui_runtime/runtime/commands.rs.html#67)

#### pub fn [new](#method.new)(key: [CommandKey](enum.CommandKey.html "enum pert_ui::CommandKey"), modifiers: [Modifiers](struct.Modifiers.html "struct pert_ui::Modifiers")) -\> [KeyChord](struct.KeyChord.html "struct pert_ui::KeyChord")

Creates an exact chord using modifier-free layout identity.

The chord matches only `modifiers`: set the
[`Modifiers`](struct.Modifiers.html "struct pert_ui::Modifiers") fields
it needs and leave the rest at their defaults. `control` and `alt` are
those keys on every platform;
[`Self::primary`](struct.KeyChord.html#method.primary "associated function pert_ui::KeyChord::primary")
instead takes the platform’s primary modifier. ASCII letters match in
either case; other characters compare exactly.

``` rust
use pert_ui::prelude::*;
use pert_ui::testing::Driver;

// Control+Alt+K on every platform, and Alt+Delete.
let control_alt = Modifiers { control: true, alt: true, ..Modifiers::default() };
let alt = Modifiers { alt: true, ..Modifiers::default() };
let insert = KeyChord::new(CommandKey::Character('K'), control_alt);
let remove = KeyChord::new(CommandKey::Named(KeyCode::Delete), alt);

let mut ui = Runtime::new();
let runs = ui.signal(0);
let counter = runs.clone();
ui.register_command("Insert link", CommandScope::Application, vec![insert, remove],
    move |_, _| counter.update(|n| *n += 1))?;
let mut driver = Driver::new(&mut ui);
driver.chord(KeyChord::new(CommandKey::Character('k'), control_alt), KeyCode::KeyK)?;
driver.chord(remove, KeyCode::Delete)?;
assert_eq!(runs.get()?, 2);
// Exact: an extra Shift is another chord.
let shifted = Modifiers { shift: true, ..control_alt };
driver.chord(KeyChord::new(CommandKey::Character('k'), shifted), KeyCode::KeyK)?;
assert_eq!(runs.get()?, 2);
```

[Source](../src/pert_ui_runtime/runtime/commands.rs.html#86)

#### pub fn [named](#method.named)(key: [KeyCode](enum.KeyCode.html "enum pert_ui::KeyCode")) -\> [KeyChord](struct.KeyChord.html "struct pert_ui::KeyChord")

One named key without modifiers, such as Space for a transport or Delete
for a selection:
`KeyChord::new(CommandKey::Named(key), Modifiers::default())`.

``` rust
use pert_ui::{CommandKey, KeyChord, KeyCode, Modifiers};
assert_eq!(
    KeyChord::named(KeyCode::Space),
    KeyChord::new(CommandKey::Named(KeyCode::Space), Modifiers::default()),
);
```

[Source](../src/pert_ui_runtime/runtime/commands.rs.html#90)

#### pub fn [primary](#method.primary)(letter: [char](https://doc.rust-lang.org/1.99.1/std/primitive.char.html)) -\> [KeyChord](struct.KeyChord.html "struct pert_ui::KeyChord")

A conventional primary-modifier letter chord.

[Source](../src/pert_ui_runtime/runtime/commands.rs.html#94)

#### pub fn [shifted](#method.shifted)(self) -\> [KeyChord](struct.KeyChord.html "struct pert_ui::KeyChord")

Adds Shift to the exact modifiers; useful for redo and command aliases.

[Source](../src/pert_ui_runtime/runtime/commands.rs.html#114)

#### pub fn [navigate_back](#method.navigate_back)() -\> [KeyChord](struct.KeyChord.html "struct pert_ui::KeyChord")

The platform’s conventional chord for going back in a navigation
history: Cmd+\[ on macOS and Alt+Left elsewhere, as browsers use.
Register it on the application’s own back command; see
[`HistoryNavigator`](struct.HistoryNavigator.html "struct pert_ui::HistoryNavigator").

``` rust
use pert_ui::{CommandKey, KeyChord, KeyCode};
let back = KeyChord::navigate_back();
if cfg!(target_os = "macos") {
    assert_eq!(back.key(), CommandKey::Character('['));
} else {
    assert_eq!(back.key(), CommandKey::Named(KeyCode::ArrowLeft));
    assert!(back.modifiers().alt);
}
```

[Source](../src/pert_ui_runtime/runtime/commands.rs.html#120)

#### pub fn [navigate_forward](#method.navigate_forward)() -\> [KeyChord](struct.KeyChord.html "struct pert_ui::KeyChord")

The platform’s conventional chord for going forward in a navigation
history: Cmd+\] on macOS and Alt+Right elsewhere; see
[`Self::navigate_back`](struct.KeyChord.html#method.navigate_back "associated function pert_ui::KeyChord::navigate_back").

[Source](../src/pert_ui_runtime/runtime/commands.rs.html#143)

#### pub fn [physical_fallback](#method.physical_fallback)(self, key: [KeyCode](enum.KeyCode.html "enum pert_ui::KeyCode")) -\> [KeyChord](struct.KeyChord.html "struct pert_ui::KeyChord")

Enables a physical fallback only for native inputs lacking logical
identity.

[Source](../src/pert_ui_runtime/runtime/commands.rs.html#149)

#### pub fn [key](#method.key)(self) -\> [CommandKey](enum.CommandKey.html "enum pert_ui::CommandKey")

The layout identity this chord matches; ASCII letters are lowercase.

[Source](../src/pert_ui_runtime/runtime/commands.rs.html#154)

#### pub fn [modifiers](#method.modifiers)(self) -\> [Modifiers](struct.Modifiers.html "struct pert_ui::Modifiers")

The exact modifiers this chord requires.

## Trait Implementations[§](#trait-implementations)

[Source](../src/pert_ui_runtime/runtime/commands.rs.html#25)[§](#impl-Clone-for-KeyChord)

### impl [Clone](https://doc.rust-lang.org/1.99.1/core/clone/trait.Clone.html "trait core::clone::Clone") for [KeyChord](struct.KeyChord.html "struct pert_ui::KeyChord")

[Source](../src/pert_ui_runtime/runtime/commands.rs.html#25)[§](#method.clone)

#### fn [clone](https://doc.rust-lang.org/1.99.1/core/clone/trait.Clone.html#tymethod.clone)(&self) -\> [KeyChord](struct.KeyChord.html "struct pert_ui::KeyChord")

Returns a duplicate of the value. [Read
more](https://doc.rust-lang.org/1.99.1/core/clone/trait.Clone.html#tymethod.clone)

1.0.0 (const:
[unstable](https://github.com/rust-lang/rust/issues/142757 "Tracking issue for const_clone"))
·
[Source](https://doc.rust-lang.org/1.99.1/src/core/clone.rs.html#245-247)[§](#method.clone_from)

#### fn [clone_from](https://doc.rust-lang.org/1.99.1/core/clone/trait.Clone.html#method.clone_from)(&mut self, source: &Self)

Performs copy-assignment from `source`. [Read
more](https://doc.rust-lang.org/1.99.1/core/clone/trait.Clone.html#method.clone_from)

[Source](../src/pert_ui_runtime/runtime/commands.rs.html#25)[§](#impl-Copy-for-KeyChord)

### impl [Copy](https://doc.rust-lang.org/1.99.1/core/marker/trait.Copy.html "trait core::marker::Copy") for [KeyChord](struct.KeyChord.html "struct pert_ui::KeyChord")

[Source](../src/pert_ui_runtime/runtime/commands.rs.html#25)[§](#impl-Debug-for-KeyChord)

### impl [Debug](https://doc.rust-lang.org/1.99.1/core/fmt/trait.Debug.html "trait core::fmt::Debug") for [KeyChord](struct.KeyChord.html "struct pert_ui::KeyChord")

[Source](../src/pert_ui_runtime/runtime/commands.rs.html#25)[§](#method.fmt)

#### fn [fmt](https://doc.rust-lang.org/1.99.1/core/fmt/trait.Debug.html#tymethod.fmt)(&self, f: &mut [Formatter](https://doc.rust-lang.org/1.99.1/core/fmt/struct.Formatter.html "struct core::fmt::Formatter")\<'\_\>) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[()](https://doc.rust-lang.org/1.99.1/std/primitive.unit.html), [Error](https://doc.rust-lang.org/1.99.1/core/fmt/struct.Error.html "struct core::fmt::Error")\>

Formats the value using the given formatter. [Read
more](https://doc.rust-lang.org/1.99.1/core/fmt/trait.Debug.html#tymethod.fmt)

[Source](../src/pert_ui_runtime/runtime/commands.rs.html#199)[§](#impl-Display-for-KeyChord)

### impl [Display](https://doc.rust-lang.org/1.99.1/core/fmt/trait.Display.html "trait core::fmt::Display") for [KeyChord](struct.KeyChord.html "struct pert_ui::KeyChord")

Presents the chord in the running platform’s convention: modifier names
joined with `+`, then the key, such as `Ctrl+Shift+S`. macOS names
Option and Cmd in its own order, as `Ctrl+Option+Shift+Cmd+S`. Letters
are uppercase. The names are English words rather than symbols such as
⌘, because the bundled fonts do not carry those glyphs. The physical
fallback is not shown.

``` rust
use pert_ui::{CommandKey, KeyChord, KeyCode, Modifiers};
let chord = KeyChord::named(KeyCode::F5);
assert_eq!(chord.to_string(), "F5");
let shifted = KeyChord::new(CommandKey::Character('z'), Modifiers::default()).shifted();
assert_eq!(shifted.to_string(), "Shift+Z");
```

[Source](../src/pert_ui_runtime/runtime/commands.rs.html#200)[§](#method.fmt-1)

#### fn [fmt](https://doc.rust-lang.org/1.99.1/core/fmt/trait.Display.html#tymethod.fmt)(&self, f: &mut [Formatter](https://doc.rust-lang.org/1.99.1/core/fmt/struct.Formatter.html "struct core::fmt::Formatter")\<'\_\>) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[()](https://doc.rust-lang.org/1.99.1/std/primitive.unit.html), [Error](https://doc.rust-lang.org/1.99.1/core/fmt/struct.Error.html "struct core::fmt::Error")\>

Formats the value using the given formatter. [Read
more](https://doc.rust-lang.org/1.99.1/core/fmt/trait.Display.html#tymethod.fmt)

[Source](../src/pert_ui_runtime/runtime/commands.rs.html#25)[§](#impl-Eq-for-KeyChord)

### impl [Eq](https://doc.rust-lang.org/1.99.1/core/cmp/trait.Eq.html "trait core::cmp::Eq") for [KeyChord](struct.KeyChord.html "struct pert_ui::KeyChord")

[Source](../src/pert_ui_runtime/runtime/commands.rs.html#25)[§](#impl-Hash-for-KeyChord)

### impl [Hash](https://doc.rust-lang.org/1.99.1/core/hash/trait.Hash.html "trait core::hash::Hash") for [KeyChord](struct.KeyChord.html "struct pert_ui::KeyChord")

[Source](../src/pert_ui_runtime/runtime/commands.rs.html#25)[§](#method.hash)

#### fn [hash](https://doc.rust-lang.org/1.99.1/core/hash/trait.Hash.html#tymethod.hash)\<\_\_H\>(&self, state: [&mut \_\_H](https://doc.rust-lang.org/1.99.1/std/primitive.reference.html))

where \_\_H:
[Hasher](https://doc.rust-lang.org/1.99.1/core/hash/trait.Hasher.html "trait core::hash::Hasher"),

Feeds this value into the given
[`Hasher`](https://doc.rust-lang.org/1.99.1/core/hash/trait.Hasher.html "trait core::hash::Hasher").
[Read
more](https://doc.rust-lang.org/1.99.1/core/hash/trait.Hash.html#tymethod.hash)

1.3.0 ·
[Source](https://doc.rust-lang.org/1.99.1/src/core/hash/mod.rs.html#234-236)[§](#method.hash_slice)

#### fn [hash_slice](https://doc.rust-lang.org/1.99.1/core/hash/trait.Hash.html#method.hash_slice)\<H\>(data: &\[Self\], state: [&mut H](https://doc.rust-lang.org/1.99.1/std/primitive.reference.html))

where H:
[Hasher](https://doc.rust-lang.org/1.99.1/core/hash/trait.Hasher.html "trait core::hash::Hasher"),
Self:
[Sized](https://doc.rust-lang.org/1.99.1/core/marker/trait.Sized.html "trait core::marker::Sized"),

Feeds a slice of this type into the given
[`Hasher`](https://doc.rust-lang.org/1.99.1/core/hash/trait.Hasher.html "trait core::hash::Hasher").
[Read
more](https://doc.rust-lang.org/1.99.1/core/hash/trait.Hash.html#method.hash_slice)

[Source](../src/pert_ui_runtime/runtime/commands.rs.html#25)[§](#impl-PartialEq-for-KeyChord)

### impl [PartialEq](https://doc.rust-lang.org/1.99.1/core/cmp/trait.PartialEq.html "trait core::cmp::PartialEq") for [KeyChord](struct.KeyChord.html "struct pert_ui::KeyChord")

[Source](../src/pert_ui_runtime/runtime/commands.rs.html#25)[§](#method.eq)

#### fn [eq](https://doc.rust-lang.org/1.99.1/core/cmp/trait.PartialEq.html#tymethod.eq)(&self, other: &[KeyChord](struct.KeyChord.html "struct pert_ui::KeyChord")) -\> [bool](https://doc.rust-lang.org/1.99.1/std/primitive.bool.html)

Equality operator `==`. [Read
more](https://doc.rust-lang.org/1.99.1/core/cmp/trait.PartialEq.html#tymethod.eq)

1.0.0 (const:
[unstable](https://github.com/rust-lang/rust/issues/143800 "Tracking issue for const_cmp"))
·
[Source](https://doc.rust-lang.org/1.99.1/src/core/cmp.rs.html#275)[§](#method.ne)

#### fn [ne](https://doc.rust-lang.org/1.99.1/core/cmp/trait.PartialEq.html#method.ne)(&self, other: [&Rhs](https://doc.rust-lang.org/1.99.1/std/primitive.reference.html)) -\> [bool](https://doc.rust-lang.org/1.99.1/std/primitive.bool.html)

Inequality operator `!=`. [Read
more](https://doc.rust-lang.org/1.99.1/core/cmp/trait.PartialEq.html#method.ne)

[Source](../src/pert_ui_runtime/runtime/commands.rs.html#25)[§](#impl-StructuralPartialEq-for-KeyChord)

### impl [StructuralPartialEq](https://doc.rust-lang.org/1.99.1/core/marker/trait.StructuralPartialEq.html "trait core::marker::StructuralPartialEq") for [KeyChord](struct.KeyChord.html "struct pert_ui::KeyChord")

## Auto Trait Implementations[§](#synthetic-implementations)

[§](#impl-Freeze-for-KeyChord)

### impl [Freeze](https://doc.rust-lang.org/1.99.1/core/marker/trait.Freeze.html "trait core::marker::Freeze") for [KeyChord](struct.KeyChord.html "struct pert_ui::KeyChord")

[§](#impl-RefUnwindSafe-for-KeyChord)

### impl [RefUnwindSafe](https://doc.rust-lang.org/1.99.1/core/panic/unwind_safe/trait.RefUnwindSafe.html "trait core::panic::unwind_safe::RefUnwindSafe") for [KeyChord](struct.KeyChord.html "struct pert_ui::KeyChord")

[§](#impl-Send-for-KeyChord)

### impl [Send](https://doc.rust-lang.org/1.99.1/core/marker/trait.Send.html "trait core::marker::Send") for [KeyChord](struct.KeyChord.html "struct pert_ui::KeyChord")

[§](#impl-Sync-for-KeyChord)

### impl [Sync](https://doc.rust-lang.org/1.99.1/core/marker/trait.Sync.html "trait core::marker::Sync") for [KeyChord](struct.KeyChord.html "struct pert_ui::KeyChord")

[§](#impl-Unpin-for-KeyChord)

### impl [Unpin](https://doc.rust-lang.org/1.99.1/core/marker/trait.Unpin.html "trait core::marker::Unpin") for [KeyChord](struct.KeyChord.html "struct pert_ui::KeyChord")

[§](#impl-UnsafeUnpin-for-KeyChord)

### impl [UnsafeUnpin](https://doc.rust-lang.org/1.99.1/core/marker/trait.UnsafeUnpin.html "trait core::marker::UnsafeUnpin") for [KeyChord](struct.KeyChord.html "struct pert_ui::KeyChord")

[§](#impl-UnwindSafe-for-KeyChord)

### impl [UnwindSafe](https://doc.rust-lang.org/1.99.1/core/panic/unwind_safe/trait.UnwindSafe.html "trait core::panic::unwind_safe::UnwindSafe") for [KeyChord](struct.KeyChord.html "struct pert_ui::KeyChord")

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

[Source](../src/downcast_rs/lib.rs.html#203)[§](#impl-DowncastSync-for-T)

### impl\<T\> [DowncastSync](../downcast_rs/trait.DowncastSync.html "trait downcast_rs::DowncastSync") for T

where T:
[Any](https://doc.rust-lang.org/1.99.1/core/any/trait.Any.html "trait core::any::Any") +
[Send](https://doc.rust-lang.org/1.99.1/core/marker/trait.Send.html "trait core::marker::Send") +
[Sync](https://doc.rust-lang.org/1.99.1/core/marker/trait.Sync.html "trait core::marker::Sync"),

[Source](../src/downcast_rs/lib.rs.html#204)[§](#method.into_any_arc)

#### fn [into_any_arc](../downcast_rs/trait.DowncastSync.html#tymethod.into_any_arc)(self: [Arc](https://doc.rust-lang.org/1.99.1/alloc/sync/struct.Arc.html "struct alloc::sync::Arc")\<T\>) -\> [Arc](https://doc.rust-lang.org/1.99.1/alloc/sync/struct.Arc.html "struct alloc::sync::Arc")\<dyn [Any](https://doc.rust-lang.org/1.99.1/core/any/trait.Any.html "trait core::any::Any") + [Send](https://doc.rust-lang.org/1.99.1/core/marker/trait.Send.html "trait core::marker::Send") + [Sync](https://doc.rust-lang.org/1.99.1/core/marker/trait.Sync.html "trait core::marker::Sync")\> [ⓘ](#)

Convert `Arc<Trait>` (where `Trait: Downcast`) to `Arc<Any>`. `Arc<Any>`
can then be further `downcast` into `Arc<ConcreteType>` where
`ConcreteType` implements `Trait`.

[Source](../src/hashbrown/lib.rs.html#151-154)[§](#impl-Equivalent%3CK%3E-for-Q)

### impl\<Q, K\> [Equivalent](../hashbrown/trait.Equivalent.html "trait hashbrown::Equivalent")\<K\> for Q

where Q:
[Eq](https://doc.rust-lang.org/1.99.1/core/cmp/trait.Eq.html "trait core::cmp::Eq") +
?[Sized](https://doc.rust-lang.org/1.99.1/core/marker/trait.Sized.html "trait core::marker::Sized"),
K:
[Borrow](https://doc.rust-lang.org/1.99.1/core/borrow/trait.Borrow.html "trait core::borrow::Borrow")\<Q\> +
?[Sized](https://doc.rust-lang.org/1.99.1/core/marker/trait.Sized.html "trait core::marker::Sized"),

[Source](../src/hashbrown/lib.rs.html#156)[§](#method.equivalent)

#### fn [equivalent](../hashbrown/trait.Equivalent.html#tymethod.equivalent)(&self, key: [&K](https://doc.rust-lang.org/1.99.1/std/primitive.reference.html)) -\> [bool](https://doc.rust-lang.org/1.99.1/std/primitive.bool.html)

Checks if this value is equivalent to the given key. [Read
more](../hashbrown/trait.Equivalent.html#tymethod.equivalent)

[Source](../src/equivalent/lib.rs.html#82-85)[§](#impl-Equivalent%3CK%3E-for-Q-1)

### impl\<Q, K\> [Equivalent](../equivalent/trait.Equivalent.html "trait equivalent::Equivalent")\<K\> for Q

where Q:
[Eq](https://doc.rust-lang.org/1.99.1/core/cmp/trait.Eq.html "trait core::cmp::Eq") +
?[Sized](https://doc.rust-lang.org/1.99.1/core/marker/trait.Sized.html "trait core::marker::Sized"),
K:
[Borrow](https://doc.rust-lang.org/1.99.1/core/borrow/trait.Borrow.html "trait core::borrow::Borrow")\<Q\> +
?[Sized](https://doc.rust-lang.org/1.99.1/core/marker/trait.Sized.html "trait core::marker::Sized"),

[Source](../src/equivalent/lib.rs.html#88)[§](#method.equivalent-1)

#### fn [equivalent](../equivalent/trait.Equivalent.html#tymethod.equivalent)(&self, key: [&K](https://doc.rust-lang.org/1.99.1/std/primitive.reference.html)) -\> [bool](https://doc.rust-lang.org/1.99.1/std/primitive.bool.html)

Compare self to `key` and return `true` if they are equal.

[Source](../src/hashbrown/lib.rs.html#166-169)[§](#impl-Equivalent%3CK%3E-for-Q-2)

### impl\<Q, K\> [Equivalent](../hashbrown/trait.Equivalent.html "trait hashbrown::Equivalent")\<K\> for Q

where Q:
[Eq](https://doc.rust-lang.org/1.99.1/core/cmp/trait.Eq.html "trait core::cmp::Eq") +
?[Sized](https://doc.rust-lang.org/1.99.1/core/marker/trait.Sized.html "trait core::marker::Sized"),
K:
[Borrow](https://doc.rust-lang.org/1.99.1/core/borrow/trait.Borrow.html "trait core::borrow::Borrow")\<Q\> +
?[Sized](https://doc.rust-lang.org/1.99.1/core/marker/trait.Sized.html "trait core::marker::Sized"),

[Source](../src/hashbrown/lib.rs.html#171)[§](#method.equivalent-2)

#### fn [equivalent](../hashbrown/trait.Equivalent.html#tymethod.equivalent)(&self, key: [&K](https://doc.rust-lang.org/1.99.1/std/primitive.reference.html)) -\> [bool](https://doc.rust-lang.org/1.99.1/std/primitive.bool.html)

Checks if this value is equivalent to the given key. [Read
more](../hashbrown/trait.Equivalent.html#tymethod.equivalent)

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

[Source](../src/smol_str/lib.rs.html#760-762)[§](#impl-ToSmolStr-for-T)

### impl\<T\> [ToSmolStr](../smol_str/trait.ToSmolStr.html "trait smol_str::ToSmolStr") for T

where T:
[Display](https://doc.rust-lang.org/1.99.1/core/fmt/trait.Display.html "trait core::fmt::Display") +
?[Sized](https://doc.rust-lang.org/1.99.1/core/marker/trait.Sized.html "trait core::marker::Sized"),

[Source](../src/smol_str/lib.rs.html#764)[§](#method.to_smolstr)

#### fn [to_smolstr](../smol_str/trait.ToSmolStr.html#tymethod.to_smolstr)(&self) -\> [SmolStr](../smol_str/struct.SmolStr.html "struct smol_str::SmolStr")

[Source](https://doc.rust-lang.org/1.99.1/src/alloc/string.rs.html#2928)[§](#impl-ToString-for-T)

### impl\<T\> [ToString](https://doc.rust-lang.org/1.99.1/alloc/string/trait.ToString.html "trait alloc::string::ToString") for T

where T:
[Display](https://doc.rust-lang.org/1.99.1/core/fmt/trait.Display.html "trait core::fmt::Display") +
?[Sized](https://doc.rust-lang.org/1.99.1/core/marker/trait.Sized.html "trait core::marker::Sized"),

[Source](https://doc.rust-lang.org/1.99.1/src/alloc/string.rs.html#2930)[§](#method.to_string)

#### fn [to_string](https://doc.rust-lang.org/1.99.1/alloc/string/trait.ToString.html#tymethod.to_string)(&self) -\> [String](https://doc.rust-lang.org/1.99.1/alloc/string/struct.String.html "struct alloc::string::String")

Converts the given value to a `String`. [Read
more](https://doc.rust-lang.org/1.99.1/alloc/string/trait.ToString.html#tymethod.to_string)

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

[Source](../src/wgpu_types/send_sync.rs.html#18)[§](#impl-WasmNotSend-for-T)

### impl\<T\> [WasmNotSend](../wgpu_types/send_sync/trait.WasmNotSend.html "trait wgpu_types::send_sync::WasmNotSend") for T

where T:
[Send](https://doc.rust-lang.org/1.99.1/core/marker/trait.Send.html "trait core::marker::Send"),

[Source](../src/wgpu_types/send_sync.rs.html#2)[§](#impl-WasmNotSendSync-for-T)

### impl\<T\> [WasmNotSendSync](../wgpu_types/send_sync/trait.WasmNotSendSync.html "trait wgpu_types::send_sync::WasmNotSendSync") for T

where T:
[WasmNotSend](../wgpu_types/send_sync/trait.WasmNotSend.html "trait wgpu_types::send_sync::WasmNotSend") +
[WasmNotSync](../wgpu_types/send_sync/trait.WasmNotSync.html "trait wgpu_types::send_sync::WasmNotSync"),

[Source](../src/wgpu_types/send_sync.rs.html#51)[§](#impl-WasmNotSync-for-T)

### impl\<T\> [WasmNotSync](../wgpu_types/send_sync/trait.WasmNotSync.html "trait wgpu_types::send_sync::WasmNotSync") for T

where T:
[Sync](https://doc.rust-lang.org/1.99.1/core/marker/trait.Sync.html "trait core::marker::Sync"),

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
