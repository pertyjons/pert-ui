//! Native event translation and lifecycle. No application-domain actions.
#[cfg(test)]
mod capture_smoke;
#[cfg(test)]
mod drag_smoke;
#[cfg(test)]
mod font_smoke;
#[cfg(test)]
mod inline_smoke;
#[cfg(test)]
mod multi_window_smoke;
// Pacing at the monitor's rate is a property of GPU presentation: CPU frames
// carry no interactive frame budget (docs/verification.md, CPU rendering cost).
#[cfg(all(test, feature = "gpu"))]
mod pacing_smoke;
#[cfg(test)]
mod smoke;
#[cfg(test)]
mod transient_smoke;
#[cfg(test)]
mod window_smoke;
#[cfg(test)]
use smoke::{Smoke, native_test_event_loop};

use crate::{App, AppError, KeyAction, PhysicalExtent, RuntimeError, Viewport, WidgetId};
use presenter::Presenter;
use presenter::{Frame, Notice};
use std::{
    sync::Arc,
    time::{Duration, Instant},
};
use winit::{
    application::ApplicationHandler,
    dpi::{LogicalSize, PhysicalSize},
    event::{ElementState, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop, EventLoopProxy},
    window::{Window, WindowId},
};
mod clipboard;
mod ime;
mod keyboard;
mod motion;
pub(crate) mod presenter;
mod schedule;
mod windows;
use schedule::{ComponentRecovery, DeviceRecovery, SurfaceRetries};
use windows::PendingClose;

struct GpuEvent {
    generation: u64,
    notice: Notice,
}

/// What crosses the winit event loop from another thread.
enum UserEvent {
    /// A GPU notice from the renderer's callback, guarded by its generation.
    Gpu(GpuEvent),
    /// A live-data wake; the flush that follows drains notified sources.
    Wake,
}

pub(crate) fn run(mut app: App) -> Result<(), AppError> {
    if app.title.trim().is_empty() {
        return Err(AppError::InvalidTitle);
    }
    // Before initialization, so a call that cannot run (a second loop in this
    // process, or the wrong thread) fails before the host's content is built.
    let event_loop = EventLoop::<UserEvent>::with_user_event()
        .build()
        .map_err(|e| AppError::EventLoop(e.to_string()))?;
    app.runtime
        .set_clipboard(clipboard::NativeClipboard::new())?;
    if let Some(initialize) = app.initialize.take()
        && let Err(error) = initialize(&mut app.runtime)
    {
        return finish_runtime(&mut app.runtime, Err(error));
    }
    // A waker taken during initialization shares the runtime's flag, so it works
    // once delivery exists; a notification before this point only marks the
    // runtime due, which the first `about_to_wait` observes.
    let wake_proxy = event_loop.create_proxy();
    app.runtime
        .set_wake_delivery(move || wake_proxy.send_event(UserEvent::Wake).is_ok());
    let mut runner = Runner::new(app, event_loop.create_proxy());
    let event_loop_error = event_loop
        .run_app(&mut runner)
        .err()
        .map(|e| AppError::EventLoop(e.to_string()));
    let error = event_loop_error
        .or(runner.error.take())
        .or(runner.current.component_recovery.error.take())
        .or_else(|| {
            runner
                .parked
                .values_mut()
                .find_map(|window| window.component_recovery.error.take())
        })
        .map_or(Ok(()), Err);
    finish_runtime(&mut runner.app.runtime, error)
}

// Finalize even when the event loop did not call exiting. Kept independent of
// native handles so terminal result ownership is verifiable headlessly.
fn finish_runtime(
    runtime: &mut crate::Runtime,
    result: Result<(), AppError>,
) -> Result<(), AppError> {
    runtime.shutdown();
    let mut errors = runtime.take_shutdown_errors().into_iter();
    if let Some(first) = errors.next() {
        Err(AppError::Shutdown(crate::AppShutdownError::new(
            result.err(),
            first,
            errors,
        )))
    } else {
        result
    }
}

