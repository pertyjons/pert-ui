**Verdict: approved for implementation, with the five corrections below.** I found no blockers. The rule fixes the verified short-popup case, and the only behaviour it changes is the automatic press-focus walk when focus is already inside the surface. This is a code-reading review only: I made no edits and ran nothing.

## The verified case

- **Run 06** (`06-temporary-probe-scrollarea-baseline.log`, HEAD 17c6c3d, exit 0, hashes the same before and after):
  - With 6 rows, the press moves focus to `page` and closes the popup; the release writes no value and fires no report.
  - With 20 rows, focus moves to `list`, the popup stays open and the click writes `["o02"]`.
  - The probe ran against a page with 40 filler lines, so `page` was a focusable, overflowing `ScrollArea`.
- **The walk path.** A row press walks row → `list` → `panel` → `popup` → owner → `form` → `page`:
  - `list` is focusable only when it overflows (`multi_select.rs:910`).
  - `popup` is a plain `Container` admitted as `Popover` (`multi_select.rs:1486-1520`); the search field holds focus inside it.
  - So `pointer.rs:495-515` passes every node until it reaches `page`. `focus_without_reveal` then marks the popup as departed (`runtime.rs:4868`), and the next sync dismisses it with `FocusLeft` (`surfaces.rs:565-571`).
- **With the rule.** The walk stops at `popup`: it is admitted, and focus is inside it. Nothing calls `focus_without_reveal`, so there is no `focus_lost`, no focus-within-lost notification, no departure flag and no `FocusLeft` dismissal. The row then gets the press from `dispatch_pointer`.
- **Unchanged paths.** Tab (`focus_next`), explicit focus and queued requests, outside presses (`surface_pointer` runs before the walk) and capture routing all bypass this loop. There is no FocusTrap type in the code, and Tab gets no new boundary.

## Corrections the implementation must follow

1. **Write the admission query as a new helper in `surfaces.rs`.** No such helper exists yet, and `ActiveSurface::admitted` is private to that module. The new `pub(super)` helper is true only when a record in `self.window.surfaces` has:
   - `root == current`,
   - `admitted`,
   - `kind.interactive()`,
   - and `self.surface_kind(current) == Some(kind)`.

   `interactive()` is required, not just `admitted`. Notification records are pushed with `admitted: true` (`surfaces.rs:529-538`), and tooltip records go through the normal admission path. The live-kind check covers a cached kind that is out of date, as the comment on `active_modal_kind` warns. Only the current window's records apply; parked windows are excluded.

2. **Placement and the containment test.**
   - Evaluate the check at every node the walk visits, after the `focusable()` check and after the direct-target check. A root that does not qualify is passed through, so in a nested chain the enclosing menu's root can still stop the walk.
   - Use the `focused` snapshot taken at `pointer.rs:494` with `self.descendant(focused, current)`, which counts the root itself as inside.
   - Including the root itself is necessary: when a modal has nothing focusable, admission focuses the root (`surfaces.rs:707`).
   - A focusable root still wins first, which is today's behaviour.

3. **Modal evidence must assert what the old walk actually does.** It differs by kind:
   - **Dialog and ApplicationDialog:** the old walk doesn't move focus. It hits the first focusable ancestor outside the dialog, `admit_focus` refuses with `ModalBlocked` (`runtime.rs:4760`), and the error comes back from `pointer_input` after the press is delivered (`pointer.rs:523`, `408-411`).
   - **ViewDialog:** `modal_allows` accepts ancestors of the owner, so the old walk really does move focus out of the dialog.

   Each modal test asserts `Ok(..)` and unchanged focus. Restoring the old walk must fail these specific assertions. The changelog and rustdoc should say that this `ModalBlocked` result is gone.

4. **The callback checks must be concrete.** For the preserved path, assert all of the following:
   - no `focus_lost` on the search field;
   - no `focus_within_changed(false)` on `panel` or `popup`;
   - no `dismiss_surface(FocusLeft)`;
   - in the MultiSelect commit-on-close mode, no commit before the row handler runs.

5. **Document the remaining limit; don't widen the rule.** When an admitted nonmodal surface has no focusable content, focus stays on the opener, outside the root. A press on the surface's content then still takes the old walk and can dismiss it. That follows from the plan's "focus outside, keep today's behaviour" clause. The `Component::focusable` and `Runtime::pointer_input` rustdoc, the contracts and the manual should therefore say "focus already inside an admitted interactive surface stays there", not "a press inside a popup never closes it". Extending the rule to the owner would be a separate decision.

## Freshness and failure cases

- **Failed sync.** The walk runs only after the flushes before hit testing succeed (`pointer.rs:447-455`). A failed sync records pending modals as not admitted (`surfaces.rs:401-409`) and sets `layout_pending`, so the next press's flush returns the error before the walk runs. A record that is unadmitted, or whose kind changed, is therefore never treated as a stopping point. The `ChangingSurface` fixture in `surfaces.rs` tests can cover "failed or changed admission doesn't stop the walk".
- **Hidden or removed surfaces.** They aren't hit-tested, and focus on hidden or removed nodes is cleared during the flush, so the `focused` snapshot is current.
- **Detached windows.** Both `descendant` and `focus_ancestors` use the presentation parent, which is `None` for a detached node. The walk therefore ends at a detached root, and the rule applies the same way inside a detached window.
- **Nested chains.** When focus is in the parent menu and the press lands on the submenu, the walk continues past the submenu root and stops at the parent root. When the press is on the parent menu while a submenu is open, `surface_pointer` dismisses the submenu and restores the opener before the walk runs.

Two things are out of scope here. The overflowing-list case still moves focus from the search field to `list`, as today; that belongs to the MultiSelect control repair. The `TODO.md` entry at line 277 tracks this design and should be removed when the runtime repair lands.
