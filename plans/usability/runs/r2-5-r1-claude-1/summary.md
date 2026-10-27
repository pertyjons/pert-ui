# Run summary: r2-5-r1-claude-1

Exploratory evidence from one AI tester run. Counts describe how this tester worked; they are not a usability score.

- Tester: claude (2.1.311 (Claude Code)), requested model opus, effective model claude-opus-5-5, effort high
- Task: 5-job-monitor.md; budget 90 min; session outcome ended (exit 0)
- Kit: commit 359122e38ec7, version 0.3.52, rustc 1.99.1 (58cf8f9c3 2026-10-13), third-party cache none
- Brief to end of session: 10.3 min
- Brief to accepted completion: 10.3 min
- Acceptance: met
- Time to first build: 4.3 min
- Builds: 38; failed 8; killed or unfinished 0; total build time 0.9 min
- First successful run or test (candidate; the author checks it exercised the app): `cargo test --test scratch -- --nocapture`, 1 tests passed, build 7, 5.3 min after the brief

## Errors by code (occurrences across builds)

- `E0615`: 4
- `unexpected closing delimiter: `}``: 1

## Warnings by code or lint (occurrences across builds)


## pert_ui paths used (candidate inventory, verify against the source)

`Capacity`, `Container`, `InspectValue`, `ListSelection`, `Mailbox`, `PointerButton`, `PointerEvent`, `PointerKind`, `ProgressBar`, `ProgressStatus`, `SelectionMode`, `SendError`, `SortDirection`, `Table`, `TableColumn`, `TableController`, `TableItems`, `TableSort`, `TableSource`, `Toast`, `ToastHost`, `ToastQueue`, `ToastReceipt`, `ToastStatus`, `UiSender`, `cpu::CpuRenderer`, `prelude::*`

## Code size and API surface (definitions in summarize.py, frozen for round 2)

- Sources: `work`
- Code lines: application 739, test 230, total 969
- Distinct pert-ui types (candidates): 38 (application 35, test 4)
- Import routes: prelude 1, root 25, module 1 (`cpu` 1); glob imports: `pert_ui::prelude::*`

`App`, `Button`, `Capacity`, `Column`, `Container`, `CpuRenderer`, `Element`, `InspectValue`, `Label`, `Length`, `ListSelection`, `Mailbox`, `Path`, `Point`, `PointerEvent`, `ProgressBar`, `ReadSignal`, `Row`, `Runtime`, `SelectionMode`, `SendError`, `Signal`, `Size`, `SortDirection`, `Table`, `TableColumn`, `TableController`, `TableItems`, `TableSort`, `TableSource`, `Toast`, `ToastHost`, `ToastQueue`, `ToastReceipt`, `ToastStatus`, `UiSender`, `Viewport`, `WidgetId`

## Proxy and model-host mentions in the transcript (read each)
