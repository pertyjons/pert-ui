//! Keyed retained views with a standard scrollable tab strip.
use crate::layout::rect;
use crate::*;
use std::{
    cell::{Cell, RefCell},
    collections::{BTreeMap, BTreeSet},
};

/// Owned tab description with a stable key, display title and content.
/// Titles are independent of identity; inactive content stays mounted but hidden.
#[must_use]
pub struct Tab {
    element: Element,
    axes: ScrollAxes,
    /// The replaceable leading icon slot, composed before the title when the
    /// tab becomes an element.
    leading: Option<IconSource>,
}
impl Tab {
    /// Creates a tab with automatic two-axis content scrolling and no close button.
    /// The key must not be blank and must be unique in its strip, and the
    /// title must not be blank; [`Tabs`] checks both when it is described
    /// ([`LayoutError::BlankKey`], [`LayoutError::DuplicateKey`],
    /// [`LayoutError::BlankLabel`]). Title size follows text admission.
    ///
    /// The title is a [`TextSource`]: fixed plain or rich text, or a binding
    /// the tab follows without being described again, for example a
    /// translated title or one that marks unsaved changes. A bound title is
    /// checked when it is read: a blank read keeps the last title and fails
    /// the strip's update with [`LayoutError::BlankLabel`]. A tab detached
    /// into its own window opens it with the current title, and the strip asks
    /// for the window to be retitled when the title changes
    /// ([`EventContext::set_window_title`]).
    pub fn new(key: impl Into<String>, title: impl Into<TextSource>, content: Element) -> Self {
        Self {
            element: Element::new(key, View::new(content)).with_label(title),
            axes: ScrollAxes::Both,
            leading: None,
        }
    }
    /// Fits content to the tab allocation and clips overflow instead of scrolling it.
    /// Use for layouts containing their own scroll areas or splits. Nested scroll
    /// areas still scroll and reveal focus; oversized content outside them is clipped.
    /// The default from [`Self::new`] remains automatic two-axis scrolling.
    ///
    /// ```
    /// # use pert_ui_runtime as pert_ui;
    /// use pert_ui::{Element, Label, ScrollArea, Tab, Tabs};
    /// let tabs = Tabs::new([Tab::new(
    ///     "items", "Items",
    ///     Element::new("items-scroll", ScrollArea::new(
    ///         Element::new("content", Label::new("Scrollable content")),
    ///     )),
    /// ).fixed()]);
    /// ```
    pub fn fixed(mut self) -> Self {
        self.axes = ScrollAxes::None;
        self
    }
    /// Restricts moving this one tab into its own native window with a fixed
    /// or reactive [`Permission`].
    ///
    /// Defaults to permitted, which only matters in a strip that opted into
    /// [`Tabs::detachable`]. A forbidden tab keeps the ordinary card, cursor and
    /// selection behavior and offers no drag-out; the rest of the strip is
    /// unaffected. The strip reads a bound permission in its update, so the
    /// affordance follows it without polling. Repeated calls combine
    /// restrictions. This is `Element::with_detachable` on the tab's view, so
    /// the same refusal also applies to `Runtime::request_detach`, which reads
    /// the current value when asked.
    ///
    /// ```
    /// # use pert_ui_runtime as pert_ui;
    /// use pert_ui::{Element, Label, Tab, Tabs};
    /// let tabs = Tabs::new([
    ///     Tab::new("editor", "Editor", Element::new("a", Label::new("Editable"))),
    ///     Tab::new("log", "Log", Element::new("b", Label::new("Stays docked")))
    ///         .detachable(false),
    /// ])
    /// .detachable(true);
    /// ```
    pub fn detachable(mut self, permission: impl Into<Permission>) -> Self {
        self.element = self.element.with_detachable(permission);
        self
    }
    /// Replaces the title with styled text and font icons, retaining the tab key
    /// and the [`Self::leading_icon`] slot. Icon-only content keeps the
    /// existing title as its human-readable name.
    ///
    /// ```
    /// # use pert_ui_runtime as pert_ui;
    /// use pert_ui::{Element, Icon, Label, RichText, Tab};
    /// let tab = Tab::new("settings", "Settings", Element::new("body", Label::new("Options")))
    ///     .rich_title(RichText::new().icon(Icon::Settings)).into_element();
    /// assert_eq!(tab.label(), Some("Settings"));
    /// ```
    pub fn rich_title(mut self, title: RichText) -> Self {
        self.element = self.element.with_rich_label(title);
        self
    }
    /// Replaces the leading icon slot: one icon shown before the title with
    /// the theme's icon gap, independent of the title itself. A second call
    /// replaces the icon rather than adding one, and [`Self::rich_title`]
    /// replaces the title while keeping the slot. Compose several icons in a
    /// rich title when more than one is wanted.
    ///
    /// ```
    /// # use pert_ui_runtime as pert_ui;
    /// use pert_ui::{Element, Icon, Label, RichText, Tab};
    /// let tab = Tab::new("mix", "Mix", Element::new("body", Label::new("Mixer")))
    ///     .leading_icon(Icon::Settings)
    ///     .leading_icon(Icon::Folder)
    ///     .into_element();
    /// assert_eq!(tab.label(), Some("Mix"));
    /// assert_eq!(tab.rich_label(), Some(&RichText::from("Mix").leading_icon(Icon::Folder)));
    /// ```
    pub fn leading_icon(mut self, icon: impl Into<IconSource>) -> Self {
        self.leading = Some(icon.into());
        self
    }
    /// Returns the ordinary labeled view description, including for reconciliation.
    /// Reusing its key retains state; changing its title updates metadata independently.
    pub fn into_element(mut self) -> Element {
        if self.element.label_source().is_some() {
            // A bound title is composed with its icon when it is read.
            self.element = self.element.with_label_leading(self.leading.take());
        }
        if let Some(icon) = &self.leading {
            let title = self
                .element
                .rich_label()
                .cloned()
                .unwrap_or_else(|| RichText::from(self.element.label().unwrap_or_default()));
            self.element = self
                .element
                .with_rich_label(title.leading_icon(icon.clone()));
        }
        self.element.children = self
            .element
            .children
            .into_iter()
            .map(|content| {
                Element::new("tab-viewport", ScrollArea::new(content).axes(self.axes))
                    .part("viewport")
            })
            .collect();
        self.element
    }
}
/// Local logical pixels below the strip, over the top of its own content, that
/// still count as the strip: a press does not become a drag-out there and a
/// release there detaches nothing. It lies inside the strip's own bounds, so
/// the strip is always the one asked about a drag over it.
const DRAG_OUT_MARGIN: f32 = 8.0;

/// Checks the described children of a container that draws a title for each
/// of them: every key must carry text and be unique among its siblings, and
/// every child must carry a nonblank label, or a bound one, which is checked
/// when it is read. The container runs it in its
/// constructor, while it still holds the children, and reports the result from
/// `check_description`, so an invalid description is refused before anything
/// mounts. Children handed to the mounted container as raw elements are
/// checked at its update instead, since the container never sees them as a
/// description.
pub(crate) fn check_labeled(children: &[Element]) -> Result<(), LayoutError> {
    let mut keys = BTreeSet::new();
    for child in children {
        if child.key().trim().is_empty() {
            return Err(LayoutError::BlankKey);
        }
        if !keys.insert(child.key()) {
            return Err(LayoutError::DuplicateKey);
        }
        // A bound label is checked when the container's update reads it.
        if child.label_source().is_none()
            && child.label().is_none_or(|label| label.trim().is_empty())
        {
            return Err(LayoutError::BlankLabel);
        }
    }
    Ok(())
}

/// The payload of a strip's drag-out: the tab's key. It is private, so no other
/// target accepts it and a release anywhere but the strip is unclaimed.
struct DraggedTab(String);

