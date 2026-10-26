# Manual

This is the application-facing guide. Start with a working window, then use the
component recipes below to build a view. Each recipe names the public type and
its required input; the generated API documentation gives every builder option,
error and keyboard rule. The examples use only `pert_ui` public APIs. The
[contracts](contracts.md) explain state and identity, and
[rendering and platform](rendering-platform.md) explains display behavior.

## Start in a few minutes

The library has not been released to a registry. Add the facade from your local
checkout to your application's `Cargo.toml`:

```toml
[dependencies]
pert-ui = { path = "../pert-ui/crates/pert-ui" }
```

Use the toolchain in the checkout's `rust-toolchain.toml`. This complete program
opens an adaptive form and keeps its values in UI-local signals:

```rust,no_run
use pert_ui::prelude::*;

fn main() -> Result<(), AppError> {
    App::new("Profile", |ui| {
        let name = ui.signal("Ada".to_owned());
        let city = ui.signal("Stockholm".to_owned());
        let (reset_name, reset_city) = (name.clone(), city.clone());
        let fields = Form::new([
            Field::new(TextField::new(&name).key("name"))
                .label("Name")
                .help("Shown to other people")
                .key("name-field"),
            Field::new(TextField::new(&city).key("city"))
                .label("City")
                .key("city-field"),
        ]);
        let reset = Button::new("Reset", move |_| {
            reset_name.set("Ada".to_owned())?;
            reset_city.set("Stockholm".to_owned())
        });
        let content = Column::new([
            Label::new("Your profile").key("heading"),
            fields.key("fields"),
            reset.key("reset"),
        ]);
        ui.mount_tree(
            ScrollArea::new(content.key("content"))
                .axes(ScrollAxes::Both)
                .key("profile-viewport"),
        )?;
        Ok(())
    })
    .run()
}
```

Save it as `src/main.rs` and run `cargo run` in the application. The same
program is maintained as the compiled package example
[`getting_started.rs`](../crates/pert-ui/examples/getting_started.rs). For a
headless host, disable default features and use `Runtime` directly; it does
not require a window or GPU. A custom Runtime host calls `shutdown` before
releasing its platform resources and drains `take_shutdown_errors` before dropping
the runtime. Shutdown delivers drag cancellation while participants are mounted;
cleanup failures remain typed even after a detached window is retired. The native
`App::run` drains this report automatically: `AppError::Shutdown` retains its
cleanup causes and any preceding initialization/event-loop/application failure.
With no cleanup failure, it returns the original result. Terminal cleanup bypasses
`on_error` because the runtime is closed; the application reports every cause
from the aggregate rather than requesting presentation retry.

Values the application creates once and must keep, such as a `Mailbox` made
in the initializer, belong to `ui.keep_alive(value)`; a local value would drop,
and a dropped mailbox closes. `ui.spawn(work, deliver)` runs work such as
reading or saving a file on a background thread and hands the result to
`deliver` on the UI thread, and `ui.start_interval(period, tick)` repeats
application work such as an autosave. When a flush fails in a component,
`ui.last_failure()` names the component and what failed: a callback, or the
children it described through `Dynamic`, `Show` or `Each`, with the keys of
the rejected description (`reconciliation of root/list at items/row-3`). The
native application's default error report includes it.

The default `desktop` feature supplies `App`, native input and GPU rendering.
`desktop-cpu`, with `default-features = false`, supplies the same `App`
presented by the CPU renderer and leaves wgpu out of the build, for tools and
machines without a usable GPU.
`png`, `jpeg` and the other image-format features add decoders; owned
`RasterImage` pixels need none of them. `svg` adds SVG loading and its
component. `serde` adds serialization for the supported inspection and
workspace documents. Enable only the features your application needs.

## The construction pattern

Each mounted node is an `Element`: a component under a stable key. Keys
identify siblings, not display text. `component.key("save")` describes a
component in place and is the same as `Element::new("save", component)`.
Containers receive child elements, so a small tree looks like this:

```rust
use pert_ui::prelude::*;
let root = Column::new([
    Label::new("Settings").key("title"),
    Button::new("Save", |_| Ok(())).key("save"),
])
.key("content");
let mut ui = Runtime::new();
ui.mount_tree(root)?;
# Ok::<(), RuntimeError>(())
```

Use `ui.signal(value)` for UI-local state. Pass `&signal` to each control that
edits or displays it: a control that only reads takes the read capability, and
a `Signal` is also a `ReadSignal` wherever a read binding is borrowed, so
`signal.map(..)` derives a value and `cx.read(&signal)` reads it in a custom
component. `set` writes a value, `update(|value| ..)` changes the current one
and `toggle` inverts a boolean; each returns `Result<(), RuntimeError>`, so a
callback can end with the write, and `set_changed` reports whether the value
changed. Use
`ui.parameter(id, spec)` when the application owns an authoritative numeric
value; retain the returned host while the control is available. Use live-data
sources for lossy display streams, never as an edit baseline. A structural
update reconciles matching keys and component types, preserving local state;
the new description's texts, bindings and options take effect, and a field
given another binding drops its unsaved draft. A custom component that
overrides `Component::reconcile` can be checked with `check_redescription`,
which compares a re-described component with a fresh mount.

