//! Constrained local split preferences and independently lockable dividers.
use crate::layout::rect;
use crate::*;
use std::{cell::Cell, collections::BTreeSet};

/// Placement of the two children; nesting composes any number of regions.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SplitAxis {
    /// First child left, second child right.
    LeftRight,
    /// First child above, second child below.
    TopBottom,
}
impl SplitAxis {
    // Orders a value along the axis and one across it as (width, height). The
    // mapping is its own inverse, so passing (width, height) yields (along, across)
    // and every extent, coordinate, bound and rectangle below goes through it.
    fn order<T>(self, along: T, across: T) -> (T, T) {
        match self {
            Self::LeftRight => (along, across),
            Self::TopBottom => (across, along),
        }
    }
    // Extent along the axis, the one the divider divides.
    fn extent(self, size: Size) -> f32 {
        self.order(size.width(), size.height()).0
    }
    // Extent across the axis, which both children share.
    fn cross(self, size: Size) -> f32 {
        self.order(size.width(), size.height()).1
    }
    // Pointer coordinate along the axis.
    fn coordinate(self, point: Point) -> f32 {
        self.order(point.x(), point.y()).0
    }
    // Available space along the axis; None is unbounded.
    fn available(self, constraints: Constraints) -> Option<f32> {
        self.order(constraints.width(), constraints.height()).0
    }
    // Available space across the axis; None is unbounded.
    fn cross_available(self, constraints: Constraints) -> Option<f32> {
        self.order(constraints.width(), constraints.height()).1
    }
    // Hard upper bound along the axis; None is unbounded.
    fn maximum(self, limits: SizeLimits) -> Option<f32> {
        self.available(limits.maximum())
    }
    // Hard upper bound across the axis; None is unbounded.
    fn cross_maximum(self, limits: SizeLimits) -> Option<f32> {
        self.cross_available(limits.maximum())
    }
    // A size from extents along and across the axis.
    fn size(self, along: f32, across: f32) -> Result<Size, crate::GeometryError> {
        let (width, height) = self.order(along, across);
        Size::new(width, height)
    }
    // Available space from extents along and across the axis.
    fn constraints(
        self,
        along: Option<f32>,
        across: Option<f32>,
    ) -> Result<Constraints, crate::GeometryError> {
        let (width, height) = self.order(along, across);
        Constraints::new(width, height)
    }
    // A band covering `extent` from `start` along the axis and all of `across`.
    fn band(self, start: f32, extent: f32, across: f32) -> Result<Rect, RuntimeError> {
        let (x, y) = self.order(start, 0.0);
        let (width, height) = self.order(extent, across);
        rect(x, y, width, height)
    }
}
/// Validated preferred split sizing, independent of resize permission.
/// Allocation clamps to child min/max limits without changing this preference.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SplitSizing(Sizing);
#[derive(Clone, Copy, Debug, PartialEq)]
enum Sizing {
    Ratio(f32),
    First(Length),
    Second(Length),
    ContentFirst,
    ContentSecond,
}
#[derive(Clone, Copy)]
enum Resolved {
    Ratio(f32),
    First(Length),
    Second(Length),
}
impl Default for SplitSizing {
    fn default() -> Self {
        Self::EQUAL
    }
}
impl SplitSizing {
    /// Equal shares of space remaining after the themed divider (8 logical pixels at Standard) (default).
    pub const EQUAL: Self = Self(Sizing::Ratio(0.5));
    /// Sizes the first child to content until user resizing establishes a fixed extent.
    pub const CONTENT_FIRST: Self = Self(Sizing::ContentFirst);
    /// Sizes the second child to content until user resizing establishes a fixed extent.
    pub const CONTENT_SECOND: Self = Self(Sizing::ContentSecond);
    /// Prefers a finite first-child fraction in `0..=1`; rejects invalid ratios.
    ///
    /// # Errors
    /// [`LayoutError::InvalidRatio`] for a non-finite ratio or one outside `0..=1`.
    pub fn ratio(value: f32) -> Result<Self, LayoutError> {
        if !value.is_finite() || !(0.0..=1.0).contains(&value) {
            return Err(LayoutError::InvalidRatio);
        }
        Ok(Self(Sizing::Ratio(value)))
    }
    /// Keeps a preferred first-child extent while the second absorbs window resizing.
    #[must_use]
    pub const fn first(extent: Length) -> Self {
        Self(Sizing::First(extent))
    }
    /// Keeps a preferred second-child extent while the first absorbs window resizing.
    #[must_use]
    pub const fn second(extent: Length) -> Self {
        Self(Sizing::Second(extent))
    }
    /// The rule and its value, for an application that reads a preference
    /// [`SplitPane::on_resize`] reported.
    #[must_use]
    pub const fn rule(self) -> SplitRule {
        match self.0 {
            Sizing::Ratio(value) => SplitRule::Ratio(value),
            Sizing::First(extent) => SplitRule::First(extent),
            Sizing::Second(extent) => SplitRule::Second(extent),
            Sizing::ContentFirst => SplitRule::ContentFirst,
            Sizing::ContentSecond => SplitRule::ContentSecond,
        }
    }
}

