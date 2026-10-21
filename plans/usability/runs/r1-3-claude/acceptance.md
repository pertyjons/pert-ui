# Acceptance: r1-3-claude (task 3, custom point picker)

Author check, 2026-10-21. The submitted `work/` (without `target/`, `.claude/`
and `.git/`) was copied to `~/pert-ui-usability/acceptance-r1-3-claude`,
built with `cargo build --all-targets` and tested with `cargo test` in the
kit sandbox (offline): builds without warnings (also after a forced rebuild of
the crate), 10 of 10 headless tests in `tests/headless.rs` pass. The headless
mode (`cargo run -- <dir>`) was rerun in the sandbox; it wrote
`capture-dark.png` and `capture-light.png` byte-identical to the submitted
ones and printed the focused picker's inspection entry. No author test was
needed: the tester's checks drive every item through simulated pointer, key
(`key_input`, `command_key_input`, held Shift) and text input, and read
results from inspection (picker `x`/`y` properties and semantic value, the
y field's semantic value) and from the application's own `Signal`s.

| Item | Result | Code | Check |
|---|---|---|---|
| 1. Drag, including outside the area, reads the clamped value | met | `src/picker.rs:185-201` (press captures the pointer, moves commit), `point_at` `:102-109` (clamp) | `tests/headless.rs:57-81` `drag_moves_handle_and_clamps_outside` (moves far right/below and to the window origin: (1, 0) and (0, 1); moves after release ignored) |
| 2. Cancel a drag, old value restored | met | Escape in `keyboard_event` `src/picker.rs:225-230`, `cancel_drag` `:111-124`, `cancel_input` `:252-256`, restore written in `deliver_notifications` `:258-263` via a `Notifier` | `tests/headless.rs:83-100` (Escape via `key_input`, then move/release ignored), `:214-231` (Escape via `command_key_input`), `:102-111` (`Runtime::cancel_input`), `:283-295` (focus loss) |
| 3. Arrow keys with and without Shift | met | `src/picker.rs:231-249` (`chord_modifiers`, `STEP` 0.01 / `STEP_LARGE` 0.1, clamped) | `tests/headless.rs:113-139` (0.01 steps, Shift held via `key_input(ShiftLeft)` gives 0.1 steps, clamp at 1.0, change reports counted) |
| 4. Field edit moves the handle; Reset restores (0.5, 0.5) | met | `src/app.rs:46-58` (`DecimalInput`s and picker share the `x`/`y` signals), Reset `:52-53` | `tests/headless.rs:168-212` (types 0.8 + Enter in the x field, picker inspection shows (0.8, 0.5); picker arrow key shows 0.51 in the y field's inspection; clicks Reset, picker inspection (0.5, 0.5)) |
| 5. CPU PNG captures, dark and light, focused picker | met | `src/main.rs:31-69` (focus, keys, `set_mode`, `request_capture`, `CpuRenderer::serve_captures`, `encode_png`) | headless mode `cargo run`; `work/capture-dark.png`, `work/capture-light.png` (reproduced byte-identically by the author) |
| 6. Value in an inspection snapshot | met | `src/picker.rs:265-287` (`Slider`, text value `x 0.25, y 0.75`, `x`/`y` `InspectValue::Number`, `dragging`) | `tests/headless.rs:141-166` (`Runtime::inspect`, entry by `inspection_id`, no picker type used) |

## Task-specific checks

- **Reusable type implementing the component contract:** met. `PointPicker`
  (`src/picker.rs:37-336`) is a public type in its own module implementing
  `Component` (`reconcile`, `update`, `measure`, `focusable`, `pointer_input`,
  `keyboard_event`, `cancel_input` calling `input_reset` as documented,
  `deliver_notifications`, `inspect`, `paint`), constructed from two
  `Signal<f64>` with `name` and `on_change` builders. Redescription is checked
  with the library's `check_redescription` (`tests/headless.rs:233-267`) and
  development validation is empty after a drag (`:269-281`).
- **Drag beyond the area clamps:** met (item 1).
- **Cancellation restores the pre-press value:** met (item 2), for Escape on
  both key paths, `Runtime::cancel_input` and focus loss.
- **Shift changes the arrow step:** met (item 3).
- **Focus indication and colors from the theme:** met. All colors are
  `ColorRole`s (`SurfaceContainer`, `Outline`, `Muted`, `Primary`/`OnPrimary`,
  `ButtonPressed`/`OnButtonPressed`); the focus ring is `ColorRole::FocusRing`
  with `MetricRole::FocusWidth` when `painter.focus_within()`
  (`src/picker.rs:289-335`). The two captures differ in colors: dark has a
  dark background (e.g. pixel (5,10) = rgb 36,41,50), a dark grey area, light
  crosshair and a light-blue handle; light has a near-white background
  (246,248,252), a light grey area, dark crosshair and a darker blue handle.
  Both show the blue focus ring around the picker, the crosshair and handle at
  (0.70, 0.30), and the fields reading 0.70 and 0.30.
- **Value in an inspection snapshot:** met (item 6).
- **Numeric fields in sync both ways:** met (item 4): field to picker read from
  the picker's inspection, picker to field read from the y field's inspection.

## Triage notes

Not acceptance failures.

- Escape during a captured drag is handled in the picker's own
  `keyboard_event`. The tester found by probing (temporarily disabling its own
  Escape branch, `&& false`, and adding `eprintln!` to its own code) that
  neither `Runtime::key_input(Escape)` nor `command_key_input` called
  `Component::cancel_input` on the capturing component, although the
  custom-widgets guide lists Escape among the causes. Logged as *runtime*.
- The restore after cancellation is written in `deliver_notifications`
  through a `Notifier` obtained in `update`, because the tester was unsure
  whether `cancel_input` may write signals. Live writes during the drag go
  straight to the bindings from `pointer_input`.
- Inspection follows the `ColorPicker` convention (role `Slider`, text value,
  numeric `x`/`y` properties) because `SemanticValue` has no two-value
  variant. The picker borrows `MetricRole::ColorAreaSize` for its default
  side and `ColorRole::ButtonPressed` for the handle while dragging.
- The `check_redescription` test runs without a viewport because with one the
  helper's two roots trigger an `UnexpectedOverlap` validation error. Logged.
- `mount` returns `RuntimeError` after an unhelpful `AppError::from`
  diagnostic (pointing at `DrawError`). Logged as *compile*.
- The tester wrote a pandoc helper (`work/.tools/doc.sh`) to read the HTML
  docs as text. No warnings, no discarded `#[must_use]` values, no private
  paths; the native `--window` path compiles but could not run in the sandbox
  (no display), as stated in FRICTION.md.
- Source reading: the transcript mentions `/kit/vendor` three times: an
  `ls /kit/vendor | head -50` directory listing at the start, a compiler note
  pointing at `/kit/vendor/pert-ui-runtime-0.3.52/src/capture.rs:90`, and a
  winit error message from the `--window` attempt. No tool call opened, read
  or searched a file under `/kit/vendor`; every documentation lookup went
  through `/kit/doc` HTML pages. No *source* entry was required.
- FRICTION.md times: all entries (15:50 to 15:54) lie between
  `brief_delivered` 15:48:02 and `ended` 15:56:04 and follow the events they
  describe by at most about two minutes. The first three entries were first
  written with guessed times (15:52, 15:56, 15:58) in the same command that
  ran `date` (15:50) and were corrected to 15:50 four seconds later; later
  entries take `$(date +%H:%M)` directly. Plausible.
