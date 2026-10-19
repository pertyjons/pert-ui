//! The text the library itself shows or names, keyed by a typed role, with
//! English defaults an application can replace.
use std::borrow::Cow;
use std::fmt::Write as _;
use std::rc::Rc;

/// One string the library itself shows or reports as a name: a context-menu
/// item, a placeholder, an accessibility name, a status or a message.
///
/// Text the application supplies (labels, titles, item names) never comes from
/// this table; only the library's own words do. A role whose text contains
/// placeholders, such as [`Self::BackTo`] with `{place}`, is filled in through
/// [`UiStrings::format`] with a [`UiMessage`]; [`Self::placeholders`] lists them.
///
/// Developer-facing text is not part of the table: error `Display` messages,
/// panics, inspection property names and shortcut key names (`Ctrl`, `Page Up`),
/// which follow platform conventions.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum StringRole {
    /// The editor context-menu item that copies the selection.
    TextCopy,
    /// The editor context-menu item that cuts the selection.
    TextCut,
    /// The editor context-menu item that pastes over the selection.
    TextPaste,
    /// The editor context-menu item that pastes before the selection.
    TextPasteBefore,
    /// The editor context-menu item that pastes after the selection.
    TextPasteAfter,
    /// The context-menu item that selects all text, offered by
    /// [`SelectableText`](crate::SelectableText).
    TextSelectAll,
    /// The description of a text editor whose draft is not saved yet.
    TextModified,
    /// A choice field's placeholder while nothing is chosen (`Select`,
    /// `MultiSelect`).
    SelectPlaceholder,
    /// A search field's placeholder inside a picker or chooser.
    SearchPlaceholder,
    /// The text a filtered list shows when nothing matches.
    NoMatches,
    /// The accessibility name of a `QuickPick` without an application name.
    PickerName,
    /// The accessibility name of a picker's result list.
    PickerResults,
    /// The accessibility name of the command palette.
    PaletteName,
    /// The command palette's search placeholder.
    PalettePlaceholder,
    /// The command palette's text when no command matches.
    PaletteNoMatches,
    /// The accessibility name of the command palette's list.
    PaletteCommands,
    /// The marker on a command that is currently disabled.
    CommandDisabled,
    /// The marker on a command whose enabled state could not be read.
    CommandUnavailable,
    /// The accessibility name of a split pane's divider.
    SplitterName,
    /// The accessibility name of a history navigator.
    HistoryName,
    /// The history navigator's back button.
    HistoryBack,
    /// The history navigator's forward button.
    HistoryForward,
    /// The history navigator's list button and the list's title.
    HistoryList,
    /// The accessibility name of the history list.
    HistoryListName,
    /// The history list's search placeholder.
    HistorySearch,
    /// The kind label of history entries in a picker.
    HistoryKind,
    /// The detail that marks the current entry in the history list.
    HistoryCurrent,
    /// The refusal shown for an entry the history dropped while its list was
    /// open.
    HistoryEntryGone,
    /// The accessibility name of a file browser.
    FileBrowserName,
    /// The file browser's filter field, placeholder and name.
    FileBrowserFilter,
    /// The accessibility name of the file browser's folder tree.
    FileBrowserFolders,
    /// The accessibility name of the file browser's entry list.
    FileBrowserEntries,
    /// The row shown while a folder's listing is requested.
    FileBrowserLoading,
    /// The row shown for a folder without entries.
    FileBrowserEmptyFolder,
    /// The row shown when the filter matches no entry.
    FileBrowserNoMatches,
    /// The accessibility name of a breadcrumb.
    BreadcrumbName,
    /// The breadcrumb's button for the segments its overflow hides.
    BreadcrumbOverflow,
    /// The accessibility name of a color picker without an application name.
    ColorPickerName,
    /// The default name of the color a picker edits.
    ColorName,
    /// The color picker's hexadecimal field label.
    ColorHex,
    /// The color picker's saturation and value area.
    ColorArea,
    /// The color picker's hue strip.
    ColorHue,
    /// The color picker's alpha strip.
    ColorAlpha,
    /// The color picker's red channel field label.
    ColorRed,
    /// The color picker's green channel field label.
    ColorGreen,
    /// The color picker's blue channel field label.
    ColorBlue,
    /// The color picker's alpha channel field label.
    ColorAlphaChannel,
    /// The refusal of hexadecimal text with alpha in a picker without alpha.
    ColorNoAlpha,
    /// A dialog's cancel action when the application names none.
    DialogCancel,
    /// A toast's dismiss action when the application names none.
    ToastDismiss,
    /// A toolbar's overflow menu button.
    ToolbarMore,
    /// A floating window's minimize-or-restore button.
    WindowMinimize,
    /// A floating window's close button.
    WindowClose,
    /// The accessibility name of a collection filter's field and its
    /// placeholder when the application names neither.
    FilterName,
    /// The name of the file browser's folder filter field.
    FileBrowserFolderFilter,
    /// The tool window options item that moves a tool window to the left edge.
    ToolWindowMoveLeft,
    /// The tool window options item that moves a tool window to the right edge.
    ToolWindowMoveRight,
    /// The tool window options item that moves a tool window to the top edge.
    ToolWindowMoveTop,
    /// The tool window options item that moves a tool window to the bottom
    /// edge.
    ToolWindowMoveBottom,
    /// The tool window options item that moves a tool window to the start
    /// group of its stripe.
    ToolWindowMoveStartGroup,
    /// The tool window options item that moves a tool window to the end group
    /// of its stripe.
    ToolWindowMoveEndGroup,
    /// The tool window options item that moves a tool window earlier in its
    /// group.
    ToolWindowMoveEarlier,
    /// The tool window options item that moves a tool window later in its
    /// group.
    ToolWindowMoveLater,
    /// The tool window options item that switches overlay mode.
    ToolWindowOverlay,
    /// The accessibility name of the left edge's tool window stripe.
    ToolWindowStripeLeft,
    /// The accessibility name of the right edge's tool window stripe.
    ToolWindowStripeRight,
    /// The accessibility name of the top edge's tool window stripe.
    ToolWindowStripeTop,
    /// The accessibility name of the bottom edge's tool window stripe.
    ToolWindowStripeBottom,
    /// The accessibility name of the divider resizing the left edge's tool
    /// windows.
    ToolWindowResizeLeft,
    /// The accessibility name of the divider resizing the right edge's tool
    /// windows.
    ToolWindowResizeRight,
    /// The accessibility name of the divider resizing the top edge's tool
    /// windows.
    ToolWindowResizeTop,
    /// The accessibility name of the divider resizing the bottom edge's tool
    /// windows.
    ToolWindowResizeBottom,
    /// The default name of a chip's remove button: `{item}` is the chip's text.
    RemoveItem,
    /// The back button's tooltip: `{place}` is the previous entry's label.
    BackTo,
    /// The forward button's tooltip: `{place}` is the next entry's label.
    ForwardTo,
    /// The refusal of text shorter than a field's minimum length: `{count}`.
    /// The English default uses count-neutral wording without plural selection.
    AtLeastCharacters,
    /// The refusal of text longer than a field's maximum length: `{count}`.
    /// The English default uses count-neutral wording without plural selection.
    AtMostCharacters,
    /// The refusal of text that is not a hexadecimal color: `{reason}`.
    NotHexColor,
    /// The color area's spoken value: `{saturation}` and `{value}` percent.
    ColorAreaValue,
    /// The palette's report that a command ran: `{command}`.
    CommandRan,
    /// The palette's report that a command is disabled: `{command}`.
    CommandNotAvailable,
    /// The palette's report that a dialog blocks a command: `{command}`.
    CommandBlocked,
    /// The palette's report that a command does not repeat: `{command}`.
    CommandNoRepeat,
    /// The palette's report that a command failed: `{command}` and `{reason}`.
    CommandFailed,
    /// A file browser row for an entry that could not be read: `{name}` and
    /// `{reason}`.
    EntryFailed,
    /// The name of a tool window's options menu: `{title}` is its title.
    ToolWindowOptions,
    /// The name of a tool window's hide button: `{title}` is its title.
    ToolWindowHide,
    /// The tool window options-menu row that docks a floating or detached
    /// tool window at its edge.
    ToolWindowDock,
    /// The tool window options-menu row that floats a tool window over its
    /// layout.
    ToolWindowFloat,
    /// The tool window options-menu row that opens a tool window in its own
    /// native window.
    ToolWindowOpenWindow,
}

