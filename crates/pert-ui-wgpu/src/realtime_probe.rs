//! Native measurement of per-frame renderer cost for real-time content.
//! Test evidence only: it motivates or rejects renderer work such as a GPU line
//! primitive or streamed images, and sets no pass/fail performance threshold.
use super::*;
use pert_ui_runtime::Element;
use pert_ui_runtime::{
    ColorRole, Component, DrawError, ImageSampling, Length, LineJoin, Painter, Point, ReadSignal,
    Rect, Runtime, RuntimeError, StrokeStyle, UpdateContext, Viewport,
};
use std::time::{Duration, Instant};
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::ActiveEventLoop,
    window::{Window, WindowId},
};

const WIDTH: u32 = 960;
const HEIGHT: u32 = 640;
const WARMUP: u64 = 12;
const FRAMES: u64 = 300;
// The plotted area in logical pixels at scale 1.
const PLOT: [f32; 4] = [80.0, 120.0, 800.0, 400.0];
const WATERFALL: [u32; 2] = [512, 256];

// The real-time content drawn in the plot, replaced on every frame.
#[derive(Clone, Copy, Debug)]
enum Workload {
    // One minimum/maximum rectangle per physical column: the technique the
    // built-in `Trace` uses, which avoids vector geometry.
    Columns { samples: usize },
    // One antialiased stroked polyline for a changing line trace.
    Polyline { samples: usize },
    // A color image scrolled by one row for a changing heatmap.
    Waterfall,
}
impl Workload {
    fn name(self) -> String {
        match self {
            Self::Columns { samples } => format!("min/max columns, {samples} samples"),
            Self::Polyline { samples } => format!("stroked polyline, {samples} points"),
            Self::Waterfall => format!("waterfall image {}x{}", WATERFALL[0], WATERFALL[1]),
        }
    }
}

// A deterministic signal whose shape moves with the frame number.
fn sample(frame: u64, index: usize, samples: usize) -> f32 {
    let t = index as f32 / samples as f32;
    let phase = frame as f32 * 0.07;
    0.5 + 0.3 * (t * 37.0 + phase).sin()
        + 0.12 * (t * 211.0 - 2.0 * phase).sin()
        + 0.05 * (t * 1301.0 + 3.0 * phase).sin()
}

