You are an independent design reviewer for the pert-ui repository. Read
AGENTS.md first. Read-only: do NOT edit files; you cannot run Cargo, so your
evidence is static.

You reviewed revision 1 of plans/document-editor.md; your report is
plans/todo-repairs-review/design-s7.md ("needs rework", three blockers and
nine further findings). The plan is now revision 2 and claims to address all
twelve findings (see its Review record). The benchmark's raw output is in
plans/document-editor-bench/results/.

Recheck:
1. Each of your twelve findings: resolved, partly resolved (what is
   missing) or not resolved.
2. New problems introduced by revision 2, especially in: the physical-pixel
   tiling and the single-glyph failure policy; the per-paragraph direction
   contract; the IME temporary replacement document; the TextChange /
   revision / undo contract; the boundary mapping table; cache generations
   and scroll anchors; the slice order (internal until public extension
   points exist) and the provisional targets.
3. Whether the plan is ready for the maintainer's checkpoint. It is a design
   plan only; nothing is to be implemented now.

Output: verdict line first ("ready for the maintainer's checkpoint",
"ready with changes" or "needs rework"), a compact status table for the
twelve findings, new findings by severity with file:line evidence and the
suggested change, then what you did not examine.
