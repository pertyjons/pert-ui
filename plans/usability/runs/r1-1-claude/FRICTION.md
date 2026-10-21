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

### 15:35 — docs-wrong — annoyance

- **Category:** docs-wrong
- **Tried:** following guide › getting_started › "Run your first window" to add the dependency.
- **Expected:** a `pert-ui = "0.3.52"` line.
- **Resolved by:** ignoring the guide; the README/kit version (0.3.52) works from the registry. The guide says "Until a registry release exists, use a local checkout … `path = "../pert-ui/crates/pert-ui"`", and points at `rust-toolchain.toml` and `cargo run -p pert-ui --example` which a registry user does not have.
- **Read:** `pert_ui/guide/getting_started/index.html`, `/kit/readme/pert-ui.md`.

### 15:38 — discover — slow

- **Category:** discover
- **Tried:** finding how to run the save "in the background" from the Apply button's callback.
- **Expected:** a `spawn` on `EventContext` (the button callback gets `&mut EventContext`, not the `Runtime`).
- **Resolved by:** `Runtime::spawn` exists (found by grepping the Runtime page's 980 lines; the guide's state chapter only mentions `Runtime::mailbox` + a hand-made thread for worker results, never `spawn`). From a button you must go through `EventContext::defer(|ui| ui.spawn(..))`, which neither page says; I composed it myself. `defer` returns a `#[must_use]` `TaskReceipt` and `spawn` a `BackgroundTask` — two handles for one save.
- **Read:** guide/state "Worker results and live data", `struct.Runtime.html#method.spawn`, `struct.BackgroundTask.html`, `struct.EventContext.html#method.defer`.

### 15:38 — understand — slow

- **Category:** understand
- **Tried:** making an out-of-range JPEG quality show a message next to the field with `IntegerInput::range(1, 100)` + `on_rejection`.
- **Expected:** a range violation to be reported like a normalizer refusal, so `Field` can show a message.
- **Resolved by:** `IntegerInput::on_rejection` says "A draft that is invalid for the field's own reasons (unparsable, out of range) is not a refusal", so the field only draws an outline with no text. Workaround: a `normalize` closure that re-checks the range and returns `InputRejection::new("Quality must be between 1 and 100")`, since normalization runs before range validation. Unparsable text ("abc") still only gets an outline: no callback reports it.
- **Read:** `struct.IntegerInput.html` (`normalize`, `on_rejection`), `struct.TextField.html#method.on_rejection`, `struct.Field.html`, `enum.FieldStatus.html`.

### 15:38 — discover — slow

- **Category:** discover
- **Tried:** finding how a headless test exercises the window-close policy installed with `App::on_close`.
- **Expected:** a headless counterpart of `App` (or a `Runtime` method) that runs the close admission handler.
- **Resolved by:** none in the library: `Runtime::request_close` only queues a `PresentationAction::Close` that "Headless hosts consume [through] Self::take_presentation_requests"; the headless host has to run the application's policy itself and `complete` the request with `PresentationStatus::Closed`/`CloseRejected`. I factor the policy into one function used by both `App::on_close` and the test host. The `App` docs do not say whether `NativeCloseAction::KeepOpen` completes the receipt as `CloseRejected`.
- **Read:** `struct.App.html#method.on_close`, `enum.NativeCloseAction.html`, `enum.WindowCloseAction.html`, `struct.Runtime.html` (`request_close`, `take_presentation_requests`, `close_detached_window`), `struct.PresentationRequest.html`, `enum.PresentationAction.html`, `enum.PresentationStatus.html`.

### 15:40 — understand — annoyance

- **Category:** understand
- **Tried:** in a test, reading the refusal message of the prefix field from the `TextField`'s `Semantics::description`, since `Semantics::names_children` says a `Field` gives its description to "every child that reports semantics … unless it has one".
- **Expected:** the control's description to carry the field's error text.
- **Resolved by:** the refused draft gives the `TextField` its own description ("Unsaved changes", from `StringRole::TextModified`), so the status message only appears on the `Field` component; I read it from the field's key (`prefix-field`). Works, but which of the two components carries the message depends on the editor's draft state, which a test author cannot guess from the docs.
- **Read:** `struct.Semantics.html` (`description`, `names_children`), `struct.ComponentInspection.html`, `struct.TextField.html#method.modified_indicator`.

