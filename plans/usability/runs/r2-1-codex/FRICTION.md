# Friction log

Copy this file to `/work/FRICTION.md`. Add one entry per event, in time
order, as it happens. When something costs you time, run `date +%H:%M` at
that moment and write the entry before you continue; do not estimate a
time. If you do write an entry later, give it the time you write it and add
`(written later)` after the time.

## Entries


### 18:47 — discover — annoyance

- **Tried:** Implementing Revert so it discards retained invalid drafts, using Runtime::text_command after locating editors from the main window.
- **Expected:** A window-to-root accessor; I guessed NativeWindowId::root and guessed App::size(width, height) from the guide's prose.
- **Resolved by:** Reading App::size and Runtime::text_command; retain editor WidgetIds explicitly instead of guessing a window root API.
- **Read:** pert_ui/guide/getting_started/index.html, struct.App.html, struct.Runtime.html#method.text_command.
- **Diagnostic:** E0599: no method named `root` found for struct `NativeWindowId`; E0061: this method takes 1 argument but 2 arguments were supplied; E0599: no method named `on_close` found for enum `Result<T, E>`.

### 18:47 — discover — annoyance

- **Tried:** Looking for a typed FieldInspection page to assert the rendered validation message.
- **Expected:** A dedicated inspection structure; /kit/doc/pert_ui/struct.FieldInspection.html does not exist.
- **Resolved by:** ComponentDetails documents a properties map; query the field's published properties instead.
- **Read:** struct.Field.html and struct.ComponentDetails.html.

### 18:48 — understand — annoyance

- **Tried:** Writing semantic assertions and using a Driver helper inside Driver::wait_until.
- **Expected:** Number to be a tuple variant and wait_until to provide the Driver; both were guesses beyond the guide examples.
- **Resolved by:** Compiler help identifies Number's named fields and wait_until's Runtime argument; query the Runtime inside the predicate.
- **Read:** guide/testing/index.html and enum.SemanticValue.html.
- **Diagnostic:** E0533: expected value, found struct variant `SemanticValue::Number`; E0308: mismatched types, expected reference `&Driver<'_>`, found reference `&Runtime`.

## Closing

- **Result:** All five acceptance items met. `cargo test --offline` passes two display-free tests using `App::headless`, `Driver` text/key/click input, semantic/property inspection, background-task settlement and `HeadlessApp::admit_close`. Tests check loaded fields and group titles; refused prefix and quality with visible field messages and retained valid values; clean/dirty button permissions and marker; Revert including a retained invalid draft; successful JSON output; failed output to a directory with dirty state retained; and close refusal followed by clean close admission. Desktop presentation was not run because there is no display.
- **Concepts I had to learn:** Runtime and keyed Elements; Signal and ReadSignal map/combine; normalizing editor drafts and Enter/focus-loss saves; FieldStatus and rejection callbacks; reactive permissions; EventContext::spawn and deferred task receipts; Driver settling, inspection semantics/properties; App headless close admission; TextCommand::DiscardDraft for Revert.
- **What worked well:** The testing guide gives unusually direct examples for this exact kind of application, including worker completion and close policy. Validation callbacks and FieldStatus compose without custom widgets. Default features work headlessly. No library source reading was needed. Compiler diagnostics resolved the API guesses quickly.
- **The three changes I want most:**
  1. A complete settings-form recipe combining validation, applied/edited values, background persistence, Revert and close policy.
  2. A guide example for discarding all retained form drafts on Revert, including fields whose bound values have not changed.
  3. Typed or documented inspection property names for Field and numeric controls, alongside examples of numeric SemanticValue assertions and Runtime queries inside wait_until.
