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

### 19:05 — understand — annoyance

- **Category:** understand (positive note, recorded for completeness)
- **Tried:** looking for how to build a selectable, filterable, reorderable list driven by my own `Vec`.
- **Expected:** to assemble it from several reference pages.
- **Resolved by:** guide › layout › "An editable list of records" is almost exactly this task (Signal<Vec<Task>>, ListSource, ListFilter, CollectionFilter, ListSelection, VirtualList::reorderable, InlineText::controlled in row mode). The only friction: the "Choosing a way" table pointed me away from `Each` correctly, but the docs are dense prose; I needed a pandoc pipeline to read the HTML offline.
- **Read:** guide/index, guide/getting_started, guide/testing, guide/layout (Children from values, Large collections, An editable list of records).

### 19:08 — understand — annoyance

- **Category:** understand (mostly positive)
- **Tried:** wrote the whole app (lib.rs) from the guide's editable-list example plus `register_command` for shortcuts, then a first headless test reading rows via `Runtime::children` of the VirtualList and `Driver::semantics`.
- **Expected:** several compile rounds.
- **Resolved by:** compiled on the first try, and the add/check/remove/reorder test passed first time. Small uncertainty I had to guess at: whether `Runtime::children` of a `VirtualList` gives rows in display order (the testing guide only states it for an `Each` `.within` container); it does in practice. Also a guess that `RuntimeError::Unmounted` is a unit variant I could use for "find returned None" (it is).
- **Read:** struct.Driver, struct.Semantics, enum.SemanticValue, struct.KeyChord, struct.Modifiers, Runtime::register_command, guide/diagnostics (Keyboard input), struct.CollectionFilter, struct.FilterMatcher, struct.InlineText (controlled, in_row).

### 19:09 — understand — annoyance

- **Category:** understand
- **Tried:** in the rename test, asserted `driver.runtime().focused() == Some(title)` after a double click on an `InlineText` title.
- **Expected:** the title itself to hold focus while editing.
- **Resolved by:** printed `runtime.key(focused)`: it is `"editor"`. The `InlineText` page does say editing happens in a `DraftEditor` child keyed `"editor"`, buried mid-paragraph in a very long first paragraph; I had skimmed past it. The draft text is also that child's semantics value, not the label's. A test helper like `Driver::editing(&id)` or a note in the testing guide would have saved the detour.
- **Read:** struct.InlineText (description), Runtime::key.

### 19:11 — understand — annoyance

- **Category:** understand
- **Tried:** registering Alt+Up/Alt+Down as `CommandScope::Application` commands so they also work when the list is not focused, while `VirtualList::reorderable` already handles Alt+arrow on the focused list.
- **Expected:** a risk of the move happening twice (list and command) when the list has focus.
- **Resolved by:** guide › diagnostics › "Keyboard input" gives the routing order (text editing, capture, bubbling, then view/window/application commands; a consuming stage ends routing), so the list's handling wins. Confirmed by a test: one move per chord from the list, and the command moves from the new-task field. Nothing on `VirtualList::reorderable` itself says that it consumes Alt+Up/Down; I had to infer it from the guide's general rule.
- **Read:** guide/diagnostics, VirtualList::reorderable, Runtime::register_command, enum.CommandScope (variant docs are one line each; "Capture" vs "Application" only made sense after the guide).

### 19:11 — understand — annoyance

- **Category:** understand
- **Tried:** `Checkbox::controlled(done, label, on_change)` for each row's done box.
- **Expected:** a way to have an unlabelled checkbox (the row's title is the label).
- **Resolved by:** kept the visible "Done" label on every row, as the guide's example does. I did not find a documented way to label a checkbox for semantics without showing the text (or to point it at the title). Cosmetic only.
- **Read:** struct.Checkbox.

### 19:11 — runtime — annoyance (positive)

- **Category:** runtime (no problem; recorded because it saved time)
- **Tried:** `App::headless()` to run the real application initializer in a test, and the `cpu` feature as a dev-dependency to render `target/tasks.png` with `CpuRenderer::serve_captures` + `FrameImage::encode_png`.
- **Expected:** some setup trouble.
- **Resolved by:** both worked as documented in guide/testing ("The close policy", "A capture"); the capture shows the layout as intended. `validation_report()` reports no issues.
- **Read:** guide/testing, struct.App (headless), struct.FrameImage.

## Closing

- **Result:** all five acceptance items are met. `cargo test` runs 7 headless tests in `tests/headless.rs`, all passing, that drive the app only with `Driver` clicks, typed text, keys and chords, and read rows, checkbox states, editor text and the status line back through inspection semantics: (1) add with Enter and the Add button, blank titles refused, checking, removing and reordering with the buttons, each updating the list and "N of M done"; (2) rename by double click and by Enter on the list, committed with Enter, discarded with Escape, empty rename refused; (3) a case-insensitive filter that hides rows, leaves the `Vec` unchanged, clears a selection it hides (Remove disabled, Delete does nothing) and keeps a visible one, with Move up acting on the shown rows; (4) arrow-key selection, Alt+Up/Down, Delete, Ctrl+N (from the list and from the search field), and Delete inside a text field editing text only; plus a run of the real `App` through `App::headless` and a CPU capture to `target/tasks.png`. I have not seen the native window: there is no display here.
- **Concepts I had to learn:** the Runtime and `mount_tree` with keyed `Element`s and key paths → `Signal`/`ReadSignal` with `map`/`combine` → `ListItems`/`ListSource` held in a binding → controlled row controls (`Checkbox::controlled`, `InlineText::controlled(...).in_row(true)`) → `ListSelection`, `ListFilter`, `CollectionFilter` and the hidden-active policy → `VirtualList::on_activate` with `EventContext::defer` and `TextCommand::BeginEdit` → `reorderable` and `ListDrop` → `register_command` with `KeyChord`/`Modifiers`/`CommandScope` and the key-routing order → `Driver`, `settle`/`advance`, `ClickCounter` intervals for single and double clicks → `Semantics`/`SemanticValue` for reading what the app shows → `InlineText`'s internal `"editor"` child → `App::headless` and CPU captures.
- **What worked well:** the guide's "An editable list of records" is nearly this exact app, test included, so the app compiled first time and most tests passed first time. Reference pages state precise behavior (routing order, filter selection policy, double-click counting, what Escape/Enter do in `InlineText`). `Driver` plus semantics makes UI-level tests short and readable. Error types work with `?` into `Box<dyn Error>` everywhere.
- **The three changes I want most:**
  1. Make the focus/semantics location of an editing `InlineText` easier to see: put the `"editor"` child in a separate, short section (or have `Driver`/`Semantics` on the label report the draft and an `editing` flag), instead of mid-way through a very long first paragraph.
  2. On `VirtualList::reorderable` (and `on_activate`), state which keys the list consumes (Alt+Up/Down, Enter), so it is clear how they interact with application commands registered on the same chords.
  3. Say on `Runtime::children` (or in the testing guide's "List rows") that a `VirtualList`'s children come back in display order and are only the mounted rows. Also give `Checkbox` a way to have no visible label but keep a name for semantics.
