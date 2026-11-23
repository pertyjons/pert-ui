# Rotary control variants and Micro typography

Public contract implemented on 2026-11-23. The example below illustrates the API. The intended consumer is an application building dense parameter panels with ordinary pert-ui controls. The visual experiment established the desired dial faces and 10-logical-pixel readout. Application routing icons, modulation semantics and panel layout belong to the application.

## Public surface

| Item | Contract |
|---|---|
| `MetricRole::MicroSize` | A positive logical font size, default 10 in Compact, Standard and Comfortable. Each density can override it through the existing theme API. DPI and UI zoom apply as for CaptionSize. |
| `TextStyle::Micro`, `TextStyle::MonospaceMicro` | Both resolve MicroSize. Micro follows FontRole::Body and MonospaceMicro follows FontRole::Monospace, including registered role overrides and fallback. |
| `TextSpan::style(self, style: TextStyle) -> Self` | Selects the span's font role and themed size together. All TextStyle variants are accepted. |
| `KnobVariant::{Bakelite, Orbit, Diode}` | Public non-exhaustive enum, Copy, Clone, Debug, PartialEq, Eq and Default. Bakelite is the default. Export alongside Knob and ArcOrigin through the runtime, facade and parameter module. |
| `Knob::variant(self, variant: KnobVariant) -> Self` | Selects the dial face. Same measurement and hit geometry for each variant. |
| `Knob::value_style(self, style: TextStyle) -> Self` | Style for the optional readout, default MonospaceMicro. |
| `Slider::value_style(self, style: TextStyle) -> Self` | Style for the optional readout, default MonospaceCaption as today. |
| `InlineSlider::style(self, style: TextStyle) -> Self` | Value and unit style, including editable value typography. Default Monospace. The secondary caption continues to use Caption as its inherited style. Follows InlineInteger and InlineDecimal. |

All builders replace earlier settings. Knob and Slider still start with the readout hidden. Selecting a style does not turn it on. Knob has no visible heading or label slot. Applications compose a Label above or beside it. Inspection and semantics continue to identify the parameter. Existing controls accepting TextStyle gain the Micro choices automatically. A menu, table or composite panel does not gain a blanket typography override. Consumers can style a supplied rich caption or choice at span level wherever the component presents rich text.

Micro denotes the shared size. The default numeric face at that size is explicitly MonospaceMicro. A panel label can independently select Micro.

Usage, with `cutoff` an existing `Parameter<f32>`:

```rust
use pert_ui::{Knob, KnobVariant, Label, RichText, TextSpan, TextStyle};

let title = Label::new("Cutoff").style(TextStyle::Micro);
let dial = Knob::new(cutoff.clone()).show_value(true);
let orbit = Knob::new(cutoff)
    .variant(KnobVariant::Orbit)
    .show_value(true)
    .value_style(TextStyle::MonospaceCaption);
let option = RichText::new().span(TextSpan::new("Lowpass").style(TextStyle::Micro));
```

## Span resolution and invalidation

Treat font and size as two replaceable properties. A new span inherits each from its enclosing style. Calling `style` writes both properties. A later `size` replaces just the size with a fixed logical length. A later `font` or `font_role` replaces just the font. A subsequent `style` replaces both again. Tint is independent in every ordering. Thus `.font_role(FontRole::Heading).style(TextStyle::Micro)` follows the body font, while reversing the calls keeps the heading font at MicroSize. This gives authors a live semantic alternative to the existing fixed `size`; exact font, font-role and literal-size builders remain useful individually.

Resolve themed sizes and font roles against the requesting runtime during measurement or painting. Preserve the existing first-text-span paragraph default, including an empty first span, grapheme/paragraph shaping across runs, icon metrics, existing size limits, and validation of the final effective properties. A foreign exact font which remains selected and a zero literal size which remains selected retain their existing errors. Apply the same positive text-size validation to MicroSize as to CaptionSize.

Both layout and paint must resolve the same effective span runs. Track span metric reads in the requesting phase and font reads with existing dependencies, including when a prepared layout is reused. Include effective sizes in rich-text cache reuse decisions, including a changed MicroSize with an unchanged outer BodySize. An explicitly styled span must survive the single-span plain-text shortcut. A palette-only change continues to affect tint through paint. Changing MicroSize must invalidate its readers without globally rebuilding unrelated layouts. See [rich content](../crates/pert-ui-runtime/src/text/rich.rs), [text preparation](../crates/pert-ui-runtime/src/text.rs) and [theme metrics](../crates/pert-ui-runtime/src/runtime/theme.rs).

