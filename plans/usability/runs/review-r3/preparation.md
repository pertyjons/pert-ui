# Round 3 preparation review

2026-10-28, fresh Claude Opus session, read-only tools (Read, Glob, Grep),
no delegation or authored fixes. Initial model requested as `opus`; recheck
pinned to `claude-opus-5-5`, high effort. Codex is the implementation author.

Initial verdict: **not ready**. The reviewer required a predeclared,
symmetric interpretation of task 6's notification inspection criterion,
explicit scope and delegated checkpoint/review roles. It also requested
link-preserving documentation reading, permission to use that tool in the
brief, model pinning, balanced paired ordering, exact head provenance,
sandbox reading/isolation checks and explicit limits of the final claims.

Author response: all incorporated in round-3-plan.md and tester-brief.md.
The task 6 acceptance paragraph and its reserved task and author prompt are
unchanged against a81449f (git comparison and a paragraph equality check).
Task 5 acceptance only records the maintainer's pre-existing interpretation.
Baseline notifications.rs confirms that visible_count and Notifications
semantics expose toast existence but not its message or error severity;
the grading rule uses existence plus independent evidence of displayed
failure content, on both commits. No post-submission grading change.

[Recheck](preparation-recheck.md): all initial issues resolved except the
new erroneous error-role wording; remove it, replace task 5's "outside-row"
phrase and explicitly give acceptance agents this interpretation. All three
corrected and author-checked. The reviewer found the plan otherwise ready
subject to the stated prelaunch checks. Raw initial CLI output was inspected
by the author. Prelaunch evidence is recorded in round-3-plan.md.
