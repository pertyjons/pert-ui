//! Backend-independent pointer events and handled propagation.
/// Finite logical coordinates or scroll displacement; negative values are allowed.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Point {
    x: f32,
    y: f32,
}
impl Point {
    /// The origin.
    pub const ZERO: Self = Self { x: 0.0, y: 0.0 };
    /// Creates a point from logical coordinates or a displacement.
    ///
    /// # Errors
    /// Returns [`GeometryError::NonFinite`](crate::GeometryError::NonFinite)
    /// for a coordinate that is NaN or infinite; every finite value, negative
    /// included, is accepted.
    pub fn new(x: f32, y: f32) -> Result<Self, crate::GeometryError> {
        if !x.is_finite() || !y.is_finite() {
            return Err(crate::GeometryError::NonFinite);
        }
        Ok(Self { x, y })
    }
    /// Horizontal logical coordinate/displacement.
    #[must_use]
    pub const fn x(self) -> f32 {
        self.x
    }
    /// Vertical logical coordinate/displacement.
    #[must_use]
    pub const fn y(self) -> f32 {
        self.y
    }
}
/// Physical pointer button identity, independent of application actions.
///
/// New buttons are named as platforms report them, so matches need a wildcard
/// arm.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
#[non_exhaustive]
pub enum PointerButton {
    /// Primary activation/drag button.
    Primary,
    /// Secondary/context-menu button.
    Secondary,
    /// Middle button.
    Middle,
    /// The mouse's side button for going back, as in a web browser.
    /// [`HistoryNavigator`](crate::HistoryNavigator) and
    /// [`HistoryButtons`](crate::HistoryButtons) can react to it.
    Back,
    /// The mouse's side button for going forward, as in a web browser.
    Forward,
    /// Additional native button identifier, for buttons with no name above.
    Other(u16),
}
/// Pointer transition translated by the platform adapter.
///
/// New kinds of pointer input are added as platforms report them, so matches
/// need a wildcard arm.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub enum PointerKind {
    /// Position changed within the window. The native `App` coalesces motion to
    /// the latest position before a frame or a discrete input boundary; consumers
    /// must not depend on receiving every physical sample or crossed hover target.
    /// Direct `Runtime::pointer_input` calls are dispatched individually.
    ///
    /// After a layout or stacking change the runtime also delivers a move at
    /// the last position, marked by [`PointerEvent::is_refresh`], so a handler
    /// can receive the same position more than once and must not invalidate
    /// again when nothing it derives from the event changed.
    Move,
    /// The pointer left the receiving component: the platform sends it when
    /// the pointer leaves the window, and the runtime delivers one to the
    /// previous target whenever the hit target changes within the window. Hover
    /// clears while capture may continue; the position is the last known one.
    Leave,
    /// A physical button went down.
    Press(PointerButton),
    /// A button went up. Controls must require a matching armed press.
    Release(PointerButton),
    /// Logical scroll displacement; positive values move toward later content.
    Scroll(Point),
    /// One step of a touchpad pinch at the pointer: content under the pointer
    /// should grow by this factor. It is routed like `Scroll`, to the component
    /// under the pointer and on to its ancestors until one handles it, and is
    /// the same in every local space, so a view transform does not convert it.
    /// Components that do not zoom ignore it. The native `App` delivers it where
    /// the windowing layer reports pinch gestures, which is macOS today; on
    /// Windows a precision touchpad's pinch arrives as a scroll with Control
    /// held instead.
    Pinch(Magnification),
}

/// A validated relative change of magnification: finite and positive, where 1
/// changes nothing, 1.1 grows by a tenth and 0.9 shrinks by a tenth.
///
/// ```
/// use pert_ui_runtime::Magnification;
/// assert_eq!(Magnification::new(1.25)?.get(), 1.25);
/// assert!(Magnification::new(0.0).is_err());
/// assert!(Magnification::new(f32::NAN).is_err());
/// # Ok::<(), pert_ui_runtime::GeometryError>(())
/// ```
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct Magnification(f32);

impl Magnification {
    /// Validates a factor.
    ///
    /// # Errors
    /// Returns [`GeometryError::NonFinite`](crate::GeometryError::NonFinite)
    /// for a non-finite factor and
    /// [`GeometryError::NonPositiveScale`](crate::GeometryError::NonPositiveScale)
    /// for zero or a negative one.
    pub fn new(factor: f32) -> Result<Self, crate::GeometryError> {
        if !factor.is_finite() {
            return Err(crate::GeometryError::NonFinite);
        }
        if factor <= 0.0 {
            return Err(crate::GeometryError::NonPositiveScale);
        }
        Ok(Self(factor))
    }
    /// The factor.
    #[must_use]
    pub const fn get(self) -> f32 {
        self.0
    }
}

