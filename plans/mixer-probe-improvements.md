# Plan: improvements found by a mixer-panel probe

Status: proposed 2026-10-18 and revised after an independent Codex review;
being implemented on branch `mixer-probe-improvements` (slice 1 in 0.3.9
and 0.3.11, slice 2 in 0.3.10, slice 3 in 0.3.12, slice 4 in 0.3.13, slice 5 in 0.3.14, slice 6 in 0.3.15
to 0.3.17). Every type and method named below is a proposal until a
slice implements and documents it.

## Background

On 2026-10-18, after the notes application of
[app-author-improvements](app-author-improvements.md), the maintainer's
session built a second application against the public facade only, chosen to
exercise what the notes application did not: a mixer panel. Channel strips
hold application-owned parameters (gain, pan, mute and an input choice) edited
through `Slider`, `Knob`, `Toggle::for_parameter` and
`SegmentedControl::for_parameter`. A simulated audio thread publishes one
level per channel through `series_bridge` into `LevelMeter`s, and a master
level into a `LevelMeter` and a `Trace`. Parameter terminal and preview sinks
forward values to the engine and record an application undo history; a
`MenuBar` with registered commands, a bottom `Toolbar`, an inspector built
with `Dynamic` and `PropertyGrid`, channels added and removed through `Each`,
and workspace capture and restore complete it. It was driven headlessly with
simulated keys, captured with the CPU renderer and run natively on Fedora 44
GNOME/Wayland. The application lived in a session scratchpad; the
observations below are reported experiment results, not a reproducible
evidence record. The simulated audio thread was an ordinary thread and proves
nothing about real-time safety.

What worked well: the series bridge and the live widgets on the first
attempt, including peak hold and a frozen meter through `running_signal`; the
parameter families with their `for_parameter` controls;
`TerminalEvent::begin_value` as the undo baseline; commands shown with their
shortcuts in menus; `Each` keeping a strip's state while other channels are
added and removed; a complete workspace restore report; and a focused button
taking Space before an application Space shortcut.

What the author ran into, verified in code or by execution:

- **Startup validation errors.** `App` runs its initializer before the native
  window exists and supplies the first viewport when the window is created.
  The probe's native run printed `EmptyHitArea` errors for the inspector's
  `PropertyGrid` editors, and mounting and flushing before `set_viewport`
  reproduces them headlessly. The slice 1 checkpoint below found that the
  native errors were not a startup effect; the zero-area viewport case is.
- **Rows cannot fill a grown height.** A `Row` given spare height through
  `Element::with_grow` in a `Column` was 485 pixels tall, but with
  `Align::Stretch` its vertical `Slider` and `LevelMeter` stayed at their
  natural 215 pixels. A row's cross extent is its tallest child
  (`crates/pert-ui-runtime/src/containers.rs`, `cross_extent`), as slice 4 of
  the previous plan decided, so a fader and meter cannot fill a channel
  strip.
- **Reacting to a change takes a detour.**
  - Parameter sinks run where runtime writes are refused, so a toolbar
    readout of the undo depth, written from `on_outcome`, stayed stale: it
    showed 0 after two commits.
  - `ParameterHost::write_base`, used for undo and reset, has no dedicated
    base-change notification; it reaches a sink only by ending an active edit
    as `Rejected(Conflict)`. The engine is therefore updated in two places.
  - `MenuItem::toggle` and `MenuItem::radio` write a signal and have no
    `on_change`. A View › Density choice works today through `MenuItem::new`
    actions that set a retained `ThemeManager`'s density, with a bound
    selection mark (density is a theme setting, not in `UiSettings`),
    but the probe did not find that path; a transport flag forwarded to a
    producer thread went through a command for the same reason.
- **Library errors and lookups.**
  - `RuntimeError` never wraps `WorkspaceError`, by the documented error
    policy (higher-level errors wrap `RuntimeError`, not the reverse), so a
    command action that saves the layout wraps the library's own error in
    `ApplicationError::new`.
  - `Runtime::find`, the lookup the guides show, failed for the key path that
    inspection reported and returned `None` without a hint; `find_address`
    resolved it. `RegionLayout` contributes a documented region segment
    (`center`) that the application did not write.
  - `CaptureTarget::Window` takes the raw `u64` of a window; the main window
    is `Runtime::primary_window().id()`, which the probe did not find.
