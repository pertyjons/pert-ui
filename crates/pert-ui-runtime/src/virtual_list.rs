//! A list or grid that mounts only the items near its viewport.

use std::{
    cell::{Cell, Ref, RefCell},
    rc::Rc,
};

use crate::{
    ArrangeContext, ColorRole, ComponentDetails, Constraints, DragEvent, DragOutcome, DragPayload,
    DragPhase, DragPreview, DragRole, DrawError, DropOperation, DropOperations, DropPosition,
    DropResponse, Element, EventContext, InputOutcome, InspectValue, Invalidation, KeyCode,
    KeyEvent, KeyTransition, KeyboardState, Label, LayoutError, Length, ListDragItems, ListDrop,
    ListSelection, MeasureContext, MetricRole, Orientation, Painter, Permission, Point,
    PointerButton, PointerEvent, PointerKind, Rect, SemanticAction, SemanticRole, SemanticValue,
    Semantics, Shape, Size, UpdateContext, VirtualAxis,
    list_selection::Gesture,
    runtime::{Component, RuntimeError},
    scrollbar,
    signal::{ReadSignal, ReceiptStatus, Signal},
};
use std::collections::BTreeSet;

/// The rows a [`VirtualList`] can show, addressed by position and by stable key.
///
/// The list reads a [`ListSource`] from a binding and asks it for the
/// count, the key at a position and the row at a position; it never iterates
/// every row. Keys are the row identity: a row that keeps its key across a
/// change of the binding keeps its retained component, state and focus, a row
/// whose key disappears is unmounted, and two rows must never share a key.
/// The list keys the element `build` returns with `key(index)`, replacing the
/// element's own key, so a row can be built with any key.
///
/// Implementations are UI-local and read-only during an update; replacing the
/// bound `Rc` is how content changes, and it invalidates only the rows whose
/// keys or positions changed. A list scrolled away from the start keeps the
/// row at the top of its viewport in place by key across a replacement, when
/// the new source still has it. Implement `index_of` in better than linear time
/// when a host reveals rows by key in large collections.
///
/// An implementation is `'static`, as [`ListSource::new`] requires: it owns its
/// rows or shares them through an `Rc`, never borrowing from a local or a
/// component (see [`Component::content_rows`]).
pub trait ListItems {
    /// Number of rows.
    fn len(&self) -> usize;
    /// Whether there are no rows.
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
    /// Stable key of the row at `index`, or `None` past the end.
    fn key(&self, index: usize) -> Option<String>;
    /// The row at `index`, or `None` past the end. The list keys it with
    /// `key(index)`; its own key is replaced.
    ///
    /// The list calls it when a row mounts and again for every mounted row
    /// when the source is replaced, and the new description is applied to the
    /// row's retained components. `build` gets no state owner, so a control in
    /// the row that binds a signal gets it in one of two ways:
    ///
    /// - **Per row.** Create the binding here with [`Signal::sibling`] from a
    ///   signal the source holds, and write changes back to the application's
    ///   records from the control's callback. Creating a signal is not a write,
    ///   so it is allowed here. Each replacement of the source gives every
    ///   mounted row a new binding, which suits a control without a draft, such
    ///   as a checkbox; a text editor ends its open draft when its binding
    ///   changes.
    /// - **Retained.** Return a row component that creates its binding once,
    ///   in its first update, and requests the control as its child with
    ///   [`UpdateContext::request_children`]. It keeps the record's value from each new description in
    ///   [`Component::reconcile`] and, after a [`Notifier`](crate::Notifier)
    ///   request, writes it to its binding in
    ///   [`Component::deliver_notifications`], since neither an update nor a
    ///   reconciliation may write. A draft then survives changes to other
    ///   rows.
    fn build(&self, index: usize) -> Option<Element>;
    /// Position of the row with `key`, or `None` when no row has it. The default
    /// scans every position.
    fn index_of(&self, key: &str) -> Option<usize> {
        (0..self.len()).find(|&index| self.key(index).as_deref() == Some(key))
    }
    /// Searchable text of the row at `index`, or `None` when the row offers
    /// none. The default offers none, so content search finds such a row only
    /// while it is mounted, through its widgets' semantics.
    ///
    /// [Content search](crate::Runtime::content_search) asks for it for rows
    /// that are not mounted, and filters can share it: it must be
    /// deterministic, free of side effects and I/O, and answered from data the
    /// source already holds. Its cost is the source's, once per row a search
    /// examines.
    fn text(&self, index: usize) -> Option<String> {
        let _ = index;
        None
    }
}

/// A shared, replaceable set of rows for a [`VirtualList`], bound through a signal.
///
/// Two sources are equal when they share the same allocation, so writing the
/// same source back to a signal changes nothing, and wrapping a new
/// implementation is what tells the list its content changed. The source is
/// UI-local and shared by reference; a host keeps its own data model and
/// implements [`ListItems`] over it.
#[derive(Clone)]
pub struct ListSource(Rc<dyn ListItems>);

impl ListSource {
    /// Wraps a row set.
    pub fn new(items: impl ListItems + 'static) -> Self {
        Self(Rc::new(items))
    }
    // For an owner that also keeps the rows it published, such as a tree.
    pub(crate) fn from_shared(items: Rc<dyn ListItems>) -> Self {
        Self(items)
    }
}

impl PartialEq for ListSource {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

impl std::ops::Deref for ListSource {
    type Target = dyn ListItems;
    fn deref(&self) -> &Self::Target {
        self.0.as_ref()
    }
}

/// The row at `index` keyed with the source's key for it, as the list mounts
/// it, or `None` when the source has no such row.
fn keyed_row(items: &ListSource, index: usize) -> Option<Element> {
    let key = items.key(index)?;
    let mut element = items.build(index)?;
    element.key = key;
    Some(element)
}

impl core::fmt::Debug for ListSource {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("ListSource")
            .field("len", &self.len())
            .finish()
    }
}

/// Reveals rows of a [`VirtualList`] by key from application code.
///
/// Created with [`Self::new`] and handed to the list with
/// [`VirtualList::controller`]. A reveal is a request: the list resolves the key
/// through [`ListItems::index_of`] at its next update, scrolls so the row is
/// inside the viewport, and mounts it. A list serves only requests made after
/// it was given the controller. A controller supports one mounted list at a
/// time, including while hidden; clones address that same list, not a group of
/// lists. It may outlive
/// that list and later be attached to another. Requests are UI-local signals, so they may be issued from event
/// handlers and application code, never from a component's `update`.
///
/// Each request returns a [`ListRevealReceipt`] that reports whether the row was
/// found and, once it is mounted, its [`WidgetId`](crate::WidgetId), so a jump
/// can go on to [`Runtime::reveal_and_focus`](crate::Runtime::reveal_and_focus)
/// the row. A row's widget does not exist until the list mounts it, which is
/// why collections keep this keyed request instead of taking a `WidgetId`.
///
/// ```
/// use pert_ui_runtime::prelude::*;
/// use pert_ui_runtime::{ListController, ListItems, ListRevealStatus, ListSource, VirtualList};
/// use std::time::Duration;
///
/// struct Rows;
/// impl ListItems for Rows {
///     fn len(&self) -> usize { 1000 }
///     fn key(&self, index: usize) -> Option<String> { Some(format!("row-{index}")) }
///     fn build(&self, index: usize) -> Option<Element> {
///         Some(Element::new(format!("row-{index}"), Label::new(format!("Row {index}"))))
///     }
///     fn index_of(&self, key: &str) -> Option<usize> {
///         key.strip_prefix("row-")?.parse().ok().filter(|index| *index < 1000)
///     }
/// }
///
/// let mut ui = Runtime::new();
/// ui.set_viewport(Viewport::new(300.0, 200.0, 1.0)?)?;
/// let controller = ListController::new(&ui);
/// let items = ui.signal(ListSource::new(Rows));
/// ui.mount_tree(Element::new(
///     "list",
///     VirtualList::new(items.read_only(), 20.0)?.controller(controller.clone()),
/// ))?;
/// ui.flush(Duration::ZERO)?;
///
/// let receipt = controller.reveal("row-900")?;
/// ui.flush(Duration::ZERO)?;
/// let ListRevealStatus::Mounted(row) = receipt.status() else { panic!("row not mounted") };
/// assert_eq!(ui.key(&row)?, "row-900");
///
/// let missing = controller.reveal("row-5000")?;
/// ui.flush(Duration::ZERO)?;
/// assert_eq!(missing.status(), ListRevealStatus::NotFound);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Clone)]
pub struct ListController {
    reveal: Signal<Option<RevealRequest>>,
}

/// One keyed reveal request with the receipt it completes.
#[derive(Clone)]
struct RevealRequest {
    serial: u64,
    key: String,
    receipt: ListRevealReceipt,
    /// Whether a list over the controller has taken the request.
    claimed: Rc<Cell<bool>>,
}
impl PartialEq for RevealRequest {
    fn eq(&self, other: &Self) -> bool {
        self.serial == other.serial && self.key == other.key
    }
}

/// Completion of one keyed reveal in a [`VirtualList`], `Tree` or `Table`.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum ListRevealStatus {
    /// No list over the controller has served the request yet, or the row is
    /// found and waits to be mounted. A hidden list serves it when shown.
    Pending,
    /// The row is mounted with this identity and lies inside the viewport.
    Mounted(crate::WidgetId),
    /// The key names no row the collection shows: unknown, or inside a
    /// collapsed tree node.
    NotFound,
    /// A newer request on the same controller replaced this one before it
    /// completed.
    Superseded,
    /// The list serving the request was removed before the row was mounted,
    /// or its mounted list was removed, for example while hidden, before
    /// reading the request.
    Cancelled,
}

/// UI-local completion of one [`ListController::reveal`] request, also
/// returned by `TreeController::reveal` and `TableController::reveal`.
///
/// [`Self::status`] reads the owned status, which stays readable after
/// runtime shutdown; [`Self::status_binding`] mirrors it as a binding, so a
/// component observes completion without polling. Receipts compare equal when
/// they report the same request. Discarding a receipt leaves the request in
/// place; it only gives up its outcome, so a caller that does not need it
/// discards it explicitly (`let _ = controller.reveal(key)?;`).
#[derive(Clone)]
#[must_use = "the receipt is the only report of whether the row was found and mounted"]
pub struct ListRevealReceipt(ReceiptStatus<ListRevealStatus>);
impl ListRevealReceipt {
    /// Pending or the retained terminal result, readable after shutdown.
    /// Reading creates no subscription or timer.
    #[must_use]
    pub fn status(&self) -> ListRevealStatus {
        self.0.get()
    }
    /// The same status as a binding, so a component that reads it during
    /// update is invalidated when the request completes. Like every binding it
    /// reads `Err(RuntimeError::Closed)` after runtime shutdown; use
    /// [`Self::status`] then.
    pub fn status_binding(&self) -> ReadSignal<ListRevealStatus> {
        self.0.binding()
    }
    /// Records a terminal status unless one is already recorded.
    fn finish(&self, status: ListRevealStatus) {
        if self.0.get() == ListRevealStatus::Pending {
            self.0.set(status);
        }
    }
}
impl PartialEq for ListRevealReceipt {
    fn eq(&self, other: &Self) -> bool {
        self.0.same(&other.0)
    }
}
impl Eq for ListRevealReceipt {}
/// Shows the current status, as [`ListRevealReceipt::status`] reads it.
impl std::fmt::Debug for ListRevealReceipt {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ListRevealReceipt")
            .field("status", &self.status())
            .finish()
    }
}

