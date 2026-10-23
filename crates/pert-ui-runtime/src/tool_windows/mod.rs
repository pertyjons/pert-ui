//! Tool windows docked at the edges of a center, opened from stripes of
//! buttons along those edges, as in IDEs.
mod divider;
mod model;
mod options;
mod panel;
mod stripe;

use crate::*;
use model::{Link, Model, Window};
use std::{
    cell::RefCell,
    collections::{BTreeMap, BTreeSet},
    rc::{Rc, Weak},
};

/// An edge of a [`ToolWindowLayout`]. Left and right panels span the inner
/// height; top and bottom panels sit between them.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, PartialOrd, Ord)]
pub enum ToolEdge {
    /// The left edge; its stripe and panels run top to bottom.
    Left,
    /// The right edge; its stripe and panels run top to bottom.
    Right,
    /// The top edge; its stripe and panels run left to right.
    Top,
    /// The bottom edge; its stripe and panels run left to right.
    Bottom,
}
impl ToolEdge {
    /// Every edge, in the order panels take their space.
    const ALL: [Self; 4] = [Self::Left, Self::Right, Self::Top, Self::Bottom];
    /// The edge's name in child keys and inspection.
    const fn name(self) -> &'static str {
        match self {
            Self::Left => "left",
            Self::Right => "right",
            Self::Top => "top",
            Self::Bottom => "bottom",
        }
    }
    /// Whether the edge's stripe and panels run vertically.
    const fn vertical(self) -> bool {
        matches!(self, Self::Left | Self::Right)
    }
    /// The edge named `name` in a workspace document.
    fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|edge| edge.name() == name)
    }
    /// The options menu item that moves a tool window to this edge.
    const fn move_role(self) -> StringRole {
        match self {
            Self::Left => StringRole::ToolWindowMoveLeft,
            Self::Right => StringRole::ToolWindowMoveRight,
            Self::Top => StringRole::ToolWindowMoveTop,
            Self::Bottom => StringRole::ToolWindowMoveBottom,
        }
    }
    /// The accessibility name of this edge's stripe.
    const fn stripe_role(self) -> StringRole {
        match self {
            Self::Left => StringRole::ToolWindowStripeLeft,
            Self::Right => StringRole::ToolWindowStripeRight,
            Self::Top => StringRole::ToolWindowStripeTop,
            Self::Bottom => StringRole::ToolWindowStripeBottom,
        }
    }
    /// The accessibility name of this edge's divider.
    const fn resize_role(self) -> StringRole {
        match self {
            Self::Left => StringRole::ToolWindowResizeLeft,
            Self::Right => StringRole::ToolWindowResizeRight,
            Self::Top => StringRole::ToolWindowResizeTop,
            Self::Bottom => StringRole::ToolWindowResizeBottom,
        }
    }
}

/// Where a tool window's button sits in its edge's stripe: from the start
/// (top or left) or from the end (bottom or right). Each group shows at most
/// one expanded tool window; when both groups of an edge are expanded they
/// share the edge.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Hash, PartialOrd, Ord)]
pub enum StripeGroup {
    /// From the start of the stripe. The default.
    #[default]
    Start,
    /// From the end of the stripe.
    End,
}
impl StripeGroup {
    /// The group's name in a workspace document.
    const fn name(self) -> &'static str {
        match self {
            Self::Start => "start",
            Self::End => "end",
        }
    }
    /// The group named `name` in a workspace document.
    fn from_name(name: &str) -> Option<Self> {
        [Self::Start, Self::End]
            .into_iter()
            .find(|group| group.name() == name)
    }
}

/// What an edge of a [`ToolWindowLayout`] holds.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Hash)]
pub enum EdgeMode {
    /// Nothing: no stripe and no panels. The default for every edge; a tool
    /// window placed on such an edge is a description error.
    #[default]
    None,
    /// Panels without a stripe: the edge's tool windows open only through
    /// [`ToolWindowController`], a reveal or their initial state.
    Panels,
    /// A stripe of buttons along the edge that expand and collapse its tool
    /// windows, and their panels.
    Stripe,
}

/// Where an expanded tool window of a [`ToolWindowLayout`] is presented.
///
/// Overlay ([`ToolWindow::overlay`]) is a variant of docking: an overlay
/// panel stays at its edge but slides over the center.
///
/// ```
/// # use pert_ui_runtime as pert_ui;
/// use pert_ui::*;
/// use std::time::Duration;
///
/// let mut ui = Runtime::new();
/// ui.set_viewport(Viewport::new(800.0, 600.0, 1.0)?)?;
/// let controller = ToolWindowController::new();
/// let layout = ToolWindowLayout::new(Element::new("editor", Label::new("Main content")))
///     .edge(ToolEdge::Left, EdgeMode::Stripe)
///     .window(
///         ToolWindow::new("files", "Files", Element::new("tree", Label::new("Files")))
///             .mode(ToolWindowMode::Floating),
///     )
///     .controller(&controller);
/// ui.mount_tree(Element::new("frame", layout))?;
/// ui.flush(Duration::ZERO)?;
/// controller.expand("files")?;
/// ui.flush(Duration::ZERO)?;
/// assert_eq!(controller.mode("files")?, ToolWindowMode::Floating);
///
/// // Opening it in its own window queues a native detach for the host.
/// controller.set_mode("files", ToolWindowMode::Window)?;
/// ui.flush(Duration::ZERO)?;
/// assert!(ui
///     .take_presentation_requests()?
///     .iter()
///     .any(|request| matches!(request.action(), PresentationAction::Detach { .. })));
/// # Ok::<(), RuntimeError>(())
/// ```
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Hash)]
#[non_exhaustive]
pub enum ToolWindowMode {
    /// At its edge, beside or over the center. The default.
    #[default]
    Docked,
    /// In a frame over the layout that the user moves by its header and
    /// resizes by its corner grip. The frame belongs to the layout, so the
    /// content keeps its state; see [`ToolWindowLayout`].
    Floating,
    /// In its own native window, through the runtime's native detachment
    /// ([`EventContext::detach_view`]). Closing that window returns the tool
    /// window to its stripe; see [`ToolWindowLayout`].
    Window,
}
impl ToolWindowMode {
    /// The mode's name in a workspace document and inspection.
    const fn name(self) -> &'static str {
        match self {
            Self::Docked => "docked",
            Self::Floating => "floating",
            Self::Window => "window",
        }
    }
    /// The mode named `name` in a workspace document.
    fn from_name(name: &str) -> Option<Self> {
        [Self::Docked, Self::Floating, Self::Window]
            .into_iter()
            .find(|mode| mode.name() == name)
    }
}

/// What a [`ToolWindowChange`] changed.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Hash)]
#[non_exhaustive]
pub enum ToolWindowChangeKind {
    /// The tool window expanded or collapsed; see
    /// [`ToolWindowChange::expanded`]. The default.
    #[default]
    Expansion,
    /// The tool window moved to another edge or group, or to another
    /// position in its group's stripe; see [`ToolWindowChange::edge`] and
    /// [`ToolWindowChange::group`].
    Placement,
    /// The tool window switched between docked and overlay, or between
    /// docked, floating and its own window; see [`ToolWindowChange::overlay`]
    /// and [`ToolWindowChange::mode`].
    Mode,
    /// The tool window's committed geometry changed: a resize of its docked
    /// panel or a move or resize of its floating frame ended, or a restore
    /// applied them; see [`ToolWindowChange::extent`] and
    /// [`ToolWindowChange::floating`]. Never reported while a gesture is in
    /// progress, nor for one that was cancelled. Changes of one tool window's
    /// geometry within one flush are delivered once, with the last value and
    /// origin.
    Geometry,
}

/// Whether a tool window is expanded or collapsed, as
/// [`ToolWindowChange::new`] takes it.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Expansion {
    /// Collapsed: its content is not shown.
    Collapsed,
    /// Expanded: its content is shown where its mode presents it.
    Expanded,
}

/// A change of a tool window, reported after it took effect, with the tool
/// window's state after it.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct ToolWindowChange {
    /// The tool window's key.
    pub key: String,
    /// What changed.
    pub kind: ToolWindowChangeKind,
    /// Whether it is expanded now.
    pub expanded: bool,
    /// The edge it is on now.
    pub edge: ToolEdge,
    /// The stripe group it is in now.
    pub group: StripeGroup,
    /// Whether it is in overlay mode now.
    pub overlay: bool,
    /// Where it is presented now.
    pub mode: ToolWindowMode,
    /// The extent of its docked panel across its edge, in logical pixels,
    /// that a resize or a restore chose; `None` while the described or
    /// default extent applies.
    pub extent: Option<f32>,
    /// Its floating frame in the layout's logical coordinates, `None` until
    /// it was first floated or after a restore without one.
    pub floating: Option<Rect>,
    /// What caused the change.
    pub origin: ChangeOrigin,
}
impl ToolWindowChange {
    /// Describes an expansion change of `key` to `expansion`, on the left
    /// edge's start group, docked; the `with_*` methods set the rest.
    #[must_use]
    pub fn new(key: String, expansion: Expansion, origin: ChangeOrigin) -> Self {
        Self {
            key,
            kind: ToolWindowChangeKind::Expansion,
            expanded: expansion == Expansion::Expanded,
            edge: ToolEdge::Left,
            group: StripeGroup::Start,
            overlay: false,
            mode: ToolWindowMode::Docked,
            extent: None,
            floating: None,
            origin,
        }
    }
    /// Sets what changed.
    #[must_use]
    pub fn with_kind(mut self, kind: ToolWindowChangeKind) -> Self {
        self.kind = kind;
        self
    }
    /// Sets the edge and group the tool window is in.
    #[must_use]
    pub fn with_placement(mut self, edge: ToolEdge, group: StripeGroup) -> Self {
        self.edge = edge;
        self.group = group;
        self
    }
    /// Sets whether the tool window is in overlay mode.
    #[must_use]
    pub fn with_overlay(mut self, overlay: bool) -> Self {
        self.overlay = overlay;
        self
    }
    /// Sets where the tool window is presented.
    #[must_use]
    pub fn with_mode(mut self, mode: ToolWindowMode) -> Self {
        self.mode = mode;
        self
    }
    /// Sets the docked panel extent across its edge, in logical pixels, and the
    /// floating frame, as the fields [`Self::extent`] and [`Self::floating`]
    /// describe them.
    #[must_use]
    pub fn with_geometry(mut self, extent: Option<f32>, floating: Option<Rect>) -> Self {
        self.extent = extent;
        self.floating = floating;
        self
    }
}