type Reorder = Box<dyn FnMut(&mut EventContext, &ListDrop<'_>) -> Result<bool, RuntimeError>>;
type OnSelect =
    Box<dyn FnMut(&mut EventContext, &crate::SelectionChange) -> Result<(), RuntimeError>>;

#[derive(Clone, PartialEq)]
enum Target {
    Tab(String, WidgetId),
    Previous,
    Next,
}
struct Header {
    key: String,
    id: WidgetId,
    text: TextLayout,
    rect: Rect,
}
/// Keyed views with first-tab selection, a themed tab strip (at least 36 logical pixels at Standard) and retained inactive state.
///
/// Titles use opaque cards with rounded upper corners (the theme corner radius), square
/// lower corners attached to the strip baseline and rectangular hit bounds; the selected card has a distinct fill and
/// accent underline joining the full-width baseline directly beneath the strip. Focus adds an independent silhouette outline. Default has no close buttons, and titles move only through [`Tabs::reorderable`]. Clicking a title selects on matched primary
/// release. Focused unmodified Left/Right/Home/End navigate; Ctrl+Tab (Shift reverses, no Alt or Super) bubbles
/// from content. Keyboard stepping skips detached tabs, and with every tab
/// detached the keys are ignored. The selected title scrolls into view; overflow arrows and wheel
/// scrolling expose other titles in narrow allocations. Wheel input bubbles to
/// ancestors when the title offset cannot change. Hiding cancels held input.
/// Removing or detaching the selected tab selects the nearest following tab of
/// the previously shown order that is still present and docked, otherwise the
/// nearest preceding one, resolved by key so that tabs inserted in the same
/// description do not shift it; reordering preserves selection by key. Empty tabs have no focus target.
/// A strip described with its tabs is checked before anything mounts: a blank
/// key, two tabs with one key or a blank title fail the mount or
/// reconciliation with [`LayoutError::BlankKey`], [`LayoutError::DuplicateKey`]
/// or [`LayoutError::BlankLabel`], and the mounted strip, if any, is left as it
/// was. Children handed to the strip as raw elements (`Element::children` or
/// [`Runtime::reconcile_children`]) must carry display labels too, normally from
/// `Tab::into_element`; a missing or whitespace-only label there returns
/// [`LayoutError::BlankLabel`] from the update that reads it.
/// Tab keys/titles/content are separate. Native drag-out is opt-in through
/// detachable. Detached content retains its owner.
///
/// The strip takes part in [`Workspace`](crate::Workspace) persistence as the
/// `tabs` kind: it captures the selected tab key, and each tab's content has
/// its own `viewport` part (`tabs/<tab key>/viewport`), so per-tab scroll
/// positions are captured with it. docs/contracts.md describes the
/// document.
///
/// Titles and overflow arrows use SurfaceHover while hovered, without moving
/// neighboring tabs.
///
/// A detachable strip reserves an `OpenInNew` marker slot in every title, so
/// revealing the marker never reflows the strip, including for tabs that
/// [`Tab::detachable(false)`](Tab::detachable) excludes: those offer no marker,
/// no Move cursor and no drag-out. A permitted docked title shows the marker
/// in Muted while the pointer is over it and uses the Move cursor; a title being
/// dragged out, and one whose view is already detached, draw an emptied
/// SurfaceContainer slot with a PanelBorder outline, a Muted title and the marker
/// in Primary. The marker is presentation only: the whole title stays the hit
/// area, and pressing a detached title activates its window.
///
/// Application code selects a tab by key with
/// [`Runtime::reveal_child`](crate::Runtime::reveal_child)`(&tabs, key)`, the
/// same selection transition a click on the title runs, so workspace capture
/// records it and a detached tab's window is asked for access instead; focus
/// stays where it is. The strip implements
/// [`Component::reveal_child`](crate::Component::reveal_child) through that
/// transition, so [`Runtime::reveal`](crate::Runtime::reveal) and
/// [`Runtime::reveal_and_focus`](crate::Runtime::reveal_and_focus) select the
/// tab that holds their target. While a tab is being dragged out, the strip
/// refuses to change its selection; a request for the tab already shown
/// reports [`ChildReveal::AlreadyVisible`](crate::ChildReveal::AlreadyVisible),
/// since nothing needs to change.
///
/// Describing a mounted strip again (through `Runtime::reconcile_children`)
/// applies its new [`Self::detachable`] setting and [`Self::reorderable`]
/// callback, or removes them. The selection, title scrolling, detached windows
/// and a drag session in progress are retained; the session follows the new
/// settings from its next event.
#[must_use]
pub struct Tabs {
    detached: BTreeMap<String, NativeWindowId>,
    movable: BTreeSet<String>,
    access: Option<PresentationReceipt>,
    children: Vec<Element>,
    /// The described tab keys, retained so that workspace restoration can
    /// validate a saved selection before the first update builds `order`.
    keys: Vec<String>,
    order: Vec<(String, RichText)>,
    selected: Option<String>,
    identities: BTreeMap<String, WidgetId>,
    headers: RefCell<Vec<Header>>,
    body_size: Cell<f32>,
    height: Cell<f32>,
    padding: Cell<f32>,
    minimum_width: Cell<f32>,
    arrow_width: Cell<f32>,
    icon_size: Cell<f32>,
    icon_gap: Cell<f32>,
    offset: Cell<f32>,
    total: Cell<f32>,
    viewport: Cell<Size>,
    reveal: Cell<bool>,
    focused: bool,
    armed: Option<Target>,
    /// Window-logical position of the press that armed a title, from which
    /// [`DRAG_THRESHOLD`](crate::DRAG_THRESHOLD) is measured.
    pressed_at: Option<Point>,
    /// The tab this strip's drag session carries, by key and by the view it
    /// had when the session started, from its start to its end. A tab removed
    /// and recreated under the same key is a different view and is not moved.
    dragging: Option<(String, WidgetId)>,
    /// Each tab's plain display label: the human-readable name that an
    /// icon-only rich title keeps, used for a detached window's title and the
    /// inspected tab names.
    labels: BTreeMap<String, String>,
    /// What each detached tab's window shows, so a changed title retitles it.
    window_titles: crate::window_titles::WindowTitles,
    reorder: Option<Reorder>,
    /// Local position of this strip's drag while it is over the strip and the
    /// strip is reorderable. The insertion line is derived from it at paint,
    /// so it follows the titles when the strip scrolls under a still pointer.
    drop_point: Option<Point>,
    /// Logical pixels scrolled on each tick while a reorder rests over an
    /// overflow arrow: negative towards the first title.
    autoscroll: f32,
    /// The dragged tab while a release would detach it: the session is not
    /// over the strip.
    detaching: Option<String>,
    /// The strip's detach permission, `None` until [`Self::detachable`] opts in.
    detachable: Option<Permission>,
    /// The detach permission's value at the last update.
    detach_allowed: bool,
    presentation_status: Option<PresentationStatus>,
    hover: Option<Target>,
    on_select: Option<OnSelect>,
    /// Selection changes recorded for [`Self::on_select`] and not yet
    /// delivered, in the order they happened.
    pending: Vec<Option<crate::SelectionChange>>,
    notifier: Option<crate::Notifier>,
    /// Whether the first update has chosen the initial selection, which is
    /// never reported as a change.
    started: bool,
    /// Why the described tabs are invalid, reported from `check_description`.
    invalid: Option<LayoutError>,
}
impl Tabs {
    /// Collects tabs; the first tab is initially selected unless
    /// [`Self::selected`] names another, with all view state retained.
    pub fn new(tabs: impl IntoIterator<Item = Tab>) -> Self {
        let children: Vec<Element> = tabs.into_iter().map(Tab::into_element).collect();
        let invalid = check_labeled(&children).err();
        Self {
            detached: BTreeMap::new(),
            window_titles: crate::window_titles::WindowTitles::default(),
            movable: BTreeSet::new(),
            access: None,
            keys: children
                .iter()
                .map(|child| child.key().to_owned())
                .collect(),
            children,
            order: Vec::new(),
            selected: None,
            identities: BTreeMap::new(),
            headers: RefCell::new(Vec::new()),
            body_size: Cell::new(0.0),
            height: Cell::new(36.0),
            padding: Cell::new(12.0),
            minimum_width: Cell::new(64.0),
            arrow_width: Cell::new(28.0),
            icon_size: Cell::new(16.0),
            icon_gap: Cell::new(4.0),
            offset: Cell::new(0.0),
            total: Cell::new(0.0),
            viewport: Cell::new(Size::ZERO),
            reveal: Cell::new(true),
            focused: false,
            armed: None,
            pressed_at: None,
            dragging: None,
            labels: BTreeMap::new(),
            reorder: None,
            drop_point: None,
            autoscroll: 0.0,
            detaching: None,
            detachable: None,
            detach_allowed: false,
            presentation_status: None,
            hover: None,
            on_select: None,
            pending: Vec::new(),
            notifier: None,
            started: false,
            invalid,
        }
    }
    /// Chooses the tab shown when the strip mounts, by key, instead of the
    /// first one. It applies only at mount: a restored workspace, the user and
    /// reveals decide after that, and describing the strip again with another
    /// key does not change the retained selection. The initial selection is
    /// not reported to [`Self::on_select`].
    ///
    /// The key must name one of the tabs given to [`Self::new`]; otherwise
    /// mounting or reconciling the description fails with
    /// [`LayoutError::UnknownKey`] before anything changes. A strip whose tabs
    /// are handed over as raw elements, which is what an empty `new` means,
    /// cannot check the key when it is described, so an unknown key then
    /// falls back to the first tab at the first update, as a restored
    /// selection that no longer exists would.
    ///
    /// ```
    /// # use pert_ui_runtime as pert_ui;
    /// use pert_ui::*;
    /// use std::time::Duration;
    /// fn tab(key: &str) -> Tab {
    ///     Tab::new(key, key, Element::new("body", Label::new(key)))
    /// }
    /// let mut ui = Runtime::new();
    /// ui.set_viewport(Viewport::new(400.0, 200.0, 1.0)?)?;
    /// let tabs = ui.mount_tree(Element::new(
    ///     "tabs",
    ///     Tabs::new([tab("mix"), tab("edit")]).selected("edit"),
    /// ))?;
    /// ui.flush(Duration::ZERO)?;
    /// assert!(ui.find_descendant(&tabs, "edit")?.is_some());
    /// assert!(ui.inspect().components.iter().any(|c| c.key == "edit" && c.visible));
    /// assert!(matches!(
    ///     ui.mount_tree(Element::new("other", Tabs::new([tab("mix")]).selected("log"))),
    ///     Err(RuntimeError::Layout(LayoutError::UnknownKey))
    /// ));
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn selected(mut self, key: impl Into<String>) -> Self {
        self.selected = Some(key.into());
        self
    }
    /// Enables dragging an inline tab outside its strip and releasing to detach.
    /// Defaults to false. The native host creates an independent decorated window
    /// on release; OS placement is not guaranteed and the window does not follow
    /// the pointer. Existing detached headers request access instead of
    /// duplicating content. The default App reports failed native operations;
    /// inspection retains status.
    ///
    /// The gesture is an ordinary [drag session](EventContext::start_drag), built
    /// only on the public contract. A press on a title stays a selection gesture
    /// while the pointer is over the strip or less than eight logical pixels
    /// below it, and until it has moved [`DRAG_THRESHOLD`](crate::DRAG_THRESHOLD)
    /// window pixels. Beyond both the session starts, with the rich title as its
    /// preview. The strip forbids the drop over that same area, so returning
    /// there shows the no-drop cursor and a release there detaches nothing. A
    /// release anywhere else, over other content or outside the window, ends the
    /// session as [`DragOutcome::Unclaimed`] and detaches the tab, if the view
    /// that was dragged is still that tab's. The native window takes the tab's
    /// plain label as its title, which an icon-only title keeps. The payload is
    /// private to the strip, so no other target claims it. Escape, hiding and
    /// focus loss cancel.
    ///
    /// Enabling it also turns on the drag affordance and detached presentation
    /// described on [`Tabs`], and widens every title by IconGap plus IconSize.
    /// Inspection reports `detachable`, the `detached_tabs` count and, during a
    /// drag that would currently detach on release, the `detaching` tab key.
    /// Detachment is not limited to tabs: any mounted view can move to its own
    /// window through [`EventContext::detach_view`] or [`Runtime::request_detach`],
    /// which is how an application offers a command or button instead of a drag.
    ///
    /// The strip offers detachment only after this is called; the permission
    /// may be fixed or bound, and repeated calls combine restrictions. The strip
    /// reads it in its update, so a bound permission turns the affordance on
    /// and off without polling; same-key redescription replaces it. Revoking it,
    /// or a tab's own permission ([`Tab::detachable`]), during a drag-out
    /// forbids that drop: the release detaches nothing. An already detached
    /// tab keeps its window and can still re-dock.
    pub fn detachable(mut self, permission: impl Into<Permission>) -> Self {
        let permission = match self.detachable.take() {
            Some(current) => current.and(permission),
            None => permission.into(),
        };
        if let Some(fixed) = permission.fixed() {
            self.detach_allowed = fixed;
        }
        self.detachable = Some(permission);
        self
    }
    /// Lets titles be dragged to a new place in the strip, and Alt with Left or
    /// Right move the selected tab one place, with no other modifier.
    ///
    /// The strip does not own its order: the application describes the tabs, so
    /// `on_reorder` performs the move in the application's model and describes
    /// the tabs again in the new order. It receives a [`ListDrop`] naming the
    /// moved tab's key and a [`DropPosition`] against the current keys, never an
    /// index, and returns whether it made the edit. A position next to the moved
    /// tab itself leaves the order as it is, which the application is free to
    /// ignore. Tab keys, and with them each view's retained state, are unchanged
    /// by a move. Refusing tells the drag that the drop was refused.
    ///
    /// A press on a title becomes a [drag session](EventContext::start_drag)
    /// once it has moved [`DRAG_THRESHOLD`](crate::DRAG_THRESHOLD) window
    /// pixels, with the rich title as its preview; below that it is still a
    /// click. While the drag is over the strip, or up to eight logical pixels
    /// below it, a Primary insertion line marks where the tab would go, and
    /// resting over an overflow arrow scrolls the titles. A drop there also
    /// selects the moved tab, as a click would. With [`Self::detachable`] as
    /// well, a release where nothing takes the drag still moves the tab into its
    /// own window; without it such a release does nothing. A detached tab's
    /// title cannot be dragged. Only this strip's own tabs are accepted.
    ///
    /// The callback runs on the strip, so [`EventContext::id`] names it, and
    /// [`EventContext::defer`] describes the tabs again once the event is over:
    ///
    /// ```
    /// # use pert_ui_runtime as pert_ui;
    /// use pert_ui::*;
    /// use std::{cell::RefCell, rc::Rc, time::Duration};
    /// fn tab(key: &str) -> Tab {
    ///     Tab::new(key, key, Element::new("body", Label::new(key)))
    /// }
    /// let order = Rc::new(RefCell::new(vec!["mix".to_owned(), "edit".to_owned()]));
    /// let model = order.clone();
    /// let strip = Tabs::new(order.borrow().iter().map(|key| tab(key))).reorderable(
    ///     move |cx, drop| {
    ///         let mut order = model.borrow_mut();
    ///         let moved = drop.items()[0].clone();
    ///         let from = order.iter().position(|key| *key == moved).ok_or(RuntimeError::Unmounted)?;
    ///         // Resolved before the removal: the position may name the moved tab.
    ///         let boundary = match drop.position() {
    ///             DropPosition::Before(key) => order.iter().position(|k| k == key),
    ///             DropPosition::After(key) => order.iter().position(|k| k == key).map(|i| i + 1),
    ///             _ => None,
    ///         }
    ///         .unwrap_or(order.len());
    ///         order.remove(from);
    ///         order.insert(boundary - usize::from(from < boundary), moved);
    ///         let strip = cx.id();
    ///         let tabs = order.iter().map(|key| tab(key).into_element()).collect();
    ///         let _task = cx.defer(move |ui| ui.reconcile_children(&strip, tabs))?;
    ///         Ok(true)
    ///     },
    /// );
    ///
    /// let mut ui = Runtime::new();
    /// ui.set_viewport(Viewport::new(400.0, 200.0, 1.0)?)?;
    /// let tabs = ui.mount_tree(Element::new("tabs", strip))?;
    /// ui.flush(Duration::ZERO)?;
    /// // Alt+Right moves the selected tab, the first, one place.
    /// ui.focus(&tabs)?;
    /// ui.key_input(KeyCode::AltLeft, KeyAction::Press)?;
    /// ui.key_input(KeyCode::ArrowRight, KeyAction::Press)?;
    /// ui.flush(Duration::ZERO)?;
    /// assert_eq!(*order.borrow(), ["edit", "mix"]);
    /// let keys: Vec<_> = ui.children(&tabs)?.iter().map(|id| ui.key(id).map(str::to_owned)).collect::<Result<_, _>>()?;
    /// assert_eq!(keys, ["edit", "mix"]);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn reorderable(
        mut self,
        on_reorder: impl FnMut(&mut EventContext, &ListDrop<'_>) -> Result<bool, RuntimeError> + 'static,
    ) -> Self {
        self.reorder = Some(Box::new(on_reorder));
        self
    }
    /// Reports every change of the selected tab after it took effect, with its
    /// [`ChangeOrigin`](crate::ChangeOrigin), so an application can follow the
    /// pages a user opens without asking the strip.
    ///
    /// The origins are:
    ///
    /// - [`ChangeOrigin::User`](crate::ChangeOrigin::User): a click, a key or a
    ///   reorder drop on the strip;
    /// - [`ChangeOrigin::Reveal`](crate::ChangeOrigin::Reveal): a reveal, or
    ///   [`Runtime::reveal_child`](crate::Runtime::reveal_child) selecting a
    ///   tab by key, which is how an application navigates;
    /// - [`ChangeOrigin::Restore`](crate::ChangeOrigin::Restore): workspace
    ///   restoration after the strip's first update;
    /// - [`ChangeOrigin::Redescription`](crate::ChangeOrigin::Redescription):
    ///   the selected tab was removed from the description and the strip chose
    ///   its neighbour, or tabs were described into an empty strip;
    /// - [`ChangeOrigin::Presentation`](crate::ChangeOrigin::Presentation): the
    ///   selected tab was detached into its own window and the strip chose its
    ///   neighbour, or a detached tab returned and became the selection.
    ///
    /// The initial selection, including one restored before the first update,
    /// is not a change and is not reported; neither is a request for the tab
    /// already shown, nor pressing a detached tab's title, which activates its
    /// window instead.
    ///
    /// Changes are delivered through
    /// [`Component::deliver_notifications`](crate::Component::deliver_notifications):
    /// in an event phase after the update and layout waves of the flush that
    /// follows the change have settled (or of the flush in progress), one call
    /// per change, oldest first, so several changes in one flush arrive in the
    /// order they happened. Input entry points and the reveal operations,
    /// including `Runtime::reveal_child`, settle in a flush of their own, so a
    /// change they cause is delivered (and a callback error returned) before
    /// they return. The callback runs on the strip
    /// ([`EventContext::id`] names it) and may write bindings and queue work.
    /// An error does not undo the change: the remaining changes are still
    /// delivered and the first error is returned from the flush. A strip
    /// removed before delivery reports nothing.
    ///
    /// Describing the strip again replaces the callback; changes not yet
    /// delivered go to the new one, and a description without a callback
    /// discards them.
    ///
    /// ```
    /// # use pert_ui_runtime as pert_ui;
    /// use pert_ui::*;
    /// use std::{cell::RefCell, rc::Rc, time::Duration};
    /// fn tab(key: &str) -> Tab {
    ///     Tab::new(key, key, Element::new("body", Label::new(key)))
    /// }
    /// let seen = Rc::new(RefCell::new(Vec::new()));
    /// let log = seen.clone();
    /// let strip = Tabs::new([tab("mix"), tab("edit")]).on_select(move |_, change| {
    ///     log.borrow_mut().push((change.selected.clone(), change.origin));
    ///     Ok(())
    /// });
    /// let mut ui = Runtime::new();
    /// ui.set_viewport(Viewport::new(400.0, 200.0, 1.0)?)?;
    /// let tabs = ui.mount_tree(Element::new("tabs", strip))?;
    /// ui.flush(Duration::ZERO)?;
    /// assert!(seen.borrow().is_empty()); // the initial selection is not a change
    /// ui.reveal_child(&tabs, "edit")?;
    /// ui.flush(Duration::ZERO)?;
    /// assert_eq!(*seen.borrow(), [(Some("edit".to_owned()), ChangeOrigin::Reveal)]);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn on_select(
        mut self,
        on_select: impl FnMut(&mut EventContext, &crate::SelectionChange) -> Result<(), RuntimeError>
        + 'static,
    ) -> Self {
        self.on_select = Some(Box::new(on_select));
        self
    }
    /// Records a selection change from `previous` for [`Self::on_select`]
    /// and asks for its delivery. Nothing is recorded without a callback,
    /// before the first update, or when the selection did not change.
    fn record(&mut self, previous: Option<String>, origin: crate::ChangeOrigin) {
        if self.on_select.is_none() || !self.started || previous == self.selected {
            return;
        }
        self.pending.push(Some(crate::SelectionChange::new(
            previous,
            self.selected.clone(),
            origin,
        )));
        if let Some(notifier) = &self.notifier {
            notifier.request();
        }
    }
    /// Whether a press on this title may become a drag at all.
    fn draggable(&self, key: &str) -> bool {
        !self.detached.contains_key(key) && (self.reorder.is_some() || self.movable(key))
    }
    /// The title boundary nearest a local x: `n` is before title `n`, and the
    /// count is after the last title.
    fn boundary_at(&self, x: f32) -> usize {
        let headers = self.headers.borrow();
        headers
            .iter()
            .position(|header| x < header.rect.x() + header.rect.width() / 2.0)
            .unwrap_or(headers.len())
    }
    fn position_for(&self, boundary: usize) -> DropPosition {
        match self.order.get(boundary) {
            Some((key, _)) => DropPosition::Before(key.clone()),
            None => match boundary
                .checked_sub(1)
                .and_then(|last| self.order.get(last))
            {
                Some((key, _)) => DropPosition::After(key.clone()),
                None => DropPosition::End,
            },
        }
    }
    /// How fast a reorder at local `x` scrolls the titles: over an overflow
    /// arrow, towards that arrow's end.
    fn autoscroll_at(&self, x: f32) -> f32 {
        let (inset, _) = self.header_space();
        if inset <= 0.0 {
            0.0
        } else if x < inset {
            -crate::virtual_list::AUTOSCROLL_STEP
        } else if x >= self.viewport.get().width() - inset {
            crate::virtual_list::AUTOSCROLL_STEP
        } else {
            0.0
        }
    }
    fn end_drop_indication(&mut self, cx: &mut EventContext) {
        if self.drop_point.take().is_some() {
            cx.invalidate(Invalidation::Paint);
        }
        if std::mem::take(&mut self.autoscroll) != 0.0 {
            cx.cancel_timer();
        }
    }
    /// Moves the selected tab one place through the application's callback.
    fn move_selected(&mut self, cx: &mut EventContext, forward: bool) -> Result<(), RuntimeError> {
        let Some(index) = self.selected.as_ref().and_then(|key| {
            self.order
                .iter()
                .position(|(candidate, _)| candidate == key)
        }) else {
            return Ok(());
        };
        let position = if forward {
            match self.order.get(index + 1) {
                Some((key, _)) => DropPosition::After(key.clone()),
                None => return Ok(()),
            }
        } else {
            match index
                .checked_sub(1)
                .and_then(|previous| self.order.get(previous))
            {
                Some((key, _)) => DropPosition::Before(key.clone()),
                None => return Ok(()),
            }
        };
        let key = self.order[index].0.clone();
        if let Some(reorder) = &mut self.reorder {
            reorder(
                cx,
                &ListDrop {
                    items: std::slice::from_ref(&key),
                    position,
                    operation: DropOperation::Move,
                    internal: true,
                },
            )?;
            self.reveal.set(true);
        }
        Ok(())
    }
    /// A title's selection gesture: the selection transition, and focus to the
    /// strip.
    fn select(&mut self, cx: &mut EventContext, key: String) -> Result<(), RuntimeError> {
        if self.show(cx, key, crate::ChangeOrigin::User)? == crate::ChildReveal::Changed {
            cx.request_focus();
        }
        Ok(())
    }
    /// The one selection transition, shared by title gestures and
    /// [`Component::reveal_child`]: a detached tab's window is asked for
    /// access, any other tab becomes the selection. Focus is the caller's.
    fn show(
        &mut self,
        cx: &mut EventContext,
        key: String,
        origin: crate::ChangeOrigin,
    ) -> Result<crate::ChildReveal, RuntimeError> {
        if let Some(window) = self.detached.get(&key) {
            self.access = Some(cx.activate_window(window)?);
            cx.invalidate(Invalidation::Paint);
            return Ok(crate::ChildReveal::AlreadyVisible);
        }
        if self.selected.as_ref() == Some(&key) {
            return Ok(crate::ChildReveal::AlreadyVisible);
        }
        let previous = self.selected.replace(key);
        self.reveal.set(true);
        cx.invalidate(Invalidation::Layout);
        self.notifier.get_or_insert_with(|| cx.notifier());
        self.record(previous, origin);
        Ok(crate::ChildReveal::Changed)
    }
    fn metrics(&self, cx: &MeasureContext<'_>) -> Result<Vec<TextLayout>, RuntimeError> {
        let mut changed = false;
        for (cell, role) in [
            (&self.body_size, MetricRole::BodySize),
            (&self.padding, MetricRole::ControlPaddingX),
            (&self.minimum_width, MetricRole::ControlWidth),
            (&self.arrow_width, MetricRole::ControlHeight),
            (&self.icon_size, MetricRole::IconSize),
            (&self.icon_gap, MetricRole::IconGap),
        ] {
            let value = cx.metric(role);
            changed |= cell.replace(value) != value;
        }
        if changed {
            self.reveal.set(true);
        }
        let mut height = cx.metric(MetricRole::TabHeight);
        let vertical = cx.metric(MetricRole::ControlPaddingY) * 2.0;
        let texts = self
            .order
            .iter()
            .map(|(_, title)| cx.rich_text(title, None, TextStyle::Body))
            .collect::<Result<Vec<_>, _>>()?;
        for text in &texts {
            height = height.max(text.size().height() + vertical);
        }
        if self.height.replace(height) != height {
            self.reveal.set(true);
        }
        Ok(texts)
    }
    /// Width of the trailing detach marker slot, reserved for every title in a
    /// detachable strip so revealing the marker on hover never reflows the strip.
    fn marker_slot(&self) -> f32 {
        if self.detach_allowed {
            self.icon_gap.get() + self.icon_size.get()
        } else {
            0.0
        }
    }
    fn title_width(&self, text: &TextLayout) -> f32 {
        (text.size().width() + 2.0 * self.padding.get() + self.marker_slot())
            .max(self.minimum_width.get())
    }
    /// Whether this tab currently offers the drag-out gesture at all.
    fn movable(&self, key: &str) -> bool {
        self.detach_allowed && self.movable.contains(key) && !self.detached.contains_key(key)
    }
    /// Whether a local point is on the strip or within `DRAG_OUT_MARGIN` below
    /// it, where a drag-out has not begun and a release does not detach. The
    /// area lies inside this component's bounds, so a drag over it is always
    /// offered to the strip, which forbids the drop there.
    fn near_strip(&self, point: Point) -> bool {
        point.y() >= 0.0
            && point.y() <= self.height.get() + DRAG_OUT_MARGIN
            && point.x() >= 0.0
            && point.x() <= self.viewport.get().width()
    }
    fn header_space(&self) -> (f32, f32) {
        let width = self.viewport.get().width();
        if self.total.get() > width {
            (
                self.arrow_width.get().min(width / 2.0),
                (width - 2.0 * self.arrow_width.get()).max(0.0),
            )
        } else {
            (0.0, width)
        }
    }
    fn target(&self, point: Point) -> Option<Target> {
        let width = self.viewport.get().width();
        if point.y() < 0.0
            || point.y() >= self.height.get()
            || point.x() < 0.0
            || point.x() >= width
        {
            return None;
        }
        let (inset, _) = self.header_space();
        if inset > 0.0 && point.x() < inset {
            return Some(Target::Previous);
        }
        if inset > 0.0 && point.x() >= width - inset {
            return Some(Target::Next);
        }
        self.headers
            .borrow()
            .iter()
            .find(|header| header.rect.contains(point))
            .map(|header| Target::Tab(header.key.clone(), header.id.clone()))
    }
    fn scroll(&self, cx: &mut EventContext, delta: f32) -> bool {
        let (_, width) = self.header_space();
        let next = (self.offset.get() + delta).clamp(0.0, (self.total.get() - width).max(0.0));
        if self.offset.replace(next) != next {
            self.reveal.set(false);
            cx.invalidate(Invalidation::Layout);
            true
        } else {
            false
        }
    }
    /// The human-readable name of the tab at `index` in display order: its
    /// plain label, which an icon-only rich title keeps.
    fn name(&self, index: usize) -> String {
        let (key, title) = &self.order[index];
        self.labels
            .get(key)
            .cloned()
            .unwrap_or_else(|| title.plain_text())
    }
}
impl Tabs {
    fn resolved_selection(&self, keys: &[String], detached: &BTreeSet<String>) -> Option<String> {
        let usable = |key: &String| keys.contains(key) && !detached.contains(key);
        if let Some(key) = self.selected.as_ref().filter(|key| usable(key)) {
            return Some(key.clone());
        }
        let old = self.selected.as_ref().and_then(|key| {
            self.order
                .iter()
                .position(|(candidate, _)| candidate == key)
        });
        let neighbour = old.and_then(|index| {
            self.order[index + 1..]
                .iter()
                .chain(self.order[..index].iter().rev())
                .map(|(key, _)| key)
                .find(|key| usable(key))
        });
        neighbour
            .or_else(|| keys.iter().find(|key| usable(key)))
            .cloned()
    }
    fn restore_selection(
        &mut self,
        state: &crate::ViewState,
        keys: &[String],
        detached: &BTreeSet<String>,
    ) -> Result<Invalidation, crate::RestoreRejection> {
        use crate::RestoreRejection;
        state.expect_only(&["selected"])?;
        if self.armed.is_some() || self.dragging.is_some() {
            return Err(RestoreRejection::Busy);
        }
        if state.get("selected").is_none() && keys.iter().all(|key| detached.contains(key)) {
            let previous = self.selected.take();
            self.reveal.set(true);
            self.record(previous, crate::ChangeOrigin::Restore);
            return Ok(Invalidation::Layout);
        }
        let key = state.text("selected")?;
        let described = keys.iter().any(|candidate| candidate == key);
        // A detached tab has its own window; selecting it inline would show
        // nothing, so the entry is reported instead of leaving an empty strip.
        if !described || detached.contains(key) {
            return Err(RestoreRejection::Unresolved("selected".to_owned()));
        }
        let previous = self.selected.replace(key.to_owned());
        self.reveal.set(true);
        self.record(previous, crate::ChangeOrigin::Restore);
        Ok(Invalidation::Layout)
    }
}

