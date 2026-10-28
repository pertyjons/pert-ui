//! UI-local font mutation and layout/paint dependency notifications.
use super::*;
use crate::{FontData, FontId, FontRole, Icon, IconGlyph};

#[derive(Default)]
pub(super) struct FontSubscribers(pub BTreeMap<u64, Weak<Node>>);
impl FontSubscribers {
    pub(super) fn track(&mut self, node: &Rc<Node>) {
        self.0.insert(node.serial, Rc::downgrade(node));
    }
    fn notify(&mut self, changed: u64) {
        if changed == 0 {
            return;
        }
        // Every subscriber's reads are judged before any is invalidated: a
        // layout invalidation clears the measurement reads of the node and its
        // ancestors, which would otherwise hide an ancestor's own reads.
        let mut invalidated = Vec::new();
        self.0.retain(|_, weak| {
            let Some(node) = weak.upgrade() else {
                return false;
            };
            if node.font_layout.iter().any(|r| r.get() & changed != 0) {
                invalidated.push((node, Invalidation::Layout));
            } else if node.font_paint.get() & changed != 0 {
                invalidated.push((node, Invalidation::Paint));
            }
            true
        });
        for (node, kind) in invalidated {
            if kind == Invalidation::Layout && node.visible.get() {
                node.window().theme_geometry_change.set(true);
            }
            node.invalidate(kind);
        }
    }
}
/// UI-thread-only access to one runtime's font registry. Clones are weak handles.
/// Registration retains bytes until runtime shutdown (or longer through live text
/// layouts), without global state, disk I/O or system-font discovery. Mutations
/// are allowed at initialization and in event callbacks, never during update,
/// layout, paint or inspection. All operations return `Closed` after shutdown.
#[derive(Clone)]
#[must_use]
pub struct FontManager {
    shared: Weak<Shared>,
}

