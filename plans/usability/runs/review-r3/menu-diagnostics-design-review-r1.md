**Verdict: needs changes.** The direction of both repairs is right and the frozen probes support it. Before implementation, the plan has to name the exact points where pointer ownership is set and cleared, and the reveal-diagnostics part needs a few small corrections. This is not an overall usability approval.

## What I checked against the evidence

- The head and baseline `*-iter2-observations.txt` files are byte-identical. P8's results come from real public input plus callback logs:
  - A parked pointer over Gamma makes Enter run `row:Gamma` instead of `command:Save`.
  - A `Driver::click` on content under the Beta row makes Enter run `row:Beta`.
  - In the long menu, the highlight goes R07→R03 and R08→R04, and Enter runs `row:R04` instead of `row:R12`.
  - Leave first, a pointer outside the menu, or a pointer never reported: the commands come out correct.
- The source matches that mechanism:
  - `refresh_hover` sends a refreshed Move (`runtime/pointer.rs:70-82`).
  - `OptionList::pointer_input` writes `active` on any Move that hits an enabled row (`select.rs:549-553`).
  - Keyboard navigation that scrolls invalidates layout (`select.rs:675-679`), which triggers the next refresh.
- `OptionList` is built in exactly two places, `Select::with_binding` (`select.rs:1458`) and `MenuBranch` (`menu.rs:1115`). So the shared fix covers Select and InlineSelect, but the probe never exercised them.
- Submenus never open on hover; only choose/Enter or ArrowRight opens them (`menu.rs:1118-1142`). That keeps the lifecycle small.
- `UpdateContext::cancel_input` only calls `lose_context` and does not call `Component::cancel_input` (`runtime.rs:2987-3000`). `input_reset` runs only on focus loss, hiding, unmount, drop, a failed key operation or a component that stops being focusable. If ownership is cleared in `input_reset`, data and option updates in `update` do not clear it.

## Menu highlight: decisions the plan must state

Reword "pointer ownership" as one private rule: **the pointer has acted on this list (moved, pressed or wheeled over it) since the list was last shown, last navigated by keyboard, or last left by the pointer.** Then list every transition:

1. **Gaining ownership:**
   - A host (non-refresh) Move inside the list gains ownership. It also sets `active` when it hits an enabled row. A Move that lands on a separator or disabled row still counts as the pointer acting.
   - A host primary Press gains ownership and should also set `active` to the pressed enabled row. Reason: `Driver::click_item` sends only Press and Release, with no Move (`testing.rs:1028`). Clicking a branch row then gets the right highlight only because a later refresh happens to land on it. Setting `active` on press removes that dependency and keeps ArrowLeft/ArrowRight consistent with the submenu that is open.
   - `pressed` and the release match stay exactly as they are now.
2. **Wheel:** gain ownership only when the offset actually changed. A wheel at the scroll limit causes no refresh. Gaining ownership there would leave the keyboard row highlighted and let a later, unrelated data or layout refresh move it.
3. **Keyboard clears ownership on:**
   - Pressed or Repeated of Up, Down, Home, End, PageUp or PageDown.
   - Pressed of the handled `on_right` or `on_left`. The plan must say this explicitly. Otherwise opening a submenu with ArrowRight lets the parent's highlight jump to the row under the pointer. ArrowLeft then returns to the wrong row and ArrowRight opens a different branch.
   - An Enter or Space press that actually arms. Clearing at the press means a refresh between press and release cannot change the row Enter runs.

   Releases, Escape while idle, modified navigation and keys that are ignored do not clear it. Clear it before calling the action, even if the action then fails.
4. **Resets:**
   - Clear ownership in `visibility_changed`, both when the list is shown and when it is hidden, and in `input_reset`.
   - Also clear it in the Select `reconcile` path that sets `active = None`, and wherever `editable` becomes false.
   - Keep it on menu redescription, including `rows_changed`, and on option, enablement or label changes in `update`. That is the "pointer-driven list follows data" case the evidence expects.
   - The plan's phrase "existing visibility/input-context resets" is ambiguous. `update` also clears `pressed` for option changes, so "reset wherever `pressed` is cleared" would contradict the data-refresh evidence item.