Extend the existing rich-content metric collection in MeasureContext::rich_text and Painter::rich_text. Resolve each distinct effective named span size through the requesting context's tracked metric accessor and pass the resolved sizes with the text request. The text service can then prepare each run without accessing the theme itself. The existing resolved-run cache key already carries the effective sizes. Resolve dependencies before cache lookup, retain the plain fast path for genuinely inherited plain content, and exercise any component-local cached measurement with a live MicroSize change. Put the first-span paragraph rule in the span builder's rustdoc.

## Numeric presentations

Use the selected readout style for measurement of the width hint, arrangement and drawing. Preserve units, decimal places, NumberFormat callbacks, value text in inspection, clipping, and placement for horizontal and vertical sliders. Numeric formatting remains a separate contract. Explicit proportional styles are supported; glyph advances then follow the chosen font. Formatter hints reserve their measured extent in that font. When the available width is insufficient the existing readout clipping policy applies.

For the built-in fixed-decimal presentation, reserve a stable slot from the specification, precision and literal unit, including a possible sign and rounding carry. Preserve today's monospaced padding in inspection. Draw the numeric content and its unit together, end-aligned within that slot using measured advances, without drawing artificial leading padding. Preserve the existing constrained-width clipping and origin for oversized text. Reserve enough space for the widest supported digit and punctuation in a proportional style, including a proportional replacement for a font role, without remeasuring the control on every value change. Fixed track length and a stable slot edge are required; exact alignment of decimal points depends on the font's advances. Keep a caller-supplied format's width hint authoritative even if its actual text is wider. Cover a narrow digit, a wide digit, a minus sign, a fractional part and a unit on Knob and both Slider orientations.

InlineSlider applies its selected style consistently to resting value, editable draft, value width hint, unit and baseline calculation. Its rich caption retains its own overrides. Adopt the incoming editor style during NumericControl reconciliation and propagate it through the inline field and mounted draft editor. Test a style change during an open edit, retaining draft text, selection, focus and parameter session while remeasuring. Same-key redescription with the same parameter replaces presentation properties and restores defaults for omitted properties. Changing a visible text style requests layout. Changing only the rotary variant requests paint. Retain the parameter binding, active edit, click series, focus, draft and selection on presentation-only changes, subject to existing geometry/input-context cancellation rules. Rebinding to a different parameter and revoking permission retain their existing cancellation behavior. Presentation changes do not publish parameter edits or user-write callbacks. See [numeric controls](../crates/pert-ui-runtime/src/numeric_controls.rs) and [component contracts](../docs/contracts.md).

## Rotary appearance and interaction

All faces follow the existing 270-degree sweep, increasing clockwise with the opening at the bottom. Use the normalized parameter value and normalized ArcOrigin for visual positions, including nonlinear parameter mappings. Draw active extent between origin and current value in either direction. At exact equality draw zero active extent. A current-position indicator remains visible there and between discrete marks.

| Variant | Face and position indicator |
|---|---|
| Bakelite | A filled face with a continuous contrasting rim, radial major/minor tick marks and a prominent pointer. Active ticks show the extent. Omit the decorative upper-left reflection from the early experiment. |
| Orbit | A continuous neutral track, highlighted arc and distinct endpoint dot for the current value. |
| Diode | A dotted track with larger or more prominent lit dots across the active extent, and a short radial pointer for the precise current value. |

Use theme colors throughout. Use Primary for enabled value indications, subdued neutral roles for inactive marks and the rim, and Muted for disabled indications. Keep enabled hover feedback, OnSurface/Muted readout, the existing independent FocusRing and error outline, including rejected edits. Respect KnobDiameter, KnobStroke and FocusWidth. Fit small allocations, zero metrics and fractional scale without invalid geometry or drawing beyond the dial allocation. Tick/diode counts are visual tuning, independent of the parameter step and of hit testing. Keep render work bounded per dial; a resting variant schedules no updates.

Use SurfaceContainer for the resting face and SurfaceHover for its interior on enabled hover. Keep the rim visible in Muted and inactive tracks or marks in Outline. The active marks, pointer and Orbit endpoint use Primary when editable and Muted when disabled. Keep the standard focus and error outlines outside the face decoration. Use the following initial geometry for visual review (internal tuning, outside the public API):

