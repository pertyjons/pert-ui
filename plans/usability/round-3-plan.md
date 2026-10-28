# Round 3 execution and closure

Status: preparation, 2026-10-28. The maintainer authorized completion through
round 3 as a persistent goal, including kit preparation, blind testing,
verification and repair of new and recurring findings, Claude review and a
documented final assessment. This executes the
[round 3 proposal](round-2-fix-plan.md#round-3-proposal).

## Scope and completion

Complete the paired task 6 comparison and head-only tasks 1 to 5, accept
each submission against the frozen checks, verify recurrence and new leads
against code and documentation, repair all verified in-scope findings and
obtain independent Claude review of the changes and final assessment.
Required gates follow AGENTS.md for each change. Findings are not dismissed
because a submission compiles or its own tests pass. Record partial or failed
submissions, isolation violations and missing evidence explicitly.

Completion requires the run records, acceptance evidence, verified findings
and their dispositions, reviewed fixes, relevant gate results and final
assessment. Do not infer human usability or platform support from AI tests.
The existing unrelated TODO entries, app-author tutorial, Windows/macOS
native evidence and publication remain outside this usability round. Newly
verified unrelated deviations go to TODO.md. Native checks affected by repairs
are run where available; missing platforms remain explicit.

## Frozen experiment

- Baseline: 26e6b5a (0.3.75), as proposed, rather than substituting a nearby
  commit. Head: library at 86a196f (0.3.88), plus the preparation-only harness
  and documentation commits recorded before launch. Do not change a kit
  during its runs or replace the experimental head with later fixes.
- Task 6 and its acceptance paragraph remain byte-identical to their seal
  a81449f. Its author prompt stays sealed too. Tasks 1 to 5 remain unchanged.
  Task 5's acceptance now explicitly records the already-decided outside-row
  notification rule. Freeze acceptance before launching any run.
- Use the same current harness files, brief, settings and friction template
  on both commits. For documentation reading use the proposal's permitted
  brief instruction: `pandoc -f html -t plain <page>`. Both kits have the
  identical instruction and the same sandbox `/usr/bin/pandoc`; this produces
  text on demand rather than pre-rendering text files. Record the overlay and
  exact CLI, model, effort, Rust and pandoc versions. No shared build cache.
- Fresh isolated sessions; Claude and Codex have one serial queue each.
  Each queue: task 6 head #1, task 6 baseline #1, task 1 head, task 2 head,
  task 6 head #2, task 6 baseline #2, task 3 head, task 4 head, task 5 head.
  Each paired subsequence alternates commits. Queues may run concurrently.
- Claude uses `opus`, high effort; Codex uses the configured `gpt-6.1-sol`,
  medium effort. Record actual configured/effective identity as the harness
  supports; do not claim an unobserved effective model. Budget 90 minutes
  per run, unchanged. Rerun both CLI isolation probes for the new versions,
  and the no-tester probe on each kit before task runs.
- Fresh Claude acceptance contexts apply the same procedure to both commits,
  adding checks to copies where required, with source and output preserved.
  Independently check acceptance and triage, particularly Codex submissions.
  Keep paired task 6 conclusions apart from head-only comparisons. Use
  summarize.py's existing counting definitions unchanged.

## Execution log

- 2026-10-28: clean checkout at 86a196f on main; Claude Code 2.1.312 and
  Codex CLI 0.165.0 installed and authenticated. Task 6 seal confirmed in
  the existing runs README. Both round 2 kit issues are addressed in the
  brief and acceptance checks; no packaged library file changed.
