//! Follow-up 1: presentation to a window surface, without read-back.
//!
//! `vello-spike present` opens one window per renderer and scale (1280×800
//! and 2560×1600 physical requested), presents every workload with frames
//! paced to one per `PERIOD`, and times the UI thread's part of each frame.
//! pert-ui wgpu: compose, then `Renderer::render`, with the time until its
//! `before_present` callback (acquire + prepare + encode + submit). vello_gpu,
//! on its own device and surface configured like pert-ui's (Fifo, desired
//! maximum frame latency 2, opaque alpha) but with a non-sRGB 8-bit format,
//! because vello_gpu writes sRGB-encoded values: encode, acquire, prepare
//! (`Renderer::render` recording), submit, present; then, outside the UI
//! frame, a blocking wait for that submission gives its GPU completion time.
use crate::{Samples, WORKLOADS, Workload, ms, pertui, vello, workloads};
use pert_ui::PhysicalExtent;
use pert_ui_wgpu::{Frame, Notice, Renderer};
use std::fmt::Write as _;
use std::sync::Arc;
use std::time::{Duration, Instant};
use winit::application::ApplicationHandler;
use winit::dpi::PhysicalSize;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::platform::pump_events::EventLoopExtPumpEvents;
use winit::window::{Window, WindowId};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

const PERIOD: Duration = Duration::from_millis(30);
const WARM: u64 = 20;

#[derive(Default)]
struct App {
    want: Option<PhysicalSize<u32>>,
    window: Option<Arc<Window>>,
    size: PhysicalSize<u32>,
    error: Option<String>,
    monitors: String,
}

impl App {
    fn create(&mut self, event_loop: &ActiveEventLoop) {
        let Some(size) = self.want.take() else { return };
        if self.monitors.is_empty() {
            self.monitors = event_loop
                .available_monitors()
                .map(|m| {
                    format!(
                        "{} {}×{} @ {:.3} Hz, scale factor {}",
                        m.name().unwrap_or_default(),
                        m.size().width,
                        m.size().height,
                        f64::from(m.refresh_rate_millihertz().unwrap_or(0)) / 1000.0,
                        m.scale_factor()
                    )
                })
                .collect::<Vec<_>>()
                .join("; ");
        }
        match event_loop.create_window(
            Window::default_attributes()
                .with_title("vello spike presentation")
                .with_inner_size(size)
                .with_resizable(false),
        ) {
            Ok(window) => {
                self.size = window.inner_size();
                self.window = Some(Arc::new(window));
            }
            Err(e) => self.error = Some(e.to_string()),
        }
    }
}

impl ApplicationHandler<Notice> for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        self.create(event_loop);
    }
    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        self.create(event_loop);
    }
    fn window_event(&mut self, _: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        if let WindowEvent::Resized(size) = event {
            self.size = size;
        }
    }
}

fn pump(event_loop: &mut EventLoop<Notice>, app: &mut App, for_: Duration) {
    let end = Instant::now() + for_;
    loop {
        let _ = event_loop.pump_app_events(Some(Duration::from_millis(2)), app);
        if Instant::now() >= end {
            break;
        }
    }
}

fn pace(started: Instant) {
    if let Some(rest) = PERIOD.checked_sub(started.elapsed()) {
        std::thread::sleep(rest);
    }
}

/// vello_gpu on a window surface.
struct VelloSurface {
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
    renderer: vello_gpu::Renderer,
    resources: vello_gpu::Resources,
    depth: wgpu::TextureView,
    scene: vello_gpu::Scene,
    info: String,
}