/// One tool window of a [`ToolWindowLayout`]: a key, a title, an optional
/// icon, the edge and group it belongs to, and its content.
///
/// The key identifies the tool window for expansion, workspace persistence,
/// reveals and the [`ToolWindowController`]; the title and icon are
/// presentation. The content's own element key must not be `actions`, `options`
/// or `hide`, which the panel uses for its header.
#[must_use]
pub struct ToolWindow {
    window: Window,
    content: Element,
    actions: Option<Element>,
}
description_debug!(ToolWindow, key = |tool| tool.window.key.as_str());
impl ToolWindow {
    /// Creates a collapsed tool window on the left edge's start group.
    ///
    /// The title is a [`TextSource`]: fixed text, or a binding the stripe
    /// button, panel header, hide and options names, drag preview and
    /// inspection follow without describing the layout again; only its plain
    /// text is used. A tool window opening in its own native window takes the
    /// title's text at that moment, and the layout asks for the window to be
    /// retitled when the title changes later
    /// ([`EventContext::set_window_title`]).
    pub fn new(key: impl Into<String>, title: impl Into<TextSource>, content: Element) -> Self {
        let title_source = title.into();
        Self {
            window: Window {
                key: key.into(),
                title: title_source
                    .fixed()
                    .map(RichText::plain_text)
                    .unwrap_or_default(),
                title_source,
                icon: None,
                edge: ToolEdge::Left,
                group: StripeGroup::Start,
                expanded: false,
                extent: None,
                restored: None,
                described_extent: None,
                resizable: Permission::allowed(),
                toggles: Permission::allowed(),
                movable: Permission::allowed(),
                reorderable: Permission::allowed(),
                draggable: Permission::allowed(),
                overlayable: Permission::allowed(),
                floatable: Permission::allowed(),
                detachable: Permission::allowed(),
                overlay: false,
                mode: ToolWindowMode::Docked,
                floating: None,
                restored_floating: None,
                placed: false,
                mode_chosen: false,
                focus_entered: false,
                described_edge: ToolEdge::Left,
                described_group: StripeGroup::Start,
                described_overlay: false,
                described_mode: ToolWindowMode::Docked,
                described_index: 0,
            },
            content,
            actions: None,
        }
    }
    /// Whether the tool window starts in overlay mode: its expanded panel
    /// slides over the center instead of taking space from it, and hides
    /// when focus leaves it. Applies until the user, the application or a
    /// restore chooses a mode. The default is docked.
    pub fn overlay(mut self, overlay: bool) -> Self {
        self.window.overlay = overlay;
        self
    }
    /// Where the tool window starts to be presented when it expands: docked
    /// (the default), floating or in its own native window. Applies until
    /// the user, the application or a restore chooses a mode.
    pub fn mode(mut self, mode: ToolWindowMode) -> Self {
        self.window.mode = mode;
        self
    }
    /// Restricts whether the user may switch this tool window between docked
    /// and floating, and move or resize its floating frame. It combines with
    /// [`ToolWindowLayout::floatable`] and the stricter wins. The default
    /// allows.
    pub fn floatable(mut self, permission: impl Into<Permission>) -> Self {
        self.window.floatable = self.window.floatable.and(permission);
        self
    }
    /// Restricts whether the user may open this tool window in its own native
    /// window and return it from there. It combines with
    /// [`ToolWindowLayout::detachable`] and the stricter wins. The default
    /// allows.
    pub fn detachable(mut self, permission: impl Into<Permission>) -> Self {
        self.window.detachable = self.window.detachable.and(permission);
        self
    }
    /// Restricts whether the user may move this tool window to another edge
    /// or stripe group, through its panel's options menu or by dragging its
    /// stripe button. It combines with [`ToolWindowLayout::movable`] and the
    /// stricter wins. The default allows.
    pub fn movable(mut self, permission: impl Into<Permission>) -> Self {
        self.window.movable = self.window.movable.and(permission);
        self
    }
    /// Restricts whether the user may move this tool window to another
    /// position within its stripe group. It combines with
    /// [`ToolWindowLayout::reorderable`] and the stricter wins. The default
    /// allows.
    pub fn reorderable(mut self, permission: impl Into<Permission>) -> Self {
        self.window.reorderable = self.window.reorderable.and(permission);
        self
    }
    /// Restricts whether the user may drag this tool window's stripe button.
    /// A drag also needs [`Self::movable`] to drop on another edge or group
    /// and [`Self::reorderable`] to drop elsewhere in its own group. It
    /// combines with [`ToolWindowLayout::draggable`] and the stricter wins.
    /// The default allows.
    pub fn draggable(mut self, permission: impl Into<Permission>) -> Self {
        self.window.draggable = self.window.draggable.and(permission);
        self
    }
    /// Restricts whether the user may switch this tool window between docked
    /// and overlay. It combines with [`ToolWindowLayout::overlayable`] and
    /// the stricter wins. The default allows.
    pub fn overlayable(mut self, permission: impl Into<Permission>) -> Self {
        self.window.overlayable = self.window.overlayable.and(permission);
        self
    }
    /// Sets the icon its stripe button and panel header show. Without one,
    /// the stripe button shows the title.
    pub fn icon(mut self, icon: impl Into<IconSource>) -> Self {
        self.window.icon = Some(icon.into());
        self
    }
    /// Places the tool window on an edge. The default is [`ToolEdge::Left`].
    pub fn edge(mut self, edge: ToolEdge) -> Self {
        self.window.edge = edge;
        self
    }
    /// Places the tool window's button in a stripe group. The default is
    /// [`StripeGroup::Start`].
    pub fn group(mut self, group: StripeGroup) -> Self {
        self.window.group = group;
        self
    }
    /// Whether the tool window starts expanded. Applies only when the tool
    /// window first mounts; when two tool windows of one edge group start
    /// expanded, the first in description order wins.
    pub fn expanded(mut self, expanded: bool) -> Self {
        self.window.expanded = expanded;
        self
    }
    /// The extent across its edge, in logical pixels, until the user resizes
    /// it. Without one, a panel takes a quarter of the layout's inner extent
    /// along that axis.
    pub fn extent(mut self, extent: Length) -> Self {
        self.window.described_extent = Some(extent.get());
        self
    }
    /// Content for the panel header, placed before the hide button, such as
    /// an application [`Menu`] or icon buttons. Its element key is the
    /// application's; the header wraps it under the key `actions`.
    pub fn actions(mut self, actions: Element) -> Self {
        self.actions = Some(actions);
        self
    }
    /// Restricts whether the user may resize this tool window's panel. It
    /// combines with the layout's [`ToolWindowLayout::resizable`] and the
    /// stricter wins; calls combine the same way. The default allows.
    pub fn resizable(mut self, permission: impl Into<Permission>) -> Self {
        self.window.resizable = self.window.resizable.and(permission);
        self
    }
    /// Restricts whether the user may expand and collapse this tool window,
    /// through its stripe button and its panel's hide button. It combines
    /// with the layout's [`ToolWindowLayout::toggles`] and the stricter wins.
    /// The application's own changes, through [`ToolWindowController`], a
    /// reveal or a workspace restore, are not restricted. The default allows.
    pub fn toggles(mut self, permission: impl Into<Permission>) -> Self {
        self.window.toggles = self.window.toggles.and(permission);
        self
    }
}

type OnChange = Box<dyn FnMut(&mut EventContext, &ToolWindowChange) -> Result<(), RuntimeError>>;

/// Keys the layout uses for its own children; a tool window may not use them.
fn reserved(key: &str) -> bool {
    key == "center"
        || ToolEdge::ALL.iter().any(|edge| {
            key == format!("stripe-{}", edge.name()) || key == format!("divider-{}", edge.name())
        })
}

/// Keys a panel uses for its header's children; content may not use them.
fn header_key(key: &str) -> bool {
    matches!(key, "actions" | "options" | "hide")
}

/// The smallest extent a panel is given across its edge while space allows,
/// in logical pixels, as for the edges of a [`RegionLayout`].
const PANEL_MINIMUM: f32 = 64.0;
/// The smallest extent left to the center along each axis while space allows.
const CENTER_MINIMUM: f32 = 64.0;
/// The share of the inner extent a panel takes before the user resizes it,
/// unless its description gives an extent.
const DEFAULT_SHARE: f32 = 0.25;
/// The smallest floating frame width, in logical pixels: wide enough for a
/// header with its buttons.
const FLOAT_MINIMUM_WIDTH: f32 = 160.0;
/// The smallest floating frame height, in logical pixels.
const FLOAT_MINIMUM_HEIGHT: f32 = 120.0;
/// How much of a floating frame's top-left corner stays inside the layout,
/// as for windows in a [`WindowLayer`].
const FLOAT_REACHABLE: f32 = 48.0;

/// The frame a floating tool window takes in a layout of `size`: its own
/// frame, kept reachable, or a default centered one.
fn floating_frame(frame: Option<Rect>, size: Size) -> Result<Rect, RuntimeError> {
    let (w, h) = (size.width(), size.height());
    let (width, height) = match frame {
        Some(frame) => (
            frame.width().max(FLOAT_MINIMUM_WIDTH),
            frame.height().max(FLOAT_MINIMUM_HEIGHT),
        ),
        None => (
            (w * 0.4).max(FLOAT_MINIMUM_WIDTH).min(w.max(0.0)),
            (h * 0.5).max(FLOAT_MINIMUM_HEIGHT).min(h.max(0.0)),
        ),
    };
    let (x, y) = frame.map_or(((w - width) / 2.0, (h - height) / 2.0), |frame| {
        (frame.x(), frame.y())
    });
    let x = x.clamp(0.0, (w - width.min(FLOAT_REACHABLE)).max(0.0));
    let y = y.clamp(0.0, (h - height.min(FLOAT_REACHABLE)).max(0.0));
    Ok(Rect::new(x, y, width, height)?)
}

