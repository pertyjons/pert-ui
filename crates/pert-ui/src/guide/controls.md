Standard controls bind to signals for UI-local values, and most also bind to a
parameter with `for_parameter` when the application owns the value. Every
control keeps its own gesture: a drag, a key hold or a text draft previews
locally, and Escape, focus loss, hiding or unmounting cancels it.

# Text and actions

Default labels wrap to allocated width. Buttons supply themed padding, a default Standard 64×32 logical
minimum, visible focus, pointer capture and matched-release Space/Enter activation.
Ignored input bubbles through logical ancestors; Tab traverses visible focus targets.
Escape/context loss cancels activation, including capture owned by nonfocusable
custom components. Native repeats remain repeats after window focus loss.

```rust
use pert_ui::{Button, Column, Element, Label, Runtime, RuntimeError, Viewport};
use std::time::Duration;
let mut ui = Runtime::new();
ui.set_viewport(Viewport::new(400.0, 200.0, 1.0)?)?;
let status = ui.signal("Ready".to_owned());
let action = status.clone();
ui.mount_tree(Element::new("content", Column::new([
    Element::new("status", Label::new(status.read_only())),
    Element::new("apply", Button::new("Apply", move |_| action.set("Applied".to_owned()).map(|_| ()))),
])))?;
ui.flush(Duration::ZERO)?;
ui.focus_next(false)?;
ui.flush(Duration::ZERO)?;
let scene = ui.paint()?;
assert!(!scene.masks().is_empty());
# Ok::<(), Box<dyn std::error::Error>>(())
```

Caption-bearing controls take their text as a [`TextSource`]: a `&str` or
`String`, a [`RichText`], or a `ReadSignal<String>` or `ReadSignal<RichText>`
the control follows. Label, Button, Link, Chip, Tooltip, Menu, MenuItem,
Checkbox, Toggle and InlineToggle names accept it through the same constructor
argument, so a translated or changing caption needs no second builder. A
binding is read in the control's update and remeasures it on change.

Explicit leading/trailing icons on Label, Button and Link survive caption
changes. Repeating a builder replaces that slot; omit it on redescription to
clear it. Chip and MenuItem offer one leading icon. Icons inside rich caption
content belong to the caption and change with it; keep persistent adornments in
the explicit slots. An icon button's text source is its semantic name and does
not replace its central icon.

```rust
use pert_ui::{Button, Element, Icon, Runtime};
use std::time::Duration;
let mut ui = Runtime::new();
let caption = ui.signal("Save".to_owned());
ui.mount_tree(Element::new("save",
    Button::new(caption.read_only(), |_| Ok(())).leading_icon(Icon::Save)))?;
ui.flush(Duration::ZERO)?;
caption.set("Spara".to_owned())?;
ui.flush(Duration::ZERO)?; // the Save icon remains
# Ok::<(), Box<dyn std::error::Error>>(())
```

For multiple icons, compose a `RichText` and pass it as the text. Repeated
slot builders do not accumulate icons.

Parley shapes the bundled OFL-licensed Noto Sans font and Swash prepares coverage
on the UI thread. No system font discovery, color emoji or complete script coverage
is promised; unsupported characters use the font's missing-glyph mark. Body,
heading and caption presets default to 14/18/12 logical pixels, overridable
through theme metrics. Font assets and attribution
ship in the runtime package. Text admission limits and bounded cache behavior are
documented on [`TextError`] and [`MeasureContext::text`]; live owners retain resources
independently of service cache eviction. TextField/TextArea and the numeric inputs
provide editing/IME over the same text service.

# Boolean and discrete controls

Checkbox, Toggle, InlineToggle and single-choice controls offer `on_change`: it
runs after a successful user write or remote proposal submission. It does not
report application writes or authority responses. Errors propagate without
rollback or replay; redescription replaces the handler. Select closes first. Its
callback receives the list context, so use `EventContext::defer_detached` for work
that must survive the list hiding. MultiSelect's `on_change` reports each user
write with the values written: a toggle under EachToggle, a chip removal, and an
OnClose draft written by a committing close; discarded and conflicting drafts
report nothing.

A Checkbox or Toggle made with `for_parameter` shows the parameter's name and
follows it when the specification changes. Its `leading_icon` and
`trailing_icon` slots decorate that name without replacing it; `label` replaces
the name with any [`TextSource`], for example a bound translation. An
InlineToggle's `name` replaces its accessible name the same way, and its icon or
glyph is its visible decoration.

