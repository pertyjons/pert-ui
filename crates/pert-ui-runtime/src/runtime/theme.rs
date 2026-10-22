//! UI-local color palettes, density measurements and retained dependencies.
use super::{Node, Runtime, RuntimeError, Shared};
use crate::{Color, Invalidation, OpaqueColor, ReadSignal};
use std::{
    collections::BTreeMap,
    rc::{Rc, Weak},
};

mod seed;

/// Explicit color scheme. Dark is the default; OS preference detection is not implicit.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum ThemeMode {
    /// Dark backgrounds with light foregrounds.
    #[default]
    Dark,
    /// Light backgrounds with dark foregrounds.
    Light,
}

/// Independent spacing preset. Standard is the default, in both color modes.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum Density {
    /// Dense desktop controls with unchanged text size.
    Compact,
    /// Balanced desktop spacing.
    #[default]
    Standard,
    /// Larger controls and more space, with unchanged text size.
    Comfortable,
}
impl Density {
    /// This density's column in a `[Compact, Standard, Comfortable]` table row,
    /// by name, so reordering the variants cannot swap columns.
    const fn column(self) -> usize {
        match self {
            Self::Compact => 0,
            Self::Standard => 1,
            Self::Comfortable => 2,
        }
    }
}

/// One value per density, selected by an exhaustive match.
#[derive(Clone, Copy)]
struct PerDensity<T> {
    compact: T,
    standard: T,
    comfortable: T,
}
impl<T> PerDensity<T> {
    const fn get(&self, density: Density) -> &T {
        match density {
            Density::Compact => &self.compact,
            Density::Standard => &self.standard,
            Density::Comfortable => &self.comfortable,
        }
    }
    fn get_mut(&mut self, density: Density) -> &mut T {
        match density {
            Density::Compact => &mut self.compact,
            Density::Standard => &mut self.standard,
            Density::Comfortable => &mut self.comfortable,
        }
    }
}

/// Semantic theme measurement in logical pixels, before DPI and user zoom.
/// Read through `MeasureContext::metric` for layout or `Painter::metric` for paint.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum MetricRole {
    /// Gap between layout children.
    ItemGap,
    /// Gap between related sections.
    SectionGap,
    /// Space inside a group border.
    GroupPadding,
    /// Opt-in padding at an outer content boundary.
    ContentMargin,
    /// Horizontal control padding on each side.
    ControlPaddingX,
    /// Vertical control padding on each side.
    ControlPaddingY,
    /// Minimum control height; text may require more.
    ControlHeight,
    /// Minimum action button width.
    ControlWidth,
    /// Minimum tab strip height; text may require more.
    TabHeight,
    /// Gap between an icon and its label.
    IconGap,
    /// Default square icon extent.
    IconSize,
    /// Space between tab silhouettes.
    TabGap,
    /// Divider allocation and hit width, retained while locked.
    ResizeWidth,
    /// Panel and separator stroke width.
    BorderWidth,
    /// Keyboard focus stroke width.
    FocusWidth,
    /// Rounded group, button, field and upper tab corner radius.
    CornerRadius,
    /// Pointer lane of an overlay scrollbar along the trailing edge, and the space
    /// one bar reserves where two cross. Wider than the visible thumb, so the bar
    /// stays reachable without enlarging what is drawn.
    ScrollbarLane,
    /// Visible overlay scrollbar thumb thickness across its axis.
    ScrollbarThumb,
    /// Shortest overlay scrollbar thumb along its axis, so long content keeps a
    /// grabbable target; a shorter track still limits it.
    ScrollbarMinimumThumb,
    /// Thickness of a progress bar's track and fill.
    ProgressHeight,
    /// Thickness of a level meter across its axis, and the preferred height of a
    /// trace in multiples of it.
    MeterThickness,
    /// Preferred diameter of a knob's dial.
    KnobDiameter,
    /// Stroke width of a knob's arc and pointer.
    KnobStroke,
    /// Vertical pointer travel that moves a knob across its full range, before
    /// Shift's fine adjustment divides the sensitivity by ten.
    KnobTravel,
    /// Preferred side of a color picker's square saturation/value area.
    ColorAreaSize,
    /// Thickness of a color picker's hue and alpha strips across their axis.
    ColorStripThickness,
    /// Body text size, independent of density.
    BodySize,
    /// Heading text size, independent of density.
    HeadingSize,
    /// Caption text size, independent of density.
    CaptionSize,
    /// Monospace text size, independent of density.
    MonospaceSize,
}

crate::tables::variant_list! {
    const METRIC_ROLES: MetricRole = [
        ItemGap,
        SectionGap,
        GroupPadding,
        ContentMargin,
        ControlPaddingX,
        ControlPaddingY,
        ControlHeight,
        ControlWidth,
        TabHeight,
        IconGap,
        IconSize,
        TabGap,
        ResizeWidth,
        BorderWidth,
        FocusWidth,
        CornerRadius,
        ScrollbarLane,
        ScrollbarThumb,
        ScrollbarMinimumThumb,
        ProgressHeight,
        MeterThickness,
        KnobDiameter,
        KnobStroke,
        KnobTravel,
        ColorAreaSize,
        ColorStripThickness,
        BodySize,
        HeadingSize,
        CaptionSize,
        MonospaceSize,
    ];
}
const METRIC_ROLE_COUNT: usize = METRIC_ROLES.len();
// Metric dependencies are `u64` masks with one bit per role; past 64 roles the
// shift would panic in debug builds and alias another role in release builds.
const _: () = assert!(
    METRIC_ROLE_COUNT <= u64::BITS as usize,
    "metric roles no longer fit one u64 dependency mask"
);
impl MetricRole {
    /// This role's bit in a metric dependency mask.
    pub(crate) const fn mask(self) -> u64 {
        1 << self as usize
    }
}

