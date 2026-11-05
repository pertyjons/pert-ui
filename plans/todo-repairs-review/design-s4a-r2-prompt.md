You are an independent design reviewer for the pert-ui repository. Read
AGENTS.md first. Read-only: do NOT edit files; you cannot run Cargo, so your
evidence is static.

You reviewed revision 1 of plans/todo-repairs-r14-popups.md (report
plans/todo-repairs-review/design-s4a.md, "needs rework"). The maintainer chose
option C and adopted your positions on the open questions. Revision 2 claims
to address every finding (see its status/record section). Content search over
semantic items (R15) is committed (7eb3e6b); a pending fix will make resumed
item scans recheck modality and keep the gallery search from indexing its own
picker.

Recheck:
1. Each finding of design-s4a.md: resolved, partly or not?
2. New problems in revision 2: the keyed placement records (validity across
   source adoption, child reconciliation, failed builds; no inheritance by a
   new incarnation), the semantic_item hook contract (no signal reads, retained
   state), the R15 overlap policy and its tests, QuickPick's intentional
   inspection changes, the ListItems doc exception.
3. Whether the note is ready to implement.

Output: verdict line first ("ready to implement", "ready with changes" or
"needs rework"), a compact status table, new findings with file:line and the
suggested change, then what you did not examine.
