# Custom point picker

Built against `pert-ui` 0.3.75 with its default features plus `cpu` and
`serde`. No library source was read.

```sh
cargo test --offline -- --nocapture
cargo run --offline -- --headless
```

Both commands exercise the same application tree without a display and write:

- `artifacts/dark.png` and `artifacts/light.png`: CPU captures with the picker focused.
- `artifacts/inspection.json`: the library's inspection snapshot, including the
  picker's semantic value and numeric `x` and `y` properties.

`cargo run --offline` opens the native application when a display is available.

The reusable `point_picker::PointPicker` takes two `Signal<f64>` values. The owner
can set those signals; user input writes them and calls the optional `on_change`
callback. Bind numeric fields to the same signals for bidirectional updates.
Values supplied by the owner must be finite and within 0..=1. Coordinates increase
rightwards and downwards. The control has a fixed 260-pixel square allocation,
with a small inset to keep the complete handle visible at endpoints.

Dragging captures the primary pointer until its release, and clamps outside
positions. Escape, focus loss, hiding and removal restore the starting point.
Context cancellation restores signals without calling application actions, per
the library's cancellation contract. Arrow keys change coordinates by 0.01;
Shift changes them by 0.1. Numeric drafts save on Enter or focus loss.

The status line records the last user input reported by the picker; the fields
show its current value, including owner writes and context cancellation.

The headless check verifies pointer capture and clamping, Escape restoration,
focus-loss restoration, all four arrows and Shift, field editing in both axes,
Reset, inspection, and both themed captures. See `FRICTION.md` for difficulties.
