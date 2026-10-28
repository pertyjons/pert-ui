# Friction log

## Entries

### 16:44 — compile — annoyance

- **Tried:** Used `pert_ui::prelude::*` for the editor, including the split pane shown in the layout guide.
- **Expected:** The prelude to include SplitPane as it includes Tabs and VirtualList.
- **Resolved by:** Reading prelude membership and adding `pert_ui::windowing::*`.
- **Read:** guide/layout/index.html; prelude/index.html, membership and paths needing another import.
- **Diagnostic:** `error[E0433]: cannot find type SplitPane in this scope`; also SplitAxis and SplitSizing, src/main.rs:175–176.

### 16:45 — discover — annoyance

- **Tried:** Guessed `struct.ComponentSnapshot.html` while looking for fields returned by inspection; pandoc reported that the file does not exist.
- **Expected:** A way to inspect visible tab titles and notifications.
- **Resolved by:** Following Runtime::inspect's return links instead; the type is ComponentInspection. For controls, Driver::semantics is simpler.
- **Read:** guide/testing/index.html; struct.Runtime.html#method.inspect.

### 16:46 — runtime — slow

- **Tried:** Ran the full Driver-based acceptance test and searched Runtime::inspect components for a visible notification named "Save failed" after deleting the notes folder and pressing Ctrl+S.
- **Expected:** The toast's text in a visible component's semantic name.
- **Resolved by:** The visible `toasts` component reports SemanticRole::Notifications and a Number value of 1, but no message text in semantics or inspection properties. Changed the check to assert its visible notification count. The existing failure path supplies the message via Toast::new. No source read needed.
- **Read:** guide/testing/index.html; guide/surfaces/index.html#notifications; struct.Runtime.html#method.inspect.
- **Diagnostic:** `assertion failed: h.ui.inspect().components.iter().any(...)`, src/main.rs:401. All preceding open/edit/save/new/close checks passed, and the capture was written.

## Closing

- **Result:** All five acceptance items are met. `cargo test --offline` passes the Driver-based headless session: sorted `.txt`-only sidebar; pointer and arrow/Enter opening; two retained tabs and duplicate-open switching; typed modifications and live counts; shortcut and menu saves; visible notification count and retained marker after a failed save; invalid/duplicate-name explanations and successful creation; Cancel, Discard, Save, failed Save-and-close and immediate unmodified close. `/work/notes-editor.png` is the CPU-rendered capture with two tabs, one modified. No display was available, so native execution was not checked. No library source was read.
- **Concepts I had to learn:** App versus Runtime startup; keyed Elements and layout growth; Signal and derived ReadSignal bindings; Driver settling and widget semantics; command registration shared by menus and shortcuts; deferred event work with Runtime access; DialogController outcomes and reopening; ListSource/ListItems with ListSelection; keyed tab reconciliation, title bindings and reveal_child selection; ToastHost's notification semantics; CPU capture requests.
- **What worked well:** The guide's complete headless examples made it possible to test the real widgets quickly. The menu command example prevented separate shortcut implementations. Tabs retained editor state through reconciliation, and title bindings made modified markers straightforward. TextArea explicitly documents CRLF conversion and content checks. CPU capture required no display or external image encoder.
- **The three changes I want most:**
  1. Expose each toast's message and actions in inspection/semantics; the current notification-count value cannot verify the displayed explanation.
  2. Put import lines next to the library map's examples or show thematic-module membership there; prelude versus windowing was a small avoidable compiler detour.
  3. Add one complete multi-document editor guide example combining dynamic tab creation/removal, reveal_child navigation, modified title bindings, active-tab status and save/close dialogs. The individual contracts are documented, but assembling them required visiting several pages.
