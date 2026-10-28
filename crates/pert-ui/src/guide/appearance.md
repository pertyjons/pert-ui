Appearance is shared state the runtime tracks like any binding: palettes and
color roles, density metrics, the application zoom, fonts and icons. A change
repaints or remeasures only the components that read what changed. Drawing goes
through [`Painter`], whose shapes, paths, text, icons and images are cached
independently of the graphics adapter.

# Themes and color roles

Each runtime starts in dark mode with built-in dark and light palettes. The shared
[`ThemeManager`] changes modes and overrides colors on the UI thread. Overrides are
stored independently per mode. `Primary`, `Secondary`, `Error`, `Warning`, `Info`
and `Success` have explicit `On…` foreground roles; backgrounds, panels, buttons,
tabs and muted text have their own tokens. Some roles follow another by default,
which [`ColorRole::follows`] reports: the foregrounds of buttons, tabs, selections
and the subdued `*Container` fills follow `OnSurface`, `PanelBorder` follows
`Divider`, and pressed action controls follow the accent pair through
`ButtonPressed` and `OnButtonPressed`. A follower takes its source's current
color until it is overridden itself, and [`Palette::with_default_color`] makes it
follow again. Following copies a color; overrides never generate other colors.
Keyboard focus has its own roles, so overriding `Primary` recolors the accent and
pressed controls without changing the focus ring. The focus ring is drawn
inward as one `FocusRing` band at `FocusWidth`. A custom component draws the
same ring with `Painter::stroke_shape` and the focused element's bounds and shape.

```rust
use pert_ui::{Button, Color, ColorRole, Component, Constraints, DrawError, Element,
              MeasureContext, Painter, Palette, Rect, Row, Runtime, RuntimeError, Size,
              ThemeMode, UpdateContext};

// External components use the same tracked color lookup as built-in controls.
// A leaf measures its own extent; without `measure` it would be given no space.
struct Accent;
impl Component for Accent {
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> { Ok(()) }
    fn measure(&self, _: &MeasureContext<'_>, _: Constraints) -> Result<Size, RuntimeError> {
        Ok(Size::new(24.0, 24.0)?)
    }
    fn paint(&self, painter: &mut Painter) -> Result<(), DrawError> {
        painter.fill_rect(Rect::new(0.0, 0.0, 24.0, 24.0)?,
                          painter.color(ColorRole::Primary));
        Ok(())
    }
}
let mut ui = Runtime::new();
let theme = ui.theme();
// Color::from_linear takes linear channels, not sRGB bytes.
let light = Palette::light()
    .with_color(ColorRole::Primary, Color::from_linear(0.30, 0.04, 0.40, 1.0)?)
    .with_color(ColorRole::OnPrimary, Color::from_linear(1.0, 1.0, 1.0, 1.0)?);
theme.set_palette(ThemeMode::Light, light)?;
theme.set_mode(ThemeMode::Light)?;
let content = ui.mount_tree(Element::new("content", Row::new([
    Element::new("accent", Accent),
    Element::new("toggle", Button::new("Toggle theme", move |_| {
        let next = match theme.mode()? {
            ThemeMode::Dark => ThemeMode::Light,
            ThemeMode::Light => ThemeMode::Dark,
        };
        theme.set_mode(next).map(|_| ())
    })),
])))?;
// Native App performs flush/paint and schedules presentation automatically.
ui.flush(std::time::Duration::ZERO)?;
assert_eq!(ui.paint()?.background(), light.color(ColorRole::Background));
let accent = ui.find(&content, &["accent"])?.ok_or("the accent is mounted")?;
let bounds = ui.bounds(&accent)?.ok_or("the accent is laid out")?;
assert_eq!((bounds.width(), bounds.height()), (24.0, 24.0));
# Ok::<(), Box<dyn std::error::Error>>(())
```

In [`App::new`], use the supplied runtime in the same way. A theme edit invalidates
only components that painted with a changed role; hidden components refresh when
shown. This does not rebuild the tree or invalidate layout/text masks. Reads via
[`ThemeManager::palette`] are untracked snapshots; use [`Painter::color`] for adaptive
drawing. [`Label::color_role`] and [`Container::background_role`] select roles, while
[`Label::color`] and [`Container::background`] provide fixed per-component overrides.
[`Container::background_role_signal`] follows a binding of a role instead, for a
fill that marks state, such as the selected strip of a mixer; a change of the
binding or of the theme repaints it without layout.
The last literal/role builder wins. Existing [`Color`] constants stay fixed.

