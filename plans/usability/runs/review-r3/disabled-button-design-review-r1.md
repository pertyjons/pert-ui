**Verdict: needs changes.** The approach holds up: it uses only existing roles, adds no API, leaves permissions alone, and the stroke geometry is sound. But one design decision is missing, one claim in the plan is inaccurate, and some details need pinning down. Once those are added to the plan, I'd approve it subject to the tests listed below.

## What I confirmed in the source

- **The defect is real.** `Button::paint` fills with `Button` whether or not the button is enabled; only the text changes, to `Muted` (`crates/pert-ui-runtime/src/controls.rs:810-841`). `Muted` is documented and tested for contrast against Background, Surface and SurfaceHover only, not against Button (`runtime/theme.rs:426-429`, `crates/pert-ui/tests/presentation/theme.rs:413-415`).
- **Hover and press are already off when disabled.** Hover is checked against `enabled` (`controls.rs:812, 819`). The pressed branch at `:810` doesn't check `enabled`, but it can't be reached while disabled: `update` resets the input when disabled (`:741-745`) and so does a fixed redescription (`:706-709`).
- **Enablement changes repaint.** Reactive enablement goes through `read_paint`, and a fixed change sets the paint flag (`:700-702`). `Painter::color` and `Painter::metric` re-subscribe on every paint (`drawing.rs:1563-1578`), so switching between the enabled and disabled role sets works.
- **The geometry fits.** `stroke_shape` draws inward and never changes layout. It is at least one physical pixel wide and draws nothing at width 0 (`shapes.rs:348-378`). With padding of at least BorderWidth, the caption and icon variants don't collide with the outline.
- **Icon, command and frameless variants all go through the same `paint`.** Frameless disabled buttons already get no fill (`:819`).
- **The gesture evidence already exists.** `crates/pert-ui/tests/controls/component_availability.rs` covers Button for fixed denial, a keyboard hold being revoked, pointer revocation, and a fixed redescription during a hold (`:79, :121, :324, :364`).
- **The contrast evidence already exists.** `assert_readable` checks Muted on Surface at 4.5:1 and Outline on Surface at 3:1 for the built-in and seeded palettes (`theme.rs` tests `:413, :423`). The plan should cite this test rather than add a duplicate.

## Findings that need a decision

1. **A disabled button would look like a text field.** Surface fill, Outline at BorderWidth and the CornerRadius shape are exactly the shared editable-field frame `field_frame` (`choice_controls.rs:144-173`). TextField, InlineText and the inline editors use it (`text_fields.rs:2920`, `inline.rs:567/675`, `inline_text.rs:1072`). A disabled TextField also draws Muted text in that frame (`text_fields.rs:2949-2953`), and an empty field shows a Muted placeholder (`:2929-2940`). Under this proposal the only visible difference is that the button's caption is centred. The plan needs to do one of two things:
   - accept a shared "inactive control" frame on purpose and say so, or
   - pick a combination of existing roles that doesn't match the field frame.

   Either way, add a capture showing a disabled Button next to an empty TextField with a placeholder and a disabled TextField, in both palettes.
2. **The claim "existing theme roles provide independent consumer control" is wrong.** Surface, Outline, Muted and BorderWidth are shared with panels, fields, swatches and supporting text, so there's no way to restyle disabled buttons alone. Overriding `Button` will no longer affect disabled buttons. The rustdoc and CHANGELOG should state this coupling as the cost of adding no new role.
3. **Select and MultiSelect have the same defect.** They fill with `Button` and switch to `Muted` text when disabled (`select.rs:1275-1295`, `multi_select.rs:1049-1062`). After this change, a disabled dropdown will look more like an enabled button than a disabled button does. Leaving them out of scope is fine, but AGENTS.md requires the finding to be recorded in TODO.md (there's no existing entry). Notification action buttons (`notifications.rs:1291-1308`) paint their own button, but they have no disabled state, so they're unaffected.

## Details to pin down

4. **Paint order.** When disabled, the fill/outline/text choice should not depend on the pressed or hovered state. Draw the fill, then the outline, then the focus ring, so a briefly lingering ring covers the outline rather than the other way round.
5. **Theme edge cases to document.** With BorderWidth set to 0 there is no outline, so only the Surface fill and Muted text distinguish a disabled button. If BorderWidth is at least half the button's height, the stroke fills the whole shape with Outline (`shapes.rs:391`), which makes the text unreadable. The field frame has the same limit.
6. **Docs to update besides the Button rustdoc.** The `Button` role says "Resting action control fill" and `OnButton` says "Text on resting or hovered buttons" (`theme.rs:446-451`). The `frameless` docs mention the "resting fill" (`controls.rs:634-637`). The Button docs say "opaque blue-gray action surface" (`:434-435`). BorderWidth is documented as "Panel and separator stroke width" (`theme.rs:105`), but fields already use it; consider widening that text.
7. **Tests to add (new, paint-focused).**
   - While a hold is revoked, the next paint has no ButtonPressed fill and does show the Surface fill and Outline stroke.
   - Re-enabling brings the Button fill back.
   - Overriding Surface, Outline, Muted or BorderWidth repaints without a relayout.
   - BorderWidth 0 draws no stroke.
   - A disabled frameless button draws no fill or stroke command.
   - A command button with a contextual predicate stays enabled and its appearance doesn't change.
8. **Composites that inherit the change.** The framed buttons generated by AlertDialog (`dialog.rs:626-633`) pick up the new look. The history navigator's back and forward buttons are frameless (`history_navigator.rs:239-244`), so they're unaffected. The CHANGELOG should mention the dialog buttons.

Optional, out of scope: Button reports no box-model `background`, `foreground` or `border`, unlike Container and Toolbar (`containers.rs:368-372`). Reporting them would make the disabled look visible to inspection-based tools.

## Scope and limits

I only read source and docs; I didn't run any tests or look at the captures. The defect evidence from head task 1 and the acceptance captures is taken as established. I didn't check the Linux, Windows or macOS native paths. I made no edits, didn't delegate, and didn't open any transcripts or credentials.