```rust
use pert_ui::{Checkbox, Column, Element, Icon, ParameterId, ParameterSpec, Runtime, Toggle};
use std::time::Duration;
let mut ui = Runtime::new();
let (bypass, host) = ui.parameter(
    ParameterId::new(1, 1, "bypass")?,
    ParameterSpec::boolean("Bypass", false)?,
)?;
let translated = ui.signal("Umgehen".to_owned());
ui.mount_tree(Element::new("controls", Column::new([
    // The specification names it; the application adds the icon.
    Element::new("named", Checkbox::for_parameter(bypass.clone()).leading_icon(Icon::Settings)),
    // The application's translation replaces the name.
    Element::new("translated", Toggle::for_parameter(bypass).label(translated.read_only())),
])))?;
ui.flush(Duration::ZERO)?;
host.replace_spec(ParameterSpec::boolean("Bypass all", false)?)?;
ui.flush(Duration::ZERO)?;
let name = |key: &str| ui.inspect().components.into_iter()
    .find(|c| c.key == key).and_then(|c| c.details.semantics).and_then(|s| s.name);
assert_eq!(name("named").as_deref(), Some("Bypass all"));
assert_eq!(name("translated").as_deref(), Some("Umgehen"));
# Ok::<(), Box<dyn std::error::Error>>(())
```

Choice leading/trailing icons and MenuItem leading icons are replaceable slots
that survive `rich_label` and `label` changes. MenuItem `enabled` combines Permission
restrictions, including command availability; `enabled_signal` is an alias.


Choice groups and dropdowns accept `.enabled(permission)` with the same restrictive
composition as basic controls. Permissions never override a parameter's editability
or a disabled option. Revocation cancels pending input at update; an open dropdown
stays open but refuses choices and remains dismissible. MultiSelect checks permission
again when closing an `OnClose` draft: closing while denied discards it; reenabling
before closing permits the usual conflict-checked commit. Its retained rows and
chips observe permissions during a successful flush, so intermediate chip descriptions
replaced before update do not individually reset input.

```rust
use pert_ui::{Choice, Column, Element, Runtime, Select};
use std::time::Duration;
let mut ui = Runtime::new();
let selected = ui.signal("one");
let editable = ui.signal(true);
let select = Select::new(&ui, selected, [Choice::new("one", "One")])?
    .enabled(editable.read_only());
ui.mount_tree(Element::new("form", Column::new([Element::new("choice", select)])))?;
ui.flush(Duration::ZERO)?;
editable.set(false)?;
ui.flush(Duration::ZERO)?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

Single-choice groups and dropdowns choose one option source at construction:
`new` validates a fixed list immediately, `bound` follows a read binding and
validates during update, and `for_parameter` uses the parameter specification.
MultiSelect provides fixed and bound list constructors. For dropdowns and
MultiSelect, keep the existing `StateOwner` argument before the value and list.

Migrate `RadioGroup::new(value, [])?.choices(options)` to
`RadioGroup::bound(value, options)`, and
`Select::new(&ui, value, [])?.choices(options)` to
`Select::bound(&ui, value, options)`. The same replacement applies to the inline,
segmented and multi-select variants. There is no separate `choices` builder or
`ChoiceError::ConflictingSource`: a parameter-bound control cannot accept an
independent list. Dynamic bindings must share the runtime; duplicate lists or
foreign bindings fail update, and correcting the source permits recovery.

```rust
use pert_ui::{Choice, Element, RadioGroup, Runtime};
use std::time::Duration;
let mut ui = Runtime::new();
let selected = ui.signal("one");
let options = ui.signal(vec![Choice::new("one", "First")]);
ui.mount_tree(Element::new("options", RadioGroup::bound(selected.clone(), options.read_only())))?;
ui.flush(Duration::ZERO)?;
options.set(vec![Choice::new("two", "Second"), Choice::new("one", "Renamed")])?;
ui.flush(Duration::ZERO)?;
assert_eq!(selected.get()?, "one"); // labels and order are not identity
# Ok::<(), Box<dyn std::error::Error>>(())
```


Basic controls and choice controls accept a fixed or reactive
`Permission` through `enabled`. Repeated restrictions combine: every condition
must allow, and `disabled()` cannot be lifted by a later call on the same
description. Re-describing the component replaces its permission. Parameter
editability remains an additional restriction.

```rust
use pert_ui::prelude::*;
use std::time::Duration;
let mut ui = Runtime::new();
ui.set_viewport(Viewport::new(320.0, 100.0, 1.0)?)?;
let editable = ui.signal(true);
let selected = ui.signal(false);
let control = ui.mount_tree(Element::new("selected",
    Checkbox::new("Selected", selected).enabled(editable.read_only())))?;
