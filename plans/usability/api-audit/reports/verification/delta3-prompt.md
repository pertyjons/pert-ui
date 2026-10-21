# API consistency audit: inventory version 3 delta review

You are the auditor "OWN" in the API consistency audit of pert-ui, a Rust
GUI library in <repo>. Read-only: do not modify the
repository; do not author fixes.

Both auditors audited from inventory version 1 and cross-verified with
version 2. The cross-verification found further inventory gaps, closed in
version 3 (plans/usability/api-audit/inputs/, see CHANGES.md, "Version 3
adds"). Your task: review exactly what version 3 adds, so that no finding
hides in those gaps. The library source is unchanged since d4ba108.

Read first: the rubric (plans/usability/api-audit/rubric.md) items C4, N3,
T4 and D2, and both auditors' reports and verifications for the groups
those items belong to (plans/usability/api-audit/reports/{codex,claude}-G1,
-G3, -G4, -G5 and reports/verification/), so you do not repeat findings
already recorded.

Review:
1. **C4:** every row of candidates/constants.txt — are documented limits
   associated MIN_*/MAX_* constants of the type they bound; is any
   constant a limit owned by the wrong type, or a limit that is not
   enforced as documented?
2. **N3:** every `Option<number>` row of candidates/numeric.txt — does a
   value with a unit say so through its type or name?
3. **T4:** the 8 fallible foreign-type conversions in candidates/errors.txt
   (`TryFrom<...> for f32/i64/bool` and the like) — error type, Display
   text, `# Errors` documentation.
4. **D2:** the 6 types now marked "prelude: yes (via ...)" — does any
   earlier D2 finding or disposition change?

For each, read the source before recording anything. Output Markdown:
`# OWN delta review: inventory version 3`, then per item: entries
processed, new findings in the report's finding format (IDs
`OWN-<item>-v3-<n>`; each with rule kind, items and configurations,
observation with file:line and signature/behavior, rule, severity,
confidence, fix and whether breaking, destination), findings already
recorded (by ID) that these entries extend, or "none".
