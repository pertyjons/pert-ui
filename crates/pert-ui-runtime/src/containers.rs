//! Opinionated layout primitives, implemented through public component callbacks.
use crate::layout::rect;
use crate::{
    ArrangeContext, Color, Component, Constraints, DrawError, Element, LayoutError, MeasureContext,
    Painter, Rect, RuntimeError, Shape, Size, UpdateContext,
};

use crate::{MetricRole, Spacing};

/// Validated finite nonnegative extent in logical pixels.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Length(f32);
impl From<u16> for Length {
    fn from(value: u16) -> Self {
        Self(f32::from(value))
    }
}
impl Length {
    /// Validates a logical extent, rejecting non-finite and negative input.
    ///
    /// # Errors
    /// [`GeometryError::NonFinite`](crate::GeometryError::NonFinite) or
    /// [`GeometryError::Negative`](crate::GeometryError::Negative).
    pub fn new(value: f32) -> Result<Self, crate::GeometryError> {
        Size::new(value, 0.0)?;
        Ok(Self(value))
    }
    /// Logical-pixel value, before DPI/user scaling.
    #[must_use]
    pub const fn get(self) -> f32 {
        self.0
    }
    /// A constant the crate knows to be finite and nonnegative.
    pub(crate) const fn trusted(value: f32) -> Self {
        Self(value)
    }
}
impl TryFrom<f32> for Length {
    type Error = crate::GeometryError;
    /// Validates like [`Length::new`].
    fn try_from(value: f32) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}
impl From<Length> for f32 {
    /// The extent in logical pixels, as [`Length::get`] returns it.
    fn from(length: Length) -> Self {
        length.0
    }
}

