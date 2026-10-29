**Verdict: approve for implementation, once the corrections below are folded into the design.** The core model holds up against the source. None of the corrections changes the architecture; each pins down a transition the design leaves open. I only read code; nothing was built or run.

## What checks out

- **Mechanism.** `ChoiceRow::pointer_input` (`multi_select.rs:592`) sets the shared highlight on every Move that lands inside a row not already highlighted, and doesn't check `is_refresh`. `Row` (`:58`) is the data struct, so the design's correction of the probe's item name is right. MultiSelect uses SearchField → `Shared::key`, VirtualList and ChoiceRow; it doesn't use OptionList.
- **Where ownership lives versus where coordinates live.** One `Cell<bool>` in `Shared` is right: rows are separate components sharing one highlight. Coordinates must be remembered per row. When a refresh moves the hit to another row, the runtime sends a Leave to the old row at the new position before the new row gets its Move (`runtime/pointer.rs:476-482`). A single shared coordinate updated by Leave would therefore swallow genuine row-to-row motion; this is the same reason MenuTitle keeps one per title. A shared coordinate that ignores Leave fails differently: a row → search field → same-row round trip would never regain ownership.
- **Row Leave versus Panel `hover_changed`.** The synthetic Leave carries no refresh marker and bubbles through VirtualList and Panel (`dispatch_pointer`, `:645`). So neither a row's Leave nor a Panel `pointer_input` Leave handler may clear ownership. `update_hover_path` (`surfaces.rs:723`) leaves Panel's hover unchanged when the hit moves between rows, so `hover_changed(false)` is the correct hook for leaving the panel.
- **Refresh ordering.** A refresh happens only at the end of a flush that did layout (`runtime.rs:6567`). `pointer_transition` flushes pending work before dispatching a host event (`:447-455`). So whatever row sits under a stationary pointer has already recorded that position before any duplicate host Move arrives. That ordering is what makes per-row memory safe, including for virtualized rows that get remounted.
- **Keys.** `SearchKey::Submit` fires only on a fresh press, and a held Enter submits once (`search.rs:436-446`). Clearing ownership before each reported key leaves Enter's target unchanged.
- **Scope.** The design is kept separate from OptionList, needs no public API, writes no signals from reconcile, leaves the frozen results alone and adds no platform coordinate fix.

## Required corrections

1. **The design must state the `active()` rule itself rather than leave it to implementation.**
   - Record the position (Move, Press, Release and Leave) before any early return.
   - Split `active()` into "option enabled" and "dropdown allowed".
   - In an allowed dropdown, a distinct non-refresh Move inside a disabled option gains ownership but changes no highlight.
   - While the dropdown is denied, motion and presses only record the position. Otherwise the clear on revocation does nothing: move while denied, restore, and the next data refresh retargets to the row under the pointer.
   - Ownership gain must come before the `!self.highlighted` guard, so moving onto the row the keyboard already highlighted still gains.
   - Say whether a press on a disabled option gains ownership. I recommend it does, as motion does.

2. **Say where revocation is noticed.** A reactive Permission is currently read only by rows (`ChoiceRow::update`, `:549`) and the Trigger. With no matching rows, nothing would see it. Have `Panel::update` read the enablement and clear ownership whenever it denies; this covers fixed redescription and reactive sources.

3. **Replace "panel show/hide/input reset".** The runtime calls `input_reset` only through `cancel_input` on a capture or gesture owner, which Panel never is. Name the real hooks: `begin`, every close (`dismiss_surface` and `focus_within_changed`) and Panel `visibility_changed`. Also state that redescription (options, labels, filter, binding, commit timing) keeps ownership. Every described filter bumps `filter_generation` (`:232-236`), so treating that as a reset would clear ownership on every redescription.

4. **Query tracking.** `Panel::update` should read `shared.query` directly and compare its revision. Today it only sees the query through `chain.filtered`.

5. **Release after a key.** Specify press A → Down (clears ownership, moves the highlight) → release inside A. The current release still sets the highlight to A (`:605`). Decide whether that release also regains ownership; I suggest yes, since it is a real pointer action. Then test it.

6. **Arrow keys while the list has focus.** When the list overflows, Tab puts focus on the VirtualList and its arrow, Page, Home and End keys scroll it without any `SearchKey`. State that this behaves like the wheel: no change in ownership; with pointer ownership the refresh follows the pointer, with keyboard ownership the highlight stays. Test it separately from `SearchKey` PageDown.

7. **Qualify the window re-entry claim.** Counterexample: the pointer is parked at P over A, leaves the window, the keyboard scrolls X under P (no refresh while the pointer is outside), and the pointer re-enters exactly at P. X has no memory of P, so this counts as motion. Change "identical-coordinate re-entry is stationary" to "…when the same row is still under the pointer", document that, and test it. A related accepted residual: X remembers P from a Leave sent when layout moved it away, the pointer moves elsewhere, X scrolls back under P, and a single jump back to P is treated as stationary. Pin that with a test too.

8. **Make the panel-exit clear testable.** A Move back into the panel always regains ownership, so nothing in the current plan would fail if the `hover_changed` clear were deleted. Add: pointer owns a row → moves just outside a short panel → bound options grow until a row lies under the stationary pointer → no retarget. The negative control is the same sequence without the clear, which should retarget.

## Lead to verify before implementing (source only, not run)

In a long list, clicking a row probably moves focus from the search field to the list. When a press lands, the runtime focuses the nearest focusable ancestor (`pointer.rs:495-515`). ChoiceRow isn't focusable, but VirtualList is whenever it overflows (`virtual_list.rs:2109-2114`). That would contradict the rustdoc's "A primary click on a row toggles it without moving focus". After such a click, Down would scroll instead of moving the highlight, Enter wouldn't toggle and typing would go nowhere. The existing test (`a_click_on_a_row_toggles_it_without_moving_focus`) uses six options, so the list never overflows. Probe a long-list click followed by a key through public input. If confirmed, either fix it here (the required click-then-keyboard tests depend on it) or record a separate TODO and limit those tests. The plan must not assume `SearchKey` follows a long-list click.

## Can public tests reach every transition?

Yes, with the additions above. Everything goes through `pointer_input` (Move, Press, Release, Leave and Scroll), `key_input` (Press and Repeat), `text_input`, bound options or permission, `set_viewport` scale, and inspection of `highlighted`, `checked`, the bound value and on_change. Tests can't create refresh events directly, so they must trigger them through layout changes: keyboard reveal, wheel or a data change. That is the correct route anyway.

Nonblocking: the wheel-at-limit test should also confirm that a wheel bubbling to an enclosing scroll area can't gain ownership.
