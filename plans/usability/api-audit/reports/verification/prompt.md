# API consistency audit: cross-verification

You are the auditor "OWN" in the API consistency audit of pert-ui, a Rust
GUI library in <repo>. You audited rubric group GROUP
independently; so did the other auditor, "OTHER", a different model.
Both reports are now submitted. Your task is the rubric's
cross-verification step for that group (plans/usability/api-audit/rubric.md,
"Cross-verification and disputes").

Read-only: do not modify the repository; do not author fixes.

## Inputs

- The rubric: plans/usability/api-audit/rubric.md.
- Your report: plans/usability/api-audit/reports/OWN-GROUP.md.
- The other report: plans/usability/api-audit/reports/OTHER-GROUP.md.
- Your assignment: plans/usability/api-audit/reports/assignment-GROUP.md.
- The inventory, version 2: plans/usability/api-audit/inputs/ and its
  CHANGES.md, which lists what version 2 adds to version 1 (version 1 is
  commit 6e39e18; `git show 6e39e18:<path>` reads it).
- The source at HEAD; the library source (crates/) is unchanged since
  d4ba108, the audited commit.

## Tasks

1. **The other auditor's findings.** For every finding in the other report
   (every `### <ID>` that is a finding, not an inventory gap):
   - **Match:** does your report contain the same departure (same rubric
     item, same items, same rule)? If so, name your finding ID. A partial
     overlap counts as a match only for the overlapping instances; name
     the rest as unmatched.
   - **Verify** every unmatched finding or unmatched part against the
     source yourself: read the cited code and the rule, and check the
     claim. Verdict: **confirmed**, **partly confirmed** (say exactly
     which instances or claims hold and which do not), or **refuted**,
     with file:line evidence. Also say whether its severity, its rule
     kind (project rule / guideline / proposal / policy question) and its
     suggested fix are right; disagree with evidence where they are not.
   - Do not take the other report's citations on trust.
2. **Your own findings disputed.** If the other report contradicts one of
   your findings or dispositions (for example it records "no findings"
   where you found one, or states a fact that conflicts with yours),
   re-check yours against the source and say whether you maintain,
   correct or withdraw it, with evidence.
3. **The inventory delta.** Review the entries version 2 adds for your
   group's rubric items (CHANGES.md says which lists). Report any new
   finding they reveal, in your report's finding format (IDs
   `OWN-<item>-d<n>`), or "none", with how many entries you processed.

## Output

Markdown:
1. `# OWN verification: GROUP`
2. `## The other auditor's findings`: a table with one row per finding:
   other ID | matched by (your ID, or "none", or "partly: ...") | verdict
   (confirmed / partly confirmed / refuted / matched) | evidence
   (file:line and a sentence) | remarks on severity, kind or fix.
3. `## My findings disputed`: one entry per dispute, or "none".
4. `## Inventory delta`: new findings in full, or "none", with coverage.
Be precise and concise.
