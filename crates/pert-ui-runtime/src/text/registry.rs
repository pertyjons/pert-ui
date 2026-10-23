//! Immutable registrations and mutable semantic mappings owned by the text service.
use super::fonts::{BUNDLED_FONTS, BundledFont, FONT_ROLES};
use super::*;
use crate::{FontData, FontError, FontId, FontRole, Icon, IconGlyph, IconSource};
use std::rc::Rc;

pub(crate) struct RegisteredFont {
    pub(super) data: FontData,
    pub(super) family: String,
}
/// Explicit default font overrides by role; `None` follows the built-in rule.
#[derive(Clone, Copy, Default)]
struct Defaults {
    body: Option<usize>,
    heading: Option<usize>,
    monospace: Option<usize>,
}
impl Defaults {
    fn slot(&mut self, role: FontRole) -> &mut Option<usize> {
        match role {
            FontRole::Body => &mut self.body,
            FontRole::Heading => &mut self.heading,
            FontRole::Monospace => &mut self.monospace,
        }
    }
    fn get(mut self, role: FontRole) -> Option<usize> {
        *self.slot(role)
    }
}
pub(crate) struct Registry {
    pub(super) owner: Rc<()>,
    pub(super) fonts: Vec<RegisteredFont>,
    defaults: Defaults,
    icons: BTreeMap<u64, IconGlyph>,
    bytes: usize,
}
impl Registry {
    pub(crate) fn new(owner: Rc<()>) -> Self {
        Self {
            owner,
            fonts: Vec::new(),
            defaults: Defaults::default(),
            icons: BTreeMap::new(),
            bytes: 0,
        }
    }
    pub(crate) fn register(
        &mut self,
        context: &mut parley::FontContext,
        data: FontData,
    ) -> Result<FontId, FontError> {
        let bytes = data.0.as_ref().as_ref();
        if bytes.len() > FontData::MAX_BYTES
            || self.bytes + bytes.len() > crate::FontManager::MAX_TOTAL_BYTES
            || self.fonts.len() >= BUNDLED_FONTS.len() + crate::FontManager::MAX_CUSTOM_FONTS
        {
            return Err(FontError::TooLarge);
        }
        // A collection needs an explicit face-selection API; never silently select one.
        if bytes.starts_with(b"ttcf") || swash::FontRef::from_index(bytes, 0).is_none() {
            return Err(FontError::InvalidData);
        }
        use swash::TableProvider;
        let face = swash::FontRef::from_index(bytes, 0).ok_or(FontError::InvalidData)?;
        if ![b"glyf", b"CFF ", b"CFF2"]
            .iter()
            .any(|tag| face.table_by_tag(u32::from_be_bytes(**tag)).is_some())
        {
            return Err(FontError::InvalidData);
        }
        let index = self.fonts.len();
        let family = format!("pert-ui-font-{index}");
        let registered = context.collection.register_fonts(
            parley::fontique::Blob::new(data.0.clone()),
            Some(parley::fontique::FontInfoOverride {
                family_name: Some(&family),
                ..Default::default()
            }),
        );
        if registered.len() != 1 || registered[0].1.len() != 1 {
            return Err(FontError::InvalidData);
        }
        self.bytes += bytes.len();
        self.fonts.push(RegisteredFont { data, family });
        Ok(FontId {
            owner: self.owner.clone(),
            index,
        })
    }
    pub(crate) fn validate(&self, id: &FontId) -> Result<usize, FontError> {
        if !Rc::ptr_eq(&id.owner, &self.owner) || id.index >= self.fonts.len() {
            Err(FontError::ForeignFont)
        } else {
            Ok(id.index)
        }
    }
    pub(crate) fn default_index(&self, role: FontRole) -> usize {
        self.defaults.get(role).unwrap_or_else(|| match role {
            FontRole::Body => BundledFont::Body.index(),
            FontRole::Heading => self.default_index(FontRole::Body),
            FontRole::Monospace => BundledFont::Monospace.index(),
        })
    }
    pub(crate) fn material_glyph(&self, character: char) -> Result<IconGlyph, FontError> {
        self.glyph(
            &FontId {
                owner: self.owner.clone(),
                index: BundledFont::Icons.index(),
            },
            character,
        )
    }
    pub(crate) fn default_id(&self, role: FontRole) -> FontId {
        FontId {
            owner: self.owner.clone(),
            index: self.default_index(role),
        }
    }
    pub(crate) fn set_default(
        &mut self,
        role: FontRole,
        id: Option<&FontId>,
    ) -> Result<u64, FontError> {
        let index = id.map(|id| self.validate(id)).transpose()?;
        let before = FONT_ROLES.map(|r| self.default_index(r));
        *self.defaults.slot(role) = index;
        Ok(FONT_ROLES.iter().zip(before).fold(0, |mask, (r, before)| {
            mask | if before != self.default_index(*r) {
                r.mask()
            } else {
                0
            }
        }))
    }
    pub(crate) fn glyph(&self, font: &FontId, character: char) -> Result<IconGlyph, FontError> {
        let index = self.validate(font)?;
        let face = swash::FontRef::from_index(self.fonts[index].data.0.as_ref().as_ref(), 0)
            .ok_or(FontError::InvalidData)?;
        if character.is_control() || character.is_whitespace() || face.charmap().map(character) == 0
        {
            return Err(FontError::MissingGlyph(character));
        }
        Ok(IconGlyph {
            font: font.clone(),
            character,
        })
    }
    pub(crate) fn icon(&self, source: &IconSource) -> Result<(usize, char, u64), FontError> {
        match source {
            IconSource::Custom(glyph) => Ok((self.validate(&glyph.font)?, glyph.character, 0)),
            IconSource::Standard(role) => {
                let (index, ch) = self
                    .icons
                    .get(&role.mask())
                    .map_or((BundledFont::Icons.index(), role.character()), |g| {
                        (g.font.index, g.character)
                    });
                Ok((index, ch, role.mask()))
            }
        }
    }
    pub(crate) fn set_icon(
        &mut self,
        role: Icon,
        glyph: Option<IconGlyph>,
    ) -> Result<u64, FontError> {
        if let Some(g) = &glyph {
            self.validate(&g.font)?;
        }
        let before = self.icon(&role.into())?;
        if let Some(g) = glyph {
            self.icons.insert(role.mask(), g);
        } else {
            self.icons.remove(&role.mask());
        }
        Ok(if self.icon(&role.into())? != before {
            role.mask()
        } else {
            0
        })
    }
    pub(crate) fn set_icons(&mut self, icons: Vec<(Icon, IconGlyph)>) -> Result<u64, FontError> {
        let mut next = BTreeMap::new();
        for (role, glyph) in icons {
            self.validate(&glyph.font)?;
            if next.insert(role.mask(), glyph).is_some() {
                return Err(FontError::DuplicateIcon);
            }
        }
        let mut changed = 0;
        for key in self.icons.keys().chain(next.keys()) {
            if self.icons.get(key) != next.get(key) {
                changed |= key;
            }
        }
        self.icons = next;
        Ok(changed)
    }
}
