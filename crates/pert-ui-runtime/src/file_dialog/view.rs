//! The mounted file dialog: its owner and the content it shows.
use super::{
    FileDialogController, FileDialogError, FileDialogMode, FileDialogOutcome, FileDialogStatus,
    FilePlace,
};
use crate::layout::rect;
use crate::{
    ArrangeContext, Button, Column, Component, Constraints, Dialog, Dynamic, Element, EventContext,
    FileBrowser, Label, MeasureContext, MetricRole, Notifier, RuntimeError, ScrollArea, ScrollAxes,
    Select, Size, SizeLimits, TextField, UpdateContext,
};
use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

/// What the content saw change since its last update.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct Change {
    /// The selected file type changed.
    pub(super) filter: bool,
    /// The entry list's active row changed.
    pub(super) active: bool,
    /// The name field's text changed.
    pub(super) name: bool,
    /// The folder shown changed.
    pub(super) current: bool,
    /// Creating folders was forbidden while the user was creating one.
    pub(super) revoked: bool,
}

/// The modal scope of a mounted file dialog.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Scope {
    Window,
    View,
}

/// A file dialog presented inside its owner's window.
///
/// `anchor` is ordinary content, typically the view the dialog belongs to,
/// and keeps its place in layout and its key path, as for a [`Dialog`]. The
/// dialog's content is closed until [`FileDialogController::open`] starts a
/// session, and appears centered over the window ([`Self::in_window`]) or
/// over the anchor's region ([`Self::in_view`]). It shows the controller's
/// [`FileBrowser`], to its left the application's places and the memory's
/// recent folders when there are any, in a save dialog a name field, a
/// file-type selector when
/// the request offers types and does not ask for a folder, a line for refusals, and cancel and confirm
/// buttons. Escape cancels; Enter and double click on an entry, and Enter in
/// the name field, confirm; Backspace inside the browser opens the parent
/// folder. Before an existing file is replaced, the content shows the
/// question in place of the browser, with Back and Replace.
///
/// # Size
///
/// The content has one fixed size ([`Self::size`], by default
/// [`Self::DEFAULT_SIZE`]); the browser gets what the footer rows leave. When
/// the window or region is smaller than the content and the dialog's padding
/// and margin, the dialog scrolls it rather than squeezing it. The user
/// cannot resize it.
///
/// # Redescription
///
/// Describing a mounted dialog again reconciles the anchor and applies a new
/// modal scope from the next session. A description with another controller
/// dismisses the mounted controller's open session with
/// [`SurfaceDismissal::OwnerRemoved`](crate::SurfaceDismissal::OwnerRemoved)
/// and mounts the new controller's content in its place. The row height is
/// fixed when the content mounts, as a [`FileBrowser`]'s is; a new size
/// applies at the next layout.
///
/// # Inspection and tests
///
/// The dialog is a [`Dialog`]: it reports
/// [`SemanticRole::Dialog`](crate::SemanticRole::Dialog), named by the
/// session's title, with the `open` property. Its content is the element
/// keyed `picker` under the owner's `dialog` segment, holding the browser
/// under `browser` and the type selector under `type`; its other parts are
/// internal structure without address segments of their own. The parts a
/// test operates are [exposed parts](Element::expose_part) of the content:
/// `accept` and `cancel` for the buttons, `name` for the name field, `filter`
/// for the type selector, `refusal` for the line that says why confirm was
/// refused and `question` for the replace question, and, when the host can
/// create folders, `new-folder` for the button and `folder-name` for the
/// field that takes the new folder's name. The places and recent
/// folders are buttons under the content's `shortcuts` segment, keyed
/// `place-N` and `recent-N`. The number stays with a folder while it is
/// listed and says nothing of its position: a list that starts empty
/// numbers its items from zero in the order shown, and a later item takes a
/// number that became free. Workspace capture includes the browser's split
/// position whether or not a session is open; nothing of a session and
/// nothing of that list is captured, and what the dialog remembers between
/// sessions belongs to the [`FileDialogMemory`](super::FileDialogMemory).
///
/// See [`FileDialogController`] for a complete example.
#[must_use]
pub struct FileDialog {
    controller: FileDialogController,
    scope: Scope,
    size: Size,
    row_height: f32,
    anchor: Option<Element>,
    /// Built with the constructor's state owner, which owns its open state.
    selector: Option<Select<String>>,
    outcome: Option<OutcomeFn>,
    notifier: Option<Notifier>,
    /// Identifies this mount's owner to the session controller.
    token: Rc<()>,
}
type OutcomeFn = Box<dyn FnMut(&mut EventContext, FileDialogOutcome) -> Result<(), RuntimeError>>;
description_debug!(FileDialog);
impl FileDialog {
    /// The content's width and height in logical pixels unless
    /// [`Self::size`] replaces them.
    pub const DEFAULT_SIZE: [u16; 2] = [720, 480];
    /// The smallest content width and height [`Self::size`] accepts, in
    /// logical pixels.
    pub const MINIMUM_SIZE: [u16; 2] = [480, 320];
    /// The browser's row height unless [`Self::row_height`] replaces it, in
    /// logical pixels.
    pub const DEFAULT_ROW_HEIGHT: f32 = 24.0;

