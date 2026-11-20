# File dialog: design note for the own window (slice 3)

Status: parked. The maintainer deferred the own window on 2026-11-20 (plan
decision 13), before deciding the questions in section 9; nothing here is
implemented, and the in-window dialog is what ships. This is the second
draft, kept as the starting point if the work is taken up. It follows the maintainer's direction of 2026-11-20: opening
in a window of its own is a property a developer sets on a view, and the file
dialog is one user of it. An independent Codex review of the first draft
(2026-11-20, source review only, nothing run) found five blockers; this
draft answers them, and [section 11](#11-review-record) lists what changed.
Every type and method named is a proposal.

## 1. Summary of the finding

The first draft claimed that the existing detach protocol makes the own
window nearly free. The review shows that this holds for two things and not
for the rest:

- **Holds: creating and destroying the native window.** `App` serves a
  detach for both presenters, and it already drops the native resources of a
  window whose runtime identity has retired, without being asked
  (`crates/pert-ui/src/platform/windows.rs`, `retire_windows`).
- **Holds: the blocking rule.** Once an application-modal root is active,
  every component outside its subtree is blocked in every window: hit
  testing, focus, targetless commands and command queries
  (`runtime/surfaces.rs:109`, `:131`, `:180`; `runtime/commands.rs:1181`).
- **Does not hold: that a detached view stays modal.** A surface's kind is
  declared by its presentation parent (`Dialog::child_surface`,
  `dialog.rs:431`). Detaching sets the view's presentation parent to none
  (`runtime/native_windows.rs:291`, `:636`), so the modal root stops being
  one the moment it moves.
- **Does not hold: that showing and hiding map onto detach and redock.**
  Detach refuses hidden views and modal subtrees, a queued detach names only
  the view and not which showing of it, hiding does not reach a detached
  child (`runtime/tree.rs:1583`), and redock selects, focuses and notifies as
  for a view the user is bringing back (`runtime/native_windows.rs:689`).

The property is still the right public shape. Underneath it the runtime needs
a presentation of its own, which reuses the host's window creation and the
blocking rule and adds what sections 3 to 6 describe. The work is comparable
to the plan's original slice 3a, not smaller; what the generalization buys is
one mechanism for views and dialogs instead of a dialog-only one.

## 2. The property

```rust
// API sketch, not an available function.
Element::new("inspector", inspector).own_window(NativeWindowOptions::new("Inspector")?)
```

Whenever the element is shown it is presented in a native window of its own
instead of in its parent's layout; while it is hidden or unmounted it has no
window. Its lifetime owner, key path, state and workspace participation are
unchanged. A later description replaces the options; whether a change applies
to an open window or to the next showing is stated per option in the slice
(title: live, through the existing retitle request; size and close policy:
next showing).

Rationale for a new way to do something possible (AGENTS.md): detaching by a
call stays for moves the user starts. The property is the declarative form
for a view that has no docked presentation, and it retires "detach it once it
is mounted" code. It does not consult `detach_permission`, which governs what
a user may drag out; instead the runtime's own request carries an origin the
runtime verifies, so the property never authorizes an ordinary caller's
detach of the same view.

## 3. A showing has an identity

Each time the element becomes visible is one *showing*, with a generation
the runtime issues. Everything that happens for a window is checked against
it, so nothing from an earlier showing can act on a later one:

- **The request.** A new presentation action carries an opaque ticket for
  the showing, not just the view: `PresentationAction::Open(ticket)`. The
  existing `Detach { view, options }` cannot tell two showings of one mounted
  panel apart, so session A's queued request could otherwise prepare session
  B's window. The host prepares and commits with the ticket. This is a change
  to the host contract; the library is unpublished and `App` is the one
  shipped host.
- **Validation points.** The ticket is checked when the host takes the
  request, at prepare, at commit, when the result is delivered and at
  retirement. A stale ticket completes as cancelled and touches nothing.
- **A showing that ends while its request is queued or held by the host**
  cancels the ticket. The next showing gets a new ticket and a new window
  identity; it does not wait for the old native window to be destroyed.
- **The owner's window when the request was made** is recorded. If the owner
  has moved to another window before commit, the showing follows it (the
  window is opened for the owner where it now is); a move is not a failure.
- **After commit, before the host has installed the real viewport**, a
  failure retires the window again and reports the showing as failed, so no
  mounted root is left without native state. This closes a gap in today's
  detach path as well (`platform/windows.rs:93`, `:163`).

## 4. Showing, pending and hiding

- **Pending.** Between the element becoming visible and the host's commit,
  the element is laid out nowhere and takes no focus. This is a state the
  runtime tracks, not a hidden element and not an in-window presentation.
- **Hiding is its own operation**, not a redock: the runtime returns the
  nodes to their logical owner as hidden, cancels held input and drags in the
  window, selects a surviving window, keeps cleanup errors and retires that
  one window identity. It does not select a tab, call `child_redocked` or
  restore focus into the closing content. Session end, an ancestor being
  hidden, removal of the property and unmount all use it; a minimized or
  occluded native window is not a hide.
- **The host learns of retirement by reconciling identities**, as `App`
  does today on every loop turn. The host contract gains the obligation: a
  host destroys the native resources of any window identity the runtime no
  longer contains, also when it has no request to serve, and the runtime
  wakes the host when it retires one.
- **Results are delivered after settling**, in a writable phase, through the
  parent component (`Component::child_window_result`, name open), so a
  failure found during an update can end a dialog session safely.
- **Fallback for an ordinary view.** A showing that fails leaves the view
  shown docked in its parent, with the reason on the result; it is not
  retried until the element is hidden and shown again or described anew.
- **The window's close button** is a request to the parent component, which
  alone changes what is visible: it may hide the element or refuse. Of the
  existing close policies only `KeepOpen` remains meaningful for the
  property; `Redock` and `RemoveView` are refused when the options are given
  to `own_window`, because one would expose a view that has no docked form
  and the other would destroy a retained child.
- **A host that cannot open windows** says so once: the host declares the
  capability (a registration on the runtime, name open), and without it a
  showing fails at once as unsupported, with a new `WindowError` variant.
  This is what keeps decision 8's promise for a bare `Runtime`, where nobody
  drains the queue and the session would otherwise block the application for
  good.

## 5. `Dialog` on top of it

```rust
// API sketch.
Dialog::new(controller, anchor, content).own_window(NativeWindowOptions::new("Open")?)
FileDialog::new(ui, &controller, anchor)        // own window; arrives with the native slice
FileDialog::in_window(ui, &controller, anchor)  // unchanged
```

- **A modal record owned by the session.** When the session opens, the
  runtime reserves the application-modal scope for the dialog's content
  directly: a record naming the modal subtree, its logical owner (the
  `Dialog`, which receives dismissals) and the showing. It does not depend on
  a presentation parent, a paint order or a nonempty clip, so it holds while
  the window is pending and after the content has moved. The existing
  predicate (`application_modal_active`) reads it. Conflict with another
  modal and nested admission are checked against it as for a surface.
- **From which moment input is blocked**: from the flush in which the owner
  observes the open session, before any request reaches the host. Held
  input, captures, drags and text composition in every other window are
  cancelled then, as application-modal admission does today
  (`runtime/surfaces.rs:662`).
- **Focus.** The component focused when the session opened is recorded with
  its window as the opener. Focus enters the dialog's content when the window
  commits. When the session ends, focus returns to the opener if it is still
  mounted, visible and admitted, else to its nearest eligible ancestor;
  operating-system activation of the owner's window is a separate request.
- **Content in the window.** In its own window the panel fills the window:
  no scrim and no centered inset, which belong to the in-window presentation
  (`dialog.rs:514`). Escape and Tab trapping apply in the dialog window's own
  surface stack, where a nested alert is admitted.
- **Title**: the window's title is the options' title, which `FileDialog`
  sets from the session's title; a plain `Dialog` must be given one, since
  its semantic name may be empty and a window title may not.
- **Size**: `FileDialog::size` is the window's client size in logical
  pixels. A size the user drags is remembered by `FileDialogMemory` as a
  schema change in the native slice (decision 12).
- **Close policies**: a dialog accepts `KeepOpen` only as
  `require_response`; otherwise the close button cancels.

## 6. Failure, lifetime and closing

One session has exactly one outcome and one showing has exactly one result;
they are decided separately, first ending wins:

| Event | Session outcome | Showing's result |
|---|---|---|
| Host lacks the capability | `Failed` | unsupported |
| Request queue full | `Failed`, delivered after settling | not queued |
| Host reports failure (creation, surface, viewport) | `Failed` | failed, with the cause |
| Host drops the request | `Failed` | cancelled |
| Accept, cancel, Escape, the dialog window's close button | that outcome | cancelled if still pending |
| Owner unmounted or hidden | `Dismissed`, as today | cancelled if still pending |
| Runtime shutdown | `Dismissed(OwnerRemoved)`, as today | cancelled |

`Failed` carries a new `SurfaceDismissal::WindowUnavailable`. There is no
fallback to the in-window dialog (decision 8).

**Closing other windows while the session is open.** The maintainer proposed
locking them. The proposal here:

- A close request for any window but the dialog's is refused while an
  own-window modal session is open. The refusal comes before focus is
  released and before any reactivation bookkeeping, so it cannot save or
  cancel a draft in the dialog; it needs no flush, so it works during error
  backoff; and it applies alike in `App`, the headless host and a custom
  host, because it is part of close admission in the runtime.
- **A close already deferred when the session opens** keeps its decision:
  an application that deferred closing the main window in order to ask
  through this very dialog must be able to accept afterwards. An accepted
  deferred close dismisses the session first.
- **The operating system quitting the application cannot be refused** (on
  macOS, Quit terminates through winit without a veto). The session is then
  dismissed with everything else at shutdown.
- A blocked close requests activation of the dialog window, where that is
  supported.

## 7. Platform

No operating-system service is assumed; each is reported per capability.
Read in the pinned winit 0.30.13 source, not run:

- **Windows**: an owned top-level window is available through a safe call
  (`WindowAttributesExtWindows::with_owner_window`); the system keeps it
  above its owner, hides it when the owner is minimized and destroys it with
  the owner. The last point needs an ordering rule against the runtime's own
  retirement.
- **macOS and X11**: only the generic parent-window call, which is `unsafe`
  and takes a raw handle. It is a new unsafe boundary and needs the
  maintainer's approval before use; without it the dialog is an ordinary
  top-level window that can end up behind its owner.
- **Wayland**: no parent relation through winit; activation of an existing
  window is unsupported, as today.

`App` today sets only title and size on a detached window. Handles stay
inside the platform adapter. No native behavior was verified.

## 8. Workspace and inspection

Workspace state is keyed by logical paths and includes hidden and detached
mounted children, so a restore never needs the dialog window to exist and
never opens a session. The component keeps its identity across the move; a
reopened dialog gets a fresh window identity, and late native events,
receipts and frame captures for a retired identity are rejected. While a
showing is pending, inspection reports no geometry for the content rather
than its former place.

## 9. Decisions asked of the maintainer

1. **Go ahead with the property as the general mechanism**, knowing from
   section 1 that it does not make the slice smaller.
2. **A new presentation action with a ticket** (section 3) instead of
   reusing `Detach` unchanged. It amends the parent plan, whose slice 3
   already called for "a distinct presentation request".
3. **The host declares that it can open windows** (section 4). This
   replaces the first draft's proposal to document the requirement only.
4. **The close lock and its two exceptions** (section 6): a close deferred
   before the session keeps its decision, and an operating-system quit
   dismisses. Also whether an in-window application-modal dialog gets the
   same lock; today it refuses no close.
5. **Fallback for an ordinary view**: shown docked on failure (proposed), or
   not shown.
6. **Owner relation per platform** (section 7): use the safe call on
   Windows; decide separately whether to accept an unsafe boundary for macOS
   and X11.
7. **The slicing** in section 10, where `FileDialog::new` becomes the
   default only in the native slice (decisions 7 and 12 of the parent plan).

## 10. Slices

Each is public API or scheduling: workspace gate, API listing gate, contract
tests and an independent review. The last is the platform row.

- **3a. Own window for an ordinary view.** The property, showings and
  tickets, the pending state, hiding as its own operation, host capability
  and the reconciliation obligation, results after settling, the fallback.
  Headless tests with a fake host and with no host: commit, refusal, a
  dropped request, a full queue, two showings faster than the host serves,
  the owner moved while pending, hide and unmount while pending and while
  open, failure after commit, workspace capture open and closed.
- **3b. Modal handoff.** The session's modal record, the pending
  reservation, blocking and cancellation in other windows, opener and focus,
  the window content without scrim, nested alert, `Dialog::own_window`.
  Tests: every row of the table in section 6; a press begun before open and
  released after commit; keys, text composition and a drag in a parked
  window; targetless commands with a main window and two detached views.
- **3c. Close admission.** The lock, deferred closes, backoff, the main
  window ending the application, the dialog window's own close and
  `require_response`, for `App` and the headless host.
- **3d. Native.** Owner relation and placement per platform, activation, a
  native check in `pert-ui-native-checks` with Linux evidence,
  `FileDialog::new` as the default, the gallery showing both presentations,
  and the remembered window size as a `FileDialogMemory` schema change.
  Windows and macOS evidence stays recorded as missing until run.

## 11. Review record

Codex, 2026-11-20, on the first draft: five blockers and eleven further
points. What this draft changed:

- The claim that a detached modal root stays modal was wrong; section 5 has
  a session-owned modal record instead of an exemption from `ModalBlocked`.
- "Laid out nowhere" could not pass ordinary surface admission; sections 4
  and 5 define a pending state and a reservation independent of paint.
- A queued `Detach` cannot tell showings apart; section 3 adds the ticket,
  and "no new presentation action" is withdrawn.
- Hiding by redock was wrong; section 4 defines the operation.
- A blanket close lock conflicted with deferred closes and cannot stop an
  operating-system quit; section 6 states precedence and limits.
- The undrained-queue hole is closed by a host capability rather than a
  documented requirement; shutdown is `Dismissed`, not `Failed`.
- The platform section is per platform, from the pinned winit source.
- The slicing is four slices, with the public default in the native one.

Not answered by this draft and left to the slices' own notes: the exact
names of the new callbacks and registration, and the first-frame contract
(what the user sees between session open and the window's first drawn
frame), which needs a native run to state.
