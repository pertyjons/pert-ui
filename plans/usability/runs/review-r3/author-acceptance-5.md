# Author review: Codex's background job monitor

2026-10-28, Codex. r3-5-head-codex: **met** under the frozen procedure.
Read independent report, added tests and actual final passing output; viewed
CPU capture and recomputed original hashes. All existing Rust files and
Cargo.toml in the acceptance copy are unchanged. No test access hook or
production repair was added. Adoption also verifies original run-log hashes.

The six independent tests supplement the tester's model-based waits with
inspection-only waits and observations. They exercise real Start/Cancel and
header pointer input, intermediate progress and completion, wall-clock thread
termination, row plus outside-row failure notification, every sortable column
in both directions, and CPU capture. A running-job positive control precedes
idle checks; after workers end, inspection and profiling show no update/layout/
paint/compose work. The full suite and repeated acceptance runs pass.

Two interpretations are explicit, not hidden as library failures. The task
requires name sorting but specifies no natural-numeric comparator: Job 1,
Job 10, Job 11, Job 2 is valid lexical text-column order. It requires one in
five jobs to fail at a random point; every fifth job with a random failure
point meets that wording, even though a random 20% choice would also meet it.
Both paths are exercised with production plans as well as controlled plans.
The persistent notification Label is outside the row and qualifies under the
pre-existing task-5 rule. It is not described as a Toast or Banner.

The original tester's expected sort order uses its own comparator and does not
alone prove correctness; the verifier instead checks monotonic displayed cell
values from inspection, including two-digit names. Its summary-count mistake
in an early test draft was corrected only in verifier source and retained in
the evidence. The task does not require terminal ProgressStatus colors; a bar
left determinate on a failed row is an application choice, not evidence that
R2-26's terminal-status contract was exercised.

Limits: Linux /proc thread-count evidence requires serial tests. The idle
report is headless runtime evidence with wall-time worker waits; it is not a
native event-loop measurement. No native execution or human-usability claim.
