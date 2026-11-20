//! The mounted file dialog: its owner and the content it shows.
use super::{
    FileDialogController, FileDialogError, FileDialogMode, FileDialogOutcome, FileDialogStatus,
};
use crate::layout::rect;
use crate::{
    ArrangeContext, Button, Component, Constraints, Dialog, Element, EventContext, FileBrowser,
    Label, MeasureContext, MetricRole, Notifier, RuntimeError, Select, Size, SizeLimits, TextField,
    UpdateContext,
};
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
/// [`FileBrowser`], in a save dialog a name field, a file-type selector when
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
/// refused and `question` for the replace question. Workspace capture
/// includes the browser's split position whether or not a session is open;
/// nothing of a session is captured.
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
            session: session.clone(),
        };
        let shows = Shows {
            name: session.0.mode == FileDialogMode::SaveFile && !question,
            filter: !session.0.filters.is_empty()
                && session.0.mode != FileDialogMode::ChooseFolder
                && !question,
            question,
        };
        if shows != self.shows {
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
    fn deliver_notifications(&mut self, _: &mut EventContext) -> Result<(), RuntimeError> {
        let change = std::mem::take(&mut self.pending);
        self.controller.observed(change)
    }
    fn child_visible(&self, key: &str) -> bool {
        match key {
            "browser" => !self.shows.question,
            "name" => self.shows.name,
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
        for key in ["accept", "cancel"] {
            if let Some(button) = index(cx, key)? {
                // Neither button may take the row from the other and the
                // refusal line.
                let wide = natural(cx, button, (width * 0.35).min(right))?;
                right -= wide;
                cx.place(button, rect(right, actions, wide, row)?)?;
                right = (right - gap).max(0.0);
            }
        }
        if let Some(status) = index(cx, "refusal")? {
            cx.place(status, rect(0.0, actions, right, row)?)?;
        }
        let mut bottom = (actions - gap).max(0.0);
        if let Some(question) = index(cx, "question")? {
            cx.place(question, rect(0.0, 0.0, width, bottom)?)?;
        }
        // The footer row: the name field, then the type selector.
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
        if let Some(browser) = index(cx, "browser")? {
            cx.place(browser, rect(0.0, 0.0, width, bottom)?)?;
        }
        Ok(())
    }
}