5. **Leave (recommended):** clear ownership when a Leave reaches the list. Without this, a pointer that hovered a row and then moved away can still have the list retargeted when it later grows under the stationary pointer. That is the same stale-pointer defect.
6. **Host Moves at an unchanged position:** the native adapter does not drop them (`platform/motion.rs:25-37`). A host Move with zero movement would retarget the highlight even after the fix. Either make a host Move gain ownership only when its position differs from the last position the list saw (including from refreshes), or record why that is unnecessary on each platform. Include this in the native check.
7. **Visible behaviour change:** a list shown under a stationary pointer (ContextMenu at the pointer, a Select popup over its anchor) now keeps its default or selected row until the pointer moves. This goes in the Menu/Select rustdoc and the CHANGELOG.

**An unverified lead in the same flow:** `MenuTitle::pointer_input` switches the open menu on *any* Move that lands inside another title while a menu is open (`menu_bar.rs:386-390`). Opening File with F10 and Down while the pointer is parked over View probably opens View instead. This is from reading the source only. Probe it first. If it reproduces, a refreshed Move should never switch menus; title hover can still update. If it does not, record it. `MultiSelect` rows (`multi_select.rs:592`) look like the same class of issue. That one is out of scope and should get a TODO.md entry after a probe.

## Reveal diagnostics: approve with corrections

- **Display and Debug are both fixed.** `TestError::NotRevealed`'s Display prints `{refusal:?}` (`testing.rs:1557`), and `TestError`'s Debug is derived, so both go through `RevealRefusal`'s Debug. The same holds for `TestError::Cleanup` nested in either, for the derived Debug of `RevealOutcome` and `RevealStatus`, and for `Box<dyn Error>`. No field, variant or signature changes. `WidgetId`'s own Debug (`runtime.rs:143-149`) is untouched.
- **Correction 1: say what the number means.**
  - `serial` is the value `Runtime::inspection_id` returns (`tree.rs:539-541`), and it is unique only within one runtime.
  - Recommendation: print `AncestorRefused { ancestor: WidgetId { serial: 3, .. } }`. That keeps the existing shape minus `mounted`, built with `debug_struct`/`finish_non_exhaustive` so `{:#?}` also works.
  - Document it as "the ancestor's inspection id at refusal, meaningful only against that runtime's inspection". `TestError::ExposedPartNotFound`'s `owner: u64` is the precedent.
- **Correction 2:** the formatter may read only `serial`. No `Cell` reads, no runtime lookup. Tests must also cover `{:#?}`.
- **Correction 3:** the CHANGELOG must say that the Debug of `RevealOutcome`, `RevealStatus` and `TestError` changes as a consequence. I found no test or doc that pins the old `mounted:` text.
- I agree with declining the key/path field. One related lead, out of scope: `hiding_refusal` falls back to `Hidden { component: target }` (`reveal.rs:679-681`), while `Hidden` is documented as meaning an explicit `set_visible`. Check that before using that path as "a correct historical reason" in tests.

## Integration recipes

- **P7 (button row):** verified on both kits. The recipe must say:
  - Clicking a row moves focus from its Button to the list.
  - Enter then runs `on_activate`, not the Button.
  - The list's `WidgetId` comes from a slot filled after mounting. Record that as friction; add no API.
- **P11 (tab removal):** the README says the refocus recipe was **not exercised**, so it is not yet "verified". Decide the guard before writing it: for example, refocus the newly selected tab's content from `on_select` with origin `Redescription` only when focus was inside the strip or its content. Test the negative case: focus in another region stays put. These recipe tests stay separate from the two runtime repairs, as runnable doctests.

## Required evidence, in addition to the plan's list

- Every transition in decisions 1–6, through public input, reading the highlight, the value or command chosen, and callbacks.
- `click_item` on a branch row, then Right/Left in the submenu.
- ArrowRight with the pointer parked over the parent.
- An Enter press, then a refresh, then the release.
- A wheel at the scroll limit.
- A menu redescription with `rows_changed` while the pointer owns the highlight.
- A Select redescription, and availability revoked then restored.
- Select/InlineSelect versions of the parked-pointer and long-list probes.
- The negative control that reverts the gating, and the MenuTitle probe.
- Diagnostics: the identical-text check across all lifecycle points, for Display, Debug and `{:#?}`, plus the Debug of `RevealOutcome` and `RevealStatus`.

## Wording fixes in the plan

- "repeated Down cannot reach the bottom" → it stalls. After 13 Downs Enter runs `row:R04` instead of `row:R12`, and the maximum creeps up by one row per cycle.
- "replaces its active row on every Move" → "on every Move that hits an enabled row".
