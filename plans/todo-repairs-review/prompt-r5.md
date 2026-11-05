You are an independent reviewer of a repair specification in the pert-ui
repository. Read AGENTS.md first. Read-only: do NOT edit files; you cannot run
Cargo, so evidence is static.

You reviewed revisions 1-4 of `plans/todo-repairs.md` (reports
`plans/todo-repairs-review/codex-r1.md` .. `codex-r4.md`). Revision 5 claims
to adopt the three r4 findings (see "Revision 4 → 5" in its Review record).
HEAD is 621022a; only the spec changed.

Check only:
1. The three r4 findings (R13 isolation, R13 placement storage, R15 inspection
   counter): resolved, partly or not?
2. New problems in the revised R13 text: the per-node debug-only "validation
   measurement" written by `measure_node` except under a probe-depth guard.
   Check against `runtime/tree.rs` (measure_node, the geometry commit near
   :2231, the measurement-change drain near :2263), `runtime/measurements.rs`
   and `validation/development.rs`. Does anything else read the latest cache
   entry for validation that the spec misses? Is suppressing writes for the
   whole nested probe correct when the probe measures a descendant for the
   first time (it then has no validation measurement)? Does the probe's result
   staying in the cache cause the drain or geometry commit to see a change?
3. Whether the whole spec is now ready to implement slice by slice.

Output: verdict line first ("ready", "ready with changes" or "needs
rework"), a compact status table, new findings by severity with file:line
evidence and suggested spec change, then what you did not examine.
