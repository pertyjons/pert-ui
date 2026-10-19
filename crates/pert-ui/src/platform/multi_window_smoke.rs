//! Real native surfaces with shared GPU and retained scheduling.
//!
//! The smoke opens an event loop, so it runs in its own process with `--exact`;
//! see docs/verification.md §Native evidence.
use super::*;
use crate::*;
use std::{cell::Cell, rc::Rc};
struct Animated {
    running: ReadSignal<bool>,
    updates: Rc<Cell<u64>>,
    phase: f32,
}
impl Component for Animated {
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        self.updates.set(self.updates.get() + 1);
        self.phase = (cx.now().as_secs_f32() % 1.0) * 100.0;
        if cx.read_paint(&self.running)? {
            cx.animate()?;
        }
        Ok(())
    }
    fn measure(&self, _: &MeasureContext<'_>, _: Constraints) -> Result<Size, RuntimeError> {
        Ok(Size::from([480, 120]))
    }
    fn paint(&self, p: &mut Painter) -> Result<(), DrawError> {
        p.fill_rect(Rect::new(self.phase, 20.0, 80.0, 80.0)?, Color::ACCENT);
        Ok(())
    }
}
struct Driver {
    runner: Runner,
    running: Signal<bool>,
    /// Invocations of the Space command.
    toggles: Rc<Cell<u32>>,
    close_decision: Signal<Option<bool>>,
    receipts: Vec<PresentationReceipt>,
    /// A retitle of the first detached window through the native host.
    title: Option<PresentationReceipt>,
    updates: Vec<Rc<Cell<u64>>>,
    phase: u8,
    baseline: Vec<u64>,
    result: Option<Result<(), AppError>>,
}
impl Driver {
    fn check(&mut self, event_loop: &ActiveEventLoop) -> Result<(), AppError> {
        let elapsed = self.runner.start.elapsed();
        if self.phase == 0 && elapsed >= Duration::from_secs(2) {
            let mut frames: Vec<_> = self
                .runner
                .parked
                .values()
                .map(|state| state.frames)
                .collect();
            frames.push(self.runner.current.frames);
            assert_eq!(frames.len(), 3, "three native resources");
            assert!(
                frames.iter().all(|frames| *frames > 2),
                "every surface was not presented: {frames:?} frames; {}",
                super::smoke::WITHHELD
            );
            for receipt in &self.receipts {
                assert!(matches!(receipt.status(), PresentationStatus::Presented(_)));
            }
            eprintln!(
                "Three native surfaces: {frames:?} presentations in {elapsed:?}; all share one GPU generation {}",
                self.runner.generation
            );
            assert_eq!(self.runner.generation, 1);
            // Native input routing requires selected root, not focus in every
            // root: Space in each unfocused window reaches the application
            // command, which toggles playback once per window.
            let started = Instant::now();
            let windows = self.runner.app.runtime.native_windows();
            assert_eq!(windows.len(), 3);
            for (index, window) in windows.iter().enumerate() {
                self.runner.app.runtime.select_window(window)?;
                self.runner.app.runtime.clear_focus();
                let before = self.running.get()?;
                let press = self
                    .runner
                    .app
                    .runtime
                    .command_key_input(CommandKeyInput::new(
                        KeyCode::Space,
                        KeyAction::Press,
                        CommandKey::Named(KeyCode::Space),
                    ))?;
                assert!(
                    press.handled && !press.deliver_text,
                    "window {index}: Space press was not routed to the command: {press:?}"
                );
                assert_eq!(press.event.transition, KeyTransition::Pressed);
                let release = self
                    .runner
                    .app
                    .runtime
                    .command_key_input(CommandKeyInput::new(
                        KeyCode::Space,
                        KeyAction::Release,
                        CommandKey::Named(KeyCode::Space),
                    ))?;
                assert_eq!(release.event.transition, KeyTransition::Released);
                assert_eq!(
                    self.running.get()?,
                    !before,
                    "window {index}: the command did not toggle exactly once"
                );
            }
            assert_eq!(self.toggles.get(), 3, "one toggle per window");
            eprintln!(
                "Three injected UI-thread command press/release pairs: {:?} (not physical OS latency)",
                started.elapsed()
            );
            self.running.set(false)?;
            self.phase = 1;
        } else if self.phase == 1 && elapsed >= Duration::from_secs(3) {
            self.runner
                .app
                .runtime
                .select_window(&self.runner.current.root)?;
            self.runner
                .recover_device("injected shared-device loss".into())?;
            assert_eq!(self.runner.generation, 2);
            self.runner.suspended(event_loop);
            assert!(self.runner.current.renderer.is_none());
            assert!(
                self.runner
                    .parked
                    .values()
                    .all(|state| state.renderer.is_none())
            );
            self.runner.resumed(event_loop);
            assert_eq!(self.runner.generation, 3);
            assert!(
                self.runner
                    .parked
                    .values()
                    .all(|state| state.renderer.is_some())
            );
            self.phase = 2;
        } else if self.phase == 2 && elapsed >= Duration::from_secs(4) {
            self.baseline = self.updates.iter().map(|value| value.get()).collect();
            let window = match self.receipts[0].status() {
                PresentationStatus::Presented(window) => window,
                status => panic!("the detach receipt is {status:?}, not presented"),
            };
            self.title = Some(
                self.runner
                    .app
                    .runtime
                    .request_window_title(&window, "pert-ui smoke retitled")?,
            );
            self.phase = 3;
        } else if self.phase == 3 && elapsed >= Duration::from_secs(5) {
            assert_eq!(
                self.baseline,
                self.updates
                    .iter()
                    .map(|value| value.get())
                    .collect::<Vec<_>>(),
                "idle roots do not tick"
            );
            assert!(self.runner.app.runtime.application_deadline().is_none());
            let retitled = self.title.as_ref().map(PresentationReceipt::status);
            assert!(
                matches!(retitled, Some(PresentationStatus::Presented(_))),
                "the native host applied the retitle: {retitled:?}"
            );
            eprintln!(
                "A detached window was retitled through the native host (winit cannot read titles back on X11/Wayland; check the title bar by eye)."
            );
            self.receipts.push(
                self.runner
                    .app
                    .runtime
                    .request_close(&self.runner.app.runtime.primary_window())?,
            );
            self.phase = 4;
        } else if self.phase == 4 && elapsed >= Duration::from_secs(6) {
            assert_eq!(
                self.runner.app.runtime.native_windows().len(),
                3,
                "deferred main close preserves every window"
            );
            assert_eq!(self.receipts[2].status(), PresentationStatus::Pending);
            self.close_decision.set(Some(false))?;
            let window = match self.receipts[0].status() {
                PresentationStatus::Presented(window) => window,
                status => panic!("the detach receipt is {status:?}, not presented"),
            };
            self.receipts
                .push(self.runner.app.runtime.request_close(&window)?);
            // A second close queued before the first is processed finds the
            // window gone: a failed request, not a component failure.
            self.receipts
                .push(self.runner.app.runtime.request_close(&window)?);
            self.phase = 5;
        } else if self.phase == 5 && elapsed >= Duration::from_secs(7) {
            assert_eq!(self.runner.app.runtime.native_windows().len(), 2);
            assert_eq!(self.runner.parked.len(), 1);
            assert_eq!(self.receipts[2].status(), PresentationStatus::CloseRejected);
            assert_eq!(self.receipts[3].status(), PresentationStatus::Closed);
            assert_eq!(
                self.receipts[4].status(),
                PresentationStatus::Failed(crate::WindowError::Runtime(RuntimeError::Unmounted))
            );
            assert!(
                self.runner.error.is_none(),
                "the stale close did not enter error recovery"
            );
            eprintln!(
                "Shared recovery, all-window suspend/resume, idle, deferred/rejected main close, detached close/re-dock and a stale queued close passed."
            );
            event_loop.exit();
            self.result = Some(Ok(()));
        }
        Ok(())
    }
}
impl ApplicationHandler<UserEvent> for Driver {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        self.runner.resumed(event_loop);
    }
    fn window_event(&mut self, event_loop: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
        self.runner.window_event(event_loop, id, event);
    }
    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: UserEvent) {
        self.runner.user_event(event_loop, event);
    }
    fn exiting(&mut self, event_loop: &ActiveEventLoop) {
        self.runner.exiting(event_loop);
    }
    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        self.runner.about_to_wait(event_loop);
        if let Err(error) = self.check(event_loop) {
            self.result = Some(Err(error));
            event_loop.exit();
        }
        if self.runner.error.is_some() {
            event_loop.exit();
        }
        let deadline = self.runner.start
            + Duration::from_secs(match self.phase {
                0 => 2,
                1 => 3,
                2 => 4,
                3 => 5,
                4 => 6,
                _ => 7,
            });
        if let ControlFlow::WaitUntil(existing) = event_loop.control_flow() {
            event_loop.set_control_flow(ControlFlow::WaitUntil(existing.min(deadline)));
        } else {
            event_loop.set_control_flow(ControlFlow::WaitUntil(deadline));
        }
    }
}
#[test]
#[ignore = "requires a native desktop session and hardware GPU adapter"]
fn three_native_surfaces_animate_settle_and_redock() -> Result<(), AppError> {
    three_surfaces(false)
}