/// A transparent single-child box with optional fill, border, padding and size limits.
///
/// Default has no child, decoration, padding or size override. Children are
/// clipped to the allocated rectangle. Child hard minimums plus padding contribute to the
/// outer minimum, even with a smaller preferred size. Explicit outer maximums
/// cap that contribution; conflicting child overflow is clipped. Hidden children
/// contribute no minimum, while padding and explicit limits remain.
/// A [`Self::border_role`] turns the box into a rounded panel: the fill and the
/// border both use the theme corner radius, and the border is painted above the
/// child at the theme border width, inside the allocation. Padding at least as
/// wide as the border keeps content clear of it.
///
/// Re-describing the container under the same key applies its background, color
/// roles, background binding, padding, preferred size, limits and cursor; its
/// child reconciles by key.
#[must_use]
#[derive(Default)]
pub struct Container {
    children: Vec<Element>,
    background: Option<Color>,
    background_role: Option<crate::ColorRole>,
    background_binding: Option<crate::ReadSignal<crate::ColorRole>>,
    /// The role the binding held at the last update.
    bound_role: Option<crate::ColorRole>,
    border_role: Option<crate::ColorRole>,
    painted_background: std::cell::Cell<Option<Color>>,
    painted_border: std::cell::Cell<Option<(f32, Color)>>,
    padding: Option<Spacing>,
    resolved_padding: std::cell::Cell<Option<f32>>,
    preferred: Option<Size>,
    limits: crate::SizeLimits,
    cursor: Option<crate::CursorIcon>,
}
description_debug!(Container);
impl Container {
    /// Wraps a child without changing its appearance or intrinsic size.
    pub fn new(child: Element) -> Self {
        Self {
            children: vec![child],
            ..Self::default()
        }
    }
    /// Reports `icon` for pointer positions inside this container.
    ///
    /// Cursor resolution asks the deepest component under the pointer first and
    /// falls back to its presentation ancestors, so a container that covers another
    /// component's area keeps that ancestor's cursor unless it answers itself. Set
    /// this on a panel that floats above unrelated content, such as a menu over a
    /// text field, so the pointer does not keep the editor's caret shape. Built-in
    /// transient surfaces use `CursorIcon::Default` for exactly that reason.
    pub fn cursor(mut self, icon: crate::CursorIcon) -> Self {
        self.cursor = Some(icon);
        self
    }
    /// Fills the allocated rectangle with a validated linear color.
    ///
    /// Replaces an earlier background, role or role binding.
    pub fn background(mut self, color: Color) -> Self {
        self.background = Some(color);
        self.background_role = None;
        self.background_binding = None;
        self
    }
    /// Fills with a theme role, replacing a literal background override.
    /// Changes to this role automatically repaint the container; default is transparent.
    /// Replaces an earlier background, role or role binding.
    pub fn background_role(mut self, role: crate::ColorRole) -> Self {
        self.background_role = Some(role);
        self.background = None;
        self.background_binding = None;
        self
    }
    /// Fills with the theme role `role` holds, following it: a new role, or a
    /// theme change to the current one, repaints the container without layout
    /// or a new description, so a list can mark its selected item by binding
    /// each item's role to the selection. Replaces an earlier background, role
    /// or role binding, as they replace this one.
    ///
    /// The binding is read when the container updates and released when it is
    /// replaced or the container unmounts. A binding from another runtime fails
    /// that update with [`RuntimeError::ForeignRuntime`].
    ///
    /// ```
    /// # use pert_ui_runtime as pert_ui;
    /// use pert_ui::{ColorRole, Container, Element, Label, Runtime, Viewport};
    /// use std::time::Duration;
    /// let mut ui = Runtime::new();
    /// ui.set_viewport(Viewport::new(200.0, 100.0, 1.0)?)?;
    /// let selected = ui.signal(false);
    /// let role = selected.map(|on| if *on { ColorRole::PrimaryContainer } else { ColorRole::Surface })?;
    /// ui.mount_tree(Element::new("strip", Container::new(Element::new("name", Label::new("Strip")))
    ///     .background_role_signal(role)))?;
    /// ui.flush(Duration::ZERO)?;
    /// selected.set(true)?; // repaints the strip; no redescription
    /// ui.flush(Duration::ZERO)?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn background_role_signal(
        mut self,
        role: impl Into<crate::ReadSignal<crate::ColorRole>>,
    ) -> Self {
        self.background_binding = Some(role.into());
        self.background = None;
        self.background_role = None;
        self
    }
    /// Outlines the allocation with a theme color at `MetricRole::BorderWidth`,
    /// rounded by `MetricRole::CornerRadius`; the background fill takes the same
    /// rounded shape. Changes to either role repaint the container. Default is
    /// no border and a square fill. `ColorRole::PanelBorder` is the resting
    /// panel outline the built-in transient surfaces use. After the first paint,
    /// inspection reports the stroke through `BoxModelInspection::border`.
    pub fn border_role(mut self, role: crate::ColorRole) -> Self {
        self.border_role = Some(role);
        self
    }
    /// Uses equal padding on all four sides: a fixed [`Length`] in logical
    /// pixels, written as a `u16` such as `.padding(24)` or a validated
    /// [`Length`], or a [`MetricRole`] resolved from the theme at each layout,
    /// as [`Spacing`] describes. The default is none; no margin is implicit, and
    /// `MetricRole::ContentMargin` suits an outer content boundary. Each call
    /// replaces the earlier padding.
    pub fn padding(mut self, padding: impl Into<Spacing>) -> Self {
        self.padding = Some(padding.into());
        self
    }
    fn inset(&self, cx: &MeasureContext<'_>) -> f32 {
        let value = self.padding.map_or(0.0, |padding| padding.resolve(cx));
        self.resolved_padding.set(Some(value));
        value
    }
    /// Sets a preferred outer size. Parent allocation can differ from this preference.
    pub fn preferred_size(mut self, size: Size) -> Self {
        self.preferred = Some(size);
        self
    }
    /// Applies already validated hard limits without a second fallible conversion.
    /// Explicit maximums cap propagated child minimums and padding; overflowing
    /// child content is clipped to this container.
    pub fn size_limits(mut self, limits: crate::SizeLimits) -> Self {
        self.limits = limits;
        self
    }
    /// Sets validated outer size limits; `None` means no upper limit.
    /// Child minimums plus padding can raise the outer minimum, up to these
    /// explicit maximums. A conflicting child minimum is clipped.
    ///
    /// # Errors
    /// [`GeometryError::InvertedBounds`](crate::GeometryError::InvertedBounds) if
    /// a minimum exceeds its corresponding maximum.
    pub fn limits(
        mut self,
        minimum: Size,
        maximum: Option<Size>,
    ) -> Result<Self, crate::GeometryError> {
        self.limits = crate::SizeLimits::new(
            minimum,
            maximum.map_or(Constraints::default(), Constraints::bounded),
        )?;
        Ok(self)
    }
}
impl Component for Container {
    fn reconcile(&mut self, next: Self) -> Option<crate::Invalidation> {
        let layout = self.padding != next.padding
            || self.preferred != next.preferred
            || self.limits != next.limits;
        let rebound =
            !crate::ReadSignal::same_optional(&self.background_binding, &next.background_binding);
        let paint = self.background != next.background
            || self.background_role != next.background_role
            || rebound
            || self.border_role != next.border_role
            || self.cursor != next.cursor;
        self.background = next.background;
        self.background_role = next.background_role;
        if rebound {
            self.background_binding = next.background_binding;
            self.bound_role = None;
        }
        self.border_role = next.border_role;
        self.padding = next.padding;
        self.preferred = next.preferred;
        self.limits = next.limits;
        self.cursor = next.cursor;
        if layout {
            Some(crate::Invalidation::Layout)
        } else if paint {
            Some(crate::Invalidation::Paint)
        } else {
            None
        }
    }
    fn size_limits(&self) -> crate::SizeLimits {
        self.limits
    }
    fn measure_limits(&self, cx: &MeasureContext<'_>) -> Result<crate::SizeLimits, RuntimeError> {
        let child = if cx.is_empty() {
            Size::ZERO
        } else {
            cx.limits(0)?.minimum()
        };
        let inset = 2.0 * f64::from(self.inset(cx));
        let own = self.limits.minimum();
        let maximum = self.limits.maximum();
        let axis = |child: f32, own: f32, maximum: Option<f32>| {
            (f64::from(child) + inset)
                .max(f64::from(own))
                .min(maximum.map_or(f64::INFINITY, f64::from)) as f32
        };
        let minimum = Size::new(
            axis(child.width(), own.width(), maximum.width()),
            axis(child.height(), own.height(), maximum.height()),
        )?;
        Ok(crate::SizeLimits::new(minimum, maximum)?)
    }
    fn cursor(&self, _: crate::Point) -> Option<crate::CursorIcon> {
        self.cursor
    }
    fn keyboard_event(
        &mut self,
        _: &mut crate::EventContext,
        _: crate::KeyEvent,
        _: &crate::KeyboardState,
    ) -> Result<crate::InputOutcome, RuntimeError> {
        Ok(crate::InputOutcome::Ignored)
    }
    fn take_children(&mut self) -> Vec<Element> {
        std::mem::take(&mut self.children)
    }
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        self.bound_role = match &self.background_binding {
            Some(binding) => Some(cx.read_paint(binding)?),
            None => None,
        };
        Ok(())
    }
    fn measure(
        &self,
        cx: &MeasureContext<'_>,
        available: Constraints,
    ) -> Result<Size, RuntimeError> {
        let inset = 2.0 * f64::from(self.inset(cx));
        let available = Constraints::new(
            available
                .width()
                .map(|v| (f64::from(v) - inset).max(0.0) as f32),
            available
                .height()
                .map(|v| (f64::from(v) - inset).max(0.0) as f32),
        )?;
        let child = if cx.is_empty() {
            Size::ZERO
        } else {
            cx.measure(0, available)?
        };
        let limits = self.measure_limits(cx)?;
        let preferred = if let Some(preferred) = self.preferred {
            preferred
        } else {
            let maximum = limits.maximum();
            let axis = |child: f32, maximum: Option<f32>| {
                (f64::from(child) + inset).min(maximum.map_or(f64::INFINITY, f64::from)) as f32
            };
            Size::new(
                axis(child.width(), maximum.width()),
                axis(child.height(), maximum.height()),
            )?
        };
        Ok(limits.constrain(preferred))
    }
    fn arrange(&self, cx: &mut ArrangeContext<'_>, size: Size) -> Result<(), RuntimeError> {
        if cx.len() > 1 {
            return Err(LayoutError::InvalidChild.into());
        }
        if !cx.is_empty() {
            let p = self.inset(cx.measurement());
            let available = Size::new(
                (size.width() - 2.0 * p).max(0.0),
                (size.height() - 2.0 * p).max(0.0),
            )?;
            cx.place(0, rect(p, p, available.width(), available.height())?)?;
        }
        Ok(())
    }
    fn paint(&self, painter: &mut Painter) -> Result<(), DrawError> {
        let color = self.background.or_else(|| {
            self.background_role
                .or(self.bound_role)
                .map(|role| painter.color(role))
        });
        self.painted_background.set(color);
        if let (Some(color), size) = (color, painter.size()) {
            let bounds = Rect::from_size(size);
            if self.border_role.is_some() {
                painter.fill_shape(bounds, panel_shape(painter)?, color)?;
            } else {
                painter.fill_rect(bounds, color);
            }
        }
        Ok(())
    }
    fn paint_foreground(&self, painter: &mut Painter) -> Result<(), DrawError> {
        if let (Some(role), size) = (self.border_role, painter.size()) {
            let width = Length::new(painter.metric(MetricRole::BorderWidth))?;
            let color = painter.color(role);
            painter.stroke_shape(Rect::from_size(size), panel_shape(painter)?, width, color)?;
            self.painted_border.set(Some((width.get(), color)));
        } else {
            self.painted_border.set(None);
        }
        Ok(())
    }
    fn inspect(&self) -> crate::ComponentDetails {
        let mut details = crate::ComponentDetails::default();
        details.box_model.padding = self.resolved_padding.get().map(|value| [value; 4]);
        details.box_model.background = self
            .background
            .or(self.painted_background.get())
            .map(Color::channels);
        details.box_model.border =
            self.painted_border
                .get()
                .map(|(width, color)| crate::BorderInspection {
                    width,
                    color: color.channels(),
                });
        details
    }
}

