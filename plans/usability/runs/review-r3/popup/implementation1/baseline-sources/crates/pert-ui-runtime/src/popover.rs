//! Owned anchored surfaces built on the public component contract.
use crate::layout::rect;
use crate::*;
use std::time::Duration;

/// Presentation and input policy for an owner's promoted child subtree.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SurfaceKind {
    /// Interactive nonmodal surface. Shared routing transfers focus on admission,
    /// consumes outside presses, dismisses on Escape and restores the opener.
    Popover,
    /// Noninteractive help. Drawing is lifted but pointer/focus routing skips the
    /// entire subtree. It must not contain interactive children.
    Tooltip,
    /// Window-modal content, centered in the window. Blocks input outside its
    /// subtree; Escape requests cancellation and outside presses are consumed.
    Dialog,
    /// Application-modal content in one available root. Blocks every other root
    /// until its admitted chain closes; unrelated modal admission is rejected.
    ApplicationDialog,
    /// Modal content centered/clipped within its logical owner's allocation.
    /// Only that owner's subtree is blocked; unrelated views remain interactive.
    ViewDialog,
    /// Nonmodal notifications placed at the bottom right of the window. Admission
    /// never transfers focus and outside presses do not dismiss notifications.
    Notification,
    /// A submenu using its owner's supplied child placement, preferring the right
    /// side and flipping left within the window. Shares popover dismissal rules.
    Submenu,
    /// A context menu at the owner's supplied local child position, constrained
    /// to the window. Shares popover focus and dismissal rules.
    ContextMenu,
}
impl SurfaceKind {
    pub(crate) fn nonmodal(self) -> bool {
        matches!(self, Self::Popover | Self::Submenu | Self::ContextMenu)
    }
    pub(crate) fn modal(self) -> bool {
        matches!(
            self,
            Self::Dialog | Self::ViewDialog | Self::ApplicationDialog
        )
    }
    pub(crate) fn interactive(self) -> bool {
        self.nonmodal() || self.modal()
    }
}
/// Why the framework asks an owner to close its anchored surface.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SurfaceDismissal {
    /// The window has no nonzero space above or below the anchor.
    ///
    /// The anchor is the owner's whole allocation, so an owner as tall as the
    /// window can never present its surface. That includes any surface owner
    /// mounted directly as a root, because a root is allocated the entire
    /// viewport; place the owner inside a layout container instead.
    NoSpace,
    /// Initial Escape press on the topmost interactive surface.
    Escape,
    /// A consumed pointer press outside the current surface chain.
    OutsidePress,
    /// Keyboard focus moved outside the logical owner subtree, or out of a
    /// nonmodal surface it had entered to another part of the owner, such as
    /// the surface's own anchor.
    FocusLeft,
    /// The native window lost its input context.
    WindowFocusLost,
    /// The owner or an ancestor became hidden. Hiding the native root ends
    /// nonmodal surfaces this way but only suspends a modal dialog.
    OwnerHidden,
    /// The owning incarnation was removed.
    OwnerRemoved,
    /// The anchor became fully clipped or the viewport lost its extent.
    AnchorClipped,
    /// Another independent nonmodal surface chain opened.
    Replaced,
    /// Another modal chain already blocks the requested scope.
    ModalConflict,
}
fn children(anchor: Element, content: Element) -> Vec<Element> {
    vec![
        Element::new("anchor", Container::new(anchor)).structural(),
        Element::new(
            "popup",
            transient_panel(
                Element::new(
                    "scroll",
                    ScrollArea::new(
                        Element::new(
                            "surface",
                            Container::new(content).padding(MetricRole::ControlPaddingX),
                        )
                        .structural(),
                    )
                    .axes(ScrollAxes::Both),
                )
                .structural(),
            ),
        )
        .structural(),
    ]
}
/// The shared frame of every built-in transient surface: a Surface fill with a
/// PanelBorder outline at border width, rounded by the theme corner radius. The
/// frame stays fixed while content inside it scrolls, and it is reproducible by
/// consumers through `Container::border_role`.
pub(crate) fn transient_panel(content: Element) -> Container {
    Container::new(content)
        .background_role(ColorRole::Surface)
        .border_role(ColorRole::PanelBorder)
        // A surface floats above unrelated content, so it must not inherit the
        // owner's cursor: a menu over a text field would show the editor caret.
        .cursor(crate::CursorIcon::Default)
}
pub(crate) fn measure_anchor(
    cx: &MeasureContext<'_>,
    available: Constraints,
) -> Result<Size, RuntimeError> {
    if cx.is_empty() {
        Ok(Size::ZERO)
    } else {
        cx.measure(0, available)
    }
}
pub(crate) fn arrange_surface(cx: &mut ArrangeContext<'_>, size: Size) -> Result<(), RuntimeError> {
    arrange_popup(cx, size, true)
}
fn arrange_popup(
    cx: &mut ArrangeContext<'_>,
    size: Size,
    anchor_width_minimum: bool,
) -> Result<(), RuntimeError> {
    if cx.len() > 2 {
        return Err(LayoutError::InvalidChild.into());
    }
    if !cx.is_empty() {
        cx.place(0, Rect::from_size(size))?;
    }
    if cx.len() == 2 {
        let viewport = cx.measurement().viewport();
        let available = Constraints::new(Some(viewport.width()), Some(viewport.height()))?;
        let popup = cx.measure(1, available)?;
        cx.place(
            1,
            rect(
                0.0,
                size.height(),
                if anchor_width_minimum {
                    popup.width().max(size.width())
                } else {
                    popup.width()
                }
                .min(viewport.width()),
                popup.height().min(viewport.height()),
            )?,
        )?;
    }
    Ok(())
}
type DismissHandler =
    Box<dyn FnMut(&mut EventContext, SurfaceDismissal) -> Result<(), RuntimeError>>;

