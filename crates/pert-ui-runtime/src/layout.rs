//! Validated geometry and public retained layout callbacks.
use crate::{Component, Rect, Runtime, RuntimeError};

/// Invalid layout configuration or a violated custom-container contract.
///
/// Invalid sizes, points and rectangles are [`GeometryError`]s; this error is
/// about how a container distributes space and treats its children.
///
/// A key variant carries the key it rejects, so the error is `Clone` but not
/// `Copy`; the argument variants stay unit, because the caller holds the
/// weight or ratio it passed.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum LayoutError {
    /// A custom layout addressed a missing child or placed a child twice, or a
    /// line index is outside a [`VirtualAxis`](crate::VirtualAxis).
    #[error("invalid child placement")]
    InvalidChild,
    /// A described item's key is empty or whitespace-only. Keys identify
    /// children for reconciliation and persistence and must carry text.
    #[error("item key is blank")]
    BlankKey,
    /// A described item's human-readable label, such as a tab title, section
    /// title, property label or breadcrumb segment, is empty or whitespace-only.
    #[error("item label is blank")]
    BlankLabel,
    /// Two described items of one container share the key held here, so
    /// neither could be addressed or retained unambiguously.
    #[error("two items share the key `{0}`")]
    DuplicateKey(String),
    /// A described item uses the key held here, which the container reserves
    /// for a part it generates itself, such as an overflow menu.
    #[error("item key `{0}` is reserved")]
    ReservedKey(String),
    /// An operation addressed the key held here, which the container does not
    /// have, such as a tool window or column that was never described or has
    /// been removed.
    #[error("no item has the key `{0}`")]
    UnknownKey(String),
    /// An operation addressed a tool-window edge the layout does not offer.
    #[error("edge is not available in this layout")]
    UnavailableEdge,
    /// A table was described without columns.
    #[error("a table needs at least one column")]
    NoColumns,
    /// A weight that shares out space, such as a weighted grid track or a flexible
    /// table column, is not finite and strictly positive.
    #[error("layout weight must be finite and positive")]
    InvalidWeight,
    /// A split ratio is not finite or lies outside `0..=1`.
    #[error("split ratio must be within 0..=1")]
    InvalidRatio,
    /// A grid was given no column tracks.
    #[error("grid needs at least one column track")]
    NoTracks,
    /// A virtualized list was given zero lanes.
    #[error("a virtual axis needs at least one lane")]
    NoLanes,
}

/// Invalid geometry: a coordinate, extent, bound, scale or transform.
///
/// This is the one error of the validated geometry constructors: [`Size`],
/// [`Constraints`], [`SizeLimits`], [`Point`](crate::Point),
/// [`Length`](crate::Length), [`Rect`], [`Viewport`](crate::Viewport),
/// [`Transform2D`](crate::Transform2D), [`ViewTransform`](crate::ViewTransform),
/// [`ViewScale`](crate::ViewScale), [`Magnification`](crate::Magnification),
/// [`AxisMapping`](crate::AxisMapping) and [`UiZoom`](crate::UiZoom). Nothing is
/// clamped: an invalid value is rejected with the reason. It converts into
/// [`RuntimeError::Geometry`](crate::RuntimeError::Geometry), and through it into
/// [`DrawError`](crate::DrawError) and the facade's application error, so `?`
/// works in every callback.
///
/// ```
/// # use pert_ui_runtime as pert_ui;
/// use pert_ui::{GeometryError, Rect, UiZoom, ViewScale};
/// assert_eq!(Rect::new(0.0, 0.0, -1.0, 4.0), Err(GeometryError::Negative));
/// assert_eq!(ViewScale::new(64.0), Err(GeometryError::ViewScaleOutOfRange));
/// assert_eq!(
///     UiZoom::new(4.0).unwrap_err().to_string(),
///     "zoom factor is outside 0.75..=2"
/// );
/// ```
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum GeometryError {
    /// A coordinate, extent, matrix entry, factor, angle or computed result is
    /// NaN or infinite.
    #[error("geometry value is not finite")]
    NonFinite,
    /// A width, height, length or available extent is negative.
    #[error("extent is negative")]
    Negative,
    /// An extent that must be nonzero, such as a window size, is zero.
    #[error("extent is zero")]
    Empty,
    /// A minimum exceeds its maximum.
    #[error("minimum exceeds maximum")]
    InvertedBounds,
    /// A scale factor is zero or negative. An [`AxisMapping`](crate::AxisMapping)
    /// also rejects a subnormal scale here, whose reciprocal is not finite.
    #[error("scale must be positive")]
    NonPositiveScale,
    /// The matrix collapses the plane onto a line or a point, so it has no inverse.
    #[error("transform is not invertible")]
    Singular,
    /// A view scale lies outside [`ViewScale::MIN`](crate::ViewScale::MIN)`..=`[`ViewScale::MAX`](crate::ViewScale::MAX).
    #[error(
        "view scale is outside {}..={}",
        crate::ViewScale::MIN.get(),
        crate::ViewScale::MAX.get()
    )]
    ViewScaleOutOfRange,
    /// A UI zoom factor lies outside [`UiZoom::MIN`](crate::UiZoom::MIN)`..=`[`UiZoom::MAX`](crate::UiZoom::MAX).
    #[error(
        "zoom factor is outside {}..={}",
        crate::UiZoom::MIN.get(),
        crate::UiZoom::MAX.get()
    )]
    ZoomOutOfRange,
}

/// Finite nonnegative width and height in logical pixels.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Size {
    width: f32,
    height: f32,
}
impl From<[u16; 2]> for Size {
    /// Integer logical-pixel dimensions are always finite and nonnegative.
    fn from([width, height]: [u16; 2]) -> Self {
        Self {
            width: f32::from(width),
            height: f32::from(height),
        }
    }
}
impl Size {
    /// Zero extent on both axes.
    pub const ZERO: Self = Self {
        width: 0.0,
        height: 0.0,
    };
    /// Validates finite, nonnegative dimensions.
    ///
    /// # Errors
    /// [`GeometryError::NonFinite`] for NaN or an infinity and
    /// [`GeometryError::Negative`] for a negative dimension.
    pub fn new(width: f32, height: f32) -> Result<Self, GeometryError> {
        if !width.is_finite() || !height.is_finite() {
            return Err(GeometryError::NonFinite);
        }
        if width < 0.0 || height < 0.0 {
            return Err(GeometryError::Negative);
        }
        Ok(Self { width, height })
    }
    /// Width in logical pixels.
    #[must_use]
    pub const fn width(self) -> f32 {
        self.width
    }
    /// Height in logical pixels.
    #[must_use]
    pub const fn height(self) -> f32 {
        self.height
    }
}