/// The rounded panel outline shared by bordered containers.
fn panel_shape(painter: &Painter) -> Result<Shape, DrawError> {
    Ok(Shape::Rounded(Length::new(
        painter.metric(MetricRole::CornerRadius),
    )?))
}

#[derive(Clone, Copy)]
enum Flow {
    Row,
    Column,
    Wrap,
}

/// Where a [`Row`] or [`Column`] puts its children along its main axis when
/// they leave space over: space that no child with an
/// [`Element::with_grow`] weight takes. Without space over, every choice
/// places like `Start`.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Justify {
    /// Children start at the leading edge, left or top; the default.
    #[default]
    Start,
    /// Children are centered as a group.
    Center,
    /// Children end at the trailing edge, right or bottom.
    End,
    /// The first child starts at the leading edge, the last ends at the
    /// trailing edge, and the space over is shared between the gaps. A single
    /// child starts at the leading edge.
    SpaceBetween,
}

/// Where a [`Row`] or [`Column`] places each child across its main axis.
///
/// A column aligns children within its width. A row aligns them within the
/// height of its tallest child, except that `Stretch` fills the height the row
/// is given: in a column that grows the row, or under a bounded allocation,
/// every child reaches it. `Stretch` gives each child that whole extent, up to
/// the child's own maximum size and never below its minimum.
///
/// A row's own measured height stays that of its tallest child, so stretching
/// fills space a parent allocates without asking for more; a row whose parent
/// gives it only its natural height, as a column that does not grow it does,
/// stretches its children to the tallest of them. Scroll content is given at
/// least the visible height, so a row in a [`ScrollArea`](crate::ScrollArea) fills that.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Align {
    /// At the leading edge: left in a column, top in a row. A column's default.
    Start,
    /// Centered: a row's default.
    Center,
    /// At the trailing edge: right in a column, bottom in a row.
    End,
    /// Across the whole extent: a column's width, or the height a row is
    /// allocated.
    Stretch,
}

/// The main-axis and cross-axis choices of a row or column.
#[derive(Clone, Copy, PartialEq)]
struct FlowStyle {
    gap: f32,
    justify: Justify,
    align: Align,
}
fn flow_limits(
    cx: &MeasureContext<'_>,
    flow: Flow,
    gap: f32,
) -> Result<crate::SizeLimits, RuntimeError> {
    let mut width = 0.0f32;
    let mut height = 0.0f32;
    for index in 0..cx.len() {
        let min = cx.limits(index)?.minimum();
        width = if matches!(flow, Flow::Row) {
            width + min.width()
        } else {
            width.max(min.width())
        };
        height = if matches!(flow, Flow::Column) {
            height + min.height()
        } else {
            height.max(min.height())
        };
    }
    let gaps = gap * cx.len().saturating_sub(1) as f32;
    if matches!(flow, Flow::Row) {
        width += gaps;
    }
    if matches!(flow, Flow::Column) {
        height += gaps;
    }
    Ok(crate::SizeLimits::at_least(Size::new(width, height)?))
}
fn flow_rects(
    cx: &MeasureContext<'_>,
    available: Constraints,
    flow: Flow,
    style: FlowStyle,
) -> Result<(Size, Vec<Rect>), RuntimeError> {
    let gap = style.gap;
    // A row's children: each one's final allocated width and height, and the
    // shortfall it took from its grown children, if one applied.
    let mut row_sizes = Vec::new();
    let mut row_shortfall = None;
    if matches!(flow, Flow::Row) {
        (row_sizes, row_shortfall) = row_sizes_of(cx, available, gap)?;
    }
    let mut out = Vec::with_capacity(cx.len());
    let mut row_start = 0;
    let (mut x, mut y, mut row_height, mut width, mut height) = (0.0, 0.0, 0.0f32, 0.0f32, 0.0f32);
    for index in 0..cx.len() {
        let child_available = match flow {
            Flow::Row => Constraints::new(None, available.height())?,
            Flow::Column => Constraints::new(available.width(), None)?,
            Flow::Wrap => available,
        };
        let size = if let Some(size) = row_sizes.get(index) {
            *size
        } else {
            cx.measure(index, child_available)?
        };
        if matches!(flow, Flow::Wrap)
            && x > 0.0
            && available
                .width()
                .is_some_and(|limit| x + size.width() > limit)
        {
            center_line(&mut out[row_start..], row_height)?;
            row_start = out.len();
            y += row_height + gap;
            x = 0.0;
            row_height = 0.0;
        }
        let minimum = cx.limits(index)?.minimum();
        let allocation_width = if matches!(flow, Flow::Column) {
            available.width().map_or(size.width(), |width| {
                size.width().min(width).max(minimum.width())
            })
        } else {
            size.width()
        };
        out.push(rect(x, y, allocation_width, size.height())?);
        width = width.max(x + size.width());
        height = height.max(y + size.height());
        row_height = row_height.max(size.height());
        match flow {
            Flow::Column => y += size.height() + gap,
            _ => x += size.width() + gap,
        }
    }
    if matches!(flow, Flow::Wrap) {
        center_line(&mut out[row_start..], row_height)?;
    }
    let natural = Size::new(width, height)?;
    if matches!(flow, Flow::Wrap) {
        return Ok((natural, out));
    }
    let shortfall = match flow {
        Flow::Column => match available.height() {
            Some(limit) => {
                let heights: Vec<f32> = out.iter().map(|bounds| bounds.height()).collect();
                take_shortfall(cx, &heights, gap, limit, Axis::Vertical)?
            }
            None => None,
        },
        _ => row_shortfall,
    };
    let size = flex(cx, available, flow, style, natural, shortfall, &mut out)?;
    Ok((size, out))
}

