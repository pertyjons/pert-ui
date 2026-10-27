//! A pannable, zoomable surface of freely placed components.
use crate::layout::rect;
use crate::*;
use std::{cell::Cell, collections::BTreeMap, rc::Rc};

/// The smallest and largest scale a user's gestures reach in a [`ZoomView`].
///
/// The default is 1/8 to 8. [`Self::new`] accepts the two scales in either
/// order, so every pair of valid [`ViewScale`]s is a valid range. The limits bound
/// wheel, keyboard and fit gestures only: a view set through
/// [`ZoomController::set`] or restored from a workspace document is shown as it
/// is, within the range [`ViewScale`] itself accepts.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScaleLimits {
    min: ViewScale,
    max: ViewScale,
}

impl Default for ScaleLimits {
    fn default() -> Self {
        Self::DEFAULT
    }
}

impl ScaleLimits {
    /// 1/8 to 8, the default.
    pub const DEFAULT: Self = Self {
        min: ViewScale::EIGHTH,
        max: ViewScale::EIGHT,
    };
    /// The range between two scales, in either order.
    #[must_use]
    pub fn new(a: ViewScale, b: ViewScale) -> Self {
        if a <= b {
            Self { min: a, max: b }
        } else {
            Self { min: b, max: a }
        }
    }
    /// The smallest scale.
    #[must_use]
    pub const fn min(self) -> ViewScale {
        self.min
    }
    /// The largest scale.
    #[must_use]
    pub const fn max(self) -> ViewScale {
        self.max
    }
    /// The nearest scale inside the range. Infinities give the nearer end and
    /// NaN gives the smallest scale.
    #[must_use]
    pub fn clamp(self, factor: f32) -> ViewScale {
        if factor.is_nan() {
            return self.min;
        }
        let factor = factor.clamp(self.min.get(), self.max.get());
        ViewScale::new(factor).unwrap_or(self.min)
    }
}

/// What a plain scroll-wheel or touchpad scroll does in a [`ZoomView`].
///
/// The platform's primary modifier (Control, or Command on macOS) selects the
/// other gesture, so both are always reachable from a wheel.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum WheelGesture {
    /// Scrolling pans the view, and the primary modifier with scrolling zooms
    /// at the pointer (default). A vertical-only wheel pans horizontally while
    /// Shift is held. This suits touchpads, whose two-finger scroll is a pan.
    #[default]
    Pan,
    /// Scrolling zooms at the pointer, and the primary modifier with scrolling
    /// pans. This suits mouse-driven editors where the wheel mostly zooms.
    Zoom,
}

/// What a [`ZoomController`] learns from the view's last arrangement, so that it
/// can fit content from application code.
#[derive(Clone, Copy)]
struct Arranged {
    /// Content bounds as left, top, right and bottom, in `f64` so that bounds of
    /// far-apart items cannot overflow.
    content: Option<[f64; 4]>,
    viewport: Size,
    limits: ScaleLimits,
}

/// Reads and changes the view of a [`ZoomView`] from application code.
///
/// Created with [`Self::new`] and handed to the view with
/// [`ZoomView::controller`]. The controller's [`Self::view`] signal is the
/// view's current transform: the view writes it for every pan and zoom the user
/// makes, and adopts at its next update whatever the application writes with
/// [`Self::set`] or [`Self::fit`]. Both are UI-local signal writes, so they may
/// be issued from event handlers and application code, never from a component's
/// `update`. One controller serves one mounted view at a time, including a
/// hidden one: [`Self::fit`] uses that view's last arrangement, which the view
/// forgets when it is unmounted or handed another controller, so `fit` then
/// returns `false` rather than fitting a view that no longer shows it. The
/// controller may outlive its view and serve a later one. Handing it to a
/// second view mounted at the same time is unsupported: the views overwrite
/// each other's arrangement, as [`ListController`](crate::ListController)'s
/// one-owner rule describes for reveal requests.
///
/// ```
/// # use pert_ui_runtime as pert_ui;
/// use pert_ui::*;
/// let mut ui = Runtime::new();
/// ui.set_viewport(Viewport::new(400.0, 300.0, 1.0)?)?;
/// let zoom = ZoomController::new(&ui);
/// let at = |x, y| Point::new(x, y);
/// let view = ZoomView::new([
///     (at(0.0, 0.0)?, Element::new("a", Label::new("First"))),
///     (at(1200.0, 900.0)?, Element::new("b", Label::new("Far away"))),
/// ])
/// .controller(&zoom);
/// ui.mount_tree(Element::new("canvas", view))?;
/// ui.flush(std::time::Duration::ZERO)?;
/// // Both labels fit once the view shrinks its content.
/// assert!(zoom.fit()?);
/// ui.flush(std::time::Duration::ZERO)?;
/// assert!(zoom.view().get()?.scale().get() < 1.0);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Clone)]
pub struct ZoomController {
    view: Signal<ViewTransform>,
    arranged: Rc<Cell<Option<Arranged>>>,
}

