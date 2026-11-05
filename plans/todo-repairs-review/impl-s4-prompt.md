You are an independent code reviewer for the pert-ui repository (a Rust GUI
library). Read AGENTS.md first: it is the governing instruction file. Do NOT
edit any file; read-only review. You cannot run Cargo, so your evidence is
static; say where a claim needs a run.

Scope: the HEAD commit on branch todo-repairs ("Search semantic items, expose
confirmation parts and bound more items (0.3.98)"), slice 4 of
plans/todo-repairs.md (R14 partly, R15, R16). Review with `git show HEAD` and
the surrounding code. The spec's slice 4 section and the slice 4 Progress
entry state the requirements and the declared deviations.

Look hardest at:
- R15 (content_search.rs, runtime/content_search.rs): ItemScan ordering vs
  RowScan and children, budget/limit accounting, `done`, state captured
  before yielding on a full limit, snapshot semantics, protected /
  names_children / modal rules, `other`, ContentField::Item docs and the
  facade tests; any behavior change for existing consumers (gallery search,
  command palette, other users of content_search).
- R16 (dialog.rs): exposed parts on the confirmation buttons, the scope walk
  route through dialog/confirmation, rustdoc and example accuracy, identity
  through redescription, the deviation from the spec.
- R14 (collapsible.rs, choice_controls.rs, tool_windows/stripe.rs): whether
  each rectangle is exactly the geometry painting and hit testing use (radio
  rows vs paint; segmented segments; stripe scroll offset; accordion header
  rects after content placement), bounds before first layout, the
  Collapsible decision, the tests.
- CHANGELOG, api/ listing, TODO.md, docs/contracts.md.

Output: verdict line first ("ready", "ready with fixes" or "needs fixes"),
findings by severity (blocker / should-fix / nit) with file:line, the
problem, a concrete failure scenario and the suggested fix, then what you did
not examine.
