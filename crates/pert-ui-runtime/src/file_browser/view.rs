//! The browser's components: the composed browser, its path bar and its rows.
use super::controller::Hooks;
use super::{DirectoryEntry, EntryKind, FileBrowserController, LinkTarget, ListingError};
use crate::{
    ArrangeContext, Breadcrumb, BreadcrumbSegment, ColorRole, Column, Component, ComponentDetails,
    Constraints, Element, EventContext, Icon, InputOutcome, InspectValue, Invalidation, KeyCode,
    KeyEvent, KeyTransition, KeyboardState, Label, MeasureContext, PointerButton, PointerEvent,
    PointerKind, Rect, RegionLayout, RuntimeError, SearchField, SemanticRole, SemanticValue,
    Semantics, Size, SizeLimits, SplitAxis, SplitPane, TextStyle, Tree, UpdateContext, VirtualList,
};
use std::path::{Path, PathBuf};
use std::rc::Rc;

/// Directory navigation and selection over a host-supplied directory source:
/// a path bar, a filter, a folder tree and the current folder's entries.
///
/// Everything the browser shows comes from its [`FileBrowserController`]; the
/// browser performs no filesystem I/O and starts no thread. Its parts, from
/// top to bottom and left to right, with their child keys:
///
/// - `path`: a [`Breadcrumb`] of the current folder, from the root that holds
///   it, or from the filesystem root for a folder outside every root.
///   Activating a segment opens that folder; middle segments collapse into a
///   menu when the bar is narrow.
/// - `filter`: a [`SearchField`] bound to [`FileBrowserController::filter`].
/// - `folders`: a [`Tree`] of the roots and their subfolders, listed lazily: a
///   folder expanded before its listing arrives shows a loading row, and one
///   whose listing failed an error row. Clicking a folder, or Enter on the
///   active one, opens it in the entry list.
/// - `entries`: a [`VirtualList`] of the current folder's entries, directories
///   first, filtered by the field. Enter or a double click on an entry (two
///   primary presses within 500 ms and four logical pixels, as
///   [`ClickCounter`](crate::ClickCounter) counts them; four quick presses
///   are two double clicks) opens a
///   directory or a link to one in the browser and hands anything else to
///   [`FileBrowserController::on_open`]. Selection follows the controller's
///   [`ListSelection`](crate::ListSelection). While the folder loads, is
///   empty, failed or matches nothing, the list shows one explanatory row.
///
/// With [`Self::folder_filter`] the tree gets a filter field of its own
/// above it: a [`CollectionFilter`](crate::CollectionFilter) under the key
/// `folders`, holding the tree under the key `tree`, that edits
/// [`FileBrowserController::folder_filter`]. It searches only folders already
/// listed and never asks the host for a listing.
///
/// The tree and the list sit in a [`SplitPane`] (key `panes`), which the user
/// can resize and the workspace remembers. Backspace in the tree or the list
/// opens the parent folder; the filter field keeps Backspace for editing. Rows are keyed by path through the controller, so the
/// tree keeps expansion and selection, and the list its selection, when a
/// listing changes. Inspection reports the browser as a
/// [`SemanticRole::Group`] named "File browser" whose value is the current
/// folder.
///
/// The browser needs a bounded height, as any virtualized collection does;
/// give it a fixed or region allocation rather than an unbounded scroll area.
/// See [`FileBrowserController`] for a complete example.
///
/// Describing a mounted browser again keeps its controller and row height:
/// its folders, selection and requests belong to the controller. Give the
/// browser a new key to replace it. Its generated parts mount even while hidden,
/// so content search and workspace capture include them before first reveal.
/// Breadcrumbs, list rows and tree rows quote or escape names consisting only
/// of whitespace,
/// making legal filesystem names visible without changing path identity or
/// navigation. Other names retain their ordinary presentation.
/// Later string-table changes redescribe them from the
/// retained controller. Incoming browser descriptions supply no replacement
/// child bindings; all parts continue to use the mounted controller.
#[must_use]
pub struct FileBrowser {
    controller: FileBrowserController,
    row_height: f32,
    folder_filter: Option<crate::Permission>,
    current: String,
    /// The string table the parts were last described with.
    strings: crate::UiStrings,
    /// A construction failure is propagated by description admission.
    parts_error: Option<RuntimeError>,
}
impl FileBrowser {
    /// Creates a browser over `controller` whose tree and list rows are
    /// `row_height` logical pixels tall.
    ///
    /// # Errors
    /// Returns a [`GeometryError`](crate::GeometryError) for a row height that
    /// is not finite and positive.
    pub fn new(
        controller: &FileBrowserController,
        row_height: f32,
    ) -> Result<Self, crate::GeometryError> {
        // Validate row geometry before Element extracts initial parts.
        let _ = Tree::new(controller.tree(), row_height)?;
        Ok(Self {
            controller: controller.clone(),
            row_height,
            folder_filter: None,
            current: String::new(),
            strings: crate::UiStrings::english(),
            parts_error: None,
        })
    }
    /// Offers a filter field above the folder tree, as a constant or a
    /// binding; not offered by default. The field edits
    /// [`FileBrowserController::folder_filter`] and shows the folders whose
    /// names match with the folders above them, searching only folders already
    /// listed. Fixed when the browser mounts.
    /// Repeated calls combine restrictions; a fixed denial cannot be lifted
    /// by a later permission on the same description.
    pub fn folder_filter(mut self, offered: impl Into<crate::Permission>) -> Self {
        self.folder_filter = Some(self.folder_filter.take().unwrap_or_default().and(offered));
        self
    }
    /// The browser's layout, with the parts named from `strings`.
    fn parts(&self, strings: &crate::UiStrings) -> Result<Element, crate::GeometryError> {
        use crate::StringRole;
        let (controller, row_height) = (&self.controller, self.row_height);
        let hooks = controller.hooks();
        let open = hooks.clone();
        let tree =
            Tree::new(controller.tree(), row_height)?.on_activate(move |_, key| open.open_key(key));
        let folders = match &self.folder_filter {
            Some(offered) => Element::new(
                "folders",
                crate::CollectionFilter::with_query(
                    controller.folder_filter(),
                    controller.tree().clone(),
                    Element::new("tree", tree)
                        .with_label(strings.get(StringRole::FileBrowserFolders)),
                )
                .name(strings.get(StringRole::FileBrowserFolderFilter))
                .offered(offered.clone()),
            ),
            None => Element::new("folders", tree)
                .with_label(strings.get(StringRole::FileBrowserFolders)),
        };
        let activate = hooks.clone();
        let list = VirtualList::new(controller.entries(), row_height)?
            .selection(controller.selection())
            .on_activate(move |cx, key| activate.activate(cx, key));
        let bar = Column::new([
            Element::new("path", PathBar::new(controller)),
            Element::new(
                "filter",
                SearchField::new(controller.filter())
                    .placeholder(strings.get(StringRole::FileBrowserFilter)),
            )
            .with_label(strings.get(StringRole::FileBrowserFilter)),
        ]);
        let panes = SplitPane::new(
            SplitAxis::LeftRight,
            folders,
            Element::new("entries", list).with_label(strings.get(StringRole::FileBrowserEntries)),
        );
        let layout = RegionLayout::new()
            .fixed()
            .top(Element::new("bar", bar))
            .center(Element::new("panes", panes));
        Ok(Element::new("layout", layout))
    }
}
impl Component for FileBrowser {
    fn retain_initial_children(&self) -> bool {
        true
    }
    fn take_children(&mut self) -> Vec<Element> {
        match self.controller.initial_strings() {
            Ok(strings) => self.strings = strings,
            Err(error) => {
                self.parts_error = Some(error);
                return Vec::new();
            }
        }
        match self.controller.current().get() {
            Ok(current) => self.current = current.to_string_lossy().into_owned(),
            Err(error) => {
                self.parts_error = Some(error);
                return Vec::new();
            }
        }
        match self.parts(&self.strings) {
            Ok(parts) => vec![parts],
            Err(error) => {
                self.parts_error = Some(error.into());
                Vec::new()
            }
        }
    }
    fn check_description(&self) -> Result<(), RuntimeError> {
        match &self.parts_error {
            Some(error) => Err(error.clone()),
            None => Ok(()),
        }
    }
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        let strings = cx.strings()?;
        if strings != self.strings {
            cx.request_children(vec![self.parts(&strings)?]);
            self.strings = strings;
        }
        // Expanding a folder in the tree records it as wanted while the tree
        // flattens; this update follows and asks the host for it.
        cx.read(&self.controller.tree().expanded())?;
        self.current = cx
            .read(&self.controller.current())?
            .to_string_lossy()
            .into_owned();
        self.controller.dispatch()
    }
    fn measure_limits(&self, cx: &MeasureContext<'_>) -> Result<SizeLimits, RuntimeError> {
        if cx.is_empty() {
            Ok(SizeLimits::default())
        } else {
            cx.limits(0)
        }
    }
    fn keyboard_event(
        &mut self,
        _: &mut EventContext,
        event: KeyEvent,
        keyboard: &KeyboardState,
    ) -> Result<InputOutcome, RuntimeError> {
        let control = keyboard.control_key(event);
        let modifiers = keyboard.modifiers();
        // Arrow keys, Alt+arrows included, belong to the tree and the list, so
        // the parent folder is Backspace that no part of the browser used.
        if control == KeyCode::Backspace
            && event.transition == KeyTransition::Pressed
            && !modifiers.alt
            && !modifiers.control
            && !modifiers.shift
            && !modifiers.super_key
        {
            self.controller.navigate_up()?;
            return Ok(InputOutcome::Handled);
        }
        Ok(InputOutcome::Ignored)
    }
    fn inspect(&self) -> ComponentDetails {
        ComponentDetails {
            semantics: Some(
                Semantics::new(SemanticRole::Group)
                    .name(self.strings.get(crate::StringRole::FileBrowserName))
                    .value(SemanticValue::Text(self.current.clone())),
            ),
            ..ComponentDetails::default()
        }
    }
}

