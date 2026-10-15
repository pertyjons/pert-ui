//! Experimental native event recorder for the multi-window input design; not a
//! pert-ui API and not a native check with a pass/fail result.
//!
//! It lives in this unpublished package so that installing the library never
//! installs it. See `docs/verification.md` for how to run it and what to
//! archive with a log. Run on the process main thread. This intentionally does not route input to a
//! runtime or infer application activation from window focus or batch boundaries.
use pert_ui::{Column, Element, Label, PhysicalExtent, Runtime, Viewport};
use pert_ui_wgpu::{Frame, Notice, Renderer};
use std::{
    error::Error,
    fs::File,
    io::{BufWriter, Write},
    sync::Arc,
    time::{Duration, Instant},
};
use winit::{
    application::ApplicationHandler,
    dpi::{LogicalPosition, LogicalSize},
    event::{DeviceEvent, DeviceId, StartCause, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop, EventLoopProxy},
    platform::modifier_supplement::KeyEventExtModifierSupplement,
    window::{Window, WindowId},
};

type ProbeResult<T = ()> = Result<T, Box<dyn Error>>;

struct ProbeWindow {
    identity: usize,
    native: Arc<Window>,
    renderer: Renderer,
    presented: u64,
}

struct Probe {
    windows: Vec<ProbeWindow>,
    // Only paints the static probe instructions. No focus/input model is tested
    // by this runtime, and the probe's separate renderers are not shared-GPU proof.
    content: Runtime,
    proxy: EventLoopProxy<(usize, Notice)>,
    log: BufWriter<File>,
    start: Instant,
    duration: Duration,
    batch: u64,
    sequence: u64,
    initialized: bool,
    error: Option<Box<dyn Error>>,
}

impl Probe {
    fn record(&mut self, window: Option<usize>, event: impl std::fmt::Display) -> ProbeResult {
        self.sequence += 1;
        writeln!(
            self.log,
            "seq={} us={} batch={} window={window:?} {event}",
            self.sequence,
            self.start.elapsed().as_micros(),
            self.batch,
        )?;
        Ok(())
    }

    fn fail(&mut self, event_loop: &ActiveEventLoop, result: ProbeResult) {
        if let Err(error) = result {
            if self.error.is_none() {
                self.error = Some(error);
            }
            event_loop.exit();
        }
    }

    fn open(&mut self, event_loop: &ActiveEventLoop) -> ProbeResult {
        if self.initialized {
            return self.record(None, "resumed-again");
        }
        self.initialized = true;
        self.record(
            None,
            "resumed; application_activation_evidence=unavailable-in-winit",
        )?;
        for index in 0..2 {
            let native = Arc::new(
                event_loop.create_window(
                    Window::default_attributes()
                        .with_title(format!(
                            "pert-ui focus probe {} — see terminal instructions",
                            index + 1
                        ))
                        .with_inner_size(LogicalSize::new(540.0, 240.0)),
                )?,
            );
            native.set_ime_allowed(true);
            native.set_ime_cursor_area(
                LogicalPosition::new(20.0, 100.0),
                LogicalSize::new(400.0, 20.0),
            );
            let size = native.inner_size();
            let proxy = self.proxy.clone();
            let renderer = Renderer::new(native.clone(), size.width, size.height, move |notice| {
                // The closed event loop owns no resources that need this notice.
                let _ = proxy.send_event((index, notice));
            })?;
            self.record(Some(index), format!(
                "created native={:?} size={size:?} scale={} focus={} visible={:?} minimized={:?}",
                native.id(), native.scale_factor(), native.has_focus(),
                native.is_visible(), native.is_minimized(),
            ))?;
            native.request_redraw();
            self.windows.push(ProbeWindow {
                identity: index,
                native,
                renderer,
                presented: 0,
            });
        }
        Ok(())
    }

    fn event(
        &mut self,
        event_loop: &ActiveEventLoop,
        id: WindowId,
        event: WindowEvent,
    ) -> ProbeResult {
        let Some(slot) = self
            .windows
            .iter()
            .position(|window| window.native.id() == id)
        else {
            return self.record(
                None,
                format!("unknown-window native={id:?} event={event:?}"),
            );
        };
        let index = self.windows[slot].identity;
        // Debug output includes native repeat/synthetic flags, logical/physical
        // keys, modifiers, text/IME, focus, size, scale and occlusion verbatim.
        self.record(Some(index), format!("event={event:?}"))?;
        if let WindowEvent::KeyboardInput { event, .. } = &event {
            self.record(
                Some(index),
                format!("key_without_modifiers={:?}", event.key_without_modifiers()),
            )?;
        }
        match event {
            WindowEvent::CloseRequested => {
                // Keep the other window alive to observe destruction/focus order.
                self.windows.remove(slot);
                self.record(Some(index), "native-resources-dropped")?;
                if self.windows.is_empty() {
                    event_loop.exit();
                }
            }
            WindowEvent::RedrawRequested => {
                let window = &mut self.windows[slot];
                let size = window.native.inner_size();
                if size.width == 0 || size.height == 0 {
                    return self.record(Some(index), "presentation-skipped-zero-size");
                }
                window.renderer.resize(size.width, size.height)?;
                let scale = window.native.scale_factor();
                self.content.set_viewport(Viewport::from_physical(
                    PhysicalExtent::new(size.width, size.height),
                    scale,
                )?)?;
                self.content.flush(self.start.elapsed())?;
                let outcome = window
                    .renderer
                    .render(self.content.paint()?, || window.native.pre_present_notify())?;
                let outcome = match outcome {
                    Frame::Presented => {
                        window.presented += 1;
                        "presented"
                    }
                    Frame::Retry => "retry-required-no-automatic-probe-recovery",
                    Frame::Occluded => "occluded",
                    Frame::Recreate => "surface-recreation-required-no-automatic-probe-recovery",
                };
                self.record(Some(index), outcome)?;
            }
            WindowEvent::Resized(_) | WindowEvent::ScaleFactorChanged { .. } => {
                self.windows[slot].native.request_redraw();
            }
            WindowEvent::Focused(_) | WindowEvent::Occluded(_) => {
                let native = &self.windows[slot].native;
                self.record(Some(index), format!(
                    "observation focus={} visible={:?} minimized={:?}; application_activation=unknown",
                    native.has_focus(), native.is_visible(), native.is_minimized(),
                ))?;
            }
            _ => {}
        }
        Ok(())
    }
}

