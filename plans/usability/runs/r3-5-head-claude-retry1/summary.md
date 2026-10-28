# Run summary: r3-5-head-claude-retry1

Exploratory evidence from one AI tester run. Counts describe how this tester worked; they are not a usability score.

- Tester: claude (2.1.312 (Claude Code)), requested model claude-opus-5-5, effective model claude-opus-5-5, effort high
- Task: 5-job-monitor.md; budget 90 min; session outcome ended (exit 0)
- Kit: commit 32dd9c4f667b, version 0.3.88, rustc 1.99.1 (58cf8f9c3 2026-10-13), third-party cache none
- Brief to end of session: 6.6 min
- Brief to accepted completion: 6.6 min
- Acceptance: met
- Time to first build: 1.5 min
- Builds: 14; failed 1; killed or unfinished 0; total build time 0.5 min
- First successful run or test (candidate; the author checks it exercised the app): `cargo test`, 7 tests passed, build 5, 5.4 min after the brief

## Errors by code (occurrences across builds)

- `E0425`: 3
- `E0433`: 2
- `E0277`: 1

## Warnings by code or lint (occurrences across builds)


## pert_ui paths used (candidate inventory, verify against the source)

`AppError`, `live::*`, `prelude::*`, `testing::*`

## Code size and API surface (definitions in summarize.py, frozen for round 2)

- Sources: `work`
- Code lines: application 509, test 277, total 786
- Distinct pert-ui types (candidates): 35 (application 28, test 8)
- Import routes: prelude 1, root 1, module 2 (`live` 1, `testing` 1); glob imports: `pert_ui::live::*`, `pert_ui::prelude::*`, `pert_ui::testing::*`

`App`, `ApplicationError`, `Button`, `Capacity`, `CaptureRequest`, `ClickCounter`, `Column`, `CpuRenderer`, `Driver`, `Element`, `FrameImage`, `Label`, `Length`, `ListSelection`, `Mailbox`, `Path`, `ProgressBar`, `Row`, `Runtime`, `SelectionMode`, `SendError`, `Signal`, `SortDirection`, `Table`, `TableColumn`, `TableController`, `TableItems`, `TableSort`, `TableSource`, `Toast`, `ToastController`, `ToastHost`, `UiSender`, `Viewport`, `WidgetId`

## Proxy and model-host mentions in the transcript (read each)
