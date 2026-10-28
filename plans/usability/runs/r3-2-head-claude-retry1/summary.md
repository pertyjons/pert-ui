# Run summary: r3-2-head-claude-retry1

Exploratory evidence from one AI tester run. Counts describe how this tester worked; they are not a usability score.

- Tester: claude (2.1.312 (Claude Code)), requested model claude-opus-5-5, effective model claude-opus-5-5, effort high
- Task: 2-task-list.md; budget 90 min; session outcome ended (exit 0)
- Kit: commit 32dd9c4f667b, version 0.3.88, rustc 1.99.1 (58cf8f9c3 2026-10-13), third-party cache none
- Brief to end of session: 6.3 min
- Brief to accepted completion: n/a (acceptance not recorded)
- Acceptance: not recorded
- Time to first build: 2.0 min
- Builds: 11; failed 1; killed or unfinished 0; total build time 1.1 min
- First successful run or test (candidate; the author checks it exercised the app): `cargo test`, 4 tests passed, build 3, 3.3 min after the brief

## Errors by code (occurrences across builds)


## Warnings by code or lint (occurrences across builds)


## pert_ui paths used (candidate inventory, verify against the source)

`AppError`, `prelude::*`, `testing::*`

## Code size and API surface (definitions in summarize.py, frozen for round 2)

- Sources: `work`
- Code lines: application 252, test 210, total 462
- Distinct pert-ui types (candidates): 30 (application 25, test 8)
- Import routes: prelude 1, root 1, module 1 (`testing` 1); glob imports: `pert_ui::prelude::*`, `pert_ui::testing::*`

`App`, `ApplicationError`, `Button`, `CaptureRequest`, `Checkbox`, `ClickCounter`, `CollectionFilter`, `Column`, `CommandKey`, `CommandScope`, `CpuRenderer`, `Driver`, `Element`, `InlineText`, `KeyChord`, `Label`, `ListFilter`, `ListItems`, `ListSelection`, `ListSource`, `Modifiers`, `Path`, `ReadSignal`, `Row`, `Runtime`, `SelectionMode`, `Signal`, `TextField`, `VirtualList`, `WidgetId`

## Proxy and model-host mentions in the transcript (read each)
