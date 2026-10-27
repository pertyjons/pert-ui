# Triage verification, round 2, group 1: tasks 1 and 2 on the branch head

Runs: r2-1-claude, r2-1-codex (task 1, settings editor), r2-2-claude,
r2-2-codex (task 2, task list), all on aca8df3 (0.3.75). Verified against
the kit's vendored sources (`kit-aca8df3/vendor/pert-ui*-0.3.75`, below
`F/` for `pert-ui-0.3.75/src` and `R/` for `pert-ui-runtime-0.3.75/src`),
the rendered docs in `kit-aca8df3/doc/`, and `docs/contracts.md` at aca8df3
(`git show aca8df3:docs/contracts.md`). Submission paths are relative to
`~/pert-ui-usability/<run>/work/`. Transcripts were not read.

Runtime claims were checked with a throwaway probe,
`~/pert-ui-usability/triage-r2/probes/group-1/work/tests/probe.rs`, run with
`plans/usability/run.sh exec kit-aca8df3 …` (target deleted afterwards).
All five probe tests pass:

- `virtual_list_children_follow_display_order`: after the source is
  reordered to c, a, b, `Runtime::children` of the `VirtualList` gives
  `["c", "a", "b"]`.
- `deferred_error_with_dropped_receipt_surfaces_from_settle`: a `Button`
  callback that drops the receipt of `cx.defer(|_| Err(Unmounted))` makes
  `Driver::click` return `Err(Runtime(Unmounted))`.
- `spawn_deliver_error_surfaces`: a `cx.spawn` whose `deliver` returns an
  error, receipt dropped, makes `Driver::wait_until` return the error.
- `same_value_write_keeps_refused_draft`: `IntegerInput` bound to 70 with a
  refused draft `150`; `q.set(70)` leaves the draft (`invalid=true`,
  semantic value 70); `TextCommand::DiscardDraft` ends it.
- `checkbox_named_by_hidden_field_label`: `Checkbox::controlled(false, "", …)`
  inside `Field::new(…).label("Buy milk").hide_label()` has the semantic
  name "Buy milk" and `labelled_by` set.

Friction entries are numbered in order of their `### HH:MM` headings,
leaving out the template heading `### <time> — …`. r2-1-claude has 5,
r2-1-codex 3, r2-2-claude 6 and r2-2-codex 2. New findings are named
N1 to N11; part 3 gives the details.

## 1. Entries