What a user may do with a component, such as resizing a divider, is a
`Permission`: `true`, `false` or a `bool` binding the application changes
at run time. Permissions only restrict: repeated settings on one description,
and a layout-wide setting combined with an item's own, combine so that the
stricter wins. Geometry, window and collection gestures use the same policy;
capability-specific builders retain their documented structural prerequisites.
For example, a table resize permission still requires a resizable column, and a
list/tree reorder permission still requires a callback. Bound revocation cancels
the gesture as its component documents, including resize preference rollback.
Compatibility builders such as `locked` retain their documented restrictions.
Basic controls and choice controls accept permissions directly
through `enabled`, for example `Button::new("Save", save).enabled(can_save)`.
`disabled()` adds a fixed denial that a later binding cannot lift.
`Permission::signal` remains available for builders taking a read binding.

Label, Button and Link keep their `leading_icon` and `trailing_icon` when bound
text changes. Each builder replaces its own slot, as do the `leading_icon`
slots of Tab, Section and Collapsible titles. Chip has one leading slot;
its trailing target belongs to removal. On redescription, omitting a slot clears
it. Icons inside rich caption content belong to the caption and change with it,
so put a persistent icon in an explicit slot. The name given to `Button::icon`
is only the action's name.

Caption-bearing controls take their text as a `TextSource`: fixed plain or rich
text, a `String` or `RichText` binding (`Signal` or `ReadSignal`) they follow, or a
`StringRole` that follows the installed string table. Label,
Button, Link, Chip, Tooltip, Menu, MenuItem, Checkbox, Toggle and InlineToggle
names use it, so one constructor argument covers fixed, rich, translated and
changing text. Titles accept it too: tabs, sections, property rows,
collapsibles, breadcrumb segments, field labels and help, inline row captions,
table columns, floating and tool windows, alert dialogs, toasts and banners,
and `Element::with_label`. A bound title is checked when read; a blank one
keeps the last title and fails the update. Placeholders and the builders that
replace a library text, such as `ToastHost::dismiss_label`, take one as well,
and their defaults are roles that follow a replaced string table. To retain several composed icons, pass a composed `RichText`
rather than repeating a slot builder. A Checkbox or Toggle made with
`for_parameter` follows the parameter's name; its icon slots decorate the name
and `label` replaces it.

## Layout and navigation components

| Component | Build it with | Use it for |
|---|---|---|
| `Column`, `Row`, `Wrap` | `Column::new([Element::new(...), ...])` and the analogous constructors; `element.with_grow(1)` gives a child of a row or column a share of the spare space, and in a column whose height is bounded it also takes a share of a shortfall, down to the child's minimum, so a grown collection below a toolbar fills the rest (rows do not shrink), `Element::spacer(key)` pushes the following children to the far end, and `.justify(Justify::End)` and `.align(Align::Stretch)` place children along and across | Vertical, horizontal and wrapping groups, such as an editor that fills the rest of a window or a status bar with right-aligned items. |
| `Grid` | `Grid::new(tracks, children)?` | Track-based layout; at least one `Track` is required. |
| `Container`, `View`, `Group` | Wrap one child element; `Group::titled(title, child)` adds a heading. | A bounded region, a visibility boundary or a decorated group. A container's `background_role` fills it with a theme role; `background_role_signal` follows a binding, for example to mark a selected strip. |
| `Dynamic`, `Show`, `Each` | `Dynamic::new(&value, \|_, value\| element)`, `Show::new(&shown, \|_\| element)`, `Each::new(&items, key, \|_, item\| element)`, optionally `.within(\|elements\| Row::new(elements).key("row"))` | Children described from a binding: one child per value, a child that exists only while a flag reads true, or one child per list item, kept by key. |
| `Divider` | `Divider::new()` | A visual separator. |
| `ScrollArea` | `ScrollArea::new(child).axes(ScrollAxes::Both)`; `.on_scroll(callback)` reports the committed offset | Overflow and focus reveal. Vertical scrolling is the default. |
| `Form` | `Form::new(fields)` | Responsive field columns. |
| `Field` | `Field::new(control).label(...).help(...)` | Label, help and status around any control; `required(true)` marks the label, and a `FieldStatus::Pending` check shows a spinner. |
| `PropertyGrid` / `Property` | `PropertyGrid::new([Property::new(key, label, editor), ...])`; `.bind_label_width(signal)` aligns several grids; `.on_resize(callback)` reports the committed label width | Labeled editors in two aligned columns with a resizable label column; group rows by placing one grid per accordion section. |
| `RegionLayout` | `RegionLayout::new().top(...).left(...).center(...)`; `.on_resize(callback)` reports a committed edge | Window-like regions; nest another region layout when needed. |
| `SplitPane` | `SplitPane::new(axis, first, second)`; `.on_resize(callback)` reports the committed preference | Two independently sized regions with a movable divider. |
| `Tabs` / `Tab` | `Tabs::new([Tab::new(key, title, content), ...])`; `.selected(key)` chooses the first shown tab | Keyed views whose inactive content keeps state. Blank or duplicate keys and blank titles are refused before the strip mounts. |
| `Collapsible` | `Collapsible::new(title, content)` with a fixed, rich or bound title; `.bind_expanded(signal)` hands the expansion to the application | A header that shows and hides its content, which keeps its state while collapsed. |
| `Accordion` / `Section` | `Accordion::new([Section::new(key, title, content), ...]).mode(ExpandMode::Single)` | Several collapsible sections, one or many open, as one Tab stop. |
| `ToolWindowLayout` / `ToolWindow` | `ToolWindowLayout::new(center).edge(ToolEdge::Left, EdgeMode::Stripe).window(ToolWindow::new(key, title, content))`; `.locked()` fixes the arrangement; `ToolWindow::overlay(true)` slides a panel over the center; `ToolWindow::mode(ToolWindowMode::Floating)` or `ToolWindowMode::Window` floats it or opens it in its own native window; a `ToolWindowController` opens, moves, reorders and switches tool windows from commands | Tool windows docked at the edges of a main area and opened from stripes of buttons, as in IDEs. The user moves them between edges and groups and reorders them through a panel's options menu or by dragging stripe buttons, switches them to overlay panels that hide when focus leaves, floats them as frames over the layout, or opens them in their own native windows that return to the stripe when closed. Every edge, stripe and user capability is optional; collapsed tool windows keep their state and size. |
| `FileBrowser` | `FileBrowser::new(&controller, row_height)?` over a `FileBrowserController` | Folder tree, path bar, filter and entry list over directory listings the host supplies; the browser never reads the disk. |
| `ZoomView` | `ZoomView::new([(position, element), ...])` with a `ZoomController`, which serves one mounted view at a time | Pannable and zoomable content. |

