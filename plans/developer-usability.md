# Plan: blind developer tests and public API review

Status: proposed 2026-10-21 and revised after a Codex review. Slice 0's
kit and design checkpoint are on branch `developer-usability`; both tester
probes passed and both pilots were accepted (the kit's README has the
evidence). Slice 0 waits for the maintainer's approval of the checkpoint
and decision on the tasks' size and budget. Every tool, file and check named below as new is a
proposal until a slice implements it.

## Background

The five probe applications (`live_mixer`, `dynamic_documents`,
`diagram_editor`, `log_explorer`, `inventory`) found real gaps, and the
plans they produced are merged. They share one bias: their author already
knew the library and worked with the checkout open. They show what the API
can do. They do not show what a developer meeting it for the first time
can find, understand and get right, and they do not test whether the
public surface follows its own conventions.

This plan adds two kinds of evidence:

- **Blind developer tests.** Testers with no prior context build specified
  applications from the packaged crates and their rendered documentation
  alone. Each tester keeps a friction log, and a harness records each
  build's diagnostics. The findings are triaged, fixed, and the same tasks
  are run again by new testers to measure the change.
- **A reviewable public API.** A generated, checked-in listing of every
  published crate's public API, kept current by CI. A systematic
  consistency audit runs against that listing, together with a survey of
  stricter lints and of the diagnostics consumers see when they misuse the
  API.

App-author slice 9 (tutorial, prelude and module layout) stays in
[its plan](app-author-improvements.md). This plan feeds it: round 1's
findings on discovery and the prelude become slice 9 inputs. Round 2
measures slice 9's result only if slice 9 lands on `main` before round 2
runs and the branch is rebased onto it; otherwise round 2's findings become
inputs to slice 9's own evaluation.

The evidence is exploratory: two AI testers, a few runs per task. It shows
observed failures and successful implementations, not their prevalence
among human developers, and every record of it says so.

## Working rules

