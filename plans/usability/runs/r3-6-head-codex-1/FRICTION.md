# Friction log

Copy this file to `/work/FRICTION.md`. Add one entry per event, in time
order, as it happens. When something costs you time, run `date +%H:%M` at
that moment and write the entry before you continue; do not estimate a
time. If you do write an entry later, give it the time you write it and add
`(written later)` after the time.

## Entries

### 16:49 — understand — annoyance

- **Tried:** Read `Tabs::selected` to switch to a newly opened or existing note.
- **Expected:** A selected-key builder that could follow an application signal on redescription.
- **Resolved by:** The page explicitly says selection is mount-only; `Tabs::on_select` documents `Runtime::reveal_child` as programmatic navigation. New tabs need a later flush before their mounted IDs exist.
- **Read:** `struct.Tabs.html`, `struct.EventContext.html#method.defer`, `struct.Runtime.html#method.reveal_child`.


### 16:50 — compile — annoyance

- **Tried:** First `cargo check --offline`, using the prelude for the window layout and an assumed `KeyChord::ctrl` constructor.
- **Expected:** Split types in the prelude and a Control shortcut constructor.
- **Resolved by:** Importing `pert_ui::windowing::*`; reading `KeyChord` and `Modifiers`, which document constructing explicit modifiers. The App error conversion diagnostic gave the exact fix.
- **Read:** `guide/layout/index.html`, `struct.KeyChord.html`, `struct.Modifiers.html`.
- **Diagnostic:** `E0599: no associated function or constant named ctrl found for struct pert_ui::KeyChord`; `E0433: cannot find type SplitPane in this scope` (also SplitAxis and SplitSizing); `E0308: expected Result<_, AppError>, found Result<Editor, RuntimeError>`.

### 16:53 — runtime — annoyance

- **Tried:** Added a pointer drag check, locating the divider midway between the sidebar bounds and the Dynamic editor bounds before any note was open.
- **Expected:** A 48-pixel drag would expand the sidebar by 48 pixels.
- **Resolved by:** Public bounds showed the empty Dynamic/Tabs region had width 0. Repeating the drag after opening a note passed, expanding the sidebar by 48 pixels. The check now exercises the populated editor. No library source was read.
- **Read:** `struct.SplitPane.html`, `struct.Runtime.html#method.pointer_input`, `struct.PointerEvent.html`, `enum.PointerKind.html`.
- **Diagnostic:** `assertion failed: d.runtime().bounds(&files)?.unwrap().width() > left.width() + 40.0`.

## Closing

- **Result:** All five acceptance items met by `cargo test --offline`. The single headless test starts the actual `App` with `App::headless`, drives it with `Driver` and host pointer events, and inspects UI semantics and files. It verifies sorted filtering, pointer and keyboard opening, selected-tab reuse, editor changes and counts, both save routes, failure notification and dirty retention, rejected and accepted names, both New routes, all close decisions including failed Save, unmodified close, empty status, and divider dragging with notes open. CPU capture: `/work/notes-editor.png`, two notes open and Alpha modified. The empty tab strip has zero width before a note opens, so the pointer divider check runs with a note open. No native display was available; no native run was attempted. No library source was read.
- **Concepts I had to learn:** Keyed element trees; runtime-owned signals and derived bindings; retained Dynamic descriptions; command registration shared by shortcuts and menu items; ListSource/ListItems and list selection; retained tab selection and reveal navigation; deferred work across flushes; modal controllers and consuming terminal outcomes before reopening; Driver input/settling and semantic inspection; CPU capture requests and serving replies.
- **What worked well:** The guide's complete examples made menus, dialogs, headless input, and CPU capture straightforward. All APIs needed were public and documented. The first application check required only import, shortcut-constructor, and error-conversion fixes. Headless input and inspection worked without OS services. Explicit dialog outcome documentation avoided reuse errors. File errors can be handled in callbacks without losing editor text.
- **The three changes I want most:**
  1. A documented recipe or controller for opening and selecting dynamically added tabs in one action, including the necessary mount/flush timing.
  2. A small notes/document editor guide showing per-document signals, dirty titles, active-document status, and shared menu/shortcut commands together.
  3. A Driver drag helper and an example of inspecting divider bounds; clarify the zero-width empty Tabs behavior in split layouts.
