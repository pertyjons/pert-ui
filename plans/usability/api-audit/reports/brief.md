# API consistency audit: auditor brief

You are one of two independent auditors of the public API of pert-ui, a Rust
GUI library in <repo>. The other auditor is a different
model working from the same inputs. You audit; you do not fix anything and
you do not delegate the audit.

## Rules

- Read-only. Do not modify any file in the repository. Write nothing except
  your own report, if you have file access, at the path your assignment
  names.
- Independence: do not read any file under
  <audit scratch>/
  other than this brief and your own report, and do not read
  plans/usability/api-audit/reports/ if it exists.
- Verified findings only. A candidate-list entry is a lead. Read the code,
  the rustdoc and the governing rule before recording a finding, and cite
  file:line for what you read. Do not record hunches.
- Work exhaustively within your assigned rubric items: process every entry
  of the candidate lists that feed them and say how many you processed. If
  you must sample, say so and how.

## Inputs

- The rubric: plans/usability/api-audit/rubric.md. Read it first, in full.
  It defines the rule kinds (project rule, guideline, consistency
  proposal), scope, the finding record and every rubric item.
- The frozen inputs: plans/usability/api-audit/inputs/
  - provenance.json: commit d4ba108, toolchain, configurations. The
    library source (crates/) has not changed since that commit; later
    commits touched only plans/.
  - candidates/*.txt: mechanical candidate lists. Each file's header says
    what it holds; a trailing [configs: ...] marks entries that exist only
    in some configurations. Columns are tab-separated; definitions are
    file:line in the repository.
  - semantic/<package>.txt: every public type with its inherent and
    trait-declared methods, with an empty role column. The script assigns
    no roles.
- The committed API listings: api/<package>/<configuration>.txt.
- The governing documents: AGENTS.md (sections "Component API
  conventions" and "Rust and data invariants"), docs/contracts.md (the
  sections the rubric names), and the Rust API Guidelines checklist the
  rubric cites (apply each code with the guideline's own qualifications;
  you know the guidelines, no web access is needed).
- The source and tests under crates/, including each package's tests/.
- Slice 3's findings: plans/developer-usability.md, section
  "#### Round 1 findings" (F1–F34) and the misleading-diagnostics list
  after it. Where the rubric says a slice 3 finding belongs to an item,
  re-examine it under that item rather than copying it.

Packages in scope: pert-ui-core, pert-ui-runtime, pert-ui-cpu,
pert-ui-wgpu, pert-ui (the facade). The facade glob re-exports
pert-ui-runtime (and its prelude), and pert-ui-runtime re-exports items
of pert-ui-core; audit an item once, at its defining package, and name the
configurations where it exists.

## Completeness check

Both auditors rely on one inventory, so a shared omission would go unseen.
While working, check whether the candidate lists your items use miss
anything: a public item the rule applies to that no list contains, or a
list that is wrong (for example a mis-rendered signature, a missed
re-export or a missed conditional impl). Report each as an inventory gap
with the evidence.

## Report format (Markdown)

1. `# <auditor> report: <group>` and one line naming the commit audited.
2. `## Dispositions`: a table with one row per assigned rubric item:
   item | disposition (findings / no findings / not assessable, with the
   reason) | coverage (which lists and how many entries were processed,
   what source was read).
3. `## Findings`: one subsection per finding, `### <ID> <short title>`,
   IDs `<auditor>-<item>-<n>` (for example `codex-B1-3`). Each records:
   - rubric item and kind of rule (project rule / guideline code /
     consistency proposal);
   - item path(s) and the affected configurations (or "all");
   - observation with file:line, and whether it is **signature** or
     **behavior**;
   - the rule it departs from, quoted or cited exactly; if a family
     exception might apply, quote the contract sentence and say why it does
     or does not cover this;
   - severity: **high** (leads a consumer to wrong behavior or data loss,
     or blocks a use), **medium** (costly confusion or a workaround),
     **low** (annoyance or inconsistency);
   - confidence: high / medium / low;
   - suggested fix and whether it is breaking;
   - destination if the maintainer does not choose it: TODO.md (verified
     deviation or concrete improvement), follow-ups (an optional
     capability waiting for an application's need) or app-author slice 9
     (module layout and prelude shape).
   Group many instances of one pattern into one finding with a list of the
   instances, rather than one finding per instance.
4. `## Inventory gaps`: each with evidence, or "none found".
5. `## Roles relied on`: the semantic-list classifications your findings
   or "no findings" dispositions depend on, as path | role | source
   reference, or "unresolved" with the reason.

Be precise and concise. Policy questions (should a rule hold, a new
exception, adopting a consistency proposal) are not yours to settle:
record them as findings labelled as policy questions.
