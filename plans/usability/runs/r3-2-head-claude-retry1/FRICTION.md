# Friction log

Copy this file to `/work/FRICTION.md`. Add one entry per event, in time
order, as it happens. When something costs you time, run `date +%H:%M` at
that moment and write the entry before you continue; do not estimate a
time. If you do write an entry later, give it the time you write it and add
`(written later)` after the time.

## Entries

### <time> — <category> — <severity>

- **Category:** one of
  - *discover*: could not find how to do it;
  - *understand*: found it, but it was unclear how it behaves;
  - *compile*: a compiler diagnostic that did not point to the fix;
  - *runtime*: behavior differed from the documentation or your expectation;
  - *docs-wrong*: the documentation is incorrect;
  - *missing*: there seems to be no way to do it;
  - *workaround*: you did it in a way you think the library did not intend;
  - *source*: you read the library's source (say why).
- **Severity:** *blocker* (you could not continue without it), *slow* (it
  cost noticeable time), *annoyance*.
- **Tried:** what you did.
- **Expected:** what you expected to happen or to find.
- **Resolved by:** what resolved it, or "unresolved".
- **Read:** the documentation pages or items you read for it.
- **Diagnostic:** for *compile*, the exact error code and the first lines.

### 17:23 — understand — annoyance

- **Category:** understand
- **Severity:** annoyance
- **Tried:** Read the getting-started chapter to learn which version to depend on.
- **Expected:** The version in the docs (0.3.88, per the crate page).
- **Resolved by:** The guide says `pert-ui = { version = "0.3" }` and also suggests a `path` dependency on a checkout; I used `"0.3.88"`. Not a problem, but the guide spends space on a checkout/`cargo run -p pert-ui --example` workflow that a docs.rs reader can't use.
- **Read:** guide/getting_started.
- **Diagnostic:** n/a

(Positive note, not friction: guide/layout "An editable list of records" is almost exactly this task: VirtualList + ListFilter + CollectionFilter + ListSelection + controlled InlineText/Checkbox + reorderable. guide/testing and testing::HeadlessApp cover the headless check. That saved a lot of discovery time.)

### 17:27 — workaround — annoyance (written later, the events were 17:24–17:26)

- **Category:** workaround
- **Severity:** annoyance
- **Tried:** In `build`, after `mount_tree`, I looked up the new-task field and the list with `Runtime::find`, which returns `Result<Option<WidgetId>, RuntimeError>`. Inside a function that returns `RuntimeError`, a `None` needs some error.
- **Expected:** A `RuntimeError` variant for "no component at this path", or a `find` variant that returns an error, like `testing::Driver::find` does (`Result<WidgetId, TestError>`).
- **Resolved by:** At first I used `.ok_or(RuntimeError::Closed)` as a placeholder. Then I found `RuntimeError::Application(ApplicationError)` and wrote a small helper that boxes a message, `RuntimeError::Application(ApplicationError::from(boxed))`. That works, but it is a lot of code for a lookup of a component I mounted myself two lines earlier. An alternative I didn't take: `mount_tree` could return the ids of keyed children.
- **Read:** struct.Runtime.html#method.find, enum.RuntimeError.html (variant list), struct.ApplicationError.html.
- **Diagnostic:** n/a

### 17:27 — understand — annoyance (written later, the event was ~17:25)

