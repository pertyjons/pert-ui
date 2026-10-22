//! Retained overlapping windows inside an ordinary clipped view.
use crate::layout::{centered, rect};
use crate::*;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

/// Presentation lifetime of one in-view window, independent of application data.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WindowStatus {
    /// The complete window is visible when its owner view is visible.
    Open,
    /// Only its title/drag bar is visible; content state is retained without deadlines.
    Minimized,
    /// Closing was accepted or the owner was removed. This incarnation cannot reopen.
    Closed,
}

/// The committed placement of a [`FloatingWindow`], as its
/// [`WindowController`] and the workspace keep it and
/// [`FloatingWindow::on_frame`] reports it, in the layer's logical pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct WindowPlacement {
    /// The preferred origin in the layer. The layer may clamp the shown
    /// position to keep the drag area reachable without changing this.
    pub position: Point,
    /// The preferred outer size, including decorations, or `None` while the
    /// window is sized by its content.
    pub size: Option<Size>,
}

/// UI-thread controller for one [`FloatingWindow`] incarnation.
/// Clones share state but do not keep the window mounted. Closing is terminal;
/// create a new description to reopen. This is local workspace state, not project
/// persistence, host undo or a worker/callback-safe endpoint. Hidden owners retain writes
/// and apply them on visibility restoration. Methods propagate signal lifecycle errors.
#[derive(Clone)]
#[must_use]
pub struct WindowController {
    status: Signal<WindowStatus>,
    placement: Signal<Option<WindowPlacement>>,
    initial: Rc<Cell<WindowPlacement>>,
    ended: Rc<Cell<bool>>,
    /// The cause of the last placement write, for [`FloatingWindow::on_frame`].
    origin: Rc<Cell<Option<ChangeOrigin>>>,
}
/// Shows the status, or the error reading it after runtime shutdown, and
/// whether this incarnation's owner has ended.
impl std::fmt::Debug for WindowController {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WindowController")
            .field("status", &self.status.get())
            .field("ended", &self.ended.get())
            .finish()
    }
}
impl WindowController {
    fn current_placement(&self) -> Result<WindowPlacement, RuntimeError> {
        Ok(self.placement.get()?.unwrap_or_else(|| self.initial.get()))
    }
    /// Observable open/minimized/closed state. Owner teardown publishes Closed.
    pub fn status(&self) -> ReadSignal<WindowStatus> {
        self.status.read_only()
    }
    /// Accepts closure, bypassing the optional close-request callback. Idempotent.
    /// The next successful visible update removes the component subtree. Hidden
    /// owners remove it when next shown; removing their parent also tears it down.
    ///
    /// # Errors
    /// Returns the errors of [`Signal::set`](crate::Signal::set): `Closed` after shutdown,
    /// or the phase write error when called during update, layout, paint, inspection or
    /// parameter delivery.
    pub fn close(&self) -> Result<bool, RuntimeError> {
        self.status.set_changed(WindowStatus::Closed)
    }
    fn live(&self) -> Result<(), RuntimeError> {
        if self.ended.get() || self.status.get()? == WindowStatus::Closed {
            Err(RuntimeError::Unmounted)
        } else {
            Ok(())
        }
    }
    /// Collapses to the title/drag bar, retaining the full-size preference and content.
    /// An undecorated window without a drag bar can be restored through this handle.
    ///
    /// # Errors
    /// Returns [`RuntimeError::Unmounted`] once the window is closed or its component
    /// removed, `BorrowConflict` for a reentrant read, and otherwise the errors of
    /// [`Signal::set`](crate::Signal::set): `Closed` after shutdown, or the phase write
    /// error when called during update, layout, paint, inspection or parameter delivery.
    pub fn minimize(&self) -> Result<bool, RuntimeError> {
        self.live()?;
        self.status.set_changed(WindowStatus::Minimized)
    }
    /// Restores minimized content with its retained identities and scroll state.
    /// Cancelled active gestures are not resumed.
    ///
    /// # Errors
    /// Returns [`RuntimeError::Unmounted`] once the window is closed or its component
    /// removed, `BorrowConflict` for a reentrant read, and otherwise the errors of
    /// [`Signal::set`](crate::Signal::set): `Closed` after shutdown, or the phase write
    /// error when called during update, layout, paint, inspection or parameter delivery.
    pub fn restore(&self) -> Result<bool, RuntimeError> {
        self.live()?;
        self.status.set_changed(WindowStatus::Open)
    }
    /// Sets a preferred parent-local origin in logical pixels. The layer constrains
    /// actual placement to keep the top-left drag/restore area reachable; shrinking
    /// the layer does not overwrite this preference. Negative finite values are valid.
    ///
    /// # Errors
    /// Returns [`RuntimeError::Unmounted`] once the window is closed or its component
    /// removed, `BorrowConflict` for a reentrant read, and otherwise the errors of
    /// [`Signal::set`](crate::Signal::set): `Closed` after shutdown, or the phase write
    /// error when called during update, layout, paint, inspection or parameter delivery.
    pub fn set_position(&self, position: Point) -> Result<bool, RuntimeError> {
        self.live()?;
        let mut p = self.current_placement()?;
        p.position = position;
        self.place(p, ChangeOrigin::Application)
    }
    /// Writes a committed placement with the cause it reports.
    fn place(
        &self,
        placement: WindowPlacement,
        origin: ChangeOrigin,
    ) -> Result<bool, RuntimeError> {
        let changed = self.placement.set_changed(Some(placement))?;
        if changed {
            self.origin.set(Some(origin));
        }
        Ok(changed)
    }
    /// Sets the preferred outer size in logical pixels, including decorations.
    /// Actual sizing respects window limits and chrome minima. This programmatic
    /// change is allowed even when user resizing is disabled. Zero is a valid
    /// preference; minimum geometry supplies the usable lower bound.
    ///
    /// # Errors
    /// Returns [`RuntimeError::Unmounted`] once the window is closed or its component
    /// removed, `BorrowConflict` for a reentrant read, and otherwise the errors of
    /// [`Signal::set`](crate::Signal::set): `Closed` after shutdown, or the phase write
    /// error when called during update, layout, paint, inspection or parameter delivery.
    pub fn set_size(&self, size: Size) -> Result<bool, RuntimeError> {
        self.live()?;
        let mut p = self.current_placement()?;
        p.size = Some(size);
        self.place(p, ChangeOrigin::Application)
    }
}

/// Result of a user close request. Host callbacks own save/confirmation policy.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WindowCloseAction {
    /// Accept and remove this window's component subtree at the next safe flush.
    Close,
    /// Keep the window open, for example while a host-owned confirmation dialog runs.
    KeepOpen,
}
type CloseRequest = Box<dyn FnMut(&mut EventContext) -> Result<WindowCloseAction, RuntimeError>>;
type FrameHandler =
    Box<dyn FnMut(&mut EventContext, WindowPlacement, ChangeOrigin) -> Result<(), RuntimeError>>;

