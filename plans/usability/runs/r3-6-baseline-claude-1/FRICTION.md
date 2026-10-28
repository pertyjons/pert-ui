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

### 16:54 — understand — annoyance

- **Category:** understand
- **Severity:** annoyance
- **Tried:** Finding how a sidebar list opens an item "with the pointer" as well as with arrows + Enter.
- **Expected:** An activation callback on `VirtualList` for both Enter and click/double-click.
- **Resolved by:** `VirtualList::on_activate` is Enter only; a primary press only selects, and `on_selection_change` reports both a click and an arrow key as `ChangeOrigin::User`, so the two cannot be told apart. I plan frameless `Button` rows (`Button::frameless`) with `item_tab_stops(false)` plus `on_activate` for Enter.
- **Read:** `struct.VirtualList.html` (description, `on_activate`, `on_selection_change`, `item_tab_stops`), `struct.ListSelectionChange.html`, `enum.ChangeOrigin.html`, guide `layout#an-editable-list-of-records`.

### 16:54 — discover — annoyance

- **Category:** discover
- **Severity:** annoyance
- **Tried:** Finding how to make `Tabs` follow a changing set of open documents.
- **Expected:** A binding-driven constructor (like `Each`) for tabs, or a guide section on open/close of tabs.
- **Resolved by:** `Tabs::reorderable`'s example shows `cx.defer(|ui| ui.reconcile_children(&strip, tabs))` with `Tab::into_element`; `Runtime::reveal_child(&tabs, key)` selects. A `Dynamic` around `Tabs` might also work, but the docs do not say whether a redescribed `Tabs` keeps its selection that way. Going with `reconcile_children`.
- **Read:** `struct.Tabs.html`, `struct.Tab.html`, `struct.Dynamic.html`, guide `layout#retained-tabs-and-resizable-edges`.

### 17:02 — discover — annoyance (written later, events 16:58–17:00)

- **Category:** discover
- **Severity:** annoyance
- **Tried:** Getting the mounted root `WidgetId` out of `App::headless`, to find widgets by key.
- **Expected:** `HeadlessApp::root()` or similar.
- **Resolved by:** `HeadlessApp` only has `runtime`, `main_window`, `admit_close`, `shutdown`; `NativeWindowId` only has `id()`. I copied the guide's pattern of an `Rc<RefCell<Option<WidgetId>>>` slot that the initializer fills (`build_app_reporting`). Works, but every test of an `App` has to do this.
- **Read:** `testing/struct.HeadlessApp.html`, `struct.NativeWindowId.html`, `struct.Runtime.html` method list, guide `testing#the-close-policy`.

### 17:02 — runtime — slow (written later, event ~16:59)

- **Category:** runtime
- **Severity:** slow
- **Tried:** A test helper that picks File menu rows with F10, a fixed number of ArrowDown presses and Enter (as the surfaces guide example does for the first row).
- **Expected:** The same key sequence picks the same row every time.
- **Resolved by:** The same sequence picked "Close tab" once and "New note" later in the same test, and the click on the close dialog's Save then failed with `NotRevealed { key: "save", refusal: ModalBlocked }`. That error made me look at the wrong dialog first. I did not find in `MenuBar`/`Menu` docs how the initial highlight is chosen when a menu reopens. Fix: the helper reads the open menu's `Semantics::items[..].selected` from inspection and presses Down until the wanted row is highlighted.
- **Read:** `struct.MenuBar.html`, guide `surfaces#menus`, `struct.SemanticItem.html`.

### 17:02 — missing — slow

- **Category:** missing
- **Severity:** slow
- **Tried:** Asserting in a headless test that the "Could not save …" toast is shown, through `Runtime::inspect()`.
- **Expected:** The toast's message as a semantic name/description, as labels and banners have.
- **Resolved by:** The `ToastHost` stack component (`key == "toasts"`) reports only `visible_count`, `paused`, `dismiss_label` and a `Notifications` role with a number value; the message text is not in inspection at all. Switched the save-failure report to a `Banner` (role `Alert`, message as name).
- **Read:** `struct.ToastHost.html`, `struct.Toast.html`, `struct.ToastController.html`, `struct.ComponentDetails.html`.

### 17:02 — source — annoyance

- **Category:** source
- **Severity:** annoyance
- **Tried:** Confirming that no inspection property carries a toast's text, since the docs do not say what a toast reports.
- **Expected:** —
- **Resolved by:** `/kit/vendor/pert-ui-runtime-0.3.75/src/notifications.rs`, `impl Component for ToastStack { fn inspect }`: only count, scroll offset, paused and dismiss label. `Banner::inspect` names the message, so I use that.
- **Read:** the source above.

### 17:03 — understand — annoyance