/// What a [`SplitSizing`] prefers, read with [`SplitSizing::rule`]. Extents
/// are logical pixels along the split axis.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub enum SplitRule {
    /// The first child's share of the space after the divider, in `0..=1`
    /// ([`SplitSizing::ratio`], [`SplitSizing::EQUAL`]).
    Ratio(f32),
    /// The first child's extent ([`SplitSizing::first`]).
    First(Length),
    /// The second child's extent ([`SplitSizing::second`]).
    Second(Length),
    /// The first child's content extent ([`SplitSizing::CONTENT_FIRST`]).
    ContentFirst,
    /// The second child's content extent ([`SplitSizing::CONTENT_SECOND`]).
    ContentSecond,
}
#[derive(Clone, Copy)]
struct Geometry {
    first: f32,
    available: f32,
    divider: Rect,
    limits: [SizeLimits; 2],
    resolved: Resolved,
}
#[derive(Clone, Copy)]
struct Drag {
    original: SplitSizing,
    position: f32,
    first: f32,
}
/// Two retained children with an independently resizable themed divider (8 logical pixels at Standard).
///
/// Equal shares and enabled resizing are defaults. Pointer dragging commits on
/// primary release. Arrow keys resize by 8 logical pixels (Shift: 32), with native
/// repeats; release commits once all owned arrow keys are up. Control, Alt and
/// Super chords are left to command bindings. Escape, context loss
/// or locking cancels a tentative resize and restores its original preference.
/// One input source owns resizing at a time: arrows are ignored during a drag,
/// and pointer admission is ignored while resize keys are held.
/// Hard child min/max limits apply; infeasible minimums overflow toward an enclosing
/// scroll viewport. If both maxima leave spare space, it stays at the trailing edge.
/// This is UI-local workspace state, never a host parameter edit or undo operation.
/// The centered grip keeps the same slim dimensions while hovered/dragged and
/// uses the button theme's corner radius. It adds equal whole physical-pixel
/// margins on both sides of the boundary line (one per side at the default 1×
/// scale), clamped to the available divider width. The full divider remains draggable.
/// Inspection reports the split as a [`SemanticRole::Splitter`] named by
/// [`StringRole::SplitterName`](crate::StringRole::SplitterName) from the
/// installed [`UiStrings`](crate::UiStrings), "Resize panes" in English.
///
/// Describing a mounted split again (through `Runtime::reconcile_children`)
/// applies the new axis, lock or resize permission and sizing rule. The current
/// preference is user workspace state: a position the user dragged or restored
/// survives a re-description whose [`Self::sizing`] rule is unchanged, and the
/// described rule replaces it only when that rule differs from the previously
/// described one. A new axis, a new sizing rule or locking cancels a tentative
/// resize first, restoring its original preference; the rest of the pointer
/// press is then ignored until release. [`Self::on_resize`] reports committed
/// preferences to the application.
#[must_use]
pub struct SplitPane {
    children: Vec<Element>,
    axis: SplitAxis,
    sizing: Cell<SplitSizing>,
    /// The rule the description selected, which decides whether a new
    /// description replaces the user's current preference.
    described: SplitSizing,
    /// Resize permission; every builder call combines into it.
    permission: Permission,
    /// The permission's value at the latest update or description.
    enabled: bool,
    focused: bool,
    hovered: bool,
    eligible: bool,
    geometry: Cell<Option<Geometry>>,
    drag: Option<Drag>,
    key_origin: Option<SplitSizing>,
    keys: BTreeSet<KeyCode>,
    /// The document extent a workspace restore produced, with the preference it
    /// resolved to. The preference stores `f32`; capture returns this exact
    /// `f64` while the preference is unchanged, so restore then capture
    /// reproduces the document instead of a widened `f32`.
    restored: Cell<Option<(SplitSizing, f64)>>,
    /// The divider's accessibility name, from the installed strings table.
    name: String,
    on_resize: Option<ResizeHandler>,
    /// The preference last reported through `on_resize` and a change waiting.
    report: crate::geometry_report::GeometryReport<SplitSizing>,
}