/// The axis a shortfall is taken along: a row's width or a column's height.
#[derive(Clone, Copy)]
enum Axis {
    Horizontal,
    Vertical,
}

/// One grown child that can give up main-axis length to a shortfall.
struct Shrinkable {
    index: usize,
    length: f64,
    weight: f64,
    minimum: f64,
}

/// What a bounded row or column whose natural lengths overflow took from its
/// grown children.
struct Shortfall {
    /// Each child's final main-axis length where it gave some up.
    lengths: Vec<Option<f32>>,
    /// Whether the whole deficit was taken, so the container is exactly its
    /// limit. A row clears it when a fill child's width changed at its fill
    /// height, since its final allocations then no longer sum to the limit.
    absorbed: bool,
}

/// Takes `deficit` from the grown children by weight, none below its
/// minimum, sharing what a minimum stops again among the others. The
/// arithmetic is in f64 and yields final lengths: a natural length of hundreds
/// of millions of pixels less nearly as much would otherwise lose the few
/// hundred that remain to f32 cancellation.
fn shortfall(mut deficit: f64, count: usize, mut shrinking: Vec<Shrinkable>) -> Shortfall {
    let mut taken = vec![0.0f64; count];
    let mut lengths = vec![0.0f64; count];
    for child in &shrinking {
        lengths[child.index] = child.length;
    }
    let mut absorbed = false;
    while !shrinking.is_empty() {
        let total: f64 = shrinking.iter().map(|child| child.weight).sum();
        // How much each may still give before its minimum.
        let rooms: Vec<f64> = shrinking
            .iter()
            .map(|child| (child.length - taken[child.index] - child.minimum).max(0.0))
            .collect();
        let share = |weight: f64| deficit * (weight / total);
        let capped: Vec<usize> = (0..shrinking.len())
            .filter(|&position| share(shrinking[position].weight) >= rooms[position])
            .collect();
        if capped.is_empty() {
            for child in &shrinking {
                taken[child.index] += share(child.weight);
            }
            absorbed = true;
            break;
        }
        for position in capped.into_iter().rev() {
            let child = shrinking.remove(position);
            taken[child.index] += rooms[position];
            deficit -= rooms[position];
        }
        if deficit <= 0.0 {
            absorbed = true;
            break;
        }
    }
    Shortfall {
        lengths: taken
            .into_iter()
            .zip(lengths)
            .map(|(taken, length)| (taken > 0.0).then_some((length - taken) as f32))
            .collect(),
        absorbed,
    }
}

/// The shortfall of a bounded row or column whose children's natural
/// main-axis `lengths` and gaps exceed `limit`, taken from its grown children;
/// `None` when they fit or no child grows.
fn take_shortfall(
    cx: &MeasureContext<'_>,
    lengths: &[f32],
    gap: f32,
    limit: f32,
    axis: Axis,
) -> Result<Option<Shortfall>, RuntimeError> {
    let count = lengths.len();
    let mut shrinking = Vec::new();
    for (index, length) in lengths.iter().enumerate() {
        let weight = cx.grow(index)?;
        if weight > 0 {
            let minimum = cx.limits(index)?.minimum();
            let minimum = match axis {
                Axis::Horizontal => minimum.width(),
                Axis::Vertical => minimum.height(),
            };
            shrinking.push(Shrinkable {
                index,
                length: f64::from(*length),
                weight: f64::from(weight),
                minimum: f64::from(minimum),
            });
        }
    }
    if shrinking.is_empty() {
        return Ok(None);
    }
    let used = lengths.iter().map(|length| f64::from(*length)).sum::<f64>()
        + f64::from(gap) * count.saturating_sub(1) as f64;
    let deficit = used - f64::from(limit);
    Ok((deficit > 0.0).then(|| shortfall(deficit, count, shrinking)))
}

/// Measures a row's children and returns each one's final allocated width and
/// height, with the shortfall the row took when its natural widths overflow
/// a bounded width.
///
/// Ordinary children are measured at the offered height and fill children
/// without one; whether the row shrinks, and by how much, is decided once from
/// those natural widths. Each ordinary child that gave up width is measured
/// again at its final width, which gives its height. Fill children are then
/// measured at the tallest resolved height, capped by the offered height: at
/// their final width when they gave some up, and otherwise unbounded, keeping
/// the width they report there.
fn row_sizes_of(
    cx: &MeasureContext<'_>,
    available: Constraints,
    gap: f32,
) -> Result<(Vec<Size>, Option<Shortfall>), RuntimeError> {
    let mut sizes = Vec::with_capacity(cx.len());
    let mut fills = Vec::with_capacity(cx.len());
    for index in 0..cx.len() {
        let fill = cx.fills_row_height(index)?;
        let offered = Constraints::new(None, if fill { None } else { available.height() })?;
        sizes.push(cx.measure(index, offered)?);
        fills.push(fill);
    }
    let mut shortfall = match available.width() {
        Some(limit) => {
            let widths: Vec<f32> = sizes.iter().map(|size| size.width()).collect();
            take_shortfall(cx, &widths, gap, limit, Axis::Horizontal)?
        }
        None => None,
    };
    let finals = |index: usize| {
        shortfall
            .as_ref()
            .and_then(|shortfall| shortfall.lengths[index])
    };
    for (index, size) in sizes.iter_mut().enumerate() {
        if !fills[index]
            && let Some(width) = finals(index)
        {
            let measured = cx.measure(index, Constraints::new(Some(width), available.height())?)?;
            *size = Size::new(width, measured.height())?;
        }
    }
    // The fill offer: the tallest provisional height, capped by the offer.
    let tallest = sizes
        .iter()
        .fold(0.0f32, |tallest, size| tallest.max(size.height()));
    let height = available
        .height()
        .map_or(tallest, |limit| tallest.min(limit));
    let mut kept = true;
    for (index, size) in sizes.iter_mut().enumerate() {
        if fills[index] {
            let width = finals(index);
            let measured = cx.measure(index, Constraints::new(width, Some(height))?)?;
            *size = match width {
                Some(width) => Size::new(width, measured.height())?,
                None => {
                    kept &= measured.width() == size.width();
                    measured
                }
            };
        }
    }
    if let Some(shortfall) = shortfall.as_mut() {
        shortfall.absorbed &= kept;
    }
    Ok((sizes, shortfall))
}

