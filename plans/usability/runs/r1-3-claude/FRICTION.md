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

### 15:50 — docs-wrong — annoyance

- **Category:** docs-wrong
- **Severity:** annoyance
- **Tried:** following the guide's "Run your first window" to add the dependency.
- **Expected:** a `pert-ui = "0.3.52"` line.
- **Resolved by:** ignoring the guide and using the version from the crate page; `pert-ui = { version = "0.3.52", features = ["cpu"] }` built first time.
- **Read:** `pert_ui/guide/getting_started/index.html` ("Until a registry release exists, use a local checkout ... `path = \"../pert-ui/crates/pert-ui\"`"), README feature table.
- **Diagnostic:** n/a

### 15:50 — understand — annoyance

- **Category:** understand
- **Severity:** annoyance
- **Tried:** finding how a two-dimensional value is exposed to inspection.
- **Expected:** a `SemanticValue` variant for a point / pair of numbers, or a guide note on 2-D controls.
- **Resolved by:** `SemanticValue` only has `Text`, `Bool`, `Number`, `Choice`. The `ColorPicker` page's "Inspection" section shows the convention the library itself uses for its saturation/value area: role `Slider`, value as text, plus numeric `properties`. I copy that (text value + `x`/`y` `InspectValue::Number` properties). Had to read a sibling control's docs to learn the convention.
- **Read:** `enum.SemanticValue.html`, `struct.Semantics.html`, `struct.ComponentDetails.html`, `enum.InspectValue.html`, `struct.ColorPicker.html`.
- **Diagnostic:** n/a

### 15:50 — understand — slow

- **Category:** understand
- **Severity:** slow
- **Tried:** deciding where a custom drag may write its binding and how Escape reaches it.
- **Expected:** a statement of whether `Component::cancel_input` may write signals (it receives an `EventContext`, and the custom-widgets chapter says event phases allow writes), and whether `Runtime::key_input(Escape)` calls `cancel_input` on a component holding pointer capture.
- **Resolved by:** partially. `cancel_input` says "Do not activate application actions from cancellation"; it is also called on removal/shutdown, so I will not write there. `Runtime::key_input` says "Escape's initial press cancels focused edit gestures before delivery" without saying what an "edit gesture" is for a custom component. Plan: write the binding live during the drag from `pointer_input`, and on cancel record the pre-press value and request a `Notifier`, then restore it in `deliver_notifications` (the documented pattern for changes made without a write context). Will verify by test.
- **Read:** `trait.Component.html` (`cancel_input`, `focus_lost`, `pointer_input`, `deliver_notifications`), `struct.Runtime.html#method.key_input`, `struct.Notifier.html`, guide `custom_widgets` "Who may write, and when", "Pointer input that converges".
- **Diagnostic:** n/a

### 15:53 — compile — annoyance

- **Category:** compile
- **Severity:** annoyance
- **Tried:** reading `entry.properties` / `entry.semantics` on an `InspectionSnapshot::components` entry, by analogy with `ComponentDetails`.
- **Expected:** the fields I set in `inspect` at the top level of the entry.
- **Resolved by:** the compiler's suggestion, `entry.details.properties`. My own guess, not the docs' fault; the diagnostic was excellent.
- **Read:** none (compiler help).
- **Diagnostic:** `error[E0609]: no field `properties` on type `&ComponentInspection`` … `help: one of the expressions' fields has a field of the same name: entry.details.properties`.

### 15:53 — runtime — slow

- **Category:** runtime
- **Severity:** slow
- **Tried:** relying on `Component::cancel_input` for Escape during a pointer drag, as the custom-widgets guide says ("Cancel it in Component::cancel_input, which the runtime calls when the widget loses its input context (Escape, hiding, removal, a modal surface)"), and `Runtime::key_input` says "Escape's initial press cancels focused edit gestures before delivery".
- **Expected:** Escape pressed while my focused picker holds pointer capture calls `cancel_input`.
- **Resolved by:** probing with `eprintln!`: with my own Escape handling disabled, neither `Runtime::key_input(Escape)` nor `Runtime::command_key_input` (native path) calls `cancel_input` while the drag is captured; it only runs later (at runtime drop). Escape is delivered to `keyboard_event` as an ordinary key. So I handle Escape in `keyboard_event` myself (cancel + restore, return `Handled`), and keep `cancel_input` for the other losses of context (`Runtime::cancel_input`, focus loss, hiding). Either the guide's "(Escape, …)" is wrong for custom widgets, or "edit gesture" means something a custom component has to opt in to and I found no way to do so.
- **Read:** guide `custom_widgets` "Pointer input that converges"; `trait.Component.html#method.cancel_input`; `struct.Runtime.html#method.key_input`; `struct.Runtime.html#method.cancel_input`; `enum.CommandKey.html`, `struct.CommandKeyInput.html`.
- **Diagnostic:** test assertion: point stayed at the dragged value instead of the pre-press value.

### 15:54 — compile — annoyance

- **Category:** compile
- **Severity:** annoyance
- **Tried:** in the `App::new` initializer, `?` on my own `mount` function, which returned `Box<dyn Error>`; then `AppError::from(io::Error)`.
- **Expected:** some way to turn an arbitrary error into `AppError`.
- **Resolved by:** made `mount` return `RuntimeError` (the `NumberInputError` from `DecimalInput::range` converts with `?`). The `AppError` page documents `ApplicationError::new` for foreign errors, but the diagnostic pointed somewhere unhelpful: it said `the trait From<std::io::Error> is not implemented for DrawError` (the inference went through `AppError: From<DrawError>`), not `AppError`.
- **Read:** `enum.AppError.html`, `struct.ApplicationError.html`.
- **Diagnostic:** `error[E0277]: the trait bound `DrawError: From<std::io::Error>` is not satisfied` at `AppError::from(...)`.