For a large collection, implement the corresponding public `ListItems`,
`TreeItems` or `TableItems` data source. Mount
`VirtualList::new(&items, row_height)?`,
`Tree::new(&controller, row_height)?` or
`Table::new(&controller, row_height)?`; the tree and table controllers own
their source and selection. Only the rows inside the collection's allocated
height mount, so a collection needs a bounded one: put it in a window, a
`SplitPane` pane, a `Container` or `CollectionFilter` with a bounded
allocation, a `Tab::fixed()` tab, a `RegionLayout::fixed()` region, a
`View::fixed()` view or as a grown child of a bounded `Column`. Default `Tabs` and `RegionLayout` content
and a `ScrollArea` give it its whole content height, which mounts every row;
in debug builds validation reports that as `UnboundedCollection`. The collection keys each
row it builds with the row's key from the source, so `build` may describe a
row under any key.
`ListSelection` holds application-visible selection by stable key; a
collection's `on_selection_change` reports each change with its origin, for
example to show the chosen item in a detail view.
`VirtualAxis` supplies viewport arithmetic if you are writing a custom
collection. Built-in reordering is opt-in; accept reported moves in your
application's data model.

A `Table` has an active cell: the selection's active row and
`TableController::active_column`. `Table::on_activate` runs on Enter with the
row and column keys. To edit cells in place, mark columns
`TableColumn::editable`, answer `TableItems::editable` for cells that cannot be
edited now, and give the table a `Table::cell_editor` that builds the editor for
one cell from a `CellEdit`: a `TextField`, `IntegerInput`, `DecimalInput` or
`Select` built with `CellEdit::session`, its signals created through
`CellEdit::owner`. Enter, F2, a double-click or typing begins an edit; the
table hosts the editor in the cell, keeps the row mounted while the edit is
open, and ends or moves the edit as the editor reports it: Enter and Escape
return to the table, Tab and Shift+Tab move to the next or previous editable
cell. The editor writes the value; publish the plain cell's new content from
its save callback. A save that sorts or filters the source can move the edited
row out of view; the table does not follow it, so call
`TableController::reveal` with its key if it should stay visible. Enable row
commands from the rows the action can reach:
`ListSelection` keeps selected keys that a filtered source no longer shows.

`FileBrowser` shows directory navigation and selection over a
`FileBrowserController`: a path bar, a filter, a folder tree and the current
folder's entries. The browser performs no filesystem I/O. Create the controller
with `FileBrowserController::new(&ui, selection, roots, request)`; its `request`
callback receives a `DirectoryRequest` whenever a folder must be listed. List it
on your own worker thread, for example with `list_directory`, and hand
`request.respond(listing)` back to `controller.deliver(response)` through a
`Mailbox`. A response superseded by a newer request, or for a folder the user
has left, is dropped as stale; a failed listing shows as an error row.
Selection reports paths through `selected_paths`, and `on_open` receives the
files the user activates.

A custom component that loads data through its host the same way uses the
bookkeeping the file browser is built on: a `RequestTracker` issues a
`RequestTicket` per key, and `status` tells whether a result that comes back
is still `Current`, `Superseded` by a newer ticket, `Settled` (cancelled or
already applied) or `Foreign`. The component calls `complete` after applying a
result and leaves it pending when applying fails, so a retry can apply it.

