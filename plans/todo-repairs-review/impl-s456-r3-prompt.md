You are an independent code reviewer for the pert-ui repository. Read
AGENTS.md first. Read-only: do NOT edit files; you cannot run Cargo, so your
evidence is static.

Recheck two fix commits against your earlier reports:
- b0133d2 answers plans/todo-repairs-review/impl-s45-r2.md (slices 4 and 5
  recheck): QuickPick::searchable opt-out instead of excluding every picker,
  explicit item/row order test, full window request queue, partial stripe
  clipping, alert paths and cancel.
- a2f1099 answers plans/todo-repairs-review/impl-s6.md (slice 6): zero-width
  border, anchors' metadata timing, command-level anchor tests, adaptive
  roles, pressed frameless button, inspection scheduling nothing, PNG
  aspect-ratio and release-only size-limit tests, FrameImage docs.

For each finding of both reports: resolved, partly or not? Then any new
problems (file:line). Note the slice 4b merge (7cf441c) sits between them;
review only these two commits plus what they touch.

Output: verdict line first ("ready" or "needs fixes"), a compact status
table, new findings, then what you did not examine.
