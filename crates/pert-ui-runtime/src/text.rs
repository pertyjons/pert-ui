//! Runtime-owned font shaping and retained coverage, without window/GPU services.

mod fit;
mod fonts;
mod icons;
mod registry;
mod rich;
use crate::{Color, DrawError, MaskImage, Painter, Rect, RuntimeError, Size};
pub use fit::{FittedText, HighlightError, TextOverflow};
pub use fonts::{FontData, FontError, FontId, FontRole, IconGlyph};
pub use icons::{Icon, IconSource};
pub use rich::{RichText, TextColor, TextSpan};
use std::{
    cell::{Cell, RefCell},
    collections::{BTreeMap, HashMap, VecDeque},
    rc::Rc,
    sync::Arc,
};

/// [`TextLimit::MAX_RASTER_SIDE`] in the signed arithmetic of canvas bounds.
const MAX_SIDE: i64 = TextLimit::MAX_RASTER_SIDE as i64;
/// The raster work of one prepared text: the glyph bitmaps it reads and the mask
/// pixels it writes are each bounded by this many bytes, the same as one mask.
const RASTER_BYTES: usize = MaskImage::MAX_PIXELS;
/// The prepared-text cache budget in bytes, a policy separate from mask size.
const CACHE_BYTES: usize = 16 * 1024 * 1024;
/// Keep enough phase-specific lines for several scrolling frames while the
/// coverage budget remains the primary memory limit.
const PHASE_LINE_ENTRIES: usize = 2048;
/// [`TextError::MAX_FONT_PIXELS`] in the floating-point arithmetic of scales.
pub(crate) const MAX_FONT_PIXELS: f64 = TextError::MAX_FONT_PIXELS as f64;

/// `scale`, lowered so the largest of `sizes` (logical pixels) is rasterized at
/// no more than [`MAX_FONT_PIXELS`]. The same sizes give the same scale, so
/// cache keys stay stable.
fn effective_scale(scale: f64, sizes: impl IntoIterator<Item = f32>) -> f64 {
    let largest = f64::from(sizes.into_iter().fold(0.0, f32::max));
    if largest > 0.0 && scale * largest > MAX_FONT_PIXELS {
        MAX_FONT_PIXELS / largest
    } else {
        scale
    }
}

/// A bounded text request could not be prepared.
///
/// Text longer than
/// [`TextEditError::MAX_TEXT_BYTES`](crate::TextEditError::MAX_TEXT_BYTES) is
/// rejected before preparation with
/// [`TextEditError::TooLarge`](crate::TextEditError::TooLarge), the one error of
/// that limit.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum TextError {
    /// A bundled font or a font used during rasterization could not be read.
    #[error("font could not be read")]
    Font,
    /// Preparing the text would exceed the named limit.
    ///
    /// Font size is not limited here: text whose size times the effective scale
    /// (DPI scale, UI zoom and any view magnification) exceeds
    /// [`Self::MAX_FONT_PIXELS`] physical pixels is rasterized at that size and
    /// its coverage stretched, so it is softer but never rejected for its size
    /// alone. The other limits are not lowered that way: see [`TextLimit`].
    #[error("text preparation exceeds its limit of {0}")]
    TooLarge(TextLimit),
    /// Width/scale is invalid, font size is zero, or geometry is not finite.
    #[error("invalid text geometry")]
    Geometry,
}
impl TextError {
    /// The largest font size, in physical pixels, that text and icons are
    /// rasterized at: 256.
    ///
    /// A larger size times effective scale is not an error. Preparation lowers
    /// the raster scale so the largest size in the text reaches exactly this
    /// value and the coverage is stretched where it is drawn, so the result is
    /// softer. [`TextLayout`] records the scale it used.
    pub const MAX_FONT_PIXELS: u32 = 256;
}
/// A preparation limit that [`TextError::TooLarge`] names.
///
/// `Display` writes the limit with its value, such as `1024 rich-text segments`.
///
/// [`TextLimit::RasterSide`] and [`TextLimit::Coverage`] are measured in
/// physical pixels, on text laid out at the effective scale: the DPI scale times
/// the UI zoom times any view magnification. Unlike a font size above
/// [`TextError::MAX_FONT_PIXELS`], crossing them does not lower the scale, so
/// text that fits at 100% zoom or at a low DPI, such as a long unwrapped label,
/// can be rejected at a higher zoom or DPI. Wrap long text with a width, or keep
/// it short enough to fit at the largest zoom offered.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum TextLimit {
    /// More than [`RichText::MAX_SEGMENTS`] segments in one rich text.
    Segments,
    /// A prepared text wider or taller than [`Self::MAX_RASTER_SIDE`] physical
    /// pixels, including an inline gap wider than that.
    RasterSide,
    /// More than [`MaskImage::MAX_PIXELS`] bytes (16 MiB) of coverage, or of the
    /// temporary glyph rasters read to compose it.
    Coverage,
}
impl TextLimit {
    /// The widest and tallest one prepared text may be, in physical pixels at
    /// its effective scale: 8192. Exceeding it is [`TextLimit::RasterSide`].
    pub const MAX_RASTER_SIDE: u32 = 8192;
}
impl std::fmt::Display for TextLimit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Segments => write!(f, "{} rich-text segments", RichText::MAX_SEGMENTS),
            Self::RasterSide => write!(f, "{} physical pixels per side", Self::MAX_RASTER_SIDE),
            Self::Coverage => write!(f, "{} MiB of coverage", RASTER_BYTES / (1024 * 1024)),
        }
    }
}

/// Semantic typography choices; ordinary labels use `Body`. ThemeMetrics owns
/// their logical font sizes. All three densities share the same default sizes;
/// DPI and UI zoom scale them together. Line spacing remains 1.4.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum TextStyle {
    /// BodySize theme role, default 14 logical pixels.
    #[default]
    Body,
    /// HeadingSize theme role, default 18 logical pixels.
    Heading,
    /// CaptionSize theme role, default 12 logical pixels.
    Caption,
    /// MonospaceSize theme role, default 14 logical-pixel Noto Sans Mono, for aligned
    /// values, tables and code. FontRole::Monospace can override the bundled default;
    /// no system discovery occurs. With the bundled font,
    /// supported ordinary letters, digits and spaces have equal advances; combining
    /// marks and complex scripts are shaped normally, not forced into character cells.
    Monospace,
    /// Secondary monospaced text: bundled Noto Sans Mono at CaptionSize
    /// (12 logical pixels by default). Digits, signs, decimal points and spaces
    /// have equal advances; ordinary shaping and bounded coverage still apply.
    MonospaceCaption,
}
impl TextStyle {
    pub(crate) fn metric(self) -> crate::MetricRole {
        match self {
            Self::Body => crate::MetricRole::BodySize,
            Self::Heading => crate::MetricRole::HeadingSize,
            Self::Caption | Self::MonospaceCaption => crate::MetricRole::CaptionSize,
            Self::Monospace => crate::MetricRole::MonospaceSize,
        }
    }

    #[cfg(test)]
    fn pixels(self) -> f32 {
        match self {
            Self::Body | Self::Monospace => 14.0,
            Self::Heading => 18.0,
            Self::Caption | Self::MonospaceCaption => 12.0,
        }
    }
    pub(crate) fn font_role(self) -> crate::FontRole {
        match self {
            Self::Heading => crate::FontRole::Heading,
            Self::Monospace | Self::MonospaceCaption => crate::FontRole::Monospace,
            _ => crate::FontRole::Body,
        }
    }
}

