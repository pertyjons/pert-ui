# Run summary: r3-3-head-claude-retry1

Exploratory evidence from one AI tester run. Counts describe how this tester worked; they are not a usability score.

- Tester: claude (2.1.312 (Claude Code)), requested model claude-opus-5-5, effective model claude-opus-5-5, effort high
- Task: 3-custom-control.md; budget 90 min; session outcome ended (exit 0)
- Kit: commit 32dd9c4f667b, version 0.3.88, rustc 1.99.1 (58cf8f9c3 2026-10-13), third-party cache none
- Brief to end of session: 5.7 min
- Brief to accepted completion: 5.7 min
- Acceptance: met
- Time to first build: 1.2 min
- Builds: 15; failed 6; killed or unfinished 0; total build time 0.8 min
- First successful run or test (candidate; the author checks it exercised the app): `cargo test`, 6 tests passed, build 5, 3.7 min after the brief

## Errors by code (occurrences across builds)

- `E0433`: 3
- `E0277`: 1

## Warnings by code or lint (occurrences across builds)

- `clippy::collapsible_if`: 1

## pert_ui paths used (candidate inventory, verify against the source)

`ApplicationError`, `CaptureRequest`, `CaptureTarget`, `DrawCommand`, `ThemeMode`, `Viewport`, `component::*`, `cpu::CpuRenderer`, `prelude::*`, `testing::*`

## Code size and API surface (definitions in summarize.py, frozen for round 2)

- Sources: `work`
- Code lines: application 358, test 264, total 622
- Distinct pert-ui types (candidates): 45 (application 38, test 14)
- Import routes: prelude 1, root 6, module 3 (`component` 1, `cpu` 1, `testing` 1); glob imports: `pert_ui::component::*`, `pert_ui::prelude::*`, `pert_ui::testing::*`

`App`, `ApplicationError`, `Button`, `CaptureRequest`, `Color`, `Column`, `Component`, `ComponentDetails`, `Constraints`, `Container`, `CpuRenderer`, `DecimalInput`, `Detail`, `Driver`, `Element`, `EventContext`, `Field`, `InputOutcome`, `InspectValue`, `InspectionQuery`, `Invalidation`, `KeyChord`, `KeyEvent`, `KeyTransition`, `KeyboardState`, `Label`, `Length`, `MeasureContext`, `Painter`, `Path`, `Point`, `PointerEvent`, `Rect`, `Row`, `Runtime`, `Selector`, `Shape`, `Signal`, `Size`, `Slider`, `StrokeStyle`, `ThemeMode`, `UpdateContext`, `Viewport`, `WidgetId`

## Proxy and model-host mentions in the transcript (read each)
