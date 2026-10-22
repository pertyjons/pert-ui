//! Adaptive form layout without replacing or reparenting its controls.
use crate::{
    ArrangeContext, Component, ComponentDetails, Constraints, Element, EventContext, InputOutcome,
    InspectValue, KeyEvent, KeyboardState, MeasureContext, MetricRole, Rect, RuntimeError, Size,
    SizeLimits, UpdateContext,
};
use std::cell::Cell;

/// An adaptive, row-major form with equally wide columns and themed spacing.
///
/// Supply keyed [`crate::Field`] elements, or any custom components. At each
/// layout, the form fits as many columns as its own allocated width permits.
/// A column prefers the widest child's unconstrained preferred width, with a
/// floor of four [`MetricRole::ControlWidth`] units (256 logical pixels at
/// standard density). All columns share spare width. An unbounded width uses
/// one column. Empty forms occupy no space.
///
/// When less than one preferred column fits, a single column may shrink to the
/// larger of that themed floor and the largest child hard minimum. Below that,
/// content overflows: place the form inside [`crate::ScrollArea`] with
/// [`crate::ScrollAxes::Both`] or a scrolling view to keep every control reachable. Rows take
/// their tallest child's measured height, with top alignment and
/// [`MetricRole::ItemGap`] between rows and columns. Labels and messages remain
/// with their control; the form does not implement validation or submission.
///
/// Resizing, density and zoom use ordinary retained layout. Children remain in
/// description order with the same identities, focus and local editing state;
/// no subscriptions, timers or new component instances are created by reflow.
/// Hidden children consume no cells. Hard maximum sizes still constrain each
/// child inside its cell. Inspection exposes `columns` after arrangement.
///
/// ```
/// # use pert_ui_runtime as pert_ui;
/// use pert_ui::{Element, Field, Form, Runtime, ScrollArea, ScrollAxes, TextField};
/// let mut ui = Runtime::new();
/// let name = ui.signal(String::new());
/// let city = ui.signal(String::new());
/// ui.mount_tree(Element::new("viewport", ScrollArea::new(Element::new(
///     "profile",
///     Form::new([
///         Element::new("name", Field::new(Element::new("input", TextField::new(name)))
///             .label("Name").help("Shown to other people")),
///         Element::new("city", Field::new(Element::new("input", TextField::new(city)))
///             .label("City")),
///     ]),
/// )).axes(ScrollAxes::Both)))?;
/// # Ok::<(), pert_ui::RuntimeError>(())
/// ```
///
/// Re-describing the form reconciles its fields by key; the form has no other
/// settings, and its column count is recomputed from content and metrics.
#[derive(Default)]
#[must_use]
pub struct Form {
    children: Vec<Element>,
    columns: Cell<Option<usize>>,
}

impl Form {
    /// Collects fields in reading and keyboard traversal order.
    ///
    /// Keys must be nonempty and unique among these children, as checked when
    /// mounting. Adaptation uses content and theme metrics; no breakpoint
    /// configuration is required.
    pub fn new(children: impl IntoIterator<Item = Element>) -> Self {
        Self {
            children: children.into_iter().collect(),
            columns: Cell::new(None),
        }
    }

    fn minimum_width(cx: &MeasureContext<'_>) -> Result<f32, RuntimeError> {
        let mut width = cx.metric(MetricRole::ControlWidth) * 4.0;
        for index in 0..cx.len() {
            width = width.max(cx.limits(index)?.minimum().width());
        }
        Ok(width)
    }

    fn placements(
        cx: &MeasureContext<'_>,
        available: Constraints,
    ) -> Result<(Size, usize, Vec<Rect>), RuntimeError> {
        if cx.is_empty() {
            return Ok((Size::ZERO, 0, Vec::new()));
        }
        let minimum = Self::minimum_width(cx)?;
        let mut preferred = minimum;
        for index in 0..cx.len() {
            preferred = preferred.max(cx.measure(index, Constraints::default())?.width());
        }
        let gap = cx.metric(MetricRole::ItemGap);
        let width = available.width().unwrap_or(preferred).max(minimum);
        // Compute in f64 so two valid large extents cannot overflow the ratio.
        let columns = (((f64::from(width) + f64::from(gap))
            / (f64::from(preferred) + f64::from(gap)))
        .floor() as usize)
            .clamp(1, cx.len());
        let cell_width =
            ((f64::from(width) - f64::from(gap) * (columns - 1) as f64) / columns as f64) as f32;
        let mut placements = Vec::with_capacity(cx.len());
        let mut y = 0.0;
        for start in (0..cx.len()).step_by(columns) {
            let end = (start + columns).min(cx.len());
            let mut sizes = Vec::with_capacity(end - start);
            let mut height = 0.0_f32;
            for index in start..end {
                let size = cx.measure(index, Constraints::new(Some(cell_width), None)?)?;
                height = height.max(size.height());
                sizes.push(size);
            }
            for (column, size) in sizes.into_iter().enumerate() {
                placements.push(Rect::new(
                    column as f32 * (cell_width + gap),
                    y,
                    cell_width,
                    size.height(),
                )?);
            }
            y += height;
            if end < cx.len() {
                y += gap;
            }
        }
        Ok((Size::new(width, y)?, columns, placements))
    }
}

impl Component for Form {
    fn reconcile(&mut self, _: Self) -> Option<crate::Invalidation> {
        None
    }
    fn take_children(&mut self) -> Vec<Element> {
        std::mem::take(&mut self.children)
    }
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
    fn clips_children(&self) -> bool {
        false
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
            return Ok(SizeLimits::default());
        }
        Ok(SizeLimits::at_least(Size::new(
            Self::minimum_width(cx)?,
            0.0,
        )?))
    }
    fn measure(
        &self,
        cx: &MeasureContext<'_>,
        available: Constraints,
    ) -> Result<Size, RuntimeError> {
        Ok(Self::placements(cx, available)?.0)
    }
    fn arrange(&self, cx: &mut ArrangeContext<'_>, size: Size) -> Result<(), RuntimeError> {
        let (_, columns, placements) =
            Self::placements(cx.measurement(), Constraints::bounded(size))?;
        for (index, bounds) in placements.into_iter().enumerate() {
            cx.place_clipped(index, bounds, bounds)?;
        }
        self.columns.set(Some(columns));
        Ok(())
    }
    fn inspect(&self) -> ComponentDetails {
        let mut details = ComponentDetails::default();
        if let Some(columns) = self.columns.get() {
            details
                .properties
                .insert("columns".into(), InspectValue::Number(columns as f64));
        }
        details
    }
}