ui.focus(&control)?;
ui.key_input(KeyCode::Space, KeyAction::Press)?;
editable.set(false)?;
ui.flush(Duration::ZERO)?; // Cancels the armed press and releases focus.
assert_ne!(ui.focused(), Some(control));
# Ok::<(), Box<dyn std::error::Error>>(())
```

[`Checkbox`] and [`Toggle`] share UI-local boolean values. [`RadioGroup`] supplies radio
buttons with one keyboard focus stop and typed [`Choice`] values. Arrow navigation
skips disabled options. Labels and ordering do not identify an option; duplicate
values are rejected. An absent external selection displays no checked option.
Dynamic option changes cancel pending activation, including remove/recreate.
These constructors bind local signals. [`Checkbox::for_parameter`],
[`Toggle::for_parameter`] and [`RadioGroup::for_parameter`] bind a boolean or choice
parameter instead, so each activation is a committed host edit; see
[parameter families](crate::guide::state#parameter-families-and-host-notifications).

```rust
use pert_ui::prelude::*;
let mut ui = Runtime::new();
let enabled = ui.signal(true);
let quality = ui.signal("balanced");
ui.mount_tree(Element::new("settings", Column::new([
    Element::new("checkbox", Checkbox::new("Enabled", enabled.clone())),
    Element::new("switch", Toggle::new("Enabled", enabled)),
    Element::new("quality", RadioGroup::new(quality, [
        Choice::new("balanced", "Balanced"),
        Choice::new("precise", "Precise"),
    ])?),
])))?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

# Sliders

[`Slider`] and [`InlineSlider`] share the numeric edit protocol. A held pointer/key
gesture previews until its matching release; Escape and context loss cancel. Shift
gives tenfold finer adjustments, Home/End reach limits, and Backspace restores the
parameter default. External base writes win over pending edits. Admission errors
remain visible in the error outline and inspection; effective telemetry is never
used as an editing baseline. Horizontal sliders drag left/right; vertical sliders
drag up/down. While focused, a vertical wheel event over the control
commits one 1% normalized step, or 0.1% with Shift, clamped to endpoints. Upward
scroll increases; magnitude is not a multiplier. Wheel input preserves an active
pointer/key gesture without stepping. Slider readouts, unfocused
and noneditable controls preserve normal scrolling.

Sliders have slim thumbs with the button theme's corner radius and optional
`show_value(true)` readouts: right of a horizontal track, centered below a
vertical track.
`decimal_places` defaults to 2. The monospaced readout reserves the larger
formatted endpoint, including a sign and rounding carry, so decimal positions and
track length stay fixed as values change. `format` installs a [`NumberFormat`]
instead, the same type the inline rows and number inputs take: its display and
unit callbacks produce the text, its width hint reserves the slot, and the text
ends at the slot's right edge. Only display matters, since sliders never parse
text; formatting never quantizes the parameter, and an unchanged clone of the
format retains layout on redescription. Metadata/format/theme changes may resize
it. Constrained allocations clip readouts without wrapping. Visible values use
[`Painter::text`] inside the reserved slot, so ordinary edits
repaint without remeasuring controls or ancestors. Only a changed reserved digit
count of the default presentation requests layout through
[`UpdateContext::request_layout`].

`unit` is literal presentation text with no conversion or domain semantics.
Sliders append it to the enabled readout, and inline rows show it after the value.
An empty unit reserves no extra space.

[`Knob`] is the built-in rotary parameter control. A custom rotary control
with another look builds on the same public parameter binding and on
[`Painter::stroke_arc`]/[`Painter::fill_triangle`], which cache coverage
exactly like the built-in geometry, as the example below does.

