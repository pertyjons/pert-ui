# Acceptance: r3-3-head-codex (task 3, custom point picker)

Verifier: an independent Claude acceptance context (claude-opus-5-5) working
from the frozen procedure (`plans/usability/acceptance.md`, task 3 paragraph)
and the round 3 rules. All builds and tests ran in the kit sandbox through
`run.sh exec KIT ACCEPTANCE_DIR …`. This report claims no native/display
execution and nothing about human usability.

**Result: met.** All six numbered items are met, each backed by passing
library-driven headless checks.

## Protocol and metadata

- Kit `/home/per/pert-ui-usability/kit-r3-head/kit.env` and
  `RUN_DIR/conditions.env` agree: commit
  `32dd9c4f667b50cfbbef7e46b18c0007e2b08ee2`, dirty=0, version 0.3.88,
  toolchain `rustc 1.99.1 (58cf8f9c3 2026-10-13)`, prepared
  2026-10-28T16:41:41+01:00. Run: tester codex (codex-cli 0.165.0, model
  gpt-6.1-sol, effort medium), task `3-custom-control.md`, budget 90 min,
  brief 17:14:31, **ended=2026-10-28T17:17:09+01:00**, outcome ended, exit 0.
- Copy: `RUN_DIR/work` → `ACCEPTANCE_DIR/work` with `rsync -a`, excluding
  `/target`, `.git`, `.claude`, `.codex`. The submission had no symlinks and
  none of the excluded metadata directories outside `target`. Credentials
  and CLI home directories were not copied. Every copied file was hash-checked
  against the original (`evidence/copy-verification.sha256`) before and after
  all test runs: no differences (`evidence/logs/06-copy-hash-recheck.log`).
  The tester's test regenerates `artifacts/*`; the regenerated files are
  byte-identical to the submitted ones. The originals are also kept in
  `evidence/original-artifacts/`.
- The original source still matches the controller's
  `original-source-manifest.json`: 31 files, 0 mismatches
  (`evidence/logs/07-original-manifest-check.log`). I wrote nothing to
  RUN_DIR.
- Dependency features: `cargo tree -e features -i pert-ui` in the sandbox
  shows pert-ui v0.3.88 from `/kit/vendor` with `default` (→ desktop, gpu,
  native), **`cpu`**, and `serde`. The cargo config replaces crates.io with
  `/kit/vendor` and runs offline. No workspace path or installed latest
  library was used (`evidence/logs/01-features.log`).
- Protocol deviations: none.

## Commands and results

All commands were run as
`/home/per/github/pert-ui/plans/usability/run.sh exec /home/per/pert-ui-usability/kit-r3-head /home/per/pert-ui-usability/acceptance-r3-3-head-codex sh -c '<cmd>'`.

