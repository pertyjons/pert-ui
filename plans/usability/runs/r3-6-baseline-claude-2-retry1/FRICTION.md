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


### 17:38 (written later) — understand — annoyance

- **Category:** understand
- **Severity:** annoyance
- **Tried:** to find how to add and select tabs at run time (opening a note adds a tab).
- **Expected:** a "Tabs from a binding" constructor, or a section in the layout guide on dynamic tab sets.
- **Resolved by:** combining three places: `Tabs::new` ("an empty `new` means tabs handed over as raw elements"), the `Tabs::reorderable` example (which re-describes the strip with `Runtime::reconcile_children` from `cx.defer`) and `Runtime::reveal_child` ("This is how application code selects a tab by key"). It worked first time once found; the guide's "Retained tabs" section mentions neither.
- **Read:** guide/layout ("Retained tabs and resizable edges"), `struct.Tabs.html`, `struct.Tab.html`, `struct.Runtime.html#method.reveal_child`, `struct.EventContext.html#method.defer`.

### 17:40 — compile — annoyance (my mistake, recorded for completeness)

- **Category:** compile
- **Severity:** annoyance
- **Tried:** `MenuBar::new(..)?` inside a function returning `RuntimeError`; I assumed `ChoiceError` would not convert and wrote a `map_err` to an arbitrary variant.
- **Expected:** —
- **Resolved by:** the `RuntimeError` page lists `impl From<ChoiceError> for RuntimeError`, so `?` works. No diagnostic; the docs were right.
- **Read:** `enum.RuntimeError.html` trait implementations.

### 17:41 — note — positive

- **Category:** understand
- **Severity:** annoyance (none, recorded as a data point)
- **Tried:** first full build of the app: sidebar `VirtualList`, `Tabs` filled by `reconcile_children`, `MenuBar` with `MenuItem::command`, two `Dialog`s, `ToastHost`, `Dynamic` status line.
- **Expected:** a round of compile errors.
- **Resolved by:** it compiled and the first exploratory headless test (`Driver`, `Runtime::inspect`) showed the list in order, the opened tab, the status line and the focused editor. The guide's examples were enough.
- **Read:** guide/testing, guide/surfaces, guide/layout, `VirtualList`, `Dialog`, `DialogController`.

### 17:43 (written later) — discover — annoyance

- **Category:** discover
- **Severity:** annoyance
- **Tried:** to choose File › Save with the pointer in a headless test: `Driver::click` on the File title, then find the "save" row.
- **Expected:** menu rows to be components I can `find` and `Driver::click`, like list rows.
- **Resolved by:** the open menu shows its rows as `Semantics::items` of an internal `items` component (found via `Runtime::inspect`), and `Driver` has no "activate item" helper. I drive the menu by keyboard (F10, ArrowDown to open on the first row, ArrowDown, Enter) as the surfaces guide's example does. `find_descendant(&bar, "items")` also returned `AmbiguousKey("items")` (each title has one); searching under `title/file` works.
- **Read:** guide/surfaces (menus), `testing/struct.Driver.html`, `struct.MenuBar.html`.

### 17:46 — understand — slow

- **Category:** understand
- **Severity:** slow (about 15 minutes)
- **Tried:** `Driver::click` on the close dialog's Save button; it failed with `NotRevealed { key: "save", refusal: AncestorRefused { ancestor: WidgetId { serial: 5, mounted: false, .. } } }`.
- **Expected:** the error to tell me the dialog was closed.
- **Resolved by:** bisecting my own test: the dialog had never opened, because the text I typed before Ctrl+W went nowhere (closing the previous tab left focus outside any editor), so the tab was unmodified and closed without a dialog. `RevealRefusal::AncestorRefused` ("`ancestor` hides the next component on the path") is accurate, but `mounted: false` in the `WidgetId` debug output is misleading: serial 5 is the mounted `Dialog` owner (`Runtime::inspect` shows it visible). A refusal naming the ancestor's key or saying "surface closed" would have saved the bisection.
- **Read:** `testing/enum.TestError.html`, `enum.RevealRefusal.html`, `struct.Dialog.html`.

### 17:47 — missing — slow

- **Category:** missing
- **Severity:** slow (about 10 minutes)
- **Tried:** to assert in a headless test that a failed save shows its notification text ("Could not save alpha.txt: ..."): `Runtime::inspect` (semantic names), `Runtime::query` with `Detail::Full` on the `toasts` part, and `Runtime::content_search` (the `ToastHost` page lists a `content_rows` implementation).
- **Expected:** the visible toasts' messages as semantic items, a property, or content-search rows, as menus expose their rows as `Semantics::items`.
- **Resolved by:** not resolved for the text. The host exposes only the count (`Semantics::value = Number(1.0)`, property `visible_count`); `ContentQuery::everything()` returns "1" for it. My test asserts the count of visible notifications instead of the message.
- **Read:** `struct.ToastHost.html`, `struct.Toast.html`, `struct.ToastController.html`, `struct.Runtime.html#method.content_search`, `struct.ContentQuery.html`, `enum.Detail.html`.

### 17:48 (written later, event at about 17:47) — workaround — annoyance