/// Owned measured text and immutable coverage, independent of its renderer.
/// Produced by public text/rich-text preparation. Clones share masks and shaped
/// geometry, and can outlive cache eviction. Reprepare after content, width, scale,
/// font defaults or style changes; component measurement tracks these dependencies.
/// Rich span colors are resolved at paint time. Text fallback uses bundled fonts;
/// missing characters use the missing-glyph mark. Icons have an exact font mapping.
#[derive(Clone, Debug)]
pub struct TextLayout {
    size: Size,
    parts: Arc<[MaskPart]>,
    colors: Arc<[TextColor]>,
    ink: Option<[f32; 4]>,
    pub(crate) shaped: Arc<parley::Layout<usize>>,
    pub(crate) scale: f32,
    pub(crate) origin: [f32; 2],
    signature: Arc<RasterSignature>,
}
#[derive(Clone, Debug)]
struct MaskPart {
    image: MaskImage,
    offset: [f32; 2],
    brush: usize,
}
impl TextLayout {
    /// Logical extent including glyph overhangs.
    #[must_use]
    pub const fn size(&self) -> Size {
        self.size
    }
    // First-line typographic extent above and below its baseline. Unlike ink
    // bounds, this stays stable when the visible glyphs change.
    pub(crate) fn first_line_box(&self) -> Option<(f32, f32)> {
        let metrics = *self.shaped.lines().next()?.metrics();
        Some((
            metrics.baseline / self.scale,
            (metrics.line_height - metrics.baseline) / self.scale,
        ))
    }
    // Baseline in the same cropped logical coordinates used by `paint`.
    pub(crate) fn first_baseline(&self) -> Option<f32> {
        self.first_line_box()
            .map(|(above, _)| above + self.origin[1])
    }
    pub(crate) fn paint_icon(
        &self,
        painter: &mut Painter,
        rect: Rect,
        color: Color,
        nominal_scale: f32,
        turn: crate::QuarterTurn,
    ) -> Result<(), DrawError> {
        let [x, y, w, h] = rect.values();
        let Some([left, top, right, bottom]) = self.ink else {
            return Ok(());
        };
        if right <= left || bottom <= top {
            return Ok(());
        }
        // Lay the ink out upright in the extent the turn will occupy, then place
        // each part where the turn puts it.
        let upright = turn.turned(Size::new(w, h)?);
        let (w, h) = (upright.width(), upright.height());
        let factor = (w / (right - left))
            .min(h / (bottom - top))
            .min(nominal_scale);
        painter.with_clip(rect, |painter| {
            for p in self.parts.iter() {
                let part = turn.place(
                    upright,
                    Rect::new(
                        (w - (right - left) * factor) / 2.0 + (p.offset[0] - left) * factor,
                        (h - (bottom - top) * factor) / 2.0 + (p.offset[1] - top) * factor,
                        p.image.width() as f32 / self.scale * factor,
                        p.image.height() as f32 / self.scale * factor,
                    )?,
                )?;
                painter.draw_mask_turned(
                    &p.image,
                    Rect::new(x + part.x(), y + part.y(), part.width(), part.height())?,
                    color,
                    turn,
                );
            }
            Ok::<(), DrawError>(())
        })?;
        Ok(())
    }
    /// Records this layout at a local logical origin. `color` supplies inherited
    /// foreground; explicit span colors are preserved. Theme span colors subscribe
    /// the painting component.
    ///
    /// # Errors
    /// As [`Self::paint_turned`].
    pub fn paint(
        &self,
        painter: &mut Painter,
        x: f32,
        y: f32,
        color: Color,
    ) -> Result<(), DrawError> {
        self.paint_turned(painter, x, y, crate::QuarterTurn::None, color)
    }
    /// Records this layout turned by a quarter turn, with the turned text's
    /// top-left at the local origin `(x, y)`.
    ///
    /// The turned text occupies [`QuarterTurn::turned`](crate::QuarterTurn::turned)
    /// of [`Self::size`], so a line turned [`Left`](crate::QuarterTurn::Left) is as
    /// wide as the line was tall and reads upwards, as a vertical axis label does.
    /// The same glyph coverage is drawn; nothing is shaped or rasterized again.
    /// Colors resolve as for [`Self::paint`].
    ///
    /// # Errors
    /// Returns [`DrawError::Runtime`] holding [`RuntimeError::Geometry`] with
    /// [`GeometryError::NonFinite`](crate::GeometryError::NonFinite) for a non-finite
    /// origin or a turned placement that is not finite.
    pub fn paint_turned(
        &self,
        painter: &mut Painter,
        x: f32,
        y: f32,
        turn: crate::QuarterTurn,
        color: Color,
    ) -> Result<(), DrawError> {
        Rect::new(x, y, self.size.width(), self.size.height())?;
        painter.text_drawn.set(true);
        let phase_parts = if turn == crate::QuarterTurn::None
            && painter.text_rendering != crate::TextRenderingMode::LineMask
            && (painter.physical_scale - f64::from(self.scale)).abs() < 0.001
        {
            let physical = [
                painter.physical_origin[0] + f64::from(x) * painter.physical_scale,
                painter.physical_origin[1] + f64::from(y) * painter.physical_scale,
            ];
            let snapped = physical.map(|value| (value * 4.0).round() * 0.25);
            let anchor = snapped.map(f64::floor);
            let phase = [
                ((snapped[0] - anchor[0]) * 4.0) as u8,
                ((snapped[1] - anchor[1]) * 4.0) as u8,
            ];
            let text = painter.text.clone();
            match text.with_service(|service| Ok(service.phase_parts(self, phase)?)) {
                Ok(parts) => Some((
                    parts,
                    [
                        ((anchor[0] - physical[0]) / painter.physical_scale) as f32,
                        ((anchor[1] - physical[1]) / painter.physical_scale) as f32,
                    ],
                )),
                // Phase coverage can be one pixel larger than the already
                // prepared mask. Retain valid text at the raster limit.
                Err(RuntimeError::Text(TextError::TooLarge(_))) => None,
                Err(error) => return Err(error.into()),
            }
        } else {
            None
        };
        let (parts, shift) = phase_parts
            .as_ref()
            .map_or((self.parts.as_ref(), [0.0, 0.0]), |(parts, shift)| {
                (parts.as_ref(), *shift)
            });
        for part in parts {
            let tint = match self.colors[part.brush] {
                TextColor::Inherit => color,
                TextColor::Literal(c) => c,
                TextColor::Role(r) => painter.color(r),
            };
            let placed = turn.place(
                self.size,
                Rect::new(
                    part.offset[0],
                    part.offset[1],
                    part.image.width() as f32 / self.scale,
                    part.image.height() as f32 / self.scale,
                )?,
            )?;
            painter.draw_mask_turned(
                &part.image,
                Rect::new(
                    x + placed.x() + shift[0],
                    y + placed.y() + shift[1],
                    placed.width(),
                    placed.height(),
                )?,
                tint,
                turn,
            );
        }
        Ok(())
    }
}
#[derive(Clone, PartialEq)]
struct ResolvedSpan {
    range: std::ops::Range<usize>,
    font: usize,
    size: f32,
    brush: usize,
    icon: bool,
}
#[derive(Clone, PartialEq)]
struct Resolved {
    text: String,
    spans: Vec<ResolvedSpan>,
    gaps: Vec<(usize, f32)>,
    colors: Vec<TextColor>,
    icon_brushes: Vec<bool>,
    default_font: usize,
    default_size: f32,
}
impl Resolved {
    fn bytes(&self) -> usize {
        self.text.capacity()
            + self.spans.capacity() * size_of::<ResolvedSpan>()
            + self.gaps.capacity() * size_of::<(usize, f32)>()
            + self.colors.capacity() * size_of::<TextColor>()
            + self.icon_brushes.capacity() * size_of::<bool>()
    }
}
#[derive(PartialEq)]
struct Key {
    resolved: Resolved,
    width: Option<f32>,
    scale: f64,
}
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct RasterSignature {
    canvas: [i64; 2],
    runs: Vec<RasterRun>,
}
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct RasterRun {
    font: (u64, u32),
    size: u32,
    coordinates: Vec<i16>,
    glyphs: Vec<(u32, u32, u32)>,
    brush: usize,
    // Target ink center and line bounds, in physical coordinates.
    alignment: Option<[u32; 3]>,
}
#[derive(Clone, Eq, Hash, PartialEq)]
struct PhaseGlyphKey {
    font: (u64, u32),
    size: u32,
    coordinates: Vec<i16>,
    glyph: u16,
    phase: [u8; 2],
}
#[derive(Clone)]
struct CachedGlyph {
    image: Arc<swash::scale::image::Image>,
}
#[derive(Clone, Eq, Hash, PartialEq)]
struct PhaseLineKey {
    signature: Arc<RasterSignature>,
    scale: u32,
    phase: [u8; 2],
}
impl RasterSignature {
    fn bytes(&self) -> usize {
        self.runs.capacity() * size_of::<RasterRun>()
            + self
                .runs
                .iter()
                .map(|r| {
                    r.coordinates.capacity() * size_of::<i16>()
                        + r.glyphs.capacity() * size_of::<(u32, u32, u32)>()
                })
                .sum::<usize>()
    }
}
struct CachedText {
    key: Key,
    signature: Arc<RasterSignature>,
    layout: TextLayout,
    bytes: usize,
}
/// One UI-local cache shared by all preparation contexts.
#[derive(Default)]
pub(crate) struct TextCache {
    service: RefCell<Option<TextService>>,
    owner: Rc<()>,
}
impl TextCache {
    pub(crate) fn with_service<T>(
        &self,
        f: impl FnOnce(&mut TextService) -> Result<T, RuntimeError>,
    ) -> Result<T, RuntimeError> {
        let mut service = self
            .service
            .try_borrow_mut()
            .map_err(|_| RuntimeError::BorrowConflict)?;
        if service.is_none() {
            *service = Some(TextService::with_owner(self.owner.clone())?);
        }
        f(service.as_mut().ok_or(TextError::Font)?)
    }
    pub(crate) fn clear(&self) {
        self.service.borrow_mut().take();
    }
    pub(crate) fn prepare(
        &self,
        text: &str,
        request: TextRequest<'_>,
    ) -> Result<TextLayout, RuntimeError> {
        self.with_service(|s| s.prepare_plain(text, request))
    }
    pub(crate) fn prepare_rich(
        &self,
        text: &RichText,
        request: TextRequest<'_>,
    ) -> Result<TextLayout, RuntimeError> {
        self.with_service(|s| s.prepare_rich(text, request))
    }
    pub(crate) fn fit(
        &self,
        text: &str,
        request: TextRequest<'_>,
        width: f32,
        overflow: TextOverflow,
    ) -> Result<FittedText, RuntimeError> {
        self.with_service(|s| s.fit_plain(text, request, width, overflow))
    }
}

