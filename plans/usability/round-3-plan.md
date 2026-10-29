# Round 3 execution and closure

Status: complete through round 3, 2026-10-29. Blind runs, independent
acceptance, corrected records, in-scope repairs, required available gates and
final Claude review are complete. The review's README condition is discharged
with targeted documentation checks and author adoption. The maintainer authorized completion through
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

In-scope findings are recurring F/R2 findings, new verified library API,
implementation and documentation findings, and kit issues encountered by
this round. Necessary breaking changes are in scope under the unpublished
compatibility policy. The goal delegates routine design choices to the
author; significant design or API changes get a written checkpoint and
fresh Claude review before implementation, with the decision recorded.
Codex owns designs, integration and verification; fresh Claude sessions perform
independent acceptance and initial triage, which Codex checks against evidence.
A fresh Claude reviewer checks the resulting triage and final assessment.
After the blind runs, implementation may be delegated to a separate Claude
author in an isolated checkout under an approved design: that patch receives
fresh Codex review, and the integrated change still receives fresh independent
review. This is a recorded post-run role adjustment, not a change to tester,
acceptance or frozen experiment conditions. Reviewers never author fixes or
delegate; an implementation author never approves its own change.

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
  brief instruction: `pandoc -f html -t gfm-raw_html <page>`. Both kits have the
  identical instruction and the same sandbox `/usr/bin/pandoc`; this produces
  text on demand rather than pre-rendering text files. Record the overlay and
  exact CLI, model, effort, Rust and pandoc versions. No shared build cache.
- Fresh isolated sessions; Claude and Codex have one serial queue each.
  Claude's queue: task 6 head #1, task 6 baseline #1, task 1 head, task 2 head,
  task 6 head #2, task 6 baseline #2, task 3 head, task 4 head, task 5 head.
  Codex's queue reverses head and baseline in both task 6 pairs; tasks 1 to
  5 stay in the same slots. This balances version order between testers.
  Each paired subsequence alternates commits. Queues may run concurrently.
- Claude uses pinned `claude-opus-5-5`, high effort; Codex uses the configured `gpt-6.1-sol`,
  medium effort. Record actual configured/effective identity as the harness
  supports; do not claim an unobserved effective model. Budget 90 minutes
  per run, unchanged. Rerun both CLI isolation probes for the new versions,
  and the no-tester probe on each kit before task runs.
- Apply task 6's sealed notification criterion identically: the UI must
  show a failure notification and inspection must expose that notification's
  existence. A `ToastStack` count increase with its Notifications role establishes
  existence on either commit; do not require message text in inspection,
  which baseline toasts cannot provide. Source inspection plus a capture or
  the component description must establish that it displays the actual
  failure. A visible failure message in another component also
  qualifies if inspection exposes it. Record message discoverability as a
  separate usability observation. Never weaken the requirement that the
  tab remains modified after failure.
- Fresh Claude acceptance contexts apply the same procedure to both commits,
  receiving the task 6 interpretation above alongside acceptance.md,
  adding checks to copies where required, with source and output preserved.
  Independently check acceptance and triage, particularly Codex submissions.
  Keep paired task 6 conclusions apart from head-only comparisons. Use
  summarize.py's existing counting definitions unchanged.
  Count recurrence under round 2's matrix rules. Round 2's task 5 results
  already use the recorded outside-row rule on both commits; no retrospective
  regrading is needed. Earlier-round comparisons are descriptive and CLI
  version changes are a confounder. Round 3 repairs have focused regression
  evidence and independent review, not blind-test evidence from these runs.

## Execution log

- 2026-10-28: clean checkout at 86a196f on main; Claude Code 2.1.312 and
  Codex CLI 0.165.0 installed and authenticated. Task 6 seal confirmed in
  the existing runs README. Both round 2 kit issues are addressed in the
  brief and acceptance checks; no packaged library file changed.
