# Author review: settings and the remaining Claude head tasks

2026-10-28, Codex. Reports, verifier tests, final outputs and original/copy
hashes reviewed. No original submission was repaired. These headless AI checks
make no native execution or human-usability claim.

| Submission | Disposition | Evidence reviewed |
| --- | --- | --- |
| r3-1-head-claude | met | All fields/refusals through input and inspection, valid-change plus later refused draft then Revert, disk and failure preservation, FIFO blocking proof of worker save, real headless close admission in dirty/resolved states. Final full suite passes. Captures reviewed. |
| r3-1-head-codex | met | Independent source read confirms all five criteria; inspection/disk/close checks replace tester model assertions. FIFO has no reader until after the click returns. Later refused draft is discarded by Revert after a valid change. Final suite passes. Only src/main.rs gains an appended cfg(test) module entry point; the diff was reviewed line by line. No production behavior changes. |
| r3-3-head-claude-retry1 | met | Clamped out-of-area drags, cancellation, every arrow and Shift direction, both numeric axes, Reset, two independently mounted controls, theme/focus pixel checks and value snapshot. Final tester and verifier suites pass; dark/light CPU captures viewed. |
| r3-4-head-claude-retry1 | met | Frozen fixture production source is unchanged except the harness manifest version. All listed behavior tests and CPU capture pass; recorded mutations fail the intended tests. The verifier's stale Cargo binary after restoring an old mtime is disclosed: a forced package rebuild gives the final passing suite on byte-restored source. Filtered Remove and shortcut from button focus add independent coverage. Capture viewed. |
| r3-5-head-claude-retry1 | met | Public input and inspection establish progress, status, every sort column/direction, outside-row failure notification and quiet intervals. The verifier observes named Linux OS threads before/after cancellation, including wall-clock app execution. Final suite and repeated acceptance tests pass. CPU capture viewed. |

All terminal verifier statuses are zero and original_source_unchanged=True.
Author recomputation confirms the original manifests, unchanged Cargo.toml and
all pre-existing Rust files in the copies, except the explicitly reviewed
cfg(test) entry point above. The archive retains its full changed file and diff
alongside added tests, rather than pretending that the copy is byte-identical.
Controller adoption also checks original run-log hashes.

Report citation corrections: the Claude task-3 report's numeric ranges for
src/headless.rs and src/picker.rs overrun the submitted files. The named
capture_png and PointPicker methods and executed tests exist in those files;
use those item names and the archived source, not the inaccurate line ranges.
Likewise, the already-adopted task-2 report's build range is offset by three
lines. Its repeated ListDropPosition borrowing claim is contradicted: that enum
owns its key and is Clone; only position() borrows the enum. These citation and
interpretation errors do not invalidate the executed acceptance checks. The
original reports are retained intact with these author corrections.

Acceptance observations still enter recurrence/new-finding triage. Overlay
scrollbars and toast placement are documented policies; disabled-button
presentation needs a separate decision. Acceptance is not round closure.
