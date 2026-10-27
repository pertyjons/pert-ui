# Run summary: r2-4-claude

Exploratory evidence from one AI tester run. Counts describe how this tester worked; they are not a usability score.

- Tester: claude (2.1.311 (Claude Code)), requested model opus, effective model claude-opus-5-5, effort high
- Task: 4-testing-an-app.md; budget 90 min; session outcome ended (exit 0)
- Kit: commit aca8df366fde, version 0.3.75, rustc 1.99.1 (58cf8f9c3 2026-10-13), third-party cache none
- Brief to end of session: 3.8 min
- Brief to accepted completion: 3.8 min
- Acceptance: met
- Time to first build: 1.6 min
- Builds: 10; failed 7; killed or unfinished 0; total build time 0.8 min
- First successful run or test (candidate; the author checks it exercised the app): `cargo test`, 7 tests passed, build 1, 2.0 min after the brief

## Errors by code (occurrences across builds)


## Warnings by code or lint (occurrences across builds)

- `unused_variables`: 1

## pert_ui paths used (candidate inventory, verify against the source)

`AppError`, `prelude::*`, `testing::*`

## Code size and API surface (definitions in summarize.py, frozen for round 2)

- Sources: `work`, task 4 counted against the fixture at the kit's commit
- Code lines added or changed: application 0, test 257, total 257
- Distinct pert-ui types (candidates): 21 (application 15, test 9)
- Import routes (paths the fixture lacks): prelude 0, root 0, module 1 (`testing` 1); glob imports: `pert_ui::testing::*`

`App`, `Button`, `CaptureRequest`, `ClickCounter`, `Column`, `CommandScope`, `CpuRenderer`, `Driver`, `Each`, `Field`, `Form`, `KeyChord`, `Label`, `Path`, `Row`, `Runtime`, `ScrollArea`, `ScrollAxes`, `TextField`, `Viewport`, `WidgetId`

## Proxy and model-host mentions in the transcript (read each)
