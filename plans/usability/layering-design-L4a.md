# Part L4a design checkpoint: one deferral call

Part L4a of [the layering plan](layering-plan.md#part-l4-behavior-unifications-0374-and-0375)
(N4 in the [complexity inventory](complexity-inventory.md#needs-a-decision))
changes behavior, so its shape is settled here before any code. Codex
reviews it, then the maintainer approves it. Line references in sections
1 to 3 and 5 are to the code at 971a809, before L1. L1 (0.3.71, 6360b00)
added timing-class lines and moved documentation, so the documentation
locations in [section 4](#4-documentation-and-records) are at 6360b00.

Status: **revision 2, approved 2026-10-27** with the recommended answers to
D1 and D2, by the author together with Codex's recheck (ready for approval),
under the maintainer's delegation of 2026-10-26. Historical passages in
approved checkpoints, including this one, are allowed matches in the final
search below; the implementation records each match it reviewed.

## What users and the library built by hand

- **The trap, as documented.** A callback on a surface that closes before
  the next flush must not use `EventContext::defer`, whose task is cancelled
  when its widget is hidden or removed first. The rule is repeated in eight
  places: `Select::on_change` (`select.rs:1481-1482`, `:1826`),
  `MenuItem::on_change` (`menu.rs:427-431`), `QuickPick`
  (`quick_pick.rs:810-813`), `HistoryNavigator`
  (`history_navigator.rs:39-41`), contracts.md's user-write callbacks
  (`contracts.md:516-517`), the manual (`manual.md:589`), the controls
  chapter (`guide/controls.md:89`) and the custom-widget chapter's dialogs
  (`guide/custom-widgets.md:286-289`).
- **The library's own consumers** use the escape every time a surface
  closes: the `diagram_editor` example's open and save dialogs
  (`examples/diagram_editor/app.rs:600`, `:821`, `:852`, each with a comment
  explaining why) and the gallery's history restore, content jump and
  navigation history (`apps/gallery/src/history.rs:211`,
  `search.rs:110`, `pages/layout/navigation.rs:212`). Three gallery tests
  guard that escape (`apps/gallery/src/tests/shell.rs:708-711`,
  `tests/layout.rs:749-752`, `tests/history.rs:138-141`).
- **Testers:** round 1's task-1 Claude run set a busy flag and then queued
  its save with `defer`; triage found that hiding the button before the
  flush would cancel the save with no error, leaving the flag set forever,
  and that the dropped receipt was the only report
  (`runs/triage/group-B.md:22`). Decision G2-8 kept `TaskReceipt`
  `#[must_use]` for exactly this silent loss
  (`api-audit/decisions.md:701-704`).
- **6f2's convenience chose the escape:** `EventContext::spawn` is
  `defer_detached` plus `Runtime::spawn` (`tasks.rs:282`), because a Save
  button that closes its dialog is the common case
  ([6f2 D2](api-audit/design-6f2.md#1-eventcontextspawn-f13)).

The need is the same everywhere: work an application callback starts should
run, whatever the callback's own surface does next.

## 1. Today's three calls, from the code

All three push onto one runtime-wide queue (`Shared::tasks`,
`runtime.rs:434`) through `enqueue` (`tasks.rs:82-95`). They differ only in
the `Owner` recorded with the task (`tasks.rs:61-69`).

| Property | `EventContext::defer` | `EventContext::defer_detached` | `Runtime::defer` |
|---|---|---|---|
| Owner recorded | `Widget(Weak<Node>)` (`tasks.rs:130-134`) | `Window(Weak<WindowShared>)` of the widget's window **at enqueue** (`:199-203`) | `Application` (`:300`) |
| Cancelled before delivery when | the node is dropped, unmounted or not effectively visible (`:329-335`) | the window is dropped or no longer mounted (`:338-341`) | never, except shutdown |
| Window selected while it runs | the node's window **at delivery** (`:336`, `:347-349`) | the window captured at enqueue (`:342`) | none: the selection the delivering flush has (`:344`) |

- **"Not effectively visible"** includes more than the widget's own
  visibility: `node.visible` is the conjunction of its ancestors' and its
  root's visibility (`tree.rs:1519`), and `Runtime::set_window_visible(false)`
  refreshes every root with `false` (`native_windows.rs:383-397`). A
  `defer` task is therefore also cancelled when its native window is
  minimized before the flush, and when a surface (popup, dialog, list)
  holding the widget closes.
- **"No longer mounted"** for a window means retired: `retire_window` clears
  `mounted` (`native_windows.rs:832-833`). That happens when a detached
  window closes with either policy: `Redock` retires it although the widget
  lives on in its original window, and `RemoveView` unmounts the view and
  retires the window (`close_detached_window`, `native_windows.rs:1297-1313`;
  `redock`, `:770`). The native host processes queued window requests,
  closes and redocks in `about_to_wait` before it flushes any root
  (`crates/pert-ui/src/platform/mod.rs:742-751`, before the per-root
  schedule at `:761-777`; `platform/windows.rs:145-163`, `:199-214`). A
  callback in a detached window that queues its work with `defer_detached`
  and then calls `cx.close_window()` or `cx.redock_window()`
  (`native_windows.rs:1365`, `:1380`) therefore loses the work when the
  close is admitted at once and no redraw flushed in between: the same trap
  as the closing surface, one level up.
- **Routing after detachment.** Detaching moves the widget's `Node::window`
  to the new root (`move_presentation`, `native_windows.rs:742`). A
  `defer` task follows the widget; a `defer_detached` task runs in the
  window it was queued in, even though the widget has left it.
- **Capacity:** `Runtime::MAX_TASKS` (64, `tasks.rs:287`) accepted and
  undelivered tasks across all three calls; the 65th is refused with
  `TaskQueueFull` (`:85-87`). A task whose owner is gone keeps its place
  until a delivery pops and drops it.
- **Admission errors:** `Shared::writable` (`tasks.rs:83`,
  `runtime.rs:709-729`): `Closed` after shutdown and the
  `WriteDuring…` phase errors.
- **Delivery point:** `flush_inner` delivers the queue after parameter
  terminal notifications, focus errors, queued commands and geometry
  changes, and before intervals, timers, background results, mailboxes,
  notifications and component evaluation (`runtime.rs:5701-5714`).
  `flush_windows` selects each visible root and flushes it
  (`native_windows.rs:1556-1563`); since the queue is runtime-wide, the
  first visible root's flush delivers every window's tasks. With every root
  hidden it delivers the queue once itself (`:1566-1572`).
- **Ordering:** one FIFO across the three calls. A delivery takes the length
  at its start and runs that many (`tasks.rs:322-327`); tasks queued during
  delivery wait for the next flush.
- **Nested flushes:** `task_dispatching` makes a flush inside a task deliver
  no tasks (`tasks.rs:303-319`); the outer loop continues in order.
- **Selection after a task:** the selection before the task is restored if
  that window is still mounted (`tasks.rs:346`, `:357-360`).
- **Receipts:** `Pending` until the task is delivered or dropped;
  `Succeeded` or `Failed(error)` from the action (`tasks.rs:361-365`);
  `Cancelled` when the task is dropped while `Pending` (`Queued::drop`,
  `:75-81`), which covers a skipped owner, the shutdown clear and a
  panicking action unwinding.
- **Queued versus executing:** a task is popped before its action runs
  (`tasks.rs:324`), so owner checks and the shutdown clear reach only tasks
  still queued. A task whose action calls `Runtime::shutdown` (directly or
  through a nested operation) is executing: the clear cancels the tasks
  behind it, and its own receipt stays `Pending` until its action returns,
  then records `Succeeded` or `Failed`; a panic records `Cancelled` as the
  popped task unwinds.
- **Failure:** an action's error is stored in the receipt and returned from
  the flush (`tasks.rs:366`); tasks not yet popped stay queued for the next
  flush; nothing is replayed. `InputErrorGuard` scopes error classification
  to the action (`tasks.rs:352`, `contracts.md:739-742`).
- **Shutdown:** `Runtime::shutdown` clears the queue (`runtime.rs:6511`),
  which cancels every pending receipt, early in teardown. Teardown then
  removes every root (`:6547`), and removal calls
  `Component::cancel_input` with an `EventContext` (`tree.rs:1660`), while
  `closed` is set only at the end (`runtime.rs:6577`). `writable` checks
  `closed`, not `shutting_down`, so a task queued from `cancel_input`
  during shutdown is accepted and never delivered or cancelled: its
  receipt stays `Pending`
  for good, against `TaskStatus::Cancelled`'s contract (`tasks.rs:11`), and
  the queue entry keeps the shared state alive through the receipt's
  binding (`signal.rs:170`, `:895`). `cancel_input`'s rustdoc discourages
  application actions there (`runtime.rs:2503-2504`), but nothing refuses
  them. No library component defers from `cancel_input` today.

### Every caller

| Caller | Call | Why it uses it |
|---|---|---|
| `examples/diagram_editor/app.rs:600`, `:821`, `:852` | `defer_detached` | open and save from a dialog that `accept()` closes in the same callback |
| `apps/gallery/src/history.rs:211`, `search.rs:110`, `pages/layout/navigation.rs:212` | `defer_detached` | restore or jump from a list or picker that closes on choice |
| `tasks.rs:172` (doctest), `tasks.rs:282` (`EventContext::spawn`) | `defer_detached` | the QuickPick example; spawn's window-owned start (6f2 D2) |
| facade tests `foundation/foundation.rs:329`, `:343`; `surfaces/multi_window.rs:157`, `:169`; `controls/component_callbacks.rs:125`; `surfaces/menu_changes.rs:229` | `defer_detached` | the window owner's contract; closing Select and menu callbacks |
| `examples/dynamic_documents.rs:26` | `defer` | a button opens a document (parameter plus reconcile) |
| `apps/gallery/src/pages/data.rs:643` | `defer` | a button queues `Runtime::spawn` (predates `EventContext::spawn`) |
| `apps/gallery/src/pages/controls.rs:1148`; `pages/tools.rs:147`, `:183`; `pages/layout/navigation.rs:367` | `defer` | buttons that start an interval, capture or restore the workspace, open a palette |
| `apps/gallery/src/pages/layout.rs:562`; `tabs.rs:533` (doctest) | `defer` | a tab strip's reorder callback describes the strip's tabs again |
| `guide/layout.md:608` | `defer` | the list recipe's `on_activate` begins editing a row's title |
| `apps/gallery/src/workspace_check.rs:58`, `interaction_check.rs:1008` | `defer` | gallery check components step themselves from `deliver_notifications` |
| facade tests `foundation/foundation.rs:206`, `:291`, `:327`; `foundation/component_module.rs:73`; `audit/audit_conformance.rs:191`; `audit/audit_notification_failures.rs:401`, `:959` | `defer` | the widget owner's contract, the component module, notification and error-classification audits |
| `examples/dynamic_documents.rs:97`; facade tests `foundation/foundation.rs:238`, `:253-264`, `:301-302`; `foundation/receipts.rs:40-48`; `foundation/phase_rules.rs:45`, `:77-79`, `:110`, `:117`, `:164`; `foundation/failure_sites.rs:565`; `audit/sweep_coverage.rs:191`; `surfaces/multi_window.rs:825`; gallery tests `tests/shell.rs:711`, `tests/layout.rs:752`, `tests/history.rs:141` | `Runtime::defer` | code holding the runtime; the gallery tests deliver Enter from inside a flush, as a native check's driver does |

`EventContext::spawn` is called from `tasks.rs:252` (doctest),
`guide/state.md:329` and `foundation/application_services.rs:359`, `:403`,
`:422`, `:433`, `:445`, `:463`. No diagnostics fixture, native check or
`plans/usability/task4-app` source defers (searched with `rg`).

## 2. The design

Cancellation and routing are two separate rules, stated per call.

### Public API

```rust
impl EventContext {
    /// Queues an application operation with mutable runtime access for the
    /// next flush. Only runtime shutdown cancels it before delivery.
    pub fn defer(
        &self,
        action: impl FnOnce(&mut Runtime) -> Result<(), RuntimeError> + 'static,
    ) -> Result<TaskReceipt, RuntimeError>;

    /// Queues an operation on this component's behalf, cancelled when the
    /// component is hidden or removed before the next flush.
    pub fn defer_while_shown(
        &self,
        action: impl FnOnce(&mut Runtime) -> Result<(), RuntimeError> + 'static,
    ) -> Result<TaskReceipt, RuntimeError>;

    /// Unchanged signature; the start is the operation
    /// `|ui| ui.spawn(work, deliver).map(|_| ())` queued with `defer`.
    pub fn spawn<T: Send + 'static>(/* unchanged */) -> Result<TaskReceipt, RuntimeError>;
}

impl Runtime {
    /// Unchanged.
    pub fn defer(/* unchanged */) -> Result<TaskReceipt, RuntimeError>;
}
```

- **Removed:** `EventContext::defer_detached`. Its replacement is
  `EventContext::defer` (decision [D1](#decisions-for-the-maintainer)).
- **No new type** and no new import: both forms are methods of
  `EventContext`, which a callback receives. L2's guarantee (prelude-only
  application code, `prelude` and `component` for a custom component) is
  unaffected.
- `TaskStatus`, `TaskReceipt`, `Runtime::MAX_TASKS` and
  `RuntimeError::TaskQueueFull` are unchanged; `TaskStatus::Cancelled`'s
  rustdoc is reworded (below).

### Cancellation owner (decision D2)

| Call | Cancelled before delivery when |
|---|---|
| `EventContext::defer`, and so `EventContext::spawn`'s start | the runtime shuts down |
| `EventContext::defer_while_shown` | the widget is unmounted or not effectively visible at delivery (its own or an ancestor's visibility, a closed surface, a hidden native window), or the runtime shuts down |
| `Runtime::defer` | the runtime shuts down |

**Queued and executing tasks.** These rules decide only whether a queued
task is delivered. Once delivered, a task is executing and nothing cancels
it: if its action calls `Runtime::shutdown`, the shutdown cancels the tasks
still queued and refuses new ones, and the executing task records its own
returned result, `Succeeded` or `Failed`, when the action returns. If the
action panics, its receipt reads `Cancelled` as the task unwinds
(`Queued::drop`, `tasks.rs:75-81`); the tasks behind it stay queued and
`Pending` until the next delivery or shutdown. Both are today's behavior.

`defer` is thereby the same lifetime on both receivers: an application
operation that only shutdown cancels before delivery.
`EventContext::defer` differs from `Runtime::defer` only in routing,
because a callback has an origin. `defer_while_shown` is today's `defer`
unchanged, under a name that states its cancellation rule; it serves work that concerns the component itself,
like the component's own timer (`EventContext::set_timer`), which hiding
and removal also stop (`runtime.rs:3478-3480`).

### Execution window (routing)

At delivery, before the action runs:

1. **`defer` and `defer_while_shown`:** if the widget that queued the task
   is still mounted, its window at delivery is selected, wherever detaching
   or redocking moved it since.
2. **`defer` only,** when that widget is gone: the window it was in when
   the task was queued (the window `EventContext::native_window` named at
   the call), if that window is still open.
3. Otherwise, and always for `Runtime::defer`: the selection is not changed;
   the task runs with whichever window the delivering flush has selected.

For `defer_while_shown`, step 1 always applies, since a task whose widget
is gone is cancelled. After the action, the earlier selection is restored if
that window is still open, as today. Routing never cancels and never fails:
a window that is not open is skipped, not selected. Work that must address
the window the event came from, wherever the widget is later, captures
`cx.native_window()` and selects it in the action.

### Kept unchanged

Capacity (one `MAX_TASKS` count across all three calls, refusal with
`TaskQueueFull`, a cancelled task's place freed at the next delivery), the
delivery point in the flush, one FIFO across the three calls, the
nested-flush rule, the receipt states and when they are set, failure
propagation (stored, returned from the flush, the rest delivered by the next
flush, never replayed), the error-classification boundary, and
`flush_windows`' single delivery with every root hidden. All three remain
UI-thread work and never a callback-safe endpoint.

### Shutdown (a fix within the part)

`enqueue` refuses with `RuntimeError::Closed` once shutdown has begun
(`Shared::shutting_down`, `runtime.rs:464`, set at `:6506`), not only after
it ended. Every task queued before that point is cancelled by the existing
clear, so after `Runtime::shutdown` returns no queued task's receipt reads
`Pending`. A task executing when shutdown is called, because its action
called it, records its own result when the action returns, as above. The
check is in `enqueue` only: `writable` stays as it is, because teardown
callbacks still restore their own bindings. The `# Errors` sections already
name "the runtime phase/shutdown error"; they gain "including from cleanup
callbacks that run during `Runtime::shutdown`".

### Implementation outline

Private to `tasks.rs`:

```rust
enum Owner {
    /// `EventContext::defer`: runs unless shut down; routed by the widget,
    /// then by its window at enqueue.
    Event { widget: Weak<Node>, origin: Weak<native_windows::WindowShared> },
    /// `EventContext::defer_while_shown`: today's `Widget` owner.
    Shown(Weak<Node>),
    /// `Runtime::defer`.
    Application,
}
```

`EventContext::spawn` calls `self.defer(...)`. The dispatch loop computes
the window by the rules above; the `Shown` arm is today's `Widget` arm. The
comment at `native_windows.rs:1567-1569` names `defer_while_shown`.

### Rustdoc

- `EventContext::defer`: the default for application callbacks, including
  those on surfaces that close (a Select, a menu row, a QuickPick choice, a
  dialog's Save button) and in detached windows that close or redock;
  cancellation by shutdown only; the routing rule; that work which needs
  its widget must handle `Unmounted` or use `defer_while_shown`. It takes
  over `defer_detached`'s QuickPick doctest (`tasks.rs:151-190`), with
  `cx.defer`. The existing paragraphs on receipts, nested flushes and
  errors stay.
- `EventContext::defer_while_shown`: for a custom component's work on
  itself, such as describing its own children again from an input hook; the
  cancellation list; routing to the widget's window at delivery. A doctest
  shows a component whose task is cancelled when it is hidden before the
  flush. It names its audience as the custom-widget layer.
- `EventContext::spawn`: "The start belongs to the application, as
  `defer`'s operation does: it runs at the next flush whatever happens to
  the widget or its window, and only shutdown cancels it before it starts."
  The receipt list
  says `Cancelled`: the runtime shut down before the start. The cancellable
  form stays `Runtime::spawn` queued with `defer`.
- `Runtime::defer`: "The selection is not changed: the operation runs with
  whichever window the delivering flush has selected; select one explicitly
  for window-specific work."
- `TaskStatus::Cancelled`: "The operation did not complete: the runtime
  shut down before delivery; for an operation queued with
  `EventContext::defer_while_shown`, its component was hidden or removed
  first; or its callback panicked and the operation unwound." The rustdoc
  of `defer` and `Runtime::defer` states the queued-versus-executing rule.
- The eight closing-surface warnings (the first bullet of [what users
  built by hand](#what-users-and-the-library-built-by-hand)) become a
  plain statement that work queued with `EventContext::defer` runs after the
  surface closes, or are removed where the callback's own rustdoc already
  says the surface closes first.

## 3. Callers whose behavior changes

| Callers | After | Changes only when, between enqueue and delivery |
|---|---|---|
| Every `defer_detached` call (section 1), migrated to `defer` | runs | the queued-in window closes or is redocked away: today `Cancelled`, now it runs (routed to the widget's window if mounted, else unchanged selection); the widget moves to another window: today it runs in the old window, now in the widget's |
| Every `EventContext::spawn` call | starts | its window closes first: today `Cancelled`, now the thread starts |
| `cx.defer` calls migrated to `defer`: `dynamic_documents.rs:26`, gallery `controls.rs:1148`, `tools.rs:147`, `:183`, `layout.rs:562`, `navigation.rs:367`, `tabs.rs:533`, `guide/layout.md:608`, facade `foundation.rs:206` | runs | the widget is hidden, removed, or its window is hidden or closed: today `Cancelled`, now it runs, and an action that addresses the removed widget returns its error (such as `Unmounted`) from the flush |
| `cx.defer` call at gallery `data.rs:643`, migrated to `cx.spawn` | starts | as the row above; the code is exactly `EventContext::spawn`'s operation |
| `cx.defer` calls migrated to `defer_while_shown`: gallery `workspace_check.rs:58`, `interaction_check.rs:1008`; facade `foundation.rs:291`, `:327`, `component_module.rs:73`, `audit_conformance.rs:191`, `audit_notification_failures.rs:401`, `:959` | unchanged | never: component-scoped work and tests of the widget owner or of error classification keep today's semantics exactly |
| `Runtime::defer` callers | unchanged | never |
| Cleanup callbacks that queue during `Runtime::shutdown` | refused with `Closed` | always; today accepted and left `Pending`. No maintained caller does this |

The rule for choosing in the migration: a callback given to a control or
container, which is application code, uses `defer`; a `Component` hook
whose task acts on the component itself uses `defer_while_shown`; a test
keeps the semantics it asserts. Among the formerly widget-owned
application callbacks migrated to `defer` (the third row), none hides or
removes its own widget in the same event: the buttons stay mounted and the
tab strip and list outlive their callbacks, so their observable behavior
changes only if something else hides or removes the widget before the
flush. The former `defer_detached` callers do close their surfaces in the
same event (the `diagram_editor` dialogs `accept()` before queuing,
`app.rs:597-600`, `:850-852`); they already ran after that closure, and
change only in the window cases of the first row.

## 4. Documentation and records

Locations are at 6360b00 (after L1).

- **contracts.md:**
  - "In the input" (`contracts.md:540-548`), added by L1: the sentence that
    work outliving a closing `Select` list, menu row or picker "is queued
    with `defer_detached`" becomes "is queued with `EventContext::defer`,
    which runs after the surface has closed".
  - "At the next flush" (`:562-570`) lists `EventContext::defer`,
    `EventContext::defer_while_shown` and `Runtime::defer`; its cancellation
    property (`:626-628`, "the widget or window a deferred task is bound
    to") becomes "the component a `defer_while_shown` task is bound to".
  - The user-write callbacks sentence (`:486-487`) and the background-work
    sentence on `EventContext::spawn` (`:828-832`) follow section 2.
  - "Calling the runtime from callbacks": the deferred-task row (`:1083`)
    lists the three calls, and the ownership paragraph (`:1099-1103`)
    becomes section 2's cancellation and routing rules, including queued
    versus executing tasks.
  - Implementation ordering (the runtime-wide queue delivered by the first
    visible root's flush) goes to `docs/internals.md` beside L1's
    delivery-order material, if L1 placed that there.
- **Guide:**
  - The state chapter's "Work deferred from a callback" table
    (`guide/state.md:32-37`) becomes `EventContext::defer` (the way),
    `Runtime::defer` (code holding the runtime) and `defer_while_shown`
    (advanced, for a component's work on itself); its closing-surface row
    goes. The spawn prose (`:345-352`) follows section 2.
  - `guide/custom-widgets.md:111` names both forms; the dialog paragraph
    (`:342-345`) says that work a closing dialog starts is queued with
    `EventContext::defer`.
  - `guide/controls.md:89` and `docs/manual.md:589` drop the
    `defer_detached` advice.
  - `guide/getting-started-continued.md:109`, `guide/state.md:27`, `:395`
    and `guide/layout.md:626`, `:666` keep `defer`, which now means the new
    rule.
- **Runtime rustdoc:** `history_navigator.rs:39-41`, `menu.rs:433-437`,
  `quick_pick.rs:813-817`, `select.rs:1481-1482` and `:1826-1828` (section
  2's rustdoc bullets); `tabs.rs:508-535` keeps `defer`; `tasks.rs` as in
  section 2, keeping L1's timing-class lines.
- **Comments and module rustdoc** naming the old reason: the gallery search
  module's opening rustdoc (`apps/gallery/src/search.rs:7-11`, which links
  `EventContext::defer_detached`) and its comment at `:105-107`;
  `history.rs:200-202`; the `diagram_editor` crate rustdoc (`main.rs:27`)
  and `app.rs:598-599`.
- **Final search.** Before the implementation is reviewed, a search over
  the maintained sources and documentation (`crates/`, `apps/`, `docs/`,
  `tests/`, `README.md`, `AGENTS.md`, `plans/usability/task4-app/`, and the
  open plans) finds no obsolete use or recommendation of
  `defer_detached`, "detached deferral" or obsolete cancellation wording ("bound to its widget", "bound to the window",
  "outlive the widget", "skipped when the widget is hidden", "cancelled
  when the window closes" said of `defer` or `spawn`), with the frozen
  records below excluded. The search command and its empty result go into
  the implementation's review record.
- **6f2's D2** (the window owner for `EventContext::spawn`'s start) is
  superseded by D2 here if approved. `design-6f2.md` and `decisions.md` are
  closed records and are not edited; the CHANGELOG entry names the
  supersession.
- **Not migrated (frozen):** the round 1 submissions, probes and triage
  under `plans/usability/runs/` and `~/pert-ui-usability/`, `plans/archive/`,
  the api-audit records and reports, the complexity inventory (evidence,
  measured at fixed commits), earlier CHANGELOG entries.
- **TODO.md:** no entry touches `tasks.rs`, deferral, receipts or `spawn`.

## 5. Tests

The tests live in the facade's area binaries: a new `deferral` module in
the foundation area for the single-window rules, and the surfaces area for
surfaces and native windows. They are of two kinds, marked in the code:

- **New-behavior tests** cover what only the new rules do. Each names the
  assertion that detects a reverted implementation and is checked with a
  compiling behavioral mutation in `tasks.rs` that restores the old rule;
  the named assertion must fail. A test that fails only because
  `defer_detached` no longer compiles is not evidence.
- **Contract and compatibility tests** check rules this part keeps. Where
  one guards a rule the rewrite could break on its own, it is checked with
  a compiling mutation that violates that rule.

### New-behavior tests

1. **Surface close, with closure settled before delivery.** Choosing an
   item and delivering the queued task in one flush does not prove
   anything: tasks run before component evaluation (`runtime.rs:5709`), and
   some surfaces settle their closure only during update (a `QuickPick`
   answering `Done` writes its phase, `quick_pick.rs:595` at 6360b00, and
   changes its children's visibility during update, `:1101`), so the old
   widget-owned task could still run. Each case therefore forces the
   problematic order, as the gallery's closing-row tests do
   (`apps/gallery/src/tests/shell.rs:708-711`):
   - flush 1 delivers a `Runtime::defer` task that sends the choosing input
     (`key_input` Enter, or a pointer click on a dialog's button); the
     callback's `cx.defer` is queued during delivery, so it waits;
   - the rest of flush 1 settles the closure; a precondition assertion
     checks that the callback's widget is hidden or unmounted;
   - flush 2 delivers the queued operation.

   Cases: `Select::on_change`, a `QuickPick` choice answering `Done`, a
   `MenuItem::on_change` row, and a modal `Dialog`'s Save button that
   closes its dialog. Named assertion, per case: after flush 2 the receipt
   reads `Succeeded` and the operation's effect is applied. Mutation:
   `EventContext::defer` records `Owner::Shown`; each case's named
   assertion must fail under it (the receipt reads `Cancelled`). A case for
   which it does not fail is not counted as evidence: it moves to the
   compatibility tests with the reason recorded.
2. **Hidden widget, hidden window.** `defer` from a button that
   `Runtime::set_visible(false)` hides before the flush, and from a widget
   whose root `set_window_visible(false)` hides, delivered by
   `flush_windows` with every root hidden. Hiding is synchronous
   (`tree.rs:1519`), so the order needs no forcing. Named assertion:
   `Succeeded`. Mutation: as 1.
3. **Window close.** A widget in a detached window queues `defer`, then the
   window closes before the flush:
   - with `DetachedClosePolicy::Redock`: named assertions `Succeeded` and
     `ran_in == ui.primary_window()` (the widget's window now);
   - with `RemoveView`: named assertions `Succeeded` and `ran_in` equal to
     the window the flush had selected.

   Mutation: `defer` is cancelled when its origin window is not mounted
   (today's `defer_detached` rule).
4. **Origin-window fallback.** A detached view in window B holds a child
   button. The child queues `defer`; the child is unmounted
   (`Runtime::unmount`, or the view described again without it) while B
   stays open; the primary
   window A is selected and `ui.flush` runs. Two variants: the test keeps
   the child's `WidgetId`, so the node is retained but unmounted
   (`WidgetId` holds the node, `runtime.rs:121`), and the test drops it, so
   the task's `Weak` no longer upgrades. Named assertions: `ran_in == B`
   inside the action, and `ui.current_window() == A` after the flush.
   Mutation: omit routing step 2, so the task runs with A selected.
5. **Detached, then hidden.** A child of a view in the primary window
   queues `defer`; `prepare_detach` and `commit_detach` move the view to
   window B; `set_visible(child, false)` hides the child; `flush_windows`
   runs. Named assertions: `Succeeded` and `ran_in == B`. This separates
   the new rule from both old forms: today's `defer` would cancel (hidden),
   today's `defer_detached` would run in the primary window. Mutations:
   `Owner::Shown` (fails on `Succeeded`) and routing to the origin window
   (fails on `ran_in`).
6. **Spawn after window close.** `cx.spawn` from a widget in a detached
   window removed with its window before the flush. Named assertions: the
   receipt reads `Succeeded` and the result is delivered. Mutation:
   `EventContext::spawn` records the old window owner. It replaces the
   window clause of `a_callback_spawn_outlives_its_widget_but_not_its_window`
   (`foundation/application_services.rs:417-457`), renamed for the new
   contract.
7. **Queuing during shutdown.** A component whose `cancel_input` calls
   `cx.defer`, `cx.defer_while_shown` and `cx.spawn` stores the three
   results; `ui.shutdown()` removes it. Named assertion: each result is
   `Err(RuntimeError::Closed)`. Mutation: drop the `shutting_down` check,
   so each is `Ok` with a receipt left `Pending`.

### Contract and compatibility tests

- **`defer_while_shown` is today's `defer`:** cancelled after unmount,
  after `set_visible(false)` and after `set_window_visible(false)`; after
  a detach between enqueue and delivery it runs in the new window; after a
  `Redock` close it runs in the primary window; after a `RemoveView` close
  it is `Cancelled`. These are today's tests at `foundation.rs:273-318`,
  migrated, plus the window cases. Contract mutation: ignore visibility in
  the `Shown` arm; the `set_visible` case must fail.
- **Routing after detachment:** a widget that stays shown queues `defer`
  and is detached before `flush_windows`; it runs in the new window
  (`ran_in == detached`). Today's `defer` already routes this way
  (`tasks.rs:336`, `native_windows.rs:742`), so this is a compatibility
  test; its contract mutation routes `defer` to the origin window while it
  is open, and `ran_in` must fail.
- **Ordering:** interleaved `defer`, `defer_while_shown` and
  `Runtime::defer` run in enqueue order, with a cancelled
  `defer_while_shown` task between them skipped without reordering.
- **Capacity:** the 65th task is refused with `TaskQueueFull` whichever
  call queues it; a cancelled task's place is free after the next delivery.
- **Nested flush:** a `defer` task that calls `ui.flush`; tasks of each form
  queued before delivery run after it, in order; tasks queued inside it wait
  for the next flush (`phase_rules.rs:65-127` extended to the three forms).
- **Failure:** a failing `defer` task: receipt `Failed`, the flush returns
  the error, a later-queued `defer_while_shown` task runs at the next flush,
  and nothing is replayed (`foundation.rs:250-270` extended).
- **Shutdown:** pending tasks of every form read `Cancelled` after
  `Runtime::shutdown`, and each call refuses with `Closed` afterwards
  (`foundation.rs:250-270`, `:320-349`, `application_services.rs:459-471`).
- **Shutdown inside a task:** three tasks are queued, one of each form; the
  first one's action calls `ui.shutdown()` and returns `Ok`. After the
  flush returns, whatever its result: the first receipt reads `Succeeded`
  (the executing task records its own result), the two behind it read
  `Cancelled`, none reads `Pending`, and a `Runtime::defer` queued from
  inside the action after the shutdown call is refused with `Closed`. A
  variant whose action returns an error after shutting down records
  `Failed`. Contract
  mutation: cancel the executing task's receipt on shutdown; the
  `Succeeded` assertion must fail.
- **Panic:** a task whose action panics leaves its receipt `Cancelled`
  after the unwind is caught, and the task queued behind it stays `Pending`
  until the next flush delivers it (today's behavior, now documented).
- **Routing compatibility:** `Runtime::defer` does not change the selection,
  and every task restores the earlier selection while that window is open
  (`multi_window.rs:146-177` migrated to `defer`, minus its cancellation
  clause, which test 3 replaces; `multi_window.rs:818-831` unchanged).
- **The gallery's closing-row tests** (`tests/shell.rs:708`,
  `tests/layout.rs:749`, `tests/history.rs:138`) stay and pass with `defer`.
- **Doctests:** `defer` (the QuickPick example), `defer_while_shown`,
  `spawn`, the `Tabs` reorder example (`tabs.rs:506-533`) and the guide's
  list recipe.

## 6. Scope, versions and evidence

- **`api/` diff:** `pert_ui_runtime::EventContext::defer_detached` removed
  and `pert_ui_runtime::EventContext::defer_while_shown` added, at each
  place the listings record `EventContext`'s methods in
  `api/pert-ui-runtime/default.txt` and `serde.txt`; nothing else. The
  facade's listings do not record these methods.
- **Breaking:** yes.
- **CHANGELOG** for 0.3.74, citing this checkpoint:
  - `Breaking:` `EventContext::defer_detached` is removed; use
    `EventContext::defer`, which now has its "run after the surface closes"
    property without its window cancellation.
  - `Breaking:` `EventContext::defer` is no longer cancelled when its
    widget is hidden or removed, or when its window closes; only shutdown
    cancels it before delivery, and it runs in its widget's current window, else the window
    it was queued in while open. For the old behavior use
    `EventContext::defer_while_shown`.
  - `Breaking:` `EventContext::spawn`'s start is no longer cancelled when
    its window closes (superseding part 6f2's window owner); to tie a start
    to a component use `cx.defer_while_shown(|ui| ui.spawn(work,
    deliver).map(|_| ()))`.
  - `Added:` `EventContext::defer_while_shown`.
  - `Fixed:` a task queued by a cleanup callback during `Runtime::shutdown`
    is refused with `Closed`; it was accepted and its receipt stayed
    `Pending`.
  - `Documentation:` the closing-surface warnings, contracts.md, the
    guide's deferral table and the migrated examples.
- **Version:** 0.3.74 (workspace version, internal requirements and
  `Cargo.lock`).
- **Gate:** the public-API row, as the plan states for L4: the workspace
  checks (formatting, Clippy, tests, the packaged examples' headless
  sessions, which include `dynamic_documents` and `diagram_editor`,
  doctests, warning-free rustdoc, the diagnostics snapshots), the API
  listing gate, the focused tests above with their mutations recorded, and
  Codex's implementation review. The gallery's sources change, so the
  headless gallery (`cargo run --locked -p pert-ui-gallery -- --headless`)
  also runs. The native checks that drive `interaction_check` and
  `workspace_check` keep their semantics (`defer_while_shown`); if no
  desktop session runs them, the record says so.

## Corrections to the plan and the inventory

- The inventory said the three calls "differ only in what cancels the task"
  (`complexity-inventory.md:251`). They also differ in routing, as the plan
  already corrected; and `Runtime::defer`'s "selected window" is the
  delivering flush's selection, which in `flush_windows` is the first
  visible root, since one root's flush delivers every window's tasks.
- The plan and inventory frame the trap as closing **surfaces**. The same
  trap exists for closing or redocking **detached windows**, because the
  native host applies window requests before it flushes; that is what
  decides D2 against the inventory's "window-bound by default"
  (`complexity-inventory.md:476`).
- `defer`'s widget owner is also cancelled by a hidden native window, not
  only a hidden or removed widget.
- The shutdown path can leave a receipt `Pending` (section 1), which the
  plan's "keep receipts" did not anticipate; section 2 fixes it.
- Citation drift since the inventory: the Select warning is now
  `contracts.md:516-517` (cited as `:494-496`) and the dialog rule
  `guide/custom-widgets.md:286-289` (cited as `:284-289`).

## Decisions for the maintainer

- **D1: the shape.**
  - **Recommended:** `EventContext::defer` is the one call for application
    callbacks, `defer_while_shown` the named form for a component's work on
    itself, `defer_detached` is removed, `Runtime::defer` stays. No new
    type, so L2's import guarantees hold without a prelude change. The name
    states the cancellation rule; `defer_for_component` would name the
    audience instead.
  - **Alternative A:** one call with a lifetime argument,
    `cx.defer(TaskLifetime::Application, action)`, with `Widget` as the
    other value. Every call states its lifetime, but every application
    deferral writes a second name, and the new enum must join the prelude
    because the representative programs would write it.
  - **Alternative B:** no widget form. A component's own work handles
    `Unmounted` itself. The smallest API, but a component that describes its
    own children from an input hook turns its removal into a flush error,
    unlike its timers, which hiding and removal end.
- **D2: what cancels `EventContext::defer` (and `spawn`'s start).**
  - **Recommended:** only shutdown, with routing to the widget's current
    window, else its window at enqueue while open. A save started in a
    closing dialog, a closing picker or a detached window that the same
    callback closes or redocks always runs, and an action whose targets are
    gone reports their error from the flush instead of disappearing. It
    makes `defer` one lifetime on both receivers and supersedes 6f2's D2.
  - **Alternative:** the window, as `defer_detached` today (the inventory's
    suggestion and 6f2's choice for `spawn`). Its benefit is deliberate:
    an operation meant for a window that has ceased to exist is not run, so
    it cannot act on that window's removed widgets, and the cancellation is
    observable as `Cancelled` in the receipt. It is silent only when the
    application drops the receipt, which `#[must_use]` warns against but
    the library's own callers do (`let _receipt`, `let _`). The
    recommendation weighs the other way for the default: an application
    operation such as a save or an open is reliable only if it runs, and the
    detached-window close and redock paths cancel exactly those operations
    before the flush. With the recommendation, an operation whose targets
    are gone fails visibly, through its receipt and the flush, instead of
    being skipped; work that should be skipped with its component uses
    `defer_while_shown`.

## Review record

- **Revision 1, Codex review (2026-10-26, against 6360b00):** needs
  revision, with five should-fix points and two nits; no boundary,
  real-time, persistence or L2 import conflict, and the API direction was
  found workable. Revision 2 addresses each point:
  1. **Receipts at shutdown:** queued and executing tasks are
     distinguished; "only shutdown" is qualified as cancellation before
     delivery; `Cancelled` on unwinding is documented in the rustdoc
     wording; a compatibility test covers shutdown inside a task, and
     another the panic outcome (sections 1, 2 and 5).
  2. **Surface tests:** each forces closure to settle before delivery (the
     input is delivered inside a running deferred task, closure settles in
     that flush, the operation is delivered in the next), with a
     precondition assertion, and each named assertion must fail under the
     mutation or the case leaves the evidence (test 1).
  3. **Origin-window fallback:** a direct test, with a retained unmounted
     `WidgetId` and a dropped node, and a mutation omitting the fallback
     (test 4).
  4. **Detachment:** the simple case is a routing compatibility test; the
     new-behavior test hides the widget after detachment and asserts
     `Succeeded` and the destination window, failing under both old forms
     (test 5).
  5. **Migration:** L1's "In the input" recommendation
     (`contracts.md:540-548`), its "at the next flush" and cancellation
     wording, the gallery search module's rustdoc (`search.rs:7-11`) and the
     other locations at 6360b00 are listed, with a final search for removed
     names and obsolete cancellation wording (section 4).
  6. **Nit, migration summary:** the "same event" statement is restricted to
     the formerly widget-owned callbacks; the `defer_detached` callers'
     closing dialogs are named (section 3).
  7. **Nit, D2's alternative:** its deliberate benefit and the receipt's
     observable cancellation are stated, with the reason the recommendation
     outweighs them.
- **Revision 2, Codex recheck (2026-10-27):** ready for approval. All seven
  points resolved; one nit, applied: the final search allows historical
  passages in approved checkpoints and records the reviewed matches instead
  of demanding an empty result.