/// The breadcrumb of the current folder, described again when it changes.
struct PathBar {
    controller: FileBrowserController,
    shown: Option<PathBuf>,
}
impl PathBar {
    fn new(controller: &FileBrowserController) -> Self {
        Self {
            controller: controller.clone(),
            shown: None,
        }
    }
    /// The folders from the root holding `current`, or from the filesystem
    /// root, down to `current`.
    fn segments(&self, current: &Path) -> Vec<BreadcrumbSegment> {
        let root = self
            .controller
            .roots()
            .iter()
            .filter(|root| current.starts_with(root))
            .max_by_key(|root| root.components().count());
        let mut folders: Vec<&Path> = current
            .ancestors()
            .take_while(|ancestor| {
                !ancestor.as_os_str().is_empty()
                    && root.is_none_or(|root| ancestor.starts_with(root))
            })
            .collect();
        folders.reverse();
        folders
            .into_iter()
            .map(|folder| {
                let name = super::display_name(folder);
                let label = visible_name(&name);
                BreadcrumbSegment::new(self.controller.key_of(folder), label)
            })
            .collect()
    }
}
impl Component for PathBar {
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        let current = cx.read(&self.controller.current())?;
        if self.shown.as_ref() != Some(&current) {
            let hooks = self.controller.hooks();
            let crumbs =
                Breadcrumb::new(self.segments(&current), move |_, key| hooks.open_key(key))?;
            cx.request_children(vec![Element::new("crumbs", crumbs)]);
            cx.request_layout();
            self.shown = Some(current);
        }
        Ok(())
    }
    fn measure_limits(&self, cx: &MeasureContext<'_>) -> Result<SizeLimits, RuntimeError> {
        if cx.is_empty() {
            Ok(SizeLimits::default())
        } else {
            cx.limits(0)
        }
    }
}

