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

### 16:50 (time corrected: originally written between 16:44 and 16:50 with a guessed time instead of `date`) — discover — annoyance

- **Category:** discover
- **Severity:** annoyance
- **Tried:** Looking for how a pointer click on a list row "opens" an item (the sidebar must open a note by pointer, or by arrows + Enter).
- **Expected:** An activation callback for double-click / click on a `VirtualList` row, like `on_activate` for Enter.
- **Resolved by:** `VirtualList::on_activate` documents only Enter; a primary press only selects. `on_selection_change` cannot tell a click from an arrow key (both `ChangeOrigin::User`). Plan: make each row's content a frameless `Button` that opens on click, and keep `on_activate` for Enter.
- **Read:** struct.VirtualList.html (description, on_selection_change, on_activate), guide/layout "An editable list of records".
- **Diagnostic:** n/a

### 16:50 (time corrected: originally written between 16:44 and 16:50 with a guessed time instead of `date`) — compile — annoyance

- **Category:** compile
- **Severity:** annoyance
- **Tried:** `App::new(...).size(Size::new(900.0, 600.0)?)` in a builder function returning `App<Notes>`; `SplitSizing::first(Length::from(220u32))`; `SplitPane` from the prelude.
- **Expected:** `App::size` to be a plain builder like the other `App` methods; `Length: From<u32>` (the guide writes `Length::from(240)`, so the integer type is invisible); `SplitPane` in the prelude since the guide's getting-started text names it next to `Tabs`.
- **Resolved by:** dropped `size` (default 960x640 is fine), used a `u16` constant, imported `pert_ui::windowing::{SplitAxis, SplitPane, SplitSizing}`.
- **Read:** struct.App.html#method.size, guide/layout.
- **Diagnostic:** `E0308 expected App<Notes>, found Result<App<Notes>, RuntimeError>`; `E0277 the trait bound pert_ui::Length: From<u32> is not satisfied` (the compiler helpfully names `From<u16>`); `E0433 cannot find type SplitPane`.

### 16:50 (time corrected: originally written between 16:44 and 16:50 with a guessed time instead of `date`) — discover — slow

- **Category:** discover
- **Severity:** slow
- **Tried:** Finding, for a test, (a) how to read a `Tabs` strip's titles and selected tab, (b) the address of a `VirtualList` row's content, (c) the addresses of a `Dialog`'s content and of `MenuBar` rows.
- **Expected:** The `Tabs`/`Tab`/`MenuBar`/`Dialog` pages to have an "Inspection and tests" section like `ToastHost` and `Table` do.
- **Resolved by:** Wrote a probe test that walks `Runtime::children` and prints `Driver::semantics` of every node. Found: the strip's semantics are `TabList` with one `SemanticItem` per title (name, `selected`); the list replaces my row element's key with the item key, so the row *is* `list/<key>` (my `find(list, ["b.txt","open"])` failed with `NotFound { resolved: ["b.txt"] }`, which was a clear diagnostic); the menu bar title is keyed `title/file` and its rows are semantic items of `title/file/menu/actions/items`; `Driver::find` skips the structural `content`/`anchor`/`dialog/panel/scroll/body` wrappers. Also, `Driver::address_children(&tabs)` returned nothing, while `find(tabs, [name, "editor"])` works.
- **Read:** guide/testing, struct.Tabs.html, struct.Tab.html, struct.MenuBar.html, struct.Dialog.html, struct.ToastHost.html (the good example).
- **Diagnostic:** n/a

### 16:50 (time corrected: originally written between 16:44 and 16:50 with a guessed time instead of `date`) — docs-wrong — slow

