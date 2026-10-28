# Author verification and provisional dispositions

2026-10-28. The [initial fresh-Claude triage](initial-head-task6.md) is
source/documentation evidence against the exact kits, not independent
acceptance or execution of its proposed probes. Further runs and acceptance
may refine the grouping. No lead is dropped merely because a tester's own
checks pass.

- **Item geometry (N2) and click_item clarity (N1):** confirmed from
  head tabs.rs::Tabs::inspect and select.rs::OptionList::inspect: neither
  supplies local_bounds. The retained head test reports ItemWithoutBounds.
  Driver's rustdoc explicitly limits its present built-in support, so
  "docs wrong" overstates it. Resolve the missing pointer-test capability
  for tab titles and menu rows, with shared Select-list geometry, under
  round-3-design-items.md. This pulls in the relevant portions of TODO.md's
  existing item-reporters entry; unrelated reporter families stay tracked.
- **Inspection/address documentation (N3):** confirmed missing on Tabs,
  MenuBar and Dialog. Document their actual semantics and address routes,
  including the AlertDialog composition where verified by a probe. Preserve
  addressing and selection ownership; add public examples and links from
  the testing guide. The report's P1/P4/P5 probes remain pending.
- **Dynamic tab navigation (N5):** source shows reveal_child_selected calls
  flush before key lookup. The tester's inference that a prior Dynamic
  write cannot be observed is not established. Run P2 against the frozen
  head before deciding the wording and recipe; do not change timing or add
  a controller to satisfy that guess. The full app-author tutorial remains
  outside this round; a focused tab lifecycle recipe is within scope.
- **List pointer activation (N4):** the Enter-only activation contract is
  documented, and a Button row already provides pointer opening through
  the public API on both commits. No runtime defect or inaccessible
  capability is established. Clarify that composition with a small pointer
  to the row-content pattern; do not silently expand on_activate's meaning.
- **SplitPane import / F3 scope:** classify under the original finding's
  discovery symptom when a run logs the missing import and rustc provides
  no route. This does not make membership incorrect: SplitPane, SplitAxis
  and SplitSizing intentionally belong to windowing. Improve the relevant
  guide's import/module pointers rather than widening the prelude. Final
  recurrence cells still require each run's own evidence.
- **App::size, Length::from(u32), guessed snapshot type, Rust borrow/cast
  errors and dialog outcome acknowledgement:** the existing signatures,
  documentation and compiler diagnostics explain them. No defect is
  verified; keep the observations as tester errors or documented behavior.
- **Baseline-only toast inspection and TaskReceipt warnings:** existing
  R2-20 and R2-4 evidence, already repaired at head. Do not write another
  implementation fix for baseline behavior. Baseline Claude's state slot
  and virtual-list ordering leads are assessed with all paired runs later.

The initial positions above preceded execution. The frozen-head probes now
pass (see probes/README.md and output.txt): address discovery, deferred Dynamic
write/reveal, missing item geometry, actual dialog paths and Select/menu lookup
all have execution evidence. They disprove the proposed Dynamic navigation and
address_children defects, and confirm the geometry/discovery gaps. Independent
acceptance is underway after the timed blind queues finished. Subsequent
library repairs are regression-tested and reviewed separately; these blind runs
do not test the repairs.

## Further source checks during implementation

- Claude task 3's component-import report is a real omission from the stated
  component-module contract, rather than solely a missing prelude import:
  `guide/custom-widgets.md` promises every name a Component implementation
  writes, including errors it returns, while `component` omits ApplicationError.
  Add that existing facade-owned error to `component` and the membership test;
  no prelude widening or new error constructor is needed. Corrects the earlier
  provisional assessment. The two-glob examples already compile.
- That run's drawing-test report exposes overbroad testing-guide wording.
  Runtime::paint and DrawCommand belong to the component/host drawing contract,
  beyond the testing module's declared seed operations. Clarify the additional
  component imports instead of adding every Runtime service to testing.
- ApplicationError already implements From<String> and From<&str>. Improve the
  new constructor's pointer to those conversions; do not introduce msg or reuse
  RuntimeError::Closed as an application failure. Geometry f32 precision is
  expected; presentation/quantization policy belongs to a custom control.
- Further acceptance lead pending verification/triage: a task 6 verifier saw a
  literal tab rendered as a missing-glyph box in a TextArea CPU capture. Read the
  text/renderer contract and reproduce it before deciding a fix or disposition.