struct Runner {
    current: NativeState,
    parked: std::collections::HashMap<WindowId, NativeState>,
    device_recovery: DeviceRecovery,
    closing: Vec<PendingClose>,
    app: App,
    proxy: EventLoopProxy<UserEvent>,
    start: Instant,
    generation: u64,
    error: Option<AppError>,
    #[cfg(test)]
    smoke: Option<Smoke>,
}
impl Runner {
    fn new(app: App, proxy: EventLoopProxy<UserEvent>) -> Self {
        // This host renders capture jobs offscreen with each window's renderer.
        app.runtime.set_capture_support(true);
        let current = NativeState::new(app.runtime.primary_window());
        Self {
            current,
            parked: std::collections::HashMap::new(),
            device_recovery: DeviceRecovery::default(),
            closing: Vec::new(),
            app,
            proxy,
            start: Instant::now(),
            generation: 0,
            error: None,
            #[cfg(test)]
            smoke: None,
        }
    }
    fn sync_ime(&mut self) {
        let state = self.app.runtime.text_input_state();
        if let Some(window) = &self.current.window {
            if self.current.ime.reconcile(&self.app.runtime) {
                self.current.ime_area = None;
                window.set_ime_allowed(false);
                window.set_ime_allowed(self.current.ime.wanted());
            }
            if let Some((_, state)) = state {
                let [x, y, w, h] = state.caret.values();
                let scale = self.app.runtime.viewport().scale();
                if self.current.ime_area == Some((state.caret, scale)) {
                    return;
                }
                self.current.ime_area = Some((state.caret, scale));
                window.set_ime_cursor_area(
                    winit::dpi::PhysicalPosition::new(f64::from(x) * scale, f64::from(y) * scale),
                    winit::dpi::PhysicalSize::new(f64::from(w) * scale, f64::from(h) * scale),
                );
            }
        }
    }
    fn native_key(&mut self, event: winit::event::KeyEvent) -> Result<(), AppError> {
        let action = if event.state == ElementState::Pressed {
            if event.repeat {
                KeyAction::Repeat
            } else {
                KeyAction::Press
            }
        } else {
            KeyAction::Release
        };
        use winit::platform::modifier_supplement::KeyEventExtModifierSupplement;
        let text = event.text.as_deref();
        let dispatch = self.app.runtime.command_key_input(keyboard::command_input(
            event.physical_key,
            action,
            &event.key_without_modifiers(),
            &event.logical_key,
            text,
            self.current.ime.composing,
            self.app.runtime.keyboard().modifiers(),
        ))?;
        self.sync_ime();
        if let Some(text) = keyboard::admitted_text(
            &dispatch,
            event.state == ElementState::Pressed,
            self.current.ime.composing,
            text,
        ) {
            self.app.runtime.text_input(&text)?;
        }
        Ok(())
    }
    fn native_ime(&mut self, event: winit::event::Ime) -> Result<(), AppError> {
        self.current.ime.receive(&mut self.app.runtime, event)?;
        Ok(())
    }
    fn scale_changed(&mut self) -> Result<(), AppError> {
        match &self.current.window {
            Some(window) => self.resize(window.inner_size()),
            None => Ok(()),
        }
    }
    fn occluded(&mut self, occluded: bool, backing_off: bool) -> Result<(), AppError> {
        self.current.occluded = occluded;
        self.current.surface_blocked = false;
        self.sync_visibility()?;
        if !occluded {
            self.current.dirty = true;
        }
        if occluded || backing_off {
            // A hide released focus above; a shown window restores it once
            // the backoff ends, in its next redraw.
            Ok(())
        } else {
            self.focus(self.current.window_focused && self.current.presentable())
        }
    }
    fn window_focused(&mut self, focused: bool, backing_off: bool) -> Result<(), AppError> {
        if self
            .current
            .window_focused(&mut self.app.runtime, focused, backing_off)?
        {
            self.sync_ime();
            self.app.runtime.flush(self.start.elapsed())?;
        }
        Ok(())
    }
    fn modifiers_changed(&mut self, modifiers: winit::event::Modifiers) -> Result<(), AppError> {
        let modifiers = modifiers.state();
        self.app.runtime.native_modifiers(crate::Modifiers {
            shift: modifiers.shift_key(),
            control: modifiers.control_key(),
            alt: modifiers.alt_key(),
            super_key: modifiers.super_key(),
        })?;
        Ok(())
    }
    fn mouse_input(
        &mut self,
        state: ElementState,
        button: winit::event::MouseButton,
    ) -> Result<(), AppError> {
        let button = match button {
            winit::event::MouseButton::Left => crate::PointerButton::Primary,
            winit::event::MouseButton::Right => crate::PointerButton::Secondary,
            winit::event::MouseButton::Middle => crate::PointerButton::Middle,
            winit::event::MouseButton::Back => crate::PointerButton::Back,
            winit::event::MouseButton::Forward => crate::PointerButton::Forward,
            winit::event::MouseButton::Other(id) => crate::PointerButton::Other(id),
        };
        self.pointer(if state == ElementState::Pressed {
            crate::PointerKind::Press(button)
        } else {
            crate::PointerKind::Release(button)
        })
    }
    fn mouse_wheel(&mut self, delta: winit::event::MouseScrollDelta) -> Result<(), AppError> {
        let scale = self.app.runtime.viewport().scale();
        let (x, y) = match delta {
            winit::event::MouseScrollDelta::LineDelta(x, y) => (-x * 40.0, -y * 40.0),
            winit::event::MouseScrollDelta::PixelDelta(position) => {
                ((-position.x / scale) as f32, (-position.y / scale) as f32)
            }
        };
        let delta = crate::Point::new(x, y)?;
        self.pointer(crate::PointerKind::Scroll(delta))
    }
    /// winit reports the change of magnification, positive to grow. An
    /// unusable value, such as NaN or a shrink to nothing, is dropped.
    fn pinch(&mut self, delta: f64) -> Result<(), AppError> {
        match crate::Magnification::new(1.0 + delta as f32) {
            Ok(factor) => self.pointer(crate::PointerKind::Pinch(factor)),
            Err(_) => Ok(()),
        }
    }
    fn pointer(&mut self, kind: crate::PointerKind) -> Result<(), AppError> {
        self.current.motion.send(&mut self.app.runtime, kind)?;
        Ok(())
    }
    fn fail(&mut self, event_loop: &ActiveEventLoop, mut error: AppError) {
        if error.is_component_failure() {
            self.current.motion.discard();
            let result =
                self.current
                    .component_recovery
                    .handle(&mut self.app, error, self.start.elapsed());
            self.sync_ime();
            match result {
                Ok(deadline) => {
                    self.current.dirty = true;
                    event_loop.set_control_flow(
                        self.start
                            .checked_add(deadline)
                            .map_or(ControlFlow::Wait, ControlFlow::WaitUntil),
                    );
                    return;
                }
                Err(failure) => error = failure,
            }
        }
        if self.error.is_none() {
            self.error = Some(error);
        }
        event_loop.exit();
    }
    fn active(&self) -> bool {
        self.current.presentable()
    }
    /// Applies the selected window's visibility, derived from its state, to
    /// the runtime in the order every hide and show path shares.
    fn sync_visibility(&mut self) -> Result<(), AppError> {
        let visible = self.current.shown();
        self.current
            .apply_visibility(&mut self.app.runtime, visible)
    }
    /// Moves the pointer to a native physical position. Motion is coalesced
    /// and delivered before the next discrete input, on the next scheduling
    /// turn, or in the next redraw; it requests no presentation by itself.
    fn cursor_moved(
        &mut self,
        position: winit::dpi::PhysicalPosition<f64>,
    ) -> Result<(), AppError> {
        let scale = self.app.runtime.viewport().scale();
        let point = crate::Point::new((position.x / scale) as f32, (position.y / scale) as f32)?;
        self.current.motion.queue_at_scale(point, scale);
        Ok(())
    }
    /// Shows the runtime's cursor for the pointer's current hit region,
    /// calling the window system only when it changes.
    fn sync_cursor(&mut self) {
        let icon = self.app.runtime.cursor_icon();
        let Some(window) = &self.current.window else {
            return;
        };
        if self.current.cursor == Some(icon) {
            return;
        }
        self.current.cursor = Some(icon);
        window.set_cursor(native_cursor(icon));
    }
    fn create_renderer(&mut self) -> Result<(), AppError> {
        let Some(window) = self.current.window.clone() else {
            return Ok(());
        };
        self.generation = self
            .generation
            .checked_add(1)
            .ok_or_else(|| AppError::Graphics("GPU generation exhausted".into()))?;
        let generation = self.generation;
        let proxy = self.proxy.clone();
        self.current.renderer = Some(
            Presenter::new(
                self.app.software,
                window,
                self.current.size.width,
                self.current.size.height,
                move |notice| {
                    // An exited event loop has no consumer. GPU callbacks may outlive it;
                    // teardown intentionally discards such notifications, never panics.
                    let _ = proxy.send_event(UserEvent::Gpu(GpuEvent { generation, notice }));
                },
            )
            .map_err(AppError::from_renderer)?,
        );
        Ok(())
    }
    /// Applies a native size. A size beyond the device's largest surface keeps
    /// the renderer's last configuration and leaves the window unpresented until
    /// a size the device accepts; the limit goes to the host's error policy,
    /// which may end the application.
    fn resize(&mut self, size: PhysicalSize<u32>) -> Result<(), AppError> {
        self.current.size = size;
        self.current.surface_blocked = false;
        let limit = match &mut self.current.renderer {
            Some(renderer) => surface_limit(renderer.resize(size.width, size.height))?,
            None => None,
        };
        self.current.oversized = limit.is_some();
        self.sync_visibility()?;
        if let Some(window) = &self.current.window {
            let scale = window.scale_factor();
            self.app.runtime.set_viewport(Viewport::from_physical(
                PhysicalExtent::new(size.width, size.height),
                scale,
            )?)?;
        }
        self.current.dirty = true;
        if let Some(limit) = limit {
            let (action, error) = self.app.handle_error(AppError::Limit(limit))?;
            if action == crate::ErrorAction::Exit {
                return Err(error);
            }
        }
        Ok(())
    }
    fn retry(&mut self) -> Result<(), AppError> {
        self.current.retry_at = Some(self.start.elapsed() + self.current.recovery.retry_delay()?);
        self.current.dirty = true;
        Ok(())
    }
    fn redraw(&mut self) -> Result<(), AppError> {
        self.current.redraw_pending = false;
        if !self.current.occluded {
            self.current.surface_blocked = false;
            self.sync_visibility()?;
        }
        if !self.active()
            || self
                .current
                .component_recovery
                .waiting(self.start.elapsed())
        {
            return Ok(());
        }
        if self.current.window_focused && self.current.restore_focus.is_some() {
            self.focus(true)?;
        }
        self.current.motion.flush(&mut self.app.runtime)?;
        self.app.runtime.flush(self.start.elapsed())?;
        self.sync_cursor();
        let scene = self.app.runtime.paint()?;
        if let (Some(renderer), Some(window)) = (&mut self.current.renderer, &self.current.window) {
            match renderer
                .render(scene, || window.pre_present_notify())
                .map_err(AppError::from_renderer)?
            {
                Frame::Presented => {
                    #[cfg(test)]
                    {
                        self.current.frames += 1;
                    }
                    self.current.component_recovery = ComponentRecovery::default();
                    self.current.exposed = true;
                    self.current.dirty = false;
                    self.current.recovery.presented();
                    self.current.retry_at = None;
                    #[cfg(test)]
                    if let Some(smoke) = &mut self.smoke {
                        smoke.frames += 1;
                    }
                }
                Frame::Retry => self.retry()?,
                Frame::Occluded => {
                    self.current.surface_blocked = true;
                    self.current.retry_at = None;
                    self.sync_visibility()?;
                }
                Frame::Recreate => {
                    renderer
                        .recreate(window.clone())
                        .map_err(AppError::from_renderer)?;
                    self.retry()?;
                }
            }
        }
        self.process_captures();
        Ok(())
    }
    /// Renders the selected window's pending capture jobs with its renderer,
    /// offscreen, and hands each frame to the runtime, once that window has
    /// no updates left to flush. A window that cannot render now completes
    /// its jobs with an error rather than leaving them; a job for a window
    /// this host no longer has is reported unknown. Other windows' jobs wait
    /// for their own turn, which `about_to_wait` gives every window.
    fn process_captures(&mut self) {
        let jobs = self.app.runtime.pending_captures();
        if jobs.is_empty() {
            return;
        }
        let now = self.start.elapsed();
        let flushed = !self
            .app
            .runtime
            .next_deadline()
            .is_some_and(|time| time <= now);
        for job in jobs {
            if job.window() != self.current.root.id() {
                let known = self
                    .app
                    .runtime
                    .native_windows()
                    .iter()
                    .any(|root| root.id() == job.window());
                if !known {
                    self.app
                        .runtime
                        .complete_capture(&job, Err(crate::CaptureError::UnknownTarget));
                }
                continue;
            }
            if !self.active() {
                self.app
                    .runtime
                    .complete_capture(&job, Err(crate::CaptureError::SurfaceUnavailable));
                continue;
            }
            if !flushed {
                // The next flush settles the updates; the job is rendered after it.
                continue;
            }
            // Painting the capture takes the window's damage, so the window
            // keeps it for its next presentation; otherwise the screen would
            // go on showing the content from before the change.
            self.current.dirty |= self.app.runtime.needs_paint();
            let frame = match self.app.runtime.capture_scene(&job) {
                Ok(scene) => match &mut self.current.renderer {
                    Some(renderer) => renderer
                        .render_offscreen(scene)
                        .map_err(capture_render_error),
                    None => Err(crate::CaptureError::SurfaceUnavailable),
                },
                Err(error) => Err(error),
            };
            self.app.runtime.complete_capture(&job, frame);
        }
    }
    fn focus(&mut self, focused: bool) -> Result<(), AppError> {
        self.current.focus(&mut self.app.runtime, focused)
    }
}
impl ApplicationHandler<UserEvent> for Runner {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.current.window.is_some() {
            if self.current.renderer.is_none() {
                let result = (|| {
                    self.create_renderer()?;
                    let mut ids: Vec<_> = self.parked.keys().copied().collect();
                    for state in self.parked.values_mut() {
                        if let Some(window) = &state.window {
                            state.renderer = Some(
                                self.current
                                    .renderer
                                    .as_ref()
                                    .ok_or_else(|| {
                                        AppError::Graphics("missing resumed GPU".into())
                                    })?
                                    .new_surface(
                                        window.clone(),
                                        state.size.width,
                                        state.size.height,
                                    )
                                    .map_err(AppError::from_renderer)?,
                            );
                        }
                    }
                    if let Some(window) = &self.current.window {
                        ids.push(window.id());
                    }
                    for id in ids {
                        if self.select_native(id)? {
                            self.resize(self.current.size)?;
                            self.current.dirty = true;
                        }
                    }
                    Ok(())
                })();
                if let Err(error) = result {
                    self.fail(event_loop, error);
                }
            }
            return;
        }
        let result = (|| {
            let window = Arc::new(
                event_loop
                    .create_window(main_window_attributes(&self.app.title, &self.app.window))
                    .map_err(|e| AppError::Window(e.to_string()))?,
            );
            self.current.size = window.inner_size();
            self.current.window = Some(window);
            self.current.occluded = false;
            self.current.surface_blocked = false;
            self.current.exposed = false;
            self.create_renderer()?;
            self.resize(self.current.size)
        })();
        if let Err(error) = result {
            self.fail(event_loop, error);
        }
    }
    fn suspended(&mut self, event_loop: &ActiveEventLoop) {
        // Desktop windows retain native identity; release every surface before
        // suspension and rebuild them on the next resume using one GPU context.
        let mut ids: Vec<_> = self.parked.keys().copied().collect();
        if let Some(window) = &self.current.window {
            ids.push(window.id());
        }
        for id in ids {
            let result = (|| {
                if !self.select_native(id)? {
                    return Ok(());
                }
                self.current.window_focused = false;
                self.current.renderer = None;
                self.sync_visibility()?;
                self.current.retry_at = None;
                self.current.redraw_pending = false;
                Ok(())
            })();
            if let Err(error) = result {
                self.fail(event_loop, error);
            }
        }
    }
    fn window_event(&mut self, event_loop: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
        match self.select_native(id) {
            Ok(true) => {}
            Ok(false) => return,
            Err(error) => {
                self.fail(event_loop, error);
                return;
            }
        }
        let backing_off = self
            .current
            .component_recovery
            .waiting(self.start.elapsed());
        if backing_off
            && !matches!(
                event,
                WindowEvent::CloseRequested
                    | WindowEvent::Resized(_)
                    | WindowEvent::ScaleFactorChanged { .. }
                    | WindowEvent::Occluded(_)
                    | WindowEvent::Focused(_)
                    | WindowEvent::RedrawRequested
            )
        {
            return;
        }
        // Button, wheel and leave boundaries flush through `pointer` below.
        // Focus loss must release input before a failed pending update can exit.
        if flushes_motion_first(&event)
            && !backing_off
            && let Err(error) = self.current.motion.flush(&mut self.app.runtime)
        {
            self.fail(event_loop, error.into());
            return;
        }
        if matches!(
            event,
            WindowEvent::KeyboardInput { .. }
                | WindowEvent::Ime(_)
                | WindowEvent::MouseInput { .. }
                | WindowEvent::MouseWheel { .. }
                | WindowEvent::PinchGesture { .. }
                | WindowEvent::Focused(true)
        ) && !backing_off
            && let Err(error) = self.app.runtime.flush(self.start.elapsed())
        {
            self.fail(event_loop, error.into());
            return;
        }
        let result = match event {
            WindowEvent::CloseRequested => {
                self.begin_close(event_loop, self.current.root.clone(), None)
            }
            WindowEvent::Resized(size) => self.resize(size),
            WindowEvent::ScaleFactorChanged { .. } => self.scale_changed(),
            WindowEvent::Occluded(occluded) => self.occluded(occluded, backing_off),
            WindowEvent::Focused(focused) => self.window_focused(focused, backing_off),
            WindowEvent::KeyboardInput {
                event,
                is_synthetic: false,
                ..
            } => self.native_key(event),
            WindowEvent::ModifiersChanged(modifiers) => self.modifiers_changed(modifiers),
            WindowEvent::Ime(event) => self.native_ime(event),
            WindowEvent::CursorMoved { position, .. } => self.cursor_moved(position),
            WindowEvent::CursorLeft { .. } => self.pointer(crate::PointerKind::Leave),
            WindowEvent::MouseInput { state, button, .. } => self.mouse_input(state, button),
            WindowEvent::MouseWheel { delta, .. } => self.mouse_wheel(delta),
            WindowEvent::PinchGesture { delta, .. } => self.pinch(delta),
            WindowEvent::RedrawRequested => self.redraw(),
            _ => Ok(()),
        };
        if let Err(error) = result {
            self.fail(event_loop, error);
        }
    }
    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: UserEvent) {
        // A wake needs no handling of its own: `about_to_wait` follows, reads the
        // runtime's pending flag through its deadline and flushes.
        let UserEvent::Gpu(event) = event else {
            return;
        };
        if event.generation != self.generation || self.current.renderer.is_none() {
            return;
        }
        match event.notice {
            Notice::Error(error) => self.fail(event_loop, AppError::Graphics(error)),
            Notice::Lost(error) => {
                if let Err(error) = self.recover_device(error) {
                    self.fail(event_loop, error);
                }
            }
        }
    }
    fn exiting(&mut self, _event_loop: &ActiveEventLoop) {
        self.closing.clear();
        self.app.runtime.shutdown();
    }
    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if let Err(error) = self
            .app
            .runtime
            .flush_commands()
            .map_err(AppError::from)
            .and_then(|_| self.presentation_requests(event_loop))
            .and_then(|_| self.complete_closes(event_loop))
            .and_then(|_| self.retire_windows())
        {
            self.fail(event_loop, error);
            return;
        }
        // Interval timers are the application's: they run on time even while
        // every window's redraw is withheld, and their deadline wakes the loop.
        if let Err(error) = self.app.runtime.service_intervals(self.start.elapsed()) {
            self.fail(event_loop, error.into());
            return;
        }
        let mut ids: Vec<_> = self.parked.keys().copied().collect();
        if let Some(window) = &self.current.window {
            ids.push(window.id());
        }
        let mut next: Option<Duration> = self.app.runtime.interval_deadline();
        for id in ids {
            match self.select_native(id) {
                Ok(true) => {
                    if let Some(deadline) = self.schedule_current(event_loop) {
                        next = Some(next.map_or(deadline, |n| n.min(deadline)));
                    }
                }
                Ok(false) => {}
                Err(error) => {
                    self.fail(event_loop, error);
                    return;
                }
            }
        }
        if self.app.runtime.has_presentation_requests() || self.app.runtime.has_command_requests() {
            next = Some(self.start.elapsed());
        }
        event_loop.set_control_flow(
            next.and_then(|time| self.start.checked_add(time))
                .map_or(ControlFlow::Wait, ControlFlow::WaitUntil),
        );
    }
}