/// Everything one text preparation takes besides its content.
#[derive(Clone, Copy)]
pub(crate) struct TextRequest<'a> {
    /// Logical wrapping width, or `None` for one unwrapped line per paragraph.
    pub(crate) width: Option<f32>,
    /// Physical pixels per logical pixel wanted. Preparation lowers it for text
    /// above [`MAX_FONT_PIXELS`]; the layout records the scale it used.
    pub(crate) scale: f64,
    /// Selects the default font role.
    pub(crate) style: TextStyle,
    /// Logical size of text without an explicit span size.
    pub(crate) font_size: f32,
    /// Logical size of inline icons; zero omits them.
    pub(crate) icon_size: f32,
    /// Logical width of each inline gap.
    pub(crate) gap: f32,
    /// Receives the bits of the font and icon roles the text reads.
    pub(crate) reads: &'a Cell<u64>,
}
impl<'a> TextRequest<'a> {
    /// Unwrapped text at scale 1 without inline icons or gaps.
    pub(crate) const fn new(style: TextStyle, font_size: f32, reads: &'a Cell<u64>) -> Self {
        Self {
            width: None,
            scale: 1.0,
            style,
            font_size,
            icon_size: 0.0,
            gap: 0.0,
            reads,
        }
    }
    pub(crate) const fn width(mut self, width: Option<f32>) -> Self {
        self.width = width;
        self
    }
    pub(crate) const fn scale(mut self, scale: f64) -> Self {
        self.scale = scale;
        self
    }
    /// Sets the logical icon size and gap width used by rich text.
    pub(crate) const fn inline(mut self, icon_size: f32, gap: f32) -> Self {
        self.icon_size = icon_size;
        self.gap = gap;
        self
    }
}
pub(crate) struct TextService {
    pub(crate) fonts: parley::FontContext,
    pub(crate) registry: registry::Registry,
    layout: parley::LayoutContext<usize>,
    scale: swash::scale::ScaleContext,
    cache: VecDeque<CachedText>,
    bytes: usize,
    font_keys: BTreeMap<(u64, u32), swash::CacheKey>,
    phase_glyphs: HashMap<PhaseGlyphKey, CachedGlyph>,
    phase_order: VecDeque<PhaseGlyphKey>,
    phase_glyph_bytes: usize,
    phase_lines: HashMap<PhaseLineKey, Arc<[MaskPart]>>,
    phase_line_order: VecDeque<PhaseLineKey>,
    phase_line_bytes: usize,
    /// Recent truncation results, so a layout pass at an unchanged width
    /// reuses its cut instead of searching again.
    fits: VecDeque<(FitKey, usize)>,
    #[cfg(test)]
    resolutions: Cell<usize>,
    #[cfg(test)]
    fit_searches: Cell<usize>,
    /// How many times text was shaped, cached or not.
    #[cfg(test)]
    shapings: Cell<usize>,
    /// The longest text shaped, in bytes.
    #[cfg(test)]
    longest_shaped: Cell<usize>,
}
/// Everything that decides where plain text is cut to fit a width.
#[derive(PartialEq)]
struct FitKey {
    /// The first line, which is all a truncation shows.
    line: String,
    /// Whether more text follows the line, so the ellipsis always shows.
    more: bool,
    /// Whether `line` is the bounded cut of a longer line, whose cluster
    /// boundaries at the join differ from a text of the same bytes.
    cut: bool,
    font: usize,
    size: u32,
    scale: u64,
    width: u32,
    overflow: TextOverflow,
}
/// The most truncation results the service remembers.
const FIT_MEMORY: usize = 64;
impl TextService {
    #[cfg(test)]
    pub(crate) fn new() -> Result<Self, TextError> {
        Self::with_owner(Rc::new(()))
    }
    fn with_owner(owner: Rc<()>) -> Result<Self, TextError> {
        let mut fonts = parley::FontContext::new();
        let mut registry = registry::Registry::new(owner);
        for font in fonts::BUNDLED_FONTS {
            let id = registry
                .register(&mut fonts, FontData::from_static(font.bytes()))
                .map_err(|_| TextError::Font)?;
            if id.index != font.index() {
                return Err(TextError::Font);
            }
        }
        Ok(Self {
            fonts,
            registry,
            layout: parley::LayoutContext::new(),
            scale: swash::scale::ScaleContext::new(),
            cache: VecDeque::new(),
            bytes: 0,
            font_keys: BTreeMap::new(),
            phase_glyphs: HashMap::new(),
            phase_order: VecDeque::new(),
            phase_glyph_bytes: 0,
            phase_lines: HashMap::new(),
            phase_line_order: VecDeque::new(),
            phase_line_bytes: 0,
            fits: VecDeque::new(),
            #[cfg(test)]
            resolutions: Cell::new(0),
            #[cfg(test)]
            fit_searches: Cell::new(0),
            #[cfg(test)]
            shapings: Cell::new(0),
            #[cfg(test)]
            longest_shaped: Cell::new(0),
        })
    }
    #[cfg(test)]
    pub(crate) fn prepare(
        &mut self,
        text: &str,
        width: Option<f32>,
        scale: f64,
        style: TextStyle,
    ) -> Result<TextLayout, RuntimeError> {
        self.prepare_plain(
            text,
            TextRequest::new(style, style.pixels(), &Cell::new(0))
                .width(width)
                .scale(scale),
        )
    }
    fn prepare_plain(
        &mut self,
        text: &str,
        request: TextRequest<'_>,
    ) -> Result<TextLayout, RuntimeError> {
        let TextRequest {
            width,
            style,
            font_size,
            reads,
            ..
        } = request;
        let scale = effective_scale(request.scale, [font_size]);
        let role = style.font_role();
        reads.set(reads.get() | role.mask());
        let font = self.registry.default_index(role);
        // Borrow the request until a miss. Ordinary rich controls use this path too.
        if let Some(entry) = self.cached_plain(text, width, scale, font, font_size) {
            return Ok(entry.layout.clone());
        }
        let request = TextRequest::new(style, font_size, reads);
        let resolved = self.resolve(&RichText::from(text), request)?;
        self.prepare_resolved(resolved, request.width(width).scale(scale), None)
    }
    /// The cached layout of plain `text` in the role's default font, exactly
    /// as `prepare_plain` would prepare it: one span covering the text (none
    /// for empty text) with no gap, icon or color of its own.
    fn cached_plain(
        &self,
        text: &str,
        width: Option<f32>,
        scale: f64,
        font: usize,
        font_size: f32,
    ) -> Option<&CachedText> {
        self.cache.iter().find(|entry| {
            let key = &entry.key;
            let r = &key.resolved;
            key.width == width
                && key.scale == scale
                && r.text == text
                && r.default_font == font
                && r.default_size == font_size
                && r.gaps.is_empty()
                && r.colors == [TextColor::Inherit]
                && r.icon_brushes == [false]
                && if text.is_empty() {
                    r.spans.is_empty()
                } else {
                    r.spans.as_slice()
                        == [ResolvedSpan {
                            range: 0..text.len(),
                            font,
                            size: font_size,
                            brush: 0,
                            icon: false,
                        }]
                }
        })
    }
    fn resolve(
        &self,
        content: &RichText,
        request: TextRequest<'_>,
    ) -> Result<Resolved, RuntimeError> {
        let TextRequest {
            style,
            font_size,
            icon_size,
            gap,
            reads,
            ..
        } = request;
        #[cfg(test)]
        self.resolutions.set(self.resolutions.get() + 1);
        if content.parts.len() > RichText::MAX_SEGMENTS {
            return Err(TextError::TooLarge(TextLimit::Segments).into());
        }
        let role = style.font_role();
        let mut resolved = Resolved {
            text: String::new(),
            spans: Vec::new(),
            gaps: Vec::new(),
            colors: vec![TextColor::Inherit],
            icon_brushes: vec![false],
            default_font: self.registry.default_index(role),
            default_size: font_size,
        };
        // The paragraph's empty-line metrics use the default font even when every
        // visible run overrides it. The first text span sets that default; an icon
        // must not change paragraph typography just because it precedes text.
        let mut first = true;
        for part in &content.parts {
            let (text, font, size, color, icon) = match part {
                rich::Inline::Text(span) => {
                    let font = match &span.font {
                        Some(rich::FontChoice::Exact(id)) => self.registry.validate(id)?,
                        choice => {
                            let r = if let Some(rich::FontChoice::Role(r)) = choice {
                                *r
                            } else {
                                role
                            };
                            reads.set(reads.get() | r.mask());
                            self.registry.default_index(r)
                        }
                    };
                    (
                        std::borrow::Cow::Borrowed(span.text.as_str()),
                        font,
                        span.size.map_or(font_size, crate::Length::get),
                        span.color,
                        false,
                    )
                }
                rich::Inline::Icon(source, color) => {
                    let (font, ch, mask) = self.registry.icon(source)?;
                    reads.set(reads.get() | mask);
                    (
                        std::borrow::Cow::Owned(ch.to_string()),
                        font,
                        icon_size,
                        *color,
                        true,
                    )
                }
                rich::Inline::Gap => {
                    resolved.gaps.push((resolved.text.len(), gap));
                    continue;
                }
            };
            if icon && size == 0.0 {
                continue;
            }
            if !size.is_finite() || size <= 0.0 {
                return Err(TextError::Geometry.into());
            }
            if resolved.text.len() + text.len() > crate::TextEditError::MAX_TEXT_BYTES {
                return Err(crate::TextEditError::TooLarge.into());
            }
            if first && !icon {
                resolved.default_font = font;
                resolved.default_size = size;
                first = false;
            }
            // Keep validation and paragraph defaults above, but never pass an
            // empty style range to the shaper (including between inline icons).
            if text.is_empty() {
                continue;
            }
            let start = resolved.text.len();
            resolved.text.push_str(&text);
            // An icon needs its own brush boundary even when a custom icon uses
            // the same font and color as adjacent text. Ordinary text can merge.
            let existing = (!icon)
                .then(|| {
                    resolved
                        .colors
                        .iter()
                        .zip(&resolved.icon_brushes)
                        .position(|(c, is_icon)| *c == color && !is_icon)
                })
                .flatten();
            let brush = if let Some(i) = existing {
                i
            } else {
                resolved.colors.push(color);
                resolved.icon_brushes.push(icon);
                resolved.colors.len() - 1
            };
            resolved.spans.push(ResolvedSpan {
                range: start..resolved.text.len(),
                font,
                size,
                brush,
                icon,
            });
        }
        if first {
            reads.set(reads.get() | role.mask());
        }
        Ok(resolved)
    }
    pub(crate) fn prepare_rich(
        &mut self,
        text: &RichText,
        request: TextRequest<'_>,
    ) -> Result<TextLayout, RuntimeError> {
        if request.gap.is_finite()
            && request.gap >= 0.0
            && let Some(plain) = text.as_plain()
        {
            return self.prepare_plain(plain, request);
        }
        let resolved = self.resolve(text, request)?;
        self.prepare_resolved(resolved, request, None)
    }
    /// Unwrapped logical width of plain text, shaped without rasterizing.
    fn plain_width(&mut self, text: &str, request: TextRequest<'_>) -> Result<f32, RuntimeError> {
        Ok(self.plain_shape(text, request)?.0)
    }
    /// Shapes plain text unwrapped without rasterizing, under the key
    /// `prepare_plain` would prepare it with, and returns its logical width
    /// with the key and the shaped layout, which preparing it can reuse.
    fn plain_shape(
        &mut self,
        text: &str,
        request: TextRequest<'_>,
    ) -> Result<(f32, Key, parley::Layout<usize>), RuntimeError> {
        let scale = effective_scale(request.scale, [request.font_size]);
        let plain = TextRequest::new(request.style, request.font_size, request.reads);
        let resolved = self.resolve(&RichText::from(text), plain)?;
        let key = Key {
            resolved,
            width: None,
            scale,
        };
        let layout = self.build_layout(&key)?;
        Ok((layout.width() / scale as f32, key, layout))
    }
    /// Prepares plain text to fit `width` as `overflow` asks. A truncated
    /// result is searched once per text, font, size, scale and width: the
    /// search shapes candidates without rasterizing them, and the cut is
    /// remembered so later layout passes prepare only the shown text.
    pub(crate) fn fit_plain(
        &mut self,
        text: &str,
        request: TextRequest<'_>,
        width: f32,
        overflow: TextOverflow,
    ) -> Result<FittedText, RuntimeError> {
        if !width.is_finite() || width < 0.0 {
            return Err(TextError::Geometry.into());
        }
        if overflow == TextOverflow::Wrap {
            return Ok(FittedText {
                layout: self.prepare_plain(text, request.width(Some(width)))?,
                shown: None,
            });
        }
        let request = request.width(None);
        // A line too long to prepare is cut first; the cut always shows the
        // ellipsis, as a line followed by a line break does.
        let bounded = fit::bounded(text, overflow);
        let (line, bounds, broken) = match &bounded {
            Some((cut, bounds)) => (cut.as_str(), bounds.clone(), true),
            None => fit::clusters(text),
        };
        let key = FitKey {
            line: line.to_owned(),
            more: broken,
            cut: bounded.is_some(),
            font: self.registry.default_index(request.style.font_role()),
            size: request.font_size.to_bits(),
            scale: effective_scale(request.scale, [request.font_size]).to_bits(),
            width: width.to_bits(),
            overflow,
        };
        // A cut is remembered only for a text that did not fit at this key,
        // so with one the whole text is not measured again.
        let remembered = self
            .fits
            .iter()
            .find(|(k, _)| *k == key)
            .map(|(_, kept)| *kept);
        if remembered.is_none()
            && !broken
            && let Some(full) = self.fit_whole(text, request, width)?
        {
            return Ok(FittedText {
                layout: full,
                shown: None,
            });
        }
        let count = bounds.len() - 1;
        // Without a line break the whole line did not fit, so at least one
        // cluster goes; after a break the whole first line may stay.
        let most = if broken {
            count
        } else {
            count.saturating_sub(1)
        };
        let mut kept = match remembered {
            Some(kept) => kept,
            None => {
                #[cfg(test)]
                self.fit_searches.set(self.fit_searches.get() + 1);
                let (mut low, mut high) = (0, most);
                while low < high {
                    let middle = (low + high).div_ceil(2);
                    let shown = fit::candidate(line, &bounds, middle, overflow);
                    // A candidate is longer with every kept cluster, so one
                    // beyond the byte limit is treated as too wide.
                    if shown.len() <= crate::TextEditError::MAX_TEXT_BYTES
                        && self.plain_width(&shown, request)? <= width
                    {
                        low = middle;
                    } else {
                        high = middle - 1;
                    }
                }
                low
            }
        };
        // The search measures advances; the prepared layout also counts glyph
        // overhangs, so step back while the result is still too wide.
        loop {
            let shown = fit::candidate(line, &bounds, kept, overflow);
            if shown.len() > crate::TextEditError::MAX_TEXT_BYTES && kept > 0 {
                kept -= 1;
                continue;
            }
            let layout = self.prepare_plain(&shown, request)?;
            if layout.size().width() <= width || kept == 0 {
                if remembered != Some(kept) {
                    self.fits.retain(|(k, _)| *k != key);
                    if self.fits.len() >= FIT_MEMORY {
                        self.fits.pop_front();
                    }
                    self.fits.push_back((key, kept));
                }
                return Ok(FittedText {
                    layout,
                    shown: Some(shown),
                });
            }
            kept -= 1;
        }
    }
    /// The whole unwrapped plain text prepared, if it fits `width`.
    ///
    /// Its advance width is read from a cached layout under the key
    /// `prepare_plain` uses, or else measured by shaping without rasterizing
    /// first: text far wider than `width` may exceed the raster limit as a
    /// whole, though a truncation fits. A fitting text then reuses that
    /// shaping for its raster, so it is shaped once. The prepared layout also
    /// counts glyph overhangs, which can still make it too wide.
    fn fit_whole(
        &mut self,
        text: &str,
        request: TextRequest<'_>,
        width: f32,
    ) -> Result<Option<TextLayout>, RuntimeError> {
        let TextRequest {
            style,
            font_size,
            reads,
            ..
        } = request;
        let scale = effective_scale(request.scale, [font_size]);
        let role = style.font_role();
        let font = self.registry.default_index(role);
        let full = if let Some(entry) = self.cached_plain(text, None, scale, font, font_size) {
            reads.set(reads.get() | role.mask());
            if entry.layout.shaped.width() / scale as f32 > width {
                return Ok(None);
            }
            entry.layout.clone()
        } else {
            let (advance, key, shaped) = self.plain_shape(text, request)?;
            if advance > width {
                return Ok(None);
            }
            let plain = TextRequest::new(style, font_size, reads);
            self.prepare_resolved(key.resolved, plain.width(None).scale(scale), Some(shaped))?
        };
        Ok((full.size().width() <= width).then_some(full))
    }
    /// Prepares resolved text through the cache; `shaped`, when given, is
    /// this very text already shaped under the same key, which a cache miss
    /// rasterizes instead of shaping again.
    fn prepare_resolved(
        &mut self,
        resolved: Resolved,
        request: TextRequest<'_>,
        shaped: Option<parley::Layout<usize>>,
    ) -> Result<TextLayout, RuntimeError> {
        let TextRequest { width, gap, .. } = request;
        let scale = effective_scale(
            request.scale,
            std::iter::once(resolved.default_size).chain(resolved.spans.iter().map(|s| s.size)),
        );
        if !scale.is_finite()
            || scale <= 0.0
            || !(scale as f32).is_finite()
            || scale as f32 <= 0.0
            || width.is_some_and(|w| !w.is_finite() || w < 0.0)
            || !gap.is_finite()
            || gap < 0.0
        {
            return Err(TextError::Geometry.into());
        }
        if resolved.gaps.iter().any(|(_, gap)| {
            !(*gap * scale as f32).is_finite() || *gap * scale as f32 > MAX_SIDE as f32
        }) {
            return Err(TextError::TooLarge(TextLimit::RasterSide).into());
        }
        let key = Key {
            resolved,
            width,
            scale,
        };
        if let Some(entry) = self.cache.iter().find(|e| e.key == key) {
            return Ok(entry.layout.clone());
        }
        let (signature, result) = self.shape(&key, shaped)?;
        let bytes = result
            .parts
            .iter()
            .map(|p| p.image.pixels().len() + size_of::<MaskPart>())
            .sum::<usize>()
            + key.resolved.text.len() * 128
            + key.resolved.bytes()
            + signature.bytes()
            + size_of::<CachedText>();
        if bytes <= CACHE_BYTES {
            while self.cache.len() >= 128 || self.bytes + bytes > CACHE_BYTES {
                if let Some(old) = self.cache.pop_front() {
                    self.bytes -= old.bytes;
                } else {
                    break;
                }
            }
            self.bytes += bytes;
            self.cache.push_back(CachedText {
                key,
                signature: result.signature.clone(),
                layout: result.clone(),
                bytes,
            });
        }
        Ok(result)
    }
    /// Shapes `key`, then reuses the coverage of a cached entry whose positioned
    /// glyphs are identical, or rasterizes and composites new coverage.
    fn shape(
        &mut self,
        key: &Key,
        shaped: Option<parley::Layout<usize>>,
    ) -> Result<(RasterSignature, TextLayout), TextError> {
        let resolved = &key.resolved;
        let scale = key.scale as f32;
        let layout = match shaped {
            Some(layout) => layout,
            None => self.build_layout(key)?,
        };
        if !layout.width().is_finite() || !layout.height().is_finite() {
            return Err(TextError::Geometry);
        }
        let canvas = [layout.width().ceil() as i64, layout.height().ceil() as i64];
        if canvas[0] > MAX_SIDE || canvas[1] > MAX_SIDE {
            return Err(TextError::TooLarge(TextLimit::RasterSide));
        }
        if canvas[0] * canvas[1] > MaskImage::MAX_PIXELS as i64 {
            return Err(TextError::TooLarge(TextLimit::Coverage));
        }
        let signature = raster_signature(resolved, &layout, canvas);
        if let Some(entry) = self.cache.iter().find(|entry| {
            entry.key.resolved == *resolved
                && entry.key.scale as f32 == scale
                && entry.signature.as_ref() == &signature
        }) {
            let mut result = entry.layout.clone();
            result.shaped = Arc::new(layout);
            return Ok((signature, result));
        }
        let (glyphs, bounds) = self.rasterize_glyphs(&layout, &signature, [0, 0], false)?;
        let [left, top, right, bottom] = bounds;
        let width = u32::try_from(right - left).map_err(|_| TextError::Geometry)?;
        let height = u32::try_from(bottom - top).map_err(|_| TextError::Geometry)?;
        let size = Size::new(width as f32 / scale, height as f32 / scale)
            .map_err(|_| TextError::Geometry)?;
        let ink = ink_bounds(&glyphs, bounds, scale);
        let parts = composite(&glyphs, bounds, scale)?;
        let retained_signature = Arc::new(signature.clone());
        Ok((
            signature,
            TextLayout {
                size,
                ink,
                parts: parts.into(),
                colors: resolved.colors.clone().into(),
                shaped: Arc::new(layout),
                scale,
                origin: [-left as f32 / scale, -top as f32 / scale],
                signature: retained_signature,
            },
        ))
    }
    /// Shapes, wraps and aligns the resolved spans in physical pixels.
    fn build_layout(&mut self, key: &Key) -> Result<parley::Layout<usize>, TextError> {
        #[cfg(test)]
        {
            self.shapings.set(self.shapings.get() + 1);
            let length = key.resolved.text.len().max(self.longest_shaped.get());
            self.longest_shaped.set(length);
        }
        let resolved = &key.resolved;
        let text = &resolved.text;
        let scale = key.scale as f32;
        let families: Vec<String> = resolved
            .spans
            .iter()
            .map(|s| {
                let name = &self.registry.fonts[s.font].family;
                if s.icon {
                    name.clone()
                } else {
                    format!(
                        "{name}, {}, {}",
                        self.registry.fonts[fonts::BundledFont::Body.index()].family,
                        self.registry.fonts[fonts::BundledFont::Monospace.index()].family
                    )
                }
            })
            .collect();
        let mut builder = self
            .layout
            .ranged_builder(&mut self.fonts, text, scale, true);
        builder.push_default(parley::FontFamily::from(
            self.registry.fonts[resolved.default_font].family.as_str(),
        ));
        builder.push_default(parley::StyleProperty::FontSize(resolved.default_size));
        builder.push_default(parley::LineHeight::Absolute(resolved.default_size * 1.4));
        for (span, family) in resolved.spans.iter().zip(&families) {
            builder.push(
                parley::FontFamily::from(family.as_str()),
                span.range.clone(),
            );
            builder.push(
                parley::StyleProperty::FontSize(span.size),
                span.range.clone(),
            );
            // Icons reserve their nominal height at the top of Parley's line
            // box; the ink is centered on neighboring text below. Text keeps
            // its ordinary 40% leading, and an absolute default keeps inline
            // gaps from inheriting icon leading.
            builder.push(
                parley::LineHeight::Absolute(if span.icon {
                    span.size
                } else {
                    span.size * 1.4
                }),
                span.range.clone(),
            );
            if span.icon {
                builder.push(parley::VerticalAlign::TOP, span.range.clone());
            }
            builder.push(parley::StyleProperty::Brush(span.brush), span.range.clone());
        }
        for (id, (index, width)) in resolved.gaps.iter().enumerate() {
            builder.push_inline_box(parley::InlineBox {
                id: id as u64,
                kind: parley::InlineBoxKind::InFlow,
                index: *index,
                width: *width * scale,
                height: 0.0,
                baseline: None,
                vertical_align: parley::VerticalAlign::BASELINE,
            });
        }
        let mut layout = builder.build(text);
        let physical_width = key.width.map(|w| w * scale);
        if physical_width.is_some_and(|v| !v.is_finite()) {
            return Err(TextError::Geometry);
        }
        layout.break_all_lines(physical_width);
        layout.align(
            parley::Alignment::Start,
            parley::AlignmentOptions::default(),
        );
        Ok(layout)
    }
    /// Renders every glyph's coverage at its pixel position, aligning icons to
    /// their neighboring text. Returns the glyphs and the canvas grown to
    /// contain them, as `[left, top, right, bottom]` physical pixels.
    fn rasterize_glyphs(
        &mut self,
        layout: &parley::Layout<usize>,
        signature: &RasterSignature,
        phase: [u8; 2],
        cache: bool,
    ) -> Result<(Vec<PlacedGlyph>, [i64; 4]), TextError> {
        let [mut right, mut bottom] = signature.canvas;
        let (mut left, mut top) = (0i64, 0i64);
        let mut raster_bytes = 0usize;
        let mut glyphs = Vec::new();
        // Each icon segment owns a unique brush, including in bidi text.
        // Look up its alignment by identity instead of coupling two traversals.
        let alignments: BTreeMap<_, _> = signature
            .runs
            .iter()
            .filter_map(|run| run.alignment.map(|alignment| (run.brush, alignment)))
            .collect();
        for line in layout.lines() {
            for item in line.items() {
                let parley::PositionedLayoutItem::GlyphRun(glyph_run) = item else {
                    continue;
                };
                let alignment = alignments.get(&glyph_run.style().brush).copied();
                let run = glyph_run.run();
                let font = run.font();
                let coordinates: Vec<i16> = if cache {
                    run.normalized_coords()
                        .iter()
                        .map(|coord| coord.to_bits())
                        .collect()
                } else {
                    Vec::new()
                };
                let mut font_ref =
                    swash::FontRef::from_index(font.data.as_ref(), font.index as usize)
                        .ok_or(TextError::Font)?;
                font_ref.key = *self
                    .font_keys
                    .entry((font.data.id(), font.index))
                    .or_default();
                let mut scaler = self
                    .scale
                    .builder(font_ref)
                    .size(run.font_size())
                    .hint(true)
                    .normalized_coords(run.normalized_coords().iter().map(|coord| coord.to_bits()))
                    .build();
                for glyph in glyph_run.positioned_glyphs() {
                    let id = u16::try_from(glyph.id).map_err(|_| TextError::Font)?;
                    let x = if cache {
                        ((glyph.x + f32::from(phase[0]) * 0.25) * 4.0).round() * 0.25
                    } else {
                        glyph.x
                    };
                    let y = if cache {
                        ((glyph.y + f32::from(phase[1]) * 0.25) * 4.0).round() * 0.25
                    } else {
                        glyph.y
                    };
                    let key = cache.then(|| PhaseGlyphKey {
                        font: (font.data.id(), font.index),
                        size: run.font_size().to_bits(),
                        coordinates: coordinates.clone(),
                        glyph: id,
                        phase: [
                            ((x - x.floor()) * 4.0).round() as u8 % 4,
                            ((y - y.floor()) * 4.0).round() as u8 % 4,
                        ],
                    });
                    let cached = key
                        .as_ref()
                        .and_then(|key| self.phase_glyphs.get(key).cloned());
                    let cached = if let Some(cached) = cached {
                        Some(cached)
                    } else {
                        let image = swash::scale::Render::new(&[swash::scale::Source::Outline])
                            .format(swash::zeno::Format::Alpha)
                            .offset(swash::zeno::Vector::new(
                                x - x.floor(),
                                // Swash's outline offset points upward in Y;
                                // destination coordinates point downward.
                                if cache {
                                    -(y - y.floor())
                                } else {
                                    y - y.floor()
                                },
                            ))
                            .render(&mut scaler, id);
                        image.map(|image| CachedGlyph {
                            image: Arc::new(image),
                        })
                    };
                    let Some(cached) = cached else {
                        continue;
                    }; // Empty outlines (e.g. spaces) have no ink.
                    let image = &cached.image;
                    if let Some(key) = key
                        && !self.phase_glyphs.contains_key(&key)
                    {
                        let bytes = image.data.len();
                        if bytes <= CACHE_BYTES {
                            while self.phase_order.len() >= 4096
                                || self.phase_glyph_bytes + bytes > CACHE_BYTES
                            {
                                let Some(old) = self.phase_order.pop_front() else {
                                    break;
                                };
                                if let Some(evicted) = self.phase_glyphs.remove(&old) {
                                    self.phase_glyph_bytes -= evicted.image.data.len();
                                }
                            }
                            self.phase_glyph_bytes += bytes;
                            self.phase_order.push_back(key.clone());
                            self.phase_glyphs.insert(key, cached.clone());
                        }
                    }
                    raster_bytes = raster_bytes
                        .checked_add(image.data.len())
                        .ok_or(TextError::TooLarge(TextLimit::Coverage))?;
                    if raster_bytes > RASTER_BYTES {
                        return Err(TextError::TooLarge(TextLimit::Coverage));
                    }
                    let x = x.floor() as i64 + i64::from(image.placement.left);
                    let mut y = y.floor() as i64 - i64::from(image.placement.top);
                    if let Some(alignment) = alignment {
                        let [center, line_top, line_bottom] = alignment.map(f32::from_bits);
                        // A phase can change the raster bounds by one pixel.
                        // Align against the unphased icon, then compensate for
                        // the new top bearing so it stays with neighboring text.
                        let unphased = (cache && phase[1] != 0)
                            .then(|| {
                                swash::scale::Render::new(&[swash::scale::Source::Outline])
                                    .format(swash::zeno::Format::Alpha)
                                    .offset(swash::zeno::Vector::new(
                                        glyph.x - glyph.x.floor(),
                                        glyph.y - glyph.y.floor(),
                                    ))
                                    .render(&mut scaler, id)
                            })
                            .flatten();
                        if let Some(reference) = &unphased {
                            raster_bytes = raster_bytes
                                .checked_add(reference.data.len())
                                .ok_or(TextError::TooLarge(TextLimit::Coverage))?;
                            if raster_bytes > RASTER_BYTES {
                                return Err(TextError::TooLarge(TextLimit::Coverage));
                            }
                        }
                        // An outline absent at zero phase keeps the ordinary
                        // bounds-based alignment of the available raster.
                        let reference = unphased.as_ref().unwrap_or(image);
                        let height = reference.placement.height as f32;
                        // Keep ink inside the existing line box. Unusually large
                        // custom glyphs retain their baseline if they cannot fit.
                        let min = line_top.ceil();
                        let max = (line_bottom - height).floor();
                        if min <= max {
                            y = (center - height * 0.5).round().clamp(min, max) as i64
                                + i64::from(reference.placement.top)
                                - i64::from(image.placement.top);
                        }
                    }
                    let glyph = PlacedGlyph {
                        x,
                        y,
                        image: cached.image,
                        brush: glyph_run.style().brush,
                    };
                    left = left.min(glyph.x);
                    top = top.min(glyph.y);
                    right = right.max(glyph.right());
                    bottom = bottom.max(glyph.bottom());
                    if right - left > MAX_SIDE || bottom - top > MAX_SIDE {
                        return Err(TextError::TooLarge(TextLimit::RasterSide));
                    }
                    if (right - left) * (bottom - top) > MaskImage::MAX_PIXELS as i64 {
                        return Err(TextError::TooLarge(TextLimit::Coverage));
                    }
                    glyphs.push(glyph);
                }
            }
        }
        Ok((glyphs, [left, top, right, bottom]))
    }

