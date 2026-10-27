# Acceptance: r2-3-codex (task 3, custom point picker)

Author check, 2026-10-27, kit `kit-aca8df3` (pert-ui 0.3.75). The submitted
`work/` (without `target/`, `.claude/` and `.git/`) was copied to
`~/pert-ui-usability/acceptance-r2-3-codex`, built with
`cargo build --offline --all-targets` and tested with `cargo test --offline`
in the kit sandbox: builds without warnings (also after touching every source
file); the single headless test `headless::acceptance` (`src/headless.rs`,
a unit test of the binary) passes. The headless mode
(`cargo run --offline -- --headless`) was rerun; it wrote
`artifacts/dark.png`, `artifacts/light.png` and `artifacts/inspection.json`
byte-identical to the submitted ones. The check mounts the application's own
`mount` tree (`src/main.rs:5-60`, the same function `App::new` calls) in a
`Runtime` with a 640x420 viewport and drives it with simulated pointer input,
`Driver::key`, `Driver::chord` (Shift), `replace_text` + Enter and `click`,
reading every value from inspection (`InspectionQuery` on the picker's `x`/`y`
properties, the fields' semantic values, `Runtime::inspect`). No author
acceptance check was needed; one triage probe was added (below).

| Item | Result | Code | Check |
|---|---|---|---|
| 1. Drag, including outside the area, reads the clamped value | met | `src/picker.rs:90-115` (press captures the pointer and records the pre-press point, moves write live, release ends the drag), `write` `:35-53` (clamp), `move_to` `:54-64` | `src/headless.rs:51-71` (press at the centre, move to (0.75, 0.25), move far right/above to (1, 0), release there; picker inspection (1, 0), x/y fields' semantics 1.0 / 0.0); `:73-75` (far left/below: (0, 1)) |
| 2. Cancel a drag, old value restored | met | Escape in `keyboard_event` `src/picker.rs:132-135`, `restore` `:65-71`; `focus_lost` `:145-147`; `cancel_input` `:148-153` | `src/headless.rs:73-80` (from (1, 0), drag to (0, 1), `Driver::key(Escape)` restores (1, 0); later release and move change nothing); `:106-111` (focus loss restores (0.5, 0.5)) |
| 3. Arrow keys with and without Shift | met | `src/picker.rs:116-144` (0.01, 0.1 with `keyboard.modifiers().shift`; y grows downwards) | `src/headless.rs:84-96` (focused picker: Right/Up to (0.51, 0.49); Shift+Left/Down via `Driver::chord` to (0.41, 0.59), read from inspection) |
| 4. Field edit moves the handle; Reset restores (0.5, 0.5) | met | `src/main.rs:22-44` (picker and `DecimalInput`s share the `x`/`y` signals; Reset `:41-44`) | `src/headless.rs:98-104` (types 0.23 / 0.78 + Enter, picker inspection (0.23, 0.78); clicks Reset, (0.5, 0.5)); `:82-83` (Reset after a drag); picker to fields `:64-71` |
| 5. CPU PNG captures, dark and light, focused picker | met | `src/headless.rs:113-115` (focus), `:136-153` (`set_mode`, `request_capture`, `CpuRenderer::serve_captures`, `encode_png`) | headless mode and test; `work/artifacts/dark.png`, `work/artifacts/light.png` (640x420, reproduced byte-identically by the author) |
| 6. Value in an inspection snapshot | met | `src/picker.rs:190-210` (`Slider`, text value `x=…, y=…`, `x`/`y` `InspectValue::Number`) | `src/headless.rs:116-134` (`Runtime::inspect`, entry found by `inspection_id`, `x`/`y` 0.5; snapshot written to `artifacts/inspection.json` with `focused: true`) |

## Task-specific checks

- **Reusable type implementing the component contract:** met. `PointPicker`
  (`src/picker.rs:6-211`) is a public library type implementing `Component`
  (`update`, `focusable`, `size_limits`, `measure`, `pointer_input`,
  `keyboard_event`, `focus_lost`, `cancel_input`, `deliver_notifications`,
  `paint`, `inspect`), constructed from two `Signal<f64>` with an
  `on_change` builder; the owner sets the value through the signals. Its
  size is a fixed 260 px square with hard-coded geometry (12 px inset,
  236 px track), not a theme metric.
- **Drag beyond the area clamps:** met (item 1).
- **Cancellation restores the pre-press value:** met (item 2), for Escape
  and focus loss. Escape's restore is reported through `on_change`; focus
  loss and `cancel_input` restore without reporting (documented,
  `src/picker.rs:26-27`).
- **Shift changes the arrow step:** met (item 3).
- **Focus indication and colors from the theme:** met. All colors are
  `ColorRole`s (`SurfaceContainer`, `Outline`, `Primary`, `OnPrimary`); the
  border becomes `ColorRole::FocusRing` when `focus_within()`
  (`src/picker.rs:160-189`), drawn as four 2 px inward rectangles rather
  than with `MetricRole::FocusWidth`. The two captures differ in colors:
  background (600,400) rgb 36,41,50 dark / 246,248,252 light; area (60,100)
  47,52,61 / 229,234,241; crosshair 153,162,175 / 134,144,158; handle and
  focus border 80,153,237 / 44,111,196. Both show the focus border, the
  handle at (0.5, 0.5) and the fields reading 0.50.
- **Value in an inspection snapshot:** met (item 6).
- **Numeric fields in sync both ways:** met (item 4): fields to picker read
  from the picker's inspection, picker to fields from the fields' semantics.

## Triage notes

Not acceptance failures. From the source only; the transcript and
FRICTION.md were not read for this acceptance.

- `size_limits` calls `unwrap` on a constant size (`src/picker.rs:85`);
  `update` clamps the owner's values but lets NaN through (`:74-80`), which
  the doc comment leaves to the owner.
- Arrow keys move the point with Ctrl or Alt held as well; keys also move it
  during a drag.
- The status line shows the last reported picker input, so after a
  focus-loss restore it keeps the drag's point (visible in both captures as
  "Last picker input: 0.16, 0.80"); documented in README.md.
- Author triage probe `tests/author_escape_route.rs` (acceptance directory
  only), to see which route Escape takes when the picker is not focused
  before the press. Result: `focused_after_press=true restored=(0.5, 0.5)
  on_change_from_escape=1` — the press focuses the picker and Escape reaches
  its `keyboard_event`; no library defect shown.

```rust
use pert_ui::{prelude::*, testing::*};
use point_picker::PointPicker;

#[test]
fn escape_route_during_drag() -> Result<(), Box<dyn std::error::Error>> {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(640., 420., 1.)?)?;
    let x = ui.signal(0.5_f64);
    let y = ui.signal(0.5_f64);
    let reports = ui.signal(0_u32);
    let r = reports.clone();
    let root = ui.mount_tree(Element::new("row", Row::new([
        Element::new("picker", PointPicker::new(x.clone(), y.clone()).on_change(move |_, _| r.update(|n| *n += 1))),
        Element::new("other", Button::new("Other", |_| Ok(()))),
    ])))?;
    let mut d = Driver::new(&mut ui);
    d.settle()?;
    let picker = d.find(&root, &["picker"])?;
    let other = d.find(&root, &["other"])?;
    d.runtime_mut().focus(&other)?;
    d.settle()?;
    let b = d.runtime().bounds(&picker)?.ok_or("no bounds")?;
    d.runtime_mut().pointer_input(PointerEvent::new(Point::new(b.x() + 30., b.y() + 30.)?, PointerKind::Press(PointerButton::Primary)))?;
    d.settle()?;
    let focused_after_press = d.runtime().focused() == Some(picker.clone());
    let reports_before = reports.get()?;
    d.key(KeyCode::Escape)?;
    println!("focused_after_press={focused_after_press} restored=({}, {}) on_change_from_escape={}",
        x.get()?, y.get()?, reports.get()? - reports_before);
    Ok(())
}
```