/// An anchored nonmodal surface with retained content and automatic placement.
///
/// The caller supplies an open-state signal, an anchor and content. False is
/// closed; true admits a surface above ordinary drawing and outside ancestor clips.
/// The surface is a rounded Surface panel with a PanelBorder outline and
/// ControlPaddingX inside; content scrolls inside the fixed frame.
/// Placement prefers below, flips above when needed and constrains/scrolls oversized
/// content inside the window. Anchor movement follows layout without polling.
/// The surface never overlaps its anchor: with no room on either side the request
/// is refused with [`SurfaceDismissal::NoSpace`] and the signal returns to false.
/// A popover mounted directly as a root is allocated the whole viewport, so it is
/// always refused; mount it inside a layout container such as a `Column`.
/// Escape/outside press, focus leaving, clipped/hidden/removed owners and window
/// focus loss set the signal false. Focus leaves when it moves out of the
/// content after having been inside it, also to the popover's own anchor (Tab,
/// Shift+Tab or [`Runtime::focus`](crate::Runtime::focus)); focus then stays
/// where it moved. Outside presses and their releases are consumed.
/// One independent interactive chain is open per window; nested popovers are allowed.
/// Admission focuses the first eligible descendant; closing restores a surviving
/// opener unless focus intentionally moved elsewhere. Hidden content retains state.
///
/// For surfaces constructed with [`Self::new`], compatible redescription
/// reconciles anchor and content by key and adopts a replacement external open
/// signal from the next update. The replaced signal is no longer read or written.
/// [`Self::on_dismiss`] reports why the framework closed the surface; closing
/// it by writing the signal is the application's own act and reports nothing.
/// [`ColorPicker::into_popover`] instead owns one retained composition: its
/// initial anchor, picker binding/options and open state survive redescription.
/// Ownership is fixed at mount; changing construction mode requires a new key.
#[must_use]
pub struct Popover {
    children: Vec<Element>,
    open: Signal<bool>,
    shown: bool,
    /// A convenience controller owns its initial anchor/content and open state.
    /// This policy remains fixed for the mounted incarnation.
    retain_parts: bool,
    /// The change handler of a [`ColorPicker`](crate::ColorPicker) this
    /// owned surface retains: a new description's handler reaches it here.
    picker_changed: Option<crate::color_picker::ChangeSlot>,
    on_dismiss: Option<DismissHandler>,
}
description_debug!(Popover);
impl Popover {
    pub(crate) fn empty(open: Signal<bool>) -> Self {
        Self {
            children: Vec::new(),
            open,
            shown: false,
            retain_parts: false,
            picker_changed: None,
            on_dismiss: None,
        }
    }
    pub(crate) fn set_list_content(&mut self, anchor: Element, content: Element) {
        self.children = vec![
            Element::new("anchor", Container::new(anchor)).structural(),
            Element::new(
                "popup",
                transient_panel(content).padding(MetricRole::ControlPaddingX),
            )
            .structural(),
        ];
    }