To send the user to a target that may be hidden, in an unselected tab, a
collapsed section, a custom container that opts in or a scrolled-away row, call
`ui.reveal(&id)` or `ui.reveal_and_focus(&id)`, or queue it from a component
with `cx.request_reveal(&id)` or `cx.request_reveal_and_focus(&id)` and
observe the receipt. The outcome says whether the target was revealed, focused
or refused, and where. Select a tab by
key with `ui.reveal_child(&tabs, key)`, and follow the selection with
`Tabs::on_select`, which reports each change with its `ChangeOrigin`, so a
navigation history can record what the user opened and skip what the
application revealed itself. For a virtualized row, call the
controller's `reveal(key)` and reveal the `WidgetId` its receipt reports once
the row is mounted. An internal link is a `Link` whose action requests a
reveal; for "search and jump", register one command per destination.

To let the user search the application's own content, call
`ui.content_search(ContentQuery::new(text)?)` and `step` the returned cursor:
it matches the semantic names and values components report (button text,
labels, field values), hidden pages and collapsed sections included, and the
rows of a `VirtualList`, `Table` or `Tree` whose source answers
`ListItems::text`, `TableItems::text` or `TreeItems::text`. Each step does at
most the query's work budget and returns at most its result limit; a search
whose content changed since its last step reports `Stale`. Exclude a subtree
with `Element::with_searchable(false)`; a concealed field's value is never
matched. `ui.jump_to_content(&hit.target)` then reveals and focuses the hit,
mounting a row first, and reports a refusal as a typed outcome; a row the
collection mounts only later comes back as `ContentJump::Pending`, which
`ui.resume_content_jump(&pending)` finishes once its status reads `Mounted`.
To present content in a `QuickPick` beside commands, list
`ContentQuery::everything()`, label each row's kind and include the hit's
`other` text, so a field is found by its name or its value.

To let the user filter a collection, wrap it in a `CollectionFilter`: a
search field above the list, table or tree that shows only the rows whose
same text matches. A list binds to a `ListFilter`'s `items()` instead of its
source; a table or tree is filtered through its controller
(`TableController::filter`, `TreeController::filter`). Rows that stay keep
their components and a selected row filtered out stays selected and hidden;
in a tree each match shows with its ancestors, expanded in a temporary layer
that leaves the user's expansion untouched. A lazy tree source answers
`TreeItems::known_children` with `None` for what it has not listed, so the
filter never forces a listing. `offered(permission)` decides whether the
field is offered at all, and `debounce` waits for a pause in typing, since
each query reads every row's text.

Tabs can opt into reordering or native detachment. Detached native views retain
their component state but require the desktop facade to create the OS window.
`FloatingWindow` instead overlaps content inside an existing view. Workspace
arrangement can be captured and restored through `Workspace`; the application
chooses where to store the document. Collapsible sections and accordions
remember their expansion there by default, a property grid its resized
label column and a tool window layout which tool windows are open and their
sizes; `persist(false)` opts out. Table restoration follows current column
capabilities: fixed columns stay in place, and movable columns stay inside their
contiguous movable runs. An incompatible saved order rejects that table entry
without applying its widths or order; inspect the workspace restore report.

## Text, actions and choices

Choice groups and dropdowns take fixed lists through `new`, reactive lists
through `bound`, and parameter-supplied options through `for_parameter`.
MultiSelect has fixed and reactive constructors. Dynamic construction is
infallible and reads the source at update; duplicate values and foreign-runtime
bindings produce typed update errors. Correcting the source recovers the control.
Replace `X::new(value, [])?.choices(source)` with `X::bound(value, source)`;
keep the leading `&ui` argument for Select, InlineSelect and MultiSelect.
Parameter construction has no independent option-list builder.