    fn phase_parts(
        &mut self,
        layout: &TextLayout,
        phase: [u8; 2],
    ) -> Result<Arc<[MaskPart]>, TextError> {
        let line_key = PhaseLineKey {
            signature: layout.signature.clone(),
            scale: layout.scale.to_bits(),
            phase,
        };
        if let Some(parts) = self.phase_lines.get(&line_key) {
            return Ok(parts.clone());
        }
        let (glyphs, bounds) =
            self.rasterize_glyphs(&layout.shaped, &layout.signature, phase, true)?;
        let original_left = (-layout.origin[0] * layout.scale).round() as i64;
        let original_top = (-layout.origin[1] * layout.scale).round() as i64;
        let mut parts = composite(&glyphs, bounds, layout.scale)?;
        let dx = (bounds[0] - original_left) as f32 / layout.scale;
        let dy = (bounds[1] - original_top) as f32 / layout.scale;
        for part in &mut parts {
            part.offset[0] += dx;
            part.offset[1] += dy;
        }
        let bytes = parts
            .iter()
            .map(|part| part.image.pixels().len() + size_of::<MaskPart>())
            .sum::<usize>();
        let parts: Arc<[MaskPart]> = parts.into();
        if bytes <= CACHE_BYTES {
            while self.phase_line_order.len() >= PHASE_LINE_ENTRIES
                || self.phase_line_bytes + bytes > CACHE_BYTES
            {
                let Some(old) = self.phase_line_order.pop_front() else {
                    break;
                };
                if let Some(evicted) = self.phase_lines.remove(&old) {
                    self.phase_line_bytes -= evicted
                        .iter()
                        .map(|part| part.image.pixels().len() + size_of::<MaskPart>())
                        .sum::<usize>();
                }
            }
            self.phase_line_bytes += bytes;
            self.phase_line_order.push_back(line_key.clone());
            self.phase_lines.insert(line_key, parts.clone());
        }
        Ok(parts)
    }
}

