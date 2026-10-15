//! A drag session driven by winit pointer events through the native adapter.
//!
//! The smoke opens an event loop, so it runs in its own process with `--exact`;
//! see docs/verification.md §Native evidence and docs/contracts.md §Input and
//! focus. The window, its compositor surface and the renderer are real;
//! the pointer is not. The check feeds constructed `WindowEvent`s (cursor
//! motion, button press and release, leaving the window) to the same
//! `window_event` entry point the operating system's events reach, and holds
//! back the real pointer and keyboard events of the session while it runs, so a
//! pointer resting over the window cannot interfere. It is therefore evidence
//! for the adapter's translation, its cursor choice, the preview's placement in
//! the presented frame and the detach that follows an unclaimed release, not
//! for what a window system delivers when a person drags.
use super::*;
use crate::*;
use std::{cell::RefCell, rc::Rc};

/// How long one step may wait for its frame before the check fails.
const STEP_DEADLINE: Duration = Duration::from_secs(5);
/// Where the preview sits relative to the pointer, in logical pixels.
const HOT_SPOT: [f32; 2] = [5.0, 5.0];
/// The preview's logical size.
const CHIP: [u16; 2] = [40, 24];
/// The preview's color, found in the rendered frame: opaque magenta, which
/// nothing else in the window draws.
fn chip_color() -> Result<Color, DrawError> {
    Color::rgba(1.0, 0.0, 1.0, 1.0)
}

/// The payload the zones recognize; the tab strip's own payload is private.
struct Item;

/// Starts a session of `Item`, with an element preview, once an armed press
/// has travelled `DRAG_THRESHOLD`, and records how each session ended.
struct Source {
    armed: Option<Point>,
    outcomes: Rc<RefCell<Vec<DragOutcome>>>,
}
impl Component for Source {
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
    fn measure(&self, _: &MeasureContext<'_>, _: Constraints) -> Result<Size, RuntimeError> {
        Ok(Size::from([120, 80]))
    }
    fn paint(&self, p: &mut Painter) -> Result<(), DrawError> {
        p.fill_rect(Rect::new(0.0, 0.0, 120.0, 80.0)?, Color::ACCENT);
        Ok(())
    }
    fn pointer_input(
        &mut self,
        cx: &mut EventContext,
        event: PointerEvent,
    ) -> Result<InputOutcome, RuntimeError> {
        match event.kind() {
            PointerKind::Press(PointerButton::Primary) => {
                self.armed = Some(event.position());
                cx.capture_pointer()?;
            }
            PointerKind::Move => {
                if let Some(start) = self.armed
                    && (event.position().x() - start.x()).hypot(event.position().y() - start.y())
                        >= DRAG_THRESHOLD
                {
                    self.armed = None;
                    cx.start_drag(
                        DragPayload::new(Item),
                        DropOperations::MOVE | DropOperations::COPY | DropOperations::LINK,
                        DragPreview::element(Element::new("chip", Chip))
                            .hot_spot(Point::new(HOT_SPOT[0], HOT_SPOT[1])?),
                    )?;
                }
            }
            PointerKind::Release(_) => self.armed = None,
            _ => return Ok(InputOutcome::Ignored),
        }
        Ok(InputOutcome::Handled)
    }
    fn drag_finished(
        &mut self,
        _: &mut EventContext,
        outcome: DragOutcome,
    ) -> Result<(), RuntimeError> {
        self.outcomes.borrow_mut().push(outcome);
        Ok(())
    }
}

/// The preview: a solid chip whose pixels the check looks for.
struct Chip;
impl Component for Chip {
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
    fn measure(&self, _: &MeasureContext<'_>, _: Constraints) -> Result<Size, RuntimeError> {
        Ok(Size::from(CHIP))
    }
    fn paint(&self, p: &mut Painter) -> Result<(), DrawError> {
        p.fill_rect(
            Rect::new(0.0, 0.0, f32::from(CHIP[0]), f32::from(CHIP[1]))?,
            chip_color()?,
        );
        Ok(())
    }
}

