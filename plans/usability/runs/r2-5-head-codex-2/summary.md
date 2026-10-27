# Run summary: r2-5-head-codex-2

Exploratory evidence from one AI tester run. Counts describe how this tester worked; they are not a usability score.

- Tester: codex (codex-cli 0.164.0), requested model gpt-6.1-sol, effective model not in the transcript, effort medium
- Task: 5-job-monitor.md; budget 90 min; session outcome ended (exit 0)
- Kit: commit aca8df366fde, version 0.3.75, rustc 1.99.1 (58cf8f9c3 2026-10-13), third-party cache none
- Brief to end of session: 4.2 min
- Brief to accepted completion: 4.2 min
- Acceptance: met
- Time to first build: 1.2 min
- Builds: 7; failed 3; killed or unfinished 0; total build time 0.7 min
- First successful run or test (candidate; the author checks it exercised the app): `cargo test --offline`, 1 tests passed, build 5, 3.2 min after the brief

## Errors by code (occurrences across builds)

- `E0277`: 1

## Warnings by code or lint (occurrences across builds)

- `dead_code`: 1
- `unused_must_use`: 1

## pert_ui paths used (candidate inventory, verify against the source)

`Capacity`, `CommandKey`, `Modifiers`, `ProgressStatus`, `SortDirection`, `TableColumn`, `TableController`, `TableItems`, `TableSort`, `TableSource`, `host::HostIntegration`, `prelude::*`, `testing::*`

## Code size and API surface (definitions in summarize.py, frozen for round 2)

- Sources: `work`
- Code lines: application 504, test 7, total 511
- Distinct pert-ui types (candidates): 29 (application 29, test 0)
- Import routes: prelude 1, root 10, module 2 (`host` 1, `testing` 1); glob imports: `pert_ui::prelude::*`, `pert_ui::testing::*`

`Align`, `App`, `Button`, `Capacity`, `CaptureRequest`, `Column`, `CommandKey`, `CpuRenderer`, `Driver`, `Element`, `HostIntegration`, `KeyChord`, `Label`, `ListSelection`, `Modifiers`, `ProgressBar`, `Row`, `Runtime`, `SelectionMode`, `Signal`, `SortDirection`, `Table`, `TableColumn`, `TableController`, `TableItems`, `TableSort`, `TableSource`, `Viewport`, `WidgetId`

## Proxy and model-host mentions in the transcript (read each)
