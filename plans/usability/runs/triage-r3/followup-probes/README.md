# Round 3 triage follow-up probes: P7–P11 (N4, N5, N6, N7, N8)

This is author-side verification evidence for the round 3 triage, written from fresh context. It is not blind testing, an independent code review, acceptance or human/native evidence. Nothing in the primary repository, the submissions or the kits was written.

## What was run

- **One source for both kits.** `source/lib.rs` contains six `#[test]` probes and imports only the public facade (`pert_ui::*`, `pert_ui::testing::{CpuRenderer, Driver, TestError}`).
  - Manifests: `source/Cargo.head.toml` (`pert-ui = "=0.3.88"`, features `["cpu"]`) and `source/Cargo.baseline.toml` (`"=0.3.75"`, `["cpu"]`). No other dependency or library path.
  - The exact inputs as compiled are in `source/as-run-{head,baseline}/`, including the resolved `Cargo.lock`. The registry checksums are pert-ui 0.3.88 `165b2b32…` and 0.3.75 `b6fb1fcc…`.
- **Harness.** Every cargo command went through `plans/usability/run.sh exec <kit> <probe-dir> …`, wrapped by `scripts/run-probe.sh`. The wrapper writes the command, the kit's `kit.env`, sha256 of `Cargo.toml`/`src/lib.rs`/`Cargo.lock`, the full output and `exit_status` into `runs/*.log`.
  - Head kit: `kit-r3-head`, commit `32dd9c4f`, dirty=0, version 0.3.88, rustc 1.99.1. Probe dir: `r3-extra-probes-head`.
  - Baseline kit: `kit-r3-baseline`, commit `26e6b5a3`, **dirty=1** (as recorded by its preparation), version 0.3.75, rustc 1.99.1. Probe dir: `r3-extra-probes-baseline`.
- **Iterations.** Both are preserved; only my own test was corrected.
  - iter1 (`source/iterations/lib.iter1.rs`, sha256 `8cd2d8b5…`): head build and test plus baseline build, all exit 0. P10's third TextArea lay below my 260 px viewport, so its crop was empty. That was my fixture error. There was also one unused-`mut` warning.
  - iter2 (`source/lib.rs`, sha256 `c1aeefed…`): viewport 440 px and the warning removed; nothing else changed. `cargo test -- --nocapture --test-threads=1` reported 6 passed on both kits, exit 0.
- **Cross-kit comparison.** `runs/{head,baseline}-iter2-observations.txt` hold the extracted `P<n>|` lines. They are **byte-identical** between the two kits. Head's iter1 and iter2 lines are identical outside P10.
- **Assertions.** They check only that input reached the intended scope: F10 focused `title/file`, the File menu became visible, the click hit row b, the canonical text. The behaviour under study is printed, not asserted.
- **Vendor sources.** `vendor-source-checksums.txt` lists the frozen sources I cite. `select.rs`, `menu.rs`, `menu_bar.rs`, `pointer.rs`, `runtime/pointer.rs`, `runtime/reveal.rs` and `dialog.rs` are identical across the kits. `testing.rs`, `runtime.rs`, `virtual_list.rs` and `tabs.rs` differ elsewhere, but the cited items are the same text at the line numbers given.

## Verdicts

| Lead | Verdict | Class |
|---|---|---|
| P8 / N6 | **Verified on both kits.** Reproduced with real public input and callback logs. | Library behaviour defect; contradicts the documented keyboard result |
| P9 / N8 | **Verified on both kits.** The rendered text changes after drop or unmount; the ancestor is unnamed. | Diagnostic defect (low). The missing key is an optional improvement |
| P7 / N4 | **Verified on both kits.** The behaviour follows the documented rules; the composition recipe is verified. | Documented behaviour plus a docs recipe; not a defect |
| P10 / N7 | **Verified on both kits.** Byte-identical CPU captures show a box. | Rendering defect; the baseline gap is now closed |
| P11 / N5 | **Verified on both kits, both paths.** Matches the documented contracts. | Documented; recipe gap (focus after closing) |

