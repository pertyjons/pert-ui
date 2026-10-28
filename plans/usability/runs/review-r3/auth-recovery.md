# Claude authentication recovery

2026-10-28. Original Claude queue finished; six attempted sessions exited 1
with an authentication error before executing model work or any cargo command.
The exact attempts, conditions and replacement mapping are in
[auth-start-failures.json](auth-start-failures.json). They are infrastructure
failures, retained separately, not usable timing/acceptance evidence.

The global Claude credential had expired and could not refresh. The last
successful tester session had retained its refreshed run-local credential as
run.sh already specifies. A new isolated, tool-free Opus session using a copy
of that retained credential returned OK. No global credential was overwritten,
and the private credential bytes are never included in repository artifacts.

The harness now accepts an explicit PERT_UI_USABILITY_CLAUDE_CREDENTIALS
source; copying, isolation, scope and no-writeback behavior are unchanged.
The source kind, not its path or contents, is recorded in conditions.env.
Both baseline/head replacements use this same harness and credential source,
with unchanged kits, brief, tasks, frozen acceptance, models and budgets.
This is a recorded authentication-only change after the initial harness
freeze, not a silent change to the experiment. The new harness SHA256 is
recorded below. The kits themselves remain immutable.

Fresh replacement sessions run in their original relative order, with
-retry1 appended, and never reuse the failed sessions' work or context.
The replacement queue checks that each original is terminal with the exact
authentication error and no cargo execution; it stops on a further session
error. The existing successful runs are neither repeated nor discarded.
The interruption between runs is a recorded condition of this exploratory
experiment. Subsequent verifiers/reviewers use private configurations and
explicit credential copies, never global customization or writeback.

The item-geometry design-review attempt also exited 1 on authentication;
it delivered no design verdict and is restarted as a fresh isolated review.

Harness SHA256: `643d7759345e3fc855a6ae14f6fc0f833ed0ed8aeb90943e76edccee9c6088dd`.
