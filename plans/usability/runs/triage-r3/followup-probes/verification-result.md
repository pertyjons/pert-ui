I ran fresh probes for P7–P11 against both frozen kits, head 0.3.88 and baseline 0.3.75. All five leads are verified. P8 and P9 are real library defects; P7 and P11 are documented behaviour that needs recipes; P10 confirms the tab box on both commits.

One probe source (`lib.rs`) compiled unchanged on both kits, with only the pinned `pert-ui` version and `cpu` feature as dependencies. Every cargo command went through `run.sh exec`. All six tests passed on both kits, and the head and baseline outputs are byte-identical, P10's captures included.

| Lead | Verdict | What it is |
|---|---|---|
| **P8 / N6** | Verified, both kits | Library defect |
| **P9 / N8** | Verified, both kits | Diagnostic defect (low) |
| **P7 / N4** | Verified, both kits | Documented behaviour; recipe verified |
| **P10 / N7** | Verified, both kits | Rendering defect |
| **P11 / N5** | Verified, both kits | Documented behaviour; recipe gap |

**P8 (stale pointer takes over the menu highlight).** I parked the pointer with one host Move while the menu was closed and sent no pointer events after that. The guide's F10, Down, Enter recipe then ran `row:Gamma` instead of the Save command.
- A `Driver::click` on content under where the Beta row opens later led to `row:Beta`.
- A pointer `Leave` first, or a pointer outside the menu, gave Save as documented.
- In a menu long enough to scroll, each scroll on Down snaps the highlight back to the row under the pointer. Thirteen Downs end on R04 instead of R12, so the keyboard can never reach the bottom.
- Cause: after layout, `refresh_hover` re-sends a synthetic Move at the last pointer position, and the menu list (`select.rs:547-551`) moves its highlight on any Move without checking `is_refresh()`.
- This contradicts the documented result (`guide/surfaces.md:160`, `menu.rs:1257`), so it is a bug, not expected behaviour.
- Suggested fix (not implemented): only let a synthetic Move change the highlight while the pointer "owns" it. A real Move over a row sets ownership; keyboard navigation and opening the menu clear it.
- Regression tests for the fix: stationary-pointer open, a click on content then the recipe, a long scrolling menu, and a real Move after opening.
- `Select` popups use the same code but were not exercised.

**P9 (`NotRevealed` diagnostic).** The original refusal comes from the closed Dialog's owner, `confirm`. It was mounted and visible when it refused.
- The same retained error prints `mounted: true` while the runtime lives and `mounted: false` after the runtime is dropped or the owner unmounted.
- A test function that returns the error, so its runtime drops first, prints BC2's exact misleading text.
- Cause: the error's Display includes `WidgetId`'s Debug output, which reads a live flag.
- The stored facts (refusal kind and id) never change; only the printed flag does. Not naming `confirm` is an optional improvement.
- Suggested fix: have `Driver` record the ancestor's key path in the error and print that.

**P7 (Button rows in a list).** After clicking a row's Button, focus stays on that Button while Down moves the list's selection. Enter then presses the Button again, reopening the clicked row (b) while c is selected.
- This follows the documented rules, so it is not a library bug.
- The recipe works on both kits: in the row's callback, call `selection.set_active(Some(key))` and `cx.request_focus_for(&list)`. Click b, Down, Enter then runs `activate:c`.
- `on_activate` stays unchanged.

**P10 (literal tab in `TextArea`).** The CPU captures show a hollow box between A and B. The stored text, inspection text and semantics all keep the tab.
- Glyph ids are not visible through the frozen kits' public API, so the "glyph 0" identification still rests only on the earlier probe run on 0.3.89.
- One detail I can't explain: B lands at the same x position with a tab or a space.

**P11 (adding and removing tabs without reveal).** Both the `Dynamic` and `reconcile_children` paths match the documented contracts.
- Adding a tab keeps the selection and sends no notification.
- Removing the focused, selected tab selects the next one with origin `Redescription`.
- Focus falls back to the tab strip, so text typed next is silently dropped (BC2's "focus lost").
- The recipe should tell applications to refocus the new tab's editor. I did not test such a recipe; that part is still pending.

**Things you should know:**
- I fixed one mistake in my own test: P10's first run had a viewport too short for the third text box. Both iterations are kept.
- The baseline kit's `kit.env` records `dirty=1`.
- While I worked, files in the main repo changed between 20:18 and 20:23: `CHANGELOG.md`, `Cargo.toml`, `controls.rs` and a new `disabled_button.rs`. I wrote nothing there; it looks like separate disabled-button work. The frozen kits these probes used are unaffected.
- Nothing was committed, delegated, or approved or closed as a task.

Everything is in `/home/per/pert-ui-usability/r3-followup-probes-evidence/`; `README.md` has the per-lead tables and source line references.
- `source/`: the probe source, both manifests, the exact compiled inputs with lockfiles (`as-run-head/`, `as-run-baseline/`), and iteration 1's source.
- `runs/`: logs with command headers, checksums and exit status, plus the extracted observations per kit.
- `captures/`: the P10 images.
- `vendor-source-checksums.txt`: checksums of the frozen source files cited.

The probe directories are `/home/per/pert-ui-usability/r3-extra-probes-head` and `/home/per/pert-ui-usability/r3-extra-probes-baseline`.
