# Author review: remaining Codex head tasks 2–4

2026-10-28, Codex. Independent reports, added verifier tests and final passing
outputs reviewed with source-integrity checks and the required captures.
Headless AI evidence only; no native execution or human-usability claim.

| Submission | Disposition | Basis |
| --- | --- | --- |
| r3-2-head-codex | met | Adding/checking/unchecking/removing/reordering update both inspected rows and status. Direct vector updates are confined to the separate source-of-truth check. Rename Enter/Escape, retained/cleared filter selection and unchanged vector, and shortcuts are driven through public input. Alt+arrows work with list focus; the task does not demand application-wide interception. Final suite passes. src/main.rs only gains the appended cfg(test) module hook, reviewed line by line and retained separately with its diff. |
| r3-3-head-codex | met | Actual drag/clamp, Escape and focus-loss restore, all arrows/Shift, numeric editing and Reset with handle pixels, independently mounted control instances, active-theme/focus pixel checks and serialized inspection. Final tester/verifier suites pass, including focus-loss cancellation not reached by the original tester. Light focused capture viewed. Existing Rust files/Cargo.toml unchanged. |
| r3-4-head-codex | met | Original production source matches the frozen fixture, with only harness dependency-version substitution. Dedicated tests cover every listed behavior. Independent Tab-only focus checks establish Ctrl+L from every intended control, with no unintended button action. CPU capture viewed. The trim mutation fails the intended test, restored source hashes match, final fresh full suite passes. Existing Rust files/Cargo.toml unchanged. |

Author recomputed all three original source manifests. Every existing Rust file
and manifest in each copy matches the original, except the explicitly reviewed
task-2 hook. Adoption additionally verifies original run-log hashes. No submitted
application was repaired. Test-only assumptions, compile failures and subsequent
corrections remain in each verifier's report/evidence.

Initial group-H triage was started before these verifiers and therefore calls
Codex acceptance pending. These final records supersede that status, not the
blind source evidence. In the final matrices the independent focus-loss and
close-draft checks must be labelled acceptance exercise, rather than attributed
to the timed blind tester. No silent promotion from compilation to execution.