    /// Creates a surface; an anchor Button may set the supplied signal to open it.
    /// All elements and the signal belong to the mounting runtime's UI thread.
    /// Compatible redescription adopts a replacement external signal and
    /// reconciles children. A convenience-owned surface from
    /// [`ColorPicker::into_popover`] retains its controller and initial parts;
    /// changing between these ownership modes requires a new element key.
    pub fn new(open: impl Into<Signal<bool>>, anchor: Element, content: Element) -> Self {
        let open: Signal<bool> = open.into();
        Self {
            children: children(anchor, content),
            open,
            shown: false,
            retain_parts: false,
            picker_changed: None,
            on_dismiss: None,
        }
    }
    /// Runs after the framework closed the surface, with the reason: Escape,
    /// an outside press, focus leaving, a hidden or removed owner, lost
    /// window focus, a clipped anchor or no space. It runs in the event
    /// phase, after the open signal was set to false, and may write state.
    /// A close the application performs by writing the signal is not a
    /// dismissal and reports nothing, so the handler sees only the user's and
    /// the framework's closes. Describing the surface again replaces the
    /// handler, or removes it when the new description has none, including on
    /// a convenience-owned surface. A failure propagates through input
    /// routing; the surface stays closed and the same close is not reported
    /// again when the runtime retries a deferred dismissal. Hiding or
    /// removing the owner of a surface the application already closed
    /// reports nothing either.
    ///
    /// ```
    /// # use pert_ui_runtime as pert_ui;
    /// use pert_ui::{
    ///     Element, KeyAction, KeyCode, Label, Popover, Runtime, SurfaceDismissal, Viewport,
    /// };
    /// let mut ui = Runtime::new();
    /// ui.set_viewport(Viewport::new(300.0, 200.0, 1.0)?)?;
    /// let open = ui.signal(false);
    /// let reason = ui.signal(None);
    /// let report = reason.clone();
    /// let popover = Popover::new(
    ///     open.clone(),
    ///     Element::new("anchor", Label::new("Anchor")),
    ///     Element::new("content", Label::new("Content")),
    /// )
    /// .on_dismiss(move |_, why| report.set(Some(why)));
    /// ui.mount_tree(Element::new("root", pert_ui::Column::new([
    ///     Element::new("popover", popover),
    /// ])))?;
    /// ui.flush(std::time::Duration::ZERO)?;
    /// open.set(true)?;
    /// ui.flush(std::time::Duration::ZERO)?;
    /// ui.key_input(KeyCode::Escape, KeyAction::Press)?;
    /// ui.key_input(KeyCode::Escape, KeyAction::Release)?;
    /// ui.flush(std::time::Duration::ZERO)?;
    /// assert_eq!(reason.get()?, Some(SurfaceDismissal::Escape));
    /// assert!(!open.get()?);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn on_dismiss(
        mut self,
        dismiss: impl FnMut(&mut EventContext, SurfaceDismissal) -> Result<(), RuntimeError> + 'static,
    ) -> Self {
        self.on_dismiss = Some(Box::new(dismiss));
        self
    }
    /// Creates a convenience surface whose initial parts and open state form one
    /// retained controller. Rebinding or changing ownership requires a new key.
    pub(crate) fn owned(open: Signal<bool>, anchor: Element, content: Element) -> Self {
        Self {
            retain_parts: true,
            ..Self::new(open, anchor, content)
        }
    }
    /// Carries a retained picker's change handler through redescription.
    pub(crate) fn carrying(mut self, changed: crate::color_picker::ChangeSlot) -> Self {
        self.picker_changed = Some(changed);
        self
    }
}
impl Component for Popover {
    fn retain_initial_children(&self) -> bool {
        self.retain_parts
    }
    fn reconcile(&mut self, next: Self) -> Option<Invalidation> {
        // The handler replaces unconditionally, like any callback, before the
        // ownership rules below decide what else the description changes.
        self.on_dismiss = next.on_dismiss;
        // The retained picker keeps everything but its handler, which follows
        // the description like any callback.
        if let (Some(mine), Some(next)) = (&self.picker_changed, &next.picker_changed) {
            mine.replace_from(next);
        }
        if self.retain_parts || self.open.read_only().same_source(&next.open.read_only()) {
            return None;
        }
        // The next update reads the new signal and shows or hides the surface.
        self.open = next.open;
        Some(Invalidation::Layout)
    }
    fn take_children(&mut self) -> Vec<Element> {
        std::mem::take(&mut self.children)
    }
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        self.shown = cx.read(&self.open)?;
        Ok(())
    }
    fn child_visible(&self, key: &str) -> bool {
        key != "popup" || self.shown
    }
    fn child_surface(&self, key: &str) -> Option<SurfaceKind> {
        (key == "popup").then_some(SurfaceKind::Popover)
    }
    fn dismiss_surface(
        &mut self,
        cx: &mut EventContext,
        reason: SurfaceDismissal,
    ) -> Result<(), RuntimeError> {
        // The runtime asks every owner of a surface child, shown or not. A
        // surface the application already closed through its signal has
        // nothing to dismiss when its owner is hidden or removed, and a
        // dismissal the runtime retries after the handler failed must not
        // report the same close twice: the signal was set before the handler
        // ran, so a false signal here means the close is already complete.
        if !self.open.get()? {
            self.shown = false;
            return Ok(());
        }
        self.open.set(false)?;
        self.shown = false;
        if let Some(dismiss) = &mut self.on_dismiss {
            dismiss(cx, reason)?;
        }
        Ok(())
    }
    fn measure(&self, cx: &MeasureContext<'_>, a: Constraints) -> Result<Size, RuntimeError> {
        measure_anchor(cx, a)
    }
    fn arrange(&self, cx: &mut ArrangeContext<'_>, s: Size) -> Result<(), RuntimeError> {
        arrange_surface(cx, s)
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
        d.semantics = Some(
            Semantics::new(SemanticRole::Popover)
                .expanded(self.shown)
                .enabled_actions(self.shown.then_some(SemanticAction::Dismiss)),
        );
        d
    }
}