impl ListController {
    /// Creates a controller belonging to `ui`.
    ///
    /// `ui` is any [`StateOwner`](crate::StateOwner): the
    /// [`Runtime`](crate::Runtime) while the application builds its tree, or
    /// the [`UpdateContext`](crate::UpdateContext) or
    /// [`EventContext`](crate::EventContext) of a component that builds this
    /// while it describes its children.
    #[must_use]
    pub fn new(ui: &impl crate::StateOwner) -> Self {
        Self {
            reveal: ui.signal(None),
        }
    }

    /// Asks the list to scroll to and mount the row with `key`, and returns a
    /// receipt for the outcome.
    ///
    /// The receipt becomes [`ListRevealStatus::NotFound`] at the list's next
    /// update for an unknown key, and [`ListRevealStatus::Mounted`] once the
    /// row is mounted, usually within the same flush. A newer request on this
    /// controller supersedes an earlier one that has not completed.
    ///
    /// # Errors
    /// The errors of [`Signal::set`](crate::Signal::set): `Closed` after the
    /// runtime shut down; `WriteDuringUpdate`, `WriteDuringLayout`,
    /// `WriteDuringPaint`, `WriteDuringInspection` or `WriteDuringDelivery`
    /// from inside that phase; or `BorrowConflict`.
    pub fn reveal(&self, key: impl Into<String>) -> Result<ListRevealReceipt, RuntimeError> {
        let previous = self.reveal.get()?;
        let serial = previous
            .as_ref()
            .map_or(1, |request| request.serial.wrapping_add(1));
        let receipt = ListRevealReceipt(ReceiptStatus::new(
            self.reveal.read_only().shared().clone(),
            ListRevealStatus::Pending,
        ));
        self.reveal.set(Some(RevealRequest {
            serial,
            key: key.into(),
            receipt: receipt.clone(),
            claimed: Rc::default(),
        }))?;
        if let Some(previous) = previous {
            previous.receipt.finish(ListRevealStatus::Superseded);
        }
        Ok(receipt)
    }
}

/// A list or grid that mounts only the items near its viewport, plus a bounded
/// overscan, and keeps the logical extent of every item.
///
/// It is vertical and single-lane by default, with a fixed item extent from
/// [`VirtualList::new`] or measured items from [`VirtualList::estimated`];
/// [`VirtualList::lanes`] and [`VirtualList::orientation`] make it a grid or
/// horizontal.
///
/// # A bounded extent
///
/// The list mounts the rows inside the extent it is allocated, so it needs a
/// bounded height (or width, along a horizontal axis). Measured without a
/// bound, it asks for its whole content extent, and a parent that grants it
/// mounts every row. A window or detached view root, a [`SplitPane`](crate::SplitPane)
/// pane, a [`Container`](crate::Container) or [`CollectionFilter`](crate::CollectionFilter)
/// with a bounded allocation, a [`Tab::fixed`](crate::Tab::fixed) tab, a
/// [`RegionLayout::fixed`](crate::RegionLayout::fixed) region, a
/// [`View::fixed`](crate::View::fixed) view and a grown child of a bounded
/// [`Column`](crate::Column), which shrinks to what its siblings leave, bound
/// it. The default [`Tab::new`](crate::Tab::new) and
/// [`RegionLayout`](crate::RegionLayout) content and a
/// [`ScrollArea`](crate::ScrollArea) do not, since they scroll their content at
/// its natural size. A bounded [`Row`](crate::Row) bounds a vertical list's
/// height, which it offers to every child, but not a horizontal list's width:
/// a row's children keep their natural widths. In debug builds validation reports a list of at least 100 rows that
/// was given its whole extent while less than half of it is visible, as
/// [`ValidationCode::UnboundedCollection`](crate::ValidationCode::UnboundedCollection).
///
/// The list is its own scroll viewport: it scrolls by wheel, by the keyboard
/// when focused (arrows, Page Up/Down, Home, End) and by its overlay scrollbar,
/// and it honors the [`Component::reveal`] contract, so `Runtime::focus` and
/// keyboard traversal scroll a mounted row into view. Rows that are not mounted
/// take part in neither; application code reaches them through a
/// [`ListController`], which scrolls to a key and mounts it.
///
/// A row that has keyboard focus or captures the pointer stays mounted while it
/// is outside the mounted range, so focus, IME composition and a gesture in
/// progress survive scrolling away; it is unmounted at the list's next update
/// once neither holds, which a focus move triggers by itself. Rows
/// keep their identity across scrolling: a row that stays in range keeps its
/// component and state, and one that leaves and returns is built again from
/// [`ListItems::build`].
///
/// [`VirtualList::lanes`] makes it a grid: items fill lanes in order, each line
/// holds that many items, and every item gets an equal share of the cross extent.
/// [`VirtualList::orientation`] makes it scroll along x instead, with the same
/// rules and a horizontal scrollbar; a wheel with only a vertical axis scrolls it
/// too. The window arithmetic is the public [`VirtualAxis`].
///
/// Without a selection the list is a scrolling viewport: unmodified arrows,
/// Page Up/Down, Home and End scroll, and it takes focus only while it overflows. Attaching a
/// [`ListSelection`] with [`VirtualList::selection`] makes the list one Tab stop
/// that navigates by item position, which crosses items that are not mounted:
/// arrows move the active item by one item or one line, Home/End go to the ends,
/// Page Up/Down move by the viewport's whole lines, and the list scrolls the
/// active item into view and mounts it. In
/// [`SelectionMode::Single`](crate::SelectionMode::Single) the selection follows
/// the active item; in [`SelectionMode::Multiple`](crate::SelectionMode::Multiple)
/// Space toggles it, Shift extends a range from the anchor and Control moves
/// without selecting. Navigation chords with Super, or with Alt other than
/// [reordering](VirtualList::reorderable), are left to command bindings. A primary press on an item that the item's own content did
/// not handle makes it active and selects it with the same modifiers, Control
/// toggling. Enter runs [`VirtualList::on_activate`]. Selected items get a
/// TabSelected fill beneath their content, and the active item the
/// [`ColorRole::FocusRing`] ring while the list has focus. An interactive row uses SurfaceHover
/// beneath its content while hovered, with selection taking precedence.
/// Interactive content inside an item keeps its own input, hover feedback and
/// Tab stop. With one lane the arrows across the
/// axis are left unhandled, so an owner such as a tree can give them a meaning.
///
/// Each item is placed in its lane's share of the cross extent, along the axis
/// where its line starts in the list's [`VirtualAxis`], and the content extent
/// is the axis's. With a fixed extent every line is that long, an item's own
/// measurement does not change its slot, and an item whose minimum is longer is
/// clipped to it; with [`VirtualList::estimated`] each mounted line is as long
/// as its longest item. Positions along the axis are kept in `f64`, so a
/// collection far longer than `f32` holds to the pixel still places every item
/// exactly. Mounting and unmounting happen through
/// [`UpdateContext::request_children`] after the list has seen its allocation,
/// so the first frame after a resize or a scroll may run one extra update wave
/// inside the same flush; nothing is scheduled once the range settles.
///
/// ```
/// use pert_ui_runtime::{Element, Label, ListItems, ListSource, Runtime, VirtualList};
///
/// struct Numbers(usize);
/// impl ListItems for Numbers {
///     fn len(&self) -> usize {
///         self.0
///     }
///     fn key(&self, index: usize) -> Option<String> {
///         (index < self.0).then(|| format!("n-{index}"))
///     }
///     fn build(&self, index: usize) -> Option<Element> {
///         self.key(index).map(|key| Element::new(key, Label::new(format!("Row {index}"))))
///     }
///     fn index_of(&self, key: &str) -> Option<usize> {
///         key.strip_prefix("n-")?.parse().ok().filter(|index| *index < self.0)
///     }
/// }
///
/// let mut ui = Runtime::new();
/// ui.set_viewport(pert_ui_runtime::Viewport::new(300.0, 200.0, 1.0)?)?;
/// let items = ui.signal(ListSource::new(Numbers(100_000)));
/// let list = ui.mount_tree(Element::new(
///     "list",
///     VirtualList::new(items.read_only(), 24.0)?,
/// ))?;
/// ui.flush(std::time::Duration::ZERO)?;
/// // Only the rows near the 200-pixel viewport are mounted, not 100,000.
/// assert!(ui.children(&list)?.len() < 40);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
///
/// Describing a mounted list again keeps everything the first description
/// gave it: source binding, axis, lanes, overscan, controller, selection and
/// options, and also the [`on_activate`](Self::on_activate),
/// [`reorderable`](Self::reorderable) and [`placement`](Self::placement)
/// callbacks, the [`reorder_permission`](Self::reorder_permission) binding and
/// the [`drag_operations`](Self::drag_operations). A later description's
/// values are ignored; the list's data and state belong to the bindings, not
/// to its description. Change rows through the source binding and the
/// controller, make a callback read application state that changes, or give
/// the list a new key to replace it.
#[must_use]
pub struct VirtualList {
    items: ReadSignal<ListSource>,
    /// Interior-mutable because arrangement records measured extents.
    axis: RefCell<VirtualAxis>,
    horizontal: bool,
    overscan: usize,
    controller: Option<ListController>,
    selection: Option<ListSelection>,
    activate: Option<Activate>,
    on_selection: Option<OnSelection>,
    /// An owner's switch for observing the selection, for a callback it
    /// installs before it knows whether its own user asks for changes.
    observe_gate: Option<Rc<Cell<bool>>>,
    /// The selection as last reported, or as first seen; `None` before the
    /// first update with a callback.
    selection_seen: Option<(Option<String>, Rc<BTreeSet<String>>)>,
    /// Changes recorded for [`Self::on_selection_change`] and not yet delivered.
    selection_pending: Vec<crate::ListSelectionChange>,
    notifier: Option<crate::Notifier>,
    outline: bool,
    /// Whether interactive content inside items takes part in Tab traversal.
    item_stops: bool,
    /// The key of an item kept mounted outside the range ([`Self::keep_mounted`]).
    keep: Option<ReadSignal<Option<String>>>,
    on_text: Option<OnText>,
    snapshot: Option<ListSource>,
    /// Whether row construction completed for the retained source snapshot.
    rows_current: bool,
    /// The scroll offset along the axis. `f64`, like the axis's positions, so
    /// that a long collection keeps every row distinct and a small scroll moves.
    offset: Cell<f64>,
    viewport: Cell<Size>,
    bar_extents: Cell<scrollbar::Extents>,
    /// The items requested at the last update, in order: `(index, key)`.
    mounted: Vec<(usize, String)>,
    range: (usize, usize),
    needed: Cell<Option<(usize, usize)>>,
    reveal_seen: Unserved,
    revealing: Revealing,
    drag: Option<f32>,
    focused: bool,
    active: Option<String>,
    active_index: Option<usize>,
    /// The selection's keys, shared with it: an update reads them without
    /// copying, and only mounted keys are looked up when painting.
    selected: Rc<BTreeSet<String>>,
    reorder: Option<Reorder>,
    reorder_permission: Permission,
    reorder_allowed: bool,
    reordering: bool,
    drag_operations: DropOperations,
    /// A primary press on an item that may still become a drag.
    armed: Option<Armed>,
    /// How a drop over the list is shown: the insertion line or the item it
    /// would go into.
    mark: Option<ListDropMark>,
    /// An owner's rule for where a drop goes, in place of item boundaries.
    placer: Option<Placer>,
    /// Logical pixels scrolled on each tick while a drag rests near an edge.
    autoscroll: f32,
    /// The pointer rests over the scrollbar, presented as draggable.
    bar_hover: bool,
    /// Local pointer position, so scrolling changes which row is highlighted
    /// even when the pointer does not move.
    hovered_point: Option<Point>,
}

