# Point picker

A reusable `PointPicker` component with live `Signal<f64>` bindings for both
coordinates and an `on_change` callback for user edits. Coordinates run from
0 to 1, increasing rightward/downward. Owner writes must be finite and in range;
invalid readings fail the runtime flush without replacing the retained point.

- Press and drag, including outside the square; release commits.
- Escape or losing input context restores the point before the press.
- Focus and use arrows for 0.01 steps; hold Shift for 0.1 steps.
- The numeric editors commit on Enter or focus loss. Reset restores the center.
- Inspection exposes numeric `x` and `y` properties and a semantic value.
- `on_change` reports pointer/key writes and Escape restoration. Input-context
  cancellation restores bindings without calling application actions, following
  the library's cancellation contract.

Run the desktop application with `cargo run --offline` on a machine with a display.
Run `cargo test --offline` without a display. The integration test mounts the same
application through `App::headless`, drives runtime pointer input and the library's
`Driver`, checks all acceptance behaviors, and regenerates:

- `artifacts/dark.png` and `artifacts/light.png`: CPU captures with picker focused.
- `artifacts/inspection.json`: the full inspection snapshot including coordinates.

`src/picker.rs` contains the control; `src/lib.rs` composes the application.