- **Category:** understand
- **Severity:** annoyance
- **Tried:** Reading the sidebar's rows from a `VirtualList` in a test.
- **Expected:** The guide's `Each` section explains row addresses; I assumed `VirtualList` rows sit directly below the list's key (`find(&sidebar, &["a.txt"])`), and that worked. But nothing says whether `Runtime::children` of a virtual list is in display order, so my helper sorts the rows by `bounds().y()`.
- **Resolved by:** Sorting by bounds. Inspection paths (`["app", …, "sidebar", "a.txt"]`) confirmed the addresses.
- **Read:** guide `testing#list-rows`, `struct.VirtualList.html`, `struct.Runtime.html#method.children`.

### 17:03 — understand — annoyance

- **Category:** understand
- **Severity:** annoyance
- **Tried:** `Runtime::find_descendant(&root, "save")` to reach the close dialog's Save button, when the File menu also has a row keyed "save".
- **Expected:** The docs to say which match wins when several descendants share a key.
- **Resolved by:** Not documented, so I switched to a path below `close-form` (`find(&form, &["actions", "save"])`). The `ModalBlocked` failure I first blamed on this was really the menu entry above (the wrong row opened New note).
- **Read:** `struct.Runtime.html#method.find_descendant`.

## Closing

- **Result:** All five acceptance items are met, each shown by a headless test in `tests/notes.rs` (`cargo test`, 5 passed). The tests start the real `App` with `App::headless` on a temporary folder, drive it only through `Driver` (clicks, typing, keys, `Ctrl+…` chords, F10 menu navigation) and read what it shows through `Runtime::inspect`/`query` semantics.
  1. `lists_notes_in_order_and_reopening_switches_tab`: only `.txt` files are listed, sorted (a `.md`, a `.json` and a directory named `folder.txt` are excluded). Two notes open by click and a third by Home/ArrowDown + Enter; reopening selects the existing tab. Also checks that the split divider resizes the sidebar.
  2. `typing_marks_modified_and_saving_clears_it`: typing sets the `*` marker and changes the status counts. Ctrl+S and File → Save write the file and clear the marker. After the folder is removed, a save shows "Could not save one.txt …" in the notification banner and the marker stays.
  3. `new_note_dialog_refuses_invalid_names`: an empty name, `a/b` and `existing` are each refused with the explanation shown and no file created. `groceries` creates `groceries.txt`, lists it and opens it. File → New note reopens the dialog with the field and error cleared.
  4. `closing_tabs`: on a modified tab, Cancel (and Escape) keep the tab and its text, Discard closes it without writing, and Save writes then closes. An unmodified tab closes with no dialog, and a failed save from the dialog keeps the tab open.
  5. `capture_two_notes_one_modified` writes `/work/capture.png` (900×600, CPU renderer): two tabs, `ideas.txt *` modified, sidebar, status line.
  The native binary (`cargo run -- <folder>`) builds but cannot open a window here: there is no display, and winit reports that neither WAYLAND_DISPLAY nor DISPLAY is set.
  One change from the task's wording: save failures appear in a `Banner` (which has a Dismiss button) rather than a toast, because a toast's text cannot be read in a headless check (see 17:02).
- **Concepts I had to learn:** `Element` keys and key paths → `Signal`/`ReadSignal` with `map`/`combine` → `App::new` initializer on a `Runtime` → registered commands (`register_command` + `KeyChord::primary`) shared by `MenuItem::command` and shortcuts → `EventContext::defer` to reach the `Runtime` from a callback → `Tabs` children described by `reconcile_children` and selected by `reveal_child`, plus `on_select` origins → `VirtualList` + `ListSource`/`ListItems` + `ListSelection` → `Dynamic` for a binding-dependent child → `DialogController` sessions (take the outcome before reopening) → `Banner`/`ToastHost` → headless testing: `App::headless`, `Driver`, inspection semantics, CPU capture.
- **What worked well:** The guide's chapters are dense but accurate, and every snippet compiles. My whole app compiled at the first `cargo build`, and 3 of 5 tests passed at the first run (the 2 failures were my own wrong expectations, plus the menu-navigation helper). The `Driver` with `chord`/`key`/`click`/`replace_text` and full inspection (paths, semantics, `items[].selected` for tabs and menus) made acceptance tests easy. Commands that feed both menu rows and shortcuts give "the shortcut does exactly what the menu item does" by construction. The `TextField` docs say exactly which chords an editor claims, so I knew Ctrl+S/N/W would reach the commands. CPU capture to PNG took ten lines.
- **The three changes I want most:**
  1. Put a toast's message (and action label) in inspection semantics, as `Banner` does, so a test can see what was notified.
  2. A way to get the mounted root (or `find` from the window) from `HeadlessApp`, instead of every test smuggling a `WidgetId` out of the initializer through an `Rc<RefCell<Option<_>>>`.
  3. A binding-driven way to describe a dynamic set of tabs (an `Each`-like `Tabs::from_signal`, or a guide section on "open/close documents in tabs"). Also an activation callback on `VirtualList` for pointer clicks (`on_activate` is Enter only), and a documented rule for which row a reopened menu highlights.
