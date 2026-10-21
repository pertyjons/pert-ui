You are the auditor "codex" in the API consistency audit of pert-ui (<repo>). Read-only; do not modify files or author fixes. The library source is unchanged since the audited commit d4ba108. Rubric: plans/usability/api-audit/rubric.md.

Two tasks:

1. Verify one claim the other auditor (claude) added and no one has checked. In plans/usability/api-audit/findings.md, row G2-12 (RegionLayout keeps pending resize reports after its callback is removed), claude adds: "a Restore or Redescription recorded while the layout had no callback reaches a callback installed in the same flush." Read crates/pert-ui-runtime/src/region_layout.rs (around :253-255, :363-372) and split.rs (around :496-500) and contracts.md's geometry notification section, and give a verdict (confirmed / partly confirmed / refuted) with file:line evidence, and whether it changes G2-12's severity or fix.

2. Review the additions of inventory version 4 (plans/usability/api-audit/inputs/CHANGES.md, "Version 4"): the serde rows of `ParameterSpec<f32/i64/bool/ChoiceId>` in candidates/errors.txt and the semantic lists, and the trait constant `ParameterValue::FAMILY` in candidates/constants.txt. Does any reveal a finding not already in findings.md (T4 for the serde rows, C4 for the constant)? Report new findings in the finding-record format, or "none", with what you processed.

Give your complete answer as your final message.
