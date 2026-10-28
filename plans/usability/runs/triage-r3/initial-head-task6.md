# Initial triage: r3-6-head-claude-1, with r3-6-baseline-codex-1 as context

This is a read-only triage. I verified every claim against the packaged kit sources only, and ran no execution probes. Acceptance is not recorded for either run, so no task requirement is marked passed here.

Path prefixes used below:
- `H/` = `kit-r3-head/vendor/pert-ui-runtime-0.3.88/src`
- `HF/` = `kit-r3-head/vendor/pert-ui-0.3.88/src`
- `B/` = `kit-r3-baseline/vendor/pert-ui-runtime-0.3.75/src`
- `RUN/` = `plans/usability/runs/r3-6-head-claude-1`

**A note on the record:** all seven head entries were written at 16:50 with corrected times, not timed as they happened. The session lasted 8.7 minutes. Treat the severities and order as approximate. The probe test the tester used to discover addresses (`tests/probe.rs`) was not kept, so nothing it reportedly printed can be checked directly.

## Per-entry verification (head)

### H1: pointer activation of list rows (discover, annoyance)
- **Claim verified.**
  - `on_activate` runs only on Enter (`H/virtual_list.rs:1103-1116`).
  - A primary press only selects (`H/virtual_list.rs:449-451`).
  - `ChangeOrigin::User` covers both a gesture and a key press (`H/runtime/notify.rs:153-154`).
- **Class:** documented behaviour. The tester's expectation (double-click calls `on_activate`) is a guess at a different contract.
- **Workaround:** a frameless `Button` as the row content (`RUN/submission/src/lib.rs:119-134`). The baseline run used the same pattern without logging it (`runs/r3-6-baseline-codex-1/submission/src/main.rs:191-201`), so the need is corroborated by code in both runs.
- **What it is not:** not a defect, and not an F/R2 recurrence. It is the same on both commits (`B/virtual_list.rs:429,1082`).
- **Recommendation:** don't widen `on_activate`. At most this is an optional-capability lead (follow-ups, or a design checkpoint), or a documentation pointer to the row-content pattern.

