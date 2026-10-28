# Run summary: r3-5-head-codex

Exploratory evidence from one AI tester run. Counts describe how this tester worked; they are not a usability score.

- Tester: codex (codex-cli 0.165.0), requested model gpt-6.1-sol, effective model not in the transcript, effort medium
- Task: 5-job-monitor.md; budget 90 min; session outcome ended (exit 0)
- Kit: commit 32dd9c4f667b, version 0.3.88, rustc 1.99.1 (58cf8f9c3 2026-10-13), third-party cache none
- Brief to end of session: 2.8 min
- Brief to accepted completion: n/a (acceptance not recorded)
- Acceptance: not recorded
- Time to first build: 1.1 min
- Builds: 3; failed 2; killed or unfinished 0; total build time 0.6 min
- First successful run or test (candidate; the author checks it exercised the app): `cargo test --offline`, 1 tests passed, build 3, 2.0 min after the brief

## Errors by code (occurrences across builds)

- `E0308`: 2

## Warnings by code or lint (occurrences across builds)


## pert_ui paths used (candidate inventory, verify against the source)

`AppError`, `SendError::Closed`, `SendError::Full`, `live::Capacity`, `live::Mailbox`, `live::UiSender`, `prelude::*`, `testing::*`

## Code size and API surface (definitions in summarize.py, frozen for round 2)

- Sources: `work`
- Code lines: application 314, test 153, total 467
- Distinct pert-ui types (candidates): 29 (application 25, test 6)
- Import routes: prelude 1, root 3, module 4 (`live` 3, `testing` 1); glob imports: `pert_ui::prelude::*`, `pert_ui::testing::*`

`App`, `Button`, `Capacity`, `CaptureRequest`, `Column`, `CpuRenderer`, `Driver`, `Element`, `Label`, `Length`, `ListSelection`, `Mailbox`, `ProgressBar`, `Row`, `Runtime`, `SelectionMode`, `SendError`, `Signal`, `Size`, `SortDirection`, `Table`, `TableColumn`, `TableController`, `TableItems`, `TableSource`, `UiSender`, `View`, `Viewport`, `WidgetId`

## Proxy and model-host mentions in the transcript (read each)
