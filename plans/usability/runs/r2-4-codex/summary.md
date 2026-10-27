# Run summary: r2-4-codex

Exploratory evidence from one AI tester run. Counts describe how this tester worked; they are not a usability score.

- Tester: codex (codex-cli 0.164.0), requested model gpt-6.1-sol, effective model not in the transcript, effort medium
- Task: 4-testing-an-app.md; budget 90 min; session outcome ended (exit 0)
- Kit: commit aca8df366fde, version 0.3.75, rustc 1.99.1 (58cf8f9c3 2026-10-13), third-party cache none
- Brief to end of session: 3.1 min
- Brief to accepted completion: 3.1 min
- Acceptance: met
- Time to first build: 0.5 min
- Builds: 7; failed 4; killed or unfinished 0; total build time 0.5 min
- First successful run or test (candidate; the author checks it exercised the app): `cargo test --offline`, 0 tests passed, build 1, 0.9 min after the brief

## Errors by code (occurrences across builds)

- `E0433`: 1
- `E0277`: 1

## Warnings by code or lint (occurrences across builds)


## pert_ui paths used (candidate inventory, verify against the source)

`AppError`, `SemanticRole`, `prelude::*`, `testing::*`

## Code size and API surface (definitions in summarize.py, frozen for round 2)

- Sources: `work`, task 4 counted against the fixture at the kit's commit
- Code lines added or changed: application 0, test 230, total 230
- Distinct pert-ui types (candidates): 20 (application 15, test 11)
- Import routes (paths the fixture lacks): prelude 0, root 1, module 1 (`testing` 1); glob imports: `pert_ui::testing::*`

`App`, `Button`, `CaptureRequest`, `ClickCounter`, `Column`, `CommandScope`, `CpuRenderer`, `Driver`, `Each`, `Field`, `Form`, `KeyChord`, `Label`, `Row`, `Runtime`, `ScrollArea`, `ScrollAxes`, `TextField`, `Viewport`, `WidgetId`

## Proxy and model-host mentions in the transcript (read each)
