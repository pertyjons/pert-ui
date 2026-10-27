# Run summary: r2-5-head-claude-1

Exploratory evidence from one AI tester run. Counts describe how this tester worked; they are not a usability score.

- Tester: claude (2.1.311 (Claude Code)), requested model opus, effective model claude-opus-5-5, effort high
- Task: 5-job-monitor.md; budget 90 min; session outcome ended (exit 0)
- Kit: commit aca8df366fde, version 0.3.75, rustc 1.99.1 (58cf8f9c3 2026-10-13), third-party cache none
- Brief to end of session: 9.6 min
- Brief to accepted completion: 9.6 min
- Acceptance: met
- Time to first build: 1.7 min
- Builds: 21; failed 2; killed or unfinished 0; total build time 1.0 min
- First successful run or test (candidate; the author checks it exercised the app): `cargo test --test explore -- --nocapture`, 1 tests passed, build 3, 4.4 min after the brief

## Errors by code (occurrences across builds)

- `E0599`: 1

## Warnings by code or lint (occurrences across builds)

- `unused_imports`: 1
- `unused_mut`: 1
- `clippy::collapsible_if`: 1

## pert_ui paths used (candidate inventory, verify against the source)

`AppError`, `Capacity`, `CommandKey`, `KeyChord`, `ListSelection`, `Modifiers`, `ProgressBar`, `ProgressStatus`, `SelectionMode`, `SendError`, `SortDirection`, `Table`, `TableColumn`, `TableController`, `TableItems`, `TableSort`, `TableSource`, `Toast`, `ToastController`, `ToastHost`, `ToastReceipt`, `ToastStatus`, `UiSender`, `prelude::*`, `testing::*`

## Code size and API surface (definitions in summarize.py, frozen for round 2)

- Sources: `work`
- Code lines: application 543, test 363, total 906
- Distinct pert-ui types (candidates): 44 (application 30, test 18)
- Import routes: prelude 1, root 23, module 1 (`testing` 1); glob imports: `pert_ui::prelude::*`, `pert_ui::testing::*`

`App`, `Button`, `Capacity`, `CaptureRequest`, `ClickCounter`, `Column`, `CommandKey`, `CpuRenderer`, `Detail`, `Driver`, `Element`, `InspectionQuery`, `KeyChord`, `Label`, `Length`, `ListSelection`, `Modifiers`, `Path`, `Point`, `PointerEvent`, `ProgressBar`, `ReadSignal`, `Row`, `Runtime`, `SelectionMode`, `Selector`, `SendError`, `Signal`, `SortDirection`, `StateOwner`, `Table`, `TableColumn`, `TableController`, `TableItems`, `TableSort`, `TableSource`, `Toast`, `ToastController`, `ToastHost`, `ToastReceipt`, `ToastStatus`, `UiSender`, `Viewport`, `WidgetId`

## Proxy and model-host mentions in the transcript (read each)
