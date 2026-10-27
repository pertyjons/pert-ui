# Run summary: r2-3-codex

Exploratory evidence from one AI tester run. Counts describe how this tester worked; they are not a usability score.

- Tester: codex (codex-cli 0.164.0), requested model gpt-6.1-sol, effective model not in the transcript, effort medium
- Task: 3-custom-control.md; budget 90 min; session outcome ended (exit 0)
- Kit: commit aca8df366fde, version 0.3.75, rustc 1.99.1 (58cf8f9c3 2026-10-13), third-party cache none
- Brief to end of session: 3.6 min
- Brief to accepted completion: 3.6 min
- Acceptance: met
- Time to first build: 1.3 min
- Builds: 5; failed 2; killed or unfinished 0; total build time 0.7 min
- First successful run or test (candidate; the author checks it exercised the app): `cargo test --offline -- --nocapture`, 1 tests passed, build 3, 2.7 min after the brief

## Errors by code (occurrences across builds)

- `E0433`: 5
- `E0277`: 3
- `E0533`: 2

## Warnings by code or lint (occurrences across builds)

- `unused_imports`: 2

## pert_ui paths used (candidate inventory, verify against the source)

`InspectValue`, `ThemeMode`, `component::*`, `prelude::*`, `testing::*`

## Code size and API surface (definitions in summarize.py, frozen for round 2)

- Sources: `work`
- Code lines: application 426, test 0, total 426
- Distinct pert-ui types (candidates): 41 (application 41, test 0)
- Import routes: prelude 1, root 2, module 2 (`component` 1, `testing` 1); glob imports: `pert_ui::component::*`, `pert_ui::prelude::*`, `pert_ui::testing::*`

`App`, `Button`, `CaptureRequest`, `Column`, `Component`, `ComponentDetails`, `Constraints`, `Container`, `CpuRenderer`, `DecimalInput`, `Detail`, `Driver`, `Element`, `EventContext`, `InputOutcome`, `InspectValue`, `InspectionQuery`, `Invalidation`, `KeyChord`, `KeyEvent`, `KeyTransition`, `KeyboardState`, `Label`, `Length`, `MeasureContext`, `Painter`, `Path`, `Point`, `PointerEvent`, `Rect`, `Row`, `Runtime`, `Selector`, `Signal`, `Size`, `SizeLimits`, `Slider`, `ThemeMode`, `UpdateContext`, `Viewport`, `WidgetId`

## Proxy and model-host mentions in the transcript (read each)
