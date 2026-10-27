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

### 19:24 — understand — annoyance

- **Category:** understand
- **Severity:** annoyance
- **Tried:** in `src/picker.rs`, `use pert_ui::{component::*, prelude::*};` as the custom widgets chapter recommends ("a widget globs prelude and component ... Both globs can be used together").
- **Expected:** both globs needed, or at least no warning.
- **Resolved by:** removing `prelude::*`; `component::*` alone covered every name the widget uses (rustc: `warning: unused import: prelude::*`). The guide's advice produces a warning in a widget-only module.
- **Read:** guide › custom_widgets › Imports.
- **Diagnostic:** `warning: unused import: prelude::*` (`#[warn(unused_imports)]`).

### 19:24 — understand — annoyance

- **Category:** understand
- **Severity:** annoyance
- **Tried:** building a `DecimalInput` with `.range(0.0, 1.0)?.step(0.01)?.decimal_places(2)?` inside a function that returns `Result<_, RuntimeError>`; these builders return `NumberInputError`, which `?` does not convert to `RuntimeError`.
- **Expected:** a `From<NumberInputError> for RuntimeError`, or a note in `DecimalInput` saying how to propagate it from an initializer.
- **Resolved by:** `.map_err(|e| RuntimeError::from(ApplicationError::new(e)))`. Works, but wraps a library configuration error as an "application error", which `ApplicationError`'s own page says not to do with library meanings. In `App::new`'s initializer (returns `AppError`) it may convert differently; I did not check.
- **Read:** `struct.DecimalInput.html`, `struct.ApplicationError.html`.
- **Diagnostic:** none (avoided by reading signatures first).

### 19:25 — discover — annoyance

