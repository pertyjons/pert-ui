# Round 3, group N: triage of the paired task 6 (notes editor) runs

I did this read-only, from fresh context, with no delegation and no fixes. I checked every claim against the frozen kits (H = `kit-r3-head/vendor/pert-ui-runtime-0.3.88/src`, HF = `…/pert-ui-0.3.88/src`, B and BF = the same at 0.3.75), the run records under `plans/usability/runs/`, the acceptance reports and the executed author probes in `triage-r3/probes`. I did not read any CLI transcripts, stderr logs, credentials or configuration. This is exploratory AI evidence only. It makes no claim about completing round 3.

**Run labels:** HC1 = head-claude-1, HC2 = head-claude-2-retry1, HX1 = head-codex-1, HX2 = head-codex-2, BC1 = baseline-claude-1, BC2 = baseline-claude-2-retry1, BX1 = baseline-codex-1, BX2 = baseline-codex-2.

**Acceptance status:**
- All eight `verifier-status.txt` files read `exit_status=0` and `original_source_unchanged=True`.
- All eight runs are author-adopted as **met**: the four Claude runs in `acceptance/author-review.md`, the four Codex runs in a Codex author review.
- The round-3 plan mentions two verifiers that hit a 429 limit and were resumed. The reports I read don't say which ones, and I didn't open stderr or transcripts. Every final status is terminal 0.
- Evidence the verifiers added is labelled **(acc)** below and kept apart from what the testers themselves exercised.

**Timing limits (disclosed, not penalised):**
- HC1's seven entries were all written at 16:50 with corrected times, so their order and severity are approximate.
- HC2's times are corrected after the fact.
- BC1 and BC2 have "written later" entries.

## Executed evidence I relied on

- **P1:** `address_children(&tabs)` agrees with `find`, both before and after settling. HC2's own passing tests also list tab keys this way (`tests/app.rs:37-39`). This contradicts HC1's claim that it returned nothing.
- **P2:** writing a `Dynamic`'s signal inside a deferred operation and then calling `reveal_child` returns `Changed` and keeps the editor's identity. The source agrees: `reveal_child_selected` flushes before it looks up the key (H/runtime/reveal.rs:484-493).
- **P3/P5:** menu, context-menu and tab items have no rectangles, so `click_item` returns `ItemWithoutBounds`. `title/<key>` is structural, so `find` cannot reach it.
- **P4:** Dialog and AlertDialog content sits under a `dialog` segment.
- **Literal tab:** the box shows in a frozen-head acceptance capture (HC1 `acc2-failed-save-notification.png`, "second□line"). The author's probe that localises it to glyph 0 ran on the 0.3.89 working tree, not the frozen kit; that is a limit.

## 1. Per-run tables

### HC1 (head, met, 8.7 min)

