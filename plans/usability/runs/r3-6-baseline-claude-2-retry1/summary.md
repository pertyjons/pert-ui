# Run summary: r3-6-baseline-claude-2-retry1

Exploratory evidence from one AI tester run. Counts describe how this tester worked; they are not a usability score.

- Tester: claude (2.1.312 (Claude Code)), requested model claude-opus-5-5, effective model claude-opus-5-5, effort high
- Task: 6-notes-editor.md; budget 90 min; session outcome ended (exit 0)
- Kit: commit 26e6b5a308df, version 0.3.75, rustc 1.99.1 (58cf8f9c3 2026-10-13), third-party cache none
- Brief to end of session: 12.9 min
- Brief to accepted completion: 12.9 min
- Acceptance: met
- Time to first build: 2.1 min
- Builds: 41; failed 15; killed or unfinished 0; total build time 1.3 min
- First successful run or test (candidate; the author checks it exercised the app): `cargo test --test explore -- --nocapture`, 1 tests passed, build 4, 4.5 min after the brief

## Errors by code (occurrences across builds)

- `E0428`: 112
- `unexpected closing delimiter: `}``: 2

## Warnings by code or lint (occurrences across builds)

- `dead_code`: 2

## pert_ui paths used (candidate inventory, verify against the source)

`Dialog`, `DialogController`, `ListItems`, `ListSelection`, `ListSource`, `MenuBar`, `MenuItem`, `SelectionMode`, `SplitAxis`, `SplitPane`, `SplitSizing`, `Tab`, `Tabs`, `Toast`, `ToastController`, `ToastHost`, `VirtualList`, `prelude::*`, `testing::*`

## Code size and API surface (definitions in summarize.py, frozen for round 2)

- Sources: `work`
- Code lines: application 361, test 344, total 705
- Distinct pert-ui types (candidates): 42 (application 36, test 10)
- Import routes: prelude 1, root 17, module 1 (`testing` 1); glob imports: `pert_ui::prelude::*`, `pert_ui::testing::*`

`Align`, `App`, `Button`, `CaptureRequest`, `Column`, `CommandScope`, `CpuRenderer`, `Dialog`, `DialogController`, `Driver`, `Dynamic`, `Element`, `EventContext`, `KeyChord`, `Label`, `Length`, `ListItems`, `ListSelection`, `ListSource`, `MenuBar`, `MenuItem`, `Path`, `Point`, `PointerEvent`, `ReadSignal`, `Row`, `Runtime`, `SelectionMode`, `Signal`, `SplitAxis`, `SplitPane`, `SplitSizing`, `Tab`, `Tabs`, `TextArea`, `TextField`, `Toast`, `ToastController`, `ToastHost`, `Viewport`, `VirtualList`, `WidgetId`

## Proxy and model-host mentions in the transcript (read each)