/// Grows, justifies and aligns the naturally placed children of a row or
/// column, returning the container's size. `shortfall` is what the container
/// took from its grown children when their natural lengths overflow its
/// bounded main axis; a row's `out` already holds its final widths.
fn flex(
    cx: &MeasureContext<'_>,
    available: Constraints,
    flow: Flow,
    style: FlowStyle,
    natural: Size,
    shortfall: Option<Shortfall>,
    out: &mut [Rect],
) -> Result<Size, RuntimeError> {
    let column = matches!(flow, Flow::Column);
    let count = out.len();
    let main = |bounds: &Rect| {
        if column {
            bounds.height()
        } else {
            bounds.width()
        }
    };
    let lengths: Vec<f32> = out.iter().map(main).collect();
    let gaps = style.gap * count.saturating_sub(1) as f32;
    let used = lengths.iter().sum::<f32>() + gaps;
    let limit = if column {
        available.height()
    } else {
        available.width()
    };
    let mut spare = limit.map_or(0.0, |limit| (limit - used).max(0.0));
    // A row that shrank grows no child back. Its spare space, which
    // `Justify` places, is what its final widths leave: none when the whole
    // deficit was taken, and otherwise only what a fill child gave up at its
    // fill height.
    let row_shrank = !column && shortfall.is_some();
    if row_shrank
        && shortfall
            .as_ref()
            .is_some_and(|shortfall| shortfall.absorbed)
    {
        spare = 0.0;
    }
    let mut extra = vec![0.0f32; count];
    let mut growing = Vec::new();
    for index in 0..count {
        let weight = cx.grow(index)?;
        if weight > 0 {
            let maximum = cx.limits(index)?.maximum();
            let maximum = if column {
                maximum.height()
            } else {
                maximum.width()
            };
            growing.push((index, f32::from(weight), maximum));
        }
    }
    let grows = !growing.is_empty();
    if row_shrank {
        growing.clear();
    }
    // Share the spare space by weight; a child that reaches its maximum takes
    // only its room, and the rest is shared again among the others.
    while spare > 0.0 && !growing.is_empty() {
        let total: f32 = growing.iter().map(|(_, weight, _)| weight).sum();
        // How much more each growing child may take before its maximum.
        let rooms: Vec<f32> = growing
            .iter()
            .map(|(index, _, maximum)| {
                maximum.map_or(f32::INFINITY, |maximum| {
                    (maximum - lengths[*index] - extra[*index]).max(0.0)
                })
            })
            .collect();
        // The weight's fraction first, so a large spare space cannot overflow.
        let share = |weight: f32| spare * (weight / total);
        let capped: Vec<usize> = (0..growing.len())
            .filter(|&position| share(growing[position].1) >= rooms[position])
            .collect();
        if capped.is_empty() {
            for (index, weight, _) in &growing {
                extra[*index] += share(*weight);
            }
            spare = 0.0;
            break;
        }
        for position in capped.into_iter().rev() {
            let (index, _, _) = growing.remove(position);
            extra[index] += rooms[position];
            spare -= rooms[position];
        }
        spare = spare.max(0.0);
    }
    // A bounded row or column whose natural lengths overflow took the deficit
    // from its grown children by weight, none below its minimum, so a grown
    // child fills the rest instead of keeping its whole natural length (which
    // for a collection is its whole content). A row measured each child it
    // shrank again at its final width, so a child that wraps has the height
    // of its new lines; a column's shrunk children keep the widths they were
    // measured at.
    let (shrunk, absorbed) = match shortfall {
        Some(shortfall) => (shortfall.lengths, shortfall.absorbed),
        None => (vec![None; count], false),
    };
    let (start, spacing) = match style.justify {
        Justify::Start => (0.0, style.gap),
        Justify::Center => (spare / 2.0, style.gap),
        Justify::End => (spare, style.gap),
        Justify::SpaceBetween if count > 1 => (0.0, style.gap + spare / (count - 1) as f32),
        Justify::SpaceBetween => (0.0, style.gap),
    };
    // A column aligns within its width. A row aligns within its tallest child,
    // except that stretching fills the height the row is given: its
    // allocation when arranged, or a bounded offer while it is measured,
    // which only places children and never changes the row's own size.
    let cross_extent = match (column, style.align) {
        (true, _) => available.width().unwrap_or(natural.width()),
        (false, Align::Stretch) => available.height().unwrap_or(natural.height()),
        (false, _) => natural.height(),
    };
    let mut position = start;
    for (index, bounds) in out.iter_mut().enumerate() {
        let length = shrunk[index].unwrap_or(lengths[index] + extra[index]);
        let [_, _, width, height] = bounds.values();
        let cross = if column { width } else { height };
        let maximum = cx.limits(index)?.maximum();
        let maximum = if column {
            maximum.width()
        } else {
            maximum.height()
        };
        let (offset, cross) = match style.align {
            Align::Start => (0.0, cross),
            Align::Center => ((cross_extent - cross) / 2.0, cross),
            Align::End => (cross_extent - cross, cross),
            Align::Stretch => {
                let minimum = cx.limits(index)?.minimum();
                let minimum = if column {
                    minimum.width()
                } else {
                    minimum.height()
                };
                (
                    0.0,
                    maximum
                        .map_or(cross_extent, |maximum| cross_extent.min(maximum))
                        .max(minimum),
                )
            }
        };
        *bounds = if column {
            rect(offset, position, cross, length)?
        } else {
            rect(position, offset, length, cross)?
        };
        position += length + spacing;
    }
    // A container that grows or places children by its spare space fills its
    // bounded main axis; otherwise it keeps its natural size.
    let fills = grows || style.justify != Justify::Start;
    // A container that shrank is its limit when the deficit was absorbed (a
    // row's only while every fill child kept its natural width), and
    // otherwise the sum of its final lengths: its floor (ungrown natural
    // lengths, grown minimums, a row's fill widths and the gaps) exceeds the
    // limit, or a row's fill child narrowed at its fill height.
    let placed = if absorbed {
        limit.unwrap_or(used)
    } else if shrunk.iter().any(Option::is_some) {
        (shrunk
            .iter()
            .zip(&lengths)
            .map(|(shrunk, length)| f64::from(shrunk.unwrap_or(*length)))
            .sum::<f64>()
            + f64::from(style.gap) * count.saturating_sub(1) as f64) as f32
    } else {
        used
    };
    let main = match limit {
        Some(limit) if fills => limit.max(placed),
        _ => {
            if column {
                natural.height()
            } else {
                natural.width()
            }
        }
    };
    Ok(if column {
        Size::new(natural.width(), main)?
    } else {
        Size::new(main, natural.height())?
    })
}