## Seed palettes

[`Palette::from_seeds`] recolors a mode's built-in palette from a brand color,
and optionally a second one for the `Secondary` roles. The accent roles take the
seed's hue and colorfulness, neutral backgrounds, surfaces and controls take
its hue as strongly as the built-in neutrals are blue, which is clearly visible
in the dark palette, and status colors keep their built-in values. Every generated
color keeps its built-in luminance, so the contrasts of the built-in palettes
hold for any seed; the seed's own lightness is not used. The result is an
ordinary palette: overrides apply on top, followers follow, and
[`Palette::with_default_color`] returns a role to its seeded value. Seed both
modes, since the manager stores them separately.

```rust
use pert_ui::{Color, ColorRole, OpaqueColor, Palette, Runtime, ThemeMode};

let ui = Runtime::new();
let theme = ui.theme();
let brand = OpaqueColor::try_from(Color::from_linear(0.05, 0.45, 0.30, 1.0)?)?;
for mode in [ThemeMode::Dark, ThemeMode::Light] {
    theme.set_palette(mode, Palette::from_seeds(mode, brand, None))?;
}
// The exact brand color is an explicit override with its own contrast choice:
// this one is too light for the seeded white text, so it gets dark text.
let light = theme.palette(ThemeMode::Light)?
    .with_color(ColorRole::Primary, Color::from(brand))
    .with_color(ColorRole::OnPrimary, Color::from_linear(0.0, 0.0, 0.0, 1.0)?);
let fill = OpaqueColor::try_from(light.color(ColorRole::Primary))?;
let text = OpaqueColor::try_from(light.color(ColorRole::OnPrimary))?;
assert!(text.contrast_ratio(fill) >= 4.5);
theme.set_palette(ThemeMode::Light, light)?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

Luminance contrast does not measure hue. A seed close to a status hue makes the
accent resemble that status: a light red seed gives a light-mode `Primary` of
about (0.57, 0.05, 0.01) in linear RGB beside the built-in `Error` of
(0.55, 0.025, 0.035), so an action and an error look nearly alike, and a green
seed does the same beside `Success`. Choose a seed away from the status hues,
or override `Primary` or the status role. Without a secondary seed,
`Secondary` keeps the built-in hue distance from the accent, and a gray seed
makes both gray.

Custom graphics adapters must clear to `DisplayList::background()` before drawing
its commands. Custom hosts check
[`HostIntegration::needs_paint`](crate::host::HostIntegration::needs_paint) after events/flushes;
a background-only edit can require presentation without component updates.
An unchanged runtime schedules no theme timer. `needs_paint` scans mounted entries
for visible drawing damage. Theme writes compare the fixed palette and visit
registered painters; this is ordinary UI work, not a callback-safe API.
System preference detection, subtree theme providers, persistence, typography and
spacing overrides are outside this color-theme API. Foreground contrast remains
an explicit application choice when overriding colors; measure it with
[`OpaqueColor::contrast_ratio`].

[`Color`] provides the arithmetic a custom component needs on resolved colors,
in linear light. [`Color::with_alpha`] replaces alpha, while
[`Color::scale_alpha`] fades a color and keeps the translucency a role already
has. [`Color::mix`] interpolates premultiplied channels, and [`Color::over`]
composites onto an [`OpaqueColor`]. Contrast is defined only for opaque colors,
so a translucent color is measured after compositing it over what lies behind
it. The measurements report; they never adjust or refuse a palette.

```rust
use pert_ui::{Color, ColorRole, OpaqueColor, Palette};
let palette = Palette::light();
let surface = OpaqueColor::try_from(palette.color(ColorRole::Surface))?;
// A hover veil drawn at 40% of the role's own opacity, as it appears on screen.
let hover = palette.color(ColorRole::Primary).scale_alpha(0.4)?.over(surface);
let text = OpaqueColor::try_from(palette.color(ColorRole::OnSurface))?;
assert!(text.contrast_ratio(surface) >= 4.5);
assert!(hover.contrast_ratio(surface) > 1.0);
# Ok::<(), Box<dyn std::error::Error>>(())
```

Built-in `Muted` supports normal-size text on `Background`, `Surface` and
`SurfaceHover` at a minimum 4.5:1 contrast. Highlighted QuickPick rows use `OnTab`
with `TabSelected` for enabled names and details. Spinner animation multiplies
its dot opacity by the configured `Primary` alpha, including reduced motion.

# Density and measurements

Dark/light mode and density are independent. Standard density is the default.
Compact and Comfortable change spacing and control sizes while retaining text
sizes. Local fixed lengths ignore density; theme role overrides remain live.

```rust
use pert_ui::{Density, prelude::*};
let mut ui = Runtime::new();
let theme = ui.theme();
theme.set_density(Density::Compact)?;
let metrics = theme.metrics(Density::Compact)?
    .with_metric(MetricRole::GroupPadding, Length::from(10))?;
