You are an independent code reviewer for the pert-ui repository (a Rust GUI
library). Read AGENTS.md first: it is the governing instruction file. Do NOT
edit any file; read-only review. You cannot run Cargo, so your evidence is
static; say where a claim needs a run.

Scope: commits 7cf441c and 86b50d7 on branch todo-repairs (slice 4b of
plans/todo-repairs.md): MultiSelect and QuickPick popup items get rectangles
through option C of plans/todo-repairs-r14-popups.md (revision 2 and its
section 9 deviations), reviewed earlier as design-s4a.md and
design-s4a-r2.md. Also read the 0.3.101 CHANGELOG entry. Note that
QuickPick::searchable (b0133d2, default true) changed the search behavior
the implementation was written against; the merge adapted one test.

Look hardest at:
- VirtualList::report_items and ListItems::semantic_item: keyed placement
  records and their validity across source adoption, rejected child
  requests (the crate-internal adoption accessors on UpdateContext /
  ArrangeContext — a privileged path? the TODO entry proposing exposure),
  failed builds, reorder/insert/remove, retained arrangements, view
  transforms; the leftover-child InvalidChild fix; inspection cost and "no
  work scheduled"; rustdoc for the new public items and the ListItems doc
  exceptions.
- MultiSelect: retained highlight/permission/draft in the panel update,
  checked vs selected semantics, disabled options, the new control/options
  parts, trigger items without bounds.
- QuickPick: ListBox moved to the result list, identity changes, the empty
  state, retained highlight, disabled-result clicks, CommandPalette
  forwarding, interaction with QuickPick::searchable.
- Content search overlap policy and its tests; migrated tests using parts.
- api/ listing, TODO.md, docs/contracts.md, CHANGELOG accuracy.

Output: verdict line first ("ready", "ready with fixes" or "needs fixes"),
findings by severity (blocker / should-fix / nit) with file:line, the
problem, a failure scenario and the fix, then what you did not examine.
