# Acceptance: r3-3-head-claude-retry1 (task 3, custom point picker)

Independent verifier: fresh Claude session (claude-opus-5-5), 2026-10-28.
Procedure: `plans/usability/acceptance.md` (task-specific paragraph "3. Custom
control"), round rules `plans/usability/round-3-plan.md`. Task file
`plans/usability/tasks/3-custom-control.md`.

**Verdict: met.** All six numbered items are met. Each one has passing,
library-driven evidence from both the tester's checks and my own
(`tests/acceptance.rs`).

This is headless evidence from the CPU renderer and the library's test driver
only. It makes no claim about native execution, native windowing or human
usability.

## Protocol and metadata

| Check | Result |
|---|---|
| KIT `kit.env` vs RUN `conditions.env` | match: commit 32dd9c4f667b50cfbbef7e46b18c0007e2b08ee2, dirty=0, version 0.3.88, rustc 1.99.1 (58cf8f9c3 2026-10-13), prepared 2026-10-28T16:41:41+01:00 |
| Session record | outcome=ended, exit_status=0, brief 17:49:10, ended=2026-10-28T17:54:50+01:00 (5.7 min of a 90 min budget) |
| Acceptance dir | initialized by the controller; the copy was made with rsync excluding `target/`, `.git`, `.claude`, `.codex`; no symlinks in the source; no credentials or CLI homes copied |
| Copy vs `original-source-manifest.json` | all 12 files are byte-identical (`evidence/05-integrity.log`) |
| Original RUN_DIR/work after my session | still matches the manifest; nothing in RUN_DIR was written |
| Library resolution | `pert-ui 0.3.88`, registry source replaced by `/kit/vendor`, offline (`home/.cargo/config.toml`). No workspace path and no installed latest version |
| Required features | `Cargo.toml`: `pert-ui = { version = "0.3.88", features = ["cpu"] }`. `cargo tree -e features -i pert-ui` shows `cpu` and `default` (→ desktop → gpu → native) (`evidence/01-toolchain-features.log`) |
| All builds and tests | run through `run.sh exec /home/per/pert-ui-usability/kit-r3-head /home/per/pert-ui-usability/acceptance-r3-3-head-claude-retry1 <cmd>` |

Protocol deviations: **none observed.** The round plan file changed on disk
during the session. A read-only `git diff` showed only additions to the
execution log and the status line; the frozen sections and task 3 rules are
unchanged.

## Build and test commands and results

Full outputs are in `evidence/02-tester-build-test.log` (tester-only, before
I added anything) and `evidence/04-final-suite.log` (final).

| Command (via `run.sh exec KIT ACC`) | Result |
|---|---|
| `cargo build --locked --all-targets` | ok |
| `cargo test --locked` (tester's suite only) | ok, `tests/picker.rs` 8 passed, 0 failed |
| `cargo clippy --locked --all-targets` | ok, no warnings |
| `cargo run --locked -- --capture /work/acceptance-captures/headless-mode` | ok, wrote dark.png and light.png |
| Final: `cargo test --locked --no-fail-fast` | ok: `tests/acceptance.rs` 8 passed and `tests/picker.rs` 8 passed, 0 failed |
| Final: `cargo clippy --locked --all-targets` (includes my tests) | ok, no warnings |

Verifier-test errors I hit and fixed in my own test file only (none of them
are application defects): two compile errors (`CaptureImage` → `FrameImage`;
`Row` needed `.key()` to become an `Element`) and one `TimeReversed` error
from my capture helper flushing at time zero after the Driver had advanced
the clock. All are logged in `evidence/03-verifier-test-iterations.log`.

## Source review

- `src/picker.rs:52-323`: `PointPicker`, a reusable public type that
  implements `pert_ui::component::Component`.
  - It is built from two owner `Signal<f64>`s plus `name` and `on_change`
    builders.
  - `measure` gives a square of `MetricRole::ColorAreaSize`, and `focusable`
    returns true.
  - `pointer_input` (165-198): a press inside the square captures the
    pointer, requests focus and stores the pre-press `origin`. Press and move
    then write the clamped point (`at`, 123-135, clamps 0..=1). Release ends
    the drag.
  - `keyboard_event` (200-235): the arrow keys step by `STEP` 0.01 or
    `SHIFT_STEP` 0.1 depending on `keyboard.modifiers().shift`. Escape during
    a drag restores `origin`.
  - `cancel_input` (237-244): restores on focus loss or cancellation, calls
    `input_reset`, and reports failures via the notifier, following the
    library contract.
  - `paint` (250-298): every color comes from `painter.color(ColorRole::…)`:
    SurfaceContainer, Outline, Muted, Primary, OnPrimary and FocusRing. There
    are no literal colors. The focus ring is drawn only when
    `painter.focus_within()`, using `MetricRole::FocusWidth`.
  - `inspect` (300-322): Slider semantics with name "Point", a text value
    `"x …, y …"` and properties `x`, `y` (Number) and `dragging`.
  - `on_change` reports user writes to the owner. The owner sets the value by
    writing the signals.
- `src/app.rs:20-73`: the picker sits beside a `Column` holding two
  `DecimalInput` fields (range 0..=1) bound to the **same** signals, a Reset
  `Button` that sets both to 0.5, and a status label.
- `src/headless.rs:94-119`: `capture_png(mode)` renders through
  `Runtime::request_capture` + `cpu::CpuRenderer::serve_captures` with the
  picker focused and returns PNG bytes. `src/main.rs` exposes it as
  `--capture <dir>`.

## Per-item results

### 1. Drag including outside the area, clamped value: **met**
- Code: `src/picker.rs:165-198` (pointer capture, clamping in `at` at
  123-135).
- Tester check: `tests/picker.rs:45` `drag_moves_the_handle_and_clamps_outside_the_area`.
  It sends `Runtime::pointer_input` press, moves beyond the bottom-right and
  left edges, then releases outside, and reads the value via
  `InspectionQuery` by id.
- My check: `tests/acceptance.rs:77` `acc1_…`.
  - Moves beyond the top-right corner give (1,1). Moves beyond the
    bottom-left corner, outside the window, give (0,0). A move outside on
    one axis only gives (0.75, 0).
  - A release outside commits (1, 0.5), and a later button-less move does
    nothing.
  - Both numeric fields' semantics follow.
  - The value is read through `Selector::semantic(Slider, "Point")`, without
    application types or ids.
- My check `acc4` also confirms in CPU capture pixels that the handle is
  painted at the point (Primary color).

### 2. Cancelled drag restores the old value: **met**
- Code: `src/picker.rs:216-226` (Escape → `restore`) and `237-244`
  (`cancel_input`).
- Tester checks: `tests/picker.rs:91` (Escape via `Driver::key`, after a drag
  outside) and `tests/picker.rs:128` (focus loss through
  `Driver::clear_focus`, the library's cancellation path).
- My check: `tests/acceptance.rs:122` `acc2_…`.
  - Starts from (0.33, 0.66). The press moves the point and the drag goes
    outside to (0,1).
  - The press focused the picker. Escape restores (0.33, 0.66) in
    inspection, in the signals and in the x field.
  - A later move or release changes nothing.
  - A fresh drag afterwards works, and Escape without a drag leaves the
    committed value alone.

### 3. Arrow keys with and without Shift: **met**
- Code: `src/picker.rs:200-235`.
- Tester check: `tests/picker.rs:144` (all four arrows, `Driver::chord` with
  Shift, clamping at the edge).
- My check: `tests/acceptance.rs:167` `acc3_…`.
  - Focus is reached through the library's Tab traversal.
  - All four arrows are tested at step 0.01 and all four with Shift at
    step 0.1, each verified through inspection. Shift steps clamp at 0 and 1.
  - Arrows do not move the picker when a numeric field is focused.

### 4. Numeric field editing moves the handle; Reset gives (0.5, 0.5): **met**
- Code: `src/app.rs:36-55` (shared signals) and `31-34` (Reset).
- Tester check: `tests/picker.rs:180`. It edits with `Driver::replace_text`
  plus Enter, checks x picker→field via semantics, and clicks Reset with
  `Driver::click`.
- My check: `tests/acceptance.rs:219` `acc4_…`.
  - x is typed and committed with Enter; y is typed and committed with Tab.
    The picker inspection shows (0.12, 0.87).
  - In the CPU capture the pixel at that point is the theme's Primary, so the
    handle moved.
  - Picker → fields works on **both** axes (arrow keys → 0.13 / 0.86 in the
    field semantics).
  - An out-of-range entry "7" leaves the picker in range.
  - A simulated Reset click gives (0.5, 0.5) in the picker inspection, both
    fields and the signals.

### 5. CPU PNG captures, dark and light, focused picker: **met**
- Code: `src/headless.rs:94-119` and `src/main.rs`.
- Tester check: `tests/picker.rs:293` writes `captures/{dark,light}.png`.
  `tests/picker.rs:258` checks that the focus ring uses the theme's
  FocusRing color by overriding it and counting draw commands.
- My checks:
  - `tests/acceptance.rs:306` `acc5_…`:
    - Writes the application's own `capture_png` output and asserts it is
      byte-identical to a raw CPU capture of the same scene.
    - In those pixels, for each mode, the picker's edge has the active
      palette's `FocusRing` color when focused and lacks it when unfocused.
    - The area fill equals the palette's `SurfaceContainer`.
    - The ring, area fill and window background all differ between dark and
      light.
  - `tests/acceptance.rs:347` `acc5b_…`: switching the theme live changes
    the ring to the new palette's color.
- Visual review: both PNGs show the picker with crosshair and handle at
  (0.3, 0.7), a blue focus ring, the x/y fields, Reset and the status label,
  in clearly different dark and light colors. The unfocused comparison
  captures show a gray outline with no ring.

### 6. Value in an inspection snapshot: **met**
- Code: `src/picker.rs:300-322`.
- Tester check: `tests/picker.rs:223` uses `Runtime::inspect()`. It finds key
  "picker" and checks the Slider role, name, text value and `x`/`y`
  properties.
- My check: `tests/acceptance.rs:366` `acc6_…`. After keyboard input it finds
  the component in `Runtime::inspect()` by semantic role and name only, and
  reads value "x 0.49, y 0.6" and the matching Number properties.

### Task-specific paragraph checks (procedure, task 3)
- **Reusable type implementing the component contract:** met.
  - `impl Component for PointPicker`, checked at compile time by
    `is_component` (`tests/acceptance.rs:397`).
  - `tests/acceptance.rs:402` `acc_reusable_…` mounts two independent
    pickers outside the app. Keys move only the focused one.
  - `on_change` reports user changes to the owner. Values the owner sets
    show up without a report.
- **A drag beyond the area clamps:** met (item 1).
- **Cancellation restores the pre-press value:** met (item 2), through both
  Escape and focus-loss cancellation.
- **Shift changes the arrow step:** met (item 3).
- **Focus indication and colors come from the theme, and the captures
  differ in colors:** met (source review plus `acc5`/`acc5b` pixel checks).
- **Value in an inspection snapshot:** met (item 6).
- **Numeric fields stay in sync both ways:** met (`acc1`, `acc2`, `acc4`, on
  both axes).

## Missing checks or limitations
- No native window run. `point_picker` without arguments (`App::run`) was not
  executed; the task requires only headless verification.
- There is no pointer-cancel event kind in this library version
  (`PointerKind` has Move/Leave/Press/Release/Scroll). Cancellation was
  exercised through Escape and focus loss, which is what the task names.
- Observation, not a defect: `y` grows upward, so (0,0) is bottom-left. This
  is documented in `src/picker.rs`, and the task does not specify an
  orientation.
- Observation: the picker lists Increment/Decrement semantic actions.
  Inspection actions are descriptive only, and the arrow keys provide them.

## Library or application defects
None found for this task. No application failures occurred. The only
failures were in my own test file, and I fixed them there (listed above).

## Added files and evidence (all inside ACCEPTANCE_DIR)
- Added test source: `work/tests/acceptance.rs` (435 lines). It is the only
  file added to the source tree. No production code or tester test was
  changed, and no test hooks or entry points were added.
- Capture paths:
  - `work/acceptance-captures/dark.png` and `light.png` (the application's
    `capture_png` output from `acc5`).
  - `work/acceptance-captures/dark-unfocused.png` and
    `light-unfocused.png` (unfocused comparison).
  - `work/acceptance-captures/headless-mode/{dark,light}.png` (from
    `cargo run -- --capture`).
  - `work/captures/{dark,light}.png` (regenerated by the tester's test).
  - `evidence/submitted-captures/{dark,light}.png` (the submitted
    originals, saved before any run).
  - All four dark copies are byte-identical, and so are all four light
    copies (`evidence/05-integrity.log`).
- Command-output evidence log:
  - `evidence/01-toolchain-features.log`
  - `evidence/02-tester-build-test.log`
  - `evidence/03-verifier-test-iterations.log`
  - `evidence/04-final-suite.log`
  - `evidence/05-integrity.log`

## Aggregate
All six items are met → `acceptance=met`. The session end time is recorded
in conditions.env, so `accepted=2026-10-28T17:54:50+01:00`.
