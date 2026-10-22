//! Semantic icons with an overridable exact-font glyph mapping.
use super::fonts::IconGlyph;

/// A library icon role. Defaults use bundled Material Icons Outlined.
/// Override roles through [`crate::IconManager`]; controls and custom components
/// resolve the same mapping. Roles express appearance, not application actions.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum Icon {
    /// Close or dismiss.
    Close,
    /// Settings.
    Settings,
    /// Expand downward.
    ChevronDown,
    /// Collapse upward.
    ChevronUp,
    /// Navigate left.
    ChevronLeft,
    /// Navigate right or open a submenu.
    ChevronRight,
    /// Check mark.
    Check,
    /// Save.
    Save,
    /// Add.
    Add,
    /// Remove.
    Remove,
    /// More actions horizontally.
    MoreHorizontal,
    /// More actions vertically.
    MoreVertical,
    /// Download.
    Download,
    /// Link.
    Link,
    /// Information.
    Info,
    /// Warning.
    Warning,
    /// Error.
    Error,
    /// Power.
    Power,
    /// Search.
    Search,
    /// Refresh.
    Refresh,
    /// Delete.
    Delete,
    /// Checked checkbox.
    CheckboxChecked,
    /// Unchecked checkbox.
    CheckboxUnchecked,
    /// Selected radio option.
    RadioChecked,
    /// Unselected radio option.
    RadioUnchecked,
    /// Start or resume.
    PlayArrow,
    /// Pause, keeping the position to resume from.
    Pause,
    /// Stop.
    Stop,
    /// A filled circle, for example an armed or recording state.
    Record,
    /// Repeat or loop.
    Repeat,
    /// Undo.
    Undo,
    /// Redo.
    Redo,
    /// Edit.
    Edit,
    /// Adjust settings or levels.
    Tune,
    /// A timer or clock.
    Timer,
    /// A microphone.
    Microphone,
    /// Audible volume.
    VolumeOn,
    /// Muted volume.
    VolumeOff,
    /// Open the same content in a separate window.
    OpenInNew,
    /// A folder or directory.
    Folder,
    /// A file or document.
    File,
    /// Shown content, such as a revealed password.
    Visibility,
    /// Hidden content, such as a concealed password.
    VisibilityOff,
    /// Go back to the previous place, as a browser's back arrow.
    Back,
    /// Go forward to the next place, as a browser's forward arrow.
    Forward,
    /// A list of places visited before.
    History,
    /// Copy to the clipboard: two overlapping sheets.
    Copy,
    /// Cut to the clipboard: scissors.
    Cut,
    /// Paste from the clipboard: a clipboard.
    Paste,
    /// A duplicate of an item: a sheet with a plus on another sheet.
    Duplicate,
}
crate::tables::variant_list! {
    const ICONS: Icon = [
        Close,
        Settings,
        ChevronDown,
        ChevronUp,
        ChevronLeft,
        ChevronRight,
        Check,
        Save,
        Add,
        Remove,
        MoreHorizontal,
        MoreVertical,
        Download,
        Link,
        Info,
        Warning,
        Error,
        Power,
        Search,
        Refresh,
        Delete,
        CheckboxChecked,
        CheckboxUnchecked,
        RadioChecked,
        RadioUnchecked,
        PlayArrow,
        Pause,
        Stop,
        Record,
        Repeat,
        Undo,
        Redo,
        Edit,
        Tune,
        Timer,
        Microphone,
        VolumeOn,
        VolumeOff,
        OpenInNew,
        Folder,
        File,
        Visibility,
        VisibilityOff,
        Back,
        Forward,
        History,
        Copy,
        Cut,
        Paste,
        Duplicate,
    ];
}
// Font roles and icons share one `u64` dependency mask; past 64 bits a shift
// would panic in debug builds and alias another role in release builds.
const _: () = assert!(
    super::fonts::FONT_ROLES.len() + ICONS.len() <= u64::BITS as usize,
    "font roles and icons no longer fit one u64 dependency mask"
);
impl Icon {
    /// Every icon role, in declaration order, for example to show them all.
    pub const ALL: &'static [Self] = &ICONS;