/// Owned description of a persistent, nonmodal window inside [`WindowLayer`].
///
/// Defaults: decorated, closable, movable and resizable at the bottom-right grip;
/// minimization is opt-in. Size is content-based until an initial/programmatic size
/// or resize establishes a preference. Decorations use the runtime theme and icons.
/// Content is an arbitrary [`Element`] with automatic two-axis overflow scrolling.
/// All views, controls, custom components and owned surfaces use normal lifecycle.
///
/// Title/corner dragging captures the pointer: motion is tentative and primary
/// release commits once. Escape, focus/context loss, hiding, removal, scale changes
/// or locking an active resize cancel it. A programmatic geometry replacement wins.
/// The movable title uses SurfaceHover and the enabled corner grip uses
/// ResizeHandleHover while hovered; neither changes frame geometry.
/// Focus the frame and use Alt+arrows to move eight logical pixels per key event;
/// Alt+Shift+arrows resizes an open, resize-enabled frame. OS repeats step again.
/// Chords that add Control or Super are left to command bindings.
/// These keyboard steps commit individually; they cannot take over a pointer drag.
/// Minimized/locked frames consume resize shortcuts without moving or changing the
/// retained restore size. No gesture enters application parameter edits or undo.
/// A title wider than its bar ends in an ellipsis; the window's inspection name
/// keeps the full title.
///
/// Builders configure initial structure before mounting. Use [`Self::controller`] for
/// programmatic changes and ordinary content bindings afterward. Keys must be
/// nonempty/unique siblings and titles must be nonblank, including undecorated
/// windows. Mount/update validates these rules; text resource errors propagate.
/// A bound title is checked when read: a blank value keeps the last valid
/// title and fails the window's update with [`RuntimeError::EmptyWindowTitle`].
#[must_use]
pub struct FloatingWindow {
    key: String,
    title: TextSource,
    state: WindowController,
    content: Element,
    actions: Vec<Element>,
    drag_area: Option<Element>,
    decorated: bool,
    closable: bool,
    minimizable: bool,
    movable: Permission,
    resizable: bool,
    permission: Permission,
    limits: SizeLimits,
    close_request: Option<CloseRequest>,
    on_frame: Option<FrameHandler>,
    /// The application's button labels, or `None` for the string table's.
    minimize_label: Option<TextSource>,
    close_label: Option<TextSource>,
}
impl FloatingWindow {
    /// Creates one UI-local window with ordinary content and standard decorations.
    /// `ui` supplies bindings only; no window/GPU is opened. Last declared window
    /// starts frontmost in the layer; the first pointer/focus activation raises it.
    ///
    /// `ui` is any [`StateOwner`](crate::StateOwner): the
    /// [`Runtime`](crate::Runtime) while the application builds its tree, or
    /// the [`UpdateContext`](crate::UpdateContext) or
    /// [`EventContext`](crate::EventContext) of a component that builds this
    /// while it describes its children.
    ///
    /// The title is a [`TextSource`]: fixed text, or a binding the title bar,
    /// inspection `title` property and accessible name follow without
    /// describing the window again. Only its plain text is shown. A blank
    /// fixed title is refused when the window is described; a blank bound
    /// title is refused when read, keeping the last valid title.
    pub fn new(
        ui: &impl crate::StateOwner,
        key: impl Into<String>,
        title: impl Into<TextSource>,
        content: Element,
    ) -> Self {
        Self {
            key: key.into(),
            title: title.into(),
            content,
            actions: Vec::new(),
            drag_area: None,
            state: WindowController {
                status: ui.signal(WindowStatus::Open),
                placement: ui.signal(None),
                initial: Rc::new(Cell::new(WindowPlacement {
                    position: Point::default(),
                    size: None,
                })),
                ended: Rc::new(Cell::new(false)),
                origin: Rc::new(Cell::new(None)),
            },
            decorated: true,
            closable: true,
            minimizable: false,
            movable: Permission::allowed(),
            resizable: true,
            permission: Permission::allowed(),
            limits: SizeLimits::default(),
            close_request: None,
            on_frame: None,
            minimize_label: None,
            close_label: None,
        }
    }
    /// Clones this description's controller. Retain it before moving into the layer.
    pub fn controller(&self) -> WindowController {
        self.state.clone()
    }
    /// Sets the initial logical origin.
    ///
    /// # Errors
    /// Returns [`RuntimeError::Unmounted`] once this description's controller
    /// is closed or its window removed, [`RuntimeError::Closed`] after the
    /// runtime shut down and `BorrowConflict` for a reentrant read of its
    /// status. This builder is valid during component update; it writes no
    /// signal. A programmatic controller placement supersedes initial geometry.
    pub fn position(self, position: Point) -> Result<Self, RuntimeError> {
        self.state.live()?;
        let mut initial = self.state.initial.get();
        initial.position = position;
        self.state.initial.set(initial);
        Ok(self)
    }
    /// Sets the initial preferred outer size, including the title bar and grip.
    ///
    /// # Errors
    /// Returns [`RuntimeError::Unmounted`] once this description's controller
    /// is closed or its window removed, [`RuntimeError::Closed`] after the
    /// runtime shut down and `BorrowConflict` for a reentrant read of its
    /// status. Initial geometry writes no signal and is valid during update.
    /// Programmatic controller placement supersedes initial geometry.
    pub fn initial_size(self, size: Size) -> Result<Self, RuntimeError> {
        self.state.live()?;
        let mut initial = self.state.initial.get();
        initial.size = Some(size);
        self.state.initial.set(initial);
        Ok(self)
    }
    /// Sets the outer size and disables user resizing; movement is independent.
    /// Chrome minima still apply; overflow content remains scrollable.
    ///
    /// # Errors
    /// Returns [`RuntimeError::Unmounted`] once this description's controller
    /// is closed or its window removed, [`RuntimeError::Closed`] after the
    /// runtime shut down and `BorrowConflict` for a reentrant read of its
    /// status. Initial geometry writes no signal and is valid during update.
    /// Programmatic controller placement supersedes initial geometry.
    pub fn fixed_size(mut self, size: Size) -> Result<Self, RuntimeError> {
        self.state.live()?;
        let mut initial = self.state.initial.get();
        initial.size = Some(size);
        self.state.initial.set(initial);
        self.resizable = false;
        Ok(self)
    }
    /// Supplies validated outer min/max bounds. Chrome minima may increase the
    /// minimum; an incompatible maximum fails layout with
    /// [`GeometryError::InvertedBounds`](crate::GeometryError::InvertedBounds).
    /// Content minima are handled by the internal scroll viewport.
    pub fn size_limits(mut self, limits: SizeLimits) -> Self {
        self.limits = limits;
        self
    }
    /// Enables/disables the frame and standard title bar. Default true.
    /// Resizing stays independent. Undecorated windows can use [`Self::drag_area`].
    pub fn decorated(mut self, decorated: bool) -> Self {
        self.decorated = decorated;
        self
    }
    /// Enables/disables the standard close button (default true when decorated).
    /// This is a fixed choice of title-bar part, not a [`Permission`]: it does
    /// not compose with `and` or follow a binding, and a later call replaces
    /// it. Programmatic closure remains available through
    /// [`WindowController::close`].
    pub fn closable(mut self, closable: bool) -> Self {
        self.closable = closable;
        self
    }
    /// Adds a minimize/restore title button (default false). Content is hidden,
    /// not unmounted, while minimized; its latest bound data appears on restore.
    /// This is a fixed choice of title-bar part, not a [`Permission`], and a
    /// later call replaces it; [`WindowController::minimize`] works without it.
    pub fn minimizable(mut self, enabled: bool) -> Self {
        self.minimizable = enabled;
        self
    }
    /// Restricts title/drag-bar movement and Alt+arrow moves (allowed by
    /// default), independently of resizing. Takes a fixed `bool` or a bound
    /// `ReadSignal<bool>`, as any [`Permission`]; calls combine restrictively
    /// ([`Permission::and`]), so a later `true` or a true signal does not lift
    /// a fixed denial. Re-describing the window replaces its permission. The
    /// window reads a binding in its update; revoking movement during a move
    /// cancels that gesture and restores the original position, and reports
    /// no frame change. Programmatic placement through the
    /// [`WindowController`] stays available.
    pub fn movable(mut self, movable: impl Into<Permission>) -> Self {
        self.movable = self.movable.and(movable);
        self
    }
    /// Reports each change of the window's committed placement, with its
    /// origin, from an event phase after layout settles, as
    /// [`Component::deliver_notifications`] describes.
    ///
    /// The callback receives the placement after the change:
    /// [`ChangeOrigin::User`] when a title-bar move or a corner resize ends
    /// on release, or for each Alt+arrow step; [`ChangeOrigin::Restore`] when
    /// [`Runtime::restore_workspace`] applies a saved frame; and
    /// [`ChangeOrigin::Application`] for a [`WindowController::set_position`]
    /// or [`WindowController::set_size`] write. Motion during a gesture is a
    /// preview and is never reported, nor is a gesture cancelled by Escape,
    /// by revoking [`Self::movable`] or [`Self::resizable`], or by losing
    /// input. Changes within one flush are delivered once, with the last
    /// placement and the origin of the last write; a write that changes
    /// nothing calls nothing.
    ///
    /// A new description replaces the callback, including removing it, and a
    /// description without one discards a change not yet delivered. The
    /// callback runs on the UI thread and may write bindings; its error is
    /// returned from the flush, the placement stays and nothing is replayed.
    ///
    /// ```
    /// # use pert_ui_runtime as pert_ui;
    /// use pert_ui::{
    ///     ChangeOrigin, Element, FloatingWindow, Label, Point, Runtime, WindowLayer,
    /// };
    /// use std::{cell::RefCell, rc::Rc, time::Duration};
    ///
    /// let mut ui = Runtime::new();
    /// let reports = Rc::new(RefCell::new(Vec::new()));
    /// let log = reports.clone();
    /// let window = FloatingWindow::new(&ui, "mixer", "Mixer", Element::new("body", Label::new("Kanaler")))
    ///     .on_frame(move |_, placement, origin| {
    ///         log.borrow_mut().push((placement.position, origin));
    ///         Ok(())
    ///     });
    /// let controller = window.controller();
    /// ui.mount_tree(Element::new("layer", WindowLayer::new([window])))?;
    /// ui.flush(Duration::ZERO)?;
    /// controller.set_position(Point::new(40.0, 24.0)?)?;
    /// ui.flush(Duration::ZERO)?;
    /// assert_eq!(*reports.borrow(), [(Point::new(40.0, 24.0)?, ChangeOrigin::Application)]);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn on_frame(
        mut self,
        on_frame: impl FnMut(
            &mut EventContext,
            WindowPlacement,
            ChangeOrigin,
        ) -> Result<(), RuntimeError>
        + 'static,
    ) -> Self {
        self.on_frame = Some(Box::new(on_frame));
        self
    }
    /// Restricts user resizing (allowed by default), independently of
    /// decoration and movement. Takes a fixed `bool` or a bound
    /// `ReadSignal<bool>`, as any [`Permission`]. Calls combine restrictively
    /// ([`Permission::and`]): a later `true` or a true signal does not lift a
    /// fixed denial or the denial set by [`Self::fixed_size`]. Revoking the
    /// permission during a resize cancels that gesture and restores its
    /// original size without recreating content.
    pub fn resizable(mut self, enabled: impl Into<Permission>) -> Self {
        self.permission = self.permission.and(enabled);
        self
    }
    /// Adds ordinary interactive title-bar content before the standard buttons.
    /// Use Button::frameless or Menu::icon for icons directly on the title bar,
    /// with a readable action name. Child input
    /// is delivered normally and this area never starts a window drag.
    /// Ignored when undecorated; keys are scoped to the action row.
    pub fn title_action(mut self, action: Element) -> Self {
        self.actions.push(action);
        self
    }
    /// Supplies a custom top drag bar, including for undecorated windows. Child
    /// controls consume their own input; ignored primary presses drag the window.
    /// The bar remains visible while minimized. The title remains its inspection name.
    pub fn drag_area(mut self, content: Element) -> Self {
        self.drag_area = Some(content);
        self
    }
    /// Replaces default close-button acceptance with host policy. Called once per
    /// completed activation on the UI thread. Errors propagate and keep the window
    /// open; the action is never replayed. Return KeepOpen while asking the user,
    /// then call WindowController::close after confirmation. No implicit save/undo occurs.
    pub fn on_close(
        mut self,
        action: impl FnMut(&mut EventContext) -> Result<WindowCloseAction, RuntimeError> + 'static,
    ) -> Self {
        self.close_request = Some(Box::new(action));
        self
    }
    /// Replaces the accessible name and help of the minimize/restore title
    /// button. By default the installed [`UiStrings`](crate::UiStrings) supply
    /// [`StringRole::WindowMinimize`](crate::StringRole::WindowMinimize),
    /// `"Minimize or restore"` in English, and the button follows a replaced
    /// table. Only shown when [`Self::minimizable`]; re-describing the window
    /// applies a changed label. The label is a [`TextSource`], so a binding
    /// keeps it current; only its plain text is used.
    pub fn minimize_label(mut self, label: impl Into<TextSource>) -> Self {
        self.minimize_label = Some(label.into());
        self
    }
    /// Replaces the accessible name and help of the close title button. By
    /// default the installed [`UiStrings`](crate::UiStrings) supply
    /// [`StringRole::WindowClose`](crate::StringRole::WindowClose),
    /// `"Close window"` in English. Only
    /// shown when [`Self::closable`]; re-describing the window applies a changed
    /// label. The label is a [`TextSource`], so a binding keeps it current;
    /// only its plain text is used.
    ///
    /// ```
    /// # use pert_ui_runtime as pert_ui;
    /// use pert_ui::{Element, FloatingWindow, Label, Runtime, WindowLayer};
    /// use std::time::Duration;
    /// let mut ui = Runtime::new();
    /// let window = FloatingWindow::new(&ui, "mixer", "Mixer", Element::new("body", Label::new("Kanaler")))
    ///     .minimizable(true)
    ///     .minimize_label("Minimera eller återställ")
    ///     .close_label("Stäng fönstret");
    /// ui.mount_tree(Element::new("layer", WindowLayer::new([window])))?;
    /// ui.flush(Duration::ZERO)?;
    /// let names: Vec<_> = ui.inspect().components.into_iter()
    ///     .filter(|c| c.key == "button")
    ///     .filter_map(|c| c.details.properties.get("text").cloned())
    ///     .collect();
    /// assert!(names.contains(&pert_ui::InspectValue::Text("Stäng fönstret".into())));
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn close_label(mut self, label: impl Into<TextSource>) -> Self {
        self.close_label = Some(label.into());
        self
    }
    /// Converts to an ordinary element for mounting or keyed reconciliation of a
    /// WindowLayer.
    ///
    /// Describing a mounted window again, under the same key, applies the new
    /// title, decoration, drag area, title actions, button labels, close policy,
    /// movement and resize settings and size limits to it; content reconciles by
    /// key. The mounted window keeps its original controller, placement and
    /// status, so the new description's position and size are not applied: keep
    /// the original controller and use it for programmatic changes. The new
    /// description's own controller is never attached and reports
    /// [`WindowStatus::Closed`] once the description is dropped.
    pub fn into_element(mut self) -> Element {
        let mut children = vec![
            Element::new(
                "content",
                ScrollArea::new(self.content).axes(ScrollAxes::Both),
            )
            .part("content"),
        ];
        let has_bar = self.decorated || self.drag_area.is_some();
        if has_bar {
            children.push(
                Element::new(
                    "title",
                    View::new(self.drag_area.take().unwrap_or_else(|| {
                        Element::new(
                            "label",
                            WindowTitle {
                                text: Caption::new(self.title.clone()),
                                prepared: RefCell::new(None),
                                shown: RefCell::new(None),
                                size: Cell::new(Size::ZERO),
                            },
                        )
                        .structural()
                    })),
                )
                .structural(),
            );
        }
        // The title buttons act on whichever controller the mounted frame keeps;
        // reconciliation points this at it.
        let chrome = Rc::new(RefCell::new(self.state.clone()));
        if self.decorated {
            let mut actions = if self.actions.is_empty() {
                Vec::new()
            } else {
                vec![
                    Element::new(
                        "custom",
                        Row::new(std::mem::take(&mut self.actions)).gap(Length::from(0)),
                    )
                    .structural(),
                ]
            };
            if self.minimizable {
                let chrome = chrome.clone();
                let label = crate::runtime::settings::library_text(
                    self.state.status.shared(),
                    self.minimize_label.clone(),
                    crate::StringRole::WindowMinimize,
                );
                actions.push(Element::new(
                    "window-minimize",
                    named_tooltip(
                        Button::icon(Icon::Remove, label.clone(), move |_| {
                            let state = chrome.borrow().clone();
                            if state.status.get()? == WindowStatus::Minimized {
                                state.restore()?;
                            } else {
                                state.minimize()?;
                            }
                            Ok(())
                        })
                        .frameless(),
                        label,
                    ),
                ));
            }
            if self.closable {
                let chrome = chrome.clone();
                let mut action = self.close_request.take();
                let label = crate::runtime::settings::library_text(
                    self.state.status.shared(),
                    self.close_label.clone(),
                    crate::StringRole::WindowClose,
                );
                actions.push(Element::new(
                    "window-close",
                    named_tooltip(
                        Button::icon(Icon::Close, label.clone(), move |cx| {
                            let decision = match &mut action {
                                Some(action) => action(cx)?,
                                None => WindowCloseAction::Close,
                            };
                            if decision == WindowCloseAction::Close {
                                let state = chrome.borrow().clone();
                                state.close()?;
                            }
                            Ok(())
                        })
                        .frameless(),
                        label,
                    ),
                ));
            }
            if !actions.is_empty() {
                children.push(
                    Element::new("actions", Row::new(actions).gap(Length::from(0))).structural(),
                );
            }
        }
        let label = self.title.fixed().map(RichText::plain_text);
        let element = Element::new(
            self.key,
            WindowFrame {
                title: Caption::new(self.title),
                state: self.state,
                chrome,
                children,
                decorated: self.decorated,
                has_bar,
                movable: true,
                move_permission: self.movable,
                resizable: self.resizable,
                permission: Permission::from(self.resizable).and(self.permission),
                limits: self.limits,
                on_frame: self.on_frame,
                reported: None,
                pending_frame: None,
                notifier: None,
                placement: WindowPlacement {
                    position: Point::default(),
                    size: None,
                },
                status: WindowStatus::Open,
                geometry: Cell::new(None),
                drag: None,
                hover: None,
            },
        );
        // A bound title names the window through its semantics once read.
        match label {
            Some(label) => element.with_label(label),
            None => element,
        }
    }
}
impl From<FloatingWindow> for Element {
    /// Converts as [`FloatingWindow::into_element`] does, so a list of floating windows can be
    /// collected into the `Vec<Element>` that
    /// [`Runtime::reconcile_children`](crate::Runtime::reconcile_children)
    /// takes with `.map(Element::from)` or `.map(Into::into)`.
    fn from(window: FloatingWindow) -> Self {
        window.into_element()
    }
}

