# Friction log

Copy this file to `/work/FRICTION.md`. Add one entry per event, in time
order, as it happens. Take each entry's time from the shell (`date +%H:%M`);
do not estimate it.

## Entries

### 15:50 — understand — annoyance

- **Category:** understand
- **Severity:** annoyance
- **Tried:** Implementing rollback in `Component::cancel_input`, using the same fallible signal writes and owner callback as ordinary input.
- **Expected:** A way to propagate a binding/callback error from cancellation, like `pointer_input` and `focus_lost` do.
- **Resolved by:** Retaining an exceptional cancellation error and returning it on the next update. Ordinary Escape handling can propagate errors directly.
- **Read:** `guide/custom_widgets/index.html` (Pointer input that converges), `trait.Component.html#method.cancel_input`, `struct.EventContext.html#method.defer`.
- **Diagnostic:** None; `cancel_input` returns `()` rather than `Result`.

### 15:51 — compile — annoyance

- **Category:** compile
- **Severity:** annoyance
- **Tried:** `CaptureTarget::window(ui.primary_window())` after reading the capture guide.
- **Expected:** The window identifier could be passed by value (the guide describes the types but its runnable example uses a path capture).
- **Resolved by:** Borrowing it as `CaptureTarget::window(&ui.primary_window())`; the compiler suggested the exact fix.
- **Read:** `guide/diagnostics/index.html` (Screenshots).
- **Diagnostic:** `error[E0308]: mismatched types` at `src/headless.rs:88:82`; `expected &NativeWindowId, found NativeWindowId`.

## Closing

- **Result:** All six acceptance items met. `cargo test --offline` passes the deterministic application acceptance session; its runtime-driven interactions read the custom value through inspection, edit both numeric fields, and reset the point. CPU captures in `artifacts/dark.png` and `artifacts/light.png` show the focused picker; `artifacts/inspection.json` includes its value. No library source was read. Native display startup was not tested.
- **Concepts I had to learn:** Retained `Component` phases; runtime-local signals; input-phase callbacks for synchronization; pointer capture and coordinate conversion; convergent move handling; cancellation; named keyboard routing and modifier state; theme paint dependencies; explicit inspection contributions; CPU capture requests.
- **What worked well:** The custom-widget guide directly covered nearly every requirement, especially outside-bounds capture and cancellation. The diagnostics guide provided runnable headless text-input and CPU-capture examples. The implementation compiled on the first `cargo check`; the headless session passed on its first execution after correcting the capture target argument.
- **The three changes I want most:** (1) Document a recommended pattern for fallible rollback from `cancel_input`, which cannot return an error. (2) Add a complete window-capture example alongside the path-capture example, including the borrowed `NativeWindowId`. (3) Add a reusable two-coordinate control example tying custom inspection properties, numeric-field callbacks, and deterministic gesture tests together; those pieces are currently spread across guides and reference pages.
