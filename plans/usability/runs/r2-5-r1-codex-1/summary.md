# Run summary: r2-5-r1-codex-1

Exploratory evidence from one AI tester run. Counts describe how this tester worked; they are not a usability score.

- Tester: codex (codex-cli 0.164.0), requested model gpt-6.1-sol, effective model not in the transcript, effort medium
- Task: 5-job-monitor.md; budget 90 min; session outcome ended (exit 0)
- Kit: commit 359122e38ec7, version 0.3.52, rustc 1.99.1 (58cf8f9c3 2026-10-13), third-party cache none
- Brief to end of session: 3.2 min
- Brief to accepted completion: 3.2 min
- Acceptance: met
- Time to first build: 0.8 min
- Builds: 5; failed 2; killed or unfinished 0; total build time 0.6 min
- First successful run or test (candidate; the author checks it exercised the app): `cargo run --offline -- --headless`, build 4, 2.6 min after the brief

## Errors by code (occurrences across builds)

- `E0599`: 3
- `E0425`: 1
- `E0308`: 1
- `E0277`: 1

## Warnings by code or lint (occurrences across builds)


## pert_ui paths used (candidate inventory, verify against the source)

`*`, `cpu::CpuRenderer::new`, `prelude::*`

## Code size and API surface (definitions in summarize.py, frozen for round 2)

- Sources: `work`
- Code lines: application 582, test 7, total 589
- Distinct pert-ui types (candidates): 29 (application 29, test 0)
- Import routes: prelude 1, root 1, module 1 (`cpu` 1); glob imports: `pert_ui::*`, `pert_ui::prelude::*`

`App`, `Banner`, `Button`, `Capacity`, `CaptureRequest`, `Column`, `CpuRenderer`, `Element`, `KeyAction`, `Label`, `ListSelection`, `Mailbox`, `Modifiers`, `ProgressBar`, `Runtime`, `SelectionMode`, `SendError`, `Signal`, `SortDirection`, `StateOwner`, `Table`, `TableColumn`, `TableController`, `TableItems`, `TableSort`, `TableSource`, `UiSender`, `Viewport`, `WidgetId`

## Proxy and model-host mentions in the transcript (read each)
