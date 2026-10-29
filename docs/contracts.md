# State, composition and interaction contracts

This document collects cross-cutting rules. Use the [manual](manual.md) for
construction examples and generated Rust documentation for each item's full
signature, defaults and errors. [Architecture](architecture.md) describes the
package boundaries; [verification](verification.md) records tested behavior.

The sections up to the public naming rules hold what an application relies
on. What a custom component or a custom host needs beyond that is under
[For component and host authors](#for-component-and-host-authors). The
[implementation notes](internals.md) record, for maintainers, the rationale
and internal ordering behind some of these rules; nothing there is a
guarantee this document does not state.

## Contents

- [Identity and composition](#identity-and-composition)
  - [Choice sources](#choice-sources)
  - [Element trees and identity](#element-trees-and-identity)
  - [Text sources and labels](#text-sources-and-labels)
  - [Component redescription](#component-redescription)
  - [Permissions and availability](#permissions-and-availability)
  - [Container composition and validation](#container-composition-and-validation)
  - [Layout, visibility and focus](#layout-visibility-and-focus)
  - [User-write callbacks](#user-write-callbacks)
- [Callback timing](#callback-timing)
- [Data and edit ownership](#data-and-edit-ownership)
  - [Controlled controls](#controlled-controls)
  - [Parameters and gestures](#parameters-and-gestures)
  - [Component notifications](#component-notifications)
  - [Producer data and mailboxes](#producer-data-and-mailboxes)
- [Input and focus](#input-and-focus)
  - [Physical keys and held actions](#physical-keys-and-held-actions)
  - [Commands and modal admission](#commands-and-modal-admission)
  - [Reveal and focus](#reveal-and-focus)
  - [Drag and drop](#drag-and-drop)
  - [Closing a window](#closing-a-window)
  - [Runtime shutdown](#runtime-shutdown)
  - [Calling the runtime from callbacks](#calling-the-runtime-from-callbacks)
- [Text and draft editing](#text-and-draft-editing)
  - [Draft policy](#draft-policy)
- [Theme palettes](#theme-palettes)
- [Workspace persistence](#workspace-persistence)
- [Inspection and diagnostics](#inspection-and-diagnostics)
- [Public compatibility policy](#public-compatibility-policy)
- [Error policy](#error-policy)
- [Public naming rules](#public-naming-rules)
- [For component and host authors](#for-component-and-host-authors)
  - [Describing and redescribing components](#describing-and-redescribing-components)
  - [Layout and focus hooks](#layout-and-focus-hooks)
  - [Reporting component notifications](#reporting-component-notifications)
  - [Failure cleanup in components](#failure-cleanup-in-components)
  - [Stacking, hit testing and pointer refresh](#stacking-hit-testing-and-pointer-refresh)
  - [Content search hooks](#content-search-hooks)
  - [Host-loaded data](#host-loaded-data)
  - [Editor sessions](#editor-sessions)
  - [Workspace hooks](#workspace-hooks)
  - [Inspection in custom components](#inspection-in-custom-components)
  - [Custom hosts](#custom-hosts)

## Identity and composition

### Choice sources

Choice controls choose their option ownership at construction: fixed `new`
lists validate immediately, reactive `bound` lists validate during update, and
`for_parameter` takes options only from the parameter specification. No builder
can combine those sources. Same-key redescription replaces value and option
sources together, preserving stable selected values and the control's documented
cancellation/draft policy. An invalid reactive list leaves the last valid
presentation in place; the initial last-valid list is empty. MultiSelect applies
replaced sources and filters to its retained list within the successful flush,
keeping surviving row identities, query and draft; this does not commit the draft.

This vocabulary governs controls whose options are values written into a
selected-value binding. Collections whose rows are content (`VirtualList`,
`Table`, `Tree`, `FileBrowser`) and keyed containers (`Tabs`,
`ToolWindowLayout`) are outside it, and name their constructors by content or
sizing.

### Element trees and identity

Applications build an owned tree of `Element`s and mount it with `Runtime` or
`App`. Keys identify siblings within one owner; labels are presentation. A
`WidgetId` is a runtime target with a distinct incarnation, so removal and
recreation cannot retarget a stale event. Keyed reconciliation preserves
compatible component state, including focus, scroll and text drafts. Structure
changes are requested explicitly; ordinary signal changes do not rebuild the
root. A custom `Component` may use the same public context, child, event and
painting operations as a built-in component. Built-ins that own local state
take a `StateOwner` in their constructors (the `Runtime`, an `UpdateContext`
or an `EventContext`), so a component can build them while it describes its
children or reacts to input, without a privileged path. `MenuItem::command`
still needs the `Runtime`, because it reads the command registry when it is
built. Component descriptions are UI-thread values and are not `Send`; a
worker returns plain data, and the UI thread builds the description from it.

### Text sources and labels

#### Captions

Caption-bearing controls (Label, Button, Link, Chip, Tooltip, Menu, MenuItem,
Checkbox, Toggle and InlineToggle names) take their text as one `TextSource`:
fixed plain or rich content, a `ReadSignal<String>` or `ReadSignal<RichText>`,
or a `StringRole` of the installed `UiStrings`. A binding or role is read in the
component's update with a subscription (to the binding or the string table), so
a change remeasures it without polling; such a caption is empty before that
first update, except a library default, which shows the table's current text.
Redescription with the same binding or role keeps the text already read;
another binding or fixed text replaces it, fixed text when the description is
adopted. A foreign binding fails the update with `ForeignRuntime`. Custom
components follow the same contract ([text
sources](#describing-and-redescribing-components)).

#### Text beyond the limits

Display text is application data, so its size does not fail a flush. An
ellipsis truncation (`TextOverflow`, `MeasureContext::fitted_text`) cuts a
first line longer than `TextEditError::MAX_TEXT_BYTES` before fitting it:
the start, or as many grapheme clusters from the start as from the end, on
the line's own cluster boundaries and within the limit together with the
ellipsis, which then always shows. A `Label` refuses what it still cannot
prepare (wrapped or rich text over the byte limit, a layout beyond a raster
limit, rich content beyond its segment limit). It keeps its last prepared
text and size, reports `text_error` with invalid semantics, and recovers
when a later text, width, style or scale fits. Its `text` property and name
stay the current text, which is the application's value. `SelectableText`
names the text it still shows, because its selection refers to that text.
Help labels, an ellipsis label's and `Tooltip::new`'s, show nothing while
refused rather than stale text. An ellipsis label offers no help for text
over the byte limit. Components built on `Label` (`AlertDialog` titles and
messages, `Group` headings, table and tree cells an application builds from
labels) inherit this. Controls that prepare their own captions (buttons,
chips, links, menu rows, tab and window titles) still fail on such text.

#### Placeholders and library text

Placeholders and the builders that replace a library text take a `TextSource`
too: the `placeholder` of TextField, TextArea, SearchField, Select,
InlineSelect, MultiSelect (with `search_placeholder`) and CollectionFilter, the
names and texts of QuickPick, CommandPalette, ColorSwatch, ColorPicker,
HistoryNavigator and CollectionFilter, `ToastHost::dismiss_label`,
`Toolbar::more_label` and `Chip::remove_name`. Their library defaults are
roles, so a replaced string table relabels them, and an application gives a
library text back explicitly by passing its `StringRole`.

#### Titles and inspection labels

Titles and secondary texts take a `TextSource` as well: Tab, Section and
Property titles, Collapsible titles, BreadcrumbSegment labels, Field labels and
help, the inline row captions, `TableColumn` titles, FloatingWindow and
ToolWindow titles and chrome labels, AlertDialog texts, and Toast and Banner
messages and action labels. `Element::with_label` accepts a binding: the
runtime reads it with the parent's update context before the parent updates,
so a container sees the current text in `ChildInfo` and the label names the
component in inspection; until that read the label is absent.

A title that must not be blank is still refused when described if it is fixed;
a bound one is checked when read, and a blank read keeps the last title and fails the update
with the same typed error (`LayoutError::BlankLabel`, `EmptyWindowTitle`). A
breadcrumb reads bound labels itself to decide which segments collapse, a table
reads bound column titles, and a visible toast follows its bound texts without
cancelling an armed activation. Toasts are identified by admission, never by
text.

A detached tab or tool window's native window opens with the title its
detach request carries and then follows its view's title, which the default
App applies to the open window; a window opened with another title takes the
view's title when first seen. `NativeWindowOptions` keep the title the window
opened with. [Custom hosts](#custom-hosts) apply the same requests.

### Control adornments

Label, Button, Link, Choice, Checkbox and Toggle keep explicit leading/trailing
icon slots separate from caption content; Chip has one leading slot and reserves
its trailing edge for removal; MenuItem has one leading slot. Repeating one of
these icon builders replaces its slot, and omitting it from a new description
clears it. Rich content from a caption source replaces embedded icons with the
caption but retains explicit slots. An icon button's text source supplies its
semantic name; its central icon remains independently described content.
Adornment changes remeasure without changing identity or cancelling an armed
activation. A Checkbox or Toggle made with `for_parameter` shows and follows the
specification's name; its slots decorate the name without replacing it, and
`label` replaces the name with an application text source. InlineToggle's
`name` replaces a parameter's name the same way.

### Component redescription

A surviving child described again takes its new description through
`Component::reconcile`: texts, bindings, ranges, permissions and callbacks
follow the new description, while focus, scroll and drafts stay. A different
binding or edit target ends the edit in progress unsaved, as an application
write does.

Reconciliation publishes no signal, with one lifecycle exception: a
redescription that rebinds a parameter control or denies it a fixed permission
cancels the parameter's gesture during reconciliation and publishes its
snapshot at once; the terminal event follows at the next safe point. The
components and derived bindings that read the snapshot settle in the flush
that reconciles.

Components whose data belongs to a controller (`VirtualList`, `Table`,
`Tree`, `FileBrowser`, `ColorPicker`, `HistoryNavigator`) keep their mounted
controller, source binding and structural options, such as a list's axis or a
table's row height, and say so; a new key replaces them. Their permissions and
callbacks follow the new description, including removal: a collection
described without its reorder callback, or with a fixed denial, cancels an
armed or active gesture at the update the re-description schedules, as a
revocation does, and a removed notification callback discards the changes it
has not yet delivered. `HistoryNavigator` retains the
history it mounted with because its generated buttons, tooltips and list derive
from it; `HistoryButtons` generates no parts and follows the history it was
described with last, forgetting a press held across the handoff.

A re-description is applied without validation, because `reconcile` cannot fail:
its bindings are checked by the next update exactly as at mount, and an invalid one, such as a signal
from another runtime, fails that update and every later flush, keeping the
component mounted with its previous presentation until a valid description
arrives. A component can instead reject a description up front, from what the
description alone shows, through `Component::check_description`: the call that
mounts or reconciles it then fails before anything changes, so an invalid
description cannot unmount children it fails to list (the tool window layout
uses this for its keys and edges).

A value a description carries from application data, such as a controlled
control's value, is the exception to rejecting what a description shows up
front: the control refuses an invalid value at adoption, at mount or at
reconciliation, and keeps its last valid presentation, as for reactive data,
and the call does not fail. Otherwise one record with an invalid value would
fail every flush in which a collection describes its rows. A controlled
control described again with the same value keeps its draft and pending
input; see [Controlled controls](#controlled-controls).

#### Value-driven children

`Dynamic`, `Show` and `Each` describe their children from a binding's value
through `UpdateContext::request_children`, so they are self-managed: an ancestor
redescription applies their new source and description callback but never
gives them children. They describe children again when the binding was
written with a new value, judged by its write counter rather than by comparing
values, or whenever they are described again, so a new callback takes effect
for an unchanged value. Only the source binding is tracked: a signal the
callback reads with `get` adds no dependency, so a description that depends on
several values combines them into one binding. Their children reconcile by key like any structural update:
a kept key keeps its component, state and focus and must keep its component
type, a new key mounts, and a removed key unmounts with its subscriptions.
`Show` therefore releases its child while hidden, unlike `View::visible_signal`,
which retains it. The description callback runs during update, where writes
are rejected; an invalid description fails the flush and keeps the previous
children until a later value or description is valid. `Each` keys each item's
content, a component or an element, with the key it gives and places the items
in a container it describes, a `Column` unless `within` says otherwise. The
default `Column` is internal structure (`Element::structural`): it adds no segment to an item's
address, which is the `Each` element's address followed by the item's key, and
like any structural wrapper it is transparent to naming, so an item the
application marks structural takes its name from the `Each`'s label or a
naming parent above the `Each`. A container given with `within` is used as it
is, and its key, `items` included, is a segment of the items' addresses. The
container's addressing is decided when it first mounts: a new description that
keeps the container's key keeps its component and its addressing, and to change
it the new container takes another key, which mounts the items again.

#### Controller ownership

A `ZoomController` likewise serves one mounted view at a time, including a
hidden one: the view records its arrangement for `fit` and forgets it when it
is unmounted or handed another controller, so sharing a controller between
simultaneously mounted views is unsupported, as it is for reveal controllers.

ColorPicker's convenience-owned popover retains its initial anchor, picker and
open state together through this contract. Same-key redescription keeps their
binding/options and active draft/gesture coherent. Changing binding/options or
switching between convenience ownership and ordinary external Popover ownership
requires a new key. Ordinary Popover construction adopts replacement external
open signals and reconciles incoming children.

#### Menus, pickers and dialogs

Menu descriptions with internally owned open state retain the current engagement.
Reusing an externally supplied signal also preserves it. A different explicit
external open signal replaces both owner and surface state, is read as supplied,
and discards the old captured command context. Incoming trigger/row callbacks
follow the selected mounted signal; the replaced signal receives no later writes.

QuickPick and CommandPalette keep active configuration for one presented session.
Redescription replaces pending settings for the next controller opening, including
when it supersedes a visible session. Pending failure retries keep the same
snapshot; role-based texts still follow installed strings. Choose callbacks and
CommandPalette outcome formatters are replaced for subsequent choices, except
when reentrant redescription retains the active choose callback. Already Pending
choices retain their captured formatter. An accepted redescription can therefore
leave current presentation unchanged.

A still-visible surface whose declared kind changes is admitted again under the
new kind's conflict/cancellation/focus rules, retaining the original opener.

An admission that an error interrupted is retried with the surface's current
kind. A pending ApplicationDialog blocks other windows and targetless command
queries before admission. A downgraded ApplicationDialog releases its
application-wide singleton. Command queries that do not flush judge blocking by
current root visibility and declared kind, not by the kind admitted earlier,
including local Dialog/ViewDialog scope and default application-command
restrictions. Hidden logical dialogs and downgraded surfaces stop blocking;
hiding only the native application-dialog window suspends its session and
continues blocking other roots.

Command queries inspect current mounted surface declarations, including newly
shown roots that no flush has synchronized yet. They do not evaluate pending
component updates or run admission callbacks. Command invocation and native key
routing flush all presentation roots before admission.

### Permissions and availability

A capability the user may exercise, such as resizing a divider, is a
`Permission`: allowed, denied or bound to a `ReadSignal<bool>`. Permissions
only restrict. Several settings on one description, and a container's
setting combined with an item's own, compose with `Permission::and`, so the
stricter wins and a fixed denial cannot be lifted by a bound permission. A
component reads its permissions in update, so a bound permission takes effect
at the next flush; revoking one while the user exercises it cancels that
interaction as the component documents (a divider restores its preference).
Table resize/reorder restrictions and VirtualList/Tree reorder restrictions use
this policy without creating capabilities: column flags and list callbacks remain
required. Revocation cancels armed/active reordering before data changes. An
active table resize restores its original fixed width or flex weight and user
preference membership; ordinary Escape keeps the existing dragged-width policy.
Selection, expansion, sorting and scrolling remain independently available.

A capability the component provides itself is restricted by the adjective
taking `impl Into<Permission>` (`SplitPane::resizable`,
`ToolWindowLayout::reorderable`). Where an application callback or a per-item
flag creates the capability (`VirtualList::reorderable`,
`TableColumn::resizable`), the adjective creates it and the restriction is
`*_permission` (`VirtualList::reorder_permission`, `Table::resize_permission`).
The argument type tells the meanings apart.

Which title-bar parts a `FloatingWindow` has (`decorated`, `closable`,
`minimizable`) and `App::resizable`, an attribute of the native window, are
ordinary `bool` properties. They choose parts rather than restrict a
capability, and `WindowController` still closes and minimizes from code.

Basic controls, choice controls and menu items use `enabled(impl Into<Permission>)` for
activation/removal availability.
`disabled()` adds a fixed denial; a later binding cannot lift it. Effective
parameter editability remains an additional restriction. Fixed redescription
denial clears armed input immediately; bound revocation is observed at update.
Redescription replaces the description's permission,
so a new enabled description can unlock a previously disabled component.
MultiSelect is a retained composite: its rows and chips observe permission changes
within a successful flush. Chip descriptions superseded before that update are
not applied individually to their input state. Row source replacement refreshes
subscriptions and cancels held row input through an event-phase notification;
notification publication errors propagate. No timer or polling is involved.

Ordinary property builders replace earlier values; Permission builders combine
restrictions. Editors take `enabled` and `editable` Permissions (`disabled()` and
`read_only()` are fixed shorthands); a bound change is observed at update and
cancels like the fixed setting.

Revoked enablement releases focus and cancels
composition, selection drags, held keys and step repeats; a parameter session
is cancelled, never committed. Revoked editability keeps focus, selection and
copying, cancels composition and editing gestures and saves nothing. What
either does to an editor's draft is the [draft policy](#draft-policy)'s.
InlineText has no separate read-only state.

Slider, Knob,
InlineSlider, ColorSwatch, ColorPicker, Collapsible and Accordion take an
`enabled` Permission: revocation cancels a gesture without committing, an armed
activation or an armed header press without toggling; ColorPicker's permission
follows same-key redescription although its binding and options stay retained.

History controls and geometry controls take a constant or a bound signal through
one `Permission` builder each, and `FloatingWindow::movable` restores the
position when revoked during a move.

Detachment is a Permission on the element:
the runtime evaluates its current value when a detach is requested, a container
reads it through `ChildInfo::detach_permission` to follow a binding, revocation
during a drag-out forbids the drop and an already detached view keeps its
window. Tool-window drag permission revocation forbids dropping the active drag,
rather than ending the drag session; resize revocation restores geometry.

### Geometry notifications

Committed geometry is observable live through callbacks with a `ChangeOrigin`:
`ScrollArea::on_scroll`, `SplitPane::on_resize`, `RegionLayout::on_resize`,
`PropertyGrid::on_resize`, `FloatingWindow::on_frame` and
`ToolWindowChangeKind::Geometry`. They run in an event phase after layout
settles, at most once per flush and component (once per divider for a
`RegionLayout`, whose dividers report separately) with the last value and origin,
only for committed values: previews and cancelled gestures report nothing, and
neither does an unchanged value. A description replaces or removes the
callback; one without a callback discards changes not yet delivered; a failing
callback keeps the change and is not replayed. The values stay inspectable and
persisted as before.

### Container composition and validation

Tab, Section, BreadcrumbSegment and Collapsible `leading_icon` builders are
replaceable slots like the control slots above: a second call replaces the
icon, and a rich title replaces the title while keeping the slot. `View::fixed`
creates one viewport however often it is called. A unified title descriptor is
deferred rather than changing these public compositions solely for symmetry.

Tabs, Accordion and PropertyGrid check the tabs, sections and rows they are
described with before anything mounts or changes: a blank key, a duplicate key
or a blank label fails the mount or reconciliation with the typed
`LayoutError` (`BlankKey`, `DuplicateKey`, `BlankLabel`), and so does an
initial `Tabs::selected` key that names no described tab (`UnknownKey`).
Collapsible refuses a blank title the same way, and `Runtime::reveal_child`
reports a key its container does not have as `UnknownKey`. Children handed
over as raw elements keep their update-time label check.

A dropdown whose value becomes read-only, or whose description becomes
disabled, while its list is open keeps that list until it is dismissed. The
list cancels active presses, shows its rows disabled and refuses choices.
Escape and an outside press still close it. Select, InlineSelect and
MultiSelect follow this. A MultiSelect closed in that state writes no
`OnClose` draft. Reenabling before close permits the draft to commit under the
usual external-write conflict checks; temporary denial does not itself erase it.

### Layout, visibility and focus

Composite hard minimums propagate before allocation. Container includes its
visible child's minimum and padding; explicit outer maximums cap that minimum
and clip conflicting child content. A smaller preferred size does not override
the propagated minimum. Row retains the parent's offered height for ordinary
children; a decoration can instead fill the resolved row content up to the
offered height ([layout hooks](#layout-and-focus-hooks)). Grid resolves fixed and weighted track widths before wrapped row
heights in both measurement and arrangement, including unbounded measurement.

Row and Column give every child its natural size first, then divide the space
their own allocation leaves over among children with an `Element::with_grow`
weight, in proportion to the weights and up to each child's maximum, sharing
what a maximum stops among the others. When a row's width or a column's
height is bounded and its children's natural lengths do not fit, the grown
children give up the difference instead, in proportion to their weights and
down to each one's minimum, so a grown child fills what the others leave; the
container then overflows only past the ungrown children's lengths plus the
grown children's minimums (for a row, the children's final widths where they
were not shrunk: any unshrunk fill child, grown or not, counts with its width
at its fill height, which may differ from its natural one, and every other
ungrown child with its natural width). A row measures each child it shrank
again at its final width and takes its height from those measurements, so a
child that wraps gets the height of its new line count; a column's shrunk
children keep the widths they were measured at. A fill child is still
measured at the row's resolved height, capped by the offered height, and its
result counts in the row's height. Whether a row shrinks, and
by how much, is decided once from the natural widths; the row's reported
width, its overflow and the space `Justify` places are computed from the final
allocations. A fill child the row did not shrink is allocated the width it
reports at its fill height; one the row shrank keeps its final width. When the
natural widths fit, growing children share the space left after the final
allocations, as before; after a shrink no child grows back, even if a fill
child narrowed. The runtime keeps a component's four most recently used
measurements, and every theme or font value they read stays a dependency
until its layout is invalidated; measuring and arranging such a row again
reuses them. The weight is element metadata that
reconciliation updates without remounting, and hidden children take no share.
`Justify` places the space no child takes, and `Align` places children across
the main axis: a column within its width, a row within its tallest child,
except that `Align::Stretch` in a row fills the height the row is allocated
when it is arranged. A row's measured height stays its tallest child's, so the
stretch fills what a parent gives (for example a column that grows the row)
without asking for more; children keep their maximum and minimum heights.
Without a weight or a justify choice, a row or column keeps its natural size.
Wrap and Grid do not grow children.

Containers provide flow, wrapping, grids, scroll areas, adaptive forms,
resizable splits, tab groups and regions. Nested regions share one runtime.
Inactive tabs keep local state and suspend presentation deadlines. Focus moves
to a visible target when content hides and returns to a valid target on reveal.
When a parent's new description removes the focused component, focus normally
goes to its nearest focusable ancestor; a component such as `Chip` can opt into
passing focus to its next, then previous, surviving sibling of the same type.
`EventContext::request_focus_within` moves focus to the first focusable
component inside a region whose components the caller does not know, as a
tool window does when Escape returns the user to the main content. A focus
request whose target is hidden is dropped, except while the target is hidden
only until an ancestor's update still pending in the same flush shows it, as
an editor is until its owner reveals it: the request then waits for that
update and applies when the flush settles successfully, or, after a failed
flush, at the later flush that retries the update. A focus restoration in the
meantime cancels it, but a focused component containing the target, such as
the label whose editor it is, keeps focus for the transfer even though it
stopped being focusable to make it.

Tab and Shift+Tab stop at focusable components whose `Component::tab_stop`
allows it; a component whose focus moves by other keys, such as menu-bar
titles in menu mode, is focusable but not a tab stop, and Tab continues from
it to the next tab stop in navigation order. A component whose
`Component::descendant_tab_stops` is false keeps its whole presentation
subtree out of traversal, up to the next surface or native window root, which
starts a scope of its own; `VirtualList::item_tab_stops(false)` uses it to make
a list with interactive rows one stop. Pointer presses, explicit focus and
focus restoration are unaffected, and traversal from a focused descendant
continues from its place in navigation order. A component may move focus on
Tab itself, as an editor moving to the next cell does; view, window and
application bindings never see an unmodified Tab ([layout and focus
hooks](#layout-and-focus-hooks)).

An element can opt out of native detachment; detachable tabs and explicit
runtime requests honor the same permission. Native multi-window support remains
experimental.

### User-write callbacks

Boolean controls and single-choice controls offer `on_change` after a successful
local write or remote proposal submission, not after authority acceptance; it
runs [in the input](#in-the-input).
A [controlled](#controlled-controls) `Checkbox` or `Toggle` writes nothing: its
`on_change`, like a controlled `InlineText`'s `on_save`, receives the user's
proposal where the other paths write.
Application writes, same-value choices and refused edits do not call it. Failure
propagates without rollback or replay. A redescribed handler replaces the old
one, including removal. Select closes before invoking its list-context callback;
application work queued with `EventContext::defer` runs after the list has
closed.
MultiSelect's `on_change` reports each user write of its bound value with the
values written: an EachToggle toggle, a chip removal in either mode and an
OnClose draft written by a committing close. Draft toggles, discarding closes
(Escape, hidden or removed owner, no space, modal conflict), unchanged or
conflicting drafts and refusals while denied never call it. A failing handler
keeps the write and the close, and is not replayed. Only the owner's
redescription replaces the handler; an open popup and its draft are kept.
`MenuItem::on_change` on a toggle or radio row reports the row's new checked
state after its own write, once the menu has requested closure, with the
activating list's or title's context; choosing the selected radio row again
writes and reports nothing. Rows without a write of their own ignore it.
`ColorPicker::on_change` reports each committed user write that changed the
bound color, after the binding and its text fields hold it: a pointer
release, a key press or repeat, or a saved hex or channel field. Edits that
leave the color unchanged (a new hue for a color without saturation keeps the
bound value exactly), application writes, cancelled or revoked gestures and
refused text report nothing. The handler follows same-key redescription,
also in the popover form, whose retained picker otherwise keeps its binding,
options and drafts.

Parameter sinks run [at parameter delivery](#at-parameter-delivery) and
cannot write to the runtime. A sink that must update bindings
sends to an application-owned `Mailbox`, whose handler runs in a later flush;
the recommended shape is a capacity-one refresh mailbox whose handler rereads
application state, sent to as the sink's last step after it recorded the
outcome, with a full mailbox meaning a refresh is already pending.
`ParameterHost::write_base` delivers the edit it ends before returning and
calls no sink otherwise; the writer forwards its own writes. No general signal
observer exists: these per-source callbacks cover user writes, outcomes and
the application's own writes.

## Callback timing

A callback the runtime calls later, or during an event, runs on the UI thread
at one of a few moments: its timing class. The class says only when the
callback runs. What it may do, whether changes are coalesced, what cancels it
and what a failure leaves behind are [separate
properties](#properties-beside-the-class), which vary within a class; each
callback's rustdoc names its class and states those properties.

| Class | When it runs | For example |
|---|---|---|
| [In the input](#in-the-input) | inside the input or focus change that caused it | a button's action, `on_change`, `on_save`, `on_activate` |
| [After settling](#after-settling) | in an event phase once the flush's updates and layout have settled | `Tabs::on_select`, `on_selection_change`, `on_scroll`, `on_modified`, `Table::on_edit_end` |
| [At the next flush](#at-the-next-flush) | at the start of a later flush, before components update | deferred tasks, mailbox handlers, background results, interval ticks |
| [At parameter delivery](#at-parameter-delivery) | when the runtime delivers a parameter edit's event | `ParameterHost::on_outcome`, `RemoteHost::on_submit` |
| [Evaluation](#evaluation) | whenever the runtime needs the value it computes | `ReadSignal::map`, `Dynamic`, filter rules, number formats |

### In the input

The callback runs synchronously, in an event phase, inside the call that
delivers the input or focus change that caused it, before that call returns:
a pointer, key, text or composition entry point, `Runtime::event`, a focus
change (`Runtime::focus` and its relatives, `HostIntegration::clear_focus` as a
window deactivation or a close admission calls it), where the component losing focus
may save a draft, or a component's own timer, such as a held step button's
repeat, in the flush that reaches its deadline. Its error propagates from that
call, except where the call has no error path for it: `clear_focus` queues a
`focus_lost` error, such as a failing save callback's, in its window and a later
flush returns it, and a timer's error comes from the flush that ran it. A
control's callback runs after the control's own
write or proposal submission; a [controlled](#controlled-controls) control
writes nothing and calls it with the user's proposal at the moment the
signal-bound form would write. A `Select` list and a menu row call it once
their surface has started to close, and a picker closes right after its
choose callback answers, so work that must outlive the surface is queued with
`EventContext::defer`, which runs after the surface has closed ([calling the
runtime from callbacks](#calling-the-runtime-from-callbacks)).

### After settling

The callback runs in an event phase once the update and layout waves of the
flush that follows the change, or of the flush in progress, have settled,
through `Component::deliver_notifications` ([component
notifications](#component-notifications)), so it sees the settled state.
Callbacks run in the order the changes happened, across components. An input
entry point or a reveal operation settles a flush of its own, so a change it
causes is delivered before it returns while its window is visible; a hidden
native window holds the callbacks until it is shown.

### At the next flush

The callback runs at the start of a later flush, before components update:
work queued with `EventContext::defer`, `EventContext::defer_while_shown` or
`Runtime::defer`,
the start of `EventContext::spawn` and a command invocation queued with
`EventContext::invoke_command` run in the next flush; a mailbox handler or a
`Runtime::spawn` result runs in the first flush after its value arrives; an
interval tick runs in the first flush at or after its due time, or in
`HostIntegration::service_intervals` for a host that cannot flush. A flush nested in a
deferred task delivers no further deferred tasks ([calling the runtime from
callbacks](#calling-the-runtime-from-callbacks)).

### At parameter delivery

The callback runs when the runtime delivers a parameter edit's event, after
the parameter has published the edit's new state: inside the call that
previewed or ended the edit when a delivery may run there, otherwise at the
next safe point, which is the next operation on that parameter or the next
flush. A remote host's submission callback runs at the same point for a
proposal.

### Evaluation

The function runs whenever the runtime needs the value it computes: while it
settles derived bindings, updates or measures a component, prepares a
control's text, or answers a query, a command availability check or an
inspection. Unless its rustdoc says how often, it may run at any of those
moments, so it must be pure: deterministic for its inputs, bounded, free of
I/O and side effects, and it writes nothing; during update, layout, paint or
inspection the runtime rejects a write. Binding derivations (`ReadSignal::map`,
and `ReadSignal::combine` over two to twelve bindings), description factories (`Dynamic`, `Show` and `Each`, a
collection's row builders and `Table::cell_editor`), filter and ranking rules,
number formats, command enabled predicates and inspection providers are
evaluation callbacks, and so are a numeric editor's normalizer and its number
format's parser, which the editor runs whenever it evaluates its draft. A text
editor's normalizer is pure as well, but runs in the input, at the save it
checks.

### Callbacks outside the classes

A few callbacks keep a moment of their own, which their rustdoc states:
`App`'s initializer, `on_close`, `on_error` and `on_window_error` run from the
native host's event loop outside a flush, and the three policies receive the
application state the initializer returned after the runtime; a command action runs at its
invocation, which is in the input for a routed chord, at the next flush for a
queued invocation and inside `Runtime::invoke_command` for a direct call; a
held-input registration's cancellation runs wherever its input is cancelled;
`Popover::on_dismiss` runs in the input that closes the surface or where the
runtime closes it (a hidden or removed owner, lost window focus, no space);
and a file browser's request callback runs during the controller's calls and
the browser's update. Host-integration callbacks, such as
`HostIntegration::set_wake_delivery`'s, which runs on the notifying thread, keep
their own threading contracts, and the `Component` hooks are placed by their
own documentation.

### Properties beside the class

Each callback's rustdoc states these properties besides its class:

- **Access:** callbacks with an `EventContext` or `&mut Runtime` may write
  bindings, and [calling the runtime from
  callbacks](#calling-the-runtime-from-callbacks) lists those that receive
  the runtime; parameter sinks and a remote host's submission callback may
  not write, and evaluation callbacks write nothing.
- **Coalescing:** `Tabs::on_select` reports each settled change, a
  collection's `on_selection_change` merges the writes between two of its
  updates, [geometry notifications](#geometry-notifications) report the last
  value once per flush for each component (each divider of a `RegionLayout`),
  and `on_modified` reports each change in order.
- **Cancellation:** what discards a callback or its pending payload before it
  runs, such as a redescription without the callback, the component's
  removal, or the component a `defer_while_shown` task is bound to.
- **Failure and replay:** what a failing callback leaves behind. As a rule
  the change it reports stays and is not reported again ([user-write
  callbacks](#user-write-callbacks), [component
  notifications](#component-notifications)); the exceptions, such as a
  parameter sink's failed delivery kept for an explicit retry or an editor's
  pending end of a refusal, are documented on the callback.

## Data and edit ownership

### Ownership paths

The paths are distinct:

| Path | Owner and behavior |
|---|---|
| `Signal<T>` / `ReadSignal<T>` | UI-local observable state. Reads register component dependencies; writes invalidate those readers. |
| `Parameter<T>` with a host | An authoritative application value with validated metadata and begin, preview, commit or cancel gestures. The host accepts or rejects edits explicitly. |
| Live data sources | Bounded, possibly lossy producer data for presentation. Consumers observe freshness, sequence and gaps; it is never an edit authority. |
| Described value with a change callback ([controlled controls](#controlled-controls)) | The application's model. The value arrives in the control's description and changes only by a new description; the control reports each user change as a proposal through its callback and never changes what it shows on its own. |

**Display controls.** A control that presents one application value and
writes nothing chooses the value's source at construction, as a choice
control chooses its options' ([choice sources](#choice-sources)): `new`
takes the value, which applies when the description is adopted and
changes only with a new description, so a collection row describes its
record's value directly; `bound` follows a `ReadSignal` from the
control's next update, and a same-key description with the same binding
keeps what was read. `ProgressBar`, `Spinner`, `ColorSwatch` and `Image`
have both. A control that presents text takes a `TextSource`, which
covers fixed and bound text (`Label::new`, `InlineReadout::new`);
`InlineReadout::formatted` formats a bound number. A presentation builder
takes the fixed value under its own name and a binding under its
`_signal` name (`Field::status` and `Field::status_signal`,
`Property::status` and `Property::status_signal`,
`Container::background_role` and `Container::background_role_signal`),
each replacing the other. `Permission`-typed builders follow the
permission rules instead; bindings that switch presence or work
(`View::visible_signal`, `ToolGroup::visible_signal`,
`Banner::visible_signal`, `LevelMeter::running_signal`,
`Trace::running_signal`) and an action row's mark
(`MenuItem::checked_signal`, `selected_signal`) have no fixed form.

### Controlled controls

`Checkbox::controlled`, `Toggle::controlled` and `InlineText::controlled` take
a value the application owns, in their description, and a callback. They suit
data the application changes synchronously in its own model, such as the
records behind a list's rows or a table's cells, whose descriptions carry no
state owner. A controlled control has no host, revision or pending state, as
a parameter has, and no binding, as a signal-bound control has: nothing is
written by the control, and the callback is the only effect of a user change.

Presentation: the control shows the described value only. A user toggle or
save does not change what it shows; a new description with a different value
does. When the callback writes the application's model and that write
describes the control again, as a list's source replacement or a `Dynamic`
does, the next successful flush reconciles the new description before layout
and paint. A raw input call does not promise to settle the redescription its
own callback caused; painting is refused while presentation work is pending,
and the desktop host flushes before every paint, so no presented frame shows
the old value after the model accepted the new one. An application that
confirms later shows the old value until it describes the new one, and shows
a pending state from its own model if it wants one. Inspection and semantics
report the described value: `checked` for a checkbox or toggle, the resting
text for `InlineText`.

Refusal: the application refuses by not changing the value it describes.
Nothing is rolled back or replayed, because the control already shows the
model's value. An `InlineText` save ends the session before the callback
runs, so a refused save closes the editor and the typed text is gone. To
refuse while keeping the draft open, `InlineText::normalize` refuses with an
`InputRejection`, which runs before the callback exactly as with a bound
label; for `InlineText` its normalizer may read application state, never
write it, is deterministic and idempotent for a given draft and unchanged
observed state, and creates no dependency, so a refused draft is checked again
at the user's next save. A `bool` is always valid. Described text the editor
does not accept (`TextField::check` fails) is refused at adoption, as a
refused bound value is: the label keeps its previous text (empty if it never
had a valid one), draws the error outline, reports `binding_error` in
inspection, marks its semantics invalid, ends an open session unsaved and
cannot begin editing, and the next acceptable description ends the refusal.
This is the exception to validating descriptions up front that
[component redescription](#component-redescription) names.

Callback timing: the callback runs at the moments a signal-bound control
writes. `Checkbox` and `Toggle` call `on_change` at a matched primary release
inside and a matched unmodified Enter or Space release, with the negation of
the value shown; two activations before the next description propose the
same value twice. `InlineText` calls `on_save` at Enter and when focus leaves
the label, after the normalizer, with the normalized draft, only when it
differs from the described text. A new description, Escape, hiding, removal,
revoked permission, a refused draft and an inspection read never call it.
A callback error propagates from the input call, with nothing to roll back;
`InlineText` keeps its rule that a failing rejection report cannot swallow a
save report.

Redescription: the same value changes nothing, so a pending activation stays
armed and an open draft stays open; a collection may describe its rows as
often as it likes without disturbing an edit. A changed value is shown at
once: it cancels a checkbox's pending activation, so a press made against
one value never releases into a proposal against another, and ends an open
`InlineText` session unsaved, as an application write does with a bound
label. Changing the path under the same key, such as from `new` to
`controlled`, is a change of binding: a pending activation is cancelled and
an open draft ends unsaved. The callback follows each
description, including the constructor's, and a later `on_change` or
`on_save` builder replaces it.

### Parameters and gestures

Parameter identities are independent of labels, order and numeric value.
`ParameterSpec<T>` defines a validated domain and default. Normalized positions
are finite values in `0.0..=1.0`; presentation formatting does not change the
canonical value. Cloning a `NumberFormat` preserves its callback identity: an
unchanged clone on a numeric control retains layout during redescription. New
callbacks or changed format metadata request new measurement. This is an
invalidation contract, not a measured rendering-speed guarantee.
Boolean, continuous, stepped and choice families retain their
own value semantics. An external base change may supersede a local preview;
rejected or stale host edits use the explicit resynchronization path. A gesture
reserves its terminal outcome at admission, so commit or cancellation is not
silently dropped when delivery fills.

### Component notifications

Application-relevant selection and expansion state a user can change inside a
component is observable: either the
application binds it with a `Signal`, or the component reports each change
through a callback with a `ChangeOrigin` (user, reveal, workspace restore,
re-description, presentation or application), so an application can tell user
navigation from changes it caused itself. `Tabs::on_select` and
`Accordion::on_change` follow this rule. A `ListSelection` is bound state the
application can read, and `VirtualList`, `Tree` and `Table` also report each
change of it through `on_selection_change`, with both the active item and the
selected keys: a collection observes its selection when it updates and reports
the origin of the latest write, `User` for a click or a key on any collection
sharing the selection and `Application` for a write through the selection or a
controller, including a filter moving the active item off a hidden row, so writes between two updates of one collection
arrive as one change. The two names mark that difference: `on_select` reports
each settled tab change; `on_selection_change` reports a collection's bound
selection as observed at its update, coalescing writes between updates.
Committed geometry is observable too:
scroll offsets, split, region and label-column preferences and floating-frame
placement report through `on_scroll`, `on_resize`, `on_frame` and
`ToolWindowChangeKind::Geometry`, besides inspection and workspace capture.
See [permissions](#permissions-and-availability) and
[geometry notifications](#geometry-notifications).

These callbacks run [after settling](#after-settling): in an event phase
once the update and layout waves of the flush that follows the change (or of
the flush in progress) have settled. They see the new state, run in the order
the changes happened, across components too, and may write bindings, which
the same flush settles. A custom component reports its own changes the same
way ([reporting component
notifications](#reporting-component-notifications)).

Input entry points and reveal operations settle in a flush of their own, so a
change they cause is delivered before they return when its current presentation window is
visible and preparation succeeds.

A failing callback never undoes the change; the remaining notifications are
still delivered and the first error is returned from the flush. A component
removed before delivery reports nothing. Removing a callback discards
its pending payloads but retains request positions, so reinstalling one cannot
move newer changes ahead of another component's notifications.

#### Delivery order and hidden windows

Ready requests follow global FIFO order across visible presentation windows.

Hidden native windows retain requests without blocking visible windows or keeping
a deadline due; showing them resumes delivery. Hidden components in visible
windows still receive callbacks. Presentation settles again after callback
writes, without delivering mailbox values, deferred tasks or timers a second
time in that flush. Callback failures consume their request once; preparation
failures retain undelivered requests.

Reentrant requests join the next frontier. It runs in the same flush if the
preceding frontier settled presentation work, including recipient preparation;
otherwise requests wait for the next flush. Notification followups share the
update-wave budget. An exhausted notification budget is not restarted for pending
reveals; ordinary queued reveals use their normal flush budgets, so independent
requests and path depth do not count as feedback.

ToolWindows reports interleaved layout changes in request order, separately from
its housekeeping. The [implementation notes](internals.md#notification-delivery)
record how delivery is prepared.

#### Failures and input cleanup

A failing input callback, or a failing notification that its dispatch caused,
cancels that input, as `HostIntegration::cancel_input` does. Errors from older or
unrelated work, such as a request an earlier input caused, still propagate,
but without cancelling another event's focus, capture or drag. An ordinary
update failure during the native-key preflush, including in another visible
window, prevents the key's dispatch and cancels input in the selected window;
older and external notification failures keep their own preservation policy.
Which component's input a failure cancels, and when cleanup is not repeated, is
[described for component authors](#failure-cleanup-in-components).

Command actions retain their explicit failure boundary: an error returned by
the action cancels input, including a nested runtime error the action chooses
to propagate. An action
can catch an unrelated nested delivery error and remain successful. Deferred
application callbacks have the same explicit returned-error boundary; caught
nested flush errors do not classify subsequent work. Parameter delivery runs
without the current input's provenance, so a failing sink cancels no input.

### Producer data and mailboxes

A time-critical producer may use only the endpoint documented for callback-safe work.
Its bounded, preallocated path has explicit overflow and reclamation behavior.
Ordinary UI senders and worker mailboxes have different thread and failure
contracts. Mailbox handlers, background results and interval ticks run [at the
next flush](#at-the-next-flush). A flush captures all mailbox delivery frontiers before invoking any
handler; values sent between handlers wait for the next flush regardless of
registration order. Concurrent worker sends join the current delivery only when
accepted before their inbox's frontier is captured. Closing or dropping a mailbox
immediately cancels its undelivered values, including a captured delivery frontier.
An `App` initializer returns a mailbox or other application-lifetime value in
its application state, which `App` keeps until the application ends;
`Runtime::keep_alive` owns one until shutdown for code that has only the
runtime, such as a custom host, so neither needs to find another owner for it.

`Runtime::spawn` runs one unit of work per call on a new thread and delivers its
result to a UI-thread callback through a runtime-owned mailbox, in arrival
order with the other mailboxes. At most `Runtime::MAX_BACKGROUND_TASKS` tasks
wait at once; each holds a reserved place in that mailbox, so a worker's send
never finds it full. Cancellation discards an undelivered result but cannot stop
the thread; a panicking work delivers nothing and fails the flush that receives
its outcome with `BackgroundPanicked`. The work holds no UI handle, and the
helper is not a callback-safe endpoint. `EventContext::spawn` queues the same
call from a callback with `EventContext::defer`: the start belongs to the
application and runs at the next flush whatever happens to the widget or its
window, its receipt reports the start rather than the result, and only
shutdown cancels it before it starts.

`Runtime::start_interval` repeats an application callback on the UI thread. It
belongs to the application rather than a window or component, so focus loss,
input cancellation and hidden windows do not stop it; the native host runs a due
timer even while a window's redraw is withheld. A late tick does not catch up,
a failing tick keeps its timer, and at most `Runtime::MAX_INTERVALS` timers run.

Live telemetry revisions also report loss/rejection changes, but freshness and
receipt time describe accepted values: loss alone cannot refresh an old value,
and clearing retained data resets its receipt time. Notified sources may drain
again at the same runtime time when a new wake arrives; resuming a suspended
source may also change its revision without advancing time.

## Input and focus

### Physical keys and held actions

Physical held-key state is distinct from committed text and IME composition.
Multiple simultaneous keys and OS repeats are represented independently.
Native numpad keys retain distinct physical identities regardless of NumLock.
`KeyboardState::control_key` captures a native key's named control meaning at
its initial press and retains it through repeat and matching release. Controls
choose actions from that meaning and retain physical event codes for gesture
ownership. Editor delivery preserves the same named meaning, independently of
the native committed-text payload. A held named control cannot start inserting
text when NumLock/layout changes its logical meaning; unchanged printable named
keys retain text delivery. Focus loss and input cancellation clear both
physical holds and their captured meanings.

Components release held actions on focus loss, hiding and unmount. The first
pointer or keyboard modality to arm an activation owns it until release or
cancellation. Pointer capture permits a gesture to finish outside its original
hit rectangle. Escape and context loss cancel temporary interaction according
to the component's contract.

### Stacking and hit testing

Overlapping content draws and takes pointer input in one order, which hover,
cursor, focus and drag targeting share; logical navigation, layout and retained
identity do not change with it. ToolWindows keeps center/docked content below
overlays and floating frames, and allows floating frames to raise among their
peers. Surface outside-press dismissal uses the surface root's clipped bounding
rectangle, including transparent body gaps. ToastHost uses actual toast
rectangles so transparent gaps reach content behind it. Toast selection follows
stable IDs and picks the first surviving following toast, then nearest
surviving preceding toast, then first newly visible toast. Custom stacking
parents and hit regions are [described for component
authors](#stacking-hit-testing-and-pointer-refresh).

### Activation and navigation keys

Default activation starts with unmodified Enter or Space. Modified activation
keys are left to command bindings while idle. The matching release of an armed
activation still completes it if modifiers changed after the press; repeats and
unmatched releases never activate it. Command repeats require that command to
own the admitted physical press; a repeated event cannot start a new command
because focus or modifiers changed while a component owned the press.

Navigation keys (arrows, Home/End, Page Up/Down) follow the same admission.
A new press acts only unmodified or with the modifiers its component documents.
Examples are Shift for fine steps or range extension, Control for moving
without selecting, and Alt for reordering or moving a frame. Other chords are
left to command bindings. Repeats and the release of a key the component
already owns keep working when modifiers change. Text editors keep their own
text chords, such as Control+Up to the start of a field.

### Commands and modal admission

Commands have application, window or view scopes. Physical key identities and
localized text remain separate. The library's own words come from the
installed `UiStrings` table, read by components in their update, so installing
another table invalidates only its readers; developer-facing text (error
messages, inspection property names, shortcut key names) stays English, and
the library ships no locale data. Modal dialogs trap focus in their scope;
nonmodal surfaces and tooltips have different dismissal and hit rules. A
nonmodal surface closes when focus moves outside its owner, or out of the
surface after having been inside it, including to the surface's own anchor.
Escape first dismisses shown tooltip help in the window and is consumed by
that; only an Escape with no help shown reaches other surfaces, the focused
component and command bindings.

Modal-blocked key bindings yield to admitted dialog input and do not own the
physical press. A disabled binding still reserves its chord, and commands
explicitly permitted in modals retain that admission. Direct and queued targeted
text commands validate the target's current modal scope before editor mutation
or clipboard access. A blocked queued request reports `ModalBlocked` from the
flush, is spent, and leaves later requests queued. A text command never moves
focus, except `TextCommand::BeginEdit`: an editor with a resting state answers
it as its own Enter/F2 does, beginning the edit and requesting focus for its
editing part, and answers `Unavailable` wherever that key does nothing. The
request settles at a flush: at the next one after the synchronous
`Runtime::text_command`, and at the end of the delivering flush for
`EventContext::request_text_command`.

#### Native and headless key entry

A native host delivers keys through `Runtime::command_key_input`, which routes
command bindings and held-input registrations before components.
`Runtime::key_input` is the headless entry point that routes neither; a test
that claims what an application's user sees from a key must use
`command_key_input`. A menu
closes before invoking its selected action. Tooltips never take focus. For
component-specific keyboard behavior, use the [manual](manual.md) and rustdoc.

### Reveal and focus

A reveal makes a possibly hidden target visible. `Runtime::reveal` opens the
containers on the target's presentation path from the outermost inwards through
the public `Component::reveal_child` hook, settles update and layout after each
change and revalidates the same `WidgetId`, then scrolls with the focus reveal
path. `Runtime::reveal_and_focus` also focuses a focusable target; a
non-focusable one is revealed without claiming focus. A container changes only
state it owns, through its ordinary transition (`Tabs` selects the tab, a
`Collapsible` or `Accordion` expands the section), and the default hook
refuses. Visibility the application binds, explicit
`Runtime::set_visible` hiding and modal blocking are typed refusals, and opening
the path is not atomic: containers opened before a refusal stay open. A target
the reveal unmounts is reported, never resolved again by key. A component
queues a reveal with `EventContext::request_reveal` or
`request_reveal_and_focus`, whose receipt always reaches a terminal status, and
a target in another native window gets a requested activation, not a
guaranteed raised window. Collections reveal rows
by key and report the mounted row's identity, since a row has none before it
mounts. Search-and-jump uses one command per destination rather than a
destination registry.

Historical reveal refusal Debug records immutable runtime-local identity,
omitting the handle's live mounted state. The same formatting propagates through
outcomes, terminal statuses/receipts and driver errors after unmount or runtime
drop; standalone WidgetId Debug remains live. A host-hidden native window
returns the operational `RuntimeError::Hidden`, including through queued reveal
receipts and content jumps. Retry once the host reports the window visible;
this differs from an explicit `set_visible` refusal.

### Content search

Content search reads what components already report: their semantic names and
values, under the inspection phase, and the rows a collection such as
`VirtualList` offers beyond its mounted children ([content search
hooks](#content-search-hooks)). It does not read semantic items
(`Semantics::items`), the rows a component draws itself, such as a toast
stack's toasts or a table's column titles. Lists expose their current bound
source even when hidden after an earlier presentation, and search reads
current source rows without updating, laying out or mounting presentation.
After a requested reconciliation of a collection's rows fails, search skips
its retained row children until a later child request succeeds.
Unavailable bound rows propagate their typed read error, rather than exposing
obsolete children. It keeps no index and schedules nothing; a held cursor retains
the owned row sources it has visited until it is dropped. A search is
a cursor whose every step is bounded by a work budget (one unit per component
examined while finding the window roots, per component visited, per mounted
row indexed and per row examined) and a result limit, so a search with no
matches over a large tree is bounded per call; a mount, unmount, changed
searchability or a replaced source of any collection the search has started on
makes the next step `Stale` instead of returning results from a tree that no
longer exists. Modality is judged in each component's own window, and a
collection a modal surface blocks offers no rows. A subtree excluded with `Element::with_searchable`,
the value of a protected component, components a modal surface blocks and rows
a source does not describe are never found.

A jump reuses the reveal path and reports its typed outcome; a row is revealed
by key through the collection
first.

### Command palette

A command palette lists an owned snapshot of the commands its captured context
reaches and ranks it with a pure, replaceable matcher. Choosing a row closes
the palette before the command is queued with the captured context, so a modal
palette never blocks its own command or hides the destination it reveals;
invocation revalidates scope, modality and enabled state when it runs. A
command that did not run, including a destination whose reveal was refused
and reported as the action's error, reopens the palette with its outcome
instead of being skipped, and the controller keeps the receipt.

### Drag and drop

Drag and drop is a runtime session between components in one window. A source
publishes an application-typed payload and allowed operations. The runtime finds
the target while the source holds capture; both sides receive a terminal
outcome. A refused drop, cancellation (including removal) and a release that
nothing claims, over no target or outside the window, are distinct outcomes.
Built-in list, tree, tab and table reordering build on this contract; the
application still owns its collection order. Dragging to another
application is not part of this contract.

### Closing a window

A close request, from the window system or `Runtime::request_close`, is
admitted before `App::on_close` runs, so the close policy sees what the user
typed. The windows an accepted close discards are affected: a detached
window's own, or every open window for the main window, which ends the
application. Admission releases each affected window's focus through
`HostIntegration::clear_focus`, as a deactivation does, and selects the previously
selected window again; each recipient's `focus_lost` runs and queues its error
in its window, and the runtime-wide held keys and held-input registrations end.
Each focused editor makes the [draft policy](#draft-policy)'s focus-loss
save attempt, which closing does not change: a local draft its rules accept is
saved, a refused one stays, a parameter draft is cancelled, and an
`EditorSession` reports `FocusLeft` with the outcome unless the edit already
ended. Drafts kept elsewhere, such as by a disabled or unfocused field, are
untouched; the editors' `on_modified` reports them, so a close policy can
count them.

When no window is in component-error backoff, admission then flushes the
affected windows: the detached window alone, or every window through
`HostIntegration::flush_windows`. A failed `focus_lost` aborts the close through `on_error`,
without replaying the request; a hidden window is not flushed. While any window
backs off nothing is flushed, so closing never requires a failing update to
succeed first, and `App::run` returns the pending error when the application
ends. A detached window closed while it waits to retry keeps that error for
`run` to return at exit.

The policy runs with the closing window selected. On a synchronous `KeepOpen`,
each affected window whose focus admission released focuses that component
again when nothing in the window holds focus after the callback and
`Runtime::can_focus` still admits it; a hidden, unmounted, unfocusable (such as
disabled) or modal-blocked component leaves focus cleared. The affected windows
are flushed again first, so what the policy changed applies before that check;
since that flush and focusing's reveal can retry a failing update, nothing is
restored during backoff. `Defer` and
`Close` restore nothing. Whatever the decision, the native
host forgets the component an affected window recorded to focus on
reactivation. `HeadlessApp::admit_close` runs the same admission without a
window; coalescing repeated requests while a decision is deferred, the
receipts and applying the decision stay with the native host. A host other
than `App` follows [its own steps](#custom-hosts).

### Runtime shutdown

Runtime shutdown cancels adopted sessions and accepted pending requests while
participants are mounted. Targets and observers receive Leave before the source's
Cancelled outcome. Replacement drags are refused during teardown. Cleanup
continues after failure; its first error per cancellation operation and previously
queued cleanup errors remain typed in `HostIntegration::take_shutdown_errors`, including
when their native window is retired. Outside shutdown, retirement appends queued
cleanup causes to a surviving window rather than discarding them. Terminal
OwnerRemoved failures are queued individually and do not retry the removed owner;
an earlier coalesced surface-dismissal cause remains observable. Within a window,
focus/terminal-removal errors keep FIFO order before that stored surface cause.
A visible surviving window stays due until delivery; no global chronology across
windows or error kinds is promised. The native App host drains that report even after initialization or event-loop
dispatch failure, and [custom hosts](#custom-hosts) drain it themselves. If
cleanup failed,
AppError::Shutdown retains the original terminal result and all cleanup causes
in report order; otherwise the original result is preserved. Terminal cleanup
does not enter component retry policy after the runtime has closed.

Shutdown stops interval timers and discards background results still on their
way, without waiting for running work, and drops values kept with
`Runtime::keep_alive` last, after components have unmounted and parameters have
ended their edits, in the reverse order of keeping.

### Calling the runtime from callbacks

Components never receive the `Runtime`: their callbacks get an
`EventContext`, `UpdateContext` or a paint or layout context, and a signal
write during update, layout, paint, inspection or delivery fails with the
matching `WriteDuring…` error. These callbacks do receive the runtime:

| Callback | Access | May call |
|---|---|---|
| Deferred task (`EventContext::defer`, `EventContext::defer_while_shown`, `Runtime::defer`) | `&mut Runtime`, during the next flush | Any operation, including those that flush |
| Command action (`Runtime::register_command`) | `&mut Runtime`, during invocation | Any operation, including those that flush; invoking itself is a `BorrowConflict` |
| Interval tick (`Runtime::start_interval`) | `&mut Runtime`, in a flush at or after its due time, or in `HostIntegration::service_intervals` | Any operation, including those that flush; a flush nested in a tick does not run that tick again |
| Command enabled predicate (`Runtime::command_enabled`) | `&Runtime` | Read-only queries; it cannot change state |

`App`'s initializer and policies also receive `&mut Runtime`, always outside
a flush, as their rustdoc describes. The initializer (`App::new`) runs before
the native event loop starts, or in `App::headless`; `on_close`, `on_error`
and `on_window_error` run from the native event loop, and `on_close` also
from `HeadlessApp::admit_close`, which has no event loop. The three policies
also receive the application state the initializer returned, by `&mut`
after the runtime, one call at a time; none runs when the initializer
failed. The state is opaque to the library, which keeps it and never reads
it, and it is dropped after the runtime has shut down, on every path.

These operations flush internally, settling update and layout before they
return: `flush` and `HostIntegration::flush_windows`; the input entry points
`key_input`, `command_key_input`, `text_input`, `HostIntegration::text_preedit`
and `pointer_input`; `focus`
when it scrolls an ancestor to reveal the target, and `focus_next`; `reveal`,
`reveal_and_focus` and `reveal_child`; and `invoke_command`, which flushes
every window before admission. A flush nested in a deferred task delivers no
further deferred tasks: tasks queued before delivery started run in order
after the current one, and tasks queued during delivery wait for the next
flush. Likewise, commands queued while queued commands are delivered, and
reveals queued while queued reveals run, wait for the next flush. A nested flush
does deliver due timers and mailbox messages.

The three calls share one queue, delivered in enqueue order, with
`Runtime::MAX_TASKS` places across them. They differ in what cancels a task
before delivery and in the window selected while it runs:

| Call | Cancelled before delivery when | Window selected while it runs |
|---|---|---|
| `EventContext::defer` | the runtime shuts down | its widget's window while the widget is mounted, wherever detaching or redocking moved it; once the widget is gone, the window it was queued in while that window is open; otherwise unchanged |
| `EventContext::defer_while_shown` | its widget is unmounted or not effectively visible (hidden itself or through an ancestor, on a closed surface, in a hidden native window), or the runtime shuts down | its widget's window at delivery |
| `Runtime::defer` | the runtime shuts down | unchanged: whichever window the delivering flush has selected |

`EventContext::defer` is the deferral for application callbacks, including
those on surfaces that close before the next flush (a `Select` change, a menu
row, a picker's choice, a dialog's Save button) and in detached windows that
the callback closes or redocks: the work runs after the closure, and an
operation whose targets are gone reports their error, such as `Unmounted`,
from the flush instead of disappearing. `defer_while_shown` serves a
component's work on itself, which ends with the component as its timer does.
Its `ShownTaskReceipt` is `#[must_use]`, since nothing else reports that
cancellation; the `TaskReceipt` the other two calls return may be dropped, as
`cx.defer(...)?;` does.
Routing never cancels a task or fails; a window that is not open is skipped,
and after the task the earlier selection is restored while its window is
open. Work that must address the window the event came from captures
`EventContext::native_window` and selects it in the task.

These rules decide only whether a queued task is delivered. Once delivered, a
task is executing and nothing cancels it: if its action calls
`Runtime::shutdown`, the shutdown cancels the tasks still queued and refuses
new ones, and the executing task records its own result, `Succeeded` or
`Failed`, when the action returns. If the action panics, its receipt reads
`Cancelled` as the task unwinds, and the tasks behind it stay queued until the
next delivery or shutdown. From the moment shutdown begins, every call refuses
with `Closed`, including from cleanup callbacks that shutdown runs, so no
queued task's receipt is left `Pending` after `Runtime::shutdown` returns. A
task whose action is executing, and calls `shutdown` itself, keeps `Pending`
until its action returns and records its own result.

## Text and draft editing

Literal U+0009 remains canonical text: shaping and Unicode analysis receive its
original bytes, preserving advance, wrap/bidi fields, grapheme/word boundaries
and valid offsets. Tab-only atoms produce no ink; a registered font's atom that
combines tab with printable text keeps its inseparable coverage. There are no
aligned tab stops, and keyboard Tab navigation is unchanged. Other unsupported
characters keep fallback/missing-glyph ink.

TextLayout's canvas keeps the logical box and retained non-tab raster bounds,
including existing empty-raster pen points. Removing a hanging tab's former box
may shrink width and translate public RTL caret/selection x positions through
the changed origin. Hit/word testing uses that same coordinate mapping; logical
analysis positions and vertical origin/baseline remain unchanged. Hanging-
whitespace geometry can fall outside size(). The current TextArea scroll clamp
cannot reveal every such caret; this separately verified limitation is recorded
for follow-up, not hidden by normalized-coordinate comparisons.

`TextField` and `TextArea` edit UI-local text with selection, clipboard and IME
composition. `DraftEditor` embeds a text session within an existing component;
numeric and inline controls can show formatted values while keeping the same
mounted identity during an edit. Preedit is separate from committed text.
Escape cancels a draft; confirmation parses and validates it before committing.
External binding changes, focus loss, revocation, hiding and close admission
follow the [draft policy](#draft-policy).

An editor's `on_rejection` reports each refused save with an `InputRejection`,
whatever refused it: the application's normalizer, a `TextField` or
`TextArea` length limit, or an integer or decimal editor's own parse, range or
step check. The library's own refusals take their messages from the installed
`UiStrings`. `None` follows once when a reported refusal ends.

A field refuses a value the application writes that it cannot hold, keeps its
previous text and accepts no edits until the binding holds acceptable text
again. `on_binding_refusal` reports such a refusal and its end in an event
phase after the flush settles, as `on_modified` reports a draft's state; it is
never a user-write callback, and refused typing and normalizer rejections do
not call it.

### Draft policy

These rules bind the built-in editors: `TextField`, `TextArea`,
`SearchField`, `InlineText`, `IntegerInput`, `DecimalInput`,
`InlineInteger`, `InlineDecimal`, `InlineSlider`, and the `Table` as a
session host. A custom component that owns a `DraftEditor` keeps its own
policy: `DraftEditor` keeps its session when hidden, and an owner may, for
example, discard on focus loss a draft it refuses on Enter.
`DraftEditorController::set_read_only` lets such an owner follow its own
editing permission without ending the session.

A **draft** is text or a value the user changed in an editor that the editor
has not written, saved or proposed; it may arise without focus, through a
text command. A **local** editor is bound to a signal or shows a controlled
value; a **parameter** editor is bound to a `Parameter` (see [parameters and
gestures](#parameters-and-gestures)). A **built-in in-place** editor shows
its value at rest and a draft only while an editing session is open:
`InlineText` and the text mode of `InlineInteger`, `InlineDecimal` and
`InlineSlider`.

A **save attempt** is the editor checking a draft and then writing, saving,
proposing or committing it, or refusing it. These inputs initiate one, and
nothing else does:

- **Enter**, a fresh press outside IME composition, in `TextField`, the
  number fields, `InlineText` and an inline row's text mode. In `TextArea`
  Enter inserts a line break and initiates nothing; a `TextArea` saves only
  on focus loss.
- **Tab and Shift+Tab** in a field with an [editor session](#editor-sessions).
- **Focus loss**, including window deactivation and close admission.
- **A step** of a local field without a session (a step button, held or not)
  and an inline row's resting step or drag release, each a save of the
  stepped value.

With a session, Enter and Tab while a composition or a selection drag is
active are kept without a save attempt and without a report, and Escape ends
only the composition or drag. Without a session, Enter during a composition
commits the composition, not the draft.

1. **Saving.** A save attempt saves a changed local draft its rules accept;
   focus loss cancels a parameter draft instead, which commits only on an
   explicit commit. Nothing else saves, commits or proposes: no revocation,
   re-enabling, hiding, removal, description or text command.
2. **Refusal.** A save attempt the editor's checks or the normalizer refuse
   writes nothing and keeps the draft for correction, with the error
   outline, whichever input initiated it; `on_rejection` reports it where
   the editor has one, and a session receives `Refused`. A built-in in-place
   editor keeps its session open after a refused focus-loss attempt. A
   parameter's protocol refusal is not this rule: it replaces the draft with
   the authoritative value (below).
3. **Application writes win.** A changed bound value, a changed controlled
   value or a parameter base or metadata change discards a draft unsaved; a
   session's next completion reports `Unchanged`. Writing the value the
   editor already shows changes nothing. The draft stays; an application
   that must end it, such as a form's Revert after writing the applied
   values, sends `TextCommand::DiscardDraft`, to every field of a form with
   `FormFields::discard_drafts`.
4. **Revoked editability** keeps focus, selection and copying, cancels a
   composition and editing gestures (a selection drag, a held step, a pressed
   stepper), keeps the draft unsaved and refuses every save attempt, with
   `Refused` to a session. An open built-in in-place session becomes
   read-only and editable again when editing is allowed, keeping its draft,
   selection and undo history; ending a held step keeps a typed or stepped
   draft.
5. **Revoked enablement** releases focus, if held, as cancelled input, not as
   focus loss, and cancels a composition and gestures. A local editor that
   shows its draft at rest (`TextField`, `TextArea`, `IntegerInput`,
   `DecimalInput`) keeps the draft unsaved, whether or not it held focus;
   re-enabling makes it editable and replays nothing; a session receives
   nothing and its edit stays open. The exceptions: a parameter draft is
   cancelled (authority); a built-in in-place session ends unsaved, as Escape
   ends it, since at rest the editor shows the value and has no disabled
   editing state; `SearchField` discards a pending query, since its owner
   shows results for the committed query.
6. **A revoked editor reports nothing to a session**, also when a focus loss
   reaches a field a description disabled before the next flush applied it.
7. **Hiding and removal** discard a draft unsaved and cancel a parameter
   draft, focused or not; a session receives nothing, since its host hid or
   removed the editor or is hidden or removed with it. Cancellation of input
   for any other reason (a pointer gesture's loss, an input failure, a
   geometry change) never discards a local draft.
8. **A pending search query exists only while the field holds focus.** An
   edit a `SearchField` receives without focus, such as a text command, is
   written at once, as without a debounce, when editing is allowed; focus
   loss writes a pending query when editing is allowed and otherwise
   discards it. So no `SearchField` holds an unwritten query without focus.
9. **Close admission adds no rule.** It is a focus loss for each affected
   window's focused editor (rule 1); drafts kept elsewhere (rules 2, 4 and 5)
   are untouched, which rule 10 makes visible to a close policy.
10. **Every built-in editor that can keep a draft without focus reports it.**
    `on_modified` on `TextField`, `TextArea`, `InlineText`, `IntegerInput`,
    `DecimalInput`, `InlineInteger` and `InlineDecimal` reports `true` when a
    draft starts to differ from the value the editor shows at rest and
    `false` when it stops, after settling. `SearchField` (rule 8) and
    `InlineSlider` (rule 1) cannot keep one. Every completion an editor
    reports to a session, Escape included, requests its modified report
    first, so a host that closes the edit and removes the editor in its
    delivery does so after the editor's report: the terminal `false` of a
    save, an unchanged confirmation or Escape always reaches the application.
    A draft that removal ends reports nothing, as for every removed
    component's notifications; an edit the `Table` ends itself is reported by
    the table (rule 11).
11. **The `Table` reports every edit's end.** Besides the editor's own
    completions, it ends edits itself: Escape on its body while the editor
    does not hold focus, another edit beginning, the edited row leaving the
    rows by deletion or a filter, a failed cell editor callback and the table
    hiding. Each end, of either kind, reaches `Table::on_edit_end` once,
    after settling, through a notification request of its own, so ends keep
    their place in the global request order. An end recorded while no
    callback is installed is not reported later, and removing the callback
    discards ends not yet delivered. Removing the table itself reports
    nothing: it is the application's own act.

`TextCommand::DiscardDraft` is how an application drops a kept draft without
touching application data: the editor discards it as Escape does, also
while disabled or read-only, reports `on_rejection(None)` within the
command where a refusal was reported and `on_modified(false)` after
settling, and a session's edit stays open (its next completion reports
`Unchanged`). It is aimed at the control that owns the draft; a
`DraftEditor` answers it `Unavailable`. `FormFields::discard_drafts` queues
it for every field a form shows, in registration order. A draft that a
changed write ended before the command arrived is reported by that write's
route: the command finds no draft and answers `Unavailable`, and a reported
refusal ends at the editor's next save or Escape.
Describing a field with another binding also discards a draft, at the cost
of a second signal holding the same value; describing it under another key
remounts it, losing focus, undo and retained layout.

For the built-in editors:

| Event | Local draft | Parameter draft | In-place session | Session report |
|---|---|---|---|---|
| Save attempt by Enter, Tab or a step | saved if accepted, else kept | the parameter outcomes below; `InlineSlider` its own | saved or proposed, else kept open | `Saved`, `Unchanged` or `Refused` |
| Save attempt by focus loss | saved if accepted, else kept | cancelled | as a local or parameter draft | `FocusLeft` with the outcome |
| Enter in `TextArea`; Enter or Tab while busy | no save attempt | no save attempt | no save attempt | nothing |
| Application write of a changed value | discarded | discarded | ended | `Unchanged` at the next completion |
| Editability revoked | kept, save attempts refused | kept, save attempts refused | read-only, save attempts refused | `Refused` at a confirmation |
| Enablement revoked | kept unsaved, focused or not | cancelled | ended unsaved | nothing; the edit stays open |
| Hiding, removal | discarded | cancelled | ended | nothing |
| Close admission | as focus loss when focused, else untouched | as focus loss | as focus loss when focused, else untouched | as focus loss |

A parameter number field's save attempt has five outcomes:

| Outcome | Draft afterwards | Callbacks and host |
|---|---|---|
| Validation refusal: does not parse, out of range, off the step lattice | kept with the outline | `on_rejection`; no `on_save`; the host sees nothing, or a gesture's `Cancelled` |
| Refused admission: the edit cannot begin (busy, stale, read-only, unavailable, delivery fault) | replaced by the parameter's value; an inline row leaves text mode | `on_rejection(None)` clears a shown refusal; no `on_save`; no host outcome |
| Terminal refusal of an admitted edit (conflict, rejection) | replaced by the parameter's value | no `on_save`; the host's outcome event reports the refusal |
| Unchanged: the draft equals the base | replaced by the equal value | no `on_save`; no host outcome, or a gesture's `Cancelled` |
| Accepted: committed, or submitted to a remote authority | replaced by the committed or submitted value | `on_save` with the value; the host's outcome event |

`InlineSlider` begins its parameter edit when text mode opens (a refused
admission opens no text mode), keeps unparsable text with the outline and the
edit open, commits on Enter even when the value is unchanged, ends text mode
on a terminal refusal, and cancels on Escape, focus loss, revocation, hiding
and removal; it has no `on_rejection` or `on_save`, and the host's outcome
events are its report.

| Session completion | `Table` |
|---|---|
| `Saved`, `Unchanged`, `Cancelled` | closes the edit; after Enter, Tab or Escape focus goes to the body or the next cell; after a departure focus stays where it went |
| `Refused` | keeps the edit open; after Enter focus stays in the editor; after a departure the editor stays unfocused until its cell is activated again |
| Ended by the table (body Escape, replacement, row left, failed callback, hiding) | the editor is removed without a report of its own; `on_edit_end` reports the reason |

### Table cell edits

A `Table` with a cell editor edits one cell at a time and hosts the editor
through an [editor session](#editor-sessions), so a parameter-bound editor,
which ignores sessions, is not a cell editor. The table owns the open cell
edit rather than the cell: the edited row stays mounted for the whole edit,
and the table acts on how the edit ended after the save's callbacks, so Tab's
destination follows the order they left; an edit that ended after focus left
the editor never moves focus. An edit closes when its row leaves the rows,
when another edit begins (discarding a refused draft), and when the table is
hidden or unmounted. Disabling the editor, a text or a number field alike,
leaves its draft unsaved and the edit open until the table ends it, which it
does on Escape on its body. `Table::on_edit_end` reports each edit's end with
a `CellEditEnd`, whether the editor completed it or the table ended it, as
the [draft policy](#draft-policy) states (rule 11 and the completion table);
removing the table reports nothing.

### Search field

`SearchField` is the editor an owner presenting results embeds, such as a
palette or chooser. Its query is committed text, never preedit. Escape
clears a non-empty query and is otherwise left to the owner. On both input
paths a fresh Escape reaches the focused editors before an open popup, menu
or dialog closes on it: `command_key_input` through its text phase, and
`key_input` by running that phase for Escape while a surface is open. A
search field with a query inside a popup therefore clears on the first
Escape and the popup closes on the second. Shown tooltip help comes before
both: on both paths an unmodified Escape first dismisses it. Navigation keys
and Enter are reported to the owner without moving focus. An optional
debounce is one runtime deadline that each edit restarts, never a polling
loop.

## Theme palettes

A `Palette` is an owned value with a base and an optional override per
`ColorRole`. The base is a built-in mode's defaults or, from
`Palette::from_seeds`, those defaults recolored once at construction from seed
colors. A role resolves to its override, otherwise its base value, and a role
that follows another (`ColorRole::follows`) takes its source's current color;
following copies and never computes. An override is kept exactly, and
`Palette::with_default_color` removes it: a follower resumes following its
source's current color, which may itself be overridden, and any other role
returns to its base value, the seeded value on a seeded palette and the
built-in value otherwise.

Seeding keeps every generated role's built-in WCAG relative luminance and
alpha, so the built-in luminance contrasts hold; it does not measure hue, and
a seed near a status hue makes the accent resemble that status.

Equality compares the base, including the seeds, and the overrides, not resolved
colors. `ThemeManager` stores one palette per mode and repaints only the
readers of roles whose resolved color changed; a changed resolved
`Background` also marks every window's scene for presentation, since the
scene carries it, even without a component that reads the role.

## Workspace persistence

`Workspace` captures deterministic arrangement separately from application
project data. It uses a separately versioned, closed document. Stable addressable
paths identify participating views; structural wrappers do not change those
paths. Capture includes user arrangement and component-provided view state, not
hover, pointer capture, transient gestures or application content. Restore
validates schema, identities and numeric ranges. Unsupported formats fail;
missing or changed views are reported and may be skipped. Revision 2 of the
document changed how an `Each`'s default container addresses its items (it
adds no segment), so a revision-1 document is rejected with
`WorkspaceError::UnsupportedVersion`, never reinterpreted, and the host falls
back to its default workspace as for any unsupported document. Layout clamps saved
sizes to current usable bounds without rewriting the preference merely because
the current window is smaller. The host chooses storage and I/O policy.

Capture is complete or fails: a participant that cannot build its state, for
example because a key it must record is not storable workspace text, fails
the whole capture (and an explicit restore) with an error naming its path,
rather than being left out of a document that reports success.

Capture and restore see the actual mounted children, hidden and detached
ones included, without flushing updates or emitting callbacks. Accordion
records only open sections whose children still exist, and Tabs the surviving
docked selection its next update would show, omitting the selection when no
docked child remains; that empty selection can be restored. A rejected entry
leaves its preferences unchanged, and a table's entry restored while the
runtime is closed or forbids writes is rejected as `RestoreRejection::Runtime`.
Custom components take part through the [workspace hooks](#workspace-hooks).

### Restore and active gestures

Restoring changed zoom or density that affects visible layout cancels captured
gestures, focused held-key actions and registered edits when geometry next
settles. Density affects layout when a visible reader uses a changed metric.
An active drag ends with a terminal cancelled outcome. Unchanged settings
preserve gestures.

## Inspection and diagnostics

Inspection captures owned snapshots on the UI thread. A remote tool receives
snapshots through an application-owned transport and never retains live UI
handles. Typed semantics report roles, names, values and supported actions.
A component without a name of its own is named by its element's label
(`Element::with_label`) or by a wrapping `Field`; for a composite that reports
no semantics itself, such as a `Select`, either reaches the internal part that
represents it. A naming parent (`Semantics::names_children`) also describes
its children: a child's own description, such as an editor's unsaved-draft
note, is kept over ordinary text such as help, but an invalid parent's error
and a parent's warning (`Semantics::warning`) replace it, so a `Field`'s error
or warning reaches its control ([inspection in custom
components](#inspection-in-custom-components)). Snapshots carry
`InspectionSnapshot::SCHEMA_VERSION`, which advances whenever a field is added,
removed or reinterpreted; the serialized form is closed, and deserializing an
`InspectionSnapshot` or a `QueryResult` checks `schema_version` itself and
rejects every other revision ("inspection schema version N is not
supported"), so a snapshot of another revision does not decode as this one.
The version is a snapshot's first field, so an order-keeping serializer writes
it first; a tool that reads several revisions compares it before decoding.
`Element::expose_part` declares scoped UI-local part metadata independently of
keys, semantic names and workspace addressing. `Runtime::find_exposed_part`
and `Driver::exposed_parts` traverse internal structural/part/slot wrappers,
stop at ordinary application children and exposed parts, and skip drag previews.
An exposed part is a nested scope; duplicate names can be listed but singular
lookup refuses them. Names match exactly, without trimming or splitting `/`;
blank declarations are refused before adoption. Same-key redescription replaces
or removes the declaration without remounting or invalidating layout solely for
that metadata. Queries do not flush, reveal, focus, select a window or schedule
work. Returned `WidgetId`s remain the event targets, including when presented in
another window, which must be selected before input. `ComponentInspection::exposed_part`
is descriptive metadata in inspection schema 17, not a transport-level selector:
the owned DTO does not reproduce the scope walk. Its serialized field is required
and nullable, and a present blank name is invalid. Workspace schema 2 and its
address markers are unchanged.

Unknown geometry remains absent until measured.

A record's geometry is the last successful layout's. Layout stages every
arrangement and commits bounds, clip and the view scale they were computed at
only when the whole pass succeeds; `ComponentInspection::view_scale` is that
committed scale, the one `border_box`, the clip and the item bounds are in,
and before a component's first layout the scale it will be laid out at. While
a component's committed geometry is from another window than the one it is
now presented in, after a detach or a redock until that window's layout
commits it, the record has no `border_box`, clip or item bounds, and
`view_scale` is again the scale it will be laid out at. A row a component
draws itself, a `SemanticItem`, has window `bounds` when the component gives
it a rectangle in its own local coordinates (`SemanticItem::local_bounds`),
from the geometry its painting and hit testing use: the runtime moves it by
the committed origin, scales it by the committed scale and cuts it to the
committed clip, so it follows view scaling, ancestor clipping, promoted
surfaces and detached windows exactly as the border box does. Item bounds are
absent before the first layout, while the component is not presented, when
nothing of the row is visible, and after a failed layout that arranged the
component: that arrangement already replaced the component's retained
geometry, so the runtime arranges every component a failed pass arranged
again at the next layout, which pairs the two again. A change not yet laid
out shows in neither, as for the border box; state a component changes
outside layout, such as a scroll offset, shows at once. A value a component
assigns to `bounds` directly is replaced, and inspection never arranges or
schedules anything for it. Development validation checks
geometry only for a native window whose latest layout ran with a viewport that
has area: before a host supplies the first viewport, and while a window's
viewport is empty, layout assigns empty boxes that are reported by inspection
but not validated. Event history is bounded,
disabled by default, and distinguishes success, failure and cancellation;
inspection itself schedules no polling. Secret input stays out of both: a
component that conceals its input (`Component::conceals_input`, as a concealed
`TextField` does) reports protected semantics without a value, and history
records none of the keys typed into it.

### Rendered capture

Capture targets an actual rendered frame of a window or widget and keeps scale,
crop and frame/state correlation. GPU readback and encoding stay outside
time-critical callbacks; image bytes stay outside event history. A whole-display screenshot
would be a separate operating-system service. Development validation and
optional profiling expose measured work without changing the ordinary
presentation schedule. Inspection replies retain an owned `RuntimeError` when
admission's flush fails, even if the value was captured successfully. A pending
capture keeps that admission cause until its host completes the request; later
successful flushes do not erase it. Runtime/drawing capture failures remain typed,
including application error identity and downcasting. With `serde`, portable
capture outcomes retain their representation; Rust-cause variants explicitly
reject encoding/decoding. Protocol adapters define their own diagnostic DTO.
[Custom hosts](#custom-hosts) render and supply the frames.

## Public compatibility policy

While no library version has been published, any change may make breaking
API or code-structure changes that improve the library, without compatibility
aliases or a version bump solely for unpublished compatibility. Record replacement usage and update repository
consumers, examples and docs together. Persisted schemas remain separately
versioned and validated. The following policy applies to published versions.

Before 1.0, a documented breaking public API change uses a new minor version
and an explicit changelog note. Patch releases preserve the published API
contract. Persisted formats carry their own version independently of the
crate version. Backend changes are reviewed for observable facade and runtime
effects. A growing public error enum is `#[non_exhaustive]`; a deliberately
closed enum says so in rustdoc so a consumer can match it exhaustively.
Inspection records that only the runtime builds are `#[non_exhaustive]` and
grow with `SCHEMA_VERSION`. Records a component builds for
`Component::inspect` are built from `Default`. Status and outcome enums are
closed unless their rustdoc says they grow.

## Error policy

Every public failure has a typed error, with the following common rules:

- Library-owned errors derive `Clone`, `Debug`, `PartialEq` and `Eq` where
  their data permits; an error retaining an I/O source derives only `Debug`.
- A variant that rejects a key, identity, path or integer value carries it,
  and its message names it (`RuntimeError::AmbiguousKey`,
  `NumberInputError::InvalidPrecision`, `ImageError::Io`); a list's
  duplicate values are named by position (`ChoiceError::DuplicateValue`),
  since an application value need not be printable. A variant that validates
  an argument the caller passed and still holds stays unit: `GeometryError`'s,
  `LayoutError::{InvalidWeight, InvalidRatio}` and
  `ParameterError::{InvalidRange, OutOfRange, OffLattice}`. By the
  maintainer's decision of 2026-10-23, a variant that rejects a
  floating-point value also stays unit in every error enum
  (`NormalizedValueError::OutOfRange`, `PathError::InvalidParameter`,
  `NumberInputError::OutOfRange`), so that errors keep `Eq`. An owned
  identity may cost the enum `Copy`, which is not part of this policy.
- Display messages begin in lowercase and have no trailing period. An error
  whose message is text supplied by the application or the operating system
  (`InputRejection`, `ListingError`) displays it verbatim, as
  `ApplicationError` does; a `ListingError` that names a path writes it
  first. A wrapped error is transparent or exposed through
  `source()`, so a chain prints each cause once. Foreign backend failures may
  become diagnostic strings; library causes remain typed.
- An application callback returns its own failure through
  `RuntimeError::Application`, which retains the source for downcasting.
  Library variants do not stand in for unrelated host failures.
- Conversions point upwards from focused leaf errors into `RuntimeError`, then
  into drawing, workspace, window or application errors as appropriate. A
  runtime failure has one normalized representation at each higher level.
- One exception at the callback boundary: an application callback propagates
  with `?` a `WorkspaceError` or `CaptureError` that a runtime method it
  called returned, or the `WindowError` a `PresentationReceipt` reports in
  `PresentationStatus::Failed`, as `RuntimeError::Application`, holding the
  whole error, downcastable to its type. No public runtime method returns a
  `WindowError`; a callback meets one in a receipt's status. An
  error whose `Runtime` variant holds a runtime failure becomes that failure
  itself, so it keeps one form. `RuntimeError` gains no variant for these,
  and `DrawError` has no such conversion, since drawing fails in paint and
  host paths that already return it.
- Errors carry no location; a rejected key or value is the failure's data,
  not where it happened. `Runtime::last_failure` reports where the
  latest flush, paint or interval service first failed in a component: its
  key path and the failing callback, or `FailurePhase::Reconcile` when the
  children it requested were rejected. A reconciliation site keeps two
  paths apart: `path` names the mounted component that requested the
  children, and `description` the keys down to the rejected description,
  which need not be mounted. Direct calls to `reconcile_children` and
  `mount_tree` return their error to the caller and record no site.
- One limit has one reporting variant across paths that share the limit.
  Every public `Result` has an `# Errors` rustdoc section naming the variants
  and conditions, including forwarded failures.
- A `TryFrom` or `FromStr` implementation states its failures in one line
  linking the constructor that documents them. Serde implementations, derived
  or hand-written, are documented at the type: a type whose deserialization
  validates has a `# Serialization` section naming the format and every
  rejection, and its impls carry no documentation of their own. They do not
  invent variants for `D::Error`.
- A conversion that only checks a value's kind may return its input unchanged
  as the error, as std's array `TryFrom` does; its rustdoc says so.

Errors do not silently discard failed I/O, edits, persistence or state
reconstruction. A caller can inspect the typed source chain and decide its
own recovery policy.

## Public naming rules

Validated values use a fallible constructor and conventional `TryFrom` where
appropriate. A single-value type's raw value is read with `get()` (a string
with `as_str()`) unless the unit itself is the point of the type. A
validated quantity (a position, extent, factor, rate or capacity, such as
`NormalizedValue`, `Length`, `UiZoom` or `Capacity`) also converts into its
raw number with `From` (`f32::from(value)`), as std's `NonZero*` types do;
numeric identities and counters (`Revision`, `Sequence`) have no such `From`. Types with
several components expose named component accessors and a whole-value
accessor named for its content (`channels`, `values`, `entries`). Limits the
library chooses are associated `MIN_*` or `MAX_*` constants of the type they
bound; intrinsic bounds of a domain (nonzero, nonnegative, the unit interval)
are documented on the constructor and its `# Errors` section without
constants. A constant that bounds the value itself has type `Self`
(`UiZoom::MAX`, `PowerExponent::MIN`), so it is a valid value and `get()`
gives the raw number; a constant that bounds a measure of it (bytes, a
count, a depth, digits) has that measure's integer type. A limit shared by
several types is defined once, and its rustdoc says why it lives there:
- a validation limit, which input fails, lives on the owner of the shared
  format (`Workspace::MAX_STRING_BYTES` for its document, the `svg_images` module's
  `MAX_NESTING` for SVG input), otherwise on the error type whose one
  variant reports it (`TextEditError::MAX_TEXT_BYTES`); the bounded
  constructors link to it from `# Errors`;
- a rendering cap, at which output degrades instead of failing, has no
  reporting variant, so it lives on the error type of the operation it keeps
  from failing, and that type's rustdoc names it as the case that is not an
  error (`TextError::MAX_FONT_PIXELS`).

A value is clamped only where the [data
invariants](../AGENTS.md#rust-and-data-invariants) allow it: a documented
control or layout limit, or a documented presentation mapping that is total
over its input; an invalid explicit text offset, identity or stored
preference value is rejected, never coerced.

Constructor parameters run: the state owner (`ui`), then the stable key of a
keyed item, then the state the constructor binds, then content and options
(`Checkbox::new(value, label)`, `MenuItem::radio(key, selection, value,
label)`, `TableController::new(ui, selection, columns, source)`).

Capability restrictions take `impl Into<Permission>` and compose with `and`;
they accept fixed booleans and read bindings without separate names. A
builder for an ordinary boolean property takes a `bool`; the binding variant
adds `_signal` and takes a `ReadSignal<bool>`. `_signal` names a method that
takes a one-way `ReadSignal` (`Runtime::command_enabled_signal`). A builder
that binds state the component also writes takes `impl Into<Signal<T>>` and is
named `bind_<property>` (`Collapsible::bind_expanded`,
`PropertyGrid::bind_label_width`). A no-argument builder moves
one fixed option of a description away from its default and has no way back
(`disabled()`, `read_only()`, `frameless()`, `nonmodal()`). An option the
application decides from data takes a `bool`, and so do the state-reporting
setters of data descriptions (`Semantics::disabled(bool)`,
`PickItem::disabled(bool)`), which report state rather than restrict a
capability. A fixed metadata shortcut on a specification stays
no-argument (`ParameterSpec::read_only()`).

When a type offers an owned and a reactive read of one value, the bare name
is the owned read and `x_binding()` the reactive one, which returns a
`ReadSignal` (`TaskReceipt::status` and `status_binding`,
`UiSettings::zoom` and `zoom_binding`, `DeliveryMonitor::snapshot` and
`snapshot_binding`). A type with only a reactive read uses the bare name
(`Parameter::snapshot`, `DialogController::status`). A retained `ReadSignal`
reads the value current at each `get()`, not the value when it was obtained,
so code that needs a point-in-time value keeps the result of `get()`, as
`Parameter::snapshot` documents beside `AnyParameter::snapshot`, which is
owned.

Geometry is in logical pixels unless its name says physical
(`PhysicalExtent`). A length a component property takes and stores is a
`Length` (`TableColumn::width`, `QuickPick::width`). Geometry values are
validated by their own constructors over bare finite `f32` logical pixels:
`Point::new` and `Rect::new`'s origin accept any finite value, `Size::new` and
`Rect::new`'s extent any finite nonnegative one, zero included. A constructor
whose argument must be positive may take `f32` and return `GeometryError`
(`VirtualList::new`'s row height). Geometry the library reports, or that is
read back for arithmetic, is bare `f32` logical pixels
(`TableColumn::fixed_width`, `Painter::metric`, and `ToolWindowChange::extent`,
which `ToolWindowChange::with_geometry` sets).

Ordinary property builders replace earlier values. An element builder
appends one element to a described collection (a layout's tool windows, a
title bar's actions, a description's semantic actions), and its plural form
appends several. A builder that sets a whole list or one slot
(`Semantics::items`, `ToolWindow::actions`, `Toast::action`) replaces.

`with_` has three meanings, told apart by the receiver. An associated
`Type::with_x(x, …)` is `new` with part `x` supplied by the caller, as
`Vec::with_capacity` is (`Menu::with_state`, `CollectionFilter::with_query`).
A consuming `self.with_x(…)` returning `Self` or `Result<Self, _>` is a
builder on a value or data type (`Color::with_alpha`, `Element::with_grow`);
it needs no readback getter, but a getter, where one exists, takes the bare
name (`Element::grow`). A borrowing `with_x(closure)` gives scoped access and
returns the closure's result (`Painter::with_clip`), or wraps it in a
`Result` when the access itself can fail before the closure runs
(`SeriesSource::with_window`). Component descriptions use bare builder names.

Cloneable UI-thread handles that drive a widget are `*Controller`. `*State`
names a value its owner holds directly, never a shared handle: captured
component or device state (`ViewState`, `KeyboardState`) or an owned state
machine (`ParameterState`). An owned point-in-time copy of observable data
is a `*Snapshot` (`ParameterSnapshot`, `InspectionSnapshot`); one inspection
record is an `*Inspection`; the summary of a finished check or batch is a
`*Report` (`ValidationReport`, `DrainReport`). Other passive data take a
descriptive noun (`ChildInfo`, `CommandEntry`, `ProfileStats`).
Application-owned state that widgets follow through handles the application
keeps, such as collection selection (`ListSelection`) and form status
(`FormFields` and its `FieldHandle`s), is named for what it holds.
`*Controller` is reserved for handles to state that a mounted widget owns.
Collection selection remains application-owned and addressed by stable key.

A `*Receipt` reports one queued request through an owned `status()` and a
`status_binding()` for reactive UI. A receipt is `#[must_use]` when dropping
it can hide an outcome that nothing else reports, a failure or a
cancellation: `ShownTaskReceipt` is the only report of a `defer_while_shown`
operation cancelled because its component was hidden or removed. A
`TaskReceipt`'s failure also propagates from the flush and its only
cancellations, shutdown and a panic as the operation unwinds, are events the
application sees itself, and a `ToastReceipt`'s failing action also
propagates and its expiry or dismissal is presentation, so neither carries
the attribute. The attribute is on the receipt type, not on the method: a
`#[must_use]` method returning `Result` would be satisfied by `?`. A cross-thread reply
that carries its answer and is consumed once, such as the `Pending<T>` an
`InspectionClient` request returns, is not a receipt. A `*Outcome` is the
non-error result of an interaction or operation, distinct from `Result` as an error carrier. A
result may instead be named for what it delivers (`Frame`, `ChildReveal`),
and it may carry a continuation for unfinished work (`ContentJump::Pending`).
An outcome of local processing does not report acceptance by the authority,
which stays a separate report, as for [user-write
callbacks](#user-write-callbacks).

## For component and host authors

This part holds what a custom component or a custom host needs beyond the
application rules above. A custom component uses the same public contexts,
child, event and painting operations as a built-in one; a custom host drives
`Runtime` in place of `App`.

### Describing and redescribing components

**Text sources.** A component that shows a `TextSource` follows the
[caption contract](#captions) through `TextSource::read`, `read_paint` and
`same_as`.

**Checking a description.** `Component::check_description` rejects a
description from what it alone shows, before the call that mounts or
reconciles it changes anything ([component
redescription](#component-redescription)). `redescribe` and
`check_redescription` perform a re-description and compare it with a fresh
mount, for built-in and custom components alike.

Conformance helpers compare the direct described component by mounted incarnation,
independently of workspace addressing metadata, and flush at the current elapsed
time. Success returns ownership of the retained test tree and removes any fresh
reference tree. Failure removes only roots this call mounted; a typed
`RedescriptionError::Cleanup` preserves the original cause and exact unreleased
root handles if removal fails. Callback side effects are not rolled back, and
teardown errors queued by components remain observable through later runtime
flushes.

#### Retained generated children

A component whose constructor generates parts for its retained controller may
opt into `Component::retain_initial_children`. Initial parts mount normally even
while hidden, so search and workspace capture can discover them before first
reveal. A compatible ancestor redescription applies only the owner's properties:
its incoming generated descendants are discarded before validation or mounting,
while mounted parts keep their bindings, metadata and identities. The policy is
captured once at mount. The component regenerates replaceable part properties
from retained state through `UpdateContext::request_children`; ordinary
self-managed components still reject incoming descendants. Direct explicit child
reconciliation remains available to the owner. `StateOwner::ui_settings` gives
custom controllers the installed table for constructing initial hidden parts,
without subscribing or scheduling presentation. Retained components track later
string-table changes through `UpdateContext::strings` on update.

### Layout and focus hooks

Decorations can opt into `Component::fills_row_height`: they contribute their
natural height first, then fill the resolved row content up to the offered
height. The public measurement context exposes the same preference to custom
containers. The public `ArrangeContext` receives a row's stretch allocation,
so a custom container can do the same, and custom containers read an
element's grow weight through `MeasureContext::grow`.

An unmodified Tab or Shift+Tab that reaches default traversal is first offered
through `Component::tab_navigation` to the focused component and the ancestors
ordinary bubbling reaches, innermost first; a component that claims it moves
focus itself, as an editor moving to the next cell does. Keys owned earlier
keep their owner: on the native path held keys, command presses, the editing
phase, capture bindings and held-input registrations come first, so view,
window and application bindings still never see an unmodified Tab. The offer
does not take the key's release.

### Reporting component notifications

Many of the changes that [component notifications](#component-notifications)
report happen where no event context exists, in `Component::update` or
`Component::restore_workspace_state_with_children`, so a component records
them and asks for `Component::deliver_notifications` through a `Notifier`. The
runtime calls that hook [after settling](#after-settling), each request one
call in request order. The hook and `Notifier` are public, so a custom
component reports its changes the same way.

### Failure cleanup in components

A custom component observes which input a failure cancels through
`Component::cancel_input`. Input cancellation applies to direct dispatch errors
and failures of notification requests attributable to that dispatch. The first
reported error retains precedence, but any encountered attributable failure
still requires cleanup, even when an older unrelated error is reported. Older
requests, their descendants, and requests generated by external/preflush work
keep their own attribution; their errors propagate without cancelling another
event's focus, capture or drag.

Collective presentation-settlement errors without a uniquely attributable request
also propagate without cancelling the current gesture after dispatch. Headless
key failure cleanup follows every mounted incarnation actually called, including
keyboard and Escape-text bubbling, even if preparation or callbacks transfer
focus. Completed focus-loss, focusability-loss, hiding and surface-admission
capture cancellation is not repeated by later failure cleanup. Transfers into an
owner's subtree preserve its existing context policy. With no dispatched callback,
preparation failure cleanup retains the preflush owner. Regaining focus or
receiving keyboard, text, IME, text-command or general event input starts a fresh
input context.

Later focusability loss still clears focus after an already completed cancellation.
If a preflush blocks a held key's release or the last held pointer button's release
before its owner receives it, cleanup still ends the held action or edit. A
pending request cannot retarget a later event. Nested input calls scope error
classification to their own call. A failure of a container's reveal hook during
a queued reveal is attributed to the reveal's request.

### Stacking, hit testing and pointer refresh

Stacking parents can assign `Component::child_stacking_priority` to stable
immediate child keys. Drawing and hit testing order priorities first, then raised
recency within equal priorities. Logical navigation, layout and retained identity
stay unchanged. Custom parents request layout when stacking or priorities
change, even with unchanged geometry, so stationary hover and drag targets are
refreshed.

That refresh is an ordinary `PointerKind::Move` at the last position, delivered
after every layout or stacking change to the capture owner, or else to the
component under the pointer, and marked by `PointerEvent::is_refresh`. It is
not delivered during a drag session, which retargets through its own service.
Built-in components rely on it to follow geometry that changed under a
stationary pointer (subregion hover, a held scrollbar thumb, text selection).
A pointer handler must converge on it: it invalidates only when the state it
derives from the event changed, since an unconditional layout invalidation makes
every layout produce the next move. In debug builds an `UpdateLimit` report
names a refresh recipient whose refreshes left new work scheduled more than
once in the failing flush.

`Component::hit_test` describes the body hit region in local logical pixels,
under a read-only UI-thread query. Returning false permits lower drawing entries
to receive pointer input without excluding independently tested children or
foreground controls. Clipping and modality still apply; capture bypasses body
rejection. Hover, cursor, focus and drag targeting share the same hit order.
The body query does not redefine a surface's presentation extent, so
outside-press dismissal still uses the surface root's clipped bounding
rectangle.

### Content search hooks

Content search reads semantic names and values through `Component::inspect`,
under the inspection phase, and the rows a collection offers through the public
`Component::try_content_rows` hook (whose default preserves existing
`Component::content_rows` implementations), which `VirtualList` implements.
`ContentRows::mounted_rows_current` lets collections report that retained row
children describe an older source; search skips those subtrees and reads
current source rows without updating, laying out or mounting presentation. The
default preserves mounted-child search and duplicate suppression for
collections whose children remain current.

### Host-loaded data

Components perform no filesystem or other blocking I/O. A component
that needs data its host must load uses a `RequestTracker`: it issues a
`RequestTicket` per key (the key, a generation and the tracker's identity),
hands it to the host, and applies a result only while `status` reports it
`Current`, then calls `complete`. A newer ticket for the same key supersedes
an older one, a cancelled or completed key reports `Settled` (so a repeated
delivery changes nothing), and a ticket from another tracker, including one
that no longer exists, reports `Foreign`. A result whose application fails
is not completed, so it can be delivered again. The host chooses how results
travel back, a `Mailbox` or anything else, and the component chooses how it
shows data that is loading or failed. Issuing fails with `Exhausted` after
`u64::MAX` tickets rather than reusing a generation. The file browser is built
on this contract and reports such failures from its methods and its update
instead of leaving a folder loading. A host must keep its own project, undo
and persistent domain state.

### Editor sessions

A host that runs one editor at a time, such as a table cell, owns an
`EditorSession` and learns from it how each edit ended, because a component
cannot call another one while it handles an event. The host calls `begin`
with an `EditorStart` before it builds the editor, which attaches to that edit
through its `session` builder (`TextField`, `IntegerInput`, `DecimalInput`,
`Select`) or `EditorSession::attach`. The editor applies the start, keeps
Enter, Tab and Escape (claiming them in its editing phase on the native path,
so capture bindings, surfaces and ancestors never see them), performs its own
save, normalization and refusal, and reports an `EditorCompletion` before any
fallible application callback runs; the report requests the host's
`deliver_notifications`. `Saved`, `Unchanged` and `Cancelled` end the edit, so
reports of an older edit, including a focus departure the host's own focus
move caused, are dropped; `Refused` keeps it open. When a callback fails,
dispatch returns its error and the completion arrives at the next flush. A
text or number field with a session is always in draft mode, and its steps
change the draft rather than the binding, so Escape restores. While a
composition or selection drag is active the editor keeps Enter and Tab without
saving and Escape ends only the composition or drag. Reattaching a mounted,
focused editor to a new edit applies that edit's start. A `Select` list's
choice completes the edit that was open when the list opened. Parameter-bound
editors ignore sessions, since a refused submission cannot be told from an
unchanged value. Hiding and removal report nothing. Disabling an editor
reports nothing either: revoking its enabled permission releases focus as
cancelled input, not as focus loss, so its draft stays unsaved and the edit
open until the host ends it (a table closes it on Escape on its body); a
focus loss that reaches an editor a description disabled, before the next
flush applies it, reports nothing too. An editor with `on_modified` requests
its modified report before each completion it reports, Escape included, so
the application hears the draft's end before a host that closes the edit
removes the editor. A
native window that hides clears focus and records `FocusLeft`, which is
delivered once the window is shown again, as hidden windows hold
notifications. A host never takes focus back after `FocusLeft`.

`Table` is such a host ([table cell edits](#table-cell-edits)). It creates
the session from its own notifier so a completion reaches it even when the
edited row is gone, keeps the edited row mounted through
`VirtualList::keep_mounted` for the whole edit and acts on completions in its
delivery.

### Workspace hooks

#### Capture with mounted children

Capture that depends on child keys uses `Component::workspace_state_with_children`.
The runtime supplies actual mounted children, including hidden and detached ones,
without flushing updates or emitting callbacks. The default delegates to the
existing capture hook. Accordion filters open preferences against current children;
Tabs resolves the same surviving docked selection as its next update, omitting
selection when no docked child remains. That empty selection can be restored.

#### Restore with mounted children

Restore admission that depends on child keys uses
`Component::restore_workspace_state_with_children`. Its snapshot reflects actual
mounted children after committed reconciliation, including hidden and detached
children, even before the parent's next update. The default delegates to the
existing restore hook. Implementations validate all fields and identities before
changing preferences; rejected entries leave those preferences unchanged.
Table restoration also checks runtime writability before changing column widths
or order, including direct calls to its public component hook. A closed runtime
or forbidden write phase is a typed `RestoreRejection::Runtime`, not success
with a delayed row refresh.

### Inspection in custom components

Custom components reach the [naming precedence](#inspection-and-diagnostics)
through the public `Semantics` fields: a naming parent
(`Semantics::names_children`) describes its children, and its error and
warning replace a child's own description. A component that conceals its
input reports it through `Component::conceals_input`. The table uses only
public extension points for what tests and validation read about it: its
column titles give their rectangles with `SemanticItem::local_bounds`, which
`Driver::click_item` clicks, and its cells report
`ValidationHints::fixed_height`, so development validation reports content
that measured or is laid out taller than the row
(`ValidationCode::ExceedsFixedHeight`); an external grid does the same. Its
wrappers that forward their child's measurement call
`ArrangeContext::refresh_forwarded` in `arrange`, so a measurement the runtime
answers from its retained results leaves the child's current; an external
forwarding wrapper calls it too.

### Custom hosts

A host that drives `Runtime` itself, in place of `App`, imports the `host`
module: its sealed `HostIntegration` trait, which `Runtime` implements, holds
the methods only a host calls (event loop, windows and presentation, native
input, captures, the clipboard and shutdown), and the module re-exports the
names a host writes. A headless test that stands in for a host reaches the
trait through `testing`. `App` drives the runtime through the same trait, and
the facade's `host` page carries a complete host loop. A host takes on what
`App` does for an application:

- **Window titles.** A detached tab or tool window's container queues a
  coalescing `PresentationAction::Retitle` through
  `EventContext::set_window_title` in the event phase when the view's title
  differs from the title the window opened with; the host applies it to the
  open window. The container follows each request's receipt, asks again for a
  retitle the host dropped unfinished, asks for a title the host refused only
  once the view's title changes, and after a full request queue asks again
  250 ms later.
- **Closing.** `Runtime::request_close` lists the steps a host other than
  `App` follows to admit a close ([closing a window](#closing-a-window)).
- **Shutdown.** Hosts using `Runtime` directly drain
  `HostIntegration::take_shutdown_errors` after shutdown and before dropping it
  ([runtime shutdown](#runtime-shutdown)).
- **Captures.** Frames that a host supplies with the wrong byte count, extent
  or crop region fail with a typed `CaptureError::Frame`. That error keeps the
  dimensions involved and serializes like other portable outcomes. Only
  backend-specific rendering diagnostics remain text ([rendered
  capture](#rendered-capture)).