/// A center for the main content with tool windows docked at its edges, as
/// in IDEs: a stripe of buttons along an edge expands a tool window as a
/// panel at that edge and collapses it again.
///
/// **Every part is optional.** Each edge holds nothing (the default), panels
/// without a stripe, or a stripe with panels ([`Self::edge`], [`EdgeMode`]).
/// A stripe has a start and an end group ([`StripeGroup`]); each group shows at
/// most one expanded tool window, and expanding another collapses it. When
/// both groups of an edge are expanded they share the edge equally. Stripe
/// buttons show icons, or icons and names ([`Self::show_names`],
/// [`Self::bind_show_names`]); a tool window without an icon always shows its title.
///
/// **Arrangement.** Stripes take the outer edges, left and right at full
/// height. Inside them, left and right panels span the inner height and top
/// and bottom panels sit between them; the center fills the rest. A panel
/// starts at its description's [`ToolWindow::extent`] or a quarter of the
/// inner extent, never below 64 logical pixels while the center keeps 64.
/// The divider between a panel and the center resizes it: a pointer drag
/// commits on release, arrow keys move it by 8 logical pixels (Shift: 32)
/// while the divider has focus, and Escape restores the size from before the
/// resize. Control, Alt and Super arrow chords are left to command bindings. A collapsed tool window takes no space and is not presented, but
/// keeps its component state, drafts and scroll positions and its last size,
/// as an unselected tab does.
///
/// **Panels.** A panel has a header with the tool window's icon and title, the
/// optional [`ToolWindow::actions`], an options menu and a hide button, and
/// its content below. Escape inside a panel that its content does not handle
/// returns focus to the first focusable component of the center, as does
/// collapsing a tool window that holds focus.
///
/// **Moving.** The options menu moves a tool window to another edge that
/// holds panels, to the other group of its stripe, or one position earlier
/// or later in its group, and switches it between docked and overlay. A
/// stripe button can also be dragged: dropped on a stripe of the same layout
/// it moves there, at the position the insertion line shows; a drop past the
/// middle of the stripe's free space lands in the end group. A drag never
/// leaves its layout, and Escape cancels it. An expanded tool window stays
/// expanded where it lands and collapses the one expanded in its new group.
///
/// **Overlay.** A tool window in overlay mode ([`ToolWindow::overlay`]) slides
/// over the center at its edge instead of taking space from it, and hides
/// when focus leaves it, as a pinned-off tool window in an IDE does. Its size
/// is its extent; its edge has no divider for it.
///
/// **Floating.** A tool window in floating mode ([`ToolWindowMode::Floating`])
/// shows its panel as a frame over the whole layout, stacked above the rest,
/// at a frame of its own or centered at first. Dragging its header moves it,
/// dragging its corner grip resizes it, and Alt+arrows move it (Alt+Shift+
/// arrows resize it) by eight logical pixels when the content leaves the keys
/// unhandled; a pointer drag commits on release and Escape returns the frame
/// from before it, as for a [`FloatingWindow`]. At least 48 logical pixels of
/// its top-left corner stay inside the layout. The frame is the panel itself,
/// still the layout's child, so the content keeps its state: putting the
/// content into a [`FloatingWindow`] would remount it. Floating tool windows
/// are not exclusive within their stripe group.
///
/// **Own window.** A tool window in window mode ([`ToolWindowMode::Window`])
/// opens its panel in its own native window when it is expanded, through
/// [`EventContext::detach_view`]: the panel moves with its identity and
/// state, and the stripe button still shows it as expanded. Collapsing it
/// (its stripe button, its hide button or the controller) or choosing another
/// mode asks the window to return the panel; the user closing the native
/// window returns it as well and collapses the tool window to its stripe,
/// reported with [`ChangeOrigin::Presentation`], keeping window mode for the
/// next expansion. A detach the host refuses returns the tool window to
/// docking, also reported with [`ChangeOrigin::Presentation`]. Native window
/// placement and decorations are the platform's.
///
/// **Keyboard.** Each stripe is one Tab stop: unmodified arrow keys along the
/// stripe move between its buttons, unmodified Home and End jump to the first
/// and last, and unmodified Enter or Space expands or collapses the button's
/// tool window; modified chords are left to command bindings. Expanding from a
/// stripe moves focus into the panel. Shortcuts such as Alt+1 are the
/// application's: it registers commands that use a [`ToolWindowController`].
/// Tab order follows the children: the center, the panels in description
/// order, the dividers, then the stripes.
///
/// **Permissions** ([`Permission`]) decide what the user may do, from locked
/// to open: [`Self::resizable`], [`Self::toggles`], [`Self::movable`],
/// [`Self::reorderable`], [`Self::draggable`], [`Self::overlayable`],
/// [`Self::floatable`] and [`Self::detachable`] on the layout combine with the
/// same permissions on each [`ToolWindow`], and the stricter wins. Switching
/// into or out of floating needs the float permission, into or out of a
/// native window the detach permission, and moving or resizing a floating
/// frame the float permission. A drag needs the drag permission and, where it drops, the
/// reorder permission within its own group or the move permission
/// elsewhere; a target it may not drop on shows that nothing can be dropped.
/// A panel has no options menu when moving, reordering and every switch of
/// mode are all denied outright. [`Self::locked`] denies everything, leaving an
/// arrangement the user can only use; the default allows everything, and
/// since permissions only restrict, no builder lifts a restriction. A bound
/// permission is read at each update: revoking resizing during a held resize
/// cancels it and restores the size from before it. Revoking floating during
/// a frame move or resize cancels it and restores its previous preference.
/// Permissions restrict the
/// user only; the application's own changes through a controller, a reveal
/// or a workspace restore still apply. Names on stripes are an application
/// setting: the layout offers no built-in toggle, and the application shows
/// one (such as a menu item) that calls [`ToolWindowController::set_names`]
/// or writes the [`Self::bind_show_names`] binding.
///
/// Stripe start/end groups remain anchored while their buttons fit. Overflow
/// uses one clipped, scrollable sequence in start-then-end order; wheel input
/// scrolls along the stripe (horizontal stripes also accept vertical wheel input).
/// Arrow/Home/End navigation reveals the selected button. A button longer than
/// the stripe reveals its leading edge. Scroll endpoints yield to ancestors;
/// scrolling cancels an armed activation without toggling a panel. During a
/// drag, wheel scrolling keeps the drag active so offscreen groups can be reached,
/// and a drag resting within 24 logical pixels of either end of an overflowing
/// stripe scrolls it, as the collections do, until it reaches that end.
///
/// **Reveals** ([`Runtime::reveal`], [`EventContext::request_reveal`]) of
/// content inside a collapsed tool window expand it, also through a layout
/// nested in another layout's center or tool window: the reveal opens the
/// path from the outermost layout inwards.
///
/// **Changes** of expansion, placement and mode are reported to
/// [`Self::on_change`] as a [`ToolWindowChange`] with its
/// [`ToolWindowChangeKind`] and [`ChangeOrigin`]: [`ChangeOrigin::User`] for
/// stripe buttons, hide buttons, the options menu, drags and an overlay
/// hiding when focus leaves it, [`ChangeOrigin::Reveal`],
/// [`ChangeOrigin::Restore`], [`ChangeOrigin::Redescription`] when a new
/// description collapses a tool window, [`ChangeOrigin::Presentation`] when a
/// native window closes or a detach fails, and [`ChangeOrigin::Application`]
/// for controller calls. Moving or resizing a floating frame is not reported,
/// as resizing a panel is not.
///
/// **Workspace.** The layout takes part in [`Workspace`] persistence as the
/// `tool-windows` kind with the maps `expanded` (every tool window's flag),
/// `extents` (the sizes the user or a restore set), `edges` and `groups`
/// (every tool window's edge, `left`, `right`, `top` or `bottom`, and group,
/// `start` or `end`), `order` (every tool window's position in the layout's
/// order), `overlay` (every tool window's overlay flag), `modes` (every tool
/// window's mode, `docked`, `floating` or `window`) and `float-x`, `float-y`,
/// `float-width` and `float-height` (the frames the user or a restore gave
/// floating tool windows) and, unless names are bound, the flag `names`. A document describes the whole arrangement:
/// `expanded` and `extents` are required, and a tool window they leave out
/// restores collapsed at its described size. The placement fields are
/// optional, so documents captured before they existed still restore; a tool
/// window they leave out returns to its described edge, group, position and
/// mode. A restore never opens a native window, since detached native windows
/// wait for a later workspace schema: a tool window the document has expanded
/// in window mode restores collapsed, in window mode, and opens its window
/// when the user expands it. A restore rejects a missing required field, unknown fields, unknown
/// tool window keys, values of the wrong type, unknown edge or group names,
/// an edge whose mode is [`EdgeMode::None`], negative or repeated positions,
/// negative or non-finite extents, unknown mode names, a floating frame
/// missing one of its four numbers or with a size that is not positive, and
/// two expanded docked tool windows in one edge group. Restore is busy while a
/// divider is being resized or a floating frame is being moved or resized.
/// [`Self::persist`]`(false)` leaves the layout out.
///
/// **Identity** is the key of each tool window. Describing the layout again
/// (through [`Runtime::reconcile_children`]) with tool windows added, removed
/// or reordered, or with new titles, icons or permissions, keeps the others'
/// expansion, size and content state; [`ToolWindow::expanded`] applies only
/// when a tool window first mounts. A tool window the user, the application
/// or a restore moved keeps its edge and group while that edge still holds
/// panels, and one whose mode was chosen keeps it; until then the
/// description's edge, group, [`ToolWindow::overlay`] and [`ToolWindow::mode`]
/// apply. A floating frame and a detached panel stay as they are. Once the
/// order was changed, tool windows that stay keep the chosen order and new
/// ones take their described place. An invalid description (see Errors) is
/// rejected by [`Runtime::mount_tree`] or [`Runtime::reconcile_children`]
/// before anything changes, through [`Component::check_description`]: the
/// mounted tool windows, including ones it leaves out, their state, stripes,
/// dividers and panels stay.
/// A controller that a new description replaces or leaves out is detached.
///
/// **Nesting.** A layout can sit in another layout's center or tool window.
/// Each owns only its own stripes and tool windows; keys are unique within
/// one layout, and focus returns to the center of the layout that owns the
/// tool window.
///
/// Inspection reports the layout as a [`SemanticRole::Group`], each stripe
/// as a [`SemanticRole::Toolbar`] with one item per button (expanded state,
/// whether the user may toggle it), each panel as a [`SemanticRole::Group`]
/// named by its title, and each divider as a [`SemanticRole::Splitter`].
///
/// # Errors
/// Mounting or reconciling the layout fails, changing nothing, with
/// [`LayoutError::BlankKey`] for a blank tool window key,
/// [`LayoutError::DuplicateKey`] for a key two tool windows share,
/// [`LayoutError::ReservedKey`] for a reserved tool window key (`center`,
/// `stripe-<edge>`, `divider-<edge>`) or content keyed `actions`, `options`
/// or `hide`, and [`LayoutError::UnavailableEdge`] for a tool window on an
/// edge whose mode is [`EdgeMode::None`].
/// Returns the original signal error, including `ForeignRuntime`, when building
/// combined layout/item permissions fails; the mounted layout is preserved.
///
/// ```
/// # use pert_ui_runtime as pert_ui;
/// use pert_ui::*;
/// use std::time::Duration;
///
/// let mut ui = Runtime::new();
/// ui.set_viewport(Viewport::new(800.0, 600.0, 1.0)?)?;
/// let controller = ToolWindowController::new();
/// let layout = ToolWindowLayout::new(Element::new("editor", Label::new("Main content")))
///     .edge(ToolEdge::Left, EdgeMode::Stripe)
///     .window(
///         ToolWindow::new("files", "Files", Element::new("tree", Label::new("Files")))
///             .icon(Icon::Folder),
///     )
///     .controller(&controller);
/// ui.mount_tree(Element::new("frame", layout))?;
/// ui.flush(Duration::ZERO)?;
/// let shown = |ui: &Runtime, key: &str| ui.inspect().components.iter().any(|c| c.key == key && c.visible);
/// assert!(!shown(&ui, "tree"));
///
/// // An application command opens the tool window.
/// controller.expand("files")?;
/// ui.flush(Duration::ZERO)?;
/// assert!(shown(&ui, "tree"));
/// # Ok::<(), RuntimeError>(())
/// ```
#[must_use]
pub struct ToolWindowLayout {
    link: Rc<Link>,
    center: Option<Element>,
    contents: Vec<(String, Element, Option<Element>)>,
    names_binding: Option<Signal<bool>>,
    persist: bool,
    on_change: Option<OnChange>,
    controller: Option<ToolWindowController>,
    /// The description's validation failure, if any; the update reports it.
    invalid: Option<LayoutError>,
    build_error: Option<RuntimeError>,
}
description_debug!(ToolWindowLayout);
impl ToolWindowLayout {
    /// Creates a layout around `center`, with every edge empty
    /// ([`EdgeMode::None`]), persistence on, names hidden and every
    /// permission allowed.
    pub fn new(center: Element) -> Self {
        Self {
            link: Link::new(Model::new()),
            center: Some(center),
            contents: Vec::new(),
            names_binding: None,
            persist: true,
            on_change: None,
            controller: None,
            invalid: None,
            build_error: None,
        }
    }
    fn model(&self) -> Rc<RefCell<Model>> {
        self.link.model()
    }
    /// Sets what an edge holds. The default for every edge is
    /// [`EdgeMode::None`].
    pub fn edge(self, edge: ToolEdge, mode: EdgeMode) -> Self {
        *self.model().borrow_mut().edges.get_mut(edge) = mode;
        self
    }
    /// Adds a tool window. Its edge must hold panels ([`Self::edge`]).
    pub fn window(mut self, window: ToolWindow) -> Self {
        let ToolWindow {
            mut window,
            content,
            actions,
        } = window;
        self.contents.push((window.key.clone(), content, actions));
        let model = self.model();
        let mut model = model.borrow_mut();
        window.described_edge = window.edge;
        window.described_group = window.group;
        window.described_overlay = window.overlay;
        window.described_mode = window.mode;
        window.described_index = model.windows.len();
        model.windows.push(window);
        drop(model);
        self
    }
    /// Adds tool windows in order; see [`Self::window`].
    pub fn windows(self, windows: impl IntoIterator<Item = ToolWindow>) -> Self {
        windows.into_iter().fold(self, Self::window)
    }
    /// Restricts resizing of every panel: each divider's permission combines
    /// this with the expanded tool windows' [`ToolWindow::resizable`], and the
    /// stricter wins. The default allows.
    pub fn resizable(self, permission: impl Into<Permission>) -> Self {
        {
            let model = self.model();
            let mut model = model.borrow_mut();
            model.resizable = model.resizable.clone().and(permission);
        }
        self
    }
    /// Restricts expanding and collapsing by the user for every tool window:
    /// it combines with each [`ToolWindow::toggles`], and the stricter wins.
    /// The default allows.
    pub fn toggles(self, permission: impl Into<Permission>) -> Self {
        {
            let model = self.model();
            let mut model = model.borrow_mut();
            model.toggles = model.toggles.clone().and(permission);
        }
        self
    }
    /// Restricts moving tool windows to another edge or stripe group by the
    /// user: it combines with each [`ToolWindow::movable`], and the stricter
    /// wins. The default allows.
    pub fn movable(self, permission: impl Into<Permission>) -> Self {
        {
            let model = self.model();
            let mut model = model.borrow_mut();
            model.movable = model.movable.clone().and(permission);
        }
        self
    }
    /// Restricts reordering tool windows within their stripe group by the
    /// user: it combines with each [`ToolWindow::reorderable`], and the
    /// stricter wins. The default allows.
    pub fn reorderable(self, permission: impl Into<Permission>) -> Self {
        {
            let model = self.model();
            let mut model = model.borrow_mut();
            model.reorderable = model.reorderable.clone().and(permission);
        }
        self
    }
    /// Restricts dragging stripe buttons: it combines with each
    /// [`ToolWindow::draggable`], and the stricter wins. The default allows.
    pub fn draggable(self, permission: impl Into<Permission>) -> Self {
        {
            let model = self.model();
            let mut model = model.borrow_mut();
            model.draggable = model.draggable.clone().and(permission);
        }
        self
    }
    /// Restricts switching tool windows between docked and overlay by the
    /// user: it combines with each [`ToolWindow::overlayable`], and the
    /// stricter wins. The default allows.
    pub fn overlayable(self, permission: impl Into<Permission>) -> Self {
        {
            let model = self.model();
            let mut model = model.borrow_mut();
            model.overlayable = model.overlayable.clone().and(permission);
        }
        self
    }
    /// Restricts switching tool windows between docked and floating, and
    /// moving or resizing floating frames, by the user: it combines with
    /// each [`ToolWindow::floatable`], and the stricter wins. The default
    /// allows.
    pub fn floatable(self, permission: impl Into<Permission>) -> Self {
        {
            let model = self.model();
            let mut model = model.borrow_mut();
            model.floatable = model.floatable.clone().and(permission);
        }
        self
    }
    /// Restricts opening tool windows in their own native windows, and
    /// returning them from there, by the user: it combines with each
    /// [`ToolWindow::detachable`], and the stricter wins. The default allows.
    pub fn detachable(self, permission: impl Into<Permission>) -> Self {
        {
            let model = self.model();
            let mut model = model.borrow_mut();
            model.detachable = model.detachable.clone().and(permission);
        }
        self
    }
    /// Denies every user capability: resizing, expanding or collapsing,
    /// moving, reordering, dragging, switching between docked and overlay,
    /// floating and detaching. An arrangement the user can only use; the
    /// application can still change it through a [`ToolWindowController`].
    pub fn locked(self) -> Self {
        self.resizable(false)
            .toggles(false)
            .movable(false)
            .reorderable(false)
            .draggable(false)
            .overlayable(false)
            .floatable(false)
            .detachable(false)
    }
    /// Whether stripe buttons start with names beside their icons. Applies
    /// only when the layout mounts; the user's later choice, through the
    /// application's own toggle, is retained and persisted. The default is
    /// icons only.
    pub fn show_names(self, names: bool) -> Self {
        self.model().borrow_mut().names = names;
        self
    }
    /// Makes whether stripe buttons show names the application's
    /// `Signal<bool>`: the layout shows what it holds, and
    /// [`ToolWindowController::set_names`] writes it. Bound names are not
    /// persisted by the layout.
    pub fn bind_show_names(mut self, names: impl Into<Signal<bool>>) -> Self {
        let names: Signal<bool> = names.into();
        self.names_binding = Some(names);
        self
    }
    /// Whether the layout takes part in workspace persistence. The default
    /// is true.
    pub fn persist(mut self, persist: bool) -> Self {
        self.persist = persist;
        self
    }
    /// Reports each expansion change after it took effect, with its origin,
    /// from an event phase as [`Component::deliver_notifications`] describes.
    /// A new description replaces the callback; a description without one
    /// discards changes not yet delivered.
    ///
    /// The callback runs on the UI thread and may write bindings, which the
    /// same flush then settles. Its error is returned from the flush after
    /// every pending change was delivered; the change itself stays.
    pub fn on_change(
        mut self,
        on_change: impl FnMut(&mut EventContext, &ToolWindowChange) -> Result<(), RuntimeError>
        + 'static,
    ) -> Self {
        self.on_change = Some(Box::new(on_change));
        self
    }
    /// Lets `controller` expand and collapse this layout's tool windows. One
    /// controller serves one mounted layout at a time.
    pub fn controller(mut self, controller: &ToolWindowController) -> Self {
        self.controller = Some(controller.clone());
        self
    }
    /// Checks the description: unique, non-reserved keys, edges that hold
    /// panels, and content keys free of the header's.
    fn validate(&self) -> Result<(), LayoutError> {
        let model = self.model();
        let model = model.borrow();
        let mut keys = BTreeSet::new();
        for window in &model.windows {
            if window.key.trim().is_empty() {
                return Err(LayoutError::BlankKey);
            }
            if reserved(&window.key) {
                return Err(LayoutError::ReservedKey(window.key.clone()));
            }
            if !keys.insert(window.key.as_str()) {
                return Err(LayoutError::DuplicateKey(window.key.clone()));
            }
            if *model.edges.get(window.edge) == EdgeMode::None {
                return Err(LayoutError::UnavailableEdge);
            }
        }
        if let Some((_, content, _)) = self
            .contents
            .iter()
            .find(|(_, content, _)| header_key(&content.key))
        {
            return Err(LayoutError::ReservedKey(content.key.clone()));
        }
        Ok(())
    }
    /// Keeps at most one expanded tool window per edge group, the first in
    /// description order, recording a collapse of a tool window that was
    /// expanded under `origin`.
    fn enforce_groups(model: &mut Model, origin: Option<ChangeOrigin>) {
        let mut seen = BTreeSet::new();
        let collapse: Vec<String> = model
            .windows
            .iter()
            .filter(|window| {
                window.expanded
                    && window.mode == ToolWindowMode::Docked
                    && !seen.insert((window.edge, window.group))
            })
            .map(|window| window.key.clone())
            .collect();
        for key in collapse {
            match origin {
                Some(origin) => {
                    model.set_expanded(&key, false, origin);
                }
                None => {
                    if let Some(window) = model.windows.iter_mut().find(|w| w.key == key) {
                        window.expanded = false;
                    }
                }
            }
        }
    }
}

