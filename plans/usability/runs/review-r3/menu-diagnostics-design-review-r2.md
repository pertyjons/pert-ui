**Verdict: needs changes.** Revision 2 correctly adopts the revision-1 transitions, and P12–P14 support both repairs. Implementation shouldn't start until the gaps below are fixed. Four of them would produce wrong behaviour if built from the plan as written (sections 1–3). The rest are documentation scope. This is a design review only: I used Read/Grep/Glob, made no edits, and ran no builds or tests.

## 1. MenuTitle: decide where the position is tracked (blocking)

The plan leaves "bar-wide versus per-title" to be chosen during implementation. The source already decides it.

- When the pointer moves from one title to another, the runtime first sends `Leave` to the old target, carrying the **new** position (`runtime/pointer.rs:476-483`). Only then does it deliver the `Move` to the new title (`:539`).
- **Bar-wide tracking that records Leave positions** therefore sees the `Move` into View at the same coordinate the File title's `Leave` just recorded. The documented switch between titles (`menu_bar.rs:533-535`) would stop working.
- **Tracking that ignores Leave positions** fails a different case. Hover View at V, move into the content, open File with the keyboard, then genuinely move back to V: nothing switches.
- **Per-title tracking that records every delivered event, including Leave,** passes every P12 row:
  - parked/refresh, duplicate Move, Leave then the identical Move, genuine Move, parked over File;
  - plus switching between titles and the round trip through the content.

Required wording:
- Positions are tracked per title.
- Record the window-space `e.position()` of every delivered Move (host or refreshed), Press and Leave, before the `available`/`action` checks.
- Nothing clears the remembered position: not Leave, `input_reset`/focus loss, reconcile or visibility.
  - Clearing it on focus loss would break this sequence: File is reopened with Left/Right, then a duplicate native Move arrives at the stationary position over File's title.

Add tests for switching between titles and for the content round trip back to a coordinate the title saw before.

## 2. OptionList: define what "last observed position" means (blocking)

- **Coordinate space.** Compare window-space `e.position()`, not `to_local`. Local coordinates change when the list scrolls or moves under a stationary pointer.
- **Record before the editable check.** `pointer_input` returns before doing anything when `!editable` (`select.rs:540`). If positions are only recorded after that, a list whose availability is restored compares the next duplicate native Move against a stale position and gives the pointer ownership.
- **Leave records its own position.** Otherwise a genuine return to the exact pixel where the pointer left would not take ownership back.
- **Never cleared.** As with titles, ownership is cleared on show/hide, `input_reset` and Leave, but the remembered position is not.
- **Name the trade-off.** Re-entering the window at the identical coordinate after a window-level Leave counts as stationary. The next distinct motion takes ownership. Test it.

## 3. Keyboard-armed activation still has pointer paths that change the row (blocking)

Enter/Space run on **release** and read `self.active` at that moment (`select.rs:629-632`, `activation.rs:79-87`). The plan only protects against a refresh between press and release. Three of its own rules still change the row while a key is armed:

- **Press.** "Primary Press … sets active." `ActivationState` refuses to arm the pointer while a key is armed (`activation.rs:39`), but under the new rule `active` would still move, so the Enter release runs the clicked row.
- **Wheel.** A wheel that changes the offset takes ownership. The layout refresh that follows then moves `active`.
- **Genuine Move.** It takes ownership and moves `active`.

Required rule: while a keyboard activation is armed, pointer input neither takes ownership nor changes `active` (hover styling only). Release or Escape restores the normal rules. Press sets `active` only when it actually arms the pointer gesture. Alternatively, bind the armed row at press time. Either way, test press, wheel and a genuine Move between Enter press and release.

## 4. Historical formatter

