//! A screenshot taken from the real native renderer and checked pixel by pixel.
//!
//! The smoke opens an event loop, so it runs in its own process with `--exact`;
//! see docs/verification.md §Native evidence. It is evidence for the platform and
//! adapter it ran on: that a capture request from another thread wakes the
//! host, that the frame comes from the same renderer that presents, that the
//! crop follows the target's physical bounds, and that the PNG is well formed.
use super::*;
use crate::*;
use std::sync::mpsc;

/// The accent rectangle drawn by the captured component, in logical pixels.
const SQUARE: f32 = 60.0;
/// The tool must have both captures within this time, or the check fails
/// rather than sleeping on.
const DEADLINE: Duration = Duration::from_secs(20);

struct Square;
impl Component for Square {
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
    fn measure(&self, _: &MeasureContext<'_>, _: Constraints) -> Result<Size, RuntimeError> {
        Ok(Size::from([SQUARE as u16, SQUARE as u16]))
    }
    fn paint(&self, p: &mut Painter) -> Result<(), DrawError> {
        p.fill_rect(Rect::new(0.0, 0.0, SQUARE, SQUARE)?, Color::ACCENT);
        Ok(())
    }
}

/// A capture as the tool thread received it, or the first failure it met,
/// named after the step that failed.
type CaptureReply = Result<Capture, String>;