/// Available measurement space in logical pixels, with an explicit unbounded
/// value on each axis.
///
/// This is available space, not permission to violate a child's intrinsic minimum.
/// A component may report a larger extent, which its container clips or scrolls.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Constraints {
    width: Option<f32>,
    height: Option<f32>,
}

/// Hard outer size limits, independent of preferred measurement and allocation.
/// Default permits any finite nonnegative size. Unbounded axes use `None`.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SizeLimits {
    minimum: Size,
    maximum: Constraints,
}
impl SizeLimits {
    /// Specifies a hard minimum with no upper bound.
    #[must_use]
    pub const fn at_least(minimum: Size) -> Self {
        Self {
            minimum,
            maximum: Constraints {
                width: None,
                height: None,
            },
        }
    }
    /// Specifies one fixed outer size: `size` is both the minimum and the
    /// maximum on each axis.
    ///
    /// Equal bounds cannot be inverted, so unlike [`SizeLimits::new`] this
    /// cannot fail, and a fixed-size component can return it from
    /// [`Component::size_limits`](crate::Component::size_limits) directly.
    /// A size in whole logical pixels is built infallibly with
    /// `Size::from([width, height])`; [`Size::new`] takes fractional values
    /// and fails on a negative or non-finite one.
    ///
    /// ```
    /// # use pert_ui_runtime as pert_ui;
    /// use pert_ui::{Constraints, Size, SizeLimits};
    /// let size = Size::from([260, 260]); // the infallible form
    /// let fixed = SizeLimits::exact(size);
    /// assert_eq!(fixed, SizeLimits::new(size, Constraints::bounded(size))?);
    /// assert_eq!(fixed.constrain(Size::from([400, 10])), size);
    /// assert_eq!(size, Size::new(260.0, 260.0)?);
    /// # Ok::<(), pert_ui::GeometryError>(())
    /// ```
    #[must_use]
    pub const fn exact(size: Size) -> Self {
        Self {
            minimum: size,
            maximum: Constraints::bounded(size),
        }
    }
    pub(crate) fn available(self, constraints: Constraints) -> Constraints {
        fn axis(value: Option<f32>, minimum: f32, maximum: Option<f32>) -> Option<f32> {
            match (value, maximum) {
                (Some(value), maximum) => Some(value.max(minimum).min(maximum.unwrap_or(f32::MAX))),
                (None, maximum) => maximum,
            }
        }
        Constraints {
            width: axis(constraints.width, self.minimum.width, self.maximum.width),
            height: axis(constraints.height, self.minimum.height, self.maximum.height),
        }
    }
    /// Validates that each finite upper bound is at least the corresponding minimum.
    ///
    /// # Errors
    /// [`GeometryError::InvertedBounds`] when a maximum is below its minimum.
    pub fn new(minimum: Size, maximum: Constraints) -> Result<Self, GeometryError> {
        if maximum.width().is_some_and(|v| v < minimum.width())
            || maximum.height().is_some_and(|v| v < minimum.height())
        {
            return Err(GeometryError::InvertedBounds);
        }
        Ok(Self { minimum, maximum })
    }
    /// Smallest permitted logical outer size.
    #[must_use]
    pub const fn minimum(self) -> Size {
        self.minimum
    }
    /// Upper logical bounds; `None` means unbounded on that axis.
    #[must_use]
    pub const fn maximum(self) -> Constraints {
        self.maximum
    }
    /// Clamps a measured size into these limits, raising it to the minimum first.
    ///
    /// This is the last step of a container's own `measure`: a preferred or natural
    /// size is a request, while limits are hard. The minimum wins over a smaller
    /// maximum only for limits built outside [`SizeLimits::new`], which rejects that
    /// combination. Both extents stay finite and nonnegative, so the result is always
    /// a valid [`Size`].
    ///
    /// ```
    /// # use pert_ui_runtime as pert_ui;
    /// # use pert_ui::{Constraints, Size, SizeLimits};
    /// let limits = SizeLimits::new(Size::new(40.0, 20.0)?, Constraints::new(Some(80.0), None)?)?;
    /// let size = limits.constrain(Size::new(120.0, 10.0)?);
    /// assert_eq!((size.width(), size.height()), (80.0, 20.0));
    /// # Ok::<(), pert_ui::GeometryError>(())
    /// ```
    #[must_use]
    pub fn constrain(self, size: Size) -> Size {
        Size {
            width: size
                .width
                .max(self.minimum.width)
                .min(self.maximum.width.unwrap_or(f32::MAX)),
            height: size
                .height
                .max(self.minimum.height)
                .min(self.maximum.height.unwrap_or(f32::MAX)),
        }
    }
}
impl Constraints {
    /// Creates available space, each extent in logical pixels; `None` means
    /// unbounded.
    ///
    /// # Errors
    /// [`GeometryError::NonFinite`] for NaN or an infinity (unbounded is `None`)
    /// and [`GeometryError::Negative`] for a negative extent.
    pub fn new(width: Option<f32>, height: Option<f32>) -> Result<Self, GeometryError> {
        let values = [width, height].into_iter().flatten();
        if values.clone().any(|v| !v.is_finite()) {
            return Err(GeometryError::NonFinite);
        }
        if values.into_iter().any(|v| v < 0.0) {
            return Err(GeometryError::Negative);
        }
        Ok(Self { width, height })
    }
    /// Bounded available space from a validated size.
    #[must_use]
    pub const fn bounded(size: Size) -> Self {
        Self {
            width: Some(size.width),
            height: Some(size.height),
        }
    }
    /// Available width in logical pixels; `None` is unbounded.
    #[must_use]
    pub const fn width(self) -> Option<f32> {
        self.width
    }
    /// Available height in logical pixels; `None` is unbounded.
    #[must_use]
    pub const fn height(self) -> Option<f32> {
        self.height
    }
}

// Offset that centers `content` within `available` on one axis, never negative:
// content at least as large as the space starts at the leading edge and overflows
// the trailing one, which a clip or an ellipsis then handles.
pub(crate) fn centered(available: f32, content: f32) -> f32 {
    ((available - content) / 2.0).max(0.0)
}

// Rectangle from raw layout arithmetic, as a runtime error for layout callbacks.
// Containers that already hold a validated `Size`, and optionally a `Point`, use
// `Rect::from_size`/`Rect::from_origin_size` and need no conversion.
pub(crate) fn rect(x: f32, y: f32, width: f32, height: f32) -> Result<Rect, RuntimeError> {
    Ok(Rect::new(x, y, width, height)?)
}