| id | verified | class | item or page | severity (tester) | suggested direction | destination if not fixed |
|---|---|---|---|---|---|---|
| r2-1-claude#1 | yes. The testing chapter promises "One `use` block covers it" (`F/guide/testing.md:1-9`), and says `testing` holds "the other names a test writes" (`:11-17`). The facade's `testing` re-exports (`F/lib.rs:266-283`) and the runtime prelude (`R/lib.rs:412-492`) lack `Semantics`, which `Driver::semantics` returns (`R/testing.rs:377`). It is only at the root and in `component`/`inspection` (`R/lib.rs:297-299`, `:606-612`, `:894-901`). A helper that names the return type fails with E0425, and rustc suggests a type parameter, not `use pert_ui::Semantics` (`r2-1-claude/diagnostics.md`). | recurrence of F3. This is the same problem in the `testing` glob: a name a program writes is outside its documented glob, and rustc offers no import path. The membership comes from 6f1 and L2. F3's own names (keyboard types, `ListDrop`) passed (see the matrix). | `pert_ui::testing`, guide/testing.md intro | annoyance | Add `Semantics` and `InspectValue`, which a test writes for `Detail::Full` properties, to `testing`. Alternatively, narrow the chapter's promise. Extend the `modules.rs` sufficiency test to cover a helper that returns `Semantics`. | TODO |
| r2-1-claude#2 | yes. `HeadlessApp` has `runtime`, `runtime_mut`, `main_window`, `admit_close` and `shutdown` (`F/app/headless.rs:151-227`), and no accessor for what the initializer mounted. `Runtime`'s tree methods (`R/runtime/tree.rs:97-676`) take a known `WidgetId` and do not list roots. The guide's close-policy example teaches the `Rc<RefCell<Option<…>>>` slot (`F/guide/testing.md:305-327`), as does `HeadlessApp`'s rustdoc (`F/app/headless.rs:25-41`). | new finding N1 (API ergonomics) | `App::new`, `HeadlessApp`, guide/testing.md "The close policy" | annoyance | Let the initializer return a value that `on_close` and `HeadlessApp` hand back, for example `HeadlessApp::state::<T>()` or a typed `App<T>`. A lighter option is an accessor for a window's mounted roots. | FU (capability), S9 (recipe) |
| r2-1-claude#3 | yes. The number inputs report the saved value as `SemanticValue::Number` and fall back to the draft text only when nothing is saved (`R/text_fields/number_inputs.rs:2118-2126`). `TextField` reports its current text, the draft (`R/text_fields.rs:3673`). `Semantics::value` says "a field's text … a slider's number" (`R/diagnostics.rs:508-512`) and does not say draft or saved. No editor page says which one it reports. | new finding N5 (documentation gap) | `Semantics::value`, `IntegerInput`, `DecimalInput`, `TextField` | annoyance | State on `Semantics::value` and on each editor which one the value carries while a draft is pending, and point to the `text` property for the number inputs' draft. Consider reporting one of them consistently. | TODO |
| r2-1-claude#4 | yes. `IntegerInput`'s rustdoc names only `validation_error` and `binding_error` (`R/text_fields/number_inputs.rs:9`, `:14`, `:2304`). The `unit`, `display_value` and `shows_display` keys the tester asserted, which pass in acceptance (`tests/headless.rs:72-74`), are written without documentation (`R/text_fields/inline.rs:794-799`, `R/text_fields/number_inputs.rs:2082`). By contrast, `Field` lists its keys (`R/field_status.rs:455`). The `InspectValue` import part is the F3 recurrence of #1 (E0433 without a suggestion, `diagnostics.md`). | new finding N6 (documentation gap) | `IntegerInput`, `DecimalInput` (inspection properties) | annoyance | List each control's inspection keys on its page, as `Field` does. | TODO |
| r2-1-claude#5 | yes. `TaskReceipt` is `#[must_use]` with the reason "the receipt is the only report of a `Cancelled` operation; failures also propagate from flush" (`R/runtime/tasks.rs:31-32`). The `EventContext::spawn` example binds `let _receipt` (`R/runtime/tasks.rs:352`), and so does the guide (`F/guide/state.md:112`). Since L4a, `defer` (and therefore `spawn`) is cancelled before delivery only by shutdown (`R/runtime/tasks.rs:123-125`); a panicking operation also reads `Cancelled` (`:12-16`). (Codex review) L4a removed only the owner-disappearance case that G2-8's illustration used (decisions.md:701-704). G2-8's rule, `#[must_use]` where dropping can hide an otherwise unreported cancellation (decisions.md:699-700), still applies: a shutdown or panic cancellation is reported only in the receipt. The reason text (`tasks.rs:31-32`) is therefore accurate. | new finding N4 (API ergonomics, optional design request). It is adjacent to F19 but not a recurrence of it. | `TaskReceipt`, `EventContext::spawn`/`defer` | annoyance | (Codex review) Optional: reconsider whether a cancellation at shutdown or by panic justifies the warning on every application callback, or say in the examples why the receipt is dropped. This is a design question, not an attribute shown to be obsolete. | TODO |
| r2-1-codex#1 | partly. The window-to-root accessor is missing, as in r2-1-claude#2. The guesses `NativeWindowId::root` and `App::size(w, h)` were the tester's own: the guide's prose is "[`App::size`] can change the initial window size" (`F/guide/getting-started-continued.md:10-11`), and the signature takes a `Size` (`F/app.rs:395`). The purpose was Revert discarding refused drafts, which is N2. The workaround collects the editor ids after mounting (`src/main.rs:250-254`). | new finding N1 (API ergonomics). The guessed signatures are tester errors. | `HeadlessApp`, `App`, `Runtime::text_command` | annoyance | as N1 | FU, S9 |
| r2-1-codex#2 | yes, as a description. No `FieldInspection` type exists. `Field`'s page lists its inspection keys (`label`, `help`, `status`, `message`, `required`) and says the message becomes the control's description (`R/field_status.rs:455-461`). The tester guessed a page name. The wish for documented keys of the numeric controls is N6. | tester error | `Field` (inspection) | annoyance | none for `Field`; N6 for the number inputs | none |
| r2-1-codex#3 | yes, as a description. `SemanticValue::Number` is a struct variant (`R/diagnostics.rs:277-285`). `Driver::wait_until` takes `FnMut(&Runtime)` (`R/testing.rs:333-337`), and the guide and rustdoc examples write `|_|` against signals (`F/guide/testing.md:282`, `R/testing.rs:329`). The compiler pointed to both fixes. Consequence: driver helpers cannot be reused inside the predicate, so both r2-1 testers queried by hand there (`tests/headless.rs:203`, `src/main.rs:312`). | tester error | `SemanticValue::Number`, `Driver::wait_until` | annoyance | Optional: a `Runtime`-level semantics lookup (round 1's AN-1 convenience) would serve the predicate. | none (FU already holds the AN-1 convenience) |
| r2-2-claude#1 | yes. The recipe "An editable list of records" (`F/guide/layout.md:494-742`) covers the task. HTML read through pandoc is a kit condition. | note without problem (positive, F14) | guide/layout.md | annoyance (positive) | none | none |
| r2-2-claude#2 | yes. `Runtime::children` says "ordered immediate children" (`R/runtime/tree.rs:136`). The testing chapter states display order only for `Each` with `within` (`F/guide/testing.md:236-260`). `VirtualList` requests its children in index order (`R/virtual_list.rs:1997-2009`, `R/virtual_axis.rs:408-425`), and the probe confirms display order after a reorder. The children are only the mounted rows, plus pinned ones (focus, capture or `keep`), which nothing says next to `children`. Using `RuntimeError::Unmounted` for a missing `find` result is the tester's own choice. | new finding N7 (documentation gap) | `Runtime::children`, `VirtualList`, guide/testing.md "List rows" | annoyance | Say in "List rows" or on `VirtualList` that its children are the mounted rows in display order. | TODO |
| r2-2-claude#3 | yes. `InlineText`'s first paragraph says editing happens in a `DraftEditor` child keyed `"editor"` that takes focus, and that inspection reports `editing` (`R/text_fields/inline_text.rs:17-29`), in a long paragraph. The testing chapter has no in-place editor section. | new finding N8 (documentation gap, structure) | `InlineText`, guide/testing.md | annoyance | Give the editing child and focus a short section of their own, and show a test reading the draft in the testing chapter. Optional: a driver helper for an editing control. | TODO; S9 (testing chapter) |
| r2-2-claude#4 | yes. `VirtualList::reorderable` says Alt with an arrow moves the active item (`R/virtual_list.rs:1118-1120`), and `on_activate` says it runs on Enter (`:1082-1093`). Neither says that the list consumes the chord before application commands. Only the guide's routing order does. `CommandScope` and its variants have one-line docs (`R/runtime/commands.rs:341-351`). The tester confirmed one move per chord (`tests/headless.rs:265-275`, `:308-316`). | new finding N9 (documentation gap). It is adjacent to F22, whose fix listed the claimed chords on `TextField`/`TextArea` only. | `VirtualList::reorderable`, `on_activate`, `CommandScope` | annoyance | List the keys each built-in collection consumes and link the routing order. Link `CommandScope`'s variants to the routing section. | TODO |
| r2-2-claude#5 | partly. `Checkbox` has no option for a hidden label (`R/choice_controls.rs:501-640`), and its page does not point to a way. One way exists: `Field::hide_label` "keeps the label as the control's name … without drawing it" (`R/field_status.rs:519-527`), and the probe shows a `Checkbox` with an empty label named by such a `Field`. | new finding N10 (documentation gap) | `Checkbox`, `Toggle`, `Field::hide_label` | annoyance (cosmetic) | Point `Checkbox`/`Toggle` to `Field::hide_label` for a control named by text shown elsewhere. | TODO |
| r2-2-claude#6 | yes. `App::headless` and the CPU capture worked as guide/testing.md "The close policy" and "A capture" describe. | note without problem (positive) | guide/testing.md | annoyance (positive) | none | none |
| r2-2-codex#1 | yes. `Modifiers` documents deliberately that there are no per-key constants and shows the field syntax (`R/input.rs:300-317`). `KeyChord::new` says only "Creates an exact chord" with no example (`R/runtime/commands.rs:30-34`), and the page's examples use `named`, `primary` and `navigate_back`. The tester wrote the right code before compiling (0 failed builds). | new finding N11 (documentation gap, minor) | `KeyChord::new` | annoyance | Add an explicit Control or Alt chord example on `KeyChord::new`. The constants were declined by design (round 1's r1-2-codex#2 follow-up). | TODO |
| r2-2-codex#2 | yes. `ListSelection` "is never pruned" (`R/list_selection.rs:44`). The recipe's Delete leaves the removed key active and relies on `selected_shown` (`F/guide/layout.md:518-521`, `:648-652`). The documentation answered the question. | note without problem | `ListSelection`, guide recipe | annoyance | Optional: the recipe's Delete could clear the selection or select a neighbour. | S9 (recipe) |
| r2-1-claude#acc-1 | yes. The author check fails (`acceptance.md`, `author_revert_discards_refused_draft`), and the probe reproduces it. Rule 3 of the draft policy: "A changed bound value … discards a draft … Writing the value the editor already shows changes nothing" (contracts.md:1250-1253 at aca8df3). `IntegerInput`: "External changed values win over drafts" (`R/text_fields/number_inputs.rs:88`). A `Signal::set` with an equal value notifies nothing, so a Revert that writes the applied values leaves a refused draft. `TextCommand::DiscardDraft` (contracts.md:1310) is the documented route. It needs each editor's `WidgetId`, through `Runtime::text_command` or, without a deferral, `EventContext::request_text_command` (`R/runtime.rs:3265-3295`). No guide recipe shows a form Revert. It is not a recurrence: F4, F6 and F21 concern closing, rejection reporting and the field description. Round 1's task 1 acceptance did not test this case. The rule predates L4b; L4b added `DiscardDraft` in text mode. | new finding N2 (API ergonomics) | `IntegerInput`/`TextField` drafts, `FormFields`, `TextCommand::DiscardDraft` | partly met acceptance (author) | A form-level discard, such as `FormFields` ending its fields' drafts, or a Revert recipe using `request_text_command(…, DiscardDraft)`. State on the editors that an equal write does not end a draft. | TODO (docs), FU (form-level discard), S9 (recipe) |
| r2-1-claude#acc-2 | yes. Four chained `combine` calls build a tuple over five signals (`src/lib.rs:164-176`), and the state reaches `on_close` through `Rc<RefCell<Option<Built>>>` (`:111-143`). `ReadSignal` derives only through `map` (one input) and `combine` (two) (`R/signal.rs:758`, `:798`). There is no n-ary form. (Codex review) `map` gives a read-only projection of a struct signal, but there is no writable field projection an editor could bind, so each field needs its own signal. | new finding N3 (API ergonomics); the slot part is N1 | `ReadSignal::combine`, form state | — (acceptance lead) | An n-ary derivation (a closure over several signals) or a writable field projection of one `Signal<Settings>`, and a settings-form recipe. | FU, S9 |
| r2-1-codex#acc-1 | yes, as r2-1-claude#acc-2 (`src/main.rs:69-83`; slot for `on_close`, `:270`). | new finding N3; the slot part is N1 | as above | — | as above | FU, S9 |
| r2-1-claude#acc-3 | partly. The `_receipt` binding is verified (`src/lib.rs:249`, after an `unused_must_use` warning). The claim that an error inside the task is never observed is refuted: a dropped receipt still surfaces `Failed` through the flush (`R/runtime/tasks.rs:153-155`), and the probe shows a failing deferred operation and a failing `deliver` returned from `Driver::click`/`wait_until`. In `App`, flush errors go to `on_error` (`F/app.rs:476-507`). Only a `Cancelled` start, at shutdown or by panic, is lost. | new finding N4 (API ergonomics) | `TaskReceipt` | — | as r2-1-claude#5 | TODO |
| r2-2-claude#acc-1 | partly, as r2-1-claude#acc-3. `let _receipt = cx.defer(…)` (`src/lib.rs:213`) is copied from the guide recipe (`F/guide/layout.md:639`). An error from the deferred `BeginEdit` would propagate from the flush. The deferral is needed because `EventContext` has no `find` for the row's title. | new finding N4 | `TaskReceipt`, guide recipe | — | as N4 | TODO |
| r2-2-codex#acc-1 | partly, as r2-2-claude#acc-1 (`src/main.rs:188`). r2-1-codex also bound `_start` and `_discard` after two warnings (`src/main.rs:141`, `:187`; `r2-1-codex/diagnostics.md`). | new finding N4 | as above | — | as N4 | TODO |
| r2-2-codex#acc-2 | yes. Both r2-2 runs register Alt+Up/Down as application commands and also use `reorderable` (`r2-2-codex/work/src/main.rs:263-274`, `:196`; `r2-2-claude/work/src/lib.rs:266-271`, `:222`). The list consumes the chord when it has focus, so one move happens per press (r2-2-claude tests this). This is not a defect: the duplication makes the chords work from other focus, as the task asks. That it is safe is documented only in the routing order (N9). | note without problem (documentation side is N9) | `VirtualList::reorderable`, commands | — | N9 | none |

