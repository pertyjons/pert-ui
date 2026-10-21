# Run summary: r1-1-claude

Exploratory evidence from one AI tester run. Counts describe how this tester worked; they are not a usability score.

- Tester: claude (2.1.306 (Claude Code)), requested model opus, effective model claude-opus-5-5, effort high
- Task: 1-settings-editor.md; budget 90 min; session outcome ended (exit 0)
- Kit: commit 359122e38ec7, version 0.3.52, rustc 1.99.1 (58cf8f9c3 2026-10-13), third-party cache none
- Brief to end of session: 5.5 min
- Brief to accepted completion: n/a (acceptance partly)
- Acceptance: partly
- Time to first build: 0.3 min
- Builds: 8; failed 1; killed or unfinished 0; total build time 0.5 min
- First successful run or test (candidate; the author checks it exercised the app): `cargo test`, 6 tests passed, build 4, 4.3 min after the brief

## Errors by code (occurrences across builds)


## Warnings by code or lint (occurrences across builds)

- `unused_must_use`: 1

## pert_ui paths used (candidate inventory, verify against the source)

`App`, `AppError`, `CaptureRequest`, `CaptureTarget`, `Field`, `FieldHandle`, `FieldStatus`, `Form`, `FormStatus`, `Group`, `InputRejection`, `IntegerInput`, `KeyAction`, `KeyCode`, `NativeCloseAction`, `NativeCloseAction::KeepOpen`, `NativeWindowId`, `PresentationAction`, `PresentationStatus`, `Runtime`, `SegmentedControl`, `SemanticValue`, `Semantics`, `TextCommand`, `TextField`, `Toggle`, `Viewport`, `WidgetId`, `cpu::CpuRenderer`, `prelude::*`

## Proxy and model-host mentions in the transcript (read each)

- `127.0.0.1:3128`: 1
- `proxy.sock`: 1