- **Category:** understand
- **Severity:** annoyance
- **Tried:** Writing the "Move up/down" buttons and the Alt+Up/Down commands so they reuse the list's reorder logic. `VirtualList::reorderable` hands a `ListDrop` whose `ListDropPosition<'_>` borrows its key and is non-exhaustive (needs a `_` arm). So I couldn't build a `ListDropPosition` from my own owned key in a simple way and pass it to the same function.
- **Expected:** Either an owned position type, or a public way to ask the list itself to "move the active row up/down" (the list already does exactly that on Alt+arrow when it has focus), so that a button or an application command could trigger it.
- **Resolved by:** My own small `Place { Before(String), After(String), End }` enum, converted from `ListDropPosition` in the reorder callback. Buttons and commands find the neighbouring *shown* row through `ListFilter::items()` + `ListItems::index_of/key`, so a move under an active filter steps over hidden rows the same way the list's Alt+arrow does.
- **Read:** struct.VirtualList.html#method.reorderable, struct.ListDrop.html, enum.ListDropPosition.html (via the guide example).
- **Diagnostic:** n/a

### 17:27 — understand — annoyance

- **Category:** understand
- **Severity:** annoyance
- **Tried:** Registering application commands for Delete and Alt+Up/Down while the list (reorderable, so it consumes Alt+arrow) and two text fields (which consume Delete) are on screen. I worried about double handling (the list moves the row *and* my command moves it again) and about Delete in a text field removing a task.
- **Expected:** A note on `register_command` saying what happens when a focused component also handles the chord.
- **Resolved by:** guide/diagnostics "Keyboard input" says the stages clearly: text editing and bubbling come before application command bindings, and "a focused editor that uses a chord keeps it from an application shortcut". My tests confirm it: Delete in the new-task field deletes a character, and Alt+Down with the list focused moves the row exactly once. Breaking my command's move-down path did not break the focused-list test, which shows the list handled it itself. Only an annoyance because the answer is in the guide, not on `register_command`.
- **Read:** struct.Runtime.html#method.register_command, guide/diagnostics#keyboard-input.
- **Diagnostic:** n/a

### 17:28 — understand — annoyance

- **Category:** understand
- **Severity:** annoyance
- **Tried:** I needed the rows a `VirtualList` shows, in order, for test assertions. The guide/testing introduction says the driver "lists a list's rows in order", so I guessed `Driver::list_rows`.
- **Expected:** A method with "rows" in its name.
- **Resolved by:** It is `Driver::address_children`, explained in guide/testing "List rows". Found in about a minute.
- **Read:** testing/struct.Driver.html, guide/testing#list-rows.
- **Diagnostic:** n/a (I fixed the name before the first compile).

### 17:28 — understand — annoyance

- **Category:** understand
- **Severity:** annoyance
- **Tried:** A CPU capture (feature `cpu`) to look at the layout, following guide/testing "A capture".
- **Expected:** It worked first time. The only friction: the guide explains why `Pending::wait` would block forever and `try_reply` must be used, which is a trap a reader would not have guessed. It's documented, but `serve_captures` could return the replies directly.
- **Resolved by:** Followed the guide; the PNG shows the intended layout.
- **Read:** guide/testing#a-capture.
- **Diagnostic:** n/a

## Closing

- **Result:** Every acceptance item is met, and `cargo test` checks it headlessly (`tests/headless.rs`, 6 tests, all passing; clippy is clean). The tests use `App::headless()` on the same `App` that `main` runs, and drive it only through `testing::Driver` (clicks, typing, keys, chords) and the runtime's focus and inspection. They do not call my model functions.
  1. Add (Enter and the Add button; blank titles are refused), check (clicking the checkbox), remove (button and Delete), reorder (buttons, and Alt+Up/Down both with and without list focus). Each one updates the `Vec` and the status line, which is asserted through the label's semantics.
  2. Rename in place: Enter on the active row or a double click on the title opens the editor; Enter saves, Escape discards (the vector and the label's semantics are unchanged).
  3. Filter: the built-in `CollectionFilter` search field is case-insensitive ("SHIP" matches "Ship it"). The shown rows are read through `Driver::address_children`, the vector stays the same, and a selected row that stays visible stays selected. A selected row that gets hidden clears the selection (`HiddenActive::Clear`), so Remove/Delete/move do nothing. Moves under a filter step over hidden rows.
  4. Ctrl+N focuses the new-task field, Delete removes the selected task (but deletes a character while a text field has focus), Alt+Up/Down move it. These are application commands, and the reorderable list handles Alt+arrow itself when it has focus.
  5. Extra checks: development validation reports no issues, and a CPU capture (`target/tmp/tasks.png`) shows the expected layout.

  I did not run the native window: there is no display. I did not read the library source.
- **Concepts I had to learn:** keyed `Element`s and key paths (`Runtime::find`) → `Signal`/`ReadSignal::map`/`combine` → `ListItems`/`ListSource` over a copy of the records, with keys from record identity → controlled controls (`Checkbox::controlled`, `InlineText::controlled().in_row(true)`) → `ListSelection`, `ListFilter`, `CollectionFilter`, `HiddenActive` → `VirtualList::on_activate` + `EventContext::defer` + `TextCommand::BeginEdit` → `reorderable`/`ListDrop` → application commands (`register_command`, `KeyChord`, `CommandScope`) and the key routing order → `App` application state and `App::headless`/`HeadlessApp::runtime_and_state_mut` → `testing::Driver` (settle, advance for separate clicks, chords, `address_children`) → CPU captures.
- **What worked well:** The guide's "An editable list of records" section is nearly this whole task, including a tested example, so the app compiled at the first `cargo build` and all tests passed at the first `cargo test`. The guide explains *why* things are done (for example, why the row controls are controlled, and why the Delete button is guarded by `selected_shown`). The testing chapter and `HeadlessApp` make headless verification of the real `App` easy. Keyboard routing (editor before app command) is documented and behaves as documented. Reference pages state behavior precisely (Escape restores, empty filter shows all, the substring matcher ignores case).
- **The three changes I want most:**
  1. A public way to trigger the list's own "move active row up/down" (and "remove active") from a button or an application command, or an owned `ListDropPosition`. Then I wouldn't have to re-derive neighbours through `ListFilter::items()` and convert positions myself.
  2. A `Runtime::find` variant that returns an error instead of `Option` (as `Driver::find` does), or a `RuntimeError` "not found" variant. Today a missing-path error needs `ApplicationError` plumbing.
  3. On reference pages such as `register_command`, a sentence about precedence over focused components, linking to the routing stages that are currently only in guide/diagnostics. Also: name the row-listing driver method in the testing intro (it says "lists a list's rows" but the method is `address_children`).
