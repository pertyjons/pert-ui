# Run summary: r1-2-codex

Exploratory evidence from one AI tester run. Counts describe how this tester worked; they are not a usability score.

- Tester: codex (codex-cli 0.163.0), requested model gpt-6.1-sol, effective model not in the transcript, effort medium
- Task: 2-task-list.md; budget 90 min; session outcome ended (exit 0)
- Kit: commit 359122e38ec7, version 0.3.52, rustc 1.99.1 (58cf8f9c3 2026-10-13), third-party cache none
- Brief to end of session: 6.9 min
- Brief to accepted completion: 6.9 min
- Acceptance: met
- Time to first build: 1.0 min
- Builds: 10; failed 4; killed or unfinished 0; total build time 0.7 min
- First successful run or test (candidate; the author checks it exercised the app): `cargo test --offline -- --nocapture`, 1 tests passed, build 7, 5.1 min after the brief

## Errors by code (occurrences across builds)

- `E0599`: 3
- `E0308`: 2
- `E0616`: 1
- `E0277`: 1

## Warnings by code or lint (occurrences across builds)

- `unused_must_use`: 6
- `unused_imports`: 2
- `dead_code`: 1

## pert_ui paths used (candidate inventory, verify against the source)

`CollectionFilter`, `CommandKey`, `CommandKeyInput`, `CommandScope`, `DraftEditor`, `DraftEditorController`, `DraftSelection`, `KeyChord`, `ListFilter`, `ListItems`, `ListSelection`, `ListSource`, `Modifiers`, `SelectionMode`, `VirtualList`, `prelude::*`

## Proxy and model-host mentions in the transcript (read each)
