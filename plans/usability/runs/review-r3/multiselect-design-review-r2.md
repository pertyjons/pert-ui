**Verdict: approve for implementation once one ordering blocker is fixed in the design.** The architecture is sound and all 8 revision‑1 corrections are adopted. The blocker and the required pins below only change ordering and wording; none needs a new API. This is a design approval only, not implementation, native or overall usability approval. I only read the code; I didn't build, run or edit anything.

## What checks out

**All 8 corrections are adopted in revision 2:**
1. Positions are recorded before early returns, and option enabled is split from dropdown allowed, with rules for disabled options and a denied dropdown (L101‑105).
2. `Panel::update` reads the permission directly, including with empty results (L105‑106).
3. The real hooks are named, input_reset is not relied on, and redescription or a changed `filter_generation` keeps ownership (L107‑112).
4. The query revision is compared directly (L106‑107).
5. Release after press A, then SearchKey Down, regains ownership and is tested (L112‑114).
6. Direct list keys follow the wheel policy (L114‑115).
7. Re-entry is qualified as "same row still under the pointer", and both per-row residuals are pinned (L117‑122).
8. The panel-exit clear is tested by growing the popup under the pointer, with a negative control, plus the nonblocking wheel-bubble case (L122‑124).

**Probe.** `run4.log` matches the plan. With 6 options, focus stays on `search#8`. With 20, focus is on `list#12` after the click. With the pointer Leave control, long‑list Enter writes nothing and typed text doesn't reach the field (lines 22 vs 27). One small wording point: L132 says "query stays empty", but the probe measured the search field's semantic value (its editor text), not the `query` signal. The conclusion holds.

**Identity lifecycle.**
- `WidgetId` holds a strong `Rc<Node>`. Components live in `Runtime::entries`, not in `Node`, so keeping the panel's ID in `Shared` creates no cycle.
- The pending request holds only a `Weak` (`native_windows.rs:23`), and an unmounted or hidden target is ignored. IDs never retarget.
- Everything used is public (`UpdateContext::id`, `EventContext::request_focus_within`), so a consumer could build the same thing.

**Target resolution.**
- `first_focusable_within` (`runtime.rs:6429`) skips Panel, which isn't focusable. It reaches `search` first because it is child 0 and `SearchField::focusable` is `editor.enabled`, which MultiSelect never disables.
- In a short list the request is a no-op: `focus_with_reveal` changes nothing when the target is already focused (`:4865`).

**Capture.** The row releases capture before making the request, and the request is applied only at the next flush. So focus leaving the list can't cancel a capture that is still held.

## Blocker

**1. When the focus request is made relative to `on_change`.** `Shared::toggle` calls `report` synchronously (`multi_select.rs:337‑340`). A later request replaces any earlier pending one in the window (`runtime.rs:3493, 3507`). As written ("after … applying its toggle"):
- **It overrides the application.** An `on_change` that calls `request_focus_for(other)` is the documented coordination use (`:3496‑3499`), and the row's request would then overwrite it.
- **It skips the repair on callback errors.** `self.shared.toggle(cx, …)?` returns early on a callback error, so the request is never made and the long list keeps focus. That is exactly the defect being repaired, now in the error path.

Fix: split `toggle` into two steps, an admitted write and then the report. On release: clear `pressed`, release capture, set the highlight, apply the write, request focus within the panel only if the write was admitted, then call `report` and propagate its error. The admission condition is `toggle`'s own enablement re-check at `:322`, which currently returns `Ok` silently on refusal.

Tests for this:
- `on_change` requests focus for "after", and focus ends there.
- `on_change` returns `Err`: the error propagates, the write stays, nothing is replayed, and focus is on search after the next flush.

## Required pins (wording and tests, no architecture change)

**2. Long‑list press → Down → release.** During a held press the list has focus, so Down scrolls it, and A keeps the capture. The release toggles only if the pointer is still inside A's clip after that scroll (`:604`). State the expected outcome for the actual scroll step and test it. If the release lands outside A, there is no toggle and focus stays on the list.

**3. Presses that don't complete a click leave the long list focused.** This covers an outside release, a press on a disabled option (which per revision 2 gains ownership but never arms, so no release reaches the row), and a press while the dropdown is denied. The rustdoc should say "a completed primary click on an enabled option returns focus to search", not "a click on a row". Decide disabled options explicitly. I recommend documenting it as a residual. The alternative is a capture that only returns focus, which adds a new armed state. Pin it with a long‑list disabled-row click test.

**4. IME residual.** In a long list, focus moves to the list at press time, which calls `SearchField::focus_lost`. That drops an uncommitted composition (`search.rs:748‑754`); a short list keeps it. Returning focus on release doesn't bring the composition back. This is the runtime's existing press-focus behaviour, which the design keeps, so document it as a residual. There is no debounced-query risk, because MultiSelect sets no debounce.

**5. Panel ID hygiene (minor).** Refresh the stored ID on every update rather than only the first, and have `unmounted` clear it only if it still holds this panel's own ID. The unmount test can only check behaviour: remove the chooser mid-gesture, then release, and expect no error and no focus change.

## Documentation the implementation must update

- `multi_select.rs:506`: "it never takes focus".
- `multi_select.rs:1161`: "toggles it without moving focus".

The sentence at `:1165` about Tab moving to the list while it scrolls stays correct under the direct-list policy.