fn center_line(placements: &mut [Rect], height: f32) -> Result<(), RuntimeError> {
    for bounds in placements {
        let [x, y, width, child_height] = bounds.values();
        *bounds = rect(x, y + (height - child_height) / 2.0, width, child_height)?;
    }
    Ok(())
}

macro_rules! flow_component {
    ($name:ident, $flow:ident, $doc:literal) => {
        #[doc = $doc]
        #[must_use]
        #[derive(Default)]
        pub struct $name {
            children: Vec<Element>,
            gap: Option<Spacing>,
            justify: Justify,
            align: Option<Align>,
        }
        description_debug!($name);
        impl $name {
            /// Collects ordered children; sibling keys must be unique at mounting.
            pub fn new(children: impl IntoIterator<Item = Element>) -> Self {
                Self {
                    children: children.into_iter().collect(),
                    gap: None,
                    justify: Justify::Start,
                    align: None,
                }
            }
            /// Overrides the inter-child gap with a fixed length or a live theme role.
            /// Default is `MetricRole::ItemGap`. Wrap uses it for both items and rows.
            pub fn gap(mut self, gap: impl Into<Spacing>) -> Self {
                self.gap = Some(gap.into());
                self
            }
            fn resolved_gap(&self, cx: &MeasureContext<'_>) -> f32 {
                self.gap
                    .unwrap_or(Spacing::Theme(MetricRole::ItemGap))
                    .resolve(cx)
            }
            fn style(&self, cx: &MeasureContext<'_>) -> FlowStyle {
                FlowStyle {
                    gap: self.resolved_gap(cx),
                    justify: self.justify,
                    align: self.align.unwrap_or(match Flow::$flow {
                        Flow::Row => Align::Center,
                        _ => Align::Start,
                    }),
                }
            }
        }
        impl Component for $name {
            fn reconcile(&mut self, next: Self) -> Option<crate::Invalidation> {
                let changed = self.gap != next.gap
                    || self.justify != next.justify
                    || self.align != next.align;
                self.gap = next.gap;
                self.justify = next.justify;
                self.align = next.align;
                changed.then_some(crate::Invalidation::Layout)
            }
            fn measure_limits(
                &self,
                cx: &MeasureContext<'_>,
            ) -> Result<crate::SizeLimits, RuntimeError> {
                flow_limits(cx, Flow::$flow, self.resolved_gap(cx))
            }
            fn clips_children(&self) -> bool {
                false
            }
            fn keyboard_event(
                &mut self,
                _: &mut crate::EventContext,
                _: crate::KeyEvent,
                _: &crate::KeyboardState,
            ) -> Result<crate::InputOutcome, RuntimeError> {
                Ok(crate::InputOutcome::Ignored)
            }
            fn take_children(&mut self) -> Vec<Element> {
                std::mem::take(&mut self.children)
            }
            fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
                Ok(())
            }
            fn measure(
                &self,
                cx: &MeasureContext<'_>,
                available: Constraints,
            ) -> Result<Size, RuntimeError> {
                Ok(flow_rects(cx, available, Flow::$flow, self.style(cx))?.0)
            }
            fn arrange(&self, cx: &mut ArrangeContext<'_>, size: Size) -> Result<(), RuntimeError> {
                let (_, placements) = flow_rects(
                    cx.measurement(),
                    Constraints::bounded(size),
                    Flow::$flow,
                    self.style(cx.measurement()),
                )?;
                for (index, bounds) in placements.into_iter().enumerate() {
                    cx.place(index, bounds)?;
                }
                Ok(())
            }
        }
    };
}
flow_component!(
    Row,
    Row,
    "One horizontal row with the theme item gap (8 logical pixels at Standard). Children receive the offered height; children opting into Component::fills_row_height contribute their natural height first, then fill the resolved content height up to the offered height. Children are vertically centered against the tallest resolved child, aligning labels with taller controls; with Align::Stretch they instead fill the height the row is allocated. Otherwise unused height below the row remains empty; overflow never implicitly wraps. Children with an [`Element::with_grow`] weight share the spare width, and give up width, down to their minimums, when the natural widths overflow a bounded width; a child that gave up width is measured again at its final width."
);
flow_component!(
    Column,
    Column,
    "One vertical column with the theme item gap (8 logical pixels at Standard) and start alignment; allocated widths fit available space while hard minimums and reported overflow are preserved."
);
/// The main-axis and cross-axis builders of [`Row`] and [`Column`]; [`Wrap`]
/// keeps its own line placement.
macro_rules! flex_builders {
    ($name:ident, $main:literal, $cross_default:literal) => {
        impl $name {
            #[doc = concat!("Places the children along the ", $main, " when they leave space over; [`Justify::Start`] by default.")]
            ///
            /// Only spare space that no child with an [`Element::with_grow`]
            /// weight takes is placed; with spare space and any choice other
            /// than `Start`, the container fills its bounded main axis.
            pub fn justify(mut self, justify: Justify) -> Self {
                self.justify = justify;
                self
            }
            #[doc = concat!("Places each child across the main axis; ", $cross_default, " by default.")]
            pub fn align(mut self, align: Align) -> Self {
                self.align = Some(align);
                self
            }
        }
    };
}
flex_builders!(
    Row,
    "row's width",
    "[`Align::Center`] within the tallest child's height"
);
flex_builders!(
    Column,
    "column's height",
    "[`Align::Start`] within the column's width"
);

