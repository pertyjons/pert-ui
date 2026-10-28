# Run summary: r3-1-head-codex

Exploratory evidence from one AI tester run. Counts describe how this tester worked; they are not a usability score.

- Tester: codex (codex-cli 0.165.0), requested model gpt-6.1-sol, effective model not in the transcript, effort medium
- Task: 1-settings-editor.md; budget 90 min; session outcome ended (exit 0)
- Kit: commit 32dd9c4f667b, version 0.3.88, rustc 1.99.1 (58cf8f9c3 2026-10-13), third-party cache none
- Brief to end of session: 4.1 min
- Brief to accepted completion: 4.1 min
- Acceptance: met
- Time to first build: 1.5 min
- Builds: 5; failed 2; killed or unfinished 0; total build time 1.8 min
- First successful run or test (candidate; the author checks it exercised the app): `cargo test --offline`, 2 tests passed, build 3, 3.3 min after the brief

## Errors by code (occurrences across builds)

- `E0283`: 2
- `E0308`: 1
- `E0277`: 1
- `E0599`: 1

## Warnings by code or lint (occurrences across builds)

- `dead_code`: 1

## pert_ui paths used (candidate inventory, verify against the source)

`AppError`, `prelude::*`, `testing::*`

## Code size and API surface (definitions in summarize.py, frozen for round 2)

- Sources: `work`
- Code lines: application 286, test 196, total 482
- Distinct pert-ui types (candidates): 20 (application 19, test 4)
- Import routes: prelude 1, root 1, module 1 (`testing` 1); glob imports: `pert_ui::prelude::*`, `pert_ui::testing::*`

`App`, `Button`, `Choice`, `Column`, `Driver`, `Field`, `FormFields`, `InputRejection`, `IntegerInput`, `Label`, `NativeCloseAction`, `ReadSignal`, `Row`, `Runtime`, `SegmentedControl`, `Signal`, `Size`, `TextField`, `Toggle`, `WidgetId`

## Proxy and model-host mentions in the transcript (read each)