theme.set_metrics(Density::Compact, metrics)?;
ui.mount_tree(Element::new("content",
    Container::new(Element::new("groups",
        Column::new([Element::new("group", Group::titled("Settings",
            Element::new("label", Label::new("Ready"))))])
            .gap(MetricRole::SectionGap)))
        .padding(MetricRole::ContentMargin)))?;
ui.flush(std::time::Duration::ZERO)?;
ui.paint()?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

Custom components read [`MeasureContext::metric`] during layout or [`Painter::metric`]
for paint-only measurements. Changes invalidate just their readers and affected
layout dependents. Hidden components defer work until shown. No timer is added.

# Boundaries, shapes and application zoom

Groups have a rounded border even without a title or focus. [`Group::frameless`]
removes the outline explicitly. RegionLayout regions have a Surface background and
share their dividing line. Fixed boundaries keep that line; resizable boundaries
add a grip, hover feedback and a directional OS cursor. Locking preserves the
allocation. [`Divider`] is a noninteractive separator for ordinary content.
Tabs have a 45-degree upper-right cut, a selected underline joining the continuous
baseline directly beneath the strip, and a separate focus outline. Hit regions remain rectangular and do not overlap adjacent tabs.

Application zoom is independent of density and color mode, and defaults to 100%.
It accepts 75–200%. The desktop host combines it with native DPI automatically,
including pointer/pixel-scroll conversion and sharp text preparation. Physical
window size remains unchanged; content wraps or scrolls in the new logical space.
Zoom and geometry-affecting theme changes cancel active input before relayout.
Selection, stable widget IDs, physical key state and split preferences are retained.
The workspace document captures the zoom; hosts decide when to store it and own
any zoom keyboard shortcuts.

```rust
use pert_ui::{UiZoom, Viewport, prelude::*};
let mut ui = Runtime::new();
ui.set_viewport(Viewport::new(800.0, 600.0, 1.5)?)?; // native size/DPI, before zoom
let settings = ui.ui_settings();
settings.set_zoom(UiZoom::new(1.25)?)?;
ui.mount_tree(Element::new("group", Group::titled("Appearance",
    Element::new("content", Column::new([
        Element::new("label", Label::new("Readable at any density")),
        Element::new("divider", Divider::new()),
        Element::new("reset", Button::new("Reset zoom", move |_| {
            settings.set_zoom(UiZoom::default()).map(|_| ())
        })),
    ])))))?;
ui.flush(std::time::Duration::ZERO)?;
assert_eq!(ui.paint()?.viewport().scale(), 1.875);
# Ok::<(), Box<dyn std::error::Error>>(())
```

# Experimental text coverage

[`UiSettings::set_text_rendering`] chooses the default retained line mask or
phase-aware glyphs composited into a run mask. The phase mode is experimental.
The setting applies to ordinary widget
text on both CPU and GPU, belongs to one runtime, and is not persisted by the
library. Switching repaints text while retaining its layout and caret geometry.