type Activate = Box<dyn FnMut(&mut EventContext, &str) -> Result<(), RuntimeError>>;
type OnText = Box<dyn FnMut(&mut EventContext, &str) -> Result<(), RuntimeError>>;
type OnSelection =
    Box<dyn FnMut(&mut EventContext, &crate::ListSelectionChange) -> Result<(), RuntimeError>>;
type Reorder = Box<dyn FnMut(&mut EventContext, &ListDrop<'_>) -> Result<bool, RuntimeError>>;
/// A placement rule: the dragged keys and the pointer in, where a drop goes
/// out, or `None` where the rule forbids one.
type Placer = Box<dyn Fn(&[String], &ListDropPoint) -> Option<ListDropPlacement>>;

/// The pointer over a [reorderable](VirtualList::reorderable) list during a
/// drag, as a [placement rule](VirtualList::placement) sees it.
///
/// It names the item under the pointer twice: by its position in the rows the
/// list shows, which is where a mark is drawn, and by its key. The
/// application's model may have changed since those rows were built, so a rule
/// resolves the key against its own model to decide the position and marks by
/// the shown position.
#[derive(Clone, Debug, PartialEq)]
pub struct ListDropPoint {
    index: Option<usize>,
    key: Option<String>,
    fraction: f32,
    across: f32,
}

impl ListDropPoint {
    /// The shown position of the item under the pointer, clamped into the
    /// list: the last item past the end, and `None` only in an empty list.
    #[must_use]
    pub const fn index(&self) -> Option<usize> {
        self.index
    }
    /// The key of that item in the rows the list shows; `None` in an empty
    /// list.
    #[must_use]
    pub fn key(&self) -> Option<&str> {
        self.key.as_deref()
    }
    /// How far along that item the pointer is, along the scrolling axis: from
    /// 0 at its start to 1 at its end, and 1 past the last item.
    #[must_use]
    pub const fn fraction(&self) -> f32 {
        self.fraction
    }
    /// The pointer's position across the scrolling axis, in the list's local
    /// logical pixels: x in a vertical list, y in a horizontal one. It may lie
    /// outside the list while a drag rests beside it.
    #[must_use]
    pub const fn across(&self) -> f32 {
        self.across
    }
}

/// Where a drop goes and how the list shows it, as a
/// [placement rule](VirtualList::placement) answers.
#[derive(Clone, Debug, PartialEq)]
pub struct ListDropPlacement {
    position: DropPosition,
    mark: ListDropMark,
}

impl ListDropPlacement {
    /// A drop at `position`, shown with `mark`.
    #[must_use]
    pub const fn new(position: DropPosition, mark: ListDropMark) -> Self {
        Self { position, mark }
    }
    /// Where the drop goes; the reorder callback receives it in its
    /// [`ListDrop`].
    #[must_use]
    pub const fn position(&self) -> &DropPosition {
        &self.position
    }
    /// How the list shows it.
    #[must_use]
    pub const fn mark(&self) -> ListDropMark {
        self.mark
    }
}

/// The indication a list draws for a drop over it, by shown position.
///
/// Positions are the rows the list shows, as [`ListDropPoint::index`] reports
/// them. A boundary past the last item draws at the list's end, and an item
/// outside the viewport draws nothing visible, so a stale mark cannot fail.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ListDropMark {
    /// A Primary insertion line at an item boundary.
    Line {
        /// The boundary before the item at this position; the item count is
        /// the boundary after the last item.
        boundary: usize,
        /// Logical pixels the line starts into the lane, for nesting such as
        /// a tree's levels. Only a single-lane vertical list indents; zero,
        /// negative and NaN draw the full lane, and the indent never exceeds
        /// it.
        indent: f32,
    },
    /// The item at this position is outlined: the drop goes into it.
    Into(usize),
}

struct Armed {
    /// Window position of the press; the threshold describes the hand.
    origin: Point,
    /// The pressed item's key. The source may be replaced before the press
    /// becomes a drag or a release, so its position is resolved again then.
    key: String,
    /// The press landed on an already selected item, so narrowing the selection
    /// waits for a release that did not become a drag.
    deferred: bool,
}

// A drag resting this close to an edge of an overflowing list scrolls it.
pub(crate) const AUTOSCROLL_ZONE: f32 = 24.0;
// The fastest autoscroll, reached at the edge itself, per tick.
pub(crate) const AUTOSCROLL_STEP: f32 = 12.0;
pub(crate) const AUTOSCROLL_INTERVAL: std::time::Duration = std::time::Duration::from_millis(30);

impl VirtualList {
    /// The lines mounted before and after the visible ones by default.
    pub const DEFAULT_OVERSCAN: usize = 4;

    /// Creates a vertical single-lane list over a binding of rows with a fixed
    /// logical row height.
    ///
    /// `items` is a binding of a [`ListSource`], not the [`ListItems`] value:
    /// wrap the rows with [`ListSource::new`] in a signal and pass the signal,
    /// or pass a derived binding such as
    /// [`ListFilter::items`](crate::ListFilter::items).
    ///
    /// # Errors
    /// Returns [`GeometryError::NonFinite`](crate::GeometryError::NonFinite)
    /// for a row height that is not finite,
    /// [`GeometryError::Negative`](crate::GeometryError::Negative) for a
    /// negative one and [`GeometryError::Empty`](crate::GeometryError::Empty)
    /// for zero.
    pub fn new(
        items: impl Into<ReadSignal<ListSource>>,
        row_height: f32,
    ) -> Result<Self, crate::GeometryError> {
        let items: ReadSignal<ListSource> = items.into();
        Ok(Self::over(items, VirtualAxis::new(row_height)?))
    }

    /// Creates a vertical single-lane list whose rows have their own heights.
    ///
    /// Each row is `estimate` tall until it is mounted; the list then measures it
    /// at the width of its lane and records the result in its
    /// [`VirtualAxis`], so the content extent and the scrollbar become exact for
    /// every row that has been seen and stay an estimate for the rest. A row
    /// measured above the viewport corrects the offset by its difference, which
    /// keeps what is on screen where it was while scrolling upwards. A line of a
    /// grid is as tall as its tallest item. Replacing the source forgets every
    /// measurement, since a position may then hold another row. Measuring adds
    /// one measurement per mounted row to each arrangement, and a changed height
    /// may cost one more update wave inside the same flush.
    ///
    /// # Errors
    /// Returns [`GeometryError::NonFinite`](crate::GeometryError::NonFinite)
    /// for an estimate that is not finite,
    /// [`GeometryError::Negative`](crate::GeometryError::Negative) for a
    /// negative one and [`GeometryError::Empty`](crate::GeometryError::Empty)
    /// for zero.
    pub fn estimated(
        items: impl Into<ReadSignal<ListSource>>,
        estimate: f32,
    ) -> Result<Self, crate::GeometryError> {
        let items: ReadSignal<ListSource> = items.into();
        Ok(Self::over(items, VirtualAxis::estimated(estimate)?))
    }

    fn over(items: ReadSignal<ListSource>, axis: VirtualAxis) -> Self {
        Self {
            items,
            axis: RefCell::new(axis),
            horizontal: false,
            overscan: Self::DEFAULT_OVERSCAN,
            controller: None,
            selection: None,
            on_selection: None,
            observe_gate: None,
            selection_seen: None,
            selection_pending: Vec::new(),
            notifier: None,
            activate: None,
            outline: true,
            item_stops: true,
            keep: None,
            on_text: None,
            snapshot: None,
            rows_current: false,
            offset: Cell::new(0.0),
            viewport: Cell::new(Size::ZERO),
            bar_extents: Cell::new(scrollbar::Extents::STANDARD),
            mounted: Vec::new(),
            range: (0, 0),
            needed: Cell::new(None),
            reveal_seen: Unserved::default(),
            revealing: Revealing::default(),
            drag: None,
            focused: false,
            active: None,
            active_index: None,
            selected: Rc::default(),
            reorder: None,
            reorder_permission: Permission::allowed(),
            reorder_allowed: true,
            reordering: false,
            drag_operations: DropOperations::MOVE,
            armed: None,
            mark: None,
            placer: None,
            bar_hover: false,
            hovered_point: None,
            autoscroll: 0.0,
        }
    }

    /// Sets how many lines beyond the visible ones stay mounted on each side, so
    /// small scrolls reuse items instead of mounting. Zero is allowed.
    pub fn overscan(mut self, lines: usize) -> Self {
        self.overscan = lines;
        self
    }

    /// Sets how many items share a line, which makes the list a grid. The item
    /// extent given to [`VirtualList::new`] stays each line's extent along the
    /// scrolling axis, and each item gets an equal share of the cross extent.
    ///
    /// # Errors
    /// Returns [`LayoutError::NoLanes`] for zero lanes.
    pub fn lanes(mut self, lanes: usize) -> Result<Self, LayoutError> {
        self.axis = RefCell::new(self.axis.into_inner().with_lanes(lanes)?);
        Ok(self)
    }

    /// Selects the scrolling axis; vertical is the default. Horizontally, the
    /// extent given to [`VirtualList::new`] is each item's width.
    pub fn orientation(mut self, orientation: Orientation) -> Self {
        self.horizontal = orientation == Orientation::Horizontal;
        self
    }

    /// Attaches a controller whose reveal requests this list serves.
    ///
    /// The list serves requests made from here on. A request made earlier,
    /// which another list with the same controller may already have served,
    /// is not replayed, so a list rebuilt with a long-lived controller opens
    /// where it starts rather than at an old target.
    pub fn controller(mut self, controller: ListController) -> Self {
        // A controller whose runtime is gone has nothing to replay either.
        self.reveal_seen = Unserved {
            seen: controller
                .reveal
                .get()
                .ok()
                .flatten()
                .map_or(0, |request| request.serial),
            source: Some(controller.reveal.clone()),
        };
        self.controller = Some(controller);
        self
    }

    /// Attaches application-owned navigation and selection state, which makes the
    /// list one Tab stop that navigates by item; see the type documentation.
    pub fn selection(mut self, selection: ListSelection) -> Self {
        self.selection = Some(selection);
        self
    }

    /// Whether the active item gets the list's outline while it has focus; true
    /// by default. An owner that marks the active item itself, as a table does
    /// for its active cell, turns it off.
    pub fn active_outline(mut self, outline: bool) -> Self {
        self.outline = outline;
        self
    }

    /// Whether interactive content inside items, such as a checkbox in a
    /// row, is a stop of Tab traversal; true by default. With false the
    /// items' content leaves traversal through
    /// [`Component::descendant_tab_stops`], so Tab moves past the list in one
    /// step; pointer presses and explicit focus still reach the content. The
    /// list keeps its own focusability: with a selection it is one stop while
    /// it has items, without one only while it overflows. An owner that turns
    /// this off gives keyboard users another way to reach the content.
    pub fn item_tab_stops(mut self, stops: bool) -> Self {
        self.item_stops = stops;
        self
    }

    /// Keeps the item whose key `key` holds mounted while the source holds
    /// it, as the list keeps an item that holds focus or a pointer gesture:
    /// scrolled out of the range it stays mounted, unrevealed and unpainted.
    /// `None` keeps nothing extra. An owner that hosts state in one item, such
    /// as a table cell's editor that must outlive focus, sets it for as long
    /// as that state lives. The list reads the signal in its update, so a
    /// change applies at the next flush; an item the source no longer holds
    /// is unmounted. Set at construction; describing the list again keeps the
    /// first signal.
    pub fn keep_mounted(mut self, key: ReadSignal<Option<String>>) -> Self {
        self.keep = Some(key);
        self
    }