    /// This icon's bit in a font dependency mask, above the font roles' bits.
    pub(crate) fn mask(self) -> u64 {
        1 << (super::fonts::FONT_ROLES.len() + self as usize)
    }
    pub(crate) fn character(self) -> char {
        match self {
            Self::Close => '\u{e5cd}',
            Self::Settings => '\u{e8b8}',
            Self::ChevronDown => '\u{e5cf}',
            Self::ChevronUp => '\u{e5ce}',
            Self::ChevronLeft => '\u{e5cb}',
            Self::ChevronRight => '\u{e5cc}',
            Self::Check => '\u{e5ca}',
            Self::Save => '\u{e161}',
            Self::Add => '\u{e145}',
            Self::Remove => '\u{e15b}',
            Self::MoreHorizontal => '\u{eae1}',
            Self::MoreVertical => '\u{e5d4}',
            Self::Download => '\u{f090}',
            Self::Link => '\u{e157}',
            Self::Info => '\u{e88e}',
            Self::Warning => '\u{e002}',
            Self::Error => '\u{e000}',
            Self::Power => '\u{e8ac}',
            Self::Search => '\u{e8b6}',
            Self::Refresh => '\u{e5d5}',
            Self::Delete => '\u{e872}',
            Self::CheckboxChecked => '\u{e834}',
            Self::CheckboxUnchecked => '\u{e835}',
            Self::RadioChecked => '\u{e837}',
            Self::RadioUnchecked => '\u{e836}',
            Self::PlayArrow => '\u{e037}',
            Self::Pause => '\u{e034}',
            Self::Stop => '\u{e047}',
            Self::Record => '\u{e061}',
            Self::Repeat => '\u{e040}',
            Self::Undo => '\u{e166}',
            Self::Redo => '\u{e15a}',
            Self::Edit => '\u{e3c9}',
            Self::Tune => '\u{e429}',
            Self::Timer => '\u{e425}',
            Self::Microphone => '\u{e029}',
            Self::VolumeOn => '\u{e050}',
            Self::VolumeOff => '\u{e04f}',
            Self::OpenInNew => '\u{e89e}',
            Self::Folder => '\u{e2c7}',
            Self::File => '\u{e24d}',
            Self::Visibility => '\u{e8f4}',
            Self::VisibilityOff => '\u{e8f5}',
            Self::Back => '\u{e5c4}',
            Self::Forward => '\u{e5c8}',
            Self::History => '\u{e889}',
            Self::Copy => '\u{f08a}',
            Self::Cut => '\u{f08b}',
            Self::Paste => '\u{f098}',
            Self::Duplicate => '\u{e3bb}',
        }
    }
}

/// An icon role resolved at presentation time, or a validated custom glyph.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum IconSource {
    /// Follows this runtime's overrides for a library role.
    Standard(Icon),
    /// Uses this exact registered font and character.
    Custom(IconGlyph),
}
impl From<Icon> for IconSource {
    fn from(value: Icon) -> Self {
        Self::Standard(value)
    }
}
impl From<IconGlyph> for IconSource {
    fn from(value: IconGlyph) -> Self {
        Self::Custom(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every role's default character has a glyph in the bundled icon font,
    /// so a new role cannot silently draw nothing.
    #[test]
    fn every_role_has_a_bundled_glyph() {
        let bytes = include_bytes!("../../assets/MaterialIconsOutlined-Regular.otf");
        let font = swash::FontRef::from_index(bytes, 0).expect("bundled icon font");
        for icon in Icon::ALL {
            assert_ne!(
                font.charmap().map(icon.character()),
                0,
                "{icon:?} has no glyph"
            );
        }
    }
}