## P8 / N6: a stale pointer takes over keyboard menu navigation

**Fixture.** The MenuBar has File = [Save (a `MenuItem::command` for a registered command), Beta, Gamma, Delta] and View = [Zoom], over 12 content buttons. The command action, the receipt and every row and button callback push to one log.

**Calibration.** A host `Move` to each row's middle highlights exactly that row (Save/Beta/Gamma/Delta at y = 60/92/124/156; panel `[12,44,128,128]`). The guide's sequence is: focus content, F10 (focus becomes `title/file#3`), Down (focus becomes `items#6`, the menu visible), Enter. Focus returns to content after Enter.

| Pointer before F10 (no pointer event after it) | Highlight after Down | Enter ran |
|---|---|---|
| never reported | Save | `receipt: Pending`, `command:Save` |
| host Move to (76,124), which Gamma will cover | **Gamma** | **`row:Gamma`** |
| the same Move, then `PointerKind::Leave` | Save | `command:Save` |
| Move to (620,620), outside the menu | Save | `command:Save` |
| `Driver::click` on content `c1`, whose middle (44.5,88) Beta will cover | **Beta** | `content:1`, **`row:Beta`** |
| never; Down, Down | Save then Beta | `row:Beta` |
| Gamma Move; Down, Down | Gamma then **Delta** | **`row:Delta`** |

**Long menu.** Save plus R01–R13; eight rows show, so the menu scrolls.
- Pointer parked over the third shown row: thirteen Downs give R02, R03 … R07, **R03**, R04 … R08, **R04**. Every Down that scrolls to reveal the next row resets the highlight to the row now under the stationary pointer. Enter runs `row:R04` where the control runs `row:R12`.
- With a Leave first, or with no pointer ever reported, the sequence is monotonic and ends at R12.

**Mechanism** (identical files in both kits):
1. A layout or stacking change makes the runtime call `refresh_hover` (head `runtime.rs:6551`, baseline `:6448`).
2. `refresh_hover` dispatches `PointerEvent::new(last, Move).refreshed()` (`runtime/pointer.rs:70-81`).
3. `OptionList::pointer_input` writes `self.active` from any `Move` that hits a row, without checking `is_refresh()` (`select.rs:547-551`). Its only other writers are the update's reset to the first enabled row and keyboard navigation (`select.rs:437, 672`).
4. Keyboard navigation invalidates only paint, unless revealing the row scrolls, in which case it invalidates layout (`select.rs:672-677`, `reveal_active` `:180-187`). That layout triggers the next refresh, which explains the long-menu resets.

**Pre-settle state.** It is not distinguishable through public calls: right after the raw `key_input` Down, before the probe's own settle, inspection already showed the menu visible and Gamma active.

**Why this is a defect, not documented behaviour.**
- The guide's recipe states that F10, Down, Enter choose File › Save (`guide/surfaces.md:160`, both kits). The ContextMenu example says "Enter runs the highlighted first row" (`menu.rs:1257`).
- No menu or Driver documentation says that a pointer position the host reported earlier, or a previous `Driver::click`, takes over the keyboard highlight.
- `PointerEvent::is_refresh` documents the refresh as following geometry under a stationary pointer. It does not say it should override keyboard navigation, and in the long menu it makes keyboard navigation non-monotonic.

**Proposed minimal repair (not implemented).** In `OptionList`, let a refreshed Move change `active` only while the pointer owns the highlight:
- set ownership on a host (non-refresh) Move that hits a row;
- clear it on keyboard navigation and when the list is shown or reset;
- leave host Moves as they are.

This keeps the `is_refresh` contract for pointer-driven hover, such as wheel scrolling under the pointer, which ignoring every refresh would break.

**Regression tests:**
- (a) a host Move parked over row 3, then F10/Down/Enter runs the first row;
- (b) `Driver::click` on content beneath a row, then the guide recipe runs Save;
- (c) a parked pointer plus repeated Down in a scrolling menu is monotonic to the end;
- (d) a host Move after opening still highlights its row;
- (e) the same checks for a `Select` popup, which shares `OptionList` but was **not exercised** here.

