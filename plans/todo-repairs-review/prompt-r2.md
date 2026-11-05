You are an independent reviewer of a repair specification in the pert-ui
repository (a Rust GUI library). Read AGENTS.md first: it is the governing
instruction file. Do NOT edit any file; this is a read-only review. You cannot
run Cargo; your evidence is static, so say so where a claim needs a run.

Context: you reviewed revision 1 of `plans/todo-repairs.md`; your report is
`plans/todo-repairs-review/codex-r1.md` (verdict "needs rework"). The author
has rewritten the spec as revision 2, claiming to adopt every finding (see its
"Review record" section). HEAD is still 621022a; only the spec changed.

Recheck:

1. For each finding in codex-r1.md: is it actually resolved in revision 2?
   Say "resolved", "partly resolved" (what is missing) or "not resolved".
2. New problems introduced by revision 2. Look hardest at:
   - R18 `RowItem`: is a concrete type-erased row value implementable with the
     current Element/Component machinery (check `layout.rs` Element,
     `described.rs` EachItem conversion, how `VirtualList`/`Tree` assign
     keys)? Metadata preservation, object safety, privileged-path rule.
   - R13 slot-height hint: does a public validation-hint mechanism exist that
     can carry a slot height, or is the spec inventing one? Is measuring at
     slot width in development validation consistent with docs/contracts.md
     and docs/internals.md (measurement cache, "unknown stays absent")?
   - R19 using the post-layout notification contract from inside a built-in
     component: is that path public and available to the tool-window host?
   - R15 resumable cursor (component, item index): consistent with how the
     search's visit cursor and budget work today?
   - R14 MultiSelect/QuickPick: popup list reporting its own items — does
     that conflict with the round-3 parts checkpoint or existing semantics?
   - Slice 6 bundling (R17, R20, R21, R23) and D8's version-bump/packaging
     policy against the AGENTS.md verification table.
   - R08's claimed normalization (Runtime vs Drawing) — check the code.
3. Anything still wrong in the scope table against TODO.md.

Output: verdict line first ("ready", "ready with changes" or "needs
rework"). Then per-r1-finding status (compact table). Then new findings by
severity (blocker / should-fix / nit) with file:line evidence, problem,
failure scenario where relevant, and suggested spec change. Finish with what
you did not examine.