### H2: compile entry (`App::size`, `Length`, `SplitPane`)
- **`App::size` returns a `Result`.** This is documented, and the doc example uses `?` with the infallible `Size::from([1280, 800])` (`HF/app.rs:636-661`). It is a tester expectation, not a finding; the diagnostic suggested the fix.
- **`Length: From<u32>`.** Only `From<u16>` exists (`H/containers.rs:13-17`). The guide's `Length::from(240)` infers `u16`. The tester chose a `u32` constant, and rustc named `From<u16>`. Tester choice, no finding.
- **`SplitPane` not in the prelude.** It is in `windowing` by design (`HF/lib.rs:130`; the prelude rule is at `HF/lib.rs:64-66`). rustc gave no import hint (`RUN/diagnostics.md:36-50`). This is a candidate F3 recurrence on both commits: the baseline tester hit it too (`B` facade `lib.rs:123`; baseline FRICTION #1). The membership is by design, so this is the F3 symptom (a missing name, no rustc hint), not a breach of the rule. The author decides F3's scope under the matrix rules.
- **Tester Rust mistakes in the diagnostics:**
  - `LayoutError::from(GeometryError)`;
  - the `&T` → `&mut T` cast in the probe;
  - E0502;
  - E0618 (a local binding shadowing a function).

### H3: inspection and addresses of Tabs, list rows, Dialog and MenuBar (discover, slow)
- **Tabs.** The page has no inspection or test section. Its TabList role, items and `selected` are reported only in code (`H/tabs.rs:1835-1857`; the docs at `H/tabs.rs:214-275` don't mention them). This is a **documentation gap, new**, present on both commits (`B/tabs.rs:1833-1840`).
- **List row key.** The replaced key is documented (`H/virtual_list.rs:30-31,58-59`), and the `NotFound` diagnostic was clear. The tester didn't read the `ListItems` page. **This overlaps TODO.md:148-160** (the placeholder key in `ListItems::build`). Both runs wrote a placeholder key that never takes effect: head `.key("open")` at `lib.rs:133`, baseline `.key("note")` at `main.rs:200`. That corroborates the TODO by code. It is not F9, whose scope is `Each`.
- **Dialog and MenuBar addresses.** Verified, see H4 and H5. **Documentation gap, new**, of the same kind as R2-22, which was scoped to tables, so this is not a recurrence of it.
- **"`address_children(&tabs)` returned nothing."**
  - **Not verified, and contradicted by the source.** `find` and `address_children_of` share `address_segment` and the same walk (`H/runtime/tree.rs:319-363,374-403,2174-2188`). Tab views are keyed elements of their own (`H/tabs.rs:37`). The `viewport` wrapper is a part (`H/tabs.rs:144-145`), so `find(tabs, [name, "editor"])` working implies that `name` would be listed.
  - **Likely explanation:** the probe ran while no tab was open. The submission's own test asserts an empty strip at startup (`RUN/submission/tests/editor.rs:121`).
  - **Probe P1 needed.** Until it runs, this stays pending, not a defect.
- **Requested Driver helper** (list a subtree's addresses): an optional-capability lead. This is not F2: the run used the `Driver` throughout.

### H4: `click_item` on menu rows; `title/file` is not an address child (docs-wrong/missing, slow)
- **`title/<key>` is structural, so `find` cannot reach it.** Verified at `H/menu_bar.rs:700-721`; the content wrapper is also structural at `:742`. `find_descendant` reaches it. The MenuBar page (`H/menu_bar.rs:535-623`) and the guide (`HF/guide/surfaces.md:107-173`) don't give addresses. **Documentation gap, new**, same structure at baseline (`B/menu_bar.rs:702-721`).
- **Menu rows report no bounds.** Verified: the menu rows are an `OptionList` (`H/menu.rs:1069,1117`) whose items are built without `local_bounds` (`H/select.rs:804-830`). So `ItemWithoutBounds` follows from the source; a runtime confirmation is probe P3.
- **The `click_item` doc is not strictly wrong; it misleads.** It names "a menu command or a tab title" as examples of items, then says only items with rectangles can be clicked, and that among built-ins only table titles and toasts have them (`H/testing.rs:830-838`). `ItemWithoutBounds` says the same (`H/testing.rs:1425-1426`). Reclassify from "docs-wrong" to **documentation clarity**. This wording is **head-only**: `click_item` and `address_children` do not exist at baseline (`B/testing.rs`: only `find` and `semantics`). The round 2 P8 fix introduced it.
- **The missing capability is existing TODO.md:170-181** ("R2-27's improvement for other controls"). That entry names `select.rs` but not menus or `ContextMenu`, so its scope should name them. This is not an R2-27 recurrence: R2-27 was scoped to table headers, and the run used no table.
- **The workaround is the documented route.** The tester opened the menu by pointer, then used Down and Enter. The guide teaches F10, Down, Enter (`HF/guide/surfaces.md:160-168`).

### H5: Dialog content is under a `dialog` segment (understand, annoyance)
- **Verified.** The anchor wrapper is structural, but the surface element `dialog` is not, so content sits at `<dialog key>/dialog/<content key>` (`H/dialog.rs:311-343`).
- The Dialog page (`H/dialog.rs:149-175`) and the guide (`HF/guide/surfaces.md:221-269`) don't state it.
- **Documentation gap, new**, on both commits (`B/dialog.rs:316-317`). It is also an asymmetry the docs should at least state; that is a documentation responsibility, not a request to change addressing.

### H6: `click_item` on a tab title (docs-wrong, annoyance)
- **Same verification as H4.** Tab items are built without `local_bounds` (`H/tabs.rs:1842-1851`).
- This is **named explicitly in TODO.md:170-172**. The doc-clarity part is head-only, as in H4.
- The workaround, reopening the note to switch tabs, is app-level and fine. `reveal_child` is the documented route (`H/tabs.rs:258-269`).

### H7: Dynamic versus `reveal_child` timing, and dialog outcomes (understand, annoyance)
- **The quote is accurate:** `defer` runs "before component evaluation" (`H/runtime/tasks.rs:206-207`).
- **The conclusion is unverified, and the source points against it:**
  - The same doc says operations may call `reveal_child`, whose nested flush "settles the tree" (`H/runtime/tasks.rs:257-262`).
  - `reveal_child` flushes before it looks up the key (`H/runtime/reveal.rs:484-493`).
  - So a `Dynamic` written earlier in the same operation may well be described before the key check. Whether it is: **probe P2, pending.**
- **If P2 shows it works,** the gap is that `reveal_child`'s doc says only "settles update and layout" (`H/runtime/reveal.rs:426-428`), not that it settles pending descriptions before resolving the key. That would be a documentation-precision lead.
- **The pattern the tester chose is documented,** and the baseline used it too: `reconcile_children` in a deferred operation (`H/tabs.rs:233,509-537`), then `reveal_child`.
- **Don't widen `Tabs::selected`:** it is documented as mount-only (`H/tabs.rs:395-407`).
- **The DialogBusy/`take_outcome` part is documented** (`H/dialog.rs:33-34,89-106`; `HF/guide/surfaces.md:224-225`), and the tester handled it correctly. No finding.

## F/R2 recurrence (initial, for the author's matrix)

**Candidate recurrence:**
- **F3:** `SplitPane`/`SplitAxis`/`SplitSizing`, with no rustc hint, in both runs. Its scope needs the author's decision.

**Not recurred (exercised without friction, by code):**
- **R2-20:** toast messages, read through the toast stack's items (`RUN/submission/tests/editor.rs:66-69,364`; `H/notifications.rs:412-427,930-938`).
- **R2-4:** `cx.defer(..)?;` with no warning (`H/runtime/tasks.rs:244-247`).
- **R2-6 and R2-12:** list rows through `address_children`, and the capture written with `encode_png`.

All of these are **pending acceptance**.

**Not reached:** F9 and R2-22/R2-27 as scoped, since no `Each` or `Table` was used.

**Baseline-only, already fixed at head:**
- **R2-20:** baseline FRICTION #3, where only a count was visible.
- **R2-4:** six `unused TaskReceipt` warnings in the baseline diagnostics.
- **Tester guess:** baseline #2's `ComponentSnapshot` page.

## New leads

| Lead | Class | Commits | TODO overlap |
|---|---|---|---|
| N1: `click_item` doc lists menu commands and tab titles as example items | documentation clarity | head only | — |
| N2: menu, context-menu and tab items have no bounds | missing capability | both (no `click_item` at baseline) | TODO.md:170-181 (menus not named) |
| N3: Tabs, MenuBar and Dialog have no inspection/address section (`dialog` segment, structural `title/<key>`, TabList items) | documentation gap | both; only head hit it | none |
| N4: pointer activation of list rows | optional capability, not a defect | both | none (no follow-up entry) |
| N5: `reveal_child` settling order | documentation precision, only if P2 confirms | pending | none |
| — : placeholder row keys | existing | both, corroborated | TODO.md:148-160 |

Baseline closing #3 (no multi-document editor recipe) is a possible shared documentation lead. Both runs had to assemble `reconcile_children` and `reveal_child` from several pages.

## Author probes needed
- **P1:** a `Tabs::new([])` strip. Check `address_children(&tabs)` when empty, after `reconcile_children` before a settle, and after one. Compare with `find(&tabs, &[key, "editor"])`.
- **P2:** in one deferred operation, write the signal of a `Dynamic` that describes a `Tabs` strip with a new tab, then call `reveal_child(new_key)`. Is the result `Changed` or `UnknownKey`, and is the state retained?
- **P3:**
  - `click_item` on a `Tabs` strip and on an open MenuBar's `items`: both should give `ItemWithoutBounds`.
  - Also on a `ContextMenu`, to settle the TODO's scope.
- **P4:**
  - `address_children` of a `Dialog`: the anchor content's key plus `dialog`.
  - `find(dlg, [content])` should fail and `find(dlg, ["dialog", content])` should succeed.
  - Do the same for `AlertDialog`.
- **P5:** `address_children` of a `MenuBar`: the content key only, with `find(bar, ["title/file"])` failing and `find_descendant` succeeding. Then check the open menu's address under the title.
- **Acceptance:** run the frozen checks independently for both runs. The tester's five-item claim is not evidence.

## Suggested minimal responsibilities (no fix authored)
- **N1:** the `Driver::click_item` rustdoc (`H/testing.rs:830-838`). Say which built-in items have bounds, without listing unclickable ones as examples, and point to the keyboard routes.
- **N2:** the owner of TODO.md:170-181 extends its list to the menu `OptionList` and `ContextMenu`. Implementing it is the author's in-scope decision for round 3.
- **N3:** the rustdoc of `Tabs`, `MenuBar`, `Dialog` and `AlertDialog`. Each gets an "Inspection and tests" section modelled on `ToastHost` (`H/notifications.rs:412`), plus a pointer from the testing chapter.
- **N4:** no change to the activation contract. Optionally, a documentation pointer from `on_activate` to the row-content pattern. A capability needs a design checkpoint.
- **N5 and P1:** act only on probe results. N5 would touch the `reveal_child` and `defer` wording; P1, only if it is a defect, would touch the runtime tree walk.
- **F3:** the author decides scope. A pointer is the most to consider, not a prelude change.
