# Friction log

Copy this file to `/work/FRICTION.md`. Add one entry per event, in time
order, as it happens. Take each entry's time from the shell (`date +%H:%M`);
do not estimate it.

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

### 15:43 — docs-wrong — annoyance

- **Category:** docs-wrong
- **Severity:** annoyance
- **Tried:** Following guide › getting_started › "Run your first window" to add the dependency.
- **Expected:** `pert-ui = "0.3.52"` as for any crates.io crate.
- **Resolved by:** Ignored the guide and used the registry version from kit/README. The guide says "Until a registry release exists, use a local checkout ... `pert-ui = { path = "../pert-ui/crates/pert-ui" }`", which contradicts the published crate the docs are rendered for.
- **Read:** pert_ui/guide/getting_started/index.html, /kit/readme/pert-ui.md
- **Diagnostic:** n/a

### 15:43 — discover — slow

- **Category:** discover
- **Severity:** slow
- **Tried:** Finding which widgets to compose for a to-do list (per-row checkbox + editable title, selection, filter, reorder). The guide's "Children from values" uses `Each` with `Label`s, while "Large collections" points to `VirtualList`; only by reading `VirtualList`, `ListSelection`, `ListFilter`, `CollectionFilter`, `ListItems`, `InlineText` reference pages did I find that `VirtualList::selection` gives arrow-key selection, `VirtualList::reorderable` gives Alt+Up/Down, `CollectionFilter` gives a search field and `InlineText` does in-place rename. ~25 min of reading.
- **Expected:** A guide example of an editable, selectable, filterable list (the most common app shape).
- **Resolved by:** Reading the reference pages; plan: `Signal<Vec<Task>>` → `ListItems` → `ListFilter` → `VirtualList` with `ListSelection`, rows `Row[Checkbox, InlineText]`.
- **Read:** guide/layout ("Children from values", "Large collections"), struct.VirtualList, struct.ListFilter, struct.CollectionFilter, struct.ListSelection, trait.ListItems, struct.InlineText, struct.Checkbox
- **Diagnostic:** n/a

### 15:45 — understand — annoyance

- **Category:** understand
- **Severity:** annoyance
- **Tried:** An application command on Alt+Down (`register_command(.., CommandScope::Application, vec![KeyChord::new(CommandKey::Named(KeyCode::ArrowDown), alt)], ..)`), pressed through `command_key_input` while the new-task `TextField` had focus.
- **Expected:** Either the command runs, or the `TextField` page says it consumes Alt+arrows.
- **Resolved by:** Accepted: the key is eaten in the text phase (stage 6 of `Runtime::command_key_input`), and the command runs when focus is on the list, a button, or nothing. `TextField` only says "Both editors support arrows ... word navigation"; which modified arrows count as editing chords is not listed, so I found out by a failing test. No way to give an app shortcut precedence over an editor (even `CommandScope::Capture` is stage 7, after text).
- **Read:** struct.Runtime (command_key_input), struct.TextField, guide/diagnostics "Keyboard input"
- **Diagnostic:** test assertion: order unchanged after Alt+Down with the text field focused.

### 15:47 — understand — slow

- **Category:** understand
- **Severity:** slow
- **Tried:** Giving each list row a `Checkbox` and an `InlineText` while the data stays in my own `Signal<Vec<Task>>`. Both controls only bind a `Signal<bool>` / `Signal<String>`, and `ListItems::build(&self, index)` gets no `StateOwner`, so it is not obvious where row signals come from.
- **Expected:** A value-plus-callback form (`Checkbox::checked(bool).on_change(..)`), or a guide note on per-row state in a `VirtualList`.
- **Resolved by:** `Signal::sibling` (found on the `Signal` page, whose text is about components owning popup state) creates a throwaway signal per build, seeded from the vector; `on_change` / `InlineText::on_save` write the vector, the derived `ListSource` changes, and the list rebuilds the mounted rows' descriptions ("Large data": "Replacing a source ... rebuilds the descriptions of the mounted rows"). It works, but I had to infer from three pages that a redescription with a new binding is fine; `InlineText` says "a new binding ... discards an open draft", so any vector change during a rename (e.g. a checkbox click) would end it. Acceptable here.
- **Read:** struct.Signal (sibling), struct.Checkbox, struct.InlineText, trait.ListItems, guide/layout "Large data"
- **Diagnostic:** n/a

### 15:47 — missing — annoyance

- **Category:** missing / workaround
- **Severity:** annoyance
- **Tried:** Starting a rename from the keyboard: Enter on the selected list row (`VirtualList::on_activate`) should open the row's `InlineText` editor.
- **Expected:** A method such as `InlineText::begin_edit` or a runtime call, like `EditorSession::begin` for table cells.
- **Resolved by:** Workaround: in `on_activate`, `EventContext::defer` a closure that does `ui.focus(&title)` and then `ui.key_input(KeyCode::F2, ..)`, i.e. synthesizes the documented F2 key. It works (tested), but injecting input from application code is surely not what the library intends.
- **Read:** struct.InlineText, struct.EditorSession, enum.TextCommand, struct.EventContext (method list), struct.VirtualList (on_activate)
- **Diagnostic:** n/a

### 15:47 — compile — annoyance

