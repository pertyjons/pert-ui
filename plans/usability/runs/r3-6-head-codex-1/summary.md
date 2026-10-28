# Run summary: r3-6-head-codex-1

Exploratory evidence from one AI tester run. Counts describe how this tester worked; they are not a usability score.

- Tester: codex (codex-cli 0.165.0), requested model gpt-6.1-sol, effective model not in the transcript, effort medium
- Task: 6-notes-editor.md; budget 90 min; session outcome ended (exit 0)
- Kit: commit 32dd9c4f667b, version 0.3.88, rustc 1.99.1 (58cf8f9c3 2026-10-13), third-party cache none
- Brief to end of session: 6.5 min
- Brief to accepted completion: 6.5 min
- Acceptance: met
- Time to first build: 1.8 min
- Builds: 7; failed 3; killed or unfinished 0; total build time 0.8 min
- First successful run or test (candidate; the author checks it exercised the app): `cargo test --offline -- --nocapture`, 1 tests passed, build 3, 3.6 min after the brief

## Errors by code (occurrences across builds)

- `E0599`: 3
- `E0433`: 3
- `E0308`: 1

## Warnings by code or lint (occurrences across builds)


## pert_ui paths used (candidate inventory, verify against the source)

`CommandKey`, `Modifiers`, `prelude::*`, `testing::*`, `windowing::*`

## Code size and API surface (definitions in summarize.py, frozen for round 2)

- Sources: `work`
- Code lines: application 378, test 260, total 638
- Distinct pert-ui types (candidates): 38 (application 33, test 6)
- Import routes: prelude 1, root 2, module 2 (`testing` 1, `windowing` 1); glob imports: `pert_ui::prelude::*`, `pert_ui::testing::*`, `pert_ui::windowing::*`

`App`, `Button`, `CaptureRequest`, `Column`, `CommandKey`, `CommandScope`, `CpuRenderer`, `Dialog`, `DialogController`, `Driver`, `Dynamic`, `Element`, `EventContext`, `KeyChord`, `Label`, `Length`, `ListItems`, `ListSelection`, `ListSource`, `MenuBar`, `MenuItem`, `Modifiers`, `Point`, `PointerEvent`, `ReadSignal`, `Row`, `Runtime`, `SelectionMode`, `Signal`, `SplitAxis`, `SplitPane`, `SplitSizing`, `Tab`, `Tabs`, `TextArea`, `TextField`, `VirtualList`, `WidgetId`

## Proxy and model-host mentions in the transcript (read each)