/// Built-in measurements by role, as `[Compact, Standard, Comfortable]`, which
/// [`Density::column`] selects by name. Rows may appear in any order;
/// `ThemeMetrics::for_density` places each value at its role's index, and the
/// const evaluation below rejects missing or duplicated roles at compile time.
const METRIC_DEFAULTS: [(MetricRole, [f32; 3]); METRIC_ROLE_COUNT] = [
    (MetricRole::ItemGap, [4.0, 8.0, 12.0]),
    (MetricRole::SectionGap, [8.0, 12.0, 16.0]),
    (MetricRole::GroupPadding, [8.0, 12.0, 16.0]),
    (MetricRole::ContentMargin, [8.0, 12.0, 20.0]),
    (MetricRole::ControlPaddingX, [8.0, 12.0, 16.0]),
    (MetricRole::ControlPaddingY, [4.0, 6.0, 10.0]),
    (MetricRole::ControlHeight, [28.0, 32.0, 40.0]),
    (MetricRole::ControlWidth, [56.0, 64.0, 72.0]),
    (MetricRole::TabHeight, [30.0, 36.0, 44.0]),
    (MetricRole::IconGap, [4.0, 6.0, 8.0]),
    (MetricRole::IconSize, [16.0, 16.0, 20.0]),
    (MetricRole::TabGap, [2.0, 2.0, 3.0]),
    (MetricRole::ResizeWidth, [6.0, 8.0, 12.0]),
    (MetricRole::BorderWidth, [1.0, 1.0, 1.0]),
    (MetricRole::FocusWidth, [2.0, 2.0, 2.0]),
    (MetricRole::CornerRadius, [4.0, 4.0, 4.0]),
    (MetricRole::ScrollbarLane, [6.0, 8.0, 12.0]),
    (MetricRole::ScrollbarThumb, [3.0, 4.0, 6.0]),
    (MetricRole::ScrollbarMinimumThumb, [10.0, 12.0, 16.0]),
    (MetricRole::ProgressHeight, [4.0, 6.0, 8.0]),
    (MetricRole::MeterThickness, [8.0, 12.0, 16.0]),
    (MetricRole::KnobDiameter, [32.0, 40.0, 48.0]),
    (MetricRole::KnobStroke, [3.0, 3.0, 4.0]),
    (MetricRole::KnobTravel, [200.0, 200.0, 200.0]),
    (MetricRole::ColorAreaSize, [160.0, 192.0, 224.0]),
    (MetricRole::ColorStripThickness, [12.0, 16.0, 20.0]),
    (MetricRole::BodySize, [14.0, 14.0, 14.0]),
    (MetricRole::HeadingSize, [18.0, 18.0, 18.0]),
    (MetricRole::CaptionSize, [12.0, 12.0, 12.0]),
    (MetricRole::MonospaceSize, [14.0, 14.0, 14.0]),
];
// Evaluates every built-in preset at compile time, so a missing or duplicated
// row fails the build.
const BUILT_IN_METRICS: PerDensity<ThemeMetrics> = PerDensity {
    compact: ThemeMetrics::for_density(Density::Compact),
    standard: ThemeMetrics::for_density(Density::Standard),
    comfortable: ThemeMetrics::for_density(Density::Comfortable),
};
/// Owned, validated measurements for one density, independent of color mode.
/// Copies are detached; install with [`ThemeManager::set_metrics`]. Default is
/// Standard. All values are finite and nonnegative; text sizes must be positive.
/// Large valid measurements can cause layout overflow. Text sizes have no upper
/// limit: text whose size times the effective scale exceeds
/// [`TextError::MAX_FONT_PIXELS`](crate::TextError::MAX_FONT_PIXELS) physical
/// pixels is rasterized at that size and stretched, so it is softer but never
/// rejected for its size alone; the limits named by [`crate::TextLimit`] still
/// apply.
#[derive(Clone, Copy, Debug, PartialEq)]
#[must_use]
pub struct ThemeMetrics {
    values: [f32; METRIC_ROLE_COUNT],
}
impl Default for ThemeMetrics {
    fn default() -> Self {
        Self::for_density(Density::Standard)
    }
}
impl ThemeMetrics {
    /// Built-in measurements. Typography, border and focus widths do not shrink
    /// with density; all values are further scaled by DPI and UI zoom.
    #[expect(
        clippy::missing_panics_doc,
        reason = "the assertions check the fixed default table, which `BUILT_IN_METRICS` \
                  evaluates for every density at compile time, so no caller can reach them"
    )]
    pub const fn for_density(density: Density) -> Self {
        let mut values = [None; METRIC_ROLE_COUNT];
        let mut index = 0;
        while index < METRIC_DEFAULTS.len() {
            let (role, by_density) = METRIC_DEFAULTS[index];
            assert!(
                values[role as usize].is_none(),
                "metric role has two defaults"
            );
            values[role as usize] = Some(by_density[density.column()]);
            index += 1;
        }
        let mut resolved = [0.0; METRIC_ROLE_COUNT];
        index = 0;
        while index < METRIC_ROLE_COUNT {
            resolved[index] = match values[index] {
                Some(value) => value,
                None => panic!("metric role has no default"),
            };
            index += 1;
        }
        Self { values: resolved }
    }
    /// Returns a detached logical-pixel value without registering a dependency.
    pub const fn metric(&self, role: MetricRole) -> f32 {
        self.values[role as usize]
    }
    /// Overrides one measurement. Rejects zero text sizes; other roles accept zero.
    /// `Length` already rejects negative and nonfinite values. See the type's
    /// documentation for how text above
    /// [`TextError::MAX_FONT_PIXELS`](crate::TextError::MAX_FONT_PIXELS) physical
    /// pixels is rasterized.
    ///
    /// # Errors
    /// [`GeometryError::Empty`](crate::GeometryError::Empty) for a zero text size.
    pub fn with_metric(
        mut self,
        role: MetricRole,
        value: crate::Length,
    ) -> Result<Self, crate::GeometryError> {
        if matches!(
            role,
            MetricRole::BodySize
                | MetricRole::HeadingSize
                | MetricRole::CaptionSize
                | MetricRole::MonospaceSize
        ) && value.get() == 0.0
        {
            return Err(crate::GeometryError::Empty);
        }
        self.values[role as usize] = value.get();
        Ok(self)
    }
}

/// A local spacing override: a live theme role or a fixed logical length.
/// Fixed lengths still follow DPI and UI zoom, but not density changes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Spacing {
    /// Resolve this theme measurement whenever layout changes.
    Theme(MetricRole),
    /// Keep this validated logical length across theme and density changes.
    Fixed(crate::Length),
}
impl Spacing {
    pub(crate) fn resolve(self, cx: &crate::MeasureContext<'_>) -> f32 {
        match self {
            Self::Theme(role) => cx.metric(role),
            Self::Fixed(value) => value.get(),
        }
    }
}
impl From<crate::Length> for Spacing {
    fn from(value: crate::Length) -> Self {
        Self::Fixed(value)
    }
}
impl From<MetricRole> for Spacing {
    fn from(value: MetricRole) -> Self {
        Self::Theme(value)
    }
}

