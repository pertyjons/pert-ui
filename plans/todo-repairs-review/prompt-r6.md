You are an independent reviewer of a repair specification in the pert-ui
repository. Read AGENTS.md first. Read-only: do NOT edit files; you cannot run
Cargo, so evidence is static.

You reviewed revisions 1-5 of `plans/todo-repairs.md` (reports
`plans/todo-repairs-review/codex-r1.md` .. `codex-r5.md`). Revision 6 replaces
R13's isolated debug-only probe with an ordinary measurement in every build
(see R13 in slice 3 and "Revision 5 → 6" in the Review record). HEAD is
621022a; only the spec changed.

Check only R13:
1. Are the r5 findings resolved by the new approach (readers, first
   measurement, change notification)?
2. New problems: is an ordinary `ArrangeContext::measure` at (slot width,
   unbounded height) during arrangement correct for `VirtualList` and `Tree`
   rows (check virtual_list.rs ~2100-2170, tree.rs TreeRow, layout.rs
   ArrangeContext::measure/place_clipped, runtime/tree.rs measure_node and
   arrangement skipping)? Could measuring a row at unbounded height change
   layout results, dependency invalidation, or cause re-layout loops? Is the
   stated cost (one cached measurement per placed row) accurate? Is
   measuring in release builds acceptable under AGENTS.md's scheduling rules
   ("retain layout ... for unchanged widgets")? Slot record rules.
3. Whether the whole spec is now ready to implement slice by slice.

Output: verdict line first ("ready", "ready with changes" or "needs
rework"), a compact status table, findings by severity with file:line
evidence and suggested spec change, then what you did not examine.
