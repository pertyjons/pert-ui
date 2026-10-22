//! Modal task surfaces and reserved UI-local outcomes.
use crate::layout::rect;
use crate::*;
use std::{
    cell::{Cell, RefCell},
    rc::{Rc, Weak},
};

/// One terminal response to an opened dialog. Retained until explicitly taken.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DialogOutcome {
    /// An explicit application or confirmation action accepted the task.
    Accepted,
    /// An explicit cancellation action or permitted Escape cancelled the task.
    Cancelled,
    /// Ownership or presentation ended without a task response.
    Dismissed(SurfaceDismissal),
    /// Admission failed, for example because another modal occupies the scope.
    Failed(SurfaceDismissal),
}
/// Observable dialog session state; there is storage for exactly one outcome.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DialogStatus {
    /// No session is open and no outcome is awaiting acknowledgement.
    Idle,
    /// Opening is requested or the dialog is visible.
    Open,
    /// A terminal outcome is reserved until [`DialogController::take_outcome`].
    Finished(DialogOutcome),
}
/// Cloneable UI-thread controller for one retained dialog owner.
///
/// Opening reserves the outcome slot. A second open, including after completion
/// before taking the outcome, returns `DialogBusy`. Clones address the same session;
/// no executor, queue, callback or application I/O is implied. Removal of the
/// mounted owner records a dismissal even if the request has not reached layout
/// yet. Reads/writes follow signal phase and runtime-lifetime restrictions.
///
/// One mounted dialog owner uses a controller at a time. The owner claims it when
/// it mounts, not when it is constructed, so a tree containing the dialog can be
/// described again with the same controller: the new description is applied to
/// the mounted owner. A second owner mounted with the same controller while the
/// first is still mounted stays closed and fails its updates with
/// [`RuntimeError::DialogBusy`] until the first unmounts.
#[derive(Clone)]
#[must_use]
pub struct DialogController {
    status: Signal<DialogStatus>,
    /// The mounted owner's token; empty while no owner is mounted.
    owner: Rc<RefCell<Weak<()>>>,
    closing: Rc<Cell<bool>>,
}
/// Shows the session status, or the error reading it after runtime shutdown,
/// and whether a mounted dialog owner uses this controller.
impl std::fmt::Debug for DialogController {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DialogController")
            .field("status", &self.status.get())
            .field("owned", &self.has_owner())
            .finish()
    }
}
impl DialogController {
    /// Creates an idle controller belonging to `ui`.
    ///
    /// `ui` is any [`StateOwner`](crate::StateOwner): the
    /// [`Runtime`](crate::Runtime) while the application builds its tree, or
    /// the [`UpdateContext`](crate::UpdateContext) or
    /// [`EventContext`](crate::EventContext) of a component that builds this
    /// while it describes its children.
    pub fn new(ui: &impl crate::StateOwner) -> Self {
        Self {
            status: ui.signal(DialogStatus::Idle),
            owner: Rc::new(RefCell::new(Weak::new())),
            closing: Rc::new(Cell::new(false)),
        }
    }
    fn has_owner(&self) -> bool {
        self.owner.borrow().strong_count() > 0
    }
    /// Whether both handles address the same session.
    fn same(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.owner, &other.owner)
    }
    /// Subscribable session state, including the retained terminal outcome.
    pub fn status(&self) -> ReadSignal<DialogStatus> {
        self.status.read_only()
    }
    /// Requests opening and reserves terminal storage. It is refused while open,
    /// awaiting acknowledgement, or before the owner has observed closure in a
    /// flush. This prevents finish/take/open within one event from retaining the
    /// previous session's affirmative focus.
    ///
    /// # Errors
    /// Returns [`RuntimeError::DialogBusy`] while a session is open, its outcome is
    /// untaken, or the owner has not yet observed the closure in a flush; otherwise the
    /// errors of [`Signal::get`](crate::Signal::get) and
    /// [`Signal::set`](crate::Signal::set) on the status signal, such as `Closed` after
    /// shutdown or a phase write error.
    pub fn open(&self) -> Result<(), RuntimeError> {
        if self.status.get()? != DialogStatus::Idle || self.closing.get() {
            return Err(RuntimeError::DialogBusy);
        }
        self.status.set(DialogStatus::Open)?;
        Ok(())
    }
    /// Completes an open session with acceptance. Returns false if already closed.
    ///
    /// # Errors
    /// Returns the errors of [`Signal::get`](crate::Signal::get) and
    /// [`Signal::set`](crate::Signal::set) on the status signal, such as `Closed` after
    /// shutdown or a phase write error.
    pub fn accept(&self) -> Result<bool, RuntimeError> {
        self.finish(DialogOutcome::Accepted)
    }
    /// Completes an open session with cancellation. Returns false if already closed.
    ///
    /// # Errors
    /// Returns the errors of [`Signal::get`](crate::Signal::get) and
    /// [`Signal::set`](crate::Signal::set) on the status signal, such as `Closed` after
    /// shutdown or a phase write error.
    pub fn cancel(&self) -> Result<bool, RuntimeError> {
        self.finish(DialogOutcome::Cancelled)
    }
    fn finish(&self, outcome: DialogOutcome) -> Result<bool, RuntimeError> {
        if self.status.get()? != DialogStatus::Open {
            return Ok(false);
        }
        let changed = self.status.set_changed(DialogStatus::Finished(outcome))?;
        self.closing.set(self.has_owner());
        Ok(changed)
    }
    /// Takes the completed outcome and releases its slot. Returns None for an
    /// idle/open session. Taking an outcome never opens another session implicitly.
    ///
    /// # Errors
    /// Returns the errors of [`Signal::get`](crate::Signal::get) and
    /// [`Signal::set`](crate::Signal::set) on the status signal, such as `Closed` after
    /// shutdown or a phase write error.
    pub fn take_outcome(&self) -> Result<Option<DialogOutcome>, RuntimeError> {
        let DialogStatus::Finished(outcome) = self.status.get()? else {
            return Ok(None);
        };
        self.status.set(DialogStatus::Idle)?;
        Ok(Some(outcome))
    }
}