/// Semantic color token resolved by [`crate::Painter::color`].
///
/// Accent roles (`Primary`, `Error`, ...) are strong fills for active or
/// selected state and status indication. Their `*Container` counterparts are
/// subdued static fills for noninteractive content; neutral `SurfaceContainer`
/// likewise separates static panels from `Button`. Every `On*` role is paired
/// with its fill: the built-in palettes keep each pair at a WCAG contrast ratio
/// of at least 4.5:1. Color semantics belong in fills and icons; labels on
/// neutral buttons should keep `OnButton`.
///
/// Some roles follow another role by default, as their documentation states and
/// [`ColorRole::follows`] reports: until a follower is overridden itself, it
/// takes its source's current color, including an override of the source.
/// Every other role has its own built-in value per mode. Following copies a
/// color and never computes one: changing an accent does not generate shades and
/// does not change roles with their own value, such as `OnPrimary` or
/// `PrimaryContainer`; customize those when needed.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum ColorRole {
    /// Main accent and active action fill.
    Primary,
    /// Text and icons on Primary.
    OnPrimary,
    /// Supporting accent.
    Secondary,
    /// Text and icons on Secondary.
    OnSecondary,
    /// Error indication. Built-in controls also paint error messages in this
    /// role directly on neutral backgrounds, so the built-in palettes keep it
    /// at 4.5:1 against Background, Surface and SurfaceContainer.
    Error,
    /// Text and icons on Error.
    OnError,
    /// Warning indication.
    Warning,
    /// Text and icons on Warning.
    OnWarning,
    /// Neutral informational accent.
    Info,
    /// Text and icons on Info.
    OnInfo,
    /// Successful outcome indication.
    Success,
    /// Text and icons on Success.
    OnSuccess,
    /// Window background. Text on it uses `OnSurface`.
    Background,
    /// Group and panel background.
    Surface,
    /// Ordinary text on the window background, neutral panels and labels. The
    /// foregrounds of buttons, tabs, selections and the subdued `*Container`
    /// fills follow it by default, so changing it changes ordinary text
    /// everywhere.
    OnSurface,
    /// Hovered neutral surface or divider.
    SurfaceHover,
    /// Supporting and disabled text. Built-in palettes provide normal-text
    /// contrast against Background, Surface and SurfaceHover. Custom palettes
    /// choose their own contrast; this role is also used by active placeholders.
    Muted,
    /// Neutral separator or border.
    Outline,
    /// Keyboard focus indicator, independent of the `Primary` accent.
    ///
    /// The built-in focus ring is drawn inward from the focused element's
    /// bounds at [`MetricRole::FocusWidth`]. A custom component reproduces it
    /// with [`Painter::stroke_shape`](crate::Painter::stroke_shape) on the
    /// same bounds and shape. Thin focusable marks such as split dividers
    /// take this color alone, also while they are dragged. Built-in palettes match
    /// their `Primary`, but overriding `Primary` does not change it.
    FocusRing,
    /// Optional contrast color for custom focus indicators over fills that
    /// resemble `FocusRing`. Built-in controls use a single-color focus ring
    /// and do not draw this role. Built-in palettes match their `OnPrimary`,
    /// but overriding `OnPrimary` does not change it.
    FocusRingContrast,
    /// Resting action control fill.
    Button,
    /// Hovered action control fill.
    ButtonHover,
    /// Text on resting or hovered buttons. Follows `OnSurface` by default.
    OnButton,
    /// Fill of an action control while it is pressed: buttons, inline toggles,
    /// stepper buttons, a chip's remove button and an activatable swatch's
    /// outline. Follows `Primary` by default, so a pressed control takes the
    /// accent; override it when the accent is too close to `Button` for a press
    /// to show.
    ButtonPressed,
    /// Text and icons on `ButtonPressed`. Follows `OnPrimary` by default,
    /// keeping the pressed pair readable wherever `Primary` and `OnPrimary` are.
    OnButtonPressed,
    /// Inactive navigation fill.
    Tab,
    /// Selected navigation fill.
    TabSelected,
    /// Navigation text. Follows `OnSurface` by default.
    OnTab,
    /// Resting border of panels and floating surfaces, visible without
    /// selection or focus. Follows `Divider` by default; override it to frame
    /// floating surfaces more strongly than separators.
    PanelBorder,
    /// Fixed panel and noninteractive content separators.
    Divider,
    /// Resting resize grip, distinct from a fixed separator.
    ResizeHandle,
    /// Hovered resize grip and track.
    ResizeHandleHover,
    /// Static neutral fill for noninteractive content such as table cells, tiles
    /// and region panels. Visually distinct from `Button`, so it does not read as
    /// an action.
    SurfaceContainer,
    /// Text and icons on SurfaceContainer. Follows `OnSurface` by default.
    OnSurfaceContainer,
    /// Static, subdued Primary-tinted fill for highlighted noninteractive content.
    PrimaryContainer,
    /// Text and icons on PrimaryContainer. Follows `OnSurface` by default.
    OnPrimaryContainer,
    /// Static, subdued Secondary-tinted fill.
    SecondaryContainer,
    /// Text and icons on SecondaryContainer. Follows `OnSurface` by default.
    OnSecondaryContainer,
    /// Static, subdued fill for error banners and callouts.
    ErrorContainer,
    /// Text and icons on ErrorContainer. Follows `OnSurface` by default.
    OnErrorContainer,
    /// Static, subdued fill for warning banners and callouts.
    WarningContainer,
    /// Text and icons on WarningContainer. Follows `OnSurface` by default.
    OnWarningContainer,
    /// Static, subdued fill for informational banners and callouts.
    InfoContainer,
    /// Text and icons on InfoContainer. Follows `OnSurface` by default.
    OnInfoContainer,
    /// Static, subdued fill for success banners and callouts.
    SuccessContainer,
    /// Text and icons on SuccessContainer. Follows `OnSurface` by default.
    OnSuccessContainer,
    /// Fill behind selected text in editors and labels.
    ///
    /// Selected text keeps the surrounding foreground weight instead of inverting
    /// to an accent fill, so a selection inside a paragraph stays as readable as
    /// the text around it. Distinct from `Primary`, which marks an active control.
    TextSelection,
    /// Text and icons on TextSelection. Follows `OnSurface` by default, so
    /// selected text keeps the color of the text around it.
    OnTextSelection,
    /// Resting scrollbar thumb. Translucent in the built-in palettes, so the bar
    /// stays in view without covering the content beneath it.
    ScrollbarThumb,
    /// Scrollbar thumb while the pointer is over its bar or dragging it. More
    /// opaque than `ScrollbarThumb` in the built-in palettes, which is how the
    /// bar shows that it can be dragged.
    ScrollbarThumbHover,
    /// Link text on the surrounding `Surface` or `Background`. Distinct from
    /// `OnSurface` so a link reads as navigation, and kept at a WCAG contrast
    /// ratio of at least 4.5:1 on both in the built-in palettes.
    Link,
    /// Matched text inside ordinary text, such as what a search or filter
    /// found, from [`RichText::highlighted`](crate::RichText::highlighted).
    /// Differs from `OnSurface` in hue, and is kept at a WCAG contrast ratio of
    /// at least 4.5:1 on `Surface`, `SurfaceHover` and `Background` in the
    /// built-in palettes.
    Highlight,
}