flow_component!(
    Wrap,
    Wrap,
    "Horizontal wrapping flow with theme item/row gaps (8 logical pixels at Standard) and horizontal start alignment. Each line vertically centers its children against that line's tallest child. Exact fits stay on the row; oversized children occupy their own row. Hidden children consume no space."
);

/// One grid-column sizing rule. Values are validated at construction.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Track(TrackKind);
#[derive(Clone, Copy, Debug, PartialEq)]
enum TrackKind {
    Content,
    Fixed(Length),
    Weight(f32),
}
impl Track {
    /// Sizes to the largest intrinsic child width in the column: the width a
    /// child measures to without a width constraint. The column keeps that
    /// width even when the grid is given less, so it counts toward the grid's
    /// hard minimum width like a fixed extent.
    pub const CONTENT: Self = Self(TrackKind::Content);
    /// Gives the column a logical extent it keeps even when the grid is given
    /// less width; a larger hard child minimum widens it. The extent counts
    /// toward the grid's hard minimum width.
    #[must_use]
    pub const fn fixed(length: Length) -> Self {
        Self(TrackKind::Fixed(length))
    }
    /// Distributes remaining width by a finite strictly positive weight.
    ///
    /// # Errors
    /// [`LayoutError::InvalidWeight`] for any other weight.
    pub fn weighted(weight: f32) -> Result<Self, LayoutError> {
        if !weight.is_finite() || weight <= 0.0 {
            return Err(LayoutError::InvalidWeight);
        }
        Ok(Self(TrackKind::Weight(weight)))
    }
}

