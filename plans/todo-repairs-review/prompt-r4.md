You are an independent reviewer of a repair specification in the pert-ui
repository (a Rust GUI library). Read AGENTS.md first. Do NOT edit any file;
read-only review; you cannot run Cargo, so your evidence is static.

You reviewed revisions 1-3 of `plans/todo-repairs.md`
(`plans/todo-repairs-review/codex-r1.md` .. `codex-r3.md`). Revision 4 claims
to adopt every r3 finding (see "Revision 3 → 4" in its Review record). HEAD
is still 621022a; only the spec changed.

Focus only on:
1. The four r3 should-fix findings (R13 isolation, R13 retention, R15
   ordering, R15 snapshot cost) and the two nits: resolved, partly or not?
2. New problems in the revised R13 and R15 text. In particular: is "the
   measurement at the constraints the child was placed with" actually
   obtainable from the retained measurement cache (check
   `runtime/measurements.rs`, `runtime/tree.rs` measure_node and the
   development fixed-height check in `validation/development.rs`)? Is a
   parent-owned placement record a real structure today, or would the slice
   have to add it? Does keeping the order items -> RowScan -> children match
   the current visit order, and does the "state captured before yielding"
   claim hold for how `content_hit` and `RowScan` are created today?
3. Whether the whole spec is now ready to implement slice by slice.

Output: verdict line first ("ready", "ready with changes" or "needs
rework"), a compact status table, then any new findings by severity with
file:line evidence and a suggested spec change, then what you did not
examine.
