Transient and modal surfaces draw above ordinary content in the same window,
escape ancestor clipping and return focus when they close. The runtime owns
their layering, hit order and dismissal; the application owns what their actions
mean.

# Select, popovers and tooltips

`Select<T>` shares typed `Choice<T>` values with `RadioGroup<T>`. Opening the list
and navigating it only changes the highlight; Enter/Space release or a matching
primary click confirms the choice. Escape and outside press cancel. Long lists
scroll with the wheel, arrows and Page/Home/End keys. [`Menu`] applies the same
navigation to fixed keyed actions and closes before invoking a selected callback;
[menus](#menus) describes submenus, context menus and menu bars.

[`Popover`] accepts an explicit open-state signal, anchor and arbitrary content.
Its surface escapes ancestor clipping, follows the anchor, flips above when
needed, and scrolls oversized content within the window. One independent nonmodal
chain is open per window; nested popovers retain logical ownership. Outside presses
and their releases are consumed. Escape closes the topmost interactive surface;
focus returns to a surviving opener unless it was intentionally moved elsewhere.
Hiding/removing an owner, a fully clipped anchor or window focus loss closes it.
Inspection reports the logical parent and separate `presentation_layer` and
`anchor_owner` properties on each promoted child.

[`Tooltip`] shows plain text after 500 ms of hover or descendant focus, using one
shared deadline. It never takes focus or intercepts pointer input. Once shown or
cancelled, it schedules no continuing wakeup. Nested menus and modal
dialogs are described below. The library draws its own surfaces and provides no
native OS dialogs.

```rust
use pert_ui::prelude::*;
let mut ui = Runtime::new();
let selection = ui.signal("one");
let status = ui.signal("Ready".to_owned());
let action_status = status.clone();
let open = ui.signal(false);
let show = open.clone();
let close = open.clone();
ui.mount_tree(Element::new("page", Column::new([
    Element::new("select", Select::new(&ui, selection, [
        Choice::new("one", "One"), Choice::new("two", "Two"),
    ])?),
    Element::new("menu", Menu::new(&ui, "Actions", vec![
        MenuItem::new("apply", "Apply", move |_| {
            action_status.set("Applied".to_owned())?;
            Ok(())
        }),
    ])?),
    Element::new("status", Label::new(status.read_only())),
    Element::new("help", Tooltip::new(
        Element::new("anchor", Button::new("Help", |_| Ok(()))),
        "Hover or focus for help.",
    )),
    Element::new("popover", Popover::new(open,
        Element::new("open", Button::new("Open details", move |_| {
            show.set(true)?; Ok(())
        })),
        Element::new("close", Button::new("Close details", move |_| {
            close.set(false)?; Ok(())
        })),
    )),
])))?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

Custom surface owners use [`Component::child_surface`] and `dismiss_surface`, ordinary
child visibility and retained layout. Runtime owns layer placement, hit ordering,
focus and lifecycle dismissal. [`UpdateContext::hovered`], `focus_within` and `now`
provide the same dependency-driven activation and timing used by Tooltip;
[`MeasureContext::viewport`] supplies window constraints. There is no renderer-specific
escape hatch. Dismissal callbacks must hide their surface on the next update;
errors from infallible teardown/focus-loss operations surface on the next flush.

# Menus

[`MenuItem::submenu`] composes keyed nested action lists. Right opens a branch;
Left/Escape returns one level; Enter/Space or a leaf click closes the whole chain
before its callback. Clicking an ancestor menu can directly select another item.
Lists skip disabled entries and scroll. [`ContextMenu`] opens on secondary press
at the pointer, or on the Menu key or Shift+F10 from focused content, and from
application code through a [`ContextMenuController`] with a captured command
context. Inputs consumed by a custom child do not bubble to its surrounding
context menu. A [`Popover`] reports why the framework closed it through
`on_dismiss`; closing it by writing its signal reports nothing.

```rust
use pert_ui::{Viewport, prelude::*};
use std::time::Duration;

let mut ui = Runtime::new();
ui.set_viewport(Viewport::new(640.0, 480.0, 1.0)?)?;
let selected = ui.signal(false);
let write = selected.clone();
ui.mount_tree(Element::new("context", ContextMenu::new(
    &ui,
    Element::new("target", Button::new("Right-click or Shift+F10", |_| Ok(()))),
    vec![MenuItem::submenu("more", "More", vec![
        MenuItem::new("apply", "Apply", move |_| write.set(true)),
        MenuItem::new("unavailable", "Unavailable", |_| Ok(())).disabled(),
    ])],
)?))?;
ui.flush(Duration::ZERO)?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

Menu rows also cover separators, check-mark toggles, radio settings and
shortcuts. [`MenuItem::command`] presents a registered command with its live
shortcut and enabled binding, and runs it with the command context captured
when the menu opened. [`Button::command`] and its icon-only form
[`Button::command_icon`] present a command as a toolbar button with its
enabled binding, capturing the context when it is activated, and
[`Tooltip::command`] shows the command's name and live shortcut as the
button's help. [`KeyChord::named`] binds a single key such as Space. [`MenuBar`] arranges top-level submenus as a desktop menu
bar: titles open on press; actual pointer motion switches an open menu, while
a stationary pointer leaves keyboard navigation in control. F10 focuses the titles,
arrows move between titles and menus, and leaving returns focus to the content.
Give the content to [`MenuBar::content`] so F10 reaches the bar from it. There are
no mnemonics and no native OS menus; see the
[menus design note](https://github.com/pertyjons/pert-ui/blob/main/docs/contracts.md).

A newly opened menu or Select list keeps its initial/selected row under a
stationary pointer. Handled navigation takes the highlight until actual pointer
motion, an admitted press or a wheel that scrolls takes it back. A successfully
armed Enter/Space activation keeps its row through release/cancellation.
Duplicate same-position host motion does not count; after native scale/zoom
changes the first physical report may have different logical coordinates and
count as motion. Menu/Select rustdoc records the full ownership rules.

```rust
use pert_ui::prelude::*;

/// A document under a menu bar whose File › Save runs a command that the
/// primary modifier with S also runs; returns whether Save ran.
fn build(ui: &mut Runtime) -> Result<Signal<bool>, RuntimeError> {
    let saved = ui.signal(false);
    let write = saved.clone();
    let save = ui.register_command("Save", CommandScope::Application,
        vec![KeyChord::primary('s')], move |_, _| write.set(true))?;
    let grid = ui.signal(true);
    let zoom = ui.signal(100u32);
    let bar = MenuBar::new(ui, vec![
        MenuItem::submenu("file", "File", vec![
            MenuItem::command(ui, "save", &save, |_, _| Ok(()))?,
            MenuItem::separator(),
            MenuItem::new("close", "Close", |_| Ok(())),
        ]),
        MenuItem::submenu("view", "View", vec![
            MenuItem::toggle("grid", grid.clone(), "Show grid"),
            MenuItem::separator(),
            MenuItem::radio("normal", zoom.clone(), 100, "100%"),
            MenuItem::radio("large", zoom.clone(), 200, "200%"),
        ]),
    ])?
    .content(Element::new("document", Button::new("Document", |_| Ok(()))));
    ui.mount_tree(Element::new("window", Column::new([Element::new("bar", bar)])))?;
    Ok(saved)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    use pert_ui::testing::*;
    use std::time::Duration;

    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(640.0, 480.0, 1.0)?)?;
    let saved = build(&mut ui)?;
    ui.flush(Duration::ZERO)?;

    // Focus the document, then F10, Down and Enter choose File › Save.
    ui.focus_next(FocusDirection::Forward)?;
    for key in [KeyCode::F10, KeyCode::ArrowDown, KeyCode::Enter] {
        ui.key_input(key, KeyAction::Press)?;
        ui.key_input(key, KeyAction::Release)?;
        ui.flush(Duration::ZERO)?;
    }
    ui.flush(Duration::ZERO)?;
    assert!(saved.get()?);
    assert_eq!(ui.focused().map(|id| ui.key(&id).map(str::to_owned)).transpose()?.as_deref(),
        Some("document"));
    Ok(())
}
```

A toggle or radio row reports a user's choice through [`MenuItem::on_change`],
after its own write, so the application can forward it, for example to a
producer thread; the application's own writes to the signal report nothing.
A setting the application applies itself, such as the theme's density through
a retained [`ThemeManager`], has no signal for the row to write: build the row with
[`MenuItem::new`] and show its state with [`MenuItem::selected_signal`] or
[`MenuItem::checked_signal`].

```rust
use pert_ui::{Density, KeyAction, Viewport, prelude::*};
use std::time::Duration;

let mut ui = Runtime::new();
ui.set_viewport(Viewport::new(640.0, 480.0, 1.0)?)?;
let theme = ui.theme();
let density = theme.density_binding()?;
let row = |key: &str, label: &str, choice: Density| -> Result<MenuItem, RuntimeError> {
    let theme = theme.clone();
    Ok(MenuItem::new(key, label, move |_| theme.set_density(choice).map(|_| ()))
        .selected_signal(density.map(move |current| *current == choice)?))
};
let looped = ui.signal(false);
let forwarded = ui.signal(None);
let engine = forwarded.clone();
let bar = MenuBar::new(&ui, vec![
    MenuItem::submenu("view", "View", vec![
        row("compact", "Compact", Density::Compact)?,
        row("standard", "Standard", Density::Standard)?,
        MenuItem::separator(),
        MenuItem::toggle("loop", looped.clone(), "Loop playback")
            .on_change(move |_, on| engine.set(Some(on))),
    ]),
])?;
ui.mount_tree(Element::new("bar", bar))?;
ui.flush(Duration::ZERO)?;
// F10 opens View; the first row is Compact.
for key in [KeyCode::F10, KeyCode::ArrowDown, KeyCode::Enter] {
    ui.key_input(key, KeyAction::Press)?;
    ui.key_input(key, KeyAction::Release)?;
    ui.flush(Duration::ZERO)?;
}
assert_eq!(ui.theme().density()?, Density::Compact);
assert_eq!(forwarded.get()?, None, "nothing chose the loop row yet");
# Ok::<(), Box<dyn std::error::Error>>(())
```

# Dialogs

[`Dialog`] and [`AlertDialog`] own modal content inside the existing native window.
[`DialogController`] reserves one terminal outcome per opening. Take that outcome before
opening again; the owner must also have observed closure in a flush. Cancel,
acceptance, owner removal and failed admission remain distinguishable. Native
focus loss keeps dialogs open. No task action or I/O is implicit in dismissal.

```rust
use pert_ui::prelude::*;

/// A delete button that asks for confirmation first; returns the dialog's
/// controller and the mounted root.
fn build(ui: &mut Runtime) -> Result<(DialogController, WidgetId), RuntimeError> {
    let confirmation = DialogController::new(ui);
    let show = confirmation.clone();
    let root = ui.mount_tree(Element::new("confirmation", AlertDialog::new(
        confirmation.clone(),
        Element::new("open", Button::new("Delete item…", move |_| show.open())),
        "Delete item?", "This action requires confirmation.", "Delete",
    )))?;
    Ok((confirmation, root))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    use pert_ui::testing::*;
    use std::time::Duration;

    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(640.0, 480.0, 1.0)?)?;
    let (confirmation, root) = build(&mut ui)?;
    ui.flush(Duration::ZERO)?;
    confirmation.open()?;
    ui.flush(Duration::ZERO)?;
    // Cancel is the initial focus target. Outside clicks do not dismiss.
    ui.key_input(KeyCode::Escape, KeyAction::Press)?;
    ui.key_input(KeyCode::Escape, KeyAction::Release)?;
    ui.flush(Duration::ZERO)?;
    assert_eq!(confirmation.take_outcome()?, Some(DialogOutcome::Cancelled));
    ui.unmount(&root)?;
    Ok(())
}
```

For arbitrary forms, use `Dialog::new(state, owner_content, dialog_content)`.
Its default scope is the window. `.view_modal()` instead blocks and clips within
the owner's allocated region: wrap the chosen view as `owner_content`. Nested
modals block their parents; conflicting independent scopes return `ModalConflict`
and record a failed outcome for the rejected session. `.require_response()`
consumes Escape until an explicit controller/button action completes the dialog.
Custom containers use the same public [`Component::child_surface`] contract and
[`SurfaceKind`] policies. `focus_gained` and `focus_lost` supply event-time lifecycle
notifications; physical held keys are never replayed on focus transfer.

```rust
use pert_ui::prelude::*;

/// A view whose Edit… button opens a form that blocks only that view.
fn build(ui: &mut Runtime) -> Result<WidgetId, RuntimeError> {
    let state = DialogController::new(ui);
    let open = state.clone();
    let cancel = state.clone();
    let accept = state.clone();
    let draft = ui.signal("Draft".to_owned());
    ui.mount_tree(Element::new("view", Dialog::new(
        state,
        Element::new("owner", Column::new([
            Element::new("heading", Label::new("One independently blocked view")),
            Element::new("open", Button::new("Edit…", move |_| open.open())),
        ])),
        Element::new("form", Column::new([
            Element::new("draft", TextField::new(draft)),
            Element::new("actions", Row::new([
                Element::new("cancel", Button::new("Cancel", move |_| cancel.cancel().map(|_| ()))),
                Element::new("accept", Button::new("Accept", move |_| accept.accept().map(|_| ()))),
            ])),
        ])),
    ).view_modal()))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    use pert_ui::testing::*;
    use std::time::Duration;

    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(640.0, 480.0, 1.0)?)?;
    build(&mut ui)?;
    ui.flush(Duration::ZERO)?;
    Ok(())
}
```

# Notifications

[`Banner`] stays in ordinary layout until its visibility binding changes or its
owner is removed. [`ToastHost`] wraps application content and presents its queue
without stealing focus. Queue capacity is three visible plus sixteen pending;
overflow returns `ToastFull`. Text and action labels have a 16 KiB UTF-8 limit.
A started toast expires after five unpaused seconds on the runtime clock; pending
toasts do not age. Hover/focus pauses the visible stack at the runtime's delivered
input time. Hidden/occluded content schedules no presentation; started lifetimes
continue while hidden, and expired items are retired on resume. These timings do
not promise OS-event timestamp accuracy. Long content supports wheel scrolling and
keyboard selection. Enter/Space activates the selected action, Delete dismisses;
failed actions close once and report both `ActionFailed` and the runtime error.

```rust
use pert_ui::{Viewport, prelude::*};
use std::time::Duration;

let mut ui = Runtime::new();
ui.set_viewport(Viewport::new(640.0, 480.0, 1.0)?)?;
let banner_visible = ui.signal(true);
let hide_banner = banner_visible.clone();
let queue = ToastController::new(&ui);
let post = queue.clone();
let root = ui.mount_tree(Element::new("notifications", ToastHost::new(
    queue.clone(),
    Element::new("content", Column::new([
        Element::new("banner", Banner::new("Connection restored", [
            Element::new("dismiss", Button::new("Dismiss", move |_| hide_banner.set(false))),
        ]).visible_signal(banner_visible.read_only())),
        Element::new("notify", Button::new("Notify", move |_| {
            post.push(Toast::new("Saved").action("Show details", |_| Ok(())))?;
            Ok(())
        })),
    ])),
)))?;
let receipt = queue.push(Toast::new("Ready"))?;
ui.flush(Duration::ZERO)?;
assert_eq!(receipt.status(), ToastStatus::Visible);
ui.flush(Duration::from_secs(5))?;
assert_eq!(receipt.status(), ToastStatus::Expired);
ui.unmount(&root)?;
# Ok::<(), Box<dyn std::error::Error>>(())
```