struct Driver {
    runner: Runner,
    /// Replies from the tool thread: the window capture, then the square capture.
    replies: mpsc::Receiver<CaptureReply>,
    received: Vec<Capture>,
    /// Tells the tool that the window has presented its first frame; taken
    /// when sent.
    presented: Option<mpsc::Sender<()>>,
    result: Option<Result<(), AppError>>,
}
impl Driver {
    /// Collects what the tool has sent without blocking the UI thread, which
    /// the tool's requests need for their rendering.
    fn check(&mut self, event_loop: &ActiveEventLoop) -> Result<(), AppError> {
        if self.runner.current.frames > 0
            && let Some(presented) = self.presented.take()
        {
            // The tool may already have failed and gone; its reply says why.
            let _ = presented.send(());
        }
        while let Ok(reply) = self.replies.try_recv() {
            self.received
                .push(reply.map_err(|error| AppError::Graphics(format!("capture tool: {error}")))?);
        }
        if self.received.len() < 2 {
            if self.runner.start.elapsed() > DEADLINE {
                return Err(AppError::Graphics(format!(
                    "only {} of 2 captures arrived within {DEADLINE:?}",
                    self.received.len()
                )));
            }
            return Ok(());
        }
        let square = self
            .received
            .pop()
            .ok_or_else(|| AppError::Graphics("no square reply".into()))?;
        let window = self
            .received
            .pop()
            .ok_or_else(|| AppError::Graphics("no window reply".into()))?;
        let viewport = self.runner.app.runtime.inspect().windows[0].viewport;
        let extent = viewport
            .physical()
            .ok_or_else(|| AppError::Graphics("the native window has no physical extent".into()))?;
        eprintln!(
            "Capture on scale {}: window {}×{} physical, square region {:?} → {}×{} pixels, PNG {} bytes",
            viewport.scale(),
            window.image.width(),
            window.image.height(),
            square.region,
            square.image.width(),
            square.image.height(),
            square.image.encode_png().len()
        );
        assert_eq!(
            (window.image.width(), window.image.height()),
            (extent.width(), extent.height()),
            "the window capture is the whole frame"
        );
        assert!(!window.overlay && !window.clipped && window.component.is_none());
        // The square sits at the window's origin; its capture is accent-colored
        // throughout, its size the logical square times the scale, and the
        // window frame carries the same pixels at that place.
        let expected = (f64::from(SQUARE) * viewport.scale()).ceil() as u32;
        assert!(
            square.image.width().abs_diff(expected) <= 1
                && square.image.height().abs_diff(expected) <= 1,
            "square capture is {}×{}, expected about {expected}",
            square.image.width(),
            square.image.height()
        );
        let accent = |pixel: &[u8]| {
            // Linear accent converts to sRGB [80, 153, 237] within a step or two.
            pixel[0].abs_diff(80) <= 3 && pixel[1].abs_diff(153) <= 3 && pixel[2].abs_diff(237) <= 3
        };
        let inner = |image: &FrameImage, x: u32, y: u32| -> Vec<u8> {
            let start = ((y * image.width() + x) * 4) as usize;
            image.pixels()[start..start + 4].to_vec()
        };
        let middle = square.image.width() / 2;
        assert!(
            accent(&inner(&square.image, 2, 2)),
            "{:?}",
            inner(&square.image, 2, 2)
        );
        assert!(accent(&inner(&square.image, middle, middle)));
        assert!(accent(&inner(&window.image, middle, middle)));
        let beyond = expected + 4;
        assert!(
            !accent(&inner(&window.image, beyond, beyond)),
            "the background beyond the square is not accent"
        );
        let background = inner(&window.image, beyond, beyond);
        assert!(
            background[..3]
                .iter()
                .all(|channel| channel.abs_diff(188) <= 2)
                && background[3].abs_diff(128) <= 1,
            "linear-premultiplied translucent white capture: {background:?}"
        );
        #[cfg(feature = "png")]
        {
            let decoded = crate::images::decode(&window.image.encode_png())
                .map_err(|error| AppError::Graphics(format!("translucent capture PNG: {error}")))?;
            assert_eq!(
                (decoded.width(), decoded.height()),
                (window.image.width(), window.image.height())
            );
            assert!(
                decoded
                    .pixels()
                    .iter()
                    .zip(window.image.pixels())
                    .all(|(actual, original)| actual.abs_diff(*original) <= 2),
                "real capture PNG restores linear-premultiplied pixels"
            );
        }
        // The PNG is well formed here; that it decodes to the same pixels is
        // checked with a real decoder when the `png` feature provides one
        // (and headlessly in tests/foundation/capture.rs).
        let png = square.image.encode_png();
        assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n");
        assert!(
            png.len() > square.image.pixels().len(),
            "stored blocks add framing"
        );
        #[cfg(feature = "png")]
        {
            let decoded = crate::images::decode(&png)
                .map_err(|error| AppError::Graphics(format!("capture PNG: {error}")))?;
            assert_eq!(
                (decoded.width(), decoded.height()),
                (square.image.width(), square.image.height())
            );
            // The accent square is opaque, so premultiplying keeps its pixels.
            assert_eq!(decoded.pixels(), square.image.pixels());
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
        // The deadline is checked on the loop's own turns; keep them coming.
        if self.result.is_none() {
            event_loop.set_control_flow(ControlFlow::WaitUntil(
                Instant::now() + Duration::from_millis(250),
            ));
        }
    }
}

#[test]
#[ignore = "requires a native desktop session and hardware GPU adapter"]
fn a_tool_thread_captures_the_window_and_a_widget_from_the_real_renderer() -> Result<(), AppError> {
    capture_smoke(false)
}

/// The same check with software presentation: the window presents CPU frames
/// through softbuffer, creates no GPU device, and its captures follow the
/// same contract.
#[cfg(feature = "cpu")]
#[test]
#[ignore = "requires a native desktop session"]
fn software_presentation_captures_the_window_and_a_widget() -> Result<(), AppError> {
    capture_smoke(true)
}

fn capture_smoke(software: bool) -> Result<(), AppError> {
    let mut app = App::new("pert-ui capture smoke", |_| Ok(())).size(Size::from([320, 240]))?;
    app.software = software;
    app.initialize.take();
    app.runtime.theme().set_mode(ThemeMode::Dark)?;
    app.runtime.theme().set_color(
        ThemeMode::Dark,
        ColorRole::Background,
        Color::from_linear(1.0, 1.0, 1.0, 0.5)?,
    )?;
    app.runtime.mount_tree(Element::new(
        "canvas",
        Column::new([Element::new("square", Square)]),
    ))?;
    let service = app
        .runtime
        .inspection_service(Capacity::new(4).map_err(|e| AppError::Graphics(e.to_string()))?)?;
    let client = service.client();
    let event_loop = native_test_event_loop()?;
    let wake = event_loop.create_proxy();
    let (send, replies) = mpsc::channel();
    let (presented, first_frame) = mpsc::channel();
    // The tool waits for a first snapshot to learn the window and for the
    // first presented frame, then asks for both captures; every request wakes
    // the host from this thread, and each reply or failure it forwards wakes
    // the loop again so the check runs and reports it.
    let tool = std::thread::spawn(move || {
        let capture = |request: CaptureRequest| -> Result<Capture, String> {
            let pending = client
                .capture(request)
                .map_err(|error| format!("capture request: {error}"))?;
            let reply = pending
                .wait()
                .map_err(|error| format!("capture reply: {error}"))?;
            reply.value.map_err(|error| format!("capture: {error}"))
        };
        let run = || -> Result<(), String> {
            let snapshot = client
                .snapshot()
                .map_err(|error| format!("snapshot request: {error}"))?
                .wait()
                .map_err(|error| format!("snapshot reply: {error}"))?;
            let window = snapshot
                .value
                .windows
                .first()
                .ok_or("the snapshot lists no window")?
                .id;
            first_frame.recv_timeout(DEADLINE).map_err(|error| {
                format!(
                    "the first frame was not presented ({error}); {}",
                    super::smoke::WITHHELD
                )
            })?;
            for request in [
                CaptureRequest::new(CaptureTarget::Window(window)),
                CaptureRequest::new(CaptureTarget::Path(vec!["canvas".into(), "square".into()])),
            ] {
                let reply = capture(request);
                let failed = reply.is_err();
                send.send(reply).map_err(|_| "the host stopped listening")?;
                let _ = wake.send_event(UserEvent::Wake);
                if failed {
                    break;
                }
            }
            Ok(())
        };
        if let Err(error) = run() {
            let _ = send.send(Err(error));
            let _ = wake.send_event(UserEvent::Wake);
        }
    });
    let runner = Runner::new(app, event_loop.create_proxy());
    let mut driver = Driver {
        runner,
        replies,
        received: Vec::new(),
        presented: Some(presented),
        result: None,
    };
    // The service must outlive the loop, or every request is cancelled.
    let outcome = event_loop
        .run_app(&mut driver)
        .map_err(|error| AppError::EventLoop(error.to_string()));
    // Dropping the service cancels what the tool still waits for, and
    // dropping the frame sender ends its wait for a first frame, so it ends.
    drop(service);
    drop(driver.presented.take());
    tool.join()
        .map_err(|_| AppError::Graphics("the capture tool thread panicked".into()))?;
    outcome?;
    if let Some(error) = driver.runner.error {
        return Err(error);
    }
    let presenter = match &driver.runner.current.renderer {
        #[cfg(feature = "gpu")]
        Some(super::Presenter::Gpu(_)) => "gpu",
        #[cfg(feature = "cpu")]
        Some(super::Presenter::Software(_)) => "software",
        None => "none",
    };
    eprintln!("Presenter: {presenter}");
    // Without the GPU renderer every window presents in software.
    let expected = if software || cfg!(not(feature = "gpu")) {
        "software"
    } else {
        "gpu"
    };
    if presenter != expected {
        return Err(AppError::Graphics(format!(
            "the window was presented by the {presenter} presenter"
        )));
    }
    driver
        .result
        .ok_or_else(|| AppError::Graphics("capture smoke exited before verification".into()))?
}