```rust
use pert_ui::{TextRenderingMode, Viewport, prelude::*};
let mut ui = Runtime::new();
ui.set_viewport(Viewport::new(480.0, 120.0, 1.0)?)?;
ui.mount_tree(Element::new("sample", Label::new("Compare text rendering")))?;
let settings = ui.ui_settings();
assert_eq!(settings.text_rendering()?, TextRenderingMode::LineMask);
settings.set_text_rendering(TextRenderingMode::PhaseLine)?;
ui.flush(std::time::Duration::ZERO)?;
let _frame = ui.paint()?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

Custom widgets use the same shape cache, focus and cursor contracts. This complete
example uses a theme size in layout and a focus-sensitive rounded frame. The
runtime caches shape coverage separately from its color, bounded to 128 entries
and 16 MiB. Cached display lists may retain evicted masks. This is UI-thread work.

```rust
use pert_ui::{component::*, prelude::*};
struct CustomPanel;
impl Component for CustomPanel {
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> { Ok(()) }
    fn measure(&self, cx: &MeasureContext<'_>, _: Constraints) -> Result<Size, RuntimeError> {
        Ok(Size::new(180.0, cx.metric(MetricRole::ControlHeight))?)
    }
    fn focusable(&self) -> bool { true }
    fn cursor(&self, _: Point) -> Option<CursorIcon> { Some(CursorIcon::Default) }
    fn paint(&self, painter: &mut Painter) -> Result<(), DrawError> {
        let rect = Rect::from_size(painter.size());
        let radius = Length::new(painter.metric(MetricRole::CornerRadius))?;
        let width = Length::new(painter.metric(MetricRole::BorderWidth))?;
        let role = if painter.focus_within() { ColorRole::Primary } else { ColorRole::PanelBorder };
        painter.fill_shape(rect, Shape::Rounded(radius), painter.color(ColorRole::Surface))?;
        painter.fill_triangle(
            [Point::new(4.0, 4.0)?,
             Point::new(16.0, 10.0)?,
             Point::new(4.0, 16.0)?],
            painter.color(ColorRole::Primary),
        )?;
        painter.stroke_shape(rect, Shape::Rounded(radius), width, painter.color(role))?;
        Ok(())
    }
}
let mut ui = Runtime::new();
ui.set_viewport(Viewport::new(300.0, 100.0, 1.0)?)?;
ui.mount_tree(Element::new("custom", CustomPanel))?;
ui.flush(std::time::Duration::ZERO)?;
ui.paint()?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

# Monospaced values

[`TextStyle::Monospace`] uses bundled Noto Sans Mono at a default 14 logical pixels, with the
same 1.4 line spacing as body text. Ordinary labels still use Noto Sans. No font
installation or runtime download is required. Use this preset for aligned values,
table cells or code; it is a typography choice, not a table widget. A label wraps
at its allocated width, so use separate grid cells for structured tables.

```rust
use pert_ui::{Element, Label, Runtime, TextStyle, Viewport};
use std::time::Duration;
let mut ui = Runtime::new();
ui.set_viewport(Viewport::new(320.0, 100.0, 1.0)?)?;
ui.mount_tree(Element::new("value", Label::new("0012   -1.50")
    .style(TextStyle::Monospace)))?;
ui.flush(Duration::ZERO)?;
let _scene = ui.paint()?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

# Fonts, rich labels and icons

Body/heading text defaults to bundled Noto Sans, monospace to Noto Sans Mono, and
icons to Material Icons Outlined. The text fonts retain their SIL OFL 1.1 licenses;
the icon font and codepoint catalog retain Apache 2.0. The runtime package includes
all asset licenses and provenance. Distributing applications must retain applicable
font notices. No installation, network request or system font discovery is needed.

A runtime owns registrations. [`FontData::from_static`] borrows embedded bytes;
[`FontData::from_owned`] takes bytes loaded by the application. Single-face TTF/OTF
outline fonts are supported, including variable fonts at default axes. Collections,
web fonts and color-only glyphs are unsupported. Registration validates before
changing defaults and returns stable UI-local [`FontId`]s. Equal internal font names
do not overwrite earlier registrations. Up to [`FontManager::MAX_CUSTOM_FONTS`]
custom registrations of at most [`FontData::MAX_BYTES`] each are admitted, within
the [`FontManager::MAX_TOTAL_BYTES`] total including bundled fonts.

This complete example registers a separate copy of the bundled monospace font;
an application can substitute its own [`FontData`] without changing the other calls:

```rust
use pert_ui::{FontData, FontRole, Runtime};

