# Run summary: r3-6-head-codex-2

Exploratory evidence from one AI tester run. Counts describe how this tester worked; they are not a usability score.

- Tester: codex (codex-cli 0.165.0), requested model gpt-6.1-sol, effective model not in the transcript, effort medium
- Task: 6-notes-editor.md; budget 90 min; session outcome ended (exit 0)
- Kit: commit 32dd9c4f667b, version 0.3.88, rustc 1.99.1 (58cf8f9c3 2026-10-13), third-party cache none
- Brief to end of session: 4.5 min
- Brief to accepted completion: 4.5 min
- Acceptance: met
- Time to first build: 1.8 min
- Builds: 6; failed 3; killed or unfinished 0; total build time 0.9 min
- First successful run or test (candidate; the author checks it exercised the app): `cargo test --offline`, 1 tests passed, build 5, 3.4 min after the brief

## Errors by code (occurrences across builds)

- `E0433`: 3

## Warnings by code or lint (occurrences across builds)


## pert_ui paths used (candidate inventory, verify against the source)

`SplitAxis`, `SplitPane`, `SplitSizing`, `prelude::*`, `testing::*`

## Code size and API surface (definitions in summarize.py, frozen for round 2)

- Sources: `work`
- Code lines: application 409, test 201, total 610
- Distinct pert-ui types (candidates): 35 (application 32, test 4)
- Import routes: prelude 1, root 3, module 1 (`testing` 1); glob imports: `pert_ui::prelude::*`, `pert_ui::testing::*`

`Align`, `App`, `Button`, `CaptureRequest`, `Column`, `CommandKey`, `CommandScope`, `CpuRenderer`, `Dialog`, `DialogController`, `Driver`, `Element`, `KeyChord`, `Label`, `Length`, `ListItems`, `ListSelection`, `ListSource`, `MenuBar`, `MenuItem`, `Modifiers`, `ReadSignal`, `Row`, `Runtime`, `SelectionMode`, `Signal`, `SplitAxis`, `SplitPane`, `SplitSizing`, `Tab`, `Tabs`, `TextArea`, `TextField`, `VirtualList`, `WidgetId`

## Proxy and model-host mentions in the transcript (read each)
