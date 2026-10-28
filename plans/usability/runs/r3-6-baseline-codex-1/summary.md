# Run summary: r3-6-baseline-codex-1

Exploratory evidence from one AI tester run. Counts describe how this tester worked; they are not a usability score.

- Tester: codex (codex-cli 0.165.0), requested model gpt-6.1-sol, effective model not in the transcript, effort medium
- Task: 6-notes-editor.md; budget 90 min; session outcome ended (exit 0)
- Kit: commit 26e6b5a308df, version 0.3.75, rustc 1.99.1 (58cf8f9c3 2026-10-13), third-party cache none
- Brief to end of session: 5.7 min
- Brief to accepted completion: 5.7 min
- Acceptance: met
- Time to first build: 1.0 min
- Builds: 7; failed 3; killed or unfinished 0; total build time 0.7 min
- First successful run or test (candidate; the author checks it exercised the app): `cargo test --offline -- --nocapture`, 1 tests passed, build 6, 4.0 min after the brief

## Errors by code (occurrences across builds)

- `E0433`: 3

## Warnings by code or lint (occurrences across builds)

- `unused_must_use`: 6

## pert_ui paths used (candidate inventory, verify against the source)

`prelude::*`, `testing::*`, `windowing::*`

## Code size and API surface (definitions in summarize.py, frozen for round 2)

- Sources: `work`
- Code lines: application 397, test 240, total 637
- Distinct pert-ui types (candidates): 37 (application 33, test 7)
- Import routes: prelude 1, root 0, module 2 (`testing` 1, `windowing` 1); glob imports: `pert_ui::prelude::*`, `pert_ui::testing::*`, `pert_ui::windowing::*`

`App`, `Button`, `CaptureRequest`, `Column`, `CommandScope`, `CpuRenderer`, `Dialog`, `DialogController`, `Driver`, `Dynamic`, `Element`, `KeyChord`, `Label`, `Length`, `ListItems`, `ListSelection`, `ListSource`, `MenuBar`, `MenuItem`, `Path`, `Row`, `Runtime`, `SelectionMode`, `Signal`, `SplitAxis`, `SplitPane`, `SplitSizing`, `Tab`, `Tabs`, `TextArea`, `TextField`, `Toast`, `ToastController`, `ToastHost`, `Viewport`, `VirtualList`, `WidgetId`

## Proxy and model-host mentions in the transcript (read each)