```rust
use pert_ui::*;
let mut ui = Runtime::new();
let (amount, host) = ui.parameter(
    ParameterId::new(1, 1, "amount")?,
    ParameterSpec::new("Amount", -1.0, 1.0, 0.0)?,
)?;
ui.mount_tree(Element::new("controls", Row::new([
    Element::new("slider", Slider::new(amount.clone()).show_value(true).decimal_places(1)?.unit("%")),
    Element::new("vertical", Slider::new(amount.clone()).orientation(Orientation::Vertical).show_value(true)),
])))?;
// Retain the authoritative owner while these controls should remain available.
# drop(host);
# Ok::<(), Box<dyn std::error::Error>>(())
```

Custom components can use the same `Painter::text` cache for text that fits a
reserved allocation. This ASCII readout grows only when its capacity changes;
ordinary updates retain layout and cached text is shared with measurement:

```rust
use pert_ui::*;
struct Readout { source: ReadSignal<String>, text: String, columns: usize }
impl Component for Readout {
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        self.text = cx.read_paint(&self.source)?;
        if self.text.len() > self.columns {
            self.columns = self.text.len();
            cx.request_layout();
        }
        Ok(())
    }
    fn measure(&self, cx: &MeasureContext<'_>, _: Constraints) -> Result<Size, RuntimeError> {
        Ok(cx.text(&"0".repeat(self.columns), None, TextStyle::MonospaceCaption)?.size())
    }
    fn paint(&self, p: &mut Painter) -> Result<(), DrawError> {
        let text = p.text(&self.text, None, TextStyle::MonospaceCaption)?;
        text.paint(p, 0.0, 0.0, p.color(ColorRole::OnSurface))
    }
}
let mut ui = Runtime::new();
ui.set_viewport(Viewport::new(300.0, 100.0, 1.0)?)?;
let value = ui.signal("0.00".to_owned());
ui.mount_tree(Element::new("readout", Readout {
    source: value.read_only(), text: String::new(), columns: 6,
}))?;
ui.flush(std::time::Duration::ZERO)?;
ui.paint()?;
value.set("12.00".to_owned())?; // fits: update and paint, no layout
ui.flush(std::time::Duration::ZERO)?;
ui.paint()?;
value.set("1234.00".to_owned())?; // grows: request_layout coalesces in this flush
ui.flush(std::time::Duration::ZERO)?;
ui.paint()?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

# Text fields

[`TextField`] edits one line using body typography. [`TextArea`] defaults to bundled
monospace and supports line breaks, soft wrapping and scrolling. Both update local
text bindings immediately, with selection, clipboard shortcuts and bounded undo.
Double-click selects a word; triple-click selects all text. Enter submits a
single-line field through its optional callback; it inserts LF in multiline.
While IME preedit is active, the IME owns confirmation keys. Escape/focus loss
discard preedit, retaining committed text; external binding writes win.

```no_run
# #[cfg(feature = "desktop")]
fn main() -> Result<(), pert_ui::AppError> {
    use pert_ui::{App, Column, Element, Label, TextArea, TextField};
    App::new("Text editing", |ui| {
        let name = ui.signal("Example".to_owned());
        let notes = ui.signal("First line\nSecond line".to_owned());
        let submitted = ui.signal(String::new());
        let output = submitted.clone();
        ui.mount_tree(Element::new("form", Column::new([
            Element::new("name", TextField::new(name).on_submit(move |_, text| {
                output.set(format!("Submitted: {text}")).map(|_| ())
            })),
            Element::new("notes", TextArea::new(notes.clone())),
            Element::new("mirror", TextArea::new(notes).read_only()),
            Element::new("result", Label::new(submitted.read_only())),
        ])))?;
        Ok(())
    }).run()
}
# #[cfg(not(feature = "desktop"))]
# fn main() {}
```

Bindings accept up to 16 KiB UTF-8, subject to the shared text preparation limits.
Single-line paste replaces line breaks and tabs with spaces; multiline paste
normalizes CRLF, CR, NEL, vertical tab, form feed and the Unicode separators to
LF, and other pasted control characters are dropped. An external binding value
the editor cannot accept is refused without rewriting the source: the field keeps
its previous text, shows the error outline and accepts no edits until the
binding is acceptable again. [`TextField::check`] and [`TextArea::check`] test a
string before it is written. Clipboard errors propagate; failed cut preserves its
source text.
On Linux the desktop clipboard tries the Wayland data-control protocol first and
falls back to X11, which reaches GNOME's clipboard through XWayland; unsupported
operations return typed errors. Clipboard reads may
allocate the full OS payload on Windows/macOS; the editor limit bounds accepted
text, not all native service allocations. These are ordinary UI-thread operations.

Custom editors use [`EventContext::text`], [`TextSelection`] and [`TextMovement`] for
hit testing and selection geometry, [`Component::text_preedit`] for composition and
[`Component::text_input_state`] for the IME anchor/session. The native adapter resets
composition when the owner or session changes. Headless hosts can install their
own [`Clipboard`] service through [`Runtime::set_clipboard`].

Escape in a text field cancels composition first, or collapses a selection and
ends dragging. With nothing to cancel it bubbles to ancestors. Numeric fields
retain their separate Escape contract: restore the saved value and consume the key.

# Numeric text entry

[`IntegerInput`] binds exact `i64` values; [`DecimalInput`] binds finite `f64` values.
Text stays local until Enter or focus loss saves a valid draft; Escape restores
its saved value. Hiding/removal cancels. Invalid text remains visible with an error
outline and never overwrites the number. External changed values win over drafts.
Ranges and keyboard steps are optional; Up/Down edits the draft. Decimal display
precision does not round the stored value, and both `.` and `,` are accepted as
decimal separators (no grouping). These are local signals, not host edit sessions;
[`IntegerInput::for_parameter`] edits a stepped parameter instead, validating drafts
exactly against its step and cancelling them on focus loss.

```rust
use pert_ui::{Column, DecimalInput, Element, IntegerInput, KeyAction, KeyCode, Runtime};
let mut ui = Runtime::new();
let count = ui.signal(12_i64);
let amount = ui.signal(1.234567_f64);
let root = ui.mount_tree(Element::new("numbers", Column::new([
    Element::new("count", IntegerInput::new(count.clone()).range(0, 100)?),
    Element::new("amount", DecimalInput::new(amount.clone()).decimal_places(2)?),
])))?;
let fields = ui.children(&root)?;
ui.focus(&fields[0])?;
ui.key_input(KeyCode::ArrowUp, KeyAction::Press)?;
ui.key_input(KeyCode::ArrowUp, KeyAction::Release)?;
assert_eq!(count.get()?, 12); // draft only
ui.focus(&fields[1])?; // saves count
assert_eq!(count.get()?, 13);
ui.clear_focus(); // unchanged rounded display preserves the full amount
assert_eq!(amount.get()?, 1.234567);
# Ok::<(), pert_ui::RuntimeError>(())
```

# Numeric wheel steps

Numeric Up/Down and wheel steps over the focused field display configured decimal
precision while retaining the exact numeric draft through undo/redo. Free typing
remains unrestricted. Shift divides arrow/wheel steps by ten (integer division,
minimum 1 for IntegerInput). A vertical wheel event steps once using the configured step;
up increases and down decreases, clamped to bounds. Enter/focus loss saves and
Escape cancels, as with keyboard steps. Unfocused fields preserve normal scrolling;
read-only, disabled, invalid and composing fields are not changed by wheel steps.

```rust
use pert_ui::{Element, IntegerInput, KeyAction, KeyCode, Point, PointerEvent,
            PointerKind, Runtime, Viewport};
