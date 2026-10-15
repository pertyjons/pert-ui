//! Nested content region layouts without native-window or modal ownership.
use crate::layout::rect;
use crate::*;
use std::{cell::RefCell, rc::Rc};

type RegionHandler = Box<
    dyn FnMut(&mut EventContext, RegionEdge, SplitSizing, ChangeOrigin) -> Result<(), RuntimeError>,
>;

/// Optional top/bottom/left/right/center regions with automatic overflow access.
///
/// Top/bottom span the full width; left/right occupy the middle band. Edges size to
/// content unless a fixed extent is supplied. Center fills the remainder. Every
/// region scrolls overflow independently by default; `Self::fixed` instead pins
/// the frame and region allocations and clips overflow. Themed dividers (8 logical pixels at Standard) separate present
/// edges; they are locked unless an edge's resize permission is supplied (`top_resizable` and
/// the other edges), and [`Self::resizable`] restricts all of them at once. Standard viewport
/// minima are 24 pixels for top/bottom heights, 64 for left/right widths and 64×64
/// for center. By default the whole region layout scrolls when these minima cannot fit;
/// fixed region layouts clip at those limits and may leave controls unreachable. Empty slots consume
/// no extent or gap. Opposite edges without center share one divider: top/left
/// resize permission and sizing take precedence when supplied, otherwise a supplied
/// bottom/right resize permission controls its own extent. With neither permission,
/// top/left sizing controls the locked divider. Region layouts may nest at any depth; they create no focus/modal scope,
/// window or event loop. All extents use logical pixels.
///
/// The layout is not a [`Workspace`](crate::Workspace) participant itself, but
/// what it builds is captured: each region's content is addressed under its
/// slot name (`top`, `bottom`, `left`, `right`, `center`), each divider's
/// sizing preference as the `<edge>-divider` part, and each region's and the
/// frame's scroll offsets as `viewport` and `shell` parts. These names are part
/// of the workspace document; docs/contracts.md describes it.
///
/// Describing a mounted layout again (through `Runtime::reconcile_children`)
/// applies new extents, resize permissions and [`Self::fixed`] to the retained
/// regions and dividers, which follow the [`SplitPane`] re-description rules:
/// a divider the user moved keeps its position unless that edge's described
/// extent changed. Adding or removing a region changes the divider structure
/// and remounts the dividers and regions around it. [`Self::on_resize`]
/// reports committed divider preferences to the application.
#[must_use]
#[derive(Default)]
pub struct RegionLayout {
    edges: PerEdge<Option<Element>>,
    center: Option<Element>,
    fixed_viewport: bool,
    fixed: PerEdge<Option<Length>>,
    /// Per-edge resize permission; `None` leaves the divider locked.
    resize: PerEdge<Option<Permission>>,
    /// Layout-wide restriction combined with every edge's permission.
    all: Permission,
    /// The application's report of committed divider preferences, shared
    /// with the dividers this description generates.
    on_resize: Rc<RefCell<Option<RegionHandler>>>,
}

/// One of the four edge regions of a [`RegionLayout`], as
/// [`RegionLayout::on_resize`] names the edge whose divider changed.
///
/// Its name is part of the workspace document format, so every property is an
/// exhaustive match rather than a position in a table: reordering the
/// variants cannot rename a persisted region or swap its sizing rules.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum RegionEdge {
    /// The full-width upper region.
    Top,
    /// The full-width lower region.
    Bottom,
    /// The left region of the middle band.
    Left,
    /// The right region of the middle band.
    Right,
}
use RegionEdge as Edge;
impl Edge {
    /// The order in which edges wrap the body, innermost first: the middle
    /// band is built around the center, then the full-width edges around it.
    const WRAPPING: [Self; 4] = [Self::Right, Self::Left, Self::Bottom, Self::Top];
    /// The slot name, also used in the divider's part name.
    const fn name(self) -> &'static str {
        match self {
            Self::Top => "top",
            Self::Bottom => "bottom",
            Self::Left => "left",
            Self::Right => "right",
        }
    }
    /// The divider axis between this edge and the body it wraps.
    const fn axis(self) -> SplitAxis {
        match self {
            Self::Top | Self::Bottom => SplitAxis::TopBottom,
            Self::Left | Self::Right => SplitAxis::LeftRight,
        }
    }
    /// Whether the edge spans the full width, sizing by height.
    const fn is_full_width(self) -> bool {
        matches!(self.axis(), SplitAxis::TopBottom)
    }
    /// Whether the edge precedes the body on its axis (top or left).
    const fn is_leading(self) -> bool {
        match self {
            Self::Top | Self::Left => true,
            Self::Bottom | Self::Right => false,
        }
    }
    /// The standard viewport minimum of the region.
    fn minimum(self) -> Size {
        if self.is_full_width() {
            Size::from([0, 24])
        } else {
            Size::from([64, 0])
        }
    }
}

