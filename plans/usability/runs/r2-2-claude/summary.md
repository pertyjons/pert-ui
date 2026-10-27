# Run summary: r2-2-claude

Exploratory evidence from one AI tester run. Counts describe how this tester worked; they are not a usability score.

- Tester: claude (2.1.311 (Claude Code)), requested model opus, effective model claude-opus-5-5, effort high
- Task: 2-task-list.md; budget 90 min; session outcome ended (exit 0)
- Kit: commit aca8df366fde, version 0.3.75, rustc 1.99.1 (58cf8f9c3 2026-10-13), third-party cache none
- Brief to end of session: 6.8 min
- Brief to accepted completion: 6.8 min
- Acceptance: met
- Time to first build: 1.0 min
- Builds: 19; failed 4; killed or unfinished 0; total build time 0.8 min
- First successful run or test (candidate; the author checks it exercised the app): `cargo test`, 1 tests passed, build 3, 3.4 min after the brief

## Errors by code (occurrences across builds)


## Warnings by code or lint (occurrences across builds)

- `dead_code`: 1
- `function `row_title` is never used`: 1
- `function `single_click` is never used`: 1

## pert_ui paths used (candidate inventory, verify against the source)

`AppError`, `CollectionFilter`, `CommandKey`, `HiddenActive`, `ListDrop`, `ListDropPosition`, `ListFilter`, `ListItems`, `ListSelection`, `ListSource`, `Modifiers`, `SelectionMode`, `TextCommand`, `VirtualList`, `prelude::*`, `testing::*`

## Code size and API surface (definitions in summarize.py, frozen for round 2)

- Sources: `work`
- Code lines: application 253, test 282, total 535
- Distinct pert-ui types (candidates): 31 (application 26, test 10)
- Import routes: prelude 1, root 14, module 1 (`testing` 1); glob imports: `pert_ui::prelude::*`, `pert_ui::testing::*`

`App`, `Button`, `CaptureRequest`, `Checkbox`, `ClickCounter`, `CollectionFilter`, `Column`, `CommandKey`, `CommandScope`, `CpuRenderer`, `Driver`, `Element`, `InlineText`, `KeyChord`, `Label`, `ListDrop`, `ListFilter`, `ListItems`, `ListSelection`, `ListSource`, `Modifiers`, `ReadSignal`, `Row`, `Runtime`, `SelectionMode`, `Signal`, `Size`, `TextField`, `Viewport`, `VirtualList`, `WidgetId`

## Proxy and model-host mentions in the transcript (read each)