let mut ui = Runtime::new();
ui.set_viewport(Viewport::new(240.0, 80.0, 1.0)?)?;
let value = ui.signal(10_i64);
let field = ui.mount_tree(Element::new("count", IntegerInput::new(value.clone()).step(2)?))?;
ui.focus(&field)?;
ui.pointer_input(PointerEvent::new(Point::new(20.0, 20.0)?,
    PointerKind::Scroll(Point::new(0.0, -40.0)?)))?;
assert_eq!(value.get()?, 10); // wheel changed only the local draft
ui.key_input(KeyCode::Enter, KeyAction::Press)?;
assert_eq!(value.get()?, 12);
# Ok::<(), Box<dyn std::error::Error>>(())
```

# Progress and activity

[`ProgressBar`] presents a [`ProgressStatus`] binding the application writes: idle,
indeterminate, a known fraction, or how the task ended. [`Spinner`] shows that
something is running when nothing is known about how far. The library never
advances or completes a task. Only indeterminate activity animates, and it stops
scheduling when hidden, unmounted or finished, or under reduced motion.

# Joined exclusive buttons

`SegmentedControl<T>` binds one `Signal<T>` to a horizontal row of equal-width
`Choice<T>` buttons. They share straight internal edges, with the same rounded
exterior corners as ordinary buttons. The selected segment uses `TabSelected`
with no underline or baseline beneath the group. Labels clip within constrained
segments.

The group has one Tab stop; arrows wrap past disabled options and Home/End choose
endpoints. Pointer activation requires matching press/release on the same value;
Space/Enter activate on release. Escape/context loss and dynamic option changes
cancel pending input. Absent external selections remain absent without rewriting
application state. Duplicate values are rejected. `SegmentedControl::bound(value,
options)` follows a reactive option list; `.disabled()` prevents editing. Optional `.on_change(...)` runs after a
user change, with the bound value already written even if the callback fails.
External updates and reselecting the same value do not invoke the callback.

```rust
use pert_ui::{Choice, Element, Runtime, SegmentedControl, ThemeMode};
let mut ui = Runtime::new();
let theme = ui.theme();
let mode = ui.signal(theme.mode()?);
ui.mount_tree(Element::new("appearance", SegmentedControl::new(mode, [
    Choice::new(ThemeMode::Dark, "Dark"),
    Choice::new(ThemeMode::Light, "Light"),
])?.on_change(move |_, value| theme.set_mode(*value).map(|_| ()))))?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