/// The window-system cursor the adapter shows for a runtime cursor.
fn native_cursor(icon: crate::CursorIcon) -> winit::window::CursorIcon {
    match icon {
        crate::CursorIcon::ResizeDiagonal => winit::window::CursorIcon::NwseResize,
        crate::CursorIcon::Move => winit::window::CursorIcon::Move,
        crate::CursorIcon::Grabbing => winit::window::CursorIcon::Grabbing,
        crate::CursorIcon::Copy => winit::window::CursorIcon::Copy,
        crate::CursorIcon::Alias => winit::window::CursorIcon::Alias,
        crate::CursorIcon::NoDrop => winit::window::CursorIcon::NoDrop,
        crate::CursorIcon::Pointer => winit::window::CursorIcon::Pointer,
        crate::CursorIcon::Text => winit::window::CursorIcon::Text,
        crate::CursorIcon::ResizeHorizontal => winit::window::CursorIcon::EwResize,
        crate::CursorIcon::ResizeVertical => winit::window::CursorIcon::NsResize,
        // `Default`, and any cursor added after this mapping.
        _ => winit::window::CursorIcon::Default,
    }
}

/// The main window's native attributes from the validated `App` geometry.
fn main_window_attributes(
    title: &str,
    geometry: &crate::app::MainWindow,
) -> winit::window::WindowAttributes {
    let logical = |size: crate::Size| LogicalSize::new(size.width(), size.height());
    let mut attributes = Window::default_attributes()
        .with_title(title)
        .with_inner_size(logical(geometry.size))
        .with_resizable(geometry.resizable);
    if let Some(min) = geometry.min {
        attributes = attributes.with_min_inner_size(logical(min));
    }
    if let Some(max) = geometry.max {
        attributes = attributes.with_max_inner_size(logical(max));
    }
    attributes
}

