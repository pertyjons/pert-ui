# Plan: blind developer tests and public API review

Status: proposed 2026-10-21 and revised after a Codex review. Slice 0 is
done on branch `developer-usability`: both tester probes passed, both
pilots were accepted (the kit's README has the evidence), and the
maintainer approved the checkpoint on 2026-10-21, keeping the tasks and
their 90-minute budget. Slice 1 is done: the API listings are under
`api/` with their CI job, on Rust 1.99.1 (0.3.52); its CI run waits for
an authorized push. Slice 2 is done: round 1's eight runs are recorded in
`plans/usability/runs/`. Slice 3 is done: 36 findings are ranked in the
round 1 findings table. Slice 4 is done: the API audit's 127 findings and
two surveys are recorded, and the joint checkpoint with the maintainer is
next. Every tool, file and check named below as new is a
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

Checkpoint outcome, approved by the maintainer on 2026-10-21 and recorded
with its evidence in the
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

Checkpoint outcome (2026-10-21), revised after a Codex review and approved
by the maintainer, who chose the `RUSTC_BOOTSTRAP` exception over a pinned
nightly and moved the pinned toolchain to Rust 1.99.1 first (0.3.52). Evidence was gathered with throwaway builds under
`target/`:

- **Tool: `cargo-public-api` 0.52.0 only**, installed with `cargo install
  --locked cargo-public-api@0.52.0` (it builds on stable). Its output is
  committed as it prints it, with `--simplified` once (blanket impls
  omitted) and no filtering of ours: a filter inferring which impl a line
  belongs to from its position was tried and dropped free functions that
  follow an impl. `cargo-semver-checks` (0.51.0) reports breaking changes
  against a baseline and gives no listing; while the crates are unpublished
  and the component review may break them, the listing's diff in review is
  what this plan needs. It is reconsidered at the first publication.
- **Toolchain: an extraction-only exception, no nightly.** rustdoc JSON
  still needs `-Z unstable-options`, and `cargo-public-api` would switch to
  a nightly by itself to build it. `api/generate.sh` instead builds the
  JSON with exactly the pinned toolchain (Rust 1.99.1 since 0.3.52) and
  `RUSTC_BOOTSTRAP=1`, set on that one command only, and passes the file
  with `--rustdoc-json` (a flag the tool's help does not list, held stable
  by the pinned tool version). This is a documented exception for
  extracting the listing: no build, test, check or package of the library
  uses `RUSTC_BOOTSTRAP`, and the library's stable and MSRV contract is
  unchanged. The alternative, a date-pinned nightly used only for
  extraction, states the unstable dependency more directly but adds a
  second toolchain to install and bump. Verified:
  1.99.0 and 1.99.1 emit format 61, which 0.52.0 parses without error, and two
  builds give identical listings.
- **Reproducible inputs.** Target `x86_64-unknown-linux-gnu`; exactly the
  pinned toolchain and tool version; `cargo rustdoc --locked -p <package>`
  with that configuration's features and nothing unified from other
  packages; no color; `RUSTFLAGS` and `RUSTDOCFLAGS` cleared for the
  extraction. The listings are a Linux view: items gated on another
  platform would not appear. They also name dependency paths (for example
  `serde_core::ser::Serialize`) and auto-trait impls that follow
  dependencies, so a dependency or toolchain update regenerates them in the
  same change, and the review reads that churn separately from intended API
  changes.
- **Configurations: those CI already checks**, each package on its own.
  The facade's `pub use pert_ui_runtime::*` is one line, so the runtime's
  listing is the bulk of what a consumer sees.
  - `pert-ui`: default; `--no-default-features`; `--features images`;
    `--features image-formats`; `--no-default-features --features png`;
    `--features svg`; `--no-default-features --features svg`; `--features
    serde`; `--no-default-features --features serde`; `--features cpu`;
    `--no-default-features --features desktop-cpu`. Each is at most a few
    hundred lines.
  - `pert-ui-runtime`: default and `serde` (CI builds it with `serde`
    through the facade); about 37,000 lines and 3.9 MB each, mostly
    because the tool, since version 0.51, lists every default trait method
    under each impl, which repeats `Component`'s methods for every widget.
  - `pert-ui-core`: default and `serde`, about 1,000 lines each.
  - `pert-ui-cpu` and `pert-ui-wgpu`: default, under 100 lines each.
- **Location.** `api/<package>/<configuration>.txt` at the root, with
  `api/generate.sh` that removes the old listings and writes every listing
  again; nothing under `crates/`, so packaging is unaffected.
- **Freshness.** A separate CI job on Linux with the pinned toolchain
  installs the tool, runs `api/generate.sh` (which stops at the first
  failure) and fails when `git status --porcelain -- api/` shows any
  changed, new or removed file, printing the regeneration command. The
  workspace jobs are unchanged. Generating the listings took about two
  minutes locally.

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

#### Round 1 findings

Outcome of slice 3 (2026-10-21). The evidence is exploratory: two AI
testers, one run per task each, plus the two task 2 pilots (marked
separately).

**Who did what:**
- **Testing:** Claude (Opus 5.5) and Codex (`gpt-6.1-sol`).
- **Verification:** five fresh Claude agents checked every entry against
  the rustdoc, the guide and the code, with throwaway probes for runtime
  claims. Their per-entry tables are in
  [`usability/runs/triage/`](usability/runs/triage/).
- **Merging and ranking:** the author (Claude) merged and ranked the
  entries, and spot-checked the decisive claims in the source.
- **Review:** Codex reviewed the triage, including the runs it produced
  itself. Two of its changes concerned its own runs (F4's count and F7's
  split); a fresh Claude agent checked both against the records and the
  source, agreed, and found that r1-1-codex's in-view close does save
  drafts, which corrected that run's acceptance note.

The 64 friction entries resolve as follows:
- 36 findings in the table below. F1 is a kit issue today. F30 to F34
  were found while verifying, not by a tester.
- Kit issues, listed in [the runs' README](usability/runs/README.md#kit-fixes-found-in-round-1).
- Tester errors: r1-1-codex#5, r1-2-codex#2 (its guesses, not its prelude
  part), r1-3-claude#4 and pilot-2-codex#5.
- Notes with no problem: r1-4-claude#6–#9, r1-4-codex#4–#5.

Ranking is by round 1 runs that hit the finding, in a friction entry or in
their code and acceptance notes, then by the worst severity a tester gave.

**"If not fixed"** is where a finding goes if the joint checkpoint after
slice 4 does not choose it:
- **S9:** app-author slice 9's inputs.
- **FU:** [follow-ups](follow-ups.md).
- **TODO:** TODO.md.

| # | Finding | Class | Item or page | Runs (pilot) | Worst | If not fixed |
|---|---|---|---|---|---|---|
| F1 | The guide's dependency instructions say to use a checkout because no registry release exists; the kit stood in for a release. Two other chapters already use a registry requirement. | Kit issue now, documentation error at the first release | guide/getting-started.md "Run your first window" | 6 (2) | annoyance | S9 and the release checklist; the round 2 brief says the kit stands in for a release |
| F2 | Testing an application headlessly has no chapter and no helpers. The task 4 testers, the task 1 testers in part, the pilots and the packaged examples' sessions built recurring helpers: click at bounds, replace a field's text, a key chord, find a component's semantics, read label and field text, settle after input. Group E maps each helper to its runs. Further problems:<br>• The typed single lookup `Runtime::query` with `Selector::id` is not pointed to; the `inspection_id` example teaches a full snapshot search.<br>• `text_input("")` is a no-op.<br>• `focus` puts the caret at the start.<br>• Input methods do not say when derived UI is settled.<br>• `pointer_input` has no example. | Documentation gap; a helper type is a missing capability | guide (no testing chapter), `Runtime::inspection_id`, `Runtime::text_input`, `Runtime::pointer_input`, `Runtime::query` | 4 (2) | slow | S9 (chapter), FU (helper type) |
| F3 | The prelude lacks `Modifiers`, `KeyChord`, `CommandKey` and `CommandScope`, against its own rule for keyboard types an application configures; `ListDrop` and `DropPosition`, which `VirtualList::reorderable` takes, are missing too. rustc suggests no `use pert_ui::…` for facade items. | API ergonomics (discovery) | `pert_ui::prelude` | 6 by use, 2 by entries (2) | annoyance | S9 |
| F4 | A focused editor's uncommitted draft is neither committed nor cancelled before `App::on_close`, so a close policy sees no change and an accepted close loses the typed text. Window deactivation commits valid drafts by clearing focus, and so does an in-view `FloatingWindow` close, whose button takes focus. Calling `Runtime::clear_focus` inside `on_close` is an undocumented workaround (r1-1-codex used it). | Defect | `App::on_close`, platform close path | 1 | partly met acceptance | TODO |
| F5 | `App::on_close` cannot be driven headlessly: `App::handle_close` is crate-private, and `take_presentation_requests` says only native hosts need it. | Missing capability | `App::on_close`, `Runtime::request_close` | 2 | slow | FU (driver), TODO (doc lines) |
| F6 | Built-in parse and range rejections of `IntegerInput` and `DecimalInput` do not reach `on_rejection`. Their messages are fixed English strings shown only in inspection, and the fields have no `on_modified`. `TextField` reports its own limits through `on_rejection`. | Missing capability | `IntegerInput::on_rejection`, `DecimalInput` | 2 | slow | TODO |
| F7a | A selection hidden by a filter stays selected, as documented, but no page shows how to tell that the selected row is hidden (from `ListFilter::items()` and `ListItems::index_of` with `ListSelection::active`). | Documentation gap | `CollectionFilter` "Selection and focus", `ListFilter` | 1 | annoyance | TODO, S9 (list recipe) |
| F7b | `CollectionFilter` moves a hidden active item only while the collection has focus, so never while the user types in its field, and offers no policy or outcome callback. `ListFilter::filter` returns `FilterOutcome::nearest_active` only to an application that applies the query itself. | Missing capability (optional policy) | `CollectionFilter`, `FilterOutcome::nearest_active` | 1 | slow | FU |
| F7c | Filters trim a non-blank query before matching, undocumented, while `FilterMatcher::matches` takes the raw query, so an application that repeats the match disagrees with the list (r1-2-claude's partly met item 3, found at acceptance). | Documentation gap | `FilterMatcher`, `ListFilter::filter`, the table and tree filters | 1 (acceptance) | partly met acceptance | TODO |
| F8 | A `Label` reports its text as the semantic name and has no value, while `SemanticValue::Text` and `Semantics::value` describe presented text. | Documentation error | `Semantics::value`, `SemanticValue::Text`, `Label` | 2 | annoyance | TODO |
| F9 | `Each` puts its rows under an implicit `items` segment and replaces the element's own key; documented, but the only example uses `within`. | Documentation gap | `Each` | 2 | annoyance | S9 |
| F10 | `CaptureTarget::window` takes `&NativeWindowId`; the guide names the type but never shows the call. | Documentation gap | guide/diagnostics.md captures, `CaptureTarget::window` | 3 (one only in its diagnostics) | annoyance | TODO |
| F11 | Error conversions point at the wrong type: the blanket `From` for `AppError` makes rustc name `DrawError`, and `Runtime::paint`'s `DrawError` does not convert into `RuntimeError` for a test helper. | API ergonomics (diagnostics) | `AppError`, `DrawError` | 2 | annoyance | TODO, slice 4 diagnostics survey |
| F12 | `Component::cancel_input` does not say whether it may write signals or how a failed restore surfaces; it returns `()`. A restore deferred to `deliver_notifications` is lost if the component is removed first. | Documentation gap | `Component::cancel_input` | 2 | slow | TODO |
| F13 | Starting background work from a callback: `EventContext` has no `spawn`, and the `defer`-then-`Runtime::spawn` pattern that the `diagram_editor` example uses is not documented. | Documentation gap; a convenience is optional | `EventContext`, `Runtime::spawn`, guide/state.md | 2 | slow | S9 (recipe), TODO, FU (convenience) |
| F14 | No recipe for an editable, selectable, filterable list of application records. The guide's task list uses `Each`, which has neither selection nor filtering. | Documentation gap | guide | 1 (2) | slow | S9 |
| F15 | `InlineText` cannot begin editing from code. Testers sent synthetic key presses, which fail while the key is still held. | Missing capability | `InlineText` | 1 (1) | slow | FU, updating "Sessions for more editors" |
| F16 | Per-row controls bound to an application's `Vec` need per-row signals; `Signal::sibling` works but is documented for popup state and discards an open rename when the vector changes. | API ergonomics | `Checkbox`, `InlineText`, `Signal::sibling`, `ListItems::build` | 1 (1) | slow | S9 (per-row state recipe), FU (controlled controls, updating "Controlled controls") |
| F17 | Owning a `DraftEditor` inside rows that are described again needs `retain_initial_children`, and cancellation during the owner's focus transfer is easy to get wrong. | Documentation gap | `DraftEditor`, `Component::retain_initial_children` | 1 (1) | slow | TODO |
| F18 | Escape does not call `cancel_input` on the focused component; it silently releases its pointer capture and timer, and then arrives as an ordinary key. The custom-widgets guide says Escape calls `cancel_input`. Not reported by a tester: guide/controls.md says Escape cancels capture owned by nonfocusable components, which a probe disproved. | Documentation error (or a contract change) | guide/custom-widgets.md, guide/controls.md, `Runtime::key_input` | 1 | slow | TODO |
| F19 | `CommandId` is `#[must_use]` although dropping it keeps the command registered, so application-lifetime commands need `let _ =`, and the guide's shortcut example warns when copied. | API ergonomics | `Runtime::register_command`, guide/diagnostics.md | 2 by warnings (1) | annoyance | TODO, slice 4 audit |
| F20 | Examples on the facade's item pages import `pert_ui_runtime` or `pert_ui_core`: 276 of 310 blocks, and an item page does not show that the item is re-exported. | Documentation error | rustdoc examples of re-exported items | 1 (1) | annoyance | S9 |
| F21 | A `Field` passes its description only to children without one, so a refused draft's control reports "Unsaved changes" rather than the field's error. | API ergonomics | `Field`, `TextField` semantics | 1 | annoyance | TODO |
| F22 | Editors take Alt+Up/Down (and Super) before application commands, also where the chord has no editing meaning; undocumented on `TextField`. | Documentation gap | `TextField`, `Runtime::command_key_input` | 1 | annoyance | TODO |
| F23 | `AmbiguousKey` does not name the key. | API ergonomics (error text) | `RuntimeError::AmbiguousKey` | 1 | annoyance | TODO, slice 4 audit |
| F24 | `SemanticValue` has no two-dimensional value; the convention is documented only on `ColorPicker`. | Documentation gap; a variant is optional | `SemanticValue`, guide inspection section | 1 | annoyance | TODO, FU (variant) |
| F25 | `check_redescription` reports a false `UnexpectedOverlap` when a viewport is set. The tester's expectation that `Redescribed::root` is the component was a tester error: the docs say the retained tree under `root`, although a pointer to `Redescribed::component` would help. | Defect | `check_redescription` | 1 | annoyance | TODO |
| F26 | `CollectionFilter` documents `filter-field` as a slot, but the wrapper is structural, so the field's path is `…/filter`. | Documentation error | `CollectionFilter` | 0 (1) | annoyance | TODO |
| F27 | `InlineText` in a list row takes the row's first press, so the list does not select. | Documentation gap | `InlineText`, `VirtualList` | 0 (1) | slow | TODO |
| F28 | `Signal::update` requires `PartialEq`; the signal page says so, the guide does not. | Documentation gap | guide/state.md | 0 (1) | annoyance | TODO |
| F29 | An empty `Label` still takes height, unlike an empty `Field` block. | API ergonomics | `Label` | author note | — | TODO |
| F30 | `ApplicationError::new` rejects a boxed `dyn Error + Send + Sync`, and there is no `From` for it, so an application using a boxed error loses the source chain. | Missing capability | `ApplicationError` | author-found (group A) | — | TODO |
| F31 | With the key `CollectionFilter` documents as valid, `filter`, the collection and the search field share one address and `find` reports `AmbiguousKey`. | Defect | `CollectionFilter` | author-found (group C) | — | TODO |
| F32 | `ColorPicker::cancel_input`'s comment says writes are refused when hiding or removing; they succeeded on every such path probed. | Documentation error (code comment) | `ColorPicker` | author-found (group D) | — | TODO |
| F33 | A fixed-size custom control must unwrap `SizeLimits::new` or switch to `measure_limits`; there is no infallible constructor for a fixed size. | API ergonomics | `SizeLimits` | author-found (group D) | — | TODO |
| F34 | The packaged examples press modifiers through `command_key_input` while the guide sets them with `native_modifiers`: two recipes for one thing. | Documentation gap | guide/diagnostics.md, the examples' sessions | author-found (group E) | — | S9 |

Two other observations:
- `FrameImage::encode_png` writes stored deflate blocks, a documented
  trade-off for determinism and no dependency, which makes captures about a
  hundred times larger than a compressed encode. It is outside this plan's
  scope and is recorded in TODO.md.
- A lead, not verified: the example sessions flush three times to settle.

**Compile diagnostics that misled testers,** for slice 4's survey; the
full texts are in the runs' `diagnostics.md`:
- `Modifiers` not found: rustc suggests the unrelated `Notifier`.
- `CommandScope`, `KeyChord` and `CommandKey` not found: no help at all.
- `?` or `AppError::from` on an unsupported error: the message names
  `DrawError` (F11).
- `paint()?` into `RuntimeError`: the cause is named but no fix is offered.
- `Modifiers::CONTROL`: no hint at the fields.
- `E0614` "f64 cannot be dereferenced" after `entry.properties`: a cascade
  from the `E0609` on the same expression, fixed by `entry.details.properties`
  alone.

The others, among them the `CaptureTarget::window` borrow, the
`PartialEq` bound, `details.properties` and `with_grow`'s `u16`, led
straight to the fix.

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

Checkpoint proposal (2026-10-21): [the rubric](usability/api-audit/rubric.md),
with its sources, the inventory method, two independent auditors with
cross-verification, the rubric items (construction, builders and
callbacks, names, types, discovery) and the survey procedures. Codex
reviewed it and approved it after two rechecks, and the maintainer
approved it on 2026-10-21.

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

#### API audit findings

Outcome of slice 4 (2026-10-21), audited at d4ba108. The merged table holds
127 findings, one row each with the record's fields. It lives in [the
findings record](usability/api-audit/findings.md) rather than here, because
the plan could not hold it. This section gives the ranking and the
questions for the checkpoint.

**How it ran:**
- **Inputs:** [`inventory.py`](usability/api-audit/inventory.py) froze the
  inputs, checked byte for byte against every `api/` listing.
- **Audit:** Codex and a fresh Claude agent audited each of the five
  rubric groups independently.
- **Inventory gaps:** the auditors found gaps in the inventory.
  - The script was fixed and the inputs regenerated three times. Both
    auditors reviewed the additions of versions 2 and 3.
  - Version 4 adds rows the auditors had already covered by hand, which
    `inputs/CHANGES.md` records.
  - Two residual gaps were checked by hand.
- **Verification:** each auditor verified the other's findings against the
  source, and the other's delta findings.
- **Disputes:** the author settled the disputed facts against the source,
  recorded per group. One dispute turned out to be a reading of the rubric
  (G2-20) and went to the maintainer.

**Result:**
- **Severity:** no finding is high after verification. Fifteen findings
  are medium or have a medium instance, and the rest are low.
- **Medium findings (the record ranks them):**
  - **Behavior:**
    - **G1-4:** `Workspace::set_settings` can make a saved workspace fail
      to load.
    - **G2-2, G2-14:** the collections ignore a redescription's
      permissions and callbacks.
    - **G2-13:** numeric reconciliation publishes a parameter snapshot.
    - **G2-20:** `on_rejection` differs between the text and number
      editors. This is F6, and whether it departs from the rubric is a
      policy question.
    - **G4-25:** a protected semantic value can leak.
  - **Types and names:**
    - **G2-9:** owners that tear down on drop lack `#[must_use]`.
    - **G3-1:** `status()` and `snapshot()` mean owned or reactive
      depending on the type.
    - **G3-6:** the wgpu renderer takes bare `u32` sizes.
    - **G4-1:** 113 types lack `Debug`.
    - **G3-11:** `Component::tab_navigation` takes two adjacent
      `bool`s.
    - **G4-14:** key errors do not name the key; this is F23.
  - **Discovery:**
    - **G5-2, G5-3, G5-4:** missing routes to `spawn`, to per-row
      bindings and to `retain_initial_children`; these are F13, F14, F16
      and F17.
- **Low findings, mostly consistency:**
  - prelude members the rule includes but the prelude lacks (G5,
    extending F3);
  - units in bare numbers (G3);
  - `# Errors` sections that understate their variants (G4);
  - inconsistent naming within families (G1, G3).
- **Policy questions:** the record's index lists the policy questions by
  theme. Each row marks its own consistency proposals, which are policy
  decisions too. The main questions are below.
  - **Collections:** whether they get a contract exception for
    redescription (G2-2, G2-14).
  - **Role suffixes:** what they mean for snapshots, inspection records and
    operation results (G3-17, G3-18).
  - **Prelude rule:** how far "every type its signatures name" reaches
    (G5-26).
  - **`get()` and limit constants:** how far each applies (G1-5, G1-9,
    G1-10).
  - **Standard-trait implementations:** whether they may forward
    `# Errors` (G4-19).
  - **`#[must_use]` on receipts:** with F19 (G2-8).

**Surveys:**
- **[Lints](usability/api-audit/lints/survey.md):**
  - Deny now: `return_self_not_must_use`, `fn_params_excessive_bools` and
    `missing_panics_doc`, the last with one `#[expect]`.
  - Deny `missing_debug_implementations` together with G4-1.
  - Decide `must_use_candidate` as a getter policy.
  - Leave `unreachable_pub` and `struct_excessive_bools` off.
- **[Diagnostics](usability/api-audit/diagnostics/survey.md):**
  - `#[diagnostic::on_unimplemented]` improves every unsatisfied
    pert-ui trait bound, the sealed `StateOwner` included.
  - `#[diagnostic::do_not_recommend]` on `AppError`'s blanket `From`
    fixes F11's message.
  - In the survey's probes, rustc 1.99.1 suggested no import from any
    non-std dependency. Of the arrangements tried, only prelude
    membership or a pointer on the prelude page helped a missing name
    (F3).
  - `compile_fail` does not check error codes on stable. Wording needs
    `trybuild` snapshots in their own job on the pinned toolchain.

**Destinations if not chosen:**
- Verified deviations go to TODO.md.
- Module layout and prelude shape go to app-author slice 9.
- Documentation-only items go to slice 5.

Each row of the record names its destination.

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

Slice 1's checkpoint: Codex found that a filter collapsing trait-impl
methods dropped free functions that follow an impl, that a combined
feature set did not match CI's configurations, and that the
`RUSTC_BOOTSTRAP` choice, the reproducibility inputs and the freshness
check needed stating; the proposal was revised to the tool's own output,
CI's configurations, an extraction-only exception, explicit inputs and a
check that sees new and removed files. A recheck approved it.

Slice 1's implementation: Codex found that unsetting the flag variables
left target-specific and configured flags in effect, and that AGENTS.md's
commands lacked the new gate; the script now sets empty encoded flags, which
take precedence over every other source, and AGENTS.md lists the API
listing gate's commands and names it in the two gate-table rows. A recheck
confirmed both. The workspace gate passed on the same code at bebb411 with
Rust 1.99.1; slice 1 changes no Rust source.

Slice 2: Codex reviewed the round 1 records, including the acceptance of
its own runs, which the record notes. It found warnings counted in the
summaries without their full text, a close refusal in r1-1-claude that an
uncommitted typed change bypasses, an unrecorded reading of moves under a
filter in r1-2-codex and a stale source pointer. The summarizer now keeps
every counted diagnostic; an author probe confirmed the close defect and
r1-1-claude became partly met; a second probe and an explicit reading were
recorded for r1-2-codex, whose verdict stays met; the pointer was fixed. A
recheck found a false README attribution in that reading, which was
removed.

Slice 3: Codex reviewed the triage, including its own runs, and reported
fourteen points: wrong merges (F7, F11, F26), counts (F2, F4, F10, F14), a
misread documentation sentence (F25), discovery findings not routed to
app-author slice 9, verified observations without a destination,
overstatements in the verification section (source availability, the
Codex model) and a missing cascading diagnostic. Its two changes to its
own runs' findings (F4's count and F7's split) were checked by a fresh
Claude agent, which agreed and corrected r1-1-codex's acceptance note about
its in-view close. A recheck confirmed the fixes and found two remaining
routing and wording points in the slice 9 inputs, which were fixed.

Slice 4's checkpoint: Codex's review of the rubric found fourteen points,
among them:
- behavior excluded although several rules state behavior;
- rustdoc JSON overwritten per configuration;
- inventories promising roles that only a reader can classify;
- candidate filters missing the departures they should find;
- callback, choice and permission rules transcribed incompletely;
- guideline codes misapplied;
- a root-only discovery rule;
- surveys without the plan's minimum lint and diagnostics sets;
- an underspecified dispute process.

The rubric was rewritten: rules are labelled as project rule, guideline or
consistency proposal; inputs are frozen per configuration with provenance;
mechanical candidates are kept apart from semantic review; and policy
disputes go to the maintainer. Two rechecks found five remaining points
(cross-family callback names, composite observation timing, C-ITER, T6's
attribution, C3's owner order), which were fixed, and approved it.

Slice 4's audit: Codex's review of the result found nine points. Among
them:
- a refutation in the author's resolutions that missed a runtime captured
  by a parameter sink (`set_visible`);
- a rubric reading settled as though it were a fact (G2-20, now a policy
  question);
- verification corrections left as annotations instead of applied to the
  rows;
- two medium instances missing from the ranking;
- a compiler claim broader than the survey measured;
- two inventory gaps (collapsed concrete serde targets, trait constants),
  closed in inventory version 4, whose additions both auditors reviewed;
- one added instance not yet cross-verified, which Codex then confirmed;
- a duplicate row and an incomplete policy index.

A recheck found three remaining points: an unpropagated correction in
G4-30, the diagnostics survey's universal conclusion, and the policy
index's claimed coverage. They were fixed, and the second recheck
approved.