let ui = Runtime::new();
let fonts = ui.fonts();
let alternate = fonts.register(FontData::bundled(FontRole::Monospace))?;
fonts.set_default(FontRole::Body, alternate.clone())?;
assert_eq!(fonts.default_font(FontRole::Heading)?, alternate);
fonts.reset_default(FontRole::Body)?;
# Ok::<(), pert_ui::RuntimeError>(())
```

Heading inherits Body until separately overridden. Explicit segment fonts take
precedence over roles. Ordinary text then falls back to the original bundled text
fonts; remaining missing characters use the missing-glyph mark. Registration alone
does not schedule work. Changed defaults remeasure only affected layout readers;
fixed-geometry [`Painter`] readers repaint. Hidden readers defer work. Handles are
weak, mutations obey the runtime's phase restrictions, and shutdown returns `Closed`.
Registrations remain immutable until shutdown; no unregister API is provided.

One [`RichText`] is a Unicode paragraph with styled segments, shared baseline,
bidirectional shaping and wrapping. Icons use IconSize, and `gap`/icon helpers use
IconGap. Span sizes are logical pixels before DPI and zoom. Preparation limits
include [`TextEditError::MAX_TEXT_BYTES`] of UTF-8, [`RichText::MAX_SEGMENTS`]
segments and the shared raster limits of [`TextLimit`]. Palette
changes recolor semantic spans without reshaping their coverage. Existing plain
and rich captions share one [`TextSource`], which also accepts a binding to
reactive rich content. Text fields continue to edit plain strings.

```rust
use pert_ui::{Button, ColorRole, Column, Element, FontRole, Icon, Label,
              RichText, Runtime, TextSpan, Viewport};
use std::time::Duration;

let mut ui = Runtime::new();
ui.set_viewport(Viewport::new(400.0, 160.0, 1.25)?)?;
let mono = ui.fonts().default_font(FontRole::Monospace)?;
let title = RichText::new().icon(Icon::Settings).gap().text("Settings ")
    .span(TextSpan::new("42.00").font(mono).color_role(ColorRole::Warning));
ui.mount_tree(Element::new("content", Column::new([
    Element::new("title", Label::new(title)),
    Element::new("save", Button::new("Save", |_| Ok(())).leading_icon(Icon::Save)),
])))?;
ui.flush(Duration::ZERO)?;
assert!(!ui.paint()?.masks().is_empty());
# Ok::<(), Box<dyn std::error::Error>>(())
```

Standard icons are semantic roles. Every built-in select/submenu arrow, tab overflow
arrow, checkbox and radio indicator uses the same public font-icon path. Slider
tracks, switch geometry and resize grips remain geometric controls.
[`TextSource`] arguments, [`Choice::rich_label`], [`MenuItem::label`],
[`Tab::rich_title`] and boolean-control `label` accept the same content. Icon-only buttons require an
inspection name; compose [`Tooltip`] to show that name on hover/focus:

```rust
use pert_ui::{Button, Element, Icon, Runtime, Tooltip};

let mut ui = Runtime::new();
ui.mount_tree(Element::new("close", Tooltip::new(
    Element::new("button", Button::icon(Icon::Close, "Close panel", |_| Ok(()))),
    "Close panel",
)))?;
# Ok::<(), pert_ui::RuntimeError>(())
```

An icon font swap needs a codepoint mapping because different fonts use different
characters for the same symbol. `icon_glyph` validates a character in one exact
registered font; no icon fallback occurs. `material_icon` resolves any snake_case
name in the bundled catalog through a setup-time linear lookup. `set_overrides`
atomically replaces all role overrides; omitted roles use their bundled glyphs.
Duplicate roles and foreign fonts reject the entire change.

```rust
use pert_ui::{Icon, Runtime};

let ui = Runtime::new();
let alternate = ui.fonts().material_icon("expand_circle_down")?;
ui.icons().set(Icon::ChevronDown, alternate)?;
// Existing selects and custom uses of ChevronDown follow this mapping.
ui.icons().set_overrides([])?;
# Ok::<(), pert_ui::RuntimeError>(())
```

Custom widgets use [`MeasureContext::rich_text`] when content affects layout,
[`Painter::rich_text`] inside fixed geometry, and [`Painter::icon`] to fit icon ink
uniformly into reserved bounds. All produce ordinary public [`TextLayout`]/mask
commands; no shaping or renderer dependency appears in their signatures.

This custom label uses only the public extension points and retains its measured
layout, just like the built-in label:

```rust
use pert_ui::{ArrangeContext, Component, Constraints, DrawError, Element, Icon,
    MeasureContext, Painter, RichText, Runtime, RuntimeError, Size, TextLayout,
    TextStyle, UpdateContext, Viewport, ColorRole, Point};