/// Owned keyed description used by [`Runtime::mount_tree`].
///
/// Descriptions are constructed on the UI thread at mount/structural-update time,
/// never per frame. Sibling keys must be nonempty and unique. A description owns
/// its component and children; mounting establishes runtime identity separately.
#[must_use]
pub struct Element {
    pub(crate) key: String,
    pub(crate) addressing: Addressing,
    pub(crate) exposed_part: Option<String>,
    pub(crate) label: Option<String>,
    pub(crate) rich_label: Option<crate::RichText>,
    /// A label from [`Self::with_label`], kept so a binding can be read.
    pub(crate) label_source: Option<crate::TextSource>,
    /// An icon a container composes before a bound label, such as a tab's
    /// leading icon slot.
    pub(crate) label_leading: Option<crate::IconSource>,
    pub(crate) detachable: crate::Permission,
    pub(crate) searchable: bool,
    /// The share of a row's or column's spare main-axis space; see
    /// [`Self::with_grow`].
    pub(crate) grow: u16,
    pub(crate) component: Box<dyn Component>,
    pub(crate) reconcile: Reconcile,
    pub(crate) kind: std::any::TypeId,
    pub(crate) children: Vec<Self>,
}
/// Shows the key, the fixed label and the children, so a described tree
/// formats as its keys. The component, a bound label and the callbacks are
/// not formatted.
impl std::fmt::Debug for Element {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Element")
            .field("key", &self.key)
            .field("label", &self.label)
            .field("children", &self.children)
            .finish_non_exhaustive()
    }
}
pub(crate) type Reconcile =
    fn(&mut dyn Component, Box<dyn Component>) -> Option<crate::Invalidation>;

/// The empty leaf of [`Element::spacer`]: no size of its own, no drawing.
struct Spacer;
impl Component for Spacer {
    fn update(&mut self, _: &mut crate::UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
}

/// Describes a component as a keyed [`Element`] in place, so a tree reads in
/// the order it is built.
///
/// `component.key("status")` is exactly `Element::new("status", component)`:
/// the key keeps the same sibling-scoped identity role, and validation still
/// happens when the element is mounted. Every [`Component`] has it through a
/// blanket implementation; implementing the trait yourself is never needed.
///
/// ```
/// # use pert_ui_runtime as pert_ui;
/// use pert_ui::{Button, Column, IntoElement, Label, Runtime};
/// let mut ui = Runtime::new();
/// let saved = ui.signal(false);
/// let save = saved.clone();
/// ui.mount_tree(
///     Column::new([
///         Label::new("Settings").key("title"),
///         Button::new("Save", move |_| save.set(true)).key("save"),
///     ])
///     .key("content"),
/// )?;
/// # Ok::<(), pert_ui::RuntimeError>(())
/// ```
pub trait IntoElement: Component + Sized + 'static {
    /// Describes this component under `key`, as [`Element::new`] does.
    fn key(self, key: impl Into<String>) -> Element {
        Element::new(key, self)
    }
}
impl<C: Component + 'static> IntoElement for C {}

