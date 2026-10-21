# Group E: headless testing of an application

Scope: input simulation, inspection, captures and the testing story. Verified
against the sources at `6976668` (0.3.52). Paths without a prefix are under
`crates/pert-ui-runtime/src/`. The probe
`probes/group-e/` (a separate Cargo project on the facade with `cpu`, built
in `target/triage-probes`) confirmed each runtime claim marked "probe". It
reported:

- `find [list, alpha]` = false and `[list, items, alpha]` = true;
- after `text_input("be")` the filter signal is `"be"`, but the filtered-out row
  is still mounted until the next `flush`;
- `focus` followed by `text_input("X")` on "Example" gives "XExample";
- `SelectAll` followed by `text_input("")` leaves "XExample";
- `key_input` Primary+A with `native_modifiers`, then Backspace, gives "";
- `text_command(SelectAll)` followed by `key_input` Backspace gives "";
- an empty `Label` has `name=None value=None`, and `Label("Hello")` has
  `name=Some("Hello") value=None`;
- a `TextField` has `value=Some(Text(..))`;
- `Runtime::query(Selector::id(..))` returned typed semantics;
- `serve_captures` without the flush after `request_capture` still served the
  job (1).

No compile diagnostics in this group: `runs/r1-4-*/diagnostics.md` both say
"None", and none of the entries is a *compile* entry.

## Entries

