# Blind-test records

Rounds 1 and 2 of [the developer-usability
plan](../../archive/developer-usability.md). Exploratory evidence from AI
testers: the times and counts describe how each tester worked, not a
usability score.

## Round 1

Run on 2026-10-21.

### Conditions

- **Kit:** prepared from commit 359122e on branch `developer-usability`
  (0.3.52, Rust 1.99.1). The plan says the round runs on `main`; the
  library at 359122e equals `main`'s apart from the move to Rust 1.99.1 in
  0.3.52, which changed no source.
- **Testers:**
  - Claude Code 2.1.306 with model `claude-opus-5-5`, effort high.
  - Codex CLI 0.163.0 with model `gpt-6.1-sol`, effort medium.
- **Isolation:** each tester ran in the [kit's sandbox](../README.md) with a
  budget of 90 minutes. Claude and Codex ran in parallel, one run each at a
  time, each run with its own target directory.
- **Acceptance:** for each run, a fresh Claude agent applied
  [the acceptance procedure](../acceptance.md). It built and tested a copy
  of the submission in the sandbox and added its own headless tests where a
  tester's check was weak; those tests are in its report, not kept here.
  The author spot-checked each report against the source, tightened one
  verdict (r1-1-codex) and confirmed one the agent had marked as borderline
  (r1-2-claude). Codex's review of these records then led to a third
  decision (r1-1-claude, confirmed by an author probe) and a recorded
  reading for moves under a filter (r1-2-codex). Each run's `acceptance.md`
  records those decisions.

### Runs

| Run | Task | Session (min) | First build (min) | Builds (failed) | Friction entries | Acceptance |
|---|---|---|---|---|---|---|
| [r1-1-claude](r1-1-claude) | 1 settings editor | 5.5 | 0.3 | 8 (1) | 7 | partly: an uncommitted typed change does not block closing |
| [r1-1-codex](r1-1-codex) | 1 settings editor | 6.3 | 2.6 | 6 (2) | 6 | partly: native close refusal untested, in-view window added for testing |
| [r1-2-claude](r1-2-claude) | 2 task list | 7.6 | 2.6 | 11 (3) | 8 | partly: actions inert for a shown selection when the query has a trailing space |
| [r1-2-codex](r1-2-codex) | 2 task list | 6.9 | 1.0 | 10 (4) | 4 | met |
| [r1-3-claude](r1-3-claude) | 3 custom control | 8.0 | 1.8 | 21 (8) | 7 | met |
| [r1-3-codex](r1-3-codex) | 3 custom control | 3.8 | 1.6 | 4 (1) | 2 | met |
| [r1-4-claude](r1-4-claude) | 4 testing an app | 4.5 | 1.0 | 8 (3) | 9 | met |
| [r1-4-codex](r1-4-codex) | 4 testing an app | 4.2 | 2.3 | 3 (1) | 5 | met |

No tester read library source. The `/kit/vendor` paths in the
transcripts are compiler and runtime diagnostic locations, or listings of
the directory. The proxy logs show only the model hosts and Codex's denied
analytics host.

### What each directory holds

- **Run records:** `conditions.env`, `summary.md` and `diagnostics.md` (from
  `summarize.py`), `acceptance.md` and `acceptance.env`, and the tester's
  `FRICTION.md`.
- **`submission/`:** the consumer source as submitted, without `Cargo.lock`
  (the kit's vendored registry fixes the versions), build output, logs, and
  the testers' own helpers for reading the rendered documentation as text.
- **`captures/`:** the PNGs that tasks 3 and 4 require. They were
  re-encoded losslessly, pixel-identical, because the library's PNG encoder
  wrote them about a hundred times larger.

Not kept: target directories, transcripts and the rest of the raw build
logs.

### Kit fixes found in round 1

Before round 2, each with what was done (2026-10-27):

- **Task 4's app README:** it gives row paths as `list/<id>/…`, but `Each`
  puts its rows under `list/items/<id>/…`. *Done by the library:* since
  layering part L3 (0.3.74) `Each`'s default container adds no address
  segment, so the README's `list/<id>/…` is the real path. The fixture's
  `.key("row")`, which `Each` replaces with the item's key, stays: `build`
  must return an `Element`, and a key is the only way to make one (TODO.md,
  `Each::new`).
- **Task 4's app `Cargo.toml`:** it requires 0.3.51, which led one tester to
  edit it; the requirement could name the kit's version. *Done:* `run.sh
  launch` sets the copied application's requirement to the kit's version,
  and the fixture itself requires the branch's version.
- **The bridge script:** `bridge.py` sits in `/kit/bin`, where a tester read
  it, and shows how the sandbox is built. It could be mounted outside
  `/kit`. *Done:* it is mounted at `/run/bridge.py`, which the tester's
  nested namespace hides with the proxy socket, and is no longer copied
  into the kit.
- **The acceptance procedure:** it should say plainly that the author's own
  check can establish an item the tester's check covers only partly. These
  acceptances read it that way. *Done:* `acceptance.md` step 3 says so.
- **Friction-log times:** several testers first wrote estimated times and
  corrected them from `date`. Entries were often written in batches, so
  their times are write times rather than event times. *Done:*
  `friction-log.md` asks for the time at the moment of the event and marks
  an entry written later with `(written later)`; round 2's comparison
  treats unmarked times as event times and marked ones as write times.
- **Task 4's fixture, `let _ =`:** an application-lifetime command needed
  `let _ = ui.register_command(…)` while `CommandId` was `#[must_use]`
  (author notes; fix-plan part 6b1 left it for the kit fixes). *Done:* the
  fixture registers the command with a plain statement.

## Round 2

Run on 2026-10-27, after the [API audit fix plan](../api-audit/fix-plan.md)
and [the layering pass](../layering-plan.md), with the measures the
maintainer decided before any run ([the plan's round 2
measures](../../archive/developer-usability.md#round-2-measures)). The
results and findings are in [the plan's round 2
results](../../archive/developer-usability.md#round-2-results).

### Round 2 conditions

- **Kits:** both prepared on 2026-10-27 with `run.sh prepare`, Rust 1.99.1,
  no third-party build cache:
  - `kit-aca8df3`: the branch head aca8df3 (0.3.75), for tasks 1 to 5;
    `dirty=0`.
  - `kit-359122e-r2`: round 1's commit 359122e (0.3.52), for task 5's
    baseline. It was prepared from a worktree of that commit with the
    current harness copied over it, as the plan's measures say. Of the
    copied files, `run.sh` and `friction-log.md` differ from 359122e's, so
    `dirty=2`. The brief and task 5 are unchanged since round 1, so the two
    task 5 kits differ only in the library and the documentation it ships.
- **Testers:**
  - Claude Code 2.1.311 with model `claude-opus-5-5`, effort high.
  - Codex CLI 0.164.0 with model `gpt-6.1-sol` requested, effort medium
    (the effective model is not in Codex's transcript).

  Round 1 used Claude Code 2.1.306 and Codex CLI 0.163.0.
- **Isolation:** the kit's sandbox with round 1's kit fixes and a budget of
  90 minutes. The isolation probes without a tester passed on both kits,
  15 checks each; the probes with a tester session were not rerun for the
  new CLI versions.
- **Order:** the maintainer ran two queues in parallel, one per tester, each
  run with its own target directory. Both queues ran in the same order,
  which mixes task 5's commits: task 5 on aca8df3, task 5 on 359122e,
  task 1, task 2, task 5 on 359122e, task 3, task 5 on aca8df3, task 4.
  The briefs were delivered between 18:37 and 19:38. Every session ended
  by itself (exit 0), none was interrupted.
- **Acceptance:** [the acceptance procedure](../acceptance.md), applied by
  fresh Claude agents: one per task for tasks 1 to 4, judging both
  testers' runs, and two for task 5, one per tester, each judging that
  tester's four runs on both commits with the same criteria. Each built and
  tested a copy of the submission in the sandbox and added its own headless
  checks, whose source is in the run's `acceptance.md`. No native window was
  run (the sandbox has no display).
- **Task 5's notification:** the task asks for a failure "in its row and in
  a notification" and names no component. r2-5-head-codex-2 and
  r2-5-r1-codex-2 showed it in a plain `Label` outside the row. The
  maintainer decided on 2026-10-27 that a failure message visible outside
  the row meets the item, the literal reading of the frozen brief, so both
  runs are met. Both commits are judged by the same rule. That `Toast`'s
  pages do not point to `Banner` is a round 2 finding.

### Round 2 runs

Run names are `r2-<task>-<tester>` for tasks 1 to 4, and
`r2-5-<commit>-<tester>-<n>` for task 5, where `head` is aca8df3 and `r1`
is 359122e. Code lines and types are `summarize.py`'s frozen code-size
measures (definitions in its header): code lines are application plus test,
for task 4 only the lines that differ from the fixture.

| Run | Task | Session (min) | First build (min) | Builds (failed) | Friction entries | Acceptance | Code lines (app + test) | Types |
|---|---|---|---|---|---|---|---|---|
| [r2-1-claude](r2-1-claude) | 1 settings editor | 7.2 | 1.9 | 20 (3) | 5 | partly: Revert leaves a refused draft shown | 281 + 287 | 31 |
| [r2-1-codex](r2-1-codex) | 1 settings editor | 3.5 | 1.1 | 6 (2) | 3 | met | 286 + 196 | 24 |
| [r2-2-claude](r2-2-claude) | 2 task list | 6.8 | 1.0 | 19 (4) | 6 | met | 253 + 282 | 31 |
| [r2-2-codex](r2-2-codex) | 2 task list | 3.0 | 1.0 | 3 (0) | 2 | met | 292 + 201 | 28 |
| [r2-3-claude](r2-3-claude) | 3 custom control | 6.3 | 1.1 | 15 (3) | 6 | met | 345 + 160 | 43 |
| [r2-3-codex](r2-3-codex) | 3 custom control | 3.6 | 1.3 | 5 (2) | 6 | met | 426 + 0 | 41 |
| [r2-4-claude](r2-4-claude) | 4 testing an app | 3.8 | 1.6 | 10 (7) | 5 | met | 0 + 257 | 21 |
| [r2-4-codex](r2-4-codex) | 4 testing an app | 3.1 | 0.5 | 7 (4) | 4 | met | 0 + 230 | 20 |
| [r2-5-head-claude-1](r2-5-head-claude-1) | 5 job monitor, aca8df3 | 9.6 | 1.7 | 21 (2) | 5 | met | 543 + 363 | 44 |
| [r2-5-head-claude-2](r2-5-head-claude-2) | 5 job monitor, aca8df3 | 11.6 | 2.0 | 41 (5) | 12 | met | 530 + 326 | 43 |
| [r2-5-head-codex-1](r2-5-head-codex-1) | 5 job monitor, aca8df3 | 4.6 | 1.5 | 5 (0) | 4 | met | 334 + 199 | 31 |
| [r2-5-head-codex-2](r2-5-head-codex-2) | 5 job monitor, aca8df3 | 4.2 | 1.2 | 7 (3) | 6 | met by the maintainer's decision | 504 + 7 | 29 |
| [r2-5-r1-claude-1](r2-5-r1-claude-1) | 5 job monitor, 359122e | 10.3 | 4.3 | 38 (8) | 6 | met | 739 + 230 | 38 |
| [r2-5-r1-claude-2](r2-5-r1-claude-2) | 5 job monitor, 359122e | 9.1 | 2.6 | 20 (4) | 7 | met | 578 + 389 | 38 |
| [r2-5-r1-codex-1](r2-5-r1-codex-1) | 5 job monitor, 359122e | 3.2 | 0.8 | 5 (2) | 2 | met | 582 + 7 | 29 |
| [r2-5-r1-codex-2](r2-5-r1-codex-2) | 5 job monitor, 359122e | 4.7 | 0.5 | 7 (1) | 2 | met by the maintainer's decision | 597 + 7 | 29 |

Every accepted run's brief-to-accepted-completion time equals its session
time. Friction entries are the `### HH:MM` headings of each `FRICTION.md`,
as the triage numbered them. The split between application and test lines
follows where the code sits: r2-3-codex's check is a `#[test]` in
`src/headless.rs` outside `#[cfg(test)]`, and the Codex task 5 runs check in
a headless mode of the binary, both application code by the frozen
definition.

Source reading: r2-5-head-claude-1 and r2-5-head-claude-2 each logged
reading the vendored `notifications.rs` to confirm that a toast's message
does not reach inspection, as the brief allows. No other tester logged a
source read. `summarize.py` found no mention of the proxy or a model host in
any round 2 transcript.

### What round 2's directories hold

The same as round 1's, with these differences:

- **Task 4's `submission/`** is the bookmarks application as the tester left
  it, the fixture with the tester's tests.
- **Not kept:** besides the target directories, transcripts and raw build
  logs, the testers' text dumps of the rendered documentation
  (`docs-text/`) and the sessions' `.git/` and `.claude/` directories.
- **`captures/`:** the PNGs the tasks require, re-encoded losslessly as in
  round 1 (pixel-identical, verified). r2-4-claude wrote its capture under
  its target directory, so it is not kept; its acceptance regenerated and
  viewed it.

### Round 2 triage

[`triage-r2/`](triage-r2/) holds the triage, as `triage/` does for round 1:

- **`group-1.md` to `group-4.md`:** the verification by four fresh Claude
  agents, one per group: tasks 1 and 2, tasks 3 and 4, task 5 on aca8df3
  and task 5 on 359122e. Each has a table of every friction entry,
  acceptance lead and relevant diagnostic with its verification and class,
  a recurrence matrix of round 1's findings with the reason for each
  *recurred* and *passed* cell, and its new findings. Changes after the
  review are marked "(Codex review)" or "(author decision after check)".
  Their "destination if not fixed" columns are superseded: the maintainer
  decided that every reopened and new finding goes to a round 2 fix plan.
- **`codex-review.md`:** Codex's review of the triage, including the runs it
  produced (needs revision). Its point 4 is the merged list of new findings
  the plan's table starts from.
- **`claude-check-of-codex-runs.md`:** a fresh Claude agent's check of
  Codex's changes to its own runs.
- **`codex-recheck.md`:** Codex's recheck (ready), with the recurrence
  counts read from the matrices.

The triage files cite the runs' working directories
(`~/pert-ui-usability/<run>/work/`, here each run's `submission/`), the kits'
vendored sources and rendered documentation, and throwaway probes, which
are outside the repository and not kept. Two acceptance notes are corrected
by the triage rather than edited: r2-5-head-codex-1's and
r2-5-head-codex-2's say `needs_paint` is reachable only through
`pert_ui::host`, but `pert_ui::testing` re-exports `HostIntegration` too
(group 3, fact P6). Group 2's leads `r2-4-claude#acc-3` and `#acc-4` come
from r2-4-codex's acceptance.

### Kit issues found in round 2

For a later round's kit; none affected a result:

- **Reading the rendered documentation:** the kit ships the guide and item
  pages as HTML, and testers without a browser converted them to text
  (r2-2-claude with pandoc, r2-5-head-claude-2 with its own script). The
  kit could ship a text rendering, or the brief could say how to read the
  HTML.
- **Task 5's acceptance item 3:** [the acceptance checks](../acceptance.md)
  do not say what counts as a notification. The maintainer's reading above
  should be written into them before task 5 is run again.
- **Tester tooling:** Codex's tool environment refused an `rm -f` in
  r2-5-r1-codex-2; the tester noted it as unrelated to pert-ui. No change.