type ResizeHandler =
    Box<dyn FnMut(&mut EventContext, SplitSizing, ChangeOrigin) -> Result<(), RuntimeError>>;
impl SplitPane {
    /// Creates a split with distinct sibling-keyed children. Nest further splits in either child.
    pub fn new(axis: SplitAxis, first: Element, second: Element) -> Self {
        Self {
            children: vec![first, second],
            axis,
            sizing: Cell::new(SplitSizing::EQUAL),
            described: SplitSizing::EQUAL,
            permission: Permission::allowed(),
            enabled: true,
            name: crate::UiStrings::english()
                .get(crate::StringRole::SplitterName)
                .to_owned(),
            focused: false,
            hovered: false,
            eligible: false,
            geometry: Cell::new(None),
            drag: None,
            key_origin: None,
            keys: BTreeSet::new(),
            restored: Cell::new(None),
            on_resize: None,
            report: crate::geometry_report::GeometryReport::new(SplitSizing::EQUAL),
        }
    }
    /// Selects an initial preferred rule; window constraint clamping never overwrites it.
    ///
    /// On a mounted split, a re-description with a different rule replaces the
    /// user's current preference; one with the same rule keeps it.
    pub fn sizing(mut self, sizing: SplitSizing) -> Self {
        self.sizing.set(sizing);
        self.described = sizing;
        self.report = crate::geometry_report::GeometryReport::new(sizing);
        self
    }
    /// Reports the committed preference after it changed, with what changed
    /// it: read the rule and its extent with [`SplitSizing::rule`], and pass it
    /// back to [`Self::sizing`] to describe the split that way.
    ///
    /// The callback runs on the UI thread from an event phase once the
    /// flush's updates and layout have settled
    /// ([`Component::deliver_notifications`]), at most once per flush, with
    /// the last change of the flush. The origin is
    ///
    /// - [`ChangeOrigin::User`] when a pointer drag ends at its release or
    ///   held arrow keys are all released;
    /// - [`ChangeOrigin::Restore`] for [`Runtime::restore_workspace`];
    /// - [`ChangeOrigin::Redescription`] when a description with another
    ///   [`Self::sizing`] rule replaced the preference.
    ///
    /// A resize in progress is a preview and is not reported; Escape, context
    /// loss and revoking [`Self::resizable`] restore the preference from
    /// before it and report nothing. A preference that ends where it started
    /// reports nothing. The callback may write bindings; its error propagates
    /// from the flush after the other pending deliveries, the preference stays
    /// and nothing is replayed. Describing the split again replaces the
    /// callback, including removing it; a description without one discards a
    /// change not yet delivered.
    ///
    /// ```
    /// use pert_ui_runtime::*;
    /// use std::{cell::RefCell, rc::Rc, time::Duration};
    ///
    /// let mut ui = Runtime::new();
    /// ui.set_viewport(Viewport::new(400.0, 200.0, 1.0)?)?;
    /// let log = Rc::new(RefCell::new(Vec::new()));
    /// let seen = log.clone();
    /// let split = ui.mount_tree(Element::new(
    ///     "split",
    ///     SplitPane::new(
    ///         SplitAxis::LeftRight,
    ///         Element::new("a", Label::new("A")),
    ///         Element::new("b", Label::new("B")),
    ///     )
    ///     .sizing(SplitSizing::first(Length::new(150.0)?))
    ///     .on_resize(move |_, sizing, origin| {
    ///         seen.borrow_mut().push((sizing.rule(), origin));
    ///         Ok(())
    ///     }),
    /// ))?;
    /// ui.flush(Duration::ZERO)?;
    /// ui.focus(&split)?;
    /// // Each held arrow key moves the divider; the release commits once.
    /// ui.key_input(KeyCode::ArrowRight, KeyAction::Press)?;
    /// ui.key_input(KeyCode::ArrowRight, KeyAction::Release)?;
    /// ui.flush(Duration::ZERO)?;
    /// assert_eq!(
    ///     *log.borrow(),
    ///     [(SplitRule::First(Length::new(158.0)?), ChangeOrigin::User)]
    /// );
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn on_resize(
        mut self,
        on_resize: impl FnMut(&mut EventContext, SplitSizing, ChangeOrigin) -> Result<(), RuntimeError>
        + 'static,
    ) -> Self {
        self.on_resize = Some(Box::new(on_resize));
        self
    }
    /// The committed preference, ignoring a resize in progress.
    fn committed(&self) -> SplitSizing {
        self.drag
            .as_ref()
            .map(|drag| drag.original)
            .or(self.key_origin)
            .unwrap_or(self.sizing.get())
    }
    /// Records a change for `on_resize`, when there is one.
    fn changed(&self, origin: ChangeOrigin) {
        if self.on_resize.is_some() {
            self.report.record(origin);
        }
    }
    /// Restricts whether the user may resize this divider. Each call combines
    /// with the permission the description already holds and the stricter
    /// wins ([`Permission::and`]), so `locked()` cannot be lifted by a later
    /// bound permission. The default allows resizing.
    ///
    /// A bound permission is read at each update. When it turns false, a
    /// tentative resize in progress, by a held pointer drag or held arrow keys,
    /// is cancelled and its original preference restored, and the rest of that
    /// press is ignored; when it turns true again, the next press can resize.
    /// Child identity and sizing are retained either way.
    pub fn resizable(mut self, permission: impl Into<Permission>) -> Self {
        self.permission = self.permission.and(permission);
        if let Some(fixed) = self.permission.fixed() {
            self.enabled = fixed;
        }
        self
    }
    /// Permanently locks this divider: `resizable(false)`. Child identity,
    /// sizing and input are retained.
    pub fn locked(self) -> Self {
        self.resizable(false)
    }
    // The preference as the document carries it: a rule name and, for the rules
    // that have one, an extent. The resolved allocation is deliberately absent;
    // it is a measurement, and the next layout produces it again.
    // A preference a restore produced reports the restored document value.
    fn sizing_values(&self) -> (&'static str, Option<f64>) {
        let committed = self.committed();
        let exact = |value: f32| match self.restored.get() {
            Some((restored, document)) if restored == committed => document,
            _ => f64::from(value),
        };
        match committed.0 {
            Sizing::Ratio(value) => ("ratio", Some(exact(value))),
            Sizing::First(extent) => ("first", Some(exact(extent.get()))),
            Sizing::Second(extent) => ("second", Some(exact(extent.get()))),
            Sizing::ContentFirst => ("content-first", None),
            Sizing::ContentSecond => ("content-second", None),
        }
    }
    fn resolve(
        &self,
        cx: &MeasureContext<'_>,
        available: Constraints,
    ) -> Result<Resolved, RuntimeError> {
        let index = match self.sizing.get().0 {
            Sizing::Ratio(value) => return Ok(Resolved::Ratio(value)),
            Sizing::First(value) => return Ok(Resolved::First(value)),
            Sizing::Second(value) => return Ok(Resolved::Second(value)),
            Sizing::ContentFirst => 0,
            Sizing::ContentSecond => 1,
        };
        let size = cx.measure(
            index,
            self.axis
                .constraints(None, self.axis.cross_available(available))?,
        )?;
        let extent = Length::new(self.axis.extent(size))?;
        Ok(if index == 0 {
            Resolved::First(extent)
        } else {
            Resolved::Second(extent)
        })
    }
    fn current(&self, geometry: Geometry) -> Resolved {
        match self.sizing.get().0 {
            Sizing::Ratio(v) => Resolved::Ratio(v),
            Sizing::First(v) => Resolved::First(v),
            Sizing::Second(v) => Resolved::Second(v),
            Sizing::ContentFirst | Sizing::ContentSecond => geometry.resolved,
        }
    }
    fn allocate(&self, available: f32, limits: [SizeLimits; 2], sizing: Resolved) -> (f32, f32) {
        let minima = limits.map(|v| f64::from(self.axis.extent(v.minimum())));
        let maxima = limits.map(|v| f64::from(self.axis.maximum(v).unwrap_or(f32::MAX)));
        let available = f64::from(available);
        if minima[0] + minima[1] > available {
            return (minima[0] as f32, minima[1] as f32);
        }
        let total = available.min(maxima[0] + maxima[1]);
        let preferred = match sizing {
            Resolved::Ratio(value) => available * f64::from(value),
            Resolved::First(value) => f64::from(value.get()),
            Resolved::Second(value) => available - f64::from(value.get()),
        };
        let first = preferred.clamp(
            minima[0].max(total - maxima[1]),
            maxima[0].min(total - minima[1]),
        );
        (first as f32, (total - first) as f32)
    }
    fn move_to(&self, cx: &mut EventContext, first: f32) -> Result<(), RuntimeError> {
        let Some(geometry) = self.geometry.get() else {
            return Ok(());
        };
        let first = first.clamp(0.0, geometry.available);
        let candidate = Resolved::First(Length::new(first)?);
        let (first, _) = self.allocate(geometry.available, geometry.limits, candidate);
        if self
            .allocate(geometry.available, geometry.limits, self.current(geometry))
            .0
            == first
        {
            return Ok(());
        }
        let next = match self.sizing.get().0 {
            Sizing::Ratio(_) if geometry.available > 0.0 => {
                SplitSizing::ratio((first / geometry.available).clamp(0.0, 1.0))?
            }
            Sizing::Ratio(_) => self.sizing.get(),
            Sizing::First(_) | Sizing::ContentFirst => SplitSizing::first(Length::new(first)?),
            Sizing::Second(_) | Sizing::ContentSecond => {
                SplitSizing::second(Length::new((geometry.available - first).max(0.0))?)
            }
        };
        if self.sizing.replace(next) != next {
            cx.invalidate(Invalidation::Layout);
        }
        Ok(())
    }
    fn cancel(&mut self) {
        if let Some(drag) = self.drag.take() {
            self.sizing.set(drag.original);
        }
        if let Some(original) = self.key_origin.take() {
            self.sizing.set(original);
        }
        self.keys.clear();
    }
}
impl Component for SplitPane {
    fn reconcile(&mut self, next: Self) -> Option<Invalidation> {
        let Self {
            children: _,
            axis,
            sizing,
            described,
            permission,
            enabled,
            focused: _,
            hovered: _,
            eligible: _,
            geometry: _,
            drag: _,
            key_origin: _,
            keys: _,
            restored: _,
            // The name follows the installed table at the next update.
            name: _,
            on_resize,
            report: _,
        } = next;
        // A callback that starts, or a description without one, takes the
        // committed preference as known.
        if on_resize.is_none() || self.on_resize.is_none() {
            self.report.discard(self.committed());
        }
        self.on_resize = on_resize;
        let axis_changed = self.axis != axis;
        let sizing_changed = self.described != described;
        // A bound permission is read at the next update, which cancels there.
        let permission_changed = !self.permission.same_as(&permission);
        let locked = permission.fixed() == Some(false);
        if axis_changed || sizing_changed || (permission_changed && locked) {
            self.cancel();
        }
        self.axis = axis;
        if sizing_changed {
            self.sizing.set(sizing.get());
            self.described = described;
            self.changed(ChangeOrigin::Redescription);
        }
        if permission_changed {
            if permission.fixed().is_some() {
                self.enabled = enabled;
            }
            self.permission = permission;
        }
        if axis_changed || sizing_changed || (permission_changed && locked) {
            Some(Invalidation::Layout)
        } else if permission_changed {
            Some(Invalidation::Paint)
        } else {
            None
        }
    }
    fn workspace_state(&self) -> Result<Option<crate::ViewState>, crate::WorkspaceError> {
        let (rule, extent) = self.sizing_values();
        let state =
            crate::ViewState::new("split")?.with("rule", crate::WorkspaceValue::text(rule)?)?;
        Ok(Some(match extent {
            Some(extent) => state.with("extent", crate::WorkspaceValue::number(extent)?)?,
            None => state,
        }))
    }
    fn restore_workspace_state(
        &mut self,
        state: &crate::ViewState,
    ) -> Result<Invalidation, crate::RestoreRejection> {
        use crate::RestoreRejection;
        state.expect_only(&["rule", "extent"])?;
        if self.drag.is_some() || !self.keys.is_empty() {
            return Err(RestoreRejection::Busy);
        }
        let document = |state: &crate::ViewState| state.number("extent");
        let extent = |state: &crate::ViewState| -> Result<f32, RestoreRejection> {
            let number = document(state)?;
            let value = number as f32;
            if number >= 0.0 && value.is_finite() {
                Ok(value)
            } else {
                Err(RestoreRejection::OutOfRange("extent".to_owned()))
            }
        };
        let length = |value: f32| {
            Length::new(value).map_err(|_| RestoreRejection::OutOfRange("extent".to_owned()))
        };
        let sizing = match state.text("rule")? {
            "ratio" => {
                if !(0.0..=1.0).contains(&document(state)?) {
                    return Err(RestoreRejection::OutOfRange("extent".to_owned()));
                }
                SplitSizing::ratio(extent(state)?)
                    .map_err(|_| RestoreRejection::OutOfRange("extent".to_owned()))?
            }
            "first" => SplitSizing::first(length(extent(state)?)?),
            "second" => SplitSizing::second(length(extent(state)?)?),
            "content-first" => {
                state.expect_only(&["rule"])?;
                SplitSizing::CONTENT_FIRST
            }
            "content-second" => {
                state.expect_only(&["rule"])?;
                SplitSizing::CONTENT_SECOND
            }
            _ => return Err(RestoreRejection::Unresolved("rule".to_owned())),
        };
        self.restored.set(match sizing.0 {
            Sizing::Ratio(_) | Sizing::First(_) | Sizing::Second(_) => {
                Some((sizing, document(state)?))
            }
            Sizing::ContentFirst | Sizing::ContentSecond => None,
        });
        self.sizing.set(sizing);
        self.changed(ChangeOrigin::Restore);
        Ok(Invalidation::Layout)
    }
    fn hover_changed(&mut self, cx: &mut EventContext, hovered: bool) {
        if !hovered && self.hovered {
            self.hovered = false;
            cx.invalidate(Invalidation::Paint);
        }
    }
    fn cursor(&self, point: Point) -> Option<CursorIcon> {
        (self.enabled
            && self
                .geometry
                .get()
                .is_some_and(|geometry| self.drag.is_some() || geometry.divider.contains(point)))
        .then_some(if self.axis == SplitAxis::LeftRight {
            CursorIcon::ResizeHorizontal
        } else {
            CursorIcon::ResizeVertical
        })
    }