impl Element {
    /// Describes a component. Key validation occurs atomically before mounting.
    pub fn new<C: Component + 'static>(key: impl Into<String>, mut component: C) -> Self {
        let children = component.take_children();
        Self {
            key: key.into(),
            addressing: Addressing::Own,
            exposed_part: None,
            label: None,
            rich_label: None,
            label_source: None,
            label_leading: None,
            detachable: crate::Permission::allowed(),
            searchable: true,
            grow: 0,
            kind: std::any::TypeId::of::<C>(),
            component: Box::new(component),
            reconcile: |current, next| {
                let current: &mut dyn std::any::Any = current;
                let next: Box<dyn std::any::Any> = next;
                // Reconciliation validates concrete types before any mutation.
                match (current.downcast_mut::<C>(), next.downcast::<C>()) {
                    (Some(current), Ok(next)) => current.reconcile(*next),
                    _ => None,
                }
            },
            children,
        }
    }
    /// Adds a display label independent of the stable key. Containers such as Tabs
    /// consume this metadata; reconciliation updates it without remounting state.
    ///
    /// The label is a [`TextSource`](crate::TextSource). Fixed rich content
    /// supplies [`Self::rich_label`] as well, with its plain text as
    /// [`Self::label`]. A binding is read by the runtime in the parent's
    /// update, which it subscribes, so a container sees the current text in
    /// [`ChildInfo::label`](crate::ChildInfo::label) and
    /// [`ChildInfo::rich_label`](crate::ChildInfo::rich_label) without
    /// polling, and the label names the component as below. Until that first
    /// read the label is absent; a mounted child described again with the
    /// same binding keeps the text already read. A binding from another
    /// runtime fails the parent's update with
    /// [`RuntimeError::ForeignRuntime`](crate::RuntimeError::ForeignRuntime).
    ///
    /// The label also names the component in inspection when the component
    /// reports [`Semantics`](crate::Semantics) without a name of its own, so a
    /// control that has no visible label, such as a search field or a list, can
    /// still be named where it is described. A name the component reports
    /// itself always wins, and a blank label names nothing.
    ///
    /// ```
    /// # use pert_ui_runtime as pert_ui;
    /// use pert_ui::{Element, Runtime, RuntimeError, SearchField};
    /// let mut ui = Runtime::new();
    /// let query = ui.signal(String::new());
    /// ui.mount_tree(Element::new("search", SearchField::new(query)).with_label("Find pages"))?;
    /// let named = ui
    ///     .inspect()
    ///     .components
    ///     .into_iter()
    ///     .find(|component| component.key == "search")
    ///     .and_then(|component| component.details.semantics)
    ///     .and_then(|semantics| semantics.name);
    /// assert_eq!(named.as_deref(), Some("Find pages"));
    /// # Ok::<(), RuntimeError>(())
    /// ```
    pub fn with_label(mut self, label: impl Into<crate::TextSource>) -> Self {
        let source: crate::TextSource = label.into();
        match source.fixed() {
            Some(content) => {
                self.label = Some(content.plain_text());
                self.rich_label = content.as_plain().is_none().then(|| content.clone());
                self.label_source = None;
            }
            None => {
                self.label = None;
                self.rich_label = None;
                self.label_source = Some(source);
            }
        }
        self
    }
    /// The bound label given to [`Self::with_label`], or `None` for fixed or
    /// absent label metadata. A container that checks its children's labels
    /// when it is described accepts a bound label there and checks the text
    /// when it is read.
    #[must_use]
    pub fn label_source(&self) -> Option<&crate::TextSource> {
        self.label_source
            .as_ref()
            .filter(|source| source.is_bound())
    }
    /// Composes `icon` before a bound label when it is read, for a container
    /// descriptor's leading icon slot.
    pub(crate) fn with_label_leading(mut self, icon: Option<crate::IconSource>) -> Self {
        self.label_leading = icon;
        self
    }
    /// Restricts moving this view into its own native window with a fixed or
    /// reactive [`Permission`](crate::Permission).
    ///
    /// Defaults to permitted, which by itself changes nothing: no library gesture
    /// detaches a view unless a container offers one, and the only container that
    /// does is `Tabs::detachable`. A denial is a refusal that both that gesture
    /// and [`crate::Runtime::request_detach`],
    /// [`HostIntegration::prepare_detach`](crate::host::HostIntegration::prepare_detach)
    /// and [`crate::EventContext::detach_view`] honor, returning
    /// `RuntimeError::NotDetachable` instead of moving the view. The runtime
    /// reads the permission's current value when a detach is asked for, so a
    /// bound permission needs no subscription there; a binding from another
    /// runtime makes the request fail with `RuntimeError::ForeignRuntime`.
    /// Repeated calls combine restrictions, and a fixed denial cannot be lifted
    /// by a binding. Reconciliation replaces the permission with the new
    /// description's without remounting, and an already detached view keeps its
    /// window and can still re-dock. A container reads the permission through
    /// [`ChildInfo::detach_permission`](crate::ChildInfo::detach_permission).
    /// `Tab::detachable` sets it for a tab.
    ///
    /// ```
    /// # use pert_ui_runtime as pert_ui;
    /// use pert_ui::testing::HostIntegration;
    /// use pert_ui::{Element, Label, NativeWindowOptions, Runtime, RuntimeError};
    /// let mut ui = Runtime::new();
    /// let view = ui.mount_tree(
    ///     Element::new("readout", Label::new("Stays here")).with_detachable(false),
    /// )?;
    /// assert_eq!(
    ///     ui.prepare_detach(&view, NativeWindowOptions::new("Readout")?).err(),
    ///     Some(RuntimeError::NotDetachable),
    /// );
    /// # Ok::<(), RuntimeError>(())
    /// ```
    pub fn with_detachable(mut self, permission: impl Into<crate::Permission>) -> Self {
        self.detachable = self.detachable.and(permission);
        self
    }
    /// Whether this description permits native detachment now: the current
    /// value of its permission, read without subscribing, with a binding that
    /// cannot be read counted as a refusal. True by default.
    #[must_use]
    pub fn detachable(&self) -> bool {
        self.detachable.get().unwrap_or(false)
    }
    /// The permission given to [`Self::with_detachable`]; allowed by default.
    #[must_use]
    pub fn detach_permission(&self) -> &crate::Permission {
        &self.detachable
    }
    /// Includes or excludes this component and its whole subtree from
    /// [content search](crate::Runtime::content_search); included by default.
    ///
    /// Exclusion is the application's choice where it describes the content,
    /// for decorative text, sample data or anything it does not want offered
    /// as a destination. A concealed field never exposes its value whether or
    /// not it is excluded; this option also keeps its name out. Reconciliation
    /// updates it without remounting.
    ///
    /// ```
    /// # use pert_ui_runtime as pert_ui;
    /// use pert_ui::prelude::*;
    /// use pert_ui::ContentQuery;
    ///
    /// let mut ui = Runtime::new();
    /// ui.mount_tree(Element::new("root", Column::new([
    ///     Element::new("shown", Label::new("Mixer settings")),
    ///     Element::new("sample", Label::new("Mixer sample text")).with_searchable(false),
    /// ])))?;
    /// let hits = ui.content_search(ContentQuery::new("mixer")?).step(&ui)?.hits;
    /// assert_eq!(hits.len(), 1);
    /// assert_eq!(hits[0].text, "Mixer settings");
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn with_searchable(mut self, searchable: bool) -> Self {
        self.searchable = searchable;
        self
    }
    /// Whether this description takes part in content search; true by default.
    #[must_use]
    pub fn searchable(&self) -> bool {
        self.searchable
    }
    /// Gives this element a share of the spare main-axis space of the
    /// [`Row`](crate::Row) or [`Column`](crate::Column) it is in: after every
    /// child has its natural size, the space left over is divided among the
    /// growing children in proportion to their weights, each up to its own
    /// maximum size, and what a maximum stops goes to the others. `0`, the
    /// default, keeps the natural size. The spare space is what the
    /// container's own allocation leaves: a column in a vertical scroll area
    /// is given at least the visible height, so a growing child fills the
    /// visible space the content leaves, and nothing once the content is
    /// taller. In a row whose width or a column whose height is bounded, a
    /// growing child also takes its share of a shortfall: when the natural
    /// lengths do not fit, growing children give up the difference by weight,
    /// down to each one's minimum, so a growing list or editor fills what the
    /// others leave instead of keeping its whole natural length, and a growing
    /// bar or label beside fixed content fits a narrow cell. A row measures
    /// each child it shrank again at its final width, so a label that wraps
    /// gets the height of its new lines and the row grows to it, and a label
    /// with [`TextOverflow::Ellipsis`](crate::TextOverflow::Ellipsis)
    /// truncates. Children without a weight keep their natural length, so a
    /// container whose ungrown children and grown minimums exceed it still
    /// overflows, which development validation reports as
    /// [`ValidationCode::EscapesParent`](crate::ValidationCode::EscapesParent);
    /// a child that must keep a width keeps it as a minimum, for example
    /// through [`Container::size_limits`](crate::Container::size_limits).
    /// [`Wrap`](crate::Wrap) and other containers ignore the
    /// weight unless they document otherwise; a custom container reads it
    /// with [`MeasureContext::grow`]. Reconciliation updates it without
    /// remounting.
    ///
    /// ```
    /// # use pert_ui_runtime as pert_ui;
    /// use pert_ui::{Column, IntoElement, Label, Runtime, TextArea, Viewport};
    /// use std::time::Duration;
    /// let mut ui = Runtime::new();
    /// ui.set_viewport(Viewport::new(400.0, 300.0, 1.0)?)?;
    /// let text = ui.signal(String::new());
    /// let root = ui.mount_tree(
    ///     Column::new([
    ///         Label::new("Notes").key("title"),
    ///         TextArea::new(&text).key("body").with_grow(1),
    ///     ])
    ///     .key("editor"),
    /// )?;
    /// ui.flush(Duration::ZERO)?;
    /// let body = ui.find(&root, &["body"])?.ok_or("mounted")?;
    /// let bounds = ui.bounds(&body)?.ok_or("laid out")?;
    /// // The text area takes the column's spare height, down to the window edge.
    /// assert_eq!(bounds.y() + bounds.height(), 300.0);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// A growing bar gives up width so the percentage beside it stays in a
    /// window narrower than the bar's preferred 192 pixels plus the label:
    ///
    /// ```
    /// # use pert_ui_runtime as pert_ui;
    /// use pert_ui::{IntoElement, Label, ProgressBar, ProgressStatus, Row, Runtime, Viewport};
    /// use std::time::Duration;
    /// let mut ui = Runtime::new();
    /// ui.set_viewport(Viewport::new(120.0, 40.0, 1.0)?)?;
    /// let root = ui.mount_tree(
    ///     Row::new([
    ///         ProgressBar::new(ProgressStatus::of(1, 2)).key("bar").with_grow(1),
    ///         Label::new("50%").key("percent"),
    ///     ])
    ///     .key("status"),
    /// )?;
    /// ui.flush(Duration::ZERO)?;
    /// let percent = ui.find(&root, &["percent"])?.ok_or("mounted")?;
    /// let bounds = ui.bounds(&percent)?.ok_or("laid out")?;
    /// // Its allocation ends inside the window, not only its clipped bounds.
    /// assert!(bounds.x() + bounds.width() <= 120.0);
    /// let bar = ui.find(&root, &["bar"])?.ok_or("mounted")?;
    /// assert!(ui.bounds(&bar)?.ok_or("laid out")?.width() < 192.0);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn with_grow(mut self, weight: u16) -> Self {
        self.grow = weight;
        self
    }
    /// This description's growth weight; `0`, natural size, by default.
    #[must_use]
    pub fn grow(&self) -> u16 {
        self.grow
    }
    /// An empty element that takes a row's or column's spare space with
    /// weight 1, pushing the elements after it to the far end.
    ///
    /// ```
    /// # use pert_ui_runtime as pert_ui;
    /// use pert_ui::{Element, IntoElement, Label, Row};
    /// let status = Row::new([
    ///     Label::new("Saved").key("message"),
    ///     Element::spacer("gap"),
    ///     Label::new("12 words").key("count"),
    /// ]);
    /// ```
    pub fn spacer(key: impl Into<String>) -> Self {
        Self::new(key, Spacer).with_grow(1)
    }
    /// Supplies styled display metadata, consumed by containers such as Tabs.
    /// `label()` returns its text without icons. If that text is blank, an existing
    /// label is preserved as its human-readable name; otherwise the name is absent.
    /// For icon-only metadata, call `with_label("Name")` before this method.
    /// Keys remain independent.
    pub fn with_rich_label(mut self, label: crate::RichText) -> Self {
        self.label_source = None;
        let plain = label.plain_text();
        if !plain.trim().is_empty() {
            self.label = Some(plain);
        }
        self.rich_label = Some(label);
        self
    }
    /// Optional styled label metadata. Ordinary `with_label` leaves this absent.
    pub fn rich_label(&self) -> Option<&crate::RichText> {
        self.rich_label.as_ref()
    }
    /// Stable sibling key; mounting validates it independently of display labels.
    pub fn key(&self) -> &str {
        &self.key
    }
    /// Optional description label, independent of the component's own text;
    /// `None` for a bound label, which the runtime reads after mounting.
    pub fn label(&self) -> Option<&str> {
        self.label.as_deref()
    }
    /// Supplies ordered children; built-in containers do this automatically.
    pub fn children(mut self, children: impl IntoIterator<Item = Self>) -> Self {
        self.children = children.into_iter().collect();
        self
    }
    /// Marks this description as internal structure of the component that built
    /// it, for [workspace persistence](crate::Workspace).
    ///
    /// A structural element contributes no segment to a
    /// [`WorkspacePath`](crate::WorkspacePath), for itself or for its
    /// descendants, and its own state is never captured. Use it for wrappers,
    /// viewports and layout scaffolding a container creates for itself: their
    /// keys are an implementation detail, and keeping them out of the document
    /// means restructuring a container does not invalidate saved workspaces.
    ///
    /// Application-supplied children nested inside remain addressable under
    /// their own keys, so a structural wrapper is invisible rather than opaque.
    /// The marking is fixed at mount; reconciliation does not change it.
    pub fn structural(mut self) -> Self {
        self.addressing = Addressing::Structural;
        self
    }
    /// Exposes this element as a named part of its enclosing composite.
    ///
    /// The name is for [`Runtime::find_exposed_part`](crate::Runtime::find_exposed_part)
    /// and the testing driver's part queries. It is independent of the logical
    /// key, semantic name and workspace addressing: this does not change
    /// [`Self::structural`], [`Self::part`] or [`Self::slot`]. A second call
    /// replaces the name, and a same-key redescription replaces or removes it
    /// without remounting the component or invalidating layout or paint solely
    /// for that change.
    ///
    /// A name must contain a non-whitespace character; mounting or adopting a
    /// blank declaration fails with [`RuntimeError::EmptyExposedPartName`]
    /// before mutation. Matching is exact, without trimming or splitting `/`.
    /// A part is a nested lookup scope: expose an internally composed control
    /// under its own name to keep its parts separate. Ordinary application
    /// children do not merge their descendants into the enclosing scope.
    /// Duplicate declared names remain inspectable, but singular lookup refuses
    /// their ambiguity. A name selects a current part; retain its `WidgetId`,
    /// not this name, when addressing an event to that incarnation.
    ///
    /// ```
    /// # use pert_ui_runtime as pert_ui;
    /// use pert_ui::prelude::*;
    /// let mut ui = Runtime::new();
    /// let root = ui.mount_tree(Container::new(
    ///     Button::new("Apply", |_| Ok(())).key("button").expose_part("action"),
    /// ).key("panel"))?;
    /// let action = ui.find_exposed_part(&root, "action")?
    ///     .ok_or_else(|| ApplicationError::from("the panel exposes action"))?;
    /// assert_eq!(ui.key(&action)?, "button");
    /// assert!(ui.find_exposed_part(&root, "button")?.is_none());
    /// # Ok::<(), RuntimeError>(())
    /// ```
    pub fn expose_part(mut self, name: impl Into<String>) -> Self {
        self.exposed_part = Some(name.into());
        self
    }
    /// Names a structural element's position, for [workspace
    /// persistence](crate::Workspace).
    ///
    /// The name replaces this element's key as its path segment, for itself and
    /// for its descendants. A container that routes several application-supplied
    /// children through internal structure must declare a slot per position:
    /// without one, two children the application happened to key alike would
    /// address the same path. Names are stable vocabulary, such as `left` or
    /// `center`, and are part of the document format.
    pub fn slot(mut self, name: impl Into<String>) -> Self {
        self.addressing = Addressing::Slot(name.into());
        self
    }
    /// Names an internal element whose own state is worth persisting, for
    /// [workspace persistence](crate::Workspace).
    ///
    /// The name addresses this element's state directly under the nearest
    /// addressable ancestor, and descendants address as if it were
    /// [`structural`](Self::structural). That keeps an internal participant's
    /// address independent of how deeply the container nests it: a divider named
    /// `left-divider` stays `frame/left-divider` however many other dividers
    /// wrap it.
    pub fn part(mut self, name: impl Into<String>) -> Self {
        self.addressing = Addressing::Part(name.into());
        self
    }
}