- Preparation review by Claude Opus: not ready; clarified scope, delegated
  design checkpoints, independent roles and the sealed task 6 notification
  interpretation, preserved link targets in the reading command, pinned
  Claude's model and balanced queue order. Task 6 acceptance paragraph and
  reserved files compared with a81449f: unchanged. Recheck found only the
  incorrect error-role wording (toasts have no severity) and requested
  removing task 5's "outside-row" wording and giving acceptance agents this
  interpretation. All three corrected; the reviewer explicitly found the
  rest resolved and ready subject to these corrections and prelaunch checks.

- Prelaunch: head kit at 32dd9c4 (dirty=0, 0.3.88), baseline at 26e6b5a
  (dirty=1, only the overlaid brief differs, 0.3.75). Packaged source paths
  match 86a196f on head. Matched harness hashes, exact Rust/pandoc versions
  and frozen acceptance hash are in [prelaunch.json](runs/review-r3/prelaunch.json).
  Both no-tester isolation probes and the fresh Claude/Codex probes passed;
  logs are in runs/review-r3. Documentation reading succeeded inside each
  sandbox for facade, guide and TextArea pages, preserving link targets;
  neither plans nor reserved directories exists in either kit. Pandoc emits
  harmless duplicate-HTML-identifier warnings on TextArea's rustdoc sections.
  Claude probe reports effective model claude-opus-5-5. Test queues start
  from these immutable kits with the order above.
- First terminal records preserved: r3-6-baseline-codex-1 and
  r3-6-head-claude-1. Neither has independent acceptance yet. Original CPU
  captures were losslessly recompressed for storage; per-run provenance
  records the original/archive hashes and verifies identical decoded IDAT
  streams. Source and test files are unchanged from the submissions.
- Acceptance procedure independently reviewed and rechecked by fresh Claude
  sessions (runs/review-r3). Clarified correct-kit validation, aggregate and
  incomplete grading, original-source protection, required features and the
  frozen task checks, report locations and task 4's temporary mutation proof.
  The recheck's remaining global-configuration concern is resolved with an
  isolated CLAUDE_CONFIG_DIR, empty setting sources and disabled skills. A
  tool-free authenticated check reported no global CLAUDE.md instruction.
  Driver probes refuse nonterminal submissions, wrong kits and escaping
  symlinks before creating acceptance state or starting a verifier. The
  driver checks kit metadata against prelaunch.json, frozen criteria and
  before/after hashes of original source and run logs. It writes reports to
  acceptance copies only; author review precedes copying them into run records.
  Acceptance builds wait until both blind queues finish to avoid adding
  compilation load to the timed experiment. A read-only initial triage of
  the completed head task 6 run is underway; no final finding is claimed yet.

- Original Codex queue finished with all planned submissions. Original Claude
  queue had successful task 6 head/baseline first runs and task 1, then six
  authenticated-start failures. They executed no model work or cargo command
  and are excluded from usability outcomes, not silently discarded. Exact
  conditions and fresh replacement mapping are preserved in
  [auth-start-failures.json](runs/review-r3/auth-start-failures.json).
- Authentication recovered with a retained refreshed tester credential copy,
  verified by a tool-free isolated session; no global credential was replaced.
  [Recovery record](runs/review-r3/auth-recovery.md) declares the minimal
  authentication-only harness change after freeze, its SHA256 and the new
  explicit-copy source metadata. Kits, reading aids, tasks, acceptance, model
  settings and budgets are unchanged. A fresh replacement Claude queue resumes
  the unsuccessful slots in original relative order with `-retry1` names,
  using no earlier failed work or context. It stops on a subsequent session
  failure instead of cascading through the queue.
- Initial independent source/documentation triage and author dispositions
  are in [triage-r3](runs/triage-r3/initial-head-task6.md), with public-API probe
  sources prepared there (execution pending until timed runs finish).
  Proposed item geometry and stable Select/menu part-address checkpoints
  have fresh isolated Claude reviews in progress; neither is implemented or
  called approved yet. Acceptance, full recurrence matrices, repairs, gates
  and final review remain outstanding.