    fn clips_children(&self) -> bool {
        false
    }
    fn take_children(&mut self) -> Vec<Element> {
        std::mem::take(&mut self.children)
    }
    fn focusable(&self) -> bool {
        self.enabled && self.eligible && self.geometry.get().is_some()
    }
    fn deliver_notifications(&mut self, cx: &mut EventContext) -> Result<(), RuntimeError> {
        if let Some((sizing, origin)) = self.report.take(self.committed())
            && let Some(on_resize) = &mut self.on_resize
        {
            on_resize(cx, sizing, origin)?;
        }
        Ok(())
    }
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        self.report.set_notifier(|| cx.notifier());
        cx.strings()?
            .get(crate::StringRole::SplitterName)
            .clone_into(&mut self.name);
        self.enabled = self.permission.read(cx)?;
        self.eligible = cx.children().iter().filter(|child| child.visible()).count() == 2;
        if !self.enabled || !self.eligible {
            self.cancel();
            cx.cancel_input();
        }
        self.focused = cx.focused() && self.enabled;
        Ok(())
    }
    fn measure_limits(&self, cx: &MeasureContext<'_>) -> Result<SizeLimits, RuntimeError> {
        let divider = cx.metric(MetricRole::ResizeWidth);
        if cx.len() > 2 {
            return Err(LayoutError::InvalidChild.into());
        }
        if cx.is_empty() {
            return Ok(SizeLimits::default());
        }
        if cx.len() == 1 {
            return cx.limits(0);
        }
        let a = cx.limits(0)?;
        let b = cx.limits(1)?;
        let min = self.axis.size(
            self.axis.extent(a.minimum()) + self.axis.extent(b.minimum()) + divider,
            self.axis
                .cross(a.minimum())
                .max(self.axis.cross(b.minimum())),
        )?;
        let sum = |a: Option<f32>, b: Option<f32>| {
            a.zip(b).map(|(a, b)| {
                (f64::from(a) + f64::from(b) + f64::from(divider)).min(f64::from(f32::MAX)) as f32
            })
        };
        let max = |a: Option<f32>, b: Option<f32>| a.zip(b).map(|(a, b)| a.max(b));
        let maximum = self.axis.constraints(
            sum(self.axis.maximum(a), self.axis.maximum(b)),
            max(self.axis.cross_maximum(a), self.axis.cross_maximum(b)),
        )?;
        Ok(SizeLimits::new(min, maximum)?)
    }
    fn measure(
        &self,
        cx: &MeasureContext<'_>,
        available: Constraints,
    ) -> Result<Size, RuntimeError> {
        let divider = cx.metric(MetricRole::ResizeWidth);
        if cx.len() > 2 {
            return Err(LayoutError::InvalidChild.into());
        }
        if cx.is_empty() {
            return Ok(Size::ZERO);
        }
        if cx.len() == 1 {
            return cx.measure(0, available);
        }
        let along = self.axis.available(available);
        let limits = [cx.limits(0)?, cx.limits(1)?];
        let resolved = self.resolve(cx, available)?;
        let portions = along.map(|v| self.allocate((v - divider).max(0.0), limits, resolved));
        let constraints = |portion| {
            self.axis
                .constraints(portion, self.axis.cross_available(available))
        };
        let first = cx.measure(0, constraints(portions.map(|v| v.0))?)?;
        let second = cx.measure(1, constraints(portions.map(|v| v.1))?)?;
        Ok(self.axis.size(
            self.axis.extent(first) + self.axis.extent(second) + divider,
            self.axis.cross(first).max(self.axis.cross(second)),
        )?)
    }
    fn arrange(&self, cx: &mut ArrangeContext<'_>, size: Size) -> Result<(), RuntimeError> {
        let divider = cx.measurement().metric(MetricRole::ResizeWidth);
        if cx.len() > 2 {
            return Err(LayoutError::InvalidChild.into());
        }
        if cx.len() != 2 {
            self.geometry.set(None);
            if cx.len() == 1 {
                cx.place(0, Rect::from_size(size))?;
            }
            return Ok(());
        }
        let available = (self.axis.extent(size) - divider).max(0.0);
        let limits = [cx.measurement().limits(0)?, cx.measurement().limits(1)?];
        let resolved = self.resolve(cx.measurement(), Constraints::bounded(size))?;
        let (first, second) = self.allocate(available, limits, resolved);
        let across = self.axis.cross(size);
        for (index, band) in [
            self.axis.band(0.0, first, across)?,
            self.axis.band(first + divider, second, across)?,
        ]
        .into_iter()
        .enumerate()
        {
            cx.place_clipped(index, band, band)?;
        }
        self.geometry.set(Some(Geometry {
            first,
            available,
            divider: self.axis.band(first, divider, across)?,
            limits,
            resolved,
        }));
        Ok(())
    }
    fn pointer_input(
        &mut self,
        cx: &mut EventContext,
        event: PointerEvent,
    ) -> Result<InputOutcome, RuntimeError> {
        let hovered = !event.is_outside()
            && self.enabled
            && self
                .geometry
                .get()
                .zip(cx.to_local(event.position()))
                .is_some_and(|(geometry, point)| geometry.divider.contains(point));
        if self.hovered != hovered {
            self.hovered = hovered;
            cx.invalidate(Invalidation::Paint);
        }
        if !self.enabled {
            cx.release_pointer();
            return Ok(InputOutcome::Ignored);
        }
        match event.kind() {
            PointerKind::Press(PointerButton::Primary) => {
                if self.key_origin.is_some() {
                    return Ok(InputOutcome::Handled);
                }
                let (Some(geometry), Some(point)) =
                    (self.geometry.get(), cx.to_local(event.position()))
                else {
                    return Ok(InputOutcome::Ignored);
                };
                if !geometry.divider.contains(point) {
                    return Ok(InputOutcome::Ignored);
                }
                self.cancel();
                self.drag = Some(Drag {
                    original: self.sizing.get(),
                    position: self.axis.coordinate(point),
                    first: geometry.first,
                });
                cx.capture_pointer()?;
                cx.invalidate(Invalidation::Paint);
            }
            PointerKind::Move if self.drag.is_some() => {
                // Local positions, so the divider follows the pointer under a
                // view transform as well.
                if let (Some(drag), Some(point)) = (self.drag, cx.to_local(event.position())) {
                    self.move_to(cx, drag.first + self.axis.coordinate(point) - drag.position)?;
                }
            }
            PointerKind::Release(PointerButton::Primary) if self.drag.is_some() => {
                self.drag = None;
                cx.release_pointer();
                cx.invalidate(Invalidation::Paint);
                self.changed(ChangeOrigin::User);
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
        // Escape belongs to this split only while it owns a tentative resize;
        // otherwise it continues to ancestors and window/application commands.
        if control == KeyCode::Escape {
            if self.drag.is_none() && self.key_origin.is_none() {
                return Ok(InputOutcome::Ignored);
            }
            self.cancel();
            cx.release_pointer();
            cx.invalidate(Invalidation::Layout);
            return Ok(InputOutcome::Handled);
        }
        if !self.enabled || !cx.focused() {
            return Ok(InputOutcome::Ignored);
        }
        if self.drag.is_some() {
            return Ok(InputOutcome::Handled);
        }
        let direction = match (self.axis, control) {
            (SplitAxis::LeftRight, KeyCode::ArrowLeft)
            | (SplitAxis::TopBottom, KeyCode::ArrowUp) => -1.0,
            (SplitAxis::LeftRight, KeyCode::ArrowRight)
            | (SplitAxis::TopBottom, KeyCode::ArrowDown) => 1.0,
            _ => return Ok(InputOutcome::Ignored),
        };
        // A new Control, Alt or Super chord is left to command bindings; keys
        // this split already holds keep resizing.
        if event.transition == KeyTransition::Pressed
            && !self.keys.contains(&event.code)
            && !crate::activation::steps_with(keyboard.modifiers())
        {
            return Ok(InputOutcome::Ignored);
        }
        match event.transition {
            KeyTransition::Pressed => {
                if self.key_origin.is_none() {
                    self.key_origin = Some(self.sizing.get());
                }
                self.keys.insert(event.code);
            }
            KeyTransition::Repeated if !self.keys.contains(&event.code) => {
                return Ok(InputOutcome::Handled);
            }
            KeyTransition::Released | KeyTransition::UnmatchedRelease => {
                self.keys.remove(&event.code);
                if self.keys.is_empty() && self.key_origin.take().is_some() {
                    self.changed(ChangeOrigin::User);
                }
                return Ok(InputOutcome::Handled);
            }
            _ => {}
        }
        if let Some(geometry) = self.geometry.get() {
            self.move_to(
                cx,
                self.allocate(geometry.available, geometry.limits, self.current(geometry))
                    .0
                    + direction
                        * if keyboard.modifiers().shift {
                            32.0
                        } else {
                            8.0
                        },
            )?;
        }
        Ok(InputOutcome::Handled)
    }
    fn cancel_input(&mut self, cx: &mut EventContext) {
        self.cancel();
        self.focused = false;
        self.hovered = false;
        cx.release_pointer();
        cx.invalidate(Invalidation::Layout);
    }
    fn paint_foreground(&self, painter: &mut Painter) -> Result<(), DrawError> {
        if let Some(geometry) = self.geometry.get() {
            let [x, y, w, h] = geometry.divider.values();
            let thickness = painter.metric(MetricRole::BorderWidth).min(w.min(h));
            let color = painter.color(if self.key_origin.is_some() || self.focused {
                ColorRole::FocusRing
            } else if self.drag.is_some() {
                ColorRole::Primary
            } else if self.enabled && self.hovered {
                ColorRole::ResizeHandleHover
            } else if self.enabled {
                ColorRole::ResizeHandle
            } else {
                ColorRole::Divider
            });
            // One shared line owns the boundary; child panels need no duplicate edge.
            let line = if self.axis == SplitAxis::LeftRight {
                Rect::new(x + (w - thickness) / 2.0, y, thickness, h)?
            } else {
                Rect::new(x, y + (h - thickness) / 2.0, w, thickness)?
            };
            painter.fill_rect(line, color);
            if self.enabled && w > 0.0 && h > 0.0 {
                let across = if self.axis == SplitAxis::LeftRight {
                    w
                } else {
                    h
                };
                let along = if self.axis == SplitAxis::LeftRight {
                    h
                } else {
                    w
                };
                // An even-width grip over an odd-width line can rasterize with
                // its only extra pixel on one side. Add the same integral pixel
                // margin to both line edges, preserving their common pixel phase.
                let scale = painter.viewport().scale();
                let margin_pixels = if thickness > 0.0 {
                    (f64::from(thickness) * scale).round().max(1.0).min(
                        (f64::from(across - thickness) * scale / 2.0)
                            .floor()
                            .max(0.0),
                    )
                } else {
                    0.0
                };
                let thickness = thickness + (2.0 * margin_pixels / scale) as f32;
                let length = (painter.metric(MetricRole::IconSize) * 1.5).min(along);
                let grip = if self.axis == SplitAxis::LeftRight {
                    Rect::new(
                        x + (w - thickness) / 2.0,
                        y + (h - length) / 2.0,
                        thickness,
                        length,
                    )?
                } else {
                    Rect::new(
                        x + (w - length) / 2.0,
                        y + (h - thickness) / 2.0,
                        length,
                        thickness,
                    )?
                };
                painter.fill_shape(
                    grip,
                    Shape::Rounded(Length::new(painter.metric(MetricRole::CornerRadius))?),
                    color,
                )?;
            }
        }
        Ok(())
    }
    fn inspect(&self) -> ComponentDetails {
        let mut details = ComponentDetails::default();
        details
            .properties
            .insert("resizable".into(), InspectValue::Bool(self.enabled));
        // The committed preference as the workspace document carries it: the
        // rule's name and, for the rules that have one, its extent or ratio.
        let (rule, extent) = self.sizing_values();
        details
            .properties
            .insert("rule".into(), InspectValue::Text(rule.into()));
        if let Some(extent) = extent {
            details
                .properties
                .insert("extent".into(), InspectValue::Number(extent));
        }
        details.semantics = Some(
            Semantics::new(SemanticRole::Splitter)
                .name(self.name.clone())
                .disabled(!self.enabled)
                .enabled_actions([SemanticAction::Resize]),
        );
        details
    }
}