| id | verified | class | item or page | severity (tester) | suggested direction | destination if not fixed |
|---|---|---|---|---|---|---|
| r1-1-claude#7 | Partly. `text_command(SelectAll)` works (probe: `Handled`). "Needs" is too strong: Ctrl/Cmd+A through `key_input` with `native_modifiers` also selects all (probe; `text_fields.rs:55-56` lists platform Ctrl/Cmd+A). No guide chapter mentions `SelectAll` or `text_command`: `rg` over `crates/pert-ui/src/guide/` finds nothing. `Runtime::text_command`'s summary calls it "a clipboard action" (`runtime.rs:5096`), and `SelectAll` is not one (`text_input.rs:132-137`). | documentation gap | guide/controls.md (text editors), guide/diagnostics.md; `Runtime::text_command` summary | annoyance | Show "replace a field's text" (focus, `text_command(SelectAll)`, type) in the testing chapter. Reword the `text_command` summary to "a text command (clipboard or selection)". | app-author slice 9 (recipe); TODO.md (summary wording) |
| r1-4-codex#1 | Yes. The guide index lists diagnostics as "keyboard input, inspection, screenshots, serialization and profiling" (`crates/pert-ui/src/guide.rs:18-19`), and no chapter covers testing. diagnostics.md covers keys (`:7-48`), inspection (`:65-118`) and captures (`:120-165`), but has no pointer input and no assertion walkthrough. `Runtime::pointer_input` (`runtime/pointer.rs:286-314`) and `PointerEvent` (`pointer.rs:146-157`) have no example; the only pointer example in the guide is `guide/controls.md:461`. The testing chapter is already planned (`plans/app-author-improvements.md:356-357`). | documentation gap | guide index, guide/diagnostics.md; `Runtime::pointer_input`, `PointerEvent` | annoyance | Add a "Testing an application" chapter: mount, viewport, find, click at bounds, type, chord, flush, read semantics, list rows, capture. Add a click example to `pointer_input`. | app-author slice 9 |
| r1-4-claude#3 | Yes. "Inspection is read-only, so listing an action does not provide a way to invoke it: tools drive components through the same input the runtime accepts from a host" (`diagnostics.rs:292-295`). The sentence names none of the input methods (`pointer_input`, `bounds`, `key_input`, `command_key_input`). | documentation gap (merged into M1) | `SemanticAction` | annoyance | Link the input methods from that sentence and point to the testing chapter. Read-only inspection is a design decision (AGENTS.md "inspection contracts"); keep it. | app-author slice 9 |
| r1-4-claude#4 (a): rows under `list` | Yes, against the fixture. `task4-app/README.md` says "`list`, one row per shown bookmark. Each row is keyed by the bookmark's id", but the real path is `list/items/<id>` (probe; `Each` puts items in "a [`Column`] keyed `items` by default", `described.rs:245-248`). The fixture's own `.key("row")` (`task4-app/src/lib.rs:118`) is dead, because Each "keys with that key, replacing the element's own" (`described.rs:245-246`). | kit issue | `plans/usability/task4-app/README.md` "Widget keys"; library side documented on `Each` | annoyance | Give the README the real paths (`content/list/items/<id>/name`) and drop the dead `.key("row")`, or keep the README vague on purpose and say so in the kit notes. | kit (before round 2) |
| r1-4-claude#4 (b): label text in `name` | Yes. `Label::inspect` sets only the name: `Semantics::new(SemanticRole::Label).name(..)` with no value (`controls.rs:401-404`; probe). The Label page says this only in the overflow and refusal paragraphs (`controls.rs:33-36`, `:47`, `:127`). `SemanticValue::Text` reads "Presented or edited text" (`diagnostics.rs:257`) and `Semantics::value` reads "What it currently presents" (`diagnostics.rs:474`). Both describe a label, which reports no value. An empty label's `None` name is documented: `Semantics::name` "empty text leaves it absent" (`diagnostics.rs:531-534`), and "Absent means … never an empty placeholder" (`diagnostics.rs:454-455`). | documentation error (misleading wording) | `SemanticValue::Text`, `Semantics::value`, `Label` | annoyance | Say on `Semantics::value` and `SemanticValue::Text` that static text (labels, button captions) is the name, and that value is the content of an editable or stateful control. Say so on `Label`'s main paragraph too. Optionally list name versus value per standard control in the testing chapter. | TODO.md |
| r1-4-claude#5 | Yes. An empty commit is a no-op: `if !self.enabled \|\| self.locked() \|\| text.is_empty() { return Ok(false) }` (`text_fields.rs:2979-2982`; probe "XExample"). `Runtime::text_input`'s doc (`runtime.rs:4822-4827`) does not say so. After `focus` the caret is at the start (probe), which only the `TextField` example's "New Example" shows (`text_fields.rs:143-150`); no sentence states it. The no-op itself is right, since an IME can commit empty text. | documentation gap | `Runtime::text_input`, `TextField` (caret on focus) | slow | State on `Runtime::text_input` (or `Component::text_input`) that empty text inserts nothing and does not delete a selection. State where `focus` puts the caret. Give the replace-text recipe in the testing chapter: `text_command(SelectAll)`, then Backspace for empty text or `text_input` for new text. | TODO.md (two doc sentences); app-author slice 9 (recipe) |
| r1-4-claude#6 | Yes, a note: 6 of 7 tests passed on the first run, and the 7th failed because of #5. | not friction | none | note | none | none |
| r1-4-claude#7 | Yes, the capture recipe is guide/diagnostics.md:143-165. The tester's nit is partly wrong: in the probe, `serve_captures` served a job with no flush after `request_capture`. The guide example (`diagnostics.md:153-158`) and the `CpuRenderer::serve_captures` example (`crates/pert-ui-cpu/src/lib.rs:241-245`) both flush in between, which suggests the flush is required. | not friction | guide/diagnostics.md "Screenshots" | note | Optional: say whether the flush between request and serve is needed, or drop it from the example. | none (optional TODO.md) |
| r1-4-claude#8 | Yes, the acceptance-3 mutation record. Each break failed only its matching test, as the task asks (`tasks/4-testing-an-app.md` Acceptance 3). | not friction | none | note | none | none |
| r1-4-claude#9 | Yes. The application needed no change: `bookmarks::build(&mut Runtime)` and the `cpu` feature were already there (`task4-app/src/lib.rs:59`, `task4-app/Cargo.toml:7`). | not friction | none | note | none | none |
| r1-4-codex#2 | Yes. `task4-app/Cargo.toml:7` has `version = "0.3.51"`, but the workspace and docs are 0.3.52 (`Cargo.toml:6`). The bump came in `bebb411` after the kit checkpoint at 0.3.51 (`plans/usability/README.md:147`). The caret requirement resolves to 0.3.52, as r1-4-claude#1 notes, so the edit was unnecessary. | kit issue | `plans/usability/task4-app/Cargo.toml` | annoyance | Bump the fixture with each version bump, or write the requirement as `"0.3"`. | kit |
| r1-4-codex#3 | Partly. The README part is the same kit issue as r1-4-claude#4 (a). The repeated `name` key is library behavior and is documented: `find_descendant` returns `AmbiguousKey` "when several descendants carry `key`" (`runtime/tree.rs:217-219`, example `:230-234`). Narrowing to `name-field` first is the intended use. | kit issue | task4-app README; `Runtime::find_descendant` (documented) | annoyance | As in r1-4-claude#4 (a). | kit |
| r1-4-codex#4 | Yes. This records the start of the required mutation check (the acceptance-3 protocol), not a problem. It was logged as "runtime — annoyance" only because the template has no note category. | not friction | none | annoyance (logged) | Kit: add a "note" category to the friction template so protocol records are not counted as friction. | kit |
| r1-4-codex#5 | Yes. The expected test failure and the source restore, which is the same protocol record as #4. | not friction | none | annoyance (logged) | As in #4. | kit |
| pilot-2-codex#7 | Yes. `Runtime::text_input` flushes before dispatch (`runtime.rs:4829-4830`). Afterwards it flushes only when notifications are pending (`deliver_input_notifications`, `runtime/notify.rs:349-353`). `TextField` writes its binding immediately (`text_fields.rs:48-49`), but derived bindings and `Each` update on the next flush (probe: the filtered-out row stays mounted until `flush`). The docs mention only "the flush that runs first" and "the flush that delivers a notification the text requested" (`runtime.rs:4822-4827`). Nothing says that the tree shows the input after the next flush. `key_input` (`runtime.rs:4490-4497`) and `pointer_input` read the same way. | documentation gap | `Runtime::text_input`, `key_input`, `command_key_input`, `pointer_input`; testing chapter | annoyance | Say once, in the `Runtime` type docs and the testing chapter, that input writes bindings and dispatches notifications, and that derived bindings, dynamic children and layout reflect the input after the next `flush`, so a test flushes before it inspects. | app-author slice 9; TODO.md (`Runtime` doc sentence) |