    /// Describes a dialog that is modal to its owner's window.
    ///
    /// `ui` is any [`StateOwner`](crate::StateOwner) of the runtime the
    /// dialog mounts in; it owns the type selector's open state.
    pub fn in_window(
        ui: &impl crate::StateOwner,
        controller: &FileDialogController,
        anchor: Element,
    ) -> Self {
        Self::with_scope(ui, controller, anchor, Scope::Window)
    }
    /// Describes a dialog confined to its anchor's region: it is presented
    /// over that region and blocks only that view.
    pub fn in_view(
        ui: &impl crate::StateOwner,
        controller: &FileDialogController,
        anchor: Element,
    ) -> Self {
        Self::with_scope(ui, controller, anchor, Scope::View)
    }
    fn with_scope(
        ui: &impl crate::StateOwner,
        controller: &FileDialogController,
        anchor: Element,
        scope: Scope,
    ) -> Self {
        let inner = &controller.0;
        Self {
            controller: controller.clone(),
            scope,
            size: Size::from(Self::DEFAULT_SIZE),
            row_height: Self::DEFAULT_ROW_HEIGHT,
            anchor: Some(anchor),
            selector: Some(Select::bound(
                ui,
                inner.filter.clone(),
                inner.texts.choices.clone(),
            )),
            outcome: None,
            notifier: None,
            token: Rc::new(()),
        }
    }
    /// Calls `outcome` with each ended session's outcome, so the application
    /// can act on it without watching [`FileDialogController::status`].
    ///
    /// The mounted dialog takes the outcome from its controller and hands it
    /// over [after settling](https://github.com/pertyjons/pert-ui/blob/main/docs/contracts.md#after-settling):
    /// in an event phase, once the flush that follows the ending has settled.
    /// The callback may write signals and may open the next session. Dialogs
    /// are called in the order their sessions ended, and a hidden dialog is
    /// called too.
    ///
    /// The retained outcome has one taker. The callback runs at most once per
    /// ended session, for a session that ended while this dialog was the
    /// controller's mounted owner; its error is returned from the flush and
    /// does not bring the outcome back. It does not run, and the outcome
    /// stays with the controller for [`FileDialogController::take_outcome`],
    /// when the application takes the outcome first, when the dialog is
    /// removed before the call (which also ends an open session as
    /// dismissed), and when the session ended with no dialog mounted or with
    /// another dialog as owner. Without this callback the outcome always
    /// stays with the controller.
    ///
    /// It replaces an earlier callback, and a description without one removes
    /// it from the mounted dialog.
    pub fn on_outcome(
        mut self,
        outcome: impl FnMut(&mut EventContext, FileDialogOutcome) -> Result<(), RuntimeError> + 'static,
    ) -> Self {
        self.outcome = Some(Box::new(outcome));
        self
    }
    /// Sets the content's size in logical pixels, replacing an earlier one.
    ///
    /// # Errors
    /// [`FileDialogError::TooSmall`] for a size narrower or lower than
    /// [`Self::MINIMUM_SIZE`].
    pub fn size(mut self, size: Size) -> Result<Self, FileDialogError> {
        let minimum = Size::from(Self::MINIMUM_SIZE);
        if size.width() < minimum.width() || size.height() < minimum.height() {
            return Err(FileDialogError::TooSmall);
        }
        self.size = size;
        Ok(self)
    }
    /// Sets the height of the browser's tree and list rows in logical pixels,
    /// replacing an earlier one.
    ///
    /// # Errors
    /// [`FileDialogError::Geometry`] for a height that is not finite,
    /// negative or zero.
    pub fn row_height(mut self, height: f32) -> Result<Self, FileDialogError> {
        // The browser's own validation, before any part is built.
        let _ = FileBrowser::new(&self.controller.0.browser, height)?;
        self.row_height = height;
        Ok(self)
    }
    /// The content element: keyed by the controller's identity inside a
    /// structural wrapper, so another controller mounts its own content
    /// while the content's key path stays `picker`.
    fn content(&mut self) -> Element {
        let controller = &self.controller;
        let inner = &controller.0;
        let mut parts = Vec::new();
        // The row height was validated when it was set; a failure here would
        // be the default's, and leaves the dialog without a browser.
        if let Ok(browser) = FileBrowser::new(&inner.browser, self.row_height) {
            parts.push(Element::new("browser", browser));
        }
        let go = controller.clone();
        let numbers: Rc<RefCell<(Slots<FilePlace>, Slots<PathBuf>)>> = Rc::default();
        parts.push(Element::new(
            "shortcuts",
            Dynamic::new(inner.texts.shortcuts.clone(), move |_, shortcuts| {
                let mut numbers = numbers.borrow_mut();
                let mut items = Vec::new();
                let mut section =
                    |name: &str, heading: &str, folders: Vec<(usize, String, PathBuf)>| {
                        if folders.is_empty() {
                            return;
                        }
                        items.push(
                            Element::new(format!("{name}-heading"), Label::new(heading))
                                .structural(),
                        );
                        for (number, label, folder) in folders {
                            let go = go.clone();
                            items.push(Element::new(
                                format!("{name}-{number}"),
                                Button::new(label, move |_| go.go_to(&folder)),
                            ));
                        }
                    };
                section(
                    "place",
                    &shortcuts.places_heading,
                    numbers
                        .0
                        .assign(&shortcuts.places)
                        .into_iter()
                        .map(|(number, place)| {
                            (number, place.label().to_owned(), place.folder().to_owned())
                        })
                        .collect(),
                );
                // A recent folder is shown by its name, or by its whole path
                // when another shares the name.
                let named = |folder: &PathBuf| super::display_name(folder);
                section(
                    "recent",
                    &shortcuts.recent_heading,
                    numbers
                        .1
                        .assign(&shortcuts.recent)
                        .into_iter()
                        .map(|(number, folder)| {
                            let name = named(&folder);
                            let shared = shortcuts
                                .recent
                                .iter()
                                .any(|other| *other != folder && named(other) == name);
                            let label = if shared {
                                folder.to_string_lossy().into_owned()
                            } else {
                                name
                            };
                            (number, label, folder)
                        })
                        .collect(),
                );
                Element::new(
                    "scroll",
                    ScrollArea::new(Element::new("items", Column::new(items)).structural())
                        .axes(ScrollAxes::Vertical),
                )
                .structural()
            }),
        ));
        let submit = controller.clone();
        parts.push(
            Element::new(
                "name",
                TextField::new(inner.name.clone())
                    .on_submit(move |_, _| submit.accept().map(|_| ())),
            )
            .with_label(inner.texts.name_label.clone())
            .structural()
            .expose_part("name"),
        );
        let create = controller.clone();
        parts.push(
            Element::new(
                "folder-name",
                TextField::new(inner.folder_name.clone())
                    .on_submit(move |_, _| create.accept().map(|_| ())),
            )
            .with_label(inner.texts.folder_label.clone())
            .structural()
            .expose_part("folder-name"),
        );
        let begin = controller.clone();
        parts.push(
            Element::new(
                "new-folder",
                Button::new(inner.texts.new_folder.clone(), move |_| {
                    begin.begin_folder()
                }),
            )
            .structural()
            .expose_part("new-folder"),
        );
        if let Some(selector) = self.selector.take() {
            parts.push(
                Element::new("type", selector)
                    .with_label(inner.texts.type_label.clone())
                    .expose_part("filter"),
            );
        }
        parts.push(
            Element::new("question", Label::new(inner.texts.question.clone()))
                .structural()
                .expose_part("question"),
        );
        parts.push(
            Element::new("refusal", Label::new(inner.texts.message.clone()))
                .structural()
                .expose_part("refusal"),
        );
        let cancel = controller.clone();
        parts.push(
            Element::new(
                "cancel",
                Button::new(inner.texts.cancel.clone(), move |_| cancel.back_or_cancel()),
            )
            .structural()
            .expose_part("cancel"),
        );
        let accept = controller.clone();
        parts.push(
            Element::new(
                "accept",
                Button::new(inner.texts.accept.clone(), move |_| {
                    accept.accept().map(|_| ())
                })
                .enabled(inner.can_confirm.clone()),
            )
            .structural()
            .expose_part("accept"),
        );
        let picker = Element::new(
            "picker",
            Picker {
                controller: controller.clone(),
                size: self.size,
                parts,
                seen: None,
                pending: Change::default(),
                notifier: None,
                shows: Shows::default(),
                focus: None,
            },
        );
        Element::new(format!("controller-{}", inner.id), Pass(vec![picker])).structural()
    }
}
impl Component for FileDialog {
    fn reconcile(&mut self, next: Self) -> Option<crate::Invalidation> {
        // The owner and the content are reconciled children; they carry the
        // new controller, scope and size themselves.
        self.controller = next.controller;
        self.scope = next.scope;
        self.size = next.size;
        self.row_height = next.row_height;
        self.outcome = next.outcome;
        // The status is read again for the adopted controller and callback.
        Some(crate::Invalidation::Paint)
    }
    fn take_children(&mut self) -> Vec<Element> {
        let Some(anchor) = self.anchor.take() else {
            return Vec::new();
        };
        let content = self.content();
        let dialog = Dialog::new(self.controller.0.dialog.clone(), anchor, content)
            .with_token(self.token.clone());
        let dialog = match self.scope {
            Scope::Window => dialog,
            Scope::View => dialog.view_modal(),
        };
        vec![
            Element::new("owner", dialog)
                .with_label(self.controller.0.texts.title.clone())
                .structural(),
        ]
    }
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        let status = cx.read(&self.controller.status())?;
        let dialog = &self.controller.0.dialog;
        let notifier = self.notifier.get_or_insert_with(|| cx.notifier());
        // The controller requests delivery at each ending from here on, in
        // the order sessions end and also while this owner is hidden.
        dialog.watch(&self.token, notifier);
        // An ending from before that registration is requested here.
        if status == FileDialogStatus::Finished
            && self.outcome.is_some()
            && dialog.ended_with(&self.token)
        {
            notifier.request();
        }
        Ok(())
    }
    fn deliver_notifications(&mut self, cx: &mut EventContext) -> Result<(), RuntimeError> {
        let Some(callback) = &mut self.outcome else {
            return Ok(());
        };
        // Only the session this mount owned is its to report: another
        // dialog's, or one that ended by this owner's removal, stays with
        // the controller.
        if !self.controller.0.dialog.ended_with(&self.token) {
            return Ok(());
        }
        // Taken here, so a repeated request finds nothing to report.
        match self.controller.take_outcome()? {
            Some(outcome) => callback(cx, outcome),
            None => Ok(()),
        }
    }
    fn measure_limits(&self, cx: &MeasureContext<'_>) -> Result<SizeLimits, RuntimeError> {
        if cx.is_empty() {
            Ok(SizeLimits::default())
        } else {
            cx.limits(0)
        }
    }
    fn measure(
        &self,
        cx: &MeasureContext<'_>,
        available: Constraints,
    ) -> Result<Size, RuntimeError> {
        if cx.is_empty() {
            return Ok(Size::ZERO);
        }
        cx.measure(0, available)
    }
    fn arrange(&self, cx: &mut ArrangeContext<'_>, size: Size) -> Result<(), RuntimeError> {
        if cx.is_empty() {
            return Ok(());
        }
        cx.place(0, rect(0.0, 0.0, size.width(), size.height())?)
    }
}

