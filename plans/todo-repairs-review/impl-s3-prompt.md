You are an independent code reviewer for the pert-ui repository (a Rust GUI
library). Read AGENTS.md first: it is the governing instruction file. Do NOT
edit any file; read-only review. You cannot run Cargo, so your evidence is
static; say where a claim needs a run.

Scope: commit 7910e87 on branch todo-repairs ("Summarize overlap reports and
report rows taller than fixed slots (0.3.97)"), slice 3 of
plans/todo-repairs.md (R12, R13). Review with `git show 7910e87` and the
surrounding code. The spec's slice 3 section (revision 7) states the
requirements; note any deviation and whether it is justified.

Look hardest at:
- R12 `overlap_issues` in crates/pert-ui-runtime/src/validation/development.rs:
  sweep correctness (break condition vs OVERLAP_TOLERANCE, both axes, sort
  stability/determinism), threshold boundary (8 listed, 9 summarized), the
  `listed` buffer, summary `components` meaning and markers, IssueKey/log
  suppression when the summary message's counts change, issue ordering
  changes vs before, the tests.
- R13 `ArrangeContext::place_fixed_slot` (layout.rs), the Placement tuple
  change and commit path (runtime/tree.rs), Node::slot_height and the
  validator's slot field/check (runtime.rs, validation/development.rs):
  ordering of the child's own geometry commit vs the parent's placement
  commit, retained arrangements, failed layouts, unmount, units/scale,
  update-time NodeData reconstruction, measurement in every build and any
  effect on layout results, cache, dependency tracking or scheduling
  (AGENTS.md retention rules), rustdoc accuracy, the facade tests in
  crates/pert-ui/tests/foundation/fixed_slots.rs.
- The FileBrowser label overflow change (file_browser/view.rs): behavior
  change, accessibility/semantics of the full name, docs.
- Gallery row-height changes, CHANGELOG accuracy, api/ listing, TODO.md.

Output: verdict line first ("ready", "ready with fixes" or "needs fixes"),
findings by severity (blocker / should-fix / nit) with file:line, the
problem, a concrete failure scenario and the suggested fix, then what you did
not examine.