/// One rendered glyph with its top-left corner at a physical canvas pixel.
struct PlacedGlyph {
    x: i64,
    y: i64,
    image: Arc<swash::scale::image::Image>,
    brush: usize,
}
impl PlacedGlyph {
    fn right(&self) -> i64 {
        self.x + i64::from(self.image.placement.width)
    }
    fn bottom(&self) -> i64 {
        self.y + i64::from(self.image.placement.height)
    }
}

/// What decides a layout's coverage: its canvas and every positioned glyph, with
/// the alignment target of each icon run. Equal signatures rasterize equally.
fn raster_signature(
    resolved: &Resolved,
    layout: &parley::Layout<usize>,
    canvas: [i64; 2],
) -> RasterSignature {
    let mut line_top = 0.0;
    RasterSignature {
        canvas,
        runs: layout
            .lines()
            .flat_map(|line| {
                let line_start = line_top;
                line_top += line.metrics().line_height;
                let line_end = line_top;
                let runs: Vec<_> = line
                    .items()
                    .filter_map(|item| {
                        if let parley::PositionedLayoutItem::GlyphRun(run) = item {
                            Some(run)
                        } else {
                            None
                        }
                    })
                    .collect();
                runs.iter()
                    .enumerate()
                    .map(|(index, glyph_run)| {
                        let run = glyph_run.run();
                        let brush = glyph_run.style().brush;
                        let alignment = resolved.icon_brushes[brush].then(|| {
                            // Use the nearest text run on this visual line, including
                            // its actual fallback font and size. Keep icons inside the
                            // nominal line box, without extra icon-font leading.
                            runs.iter()
                                .enumerate()
                                .filter(|(_, other)| !resolved.icon_brushes[other.style().brush])
                                .min_by_key(|(other, _)| index.abs_diff(*other))
                                .map(|(_, other)| {
                                    let metrics = other.run().font_metrics();
                                    let cap = metrics
                                        .cap_height
                                        .filter(|h| h.is_finite() && *h > 0.0)
                                        .unwrap_or(other.run().font_size() * 0.7);
                                    [other.baseline() - cap * 0.5, line_start, line_end]
                                        .map(f32::to_bits)
                                })
                                .unwrap_or_else(|| {
                                    [(line_start + line_end) * 0.5, line_start, line_end]
                                        .map(f32::to_bits)
                                })
                        });
                        RasterRun {
                            font: (run.font().data.id(), run.font().index),
                            brush,
                            alignment,
                            size: run.font_size().to_bits(),
                            coordinates: run
                                .normalized_coords()
                                .iter()
                                .map(|coord| coord.to_bits())
                                .collect(),
                            glyphs: glyph_run
                                .positioned_glyphs()
                                .map(|glyph| (glyph.id, glyph.x.to_bits(), glyph.y.to_bits()))
                                .collect(),
                        }
                    })
                    .collect::<Vec<_>>()
            })
            .collect(),
    }
}

/// The logical bounds of all ink relative to the canvas's top-left corner, or
/// `None` when no glyph has ink.
fn ink_bounds(glyphs: &[PlacedGlyph], canvas: [i64; 4], scale: f32) -> Option<[f32; 4]> {
    let [left, top, right, bottom] = canvas;
    if glyphs.is_empty() {
        return None;
    }
    let mut bounds = [right, bottom, left, top];
    for glyph in glyphs {
        bounds[0] = bounds[0].min(glyph.x);
        bounds[1] = bounds[1].min(glyph.y);
        bounds[2] = bounds[2].max(glyph.right());
        bounds[3] = bounds[3].max(glyph.bottom());
    }
    Some([
        (bounds[0] - left) as f32 / scale,
        (bounds[1] - top) as f32 / scale,
        (bounds[2] - left) as f32 / scale,
        (bounds[3] - top) as f32 / scale,
    ])
}

/// Blends each run of consecutive glyphs with one brush into one coverage mask.
/// A single run keeps the whole canvas, so plain text preserves its layout
/// geometry.
fn composite(
    glyphs: &[PlacedGlyph],
    canvas: [i64; 4],
    scale: f32,
) -> Result<Vec<MaskPart>, TextError> {
    let [left, top, right, bottom] = canvas;
    let mut parts = Vec::new();
    let mut total = 0usize;
    for run in glyphs.chunk_by(|a, b| a.brush == b.brush) {
        let brush = run[0].brush;
        let (mut l, mut t, mut r, mut b) = (right, bottom, left, top);
        for glyph in run {
            l = l.min(glyph.x);
            t = t.min(glyph.y);
            r = r.max(glyph.right());
            b = b.max(glyph.bottom());
        }
        if run.len() == glyphs.len() {
            (l, t, r, b) = (left, top, right, bottom);
        }
        if r > l && b > t {
            let w = (r - l) as usize;
            let h = (b - t) as usize;
            total = total
                .checked_add(w * h)
                .ok_or(TextError::TooLarge(TextLimit::Coverage))?;
            if total > RASTER_BYTES {
                return Err(TextError::TooLarge(TextLimit::Coverage));
            }
            let mut pixels = vec![0u8; w * h];
            for glyph in run {
                let gw = glyph.image.placement.width as usize;
                if gw == 0 {
                    continue;
                }
                for (index, coverage) in glyph.image.data.iter().copied().enumerate() {
                    let px = (glyph.x - l) as usize + index % gw;
                    let py = (glyph.y - t) as usize + index / gw;
                    let dest = pixels.get_mut(py * w + px).ok_or(TextError::Geometry)?;
                    *dest = (u16::from(coverage)
                        + u16::from(*dest) * (255 - u16::from(coverage)) / 255)
                        as u8;
                }
            }
            parts.push(MaskPart {
                image: MaskImage::new(w as u32, h as u32, pixels)
                    .map_err(|_| TextError::TooLarge(TextLimit::Coverage))?,
                offset: [(l - left) as f32 / scale, (t - top) as f32 / scale],
                brush,
            });
        }
    }
    Ok(parts)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn image(layout: &TextLayout) -> &MaskImage {
        &layout.parts.first().unwrap().image
    }

    #[test]
    fn phase_cache_reuses_visible_rows_across_vertical_scroll_phases()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut service = TextService::new()?;
        let layouts = (0..60)
            .map(|row| {
                service.prepare(
                    &format!("Hamburgefonts 0123456789 — Il1 O0 row {row}"),
                    None,
                    0.9,
                    TextStyle::Body,
                )
            })
            .collect::<Result<Vec<_>, _>>()?;
        let mut warmed = Vec::new();
        for y_phase in 0..4 {
            for layout in &layouts {
                warmed.push(service.phase_parts(layout, [0, y_phase])?);
            }
        }
        for y_phase in 0..4 {
            for (row, layout) in layouts.iter().enumerate() {
                let cached = service.phase_parts(layout, [0, y_phase])?;
                assert!(Arc::ptr_eq(&cached, &warmed[y_phase as usize * 60 + row]));
            }
        }
        assert!(service.phase_line_bytes <= CACHE_BYTES);
        Ok(())
    }

    #[test]
    #[ignore = "opt-in release-profile measurement of phase-cache reuse"]
    fn phase_cache_scroll_profile() -> Result<(), Box<dyn std::error::Error>> {
        use std::time::{Duration, Instant};

        for scale in [0.75, 0.9, 1.25] {
            for rows in [20, 60, 100] {
                for (motion, phases) in [("vertical", 4usize), ("two-axis", 16usize)] {
                    let mut service = TextService::new()?;
                    let layouts = (0..rows)
                        .map(|row| {
                            service.prepare(
                                &format!("Hamburgefonts 0123456789 — Il1 O0 row {row}"),
                                None,
                                scale,
                                TextStyle::Body,
                            )
                        })
                        .collect::<Result<Vec<_>, _>>()?;
                    let mut samples = Vec::with_capacity(32);
                    let mut hits = 0usize;
                    let mut misses = 0usize;
                    let mut evictions = 0usize;
                    let mut peak_line_bytes = 0usize;
                    let mut peak_glyph_bytes = 0usize;
                    for frame in 0..(phases + 32) {
                        let step = frame % phases;
                        let phase = if phases == 4 {
                            [0, step as u8]
                        } else {
                            [(step % 4) as u8, (step / 4) as u8]
                        };
                        let start = Instant::now();
                        for layout in &layouts {
                            let key = PhaseLineKey {
                                signature: layout.signature.clone(),
                                scale: layout.scale.to_bits(),
                                phase,
                            };
                            let hit = service.phase_lines.contains_key(&key);
                            let before = service.phase_lines.len();
                            let parts = service.phase_parts(layout, phase)?;
                            std::hint::black_box(parts);
                            if frame >= phases {
                                if hit {
                                    hits += 1;
                                } else {
                                    misses += 1;
                                    if service.phase_lines.contains_key(&key) {
                                        evictions += before + 1 - service.phase_lines.len();
                                    }
                                }
                            }
                        }
                        if frame >= phases {
                            samples.push(start.elapsed());
                        }
                        peak_line_bytes = peak_line_bytes.max(service.phase_line_bytes);
                        peak_glyph_bytes = peak_glyph_bytes.max(service.phase_glyph_bytes);
                        assert!(service.phase_line_bytes <= CACHE_BYTES);
                        assert!(service.phase_glyph_bytes <= CACHE_BYTES);
                        assert!(service.phase_lines.len() <= PHASE_LINE_ENTRIES);
                        assert!(service.phase_glyphs.len() <= 4096);
                    }
                    samples.sort_unstable();
                    let percentile =
                        |rank: usize| -> Duration { samples[(samples.len() - 1) * rank / 100] };
                    println!(
                        "phase cache scale={scale:.2} rows={rows} motion={motion}: hit={hits} miss={misses} evict={evictions}, line={peak_line_bytes} B/{} entries, glyph={peak_glyph_bytes} B/{} entries, update median/p95={}/{} us",
                        service.phase_lines.len(),
                        service.phase_glyphs.len(),
                        percentile(50).as_micros(),
                        percentile(95).as_micros(),
                    );
                }
            }
        }
        Ok(())
    }

    #[test]
    fn width_changes_reuse_identical_rasters_but_reflow_matches_fresh_preparation() {
        let mut service = TextService::new().unwrap();
        for scale in [1.0, 1.25, 2.0] {
            for style in [TextStyle::Body, TextStyle::Monospace] {
                let first = service
                    .prepare("Count: 42", Some(180.0), scale, style)
                    .unwrap();
                let wider = service
                    .prepare("Count: 42", Some(220.0), scale, style)
                    .unwrap();
                assert_eq!(
                    image(&first).pixels().as_ptr(),
                    image(&wider).pixels().as_ptr()
                );
                for text in [
                    "A longer label with several words",
                    "Åäö e\u{301} ffi",
                    "abc אבג 123",
                    "אבג 123",
                    "a\n\nb  ",
                    "",
                ] {
                    let mut fresh = TextService::new().unwrap();
                    for width in [220.0, 80.0, 81.0, 20.0, 0.0, 180.0] {
                        let actual = service.prepare(text, Some(width), scale, style).unwrap();
                        // Empty the comparison cache so equivalence cannot mask stale reuse.
                        fresh.cache.clear();
                        fresh.bytes = 0;
                        let expected = fresh.prepare(text, Some(width), scale, style).unwrap();
                        assert_eq!(actual.size(), expected.size());
                        assert_eq!(
                            actual.parts.first().map(|p| &p.image).map(|i| (
                                i.width(),
                                i.height(),
                                i.pixels()
                            )),
                            expected.parts.first().map(|p| &p.image).map(|i| (
                                i.width(),
                                i.height(),
                                i.pixels()
                            ))
                        );
                    }
                }
                let narrow = service
                    .prepare("Count: 42", Some(30.0), scale, style)
                    .unwrap();
                assert!(narrow.size().height() > first.size().height());
            }
        }
    }
    #[test]
    fn sizes_above_the_raster_limit_lower_the_scale_instead_of_failing() {
        let mut service = TextService::new().unwrap();
        let reads = Cell::new(0);
        // A plain 200-pixel label at scale 2 rasterizes at 256 pixels.
        let plain = service
            .prepare_plain(
                "Aa",
                TextRequest::new(TextStyle::Body, 200.0, &reads).scale(2.0),
            )
            .unwrap();
        assert_eq!(plain.scale, (MAX_FONT_PIXELS / 200.0) as f32);
        let limit = service
            .prepare_plain(
                "Aa",
                TextRequest::new(TextStyle::Body, 128.0, &reads).scale(2.0),
            )
            .unwrap();
        assert_eq!(limit.scale, 2.0, "exactly at the limit keeps the scale");
        // The largest rich span decides, even when the style's size is small.
        let content = RichText::new()
            .text("a")
            .span(TextSpan::new("B").size(crate::Length::new(400.0).unwrap()));
        let rich = service
            .prepare_rich(
                &content,
                TextRequest::new(TextStyle::Body, 14.0, &reads).scale(1.0),
            )
            .unwrap();
        assert_eq!(rich.scale, (MAX_FONT_PIXELS / 400.0) as f32);
        // Logical size tracks the text, not its raster: about 400 * 1.4 tall.
        assert!(
            (rich.size().height() - 560.0).abs() < 60.0,
            "{:?}",
            rich.size()
        );
        let again = service
            .prepare_rich(
                &content,
                TextRequest::new(TextStyle::Body, 14.0, &reads).scale(3.0),
            )
            .unwrap();
        assert!(
            Arc::ptr_eq(&rich.parts, &again.parts),
            "scales above the limit share one preparation"
        );
    }
    #[test]
    fn cache_budget_and_entry_limit_include_owned_metadata() {
        let mut service = TextService::new().unwrap();
        let live = service
            .prepare("Keep alive", Some(180.0), 1.0, TextStyle::Body)
            .unwrap();
        let pixels = image(&live).pixels().to_vec();
        for index in 0..140 {
            service
                .prepare(&format!("Label {index}"), Some(180.0), 1.0, TextStyle::Body)
                .unwrap();
            assert!(service.cache.len() <= 128);
            assert!(service.bytes <= CACHE_BYTES);
            assert_eq!(
                service.bytes,
                service.cache.iter().map(|entry| entry.bytes).sum()
            );
        }
        assert!(
            !service
                .cache
                .iter()
                .any(|entry| entry.key.resolved.text == "Keep alive")
        );
        assert_eq!(image(&live).pixels(), pixels);
    }
}