Until a repair lands, sending `PointerKind::Leave` before keyboard steps is a verified test workaround. It is not a fix.

## P9 / N8: `NotRevealed` diagnostics

**Real refusal.** A closed `Dialog` (`confirm`) has content `save`.
- `Runtime::reveal(save)` returns `Refused(AncestorRefused { ancestor: WidgetId { serial: 3, mounted: true, .. } })`.
- `Driver::click(save)` returns `TestError::NotRevealed { key: "save", refusal: AncestorRefused { ancestor } }`.
- `ancestor == confirm owner`: its `ui.key` is `confirm` and `inspection_id` is 3. Inspection shows it visible, with its `dialog` child (id 6) hidden.
- `Dialog` has no `reveal_child`, so the default `Refused` applies. That is the true historical reason.

**The same retained error value, formatted at different times:**

| When | Display / Debug fragment |
|---|---|
| runtime alive | `` `save` could not be revealed: AncestorRefused { ancestor: WidgetId { serial: 3, mounted: true, .. } } `` |
| alive, dialog opened afterwards, click now Ok | unchanged, `mounted: true` |
| after `drop(ui)` | the same text with **`mounted: false`** |
| second runtime: the owner unmounted while alive | **`mounted: false`** |
| test-shaped fn with the runtime as a local, `Err` printed by the caller | `Error: NotRevealed { key: "save", refusal: AncestorRefused { ancestor: WidgetId { serial: 3, mounted: false, .. } } }`, BC2's exact form |

**Cause.**
- `TestError::NotRevealed`'s Display is `` "`{key}` could not be revealed: {refusal:?}" `` (head `testing.rs:1444`, baseline `:729`).
- `WidgetId`'s Debug prints the live `mounted` cell (head `runtime.rs:143-149`, baseline `:140-146`).
- Unmounting, including the subtree removal in `Runtime`'s drop, sets that cell to false (head `runtime/tree.rs:1627`, baseline `:1566`).

**Assessment.**
- The historical facts carried, the variant and the serial, never change.
- What is wrong is that the message embeds live state. In the ordinary failing-test shape, where the runtime has already dropped when the harness prints the error, it always says `mounted: false` about an ancestor that was mounted and visible when it refused. That is a verified, low-severity diagnostic defect.
- Not naming the ancestor's key or path is an optional improvement. The target's key is named, and while the runtime lives the serial maps to inspection id 3. After drop nothing can map it, which is why BC2 had to bisect.

**Proposed repair (not implemented).**
- `Driver` already holds the runtime when it builds the error. Have it capture the refusing ancestor's key path into `TestError::NotRevealed`, and format Display from that, for example ``…: `root/confirm` refused to show it``.
- Separately decide whether `WidgetId`'s Debug should stop printing the live flag, or label it as current state. That is a cross-cutting change affecting all Debug output.
- Regression test: format the error after dropping the runtime; it names `confirm` and does not claim `mounted: false`.

## P7 / N4: Button rows with `item_tab_stops(false)`

**Fixture.** A Single-selection `VirtualList` with rows a–e, each an `Open <key>` Button, `item_tab_stops(false)` and `on_activate` logging. Steps: `Driver::click(b)`, Down, Enter, Down, Tab. Identical on both kits:

| Row callback | After click b | Down | Enter | Down | Tab |
|---|---|---|---|---|---|
| logs only | focus `b` button, active None | focus `b`, active/selected **a** | **`button:b`** again; active a | b | `before` |
| + `selection.set_active(Some(b))` | focus `b`, active b | focus `b`, active **c** | **`button:b`**, which resets active to b | c | `before` |
| + `set_active` + `cx.request_focus_for(&list)` | **focus `list`**, active b | active c | **`activate:c`** | d | `before` |

**Documented rules this follows** (head `virtual_list.rs:449-451, 911-920`):
- A press handled by item content does not select.
- Pointer presses still reach content when `item_tab_stops(false)` is set.
- Arrows bubble from the focused content to the list.
- Enter on a focused Button presses that Button.