impl ApplicationHandler<(usize, Notice)> for Probe {
    fn new_events(&mut self, event_loop: &ActiveEventLoop, cause: StartCause) {
        self.batch += 1;
        let result = self.record(None, format!("new_events={cause:?}"));
        self.fail(event_loop, result);
    }

    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let result = self.open(event_loop);
        self.fail(event_loop, result);
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
        let result = self.event(event_loop, id, event);
        self.fail(event_loop, result);
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, (window, notice): (usize, Notice)) {
        let message = match notice {
            Notice::Lost(message) => format!("device-lost: {message}"),
            Notice::Error(message) => format!("gpu-error: {message}"),
        };
        let result = self.record(Some(window), &message);
        self.fail(event_loop, result);
        self.fail(event_loop, Err(message.into()));
    }

    fn device_event(&mut self, event_loop: &ActiveEventLoop, id: DeviceId, event: DeviceEvent) {
        let result = self.record(None, format!("device={id:?} event={event:?}"));
        self.fail(event_loop, result);
    }

    fn suspended(&mut self, event_loop: &ActiveEventLoop) {
        let result = self.record(None, "suspended (not inferred from focus)");
        self.fail(event_loop, result);
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        let result = (|| {
            self.record(None, "about_to_wait")?;
            self.log.flush()?;
            if self.start.elapsed() >= self.duration {
                self.record(None, "probe-duration-elapsed (not an activation decision)")?;
                event_loop.exit();
            } else {
                event_loop.set_control_flow(ControlFlow::WaitUntil(self.start + self.duration));
            }
            Ok(())
        })();
        self.fail(event_loop, result);
    }

    fn exiting(&mut self, event_loop: &ActiveEventLoop) {
        let result = (|| {
            for slot in 0..self.windows.len() {
                let index = self.windows[slot].identity;
                self.record(
                    Some(index),
                    format!("exit presented_frames={}", self.windows[slot].presented),
                )?;
            }
            self.record(None, "exiting")?;
            self.log.flush()?;
            Ok(())
        })();
        self.fail(event_loop, result);
    }
}

fn main() -> ProbeResult {
    let mut args = std::env::args_os().skip(1);
    let path = args
        .next()
        .ok_or("usage: pert-ui-focus-probe NEW_LOG_PATH [SECONDS=60]")?;
    let seconds = args
        .next()
        .map(|value| value.to_string_lossy().parse::<u64>())
        .transpose()?
        .unwrap_or(60);
    if args.next().is_some() || !(1..=600).contains(&seconds) {
        return Err("expected a new log path and duration of 1..=600 seconds".into());
    }
    let log = BufWriter::new(File::create_new(path)?);
    let event_loop = EventLoop::<(usize, Notice)>::with_user_event().build()?;
    let mut content = Runtime::new();
    content.mount_tree(Element::new(
        "instructions",
        Column::new([
            Element::new(
                "title",
                Label::new("Native focus/input probe — no application input routing"),
            ),
            Element::new(
                "transfer",
                Label::new("Hold A or Shift; click the other probe window; release."),
            ),
            Element::new(
                "deactivate",
                Label::new("Repeat while switching to another app, then return."),
            ),
            Element::new(
                "visibility",
                Label::new("Minimize/restore; test repeat, layouts and IME."),
            ),
            Element::new(
                "close",
                Label::new("Close one window to observe focus; close both to finish."),
            ),
        ]),
    ))?;
    let mut probe = Probe {
        windows: Vec::new(),
        content,
        proxy: event_loop.create_proxy(),
        log,
        start: Instant::now(),
        duration: Duration::from_secs(seconds),
        batch: 0,
        sequence: 0,
        initialized: false,
        error: None,
    };
    probe.record(
        None,
        format!(
            // Dependency versions are not known to the binary; archive the
            // resolved ones from `Cargo.lock` with the log.
            "probe-format=1 os={} arch={} duration_seconds={seconds}",
            std::env::consts::OS,
            std::env::consts::ARCH
        ),
    )?;
    writeln!(
        std::io::stderr(),
        "Focus probe: two windows for {seconds}s. Hold A/Shift, transfer between probe windows, switch applications, minimize/restore, and close one window. The log contains raw keys/text delivered to this probe. A successful exit only records events; it does not validate continuity."
    )?;
    event_loop.run_app(&mut probe)?;
    if let Some(error) = probe.error {
        return Err(error);
    }
    Ok(())
}
