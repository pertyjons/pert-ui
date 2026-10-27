# Acceptance: r2-3-claude (task 3, custom point picker)

Author check, 2026-10-27, kit `kit-aca8df3` (pert-ui 0.3.75). The submitted
`work/` (without `target/`, `.claude/` and `.git/`) was copied to
`~/pert-ui-usability/acceptance-r2-3-claude`, built with
`cargo build --offline --all-targets` and tested with `cargo test --offline`
in the kit sandbox: builds without warnings (also after touching every source
file), 7 of 7 headless tests in `tests/headless.rs` pass. The headless mode
(`cargo run -- --capture <dir>`) was rerun in the sandbox; it wrote
`dark.png` and `light.png` byte-identical to the submitted
`captures/dark.png` and `captures/light.png` and printed the focused picker's
inspection entry (`focused=true`, `Slider`, value `"0.30, 0.70"`, `x`/`y`
numbers). No author test was needed: the tester's checks drive every item
through `App::headless` with simulated pointer input, `Driver::key`,
`Driver::chord` (Shift), `replace_text` + Enter and `click`, and read results
from inspection (picker semantics and properties, the fields' semantic values)
and from the application's own `Signal`s.

| Item | Result | Code | Check |
|---|---|---|---|
| 1. Drag, including outside the area, reads the clamped value | met | `src/picker.rs:152-190` (press inside captures the pointer and requests focus, moves write live, release commits), `point_at` `:84-89` (clamp) | `tests/headless.rs:32-56` `drag_outside_clamps` (moves far up-left: (0, 1); far down-right and release: (1, 0), read from the signals and the picker's semantic value `"1.00, 0.00"`; one committed edit) |
| 2. Cancel a drag, old value restored | met | Escape in `keyboard_event` `src/picker.rs:201-204`, `cancel_drag` `:111-117`; `cancel_input` `:229-234` | `tests/headless.rs:58-80` `escape_cancels_drag` (from (0.25, 0.75), drag outside, `Driver::key(Escape)` restores; later release changes nothing; not reported); `:163-181` (focus loss restores) |
| 3. Arrow keys with and without Shift | met | `src/picker.rs:205-226` (`STEP` 0.01, `COARSE_STEP` 0.1 with `keyboard.modifiers().shift`) | `tests/headless.rs:82-95` `arrow_keys_step` (Right/Up to (0.51, 0.51); Shift+Left/Down via `Driver::chord` to (0.41, 0.41); four reports) |
| 4. Field edit moves the handle; Reset restores (0.5, 0.5) | met | `src/app.rs:27-66` (`DecimalInput`s and the picker share the `x`/`y` signals; Reset `:46-49`) | `tests/headless.rs:104-128` (types 0.2 / 0.9 + Enter, picker semantics `"0.20, 0.90"`; picker ArrowRight shows 0.21 in the x field's semantics; clicks Reset, picker `"0.50, 0.50"` and both fields 0.5) |
| 5. CPU PNG captures, dark and light, focused picker | met | `src/app.rs:82-101` (focus, `set_mode`, `request_capture`, `CpuRenderer::serve_captures`, `encode_png`), `src/main.rs:17-36` | headless mode `cargo run -- --capture`; `work/captures/dark.png`, `work/captures/light.png` (reproduced byte-identically by the author); `tests/headless.rs:150-161` |
| 6. Value in an inspection snapshot | met | `src/picker.rs:236-251` (`Slider`, text value `"x, y"`, `x`/`y` `InspectValue::Number`, `dragging`) | `tests/headless.rs:130-148` (`Runtime::inspect`, entry by key `picker`, role, value and properties; no picker type used) |

## Task-specific checks

- **Reusable type implementing the component contract:** met. `PointPicker`
  (`src/picker.rs:31-296`) is a public type in its own module implementing
  `Component` (`update`, `measure`, `focusable`, `pointer_input`,
  `keyboard_event`, `cancel_input` calling `input_reset`, `inspect`,
  `paint`), constructed from two `Signal<f64>` with `name` and `on_change`
  builders; the owner sets the value through the signals. Its size follows
  the available space up to `MetricRole::ColorAreaSize`.
- **Drag beyond the area clamps:** met (item 1).
- **Cancellation restores the pre-press value:** met (item 2), for Escape
  and focus loss; the restore is not reported as an edit.
- **Shift changes the arrow step:** met (item 3).
- **Focus indication and colors from the theme:** met. All colors are
  `ColorRole`s (`SurfaceContainer`, `Outline`, `Divider`, `OnSurface`,
  `Primary`, `OnPrimary`); the focus ring is `ColorRole::FocusRing` with
  `MetricRole::FocusWidth` when `painter.focus_within()`
  (`src/picker.rs:253-295`). The two captures differ in colors: background
  (400,250) rgb 36,41,50 dark / 246,248,252 light; area (40,150) 47,52,61 /
  229,234,241; crosshair 239,242,248 / 33,39,48; handle and focus ring
  80,153,237 / 44,111,196. Both show the focus ring, the crosshair and handle
  at (0.30, 0.70) and the fields reading 0.30 and 0.70.
- **Value in an inspection snapshot:** met (item 6).
- **Numeric fields in sync both ways:** met (item 4): field to picker read
  from the picker's semantics, picker to field from the x field's semantics.

## Triage notes

Not acceptance failures. From the source only; the transcript and
FRICTION.md were not read for this acceptance.

- `cancel_input` discards a failed restore (`let _ = self.cancel_drag(cx)`,
  `src/picker.rs:232`) with a comment; the restore writes the signals
  directly from `cancel_input`.
- Inspection follows the `ColorPicker` convention (role `Slider`, text value,
  numeric `x`/`y` properties) since `SemanticValue` has no two-value variant.
- `app::application` uses `expect` for the constant window size
  (`src/app.rs:77`).
