# Round 3 checkpoint: pointer focus stays inside an admitted surface

Status: proposed 2026-10-29; independent design review precedes implementation.
This follows a verified case discovered by the MultiSelect repair. It is a
separate runtime responsibility within the usability goal, not a privileged
MultiSelect override.

## Verified defect

The unmodified-source run06 in the MultiSelect author evidence places a short
popup inside an overflowing ScrollArea. A row press focuses the outer `page`,
closes the popup and the release writes no value/callback. With an overflowing
list, its own list is focusable so the popup stays open and the click writes.
Root read the actual log and Runtime::pointer_transition: the automatic focus
walk passes nonfocusable row/panel/surface nodes until it reaches a focusable
ancestor outside the surface. That focus departure dismisses the popup before
the row can handle its press. The isolated MultiSelect change cannot repair
an event that is lost before its handler.

The existing direct-target rule preserves focus when a pressed nonfocusable
component itself contains focus. It does not apply to the surface ancestor
that contains the search sibling. Keep that direct-target rule.

## Proposed generic rule

For primary/other presses that already run the ordinary automatic focus walk:

- The nearest focusable component/ancestor inside the hit subtree/surface still
  receives focus exactly as today. Do not make rows or panels focusable.
- If the walk reaches the root of an admitted interactive surface, and current
  focus is inside that same root, stop there and preserve that focus instead of
  walking outside. Apply to the existing interactive SurfaceKind families
  (popover/context/submenu and modal dialogs); exclude tooltip/notification.
- Require actual current admission from surface state, not merely a component
  describing child_surface. The surface module owns the private query; the
  pointer module uses it without exposing fields or introducing a public hook.
- If focus is outside that surface, do not invent a new preferred focus target:
  retain current admission/focus policies and the existing walk behavior.
- Explicit Runtime::focus, queued focus requests, application callbacks, Tab,
  dismissal/outside presses and capture routing remain unchanged. An app can
  still move focus outside and dismiss its popup intentionally.

No new API, field/schema, focusable/tab-stop override, dependency, renderer or
platform behavior. This uses the public Component::child_surface contract so
custom widgets get the same behavior as built-ins. The change removes an
unintended outside transfer; it does not force a new focus event on the surface.

## Required evidence and documentation

Public input regression: short MultiSelect in an overflowing ScrollArea opens,
clicks an enabled row and remains open, writes once, keeps search focus, then
Down/Enter/type work under the completed control repair. Also test the original
unmodified-source shape of the bug separately from stale-pointer ownership.

Use a public custom surface component to cover nonfocusable leaf/sibling focus
inside popover/context/submenu and modal families, including nested chains;
a focusable ancestor inside the surface still wins. Keep the direct-target
ancestor rule, outside/no-surface focus, already-outside focus, explicit app
focus transfers/dismissal and removed/hidden surfaces correct. No unintended
focus-lost/save/cancel callback fires on the preserved-focus path. A restored
old walk must fail the specific short-popup click/value assertions. Record
actual source hashes, commands/status and exact restore/recompile evidence.

Update Component::focusable and Runtime::pointer_input rustdoc, surface focus
contracts/manual, affected control wording and CHANGELOG; no claim of a new
focus trap or all-input boundary. Run focused contracts, required workspace/API
and affected selected native interaction checks with platform/limits, then
fresh independent implementation review. Do not claim hidden/old geometry as
current focus or reuse the native menu case as MultiSelect click evidence.

The reviewer inspects the declared source/evidence read-only and authors no
fixes or delegation. Approve this precise rule or identify counterexamples
before implementation.