use std::{cell::RefCell, time::Duration};

struct Caption {
    content: RichText,
    layout: RefCell<Option<TextLayout>>,
}
impl Component for Caption {
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> { Ok(()) }
    fn measure(&self, cx: &MeasureContext<'_>, space: Constraints) -> Result<Size, RuntimeError> {
        Ok(cx.rich_text(&self.content, space.width(), TextStyle::Body)?.size())
    }
    fn arrange(&self, cx: &mut ArrangeContext<'_>, size: Size) -> Result<(), RuntimeError> {
        *self.layout.borrow_mut() = Some(cx.measurement().rich_text(
            &self.content, Some(size.width()), TextStyle::Body,
        )?);
        Ok(())
    }
    fn paint(&self, painter: &mut Painter) -> Result<(), DrawError> {
        if let Some(layout) = self.layout.borrow().as_ref() {
            layout.paint(painter, Point::ZERO, painter.color(ColorRole::OnSurface))?;
        }
        Ok(())
    }
}
let mut ui = Runtime::new();
ui.set_viewport(Viewport::new(240.0, 80.0, 1.0)?)?;
ui.mount_tree(Element::new("caption", Caption {
    content: RichText::from("Details").leading_icon(Icon::Info),
    layout: RefCell::new(None),
}))?;
ui.flush(Duration::ZERO)?;
assert!(!ui.paint()?.masks().is_empty());
# Ok::<(), Box<dyn std::error::Error>>(())
```

# Images and image formats

[`Image`] places a decoded [`RasterImage`] in the layout with a fit (`Contain`,
`Cover` or `Fill`) and a sampling mode, and [`Painter::draw_image`] draws one from a
custom component. Neither needs a feature: an application that builds pixels in
code or brings its own decoder uses [`RasterImage::from_srgba8`] directly.

Decoding files is opt-in, one Cargo feature per format, so a build carries only
the decoders it asked for; the [feature table](crate#cargo-features) lists them.
None is enabled by default.

Every format feature enables the [`images`] module, whose `decode` takes bytes and
`load` reads a file. The format is recognized from the content, and a recognized
format whose feature is off is reported as [`ImageError::FormatDisabled`] with the
feature to enable. Decoding happens on the calling thread and never in `paint`;
decode large images before they are needed or on a worker, since `RasterImage` is
[`Send`]. Size is checked from the header against the 64 MiB image limit before any
pixel is decoded.

```no_run
# #[cfg(all(feature = "native", feature = "png"))]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use pert_ui::images;
    use pert_ui::{App, Element, Image, ImageFit};
    // Decoded once, before the window opens.
    let logo = images::load("assets/logo.png")?;
    App::new("Images", move |ui| {
        ui.mount_tree(Element::new(
            "logo",
            Image::new(logo)
                .fit(ImageFit::Contain)
                .description("Product logo"),
        ))?;
        Ok(())
    })
    .run()?;
    Ok(())
}
# #[cfg(not(all(feature = "native", feature = "png")))]
# fn main() {}
```

Enable the feature in the application's manifest, for example
`pert-ui = { version = "0.3", features = ["png", "jpeg"] }`.

# SVG graphics

The `svg` feature adds the [`svg`] module, which rasterizes SVG icons and similar
graphics with resvg. Its [`Svg`] component rasterizes for the physical size of its
allocation during layout, never in `paint`, and keeps that result until the size
or the display scale changes. [`Svg::tinted`] turns a monochrome drawing into a
coverage mask colored by a theme role, so it follows theme changes like the
built-in icons; [`Svg::new`] keeps the drawing's own colors. SVG `<text>` is not
rendered: convert text to paths, or place a [`Label`] beside the graphic.

Features follow one placement rule: every widget, including a feature-gated one
such as `App` or [`Svg`], is at the crate root beside the others, while a feature
module such as `images` or [`svg`] holds its loaders, rasterizers and their errors.