crate::tables::variant_list! {
    /// Every string role in declaration order, checked complete at compile time.
    pub(crate) const STRING_ROLES: StringRole = [
        TextCopy, TextCut, TextPaste, TextPasteBefore, TextPasteAfter, TextSelectAll, TextModified,
        SelectPlaceholder, SearchPlaceholder, NoMatches, PickerName, PickerResults,
        PaletteName, PalettePlaceholder, PaletteNoMatches, PaletteCommands,
        CommandDisabled, CommandUnavailable, SplitterName, HistoryName, HistoryBack,
        HistoryForward, HistoryList, HistoryListName, HistorySearch, HistoryKind,
        HistoryCurrent, HistoryEntryGone, FileBrowserName, FileBrowserFilter,
        FileBrowserFolders, FileBrowserEntries, FileBrowserLoading,
        FileBrowserEmptyFolder, FileBrowserNoMatches, BreadcrumbName,
        BreadcrumbOverflow, ColorPickerName, ColorName, ColorHex, ColorArea,
        ColorHue, ColorAlpha, ColorRed, ColorGreen, ColorBlue, ColorAlphaChannel,
        ColorNoAlpha, DialogCancel, ToastDismiss, ToolbarMore, WindowMinimize,
        WindowClose, FilterName, FileBrowserFolderFilter, ToolWindowMoveLeft,
        ToolWindowMoveRight, ToolWindowMoveTop, ToolWindowMoveBottom,
        ToolWindowMoveStartGroup, ToolWindowMoveEndGroup, ToolWindowMoveEarlier,
        ToolWindowMoveLater, ToolWindowOverlay, ToolWindowStripeLeft,
        ToolWindowStripeRight, ToolWindowStripeTop, ToolWindowStripeBottom,
        ToolWindowResizeLeft, ToolWindowResizeRight, ToolWindowResizeTop,
        ToolWindowResizeBottom, RemoveItem, BackTo, ForwardTo, AtLeastCharacters,
        AtMostCharacters, NotHexColor, ColorAreaValue, CommandRan,
        CommandNotAvailable, CommandBlocked, CommandNoRepeat, CommandFailed,
        EntryFailed, ToolWindowOptions, ToolWindowHide,
        ToolWindowDock, ToolWindowFloat, ToolWindowOpenWindow,
    ];
}
const STRING_ROLE_COUNT: usize = STRING_ROLES.len();

