[pert_ui](index.html)

# Struct TextArea Copy item path

[Source](../src/pert_ui_runtime/text_fields.rs.html#265)

``` rust
pub struct TextArea(/* private fields */);
```

Expand description

A multiline local text editor with monospace typography by default.

Uses bundled Noto Sans Mono, a 320-pixel preferred width and six visible
lines. Soft wraps to its allocation and scrolls internally to reveal the
caret; wheel scrolls the text. Enter inserts LF, Tab traverses focus.
Pasted CRLF, CR, NEL, vertical tab, form feed and the Unicode line and
paragraph separators become LF, pasted tabs are kept and other pasted
control characters are dropped. Bound text accepts LF and tabs, but no
other control characters or line separators; text read from a file with
CRLF line endings, for example, must be converted before it is written
to the binding. Call `style` to override typography.

Text taller than the field shows the same vertical scrollbar as scroll
areas, views and windows: a track strip along the text’s right edge with
a thumb in `ColorRole::ScrollbarThumb`, or `ScrollbarThumbHover` while
the pointer is over the strip or dragging it, sized by the theme’s
scrollbar metrics (8, 4 and at least 12 logical pixels at Standard). It
appears only while the text overflows and overlays the text instead of
rewrapping it, so showing it never reflows the lines. Primary dragging
preserves the thumb grab position; clicking the track centers the thumb
there and starts a drag. Release ends dragging, and Escape or context
loss cancels it at the current offset without restoring the previous
one. Scrolling never moves the caret or selection, and the next caret
movement, edit or resize reveals the caret again. Scrolling repaints
without remeasuring, and the pointer keeps the ordinary arrow over the
bar. A read-only field keeps its bar; a disabled one shows none, because
it does not scroll at all. Single-line fields scroll horizontally
without a scrollbar.

Binding ownership and refused bindings,
[`TextEditError::MAX_TEXT_BYTES`](enum.TextEditError.html#associatedconstant.MAX_TEXT_BYTES "associated constant pert_ui::TextEditError::MAX_TEXT_BYTES")/preparation
limits, selection, clipboard, undo, cancellation and scheduling follow
`TextField`, and so do the [chords it
claims](struct.TextField.html#keys "struct pert_ui::TextField"), except
that a text area also claims Up and Down with Alt or Super outside
macOS, as line movement. This is plain text, with no syntax highlighting
or host project undo policy. An enabled field uses SurfaceHover under
the pointer without changing layout.

## [§](#normalization)Normalization

[`Self::normalize`](struct.TextArea.html#method.normalize "method pert_ui::TextArea::normalize")
follows [`TextField`’s
normalization](struct.TextField.html#normalization "struct pert_ui::TextField"),
except that Enter inserts a line break, so only focus transfer or window
focus loss saves the draft.

``` rust
use pert_ui::{Element, Runtime, TextArea};
let mut ui = Runtime::new();
let notes = ui.signal(String::from("First line\nSecond line"));
ui.mount_tree(Element::new("notes", TextArea::new(notes)))?;
```

## Implementations[§](#implementations)

[Source](../src/pert_ui_runtime/text_fields.rs.html#858)[§](#impl-TextArea)

### impl [TextArea](struct.TextArea.html "struct pert_ui::TextArea")

[Source](../src/pert_ui_runtime/text_fields.rs.html#858)

#### pub fn [new](#method.new)(value: impl [Into](https://doc.rust-lang.org/1.99.1/core/convert/trait.Into.html "trait core::convert::Into")\<[Signal](struct.Signal.html "struct pert_ui::Signal")\<[String](https://doc.rust-lang.org/1.99.1/alloc/string/struct.String.html "struct alloc::string::String")\>\>) -\> [TextArea](struct.TextArea.html "struct pert_ui::TextArea")

Creates an enabled editor. The binding is subscribed on the first
update; a foreign-runtime binding fails that update, and a value the
editor does not accept is refused as described above.

[Source](../src/pert_ui_runtime/text_fields.rs.html#858)

#### pub fn [check](#method.check)(text: &[str](https://doc.rust-lang.org/1.99.1/std/primitive.str.html)) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[()](https://doc.rust-lang.org/1.99.1/std/primitive.unit.html), [TextEditError](enum.TextEditError.html "enum pert_ui::TextEditError")\>

Checks whether `text` is acceptable as this editor’s bound value, so an
application can reject or convert text, such as a document read from a
file, before writing it to the binding. This performs no text
preparation and needs no runtime.

##### [§](#errors)Errors

Returns
[`TextEditError::TooLarge`](enum.TextEditError.html#variant.TooLarge "variant pert_ui::TextEditError::TooLarge")
above
[`TextEditError::MAX_TEXT_BYTES`](enum.TextEditError.html#associatedconstant.MAX_TEXT_BYTES "associated constant pert_ui::TextEditError::MAX_TEXT_BYTES")
bytes and
[`TextEditError::InvalidText`](enum.TextEditError.html#variant.InvalidText "variant pert_ui::TextEditError::InvalidText")
for a control character or a Unicode line or paragraph separator that
the editor does not accept.

[Source](../src/pert_ui_runtime/text_fields.rs.html#858)

#### pub fn [style](#method.style)(self, style: [TextStyle](enum.TextStyle.html "enum pert_ui::TextStyle")) -\> [TextArea](struct.TextArea.html "struct pert_ui::TextArea")

Overrides the default typography using a bundled semantic style.

[Source](../src/pert_ui_runtime/text_fields.rs.html#858)

#### pub fn [read_only](#method.read_only)(self) -\> [TextArea](struct.TextArea.html "struct pert_ui::TextArea")

Allows selection/copy but prevents editing and native IME activation.
Equivalent to `editable(false)`; a later permission cannot lift it.

[Source](../src/pert_ui_runtime/text_fields.rs.html#858)

#### pub fn [editable](#method.editable)(self, permission: impl [Into](https://doc.rust-lang.org/1.99.1/core/convert/trait.Into.html "trait core::convert::Into")\<[Permission](struct.Permission.html "struct pert_ui::Permission")\>) -\> [TextArea](struct.TextArea.html "struct pert_ui::TextArea")

Restricts editing with a fixed or reactive permission; editable by
default. Repeated calls combine restrictions; every condition must
allow, and same-key redescription replaces the permission with the new
description’s.

While editing is denied the editor keeps focus, scrolling, selection and
copying but refuses edits and native IME activation. Revoking it,
observed at the next update, cancels a composition and a selection drag;
an unsaved draft stays but is not saved, also not on focus loss, until
editing is allowed again, which replays nothing. A binding from another
runtime fails the update with
[`RuntimeError::ForeignRuntime`](enum.RuntimeError.html#variant.ForeignRuntime "variant pert_ui::RuntimeError::ForeignRuntime").

[Source](../src/pert_ui_runtime/text_fields.rs.html#858)

#### pub fn [disabled](#method.disabled)(self) -\> [TextArea](struct.TextArea.html "struct pert_ui::TextArea")

Prevents interaction and automatic focus; still follows its binding.
Equivalent to `enabled(false)`; a later permission cannot lift it.

[Source](../src/pert_ui_runtime/text_fields.rs.html#858)

#### pub fn [enabled](#method.enabled)(self, permission: impl [Into](https://doc.rust-lang.org/1.99.1/core/convert/trait.Into.html "trait core::convert::Into")\<[Permission](struct.Permission.html "struct pert_ui::Permission")\>) -\> [TextArea](struct.TextArea.html "struct pert_ui::TextArea")

Restricts focus and input with a fixed or reactive permission; enabled
by default. Repeated calls combine restrictions; every condition must
allow, and same-key redescription replaces the permission with the new
description’s. A disabled editor still follows its binding.

Revoking it, observed at the next update, makes the editor unfocusable
and releases focus, cancels a composition and a selection drag, and
keeps an unsaved draft without saving it, including on the focus loss
the revocation causes. Enabling it again makes the draft editable and
replays nothing. A binding from another runtime fails the update with
[`RuntimeError::ForeignRuntime`](enum.RuntimeError.html#variant.ForeignRuntime "variant pert_ui::RuntimeError::ForeignRuntime").

[Source](../src/pert_ui_runtime/text_fields.rs.html#858)

#### pub fn [normalize](#method.normalize)( self, normalize: impl [Fn](https://doc.rust-lang.org/1.99.1/core/ops/function/trait.Fn.html "trait core::ops::function::Fn")([String](https://doc.rust-lang.org/1.99.1/alloc/string/struct.String.html "struct alloc::string::String")) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[String](https://doc.rust-lang.org/1.99.1/alloc/string/struct.String.html "struct alloc::string::String"), [InputRejection](struct.InputRejection.html "struct pert_ui::InputRejection")\> + 'static, ) -\> [TextArea](struct.TextArea.html "struct pert_ui::TextArea")

Adjusts the entered text when it is saved, such as trimming it or
changing its case, or refuses it with an
[`InputRejection`](struct.InputRejection.html "struct pert_ui::InputRejection").

Timing class: [in the
input](https://github.com/pertyjons/pert-ui/blob/main/docs/contracts.md#in-the-input),
at the save it checks.

A normalizing editor holds a **draft**: edits stay local, and the
binding is written only by a save of a changed draft, which runs the
normalizer, checks the result as bound text
([`Self::check`](struct.TextArea.html#method.check "associated function pert_ui::TextArea::check")),
prepares it and writes it. See
[`Self`](struct.TextArea.html#normalization "struct pert_ui::TextArea")
for when saves happen. A refused draft writes nothing and stays for
correction with the error outline; the refusal ends when the draft
changes. Text the application writes to the binding is never normalized,
and it wins over a draft. Without a normalizer the editor writes the
binding on every edit, as by default.

The function must be deterministic, bounded and free of side effects,
binding writes and I/O, and idempotent: normalizing its own output must
return that output unchanged, so a saved value stays as it is on the
next save. It never sees IME preedit, only committed text. Installing
another normalizer replaces this one.

[Source](../src/pert_ui_runtime/text_fields.rs.html#858)

#### pub fn [on_rejection](#method.on_rejection)( self, report: impl [FnMut](https://doc.rust-lang.org/1.99.1/core/ops/function/trait.FnMut.html "trait core::ops::function::FnMut")(&mut [EventContext](struct.EventContext.html "struct pert_ui::EventContext"), [Option](https://doc.rust-lang.org/1.99.1/core/option/enum.Option.html "enum core::option::Option")\<&[InputRejection](struct.InputRejection.html "struct pert_ui::InputRejection")\>) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[()](https://doc.rust-lang.org/1.99.1/std/primitive.unit.html), [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\> + 'static, ) -\> [TextArea](struct.TextArea.html "struct pert_ui::TextArea")

Tells the application about the normalizer’s refusals, so it can present
them, for example by publishing a
[`FieldStatus`](enum.FieldStatus.html "enum pert_ui::FieldStatus")
through its
[`FieldHandle`](struct.FieldHandle.html "struct pert_ui::FieldHandle").

Timing class: [in the
input](https://github.com/pertyjons/pert-ui/blob/main/docs/contracts.md#in-the-input).

The callback runs on the UI thread in the event phase. It receives
`Some` on each save the normalizer or a length limit
([`Self::min_length`](struct.TextArea.html#method.min_length "method pert_ui::TextArea::min_length"),
[`Self::max_length`](struct.TextArea.html#method.max_length "method pert_ui::TextArea::max_length"))
refuses, also in an editor with limits and no normalizer, and on Enter
in a required field whose unchanged text is below its minimum, including
a repeated refusal of the same draft, and `None` once when a reported
refusal ends: at a later save that succeeds or finds nothing to save
(including focus loss), at Escape, and at
[`TextCommand::DiscardDraft`](enum.TextCommand.html#variant.DiscardDraft "variant pert_ui::TextCommand::DiscardDraft"),
which
[`FormFields::discard_drafts`](struct.FormFields.html#method.discard_drafts "method pert_ui::FormFields::discard_drafts")
sends to every field of a form. An application write or hiding that
discards the draft happens outside an event, so the next save or Escape
reports its end; that Escape still reaches the field’s owner. Removal
and shutdown report nothing: an owner that removes a field clears
whatever status it published for it. Only
[`InputRejection`](struct.InputRejection.html "struct pert_ui::InputRejection")s
are reported, not the field’s own text checks.

An error from the callback propagates from the input call that saved.
The field has already applied the outcome by then: a refused draft is
kept and marked, and a saved one is written. A failed `None` stays
pending and is reported again at the next save or Escape. Replaces an
earlier callback.

``` rust
use pert_ui::{
    Element, Field, FieldStatus, FormFields, InputRejection, KeyAction, KeyCode,
    Runtime, TextField,
};
let mut ui = Runtime::new();
let form = FormFields::new(&ui);
let handle = form.field("name")?;
let status = handle.clone();
let name = ui.signal(String::new());
let field = TextField::new(name.clone())
    .normalize(|text| {
        let text = text.trim().to_owned();
        if text.is_empty() { Err(InputRejection::new("Enter a name")) } else { Ok(text) }
    })
    .on_rejection(move |_, rejection| {
        let next = rejection.map_or(FieldStatus::Normal, |r| FieldStatus::error(r.message()));
        status.set(next)
    });
let root = ui.mount_tree(Element::new("name",
    Field::new(Element::new("input", field)).label("Name").form(&handle)))?;
let input = ui.find_descendant(&root, "input")?.expect("the field's control");
ui.focus(&input)?;
let enter = |ui: &mut Runtime| -> Result<(), pert_ui::RuntimeError> {
    ui.key_input(KeyCode::Enter, KeyAction::Press)?;
    ui.key_input(KeyCode::Enter, KeyAction::Release).map(|_| ())
};
ui.text_input("  ")?;
enter(&mut ui)?;
assert_eq!(handle.status().get()?, FieldStatus::error("Enter a name"));
ui.text_input("Ada")?;
enter(&mut ui)?;
assert_eq!(handle.status().get()?, FieldStatus::Normal);
assert_eq!(name.get()?, "Ada");
```

[Source](../src/pert_ui_runtime/text_fields.rs.html#858)

#### pub fn [on_save](#method.on_save)( self, saved: impl [FnMut](https://doc.rust-lang.org/1.99.1/core/ops/function/trait.FnMut.html "trait core::ops::function::FnMut")(&mut [EventContext](struct.EventContext.html "struct pert_ui::EventContext"), &[str](https://doc.rust-lang.org/1.99.1/std/primitive.str.html)) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[()](https://doc.rust-lang.org/1.99.1/std/primitive.unit.html), [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\> + 'static, ) -\> [TextArea](struct.TextArea.html "struct pert_ui::TextArea")

Tells the application about each save that wrote the binding, with the
saved (normalized) text, so it can act on a value the user finished
entering, for example to update other state from it.

Timing class: [in the
input](https://github.com/pertyjons/pert-ui/blob/main/docs/contracts.md#in-the-input).

Only an editor in draft mode saves, one with a normalizer, a length
limit or, for a `TextField`, a
[session](struct.TextField.html#method.session "method pert_ui::TextField::session"):
see
[`Self::normalize`](struct.TextArea.html#method.normalize "method pert_ui::TextArea::normalize")
for when. The callback runs on the UI thread in the event phase, after
the binding holds the saved text, once per save that wrote it, whether
Enter or focus loss saved it. It does not run for a refused draft, a
draft that normalizes to the bound text (which writes nothing), Escape,
application writes, hiding or removal. Without a normalizer or a length
limit the editor writes the binding on every edit and never calls it;
observe the binding instead. To save finished edits without adjusting or
limiting the text, for example to record each one as one undo step,
install the identity normalizer: `.normalize(Ok)`. A `TextField` then
saves on Enter and on focus loss; in a `TextArea` Enter inserts a line
break, so it saves on focus loss.

The callback may write bindings. Its error propagates from the input
call that saved; the text has been written by then. A failing
[`Self::on_rejection`](struct.TextArea.html#method.on_rejection "method pert_ui::TextArea::on_rejection")
report does not suppress it: both run, and the report’s error is
returned first. Replaces an earlier callback.

[Source](../src/pert_ui_runtime/text_fields.rs.html#858)

#### pub fn [placeholder](#method.placeholder)(self, text: impl [Into](https://doc.rust-lang.org/1.99.1/core/convert/trait.Into.html "trait core::convert::Into")\<[TextSource](struct.TextSource.html "struct pert_ui::TextSource")\>) -\> [TextArea](struct.TextArea.html "struct pert_ui::TextArea")

Sets muted hint text shown while the editor is empty and nothing is
being composed, such as “Name of the project”. It is presentation only:
never part of the value, of the semantic value or of inspection’s
`text`; inspection reports it as `placeholder`. Empty, the default,
shows none.

The hint is a
[`TextSource`](struct.TextSource.html "struct pert_ui::TextSource"):
fixed plain or rich text, a binding the editor follows, or a
[`StringRole`](enum.StringRole.html "enum pert_ui::StringRole") that
follows the installed string table. A binding or role is read in the
editor’s update; describing the editor again with the same source keeps
the text already read.

[Source](../src/pert_ui_runtime/text_fields.rs.html#858)

#### pub fn [required](#method.required)(self, required: [bool](https://doc.rust-lang.org/1.99.1/std/primitive.bool.html)) -\> [TextArea](struct.TextArea.html "struct pert_ui::TextArea")

Marks the value as required by the application. Semantics report it
([`Semantics::required`](struct.Semantics.html#method.required "method pert_ui::Semantics::required"))
and inspection reports `required`. It decides nothing by itself: an
empty required editor is not an error unless the application publishes
one, except that
[`Self::min_length`](struct.TextArea.html#method.min_length "method pert_ui::TextArea::min_length")
then applies to the empty value too. Wrap the editor in a
[`Field`](struct.Field.html "struct pert_ui::Field") with
[`Field::required`](struct.Field.html#method.required "method pert_ui::Field::required")
to show the marker beside the label. Default: not required.

[Source](../src/pert_ui_runtime/text_fields.rs.html#858)

#### pub fn [min_length](#method.min_length)(self, graphemes: [Option](https://doc.rust-lang.org/1.99.1/core/option/enum.Option.html "enum core::option::Option")\<[usize](https://doc.rust-lang.org/1.99.1/std/primitive.usize.html)\>) -\> [TextArea](struct.TextArea.html "struct pert_ui::TextArea")

Sets a minimum length in grapheme clusters (user-perceived characters,
not bytes or `char`s): refuses a saved value with fewer than `graphemes`
of them, with an
[`InputRejection`](struct.InputRejection.html "struct pert_ui::InputRejection")
that uses
[`StringRole::AtLeastCharacters`](enum.StringRole.html#variant.AtLeastCharacters "variant pert_ui::StringRole::AtLeastCharacters")
from the installed string table (English default: “Character count must
be at least `count`”).

A length limit puts the editor in draft mode, as
[`Self::normalize`](struct.TextArea.html#method.normalize "method pert_ui::TextArea::normalize")
does: edits stay local and a save checks the limits after normalizing,
then writes. An empty value passes unless the editor is
[required](struct.TextArea.html#method.required "method pert_ui::TextArea::required"),
so an optional field may stay empty. Typing is never stopped at a limit;
the check happens when the draft is saved. Text the application writes
is not checked. `None`, the default, sets no minimum.

[Source](../src/pert_ui_runtime/text_fields.rs.html#858)

#### pub fn [max_length](#method.max_length)(self, graphemes: [Option](https://doc.rust-lang.org/1.99.1/core/option/enum.Option.html "enum core::option::Option")\<[usize](https://doc.rust-lang.org/1.99.1/std/primitive.usize.html)\>) -\> [TextArea](struct.TextArea.html "struct pert_ui::TextArea")

Sets a maximum length in grapheme clusters, as
[`Self::min_length`](struct.TextArea.html#method.min_length "method pert_ui::TextArea::min_length")
counts them: refuses a saved value with more than `graphemes` of them,
with an
[`InputRejection`](struct.InputRejection.html "struct pert_ui::InputRejection")
from
[`StringRole::AtMostCharacters`](enum.StringRole.html#variant.AtMostCharacters "variant pert_ui::StringRole::AtMostCharacters")
in the installed string table (English default: “Character count must be
at most `count`”). Draft mode and checking follow
[`Self::min_length`](struct.TextArea.html#method.min_length "method pert_ui::TextArea::min_length");
typing past the limit is allowed and the
[counter](struct.TextArea.html#method.counter "method pert_ui::TextArea::counter")
shows it in the Error role. `None`, the default, sets no maximum.

[Source](../src/pert_ui_runtime/text_fields.rs.html#858)

#### pub fn [counter](#method.counter)(self, shown: [bool](https://doc.rust-lang.org/1.99.1/std/primitive.bool.html)) -\> [TextArea](struct.TextArea.html "struct pert_ui::TextArea")

Shows the draft’s grapheme count against its limit inside the field, in
caption text: “12/50” against the maximum, or against the minimum when
there is no maximum. The count is Muted, or in the Error role while it
is above the maximum or below a nonzero minimum. Without a limit it
shows the count alone. Its space is reserved for the widest count a
draft can reach, so typing never reflows the field and no count is
clipped. Default: off.

[Source](../src/pert_ui_runtime/text_fields.rs.html#858)

#### pub fn [modified_indicator](#method.modified_indicator)(self, shown: [bool](https://doc.rust-lang.org/1.99.1/std/primitive.bool.html)) -\> [TextArea](struct.TextArea.html "struct pert_ui::TextArea")

Paints a small mark in the field’s top end corner, in the Primary role,
while an unsaved draft differs from the saved value. Only an editor in
draft mode (with a
[normalizer](struct.TextArea.html#method.normalize "method pert_ui::TextArea::normalize"),
a length limit or, for a `TextField`, a
[session](struct.TextField.html#method.session "method pert_ui::TextField::session"))
holds such a draft; otherwise every edit is written at once and nothing
is ever unsaved. Inspection reports the state as `dirty` and the
semantics describe an unsaved draft with the installed
[`UiStrings`](struct.UiStrings.html "struct pert_ui::UiStrings")’
[`StringRole::TextModified`](enum.StringRole.html#variant.TextModified "variant pert_ui::StringRole::TextModified"),
with or without the mark. Default: no mark.

[Source](../src/pert_ui_runtime/text_fields.rs.html#858)

#### pub fn [on_modified](#method.on_modified)( self, modified: impl [FnMut](https://doc.rust-lang.org/1.99.1/core/ops/function/trait.FnMut.html "trait core::ops::function::FnMut")(&mut [EventContext](struct.EventContext.html "struct pert_ui::EventContext"), [bool](https://doc.rust-lang.org/1.99.1/std/primitive.bool.html)) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[()](https://doc.rust-lang.org/1.99.1/std/primitive.unit.html), [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\> + 'static, ) -\> [TextArea](struct.TextArea.html "struct pert_ui::TextArea")

Calls `modified` in the event phase with `true` when the draft starts to
differ from the saved value and `false` when it stops: after a save,
Escape, an application write that discards the draft, or an edit back to
the saved text. Changes made in an update, such as an application write,
are delivered later in the same flush; each change is delivered once and
in order. The callback may write signals, for example
[`FieldHandle::set_modified`](struct.FieldHandle.html#method.set_modified "method pert_ui::FieldHandle::set_modified")
so a [`FormFields`](struct.FormFields.html "struct pert_ui::FormFields")
counts unsaved fields. Only an editor in draft mode ever reports `true`;
it keeps a draft unsaved while disabled or read-only, which this
reports, as the [draft
policy](https://github.com/pertyjons/pert-ui/blob/main/docs/contracts.md#draft-policy)
states. Its error is returned from the flush or input call that
delivered it and is not replayed; a removed field reports nothing. With
a
[session](struct.TextField.html#method.session "method pert_ui::TextField::session")
the report of a completion is requested before the session’s, so a host
that ends the edit and removes the field does so after the field’s
`false`. Replaced by a later call and by each description.

Timing class: [after
settling](https://github.com/pertyjons/pert-ui/blob/main/docs/contracts.md#after-settling).

``` rust
use pert_ui::prelude::*;
use pert_ui::KeyAction;
let mut ui = Runtime::new();
let form = FormFields::new(&ui);
let handle = form.field("name")?;
let name = ui.signal(String::new());
let report = handle.clone();
let field = ui.mount_tree(Element::new("name",
    TextField::new(name.clone())
        .max_length(Some(20))
        .on_modified(move |_, modified| report.set_modified(modified).map(|_| ()))))?;
ui.flush(std::time::Duration::ZERO)?;
ui.focus(&field)?;
ui.text_input("Ada")?;
ui.flush(std::time::Duration::ZERO)?;
assert_eq!(form.summary().get()?.modified(), 1);
ui.key_input(KeyCode::Enter, KeyAction::Press)?;
ui.key_input(KeyCode::Enter, KeyAction::Release)?;
assert_eq!(name.get()?, "Ada");
assert_eq!(form.summary().get()?.modified(), 0);
```

[Source](../src/pert_ui_runtime/text_fields.rs.html#858)

#### pub fn [on_binding_refusal](#method.on_binding_refusal)( self, refusal: impl [FnMut](https://doc.rust-lang.org/1.99.1/core/ops/function/trait.FnMut.html "trait core::ops::function::FnMut")(&mut [EventContext](struct.EventContext.html "struct pert_ui::EventContext"), [Option](https://doc.rust-lang.org/1.99.1/core/option/enum.Option.html "enum core::option::Option")\<&[TextEditError](enum.TextEditError.html "enum pert_ui::TextEditError")\>) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[()](https://doc.rust-lang.org/1.99.1/std/primitive.unit.html), [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\> + 'static, ) -\> [TextArea](struct.TextArea.html "struct pert_ui::TextArea")

Reports when the field refuses a value the application wrote to its
binding, with the reason, and with `None` when an acceptable value ends
the refusal.

Timing class: [after
settling](https://github.com/pertyjons/pert-ui/blob/main/docs/contracts.md#after-settling).

A refused value, such as text longer than
[`TextEditError::MAX_TEXT_BYTES`](enum.TextEditError.html#associatedconstant.MAX_TEXT_BYTES "associated constant pert_ui::TextEditError::MAX_TEXT_BYTES")
or a control character the field does not accept, leaves the field
showing its previous text, marked as refused and accepting no edits, as
the type describes; this callback is how the application learns of it,
for example to show a
[`FieldStatus`](enum.FieldStatus.html "enum pert_ui::FieldStatus") or
shorten the text. It concerns only the application’s own writes: refused
typing and a normalizer’s rejection, reported by
[`Self::on_rejection`](struct.TextArea.html#method.on_rejection "method pert_ui::TextArea::on_rejection"),
never call it. A refusal that starts and ends within one flush reports
nothing, and a later different reason reports again.

The callback runs in an event phase after the flush that saw the change
has settled, as
[`Self::on_modified`](struct.TextArea.html#method.on_modified "method pert_ui::TextArea::on_modified")
does, and may write bindings; its error is returned from the flush and
does not undo the refusal. Its reports keep their order with the field’s
other reports, such as
[`Self::on_modified`](struct.TextArea.html#method.on_modified "method pert_ui::TextArea::on_modified"),
and with other components’ notifications. Describing the field again
replaces the callback; a description without one stops the reports, and
installing one on a field that already refuses its value reports that
refusal.

``` rust
use pert_ui::{IntoElement, Runtime, TextArea, TextEditError, TextField};
use std::time::Duration;
let mut ui = Runtime::new();
let body = ui.signal(String::new());
let problem = ui.signal(None::<TextEditError>);
let report = problem.clone();
 let editor = TextArea::new(&body)
    .on_binding_refusal(move |_, refusal| report.set(refusal.cloned()));
ui.mount_tree(editor.key("body"))?;
ui.flush(Duration::ZERO)?;
body.set("x".repeat(TextEditError::MAX_TEXT_BYTES + 1))?;
ui.flush(Duration::ZERO)?;
assert_eq!(problem.get()?, Some(TextEditError::TooLarge));
body.set("short".to_owned())?;
ui.flush(Duration::ZERO)?;
assert_eq!(problem.get()?, None);
```

## Trait Implementations[§](#trait-implementations)

[Source](../src/pert_ui_runtime/text_fields.rs.html#858)[§](#impl-Component-for-TextArea)

### impl [Component](trait.Component.html "trait pert_ui::Component") for [TextArea](struct.TextArea.html "struct pert_ui::TextArea")

[Source](../src/pert_ui_runtime/text_fields.rs.html#858)[§](#method.reconcile)

#### fn [reconcile](trait.Component.html#method.reconcile)(&mut self, next: [TextArea](struct.TextArea.html "struct pert_ui::TextArea")) -\> [Option](https://doc.rust-lang.org/1.99.1/core/option/enum.Option.html "enum core::option::Option")\<[Invalidation](enum.Invalidation.html "enum pert_ui::Invalidation")\>

Applies the new binding, style, permissions, normalizer and callbacks; a
different binding ends the edit in progress unsaved, as described on the
type.

[Source](../src/pert_ui_runtime/text_fields.rs.html#858)[§](#method.focusable)

#### fn [focusable](trait.Component.html#method.focusable)(&self) -\> [bool](https://doc.rust-lang.org/1.99.1/std/primitive.bool.html)

Whether automatic keyboard traversal and pointer presses may focus this
component. Default is false; explicit low-level `Runtime::focus` remains
available. [Read more](trait.Component.html#method.focusable)

[Source](../src/pert_ui_runtime/text_fields.rs.html#858)[§](#method.conceals_input)

#### fn [conceals_input](trait.Component.html#method.conceals_input)(&self) -\> [bool](https://doc.rust-lang.org/1.99.1/std/primitive.bool.html)

Whether this component holds secret input, such as a password, that
diagnostics must not record. [Read
more](trait.Component.html#method.conceals_input)

[Source](../src/pert_ui_runtime/text_fields.rs.html#858)[§](#method.cursor)

#### fn [cursor](trait.Component.html#method.cursor)(&self, point: [Point](struct.Point.html "struct pert_ui::Point")) -\> [Option](https://doc.rust-lang.org/1.99.1/core/option/enum.Option.html "enum core::option::Option")\<[CursorIcon](enum.CursorIcon.html "enum pert_ui::CursorIcon")\>

Returns a cursor for a point in local logical coordinates, or `None` to
defer to ancestors. Called read-only on the UI thread after layout; no
writes or I/O are allowed. Captured components may return a resize
cursor outside their bounds until the gesture ends. Default defers.

[Source](../src/pert_ui_runtime/text_fields.rs.html#858)[§](#method.text_input_state)

#### fn [text_input_state](trait.Component.html#method.text_input_state)(&self) -\> [Option](https://doc.rust-lang.org/1.99.1/core/option/enum.Option.html "enum core::option::Option")\<[TextInputState](struct.TextInputState.html "struct pert_ui::TextInputState")\>

Returns a local IME candidate anchor for an editable text recipient.
Default disables native composition. Called read-only after
layout/paint; implementations must use their current caret and never
mutate bindings. Advance
[`TextInputState::session`](struct.TextInputState.html#structfield.session "field pert_ui::TextInputState::session")
when canceling composition, replacing bound text or repositioning its
caret during composition. Never reuse a session; return `None` on
exhaustion.

[Source](../src/pert_ui_runtime/text_fields.rs.html#858)[§](#method.update)

#### fn [update](trait.Component.html#tymethod.update)(&mut self, cx: &mut [UpdateContext](struct.UpdateContext.html "struct pert_ui::UpdateContext")) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[()](https://doc.rust-lang.org/1.99.1/std/primitive.unit.html), [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\>

Refreshes retained data according to the context’s invalidation. [Read
more](trait.Component.html#tymethod.update)

[Source](../src/pert_ui_runtime/text_fields.rs.html#858)[§](#method.measure)

#### fn [measure](trait.Component.html#method.measure)( &self, cx: &[MeasureContext](struct.MeasureContext.html "struct pert_ui::MeasureContext")\<'\_\>, available: [Constraints](struct.Constraints.html "struct pert_ui::Constraints"), ) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[Size](struct.Size.html "struct pert_ui::Size"), [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\>

Measures content in logical pixels under the available space. [Read
more](trait.Component.html#method.measure)

[Source](../src/pert_ui_runtime/text_fields.rs.html#858)[§](#method.take_children)

#### fn [take_children](trait.Component.html#method.take_children)(&mut self) -\> [Vec](https://doc.rust-lang.org/1.99.1/alloc/vec/struct.Vec.html "struct alloc::vec::Vec")\<[Element](struct.Element.html "struct pert_ui::Element")\>

Transfers initial child descriptions into the common retained tree.
[Read more](trait.Component.html#method.take_children)

[Source](../src/pert_ui_runtime/text_fields.rs.html#858)[§](#method.child_visible)

#### fn [child_visible](trait.Component.html#method.child_visible)(&self, key: &[str](https://doc.rust-lang.org/1.99.1/std/primitive.str.html)) -\> [bool](https://doc.rust-lang.org/1.99.1/std/primitive.bool.html)

Controls presentation of an immediate child by its stable sibling key.
Hidden descendants retain state and suspend work. Defaults to true. Read
bindings during update, then return retained state here; changes require
layout invalidation. This combines with explicit runtime/ancestor
visibility.

[Source](../src/pert_ui_runtime/text_fields.rs.html#858)[§](#method.child_surface)

#### fn [child_surface](trait.Component.html#method.child_surface)(&self, key: &[str](https://doc.rust-lang.org/1.99.1/std/primitive.str.html)) -\> [Option](https://doc.rust-lang.org/1.99.1/core/option/enum.Option.html "enum core::option::Option")\<[SurfaceKind](enum.SurfaceKind.html "enum pert_ui::SurfaceKind")\>

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

[Source](../src/pert_ui_runtime/text_fields.rs.html#858)[§](#method.dismiss_surface)

#### fn [dismiss_surface](trait.Component.html#method.dismiss_surface)( &mut self, \_: &mut [EventContext](struct.EventContext.html "struct pert_ui::EventContext"), \_: [SurfaceDismissal](enum.SurfaceDismissal.html "enum pert_ui::SurfaceDismissal"), ) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[()](https://doc.rust-lang.org/1.99.1/std/primitive.unit.html), [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\>

Closes this owner’s surface in response to shared routing/lifecycle
rules. Must hide its surface child on the next update and release local
actions, except modal owners may ignore Escape when explicit response is
required and ignore outside press, focus leaving and native focus loss.
Ownership removal/hiding and admission failure must always terminate
modal sessions. Do not activate application commands here. [Read
more](trait.Component.html#method.dismiss_surface)

[Source](../src/pert_ui_runtime/text_fields.rs.html#858)[§](#method.arrange)

#### fn [arrange](trait.Component.html#method.arrange)( &self, cx: &mut [ArrangeContext](struct.ArrangeContext.html "struct pert_ui::ArrangeContext")\<'\_\>, size: [Size](struct.Size.html "struct pert_ui::Size"), ) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[()](https://doc.rust-lang.org/1.99.1/std/primitive.unit.html), [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\>

Places every visible child once, relative to this component’s origin.
[Read more](trait.Component.html#method.arrange)

[Source](../src/pert_ui_runtime/text_fields.rs.html#858)[§](#method.paint)

#### fn [paint](trait.Component.html#method.paint)(&self, p: &mut [Painter](struct.Painter.html "struct pert_ui::Painter")) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[()](https://doc.rust-lang.org/1.99.1/std/primitive.unit.html), [DrawError](enum.DrawError.html "enum pert_ui::DrawError")\>

Records backend-independent drawing from retained data after an update.
[Read more](trait.Component.html#method.paint)

[Source](../src/pert_ui_runtime/text_fields.rs.html#858)[§](#method.hover_changed)

#### fn [hover_changed](trait.Component.html#method.hover_changed)(&mut self, cx: &mut [EventContext](struct.EventContext.html "struct pert_ui::EventContext"), hovered: [bool](https://doc.rust-lang.org/1.99.1/std/primitive.bool.html))

Reports when the frontmost pointer enters/leaves this logical subtree,
including native focus loss. Default does nothing. Called at the
runtime’s delivered event time, independently of pressed-button gestures
and pointer bubbling; use it for local hover timing, not host actions or
I/O. Hiding and removal cancel context through `cancel_input` instead.

[Source](../src/pert_ui_runtime/text_fields.rs.html#858)[§](#method.text_key_event)

#### fn [text_key_event](trait.Component.html#method.text_key_event)( &mut self, cx: &mut [EventContext](struct.EventContext.html "struct pert_ui::EventContext"), input: [CommandKeyInput](struct.CommandKeyInput.html "struct pert_ui::CommandKeyInput"), transition: [KeyTransition](enum.KeyTransition.html "enum pert_ui::KeyTransition"), \_: &[KeyboardState](struct.KeyboardState.html "struct pert_ui::KeyboardState"), ) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[TextKeyOutcome](enum.TextKeyOutcome.html "enum pert_ui::TextKeyOutcome"), [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\>

Optional editing phase before capture bindings and ordinary bubbling.
Custom editors return InsertText for printable insertion, Handled for
editing chords (including disabled local undo), and Ignored to continue
from their parent without dispatching the editing action twice. [Read
more](trait.Component.html#method.text_key_event)

[Source](../src/pert_ui_runtime/text_fields.rs.html#858)[§](#method.keyboard_event)

#### fn [keyboard_event](trait.Component.html#method.keyboard_event)( &mut self, cx: &mut [EventContext](struct.EventContext.html "struct pert_ui::EventContext"), e: [KeyEvent](struct.KeyEvent.html "struct pert_ui::KeyEvent"), keys: &[KeyboardState](struct.KeyboardState.html "struct pert_ui::KeyboardState"), ) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[InputOutcome](enum.InputOutcome.html "enum pert_ui::InputOutcome"), [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\>

Handles keyboard input with explicit propagation. [Read
more](trait.Component.html#method.keyboard_event)

[Source](../src/pert_ui_runtime/text_fields.rs.html#858)[§](#method.text_command)

#### fn [text_command](trait.Component.html#method.text_command)( &mut self, cx: &mut [EventContext](struct.EventContext.html "struct pert_ui::EventContext"), command: [TextCommand](enum.TextCommand.html "enum pert_ui::TextCommand"), ) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[TextCommandOutcome](enum.TextCommandOutcome.html "enum pert_ui::TextCommandOutcome"), [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\>

Performs a text command on this component’s own text: a clipboard or
selection action on its selection and caret, or beginning an edit. [Read
more](trait.Component.html#method.text_command)

[Source](../src/pert_ui_runtime/text_fields.rs.html#858)[§](#method.text_input)

#### fn [text_input](trait.Component.html#method.text_input)( &mut self, cx: &mut [EventContext](struct.EventContext.html "struct pert_ui::EventContext"), text: &[str](https://doc.rust-lang.org/1.99.1/std/primitive.str.html), ) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[()](https://doc.rust-lang.org/1.99.1/std/primitive.unit.html), [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\>

Receives committed UTF-8 text independently of physical keys and
repeats. [Read more](trait.Component.html#method.text_input)

[Source](../src/pert_ui_runtime/text_fields.rs.html#858)[§](#method.text_preedit)

#### fn [text_preedit](trait.Component.html#method.text_preedit)( &mut self, cx: &mut [EventContext](struct.EventContext.html "struct pert_ui::EventContext"), text: &[TextPreedit](struct.TextPreedit.html "struct pert_ui::TextPreedit"), ) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[()](https://doc.rust-lang.org/1.99.1/std/primitive.unit.html), [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\>

Receives temporary composition without committing it to the text
binding. Empty preedit cancels composition. Clear temporary state on
context loss. [Read more](trait.Component.html#method.text_preedit)

[Source](../src/pert_ui_runtime/text_fields.rs.html#858)[§](#method.pointer_input)

#### fn [pointer_input](trait.Component.html#method.pointer_input)( &mut self, cx: &mut [EventContext](struct.EventContext.html "struct pert_ui::EventContext"), e: [PointerEvent](struct.PointerEvent.html "struct pert_ui::PointerEvent"), ) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[InputOutcome](enum.InputOutcome.html "enum pert_ui::InputOutcome"), [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\>

Handles a pointer event, bubbling ignored events through logical
ancestors. Captured events keep their owner even outside its clipped
bounds. [Read more](trait.Component.html#method.pointer_input)

[Source](../src/pert_ui_runtime/text_fields.rs.html#858)[§](#method.deliver_notifications)

#### fn [deliver_notifications](trait.Component.html#method.deliver_notifications)( &mut self, cx: &mut [EventContext](struct.EventContext.html "struct pert_ui::EventContext"), ) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[()](https://doc.rust-lang.org/1.99.1/std/primitive.unit.html), [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\>

Delivers the change notifications this component queued, after it asked
for this call through a
[`Notifier`](struct.Notifier.html "struct pert_ui::Notifier"). [Read
more](trait.Component.html#method.deliver_notifications)

[Source](../src/pert_ui_runtime/text_fields.rs.html#858)[§](#method.focus_lost)

#### fn [focus_lost](trait.Component.html#method.focus_lost)(&mut self, cx: &mut [EventContext](struct.EventContext.html "struct pert_ui::EventContext")) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[()](https://doc.rust-lang.org/1.99.1/std/primitive.unit.html), [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\>

Ends focus on an in-window transfer or window focus loss, after the
runtime has released this component’s pointer capture, edit sessions and
deadlines. A transfer into this component’s own subtree, such as into an
embedded
[`DraftEditor`](struct.DraftEditor.html "struct pert_ui::DraftEditor"),
keeps them: the component still owns the interaction it started. [Read
more](trait.Component.html#method.focus_lost)

[Source](../src/pert_ui_runtime/text_fields.rs.html#858)[§](#method.tab_navigation)

#### fn [tab_navigation](trait.Component.html#method.tab_navigation)( &mut self, cx: &mut [EventContext](struct.EventContext.html "struct pert_ui::EventContext"), direction: [FocusDirection](enum.FocusDirection.html "enum pert_ui::FocusDirection"), transition: [KeyTransition](enum.KeyTransition.html "enum pert_ui::KeyTransition"), ) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[InputOutcome](enum.InputOutcome.html "enum pert_ui::InputOutcome"), [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\>

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

[Source](../src/pert_ui_runtime/text_fields.rs.html#858)[§](#method.visibility_changed)

#### fn [visibility_changed](trait.Component.html#method.visibility_changed)(&mut self, visible: [bool](https://doc.rust-lang.org/1.99.1/std/primitive.bool.html))

Called on a visibility transition, after hidden editing has been
cancelled.

[Source](../src/pert_ui_runtime/text_fields.rs.html#858)[§](#method.cancel_input)

#### fn [cancel_input](trait.Component.html#method.cancel_input)(&mut self, cx: &mut [EventContext](struct.EventContext.html "struct pert_ui::EventContext"))

Cancels component-owned input with access to its invalidation context.
[Read more](trait.Component.html#method.cancel_input)

[Source](../src/pert_ui_runtime/text_fields.rs.html#858)[§](#method.input_reset)

#### fn [input_reset](trait.Component.html#method.input_reset)(&mut self)

Releases component-owned held-key actions when its input context is
lost. [Read more](trait.Component.html#method.input_reset)

[Source](../src/pert_ui_runtime/text_fields.rs.html#858)[§](#method.inspect)

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

[Source](../src/pert_ui_runtime/runtime.rs.html#1529)[§](#method.focus_siblings_on_removal)

#### fn [focus_siblings_on_removal](trait.Component.html#method.focus_siblings_on_removal)(&self) -\> [bool](https://doc.rust-lang.org/1.99.1/std/primitive.bool.html)

Whether focus passes to a sibling of the same type when this component
is removed while it holds focus. [Read
more](trait.Component.html#method.focus_siblings_on_removal)

[Source](../src/pert_ui_runtime/runtime.rs.html#1541)[§](#method.receives_unfocused_keys)

#### fn [receives_unfocused_keys](trait.Component.html#method.receives_unfocused_keys)(&self) -\> [bool](https://doc.rust-lang.org/1.99.1/std/primitive.bool.html)

Whether this component receives keys while nothing in its window has
focus. [Read more](trait.Component.html#method.receives_unfocused_keys)

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

[Source](../src/pert_ui_runtime/runtime.rs.html#2507)[§](#method.focus_gained)

#### fn [focus_gained](trait.Component.html#method.focus_gained)(&mut self, \_cx: &mut [EventContext](struct.EventContext.html "struct pert_ui::EventContext"))

Receives focus after the runtime has installed this recipient, at the
event’s elapsed time. Default does nothing. Use this infallible
UI-thread lifecycle notification for local input/timing state; do not
perform host actions or I/O. Physical held keys are not replayed.
Fallible state work belongs in a subsequent update or explicit input
callback.

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

[Source](../src/pert_ui_runtime/runtime.rs.html#2657)[§](#method.timer_elapsed)

#### fn [timer_elapsed](trait.Component.html#method.timer_elapsed)(&mut self, \_cx: &mut [EventContext](struct.EventContext.html "struct pert_ui::EventContext")) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[()](https://doc.rust-lang.org/1.99.1/std/primitive.unit.html), [RuntimeError](enum.RuntimeError.html "enum pert_ui::RuntimeError")\>

Runs a timed action whose deadline, set with
[`EventContext::set_timer`](struct.EventContext.html#method.set_timer "method pert_ui::EventContext::set_timer"),
has elapsed. [Read more](trait.Component.html#method.timer_elapsed)

[Source](../src/pert_ui_runtime/runtime.rs.html#2673)[§](#method.unmounted)

#### fn [unmounted](trait.Component.html#method.unmounted)(&mut self)

Called once after subscriptions, deadlines and edit ownership are
released.

[Source](../src/pert_ui_runtime/text_fields.rs.html#266)[§](#impl-Debug-for-TextArea)

### impl [Debug](https://doc.rust-lang.org/1.99.1/core/fmt/trait.Debug.html "trait core::fmt::Debug") for [TextArea](struct.TextArea.html "struct pert_ui::TextArea")

Shows only the type. A description’s content, bindings and callbacks are
not formatted; where the description is mounted as an
[`Element`](struct.Element.html "struct pert_ui::Element"), that
element’s `Debug` shows its key.

[Source](../src/pert_ui_runtime/text_fields.rs.html#266)[§](#method.fmt)

#### fn [fmt](https://doc.rust-lang.org/1.99.1/core/fmt/trait.Debug.html#tymethod.fmt)(&self, f: &mut [Formatter](https://doc.rust-lang.org/1.99.1/core/fmt/struct.Formatter.html "struct core::fmt::Formatter")\<'\_\>) -\> [Result](https://doc.rust-lang.org/1.99.1/core/result/enum.Result.html "enum core::result::Result")\<[()](https://doc.rust-lang.org/1.99.1/std/primitive.unit.html), [Error](https://doc.rust-lang.org/1.99.1/core/fmt/struct.Error.html "struct core::fmt::Error")\>

Formats the value using the given formatter. [Read
more](https://doc.rust-lang.org/1.99.1/core/fmt/trait.Debug.html#tymethod.fmt)

## Auto Trait Implementations[§](#synthetic-implementations)

[§](#impl-Freeze-for-TextArea)

### impl \![Freeze](https://doc.rust-lang.org/1.99.1/core/marker/trait.Freeze.html "trait core::marker::Freeze") for [TextArea](struct.TextArea.html "struct pert_ui::TextArea")

[§](#impl-RefUnwindSafe-for-TextArea)

### impl \![RefUnwindSafe](https://doc.rust-lang.org/1.99.1/core/panic/unwind_safe/trait.RefUnwindSafe.html "trait core::panic::unwind_safe::RefUnwindSafe") for [TextArea](struct.TextArea.html "struct pert_ui::TextArea")

[§](#impl-Send-for-TextArea)

### impl \![Send](https://doc.rust-lang.org/1.99.1/core/marker/trait.Send.html "trait core::marker::Send") for [TextArea](struct.TextArea.html "struct pert_ui::TextArea")

[§](#impl-Sync-for-TextArea)

### impl \![Sync](https://doc.rust-lang.org/1.99.1/core/marker/trait.Sync.html "trait core::marker::Sync") for [TextArea](struct.TextArea.html "struct pert_ui::TextArea")

[§](#impl-UnwindSafe-for-TextArea)

### impl \![UnwindSafe](https://doc.rust-lang.org/1.99.1/core/panic/unwind_safe/trait.UnwindSafe.html "trait core::panic::unwind_safe::UnwindSafe") for [TextArea](struct.TextArea.html "struct pert_ui::TextArea")

[§](#impl-Unpin-for-TextArea)

### impl [Unpin](https://doc.rust-lang.org/1.99.1/core/marker/trait.Unpin.html "trait core::marker::Unpin") for [TextArea](struct.TextArea.html "struct pert_ui::TextArea")

[§](#impl-UnsafeUnpin-for-TextArea)

### impl [UnsafeUnpin](https://doc.rust-lang.org/1.99.1/core/marker/trait.UnsafeUnpin.html "trait core::marker::UnsafeUnpin") for [TextArea](struct.TextArea.html "struct pert_ui::TextArea")

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
