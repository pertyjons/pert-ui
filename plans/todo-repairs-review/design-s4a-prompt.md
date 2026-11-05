You are an independent design reviewer for the pert-ui repository. Read
AGENTS.md first. Read-only: do NOT edit files; you cannot run Cargo, so your
evidence is static.

Review plans/todo-repairs-r14-popups.md, the slice 4a design checkpoint of
plans/todo-repairs.md (see its R14 section and decision D9): how MultiSelect
and QuickPick items get rectangles so testing::Driver::click_item can click
them. The note recommends option C (items reported by an opt-in on
VirtualList, fed by a proposed ListItems::semantic_item) over options A, B
and D.

Evaluate: accuracy of every claim about the current code; whether option C
conforms to AGENTS.md (no privileged paths, public API documented, identity
rules, reconciliation rules, "unknown stays absent", inspection must not
schedule work); the rejection reasons for A, B and D; semantics preservation
(labels, selection/checked, disabled activation, shortcuts, actions); the
interaction with content search item hits (R15, now implemented:
Semantics::items names are searched) — duplicate hits for MultiSelect's two
item sets; QuickPick's re-keyed list; the test list; the gate. Answer the
note's open questions with your recommendation.

Output: verdict line first ("ready for the maintainer's checkpoint",
"ready with changes" or "needs rework"), findings by severity with file:line
evidence and the suggested change, your positions on the open questions,
then what you did not examine.
