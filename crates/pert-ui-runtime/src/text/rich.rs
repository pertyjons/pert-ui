//! Owned inline label content with inherited typography.
use crate::{Color, ColorRole, FontId, FontRole, IconSource, Length};

/// Text or icon tint. Inherited tint is supplied when painting the complete layout.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum TextColor {
    /// Use the containing label/control's current foreground, including disabled state.
    #[default]
    Inherit,
    /// Follow a semantic theme role; changes invalidate only paint.
    Role(ColorRole),
    /// Preserve a literal linear color across theme changes.
    Literal(Color),
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum FontChoice {
    Exact(FontId),
    Role(FontRole),
}

/// One owned text segment. Unspecified properties inherit the surrounding text style.
/// Adjacent segments form one Unicode paragraph; segment boundaries are not line breaks.
///
/// A span is a UI-thread value, neither `Send` nor `Sync`, because it can
/// name a registered font ([`FontId`]), which belongs to its runtime's font
/// registry. A worker returns plain text or its own data, and the UI thread
/// builds the span.
#[derive(Clone, Debug, PartialEq)]
#[must_use]
pub struct TextSpan {
    pub(crate) text: String,
    pub(crate) font: Option<FontChoice>,
    pub(crate) size: Option<Length>,
    pub(crate) color: TextColor,
}
impl TextSpan {
    /// Creates text with inherited font, size and color.
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            font: None,
            size: None,
            color: TextColor::Inherit,
        }
    }
    /// Selects one immutable registered font, followed by bundled text fallback.
    pub fn font(mut self, font: FontId) -> Self {
        self.font = Some(FontChoice::Exact(font));
        self
    }
    /// Follows a runtime font role, including later overrides.
    pub fn font_role(mut self, role: FontRole) -> Self {
        self.font = Some(FontChoice::Role(role));
        self
    }
    /// Sets logical font size before DPI/zoom. Zero is rejected during preparation.
    pub fn size(mut self, size: Length) -> Self {
        self.size = Some(size);
        self
    }
    /// Preserves a literal tint across theme changes.
    pub fn color(mut self, color: Color) -> Self {
        self.color = TextColor::Literal(color);
        self
    }
    /// Follows a theme tint, replacing any literal override.
    pub fn color_role(mut self, role: ColorRole) -> Self {
        self.color = TextColor::Role(role);
        self
    }
}
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Inline {
    Text(TextSpan),
    Icon(IconSource, TextColor),
    Gap,
}
/// Owned display text composed of styled text segments and font icons.
/// Plain strings convert to one inherited segment. Icons use the themed IconSize;
/// `gap` uses IconGap. Inline icon ink is centered on the nearest text run's
/// capital height on the same visual line (70% of font size when unavailable),
/// constrained to that line's bounds. Icon-only lines center their ink vertically.
/// Icons reserve their nominal size without text leading; surrounding text retains
/// its normal line height. Default icons therefore do not enlarge buttons or tabs.
/// Larger custom icon sizes may increase the line height.
/// Preparation admits at most
/// [`TextEditError::MAX_TEXT_BYTES`](crate::TextEditError::MAX_TEXT_BYTES) of UTF-8
/// and [`Self::MAX_SEGMENTS`] segments.
/// Rich labels are display content; editable text fields continue to edit plain strings.
///
/// Rich text is a UI-thread value, neither `Send` nor `Sync`, because a span
/// can name a registered font ([`FontId`]). A worker, for example the job of
/// [`Runtime::spawn`](crate::Runtime::spawn), returns plain text or its own
/// data, and the UI thread builds the `RichText`.
#[derive(Clone, Debug, Default, PartialEq)]
#[must_use]
pub struct RichText {
    pub(crate) parts: Vec<Inline>,
}
impl RichText {
    /// The most segments one rich text may have when it is prepared: 1,024. Each
    /// text, span, icon and gap is one segment, including the gap that
    /// [`Self::leading_icon`] and [`Self::trailing_icon`] insert. Content with more is not
    /// rejected while it is built; preparing it fails with
    /// [`TextError::TooLarge`](crate::TextError::TooLarge) holding
    /// [`TextLimit::Segments`](crate::TextLimit::Segments).
    pub const MAX_SEGMENTS: usize = 1024;
    /// Creates empty content; append text, spans and icons in logical reading order.
    pub fn new() -> Self {
        Self::default()
    }
    /// Appends ordinary text with inherited typography.
    pub fn text(self, text: impl Into<String>) -> Self {
        self.span(TextSpan::new(text))
    }
    /// Appends a styled segment without inserting whitespace or a line break.
    /// Empty text contributes no glyphs. Its font and size are still validated;
    /// the first text segment still supplies the paragraph's default typography.
    pub fn span(mut self, span: TextSpan) -> Self {
        self.parts.push(Inline::Text(span));
        self
    }
    /// Appends an icon with the containing control's foreground.
    pub fn icon(self, icon: impl Into<IconSource>) -> Self {
        self.tinted_icon(icon, TextColor::Inherit)
    }
    /// Appends an icon with an explicit inherited, semantic or literal tint.
    pub fn tinted_icon(mut self, icon: impl Into<IconSource>, color: TextColor) -> Self {
        self.parts.push(Inline::Icon(icon.into(), color));
        self
    }
    /// Inserts one theme-controlled icon gap. No padding characters are required.
    pub fn gap(mut self) -> Self {
        self.parts.push(Inline::Gap);
        self
    }
    /// Prepends an icon and themed gap. An empty label receives no gap.
    pub fn leading_icon(mut self, icon: impl Into<IconSource>) -> Self {
        if self.parts.iter().any(|part| match part {
            Inline::Text(span) => !span.text.is_empty(),
            Inline::Icon(..) => true,
            Inline::Gap => false,
        }) {
            self.parts.insert(0, Inline::Gap);
        }
        self.parts
            .insert(0, Inline::Icon(icon.into(), TextColor::Inherit));
        self
    }
    /// Appends an icon and themed gap. An empty label receives no gap.
    pub fn trailing_icon(mut self, icon: impl Into<IconSource>) -> Self {
        if self.parts.iter().any(|part| match part {
            Inline::Text(span) => !span.text.is_empty(),
            Inline::Icon(..) => true,
            Inline::Gap => false,
        }) {
            self.parts.push(Inline::Gap);
        }
        self.icon(icon)
    }
    /// Plain textual description for inspection. Icons and layout gaps are omitted.
    /// An icon-only action must supply its own human-readable action name.
    pub fn plain_text(&self) -> String {
        self.parts
            .iter()
            .filter_map(|part| {
                if let Inline::Text(span) = part {
                    Some(span.text.as_str())
                } else {
                    None
                }
            })
            .collect()
    }
    pub(crate) fn as_plain(&self) -> Option<&str> {
        match self.parts.as_slice() {
            [Inline::Text(span)]
                if span.font.is_none()
                    && span.size.is_none()
                    && span.color == TextColor::Inherit =>
            {
                Some(&span.text)
            }
            _ => None,
        }
    }
    // Unused dimensions do not participate in resolved geometry or its cache key.
    pub(crate) fn metrics(&self, mut metric: impl FnMut(crate::MetricRole) -> f32) -> (f32, f32) {
        let size = if self.has_icons() {
            metric(crate::MetricRole::IconSize)
        } else {
            0.0
        };
        let gap = if self.has_gaps() {
            metric(crate::MetricRole::IconGap)
        } else {
            0.0
        };
        (size, gap)
    }
    pub(crate) fn has_icons(&self) -> bool {
        self.parts.iter().any(|p| matches!(p, Inline::Icon(..)))
    }
    pub(crate) fn has_gaps(&self) -> bool {
        self.parts.iter().any(|p| matches!(p, Inline::Gap))
    }
}
impl From<String> for RichText {
    fn from(text: String) -> Self {
        Self::new().text(text)
    }
}
impl From<&str> for RichText {
    fn from(text: &str) -> Self {
        Self::new().text(text)
    }
}

// All embedded floats are validated finite quantities; equality is reflexive.
impl Eq for TextColor {}
impl Eq for TextSpan {}
impl Eq for Inline {}
impl Eq for RichText {}