impl StringRole {
    /// Every role, in declaration order.
    pub const ALL: &'static [Self] = &STRING_ROLES;

    /// The placeholders this role's text must contain, each written in braces
    /// (`{place}`); empty for plain text.
    #[must_use]
    pub const fn placeholders(self) -> &'static [&'static str] {
        match self {
            Self::RemoveItem => &["item"],
            Self::BackTo | Self::ForwardTo => &["place"],
            Self::AtLeastCharacters | Self::AtMostCharacters => &["count"],
            Self::NotHexColor => &["reason"],
            Self::ColorAreaValue => &["saturation", "value"],
            Self::CommandRan
            | Self::CommandNotAvailable
            | Self::CommandBlocked
            | Self::CommandNoRepeat => &["command"],
            Self::CommandFailed => &["command", "reason"],
            Self::EntryFailed => &["name", "reason"],
            Self::ToolWindowOptions | Self::ToolWindowHide => &["title"],
            _ => &[],
        }
    }
}

/// English defaults by role. Rows may appear in any order; [`UiStrings`]
/// places each at its role's index, and the const evaluation below rejects a
/// missing or duplicated role, or a template whose placeholders do not match
/// its role, at compile time.
const ENGLISH: [(StringRole, &str); STRING_ROLE_COUNT] = [
    (StringRole::TextCopy, "Copy"),
    (StringRole::TextCut, "Cut"),
    (StringRole::TextPaste, "Paste"),
    (StringRole::TextPasteBefore, "Paste Before"),
    (StringRole::TextPasteAfter, "Paste After"),
    (StringRole::TextSelectAll, "Select All"),
    (StringRole::TextModified, "Unsaved changes"),
    (StringRole::SelectPlaceholder, "Select…"),
    (StringRole::SearchPlaceholder, "Search"),
    (StringRole::NoMatches, "No matches"),
    (StringRole::PickerName, "Quick pick"),
    (StringRole::PickerResults, "Results"),
    (StringRole::PaletteName, "Command palette"),
    (StringRole::PalettePlaceholder, "Type a command"),
    (StringRole::PaletteNoMatches, "No matching commands"),
    (StringRole::PaletteCommands, "Commands"),
    (StringRole::CommandDisabled, "Disabled"),
    (StringRole::CommandUnavailable, "Unavailable"),
    (StringRole::SplitterName, "Resize panes"),
    (StringRole::HistoryName, "Navigation history"),
    (StringRole::HistoryBack, "Back"),
    (StringRole::HistoryForward, "Forward"),
    (StringRole::HistoryList, "History"),
    (StringRole::HistoryListName, "Visited places"),
    (StringRole::HistorySearch, "Search history"),
    (StringRole::HistoryKind, "History"),
    (StringRole::HistoryCurrent, "Current"),
    (
        StringRole::HistoryEntryGone,
        "That place is no longer in the history.",
    ),
    (StringRole::FileBrowserName, "File browser"),
    (StringRole::FileBrowserFilter, "Filter entries"),
    (StringRole::FileBrowserFolders, "Folders"),
    (StringRole::FileBrowserEntries, "Entries"),
    (StringRole::FileBrowserLoading, "Loading…"),
    (StringRole::FileBrowserEmptyFolder, "This folder is empty"),
    (
        StringRole::FileBrowserNoMatches,
        "No entries match the filter",
    ),
    (StringRole::BreadcrumbName, "Breadcrumb"),
    (StringRole::BreadcrumbOverflow, "Show hidden segments"),
    (StringRole::ColorPickerName, "Color picker"),
    (StringRole::ColorName, "Color"),
    (StringRole::ColorHex, "Hex"),
    (StringRole::ColorArea, "Saturation and value"),
    (StringRole::ColorHue, "Hue"),
    (StringRole::ColorAlpha, "Alpha"),
    (StringRole::ColorRed, "R"),
    (StringRole::ColorGreen, "G"),
    (StringRole::ColorBlue, "B"),
    (StringRole::ColorAlphaChannel, "A"),
    (StringRole::ColorNoAlpha, "This color has no alpha"),
    (StringRole::DialogCancel, "Cancel"),
    (StringRole::ToastDismiss, "Dismiss"),
    (StringRole::ToolbarMore, "More"),
    (StringRole::WindowMinimize, "Minimize or restore"),
    (StringRole::WindowClose, "Close window"),
    (StringRole::FilterName, "Filter"),
    (StringRole::FileBrowserFolderFilter, "Filter folders"),
    (StringRole::ToolWindowMoveLeft, "Move to Left"),
    (StringRole::ToolWindowMoveRight, "Move to Right"),
    (StringRole::ToolWindowMoveTop, "Move to Top"),
    (StringRole::ToolWindowMoveBottom, "Move to Bottom"),
    (StringRole::ToolWindowMoveStartGroup, "Move to start group"),
    (StringRole::ToolWindowMoveEndGroup, "Move to end group"),
    (StringRole::ToolWindowMoveEarlier, "Move earlier"),
    (StringRole::ToolWindowMoveLater, "Move later"),
    (StringRole::ToolWindowOverlay, "Overlay"),
    (StringRole::ToolWindowStripeLeft, "Left tool windows"),
    (StringRole::ToolWindowStripeRight, "Right tool windows"),
    (StringRole::ToolWindowStripeTop, "Top tool windows"),
    (StringRole::ToolWindowStripeBottom, "Bottom tool windows"),
    (StringRole::ToolWindowResizeLeft, "Resize left tool windows"),
    (
        StringRole::ToolWindowResizeRight,
        "Resize right tool windows",
    ),
    (StringRole::ToolWindowResizeTop, "Resize top tool windows"),
    (
        StringRole::ToolWindowResizeBottom,
        "Resize bottom tool windows",
    ),
    (StringRole::RemoveItem, "Remove {item}"),
    (StringRole::BackTo, "Back to {place}"),
    (StringRole::ForwardTo, "Forward to {place}"),
    (
        StringRole::AtLeastCharacters,
        "Character count must be at least {count}",
    ),
    (
        StringRole::AtMostCharacters,
        "Character count must be at most {count}",
    ),
    (StringRole::NotHexColor, "Not a hex color: {reason}"),
    (
        StringRole::ColorAreaValue,
        "saturation {saturation} %, value {value} %",
    ),
    (StringRole::CommandRan, "“{command}” ran."),
    (
        StringRole::CommandNotAvailable,
        "“{command}” is not available right now.",
    ),
    (
        StringRole::CommandBlocked,
        "“{command}” is blocked by an open dialog.",
    ),
    (StringRole::CommandNoRepeat, "“{command}” does not repeat."),
    (StringRole::CommandFailed, "“{command}” failed: {reason}"),
    (StringRole::EntryFailed, "{name} — {reason}"),
    (StringRole::ToolWindowOptions, "{title} options"),
    (StringRole::ToolWindowHide, "Hide {title}"),
    (StringRole::ToolWindowDock, "Dock"),
    (StringRole::ToolWindowFloat, "Float"),
    (StringRole::ToolWindowOpenWindow, "Open in window"),
];

