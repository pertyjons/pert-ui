# Run summary: r2-5-head-codex-1

Exploratory evidence from one AI tester run. Counts describe how this tester worked; they are not a usability score.

- Tester: codex (codex-cli 0.164.0), requested model gpt-6.1-sol, effective model not in the transcript, effort medium
- Task: 5-job-monitor.md; budget 90 min; session outcome ended (exit 0)
- Kit: commit aca8df366fde, version 0.3.75, rustc 1.99.1 (58cf8f9c3 2026-10-13), third-party cache none
- Brief to end of session: 4.6 min
- Brief to accepted completion: 4.6 min
- Acceptance: met
- Time to first build: 1.5 min
- Builds: 5; failed 0; killed or unfinished 0; total build time 0.7 min
- First successful run or test (candidate; the author checks it exercised the app): `cargo test --offline --test headless -- --nocapture`, 1 tests passed, build 3, 2.9 min after the brief

## Errors by code (occurrences across builds)


## Warnings by code or lint (occurrences across builds)


## pert_ui paths used (candidate inventory, verify against the source)

`Capacity`, `Mailbox`, `SendError::Full`, `UiSender`, `prelude::*`, `testing::*`

## Code size and API surface (definitions in summarize.py, frozen for round 2)

- Sources: `work`
- Code lines: application 334, test 199, total 533
- Distinct pert-ui types (candidates): 31 (application 23, test 9)
- Import routes: prelude 1, root 4, module 1 (`testing` 1); glob imports: `pert_ui::prelude::*`, `pert_ui::testing::*`

`App`, `Banner`, `Button`, `Capacity`, `CaptureRequest`, `Column`, `CpuRenderer`, `Detail`, `Driver`, `Element`, `InspectionQuery`, `KeyAction`, `Label`, `ListSelection`, `Mailbox`, `ProgressBar`, `Runtime`, `SelectionMode`, `Selector`, `SendError`, `Signal`, `SortDirection`, `Table`, `TableColumn`, `TableController`, `TableItems`, `TableSort`, `TableSource`, `UiSender`, `Viewport`, `WidgetId`

## Proxy and model-host mentions in the transcript (read each)
