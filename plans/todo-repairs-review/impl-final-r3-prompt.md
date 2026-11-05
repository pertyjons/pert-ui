You are an independent code reviewer for the pert-ui repository. Read
AGENTS.md first. Read-only: do NOT edit files; you cannot run Cargo, so your
evidence is static.

Recheck only commit 69981d8 on branch todo-repairs against
your report plans/todo-repairs-review/impl-final-r2.md. It tightens test
assertions in crates/pert-ui/tests/collections/popup_items.rs,
crates/pert-ui/tests/presentation/disabled_button.rs (masks_in, differing,
check_muted_parts: parts identified against a reference control because a
scene shares masks by content), makes the #[cfg(test)] stroke fault in
crates/pert-ui-runtime/src/shapes.rs a drop guard used in select.rs, and
updates plans/todo-repairs.md and CHANGELOG.md.

For each finding of impl-final-r2.md: resolved, partly or not? Check in
particular whether the reference-control comparison could pass vacuously
or for the wrong reason (for example if masks were not shared by content,
or if differing sizes change shared masks), and whether the records are
now accurate. Then any new problems with file:line, and whether the
branch's repairs are ready to merge from a code-review standpoint (native
checks are out of scope).

Output: verdict line first ("ready", "ready with nits" or "needs fixes"),
a compact status table, findings, then what you did not examine. Be brief.