- **Missing type: `RevealReceipt`.** Its Debug prints `status()` (`reveal.rs:157-162`) and is documented as readable after shutdown. That makes it the clearest historical case. Add it to the documented consequences and to the after-drop tests.
- **Wording of the number.** `inspection_id` refuses an unmounted id (`tree.rs:539-541` → `check`, `runtime.rs:4597-4604`). Say instead: "the value `Runtime::inspection_id` returned for that component while it was mounted, meaningful only within that runtime".
- The rest is right:
  - `serial` is an immutable `Node` field, so no `Cell` or runtime read is needed;
  - `TestError` Display goes through `{refusal:?}` (`testing.rs:1557`);
  - `WidgetId`'s live Debug (`runtime.rs:143-149`) stays unchanged.

## 5. Hidden-window error docs: incomplete list

`Err(RuntimeError::Hidden)` comes out of `reveal_selected` (`reveal.rs:619-628`), so it reaches more callers than the three the plan names. Also document it for:

- `Driver::click_item` (`testing.rs:1015`);
- `EventContext::request_reveal` and `request_reveal_and_focus` receipts, as `RevealStatus::Finished(Err(Hidden))`. `reveal.rs:221-223` currently names only `Unmounted`.
- the `RuntimeError::Hidden` variant doc itself, which says it "cannot receive an editing event";
- the `RevealOutcome` type doc (`reveal.rs:60-61`): state that a host-hidden window is an operational error, and that the call can be retried once the host reports the window visible.

Also check how the content-search jump (`content_search.rs:361-402`) documents the errors it forwards. Correcting the comment at `reveal.rs:620-621` is in scope as planned.

## 6. Behaviour-change docs: missing MenuBar

The plan names Menu/Select rustdoc and the CHANGELOG. P12 changes the MenuBar contract sentence "pointing at another title switches" (`menu_bar.rs:533-535`); it should say actual pointer motion. Its F10 recipe (`guide/surfaces.md:160`) also needs updating.

## 7. Public recipes

- **Tab removal: not implementable as written.**
  - "focus was in that tab strip/content and has moved to the strip" can't be observed after the fact. In `on_select` the recipient is the Tabs node (`tabs.rs:1130-1142`), so the guard you can actually write is `origin == Redescription && cx.focused()`. That also refocuses when focus was already on the strip before the close. Decide whether that is intended and test it.
  - `EventContext` has no descendant lookup.
  - `request_focus_within(&tabs)` would land on the focusable strip itself (`runtime.rs:3510-3524`).
  - So the recipe must say how it gets the new tab's content identity: a per-tab slot (the same friction as P7) or `defer` plus `Runtime::find_descendant`, re-checking the guard when the deferred work runs.
- **Button row: correct; one simplification, not blocking.** Mounting an empty source first isn't needed. A slot read when the click happens works too. Also mention that `request_focus_for` silently ignores a stale id.

## Confirmed as adequate

- Every ownership-clearing point matches the code:
  - keyboard navigation;
  - `on_right`/`on_left` cleared before the action;
  - Enter/Space that actually arms;
  - visibility both ways, `input_reset`, Leave;
  - Select reconcile's `active = None` path (`select.rs:346`);
  - both places where `editable` becomes false (`:311`, `:426`).
- Ownership is kept on `rows_changed` and on enablement/label/option updates (`:283-287`, `:359-423`). That matches the data-following evidence.
- A refresh always comes before a host Move once layout is pending (`pointer.rs:447-455`), so a duplicate same-position Move compares against the refreshed position as intended.
- Separator/disabled rows counting as movement, wheel only when the offset changes, Press for `click_item`, and no publishing during reconcile are all consistent.
- **MultiSelect is not silently included.** It is recorded separately in `TODO.md:243-252` (in the uncommitted working tree), and the plan excludes it explicitly.

## Lower-priority residual (record, don't block)

Native motion rescales its logical position on zoom/scale changes (`platform/motion.rs:52-58`). `pointer_position`, which the refresh uses, is not rescaled. After a zoom with a menu open, the next stationary native Move compares as "moved" and takes ownership. Either test this or document it as accepted.
