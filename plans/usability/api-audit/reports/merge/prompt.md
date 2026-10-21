# API consistency audit: merge sheet for group GROUP

You prepare the merge sheet for rubric group GROUP of the API consistency
audit of pert-ui (<repo>). Read-only: do not modify the
repository. The author settles disputed facts afterwards; your sheet makes
that possible.

Inputs, all under plans/usability/api-audit/:
- rubric.md ("Finding record", "Cross-verification and disputes");
- reports/codex-GROUP.md and reports/claude-GROUP.md (the two independent
  reports);
- reports/verification/codex-GROUP.md (Codex verifying Claude's findings)
  and reports/verification/claude-GROUP.md (Claude verifying Codex's), each
  with "My findings disputed" and "Inventory delta" sections;
- plans/developer-usability.md, "#### Round 1 findings" (F1–F34), for
  cross-references.

Produce one merged finding per distinct departure:
- **Merge** findings that both auditors report (or that verification
  matched) into one entry listing every source ID.
- **Status:** `both` (reported by both), `confirmed` (one reported, the
  other confirmed), `partly` (keep only the confirmed instances and list the
  refuted ones), `withdrawn` (its author withdrew it in verification: list
  it in a separate short section, not in the table), `refuted`
  (contradicted and not withdrawn: goes to "Disputed facts"), `policy`
  (a policy question for the maintainer, in either report or arising in
  verification), `delta-unverified` (a finding from an "Inventory delta"
  section, not yet checked by the other auditor).
- **Corrections in verification** (lowered severity, changed rule kind,
  narrowed instances, corrected facts) are applied; note them.
- **Severity:** if the auditors disagree after verification, give both and
  propose one with a one-line reason.

Output Markdown:
1. `# Merge sheet: GROUP`
2. `## Findings`: a table with columns
   `# | Rubric item, rule kind | Finding (one or two sentences, naming the items) | Sig/beh | Configs | Severity | Confidence | Fix (breaking?) | Destination | Sources and status | Slice 3`
   numbered GROUP-1, GROUP-2, … ("Slice 3" names an F-number it extends,
   or "-"). Policy questions are rows with status `policy` and state the
   question.
3. `## Disputed facts`: for each fact still disputed after verification:
   the claim, each side's evidence with file:line, and what in the source
   would settle it. Read the source yourself and say which side the code
   supports, with file:line; the author decides.
4. `## Withdrawn and refuted parts`: one line each, with the reason.
Be faithful to the reports; do not add findings of your own.