/// Places the English rows at their roles' indices, rejecting a missing or
/// duplicated role and a template that does not match its role.
const fn english_table() -> [&'static str; STRING_ROLE_COUNT] {
    let mut texts: [Option<&str>; STRING_ROLE_COUNT] = [None; STRING_ROLE_COUNT];
    let mut index = 0;
    while index < ENGLISH.len() {
        let (role, text) = ENGLISH[index];
        assert!(
            texts[role as usize].is_none(),
            "string role has two defaults"
        );
        assert!(
            template_matches(text.as_bytes(), role.placeholders()),
            "a default string does not match its role's placeholders"
        );
        texts[role as usize] = Some(text);
        index += 1;
    }
    let mut resolved = [""; STRING_ROLE_COUNT];
    index = 0;
    while index < STRING_ROLE_COUNT {
        resolved[index] = match texts[index] {
            Some(text) => text,
            None => panic!("string role has no default"),
        };
        index += 1;
    }
    resolved
}
const ENGLISH_TABLE: [&str; STRING_ROLE_COUNT] = english_table();

/// Whether `template` uses exactly the `allowed` placeholders, each at least
/// once, with `{{` and `}}` as literal braces and no other brace.
const fn template_matches(template: &[u8], allowed: &[&str]) -> bool {
    let mut seen = [false; 4];
    if allowed.len() > seen.len() {
        return false;
    }
    let mut index = 0;
    while index < template.len() {
        match template[index] {
            b'{' => {
                if index + 1 < template.len() && template[index + 1] == b'{' {
                    index += 2;
                    continue;
                }
                let start = index + 1;
                let mut end = start;
                while end < template.len() && template[end] != b'}' {
                    if template[end] == b'{' {
                        return false;
                    }
                    end += 1;
                }
                if end == template.len() {
                    return false;
                }
                let mut found = false;
                let mut slot = 0;
                while slot < allowed.len() {
                    if bytes_equal(template, start, end, allowed[slot].as_bytes()) {
                        seen[slot] = true;
                        found = true;
                    }
                    slot += 1;
                }
                if !found {
                    return false;
                }
                index = end + 1;
            }
            b'}' => {
                if index + 1 < template.len() && template[index + 1] == b'}' {
                    index += 2;
                    continue;
                }
                return false;
            }
            _ => index += 1,
        }
    }
    let mut slot = 0;
    while slot < allowed.len() {
        if !seen[slot] {
            return false;
        }
        slot += 1;
    }
    true
}