/// Numbers for listed items that stay with an item while it is listed, so a
/// reordered list never puts another item under a key that holds a press. A
/// number is free for a new item only from the build after its item left.
struct Slots<T>(Vec<Option<T>>);
impl<T> Default for Slots<T> {
    fn default() -> Self {
        Self(Vec::new())
    }
}
impl<T: Clone + PartialEq> Slots<T> {
    /// The distinct `items` in their order, each with its number.
    fn assign(&mut self, items: &[T]) -> Vec<(usize, T)> {
        let left: Vec<usize> = (0..self.0.len())
            .filter(|&slot| {
                self.0[slot]
                    .as_ref()
                    .is_some_and(|held| !items.contains(held))
            })
            .collect();
        let mut assigned: Vec<(usize, T)> = Vec::new();
        for item in items {
            if assigned.iter().any(|(_, seen)| seen == item) {
                continue;
            }
            let held = self.0.iter().position(|slot| slot.as_ref() == Some(item));
            let number = held.unwrap_or_else(|| {
                let free = self.0.iter().position(Option::is_none);
                let number = free.unwrap_or(self.0.len());
                if free.is_none() {
                    self.0.push(None);
                }
                self.0[number] = Some(item.clone());
                number
            });
            assigned.push((number, item.clone()));
        }
        for slot in left {
            self.0[slot] = None;
        }
        while self.0.last().is_some_and(Option::is_none) {
            self.0.pop();
        }
        assigned
    }
}

