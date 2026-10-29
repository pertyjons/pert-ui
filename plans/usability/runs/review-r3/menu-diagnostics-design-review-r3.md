**Verdict: approved for implementation. No blockers remain.** Revision 3 correctly adopts all seven blocking corrections from the revision-2 review. This approves the design only: the required tests, the workspace/API gate, native checks and a separate fresh implementation review are still owed. I only read files; I made no edits and ran no builds or code.

## Each correction

1. **Per-title coordinates are never cleared — adopted** (plan lines 58–67).
   - Positions are tracked per title, not across the whole bar.
   - Each title records the window-space position of every Move, Press and Leave it receives, before the availability and action checks.
   - Nothing clears the recorded position: not input reset or focus loss, reconcile, visibility or Leave.
   - A title switches only on a host Move at a new coordinate, not a refreshed one.
   - The tests cover switching between titles, the round trip through the content back to a coordinate the title saw before, a duplicate Move after reopening with Left/Right, and every P12 row.

2. **OptionList coordinates are recorded before the editable check and on Leave — adopted** (lines 24–31).
   - It compares the window-space `e.position()`.
   - It records host and refreshed Moves, Press and Leave before the editable check.
   - Nothing clears the recorded position. Clearing ownership is a separate step.

3. **Pointer ownership is frozen while a keyboard activation is armed — adopted** (lines 33–35, 51–56).
   - Pointer Move, Press and wheel update hover only. They can't take ownership or change the active row.
   - A Press sets the active row only if it actually arms the pointer gesture.
   - Recovery after Escape or release is tested.

4. **Tab recipe guard and deferred identity — adopted and implementable** (lines 172–178).
   - `EventContext::focused()` means "this recipient has focus" (`runtime.rs:3547-3549`).
   - `Runtime::focused()` returns `Option<WidgetId>` (`runtime.rs:4127`), so the plan's "recheck `ui.focused()==tabs`" can be written.
   - `Runtime::find_descendant` (`tree.rs:239`) is the public lookup, and `defer` gives the work `&mut Runtime`.
   - The plan explicitly includes the case where focus was already on the strip, and makes no claim about focus on an earlier descendant.

5. **Formatter receipt and id wording — adopted** (lines 153–160, 190).
   - The wording now says the number is "the value `Runtime::inspection_id` returned … while mounted, meaningful only within that runtime". That matches the code: `inspection_id` checks the id and then returns `id.incarnation()` (`tree.rs:539-541`).
   - `RevealReceipt` is in the documented consequences and in the after-drop tests.

6. **Hidden-error docs — complete.**
   - Line 165 already covered `Runtime::reveal`/`reveal_and_focus` and `Driver::click`, plus the internal comment.
   - Revision 3 adds `Driver::click_item`, the queued `Finished(Err(Hidden))` receipts, the `RuntimeError::Hidden` variant, `RevealOutcome` and the content-search jump.
   - I checked that `reveal_target` has no other callers: only `reveal.rs:373` (reveal), `:423` (reveal_and_focus) and `:714` (queued). The other `Hidden` sources in the code aren't part of the reveal path.

7. **MenuBar wording — adopted** (lines 191–192). Both the MenuBar rustdoc and the F10 guide recipe now require actual pointer motion for title switching.

## Non-blocking clarifications for the implementer

- **Same-coordinate re-entry for OptionList.** The trade-off sentence (lines 66–67) sits only in the MenuTitle paragraph. Revision 2's §2 also asked for it, with a test, for OptionList. The OptionList rules already produce that behaviour, so this only affects where it is documented and tested.
- **Zoom/scale residual.** Line 192 says it "must be documented" but doesn't say where. The MenuBar/Menu/Select rustdoc that mentions the motion requirement, plus the CHANGELOG, would fit.
- **Tab recipe robustness.**
  - The deferred work could also check that the tab chosen when it was queued is still selected before focusing.
  - It should do nothing when `find_descendant` returns `None`.
  - It should use an editor key unique to the app, because `find_descendant` refuses a duplicate key (`AmbiguousKey`).

I found nothing outside this scope to record in TODO.md.
