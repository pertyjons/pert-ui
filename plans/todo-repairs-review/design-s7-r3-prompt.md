You are an independent design reviewer for the pert-ui repository. Read
AGENTS.md first. Read-only: do NOT edit files; you cannot run Cargo, so your
evidence is static.

You reviewed revisions 1 and 2 of plans/document-editor.md (reports
plans/todo-repairs-review/design-s7.md and design-s7-r2.md). Revision 3
claims to address every point of design-s7-r2.md (see its Review record).

Recheck:
1. Each point of design-s7-r2.md (blockers 1–2, findings 3–12): resolved,
   partly resolved (what is missing) or not resolved.
2. New problems in revision 3, especially: identity tokens compared by
   pointer (serialization, Eq/Hash, persistence rules in AGENTS.md); the
   pre-allocation raster admission with outline bounds widened by one pixel
   (soundness for hinting, synthetic bold/oblique, color/bitmap glyphs,
   variable fonts); the replacement-marker policy; the bounded visible fill;
   cropped per-paragraph tiles and the memory bound; IME mappings; undo
   restoring earlier revisions; slice 7's public data types.
3. Whether the plan is ready for the maintainer's checkpoint (design only;
   nothing is implemented now).

Output: verdict line first ("ready for the maintainer's checkpoint",
"ready with changes" or "needs rework"), a compact status table, new
findings by severity with file:line evidence and the suggested change, then
what you did not examine.
