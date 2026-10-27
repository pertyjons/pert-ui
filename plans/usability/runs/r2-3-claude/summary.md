# Run summary: r2-3-claude

Exploratory evidence from one AI tester run. Counts describe how this tester worked; they are not a usability score.

- Tester: claude (2.1.311 (Claude Code)), requested model opus, effective model claude-opus-5-5, effort high
- Task: 3-custom-control.md; budget 90 min; session outcome ended (exit 0)
- Kit: commit aca8df366fde, version 0.3.75, rustc 1.99.1 (58cf8f9c3 2026-10-13), third-party cache none
- Brief to end of session: 6.3 min
- Brief to accepted completion: 6.3 min
- Acceptance: met
- Time to first build: 1.1 min
- Builds: 15; failed 3; killed or unfinished 0; total build time 0.5 min
- First successful run or test (candidate; the author checks it exercised the app): `cargo test`, 3 tests passed, build 3, 3.7 min after the brief

## Errors by code (occurrences across builds)

- `E0433`: 3
- `E0425`: 1

## Warnings by code or lint (occurrences across builds)

- `unused_imports`: 2

## pert_ui paths used (candidate inventory, verify against the source)

`InspectValue`, `SemanticRole`, `ThemeMode`, `ThemeMode::Dark`, `ThemeMode::Light`, `component::*`, `prelude::*`, `testing::*`, `testing::HeadlessApp`

## Code size and API surface (definitions in summarize.py, frozen for round 2)

- Sources: `work`
- Code lines: application 345, test 160, total 505
- Distinct pert-ui types (candidates): 43 (application 42, test 11)
- Import routes: prelude 1, root 5, module 3 (`component` 1, `testing` 2); glob imports: `pert_ui::component::*`, `pert_ui::prelude::*`, `pert_ui::testing::*`

`Align`, `App`, `ApplicationError`, `Button`, `CaptureRequest`, `Column`, `Component`, `ComponentDetails`, `Constraints`, `Container`, `CpuRenderer`, `DecimalInput`, `Divider`, `Driver`, `Element`, `EventContext`, `Field`, `Form`, `HeadlessApp`, `InputOutcome`, `InspectValue`, `Invalidation`, `KeyChord`, `KeyEvent`, `KeyTransition`, `KeyboardState`, `Length`, `MeasureContext`, `Painter`, `Path`, `Point`, `PointerEvent`, `Rect`, `Row`, `Runtime`, `Shape`, `Signal`, `Size`, `Slider`, `StrokeStyle`, `ThemeMode`, `UpdateContext`, `WidgetId`

## Proxy and model-host mentions in the transcript (read each)
