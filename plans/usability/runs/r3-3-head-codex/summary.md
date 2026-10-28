# Run summary: r3-3-head-codex

Exploratory evidence from one AI tester run. Counts describe how this tester worked; they are not a usability score.

- Tester: codex (codex-cli 0.165.0), requested model gpt-6.1-sol, effective model not in the transcript, effort medium
- Task: 3-custom-control.md; budget 90 min; session outcome ended (exit 0)
- Kit: commit 32dd9c4f667b, version 0.3.88, rustc 1.99.1 (58cf8f9c3 2026-10-13), third-party cache none
- Brief to end of session: 2.6 min
- Brief to accepted completion: n/a (acceptance not recorded)
- Acceptance: not recorded
- Time to first build: 1.1 min
- Builds: 3; failed 1; killed or unfinished 0; total build time 0.7 min
- First successful run or test (candidate; the author checks it exercised the app): `cargo test --offline`, 1 tests passed, build 2, 2.0 min after the brief

## Errors by code (occurrences across builds)

- `E0433`: 1
- `E0271`: 1

## Warnings by code or lint (occurrences across builds)


## pert_ui paths used (candidate inventory, verify against the source)

`ApplicationError`, `component::*`, `prelude::*`, `testing::*`

## Code size and API surface (definitions in summarize.py, frozen for round 2)

- Sources: `work`
- Code lines: application 259, test 152, total 411
- Distinct pert-ui types (candidates): 36 (application 29, test 10)
- Import routes: prelude 1, root 1, module 2 (`component` 1, `testing` 1); glob imports: `pert_ui::component::*`, `pert_ui::prelude::*`, `pert_ui::testing::*`

`App`, `ApplicationError`, `Button`, `CaptureRequest`, `Column`, `Component`, `ComponentDetails`, `Constraints`, `CpuRenderer`, `DecimalInput`, `Driver`, `EventContext`, `InputOutcome`, `InspectValue`, `KeyChord`, `KeyEvent`, `KeyTransition`, `KeyboardState`, `Label`, `Length`, `MeasureContext`, `Painter`, `Point`, `PointerEvent`, `Rect`, `Row`, `Runtime`, `Shape`, `Signal`, `Size`, `SizeLimits`, `Slider`, `ThemeMode`, `UpdateContext`, `Viewport`, `WidgetId`

## Proxy and model-host mentions in the transcript (read each)