## 2. Recurrence matrix

Abbreviations: R = recurred, P = passed, — = not reached.

| F | r2-1-claude | r2-1-codex | r2-2-claude | r2-2-codex |
|---|---|---|---|---|
| F1 | P | P | P | P |
| F2 | P | P | P | P |
| F3 | R | — | P | P |
| F4 | P | P | — | — |
| F5 | P | P | — | — |
| F6 | P | P | — | — |
| F7a | — | — | P | P |
| F7b | — | — | P | P |
| F7c | — | — | P | P |
| F8 | P | P | P | P |
| F9 | — | — | — | — |
| F10 | P | — | P | — |
| F11 | — | — | — | — |
| F12 | — | — | — | — |
| F13 | P | P | — | — |
| F14 | — | — | P | P |
| F15 | — | — | P | P |
| F16 | — | — | P | P |
| F17 | — | — | — | — |
| F18 | — | — | — | — |
| F19 | — | — | P | P |
| F20 | P | P | P | P |
| F21 | P | — | — | — |
| F22 | — | — | P | — |
| F23 | — | — | — | — |
| F24 | — | — | — | — |
| F25 | — | — | — | — |
| F26 | — | — | P | P |
| F27 | — | — | P | P |
| F28 | P | P | P | P |
| F29 | — | — | — | — |
| F30 | — | — | — | — |
| F31 | — | — | — | — |
| F32 | — | — | — | — |
| F33 | — | — | — | — |
| F34 | — | — | P | P |