### 15:54 — understand — annoyance

- **Category:** understand
- **Severity:** annoyance
- **Tried:** `check_redescription(&mut ui, before, after, fresh)` in a test with a viewport, then focusing `Redescribed::root` and pressing an arrow key.
- **Expected:** the check to be quiet, and `root` to be my component.
- **Resolved by:** `root` is a wrapping column keyed `redescribe`; the component is `Redescribed::component` (the struct page says so, the function page only says "returns the retained tree under Redescribed::root"). Also, the helper mounts the reference copy as a second root at the same place, so with a viewport set, development validation prints `[pert-ui validation ERROR UnexpectedOverlap] Components "redescribe" (#1) and "fresh" (#3) overlap` — an error the library's own test helper causes. Removing the viewport from that test silences it.
- **Read:** `fn.check_redescription.html`, `struct.Redescribed.html`, `trait.Component.html#method.reconcile`.
- **Diagnostic:** the validation line above (stderr), test assertion `the new handler runs: left 0, right 2`.

## Closing

- **Result:** All six acceptance items are met and checked headlessly. I never read `/kit/vendor`; one compiler note pointed at a vendored path, which I did not open.
  1. `tests/headless.rs::drag_moves_handle_and_clamps_outside`: press at (0.25, 0.75), drag far outside on both sides, values clamp to (1, 0) and (0, 1). Release ends the drag.
  2. `escape_cancels_drag_and_restores` (headless `key_input`), `escape_on_native_key_path_cancels_drag` (`command_key_input`), `runtime_cancel_input_restores`, `focus_loss_cancels_drag_and_restores`: each restores the point from before the press.
  3. `arrow_keys_step_with_and_without_shift`: steps of 0.01 and 0.1 (Shift held via `key_input(ShiftLeft)`), clamped at 1.0. Steps that change nothing are not reported.
  4. `editing_a_field_moves_the_handle_and_reset_restores`: typing 0.8 + Enter in the x field moves the picker (checked through its inspection entry). An arrow key on the picker shows in the y field. Clicking Reset restores (0.5, 0.5).
  5. `cargo run` (headless CPU) writes `capture-dark.png` and `capture-light.png` at scale 2. The picker is focused (theme `FocusRing`) at (0.70, 0.30). `cargo run -- --window` opens the native `App`. It can't run here: `EventLoop("... neither WAYLAND_DISPLAY nor WAYLAND_SOCKET nor DISPLAY is set.")`.
  6. `value_appears_in_inspection_snapshot`: `Runtime::inspect()` → the entry with `inspection_id(picker)` has semantics `Slider`, value `Text("x 0.25, y 0.75")` and properties `x`/`y` as `InspectValue::Number`. The test reads it without using the picker's type.
  - Extra: `redescription_takes_new_bindings_and_handler` (a `reconcile` implementation checked with `check_redescription`), and `no_validation_issues` (the development validation report is empty after a drag and a paint).
- **Concepts I had to learn:** `Runtime` + `flush` as the headless host; `Element`/keys (`IntoElement::key`) and `find_descendant`; `Signal` bindings shared by several controls; the `Component` trait phases (update reads with `read_paint`, measure/paint read retained state, writes only in event phases); `MeasureContext::metric`/`MetricRole`; `Painter` with `ColorRole`s and `focus_within`; pointer capture and converging moves (`is_refresh`); `KeyboardState::control_key`/`chord_modifiers`; `cancel_input` vs `focus_lost`; `Notifier` + `deliver_notifications` for writes needed after a cancellation; `ComponentDetails`/`Semantics`/`InspectValue` and `inspection_id`; `reconcile`/`check_redescription`; `ThemeManager::set_mode`; `set_capture_support` + `request_capture` + `CpuRenderer::serve_captures` + `encode_png`.
- **What worked well:** The guide's custom-widget chapter and the `Component` trait page are thorough, and the inline examples compile almost verbatim. My first build of the control compiled with no errors. The `pointer_input` example (a drag that converges) and the `ColorRole::FocusRing` note ("A custom component reproduces it with Painter::stroke_shape on the same bounds and shape") told me exactly what to do. Headless testing is excellent: real input entry points, deterministic flushes, inspection ids, and a CPU capture in ~10 lines with no window. Theme switching repainted my control with no extra work. Compiler help was good (e.g. `entry.details.properties`).
- **The three changes I want most:**
  1. Make Escape reach custom widgets as documented: either the runtime calls `cancel_input` on the component holding pointer capture when Escape is pressed (as the guide says), or the docs say plainly that a custom widget handles Escape in `keyboard_event`. Also say whether `cancel_input` may write bindings, or recommend the `Notifier` pattern for restoring a value.
  2. A guide section or example for a small custom *value* control: bind and set the value, report changes, live writes vs preview/commit, cancel/restore, focus ring, inspection. It should also cover multi-value semantics, either a 2-D `SemanticValue` or a stated convention like ColorPicker's "text value + numeric properties". Right now these pieces are spread across `Component`, `ColorRole`, `ColorPicker` and `Semantics`.
  3. Keep the guide's install instructions in step with the release (the getting-started page still says to use a path dependency to a checkout). Make `check_redescription` avoid triggering its own `UnexpectedOverlap` validation error when a viewport is set.