- **Category:** workaround
- **Severity:** annoyance
- **Tried:** after closing the selected tab with `Runtime::reconcile_children`, focus the editor of the tab the strip selects in its place.
- **Expected:** either focus to follow the newly shown tab's content, or a documented way to learn the new selection inside the same command.
- **Resolved by:** in the command action I call `ui.flush(ui.now())` after `reconcile_children` so that `Tabs::on_select` (`ChangeOrigin::Redescription`) delivers the neighbour into my `active` signal, then `Runtime::focus` its editor. `EventContext::defer` documents nested flushes as allowed; for a `register_command` action it is not stated, though it works. Without this, focus was left nowhere and typing after a close went to no editor.
- **Read:** `struct.Tabs.html` (selection after removal), `struct.Tabs.html#method.on_select`, `struct.EventContext.html#method.defer`, `struct.Runtime.html#method.flush`.

### 17:48 — runtime — annoyance (expected)

- **Category:** runtime
- **Severity:** annoyance
- **Tried:** `cargo run -- <folder>` in the sandbox.
- **Expected:** a failure, since there is no display.
- **Resolved by:** it fails cleanly: `Error: EventLoop("... neither WAYLAND_DISPLAY nor WAYLAND_SOCKET nor DISPLAY is set.")`. All verification is headless (`Runtime` + `Driver`, `App::headless`, `CpuRenderer`).
- **Read:** —

## Closing

- **Result:** All five acceptance items are met, each shown by a headless check run with `cargo test`:
  1. `tests/acceptance.rs::lists_notes_and_opens_tabs`: the folder holds three `.txt` notes, a `.md` file and a directory named `folder.txt`; only the three notes are listed, in order. Alpha opens by a click and beta by focusing the list, then ArrowDown and Enter. Reopening either one (by click, then by keyboard) selects its existing tab; the tab count stays at two.
  2. `typing_marks_and_saving_clears`: typing sets `alpha.txt *` and changes the status counts, including Enter adding a line. Ctrl+S and File › Save (via the menu bar, by keyboard) write the file and clear the marker. After the folder is removed, a save (both Ctrl+S and the menu) leaves the marker and raises the visible-notification count. The toast's *text* cannot be checked, see the 17:47 "missing" entry.
  3. `new_note_dialog_refuses_and_creates`: an empty name, `a/b`, `back\slash`, `.hidden`, a 65-character name and an existing name (`alpha`, `alpha.txt`) are each refused with the explanation shown in the dialog and no file created. `delta` (Enter) and `epsilon.txt` (Create button, dialog opened from File › New note) are created empty, listed in order and opened.
  4. `closing_tabs`: an unmodified tab closes without a dialog. On a modified tab, Cancel keeps the tab and its text with the file untouched, Discard closes it without writing, and Save writes the file and closes the tab. A failed Save keeps the tab open and modified and shows a notification.
  5. `tests/capture.rs` writes `/work/capture.png` (960×640, CPU renderer): two notes open, `ideas.txt *` modified, with the status line visible.
  Also `tests/startup.rs` starts the app through `App::new(..).headless()`, as `main` does, and checks that the divider resizes by keyboard (+16 px for two ArrowRight presses) and by pointer drag (+60 px).
- **Concepts I had to learn:** keyed `Element`s and key paths. `Signal`/`ReadSignal`, with `map`/`combine` for the tab title and the status. `Dynamic` for one child built from a binding. `VirtualList` with `ListSource`/`ListItems`/`ListSelection` and `on_activate`. `Tabs` re-described through `Runtime::reconcile_children`, selected through `reveal_child`, followed through `on_select`. Commands (`register_command`, `KeyChord::primary`, `MenuItem::command`), whose actions get `&mut Runtime`. `EventContext::defer` for runtime work from button callbacks. `Dialog`/`DialogController` with their outcome slot (`take_outcome` before `open`). `ToastHost`/`ToastController`. For testing: `Driver` (settle, click, chord, key, type/replace text), `Runtime::inspect`/`query`/`Semantics`, `RevealRefusal`, and `CpuRenderer` capture with `FrameImage::encode_png`.
- **What worked well:** the guide's examples are complete, runnable and close to real needs: the testing chapter and the surfaces chapter (menu bar plus command plus test) almost gave me the app's skeleton. The app compiled on the first try and most behaviour was right the first time. Errors are typed and precise (`AmbiguousKey`, `NotRevealed { refusal }`). Inspection exposes tab titles and selection as semantic items, and the CPU capture took four lines. Commands that receive `&mut Runtime` made New/Save/Close straightforward, and shortcut and menu share one code path by construction.
- **The three changes I want most:**
  1. Expose toast messages to inspection or content search (for example as `Semantics::items` of the toast host), so a test can assert *what* was notified, not only how many.
  2. Let `Driver` activate a semantic item (a menu row, a tab title, a toast action), or make menu rows addressable components, so pointer paths through menus can be tested without hand-made `PointerEvent`s.
  3. Add a short "tabs from data" recipe to the layout guide (open, reveal, close a tab, and where focus goes after a close), and make `RevealRefusal`/`WidgetId` debug output name the refusing component's key rather than printing `mounted: false` for a mounted widget.