/// A retained modal task surface wrapping ordinary owner content.
///
/// `anchor` occupies ordinary layout (typically an opener or an entire view).
/// The dialog content appears centered in the window inside a rounded Surface
/// panel with a PanelBorder outline, GroupPadding and scroll access. Window modality is default; [`Self::view_modal`] confines both
/// presentation and blocking to this owner's allocated region. Tab stays within
/// the focused modal chain, and scoped shortcuts cannot bubble into blocked
/// ancestors. Nested dialogs are allowed; unrelated overlapping modal admission
/// fails with `RuntimeError::ModalConflict` and a retained failure outcome.
///
/// Escape cancels by default; outside clicks never close or click through.
/// Native focus loss resets input while leaving the session open. A natively
/// hidden root (minimized, occluded or zero-sized) only suspends the session: it
/// stays open and is presented again when the root is shown. Logically hiding
/// the owner or an ancestor, removing the owner and lack of presentation space
/// terminate the session. Closing restores an eligible opener. Content/update
/// errors propagate through runtime recovery; they do not imply acceptance or
/// retry application actions.
///
/// The controller is claimed when the dialog mounts; see [`DialogController`]
/// for a second concurrent owner. Describing the dialog again applies the new
/// description to the mounted owner: its content and anchor reconcile by key, a
/// changed [`Self::require_response`] and accessible name apply at once, and a
/// changed modality applies from the next session, since an open session keeps
/// the scope it was admitted with. A description with a different controller
/// dismisses the old controller's open session with
/// [`SurfaceDismissal::OwnerRemoved`] and presents the new controller's state.
#[must_use]
pub struct Dialog {
    state: DialogController,
    /// Identifies this mount as the controller's owner.
    token: Rc<()>,
    /// An alert's title, which names it.
    alert: Option<Caption>,
    anchor: Option<Element>,
    content: Option<Element>,
    shown: bool,
    /// The modality of the presented session.
    kind: SurfaceKind,
    /// The described modality, adopted while no session is presented.
    described: SurfaceKind,
    escape: bool,
}
impl Dialog {
    /// Describes one owner using `controller` and arbitrary content. The
    /// controller is claimed when the dialog mounts, not here.
    pub fn new(controller: DialogController, anchor: Element, content: Element) -> Self {
        Self::with_parts(controller, anchor, Some(content))
    }
    fn with_parts(controller: DialogController, anchor: Element, content: Option<Element>) -> Self {
        Self {
            state: controller,
            token: Rc::new(()),
            alert: None,
            anchor: Some(anchor),
            content,
            shown: false,
            kind: SurfaceKind::Dialog,
            described: SurfaceKind::Dialog,
            escape: true,
        }
    }
    /// Blocks every window except this dialog's admitted chain. Admission requires
    /// an available, nonzero presentation root and rejects unrelated modal chains.
    /// Uses the invoking root; native activation is never assumed.
    pub fn application_modal(mut self) -> Self {
        self.kind = SurfaceKind::ApplicationDialog;
        self.described = self.kind;
        self
    }
    /// Blocks only the owner's presentation subtree; other views remain usable.
    pub fn view_modal(mut self) -> Self {
        self.kind = SurfaceKind::ViewDialog;
        self.described = self.kind;
        self
    }
    /// Requires an explicit action: Escape is consumed without closing.
    pub fn require_response(mut self) -> Self {
        self.escape = false;
        self
    }
    fn owns(&self) -> bool {
        self.state
            .owner
            .borrow()
            .upgrade()
            .is_some_and(|owner| Rc::ptr_eq(&owner, &self.token))
    }
    /// Takes the controller over unless another live mount owns it. Returns
    /// whether this mount owns it.
    fn claim(&self) -> bool {
        let mut owner = self.state.owner.borrow_mut();
        match owner.upgrade() {
            Some(current) if !Rc::ptr_eq(&current, &self.token) => false,
            _ => {
                *owner = Rc::downgrade(&self.token);
                true
            }
        }
    }
    /// Dismisses an open session and gives the controller up, if this mount owns it.
    fn release(&self) {
        if !self.owns() {
            return;
        }
        if self.state.status.get().ok() == Some(DialogStatus::Open) {
            self.state
                .status
                .publish_copy(DialogStatus::Finished(DialogOutcome::Dismissed(
                    SurfaceDismissal::OwnerRemoved,
                )));
        }
        *self.state.owner.borrow_mut() = Weak::new();
        self.state.closing.set(false);
    }
}
impl Drop for Dialog {
    fn drop(&mut self) {
        // Normally already released by `unmounted`; a description that never
        // mounted owns nothing.
        self.release();
    }
}
impl Component for Dialog {
    fn reconcile(&mut self, mut next: Self) -> Option<Invalidation> {
        let mut invalidation = None;
        if !self.state.same(&next.state) {
            self.release();
            // `next` keeps the old controller, which its token never owned, so
            // dropping it releases nothing.
            std::mem::swap(&mut self.state, &mut next.state);
            self.claim();
            invalidation = Some(Invalidation::Layout);
        }
        if self.described != next.described {
            self.described = next.described;
            if !self.shown {
                self.kind = self.described;
            }
            invalidation = Some(Invalidation::Layout);
        }
        let alert_changed = match (&mut self.alert, next.alert.take()) {
            (Some(mine), Some(theirs)) => mine.reconcile(theirs),
            (None, None) => false,
            (mine, theirs) => {
                *mine = theirs;
                true
            }
        };
        if self.escape != next.escape || alert_changed {
            self.escape = next.escape;
            invalidation.get_or_insert(Invalidation::Paint);
        }
        invalidation
    }
    fn mounted(&mut self) {
        self.claim();
    }
    fn unmounted(&mut self) {
        self.release();
    }
    fn take_children(&mut self) -> Vec<Element> {
        let (Some(anchor), Some(content)) = (self.anchor.take(), self.content.take()) else {
            return Vec::new();
        };
        vec![
            Element::new("anchor", Container::new(anchor)).structural(),
            Element::new(
                "dialog",
                DialogPanel {
                    children: vec![
                        Element::new(
                            "panel",
                            crate::popover::transient_panel(
                                Element::new(
                                    "scroll",
                                    ScrollArea::new(
                                        Element::new(
                                            "body",
                                            Container::new(content)
                                                .padding(MetricRole::GroupPadding),
                                        )
                                        .structural(),
                                    )
                                    .axes(ScrollAxes::Both),
                                )
                                .structural(),
                            ),
                        )
                        .structural(),
                    ],
                },
            ),
        ]
    }
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        // Claimed again on every update, so a mount that found the controller
        // taken takes it over once the earlier owner has unmounted.
        if !self.claim() {
            self.shown = false;
            return Err(RuntimeError::DialogBusy);
        }
        if let Some(alert) = &mut self.alert {
            // The heading child draws the title; the name only follows it.
            alert.update_paint(cx)?;
        }
        self.shown = cx.read(&self.state.status())? == DialogStatus::Open;
        if !self.shown {
            self.state.closing.set(false);
            self.kind = self.described;
        }
        Ok(())
    }
    fn child_visible(&self, key: &str) -> bool {
        key != "dialog" || self.shown
    }
    fn child_surface(&self, key: &str) -> Option<SurfaceKind> {
        (key == "dialog").then_some(self.kind)
    }
    fn dismiss_surface(
        &mut self,
        _: &mut EventContext,
        reason: SurfaceDismissal,
    ) -> Result<(), RuntimeError> {
        if matches!(
            reason,
            SurfaceDismissal::OutsidePress
                | SurfaceDismissal::FocusLeft
                | SurfaceDismissal::WindowFocusLost
        ) || (reason == SurfaceDismissal::Escape && !self.escape)
        {
            return Ok(());
        }
        let outcome = match reason {
            SurfaceDismissal::Escape => DialogOutcome::Cancelled,
            SurfaceDismissal::ModalConflict | SurfaceDismissal::NoSpace => {
                DialogOutcome::Failed(reason)
            }
            _ => DialogOutcome::Dismissed(reason),
        };
        self.state.finish(outcome)?;
        self.shown = false;
        Ok(())
    }
    fn measure(&self, cx: &MeasureContext<'_>, a: Constraints) -> Result<Size, RuntimeError> {
        crate::popover::measure_anchor(cx, a)
    }
    fn arrange(&self, cx: &mut ArrangeContext<'_>, s: Size) -> Result<(), RuntimeError> {
        crate::popover::arrange_surface(cx, s)
    }
    fn keyboard_event(
        &mut self,
        _: &mut EventContext,
        _: KeyEvent,
        _: &KeyboardState,
    ) -> Result<InputOutcome, RuntimeError> {
        Ok(InputOutcome::Ignored)
    }
    fn inspect(&self) -> ComponentDetails {
        let mut d = ComponentDetails::default();
        d.properties
            .insert("open".into(), InspectValue::Bool(self.shown));
        d.properties.insert(
            "window_modal".into(),
            InspectValue::Bool(self.kind == SurfaceKind::Dialog),
        );
        d.semantics = Some(
            Semantics::new(if self.alert.is_some() {
                SemanticRole::AlertDialog
            } else {
                SemanticRole::Dialog
            })
            .name(self.alert.as_ref().map(Caption::text).unwrap_or_default())
            .expanded(self.shown)
            .enabled_actions((self.shown && self.escape).then_some(SemanticAction::Dismiss)),
        );
        d
    }
}
struct DialogPanel {
    children: Vec<Element>,
}
impl Component for DialogPanel {
    fn take_children(&mut self) -> Vec<Element> {
        std::mem::take(&mut self.children)
    }
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
    fn measure(&self, cx: &MeasureContext<'_>, a: Constraints) -> Result<Size, RuntimeError> {
        cx.measure(0, a)
    }
    fn arrange(&self, cx: &mut ArrangeContext<'_>, size: Size) -> Result<(), RuntimeError> {
        let margin = cx.measurement().metric(MetricRole::GroupPadding);
        let width = (size.width() - 2.0 * margin).max(0.0);
        let height = (size.height() - 2.0 * margin).max(0.0);
        let desired = cx.measure(0, Constraints::new(Some(width), Some(height))?)?;
        let w = desired.width().min(width);
        let h = desired.height().min(height);
        cx.place(
            0,
            rect((size.width() - w) / 2.0, (size.height() - h) / 2.0, w, h)?,
        )
    }
    fn paint(&self, p: &mut Painter) -> Result<(), DrawError> {
        if let Some(size) = p.size() {
            p.fill_rect(Rect::from_size(size), Color::rgba(0.0, 0.0, 0.0, 0.35)?);
        }
        Ok(())
    }
    fn pointer_input(
        &mut self,
        _: &mut EventContext,
        _: PointerEvent,
    ) -> Result<InputOutcome, RuntimeError> {
        Ok(InputOutcome::Handled)
    }
    fn keyboard_event(
        &mut self,
        _: &mut EventContext,
        _: KeyEvent,
        _: &KeyboardState,
    ) -> Result<InputOutcome, RuntimeError> {
        Ok(InputOutcome::Ignored)
    }
}