/// Lays its only child out as itself; a key without a presentation.
struct Pass(Vec<Element>);
impl Component for Pass {
    fn take_children(&mut self) -> Vec<Element> {
        std::mem::take(&mut self.0)
    }
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
    fn measure_limits(&self, cx: &MeasureContext<'_>) -> Result<SizeLimits, RuntimeError> {
        if cx.is_empty() {
            Ok(SizeLimits::default())
        } else {
            cx.limits(0)
        }
    }
    fn measure(
        &self,
        cx: &MeasureContext<'_>,
        available: Constraints,
    ) -> Result<Size, RuntimeError> {
        if cx.is_empty() {
            return Ok(Size::ZERO);
        }
        cx.measure(0, available)
    }
    fn arrange(&self, cx: &mut ArrangeContext<'_>, size: Size) -> Result<(), RuntimeError> {
        if cx.is_empty() {
            return Ok(());
        }
        cx.place(0, rect(0.0, 0.0, size.width(), size.height())?)
    }
}

/// Which of the content's optional parts the session shows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Shows {
    shortcuts: bool,
    /// The New folder button.
    new_folder: bool,
    /// The field for a new folder's name.
    folder_name: bool,
    name: bool,
    filter: bool,
    question: bool,
}

/// What confirm is judged from, as the content last saw it.
#[derive(Clone, PartialEq)]
struct Seen {
    session: super::Config,
    current: PathBuf,
    active: Option<String>,
    selected: std::rc::Rc<std::collections::BTreeSet<String>>,
    text_filter: String,
    filter: String,
    name: String,
    folder_name: String,
}