/// A row's text, vertically centered in the row.
fn arrange_line(cx: &mut ArrangeContext<'_>, size: Size) -> Result<(), RuntimeError> {
    if cx.is_empty() {
        return Ok(());
    }
    let measured = cx.measure(0, Constraints::new(Some(size.width()), None)?)?;
    let y = crate::layout::centered(size.height(), measured.height());
    cx.place(
        0,
        Rect::new(0.0, y, size.width(), measured.height().min(size.height()))?,
    )
}
fn line_limits(cx: &MeasureContext<'_>) -> Result<SizeLimits, RuntimeError> {
    if cx.is_empty() {
        Ok(SizeLimits::default())
    } else {
        cx.limits(0)
    }
}

/// A placeholder row's text: bound to the string table, or the host's
/// listing error message.
#[derive(Clone)]
pub(super) enum PlaceholderText {
    Bound(crate::ReadSignal<String>),
    Fixed(String),
}

/// A loading, empty or error row, which has no path.
pub(super) fn placeholder(
    key: &str,
    text: PlaceholderText,
    error: Option<&ListingError>,
) -> Element {
    let label = match text {
        PlaceholderText::Bound(text) => Label::new(text),
        PlaceholderText::Fixed(text) => Label::new(text),
    }
    .style(TextStyle::Caption);
    let label = match error {
        Some(_) => label.leading_icon(Icon::Error).color_role(ColorRole::Error),
        None => label.color_role(ColorRole::Muted),
    };
    Element::new(key, label)
}