The [app-author working rules](app-author-improvements.md#working-rules)
apply, with these specifics:

- One branch, `developer-usability`, from `main`. One commit per slice,
  plus review-fix commits. No merge or push without the maintainer's
  instruction.
- A workspace patch version bump per slice that changes a packaged file,
  with its CHANGELOG entry. Slices 0 to 4 and 7 change only plans, `docs/`,
  `TODO.md`, the root `api/` listing, CI and AGENTS.md, and take no version.
- An independent Codex review (`codex exec -s read-only`) of every slice
  before it is called done. Slices 0, 1 and 4 begin with a design
  checkpoint that Codex reviews and the maintainer approves. Codex cannot
  run Cargo in the read-only sandbox; the author runs the gates.
- Testers never see this plan, the checkout, the gallery sources, Git
  history or earlier friction logs. The prompts given to them name the
  task, not known weak spots.
- Build outputs of blind runs go on disk outside the checkout, never on a
  RAM-backed tmpfs, and their target directories are deleted after the run.
  The records listed in slice 2 are kept.
- A finding is recorded only after the author has verified it against the
  documentation and code, as AGENTS.md requires for TODO.md entries. A
  tester's wrong assumption that recurs across runs is a reason to
  investigate and to rank it higher, not a classification: it becomes a
  documentation or API finding only when the verifier names the misleading
  wording, the discovery obstacle or the contract problem behind it.
  Otherwise it stays tester error.
- Slice 6 may make documented breaking changes, which this plan makes
  explicit scope: each one is marked `Breaking:` in the
  CHANGELOG with its replacement usage, and every repository consumer is
  migrated in the same slice. Persisted schemas are not changed by this
  plan.
- Gates follow the AGENTS.md table. Slice 1 changes the CI workflow and
  therefore updates AGENTS.md's gate table and commands in the same commit.

## Slices in order

### 0. Blind-test kit and pilot

Documentation and an unpackaged harness only, no version.

Design checkpoint first. It decides and records:

- **Isolation.** Each run gets a consumer directory outside the checkout,
  built like the packaged-consumer check in
  [verification](../docs/verification.md): the library packages from
  `cargo package`, extracted and resolved without workspace paths, plus
  vendored third-party dependencies so the run builds offline. The
  directory holds the crates' README files and rendered documentation of
  those packages generated as docs.rs would: each crate with its
  `[package.metadata.docs.rs]` features (for the facade, the defaults plus
  `image-formats`, `svg`, `serde` and `cpu`), the packaged guide included,
  and the dependency documentation that public re-exports link to. The
  checkpoint records the generation command and checks the cross-crate
  links offline. These documentation features are recorded separately from
  each task's compilation features. The tester's working directory names
  no path into the checkout.
- **Enforced isolation.** The repository is public on GitHub, and the
  checkout, the maintainer's instructions and memory sit on the same
  machine. The boundary must be enforced by the launcher, not promised by
  the brief: no network for any tool the tester runs, shell commands
  included (the CLI's own connection to its model is the only exception),
  no web-fetch or web-search tools, and no file reads outside the consumer
  directory apart from the toolchain and system files compilation needs.
  The checkpoint chooses the mechanism (for example a container or user
  namespace with only the consumer directory and the toolchain mounted)
  and verifies it with negative probes that must fail: a shell request to
  GitHub, a web-tool request, and shell and file-tool reads of a canary
  file placed in the checkout.
- **Clean tester context.** A tester session starts without the
  maintainer's user-level instructions, memory or project history: no
  `~/.claude/CLAUDE.md` or auto-memory for the Claude session, and no
  user-level Codex instructions or session history. The checkpoint fixes
  this in the launcher configuration (for example a separate home or
  configuration directory per run) and verifies it from the session's
  startup evidence and with a canary instruction placed in each user-level
  location that must not show up in the session's behavior, not by asking
  the session what it loaded.
- **Source reading.** A docs.rs reader can open the source, so reading the
  vendored pert-ui source is allowed, but each time is logged as a friction
  entry with its reason. Reading anything outside the consumer directory is
  not allowed, and the enforced boundary makes it impossible.
- **Testers.** Codex (`codex exec` with a workspace-write sandbox rooted in
  the consumer directory) and a fresh Claude Code session (`claude -p`
  started in the consumer directory), both inside the enforced isolation
  above. The checkpoint verifies that both can build offline with Cargo
  and view the PNG captures they produce. A human tester
  is welcome with the same kit; if none is available, the evidence record
  says so.
- **Measurement.** A `cargo` wrapper script in the consumer directory that
  every build goes through. It appends a timestamp, the command and the
  full diagnostics to a log. The launcher records when the brief was
  delivered and when the run ended. A summary script reports per run: wall
  time from brief to independently accepted completion or timeout, time to
  the first build, total build time separately, number of builds, error and
  warning counts by code, and the time and build count to the first
  successful headless session. Build and diagnostic counts are descriptive
  observations that depend on the tester's compile strategy, not a
  usability score. The `pert_ui` items the finished code imports are
  listed from its source.
- **Run conditions.** Each run starts from the same prepared state (a
  vendored dependency set and a pre-built cache of third-party crates, or
  none, the same for every run) and records the model, its reasoning
  effort, the CLI and toolchain versions and any interruption.
- **Acceptance.** Each task's acceptance checks are written before any run
  and applied by the author, not the tester: a review of the finished
  source against the checklist plus the author's own headless test of the
  required behaviors where the tester's session might exercise only what
  it implemented. Unfinished submissions are evaluated the same way.
- **Budget.** A wall-time limit per run. An unfinished task is a result,
  recorded with how far the tester got.

Checkpoint outcome, recorded with its evidence in the
[kit's README](usability/README.md#checkpoint-decisions-slice-0): a
bubblewrap sandbox per run with a cleared environment and only loopback,
and a host-side proxy that admits the model and authentication hosts only,
reached through a loopback bridge whose socket the tester cannot open.
Two decisions narrow the bullets above. Tool commands are kept off the
network by each CLI's own sandbox, which stays on inside the outer one:
Codex's `workspace-write` sandbox and Claude Code's sandbox in strict mode
with an empty domain allowlist. Tools that reach the web, cloud sessions
or other services from the provider's side are turned off and the probe
checks for them: Claude's web, cloud and scheduling tools and the account's
claude.ai connectors, and Codex's apps, plugins, browser and computer use.
Claude's file tools run outside its
sandbox and are confined by the outer one only; the transcripts are
audited for proxy and model-host use. Instead of canary
instructions in the maintainer's user-level files, which the maintainer's
own sessions load, the probe shows that those files do not exist in the
sandbox and that its environment holds only the run's variables.

Then the kit, in `plans/usability/`:

- `tester-brief.md`: the shared instructions (rules above, wrapper, friction
  log, stop condition). It describes the library only as the documentation
  does.
- `friction-log.md`: the template. One entry per event: time, category
  (*discover*: could not find how; *understand*: found it, unclear how it
  behaves; *compile*: a diagnostic that did not point to the fix;
  *runtime*: behavior differed from the documentation or expectation;
  *docs-wrong*; *missing*: no way to do it; *workaround*), what was tried,
  what was expected, what resolved it and which documentation page or item
  was read, and a severity (*blocker*, *slow*, *annoyance*). A closing
  section lists the concepts the tester had to learn, what worked well and
  the three changes they want most.
- `tasks/`: one specification per task, each with features, constraints
  (the crate features to use, headless verification required) and an
  acceptance checklist. The tasks avoid the probes' domains:
  1. **Settings editor.** Grouped fields: validated text, a number with a
     range and a unit, a choice and a toggle. Apply and Revert with dirty
     state, saving to a file through background work with visible failure,
     and refusing to close with unsaved changes. A headless session shows
     an invalid value refused and a failed save reported.
  2. **Task list.** A value-driven list with add, remove, reorder, inline
     rename, selection, search filtering and keyboard shortcuts. This task
     overlaps the tutorial app-author slice 9 plans, on purpose.
  3. **Custom control.** A drawn, interactive control the library does not
     ship (a two-dimensional point picker), placed in a layout beside
     built-in controls: pointer drag with capture and cancellation,
     keyboard adjustment, theme-driven focus indication, and its value in
     inspection. A headless session drags it and captures it with the CPU
     renderer.
  4. **Testing an existing app.** A small given application, written from
     the public API and included in the kit, for which the tester writes
     headless tests of specified behaviors and one CPU capture.
  5. **Reserved task.** Written now with its acceptance checks, in another
     domain, and not run or used to guide fixes until round 2, where it is
     run against both the round-1 commit and the branch head.
- `run.sh` and `summarize.py`: building the consumer directory from the
  current checkout's packages, launching and probing runs, and summarizing
  a run's build log.

Pilot: one run of task 2 by each tester, to prove isolation, offline
builds and the logs. Fix the kit, not the library, from what the pilot
shows; the pilot's findings still go into slice 3's triage, marked as
pilot.

### 1. Public API listing

CI, AGENTS.md and the root `api/` directory, no version.

Design checkpoint first. It decides and records:

- **The tool.** Candidates: `cargo-public-api` (a full listing, generated
  from rustdoc JSON), `cargo-semver-checks` (reports breaking changes
  against a baseline, not a listing) or both. The checkpoint verifies, not
  assumes, which toolchain each needs; rustdoc JSON output is unstable, so
  a tool may need a pinned nightly beside the pinned stable toolchain. That
  would be the first nightly in the project, which the checkpoint weighs
  against the stable-toolchain policy. The tool is installed at a pinned
  version with `--locked`.
- **Configurations.** Which feature sets are listed per published crate:
  at least the facade's defaults and `--no-default-features`, and the
  features that add public items (images, SVG, serde, CPU). The listing
  must match the configurations CI already supports; no `--all-features`.
- **Location and format.** One file per crate and configuration under a
  root `api/` directory, outside every package, so packaging is
  unaffected.
- **Freshness.** A CI step on one Linux job regenerates the listings and
  fails on a difference, with the regeneration command in its message.

Implementation: the listings, the CI step, and in the same commit the
AGENTS.md gate table and commands and CONTRIBUTING.md's description of how
to regenerate. AGENTS.md gains the rule that a change to `api/` is part of
the review and every added, removed or changed item appears in the
CHANGELOG entry.

Gate: the workspace gate, the new step run locally, and author review of
the workflow. The CI run itself happens only after an authorized push; until
then the evidence record says it is pending.

### 2. Blind test, round 1

Documentation only, no version.

Run tasks 1 to 4 on `main` at the start of the slice, each once by Codex
and once by Claude, from consumer directories built by `run.sh`. Runs are
sequential, or parallel only with separate target directories that fit on
disk.

Keep in `plans/usability/runs/`, one directory per run: the friction log,
the build summary, the run conditions, the commit it ran against, the
consumer source as submitted, the author's acceptance result, the captures
the task requires, and the full text of every diagnostic a *compile* entry
or the summary points to. Do not keep the target directories or the rest
of the raw build log.

### 3. Triage of round 1

Documentation only, no version.

- Verify every friction entry against the documentation and code. Classify
  it as a documentation gap, documentation error, API ergonomics, defect,
  missing capability or tester error, and give it the item or page it
  concerns.
- Merge duplicates across runs and rank them by how many runs hit them and
  by severity.
- Collect the compile diagnostics that misled testers, from the build
  summaries and the *compile* entries. They feed slice 4's diagnostics
  survey.
- A Codex review of the triage. It reads the friction logs and the
  classification and reports misclassified or missed findings. Codex also
  produced half the logs, so the review record states that it reviewed
  the triage of its own runs; a finding Codex's review reclassifies or
  drops in one of its own runs is also checked by a fresh Claude agent.
- Record the outcome in a findings table in this plan, and the metrics in a
  new "Developer usability" section of
  [verification](../docs/verification.md). Discovery and prelude findings
  are added to app-author slice 9's inputs. Verified deviations, defects
  and concrete improvements outside this plan's fix scope go to TODO.md;
  optional capabilities that need an application to show the need go to
  [follow-ups](follow-ups.md).
- State in the findings table and the verification section that the
  evidence is exploratory, and which model family tested, triaged and
  reviewed each part.

### 4. API consistency audit

Documentation only, no version. The rubric, the inventories, the
construction, builder, name and type audit and the lint survey read only
the listing and the code, and may run in parallel with slices 2 and 3. The
discovery item and the diagnostics survey start from round 1's imports and
slice 3's misleading diagnostics, and wait for slice 3.

Design checkpoint first: the rubric, which Codex reviews before the audit.
Its sources are the project's own conventions (AGENTS.md's component API
conventions and Rust invariants, [contracts](../docs/contracts.md)) and the
Rust API Guidelines checklist. It covers at least:

- **Construction.** `new`, `bound` and `for_parameter` used as the
  documented exclusive sources; `from_*`, `try_*` and `with_*` used for one
  meaning each; required state owners leading constructor arguments.
- **Builders and callbacks.** Builders that replace, restrictions that
  compose with `Permission::and`, `#[must_use]` on builders and handles,
  and one signature shape and timing vocabulary for `on_*` callbacks.
- **Names.** No `get_` getters; one word per concept across families;
  units named where a value has one; an enum where a `bool` parameter
  hides meaning.
- **Types.** Common traits (`Debug`, `Clone`, `PartialEq`, `Eq`, `Hash`,
  `Default`) on value types where they make sense; `Send` and `Sync` as
  documented; `#[non_exhaustive]` where growth is expected; error types
  named, typed and with `Display` text that says what failed, with which
  value, and what to do.
- **Discovery.** Items a consumer needs that are reachable only through a
  deep path, and prelude contents compared with what the probes and round
  1 imported.

Inventories for each rubric item are generated from the `api/` listing
with a small unpackaged script in `plans/usability/`. Two auditors work
independently, Codex and a fresh Claude agent, as the
[plan audit](plan-audit-repairs.md#background) did; findings only one
reports are verified by the other.

Two surveys complete the audit:

- **Lints.** Run candidate lints as warnings across the workspace and count
  the real hits per lint after reading them: at least
  `missing_debug_implementations`, `unreachable_pub`,
  `clippy::must_use_candidate`, `clippy::return_self_not_must_use`,
  `clippy::missing_panics_doc`, `clippy::fn_params_excessive_bools` and
  `clippy::struct_excessive_bools`. Record which are worth denying and the
  exceptions each would need.
- **Diagnostics.** For the traits consumers implement (`Component`,
  `ListItems`, `TreeItems`, `TableItems`, `ContentRows`, `Clipboard`), the
  sealed ones (`StateOwner`) and the blanket extension traits consumers
  only call (`IntoElement`: a missing import, a missing `Component`
  implementation, an attempted own implementation), the compiler messages a
  consumer gets for the common mistakes, starting from slice 3's misleading
  diagnostics, and whether `#[diagnostic::on_unimplemented]` would point to
  the fix. Decide how messages are tested: `compile_fail` doctests check
  only that code fails, while `trybuild` compares the message text, which
  changes with the compiler and would run only on the pinned toolchain.

Then the joint checkpoint with the maintainer: slice 3's triage and this
audit together, and the maintainer's choice of what slices 5 and 6 fix.
Findings not chosen keep slice 3's destinations, with the reason: verified
deviations and concrete improvements go to TODO.md, and optional
capabilities waiting for an application's need go to
[follow-ups](follow-ups.md). Not choosing a fix does not change where a
finding goes.

### 5. Documentation and guide fixes

Rustdoc and the packaged guide; a version.

The documentation gaps and errors chosen at the checkpoint. Each fix names
the friction entries it answers. Examples a tester was missing become
doctests. Gate: the rustdoc row of the AGENTS.md table, plus the headless
sessions of affected examples.

### 6. API fixes

Public API; one version per sub-slice, which the checkpoint fixes. Probable
sub-slices, each shaped by what is chosen:

- **6a Consistency.** Renames, signature and trait changes from the audit,
  as `Breaking:` entries with their replacement usage. The `api/` diff
  shows every change.
- **6b Lints.** The chosen lints added to `[workspace.lints]` and to
  AGENTS.md's lint list, their hits fixed, and each exception an
  `#[expect(..., reason = ...)]` at the item.
- **6c Diagnostics.** `#[diagnostic::on_unimplemented]` where the survey
  showed it helps, rewritten error `Display` text, and the tests the
  checkpoint chose for both.
- **6d Ergonomics and defects.** The API ergonomics findings and defects
  from round 1 that were chosen, each with a regression or contract test.

Gate: the public-API row of the AGENTS.md table for each sub-slice, and the
dependency row if a sub-slice adds a dev-dependency such as `trybuild`.

### 7. Blind test, round 2, and records

Documentation only, no version.

- Run tasks 1 to 4 again on the branch head, with new tester sessions,
  built by the same kit under the same run conditions (or the differences
  recorded). Run the reserved task 5 against both the round-1 commit and
  the branch head, so that fixes aimed at the known tasks are not the only
  thing compared.
- Compare the per-run results and the recurrence of round 1's findings in
  the verification section, as exploratory evidence. A finding that recurs
  is reopened, not declared fixed.
- Status lines, the review record, CHANGELOG cross-references, and the
  plan archived to `plans/archive/` when the maintainer closes it, with
  its own relative links and the links to it from other plans, `docs/`
  and TODO.md updated and checked in the same change.

## Not in this plan

- Property, model-based and fuzz testing, mutation testing and visual
  regression captures. They are separate stability work.
- Native evidence on Windows and macOS (TODO.md NATIVE-01 to NATIVE-05).
- `cargo-semver-checks` against a published baseline. It becomes useful
  after the first publication and is recorded in
  [follow-ups](follow-ups.md) if slice 1 does not adopt it.
- Implementing app-author slice 9. This plan only adds inputs to it.
- Publication, or recruiting external developers. Both need the
  maintainer's decision.

## Review record

2026-10-21, Codex (`codex exec -s read-only`) on the proposed plan, static
only: no Cargo, tester, isolation probe or tool was run. It reported five
major and five minor findings, all verified against the plan and code and
applied above: isolation enforced by the launcher and verified with
negative probes and canaries; timing from brief to accepted completion
with recorded run conditions; author-applied acceptance checks and the
retained run records; the evidence labelled exploratory, with a reserved
task as the round-2 baseline comparison; recurrence as a reason to
investigate rather than a classification; docs.rs features for the
rendered documentation; `IntoElement` as a blanket extension trait in the
diagnostics survey; slice 4's dependent parts waiting for slice 3; one
rule for TODO.md and follow-up destinations; and link repair on
archiving. It also found that the workspace-gate block in
`docs/verification.md` omits the packaged examples' headless sessions;
that is outside this plan and recorded in TODO.md.

2026-10-21, Codex on slice 0's design checkpoint and harness, static (it
cannot run bwrap, Cargo or the CLIs): fifteen findings, eleven major. All
were verified and addressed: a cleared sandbox environment; new-only run
directories and a kit marker written last; physical path checks; no
initialization inside command substitution; run-local credential copies
instead of host mounts; probes that fail on any failed check; a wrapper
that keeps stdout and stderr apart and records each build before it runs;
structured build classification; bounded diagnostics with JSON support;
the import list marked as a candidate inventory; acceptance-based
completion with recorded model and effort; acceptance initialization; and
the plan text aligned. The proxy finding (any sandbox process can reach
the allowed hosts) was first resolved by keeping Codex's own sandbox on and
narrowing the guarantee for Claude; once `socat` was installed, Claude
Code's own sandbox was turned on as well, recorded in the checkpoint
outcome.
A first recheck found credential write-back, probe exit handling, session
classification, effective-model recording, task 4's application visible to
all tasks, a broken-pipe hang in the wrapper and an undocumented public type
in the application; these were fixed. A second recheck confirmed them and
found three remaining points (the probe accepting `cargo build --help`, a
kit from an earlier script keeping task 4's application, and interruption
causes), which were fixed; Codex's effective model stays unrecorded, as the
kit's README says. Codex reviewed turning on Claude Code's own sandbox and
found that the proxy socket was open to the tester's processes and that the
probe's network and write checks could pass on the session's prose or on a
skipped command; the socket was hidden in a nested namespace and the checks
now read the exact commands' recorded results. Two rechecks confirmed the
fixes. The maintainer's first Claude tester probe passed its isolation
checks but showed the account's claude.ai connectors being tried and the
cloud and scheduling tools present; turning them off and checking for them
was reviewed by Codex, whose two findings in the new check (a tool list
that could default to empty and a missing init event passing) were fixed
and confirmed. The first Codex tester probe found that `web_search =
"disabled"` leaves Codex's apps server with a server-side web tool, which
the session used to open the project's GitHub page; the apps and related
features are now off and checked, and the probe's shell checks rest on
host-side evidence (the proxy log and the tester's home) and the log of a
read-only script the session runs, since Codex's `--json` output omits
commands run through its script tool. Codex's review of that change found
the canary's content in its file name, incomplete or repeated script logs
passing, prose counted as an apps tool call and a tester-writable script
and log; all were fixed, the last by moving the verdict to the host.