## Author notes (plans/usability/author-notes.md)

| note | verified | class | item or page | suggested direction | destination |
|---|---|---|---|---|---|
| AN-1: reading text needs `inspection_id`, a full `inspect()` snapshot and a string-keyed map | Partly. The documented path really is a linear search over a whole snapshot: the `inspection_id` example (`runtime/tree.rs:408-417`), which both task-4 testers copied (`r1-4-claude/submission/tests/support/mod.rs` `semantics`, `r1-4-codex/submission/tests/ui.rs:35-46`). A cheaper, typed path exists but cannot be discovered. `Runtime::query` on the UI thread with `Selector::id` is "one lookup" at `Detail::Semantics` (`runtime/inspect.rs:207-218`, `inspection.rs:69-72`, `:107-111`; probe). No string-keyed map is needed for text: a label's text is `Semantics::name` and a field's is `SemanticValue::Text`. The `text` property in the map (`controls.rs:376-379`) is the untyped alternative. The guide shows `query` only through the cross-thread `InspectionClient` (`diagnostics.md:87-118`), and `inspection_id` does not link to `Runtime::query`. | documentation gap; also API ergonomics (no `Runtime::semantics(&WidgetId)`) | `Runtime::inspection_id` example, `Runtime::query`, guide/diagnostics.md "Inspection" | Change the `inspection_id` example to `ui.query(&InspectionQuery::components(Selector::id(id)).detail(Detail::Semantics))`, and show it in the guide. A `Runtime::semantics(&WidgetId)` convenience is optional. | TODO.md (example and link); follow-ups (convenience accessor) |
| AN-2: select-all needs `command_key_input` with `native_modifiers`, while plain keys use `key_input` | Partly; for select-all the premise does not hold. The probe selected all with `key_input` plus `native_modifiers(PRIMARY)`, and `text_command(SelectAll)` needs no key at all. `command_key_input` is needed only when the key must reach application commands (`diagnostics.md:18-26`; `runtime.rs:4486-4490`). Task 4's Ctrl+L is such a test, so a task-4 author does meet both entry points. The guide says "a test of what a user sees from a key … uses `command_key_input`" (`diagnostics.md:24-26`), so both testers sent Ctrl+A that way too, which is correct. The ergonomic cost is real: a chord takes four calls (set modifiers, press, release, clear modifiers). | API ergonomics | `Runtime::native_modifiers` / `command_key_input`; guide/diagnostics.md "Keyboard input" | Provide a chord helper in the testing chapter, or a library helper (see M1). | app-author slice 9; follow-ups (helper) |
| AN-3: Each's `items` key segment | Yes. It is documented (`described.rs:245-248`; probe), but `Each`'s only example uses `within(..)` with a custom key (`described.rs:262-266`), so the default `items` segment never appears in a path example. Both task-4 testers hit it and resolved it from the `Each` page; the kit README made it worse (see r1-4-claude#4 (a)). | documentation gap (minor) | `Each` example; `Runtime::find` | Add a default-container lookup (`ui.find(&root, &["list", "items", "<key>"])`) to the `Each` example. Show a key tree for `Each` and `Field` in the testing chapter (r1-4-codex closing #3). | TODO.md (example); app-author slice 9 |

## Helpers testers built by hand

Sources: the r1-4 submissions (`r1-4-claude/submission/tests/support/mod.rs`,
`r1-4-codex/submission/tests/ui.rs:13-135`), r1-1 acceptance notes and pilot
closings. The library's own packaged examples build the same helpers
(`crates/pert-ui/examples/inventory/session.rs:49-110`,
`log_explorer/session.rs:44-88`, `diagram_editor/session.rs:29-79`).

| helper | built by | library counterpart | where it is documented |
|---|---|---|---|
| Click at a component's bounds (reveal or focus, then Press/Release at the centre) | r1-4-claude, r1-4-codex, pilot-2-claude; examples inventory `click` and diagram_editor `click` | None as one call. The pieces are `Runtime::reveal` / `reveal_and_focus` (`runtime/reveal.rs:347,394`), `Runtime::bounds` (`runtime/tree.rs:383-394`), `PointerEvent::new` (`pointer.rs:149`) and `Runtime::pointer_input`. | No example on `pointer_input` or `PointerEvent`. There is one pointer example in guide/controls.md:461. Nothing says that a scrolled-out component must be revealed before its bounds are hit: `bounds` says only "Hidden components retain their last bounds". |
| Replace a field's text (select all, delete, type) | r1-4-claude `type_into`, r1-4-codex `fill`, r1-1-claude (with `text_command(SelectAll)`) | Partly: `Runtime::text_command(id, TextCommand::SelectAll)` (`runtime.rs:5096-5130`), then `text_input`. Empty text needs a Backspace key, because `text_input("")` is a no-op (r1-4-claude#5). | Nowhere as a recipe. `SelectAll` is on `TextCommand` (`text_input.rs:132-137`), behind a summary that calls it a clipboard action. |
| Key chord with modifiers (set modifiers, press, release, clear) | r1-4-claude `chord`/`press`, r1-4-codex `key`+`fill`, pilot-2-codex; examples inventory and log_explorer `chord` | None as one call. `native_modifiers` + `command_key_input` (`runtime/commands.rs:1301,1572`). | guide/diagnostics.md:24-48 has a complete recipe. The packaged examples use a different one: they press the physical modifier key through `command_key_input` and never call `native_modifiers` (`inventory/session.rs:78-93`, `log_explorer/session.rs:74-88`). |
| Find a component's semantics | r1-4-claude and r1-4-codex `semantics` (both linear searches over `inspect()`); r1-1-* likewise | Yes: `Runtime::query` + `Selector::id(ui.inspection_id(&w)?)` + `Detail::Semantics` (`runtime/inspect.rs:207-218`). | Undiscoverable. `inspection_id`'s example teaches the linear search (`runtime/tree.rs:408-417`), and the guide shows `query` only cross-thread (`diagnostics.md:107`). See AN-1. |
| Read a label's text | r1-4-claude `label`, r1-4-codex `label`, r1-1-codex | Yes: `Semantics::name` (or the untyped `text` property). | Only in Label's overflow and refusal paragraphs (`controls.rs:33-36,47,127`). `SemanticValue::Text` and `Semantics::value` point the other way (r1-4-claude#4 (b)). |
| Read a field's text | r1-4-claude `field`, r1-4-codex `value` | Yes: `SemanticValue::Text` in `Semantics::value` (`text_fields.rs:3527`). | `SemanticValue::Text` "Presented or edited text" (`diagnostics.rs:257`). |
| List rows (keys in order, per-row labels) | r1-4-claude `rows`/`shown`, r1-4-codex `rows`/`ids`; example inventory `editing` | Yes: `Runtime::children` + `Runtime::key` (`runtime/tree.rs:124,132`). Or `Runtime::query(Selector::children([.., "list", "items"]))` with semantics in one call (`inspection.rs:99-101`; probe). | `children`/`key` are documented. The `items` segment is on `Each`; see AN-3. |
| Wait for background work (loop over `flush`, with a sleep) | r1-1-claude (`tests/headless.rs:94-103`) | Partly. The `Runtime::spawn` example itself polls with `sleep` + `flush` (`runtime/application.rs:178-183`: "a headless one polls"). A blocking alternative exists: `Runtime::set_wake_delivery` (`runtime.rs:4172-4185`) can signal a channel the test waits on, and `next_deadline` (`runtime.rs:5188-5190`) tells whether work is pending. There is no "run until idle" helper. | The polling recipe is documented on `spawn`. The wake-based wait is not presented for tests. |
| Settle after each action (flush after every input) | Every task-4 helper; pilot-2-codex#7; examples `settle` = three flushes 16 ms apart (`inventory/session.rs:54-59`, `log_explorer/session.rs:50-55`) | `Runtime::flush` | Not stated for input (pilot-2-codex#7). See the lead U2. |

## Merged findings

- **M1. No testing walkthrough and no test-driver helpers.** Entries:
  r1-4-codex#1, r1-4-claude#3, r1-1-claude#7 (partly), r1-4-claude closing #1,
  r1-4-codex closing #1-2, pilot-2-codex closing #3, pilot-2-claude closing #3
  (asks for a `pert_ui::testing` module). Four runs ask for helpers or a
  walkthrough, and the library's own three packaged sessions rebuild click,
  chord and settle helpers. The chapter is planned in app-author slice 9
  (`plans/app-author-improvements.md:356-357`), so it goes there. A public
  helper type (click, set text, chord, settle, semantics-of) is a missing
  capability with three applications already showing the need; it goes to
  follow-ups unless slice 9 takes it.
- **M2. Label text is the semantic name, not the value.** Entries:
  r1-4-claude#4 (b) and r1-1-codex#4 (another group), two runs. Documentation
  error in `SemanticValue::Text` and `Semantics::value`.
- **M3. Each's `items` segment.** Entries: r1-4-claude#4 (a), r1-4-codex#3 and
  AN-3, two runs. Both were caused mostly by the kit README; the library
  documents the segment but never shows it in an example.
- **M4. Replacing a field's text.** Entries: r1-4-claude#5, r1-1-claude#7, the
  r1-4-codex `fill` helper and AN-2. Contributing causes: the `text_input("")`
  no-op is undocumented, the caret placement on focus is undocumented, and
  `SelectAll` sits behind a "clipboard action" summary.
- **M5. When input is settled.** Entries: pilot-2-codex#7 and every task-4
  helper flushing after each step.
- **M6. Kit fixture drift.** Entries: r1-4-codex#2 (version) and the README
  key description (M3).

## Library observations no tester reported

- **U1 (verified).** `Runtime::query` with `Selector::id` is the cheap, typed
  per-component lookup on the UI thread. The `inspection_id` example
  (`runtime/tree.rs:408-417`) instead teaches a full `inspect()` and a linear
  search, which both task-4 testers and the author copied. This is the same
  fix as AN-1 (TODO.md).
- **U2 (lead, not verified).** The packaged `inventory` and `log_explorer`
  sessions "settle" with three flushes 16 ms apart (`inventory/session.rs:54-59`,
  `log_explorer/session.rs:50-55`), with no comment. `Runtime::flush` docs
  (`runtime.rs:5293-5317`) say feedback settles within `MAX_FLUSH_WAVES` in
  one flush. Either one flush suffices, and the examples teach a superstition
  that a testing chapter would copy, or some work (timers, pointer refresh,
  reveal) needs later flushes, and the docs should say which. Check before
  recording.
- **U3 (verified from the code).** There are two chord recipes. The guide
  reports modifiers with `native_modifiers` (`diagnostics.md:41-45`). The
  packaged examples press the physical modifier key through
  `command_key_input` instead (`inventory/session.rs:78-93`,
  `log_explorer/session.rs:74-88`). A reader who compares them cannot tell
  which one the native host's behavior matches; the native host does both.
  Align the examples with the guide or explain the difference (TODO.md).
- **U4 (verified).** `Runtime::pointer_input` and `PointerEvent` have no doc
  example, although a click is the most common test action. This is part of
  r1-4-codex#1.
- **U5 (verified, minor).** `Each` silently replaces the key of the element
  that `build` returns (`described.rs:245-246`). The kit application, written
  by the library author, keys each row `.key("row")`, and that key never
  appears (`task4-app/src/lib.rs:118`). The behavior is documented; the
  example could say "the element's own key is ignored" more visibly. Low
  priority, author's call.
