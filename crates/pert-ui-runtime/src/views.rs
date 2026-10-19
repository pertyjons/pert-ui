//! Opinionated content boundaries, groups and scrolling viewports.
use crate::layout::rect;
use crate::scrollbar;
use crate::*;
use std::cell::Cell;

/// A retained content boundary; its element key names the logical view.
/// Hiding content retains descendant state and suspends updates/deadlines.
/// A view introduces no native window, event loop or modal scope.
///
/// Re-describing the view under the same key applies its visibility binding;
/// without one the view is shown. Its children reconcile by key.
#[must_use]
pub struct View {
    children: Vec<Element>,
    shown: Option<ReadSignal<bool>>,
    visible: bool,
    fixed: bool,
}
impl View {
    /// Creates a visible view around one content element.
    pub fn new(content: Element) -> Self {
        Self {
            children: vec![content],
            shown: None,
            visible: true,
            fixed: false,
        }
    }
    /// Adds a fixed viewport: content starts at local zero and clips overflow.
    /// It never scrolls or auto-reveals focus. Nested `ScrollArea`/`Tabs` may scroll
    /// their own content. To keep this view fixed in the window, place it outside
    /// scrolling ancestors, for example in a `RegionLayout::fixed` region.
    /// Calling it again changes nothing: a view has at most one viewport.
    pub fn fixed(mut self) -> Self {
        if self.fixed {
            return self;
        }
        self.fixed = true;
        self.children = self
            .children
            .into_iter()
            .map(|child| {
                Element::new(
                    "fixed-viewport",
                    ScrollArea::new(child).axes(ScrollAxes::None),
                )
                .structural()
            })
            .collect();
        self
    }
    /// Binds content visibility; the lightweight owner remains subscribed while hidden.
    ///
    /// The visibility is the application's: a reveal through
    /// [`Runtime::reveal`](crate::Runtime::reveal) does not change the signal and
    /// refuses with [`RevealRefusal::ApplicationControlled`](crate::RevealRefusal::ApplicationControlled)
    /// while it reads false.
    pub fn visible_signal(mut self, shown: impl Into<ReadSignal<bool>>) -> Self {
        let shown: ReadSignal<bool> = shown.into();
        self.shown = Some(shown);
        self
    }
}
impl Component for View {
    fn reconcile(&mut self, next: Self) -> Option<Invalidation> {
        let changed = !ReadSignal::same_optional(&self.shown, &next.shown);
        self.shown = next.shown;
        if self.shown.is_none() {
            self.visible = true;
        }
        changed.then_some(Invalidation::Layout)
    }
    fn inspect(&self) -> ComponentDetails {
        ComponentDetails {
            semantics: Some(Semantics::new(SemanticRole::View)),
            ..ComponentDetails::default()
        }
    }
    fn command_target(&self) -> bool {
        true
    }
    fn measure_limits(&self, cx: &MeasureContext<'_>) -> Result<SizeLimits, RuntimeError> {
        if cx.is_empty() {
            Ok(SizeLimits::default())
        } else {
            cx.limits(0)
        }
    }
    fn clips_children(&self) -> bool {
        false
    }
    fn take_children(&mut self) -> Vec<Element> {
        std::mem::take(&mut self.children)
    }
    fn child_visible(&self, _: &str) -> bool {
        self.visible
    }
    fn reveal_child(
        &mut self,
        _: &mut EventContext,
        _: &str,
    ) -> Result<crate::ChildReveal, RuntimeError> {
        // Only a bound signal hides the content, and it is the application's.
        Ok(if self.shown.is_some() {
            crate::ChildReveal::ApplicationControlled
        } else {
            crate::ChildReveal::AlreadyVisible
        })
    }
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        if let Some(shown) = &self.shown {
            self.visible = cx.read(shown)?;
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
}

/// Related content in a standard surface with theme padding (12 logical pixels at Standard) and an optional heading.
/// A rounded neutral border remains visible at rest, with accent when focus is
/// inside. Padding is measured from the outer edge; the inward border consumes
/// that padding without adding another inset. Typography and spacing follow theme.
///
/// Re-describing the group under the same key applies its title, which is also
/// its accessible name, padding, framing and inner layout; its children
/// reconcile by key.
#[must_use]
pub struct Group {
    title: String,
    inner: Container,
    padding: Spacing,
    framed: bool,
    background: Cell<Option<Color>>,
}
impl Group {
    /// Creates an untitled group.
    pub fn new(content: Element) -> Self {
        Self {
            title: String::new(),
            padding: Spacing::Theme(MetricRole::GroupPadding),
            framed: true,
            background: Cell::new(None),
            inner: Container::new(content)
                .background_role(ColorRole::Surface)
                .padding(MetricRole::GroupPadding),
        }
    }
    /// Uses equal padding: a fixed [`Length`] in logical pixels, retained
    /// across densities, or a [`MetricRole`] resolved from the theme, as
    /// [`Spacing`] describes. The default is `MetricRole::GroupPadding`. The
    /// inward border occupies this padding; zero permits content up to the
    /// edge. Each call replaces the earlier padding.
    pub fn padding(mut self, padding: impl Into<Spacing>) -> Self {
        self.padding = padding.into();
        self.inner = self.inner.padding(self.padding);
        self
    }
    /// Removes the outline, retaining the surface and themed padding.
    /// Groups are framed by default, including groups without a title.
    pub fn frameless(mut self) -> Self {
        self.framed = false;
        self
    }
    /// Creates a group with a semantic heading above its content.
    pub fn titled(title: impl Into<String>, content: Element) -> Self {
        let title = title.into();
        let mut group = Self::new(
            Element::new(
                "group-content",
                Column::new([
                    Element::new(
                        "heading",
                        Label::new(title.clone()).style(TextStyle::Heading),
                    )
                    .structural(),
                    Element::new("body", View::new(content)).structural(),
                ]),
            )
            .structural(),
        );
        group.title = title;
        group
    }
}
impl Component for Group {
    fn reconcile(&mut self, next: Self) -> Option<Invalidation> {
        let Self {
            title,
            inner,
            padding,
            framed,
            background: _,
        } = next;
        // The heading label is a descendant and reconciles itself; the title
        // kept here is the group's accessible name, read at inspection.
        self.title = title;
        let inner = self.inner.reconcile(inner);
        let layout = self.padding != padding;
        let paint = self.framed != framed;
        self.padding = padding;
        self.framed = framed;
        if layout {
            Some(Invalidation::Layout)
        } else if paint {
            inner.max(Some(Invalidation::Paint))
        } else {
            inner
        }
    }
    fn measure_limits(&self, cx: &MeasureContext<'_>) -> Result<SizeLimits, RuntimeError> {
        let minimum = if cx.is_empty() {
            Size::ZERO
        } else {
            cx.limits(0)?.minimum()
        };
        Ok(SizeLimits::at_least(Size::new(
            minimum.width() + 2.0 * self.padding.resolve(cx),
            minimum.height() + 2.0 * self.padding.resolve(cx),
        )?))
    }
    fn clips_children(&self) -> bool {
        false
    }
    fn take_children(&mut self) -> Vec<Element> {
        self.inner.take_children()
    }
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        self.inner.update(cx)
    }
    fn measure(
        &self,
        cx: &MeasureContext<'_>,
        available: Constraints,
    ) -> Result<Size, RuntimeError> {
        self.inner.measure(cx, available)
    }
    fn arrange(&self, cx: &mut ArrangeContext<'_>, size: Size) -> Result<(), RuntimeError> {
        self.inner.arrange(cx, size)
    }
    fn paint(&self, painter: &mut Painter) -> Result<(), DrawError> {
        if let Some(size) = painter.size() {
            let shape = Shape::Rounded(Length::new(painter.metric(MetricRole::CornerRadius))?);
            painter.fill_shape(
                Rect::from_size(size),
                shape,
                painter.color(ColorRole::Surface),
            )?;
        }
        Ok(())
    }
    fn paint_foreground(&self, painter: &mut Painter) -> Result<(), DrawError> {
        if self.framed
            && let Some(size) = painter.size()
        {
            let role = if painter.focus_within() {
                ColorRole::Primary
            } else {
                ColorRole::PanelBorder
            };
            let shape = Shape::Rounded(Length::new(painter.metric(MetricRole::CornerRadius))?);
            let width = Length::new(painter.metric(MetricRole::BorderWidth))?;
            painter.stroke_shape(Rect::from_size(size), shape, width, painter.color(role))?;
        }
        self.background
            .set(painter.size().map(|_| painter.color(ColorRole::Surface)));
        Ok(())
    }
    fn inspect(&self) -> ComponentDetails {
        let mut details = self.inner.inspect();
        details.box_model.background = self.background.get().map(Color::channels);
        details.semantics = Some(Semantics::new(SemanticRole::Group).name(self.title.as_str()));
        details
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

/// Axes on which a scroll viewport allows content overflow.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ScrollAxes {
    /// Fixed viewport: content starts at zero, overflow clips, and this viewport
    /// never scrolls or reveals focus. Nested scrolling children remain independent.
    None,
    /// Wrap to viewport width and scroll vertically (default).
    #[default]
    Vertical,
    /// Preserve horizontal content extent and scroll horizontally.
    Horizontal,
    /// Scroll both axes, while offering viewport width for responsive content.
    Both,
}
/// A clipped viewport retaining local scroll position with automatic focus reveal.
///
/// Defaults to vertical scrolling. Wheel, unmodified arrows and PageUp/PageDown
/// scroll; unmodified Home/End select the start/end. Modified chords of these
/// keys are left to command bindings. Scrollbars appear only on enabled overflowing axes.
/// Primary dragging preserves the thumb grab position; clicking its track centers
/// the thumb there and starts a drag. Each bar owns an edge strip of
/// [`MetricRole::ScrollbarLane`] (8 logical pixels at Standard), above child
/// input/drawing, with a visible thumb of [`MetricRole::ScrollbarThumb`] (4) that
/// is never shorter than [`MetricRole::ScrollbarMinimumThumb`] (12) while the
/// track allows it, so the bar follows theme density. The thumb is drawn in
/// [`ColorRole::ScrollbarThumb`], translucent in the built-in palettes, and in
/// the more opaque [`ColorRole::ScrollbarThumbHover`] while the pointer is over
/// its strip or dragging it, which shows that the bar can be dragged. Moving
/// into content that takes the pointer ends the emphasis. Both bars
/// stop short of their shared corner. Release ends dragging; Escape or context
/// loss cancels dragging at the current offset, without restoring the old position.
/// Content keeps its hard minimums; offsets clamp after resize/content changes.
/// No repaint timer is used. Hiding retains position and cancels interaction.
/// Describing a mounted viewport again with other [`Self::axes`] keeps its
/// offset, clamped to the new axes, and ends a thumb drag at the current offset.
/// [`Self::on_scroll`] reports offset changes to the application.
#[must_use]
pub struct ScrollArea {
    children: Vec<Element>,
    axes: ScrollAxes,
    offset: Cell<Point>,
    on_scroll: Option<ScrollHandler>,
    /// The offset last reported through `on_scroll` and a change waiting.
    report: crate::geometry_report::GeometryReport<Point>,
    extent: Cell<Size>,
    viewport: Cell<Size>,
    bar_extents: Cell<scrollbar::Extents>,
    drag: Option<(bool, f32)>,
    /// The bar under the pointer, `true` for the horizontal one, presented as
    /// draggable.
    bar_hover: Option<bool>,
}
impl ScrollArea {
    /// Wraps one child in a vertical viewport.
    pub fn new(content: Element) -> Self {
        Self {
            children: vec![content],
            axes: ScrollAxes::Vertical,
            offset: Cell::new(Point::default()),
            extent: Cell::new(Size::ZERO),
            viewport: Cell::new(Size::ZERO),
            bar_extents: Cell::new(scrollbar::Extents::STANDARD),
            drag: None,
            bar_hover: None,
            on_scroll: None,
            report: crate::geometry_report::GeometryReport::new(Point::default()),
        }
    }
    /// Reports the scroll offset after it changed, with what changed it.
    ///
    /// The offset is in the content's logical pixels from its top-left corner,
    /// after arrangement clamped it to the content and the viewport. The
    /// callback runs on the UI thread from an event phase once the flush's
    /// updates and layout have settled ([`Component::deliver_notifications`]),
    /// at most once per flush: changes within one flush, such as the many
    /// wheel events of continuous scrolling, report the last offset with the
    /// origin of the last change, and a flush that ends where it started
    /// reports nothing. The origin is
    ///
    /// - [`ChangeOrigin::User`] for the wheel, keys and scrollbars;
    /// - [`ChangeOrigin::Reveal`] for scrolling that reveals focus or a
    ///   requested child ([`Runtime::reveal`] and its relatives);
    /// - [`ChangeOrigin::Restore`] for [`Runtime::restore_workspace`];
    /// - [`ChangeOrigin::Redescription`] when arrangement clamped the offset
    ///   because the content, the viewport or [`Self::axes`] changed.
    ///
    /// A thumb drag reports as it moves, since every position is committed;
    /// Escape ends the drag where it is and reports nothing more. The
    /// callback may write bindings; its error propagates from the flush after
    /// the other pending deliveries, the offset stays and nothing is
    /// replayed. Describing the viewport again replaces the callback,
    /// including removing it; a description without one discards a change not
    /// yet delivered.
    ///
    /// ```
    /// use pert_ui_runtime::*;
    /// use std::{cell::RefCell, rc::Rc, time::Duration};
    ///
    /// let mut ui = Runtime::new();
    /// ui.set_viewport(Viewport::new(200.0, 100.0, 1.0)?)?;
    /// let log = Rc::new(RefCell::new(Vec::new()));
    /// let seen = log.clone();
    /// let rows = (0..20).map(|i| Element::new(format!("row-{i}"), Label::new(format!("Row {i}"))));
    /// let area = ui.mount_tree(Element::new(
    ///     "list",
    ///     ScrollArea::new(Element::new("rows", Column::new(rows))).on_scroll(move |_, offset, origin| {
    ///         seen.borrow_mut().push((offset.y(), origin));
    ///         Ok(())
    ///     }),
    /// ))?;
    /// ui.flush(Duration::ZERO)?;
    /// ui.focus(&area)?;
    /// ui.key_input(KeyCode::ArrowDown, KeyAction::Press)?;
    /// ui.key_input(KeyCode::ArrowDown, KeyAction::Release)?;
    /// ui.flush(Duration::ZERO)?;
    /// assert_eq!(*log.borrow(), [(40.0, ChangeOrigin::User)]);
    /// # Ok::<(), RuntimeError>(())
    /// ```
    pub fn on_scroll(
        mut self,
        on_scroll: impl FnMut(&mut EventContext, Point, ChangeOrigin) -> Result<(), RuntimeError>
        + 'static,
    ) -> Self {
        self.on_scroll = Some(Box::new(on_scroll));
        self
    }
    /// Records a change for `on_scroll`, when there is one.
    fn changed(&self, origin: ChangeOrigin) {
        if self.on_scroll.is_some() {
            self.report.record(origin);
        }
    }
    /// Selects meaningful overflow axes; vertical remains the ordinary default.
    /// Layout resolves offsets on disabled axes to zero, including restored
    /// preferences and retained positions after redescription.
    pub fn axes(mut self, axes: ScrollAxes) -> Self {
        self.axes = axes;
        self
    }
    fn bars(&self) -> (bool, bool) {
        let e = self.extent.get();
        let v = self.viewport.get();
        (
            matches!(self.axes, ScrollAxes::Horizontal | ScrollAxes::Both)
                && e.width() > v.width()
                && v.width() > 0.0,
            matches!(self.axes, ScrollAxes::Vertical | ScrollAxes::Both)
                && e.height() > v.height()
                && v.height() > 0.0,
        )
    }
    // Shared geometry for painting, track hits and thumb travel. A crossing bar
    // reserves its own lane at the far end so the two never overlap in the corner.
    fn bar(&self, horizontal: bool) -> Result<scrollbar::ScrollbarGeometry, crate::GeometryError> {
        let (h, v) = self.bars();
        let extent = self.extent.get();
        let offset = self.offset.get();
        let (content, position, other) = if horizontal {
            (extent.width(), offset.x(), v)
        } else {
            (extent.height(), offset.y(), h)
        };
        let extents = self.bar_extents.get();
        scrollbar::geometry(
            extents,
            Rect::from_size(self.viewport.get()),
            !horizontal,
            content,
            position,
            if other { extents.lane } else { 0.0 },
        )
    }
    /// The bar whose track contains a local point, `true` for the horizontal one.
    fn bar_at(&self, point: Point) -> Option<bool> {
        let (h, v) = self.bars();
        [(true, h), (false, v)]
            .into_iter()
            .find(|(horizontal, enabled)| {
                *enabled
                    && self
                        .bar(*horizontal)
                        .is_ok_and(|bar| bar.track.contains(point))
            })
            .map(|(horizontal, _)| horizontal)
    }
    fn set_bar_hover(&mut self, cx: &mut EventContext, hover: Option<bool>) {
        if self.bar_hover != hover {
            self.bar_hover = hover;
            cx.invalidate(Invalidation::Paint);
        }
    }
    fn drag_to(&self, cx: &mut EventContext, point: Point) -> Result<(), RuntimeError> {
        let Some((horizontal, grab)) = self.drag else {
            return Ok(());
        };
        let (h, v) = self.bars();
        if !(if horizontal { h } else { v }) {
            return Ok(());
        }
        let bar = self.bar(horizontal)?;
        if let Some(next) = bar.offset_for_pointer(point, grab) {
            let old = self.offset.get();
            self.scroll(
                cx,
                if horizontal { next } else { old.x() },
                if horizontal { old.y() } else { next },
                ChangeOrigin::User,
            )?;
        }
        Ok(())
    }
    fn content(
        &self,
        cx: &MeasureContext<'_>,
        available: Constraints,
    ) -> Result<Size, RuntimeError> {
        if cx.is_empty() {
            return Ok(Size::ZERO);
        }
        cx.measure(
            0,
            Constraints::new(
                if self.axes == ScrollAxes::Horizontal {
                    None
                } else {
                    available.width()
                },
                if matches!(self.axes, ScrollAxes::Horizontal | ScrollAxes::None) {
                    available.height()
                } else {
                    None
                },
            )?,
        )
    }
    fn scroll(
        &self,
        cx: &mut EventContext,
        x: f32,
        y: f32,
        origin: ChangeOrigin,
    ) -> Result<bool, RuntimeError> {
        let extent = self.extent.get();
        let viewport = self.viewport.get();
        let next = Point::new(
            if matches!(self.axes, ScrollAxes::Vertical | ScrollAxes::None) {
                0.0
            } else {
                x.clamp(0.0, (extent.width() - viewport.width()).max(0.0))
            },
            if matches!(self.axes, ScrollAxes::Horizontal | ScrollAxes::None) {
                0.0
            } else {
                y.clamp(0.0, (extent.height() - viewport.height()).max(0.0))
            },
        )?;
        let changed = self.offset.replace(next) != next;
        if changed {
            cx.invalidate(Invalidation::Layout);
            self.changed(origin);
        }
        Ok(changed)
    }
}

type ScrollHandler =
    Box<dyn FnMut(&mut EventContext, Point, ChangeOrigin) -> Result<(), RuntimeError>>;
impl Component for ScrollArea {
    fn reconcile(&mut self, next: Self) -> Option<Invalidation> {
        let Self {
            children: _,
            axes,
            offset: _,
            extent: _,
            viewport: _,
            bar_extents: _,
            drag: _,
            bar_hover: _,
            on_scroll,
            report: _,
        } = next;
        // A callback that starts, or a description without one, takes the
        // current offset as known.
        if on_scroll.is_none() || self.on_scroll.is_none() {
            self.report.discard(self.offset.get());
        }
        self.on_scroll = on_scroll;
        if self.axes == axes {
            return None;
        }
        // The scroll position is retained and arrangement clamps it to the new
        // axes. A thumb drag may belong to a bar the new axes remove, so it
        // ends; the rest of that press is ignored until release.
        self.axes = axes;
        self.drag = None;
        self.bar_hover = None;
        Some(Invalidation::Layout)
    }
    fn workspace_state(&self) -> Result<Option<crate::ViewState>, crate::WorkspaceError> {
        let offset = self.offset.get();
        let state = crate::ViewState::new("scroll")?
            .with("x", crate::WorkspaceValue::number(f64::from(offset.x()))?)?
            .with("y", crate::WorkspaceValue::number(f64::from(offset.y()))?)?;
        Ok(Some(state))
    }
    fn restore_workspace_state(
        &mut self,
        state: &crate::ViewState,
    ) -> Result<Invalidation, crate::RestoreRejection> {
        use crate::RestoreRejection;
        state.expect_only(&["x", "y"])?;
        if self.drag.is_some() {
            return Err(RestoreRejection::Busy);
        }
        let axis = |name: &str| -> Result<f32, RestoreRejection> {
            let number = state.number(name)?;
            let value = number as f32;
            if number >= 0.0 && value.is_finite() {
                Ok(value)
            } else {
                Err(RestoreRejection::OutOfRange(name.to_owned()))
            }
        };
        let offset = Point::new(axis("x")?, axis("y")?)
            .map_err(|_| RestoreRejection::OutOfRange("x".to_owned()))?;
        // The saved offset is a preference. Arrangement clamps it against the
        // content and viewport this window actually has.
        self.offset.set(offset);
        self.changed(ChangeOrigin::Restore);
        Ok(Invalidation::Layout)
    }
    fn take_children(&mut self) -> Vec<Element> {
        std::mem::take(&mut self.children)
    }
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        self.report.set_notifier(|| cx.notifier());
        Ok(())
    }
    fn deliver_notifications(&mut self, cx: &mut EventContext) -> Result<(), RuntimeError> {
        if let Some((offset, origin)) = self.report.take(self.offset.get())
            && let Some(on_scroll) = &mut self.on_scroll
        {
            on_scroll(cx, offset, origin)?;
        }
        Ok(())
    }
    fn can_reveal(&self) -> bool {
        self.axes != ScrollAxes::None
    }
    fn focusable(&self) -> bool {
        let e = self.extent.get();
        let v = self.viewport.get();
        self.axes != ScrollAxes::None && (e.width() > v.width() || e.height() > v.height())
    }
    fn measure(
        &self,
        cx: &MeasureContext<'_>,
        available: Constraints,
    ) -> Result<Size, RuntimeError> {
        let content = self.content(cx, available)?;
        Ok(Size::new(
            available
                .width()
                .map_or(content.width(), |v| v.min(content.width())),
            available
                .height()
                .map_or(content.height(), |v| v.min(content.height())),
        )?)
    }
    fn arrange(&self, cx: &mut ArrangeContext<'_>, size: Size) -> Result<(), RuntimeError> {
        if cx.len() > 1 {
            return Err(LayoutError::InvalidChild.into());
        }
        self.bar_extents
            .set(scrollbar::Extents::read(cx.measurement()));
        let content = self.content(cx.measurement(), Constraints::bounded(size))?;
        let extent = Size::new(
            content.width().max(size.width()),
            if self.axes == ScrollAxes::None {
                size.height()
            } else {
                content.height().max(size.height())
            },
        )?;
        self.extent.set(extent);
        self.viewport.set(size);
        let old = self.offset.get();
        let offset = Point::new(
            if matches!(self.axes, ScrollAxes::Vertical | ScrollAxes::None) {
                0.0
            } else {
                old.x().min((extent.width() - size.width()).max(0.0))
            },
            if matches!(self.axes, ScrollAxes::Horizontal | ScrollAxes::None) {
                0.0
            } else {
                old.y().min((extent.height() - size.height()).max(0.0))
            },
        )?;
        if self.offset.replace(offset) != offset {
            // Clamping completes a change already recorded, or is one itself.
            if self.on_scroll.is_some() {
                self.report.record_if_idle(ChangeOrigin::Redescription);
            }
        }
        if !cx.is_empty() {
            let minimum = cx.measurement().limits(0)?.minimum();
            let width = if self.axes == ScrollAxes::Horizontal {
                extent.width()
            } else {
                size.width().max(minimum.width())
            };
            cx.place(0, rect(-offset.x(), -offset.y(), width, extent.height())?)?;
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
        // The bar under the pointer shows that it can be dragged. Moving into a
        // child that takes the pointer delivers `Leave` here, which clears it.
        let over =
            (event.kind() == PointerKind::Move && !event.is_outside() && cx.pointer_inside())
                .then(|| self.bar_at(point))
                .flatten();
        if self.drag.is_none() && matches!(event.kind(), PointerKind::Move | PointerKind::Leave) {
            self.set_bar_hover(cx, over);
        }
        match event.kind() {
            PointerKind::Press(PointerButton::Primary)
                if cx.pointer_inside() && !event.is_outside() =>
            {
                let (h, v) = self.bars();
                for (horizontal, enabled) in [(true, h), (false, v)] {
                    if !enabled {
                        continue;
                    }
                    let bar = self.bar(horizontal)?;
                    if bar.track.contains(point) {
                        let grab = bar.grab_offset(point);
                        cx.capture_pointer()?;
                        self.drag = Some((horizontal, grab));
                        self.drag_to(cx, point)?;
                        return Ok(InputOutcome::Handled);
                    }
                }
            }
            PointerKind::Move if self.drag.is_some() => {
                self.drag_to(cx, point)?;
                return Ok(InputOutcome::Handled);
            }
            PointerKind::Release(PointerButton::Primary) if self.drag.is_some() => {
                self.drag_to(cx, point)?;
                self.drag = None;
                cx.release_pointer();
                // The drag's emphasis ends; the bar stays emphasized only if the
                // pointer is still over it.
                let over = (!event.is_outside() && cx.pointer_inside())
                    .then(|| self.bar_at(point))
                    .flatten();
                self.set_bar_hover(cx, over);
                cx.invalidate(Invalidation::Paint);
                return Ok(InputOutcome::Handled);
            }
            _ => {}
        }
        if let PointerKind::Scroll(delta) = event.kind() {
            let old = self.offset.get();
            let x = if self.axes == ScrollAxes::Horizontal && delta.x() == 0.0 {
                delta.y()
            } else {
                delta.x()
            };
            if self.scroll(cx, old.x() + x, old.y() + delta.y(), ChangeOrigin::User)? {
                return Ok(InputOutcome::Handled);
            }
        }
        Ok(InputOutcome::Ignored)
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
                .and_then(|point| self.bar_at(point));
            self.set_bar_hover(cx, over);
            cx.invalidate(Invalidation::Paint);
            return Ok(InputOutcome::Handled);
        }
        // Modified navigation chords are left to command bindings.
        if keys.chord_modifiers(event) != Modifiers::default() {
            return Ok(InputOutcome::Ignored);
        }
        let old = self.offset.get();
        let viewport = self.viewport.get();
        let extent = self.extent.get();
        let (x, y) = match control {
            KeyCode::ArrowDown => (old.x(), old.y() + 40.0),
            KeyCode::ArrowUp => (old.x(), old.y() - 40.0),
            KeyCode::ArrowLeft => (old.x() - 40.0, old.y()),
            KeyCode::ArrowRight => (old.x() + 40.0, old.y()),
            KeyCode::PageDown => (
                old.x(),
                old.y() + viewport.height() * scrollbar::PAGE_STEP_FRACTION,
            ),
            KeyCode::PageUp => (
                old.x(),
                old.y() - viewport.height() * scrollbar::PAGE_STEP_FRACTION,
            ),
            KeyCode::Home => (0.0, 0.0),
            KeyCode::End => (extent.width(), extent.height()),
            _ => return Ok(InputOutcome::Ignored),
        };
        Ok(if self.scroll(cx, x, y, ChangeOrigin::User)? {
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
        let [x, y, w, h] = [0.0, 0.0, size.width(), size.height()];
        let [rx, ry, rw, rh] = rect.values();
        let old = self.offset.get();
        self.scroll(
            cx,
            old.x() + scrollbar::reveal_delta(x, w, rx, rw),
            old.y() + scrollbar::reveal_delta(y, h, ry, rh),
            ChangeOrigin::Reveal,
        )
    }
    fn hit_test_foreground(&self, point: Point) -> bool {
        let (h, v) = self.bars();
        [(true, h), (false, v)]
            .into_iter()
            .any(|(horizontal, enabled)| {
                enabled
                    && self
                        .bar(horizontal)
                        .is_ok_and(|bar| bar.track.contains(point))
            })
    }
    fn input_reset(&mut self) {
        self.drag = None;
        self.bar_hover = None;
    }
    fn hover_changed(&mut self, cx: &mut EventContext, hovered: bool) {
        if !hovered {
            self.set_bar_hover(cx, None);
        }
    }
    fn paint_foreground(&self, painter: &mut Painter) -> Result<(), DrawError> {
        let (h, v) = self.bars();
        for (horizontal, enabled) in [(true, h), (false, v)] {
            if enabled {
                let thumb = self.bar(horizontal)?.thumb;
                let active = self.bar_hover == Some(horizontal)
                    || self.drag.is_some_and(|(dragged, _)| dragged == horizontal);
                painter.fill_rect(thumb, scrollbar::thumb_color(painter, active));
            }
        }
        Ok(())
    }
    fn inspect(&self) -> ComponentDetails {
        let mut details = ComponentDetails::default();
        details.properties.insert(
            "scroll_x".into(),
            InspectValue::Number(f64::from(self.offset.get().x())),
        );
        details.properties.insert(
            "scroll_y".into(),
            InspectValue::Number(f64::from(self.offset.get().y())),
        );
        let (horizontal, vertical) = self.bars();
        details.semantics = Some(
            Semantics::new(SemanticRole::ScrollArea)
                .enabled_actions((horizontal || vertical).then_some(SemanticAction::Scroll)),
        );
        details
    }
}

/// Noninteractive content separator with themed line width and surrounding space.
/// Defaults to a horizontal line. Parent layouts still own their inter-child gap;
/// use `spacing(Length::from(0))` to remove this separator's own breathing room.
///
/// Re-describing the divider under the same key applies its orientation and
/// spacing.
#[must_use]
pub struct Divider {
    vertical: bool,
    spacing: Spacing,
    inset: Cell<f32>,
    thickness: Cell<f32>,
}
impl Default for Divider {
    fn default() -> Self {
        Self::new()
    }
}
impl Divider {
    /// Horizontal separator with SectionGap spacing on each side of the line.
    pub fn new() -> Self {
        Self {
            vertical: false,
            spacing: Spacing::Theme(MetricRole::SectionGap),
            inset: Cell::new(0.0),
            thickness: Cell::new(0.0),
        }
    }
    /// Selects a vertical separator for use in a row or explicitly sized container.
    pub fn vertical(mut self) -> Self {
        self.vertical = true;
        self
    }
    /// Sets equal space on both sides of the line; fixed or live theme role.
    pub fn spacing(mut self, value: impl Into<Spacing>) -> Self {
        self.spacing = value.into();
        self
    }
}
impl Component for Divider {
    fn fills_row_height(&self) -> bool {
        self.vertical
    }
    fn reconcile(&mut self, next: Self) -> Option<Invalidation> {
        let Self {
            vertical,
            spacing,
            inset: _,
            thickness: _,
        } = next;
        let changed = self.vertical != vertical || self.spacing != spacing;
        self.vertical = vertical;
        self.spacing = spacing;
        changed.then_some(Invalidation::Layout)
    }
    fn inspect(&self) -> ComponentDetails {
        ComponentDetails {
            semantics: Some(Semantics::new(SemanticRole::Separator)),
            ..ComponentDetails::default()
        }
    }
    fn measure_limits(&self, cx: &MeasureContext<'_>) -> Result<SizeLimits, RuntimeError> {
        let extent = 2.0 * self.spacing.resolve(cx) + cx.metric(MetricRole::BorderWidth);
        Ok(SizeLimits::at_least(if self.vertical {
            Size::new(extent, 0.0)?
        } else {
            Size::new(0.0, extent)?
        }))
    }

    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
    fn measure(
        &self,
        cx: &MeasureContext<'_>,
        available: Constraints,
    ) -> Result<Size, RuntimeError> {
        let inset = self.spacing.resolve(cx);
        let thickness = cx.metric(MetricRole::BorderWidth);
        self.inset.set(inset);
        self.thickness.set(thickness);
        let extent = 2.0 * inset + thickness;
        Ok(if self.vertical {
            Size::new(
                extent,
                available
                    .height()
                    .unwrap_or_else(|| cx.metric(MetricRole::ControlHeight)),
            )?
        } else {
            Size::new(available.width().unwrap_or(0.0), extent)?
        })
    }
    fn paint(&self, painter: &mut Painter) -> Result<(), DrawError> {
        if let Some(size) = painter.size() {
            let inset = self.inset.get();
            let rect = if self.vertical {
                Rect::new(
                    inset.min(size.width()),
                    0.0,
                    self.thickness.get().min((size.width() - inset).max(0.0)),
                    size.height(),
                )?
            } else {
                Rect::new(
                    0.0,
                    inset.min(size.height()),
                    size.width(),
                    self.thickness.get().min((size.height() - inset).max(0.0)),
                )?
            };
            painter.fill_rect(rect, painter.color(ColorRole::Divider));
        }
        Ok(())
    }
}