So this is not a library defect. Its cost is a split state: the visible selection (c) disagrees with what Enter does (reopens b).

**Proposed disposition.** Add the N4 recipe, verified here on both kits. The row's callback calls `selection.set_active(Some(key))` and `cx.request_focus_for(&list_id)`; arrows then continue from the clicked row and Enter runs `on_activate`. One friction point: the callback needs the list's `WidgetId`, which this probe passed through a slot filled after mounting. Do not change what `on_activate` means.

## P10 / N7: a literal tab in `TextArea` (CPU)

**Fixture.** Three `TextArea`s: "A\tB", "A B" and "AB". Captured with `CaptureTarget::window`, served by `CpuRenderer`, written as PNG.
- `captures/head-iter2-p10-textareas.png` and `captures/baseline-iter2-p10-textareas.png` are **byte-identical** (sha256 `ba2ea4ac…`). `captures/head-iter1-p10-textareas.png` is iteration 1's capture with the clipped third field.
- In all three cases the canonical value, inspection `text` and semantics value keep the input: `[65, 9, 66]` / `Text("A\tB")` for the tab field.

**Ink analysis.** Columns that differ from the field's fill:
- tab: x 12–20 (A), **21–28**, 30–37 (B);
- space: 12–20, 30–37;
- adjacent: 12–20, 21–28 (B).

The ASCII dump in the log shows a hollow rectangle at x 21–28 over rows 11–20 for the tab, and blank columns there for the space. The rendered capture shows the box visually.

**Observed but not explained:** B starts at the same x (30) for "A\tB" and "A B".

**Limit.** The public API of the frozen kits does not expose glyph ids. This evidence shows a visible box-shaped mark; that it is glyph 0 still rests on the author's private probe, which ran on the 0.3.89 working tree.

**Disposition.** N7 is a rendering defect at both commits. The repair is as `round-3-design-tabs-whitespace.md`; I did not assess that design.

## P11 / N5: dynamic tabs without reveal

**Fixture.** The same steps on a `Dynamic` over `Signal<Vec<String>>` and on `Runtime::reconcile_children` with `Tab::into_element`. Results are identical on both kits and both paths:
1. **Add c without a reveal.** The selection stays A, no notification is sent, and A's editor keeps its identity. In the Dynamic path the strip keeps its identity too.
2. **`reveal_child(b)`.** Returns `Changed` and logs `a→b Reveal`.
3. **Focus b's editor and type "1".** b's text becomes `1text b`: the caret starts at the beginning, as documented.
4. **Remove the focused, selected b.** The selection becomes **C**, logged as `b→c Redescription` (documented, `tabs.rs:224-227`). b's editor unmounts. **Focus goes to the Tabs strip** (`tabs#…`), not to C's editor, following the documented ancestor fallback (`runtime.rs:1512-1529` head).
5. **Type "Z".** No text changes anywhere; the keystroke is silently dropped. This is BC2's "focus lost after closing a tab".
6. **ArrowLeft.** Moves to A, logged as `c→a User`.
7. **Add d.** The selection is kept.
8. **Remove all.** Logs `a→None Redescription` and focus becomes `<none>`.
9. **Describe d into the empty strip.** Logs `None→d Redescription`.

**Disposition.** This is documented behaviour. N5's recipe should state:
- adding tabs keeps the selection;
- closing the selected tab selects the following one, with origin `Redescription`;
- focus then lands on the strip, so an editor application refocuses the new tab's editor itself.

A concrete refocus recipe was **not exercised** here and is still pending.

## Paths

- Evidence: `/home/per/pert-ui-usability/r3-followup-probes-evidence/`
  - `README.md`
  - `source/`
  - `runs/*.log` and `runs/*-observations.txt`
  - `captures/`
  - `scripts/run-probe.sh`
  - `vendor-source-checksums.txt`
- Probe dirs: `/home/per/pert-ui-usability/r3-extra-probes-{head,baseline}/` (work, home, log, built targets).

No CLI configuration or transcripts are archived here.