/// A clipped, nonmodal collection of independently positioned overlapping windows.
///
/// Fills finite available space; unbounded axes prefer 640×400 logical pixels.
/// Last declared child starts frontmost. Pointer/focus activation raises a window
/// without changing logical child order or identity. Tab traversal remains in
/// logical order and raises covered recipients; ordinary menus/dialogs retain their
/// existing ownership, dismissal and modality. This layer introduces no event loop.
///
/// Place in a View, tab, split or RegionLayout. Hiding its ancestor suspends all
/// descendants. Windows may exceed this viewport; their origin is constrained so
/// at least the smaller of 48 pixels, the window extent and the layer extent
/// of the top-left corner stays reachable. Preferences
/// survive temporary viewport clamping. Partially overlapped windows remain active;
/// this does not implement opaque-region occlusion culling.
///
/// Re-describing the layer reconciles its windows by key; each window's
/// placement and status stay in its controller.
#[must_use]
pub struct WindowLayer {
    children: Vec<Element>,
}
impl WindowLayer {
    /// Creates a layer with default presentation from window descriptions.
    pub fn new(windows: impl IntoIterator<Item = FloatingWindow>) -> Self {
        Self {
            children: windows
                .into_iter()
                .map(FloatingWindow::into_element)
                .collect(),
        }
    }
}
impl Component for WindowLayer {
    fn stacks_children(&self) -> bool {
        true
    }
    fn take_children(&mut self) -> Vec<Element> {
        std::mem::take(&mut self.children)
    }
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
    fn keyboard_event(
        &mut self,
        _: &mut EventContext,
        _: KeyEvent,
        _: &KeyboardState,
    ) -> Result<InputOutcome, RuntimeError> {
        Ok(InputOutcome::Ignored)
    }
    fn measure(
        &self,
        _: &MeasureContext<'_>,
        available: Constraints,
    ) -> Result<Size, RuntimeError> {
        Ok(Size::new(
            available.width().unwrap_or(640.0),
            available.height().unwrap_or(400.0),
        )?)
    }
    fn arrange(&self, cx: &mut ArrangeContext<'_>, size: Size) -> Result<(), RuntimeError> {
        for index in 0..cx.len() {
            let preferred = cx.measurement().position(index)?;
            let child = cx.measure(index, Constraints::bounded(size))?;
            let x = preferred
                .x()
                .clamp(0.0, (size.width() - child.width().min(48.0)).max(0.0));
            let y = preferred
                .y()
                .clamp(0.0, (size.height() - child.height().min(48.0)).max(0.0));
            cx.place(index, rect(x, y, child.width(), child.height())?)?;
        }
        Ok(())
    }
}