impl VelloSurface {
    fn new(window: Arc<Window>, width: u32, height: u32) -> Result<Self> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::VULKAN,
            ..wgpu::InstanceDescriptor::new_without_display_handle()
        });
        let surface = instance.create_surface(window)?;
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            ..Default::default()
        }))?;
        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("vello_gpu presentation"),
            required_limits: wgpu::Limits::default().using_resolution(adapter.limits()),
            ..Default::default()
        }))?;
        let caps = surface.get_capabilities(&adapter);
        // vello_gpu's shaders write sRGB-encoded values, so the surface must
        // store them unchanged: an 8-bit non-sRGB format.
        let format = [wgpu::TextureFormat::Bgra8Unorm, wgpu::TextureFormat::Rgba8Unorm]
            .into_iter()
            .find(|f| caps.formats.contains(f))
            .ok_or("no 8-bit non-sRGB surface format")?;
        let alpha_mode = [wgpu::CompositeAlphaMode::Opaque, wgpu::CompositeAlphaMode::PreMultiplied]
            .into_iter()
            .find(|m| caps.alpha_modes.contains(m))
            .ok_or("no opaque or premultiplied alpha")?;
        let mut config = surface.get_default_config(&adapter, width, height).ok_or("no surface config")?;
        config.format = format;
        config.alpha_mode = alpha_mode;
        config.present_mode = wgpu::PresentMode::Fifo;
        config.desired_maximum_frame_latency = 2;
        surface.configure(&device, &config);
        let (w16, h16) = (width as u16, height as u16);
        let (renderer, resources) = vello_gpu::Renderer::new(
            &device,
            &vello_gpu::RenderTargetConfig { format, width: w16, height: h16 },
        );
        let depth = vello_gpu::Renderer::create_depth_texture_view(
            &device,
            &vello_gpu::RenderSize { width: w16, height: h16 },
        );
        let info = format!(
            "surface format {format:?} (formats offered: {:?}), alpha {alpha_mode:?}, present mode Fifo, desired maximum frame latency 2",
            caps.formats
        );
        Ok(Self {
            device,
            queue,
            surface,
            config,
            renderer,
            resources,
            depth,
            scene: vello_gpu::Scene::new(w16, h16),
            info,
        })
    }

    /// One frame: returns [encode, acquire, prepare, submit, present] and
    /// the GPU completion time measured from submit, or `None` when the
    /// surface gave no texture.
    fn frame(&mut self, frame: &vello::Frame<'_>, scale: f64) -> Result<Option<([Duration; 5], Duration)>> {
        let t = Instant::now();
        self.scene.reset();
        vello::encode(&mut self.scene, frame, scale);
        let encode = t.elapsed();
        let t = Instant::now();
        let texture = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(t) | wgpu::CurrentSurfaceTexture::Suboptimal(t) => t,
            wgpu::CurrentSurfaceTexture::Outdated => {
                self.surface.configure(&self.device, &self.config);
                return Ok(None);
            }
            _ => return Ok(None),
        };
        let acquire = t.elapsed();
        let t = Instant::now();
        let view = texture.texture.create_view(&Default::default());
        let mut encoder = self.device.create_command_encoder(&Default::default());
        let size = vello_gpu::RenderSize { width: self.config.width as u16, height: self.config.height as u16 };
        self.renderer
            .render(
                &self.scene,
                &mut self.resources,
                &self.device,
                &self.queue,
                &mut encoder,
                &size,
                &view,
                Some(&self.depth),
                &vello_gpu::TextureBindings::new(),
                vello_gpu::TargetInit::Clear(vello_gpu::ClearSettings::default()),
            )
            .map_err(|e| format!("vello_gpu render: {e:?}"))?;
        let commands = encoder.finish();
        let prepare = t.elapsed();
        let t = Instant::now();
        let submitted = Instant::now();
        let index = self.queue.submit([commands]);
        let submit = t.elapsed();
        let t = Instant::now();
        self.queue.present(texture);
        let present = t.elapsed();
        self.device.poll(wgpu::PollType::Wait { submission_index: Some(index), timeout: None })?;
        let gpu = submitted.elapsed();
        Ok(Some(([encode, acquire, prepare, submit, present], gpu)))
    }
}

fn cell(s: &Samples) -> String {
    s.cell()
}