// Keep backend types private while retaining portable runtime/drawing causes.
fn capture_render_error(error: crate::platform::presenter::RenderError) -> crate::CaptureError {
    match error {
        crate::platform::presenter::RenderError::Drawing(error) => error.into(),
        crate::platform::presenter::RenderError::Unconfigured => {
            crate::CaptureError::SurfaceUnavailable
        }
        crate::platform::presenter::RenderError::Limit(
            limit @ crate::LimitError::FrameSize { .. },
        ) => crate::CaptureError::Limit(limit),
        error => crate::CaptureError::Backend(error.to_string()),
    }
}

/// Separates a renderer resize's surface-extent limit, which the window
/// survives unpresented, from its other failures, which end the application.
fn surface_limit(
    result: Result<(), crate::platform::presenter::RenderError>,
) -> Result<Option<crate::LimitError>, AppError> {
    match result {
        Ok(()) => Ok(None),
        Err(crate::platform::presenter::RenderError::Limit(
            limit @ (crate::LimitError::SurfaceExtent { .. } | crate::LimitError::FrameSize { .. }),
        )) => Ok(Some(limit)),
        Err(error) => Err(AppError::from_renderer(error)),
    }
}

struct NativeState {
    root: crate::NativeWindowId,
    #[cfg(test)]
    frames: u64,
    window: Option<Arc<Window>>,
    renderer: Option<Presenter>,
    size: PhysicalSize<u32>,
    motion: motion::Motion,
    occluded: bool,
    /// Native keyboard focus as the window's focus events last reported it.
    /// A window has none until its first gain.
    window_focused: bool,
    surface_blocked: bool,
    /// The window has presented a frame since it was created, so it has been
    /// on screen. Before that, a blocked surface means the window is not on
    /// screen yet rather than hidden.
    exposed: bool,
    /// The last native size exceeds the device's largest surface; the renderer
    /// keeps its previous configuration and nothing is presented until a
    /// resize the device accepts.
    oversized: bool,
    dirty: bool,
    redraw_pending: bool,
    retry_at: Option<Duration>,
    recovery: SurfaceRetries,
    component_recovery: ComponentRecovery,
    restore_focus: Option<WidgetId>,
    ime: ime::NativeIme,
    ime_area: Option<(crate::Rect, f64)>,
    /// The cursor last shown, so an unchanged one is not set again.
    cursor: Option<crate::CursorIcon>,
}
impl NativeState {
    fn new(root: crate::NativeWindowId) -> Self {
        Self {
            root,
            #[cfg(test)]
            frames: 0,
            window: None,
            renderer: None,
            size: PhysicalSize::new(0, 0),
            motion: motion::Motion::default(),
            occluded: false,
            window_focused: false,
            surface_blocked: false,
            exposed: false,
            oversized: false,
            dirty: true,
            redraw_pending: false,
            retry_at: None,
            recovery: SurfaceRetries::default(),
            component_recovery: ComponentRecovery::default(),
            restore_focus: None,
            ime: ime::NativeIme::default(),
            ime_area: None,
            cursor: None,
        }
    }
    /// Whether this window can present now: it exists, has a surface on the
    /// current device and nothing blocks presentation. Scheduling, redraws and
    /// focus restoration follow it.
    fn presentable(&self) -> bool {
        self.has_surface() && !self.presentation_blocked()
    }
    /// Whether the runtime sees this window's content as visible. The one
    /// visibility rule every hide and show path applies to the runtime.
    fn shown(&self) -> bool {
        self.has_surface() && !self.content_hidden()
    }
    fn has_surface(&self) -> bool {
        self.window.is_some() && self.renderer.is_some()
    }
    /// Whether occlusion, a compositor block, an oversized or zero size stops
    /// presentation.
    fn presentation_blocked(&self) -> bool {
        self.occluded || self.surface_blocked || self.unpresentable_size()
    }
    /// Whether the window is hidden, as opposed to only paused. A compositor
    /// block before the window has been on screen is the window not being
    /// shown yet, not a hide: macOS reports a new window's surface occluded
    /// until the window server first shows it. That pauses presentation
    /// without ending input, timers, gestures and open surfaces set up while
    /// the window was being created. Real occlusion arrives as an `Occluded`
    /// window event and hides the window at any time.
    fn content_hidden(&self) -> bool {
        self.occluded || (self.surface_blocked && self.exposed) || self.unpresentable_size()
    }
    fn unpresentable_size(&self) -> bool {
        self.oversized || self.size.width == 0 || self.size.height == 0
    }
    /// Applies a native focus event to the runtime and returns whether it
    /// ended focus the window had, which the caller follows with an IME
    /// update and a flush. A loss without an earlier gain changes nothing:
    /// winit's macOS backend announces every new window's initial state as a
    /// loss before the window becomes key, and treating it as one would end
    /// the focus, timers and gestures of the widget the application focused
    /// while creating the window.
    fn window_focused(
        &mut self,
        runtime: &mut crate::Runtime,
        focused: bool,
        backing_off: bool,
    ) -> Result<bool, AppError> {
        if !focused && !self.window_focused {
            return Ok(false);
        }
        self.window_focused = focused;
        if !focused {
            self.focus(runtime, false)?;
        } else if !backing_off && self.presentable() {
            self.focus(runtime, true)?;
        }
        // A gain while the window cannot present changes nothing: a window
        // the runtime sees as hidden released its focus when it hid and keeps
        // the remembered target until its redraw restores it, and a window
        // not yet on screen keeps the focus, timers and gestures set up while
        // it was created.
        Ok(!focused && !backing_off)
    }
    /// Applies `visible` to the runtime's selected window, which must be this
    /// one. A hide records and releases focus first, because the runtime
    /// clears focus when the window hides and the focused widget would
    /// otherwise not be restored when it shows again.
    fn apply_visibility(
        &mut self,
        runtime: &mut crate::Runtime,
        visible: bool,
    ) -> Result<(), AppError> {
        if !visible {
            self.focus(runtime, false)?;
        }
        runtime.set_window_visible(visible)?;
        Ok(())
    }
    /// Releases this window's focus, remembering the focused widget, or
    /// restores the remembered widget; one that is no longer focusable here
    /// passes focus to the next focusable widget instead.
    fn focus(&mut self, runtime: &mut crate::Runtime, focused: bool) -> Result<(), AppError> {
        if focused {
            if let Some(id) = self.restore_focus.take() {
                if runtime.window_of(&id).ok().as_ref() != Some(&self.root) {
                    runtime.focus_next(crate::FocusDirection::Forward)?;
                    return Ok(());
                }
                match runtime.focus(&id) {
                    Ok(()) => (),
                    Err(
                        RuntimeError::Hidden | RuntimeError::Unmounted | RuntimeError::ModalBlocked,
                    ) => {
                        runtime.focus_next(crate::FocusDirection::Forward)?;
                    }
                    Err(error) => return Err(error.into()),
                }
            }
        } else {
            self.motion.discard();
            if let Some(id) = runtime.focused() {
                self.restore_focus = Some(id);
            }
            runtime.clear_focus();
        }
        Ok(())
    }
    /// Starts this window over on a recreated device: a fresh surface retry
    /// budget, no pending retry or compositor block, and a frame to present.
    fn device_recreated(&mut self) {
        self.recovery = SurfaceRetries::default();
        self.dirty = true;
        self.retry_at = None;
        self.surface_blocked = false;
    }
}