// Presentation alone escapes unnamed-looking paths; filtering and identities
// continue to use the original name and canonical path.
fn visible_name(name: &str) -> String {
    if name.trim().is_empty() {
        format!("{name:?}")
    } else {
        name.to_owned()
    }
}

/// A folder in the tree; a primary press opens it in the entry list.
pub(super) struct FolderRow {
    path: PathBuf,
    hooks: Rc<Hooks>,
    children: Vec<Element>,
}
impl FolderRow {
    pub(super) fn new(name: String, path: PathBuf, hooks: Rc<Hooks>) -> Self {
        Self {
            path,
            hooks,
            children: vec![Element::new(
                "name",
                Label::new(visible_name(&name)).leading_icon(Icon::Folder),
            )],
        }
    }
}
impl Component for FolderRow {
    fn reconcile(&mut self, next: Self) -> Option<Invalidation> {
        self.path = next.path;
        self.hooks = next.hooks;
        None
    }
    fn take_children(&mut self) -> Vec<Element> {
        std::mem::take(&mut self.children)
    }
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
    fn measure_limits(&self, cx: &MeasureContext<'_>) -> Result<SizeLimits, RuntimeError> {
        line_limits(cx)
    }
    fn arrange(&self, cx: &mut ArrangeContext<'_>, size: Size) -> Result<(), RuntimeError> {
        arrange_line(cx, size)
    }
    fn pointer_input(
        &mut self,
        _: &mut EventContext,
        event: PointerEvent,
    ) -> Result<InputOutcome, RuntimeError> {
        if event.kind() == PointerKind::Press(PointerButton::Primary) && !event.is_outside() {
            self.hooks.open_folder(&self.path)?;
        }
        // The tree still selects the row and takes focus.
        Ok(InputOutcome::Ignored)
    }
    fn inspect(&self) -> ComponentDetails {
        let mut details = ComponentDetails::default();
        details.properties.insert(
            "path".into(),
            InspectValue::Text(self.path.to_string_lossy().into_owned()),
        );
        details
    }
}

