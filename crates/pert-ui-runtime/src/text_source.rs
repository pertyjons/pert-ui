//! One shape for the text a control shows: fixed or bound, plain or rich.

use crate::{IconSource, ReadSignal, RichText, RuntimeError, Signal, StringRole, UpdateContext};

/// The text a control shows: fixed plain or rich content, a
/// `ReadSignal<String>` or `ReadSignal<RichText>` the application owns, or a
/// [`StringRole`] of the installed [`UiStrings`](crate::UiStrings).
///
/// [`Label`](crate::Label), [`Button`](crate::Button), [`Link`](crate::Link),
/// [`Chip`](crate::Chip), [`Tooltip`](crate::Tooltip),
/// [`MenuItem`](crate::MenuItem), [`Menu`](crate::Menu),
/// [`Checkbox`](crate::Checkbox), [`Toggle`](crate::Toggle) and
/// [`InlineToggle`](crate::InlineToggle) take `impl Into<TextSource>`, as do
/// titles, placeholders and the builders that replace a library text, such as
/// a toast host's dismiss label; so `"Gain"`, a `String`, a [`RichText`], a
/// read signal of either and a [`StringRole`] can be passed directly. A
/// binding to text is how a caption follows a translation or a changing state
/// without describing the control again; a role follows the installed string
/// table ([`UiSettings::set_strings`](crate::UiSettings::set_strings)), which is
/// how a library default text is given back explicitly.
///
/// The shared contract:
///
/// - A component reads a binding or a role in its update with [`Self::read`],
///   which subscribes it to the binding or the string table, so a change
///   measures and paints it again without polling. Before the first update
///   such a caption is empty.
/// - Describing the component again with the same binding or role keeps the
///   text it has read; another source replaces it. Fixed text applies when the
///   description is adopted.
/// - Explicit icon slots, such as a `leading_icon` builder, are independent of
///   the caption: rich content from the source replaces only the caption, and
///   icons embedded in that rich content are part of it.
/// - A binding from another runtime fails the component's update with
///   [`RuntimeError::ForeignRuntime`].
///
/// ```
/// use pert_ui_runtime::{Element, Label, Runtime, TextSource};
/// use std::time::Duration;
///
/// let mut ui = Runtime::new();
/// let title = ui.signal(String::from("Mixer"));
/// ui.mount_tree(Element::new("title", Label::new(title.read_only())))?;
/// ui.flush(Duration::ZERO)?;
/// title.set("Mischpult".into())?;
/// ui.flush(Duration::ZERO)?;
/// let label = ui.inspect().components.into_iter().find(|c| c.key == "title").unwrap();
/// assert_eq!(label.details.semantics.unwrap().name.as_deref(), Some("Mischpult"));
///
/// // Fixed text, plain or rich, converts the same way, and so does a role
/// // of the string table.
/// let fixed = TextSource::from("Mixer");
/// assert_eq!(fixed.fixed().map(|text| text.plain_text()).as_deref(), Some("Mixer"));
/// let library = TextSource::from(pert_ui_runtime::StringRole::SearchPlaceholder);
/// assert!(library.fixed().is_none());
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Clone, Debug)]
#[must_use]
pub struct TextSource(Source);

#[derive(Clone, Debug)]
enum Source {
    Fixed(RichText),
    Plain(ReadSignal<String>),
    Rich(ReadSignal<RichText>),
    /// A role of the installed string table, with the library's initial
    /// text for a default whose current text is known before the first read.
    Role(StringRole, Option<RichText>),
}

impl Default for TextSource {
    /// Fixed empty text.
    fn default() -> Self {
        Self(Source::Fixed(RichText::default()))
    }
}

