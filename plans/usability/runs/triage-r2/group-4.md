# Triage verification, group 4: task 5 (job monitor) on the round 1 library

Runs: r2-5-r1-claude-1, r2-5-r1-claude-2, r2-5-r1-codex-1, r2-5-r1-codex-2.
All four ran `plans/usability/reserved/5-job-monitor.md` against commit
359122e (0.3.52), kit `~/pert-ui-usability/kit-359122e-r2`. Every claim is
checked against that commit's vendored crates (paths below without a prefix
are under `~/pert-ui-usability/kit-359122e-r2/vendor/`, for example
`pert-ui-runtime-0.3.52/src/table.rs:233`). For each new finding, the state at
the branch head aca8df3 (0.3.75) is checked in
`~/pert-ui-usability/kit-aca8df3/vendor/pert-ui*-0.3.75`.

Runtime claims were reproduced with a throwaway probe,
`~/pert-ui-usability/triage-r2/probes/g4-r1/work/src/main.rs` (pert-ui 0.3.52
without default features, built offline in the kit sandbox with `run.sh exec`;
its `target/` was deleted afterwards). With a 230 px table column holding
`Row[ProgressBar.with_grow(1), Label("42%")]`, it printed:

```text
[pert-ui validation ERROR EscapesParent] Component "percent" (#11): visible bounds leave its parent "progress" without a clip; ...
cell: Some(Rect([0.0, 32.0, 230.0, 32.0]))
row: Some(Rect([12.0, 38.0, 206.0, 20.0]))
bar: Some(Rect([12.0, 45.0, 192.0, 6.0]))
percent: Some(Rect([212.0, 38.0, 28.0, 20.0]))
find_descendant(row, state) = Err(AmbiguousKey)
toast-ish: path=Some(["toasts", "toasts"]) sem=Semantics { role: Notifications, name: None, description: None, value: Some(Number { value: 1.0, .. }), actions: [Dismiss], items: [], .. } props={"anchor_owner": .., "dismiss_label": Text("Dismiss"), "paused": Bool(false), "presentation_layer": Text("Notification"), "scroll_offset": .., "visible_count": Number(1.0)}
```

