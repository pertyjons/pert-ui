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

### 15:42 — docs-wrong — annoyance

- **Tried:** Followed the getting-started dependency instructions.
- **Expected:** A registry dependency matching the rendered version.
- **Resolved by:** Used `pert-ui = "0.3.52"` per the tester brief and crate index version.
- **Read:** `/kit/doc/pert_ui/guide/getting_started/index.html`, “Run your first window”; `/kit/doc/pert_ui/index.html`.
- **Diagnostic:** Guide says “Until a registry release exists, use a local checkout” and shows a path dependency, despite this test providing a registry package.

### 15:43 — compile — annoyance

- **Tried:** First implementation used guessed `Modifiers::CONTROL`/`ALT` constants and a fallible floating-point grow weight.
- **Expected:** Modifier constants analogous to the guide's `PRIMARY`; grow weight similar to other geometry builders.
- **Resolved by:** Read the reference: modifiers have public `control`/`alt` fields, and `Element::with_grow` takes an infallible `u16`. Also followed compiler help to use `PointerEvent::kind()`.
- **Read:** `struct.Modifiers.html`, `struct.Element.html`, `struct.PointerEvent.html`.
- **Diagnostic:** `E0599: no associated function or constant named CONTROL found for struct Modifiers`; same for ALT. `E0308: expected u16, found floating-point number`; `E0599: no method named ok found for struct pert_ui::Element`; `E0277: the ? operator cannot be applied to type pert_ui::Element`. `E0616: field kind of struct pert_ui::PointerEvent is private` (help correctly suggests `kind()`).

### 15:45 — understand — slow

- **Tried:** Chose `ListFilter` and `CollectionFilter` to keep single selection valid while searching.
- **Expected:** Filtering to clear or move selection when the selected task disappears.
- **Resolved by:** Their reference pages explicitly retain hidden selection (and only move it when focus is inside the collection). Added an application wrapper that observes tasks, query and active selection, then normalizes selection in `deliver_notifications`, using the documented `Notifier` pattern so it does not write during update.
- **Read:** `struct.ListSelection.html`, `struct.ListFilter.html`, `struct.CollectionFilter.html`, `struct.Notifier.html`.
- **Diagnostic:** No compiler error; this is a documented policy that requires additional application work for this task's selection policy.

### 15:46 — runtime — slow

- **Tried:** Headless pointer selection, Alt+Up/Down reordering, then Enter, text input, Enter to rename via `DraftEditor`.
- **Expected:** The vector's second task to become “Renamed”.
- **Resolved by:** Read `Component::retain_initial_children`: the title retained its original draft controller, while normal child reconciliation replaced the editor with a newly constructed controller after reorder. Returning true keeps the editor and owner on the same controller.
- **Read:** `struct.DraftEditor.html` example and owner contract; `struct.VirtualList.html` source replacement contract; `trait.Component.html#method.retain_initial_children` and `reconcile`.
- **Diagnostic:** Test assertion failed: `left: "Beta", right: "Renamed"` after the Enter commit.

## Closing

- **Result:** All five acceptance items met. `cargo test --offline` passes two headless simulated-input tests. They exercise Enter/Add submission, empty refusal, checkbox on/off, button/Delete removal, button/Alt-arrow reorder, pointer/arrow single selection, Enter rename commit and Escape cancellation, case-insensitive filtering, hidden/no-match selection handling, Ctrl+N focus, and the actual status label's inspection semantics. Inputs go through `Runtime`; tests only read application data. Native presentation was not attempted because there is no display.
- **Concepts I had to learn:** Keyed elements and signals; retained/derived bindings; `ListItems`/`ListSource` and bounded `VirtualList`; `ListSelection`; `ListFilter`/`CollectionFilter`; `DraftEditorController` owner lifecycle; command input versus component key input and native modifiers; event-phase notifications; reconciliation of controller-owned children; headless geometry and semantic inspection.
- **What worked well:** The crate index clearly explains facade re-exports and features. Guide examples and complete item reference contracts were sufficient without vendor source. The draft editor example gave a practical starting point. The diagnostics guide explicitly says to test shortcuts with native modifiers and `command_key_input`, which prevented testing the wrong input path. Headless runtime input, layout, inspection and painting were straightforward and fast.
- **The three changes I want most:** (1) Give the getting-started chapter a registry dependency option consistent with the published version. (2) Extend the DraftEditor example with a row in a changing keyed list, showing `retain_initial_children` so controller and child stay paired across source replacement. (3) Offer a documented collection-filter selection policy/example that clears or relocates hidden selection even when the search field owns focus.
- **Source reading:** None. Compiler diagnostics included vendor locations, but no library source file was opened.
