# Run summary: r3-6-baseline-codex-2

Exploratory evidence from one AI tester run. Counts describe how this tester worked; they are not a usability score.

- Tester: codex (codex-cli 0.165.0), requested model gpt-6.1-sol, effective model not in the transcript, effort medium
- Task: 6-notes-editor.md; budget 90 min; session outcome ended (exit 0)
- Kit: commit 26e6b5a308df, version 0.3.75, rustc 1.99.1 (58cf8f9c3 2026-10-13), third-party cache none
- Brief to end of session: 7.6 min
- Brief to accepted completion: n/a (acceptance not recorded)
- Acceptance: not recorded
- Time to first build: 2.0 min
- Builds: 11; failed 7; killed or unfinished 0; total build time 0.9 min
- First successful run or test (candidate; the author checks it exercised the app): `cargo test --offline -- --nocapture`, 1 tests passed, build 9, 6.0 min after the brief

## Errors by code (occurrences across builds)

- `E0425`: 2
- `E0433`: 1

## Warnings by code or lint (occurrences across builds)

- `unused_must_use`: 5

## pert_ui paths used (candidate inventory, verify against the source)

`Length`, `ListItems`, `ListSelection`, `ListSource`, `SelectionMode`, `Semantics`, `SplitAxis`, `SplitPane`, `SplitSizing`, `Toast`, `ToastController`, `ToastHost`, `prelude::*`, `testing::*`

## Code size and API surface (definitions in summarize.py, frozen for round 2)

- Sources: `work`
- Code lines: application 394, test 261, total 655
- Distinct pert-ui types (candidates): 38 (application 34, test 7)
- Import routes: prelude 1, root 12, module 1 (`testing` 1); glob imports: `pert_ui::prelude::*`, `pert_ui::testing::*`

`Align`, `App`, `Button`, `CaptureRequest`, `Column`, `CommandScope`, `CpuRenderer`, `Dialog`, `DialogController`, `Driver`, `Dynamic`, `Element`, `KeyChord`, `Label`, `Length`, `ListItems`, `ListSelection`, `ListSource`, `MenuBar`, `MenuItem`, `ReadSignal`, `Row`, `Runtime`, `SelectionMode`, `Signal`, `SplitAxis`, `SplitPane`, `SplitSizing`, `Tab`, `Tabs`, `TextArea`, `TextField`, `Toast`, `ToastController`, `ToastHost`, `Viewport`, `VirtualList`, `WidgetId`

## Proxy and model-host mentions in the transcript (read each)