#[cfg(test)]
mod rich_tests {
    use super::*;
    fn rich(
        service: &mut TextService,
        content: &RichText,
        width: Option<f32>,
        scale: f64,
    ) -> TextLayout {
        service
            .prepare_rich(
                content,
                TextRequest::new(TextStyle::Body, 14.0, &Cell::new(0))
                    .width(width)
                    .scale(scale)
                    .inline(16.0, 6.0),
            )
            .unwrap()
    }
    fn ink_center(part: &MaskPart, scale: f32) -> f32 {
        let width = part.image.width() as usize;
        let rows: Vec<_> = part
            .image
            .pixels()
            .chunks(width)
            .enumerate()
            .filter(|(_, row)| row.iter().any(|p| *p != 0))
            .map(|(y, _)| y)
            .collect();
        part.offset[1] * scale + (rows[0] + rows[rows.len() - 1] + 1) as f32 * 0.5
    }

    #[test]
    fn plain_cache_hits_skip_resolution_and_recollect_current_font_roles() {
        for caption in ["Cache me", ""] {
            let mut service = TextService::new().unwrap();
            let content = RichText::from(caption);
            let first = rich(&mut service, &content, None, 1.0);
            let resolutions = service.resolutions.get();
            let reads = Cell::new(0);
            let second = service
                .prepare_rich(
                    &content,
                    TextRequest::new(TextStyle::Heading, 14.0, &reads).inline(16.0, 6.0),
                )
                .unwrap();
            assert_eq!(reads.get(), FontRole::Heading.mask());
            assert_eq!(service.resolutions.get(), resolutions);
            assert!(Arc::ptr_eq(&first.parts, &second.parts));
            let plain = service
                .prepare(caption, None, 1.0, TextStyle::Body)
                .unwrap();
            assert!(Arc::ptr_eq(&first.parts, &plain.parts));
            assert_eq!(service.resolutions.get(), resolutions);
            let mono = service.registry.default_id(FontRole::Monospace);
            service
                .registry
                .set_default(FontRole::Body, Some(&mono))
                .unwrap();
            let changed = rich(&mut service, &content, None, 1.0);
            assert!(!Arc::ptr_eq(&first.parts, &changed.parts));
            service.registry.set_default(FontRole::Body, None).unwrap();
            let restored = rich(&mut service, &content, None, 1.0);
            assert!(Arc::ptr_eq(&first.parts, &restored.parts));
        }
    }

    #[test]
    fn empty_styled_spans_keep_paragraph_defaults_and_validate_fonts() {
        let mut service = TextService::new().unwrap();
        let content = RichText::new()
            .span(TextSpan::new("").size(crate::Length::new(28.0).unwrap()))
            .icon(Icon::Add);
        let actual = rich(&mut service, &content, None, 1.0);
        let reads = Cell::new(0);
        let expected = service
            .prepare_rich(
                &RichText::new().icon(Icon::Add),
                TextRequest::new(TextStyle::Body, 28.0, &reads).inline(16.0, 6.0),
            )
            .unwrap();
        assert_eq!(actual.size(), expected.size());
        assert_eq!(actual.shaped.height(), expected.shaped.height());
        let foreign = TextService::new()
            .unwrap()
            .registry
            .default_id(FontRole::Body);
        let invalid = RichText::new()
            .icon(Icon::Add)
            .span(TextSpan::new("").font(foreign));
        assert!(matches!(
            service.prepare_rich(
                &invalid,
                TextRequest::new(TextStyle::Body, 14.0, &reads).inline(16.0, 6.0),
            ),
            Err(RuntimeError::Font(FontError::ForeignFont))
        ));
    }

    #[test]
    fn bundled_fonts_are_embedded_once_and_registered_by_role() {
        let service = TextService::new().unwrap();
        for font in fonts::BUNDLED_FONTS {
            let registered = service.registry.fonts[font.index()]
                .data
                .0
                .as_ref()
                .as_ref();
            assert_eq!(registered.as_ptr(), font.bytes().as_ptr());
        }
        for (role, font) in [
            (FontRole::Body, fonts::BundledFont::Body),
            (FontRole::Heading, fonts::BundledFont::Body),
            (FontRole::Monospace, fonts::BundledFont::Monospace),
        ] {
            let data = FontData::bundled(role);
            assert_eq!(data.0.as_ref().as_ref().as_ptr(), font.bytes().as_ptr());
            assert_eq!(service.registry.default_index(role), font.index());
        }
    }

    #[test]
    fn collection_with_two_readable_faces_is_rejected_before_registration() {
        let font = fonts::BundledFont::Body.bytes();
        let mut bytes = b"ttcf\0\x01\0\0\0\0\0\x02\0\0\0\x14\0\0\0\x14".to_vec();
        bytes.extend_from_slice(font);
        let tables = u16::from_be_bytes([font[4], font[5]]) as usize;
        for index in 0..tables {
            let start = 20 + 12 + index * 16 + 8;
            let offset = u32::from_be_bytes(bytes[start..start + 4].try_into().unwrap());
            bytes[start..start + 4].copy_from_slice(&(offset + 20).to_be_bytes());
        }
        assert!(swash::FontRef::from_index(&bytes, 1).is_some());
        let mut service = TextService::new().unwrap();
        let before = service.registry.fonts.len();
        assert_eq!(
            service
                .registry
                .register(&mut service.fonts, FontData::from_owned(bytes)),
            Err(FontError::InvalidData)
        );
        assert_eq!(service.registry.fonts.len(), before);
        assert!(
            service
                .prepare("Still usable", None, 1.0, TextStyle::Body)
                .is_ok()
        );
    }

    #[test]
    fn wrapped_bidi_icons_use_their_own_alignment_and_reuse_coverage() {
        let content = RichText::new()
            .icon(Icon::Close)
            .gap()
            .text("ABC אבג ")
            .icon(Icon::Save)
            .gap()
            .text("דהו XYZ ")
            .icon(Icon::Settings);
        for scale in [1.0, 1.25, 2.0] {
            let mut service = TextService::new().unwrap();
            let layout = rich(&mut service, &content, Some(90.0), scale);
            assert!(layout.shaped.lines().count() > 1);
            let signature = &service.cache.back().unwrap().signature;
            for run in &signature.runs {
                if let Some([center, _, _]) = run.alignment {
                    let part = layout
                        .parts
                        .iter()
                        .find(|part| part.brush == run.brush)
                        .unwrap();
                    assert!((ink_center(part, scale as f32) - f32::from_bits(center)).abs() <= 1.0);
                }
            }
            let again = rich(&mut service, &content, Some(90.0), scale);
            assert!(Arc::ptr_eq(&layout.parts, &again.parts));
        }
    }

    #[test]
    fn inline_icon_ink_aligns_with_neighboring_text_at_each_scale_and_font() {
        for scale in [1.0, 1.25, 2.0] {
            let mut service = TextService::new().unwrap();
            for role in [FontRole::Body, FontRole::Monospace] {
                let font = service.registry.default_id(role);
                let custom = service.registry.glyph(&font, 'A').unwrap();
                for icon in [
                    Icon::Settings.into(),
                    Icon::Power.into(),
                    Icon::ChevronDown.into(),
                    IconSource::Custom(custom),
                ] {
                    for trailing in [false, true] {
                        for size in [14.0, 22.0] {
                            let content = RichText::new().span(
                                TextSpan::new("HI")
                                    .font(font.clone())
                                    .size(crate::Length::new(size).unwrap()),
                            );
                            let content = if trailing {
                                content.trailing_icon(icon.clone())
                            } else {
                                content.leading_icon(icon.clone())
                            };
                            let layout = rich(&mut service, &content, Some(200.0), scale);
                            assert_eq!(layout.parts.len(), 2);
                            let delta = (ink_center(&layout.parts[0], scale as f32)
                                - ink_center(&layout.parts[1], scale as f32))
                            .abs();
                            assert!(
                                delta <= 1.0,
                                "ink center delta {delta}, scale {scale}, size {size}"
                            );
                            // Width-independent reuse retains the aligned pixels.
                            let wider = rich(&mut service, &content, Some(240.0), scale);
                            assert!(Arc::ptr_eq(&layout.parts, &wider.parts));
                        }
                    }
                }
            }
            let standalone = rich(
                &mut service,
                &RichText::new().icon(Icon::Settings),
                None,
                scale,
            );
            let material = FontId {
                owner: service.registry.owner.clone(),
                index: fonts::BundledFont::Icons.index(),
            };
            let baseline = rich(
                &mut service,
                &RichText::new().span(
                    TextSpan::new("\u{e8b8}")
                        .font(material)
                        .size(crate::Length::new(16.0).unwrap()),
                ),
                None,
                scale,
            );
            let text = rich(&mut service, &RichText::from("HI"), None, scale);
            assert_eq!(standalone.size().height(), text.size().height());
            assert_eq!(standalone.size().width(), baseline.size().width());
            let ink_rows = |image: &MaskImage| {
                image
                    .pixels()
                    .chunks(image.width() as usize)
                    .filter(|row| row.iter().any(|alpha| *alpha != 0))
                    .flatten()
                    .copied()
                    .collect::<Vec<_>>()
            };
            assert_eq!(
                ink_rows(&standalone.parts[0].image),
                ink_rows(&baseline.parts[0].image)
            );
        }
    }