crate::tables::variant_list! {
    const COLOR_ROLES: ColorRole = [
        Primary,
        OnPrimary,
        Secondary,
        OnSecondary,
        Error,
        OnError,
        Warning,
        OnWarning,
        Info,
        OnInfo,
        Success,
        OnSuccess,
        Background,
        Surface,
        OnSurface,
        SurfaceHover,
        Muted,
        Outline,
        FocusRing,
        FocusRingContrast,
        Button,
        ButtonHover,
        OnButton,
        ButtonPressed,
        OnButtonPressed,
        Tab,
        TabSelected,
        OnTab,
        PanelBorder,
        Divider,
        ResizeHandle,
        ResizeHandleHover,
        SurfaceContainer,
        OnSurfaceContainer,
        PrimaryContainer,
        OnPrimaryContainer,
        SecondaryContainer,
        OnSecondaryContainer,
        ErrorContainer,
        OnErrorContainer,
        WarningContainer,
        OnWarningContainer,
        InfoContainer,
        OnInfoContainer,
        SuccessContainer,
        OnSuccessContainer,
        TextSelection,
        OnTextSelection,
        ScrollbarThumb,
        ScrollbarThumbHover,
        Link,
        Highlight,
    ];
}
const COLOR_ROLE_COUNT: usize = COLOR_ROLES.len();
// Color dependencies are `u64` masks with one bit per role; past 64 roles the
// shift would panic in debug builds and alias another role in release builds.
const _: () = assert!(
    COLOR_ROLE_COUNT <= u64::BITS as usize,
    "color roles no longer fit one u64 dependency mask"
);
impl ColorRole {
    /// This role's bit in a color dependency mask.
    pub(crate) const fn mask(self) -> u64 {
        1 << self as usize
    }
    /// Returns the role this role follows by default, or `None` when it has its
    /// own built-in value in each mode.
    ///
    /// A follower without an override takes its source's current color, so an
    /// override of the source also changes it. A source never follows another
    /// role itself. The result describes the built-in defaults and does not
    /// depend on any palette's overrides.
    ///
    /// ```
    /// # use pert_ui_runtime as pert_ui;
    /// use pert_ui::ColorRole;
    ///
    /// assert_eq!(ColorRole::ButtonPressed.follows(), Some(ColorRole::Primary));
    /// assert_eq!(ColorRole::FocusRing.follows(), None);
    /// ```
    pub const fn follows(self) -> Option<Self> {
        match COLOR_SOURCES[self as usize] {
            Source::Own { .. } => None,
            Source::Follows(source) => Some(source),
        }
    }
}

/// A role's built-in default: its own value per mode, or another role whose
/// current color it takes.
#[derive(Clone, Copy)]
enum Source {
    Own { dark: Color, light: Color },
    Follows(ColorRole),
}
const fn own(dark: Color, light: Color) -> Source {
    Source::Own { dark, light }
}
const fn follows(source: ColorRole) -> Source {
    Source::Follows(source)
}

