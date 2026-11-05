You are an independent code reviewer for the pert-ui repository (a Rust GUI
library). Read AGENTS.md first: it is the governing instruction file.
Read-only: do NOT edit files. You cannot run Cargo, so your evidence is
static; say where a claim needs a run.

Recheck commits 587d99f and 8f763d8 on branch todo-repairs against your
previous report plans/todo-repairs-review/impl-final.md. They add:
- the picker row hit's target check and a test that a content search held
  across a QuickPick query or session change is stale
  (crates/pert-ui/tests/collections/popup_items.rs);
- a nested-collection order test with explicit targets
  (crates/pert-ui/tests/collections/content_search.rs, ItemOwner);
- disabled-anchor tests for icon captions, the selected chip and overridden
  Outline/Muted roles (crates/pert-ui/tests/presentation/disabled_button.rs);
- a failed-paint test for both dropdown anchors
  (crates/pert-ui-runtime/src/select.rs, paint_failure_tests) using a
  #[cfg(test)] stroke fault in crates/pert-ui-runtime/src/shapes.rs
  (faults module). Judge whether that test-only hook is acceptable under
  AGENTS.md (global mutable state, privileged paths, production code
  impact) and whether the test would catch the regression it targets.
- the Progress entry in plans/todo-repairs.md and the 0.3.101 CHANGELOG
  entry, and the trailing-whitespace fix in the review records.

For each finding of impl-final.md: resolved, partly or not? Then any new
problems (file:line), including weak or vacuous assertions, heuristics in
the test helpers (for example how icons are told from text by square
masks) that could pass for the wrong reason, and documentation accuracy.
Finally say whether the branch's repairs are ready to merge from a
code-review standpoint (native checks are out of scope).

Output: verdict line first ("ready", "ready with nits" or "needs fixes"),
a compact status table, findings by severity with file:line, problem and
fix, then what you did not examine.