| Component | Build it with | Input and behavior |
|---|---|---|
| `SelectableText` | `SelectableText::new(text)` with fixed, rich or bound text | Read-only text a reader can select and copy, such as a log message or error details: no frame, wraps like a multiline label, one Tab stop, drag, double/triple click and Shift with the editors' movement keys select, Ctrl/Cmd+C or its context menu copies. Rich text is shown and copied as plain text. |
| `Label` | `Label::new(text)` with fixed, rich or bound text | Static or reactive text; rich text and icons are optional. `overflow(TextOverflow::Ellipsis)` or `MiddleEllipsis` keeps it on one line with "…", the full text as its name and, up to the text limit, as help on hover; a line too long to prepare is cut first. Text it cannot prepare is refused without failing the flush (`text_error` in inspection). `RichText::highlighted` marks search matches. |
| `Button` | `Button::new(label, callback)`, or `Button::command(&ui, &command, on_receipt)?` and `command_icon` for a registered command | An action callback; keyboard activation starts with unmodified Enter or Space. A command button shows the command's name (or an icon), follows its enabled binding and reports each invocation's receipt. |
| `Link` | `Link::new(text, callback)`, optionally `.underline(LinkUnderline::Always)` | Navigation presented as text-sized link text, activated like a button by pointer or unmodified Enter or Space. The destination is the application's, typically a reveal through `EventContext::request_reveal`. |
| `Chip` | `Chip::new(text)` or `Chip::removable(text, on_remove)` | A compact tag or chosen item. A static chip has no Tab stop; a removable one reports Delete, Backspace or its × button to the owner, which removes the item and describes its chips again. Focus moves on to the next chip. |
| `Checkbox`, `Toggle` | `Checkbox::new(bool_signal, label)` / `Toggle::new(bool_signal, label)`; `Checkbox::controlled(value, label, on_change)` for a value the application owns | Two presentations of a boolean. |
| `RadioGroup` | `RadioGroup::new(signal, [Choice::new(value, label), ...])?` | One typed selection with arrow navigation. |
| `SegmentedControl` | `SegmentedControl::new(signal, choices)?` | Joined, exclusive choices. |
| `Select` | `Select::new(&ui, signal, choices)?` | A popup for one selection. |
| `MultiSelect` | `MultiSelect::new(&ui, vec_signal, choices)?` | Several values from one option set, shown as removable chips. The popup filters its list with a search field; Up/Down move the highlight and Enter or a click toggles. `commit(MultiSelectCommit::OnClose)` writes a draft when the popup closes instead of on every toggle; Escape discards it. |
| `Menu` | `Menu::new(&ui, title, items)?` with `MenuItem::new(key, label, action)` rows | An anchored action list and submenus. `MenuItem::toggle` and `radio` rows write a binding; their `on_change` reports each choice the user makes. |
| `Breadcrumb` | `Breadcrumb::new([BreadcrumbSegment::new(key, label), ...], on_activate)?` | A path of keyed segments; activating one reports its key. The last segment is the current location. At narrow widths the middle segments collapse into a menu. |
| `ContextMenu` | `ContextMenu::new(&ui, content, items)?`; `.controller()` opens it from application code | A menu opened from content by secondary press, its keyboard shortcut or its controller with a captured command context. |
| `MenuBar` | `MenuBar::new(&ui, menus)?.content(content)` | A top-level menu strip and its owned content; `leading` and `trailing` add content before the titles and at the row's right edge. |
| `Toolbar` / `ToolGroup` | `Toolbar::new(&ui, [ToolGroup::new(key, controls), ...])?` | Adaptive action strips and overflow. |
| `ColorSwatch` | `ColorSwatch::new(color)` or `ColorSwatch::bound(read_signal)`, optionally `.on_activate(callback)` | A color sample, with a checkerboard under translucent colors; activatable like a button, for example to open a color editor. |
| `ColorPicker` | `ColorPicker::new(color_signal)`, optionally `.alpha(true)`; `.into_popover()` opens it from a swatch | A saturation/value area, hue and optional alpha strips and hex/channel text entry over a linear `Color`. Edits happen in HSV, so the hue survives at zero saturation. The binding is written once per completed edit: at a drag's release, per key step and per saved field; Escape cancels a drag. `on_change` reports each such user write that changed the color, for example to record undo. |

`Choice<T>` carries a value and display label. Reordering or renaming a choice
does not change its identity. Boolean and choice controls also offer
`for_parameter` constructors for authoritative bindings. Menus close before
calling an action; commands use their registered scope and current enabled
binding. `Runtime::commands_for(&context)` lists the commands a captured
context can reach, with their names, shortcuts and enabled state, for a
command palette or a shortcut list; `CommandPalette` presents that listing.
`KeyChord::named(KeyCode::Space)` is a chord of one named key without
modifiers. A command action may use `?` on the workspace and capture errors
of the runtime methods it calls, and on the window error a presentation
receipt reports as failed. They arrive as `RuntimeError::Application`,
downcastable to the original error, except a `Runtime` variant, which becomes
the runtime error it holds.
A command that copies the application's own data, such as selected table rows,
writes it with `Runtime::set_clipboard_text`, which needs no component and
moves no focus; `Runtime::clipboard_text` reads it.
"Search the UI and jump there" is a palette over commands the application
registers per destination, each revealing its target with
`Runtime::reveal_and_focus`. The gallery shows combined menus and controls on
real pages.

`NavigationHistory<T>` records the places an application visits, such as
pages or documents, with back and forward movement like a browser. Call
`visit(place, label)` when the user navigates; `back`, `forward` and `go_to`
return the entry to restore, and the application restores it itself, for
example with `Runtime::reveal_child`. Record only `ChangeOrigin::User`
changes from callbacks such as `Tabs::on_select`, so the history's own
restorations are not recorded; a visit equal to the current entry records
nothing either. Its `state`, `can_go_back`, `can_go_forward` and `current`
signals drive buttons and lists without polling. It keeps 100 entries by
default and is not persisted.

`HistoryNavigator::new(&ui, history, restore)` (any `StateOwner`) presents that history as a
browser does: back and forward icon buttons that are disabled while there is
nowhere to go and whose tooltips name their target, and a history icon that
opens a searchable list of the visited places, newest first. Each moves the
history and calls `restore` with the entry; the control never shows a place
itself. `buttons` and `list` restrict which parts are offered, each taking a
constant or a bound signal as a `Permission`. Wrap the content in `HistoryButtons` so the mouse's back and forward
side buttons (`PointerButton::Back` and `PointerButton::Forward`) do the same,
and register the platform's shortcuts from `KeyChord::navigate_back` and
`KeyChord::navigate_forward` as commands that call `back` and `forward`.

## Numeric and text input

