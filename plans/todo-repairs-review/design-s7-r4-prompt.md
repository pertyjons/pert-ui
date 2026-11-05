You are an independent design reviewer for the pert-ui repository. Read
AGENTS.md first. Read-only: do NOT edit files; you cannot run Cargo, so your
evidence is static.

You reviewed revisions 1–3 of plans/document-editor.md (reports
plans/todo-repairs-review/design-s7.md, design-s7-r2.md and design-s7-r3.md). Revision 4
claims to address every point of design-s7-r3.md (see its Review record).

Recheck:
1. Each point of design-s7-r3.md (blocker 1, findings 2–7): resolved,
   partly resolved (what is missing) or not resolved.
2. New problems in revision 4, especially: pert-ui's own outline flattening
   and the per-cell work charge (soundness against zeno's actual cell/row
   behavior; quarter-pixel tolerance vs rendered coverage), the banded
   rendering bound, whole-glyph markers on a paragraph-anchored grid, the
   overhang limit and halo in the visible fill, the total IME mapping, the
   frame memory formula, and the conservative undo accounting.
3. Whether the plan is ready for the maintainer's checkpoint (design only;
   nothing is implemented now).

Output: verdict line first ("ready for the maintainer's checkpoint",
"ready with changes" or "needs rework"), a compact status table, new
findings by severity with file:line evidence and the suggested change, then
what you did not examine.