impl Component for ToolWindowLayout {
    fn reconcile(&mut self, next: Self) -> Option<Invalidation> {
        // An invalid description changes nothing: the mounted tool windows
        // and their state stay, its children never remove a stripe or a
        // divider (see `build`), and the update reports the error until a
        // valid description arrives.
        if let Some(error) = next.invalid {
            self.invalid = Some(error);
            return Some(Invalidation::Layout);
        }
        let described = next.link.model();
        next.link.redirect(self.model());
        let described = std::mem::replace(&mut *described.borrow_mut(), Model::new());
        {
            let model = self.model();
            let mut model = model.borrow_mut();
            let previous = std::mem::take(&mut model.windows);
            let rank = |key: &str| previous.iter().position(|old| old.key == key);
            let mut windows = Vec::with_capacity(described.windows.len());
            for mut window in described.windows {
                if let Some(old) = previous.iter().find(|old| old.key == window.key) {
                    // The same binding keeps the title it read.
                    if window.title_source.same_as(&old.title_source) {
                        window.title.clone_from(&old.title);
                    }
                    window.expanded = old.expanded;
                    window.focus_entered = old.focus_entered;
                    window.extent = old.extent;
                    window.restored = old.restored;
                    // A placement the user, the application or a restore
                    // chose survives while its edge still holds panels.
                    if old.placed && *described.edges.get(old.edge) != EdgeMode::None {
                        window.edge = old.edge;
                        window.group = old.group;
                        window.placed = true;
                    }
                    if old.mode_chosen {
                        window.overlay = old.overlay;
                        window.mode = old.mode;
                        window.mode_chosen = true;
                    }
                    window.floating = old.floating;
                    window.restored_floating = old.restored_floating;
                }
                windows.push(window);
            }
            if model.reordered {
                // Tool windows that stay keep their chosen order among the
                // positions they hold; new ones take their described place.
                let mut staying: Vec<usize> = windows
                    .iter()
                    .enumerate()
                    .filter(|(_, window)| rank(&window.key).is_some())
                    .map(|(position, _)| position)
                    .collect();
                let slots = staying.clone();
                staying.sort_by_key(|position| rank(&windows[*position].key));
                let mut taken: Vec<Option<Window>> = windows.into_iter().map(Some).collect();
                let moved: Vec<Window> = staying
                    .iter()
                    .filter_map(|position| taken[*position].take())
                    .collect();
                for (slot, window) in slots.into_iter().zip(moved) {
                    taken[slot] = Some(window);
                }
                windows = taken.into_iter().flatten().collect();
            }
            model.windows = windows;
            model.edges = described.edges;
            model.resizable = described.resizable;
            model.toggles = described.toggles;
            model.movable = described.movable;
            model.reorderable = described.reorderable;
            model.draggable = described.draggable;
            model.overlayable = described.overlayable;
            model.floatable = described.floatable;
            model.detachable = described.detachable;
            let keys: BTreeSet<String> = model
                .windows
                .iter()
                .map(|window| window.key.clone())
                .collect();
            model.panels.retain(|key, _| keys.contains(key));
            Self::enforce_groups(&mut model, Some(ChangeOrigin::Redescription));
            if next.on_change.is_none() {
                for delivery in model.deliveries.borrow_mut().iter_mut() {
                    *delivery = None;
                }
            }
        }
        self.names_binding = next.names_binding;
        self.persist = next.persist;
        self.on_change = next.on_change;
        let attached = next.controller;
        if let Some(previous) = self.controller.take()
            && !attached
                .as_ref()
                .is_some_and(|controller| controller.same_as(&previous))
        {
            previous.detach(&self.link);
        }
        self.controller = attached;
        self.invalid = None;
        self.build_error = None;
        Some(Invalidation::Layout)
    }
    fn take_children(&mut self) -> Vec<Element> {
        self.build_error = None;
        self.invalid = self.validate().err();
        {
            let model = self.model();
            Self::enforce_groups(&mut model.borrow_mut(), None);
        }
        match self.build(self.invalid.is_none()) {
            Ok(children) => children,
            Err(error) => {
                self.build_error = Some(error);
                Vec::new()
            }
        }
    }
    fn check_description(&self) -> Result<(), RuntimeError> {
        // Rejected before anything mounts, unmounts or reconciles, so an
        // invalid description never removes a tool window it fails to list.
        if let Some(error) = &self.build_error {
            Err(error.clone())
        } else if let Some(error) = self.invalid.clone() {
            Err(error.into())
        } else {
            Ok(())
        }
    }
    fn unmounted(&mut self) {
        if let Some(controller) = &self.controller {
            controller.detach(&self.link);
        }
    }
    fn stacks_children(&self) -> bool {
        // Floating frames overlap the rest; a press or focus raises the one
        // it reaches. Without them every child keeps its own area.
        self.model()
            .borrow()
            .windows
            .iter()
            .any(|window| window.expanded && window.mode != ToolWindowMode::Docked)
    }
    fn child_stacking_priority(&self, key: &str) -> i32 {
        let model = self.model();
        let model = model.borrow();
        match model.window(key) {
            Some(window) if window.mode != ToolWindowMode::Docked => 2,
            Some(window) if window.overlay => 1,
            _ => 0,
        }
    }
    fn child_redocked(&mut self, cx: &mut EventContext, child: &WidgetId) {
        let model = self.model();
        let mut model = model.borrow_mut();
        let Some(key) = model
            .panels
            .iter()
            .find(|(_, id)| *id == child)
            .map(|(key, _)| key.clone())
        else {
            return;
        };
        model.detached.remove(&key);
        // A return the panel asked for followed a collapse or a change of
        // mode, already recorded. Any other return, such as the user closing
        // the native window, brings the tool window back to its stripe.
        if model.redocking.remove(&key).is_none()
            && model
                .window(&key)
                .is_some_and(|window| window.expanded && window.mode == ToolWindowMode::Window)
        {
            model.set_expanded(&key, false, ChangeOrigin::Presentation);
        }
        model.request();
        cx.invalidate(Invalidation::Layout);
    }
    fn child_visible(&self, key: &str) -> bool {
        let model = self.model();
        let model = model.borrow();
        if model.window(key).is_none() {
            return self.child_visible_in(&model, key);
        }
        model.window(key).is_some_and(|window| window.expanded)
    }
    fn reveal_child(
        &mut self,
        cx: &mut EventContext,
        key: &str,
    ) -> Result<ChildReveal, RuntimeError> {
        self.reveal(cx, key)
    }
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        self.update_layout(cx)
    }
    fn measure(
        &self,
        cx: &MeasureContext<'_>,
        available: Constraints,
    ) -> Result<Size, RuntimeError> {
        if let (Some(width), Some(height)) = (available.width(), available.height()) {
            return Ok(Size::new(width, height)?);
        }
        let mut width: f32 = 0.0;
        let mut height: f32 = 0.0;
        for index in 0..cx.len() {
            let size = cx.measure(
                index,
                Constraints::new(available.width(), available.height())?,
            )?;
            width = width.max(size.width());
            height = height.max(size.height());
        }
        Ok(Size::new(
            available.width().unwrap_or(width),
            available.height().unwrap_or(height),
        )?)
    }
    fn arrange(&self, cx: &mut ArrangeContext<'_>, size: Size) -> Result<(), RuntimeError> {
        let divider = cx.measurement().metric(MetricRole::ResizeWidth);
        let index_of =
            |cx: &ArrangeContext<'_>, key: &str| -> Result<Option<usize>, RuntimeError> {
                for index in 0..cx.len() {
                    if cx.measurement().key(index)? == key {
                        return Ok(Some(index));
                    }
                }
                Ok(None)
            };
        let model = self.model();
        let (mut x0, mut y0, mut x1, mut y1) = (0.0f32, 0.0f32, size.width(), size.height());
        // Stripes take the outer edges: left and right at full height first.
        // Children read the model while measuring, so it is not borrowed here.
        for edge in ToolEdge::ALL {
            let Some(index) = index_of(cx, &format!("stripe-{}", edge.name()))? else {
                continue;
            };
            // A stripe an invalid description kept stays hidden and takes no
            // space.
            if *model.borrow().edges.get(edge) != EdgeMode::Stripe {
                continue;
            }
            let natural = cx.measure(index, Constraints::new(None, None)?)?;
            if edge.vertical() {
                let thickness = natural.width().min((x1 - x0).max(0.0));
                let x = if edge == ToolEdge::Left {
                    x0
                } else {
                    x1 - thickness
                };
                let rect = Rect::new(x, y0, thickness, (y1 - y0).max(0.0))?;
                cx.place_clipped(index, rect, rect)?;
                if edge == ToolEdge::Left {
                    x0 += thickness;
                } else {
                    x1 -= thickness;
                }
            } else {
                let thickness = natural.height().min((y1 - y0).max(0.0));
                let y = if edge == ToolEdge::Top {
                    y0
                } else {
                    y1 - thickness
                };
                let rect = Rect::new(x0, y, (x1 - x0).max(0.0), thickness)?;
                cx.place_clipped(index, rect, rect)?;
                if edge == ToolEdge::Top {
                    y0 += thickness;
                } else {
                    y1 -= thickness;
                }
            }
        }
        // Panels: left and right span the inner height, top and bottom sit
        // between them.
        for edge in ToolEdge::ALL {
            let open: Vec<(String, Option<f32>)> = model
                .borrow()
                .open_on(edge)
                .into_iter()
                .map(|window| {
                    (
                        window.key.clone(),
                        window.extent.or(window.described_extent),
                    )
                })
                .collect();
            if open.is_empty() {
                *model.borrow_mut().geometry.get_mut(edge) = None;
                continue;
            }
            let along = if edge.vertical() { x1 - x0 } else { y1 - y0 }.max(0.0);
            let maximum = (along - divider - CENTER_MINIMUM).max(0.0);
            let minimum = PANEL_MINIMUM.min(maximum);
            let preferred = open[0].1.unwrap_or(along * DEFAULT_SHARE);
            let extent = preferred.clamp(minimum, maximum);
            *model.borrow_mut().geometry.get_mut(edge) = Some(model::EdgeGeometry {
                extent,
                minimum,
                maximum,
            });
            let cross = if edge.vertical() { y1 - y0 } else { x1 - x0 }.max(0.0);
            let share = cross / open.len() as f32;
            for (position, (key, _)) in open.iter().enumerate() {
                let Some(index) = index_of(cx, key)? else {
                    continue;
                };
                let offset = share * position as f32;
                let rect = match edge {
                    ToolEdge::Left => Rect::new(x0, y0 + offset, extent, share)?,
                    ToolEdge::Right => Rect::new(x1 - extent, y0 + offset, extent, share)?,
                    ToolEdge::Top => Rect::new(x0 + offset, y0, share, extent)?,
                    ToolEdge::Bottom => Rect::new(x0 + offset, y1 - extent, share, extent)?,
                };
                cx.place_clipped(index, rect, rect)?;
            }
            let rule = divider.min((along - extent).max(0.0));
            let line = match edge {
                ToolEdge::Left => Rect::new(x0 + extent, y0, rule, cross)?,
                ToolEdge::Right => Rect::new(x1 - extent - rule, y0, rule, cross)?,
                ToolEdge::Top => Rect::new(x0, y0 + extent, cross, rule)?,
                ToolEdge::Bottom => Rect::new(x0, y1 - extent - rule, cross, rule)?,
            };
            if let Some(index) = index_of(cx, &format!("divider-{}", edge.name()))? {
                cx.place_clipped(index, line, line)?;
            }
            match edge {
                ToolEdge::Left => x0 += extent + rule,
                ToolEdge::Right => x1 -= extent + rule,
                ToolEdge::Top => y0 += extent + rule,
                ToolEdge::Bottom => y1 -= extent + rule,
            }
        }
        if let Some(index) = index_of(cx, "center")? {
            let rect = Rect::new(x0, y0, (x1 - x0).max(0.0), (y1 - y0).max(0.0))?;
            cx.place_clipped(index, rect, rect)?;
        }
        // Overlay panels lie over the center at its side of their edge; they
        // come after the center among the children, so they paint above it.
        for edge in ToolEdge::ALL {
            let open: Vec<(String, Option<f32>)> = model
                .borrow()
                .overlays_on(edge)
                .into_iter()
                .map(|window| {
                    (
                        window.key.clone(),
                        window.extent.or(window.described_extent),
                    )
                })
                .collect();
            if open.is_empty() {
                continue;
            }
            let along = if edge.vertical() { x1 - x0 } else { y1 - y0 }.max(0.0);
            let cross = if edge.vertical() { y1 - y0 } else { x1 - x0 }.max(0.0);
            let share = cross / open.len() as f32;
            for (position, (key, preferred)) in open.iter().enumerate() {
                let Some(index) = index_of(cx, key)? else {
                    continue;
                };
                let extent = preferred
                    .unwrap_or(along * DEFAULT_SHARE)
                    .clamp(PANEL_MINIMUM.min(along), along);
                let offset = share * position as f32;
                let rect = match edge {
                    ToolEdge::Left => Rect::new(x0, y0 + offset, extent, share)?,
                    ToolEdge::Right => Rect::new(x1 - extent, y0 + offset, extent, share)?,
                    ToolEdge::Top => Rect::new(x0 + offset, y0, share, extent)?,
                    ToolEdge::Bottom => Rect::new(x0 + offset, y1 - extent, share, extent)?,
                };
                cx.place_clipped(index, rect, rect)?;
            }
        }
        // Floating frames lie over the whole layout. A tool window waiting
        // for its own native window shows there until the window takes it.
        let floating: Vec<(String, Option<Rect>)> = {
            let model = model.borrow();
            model
                .windows
                .iter()
                .filter(|window| {
                    window.expanded
                        && (window.mode == ToolWindowMode::Floating
                            || (window.mode == ToolWindowMode::Window
                                && !model.detached.contains_key(&window.key)))
                })
                .map(|window| (window.key.clone(), window.floating))
                .collect()
        };
        for (key, frame) in floating {
            if let Some(index) = index_of(cx, &key)? {
                let rect = floating_frame(frame, size)?;
                cx.place_clipped(index, rect, rect)?;
            }
        }
        Ok(())
    }
    fn deliver_notifications(&mut self, cx: &mut EventContext) -> Result<(), RuntimeError> {
        let model = self.model();
        let (change, dirty, return_focus, center, revision) = {
            let mut model = model.borrow_mut();
            let change = model.deliveries.borrow_mut().pop_front().flatten();
            model.hide_left_overlays();
            let return_focus = std::mem::take(&mut model.return_focus);
            if return_focus {
                model.focused_panel = None;
            }
            (
                change,
                model.presentation_dirty.replace(false),
                return_focus,
                model.center.clone(),
                model.revision.clone(),
            )
        };
        let mut result = Ok(());
        if let Err(error) = model.borrow_mut().window_titles.deliver(cx) {
            result = Err(error);
        }
        if let Err(error) = self.request_detachments(cx)
            && result.is_ok()
        {
            result = Err(error);
        }
        if dirty {
            cx.invalidate(Invalidation::Layout);
        }
        if dirty
            && let Some(revision) = revision
            && let Err(error) = revision
                .get()
                .and_then(|value| revision.set(value.wrapping_add(1)))
            && result.is_ok()
        {
            result = Err(error);
        }
        if return_focus
            && let Some(center) = center
            && let Err(error) = cx.request_focus_within(&center)
            && result.is_ok()
        {
            result = Err(error);
        }
        if let (Some(on_change), Some(change)) = (&mut self.on_change, change)
            && let Err(error) = on_change(cx, &change)
            && result.is_ok()
        {
            result = Err(error);
        }
        result
    }
    fn workspace_state(&self) -> Result<Option<ViewState>, WorkspaceError> {
        if !self.persist {
            return Ok(None);
        }
        let model = self.model();
        let model = model.borrow();
        let expanded = WorkspaceValue::map(
            model
                .windows
                .iter()
                .map(|window| (window.key.clone(), WorkspaceValue::flag(window.expanded))),
        )?;
        let mut extents = Vec::new();
        for window in &model.windows {
            if let Some(extent) = window.extent {
                let number = match window.restored {
                    Some((value, document)) if value == extent => document,
                    _ => f64::from(extent),
                };
                extents.push((window.key.clone(), WorkspaceValue::number(number)?));
            }
        }
        let edges = WorkspaceValue::map(
            model
                .windows
                .iter()
                .map(|window| (window.key.clone(), WorkspaceValue::name(window.edge.name()))),
        )?;
        let groups = WorkspaceValue::map(model.windows.iter().map(|window| {
            (
                window.key.clone(),
                WorkspaceValue::name(window.group.name()),
            )
        }))?;
        let order =
            WorkspaceValue::map(model.windows.iter().enumerate().map(|(position, window)| {
                (
                    window.key.clone(),
                    WorkspaceValue::integer(i64::try_from(position).unwrap_or(i64::MAX)),
                )
            }))?;
        let overlay = WorkspaceValue::map(
            model
                .windows
                .iter()
                .map(|window| (window.key.clone(), WorkspaceValue::flag(window.overlay))),
        )?;
        let modes = WorkspaceValue::map(
            model
                .windows
                .iter()
                .map(|window| (window.key.clone(), WorkspaceValue::name(window.mode.name()))),
        )?;
        let mut frames: [Vec<(String, WorkspaceValue)>; 4] = Default::default();
        for window in &model.windows {
            let Some(frame) = window.floating else {
                continue;
            };
            let numbers = match window.restored_floating {
                Some((restored, numbers)) if restored == frame => numbers,
                _ => frame.values().map(f64::from),
            };
            for (list, number) in frames.iter_mut().zip(numbers) {
                list.push((window.key.clone(), WorkspaceValue::number(number)?));
            }
        }
        let [float_x, float_y, float_width, float_height] = frames;
        let mut state = ViewState::new("tool-windows")?
            .with("expanded", expanded)?
            .with("extents", WorkspaceValue::map(extents)?)?
            .with("edges", edges)?
            .with("groups", groups)?
            .with("order", order)?
            .with("overlay", overlay)?
            .with("modes", modes)?
            .with("float-x", WorkspaceValue::map(float_x)?)?
            .with("float-y", WorkspaceValue::map(float_y)?)?
            .with("float-width", WorkspaceValue::map(float_width)?)?
            .with("float-height", WorkspaceValue::map(float_height)?)?;
        if self.names_binding.is_none() {
            state = state.with("names", WorkspaceValue::flag(model.names))?;
        }
        Ok(Some(state))
    }
    fn restore_workspace_state(
        &mut self,
        state: &ViewState,
    ) -> Result<Invalidation, RestoreRejection> {
        state.expect_only(&[
            "expanded",
            "extents",
            "names",
            "edges",
            "groups",
            "order",
            "overlay",
            "modes",
            "float-x",
            "float-y",
            "float-width",
            "float-height",
        ])?;
        let model = self.model();
        let mut model = model.borrow_mut();
        // The document describes the whole arrangement: `expanded` and
        // `extents` are required, and a tool window they leave out restores
        // collapsed at its described size. The placement fields came later
        // and are optional; a tool window they leave out returns to its
        // described edge, group, position and mode. Everything is checked
        // before anything changes.
        let known = |key: &String| -> Result<(), RestoreRejection> {
            if model.window(key).is_some() {
                Ok(())
            } else {
                Err(RestoreRejection::Unresolved(key.clone()))
            }
        };
        let mut placement: Vec<(String, ToolEdge, StripeGroup)> = model
            .windows
            .iter()
            .map(|window| {
                (
                    window.key.clone(),
                    window.described_edge,
                    window.described_group,
                )
            })
            .collect();
        for (field, is_edge) in [("edges", true), ("groups", false)] {
            if state.get(field).is_none() {
                continue;
            }
            for (key, value) in state.map(field)? {
                known(key)?;
                let name = value
                    .as_text()
                    .ok_or_else(|| RestoreRejection::Value(field.to_owned()))?;
                let Some(entry) = placement.iter_mut().find(|(window, ..)| window == key) else {
                    return Err(RestoreRejection::Unresolved(key.clone()));
                };
                if is_edge {
                    let edge = ToolEdge::from_name(name)
                        .ok_or_else(|| RestoreRejection::Value(field.to_owned()))?;
                    if *model.edges.get(edge) == EdgeMode::None {
                        return Err(RestoreRejection::OutOfRange(field.to_owned()));
                    }
                    entry.1 = edge;
                } else {
                    entry.2 = StripeGroup::from_name(name)
                        .ok_or_else(|| RestoreRejection::Value(field.to_owned()))?;
                }
            }
        }
        let mut order = Vec::new();
        if state.get("order").is_some() {
            let mut seen = BTreeSet::new();
            for (key, value) in state.map("order")? {
                known(key)?;
                let position = value
                    .as_integer()
                    .ok_or_else(|| RestoreRejection::Value("order".to_owned()))?;
                // A position names a slot of this layout's order.
                let in_range = usize::try_from(position).is_ok_and(|p| p < model.windows.len());
                if !in_range || !seen.insert(position) {
                    return Err(RestoreRejection::OutOfRange("order".to_owned()));
                }
                order.push((key.clone(), position));
            }
        }
        let mut overlay: Vec<(String, bool)> = model
            .windows
            .iter()
            .map(|window| (window.key.clone(), window.described_overlay))
            .collect();
        if state.get("overlay").is_some() {
            for (key, value) in state.map("overlay")? {
                known(key)?;
                let flag = value
                    .as_flag()
                    .ok_or_else(|| RestoreRejection::Value("overlay".to_owned()))?;
                if let Some(entry) = overlay.iter_mut().find(|(window, _)| window == key) {
                    entry.1 = flag;
                }
            }
        }
        let mut modes: Vec<(String, ToolWindowMode)> = model
            .windows
            .iter()
            .map(|window| (window.key.clone(), window.described_mode))
            .collect();
        if state.get("modes").is_some() {
            for (key, value) in state.map("modes")? {
                known(key)?;
                let mode = value
                    .as_text()
                    .and_then(ToolWindowMode::from_name)
                    .ok_or_else(|| RestoreRejection::Value("modes".to_owned()))?;
                if let Some(entry) = modes.iter_mut().find(|(window, _)| window == key) {
                    entry.1 = mode;
                }
            }
        }
        // A floating frame needs all four numbers; a tool window the fields
        // leave out gets the default frame.
        let frame_fields = ["float-x", "float-y", "float-width", "float-height"];
        let mut frames: Vec<(String, Rect, [f64; 4])> = Vec::new();
        let given: Vec<bool> = frame_fields
            .iter()
            .map(|field| state.get(field).is_some())
            .collect();
        if given.iter().any(|given| *given) {
            if !given.iter().all(|given| *given) {
                return Err(RestoreRejection::Value("float-x".to_owned()));
            }
            let maps = frame_fields
                .iter()
                .map(|field| state.map(field))
                .collect::<Result<Vec<_>, _>>()?;
            for key in maps[0].keys() {
                known(key)?;
                let mut numbers = [0.0f64; 4];
                for ((number, map), field) in numbers.iter_mut().zip(&maps).zip(frame_fields) {
                    let value = map
                        .get(key)
                        .ok_or_else(|| RestoreRejection::Value(field.to_owned()))?;
                    *number = value
                        .as_number()
                        .or_else(|| value.as_integer().map(|value| value as f64))
                        .ok_or_else(|| RestoreRejection::Value(field.to_owned()))?;
                    let extent = matches!(field, "float-width" | "float-height");
                    if !number.is_finite()
                        || !(*number as f32).is_finite()
                        || (extent && (*number <= 0.0 || (*number as f32) <= 0.0))
                    {
                        return Err(RestoreRejection::OutOfRange(field.to_owned()));
                    }
                }
                let [x, y, width, height] = numbers.map(|number| number as f32);
                let frame = Rect::new(x, y, width, height)
                    .ok()
                    .filter(|frame| frame.values().iter().all(|value| value.is_finite()))
                    .ok_or_else(|| RestoreRejection::OutOfRange("float-x".to_owned()))?;
                frames.push((key.clone(), frame, numbers));
            }
            for map in &maps[1..] {
                if let Some(extra) = map.keys().find(|key| !maps[0].contains_key(*key)) {
                    known(extra)?;
                    return Err(RestoreRejection::Value("float-x".to_owned()));
                }
            }
        }
        let mut expanded: Vec<(String, bool)> = model
            .windows
            .iter()
            .map(|window| (window.key.clone(), false))
            .collect();
        for (key, value) in state.map("expanded")? {
            known(key)?;
            let flag = value
                .as_flag()
                .ok_or_else(|| RestoreRejection::Value("expanded".to_owned()))?;
            if let Some(entry) = expanded.iter_mut().find(|(window, _)| window == key) {
                entry.1 = flag;
            }
        }
        let mut extents = Vec::new();
        for (key, value) in state.map("extents")? {
            known(key)?;
            let number = value
                .as_number()
                .or_else(|| value.as_integer().map(|value| value as f64))
                .ok_or_else(|| RestoreRejection::Value("extents".to_owned()))?;
            // Checked before the conversion, which would turn a tiny negative
            // number into -0.0.
            if !number.is_finite() || number < 0.0 {
                return Err(RestoreRejection::OutOfRange("extents".to_owned()));
            }
            let extent = number as f32;
            if !extent.is_finite() {
                return Err(RestoreRejection::OutOfRange("extents".to_owned()));
            }
            extents.push((key.clone(), extent, number));
        }
        let names = match state.get("names") {
            Some(_) => Some(state.flag("names")?),
            None => None,
        };
        // Native windows are not reopened by a restore (detached native
        // windows wait for a later workspace schema): a tool window the
        // document has expanded in its own window restores collapsed, ready
        // to open its window when the user expands it.
        for ((_, mode), (_, open)) in modes.iter().zip(expanded.iter_mut()) {
            if *mode == ToolWindowMode::Window {
                *open = false;
            }
        }
        // Two expanded docked tool windows in one edge group cannot both show.
        let mut groups = BTreeSet::new();
        for (((_, edge, group), (_, open)), (_, mode)) in
            placement.iter().zip(&expanded).zip(&modes)
        {
            if *open && *mode == ToolWindowMode::Docked && !groups.insert((*edge, *group)) {
                return Err(RestoreRejection::Value("expanded".to_owned()));
            }
        }
        if ToolEdge::ALL
            .into_iter()
            .any(|edge| *model.resizing.get(edge))
            || !model.floating_interactions.is_empty()
        {
            return Err(RestoreRejection::Busy);
        }
        // Order: each tool window the document positions takes that slot; the
        // others fill the remaining slots in described order.
        let before: Vec<String> = model.windows.iter().map(|w| w.key.clone()).collect();
        let mut slots: Vec<Option<Window>> = (0..model.windows.len()).map(|_| None).collect();
        let mut rest = Vec::new();
        for window in std::mem::take(&mut model.windows) {
            match order.iter().find(|(key, _)| *key == window.key) {
                Some((_, position)) => {
                    // Checked in range and unique above.
                    let slot = usize::try_from(*position).unwrap_or(0);
                    slots[slot] = Some(window);
                }
                None => rest.push(window),
            }
        }
        rest.sort_by_key(|window| window.described_index);
        let mut rest = rest.into_iter();
        model.windows = slots
            .into_iter()
            .filter_map(|slot| slot.or_else(|| rest.next()))
            .collect();
        let moved: Vec<String> = model
            .windows
            .iter()
            .enumerate()
            .filter(|(index, window)| before.get(*index) != Some(&window.key))
            .map(|(_, window)| window.key.clone())
            .collect();
        model.reordered = model
            .windows
            .windows(2)
            .any(|pair| pair[0].described_index > pair[1].described_index);
        // Apply the validated final presentation in one pass. Intermediate
        // docking must not collapse a sibling the document keeps expanded.
        let mut changes = Vec::new();
        let mut return_focus = false;
        let focused = model.focused_panel.clone();
        for window in &mut model.windows {
            let old = (
                window.edge,
                window.group,
                window.overlay,
                window.mode,
                window.expanded,
            );
            if let Some((_, edge, group)) = placement.iter().find(|(key, ..)| key == &window.key) {
                window.edge = *edge;
                window.group = *group;
            }
            if let Some((_, overlay)) = overlay.iter().find(|(key, _)| key == &window.key) {
                window.overlay = *overlay;
            }
            if let Some((_, mode)) = modes.iter().find(|(key, _)| key == &window.key) {
                window.mode = *mode;
            }
            if let Some((_, expanded)) = expanded.iter().find(|(key, _)| key == &window.key) {
                window.expanded = *expanded;
            }
            if (old.0, old.1) != (window.edge, window.group) || moved.contains(&window.key) {
                changes.push((window.key.clone(), ToolWindowChangeKind::Placement));
            }
            if old.2 != window.overlay || old.3 != window.mode {
                changes.push((window.key.clone(), ToolWindowChangeKind::Mode));
            }
            if old.4 != window.expanded {
                window.focus_entered = false;
                return_focus |= !window.expanded && focused.as_deref() == Some(window.key.as_str());
                changes.push((window.key.clone(), ToolWindowChangeKind::Expansion));
            }
        }
        model.return_focus |= return_focus;
        // Preserve deterministic delivery: placement/mode changes, then
        // collapses before expansions, with each class in key order.
        let order = |(key, kind): &(String, ToolWindowChangeKind)| {
            let rank = match kind {
                ToolWindowChangeKind::Placement => 0,
                ToolWindowChangeKind::Mode | ToolWindowChangeKind::Geometry => 1,
                ToolWindowChangeKind::Expansion => {
                    if model.is_expanded(key) {
                        3
                    } else {
                        2
                    }
                }
            };
            (rank, key.clone())
        };
        changes.sort_by_key(order);
        for (key, kind) in changes {
            model.record(&key, kind, ChangeOrigin::Restore);
        }
        let geometry_before: Vec<(String, Option<f32>, Option<Rect>)> = model
            .windows
            .iter()
            .map(|window| (window.key.clone(), window.extent, window.floating))
            .collect();
        for window in &mut model.windows {
            match frames.iter().find(|(key, ..)| key == &window.key) {
                Some((_, frame, numbers)) => {
                    window.floating = Some(*frame);
                    window.restored_floating = Some((*frame, *numbers));
                }
                None => {
                    window.floating = None;
                    window.restored_floating = None;
                }
            }
        }
        // What differs from the description outlives a re-description, as
        // the user's choices do; what matches it follows the description.
        for window in &mut model.windows {
            window.placed =
                (window.edge, window.group) != (window.described_edge, window.described_group);
            window.mode_chosen =
                window.overlay != window.described_overlay || window.mode != window.described_mode;
        }
        for window in &mut model.windows {
            match extents.iter().find(|(key, ..)| key == &window.key) {
                Some((_, extent, number)) => {
                    window.extent = Some(*extent);
                    window.restored = Some((*extent, *number));
                }
                None => {
                    window.extent = None;
                    window.restored = None;
                }
            }
        }
        for (key, extent, floating) in geometry_before {
            if model
                .window(&key)
                .is_some_and(|window| (window.extent, window.floating) != (extent, floating))
            {
                model.record(&key, ToolWindowChangeKind::Geometry, ChangeOrigin::Restore);
            }
        }
        if let Some(names) = names
            && self.names_binding.is_none()
        {
            model.names = names;
        }
        model.request();
        Ok(Invalidation::Layout)
    }
    fn inspect(&self) -> ComponentDetails {
        let model = self.model();
        let model = model.borrow();
        let mut details = ComponentDetails {
            semantics: Some(Semantics::new(SemanticRole::Group)),
            ..ComponentDetails::default()
        };
        let expanded: Vec<&str> = model
            .windows
            .iter()
            .filter(|window| window.expanded)
            .map(|window| window.key.as_str())
            .collect();
        details
            .properties
            .insert("expanded".into(), InspectValue::Text(expanded.join(",")));
        let overlay: Vec<&str> = model
            .windows
            .iter()
            .filter(|window| window.overlay)
            .map(|window| window.key.as_str())
            .collect();
        details
            .properties
            .insert("overlay".into(), InspectValue::Text(overlay.join(",")));
        let modes: Vec<String> = model
            .windows
            .iter()
            .filter(|window| window.mode != ToolWindowMode::Docked)
            .map(|window| format!("{}:{}", window.key, window.mode.name()))
            .collect();
        details
            .properties
            .insert("modes".into(), InspectValue::Text(modes.join(",")));
        let detached: Vec<&str> = model.detached.keys().map(String::as_str).collect();
        details
            .properties
            .insert("detached".into(), InspectValue::Text(detached.join(",")));
        let placement: Vec<String> = model
            .windows
            .iter()
            .map(|window| {
                format!(
                    "{}:{}:{}",
                    window.key,
                    window.edge.name(),
                    window.group.name()
                )
            })
            .collect();
        details
            .properties
            .insert("placement".into(), InspectValue::Text(placement.join(",")));
        details
            .properties
            .insert("names".into(), InspectValue::Bool(model.names));
        details
            .properties
            .insert("persist".into(), InspectValue::Bool(self.persist));
        details
    }
}
impl ToolWindowLayout {
    /// Builds the description's children: the center, a panel per tool
    /// window, a divider per edge that holds panels and a stripe per edge
    /// that has one. An invalid description gets a stripe and a divider on
    /// every edge, which its update leaves hidden, so re-describing a layout
    /// with it removes none that the mounted layout shows.
    ///
    /// # Errors
    /// Returns the error of building a panel's hide button binding.
    fn build(&mut self, valid: bool) -> Result<Vec<Element>, RuntimeError> {
        let model = self.model();
        let model = model.borrow();
        let mut children = Vec::new();
        if let Some(center) = self.center.take() {
            children.push(Element::new("center", Container::new(center)).structural());
        }
        let mut built = BTreeSet::new();
        for (key, content, actions) in std::mem::take(&mut self.contents) {
            // An invalid key fails the update; skipping its panel keeps the
            // mount from failing first on a duplicate sibling key.
            if key.trim().is_empty()
                || reserved(&key)
                || header_key(&content.key)
                || !built.insert(key.clone())
            {
                continue;
            }
            let Some(window) = model.window(&key) else {
                continue;
            };
            // The options menu exists unless the user may never move,
            // reorder or switch the tool window.
            let options = [
                model.move_permission(&key),
                model.reorder_permission(&key),
                model.overlay_permission(&key),
                model.float_permission(&key),
                model.detach_permission(&key),
            ]
            .iter()
            .any(|permission| permission.fixed() != Some(false));
            let panel = panel::Panel::new(
                self.link.clone(),
                window,
                model.toggle_permission(&key),
                options,
                content,
                actions,
            )?;
            children.push(Element::new(key, panel));
        }
        for edge in ToolEdge::ALL {
            if !valid || *model.edges.get(edge) != EdgeMode::None {
                children.push(
                    Element::new(
                        format!("divider-{}", edge.name()),
                        divider::EdgeDivider::new(self.link.clone(), edge),
                    )
                    .structural(),
                );
            }
        }
        for edge in ToolEdge::ALL {
            if !valid || *model.edges.get(edge) == EdgeMode::Stripe {
                children.push(
                    Element::new(
                        format!("stripe-{}", edge.name()),
                        stripe::Stripe::new(self.link.clone(), edge),
                    )
                    .structural(),
                );
            }
        }
        Ok(children)
    }
    /// Asks the runtime to present each expanded tool window in window mode
    /// that has no native window yet, from the layout's delivery. A request
    /// the runtime refuses returns the tool window to docking.
    ///
    /// # Errors
    /// Returns the runtime's refusal of the request, such as
    /// [`RuntimeError::WindowRequestFull`], after docking the tool window.
    fn request_detachments(&mut self, cx: &mut EventContext) -> Result<(), RuntimeError> {
        let model = self.model();
        let wanted: Vec<(String, String, WidgetId, Option<Rect>)> = {
            let model = model.borrow();
            model
                .windows
                .iter()
                .filter(|window| {
                    window.expanded
                        && window.mode == ToolWindowMode::Window
                        && !model.detached.contains_key(&window.key)
                        && !model.detaching.contains_key(&window.key)
                })
                .filter_map(|window| {
                    let panel = model.panels.get(&window.key)?.clone();
                    Some((
                        window.key.clone(),
                        window.title.clone(),
                        panel,
                        window.floating,
                    ))
                })
                .collect()
        };
        let mut result = Ok(());
        for (key, title, panel, frame) in wanted {
            let request = (|| {
                let mut options = NativeWindowOptions::new(title)?;
                if let Some(frame) = frame {
                    options = options.size(Size::new(frame.width(), frame.height())?)?;
                }
                cx.detach_view(&panel, options)
            })();
            match request {
                Ok(receipt) => {
                    model.borrow_mut().detaching.insert(key, receipt);
                }
                Err(error) => {
                    model.borrow_mut().set_mode(
                        &key,
                        ToolWindowMode::Docked,
                        ChangeOrigin::Presentation,
                    );
                    if result.is_ok() {
                        result = Err(error);
                    }
                }
            }
        }
        result
    }
    fn reveal(&mut self, cx: &mut EventContext, key: &str) -> Result<ChildReveal, RuntimeError> {
        let model = self.model();
        let mut model = model.borrow_mut();
        if model.window(key).is_none() {
            return Ok(if self.child_visible_in(&model, key) {
                ChildReveal::AlreadyVisible
            } else {
                ChildReveal::Refused
            });
        }
        if model.is_expanded(key) {
            return Ok(ChildReveal::AlreadyVisible);
        }
        model.set_expanded(key, true, ChangeOrigin::Reveal);
        cx.invalidate(Invalidation::Layout);
        Ok(ChildReveal::Changed)
    }
    fn update_layout(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        if let Some(error) = self.invalid.clone() {
            return Err(error.into());
        }
        let names = match &self.names_binding {
            Some(binding) => Some(cx.read(binding)?),
            None => None,
        };
        let model = self.model();
        let revision = model::revision(&model, cx);
        cx.read(&revision)?;
        // Every title, for the names the layout itself gives, such as a
        // native window's title when a tool window detaches.
        model::read_titles(&model, cx, |_| true)?;
        let mut model = model.borrow_mut();
        if model.notifier.is_none() {
            model.notifier = Some(cx.notifier());
        }
        model.started = true;
        model.names_binding = self.names_binding.clone();
        if let Some(names) = names
            && model.names != names
        {
            // The stripes follow at the delivery, which marks the model changed.
            model.names = names;
            model.request();
        }
        // The center and the panels also register themselves as they update,
        // since children requested here mount after this update.
        let mut detached = BTreeMap::new();
        for child in cx.children() {
            if child.key() == "center" {
                model.center = Some(child.id().clone());
            } else if model.window(child.key()).is_some() {
                model
                    .panels
                    .insert(child.key().to_owned(), child.id().clone());
                if let Some(window) = child.detached_window() {
                    detached.insert(child.key().to_owned(), window);
                }
            }
        }
        model.detached = detached;
        // A detached panel's window follows its tool window's title.
        let views: Vec<_> = model
            .detached
            .iter()
            .filter_map(|(key, window)| {
                Some((
                    key.clone(),
                    window.clone(),
                    model.window(key)?.title.clone(),
                ))
            })
            .collect();
        if model.window_titles.update(cx, views)? {
            model.request();
        }
        // A detach request that failed returns the tool window to docking; one
        // presented is now among the detached children.
        let receipts: Vec<(String, PresentationReceipt)> = model
            .detaching
            .iter()
            .map(|(key, receipt)| (key.clone(), receipt.clone()))
            .collect();
        // A return request that failed is asked again at the next delivery.
        let returning: Vec<(String, PresentationReceipt)> = model
            .redocking
            .iter()
            .map(|(key, receipt)| (key.clone(), receipt.clone()))
            .collect();
        for (key, receipt) in returning {
            if matches!(
                cx.read(&receipt.status_binding())?,
                PresentationStatus::Failed(_) | PresentationStatus::CloseRejected
            ) {
                model.redocking.remove(&key);
                model.request();
            }
        }
        for (key, receipt) in receipts {
            let status = cx.read(&receipt.status_binding())?;
            match status {
                PresentationStatus::Pending => {}
                PresentationStatus::Presented(_) => {
                    model.detaching.remove(&key);
                }
                _ => {
                    model.detaching.remove(&key);
                    if !model.detached.contains_key(&key) {
                        model.set_mode(&key, ToolWindowMode::Docked, ChangeOrigin::Presentation);
                    }
                }
            }
        }
        drop(model);
        if let Some(controller) = &self.controller {
            controller.attach(&self.link);
        }
        cx.request_layout();
        Ok(())
    }
}
impl ToolWindowLayout {
    /// Whether a child that is not a tool window's panel shows: the center
    /// always, a stripe while its edge has one, a divider while its edge
    /// holds an expanded docked panel. Anything else, such as a panel an
    /// invalid description added, stays hidden.
    fn child_visible_in(&self, model: &Model, key: &str) -> bool {
        if key == "center" {
            return true;
        }
        for edge in ToolEdge::ALL {
            if key == format!("stripe-{}", edge.name()) {
                return *model.edges.get(edge) == EdgeMode::Stripe;
            }
            if key == format!("divider-{}", edge.name()) {
                return *model.edges.get(edge) != EdgeMode::None && !model.open_on(edge).is_empty();
            }
        }
        false
    }
}