- All planned successful blind sessions are terminal, including the fresh
  Claude replacements. Independent acceptance now runs in separate kit-backed
  copies, two fresh Claude contexts at a time, with original source/log hash
  checks. Its first launcher stopped before any verifier started because the
  historical queue.py record shadowed Python's standard queue module; the
  record is renamed blind-queue.py and the launcher uses python3 -I. No
  acceptance context was replayed or result discarded.
- Frozen-head author probes compiled and all eight passed. Their source and
  full output are in triage-r3/probes. These confirm the reported menu/tab
  geometry and Select lookup gaps, and contradict the hypothesized Dynamic
  timing and tab-address enumeration defects. Geometry revision 2 and
  named-part revision 3 received independent Claude approval; reviews are in
  runs/review-r3. Implementation is underway under the delegated approval:
  part lookup/metadata are independent of workspace paths and semantic names,
  with inspection schema 17 and workspace schema 2. The runtime's first
  package check passes; complete tests, documentation, version/changelog/API
  listing update, semantic review and final gates remain required.

- Both blind queues are terminal with all planned successful submissions,
  including the separately declared authentication replacements. Fresh
  independent acceptance began only afterwards. Five verifier sessions
  completed; these reports still require author review before adoption.
  Two further verifiers ended with the service's `usage_limit_reached` (429),
  reporting a reset at 19:10 Europe/Stockholm. Their copies/transcripts remain
  intact and are not graded from an interrupted verifier; no successful
  verifier or blind submission will be replayed. Reversible implementation and
  regression testing continue while the service limit is active.
- Implementation probes confirmed the approved part-scope boundaries. In
  QuickPick and CommandPalette the existing addressable `surface` remains
  a boundary: find that address first, then query its exposed `search` part,
  then that search's `clipboard`. The picker does not expose `search` directly.
  This clarifies the design's forwarding wording without changing addressing
  or adding a privileged lookup route.

- Geometry/parts regression tests now pass, including scoped built-in/custom
  lookup, metadata-only idleness, field naming, workspace collisions/round trips,
  another window, serialization and real item pointer input with clipping,
  scrolling, dynamic titles and detached-title activation. A pre-existing
  Driver refusal test used Tabs as its no-bounds fixture; it is migrated to a
  custom reporter that deliberately omits rectangles, keeping that refusal
  contract while Tabs gains geometry. The guide's import-contract check caught
  a custom component example in the testing chapter; the example now lives in
  custom-widgets with a link, keeping each chapter's intended imports.
- Native gallery pointer steps pass headlessly and in both selected native
  interaction checks (wgpu and CPU/softbuffer), Fedora 44 GNOME Shell 50.6,
  Wayland. See docs/verification.md for exact limits and platform metadata.
  The selected adapter was not printed; skipped native checks remain skipped.
- A literal U+0009 missing-glyph box is independently observed and author
  reproduced in TextService; source/output are in the tab-rendering probes.
  A separate presentation-policy checkpoint is proposed for Claude review;
  no text repair is implemented yet. Existing frozen acceptance captures stay
  unchanged. Workspace/API/documentation gates for the first repair checkpoint
  are running, with failures and corrected author test assumptions retained.

- The first completed independent acceptance reports have been author reviewed
  and adopted: Claude's paired task-6 submissions and head task 2, all met.
  Added verifier tests, outputs and captures are preserved with each run;
  original source and all pre-existing Rust/Cargo files in the copies have
  identical hashes. This is partial acceptance progress, not an overall result.
- All first-checkpoint workspace commands, API generation and relevant serde
  configurations passed; outputs and prior failed attempts are preserved in
  runs/review-r3/gates. API listings still need their committed-clean check.
  The existing AppError-rustdoc leaf-list TODO and other-platform native gaps
  remain out of this repair's scope.
- At the declared service reset, an isolated tool-free Claude probe returned
  OK. The verifier continuation skips completed sessions, resumes the interrupted
  graders with their own contexts/copies, and starts fresh remaining graders.
  A fresh read-only implementation reviewer is checking the first repairs;
  the separate literal-tab policy remains a proposal awaiting independent review.

- All independent verifiers are terminal, and author reviews 1–5 have adopted
  every successful blind submission as met against unchanged frozen criteria.
  The adoption index and both initial/continued controller outcomes are in
  review-r3. Test-only hooks are preserved with diffs; original submissions
  remain unchanged. The resumed graders are continuations, not blind reruns.