/// Whether `text[start..end]` equals `name`.
const fn bytes_equal(text: &[u8], start: usize, end: usize, name: &[u8]) -> bool {
    if end - start != name.len() {
        return false;
    }
    let mut index = 0;
    while index < name.len() {
        if text[start + index] != name[index] {
            return false;
        }
        index += 1;
    }
    true
}

/// A refused replacement string for [`UiStrings::with`].
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum UiStringError {
    /// The text is empty or only whitespace; every role is shown or spoken, so
    /// it needs words.
    #[error("the text for {0:?} is blank")]
    Blank(StringRole),
    /// The text is longer than [`UiStrings::MAX_LENGTH`] bytes.
    #[error("the text for {0:?} is longer than {max} bytes", max = UiStrings::MAX_LENGTH)]
    TooLong(StringRole),
    /// The text contains a control character, such as a line break.
    #[error("the text for {0:?} contains a control character")]
    ControlCharacter(StringRole),
    /// The text's placeholders differ from the role's: one is missing, unknown
    /// or unbalanced. [`StringRole::placeholders`] lists the expected ones.
    #[error("the text for {0:?} does not use exactly its placeholders")]
    Placeholders(StringRole),
}

/// The complete table of library strings, one validated text per
/// [`StringRole`], with English defaults.
///
/// Build a replacement from [`Self::english`] with [`Self::with`] and install
/// it with [`UiSettings::set_strings`](crate::UiSettings::set_strings); every
/// built-in component that shows one of these strings reads the table in its
/// update, so installing a table invalidates exactly those components and
/// nothing polls. Components that accept a per-instance text, such as
/// `Dialog::cancel_label`, keep the application's text over the table's.
///
/// Clones share the texts and are cheap. The table is UI-thread data like the
/// theme; it is not part of the workspace document.
///
/// ```
/// use pert_ui_runtime::{Runtime, StringRole, UiMessage, UiStrings};
/// let ui = Runtime::new();
/// let swedish = UiStrings::english()
///     .with(StringRole::TextCopy, "Kopiera")?
///     .with(StringRole::BackTo, "Tillbaka till {place}")?;
/// ui.ui_settings().set_strings(swedish.clone())?;
/// assert_eq!(ui.ui_settings().strings()?.get(StringRole::TextCopy), "Kopiera");
/// assert_eq!(
///     swedish.format(UiMessage::BackTo { place: "Färger" }),
///     "Tillbaka till Färger"
/// );
/// // A template must keep its role's placeholders.
/// assert!(UiStrings::english().with(StringRole::BackTo, "Tillbaka").is_err());
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Clone, Debug)]
#[must_use]
pub struct UiStrings {
    texts: Rc<[Cow<'static, str>; STRING_ROLE_COUNT]>,
}

impl PartialEq for UiStrings {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.texts, &other.texts) || self.texts == other.texts
    }
}
impl Eq for UiStrings {}

