# Run summary: r2-2-codex

Exploratory evidence from one AI tester run. Counts describe how this tester worked; they are not a usability score.

- Tester: codex (codex-cli 0.164.0), requested model gpt-6.1-sol, effective model not in the transcript, effort medium
- Task: 2-task-list.md; budget 90 min; session outcome ended (exit 0)
- Kit: commit aca8df366fde, version 0.3.75, rustc 1.99.1 (58cf8f9c3 2026-10-13), third-party cache none
- Brief to end of session: 3.0 min
- Brief to accepted completion: 3.0 min
- Acceptance: met
- Time to first build: 1.0 min
- Builds: 3; failed 0; killed or unfinished 0; total build time 0.6 min
- First successful run or test (candidate; the author checks it exercised the app): `cargo test --offline`, 3 tests passed, build 2, 2.1 min after the brief

## Errors by code (occurrences across builds)


## Warnings by code or lint (occurrences across builds)

- `dead_code`: 1

## pert_ui paths used (candidate inventory, verify against the source)

`CommandKey`, `CommandScope`, `KeyChord`, `KeyCode`, `Modifiers`, `prelude::*`, `testing::*`

## Code size and API surface (definitions in summarize.py, frozen for round 2)

- Sources: `work`
- Code lines: application 292, test 201, total 493
- Distinct pert-ui types (candidates): 28 (application 26, test 7)
- Import routes: prelude 1, root 5, module 1 (`testing` 1); glob imports: `pert_ui::prelude::*`, `pert_ui::testing::*`

`Align`, `App`, `Button`, `Checkbox`, `CollectionFilter`, `Column`, `CommandKey`, `CommandScope`, `Driver`, `Element`, `InlineText`, `InputRejection`, `KeyChord`, `Label`, `ListDrop`, `ListFilter`, `ListItems`, `ListSelection`, `ListSource`, `Modifiers`, `Row`, `Runtime`, `SelectionMode`, `Signal`, `TextField`, `Viewport`, `VirtualList`, `WidgetId`

## Proxy and model-host mentions in the transcript (read each)