/// Built-in defaults by role. Rows may appear in any order; [`COLOR_SOURCES`]
/// places each at its role's index, and its const evaluation rejects missing or
/// duplicated roles and a follower whose source follows another role.
#[rustfmt::skip]
const COLOR_DEFAULTS: [(ColorRole, Source); COLOR_ROLE_COUNT] = [
//   role                              dark                    light
    (ColorRole::Primary,              own(Color::ACCENT,          c(0.025, 0.16, 0.55))),
    (ColorRole::OnPrimary,            own(Color::BACKGROUND,      c(1.0, 1.0, 1.0))),
    (ColorRole::Secondary,            own(c(0.45, 0.20, 0.75),    c(0.30, 0.055, 0.46))),
    (ColorRole::OnSecondary,          own(Color::BACKGROUND,      c(1.0, 1.0, 1.0))),
    (ColorRole::Error,                own(c(0.95, 0.30, 0.26),    c(0.55, 0.025, 0.035))),
    (ColorRole::OnError,              own(Color::BACKGROUND,      c(1.0, 1.0, 1.0))),
    (ColorRole::Warning,              own(c(0.95, 0.48, 0.06),    c(0.95, 0.48, 0.06))),
    (ColorRole::OnWarning,            own(Color::BACKGROUND,      c(0.015, 0.02, 0.03))),
    (ColorRole::Info,                 own(c(0.08, 0.48, 0.65),    c(0.02, 0.20, 0.34))),
    (ColorRole::OnInfo,               own(Color::BACKGROUND,      c(1.0, 1.0, 1.0))),
    (ColorRole::Success,              own(c(0.08, 0.55, 0.22),    c(0.02, 0.22, 0.065))),
    (ColorRole::OnSuccess,            own(Color::BACKGROUND,      c(1.0, 1.0, 1.0))),
    (ColorRole::Background,           own(Color::BACKGROUND,      c(0.92, 0.94, 0.97))),
    (ColorRole::Surface,              own(Color::SURFACE,         c(1.0, 1.0, 1.0))),
    (ColorRole::OnSurface,            own(Color::TEXT,            c(0.015, 0.02, 0.03))),
    (ColorRole::SurfaceHover,         own(Color::SURFACE_HOVER,   c(0.78, 0.83, 0.90))),
    (ColorRole::Muted,                own(c(0.57, 0.61, 0.68),    c(0.10, 0.13, 0.17))),
    (ColorRole::Outline,              own(Color::MUTED,           c(0.24, 0.28, 0.34))),
    (ColorRole::FocusRing,            own(Color::ACCENT,          c(0.025, 0.16, 0.55))),
    (ColorRole::FocusRingContrast,    own(Color::BACKGROUND,      c(1.0, 1.0, 1.0))),
    (ColorRole::Button,               own(Color::BUTTON,          c(0.46, 0.58, 0.77))),
    (ColorRole::ButtonHover,          own(Color::BUTTON_HOVER,    c(0.36, 0.48, 0.68))),
    (ColorRole::OnButton,             follows(ColorRole::OnSurface)),
    (ColorRole::ButtonPressed,        follows(ColorRole::Primary)),
    (ColorRole::OnButtonPressed,      follows(ColorRole::OnPrimary)),
    (ColorRole::Tab,                  own(Color::TAB,             c(0.84, 0.87, 0.92))),
    (ColorRole::TabSelected,          own(Color::TAB_SELECTED,    c(0.64, 0.77, 0.95))),
    (ColorRole::OnTab,                follows(ColorRole::OnSurface)),
    (ColorRole::PanelBorder,          follows(ColorRole::Divider)),
    (ColorRole::Divider,              own(c(0.20, 0.24, 0.30),    c(0.30, 0.35, 0.43))),
    (ColorRole::ResizeHandle,         own(c(0.40, 0.46, 0.55),    c(0.16, 0.21, 0.28))),
    (ColorRole::ResizeHandleHover,    own(c(0.60, 0.68, 0.78),    c(0.055, 0.11, 0.20))),
    (ColorRole::SurfaceContainer,     own(c(0.028, 0.034, 0.047), c(0.78, 0.82, 0.88))),
    (ColorRole::OnSurfaceContainer,   follows(ColorRole::OnSurface)),
    (ColorRole::PrimaryContainer,     own(c(0.03, 0.07, 0.16),    c(0.70, 0.80, 1.0))),
    (ColorRole::OnPrimaryContainer,   follows(ColorRole::OnSurface)),
    (ColorRole::SecondaryContainer,   own(c(0.08, 0.035, 0.13),   c(0.85, 0.72, 0.95))),
    (ColorRole::OnSecondaryContainer, follows(ColorRole::OnSurface)),
    (ColorRole::ErrorContainer,       own(c(0.14, 0.02, 0.025),   c(1.0, 0.72, 0.70))),
    (ColorRole::OnErrorContainer,     follows(ColorRole::OnSurface)),
    (ColorRole::WarningContainer,     own(c(0.13, 0.07, 0.01),    c(1.0, 0.85, 0.55))),
    (ColorRole::OnWarningContainer,   follows(ColorRole::OnSurface)),
    (ColorRole::InfoContainer,        own(c(0.02, 0.08, 0.11),    c(0.70, 0.90, 1.0))),
    (ColorRole::OnInfoContainer,      follows(ColorRole::OnSurface)),
    (ColorRole::SuccessContainer,     own(c(0.02, 0.09, 0.04),    c(0.72, 0.95, 0.75))),
    (ColorRole::OnSuccessContainer,   follows(ColorRole::OnSurface)),
    (ColorRole::TextSelection,        own(c(0.03, 0.10, 0.34),    c(0.55, 0.72, 0.98))),
    (ColorRole::OnTextSelection,      follows(ColorRole::OnSurface)),
    (ColorRole::ScrollbarThumb,       own(ca(0.60, 0.68, 0.78, 0.35), ca(0.16, 0.21, 0.28, 0.35))),
    (ColorRole::ScrollbarThumbHover,  own(ca(0.60, 0.68, 0.78, 0.8),  ca(0.16, 0.21, 0.28, 0.8))),
    (ColorRole::Link,                 own(c(0.30, 0.55, 1.0),     c(0.02, 0.12, 0.55))),
    (ColorRole::Highlight,            own(c(0.95, 0.55, 0.08),    c(0.25, 0.08, 0.0))),
];

/// [`COLOR_DEFAULTS`] indexed by role.
const COLOR_SOURCES: [Source; COLOR_ROLE_COUNT] = {
    let mut sources = [None; COLOR_ROLE_COUNT];
    let mut index = 0;
    while index < COLOR_DEFAULTS.len() {
        let (role, source) = COLOR_DEFAULTS[index];
        assert!(
            sources[role as usize].is_none(),
            "color role has two defaults"
        );
        sources[role as usize] = Some(source);
        index += 1;
    }
    let mut resolved = [Source::Follows(ColorRole::Primary); COLOR_ROLE_COUNT];
    index = 0;
    while index < COLOR_ROLE_COUNT {
        resolved[index] = match sources[index] {
            Some(source) => source,
            None => panic!("color role has no default"),
        };
        index += 1;
    }
    index = 0;
    while index < COLOR_ROLE_COUNT {
        if let Source::Follows(source) = resolved[index] {
            assert!(
                matches!(resolved[source as usize], Source::Own { .. }),
                "a followed color role must have its own default"
            );
        }
        index += 1;
    }
    resolved
};