- **Command and control conveniences.**
  - A button that runs a command needs `command_context` and `invoke_command`
    by hand, while menus have `MenuItem::command`.
  - A chord without modifiers needs
    `KeyChord::new(CommandKey::Named(..), Modifiers::default())`.
  - An `InlineToggle` without an icon shows its name's first character, as
    documented, which reads as a broken control in a toolbar.
  - The icon set has `PlayArrow`, `Stop` and `Record` but no pause.
- **Presentation gaps.**
  - `ParameterMapping` is linear or logarithmic, and only logarithmic
    requires a strictly positive domain, so a control whose domain includes
    zero or negative values cannot concentrate resolution in part of its
    range.
  - `Container::background_role` selects a fixed role (theme changes to that
    role repaint it), so marking the selected strip cannot follow a binding
    directly.
- **Discoverability.** The prelude has `MenuBar` and `MenuItem` but not
  `CommandScope`, `KeyChord`, `CommandSource`, the live-data types
  (`series_bridge`, `SeriesConfig`, `SeriesSource`, `Delivery`, `Timestamp`),
  `LevelMeter`, `Trace`, `MeterAxis` or `EditOutcome`. Inspection bounds are
  found only under `details.box_model.border_box`.

## Working rules

The rules of [app-author-improvements](app-author-improvements.md#working-rules)
apply unchanged: one slice per branch with its rustdoc, examples, gallery,
manual and CHANGELOG; the maintainer's patch version bump per slice; an
independent Codex review before a slice is called merge-ready; the contracts
read before each slice; and the gate table there. Breaking changes are
permitted while the library is unpublished, with replacement usage recorded
and repository consumers migrated in the same slice.

Additionally:

- **Design checkpoint.** Slices 1, 3 and 4 begin with a chosen alternative,
  a compiling API sketch and an enumerated list of contract tests, reviewed
  before implementation starts. A slice is implementable only after that.
- Slice 1 changes the native startup path and takes the dependency, feature
  and platform row of the gate table, with native evidence recorded in
  [verification](../docs/verification.md). A mapping change in slice 5 that
  touches serialization also runs the serde configurations.

## Slices in order

### 1. Startup without a viewport

- First identify the flush that lays out initializer content before the
  native path supplies its viewport, and keep a regression fixture for it.
- Choose a policy that distinguishes three cases: no viewport supplied yet,
  an explicit zero-sized viewport (a minimized window), and a child that is
  genuinely zero-sized after layout. Alternatives: `App` sets a provisional
  viewport from its requested size before the initializer, or geometry
  diagnostics wait for a supplied viewport. A headless `Runtime` follows the
  same policy as the native one, stated separately if the provisional size
  comes from `App` configuration.
- Acceptance: no false geometry diagnostic for content mounted before the
  first viewport; an interactive component that is zero-sized after layout
  with a viewport is still reported; the first real size replaces a
  provisional one without presenting a frame at the wrong size.

Status: implemented in 0.3.9, as the checkpoint below decided, with the
Codex implementation review's fixes in 0.3.11: the report is pending while
any window's layout is pending, and withheld geometry issues are not logged
again. Tests: `crates/pert-ui/tests/foundation/startup_viewport.rs`.

#### Design checkpoint (2026-10-19)

Investigation, on Fedora 44 GNOME/Wayland with temporary instrumentation of
the native runner's flush and `set_viewport` sites: the runner supplies the
viewport in `resumed`, before any flush it drives itself, both for a minimal
`PropertyGrid` application and for the reconstructed mixer, and neither
reported a validation issue at startup. An initializer still runs before the
event loop and can flush explicitly, or call an operation that flushes,
against the default zero-sized viewport. The probe's native errors named
components with higher identities than a fresh mount produces, so they came
after the inspector was rebuilt, not at startup; a `SplitPane` whose
inspector pane is reduced to zero width reproduces the same issue
headlessly, and that is a genuinely zero-sized pane, which is still to be
reported. What remains is the zero-area viewport itself:

- A `Runtime` lays out against its default zero-sized viewport, so an
  initializer (or a headless test) that flushes before the first viewport
  gets `EmptyHitArea` issues for every interactive component it measures at
  zero width.
- A host that reports a minimized window as a zero extent, as `Viewport`
  documents, and flushes it, gets the same issues for the whole tree on
  every minimize, although nothing is presented. The native runner does not
  flush a hidden window, so this concerns headless and custom hosts.
- `Viewport` claims that no geometry is produced for a zero extent; layout in
  fact runs and assigns zero-sized boxes, which inspection reports.

Chosen alternative: geometry validation waits for a window's layout with a
nonzero viewport. A provisional `App` viewport is not needed, since no
runner-driven flush precedes the real one, and it would leave headless
runtimes and minimized windows unchanged. Layout keeps running at a zero
extent (skipping it would leave layout pending, which the flush loop and
input paths settle on), so the documented claim is corrected instead.

- The development validator evaluates geometry only for a native window
  whose present effective viewport has width and height and whose layout is
  not pending, so the geometry it checks was laid out against that
  viewport. Content flushed before the first viewport, and a window laid
  out while its viewport is empty, report no geometry issue; after a
  viewport change the report keeps the last validation, marked `pending`,
  until the flush that lays the window out. The native runner neither
  flushes nor lays out a hidden or zero-sized window, so this rule matters
  to headless runtimes, custom hosts that flush at a zero extent and
  initializers that flush; immediate revalidation from `set_viewport`, which
  takes `&self`, would need a signature change for no native benefit.
  (A first implementation keyed the rule to layout runs; components
  mounted with `Runtime::mount` that report their own boxes never pass
  through layout, so the rule reads the viewport at validation instead.)
- The rule is applied when validating, not where geometry is recorded, so
  every ingestion path (layout and the bounds copied on each component
  update) obeys it. For nodes of an unmeasured window every geometry-derived
  rule is skipped: `InvalidBounds`, `InvalidClip`, `PaddingExceedsBounds`,
  `EmptyHitArea`, `CollapsedParent`, `EscapesParent` and `UnexpectedOverlap`;
  its debug-overlay boxes are absent. Other windows, including in-view
  windows of a measured native window, and non-geometry rules are
  unaffected. An empty report while a window has no area is not evidence of
  a valid layout; the documentation says so.
- No public API changes. `Viewport`, `Runtime::set_viewport`,
  `Runtime::validation_report`, `ValidationCode` and `App::new` state the
  rule, and the diagnostics guide states that an initializer runs before the
  first viewport and that geometry read there is zero-sized.
- Release builds have no development validator and are unchanged.

Contract tests (facade, `foundation` area, debug builds):

1. A `PropertyGrid` with an editor, mounted and flushed with no viewport,
   reports no issue, and inspection shows its zero-sized box (documented).
2. Setting a nonzero viewport afterwards lays out and reports no issue.
3. Setting a viewport of zero width, and separately of zero height, after a
   nonempty layout with a geometry issue leaves that issue in a pending
   report until the next flush, which clears it; restoring a size
   re-evaluates after layout.
4. A repeated update of a bound interactive component while the window has
   no area reports nothing.
5. With a nonzero viewport, an interactive component in a `SplitPane` pane
   sized to zero is still reported as `EmptyHitArea`, and so is one in a
   collapsed in-view window of a measured native window.
6. A non-geometry rule (an invalid shortcut description) is still reported
   with no viewport.
7. A native secondary window (`prepare_detach`, `commit_detach`,
   `select_window`) with a zero viewport does not suppress the primary
   window's issues, and the primary's zero viewport does not suppress the
   secondary's.

Codex reviewed the first version of this checkpoint (2026-10-19): no
blocker. Applied: the rule is enforced centrally because component updates
copy retained bounds into the validator outside layout; retention until the
next layout is documented, since a minimized native window is not flushed; the
startup claim is limited to runner-driven flushes; in-view and native
secondary windows are tested separately, with both zero axes.

### 2. Cross-axis fill in rows

Status: implemented in 0.3.10. Decision taken while implementing: under
`Align::Stretch` a row's cross extent is the height it is given (its
allocation when arranged; a bounded offer while measured only places
children and never changes the row's measured size), clamped per child to
its maximum and raised to its minimum. A row given only its natural height,
by an ungrown column, stretches to its tallest child; one in a scroll area is
given the visible height. `Divider::vertical` and other `fills_row_height`
children follow. Tests: `crates/pert-ui/tests/foundation/row_stretch.rs`.

- Revisit the previous plan's slice 4 decision: under `Align::Stretch`, a row
  whose allocated height is bounded and larger than its natural height
  fills that height, so a child without a maximum reaches the row's
  allocation. Define "allocated" separately from an offered maximum during
  measurement. The default `Center` alignment and its placement stay.
- Expected results for: a grown row in a column, a row under unbounded
  height, child maxima, minimum overflow, nested rows, children using
  `fills_row_height`, rows inside scroll containers, hidden children and
  same-key redescription that changes alignment.
- A vertical `Slider` and a vertical `LevelMeter` draw correctly over their
  valid extents, with defined behavior when a readout cannot fit.
- A custom container can reproduce the result through the public layout
  contexts; update the layout contract and the layout guide.

### 3. Reacting to user writes and application changes

Status: implemented in 0.3.12 as the checkpoint below decided:
`MenuItem::on_change`, the refresh-mailbox pattern in the state guide and
contract, and `write_base`'s delivery order documented. Tests:
`crates/pert-ui/tests/surfaces/menu_changes.rs` and
`crates/pert-ui/tests/foundation/sink_follow_ups.rs`.

One decision governs three gaps, so they share a slice and its design
checkpoint.

- **Menu rows:** `on_change` for `MenuItem::toggle` and `MenuItem::radio`
  under the [user-write callback contract](../docs/contracts.md#user-write-callbacks): successful
  user writes only, delivered after the menu closes; application writes,
  re-choosing the selected radio value and refused writes report nothing;
  failures neither undo the write nor replay; handler replacement and removal
  on redescription follow the contract. `radio` erases its value type, so the
  sketch fixes the callback signature and how it receives the written value,
  which event context it gets and when work needs `defer_detached`.
  Restrictive `Permission` composition and cancellation on revocation are
  unchanged. The guide also shows the existing custom-action pattern with a
  bound mark, since it already covers settings such as density.
- **Sink follow-ups:** a way for an application to update bindings after a
  terminal or preview outcome, while writes stay forbidden during the sink.
  Decide the owner (runtime, host or registration), admission and capacity
  with a defined full-queue result, whether previews coalesce, and behavior
  on sink failure, `retry_outcome` (a retried delivery must not enqueue a
  second follow-up), host removal, unregistering and shutdown. Report a
  follow-up's own error separately from sink delivery failure. If it uses
  [component notifications](../docs/contracts.md#component-notifications),
  adopt their FIFO, removal, hidden-window and failure rules; otherwise
  document the differences. Tests prove that no write happens during sink
  delivery or update and that terminal information cannot silently
  disappear.
- **Host writes:** keep the delivery of an edit that `write_base` ends as
  `Rejected(Conflict)`, and decide whether hosts get a separate notification
  of authoritative base changes or whether the documented answer is that the
  writer forwards its own writes. Test undo and reset during an active
  gesture, equal-value writes and notification order.
- Decide whether these per-source callbacks suffice or whether the general
  signal observer listed under the previous plan's optional follow-ups is now
  justified. If it is taken up, define ownership, event-phase delivery,
  batching, reentrancy, failure and feedback budgets before implementing it.

#### Design checkpoint (2026-10-19)

Chosen alternative: one new builder for menu settings rows; the existing
`Mailbox` as the documented follow-up path for sinks; the writer forwards its
own host writes; no general signal observer.

API sketch (the only new item):

```rust
impl MenuItem {
    /// Reports each successful user write of a toggle or radio row with the
    /// row's new checked state: the toggle's new value, or `true` for a radio
    /// row that became selected.
    pub fn on_change(
        self,
        handler: impl FnMut(&mut EventContext, bool) -> Result<(), RuntimeError> + 'static,
    ) -> Self;
}
```

- **Menu rows.** The handler runs synchronously after the row's own write
  succeeded, after the menu has requested its closure, with the activating
  list's or menu-bar title's event context; work that must outlive a list
  uses `defer_detached`, as for `Select`. The erased action detects its own
  write rather than inferring one from the mark, and deselected sibling radio
  rows report nothing. Choosing a radio row whose value is already selected writes
  nothing and reports nothing; a refused write (its error) reports nothing;
  application writes to the signal report nothing. A failing handler keeps
  the write, propagates as the action's error and is not replayed. The
  handler belongs to the row description, so a menu's redescription
  replaces or removes it. On rows without a write of their own (`new`,
  `command`, `submenu`, separators) it has no effect; `checked_signal` and
  `selected_signal` remain the pattern for settings the application applies
  itself, such as density through a retained `ThemeManager`, and the
  guide shows it. Permission and enablement are unchanged.
- **Sink follow-ups.** No new API: a sink sends to an application-owned
  [`Mailbox`](../docs/contracts.md), whose handler runs at the start of a
  later flush where writes are allowed. Ownership, capacity and order are the
  mailbox's: bounded, FIFO within the mailbox, `SendError::Full` or `Closed`
  returned to the sink. The recommended shape is a dedicated refresh mailbox
  of capacity one whose token carries no state and whose handler rereads all
  the application state the bindings show, such as the undo history's
  length. The sink first records the outcome in that state, then sends as
  its last step and returns `Ok` whether or not the token fitted: `Full`
  means a refresh is already pending and will read the recorded outcome, so
  pending refreshes collapse (no promise of one callback per flush), and the
  delivery report may then drop the delivered event as it does for any
  successful delivery. A sink re-run by `retry_outcome` (only after a failed
  delivery) sends again harmlessly; earlier side effects of a failing sink
  are the application's to make idempotent.
  Removing the host or clearing its sinks does not close the mailbox;
  runtime shutdown closes it and drops undelivered values. A follow-up's
  error is returned from the flush that delivers it, separately from the
  delivery report. Sending takes a lock and can wake the event loop, so it is
  UI-thread work, not a callback-safe endpoint.
- **Host writes.** No base-change notification: `write_base` is called by
  the application, which forwards its own write (undo, reset) to its engine
  where it calls it. A write during an active edit keeps ending that edit as
  `Rejected(Conflict)`, delivered to the terminal sink. Documented on
  `write_base` with the order of the delivery relative to the write.
- **Observer.** Not taken up. The probe's cases are a user write (menu
  `on_change`), an outcome (sink plus mailbox) and the application's own
  write; none needs to observe arbitrary signals.

Contract tests:

1. Toggle row: a user choice writes and reports the new value once; an
   application write to the signal reports nothing; a radio row reports
   `true` once, and choosing the selected radio row again reports nothing.
2. A handler error keeps the written value, propagates and is not replayed
   on the next choice's behalf.
3. A menu redescription replaces the handler and a description without one
   removes it.
4. `on_change` on a `new` row has no effect and the row's action still runs.
5. A disabled settings row neither writes nor reports.
6. A terminal sink that sends a refresh token updates an undo-depth signal
   by the next flush; a write attempted inside the sink is refused.
7. With the capacity-one refresh mailbox already full, a second commit's
   sink still returns `Ok`, and the one pending refresh shows both commits.
   A preview sink refreshes the same way.
9. A failing refresh handler consumes its token, returns its error from
   that flush and does not fault the parameter or replay the sink; a closed
   mailbox returns `Closed` to the sink, which still returns `Ok`.
10. A settings row reports through pointer and keyboard choice in a nested
    `Menu`, a `ContextMenu` and a direct `MenuBar` title, and work queued
    with `defer_detached` from the handler still runs after the list hides.
8. `write_base` during an active drag delivers `Rejected(Conflict)` with
   the edit's `begin_value` to the terminal sink, in the order documented,
   also for an equal value; `write_base` without an edit calls no sink.

Codex reviewed this checkpoint (2026-10-19): no blocker. Applied: a sink
records the outcome before sending, because a successful delivery may drop
the event from the report; a dedicated capacity-one refresh mailbox, since a
full shared mailbox proves nothing; the activating context includes direct
menu-bar titles; and tests for previews, mailbox failure and closure,
context menus, titles and detached work. It confirmed that `write_base`
publishes the new base and delivers `Rejected(Conflict)` synchronously
before returning, also for an equal value.

### 4. Errors, lookup and command conveniences

Status: implemented in 0.3.13 as the checkpoint below decided. Not done from
its tests: the receipt's captured target is not asserted separately (the
existing `command_context` tests cover it). Tests:
`crates/pert-ui/tests/foundation/callback_conveniences.rs`.

- **Error path:** decide, as an error-policy change or not, how application
  callbacks propagate the library's higher-level errors with `?`. Either keep
  the hierarchy and document a typed adaptation path for command actions,
  deferred tasks and callbacks, or revise the hierarchy deliberately,
  including how existing `WorkspaceError::Runtime` causes normalize. The
  inventory is bounded to the errors of runtime methods an application calls
  from those closures: workspace capture, decode and restore, window
  operations and capture requests. Test source chains and downcasting.
- **Lookup:** choose the lookup the guides show: make the address lookup the
  default, or rename the two so the full-path form is explicitly the
  internal one. Its documentation states which segments built-in containers
  add, including `RegionLayout`'s region segments; an unresolved path stays
  `None`.
- **Capture identity:** type the window in capture requests without putting
  the UI-local `NativeWindowId` handle into the portable request, which
  supports serde and travels through `InspectionClient::capture`. Cover
  retired and foreign identities, serialization and `Region`'s window
  consistently, and point the capture guide at `primary_window`.
- **Command buttons:** `Button::command` mirrors `MenuItem::command`: it
  follows the command's simple enabled binding, does not present contextual
  predicates as reactive state, captures target and focus at activation, and
  gives an observable receipt so queued refusals and failures are not lost.
  Command availability composes with `Button::enabled` through
  `Permission::and`; same-key redescription replaces the description's
  restrictions. Tests cover pointer and keyboard activation, stale commands,
  modal refusal, revocation while armed, callback replacement and failure
  without replay, with no polling and no built-in-only access.
- A shorthand for a chord of one named key without modifiers.

#### Design checkpoint (2026-10-19)

Chosen alternatives and API sketch:

```rust
// Error path: `?` carries a higher-level library error out of a callback.
impl From<WorkspaceError> for RuntimeError { /* Runtime(e) => e, other => Application */ }
impl From<WindowError> for RuntimeError { /* the same */ }
impl From<CaptureError> for RuntimeError { /* the same */ }

// Lookup: the address lookup becomes the default name.
impl Runtime {
    pub fn find(&self, root: &WidgetId, path: &[&str]) -> Result<Option<WidgetId>, RuntimeError>;      // was find_address
    pub fn find_path(&self, root: &WidgetId, path: &[&str]) -> Result<Option<WidgetId>, RuntimeError>; // was find
}

// Capture identity: typed construction, portable payload.
impl CaptureTarget {
    pub fn window(window: &NativeWindowId) -> Self;               // Self::Window(window.id())
    pub fn region(window: &NativeWindowId, area: Rect) -> Self;   // Self::Region { window: window.id(), .. }
}

// Command buttons and chords.
impl Button {
    pub fn command(
        ui: &Runtime,
        command: &CommandId,
        on_receipt: impl FnMut(CommandReceipt, &mut EventContext) -> Result<(), RuntimeError> + 'static,
    ) -> Result<Self, RuntimeError>;
}
impl KeyChord {
    pub fn named(key: KeyCode) -> Self; // no modifiers
}
```

- **Error path.** This is a deliberate exception to the error policy at
  the callback boundary, approved here: `RuntimeError` gets no variant for a
  higher-level error, but `?` converts the errors of the runtime methods an
  application calls from command actions, deferred tasks and component
  callbacks (workspace capture, decode and restore, window operations and
  capture requests) into `RuntimeError::Application`. The envelope is the
  error the called method returned, kept whole as the `ApplicationError`'s
  error and downcastable to its type (`WorkspaceError`, `WindowError`,
  `CaptureError`); the one canonicalization is that an error holding a
  runtime cause in its `Runtime` variant becomes that runtime error itself,
  so a runtime failure keeps one form however it travelled. `DrawError`
  gets no conversion: drawing fails in paint and host paths, which already
  return it, and a conversion would give a drawing failure inside a window
  error two envelopes. Through the facade such a callback failure reaches
  `AppError::Runtime(RuntimeError::Application(..))`, not the variant a
  direct call's error converts to; the docs say so. `RuntimeError`,
  `ApplicationError` and the contract's error policy state the exception.
- **Lookup.** `find` resolves addressable segments, the path inspection and
  `Selector::path` report, and `find_path` keeps the full internal path; the
  old `find_address` name is removed and repository consumers migrate (a
  rename in two steps, so every old `find` caller is visited). `find`'s
  documentation states that built-in containers add addressable segments
  only where they document them, such as `RegionLayout`'s `top`, `bottom`,
  `left`, `right` and `center`, and that an unresolved path stays `None`.
  Paths are relative to `root`: an inspection path starts with the root's
  own key, so `find(&root, &path[1..])` resolves it, and the docs show this.
- **Capture identity.** The variants keep the portable `u64`, the identity
  inspection reports, so serde and `InspectionClient::capture` are
  unchanged and no UI-local handle enters a request. The constructors take a
  `NativeWindowId`, for code on the UI thread; a retired window's identity is
  answered `UnknownTarget`, and identities from another runtime are not
  distinguishable from this runtime's (documented). The capture guide shows
  `CaptureTarget::window(&ui.primary_window())`.
- **Command buttons.** `Button::command` mirrors `MenuItem::command`: the
  command's name is the caption, a simple enabled binding disables the
  button, composing with a later `enabled` through `Permission::and`; a
  contextual predicate leaves it enabled and refuses through the receipt
  (`CommandOutcome::Disabled`). Activation (pointer release or keyboard)
  captures `command_context(CommandSource::Button)` then and queues through
  `EventContext::invoke_command`; `on_receipt` receives each receipt, a full
  queue is the action's error, and failures are not replayed.
  Same-key redescription replaces the action, its receipt handler and the
  description's restrictions. As for `MenuItem::command`, the enabled
  binding is the one the command has when the button is built; a later
  replacement of the command's binding or predicate needs a new description
  of the button (documented). Icon-only command buttons stay out of scope:
  `Button::icon` with `invoke_command` covers them. A stale command fails construction with
  `StaleCommand`; a command unregistered later answers through the receipt.
- **Chord.** `KeyChord::named(key)` is `KeyChord::new(CommandKey::Named(key),
  Modifiers::default())`.

Contract tests:

1. `?` from a command action on a `WorkspaceError` (decode of malformed
   text) yields `RuntimeError::Application` downcastable to it, and through
   the facade `AppError::Runtime`; an error with a source of its own keeps
   it in the chain; `WorkspaceError::Runtime(e)`, `WindowError::Runtime(e)`
   and `CaptureError::Runtime(e)` convert to `e` exactly, and
   `WindowError::Drawing(d)` stays a whole `WindowError`.
2. `find` resolves an inspection path (without its root segment) through
   `RegionLayout`'s `center` segment, refuses an ambiguous address and
   resolves a `Part`; `find_path` needs the internal levels; both return
   `None` for a missing path.
3. `CaptureTarget::window(&ui.primary_window())` equals
   `Window(primary.id())` and `region` its `Region`, both serialize as
   before; a retired window's target is answered `UnknownTarget`; another
   runtime's window with the same number is documented as indistinguishable.
4. `Button::command`: pointer and keyboard activation run the command once
   with the receipt; a disabled binding disables it and composes with
   `enabled(false)`; a modal refusal arrives as a `Blocked` receipt; a
   contextual refusal as `Disabled`; revocation while armed cancels the
   activation; a redescription replaces the receipt handler; a failing
   action, and a failing receipt handler, are reported once and not
   replayed; an unregistered command answers through the receipt and a
   stale one fails construction; a full command queue is the action's
   error; the receipt's context names the target current at activation.
5. `KeyChord::named(KeyCode::Space)` equals the long form and routes.

Codex reviewed this checkpoint (2026-10-19): no blocker. Applied: the
conversion is an explicitly approved callback-boundary exception with a
whole-error envelope; `DrawError` is left out to avoid two envelopes;
lookup paths are relative to the root; the enabled binding is chosen at
construction, as for menus; more boundary tests.

### 5. Parameter mapping and presentation

Status: implemented in 0.3.14. Decisions taken while implementing: one new
shape, `Power(PowerExponent)`, with the exponent bounded to 0.1..=10; within
that range a high exponent still has a domain-dependent plateau at the lower
bound in `f32` (documented and tested after the Codex review); round trips are approximate,
with a tolerance that grows with the exponent near the lower bound (tested);
an exponent of one is a distinct mapping that positions like `Linear`. The
background binding is `Container::background_role_signal`, read with paint
invalidation. `InlineToggle` got the documentation repair, not a text
presentation: its one-letter form is the intended dense-strip presentation.
Tests: `crates/pert-ui-core/tests/parameters.rs`, the core serialization
tests, `crates/pert-ui/tests/presentation/bound_background.rs` and the
power-mapped slider test in `crates/pert-ui/tests/controls/numeric_controls.rs`.

- **Mapping:** a general monotonic mapping beyond linear and logarithmic, as
  a closed set of shapes with validated parameters (for example a power
  curve over any finite domain) rather than unit presets. Mathematical
  monotonicity and invertibility are stated separately from the numerical
  tolerance of `f32` round trips, which stay approximate as today. Specify
  degenerate and extreme finite domains, metadata replacement during an edit,
  `Eq` and the serde representation with rejection of invalid shapes, and
  keep existing `Linear` and `Logarithmic` positions unchanged under
  regression tests.
- **Bound background:** a binding for `Container`'s background role, with
  replacing-builder semantics against literal colors, unsubscription on
  replacement and unmount, foreign-runtime rejection and repaint-only
  updates, so selection can be shown without describing items again.
- **InlineToggle:** choose before coding between a text presentation, with
  its sizing, truncation and redescription behavior, and a documentation
  repair naming the guide and example that tell toolbar authors to give an
  icon or glyph.
- A pause icon in the bundled set, replaceable through `IconManager` like the
  others.

### 6. A packaged live-data example

Status: implemented in 0.3.15 as `crates/pert-ui/examples/live_mixer.rs`, with
the Codex implementation review's fixes in 0.3.16: sinks send the base an
edit leaves (a cancelled audition was left in the engine), a visible Retry
path delivers faulted outcomes and undelivered undo or reset writes, and the
headless session covers those cases and a full queue; a Codex recheck led to
0.3.17, recording a commit in the undo history once regardless of delivery
and covering an undo while the queue is full and a stopped engine. Its
headless session is the deterministic check (each step waits, bounded, for
what it asserts); it compiled from the extracted packaged artifacts and its
window ran on Fedora 44 GNOME/Wayland, recorded in
[verification](../docs/verification.md). Building it confirmed the
prelude finding: it imports `DeliveryMonitor`, `CommandReceipt`, `KeyChord`,
`LevelMeter`, `PowerExponent` and the live-data names explicitly.

After slices 3 and 5: a packaged example application, explicitly an
application-owned simulation, with a producer thread, `series_bridge` and
live widgets, parameters forwarded to the producer through the sinks, an
application undo history and a saved workspace.

- Bounded command delivery to the producer with defined full and closed
  handling, no blocking producer calls from sinks, sink-fault recovery, and
  producer stop and drop on shutdown.
- Undo and reset during an active edit, and workspace decode and restore
  failures reported to the user.
- A deterministic headless run is part of its check, and it compiles against
  the extracted packaged artifacts without workspace-only resources. Opening
  a native window stays `no_run`; native execution is recorded separately.
- It does not claim real-time safety; the time-critical endpoint rules of
  AGENTS.md apply to real producers.

## Relation to the previous plan

Slices 8 (operating system services) and 9 (tutorial and API layout) of
[app-author-improvements](app-author-improvements.md) remain open. The
prelude and discoverability findings above are input to its slice 9 prelude
decision, not a separate slice here: they argue that a smaller prelude still
needs the command and live-data names an ordinary application uses.

## Not in this plan

Audio, unit presets or any other domain semantics in the library; a
library-owned undo history; accessibility integration and X11. The missing
native evidence recorded in [TODO.md](../TODO.md) stays missing; the example
in slice 6 does not substitute for it.

## Review record

2026-10-18, Codex (`codex exec -s read-only`) on the first draft: no blocker.
Confirmed and applied: `write_base` does reach a sink when it ends an active
edit; `Runtime::primary_window` already names the main window;
`RuntimeError` deliberately never wraps `WorkspaceError`, so slice 4 is an
error-policy decision rather than a boxed variant; `ParameterId`, `ChoiceId`
and `ChoiceOption` are in the prelude; only logarithmic mapping requires a
positive domain; density is reachable today through custom menu actions with
a bound mark; the startup claim was narrowed to the probe's observation; and
the slices gained the design checkpoint and the missing contract decisions
and tests. Codex did not run the probe, tests or native checks.