/// A value per edge, addressed by [`Edge`] rather than by position.
#[derive(Default)]
struct PerEdge<T> {
    top: T,
    bottom: T,
    left: T,
    right: T,
}
impl<T> PerEdge<T> {
    fn get(&self, edge: Edge) -> &T {
        match edge {
            Edge::Top => &self.top,
            Edge::Bottom => &self.bottom,
            Edge::Left => &self.left,
            Edge::Right => &self.right,
        }
    }
    fn get_mut(&mut self, edge: Edge) -> &mut T {
        match edge {
            Edge::Top => &mut self.top,
            Edge::Bottom => &mut self.bottom,
            Edge::Left => &mut self.left,
            Edge::Right => &mut self.right,
        }
    }
}
impl RegionLayout {
    /// Creates an empty region layout. Add only the regions that have content.
    pub fn new() -> Self {
        Self::default()
    }
    /// Keeps the frame and region viewports fixed at their allocation origins.
    /// Overflow is clipped instead of scrolling the frame/regions. Put scrolling
    /// content in `ScrollArea` or `Tabs`, whose nested viewports scroll independently.
    /// Use this for application chrome: top menus and tab strips stay in place.
    pub fn fixed(mut self) -> Self {
        self.fixed_viewport = true;
        self
    }
    /// Sets the full-width upper region, replacing previous unmounted content.
    pub fn top(mut self, content: Element) -> Self {
        self.edges.top = Some(content);
        self
    }
    /// Sets the full-width lower region.
    pub fn bottom(mut self, content: Element) -> Self {
        self.edges.bottom = Some(content);
        self
    }
    /// Sets the left region of the middle band.
    pub fn left(mut self, content: Element) -> Self {
        self.edges.left = Some(content);
        self
    }
    /// Sets the right region of the middle band.
    pub fn right(mut self, content: Element) -> Self {
        self.edges.right = Some(content);
        self
    }
    /// Sets the filling center; this may itself be another region layout.
    pub fn center(mut self, content: Element) -> Self {
        self.center = Some(content);
        self
    }
    /// Supplies the top edge's resize permission. An edge without one keeps a
    /// locked divider. Repeated calls combine and the stricter wins
    /// ([`Permission::and`]), as does [`Self::resizable`] for the whole layout.
    /// A bound permission is read by the divider at each update; revoking it
    /// during a held resize cancels that resize, as [`SplitPane::resizable`]
    /// describes.
    pub fn top_resizable(self, permission: impl Into<Permission>) -> Self {
        self.edge_resizable(Edge::Top, permission.into())
    }
    /// Supplies the bottom edge's resize permission; see [`Self::top_resizable`].
    pub fn bottom_resizable(self, permission: impl Into<Permission>) -> Self {
        self.edge_resizable(Edge::Bottom, permission.into())
    }
    /// Supplies the left edge's resize permission; see [`Self::top_resizable`].
    pub fn left_resizable(self, permission: impl Into<Permission>) -> Self {
        self.edge_resizable(Edge::Left, permission.into())
    }
    /// Supplies the right edge's resize permission; see [`Self::top_resizable`].
    pub fn right_resizable(self, permission: impl Into<Permission>) -> Self {
        self.edge_resizable(Edge::Right, permission.into())
    }
    /// A layout-wide restriction on resizing, combined with each edge's own
    /// permission: a divider may be dragged only while both its edge's
    /// permission (`top_resizable` and the others) and this one allow it. So
    /// `resizable(false)` locks the whole layout, a bound signal locks and
    /// unlocks it at run time, and the restriction never grants resizing to an
    /// edge that was given no permission of its own, because the default for an
    /// edge is a locked divider. Repeated calls combine and the stricter wins
    /// ([`Permission::and`]). The default restricts nothing.
    pub fn resizable(mut self, permission: impl Into<Permission>) -> Self {
        self.all = self.all.and(permission);
        self
    }
    fn edge_resizable(mut self, edge: Edge, permission: Permission) -> Self {
        let slot = self.resize.get_mut(edge);
        *slot = Some(match slot.take() {
            Some(previous) => previous.and(permission),
            None => permission,
        });
        self
    }
    /// Fixes the preferred height of the top region; excess content remains scrollable.
    pub fn top_height(mut self, extent: Length) -> Self {
        self.fixed.top = Some(extent);
        self
    }
    /// Fixes the preferred height of the bottom region.
    pub fn bottom_height(mut self, extent: Length) -> Self {
        self.fixed.bottom = Some(extent);
        self
    }
    /// Fixes the preferred width of the left region.
    pub fn left_width(mut self, extent: Length) -> Self {
        self.fixed.left = Some(extent);
        self
    }
    /// Fixes the preferred width of the right region.
    pub fn right_width(mut self, extent: Length) -> Self {
        self.fixed.right = Some(extent);
        self
    }
    /// Reports a divider's committed preference after it changed, with the
    /// edge whose extent it sets and what changed it, as
    /// [`SplitPane::on_resize`] describes for one split: read the extent with
    /// [`SplitSizing::rule`] ([`SplitRule::First`] for a top or left edge,
    /// [`SplitRule::Second`] for a bottom or right one, a content rule while
    /// the edge follows its content). Opposite edges that share one divider
    /// report the edge whose permission or sizing controls it.
    ///
    /// Origins are [`ChangeOrigin::User`] when a drag or held keys end,
    /// [`ChangeOrigin::Restore`] for [`Runtime::restore_workspace`] and
    /// [`ChangeOrigin::Redescription`] when a described extent replaced the
    /// preference; a resize in progress and a cancelled one report nothing.
    /// Each divider reports at most once per flush, from an event phase after
    /// layout settles. Describing the layout again replaces the callback,
    /// including removing it, which also drops changes not yet delivered.
    pub fn on_resize(
        self,
        on_resize: impl FnMut(
            &mut EventContext,
            RegionEdge,
            SplitSizing,
            ChangeOrigin,
        ) -> Result<(), RuntimeError>
        + 'static,
    ) -> Self {
        *self.on_resize.borrow_mut() = Some(Box::new(on_resize));
        self
    }
}
impl Component for RegionLayout {
    fn reconcile(&mut self, next: Self) -> Option<Invalidation> {
        let Self {
            edges: _,
            center: _,
            fixed_viewport,
            fixed,
            resize,
            all,
            // The generated dividers forward to the new description's
            // callback, which they adopted as they reconciled.
            on_resize: _,
        } = next;
        // Every property was already applied to the generated regions and
        // dividers, which reconcile and invalidate themselves; these copies
        // only keep this description current.
        self.fixed_viewport = fixed_viewport;
        self.fixed = fixed;
        self.resize = resize;
        self.all = all;
        None
    }
    fn take_children(&mut self) -> Vec<Element> {
        fn region(key: &str, content: Element, minimum: Size, axes: ScrollAxes) -> Element {
            Element::new(
                key,
                Container::new(
                    Element::new("viewport", ScrollArea::new(content).axes(axes)).part("viewport"),
                )
                .size_limits(SizeLimits::at_least(minimum))
                .background_role(ColorRole::Surface),
            )
            .slot(key)
        }
        let axes = if self.fixed_viewport {
            ScrollAxes::None
        } else {
            ScrollAxes::Both
        };
        let mut body = self
            .center
            .take()
            .map(|content| region("center", content, Size::from([64, 64]), axes));
        let mut lone_edge = None;
        for edge in Edge::WRAPPING {
            let Some(content) = self.edges.get_mut(edge).take() else {
                continue;
            };
            let edge_region = region(edge.name(), content, edge.minimum(), axes);
            let Some(previous) = body.take() else {
                body = Some(edge_region);
                lone_edge = Some(edge);
                continue;
            };
            // Opposite edges share one divider when there is no center. Prefer
            // top/left permission when supplied, otherwise bottom/right's rule.
            let seed = lone_edge.take();
            let controller = seed
                .filter(|seed| {
                    seed.axis() == edge.axis()
                        && self.resize.get(edge).is_none()
                        && self.resize.get(*seed).is_some()
                })
                .unwrap_or(edge);
            let previous = if let Some(seed) = seed.filter(|seed| seed.axis() != edge.axis()) {
                Element::new(
                    "lone-region",
                    LoneRegion {
                        child: Some(previous),
                        edge: seed,
                        extent: *self.fixed.get(seed),
                    },
                )
                .structural()
            } else {
                previous
            };
            let sizing = match (controller.is_leading(), *self.fixed.get(controller)) {
                (true, Some(extent)) => SplitSizing::first(extent),
                (false, Some(extent)) => SplitSizing::second(extent),
                (true, None) => SplitSizing::CONTENT_FIRST,
                (false, None) => SplitSizing::CONTENT_SECOND,
            };
            let split = if edge.is_leading() {
                SplitPane::new(edge.axis(), edge_region, previous)
            } else {
                SplitPane::new(edge.axis(), previous, edge_region)
            }
            .sizing(sizing);
            let split = match self.resize.get_mut(controller).take() {
                Some(permission) => split.resizable(permission.and(self.all.clone())),
                None => split.locked(),
            };
            let slot = self.on_resize.clone();
            let split = split.on_resize(move |cx, sizing, origin| {
                // Taken out for the call, so no borrow spans application code.
                let handler = slot.borrow_mut().take();
                let Some(mut handler) = handler else {
                    return Ok(());
                };
                let result = handler(cx, controller, sizing, origin);
                slot.borrow_mut().get_or_insert(handler);
                result
            });
            body = Some(
                Element::new(format!("split-{}", edge.name()), split)
                    .part(format!("{}-divider", edge.name())),
            );
        }
        if let Some(edge) = lone_edge {
            body = Some(
                Element::new(
                    "lone-region",
                    LoneRegion {
                        child: body,
                        edge,
                        extent: *self.fixed.get(edge),
                    },
                )
                .structural(),
            );
        }
        vec![
            Element::new(
                "shell-viewport",
                ScrollArea::new(
                    Element::new(
                        "regions",
                        View::new(body.unwrap_or_else(|| {
                            Element::new("empty", Container::default()).structural()
                        })),
                    )
                    .structural(),
                )
                .axes(axes),
            )
            .part("shell"),
        ]
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
}

// Preserves a sole edge's content preference without inventing a divider.
struct LoneRegion {
    child: Option<Element>,
    edge: Edge,
    extent: Option<Length>,
}
impl Component for LoneRegion {
    fn reconcile(&mut self, next: Self) -> Option<Invalidation> {
        let Self {
            child: _,
            edge,
            extent,
        } = next;
        let changed = self.edge != edge || self.extent != extent;
        self.edge = edge;
        self.extent = extent;
        changed.then_some(Invalidation::Layout)
    }
    fn take_children(&mut self) -> Vec<Element> {
        self.child.take().into_iter().collect()
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
        let natural = cx.measure(0, available)?;
        Ok(match (self.edge.is_full_width(), self.extent) {
            (true, Some(extent)) => Size::new(natural.width(), extent.get())?,
            (false, Some(extent)) => Size::new(extent.get(), natural.height())?,
            (_, None) => natural,
        })
    }
    fn arrange(&self, cx: &mut ArrangeContext<'_>, size: Size) -> Result<(), RuntimeError> {
        if cx.is_empty() {
            return Ok(());
        }
        let preferred = self.measure(cx.measurement(), Constraints::bounded(size))?;
        let minimum = cx.measurement().limits(0)?.minimum();
        let width = if self.edge.is_full_width() {
            size.width()
        } else {
            preferred.width().max(minimum.width()).min(size.width())
        };
        let height = if self.edge.is_full_width() {
            preferred.height().max(minimum.height()).min(size.height())
        } else {
            size.height()
        };
        cx.place(
            0,
            rect(
                if self.edge == Edge::Right {
                    size.width() - width
                } else {
                    0.0
                },
                if self.edge == Edge::Bottom {
                    size.height() - height
                } else {
                    0.0
                },
                width,
                height,
            )?,
        )
    }
}