/// Complete, owned color palette with validated linear RGBA values.
///
/// A palette starts from one built-in mode's defaults, [`Palette::dark`] or
/// [`Palette::light`], or from those defaults recolored from seed colors,
/// [`Palette::from_seeds`], and stores an optional override per role. A role
/// without an override resolves to its base value for that mode or, when it
/// follows another role (see [`ColorRole::follows`]), to its source's current
/// color.
/// Defaults to the dark palette. No files, global state, fonts or backend handles
/// are owned. Copies are detached values; install one with [`ThemeManager::set_palette`].
/// Explicit overrides are preserved exactly, including alpha. Following copies a
/// color and never computes one, so applications own contrast choices: an
/// override of a source should still read well on the fills its followers are
/// drawn on.
///
/// Equality compares the starting mode, the seeds and the overrides, not the
/// resolved colors: a palette with an override equal to the default compares
/// unequal to one without it, and so do palettes seeded from different colors
/// that resolve alike.
///
/// ```
/// # use pert_ui_runtime as pert_ui;
/// use pert_ui::{Color, ColorRole, Palette};
///
/// let brand = Color::from_linear(0.6, 0.1, 0.4, 1.0)?;
/// let palette = Palette::dark().with_color(ColorRole::Primary, brand);
/// // ButtonPressed follows Primary; FocusRing keeps its own value.
/// assert_eq!(palette.color(ColorRole::ButtonPressed), brand);
/// assert_eq!(palette.color(ColorRole::FocusRing), Palette::dark().color(ColorRole::FocusRing));
///
/// // An override of the follower itself is kept, and removing it resumes following.
/// let pressed = Color::from_linear(0.3, 0.05, 0.2, 1.0)?;
/// let palette = palette.with_color(ColorRole::ButtonPressed, pressed);
/// assert_eq!(palette.color(ColorRole::ButtonPressed), pressed);
/// let palette = palette.with_default_color(ColorRole::ButtonPressed);
/// assert_eq!(palette.color(ColorRole::ButtonPressed), brand);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
#[must_use]
pub struct Palette {
    mode: ThemeMode,
    seeds: Option<Seeds>,
    overrides: [Option<Color>; COLOR_ROLE_COUNT],
}
/// The seeds a palette was generated from and the base colors they gave.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Seeds {
    primary: OpaqueColor,
    secondary: Option<OpaqueColor>,
    colors: [Color; COLOR_ROLE_COUNT],
}
impl Default for Palette {
    fn default() -> Self {
        Self::dark()
    }
}
impl Palette {
    /// Built-in dark colors, retaining the original neutral control defaults.
    pub const fn dark() -> Self {
        Self::built_in(ThemeMode::Dark)
    }
    /// Built-in light colors with dark text and distinct neutral control fills.
    pub const fn light() -> Self {
        Self::built_in(ThemeMode::Light)
    }
    const fn built_in(mode: ThemeMode) -> Self {
        Self {
            mode,
            seeds: None,
            overrides: [None; COLOR_ROLE_COUNT],
        }
    }
    /// The built-in palette of `mode` recolored from seed colors: an accent,
    /// `primary`, and optionally `secondary` for the `Secondary` roles.
    ///
    /// The accent roles (`Primary`, `FocusRing`, `PrimaryContainer`,
    /// `TextSelection`, `Link` and `TabSelected`) take the seed's hue and the
    /// seed's chroma relative to the built-in `Primary`. The `Secondary` roles
    /// do the same with `secondary`; without it they take the primary seed's
    /// change, keeping the built-in hue offset between the two. Neutral roles
    /// (backgrounds, surfaces, buttons, tabs, outlines, dividers, handles,
    /// scrollbar thumbs, `Muted` and `OnSurface`) take the seed hue at no more
    /// chroma than they have built in, so they are as tinted as the built-in
    /// neutrals are blue (clearly so in the dark palette), and a gray seed
    /// makes them gray. Status colors, their containers, `Highlight`, the `On…`
    /// foregrounds with their own values and `FocusRingContrast` keep their
    /// built-in values, and followers follow as on a built-in palette.
    ///
    /// Every generated color keeps its built-in WCAG relative luminance, and
    /// translucent roles keep their alpha, so every contrast of the built-in
    /// palette between luminances holds for any seed. The seed's own lightness
    /// is therefore not used: a seed sets the hue and colorfulness of the
    /// accent, and the mode sets how light it is. A color that would leave the
    /// sRGB gamut at its luminance loses chroma, not hue. For the exact brand
    /// color, override `Primary` afterwards and choose its foreground. A seed
    /// close to a status hue makes the accent resemble that status, for
    /// example a red seed next to `Error` or a green one next to `Success`;
    /// the palette does not correct that.
    ///
    /// Generation runs once here, on the calling thread, in a bounded number
    /// of steps; resolving a color afterwards costs the same as on a built-in
    /// palette. The manager stores the two modes separately, so seed both.
    ///
    /// ```
    /// # use pert_ui_runtime as pert_ui;
    /// use pert_ui::{Color, ColorRole, OpaqueColor, Palette, Runtime, ThemeMode};
    ///
    /// let brand = OpaqueColor::try_from(Color::from_linear(0.55, 0.05, 0.40, 1.0)?)?;
    /// let ui = Runtime::new();
    /// let theme = ui.theme();
    /// for mode in [ThemeMode::Dark, ThemeMode::Light] {
    ///     theme.set_palette(mode, Palette::from_seeds(mode, brand, None))?;
    /// }
    ///
    /// // The accent keeps the built-in luminance, so its foreground still reads.
    /// let palette = Palette::from_seeds(ThemeMode::Dark, brand, None);
    /// let fill = OpaqueColor::try_from(palette.color(ColorRole::Primary))?;
    /// let text = OpaqueColor::try_from(palette.color(ColorRole::OnPrimary))?;
    /// assert!(text.contrast_ratio(fill) >= 4.5);
    ///
    /// // Removing an override returns a role to its seeded value.
    /// let exact = palette.with_color(ColorRole::Primary, Color::from(brand));
    /// assert_eq!(exact.with_default_color(ColorRole::Primary), palette);
    ///
    /// // Seeds without hue give the same gray accent but compare unequal.
    /// let black = OpaqueColor::try_from(Color::from_linear(0.0, 0.0, 0.0, 1.0)?)?;
    /// let gray = OpaqueColor::try_from(Color::from_linear(0.2, 0.2, 0.2, 1.0)?)?;
    /// let (black, gray) = (
    ///     Palette::from_seeds(ThemeMode::Dark, black, None),
    ///     Palette::from_seeds(ThemeMode::Dark, gray, None),
    /// );
    /// assert_ne!(black, gray);
    /// let [r, g, b, _] = gray.color(ColorRole::Primary).channels();
    /// assert!((r - g).abs() < 1e-3 && (g - b).abs() < 1e-3);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn from_seeds(
        mode: ThemeMode,
        primary: OpaqueColor,
        secondary: Option<OpaqueColor>,
    ) -> Self {
        Self {
            mode,
            seeds: Some(Seeds {
                primary,
                secondary,
                colors: seed::generate(mode, primary, secondary),
            }),
            overrides: [None; COLOR_ROLE_COUNT],
        }
    }
    /// Returns one role's straight-alpha linear color without tracking a dependency:
    /// its override, otherwise its seeded or built-in value for this palette's
    /// mode or the current color of the role it follows.
    pub const fn color(&self, role: ColorRole) -> Color {
        if let Some(color) = self.overrides[role as usize] {
            return color;
        }
        match COLOR_SOURCES[role as usize] {
            Source::Own { dark, light } => match &self.seeds {
                Some(seeds) => seeds.colors[role as usize],
                None => match self.mode {
                    ThemeMode::Dark => dark,
                    ThemeMode::Light => light,
                },
            },
            // A source has its own default, so this recursion is one level deep.
            Source::Follows(source) => self.color(source),
        }
    }
    /// Overrides one role, leaving every other override unchanged. The role keeps
    /// this color even when it follows another role whose color changes later,
    /// and roles that follow it take the new color.
    pub fn with_color(mut self, role: ColorRole, color: Color) -> Self {
        self.overrides[role as usize] = Some(color);
        self
    }
    /// Removes one role's override: a role that follows another resumes
    /// following it, and any other role returns to its seeded or built-in
    /// value for this palette's mode. A role without an override is unchanged.
    pub fn with_default_color(mut self, role: ColorRole) -> Self {
        self.overrides[role as usize] = None;
        self
    }
}

// Only literal validated defaults enter this private const constructor.
const fn c(red: f32, green: f32, blue: f32) -> Color {
    Color::linear_rgb(red, green, blue)
}
/// A translucent palette color, with straight alpha.
const fn ca(red: f32, green: f32, blue: f32, alpha: f32) -> Color {
    Color::linear_rgba(red, green, blue, alpha)
}