impl Component for Tabs {
    fn check_description(&self) -> Result<(), RuntimeError> {
        // The children were taken when the element was built; the
        // constructor checked them and kept their keys.
        if let Some(error) = self.invalid {
            return Err(error.into());
        }
        if let Some(key) = &self.selected
            && !self.keys.is_empty()
            && !self.keys.contains(key)
        {
            return Err(LayoutError::UnknownKey.into());
        }
        Ok(())
    }
    fn reconcile(&mut self, next: Self) -> Option<Invalidation> {
        // Selection, scrolling, detached windows and any drag session are local
        // state. A session in progress consults the new settings at its next
        // event and when it ends.
        let reorderable = next.reorder.is_some();
        let layout = !match (&self.detachable, &next.detachable) {
            (Some(mine), Some(theirs)) => mine.same_as(theirs),
            (None, None) => true,
            _ => false,
        };
        let paint = self.reorder.is_some() != reorderable;
        self.keys = next.keys;
        if layout {
            self.detachable = next.detachable;
            // A bound permission is read at the update this schedules.
            self.detach_allowed = self
                .detachable
                .as_ref()
                .and_then(Permission::fixed)
                .unwrap_or(false);
        }
        self.reorder = next.reorder;
        self.on_select = next.on_select;
        if self.on_select.is_none() {
            // Keep one slot per outstanding runtime request. Old requests must
            // not consume a newer change after the callback is reinstalled.
            self.pending.fill(None);
        }
        if !reorderable {
            // No insertion line without a reorder; a pending autoscroll tick
            // finds nothing to do and stops.
            self.drop_point = None;
            self.autoscroll = 0.0;
        }
        if layout {
            Some(Invalidation::Layout)
        } else if paint {
            Some(Invalidation::Paint)
        } else {
            None
        }
    }
    fn workspace_state(&self) -> Result<Option<crate::ViewState>, crate::WorkspaceError> {
        let state = crate::ViewState::new("tabs")?;
        // Before the first update the selection is still implicit: the described
        // order decides it, exactly as `update` will.
        let selected = self.selected.clone().or_else(|| {
            self.order
                .first()
                .map(|(key, _)| key.clone())
                .or_else(|| self.keys.first().cloned())
        });
        Ok(Some(match selected {
            Some(key) => state.with("selected", crate::WorkspaceValue::text(key)?)?,
            None => state,
        }))
    }
    fn workspace_state_with_children(
        &self,
        children: &[ChildInfo],
    ) -> Result<Option<crate::ViewState>, crate::WorkspaceError> {
        let keys = children
            .iter()
            .map(|child| child.key().to_owned())
            .collect::<Vec<_>>();
        let detached = children
            .iter()
            .filter(|child| child.detached_window().is_some())
            .map(|child| child.key().to_owned())
            .collect();
        let state = crate::ViewState::new("tabs")?;
        Ok(Some(match self.resolved_selection(&keys, &detached) {
            Some(key) => state.with("selected", crate::WorkspaceValue::text(key)?)?,
            None => state,
        }))
    }
    fn restore_workspace_state(
        &mut self,
        state: &crate::ViewState,
    ) -> Result<Invalidation, crate::RestoreRejection> {
        let keys = if self.order.is_empty() {
            self.keys.clone()
        } else {
            self.order.iter().map(|(key, _)| key.clone()).collect()
        };
        let detached = self.detached.keys().cloned().collect();
        self.restore_selection(state, &keys, &detached)
    }
    fn restore_workspace_state_with_children(
        &mut self,
        state: &crate::ViewState,
        children: &[ChildInfo],
    ) -> Result<Invalidation, crate::RestoreRejection> {
        let keys = children
            .iter()
            .map(|child| child.key().to_owned())
            .collect::<Vec<_>>();
        let detached = children
            .iter()
            .filter(|child| child.detached_window().is_some())
            .map(|child| child.key().to_owned())
            .collect();
        self.restore_selection(state, &keys, &detached)
    }
    fn child_redocked(&mut self, cx: &mut EventContext, child: &WidgetId) {
        if let Some((key, _)) = self.identities.iter().find(|(_, id)| *id == child) {
            let previous = self.selected.replace(key.clone());
            self.reveal.set(true);
            cx.invalidate(Invalidation::Layout);
            self.notifier.get_or_insert_with(|| cx.notifier());
            self.record(previous, crate::ChangeOrigin::Presentation);
        }
    }
    fn hover_changed(&mut self, cx: &mut EventContext, hovered: bool) {
        if !hovered && self.hover.take().is_some() {
            cx.invalidate(Invalidation::Paint);
        }
    }
    fn take_children(&mut self) -> Vec<Element> {
        std::mem::take(&mut self.children)
    }
    fn child_visible(&self, key: &str) -> bool {
        self.selected.as_deref() == Some(key)
    }
    fn reveal_child(
        &mut self,
        cx: &mut EventContext,
        key: &str,
    ) -> Result<crate::ChildReveal, RuntimeError> {
        // The runtime asks only about a mounted child. A tab being dragged out
        // keeps the strip as it is, as workspace restoration does.
        if self.dragging.is_some() {
            return Ok(crate::ChildReveal::Refused);
        }
        self.show(cx, key.to_owned(), crate::ChangeOrigin::Reveal)
    }
    fn deliver_notifications(&mut self, cx: &mut EventContext) -> Result<(), RuntimeError> {
        self.window_titles.deliver(cx)?;
        // `record` requests one delivery per change, so each call delivers the
        // oldest change; that keeps this strip's changes in order with other
        // components' changes made in between.
        if self.pending.is_empty() {
            return Ok(());
        }
        let Some(change) = self.pending.remove(0) else {
            return Ok(());
        };
        match &mut self.on_select {
            Some(on_select) => on_select(cx, &change),
            None => Ok(()),
        }
    }
    fn focusable(&self) -> bool {
        !self.order.is_empty()
    }
    fn measure_limits(&self, cx: &MeasureContext<'_>) -> Result<SizeLimits, RuntimeError> {
        self.metrics(cx)?;
        if self.order.is_empty() {
            Ok(SizeLimits::default())
        } else {
            Ok(SizeLimits::at_least(Size::new(96.0, self.height.get())?))
        }
    }
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        self.notifier.get_or_insert_with(|| cx.notifier());
        if let Some(receipt) = &self.access {
            self.presentation_status = Some(cx.read_paint(&receipt.status_binding())?);
        }
        self.detached = cx
            .children()
            .iter()
            .filter_map(|child| {
                child
                    .detached_window()
                    .map(|window| (child.key().to_owned(), window))
            })
            .collect();
        self.detach_allowed = match &self.detachable {
            Some(permission) => permission.read(cx)?,
            None => false,
        };
        // Each tab's own permission is read here too, so a bound one moves the
        // affordance without polling; reconciling fixed metadata already
        // invalidates the strip's layout.
        let permissions: Vec<(String, Permission)> = cx
            .children()
            .iter()
            .map(|child| (child.key().to_owned(), child.detach_permission().clone()))
            .collect();
        self.movable = BTreeSet::new();
        for (key, permission) in permissions {
            if permission.read(cx)? {
                self.movable.insert(key);
            }
        }
        self.identities = cx
            .children()
            .iter()
            .map(|child| (child.key().to_owned(), child.id().clone()))
            .collect();
        let labels = cx
            .children()
            .iter()
            .filter_map(|child| Some((child.key().to_owned(), child.label()?.to_owned())))
            .collect();
        let order: Vec<_> = cx
            .children()
            .iter()
            .map(|child| {
                Ok((child.key().to_owned(), {
                    let label = child
                        .label()
                        .filter(|label| !label.trim().is_empty())
                        .ok_or(LayoutError::BlankLabel)?;
                    child
                        .rich_label()
                        .cloned()
                        .unwrap_or_else(|| RichText::from(label))
                }))
            })
            .collect::<Result<_, RuntimeError>>()?;
        // Only valid titles replace the last ones, which a detached tab's
        // window keeps showing.
        self.labels = labels;
        let views: Vec<_> = self
            .detached
            .iter()
            .filter_map(|(key, window)| {
                Some((key.clone(), window.clone(), self.labels.get(key)?.clone()))
            })
            .collect();
        if self.window_titles.update(cx, views)?
            && let Some(notifier) = &self.notifier
        {
            notifier.request();
        }
        let keys = order.iter().map(|(key, _)| key.clone()).collect::<Vec<_>>();
        let detached = self.detached.keys().cloned().collect();
        let replacement = self.resolved_selection(&keys, &detached);
        if self.selected != replacement {
            // A selection still described was detached; anything else left
            // the description (or the strip was empty).
            let origin = match &self.selected {
                Some(key) if order.iter().any(|(candidate, _)| candidate == key) => {
                    crate::ChangeOrigin::Presentation
                }
                _ => crate::ChangeOrigin::Redescription,
            };
            let previous = std::mem::replace(&mut self.selected, replacement);
            self.reveal.set(true);
            self.record(previous, origin);
        }
        if self.order != order {
            self.order = order;
            self.reveal.set(true);
        }
        self.focused = cx.focused();
        self.started = true;
        Ok(())
    }
    fn measure(
        &self,
        cx: &MeasureContext<'_>,
        available: Constraints,
    ) -> Result<Size, RuntimeError> {
        let texts = self.metrics(cx)?;
        if self.order.is_empty() {
            return Ok(Size::ZERO);
        }
        let body = if cx.is_empty() {
            Size::ZERO
        } else {
            cx.measure(
                0,
                Constraints::new(
                    available.width(),
                    available
                        .height()
                        .map(|height| (height - self.height.get()).max(0.0)),
                )?,
            )?
        };
        let mut titles = 0.0;
        for text in &texts {
            titles += self.title_width(text);
        }
        Ok(Size::new(
            available
                .width()
                .unwrap_or(body.width().max(titles.min(480.0))),
            body.height() + self.height.get(),
        )?)
    }
    fn arrange(&self, cx: &mut ArrangeContext<'_>, size: Size) -> Result<(), RuntimeError> {
        let texts = self.metrics(cx.measurement())?;
        if cx.len() > 1 {
            return Err(LayoutError::InvalidChild.into());
        }
        let mut headers = Vec::with_capacity(self.order.len());
        let mut x = 0.0;
        for ((key, _), text) in self.order.iter().zip(texts) {
            let width = self.title_width(&text);
            headers.push(Header {
                key: key.clone(),
                id: self
                    .identities
                    .get(key)
                    .ok_or(LayoutError::InvalidChild)?
                    .clone(),
                text,
                rect: rect(x, 0.0, width, self.height.get())?,
            });
            x += width;
        }
        if !x.is_finite() {
            return Err(crate::GeometryError::NonFinite.into());
        }
        self.total.set(x);
        if self.viewport.replace(size).width() != size.width() {
            self.reveal.set(true);
        }
        let (inset, width) = self.header_space();
        let mut offset = self.offset.get().min((x - width).max(0.0));
        if self.reveal.replace(false)
            && let Some(header) = headers
                .iter()
                .find(|header| self.selected.as_ref() == Some(&header.key))
        {
            let [x, _, w, _] = header.rect.values();
            if x < offset {
                offset = x;
            } else if x + w > offset + width {
                offset = (x + w - width).min(x);
            }
        }
        self.offset.set(offset);
        for header in &mut headers {
            let [x, y, w, h] = header.rect.values();
            header.rect = rect(x - offset + inset, y, w, h)?;
        }
        *self.headers.borrow_mut() = headers;
        if !cx.is_empty() {
            cx.place(
                0,
                rect(
                    0.0,
                    self.height.get(),
                    size.width(),
                    (size.height() - self.height.get()).max(0.0),
                )?,
            )?;
        }
        Ok(())
    }
    fn pointer_input(
        &mut self,
        cx: &mut EventContext,
        event: PointerEvent,
    ) -> Result<InputOutcome, RuntimeError> {
        // An unarranged component has no origin to subtract, and nothing to hit.
        let Some(point) = cx.to_local(event.position()) else {
            return Ok(InputOutcome::Ignored);
        };
        let target = if event.is_outside() || !cx.pointer_inside() {
            None
        } else {
            self.target(point)
        };
        if self.hover != target {
            self.hover = target.clone();
            cx.invalidate(Invalidation::Paint);
        }
        match event.kind() {
            PointerKind::Press(PointerButton::Primary) if target.is_some() => {
                self.armed = target;
                self.pressed_at = Some(event.position());
                cx.capture_pointer()?;
                cx.invalidate(Invalidation::Paint);
            }
            // A reorderable strip starts its session once the press has moved
            // far enough; one that only detaches waits until the pointer has
            // also left the strip, where the press stays a selection gesture.
            // The distance is measured in window pixels, since it describes the
            // hand; the strip's own area is local, and may be scaled by a view.
            PointerKind::Move
                if (self.reorder.is_some() || !self.near_strip(point))
                    && !cx.drag_active()
                    && self.pressed_at.is_some_and(|start| {
                        (event.position().x() - start.x()).hypot(event.position().y() - start.y())
                            >= DRAG_THRESHOLD
                    })
                    && let Some(Target::Tab(key, view)) = &self.armed
                    && self.draggable(key) =>
            {
                let (key, view) = (key.clone(), view.clone());
                let title = self
                    .order
                    .iter()
                    .find(|(candidate, _)| *candidate == key)
                    .map(|(_, title)| title.clone())
                    .ok_or(RuntimeError::Unmounted)?;
                self.armed = None;
                self.pressed_at = None;
                cx.start_drag(
                    DragPayload::new(DraggedTab(key.clone())),
                    DropOperations::MOVE,
                    DragPreview::element(Element::new("tab", Label::new(title))),
                )?;
                // A reorderable drag starts over the strip; the first delivery
                // of the session decides what a release there would do.
                self.detaching = (self.reorder.is_none()).then(|| key.clone());
                self.dragging = Some((key, view));
                cx.invalidate(Invalidation::Paint);
                return Ok(InputOutcome::Handled);
            }
            PointerKind::Release(PointerButton::Primary) if self.armed.is_some() => {
                let armed = self.armed.take();
                self.pressed_at = None;
                cx.release_pointer();
                if armed == target {
                    match armed {
                        Some(Target::Tab(key, _)) => self.select(cx, key)?,
                        Some(Target::Previous) => {
                            self.scroll(cx, -self.header_space().1 * 0.75);
                        }
                        Some(Target::Next) => {
                            self.scroll(cx, self.header_space().1 * 0.75);
                        }
                        None => {}
                    }
                }
                cx.invalidate(Invalidation::Paint);
            }
            PointerKind::Scroll(delta) if point.y() < self.height.get() && point.y() >= 0.0 => {
                return Ok(
                    if self.scroll(
                        cx,
                        if delta.x() == 0.0 {
                            delta.y()
                        } else {
                            delta.x()
                        },
                    ) {
                        InputOutcome::Handled
                    } else {
                        InputOutcome::Ignored
                    },
                );
            }
            PointerKind::Move | PointerKind::Leave => {
                return Ok(if target.is_some() {
                    InputOutcome::Handled
                } else {
                    InputOutcome::Ignored
                });
            }
            _ => return Ok(InputOutcome::Ignored),
        }
        Ok(InputOutcome::Handled)
    }
    fn keyboard_event(
        &mut self,
        cx: &mut EventContext,
        event: KeyEvent,
        keyboard: &KeyboardState,
    ) -> Result<InputOutcome, RuntimeError> {
        let control = keyboard.control_key(event);
        if !matches!(
            event.transition,
            KeyTransition::Pressed | KeyTransition::Repeated
        ) || self.order.is_empty()
        {
            return Ok(InputOutcome::Ignored);
        }
        if control == KeyCode::Escape && self.armed.is_some() {
            self.cancel_input(cx);
            cx.invalidate(Invalidation::Paint);
            cx.release_pointer();
            return Ok(InputOutcome::Handled);
        }
        let modifiers = keyboard.modifiers();
        // The keyboard equivalent of dragging the selected title one place.
        if self.reorder.is_some()
            && cx.focused()
            && modifiers.alt
            && !modifiers.shift
            && !modifiers.control
            && !modifiers.super_key
            && matches!(control, KeyCode::ArrowLeft | KeyCode::ArrowRight)
        {
            self.move_selected(cx, control == KeyCode::ArrowRight)?;
            return Ok(InputOutcome::Handled);
        }
        let current = self
            .selected
            .as_ref()
            .and_then(|key| {
                self.order
                    .iter()
                    .position(|(candidate, _)| candidate == key)
            })
            .unwrap_or(0);
        let len = self.order.len();
        // Stepping skips detached tabs, as the removal fallback does: selecting
        // one would only activate its window and leave the selection in place,
        // so the next press would land on it again.
        let docked = |index: &usize| !self.detached.contains_key(&self.order[*index].0);
        let forward = |from: usize| (1..=len).map(move |step| (from + step) % len);
        let backward = |from: usize| (1..=len).map(move |step| (from + len - step % len) % len);
        let index = if control == KeyCode::Tab
            && modifiers.control
            && !modifiers.alt
            && !modifiers.super_key
        {
            if modifiers.shift {
                backward(current).find(docked)
            } else {
                forward(current).find(docked)
            }
        } else if cx.focused() && modifiers == Modifiers::default() {
            match control {
                KeyCode::ArrowLeft => backward(current).find(docked),
                KeyCode::ArrowRight => forward(current).find(docked),
                KeyCode::Home => (0..len).find(docked),
                KeyCode::End => (0..len).rev().find(docked),
                _ => return Ok(InputOutcome::Ignored),
            }
        } else {
            return Ok(InputOutcome::Ignored);
        };
        let Some(index) = index else {
            // Every tab is detached: there is no inline title to step to.
            return Ok(InputOutcome::Ignored);
        };
        self.select(cx, self.order[index].0.clone())?;
        Ok(InputOutcome::Handled)
    }
    fn input_reset(&mut self) {
        self.drop_point = None;
        // A pending autoscroll tick finds nothing to do and stops.
        self.autoscroll = 0.0;
        self.armed = None;
        self.pressed_at = None;
        self.detaching = None;
        self.hover = None;
        self.focused = false;
    }
    fn drag_event(
        &mut self,
        cx: &mut EventContext,
        event: &DragEvent<'_>,
    ) -> Result<DropResponse, RuntimeError> {
        // Only this strip's own drag concerns it.
        if event.role() != DragRole::Candidate
            || event.source() != &cx.id()
            || event
                .payload()
                .get::<DraggedTab>()
                .is_none_or(|tab| self.dragging.as_ref().map(|(key, _)| key) != Some(&tab.0))
        {
            return Ok(DropResponse::Rejected);
        }
        let point = cx
            .to_local(event.position())
            .filter(|point| event.phase() != DragPhase::Leave && self.near_strip(*point));
        let detaching = match &self.dragging {
            Some((key, _)) if point.is_none() && self.movable(key) => Some(key.clone()),
            _ => None,
        };
        if self.detaching != detaching {
            self.detaching = detaching;
            cx.invalidate(Invalidation::Paint);
        }
        let Some(point) = point.filter(|_| self.reorder.is_some()) else {
            self.end_drop_indication(cx);
            // Back on a strip that only detaches, a release does nothing, which
            // the no-drop cursor says.
            return Ok(if point.is_some() {
                DropResponse::Forbidden
            } else {
                DropResponse::Rejected
            });
        };
        if event.phase() == DragPhase::Drop {
            self.end_drop_indication(cx);
            let Some((key, _)) = self.dragging.clone() else {
                return Ok(DropResponse::Rejected);
            };
            let position = self.position_for(self.boundary_at(point.x()));
            let Some(reorder) = &mut self.reorder else {
                return Ok(DropResponse::Rejected);
            };
            let performed = reorder(
                cx,
                &ListDrop {
                    items: std::slice::from_ref(&key),
                    position,
                    operation: DropOperation::Move,
                    internal: true,
                },
            )?;
            return Ok(if performed {
                DropResponse::Accepted(DropOperation::Move)
            } else {
                DropResponse::Rejected
            });
        }
        if self.drop_point.replace(point) != Some(point) {
            cx.invalidate(Invalidation::Paint);
        }
        let speed = self.autoscroll_at(point.x());
        if speed != 0.0 && self.autoscroll == 0.0 {
            cx.set_timer(crate::virtual_list::AUTOSCROLL_INTERVAL)?;
        } else if speed == 0.0 && self.autoscroll != 0.0 {
            cx.cancel_timer();
        }
        self.autoscroll = speed;
        Ok(DropResponse::Accepted(DropOperation::Move))
    }
    fn timer_elapsed(&mut self, cx: &mut EventContext) -> Result<(), RuntimeError> {
        if self.autoscroll == 0.0 || !cx.drag_active() {
            self.autoscroll = 0.0;
            return Ok(());
        }
        let before = self.offset.get();
        self.scroll(cx, self.autoscroll);
        if self.offset.get() == before {
            self.autoscroll = 0.0;
        } else {
            // The titles move under a still pointer; the insertion line is
            // derived from them at paint, so it follows.
            cx.set_timer(crate::virtual_list::AUTOSCROLL_INTERVAL)?;
        }
        Ok(())
    }
    fn drag_finished(
        &mut self,
        cx: &mut EventContext,
        outcome: DragOutcome,
    ) -> Result<(), RuntimeError> {
        self.detaching = None;
        self.end_drop_indication(cx);
        cx.invalidate(Invalidation::Paint);
        let Some((key, view)) = self.dragging.take() else {
            return Ok(());
        };
        // A drop in the strip selects the moved tab, as a click would, so a
        // press that wavered past the threshold still selects.
        if matches!(outcome, DragOutcome::Dropped(_)) {
            if self.order.iter().any(|(candidate, _)| *candidate == key) {
                self.select(cx, key)?;
            }
            return Ok(());
        }
        // Released where nothing claimed it: the tab moves to its own window,
        // provided the same view is still here and still permitted to.
        if outcome != DragOutcome::Unclaimed
            || !self.movable(&key)
            || self.identities.get(&key) != Some(&view)
        {
            return Ok(());
        }
        let Some(title) = self.labels.get(&key).cloned() else {
            return Ok(());
        };
        self.select(cx, key)?;
        self.access = Some(cx.detach_view(&view, NativeWindowOptions::new(title)?)?);
        Ok(())
    }
    fn cursor(&self, point: Point) -> Option<CursorIcon> {
        match self.target(point)? {
            // Only a title that can be dragged offers to be.
            Target::Tab(key, _) if self.draggable(&key) => Some(CursorIcon::Move),
            _ => None,
        }
    }
    fn paint(&self, painter: &mut Painter) -> Result<(), DrawError> {
        let size = self.viewport.get();
        if self.order.is_empty() {
            return Ok(());
        }
        painter.fill_rect(
            Rect::new(0.0, 0.0, size.width(), self.height.get())?,
            painter.color(ColorRole::Surface),
        );
        let border = painter
            .metric(MetricRole::BorderWidth)
            .min(self.height.get());
        painter.fill_rect(
            Rect::new(0.0, self.height.get() - border, size.width(), border)?,
            painter.color(ColorRole::Divider),
        );
        let (inset, width) = self.header_space();
        painter.with_clip(
            Rect::new(inset, 0.0, width, self.height.get())?,
            |painter| {
                for header in self.headers.borrow().iter() {
                    let [x, _, w, _] = header.rect.values();
                    if x + w <= inset || x >= inset + width {
                        continue;
                    }
                    let selected = self.selected.as_ref() == Some(&header.key);
                    let detached = self.detached.contains_key(&header.key);
                    let leaving = self.detaching.as_deref() == Some(header.key.as_str());
                    let hovered =
                        self.hover == Some(Target::Tab(header.key.clone(), header.id.clone()));
                    let color = if leaving || detached {
                        // An emptied slot reads as static content, not as a card.
                        painter.color(ColorRole::SurfaceContainer)
                    } else if selected {
                        painter.color(ColorRole::TabSelected)
                    } else if hovered {
                        painter.color(ColorRole::SurfaceHover)
                    } else {
                        painter.color(ColorRole::Tab)
                    };
                    let gap = painter.metric(MetricRole::TabGap).min(w);
                    let rect = Rect::new(
                        x + gap / 2.0,
                        0.0,
                        (w - gap).max(0.0),
                        self.height.get() - border,
                    )?;
                    // Rounded above, square where the tab meets the strip divider.
                    let shape =
                        Shape::RoundedTop(Length::new(painter.metric(MetricRole::CornerRadius))?);
                    painter.fill_shape(rect, shape, color)?;
                    if leaving || detached {
                        painter.stroke_shape(
                            rect,
                            shape,
                            Length::new(border)?,
                            painter.color(ColorRole::PanelBorder),
                        )?;
                    }
                    let [x, _, w, _] = header.rect.values();
                    header.text.paint(
                        painter,
                        x + self.padding.get(),
                        ((self.height.get() - border - header.text.size().height()) / 2.0).max(0.0),
                        painter.color(if detached || leaving {
                            ColorRole::Muted
                        } else {
                            ColorRole::OnTab
                        }),
                    )?;
                    let side = self
                        .icon_size
                        .get()
                        .min(w - 2.0 * self.padding.get())
                        .min(self.height.get() - border);
                    let offered = detached || leaving || (hovered && self.movable(&header.key));
                    if self.detach_allowed && offered && side > 0.0 {
                        painter.icon(
                            Icon::OpenInNew,
                            Rect::new(
                                x + w - self.padding.get() - side,
                                (self.height.get() - border - side) / 2.0,
                                side,
                                side,
                            )?,
                            painter.color(if detached || leaving {
                                ColorRole::Primary
                            } else {
                                ColorRole::Muted
                            }),
                        )?;
                    }
                    if selected {
                        painter.fill_rect(
                            Rect::new(x, (self.height.get() - 3.0).max(0.0), w, 3.0)?,
                            painter.color(ColorRole::Primary),
                        );
                    }
                    if self.focused && selected {
                        painter.focus_ring(rect, shape)?;
                    }
                }
                if let Some(point) = self.drop_point {
                    // Where the dragged title would go, between two titles.
                    let headers = self.headers.borrow();
                    let boundary = self.boundary_at(point.x());
                    let edge = match headers.get(boundary) {
                        Some(header) => Some(header.rect.x()),
                        None => headers
                            .last()
                            .map(|header| header.rect.x() + header.rect.width()),
                    };
                    if let Some(edge) = edge {
                        let thickness = painter.metric(MetricRole::FocusWidth);
                        let x = (edge - thickness / 2.0)
                            .clamp(inset, (inset + width - thickness).max(inset));
                        painter.fill_rect(
                            Rect::new(x, 0.0, thickness, self.height.get() - border)?,
                            painter.color(ColorRole::Primary),
                        );
                    }
                }
                Ok::<(), DrawError>(())
            },
        )?;
        if inset > 0.0 {
            let side = painter
                .metric(MetricRole::IconSize)
                .min(inset)
                .min(self.height.get());
            for (base, icon, target) in [
                (0.0, Icon::ChevronLeft, Target::Previous),
                (size.width() - inset, Icon::ChevronRight, Target::Next),
            ] {
                if self.hover == Some(target) {
                    painter.fill_shape(
                        Rect::new(base, 0.0, inset, self.height.get() - border)?,
                        Shape::Rounded(Length::new(painter.metric(MetricRole::CornerRadius))?),
                        painter.color(ColorRole::SurfaceHover),
                    )?;
                }
                painter.icon(
                    icon,
                    Rect::new(
                        base + (inset - side) / 2.0,
                        (self.height.get() - side) / 2.0,
                        side,
                        side,
                    )?,
                    painter.color(ColorRole::OnTab),
                )?;
            }
        }
        Ok(())
    }
    fn inspect(&self) -> ComponentDetails {
        let mut details = ComponentDetails::default();
        details
            .properties
            .insert("detachable".into(), InspectValue::Bool(self.detach_allowed));
        details.properties.insert(
            "reorderable".into(),
            InspectValue::Bool(self.reorder.is_some()),
        );
        details.properties.insert(
            "scroll_x".into(),
            InspectValue::Number(f64::from(self.offset.get())),
        );
        details.properties.insert(
            "detached_tabs".into(),
            InspectValue::Number(self.detached.len() as f64),
        );
        if self.detach_allowed {
            details.properties.insert(
                "movable_tabs".into(),
                InspectValue::Number(
                    self.order
                        .iter()
                        .filter(|(key, _)| self.movable(key) || self.detached.contains_key(key))
                        .count() as f64,
                ),
            );
        }
        if let Some(key) = &self.detaching {
            details
                .properties
                .insert("detaching".into(), InspectValue::Text(key.clone()));
        }
        if let Some(status) = &self.presentation_status {
            details.properties.insert(
                "presentation_status".into(),
                InspectValue::Text(format!("{status:?}")),
            );
        }
        if let Some(key) = &self.selected {
            details
                .properties
                .insert("selected".into(), InspectValue::Text(key.clone()));
        }
        let selected = self
            .selected
            .as_ref()
            .and_then(|key| self.order.iter().position(|(other, _)| other == key));
        details.semantics = Some(
            Semantics::new(SemanticRole::TabList)
                .value(SemanticValue::Choice {
                    selected,
                    count: self.order.len(),
                    label: selected.map(|index| self.name(index)),
                })
                .items(self.order.iter().enumerate().map(|(index, (key, _))| {
                    let item = SemanticItem::new(self.name(index))
                        .selected(selected == Some(index))
                        .action(SemanticAction::Select);
                    if self.draggable(key) {
                        item.action(SemanticAction::Move)
                    } else {
                        item
                    }
                }))
                .enabled_actions((!self.order.is_empty()).then_some(SemanticAction::Select)),
        );
        details.properties.insert(
            "tab_count".into(),
            InspectValue::Number(self.order.len() as f64),
        );
        details
    }
}