    /// Calls `handler` with committed text (typed characters, or an IME's
    /// committed composition) delivered while the list itself holds focus;
    /// text never reaches it while an item's content is focused. The list
    /// has no text input state, so an IME composes nothing over it. A table
    /// begins a cell edit from typed text this way. The handler runs in the
    /// event phase; its error propagates from the text input call. Set at
    /// construction; describing the list again keeps the first handler.
    pub fn on_text_input(
        mut self,
        handler: impl FnMut(&mut EventContext, &str) -> Result<(), RuntimeError> + 'static,
    ) -> Self {
        self.on_text = Some(Box::new(handler));
        self
    }

    /// Reports every change of the list's [`ListSelection`] after it took
    /// effect, with its [`ChangeOrigin`](crate::ChangeOrigin), so an
    /// application can follow the user's choice, for example to show the
    /// chosen item in a detail view.
    ///
    /// The list observes its selection when it updates. A change it sees is
    /// reported with the origin of the latest write: [`ChangeOrigin::User`](crate::ChangeOrigin::User)
    /// for a click or key on this list or another collection sharing the
    /// selection, and [`ChangeOrigin::Application`](crate::ChangeOrigin::Application)
    /// for a write through the [`ListSelection`] methods or a controller,
    /// including a [`CollectionFilter`](crate::CollectionFilter) moving the
    /// active item off a row its new query hides. Writes made
    /// between two updates of the list arrive as one change, carrying the
    /// origin of the last of them. The selection the list first sees is not a
    /// change, and a list that is hidden reports a change made meanwhile when
    /// it is shown and updates again. Without a selection nothing is reported.
    ///
    /// Unlike [`Tabs::on_select`](crate::Tabs::on_select), which reports each
    /// settled tab change, this reports the bound selection as the list
    /// observes it at its update, so the coalescing above is part of its
    /// meaning. Changes are delivered in an event phase after the update and
    /// layout waves of the flush have settled, one call per change, oldest
    /// first, and an input entry point settles its flush before it returns,
    /// as for every component notification. The callback may write
    /// bindings; an error does not undo the change, and the first error is
    /// returned from the flush. Like the other callbacks, it belongs to the
    /// first description of a mounted list.
    ///
    /// ```
    /// use pert_ui_runtime::*;
    /// use std::{cell::RefCell, rc::Rc, time::Duration};
    /// struct Fruit;
    /// impl ListItems for Fruit {
    ///     fn len(&self) -> usize { 3 }
    ///     fn key(&self, index: usize) -> Option<String> {
    ///         ["apple", "pear", "plum"].get(index).map(|key| (*key).to_owned())
    ///     }
    ///     fn build(&self, index: usize) -> Option<Element> {
    ///         Some(Label::new(self.key(index)?).key("row"))
    ///     }
    /// }
    /// let mut ui = Runtime::new();
    /// ui.set_viewport(Viewport::new(200.0, 200.0, 1.0)?)?;
    /// let items = ui.signal(ListSource::new(Fruit));
    /// let selection = ListSelection::new(&ui, SelectionMode::Single);
    /// let shown = ui.signal(None::<String>);
    /// let detail = shown.clone();
    /// let list = VirtualList::new(&items, 24.0)?
    ///     .selection(selection.clone())
    ///     .on_selection_change(move |_, change| detail.set(change.active.clone()));
    /// let id = ui.mount_tree(list.key("fruit"))?;
    /// ui.flush(Duration::ZERO)?;
    /// ui.focus(&id)?;
    /// ui.key_input(KeyCode::ArrowDown, KeyAction::Press)?;
    /// ui.key_input(KeyCode::ArrowDown, KeyAction::Release)?;
    /// assert_eq!(shown.get()?.as_deref(), Some("apple"));
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn on_selection_change(
        mut self,
        on_change: impl FnMut(
            &mut EventContext,
            &crate::ListSelectionChange,
        ) -> Result<(), RuntimeError>
        + 'static,
    ) -> Self {
        self.on_selection = Some(Box::new(on_change));
        self
    }

    /// Like [`Self::on_selection_change`] for an owner that installs the
    /// callback at construction: changes are observed only while `gate` reads
    /// true, so an owner without a user callback costs nothing.
    pub(crate) fn on_selection_change_gated(
        mut self,
        gate: Rc<Cell<bool>>,
        on_change: impl FnMut(
            &mut EventContext,
            &crate::ListSelectionChange,
        ) -> Result<(), RuntimeError>
        + 'static,
    ) -> Self {
        self.observe_gate = Some(gate);
        self.on_selection_change(on_change)
    }
    /// Whether this list observes its selection for a callback.
    fn observes_selection(&self) -> bool {
        self.on_selection.is_some() && self.observe_gate.as_ref().is_none_or(|gate| gate.get())
    }

    /// Records a change of the observed selection for
    /// [`Self::on_selection_change`]; the first observation is not a change.
    fn observe_selection(&mut self, active: Option<String>) {
        if !self.observes_selection() {
            return;
        }
        let Some(selection) = &self.selection else {
            return;
        };
        let selected = self.selected.clone();
        let Some((previous_active, previous_selected)) = self.selection_seen.take() else {
            self.selection_seen = Some((active, selected));
            return;
        };
        if previous_active != active
            || !(Rc::ptr_eq(&previous_selected, &selected) || previous_selected == selected)
        {
            self.selection_pending.push(crate::ListSelectionChange {
                previous_active,
                active: active.clone(),
                previous_selected,
                selected: selected.clone(),
                origin: selection.last_origin(),
            });
            if let Some(notifier) = &self.notifier {
                notifier.request();
            }
        }
        self.selection_seen = Some((active, selected));
    }

    /// Runs when Enter is pressed on the active item of a list with a selection.
    /// The callback receives the item's key and may write bindings; failures
    /// propagate through input routing.
    pub fn on_activate(
        mut self,
        activate: impl FnMut(&mut EventContext, &str) -> Result<(), RuntimeError> + 'static,
    ) -> Self {
        self.activate = Some(Box::new(activate));
        self
    }

    /// Makes items draggable and the list a drop target for
    /// [`ListDragItems`], which is how rows are reordered and moved between lists.
    ///
    /// A primary press on an item that the item's own content did not handle
    /// arms a drag; once it has moved [`DRAG_THRESHOLD`](crate::DRAG_THRESHOLD)
    /// the list starts a [drag session](EventContext::start_drag) carrying the
    /// pressed item's key, or every selected key when the pressed item is part of
    /// a selection. Pressing a selected item narrows the selection only on a
    /// release that did not become a drag. The preview is the pressed item built
    /// again from [`ListItems::build`], or a count for several items.
    ///
    /// While such a drag is over the list, a Primary insertion line marks the
    /// boundary a drop would use, and resting near an edge of an overflowing list
    /// scrolls it. On a drop `on_drop` receives a [`ListDrop`] whose position
    /// names the application's keys, never an index, and performs the edit in the
    /// application's model by replacing the [`ListSource`]: **a move is this one
    /// edit**, not an insertion here plus a removal at the source. Return `false`
    /// when the application refuses, and the source is told the drop was refused.
    /// Items from another list arrive the same way, with
    /// [`ListDrop::is_internal`] false.
    ///
    /// A drag is never the only way: with a [`ListSelection`], Alt with an arrow
    /// along the list moves the active item, or the selection containing it, by
    /// one place through the same callback.
    pub fn reorderable(
        mut self,
        on_drop: impl FnMut(&mut EventContext, &ListDrop<'_>) -> Result<bool, RuntimeError> + 'static,
    ) -> Self {
        self.reorder = Some(Box::new(on_drop));
        self
    }
    /// Restricts pointer and keyboard reordering and accepting collection drops.
    /// Default allowed; a callback installed through [`Self::reorderable`] is
    /// still required. Repeated restrictions combine through [`Permission::and`].
    /// Revocation clears an armed press/drop indication and cancels this list's
    /// active drag. Denied Alt-arrow reordering falls through to ordinary selection
    /// navigation. Selection, activation and scrolling remain available. Bound
    /// changes are observed during update without polling. Compatible
    /// redescription retains this descriptor and its bindings; change its bound
    /// signal or use a new list key to replace the restriction.
    pub fn reorder_permission(mut self, permission: impl Into<Permission>) -> Self {
        self.reorder_permission = self.reorder_permission.and(permission);
        self
    }

    /// The operations a drag started by this list permits; a move by default.
    /// Has no effect without [`Self::reorderable`].
    pub fn drag_operations(mut self, operations: DropOperations) -> Self {
        self.drag_operations = operations;
        self
    }

    fn axis(&self) -> Ref<'_, VirtualAxis> {
        self.axis.borrow()
    }

    /// Replaces the item boundaries a drop uses with a rule of the owner's, as
    /// [`Tree::reorderable`](crate::Tree::reorderable) does to offer dropping
    /// into a node and after an enclosing level. Has no effect without
    /// [`Self::reorderable`].
    ///
    /// While a [`ListDragItems`] drag is over the list as the drop candidate,
    /// the rule receives the dragged keys and a [`ListDropPoint`], and answers
    /// where a drop there goes and how the list marks it, or `None` to forbid
    /// one there: the list then answers `Forbidden`, draws no mark, and a
    /// release cancels. The position it answers is what the reorder callback
    /// receives. It runs on the UI thread on every pointer movement of the
    /// drag, so it should answer from the application's model without
    /// building rows.
    ///
    /// ```
    /// use pert_ui_runtime::*;
    /// struct Groups;
    /// impl ListItems for Groups {
    ///     fn len(&self) -> usize { 2 }
    ///     fn key(&self, index: usize) -> Option<String> {
    ///         ["reports", "settings"].get(index).map(|key| (*key).to_owned())
    ///     }
    ///     fn build(&self, index: usize) -> Option<Element> {
    ///         let key = self.key(index)?;
    ///         Some(Element::new(key.clone(), Label::new(key)))
    ///     }
    /// }
    /// let ui = Runtime::new();
    /// let items = ui.signal(ListSource::new(Groups));
    /// // Every drop goes into the group under the pointer, never between.
    /// let list = VirtualList::new(items.read_only(), 24.0)?
    ///     .reorderable(|_, drop| Ok(matches!(drop.position(), DropPosition::Into(_))))
    ///     .placement(|_keys, at| {
    ///         let (Some(index), Some(key)) = (at.index(), at.key()) else {
    ///             return None;
    ///         };
    ///         Some(ListDropPlacement::new(
    ///             DropPosition::Into(key.to_owned()),
    ///             ListDropMark::Into(index),
    ///         ))
    ///     });
    /// # let _ = list;
    /// # Ok::<(), GeometryError>(())
    /// ```
    pub fn placement(
        mut self,
        rule: impl Fn(&[String], &ListDropPoint) -> Option<ListDropPlacement> + 'static,
    ) -> Self {
        self.placer = Some(Box::new(rule));
        self
    }

    /// The active item's position, if it is in the current source.
    pub(crate) const fn active_index(&self) -> Option<usize> {
        self.active_index
    }

    /// Hands a keyboard move an owner resolved to the reorder callback, as the
    /// list's own Alt with an arrow does. `false` when the list is not
    /// reorderable or the application refused.
    pub(crate) fn reorder_keys(
        &mut self,
        cx: &mut EventContext,
        keys: &[String],
        position: DropPosition,
    ) -> Result<bool, RuntimeError> {
        if !self.reorder_allowed {
            return Ok(false);
        }
        let Some(reorder) = &mut self.reorder else {
            return Ok(false);
        };
        reorder(
            cx,
            &ListDrop {
                items: keys,
                position,
                operation: DropOperation::Move,
                internal: true,
            },
        )
    }

