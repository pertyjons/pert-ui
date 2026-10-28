# Run summary: r3-6-head-claude-2-retry1

Exploratory evidence from one AI tester run. Counts describe how this tester worked; they are not a usability score.

- Tester: claude (2.1.312 (Claude Code)), requested model claude-opus-5-5, effective model claude-opus-5-5, effort high
- Task: 6-notes-editor.md; budget 90 min; session outcome ended (exit 0)
- Kit: commit 32dd9c4f667b, version 0.3.88, rustc 1.99.1 (58cf8f9c3 2026-10-13), third-party cache none
- Brief to end of session: 7.4 min
- Brief to accepted completion: 7.4 min
- Acceptance: met
- Time to first build: 1.7 min
- Builds: 19; failed 6; killed or unfinished 0; total build time 0.6 min
- First successful run or test (candidate; the author checks it exercised the app): `cargo test --test listing -- --nocapture`, 1 tests passed, build 4, 4.1 min after the brief

## Errors by code (occurrences across builds)

- `E0433`: 2
- `E0599`: 1
- `E0425`: 1

## Warnings by code or lint (occurrences across builds)

- `dead_code`: 9
- `unused_imports`: 1

## pert_ui paths used (candidate inventory, verify against the source)

`AppError`, `prelude::*`, `testing::*`, `windowing::*`

## Code size and API surface (definitions in summarize.py, frozen for round 2)

- Sources: `work`
- Code lines: application 395, test 318, total 713
- Distinct pert-ui types (candidates): 46 (application 37, test 11)
- Import routes: prelude 1, root 1, module 2 (`testing` 1, `windowing` 1); glob imports: `pert_ui::prelude::*`, `pert_ui::testing::*`, `pert_ui::windowing::*`

`Align`, `App`, `Button`, `CaptureRequest`, `Choice`, `ClickCounter`, `Column`, `CommandScope`, `CpuRenderer`, `Dialog`, `DialogController`, `Driver`, `Dynamic`, `Element`, `Field`, `InspectionQuery`, `KeyChord`, `Label`, `Length`, `ListItems`, `ListSelection`, `ListSource`, `MenuBar`, `MenuItem`, `Path`, `Point`, `PointerEvent`, `ReadSignal`, `Row`, `Runtime`, `SelectionMode`, `Selector`, `Signal`, `SplitAxis`, `SplitPane`, `SplitSizing`, `Tab`, `Tabs`, `TextArea`, `TextField`, `TextStyle`, `Toast`, `ToastController`, `ToastHost`, `VirtualList`, `WidgetId`

## Proxy and model-host mentions in the transcript (read each)
