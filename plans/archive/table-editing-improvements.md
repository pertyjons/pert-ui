# Plan: improvements found by an inventory-editor probe

Status: proposed 2026-10-19 and revised after an independent Codex review of
the findings and the first draft (see the review record); implemented in
0.3.32 to 0.3.44 and squash-merged into `main` as 5a6e451 (slice 0
documentation only; slice 1 in 0.3.32 and 0.3.33; slice 2 in 0.3.34; slice 3
in 0.3.35 and 0.3.36; slice 4 in 0.3.37 and 0.3.38; slice 5 in 0.3.39 and
0.3.40; slice 6 in 0.3.41 and 0.3.42; slice 7 in 0.3.43 and 0.3.44). The API
sketches below are the design as proposed; each slice's status line says
where the implementation differs.

## Background

On 2026-10-19, after the log explorer of
[log-explorer-improvements](log-explorer-improvements.md), the maintainer
asked for a probe that exercises inline fields and a toolbar with varied
functionality. The maintainer's session built a fifth application against
the public facade only: a spreadsheet-like inventory of 10 000 (and
100 000) generated items in a `Table` edited in place, with a text editor
for names, a `Select` for the category, `IntegerInput` and `DecimalInput`
for quantity and price, and a `Checkbox` column. Above it sits a `Toolbar`
of registered commands: add, duplicate and delete rows, application-owned
undo and redo, copying and pasting rows as tab-separated text, an
`InlineSelect` category filter, an `InlineToggle`, an `InlineText` title
and an `InlineInteger` VAT rate with a unit. Below it are a
`CollectionFilter` and a status row with an `InlineReadout` total. A headless
session drove it through `command_key_input`, as the native host does, and
measured release-build flushes on Fedora 44 x86_64. The maintainer ran the
window natively on Fedora 44 GNOME/Wayland and found that it worked well,
without trying every part. The application lived in a session scratchpad;
the observations are reported experiment results, not a reproducible
evidence record.

What worked well: commands registered once drive shortcuts and toolbar
buttons, and the buttons gray out through `command_enabled_signal`; Delete
in a text editor does not delete rows and the undo chord inside an editor is
the editor's own; toolbar groups collapse into More as documented by
`collapse_first` and `essential`; the toolbar's inline fields edit and save
as documented; a `TextField` editor with `normalize` keeps a draft refused
on Enter open; a `Select` works as a cell editor. About 680 components mount
at 10 000 and 100 000 rows. Opening, typing and saving one cell took 6 ms at
10 000 rows and 12 ms at 100 000, undo 3 ms and a two-row paste 4 ms; these
are observations of this application, whose own eager per-row state and
linear scans dominate them, not a bound.

What the author ran into, verified in code (file references are to the
checkout on 2026-10-19) or by the probe:

- **Editing in place has no library contract.** `Table::on_activate`
  (`table.rs`) runs on Enter with the row and column keys, "where an
  application opens an editor for the cell". The probe wrote a custom
  `Component` of about 70 lines that obtains an `EventContext` through a
  notification, focuses its child and requests `SelectAll`, ends the edit
  from `focus_within_changed(false)` and on Escape, returns focus to the
  table body through a saved `WidgetId`, and republishes the source with an
  "editing" marker so `TableItems::cell` builds the editor in one cell. The
  gallery's table only reports "Activated". The pieces for a correct editor
  exist (`DraftEditor` and `DraftEditorController::begin` with
  `DraftSelection::All`, the inline family, `PropertyGrid`'s keyed editors,
  which are not virtualized), but nothing composes them for a collection, and
  the probe's own wrapper got the lifecycle wrong (see the probe defects
  below).
- **No common completion contract across editors.** A table that coordinates
  an editor must know synchronously whether Enter or Tab saved, found nothing
  to save, or was refused, and must cancel on Escape. The existing callbacks
  do not tell it: `TextField::on_save` reports only a save that changed the
  binding, and only in draft mode (`text_fields.rs`); the numeric `on_save`
  excludes unchanged and refused saves and also reports step-button writes
  (`number_inputs.rs`); a `Select` reports a selection, not a draft save; and
  an `on_save` error is an operational failure after the write, not a
  refusal. Opening a quantity of 27 and pressing Enter unchanged calls no
  callback, so a table waiting for one can neither close nor move.
- **Only Enter activates a cell.** A double-click and committed text typed on
  the active cell do nothing: the row's press handler only records the column
  (`TableRow::pointer_input`), and `Runtime::text_input` goes to the focused
  component, the body list, which ignores it. F2 had to be an application
  command. `InlineText` already enters editing on a `ClickCounter`
  double-click, Enter or F2 with its text selected.
- **Cell content is in Tab traversal, and Tab handling is split.**
  `Component::tab_stop` is per component and `focus_next`
  (`runtime/pointer.rs`) enumerates every descendant stop, so a container
  cannot keep its content out of traversal: with a checkbox column, Tab from
  the table visits every mounted row's checkbox before leaving the table, and
  Tab from an open editor lands on a checkbox. The table rustdoc says cells
  are not Tab stops and that interactive cell content keeps its own input.
  Claiming Tab is possible only in the editing phase: on the native path
  `Component::text_key_event` runs from the focused component up through the
  editors before Tab traversal (`runtime/commands.rs`), while ordinary
  `keyboard_event` bubbling runs after it and never sees an unmodified Tab;
  `Runtime::key_input` traverses before dispatch (`runtime.rs`
  `key_transition`). The `DraftEditor` rustdoc says the keys it leaves,
  Tab included, reach the owner "by ordinary bubbling", which does not hold
  for Tab (recorded in TODO.md).
