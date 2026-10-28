# Run summary: r3-6-baseline-claude-1

Exploratory evidence from one AI tester run. Counts describe how this tester worked; they are not a usability score.

- Tester: claude (2.1.312 (Claude Code)), requested model claude-opus-5-5, effective model claude-opus-5-5, effort high
- Task: 6-notes-editor.md; budget 90 min; session outcome ended (exit 0)
- Kit: commit 26e6b5a308df, version 0.3.75, rustc 1.99.1 (58cf8f9c3 2026-10-13), third-party cache none
- Brief to end of session: 11.6 min
- Brief to accepted completion: 11.6 min
- Acceptance: met
- Time to first build: 3.0 min
- Builds: 20; failed 11; killed or unfinished 0; total build time 1.5 min
- First successful run or test (candidate; the author checks it exercised the app): `cargo test --test explore -- --nocapture`, 1 tests passed, build 4, 7.0 min after the brief

## Errors by code (occurrences across builds)

- `E0425`: 1

## Warnings by code or lint (occurrences across builds)

- `unused_imports`: 1

## pert_ui paths used (candidate inventory, verify against the source)

`Align`, `Banner`, `CommandScope`, `Detail`, `Dialog`, `DialogController`, `Dynamic`, `InspectionQuery`, `KeyChord`, `Length`, `ListItems`, `ListSelection`, `ListSource`, `MenuBar`, `MenuItem`, `SelectionMode`, `Selector`, `SemanticRole`, `Semantics`, `SplitAxis`, `SplitPane`, `SplitSizing`, `Tab`, `Tabs`, `TextArea`, `VirtualList`, `prelude::*`, `testing::*`

## Code size and API surface (definitions in summarize.py, frozen for round 2)

- Sources: `work`
- Code lines: application 432, test 340, total 772
- Distinct pert-ui types (candidates): 44 (application 35, test 11)
- Import routes: prelude 1, root 26, module 1 (`testing` 1); glob imports: `pert_ui::prelude::*`, `pert_ui::testing::*`

`Align`, `App`, `Banner`, `Button`, `CaptureRequest`, `ClickCounter`, `Column`, `CommandScope`, `CpuRenderer`, `Detail`, `Dialog`, `DialogController`, `Driver`, `Dynamic`, `Element`, `EventContext`, `HeadlessApp`, `InspectionQuery`, `KeyChord`, `Label`, `Length`, `ListItems`, `ListSelection`, `ListSource`, `Menu`, `MenuBar`, `MenuItem`, `Path`, `ReadSignal`, `Row`, `Runtime`, `SelectionMode`, `Selector`, `Signal`, `Size`, `SplitAxis`, `SplitPane`, `SplitSizing`, `Tab`, `Tabs`, `TextArea`, `TextField`, `VirtualList`, `WidgetId`

## Proxy and model-host mentions in the transcript (read each)
