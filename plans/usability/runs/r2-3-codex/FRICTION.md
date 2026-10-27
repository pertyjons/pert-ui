# Friction log

Copy this file to `/work/FRICTION.md`. Add one entry per event, in time
order, as it happens. When something costs you time, run `date +%H:%M` at
that moment and write the entry before you continue; do not estimate a
time. If you do write an entry later, give it the time you write it and add
`(written later)` after the time.

## Entries

### 18:57 — understand — annoyance

- **Tried:** Read the custom widgets guide to implement cancellation using `Component::cancel_input`.
- **Expected:** Escape would use the same cancellation hook as loss of input context.
- **Resolved by:** The guide explicitly says ordinary Escape does not call `cancel_input`; I will restore the gesture in `keyboard_event` as well as context/focus loss.
- **Read:** `/kit/doc/pert_ui/guide/custom_widgets/index.html`, “Pointer input that converges”; `Component::pointer_input` and `Component::keyboard_event`.

### 18:58 — compile — annoyance

- **Tried:** `Container::padding(24)` in the first `cargo check --offline`.
- **Expected:** A numeric literal to convert to padding.
- **Resolved by:** The compiler lists `From<Length>` and `From<MetricRole>`; use `Length::from(24)`. The API page does specify those types; I overlooked that detail.
- **Read:** `struct.Container.html#method.padding`.
- **Diagnostic:** `error[E0277]: the trait bound Spacing: From<{integer}> is not satisfied` at `src/main.rs`, `.padding(24)`.

### 18:58 — compile — annoyance

- **Tried:** Assert a decimal editor's value as `SemanticValue::Number(1.)`.
- **Expected:** The number variant to be a tuple like `InspectValue::Number`.
- **Resolved by:** Read `enum.SemanticValue.html`; the variant also carries optional minimum and maximum. Use a pattern matching `value`.
- **Read:** Testing guide and `enum.SemanticValue.html`.
- **Diagnostic:** `error[E0533]: expected value, found struct variant SemanticValue::Number` at `src/headless.rs:52`.

### 18:58 — compile — annoyance

- **Tried:** Serialize `Runtime::inspect()` to JSON using `serde_json::to_vec_pretty` with only the required `cpu` feature.
- **Expected:** The `Serialize` implementation shown on the rendered `InspectionSnapshot` page to be available.
- **Resolved by:** Diagnostics guide “Serialization” and README feature table explain that `serde` is off by default; enable it.
- **Read:** `struct.InspectionSnapshot.html`, `guide/diagnostics/index.html`, `/kit/readme/pert-ui.md`.
- **Diagnostic:** `error[E0277]: the trait bound InspectionSnapshot: serde::Serialize is not satisfied` at `src/headless.rs:93`.

### 18:58 — discover — annoyance

- **Tried:** Use `InspectValue` and `ThemeMode` with the testing guide's `prelude::*` and `testing::*` imports.
- **Expected:** Those two globs to cover the types needed by an inspecting themed headless check.
- **Resolved by:** Import the two types explicitly from the crate root, as the crate overview explains for other names.
- **Read:** Crate overview; testing guide “Reading what the runtime shows” and “A capture”.
- **Diagnostic:** `error[E0433]: cannot find type InspectValue in this scope`; `error[E0433]: cannot find type ThemeMode in this scope`.


### 19:00 — runtime — annoyance

- **Tried:** Reviewed the CPU captures after the acceptance test, including a focus-loss cancellation.
- **Expected:** The status caption to describe the current point consistently with the fields.
- **Resolved by:** My callback only recorded user writes, and cancellation correctly restored the signals without an application action. Relabeled the caption “Last picker input” to make its meaning explicit. The numeric fields and handle were already correct. Re-ran tests and the headless executable successfully.
- **Read:** `trait.Component.html#method.cancel_input`; custom widgets guide “Who may write, and when”.

## Closing

- **Result:** All six acceptance items met. `cargo test --offline -- --nocapture` passed the application acceptance test, and `cargo run --offline -- --headless` passed the same check. Real runtime pointer events exercise inside motion, outside clamping, release and Escape restoration; Driver drives all four arrow keys, Shift chords, numeric editing and Reset. An additional focus-loss cancellation check passed. CPU captures of the focused application are in `artifacts/dark.png` and `artifacts/light.png`; the typed `x` and `y` inspection properties and semantic value are in `artifacts/inspection.json`. The public reusable control is exported by `src/lib.rs` and implemented in `src/picker.rs`. The native window was not run because there is no display.
- **Concepts I had to learn:** Components and retained updates; signals and tracked reads; event versus evaluation phases; pointer capture, local coordinates and convergent refresh events; explicit Escape handling versus context cancellation; themed paint dependencies and focus tracking; semantic values versus numeric inspection properties; numeric draft commits; Driver settling and queries; CPU capture requests; optional snapshot serialization.
- **What worked well:** The custom widgets guide directly addresses this task, including the subtle Escape behavior and restrictions on cancellation callbacks. Its pointer example explains how to avoid refresh loops. The testing chapter's Driver and CPU capture recipes worked without a display. Inspectable numeric properties let the check read the custom control without downcasting. No library source reads were needed.
- **The three changes I want most:**
  1. Mark feature-gated trait implementations clearly on rendered inspection pages, especially Serialize/Deserialize.
  2. Add a compact complete custom-control example combining binding, paint, focus, capture, Escape, context cancellation and inspection; the individual contracts are good but currently spread across several pages.
  3. Add explicit inspection-property and theme imports to a themed custom-widget headless example, and show the difference between SemanticValue::Number and InspectValue::Number beside one another.
