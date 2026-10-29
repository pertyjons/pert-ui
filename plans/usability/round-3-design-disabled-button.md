# Round 3 checkpoint: disabled Button presentation

Status: revision 2 approved with the recorded corrections and implementation
requirements below, 2026-10-28. Implemented on working 0.3.90; initial
implementation review approved code/tests, with documentation corrections.
Final workspace gates and API generation passed; the
[final independent review](runs/review-r3/disabled-button-records-review-r3.md)
approves this checkpoint on 2026-10-29. This changes presentation, not permissions.

## Verified finding and scope

The frozen head's Button::paint uses Button fill both enabled and disabled;
only OnButton changes to Muted. Both the blind tester and independent task-1
verifier found the states hard to distinguish, and their clean/dirty captures
show the same button surface. Source confirms that the hover/pressed treatment
is already denied when disabled. Muted's documented normal-text contrast is
against Background, Surface and SurfaceHover, not Button.

Use an outlined neutral surface for a disabled framed Button: fill Surface,
text/icons Muted, inward outline Outline at MetricRole::BorderWidth, preserving
the same size, padding and CornerRadius. Enabled buttons keep their current
Button/hover/pressed fills and OnButton pairing. The explicit frameless family
keeps no fill/outline when disabled and continues using Muted text. Do not dim
opacity, hardcode colors, add a disabled-role palette or change every other
control family. This replaces the same-surface/text-only distinction for
framed buttons. Existing theme roles are shared with fields/panels/supporting text: there is
no independent disabled-button-only override. Overriding Button no longer
changes a disabled framed button; Surface/Muted/Outline/BorderWidth do.

Keep Permission composition, disabled semantics, focus/held-input cancellation,
callback timing and restrictions unchanged. Live enablement changes must clear
the old active gesture as today and repaint the appearance. No new API, schema
or package boundary is needed. Custom palettes choose their own contrast;
built-in palette checks cover the chosen roles only.

The author selects this under the delegated usability goal because the finding
has actual visual and source evidence. It is intentionally limited to Button,
including its icon/content variants which share the same paint path.

## Required implementation evidence

- Public input proves a disabled button never fires from pointer/keyboard and
  enablement changes during a hold cancel without replay. Preserve existing
  contract tests rather than duplicating their implementation.
- Both built-in palettes: enabled/disabled CPU captures differ in their surface
  and boundary; disabled label remains readable. Reuse the existing theme
  assert_readable evidence for Muted/Surface (4.5:1) and Outline/Surface (3:1),
  rather than duplicating the same palette test. Capture ordinary text and
  an icon variant, plus disabled frameless with no added fill or outline.
- Theme overrides and live enablement/geometry remain correct. Inspect display
  commands for adaptive role resolution and inward stroke; CPU captures check
  actual pixels so this is not just a mirror of the conditional.
- Update Button rustdoc and the controls/appearance guidance, CHANGELOG and
  rendering-platform/default-role documentation where affected. No new default
  colors or arbitrary numeric color table is introduced.
- Focused package checks, affected doctests/strict rustdoc, required workspace
  gate and API generation (no public signature difference expected), followed
  by independent implementation review. Linux headless evidence does not claim
  new native/human/Windows/macOS results.

Reviewer reads source and relevant contracts, authors no fixes and does not
delegate. Assess whether existing roles/frameless treatment and stroke geometry
make this coherent, and identify any missed enablement or shared paint path.

## Decisions after the initial review

The [initial review](runs/review-r3/disabled-button-design-review-r1.md)
confirmed the paint path, role contrast and stroke behavior. Accept its requested
decisions as follows:

- Deliberately share the neutral inactive control frame with disabled fields.
  This is a state vocabulary, not a new control identity. Button captions remain
  centered; editable text is aligned for editing. Add adjacent enabled/disabled
  buttons, an empty placeholder field and a disabled field in both-palette CPU
  captures so the tradeoff is reviewable. Do not present this as an independent
  disabled-button palette.
- Choose disabled roles before considering press/hover. Paint fill, then inward
  outline, then focus ring, then content; no stale ButtonPressed on revocation.
  Reuse existing availability cancellation tests, with paint-focused checks
  for revocation/re-enable.
- BorderWidth zero intentionally omits the outline. Custom widths follow the
  existing field-frame inward-stroke rule: a width reaching half the smaller
  extent fills the shape. Document that geometry and custom contrast ownership,
  rather than silently clamping a valid theme metric.
- Text/icon/command variants share Button's paint. Contextual command
  predicates do not change its presentation enablement contract. Generated
  framed AlertDialog buttons share the path but are never disabled, so they
  have no visible change. Empty/all-fixed-disabled Menu triggers use framed
  Button and do change; icon-menu triggers, HistoryNavigator and tool-panel
  hide buttons are frameless and retain text-only treatment.
- Update ColorRole::Button/OnButton, MetricRole::BorderWidth, Button/frameless,
  controls/appearance and relevant rendering documentation together. The shared
  roles and custom-width behavior must be explicit.
- Additional focused evidence: Surface/Outline/Muted/BorderWidth overrides
  repaint without relayout; BorderWidth zero has no stroke; disabled frameless
  has no fill/outline; contextual command availability keeps its current look.
  Existing palette and availability tests supply their already-proven contracts.
- Select/MultiSelect's direct Button/Muted paint is a verified related
  presentation improvement outside this Button checkpoint, not repaired by it.
  Record it in TODO.md with source/rule and a consistent-treatment suggestion.
  Button's absent box-model color fields are also a verified inspection
  improvement; record separately without adding public paint metadata here.

Implementation evidence is in [disabled-button](runs/review-r3/disabled-button/README.md).
The [initial implementation review](runs/review-r3/disabled-button-implementation-review-r1.md)
approved code/tests subject to completed gates and the status/changelog repairs.
No native or human evidence follows from this approval.

The [revision-2 review](runs/review-r3/disabled-button-design-review-r2.md)
approves the policy subject to the factual correction above and a focused
Menu trigger test: an empty/all-fixed-disabled menu's exposed trigger paints
Surface/Outline, while its icon-menu counterpart stays without fill/outline.
These requirements are accepted before implementation. No AlertDialog visual
change is claimed. No further design choice remains open.

Post-review tightening: disabled frameless controls now read no undrawn frame
color/metric dependencies. A direct pre-flush idle assertion catches unused
Surface/CornerRadius/BorderWidth subscriptions. The frameless test uses its own
scene, so a wrong Button-colored fill cannot hide behind an enabled neighbor.
CPU evidence includes a frameless control and asserts the disabled field's
shared frame pixels, plus zero, three-pixel and shape-filling border widths.
Focused recheck and the complete final workspace gate passed; see
[final evidence](runs/review-r3/disabled-button/README.md). Source hashes
correlate the current implementation with that run. The r2 implementation
review approved code/tests; its pending records were discharged by r3.
No overall usability closure is claimed.