- **Category:** docs-wrong / missing
- **Severity:** slow
- **Tried:** Clicking File › Save in a headless test: `Driver::find(&bar, &["title/file"])` (the key shown by `Runtime::children`), then `Driver::click_item(&items, 1)` on the open menu's rows.
- **Expected:** `Driver::click_item`'s page names "a menu command or a tab title" as the kind of item it clicks, so I expected menu rows to be clickable.
- **Resolved by:** `find` reports `NotFound { path: ["title/file"], resolved: [] }` (bar titles are not address children: `address_children(&bar)` lists only `main`), so I used `Runtime::find_descendant(&bar, "title/file")`. `click_item` then failed with `ItemWithoutBounds { key: "items", index: 1 }`: menu rows report `bounds: None`. Reading further down `click_item`, only table titles and toasts give bounds. I open the menu by clicking the title (real pointer input) and choose the row with Down/Enter. A test cannot click a menu row with the pointer through the driver.
- **Read:** testing/struct.Driver.html#method.click_item, struct.Menu.html, struct.MenuBar.html.
- **Diagnostic:** `TestError::ItemWithoutBounds { key: "items", index: 1 }`

### 16:50 (time corrected: originally written between 16:44 and 16:50 with a guessed time instead of `date`) — understand — annoyance

- **Category:** understand
- **Severity:** annoyance
- **Tried:** `Driver::find(&root, &["new-dialog", "new-form", "name"])` for the New note dialog's field.
- **Expected:** The dialog content addressed directly under the dialog's key, as the anchor content is (`new-dialog/close-dialog/...` works without an `anchor` segment).
- **Resolved by:** `NotFound { resolved: ["new-dialog"] }`; listing `address_children` showed `["close-dialog", "dialog"]`, so the content is at `new-dialog/dialog/new-form/...`. The anchor adds no segment but the dialog surface adds `dialog`; the `Dialog` page does not give the addresses.
- **Read:** struct.Dialog.html, guide/surfaces "Dialogs".
- **Diagnostic:** `TestError::NotFound { path: ["new-dialog", "new-form", "name"], resolved: ["new-dialog"] }`

### 16:50 (time corrected: originally written between 16:44 and 16:50 with a guessed time instead of `date`) — docs-wrong — annoyance

- **Category:** docs-wrong
- **Severity:** annoyance
- **Tried:** `Driver::click_item(&tabs, 0)` to select a tab by its title with the pointer.
- **Expected:** It to work, since `click_item`'s documentation lists "a tab title" among its items.
- **Resolved by:** `TestError::ItemWithoutBounds { key: "tabs", index: 0 }`: tab titles, like menu rows, report no bounds. Switched tabs by reopening the note from the list (which the app turns into a tab switch).
- **Read:** testing/struct.Driver.html#method.click_item.
- **Diagnostic:** `ItemWithoutBounds { key: "tabs", index: 0 }`

### 16:51 (written later) — understand — annoyance

- **Category:** understand
- **Severity:** annoyance
- **Tried:** Planning how opening a note adds a tab and selects it. My first idea was a `Dynamic` that describes the `Tabs` strip from a signal, followed by `EventContext::defer` → `Runtime::reveal_child`.
- **Expected:** A documented way to select a tab added in the same step, e.g. `Tabs::selected` working after mount, or a note on `reveal_child` about tabs not yet described.
- **Resolved by:** `defer` runs "before component evaluation", so a `Dynamic` would not have described the new tab when `reveal_child` ran. Instead, a strip mounted with `Tabs::new([])` gets its children from `Runtime::reconcile_children` inside the deferred operation, then `reveal_child`. This worked first time. A `DialogController` also has to `take_outcome` before it can open again: after Escape the slot stays filled and `open` returns `DialogBusy`. The page says so, so I handled it up front.
- **Read:** struct.EventContext.html#method.defer, struct.Tabs.html (`selected`, `on_select`, `reveal_child` paragraph), struct.DialogController.html.
- **Diagnostic:** n/a

## Closing

