//! Presentation scheduling and the recovery policies behind it: component
//! failure backoff, per-window surface retries and the one device recreation.
use super::*;
use crate::host::HostIntegration;

// This allowance spans update, input and paint errors. Only a successfully
// presented frame resets it; successful flushes cannot hide repeated paint errors.
#[derive(Default)]
pub(super) struct ComponentRecovery {
    attempts: u8,
    pub(super) retry_at: Option<Duration>,
    pub(super) error: Option<AppError>,
}
impl ComponentRecovery {
    pub(super) fn handle(
        &mut self,
        app: &mut App,
        error: AppError,
        now: Duration,
    ) -> Result<Duration, AppError> {
        app.runtime.cancel_input()?;
        let (action, error) = app.handle_error(error)?;
        if action == crate::ErrorAction::Retry
            && let Some(deadline) = self.retry(now)
        {
            self.error = Some(error);
            return Ok(deadline);
        }
        Err(error)
    }
    fn retry(&mut self, now: Duration) -> Option<Duration> {
        if self.attempts >= 3 {
            return None;
        }
        let deadline = now.checked_add(Duration::from_millis(100 << self.attempts))?;
        self.attempts += 1;
        self.retry_at = Some(deadline);
        Some(deadline)
    }
    pub(super) fn waiting(&self, now: Duration) -> bool {
        self.retry_at.is_some_and(|deadline| now < deadline)
    }
}

/// One window's surface-acquisition retry budget. A presented frame restores
/// it, and so does a recreated device, through `NativeState::device_recreated`.
#[derive(Default)]
pub(super) struct SurfaceRetries {
    failures: u8,
}
impl SurfaceRetries {
    pub(super) fn retry_delay(&mut self) -> Result<Duration, AppError> {
        if self.failures >= 3 {
            return Err(AppError::Surface(
                "acquisition did not recover after three retries".into(),
            ));
        }
        self.failures += 1;
        Ok(Duration::from_millis(16 * u64::from(self.failures)))
    }
    pub(super) fn presented(&mut self) {
        self.failures = 0;
    }
}

/// The application's single allowance for recreating the shared GPU device.
#[derive(Default)]
pub(super) struct DeviceRecovery {
    recovered: bool,
}
impl DeviceRecovery {
    fn device_lost(&mut self, message: String) -> Result<(), AppError> {
        if self.recovered {
            return Err(AppError::DeviceLost(message));
        }
        self.recovered = true;
        Ok(())
    }
}

// A compositor may withhold a redraw indefinitely while a Wayland window is
// minimized. Keep accumulated state/deadlines, but neither evaluate nor wake for
// presentation until the outstanding native redraw arrives.
fn prepare_updates(
    runtime: &mut crate::Runtime,
    now: Duration,
    ready: bool,
) -> Result<usize, RuntimeError> {
    if ready && runtime.next_deadline().is_some_and(|time| time <= now) {
        Ok(runtime.flush(now)?.components_updated)
    } else {
        // No flush now, so a tool's request is answered from the last one
        // rather than stranded behind a redraw the compositor may withhold.
        runtime.service_inspection()?;
        Ok(0)
    }
}

/// Whether the selected window may flush due updates on this turn: when no
/// native redraw is outstanding, or when a capture job waits for it. A
/// capture is an explicit, bounded request, so it is flushed for and rendered
/// offscreen on the turn it arrives rather than after a redraw the compositor
/// may withhold indefinitely (a minimized Wayland window).
fn updates_ready(
    runtime: &crate::Runtime,
    root: &crate::NativeWindowId,
    redraw_pending: bool,
) -> bool {
    !redraw_pending
        || runtime
            .pending_captures()
            .iter()
            .any(|job| job.window() == root.id())
}

fn presentation_deadline(
    ready: bool,
    runtime: Option<Duration>,
    retry: Option<Duration>,
) -> Option<Duration> {
    if !ready {
        return None;
    }
    match (runtime, retry) {
        (Some(a), Some(b)) => Some(a.min(b)),
        (a, b) => a.or(b),
    }
}

