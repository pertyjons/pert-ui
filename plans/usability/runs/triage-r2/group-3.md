# Round 2 triage, group 3: task 5 (background job monitor) on the branch head

Runs: r2-5-head-claude-1, r2-5-head-claude-2, r2-5-head-codex-1,
r2-5-head-codex-2, all on aca8df3 (0.3.75). Claims are checked in the kit's
vendored crates (`~/pert-ui-usability/kit-aca8df3/vendor/pert-ui*-0.3.75`,
paths below are relative to the named crate's `src/`), the rendered docs in
`kit-aca8df3/doc/` and `git show aca8df3:…`. Run code is cited as
`<run>/work/<path>:<line>`. Runtime claims were reproduced with a throwaway
probe, `~/pert-ui-usability/triage-r2/probes/group-3/work/tests/probe.rs`
(a three-column table at 900 and 500 px with a `ProgressBar` + `Label` row
and a `Button` in a 28 px row; it prints bounds, the validation report and
lookups, and writes CPU captures `cap-900.png` and `cap-500.png`; built
offline in the kit, `target/` deleted).

Facts the rows refer to:

- **P1, ProgressBar width.** `ProgressBar::measure` returns
  `min(available width, 3 × ControlWidth)` (runtime `progress.rs:215-226`);
  ControlWidth is 64 at Standard (`runtime/theme.rs:202`), so 192 px. The
  page says "three themed ControlWidths wide by preference, and fills the
  width it is given" (`progress.rs:100-102`); `with_grow` says "Rows never
  shrink a child" (`layout.rs:611`), as does guide/layout.md:459-460 in the
  collections paragraph. Probe: in a 300 px progress cell the grown bar is
  240 px and fits; in a 167 px cell it stays 192 px, debug validation reports
  `EscapesParent` for it, and the capture shows the "42%" label clipped away.
- **P2, Table addresses.** Table children are `body` and `edit end reporter`
  (probe); `body` is structural (`table.rs:2269`), so `find(&table,
  ["body"])` is `None` (probe) and a test reaches it as `children(&table)[0]`,
  as the `Table` page's own example does (`table.rs:1386-1389`). Each cell is
  wrapped in a component keyed by the column's key and the application's
  element keeps its own key below it: the probe resolves `["r1", "action",
  "cancel"]`, and `find_descendant(&row, "progress")` with a cell element
  keyed like its column fails with `AmbiguousKey("progress")`. The docs say
  only "the element's own key is free, since the table wraps each cell"
  (`table.rs:239`). `Runtime::find` explains structural levels and the
  ambiguity (`runtime/tree.rs:255-259`); `Runtime::key` does not
  (`runtime/tree.rs:127-133`). `TestError::NotFound` holds the whole path,
  not the first unresolved segment (`testing.rs:715-719`).
- **P3, Table row height.** `Table::new(controller, row_height)` documents "a
  fixed logical row height" and its geometry errors only (`table.rs:1513-1520`).
  A cell gives its content at most the row's height (`table.rs:976-980`) and
  is placed with a clip to the cell (`table.rs:1078`). Probe: a `Button` in a
  28 px row has 32 px bounds, the validation report is empty (the clip
  suppresses `EscapesParent`), and the capture shows the buttons of adjacent
  rows cut at the row edge and touching, with no gap.
- **P4, Toast inspection.** `ToastStack::inspect` (runtime
  `notifications.rs:808-836`) reports `visible_count`, `scroll_offset`,
  `paused`, `dismiss_label` and `Semantics` with role `Notifications`, a
  numeric value equal to the count and the Dismiss action. Toasts are painted
  by the stack, not child components (the stack has no `take_children`;
  `notifications.rs:546-566`), so no message or action label reaches
  inspection. `ToastHost`'s page mentions only `dismiss_label` among
  inspection properties (`notifications.rs:428-430`).
- **P5, Toast lifetime.** Every toast lives five seconds of unpaused runtime
  time (`notifications.rs:129-133`, `:151-152`, `:357`); `Toast` has `new` and
  `action` only. The stack schedules one wake at expiry (`cx.wake_after`,
  `notifications.rs:879`); there is no animation in `notifications.rs`.
  `Banner` is persistent ("No automatic expiry, focus transfer or timer",
  `notifications.rs:14-17`), is in the prelude (runtime `lib.rs:422`), and
  is introduced next to `ToastHost` in guide/surfaces.md:313-317;
  `ProgressStatus::Failed` points to it (`progress.rs:33-35`). The `Toast`
  and `ToastHost` pages do not mention it.
- **P6, idle check.** At 0.3.75 the inherent `needs_paint` is `pub(crate)`
  (`runtime/compose.rs:342`); the public method is
  `HostIntegration::needs_paint` (`host.rs:272-278`). Layering L3 (N3) moved
  it; at 359122e it was `pub fn needs_paint` on `Runtime`
  (`crates/pert-ui-runtime/src/runtime/compose.rs:328`). The facade's
  `testing` module re-exports `HostIntegration` (facade `lib.rs:266-268`, its
  page `lib.rs:211-220`), so `use pert_ui::testing::*` reaches it:
  r2-5-head-claude-1's acceptance check calls `needs_paint()` with only the
  prelude and `testing` globs and passes. The testing chapter
  (guide/testing.md) never mentions `needs_paint`, `next_deadline` or an
  idle check; guide/appearance.md:127 names `HostIntegration::needs_paint`
  for hosts.
- **P7, per-row ProgressBar.** `ProgressBar::new` takes
  `impl Into<ReadSignal<ProgressStatus>>`; `ReadSignal` converts only from
  `Signal<T>`, `&Signal<T>` and `&ReadSignal<T>` (runtime `signal.rs:275-294`);
  there is no constant binding. `TableItems::cell` gets no state owner and
  sends controls to their controlled form (`table.rs:262-271`); controlled
  forms exist for `Checkbox`, `Toggle` and `InlineText` (`signal.rs:353-369`),
  none for `ProgressBar`. guide/state.md:20 and guide/layout.md:537 give
  `Signal::sibling` as the advanced per-row route.

## Entries

Rows `<run>#diag-<n>` come from a run's `diagnostics.md` rather than its
friction log (Codex review).

| id | verified | class | item or page | severity (tester) | suggested direction | destination if not fixed |
|---|---|---|---|---|---|---|
| r2-5-head-claude-1#1 | yes, P7. The workaround (a `Signal<ProgressStatus>` per job, created in the Start callback, `src/lib.rs:103-130`) duplicates the row data republished with `set_source`. | recurrence of F16 (a display-only control without a fixed or controlled form; 6f3 gave `Checkbox`, `Toggle` and `InlineText` theirs) | `ProgressBar::new`, `TableItems::cell`, `ReadSignal` | annoyance | A fixed form, `ProgressBar::fixed(ProgressStatus)`, on its own constructor path (as AGENTS.md's ownership-at-construction rule asks), or a constant `ReadSignal`; `TableItems::cell` then names display controls too. | FU (controlled controls), TODO |
| r2-5-head-claude-1#2 | yes, P4. "`DisplayList` has no text" holds: `DrawCommand` holds drawing primitives, not recoverable strings (runtime `drawing.rs:1100`). The tester's "`content_search` searches semantics only" is too broad and is not relied on: content search also reads collection rows through `ContentRows` (runtime `runtime/content_search.rs:11-20`) (Codex review). | new finding NF-1 (missing capability) | `ToastHost` inspection | slow | Report each visible toast as a `SemanticItem` (message as name, action label as an action) in the stack's semantics, as table cells are addressable. | TODO |
| r2-5-head-claude-1#3 | yes, P4: the page names only `dismiss_label`, so a reader cannot tell from the docs that the message is absent. | new finding NF-1 (documentation gap part) | `ToastHost` page | annoyance | State on `ToastHost` what inspection reports, as `ProgressBar`'s page does (`progress.rs:123-125`). | TODO |
| r2-5-head-claude-1#4 | yes, P3 (probe: button 32 px in a 28 px row, no validation issue, rows' buttons touch). | new finding NF-4 (documentation gap; a theme-following default is API ergonomics) | `Table::new` | annoyance | Say on `Table::new` that the row must hold its tallest control (ControlHeight plus padding) and show the metric lookup, or let the row height follow `MetricRole::ControlHeight` by default with a fixed height as an option. | TODO |
| r2-5-head-claude-1#5 | yes: `Failed` and `Cancelled` are full tracks with no fraction (`progress.rs:33-38`); documented, as the tester says. | new finding NF-8 (missing capability, optional) | `ProgressStatus` | annoyance | Optional: terminal statuses that keep the reached fraction, or a builder on the bar. | FU |
| r2-5-head-claude-2#1 | yes: the kit ships the rendered HTML (`kit-aca8df3/doc/`) and the README as Markdown (`kit-aca8df3/readme/`); the tester had no browser and wrote an HTML-to-text script. | kit issue | kit documentation format | annoyance | Ship a text rendering of the guide and item pages in the kit, or say in the brief how to read the HTML. | runs README (kit fixes) |
| r2-5-head-claude-2#2 | yes: README "Cargo features" table; the first build succeeded. | note without problem (positive) | README features | annoyance (noted for completeness) | — | — |
| r2-5-head-claude-2#3 | yes, P7. Workaround as claude-1#1 (`src/lib.rs:187-211`). | recurrence of F16 | `ProgressBar::new`, `TableItems::cell` | slow | As claude-1#1. | FU, TODO |
| r2-5-head-claude-2#4 | yes, P5: a toast's expiry is one scheduled wake; `ProgressBar`'s page shows `next_deadline` as the check (`progress.rs:140-144`). | note without problem | `Toast`, idle checks | annoyance | — | — |
| r2-5-head-claude-2#5 | yes, P1 (probe reproduces the `EscapesParent` and the clipped label). The tester's fix (`Container::preferred_size(Size::new(24, 6))` with `with_grow(1)`, `src/lib.rs:321-326`) works because the container's preferred width replaces the bar's. | new finding NF-2 (API ergonomics; documentation gap on `ProgressBar`) | `ProgressBar` sizing in a `Row`, guide/layout.md | slow | Let the bar's natural width be small (one ControlWidth or less) and rely on `with_grow` to fill, or say on `ProgressBar` that its 192 px preference is not shrunk by a `Row`, with a bar-plus-percent table cell example. | TODO, S9 (table recipe) |
| r2-5-head-claude-2#6 | yes, P2: no page states a table cell's address. | new finding NF-3 (documentation gap) | `Table`, `TableItems`, guide/testing.md "List rows" | annoyance | Document the address `<table>/<row key>/<column key>/<content key>` on `Table` and `TableItems::cell`, and add the table case to the testing chapter's "List rows". | TODO, S9 |
| r2-5-head-claude-2#7 | yes: `Driver::semantics` returns `Semantics` (runtime `testing.rs:377`), which is in neither the prelude (runtime `lib.rs:412-491`) nor `testing` (facade `lib.rs:266-283`), only in `inspection` (runtime `lib.rs:890-904`); guide/testing.md:1-9 says the two globs cover a test. rustc offered "you might be missing a type parameter", no import (diagnostics.md). | recurrence of F3 (a name a glob is documented to cover is missing and rustc suggests no `use pert_ui::…`; here the `testing` module 6f1 added, not the prelude) (Codex review; was new finding NF-6) | `pert_ui::testing` | annoyance | Add `Semantics` (and `FlushReport`, which `settle`/`advance` return) to `testing`. | TODO |
| r2-5-head-claude-2#8 | yes, P2: `Runtime::key`/`children` show the structural `body` (probe) and ToastHost's structural `content` (`notifications.rs:465`), neither of which is a path segment; `NotFound` names the whole path. | new finding NF-3 | `Runtime::key`, `Runtime::children`, `TestError::NotFound`, `Table` | slow | `Runtime::key` points to `find`'s structural levels (or inspection marks a component structural); `NotFound` names the longest resolved prefix. | TODO |
| r2-5-head-claude-2#9 | yes, P4 | new finding NF-1 | `ToastHost` inspection | slow | As claude-1#2. | TODO |
| r2-5-head-claude-2#10 | yes, P4 (source read to prove a negative) | new finding NF-1 (documentation gap part) | `ToastHost` page | annoyance | As claude-1#3. | TODO |
| r2-5-head-claude-2#11 | yes, P3 | new finding NF-4 | `Table::new` | annoyance | As claude-1#4. | TODO |
| r2-5-head-claude-2#12 | yes, P6. The tester imported `pert_ui::host::HostIntegration`, which the `testing` glob already provides. | new finding NF-5 (documentation gap), caused by layering L3 (N3) | guide/testing.md, `HostIntegration::needs_paint` | annoyance | A testing-chapter section "Nothing scheduled while idle": `paint`, then `needs_paint()` (in scope through `testing::*`), `next_deadline()` and `FlushReport::components_updated`; optionally a `Driver` idle query. | S9 (testing chapter), TODO |
| r2-5-head-claude-2#diag-1 | yes: `ApplicationError::new(error.to_string())` failed with E0277, `String: std::error::Error` not satisfied, and rustc's help said to pass the `std::io::Error` itself (diagnostics.md:3-22); the final code does (`src/lib.rs:209`). `ApplicationError::new` takes `impl Error + Send + Sync + 'static` (runtime `error.rs:59`). From diagnostics.md, not a friction entry. (Codex review) | tester error | `ApplicationError::new` | — (one failed build; not logged) | — | — |
| r2-5-head-claude-2#acc-1 | yes, P1 (probe). The lead's "others used `with_grow(1)` and looked correct" holds only for claude-1, whose progress column (flex 2 of 900 px, `src/lib.rs:380-384`) is wider than bar plus label; codex-1 and codex-2 hit the same `EscapesParent` and worked around it (a `Column` and 64 px rows; a wider column). | new finding NF-2 | `ProgressBar` in a `Row` | — (author lead) | As claude-2#5. | TODO |
| r2-5-head-claude-1#acc-1 | yes, P4. Correction to the lead: only the two Claude runs kept their own message lists with `ToastReceipt`s (claude-1 `src/lib.rs:97-99`, `:371-374`; claude-2 `src/lib.rs:145-147`, `:173-176`). codex-1 used `Banner` (`src/lib.rs:316`), which inspection names; codex-2 used a `Label`. | new finding NF-1 | `ToastHost` inspection | — (author lead) | As claude-1#2. | TODO |
| r2-5-head-claude-1#acc-2 | yes: `tests/monitor.rs:130-138` focuses `children(&table)[0]` and sets the active row and column through the controller before the Alt chord. Documented: Alt+Up/Down sorts the active cell's column (`table.rs:1213-1217`), the active cell is the selection's active row and the controller's active column (`table.rs:1232-1233`), and the `Table` example reaches the body the same way (`table.rs:1386-1389`). A click on a cell does both in one step (codex-2, `src/main.rs:445-446`). | note without problem (the body's lack of a key-path address is recorded under NF-3) | `Table` keyboard sorting | — (author lead) | — | — |
| r2-5-head-codex-1#1 | yes, P7; the tester followed guide/state.md:20 and made a `sibling` per cell build (`src/lib.rs:70`). | recurrence of F16 | `ProgressBar::new`, `TableItems::cell` | annoyance | As claude-1#1. | FU, TODO |
| r2-5-head-codex-1#2 | yes, P5. | new finding NF-7 (documentation gap; a lifetime option is a missing capability, optional) | `Toast`, `ToastController::push` | annoyance | `Toast`'s page points to `Banner` for a notification that stays; optionally a lifetime builder or a persistent toast. | TODO, FU (option) |
| r2-5-head-codex-1#3 | yes: `Runtime::keep_alive` documents mailboxes kept from an initializer. | note without problem (positive) | `Runtime::keep_alive`, `Mailbox` | annoyance | — | — |
| r2-5-head-codex-1#4 | yes, P1. Fixed by a `Column` of bar and percent and 64 px rows (`src/lib.rs:69-75`, `:309`). | new finding NF-2 | `ProgressBar` in a `Row` | slow | As claude-2#5. | TODO, S9 |
| r2-5-head-codex-2#1 | yes: the guide has no collections chapter (collections are in guide/layout.md) and there is no `TableState`; `Table`'s example names `TableController` and `TableItems`. Resolved from the docs in minutes. | note without problem | guide index, `Table` | annoyance | — | — |
| r2-5-head-codex-2#2 | partly, P5: the five-second lifetime is right; "an animated presentation" is not (one wake at expiry, no animation). The tester read `Toast`, `ToastHost` and `ProgressBar`, none of which mentions `Banner`. | new finding NF-7 | `Toast`, `ToastHost` | annoyance | As codex-1#2. | TODO, FU |
| r2-5-head-codex-2#3 | yes: `Runtime::shutdown` returns `()` (runtime `runtime.rs:6566`), `HeadlessApp::shutdown` returns `Result` (facade `app/headless.rs:227`); the tester calls it its own mistaken transfer and rustc pointed to it. | tester error | `Runtime::shutdown` | annoyance | — | — |
| r2-5-head-codex-2#4 | yes, P2 (probe reproduces `AmbiguousKey` for a cell element keyed like its column). The key was named in the error (F23's fix). | new finding NF-3 | `TableItems::cell`, `Runtime::find_descendant` | slow | As claude-2#6: document that a cell's wrapper takes the column's key, so `find` with the full path, not `find_descendant`, reaches cell content. | TODO |
| r2-5-head-codex-2#5 | yes, P1. Fixed by a wider progress column and a shorter label. | new finding NF-2 | `ProgressBar` in a `Row`, `TableColumn` widths | slow | As claude-2#5. | TODO, S9 |
| r2-5-head-codex-2#6 | yes: the test assumed equal-length jobs on separate threads finish in one flush; the tester names it application-test friction. | tester error | — | annoyance | — | — |
| r2-5-head-codex-2#diag-1 | yes: `cx.defer(…)` in the Start callback warned `unused_must_use` for `TaskReceipt` ("the receipt is the only report of a `Cancelled` operation; failures also propagate from flush", diagnostics.md:14-37); the final code binds `let _receipt` (`src/main.rs:215`). Same issue as group-1's N4 (`TaskReceipt` kept `#[must_use]` on `defer`/`spawn` although only shutdown cancels them since L4a); recorded here, not merged. From diagnostics.md, not a friction entry. (Codex review) | new finding (group-1 N4, API ergonomics) | `TaskReceipt`, `EventContext::defer` | — (warning; not logged) | As group-1 N4. | TODO |
| r2-5-head-codex-2#acc-1 | partly, P6. `needs_paint` is reachable through `pert_ui::testing::*` as well as `pert_ui::host` (facade `lib.rs:266-268`), so "only through `pert_ui::host`" is wrong; the inherent method did become `pub(crate)` in L3. The gap is that nothing in the testing docs leads to it: codex-2 and claude-2 found it by searching and imported it from `host`, codex-1 and claude-1's tester never checked paint (their acceptance checks added it). | new finding NF-5, caused by layering L3 (N3) | guide/testing.md, `HostIntegration::needs_paint` | — (author lead) | As claude-2#12. | S9, TODO |
| r2-5-head-codex-2#acc-2 | yes. Its 19:00 entry shows it read `Toast`, `ToastHost` and `ProgressBar` only and chose a plain `Label` (`src/main.rs:160`, `:315`) to avoid the toast's expiry wake; it never mentions `Banner`, which guide/surfaces.md:315 introduces beside `ToastHost` and `ProgressStatus::Failed` names. codex-1 found `Banner` from the same pages' neighbourhood. The task asks for the failure "in a notification" (5-job-monitor.md:15) and names no component or `Alert` semantics; the acceptance file itself calls item 3 a judgement (`acceptance.md:49`). | acceptance-policy ambiguity, pending the maintainer's decision on whether a visible message outside the row counts or a notification surface is required (Codex review; was tester error). The missing pointer from `Toast` to `Banner`, which this run's entry does evidence, is NF-7 | `Banner`, `Toast` | — (author lead) | As codex-1#2. | TODO |

Other observation, not an entry: r2-5-head-claude-1's one failed build
(E0599, `border_box.map(|b| b.y())`) comes from inspection geometry being
`Option<[f32; 4]>` (`BoxModelInspection::border_box`, runtime
`diagnostics.rs:107`) while `Runtime::bounds` returns a `Rect`. The tester
fixed it from the compiler message; not logged as friction.

## Recurrence matrix

Rule (author decision, applied after the Codex review): `recurred` needs
the problem or its workaround in the friction log, code, diagnostics or
acceptance; reading an affected page without visible effect is not
recurrence. `passed` needs evidence that the run exercised the area.

| F | claude-1 | claude-2 | codex-1 | codex-2 |
|---|---|---|---|---|
| F1 | passed | passed | passed | passed |
| F2 | passed | passed | passed | passed |
| F3 | passed | recurred | not reached | passed |
| F4 | not reached | not reached | not reached | not reached |
| F5 | not reached | not reached | not reached | not reached |
| F6 | not reached | not reached | not reached | not reached |
| F7a | not reached | not reached | not reached | not reached |
| F7b | not reached | not reached | not reached | not reached |
| F7c | not reached | not reached | not reached | not reached |
| F8 | passed | passed | passed | passed |
| F9 | not reached | not reached | not reached | not reached |
| F10 | passed | passed | passed | passed |
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
| F23 | not reached | not reached | not reached | passed |
| F24 | not reached | not reached | not reached | not reached |
| F25 | not reached | not reached | not reached | not reached |
| F26 | not reached | not reached | not reached | not reached |
| F27 | not reached | not reached | not reached | not reached |
| F28 | not reached | not reached | passed | not reached |
| F29 | not reached | not reached | not reached | not reached |
| F30 | not reached | not reached | not reached | not reached |
| F31 | not reached | not reached | not reached | not reached |
| F32 | not reached | not reached | not reached | not reached |
| F33 | not reached | not reached | not reached | not reached |
| F34 | passed | not reached | passed | passed |

Reasons:

- **F1, passed ×4:** each `work/Cargo.toml:7` is
  `pert-ui = { version = "0.3.75", features = ["cpu"] }`, with no setup
  friction; claude-2#2 found the README feature table clear.
- **F2, passed ×4:** every run tests through `pert_ui::testing::Driver` with
  the chapter's helpers and no helper of its own for clicking, chords,
  settling, waiting or reading semantics: claude-1 `tests/monitor.rs:62`,
  `:130-138`; claude-2 `tests/headless.rs:80`, `:107-116`; codex-1
  `tests/headless.rs:15-23`, `:59-110`; codex-2 `src/main.rs:351-503`.
  claude-2's header clicks by `pointer_input` (`tests/headless.rs:240`)
  follow the chapter's pointer for a point that is no component. The idle
  check the chapter lacks is NF-5, not one of F2's items.
- **F3:** claude-1 passed (`CommandKey`, `KeyChord`, `Modifiers` written,
  `tests/monitor.rs:13`, `:137-138`; they are now in the prelude, runtime
  `lib.rs:491`, and the root import is redundant but harmless); codex-2
  passed (`src/main.rs:354`, `:451-454`). claude-2 recurred (Codex review):
  `Semantics`, which `Driver::semantics` returns, is in neither the prelude
  nor `testing`, its first use failed with E0425 and rustc suggested no
  import (claude-2#7, diagnostics.md:25). codex-1 pressed keys with
  `KeyCode` and `KeyAction` only and wrote none of F3's names.
- **F8, passed ×4:** each reads a label's text as the semantic name without
  friction: claude-1 `tests/monitor.rs:62`; claude-2 `tests/headless.rs:80`
  (cell helper); codex-1 `tests/headless.rs:23`, `:64`; codex-2
  `src/main.rs:410-412`.
- **F10, passed ×4:** `CaptureTarget::window(&…primary_window())` at
  claude-1 `tests/monitor.rs:335`, claude-2 `tests/headless.rs:342`, codex-1
  `tests/headless.rs:169`, codex-2 `src/main.rs:425`.
- **F11, not reached ×4 (author decision after check; the Codex review had
  set passed ×4):** F11 is the misleading diagnostic when a conversion into
  `AppError` or `RuntimeError` fails. Every run's `?` conversions succeeded
  (`build(…)?` in `App::new`'s initializer, which returns
  `Result<(), AppError>`, facade `app.rs:358-361`: claude-1 `src/lib.rs:446`,
  codex-1 `src/main.rs:4`, codex-2 `src/main.rs:327`; claude-2 also
  `Size::new(…)?` in `src/main.rs:6`), and these worked the same at 0.3.52.
  claude-2's one failed conversion-related build, E0277 for
  `ApplicationError::new(error.to_string())` (claude-2#diag-1), is not F11:
  it is the `impl Error` bound of `ApplicationError::new` rejecting a
  `String`, not the blanket `From` for `AppError`, and rustc's help (pass
  the `io::Error`) was correct.
- **F13, passed ×4:** each starts a worker thread from the Start button's
  callback and reports through `Runtime::mailbox`, the route the state
  chapter's "Choosing a way" table gives, without friction: claude-1
  `src/lib.rs:103-130`, claude-2 `src/lib.rs:187-211`, codex-1
  `src/lib.rs:269-298`, codex-2 `src/main.rs:207-305` (through `cx.defer`).
  No run used `EventContext::spawn`, so this shows the area works, not
  that F13's addition was adopted (Codex review).
- **F16, recurred ×4:** a per-row `ProgressBar` needs a binding the cell
  cannot make (P7): claude-1#1, claude-2#3 and codex-1#1 by entry; codex-2
  by code, a `Signal<ProgressStatus>` per job held beside the republished
  rows (`src/main.rs:40`, `:246`, `:61`, written at `:191`).
- **F20, passed ×4:** each read item pages with examples (claude-1 and
  codex-1 `ProgressBar`, claude-2 `ProgressBar` and `Row`, codex-2 `Table`'s
  example); the rendered examples import `pert_ui::` (`doc/pert_ui/
  struct.ProgressBar.html` 1 block, `struct.Table.html` 4 blocks, all
  `use pert_ui::…`; `pert_ui_runtime` appears only in source links).
- **F23, codex-2 passed:** its `AmbiguousKey("state")` named the key
  (codex-2#4), which let it rename the label at once.
- **F28, codex-1 passed:** `Signal::update` on the jobs vector
  (`src/lib.rs:253`, `:292`), no failed build.
- **F29, not reached (Codex review; was passed for codex-1 and codex-2):**
  their empty `Label::new("")` sits in the action cell of fixed-height table
  rows (codex-1 `src/lib.rs:93`, codex-2 `src/main.rs:82`), where the
  label's own height does not affect layout, so the runs do not exercise
  F29.
- **F34:** claude-1 (`Driver::chord`, `tests/monitor.rs:138`) and codex-2
  (`src/main.rs:451-454`) used the chapter's chord recipe; codex-1 pressed
  `AltLeft` and the arrow through `Runtime::key_input`
  (`tests/headless.rs:121-125`), the route the chapter gives for one
  component's own key handling. None met friction. claude-2 sorted by
  pointer only.

## New findings summary

- **NF-1: toast messages are not in inspection.** Missing capability, with a
  documentation gap on `ToastHost`. Severity slow. Runs: claude-1 (#2, #3,
  acc-1), claude-2 (#9, #10). Evidence P4. Both Claude runs read the source
  to prove the absence and kept their own message lists with
  `ToastReceipt`s to test the text; codex-1's `Banner` was inspectable. Fix:
  toasts as `SemanticItem`s in the stack's semantics, and the `ToastHost`
  page saying what inspection reports.
- **NF-2: a `ProgressBar` in a `Row` keeps its 192 px preference.** API
  ergonomics with a documentation gap on `ProgressBar`. Severity slow. Runs:
  claude-2 (#5, acc-1), codex-1 (#4), codex-2 (#5); claude-1 avoided it with
  a wide column. Evidence P1 and the probe. Each run hit `EscapesParent` and
  chose a different workaround (a small `Container`, a `Column` with 64 px
  rows, a wider column). Fix: a small natural width that `with_grow` fills,
  or the rule on the `ProgressBar` page with a bar-plus-percent cell example.
- **NF-3: a table cell's address is undocumented and structural levels
  mislead.** Documentation gap. Severity slow. Runs: claude-2 (#6, #8),
  codex-2 (#4); claude-1#acc-2 shows the body is reached only as
  `children()[0]`. Evidence P2 and the probe. Fix: document
  `<table>/<row>/<column>/<content>` on `Table`/`TableItems::cell` and in the
  testing chapter, point `Runtime::key` to structural levels, and let
  `TestError::NotFound` name the first unresolved segment.
- **NF-4: `Table::new`'s fixed row height has no guidance, and a too-tall
  control is clipped silently.** Documentation gap (a theme-following
  default would be API ergonomics). Severity annoyance. Runs: claude-1 (#4),
  claude-2 (#11). Evidence P3 and the probe capture. Already present at
  359122e (Codex review): the same "fixed logical row height" constructor
  with geometry errors only (`kit-359122e-r2/vendor/pert-ui-runtime-0.3.52/
  src/table.rs:1430-1436`) and the same cell clip (`:1024-1025`); only head
  testers reported it.
- **NF-5: the testing docs name no idle check.** Documentation gap.
  Severity annoyance. Runs: claude-2 (#12), codex-2 (acc-1); codex-1 by
  acceptance (its tester checked deadlines and updates but not paint;
  `r2-5-head-codex-1/acceptance.md:51`) (Codex review). That acceptance
  note (`acceptance.md:51-55`), like codex-2's (`acceptance.md:57-59`), wrongly says `needs_paint` is
  reachable only through `pert_ui::host`; `pert_ui::testing::*` re-exports
  `HostIntegration` too (facade `lib.rs:266-268`) (author decision after
  check). claude-1's tester
  did not check paint either. Evidence P6. The method is in scope through
  `pert_ui::testing::*`; only the pointer is missing. At 359122e (Codex
  review): the recipe was missing too (there was no testing chapter;
  `needs_paint` appears in that kit's guide only in appearance.md), but the
  method was an inherent public `Runtime::needs_paint`
  (`kit-359122e-r2/…/pert-ui-runtime-0.3.52/src/runtime/compose.rs:322-328`).
  Layering L3 (N3) moved it behind `HostIntegration`, which made it harder
  to find without removing access.
- **NF-7: `Toast` has a fixed five-second lifetime and its pages do not point
  to `Banner`.** Documentation gap; a persistent toast is an optional
  missing capability. Severity annoyance. Runs: codex-1 (#2), codex-2 (#2,
  acc-2). Evidence P5. Both Codex runs avoided toasts so that nothing would
  be scheduled after the last job ended; codex-1 found `Banner`, codex-2
  used a plain label and its acceptance item 3 was judged only partly met
  (an acceptance-policy question, see codex-2#acc-2). Already present at
  359122e (Codex review): the same five-second contract and only `new` and
  `action` (`kit-359122e-r2/vendor/pert-ui-runtime-0.3.52/src/
  notifications.rs:128-131`, `:150`, `:158`), with no pointer to `Banner`.
- **NF-8: `ProgressStatus::Failed` and `Cancelled` drop the reached
  fraction.** Missing capability, optional; documented. Severity annoyance.
  Run: claude-1 (#5).

NF-6 (`Semantics` not in `testing`) is withdrawn: it is F3 recurring
(Codex review).

Reopened:
- **F16** (all four runs; display-only `ProgressBar` has neither a
  controlled nor a fixed form, P7).
- **F3** (claude-2#7: `Semantics` missing from the `testing` glob the
  chapter says covers a test, no import suggested by rustc) (Codex review).

Issues recorded in other groups, seen here (cross-references, not merged):
- **group-1 N4** (`TaskReceipt` `#[must_use]` on `defer`/`spawn`): codex-2
  by diagnostics (codex-2#diag-1) (Codex review).
- **group-4 N5** (table header titles not addressable in a headless test;
  pointer sorting needs positions computed from geometry), by code:
  claude-2 computes the header point from the table's top and a row cell's
  bounds (`tests/headless.rs:230-245`) (Codex review); claude-1 likewise
  clicks at the table's origin plus a fixed offset to hit the Name title
  (`tests/monitor.rs:346-358`) (found while verifying the review point).