/// Explicit confirmation with Cancel before the affirmative action in focus order.
///
/// Escape always maps to cancellation, outside clicks do nothing, and affirmation
/// requires its button or an explicit controller call. The host takes the outcome
/// before performing I/O or destructive work; closing is not evidence that work
/// succeeded. Uses the same ownership, modality, receipt and re-description
/// contract as [`Dialog`]: window-modal by default, [`Self::view_modal`] or
/// [`Self::application_modal`] otherwise. Inspection reports a
/// [`SemanticRole::AlertDialog`] named by the title, with the same `open` and
/// `window_modal` properties and Dismiss action as a `Dialog`. Describing it
/// again applies a changed title, message and button labels.
///
/// The title, message and button labels are [`TextSource`]s: fixed plain or
/// rich text, or bindings the heading, message, buttons and accessible name
/// follow while the confirmation stays mounted or open. The title names the
/// confirmation by its plain text.
#[must_use]
pub struct AlertDialog {
    dialog: Dialog,
    /// The title, also kept by `dialog` for the accessible name.
    title: TextSource,
    message: TextSource,
    accept_label: TextSource,
    /// The application's cancel label, or `None` for the string table's.
    cancel_label: Option<TextSource>,
}
impl AlertDialog {
    /// Describes a default-styled heading, message, Cancel and affirmation
    /// buttons. `accept_label` describes the host action, for example "Delete".
    /// `controller` is claimed when the confirmation mounts, as for [`Dialog::new`].
    pub fn new(
        controller: DialogController,
        anchor: Element,
        title: impl Into<TextSource>,
        message: impl Into<TextSource>,
        accept_label: impl Into<TextSource>,
    ) -> Self {
        let title = title.into();
        let mut dialog = Dialog::with_parts(controller, anchor, None);
        dialog.alert = Some(Caption::new(title.clone()));
        Self {
            dialog,
            title,
            message: message.into(),
            accept_label: accept_label.into(),
            cancel_label: None,
        }
    }
    /// Replaces the cancellation button's label and accessible name. By default
    /// the installed [`UiStrings`](crate::UiStrings) supply
    /// [`StringRole::DialogCancel`](crate::StringRole::DialogCancel), `"Cancel"`
    /// in English, and the button follows a replaced table. The button still cancels;
    /// re-describing the confirmation applies a changed label. The label is a
    /// [`TextSource`], so a binding keeps it current.
    ///
    /// ```
    /// # use pert_ui_runtime as pert_ui;
    /// use pert_ui::{AlertDialog, Button, DialogController, Element, Runtime};
    /// use std::time::Duration;
    /// let mut ui = Runtime::new();
    /// let state = DialogController::new(&ui);
    /// let prompt = AlertDialog::new(
    ///     state.clone(),
    ///     Element::new("open", Button::new("Radera…", |_| Ok(()))),
    ///     "Radera spåret?",
    ///     "Det går inte att ångra.",
    ///     "Radera",
    /// )
    /// .cancel_label("Avbryt");
    /// ui.mount_tree(Element::new("prompt", prompt))?;
    /// ui.flush(Duration::ZERO)?;
    /// let cancel = ui.inspect().components.into_iter().find(|c| c.key == "cancel")
    ///     .and_then(|c| c.details.properties.get("text").cloned());
    /// assert_eq!(cancel, Some(pert_ui::InspectValue::Text("Avbryt".into())));
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn cancel_label(mut self, label: impl Into<TextSource>) -> Self {
        self.cancel_label = Some(label.into());
        self
    }
    /// Confines blocking and presentation to the wrapped owner's allocation,
    /// as [`Dialog::view_modal`].
    pub fn view_modal(mut self) -> Self {
        self.dialog = self.dialog.view_modal();
        self
    }
    /// Blocks every window except this confirmation's admitted chain, as
    /// [`Dialog::application_modal`], for example a quit prompt.
    pub fn application_modal(mut self) -> Self {
        self.dialog = self.dialog.application_modal();
        self
    }
}
impl Component for AlertDialog {
    fn reconcile(&mut self, next: Self) -> Option<Invalidation> {
        // The title, message and buttons are reconciled children.
        self.dialog.reconcile(next.dialog)
    }
    fn mounted(&mut self) {
        self.dialog.mounted();
    }
    fn unmounted(&mut self) {
        self.dialog.unmounted();
    }
    fn take_children(&mut self) -> Vec<Element> {
        if self.dialog.anchor.is_some() && self.dialog.content.is_none() {
            let cancel = self.dialog.state.clone();
            let accept = self.dialog.state.clone();
            let title = std::mem::take(&mut self.title);
            self.dialog.content = Some(Element::new(
                "confirmation",
                Column::new([
                    Element::new("title", Label::new(title).style(TextStyle::Heading)).structural(),
                    Element::new("message", Label::new(std::mem::take(&mut self.message)))
                        .structural(),
                    Element::new(
                        "actions",
                        Row::new([
                            Element::new("cancel", {
                                let text = crate::runtime::settings::library_text(
                                    self.dialog.state.status.shared(),
                                    self.cancel_label.take(),
                                    crate::StringRole::DialogCancel,
                                );
                                Button::new(text, move |_| cancel.cancel().map(|_| ()))
                            })
                            .structural(),
                            Element::new(
                                "accept",
                                Button::new(std::mem::take(&mut self.accept_label), move |_| {
                                    accept.accept().map(|_| ())
                                }),
                            )
                            .structural(),
                        ]),
                    )
                    .structural(),
                ]),
            ));
        }
        self.dialog.take_children()
    }
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        self.dialog.update(cx)
    }
    fn child_visible(&self, key: &str) -> bool {
        self.dialog.child_visible(key)
    }
    fn child_surface(&self, key: &str) -> Option<SurfaceKind> {
        self.dialog.child_surface(key)
    }
    fn dismiss_surface(
        &mut self,
        cx: &mut EventContext,
        r: SurfaceDismissal,
    ) -> Result<(), RuntimeError> {
        self.dialog.dismiss_surface(cx, r)
    }
    fn measure(&self, cx: &MeasureContext<'_>, a: Constraints) -> Result<Size, RuntimeError> {
        self.dialog.measure(cx, a)
    }
    fn arrange(&self, cx: &mut ArrangeContext<'_>, s: Size) -> Result<(), RuntimeError> {
        self.dialog.arrange(cx, s)
    }
    fn keyboard_event(
        &mut self,
        cx: &mut EventContext,
        e: KeyEvent,
        k: &KeyboardState,
    ) -> Result<InputOutcome, RuntimeError> {
        self.dialog.keyboard_event(cx, e, k)
    }
    fn inspect(&self) -> ComponentDetails {
        self.dialog.inspect()
    }
}
