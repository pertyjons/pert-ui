**Verdict: needs changes, but only to evidence and records. The source code and tests are approved.** I can't approve the checkpoint yet: the final workspace gate is still running, and the evidence README doesn't describe the post-review evidence.

## Repairs and tightening I verified

- **Plan status** (`round-3-design-disabled-button.md:3-7`, `:103-121`) is fixed. It points to the evidence and the first implementation review, and describes the post-review tightening.
- **CHANGELOG** is fixed. `CHANGELOG.md:5-6` now says "independently reviewed Button design checkpoint". There is a single blank line before `## 0.3.89` at `:32-33`.
- **Frame roles are read only when drawn** (`controls.rs:816-866`):
  - A disabled frameless button reads no CornerRadius, Surface, BorderWidth or Outline; it reads only Muted for its content.
  - An enabled frameless button that has focus still reads CornerRadius and draws the focus ring.
  - A disabled framed button reads exactly the four roles it draws.
- **The pre-flush idle assertion can catch a false dependency** (`disabled_button.rs:143-161`).
  - Theme changes invalidate their subscribers immediately, by role (`theme.rs:966-996`, `:1019-1037`).
  - `needs_paint` checks each component's paint-dirty flag (`compose.rs:342-362`).
  - So an unused Surface, CornerRadius or BorderWidth read would make the assertion fail.
- **Separate scenes.**
  - The frameless scene contains only that button. It asserts no Button, Surface or Outline is painted, that Muted is, and that the button reports disabled.
  - The contextual-command button is in a fresh runtime (`:163-181`).
- **CPU pixels.**
  - Cases are keyed by widget id and role (`:280-286`). They cover enabled, disabled, icon, the disabled field's shared frame, and the frameless button against Background.
  - The border-width test (`:309-346`) checks 3 (Outline at 1.5, Surface at 4.0), 0 (Surface at both) and 240 (Outline at both).
  - That would catch a centred stroke, a drawn zero-width stroke, or clamping.
- **Menu rustdoc** (`menu.rs:674-675`) now states the framed/icon distinction. The `[Button]` link resolves through `use crate::*`.
- **Docs** in `theme.rs` (BorderWidth, Button, OnButton), the guides and `rendering-platform.md:24-28` are consistent.
- **Captures.** Both `captures-final` PNGs show the enabled/disabled buttons, both fields, the icon button and the frameless "No frame" button with no fill or outline. I can see the images but can't measure pixel values.

## Findings

1. **Final gate not concluded (blocking).** In `r3-disabled-button-final-gates`:
   - `outcomes.json` records only format and Clippy, both exit 0.
   - `workspace-tests.log` has no outcome. It was still growing in the gallery unit tests (line ~3339) while I read it.
   - Everything up to that point is green, including all 6 `disabled_button` tests (lines 2416-2578).
   - Examples, doctests, strict rustdoc and diagnostics haven't run, and there is no `source-unchanged.txt` yet.
   - The earlier all-zero gate covered older source: its hashes for `controls.rs`, `menu.rs`, `theme.rs` and the test differ. So it doesn't cover the changed rustdoc.
2. **Evidence README is out of date (fix before commit).** `disabled-button/README.md` doesn't describe:
   - the 6-test `review-focused.log` (line 7 still points to the 5-test "final" run),
   - the `captures-final/` directory, which also lacks a `provenance.json`,
   - the new frameless capture (lines 19-20),
   - the final-gates location.

   The focused and Clippy logs there also lack command headers.
3. **Optional test gap.** No test focuses an enabled frameless Button and asserts its focus ring. I verified that path only by reading the code.

## Limits

- I couldn't hash current files against `source-manifest.json` or diff `Cargo.toml`/`Cargo.lock`.
- API generation is still pending. Only private paint code and rustdoc changed, so I expect no listing difference, but I didn't check it.
- No native, GPU, Windows/macOS, human or overall-usability claim is made.
- I made no edits, ran no commands, didn't delegate, and read no transcripts, configuration or credentials.