| # | Event | Evidence | Status | Class | Disposition |
|---|---|---|---|---|---|
| 1 | No pointer activation on `VirtualList` rows (annoyance) | `on_activate` fires on Enter only (H/virtual_list.rs:1103-1116). A press on content that handles it does not select (:449-451). Worked around with Button rows (lib.rs:119-134). | verified | documented behaviour | merge into N4 |
| 2a | `App::size` returns `Result` | HF/app.rs:646-658; rustc gave the fix | verified | tester expectation | none (optional: integer/infallible size) |
| 2b | No `Length: From<u32>` | Only `From<u16>` exists (H/containers.rs:13); rustc named it | verified | tester choice | none |
| 2c | `SplitPane` not in the prelude | E0433 with no import hint (diagnostics.md:36-50). Routed to `windowing` on the prelude page (HF/lib.rs:130). Guide mentions it beside Tabs without a module note (getting-started-continued.md:25). | verified | **F3 recurrence** (symptom; membership is by design) | pointer fix, no prelude change |
| 3a | Tabs page has no inspection section | Not documented in H/tabs.rs:214-275; role and items only in code (:1836-1846) | verified | new, docs | N3 |
| 3b | Row key replaced by the list | Documented in prose (H/virtual_list.rs:30-31, 58-59). The placeholder `.key("open")` never takes effect. | verified | documented; existing TODO (ListItems placeholder key) | TODO corroboration |
| 3c | Dialog and MenuBar addresses undocumented | Probes P4/P5; MenuBar docs at H/menu_bar.rs:530-623 | verified | new, docs | N3 |
| 3d | "`address_children(&tabs)` returned nothing" | P1, plus HC2's passing tests | **contradicted** | tester error (probe likely ran before any tab existed) | none |
| 3e | Wants a subtree-address helper | One-level `address_children` exists | — | optional suggestion | none |
| 4 | `find(bar, ["title/file"])` fails; `click_item` on menu rows fails (slow) | P3/P5; doc text at H/testing.rs:830-838 | verified | new, missing capability (N2) plus docs clarity, head only (N1) | N1, N2 |
| 5 | Dialog content needs the `dialog` segment | P4; H/dialog.rs:315-343 | verified | new, docs | N3 |
| 6 | `click_item` on a tab title fails | P3 | verified | N1 + N2 | N1, N2 |
| 7a | "A `Dynamic` would not be described before `reveal_child`" | P2; reveal.rs:484-493 | **contradicted** | tester inference; docs are imprecise | N5 |
| 7b | Must take the outcome before reopening (`DialogBusy`) | Documented (H/dialog.rs:33-34, 89-106) | verified | documented, positive | none |
| diag | `LayoutError::from(GeometryError)`, `&T→&mut T` cast, E0502, E0618 | diagnostics.md | verified | tester Rust errors | none |
| wishes | (1) Menu rows and tab titles clickable, or fix the doc. (2) Inspection sections plus an address-listing helper. (3) Pointer activation. | as above | — | — | N1/N2, N3, N4 |
| (acc) | Verifier first counted any visible Dialog-role component as an open dialog; the owner is always present and its `expanded` / `open` property shows the state (H/dialog.rs:408-425) | verifier error that exposes the docs gap | verified | supports N3 | — |
| (acc) | Caret sits at the start after `focus()` | Documented (HF/guide/testing.md:142-143) | verified | verifier error | none |
| (acc) | Literal tab draws a box | Frozen-head capture | verified | new defect | N7 |
| (acc) | Menu row clicked from geometry derived from the panel | No item bounds | verified | N2 | — |

### HC2 (head, met, 7.4 min)

| # | Event | Evidence | Status | Class | Disposition |
|---|---|---|---|---|---|
| 1 | Long pages read through pandoc; wants a signature index; praises the guide | Reading aid only | verified | kit / reading aid, plus positive note | kit list (optional index) |
| 2 | Row key replaced; claims this is "only a comment" | Stated in prose on `ListItems::build` (H/virtual_list.rs:58-59) | partly contradicted | documented; existing TODO | TODO corroboration |
| 3 | `dialog` segment undocumented | P4; source grep confirms no doc | verified | new, docs | N3 |
| diag | `ColorRole::Danger` guess; `WidgetId`/`KeyCode` E0425/E0433 after a scratch test dropped the prelude glob | diagnostics.md | verified | tester errors (not F3: both names are prelude members) | none |
| wishes | (1) Table of structural segments. (2) Signature index. (3) Tabs described from a binding, and does selection survive a `Dynamic`? | Selection retention on redescription is documented (H/tabs.rs:395-407); P2 shows `Dynamic` + reveal works | — | (1) N3; (2) kit; (3) optional, needs a checkpoint; recipe gap is N5 | N3, N5 |
| note | Divider drag at "+4 px" | Divider is documented as 8 px at Standard (H/split.rs:97, 172) | verified | not a guess; no finding | — |
| (acc) | Menu titles have `path: None`; rows have no bounds | — | verified | N2/N3 | — |
| (acc) | Toast message readable as a semantic item | — | verified | positive (R2-20 fixed) | — |

### HX1 (head, met, 6.5 min)

| # | Event | Evidence | Status | Class | Disposition |
|---|---|---|---|---|---|
| 1 | `Tabs::selected` is mount-only; believes new tabs "need a later flush" | Mount-only is documented (H/tabs.rs:395-407). The flush belief is contradicted by P2. The code does a double `defer` (main.rs:107-120) and settles three times (:404-408). | partly contradicted | documented, plus a misinference shown in code | N5 |
| 2a | `SplitPane` not in prelude | diagnostics.md:97-118 | verified | **F3 recurrence** (symptom) | pointer |
| 2b | Guessed `KeyChord::ctrl` | rustc listed the constructors; `KeyChord::new` has an explicit Control example (H/runtime/commands.rs:41-66) | verified | tester guess; R2-10 passed | optional `ctrl` sugar |
| 2c | E0308 returning `Result<_, RuntimeError>` where `AppError` is expected | rustc suggested `Ok(..?)` (diagnostics.md:121-133) | verified | accurate diagnostic; not the F11 failure | none |
| 3 | Empty Tabs region has zero width | `content` Column has no `.align` (main.rs:297-305); Column defaults to `Align::Start` (H/containers.rs:1099-1103) | contradicted as a Tabs issue | application layout | none |
| wishes | (1) Recipe or controller for adding and selecting tabs, including timing. (2) Document-editor guide. (3) Drag helper and divider bounds; explain zero width. | — | — | (1)(2) N5; (3) optional; zero width is the app's layout | — |
| (acc) | No accessor for the mounted root | `App<S>` hands state back (R2-1 fix); the app stores its root in S | verified | documented design, optional | none |
| (acc) | Notification is a plain `Label`; byte-order sort | — | verified | application observations | — |
| (acc) | Menu rows and tabs have no bounds | — | verified | N2 | — |

