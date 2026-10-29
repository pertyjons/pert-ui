# Author disposition of initial group-H triage

2026-10-28, Codex. Read the fresh-Claude
[initial report](initial-head-tasks1-5.md), actual source/doc paths, probes,
diagnostics and completed independent acceptance. The initial report is retained
unchanged, including its historical pending-Codex-acceptance status. All head
acceptances have since passed author review. No repaired source is attributed
to a frozen blind run.

## Findings and repair scope

| Initial group-H finding | Author disposition |
| --- | --- |
| F3 component ApplicationError omission | Verified against the module's explicit authoring contract. Existing error now exported through component, with component-only callback failure and membership tests. Viewport, live mailbox and drawing types keep their intentional modules; guide/import pointers repair the discovery symptom without widening the prelude. |
| N2 testing import wording | Verified. The module's paint-output exception was clearer than the guide introduction. Guide now names component imports for DisplayList/DrawCommand. The custom component example belongs in custom-widgets and is linked from testing. |
| N3 Select discovery | Verified and reproduced on frozen head. Scoped control/options parts with public pointer examples repair it without exposing private keys or changing workspace paths/naming. |
| N4 disabled Button | Verified source and actual clean/dirty captures show identical fills with only text changing. Separate disabled-button design checkpoint proposed; no implementation or verification claim yet. |
| N5 named App initializer | Verified doc gap across several runs. App::new now demonstrates the RuntimeError builder wrapper with a complete headless doctest. No generic initializer error API is needed. |
| N6 message error conversion | From<String> and From<&str> already exist. new's rustdoc now points to them. Decline a duplicate msg constructor or From<&str> for RuntimeError. |
| N7 empty editor value | Verified in input/inspection tests. Guide now states Some(Text(String::new())) and the protected exception. Add the same precision to Semantics::value and TextField's inspection section; avoid a misleading absence rule. |
| N8 mailbox recipe | Verified guide gap, while Mailbox's own rustdoc already had a short example. New full guide recipe keeps the receiver alive, handler over independent UI signals, sender in the starter, and explicit Full retry / Closed stop without a model ownership cycle. |
| N9 Table set_source | Existing event-phase writes work; no new callback timing. Rustdoc explains cell callbacks and next-update observation, per-mounted-row work and BorrowConflict context. The guide distinguishes whole-source replacement from bound per-cell content. |
| N10 command routing pointer | Verified register_command lacked the link although CommandScope and VirtualList already documented precedence. Added direct routing/scope link and focused-control precedence, with runtime doctest and strict documentation checks. |

## Contradicted leads and optional requests

- ScrollAxes::Both does not remove the width bound. The source gives viewport
  width for Both and Vertical. The ScrollArea contract explicitly overlays
  scrollbars above content. Actual settings captures show the right border
  covered only when the vertical lane appears. This is documented overlay
  behavior, not a demonstrated unbounded-width defect; no layout rewrite or
  new gutter API follows from the report. The suggested PR-1 experiment is
  unnecessary to establish those source/documentation facts.
- ListDropPosition owns its key and implements Clone. ListDrop::position borrows
  that enum; ListDrop's event itself is not an owned command constructor.
  Factor shared reorder logic around keys plus &ListDropPosition when needed.
  The task-2 acceptance report's repeated contrary claim is corrected in
  author-acceptance-2.md without altering the original report.
- Runtime::find's Option and Driver::find's test error serve different callers.
  Existing ok_or(ApplicationError::from(...)) is sufficient. No extra find API.
- Frameless Button row content already supplies pointer opening; list
  on_activate remains Enter-only. Keep cancellation and notification timing.
- Padding already belongs to Container::padding; decline a duplicate Row/Column
  builder. The f32/f64, typed-normalizer and choice Eq wishes get concise
  pointers/examples in the first checkpoint; they do not establish missing
  constructors or change core geometry types.
- One-line semantic-text helpers and capture wrappers are optional convenience
  requests. Existing documented composition is sufficient; no new test API
  solely for spelling. The existing List rows section names address_children;
  an extra intro pointer is optional.
- Dead placeholder row keys corroborate the existing TODO about ListItems /
  TreeItems output construction; not a recurrence of Each's addressed rows.
  Toast overlap and lifetime are documented presentation policy.
- The SegmentedControl item-bounds limitation remains in the existing
  item-reporters TODO. The current geometry checkpoint deliberately covers
  Tabs and the shared OptionList, not every control family.
- Source-only guesses about every-fifth-job failure and lexical name sorting
  are not application acceptance failures. The sealed task requires one job in
  five failing at a random point, and sorting a name text column, without
  specifying random failure selection or natural-number sorting. Independent
  checks actually exercise those paths; author-acceptance-5.md records the
  interpretation. A determinate bar on a failed row is an application choice,
  not a terminal ProgressStatus exercise.

## Recurrence evidence precision for the final merge

Use the initial report's per-run F/R2 tables as a draft, with these corrections
and labels. Do not infer passing usability from all tasks meeting acceptance.

- Codex acceptance is now complete. Its additional checks are separate
  acceptance exercise (A), not evidence that the timed tester reached them.
- X1/F4: the independent item5 check submits a focused unconfirmed valid draft
  through admit_close and observes KeepOpen; original blind tests had not
  established that case. X1/F6 independent field-message inspection confirms
  the rejection callback's published Field status without reading the model.
- X3/F12: independent v8_focus_loss_cancels_drag now executes the actual
  cancel_input path; the blind tester's Escape-only case did not.
- C2/F30: the boxed conversion helper compiles but was not executed. Mark
  original blind exercise as not reached, not P, under round 3's strict rule.
- F1 registry consumption is supported by actual kit-backed builds/tests,
  not only the source manifest declaration. Task-4's supplied manifest does
  not count as an independently authored consumption route.
- F11 remains limited to a relevant conversion diagnostic. The named App
  builder signature mismatch is N5 and not evidence of F11 passing.
- Do not promote X5 to R2-26: its failed/cancelled rows use a determinate
  ProgressStatus, so terminal-status fraction behavior was not exercised.
- Initial source line numbers are navigational aids and some acceptance reports
  overrun their source. Author correction records name actual items/tests;
  executed test output and retained source determine the verdict.

The final combined recurrence report and fresh-Claude recheck are still owed.
No matrix total, repaired-head blind improvement or whole-round closure is
claimed by this author note.
