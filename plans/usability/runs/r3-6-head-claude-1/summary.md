# Run summary: r3-6-head-claude-1

Exploratory evidence from one AI tester run. Counts describe how this tester worked; they are not a usability score.

- Tester: claude (2.1.312 (Claude Code)), requested model claude-opus-5-5, effective model claude-opus-5-5, effort high
- Task: 6-notes-editor.md; budget 90 min; session outcome ended (exit 0)
- Kit: commit 32dd9c4f667b, version 0.3.88, rustc 1.99.1 (58cf8f9c3 2026-10-13), third-party cache none
- Brief to end of session: 8.7 min
- Brief to accepted completion: 8.7 min
- Acceptance: met
- Time to first build: 3.6 min
- Builds: 24; failed 14; killed or unfinished 0; total build time 0.9 min
- First successful run or test (candidate; the author checks it exercised the app): `cargo test --test probe -- --nocapture`, 1 tests passed, build 4, 4.7 min after the brief

## Errors by code (occurrences across builds)

- `E0308`: 2
- `E0433`: 2
- `E0277`: 1
- `casting `&T` to `&mut T` is undefined behavior, even if the `: 1
- `E0502`: 1
- `E0618`: 1

## Warnings by code or lint (occurrences across builds)

- `unused_imports`: 8
- `dead_code`: 1
- `function `read` is never used`: 1
- `function `menu` is never used`: 1

## pert_ui paths used (candidate inventory, verify against the source)

`AppError`, `CommandContext`, `DialogController`, `DialogStatus`, `ListSelection`, `SelectionMode`, `Toast`, `ToastController`, `prelude::*`, `testing::*`, `windowing::SplitAxis`, `windowing::SplitPane`, `windowing::SplitSizing`

## Code size and API surface (definitions in summarize.py, frozen for round 2)

- Sources: `work`
- Code lines: application 404, test 337, total 741
- Distinct pert-ui types (candidates): 43 (application 37, test 10)
- Import routes: prelude 1, root 8, module 4 (`testing` 1, `windowing` 3); glob imports: `pert_ui::prelude::*`, `pert_ui::testing::*`

`Align`, `App`, `Button`, `CaptureRequest`, `Choice`, `ClickCounter`, `Column`, `CommandContext`, `CommandScope`, `CpuRenderer`, `Dialog`, `DialogController`, `DialogStatus`, `Driver`, `Dynamic`, `Element`, `FrameImage`, `KeyChord`, `Label`, `Length`, `ListItems`, `ListSelection`, `ListSource`, `MenuBar`, `MenuItem`, `Path`, `ReadSignal`, `Row`, `Runtime`, `SelectionMode`, `Signal`, `SplitAxis`, `SplitPane`, `SplitSizing`, `Tab`, `Tabs`, `TextArea`, `TextField`, `Toast`, `ToastController`, `ToastHost`, `VirtualList`, `WidgetId`

## Proxy and model-host mentions in the transcript (read each)