### HX2 (head, met, 4.5 min)

| # | Event | Evidence | Status | Class | Disposition |
|---|---|---|---|---|---|
| 1 | Activation is Enter-only | as HC1 #1 | verified | documented | N4 |
| 2 | `SplitPane` not in prelude | diagnostics.md:4-25 | verified | **F3 recurrence** (symptom) | pointer |
| 3 | `click_item(&tabs, 0)` gives `ItemWithoutBounds` (slow) | P3; H/testing.rs:830-838 | verified | N1 + N2 | N1, N2 |
| 4 | Column needs `align(Stretch)` | Documented default (H/containers.rs:1099-1103) | verified | documented; tester learned it | none |
| wishes | (1) Tab bounds. (2) Unified activation or an example. (3) Compact import and grow/stretch reference. | — | — | N2, N4, optional | — |
| (acc) | Plain-Label notification; per-tab status line; byte-order sort | — | — | application observations | — |
| (acc) | Menu titles not addressable | — | verified | N2/N3 | — |

### BC1 (baseline, met, 11.6 min)

| # | Event | Evidence | Status | Class | Disposition |
|---|---|---|---|---|---|
| 1 | Activation is Enter-only | B/virtual_list.rs:1082, same as head | verified | documented | N4 |
| 2 | Tabs from a changing set; does selection survive a `Dynamic`? | B/tabs.rs:271, 396 | verified | recipe gap, both commits | N5 |
| 3 | Root `WidgetId` has to be smuggled out through an `Rc<RefCell<Option>>` slot (written later) | lib.rs:264-273; notes.rs:26-28 | verified | **R2-1 recurrence**, baseline only (fixed by P11) | none |
| 4 | F10 + fixed Down presses pick different rows (runtime, slow; written later) | `refresh_hover` sends a synthetic Move at the last pointer position (B/runtime/pointer.rs:70-81, identical at head). `OptionList` sets its highlight on any Move (B/select.rs:547-551, identical at head). | mechanism verified; exact repro pending (P8) | **new**, both commits | N6 |
| 5 | Toast text not in inspection | B ToastStack inspect | verified | **R2-20 recurrence**, baseline only | none |
| 6 | Read the source to confirm #5 | same event as #5 | verified | source read | — |
| 7 | Is `VirtualList` child order display order? Sorted by y | notes.rs:52-63 | verified | **R2-6 recurrence**, baseline only | none |
| 8 | "Docs don't say which match wins" for `find_descendant` | `AmbiguousKey` documented with an example (B/runtime/tree.rs:174-203) | **contradicted** | tester error | none |
| diag | `Semantics` not found; rustc suggests a type parameter | diagnostics.md:14-24 | verified | **F3 recurrence** (the round-2 name, fixed at head) | none |
| code | `let _receipt` five times; `pending.wait()` on the UI thread | lib.rs:218 etc.; notes.rs:413 | verified (inferred from code) | R2-4, R2-28 | none (fixed at head) |
| wishes | Toast text; root accessor; tabs from a binding, row pointer activation, menu highlight rule | — | — | R2-20, R2-1 (both fixed); N5, N4, N6 | — |
| (acc) | Sidebar highlight doesn't follow the opened note | Consequence of the Button row (B/virtual_list.rs:429-ish rule) | verified | N4 cost | — |

### BC2 (baseline, met, 12.9 min)