    /// Whether the list was made [`Self::reorderable`].
    pub(crate) fn is_reorderable(&self) -> bool {
        self.reorder.is_some() && self.reorder_allowed
    }

    // The keys a drag of the item at `index` carries: the selection when the
    // item belongs to it, in list order, and otherwise the item alone.
    pub(crate) fn dragged_keys(&self, index: usize) -> Vec<String> {
        let Some(items) = &self.snapshot else {
            return Vec::new();
        };
        let Some(key) = items.key(index) else {
            return Vec::new();
        };
        if self.selected.len() > 1 && self.selected.contains(&key) {
            let mut keys: Vec<_> = self
                .selected
                .iter()
                .filter_map(|key| items.index_of(key).map(|index| (index, key.clone())))
                .collect();
            keys.sort_by_key(|(index, _)| *index);
            keys.into_iter().map(|(_, key)| key).collect()
        } else {
            vec![key]
        }
    }

    // The item boundary nearest a local point: `n` is before item `n`, and the
    // count is after the last item.
    fn boundary_at(&self, point: Point) -> usize {
        let count = self.axis().count();
        let (along, across) = self.along(point);
        let position = (f64::from(along) + self.offset.get()).max(0.0);
        if count == 0 {
            return 0;
        }
        if position >= self.axis().extent() {
            return count;
        }
        let Some(index) = self.item_at(point) else {
            return count;
        };
        let Ok(slot) = self.slot(index) else {
            return index;
        };
        // Items follow each other along the list in one lane and across it in a
        // grid, so the half that decides before or after lies along that order.
        let after = if self.axis().lanes() == 1 {
            let (start, extent) = if self.horizontal {
                (slot.x(), slot.width())
            } else {
                (slot.y(), slot.height())
            };
            along >= start + extent / 2.0
        } else {
            let (start, extent) = if self.horizontal {
                (slot.y(), slot.height())
            } else {
                (slot.x(), slot.width())
            };
            across >= start + extent / 2.0
        };
        index + usize::from(after)
    }

    fn position_for(&self, boundary: usize) -> DropPosition {
        let Some(items) = &self.snapshot else {
            return DropPosition::End;
        };
        match items.key(boundary) {
            Some(key) => DropPosition::Before(key),
            None => match boundary.checked_sub(1).and_then(|last| items.key(last)) {
                Some(key) => DropPosition::After(key),
                None => DropPosition::End,
            },
        }
    }

    // The pointer as an owner's placement sees it: the item under it, clamped
    // into the list, and how far along that item it is.
    fn drop_point(&self, point: Point) -> ListDropPoint {
        let (along, across) = self.along(point);
        let count = self.axis().count();
        let viewport = self.viewport.get();
        let (main, cross) = if self.horizontal {
            (viewport.width(), viewport.height())
        } else {
            (viewport.height(), viewport.width())
        };
        let past_end = f64::from(along) + self.offset.get() >= self.axis().extent();
        let clamped = |value: f32, extent: f32| value.clamp(0.0, (extent - 0.5).max(0.0));
        let inside = if self.horizontal {
            Point::new(clamped(along, main), clamped(across, cross))
        } else {
            Point::new(clamped(across, cross), clamped(along, main))
        };
        let index = if count == 0 {
            None
        } else if past_end {
            Some(count - 1)
        } else {
            inside.ok().and_then(|inside| self.item_at(inside))
        };
        let fraction = match index {
            Some(_) if past_end => 1.0,
            Some(index) => self.slot(index).map_or(0.5, |slot| {
                let (start, extent) = if self.horizontal {
                    (slot.x(), slot.width())
                } else {
                    (slot.y(), slot.height())
                };
                if extent > 0.0 {
                    ((along - start) / extent).clamp(0.0, 1.0)
                } else {
                    0.5
                }
            }),
            None => 0.0,
        };
        ListDropPoint {
            index,
            key: index.and_then(|index| self.snapshot.as_ref()?.key(index)),
            fraction,
            across,
        }
    }

    // Where a drop at a local point goes: the owner's placement, or the item
    // boundary nearest it. `None` where the owner forbids a drop.
    fn placement_at(&self, keys: &[String], point: Point) -> Option<ListDropPlacement> {
        match &self.placer {
            Some(placer) => placer(keys, &self.drop_point(point)),
            None => {
                let boundary = self.boundary_at(point);
                Some(ListDropPlacement {
                    position: self.position_for(boundary),
                    mark: ListDropMark::Line {
                        boundary,
                        indent: 0.0,
                    },
                })
            }
        }
    }

    // The insertion line for a boundary, in the list's own coordinates.
    fn insertion_line(&self, boundary: usize, thickness: f32) -> Option<Rect> {
        let count = self.axis().count();
        let (index, trailing) = if boundary < count {
            (boundary, false)
        } else {
            (count.checked_sub(1)?, true)
        };
        let slot = self.slot(index).ok()?;
        let [x, y, w, h] = slot.values();
        let single = self.axis().lanes() == 1;
        // Across the lane in a single-lane list, along the item's leading or
        // trailing edge in a grid.
        let vertical_line = single == self.horizontal;
        if vertical_line {
            let at = if trailing { x + w } else { x };
            Rect::new(at - thickness / 2.0, y, thickness, h).ok()
        } else {
            let at = if trailing { y + h } else { y };
            Rect::new(x, at - thickness / 2.0, w, thickness).ok()
        }
    }

    // How fast a drag at `point` scrolls the list: negative towards the start.
    fn autoscroll_at(&self, point: Point) -> f32 {
        if !self.overflows() {
            return 0.0;
        }
        let (along, _) = self.along(point);
        let main = self.main();
        let zone = AUTOSCROLL_ZONE.min(main / 3.0);
        if zone <= 0.0 {
            return 0.0;
        }
        let depth = if along < zone {
            -(zone - along.max(0.0)) / zone
        } else if along > main - zone {
            (along.min(main) - (main - zone)) / zone
        } else {
            0.0
        };
        depth * AUTOSCROLL_STEP
    }

    fn set_bar_hover(&mut self, cx: &mut EventContext, hover: bool) {
        if self.bar_hover != hover {
            self.bar_hover = hover;
            cx.invalidate(Invalidation::Paint);
        }
    }
    fn set_row_hover(&mut self, cx: &mut EventContext, point: Option<Point>) {
        if self.hovered_point != point {
            self.hovered_point = point;
            cx.invalidate(Invalidation::Paint);
        }
    }

    fn end_drop_indication(&mut self, cx: &mut EventContext) {
        if self.mark.take().is_some() {
            cx.invalidate(Invalidation::Paint);
        }
        if std::mem::take(&mut self.autoscroll) != 0.0 {
            cx.cancel_timer();
        }
    }

    // Moves the active item, or the selection containing it, one place.
    fn move_by_key(&mut self, cx: &mut EventContext, forward: bool) -> Result<bool, RuntimeError> {
        let (Some(index), Some(items)) = (self.active_index, self.snapshot.clone()) else {
            return Ok(false);
        };
        let keys = self.dragged_keys(index);
        let indices: Vec<_> = keys.iter().filter_map(|key| items.index_of(key)).collect();
        let (Some(first), Some(last)) = (indices.iter().min(), indices.iter().max()) else {
            return Ok(false);
        };
        let position = if forward {
            match items.key(last + 1) {
                Some(key) => DropPosition::After(key),
                None => return Ok(false),
            }
        } else {
            match first
                .checked_sub(1)
                .and_then(|previous| items.key(previous))
            {
                Some(key) => DropPosition::Before(key),
                None => return Ok(false),
            }
        };
        if !self.reorder_allowed {
            return Ok(false);
        }
        let Some(reorder) = &mut self.reorder else {
            return Ok(false);
        };
        reorder(
            cx,
            &ListDrop {
                items: &keys,
                position,
                operation: DropOperation::Move,
                internal: true,
            },
        )
    }

    // The viewport's extent along and across the scrolling axis.
    fn main(&self) -> f32 {
        let viewport = self.viewport.get();
        if self.horizontal {
            viewport.width()
        } else {
            viewport.height()
        }
    }
    fn along(&self, point: Point) -> (f32, f32) {
        if self.horizontal {
            (point.x(), point.y())
        } else {
            (point.y(), point.x())
        }
    }

    /// The items the current offset and viewport need mounted.
    /// The list's rows and extents along its axis, for development
    /// validation's `UnboundedCollection`.
    #[cfg(debug_assertions)]
    pub(crate) fn extent_for_validation(&self) -> crate::validation::development::CollectionExtent {
        crate::validation::development::CollectionExtent {
            rows: self.axis().count(),
            content: self.axis().extent(),
            allocated: f64::from(self.main()),
            horizontal: self.horizontal,
        }
    }

    fn wanted(&self) -> (usize, usize) {
        let window = self
            .axis()
            .window(self.offset.get(), self.main(), self.overscan);
        (window.start, window.end)
    }

    // An item's slot in the list's own coordinates at the current offset.
    fn slot(&self, index: usize) -> Result<Rect, crate::GeometryError> {
        let viewport = self.viewport.get();
        let cross = if self.horizontal {
            viewport.height()
        } else {
            viewport.width()
        };
        // Both edges of a lane come from the same expression, so one lane's end is
        // exactly the next lane's start. Multiplying a rounded lane width instead
        // lets neighbors overlap by a fraction of a pixel. The width subtracts two
        // edges whose ratio is at most two, which floating point does exactly, so
        // `across + lane` reproduces the next edge bit for bit.
        let lanes = self.axis().lanes() as f32;
        let edge = |lane: usize| cross * lane as f32 / lanes;
        let across = edge(self.axis().lane(index));
        let lane = edge(self.axis().lane(index) + 1) - across;
        // Relative to the offset in `f64`, so the start is as small as the
        // viewport and exact in `f32` however far down the collection it is.
        let start = (self.axis().line_offset(index) - self.offset.get()) as f32;
        let extent = self.axis().line_extent(self.axis().line(index));
        if self.horizontal {
            Rect::new(start, across, extent, lane)
        } else {
            Rect::new(across, start, lane, extent)
        }
    }

    // The item under a point in the list's own coordinates, if any.
    fn item_at(&self, point: Point) -> Option<usize> {
        let (along, across) = self.along(point);
        let viewport = self.viewport.get();
        let cross = if self.horizontal {
            viewport.height()
        } else {
            viewport.width()
        };
        if along < 0.0 || across < 0.0 || across >= cross || cross <= 0.0 {
            return None;
        }
        // Truncation is the intended floor of nonnegative quotients.
        let position = f64::from(along) + self.offset.get();
        if position >= self.axis().extent() {
            return None;
        }
        let line = self.axis().line_at(position);
        let lane = ((across / cross) * self.axis().lanes() as f32).floor() as usize;
        let index = line
            .checked_mul(self.axis().lanes())?
            .checked_add(lane.min(self.axis().lanes() - 1))?;
        (index < self.axis().count()).then_some(index)
    }

    fn scroll(&self, cx: &mut EventContext, offset: f64) -> Result<bool, RuntimeError> {
        let next = self.axis().clamp_offset(offset, self.main());
        let changed = self.offset.replace(next) != next;
        if changed {
            cx.invalidate(Invalidation::Layout);
        }
        Ok(changed)
    }