- **Category:** compile
- **Severity:** annoyance
- **Tried:** `CaptureTarget::window(h.ui.primary_window())`, from the diagnostics guide's prose "CaptureTarget::window builds a window target from a NativeWindowId, such as the main window, Runtime::primary_window".
- **Expected:** It takes the value `primary_window()` returns.
- **Resolved by:** It takes `&NativeWindowId`; added `&`. The compiler message pointed straight at it.
- **Read:** guide/diagnostics "Screenshots"
- **Diagnostic:** `error[E0308]: mismatched types ... expected '&NativeWindowId', found 'NativeWindowId'`

### 15:47 — discover — annoyance

- **Category:** discover
- **Severity:** annoyance
- **Tried:** `Modifiers { alt: true, ..Modifiers::default() }` with only `use pert_ui::prelude::*`.
- **Expected:** `Modifiers` in the prelude, since `KeyCode`, `KeyAction` and `KeyChord`'s friends are used with it.
- **Resolved by:** `use pert_ui::Modifiers` (also `KeyChord`, `CommandKey`, `CommandScope`, `CommandKeyInput`, `PointerEvent`, `PointerKind`, `Point` come from the root). The prelude's membership rule explains it, but the split is hard to predict.
- **Read:** pert_ui/prelude/index.html
- **Diagnostic:** `error[E0422]: cannot find struct, variant or union type 'Modifiers' in this scope`

### 15:47 — understand — annoyance

- **Category:** understand
- **Severity:** annoyance
- **Tried:** Making "keep the selection meaningful" under filtering. `CollectionFilter` moves the active item only when the list has focus; otherwise "a selected row that is filtered out stays selected and hidden", and `CollectionFilter` has no on-change callback to react to a new query.
- **Expected:** A way to learn about a hidden selection.
- **Resolved by:** A derived `ReadSignal<bool>`: `selection.active().combine(&filter.items(), |a, items| items.index_of(a)...)`, bound to the Remove/Move buttons with `Button::enabled` and to the commands with `Runtime::command_enabled_signal`. That works well and was easy once found; the derived-signal approach is not suggested on the filter pages.
- **Read:** struct.CollectionFilter, struct.ListFilter, struct.ListSelection, struct.Permission, struct.Runtime (command_enabled_signal)
- **Diagnostic:** n/a

## Closing

- **Result:** All five acceptance items are met, and `cargo test` (7 tests in `tests/headless.rs`) shows it. The tests use only the runtime's input entry points: `command_key_input` with `native_modifiers` for keys and shortcuts, `text_input`, and `pointer_input` clicks and double-clicks at `Runtime::bounds`. Assertions read the task vector, the status signal and the mounted rows. Covered: add (Enter and the Add button, empty or blank titles refused), check/uncheck by click, select by pointer and arrows, reorder (Alt+Up/Down on the list, the app shortcuts, the buttons, and among the shown rows under a filter), remove (Delete and the button; Delete inside the text field still edits text), rename (double-click or Enter on the row, Enter commits, Escape discards, an empty title is refused), and filtering (case-insensitive, hidden rows unmounted, the vector unchanged, Remove/Move disabled while the selected task is hidden). A CPU capture (`target/tasks.png`) shows the layout, and `validation_report()` lists no issues. The native `App` binary builds; I could not run it without a display. Shortcut limit: Alt+Up/Down do nothing while a text field has focus, because the editor keeps them.
- **Concepts I had to learn:** `Runtime` as the headless host that `App::new` also hands its initializer; `Element` keys and `.key()`; `Signal`/`ReadSignal`, `map`/`combine`; `flush`; `VirtualList` with `ListItems`/`ListSource` (keys are identity, replacing the source rebuilds the rows); `ListSelection`; `ListFilter` + `CollectionFilter`; `reorderable`/`ListDrop`/`DropPosition`; `InlineText` drafts and `normalize`; `Signal::sibling` for per-row state; `register_command`/`KeyChord`/`CommandScope` and the key-routing stages of `command_key_input`; `Permission`, and `command_enabled_signal`; `find`/`find_descendant`/`bounds` to drive input in tests; `EventContext::defer`; capture with `CpuRenderer::serve_captures`.
- **What worked well:** The reference pages are thorough and nearly every one has a runnable headless example, so my tests copied their pattern directly. The built-in behaviours covered most of the task: arrow selection, Alt+arrow reorder through one callback, a case-insensitive filter that keeps row identity, and InlineText's Enter/Escape draft. Almost everything worked the first time it compiled, and 3 of the 4 first tests passed straight away. The headless runtime with `command_key_input` made it easy to verify shortcuts as a user would trigger them. I never needed the library source.
- **The three changes I want most:**
  1. A guide chapter or gallery-style example for the most common app shape: an editable, selectable, filterable list driven by an application `Vec`, including how rows get per-row state (`Signal::sibling` vs. something better), and controls that take a plain value plus a change callback.
  2. A programmatic way to start editing an `InlineText` (e.g. `begin_edit`, or a `Runtime`/`EventContext` call), so "Enter/F2 on the selected row renames" needs no synthesized key.
  3. Document exactly which keys and chords each editor consumes in the text phase (e.g. TextField and Alt+Up/Down), and offer a command scope that takes precedence over editors for chosen chords. Also fix the getting-started dependency instructions (path checkout vs. the published crate).