| Component | Build it with | Notes |
|---|---|---|
| `Slider`, `InlineSlider` | `Slider::new(parameter)` / `InlineSlider::new(parameter)` | Continuous parameter editing with begin, preview, commit and cancel. Orientation, value display and formatting are builders. |
| `Knob` | `Knob::new(parameter)` | The slider's edit protocol, keyboard and wheel behavior in a rotary dial: vertical drag relative to the press, Shift for fine adjustment, Backspace or a double-click for the default. `show_value`, `unit`, `format` and `arc_origin` (fill the arc from the minimum or from the default) are builders. |
| `IntegerInput`, `DecimalInput` | `IntegerInput::new(signal)` / `DecimalInput::new(signal)` | Local numeric text entry with validation. `stepper(true)` adds step buttons; `format` and `unit` set what the field shows at rest. |
| `InlineInteger`, `InlineDecimal` | Construct from a numeric signal. | Compact editable value rows. |
| `InlineReadout` | `InlineReadout::new(read_signal)` | A read-only compact value. |
| `InlineToggle`, `InlineChoice`, `InlineSelect` | `InlineToggle::new(bool_signal, name)`, `InlineChoice::new(signal, choices)?`, `InlineSelect::new(&ui, signal, choices)?` | Controls that fit a toolbar or dense row. |
| `TextField`, `TextArea` | `TextField::new(string_signal)` / `TextArea::new(string_signal)` | Single-line and multiline editing with selection, clipboard and IME. `placeholder`, `required`, `min_length`/`max_length` with a `counter`; `TextField::concealed` for a password, with an optional `revealable` eye button; `TextField::clearable`, `leading_icon` and `suffix`; `modified_indicator` and `on_modified` for unsaved drafts; `on_binding_refusal` reports when the field refuses a value the application wrote. |
| `SearchField` | `SearchField::new(string_signal)` | A query with a search icon and clear button. Escape clears; Up/Down, Page Up/Down and Enter go to `on_key` for a result list while focus stays in the field; `debounce` delays the query until typing pauses. |
| `InlineText` | Bind a string signal, or `InlineText::controlled(text, on_save)` for text the application owns. | Compact editable text. |
| `DraftEditor` | `DraftEditor::new(DraftEditorController::new())` | Embed a draft text session in a custom component. |

`ParameterSpec` defines the numeric domain, default and mapping: linear,
logarithmic for a positive domain, or `ParameterMapping::Power` with a
`PowerExponent` to give part of the range more travel. Formatting
and units are presentation choices. A slider's local preview is separate from
the host's accepted base value. Escape or lost context cancels a pending edit.
For text input, IME preedit is kept separate from committed text.
Every input field that saves a value (`TextField`, `TextArea`, `InlineText`
and the integer and decimal inputs) accepts `normalize`, the application's
adjustment of an entered value when it is saved, such as rounding to tenths or
trimming text. The normalizer may refuse the value with an `InputRejection`,
which keeps the draft for correction; `on_rejection` tells the application
about it, and about the field's own refusals (a length limit, or a number that
does not parse or is out of range), so it can publish them as the field's
`FieldStatus`. `on_save` reports each save
that wrote the local binding or submitted a remote proposal, or, on an
`InlineText::controlled` label, which writes nothing, proposed it, with the
saved value; Enter, focus loss or a step button saves, and the callback does not say
which. Submission does not mean external authority acceptance. Values the application writes are never
normalized. A normalizing `TextField` or `TextArea` holds a draft and writes
its binding only when the draft is saved, instead of on every edit; the
identity normalizer `.normalize(Ok)` gives a field that saves, and so calls
`on_save`, without adjusting anything, for example to record each finished
edit as one undo step: a `TextField` on Enter or focus loss, a `TextArea`,
where Enter inserts a line break, on focus loss. Length
limits (`min_length`, `max_length`) count grapheme clusters and are checked
when the draft is saved, after normalization; an empty field passes unless it is
`required`. `SearchField` writes its query as typed (after any debounce), and
`DraftEditor` leaves saving to the component that embeds it, so neither has
these hooks. A concealed `TextField` shows one dot per grapheme and keeps its
text out of the clipboard, the IME, inspection and event history. A
`TextField` can carry a clear button, a leading icon and suffix text, all
inside its single Tab stop. An editor in draft mode reports when its draft
starts or stops differing from the saved value through `on_modified`, and
`FieldHandle::set_modified` lets a `FormFields` count unsaved fields in
`FormSummary::modified`. `FieldStatus` (normal, warning, error or a pending
check) and `FormFields` can present and navigate application-supplied statuses
without giving any one control special ownership of validation.

Remote parameters use `Runtime::remote_parameter` and retain a submitted proposal
without changing their authoritative base. Built-in controls display that proposal
and refuse further edits while authority is pending, unknown or disconnected. A user save/change
callback reports local commitment or successful submission once; an authority
correction or rejection is observed through the binding and receipt, not a second
user callback. A failing callback cannot retract a retained proposal. Transport
handoff is separately reported by `SubmissionDelivery`:
a full queue leaves the complete proposal retained for explicit `retry_submission`.
A successful retry proves handoff only. Route external responses onto the UI
thread; see the runnable `RemoteHost` rustdoc example and the packaged dynamic
application example for UI mailbox integration.