impl TryFrom<f32> for Magnification {
    type Error = crate::GeometryError;
    fn try_from(factor: f32) -> Result<Self, Self::Error> {
        Self::new(factor)
    }
}
/// Owned pointer event with a validated logical window-coordinate position.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PointerEvent {
    position: Point,
    kind: PointerKind,
    outside: bool,
    refresh: bool,
}
impl PointerEvent {
    /// Creates an event from validated coordinates and displacement.
    #[must_use]
    pub const fn new(position: Point, kind: PointerKind) -> Self {
        Self {
            position,
            kind,
            outside: false,
            refresh: false,
        }
    }
    /// The move the runtime repeats at the last position after a layout or
    /// stacking change.
    pub(crate) const fn refreshed(mut self) -> Self {
        self.refresh = true;
        self
    }
    /// Whether the runtime generated this [`PointerKind::Move`] after a layout
    /// or stacking change, at the last position the host reported, rather
    /// than the host reporting motion. Always false for an event made with
    /// [`Self::new`], which is how every host-supplied event is made.
    ///
    /// The runtime repeats the last position so that a component under a
    /// stationary pointer can follow geometry that changed beneath it: a hover
    /// subregion that moved, a held scrollbar thumb whose content grew, a
    /// text selection whose layout changed. It is delivered like any move: to
    /// the capture owner while one exists, otherwise to the component under
    /// the pointer, bubbling as usual. It is not delivered during a drag
    /// session ([`EventContext::start_drag`](crate::EventContext::start_drag)).
    ///
    /// A handler must converge on it: when the state it derives from the event
    /// (a local position, a resolved drag offset, a hit subregion) is
    /// unchanged, it must not invalidate again, or the layout it requests
    /// produces the next refresh and the flush ends in
    /// [`RuntimeError::UpdateLimit`](crate::RuntimeError::UpdateLimit).
    /// Ignoring refreshes altogether is right only for a component whose
    /// result depends on the window position alone; see
    /// [`Component::pointer_input`](crate::Component::pointer_input) for a
    /// drag that converges.
    #[must_use]
    pub const fn is_refresh(self) -> bool {
        self.refresh
    }
    /// Marks coordinates as last-known while the pointer is outside the window.
    /// Capture still receives transitions, but hit testing and activation stay outside.
    #[must_use]
    pub const fn outside(mut self) -> Self {
        self.outside = true;
        self
    }
    /// Whether the position is history rather than a current hit on the
    /// receiver: true for an event marked [`Self::outside`] and for every
    /// [`PointerKind::Leave`], including one delivered within the window.
    #[must_use]
    pub const fn is_outside(self) -> bool {
        self.outside || matches!(self.kind, PointerKind::Leave)
    }
    /// Logical position; last known position for `Leave`.
    #[must_use]
    pub const fn position(self) -> Point {
        self.position
    }
    /// Pointer transition.
    #[must_use]
    pub const fn kind(self) -> PointerKind {
        self.kind
    }
}
/// Whether an input callback consumed an event.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum InputOutcome {
    /// Continue bubbling through logical ancestors.
    #[default]
    Ignored,
    /// Stop propagation; do not activate another target underneath.
    Handled,
}
/// Platform-independent pointer appearance for a component's current hit region.
/// Native adapters map these to their OS cursors; no OS handles enter the runtime.
///
/// Cursors are added as components need them, so a custom host's mapping needs
/// a wildcard arm, typically to [`Self::Default`].
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[non_exhaustive]
pub enum CursorIcon {
    /// Text selection and insertion (I-beam).
    Text,
    /// Ordinary pointing arrow.
    #[default]
    Default,
    /// A pointing hand over a link or another navigation target.
    Pointer,
    /// Left/right resizing across a vertical boundary.
    ResizeHorizontal,
    /// Up/down resizing across a horizontal boundary.
    ResizeVertical,
    /// Northwest/southeast resizing, such as a bottom-right size grip.
    ResizeDiagonal,
    /// Moving a freely positioned component.
    Move,
    /// Holding something that is being dragged, with no different outcome to show.
    Grabbing,
    /// Releasing here would copy what is being dragged.
    Copy,
    /// Releasing here would link to what is being dragged.
    Alias,
    /// What is being dragged cannot be dropped here.
    NoDrop,
}
