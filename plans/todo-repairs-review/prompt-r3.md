You are an independent reviewer of a repair specification in the pert-ui
repository (a Rust GUI library). Read AGENTS.md first: it is the governing
instruction file. Do NOT edit any file; read-only review. You cannot run
Cargo; your evidence is static.

Context: you reviewed revisions 1 and 2 of `plans/todo-repairs.md`
(`plans/todo-repairs-review/codex-r1.md`, `codex-r2.md`, both "needs
rework"). The author has produced revision 3, claiming to adopt every r2
finding (see the "Revision 2 → 3" entry of its "Review record"). HEAD is
still 621022a; only the spec changed. The maintainer has accepted D6 (TEXT-01
is a design plan only).

Recheck:
1. Each r2 finding (the blocker R18, should-fix R13, R15, R14, R12, the
   verification/D8 point, and the R14 nit): resolved, partly resolved (what is
   missing) or not resolved, in the specification.
2. New problems introduced by revision 3. Look hardest at:
   - R18 `RowItem` wrapping a complete `Element` built with an internal
     placeholder key: does `Element::new` accept that (key validation at
     construction vs mount)? Does a public `RowItem::into_element(key)`
     create any identity or privileged-path problem? Interaction with
     `EachItem`.
   - R13 `ArrangeContext::place_fixed_slot`: is measuring through
     `ArrangeContext::measure` during arrangement allowed by the layout
     contract (docs/contracts.md, docs/internals.md), and is "development
     validation enabled" a real, checkable condition in the code? Retention
     and clearing rules.
   - R15 `ItemScan` snapshot: consistent with the existing cursor, ordering
     with RowScan and children, budget accounting, `done`.
   - R14 slice 4a design checkpoint: acceptable boundary?
3. Whether the spec is now ready to implement slice by slice.

Output: verdict line first ("ready", "ready with changes" or "needs
rework"). A compact status table for the r2 findings. New findings by
severity (blocker / should-fix / nit) with file:line evidence, problem,
failure scenario where relevant and suggested spec change. Finish with what
you did not examine.
