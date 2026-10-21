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

### 15:36 — docs-wrong — annoyance

- **Tried:** Followed the getting-started dependency instructions.
- **Expected:** A crates.io dependency matching the facade version.
- **Resolved by:** Used the tester brief's `pert-ui = "0.3.52"`; the guide instead says to use a local checkout because no registry release exists.
- **Read:** `/kit/doc/pert_ui/index.html`, `guide/getting_started/index.html`.

### 15:36 — understand — slow

- **Tried:** Looked for a numeric range-error callback to show a Field error.
- **Expected:** IntegerInput::on_rejection would report out-of-range input.
- **Resolved by:** Its docs explicitly exclude built-in parse/range failures. Use a domain-checking normalizer (before range validation) to provide an InputRejection message, plus permanent help for non-integer drafts.
- **Read:** `struct.IntegerInput.html#method.normalize`, `#method.on_rejection`, `struct.Field.html`.

### 15:39 — compile — annoyance

- **Tried:** Added painting to a test helper returning RuntimeError.
- **Expected:** Runtime and drawing errors would share a conversion.
- **Resolved by:** Assert painting succeeds separately in the test helper.
- **Read:** `struct.Runtime.html#method.paint`; guide diagnostics.
- **Diagnostic:** `error[E0277]: ? couldn't convert the error to pert_ui::RuntimeError`; `the trait From<DrawError> is not implemented for pert_ui::RuntimeError` at `self.ui.paint()?`.

### 15:39 — understand — annoyance

- **Tried:** Read the dirty-marker Label through Semantics::value, as for TextField.
- **Expected:** Visible label text would be exposed as SemanticValue::Text.
- **Resolved by:** Labels expose caption text as their semantic name; use that for label assertions and values for editors.
- **Read:** `struct.Semantics.html`, `struct.Label.html`, guide diagnostics.
- **Diagnostic:** Headless test panic: `dirty-marker: None`.

### 15:41 — discover — annoyance

- **Timing note:** This minor navigation difficulty was missed in the live log and recorded during closing; the heading is the recording time.
- **Tried:** Guessed reference filenames `struct.InputRejections.html` and `struct.NativeCloseDecision.html` while navigating validation and native close policy.
- **Expected:** A plural rejection collection and a separate deferred-decision type.
- **Resolved by:** The actual signatures use InputRejection and NativeCloseAction::Defer(ReadSignal<Option<bool>>); followed the documented types and enumerated the available pages.
- **Read:** `struct.TextField.html#method.on_rejection`, `struct.App.html#method.on_close`, `enum.NativeCloseAction.html`.
- **Diagnostic:** Documentation extraction raised FileNotFoundError for those two guessed pages.

### 15:41 — discover — slow

- **Timing note:** This test-design difficulty was recorded during closing rather than when first encountered; the heading is the recording time.
- **Tried:** Sought a way to drive App's native close event without a display.
- **Expected:** A headless App close-request driver.
- **Resolved by:** App::run owns a native event loop, so mounted FloatingWindow in WindowLayer and drove its actual close button through Runtime. Both the floating close callback and App::on_close use the same CloseGuard. Native close additionally clears focus to commit valid pending drafts. The test verifies visible refusal and that the editor remains open. Native OS integration itself is not exercised.
- **Read:** `struct.App.html#method.run`, `#method.on_close`, `struct.Runtime.html#method.request_close`, `struct.FloatingWindow.html#method.on_close`, `#method.close_label`, `struct.WindowLayer.html`.

## Closing

- **Result:** All five acceptance items met through two passing headless tests (`cargo test --offline`). The tests use pert-ui Runtime focus, text_command, text_input and key_input, inspect widget semantics/properties, and paint scenes without a display. They verify every loaded field and titled group, prefix and quality refusal with visible explanations and retained values, clean/dirty buttons and marker, Revert for all fields, successful JSON persistence and reload, failed persistence leaving dirty, and the real floating-window close button refusing dirty closure. `cargo check --offline` also passed. The desktop OS close hook is wired to the same guard but cannot be exercised without a display.
- **Concepts I had to learn:** Keyed Elements and retained Runtime; local Signal and derived map/combine; draft editors and normalize/on_save/on_rejection; FieldStatus; typed inspection semantics versus properties; worker UiSender and retained Mailbox; FloatingWindow/WindowLayer and native close admission.
- **What worked well:** The facade map and guide quickly pointed to the right contracts. Draft normalization retained old values without custom rollback. Field made visible error presentation straightforward. Mailbox documentation explained worker ownership and lifetime. Runtime input and inspection made meaningful tests possible without a renderer or display. No library source was read.
- **The three changes I want most:**
  1. Report every numeric validation failure, including parse and range failures, through one callback suitable for FieldStatus.
  2. Add a complete settings-form example covering validation, dirty state, worker save, and native/headless close policy together.
  3. Explain which standard controls expose text in semantic name versus value, and provide a headless native-close admission harness.
