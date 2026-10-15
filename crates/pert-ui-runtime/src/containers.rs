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
}
impl TryFrom<f32> for Length {
    type Error = crate::GeometryError;
    /// Validates like [`Length::new`].
    fn try_from(value: f32) -> Result<Self, Self::Error> {
        Self::new(value)
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
/// roles, padding, preferred size, limits and cursor; its child reconciles by
/// key.
#[must_use]
#[derive(Default)]
pub struct Container {
    children: Vec<Element>,
    background: Option<Color>,
    background_role: Option<crate::ColorRole>,
    border_role: Option<crate::ColorRole>,
    painted_background: std::cell::Cell<Option<Color>>,
    painted_border: std::cell::Cell<Option<(f32, Color)>>,
    padding: Option<Spacing>,
    resolved_padding: std::cell::Cell<Option<f32>>,
    preferred: Option<Size>,
    limits: crate::SizeLimits,
    cursor: Option<crate::CursorIcon>,
}
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
    pub fn background(mut self, color: Color) -> Self {
        self.background = Some(color);
        self.background_role = None;
        self
    }
    /// Fills with a theme role, replacing a literal background override.
    /// Changes to this role automatically repaint the container; default is transparent.
    pub fn background_role(mut self, role: crate::ColorRole) -> Self {
        self.background_role = Some(role);
        self.background = None;
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
    /// pixels, or a [`MetricRole`] resolved from the theme at each layout, as
    /// [`Spacing`] describes. The default is none; no margin is implicit, and
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
        let paint = self.background != next.background
            || self.background_role != next.background_role
            || self.border_role != next.border_role
            || self.cursor != next.cursor;
        self.background = next.background;
        self.background_role = next.background_role;
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
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
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
        let color = self
            .background
            .or_else(|| self.background_role.map(|role| painter.color(role)));
        self.painted_background.set(color);
        if let (Some(color), Some(size)) = (color, painter.size()) {
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
        if let (Some(role), Some(size)) = (self.border_role, painter.size()) {
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
    gap: f32,
) -> Result<(Size, Vec<Rect>), RuntimeError> {
    // Ordinary children keep the offered height, including bounded viewports.
    // Separators first contribute their natural height, then fill the resolved
    // row. Only opted-in children need a second measurement constraint.
    let mut row_sizes = Vec::new();
    if matches!(flow, Flow::Row) {
        let mut height = 0.0f32;
        for index in 0..cx.len() {
            let offered = Constraints::new(
                None,
                if cx.fills_row_height(index)? {
                    None
                } else {
                    available.height()
                },
            )?;
            let size = cx.measure(index, offered)?;
            height = height.max(size.height());
            row_sizes.push(size);
        }
        let height = available.height().map_or(height, |limit| height.min(limit));
        for (index, size) in row_sizes.iter_mut().enumerate() {
            if cx.fills_row_height(index)? {
                *size = cx.measure(index, Constraints::new(None, Some(height))?)?;
            }
        }
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
    if matches!(flow, Flow::Row | Flow::Wrap) {
        center_line(&mut out[row_start..], row_height)?;
    }
    Ok((Size::new(width, height)?, out))
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
        }
        impl $name {
            /// Collects ordered children; sibling keys must be unique at mounting.
            pub fn new(children: impl IntoIterator<Item = Element>) -> Self {
                Self {
                    children: children.into_iter().collect(),
                    gap: None,
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
        }
        impl Component for $name {
            fn reconcile(&mut self, next: Self) -> Option<crate::Invalidation> {
                let changed = self.gap != next.gap;
                self.gap = next.gap;
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
                Ok(flow_rects(cx, available, Flow::$flow, self.resolved_gap(cx))?.0)
            }
            fn arrange(&self, cx: &mut ArrangeContext<'_>, size: Size) -> Result<(), RuntimeError> {
                let (_, placements) = flow_rects(
                    cx.measurement(),
                    Constraints::bounded(size),
                    Flow::$flow,
                    self.resolved_gap(cx.measurement()),
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
    "One horizontal row with the theme item gap (8 logical pixels at Standard). Children receive the offered height; children opting into Component::fills_row_height contribute their natural height first, then fill the resolved content height up to the offered height. Children are vertically centered against the tallest resolved child, aligning labels with taller controls. Unused height below the row remains empty; overflow never implicitly wraps."
);
flow_component!(
    Column,
    Column,
    "One vertical column with the theme item gap (8 logical pixels at Standard) and start alignment; allocated widths fit available space while hard minimums and reported overflow are preserved."
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
/// use pert_ui_runtime::{Element, Grid, Label, Length, Runtime, Track, Viewport};
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
