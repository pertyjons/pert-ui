//! Native animation pacing against the monitor's reported presentation rate.
//!
//! The smoke opens an event loop, so it runs in its own process with `--exact`;
//! see docs/verification.md §Native evidence. It is evidence for the display it
//! ran on: a 60 Hz monitor exercises only the fallback ceiling.
use super::*;
use crate::*;
use std::{cell::RefCell, rc::Rc};

/// Settling time before measuring, for the first frames and monitor discovery.
const WARM_UP: Duration = Duration::from_secs(1);
/// Length of the measured interval.
const MEASURED: Duration = Duration::from_secs(3);

struct Animated {
    updates: Rc<RefCell<Vec<Instant>>>,
    phase: f32,
}
impl Component for Animated {
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        self.updates.borrow_mut().push(Instant::now());
        self.phase = (cx.now().as_secs_f32() % 1.0) * 100.0;
        cx.animate()?;
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
    updates: Rc<RefCell<Vec<Instant>>>,
    /// Wall-clock start, update count and presentation count when measuring began.
    baseline: Option<(Instant, usize, u64)>,
    result: Option<Result<(), AppError>>,
}
impl Driver {
    fn check(&mut self, event_loop: &ActiveEventLoop) -> Result<(), AppError> {
        let elapsed = self.runner.start.elapsed();
        let frames = self.runner.current.frames;
        let Some((started, updates, presented)) = self.baseline else {
            if elapsed >= WARM_UP {
                self.baseline = Some((Instant::now(), self.updates.borrow().len(), frames));
            }
            return Ok(());
        };
        if started.elapsed() < MEASURED {
            return Ok(());
        }
        let seconds = started.elapsed().as_secs_f64();
        let times = self.updates.borrow();
        let measured = &times[updates..];
        let update_rate = measured.len() as f64 / seconds;
        let frame_rate = (frames - presented) as f64 / seconds;
        let mut intervals: Vec<_> = measured
            .windows(2)
            .map(|pair| (pair[1] - pair[0]).as_secs_f64() * 1000.0)
            .collect();
        intervals.sort_by(f64::total_cmp);
        let percentile = |p: f64| {
            intervals
                .get(((intervals.len() as f64 - 1.0) * p).round() as usize)
                .copied()
                .unwrap_or(f64::NAN)
        };
        let rate = self.runner.app.host.runtime.presentation_rate();
        let monitor = self
            .runner
            .current
            .window
            .as_ref()
            .and_then(|window| window.current_monitor())
            .and_then(|monitor| monitor.name());
        eprintln!(
            "Pacing on {monitor:?} at a reported {:?} Hz: {update_rate:.1} updates/s, \
             {frame_rate:.1} presentations/s over {seconds:.2} s; interval ms \
             median {:.2}, p95 {:.2}, max {:.2}",
            rate.map(PresentationRate::hertz),
            percentile(0.5),
            percentile(0.95),
            percentile(1.0),
        );
        // The ceiling is the reported rate, or the 60 Hz fallback when unknown.
        let ceiling = rate.map_or(60.0, PresentationRate::hertz);
        assert!(
            update_rate <= ceiling * 1.05 && frame_rate <= ceiling * 1.05,
            "animation exceeded the {ceiling} Hz ceiling"
        );
        // Above the fallback, pacing must follow the monitor, not stay at 60 Hz.
        if ceiling > 66.0 {
            assert!(
                update_rate > 66.0 && frame_rate > 66.0,
                "a {ceiling} Hz monitor paced at the 60 Hz fallback"
            );
        } else {
            eprintln!("The reported rate does not exceed 60 Hz; only the fallback was exercised.");
        }
        event_loop.exit();
        self.result = Some(Ok(()));
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
    }
}
#[test]
#[ignore = "requires a native desktop session and hardware GPU adapter"]
fn animation_follows_the_monitor_presentation_rate() -> Result<(), AppError> {
    let mut app = App::new("pert-ui pacing smoke", |_| Ok(()));
    let updates = Rc::new(RefCell::new(Vec::new()));
    app.host.runtime.mount_tree(Element::new(
        "animated",
        Animated {
            updates: updates.clone(),
            phase: 0.0,
        },
    ))?;
    let event_loop = native_test_event_loop()?;
    let runner = Runner::new(app.start()?.into(), event_loop.create_proxy());
    let mut driver = Driver {
        runner,
        updates,
        baseline: None,
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
        .ok_or_else(|| AppError::Graphics("pacing smoke exited before verification".into()))?
}
