# Acceptance: r1-3-codex (task 3, custom point picker)

Author check, 2026-10-21. The submitted `work/` (without `target/`) was
copied to `~/pert-ui-usability/acceptance-r1-3-codex`, its `artifacts/` removed
so they are regenerated, and built and tested in the kit sandbox: `cargo build
--all-targets` builds without warnings (`cargo clippy --all-targets` is clean
for the submission as well); `cargo test` passes the one headless test
(`headless::tests::application_acceptance`, which calls the same
`headless::check` session that `cargo run` runs by default); `cargo run`
(headless mode) passes and writes `artifacts/dark.png`, `light.png` and
`inspection.json`. The regenerated PNGs are byte-identical to the submitted
ones. The `--window` mode was not run (no display; the tester says so too).

The tester's session drives the app with `Runtime::pointer_input`,
`command_key_input` with `native_modifiers` (Shift, primary+A) and
`text_input`, and reads the picker's value only from `ui.inspect()` (custom
`properties["value"]` object). Its picker-to-field direction of the sync was
checked only by reading the application's own `x`/`y` signals, and focus and
theme use only by the two captures, so an author test was added (below).

Author test: `acceptance-r1-3-codex/work/tests/author.rs`
(`author_sync_cancel_focus_theme`), passing in the sandbox. It drags beyond
the lower-right corner and reads `1.00`/`1.00` from the numeric fields' own
inspection `text`; cancels a drag with the library's `Runtime::cancel_input`
(not Escape) and sees the pre-press value restored in picker and fields, and
that capture is released; sets the owner's signal and reads it back from
inspection; moves by ArrowLeft and Shift+ArrowUp and reads the fields
(`0.29`, `0.50`); captures the window focused and unfocused in both themes
and finds the picker's edge pixels differ; overrides the dark palette's
`FocusRing` (magenta) and `Surface` (green) with `ThemeManager::set_color` and
finds both in the picker's pixels (`artifacts/author-overridden.png`).

| Item | Result | Code | Check |
|---|---|---|---|
| 1. Drag, including outside, reads clamped value | met | `src/picker.rs:70-81` (`at`, local position over size) and `:10-23` (`PickPoint::new` clamps); pointer capture on press `:108-113`, moves while armed `:114`, release `:115-119` | `src/headless.rs:55-79`: press center, move to (0.75, 0.25), move beyond top-right reads (1, 0) from inspection, release outside, later move does not change it; author test beyond bottom-right |
| 2. Cancel restores old value | met | Escape while dragging `src/picker.rs:143-146`; `cancel_input` `:152-157`; both via `restore` `:63-69` | `src/headless.rs:80-91` (press, drag outside, Escape, (1, 0) back from inspection); author test via `Runtime::cancel_input` |
| 3. Arrow keys with and without Shift | met | `src/picker.rs:124-151`, step `:136` from `KeyboardState::modifiers().shift` | `src/headless.rs:92-103` (0.01 then 0.1 steps, read from inspection); author test |
| 4. Field edit moves handle; Reset restores (0.5, 0.5) | met | fields to picker `src/lib.rs:26-42` (`DecimalInput::on_save` writes the point); picker to fields `:16-24` (`on_change`); Reset `:44-48` writes all three signals | `src/headless.rs:104-120` (select-all, `text_input`, Enter in each field, picker read from inspection), `:121-136` (Reset by pointer click, inspection); the reverse direction is checked there only through the app's signals, so the author test reads the fields' inspection |
| 5. CPU captures, dark and light, focused picker | met | focus ring `src/picker.rs:206-213` (`FocusRing`, `MetricRole::FocusWidth`, `focus_within`); all colors are `ColorRole`s `:175-205` | `src/headless.rs:137-158` (focus, `set_mode`, window capture served by `CpuRenderer`, asserts focused in inspection); author test (focused vs unfocused, palette overrides) |
| 6. Value in an inspection snapshot | met | `src/picker.rs:158-174`: `properties["value"]` = `{x, y}` numbers, plus `Semantics` role Slider with a text value | every value check in `src/headless.rs` reads it (`observed`, `:19-36`); `artifacts/inspection.json` contains it |

Task-specific checks from `acceptance.md`:

- Reusable type implementing the component contract: met. `PointPicker`
  (`src/picker.rs:28-82`) owns a `Signal<PickPoint>`, takes an `on_change`
  callback and `impl Component` (`:83`) with focus, measure, pointer, key,
  cancel, inspect and paint.
- Drag beyond the area clamps: met (item 1).
- Cancellation restores the pre-press value: met, both by Escape (tester)
  and by the library's input cancellation (author test).
- Shift changes the arrow step: met (item 3).
- Focus indication and colors from the theme: met. Both captures were
  opened: dark shows a dark-grey surface, light-grey crosshair, blue handle
  and a blue focus border; light shows a white surface, dark crosshair, blue
  handle and a blue focus border; the fields read `0.50`/`0.50` after Reset
  in both. The author test confirms the border is the theme's `FocusRing`,
  absent when unfocused, and the surface the theme's `Surface`.
- Value in an inspection snapshot: met (item 6).
- Numeric fields in sync both ways: met. Fields to picker by the tester's
  check; picker to fields (drag, keys, cancellation) by the author test
  through the fields' inspection.

Notes for the triage (not acceptance failures):

- Three separate signals (`point`, `x`, `y`) synchronized by callbacks rather
  than one source; the fields follow only user changes in the picker, so an
  owner write of `point` (other than Reset, which writes all three) would
  leave the fields stale. The application has no such write.
- `cancel_input` cannot return an error, so the tester stores a failed
  restore and returns it from the next `update` (`src/picker.rs:85-87`,
  `:152-157`); this is the friction entry at 15:50.
- `size_limits` uses `.unwrap()` twice in a production path
  (`src/picker.rs:96-97`); the control has a fixed 260 × 260 size.
  `PickPoint::new` replaces non-finite values with 0.5 instead of rejecting
  them.
- The control requests focus on press and handles Escape only while a drag
  is armed; `Leave` is ignored, as the capture continues.
- The tester read the documentation by converting the rustdoc HTML under
  `/kit/doc` to text with its own Python `HTMLParser` script, then deleted
  the helper files (`read_docs.py`, `api.txt` and others) before ending.
- The only `/kit/vendor/pert-ui*/src` path in the transcripts is
  `pert-ui-runtime-0.3.52/src/capture.rs:90:12`, the "associated function
  defined here" note of the `CaptureTarget::window` type error in compiler
  output. No library source was read; the friction log needs no *source*
  entry.
- No warnings and no discarded `#[must_use]` values; the captures use
  `set_capture_support`, `request_capture` and `CpuRenderer::serve_captures`
  as documented.
- Friction times: brief delivered 15:48:16, ended 15:52:06; entries at 15:50
  and 15:51 match the session record (the 15:50 entry was written right
  after a `date +%H:%M`; the second was first written as 15:52 in the same
  command as `date` and corrected to 15:51 once the output was seen). The
  Closing's claims (first `cargo check` passed, first `cargo test` failed on
  the capture argument, then passed) match `log/builds.jsonl`.