Reasons for every R and P cell:

- **F1, all P:** each `Cargo.toml:7` depends on `pert-ui = "0.3.75"` from the
  kit registry and no entry concerns setup. r2-1-codex and r2-2-claude read
  `guide/getting_started`. The guide now leads with the registry form.
- **F2, all P:** each run tests through `Driver` (click, `replace_text`,
  `key`/`chord`, `settle`, `wait_until`, `semantics`) and the testing chapter,
  with no hand-built input helpers: r2-1-claude `tests/headless.rs`,
  r2-1-codex `src/main.rs:305-487`, r2-2-claude `tests/headless.rs`,
  r2-2-codex `src/main.rs:308-511`. Their wrappers are one-line
  `find` + `semantics` helpers. The new testing friction is listed
  separately (N1, N6, N7, N8, and F3's recurrence).
- **F3, r2-1-claude R:** `Semantics` and `InspectValue`, which a test writes,
  are outside `prelude` and `testing`. rustc suggests a type parameter or
  nothing (r2-1-claude#1, #4).
- **F3, r2-1-codex — (Codex review):** compiling on `prelude::*` and
  `testing::*` (`src/main.rs:1`, `:292`) does not exercise the area. The
  code names none of the affected types (`Modifiers`, `KeyChord`,
  `CommandKey`, `CommandScope`, `ListDrop`, `Semantics`, `InspectValue`);
  its only inspection names are `InspectionQuery`, `Selector` and `Detail`
  (`:312`), all in `testing`. Changed from passed to not reached.
- **F3, r2-2-claude P / r2-2-codex P:** the run used `KeyChord`, `Modifiers`,
  `CommandKey`, `CommandScope`, `ListDrop` and `ListDropPosition`, which are
  now in the prelude (`R/lib.rs:462-491`), with no import error
  (r2-2-codex: 0 failed builds; r2-2-claude: no compile errors). Both import
  them again from the root by name (`r2-2-claude/work/src/lib.rs:11-14`,
  `r2-2-codex/work/src/main.rs:2`), which is redundant but harmless, as
  many guide examples import by name.
- **F4, r2-1-claude P:** `admit_close` returns KeepOpen with an unconfirmed
  focused prefix draft, which admission saves (`tests/headless.rs:282-300`;
  acceptance item 5).
- **F4, r2-1-codex P, by author check:** the author's check on the run's code
  (`author_close_with_uncommitted_draft_is_refused`) passes with a typed,
  unconfirmed prefix.
- **F5, r2-1 runs P:** both drive `App::on_close` headlessly through
  `HeadlessApp::admit_close` (r2-1-claude `tests/headless.rs:282`,
  r2-1-codex `src/main.rs:434`).
- **F6, r2-1 runs P:** `IntegerInput::range` with `on_rejection` → `Field`
  status (r2-1-claude `src/lib.rs:145-147`, `:188-207`; r2-1-codex
  `src/main.rs:107-126`). The author checks show the library's parse
  refusal ("Enter a whole number" for `abc`, `12.5`) reaching
  `on_rejection`.
- **F7a, r2-2 runs P:** `selected_shown` is derived from
  `filter.items().combine(&active, …index_of…)`
  (`r2-2-claude/work/src/lib.rs:180-181`, `r2-2-codex/work/src/main.rs:171-176`),
  and Remove is gated on it. Author checks pass.
- **F7b, r2-2 runs P:** both use `.hidden_active(HiddenActive::Clear)`
  (`lib.rs:240`, `main.rs:218`), and the tests assert a cleared selection
  when typing hides it.
- **F7c, r2-2 runs P, by author check:** actions work on `filter.items()` rather than
  repeating the match. The author's padded-query check (`" milk "`) passes
  on both.
- **F8, all P:** each reads a label's text from `Semantics::name`: the status
  and marker in r2-1-claude (`tests/headless.rs`, author `status`), r2-1-codex
  `name(…)` (`src/main.rs:377`), and the status in r2-2-claude (`status()`)
  and r2-2-codex (acceptance "status label from inspection").
- **F10, r2-1-claude P:** `CaptureTarget::window(&driver.runtime().primary_window())`
  (`tests/headless.rs:310`) compiled first time.
- **F10, r2-2-claude P:** the same call (`tests/headless.rs:342`).
- **F11, all — (author decision after check):** the Codex review had set
  these cells to passed because each initializer converts a `RuntimeError`
  into `AppError` with `?` (r2-1-claude `src/lib.rs:126`, `:149`; r2-1-codex
  `src/main.rs:264`; r2-2-claude `src/lib.rs:279`; r2-2-codex
  `src/main.rs:297-299`). A conversion that succeeds worked the same way at
  0.3.52, so it does not exercise F11. F11 is the misleading diagnostic when
  a conversion fails, and no run's `diagnostics.md` shows a failed
  conversion or that diagnostic. Set back to not reached.
- **F13, r2-1 runs P:** both start the save with `EventContext::spawn` from
  the Apply callback (r2-1-claude `src/lib.rs:249`, r2-1-codex
  `src/main.rs:141`), as documented. The receipt warning is N4, not F13.
- **F14, r2-2 runs P:** both apps follow the recipe (r2-2-claude#1, the
  positive note; r2-2-codex's closing note).
- **F15, r2-2 runs P:** `on_activate` sends `TextCommand::BeginEdit`
  (`lib.rs:211-220`, `main.rs:186-195`). Enter on the list starts a rename in
  the tester and author tests.
- **F16, r2-2 runs P:** `Checkbox::controlled` and `InlineText::controlled`
  per row (`lib.rs:40-51`, `main.rs:30-37`). Author checks write the vector
  directly and the rows follow.
- **F19, r2-2 runs P:** every `register_command` result is dropped
  (`lib.rs:256-271`, `main.rs:243-274`), and the only warnings are dead code.
- **F20, all P:** they read item pages with examples (r2-1-claude
  `EventContext::spawn`, r2-1-codex `App`/`Runtime::text_command`,
  r2-2 `KeyChord`, `Modifiers`, `InlineText`, `ListSelection`). In the kit's
  rendered pages no visible example imports `pert_ui_runtime`. The only
  remaining mentions are source links and the `testing` page's re-export
  lines.
- **F21, r2-1-claude P:** the tests assert the field's error as the refused
  control's semantic description (`tests/headless.rs:111-155`).
- **F22, r2-2-claude P:** Alt+Down with the new-task `TextField` focused
  reaches the application command (`tests/headless.rs:308-316`), and Delete
  in a field edits text only (`:290-295`).
- **F26, r2-2 runs P:** the search field is found at
  `["tasks", "filter-field", "filter"]` (`lib.rs:162`; author `SEARCH` in
  r2-2-codex).
- **F27, r2-2 runs P:** the titles are `.in_row(true)` (`lib.rs:51`,
  `main.rs:37`), and a single click on a title selects the row (author
  `select`; r2-2-codex test `:475-477`).
- **F28, all P:** custom signal values derive `PartialEq`, which `set`/`update`
  need (r2-1-claude `Settings`, r2-1-codex `src/main.rs:5`, r2-2-claude
  `lib.rs:17`, r2-2-codex `main.rs:4`), with no bound error.
- **F29, r2-1-claude — (applying the review's rule):** `Label::new(close_notice)`
  is empty while no close was refused, but it is the last child of the
  column (`src/lib.rs:286-298`). Any height it took would be invisible
  below the content, and no test or capture measures it. Changed from
  passed to not reached.
- **F34, r2-2 runs P:** modifiers are pressed only through `Driver::chord`
  (r2-2-claude `tests/headless.rs:315`, r2-2-codex `src/main.rs:343-345`).

Not reached, where the area was near but not exercised: F21 in r2-1-codex,
which read the message from the `Field`'s `message` property and never the
control's description. F22 in r2-2-codex, whose Alt+arrow chords were sent
only with the list focused. F23 in r2-1-codex: three group labels share the
key `title`, but every lookup goes through `Driver::find` below each
group, so no `AmbiguousKey` arose.

Matrix rule (author decision, applied after the Codex review): a cell is
`recurred` only with evidence of the problem or its workaround in the
friction log, code, diagnostics or acceptance, and `passed` only with
evidence that the run exercised the area. Acceptance checks run on the
submission's code count as that evidence both ways (author decision after
check). This is why F4 in r2-1-codex
(author check with an unconfirmed draft) and F7c in both r2-2 runs (author
check with a padded query) stay passed. The other passed cells were
rechecked against this rule and keep their cited code.

## 3. New findings summary

| # | Title | Class | Severity | Runs | Evidence |
|---|---|---|---|---|---|
| N1 | An `App` initializer's content and state are reachable from `on_close` and tests only through an application-built `Rc<RefCell<Option<_>>>` slot. No accessor exists for mounted roots or a window's content. | API ergonomics | annoyance | r2-1-claude (#2, acc-2), r2-1-codex (#1, acc-1), r2-2-claude by code (`tests/headless.rs:301-305`). (Codex review) Cross-reference, code-derived and not reported as friction, in other groups' runs: r2-3-claude (`tests/headless.rs:10-12`, `app::Slot`), r2-5-head-claude-1 (`tests/app.rs:21`, `Rc<RefCell<Option<MonitorHandles>>>`), r2-5-head-claude-2 (`src/lib.rs:423`, `Rc<OnceCell<Built>>`). Not merged here. | `F/app/headless.rs:151-227`; `R/runtime/tree.rs:97-676`; taught pattern `F/guide/testing.md:305-327`, `F/app/headless.rs:25-41` |
| N2 | A form Revert that writes the applied values back leaves a refused draft shown, because an equal write is not a change. Ending it needs `DiscardDraft` per editor with its `WidgetId`. There is no form-level discard and no recipe. | API ergonomics | partly met acceptance | r2-1-claude (acc-1, author check fails), r2-1-codex (worked around: `src/main.rs:187-192`, `:250-254`) | contracts.md:1250-1253, :1310 (aca8df3); `R/text_fields/number_inputs.rs:88`; `R/runtime.rs:3265-3295`; probe `same_value_write_keeps_refused_draft` |
| N3 | Dirty state over N fields needs N−1 chained `combine` calls building tuples. There is no n-ary derivation and no writable field projection; `map` projects read-only (Codex review). | API ergonomics | — (acceptance lead) | r2-1-claude, r2-1-codex | `R/signal.rs:758`, `:798`; `r2-1-claude/work/src/lib.rs:164-176`; `r2-1-codex/work/src/main.rs:69-83` |
| N4 | `TaskReceipt` stays `#[must_use]` on `cx.spawn`/`cx.defer`, although failures propagate from the flush. Since L4a only shutdown, or a panic, cancels these. Every callback writes `let _receipt`. The acceptance claim that errors go unobserved is refuted. (Codex review) G2-8's rule still holds, because a shutdown or panic cancellation is reported only in the receipt. This is an optional design request, not a verified obsolete attribute. | API ergonomics (optional design request; adjacent to F19, not a recurrence) | annoyance | r2-1-claude (#5, acc-3), r2-1-codex (2 warnings), r2-2-claude (acc-1), r2-2-codex (acc-1). (Codex review) Cross-reference: r2-5-head-codex-2 (group 3) has the same warning (`diagnostics.md:15-16`) and then binds `_receipt` (`src/main.rs:215`). | `R/runtime/tasks.rs:31-32`, `:123-125`, `:153-155`; decisions.md:689-722; probe `deferred_error_…`, `spawn_deliver_error_surfaces` |
| N5 | `Semantics::value` of a pending draft differs by editor and is undocumented: number inputs report the saved number, `TextField` the draft. | documentation gap | annoyance | r2-1-claude (#3) | `R/diagnostics.rs:508-512`; `R/text_fields/number_inputs.rs:2118-2126`; `R/text_fields.rs:3673` |
| N6 | The number inputs' inspection keys (`unit`, `display_value`, `shows_display`, `text`) are undocumented. | documentation gap | annoyance | r2-1-claude (#4), r2-1-codex (#2's wish) | `R/text_fields/number_inputs.rs:9`, `:14`, `:2304` vs `R/text_fields/inline.rs:794-799`, `number_inputs.rs:2082`; contrast `R/field_status.rs:455` |
| N7 | `Runtime::children` of a `VirtualList` gives the mounted rows in display order, which is stated nowhere. The testing chapter covers only `Each` with `within`. | documentation gap | annoyance | r2-2-claude (#2). (author decision after check) r2-2-codex read the rows with `Runtime::children` without friction (`src/main.rs:376-391`); the sort by `Runtime::bounds` in both r2-2 acceptances is the author's caution, not the testers' work. | `R/runtime/tree.rs:136`; `R/virtual_list.rs:1997-2009`; `R/virtual_axis.rs:408-425`; probe `virtual_list_children_follow_display_order` |
| N8 | `InlineText`'s editing child `"editor"`, which holds focus and the draft, is described in the middle of a long paragraph. The testing chapter has no in-place editor section. | documentation gap | annoyance | r2-2-claude (#3) | `R/text_fields/inline_text.rs:17-29` |
| N9 | `VirtualList::reorderable` and `on_activate` do not say that the list consumes Alt+arrow and Enter before application commands. `CommandScope` variants have one-line docs. | documentation gap (adjacent to F22) | annoyance | r2-2-claude (#4), r2-2-codex (acc-2, duplicated handlers) | `R/virtual_list.rs:1082-1120`; `R/runtime/commands.rs:341-351` |
| N10 | `Checkbox`/`Toggle` do not point to `Field::hide_label` for a control named by text shown elsewhere. The route works (probe). | documentation gap | annoyance | r2-2-claude (#5) | `R/choice_controls.rs:501-640`; `R/field_status.rs:519-527`; probe `checkbox_named_by_hidden_field_label` |
| N11 | `KeyChord::new` has no example of an explicit Control or Alt chord. The field syntax is shown only on `Modifiers`. | documentation gap | annoyance | r2-2-codex (#1) | `R/runtime/commands.rs:30-34`; `R/input.rs:300-317` |

Class counts over the 23 rows: recurrence 1 (F3), new finding 16,
tester error 2, note without problem 4, kit issue 0.

Not verified: whether the native window shows N2's stale draft as the
captures do. No display was available; the semantics come from the author
check and the probe.