impl Default for UiStrings {
    fn default() -> Self {
        Self::english()
    }
}

impl UiStrings {
    /// The longest text a role accepts, in bytes.
    pub const MAX_LENGTH: usize = 1024;

    /// The built-in English table.
    pub fn english() -> Self {
        Self {
            texts: Rc::new(ENGLISH_TABLE.map(Cow::Borrowed)),
        }
    }

    /// Returns a table with `role`'s text replaced.
    ///
    /// A role with placeholders takes a template that uses exactly those
    /// placeholders, each at least once and in any order, written in braces;
    /// `{{` and `}}` stand for literal braces.
    ///
    /// # Errors
    /// [`UiStringError::Blank`] for empty or whitespace text,
    /// [`UiStringError::TooLong`] beyond [`Self::MAX_LENGTH`] bytes,
    /// [`UiStringError::ControlCharacter`] for line breaks and other control
    /// characters, and [`UiStringError::Placeholders`] for a template whose
    /// placeholders differ from [`StringRole::placeholders`]. This consumes the
    /// table on success and on error. Clone it before calling when the original
    /// must remain available after a rejected replacement; clones share storage
    /// until a successful replacement creates a new table.
    pub fn with(self, role: StringRole, text: impl Into<String>) -> Result<Self, UiStringError> {
        let text = text.into();
        if text.trim().is_empty() {
            return Err(UiStringError::Blank(role));
        }
        if text.len() > Self::MAX_LENGTH {
            return Err(UiStringError::TooLong(role));
        }
        if text.chars().any(char::is_control) {
            return Err(UiStringError::ControlCharacter(role));
        }
        if !template_matches(text.as_bytes(), role.placeholders()) {
            return Err(UiStringError::Placeholders(role));
        }
        let mut texts = (*self.texts).clone();
        texts[role as usize] = Cow::Owned(text);
        Ok(Self {
            texts: Rc::new(texts),
        })
    }