struct Content {
    signal: ReadSignal<u64>,
    workload: Workload,
    values: Vec<f32>,
    // Rows of RGBA8, newest first; rebuilt into an image on every paint.
    rows: std::collections::VecDeque<Vec<u8>>,
}
impl Content {
    fn new(signal: ReadSignal<u64>, workload: Workload) -> Self {
        Self {
            signal,
            workload,
            values: Vec::new(),
            rows: std::collections::VecDeque::new(),
        }
    }
}
impl Component for Content {
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        let frame = cx.read_paint(&self.signal)?;
        match self.workload {
            Workload::Columns { samples } | Workload::Polyline { samples } => {
                self.values = (0..samples).map(|i| sample(frame, i, samples)).collect();
            }
            Workload::Waterfall => {
                let [width, height] = WATERFALL.map(|v| v as usize);
                let row = (0..width)
                    .flat_map(|x| {
                        let level = sample(frame, x, width).clamp(0.0, 1.0);
                        [(level * 255.0) as u8, (level * 120.0) as u8, 64, 255]
                    })
                    .collect();
                self.rows.push_front(row);
                self.rows.resize(height, vec![0; width * 4]);
            }
        }
        Ok(())
    }
    fn paint(&self, painter: &mut Painter) -> Result<(), DrawError> {
        let [x, y, width, height] = PLOT;
        let color = painter.color(ColorRole::Primary);
        match self.workload {
            Workload::Columns { samples } => {
                let columns = (width as usize).min(samples);
                let column_width = width / columns as f32;
                for column in 0..columns {
                    let values = &self.values[column * samples / columns..][..samples / columns];
                    let (low, high) = values
                        .iter()
                        .fold((f32::MAX, f32::MIN), |(l, h), &v| (l.min(v), h.max(v)));
                    let top = y + (1.0 - high) * height;
                    let bottom = y + (1.0 - low) * height;
                    painter.fill_rect(
                        Rect::new(
                            x + column as f32 * column_width,
                            top,
                            column_width,
                            (bottom - top).max(1.0),
                        )?,
                        color,
                    );
                }
            }
            Workload::Polyline { samples } => {
                let points = self
                    .values
                    .iter()
                    .enumerate()
                    .map(|(i, &v)| {
                        Point::new(
                            x + i as f32 * width / (samples - 1) as f32,
                            y + (1.0 - v) * height,
                        )
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                let style = StrokeStyle::new(Length::new(1.5)?).with_join(LineJoin::Round);
                painter.stroke_polyline(&points, style, color)?;
            }
            Workload::Waterfall => {
                // A content-identified image must be rebuilt for every change.
                let pixels = self.rows.iter().flatten().copied().collect();
                let image =
                    pert_ui_runtime::RasterImage::from_srgba8(WATERFALL[0], WATERFALL[1], pixels)?;
                painter.draw_image(
                    &image,
                    Rect::new(x, y, width, height)?,
                    ImageSampling::Linear,
                );
            }
        }
        Ok(())
    }
}

// Median and 95th percentile of one phase's samples.
fn summary(samples: &mut [Duration]) -> String {
    samples.sort_unstable();
    let median = samples[samples.len() / 2];
    let p95 = samples[(samples.len() * 95).div_ceil(100) - 1];
    format!("median {median:?}, p95 {p95:?}")
}

impl Renderer {
    fn measure_realtime_workloads(&mut self, window: &Window) -> Result<(), RenderError> {
        if self.extent != Some(PhysicalExtent::new(WIDTH, HEIGHT)) {
            return Err(RenderError::Graphics(format!(
                "real-time probe needs a {WIDTH}x{HEIGHT} physical surface, got {:?}",
                self.extent
            )));
        }
        let result = (|| -> Result<(), Box<dyn std::error::Error>> {
            let target = self.device.create_texture(&wgpu::TextureDescriptor {
                label: Some("real-time probe target"),
                size: wgpu::Extent3d {
                    width: WIDTH,
                    height: HEIGHT,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: self.config.format,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
                view_formats: &[],
            });
            let view = target.create_view(&Default::default());
            println!(
                "Real-time probe on {:?}: {WIDTH}x{HEIGHT}, scale {}, {FRAMES} frames after {WARMUP} warm-up frames, plot {PLOT:?}. Times: update = signal set + flush (includes producing the data), compose = paint (includes building the waterfall image), rasterize = CPU vector coverage, prepare = vertex encoding and resource upload, GPU = encode + submit + wait for completion of one full offscreen pass. Presentation is not measured.",
                self.adapter.get_info(),
                window.scale_factor(),
            );
            for workload in [
                Workload::Columns { samples: 4096 },
                Workload::Polyline { samples: 800 },
                Workload::Polyline { samples: 4096 },
                Workload::Waterfall,
            ] {
                self.measure_workload(workload, &target, &view)?;
            }
            Ok(())
        })();
        result.map_err(|error| RenderError::Graphics(error.to_string()))
    }

    fn measure_workload(
        &mut self,
        workload: Workload,
        target: &wgpu::Texture,
        view: &wgpu::TextureView,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Start every workload from an empty renderer state.
        self.prepare_scene(Runtime::new().paint()?)?;
        let mut ui = Runtime::new();
        ui.set_viewport(Viewport::new(WIDTH as f32, HEIGHT as f32, 1.0)?)?;
        let signal = ui.signal(0_u64);
        ui.mount_tree(Element::new(
            "content",
            Content::new(signal.read_only(), workload),
        ))?;
        let mut phases: [Vec<Duration>; 6] = Default::default();
        let mut previous: Option<DisplayList> = None;
        let mut first_pixels = None;
        for frame in 0..WARMUP + FRAMES {
            let start = Instant::now();
            signal.set(frame)?;
            ui.flush(Duration::from_millis(frame + 1))?;
            let update = start.elapsed();
            let start = Instant::now();
            let scene = ui.paint()?;
            let compose = start.elapsed();
            if previous.as_ref() == Some(&scene) {
                return Err(format!("{} did not change between frames", workload.name()).into());
            }
            previous = Some(scene.clone());
            let start = Instant::now();
            let rasterized = self.rasterizer.prepare(&scene)?;
            let rasterize = start.elapsed();
            let start = Instant::now();
            self.prepare_rasterized(rasterized)?;
            let prepare = start.elapsed();
            let gpu = self.submit_probe_pass(view, None)?;
            if frame >= WARMUP {
                let total = update + compose + rasterize + prepare + gpu;
                for (phase, value) in phases
                    .iter_mut()
                    .zip([update, compose, rasterize, prepare, gpu, total])
                {
                    phase.push(value);
                }
            }
            if frame == WARMUP {
                first_pixels = Some(self.probe_pixels(target, WIDTH, HEIGHT)?);
            }
        }
        if first_pixels == Some(self.probe_pixels(target, WIDTH, HEIGHT)?) {
            return Err(format!("{} drew identical first and last frames", workload.name()).into());
        }
        let scene = self.scene.as_ref().ok_or("no prepared scene")?;
        let mask_bytes: usize = scene.masks().iter().map(|m| m.pixels().len()).sum();
        let image_bytes: usize = scene.images().iter().map(|i| i.pixels().len()).sum();
        if matches!(workload, Workload::Polyline { .. }) && scene.masks().is_empty() {
            return Err("the polyline produced no coverage mask".into());
        }
        let [update, compose, rasterize, prepare, gpu, total] = &mut phases;
        println!(
            "Real-time probe {}: last frame {} commands, {} vertex bytes, {} batches, {} masks ({mask_bytes} bytes), {} images ({image_bytes} bytes); update {}; compose {}; rasterize {}; prepare {}; GPU {}; total {}",
            workload.name(),
            scene.len(),
            self.encoded.bytes.len(),
            self.batches.len(),
            scene.masks().len(),
            scene.images().len(),
            summary(update),
            summary(compose),
            summary(rasterize),
            summary(prepare),
            summary(gpu),
            summary(total),
        );
        Ok(())
    }
}

struct Harness {
    window: Option<Arc<Window>>,
    renderer: Option<Renderer>,
    error: Option<Box<dyn std::error::Error>>,
    proxy: winit::event_loop::EventLoopProxy<Notice>,
}
impl ApplicationHandler<Notice> for Harness {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let result = (|| -> Result<(), Box<dyn std::error::Error>> {
            let window = Arc::new(
                event_loop.create_window(
                    Window::default_attributes()
                        .with_title("pert-ui real-time probe")
                        .with_inner_size(winit::dpi::PhysicalSize::new(WIDTH, HEIGHT)),
                )?,
            );
            let size = window.inner_size();
            let proxy = self.proxy.clone();
            self.renderer = Some(Renderer::new(
                window.clone(),
                PhysicalExtent::new(size.width, size.height),
                move |notice| {
                    let _ = proxy.send_event(notice);
                },
            )?);
            window.request_redraw();
            self.window = Some(window);
            Ok(())
        })();
        if let Err(error) = result {
            self.error = Some(error);
            event_loop.exit();
        }
    }
    fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        if matches!(event, WindowEvent::RedrawRequested)
            && let (Some(renderer), Some(window)) = (&mut self.renderer, &self.window)
        {
            if let Err(error) = renderer.measure_realtime_workloads(window) {
                self.error = Some(error.into());
            }
            event_loop.exit();
        }
    }
    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: Notice) {
        let (Notice::Error(message) | Notice::Lost(message)) = event;
        self.error = Some(RenderError::Graphics(message).into());
        event_loop.exit();
    }
}

/// Measures per-frame renderer phases for changing traces and images.
pub(super) fn realtime_workload_probe() -> Result<(), Box<dyn std::error::Error>> {
    let event_loop = super::native_smoke::native_test_event_loop()?;
    let mut harness = Harness {
        window: None,
        renderer: None,
        error: None,
        proxy: event_loop.create_proxy(),
    };
    event_loop.run_app(&mut harness)?;
    match harness.error {
        Some(error) => Err(error),
        None => Ok(()),
    }
}