- **Category:** discover
- **Severity:** annoyance
- **Tried:** `ThemeMode` in `src/app.rs` with only `use pert_ui::prelude::*;` (the appearance chapter's theme example uses it next to `Runtime`, which the prelude has).
- **Expected:** the theme-mode enum in the prelude, since switching dark/light is a first-program thing.
- **Resolved by:** `use pert_ui::ThemeMode;` from the crate root.
- **Read:** guide › appearance › Themes and color roles.
- **Diagnostic:** `error[E0425]: cannot find type ThemeMode in this scope` (no import suggestion from rustc).

### 19:25 — discover — annoyance

- **Category:** discover
- **Severity:** annoyance
- **Tried:** in the test file, `use pert_ui::prelude::*; use pert_ui::testing::*;` as the testing chapter's "one use block covers it", then asserting on `SemanticRole::Slider` and `InspectValue::Number` from an inspection snapshot.
- **Expected:** the testing glob to hold the inspection names a test compares with (it has `SemanticValue`, but not `SemanticRole` or `InspectValue`).
- **Resolved by:** `use pert_ui::{InspectValue, SemanticRole};`. rustc's suggestion was misleading: `SemanticValue::Slider`.
- **Read:** guide › testing (intro).
- **Diagnostic:** `error[E0433]: cannot find type SemanticRole in this scope ... help: an enum with a similar name exists: SemanticValue::Slider`.

### 19:25 — understand — annoyance

- **Category:** understand
- **Severity:** annoyance
- **Tried:** asserting a `DecimalInput`'s semantics as `SemanticValue::Text("0.21")`.
- **Expected:** I guessed text (it is a text editor); it reports `Number { value, minimum, maximum }`. The `DecimalInput` page does not say what it reports to inspection (the `ColorPicker` page has an "Inspection" section; `DecimalInput`/`IntegerInput` do not, as far as I read). `SemanticValue::Number`'s doc implies it, so this was my guess, not a doc error.
- **Resolved by:** the test failure message showed the value; matched on `Number`.
- **Read:** `struct.DecimalInput.html`, `enum.SemanticValue.html`.
- **Diagnostic:** `left: Some(Number { value: 0.21, minimum: Some(0.0), maximum: Some(1.0) }) right: Some(Text("0.21"))`.

### 19:26 — understand — annoyance (written later, about 19:20)

- **Category:** understand
- **Severity:** annoyance
- **Tried:** deciding when a custom control should write its bindings during a drag, and how to cancel it on Escape.
- **Expected:** one place that says it. It is spread over three pages: guide › custom widgets › "Pointer input that converges" (Escape ends capture, then reaches `keyboard_event`; cancel the gesture there), `Component::cancel_input` (may restore own bindings, must not call application callbacks) and `ColorPicker` (previews and writes only on release). Once read, everything worked first time; this is about finding it, not about it being wrong.
- **Resolved by:** writing the bindings live during the drag (so the numeric fields follow), restoring them on Escape in `keyboard_event` and in `cancel_input`, and calling `on_change` only on the release / per key press.
- **Read:** guide › custom_widgets; `trait.Component.html` (`pointer_input`, `keyboard_event`, `cancel_input`, `focus_lost`); `struct.ColorPicker.html`.
- **Diagnostic:** none.

## Closing

- **Result:** all six acceptance items are met, and `cargo test` checks each of them headlessly through `App::headless`, `testing::Driver`, `Runtime::pointer_input`, `Runtime::inspect` and `CpuRenderer` (`tests/headless.rs`, 7 tests, all passing):
  1. `drag_outside_clamps`: press in the middle, move far up and left (reads 0, 1), then far down and right and release outside (reads 1, 0). It also reads the value back through `Driver::semantics`.
  2. `escape_cancels_drag`: drag from (0.25, 0.75) to x = 1, Escape restores (0.25, 0.75), a later release changes nothing, and `on_change` is not called. `focus_loss_cancels_drag_and_layout_validates` checks the same restore on focus loss (through `cancel_input`) and that the development validation report has no issues.
  3. `arrow_keys_step`: Right/Up give 0.51, then Shift+Left/Shift+Down give 0.41.
  4. `fields_move_handle_and_reset_restores`: typing into the x/y `DecimalInput`s moves the picker; picker keys update the fields; clicking Reset restores (0.5, 0.5) in the picker and both fields.
  5. `cpu_captures_in_both_themes`, and `cargo run -- --capture captures` writes `captures/dark.png` and `captures/light.png` with the picker focused. The focus ring uses `FocusRing`/`FocusWidth`, and every color comes from a `ColorRole`.
  6. `value_in_inspection_snapshot`: the picker's entry in `Runtime::inspect()` is a `Slider` whose value is the text `"0.12, 0.88"`, with the `x` and `y` properties as `InspectValue::Number`.
  I could not open the native window (there is no display), so `cargo run` without arguments is untested.
- **Concepts I had to learn:** `Element` and keys, with key paths for `find`; `Signal`, and reading it with `UpdateContext::read_paint` in `update`; `Component` phases (no writes in update, paint or inspect; writes in input callbacks); measure and paint with `Painter`, `ColorRole` and `MetricRole`; `focusable`, and `Painter::focus_within` for the ring; pointer capture and convergence on refresh moves; how Escape is delivered compared with `cancel_input`; `ComponentDetails`/`Semantics` and `SemanticValue::Text` plus number properties for a multi-number value; `DecimalInput` drafts (saved on Enter); `App::headless`, `Driver` and settling; `ThemeManager::set_mode`; capture support, `request_capture`, `CpuRenderer::serve_captures` and `FrameImage::encode_png`.
- **What worked well:** the guide's custom-widgets and testing chapters were enough to write a working control and tests almost without trial and error: the control compiled on the first try, and the only test failure was an assertion of mine that expected the wrong value type; the picker's behavior was right the first time. The `Component::pointer_input` example drag, the `SemanticValue` note that tells you exactly how to report a point, and the `FocusRing` doc that tells a custom component how to reproduce the built-in ring all answered the question at the place I needed it. `App::headless` and a slot for the handles made the native application itself testable. I never needed to read the library source.
- **The three changes I want most:**
  1. One section, in the custom-widgets chapter, on a value-editing control's gesture contract: when to write (live or on commit), where Escape arrives, `cancel_input` restores, and what `on_change` should report. Today this is spread over `Component`, the guide and `ColorPicker`.
  2. Glob coverage that matches the guide: `ThemeMode` in the prelude, and `SemanticRole`/`InspectValue` in `testing` (or the guide saying which module to add); `component` + `prelude` together should not warn when the guide recommends both.
  3. `From<NumberInputError>` (and the other builder errors) for `RuntimeError`/`AppError`, or a documented way to propagate them from an initializer, plus an "Inspection" section on `DecimalInput`/`IntegerInput` like `ColorPicker`'s.
