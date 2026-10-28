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

### 17:51 — compile — annoyance

- **Category:** compile
- **Severity:** annoyance
- **Tried:** wrote my picker module with only `use pert_ui::component::*;`, following the custom widgets guide's "Imports" section ("A widget module globs `component`"), and copied the validation pattern of its `Level` example (`ApplicationError::from("...").into()`).
- **Expected:** `component` to hold every name a widget implementation writes, as the guide says it does.
- **Resolved by:** adding `use pert_ui::ApplicationError;`. The `Level` example globs both `component` and `prelude`, so the doc example compiles, but a widget that follows the "only component" advice does not.
- **Read:** guide::custom_widgets (Imports; A control that edits a value).
- **Diagnostic:** `error[E0433]: cannot find type `ApplicationError` in this scope` at `src/picker.rs`, plain "use of undeclared type", no import suggestion.

### 17:52 — runtime — annoyance (written later, at 17:54)

- **Category:** understand
- **Severity:** annoyance
- **Tried:** a headless test pressing the picker at 10 % of its width and expecting x = 0.1.
- **Expected:** 0.1.
- **Resolved by:** the value was 0.100000001: pointer positions and `Size` are `f32`, so a widget that stores `f64` values carries f32 noise into a `DecimalInput` (which displays the shortest round-trip text, so the field would show `0.100000001`). I round pointer-derived values to four decimals in the picker. Not a library bug, but no page mentions that geometry is f32 while `DecimalInput` is f64, nor suggests a rounding policy for custom controls.
- **Read:** struct.PointerEvent, struct.DecimalInput.
- **Diagnostic:** `left: (0.100000001, 0.899999976) right: (0.1, 0.9)`.

### 17:53 — compile — annoyance

- **Category:** compile
- **Severity:** annoyance
- **Tried:** in `headless.rs` (globbing only `prelude`) `ui.set_viewport(Viewport::new(..))`; in `app.rs` `ui.find(..)?.ok_or("the picker is mounted")?` in a function returning `RuntimeError`.
- **Expected:** `Viewport`, which every headless runtime needs, to be in the prelude; a `&str` to convert into `RuntimeError` as it does into `ApplicationError`.
- **Resolved by:** importing `pert_ui::Viewport` by name (the testing guide says it is in `testing`, not in the prelude), and `ok_or(ApplicationError::from(".."))`.
- **Read:** guide::testing (first section), struct.ApplicationError.
- **Diagnostic:** `error[E0433]: cannot find type `Viewport` in this scope`; `error[E0277]: `?` couldn't convert the error to `pert_ui::RuntimeError` ... the trait `From<&str>` is not implemented for `pert_ui::RuntimeError``.

### 17:53 — docs-wrong — annoyance

- **Category:** docs-wrong
- **Severity:** annoyance
- **Tried:** a test with `use pert_ui::prelude::*; use pert_ui::testing::*;` that counts focus-ring rectangles in `ui.paint()?.commands()` with `DrawCommand::FillRect`, as the custom widgets guide's `Level` example does.
- **Expected:** the testing guide says "The module is closed: every type those names take, return or hold is in `testing` or the prelude, so a test ... needs no other import". `Runtime::paint` returns a `DisplayList` of `DrawCommand`s, which are in neither.
- **Resolved by:** `use pert_ui::DrawCommand;`.
- **Read:** guide::testing (introduction), guide::custom_widgets (A control that edits a value).
- **Diagnostic:** `error[E0433]: cannot find type `DrawCommand` in this scope`.

## Closing

- **Result:** every acceptance item is met, and `cargo test` (8 tests in `tests/picker.rs`) checks each one headlessly through `Runtime`, `testing::Driver` and inspection queries:
  1. `drag_moves_the_handle_and_clamps_outside_the_area` presses, drags beyond the right/bottom and the left edge, releases outside, and reads the clamped value from the inspection properties `x`/`y`.
  2. `escape_cancels_a_drag_and_restores_the_old_point` (Escape), plus `focus_loss_during_a_drag_restores_the_old_point` (`cancel_input`).
  3. `arrow_keys_move_the_point_with_and_without_shift` uses `Driver::key` and `Driver::chord(KeyChord::named(..).shifted(), ..)`.
  4. `numeric_fields_move_the_handle_and_reset_restores_the_centre` types into the `DecimalInput`s, checks the reverse direction (picker key to field value), and clicks Reset.
  5. `cpu_captures_in_both_themes` (and `cargo run -- --capture <dir>`) writes `captures/dark.png` and `captures/light.png` with the picker focused, through `CpuRenderer::serve_captures`. Development validation reports no issues. `the_focus_ring_follows_the_theme` checks that the ring takes the theme's `FocusRing` color.
  6. `the_value_appears_in_an_inspection_snapshot` finds the picker in `Runtime::inspect()` by key: role Slider, a text value, and numeric `x`/`y` properties.
  The native window could not be opened here (no display: winit reports "neither WAYLAND_DISPLAY nor WAYLAND_SOCKET nor DISPLAY is set"), so the native path was not run.
- **Concepts I had to learn:** `Element` keys and key paths; `Signal`/`ReadSignal`, and reading in `update` (`read_paint` for paint-only data); the phases (no writes in update/paint/inspect, writes in input callbacks); `Component` hooks (`measure`, `paint` with `Painter` and theme `ColorRole`/`MetricRole`, `focusable`, `pointer_input` with pointer capture and convergence, `keyboard_event` with `KeyboardState::modifiers`, `cancel_input` and the Escape route, `deliver_notifications` with `Notifier` for failed restores); `ComponentDetails`/`Semantics`/`InspectValue` for inspection; `Runtime::flush` and `Driver::settle`; inspection queries by `inspection_id`; capture support with `CpuRenderer::serve_captures` and `Pending::try_reply`; `ThemeManager::set_mode`.
- **What worked well:** the custom widgets guide's `Level` example is almost exactly the control the task asks for (live writes, Escape restore, `cancel_input`, focus ring, inspection), with the reasons spelled out, so the picker worked on its first run apart from my own f32 rounding. The testing guide answered every headless question (driver, chords, queries, CPU capture) with runnable examples. The reference pages state their contracts precisely (for example `ColorRole::FocusRing` says how to reproduce the built-in ring, and `ColorPicker` says how a multi-number value should be inspected). I did not need to read the library source.
- **The three changes I want most:**
  1. Make the import story match the guide's claims: put `ApplicationError` in `component` (the guide's widget examples use it), `Viewport` in the prelude, and `DrawCommand`/`DisplayList` in `testing` (or reword "the module is closed").
  2. A short note in the custom widgets chapter on f32 geometry versus f64 values, with a suggested rounding for controls that feed `DecimalInput`.
  3. `From<&str>`/`From<String>` for `RuntimeError` (through `ApplicationError`), so `ok_or("...")?` works in functions that return `RuntimeError`, as it does in the guide's examples that return `Box<dyn Error>`.
