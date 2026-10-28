# Friction log

## Entries

### 17:11 — understand — annoyance

- **Tried:** Looked for pointer activation alongside keyboard activation on `VirtualList`.
- **Expected:** `on_activate` to handle both clicking and Enter.
- **Resolved by:** The API explicitly says activation is Enter-only; I will use button content for pointer opening and retain list selection/on_activate for arrow keys and Enter.
- **Read:** `guide/layout/index.html`, `struct.VirtualList.html` sections navigation and `on_activate`.

### 17:12 — compile — annoyance

- **Tried:** Imported `pert_ui::prelude::*` and built a `SplitPane`.
- **Expected:** The prelude to include the split types alongside Tabs and other layout components.
- **Resolved by:** Added root imports for SplitPane, SplitAxis and SplitSizing, as the layout chapter's split example does.
- **Read:** `guide/layout/index.html`, independent split dividers example.
- **Diagnostic:** `error[E0433]: cannot find type SplitPane in this scope`, also SplitAxis and SplitSizing at src/lib.rs:184–185.

### 17:13 — runtime — slow

- **Tried:** `Driver::click_item(&tabs, 0)` to select the first tab in the acceptance check.
- **Expected:** Tab titles to expose click bounds, since the Driver documentation names a tab title as an example semantic item.
- **Resolved by:** Use focus on the actual tab strip and Home keyboard navigation; the Driver page's later paragraph says only some built-in components give item rectangles.
- **Read:** `testing/struct.Driver.html#method.click_item`, `struct.Tabs.html` keyboard navigation.
- **Diagnostic:** `Error: ItemWithoutBounds { key: "tabs", index: 0 }`.

### 17:13 — understand — annoyance

- **Tried:** Inspected the first CPU capture with a grown TextArea inside a Column.
- **Expected:** The editor to fill its tab horizontally as well as vertically.
- **Resolved by:** Added `Column::align(Align::Stretch)` to the editor page and body. `with_grow(1)` only controls the main axis.
- **Read:** `guide/layout/index.html#growing-shrinking-and-stretching-rows-and-columns`, `struct.Column.html#method.align`.


## Closing

- **Result:** All five acceptance items met. `cargo test --offline` passes the headless integration check using the real application's `App::headless` and pert-ui Driver. It checks sorted filtering, pointer and keyboard opening, existing-tab selection, typing and counts, Ctrl+S and File → Save, failed saves, invalid and valid creation, Cancel/Discard/Save close choices, failed Save-and-close, and immediate unmodified close. `/work/notes.png` is a CPU capture with two notes open and Alpha modified.
- **Concepts I had to learn:** App initialization and headless startup; Signal and derived ReadSignal bindings; layout growth versus cross-axis stretching; keyed retained tabs and reconciliation; explicit reveal for tab selection; list sources and selection; command-backed menu items; deferred callbacks with Runtime access; dialog controller outcome lifecycle; semantic inspection and CPU capture servicing.
- **What worked well:** The guide's complete command, dialog, testing and capture examples worked with little adaptation. Reactive tab titles and text-derived labels directly cover modified markers and counts. Headless startup runs the same initializer as the desktop app. No library source reads were necessary.
- **The three changes I want most:** (1) Give tab title semantic items bounds so Driver::click_item can select them. (2) Offer a unified pointer/keyboard list activation callback, or a note-browser example showing the intended pattern. (3) Put a compact prelude/import and grow/stretch reference next to the first application example.