# Inline numeric rows

[`InlineSlider`], [`InlineInteger`] and [`InlineDecimal`] are one-line controls with a
leading caption and a trailing value/unit, sized for toolbars, channel strips and
status bars. They are separate types from [`Slider`], [`IntegerInput`] and
[`DecimalInput`], not a mode of them, and they follow the ordinary theme metrics, so
density scales them like every other control. Click the value or press Enter/F2
to type; drag it horizontally to adjust. Focused arrows/wheel step; Shift adjusts
finely. The slider retains its parameter transactions. Local number drags save on
release and resting key/wheel steps save immediately; text drafts save on
Enter/focus loss. Escape cancels. Slider text cancels on focus loss, following
parameter ownership.

```rust
use pert_ui::{Column, Element, InlineDecimal, InlineInteger, InlineSlider,
    ParameterId, ParameterSpec, Runtime};
let mut ui = Runtime::new();
let count = ui.signal(8_i64);
let decimal = ui.signal(0.9_f64);
let (amount, _host) = ui.parameter(ParameterId::new(1, 1, "amount")?,
    ParameterSpec::new("Amount", 0.0, 2.0, 0.9)?)?;
ui.mount_tree(Element::new("controls", Column::new([
    Element::new("slider", InlineSlider::new(amount).label("Glide").unit("s")),
    Element::new("integer", InlineInteger::new(count).range(0, 100)?
        .label("Vol").unit("%")),
    Element::new("decimal", InlineDecimal::new(decimal).range(0.0, 2.0)?
        .decimal_places(2)?.label("Time").unit("s")),
])))?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

`NumberFormat<T>` supplies display text, editing text, parsing and an optional
value-dependent suffix, all in application-defined units. It remains UI-local;
canonical quantities and the parameter mapping retain their own validation.
Its width example reserves the entire value/unit slot, preventing adjacent
controls from moving as values change. Formatting never rounds stored values.

```rust
use pert_ui::{Element, InlineDecimal, NumberFormat, NumberInputError, Runtime};
let mut ui = Runtime::new();
let seconds = ui.signal(0.0009_f64);
let milliseconds = NumberFormat::new("9999.00 ms",
    |v: f64| format!("{:.2}", v * 1000.0),
    |s| s.trim().replace(',', ".").parse::<f64>().map(|v| v / 1000.0)
        .map_err(|_| NumberInputError::InvalidNumber))
    .edit_text(|v| (v * 1000.0).to_string());
ui.mount_tree(Element::new("delay", InlineDecimal::new(seconds)
    .range(0.0, 2.0)?.step(0.0001)?.label("Delay").unit("ms")
    .format(milliseconds)))?;