impl ZoomController {
    /// Creates a controller belonging to `ui`, starting at the identity view.
    ///
    /// `ui` is any [`StateOwner`](crate::StateOwner): the
    /// [`Runtime`](crate::Runtime) while the application builds its tree, or
    /// the [`UpdateContext`](crate::UpdateContext) or
    /// [`EventContext`](crate::EventContext) of a component that builds this
    /// while it describes its children.
    #[must_use]
    pub fn new(ui: &impl crate::StateOwner) -> Self {
        Self {
            view: ui.signal(ViewTransform::IDENTITY),
            arranged: Rc::new(Cell::new(None)),
        }
    }
    /// The view's current transform. Read it in `update` to follow the view,
    /// for example in a zoom readout.
    #[must_use]
    pub fn view(&self) -> ReadSignal<ViewTransform> {
        self.view.read_only()
    }
    /// Shows the content with `view`, which is not bounded by the view's
    /// [`ScaleLimits`]. An equal transform changes nothing;
    /// [`Self::set_changed`] reports whether it changed.
    ///
    /// # Errors
    /// The errors of [`Signal::set`](crate::Signal::set): `Closed` after the
    /// runtime shut down; `WriteDuringUpdate`, `WriteDuringLayout`,
    /// `WriteDuringPaint`, `WriteDuringInspection` or `WriteDuringDelivery`
    /// from inside that phase; or `BorrowConflict`.
    pub fn set(&self, view: ViewTransform) -> Result<(), RuntimeError> {
        self.view.set(view)
    }
    /// Shows the content with `view` like [`Self::set`] and returns whether the
    /// transform changed.
    ///
    /// # Errors
    /// The errors of [`Self::set`].
    pub fn set_changed(&self, view: ViewTransform) -> Result<bool, RuntimeError> {
        self.view.set_changed(view)
    }
    /// Shows all content, centered with a 16-pixel margin, at the largest scale
    /// that fits but not above 100%, then held within the view's
    /// [`ScaleLimits`]: a smallest limit above 100% wins, and content still
    /// larger at the smallest scale stays centered. Uses the view's last
    /// arrangement; returns `false` without a change when the view has not been
    /// arranged yet, holds no content, or the fitted offset cannot be
    /// represented as a finite `f32`.
    ///
    /// # Errors
    /// Same as [`Self::set`].
    pub fn fit(&self) -> Result<bool, RuntimeError> {
        match self.arranged.get().and_then(fitted) {
            Some(view) => {
                self.view.set(view)?;
                Ok(true)
            }
            None => Ok(false),
        }
    }
}

impl core::fmt::Debug for ZoomController {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("ZoomController").finish_non_exhaustive()
    }
}

/// Space kept around fitted content, in the view's local logical pixels.
const FIT_MARGIN: f32 = 16.0;
/// Scroll distance, in the view's local pixels, that doubles or halves the
/// scale; one 40-pixel wheel notch is then about 13%.
const WHEEL_DOUBLING: f32 = 200.0;
/// Scale factor of one zoom key press.
const KEY_ZOOM_STEP: f32 = 1.25;
/// Pan distance of one arrow key press, in the view's local pixels.
const KEY_PAN_STEP: f32 = 40.0;
/// Grid lines closer than this many local pixels are not drawn.
const MIN_GRID_STEP: f32 = 8.0;

