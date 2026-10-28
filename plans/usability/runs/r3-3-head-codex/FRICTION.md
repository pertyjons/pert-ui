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

### 17:14 — discover — annoyance

- **Tried:** Guessed `guide/custom_components/index.html` before reading the guide index.
- **Expected:** A chapter for custom components at that path.
- **Resolved by:** The guide index links `guide/custom_widgets/index.html`; its live-edit control example directly answers most of this task.
- **Read:** `/kit/doc/pert_ui/index.html`, `/kit/doc/pert_ui/guide/index.html`, `guide/custom_widgets/index.html`.
- **Diagnostic:** pandoc: /kit/doc/pert_ui/guide/custom_components/index.html: withBinaryFile: does not exist (No such file or directory).

### 17:16 — compile — annoyance

- **Tried:** Used the `component::*` import for a custom control, following the custom-widget example, and passed a `RuntimeError`-returning mount function directly to `App::new`.
- **Expected:** All example names available from the glob; compatible initializer error type.
- **Resolved by:** Importing `pert_ui::ApplicationError` explicitly (the guide's Level example also imports `prelude::*`) and wrapping mount in `|ui| Ok(mount(ui)?)` to convert the error.
- **Read:** `guide/custom_widgets/index.html` (Level example and imports), compiler's `App::new` bound.
- **Diagnostic:** `error[E0433]: cannot find type ApplicationError in this scope` at src/picker.rs:43; `error[E0271]: expected mount to return Result<_, AppError>, but it returns Result<State, RuntimeError>` at src/lib.rs:35.

## Closing

- **Result:** All six acceptance items met. `cargo test --offline` passes the integration check using the application's `App::headless`, library `Driver`, runtime pointer input and inspection. Captures are `artifacts/dark.png` and `artifacts/light.png`; both visibly show the focused picker. `artifacts/inspection.json` includes numeric x/y properties. No library source was read. Native display execution was unavailable, as expected.
- **Concepts I had to learn:** Module-specific preludes; retained components and update-phase binding reads; event-phase writes; pointer capture and repeated refresh events; Escape versus `cancel_input`; owner bindings and user-write callbacks; theme roles and focus painting; inspection properties and semantics; numeric editor draft/commit behavior; headless Driver and CPU capture request/reply; hard size limits versus preferred measurement.
- **What worked well:** The custom-widgets Level example covered almost the entire interaction contract, including cancellation and restore error handling. The testing chapter gave working recipes for typing, inspection and CPU captures. The first full integration run passed without runtime surprises. Numeric controls share signals directly, making synchronization straightforward.
- **The three changes I want most:** A directly linked two-axis custom-control example; an imports summary beside each long custom-widget example (including `ApplicationError`); an application example that factors a reusable mount function and shows the initializer error conversion explicitly.