impl Runner {
    pub(super) fn recover_device(&mut self, message: String) -> Result<(), AppError> {
        self.device_recovery.device_lost(message)?;
        self.current.renderer = None;
        for window in self.parked.values_mut() {
            window.renderer = None;
        }
        self.create_renderer()?;
        let renderer = self
            .current
            .renderer
            .as_ref()
            .ok_or_else(|| AppError::Graphics("missing recovery device".into()))?;
        for window in self.parked.values_mut() {
            if let Some(native) = &window.window {
                window.renderer = Some(
                    renderer
                        .new_surface(native.clone(), window.size.width, window.size.height)
                        .map_err(AppError::from_renderer)?,
                );
                window.device_recreated();
            }
        }
        self.current.device_recreated();
        Ok(())
    }
    pub(super) fn schedule_current(&mut self, event_loop: &ActiveEventLoop) -> Option<Duration> {
        if self.error.is_some() {
            return None;
        }
        // Read on native event turns, never by adding a display-polling timer.
        // Unknown or unusable monitor rates keep the runtime's 60 Hz fallback.
        let rate = self
            .current
            .window
            .as_ref()
            .and_then(|window| window.current_monitor())
            .and_then(|monitor| monitor.refresh_rate_millihertz())
            .and_then(|rate| crate::PresentationRate::new(f64::from(rate) / 1000.0).ok());
        if let Err(error) = self.app.runtime.set_presentation_rate(rate) {
            self.fail(event_loop, error.into());
            return None;
        }
        let now = self.start.elapsed();
        let mut next: Option<Duration> = None;
        #[cfg(test)]
        if self.smoke.is_some() {
            match self.smoke_step(event_loop) {
                Ok(deadline) => next = Some(next.map_or(deadline, |time| time.min(deadline))),
                Err(error) => {
                    self.fail(event_loop, error);
                    return None;
                }
            }
        }
        if self.active() && self.current.component_recovery.waiting(now) {
            next = match (next, self.current.component_recovery.retry_at) {
                (Some(a), Some(b)) => Some(a.min(b)),
                (a, b) => a.or(b),
            };
            if let Err(error) = self.app.runtime.service_inspection() {
                self.fail(event_loop, error.into());
                return None;
            }
        } else if !self.active() {
            // An occluded, lost or zero-sized window flushes nothing; answer
            // inspection from the last flush so a request is not stranded,
            // and report its captures unavailable rather than leaving them.
            if let Err(error) = self.app.runtime.service_inspection() {
                self.fail(event_loop, error.into());
                return None;
            }
            self.process_captures();
        } else {
            if self.current.retry_at.is_some_and(|time| time <= now) {
                self.current.retry_at = None;
            }
            // Coalesced motion is delivered once per turn while no redraw is
            // outstanding; during one, the redraw delivers it.
            if !self.current.redraw_pending
                && let Err(error) = self.current.motion.flush(&mut self.app.runtime)
            {
                self.fail(event_loop, error.into());
                return None;
            }
            let ready = updates_ready(
                &self.app.runtime,
                &self.current.root,
                self.current.redraw_pending,
            );
            if let Err(error) = prepare_updates(&mut self.app.runtime, now, ready) {
                self.fail(event_loop, error.into());
                return None;
            }
            // A capture admitted while the redraw is withheld was flushed for
            // above and is rendered here, so it does not wait on a compositor.
            self.process_captures();
            self.sync_cursor();
            // Only damage presents: an update or pointer motion that changed
            // nothing visible requests no frame.
            self.current.dirty |= self.app.runtime.needs_paint();
            if self.current.dirty
                && !self.current.redraw_pending
                && self.current.retry_at.is_none()
                && let Some(window) = &self.current.window
            {
                window.request_redraw();
                self.current.redraw_pending = true;
            }
            let scheduled = presentation_deadline(
                !self.current.redraw_pending,
                self.app.runtime.next_deadline(),
                self.current.retry_at,
            );
            next = match (next, scheduled) {
                (Some(a), Some(b)) => Some(a.min(b)),
                (a, b) => a.or(b),
            };
        }
        self.sync_ime();
        next
    }
}

#[cfg(test)]
mod scheduling_tests {
    use super::*;
    use crate::{Component, Runtime, UpdateContext};
    use std::{cell::Cell, rc::Rc};