/// Shows no registered fonts: they are read through the manager, which fails
/// after shutdown.
impl std::fmt::Debug for FontManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FontManager").finish_non_exhaustive()
    }
}
impl FontManager {
    /// The most custom fonts one runtime admits: 256. Bundled fonts do not count;
    /// registrations are never removed, so the budget lasts until shutdown.
    pub const MAX_CUSTOM_FONTS: usize = 256;
    /// The most font bytes one runtime holds: 64 MiB, including the bundled
    /// fonts. Registrations are never removed, so the budget lasts until
    /// shutdown.
    pub const MAX_TOTAL_BYTES: usize = 64 * 1024 * 1024;
    /// Registers a single-face TTF/OTF under a unique internal name. Variable fonts
    /// use default axes. Does not change defaults or schedule a frame.
    /// At most [`Self::MAX_CUSTOM_FONTS`] custom registrations of at most
    /// [`FontData::MAX_BYTES`] each are admitted, within the
    /// [`Self::MAX_TOTAL_BYTES`] total that includes the bundled fonts.
    ///
    /// # Errors
    /// Returns [`RuntimeError::Closed`] after the runtime is dropped or shut down, and
    /// `WriteDuringUpdate`, `WriteDuringLayout`, `WriteDuringPaint`,
    /// `WriteDuringInspection` or `WriteDuringDelivery` when called from inside that phase;
    /// [`RuntimeError::Font`] with
    /// [`FontError::InvalidData`](crate::FontError::InvalidData) for data that is not a
    /// readable single-face outline font, and
    /// [`FontError::TooLarge`](crate::FontError::TooLarge) beyond a font-count or data
    /// limit.
    pub fn register(&self, data: FontData) -> Result<FontId, RuntimeError> {
        let shared = self.shared.upgrade().ok_or(RuntimeError::Closed)?;
        shared.writable()?;
        shared
            .text
            .with_service(|s| Ok(s.registry.register(&mut s.fonts, data)?))
    }
    /// Returns the currently resolved font for a semantic role without subscribing.
    ///
    /// # Errors
    /// Returns [`RuntimeError::Closed`] after the runtime is dropped or shut down.
    pub fn default_font(&self, role: FontRole) -> Result<FontId, RuntimeError> {
        let shared = self.shared.upgrade().ok_or(RuntimeError::Closed)?;
        shared.readable()?;
        shared
            .text
            .with_service(|s| Ok(s.registry.default_id(role)))
    }
    /// Changes a role's font. Only readers of changed roles are invalidated;
    /// explicit-font text is unaffected. Heading inherits Body until overridden.
    ///
    /// # Errors
    /// Returns [`RuntimeError::Closed`] after the runtime is dropped or shut down, and
    /// `WriteDuringUpdate`, `WriteDuringLayout`, `WriteDuringPaint`,
    /// `WriteDuringInspection` or `WriteDuringDelivery` when called from inside that phase;
    /// and [`RuntimeError::Font`] with
    /// [`FontError::ForeignFont`](crate::FontError::ForeignFont) for a font of another
    /// runtime; the previous default is kept.
    pub fn set_default(&self, role: FontRole, font: FontId) -> Result<(), RuntimeError> {
        self.change_default(role, Some(font))
    }
    /// Restores bundled Body/Monospace or Heading's inheritance from Body.
    ///
    /// # Errors
    /// Returns [`RuntimeError::Closed`] after the runtime is dropped or shut down, and
    /// `WriteDuringUpdate`, `WriteDuringLayout`, `WriteDuringPaint`,
    /// `WriteDuringInspection` or `WriteDuringDelivery` when called from inside that phase.
    pub fn reset_default(&self, role: FontRole) -> Result<(), RuntimeError> {
        self.change_default(role, None)
    }
    fn change_default(&self, role: FontRole, font: Option<FontId>) -> Result<(), RuntimeError> {
        let shared = self.shared.upgrade().ok_or(RuntimeError::Closed)?;
        shared.writable()?;
        let changed = shared
            .text
            .with_service(|s| Ok(s.registry.set_default(role, font.as_ref())?))?;
        shared.font_subscribers.borrow_mut().notify(changed);
        Ok(())
    }
    /// Resolves an upstream snake_case name from the complete bundled Material
    /// Icons Outlined catalog, such as `folder_open`.
    /// The result uses the bundled font exactly; use `Icon` roles for live overrides.
    ///
    /// # Errors
    /// Returns [`RuntimeError::Closed`] after the runtime is dropped or shut down, and
    /// [`RuntimeError::Font`] with
    /// [`FontError::UnknownIcon`](crate::FontError::UnknownIcon) for a name the catalog
    /// lacks.
    pub fn material_icon(&self, name: &str) -> Result<IconGlyph, RuntimeError> {
        let shared = self.shared.upgrade().ok_or(RuntimeError::Closed)?;
        shared.readable()?;
        let character = include_str!("../../assets/MaterialIconsOutlined-Regular.codepoints")
            .lines()
            .find_map(|line| {
                let (key, code) = line.split_once(' ')?;
                (key == name)
                    .then(|| u32::from_str_radix(code, 16).ok().and_then(char::from_u32))
                    .flatten()
            })
            .ok_or_else(|| crate::FontError::UnknownIcon(name.to_owned()))?;
        shared
            .text
            .with_service(|s| Ok(s.registry.material_glyph(character)?))
    }
    /// Validates one Unicode scalar in an exact registered font.
    /// No fallback is attempted.
    ///
    /// # Errors
    /// Returns [`RuntimeError::Closed`] after the runtime is dropped or shut down, and
    /// [`RuntimeError::Font`] with
    /// [`FontError::ForeignFont`](crate::FontError::ForeignFont) for a font of another
    /// runtime, [`FontError::MissingGlyph`](crate::FontError::MissingGlyph) for a character
    /// the font lacks, whitespace or a control character, and
    /// [`FontError::InvalidData`](crate::FontError::InvalidData) when the font cannot be
    /// read.
    pub fn icon_glyph(&self, font: FontId, character: char) -> Result<IconGlyph, RuntimeError> {
        let shared = self.shared.upgrade().ok_or(RuntimeError::Closed)?;
        shared.readable()?;
        shared
            .text
            .with_service(|s| Ok(s.registry.glyph(&font, character)?))
    }
}
/// UI-thread-only overrides shared by built-in and custom icon users.
/// Unspecified roles use bundled Material Icons Outlined. Clones weakly reference
/// the runtime. Mutations have the same lifecycle restrictions as [`FontManager`].
#[derive(Clone)]
#[must_use]
pub struct IconManager {
    shared: Weak<Shared>,
}