#[derive(Clone, Copy)]
struct Geometry {
    size: Size,
    title: Rect,
    grip: Rect,
    limits: SizeLimits,
}
#[derive(Clone, Copy)]
struct Drag {
    original: WindowPlacement,
    last: WindowPlacement,
    pointer: Point,
    origin: Point,
    size: Size,
    resize: bool,
}
struct WindowFrame {
    title: Caption,
    state: WindowController,
    /// The controller the title buttons of this description act on.
    chrome: Rc<RefCell<WindowController>>,
    children: Vec<Element>,
    decorated: bool,
    has_bar: bool,
    /// Whether moving is allowed now, read from `move_permission` at update.
    movable: bool,
    move_permission: Permission,
    resizable: bool,
    permission: Permission,
    limits: SizeLimits,
    on_frame: Option<FrameHandler>,
    /// The committed placement last seen by the update.
    reported: Option<WindowPlacement>,
    /// A committed change not yet delivered to `on_frame`.
    pending_frame: Option<(WindowPlacement, ChangeOrigin)>,
    notifier: Option<Notifier>,
    placement: WindowPlacement,
    status: WindowStatus,
    geometry: Cell<Option<Geometry>>,
    drag: Option<Drag>,
    hover: Option<bool>,
}
fn bounded(value: f64) -> f32 {
    value.clamp(0.0, f64::from(f32::MAX)) as f32
}
impl WindowFrame {
    fn metrics(&self, cx: &MeasureContext<'_>) -> Result<(f32, f32, f32, Size), RuntimeError> {
        let border = if self.decorated {
            cx.metric(MetricRole::BorderWidth)
        } else {
            0.0
        };
        let mut header = if self.has_bar {
            cx.metric(MetricRole::ControlHeight)
        } else {
            0.0
        };
        let mut actions = Size::ZERO;
        for i in 0..cx.len() {
            if cx.key(i)? == "actions" {
                actions = cx.measure(i, Constraints::default())?;
                header = header.max(actions.height());
            }
            if cx.key(i)? == "title" {
                header = header.max(
                    cx.measure(i, Constraints::new(Some(160.0), None)?)?
                        .height(),
                );
            }
        }
        let grip = if self.resizable && self.status == WindowStatus::Open {
            cx.metric(MetricRole::ResizeWidth).max(12.0)
        } else {
            0.0
        };
        // Use whole logical-pixel chrome extents so adjacent translated title/body
        // boundaries do not acquire fractional overlap from f32 addition.
        Ok((border.ceil(), header.ceil(), grip.ceil(), actions))
    }
    fn cancel(&mut self, cx: &mut EventContext) {
        if let Some(drag) = self.drag.take() {
            // A concurrent programmatic replacement wins over gesture rollback.
            if self.state.current_placement().ok() == Some(drag.original) {
                self.placement = drag.original;
            }
            cx.release_pointer();
            cx.invalidate(Invalidation::Layout);
        }
    }
    fn begin(
        &mut self,
        cx: &mut EventContext,
        event: PointerEvent,
        resize: bool,
    ) -> Result<(), RuntimeError> {
        let Some(size) = cx.size() else {
            return Ok(());
        };
        cx.capture_pointer()?;
        let placement = self.state.current_placement()?;
        self.drag = Some(Drag {
            original: placement,
            last: placement,
            pointer: event.position(),
            origin: cx.position_in_parent().unwrap_or_default(),
            size,
            resize,
        });
        Ok(())
    }
    fn motion(&mut self, cx: &mut EventContext, event: PointerEvent) -> Result<(), RuntimeError> {
        let Some(mut drag) = self.drag else {
            return Ok(());
        };
        if self.state.current_placement()? != drag.original {
            self.drag = None;
            cx.release_pointer();
            return Ok(());
        }
        // The pointer moves in the window; the placement is in the layer's content.
        let moved = cx.to_local_delta(Point::new(
            event.position().x() - drag.pointer.x(),
            event.position().y() - drag.pointer.y(),
        )?);
        let (dx, dy) = (f64::from(moved.x()), f64::from(moved.y()));
        if moved == Point::ZERO {
            if self.placement != drag.original {
                self.placement = drag.original;
                cx.invalidate(Invalidation::Layout);
            }
            drag.last = drag.original;
            self.drag = Some(drag);
            return Ok(());
        }
        let mut next = drag.last;
        if drag.resize {
            if let Some(g) = self.geometry.get() {
                next.size = Some(g.limits.constrain(Size::new(
                    bounded(f64::from(drag.size.width()) + dx),
                    bounded(f64::from(drag.size.height()) + dy),
                )?));
            }
        } else {
            let (w, h) = cx
                .parent_size()
                .map_or((f32::MAX, f32::MAX), |size| (size.width(), size.height()));
            next.position = Point::new(
                bounded(f64::from(drag.origin.x()) + dx)
                    .min((w - drag.size.width().min(48.0)).max(0.0)),
                bounded(f64::from(drag.origin.y()) + dy)
                    .min((h - drag.size.height().min(48.0)).max(0.0)),
            )?;
        }
        if self.placement != next {
            self.placement = next;
            cx.invalidate(Invalidation::Layout);
        }
        drag.last = next;
        self.drag = Some(drag);
        Ok(())
    }
}
impl Drop for WindowFrame {
    fn drop(&mut self) {
        self.state.ended.set(true);
        self.state.status.publish_copy(WindowStatus::Closed);
    }
}
impl Component for WindowFrame {
    fn check_description(&self) -> Result<(), RuntimeError> {
        // A bound title is checked when its update reads it.
        if !self.title.source().is_bound() && self.title.text().trim().is_empty() {
            return Err(RuntimeError::EmptyWindowTitle);
        }
        Ok(())
    }
    fn reconcile(&mut self, mut next: Self) -> Option<Invalidation> {
        // The new title buttons were adopted by the reconciled children; point
        // them at the controller this mounted window keeps.
        *next.chrome.borrow_mut() = self.state.clone();
        self.chrome = next.chrome.clone();
        let layout = self.decorated != next.decorated
            || self.has_bar != next.has_bar
            || !self.move_permission.same_as(&next.move_permission)
            || !self.permission.same_as(&next.permission)
            || self.limits != next.limits;
        // The frame does not draw the title, but its name follows it.
        let title = std::mem::replace(&mut next.title, Caption::new(""));
        let layout = self.title.reconcile(title) | layout;
        self.decorated = next.decorated;
        self.has_bar = next.has_bar;
        self.move_permission = next.move_permission.clone();
        if let Some(movable) = self.move_permission.fixed() {
            self.movable = movable;
        }
        self.permission = next.permission.clone();
        self.limits = next.limits;
        // The callback is replaced, including removed; without one, a change
        // not yet delivered is discarded.
        self.on_frame = next.on_frame.take();
        if self.on_frame.is_none() {
            self.pending_frame = None;
        }
        layout.then_some(Invalidation::Layout)
    }
    fn workspace_state(&self) -> Result<Option<crate::ViewState>, crate::WorkspaceError> {
        let placement = self.state.current_placement()?;
        let minimized = self.state.status.get()? == WindowStatus::Minimized;
        let mut state = crate::ViewState::new("floating-window")?
            .with(
                "x",
                crate::WorkspaceValue::number(f64::from(placement.position.x()))?,
            )?
            .with(
                "y",
                crate::WorkspaceValue::number(f64::from(placement.position.y()))?,
            )?
            .with("minimized", crate::WorkspaceValue::flag(minimized))?;
        // A window that never left its described size has none of its own.
        if let Some(size) = placement.size {
            state = state
                .with(
                    "width",
                    crate::WorkspaceValue::number(f64::from(size.width()))?,
                )?
                .with(
                    "height",
                    crate::WorkspaceValue::number(f64::from(size.height()))?,
                )?;
        }
        Ok(Some(state))
    }
    fn restore_workspace_state(
        &mut self,
        state: &crate::ViewState,
    ) -> Result<Invalidation, crate::RestoreRejection> {
        use crate::RestoreRejection;
        state.expect_only(&["x", "y", "width", "height", "minimized"])?;
        if self.drag.is_some() {
            return Err(RestoreRejection::Busy);
        }
        // A closed window or removed component no longer exists; any other
        // refusal is the runtime's and keeps its cause.
        let refused = |error: RuntimeError| match error {
            RuntimeError::Unmounted => RestoreRejection::Unresolved("window".to_owned()),
            error => RestoreRejection::Runtime(error),
        };
        self.state.live().map_err(refused)?;
        let extent = |name: &str| -> Result<f32, RestoreRejection> {
            let number = state.number(name)?;
            let value = number as f32;
            if value.is_finite() && (!matches!(name, "width" | "height") || number >= 0.0) {
                Ok(value)
            } else {
                Err(RestoreRejection::OutOfRange(name.to_owned()))
            }
        };
        let position = Point::new(extent("x")?, extent("y")?)
            .map_err(|_| RestoreRejection::OutOfRange("x".to_owned()))?;
        let size = match (state.get("width"), state.get("height")) {
            (None, None) => None,
            (Some(_), Some(_)) => Some(
                Size::new(extent("width")?, extent("height")?)
                    .map_err(|_| RestoreRejection::OutOfRange("width".to_owned()))?,
            ),
            // Half a size is not a size.
            _ => return Err(RestoreRejection::Value("width".to_owned())),
        };
        let minimized = state.flag("minimized")?;
        let previous = self.state.current_placement().map_err(refused)?;
        self.state
            .place(WindowPlacement { position, size }, ChangeOrigin::Restore)
            .map_err(refused)?;
        let status = if minimized {
            self.state.minimize()
        } else {
            self.state.restore()
        };
        if let Err(error) = status {
            // Nothing is applied in part: the placement returns as well. The
            // status write's cause is the one reported.
            let _ = self.state.placement.set(Some(previous));
            return Err(refused(error));
        }
        Ok(Invalidation::Layout)
    }
    fn layout_position(&self) -> Point {
        self.placement.position
    }
    fn take_children(&mut self) -> Vec<Element> {
        std::mem::take(&mut self.children)
    }
    fn focusable(&self) -> bool {
        self.status != WindowStatus::Closed
    }
    fn child_visible(&self, key: &str) -> bool {
        self.status == WindowStatus::Open || key != "content"
    }
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        if self.title.source().is_bound() {
            // The frame names the window; its bar child draws the title.
            self.title
                .update_nonblank(cx, || RuntimeError::EmptyWindowTitle)?;
        } else if self.title.text().trim().is_empty() {
            return Err(RuntimeError::EmptyWindowTitle);
        }
        let status = cx.read(&self.state.status())?;
        let placement = cx
            .read(&self.state.placement)?
            .unwrap_or_else(|| self.state.initial.get());
        let resizable = self.permission.read(cx)?;
        let movable = self.move_permission.read(cx)?;
        if self.drag.is_some_and(|d| {
            d.original != placement
                || (d.resize && !resizable)
                || (!d.resize && (!movable || !self.has_bar))
        }) || status != self.status
        {
            // A cancelled gesture restores its original placement below and
            // commits nothing.
            self.drag = None;
            cx.cancel_input();
        }
        self.resizable = resizable;
        self.movable = movable;
        let notifier = self.notifier.get_or_insert_with(|| cx.notifier());
        let origin = self.state.origin.take();
        if self.reported.is_some_and(|reported| reported != placement) && self.on_frame.is_some() {
            self.pending_frame = Some((placement, origin.unwrap_or(ChangeOrigin::Application)));
            notifier.request();
        }
        self.reported = Some(placement);
        self.status = status;
        if self.drag.is_none() {
            self.placement = placement;
        }
        if status == WindowStatus::Closed {
            cx.request_unmount();
        }
        Ok(())
    }
    fn measure_limits(&self, cx: &MeasureContext<'_>) -> Result<SizeLimits, RuntimeError> {
        let (border, header, grip, actions) = self.metrics(cx)?;
        let mut title_minimum: f32 = if self.has_bar { 48.0 } else { grip };
        for index in 0..cx.len() {
            if cx.key(index)? == "title" {
                title_minimum = title_minimum.max(cx.limits(index)?.minimum().width());
            }
        }
        let gap = if self.has_bar {
            cx.metric(MetricRole::ItemGap)
        } else {
            0.0
        };
        let minimum = Size::new(
            self.limits
                .minimum()
                .width()
                .max(actions.width() + 2.0 * border + title_minimum + gap),
            if self.status == WindowStatus::Minimized {
                header + 2.0 * border
            } else {
                self.limits
                    .minimum()
                    .height()
                    .max(header + grip + 2.0 * border + 24.0)
            },
        )?;
        let maximum = if self.status == WindowStatus::Minimized {
            Constraints::new(self.limits.maximum().width(), Some(minimum.height()))?
        } else {
            self.limits.maximum()
        };
        Ok(SizeLimits::new(minimum, maximum)?)
    }
    fn measure(
        &self,
        cx: &MeasureContext<'_>,
        available: Constraints,
    ) -> Result<Size, RuntimeError> {
        let (border, header, grip, actions) = self.metrics(cx)?;
        let limits = self.measure_limits(cx)?;
        if let Some(size) = self.placement.size {
            return Ok(limits.constrain(size));
        }
        let mut content = Size::ZERO;
        for i in 0..cx.len() {
            if cx.key(i)? == "content" {
                content = cx.measure(
                    i,
                    Constraints::new(
                        available.width().map(|w| (w - 2.0 * border).max(0.0)),
                        available
                            .height()
                            .map(|h| (h - header - grip - 2.0 * border).max(0.0)),
                    )?,
                )?;
            }
        }
        Ok(limits.constrain(Size::new(
            (content.width() + 2.0 * border).max(actions.width() + 160.0),
            content.height() + header + grip + 2.0 * border,
        )?))
    }
    fn arrange(&self, cx: &mut ArrangeContext<'_>, size: Size) -> Result<(), RuntimeError> {
        let (border, header, grip, actions) = self.metrics(cx.measurement())?;
        let gap = if self.has_bar {
            cx.measurement().metric(MetricRole::ItemGap)
        } else {
            0.0
        };
        let title_width = (size.width() - 2.0 * border - actions.width() - gap).max(0.0);
        let title = rect(border, border, title_width, header)?;
        let resize = rect(
            (size.width() - grip - border).max(0.0),
            (size.height() - grip - border).max(0.0),
            grip,
            grip,
        )?;
        self.geometry.set(Some(Geometry {
            size,
            title,
            grip: resize,
            limits: self.measure_limits(cx.measurement())?,
        }));
        for i in 0..cx.len() {
            let area = match cx.measurement().key(i)? {
                "content" => rect(
                    border,
                    border + header,
                    (size.width() - 2.0 * border).max(0.0),
                    (size.height() - 2.0 * border - header - grip).max(0.0),
                )?,
                "title" => title,
                "actions" => rect(
                    size.width() - border - actions.width(),
                    border,
                    actions.width(),
                    header,
                )?,
                _ => return Err(LayoutError::InvalidChild.into()),
            };
            cx.place_clipped(i, area, area)?;
        }
        Ok(())
    }
    fn paint(&self, p: &mut Painter) -> Result<(), DrawError> {
        let Some(g) = self.geometry.get() else {
            return Ok(());
        };
        if self.decorated {
            p.fill_shape(
                Rect::from_size(g.size),
                Shape::Rounded(Length::new(p.metric(MetricRole::CornerRadius))?),
                p.color(ColorRole::Surface),
            )?;
            p.fill_shape(
                Rect::new(0.0, 0.0, g.size.width(), g.title.y() + g.title.height())?,
                Shape::RoundedTop(Length::new(p.metric(MetricRole::CornerRadius))?),
                p.color(ColorRole::SurfaceContainer),
            )?;
            if self.hover == Some(false) {
                p.fill_rect(g.title, p.color(ColorRole::SurfaceHover));
            }
        }
        if self.hover == Some(true) {
            p.fill_shape(
                g.grip,
                Shape::Rounded(Length::new(p.metric(MetricRole::CornerRadius))?),
                p.color(ColorRole::ResizeHandleHover),
            )?;
        }
        Ok(())
    }
    fn paint_foreground(&self, p: &mut Painter) -> Result<(), DrawError> {
        let Some(g) = self.geometry.get() else {
            return Ok(());
        };
        if self.decorated {
            p.stroke_shape(
                Rect::from_size(g.size),
                Shape::Rounded(Length::new(p.metric(MetricRole::CornerRadius))?),
                Length::new(p.metric(MetricRole::BorderWidth))?,
                p.color(if p.focus_within() {
                    ColorRole::Primary
                } else {
                    ColorRole::PanelBorder
                }),
            )?;
        }
        if self.resizable && self.status == WindowStatus::Open {
            crate::resize_grip::paint_corner_triangle(p, g.grip)?;
        }
        Ok(())
    }
    fn cursor(&self, point: Point) -> Option<CursorIcon> {
        let g = self.geometry.get()?;
        if self.drag.is_some_and(|d| d.resize) || (self.resizable && g.grip.contains(point)) {
            Some(CursorIcon::ResizeDiagonal)
        } else if self.drag.is_some() || (self.movable && self.has_bar && g.title.contains(point)) {
            Some(CursorIcon::Move)
        } else {
            None
        }
    }
    fn pointer_input(
        &mut self,
        cx: &mut EventContext,
        event: PointerEvent,
    ) -> Result<InputOutcome, RuntimeError> {
        if self.drag.is_some_and(|drag| {
            if drag.resize {
                !self.permission.get().unwrap_or(false)
            } else {
                !self.move_permission.get().unwrap_or(false)
            }
        }) {
            self.cancel(cx);
            return Ok(InputOutcome::Handled);
        }
        let Some(g) = self.geometry.get() else {
            return Ok(InputOutcome::Ignored);
        };
        let local = cx
            .to_local(event.position())
            .ok_or(crate::GeometryError::NonFinite)?;
        if matches!(event.kind(), PointerKind::Move | PointerKind::Leave) {
            let hover = if event.kind() == PointerKind::Leave || event.is_outside() {
                None
            } else if self.resizable && self.status == WindowStatus::Open && g.grip.contains(local)
            {
                Some(true)
            } else if self.movable && self.has_bar && g.title.contains(local) {
                Some(false)
            } else {
                None
            };
            if self.hover != hover {
                self.hover = hover;
                cx.invalidate(Invalidation::Paint);
            }
        }
        match event.kind() {
            PointerKind::Press(PointerButton::Primary)
                if self.drag.is_none() && !event.is_outside() =>
            {
                let resize =
                    self.resizable && self.status == WindowStatus::Open && g.grip.contains(local);
                let moving = self.movable && self.has_bar && g.title.contains(local);
                if resize || moving {
                    cx.request_focus();
                    self.begin(cx, event, resize)?;
                    return Ok(InputOutcome::Handled);
                }
            }
            PointerKind::Move if self.drag.is_some() && !event.is_outside() => {
                self.motion(cx, event)?;
                return Ok(InputOutcome::Handled);
            }
            PointerKind::Release(PointerButton::Primary) if self.drag.is_some() => {
                if !event.is_outside() {
                    self.motion(cx, event)?;
                }
                if self.drag.is_some() {
                    self.state.place(self.placement, ChangeOrigin::User)?;
                }
                self.drag = None;
                cx.release_pointer();
                return Ok(InputOutcome::Handled);
            }
            _ => {}
        }
        Ok(InputOutcome::Ignored)
    }
    fn hover_changed(&mut self, cx: &mut EventContext, hovered: bool) {
        if !hovered && self.hover.take().is_some() {
            cx.invalidate(Invalidation::Paint);
        }
    }
    fn keyboard_event(
        &mut self,
        cx: &mut EventContext,
        event: KeyEvent,
        keyboard: &KeyboardState,
    ) -> Result<InputOutcome, RuntimeError> {
        let control = keyboard.control_key(event);
        if control == KeyCode::Escape
            && event.transition == KeyTransition::Pressed
            && self.drag.is_some()
        {
            self.cancel(cx);
            return Ok(InputOutcome::Handled);
        }
        // Focus the frame itself, then Alt+arrows moves and Alt+Shift+arrows resizes.
        if self.drag.is_none()
            && cx.focused()
            && keyboard.modifiers().alt
            // Only Alt, optionally with Shift to resize; other chords belong
            // to command bindings.
            && !keyboard.modifiers().control
            && !keyboard.modifiers().super_key
            && matches!(
                event.transition,
                KeyTransition::Pressed | KeyTransition::Repeated
            )
        {
            let delta = match control {
                KeyCode::ArrowLeft => (-8.0, 0.0),
                KeyCode::ArrowRight => (8.0, 0.0),
                KeyCode::ArrowUp => (0.0, -8.0),
                KeyCode::ArrowDown => (0.0, 8.0),
                _ => return Ok(InputOutcome::Ignored),
            };
            if let Some(g) = self.geometry.get() {
                if keyboard.modifiers().shift {
                    if !self.resizable || self.status != WindowStatus::Open {
                        return Ok(InputOutcome::Handled);
                    }
                    self.state.live()?;
                    let mut placement = self.state.current_placement()?;
                    placement.size = Some(g.limits.constrain(Size::new(
                        (g.size.width() + delta.0).max(0.0),
                        (g.size.height() + delta.1).max(0.0),
                    )?));
                    self.state.place(placement, ChangeOrigin::User)?;
                } else if self.movable
                    && let (Some(origin), Some(parent)) =
                        (cx.position_in_parent(), cx.parent_size())
                {
                    let (w, h) = (parent.width(), parent.height());
                    self.state.live()?;
                    let mut placement = self.state.current_placement()?;
                    placement.position = Point::new(
                        (origin.x() + delta.0).clamp(0.0, (w - g.size.width().min(48.0)).max(0.0)),
                        (origin.y() + delta.1).clamp(0.0, (h - g.size.height().min(48.0)).max(0.0)),
                    )?;
                    self.state.place(placement, ChangeOrigin::User)?;
                }
                return Ok(InputOutcome::Handled);
            }
        }
        Ok(InputOutcome::Ignored)
    }
    fn cancel_input(&mut self, cx: &mut EventContext) {
        self.cancel(cx);
    }
    fn deliver_notifications(&mut self, cx: &mut EventContext) -> Result<(), RuntimeError> {
        if let (Some(on_frame), Some((placement, origin))) =
            (&mut self.on_frame, self.pending_frame.take())
        {
            on_frame(cx, placement, origin)?;
        }
        Ok(())
    }
    fn inspect(&self) -> ComponentDetails {
        let mut d = ComponentDetails::default();
        d.properties.insert(
            "title".into(),
            InspectValue::Text(self.title.text().to_owned()),
        );
        d.properties.insert(
            "window_status".into(),
            InspectValue::Text(format!("{:?}", self.status)),
        );
        d.properties
            .insert("decorated".into(), InspectValue::Bool(self.decorated));
        d.properties
            .insert("resizable".into(), InspectValue::Bool(self.resizable));
        d.properties
            .insert("movable".into(), InspectValue::Bool(self.movable));
        d.semantics = Some(
            Semantics::new(SemanticRole::Window)
                .name(self.title.text())
                .actions(self.movable.then_some(SemanticAction::Move))
                .actions(self.resizable.then_some(SemanticAction::Resize)),
        );
        d
    }
}