    /// The text for `role`. For a role with placeholders this is the template
    /// itself; use [`Self::format`] to fill it in.
    #[must_use]
    pub fn get(&self, role: StringRole) -> &str {
        &self.texts[role as usize]
    }

    /// Fills in the template of `message`'s role with its arguments.
    #[must_use]
    pub fn format(&self, message: UiMessage<'_>) -> String {
        let role = message.role();
        let mut out = String::new();
        let template = self.get(role);
        let mut rest = template;
        while let Some(position) = rest.find(['{', '}']) {
            out.push_str(&rest[..position]);
            let tail = &rest[position..];
            if let Some(after) = tail.strip_prefix("{{") {
                out.push('{');
                rest = after;
            } else if let Some(after) = tail.strip_prefix("}}") {
                out.push('}');
                rest = after;
            } else if let Some(close) = tail.find('}') {
                message.write_argument(&tail[1..close], &mut out);
                rest = &tail[close + 1..];
            } else {
                // Validated templates are balanced; keep any remainder as text.
                out.push_str(tail);
                rest = "";
            }
        }
        out.push_str(rest);
        out
    }
}

/// A library message with its typed arguments, filled into the template of
/// its [`StringRole`] by [`UiStrings::format`].
#[derive(Clone, Copy, Debug)]
#[non_exhaustive]
pub enum UiMessage<'a> {
    /// [`StringRole::RemoveItem`].
    RemoveItem {
        /// The chip's text.
        item: &'a str,
    },
    /// [`StringRole::BackTo`].
    BackTo {
        /// The previous entry's label.
        place: &'a str,
    },
    /// [`StringRole::ForwardTo`].
    ForwardTo {
        /// The next entry's label.
        place: &'a str,
    },
    /// [`StringRole::AtLeastCharacters`].
    AtLeastCharacters {
        /// The minimum length in grapheme clusters.
        count: usize,
    },
    /// [`StringRole::AtMostCharacters`].
    AtMostCharacters {
        /// The maximum length in grapheme clusters.
        count: usize,
    },
    /// [`StringRole::NotHexColor`].
    NotHexColor {
        /// Why the text is not a hexadecimal color.
        reason: &'a str,
    },
    /// [`StringRole::ColorAreaValue`].
    ColorAreaValue {
        /// Saturation in whole percent.
        saturation: u8,
        /// Value (brightness) in whole percent.
        value: u8,
    },
    /// [`StringRole::CommandRan`].
    CommandRan {
        /// The command's name.
        command: &'a str,
    },
    /// [`StringRole::CommandNotAvailable`].
    CommandNotAvailable {
        /// The command's name.
        command: &'a str,
    },
    /// [`StringRole::CommandBlocked`].
    CommandBlocked {
        /// The command's name.
        command: &'a str,
    },
    /// [`StringRole::CommandNoRepeat`].
    CommandNoRepeat {
        /// The command's name.
        command: &'a str,
    },
    /// [`StringRole::CommandFailed`].
    CommandFailed {
        /// The command's name.
        command: &'a str,
        /// Why it failed.
        reason: &'a str,
    },
    /// [`StringRole::EntryFailed`].
    EntryFailed {
        /// The entry's display name.
        name: &'a str,
        /// Why it could not be read.
        reason: &'a str,
    },
    /// [`StringRole::ToolWindowOptions`].
    ToolWindowOptions {
        /// The tool window's title.
        title: &'a str,
    },
    /// [`StringRole::ToolWindowHide`].
    ToolWindowHide {
        /// The tool window's title.
        title: &'a str,
    },
}