| # | Event | Evidence | Status | Class | Disposition |
|---|---|---|---|---|---|
| 1 | Tabs at run time assembled from three pages | BF/guide/layout.md:152-182 has no recipe | verified | recipe gap | N5 |
| 2 | Assumed `ChoiceError` won't convert | `From` is documented; no diagnostic | verified | tester error (F11 not reached) | none |
| 3 | First build worked | — | — | positive | — |
| 4 | Menu rows can't be clicked; `find_descendant(bar, "items")` gives `AmbiguousKey` | No `click_item` at baseline; the ambiguity is documented behaviour | verified | N2 (both commits) + N3; F23 passed | N2, N3 |
| 5 | `NotRevealed { AncestorRefused { … mounted: false } }` (slow) | Root cause was the tester's own focus bug. `WidgetId`'s `Debug` prints the live `mounted` flag (H/runtime.rs:143-149), and the refusal carries only an id. | mixed: application error plus a diagnostic lead (pending P9) | tester error + new lead | N8 |
| 6 | Toast text missing (slow) | — | verified | **R2-20 recurrence** | none |
| 7 | Focus lost after closing a tab; nested `flush` inside a command action | Not covered by `register_command` docs (B/runtime/commands.rs:519-553); `defer` documents nested flushes | verified | docs precision | N5 |
| 8 | `cargo run` fails without a display | expected | — | note | none |
| diag | E0428 and a delimiter error in scratch files | tester noise | — | — | — |
| code | Root slot in `startup.rs:17`; `wait()` in `capture.rs:40` | — | inferred from code | R2-1, R2-28 | — |
| wishes | Toast text; a `Driver` "activate item"; tabs recipe; name the refusing key | — | — | R2-20; N2; N5; N8 | — |
| (acc) | Hover highlight left by the last pointer position shows in the verifier's capture | — | verified | supports N6 | — |

### BX1 (baseline, met, 5.7 min)

| # | Event | Evidence | Status | Class | Disposition |
|---|---|---|---|---|---|
| 1 | `SplitPane` not in prelude | diagnostics.md:4-25 | verified | **F3 recurrence** (symptom) | pointer |
| 2 | Guessed a `ComponentSnapshot` page | — | verified | tester error | none |
| 3 | Toast text missing (slow) | — | verified | **R2-20 recurrence** | none |
| diag | `TaskReceipt` must-use warnings ×6, then `let _ =` | diagnostics.md:27-110; main.rs:197 etc. | verified | **R2-4 recurrence** | none |
| code | `pending.wait()` (main.rs:632) | — | inferred from code | R2-28 | none |
| wishes | Toast text; import lines next to the library map; multi-document editor guide | — | — | R2-20; F3 pointer; N5 | — |
| (acc) | After clicking a Button row, arrows need Tab to reach the list | Plausible from the documented content-press rule | pending P7 | N4 cost | — |

### BX2 (baseline, met, 7.6 min)

| # | Event | Evidence | Status | Class | Disposition |
|---|---|---|---|---|---|
| 1 | `TextArea` has no on-change callback | `on_save` documents observing the binding instead | verified | documented, positive | none |
| 2 | `SplitPane` not in prelude | — | verified | **F3 recurrence** (symptom) | pointer |
| 3 | Guessed a `ComponentSnapshot` page | — | verified | tester error | none |
| 4 | `Semantics` not in the `testing` glob | diagnostics.md:83-96 | verified | **F3 recurrence** (baseline only) | none |
| 5 | `AmbiguousKey("Alpha.txt")` | Documented; the error names the key | verified | F23 passed | none |
| 6 | Toast text missing (slow) | — | verified | **R2-20 recurrence** | none |
| 7 | Reopening an already-selected row by pointer; Button + `set_active` + `Weak` (slow) | main.rs:50-63 | verified | documented contract; composition cost | N4 |
| diag/code | `TaskReceipt` warnings ×5 then `let _receipt`; `wait()` (main.rs:508) | — | verified | R2-4; R2-28 (inferred from code) | none |
| (acc) | Verifier errors: `TimeReversed` from `flush(ZERO)`, `AmbiguousKey("dialog")` across nested dialogs, a focused row button counted as the list | — | — | verifier errors; supports N3 and N4 | — |

**Event counts (template excluded; mixed entries split without adding events):** HC1 7, HC2 3, HX1 3, HX2 4, BC1 8, BC2 8, BX1 3, BX2 7.

## 2. Recurrence matrices

R = recurred, P = passed by actual exercise, — = not reached.

### F1–F34