/// The fitted view for an arrangement, or `None` without content.
fn fitted(arranged: Arranged) -> Option<ViewTransform> {
    let [x, y, right, bottom] = arranged.content?;
    let (w, h) = (right - x, bottom - y);
    let viewport = arranged.viewport;
    let (vw, vh) = (f64::from(viewport.width()), f64::from(viewport.height()));
    let margin = f64::from(FIT_MARGIN);
    let room = |extent: f64, length: f64| {
        if length > 0.0 {
            ((extent - 2.0 * margin).max(1.0) / length).min(1.0)
        } else {
            1.0
        }
    };
    let scale = arranged.limits.clamp(room(vw, w).min(room(vh, h)) as f32);
    let s = f64::from(scale.get());
    // A view that cannot be represented, for content far beyond what any scale
    // brings near the window, leaves the view unchanged.
    let offset = Point::new(
        (vw / 2.0 - s * (x + w / 2.0)) as f32,
        (vh / 2.0 - s * (y + h / 2.0)) as f32,
    )
    .ok()?;
    Some(ViewTransform::new(scale, offset))
}

#[derive(Clone, Copy)]
struct Pan {
    button: PointerButton,
    start: Point,
    offset: Point,
}

/// A clipped surface that shows freely placed components at a zoom and pan the
/// user controls: a node canvas, a diagram or a map of modules.
///
/// Each item is an ordinary component placed at a content-space origin and
/// measured at its preferred size. The view presents them through one
/// [`ViewTransform`], so controls inside keep working at any zoom, text stays
/// sharp and pointer positions reach them in their own local space; see
/// [`Component::child_transform`]. Content may lie anywhere, including at
/// negative coordinates, and panning is not bounded by it.
///
/// Input, with opinionated defaults:
///
/// - **Pointer.** Dragging with the primary button where no item takes the
///   press, or with the middle button anywhere an item ignores it, pans. The
///   wheel pans, and zooms at the pointer with the platform's primary modifier;
///   [`Self::wheel`] swaps the two. A touchpad pinch
///   ([`PointerKind::Pinch`]) zooms at the pointer. A gesture zooms within [`ScaleLimits`],
///   1/8 to 8 unless [`Self::scale_limits`] says otherwise.
/// - **Keyboard,** while the view itself has focus, from a press on its
///   background or by Tab, and no Control, Alt or Command is held: arrows pan,
///   `=` and `-` zoom at the center, `0` shows 100% at the center (or the
///   nearest limit), and Home fits all content. Keys an item inside ignores do
///   not move the view. Escape ends a pan where it is.
///
/// A pan moves retained drawing and costs what scrolling costs. A zoom changes
/// the scale, which invalidates the layout of every component inside, because
/// text is prepared at the size it is shown; during a continuous zoom the
/// runtime stretches the prepared drawing instead and prepares it once the
/// scale rests, as [`Component::child_transform`] describes.
/// `docs/rendering-platform.md` records the measured cost. Focus moving to an item outside the view pans it into view
/// through [`Component::reveal`]. Surfaces an item opens, such as menus and
/// tooltips, are not scaled.
///
/// The view fills the space it is given, 640×400 logical pixels on an
/// unbounded axis, and draws nothing of its own except an optional
/// [grid](Self::grid) and its focus outline; wrap it in a
/// [`Container`](crate::Container) for a background. Its scale and offset are
/// workspace state with kind `zoom` and values `scale`, `x` and `y`. A
/// [`ZoomController`] reads and sets the view from application code.
///
/// Re-describing the view under the same key applies its items' origins,
/// scale limits, wheel gesture and grid; items keep their identity by key.
/// The shown transform is retained. A description with a different
/// controller adopts that controller's transform at the next update and ends
/// a pan in progress, releasing its capture, so a later move cannot write the
/// old gesture's offset into the new controller.
///
/// ```
/// # use pert_ui_runtime as pert_ui;
/// use pert_ui::*;
/// let mut ui = Runtime::new();
/// ui.set_viewport(Viewport::new(400.0, 300.0, 1.0)?)?;
/// let at = |x, y| Point::new(x, y);
/// let canvas = ZoomView::new([
///     (at(20.0, 20.0)?, Element::new("oscillator", Button::new("Oscillator", |_| Ok(())))),
///     (at(220.0, 120.0)?, Element::new("filter", Button::new("Filter", |_| Ok(())))),
/// ])
/// .grid(Length::new(40.0)?);
/// let root = ui.mount_tree(Element::new("canvas", canvas))?;
/// ui.flush(std::time::Duration::ZERO)?;
/// let filter = ui.children(&root)?[1].clone();
/// assert_eq!(ui.bounds(&filter)?.expect("placed").values()[..2], [220.0, 120.0]);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[must_use]
pub struct ZoomView {
    children: Vec<Element>,
    /// Content-space origins by child key, so a re-described item moves with its
    /// key rather than its position in the list.
    origins: BTreeMap<String, Point>,
    view: ViewTransform,
    /// The controller's value this view last adopted or wrote, so an
    /// application write is told apart from the view's own.
    seen: Option<ViewTransform>,
    controller: Option<ZoomController>,
    limits: ScaleLimits,
    wheel: WheelGesture,
    grid: Option<Length>,
    viewport: Cell<Size>,
    content: Cell<Option<[f64; 4]>>,
    pan: Option<Pan>,
    /// A pan ended by a controller replacement, whose capture the next
    /// update releases.
    retargeted: bool,
    focused: bool,
}
description_debug!(ZoomView);

