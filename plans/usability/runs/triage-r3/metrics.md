# Round 3 descriptive metrics

Status: author draft; independent final recheck pending. Extracted from the
retained run summaries, with hashes in [metrics.json](metrics.json). The
summarize.py definitions are unchanged. All listed submissions met the frozen
acceptance after independent checking and author adoption; these times are
those of the timed tester session, not verifier work or post-run repairs.

Task 4's App/test lines count additions/changes against the supplied fixture;
other tasks count all application/test source under the script's rules. Task
4's candidate type inventory still includes the supplied application as the
existing summary defines it. These are not interchangeable code-size totals.

Build failures count attempts, not distinct library defects. Code lines and
candidate type counts use the frozen script's lexical definitions; type
candidates are not a resolved public API count. First build is a timing marker,
not app completion. The summaries retain candidate first-success commands;
this report does not promote a smoke test to full task coverage. No measurement
here is a human usability score or an estimate for application authors generally.

## Head-only tasks 1–5

| Run | Session min | First build min | Builds (failed) | App/test lines | Candidate types |
| --- | --- | --- | --- | --- | --- |
| [r3-1-head-claude](../r3-1-head-claude/summary.md) | 7.6 | 0.8 | 20 (5) | 307/404 | 32 |
| [r3-1-head-codex](../r3-1-head-codex/summary.md) | 4.1 | 1.5 | 5 (2) | 286/196 | 20 |
| [r3-2-head-claude-retry1](../r3-2-head-claude-retry1/summary.md) | 6.3 | 2.0 | 11 (1) | 252/210 | 30 |
| [r3-2-head-codex](../r3-2-head-codex/summary.md) | 3.1 | 1.2 | 3 (1) | 223/146 | 26 |
| [r3-3-head-claude-retry1](../r3-3-head-claude-retry1/summary.md) | 5.7 | 1.2 | 15 (6) | 358/264 | 45 |
| [r3-3-head-codex](../r3-3-head-codex/summary.md) | 2.6 | 1.1 | 3 (1) | 259/152 | 36 |
| [r3-4-head-claude-retry1](../r3-4-head-claude-retry1/summary.md) | 2.7 | 0.5 | 6 (2) | 0/280 | 22 |
| [r3-4-head-codex](../r3-4-head-codex/summary.md) | 2.4 | 1.0 | 3 (1) | 0/216 | 21 |
| [r3-5-head-claude-retry1](../r3-5-head-claude-retry1/summary.md) | 6.6 | 1.5 | 14 (1) | 509/277 | 35 |
| [r3-5-head-codex](../r3-5-head-codex/summary.md) | 2.8 | 1.1 | 3 (2) | 314/153 | 29 |

## Paired notes task

| Run | Session min | First build min | Builds (failed) | App/test lines | Candidate types |
| --- | --- | --- | --- | --- | --- |
| [r3-6-head-claude-1](../r3-6-head-claude-1/summary.md) | 8.7 | 3.6 | 24 (14) | 404/337 | 43 |
| [r3-6-head-claude-2-retry1](../r3-6-head-claude-2-retry1/summary.md) | 7.4 | 1.7 | 19 (6) | 395/318 | 46 |
| [r3-6-baseline-claude-1](../r3-6-baseline-claude-1/summary.md) | 11.6 | 3.0 | 20 (11) | 432/340 | 44 |
| [r3-6-baseline-claude-2-retry1](../r3-6-baseline-claude-2-retry1/summary.md) | 12.9 | 2.1 | 41 (15) | 361/344 | 42 |
| [r3-6-head-codex-1](../r3-6-head-codex-1/summary.md) | 6.5 | 1.8 | 7 (3) | 378/260 | 38 |
| [r3-6-head-codex-2](../r3-6-head-codex-2/summary.md) | 4.5 | 1.8 | 6 (3) | 409/201 | 35 |
| [r3-6-baseline-codex-1](../r3-6-baseline-codex-1/summary.md) | 5.7 | 1.0 | 7 (3) | 397/240 | 37 |
| [r3-6-baseline-codex-2](../r3-6-baseline-codex-2/summary.md) | 7.6 | 2.0 | 11 (7) | 394/261 | 38 |

Within each tester, the two-session medians below describe only the frozen
head/baseline pair. Order was balanced between testers, with each paired
subsequence alternating commits; it does not remove model-specific strategies,
learning, noise or the tiny sample. Do not compare different testers as if they
were interchangeable human participants.

| Tester | Head median session min | Baseline median session min |
| --- | --- | --- |
| Claude | 8.05 | 12.25 |
| Codex | 5.50 | 6.65 |

These runs compare 0.3.88 with 0.3.75. No timing or code-size improvement is
attributed to the subsequent 0.3.89+ repairs, which were not blind-tested here.
Earlier-round CLI/version changes are additional confounders; no causal
cross-round speed claim follows. Zero-model/tool authentication starts are
infrastructure failures and are separately preserved, not inserted as failed
usability sessions or silently dropped from the execution history.

[Independent records recheck](independent-records-review-r2.md) approves this
record after the stated author corrections; it does not approve runtime fixes
or overall round closure.