    #[test]
    fn segment_boundaries_preserve_clusters_bidi_and_wrapping() {
        for scale in [1.0, 1.25, 2.0] {
            for text in ["office e\u{301} אבג 123", "abc\n\nשלום", "Åäö café"] {
                let split = text
                    .chars()
                    .fold(RichText::new(), |t, ch| t.text(ch.to_string()));
                let mut service = TextService::new().unwrap();
                for width in [None, Some(200.0), Some(60.0), Some(10.0)] {
                    let plain = service
                        .prepare(text, width, scale, TextStyle::Body)
                        .unwrap();
                    let split = rich(&mut service, &split, width, scale);
                    assert_eq!(plain.size(), split.size());
                    let masks = |t: &TextLayout| {
                        t.parts
                            .iter()
                            .map(|p| (p.offset, p.image.clone()))
                            .collect::<Vec<_>>()
                    };
                    assert_eq!(masks(&plain), masks(&split));
                    for index in 0..=text.len() {
                        assert_eq!(
                            crate::TextSelection::new(&plain, index, index).caret(&plain),
                            crate::TextSelection::new(&split, index, index).caret(&split)
                        );
                    }
                }
            }
        }
    }
    #[test]
    fn all_catalog_entries_exist_and_custom_font_names_do_not_alias() {
        let mut service = TextService::new().unwrap();
        for line in include_str!("../assets/MaterialIconsOutlined-Regular.codepoints").lines() {
            let (_, hex) = line.split_once(' ').unwrap();
            let ch = char::from_u32(u32::from_str_radix(hex, 16).unwrap()).unwrap();
            let _ = service.registry.material_glyph(ch).unwrap();
        }
        let first = service
            .registry
            .register(&mut service.fonts, FontData::bundled(FontRole::Body))
            .unwrap();
        let second = service
            .registry
            .register(&mut service.fonts, FontData::bundled(FontRole::Body))
            .unwrap();
        let a = rich(
            &mut service,
            &RichText::new().span(TextSpan::new("Test").font(first)),
            None,
            1.0,
        );
        let b = rich(
            &mut service,
            &RichText::new().span(TextSpan::new("Test").font(second)),
            None,
            1.0,
        );
        let font = |t: &TextLayout| {
            t.shaped
                .lines()
                .flat_map(|l| l.items())
                .find_map(|i| {
                    if let parley::PositionedLayoutItem::GlyphRun(r) = i {
                        Some(r.run().font().data.id())
                    } else {
                        None
                    }
                })
                .unwrap()
        };
        assert_ne!(font(&a), font(&b));
        assert_eq!(a.parts[0].image, b.parts[0].image);
    }
    #[test]
    fn text_fallback_uses_bundled_text_and_icon_sources_stay_exact() {
        let mut service = TextService::new().unwrap();
        let icon_font = FontId {
            owner: service.registry.owner.clone(),
            index: fonts::BundledFont::Icons.index(),
        };
        let normal = service.prepare("ΩЖ", None, 1.0, TextStyle::Body).unwrap();
        let fallback = rich(
            &mut service,
            &RichText::new().span(TextSpan::new("ΩЖ").font(icon_font)),
            None,
            1.0,
        );
        assert_eq!(normal.size(), fallback.size());
        // Parley's line boxes can place the same fallback glyph ink at a
        // different baseline when the requested family changes. The actual
        // glyph coverage must still come from the bundled text font.
        let ink_rows = |image: &MaskImage| {
            image
                .pixels()
                .chunks(image.width() as usize)
                .filter(|row| row.iter().any(|alpha| *alpha != 0))
                .flatten()
                .copied()
                .collect::<Vec<_>>()
        };
        assert_eq!(
            normal.parts[0].image.width(),
            fallback.parts[0].image.width()
        );
        assert_eq!(
            ink_rows(&normal.parts[0].image),
            ink_rows(&fallback.parts[0].image)
        );
        let custom = service
            .registry
            .glyph(&service.registry.default_id(FontRole::Body), 'A')
            .unwrap();
        service
            .registry
            .set_icon(Icon::Close, Some(custom))
            .unwrap();
        let actual = rich(&mut service, &RichText::new().icon(Icon::Close), None, 1.0);
        assert!(!actual.parts.is_empty());
    }

    fn fit(
        service: &mut TextService,
        text: &str,
        width: f32,
        scale: f64,
        overflow: TextOverflow,
    ) -> FittedText {
        let reads = Cell::new(0);
        service
            .fit_plain(
                text,
                TextRequest::new(TextStyle::Body, 14.0, &reads).scale(scale),
                width,
                overflow,
            )
            .unwrap()
    }

    /// The grapheme cluster boundaries of `text`, including its end.
    fn cluster_bounds(text: &str) -> Vec<usize> {
        unicode_segmentation::UnicodeSegmentation::grapheme_indices(text, true)
            .map(|(index, _)| index)
            .chain([text.len()])
            .collect()
    }

    #[test]
    fn a_line_beyond_the_byte_limit_is_cut_on_its_own_cluster_boundaries() {
        let limit = crate::TextEditError::MAX_TEXT_BYTES;
        let budget = limit - fit::ELLIPSIS.len();
        // Exactly the limit is not cut; one byte more is.
        assert!(fit::bounded(&"a".repeat(limit), TextOverflow::Ellipsis).is_none());
        let (cut, bounds) = fit::bounded(&"a".repeat(limit + 1), TextOverflow::Ellipsis).unwrap();
        assert_eq!(cut.len(), budget);
        assert_eq!(bounds, cluster_bounds(&cut));
        // A line break ends the line before the limit applies.
        let broken = format!("short\n{}", "a".repeat(2 * limit));
        assert!(fit::bounded(&broken, TextOverflow::Ellipsis).is_none());
        // One cluster longer than the budget leaves nothing.
        let huge = format!("e{}", "\u{301}".repeat(limit));
        let (cut, bounds) = fit::bounded(&huge, TextOverflow::Ellipsis).unwrap();
        assert_eq!((cut.as_str(), bounds.as_slice()), ("", &[0][..]));
        assert_eq!(
            fit::candidate(&cut, &bounds, 0, TextOverflow::Ellipsis),
            fit::ELLIPSIS
        );
        // Regional-indicator flags and combining marks across both shares,
        // after an odd and an even number of leading indicators.
        for lead in ["", "\u{1F1F8}"] {
            let flags = "\u{1F1F8}\u{1F1EA}".repeat(limit / 8 + 1);
            let marks = "e\u{301}\u{302}".repeat(limit / 5 + 1);
            let text = format!("{lead}{flags} {marks}x{flags}");
            let whole = cluster_bounds(&text);
            for overflow in [TextOverflow::Ellipsis, TextOverflow::MiddleEllipsis] {
                let (cut, bounds) = fit::bounded(&text, overflow).unwrap();
                assert!(cut.len() <= budget, "{overflow:?}");
                assert_eq!(*bounds.last().unwrap(), cut.len());
                let count = bounds.len() - 1;
                let shown = fit::candidate(&cut, &bounds, count, overflow);
                assert!(shown.len() <= limit);
                let (head, tail) = shown.split_once(fit::ELLIPSIS).unwrap();
                assert!(text.starts_with(head) && text.ends_with(tail));
                assert!(whole.contains(&head.len()), "{overflow:?} head");
                assert!(
                    whole.contains(&(text.len() - tail.len())),
                    "{overflow:?} tail"
                );
                if overflow == TextOverflow::MiddleEllipsis {
                    // As many clusters from each end, and the ellipsis at the join.
                    let join = bounds.iter().position(|b| *b == head.len()).unwrap();
                    assert_eq!(join, count - join);
                    assert!(!tail.is_empty());
                } else {
                    assert!(tail.is_empty());
                }
            }
        }
    }

    #[test]
    fn the_cut_keeps_whole_source_clusters_at_each_share_boundary() {
        let limit = crate::TextEditError::MAX_TEXT_BYTES;
        let budget = limit - fit::ELLIPSIS.len();
        let filler = "x".repeat(limit);
        // A combining sequence across the end of the head's share is left out.
        let text = format!("{}e\u{301}\u{302}{filler}", "a".repeat(budget - 2));
        let (cut, _) = fit::bounded(&text, TextOverflow::Ellipsis).unwrap();
        assert_eq!(cut, "a".repeat(budget - 2));
        // Middle: the head's share is budget - budget / 2 bytes, the tail's
        // budget / 2. Whitespace beside the join is trimmed by the cluster
        // boundaries; the kept counts are equal.
        let (head, tail) = (budget - budget / 2, budget / 2);
        let text = format!("{} {filler} {}", "h".repeat(head - 1), "t".repeat(tail - 1));
        let (cut, bounds) = fit::bounded(&text, TextOverflow::MiddleEllipsis).unwrap();
        let count = bounds.len() - 1;
        assert_eq!(count % 2, 0);
        let shown = fit::candidate(&cut, &bounds, count, TextOverflow::MiddleEllipsis);
        let (start, end) = shown.split_once(fit::ELLIPSIS).unwrap();
        assert!(
            !start.ends_with(' ') && !end.starts_with(' '),
            "{:?}",
            &shown[..8]
        );
        assert!(start.chars().all(|c| c == 'h') && end.chars().all(|c| c == 't'));
        // Fewer kept clusters come from both ends, on source boundaries.
        for kept in [1, 2, 7, count / 3] {
            let shown = fit::candidate(&cut, &bounds, kept, TextOverflow::MiddleEllipsis);
            let (start, end) = shown.split_once(fit::ELLIPSIS).unwrap();
            assert!(text.starts_with(start) && text.ends_with(end), "{kept}");
        }
        // A tail run of regional indicators of either parity: pairs from the
        // run's start, so an odd run ends in a single indicator.
        for run in [3, 4, 2001, 2002] {
            let text = format!("{filler}x{}", "\u{1F1F8}".repeat(run));
            let whole = cluster_bounds(&text);
            let (cut, bounds) = fit::bounded(&text, TextOverflow::MiddleEllipsis).unwrap();
            let count = bounds.len() - 1;
            for kept in [count, count - 1, 2] {
                let shown = fit::candidate(&cut, &bounds, kept, TextOverflow::MiddleEllipsis);
                let (_, end) = shown.split_once(fit::ELLIPSIS).unwrap();
                assert!(text.ends_with(end));
                assert!(
                    whole.contains(&(text.len() - end.len())),
                    "run {run}, kept {kept}"
                );
            }
        }
        // One cluster longer than the budget on both sides leaves "…" alone.
        let huge = format!("e{}", "\u{301}".repeat(limit));
        let (cut, bounds) = fit::bounded(&huge, TextOverflow::MiddleEllipsis).unwrap();
        assert_eq!(
            fit::candidate(&cut, &bounds, 0, TextOverflow::MiddleEllipsis),
            fit::ELLIPSIS
        );
    }

    #[test]
    fn a_line_at_the_limit_is_truncated_within_it() {
        // Not cut by the bound, but no candidate with its ellipsis may pass
        // the byte limit either: zero-width clusters fit any width.
        let mut service = TextService::new().unwrap();
        let limit = crate::TextEditError::MAX_TEXT_BYTES;
        for text in [
            format!("{}a\n", "\u{200B}".repeat(limit / 3)),
            format!("{}a", "\u{200B}".repeat(limit / 3)),
            format!("{}\n", "\u{200B}".repeat(limit / 3 - 1)),
        ] {
            for overflow in [TextOverflow::Ellipsis, TextOverflow::MiddleEllipsis] {
                let fitted = fit(&mut service, &text, 120.0, 1.0, overflow);
                if let Some(shown) = fitted.shown() {
                    assert!(shown.len() <= limit, "{overflow:?}");
                }
            }
        }
    }

    #[test]
    fn a_cut_line_is_fitted_with_its_ellipsis_and_never_shaped_whole() {
        let mut service = TextService::new().unwrap();
        let limit = crate::TextEditError::MAX_TEXT_BYTES;
        // A long log line, and zero-width clusters whose whole cut would fit
        // the width: the ellipsis shows either way.
        let words = "word ".repeat(200_000);
        let invisible = "\u{200B}".repeat(20_000);
        for overflow in [TextOverflow::Ellipsis, TextOverflow::MiddleEllipsis] {
            for text in [&words, &invisible] {
                let fitted = fit(&mut service, text, 120.0, 1.0, overflow);
                let shown = fitted.shown().expect("cut text is truncated");
                assert!(shown.contains(fit::ELLIPSIS));
                assert!(shown.len() <= limit);
            }
        }
        assert!(service.longest_shaped.get() <= limit);
        // A cut and an ordinary text of the same bytes keep separate entries,
        // warm or cold: the ordinary one fits whole at the same width.
        let (cut, _) = fit::bounded(&invisible, TextOverflow::Ellipsis).unwrap();
        for _ in 0..2 {
            let ordinary = fit(&mut service, &cut, 120.0, 1.0, TextOverflow::Ellipsis);
            assert_eq!(ordinary.shown(), None);
            let again = fit(&mut service, &invisible, 120.0, 1.0, TextOverflow::Ellipsis);
            assert!(again.shown().is_some());
        }
    }