/// Delayed, noninteractive help for an anchor's hover or descendant keyboard focus.
///
/// Help is a rounded Surface panel with a PanelBorder outline and ControlPaddingX
/// around the text. Shows after 500 ms using the shared runtime scheduler. Leaving hover and focus,
/// window focus loss, hiding/removal or a fully clipped anchor cancels presentation.
/// An unmodified Escape press dismisses shown help in the window and takes that
/// Escape, so a second Escape reaches surfaces, the focused component and
/// commands as usual; with no help shown, Escape is not consumed.
/// Dismissed help stays suppressed until the activation context leaves. No polling,
/// independent timer, focus capture or interactive tooltip content is introduced.
///
/// Re-describing the tooltip under the same key reconciles its anchor and help
/// by key, so new help text applies; the show delay and any suppression after
/// dismissal are retained.
#[must_use]
pub struct Tooltip {
    children: Vec<Element>,
    timer: HelpTimer,
}
description_debug!(Tooltip);
impl Tooltip {
    /// Wraps an anchor with default-styled help text and a 500 ms show delay.
    /// The text is a [`TextSource`]: fixed plain or rich content, or a binding
    /// the help follows, for controls whose accessible name changes with their
    /// state. Text changes remeasure the help when needed. The panel follows
    /// its content width, bounded by the viewport; a wider anchor does not
    /// stretch short help text. Help text its [`Label`] cannot prepare, such
    /// as text too long to lay out at the panel's width, is refused as the
    /// label documents, and the panel is then empty rather than showing
    /// earlier text.
    pub fn new(anchor: Element, text: impl Into<TextSource>) -> Self {
        Self::with_help(anchor, Label::new(text).clearing_refused())
    }
    /// Wraps an anchor that presents a registered command, such as a
    /// [`Button::command_icon`](crate::Button::command_icon), with help naming
    /// the command and its first shortcut, for example `Undo (Ctrl+Z)`, as a
    /// menu row shows them. The shortcut follows
    /// [`Runtime::command_shortcuts`](crate::Runtime::command_shortcuts), so
    /// rebinding the command updates the help; a command without shortcuts
    /// shows its name alone. The name is the one the command has when the
    /// tooltip is built, and unregistering the command later leaves the help
    /// with its last name and shortcut. Otherwise this is [`Self::new`].
    ///
    /// # Errors
    /// `StaleCommand` if the command is no longer registered, and the errors of
    /// [`ReadSignal::map`] when called where a binding cannot be derived, such
    /// as `WriteDuringInspection` inside an inspection callback.
    pub fn command(
        ui: &crate::Runtime,
        anchor: Element,
        command: &crate::CommandId,
    ) -> Result<Self, RuntimeError> {
        let name = ui.command_name(command)?.to_owned();
        let help = ui
            .command_shortcuts(command)?
            .map(move |chords| match chords.first() {
                Some(chord) => format!("{name} ({chord})"),
                None => name.clone(),
            })?;
        Ok(Self::new(anchor, help))
    }
    fn with_help(anchor: Element, help: Label) -> Self {
        Self {
            children: children(anchor, Element::new("help", help).structural()),
            timer: HelpTimer::default(),
        }
    }
}
impl Component for Tooltip {
    fn take_children(&mut self) -> Vec<Element> {
        std::mem::take(&mut self.children)
    }
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        let active = cx.hovered() || cx.focus_within();
        self.timer.update(cx, active)
    }
    fn child_visible(&self, key: &str) -> bool {
        key != "popup" || self.timer.shown()
    }
    fn child_surface(&self, key: &str) -> Option<SurfaceKind> {
        (key == "popup").then_some(SurfaceKind::Tooltip)
    }
    fn dismiss_surface(
        &mut self,
        _: &mut EventContext,
        _: SurfaceDismissal,
    ) -> Result<(), RuntimeError> {
        self.timer.dismiss();
        Ok(())
    }
    fn measure(&self, cx: &MeasureContext<'_>, a: Constraints) -> Result<Size, RuntimeError> {
        measure_anchor(cx, a)
    }
    fn arrange(&self, cx: &mut ArrangeContext<'_>, s: Size) -> Result<(), RuntimeError> {
        arrange_popup(cx, s, false)
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
            .insert("open".into(), InspectValue::Bool(self.timer.shown()));
        d.semantics = Some(Semantics::new(SemanticRole::Tooltip).expanded(self.timer.shown()));
        d
    }
}