| F | HC1 | HC2 | HX1 | HX2 | BC1 | BC2 | BX1 | BX2 |
|---|---|---|---|---|---|---|---|---|
| F1 | P | P | P | P | P | P | P | P |
| F2 | P | P | P | P | P | P | P | P |
| F3 | R | P | R | R | R | P | R | R |
| F4, F5, F6, F7a, F7b, F7c | — | — | — | — | — | — | — | — |
| F8 | P | P | P | P | P | P | P | P |
| F9 | — | — | — | — | — | — | — | — |
| F10 | P | P | P | P | P | P | P | P |
| F11, F12, F13 | — | — | — | — | — | — | — | — |
| F14 | P | P | P | P | P | P | P | P |
| F15–F19 | — | — | — | — | — | — | — | — |
| F20 | — | — | P | — | P | P | — | — |
| F21 | — | — | — | — | — | — | — | — |
| F22 | — | — | — | — | P | — | — | — |
| F23 | — | — | — | — | — | P | — | P |
| F24–F27 | — | — | — | — | — | — | — | — |
| F28 | P | P | P | — | — | P | P | — |
| F29–F33 | — | — | — | — | — | — | — | — |
| F34 | P | P | P | P | P | P | P | P |

**Reasons:**
- **F1 P ×8:** every `Cargo.toml:7` uses a registry requirement with `cpu` (HX2 and BX1 use `0.3`), and no run logged setup friction.
- **F2 P ×8:** all tests drive the app through `Driver`:
  - HC1 `tests/editor.rs:111-414`; HC2 `tests/app.rs:24-373`; HX1 `main.rs:463-655`; HX2 `tests/headless.rs:44-207`;
  - BC1 `notes.rs:120-176` (its own query helper is a convenience; `Driver::semantics` exists at B/testing.rs:377); BC2 `acceptance.rs:30-150`; BX1 `main.rs:412-505`; BX2 `main.rs:458, 476`.
  - HX1's triple settle compensates for its own double `defer`; it is not F2.
- **F3:**
  - R, symptom recurrence: E0433 for `SplitPane`/`SplitAxis`/`SplitSizing` with no rustc route, the same kind round 2 counted for L2-moved names such as `ThemeMode`. HC1 (diag:36-50), HX1 (:97-118), HX2 (:4-25), BX1 (:4-25), BX2 (:4-9).
  - R, `Semantics` absent from `testing` at 0.3.75 (fixed at head, HF/lib.rs:367): BC1 (diag:14-24, misleading "type parameter" hint) and BX2 (:83-96).
  - P: HC2 globbed `windowing::*` (lib.rs:4) and wrote `KeyChord`/`CommandScope`/`KeyCode` through the prelude. BC2 used named root split imports as layout.md:122 shows, with no failure.
  - Disposition: membership is intentional (prelude page, HF/lib.rs:121-133). Fix the pointers, not the prelude: getting-started-continued.md:25 mentions `SplitPane` beside `Tabs` with no import note, and layout.md:122 uses root named imports.
