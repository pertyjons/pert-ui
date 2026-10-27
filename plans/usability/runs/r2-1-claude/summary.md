# Run summary: r2-1-claude

Exploratory evidence from one AI tester run. Counts describe how this tester worked; they are not a usability score.

- Tester: claude (2.1.311 (Claude Code)), requested model opus, effective model claude-opus-5-5, effort high
- Task: 1-settings-editor.md; budget 90 min; session outcome ended (exit 0)
- Kit: commit aca8df366fde, version 0.3.75, rustc 1.99.1 (58cf8f9c3 2026-10-13), third-party cache none
- Brief to end of session: 7.2 min
- Brief to accepted completion: n/a (acceptance partly)
- Acceptance: partly
- Time to first build: 1.9 min
- Builds: 20; failed 3; killed or unfinished 0; total build time 0.8 min
- First successful run or test (candidate; the author checks it exercised the app): `cargo test`, 1 tests passed, build 5, 4.1 min after the brief

## Errors by code (occurrences across builds)

- `E0433`: 3
- `E0425`: 2

## Warnings by code or lint (occurrences across builds)

- `unused_imports`: 6
- `unused import: `time::Duration``: 6
- `unused_must_use`: 1

## pert_ui paths used (candidate inventory, verify against the source)

`AppError`, `InspectValue`, `Semantics`, `prelude::*`, `testing::*`

## Code size and API surface (definitions in summarize.py, frozen for round 2)

- Sources: `work`
- Code lines: application 281, test 287, total 568
- Distinct pert-ui types (candidates): 31 (application 23, test 11)
- Import routes: prelude 1, root 3, module 1 (`testing` 1); glob imports: `pert_ui::prelude::*`, `pert_ui::testing::*`

`App`, `Button`, `CaptureRequest`, `Choice`, `Column`, `CpuRenderer`, `Detail`, `Driver`, `Field`, `FormFields`, `Group`, `HeadlessApp`, `InputRejection`, `InspectValue`, `InspectionQuery`, `IntegerInput`, `Label`, `NativeCloseAction`, `Path`, `ReadSignal`, `Row`, `Runtime`, `ScrollArea`, `ScrollAxes`, `SegmentedControl`, `Selector`, `Signal`, `Size`, `TextField`, `Toggle`, `WidgetId`

## Proxy and model-host mentions in the transcript (read each)