/// The dialog's content: the browser over a footer of fixed-height rows.
struct Picker {
    controller: FileDialogController,
    size: Size,
    parts: Vec<Element>,
    seen: Option<Seen>,
    /// Changes observed and not yet handed to the controller.
    pending: Change,
    notifier: Option<Notifier>,
    shows: Shows,
    /// A part to move focus to after settling, and whether to the first
    /// focusable component inside it rather than to the part itself.
    focus: Option<(crate::WidgetId, bool)>,
}
impl Picker {
    /// The widest the column of places and recent folders gets, in logical
    /// pixels.
    const SHORTCUTS_WIDTH: f32 = 168.0;
}
impl Component for Picker {
    fn reconcile(&mut self, next: Self) -> Option<crate::Invalidation> {
        let changed = self.size != next.size;
        self.size = next.size;
        changed.then_some(crate::Invalidation::Layout)
    }
    fn take_children(&mut self) -> Vec<Element> {
        std::mem::take(&mut self.parts)
    }
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        let inner = &self.controller.0;
        let selection = inner.browser.selection();
        let session = cx.read(&inner.config.read_only())?;
        let question = cx.read(&inner.question.read_only())?.is_some();
        let seen = Seen {
            current: cx.read(&inner.browser.current())?,
            active: cx.read(&selection.active())?,
            selected: cx.read(&selection.selected())?,
            text_filter: cx.read(&inner.browser.filter().read_only())?,
            filter: cx.read(&inner.filter.read_only())?,
            name: cx.read(&inner.name.read_only())?,
            folder_name: cx.read(&inner.folder_name.read_only())?,
            session: session.clone(),
        };
        let creating = cx.read(&inner.creating.read_only())?.is_some();
        // The revision is read for its subscription: the capability and the
        // permission themselves are replaced through the controller.
        let _ = cx.read(&inner.capability.read_only())?;
        let may_create = inner.can_operate.get() && inner.new_folder.borrow().read_paint(cx)?;
        let listed = matches!(
            cx.read(&inner.browser.listing())?,
            crate::CurrentListing::Loaded(_)
        );
        if creating && !may_create && !self.pending.revoked {
            self.pending.revoked = true;
            self.notifier.get_or_insert_with(|| cx.notifier()).request();
        }
        let shows = Shows {
            shortcuts: !cx.read(&inner.texts.shortcuts)?.is_empty() && !question,
            new_folder: may_create && listed && !creating && !question,
            folder_name: creating && !question,
            name: session.0.mode == FileDialogMode::SaveFile && !question && !creating,
            filter: !session.0.filters.is_empty()
                && session.0.mode != FileDialogMode::ChooseFolder
                && !question
                && !creating,
            question,
        };
        if shows != self.shows {
            // Focus follows the naming: into the field when it appears, and
            // when it goes away to the button that opened it or, where that
            // is not offered, into the browser.
            let child = |key: &str| {
                cx.children()
                    .iter()
                    .find(|child| child.key() == key)
                    .map(|child| child.id().clone())
            };
            let focus = if shows.folder_name && !self.shows.folder_name {
                child("folder-name").map(|field| (field, false))
            } else if !shows.folder_name && self.shows.folder_name && !shows.question {
                if shows.new_folder {
                    child("new-folder").map(|button| (button, false))
                } else {
                    child("browser").map(|browser| (browser, true))
                }
            } else {
                None
            };
            if focus.is_some() {
                self.focus = focus;
                self.notifier.get_or_insert_with(|| cx.notifier()).request();
            }
            self.shows = shows;
            cx.request_layout();
        }
        if let Some(previous) = &self.seen
            && *previous != seen
            // A new session starts from its own state; nothing was changed
            // by the user yet.
            && previous.session == seen.session
        {
            self.pending.filter |= previous.filter != seen.filter;
            self.pending.active |= previous.active != seen.active;
            self.pending.name |= previous.name != seen.name;
            self.pending.current |= previous.current != seen.current;
            self.notifier.get_or_insert_with(|| cx.notifier()).request();
        }
        self.seen = Some(seen);
        Ok(())
    }
    fn deliver_notifications(&mut self, cx: &mut EventContext) -> Result<(), RuntimeError> {
        if let Some((target, within)) = self.focus.take() {
            if within {
                cx.request_focus_within(&target)?;
            } else {
                cx.request_focus_for(&target)?;
            }
        }
        let change = std::mem::take(&mut self.pending);
        if change == Change::default() {
            return Ok(());
        }
        self.controller.observed(change)
    }
    fn child_visible(&self, key: &str) -> bool {
        match key {
            "browser" => !self.shows.question,
            "shortcuts" => self.shows.shortcuts,
            "name" => self.shows.name,
            "folder-name" => self.shows.folder_name,
            "new-folder" => self.shows.new_folder,
            "type" => self.shows.filter,
            "question" => self.shows.question,
            _ => true,
        }
    }
    fn size_limits(&self) -> SizeLimits {
        // Hard limits: the dialog scrolls a content it cannot fit, and never
        // arranges it narrower than this.
        SizeLimits::exact(self.size)
    }
    fn measure(&self, _: &MeasureContext<'_>, _: Constraints) -> Result<Size, RuntimeError> {
        Ok(self.size)
    }
    fn arrange(&self, cx: &mut ArrangeContext<'_>, size: Size) -> Result<(), RuntimeError> {
        let gap = cx.measurement().metric(MetricRole::ItemGap);
        let row = cx.measurement().metric(MetricRole::ControlHeight);
        let (width, height) = (size.width(), size.height());
        let index = |cx: &ArrangeContext<'_>, key: &str| -> Result<Option<usize>, RuntimeError> {
            for index in 0..cx.len() {
                if cx.measurement().key(index)? == key {
                    return Ok(Some(index));
                }
            }
            Ok(None)
        };
        let natural = |cx: &ArrangeContext<'_>, index: usize, most: f32| {
            cx.measure(index, Constraints::new(None, None)?)
                .map(|size| size.width().min(most))
        };
        // The action row: the status line, then cancel and confirm.
        let actions = (height - row).max(0.0);
        let mut right = width;
        // With New folder in the row the buttons share it more tightly, so
        // the refusal line always keeps a part of it.
        let crowded = index(cx, "new-folder")?.is_some();
        let most = if crowded { 0.28 } else { 0.35 };
        for key in ["accept", "cancel"] {
            if let Some(button) = index(cx, key)? {
                // Neither button may take the row from the other and the
                // refusal line.
                let wide = natural(cx, button, (width * most).min(right))?;
                right -= wide;
                cx.place(button, rect(right, actions, wide, row)?)?;
                right = (right - gap).max(0.0);
            }
        }
        // The New folder button leads the row; the status line has the rest.
        let mut left = 0.0;
        if let Some(button) = index(cx, "new-folder")? {
            let wide = natural(cx, button, (width * 0.22).min(right))?;
            cx.place(button, rect(0.0, actions, wide, row)?)?;
            left = (wide + gap).min(right);
        }
        if let Some(status) = index(cx, "refusal")? {
            cx.place(status, rect(left, actions, right - left, row)?)?;
        }
        let mut bottom = (actions - gap).max(0.0);
        if let Some(question) = index(cx, "question")? {
            cx.place(question, rect(0.0, 0.0, width, bottom)?)?;
        }
        // The footer row holds the new folder's name while one is created.
        if let Some(folder) = index(cx, "folder-name")? {
            let top = (bottom - row).max(0.0);
            cx.place(folder, rect(0.0, top, width, row)?)?;
            bottom = (top - gap).max(0.0);
        }
        // Otherwise: the name field, then the type selector.
        let name = index(cx, "name")?;
        let filter = index(cx, "type")?;
        if name.is_some() || filter.is_some() {
            let top = (bottom - row).max(0.0);
            let mut right = width;
            if let Some(filter) = filter {
                let most = if name.is_some() { width * 0.4 } else { width };
                let wide = natural(cx, filter, most)?;
                right -= wide;
                cx.place(filter, rect(right, top, wide, row)?)?;
                right = (right - gap).max(0.0);
            }
            if let Some(name) = name {
                cx.place(name, rect(0.0, top, right, row)?)?;
            }
            bottom = (top - gap).max(0.0);
        }
        // Places and recent folders take a column at the left when there
        // are any.
        let mut left = 0.0;
        if let Some(shortcuts) = index(cx, "shortcuts")? {
            let wide = Self::SHORTCUTS_WIDTH.min(width * 0.3);
            cx.place(shortcuts, rect(0.0, 0.0, wide, bottom)?)?;
            left = (wide + gap).min(width);
        }
        if let Some(browser) = index(cx, "browser")? {
            cx.place(browser, rect(left, 0.0, width - left, bottom)?)?;
        }
        Ok(())
    }
}