pub fn run() -> Result<()> {
    let mut event_loop = EventLoop::<Notice>::with_user_event().build()?;
    let mut app = App::default();
    let frames: u64 = std::env::var("SPIKE_FRAMES").ok().and_then(|f| f.parse().ok()).unwrap_or(150);
    let mut out = String::new();
    let mut notes = String::new();
    pump(&mut event_loop, &mut app, Duration::from_millis(100));
    for scale in [1.0_f64, 2.0] {
        let requested = PhysicalSize::new((1280.0 * scale) as u32, (800.0 * scale) as u32);
        for renderer in ["pert-ui wgpu", "vello_gpu"] {
            app.want = Some(requested);
            pump(&mut event_loop, &mut app, Duration::from_millis(800));
            if let Some(e) = app.error.take() {
                return Err(e.into());
            }
            let window = app.window.clone().ok_or("no window")?;
            let size = app.size;
            writeln!(
                notes,
                "- {renderer}, scale {scale}: requested {}×{}, got {}×{} physical (window scale factor {}).",
                requested.width,
                requested.height,
                size.width,
                size.height,
                window.scale_factor()
            )?;
            let logical = (size.width as f32 / scale as f32, size.height as f32 / scale as f32);
            if renderer == "pert-ui wgpu" {
                let mut r = Renderer::new(
                    window.clone(),
                    PhysicalExtent::new(size.width, size.height),
                    |notice| eprintln!("renderer notice: {notice:?}"),
                )?;
                for w in WORKLOADS {
                    eprintln!("{renderer} {} scale {scale}", w.name);
                    let row = pert_frames(&mut event_loop, &mut app, &mut r, w, logical, scale, frames)?;
                    out += &row;
                }
            } else {
                let mut v = VelloSurface::new(window.clone(), size.width, size.height)?;
                writeln!(notes, "  vello_gpu {}.", v.info)?;
                for w in WORKLOADS {
                    eprintln!("{renderer} {} scale {scale}", w.name);
                    let row = vello_frames(&mut event_loop, &mut app, &mut v, w, scale, frames)?;
                    out += &row;
                }
            }
            app.window = None;
            drop(window);
            pump(&mut event_loop, &mut app, Duration::from_millis(300));
        }
    }
    let mut report = format!(
        "# Presentation without read-back\n\nMonitors: {}\n\nFrames paced to one per {} ms; {frames} measured frames after {WARM} warm-up frames per renderer, workload and scale. Times in ms, median / p95.\n\n{notes}\n",
        app.monitors,
        PERIOD.as_millis()
    );
    report += "| Workload | Scale | Renderer | UI frame (start → after present) | Phases | GPU completion after submit | Frames not presented |\n|---|---|---|---|---|---|---|\n";
    report += &out;
    print!("{report}");
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn pert_frames(
    event_loop: &mut EventLoop<Notice>,
    app: &mut App,
    renderer: &mut Renderer,
    w: Workload,
    logical: (f32, f32),
    scale: f64,
    frames: u64,
) -> Result<String> {
    let mut scene = pertui::Scene::sized(w.kind, logical, scale)?;
    let (mut compose, mut to_present, mut call, mut total) =
        (Samples::default(), Samples::default(), Samples::default(), Samples::default());
    let mut missed = 0;
    for frame in 0..WARM + frames {
        let _ = event_loop.pump_app_events(Some(Duration::ZERO), app);
        let started = Instant::now();
        let (list, _, c) = scene.compose(frame)?;
        let t = Instant::now();
        let mut before = None;
        let result = renderer.render(list, || before = Some(t.elapsed()))?;
        let r = t.elapsed();
        if result != Frame::Presented {
            missed += 1;
        }
        if frame >= WARM {
            compose.push(c);
            call.push(r);
            to_present.push(before.unwrap_or_default());
            total.push(started.elapsed());
        }
        pace(started);
    }
    Ok(format!(
        "| {} | {scale} | pert-ui wgpu | {} | compose: {}<br>`render` call: {}<br>of which until `before_present`: {} | not reachable through the public API | {missed} |\n",
        w.name,
        cell(&total),
        cell(&compose),
        cell(&call),
        cell(&to_present),
    ))
}

fn vello_frames(
    event_loop: &mut EventLoop<Notice>,
    app: &mut App,
    v: &mut VelloSurface,
    w: Workload,
    scale: f64,
    frames: u64,
) -> Result<String> {
    let mut data = crate::Data::new(w.kind);
    let mut phases: [Samples; 5] = Default::default();
    let (mut total, mut gpu) = (Samples::default(), Samples::default());
    let mut missed = 0;
    let _ = workloads::LOGICAL;
    for frame in 0..WARM + frames {
        let _ = event_loop.pump_app_events(Some(Duration::ZERO), app);
        let started = Instant::now();
        let f = data.advance(w.kind, frame);
        match v.frame(&f, scale)? {
            Some((p, g)) => {
                if frame >= WARM {
                    for (s, d) in phases.iter_mut().zip(p) {
                        s.push(d);
                    }
                    total.push(p.iter().sum());
                    gpu.push(g);
                }
            }
            None => missed += 1,
        }
        pace(started);
    }
    let names = ["encode", "acquire", "prepare (`Renderer::render`)", "submit", "present"];
    let phases: Vec<String> = names.iter().zip(&phases).map(|(n, s)| format!("{n}: {}", cell(s))).collect();
    let _ = ms;
    Ok(format!(
        "| {} | {scale} | vello_gpu | {} | {} | {} | {missed} |\n",
        w.name,
        cell(&total),
        phases.join("<br>"),
        cell(&gpu),
    ))
}