/// Shared delayed-help scheduling for tooltips and controls that present their
/// own accessible name: one 500 ms deadline while the activation context stays,
/// suppression after dismissal until that context leaves, and no polling.
#[derive(Default)]
pub(crate) struct HelpTimer {
    shown: bool,
    started: Option<Duration>,
    suppressed: bool,
}
impl HelpTimer {
    /// Advances the timer for the current update; `active` is whether hover or
    /// keyboard focus currently justifies help.
    pub(crate) fn update(
        &mut self,
        cx: &mut UpdateContext,
        active: bool,
    ) -> Result<(), RuntimeError> {
        if !active {
            self.shown = false;
            self.started = None;
            self.suppressed = false;
        } else if !self.suppressed {
            let start = *self.started.get_or_insert(cx.now());
            let deadline = start
                .checked_add(Duration::from_millis(500))
                .ok_or(RuntimeError::Exhausted)?;
            self.shown = cx.now() >= deadline;
            if !self.shown {
                cx.wake_after(deadline - cx.now())?;
            }
        }
        Ok(())
    }
    /// Whether the help surface is currently presented.
    pub(crate) fn shown(&self) -> bool {
        self.shown
    }
    /// Hides help until the activation context leaves.
    pub(crate) fn dismiss(&mut self) {
        self.shown = false;
        self.started = None;
        self.suppressed = true;
    }
    /// Hides help and restarts the delay, for a changed help subject.
    pub(crate) fn restart(&mut self) {
        self.shown = false;
        self.started = None;
        self.suppressed = false;
    }
}

/// Places a component's single help child below its allocation, constrained to
/// the window, for owners that promote that child as a tooltip surface.
pub(crate) fn arrange_help(cx: &mut ArrangeContext<'_>, size: Size) -> Result<(), RuntimeError> {
    if cx.len() != 1 {
        return Err(LayoutError::InvalidChild.into());
    }
    let viewport = cx.measurement().viewport();
    let help = cx.measure(
        0,
        Constraints::new(Some(viewport.width()), Some(viewport.height()))?,
    )?;
    cx.place(
        0,
        rect(
            0.0,
            size.height(),
            help.width().min(viewport.width()),
            help.height().min(viewport.height()),
        )?,
    )
}