impl TextSource {
    /// The fixed content, or `None` for a binding or a role.
    #[must_use]
    pub fn fixed(&self) -> Option<&RichText> {
        match &self.0 {
            Source::Fixed(content) => Some(content),
            Source::Plain(_) | Source::Rich(_) | Source::Role(..) => None,
        }
    }
    /// The string-table role, or `None` for fixed text or a binding.
    #[must_use]
    pub fn role(&self) -> Option<StringRole> {
        match &self.0 {
            Source::Role(role, _) => Some(*role),
            _ => None,
        }
    }
    /// The library's default text for `role`, showing `initial` (the role's
    /// current text) until the first read.
    pub(crate) fn library(role: StringRole, initial: String) -> Self {
        Self(Source::Role(role, Some(RichText::from(initial))))
    }
    /// The content shown before the first read: fixed content, the library's
    /// initial text, or nothing for an application binding.
    fn initial(&self) -> RichText {
        match &self.0 {
            Source::Fixed(content) | Source::Role(_, Some(content)) => content.clone(),
            Source::Plain(_) | Source::Rich(_) | Source::Role(_, None) => RichText::default(),
        }
    }
    /// Whether the text is read in an update: a binding or a string-table
    /// role, as opposed to fixed text.
    #[must_use]
    pub fn is_bound(&self) -> bool {
        self.fixed().is_none()
    }
    /// Whether both sources show the same text without reading anything:
    /// equal fixed content, the same role, or the same binding (see
    /// [`ReadSignal::same_source`]). A component uses it in its reconcile to
    /// keep text it has already read.
    #[must_use]
    pub fn same_as(&self, other: &Self) -> bool {
        match (&self.0, &other.0) {
            (Source::Fixed(mine), Source::Fixed(theirs)) => mine == theirs,
            (Source::Plain(mine), Source::Plain(theirs)) => mine.same_source(theirs),
            (Source::Role(mine, _), Source::Role(theirs, _)) => mine == theirs,
            (Source::Rich(mine), Source::Rich(theirs)) => mine.same_source(theirs),
            _ => false,
        }
    }
    /// The current content, read in a component's update. A binding is read
    /// with [`UpdateContext::read`], so its changes measure the component
    /// again; fixed content is returned as it is.
    ///
    /// # Errors
    /// Returns the runtime error of reading the binding, such as
    /// [`RuntimeError::ForeignRuntime`] for a signal from another runtime or
    /// [`RuntimeError::Closed`] after the runtime closed.
    pub fn read(&self, cx: &mut UpdateContext) -> Result<RichText, RuntimeError> {
        match &self.0 {
            Source::Fixed(content) => Ok(content.clone()),
            Source::Plain(source) => Ok(RichText::from(cx.read(source)?)),
            Source::Rich(source) => cx.read(source),
            Source::Role(role, _) => Ok(RichText::from(cx.strings()?.get(*role))),
        }
    }
    /// Reads text whose changes leave the component's geometry unchanged,
    /// such as an accessible name that is not drawn, subscribing with
    /// [`UpdateContext::read_paint`]. Text that is measured must use
    /// [`Self::read`] or request layout when it changes.
    ///
    /// # Errors
    /// The read failures of [`Self::read`].
    pub fn read_paint(&self, cx: &mut UpdateContext) -> Result<RichText, RuntimeError> {
        match &self.0 {
            Source::Fixed(content) => Ok(content.clone()),
            Source::Plain(source) => Ok(RichText::from(cx.read_paint(source)?)),
            Source::Rich(source) => cx.read_paint(source),
            // The string table subscribes for layout; a role's text is short
            // and rarely changes.
            Source::Role(role, _) => Ok(RichText::from(cx.strings()?.get(*role))),
        }
    }
}

impl From<&str> for TextSource {
    fn from(text: &str) -> Self {
        Self(Source::Fixed(RichText::from(text)))
    }
}
impl From<String> for TextSource {
    fn from(text: String) -> Self {
        Self(Source::Fixed(RichText::from(text)))
    }
}
impl From<&String> for TextSource {
    fn from(text: &String) -> Self {
        Self::from(text.as_str())
    }
}
impl From<StringRole> for TextSource {
    /// Follows the role's text in the installed string table.
    fn from(role: StringRole) -> Self {
        Self(Source::Role(role, None))
    }
}
impl From<RichText> for TextSource {
    fn from(content: RichText) -> Self {
        Self(Source::Fixed(content))
    }
}
impl From<ReadSignal<String>> for TextSource {
    fn from(source: ReadSignal<String>) -> Self {
        Self(Source::Plain(source))
    }
}
impl From<ReadSignal<RichText>> for TextSource {
    fn from(source: ReadSignal<RichText>) -> Self {
        Self(Source::Rich(source))
    }
}
impl From<&Signal<String>> for TextSource {
    /// Follows the signal read-only; the control never writes it.
    fn from(source: &Signal<String>) -> Self {
        Self::from(source.read_only())
    }
}
impl From<&Signal<RichText>> for TextSource {
    /// Follows the signal read-only; the control never writes it.
    fn from(source: &Signal<RichText>) -> Self {
        Self::from(source.read_only())
    }
}