/// A row-major grid with explicit column tracks and theme item gaps (8 logical pixels at Standard).
///
/// Rows use their tallest child; empty trailing cells consume no content. Columns
/// preserve declared hard minimum widths; fixed/weighted tracks constrain wrapping.
/// The grid reports the sum of its fixed extents, the intrinsic widths of its
/// content columns, hard child minimums and gaps as its own hard minimum, so a
/// parent such as a `ScrollArea` allocates the width its columns occupy instead
/// of letting them overflow the grid.
/// Weighted columns share spare finite width. With unbounded width, their
/// intrinsic widths determine the total budget, which is distributed by weight
/// before measuring wrapped row heights. Fixed tracks use their declared extent
/// or larger hard child minimum in both measurement and arrangement.
///
/// Column assignment follows the visible children in their logical order.
/// Hidden children occupy no cells, so hiding a single cell shifts later cells
/// between columns. Hide every cell in a complete row before flushing when
/// later label/value pairs must stay in their original columns; hidden cells
/// keep their retained identities and return to their logical positions when shown.
///
/// ```
/// # use pert_ui_runtime as pert_ui;
/// use pert_ui::{Element, Grid, Label, Length, Runtime, Track, Viewport};
/// use std::time::Duration;
/// let mut ui = Runtime::new();
/// ui.set_viewport(Viewport::new(320.0, 160.0, 1.0)?)?;
/// let grid = ui.mount_tree(Element::new("fields", Grid::new(
///     [Track::fixed(Length::from(120)), Track::fixed(Length::from(120))],
///     [
///         Element::new("first-label", Label::new("First")),
///         Element::new("first-value", Label::new("A")),
///         Element::new("second-label", Label::new("Second")),
///         Element::new("second-value", Label::new("B")),
///     ],
/// )?))?;
/// ui.flush(Duration::ZERO)?;
/// let first_label = ui.find_descendant(&grid, "first-label")?.unwrap();
/// let first_value = ui.find_descendant(&grid, "first-value")?.unwrap();
/// let second_label = ui.find_descendant(&grid, "second-label")?.unwrap();
/// let second_value = ui.find_descendant(&grid, "second-value")?.unwrap();
/// let label_x = ui.bounds(&second_label)?.unwrap().x();
/// let value_x = ui.bounds(&second_value)?.unwrap().x();
/// ui.set_visible(&first_label, false)?;
/// ui.set_visible(&first_value, false)?;
/// ui.flush(Duration::ZERO)?;
/// assert_eq!(ui.bounds(&second_label)?.unwrap().x(), label_x);
/// assert_eq!(ui.bounds(&second_value)?.unwrap().x(), value_x);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
///
/// Re-describing the grid under the same key applies its gap and tracks; its
/// children reconcile by key.
#[must_use]
pub struct Grid {
    gap: Option<Spacing>,
    tracks: Vec<Track>,
    children: Vec<Element>,
}
description_debug!(Grid);
impl Grid {
    /// Creates a grid.
    ///
    /// # Errors
    /// Returns [`LayoutError::NoTracks`] for an empty track list.
    pub fn new(
        tracks: impl IntoIterator<Item = Track>,
        children: impl IntoIterator<Item = Element>,
    ) -> Result<Self, LayoutError> {
        let tracks: Vec<_> = tracks.into_iter().collect();
        if tracks.is_empty() {
            return Err(LayoutError::NoTracks);
        }
        Ok(Self {
            gap: None,
            tracks,
            children: children.into_iter().collect(),
        })
    }
    /// Overrides both row and column gaps with a fixed length or live theme role.
    /// Default is `MetricRole::ItemGap`.
    pub fn gap(mut self, gap: impl Into<Spacing>) -> Self {
        self.gap = Some(gap.into());
        self
    }
    fn placements(
        &self,
        cx: &MeasureContext<'_>,
        available: Constraints,
    ) -> Result<(Size, Vec<Rect>), RuntimeError> {
        if cx.is_empty() {
            return Ok((Size::ZERO, Vec::new()));
        }
        let gap = self
            .gap
            .unwrap_or(Spacing::Theme(MetricRole::ItemGap))
            .resolve(cx);
        let columns = self.tracks.len();
        let mut widths = vec![0.0f32; columns];
        let mut intrinsic_weights = vec![0.0f32; columns];
        let mut sizes = Vec::with_capacity(cx.len());
        for index in 0..cx.len() {
            let column = index % columns;
            let minimum = cx.limits(index)?.minimum();
            let size = if matches!(self.tracks[column].0, TrackKind::Content) {
                cx.measure(index, Constraints::default())?
            } else {
                if available.width().is_none()
                    && matches!(self.tracks[column].0, TrackKind::Weight(_))
                {
                    intrinsic_weights[column] = intrinsic_weights[column]
                        .max(cx.measure(index, Constraints::default())?.width());
                }
                minimum
            };
            widths[column] = widths[column].max(size.width());
            sizes.push(size);
        }
        for (width, track) in widths.iter_mut().zip(&self.tracks) {
            if let TrackKind::Fixed(extent) = track.0 {
                *width = width.max(extent.get());
            }
        }
        let gaps = gap * columns.saturating_sub(1) as f32;
        let base = widths.iter().map(|v| f64::from(*v)).sum::<f64>() + f64::from(gaps);
        // An unbounded measurement chooses a finite intrinsic budget for
        // weighted columns, then distributes it by the same rule as arrange.
        let intrinsic_extra = widths
            .iter()
            .zip(&intrinsic_weights)
            .map(|(minimum, intrinsic)| f64::from((*intrinsic - *minimum).max(0.0)))
            .sum::<f64>();
        let extra = available
            .width()
            .map_or(intrinsic_extra, |w| (f64::from(w) - base).max(0.0));
        let weights: f64 = self
            .tracks
            .iter()
            .map(|t| match t.0 {
                TrackKind::Weight(w) => f64::from(w),
                _ => 0.0,
            })
            .sum();
        if weights > 0.0 {
            for (width, track) in widths.iter_mut().zip(&self.tracks) {
                if let TrackKind::Weight(weight) = track.0 {
                    *width += (extra * f64::from(weight) / weights) as f32;
                }
            }
        }
        // Track width determines wrapping; preferred overflow does not expand
        // fixed/weighted columns. Explicit hard minimums were honored above.
        let mut rows = vec![0.0f32; cx.len().div_ceil(columns)];
        for (index, size) in sizes.iter_mut().enumerate() {
            *size = cx.measure(
                index,
                Constraints::new(Some(widths[index % columns]), None)?,
            )?;
            rows[index / columns] = rows[index / columns].max(size.height());
        }
        let mut out = Vec::with_capacity(cx.len());
        let mut y = 0.0;
        for (row, height) in rows.iter().enumerate() {
            let mut x = 0.0;
            for (column, width) in widths.iter().enumerate() {
                if row * columns + column < cx.len() {
                    out.push(rect(x, y, *width, *height)?);
                }
                x += width + gap;
            }
            y += height + gap;
        }
        Ok((
            Size::new(widths.iter().sum::<f32>() + gaps, (y - gap).max(0.0))?,
            out,
        ))
    }
    /// The size no arrangement goes below: each column keeps its fixed extent,
    /// or for a content column its children's intrinsic widths, and its
    /// children's hard minimum widths, and each row its children's hard minimum
    /// heights, with gaps between them.
    fn minimum(&self, cx: &MeasureContext<'_>) -> Result<Size, RuntimeError> {
        if cx.is_empty() {
            return Ok(Size::ZERO);
        }
        let gap = self
            .gap
            .unwrap_or(Spacing::Theme(MetricRole::ItemGap))
            .resolve(cx);
        let columns = self.tracks.len();
        let mut widths: Vec<f32> = self
            .tracks
            .iter()
            .map(|track| match track.0 {
                TrackKind::Fixed(extent) => extent.get(),
                TrackKind::Content | TrackKind::Weight(_) => 0.0,
            })
            .collect();
        let mut rows = vec![0.0f32; cx.len().div_ceil(columns)];
        for index in 0..cx.len() {
            let minimum = cx.limits(index)?.minimum();
            let column = index % columns;
            // A content column is laid out at its children's intrinsic widths
            // whatever width the grid is given, as `placements` measures them.
            let width = if matches!(self.tracks[column].0, TrackKind::Content) {
                cx.measure(index, Constraints::default())?.width()
            } else {
                minimum.width()
            };
            widths[column] = widths[column].max(width);
            rows[index / columns] = rows[index / columns].max(minimum.height());
        }
        // Summed as `placements` sums them, so an arrangement at this minimum
        // fits its own columns exactly.
        let width = widths.iter().sum::<f32>() + gap * columns.saturating_sub(1) as f32;
        let height = rows.iter().sum::<f32>() + gap * rows.len().saturating_sub(1) as f32;
        Ok(Size::new(width, height)?)
    }
}
impl Component for Grid {
    fn reconcile(&mut self, next: Self) -> Option<crate::Invalidation> {
        let Self {
            gap,
            tracks,
            children: _,
        } = next;
        let changed = self.gap != gap || self.tracks != tracks;
        self.gap = gap;
        self.tracks = tracks;
        changed.then_some(crate::Invalidation::Layout)
    }
    fn measure_limits(&self, cx: &MeasureContext<'_>) -> Result<crate::SizeLimits, RuntimeError> {
        Ok(crate::SizeLimits::at_least(self.minimum(cx)?))
    }
    fn clips_children(&self) -> bool {
        false
    }
    fn keyboard_event(
        &mut self,
        _: &mut crate::EventContext,
        _: crate::KeyEvent,
        _: &crate::KeyboardState,
    ) -> Result<crate::InputOutcome, RuntimeError> {
        Ok(crate::InputOutcome::Ignored)
    }
    fn take_children(&mut self) -> Vec<Element> {
        std::mem::take(&mut self.children)
    }
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
    fn measure(
        &self,
        cx: &MeasureContext<'_>,
        available: Constraints,
    ) -> Result<Size, RuntimeError> {
        Ok(self.placements(cx, available)?.0)
    }
    fn arrange(&self, cx: &mut ArrangeContext<'_>, size: Size) -> Result<(), RuntimeError> {
        let (_, placements) = self.placements(cx.measurement(), Constraints::bounded(size))?;
        for (index, bounds) in placements.into_iter().enumerate() {
            cx.place_clipped(index, bounds, bounds)?;
        }
        Ok(())
    }
}