- **F8 P ×8:** each run reads a Label's text as its semantic name. HC1 editor.rs:56-63; HC2 app.rs:81-84; HX1 main.rs:414-419; HX2 headless.rs:16-21; BC1 notes.rs:79-84; BC2 acceptance.rs:70-74; BX1 main.rs:456-463; BX2 main.rs:476.
- **F9 — :** no `Each`. `ListItems` key replacement is a different item (existing TODO), not F9.
- **F10 P ×8:** every run captures with `CaptureTarget::window(&…primary_window())`. HC1 editor.rs:402; HC2 app.rs:334; HX1 main.rs:540; HX2 headless.rs:110; BC1 notes.rs:410; BC2 capture.rs:36; BX1 main.rs:627; BX2 main.rs:503.
- **F11 — ×8:** no failed conversion of F11's kind. HX1's E0308 suggested `?` correctly and would have read the same before the fix; HC1's `LayoutError` E0308 was accurate; BC2 assumed a failure without any diagnostic.
- **F13 — :** no background work; `defer` was only used for synchronous runtime operations.
- **F14 P ×8 (qualified: a selectable list of application records only, no editing or filtering):** HC1 lib.rs:107-192 (followed the guide's "editable list of records"); HC2 lib.rs:104-366; HX1 main.rs:16-35, 263; HX2 lib.rs:40-65, 298; BC1 lib.rs:226-361; BC2 lib.rs:248-318; BX1 main.rs:177-262; BX2 main.rs:39-65.
- **F16 — :** rows hold fixed captions, and the tab editors' per-document signals are the application's model, not per-row bindings of a `Vec`.
- **F19 — :** every `CommandId` is kept for `MenuItem::command`; none is dropped.
- **F20 P where a run adopted an item-page example** (rendered with `use pert_ui::…`): HX1 used the `KeyChord::new` example (H/runtime/commands.rs:41-66 → main.rs:6-14); BC1 and BC2 used the `Tabs::reorderable` example (B/tabs.rs:507-535; FRICTION 16:54 and 17:38). Reading pages alone is not counted.
- **F22 P BC1:** its closing notes say the TextField docs on which chords an editor claims let it predict that Ctrl+S/N/W reach the commands, and the behaviour was confirmed in tests (notes.rs:237-244). Elsewhere, Ctrl chords worked the same before the fix, so not reached.
- **F23 P:** BC2 `AmbiguousKey("items")` and BX2 `AmbiguousKey("Alpha.txt")` both named the key.
- **F27 — :** no `InlineText`. The Button-row press rule is a different widget family; see N4.
- **F28 P where `Signal::update` built cleanly:** HC1 lib.rs:466; HC2 :257; HX1 :96, 146, 200 (its manual `PartialEq for Note` at :47-51 compiled first time); BC2 :238; BX1 :159. HX2, BC1 and BX2 only call `set`.
- **F29 — :** empty Labels appear but their height is never measured.
- **F34 P ×8:** chords go through `Driver::chord`. HC1 editor.rs:188; HC2 app.rs:125; HX1 main.rs:521; HX2 headless.rs:41; BC1 notes.rs:168; BC2 acceptance.rs:141; BX1 main.rs:448; BX2 main.rs:458.

### R2-1–R2-28

| R2 | HC1 | HC2 | HX1 | HX2 | BC1 | BC2 | BX1 | BX2 |
|---|---|---|---|---|---|---|---|---|
| R2-1 | P | P | P | P | R | R* | — | — |
| R2-2, R2-3 | — | — | — | — | — | — | — | — |
| R2-4 | P | P | P | P | R* | P | R | R |
| R2-5 | — | — | — | — | — | — | — | — |
| R2-6 | P | P | P | P | R | P | P | P |
| R2-7, R2-8, R2-9 | — | — | — | — | — | — | — | — |
| R2-10 | — | — | P | P | — | — | — | — |
| R2-11 | P | P | — | — | — | — | — | — |
| R2-12 | P | P | P | P | P | P | P | P |
| R2-13 | P | P | P | — | P | P | P | — |
| R2-14–R2-19 | — | — | — | — | — | — | — | — |
| R2-20 | P | P | — | — | R | R | R | R |
| R2-21–R2-27 | — | — | — | — | — | — | — | — |
| R2-28 | P | P | P | P | R* | R* | R* | R* |

\* = inferred from code.

**Reasons:**
- **R2-1:**
  - P: head runs get state back through `App<S>`. HC1 lib.rs:139 and editor.rs:115; HC2 lib.rs:337 and app.rs:28; HX1 main.rs:374 and :474 (its internal `root` slot is callback plumbing, not a test handoff); HX2 lib.rs:241 and headless.rs:55.
  - R: BC1 (friction 17:02, lib.rs:264-273); BC2 (startup.rs:17, inferred from code).
  - — : BX1 and BX2 mount `build()` on a `Runtime` and never test the initializer.
- **R2-3 — ×8:** every derivation has two inputs; the n-ary case never comes up.
- **R2-4:**
  - P: `cx.defer(..)?;` with no warning. HC1 lib.rs:126; HC2 :123; HX1 :110; HX2 :57. BC2 discards with `.map(|_| ())` (lib.rs:266) without a warning.
  - R: BC1 binds `let _receipt` five times (lib.rs:218, 243, 359, 440, 455; inferred from code); BX1 (6 warnings, then `let _ =`); BX2 (5 warnings, then `let _receipt`, main.rs:58).
- **R2-6:**
  - P: head runs use `address_children` (HC1 editor.rs:41; HC2 app.rs:34, 38; HX1 :479; HX2 :60).
  - P, qualified: BC2, BX1 and BX2 relied on `children` order, undocumented at 0.3.75, without friction (BC2 acceptance.rs:54-59; BX1 :511-516; BX2 :526).
  - R: BC1 (friction 17:03; sorted rows by y, notes.rs:52-63).
- **R2-10 P:** HX1 resolved its `ctrl` guess from the documented example (friction 16:50); HX2 wrote `KeyChord::new` with Control without friction (lib.rs:31-39).
- **R2-11 P:** `Driver::clear_focus` in HC1 editor.rs:398 and HC2 app.rs:330.
- **R2-12 P ×8:** every run calls `encode_png` (HC1 :411; HC2 :339; HX1 :546; HX2 :118; BC1 :416; BC2 capture.rs:42; BX1 :633; BX2 :509). The baseline chapter lacked the step but no run hit friction.
- **R2-13 P:** each reads an empty label's name as absent or empty without friction. HC1 editor.rs:244, 279; HC2 app.rs:110, 214; HX1 :485, 653; BC1 notes.rs:187; BC2 acceptance.rs:156, 280; BX1 :613.
- **R2-20:**
  - P: HC1 and HC2 read toast items (editor.rs:65-69, 208-210; app.rs:87-90, 144-146).
  - — : HX1 and HX2 show the failure in a plain Label.
  - R: all four baseline runs logged it as friction.
- **R2-27 — :** no table. Menu and tab items are a different widget family, recorded as N2.
- **R2-28:**
  - P: head runs use `try_reply` (HC1 :407; HC2 :337; HX1 :545; HX2 :115).
  - R, inferred from code: baseline runs call `wait()` on the UI thread (BC1 notes.rs:413; BC2 capture.rs:40; BX1 :632; BX2 :508), following BF/guide/testing.md:376 although B/inspection_service.rs:584-585 says never to.

**Totals:**
- **Head:** only F3 recurs, 3 occurrences, all the `SplitPane` symptom. No R2 finding recurs.
- **Baseline:** F3 ×3, R2-1 ×2, R2-4 ×3, R2-6 ×1, R2-20 ×4, R2-28 ×4 (code-inferred), 17 occurrences in all. Every baseline-only R2 recurrence is repaired at head; this is descriptive, and the post-run repairs don't change the comparison.

## 3. New findings (deduplicated)

- **N1. `click_item` docs name examples that can't be clicked** (documentation clarity; head only; slow).
  - Evidence: H/testing.rs:830-838 lists "a menu command or a tab title" as items, then restricts clicking to table titles and toasts. Hit by HC1 #4 and #6, and HX2 #3.
  - Repair: say which built-in components provide rectangles and point to the keyboard routes (F10/arrows; Tabs Home/End or `reveal_child`). Overlap: once round-3-design-items is implemented the examples become true.
- **N2. Menu rows and tab titles have no item rectangles, and MenuBar titles can't be reached with `find`** (missing capability; both commits; slow).
  - Evidence: P3 and P5; H/tabs.rs:1843-1846; the menu `OptionList` builds items without `local_bounds`. Hit by HC1, HX2 and BC2, and every one of the eight verifiers built geometry or hover-scan helpers **(acc)**.
  - Repair: fill `local_bounds` from the same geometry painting and hit-testing use, under the staged-layout contract the table headers follow, and add pointer tests for File › Save and a tab. Keep workspace paths and semantic names independent.
  - Overlap: TODO item-reporters entry (it no longer names tabs or menus, which have moved into round-3-design-items.md), round-3-design-parts.md. This is not R2-27.
- **N3. Tabs, MenuBar and Dialog/AlertDialog have no "Inspection and tests" section** (documentation gap; both commits; slow).
  - What's missing: the TabList role, items and selection; structural `title/<key>` titles and rows as items under `…/items`; that the Dialog owner component is always present, with `expanded` and the `open` property showing whether the dialog is shown (H/dialog.rs:408-425); content at `<owner>/dialog/<content>`; the anchor adds no segment.
  - Hit by HC1 #3 and #5, HC2 #3 and wish 1, BC2 #4. Supported by the HC1 verifier's open-dialog mistake and BX2's verifier `AmbiguousKey("dialog")` **(acc)**. Backed by P4/P5.
  - Repair: add sections modelled on ToastHost, a table of structural and part segments in the testing chapter, and a link from `click_item`. Overlap: the AlertDialog confirmation-button TODO.
- **N4. No documented pattern for opening list rows with the pointer** (documentation gap plus an optional capability; both commits; slow at worst, BX2).
  - Enter-only activation is documented, and all eight runs composed Button rows by code. The cost is documented but scattered: a press handled by the row's content does not select (H/virtual_list.rs:449-451), so the selection doesn't follow (BC1 acc), BX2 had to call `set_active` manually and hold a `Weak`, and focus continuity is lost (BX1 and BX2 acc; pending P7).
  - Repair: a "rows that open on click" recipe covering selection, focus and keys. A pointer-activation option (double click or activate-on-click) needs a design checkpoint. Don't widen `on_activate` silently. Not a defect.
- **N5. No recipe for tabs built from application data, and `reveal_child`'s settle order is unclear** (documentation gap and precision; both commits; annoyance; friction or wishes in all eight runs).
  - Missing: open (`reconcile_children` with `Tab::into_element`), select (`reveal_child`), close, where focus goes after the focused tab is removed, and the `Dynamic` alternative with retained selection.
  - `reveal_child` doesn't say it flushes pending writes before resolving the key (H/runtime/reveal.rs:426-428, 459-464). HC1 inferred a limitation from this; HX1 wrote a double `defer` and a triple settle. Both are contradicted by P2 and the source.
  - Also: `register_command`/`invoke_command` don't say that an action may call operations that flush (BC2 #7). Contrast `defer` (H/runtime/tasks.rs:257-262).
  - Don't add a bound-Tabs API without a checkpoint; `Dynamic` and `reconcile_children` already work. Overlap: the author's "focused tab lifecycle recipe".
- **N6. Opening a menu by keyboard picks up a stale pointer highlight** (behaviour plus a docs gap; both commits; slow, seen at baseline).
  - Mechanism: `refresh_hover` sends a synthetic Move (pointer.rs:70-81) and `OptionList` moves its highlight on any Move (select.rs:547-551), so the guide's F10/Down/Enter recipe (HF/guide/surfaces.md:160-166) depends on where the previous `Driver::click` left the pointer. Evidence: BC1 #4 and the BC2 verifier capture **(acc)**.
  - Repair: ignore `refreshed()` moves for the keyboard highlight, or document it in `Driver` and recommend a `PointerKind::Leave`; add a regression test. Pending P8.
- **N7. A literal tab in `TextArea` draws a missing-glyph box** (rendering defect; head observed; baseline pending; acceptance-added, no tester severity).
  - Multiline editors accept and keep U+0009 (H/text_fields.rs:2024-2040, 3690-3699). This is a separate lead, not a failed notes requirement. Repair is as round-3-design-tabs-whitespace.md revision 3 (non-printing tab, still awaiting its recheck). Pending P10.
- **N8. `NotRevealed` diagnostics mislead after teardown** (diagnostic-quality lead; both commits; BC2 #5, mixed with the tester's own bug).
  - `AncestorRefused` carries only a `WidgetId`, and `Debug` prints live `mounted` state (H/runtime.rs:143-149), which plausibly reads `false` once the test's runtime has been dropped.
  - Repair: name the ancestor's key in the refusal. Pending P9.

**Optional suggestions, not findings:** a Control `KeyChord` constructor (`primary` is the documented route); an infallible `App::size` or integer `Length`; a subtree-address helper; a drag helper and an inspectable divider rectangle; a compact import and stretch reference; a signature index (kit reading aid).

**Positive notes:** the testing and surfaces chapters (HC1, HC2, BC2); `MenuItem::command` making shortcut and menu share one action; `NotFound { resolved }`; ToastHost's inspection section; `TextArea::on_save` docs (BX2).

**Kit issue:** HC2 #1, reading very long pages through pandoc.

**Changes to the initial head triage (initial-head-task6.md):**
- Its H3 and H7 claims are now contradicted by execution.
- N4 stays a documentation and optional-capability item, now with recurrence in all eight runs.
- N5 becomes a verified documentation-precision finding, with two independent head misreadings.
- Its TODO citation for tab titles doesn't match the current TODO.

## Author probes proposed

| Probe | Check | Kits |
|---|---|---|
| P7 | After `Driver::click` on a Button row (selection set, `item_tab_stops(false)`): what is focused, the list's active item and selection, and whether Down/Enter reach the list | both |
| P8 | Click at a point the second menu row will cover, then F10 and Down: which row is highlighted; then the same after a `PointerKind::Leave` | both |
| P9 | A test returning `Err(NotRevealed)`: print its `Debug` before and after the runtime is dropped | both |
| P10 | CPU capture of a `TextArea` containing `"A\tB"` | baseline, plus a frozen-head rerun of the glyph probe |
| P11 | A `Dynamic` that adds and removes tabs without a reveal: selection retention, `on_select` origin, and the focus target after the focused tab is removed | both |