impl ZoomView {
    /// A view of components placed at content-space origins. Keys must be
    /// distinct siblings, as for any container.
    pub fn new(items: impl IntoIterator<Item = (Point, Element)>) -> Self {
        let mut children = Vec::new();
        let mut origins = BTreeMap::new();
        for (origin, element) in items {
            origins.insert(element.key().to_owned(), origin);
            children.push(element);
        }
        Self {
            children,
            origins,
            view: ViewTransform::IDENTITY,
            seen: None,
            controller: None,
            limits: ScaleLimits::DEFAULT,
            wheel: WheelGesture::Pan,
            grid: None,
            viewport: Cell::new(Size::ZERO),
            content: Cell::new(None),
            pan: None,
            retargeted: false,
            focused: false,
        }
    }
    /// Bounds the scale that gestures reach; see [`ScaleLimits`].
    pub fn scale_limits(mut self, limits: ScaleLimits) -> Self {
        self.limits = limits;
        self
    }
    /// Selects what a plain wheel does; see [`WheelGesture`].
    pub fn wheel(mut self, gesture: WheelGesture) -> Self {
        self.wheel = gesture;
        self
    }
    /// Draws a grid of content-space squares of `spacing` behind the items, in
    /// [`ColorRole::Outline`] with one-physical-pixel lines. Lines that would be
    /// closer than eight logical pixels at the current scale are left out. Off by
    /// default; a zero spacing draws nothing.
    pub fn grid(mut self, spacing: Length) -> Self {
        self.grid = Some(spacing);
        self
    }
    /// Shares the view with application code; see [`ZoomController`]. The view
    /// starts at the controller's current transform.
    pub fn controller(mut self, controller: &ZoomController) -> Self {
        self.controller = Some(controller.clone());
        self
    }

    /// Shows `view`, publishing it to the controller. Called from events, where
    /// signal writes are allowed.
    fn show(&mut self, cx: &mut EventContext, view: ViewTransform) -> Result<(), RuntimeError> {
        if self.view == view {
            return Ok(());
        }
        if let Some(controller) = &self.controller {
            controller.view.set(view)?;
            self.seen = Some(view);
        }
        self.view = view;
        // Layout, not paint: the children's displayed rectangles move.
        cx.invalidate(Invalidation::Layout);
        Ok(())
    }
    fn zoom_at(
        &mut self,
        cx: &mut EventContext,
        factor: f32,
        anchor: Point,
    ) -> Result<(), RuntimeError> {
        let scale = self.limits.clamp(self.view.scale().get() * factor);
        match self.view.zoomed_at(scale, anchor) {
            Ok(view) => self.show(cx, view),
            // A far-off anchor at an extreme scale leaves the view where it is.
            Err(_) => Ok(()),
        }
    }
    fn pan_by(&mut self, cx: &mut EventContext, dx: f32, dy: f32) -> Result<(), RuntimeError> {
        let offset = self.view.offset();
        match Point::new(offset.x() + dx, offset.y() + dy) {
            Ok(offset) => self.show(cx, ViewTransform::new(self.view.scale(), offset)),
            Err(_) => Ok(()),
        }
    }
    fn center(&self) -> Point {
        let size = self.viewport.get();
        Point::new(size.width() / 2.0, size.height() / 2.0).unwrap_or_default()
    }
    fn arranged(&self) -> Arranged {
        Arranged {
            content: self.content.get(),
            viewport: self.viewport.get(),
            limits: self.limits,
        }
    }
}