- Initial full head-task triage is preserved as initial-head-tasks1-5.md.
  It started before Codex acceptance finished, so its pending-acceptance status
  is historical; the final matrices must distinguish later acceptance exercise
  from timed tester exercise. Report citation errors and the owned
  ListDropPosition correction are in the author reviews.
- Independent implementation review rejected an already-selected-tab native
  step and idle tests whose baseline could absorb invalidation. Both were
  repaired, with intentional negative controls proving the repaired checks
  fail on those regressions and byte-restoring the source. The corrected
  native scenario passes GPU and CPU; focused post-review outputs are in
  gates/review-recheck. Fresh implementation re-review is underway.
- The first literal-tab proposal was rejected because substituting spaces
  changes Unicode/grapheme/bidi behavior. Revision 2 preserves original text
  and tags only tab ink privately; fresh review is underway before any
  implementation. Disabled Button appearance has a separate proposed
  presentation checkpoint, based on source and actual acceptance captures.

- First library checkpoint committed as 3bbfb813a8c02c417d68386b0c5547b4b8139580
  (0.3.89), after independent implementation approval and green workspace/serde,
  examples, documentation, diagnostics and API generation. The committed API
  listing status is clean. Exact later doc-only differences and supplemental
  checks are recorded with the final-0.3.89 gate evidence.
- Disabled Button's approved design is implemented and reviewed on working
  0.3.90. Initial review approved code/tests and requested status/changelog
  corrections; optional test/dependency gaps were also tightened. Final
  workspace checks and a fresh focused recheck are running on that source.
- Post-run implementation-role adjustment: Claude implements only the approved
  text coverage/fixture/editor tests in a detached checkout at 3bbfb813, with
  explicit file scope and no version/dependency/publication authority. Codex
  remains the integration author; a fresh Codex reviewer must inspect the
  Claude patch before adoption. The final integrated implementation and overall
  assessment retain independent review. No original blind source is modified.
- Fresh frozen-kit probes P7–P11 execute on both commits. Source, output and
  captures are preserved in triage-r3/followup-probes, with author dispositions.
  They verify stale-pointer menu keyboard selection and changing historical
  reveal diagnostics, plus documented row/tab focus composition behavior.
  A separate menu-keyboard/diagnostics checkpoint awaits design approval.
  TextArea tab-box evidence now covers baseline as well as head.

- 2026-10-29 continuation: disabled Button checkpoint committed as b9e4c6c
  (0.3.90). Both complete workspace runs passed, final API generation is clean,
  final source hashes correlate the executed code, and independent records
  review approved after the earlier pending-evidence verdict. No overall
  usability closure follows.
- Fresh Codex review of the isolated text patch requires explicit public
  coordinate/extent evidence, mixed-font/icon cases and qualified rustdoc.
  Revision 4's natural visible-ink canvas policy is independently approved
  with exact LTR/RTL, hit-test, phase, editor and negative-control requirements;
  Claude is repairing its scoped patch before independent re-review. A
  suspected whitespace-wide hanging-caret limitation must be reproduced before
  it is recorded as a verified separate finding.
- P12–P14 are archived under triage-r3/extra-input-probes, with exact paired
  observations and author dispositions. MenuTitle's parked-pointer switch and
  MultiSelect's blocked keyboard highlight are verified; MultiSelect is
  recorded in TODO.md as a separate row-family checkpoint. A legitimate wrong
  Hidden-refusal fallback was not established. Host-hidden reveal's missing
  operational-error documentation is verified and joins the diagnostics repair.
- Menu ownership revision 3 explicitly adopts review corrections: per-title
  window-space position memory, no reset of remembered coordinates, distinct
  ownership resets, keyboard-armed pointer protection and observable/deferred
  tab-refocus guards. Fresh design recheck precedes implementation. Existing
  unrelated TODO and missing native-platform evidence remain open.

