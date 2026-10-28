# Run summary: r3-2-head-codex

Exploratory evidence from one AI tester run. Counts describe how this tester worked; they are not a usability score.

- Tester: codex (codex-cli 0.165.0), requested model gpt-6.1-sol, effective model not in the transcript, effort medium
- Task: 2-task-list.md; budget 90 min; session outcome ended (exit 0)
- Kit: commit 32dd9c4f667b, version 0.3.88, rustc 1.99.1 (58cf8f9c3 2026-10-13), third-party cache none
- Brief to end of session: 3.1 min
- Brief to accepted completion: 3.1 min
- Acceptance: met
- Time to first build: 1.2 min
- Builds: 3; failed 1; killed or unfinished 0; total build time 0.6 min
- First successful run or test (candidate; the author checks it exercised the app): `cargo test --offline`, 1 tests passed, build 2, 2.1 min after the brief

## Errors by code (occurrences across builds)

- `E0271`: 1

## Warnings by code or lint (occurrences across builds)

- `dead_code`: 1

## pert_ui paths used (candidate inventory, verify against the source)

`AppError`, `prelude::*`, `testing::*`

## Code size and API surface (definitions in summarize.py, frozen for round 2)

- Sources: `work`
- Code lines: application 223, test 146, total 369
- Distinct pert-ui types (candidates): 26 (application 25, test 5)
- Import routes: prelude 1, root 1, module 1 (`testing` 1); glob imports: `pert_ui::prelude::*`, `pert_ui::testing::*`

`App`, `Button`, `Checkbox`, `CollectionFilter`, `Column`, `CommandKey`, `CommandScope`, `Driver`, `Element`, `InlineText`, `KeyChord`, `Label`, `ListDrop`, `ListFilter`, `ListItems`, `ListSelection`, `ListSource`, `Modifiers`, `ReadSignal`, `Row`, `Runtime`, `SelectionMode`, `Signal`, `TextField`, `VirtualList`, `WidgetId`

## Proxy and model-host mentions in the transcript (read each)
