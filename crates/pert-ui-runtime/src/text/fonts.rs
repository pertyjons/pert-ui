//! Font sources and stable local identities; registration remains in the text service.
use std::{fmt, rc::Rc, sync::Arc};

// Each bundled file is embedded once; registration and `FontData::bundled` share it.
static NOTO_SANS: &[u8] = include_bytes!("../../assets/NotoSans.ttf");
static NOTO_SANS_MONO: &[u8] = include_bytes!("../../assets/NotoSansMono.ttf");
static MATERIAL_ICONS: &[u8] = include_bytes!("../../assets/MaterialIconsOutlined-Regular.otf");

/// A font every runtime registers before any custom font.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum BundledFont {
    /// Noto Sans: the Body and Heading default and the first text fallback.
    Body,
    /// Noto Sans Mono: the Monospace default and the second text fallback.
    Monospace,
    /// Material Icons Outlined: the standard icon glyphs.
    Icons,
}
crate::tables::variant_list! {
    /// Bundled fonts in registration order.
    pub(crate) const BUNDLED_FONTS: BundledFont = [Body, Monospace, Icons];
}
impl BundledFont {
    pub(crate) fn bytes(self) -> &'static [u8] {
        match self {
            Self::Body => NOTO_SANS,
            Self::Monospace => NOTO_SANS_MONO,
            Self::Icons => MATERIAL_ICONS,
        }
    }
    /// This font's registry index: bundled fonts are registered first, in
    /// [`BUNDLED_FONTS`] order, which `variant_list!` ties to the discriminant.
    pub(crate) const fn index(self) -> usize {
        self as usize
    }
}

/// Immutable font bytes. Static sources are borrowed; owned sources share their allocation.
/// Registration accepts single-face OpenType/TrueType fonts, including variable fonts
/// at their default axes. Collections, web fonts and color-only glyphs are unsupported.
/// The application owns file I/O and permission to redistribute its custom fonts.
#[derive(Clone)]
#[must_use]
pub struct FontData(pub(super) Arc<dyn AsRef<[u8]> + Send + Sync>);
impl FontData {
    /// The largest font, in bytes, that
    /// [`FontManager::register`](crate::FontManager::register) admits: 16 MiB.
    /// Larger data is rejected with [`FontError::TooLarge`] before it is parsed.
    pub const MAX_BYTES: usize = 16 * 1024 * 1024;
    /// Borrows embedded bytes without copying them. Validation happens on registration.
    pub fn from_static(bytes: &'static [u8]) -> Self {
        Self(Arc::new(bytes))
    }
    /// Returns the library's original body/heading or monospace font bytes.
    /// Useful for deriving an explicit registration independent of mutable defaults.
    /// These unmodified fonts retain their bundled SIL OFL notices.
    pub fn bundled(role: FontRole) -> Self {
        Self::from_static(match role {
            FontRole::Body | FontRole::Heading => BundledFont::Body.bytes(),
            FontRole::Monospace => BundledFont::Monospace.bytes(),
        })
    }
    /// Takes ownership of font bytes. Validation happens on registration.
    pub fn from_owned(bytes: Vec<u8>) -> Self {
        Self(Arc::new(bytes))
    }
}
impl fmt::Debug for FontData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FontData")
            .field("bytes", &self.0.as_ref().as_ref().len())
            .finish()
    }
}
/// Stable identity of one immutable registration in one runtime.
/// Clone to reuse it. IDs are UI-thread-only, never reused, and cannot be persisted
/// or used in another runtime. Holding an ID does not retain the runtime or font bytes.
///
/// ```compile_fail
/// # use pert_ui_runtime as pert_ui;
/// use pert_ui::FontId;
/// fn needs_send<T: Send>() {}
/// needs_send::<FontId>();
/// ```
#[derive(Clone, Debug)]
#[must_use]
pub struct FontId {
    pub(super) owner: Rc<()>,
    pub(super) index: usize,
}
impl PartialEq for FontId {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.owner, &other.owner) && self.index == other.index
    }
}
impl Eq for FontId {}

/// Default font selection, separate from the size chosen by [`crate::TextStyle`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FontRole {
    /// Body and caption text; initially bundled Noto Sans.
    Body,
    /// Heading text; inherits Body unless explicitly overridden.
    Heading,
    /// Monospace text and captions; initially bundled Noto Sans Mono.
    Monospace,
}
crate::tables::variant_list! {
    pub(crate) const FONT_ROLES: FontRole = [Body, Heading, Monospace];
}
impl FontRole {
    /// This role's bit in a font dependency mask. Font roles take the low bits
    /// and [`crate::Icon`] roles the bits above them in the same `u64`.
    pub(crate) fn mask(self) -> u64 {
        1 << self as usize
    }
}

/// Registration or icon validation failure. Invalid changes preserve prior defaults.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum FontError {
    /// Bytes are not a readable single-face outline font.
    #[error("invalid or unsupported font data")]
    InvalidData,
    /// Registration exceeds [`FontData::MAX_BYTES`] per font,
    /// [`FontManager::MAX_TOTAL_BYTES`](crate::FontManager::MAX_TOTAL_BYTES) in total or
    /// [`FontManager::MAX_CUSTOM_FONTS`](crate::FontManager::MAX_CUSTOM_FONTS) custom
    /// fonts.
    #[error("font registration exceeds its resource limit")]
    TooLarge,
    /// The font or glyph belongs to another runtime.
    #[error("font belongs to another runtime")]
    ForeignFont,
    /// The scalar has no glyph, is whitespace or is a control character.
    #[error("icon character is missing from its font")]
    MissingGlyph,
    /// No bundled Material Icons Outlined entry has the supplied name.
    #[error("unknown Material icon name")]
    UnknownIcon,
    /// A replacement set lists the same semantic icon more than once.
    #[error("duplicate icon override")]
    DuplicateIcon,
}

/// A validated character in one exact font, with no fallback to another font.
/// Created by [`crate::FontManager::icon_glyph`]. Font data stays owned by its runtime.
#[derive(Clone, Debug, PartialEq, Eq)]
#[must_use]
pub struct IconGlyph {
    pub(super) font: FontId,
    pub(super) character: char,
}
