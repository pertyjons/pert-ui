# Run summary: r3-4-head-claude-retry1

Exploratory evidence from one AI tester run. Counts describe how this tester worked; they are not a usability score.

- Tester: claude (2.1.312 (Claude Code)), requested model claude-opus-5-5, effective model claude-opus-5-5, effort high
- Task: 4-testing-an-app.md; budget 90 min; session outcome ended (exit 0)
- Kit: commit 32dd9c4f667b, version 0.3.88, rustc 1.99.1 (58cf8f9c3 2026-10-13), third-party cache none
- Brief to end of session: 2.7 min
- Brief to accepted completion: 2.7 min
- Acceptance: met
- Time to first build: 0.5 min
- Builds: 6; failed 2; killed or unfinished 0; total build time 0.5 min
- First successful run or test (candidate; the author checks it exercised the app): `cargo test --test behaviors`, 7 tests passed, build 2, 1.6 min after the brief

## Errors by code (occurrences across builds)


## Warnings by code or lint (occurrences across builds)


## pert_ui paths used (candidate inventory, verify against the source)

`AppError`, `prelude::*`, `testing::*`

## Code size and API surface (definitions in summarize.py, frozen for round 2)

- Sources: `work`, task 4 counted against the fixture at the kit's commit
- Code lines added or changed: application 0, test 280, total 280
- Distinct pert-ui types (candidates): 22 (application 16, test 9)
- Import routes (paths the fixture lacks): prelude 0, root 0, module 1 (`testing` 1); glob imports: `pert_ui::testing::*`

`App`, `Button`, `CaptureRequest`, `Column`, `CommandScope`, `CpuRenderer`, `Driver`, `Each`, `Field`, `Form`, `FrameImage`, `KeyChord`, `Label`, `Path`, `ReadSignal`, `Row`, `Runtime`, `ScrollArea`, `ScrollAxes`, `TextField`, `Viewport`, `WidgetId`

## Proxy and model-host mentions in the transcript (read each)