    fn bar(&self) -> Result<scrollbar::ScrollbarGeometry, crate::GeometryError> {
        scrollbar::geometry(
            self.bar_extents.get(),
            Rect::from_size(self.viewport.get()),
            !self.horizontal,
            // The bar is drawn in `f32`; its thumb moves in whole pixels, far
            // coarser than the rounding of either value.
            self.axis().extent() as f32,
            self.offset.get() as f32,
            0.0,
        )
    }

    fn overflows(&self) -> bool {
        self.main() > 0.0 && self.axis().extent() > f64::from(self.main())
    }

    fn drag_to(&self, cx: &mut EventContext, point: Point) -> Result<(), RuntimeError> {
        let Some(grab) = self.drag else {
            return Ok(());
        };
        if !self.overflows() {
            return Ok(());
        }
        let bar = self.bar()?;
        if let Some(next) = bar.offset_for_pointer(point, grab) {
            self.scroll(cx, f64::from(next))?;
        }
        Ok(())
    }

    // Measures every mounted item at its lane's cross extent and records each
    // line as its tallest item, correcting the offset for lines above the view.
    fn measure_lines(&self, cx: &ArrangeContext<'_>, size: Size) -> Result<(), RuntimeError> {
        let lanes = self.axis().lanes() as f32;
        let cross = if self.horizontal {
            size.height()
        } else {
            size.width()
        } / lanes;
        let mut lines: std::collections::BTreeMap<usize, f32> = std::collections::BTreeMap::new();
        for child in 0..cx.len() {
            let key = cx.measurement().key(child)?;
            let Some(index) = self
                .mounted
                .iter()
                .find(|(_, mounted)| mounted == key)
                .map(|(index, _)| *index)
            else {
                continue;
            };
            let available = if self.horizontal {
                Constraints::new(None, Some(cross))?
            } else {
                Constraints::new(Some(cross), None)?
            };
            let measured = cx.measure(child, available)?;
            let extent = if self.horizontal {
                measured.width()
            } else {
                measured.height()
            };
            let line = self.axis().line(index);
            let tallest = lines.entry(line).or_insert(0.0);
            *tallest = tallest.max(extent);
        }
        let mut offset = self.offset.get();
        let mut axis = self.axis.borrow_mut();
        for (line, extent) in lines {
            // A row that measures to nothing keeps its estimate rather than
            // collapsing the line it is still expected to fill.
            if extent > 0.0 {
                offset = axis.measure_line(line, extent, offset)?;
            }
        }
        drop(axis);
        self.offset.set(offset);
        Ok(())
    }

    // Where keyboard navigation lands for a key, as an item index.
    fn destination(&self, code: KeyCode) -> Option<usize> {
        let count = self.axis().count();
        if count == 0 {
            return None;
        }
        let lanes = self.axis().lanes();
        let (next, previous, next_lane, previous_lane) = if self.horizontal {
            (
                KeyCode::ArrowRight,
                KeyCode::ArrowLeft,
                KeyCode::ArrowDown,
                KeyCode::ArrowUp,
            )
        } else {
            (
                KeyCode::ArrowDown,
                KeyCode::ArrowUp,
                KeyCode::ArrowRight,
                KeyCode::ArrowLeft,
            )
        };
        let page = self
            .axis()
            .lines_per_page(self.main())
            .saturating_mul(lanes);
        let last = count - 1;
        // With nothing active yet, forward keys start at the first item and
        // backward keys at the last, so the first press always lands on an item.
        let Some(current) = self.active_index else {
            return if code == next || code == next_lane && lanes > 1 {
                Some(0)
            } else if code == previous || code == previous_lane && lanes > 1 {
                Some(last)
            } else {
                match code {
                    KeyCode::Home | KeyCode::PageDown => Some(0),
                    KeyCode::End | KeyCode::PageUp => Some(last),
                    _ => None,
                }
            };
        };
        let current = current.min(last);
        Some(if code == next {
            current.saturating_add(lanes).min(last)
        } else if code == previous {
            current.saturating_sub(lanes)
        } else if code == next_lane && lanes > 1 {
            current.saturating_add(1).min(last)
        } else if code == previous_lane && lanes > 1 {
            current.saturating_sub(1)
        } else {
            match code {
                KeyCode::PageDown => current.saturating_add(page).min(last),
                KeyCode::PageUp => current.saturating_sub(page),
                KeyCode::Home => 0,
                KeyCode::End => last,
                _ => return None,
            }
        })
    }

    // Applies a selection gesture to an item and scrolls it into view.
    fn choose(
        &mut self,
        cx: &mut EventContext,
        index: usize,
        gesture: Gesture,
    ) -> Result<(), RuntimeError> {
        if let (Some(selection), Some(items)) = (&self.selection, &self.snapshot) {
            selection.with_origin(crate::ChangeOrigin::User, || {
                selection.apply(&**items, index, gesture)
            })?;
            self.active_index = Some(index);
            let offset = self.axis().reveal(index, self.offset.get(), self.main());
            self.scroll(cx, offset)?;
        }
        Ok(())
    }
}

/// The served reveal request whose row is not mounted yet. A list removed
/// before mounting it cancels the receipt.
#[derive(Default)]
struct Revealing(Option<(String, ListRevealReceipt)>);
impl Drop for Revealing {
    fn drop(&mut self) {
        if let Some((_, receipt)) = self.0.take() {
            receipt.finish(ListRevealStatus::Cancelled);
        }
    }
}

/// The rows a list offers to content search: its current source and the
/// controller through which a hit for a row that is not mounted is revealed.
struct ListContent {
    items: ListSource,
    controller: Option<ListController>,
    revision: u64,
    mounted_current: bool,
}
impl crate::ContentRows for ListContent {
    fn len(&self) -> usize {
        self.items.len()
    }
    fn key(&self, index: usize) -> Option<String> {
        self.items.key(index)
    }
    fn text(&self, index: usize) -> Option<String> {
        self.items.text(index)
    }
    fn revision(&self) -> u64 {
        self.revision
    }
    fn mounted_rows_current(&self) -> bool {
        self.mounted_current
    }
    fn reveal(&self, key: &str) -> Result<ListRevealReceipt, RuntimeError> {
        // Only a list that has never been shown lacks a controller; a jump
        // shows the list before it reveals a row.
        self.controller
            .as_ref()
            .ok_or(RuntimeError::Hidden)?
            .reveal(key)
    }
}

/// The last request serial this list has read from its controller. A list
/// removed while a newer request is waiting, for example because it was
/// hidden and never updated again, cancels that request when no other list
/// has taken it, since no list attached later replays it.
#[derive(Default)]
struct Unserved {
    seen: u64,
    source: Option<Signal<Option<RevealRequest>>>,
}
impl Drop for Unserved {
    fn drop(&mut self) {
        if let Some(Ok(Some(request))) = self.source.as_ref().map(Signal::get)
            && request.serial != self.seen
            && !request.claimed.get()
        {
            request.receipt.finish(ListRevealStatus::Cancelled);
        }
    }
}