/// How a mounted element contributes to [`WorkspacePath`](crate::WorkspacePath)
/// addressing.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) enum Addressing {
    /// The element's key is its path segment. The default for every description.
    #[default]
    Own,
    /// Contributes nothing and never participates.
    Structural,
    /// Contributes `name` for itself and its descendants.
    Slot(String),
    /// Contributes `name` for itself only.
    Part(String),
}

/// Measures a custom container's visible children without exposing runtime internals.
///
/// Calls are UI-local, read-only and cached by child identity/constraints. Binding
/// reads belong in `Component::update`. Hidden children are absent from this list.
pub struct MeasureContext<'a> {
    pub(crate) node: &'a std::rc::Rc<crate::runtime::Node>,
    pub(crate) reads: &'a std::cell::Cell<u64>,
    pub(crate) font_reads: &'a std::cell::Cell<u64>,
    pub(crate) runtime: &'a Runtime,
    pub(crate) children: &'a [u64],
}

/// Shows the number of visible children, not their measurements.
impl std::fmt::Debug for MeasureContext<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MeasureContext")
            .field("children", &self.children.len())
            .finish_non_exhaustive()
    }
}
impl MeasureContext<'_> {
    /// Current logical window bounds for anchored surface sizing. Native resize
    /// and UI zoom invalidate layout before this value is consumed again.
    #[must_use]
    pub fn viewport(&self) -> crate::Viewport {
        self.runtime.viewport()
    }

    /// Physical pixels per component-local logical pixel used to prepare retained
    /// raster coverage. This combines the effective window scale with accumulated
    /// ancestor [`crate::ViewTransform`] scales, using the same density as text.
    /// Magnification is capped at the greater of the window scale and 8 physical
    /// pixels per local pixel; shrinking uses the accumulated scale without that
    /// cap. Thus a denser window is never reduced by the view rasterization cap.
    ///
    /// A single view-scale change invalidates layout immediately. During a
    /// continuous zoom this value retains the prepared scale until the view
    /// settles; composition stretches existing coverage in the meantime. Panning
    /// does not change it. A promoted surface uses its native window's scale.
    /// Custom raster widgets use this value during measure/arrange and retain
    /// their resources until their physical dimensions change.
    #[must_use]
    pub fn raster_scale(&self) -> f64 {
        self.node.raster_scale()
    }

    /// Resolves a theme measurement and subscribes this component's current layout
    /// callback. Changes invalidate layout and paint; hidden components defer work.
    /// Limits, measure and arrange collect separate dependencies, including text
    /// size reads made by `text`. Custom widgets use the same path as built-ins.
    #[must_use]
    pub fn metric(&self, role: crate::MetricRole) -> f32 {
        self.reads.set(self.reads.get() | role.mask());
        self.runtime.metric(self.node, role)
    }

    /// Prepares shared default typography at this component's effective scale:
    /// the viewport scale times the accumulated
    /// [view scale](crate::ViewTransform) of its ancestors, so text inside a
    /// magnified view is prepared at its displayed size, subject to the cap
    /// described by [`Self::raster_scale`]. A view scale change
    /// invalidates the layout of the components under it for that reason; during
    /// a continuous zoom that happens once the scale has rested, and text is
    /// prepared at the scale it was last prepared at until then.
    ///
    /// Width is optional logical wrapping space. The UI-local service caches at
    /// most 128 preparations and 16 MiB total for coverage and owned cache metadata,
    /// evicting oldest entries. Width changes that preserve all positioned glyphs
    /// and raster bounds reuse the same coverage. Valid preparations exceeding
    /// the cache budget remain usable without being cached.
    /// Live `TextLayout` owners retain their own shared resources beyond eviction.
    ///
    /// # Errors
    /// Returns [`RuntimeError::Text`] for invalid geometry (a non-finite or negative width,
    /// a zero font size), an unreadable font or a result beyond a
    /// [`TextLimit`](crate::TextLimit); [`RuntimeError::TextEdit`] with
    /// [`TextEditError::TooLarge`](crate::TextEditError::TooLarge) for text longer than
    /// [`TextEditError::MAX_TEXT_BYTES`](crate::TextEditError::MAX_TEXT_BYTES); and
    /// `BorrowConflict` when text is prepared reentrantly.
    pub fn text(
        &self,
        text: &str,
        width: Option<f32>,
        style: crate::TextStyle,
    ) -> Result<crate::TextLayout, RuntimeError> {
        self.runtime.track_fonts(self.node);
        self.runtime.prepare_text(
            self.node,
            text,
            crate::text::TextRequest::new(style, self.metric(style.metric()), self.font_reads)
                .width(width),
        )
    }
    /// Prepares plain text to fit `width` logical pixels as `overflow` asks:
    /// wrapped at the width, or kept on one line with an ellipsis replacing the
    /// grapheme clusters that do not fit. This is how [`Label::overflow`](crate::Label::overflow)
    /// truncates, and custom widgets use the same path.
    ///
    /// Typography, scale, tracking and caching are those of [`Self::text`].
    /// A truncated result is searched by shaping candidates without
    /// rasterizing them; the cut is remembered per text, font, size, scale and
    /// width, so later layout passes at the same width prepare only the shown
    /// text. When even the ellipsis alone does not fit, the result shows the
    /// ellipsis and is wider than `width`; the caller clips it. With an
    /// ellipsis mode, a first line longer than
    /// [`TextEditError::MAX_TEXT_BYTES`](crate::TextEditError::MAX_TEXT_BYTES) is
    /// cut first, as [`TextOverflow`](crate::TextOverflow) describes, so its
    /// length is not an error and the result is always truncated.
    ///
    /// # Errors
    /// The errors of [`Self::text`], except that with an ellipsis mode the
    /// byte limit is never one; a shown truncation wider than the raster
    /// limit (in a very wide `width`) is still
    /// [`TextLimit::RasterSide`](crate::TextLimit::RasterSide). A non-finite
    /// or negative `width` is [`RuntimeError::Text`] with
    /// [`TextError::Geometry`](crate::TextError::Geometry).
    ///
    /// ```
    /// # use pert_ui_runtime as pert_ui;
    /// use pert_ui::{
    ///     Component, Constraints, Element, MeasureContext, Runtime, RuntimeError, Size,
    ///     TextOverflow, TextStyle, UpdateContext, Viewport,
    /// };
    /// use std::{cell::Cell, rc::Rc, time::Duration};
    ///
    /// struct Name(Rc<Cell<bool>>);
    /// impl Component for Name {
    ///     fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
    ///         Ok(())
    ///     }
    ///     fn measure(&self, cx: &MeasureContext<'_>, _: Constraints) -> Result<Size, RuntimeError> {
    ///         let fitted = cx.fitted_text(
    ///             "A rather long track name",
    ///             60.0,
    ///             TextStyle::Body,
    ///             TextOverflow::Ellipsis,
    ///         )?;
    ///         self.0.set(fitted.is_truncated());
    ///         Ok(fitted.layout().size())
    ///     }
    /// }
    ///
    /// let mut ui = Runtime::new();
    /// ui.set_viewport(Viewport::new(320.0, 80.0, 1.0)?)?;
    /// let truncated = Rc::new(Cell::new(false));
    /// ui.mount_tree(Element::new("name", Name(truncated.clone())))?;
    /// ui.flush(Duration::ZERO)?;
    /// assert!(truncated.get());
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn fitted_text(
        &self,
        text: &str,
        width: f32,
        style: crate::TextStyle,
        overflow: crate::TextOverflow,
    ) -> Result<crate::FittedText, RuntimeError> {
        self.runtime.track_fonts(self.node);
        self.runtime.prepare_fitted_text(
            self.node,
            text,
            crate::text::TextRequest::new(style, self.metric(style.metric()), self.font_reads),
            width,
            overflow,
        )
    }
    /// Prepares one rich Unicode paragraph with shared baseline and wrapping.
    /// Tracks font/icon roles and theme sizes for this layout callback. Exact fonts
    /// are immutable; role overrides invalidate only their readers. Span colors are
    /// resolved by `TextLayout::paint`, without reshaping on palette changes.
    /// Width is optional logical wrapping space, as for [`Self::text`].
    /// Uses the same limits/cache as `text`, plus at most
    /// [`RichText::MAX_SEGMENTS`](crate::RichText::MAX_SEGMENTS) inline segments.
    ///
    /// # Errors
    /// The errors of [`Self::text`], plus [`RuntimeError::Font`] with
    /// [`FontError::ForeignFont`](crate::FontError::ForeignFont) for an exact font or
    /// custom icon glyph of another runtime, and [`RuntimeError::Text`] with
    /// [`TextLimit::Segments`](crate::TextLimit::Segments) for more than
    /// [`RichText::MAX_SEGMENTS`](crate::RichText::MAX_SEGMENTS) inline segments.
    pub fn rich_text(
        &self,
        text: &crate::RichText,
        width: Option<f32>,
        style: crate::TextStyle,
    ) -> Result<crate::TextLayout, RuntimeError> {
        self.runtime.track_fonts(self.node);
        let size = self.metric(style.metric());
        let (icon_size, gap) = text.metrics(|role| self.metric(role));
        self.runtime.prepare_rich_text(
            self.node,
            text,
            crate::text::TextRequest::new(style, size, self.font_reads)
                .width(width)
                .inline(icon_size, gap),
        )
    }
    /// Stable sibling key at a visible child index; hidden children are absent.
    ///
    /// # Errors
    /// Returns [`RuntimeError::Layout`] with [`LayoutError::InvalidChild`] for an index
    /// outside the visible children.
    pub fn key(&self, index: usize) -> Result<&str, RuntimeError> {
        let serial = *self.children.get(index).ok_or(LayoutError::InvalidChild)?;
        Ok(self.runtime.child_key(serial))
    }
    /// Reads the child's preferred parent-local origin for free positioning.
    /// Flow containers may ignore this preference.
    ///
    /// # Errors
    /// Returns [`RuntimeError::Layout`] with [`LayoutError::InvalidChild`] for an index
    /// outside the visible children.
    pub fn position(&self, index: usize) -> Result<crate::Point, RuntimeError> {
        let serial = *self.children.get(index).ok_or(LayoutError::InvalidChild)?;
        Ok(self.runtime.child_position(serial))
    }
    /// Reads whether a visible child opts into the row's resolved content height.
    ///
    /// Custom horizontal containers may honor [`Component::fills_row_height`]
    /// with the same natural-then-stretched measurement policy as [`crate::Row`].
    /// The read-only query does not measure the child or schedule work.
    ///
    /// # Errors
    /// Returns [`RuntimeError::Layout`] with [`LayoutError::InvalidChild`] for an
    /// index outside the visible children.
    pub fn fills_row_height(&self, index: usize) -> Result<bool, RuntimeError> {
        let serial = *self.children.get(index).ok_or(LayoutError::InvalidChild)?;
        Ok(self.runtime.child_fills_row_height(serial))
    }
    /// A child's growth weight from [`Element::with_grow`]: its share of spare
    /// main-axis space in a container that distributes it, `0` for none. A
    /// custom container that grows children reads it here.
    ///
    /// # Errors
    /// Returns [`RuntimeError::Layout`] with [`LayoutError::InvalidChild`] for an
    /// index outside the visible children.
    pub fn grow(&self, index: usize) -> Result<u16, RuntimeError> {
        let serial = *self.children.get(index).ok_or(LayoutError::InvalidChild)?;
        Ok(self.runtime.child_grow(serial))
    }
    /// Reads a child's hard size limits.
    ///
    /// # Errors
    /// Returns [`RuntimeError::Layout`] with [`LayoutError::InvalidChild`] for an index
    /// outside the visible children, and forwards the errors of the child's
    /// [`Component::measure_limits`].
    pub fn limits(&self, index: usize) -> Result<SizeLimits, RuntimeError> {
        let serial = *self.children.get(index).ok_or(LayoutError::InvalidChild)?;
        self.runtime.child_limits(serial)
    }
    /// Number of visible children, in presentation order.
    #[must_use]
    pub fn len(&self) -> usize {
        self.children.len()
    }
    /// Whether there are no visible children.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.children.is_empty()
    }
    /// Measures a child under available space, reusing one of the child's
    /// four most recent measurements at equal constraints, as
    /// [`Component::measure`] describes.
    ///
    /// # Errors
    /// Returns [`RuntimeError::Layout`] with [`LayoutError::InvalidChild`] for an index
    /// outside the visible children, and forwards the errors of the child's
    /// [`Component::measure_limits`] and [`Component::measure`].
    pub fn measure(&self, index: usize, available: Constraints) -> Result<Size, RuntimeError> {
        let serial = *self.children.get(index).ok_or(LayoutError::InvalidChild)?;
        self.runtime.measure_node(serial, available)
    }
}

