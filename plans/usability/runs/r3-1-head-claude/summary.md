# Run summary: r3-1-head-claude

Exploratory evidence from one AI tester run. Counts describe how this tester worked; they are not a usability score.

- Tester: claude (2.1.312 (Claude Code)), requested model claude-opus-5-5, effective model claude-opus-5-5, effort high
- Task: 1-settings-editor.md; budget 90 min; session outcome ended (exit 0)
- Kit: commit 32dd9c4f667b, version 0.3.88, rustc 1.99.1 (58cf8f9c3 2026-10-13), third-party cache none
- Brief to end of session: 7.6 min
- Brief to accepted completion: 7.6 min
- Acceptance: met
- Time to first build: 0.8 min
- Builds: 20; failed 5; killed or unfinished 0; total build time 0.9 min
- First successful run or test (candidate; the author checks it exercised the app): `cargo test --test explore -- --nocapture`, 1 tests passed, build 5, 3.1 min after the brief

## Errors by code (occurrences across builds)

- `E0599`: 3
- `E0609`: 1
- `E0618`: 1

## Warnings by code or lint (occurrences across builds)

- `unused_imports`: 3

## pert_ui paths used (candidate inventory, verify against the source)

`AppError`, `prelude::*`, `testing::*`

## Code size and API surface (definitions in summarize.py, frozen for round 2)

- Sources: `work`
- Code lines: application 307, test 404, total 711
- Distinct pert-ui types (candidates): 32 (application 24, test 12)
- Import routes: prelude 1, root 1, module 1 (`testing` 1); glob imports: `pert_ui::prelude::*`, `pert_ui::testing::*`

`Align`, `App`, `Button`, `CaptureRequest`, `Choice`, `Column`, `CpuRenderer`, `Detail`, `Driver`, `Field`, `FieldHandle`, `FormFields`, `Group`, `HeadlessApp`, `InputRejection`, `InspectValue`, `InspectionQuery`, `IntegerInput`, `Label`, `NativeCloseAction`, `Path`, `ReadSignal`, `Row`, `Runtime`, `ScrollArea`, `Select`, `Selector`, `Signal`, `TextField`, `TextStyle`, `Toggle`, `WidgetId`

## Proxy and model-host mentions in the transcript (read each)