// A one-line title ends in an ellipsis instead of growing chrome when
// narrowed; the window's semantics keep the full title. Uses the same public
// text and local drawing contract as an external component.
struct WindowTitle {
    text: Caption,
    prepared: RefCell<Option<TextLayout>>,
    /// The shown text while the title is truncated.
    shown: RefCell<Option<String>>,
    size: Cell<Size>,
}
impl Component for WindowTitle {
    fn reconcile(&mut self, next: Self) -> Option<Invalidation> {
        self.text
            .reconcile(next.text)
            .then_some(Invalidation::Layout)
    }
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        // The frame reports a blank title; the bar keeps the last valid one.
        match self
            .text
            .update_nonblank(cx, || RuntimeError::EmptyWindowTitle)
        {
            Ok(_) | Err(RuntimeError::EmptyWindowTitle) => Ok(()),
            Err(error) => Err(error),
        }
    }
    fn measure(&self, cx: &MeasureContext<'_>, a: Constraints) -> Result<Size, RuntimeError> {
        let padding = 2.0 * cx.metric(MetricRole::ControlPaddingX);
        let line = a
            .width()
            .map_or(f32::MAX, |width| (width - padding).max(0.0));
        let fitted = cx.fitted_text(
            self.text.text(),
            line,
            TextStyle::Body,
            TextOverflow::Ellipsis,
        )?;
        *self.shown.borrow_mut() = fitted.shown().map(str::to_owned);
        let layout = fitted.into_layout();
        let size = layout.size();
        *self.prepared.borrow_mut() = Some(layout);
        Ok(Size::new(
            (size.width() + padding).min(a.width().unwrap_or(f32::MAX)),
            (size.height() + 2.0 * cx.metric(MetricRole::ControlPaddingY))
                .max(cx.metric(MetricRole::ControlHeight)),
        )?)
    }
    fn arrange(&self, _: &mut ArrangeContext<'_>, size: Size) -> Result<(), RuntimeError> {
        self.size.set(size);
        Ok(())
    }
    fn paint(&self, p: &mut Painter) -> Result<(), DrawError> {
        if let Some(text) = self.prepared.borrow().as_ref() {
            text.paint(
                p,
                crate::Point::new(
                    p.metric(MetricRole::ControlPaddingX),
                    centered(self.size.get().height(), text.size().height()),
                )?,
                p.color(ColorRole::OnButton),
            )?;
        }
        Ok(())
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
        let mut details = ComponentDetails::default();
        details.properties.insert(
            "text".into(),
            InspectValue::Text(self.text.text().to_owned()),
        );
        let shown = self.shown.borrow();
        details
            .properties
            .insert("truncated".into(), InspectValue::Bool(shown.is_some()));
        if let Some(shown) = shown.as_ref() {
            details
                .properties
                .insert("shown".into(), InspectValue::Text(shown.clone()));
        }
        details
    }
}

/// A title-bar button in its tooltip, which shows the button's name: both
/// follow the string table's text when the label is the library's.
fn named_tooltip(button: Button, label: TextSource) -> Tooltip {
    Tooltip::new(Element::new("button", button), label)
}
