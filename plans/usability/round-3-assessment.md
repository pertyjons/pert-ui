# Round 3 assessment

Proposed closure, 2026-10-29. Experiment records and isolated repairs are
independently approved. All required available repair gates pass; fresh Claude
review of the final integration and this assessment remains pending. The
persistent goal is not marked complete before that review and final record adoption.

## Experimental result

Every completed blind submission met the unchanged frozen acceptance criteria:
head-only tasks 1–5 and the paired head/baseline notes-editor task. The
[adoption index](runs/review-r3/adopted-acceptance.json) and per-run acceptance
copies retain the independent checks and author decisions. Original submissions,
reserved task 6 and its sealed acceptance remain unchanged. Authenticated starts
that performed no model work or build are preserved as infrastructure failures;
they are not counted as failed usability sessions. Interrupted graders resumed
their own contexts rather than rerunning successful blind sessions.

The [metrics](runs/triage-r3/metrics.md) compare frozen 0.3.88 with 0.3.75,
using the same isolated harness and the recorded order. Within-tester notes-task
median session time is 8.05 versus 12.25 minutes for Claude and 5.50 versus
6.65 for Codex. These are descriptive observations from a small AI sample,
with model strategies, learning and timing noise. They do not establish human
usability, a causal speed gain, or the speed of the subsequent repairs.

The [recurrence matrices](runs/triage-r3/recurrence-matrices.md) distinguish
actual timed exercise, recurrence, source-inferred baseline recurrence and
unreached cases. F3 import discovery recurred on the frozen head; absence of
other recurrence is bounded by what each task actually exercised. Later
acceptance checks do not turn unreached blind cases into successful timed
exercise. The [independent records review](runs/triage-r3/independent-records-review-r2.md)
approves the corrected records, separately from implementation or closure.

## Resulting library work

The [repair ledger](runs/triage-r3/new-findings.md) links every verified finding
and disposition back to the initial reports and author reproduction. The first
completed checkpoints add public composite-part discovery and actual item
geometry, improve initializer/mailbox/table/testing documentation, distinguish
disabled Button presentation, stabilize retained refusal diagnostics, document
host-hidden operational errors, demonstrate public list/tab focus continuity,
repair stationary-pointer menu navigation, and remove literal tab-only ink
without changing canonical text or Unicode analysis. Each checkpoint retains
its independent verdict, author repairs and relevant gate evidence.

MultiSelect highlight/click continuity and the generic press-focus walk now
have independently approved isolated implementations and a green workspace/API
checkpoint. Root strengthened an Escape assertion after integrated review, reran the
affected headless/native checks and obtained final integrated approval before
committing 6b8b3bf (0.3.93). The zero-area-raster/wrapped-caret patch is
independently approved after prose corrections and committed as ddfdb20 (0.3.94);
all eleven workspace commands, API generation and selected native checks pass.
The initial missing-API-file failure came from root overlapping generation with
API-reading tests; the complete successful retry waits for generation. Both
attempts and the source-correlation limits are preserved in the
[integration evidence](runs/review-r3/whitespace/integrated/README.md). No passing frozen
submission substitutes for fixing these runtime defects. None of the post-run
repairs has a repaired-head blind timing result in this experiment.

The text policy explicitly retains bounded outward physical-pixel rounding,
positive-area transparent phase rasters and changed fractional-edge sampling
where mask extents change. It pins displayed wrapped carets, with raw selection,
preedit and hit geometry unchanged. These measured qualifications are in
[design revision4](round-3-design-hanging-whitespace.md) and the independent
review/repair records; no universal pixel-identity or fixed-offset claim follows.

## Limits and disposition

Native execution is recorded per affected checkpoint on the available Linux
Wayland desktop, for selected GPU and CPU/softbuffer checks. Runtime-injected
input does not prove OS event translation, human interaction or Windows/macOS
behavior. A selected check does not imply skipped native checks passed. Tab
coverage evidence includes real shaping and CPU pixels; native interaction
success alone is not a GPU tab-pixel comparison.

The focused author release-profile text tests pass with the pre-existing
Measurements::is_empty cfg-related unused-code warning, explicitly recorded in
TODO.md. Strict public rustdoc and required dev-profile Clippy are separate
checks; their success does not claim every exploratory/release build is warning-free.

Existing unrelated [TODO findings](../../TODO.md), optional component-family
extensions, a full app-author tutorial, unavailable platform evidence and public
publication remain outside the round's agreed scope. Newly verified unrelated
findings are recorded there rather than silently added to a repair. No publish,
push or merge follows from completing this local usability work.

The proposed decision is to complete round 3 after independent final approval
and the committed-tree/API check. Every verified in-scope finding has a repair
or an evidence-based disposition in the ledger; optional/unrelated work remains
explicit rather than becoming an unbounded new round. The approved plan does
not require another blind experiment on the repaired head.

Historical pending/request-changes status lines in individual run/checkpoint
records describe their earlier review stages. This assessment and the execution
plan carry the current closure decision; the original evidence is preserved.


The first final-review invocation stopped at Claude's five-hour service limit
without completing a review. [The preserved attempt](runs/review-r3/final-review-attempt1/README.md)
is infrastructure evidence, not approval or a source rejection. Reported reset
is 15:00 Europe/Stockholm (14:00 UTC), 2026-10-29. The code checkpoint and its
committed API check are complete; a fresh final Claude review is the remaining
closure gate.