/// Shows no glyph overrides.
impl std::fmt::Debug for IconManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IconManager").finish_non_exhaustive()
    }
}
impl IconManager {
    /// Replaces one role's glyph. Different icon fonts need explicit codepoint mappings.
    ///
    /// # Errors
    /// Returns [`RuntimeError::Closed`] after the runtime is dropped or shut down, and
    /// `WriteDuringUpdate`, `WriteDuringLayout`, `WriteDuringPaint`,
    /// `WriteDuringInspection` or `WriteDuringDelivery` when called from inside that phase;
    /// and [`RuntimeError::Font`] with
    /// [`FontError::ForeignFont`](crate::FontError::ForeignFont) for a glyph from another
    /// runtime; prior state is preserved.
    pub fn set(&self, role: Icon, glyph: IconGlyph) -> Result<(), RuntimeError> {
        self.change(role, Some(glyph))
    }
    /// Restores the bundled glyph for one role.
    ///
    /// # Errors
    /// Returns [`RuntimeError::Closed`] after the runtime is dropped or shut down, and
    /// `WriteDuringUpdate`, `WriteDuringLayout`, `WriteDuringPaint`,
    /// `WriteDuringInspection` or `WriteDuringDelivery` when called from inside that phase.
    pub fn reset(&self, role: Icon) -> Result<(), RuntimeError> {
        self.change(role, None)
    }
    fn change(&self, role: Icon, glyph: Option<IconGlyph>) -> Result<(), RuntimeError> {
        let shared = self.shared.upgrade().ok_or(RuntimeError::Closed)?;
        shared.writable()?;
        let changed = shared
            .text
            .with_service(|s| Ok(s.registry.set_icon(role, glyph)?))?;
        shared.font_subscribers.borrow_mut().notify(changed);
        Ok(())
    }
    /// Atomically replaces all overrides. Omitted roles use bundled glyphs.
    /// An empty iterator
    /// restores every default. Only components reading changed roles are invalidated.
    ///
    /// # Errors
    /// Returns [`RuntimeError::Closed`] after the runtime is dropped or shut down, and
    /// `WriteDuringUpdate`, `WriteDuringLayout`, `WriteDuringPaint`,
    /// `WriteDuringInspection` or `WriteDuringDelivery` when called from inside that phase;
    /// and [`RuntimeError::Font`] with
    /// [`FontError::ForeignFont`](crate::FontError::ForeignFont) for a glyph from another
    /// runtime or [`FontError::DuplicateIcon`](crate::FontError::DuplicateIcon) for a role
    /// listed twice. Either rejects the entire change.
    pub fn set_overrides(
        &self,
        icons: impl IntoIterator<Item = (Icon, IconGlyph)>,
    ) -> Result<(), RuntimeError> {
        // Exhaust user iteration before borrowing the registry or checking lifecycle:
        // an iterator may itself call UI APIs or drop the runtime.
        let icons = icons.into_iter().collect();
        let shared = self.shared.upgrade().ok_or(RuntimeError::Closed)?;
        shared.writable()?;
        let changed = shared
            .text
            .with_service(|s| Ok(s.registry.set_icons(icons)?))?;
        shared.font_subscribers.borrow_mut().notify(changed);
        Ok(())
    }
}
impl Runtime {
    /// Font registration and semantic defaults for this runtime; no work is scheduled.
    pub fn fonts(&self) -> FontManager {
        FontManager {
            shared: Rc::downgrade(&self.shared),
        }
    }
    /// Standard icon mappings for this runtime; no work is scheduled.
    pub fn icons(&self) -> IconManager {
        IconManager {
            shared: Rc::downgrade(&self.shared),
        }
    }
    pub(crate) fn track_fonts(&self, node: &Rc<Node>) {
        self.shared.font_subscribers.borrow_mut().track(node);
    }
}