    #[test]
    fn ellipsis_fits_the_width_keeps_whole_graphemes_and_is_remembered() {
        let mut service = TextService::new().unwrap();
        let text = "Cafe\u{301} au lait with oat milk and e\u{301}clairs";
        for scale in [1.0, 1.5, 2.0] {
            for width in [40.0, 75.0, 120.0, 170.0] {
                for overflow in [TextOverflow::Ellipsis, TextOverflow::MiddleEllipsis] {
                    let fitted = fit(&mut service, text, width, scale, overflow);
                    let shown = fitted.shown().expect("narrower than the text");
                    assert!(
                        fitted.layout().size().width() <= width,
                        "{shown:?} at {width}"
                    );
                    assert!(shown.contains(fit::ELLIPSIS));
                    // Every kept cluster is whole: removing the ellipsis leaves
                    // a prefix and a suffix of the text on cluster boundaries.
                    let (head, tail) = shown.split_once(fit::ELLIPSIS).unwrap();
                    assert!(text.starts_with(head) && text.ends_with(tail));
                    let boundaries: Vec<usize> =
                        unicode_segmentation::UnicodeSegmentation::grapheme_indices(text, true)
                            .map(|(index, _)| index)
                            .chain([text.len()])
                            .collect();
                    assert!(boundaries.contains(&head.len()));
                    assert!(boundaries.contains(&(text.len() - tail.len())));
                    if overflow == TextOverflow::Ellipsis {
                        assert!(tail.is_empty());
                    }
                }
            }
        }
        // A wider width keeps more text, and the whole text fits untouched.
        let narrow = fit(&mut service, text, 80.0, 1.0, TextOverflow::Ellipsis);
        let wide = fit(&mut service, text, 160.0, 1.0, TextOverflow::Ellipsis);
        assert!(wide.shown().unwrap().len() > narrow.shown().unwrap().len());
        let whole = fit(&mut service, text, 2000.0, 1.0, TextOverflow::Ellipsis);
        assert!(!whole.is_truncated());
        // The same text, font, size, scale and width reuse the cut.
        let searches = service.fit_searches.get();
        let again = fit(&mut service, text, 80.0, 1.0, TextOverflow::Ellipsis);
        assert_eq!(again.shown(), narrow.shown());
        assert_eq!(service.fit_searches.get(), searches);
        let _ = fit(&mut service, text, 81.0, 1.0, TextOverflow::Ellipsis);
        assert_eq!(service.fit_searches.get(), searches + 1);
    }

    fn fit_at(
        service: &mut TextService,
        text: &str,
        size: f32,
        width: f32,
        scale: f64,
    ) -> Result<FittedText, RuntimeError> {
        let reads = Cell::new(0);
        service.fit_plain(
            text,
            TextRequest::new(TextStyle::Body, size, &reads).scale(scale),
            width,
            TextOverflow::Ellipsis,
        )
    }

    #[test]
    fn a_fitting_ellipsis_text_is_shaped_once_and_then_not_at_all() {
        let mut service = TextService::new().unwrap();
        let text = "A short cell";
        let before = service.shapings.get();
        let first = fit(&mut service, text, 400.0, 1.0, TextOverflow::Ellipsis);
        assert!(!first.is_truncated());
        assert_eq!(service.shapings.get(), before + 1, "cold: shaped once");
        // Measure, arrange and repaint at the same width, and a narrower
        // width at which it still fits.
        for width in [400.0, 400.0, 400.0, 200.0] {
            let again = fit(&mut service, text, width, 1.0, TextOverflow::Ellipsis);
            assert!(!again.is_truncated());
            assert_eq!(again.layout().size(), first.layout().size());
        }
        assert_eq!(service.shapings.get(), before + 1, "warm: not shaped");
    }

    #[test]
    fn a_truncated_text_measures_the_whole_string_once() {
        let mut service = TextService::new().unwrap();
        let text = "A message far too long for its narrow table column";
        let first = fit(&mut service, text, 80.0, 1.0, TextOverflow::Ellipsis);
        assert!(first.is_truncated());
        let shaped = service.shapings.get();
        for _ in 0..3 {
            let again = fit(&mut service, text, 80.0, 1.0, TextOverflow::Ellipsis);
            assert_eq!(again.shown(), first.shown());
        }
        assert_eq!(service.shapings.get(), shaped, "the remembered cut answers");
    }

    #[test]
    fn cold_and_warm_fits_agree_around_the_advance_width() {
        let reads = Cell::new(0);
        let request = TextRequest::new(TextStyle::Body, 14.0, &reads);
        let mut texts = vec!["Wide Words", "fff", "ƒjord"];
        // A text whose glyphs overhang their advance fits by advance and not
        // by ink at its advance width; the bundled font has some.
        let mut probe = TextService::new().unwrap();
        let overhanging = ["ƒ", "j", "f", "ʃ", "y", "Ψ"]
            .into_iter()
            .find(|text| {
                let advance = probe.plain_width(text, request).unwrap();
                probe.prepare_plain(text, request).unwrap().size().width() > advance
            })
            .expect("a glyph that overhangs its advance");
        texts.push(overhanging);
        for text in texts {
            let advance = TextService::new()
                .unwrap()
                .plain_width(text, request)
                .unwrap();
            for width in [advance - 0.5, advance, advance + 0.5] {
                let cold = fit(
                    &mut TextService::new().unwrap(),
                    text,
                    width,
                    1.0,
                    TextOverflow::Ellipsis,
                );
                let mut service = TextService::new().unwrap();
                service.prepare_plain(text, request).unwrap();
                let _ = fit(&mut service, text, width, 1.0, TextOverflow::Ellipsis);
                let warm = fit(&mut service, text, width, 1.0, TextOverflow::Ellipsis);
                assert_eq!(cold.shown(), warm.shown(), "{text:?} at {width}");
                assert_eq!(cold.layout().size(), warm.layout().size());
                // Font handles differ between services; the raster and the
                // positioned glyphs do not.
                let glyphs = |layout: &TextLayout| {
                    let signature = &layout.signature;
                    (
                        signature.canvas.to_vec(),
                        signature
                            .runs
                            .iter()
                            .map(|run| run.glyphs.clone())
                            .collect::<Vec<_>>(),
                    )
                };
                assert_eq!(glyphs(cold.layout()), glyphs(warm.layout()));
            }
        }
    }

    #[test]
    fn only_a_plain_unwrapped_entry_under_the_same_key_answers_the_width() {
        let reads = Cell::new(0);
        let text = "Cached text";
        // Wrapped and rich preparations of the same text do not answer it.
        let mut service = TextService::new().unwrap();
        let request = TextRequest::new(TextStyle::Body, 14.0, &reads);
        service
            .prepare_plain(text, request.width(Some(500.0)))
            .unwrap();
        service
            .prepare_rich(
                &RichText::new()
                    .span(TextSpan::new(text).color(Color::rgba(1.0, 0.0, 0.0, 1.0).unwrap())),
                request,
            )
            .unwrap();
        // Entries with an icon or a gap after the same text neither.
        service
            .prepare_rich(
                &RichText::from(text).icon(Icon::Check),
                request.inline(16.0, 6.0),
            )
            .unwrap();
        service
            .prepare_rich(
                &RichText::from(text).gap().span(TextSpan::new("")),
                request.inline(16.0, 6.0),
            )
            .unwrap();
        let before = service.shapings.get();
        let _ = fit_at(&mut service, text, 14.0, 400.0, 1.0).unwrap();
        assert_eq!(service.shapings.get(), before + 1);
        // Another size or scale misses.
        let _ = fit_at(&mut service, text, 15.0, 400.0, 1.0).unwrap();
        let _ = fit_at(&mut service, text, 14.0, 400.0, 2.0).unwrap();
        assert_eq!(service.shapings.get(), before + 3);
        // Scales that reduce to the same effective scale hit: 200 px is
        // rasterized at 256 at both 2× and 3×.
        let _ = fit_at(&mut service, "Big", 200.0, 4000.0, 2.0).unwrap();
        let shaped = service.shapings.get();
        let _ = fit_at(&mut service, "Big", 200.0, 4000.0, 3.0).unwrap();
        assert_eq!(service.shapings.get(), shaped);
    }

    #[test]
    fn a_cache_hit_records_the_font_read_and_another_font_misses() {
        let mut service = TextService::new().unwrap();
        let text = "Read tracking";
        let _ = fit_at(&mut service, text, 14.0, 400.0, 1.0).unwrap();
        let reads = Cell::new(0);
        let before = service.shapings.get();
        let hit = service
            .fit_plain(
                text,
                TextRequest::new(TextStyle::Body, 14.0, &reads),
                400.0,
                TextOverflow::Ellipsis,
            )
            .unwrap();
        assert!(!hit.is_truncated());
        assert_eq!(service.shapings.get(), before, "a hit");
        assert_ne!(reads.get() & FontRole::Body.mask(), 0);
        // The body role now resolves to the monospace font.
        let monospace = service.registry.default_id(FontRole::Monospace);
        service
            .registry
            .set_default(FontRole::Body, Some(&monospace))
            .unwrap();
        let _ = fit_at(&mut service, text, 14.0, 400.0, 1.0).unwrap();
        assert_eq!(service.shapings.get(), before + 1);
    }

    #[test]
    fn evicting_the_layout_or_the_cut_keeps_the_result() {
        let mut service = TextService::new().unwrap();
        let text = "A message far too long for its narrow table column";
        let first = fit(&mut service, text, 80.0, 1.0, TextOverflow::Ellipsis);
        let searches = service.fit_searches.get();
        // Preparing other texts evicts the prepared candidate from the
        // bounded layout cache; the cut is still remembered.
        let reads = Cell::new(0);
        for index in 0..130 {
            service
                .prepare_plain(
                    &format!("filler {index}"),
                    TextRequest::new(TextStyle::Body, 14.0, &reads),
                )
                .unwrap();
        }
        let shaped = service.shapings.get();
        let again = fit(&mut service, text, 80.0, 1.0, TextOverflow::Ellipsis);
        assert_eq!(again.shown(), first.shown());
        assert_eq!(service.fit_searches.get(), searches, "no new search");
        assert_eq!(service.shapings.get(), shaped + 1, "the candidate again");
        // Fitting more texts than `FIT_MEMORY` forgets the cut; the search
        // finds it again.
        for index in 0..FIT_MEMORY {
            let _ = fit(
                &mut service,
                &format!("another long text that is cut, number {index}"),
                80.0,
                1.0,
                TextOverflow::Ellipsis,
            );
        }
        let searches = service.fit_searches.get();
        let again = fit(&mut service, text, 80.0, 1.0, TextOverflow::Ellipsis);
        assert_eq!(again.shown(), first.shown());
        assert_eq!(service.fit_searches.get(), searches + 1);
    }

    #[test]
    fn a_text_beyond_the_raster_limit_truncates_unless_the_cut_is_too() {
        let mut service = TextService::new().unwrap();
        let text = "W".repeat(1000);
        let reads = Cell::new(0);
        let request = TextRequest::new(TextStyle::Body, 14.0, &reads);
        assert!(service.plain_width(&text, request).unwrap() > MAX_SIDE as f32);
        let fitted = fit(&mut service, &text, 100.0, 1.0, TextOverflow::Ellipsis);
        assert!(fitted.is_truncated());
        assert_eq!(
            fit_at(&mut service, &text, 14.0, 9000.0, 1.0).err(),
            Some(RuntimeError::Text(TextError::TooLarge(
                TextLimit::RasterSide
            )))
        );
    }

    #[test]
    fn ellipsis_handles_line_breaks_tiny_widths_and_invalid_widths() {
        let mut service = TextService::new().unwrap();
        let broken = fit(
            &mut service,
            "First\nsecond",
            500.0,
            1.0,
            TextOverflow::Ellipsis,
        );
        assert_eq!(broken.shown(), Some("First\u{2026}"));
        let tiny = fit(
            &mut service,
            "Overflowing",
            1.0,
            1.0,
            TextOverflow::Ellipsis,
        );
        assert_eq!(tiny.shown(), Some("\u{2026}"));
        let wrapped = fit(&mut service, "Two words", 30.0, 1.0, TextOverflow::Wrap);
        assert!(!wrapped.is_truncated());
        let reads = Cell::new(0);
        for width in [f32::NAN, -1.0, f32::INFINITY] {
            assert!(
                service
                    .fit_plain(
                        "x",
                        TextRequest::new(TextStyle::Body, 14.0, &reads),
                        width,
                        TextOverflow::Ellipsis,
                    )
                    .is_err()
            );
        }
    }

    #[test]
    fn highlights_validate_boundaries_and_merge_ranges() {
        let text = "Reverb send";
        let content = RichText::highlighted(text, [7..11, 0..3, 2..6, 4..4]).unwrap();
        assert_eq!(content.plain_text(), text);
        let spans: Vec<_> = content
            .parts
            .iter()
            .map(|part| match part {
                rich::Inline::Text(span) => (span.text.clone(), span.color),
                _ => unreachable!(),
            })
            .collect();
        let lit = TextColor::Role(crate::ColorRole::Highlight);
        assert_eq!(
            spans,
            vec![
                ("Reverb".to_owned(), lit),
                (" ".to_owned(), TextColor::Inherit),
                ("send".to_owned(), lit),
            ]
        );
        assert_eq!(
            RichText::highlighted(text, std::iter::once(3..20)),
            Err(HighlightError::OutOfBounds {
                start: 3,
                end: 20,
                len: 11
            })
        );
        assert_eq!(
            RichText::highlighted("e\u{301}x", std::iter::once(0..1)),
            Err(HighlightError::NotGraphemeBoundary { index: 1 })
        );
        assert_eq!(
            RichText::highlighted("", std::iter::once(0..0))
                .unwrap()
                .plain_text(),
            ""
        );
    }
}
