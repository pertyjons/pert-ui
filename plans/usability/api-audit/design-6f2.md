# Part 6f2 design checkpoint: editing and collection capabilities

Part 6f2 of [the fix plan](fix-plan.md#6f2-editing-and-collection-capabilities)
adds public API, so its shape is settled here before any code. Codex reviews
it, then the maintainer approves it. It closes round 1's F7b, F13, F15 and
F27 ([the round 1 table](../../archive/developer-usability.md)). The evidence is
triage [group B](../runs/triage/group-B.md) (F13) and
[group C](../runs/triage/group-C.md) (F7b, F15, F27).

Status: **revision 4, approved by the maintainer on 2026-10-26** with the
recommended answers to D1, D2 and D3, after Codex's first review and
three rechecks found it ready (review record at the end). Three choices are the maintainer's
([decisions](#decisions-for-the-maintainer)); each section states the
recommended answer.

## What the testers built by hand

- **F13:** a callback that starts background work. Its `EventContext` has
  no `spawn`. Both task-1 runs found their own way: one queued
  `Runtime::spawn` through `EventContext::defer`, as the `diagram_editor`
  example does; the other used a hand-made thread and a mailbox, which is
  what the guide taught. Since 5c the guide's worker section shows the
  `defer` pattern. Its nesting is the reason for the convenience.
- **F15:** starting a row's rename from code. `InlineText` begins editing
  only on a double-click or a fresh Enter/F2 while focused. The task-2
  tester focused the label and sent a synthesized F2
  (`r1-2-claude/submission/src/lib.rs:274-288`). That fails while the
  physical key is still held, because the press is classified `Repeated`
  (`input.rs:547`).
- **F27:** `InlineText` in a list row takes every primary press
  (`inline_text.rs:442-450`), so clicking a row's title never selects the
  row. The tester's workaround, `enabled(selected)`, also made the label
  unfocusable and drew it disabled.
- **F7b:** while the user types in a `CollectionFilter`'s field, the
  collection has no focus, so a filtered-out active item stays active and
  hidden (`collection_filter.rs:758-760`). An action on "the selected row"
  then acts on a row the user cannot see. The codex run wrapped the list in
  a `SelectionGuard` component that rewrote the selection in
  `deliver_notifications`.

## 1. `EventContext::spawn` (F13)

```rust
impl EventContext {
    /// Starts `work` on a background thread from a callback and hands its
    /// result to `deliver` on the UI thread, as `Runtime::spawn` does.
    pub fn spawn<T: Send + 'static>(
        &self,
        work: impl FnOnce() -> T + Send + 'static,
        deliver: impl FnOnce(T) -> Result<(), RuntimeError> + 'static,
    ) -> Result<TaskReceipt, RuntimeError>;
}
```

- **What it is:** a queued start, exactly the deferred operation
  `|ui| ui.spawn(work, deliver).map(|_| ())`.
  - The thread starts at the beginning of the next flush, before component
    evaluation, as every deferred operation does.
  - Once started, the result follows `Runtime::spawn`'s
    application-lifetime rules: arrival order, a panic returned as
    `BackgroundPanicked`, nothing delivered after shutdown. The owner below
    governs only the queued start.
- **The receipt** reports the start, not the result:
  - `Succeeded`: the thread started.
  - `Failed(error)`: `Runtime::spawn` refused, with its error, and nothing
    started. Examples are `BackgroundFull` at `MAX_BACKGROUND_TASKS`
    waiting tasks, `BackgroundSpawn(_)` with the operating system's reason
    and `Exhausted` (`application.rs:229-235`). As with every deferred
    operation, the error also propagates from that flush.
  - `Cancelled`: the owner was gone before the start, or the runtime shut
    down first.

  It is `#[must_use]` with `TaskReceipt`'s existing reason.
- **No cancel handle.** Code that must discard an undelivered result
  queues `Runtime::spawn` with `defer` and keeps its `BackgroundTask`, and
  the rustdoc says so. Returning both would need a new pair type for a
  case no tester had.
- **The owner (decision D2):**
  - **Recommended: the window, as `defer_detached`.** The start runs while
    the window is open, even when the widget is hidden or removed in the
    same callback. A Save button that closes its dialog is the common
    case, and the `diagram_editor` example uses `defer_detached` for
    exactly that (`app.rs:852`).
  - **Alternative: the widget, as `defer`.** The start is cancelled when
    the widget is unmounted or logically hidden before the next flush
    (`tasks.rs:252-256`). This is symmetric with `defer`'s name, but it
    reintroduces that loss.
- **Errors at the call:** those of `defer_detached`, that is
  `TaskQueueFull` at `Runtime::MAX_TASKS` and the phase/shutdown errors,
  without accepting the operation.
- **Documentation:**
  - `Runtime::spawn`'s paragraph on callbacks points to
    `EventContext::spawn` first, and to `defer` plus `Runtime::spawn` for
    a cancellable task.
  - `EventContext::defer`'s sentence "It is also how a callback starts
    background work" moves to `spawn`.
  - The guide's "Worker results and live data" switches its example
    (`guide/state.md:285-318`) to `cx.spawn`, per the fix plan. Its prose
    after the example (`:320-323`, "runs only while the button is mounted
    and shown") is rewritten for the chosen owner.
  - The data-contract table in `guide/getting-started-continued.md:40`
    names it beside `Runtime::mailbox`.
  - `docs/contracts.md`'s background-work paragraph (`:674`) gains one
    sentence.
- **Tests:** behavior tests fail with the change reverted.
  - A button callback's `spawn` delivers its result, and the receipt reads
    `Succeeded` once the thread started.
  - `Failed(BackgroundFull)` at `MAX_BACKGROUND_TASKS` waiting tasks, and
    that flush returns the error.
  - The owner:
    - with D2 as recommended, the start still runs after the button is
      removed in the same callback, and it is `Cancelled` when the window
      closes before the next flush;
    - with the alternative, removal and hiding cancel it.
  - It is `Cancelled` when the runtime shuts down before the next flush.

## 2. Starting an edit from code (F15)

### Why the planned `session` builder does not reach F15

The fix plan names `InlineText::session(&EditorSession)`. It follows the
"Sessions for more editors" follow-up. The session protocol binds an
editor to one edit when the host builds it:

1. The host calls `EditorSession::begin`.
2. It then describes the editor with `.session(&session)`.
3. The editor applies the start from its next delivery
   (`editor_session.rs:106-112`, `text_fields.rs:1741-1760`).

That suits a table, which mounts a fresh editor per edit. It does not let
code begin a rename of a label that is already mounted, for two reasons:

- `begin` wakes nothing, so a mounted `InlineText` would start only after
  its row is described again. `ListItems::build` runs only when the source
  is replaced or the mounted rows change (`virtual_list.rs:58-66`), so the
  tester would have had to rebuild every row to start one rename.
- `EditorSession::new` needs a host's `Notifier`, which application code
  outside a component does not have.

The F15 evidence is one need: start editing this label now, as F2 does,
from a list's `on_activate` or a menu command.

### Recommended: a `TextCommand` that begins editing (decision D1)

```rust
#[non_exhaustive]
pub enum TextCommand {
    // …existing variants…
    /// Begins editing in an editor that has a resting state, as its own
    /// editing key (Enter or F2) does.
    BeginEdit,
}
```

`TextCommand` is already `Copy` and `#[non_exhaustive]`
(`text_input.rs:138-140`), so the addition is not breaking.

**The contract changes.** Today a text command "never moves focus"
(`text_input.rs:134-137`, `runtime.rs:3217`). `BeginEdit` is the one
command that requests focus, and the change is made together in four
places:

- the `TextCommand` rustdoc;
- `Component::text_command`;
- `Runtime::text_command` and `EventContext::request_text_command`;
- `docs/contracts.md`'s text-command paragraph (`:804`).

**What `Handled` means.** The editor began editing. Its draft is open with
its text selected, and inspection reports `editing: true`. The editor has
also requested focus for its editing part, as `DraftSlot::begin` does
(`draft.rs:997`). That request settles at a flush:

- **`Runtime::text_command`** delivers synchronously and does not flush,
  so focus moves at the next flush. The rustdoc says so, and
  `testing::Driver` users settle as after any action.
- **`EventContext::request_text_command`** is delivered in the event phase
  of a flush (`runtime.rs:3213-3226`). Its focus request settles at the end
  of that flush.

Admission is unchanged: foreign, unmounted, hidden and modal-blocked
targets are refused before the editor sees the command. Routing is
unchanged too: a non-editor target passes the command to the focused
component inside its subtree (`runtime.rs:5419-5440`). A resting label has
no focused descendant, so code targets the label itself; the rustdoc says
so, and the routing test covers it.

**Answers.** One rule covers every editor with a resting state: a resting
editor does exactly what its Enter/F2 path does now, and answers
`Unavailable` wherever that path does nothing.

- **`InlineText`** (`inline_text.rs:462-470`):
  - at rest, enabled and not refusing its binding, it begins editing with
    the text selected and answers `Handled`;
  - while disabled or refusing its binding, `Unavailable`;
  - already editing, `Unavailable`; the draft is not reselected.
- **`InlineInteger` and `InlineDecimal`:** the same-kind rule applies,
  since they have a resting state that Enter/F2 enters
  (`number_inputs.rs:234`, `:1474-1483`). Today they answer
  `NotAnEditor` at rest (`number_inputs.rs:1651-1656`). With `BeginEdit`
  they take their Enter/F2 path, with one deliberate narrowing:
  - **during a gesture:** `Unavailable` while `held()` is true
    (`number_inputs.rs:1007-1010`), that is during a step drag, a pressed
    stepper or a held step key. The key path refuses only the first two
    (`:1468-1470`). A command from code must not start text mode under a
    held arrow's preview, and the gesture keeps its own lifetime: it ends
    on its release, focus loss, hiding or unmount as today;
  - **disabled:** `Unavailable`, as the key path ignores the key;
  - **refused since mount** (no valid value to show, `:1477-1480`):
    `Unavailable`;
  - **read-only** (no edit authority, a non-editable parameter, a refused
    bound value): `Handled`. The command enters read-only text mode exactly
    as Enter/F2 does (`inline.rs:213-219`), so the value can be selected
    and copied and nothing is saved;
  - otherwise they begin text mode and answer `Handled`.

  Already in text mode they keep answering as their `IntegerInput` field
  does, which is `Unavailable` for `BeginEdit`.
- **Editors without a resting state** (`TextField`, `TextArea`,
  `SearchField`, the number inputs, `DraftEditor`): `Unavailable`. They
  are always editing.
- **`SelectableText`:** `Unavailable`, which its handler already answers
  for every command other than Copy and SelectAll
  (`selectable_text.rs:558-559`), in line with the hook's rule for a
  command that does not apply (`runtime.rs:1145-1147`).
- **Components without a `text_command`:** `NotAnEditor`, the default.

The command needs no privileged path: a custom editor answers it from
`Component::text_command` as it answers the other commands. The tester's
code shrinks to `ui.text_command(&title, TextCommand::BeginEdit)`, and
nothing depends on key state.

**`session` stays in the follow-up.** "Sessions for more editors" keeps
`InlineText` in its list, with a note: F15 was met by the command, and a
session editor still needs a host that describes the label again for each
edit. The fix plan's F15 row and the part's `api/` line are amended to
match.

**Alternative A: the plan as written.** Add `InlineText::session`: it
applies the start at its next update after the attachment changes, and
reports Enter, Escape, Tab and focus loss as `TextField` does. The guide
recipe would show a retained row component that hosts the session and
describes its label again to begin. That is a complete session editor,
but it costs a host component per row for what one command does, and no
round 1 code hosted one.

**Alternative B: both.** The command now, and the session editor as well.
Recommended only if the maintainer wants the follow-up closed in this part.

### F15's documentation line

`Runtime::key_input` gains this note: "A synthesized press of a key that
is physically held is a repeat (`KeyTransition::Repeated`), as the
keyboard state classifies it. Controls that act on a fresh press, such as
`InlineText`'s F2, ignore it. To start editing from code, use
`TextCommand::BeginEdit`." The `testing::Driver::key` docs get the same
pointer, since the driver calls `key_input`.

### Tests

Behavior tests fail with the change reverted.

- **The tester's failure:** hold F2 while focus is elsewhere (a button),
  and the resting label stays resting. Then `BeginEdit` on the label
  answers `Handled`, and after a flush focus is in its editor and
  `text_input` replaces the selected text.
- **Timing:**
  - direct `Runtime::text_command`: editing is true at once, and focus
    moves into the editor only at the next flush;
  - queued `request_text_command` from a list's `on_activate`: the active
    row's rename is open and focused when that flush returns.
- **Routing:**
  - on the label itself, the command works;
  - on its row (a non-editor) with no focused descendant, it answers
    `NotAnEditor`;
  - on a row whose resting `InlineText` (without `in_row`) has focus, it
    reaches that label through the descendant route (`runtime.rs:5437`)
    and answers `Handled`.
- **Admission, direct:** a hidden label returns `Hidden` and a
  modal-blocked one `ModalBlocked`, with nothing begun.
- **Admission, queued:**
  - a `request_text_command` whose target is hidden or unmounted before
    the flush is dropped, and nothing begins;
  - one whose target is modal-blocked by then makes the flush return
    `ModalBlocked`, and nothing begins (`runtime.rs:6125-6150`).
- **`Unavailable`** while editing, while disabled and while the binding is
  refused, with nothing changed.
- **Inline numbers:**
  - `BeginEdit` begins text mode at rest;
  - it answers `Unavailable` during a step drag and during a held step
    key, and the gesture then ends on its release as before;
  - it answers `Unavailable` while disabled;
  - a read-only local row and a parameter-bound row without edit authority
    enter read-only text mode, and nothing is saved.
- **Compatibility checks** pass with or without the change: `TextField`
  and `SelectableText` answer `Unavailable`.

## 3. `InlineText` as a list-row label (F27)

```rust
impl InlineText {
    /// Makes the label part of a selectable row: a single press reaches
    /// the row, and a double-click edits. Default `false`.
    pub fn in_row(mut self, in_row: bool) -> Self;
}
```

### Recommended rules (decision D3)

**At rest the label is not focusable** (`focusable` is false while not
editing). A press on it therefore focuses the nearest focusable ancestor,
the list (`pointer.rs:496-516`). The list stays the collection's keyboard
owner. The label's click series is not reset by a focus move, because the
label never had focus. Its own Enter/F2 is unreachable at rest. The
keyboard route is the list's: `on_activate` (Enter) or an application
command issues `TextCommand::BeginEdit` for the active row's label.

**Presses at rest:**

- A primary press with Shift, Control, Alt or Super held resets the click
  series and is `Ignored`. The list selects with those modifiers.
- An unmodified primary press counts in the label's `ClickCounter` (500
  ms, four logical pixels):
  - the first press of a series is `Ignored`, so the list makes the row
    active, selects it, and may arm a reorder drag;
  - the second press of the series begins editing, resets the series and
    is `Handled`.
- A release answers like the press it ends: `Handled` after a press the
  label handled, `Ignored` otherwise. The label keeps that one flag per
  press and clears it in `input_reset`.
- A press the label ignored is not captured by it: the list's own capture
  rules apply, as for any content that does not handle a press
  (`virtual_list.rs:419-421`).
- **What breaks a series.** `ClickCounter` leaves resets to its owner
  (`click.rs:27-33`). In row mode the label resets the series:
  - when its hover ends (`Component::hover_changed` with `false`). Outside
    a drag session the runtime updates the hover path on every pointer
    event, also while another component holds the capture
    (`pointer.rs:466-482`). So a click on label A, then on label B, then on
    A again within the interval is three single clicks;
  - when a reorder drag starts from it. Adopting a drag session clears the
    hover path (`runtime/drag.rs:364-366`), which ends the label's hover
    wherever the pointer is. Pointer events during the session bypass hover
    handling (`pointer.rs:367-370`), so no hover returns before the drag
    ends. A drag that stays on the label and returns to its start
    therefore also breaks the series;
  - on a modified press, on `input_reset`, on hiding, on disabling and
    when an edit begins.

**While editing:** the rules are today's. The editor takes presses on its
text, and a press on the padding stops at the label
(`inline_text.rs:433-440`). When editing ends by key, the label does not
request focus for itself, since it is not focusable at rest. Focus falls
back to the nearest focusable ancestor, the list, by the documented
fallback (`runtime.rs:1343-1350`). Focus leaving by other means saves, as
today.

**Precedence with reordering:** a single press never renames, so a
selected row can be dragged by its label. A double-click renames, and its
second press does not reach the list.

**Alternative (the plan's F27 wording): a press on the selected row also
edits.** The builder would take `selected: impl Into<ReadSignal<bool>>`,
read in update with a paint dependency. An unmodified press on a label
whose row was selected at its last update would begin editing and be
handled. This costs two things:

- In a reorderable list, a selected row can no longer be dragged by its
  label, since the press that would arm the drag begins the rename. File
  managers avoid this by renaming on a delayed release, which the label
  cannot see: once the list holds the pointer, the release goes to the
  list.
- A press on the selected row renames even while the list lacks focus,
  for example right after typing in the filter field. The runtime focuses
  the hit target's ancestors and flushes before delivering the press
  (`pointer.rs:488-520`). A binding therefore cannot see focus as it was
  before the press, and the behavior can only be documented, not adapted.

### Common to both

- **Without `in_row`:** behavior is unchanged. The default press behavior
  is documented on `InlineText`: a single primary press is handled and
  focuses the label, a double-click edits, and a row around it never sees
  the press. This closes F27's documentation half.
- **Redescription:** `in_row` is an ordinary property, and a new
  description replaces it. Turning it on or off never ends a draft. Turning
  it on while the label has focus at rest makes it non-focusable, so focus
  falls back to its ancestor by the documented rule.
- **Inspection:** an `in_row` property (`Bool`) while the mode is on.
- **Tab traversal:** the label adds no Tab stop at rest. Other row
  content, such as the recipe's checkbox, keeps its own stop
  (`virtual_list.rs:873`). The rustdoc says this rather than calling the
  list one Tab stop.

### Tests

Pointer presses go through the runtime to a `VirtualList` with a
`ListSelection`, and each test sends complete press/release pairs.

- **Selection:** a single press on a row's label selects the row, does not
  edit, and leaves focus on the list.
- **Double-click:** on an unselected row, it selects the row and edits.
- **The series:**
  - two presses farther apart than the counter's interval do not edit;
  - A, B, A within the interval does not edit A;
  - a drag from a label onto another row, its release, and then a quick
    press on the first label does not edit;
  - a reorder drag that stays on the label and returns to its starting
    point, its release, and then a quick press does not edit;
  - Ctrl+press twice toggles the selection twice and does not edit;
  - a modified press between two unmodified ones resets the series.
- **Reordering:** a drag that starts on a selected row's label reorders
  and does not edit (the list is `reorderable`).
- **Leaving an edit:** Enter ends the edit with focus on the list, and
  arrows then navigate.
- **Tab** passes over the labels at rest.
- **Redescription:** turning `in_row` off and on during a draft keeps the
  draft.
- **Without `in_row`:** a press still focuses the label and the row stays
  unselected. This is a compatibility check and passes with or without the
  change.

## 4. `CollectionFilter`'s hidden active item (F7b)

```rust
/// What a `CollectionFilter` does with an active item that a filter change
/// leaves hidden while focus is outside the collection.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum HiddenActive {
    /// It stays active and hidden, and is shown again when the filter
    /// clears. The default, and the behavior before this option.
    #[default]
    Keep,
    /// It moves to the nearest row still shown, as it does while the
    /// collection has focus.
    Nearest,
    /// No item stays active.
    Clear,
}

impl CollectionFilter {
    /// Chooses what happens to a hidden active item while the collection
    /// lacks focus. Default `HiddenActive::Keep`.
    pub fn hidden_active(mut self, policy: HiddenActive) -> Self;
}
```

### When it acts

The policy is evaluated once per delivery that changed what the filter
shows (`deliver_notifications`, `collection_filter.rs:734-775`). That is a
query applied, a table's or tree's source replacement, or both. It is
evaluated after the change is applied, against the active key `A` that
the selection holds at that moment.

- **Focus in the collection** (as read at that update, the same moment as
  today): today's rule holds whatever the policy. The active item moves
  to the nearest row. When no row remains and the query is not empty,
  focus moves to the field and the hidden active item stays
  (`collection_filter.rs:758-772`). That condition is unchanged.
- **`A` is `None` or shown:** nothing happens.
- **`Keep`:** nothing happens.
- **`Clear`:** `set_active(None)`. This is a state rule, not a transition
  rule. An active item hidden after the change is cleared whether it
  became hidden now or earlier, and whether or not a row remains.
  - "Shown" is a crate-internal membership test per target: the filtered
    list's `index_of`, the table controller's shown rows, and the tree's
    flattened rows with the temporary expansion.
  - The rustdoc states the invariant with its scope. Under `Clear`, when
    a delivered filter change is evaluated with focus outside the
    collection, the active item is shown or there is none afterwards. It
    says nothing about later application writes, nor about the focused
    branch, which keeps a hidden active item when no row remains.
- **`Nearest`:** `set_active(Some(nearest))`, but only when this change
  hid `A` and a row remains. That is the transition
  `FilterOutcome::nearest_active` describes. Otherwise it keeps `A`, as
  the focused branch does:
  - `A` was already hidden before the change;
  - the transition belonged to an earlier active key;
  - no row remains.

### Transport for source replacements

Tables and trees publish a replacement as `(revision, nearest_active)`
through their `replaced` signal (`table.rs:676`, `tree.rs:451`), and
`CollectionFilter` rebuilds an outcome from it (`collection_filter.rs:746`).
The crate-internal payload becomes `(revision, Option<(hidden, nearest)>)`.
`hidden` is the active key the replacement hid, computed by the controller
from its before/after membership, and `nearest` is its nearest shown row,
if any. Nothing public changes.

- **Correlation:** `Nearest` uses a transition only when `hidden == A`.
  An application that moved the active item before delivery is left
  alone.
- **Coalescing:** the signal holds the latest replacement. When two
  replacements arrive before one delivery, only the last one's transition
  is seen. If the first hid `A`, the second finds `A` already hidden, so
  `Nearest` keeps it. `Clear` is unaffected, since it reads state. The
  rustdoc states this.
- **Precedence:** a query change and a replacement in the same delivery
  apply the query over the new source, as today (`:734-745`). The query's
  outcome supplies the transition, and the replacement is marked handled.
  `Clear` again reads state.

### The writes

- They go through `ListSelection::set_active` and are application-origin
  writes, as the focused branch's write is reported (`:761-763`).
- In `SelectionMode::Single` the selection follows the active item. In
  `Multiple` only the active item changes: hidden selected keys stay
  selected, because "the selection is never pruned"
  (`collection_filter.rs:230-231`) is unchanged.

### Other rules

- **Redescription:** the policy is an ordinary property. A new description
  replaces it, and it applies from the next delivered change. Changing it
  acts on nothing by itself.
- **Inspection:** a `hidden_active` property (`Text`: `keep`, `nearest`,
  `clear`).
- **Naming:** the type sits beside `FilterMatcher` and `FilterOutcome` in
  the filter module. Its name says what it governs, as `SelectionMode` and
  `ExpandMode` do. An alternative name is `FilterSelection`.

### Documentation

- The "Selection and focus" paragraph on `CollectionFilter`, and
  `FilterOutcome`'s sentence "moves it only while the collection has
  focus", name the option and the `Clear` invariant.
- For `Multiple` selections, the rustdoc gives a guard over the selected
  keys rather than the active one, since an action on the selection acts
  on every selected key:
  `filter.items().combine(&selection.selected(), |rows, keys| keys.iter().all(|key| rows.index_of(key).is_some()))`.

### Tests

The behavior tests run for a list, a table and a tree target, and fail
with the policy logic removed.

- **Query typed in the field (collection unfocused):**
  - `Nearest` moves the active item to the next shown row, or to the
    previous one at the end;
  - `Clear` leaves no item active.
- **Zero rows remaining:** `Clear` clears.
- **Already hidden:** the application sets the active item to a hidden
  key, then the query changes, and `Clear` clears.
- **Source replacements:**
  - a table's replacement that hides the active row: `Nearest` moves it,
    and `Clear` clears it;
  - a tree's replacement that hides the active node, by the flattened
    rows with the temporary expansion: the same;
  - two consecutive replacements before one delivery: `Clear` clears;
  - a replacement together with a query change: `Nearest` follows the
    query's outcome.
- **`Multiple` mode:** under `Nearest` the active key changes to the
  nearest row and the hidden selected keys stay selected. Under `Clear`
  the active key is `None` and the selected keys stay.

Compatibility checks pass with or without the change:

- `Keep`, unfocused: the item stays hidden;
- `Nearest` keeps the item when no row remains, when it was already
  hidden, when the active key changed before delivery, and after two
  consecutive replacements (the stale transition);
- the focused branch under every policy gives today's result, including
  zero results with a non-empty query, where focus moves to the field and
  the hidden active item stays.

## 5. The guide recipe (F15, F27, F7b)

The list recipe in `guide/layout.md` (`:476-700`) is the fix plan's
demonstration for all three findings. It is migrated completely, through
the facade only, and its assertions cover the new behavior:

- **Rows:** `Rows` passes the title row nothing new. The retained `Title`
  builds its inner label as `InlineText::new(&shown).in_row(true)`
  (`layout.md:588-592`).
- **Activation:** `on_activate` begins the rename instead of writing
  `opened` (`:646`). Its `EventContext` belongs to the list, so
  `cx.id()` names it. It queues
  `cx.defer(move |ui| { let label = ui.find_descendant(&list, …)?; … ui.text_command(&label, TextCommand::BeginEdit) })`,
  where the path is the row key, then `title`, then `editor`. The
  `opened` signal and its assertion are removed, or kept on a separate
  "Open" button if the recipe still needs to show activation by key.
- **Assertions:** the title test no longer synthesizes F2 (`:686-692`).
  - Enter on the active row opens its rename, `text_input` replaces the
    title, and Enter saves it.
  - A press on another row's title selects that row.
  - A double-click on a title edits it.
- **The filter:** it uses `.hidden_active(HiddenActive::Clear)`. The
  "typing in the field hides the selected row" step then asserts that no
  row is active and Delete is disabled. The derived `selected_shown`
  signal (`:638-640`) stays as Delete's guard. Under `Clear` it is not the
  same as "a row is active", because the focused branch and application
  writes can still leave a hidden active item.
- **Prose:** the current `InlineText` press wording, added in 5c, is
  replaced by the row rules above.

## 6. Scope, versions and evidence

- **`api/` diff** (revised from the plan's line):
  - `EventContext::spawn`;
  - `TextCommand::BeginEdit`;
  - `InlineText::in_row`;
  - `HiddenActive` and `CollectionFilter::hidden_active`;
  - their facade re-exports and prelude placement by the 6e rule.
    `HiddenActive` joins `CollectionFilter` wherever that is exported.

  No `InlineText::session` unless D1 is A or B.
- **Breaking:** no.
- **CHANGELOG:**
  - "Added" for 0.3.69 names each item.
  - "Changed" covers `TextCommand`'s focus contract for `BeginEdit` and
    the inline number rows' new answer to it.
  - "Documentation" covers the `InlineText` press behavior, which is
    unchanged in code.
- **Gate:** the public-API row: the workspace checks, the API listing,
  the diagnostics snapshots and focused contract tests. The guide doctest
  covers the recipe.
- **TODO.md:** no new entry is planned. The two 2026-10-26 entries from
  the 6f1 hand check (`diagram_editor` rename overlap,
  `DraftAlign::Center`) are not part of 6f2.
- **Follow-ups:** "Sessions for more editors" is updated as in section 2.
  "Controlled controls" is untouched. 6f3's controlled `InlineText` builds
  on `in_row`, and with D1 as recommended it no longer depends on a
  session.

## Decisions for the maintainer

- **D1 (F15):** how code starts an edit.
  - **Recommended:** `TextCommand::BeginEdit`, answered by `InlineText`
    and the inline number rows.
  - **A:** the planned `InlineText::session`.
  - **B:** both.
- **D2 (F13):** who owns `EventContext::spawn`'s queued start.
  - **Recommended:** the window, as `defer_detached`.
  - **Alternative:** the widget, as `defer`.
- **D3 (F27):** what edits in row mode.
  - **Recommended:** a double-click only. The label is not focusable at
    rest, and the keyboard route is the list's `on_activate` with
    `BeginEdit`.
  - **Alternative (the plan's wording):** a press on the selected row as
    well. It costs dragging a selected row by its label, and it edits even
    while the list lacks focus.

## Review record

- **Revision 1, Codex review (2026-10-26):** needs revision, with eight
  points and no blocker. The session argument, `TextCommand` being
  `Copy` and `#[non_exhaustive]` with both routes existing, and D2's
  recommendation were confirmed. Revision 2 addresses each point:
  1. **`BeginEdit` and focus:** the contract change in four places, what
     `Handled` guarantees, settlement timing per route, admission and
     routing unchanged, with tests (section 2).
  2. **Source replacements:** the "became hidden" fact for table/tree
     replacements; `Clear` made a state rule; `Nearest` correlated with
     the hidden key; coalescing and precedence defined, with tests
     (section 4).
  3. **The row press state machine:** explicit press, release, count and
     reset rules and the precedence with reordering. The recommendation
     changes to double-click only, and the label is not focusable at
     rest (section 3).
  4. **D3's focus claim:** the claim that a binding could include
     collection focus is removed. The pre-press focus limitation is
     disclosed under the alternative.
  5. **Inline numbers:** the same-kind rule is applied to `InlineInteger`
     and `InlineDecimal`, the current answer is corrected to
     `NotAnEditor`, and the TODO disposition is removed.
  6. **The guide recipe:** a complete migration is specified (section 5).
     The Tab wording is corrected, and the `Multiple`-selection guard is
     documented.
  7. **Tests:** the held-F2 test now holds the key while focus is
     elsewhere. Boundary tests are added, and compatibility checks are
     separated from behavior tests.
  8. **The receipt and the guide prose:** `Failed(error)` is defined
     generically, including `Exhausted`, and hiding is named for the
     widget owner. The guide prose is migrated, and the window owner is
     stated to govern only the start.
- **Revision 2, Codex recheck (2026-10-26):** needs revision, with five
  points and no blocker. Points 2, 4, 6 and 8 of the first review, and the
  focus part of point 1, were resolved; D1 to D3 were found fairly framed.
  Revision 3 addresses each point:
  1. **Interrupted series:** the label resets its series on hover loss,
     which the runtime delivers during another component's capture, and
     A-B-A and drag tests are added. (Revision 3 also documented a
     remaining case, a drag returning to its start. The second recheck
     found that the runtime excludes it; revision 4 removes it.)
  2. **Inline numbers:** read-only text-mode entry is kept as Enter/F2
     does it. `Unavailable` during any `held()` gesture, including a held
     step key, is a stated narrowing with its reason, and tests are added.
  3. **`SelectableText`:** it answers `Unavailable`, as its handler already
     does, and the test is classified as a compatibility check.
  4. **The `Clear` invariant:** scoped to evaluation with focus outside
     the collection. The guide's equivalence claim is removed, the
     focused zero-results case is a compatibility check, and the
     non-empty-query condition is stated as unchanged.
  5. **Tests:** positive descendant routing, queued admission, tree source
     replacements and counter resets are added. Unchanged `Nearest`
     outcomes are compatibility checks, and the `Multiple` tests assert
     the changed active key.
- **Revision 3, Codex second recheck (2026-10-26):** needs revision, with
  one point. The other open points were resolved, and D1 to D3 remain
  fairly framed.
  1. **The drag exception:** the documented exception, a reorder drag that
     returns to its start and is then counted as a double-click, does not
     exist. Adopting a drag session clears the hover path
     (`runtime/drag.rs:364-366`), which resets the series. Revision 4
     removes the exception, qualifies "every pointer event" as outside a
     drag session, and adds the returning-drag test.
- **Revision 4, Codex third recheck (2026-10-26):** ready for approval, no
  findings. It confirmed that the `VirtualList` reorder drag is an adopted
  runtime drag session (`virtual_list.rs:2209`, `runtime/drag.rs:364`), so
  the reset claim holds, and that the earlier points stay closed.