pub(super) struct State {
    density: Density,
    metrics: PerDensity<ThemeMetrics>,
    pub(super) metric_subscribers: BTreeMap<u64, Weak<Node>>,
    mode: ThemeMode,
    dark: Palette,
    light: Palette,
    pub(super) subscribers: BTreeMap<u64, (Weak<Node>, u64)>,
}
impl Default for State {
    fn default() -> Self {
        Self {
            density: Density::Standard,
            metrics: BUILT_IN_METRICS,
            metric_subscribers: BTreeMap::new(),
            mode: ThemeMode::Dark,
            dark: Palette::dark(),
            light: Palette::light(),
            subscribers: BTreeMap::new(),
        }
    }
}
impl State {
    pub(crate) fn active_metrics(&self) -> ThemeMetrics {
        *self.metrics.get(self.density)
    }
    pub(super) fn track_metrics(&mut self, node: &Rc<Node>) {
        self.metric_subscribers
            .insert(node.serial, Rc::downgrade(node));
    }
    fn notify_metrics(&mut self, before: ThemeMetrics) {
        let after = self.active_metrics();
        let changed = METRIC_ROLES
            .iter()
            .filter(|role| before.metric(**role) != after.metric(**role))
            .fold(0, |mask, role| mask | role.mask());
        if changed == 0 {
            return;
        }
        self.metric_subscribers.retain(|_, weak| {
            let Some(node) = weak.upgrade() else {
                return false;
            };
            if node
                .metric_layout
                .iter()
                .any(|reads| reads.get() & changed != 0)
            {
                if node.visible.get() {
                    node.window().theme_geometry_change.set(true);
                }
                node.invalidate(Invalidation::Layout);
            } else if node.metric_paint.get() & changed != 0 {
                node.invalidate(Invalidation::Paint);
            }
            true
        });
    }

    pub(super) fn active(&self) -> Palette {
        self.palette(self.mode)
    }
    fn palette(&self, mode: ThemeMode) -> Palette {
        match mode {
            ThemeMode::Dark => self.dark,
            ThemeMode::Light => self.light,
        }
    }
    pub(super) fn subscribe(&mut self, node: &Rc<Node>, roles: u64) {
        if roles == 0 {
            self.subscribers.remove(&node.serial);
        } else {
            self.subscribers
                .insert(node.serial, (Rc::downgrade(node), roles));
        }
    }
    fn notify(&mut self, before: Palette) {
        let after = self.active();
        let changed = COLOR_ROLES
            .iter()
            .filter(|role| before.color(**role) != after.color(**role))
            .fold(0, |mask, role| mask | role.mask());
        if changed == 0 {
            return;
        }
        self.subscribers.retain(|_, (weak, roles)| {
            if let Some(node) = weak.upgrade() {
                if changed & *roles != 0 {
                    node.invalidate(Invalidation::Paint);
                }
                true
            } else {
                false
            }
        });
    }
}

/// Shared UI-thread handle to one runtime's color palettes and density presets.
///
/// Obtain through [`Runtime::theme`]. Clones share configuration, with no global
/// theme or worker/callback-safe endpoint. Setters are allowed in initialization, event
/// callbacks and ordinary external UI updates, but rejected during update, layout,
/// paint or inspection and after runtime shutdown. Changes coalesce; only painters
/// that read changed roles are invalidated. Hidden painters refresh when shown;
/// unmount releases their theme dependencies. Color-only edits retain geometry and text.
/// Metric edits refresh their layout or paint readers; measurement dependencies are
/// recollected independently for limits, measure and arrange callbacks.
/// Mutations compare fixed-size token arrays and visit subscribers on the UI thread.
/// Editing an inactive palette schedules no presentation work.
///
/// ```compile_fail
/// # use pert_ui_runtime as pert_ui;
/// let theme = pert_ui::Runtime::new().theme();
/// std::thread::spawn(move || drop(theme));
/// ```
#[derive(Clone)]
#[must_use]
pub struct ThemeManager {
    shared: Rc<Shared>,
}

/// Shows no theme values: they are read through the manager, which fails after
/// shutdown.
impl std::fmt::Debug for ThemeManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ThemeManager").finish_non_exhaustive()
    }
}
impl ThemeManager {
    /// Current density, independent of dark/light.
    ///
    /// # Errors
    /// Returns [`RuntimeError::Closed`] after shutdown.
    pub fn density(&self) -> Result<Density, RuntimeError> {
        self.shared.readable()?;
        Ok(self.shared.theme.borrow().density)
    }
    /// Copies one density's metrics without subscribing.
    ///
    /// # Errors
    /// Returns [`RuntimeError::Closed`] after shutdown.
    pub fn metrics(&self, density: Density) -> Result<ThemeMetrics, RuntimeError> {
        self.shared.readable()?;
        Ok(*self.shared.theme.borrow().metrics.get(density))
    }
    /// Selects a density, retaining each preset's overrides and the color mode.
    /// Changed layout readers remeasure; paint-only readers repaint. Hidden readers
    /// defer work until shown. Returns whether selection changed.
    ///
    /// # Errors
    /// Returns [`RuntimeError::Closed`] after shutdown, and `WriteDuringUpdate`,
    /// `WriteDuringLayout`, `WriteDuringPaint`, `WriteDuringInspection` or
    /// `WriteDuringDelivery` when called from inside that phase; the configuration is left
    /// unchanged.
    pub fn set_density(&self, density: Density) -> Result<bool, RuntimeError> {
        self.shared.writable()?;
        let mut state = self.shared.theme.borrow_mut();
        if state.density == density {
            return Ok(false);
        }
        let before = state.active_metrics();
        state.density = density;
        state.notify_metrics(before);
        drop(state);
        self.shared.settings_signals.density.publish(density);
        Ok(true)
    }
    /// Observes the selected density: a read-only signal that every change
    /// through [`Self::set_density`], including one made by
    /// [`Runtime::restore_workspace`], updates.
    ///
    /// A component reading it in `update` is invalidated when the selection
    /// changes, like any signal, so a menu radio row can follow it. Components
    /// that only draw with the active metrics need not read it: the painter
    /// already subscribes them to what they use. Clones and later calls share
    /// one signal while any of them is alive, and the runtime publishes
    /// nothing while none is.
    ///
    /// # Errors
    /// Returns `Closed` after shutdown.
    pub fn density_binding(&self) -> Result<ReadSignal<Density>, RuntimeError> {
        self.shared.readable()?;
        let density = self.shared.theme.borrow().density;
        Ok(self
            .shared
            .settings_signals
            .density
            .read(&self.shared, density))
    }
    /// Replaces one density's validated metrics. Inactive presets schedule no work.
    /// Supply `ThemeMetrics::for_density` to reset a preset. Returns whether stored
    /// values changed.
    ///
    /// # Errors
    /// Returns [`RuntimeError::Closed`] after shutdown, and `WriteDuringUpdate`,
    /// `WriteDuringLayout`, `WriteDuringPaint`, `WriteDuringInspection` or
    /// `WriteDuringDelivery` when called from inside that phase; the stored metrics are
    /// left unchanged.
    pub fn set_metrics(
        &self,
        density: Density,
        metrics: ThemeMetrics,
    ) -> Result<bool, RuntimeError> {
        self.shared.writable()?;
        let mut state = self.shared.theme.borrow_mut();
        if *state.metrics.get(density) == metrics {
            return Ok(false);
        }
        let before = state.active_metrics();
        *state.metrics.get_mut(density) = metrics;
        state.notify_metrics(before);
        Ok(true)
    }