/// Whether coalesced pointer motion is delivered before `event`, preserving
/// order across key input, modifier changes and geometry/scale transitions. A
/// modifier change resolves a drag at the pointer's current position, so the
/// motion that came before it goes first.
fn flushes_motion_first(event: &WindowEvent) -> bool {
    matches!(
        event,
        WindowEvent::KeyboardInput { .. }
            | WindowEvent::ModifiersChanged(_)
            | WindowEvent::Ime(_)
            | WindowEvent::Resized(_)
            | WindowEvent::ScaleFactorChanged { .. }
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::presenter::RenderError;

    #[test]
    fn motion_is_delivered_before_a_modifier_change() {
        assert!(flushes_motion_first(&WindowEvent::ModifiersChanged(
            winit::event::Modifiers::default()
        )));
        assert!(flushes_motion_first(&WindowEvent::Ime(
            winit::event::Ime::Enabled
        )));
        assert!(!flushes_motion_first(&WindowEvent::RedrawRequested));
    }

    #[test]
    fn capture_renderer_errors_keep_portable_causes_and_distinct_limits() {
        let runtime = RuntimeError::Geometry(crate::GeometryError::Negative);
        assert_eq!(
            capture_render_error(RenderError::Drawing(crate::DrawError::Runtime(
                runtime.clone()
            ))),
            crate::CaptureError::Runtime(runtime)
        );
        assert_eq!(
            capture_render_error(RenderError::Drawing(crate::DrawError::InvalidColor)),
            crate::CaptureError::Drawing(crate::DrawError::InvalidColor)
        );
        assert_eq!(
            capture_render_error(RenderError::Unconfigured),
            crate::CaptureError::SurfaceUnavailable
        );
        assert_eq!(
            capture_render_error(RenderError::Limit(crate::LimitError::FrameSize {
                width: 10000,
                height: 10000
            })),
            crate::CaptureError::Limit(crate::LimitError::FrameSize {
                width: 10000,
                height: 10000
            })
        );
        for limit in [
            crate::LimitError::SurfaceExtent {
                width: 10000,
                height: 2,
                max: 8192,
            },
            crate::LimitError::VertexCount,
        ] {
            let diagnostic = RenderError::Limit(limit);
            assert_eq!(
                capture_render_error(diagnostic.clone()),
                crate::CaptureError::Backend(diagnostic.to_string())
            );
        }
    }

    struct ShutdownSource {
        cause: RuntimeError,
        callbacks: std::rc::Rc<std::cell::Cell<usize>>,
    }
    impl crate::Component for ShutdownSource {
        fn update(&mut self, _: &mut crate::UpdateContext) -> Result<(), RuntimeError> {
            Ok(())
        }
        fn measure(
            &self,
            _: &crate::MeasureContext<'_>,
            _: crate::Constraints,
        ) -> Result<crate::Size, RuntimeError> {
            Ok(crate::Size::from([20, 20]))
        }
        fn drag_finished(
            &mut self,
            _: &mut crate::EventContext,
            _: crate::DragOutcome,
        ) -> Result<(), RuntimeError> {
            self.callbacks.set(self.callbacks.get() + 1);
            Err(self.cause.clone())
        }
    }
    fn pending_shutdown() -> (
        crate::Runtime,
        RuntimeError,
        std::rc::Rc<std::cell::Cell<usize>>,
    ) {
        let mut runtime = crate::Runtime::new();
        let cause = RuntimeError::from(crate::ApplicationError::from("terminal callback failed"));
        let callbacks = std::rc::Rc::new(std::cell::Cell::new(0));
        runtime
            .set_viewport(crate::Viewport::new(100.0, 100.0, 1.0).unwrap())
            .unwrap();
        let source = runtime
            .mount(
                "source",
                ShutdownSource {
                    cause: cause.clone(),
                    callbacks: callbacks.clone(),
                },
            )
            .unwrap();
        runtime.flush(runtime.now()).unwrap();
        runtime
            .pointer_input(crate::PointerEvent::new(
                crate::Point::new(5.0, 5.0).unwrap(),
                crate::PointerKind::Press(crate::PointerButton::Primary),
            ))
            .unwrap();
        runtime
            .event(&source, |cx| {
                cx.start_drag(
                    crate::DragPayload::new("item"),
                    crate::DropOperations::MOVE,
                    crate::DragPreview::none(),
                )
            })
            .unwrap()
            .unwrap();
        (runtime, cause, callbacks)
    }
    #[test]
    fn clean_shutdown_preserves_success_and_existing_typed_failure() {
        let mut runtime = crate::Runtime::new();
        assert!(finish_runtime(&mut runtime, Ok(())).is_ok());
        let mut runtime = crate::Runtime::new();
        let prior = RuntimeError::from(crate::ApplicationError::from("initialization failed"));
        assert!(
            matches!(finish_runtime(&mut runtime, Err(prior.clone().into())), Err(AppError::Runtime(error)) if error == prior)
        );
    }
    #[test]
    fn shutdown_forwards_pending_terminal_failure_once_without_recovery() {
        let (mut runtime, cause, callbacks) = pending_shutdown();
        let error = finish_runtime(&mut runtime, Ok(())).unwrap_err();
        assert!(
            matches!(&error, AppError::Shutdown(shutdown) if shutdown.failure().is_none() && shutdown.errors() == std::slice::from_ref(&cause))
        );
        assert_eq!(
            std::error::Error::source(&error)
                .unwrap()
                .downcast_ref::<RuntimeError>(),
            Some(&cause)
        );
        assert!(!error.is_component_failure());
        assert_eq!(callbacks.get(), 1);
        assert!(finish_runtime(&mut runtime, Ok(())).is_ok());
        assert_eq!(callbacks.get(), 1);
    }
    #[test]
    fn shutdown_preserves_prior_initialization_or_event_loop_failure() {
        for event_loop in [false, true] {
            let (mut runtime, cause, callbacks) = pending_shutdown();
            let prior = if event_loop {
                AppError::EventLoop("loop failed".into())
            } else {
                RuntimeError::from(crate::ApplicationError::from("initialization failed")).into()
            };
            let error = finish_runtime(&mut runtime, Err(prior)).unwrap_err();
            let source = std::error::Error::source(&error)
                .unwrap()
                .downcast_ref::<AppError>()
                .unwrap();
            assert_eq!(matches!(source, AppError::EventLoop(_)), event_loop);
            let AppError::Shutdown(shutdown) = error else {
                panic!("all terminal causes must be retained")
            };
            let (prior, errors) = shutdown.into_parts();
            let prior = prior.expect("the original failure is retained");
            assert_eq!(errors, [cause]);
            assert_eq!(matches!(prior, AppError::EventLoop(_)), event_loop);
            assert_eq!(
                matches!(prior, AppError::Runtime(RuntimeError::Application(_))),
                !event_loop
            );
            assert_eq!(callbacks.get(), 1);
        }
    }
    #[test]
    fn shutdown_forwards_already_closed_reports_without_repeating_callbacks() {
        let (mut runtime, cause, callbacks) = pending_shutdown();
        runtime.shutdown();
        assert_eq!(callbacks.get(), 1);
        assert!(
            matches!(finish_runtime(&mut runtime, Ok(())), Err(AppError::Shutdown(shutdown)) if shutdown.failure().is_none() && shutdown.errors() == [cause])
        );
        assert!(finish_runtime(&mut runtime, Ok(())).is_ok());
        assert_eq!(callbacks.get(), 1);
    }

    struct ShutdownFocus(RuntimeError);
    impl crate::Component for ShutdownFocus {
        fn update(&mut self, _: &mut crate::UpdateContext) -> Result<(), RuntimeError> {
            Ok(())
        }
        fn measure(
            &self,
            _: &crate::MeasureContext<'_>,
            _: crate::Constraints,
        ) -> Result<crate::Size, RuntimeError> {
            Ok(crate::Size::from([20, 20]))
        }
        fn focusable(&self) -> bool {
            true
        }
        fn focus_lost(&mut self, _: &mut crate::EventContext) -> Result<(), RuntimeError> {
            Err(self.0.clone())
        }
    }
    #[test]
    fn shutdown_keeps_multiple_owned_causes_in_runtime_report_order() {
        let mut runtime = crate::Runtime::new();
        runtime
            .set_viewport(crate::Viewport::new(100.0, 100.0, 1.0).unwrap())
            .unwrap();
        let a = RuntimeError::from(crate::ApplicationError::from("first focus failure"));
        let b = RuntimeError::from(crate::ApplicationError::from("second focus failure"));
        let root = runtime
            .mount_tree(crate::Element::new(
                "root",
                crate::Column::new([
                    crate::Element::new("a", ShutdownFocus(a.clone())),
                    crate::Element::new("b", ShutdownFocus(b.clone())),
                ]),
            ))
            .unwrap();
        runtime.flush(runtime.now()).unwrap();
        for key in ["a", "b"] {
            let target = runtime.find_descendant(&root, key).unwrap().unwrap();
            runtime.focus(&target).unwrap();
            runtime.clear_focus();
        }
        let error = finish_runtime(&mut runtime, Ok(())).unwrap_err();
        assert_eq!(error.to_string(), "application shutdown cleanup failed");
        assert!(
            matches!(error, AppError::Shutdown(shutdown) if shutdown.failure().is_none() && shutdown.errors() == [a, b])
        );
        assert!(runtime.take_shutdown_errors().is_empty());
    }

    #[test]
    fn only_a_surface_extent_limit_keeps_the_window_alive() {
        let extent = crate::LimitError::SurfaceExtent {
            width: 40_000,
            height: 600,
            max: 16_384,
        };
        assert!(matches!(surface_limit(Ok(())), Ok(None)));
        assert!(matches!(
            surface_limit(Err(RenderError::Limit(extent.clone()))),
            Ok(Some(limit)) if limit == extent
        ));
        // Other limits and failures keep their fatal mapping.
        assert!(matches!(
            surface_limit(Err(RenderError::Limit(crate::LimitError::VertexCount))),
            Err(AppError::Limit(crate::LimitError::VertexCount))
        ));
        assert!(matches!(
            surface_limit(Err(RenderError::Surface("lost".into()))),
            Err(AppError::Surface(_))
        ));
    }
}