/// Assigns custom-container children rectangles relative to its content origin.
///
/// Every visible child must be placed exactly once. Rectangles may overflow;
/// runtime clips to ancestor allocations. State writes are forbidden. A returned
/// error leaves layout pending for retry; no partial layout is presented.
pub struct ArrangeContext<'a> {
    pub(crate) measure: MeasureContext<'a>,
    pub(crate) placements: Vec<Option<Rect>>,
    pub(crate) clips: Vec<Option<Rect>>,
}

/// Shows the number of visible children and the rectangles placed so far.
impl std::fmt::Debug for ArrangeContext<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ArrangeContext")
            .field("children", &self.measure.children.len())
            .field("placements", &self.placements)
            .finish_non_exhaustive()
    }
}
impl ArrangeContext<'_> {
    /// Borrows child measurement access, for algorithms shared with `measure`.
    #[must_use]
    pub fn measurement(&self) -> &MeasureContext<'_> {
        &self.measure
    }
    /// Number of visible children.
    #[must_use]
    pub fn len(&self) -> usize {
        self.measure.len()
    }
    /// Whether there are no visible children.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.measure.is_empty()
    }
    /// Measures a child, reusing one of the child's four most recent
    /// measurements at equal constraints, as [`Component::measure`] describes.
    ///
    /// # Errors
    /// As [`MeasureContext::measure`].
    pub fn measure(&self, index: usize, available: Constraints) -> Result<Size, RuntimeError> {
        self.measure.measure(index, available)
    }
    /// Measures child `index` again under the space this component's own
    /// [`Component::measure`] was last given, so the child's latest
    /// measurement is the one this component's measurement was computed from.
    /// That space is the constraints this component was last measured under,
    /// within its resolved size limits
    /// ([`Component::measure_limits`]), exactly
    /// as the runtime passes them to `measure`: a component with a maximum
    /// width refreshes its child at that width, not at the wider space its
    /// parent offered.
    ///
    /// It is for a component that forwards a child's measurement, whose
    /// [`Component::measure`] returns what `cx.measure(index, space)` gives
    /// for the child under the space it was offered, and that reports
    /// [`ValidationHints::fixed_height`](crate::ValidationHints::fixed_height).
    /// Development validation checks the child against the height it was last
    /// measured at. When the runtime answers this component's own measurement
    /// from its retained results, its `measure` is not called, so the child's
    /// latest measurement can be from other constraints, such as a column's
    /// previous width: a report of content too tall for the fixed height
    /// would then stay after the content fits, or be missed. Call it in
    /// [`Component::arrange`], before placing the child. A fixed-height
    /// component that measures its child in `arrange` with the constraints it
    /// gives, as a table cell does, needs no call; neither does a component
    /// that does not report `fixed_height`.
    ///
    /// The measurement reuses the child's retained result for those
    /// constraints when it has one. Before this component's first measurement
    /// it measures nothing. Development builds only: release builds keep no
    /// such record, and the call checks `index` and measures nothing.
    ///
    /// ```
    /// # use pert_ui_runtime as pert_ui;
    /// use pert_ui::component::*;
    /// use pert_ui::prelude::*;
    ///
    /// /// Gives its one child exactly its own measurement and size, and asks
    /// /// that the child fit the height it is given.
    /// struct Band(Option<Element>);
    /// impl Component for Band {
    ///     fn take_children(&mut self) -> Vec<Element> {
    ///         self.0.take().into_iter().collect()
    ///     }
    ///     fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
    ///         Ok(())
    ///     }
    ///     fn measure(&self, cx: &MeasureContext<'_>, space: Constraints) -> Result<Size, RuntimeError> {
    ///         cx.measure(0, space)
    ///     }
    ///     fn arrange(&self, cx: &mut ArrangeContext<'_>, size: Size) -> Result<(), RuntimeError> {
    ///         cx.refresh_forwarded(0)?;
    ///         cx.place(0, Rect::from_size(size))
    ///     }
    ///     fn inspect(&self) -> ComponentDetails {
    ///         let mut details = ComponentDetails::default();
    ///         details.validation.fixed_height = true;
    ///         details
    ///     }
    /// }
    ///
    /// let mut ui = Runtime::new();
    /// ui.mount_tree(Element::new("band", Band(Some(Element::new("text", Label::new("Fits"))))))?;
    /// ui.flush(ui.now())?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// # Errors
    /// Returns [`RuntimeError::Layout`] with [`LayoutError::InvalidChild`] for
    /// an index outside the visible children, and in development builds
    /// forwards the errors of this component's [`Component::measure_limits`]
    /// and of [`MeasureContext::measure`].
    pub fn refresh_forwarded(&self, index: usize) -> Result<(), RuntimeError> {
        if index >= self.len() {
            return Err(LayoutError::InvalidChild.into());
        }
        #[cfg(debug_assertions)]
        if let Some(constraints) = self
            .measure
            .runtime
            .forwarded_constraints(self.measure.node)?
        {
            self.measure.measure(index, constraints)?;
        }
        Ok(())
    }
    /// Places a child with an explicit parent-local clipping boundary for its entire
    /// subtree, including overflowing flow containers. Useful for independent grid
    /// cells; the boundary is intersected with inherited viewport clipping.
    ///
    /// # Errors
    /// As [`Self::place`].
    pub fn place_clipped(
        &mut self,
        index: usize,
        bounds: Rect,
        clip: Rect,
    ) -> Result<(), RuntimeError> {
        self.place(index, bounds)?;
        self.clips[index] = Some(clip);
        Ok(())
    }
    /// Requests one more update of this component after the current layout pass.
    ///
    /// Use it for retained decisions that depend on the final allocation, such as
    /// collapsing toolbar groups into an overflow surface: record the layout-derived
    /// state in interior-mutable fields, request the update, and apply it in
    /// `Component::update` by changing `child_visible` results and requesting
    /// layout. The runtime runs another update wave and layout inside the same
    /// flush, so the presented frame already reflects the applied state. The
    /// component must converge: a flush is limited to `Runtime::MAX_FLUSH_WAVES`
    /// waves and then fails with `RuntimeError::UpdateLimit`. Nothing is scheduled
    /// once the state settles; this is not a timer.
    pub fn request_update(&self) {
        self.measure.node.invalidate(crate::Invalidation::Paint);
    }
    /// Places a child once.
    ///
    /// # Errors
    /// Returns [`RuntimeError::Layout`] with [`LayoutError::InvalidChild`] for an index
    /// outside the visible children or a child placed before.
    pub fn place(&mut self, index: usize, rect: Rect) -> Result<(), RuntimeError> {
        let slot = self
            .placements
            .get_mut(index)
            .ok_or(LayoutError::InvalidChild)?;
        if slot.is_some() {
            return Err(LayoutError::InvalidChild.into());
        }
        *slot = Some(rect);
        Ok(())
    }
}