/// Lets the application expand and collapse the tool windows of one mounted
/// [`ToolWindowLayout`], for example from commands bound to shortcuts such
/// as Alt+1, and set whether stripes show names.
///
/// Create it, pass it to [`ToolWindowLayout::controller`] and keep it. It
/// attaches when the layout first updates and detaches when the layout is
/// removed. Its changes take effect at once in the layout's state and are
/// presented at the next flush; they report [`ChangeOrigin::Application`] to
/// [`ToolWindowLayout::on_change`]. Permissions do not restrict them. Call
/// it from event handlers, command actions or outside a flush, never from a
/// component's update, layout or paint. UI-thread only.
///
/// To move focus into a tool window after expanding it, reveal and focus its
/// content with [`EventContext::request_reveal`] or
/// [`Runtime::reveal_and_focus`], which also expands it.
#[derive(Clone, Default)]
pub struct ToolWindowController {
    link: Rc<RefCell<Weak<Link>>>,
}
/// Shows only whether the controller is attached to a mounted layout.
impl std::fmt::Debug for ToolWindowController {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ToolWindowController")
            .field("attached", &self.link.borrow().upgrade().is_some())
            .finish()
    }
}
impl ToolWindowController {
    /// Creates a detached controller.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
    fn attach(&self, link: &Rc<Link>) {
        *self.link.borrow_mut() = Rc::downgrade(link);
    }
    /// Detaches from `link` when it is attached there, so a layout that no
    /// longer uses this controller cannot be driven through it.
    fn detach(&self, link: &Rc<Link>) {
        let attached = self
            .link
            .borrow()
            .upgrade()
            .is_some_and(|current| Rc::ptr_eq(&current, link));
        if attached {
            *self.link.borrow_mut() = Weak::new();
        }
    }
    /// Whether both handles are the same controller.
    fn same_as(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.link, &other.link)
    }
    fn model(&self) -> Result<Rc<RefCell<Model>>, RuntimeError> {
        self.link
            .borrow()
            .upgrade()
            .map(|link| link.model())
            .ok_or(RuntimeError::Unmounted)
    }
    fn set(&self, key: &str, expanded: Option<bool>) -> Result<bool, RuntimeError> {
        let model = self.model()?;
        model::check_writable(&model)?;
        let mut model = model.borrow_mut();
        let current = model
            .window(key)
            .map(|window| window.expanded)
            .ok_or_else(|| LayoutError::UnknownKey(key.to_owned()))?;
        Ok(model.set_expanded(key, expanded.unwrap_or(!current), ChangeOrigin::Application))
    }
    /// Expands a tool window, collapsing the one expanded in its edge group.
    /// Returns whether anything changed.
    ///
    /// # Errors
    /// Returns [`RuntimeError::Unmounted`] when no mounted layout uses this
    /// controller; the errors of [`Signal::set`](crate::Signal::set) for the
    /// layout's state write: `Closed` after the runtime shut down,
    /// `WriteDuringUpdate`, `WriteDuringLayout`, `WriteDuringPaint`,
    /// `WriteDuringInspection` or `WriteDuringDelivery` from inside that phase,
    /// or `BorrowConflict`; and [`RuntimeError::Layout`] with
    /// [`LayoutError::UnknownKey`] for a key the layout does not have.
    pub fn expand(&self, key: &str) -> Result<bool, RuntimeError> {
        self.set(key, Some(true))
    }
    /// Collapses a tool window; focus inside it returns to the center.
    /// Returns whether anything changed.
    ///
    /// # Errors
    /// As [`Self::expand`].
    pub fn collapse(&self, key: &str) -> Result<bool, RuntimeError> {
        self.set(key, Some(false))
    }
    /// Expands a collapsed tool window or collapses an expanded one. Returns
    /// whether it is expanded now.
    ///
    /// # Errors
    /// As [`Self::expand`].
    pub fn toggle(&self, key: &str) -> Result<bool, RuntimeError> {
        self.set(key, None)?;
        self.is_expanded(key)
    }
    /// Whether a tool window is expanded.
    ///
    /// # Errors
    /// Returns [`RuntimeError::Unmounted`] when no mounted layout uses this
    /// controller and [`RuntimeError::Layout`] with [`LayoutError::UnknownKey`]
    /// for a key the layout does not have.
    pub fn is_expanded(&self, key: &str) -> Result<bool, RuntimeError> {
        let model = self.model()?;
        let model = model.borrow();
        model
            .window(key)
            .map(|window| window.expanded)
            .ok_or_else(|| LayoutError::UnknownKey(key.to_owned()).into())
    }
    /// Moves a tool window to the end of `group` on `edge`. An expanded tool
    /// window stays expanded and collapses the one expanded in its new
    /// group. Returns whether anything changed.
    ///
    /// # Errors
    /// Returns `Unmounted` when no mounted layout uses this controller,
    /// [`LayoutError::UnknownKey`] for a key the layout does not have and
    /// [`LayoutError::UnavailableEdge`] for an edge whose mode is
    /// [`EdgeMode::None`].
    pub fn move_to(
        &self,
        key: &str,
        edge: ToolEdge,
        group: StripeGroup,
    ) -> Result<bool, RuntimeError> {
        let model = self.model()?;
        model::check_writable(&model)?;
        let mut model = model.borrow_mut();
        if model.window(key).is_none() {
            return Err(LayoutError::UnknownKey(key.to_owned()).into());
        }
        if *model.edges.get(edge) == EdgeMode::None {
            return Err(LayoutError::UnavailableEdge.into());
        }
        Ok(model.move_window(key, edge, group, None, ChangeOrigin::Application))
    }
    /// Moves a tool window to `position` among the other tool windows of its
    /// stripe group (0 is first; past the end makes it last). Returns
    /// whether anything changed.
    ///
    /// # Errors
    /// As [`Self::expand`].
    pub fn reorder(&self, key: &str, position: usize) -> Result<bool, RuntimeError> {
        let model = self.model()?;
        model::check_writable(&model)?;
        let mut model = model.borrow_mut();
        let (edge, group) = model
            .window(key)
            .map(|window| (window.edge, window.group))
            .ok_or_else(|| LayoutError::UnknownKey(key.to_owned()))?;
        Ok(model.move_window(key, edge, group, Some(position), ChangeOrigin::Application))
    }
    /// Presents a tool window docked, floating or in its own native window.
    /// A tool window in window mode opens its window when it is expanded,
    /// at the next flush; leaving window mode returns it from there. Returns
    /// whether anything changed.
    ///
    /// # Errors
    /// As [`Self::expand`].
    pub fn set_mode(&self, key: &str, mode: ToolWindowMode) -> Result<bool, RuntimeError> {
        let model = self.model()?;
        model::check_writable(&model)?;
        let mut model = model.borrow_mut();
        if model.window(key).is_none() {
            return Err(LayoutError::UnknownKey(key.to_owned()).into());
        }
        Ok(model.set_mode(key, mode, ChangeOrigin::Application))
    }
    /// Where a tool window is presented when expanded.
    ///
    /// # Errors
    /// As [`Self::is_expanded`].
    pub fn mode(&self, key: &str) -> Result<ToolWindowMode, RuntimeError> {
        let model = self.model()?;
        let model = model.borrow();
        model
            .window(key)
            .map(|window| window.mode)
            .ok_or_else(|| LayoutError::UnknownKey(key.to_owned()).into())
    }
    /// Switches a tool window between docked (false) and overlay (true).
    /// Returns whether anything changed.
    ///
    /// # Errors
    /// As [`Self::expand`].
    pub fn set_overlay(&self, key: &str, overlay: bool) -> Result<bool, RuntimeError> {
        let model = self.model()?;
        model::check_writable(&model)?;
        let mut model = model.borrow_mut();
        if model.window(key).is_none() {
            return Err(LayoutError::UnknownKey(key.to_owned()).into());
        }
        Ok(model.set_overlay(key, overlay, ChangeOrigin::Application))
    }
    /// Where a tool window is: its edge, its stripe group and its position
    /// among that group's tool windows.
    ///
    /// # Errors
    /// As [`Self::expand`].
    pub fn placement(&self, key: &str) -> Result<(ToolEdge, StripeGroup, usize), RuntimeError> {
        let model = self.model()?;
        let model = model.borrow();
        let window = model
            .window(key)
            .ok_or_else(|| LayoutError::UnknownKey(key.to_owned()))?;
        let position = model
            .group_keys(window.edge, window.group)
            .iter()
            .position(|other| other == key)
            .unwrap_or(0);
        Ok((window.edge, window.group, position))
    }
    /// Whether a tool window is in overlay mode.
    ///
    /// # Errors
    /// As [`Self::expand`].
    pub fn is_overlay(&self, key: &str) -> Result<bool, RuntimeError> {
        let model = self.model()?;
        let model = model.borrow();
        model
            .window(key)
            .map(|window| window.overlay)
            .ok_or_else(|| LayoutError::UnknownKey(key.to_owned()).into())
    }
    /// Sets whether stripe buttons show names beside their icons. With
    /// [`ToolWindowLayout::bind_show_names`] it writes that binding. Returns whether
    /// anything changed.
    ///
    /// # Errors
    /// Returns [`RuntimeError::Unmounted`] when no mounted layout uses this
    /// controller, and the errors of [`Signal::set`](crate::Signal::set), for
    /// the layout's state write and the bound names signal alike: `Closed`
    /// after the runtime shut down, `WriteDuringUpdate`, `WriteDuringLayout`,
    /// `WriteDuringPaint`, `WriteDuringInspection` or `WriteDuringDelivery`
    /// from inside that phase, or `BorrowConflict`, even when names have no
    /// binding.
    pub fn set_names(&self, names: bool) -> Result<bool, RuntimeError> {
        let model = self.model()?;
        model::check_writable(&model)?;
        let binding = model.borrow().names_binding.clone();
        if let Some(binding) = binding {
            return binding.set_changed(names);
        }
        let mut model = model.borrow_mut();
        if model.names == names {
            return Ok(false);
        }
        model.names = names;
        model.request();
        Ok(true)
    }
    /// Whether stripe buttons show names.
    ///
    /// # Errors
    /// Returns `Unmounted` when no mounted layout uses this controller.
    pub fn names(&self) -> Result<bool, RuntimeError> {
        Ok(self.model()?.borrow().names)
    }
}
