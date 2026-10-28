# Round 3 checkpoint: disabled Button presentation

Status: proposed 2026-10-28; requires independent Claude design review before
implementation. This is a default-presentation change, not a permission change.

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
framed buttons. Existing theme roles provide independent consumer control.

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
  and boundary; disabled label remains readable. Assert Muted on Surface has
  at least 4.5:1 contrast via the existing Color API. Capture ordinary text and
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
