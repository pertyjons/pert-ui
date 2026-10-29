**Verdict: approved for the code and tests, with two small documentation corrections the author should make before committing.** The approval holds only once the external workspace gates finish green. I have not assumed any check passed that hasn't finished.

## What I verified in the source

- **Paint order (`controls.rs:816-862`).** The disabled check now comes first (`!self.enabled` → Surface), before the pressed or hover branches. That removes the earlier reliance on reset paths to hide a stale ButtonPressed. The order is fill, then inward Outline (only when disabled and framed), then focus ring, then content. Disabled text uses Muted whatever the press state.
- **Enabled and frameless buttons.** Enabled buttons keep Button / ButtonHover / ButtonPressed and OnButton / OnButtonPressed, with no outline. A disabled frameless button gets no fill and no stroke: the fill needs `self.enabled`, and the stroke needs `!self.frameless`.
- **Stroke geometry (`shapes.rs:366-449`).** The stroke draws inward. It draws nothing at width 0, at least one physical pixel otherwise, and at half the smaller side or more it fills the whole shape. It is emitted as FillRect/DrawMask, so the test helper `uses()` does see it. The docs describe these limits rather than silently clamping.
- **Theme reads are paint-only.** `Painter::color` and `Painter::metric` are paint dependencies, and the button's layout reads ControlHeight, ControlWidth and the padding metrics, not BorderWidth or the colours. The test that records Layout spans (`tree.rs:2106-2113`) is therefore meaningful.
- **Shared frame is real.** `field_frame` (`choice_controls.rs:148-173`) uses the same Surface/Outline/BorderWidth/CornerRadius, and TextField calls it (`text_fields.rs:2928`).
- **Composites.**
  - The framed `Menu::new` trigger becomes disabled (`menu.rs:823`), so it changes. The `Menu::icon` trigger is frameless (`:868`) and doesn't.
  - Toolbar "More", window minimize/close, the tool-panel hide button and HistoryNavigator are all frameless. I found no other internal framed Button that can be disabled.
  - AlertDialog buttons are never disabled, and the CHANGELOG correctly claims no visual change for them.
- **Docs.** These are consistent and state the role coupling explicitly:
  - Button and `frameless` rustdoc
  - `ColorRole::Button` / `OnButton` and `MetricRole::BorderWidth` docs
  - the controls and appearance guides
  - `rendering-platform.md`
  - the 0.3.90 CHANGELOG entry
- **Out-of-scope items.** The two TODO.md entries (Select/MultiSelect disabled dropdown paint, and `Button::inspect` box-model colours, `TODO.md:228-241`) exist and are not fixed here, as intended.

## Tests and evidence

`disabled_button.rs` is registered in `presentation/main.rs:16`. All 5 tests pass in the final focused log, and also in the external workspace-test log (lines 2412-2630).

- **Revocation and re-enable.** The test confirms ButtonPressed paints first, then after revocation Surface, Outline and Muted with no ButtonPressed, and unchanged bounds. After re-enabling it confirms the Button fill with no Outline, that a release replays nothing, and that a fresh click fires once. It would catch a missing input reset, because the action runs synchronously in `pointer_input`.
- **Overrides.** Surface, Outline and Muted overrides each appear in the scene with no Layout span. BorderWidth 3 draws the outline and BorderWidth 0 draws none, again with no layout.
- **Menu.** The framed trigger paints Surface and Outline; the icon trigger doesn't. Neither paints Button, and both use Muted.
- **CPU pixels.** In both palettes, the test samples the real rendered pixels for the enabled button, the disabled button and the disabled icon button. It checks the interior at x+3 against Button or Surface, and the edge at x+0.5 against Button or Outline, within ±1. The test can catch real failures:
  - If the fill were missing, the Background colour would show through, which is distinct from Surface in both palettes.
  - A centred (half-outside) stroke would blend the edge pixel, so it would fail the ±1 check.
- **Reused evidence.** The theme readability test, `built_in_palettes_keep_readable_foregrounds_and_visible_outlines` (covering Muted on Surface and Outline on Surface, `theme.rs:413/423`), passes, as do the 9 `component_availability` tests.
- **Captures.** Both PNGs were readable here. They show enabled and disabled "Apply", an empty placeholder field, a disabled field and a disabled icon button. They match the claimed treatment. I can view the images but can't measure pixel values, so pixel accuracy rests on the test's assertions.

## Findings (none block the code)

1. **Plan status is out of date** (`round-3-design-disabled-button.md:3-4` and `:100`). It still says "No implementation evidence yet" and "No code has been written for this checkpoint". Update it to point to `runs/review-r3/disabled-button/` and this review. *Needs fixing.*
2. **CHANGELOG wording** (`CHANGELOG.md:5`). "from the independently reviewed Button checkpoint" reads as if the implementation had been reviewed, but the earlier reviews covered the design. Suggested: "independently reviewed design". There is also a double blank line at `:30-31`. *Needs fixing.*
3. **Test gaps (optional):**
   - The adjacent TextField pixels are captured but never asserted, so the "shared frame" claim relies on the code being identical plus a person looking at it. One interior and one edge assertion on the disabled field would make it mechanical.
   - The frameless/contextual test puts the frameless button in the same scene as a button that legitimately paints Button. So a wrong Button-coloured fill on the disabled frameless button would go unnoticed, and its Muted content isn't asserted either. Separate scenes, or sampling the frameless button's bounds, would close that.
   - The BorderWidth 3 case checks only that the outline is present, not its inward width.
   - The design text lists a "disabled frameless" capture, but the CPU capture has none; it's covered by display-command assertions only.
4. **Small paint-dependency nit (optional).** A disabled frameless button reads `ColorRole::Surface` (`controls.rs:818-819`) without drawing it, so it repaints needlessly when Surface changes. Before this change it read Button the same way. Reading the fill colour only when it's actually filled would tidy this.
5. **Optional docs.** The `Menu` rustdoc ("Empty/all-disabled menus have a disabled trigger", `menu.rs:673-674`) could say that the framed trigger takes the inactive frame and the icon trigger stays frameless. The guides already say so.

I found no new verified issues outside this scope.

## Gate status and evidence limits

- **External gates** (`/home/per/pert-ui-usability/r3-disabled-button-gates`):
  - `cargo fmt --all --check`: exit 0.
  - Workspace Clippy: exit 0.
  - `cargo test --workspace`: no recorded outcome. The log reaches the gallery unit tests with no failures so far. **Not concluded.**
  - Doctests, strict rustdoc, the example sessions and the diagnostics check: not recorded.
- **API listing gate:** pending; it's the author's responsibility. I didn't check the `api/` listings for differences.
- **Things I couldn't verify:**
  - I couldn't diff files (no git access in this review), so I couldn't confirm that the `Cargo.toml` / `Cargo.lock` changes are only the version bump to 0.3.90. The version lines I saw are consistent with that.
  - I couldn't hash files against `source-manifest.json` or `provenance.json`.
  - The focused-check logs have no command headers (the README says those commands were recorded afterwards).
- **Not covered:** the text-tab work and the other modified round-3 plan files.

This review makes no native, GPU, Windows/macOS or human-input claim and does not close the usability round. I made no edits, ran no commands, didn't delegate, and didn't read any transcripts, configs or credentials.