/// The same run with software presentation: every native window gets its own
/// softbuffer surface, and detach, close and redock keep working.
#[cfg(feature = "cpu")]
#[test]
#[ignore = "requires a native desktop session"]
fn three_software_surfaces_animate_settle_and_redock() -> Result<(), AppError> {
    three_surfaces(true)
}

fn three_surfaces(software: bool) -> Result<(), AppError> {
    let mut app = App::new("pert-ui multi-window smoke", |_| Ok(()));
    app.software = software;
    app.initialize.take();
    let close_decision = app.runtime.signal(None);
    let decision = close_decision.read_only();
    app = app.on_close(move |ui, window| {
        Ok(if *window == ui.primary_window() {
            NativeCloseAction::Defer(decision.clone())
        } else {
            NativeCloseAction::Close
        })
    });
    let running = app.runtime.signal(true);
    let mut updates = Vec::new();
    let mut receipts = Vec::new();
    let state = running.clone();
    let toggles = Rc::new(Cell::new(0));
    let counted = toggles.clone();
    let _command = app.runtime.register_command(
        "Play",
        CommandScope::Application,
        vec![KeyChord::new(
            CommandKey::Named(KeyCode::Space),
            Modifiers::default(),
        )],
        move |_, _| {
            counted.set(counted.get() + 1);
            state.set(!state.get()?)
        },
    )?;
    let mut elements = Vec::new();
    for index in 0..3 {
        let count = Rc::new(Cell::new(0));
        updates.push(count.clone());
        elements.push(Element::new(
            format!("view-{index}"),
            Animated {
                running: running.read_only(),
                updates: count,
                phase: 0.0,
            },
        ));
    }
    let root = app
        .runtime
        .mount_tree(Element::new("layout", Column::new(elements)))?;
    for (index, view) in app.runtime.children(&root)?.into_iter().enumerate().skip(1) {
        receipts.push(app.runtime.request_detach(
            &view,
            NativeWindowOptions::new(format!("pert-ui smoke {index}"))?,
        )?);
    }
    let event_loop = native_test_event_loop()?;
    let runner = Runner::new(app, event_loop.create_proxy());
    let mut driver = Driver {
        runner,
        running,
        toggles,
        close_decision,
        receipts,
        title: None,
        updates,
        phase: 0,
        baseline: Vec::new(),
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
        .ok_or_else(|| AppError::Graphics("native smoke exited before verification".into()))?
}