Retain an `AuthorityReceipt` for each submission's full edit token and connection
generation. Disconnect makes an in-flight request `Unknown`; reconcile the real
external store before `resynchronize` installs its fresh base/generation. Old
responses become `Stale`; a local terminal rejection after resynchronization or
removal does not prove that external mutation never ran. The host owns external
deduplication, reconciliation and undo. Reading only the latest receipt loses the
correlation needed for older unknown outcomes.

Capture the latest receipt immediately after each submission or submitting input,
before it can be replaced. Its pending, unknown and resolved states carry the
full proposal. A connected host write or connected resynchronization locally
resolves a pending receipt as `Rejected(Conflict)`; this is local supersession,
not proof of external refusal. Disconnect before recovering an externally unknown
proposal to preserve its `Unknown` receipt. Reconcile the actual external store
regardless of that local status.

A first authority response after a same-generation host write can return
`AlreadyResolved`, even when the authority accepted the mutation. The UI adapter
does not apply that response; reconcile its content with the actual external store.
Resynchronization instead changes the generation and returns `Stale` for old
responses. Neither disposition alone proves a duplicate or external refusal.

`ParameterHost::set_effective` supplies read-only data validated independently of
the editable domain. Built-in parameter controls show editable preview/base and
do not render this overlay. A custom display reads `ParameterSnapshot::effective`
(which already falls back to preview/base), checks the editable specification to
indicate clipping, and presents the supplied value without clamping or writing it
back. The public `set_effective` rustdoc demonstrates an `InlineReadout` built from
a mapped snapshot.

## Feedback, surfaces and windows

| Component | Build it with | Use it for |
|---|---|---|
| `ProgressBar` | `ProgressBar::new(status.read_only())` | Determinate or indeterminate progress. |
| `Spinner` | `Spinner::new(busy.read_only())` | Activity while a task is active. |
| `Banner` | `Banner::new(message, actions)` | Persistent in-view feedback. |
| `ToastHost` | `ToastHost::new(controller, content)` with `ToastController::new(&ui)` | Bounded transient notifications; enqueue `Toast::new(message)` through the controller. |
| `Tooltip` | `Tooltip::new(anchor, text)` | Delayed, noninteractive help. |
| `Popover` | `Popover::new(open_signal, anchor, content)`; `.on_dismiss(handler)` reports the framework's closes | Anchored, interactive nonmodal content. |
| `QuickPick` | Create `QuickPickController::new(&ui)`, wrap content in `QuickPick::new(controller, content, choose)` and open it with `controller.open(items)` from an event or command action | Choosing one of the application's own items (pages, history entries, search results, named values) from a searchable list. Each `PickItem` has a key, label, optional detail and kind; the `choose` callback answers `Done`, `Refused(message)` or `Pending` for a result that finishes later. Modal by default; `nonmodal()` closes when focus leaves. |
| `CommandPalette` | Create `CommandPaletteController::new(&ui)`, wrap content in `CommandPalette::new(controller, content)` and open it from a command action with `controller.open(ui, context)` | Searching and running the registered commands. Typing filters, Up/Down move the highlight while focus stays in the search field, Enter runs the command. A command that did not run reopens the palette with the reason. Modal by default; `nonmodal()` for a surface that closes when focus leaves. |
| `Dialog`, `AlertDialog` | Create `DialogController::new(&ui)`, then use `Dialog::new(controller, anchor, content)` or `AlertDialog::new(controller, anchor, title, message, accept_label)` | Window-scoped decisions with a terminal outcome. |
| `FloatingWindow` | `FloatingWindow::new(&ui, key, title, content)`; `.on_frame(callback)` reports the committed placement | Overlapping, movable content in one native window. |
| `WindowLayer` | `WindowLayer::new([floating_window, ...])` | Stacking and routing for in-view floating windows. |

Transient surfaces draw above ordinary content, follow their owner and close
according to their documented dismissal rules. A dialog is modal; a tooltip
never intercepts input. Notifications report bounded delivery and receipts.
The application owns the meaning of actions and the work that produces them.

## Data display and custom drawing

`LevelMeter` and `Trace` accept a `SeriesSource<f32>` and a
`ParameterSpec<f32>` display scale: construct them with
`LevelMeter::new(source, scale)` and `Trace::new(source, scale)`.
Their source can be lossy; freshness and gaps are explicit.
`Image::new(raster_image)` displays owned pixels with fit and sampling options.
With the `svg` feature, `Svg::new(bytes)?` keeps declared colors and
`Svg::tinted(bytes, role)?` creates a theme-colored mark. `Color` holds linear
channels; `Color::from_srgb8`, `Color::from_srgb` and `Color::from_hex` convert
the gamma-encoded sRGB values and `#rrggbb` text that design tools and users
write, and `to_srgb8`, `to_srgb` and `to_hex` convert back. `with_alpha`,
`scale_alpha`, `mix` and `over` adjust and composite resolved colors, and
`OpaqueColor` measures WCAG 2.x contrast once a color is opaque. `Shape` is a
silhouette passed to painting methods, not a mounted component. A custom
component can record its own rectangles, text, masks, images and paths through
`Painter`.

