# Run summary: r2-5-head-claude-2

Exploratory evidence from one AI tester run. Counts describe how this tester worked; they are not a usability score.

- Tester: claude (2.1.311 (Claude Code)), requested model opus, effective model claude-opus-5-5, effort high
- Task: 5-job-monitor.md; budget 90 min; session outcome ended (exit 0)
- Kit: commit aca8df366fde, version 0.3.75, rustc 1.99.1 (58cf8f9c3 2026-10-13), third-party cache none
- Brief to end of session: 11.6 min
- Brief to accepted completion: 11.6 min
- Acceptance: met
- Time to first build: 2.0 min
- Builds: 41; failed 5; killed or unfinished 0; total build time 0.9 min
- First successful run or test (candidate; the author checks it exercised the app): `cargo test --test explore -- --nocapture`, 1 tests passed, build 4, 5.1 min after the brief

## Errors by code (occurrences across builds)

- `E0277`: 1
- `E0425`: 1

## Warnings by code or lint (occurrences across builds)

- `unused_imports`: 1

## pert_ui paths used (candidate inventory, verify against the source)

`Capacity`, `Mailbox`, `Semantics`, `SendError`, `SortDirection`, `TableSort`, `UiSender`, `cpu::CpuRenderer`, `host::HostIntegration`, `prelude::*`, `testing::*`

## Code size and API surface (definitions in summarize.py, frozen for round 2)

- Sources: `work`
- Code lines: application 530, test 326, total 856
- Distinct pert-ui types (candidates): 43 (application 34, test 12)
- Import routes: prelude 1, root 7, module 3 (`cpu` 1, `host` 1, `testing` 1); glob imports: `pert_ui::prelude::*`, `pert_ui::testing::*`

`App`, `ApplicationError`, `Button`, `Capacity`, `CaptureRequest`, `ClickCounter`, `Column`, `Container`, `CpuRenderer`, `Driver`, `Element`, `HostIntegration`, `Label`, `Length`, `ListSelection`, `Mailbox`, `Path`, `Point`, `PointerEvent`, `ProgressBar`, `ReadSignal`, `Row`, `Runtime`, `SelectionMode`, `SendError`, `Signal`, `Size`, `SortDirection`, `StateOwner`, `Table`, `TableColumn`, `TableController`, `TableItems`, `TableSort`, `TableSource`, `Toast`, `ToastController`, `ToastHost`, `ToastReceipt`, `ToastStatus`, `UiSender`, `Viewport`, `WidgetId`

## Proxy and model-host mentions in the transcript (read each)