- **Result:** All five acceptance items are met, and `cargo test` shows it (`tests/editor.rs`, 5 tests, all passing). Every test starts the real `App` with `app(dir).headless()` and drives it with `pert_ui::testing::Driver`: clicks, typed text, chords, keys and semantics/inspection reads.
  1. `lists_notes_and_opens_tabs`: the list shows only `alpha.txt, beta.txt, gamma.txt` (no `readme.md`), in order. It opens two notes with the pointer, and reopening `beta.txt` selects its tab without adding one. It also opens a note with the arrow keys and Enter, and moves the divider with its arrow keys.
  2. `edits_and_saves`: typing adds the `•` marker and updates the status line (`beta.txt: 3 lines, 7 words`). Ctrl+S and File › Save both write the file and clear the marker. Removing the folder makes the next save fail: a "Could not save beta.txt…" toast appears and the marker stays.
  3. `new_note_dialog`: an empty name, `a/b` and an existing name (`alpha` and `beta.txt`) are refused, the explanation shows in the dialog and no file is created. `delta` creates `delta.txt`, adds it to the list and opens it. File › New note reopens the dialog cleared.
  4. `closing_tabs`: Cancel (and Escape) keep the tab and its text; Discard (reached through File › Close tab) closes without writing; Save writes, then closes. A failed Save keeps the tab and shows a toast. An unmodified tab closes with no dialog (status stays `Idle`), and closing the last tab empties the status line.
  5. `capture_two_notes_one_modified` writes `/work/capture.png` (960×640, CPU renderer): two tabs, `beta.txt •` modified.
  Not verified: the native window, because there is no display; `cargo run` is untested. I did not have to read any library source.
- **Concepts I had to learn:** `App::new` initializer → application state `S` → `App::headless`/`HeadlessApp::runtime_and_state_mut`; keyed `Element`s and key-path addresses (`Driver::find`, `address_children`); `Signal`/`ReadSignal::map`/`combine` derived bindings (tab title with marker, status line); `TextSource` accepting a binding (bound tab titles and labels); `Dynamic` for a child described from a binding; `VirtualList` + `ListSource`/`ListItems` + `ListSelection` + `on_activate`; `Tabs` with raw elements, `Runtime::reconcile_children`, `Runtime::reveal_child`, `Tabs::on_select`; `EventContext::defer` for runtime access from callbacks; commands (`register_command`, `KeyChord::primary`, `MenuItem::command`) shared by menu and shortcut; `MenuBar::content`; `Dialog`/`DialogController` sessions and the outcome slot (`take_outcome` before reopening); `ToastController`/`ToastHost` and reading toasts from the stack's semantic items; `SplitPane`/`SplitSizing`; the CPU capture (`set_capture_support`, `request_capture`, `CpuRenderer::serve_captures`, `Pending::try_reply`, `encode_png`).
- **What worked well:** The testing guide is excellent. Every step I needed (clicks, chords, settling, list rows, close policy, CPU capture) had a runnable example, and the capture example worked on the first try. Commands made "the shortcut does exactly what the menu item does" trivial, and the menu even shows the shortcut. Bound tab titles (`Tab::new(key, ReadSignal<String>, ...)`) made the modified marker one `combine`. `TestError::NotFound { path, resolved }` showed exactly where a lookup stopped. The `ToastHost` page's "Inspection and tests" section is the model the other surfaces should follow. The library did what its documentation said every time I relied on it; nothing surprised me at runtime.
- **The three changes I want most:**
  1. Make menu rows and tab titles clickable from tests: give their semantic items `bounds` (as `Driver::click_item`'s own doc implies), or correct that doc. Today a test cannot click File › Save or a tab title with the pointer.
  2. Give `Tabs`, `MenuBar` and `Dialog` an "Inspection and tests" section like `ToastHost`'s: what their semantics report (TabList items, the `title/<key>` parts) and the addresses of their parts (`<dialog>/dialog/<content>`; bar titles are not address children). A small `Driver` helper to list a subtree's addresses would have replaced my probe test.
  3. A pointer "activate" for list rows (double-click → `on_activate`, or an option for single-click activation), so a sidebar that opens items needs no `Button` inside each row. Minor: `App::size` as an infallible builder (or one taking integers), and `Length: From<u32>`.