    /// The structure `Runner::recover_device` uses: one device allowance for
    /// the application, one surface budget per window, and every window's
    /// budget restored on the recreated device.
    #[test]
    fn acquisition_failures_then_loss_notice_get_one_device_recovery_and_fresh_window_budgets()
    -> Result<(), AppError> {
        let root = Runtime::new().primary_window();
        let mut device = DeviceRecovery::default();
        let mut exhausted = NativeState::new(root.clone());
        let mut other = NativeState::new(root);
        // Unclassified acquisition failures give the queued loss notice time
        // to arrive; one window used its whole budget before it did.
        for delay in [16, 32, 48] {
            assert_eq!(
                exhausted.recovery.retry_delay()?,
                Duration::from_millis(delay)
            );
        }
        assert_eq!(other.recovery.retry_delay()?, Duration::from_millis(16));
        exhausted.surface_blocked = true;
        device.device_lost("device disappeared during acquisition".into())?;
        for window in [&mut exhausted, &mut other] {
            window.device_recreated();
            assert!(window.dirty && !window.surface_blocked && window.retry_at.is_none());
            for delay in [16, 32, 48] {
                assert_eq!(window.recovery.retry_delay()?, Duration::from_millis(delay));
            }
            assert!(matches!(
                window.recovery.retry_delay(),
                Err(AppError::Surface(_))
            ));
            window.recovery.presented();
            assert_eq!(window.recovery.retry_delay()?, Duration::from_millis(16));
        }
        assert!(matches!(
            device.device_lost("lost again".into()),
            Err(AppError::DeviceLost(_))
        ));
        Ok(())
    }

    /// Every hide path records the focused widget before the runtime clears
    /// focus, so the widget is focused again when the window shows.
    #[test]
    fn hiding_then_showing_a_window_restores_its_focused_widget() -> Result<(), AppError> {
        let mut runtime = Runtime::new();
        runtime.set_viewport(crate::Viewport::new(320.0, 100.0, 1.0)?)?;
        let text = runtime.signal(String::new());
        let field =
            runtime.mount_tree(crate::Element::new("field", crate::TextField::new(text)))?;
        runtime.flush(Duration::ZERO)?;
        runtime.focus(&field)?;
        let mut state = NativeState::new(runtime.primary_window());
        state.apply_visibility(&mut runtime, false)?;
        assert_eq!(runtime.focused(), None);
        assert_eq!(state.restore_focus.as_ref(), Some(&field));
        // A repeated hide (occlusion after a minimize) keeps the target.
        state.apply_visibility(&mut runtime, false)?;
        assert_eq!(state.restore_focus.as_ref(), Some(&field));
        state.apply_visibility(&mut runtime, true)?;
        runtime.flush(Duration::from_millis(1))?;
        state.focus(&mut runtime, true)?;
        assert_eq!(runtime.focused(), Some(field));
        Ok(())
    }