impl Component for ZoomView {
    fn reconcile(&mut self, next: Self) -> Option<Invalidation> {
        let mut changed = self.origins != next.origins
            || self.limits != next.limits
            || self.grid.map(Length::get) != next.grid.map(Length::get);
        self.origins = next.origins;
        self.limits = next.limits;
        self.wheel = next.wheel;
        self.grid = next.grid;
        let same_controller = match (&self.controller, &next.controller) {
            (Some(a), Some(b)) => a.view.read_only().same_source(&b.view.read_only()),
            (None, None) => true,
            _ => false,
        };
        if !same_controller {
            // A new controller's transform is adopted at the next update. A
            // pan belongs to the old controller's transform: it ends here and
            // the update releases its capture, so later motion cannot write
            // an offset computed from the old gesture into the new controller.
            if self.pan.take().is_some() {
                self.retargeted = true;
            }
            // The old controller no longer serves this view, so a fit through
            // it must not use this view's arrangement.
            if let Some(previous) = &self.controller {
                previous.arranged.set(None);
            }
            self.controller = next.controller;
            self.seen = None;
            changed = true;
        }
        changed.then_some(Invalidation::Layout)
    }
    fn workspace_state(&self) -> Result<Option<ViewState>, WorkspaceError> {
        let offset = self.view.offset();
        let state = ViewState::new("zoom")?
            .with(
                "scale",
                WorkspaceValue::number(f64::from(self.view.scale().get()))?,
            )?
            .with("x", WorkspaceValue::number(f64::from(offset.x()))?)?
            .with("y", WorkspaceValue::number(f64::from(offset.y()))?)?;
        Ok(Some(state))
    }
    fn restore_workspace_state(
        &mut self,
        state: &ViewState,
    ) -> Result<Invalidation, RestoreRejection> {
        state.expect_only(&["scale", "x", "y"])?;
        if self.pan.is_some() {
            return Err(RestoreRejection::Busy);
        }
        let number = |name: &str| -> Result<f32, RestoreRejection> {
            let value = state.number(name)? as f32;
            if value.is_finite() {
                Ok(value)
            } else {
                Err(RestoreRejection::OutOfRange(name.to_owned()))
            }
        };
        let raw_scale = state.number("scale")?;
        if !(f64::from(ViewScale::MIN.get())..=f64::from(ViewScale::MAX.get())).contains(&raw_scale)
        {
            return Err(RestoreRejection::OutOfRange("scale".to_owned()));
        }
        let scale = ViewScale::new(number("scale")?)
            .map_err(|_| RestoreRejection::OutOfRange("scale".to_owned()))?;
        let offset = Point::new(number("x")?, number("y")?)
            .map_err(|_| RestoreRejection::OutOfRange("x".to_owned()))?;
        let view = ViewTransform::new(scale, offset);
        if let Some(controller) = &self.controller {
            controller
                .view
                .set(view)
                .map_err(RestoreRejection::Runtime)?;
            self.seen = Some(view);
        }
        self.view = view;
        Ok(Invalidation::Layout)
    }
    fn take_children(&mut self) -> Vec<Element> {
        std::mem::take(&mut self.children)
    }
    fn unmounted(&mut self) {
        // The controller may serve a later view; until then there is nothing
        // to fit.
        if let Some(controller) = &self.controller {
            controller.arranged.set(None);
        }
    }
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        if std::mem::take(&mut self.retargeted) {
            cx.cancel_input();
        }
        self.focused = cx.focused();
        if let Some(controller) = &self.controller {
            let view = cx.read(&controller.view)?;
            if self.seen != Some(view) {
                if self.pan.take().is_some() {
                    cx.cancel_input();
                }
                self.seen = Some(view);
                self.view = view;
            }
        }
        Ok(())
    }
    fn child_transform(&self) -> ViewTransform {
        self.view
    }
    fn focusable(&self) -> bool {
        true
    }
    fn can_reveal(&self) -> bool {
        true
    }
    fn measure(
        &self,
        _: &MeasureContext<'_>,
        available: Constraints,
    ) -> Result<Size, RuntimeError> {
        Ok(Size::new(
            available.width().unwrap_or(640.0),
            available.height().unwrap_or(400.0),
        )?)
    }
    fn arrange(&self, cx: &mut ArrangeContext<'_>, size: Size) -> Result<(), RuntimeError> {
        self.viewport.set(size);
        let mut bounds: Option<[f64; 4]> = None;
        for index in 0..cx.len() {
            let child = cx.measure(index, Constraints::default())?;
            let origin = self
                .origins
                .get(cx.measurement().key(index)?)
                .copied()
                .unwrap_or_default();
            let placed = rect(origin.x(), origin.y(), child.width(), child.height())?;
            let [x, y, w, h] = placed.values().map(f64::from);
            bounds = Some(match bounds {
                None => [x, y, x + w, y + h],
                Some([l, t, r, b]) => [l.min(x), t.min(y), r.max(x + w), b.max(y + h)],
            });
            cx.place(index, placed)?;
        }
        self.content.set(bounds);
        if let Some(controller) = &self.controller {
            controller.arranged.set(Some(self.arranged()));
        }
        Ok(())
    }
    fn cursor(&self, _: Point) -> Option<CursorIcon> {
        self.pan.is_some().then_some(CursorIcon::Grabbing)
    }
    fn pointer_input(
        &mut self,
        cx: &mut EventContext,
        event: PointerEvent,
    ) -> Result<InputOutcome, RuntimeError> {
        let Some(point) = cx.to_local(event.position()) else {
            return Ok(InputOutcome::Ignored);
        };
        match event.kind() {
            PointerKind::Scroll(delta) => {
                let modifiers = cx.modifiers();
                let zoom = (self.wheel == WheelGesture::Zoom) != modifiers.primary();
                if zoom {
                    if delta.y() == 0.0 {
                        return Ok(InputOutcome::Ignored);
                    }
                    self.zoom_at(cx, (-delta.y() / WHEEL_DOUBLING).exp2(), point)?;
                } else if modifiers.shift && delta.x() == 0.0 {
                    self.pan_by(cx, -delta.y(), 0.0)?;
                } else {
                    self.pan_by(cx, -delta.x(), -delta.y())?;
                }
            }
            PointerKind::Pinch(factor) => self.zoom_at(cx, factor.get(), point)?,
            PointerKind::Press(button @ (PointerButton::Primary | PointerButton::Middle))
                if self.pan.is_none() && cx.pointer_inside() && !event.is_outside() =>
            {
                cx.capture_pointer()?;
                cx.request_focus();
                self.pan = Some(Pan {
                    button,
                    start: point,
                    offset: self.view.offset(),
                });
                cx.invalidate(Invalidation::Paint);
            }
            PointerKind::Move => {
                let Some(pan) = self.pan else {
                    return Ok(InputOutcome::Ignored);
                };
                if let Ok(offset) = Point::new(
                    pan.offset.x() + point.x() - pan.start.x(),
                    pan.offset.y() + point.y() - pan.start.y(),
                ) {
                    self.show(cx, ViewTransform::new(self.view.scale(), offset))?;
                }
            }
            PointerKind::Release(button) if self.pan.is_some_and(|pan| pan.button == button) => {
                self.pan = None;
                cx.release_pointer();
                cx.invalidate(Invalidation::Paint);
            }
            _ => return Ok(InputOutcome::Ignored),
        }
        Ok(InputOutcome::Handled)
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
        if control == KeyCode::Escape && self.pan.take().is_some() {
            cx.release_pointer();
            cx.invalidate(Invalidation::Paint);
            return Ok(InputOutcome::Handled);
        }
        // Navigation keys belong to the view only while it has focus itself, so
        // a key a focused item ignores does not move the canvas, and a chord
        // with a modifier stays free for application commands.
        let modifiers = cx.modifiers();
        if !cx.focused() || modifiers.control || modifiers.alt || modifiers.super_key {
            return Ok(InputOutcome::Ignored);
        }
        let center = self.center();
        match control {
            KeyCode::ArrowLeft => self.pan_by(cx, KEY_PAN_STEP, 0.0)?,
            KeyCode::ArrowRight => self.pan_by(cx, -KEY_PAN_STEP, 0.0)?,
            KeyCode::ArrowUp => self.pan_by(cx, 0.0, KEY_PAN_STEP)?,
            KeyCode::ArrowDown => self.pan_by(cx, 0.0, -KEY_PAN_STEP)?,
            KeyCode::Equal => self.zoom_at(cx, KEY_ZOOM_STEP, center)?,
            KeyCode::Minus => self.zoom_at(cx, KEY_ZOOM_STEP.recip(), center)?,
            KeyCode::Digit0 => {
                if let Ok(view) = self.view.zoomed_at(self.limits.clamp(1.0), center) {
                    self.show(cx, view)?;
                }
            }
            KeyCode::Home => {
                if let Some(view) = fitted(self.arranged()) {
                    self.show(cx, view)?;
                }
            }
            _ => return Ok(InputOutcome::Ignored),
        }
        Ok(InputOutcome::Handled)
    }
    fn reveal(&mut self, cx: &mut EventContext, rect: Rect) -> Result<bool, RuntimeError> {
        // Compared in local pixels, where the offset is measured.
        let (Some(size), Some(rect)) = (cx.size(), cx.to_local_rect(rect)) else {
            return Ok(false);
        };
        let [rx, ry, rw, rh] = rect.values();
        let dx = scrollbar::reveal_delta(0.0, size.width(), rx, rw);
        let dy = scrollbar::reveal_delta(0.0, size.height(), ry, rh);
        if dx == 0.0 && dy == 0.0 {
            return Ok(false);
        }
        let before = self.view;
        self.pan_by(cx, -dx, -dy)?;
        Ok(self.view != before)
    }
    fn focus_gained(&mut self, cx: &mut EventContext) {
        self.focused = true;
        cx.invalidate(Invalidation::Paint);
    }
    fn focus_lost(&mut self, cx: &mut EventContext) -> Result<(), RuntimeError> {
        self.input_reset();
        cx.invalidate(Invalidation::Paint);
        Ok(())
    }
    fn cancel_input(&mut self, cx: &mut EventContext) {
        self.input_reset();
        self.focused = cx.focused();
        cx.invalidate(Invalidation::Paint);
    }
    fn input_reset(&mut self) {
        // A pan ends where it is, as a scrollbar drag does; nothing is restored.
        self.pan = None;
        self.focused = false;
    }
    fn visibility_changed(&mut self, visible: bool) {
        if !visible {
            self.focused = false;
        }
    }
    fn paint(&self, painter: &mut Painter) -> Result<(), DrawError> {
        let (Some(spacing), size) = (self.grid, painter.size()) else {
            return Ok(());
        };
        // The view draws in its own space, so it maps the content grid itself.
        let step = spacing.get() * self.view.scale().get();
        if step < MIN_GRID_STEP {
            return Ok(());
        }
        let line = painter.color(ColorRole::Outline);
        let width = 1.0 / painter.viewport().scale() as f32;
        let first = |offset: f32| offset.rem_euclid(step);
        let mut x = first(self.view.offset().x());
        while x < size.width() {
            painter.fill_rect(Rect::new(x, 0.0, width, size.height())?, line);
            x += step;
        }
        let mut y = first(self.view.offset().y());
        while y < size.height() {
            painter.fill_rect(Rect::new(0.0, y, size.width(), width)?, line);
            y += step;
        }
        Ok(())
    }
    fn paint_foreground(&self, painter: &mut Painter) -> Result<(), DrawError> {
        let (true, size) = (self.focused, painter.size()) else {
            return Ok(());
        };
        painter.focus_ring(Rect::from_size(size), Shape::Rectangle)
    }
    fn inspect(&self) -> ComponentDetails {
        let mut details = ComponentDetails::default();
        let offset = self.view.offset();
        for (name, value) in [
            ("view_scale", self.view.scale().get()),
            ("view_x", offset.x()),
            ("view_y", offset.y()),
        ] {
            details
                .properties
                .insert(name.into(), InspectValue::Number(f64::from(value)));
        }
        details.semantics = Some(
            Semantics::new(SemanticRole::ScrollArea).enabled_actions(Some(SemanticAction::Scroll)),
        );
        details
    }
}