The friction entries are numbered by their `### HH:MM` headings in file order
(the template's `### <time>` block is not counted). Acceptance leads are
`#acc-n`; the name-sorting lead is `r2-5-r1-codex-1#acc-1` and
`r2-5-r1-codex-2#acc-2`.

## Entries

| id | verified | class | item or page | severity (tester) | suggested direction | destination if not fixed |
|---|---|---|---|---|---|---|
| r2-5-r1-claude-1#1 | yes. `pert-ui-0.3.52/src/guide/getting-started.md:9-15` says "Until a registry release exists, use a local checkout" with a `path` dependency; `getting-started-continued.md:5-6` gives `cargo run --locked -p pert-ui --example …`. The examples the guide names are GitHub links (`guide/state.md:313`, `:324`). Partly false: "only links … not shipped": the package includes them (`pert-ui-0.3.52/Cargo.toml` `include = ["/examples/**", …]`, `pert-ui-0.3.52/examples/live_mixer.rs`), but the docs never say so and the tester did not read the vendor tree. | recurrence of F1 | guide/getting-started.md "Run your first window" | annoyance | Fixed at head by 5c: `getting-started.md:8-21` (0.3.75) leads with `pert-ui = { version = "0.3" }` and keeps the checkout as the alternative. | addressed at head (5c) |
| r2-5-r1-claude-1#2 | yes. `ProgressBar::new(status: impl Into<ReadSignal<ProgressStatus>>)` (`pert-ui-runtime-0.3.52/src/progress.rs:152`); `ReadSignal` converts only from `Signal<T>`, `&Signal<T>` and `&ReadSignal<T>` (`signal.rs:238-253`), so a fixed value needs a signal. The tester kept one `Signal<ProgressStatus>` per job made with `cx.signal` (`work/src/monitor.rs:394`). Still present at head: `progress.rs:157-158` (0.3.75) is unchanged; 6f3's controlled forms cover `Checkbox` and `InlineText` only. | recurrence of F16 (in a display control: rows rebuilt from a data snapshot need a signal per row for a bar) | `ProgressBar::new`, `ReadSignal` | annoyance | Accept a fixed `ProgressStatus` (for example `ProgressBar::fixed(status)` or `From<T> for ReadSignal<T>` as a constant source), and extend F16's per-row state recipe to display controls in table cells. | FU (F16's controlled controls), S9 (per-row recipe) |
| r2-5-r1-claude-1#3 | yes, by probe (output above): in a 230 px column the cell's row is 206 px, the bar keeps its 192 px preference and the 28 px label ends at 240, past the cell; validation reports `EscapesParent`. Causes as stated: `ProgressBar` measures `min(available, 3 × ControlWidth)` (`progress.rs:97`, `:214-219`) and `Element::with_grow` says "Rows never shrink a child" (`layout.rs:561`), while a bounded `Column` gives a growing child its share of a shortfall (`layout.rs:556-561`). Workaround: a `Container` with a fixed preferred size (`work/src/monitor.rs:216-218`). Still present at head (`layout.rs:611`, `progress.rs:220-224` in 0.3.75). | new finding: API ergonomics (layout) | `Row`, `Element::with_grow`, `ProgressBar` | annoyance | Let a growing `Row` child take its share of a shortfall down to its minimum, as a bounded `Column` already does; at least say on `ProgressBar` that a bar beside a label in a narrow cell needs a bounded width. | TODO |
| r2-5-r1-claude-1#4 | yes. `Rect` is a tuple struct `Rect([f32; 4])` with accessor methods (`pert-ui-runtime-0.3.52/src/drawing.rs:53-54`); rustc's E0615 suggested `x()` and `width()` (run `diagnostics.md`), which worked. | tester error (guess; the diagnostic led straight to the fix) | `Rect` | annoyance | none | none |
| r2-5-r1-claude-1#5 | yes. `TableItems` says only "the element's own key is free, since the table wraps each cell" (`table.rs:233`); the wrapper is `Element::new(column.key, TableCell { … })` (`table.rs:504-511`), so a cell's address is `table/row-key/column-key/content-key`, which no page states. The probe found the row at `["content", "jobs", "job-1"]` and the cell under the column key. Still present at head (`table.rs:239`, `:531-533` in 0.3.75; `guide/testing.md` has no table section). | new finding: documentation gap | `Table`, `TableItems::cell` | annoyance (positive note) | State the cell address on `TableItems::cell` and `Table` (row key, then column key, then the content's key), with a `Runtime::find` line in the example. | TODO |
| r2-5-r1-claude-1#6 | yes. `ToastStack::inspect` (`notifications.rs:797-826`) reports role `Notifications`, the visible count as value, `Dismiss`, and `visible_count`, `scroll_offset`, `paused`, `dismiss_label`; no message text and no items. Probe output above. `ToastQueue` has only `new` and `push`, `ToastReceipt` only `status` and `status_binding` (`notifications.rs:196-275`), so nothing reads a message back. Still present at head: `notifications.rs:808-835` (0.3.75) is unchanged. | new finding: missing capability | `ToastHost` inspection | slow | Report each visible toast as a `SemanticItem` (message as name, its action labels) or as a child, so inspection and content search see what the stack shows. | FU; TODO |
| r2-5-r1-claude-2#1 | yes, as r2-5-r1-claude-1#1. The registry form it saw is real: `guide/diagnostics.md:174` and `guide/appearance.md:504` use `pert-ui = { version = "…", features = […] }`. | recurrence of F1 | same | annoyance | as claude-1#1 | addressed at head (5c) |
| r2-5-r1-claude-2#2 | yes, as r2-5-r1-claude-1#3 (probe). The quoted message and the 192 px figure (3 × 64) are exact. | new finding: API ergonomics (layout), as claude-1#3 | `Row`, `ProgressBar`, `Table` | annoyance | as claude-1#3 | TODO |
| r2-5-r1-claude-2#3 | yes, by probe: content keyed `state` in the `state` column makes `find_descendant(&row, "state")` return `AmbiguousKey`, because the table's wrapper carries the column key (`table.rs:504-511`) and `find_descendant` refuses several matches (`runtime/tree.rs:202-209`, which names "a container keyed like the control it holds" as a common cause). "The element's own key is free" (`table.rs:233`) holds for sibling uniqueness but hides this. Still present at head (`table.rs:239`, `:531-533`); 6c1's `AmbiguousKey(String)` (`runtime.rs:337`) would now name the key, which the tester already knew. | new finding: documentation gap, as claude-1#5 | `TableItems::cell`, `Runtime::find_descendant` | annoyance | as claude-1#5, adding that content keyed like its column collides with the wrapper in `find_descendant`. | TODO |
| r2-5-r1-claude-2#4 | yes, as r2-5-r1-claude-1#6. Its further claim that `DisplayList` commands carry no text holds: `DrawCommand` has only `DrawVector`, `FillRect`, `DrawMask` and `DrawImage` (`pert-ui-runtime-0.3.52/src/drawing.rs:1026-1088`), so text arrives as glyph masks, not strings (Codex review). | new finding: missing capability, as claude-1#6 | `ToastHost` inspection | slow | as claude-1#6 | FU; TODO |
| r2-5-r1-claude-2#5 | yes. `ProgressStatus::Failed` and `Cancelled` are documented as a full track (`progress.rs:31-38`) and drawn full width (`progress.rs:184-185`); no terminal variant keeps a fraction. Documented behavior, not a defect. Still present at head (`progress.rs:31-38`, `:190-191` in 0.3.75). | new finding: missing capability | `ProgressStatus` | annoyance | Optional: terminal statuses that keep the fraction reached (or a `ProgressBar` option to keep the last fill in the terminal role). | FU |
| r2-5-r1-claude-2#6 | yes. The header is "drawn by the table" (`table.rs:1118`); titles are `SemanticItem`s on the table component with `Select` actions but no bounds (`table.rs:2824-2858`), so a test cannot address a title; the inspection property `column_widths` (`table.rs:2811-2823`) is the only geometry. The keyboard route (Alt+Up/Down on the active column, `table.rs:1157-1163`) is documented. claude-1 also clicked titles at positions computed from its own column widths (`work/src/headless.rs:132-143`); both codex runs used the keyboard route. Still present at head: the `testing` driver's `click` takes a `WidgetId` (`testing.rs:424`); its `chord` (`:615`) eases the keyboard route. | new finding: API ergonomics (headless testing) | `Table` header, testing | annoyance | Say in the Table docs (and at head in guide/testing.md) how a test sorts: `set_active_column` plus Alt+Up/Down, or a title's position from `column_widths`; optionally give header items bounds. | S9 (testing chapter), TODO |
| r2-5-r1-claude-2#7 | yes for the contradiction: `Pending::wait` says "Never call this on the UI thread: the reply comes from that thread's next flush" (`inspection.rs:582-583`), while `guide/diagnostics.md:114` and `:159`, the `cpu` module example (`pert-ui-0.3.52/src/cpu.rs:35`) and `CpuRenderer::serve_captures` (`pert-ui-cpu-0.3.52/src/lib.rs:246`) call it on the UI thread after the reply has arrived. The `Unsupported` part is the tester's error, as it says: `CaptureError::Unsupported` documents the missing `set_capture_support` (`capture.rs:165-167`), though its message ("this host does not render captures") does not name the call. Still present at head (`inspection_service.rs:584-585`; `guide/diagnostics.md:170`, `:178`; `guide/testing.md:376` in 0.3.75). | new finding: documentation error (the `Unsupported` part is a tester error) | `Pending::wait`, guide/diagnostics.md "Screenshots" | annoyance | Narrow the rule: waiting on the UI thread blocks forever unless the reply already arrived (after the flush that serves it); or switch the examples to `try_reply`. | TODO |
| r2-5-r1-codex-1#1 | yes, as r2-5-r1-claude-1#1. | recurrence of F1 | same | annoyance | as claude-1#1 | addressed at head (5c) |
| r2-5-r1-codex-1#2 | yes. `CaptureTarget::window(window: &NativeWindowId)` (`capture.rs:90`) gave E0308 with the borrow hint (run `diagnostics.md`), and the 0.3.52 guide shows no call: F10 (fixed at head by 5c, `guide/diagnostics.md:139-141`, 0.3.75). The other three are guesses: `ListRevealStatus` has `Pending`, `Mounted(WidgetId)`, `NotFound`, … (`virtual_list.rs:200-208`); `Modifiers` has only `PRIMARY` (`input.rs:310`) and public fields, as round 1's r1-2-codex#2 found (still only `PRIMARY` at head, `input.rs:353`); `clear_focus` returns `()`. | recurrence of F10 (the window borrow); the rest tester error | `CaptureTarget::window`; `ListRevealStatus`, `Modifiers` | annoyance | F10: done at head. Optional, as round 1 suggested: `Modifiers::SHIFT/CONTROL/ALT/SUPER` constants. | addressed at head (F10); FU (constants) |
| r2-5-r1-codex-2#1 | yes: the getting-started part as r2-5-r1-claude-1#1. `struct.TableRow.html` does not exist; the `Table` example explains `TableItems`/`TableSource`, as the tester then found. | recurrence of F1; the `TableRow` guess is a tester error | guide/getting-started.md; `Table` | annoyance | as claude-1#1 | addressed at head (5c) |
| r2-5-r1-codex-2#2 | yes as reported: the tool environment refused `rm -f`; the tester says it is not a pert-ui problem. | kit issue (tester tooling) | none | annoyance | none | none |
| r2-5-r1-claude-1#acc-1 | yes, as r2-5-r1-claude-1#6 (`notifications.rs:797-826`, probe). All four testers that used toasts kept their own messages: claude-1 `work/src/monitor.rs:119-126`, `:362-373`; claude-2 `work/src/lib.rs:311`, `:543-546`. codex-1 used a `Banner`, whose message inspection does report (role `Alert` named with the message, its acceptance check); codex-2 used no notification surface. | new finding: missing capability, as claude-1#6 | `ToastHost` inspection | — | as claude-1#6 | FU; TODO |
| r2-5-r1-claude-1#acc-2 | yes, by probe: without the wrapper the label escapes the cell (output above). The comment's reasons are exact (`progress.rs:97`, `:214-219`; `layout.rs:561`). | new finding: API ergonomics (layout), as claude-1#3 | `Row`, `ProgressBar` | — | as claude-1#3 | TODO |
| r2-5-r1-claude-1#acc-3 | yes. `Runtime::paint` plus `CpuRenderer::render` (`work/src/headless.rs:197-203`) is a documented route: the `cpu` module says the renderer turns a `DisplayList` from `Runtime::paint` into a `FrameImage` (`pert-ui-0.3.52/src/cpu.rs:1-5`), and `CpuRenderer::render` is "what a capture keeps" (`pert-ui-cpu-0.3.52/src/lib.rs:135-152`). | note without problem | `cpu::CpuRenderer` | — | none | none |
| r2-5-r1-codex-2#acc-1 | partly. The "notification" is a `Label` bound to a text signal (`work/src/main.rs:156`, `:221-222`, `:361`, `:387`), while its README speaks of "a persistent notification banner". Its friction log has no entry on notifications and its closing lists no notification concept, so there is no evidence that it looked for the facility (transcripts not read). The facility is findable: the crate root map has a "Menus, dialogs and notifications" row naming `ToastHost` (`pert-ui-0.3.52/src/root.md:65`), guide/surfaces.md has a "Notifications" section on `Banner` and `ToastHost` (`:283-303`), and both are in the prelude; codex-1 found `Banner` in the same kit. Under the task's literal wording ("the failure is shown in its row and in a notification", `plans/usability/reserved/5-job-monitor.md:15`), which names no component, a visible failure message outside the row meets it; the acceptance file itself leaves this to the author (`r2-5-r1-codex-2/acceptance.md:48-51`). (Codex review) | acceptance-policy ambiguity, pending the maintainer's decision whether a notification must be a library surface (`Banner` or `Toast`); a tester error only under that stricter reading (Codex review) | none | — | Decide the criterion and apply it equally to every task 5 run that shows the failure in a plain label. | maintainer decision |
| r2-5-r1-codex-1#acc-1 | yes. Name order compares the text "Job N" (`work/src/main.rs:117`). The library does not sort: "The order of rows is the application's: the table never sorts them" (`table.rs:235`), and `Table::on_sort` leaves sorting to the application (`table.rs:1526-1528`); nothing compares text for it. | note without problem (application choice) | none | — | none | none |
| r2-5-r1-codex-2#acc-2 | yes, as r2-5-r1-codex-1#acc-1 (`work/src/main.rs:160`). | note without problem (application choice) | none | — | none | none |

## Recurrence matrix

Rule (author decision, applied after the Codex review): `recurred` needs
evidence of the problem or its workaround in the friction log, code,
diagnostics or acceptance; reading an affected page without visible effect is
not recurrence. `passed` needs evidence that the run exercised the area. A
`recurred` cell supported only by code, with no friction entry, is marked
"inferred from code". (Codex review)

Columns: c1 = r2-5-r1-claude-1, c2 = r2-5-r1-claude-2, x1 = r2-5-r1-codex-1,
x2 = r2-5-r1-codex-2. Code paths are under each run's `work/`.

| F | c1 | c2 | x1 | x2 |
|---|---|---|---|---|
| F1 | recurred | recurred | recurred | recurred |
| F2 | recurred | recurred | recurred | recurred |
| F3 | not reached | not reached | recurred (inferred from code) | recurred (inferred from code) |
| F4 | not reached | not reached | not reached | not reached |
| F5 | not reached | not reached | not reached | not reached |
| F6 | not reached | not reached | not reached | not reached |
| F7a | not reached | not reached | not reached | not reached |
| F7b | not reached | not reached | not reached | not reached |
| F7c | not reached | not reached | not reached | not reached |
| F8 | passed | passed | passed | passed |
| F9 | not reached | not reached | not reached | not reached |
| F10 | not reached | passed | recurred | not reached |
| F11 | not reached | not reached | not reached | not reached |
| F12 | not reached | not reached | not reached | not reached |
| F13 | passed | passed | passed | passed |
| F14 | not reached | not reached | not reached | not reached |
| F15 | not reached | not reached | not reached | not reached |
| F16 | recurred | recurred | recurred | recurred |
| F17 | not reached | not reached | not reached | not reached |
| F18 | not reached | not reached | not reached | not reached |
| F19 | not reached | not reached | not reached | not reached |
| F20 | passed | passed | passed | passed |
| F21 | not reached | not reached | not reached | not reached |
| F22 | not reached | not reached | not reached | not reached |
| F23 | not reached | recurred | not reached | not reached |
| F24 | not reached | not reached | not reached | not reached |
| F25 | not reached | not reached | not reached | not reached |
| F26 | not reached | not reached | not reached | not reached |
| F27 | not reached | not reached | not reached | not reached |
| F28 | not reached | not reached | not reached | not reached |
| F29 | not reached | not reached | not reached | not reached |
| F30 | not reached | not reached | not reached | not reached |
| F31 | not reached | not reached | not reached | not reached |
| F32 | not reached | not reached | not reached | not reached |
| F33 | not reached | not reached | not reached | not reached |
| F34 | not reached | not reached | passed | passed |

Reasons for the `recurred` and `passed` cells:

- **F1, all four:** each logged the checkout-only dependency instruction
  (c1#1, c2#1, x1#1, x2#1).
- **F2, c1:** built its own driver, `src/headless.rs`: click at a widget's
  bounds (`:107-112`), header click by computed position (`:132-143`), text
  through `inspection_id` and a full snapshot search (`:146-161`), and
  wait-until loops.
- **F2, c2:** test helpers for clicks at bounds, cell text from a full
  snapshot (`tests/monitor.rs:190`, `:335-336`) and stepped settling.
- **F2, x1:** `wait_for`, `visible_text` over the whole snapshot and a
  recursive `row_names` walk with `inspection_id` per child
  (`src/main.rs:376-420`).
- **F2, x2:** `press`, `wait_for` and a `visible_text` dump of every
  component (`src/main.rs:406-431`); its closing asks for "a compact
  headless interaction recipe".
- **F3, x1 (inferred from code, no friction entry; Codex review):**
  `Modifiers` reached through the root glob `use pert_ui::*`
  (`src/main.rs:499-505`); it is not in the 0.3.52 prelude.
- **F3, x2 (inferred from code, no friction entry; Codex review):**
  `use pert_ui::{…, KeyAction, KeyCode, Modifiers}`
  (`src/main.rs:433`), `Modifiers` from the root (KeyAction and KeyCode
  redundantly; they are in the prelude).
- **F8, all four:** read Label text as `Semantics::name` with no friction:
  c1 `src/headless.rs:146-161`, c2 `tests/monitor.rs:190`, x1 `visible_text`
  and `row_names` (names), x2 `visible_text` (name, value and description
  printed together).
- **F10, c2:** `CaptureTarget::window(&s.ui.primary_window())`
  (`tests/monitor.rs:348`, `:452`, `src/main.rs:70`) with no E0308 in its
  diagnostics.
- **F10, x1:** E0308 "expected `&NativeWindowId`, found `NativeWindowId`"
  at `src/main.rs:174` (x1#2).
- **F13, all four:** started background work from the Start button's callback
  with a mailbox `UiSender`, without friction: c1 `src/monitor.rs:380-406`;
  c2 `src/lib.rs:318-345`; x1 `src/main.rs:195-283` (called from the button
  at `:339`); x2 `src/main.rs:299-328`. All used `std::thread`, which the task
  asks for; none tried `Runtime::spawn`. 0.3.52 has no `EventContext::spawn`,
  so these cells say nothing about the head's F13 fix. (Codex review)
- **F16, c1:** entry c1#2, and one `Signal<ProgressStatus>` per job made in
  the Start callback (`src/monitor.rs:394`).
- **F16, c2:** every row cell bound to per-job signals created as siblings
  of a placeholder `Signal<()>` (`src/lib.rs:314`, `:323-337`, `:585`).
- **F16, x1:** one `ui.signal(ProgressStatus…)` per job (`src/main.rs:221`)
  read by the cell (`:74`).
- **F16, x2:** a new `store.summary.sibling(ProgressStatus…)` on every cell
  build (`src/main.rs:246`).
- **F20, all four:** read item pages whose examples import `pert_ui_runtime`
  (rendered `struct.Table.html` and `struct.ProgressBar.html` at 0.3.52 do)
  and logged no friction; no submission imports `pert_ui_runtime` or
  `pert_ui_core`. c1 and c2 read `struct.ProgressBar.html`, x1 and x2
  `struct.Table.html` (their entries' Read lines); x2 notes that the Table
  examples compile with the facade imports. Kept `passed` under the matrix
  rule: reading the pages left no visible effect in code or diagnostics.
  Codex's proposal to count exposure as recurrence is rejected for that
  reason (Codex review).
- **F23, c2 (Codex review, was `passed`):** the run hit the unnamed error:
  its friction entry c2#3 records `Error: AmbiguousKey`, and at 0.3.52 the
  variant is the unit `AmbiguousKey` with the message "several descendants
  share the key" (`pert-ui-runtime-0.3.52/src/runtime.rs:319-320`). The
  call named the key, so the missing name cost little; the main friction was
  the cell wrapper (new finding N2).
- **F34, x1 and x2:** pressed Alt for keyboard sorting with
  `Runtime::native_modifiers`, the guide's route (x1 `src/main.rs:499-505`,
  x2 `src/main.rs:537-543`); x1's friction was a guessed `Modifiers::ALT`
  constant, not the two recipes.

Notes on `not reached` cells:
- F11 (author decision after check; was `passed`): F11 is the misleading
  diagnostic when a conversion fails. Each run only converted successfully
  (`?` on `RuntimeError` into `AppError` in the `App::new` closure, `paint()?`
  in boxed-error helpers: c1 `src/main.rs:19-27`, c2 `src/main.rs:12-16`,
  x1 `src/main.rs:360-363`, x2 `src/main.rs:397-399`), which worked the same
  at 0.3.52, and no diagnostics file shows a failed conversion.
- F14: the runs show records in a table without editing or filtering; x1 and
  x2 ask for a "sortable worker-progress table" or "background-job" example,
  which is close to F14's recipe request but outside its list scope.
- F19: no run registers a command. c2's probe got an `unused_must_use`
  warning on `ToastReceipt` (`diagnostics.md`), the same pattern of a
  `#[must_use]` handle (`notifications.rs:195`); at head `ToastReceipt` is no
  longer `#[must_use]` (`notifications.rs:199-200`, 0.3.75), from 6b1's G2-8.
- F29: x2 puts `Label::new("")` in the action cell of finished rows
  (`src/main.rs:276`), but the fixed row height hides the empty label's
  height.
- F33: c1 builds a constant size with `Size::new(…).expect(…)`
  (`src/monitor.rs:217`), the same shape as F33, but for `Size`, not
  `SizeLimits` in a custom control.

## New findings summary

Severity is the worst a tester of this group gave; the runs' own severities
are in the entries table (all of N1's reports here are annoyance). Other
groups' severities for the same issue are not merged here. (Codex review)

| # | Title | Class | Severity | Runs | Evidence | At head (aca8df3) |
|---|---|---|---|---|---|---|
| N1 | A growing child in a `Row` never shrinks, so a `ProgressBar` (192 px preferred) plus its percentage overflows a table cell | API ergonomics (layout) | annoyance | c1 (#3, acc-2), c2 (#2) | Probe: row 206 px, bar 192, label to 240; `progress.rs:97`, `:214-219`; `layout.rs:556-561` (a bounded Column gives growing children their share of a shortfall, a Row does not). | still present (`layout.rs:611`, `progress.rs:220-224`) |
| N2 | A table cell's address is undocumented, and content keyed like its column collides with the cell wrapper in `find_descendant` | documentation gap | annoyance | c1 (#5), c2 (#3) | `table.rs:233` ("own key is free"), wrapper keyed by the column `table.rs:504-511`; probe `Err(AmbiguousKey)`. | still present (`table.rs:239`, `:531-533`); `AmbiguousKey` now names the key (6c1) |
| N3 | Toast messages are not in inspection or anywhere else a test can read | missing capability | slow | c1 (#6, acc-1), c2 (#4) | `notifications.rs:797-826`; probe: role `Notifications`, value 1, no name, no items; `ToastQueue`/`ToastReceipt` expose no message. | still present (`notifications.rs:808-835`; queue renamed `ToastController`) |
| N4 | Terminal `ProgressStatus` values draw a full track, losing the fraction where a job stopped | missing capability (optional) | annoyance | c2 (#5) | `progress.rs:31-38`, `:184-185` (documented). | still present (`progress.rs:31-38`, `:190-191`) |
| N5 | Table header titles cannot be addressed by a headless test; pointer sorting needs positions computed from the application's own widths | API ergonomics (headless testing) | annoyance | c2 (#6); c1 by code (`src/headless.rs:132-143`) | `table.rs:1118`, header items without bounds `table.rs:2824-2858`; keyboard route documented `table.rs:1157-1163`. | still present; the `testing` driver's `chord` eases the keyboard route |
| N6 | `Pending::wait` forbids the UI thread, but the guide and the CPU examples wait on it there | documentation error | annoyance | c2 (#7) | `inspection.rs:582-583` against `guide/diagnostics.md:114`, `:159`, `cpu.rs:35`, `pert-ui-cpu/src/lib.rs:246`. | still present (`inspection_service.rs:584-585`; `guide/diagnostics.md:170`, `:178`, `guide/testing.md:376`) |

Not new: c1#2 (`ProgressBar` takes only a binding) is F16 in a display
control. It is still present at head, where 6f3's controlled forms cover
`Checkbox` and `InlineText` only.