    struct Timed(Rc<Cell<u32>>);
    impl Component for Timed {
        fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
            Ok(())
        }
        fn timer_elapsed(&mut self, _: &mut crate::EventContext) -> Result<(), RuntimeError> {
            self.0.set(self.0.get() + 1);
            Ok(())
        }
    }
    /// A runtime whose focused widget armed a one-second timer while its
    /// window was being created, as application setup can, and the count of
    /// the timer's deliveries.
    fn timed_widget() -> Result<(Runtime, crate::WidgetId, Rc<Cell<u32>>), AppError> {
        let mut runtime = Runtime::new();
        runtime.set_viewport(crate::Viewport::new(320.0, 100.0, 1.0)?)?;
        let fired = Rc::new(Cell::new(0));
        let timed = runtime.mount_tree(crate::Element::new("timed", Timed(fired.clone())))?;
        runtime.flush(Duration::ZERO)?;
        runtime.event(&timed, |cx| cx.set_timer(Duration::from_secs(1)))??;
        assert_eq!(runtime.focused(), Some(timed.clone()));
        Ok((runtime, timed, fired))
    }

    /// macOS reports a new window's surface occluded until the window server
    /// first shows it. That pauses presentation without hiding the window, so
    /// focus and the timer armed during setup survive; once the window has
    /// been on screen, the same block is a hide that ends both.
    #[test]
    fn a_blocked_surface_hides_a_window_only_after_it_has_been_on_screen() -> Result<(), AppError> {
        let (mut runtime, timed, fired) = timed_widget()?;
        let mut state = NativeState::new(runtime.primary_window());
        state.size = PhysicalSize::new(320, 100);
        state.surface_blocked = true;
        assert!(state.presentation_blocked());
        assert!(!state.content_hidden());
        state.apply_visibility(&mut runtime, !state.content_hidden())?;
        assert_eq!(runtime.focused(), Some(timed.clone()));
        runtime.flush(Duration::from_secs(1))?;
        assert_eq!(fired.get(), 1, "the setup timer fired");

        runtime.event(&timed, |cx| cx.set_timer(Duration::from_secs(1)))??;
        state.exposed = true;
        assert!(state.content_hidden());
        state.apply_visibility(&mut runtime, !state.content_hidden())?;
        assert_eq!(runtime.focused(), None);
        assert_eq!(state.restore_focus.as_ref(), Some(&timed));
        state.surface_blocked = false;
        state.apply_visibility(&mut runtime, !state.content_hidden())?;
        runtime.flush(Duration::from_secs(3))?;
        assert_eq!(fired.get(), 1, "hiding cancelled the timer");
        Ok(())
    }

    /// Occlusion the window system reports hides a window whether or not it
    /// has been on screen yet; only a blocked surface depends on that.
    #[test]
    fn reported_occlusion_hides_a_window_before_it_has_been_on_screen() {
        let mut state = NativeState::new(Runtime::new().primary_window());
        state.size = PhysicalSize::new(320, 100);
        assert!(!state.content_hidden());
        state.occluded = true;
        assert!(state.content_hidden());
        state.occluded = false;
        state.size = PhysicalSize::new(0, 100);
        assert!(state.content_hidden());
    }

    /// winit's macOS backend announces a new window's initial focus state as
    /// a loss before the window becomes key. Without an earlier gain that is
    /// no loss: the widget the application focused keeps focus and its timer.
    /// A loss after a gain releases focus and cancels the timer.
    #[test]
    fn a_focus_loss_before_the_first_gain_keeps_focus_and_timers() -> Result<(), AppError> {
        let (mut runtime, timed, fired) = timed_widget()?;
        let mut state = NativeState::new(runtime.primary_window());
        assert!(!state.window_focused(&mut runtime, false, false)?);
        assert_eq!(runtime.focused(), Some(timed.clone()));
        runtime.flush(Duration::from_secs(1))?;
        assert_eq!(fired.get(), 1, "the setup timer fired");

        runtime.event(&timed, |cx| cx.set_timer(Duration::from_secs(1)))??;
        // The gain as the window's focus event records it; applying it here
        // would need a presentable window.
        state.window_focused = true;
        assert!(state.window_focused(&mut runtime, false, false)?);
        assert!(!state.window_focused);
        assert_eq!(runtime.focused(), None);
        assert_eq!(state.restore_focus.as_ref(), Some(&timed));
        runtime.flush(Duration::from_secs(3))?;
        assert_eq!(fired.get(), 1, "the loss cancelled the timer");
        Ok(())
    }

    /// On macOS the first redraw can find the surface blocked before the
    /// window reports its first focus gain. That gain must not clear the
    /// focus and cancel the timer the application set up: the test state has
    /// no surface, so it cannot present, as during that blocked startup.
    #[test]
    fn a_focus_gain_before_the_window_can_present_keeps_focus_and_timers() -> Result<(), AppError> {
        let (mut runtime, timed, fired) = timed_widget()?;
        let mut state = NativeState::new(runtime.primary_window());
        state.size = PhysicalSize::new(320, 100);
        assert!(!state.window_focused(&mut runtime, false, false)?);
        state.surface_blocked = true;
        state.apply_visibility(&mut runtime, !state.content_hidden())?;
        assert!(!state.presentable());
        assert!(!state.window_focused(&mut runtime, true, false)?);
        assert!(state.window_focused);
        assert_eq!(runtime.focused(), Some(timed.clone()));
        state.surface_blocked = false;
        state.apply_visibility(&mut runtime, !state.content_hidden())?;
        assert_eq!(runtime.focused(), Some(timed));
        runtime.flush(Duration::from_secs(1))?;
        assert_eq!(fired.get(), 1, "the setup timer fired");
        Ok(())
    }

    /// While the compositor withholds a redraw, an animating window does not
    /// flush, except for a capture job that targets it: that job is admitted,
    /// flushed for and rendered on the scheduling turn instead of waiting for
    /// a redraw that may never come.
    #[test]
    fn withheld_redraw_flushes_for_a_waiting_capture_only() -> Result<(), AppError> {
        let mut runtime = Runtime::new();
        runtime.set_capture_support(true);
        runtime.set_viewport(crate::Viewport::new(320.0, 100.0, 1.0)?)?;
        runtime.mount_tree(crate::Element::new("live", Live))?;
        let root = runtime.primary_window();
        assert_eq!(prepare_updates(&mut runtime, Duration::ZERO, true)?, 1);
        runtime.paint()?;
        let now = Duration::from_secs(1);
        let ready = updates_ready(&runtime, &root, true);
        assert!(!ready);
        assert_eq!(prepare_updates(&mut runtime, now, ready)?, 0);
        let _reply = runtime.request_capture(crate::CaptureRequest::new(
            crate::CaptureTarget::window(&root),
        ));
        let ready = updates_ready(&runtime, &root, true);
        assert!(ready);
        assert_eq!(prepare_updates(&mut runtime, now, ready)?, 1);
        let jobs = runtime.pending_captures();
        assert_eq!(jobs.len(), 1);
        assert!(runtime.capture_scene(&jobs[0]).is_ok());
        Ok(())
    }

    #[test]
    fn component_failures_back_off_and_successful_flush_does_not_reset_paint_budget() {
        let calls = std::rc::Rc::new(std::cell::Cell::new(0));
        let observed = calls.clone();
        let mut app = App::new("recovery", |_| Ok(())).on_error(move |ui, _| {
            assert!(ui.keyboard().pressed().next().is_none());
            observed.set(observed.get() + 1);
            crate::ErrorAction::Retry
        });
        let mut recovery = ComponentRecovery::default();
        let mut now = Duration::ZERO;
        for milliseconds in [100, 200, 400] {
            let deadline = recovery
                .handle(
                    &mut app,
                    AppError::Drawing(crate::DrawError::InvalidColor),
                    now,
                )
                .unwrap();
            assert_eq!(deadline - now, Duration::from_millis(milliseconds));
            assert!(recovery.waiting(now));
            assert!(recovery.waiting(deadline - Duration::from_nanos(1)));
            assert!(!recovery.waiting(deadline));
            app.runtime.flush(deadline).unwrap();
            now = deadline;
        }
        assert!(matches!(
            recovery.handle(
                &mut app,
                AppError::Drawing(crate::DrawError::InvalidColor),
                now
            ),
            Err(AppError::Drawing(crate::DrawError::InvalidColor))
        ));
        assert_eq!(calls.get(), 4);
        // A presented frame starts a new independent recovery allowance.
        recovery = ComponentRecovery::default();
        assert!(
            recovery
                .handle(
                    &mut app,
                    AppError::Runtime(RuntimeError::BorrowConflict),
                    now
                )
                .is_ok()
        );
    }

    #[test]
    fn custom_error_policy_can_repair_state_or_exit_with_original_error() {
        let mut app = App::new("repair", |_| Ok(()));
        let signal = app.runtime.signal(0);
        let repair = signal.clone();
        app = app.on_error(move |_, error| {
            assert!(matches!(
                error,
                AppError::Runtime(RuntimeError::BorrowConflict)
            ));
            repair.set(1).unwrap();
            crate::ErrorAction::Retry
        });
        let mut recovery = ComponentRecovery::default();
        recovery
            .handle(
                &mut app,
                AppError::Runtime(RuntimeError::BorrowConflict),
                Duration::ZERO,
            )
            .unwrap();
        assert_eq!(signal.get().unwrap(), 1);
        assert!(recovery.error.is_some()); // retained if close occurs during backoff
        app = app.on_error(|_, _| crate::ErrorAction::Exit);
        assert!(matches!(
            recovery.handle(
                &mut app,
                AppError::Runtime(RuntimeError::Exhausted),
                Duration::ZERO
            ),
            Err(AppError::Runtime(RuntimeError::Exhausted))
        ));
    }

    struct Live;
    impl Component for Live {
        fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
            cx.animate()?;
            Ok(())
        }
    }
    #[test]
    fn withheld_redraw_suspends_live_updates_until_compositor_resumes() -> Result<(), RuntimeError>
    {
        let mut runtime = Runtime::new();
        runtime.set_viewport(crate::Viewport::new(320.0, 100.0, 1.0)?)?;
        runtime.mount_tree(crate::Element::new("live", Live))?;
        assert_eq!(prepare_updates(&mut runtime, Duration::ZERO, true)?, 1);
        let pending = runtime.next_deadline();
        assert!(pending.is_some());
        for seconds in 1..=30 {
            assert_eq!(
                prepare_updates(&mut runtime, Duration::from_secs(seconds), false)?,
                0
            );
            assert_eq!(
                presentation_deadline(false, runtime.next_deadline(), None),
                None
            );
            assert_eq!(runtime.next_deadline(), pending);
        }
        // One current update, no catch-up frames for the elapsed hidden interval.
        let now = Duration::from_secs(31);
        assert_eq!(prepare_updates(&mut runtime, now, true)?, 1);
        assert!(runtime.next_deadline().is_some_and(|time| time > now));
        Ok(())
    }
}