# Ok::<(), pert_ui::RuntimeError>(())
```

Text mode keeps the unit outside selection and IME content. Read-only local
number inputs still allow selection and copying; disabled inputs do not interact.
Custom format callbacks must be deterministic, bounded and free of side effects.
Their output uses the same text limits as other single-line editors. Longer output
clips to the reserved slot; editing scrolls within it. Neither resting focus nor
hidden controls schedule a caret timer.

# Inline controls and toolbars

The `Inline*` family are one-line controls for toolbars, channel strips, status
bars and browser headers. Each fits a ControlHeight row, follows the ordinary
theme metrics for density and zoom, owns one Tab stop and keeps arrow keys for
itself. Icon-only members carry an accessible name that inspection and delayed
help present. [`InlineToggle`] is a latched square button bound to a `Signal<bool>`;
[`InlineChoice`] is an exclusive selection with square icon segments; [`InlineSelect`]
is a caption, current value and chevron opening the [`Select`] list; [`InlineText`] is
an editable label whose draft saves on Enter or focus loss and cancels on Escape;
[`InlineReadout`] is a read-only caption/value/unit row; the numeric rows above
accept `stepper(true)` for minus/plus buttons. "Compact" refers only to
[`Density::Compact`].

[`Toolbar`] hosts them in a strip with a SurfaceContainer fill and a PanelBorder
line. Groups ([`ToolGroup::new`], whose controls are built and mounted once) move
behind a "More" button when the strip is narrower than its content, starting with
`collapse_first` groups, then from the trailing end and never an `essential`
group; [`ToolbarOverflow::Wrap`] wraps lines instead. A separator follows the group
before it and a spacer pushes later groups to the trailing edge. A collapsing
group is the same retained subtree moved to the overflow surface with
[`UpdateContext::request_reparent`], keeping its state and its position among its
peers; it is hidden while that surface is closed, so an active gesture or draft
in it cancels under the hiding rules.
[`ToolGroup::visible_signal`] binds a group's presence to a `ReadSignal<bool>` for
contextual groups, such as the options of the selected tool: a hidden group
takes no space, never collapses and cannot be focused. Nest placement, not
toolbars: a global strip in a region layout's top slot and a local strip in a
view are siblings.

```rust
use pert_ui::{Button, Choice, Element, Icon, InlineChoice, InlineReadout, InlineSelect,
    InlineText, InlineToggle, Runtime, RuntimeError, ToolGroup, Toolbar};
let mut ui = Runtime::new();
let looping = ui.signal(false);
let snap = ui.signal("1/16");
let tool = ui.signal("select");
let name = ui.signal(String::from("Lead"));
let position = ui.signal(String::from("001:01:000"));
let toolbar = Toolbar::new(&ui, [
    ToolGroup::new("transport", [
        Element::new("play", Button::icon(Icon::PlayArrow, "Play", |_| Ok(())).frameless()),
    ]).essential(),
    ToolGroup::new("loop", [
        Element::new("loop", InlineToggle::new(looping, "Loop").icon(Icon::Repeat)),
    ]),
    ToolGroup::new("snap", [Element::new("snap",
        InlineSelect::new(&ui, snap, [Choice::new("1/4", "1/4"), Choice::new("1/16", "1/16")])
?
            .label("Snap"))]),
    ToolGroup::new("tools", [Element::new("mode",
        InlineChoice::new(tool, [
            Choice::icon("select", Icon::Edit, "Select"),
            Choice::icon("erase", Icon::Delete, "Erase"),
        ])?)]),
    ToolGroup::spacer("spacer"),
    ToolGroup::new("session", [
        Element::new("name", InlineText::new(name).placeholder("Untitled")),
        Element::new("pos", InlineReadout::new(position.read_only()).label("Pos")
            .width_hint("000:00:000")),
    ]).collapse_first(),
])?;
ui.mount_tree(Element::new("toolbar", toolbar))?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

The toolbar owns no commands: buttons carry their own activation and toggles
their own signals, so the host decides what play or loop mean. Overflow is
decided from the final allocation during layout and applied in the same flush
through [`ArrangeContext::request_update`], a public hook custom containers can use
for the same purpose.

Disabling SearchField discards a pending debounced query without publishing it.
Disabling TextField/TextArea retains an unsaved draft but never saves it on focus
loss while disabled; reenabling permits a subsequent save. Read-only selection
and copying remain separate from enablement.
