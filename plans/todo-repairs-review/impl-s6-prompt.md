You are an independent code reviewer for the pert-ui repository (a Rust GUI
library). Read AGENTS.md first. Read-only: do NOT edit files; you cannot run
Cargo, so your evidence is static.

Scope: commits f92fca3 and 74d8f85 on branch todo-repairs, slice 6 of
plans/todo-repairs.md (R17, R20, R21, R23). Review with `git show f92fca3
74d8f85`; the spec's slice 6 section and Progress entry state the
requirements and declared deviations.

Look hardest at:
- R21 (capture.rs png module): filter implementations (Sub/Up/Average/Paeth
  correctness, first-row prior, bpp handling), the min-sum selection and its
  determinism, compress_to_vec_zlib usage, header filter-method byte, width 0
  or empty images, memory, the tests' independence (png crate decoder), the
  rustdoc's determinism scope, the capture_smoke change; the new workspace
  dependencies (versions, features, MSRV, dev-dependency placement,
  packaging implications).
- R20/R17 (controls.rs control_frame/Painted, select.rs, multi_select.rs):
  equivalence of Button's previous paint (focus ring, frameless/hover,
  disabled outline) with the refactor; anchors' disabled treatment and
  hover; Painted replacement semantics, absence before paint, inline Select
  reporting, reconcile keeping paint metadata; "inspection computes nothing";
  docs (rendering-platform.md, Button rustdoc); tests reading each anchor's
  own inspection.
- R23 lockfile update; TODO.md and CHANGELOG accuracy.

Output: verdict line first ("ready", "ready with fixes" or "needs fixes"),
findings by severity with file:line, the problem, a failure scenario and the
fix, then what you did not examine.