- **Number inputs consume Escape with nothing to cancel.** In their ordinary
  enabled, editable branch `IntegerInput` and `DecimalInput` return
  `Handled` for Escape even with a clean draft (`number_inputs.rs`,
  `NumberEditor::key`), while `TextField` lets an Escape it does not need
  bubble (`text_fields.rs`, the clean-draft branch). A container around a
  number field (a cell editor, dialog content) cannot close on Escape. A
  `TextField` consumes Escape to cancel composition, collapse a selection or
  discard a changed draft, and the native text phase gives it Escape before
  capture bindings and ancestors, so a cell editor needs a deliberate
  precedence to cancel and close in one press.
- **Select-all on open (withdrawn).** The probe reported that focus and
  `SelectAll` requested in one event left an `IntegerInput` unselected
  (typing "12" over 27 gave 1227). Slice 1 could not reproduce it, neither
  with an isolated field in either request order nor in the final probe,
  whose quantity editor opens with `0..2` selected and is replaced by typed
  text; the observation most likely came from an earlier probe version that
  requested the command before the editor existed. Slice 1 keeps the tests.
- **Per-cell state is awkward, not impossible.** `TableItems::cell(&self, …)`
  receives no state owner while every control binds a `Signal`. The probe
  created one checkbox signal per row in advance and kept them in step with
  its store, and wrapped the `Select` editor in a `Dynamic` to get a
  `DescribeContext`. A retained custom cell component can create its state in
  `update` (`UpdateContext` is a `StateOwner`), and `Signal::sibling` creates
  independent state from an existing binding, so only visible cells need
  state; no example or adapter shows that.
- **Command presentation is manual.** `Button::command` (`controls.rs`)
  builds a text button and tells an icon-only command button to use
  `Button::icon` with an action that calls `invoke_command`, so the
  application copies the enabled binding itself. Nothing derives a command's
  shortcut help (`Runtime::command_shortcuts`) for a button the way menu rows
  show it; an application can compose it with `Tooltip::new`, minding the
  button's own name help.
- **No icon roles for common editing actions.** `Icon` (`text/icons.rs`) has
  Undo, Redo, Add, Delete and Edit but no Copy, Cut, Paste or Duplicate;
  `IconSource::Custom` supplies other glyphs. Coverage of the bundled font is
  to be checked in the slice.
- **No public inspection id for a `WidgetId`.** The serial shared by
  `WidgetId` and `ComponentInspection::id` is crate-private. Tests can use
  snapshot paths and parent ids, but finding a known component's entry is
  roundabout, and a bare key was ambiguous in the probe (a table column and a
  toolbar control were both keyed `category`).
- **Guide gaps, not defects.** An edited row that a sort moves out of view
  stays active but off-screen until the application calls
  `TableController::reveal`; rows holding focus stay mounted
  (`virtual_list.rs` pinning) but are not kept revealed. `Runtime::key_input`
  bypasses command bindings (documented), so a session that tests shortcuts
  must use `command_key_input`. `find_descendant` reports `AmbiguousKey` when
  a toolbar group and its control share a key.
- **Probe defects found by the review**, recorded so slice 0 and the example
  do not repeat them: the wrapper closed and removed the editor on every
  focus departure, so a draft refused on focus loss was lost, and it always
  requested focus back to the table, stealing focus from a toolbar click;
  command enablement followed the raw selection while actions resolved only
  rows in the filtered order, so Delete stayed enabled for a row filtered
  out (`ListSelection` deliberately keeps keys absent from a source); and the
  per-row signal map made every source change linear.

## Working rules