/// An entry of the current folder; a double click activates it.
pub(super) struct EntryRow {
    key: String,
    path: PathBuf,
    kind: EntryKind,
    /// The listing entry the row shows, to notice when a refresh replaced it.
    entry: DirectoryEntry,
    hooks: Rc<Hooks>,
    children: Vec<Element>,
    /// The primary presses since the last double click.
    clicks: crate::ClickCounter,
    /// The failure text shown for an unreadable entry.
    failed: Option<String>,
}
impl EntryRow {
    pub(super) fn new(key: String, entry: &DirectoryEntry, hooks: Rc<Hooks>) -> Self {
        let name = visible_name(&entry.name().to_string_lossy());
        let failed = entry.error().map(|error| {
            crate::UiStrings::english().format(crate::UiMessage::EntryFailed {
                name: &name,
                reason: error.message(),
            })
        });
        let label = match (failed.clone(), entry.kind()) {
            (Some(text), _) => failure_label(text),
            (None, EntryKind::Directory) => Label::new(name).leading_icon(Icon::Folder),
            (None, EntryKind::Symlink(target)) => Label::new(name)
                .leading_icon(match target {
                    LinkTarget::Directory => Icon::Folder,
                    LinkTarget::Broken => Icon::Error,
                    _ => Icon::File,
                })
                .trailing_icon(Icon::Link),
            (None, _) => Label::new(name).leading_icon(Icon::File),
        };
        Self {
            key,
            path: entry.path().to_owned(),
            kind: entry.kind(),
            entry: entry.clone(),
            hooks,
            children: vec![Element::new("name", label)],
            clicks: crate::ClickCounter::new(),
            failed,
        }
    }
}

/// The label of an entry that could not be read.
fn failure_label(text: String) -> Label {
    Label::new(text)
        .leading_icon(Icon::Error)
        .color_role(ColorRole::Error)
}
impl Component for EntryRow {
    fn reconcile(&mut self, next: Self) -> Option<Invalidation> {
        // The name label is a described child, so the runtime reconciles it
        // with the new entry's text and icons. A press armed on the entry a
        // refresh replaced must not complete a double click on the new one.
        if self.entry != next.entry {
            self.clicks.reset();
        }
        self.key = next.key;
        self.path = next.path;
        self.kind = next.kind;
        self.entry = next.entry;
        self.hooks = next.hooks;
        self.failed = next.failed;
        // The update writes the failure text in the installed table again.
        Some(Invalidation::Paint)
    }
    fn take_children(&mut self) -> Vec<Element> {
        std::mem::take(&mut self.children)
    }
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        if let Some(error) = self.entry.error() {
            let name = visible_name(&self.entry.name().to_string_lossy());
            let text = cx.strings()?.format(crate::UiMessage::EntryFailed {
                name: &name,
                reason: error.message(),
            });
            if self.failed.as_ref() != Some(&text) {
                cx.request_children(vec![Element::new("name", failure_label(text.clone()))]);
                self.failed = Some(text);
            }
        }
        Ok(())
    }
    fn measure_limits(&self, cx: &MeasureContext<'_>) -> Result<SizeLimits, RuntimeError> {
        line_limits(cx)
    }
    fn arrange(&self, cx: &mut ArrangeContext<'_>, size: Size) -> Result<(), RuntimeError> {
        arrange_line(cx, size)
    }
    fn pointer_input(
        &mut self,
        cx: &mut EventContext,
        event: PointerEvent,
    ) -> Result<InputOutcome, RuntimeError> {
        if event.kind() != PointerKind::Press(PointerButton::Primary) || event.is_outside() {
            return Ok(InputOutcome::Ignored);
        }
        if self.clicks.press_event(cx, event) > Some(1) {
            // Acted on: four quick presses open twice.
            self.clicks.reset();
            self.hooks.activate(cx, &self.key)?;
            return Ok(InputOutcome::Handled);
        }
        // The list still selects the row and takes focus.
        Ok(InputOutcome::Ignored)
    }
    fn input_reset(&mut self) {
        self.clicks.reset();
    }
    fn inspect(&self) -> ComponentDetails {
        let mut details = ComponentDetails::default();
        details.properties.insert(
            "path".into(),
            InspectValue::Text(self.path.to_string_lossy().into_owned()),
        );
        details.properties.insert(
            "kind".into(),
            InspectValue::Text(
                match self.kind {
                    EntryKind::Directory => "directory",
                    EntryKind::File => "file",
                    EntryKind::Symlink(_) => "symlink",
                    _ => "other",
                }
                .into(),
            ),
        );
        details
    }
}