    /// Reads the selected mode without subscribing.
    ///
    /// # Errors
    /// Returns [`RuntimeError::Closed`] after shutdown.
    pub fn mode(&self) -> Result<ThemeMode, RuntimeError> {
        self.shared.readable()?;
        Ok(self.shared.theme.borrow().mode)
    }
    /// Copies a mode's palette without subscribing.
    ///
    /// # Errors
    /// Returns [`RuntimeError::Closed`] after shutdown.
    pub fn palette(&self, mode: ThemeMode) -> Result<Palette, RuntimeError> {
        self.shared.readable()?;
        Ok(self.shared.theme.borrow().palette(mode))
    }
    /// Switches mode, preserving both palettes and their overrides.
    /// Returns whether the mode changed, even if the palettes have equal colors.
    ///
    /// # Errors
    /// Returns [`RuntimeError::Closed`] after shutdown, and `WriteDuringUpdate`,
    /// `WriteDuringLayout`, `WriteDuringPaint`, `WriteDuringInspection` or
    /// `WriteDuringDelivery` when called from inside that phase; failure leaves state
    /// intact.
    pub fn set_mode(&self, mode: ThemeMode) -> Result<bool, RuntimeError> {
        self.shared.writable()?;
        let mut state = self.shared.theme.borrow_mut();
        if state.mode == mode {
            return Ok(false);
        }
        let before = state.active();
        state.mode = mode;
        self.notify(&mut state, before);
        drop(state);
        self.shared.settings_signals.theme_mode.publish(mode);
        Ok(true)
    }
    /// Observes the selected color mode as a read-only signal that every
    /// change through [`Self::set_mode`], including one made by
    /// [`Runtime::restore_workspace`], updates. Sharing, cost and
    /// invalidation follow [`Self::density_binding`].
    ///
    /// # Errors
    /// Returns `Closed` after shutdown.
    pub fn mode_binding(&self) -> Result<ReadSignal<ThemeMode>, RuntimeError> {
        self.shared.readable()?;
        let mode = self.shared.theme.borrow().mode;
        Ok(self
            .shared
            .settings_signals
            .theme_mode
            .read(&self.shared, mode))
    }
    /// Replaces one mode's complete palette. Returns whether its stored value changed.
    /// To reset overrides, supply [`Palette::dark`] or [`Palette::light`].
    ///
    /// # Errors
    /// Returns [`RuntimeError::Closed`] after shutdown, and `WriteDuringUpdate`,
    /// `WriteDuringLayout`, `WriteDuringPaint`, `WriteDuringInspection` or
    /// `WriteDuringDelivery` when called from inside that phase; failure leaves state
    /// intact.
    pub fn set_palette(&self, mode: ThemeMode, palette: Palette) -> Result<bool, RuntimeError> {
        self.shared.writable()?;
        let mut state = self.shared.theme.borrow_mut();
        if state.palette(mode) == palette {
            return Ok(false);
        }
        let before = state.active();
        match mode {
            ThemeMode::Dark => state.dark = palette,
            ThemeMode::Light => state.light = palette,
        }
        self.notify(&mut state, before);
        Ok(true)
    }
    /// Overrides one role in one mode; other values and the active mode are preserved.
    /// Returns whether the stored color changed. As with [`Palette::with_color`],
    /// the role then keeps this color instead of following its source, and roles
    /// that follow it take it; remove the override with
    /// [`Palette::with_default_color`] and [`Self::set_palette`].
    /// No automatic contrast adjustment occurs.
    ///
    /// # Errors
    /// As [`Self::set_palette`]; failure leaves state intact.
    pub fn set_color(
        &self,
        mode: ThemeMode,
        role: ColorRole,
        color: Color,
    ) -> Result<bool, RuntimeError> {
        self.set_palette(mode, self.palette(mode)?.with_color(role, color))
    }
    fn notify(&self, state: &mut State, before: Palette) {
        if before.color(ColorRole::Background) != state.active().color(ColorRole::Background) {
            for window in self
                .shared
                .windows
                .borrow()
                .iter()
                .filter_map(Weak::upgrade)
            {
                window.scene_dirty.set(true);
            }
        }
        state.notify(before);
    }
}
impl Runtime {
    /// Returns the runtime's shared theme manager, initially dark with built-in palettes.
    /// Retain a clone in event callbacks to change modes or role overrides at runtime.
    pub fn theme(&self) -> ThemeManager {
        ThemeManager {
            shared: self.shared.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every role's default source, so an unintended follower fails as well as
    /// a missing one. The facade's theme tests cover resolution and overrides.
    #[test]
    fn exactly_the_documented_roles_follow_their_sources() {
        use ColorRole::*;
        let followers = [
            (OnButton, OnSurface),
            (OnTab, OnSurface),
            (OnTextSelection, OnSurface),
            (OnSurfaceContainer, OnSurface),
            (OnPrimaryContainer, OnSurface),
            (OnSecondaryContainer, OnSurface),
            (OnErrorContainer, OnSurface),
            (OnWarningContainer, OnSurface),
            (OnInfoContainer, OnSurface),
            (OnSuccessContainer, OnSurface),
            (PanelBorder, Divider),
            (ButtonPressed, Primary),
            (OnButtonPressed, OnPrimary),
        ];
        for role in COLOR_ROLES {
            let expected = followers
                .iter()
                .find(|(follower, _)| *follower == role)
                .map(|(_, source)| *source);
            assert_eq!(role.follows(), expected, "{role:?}");
        }
    }
}
