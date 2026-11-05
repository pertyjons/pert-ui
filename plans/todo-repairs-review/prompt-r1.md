You are an independent reviewer of a repair specification in the pert-ui
repository (a Rust GUI library). Read AGENTS.md first: it is the governing
instruction file. Then evaluate `plans/todo-repairs.md` against `TODO.md`,
the code and the design notes (`docs/`). Do NOT edit any file; this is a
read-only review. You cannot run Cargo (read-only sandbox); your evidence is
static, so say so where a claim needs a run.

The maintainer asked to repair every TODO.md entry that does not depend on
Windows or macOS. The spec classifies the 28 open entries, verifies each
against `main` (621022a, 0.3.94), and proposes eight slices.

Evaluate, and look hardest at:

1. Scope classification. Is every one of the 28 open TODO.md entries
   accounted for exactly once? Is any entry wrongly excluded as Windows/macOS
   (or wrongly included)? Is the clipboard-win treatment right?
2. Current-state claims. For each R01-R22, check the cited file:line and the
   described behaviour against the code. Report any claim that is wrong,
   stale or imprecise.
3. Proposed fixes. For each item: does the fix actually resolve the TODO
   entry? Does it conform to AGENTS.md (public API documentation, typed
   errors, "unknown stays absent", no signal publication from reconciliation,
   identity rules, privileged-path rule, dependency rules, verification table,
   lint-change discussion)? Is there a simpler or more correct fix? Name
   concrete risks: R12 summary threshold and public ValidationIssue shape,
   R13 code reuse vs new variant, R14 geometry source and scrolled rows, R15
   ContentHit shape and jump semantics, R16 part naming vs the existing parts
   checkpoint, R17 retained paint metadata, R18 EachItem as a trait-object
   return type (check its definition and whether the collection can still
   assign the key), R19 detach timing and duplicate requests, R20 shared
   helper, R21 own deflate vs miniz_oxide (determinism, correctness risk,
   test plan).
4. Tests. Missing or weak tests per item; tests that would not catch the
   regression they target.
5. Slicing, order, versions, gates. Is each slice's gate the right AGENTS.md
   row? Hidden dependencies between slices (api/ regeneration, CHANGELOG,
   diagnostics snapshots, gallery)?
6. Slice 7 (TEXT-01) and slice 8 (Linux native evidence): is the boundary
   right and is anything missing?
7. The decisions D1-D7: do you agree with each recommendation? Why or why not.

Output format: a verdict line first ("ready", "ready with changes" or "needs
rework"). Then findings ordered by severity (blocker / should-fix / nit),
each with: item ID, file:line evidence, the problem, a concrete failure
scenario where relevant, and the suggested change to the spec. Then your
position on D1-D7. Finish with what you did not examine.
