# Run summary: r3-4-head-codex

Exploratory evidence from one AI tester run. Counts describe how this tester worked; they are not a usability score.

- Tester: codex (codex-cli 0.165.0), requested model gpt-6.1-sol, effective model not in the transcript, effort medium
- Task: 4-testing-an-app.md; budget 90 min; session outcome ended (exit 0)
- Kit: commit 32dd9c4f667b, version 0.3.88, rustc 1.99.1 (58cf8f9c3 2026-10-13), third-party cache none
- Brief to end of session: 2.4 min
- Brief to accepted completion: n/a (acceptance not recorded)
- Acceptance: not recorded
- Time to first build: 1.0 min
- Builds: 3; failed 1; killed or unfinished 0; total build time 0.4 min
- First successful run or test (candidate; the author checks it exercised the app): `cargo test --manifest-path /work/bookmarks/Cargo.toml --offline`, 8 tests passed, build 1, 1.4 min after the brief

## Errors by code (occurrences across builds)


## Warnings by code or lint (occurrences across builds)


## pert_ui paths used (candidate inventory, verify against the source)

`AppError`, `prelude::*`, `testing::*`

## Code size and API surface (definitions in summarize.py, frozen for round 2)

- Sources: `work`, task 4 counted against the fixture at the kit's commit
- Code lines added or changed: application 0, test 216, total 216
- Distinct pert-ui types (candidates): 21 (application 16, test 9)
- Import routes (paths the fixture lacks): prelude 0, root 0, module 1 (`testing` 1); glob imports: `pert_ui::testing::*`

`App`, `Button`, `CaptureRequest`, `Column`, `CommandScope`, `CpuRenderer`, `Driver`, `Each`, `Field`, `Form`, `KeyChord`, `Label`, `Path`, `ReadSignal`, `Row`, `Runtime`, `ScrollArea`, `ScrollAxes`, `TextField`, `Viewport`, `WidgetId`

## Proxy and model-host mentions in the transcript (read each)
