# Run summary: r2-5-r1-codex-2

Exploratory evidence from one AI tester run. Counts describe how this tester worked; they are not a usability score.

- Tester: codex (codex-cli 0.164.0), requested model gpt-6.1-sol, effective model not in the transcript, effort medium
- Task: 5-job-monitor.md; budget 90 min; session outcome ended (exit 0)
- Kit: commit 359122e38ec7, version 0.3.52, rustc 1.99.1 (58cf8f9c3 2026-10-13), third-party cache none
- Brief to end of session: 4.7 min
- Brief to accepted completion: 4.7 min
- Acceptance: met
- Time to first build: 0.5 min
- Builds: 7; failed 1; killed or unfinished 0; total build time 0.6 min
- First successful run or test (candidate; the author checks it exercised the app): `cargo run --offline -- --headless`, build 5, 2.8 min after the brief

## Errors by code (occurrences across builds)

- `E0425`: 1

## Warnings by code or lint (occurrences across builds)

- `unused_imports`: 2

## pert_ui paths used (candidate inventory, verify against the source)

`Capacity`, `CaptureRequest`, `CaptureTarget`, `KeyAction`, `KeyAction::Press`, `KeyAction::Release`, `KeyCode`, `ListSelection`, `Mailbox`, `Modifiers`, `ProgressBar`, `ProgressStatus`, `SelectionMode`, `SendError::Closed`, `SendError::Full`, `SortDirection`, `TableColumn`, `TableController`, `TableItems`, `TableSort`, `TableSource`, `UiSender`, `cpu::CpuRenderer::new`, `prelude::*`

## Code size and API surface (definitions in summarize.py, frozen for round 2)

- Sources: `work`
- Code lines: application 597, test 7, total 604
- Distinct pert-ui types (candidates): 29 (application 29, test 0)
- Import routes: prelude 1, root 22, module 1 (`cpu` 1); glob imports: `pert_ui::prelude::*`

`App`, `Button`, `Capacity`, `CaptureRequest`, `Column`, `CpuRenderer`, `Element`, `KeyAction`, `Label`, `ListSelection`, `Mailbox`, `Modifiers`, `Path`, `ProgressBar`, `Row`, `Runtime`, `SelectionMode`, `SendError`, `Signal`, `SortDirection`, `Table`, `TableColumn`, `TableController`, `TableItems`, `TableSort`, `TableSource`, `UiSender`, `Viewport`, `WidgetId`

## Proxy and model-host mentions in the transcript (read each)
