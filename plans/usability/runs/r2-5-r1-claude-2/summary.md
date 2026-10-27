# Run summary: r2-5-r1-claude-2

Exploratory evidence from one AI tester run. Counts describe how this tester worked; they are not a usability score.

- Tester: claude (2.1.311 (Claude Code)), requested model opus, effective model claude-opus-5-5, effort high
- Task: 5-job-monitor.md; budget 90 min; session outcome ended (exit 0)
- Kit: commit 359122e38ec7, version 0.3.52, rustc 1.99.1 (58cf8f9c3 2026-10-13), third-party cache none
- Brief to end of session: 9.1 min
- Brief to accepted completion: 9.1 min
- Acceptance: met
- Time to first build: 2.6 min
- Builds: 20; failed 4; killed or unfinished 0; total build time 0.7 min
- First successful run or test (candidate; the author checks it exercised the app): `cargo run -q --example probe`, build 6, 6.7 min after the brief

## Errors by code (occurrences across builds)


## Warnings by code or lint (occurrences across builds)

- `unused_must_use`: 1

## pert_ui paths used (candidate inventory, verify against the source)

`ApplicationError`, `Capacity`, `CaptureRequest`, `CaptureTarget`, `InspectValue`, `KeyAction`, `KeyCode`, `Length`, `ListSelection`, `Point`, `PointerButton`, `PointerEvent`, `PointerKind`, `ProgressStatus`, `SelectionMode`, `SendError`, `SortDirection`, `Table`, `TableColumn`, `TableController`, `TableItems`, `TableSort`, `TableSource`, `TextOverflow::Ellipsis`, `Toast`, `ToastHost`, `ToastQueue`, `ToastReceipt`, `ToastStatus`, `UiSender`, `Viewport`, `cpu::CpuRenderer`, `prelude::*`

## Code size and API surface (definitions in summarize.py, frozen for round 2)

- Sources: `work`
- Code lines: application 578, test 389, total 967
- Distinct pert-ui types (candidates): 38 (application 32, test 12)
- Import routes: prelude 1, root 31, module 1 (`cpu` 1); glob imports: `pert_ui::prelude::*`

`App`, `ApplicationError`, `Button`, `Capacity`, `CaptureRequest`, `Column`, `Component`, `CpuRenderer`, `Element`, `InspectValue`, `KeyAction`, `Label`, `Length`, `ListSelection`, `Path`, `Point`, `PointerEvent`, `ProgressBar`, `Row`, `Runtime`, `SelectionMode`, `SendError`, `Signal`, `SortDirection`, `Table`, `TableColumn`, `TableController`, `TableItems`, `TableSort`, `TableSource`, `Toast`, `ToastHost`, `ToastQueue`, `ToastReceipt`, `ToastStatus`, `UiSender`, `Viewport`, `WidgetId`

## Proxy and model-host mentions in the transcript (read each)
