# Point picker

A reusable `PointPicker` component in `src/picker.rs`, used beside bounded
numeric editors and Reset. Coordinates start at the upper left. Owners set its
`Signal<PickPoint>`; `on_change` reports user changes, including cancellation
rollback. The fixed 260 × 260 square uses semantic theme colors and the theme's
focus width. Inspection exposes `properties.value` as an object with numeric
`x` and `y` members.

Run the deterministic display-free acceptance session:

```sh
cargo run --offline
cargo test --offline
```

The session sends runtime pointer, native keyboard, and committed text events,
checks inspection values, and writes `artifacts/dark.png`, `artifacts/light.png`,
and `artifacts/inspection.json`. It tests capture outside the square, release,
Escape rollback, fine and Shift arrow steps, edits to both numeric fields, and
Reset. The captures show the picker focused.

On a desktop with a display, `cargo run --offline -- --window` opens the same
application. Native window startup was not exercised in this display-free environment.