/// A target that answers every drag of `Item` the same way and rejects
/// anything else, such as the tab strip's payload.
struct Zone(DropResponse);
impl Component for Zone {
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
    fn measure(&self, _: &MeasureContext<'_>, _: Constraints) -> Result<Size, RuntimeError> {
        Ok(Size::from([120, 80]))
    }
    fn paint(&self, p: &mut Painter) -> Result<(), DrawError> {
        p.fill_rect(
            Rect::new(4.0, 4.0, 112.0, 72.0)?,
            Color::rgba(0.2, 0.2, 0.2, 1.0)?,
        );
        Ok(())
    }
    fn drag_event(
        &mut self,
        _: &mut EventContext,
        event: &DragEvent<'_>,
    ) -> Result<DropResponse, RuntimeError> {
        Ok(if event.payload().get::<Item>().is_some() {
            self.0
        } else {
            DropResponse::Rejected
        })
    }
}

/// What the check waits for before its next step.
enum Await {
    Nothing,
    /// A main-window presentation after this count.
    Frame(u64),
    /// The detached tab's window and its first presentation.
    Detached,
}

struct Driver {
    runner: Runner,
    outcomes: Rc<RefCell<Vec<DragOutcome>>>,
    step: u8,
    waiting: Await,
    deadline: Instant,
    /// The first tab's view, which the drag-out detaches.
    tab: WidgetId,
    /// The compositor's scale for the main window, recorded at the first frame.
    native_scale: f64,
    /// The physical pointer position of the fractional-scale step.
    pointer: [f64; 2],
    result: Option<Result<(), AppError>>,
}
impl Driver {
    fn error(message: impl Into<String>) -> AppError {
        AppError::Graphics(format!("drag smoke: {}", message.into()))
    }
    fn ensure(condition: bool, message: impl FnOnce() -> String) -> Result<(), AppError> {
        if condition {
            Ok(())
        } else {
            Err(Self::error(message()))
        }
    }
    /// The main window's adapter state, wherever the runner keeps it now.
    fn main(&self) -> Result<&NativeState, AppError> {
        let primary = self.runner.app.runtime.primary_window();
        if self.runner.current.root == primary {
            return Ok(&self.runner.current);
        }
        self.runner
            .parked
            .values()
            .find(|state| state.root == primary)
            .ok_or_else(|| Self::error("the main window's state is gone"))
    }
    fn main_id(&self) -> Result<WindowId, AppError> {
        self.main()?
            .window
            .as_ref()
            .map(|window| window.id())
            .ok_or_else(|| Self::error("the main window is not open"))
    }
    fn scale(&self) -> f64 {
        self.runner.app.runtime.viewport().scale()
    }
    /// Feeds one constructed event to the adapter's native entry point and
    /// surfaces the failure it recorded, if any.
    fn inject(&mut self, event_loop: &ActiveEventLoop, event: WindowEvent) -> Result<(), AppError> {
        let id = self.main_id()?;
        self.runner.window_event(event_loop, id, event);
        match self.runner.error.take() {
            Some(error) => Err(error),
            None => Ok(()),
        }
    }
    /// Moves the pointer to a physical position in the main window.
    fn move_to(&mut self, event_loop: &ActiveEventLoop, x: f64, y: f64) -> Result<(), AppError> {
        self.inject(
            event_loop,
            WindowEvent::CursorMoved {
                device_id: winit::event::DeviceId::dummy(),
                position: winit::dpi::PhysicalPosition::new(x, y),
            },
        )
    }
    /// Moves the pointer to a window-logical point, converted at the current scale.
    fn move_logical(
        &mut self,
        event_loop: &ActiveEventLoop,
        point: [f32; 2],
    ) -> Result<(), AppError> {
        let scale = self.scale();
        self.move_to(
            event_loop,
            f64::from(point[0]) * scale,
            f64::from(point[1]) * scale,
        )
    }
    /// Presses on the source and crosses the drag threshold over it.
    fn start_drag(&mut self, event_loop: &ActiveEventLoop) -> Result<(), AppError> {
        let [x, y] = self.center("source")?;
        self.move_logical(event_loop, [x, y])?;
        self.button(event_loop, ElementState::Pressed)?;
        self.move_logical(event_loop, [x + 2.0 * DRAG_THRESHOLD, y])
    }
    fn button(
        &mut self,
        event_loop: &ActiveEventLoop,
        state: ElementState,
    ) -> Result<(), AppError> {
        self.inject(
            event_loop,
            WindowEvent::MouseInput {
                device_id: winit::event::DeviceId::dummy(),
                state,
                button: winit::event::MouseButton::Left,
            },
        )
    }
    /// The center of the component keyed `key`, in window-logical pixels.
    fn center(&self, key: &str) -> Result<[f32; 2], AppError> {
        let [x, y, width, height] = self.border_box(key)?;
        Ok([x + width / 2.0, y + height / 2.0])
    }
    fn border_box(&self, key: &str) -> Result<[f32; 4], AppError> {
        self.runner
            .app
            .runtime
            .inspect()
            .components
            .iter()
            .find(|entry| entry.key == key)
            .and_then(|entry| entry.details.box_model.border_box)
            .ok_or_else(|| Self::error(format!("no laid-out component keyed {key}")))
    }
    fn await_frame(&mut self) -> Result<(), AppError> {
        self.waiting = Await::Frame(self.main()?.frames);
        self.deadline = Instant::now() + STEP_DEADLINE;
        Ok(())
    }
    /// Both what the runtime asks for and what the adapter last handed the
    /// window system are `expected`.
    fn check_cursor(
        &self,
        expected: CursorIcon,
        native: winit::window::CursorIcon,
    ) -> Result<(), AppError> {
        let runtime = self.runner.app.runtime.cursor_icon();
        let shown = self.main()?.cursor;
        Self::ensure(runtime == expected && shown == Some(expected), || {
            format!(
                "expected cursor {expected:?}; the runtime asks for {runtime:?}, the adapter shows {shown:?}"
            )
        })?;
        Self::ensure(native_cursor(expected) == native, || {
            format!(
                "{expected:?} maps to {:?}, not {native:?}",
                native_cursor(expected)
            )
        })?;
        eprintln!("Cursor {expected:?}: the adapter set winit {native:?}");
        Ok(())
    }
    /// The preview sits at the pointer less the hot spot in logical pixels,
    /// and the frame the window's renderer produces carries its color at the
    /// matching physical pixels, but not left of and above its corner.
    fn check_preview(&mut self, physical: [f64; 2]) -> Result<(), AppError> {
        let scale = self.scale();
        let drag = self
            .runner
            .app
            .runtime
            .inspect()
            .drag
            .ok_or_else(|| Self::error("no session"))?;
        let preview = drag.preview.ok_or_else(|| Self::error("no preview root"))?;
        let bounds = self
            .runner
            .app
            .runtime
            .inspect()
            .components
            .iter()
            .find(|entry| entry.id == preview)
            .and_then(|entry| entry.details.box_model.border_box)
            .ok_or_else(|| Self::error("the preview is not laid out"))?;
        let expected = [
            (physical[0] / scale) as f32 - HOT_SPOT[0],
            (physical[1] / scale) as f32 - HOT_SPOT[1],
        ];
        Self::ensure(
            (bounds[0] - expected[0]).abs() < 0.01 && (bounds[1] - expected[1]).abs() < 0.01,
            || format!("preview at {bounds:?}, expected its origin at {expected:?}"),
        )?;
        let scene = self.runner.app.runtime.paint().map_err(AppError::from)?;
        let renderer = self
            .runner
            .current
            .renderer
            .as_mut()
            .ok_or_else(|| Self::error("no renderer"))?;
        let image = renderer
            .render_offscreen(scene)
            .map_err(AppError::from_renderer)?;
        let pixel = |x: f64, y: f64| -> Result<[u8; 4], AppError> {
            let (x, y) = (x.round() as u32, y.round() as u32);
            Self::ensure(x < image.width() && y < image.height(), || {
                format!(
                    "pixel ({x}, {y}) outside the {}×{} frame",
                    image.width(),
                    image.height()
                )
            })?;
            let start = ((y * image.width() + x) * 4) as usize;
            let mut rgba = [0; 4];
            rgba.copy_from_slice(&image.pixels()[start..start + 4]);
            Ok(rgba)
        };
        let magenta = |rgba: [u8; 4]| rgba[0] >= 250 && rgba[1] <= 5 && rgba[2] >= 250;
        // Inside the chip, and just outside its top-left corner.
        let inside = [physical[0] + 10.0 * scale, physical[1] + 8.0 * scale];
        let outside = [
            physical[0] - (f64::from(HOT_SPOT[0]) + 3.0) * scale,
            physical[1] - (f64::from(HOT_SPOT[1]) + 3.0) * scale,
        ];
        let (inner, outer) = (pixel(inside[0], inside[1])?, pixel(outside[0], outside[1])?);
        Self::ensure(magenta(inner) && !magenta(outer), || {
            format!(
                "rendered preview: {inner:?} inside at {inside:?}, {outer:?} outside at {outside:?}"
            )
        })?;
        eprintln!(
            "Preview at scale {scale}: pointer at physical {physical:?}, preview origin logical {:?}, \
             rendered {}×{} frame has {inner:?} at {inside:?} and {outer:?} at {outside:?}",
            [bounds[0], bounds[1]],
            image.width(),
            image.height()
        );
        Ok(())
    }
    fn outcome(&self) -> Option<DragOutcome> {
        self.outcomes.borrow().last().copied()
    }
    /// Fails a step whose frame or window has not arrived by its deadline.
    fn overdue(&self) -> Result<(), AppError> {
        Self::ensure(Instant::now() < self.deadline, || {
            let state = &self.runner.current;
            format!(
                "step {} waited {STEP_DEADLINE:?} in vain (redraw pending {}, dirty {}, \
                 paint needed {}, presentable {}); {}",
                self.step,
                state.redraw_pending,
                state.dirty,
                self.runner.app.runtime.needs_paint(),
                state.presentable(),
                super::smoke::WITHHELD
            )
        })
    }
    /// Whether the awaited frame or window has arrived; checks the deadline
    /// while it has not.
    fn arrived(&mut self) -> Result<bool, AppError> {
        let arrived = match self.waiting {
            Await::Nothing => true,
            Await::Frame(frames) => self.main()?.frames > frames,
            Await::Detached => {
                let primary = self.runner.app.runtime.primary_window();
                self.runner.app.runtime.native_windows().len() == 2
                    && self
                        .runner
                        .parked
                        .values()
                        .chain([&self.runner.current])
                        .any(|state| state.root != primary && state.frames > 0)
            }
        };
        if arrived {
            self.waiting = Await::Nothing;
        } else {
            self.overdue()?;
        }
        Ok(arrived)
    }
    /// One step of the script. Each step checks what the frame awaited by
    /// the previous one showed, then injects input and awaits the next.
    fn advance(&mut self, event_loop: &ActiveEventLoop) -> Result<(), AppError> {
        if !self.arrived()? {
            return Ok(());
        }
        match self.step {
            0 => {
                let Some(window) = self.main()?.window.clone() else {
                    return self.overdue();
                };
                if self.main()?.frames == 0 {
                    return self.overdue();
                }
                self.native_scale = window.scale_factor();
                eprintln!(
                    "Native window: scale factor {}, inner size {:?} physical, monitor {:?}",
                    self.native_scale,
                    window.inner_size(),
                    window.current_monitor().and_then(|monitor| monitor.name()),
                );
                self.start_drag(event_loop)?;
            }
            1 => {
                let drag = self.runner.app.runtime.inspect().drag;
                Self::ensure(
                    drag.as_ref().is_some_and(|drag| drag.target.is_none()),
                    || format!("no session without a target after the threshold: {drag:?}"),
                )?;
                self.check_cursor(CursorIcon::Grabbing, winit::window::CursorIcon::Grabbing)?;
                let center = self.center("copy")?;
                self.move_logical(event_loop, center)?;
            }
            2 => {
                self.check_cursor(CursorIcon::Copy, winit::window::CursorIcon::Copy)?;
                let scale = self.scale();
                let [x, y] = self.center("copy")?;
                self.check_preview([f64::from(x) * scale, f64::from(y) * scale])?;
                let center = self.center("link")?;
                self.move_logical(event_loop, center)?;
            }
            3 => {
                self.check_cursor(CursorIcon::Alias, winit::window::CursorIcon::Alias)?;
                let center = self.center("forbid")?;
                self.move_logical(event_loop, center)?;
            }
            4 => {
                self.check_cursor(CursorIcon::NoDrop, winit::window::CursorIcon::NoDrop)?;
                self.button(event_loop, ElementState::Released)?;
                Self::ensure(!self.runner.app.runtime.drag_active(), || {
                    "a release over the forbidding target did not end the session".into()
                })?;
                // A fractional scale the compositor did not choose, applied to
                // the runtime's viewport over the same physical surface: the
                // adapter converts pointer positions with it and the renderer
                // draws at it, as they would for a compositor scale. A scale
                // change cancels a running session like any other gesture, so
                // a new one starts at the new scale.
                let fractional = if (self.native_scale - 1.25).abs() < 1e-6 {
                    1.5
                } else {
                    1.25
                };
                let size = self.main()?.size;
                self.runner
                    .app
                    .runtime
                    .set_viewport(Viewport::from_physical(
                        PhysicalExtent::new(size.width, size.height),
                        fractional,
                    )?)?;
                self.runner.app.runtime.flush(self.runner.start.elapsed())?;
                self.start_drag(event_loop)?;
            }
            5 => {
                self.check_cursor(CursorIcon::Grabbing, winit::window::CursorIcon::Grabbing)?;
                // A physical position that is not a whole logical one.
                let scale = self.scale();
                let [x, y] = self.center("copy")?;
                self.pointer = [
                    (f64::from(x) * scale).floor() + 0.5,
                    (f64::from(y) * scale).floor() + 0.5,
                ];
                self.move_to(event_loop, self.pointer[0], self.pointer[1])?;
            }
            6 => {
                self.check_cursor(CursorIcon::Copy, winit::window::CursorIcon::Copy)?;
                self.check_preview(self.pointer)?;
                self.button(event_loop, ElementState::Released)?;
                Self::ensure(!self.runner.app.runtime.drag_active(), || {
                    "a release over the copy target did not end the session".into()
                })?;
                // Back to the compositor's scale, and a new session there.
                let size = self
                    .main()?
                    .window
                    .as_ref()
                    .map(|window| window.inner_size())
                    .ok_or_else(|| Self::error("no window"))?;
                self.runner.resize(size)?;
                self.start_drag(event_loop)?;
            }
            7 => {
                self.check_cursor(CursorIcon::Grabbing, winit::window::CursorIcon::Grabbing)?;
                // Out of the window with the button held, as motion beyond
                // its edge without a leave.
                self.move_to(event_loop, -20.0, -20.0)?;
            }
            8 => {
                let drag = self.runner.app.runtime.inspect().drag;
                Self::ensure(
                    drag.as_ref().is_some_and(|drag| drag.target.is_none()),
                    || format!("outside the window the session has a target: {drag:?}"),
                )?;
                self.check_cursor(CursorIcon::Grabbing, winit::window::CursorIcon::Grabbing)?;
                self.button(event_loop, ElementState::Released)?;
                // The source is told within the release's own dispatch.
                Self::ensure(
                    self.outcome() == Some(DragOutcome::Unclaimed)
                        && !self.runner.app.runtime.drag_active(),
                    || format!("a release outside ended as {:?}", self.outcome()),
                )?;
                eprintln!(
                    "Release beyond the window edge without a leave: {:?}",
                    self.outcome()
                );
            }
            9 => {
                self.check_cursor(CursorIcon::Default, winit::window::CursorIcon::Default)?;
                // Drag a detachable tab out of the window: past the strip,
                // beyond the top edge, a leave, then the release outside.
                let [x, y, _, _] = self.border_box("tabs")?;
                self.move_logical(event_loop, [x + 20.0, y + 15.0])?;
                self.button(event_loop, ElementState::Pressed)?;
                let center = self.center("forbid")?;
                self.move_logical(event_loop, center)?;
            }
            10 => {
                let snapshot = self.runner.app.runtime.inspect();
                let strip = snapshot
                    .components
                    .iter()
                    .find(|entry| entry.key == "tabs")
                    .map(|entry| entry.id);
                let drag = snapshot.drag;
                Self::ensure(
                    drag.as_ref()
                        .is_some_and(|drag| Some(drag.source) == strip && drag.target.is_none()),
                    || format!("no tab session from the strip: {drag:?}"),
                )?;
                self.check_cursor(CursorIcon::Grabbing, winit::window::CursorIcon::Grabbing)?;
                let scale = self.scale();
                self.move_to(event_loop, 60.0 * scale, -40.0 * scale)?;
                self.inject(
                    event_loop,
                    WindowEvent::CursorLeft {
                        device_id: winit::event::DeviceId::dummy(),
                    },
                )?;
                Self::ensure(self.runner.app.runtime.drag_active(), || {
                    "leaving the window with the button held ended the session".into()
                })?;
                self.button(event_loop, ElementState::Released)?;
                Self::ensure(!self.runner.app.runtime.drag_active(), || {
                    "a release after the leave did not end the session".into()
                })?;
                self.waiting = Await::Detached;
                self.deadline = Instant::now() + STEP_DEADLINE;
                self.step += 1;
                return Ok(());
            }
            11 => {
                let windows = self.runner.app.runtime.native_windows().len();
                let window = self.runner.app.runtime.window_of(&self.tab)?;
                Self::ensure(window != self.runner.app.runtime.primary_window(), || {
                    "the dragged tab stayed in the main window".into()
                })?;
                eprintln!(
                    "Leave then release outside: the tab detached into a presented native \
                     window ({windows} native windows)"
                );
                self.result = Some(Ok(()));
                event_loop.exit();
                return Ok(());
            }
            _ => return Ok(()),
        }
        self.step += 1;
        self.await_frame()
    }
}
impl ApplicationHandler<UserEvent> for Driver {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        self.runner.resumed(event_loop);
    }
    fn window_event(&mut self, event_loop: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
        // The session's own pointer and keyboard are held back: only the
        // injected input drives the check.
        if matches!(
            event,
            WindowEvent::CursorMoved { .. }
                | WindowEvent::CursorEntered { .. }
                | WindowEvent::CursorLeft { .. }
                | WindowEvent::MouseInput { .. }
                | WindowEvent::MouseWheel { .. }
                | WindowEvent::PinchGesture { .. }
                | WindowEvent::KeyboardInput { .. }
                | WindowEvent::ModifiersChanged(_)
                | WindowEvent::Ime(_)
        ) {
            return;
        }
        self.runner.window_event(event_loop, id, event);
    }
    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: UserEvent) {
        self.runner.user_event(event_loop, event);
    }
    fn exiting(&mut self, event_loop: &ActiveEventLoop) {
        self.runner.exiting(event_loop);
    }
    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if self.result.is_none()
            && self.runner.error.is_none()
            && let Err(error) = self.advance(event_loop)
        {
            self.result = Some(Err(error));
            event_loop.exit();
            return;
        }
        self.runner.about_to_wait(event_loop);
        if self.runner.error.is_some() || self.result.is_some() {
            event_loop.exit();
            return;
        }
        // Look at the script again soon; the runner's own deadline stands
        // when it is earlier.
        let soon = Instant::now() + Duration::from_millis(20);
        let next = match event_loop.control_flow() {
            ControlFlow::WaitUntil(time) => time.min(soon),
            _ => soon,
        };
        event_loop.set_control_flow(ControlFlow::WaitUntil(next));
    }
}
#[test]
#[ignore = "requires a native desktop session and hardware GPU adapter"]
fn injected_pointer_drives_a_drag_session_through_the_native_adapter() -> Result<(), AppError> {
    let mut app = App::new("pert-ui drag smoke", |_| Ok(()));
    app.initialize.take();
    let outcomes = Rc::new(RefCell::new(Vec::new()));
    let tab = |key: &str| {
        Tab::new(
            key,
            format!("Tab {key}"),
            Element::new("body", Label::new("Content")),
        )
        .into_element()
    };
    let zone = |key: &str, answer| Element::new(key, Zone(answer));
    let root = app.runtime.mount_tree(Element::new(
        "page",
        Column::new([
            Element::new("tabs", Tabs::new([]).detachable(true))
                .children([tab("first"), tab("second")]),
            Element::new(
                "zones",
                Row::new([
                    Element::new(
                        "source",
                        Source {
                            armed: None,
                            outcomes: outcomes.clone(),
                        },
                    ),
                    zone("copy", DropResponse::Accepted(DropOperation::Copy)),
                    zone("link", DropResponse::Accepted(DropOperation::Link)),
                    zone("forbid", DropResponse::Forbidden),
                ])
                .gap(Length::new(0.0)?),
            ),
        ]),
    ))?;
    let tabs = app
        .runtime
        .find_descendant(&root, "tabs")?
        .ok_or(RuntimeError::Unmounted)?;
    let tab = app
        .runtime
        .children(&tabs)?
        .into_iter()
        .next()
        .ok_or(RuntimeError::Unmounted)?;
    let event_loop = native_test_event_loop()?;
    let runner = Runner::new(app, event_loop.create_proxy());
    let mut driver = Driver {
        runner,
        outcomes,
        tab,
        step: 0,
        waiting: Await::Nothing,
        deadline: Instant::now() + Duration::from_secs(20),
        native_scale: 1.0,
        pointer: [0.0; 2],
        result: None,
    };
    event_loop
        .run_app(&mut driver)
        .map_err(|error| AppError::EventLoop(error.to_string()))?;
    if let Some(error) = driver.runner.error {
        return Err(error);
    }
    driver
        .result
        .ok_or_else(|| AppError::Graphics("drag smoke exited before verification".into()))?
}