- 2026-10-29: historical diagnostics committed d479516 (0.3.91), after all eleven
  workspace commands/API and independent conditional review. Missing
  resume_content_jump docs were repaired with targeted doctests/strict docs;
  the raw verdict and author condition discharge remain distinct. The extra
  source guard's TODO item-name correction is explicitly recorded, with every
  compiled source byte unchanged.
- Focus-continuity recipes committed 17c6c3d, fresh Codex approved, integrated
  public tests/doctests/Clippy/strict docs passed. The consumer-facing command
  action may-flush sentence and VirtualList recipe link resolve the remaining
  documentation sub-points without changing runtime policy.
- Menu and tab patches received fresh Codex approval after required test/doc
  repairs, then were integrated as working0.3.92. All eleven workspace commands,
  API generation and selected Linux GPU/CPU native interaction checks passed;
  restored old gates reject the actual new native scenario assertions. Fresh
  integrated review approves code; API/record conditions are discharged in its
  evidence README. No repaired-head blind run is claimed.
- Recurrence matrices/metrics/new-finding ledger received independent records
  approval after stricter empty-label, actual-example and must-use exercise
  corrections. Acceptance-only exercise remains separate from timed P cells.
- P13 and a new current-source long-list click-focus probe feed MultiSelect's
  approved separate repair. Empty-raster canvas and hanging-caret findings are
  actively assigned a separate whitespace checkpoint within this goal, with
  temporary TODO entries until fixed. The first proposed sideways-reveal policy
  was rejected for prose jiggle; revision3 pins displayed wrapped carets while
  retaining canonical TextSelection/hit geometry. No new scrolling behavior is
  claimed before implementation and review.

- 2026-10-29 continuation: approved MultiSelect source/tests integrated. The
  root targeted test run passes; a new gallery scenario reproduces the separate
  short-popup focus escape after its long-list continuation step succeeds.
  The generic popup-focus revision2 and whitespace revision3 designs have
  independent review with all corrections adopted before their isolated Claude
  implementations. Both require fresh Codex review and integrated verification.
  The [assessment draft](round-3-assessment.md) explicitly retains those gates
  and final independent Claude review; it does not close the goal prematurely.

- MultiSelect and generic surface press focus committed6b8b3bf (0.3.93) after
  full workspace/API and selected GPU/CPU native success. Independent integrated
  review1 required intermediate Escape-clear assertions; root strengthened them,
  reran focused/native checks and received review2 approval. Native overflow is
  not asserted at its window size; outside-ScrollArea evidence is headless.
- Whitespace production logic passed independent review. Root adopted measured
  fractional rounding, transparent phase-raster and mask-edge sampling limits in
  revision4, repaired prose/test labels and received final isolated review3
  approval. Source baseline hashes matched before root integration. Working0.3.94
  now runs the full workspace/API and affected native checks; overall closure
  remains pending those results and independent final assessment review.

- Working 0.3.94 full retry gate passes all eleven commands after the initial
  API-generation/test overlap failure. API generation is clean; selected native
  text probe and GPU/CPU gallery interactions pass with platform/source limits.
  Raw failures, exact archives and post-gate Markdown-only differences remain
  explicit. Final integrated Claude review and committed checks precede closure.

- Text checkpoint committed ddfdb20 (0.3.94); API status is clean after commit.
  Final Claude review attempt1 exits at the five-hour service limit, before a
  completed review. Its result/prompt/status are preserved. Reported availability
  reset is 15:00 Europe/Stockholm (14:00 UTC), 2026-10-29. No final approval or
  goal completion is claimed; the complete source and records await that gate.

- After the service reset, fresh final Claude review2 accepts the integration,
  records and bounded closure after correcting a stale runtime crate-README
  sentence. The correction passes strict runtime rustdoc, runtime doctests and
  whitespace checks. The reviewer explicitly requires no new full gate or
  review round. Author adoption also corrects stale statuses and the native/
  retry timing wording, and records the verified outside-scope native GPU-summary
  reporting defect in TODO.md. The final local record commit and clean API/tree
  check close the authorized goal; remaining limits stay explicit in the
  [assessment](round-3-assessment.md).