A custom widget implements `Component` and returns measured size, arranges any
children, handles events and paints. It requests layout only when its size can
change; ordinary value changes can invalidate paint alone. After every layout
the runtime repeats the last pointer position as a move
(`PointerEvent::is_refresh`), so a widget that drags or hovers invalidates only
when what it derives from the event changed. `ClickCounter` counts double and
triple clicks by the same rule as the built-in editors, knobs and file rows
(500 ms and four logical pixels from the previous press); the widget resets it
when its input is cancelled, and after a double click that does one thing,
such as opening an item, but not between the steps of a cycle such as word
and then whole-text selection. The complete
public-API example in the [getting-started guide](../crates/pert-ui/src/guide/getting-started-continued.md)
shows a custom widget, and the [custom widgets chapter](../crates/pert-ui/src/guide/custom-widgets.md)
covers dragging, clicks, held keys, described children, inspection and an
inspector over an application document. The packaged
[`diagram_editor`](../crates/pert-ui/examples/diagram_editor/main.rs) example
puts them together in a canvas widget with an application around it; run its
headless session with
`cargo run -p pert-ui --example diagram_editor --no-default-features --features cpu`. The
[`log_explorer`](../crates/pert-ui/examples/log_explorer/main.rs) example
shows large data: bounded collections over hundreds of thousands of lines,
worker loading, a live tail and copying; run its headless session with
`cargo run -p pert-ui --example log_explorer --no-default-features`. The
[`inventory`](../crates/pert-ui/examples/inventory/main.rs) example edits a
table in place like a spreadsheet, with a toolbar of commands, undo and row
copy and paste; run its headless session with
`cargo run -p pert-ui --example inventory --no-default-features`. The
[`live_mixer`](../crates/pert-ui/examples/live_mixer.rs) example connects a
producer thread to meters and parameters, with undo, retry of undelivered
edits and a saved layout; run its headless session with
`cargo run -p pert-ui --example live_mixer --no-default-features`. The [gallery guide](../apps/gallery/README.md) points to
live examples and the current component pages.

## Appearance, diagnostics and persistence

`ThemeManager`, `UiSettings` and their typed color, density, metric and zoom
roles configure appearance without a renderer choice. `FontManager` and
`IconManager` replace bundled assets when needed. Use `Runtime::inspect` or the
inspection service to capture owned semantic snapshots; name a control that
has no visible label with `Element::with_label` or a `Field`. Optional profiling
and the debug overlay help find actual invalidation and layout costs. Capture
requests target a rendered window or widget, for example
`CaptureTarget::window(&ui.primary_window())` or `CaptureTarget::region`;
they are separate from inspection event history. `Runtime::find` looks a
component up by the path inspection reports, without its root segment. `InspectionReply::flush_error` carries an owned `RuntimeError`
without discarding a reply's value. Match it directly; when an application
protocol needs text, use `.as_ref().map(ToString::to_string)`. Capture failures
retain runtime and drawing causes, including application errors. The `serde`
feature serializes portable capture outcomes and backend diagnostics; it rejects
Rust-cause variants, which need an application-defined transport DTO.
`Workspace` persists arrangement, while the application keeps
its own data and save policy.

For text coverage, `UiSettings::set_text_rendering` selects `LineMask` (the
default) or `PhaseLine` (experimental). An application can switch them at runtime through the public facade; the choice
is local to that runtime and is not saved by the library. The gallery's Text
rendering page shows both with editable and scrolling text. A mode change
repaints text without changing layout or caret positions.

The words the library itself shows, such as the editors' clipboard menu,
placeholders, accessibility names and refusal messages, come from a
replaceable `UiStrings` table keyed by `StringRole`. Install a translated
table with `UiSettings::set_strings`; the components that show those words
follow it without being described again, and text the application gives a
component itself is never replaced. `NumberLocale` with
`NumberFormat::localized_decimal` or `localized_integer` writes and parses
numbers with an application-chosen decimal separator and digit grouping.


### Component review workflow notes

Boolean controls and single-choice controls support `on_change` for successful
user writes or remote submissions; a controlled Checkbox or Toggle, which writes
nothing, reports the user's proposal instead. External writes and authority
responses do not invoke it. Select closes its list before calling the handler;
work queued with `EventContext::defer` runs after the list has closed.
MultiSelect's `on_change` reports each user write with the values written: a
toggle under EachToggle, a chip removal, and an OnClose draft written by a
committing close. Discarded and conflicting drafts report nothing.

`Choice` leading/trailing icons and `MenuItem` leading icons occupy replaceable
slots that survive caption replacement. Use rich content for multiple icons.
MenuItem `enabled` accepts a restrictive Permission; command and application
restrictions combine.

Accordion's `on_change` receives `(context, &ExpansionChange)`, the open keys
and their origin, after layout settles, including restore and redescription
changes after initial preparation.
FloatingWindow placement builders work from `UpdateContext`; controller placement
supersedes initial geometry. A ListController addresses one mounted list, including
when that list is hidden. It may be reused after the prior list is removed.

See [the composition contracts](contracts.md#identity-and-composition) for
retained-controller and editor-policy exceptions, and [verification](verification.md)
for execution evidence and native-platform gaps.