impl Component for VirtualList {
    fn deliver_notifications(&mut self, cx: &mut EventContext) -> Result<(), RuntimeError> {
        // `observe_selection` requests one delivery per change, so each call
        // delivers the oldest change, in order with other components' changes.
        if self.selection_pending.is_empty() {
            return Ok(());
        }
        let change = self.selection_pending.remove(0);
        match &mut self.on_selection {
            Some(on_change) => on_change(cx, &change),
            None => Ok(()),
        }
    }
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        let allowed = self.reorder_permission.read_paint(cx)?;
        if self.reorder_allowed && !allowed {
            if self.armed.is_some() || self.reordering {
                cx.cancel_input();
            } else if self.autoscroll != 0.0 {
                cx.cancel_timer();
            }
            self.armed = None;
            self.mark = None;
            self.autoscroll = 0.0;
        }
        self.reorder_allowed = allowed;
        // Reading focus subscribes the list to focus moving into, out of or
        // within it, so a row that loses focus stops being pinned promptly.
        let _ = cx.focus_within();
        self.focused = cx.focused();
        let items = cx.read(&self.items)?;
        let replaced = self.snapshot.as_ref() != Some(&items) || !self.rows_current;
        self.rows_current = false;
        // A replaced source, such as a filtered one, keeps the row at the top
        // of the viewport where it was, by key, rather than the numeric
        // offset, which would now show another row.
        let anchor = match (&self.snapshot, replaced) {
            (Some(previous), true) if self.offset.get() > 0.0 => {
                let axis = self.axis.borrow();
                let index = axis.line_at(self.offset.get()) * axis.lanes();
                previous
                    .key(index)
                    .map(|key| (key, self.offset.get() - axis.line_offset(index)))
            }
            _ => None,
        };
        {
            let mut axis = self.axis.borrow_mut();
            axis.set_count(items.len());
            // A position may now hold another row, so its measurement is stale.
            if replaced {
                axis.forget();
            }
        }
        if let Some((key, within)) = anchor
            && let Some(index) = items.index_of(&key)
        {
            let start = self.axis.borrow().line_offset(index);
            self.offset.set(start + within);
            cx.request_layout();
        }
        // Geometry and the anchor now describe this source, even if building
        // its rows later fails. Track row construction separately for search.
        self.snapshot = Some(items.clone());
        // A list without an application controller keeps its own, so content
        // search can reveal a row that is not mounted through the same
        // keyed request as an application would.
        if self.controller.is_none() {
            let controller = ListController::new(cx);
            self.reveal_seen = Unserved {
                seen: 0,
                source: Some(controller.reveal.clone()),
            };
            self.controller = Some(controller);
        }
        if let Some(selection) = self.selection.clone() {
            let active = cx.read_paint(&selection.active())?;
            self.selected = cx.read_paint(&selection.selected())?;
            if self.observes_selection() {
                self.notifier.get_or_insert_with(|| cx.notifier());
            }
            self.observe_selection(active.clone());
            // The position is resolved only when the key or the source changed,
            // since a source may answer `index_of` in linear time.
            if replaced || active != self.active {
                self.active_index = active.as_deref().and_then(|key| items.index_of(key));
                // An active item set from outside, by the application or an owner
                // such as a tree, is shown like one reached by the keyboard.
                if active != self.active
                    && let Some(index) = self.active_index
                {
                    self.offset
                        .set(self.axis().reveal(index, self.offset.get(), self.main()));
                    cx.request_layout();
                }
                self.active = active;
            }
        }
        if let Some(controller) = &self.controller
            && let Some(request) = cx.read(&controller.reveal)?
            && request.serial != self.reveal_seen.seen
        {
            self.reveal_seen.seen = request.serial;
            request.claimed.set(true);
            if let Some((_, older)) = self.revealing.0.take() {
                older.finish(ListRevealStatus::Superseded);
            }
            if let Some(index) = items.index_of(&request.key) {
                self.offset
                    .set(self.axis().reveal(index, self.offset.get(), self.main()));
                cx.request_layout();
                self.revealing.0 = Some((request.key, request.receipt));
            } else {
                request.receipt.finish(ListRevealStatus::NotFound);
            }
        }
        let (start, end) = self.needed.take().unwrap_or_else(|| self.wanted());
        let count = self.axis().count();
        let (start, end) = (start.min(count), end.min(count));
        // Items outside the range stay mounted while they own focus or a
        // gesture, or while the owner keeps them.
        let kept = match &self.keep {
            Some(key) => cx.read(key)?.and_then(|key| items.index_of(&key)),
            None => None,
        };
        let pinned = cx
            .children()
            .iter()
            .filter(|child| child.focus_within() || child.captures_pointer())
            .filter_map(|child| items.index_of(child.key()))
            .chain(kept);
        let mut wanted = Vec::new();
        for index in self.axis().mount_order(start..end, pinned) {
            wanted.push((index, items.key(index).ok_or(LayoutError::InvalidChild)?));
        }
        if replaced || wanted != self.mounted {
            let mut elements = Vec::with_capacity(wanted.len());
            for (index, key) in &wanted {
                let mut element = items.build(*index).ok_or(LayoutError::InvalidChild)?;
                element.key.clone_from(key);
                elements.push(element);
            }
            cx.request_children(elements);
            self.mounted = wanted;
            cx.request_layout();
        }
        self.range = (start, end);
        // A requested row completes its receipt once it is mounted and stays
        // wanted; mounting it schedules the next update, which finds it.
        if let Some((key, receipt)) = &self.revealing.0 {
            if items.index_of(key).is_none() {
                receipt.finish(ListRevealStatus::NotFound);
                self.revealing.0 = None;
            } else if self.mounted.iter().any(|(_, wanted)| wanted == key)
                && let Some(child) = cx.children().iter().find(|child| child.key() == key)
            {
                receipt.finish(ListRevealStatus::Mounted(child.id().clone()));
                self.revealing.0 = None;
            }
        }
        self.rows_current = true;
        Ok(())
    }

    fn can_reveal(&self) -> bool {
        true
    }

    fn content_rows(&self) -> Option<Rc<dyn crate::ContentRows>> {
        // The legacy optional hook cannot report unavailable bound rows.
        // Runtime uses the fallible hook below and preserves those failures.
        self.try_content_rows().unwrap_or_default()
    }

    fn try_content_rows(&self) -> Result<Option<Rc<dyn crate::ContentRows>>, RuntimeError> {
        // Hidden lists do not update presentation, but searches need the current
        // source. The search retains this allocation, so its identity cannot be
        // reused while its cursor still references it.
        let items = self.items.get()?;
        let revision = Rc::as_ptr(&items.0).cast::<()>() as usize as u64;
        let mounted_current = self.rows_current && self.snapshot.as_ref() == Some(&items);
        Ok(Some(Rc::new(ListContent {
            items,
            controller: self.controller.clone(),
            revision,
            mounted_current,
        })))
    }

    fn descendant_tab_stops(&self) -> bool {
        self.item_stops
    }

    fn focusable(&self) -> bool {
        if self.selection.is_some() {
            self.axis().count() > 0
        } else {
            self.overflows()
        }
    }

    fn measure(
        &self,
        cx: &MeasureContext<'_>,
        available: Constraints,
    ) -> Result<Size, RuntimeError> {
        let _ = cx;
        // The natural size of an unconstrained list; a long list is given a
        // viewport, so the rounding of a huge extent never places anything.
        let extent = self.axis().extent() as f32;
        Ok(if self.horizontal {
            Size::new(
                available.width().unwrap_or(extent),
                available.height().unwrap_or(0.0),
            )?
        } else {
            Size::new(
                available.width().unwrap_or(0.0),
                available.height().unwrap_or(extent),
            )?
        })
    }

    fn arrange(&self, cx: &mut ArrangeContext<'_>, size: Size) -> Result<(), RuntimeError> {
        self.viewport.set(size);
        self.bar_extents
            .set(scrollbar::Extents::read(cx.measurement()));
        if self.axis().is_estimated() {
            self.measure_lines(cx, size)?;
        }
        self.offset
            .set(self.axis().clamp_offset(self.offset.get(), self.main()));
        for child in 0..cx.len() {
            let key = cx.measurement().key(child)?;
            let index = self
                .mounted
                .iter()
                .find(|(_, mounted)| mounted == key)
                .map(|(index, _)| *index)
                .ok_or(LayoutError::InvalidChild)?;
            let slot = self.slot(index)?;
            cx.place_clipped(child, slot, slot)?;
        }
        let wanted = self.wanted();
        if wanted != self.range {
            self.needed.set(Some(wanted));
            cx.request_update();
        }
        Ok(())
    }

    fn pointer_input(
        &mut self,
        cx: &mut EventContext,
        event: PointerEvent,
    ) -> Result<InputOutcome, RuntimeError> {
        let Some(point) = cx.to_local(event.position()) else {
            return Ok(InputOutcome::Ignored);
        };
        // The bar under the pointer shows that it can be dragged. Moving onto an
        // item that takes the pointer delivers `Leave` here, which clears it.
        if self.drag.is_none() && matches!(event.kind(), PointerKind::Move | PointerKind::Leave) {
            let over = event.kind() == PointerKind::Move
                && !event.is_outside()
                && cx.pointer_inside()
                && self.hit_test_foreground(point);
            self.set_bar_hover(cx, over);
            let row = if !over
                && event.kind() == PointerKind::Move
                && !event.is_outside()
                && cx.pointer_inside()
                && (self.selection.is_some() || self.is_reorderable())
            {
                self.item_at(point).map(|_| point)
            } else {
                None
            };
            self.set_row_hover(cx, row);
        }
        match event.kind() {
            PointerKind::Press(PointerButton::Primary)
                if cx.pointer_inside() && !event.is_outside() =>
            {
                if self.overflows() {
                    let bar = self.bar()?;
                    if bar.track.contains(point) {
                        let grab = bar.grab_offset(point);
                        cx.capture_pointer()?;
                        self.drag = Some(grab);
                        self.drag_to(cx, point)?;
                        return Ok(InputOutcome::Handled);
                    }
                }
                let item = self.item_at(point);
                let modifiers = cx.modifiers();
                // Pressing one of several selected items keeps them all, so they
                // can be dragged together; a plain release narrows it instead.
                let deferred = self.reorder.is_some()
                    && self.reorder_allowed
                    && !modifiers.shift
                    && !modifiers.control
                    && self.selected.len() > 1
                    && item
                        .and_then(|index| self.snapshot.as_ref()?.key(index))
                        .is_some_and(|key| self.selected.contains(&key));
                if self.reorder.is_some()
                    && self.reorder_allowed
                    && let Some(key) = item.and_then(|index| self.snapshot.as_ref()?.key(index))
                {
                    cx.capture_pointer()?;
                    self.armed = Some(Armed {
                        origin: event.position(),
                        key,
                        deferred,
                    });
                }
                if self.selection.is_some()
                    && let Some(index) = item
                {
                    cx.request_focus();
                    if !deferred {
                        self.choose(
                            cx,
                            index,
                            Gesture::Press {
                                extend: modifiers.shift,
                                toggle: modifiers.control,
                            },
                        )?;
                    }
                    return Ok(InputOutcome::Handled);
                }
                if self.armed.is_some() {
                    return Ok(InputOutcome::Handled);
                }
            }
            PointerKind::Move if self.armed.is_some() => {
                if let Some(armed) = &self.armed
                    && (event.position().x() - armed.origin.x())
                        .hypot(event.position().y() - armed.origin.y())
                        >= crate::DRAG_THRESHOLD
                {
                    // The pressed item by key; it may have moved or gone.
                    let index = self
                        .snapshot
                        .as_ref()
                        .and_then(|items| items.index_of(&armed.key));
                    self.armed = None;
                    let Some(index) = index else {
                        cx.release_pointer();
                        return Ok(InputOutcome::Handled);
                    };
                    let keys = self.dragged_keys(index);
                    let preview = match (keys.len(), self.snapshot.as_ref()) {
                        (1, Some(items)) => keyed_row(items, index)
                            .map_or_else(DragPreview::none, DragPreview::element),
                        (count, _) => DragPreview::element(Element::new(
                            "items",
                            Label::new(format!("{count} items")),
                        )),
                    };
                    if !keys.is_empty() {
                        cx.start_drag(
                            DragPayload::new(ListDragItems::new(keys)),
                            self.drag_operations,
                            preview,
                        )?;
                        self.reordering = true;
                    }
                }
                return Ok(InputOutcome::Handled);
            }
            PointerKind::Release(PointerButton::Primary) if self.armed.is_some() => {
                if let Some(armed) = self.armed.take() {
                    cx.release_pointer();
                    if armed.deferred
                        && let Some(index) = self
                            .snapshot
                            .as_ref()
                            .and_then(|items| items.index_of(&armed.key))
                    {
                        self.choose(
                            cx,
                            index,
                            Gesture::Press {
                                extend: false,
                                toggle: false,
                            },
                        )?;
                    }
                }
                return Ok(InputOutcome::Handled);
            }
            PointerKind::Move if self.drag.is_some() => {
                self.drag_to(cx, point)?;
                return Ok(InputOutcome::Handled);
            }
            PointerKind::Release(PointerButton::Primary) if self.drag.is_some() => {
                self.drag_to(cx, point)?;
                self.drag = None;
                cx.release_pointer();
                let over =
                    !event.is_outside() && cx.pointer_inside() && self.hit_test_foreground(point);
                self.set_bar_hover(cx, over);
                cx.invalidate(Invalidation::Paint);
                return Ok(InputOutcome::Handled);
            }
            PointerKind::Scroll(delta) => {
                // A wheel with one axis still scrolls a horizontal list.
                let amount = if self.horizontal && delta.x() != 0.0 {
                    delta.x()
                } else {
                    delta.y()
                };
                if self.scroll(cx, self.offset.get() + f64::from(amount))? {
                    return Ok(InputOutcome::Handled);
                }
            }
            _ => {}
        }
        Ok(InputOutcome::Ignored)
    }

    fn text_input(&mut self, cx: &mut EventContext, text: &str) -> Result<(), RuntimeError> {
        match &mut self.on_text {
            Some(handler) => handler(cx, text),
            None => Ok(()),
        }
    }

    fn keyboard_event(
        &mut self,
        cx: &mut EventContext,
        event: KeyEvent,
        keys: &KeyboardState,
    ) -> Result<InputOutcome, RuntimeError> {
        let control = keys.control_key(event);
        if !matches!(
            event.transition,
            KeyTransition::Pressed | KeyTransition::Repeated
        ) {
            return Ok(InputOutcome::Ignored);
        }
        if control == KeyCode::Escape && self.drag.take().is_some() {
            // The drag's emphasis ends; as on release, the bar stays emphasized
            // only if the stationary pointer is still over it.
            cx.release_pointer();
            let over = cx
                .pointer_position()
                .filter(|_| cx.pointer_inside())
                .and_then(|point| cx.to_local(point))
                .is_some_and(|point| self.hit_test_foreground(point));
            self.set_bar_hover(cx, over);
            cx.invalidate(Invalidation::Paint);
            return Ok(InputOutcome::Handled);
        }
        if self.selection.is_some() {
            let pressed = event.transition == KeyTransition::Pressed;
            let (forward, backward) = if self.horizontal {
                (KeyCode::ArrowRight, KeyCode::ArrowLeft)
            } else {
                (KeyCode::ArrowDown, KeyCode::ArrowUp)
            };
            // The keyboard equivalent of dragging the active item one place.
            if self.reorder.is_some()
                && self.reorder_allowed
                && crate::activation::alt_only(keys.chord_modifiers(event))
                && (control == forward || control == backward)
            {
                self.move_by_key(cx, control == forward)?;
                return Ok(InputOutcome::Handled);
            }
            match (control, self.active_index) {
                (KeyCode::Space, Some(index)) if pressed => {
                    self.choose(cx, index, Gesture::Toggle)?;
                    return Ok(InputOutcome::Handled);
                }
                (KeyCode::Enter, Some(_)) if pressed => {
                    if let (Some(activate), Some(key)) = (&mut self.activate, &self.active) {
                        activate(cx, key)?;
                        return Ok(InputOutcome::Handled);
                    }
                    return Ok(InputOutcome::Ignored);
                }
                _ => {}
            }
            let modifiers = keys.chord_modifiers(event);
            // Shift extends and Control keeps; Alt along the list belongs to
            // reordering, which falls through here only when denied. Other
            // chords are left to command bindings.
            let reorder_key = self.reorder.is_some()
                && (control == forward || control == backward)
                && crate::activation::alt_only(modifiers);
            if modifiers.super_key || (modifiers.alt && !reorder_key) {
                return Ok(InputOutcome::Ignored);
            }
            let Some(index) = self.destination(control) else {
                return Ok(InputOutcome::Ignored);
            };
            self.choose(
                cx,
                index,
                Gesture::Move {
                    extend: modifiers.shift,
                    keep: modifiers.control,
                },
            )?;
            return Ok(InputOutcome::Handled);
        }
        // Modified scrolling chords are left to command bindings.
        if cx.modifiers() != crate::Modifiers::default() {
            return Ok(InputOutcome::Ignored);
        }
        let old = self.offset.get();
        let page = f64::from(self.main() * scrollbar::PAGE_STEP_FRACTION);
        let step = f64::from(self.axis().item_extent());
        let (forward, backward) = if self.horizontal {
            (KeyCode::ArrowRight, KeyCode::ArrowLeft)
        } else {
            (KeyCode::ArrowDown, KeyCode::ArrowUp)
        };
        let next = if control == forward {
            old + step
        } else if control == backward {
            old - step
        } else {
            match control {
                KeyCode::PageDown => old + page,
                KeyCode::PageUp => old - page,
                KeyCode::Home => 0.0,
                KeyCode::End => self.axis().extent(),
                _ => return Ok(InputOutcome::Ignored),
            }
        };
        Ok(if self.scroll(cx, next)? {
            InputOutcome::Handled
        } else {
            InputOutcome::Ignored
        })
    }

    fn reveal(&mut self, cx: &mut EventContext, rect: Rect) -> Result<bool, RuntimeError> {
        // Compared in local pixels, where the scroll offset is measured.
        let (Some(size), Some(rect)) = (cx.size(), cx.to_local_rect(rect)) else {
            return Ok(false);
        };
        let (start, extent, target, length) = if self.horizontal {
            (0.0, size.width(), rect.x(), rect.width())
        } else {
            (0.0, size.height(), rect.y(), rect.height())
        };
        let delta = scrollbar::reveal_delta(start, extent, target, length);
        self.scroll(cx, self.offset.get() + f64::from(delta))
    }

    fn hit_test_foreground(&self, point: Point) -> bool {
        self.overflows() && self.bar().is_ok_and(|bar| bar.track.contains(point))
    }

    fn input_reset(&mut self) {
        self.drag = None;
        self.armed = None;
        self.bar_hover = false;
        self.hovered_point = None;
        // Hiding ends a drop indication without a `Leave`. A pending
        // autoscroll tick then finds nothing to do and stops.
        self.mark = None;
        self.autoscroll = 0.0;
    }

    fn hover_changed(&mut self, cx: &mut EventContext, hovered: bool) {
        if !hovered {
            self.set_bar_hover(cx, false);
            self.set_row_hover(cx, None);
        }
    }

    fn drag_event(
        &mut self,
        cx: &mut EventContext,
        event: &DragEvent<'_>,
    ) -> Result<DropResponse, RuntimeError> {
        let Some(items) = event.payload().get::<ListDragItems>() else {
            return Ok(DropResponse::Rejected);
        };
        if !self.reorder_allowed || self.reorder.is_none() || event.phase() == DragPhase::Leave {
            self.end_drop_indication(cx);
            return Ok(DropResponse::Rejected);
        }
        let Some(point) = cx.to_local(event.position()) else {
            return Ok(DropResponse::Rejected);
        };
        // A list above the target scrolls at its edges and marks nothing.
        let candidate = event.role() == DragRole::Candidate;
        let placement = candidate
            .then(|| self.placement_at(items.keys(), point))
            .flatten();
        if event.phase() == DragPhase::Drop {
            self.end_drop_indication(cx);
            let Some(placement) = placement else {
                return Ok(DropResponse::Forbidden);
            };
            let position = placement.position;
            let internal = *event.source() == cx.id();
            let Some(reorder) = &mut self.reorder else {
                return Ok(DropResponse::Rejected);
            };
            let performed = reorder(
                cx,
                &ListDrop {
                    items: items.keys(),
                    position,
                    operation: event.preferred(),
                    internal,
                },
            )?;
            return Ok(if performed {
                DropResponse::Accepted(event.preferred())
            } else {
                DropResponse::Rejected
            });
        }
        let mark = placement.as_ref().map(|placement| placement.mark);
        if self.mark != mark {
            self.mark = mark;
            cx.invalidate(Invalidation::Paint);
        }
        let speed = self.autoscroll_at(point);
        if speed != 0.0 && self.autoscroll == 0.0 {
            cx.set_timer(AUTOSCROLL_INTERVAL)?;
        } else if speed == 0.0 && self.autoscroll != 0.0 {
            cx.cancel_timer();
        }
        self.autoscroll = speed;
        Ok(if candidate && placement.is_none() {
            DropResponse::Forbidden
        } else {
            DropResponse::Accepted(event.preferred())
        })
    }

    fn drag_finished(&mut self, cx: &mut EventContext, _: DragOutcome) -> Result<(), RuntimeError> {
        self.reordering = false;
        self.armed = None;
        cx.invalidate(Invalidation::Paint);
        Ok(())
    }

    fn timer_elapsed(&mut self, cx: &mut EventContext) -> Result<(), RuntimeError> {
        if self.autoscroll == 0.0 || !cx.drag_active() {
            self.autoscroll = 0.0;
            return Ok(());
        }
        // The rows move under a still pointer, so the runtime resolves the drag
        // again after this layout and the insertion line follows.
        if self.scroll(cx, self.offset.get() + f64::from(self.autoscroll))? {
            cx.set_timer(AUTOSCROLL_INTERVAL)?;
        } else {
            self.autoscroll = 0.0;
        }
        Ok(())
    }

    fn paint(&self, painter: &mut Painter) -> Result<(), DrawError> {
        if self.selected.is_empty() && self.hovered_point.is_none() {
            return Ok(());
        }
        let Some(size) = painter.size() else {
            return Ok(());
        };
        let bounds = Rect::from_size(size);
        let hovered_index = self.hovered_point.and_then(|point| self.item_at(point));
        for (index, key) in &self.mounted {
            let selected = self.selected.contains(key);
            if selected || hovered_index == Some(*index) {
                let slot = self.slot(*index)?;
                painter.fill_rect(
                    slot.intersection(bounds),
                    painter.color(if selected {
                        ColorRole::TabSelected
                    } else {
                        ColorRole::SurfaceHover
                    }),
                );
            }
        }
        Ok(())
    }

    fn paint_foreground(&self, painter: &mut Painter) -> Result<(), DrawError> {
        // While a drop is indicated, the indication is what the list shows.
        if self.focused
            && self.outline
            && self.mark.is_none()
            && let Some(index) = self.active_index
            && self.mounted.iter().any(|(mounted, _)| *mounted == index)
            && let Some(size) = painter.size()
        {
            let slot = self.slot(index)?.intersection(Rect::from_size(size));
            if !slot.is_empty() {
                painter.focus_ring(slot, Shape::Rectangle)?;
            }
        }
        if let Some(ListDropMark::Into(index)) = self.mark
            && let Some(size) = painter.size()
        {
            let slot = self.slot(index)?.intersection(Rect::from_size(size));
            if !slot.is_empty() {
                let width = Length::new(painter.metric(MetricRole::FocusWidth))?;
                painter.stroke_shape(
                    slot,
                    Shape::Rectangle,
                    width,
                    painter.color(ColorRole::Primary),
                )?;
            }
        }
        if let Some(ListDropMark::Line { boundary, indent }) = self.mark
            && let Some(size) = painter.size()
            && let Some(line) =
                self.insertion_line(boundary, painter.metric(MetricRole::FocusWidth))
        {
            // An indent moves the start of a line across a single vertical
            // lane, which is where an owner such as a tree nests its items.
            let line = if indent > 0.0 && !self.horizontal && self.axis().lanes() == 1 {
                let [x, y, w, h] = line.values();
                let indent = indent.min(w);
                Rect::new(x + indent, y, w - indent, h)?
            } else {
                line
            };
            // Kept inside the list so the first and last boundaries stay visible.
            let [x, y, w, h] = line.values();
            let line = Rect::new(
                x.clamp(0.0, (size.width() - w).max(0.0)),
                y.clamp(0.0, (size.height() - h).max(0.0)),
                w,
                h,
            )?;
            painter.fill_rect(line, painter.color(ColorRole::Primary));
        }
        if self.overflows() {
            let thumb = self.bar()?.thumb;
            let active = self.bar_hover || self.drag.is_some();
            painter.fill_rect(thumb, scrollbar::thumb_color(painter, active));
        }
        Ok(())
    }

    fn inspect(&self) -> ComponentDetails {
        let mut details = ComponentDetails::default();
        let count = self.axis().count();
        details
            .properties
            .insert("rows".into(), InspectValue::Number(count as f64));
        details.properties.insert(
            "mounted".into(),
            InspectValue::Number(self.mounted.len() as f64),
        );
        details.properties.insert(
            "range_start".into(),
            InspectValue::Number(self.range.0 as f64),
        );
        details.properties.insert(
            "range_end".into(),
            InspectValue::Number(self.range.1 as f64),
        );
        details.properties.insert(
            if self.horizontal {
                "scroll_x"
            } else {
                "scroll_y"
            }
            .into(),
            InspectValue::Number(self.offset.get()),
        );
        details.properties.insert(
            "lanes".into(),
            InspectValue::Number(self.axis().lanes() as f64),
        );
        details.properties.insert(
            "reorderable".into(),
            InspectValue::Bool(self.reorder.is_some() && self.reorder_allowed),
        );
        match self.mark {
            Some(ListDropMark::Line { boundary, .. }) => {
                details.properties.insert(
                    "drop_boundary".into(),
                    InspectValue::Number(boundary as f64),
                );
            }
            Some(ListDropMark::Into(index)) => {
                details
                    .properties
                    .insert("drop_into".into(), InspectValue::Number(index as f64));
            }
            None => {}
        }
        details.validation.interactive = self.focusable();
        // Only part of a long collection is mounted, so the count is reported
        // here rather than left to be counted from the children, and the rows are
        // deliberately not enumerated as items.
        // A reorderable list names the action, not the gesture: its items move
        // by drag and, with a selection, by Alt with an arrow.
        let semantics = Semantics::new(SemanticRole::List).enabled_actions(
            (self.reorder.is_some() && self.reorder_allowed && count > 0)
                .then_some(SemanticAction::Move),
        );
        details.semantics = Some(if self.selection.is_some() {
            semantics
                .value(SemanticValue::Choice {
                    selected: self.active_index,
                    count,
                    label: None,
                })
                .enabled_actions((count > 0).then_some(SemanticAction::Select))
                .enabled_actions(self.overflows().then_some(SemanticAction::Scroll))
        } else {
            semantics
                .value(SemanticValue::Number {
                    value: count as f64,
                    minimum: Some(0.0),
                    maximum: None,
                })
                .enabled_actions(self.overflows().then_some(SemanticAction::Scroll))
        });
        details
    }
}
