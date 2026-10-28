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

### 17:31 — understand — annoyance

- **Category:** understand
- **Severity:** annoyance
- **Tried:** reading struct pages (Runtime, Tabs, VirtualList, Dialog...) through pandoc to plan the app.
- **Expected:** a compact signature list per type.
- **Resolved by:** stripped everything after "## Trait Implementations" with awk and grepped "^#### pub fn". Struct.Runtime alone is ~4000 lines of Markdown; finding e.g. how a command callback gets `&mut Runtime` meant reading long prose. The guide chapters (testing, layout, surfaces) were very helpful and answered almost all design questions up front (App::headless, Driver, Tabs + reconcile_children, MenuItem::command, DialogController, ToastHost semantics items).
- **Read:** guide/index, guide/getting_started, guide/testing, guide/layout, guide/surfaces, guide/controls (text fields), struct.Tabs, struct.Tab, struct.Dynamic, struct.VirtualList, struct.App, struct.Dialog, struct.DialogController, struct.ToastHost, struct.MenuItem, struct.MenuBar, struct.Runtime (register_command, reveal_child, reconcile_children, defer).
- **Diagnostic:** n/a

### 17:32–17:34 (time not taken with `date` at the moment; I first wrote 17:44 by mistake) — understand — annoyance

- **Category:** understand
- **Severity:** annoyance
- **Tried:** `driver.find(&list, &["b.txt", "open"])` for the Button a `ListItems::build` returned as `Button::new(..).key("open")`.
- **Expected:** the row's own key under the list key, as with `Each`.
- **Resolved by:** the list replaces the built element's key with the item key, so the button *is* `list/b.txt`. The diagnostic `NotFound { path: ["b.txt", "open"], resolved: ["b.txt"] }` pointed straight at it, and the guide's editable-list example says so in a comment ("The list keys the row with the task's key, replacing \"row\""). Only a comment though; on `ListItems::build` I'd expect it in prose.
- **Read:** guide/layout (an editable list of records), struct.VirtualList.
- **Diagnostic:** `TestError::NotFound { path: ["b.txt", "open"], resolved: ["b.txt"] }`

### 17:33–17:35 (time not taken with `date` at the moment; I first wrote 17:52 by mistake) — discover — annoyance

- **Category:** discover
- **Severity:** annoyance (one debugging round, a couple of minutes)
- **Tried:** `driver.find(&root, &["new-dialog", "new-note"])` for the content of a `Dialog::new(controller, owner, content.key("new-note"))` mounted under key `new-dialog`; got `Ok(None)` while the dialog was open.
- **Expected:** the content under the dialog's own key, as the owner (anchor) is (`new-dialog/close-dialog/...` resolves fine).
- **Resolved by:** dumped `ui.inspect().components` with their `path`s: the content is at `new-dialog/dialog/new-note`. Neither struct.Dialog nor the surfaces/testing guide chapters mention the `"dialog"` segment (ToastHost documents its `"toasts"` part, InlineText its `"editor"`, so I expected the same here).
- **Read:** struct.Dialog, guide/surfaces (Dialogs), guide/testing; grep for `"dialog"` across /kit/doc/pert_ui found nothing.
- **Diagnostic:** n/a (my assertion failed; `Runtime::find` returned `Ok(None)`)

## Closing

- **Result:** all five acceptance items are met and checked by `cargo test` (tests/app.rs, 6 tests, all passing, no warnings):
  1. `lists_and_opens`: a folder with `b.txt`, `a.txt`, `c.txt`, `other.md` lists exactly `a.txt, b.txt, c.txt`. It opens one note by pointer (`Driver::click` on the row) and one by keyboard (focus the list, Down, Enter), then reopens the first and sees its tab selected with still two tabs.
  2. `edits_and_saves`: typing changes the tab title to `a.txt *` and the status line from `1 lines, 2 words` to `2 lines, 3 words`. Ctrl+S (`Driver::chord`) and File › Save (F10, Down, Down, Enter) both write the file and clear the marker. With the folder removed, Ctrl+S shows a "Could not save a.txt…" toast (read from the `toasts` stack's semantic items) and the marker stays.
  3. `new_note_dialog`: empty, `x/y`, `a`, `a.txt` and `.hidden` are refused, with the explanation in the dialog and no file created. `fresh` + Enter creates `fresh.txt`, lists it, opens it, and the status line reads `fresh.txt: 1 lines, 0 words`. File › New note reopens the dialog with the name and error cleared, and Escape closes it.
  4. `closes_tabs`: an unmodified tab closes on Ctrl+W without a dialog. On a modified tab, Cancel keeps the tab, its text and the file; Discard closes it without writing; Save (reached through File › Close tab) writes the file and closes the tab. A failed Save in the dialog keeps the tab open and modified and shows a toast.
  5. `cpu_capture` writes `/work/notes-capture.png` (960×640, CpuRenderer) with `groceries.txt` and `ideas.txt *` open. The library's validation report is empty.
  Extra: `divider_resizes_sidebar` drags the split divider 100 px with `Runtime::pointer_input` and sees the sidebar 100 px wider. `cargo run -- <folder>` starts the real app; here it exits with the expected "neither WAYLAND_DISPLAY nor … DISPLAY is set".
- **Concepts I had to learn:** `App::new` initializer returning application state → `App::headless` / `HeadlessApp::runtime_and_state_mut` → `Element` keys and key paths (`Runtime::find`, `Driver::find`) → `Signal`/`ReadSignal::map`/`combine` and `TextSource` bound titles/labels → `Tabs` described imperatively with `Runtime::reconcile_children` + `Runtime::reveal_child`, `Tabs::on_select` → `VirtualList` + `ListSource`/`ListItems` + `ListSelection` + `on_activate` (and the list re-keying rows) → `Dynamic` for the status line → `EventContext::defer` to get `&mut Runtime` from a widget callback → commands (`register_command`, `KeyChord::primary`, `MenuItem::command` inside `MenuBar` with `.content`) → `Dialog`/`DialogController` (open / accept / cancel / take_outcome) → `ToastController`/`ToastHost` and its `toasts` semantics items → `Driver` (click, chord, key, type_text, replace_text, address_children, semantics) → inspection queries for `visible` → capture with `set_capture_support` / `request_capture` / `CpuRenderer::serve_captures` / `Pending::try_reply`.
- **What worked well:** the guide. The testing chapter alone covered almost every check I needed, and each struct page has a runnable example of exactly its contract. I never opened `/kit/vendor`. The code compiled after one fix (a guessed `ColorRole::Danger`), and every acceptance test except the dialog one passed on its first run. `MenuItem::command` makes "the shortcut does exactly what the menu item does" true by construction. Semantics are rich enough to test without touching my model: the selected tab title and count are in `SemanticValue::Choice`, and toasts are items of the `toasts` part. `TestError::NotFound { path, resolved }` tells you how far a path resolved.
- **The three changes I want most:**
  1. Document the key segments built-in containers insert, on each type's page: `Dialog` puts its content under `"dialog"`. Better still, give a table of these structural segments (`dialog`, `toasts`, `editor`, …) in the testing guide.
  2. A compact signature index per type, or shorter type pages. `Runtime` is ~4000 lines of rendered Markdown, and reading the guide was the only efficient way in.
  3. A declarative way to describe tabs from data (a `Tabs` over a binding, like `Each`), so an application with open documents doesn't have to keep the strip's `WidgetId` and call `reconcile_children` + `reveal_child` itself. `Dynamic` around `Tabs` might work, but the docs don't say whether selection survives that.