| # | Command (in `/work`) | Result | Log |
|---|---|---|---|
| 1 | `cargo tree -e features -i pert-ui --offline`; cargo/rustc versions | features default+cpu+serde, 0.3.88, rustc 1.99.1 | `evidence/logs/01-features.log` |
| 2 | `cargo build --offline --all-targets && cargo test --offline` (tester's suite, unchanged) | builds, no warnings; `application_headless_acceptance` **ok** (1 passed) | `evidence/logs/02-tester-build-test.log` |
| 3 | `cargo test --offline --test acceptance_verifier` (first attempt) | **verifier-test compile error**: I named the type `CaptureImage`, but the library's type is `FrameImage`. This was my error, not an application defect. | `evidence/logs/03-verifier-attempt1.log` |
| 4 | same, after `sed s/CaptureImage/FrameImage/` in the verifier test | 7 passed, 0 failed | `evidence/logs/04-verifier-attempt2.log` |
| 5 | `cargo test --offline --all-targets && cargo test --offline --doc` (final suite) | tester 1/1 ok, verifier 7/7 ok, doc 0; exit 0 | `evidence/logs/05-final-full-suite.log` |

The tester's own build history (`RUN_DIR/log/builds.jsonl`) has one failed
`cargo check` (E0433 missing `ApplicationError` import, E0271 initializer
error type). After that, `cargo test --offline` passed twice.

## Added test source and captures (acceptance copy only)

- `work/tests/acceptance_verifier.rs`: an integration test file. No
  production code, Cargo manifest or tester test was changed. No test access
  hook or entry point was added. The tests mount the application with the
  public `point_picker::app().headless()`. `v7` also mounts the public
  `point_picker::picker::PointPicker` type through the library's
  `App::new`/`mount_tree`. The tests drive the app only through:
  - `Runtime::pointer_input`
  - the `Driver` `key`/`chord`/`replace_text`/`click`/`clear_focus` methods
  - `Runtime::focus`
  - theme overrides through `Runtime::theme().set_color`
  - `Runtime::inspect()`, serialized to JSON and read by key with
    `serde_json::Value`, so the reader needs no application types
  - `Driver::semantics`
  - CPU captures (`request_capture` + `CpuRenderer::serve_captures`)

  For owner-side checks, the app's public `State` signals are read and
  written to exercise the "owner sets the value / is told of changes"
  contract.
- Captures and outputs (written by the tests): `work/acceptance-captures/`
  - `v5-dark-focused.png`, `v5-light-focused.png`: default palettes, picker
    focused
  - `v5-{dark,light}-focused-overridden.png` and
    `v5-{dark,light}-unfocused-overridden.png`: FocusRing overridden to red
    and Surface to blue
  - `v4-after-field-edit.png`, `v4-after-reset.png`: Primary overridden to
    green, to locate the handle
  - `v6-inspection.json`

  Hashes are in `evidence/added-files.sha256`.
- Tester artifacts (regenerated, identical): `work/artifacts/dark.png`,
  `light.png`, `inspection.json`.

## Code under review

- Control: `work/src/picker.rs`
  - `pub struct PointPicker` (`:7`) with `new(x, y)` (`:18`) and
    `on_change` (`:28`)
  - `impl Component for PointPicker` (`:55`)
  - `update` reads the bound signals (`:56-65`)
  - `pointer_input` (`:75-114`): press captures the pointer and focuses
    (`:81-85`); press, move and release write clamped local coordinates
    (`:89-100`, `:104-111`)
  - `keyboard_event` (`:115-143`): step 0.1 with Shift, otherwise 0.01
    (`:125`); Escape restores the pre-press point (`:127-133`); arrows
    `:135-138`; the result is clamped (`:141`)
  - `cancel_input` restores (`:144-149`)
  - `paint` uses `ColorRole::Surface` and `ColorRole::Primary`. The focus
    ring is drawn only when `focus_within()`, with `ColorRole::FocusRing`
    and `MetricRole::FocusWidth` (`:153-183`).
  - `inspect` exposes `x`/`y` `InspectValue::Number` properties and a
    `Slider` semantic with value text (`:184-201`).
- App: `work/src/lib.rs`
  - the picker shares the `x`/`y` signals with two `DecimalInput`s (range
    0..1, step 0.01) (`:22-37`)
  - Reset sets both signals to 0.5 (`:38-42`)
  - `App::new` (`:61`)
- Tester check: `work/tests/headless.rs::application_headless_acceptance`
  (`:31-159`).

## Per-item results

### Item 1: a headless check drags the handle, including outside the area, and reads the clamped value. **Met**

- Tester: `headless.rs:48-64`. The test presses at the center, moves to
  (0.8, 0.2), moves outside to the right and top (→ (1.0, 0.0)) and releases
  outside. It reads the value through inspection properties and checks that
  the fields follow (`:67-74`).
- Verifier: `v1_v2_drag_clamp_and_cancel`:
  - A press inside moves the handle to the press point immediately
    ((0.3, 0.7)) and focuses the picker.
  - Dragging beyond the top-left gives (0, 0), and beyond the bottom-right
    gives (1, 1). Moving back inside gives (0.6, 0.4).
  - Releasing outside keeps the clamped (1.0, 0.25). The fields show it too.
  - Moves after the release do not drag.
  - A press outside the area does not move the point.
  - Passed.

### Item 2: a headless check cancels a drag and sees the old value restored. **Met**

- Tester: `headless.rs:76-88`. The test presses, drags outside, presses
  Escape through `Driver::key`, sees (1.0, 0.0) restored and checks that a
  later move no longer drags.
- Verifier `v1`: the test presses at (0.2, 0.2) and drags outside to
  (0, 1). After Escape, inspection shows the pre-press (1.0, 0.25) and both
  numeric fields show it too. A later move and release do not change it.
  Passed.
- Informational (not required): `v8_focus_loss_cancels_drag`. Moving focus
  during a drag goes through the library's input-context loss →
  `cancel_input` and restores the pre-press point. Later moves and the
  release do not change it. Passed.

### Item 3: a headless check moves the point with the arrow keys, with and without Shift. **Met**

- Tester: `headless.rs:95-109` covers ←/↓ plain, ←/↓ with Shift (via
  `Driver::chord(KeyChord::named(..).shifted(), ..)`) and →/↑ plain.
- Verifier: `v3_arrow_keys_with_and_without_shift`:
  - All four arrows were tested both plain (±0.01) and with Shift (±0.1),
    each with an exact expected value.
  - Values clamp at the 0 and 1 edges.
  - The numeric fields follow.
  - With focus cleared, arrows (plain and Shift) do not move the picker.
  - Passed.

### Item 4: editing a numeric field moves the handle, and Reset restores (0.5, 0.5). **Met**

- Tester: `headless.rs:111-117`. `replace_text` + Enter on both fields
  gives (0.25, 0.75) in inspection. A `Driver::click` on Reset gives
  (0.5, 0.5).
- Verifier: `v4_fields_sync_and_reset`. Theme Primary was overridden to
  green so the painted handle could be located in a capture.
  - x→0.25 and y→0.8 by field editing (Enter) move the inspected value. The
    CPU capture shows the handle painted at (0.25, 0.8) and not at the
    center (`v4-after-field-edit.png`).
  - Committing a field on focus loss (Tab) also moves the picker.
  - Picker keyboard input updates the field.
  - Reset through a simulated click restores (0.5, 0.5) in inspection, in
    both fields and in the painted handle position (`v4-after-reset.png`).
  - Owner writes to the signals move the picker and the fields without a
    change report. A user arrow press is reported to the owner through
    `on_change`.
  - Passed. This also establishes the task paragraph's two-way sync check.

### Item 5: CPU capture PNGs of the application in the dark and light themes, showing the focused picker. **Met**

- Tester: `headless.rs:123-142` writes `artifacts/dark.png` and
  `artifacts/light.png` through `CpuRenderer::serve_captures`, with the
  picker focused (asserted). I viewed both: the dark and light palettes are
  shown, and the picker has its focus border. The files differ.
- Verifier: `v5_captures_theme_and_focus_ring`:
  - The default dark and light focused captures differ in pixels, and the
    picker surface pixel differs between themes.
  - With theme FocusRing overridden to red and Surface to blue in both
    modes, each focused capture has at least 2·(w+h) red pixels around the
    picker bounds, and its surface pixel is exactly the theme's Surface
    color.
  - With focus cleared, each unfocused capture has zero FocusRing pixels.
  - This shows that the focus indication and colors come from the active
    theme. Passed. I viewed the captures.

### Item 6: the picker's value appears in an inspection snapshot. **Met**

- Tester: `headless.rs:143-151` writes `artifacts/inspection.json` and
  asserts the `x` property 0.35 by component key. `read_point`
  (`:4-15`) reads properties from `Runtime::inspect()`.
- Verifier: `v6_value_in_inspection_snapshot` and the `json_point` helper
  used by every verifier test. They read the serialized snapshot as generic
  JSON by component key `"picker"`:
  `{"x": {"Number": 0.6}, "y": {"Number": 0.51}}`. The semantic value is
  `Text("0.6, 0.51")` with role `Slider` (`v6-inspection.json`). Passed.

### Task paragraph checks (task 3)

| Check | Evidence | Status |
|---|---|---|
| Reusable type implementing the component contract | `PointPicker` `impl Component` (`picker.rs:55`). `v7_reusable_control_two_instances` mounts two independent instances in a different library-mounted tree. Each is driven by pointer or keyboard input and keeps its own value. | established |
| Drag beyond the area clamps | Item 1 (tester + `v1`, all four sides) | established |
| Cancellation restores the pre-press value | Item 2 (tester + `v1`; `v8` informational) | established |
| Shift changes the arrow step | Item 3 (`v3`, all directions) | established |
| Focus indication and colors come from the theme; captures differ in colors | Item 5 (`v5` role overrides, focused/unfocused) | established |
| Value in inspection snapshot | Item 6 | established |
| Numeric fields stay in sync both ways | Item 4 (`v4`), item 1 (fields after drag), item 3 (fields after keys) | established |

## Missing evidence

None for the numbered items. Not covered, because the task did not require
it:

- native/display execution (no display, by design)
- behavior with an invalid owner value (the README says the runtime flush
  fails)
- whether pointer capture is released explicitly after an Escape
  cancellation. Only the observable behavior was tested: no further drag
  and a correct value.

## Observations (not defects)

- In the default palettes, FocusRing and the Primary crosshair look alike.
  The ring is still a distinct border, present only while focused.
- `on_change` is called on Escape restoration, but not on input-context
  cancellation. The README documents this choice.
- Tester friction (from `FRICTION.md`):
  - `ApplicationError` is not in `component::*`.
  - `App::new` requires `AppError`, so a `RuntimeError`-returning mount
    needs a conversion.
  - A guessed guide path (`custom_components`) did not exist.
- The session took 2.6 minutes from brief to end.

## Verdict

acceptance=met; accepted=2026-10-28T17:17:09+01:00 (the session end time
from `RUN_DIR/conditions.env`).