### 15:40 — runtime — annoyance

- **Category:** runtime
- **Tried:** `Runtime::find_descendant(root, "quality")` while a `Group` and an `IntegerInput` were both keyed `quality` at different depths.
- **Expected:** nothing; my own key reuse.
- **Resolved by:** the error was a bare `AmbiguousKey` (no key named in the message); I renamed group keys to `*-group`. `find` with a full path would also work, but the path through `Group`/`Form`/`Field` (which of them add segments) is not listed anywhere I could find, so I used `find_descendant` with unique leaf keys.
- **Read:** `struct.Runtime.html` (`find`, `find_descendant`).

### 15:40 — understand — annoyance

- **Category:** understand
- **Tried:** headless verification with `Runtime` + `key_input`/`text_input`/`text_command(SelectAll)` + `inspect()`, and a CPU capture with `CpuRenderer::serve_captures`.
- **Expected:** —
- **Resolved by:** it worked first time; the diagnostics chapter example for captures was copy-paste ready. Noting only that clearing a field before typing needs `text_command(id, TextCommand::SelectAll)`, which I found in the Runtime page, not the controls chapter.
- **Read:** guide/diagnostics "Screenshots", `struct.Runtime.html#method.text_command`, `enum.TextCommand.html`.

## Closing

- **Result:** all five acceptance items are met and shown by `cargo test` (`tests/headless.rs`, 6 tests, all passing): (1) loaded values, titled groups, fields and buttons checked through `Runtime::inspect` semantics, plus CPU captures in `target/*.png`; (2) invalid prefix and quality 250 refused, message shown on the `Field`, old values kept (dirty marker empty, Apply disabled, quality still 75, Escape shows the applied prefix); (3) Apply/Revert disabled when clean, enabled when dirty, Revert restores the values; (4) a successful save writes the JSON through `Runtime::spawn` and clears the dirty state, and a fresh runtime loads it; a save into a missing directory is reported and leaves the state dirty; (5) closing is refused while dirty. Caveats: the close check plays the native host's part itself (`request_close` → `take_presentation_requests` → the same policy `App::on_close` uses → `complete`), because the library has no headless runner for `App::on_close`. The native `App` binary builds but could not be run without a display. Unparsable numeric text ("abc") gets only the red outline, with no message.
- **Concepts I had to learn:** `Runtime` and the `App` initializer; `Element`/keys (`.key()`); `Signal`/`ReadSignal` and derived bindings (`map`, `combine`); `Permission` from a `ReadSignal<bool>` (`Button::enabled`); `Group::titled`, `Form`, `Field` + `FormStatus`/`FieldHandle`/`FieldStatus`; editor draft mode, `normalize` + `InputRejection` + `on_rejection`; the IntegerInput difference between its own invalidity and a refusal; `EventContext::defer` → `Runtime::spawn` for background work; `NativeCloseAction` and the presentation-request queue; inspection `Semantics`; `CpuRenderer` captures.
- **What worked well:** the API matched my guesses often enough that the first build of the whole app compiled without errors. Reference pages state behavior precisely (when drafts save, what is reported, what stays in effect). `Field` + `normalize` + `on_rejection` gives "refuse, explain, keep the old value" with almost no code. Headless driving through `Runtime` (focus, typed text, keys, `inspect`) is a good first-class test story, and CPU captures made visual checks trivial.
- **The three changes I want most:**
  1. A headless way to exercise `App` policies, at least `on_close` (for example `Runtime::set_close_handler` or a headless `App::run_with(host)`), so tests don't re-implement host behavior. Also document that `KeepOpen` completes the receipt as `CloseRejected`.
  2. Let `IntegerInput`/`DecimalInput` report their own invalidity (out of range, unparsable) through `on_rejection` or a sibling callback with a localized message, so `Field` can explain it without a normalizer that duplicates `range`.
  3. `EventContext::spawn` (or a guide section "background work from a button") instead of composing `defer` + `Runtime::spawn`; and update the getting-started dependency instructions to the registry version.
