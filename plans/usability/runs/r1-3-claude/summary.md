# Run summary: r1-3-claude

Exploratory evidence from one AI tester run. Counts describe how this tester worked; they are not a usability score.

- Tester: claude (2.1.306 (Claude Code)), requested model opus, effective model claude-opus-5-5, effort high
- Task: 3-custom-control.md; budget 90 min; session outcome ended (exit 0)
- Kit: commit 359122e38ec7, version 0.3.52, rustc 1.99.1 (58cf8f9c3 2026-10-13), third-party cache none
- Brief to end of session: 8.0 min
- Brief to accepted completion: 8.0 min
- Acceptance: met
- Time to first build: 1.8 min
- Builds: 21; failed 8; killed or unfinished 0; total build time 1.1 min
- First successful run or test (candidate; the author checks it exercised the app): `cargo test --test headless escape -- --nocapture`, 1 tests passed, build 5, 4.6 min after the brief

## Errors by code (occurrences across builds)

- `E0609`: 3
- `E0277`: 2
- `E0308`: 2
- `E0614`: 1

## Warnings by code or lint (occurrences across builds)


## pert_ui paths used (candidate inventory, verify against the source)

`App`, `Button`, `CaptureRequest`, `CaptureTarget`, `ColorRole`, `Column`, `CommandKey`, `CommandKeyInput`, `Component`, `ComponentDetails`, `Constraints`, `Container`, `DecimalInput`, `DrawError`, `Element`, `EventContext`, `Field`, `InputOutcome`, `InspectValue`, `IntoElement`, `Invalidation`, `KeyAction`, `KeyCode`, `KeyEvent`, `KeyTransition`, `KeyboardState`, `Label`, `Length`, `MeasureContext`, `MetricRole`, `Notifier`, `Painter`, `Point`, `PointerButton`, `PointerEvent`, `PointerKind`, `Rect`, `Row`, `Runtime`, `RuntimeError`, `SemanticAction`, `SemanticRole`, `SemanticValue`, `Semantics`, `Shape`, `Signal`, `Size`, `StrokeStyle::new`, `ThemeMode`, `UpdateContext`, `Viewport`, `WidgetId`, `check_redescription`, `cpu::CpuRenderer`

## Proxy and model-host mentions in the transcript (read each)
