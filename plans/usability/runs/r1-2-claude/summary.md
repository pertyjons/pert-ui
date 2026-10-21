# Run summary: r1-2-claude

Exploratory evidence from one AI tester run. Counts describe how this tester worked; they are not a usability score.

- Tester: claude (2.1.306 (Claude Code)), requested model opus, effective model claude-opus-5-5, effort high
- Task: 2-task-list.md; budget 90 min; session outcome ended (exit 0)
- Kit: commit 359122e38ec7, version 0.3.52, rustc 1.99.1 (58cf8f9c3 2026-10-13), third-party cache none
- Brief to end of session: 7.6 min
- Brief to accepted completion: n/a (acceptance partly)
- Acceptance: partly
- Time to first build: 2.6 min
- Builds: 11; failed 3; killed or unfinished 0; total build time 0.7 min
- First successful run or test (candidate; the author checks it exercised the app): `cargo test`, 4 tests passed, build 6, 5.5 min after the brief

## Errors by code (occurrences across builds)

- `E0422`: 1
- `E0433`: 1
- `E0308`: 1

## Warnings by code or lint (occurrences across builds)

- `unused_must_use`: 5
- `dead_code`: 1

## pert_ui paths used (candidate inventory, verify against the source)

`CaptureRequest`, `CaptureTarget`, `CommandKey`, `CommandKeyInput`, `CommandScope`, `DropPosition`, `FilterMatcher`, `KeyChord`, `ListDrop`, `Modifiers`, `Point`, `PointerEvent`, `PointerKind`, `cpu::CpuRenderer`, `prelude::*`

## Proxy and model-host mentions in the transcript (read each)