impl UiMessage<'_> {
    /// The role whose template this message fills in.
    #[must_use]
    pub const fn role(&self) -> StringRole {
        match self {
            Self::RemoveItem { .. } => StringRole::RemoveItem,
            Self::BackTo { .. } => StringRole::BackTo,
            Self::ForwardTo { .. } => StringRole::ForwardTo,
            Self::AtLeastCharacters { .. } => StringRole::AtLeastCharacters,
            Self::AtMostCharacters { .. } => StringRole::AtMostCharacters,
            Self::NotHexColor { .. } => StringRole::NotHexColor,
            Self::ColorAreaValue { .. } => StringRole::ColorAreaValue,
            Self::CommandRan { .. } => StringRole::CommandRan,
            Self::CommandNotAvailable { .. } => StringRole::CommandNotAvailable,
            Self::CommandBlocked { .. } => StringRole::CommandBlocked,
            Self::CommandNoRepeat { .. } => StringRole::CommandNoRepeat,
            Self::CommandFailed { .. } => StringRole::CommandFailed,
            Self::EntryFailed { .. } => StringRole::EntryFailed,
            Self::ToolWindowOptions { .. } => StringRole::ToolWindowOptions,
            Self::ToolWindowHide { .. } => StringRole::ToolWindowHide,
        }
    }

    /// Appends the argument called `name`; validated templates name only this
    /// message's placeholders.
    fn write_argument(&self, name: &str, out: &mut String) {
        match (self, name) {
            (Self::RemoveItem { item }, "item") => out.push_str(item),
            (Self::BackTo { place } | Self::ForwardTo { place }, "place") => out.push_str(place),
            (Self::AtLeastCharacters { count } | Self::AtMostCharacters { count }, "count") => {
                let _ = write!(out, "{count}");
            }
            (Self::NotHexColor { reason }, "reason")
            | (Self::CommandFailed { reason, .. }, "reason")
            | (Self::EntryFailed { reason, .. }, "reason") => out.push_str(reason),
            (Self::ColorAreaValue { saturation, .. }, "saturation") => {
                let _ = write!(out, "{saturation}");
            }
            (Self::ColorAreaValue { value, .. }, "value") => {
                let _ = write!(out, "{value}");
            }
            (
                Self::CommandRan { command }
                | Self::CommandNotAvailable { command }
                | Self::CommandBlocked { command }
                | Self::CommandNoRepeat { command }
                | Self::CommandFailed { command, .. },
                "command",
            ) => out.push_str(command),
            (Self::EntryFailed { name: entry, .. }, "name") => out.push_str(entry),
            (Self::ToolWindowOptions { title } | Self::ToolWindowHide { title }, "title") => {
                out.push_str(title);
            }
            // Unreachable for validated templates; an unknown name writes nothing.
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_message_fills_every_placeholder_of_its_role() {
        let strings = UiStrings::english();
        let messages = [
            UiMessage::RemoveItem { item: "x" },
            UiMessage::BackTo { place: "x" },
            UiMessage::ForwardTo { place: "x" },
            UiMessage::AtLeastCharacters { count: 7 },
            UiMessage::AtMostCharacters { count: 7 },
            UiMessage::NotHexColor { reason: "x" },
            UiMessage::ColorAreaValue {
                saturation: 7,
                value: 7,
            },
            UiMessage::CommandRan { command: "x" },
            UiMessage::CommandNotAvailable { command: "x" },
            UiMessage::CommandBlocked { command: "x" },
            UiMessage::CommandNoRepeat { command: "x" },
            UiMessage::CommandFailed {
                command: "x",
                reason: "x",
            },
            UiMessage::EntryFailed {
                name: "x",
                reason: "x",
            },
            UiMessage::ToolWindowOptions { title: "x" },
            UiMessage::ToolWindowHide { title: "x" },
        ];
        for message in messages {
            let text = strings.format(message);
            assert!(!text.contains('{') && !text.contains('}'), "{text}");
            for placeholder in message.role().placeholders() {
                let _ = placeholder;
                assert!(text.contains('x') || text.contains('7'), "{text}");
            }
        }
        // Every role with placeholders has a message.
        let with_placeholders = StringRole::ALL
            .iter()
            .filter(|role| !role.placeholders().is_empty())
            .count();
        assert_eq!(with_placeholders, messages.len());
    }

    #[test]
    fn templates_reject_unknown_missing_and_unbalanced_placeholders() {
        assert!(template_matches(b"Back to {place}", &["place"]));
        assert!(template_matches(b"{place} {{literal}}", &["place"]));
        assert!(!template_matches(b"Back to", &["place"]));
        assert!(!template_matches(b"Back to {where}", &["place"]));
        assert!(!template_matches(b"Back to {place", &["place"]));
        assert!(!template_matches(b"Back } to {place}", &["place"]));
        assert!(!template_matches(b"{a{b}}", &["a"]));
        assert!(template_matches(b"Copy", &[]));
        assert!(!template_matches(b"Copy {x}", &[]));
    }
}