The rules of [app-author-improvements](../app-author-improvements.md#working-rules)
apply unchanged: one slice per commit with its rustdoc, examples, gallery,
manual and CHANGELOG; the maintainer's patch version bump per slice; an
independent Codex review before a slice is called merge-ready; the contracts
read before each slice; and the gate table there. Breaking changes are
permitted while the library is unpublished, with replacement usage recorded
and repository consumers migrated in the same slice.

Additionally:

- **Design checkpoint.** Slices 4, 5 and 6 begin with a chosen alternative, a
  compiling API sketch and an enumerated list of contract tests, reviewed
  before implementation starts.
- **No privileged path.** The traversal scope, the Tab hook and the editor
  session are public extension points, and slice 5 proves them with a
  consumer-defined editor and collection before slice 6 uses them in `Table`.
- **Ownership.** The library still owns no data model or undo history: an
  edit session ends in an application decision that writes or refuses.
  Successful-write callbacks stay separate from session completion.
- **Focus.** A session returns focus to its collection only after a keyboard
  completion or cancellation it owns. A departure by other means (a press
  elsewhere, native window focus loss) never takes focus back, as the
  `DraftEditor` owner contract already requires.

## Slices in order

### 0. Documentation of table editing as it is

Status: done (documentation only, no version bump). The `DraftEditor`
rustdoc now states that Tab moves focus before ordinary bubbling, which
resolved the TODO.md entry.

- The manual's collection section and the `Table` rustdoc describe how an
  application edits a cell with today's API, built on `DraftEditor` or the
  inline family rather than the probe's wrapper: a draft refused on focus
  loss stays open, focus is returned only after Enter or Escape, the edited
  row is revealed after a sort, and command enablement follows the rows the
  action can reach. Slice 6 replaces the recipe with the new contract.
- The testing guide repeats next to `key_input` that it bypasses command
  bindings and the editing phase, and names `command_key_input`; the
  `find_descendant` rustdoc names the group-and-control key clash as a cause
  of `AmbiguousKey`.
- The `DraftEditor` rustdoc states how Tab reaches an owner (the editing
  phase on the native path, not ordinary bubbling), resolving the TODO.md
  entry if slice 4 does not change the route first.

### 1. Number inputs: Escape and select-all

Status: done in 0.3.32, with a review fix in 0.3.33 (Escape during a
selection drag over clean text stays with the field). The select-all loss
did not reproduce (see the background): the tests cover both request orders
for every built-in editor and a draft set while the editor mounts, and the
probe showed the selection surviving the opening key's release; the
redescription and forwarding-wrapper cases listed below were not added,
since there was no loss to locate. A clean number field leaves Escape to
its owner whether or not its text is selected; unlike `TextField`, it has no
selection to collapse first.

- First isolate the select-all loss with a plain `IntegerInput` outside any
  wrapper, observing the selection after command delivery, layout, focus and
  the opening key's release, and fix the step that clears it. Do not reorder
  the flush's global queues unless the isolation shows that the order is the
  cause. Contract tests across `TextField`, `TextArea`, `IntegerInput`,
  `DecimalInput` and `SearchField`: focus and `SelectAll` requested in one
  event, then typed text replaces the value; also with an unchanged binding,
  an external replacement in the same flush, same-key redescription, and a
  command sent to a wrapper that forwards to a focused descendant.
- `IntegerInput` and `DecimalInput` leave Escape unhandled when there is
  nothing to cancel, as `TextField` does: no changed draft, composition,
  refusal report, drag or held step. Disabled and read-only fields and the
  inline rows keep their documented behavior. Contract tests: Escape bubbles
  from a clean field, including one whose text is selected only if that
  matches `TextField`; it is consumed while a draft, composition, refusal or
  held step exists; headless and native key paths agree.

### 2. Command buttons with icons and shortcut help

Status: done in 0.3.34. Buttons have no help of their own, so the help is a
`Tooltip::command` wrapper rather than a property of the button; text
command buttons get no automatic help, since they already show the name.

- `Button::command_icon(ui, command, icon, on_receipt)` (name settled in the
  slice) builds an icon-only command button with the command's enabled
  binding and its name as accessible name, replacing the hand-wired recipe in
  the `Button::command` rustdoc.
- Command buttons present the command's name and first shortcut as help,
  following `command_shortcuts` as menu rows do, in place of the button's own
  name help rather than as a second tooltip. Whether a text command button
  shows help at all is settled in the slice.

### 3. Editing icon roles and inspection ids

Status: done in 0.3.35 (icons) and 0.3.36 (`Runtime::inspection_id`, with
the review fixes for slice 2). The id is a `Runtime` method so that a stale
or foreign handle reports `Unmounted` or `ForeignRuntime` as `bounds` does.

- `Icon` gains Copy, Cut, Paste and Duplicate after checking the bundled
  Material font's glyphs, with the gallery's icon list.
- `WidgetId` (or `Runtime`) exposes the inspection id of a mounted
  component, documented as equal to `ComponentInspection::id` for that
  incarnation, scoped to its runtime, and changing on remount; a stale handle
  reports what `bounds` reports.

### 4. Keyboard traversal scope and a Tab hook

Status: done in 0.3.37 as the revised checkpoint below describes, with
review fixes in 0.3.38 (error documentation and further regression tests).

- **Traversal scope.** A component can keep its descendants out of Tab
  traversal (for example `Component::traversal(&self) -> Traversal` with
  `Descendants` and `SelfOnly`), so a collection body is one Tab stop
  although its cells contain checkboxes or buttons. Pointer presses and
  explicit focus still reach the content; the collection makes it reachable
  from the keyboard (slice 6 forwards Space or Enter on the active cell).
- **Tab hook.** One defined place, after the editing phase and capture
  bindings and before default traversal, where the focused component's
  ancestors can claim an unmodified Tab or Shift+Tab, innermost first. No
  ordinary key handler gains an earlier phase, so command precedence is
  unchanged. `Runtime::key_input` and the native path take the same route.

Contract tests: a scoped container is one stop in both directions and nested
scopes compose; focus inside it by pointer or explicit focus still works and
Tab leaves it; a claiming ancestor receives Tab and Shift+Tab, the innermost
claim wins, and an unclaimed Tab traverses; matched releases after a focus
change, repeats, held keys, changing modifiers, capture versus view and
application bindings on Tab, a disabled binding reserving its chord, and
modal admission follow the existing rules.

#### Design checkpoint (2026-10-20)

Today, verified in code: `Runtime::key_transition` (`runtime.rs`) calls
`focus_next` for an unmodified Tab press or repeat before any component sees
the key and dispatches its release to the focused component. On the native
path `command_key_input` (`runtime/commands.rs`) first honors existing key
ownership (suppressed and held keys, a command or held registration that owns
the press, orphan repeats), then runs the editing phase (`text_key_event` from
the focused component up through editors), then capture bindings and held
input, which acquire ownership only on a fresh press, then the same
traversal, and only then ordinary bubbling and the view, window and
application bindings, which therefore never see an unmodified Tab.
`focus_next` (`runtime/pointer.rs`) takes every visible, revealable,
modal-admitted component whose `focusable` and `tab_stop` hold, in the
flattened navigation order of `navigation_order` (`runtime/tree.rs`), which
visits ordinary content depth first and defers surface content until after
it. Ordinary bubbling walks presentation parents and stops at the first one
a modal surface does not admit.

Chosen (revised after review):

```rust
pub trait Component {
    // ...existing methods...

    /// Whether this component's descendants take part in Tab traversal.
    /// Default true. With false, keyboard traversal ([`Runtime::focus_next`],
    /// Tab and Shift+Tab) skips the descendants in this component's own
    /// presentation subtree, while the component itself stays a stop when
    /// [`Self::tab_stop`] holds. A surface (popup, menu, dialog) and a native
    /// window root start a traversal scope of their own, so a dialog opened
    /// from inside the component keeps its stops. Pointer presses, explicit
    /// focus and the descendants' own focus requests still reach them, and
    /// traversal from a focused descendant continues from its place in
    /// navigation order.
    fn descendant_tab_stops(&self) -> bool {
        true
    }

    /// Offers an unmodified Tab (`reverse` false) or Shift+Tab press or repeat
    /// that has reached default traversal to the focused component and then to
    /// each ancestor ordinary bubbling would reach, innermost first. Return
    /// `Handled` to claim it: traversal does not run, and the component moves
    /// focus itself if it wants to. Default `Ignored`.
    ///
    /// Keys owned before traversal keep their owner: on the native path
    /// ([`Runtime::command_key_input`]) held and suppressed keys, existing
    /// command presses, the editing phase, capture bindings and held input all
    /// come first. Claiming does not acquire the key's release, which is
    /// dispatched as before, to the component focused when it arrives.
    fn tab_navigation(
        &mut self,
        cx: &mut EventContext,
        reverse: bool,
        repeat: bool,
    ) -> Result<InputOutcome, RuntimeError> {
        let _ = (cx, reverse, repeat);
        Ok(InputOutcome::Ignored)
    }
}

impl VirtualList {
    /// Whether interactive content inside items is a Tab stop. Default true.
    /// With false, item content leaves traversal; the body keeps its own
    /// focusability (a list without a selection is a stop only while it
    /// overflows), so an application reaching item content from the
    /// keyboard must provide the route. `Table` uses it in slice 6.
    pub fn item_tab_stops(self, stops: bool) -> Self;
}
```

- **Scope check.** `focus_next` drops a candidate when any presentation
  ancestor, the presentation root included, answers
  `descendant_tab_stops() == false`, walking up only to the nearest surface
  root or native window root. Excluded descendants are ordinary content of
  the scope, which navigation order visits contiguously right after the
  scope, so keeping the focused descendant's place means Tab continues after
  the scope's ordinary subtree and Shift+Tab reaches the scope itself or the
  stop before it; surfaces owned inside the scope are not excluded and keep
  their deferred place. The documentation describes traversal in terms of
  that flattened order. Nested scopes compose because any one ancestor up to
  the boundary excludes.
- **One walk for the hook.** The offer follows `dispatch_keyboard_tracked`'s
  walk (presentation parents, stopping at a parent a modal does not admit)
  and records recipients for failure cleanup like ordinary bubbling, so a
  component outside a modal cannot claim Tab from inside it. Tab with
  modifiers added or released during a hold behaves as today: only an event
  that is unmodified when it arrives reaches traversal and the hook.
- **Restoration is unchanged.** Focus restoration after hiding or unmounting
  (nearest focusable ancestor, sibling handoff, first focusable component)
  is not filtered by the scope; only traversal is.
- **No change for Table yet.** Turning `item_tab_stops(false)` on in `Table`
  without slice 6's forwarding of Space and Enter would make a checkbox
  column unreachable from the keyboard, so slice 4 only adds the extension
  points and the `VirtualList` builder; slice 6 enables it.
- **Errors.** A `tab_navigation` error propagates from the key entry point,
  the key reaches no other handler and recipients are cleaned up as for
  `keyboard_event`. Committed effects such as a signal write are not undone;
  a queued focus request follows the entry point's failure cleanup, which on
  the native path cancels it (corrected after the implementation review).

Rejected: delivering Tab through ordinary component bubbling and traversing
only when it is unhandled (before view, window and application bindings, so
command precedence would hold). Several components answer `Handled` for every
key in some state (a number input's held step button, menus in menu mode),
so Tab would silently stop traversing there; a dedicated offer that
components opt into avoids auditing every key handler. A `Traversal` enum
was considered for the scope; a boolean covers the one behavior needed, and
a later richer scope would be a new default method.

Additional contract tests: `focus_next` called directly honors the scope; a
scoped root-mounted list, a nested scope and a scope inside a detached view
or tool window; a view, window and application dialog opened from inside a
scope keeps its stops, and a nonmodal popup owned inside the scope is
reached in its deferred place and dismissed when traversal leaves it;
tooltips stay excluded; menu-bar titles keep `tab_stop` false and F10 and
menu mode are unchanged; the offer reaches an ancestor through a non-editor
focused child on both paths; a modal dialog's content claims Tab while the
page behind cannot; a capture binding on Tab and a held Tab registration win
over the offer on the native path, and their release ownership is kept; an
error from a hook that made no other change leaves focus where it was; a
focused cell control that unmounts restores focus by the existing rules.

Review of the checkpoint (Codex, 2026-10-20): found that the first draft's
scope would have removed a dialog's stops when the dialog was opened from
inside a scope, ignored a root-mounted scope, misdescribed traversal around
deferred surfaces, omitted key ownership before the editing phase, promised
a body stop a non-overflowing list does not have, and implied a rollback the
runtime does not provide. The design above addresses each.

### 5. An editor session protocol

Status: done in 0.3.39 following the second revision below. The types are
named `EditorSession`, `EditorAttachment`, `EditorStart`, `EditorCompletion`,
`EditorTrigger` and `EditorOutcome`, since `EditSession` and `EditOutcome`
already name the parameter edit protocol. The start applies from the
editor's own delivery. The review fixes in 0.3.40 made `Select::for_parameter`
ignore sessions, as the number fields do.

Design checkpoint first. A public protocol through which a host (a
collection, later a property grid or tree) runs one editor at a time:

- explicit begin (with all text selected, the caret at the end, or initial
  typed text), confirm and cancel operations, and a synchronous completion
  outcome that distinguishes saved, unchanged, refused and cancelled;
- Escape precedence: tooltip dismissal, then the editor's composition or
  drag cancellation, then cancelling the session in the same press, in both
  key paths; a selected but clean text is not a reason to keep Escape;
- adapters for the built-in editors (`TextField` and `DraftEditor`-based
  editors, the number inputs, `Select` and other choice editors with their
  popups), each documenting how its own `on_save`, normalizer and refusal
  map to the outcome;
- the alternative to compare at the checkpoint: a host built around
  `DraftEditorController` with application parse, validate and commit
  callbacks, plus explicit choice-editor adapters, which reuses fewer
  existing controls but has narrower ownership.

Proven by a consumer-defined editor and a consumer-defined collection in the
facade's tests before slice 6. Contract tests: unchanged completion,
normalizer-equivalent values, refused saves on Enter, Tab and focus loss, a
callback failure after an accepted write, popup focus inside a choice
editor, native window focus loss versus hiding, permission revocation, and
Unicode initial text with and without an IME composition.

#### Design checkpoint (2026-10-20, revised)

Constraint found in the runtime: a component cannot call another component
synchronously. During an event the runtime holds the handling component's
entry; `EventContext` reaches others only through queued requests (focus,
text commands, reveal), which apply at the next flush. `DraftEditor` works
because owner and editor share state through `DraftEditorController`, which
the built-in editors do not have.

The first draft let completion keys bubble to the host. Review showed that
this cannot convey an outcome: a capture binding sees an Enter the editor
left, a surface closes on an Escape before the host, a `Select` that closes
on its current option calls no `on_change`, option activation commits on a
release while text editors save on a press, and a focus departure carries
no outcome. The revised design keeps keys where they are and adds a shared
record, as `DraftEditorController` does.

Chosen: **a host-owned `EditSession` the editor reports into.**

```rust
/// One edit a host runs at a time, shared by the host and the editor it
/// mounts. UI-thread only; clones share the session.
#[derive(Clone)]
pub struct EditSession(Rc<SessionInner>);
impl EditSession {
    /// A session reporting to the host through its own notifier: each
    /// completion requests the host's `deliver_notifications`.
    pub fn new(notifier: Notifier) -> Self;
    /// How the editor starts once it gains focus: with its value selected,
    /// the caret at the end, or the typed text in place of the value.
    pub fn begin(&self, start: EditStart);
    /// The latest completion the editor reported, once.
    pub fn take_completion(&self) -> Option<EditCompletion>;
}
pub enum EditStart { SelectAll, CaretAtEnd, Typed(String) }
pub struct EditCompletion { pub trigger: EditTrigger, pub outcome: EditOutcome }
pub enum EditTrigger { Enter, Tab { reverse: bool }, Escape, Choice, FocusLeft }
pub enum EditOutcome { Saved, Unchanged, Refused, Cancelled }

impl TextField { pub fn session(self, session: &EditSession) -> Self; }
impl IntegerInput { pub fn session(self, session: &EditSession) -> Self; } // and DecimalInput
impl<T> Select<T> { pub fn session(self, session: &EditSession) -> Self; }
```

| Input | Editor with a session | Reports |
|---|---|---|
| Enter press | saves the draft (normalizer, limits, `on_save`) and keeps the key | `Enter`: `Saved`, `Unchanged` or `Refused` |
| Tab, Shift+Tab | saves in `tab_navigation` and claims the key; focus stays | `Tab`: the same outcomes |
| Escape press | discards the draft in one press and keeps the key | `Escape`, `Cancelled` |
| composition or selection drag active | Enter, Tab and Escape act on the composition or drag only | nothing |
| a choice in an open `Select` list, by key or click, current option included | writes it (or finds it current) and closes | `Choice`: `Saved` or `Unchanged` |
| Escape on a closed `Select` | keeps the key | `Escape`, `Cancelled` |
| Escape in an open `Select` list | closes the list as today | nothing |
| focus leaving by other means | the field's focus-loss save as today | `FocusLeft`: `Saved`, `Unchanged` or `Refused` |

- **Keys stay with the editor.** Enter, Tab and Escape are consumed by an
  editor with a session, so no capture binding, surface or ancestor sees
  them; the host learns of the completion from its notifier in the same
  flush and decides: end the session, move to the next cell, or keep it
  open after `Refused`. Hiding and removal report nothing: the host caused
  them.
- **Drafts.** A field with a session is in draft mode whatever its
  normalizer and limits, so Escape can always restore. An application write
  that wins over the draft reports `Unchanged`. A normalized text equal to
  the bound value is `Unchanged` and calls no `on_save`. A failing `on_save`
  propagates after the write and the completion is still `Saved`; nothing is
  replayed.
- **Sources.** Sessions apply to editors over local signals.
  `IntegerInput::for_parameter`-style sources report no outcome the session
  can trust (unchanged and refused submissions look alike), so a session on
  them is documented as ignored; the table use case binds application-owned
  drafts.
- **Start.** The editor applies the session's `EditStart` when it next
  gains focus, so the host focuses it with `request_focus_within` and needs
  no queued text; `Typed` replaces the value with the text and puts the
  caret after it. Typed text arrives as committed text; a host has no text
  input state, so typing on it never starts a composition, and an IME user
  begins with Enter or F2.
- **Read-only, disabled, refused binding.** A read-only or refused-binding
  field reports `Unchanged` for Enter and Tab with no draft and keeps the
  session's Escape; a disabled field cannot hold focus, so it reports
  nothing.
- **Custom editors** follow the same contract: hold the `EditSession`, call
  its reporting method (public, documented with the outcomes above) from
  their own key handling, and apply `EditStart` on focus.

Rejected: key bubbling (the first draft), for the reasons above; and a host
built only on `DraftEditorController` with application parse, validate and
commit callbacks, which would reimplement number parsing, ranges and
formatting per application and still need separate choice adapters.

Contract tests: each row of the table on both key paths; `on_save` once per
saved Enter, Tab or focus loss and never for Escape or an unchanged or
normalizer-equivalent draft; a refused save reported and the draft kept;
composition and drag protection; `Select` by key, by click and on the
current option; `EditStart` variants including Unicode typed text; a
session on a parameter source reporting nothing; a failing `on_save`; a
consumer-defined editor and a consumer-defined host (a list whose rows open
an editor) driving the protocol through the public API only.

#### Second revision (2026-10-20), after the review of the revision

The review accepted the shared session and found gaps; each is decided here
and replaces the conflicting parts above.

- **Attachment and lifetime.** `EditSession::begin(start)` starts a new
  edit and returns nothing; an editor built with `.session(&session)` (or a
  custom editor calling `session.attach()`) holds an `EditAttachment` bound
  to the edit current at that moment, so the host calls `begin` before it
  builds the editor. Reports from an attachment of an older edit, or after
  `EditSession::end`, are dropped. A terminal report (`Saved`, `Unchanged`,
  `Cancelled`) ends the edit, so a focus departure that follows the host's
  own focus move reports nothing; `Refused` keeps it open for a retry.
  Pending reports queue in order and `take_completion` returns the oldest.
- **Delivery.** The report is recorded before any fallible callback
  (`on_save`, `on_change`) runs. When that callback fails, input dispatch
  returns its error and skips notification delivery, so the host receives the
  completion at the next flush; nothing is replayed.
- **Tab precedence.** An editor with a session claims Tab in its editing
  phase (`text_key_event`) on the native path, before capture bindings, and
  in `tab_navigation` on `key_input`; both run one completion. While a
  composition or selection drag is active the key acts on it and nothing is
  reported. Repeats of a claimed Tab are consumed without a second report.
  Tab confirms like Enter (`submit_save`, which validates an untouched
  required field); a focus departure uses the passive save as today.
- **Escape and help.** Today an editor's Escape runs before tooltip
  dismissal on both paths although the contracts give help the first
  dismissal; the slice moves tooltip dismissal ahead of the editing phase
  on both paths (a fix of the existing order), and session editors then
  consume Escape in the editing phase, so an enclosing dialog stays open.
- **Start.** The editor applies the pending `EditStart` from its own
  `deliver_notifications`, requested when its update sees one, so it works
  for an already focused editor and fallible text preparation reports its
  error from the flush. `Typed` text that a single-line editor cannot hold
  (a line break) is refused with the editor's ordinary text error and the
  value kept selected. The attachment's `take_start` serves custom editors.
- **Select.** Its own matrix: on the closed trigger Enter and Space open the
  list as today and report nothing; Tab reports `Tab: Unchanged`; Escape is
  claimed in the editing phase and reports `Cancelled`. In the open list a
  choice by key release or click reports `Choice` (`Saved` or, for the
  current option, `Unchanged`) from the choose closure before `on_change`;
  Escape closes the list only; Tab is left to the list's existing behavior.
  Focus moving between the trigger and its own list is not a departure; leaving
  the composite reports `FocusLeft: Unchanged`.
- **Numbers.** With a session the stepper buttons and arrow steps edit the
  draft instead of saving each step, so Escape restores; during a held step
  every key is consumed as today and nothing is reported.
- **Read-only and refused bindings.** A dirty field that became read-only
  or whose binding is refused reports `Refused` for Enter and Tab (its draft
  is kept, not saved); a clean one reports `Unchanged`. A native window
  losing focus or hiding clears focus and therefore reports `FocusLeft`;
  hiding or removing the editor itself reports nothing.
- **Scope.** Sessions in this slice: `TextField`, `IntegerInput` and
  `DecimalInput` over local signals, and `Select` over a local signal.
  `InlineText`, the inline number rows, `TextArea`, `SearchField` and
  parameter sources ignore sessions entirely (no start, no key consumption),
  as documented on `.session`; inline rows already own a resting and editing
  policy that a later slice can map onto the protocol. Those sessions are in
  the [follow-up index](../follow-ups.md).
- **API conventions.** `.session` replaces an earlier session; `EditStart`,
  `EditTrigger` and `EditOutcome` are `#[non_exhaustive]`; `take_completion`
  and `take_start` are `#[must_use]`; reporting is infallible.

### 6. Cell editing in `Table`

Status: done in 0.3.41 as the revised checkpoint below describes, with
review fixes in 0.3.42 (unmount cleanup, mount-time Tab policy, reaching an
open edit whose cell became ineligible, a table-owned wrapper around cell
content keyed like the editor's host). The edit lives in
`table/editing.rs`. Columns cannot be removed from a mounted table today,
so the removed-column rule has no trigger yet.

Design checkpoint first, on slices 4 and 5.

- An edit begins on Enter or F2 on the active cell, on a double-click of a
  cell and on committed text typed on the active cell, which becomes the
  initial draft; the checkpoint decides whether typing on the table can
  start an IME composition.
- The application supplies an editor per column through a callback that
  receives a state owner, the row and column keys and how the edit began,
  and decides the save. Enter confirms and moves down, Tab and Shift+Tab
  confirm and move to the next or previous editable cell, Escape cancels, all
  per slice 5's outcomes; a refused confirmation keeps the session open, and
  focus departing by other means confirms without taking focus back.
- The edited row stays mounted while the session lasts. The checkpoint
  separates that from staying revealed: whether scrolling the editor away
  confirms, leaves it pinned off-screen, or keeps it in an overlay; and what
  a source replacement that removes, moves or filters out the edited row
  does to the session.
- The body is one traversal scope (slice 4); Space or Enter on an active cell
  whose content is interactive at rest (a checkbox) reaches that content.
- **Per-cell state.** Before changing `TableItems::cell`, show a lazy
  retained cell adapter that creates state for mounted cells only. Controlled
  controls (a `Checkbox` built from a `bool` that reports a toggle) are
  considered only with their refusal, optimistic-presentation and callback
  timing specified.

Contract tests: each way of beginning, each ending with the application's
decision called once or not at all, refusals, sorting and filtering during
an edit, the edited row scrolled away, column removal and rebinding,
reentrant source changes from the save callback, and no session surviving
hiding or unmount.

#### Design checkpoint (2026-10-20, revised after review)

Built on slices 4 and 5 as they exist. The first version was reviewed by
Codex before implementation; the review found that focus pinning cannot keep
a refused edit's row mounted, that navigation after a save's callbacks and
at the table's ends was undefined, and gaps in eligibility, double-click
precedence, editor construction, the one-stop keyboard route and start
modes. This revision answers each; a second review confirmed them and led to
delivering completions to the table rather than the cell host, the Tab exit
depending on `cell_tab_stops`, and separate rules for read-only, disabled
and ineligible cells.

```rust
impl TableColumn {
    /// Whether cells of this column are edited in place by the table's
    /// cell editor. Default false.
    pub fn editable(self, editable: bool) -> Self;
}
pub trait TableItems {
    // ...existing methods...
    /// Whether the cell of row `index` in an editable column can be edited
    /// now. Default true. Answered from data the source holds.
    fn editable(&self, index: usize, column: &str) -> bool { true }
}
impl Table {
    /// Edits cells of editable columns in place. `editor` builds the editor
    /// for one cell when an edit begins. Replaces an earlier editor; the
    /// first description's wins on redescription, as for the table's other
    /// callbacks.
    pub fn cell_editor(
        self,
        editor: impl FnMut(&mut CellEdit<'_, '_>) -> Result<Element, RuntimeError> + 'static,
    ) -> Self;
    /// Whether interactive content inside cells is a Tab stop. Default true;
    /// with false the body is one stop (`VirtualList::item_tab_stops`).
    pub fn cell_tab_stops(self, stops: bool) -> Self;
}
/// One cell edit as the table's editor callback sees it.
#[non_exhaustive]
pub struct CellEdit<'a, 'b> { /* row, column, start, session, owner */ }
impl CellEdit<'_, '_> {
    pub fn row(&self) -> &str;
    pub fn column(&self) -> &str;
    pub fn start(&self) -> &EditorStart;
    /// Pass to the editor's `session` builder.
    pub fn session(&self) -> &EditorSession;
    /// The state owner for the editor's signals and choice controls.
    pub fn owner(&mut self) -> &mut UpdateContext<'b>;
}
impl VirtualList {
    /// Receives committed text typed while the list itself is focused.
    pub fn on_text_input(self, f: impl FnMut(&mut EventContext, &str) -> Result<(), RuntimeError> + 'static) -> Self;
    /// Keeps the item with this key mounted while the source holds it,
    /// whether or not it holds focus; `None` keeps nothing extra.
    pub fn keep_mounted(self, key: ReadSignal<Option<String>>) -> Self;
}
```

- **The edit belongs to the table.** The table keeps the open edit (row key,
  column key, session) in its own state, not in the cell, and creates the
  session from its own notifier, so completions reach the `Table` component's
  delivery even when the cell host is gone. The body keeps the
  edited row mounted with `VirtualList::keep_mounted` for the whole edit,
  focused or not, scrolled away or not (it is not kept in view). The row
  builder puts a table-owned host component in the edited cell instead of
  `TableItems::cell`. The edit closes on a terminal report, when the edited
  row or column leaves the table, when another edit begins, and when the
  table is hidden or unmounted; closing calls `EditorSession::end` and
  republishes the plain cell.
- **Eligibility.** A cell can be edited when its column is `editable`, the
  table has a `cell_editor`, and `TableItems::editable` answers true for it.
  The editor must be a session editor that takes focus: `TextField`,
  `IntegerInput`, `DecimalInput`, `Select`, or a custom editor following the
  protocol. Parameter-bound editors ignore sessions and are not cell editors;
  an application edits a parameter through a local editor and writes it in
  its save callback. Eligibility is checked when an edit begins and when Tab
  picks a destination, never during an open edit: an application that wants
  to stop an open edit revokes the editor's own permission, and the editor's
  rules apply. A read-only editor keeps focus and reports as slice 5 says
  (`Refused` with a changed draft, else `Unchanged`). A disabled editor loses
  focus, or never takes it; the edit stays open as its reports leave it, and
  Escape on the body cancels an open edit whose editor does not hold focus.
- **Construction.** The host calls the closure once per edit, from its first
  `update`, with its `UpdateContext` as the state owner (so a `Select` needs
  no `Dynamic`), and keeps the editor and its state across updates and row
  rebuilds; same-key rebuilds of the row reconcile the host without calling
  the closure again. A closure error fails that flush and is not retried: the
  edit closes without an editor and the body keeps focus. The host focuses
  the editor from its delivery after the editor mounted, with
  `request_focus_within`, so a composite editor's first focusable part takes
  focus.
- **Begin.** Each begin makes the cell active (row and column). On an
  eligible cell:
  - Enter begins with `SelectAll`; on other cells Enter still runs
    `on_activate`.
  - F2 and a double-click begin with `CaretAtEnd`.
  - Committed text typed while the body is focused begins with
    `Typed(text)`, through `VirtualList::on_text_input`. Text that is only
    whitespace does not begin an edit, since Space selects. A `Select` editor
    ignores the start. The body has no text input state, so typing never
    starts a composition; an IME user begins with Enter or F2.
  - Begin on the cell whose edit is open (a refused edit the user left)
    focuses its editor again. Begin on another cell closes the open edit,
    discarding a refused draft, and opens the new one.
- **Double-click.** `TableRow` counts primary presses with a `ClickCounter`
  per cell: a press on another cell, or with Shift, Control, Alt or Super
  held, starts a new count. The first press is left unhandled, as today, so
  the body selects the row and takes focus. The second press of a series is
  also left unhandled for selection, and begins the edit; the editor's focus
  request comes from its delivery, after the body's, so the editor wins.
  Content inside the cell that handles the press (a checkbox) takes it first
  and no edit begins.
- **End.** The host acts only on session reports; it never intercepts keys or
  infers completion from pointer positions. Help, composition, a selection
  drag and an open `Select` list therefore take Escape first, as slice 5
  specifies, and a press that leaves focus in the editor (cell padding, a
  scrollbar) leaves the edit open. The table processes a report in its own
  delivery, which comes after the save's callbacks have run, so a sort or
  filter they caused applies to the destination. When the edited row is no
  longer in the table at that point, the edit closes without navigation and
  the body takes focus if the editor held it; otherwise:
  - `Cancelled`, and `Saved` or `Unchanged` from Enter or a choice, close the
    edit; when focus is still in the editor, the body takes focus with the
    edited cell active, if its row is still in the table.
  - `Saved` or `Unchanged` from Tab or Shift+Tab close the edit and begin one
    with `SelectAll` on the next or previous eligible cell: the next eligible
    column in the current column order, then the first or last eligible
    column of the next or previous row in the current row order. Past the
    last cell of the last row, or before the first of the first, the body
    takes focus on the edited cell; the next Tab continues by ordinary
    traversal from the body, which leaves the table with
    `cell_tab_stops(false)` and otherwise reaches interactive cell content
    after the body. A destination row off-screen is made active, which
    reveals it, and its host mounts with it.
  - `FocusLeft` with `Saved` or `Unchanged` closes the edit without moving
    focus.
  - `Refused` keeps the edit open with its draft and the row mounted.
  - A report delivered after focus left the editor for elsewhere (a delayed
    delivery after a callback error) closes or keeps the edit by its outcome
    but never moves focus.
- **Source changes during an edit.** Rows are keyed, so a reorder (sorting)
  or a same-key replacement keeps the host, the editor and its draft; the
  editor follows external writes by its own rules. When the edited row leaves
  the published rows, by deletion or by a filter, the edit closes and its
  draft is discarded, with no report (the editor is gone), and when the
  editor held focus the body takes it; the selection is not pruned, as today.
  A removed column closes the edit the same way.
- **Cell content at rest.** `TableItems::cell` keeps its signature: a cell
  whose control needs state uses a retained component that creates it in
  `update` (shown in slice 7) rather than a signal per row in advance. With
  `cell_tab_stops(false)` the body is one Tab stop; F2 on a non-editable
  active cell with focusable content moves focus into that content (Enter
  keeps `on_activate`, Space keeps selection), and Escape that the content
  leaves unhandled returns focus to the body. That is the keyboard route the
  custom-widget guide requires when descendant stops are suppressed.
- **Not in this slice:** moving down after Enter, range selection. Both are
  in the [follow-up index](../follow-ups.md).

Contract tests, on both key paths where keys are involved: each begin (Enter
with `SelectAll`, F2 and double-click with the caret at the end, typed text,
whitespace not beginning, Enter on a non-editable column running
`on_activate`, `TableItems::editable` false); each end (Escape, Enter, a
choice, Tab and Shift+Tab across columns and rows and past both ends,
`FocusLeft`); a refused edit staying open when focus leaves and while
scrolled away, then focused again by Enter or discarded by another begin; a
save callback that sorts or removes the edited row before Tab's destination
is chosen; a filter and a deletion closing the edit; a closure error; an
editor that cannot take focus; help, an open list and a drag taking Escape
first; a read-only revocation keeping focus and a disabled editor closed by
Escape on the body; double-click with a modifier and on an interactive cell;
`cell_tab_stops(false)` with F2 into a checkbox and Escape back; a `Select`
editor built with the host's owner; no edit surviving hiding or unmount.

### 7. An editable-table example

Status: done in 0.3.43. The example is `crates/pert-ui/examples/inventory`;
its headless session and timings are recorded in
[verification](../../docs/verification.md#table-editing-plan). The
gallery edits the Collections table's Note column in place. Review fixes in
0.3.44 made the row commands follow every filter, the search included,
bounded the store's numbers and checked its edit groups in order, and
widened the session's checks.

- `crates/pert-ui/examples/inventory` (name settled in the slice): the probe's
  editor rebuilt on the slices above, without the probe defects, with the
  application-owned store and undo, the toolbar and its commands, copy and
  paste of rows, and a headless mode in its tests, as the `log_explorer`
  example has.
- The gallery's table page edits cells in place instead of reporting
  activation.

## Optional follow-ups

Taken up only when an application needs them, each with its own checkpoint:
range selection of cells and multi-cell copy and paste; a fill handle; a
row-level edit transaction across several cells; the session protocol in
`Tree` rows and `PropertyGrid`. They are in the
[follow-up index](../follow-ups.md), with Enter moving down, sessions for the
editors slice 5 left out, controlled controls and the removed-column rule.

## Not in this plan

A spreadsheet or formula component in the library, an undo stack owned by
the library, and accessibility integration.

## Review record

- 2026-10-19, Codex (`codex exec -s read-only`), review of the first draft,
  the findings log and the probe sources. Confirmed the Enter-only
  activation, the per-component Tab stop, the numeric Escape branch, the
  missing icon-command constructor and icon roles, and the private
  inspection serial. Corrected the draft: `text_key_event` can claim Tab on
  the native path, `DraftEditorController::begin` already begins with the
  text selected, cell state can be created lazily in a retained component or
  with `Signal::sibling`, `PropertyGrid` already hosts keyed editors, and
  the flush order does not explain the select-all loss. Found that the
  draft's slice 4 could not coordinate editors through their `on_save`
  callbacks (no unchanged or refused outcome), that one-press Escape needs a
  defined precedence over the editing phase, and the probe's lifecycle and
  enablement defects listed above. Its proposed order (correct the
  background, isolate the numeric loss, design completion and traversal,
  prove them with a custom collection, then integrate) is the order above.
  Each correction was checked against the code before it was adopted.

Implementation, 2026-10-20: slices 4, 5 and 6 had Codex design checkpoints,
recorded in their sections. Slice 5 was revised twice and slice 6 once
before approval. Every slice had an independent Codex implementation review
whose findings were fixed (0.3.33, 0.3.36, 0.3.38, 0.3.40, 0.3.42 and
0.3.44), as
the squash commit 5a6e451 states. Per-slice gate runs were not recorded;
[verification](../../docs/verification.md#table-editing-plan) records the
workspace gate on the merged result.

2026-10-20 audit ([plan-audit-repairs](../plan-audit-repairs.md)): several
contract tests of slices 1 and 4 to 6 are missing, some rustdoc is stale
after slice 4, and the manual lost the sorted-row caveat. That plan
repairs them.
