# Friction log

Copy this file to `/work/FRICTION.md`. Add one entry per event, in time
order, as it happens. When something costs you time, run `date +%H:%M` at
that moment and write the entry before you continue; do not estimate a
time. If you do write an entry later, give it the time you write it and add
`(written later)` after the time.

## Entries

### 17:04 — understand — annoyance

- **Tried:** Looked for a TextArea edit callback to update modified titles and counts.
- **Expected:** An on-change callback alongside on_save.
- **Resolved by:** TextArea::on_save explicitly says it never runs for a plain editor; observe the binding instead. Used text.combine(saved) for titles and text.map for counts.
- **Read:** /kit/doc/pert_ui/struct.TextArea.html#method.on_save; /kit/doc/pert_ui/struct.Tab.html#method.new; guide/state.

### 17:05 — compile — annoyance

- **Tried:** Used SplitPane with the prelude and explicit SplitAxis/SplitSizing imports.
- **Expected:** The prelude to contain the split component, like Tabs and VirtualList.
- **Resolved by:** Added explicit pert_ui::SplitPane import, as the layout guide example does.
- **Read:** guide/layout, independent split dividers.
- **Diagnostic:** error[E0433]: cannot find type `SplitPane` in this scope; src/main.rs:192:9; use of undeclared type `SplitPane`.

### 17:06 — discover — annoyance

- **Tried:** Guessed struct.ComponentSnapshot.html while finding inspection fields; pandoc reported “does not exist (No such file or directory)”.
- **Expected:** A snapshot record type with that name.
- **Resolved by:** Followed Semantics links to ComponentInspection and SemanticItem. Tab titles are semantic items within Tabs, with a selected flag, rather than separate widgets.
- **Read:** struct.Semantics.html, struct.ComponentInspection.html, struct.SemanticItem.html.

### 17:06 — compile — annoyance

- **Tried:** Named Semantics as the return type of a Driver helper after importing prelude and testing.
- **Expected:** The testing glob to include the type returned by Driver::semantics.
- **Resolved by:** Explicit pert_ui::Semantics import.
- **Read:** guide/testing, struct.Semantics.html.
- **Diagnostic:** error[E0425]: cannot find type `Semantics` in this scope; src/main.rs:451:50; not found in this scope.

### 17:06 — runtime — annoyance

- **Tried:** Used find_descendant(root, "Alpha.txt") in a test helper after opening Alpha.txt.
- **Expected:** To find the tab's editor by note name.
- **Resolved by:** Scoped the lookup to Tabs; the sidebar row correctly has the same note key in another branch.
- **Read:** guide/testing, reading what the runtime shows.
- **Diagnostic:** test panic: called Result::unwrap() on an Err value: AmbiguousKey("Alpha.txt"), src/main.rs:427:18.

### 17:07 — understand — slow

- **Tried:** After a failed save, searched visible component semantic names for “Save failed”.
- **Expected:** Toast text to appear as a semantic name like a Label.
- **Resolved by:** Inspection shows a visible Notifications component with Number value 1 and visible_count=1, but no message text. Assert the visible notification count instead. Message text is not exposed by this component's semantics.
- **Read:** guide/surfaces notifications; struct.ToastHost.html (next).
- **Diagnostic:** acceptance_through_ui assertion failed at src/main.rs:619:9: no visible component's semantic name contains “Save failed”.

### 17:08 — understand — slow

- **Tried:** Opening notes from VirtualList::on_selection_change, with on_activate for Enter.
- **Expected:** Repeated pointer activation of a selected row to switch back after selecting another tab.
- **Resolved by:** on_activate documents Enter only; selection callbacks cannot report clicking an unchanged selection. Put a Button in each row to open on every pointer activation, set the list active item there, and keep on_activate for keyboard Enter. Rows hold a Weak editor reference to avoid a signal/source ownership cycle.
- **Read:** struct.VirtualList.html#method.on_activate, #method.on_selection_change; struct.ListSelection.html#method.set_active.

## Closing

- **Result:** All acceptance items 1–5 met. `cargo test --offline` passes `tests::acceptance_through_ui` using the library's Driver and inspection, without a display or direct calls to editor actions. `cargo check --offline` passes without warnings. The test covers sorted .txt-only listing, pointer and keyboard opening, tab reuse (including repeated clicks on an unchanged sidebar selection), typing/counts/markers, shortcut and menu saves, validation and creation, all close choices, unmodified close, and failed saves including Save on close. `/work/notes-editor.png` is a 1000×700 CPU capture with Alpha.txt modified and Beta.txt also open. For save errors the test observes the visible notification count; ToastHost does not expose the message text in semantics. No library source was read.
- **Concepts I had to learn:** Keyed Element trees and Runtime/App initialization; signals and derived bindings; retained tabs and reveal_child navigation; ListSource/ListItems and selection versus activation; application commands shared by menus and shortcuts; DialogController outcomes and acknowledgement before reopening; deferred event work; Driver settling and semantic items; CPU capture requests.
- **What worked well:** The guide's runnable examples mapped closely to this task. Tab titles accept live text bindings, so dirty markers required no custom widget. TextArea bindings and derived labels handled live counts. Command-backed menus shared handlers and shortcuts directly. Driver text replacement, chords, inspection and the CPU renderer worked headlessly, and the main acceptance scenario runs in about 0.2 seconds after compilation. Keyed reconciliation preserved text-editor state while adding/removing tabs.
- **The three changes I want most:**
  1. Expose toast message text as semantic items or another documented inspection field, so a headless test can assert the error explanation itself.
  2. Offer a list activation callback covering both pointer activation (including an already-selected row) and Enter; document a standard file-list pattern. A Button per row works but adds ownership and selection wiring.
  3. Put explicit import blocks in testing and layout reference examples for types outside the prelude/testing glob, including Semantics and SplitPane; a compact application recipe joining a dynamic file list, retained tabs and dialogs would reduce page switching.