- Bakelite uses 21 evenly spaced tick positions including both endpoints, every fifth a major tick. Diode uses 19 evenly spaced positions including both endpoints. Orbit draws a continuous neutral track and has no discrete positions.
- Let each mark have normalized position t, origin o and current position v. For a nonempty interval light positions in the closed interval between o and v, including a mark coincident with either end. If v equals o, every scale mark is inactive. A pointer or endpoint still indicates the current position. Draw the Orbit arc with the same endpoints.
- Review normal-size captures at Compact in both palettes and at fractional display scale. The pointer or endpoint, active extent, idle track and hover change must be distinguishable within the existing hit area. Tune strokes or mark spacing if they merge before approving the implementation. Keep transparent gaps between Diode dots at the default Compact metrics.

All variants use the same numeric control's subscription, normalized mapping, hit testing and input code. Drag travel, Shift precision, wheel and keyboard editing, click cancellation, double-click/default reset, authority, remote submission and focus/permission-loss handling retain their contracts. Routing and panel icons remain host presentation. This change adds no skin callback, edit controller, synth-specific component or new data ownership. The variant builder makes the existing dial behavior usable with these appearances without authoring a custom component.

## Compatibility, implementation and evidence

Adding variants to today's exhaustive TextStyle is source-breaking for consumers with exhaustive matches. Migrate those matches and document both new arms. MetricRole already permits extension. The ordinary Knob adopts Bakelite, with Orbit the closest continuous-arc alternative for consumers of the former look. Orbit uses an endpoint dot where the former face used a pointer line. A visible Knob readout changes from 12 to 10 logical pixels at default settings; `.value_style(TextStyle::MonospaceCaption)` restores its former typography. Hidden readouts, Slider and InlineSlider retain their defaults. Typography and visual variants are presentation properties. This proposal introduces no persisted field or schema migration.

1. Implement the theme metric, text styles and span resolution. Extend the typed metric table and completeness checks. Classify positive text metrics exhaustively for validation so a new text role cannot silently accept zero. Exercise both font roles across densities, theme override and zoom, invalid zero size, explicit font/size precedence, inherited tint, mixed spans, empty first span and retained layout after a live metric change.
2. Add numeric typography builders with public rustdoc and facade doctests. Verify hidden/shown readouts, unit and formatter hints, proportional and monospace styles, vertical slider clipping, caption/value baselines, same-key replacement and editing a styled InlineSlider draft. Changing text style must use the new geometry for measurement and painting together.
3. Implement rotary variants against the common layout and event handling. Compare CPU-rendered dark and light faces at minimum, intermediate and maximum values, at a non-minimum default and on either side of it. Include dense allocations, custom palette, hover, disabled, focus and rejection states. Confirm a variant switch during dragging neither resets the value nor ends the edit. Reuse existing gesture and parameter tests. Replace the old default-face test that counts one fewer draw command at the default origin with an assertion on the visible active extent, retained identity, bounds and unchanged parameter value. See the [knob contract tests](../crates/pert-ui/tests/controls/knob.rs). Give the gallery a public-facade comparison with separate labels and representative densities.
4. Update the changelog, public API listings, relevant guide/design text and runnable examples with implementation. Run the [public API workspace gate](../AGENTS.md#verification-and-review), targeted behavior and pixel checks, and review the API diff and semantics before merge. Add KnobVariant to the [facade module export checks](../crates/pert-ui/tests/foundation/modules.rs). Treat the disposable experiment as visual reference only; shipped builds, tests and examples must be self-contained.

## Design review

Claude reviewed the initial specification against the repository on 2026-11-23. The review found no architectural obstacle and requested clarification before implementation. The revised sections specify the numeric font default, proportional alignment, tracked span-size resolution, rotary role mapping, hover and mark intervals. The verification scope now includes inline restyling during a draft, positive text metric validation, facade exports and Compact visual acceptance. TextStyle retains its current exhaustive contract in this proposal. Making it non-exhaustive would be a separate public API decision. Claude re-read the revised specification and reported that all seven findings were addressed, with no remaining design blocker. The follow-up asked for confirmation of the cache key detail. Author review verified that Key contains Resolved, whose ResolvedSpan entries carry their effective size in the [text implementation](../crates/pert-ui-runtime/src/text.rs).

## Implementation verification

The gallery's **Input & parameters** page presents the three faces on one parameter.
The controls suite exercises a face change during a drag, readout styles and
retained draft selection. The typography suite changes MicroSize while labels
and a button hold prepared text. The CPU tests render Compact faces in both
palettes at 1.25× scale and compare active coverage on either side of the default.
The compact previews were also inspected visually. An independent Claude
implementation review prompted shared glyph measurements for numeric slots
and focused checks for style replacement and retained text.