/// A component's caption: its source, the content last read from it and the
/// explicit icon slots, applying the [`TextSource`] contract in one place.
#[derive(Debug)]
pub(crate) struct Caption {
    source: TextSource,
    content: RichText,
    text: String,
    pub(crate) leading: Option<IconSource>,
    pub(crate) trailing: Option<IconSource>,
}

impl Caption {
    pub(crate) fn new(source: impl Into<TextSource>) -> Self {
        let source = source.into();
        let content = source.initial();
        Self {
            text: content.plain_text(),
            content,
            source,
            leading: None,
            trailing: None,
        }
    }
    pub(crate) fn source(&self) -> &TextSource {
        &self.source
    }
    /// The caption's plain text, for names and inspection.
    pub(crate) fn text(&self) -> &str {
        &self.text
    }
    /// The caption with its icon slots.
    pub(crate) fn display(&self) -> std::borrow::Cow<'_, RichText> {
        crate::controls::adorned_content(
            &self.content,
            self.leading.as_ref(),
            self.trailing.as_ref(),
        )
    }
    /// Replaces the source, keeping the read text for the same binding.
    /// Icon slots are left alone. Returns whether the shown content changed.
    pub(crate) fn set_source(&mut self, source: TextSource) -> bool {
        if self.source.same_as(&source) {
            return false;
        }
        self.source = source;
        // A new binding is empty until the coming update reads it.
        self.set_content(self.source.initial());
        true
    }
    /// Adopts a new description: its source and its icon slots. Returns
    /// whether the presentation changed.
    pub(crate) fn reconcile(&mut self, next: Self) -> bool {
        let slots = self.leading != next.leading || self.trailing != next.trailing;
        self.leading = next.leading;
        self.trailing = next.trailing;
        self.set_source(next.source) | slots
    }
    /// Reads a bound source for measured text. Returns whether the content
    /// changed.
    pub(crate) fn update(&mut self, cx: &mut UpdateContext) -> Result<bool, RuntimeError> {
        if !self.source.is_bound() {
            return Ok(false);
        }
        let content = self.source.read(cx)?;
        Ok(self.set_content(content))
    }
    /// Reads a bound source for measured text that must not be blank, such as
    /// a title. A blank read keeps the last valid content and returns
    /// `blank()`, so the component keeps its presentation and the update
    /// reports a typed failure. Fixed text is checked where it is described.
    /// Returns whether the content changed.
    pub(crate) fn update_nonblank(
        &mut self,
        cx: &mut UpdateContext,
        blank: impl FnOnce() -> RuntimeError,
    ) -> Result<bool, RuntimeError> {
        if !self.source.is_bound() {
            return Ok(false);
        }
        let content = self.source.read(cx)?;
        if content.plain_text().trim().is_empty() {
            return Err(blank());
        }
        Ok(self.set_content(content))
    }
    /// Reads a bound source for text that is not measured, such as a name.
    /// Returns whether the content changed.
    pub(crate) fn update_paint(&mut self, cx: &mut UpdateContext) -> Result<bool, RuntimeError> {
        if !self.source.is_bound() {
            return Ok(false);
        }
        let content = self.source.read_paint(cx)?;
        Ok(self.set_content(content))
    }
    /// Shows `content` directly, for a caption that follows another value
    /// such as a parameter's name. Returns whether it changed.
    pub(crate) fn set_content(&mut self, content: RichText) -> bool {
        if self.content == content {
            return false;
        }
        self.text = content.plain_text();
        self.content = content;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Runtime;

    #[test]
    fn only_library_defaults_show_text_before_the_first_read() {
        let ui = Runtime::new();
        let table = ui.signal("Close".to_owned());
        let role = StringRole::WindowClose;
        let library = Caption::new(TextSource::library(role, "Close".into()));
        assert_eq!(library.text(), "Close");
        assert_eq!(Caption::new(table.read_only()).text(), "");
        assert_eq!(
            Caption::new(role).text(),
            "",
            "an application role is read first"
        );
        let mut fixed = Caption::new("Old");
        assert!(fixed.set_source(TextSource::library(role, "Close".into())));
        assert_eq!(fixed.text(), "Close");
        assert!(!fixed.set_source(role.into()), "the same role");
        assert_eq!(fixed.text(), "Close");
        assert!(fixed.set_source(table.read_only().into()));
        assert_eq!(fixed.text(), "", "a binding never shows old text");
    }
}
